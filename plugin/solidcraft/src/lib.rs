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
use rulercraft_mesh::io::{ImportLimits, read_mesh};
use rulercraft_mesh::{Mesh, Unit};
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

/// Enclosed volume of the scan (meaningful when the scan is closed), mm³.
fn enclosed_volume(m: &Mesh) -> f64 {
    let v: f64 = m
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| {
                m.positions
                    .get(i as usize)
                    .map_or([0.0; 3], |p| p.map(f64::from))
            });
            (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0
        })
        .sum();
    v.abs()
}

/// SolidCraft rebuilds the design privately; its volume next to the scan's.
fn check_in_solidcraft(text: &[u8], scan_volume: f64) -> Value {
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
            "difference_percent": if scan_volume > 0.0 { 100.0 * (rebuilt - scan_volume) / scan_volume } else { 0.0 },
            "note": "the scan's volume means something only when the scan is closed",
        }),
        None => json!({"error": reply.get("error").cloned().unwrap_or(Value::Null)}),
    }
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
        let unit = match options.get("units").and_then(Value::as_str) {
            None => Unit::Mm,
            Some(u) => Unit::parse(u).ok_or_else(|| {
                PluginError::Unsupported(format!("scan units `{u}`: choose mm, cm, m or in"))
            })?,
        };
        let bytes = read_all(file.size)?;
        check_cancel()?;
        control::progress(0.35, "Reading the mesh");
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
        let check = check_in_solidcraft(&text, enclosed_volume(&mesh));
        if let Some(d) = report.get_mut("details").and_then(Value::as_object_mut) {
            d.insert("volume_check".into(), check);
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
