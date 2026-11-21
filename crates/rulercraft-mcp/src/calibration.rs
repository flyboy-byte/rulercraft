// SPDX-License-Identifier: MIT OR Apache-2.0
use super::FileUnit;
use rmcp::schemars;
use rulercraft_mesh::Mesh;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Calibration {
    /// Two vertex IDs on the raw point cloud.
    vertices: [usize; 2],
    /// Physical distance between the two selected points, mm.
    distance_mm: f64,
    /// Optional second hand reading, used only to check the resulting scale.
    check: Option<Span>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct Span {
    vertices: [usize; 2],
    distance_mm: f64,
}

fn points_and_length(
    mesh: &Mesh,
    ids: [usize; 2],
    reading: f64,
) -> Result<([[f32; 3]; 2], f64), String> {
    if !reading.is_finite() || reading <= 0. {
        return Err("calibration distance_mm must be finite and positive".into());
    }
    let point = |i: usize| {
        mesh.positions
            .get(i)
            .copied()
            .ok_or_else(|| format!("calibration vertex {i} is outside the cloud"))
    };
    let points = [point(ids[0])?, point(ids[1])?];
    let length = points[0]
        .into_iter()
        .zip(points[1])
        .map(|(a, b)| (f64::from(a) - f64::from(b)).powi(2))
        .sum::<f64>()
        .sqrt();
    if length == 0. {
        return Err("calibration needs two distinct locations".into());
    }
    Ok((points, length))
}

impl Calibration {
    pub(crate) fn apply(&self, mesh: &mut Mesh, unit: FileUnit) -> Result<Value, String> {
        let (points, length) = points_and_length(mesh, self.vertices, self.distance_mm)?;
        let factor = self.distance_mm / length;
        if !factor.is_finite() || factor <= 0. {
            return Err("calibration scale is outside the supported range".into());
        }
        let mut record = json!({"source_unit":unit,"vertices":self.vertices,"source_points_mm":points,
            "distance_mm":self.distance_mm,"source_span_mm":length,"factor":factor});
        for coord in mesh.positions.iter_mut().flatten() {
            let scaled = (f64::from(*coord) * factor) as f32;
            if !scaled.is_finite() || (scaled == 0. && *coord != 0.) {
                return Err("calibrated position is outside the supported range".into());
            }
            *coord = scaled;
        }
        let (_, calibrated_span) = points_and_length(mesh, self.vertices, self.distance_mm)?;
        // Allow ordinary f32 rounding near the origin, scaled to the length being verified.
        let rounding = self.distance_mm * (8.0 * f64::from(f32::EPSILON));
        if (calibrated_span - self.distance_mm).abs() > rounding {
            return Err("coordinate precision cannot reproduce the calibration span. Move the source cloud closer to its origin and pick the reference again.".into());
        }
        if let Some(check) = &self.check {
            let (_, scaled) = points_and_length(mesh, check.vertices, check.distance_mm)?;
            record["check"] = json!({"vertices":check.vertices,"distance_mm":check.distance_mm,
                "scaled_span_mm":scaled,"difference_mm":scaled-check.distance_mm});
        }
        Ok(record)
    }
}
