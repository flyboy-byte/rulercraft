// SPDX-License-Identifier: MIT OR Apache-2.0
use super::{Filters, Loaded};
use rmcp::schemars;
use rulercraft_mesh::{
    fit::{Clip, CylinderOptions, fit_cylinder, fit_plane},
    math::vec3::{Vec3, add, cross, dot, length, normalize, scale, sub},
    solidcraft::FORMAT,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum Depth {
    Through(bool),
    BlindDepthMm(f64),
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlateArgs {
    /// Three top-corner vertex IDs: origin, length corner, width corner.
    corners: [usize; 3],
    /// Fit two opposing side faces to refine the width and its origin.
    width_faces: Option<WidthFaces>,
    thickness_mm: f64,
    diameter_mm: f64,
    /// Center in local plate coordinates, measured from the first corner.
    center_mm: Option<[f64; 2]>,
    /// Selected bore wall, as an alternative to an explicit center.
    bore_selection: Option<Filters>,
    /// Maximum difference between the free fit and hand diameter, required with a bore selection.
    max_diameter_difference_mm: Option<f64>,
    /// Optional description of how the caller determined the hole center.
    center_note: Option<String>,
    depth: Depth,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct WidthFaces {
    origin_side: Filters,
    opposite_side: Filters,
    /// Maximum accepted RMS residual in mm for each fitted plane.
    max_sigma_mm: f64,
}

impl WidthFaces {
    fn measure(
        &self,
        loaded: &Loaded,
        top: Vec3,
        axis: Vec3,
        picked: f64,
    ) -> Result<(f64, f64, Value), String> {
        if !self.max_sigma_mm.is_finite() || self.max_sigma_mm <= 0.0 {
            return Err("width_faces max_sigma_mm must be finite and positive".into());
        }
        let min_alignment = 10_f64.to_radians().cos();
        let fit = |name: &str, filters: &Filters| {
            let ids = loaded.select(&filters.selection())?;
            if ids.len() > 1_000_000 {
                return Err(format!(
                    "{name} selection exceeds the 1 million point fit limit"
                ));
            }
            let plane = fit_plane(&loaded.mesh.positions, None, &ids, Clip::None)
                .map_err(|e| format!("cannot fit {name} width face: {e}"))?;
            let alignment = dot(plane.normal, axis);
            if alignment.abs() < min_alignment {
                return Err(format!(
                    "{name} width face must face within 10 degrees of the width axis"
                ));
            }
            if plane.sigma > self.max_sigma_mm {
                return Err(format!(
                    "{name} width face RMS {:.4} mm exceeds max_sigma_mm {:.4}",
                    plane.sigma, self.max_sigma_mm
                ));
            }
            Ok((plane, alignment))
        };
        let (origin, low_alignment) = fit("origin_side", &self.origin_side)?;
        let (opposite, high_alignment) = fit("opposite_side", &self.opposite_side)?;
        if dot(origin.normal, opposite.normal).abs() < min_alignment {
            return Err("width faces must be parallel within 10 degrees".into());
        }
        // Measure near the observed patches, avoiding extrapolation from the middle of a
        // noisy face to a corner. Remove the width component to retain the picked origin.
        let midpoint = sub(scale(add(origin.center, opposite.center), 0.5), top);
        let station = add(top, sub(midpoint, scale(axis, dot(midpoint, axis))));
        let low = dot(sub(origin.center, station), origin.normal) / low_alignment;
        let high = dot(sub(opposite.center, station), opposite.normal) / high_alignment;
        let low_sigma = origin.sigma / low_alignment.abs();
        let high_sigma = opposite.sigma / high_alignment.abs();
        let width = high - low;
        if width <= 2.0 * (low_sigma + high_sigma) {
            return Err(
                "width faces must be distinct and run from origin_side to opposite_side".into(),
            );
        }
        Ok((
            width,
            low,
            json!({
                "source":"side_face_fits","mm":width,"picked_mm":picked,"measurement_origin_mm":station,
                "origin_side":{"fit":origin,"offset_mm":low},
                "opposite_side":{"fit":opposite,"offset_mm":high}
            }),
        ))
    }
}

pub(crate) fn plate(loaded: &Loaded, args: PlateArgs) -> Result<Value, String> {
    let point = |id: usize| {
        loaded
            .mesh
            .positions
            .get(id)
            .map(|p| p.map(f64::from))
            .ok_or_else(|| format!("plate corner vertex {id} is outside the open geometry"))
    };
    let [origin, along, across] = args.corners;
    let top = point(origin)?;
    let long_edge = sub(point(along)?, top);
    let side_edge = sub(point(across)?, top);
    let x_axis = normalize(long_edge).ok_or("plate length picks coincide")?;
    let side = normalize(side_edge).ok_or("plate width picks coincide")?;
    let cosine = dot(x_axis, side).clamp(-1.0, 1.0);
    if cosine.abs() > 10_f64.to_radians().sin() {
        return Err("plate corner picks must be within 10 degrees of square".into());
    }
    let normal = normalize(cross(x_axis, side)).ok_or("plate corners are collinear")?;
    let y_axis = cross(normal, x_axis);
    let plate_length = length(long_edge);
    let picked_width = dot(side_edge, y_axis);
    let (width, width_origin, width_evidence) = match args.width_faces {
        Some(faces) => faces.measure(loaded, top, y_axis, picked_width)?,
        None => (
            picked_width,
            0.0,
            json!({"source":"scan_picks","mm":picked_width,"vertices":[origin,across]}),
        ),
    };
    let top = add(top, scale(y_axis, width_origin));
    let thickness = args.thickness_mm;
    let diameter = args.diameter_mm;
    if [thickness, diameter]
        .iter()
        .any(|&v| !v.is_finite() || v <= 0.0)
    {
        return Err("plate thickness and hole diameter must be finite and positive".into());
    }
    let (center, center_source, bore_fit) = match (args.center_mm, args.bore_selection.as_ref()) {
        (Some(center), None) if args.max_diameter_difference_mm.is_none() => {
            ([center[0], center[1] - width_origin], "caller", None)
        }
        (None, Some(filters)) => {
            let allowance = args
                .max_diameter_difference_mm
                .filter(|v| v.is_finite() && *v > 0.0)
                .ok_or("a positive max_diameter_difference_mm is required with bore_selection")?;
            let selected = loaded.select(&filters.selection())?;
            if selected.len() > 1_000_000 {
                return Err("bore selection exceeds the 1 million point fit limit".into());
            }
            let fit = fit_cylinder(
                &loaded.mesh.positions,
                Some(&loaded.graph.vertex_normals),
                &selected,
                CylinderOptions {
                    clip: Clip::None,
                    ..Default::default()
                },
            )
            .map_err(|e| format!("bore fit failed: {e}. Select more wall or supply center_mm."))?;
            let alignment = dot(fit.axis, normal).clamp(-1.0, 1.0);
            if fit.coverage_deg < 180.0 || alignment.abs() < 10_f64.to_radians().cos() {
                return Err("bore fit needs at least 180 degrees of wall and an axis within 10 degrees of normal. Select more wall or supply center_mm.".into());
            }
            let difference = fit.diameter - diameter;
            if difference.abs() > allowance {
                return Err(format!(
                    "free bore diameter {:.4} mm differs from the hand value {diameter} mm by {difference:.4} mm. Check the selection and measurement, or supply center_mm.",
                    fit.diameter
                ));
            }
            let offset = sub(fit.axis_point, top);
            let hit = add(offset, scale(fit.axis, -dot(offset, normal) / alignment));
            let center = [dot(hit, x_axis), dot(hit, y_axis)];
            (
                center,
                "bore_fit",
                Some(json!({"diameter_mm":fit.diameter,
                "difference_mm":difference,"coverage_deg":fit.coverage_deg,
                "sigma_mm":fit.sigma,"used":fit.used,"tilt_deg":alignment.abs().acos().to_degrees(),
                "axis":fit.axis,"axis_point_mm":fit.axis_point})),
            )
        }
        _ => {
            return Err(
                "supply either center_mm or bore_selection with its diameter allowance".into(),
            );
        }
    };
    let [x, y] = center;
    let radius = diameter / 2.0;
    if !(x > radius && x < plate_length - radius && y > radius && y < width - radius) {
        return Err("hole must fit strictly inside the plate edges".into());
    }
    let parameter = |name, value: f64| json!({"name":name,"expr":format!("{value} mm"),"unit":"mm","model":false});
    let mut params = vec![
        parameter("length", plate_length),
        parameter("width", width),
        parameter("thickness", thickness),
        parameter("hole_diameter", diameter),
        parameter("hole_x", x),
        parameter("hole_y", y),
    ];
    let (bottom, height, depth_evidence) =
        match args.depth {
            Depth::Through(true) => (
                json!(0),
                "thickness",
                json!({"source":"caller","through":true}),
            ),
            Depth::BlindDepthMm(depth) if depth.is_finite() && depth > 0.0 && depth < thickness => {
                params.push(parameter("hole_depth", depth));
                (
                    json!("thickness - hole_depth"),
                    "hole_depth",
                    json!({"source":"hand","mm":depth}),
                )
            }
            _ => return Err(
                "choose a through hole or a positive blind depth smaller than the plate thickness"
                    .into(),
            ),
        };
    Ok(json!({
        "design": {"format":FORMAT,"name":"Assisted plate","units":"mm","params":params,
            "features":[
                {"id":1,"name":"Plate","type":"box","operation":"new_body","body_names":["Plate"],
                    "corner":[0,0,0],"length":"length","width":"width","height":"thickness",
                    "suppressed":false,"param_names":[]},
                {"id":2,"name":"Hole","type":"cylinder","operation":"cut","base":["hole_x","hole_y",bottom],
                    "axis":[0,0,1],"radius":"hole_diameter / 2","height":height,
                    "suppressed":false,"param_names":[]}
            ], "next_id":3,"next_component_color_index":0},
        "evidence":{
            "length":{"source":"scan_picks","mm":plate_length,"vertices":[origin,along]},
            "width":width_evidence,
            "thickness":{"source":"hand","mm":thickness},
            "hole_diameter":{"source":"hand","mm":diameter},
            "hole_x":{"source":center_source,"mm":x,"note":args.center_note},
            "hole_y":{"source":center_source,"mm":y,"note":args.center_note},
            "hole_depth":depth_evidence,
            "bore_fit":bore_fit
        },
        "frame":{"top_origin_mm":top,"x_axis":x_axis,"y_axis":y_axis,"normal":normal,
            "observed_angle_deg":cosine.acos().to_degrees()}
    }))
}
