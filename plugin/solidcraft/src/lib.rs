// SPDX-License-Identifier: MIT OR Apache-2.0
//! RulerCraft as a SolidCraft importer plugin (DECISIONS.md D-014, docs/specs/solidcraft-plugin.md).
//!
//! SolidCraft hands over a scan file; this reads it with RulerCraft's readers, rebuilds it as an editable
//! part with `rulercraft_mesh::solidcraft::design`, writes the design, and checks it by having SolidCraft
//! rebuild it in a private session (`evaluate`) and comparing volumes. All the geometry is the engine's;
//! this file only moves bytes across SolidCraft's published interface (`wit/plugin.wit`).
#![deny(unsafe_code)]

/// The bindings wit-bindgen generates from SolidCraft's WIT. Its ABI glue is the only `unsafe` here.
#[allow(unsafe_code, missing_docs, clippy::all)]
mod bindings {
    wit_bindgen::generate!({ path: "wit", world: "importer-plugin", pub_export_macro: true });
}

use bindings::craft::plugin::types::{FieldFailure, PluginError};
use bindings::craft::plugin::{control, design, evaluate, source};
use bindings::exports::craft::plugin::importer::Guest;
use rulercraft_mesh::Unit;
use rulercraft_mesh::io::{ImportLimits, format_name, read_mesh};
use serde_json::{Value, json};

/// Largest read or write per host call (SolidCraft's limit).
const CHUNK: u32 = 1 << 20;

struct Plugin;

fn invalid(field: &str, reason: impl ToString) -> PluginError {
    PluginError::InvalidInput(FieldFailure {
        field: field.into(),
        reason: reason.to_string(),
    })
}

fn check_cancel() -> Result<(), PluginError> {
    if control::cancelled() {
        Err(PluginError::Cancelled)
    } else {
        Ok(())
    }
}

/// The whole file being imported, read in chunks.
fn read_all(size: u64) -> Result<Vec<u8>, PluginError> {
    let limit = ImportLimits::default().max_bytes as u64;
    if size > limit {
        return Err(PluginError::Unsupported(format!(
            "the scan is {size} bytes; RulerCraft reads at most {limit}"
        )));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(size).unwrap_or(0));
    while (bytes.len() as u64) < size {
        check_cancel()?;
        let chunk = source::read("", bytes.len() as u64, CHUNK)?;
        if chunk.is_empty() {
            break;
        }
        bytes.extend_from_slice(&chunk);
        control::progress(
            0.3 * bytes.len() as f32 / size.max(1) as f32,
            "Reading the scan",
        );
    }
    Ok(bytes)
}

/// SolidCraft rebuilds the design privately; its volume next to the scan's.
fn check_in_solidcraft(text: &[u8], scan_volume: Option<f64>) -> Value {
    for chunk in text.chunks(CHUNK as usize) {
        if let Err(e) = evaluate::stage(chunk) {
            evaluate::reset();
            return json!({"error": format!("{e:?}")});
        }
    }
    let reply = evaluate::run("inspect.measure", "{}");
    evaluate::reset();
    let reply: Value = match reply {
        Ok(r) => serde_json::from_str(&r).unwrap_or(Value::Null),
        Err(e) => return json!({"error": format!("{e:?}")}),
    };
    match reply.pointer("/total/volume_mm3").and_then(Value::as_f64) {
        Some(rebuilt) => json!({
            "rebuilt_volume_mm3": rebuilt,
            "scan_volume_mm3": scan_volume,
            "difference_percent": scan_volume.map(|scan| 100.0 * (rebuilt - scan) / scan),
            "note": scan_volume.is_none().then_some(
                "Scan volume requires a closed mesh with consistent triangle winding and positive volume.",
            ),
        }),
        None => json!({"error": reply.get("error").cloned().unwrap_or(Value::Null)}),
    }
}

/// A few OBJ exporters write an explicit units comment before the geometry.
fn obj_header_units(bytes: &[u8]) -> Result<Option<&'static str>, PluginError> {
    let mut declared = None;
    for line in bytes
        .split(|&byte| byte == b'\n')
        .scan(0usize, |size, line| {
            *size = size.saturating_add(line.len() + 1);
            (*size <= 64 * 1024).then_some(line)
        })
    {
        let Ok(line) = std::str::from_utf8(line) else {
            break;
        };
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() {
            continue;
        }
        let Some(comment) = line.strip_prefix('#') else {
            break;
        };
        let mut words = comment.split_whitespace();
        if !words
            .next()
            .is_some_and(|word| word.eq_ignore_ascii_case("units"))
        {
            continue;
        }
        let (Some(unit), None) = (words.next(), words.next()) else {
            continue;
        };
        let unit = match unit.to_ascii_lowercase().as_str() {
            "mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => "mm",
            "cm" | "centimeter" | "centimeters" | "centimetre" | "centimetres" => "cm",
            "m" | "meter" | "meters" | "metre" | "metres" => "m",
            "in" | "inch" | "inches" => "in",
            _ => {
                return Err(invalid(
                    "units",
                    "OBJ units comment is unsupported. Choose the source units explicitly.",
                ));
            }
        };
        if declared.is_some_and(|previous| previous != unit) {
            return Err(invalid(
                "units",
                "OBJ units comments disagree. Choose the source units explicitly.",
            ));
        }
        declared = Some(unit);
    }
    Ok(declared)
}

fn import_units(name: &str, bytes: &[u8], options: &Value) -> Result<(Unit, Value), PluginError> {
    let format = format_name(name).map_err(|e| invalid("file", e))?;
    let requested = match options.get("units") {
        Some(value) => value
            .as_str()
            .ok_or_else(|| invalid("units", "choose auto, mm, cm, m or in"))?,
        None => "auto",
    };
    if requested != "auto" && Unit::parse(requested).is_none() {
        return Err(invalid("units", "choose auto, mm, cm, m or in"));
    }
    let (label, source) = match (format, requested) {
        ("3mf", _) => (None, "3mf_model"),
        ("glb" | "gltf", _) => (Some("m"), "gltf_standard"),
        ("obj", "auto") => match obj_header_units(bytes)? {
            Some(unit) => (Some(unit), "obj_header"),
            None => (Some("mm"), "default"),
        },
        (_, "auto") => (Some("mm"), "default"),
        (_, unit) => (Some(unit), "override"),
    };
    let unit = Unit::parse(label.unwrap_or("mm"))
        .ok_or_else(|| invalid("units", "choose auto, mm, cm, m or in"))?;
    Ok((
        unit,
        json!({"source":source,"file_unit":label,"internal_unit":"mm"}),
    ))
}

impl Guest for Plugin {
    fn import_file(
        _entry: String,
        file: source::FileInfo,
        options: String,
    ) -> Result<String, PluginError> {
        let options: Value = if options.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&options).map_err(|e| invalid("options", e))?
        };
        let bytes = read_all(file.size)?;
        check_cancel()?;
        control::progress(0.35, "Reading the mesh");
        let (unit, units) = import_units(&file.name, &bytes, &options)?;
        let mesh = read_mesh(&file.name, &bytes, unit, ImportLimits::default())
            .map_err(|e| invalid("file", e))?;
        drop(bytes);
        check_cancel()?;
        control::progress(0.45, "Finding faces and holes");
        let stem = std::path::Path::new(&file.name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("scan")
            .to_string();
        let rebuilt = rulercraft_mesh::solidcraft::design(&mesh, &stem)
            .map_err(|e| PluginError::Unsupported(e.to_string()))?;
        check_cancel()?;
        control::progress(0.8, "Writing the part");
        let text = rebuilt.design.to_string().into_bytes();
        for chunk in text.chunks(CHUNK as usize) {
            design::write(chunk)?;
        }
        control::progress(0.9, "Checking the part in SolidCraft");
        let mut report = rebuilt.report;
        let check = check_in_solidcraft(&text, report["details"]["scan_volume_mm3"].as_f64());
        if let Some(d) = report.get_mut("details").and_then(Value::as_object_mut) {
            d.insert("volume_check".into(), check);
            d.insert("import_units".into(), units);
        }
        control::progress(1.0, "Done");
        Ok(report.to_string())
    }
}

/// The exported entry points wit-bindgen generates for `Plugin` (ABI glue, so `unsafe` is allowed here).
#[allow(unsafe_code)]
mod export {
    use super::Plugin;
    crate::bindings::export!(Plugin with_types_in crate::bindings);
}
