// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unaligned mesh deviation exposed through the command-line interface.
use rulercraft_mesh::Unit;
use rulercraft_mesh::deviation::deviation;
use serde_json::json;

pub(crate) fn run(args: &[String]) -> Result<String, String> {
    let [measured_path, reference_path, rest @ ..] = args else {
        return Err("deviation needs <measured> <reference>".into());
    };
    let (mut measured_unit, mut reference_unit) = (Unit::Mm, Unit::Mm);
    let (mut tolerance, mut offset, mut limit) = (0.1f64, 0usize, 256usize);
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--units" | "--reference-units" => {
                let value = it
                    .next()
                    .ok_or_else(|| format!("{flag} needs mm, cm, m or in"))?;
                let unit = Unit::parse(value)
                    .ok_or_else(|| format!("unknown unit `{value}`: use mm, cm, m or in"))?;
                if flag == "--units" {
                    measured_unit = unit;
                } else {
                    reference_unit = unit;
                }
            }
            "--tolerance" => {
                tolerance = it
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .ok_or("--tolerance needs a finite nonnegative number")?;
                if !tolerance.is_finite() || tolerance < 0.0 {
                    return Err("--tolerance needs a finite nonnegative number".into());
                }
            }
            "--offset" | "--limit" => {
                let value = it
                    .next()
                    .and_then(|s| s.parse::<usize>().ok())
                    .ok_or_else(|| format!("{flag} needs a nonnegative integer"))?;
                if flag == "--offset" {
                    offset = value;
                } else {
                    limit = value;
                }
            }
            other => return Err(format!("unknown deviation option `{other}`")),
        }
    }
    if limit > 1024 {
        return Err("deviation page limit must be at most 1024".into());
    }
    let measured = crate::load(measured_path, measured_unit)?;
    let reference = crate::load(reference_path, reference_unit)?;
    let result = deviation(&measured, &reference, tolerance).map_err(|e| e.to_string())?;
    let total = result.distances.len();
    let samples: Vec<_> = result
        .distances
        .iter()
        .enumerate()
        .skip(offset)
        .take(limit)
        .map(|(vertex, distance)| json!({"vertex":vertex,"distance":distance}))
        .collect();
    Ok(json!({"stats":result.stats,"total":total,"offset":offset,"returned":samples.len(),
        "truncated":offset.saturating_add(samples.len()) < total,"samples":samples,
        "convention":"Unaligned signed distances in mm; sign follows the reference triangle winding."}).to_string())
}
