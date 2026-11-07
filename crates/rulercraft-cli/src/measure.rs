// SPDX-License-Identifier: MIT OR Apache-2.0
//! JSON boundary for the typed element-relation command.
use rulercraft_mesh::commands::{MeasureRequest, measure};
use rulercraft_mesh::measure::{Element, Geometry, RelationKind};
use serde_json::{Value, json};
use std::io::Read;
fn element(path: &str) -> Result<Element, String> {
    const MAX_BYTES: u64 = 1 << 20;
    let file = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{path}: {e}"))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(format!("{path}: element JSON exceeds the 1 MiB limit"));
    }
    let v: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("{path}: invalid element JSON: {e}"))?;
    let g = v.get("fit").unwrap_or(&v);
    let vector = |name: &str| -> Result<[f64; 3], String> {
        let a = g
            .get(name)
            .and_then(Value::as_array)
            .filter(|a| a.len() == 3)
            .ok_or_else(|| format!("{path}: {name} needs three numbers"))?;
        let mut out = [0.; 3];
        for (i, v) in a.iter().enumerate() {
            out[i] = v
                .as_f64()
                .ok_or_else(|| format!("{path}: {name} needs three finite numbers"))?;
        }
        Ok(out)
    };
    let geometry = match g.get("kind").and_then(Value::as_str) {
        Some("point") => Geometry::Point(vector("position")?),
        Some("sphere") => Geometry::Sphere {
            center: vector("center")?,
        },
        Some("plane") => Geometry::Plane {
            center: vector("center")?,
            normal: vector("normal")?,
        },
        Some("cylinder") => Geometry::Cylinder {
            axis_point: vector("axis_point")?,
            axis: vector("axis")?,
        },
        _ => {
            return Err(format!(
                "{path}: element kind must be point, sphere, plane or cylinder"
            ));
        }
    };
    let id = match v.get("id").or_else(|| g.get("id")) {
        Some(id) => id
            .as_str()
            .ok_or_else(|| format!("{path}: id must be a string"))?,
        None => path,
    };
    Ok(Element {
        id: id.to_string(),
        geometry,
    })
}
pub(crate) fn run(args: &[String]) -> Result<String, String> {
    let [first, second] = args else {
        return Err("Usage: rulercraft measure first.json second.json".into());
    };
    let out = measure(&MeasureRequest {
        first: element(first)?,
        second: element(second)?,
    })
    .map_err(|e| e.to_string())?;
    let kind = match out.kind {
        RelationKind::PointPoint => "point_point",
        RelationKind::PointPlane => "point_plane",
        RelationKind::PlanePlane => "plane_plane",
        RelationKind::AxisPlane => "axis_plane",
        RelationKind::AxisAxis => "axis_axis",
    };
    Ok(json!({"first_id":out.first_id,"second_id":out.second_id,"kind":kind,"angle_deg":out.angle.map(f64::to_degrees),"distance":out.distance,"delta":out.delta,"convention":out.convention}).to_string())
}
