// SPDX-License-Identifier: MIT OR Apache-2.0
//! A scan as an editable SolidCraft part (docs/specs/mesh-to-cad.md, "SolidCraft design").
//!
//! The mesh is segmented into flat and round regions; the largest flat region is the base. The part is
//! rebuilt as a plate (a box over the base outline, as thick as the farthest parallel face) with round
//! holes cut square to it, every size a named parameter, in a frame where the plate's corner is the origin.
//! What does not fit that shape is listed in a loss report, never guessed. The output is the plain JSON
//! design SolidCraft opens (its book, "Writing a plugin", "An importer"); nothing here talks to SolidCraft.
use crate::Mesh;
use crate::graph::MeshGraph;
use crate::math::{cross, dot, length, normalize, scale, sub};
use crate::segment::{segment_cylinders, segment_planes};
use serde_json::{Value, json};

/// The SolidCraft design format written. Later SolidCraft versions upgrade it when they open it.
pub const FORMAT: &str = "solidcraft/57";
/// A face or axis within this many degrees counts as parallel or square to the base.
pub const SQUARE_DEG: f64 = 5.0;

/// What [`design`] made: the design JSON and the loss report SolidCraft shows after an import.
#[derive(Debug, Clone, PartialEq)]
pub struct Rebuilt {
    /// The design, in SolidCraft's plain JSON form.
    pub design: Value,
    /// `{summary, items: [{severity, title, subject, message, code}], details}`.
    pub report: Value,
}

/// Why nothing could be rebuilt.
#[derive(Debug, Clone, PartialEq)]
pub struct NotRebuilt(pub String);

impl std::fmt::Display for NotRebuilt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

struct Loss {
    severity: &'static str,
    title: &'static str,
    subject: String,
    message: String,
    code: &'static str,
}

impl Loss {
    fn json(&self) -> Value {
        json!({"severity": self.severity, "title": self.title, "subject": self.subject,
               "message": self.message, "code": self.code})
    }
}

/// A number as SolidCraft expression text, to a micrometre.
fn mm(v: f64) -> String {
    let r = (v * 1000.0).round() / 1000.0;
    let r = if r == 0.0 { 0.0 } else { r };
    format!("{r}")
}

fn acute_deg(a: [f64; 3], b: [f64; 3]) -> f64 {
    let c = dot(a, b).abs().min(1.0);
    c.acos().to_degrees()
}

/// Rebuilds `mesh` as a plate with holes. `name` names the design (usually the file name).
pub fn design(mesh: &Mesh, name: &str) -> Result<Rebuilt, NotRebuilt> {
    let graph = MeshGraph::build(mesh);
    let planes = segment_planes(mesh, &graph, Default::default()).regions;
    let cylinders = segment_cylinders(mesh, &graph, Default::default()).regions;
    let pos = |i: u32| mesh.positions[i as usize].map(f64::from);

    // The base: the largest flat region, with a right-handed frame (u, v, n).
    let base = planes
        .iter()
        .max_by(|a, b| a.area.total_cmp(&b.area))
        .ok_or_else(|| NotRebuilt("the mesh has no flat face to build on".into()))?;
    let o = base.plane.center;
    let n = base.plane.normal;
    let u = base.plane.basis_u;
    let v = if dot(cross(u, base.plane.basis_v), n) < 0.0 {
        scale(base.plane.basis_v, -1.0)
    } else {
        base.plane.basis_v
    };
    let local = |p: [f64; 3]| {
        let d = sub(p, o);
        [dot(d, u), dot(d, v), dot(d, n)]
    };

    // Outline: the base face's own points in its frame.
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for &i in &base.vertices {
        let l = local(pos(i));
        for k in 0..2 {
            lo[k] = lo[k].min(l[k]);
            hi[k] = hi[k].max(l[k]);
        }
    }
    let (len, wid) = (hi[0] - lo[0], hi[1] - lo[1]);
    if !(len > 0.0 && wid > 0.0) {
        return Err(NotRebuilt("the base face has no extent".into()));
    }

    // Thickness: the parallel face farthest from the base.
    let mut losses: Vec<Loss> = Vec::new();
    let mut far: Option<f64> = None;
    for (k, p) in planes.iter().enumerate() {
        if std::ptr::eq(p, base) {
            continue;
        }
        if acute_deg(p.plane.normal, n) <= SQUARE_DEG {
            let h = dot(sub(p.plane.center, o), n);
            if far.is_none_or(|f| h.abs() > f.abs()) {
                far = Some(h);
            }
        } else if acute_deg(p.plane.normal, n) < 90.0 - SQUARE_DEG {
            losses.push(Loss {
                severity: "dropped",
                title: "Slanted faces not rebuilt",
                subject: format!("face {}", k + 1),
                message: format!(
                    "a {:.2} mm² face at {:.1}° to the base (a chamfer or draft) is not part of the plate",
                    p.area,
                    acute_deg(p.plane.normal, n)
                ),
                code: "slanted_face",
            });
        }
    }
    let h = far.ok_or_else(|| {
        NotRebuilt("no face is parallel to the base, so the part's thickness is unknown".into())
    })?;
    let thick = h.abs();
    let z0 = h.min(0.0); // the plate runs from z0 to z0 + thick in the base frame
    let to_part = |l: [f64; 3]| [l[0] - lo[0], l[1] - lo[1], l[2] - z0];

    // Holes: cylinders square to the base whose surface faces their axis.
    let tol = (0.02 * thick).max(0.05);
    let margin = (0.1 * thick).max(0.5);
    let mut holes: Vec<Value> = Vec::new();
    let mut hole_area = 0.0;
    for (k, c) in cylinders.iter().enumerate() {
        let f = &c.cylinder;
        let subject = format!("round {} (Ø{:.3} mm)", k + 1, f.diameter);
        if acute_deg(f.axis, n) > SQUARE_DEG {
            losses.push(Loss {
                severity: "dropped",
                title: "Round features not square to the base",
                subject,
                message: format!(
                    "its axis is {:.1}° off the base normal",
                    acute_deg(f.axis, n)
                ),
                code: "tilted_round",
            });
            continue;
        }
        let Some(a) = normalize(f.axis) else { continue };
        let mut facing = 0.0;
        let (mut t0, mut t1) = (f64::INFINITY, f64::NEG_INFINITY);
        for &i in &c.vertices {
            let p = pos(i);
            let d = sub(p, f.axis_point);
            let radial = sub(d, scale(a, dot(d, a)));
            if let Some(r) = normalize(radial) {
                let vn = graph.vertex_normals[i as usize].map(f64::from);
                facing += dot(vn, r);
            }
            let z = local(p)[2] - z0;
            t0 = t0.min(z);
            t1 = t1.max(z);
        }
        if facing >= 0.0 {
            losses.push(Loss {
                severity: "dropped",
                title: "Bosses and pins not rebuilt",
                subject,
                message: "its surface faces away from its axis, so it is a boss or pin, not a hole"
                    .into(),
                code: "boss",
            });
            continue;
        }
        let at = to_part(local(f.axis_point));
        let (x, y) = (at[0], at[1]);
        let r = f.diameter / 2.0;
        if x - r < -tol || y - r < -tol || x + r > len + tol || y + r > wid + tol {
            losses.push(Loss {
                severity: "dropped",
                title: "Holes outside the plate",
                subject,
                message: "it does not lie wholly inside the base outline".into(),
                code: "hole_outside",
            });
            continue;
        }
        let (bottom, top) = (t0 <= tol, t1 >= thick - tol);
        let (z, height, kind) = match (bottom, top) {
            (true, true) => (-margin, thick + 2.0 * margin, "through"),
            (false, true) => (t0, thick - t0 + margin, "blind from the top"),
            (true, false) => (-margin, t1 + margin, "blind from the bottom"),
            (false, false) => {
                losses.push(Loss {
                    severity: "dropped",
                    title: "Holes that open on neither face",
                    subject,
                    message: format!("its wall runs from {t0:.3} to {t1:.3} mm, inside the plate"),
                    code: "hole_internal",
                });
                continue;
            }
        };
        if kind == "through" {
            hole_area += std::f64::consts::PI * r * r;
        }
        holes.push(
            json!({"x": x, "y": y, "z": z, "height": height, "diameter": f.diameter,
                          "kind": kind, "sigma": f.sigma, "coverage_deg": f.coverage_deg}),
        );
    }

    // An outline that is not a full rectangle is simplified to its bounding rectangle.
    let expected = len * wid - hole_area;
    if base.area < 0.98 * expected {
        losses.push(Loss {
            severity: "degraded",
            title: "Outline simplified to a rectangle",
            subject: "base outline".into(),
            message: format!(
                "the base face covers {:.1} mm² of the {:.1} mm² its rectangle leaves after through holes",
                base.area, expected
            ),
            code: "outline_rectangle",
        });
    }

    // The design: parameters for every size, a box, then one cut cylinder per hole.
    let mut params = vec![
        json!({"name": "length", "expr": format!("{} mm", mm(len))}),
        json!({"name": "width", "expr": format!("{} mm", mm(wid))}),
        json!({"name": "thickness", "expr": format!("{} mm", mm(thick))}),
    ];
    let mut features = vec![json!({
        "id": 1, "name": "Plate", "suppressed": false, "body_names": ["Plate"],
        "param_names": [], "type": "box", "corner": ["0", "0", "0"],
        "length": "length", "width": "width", "height": "thickness", "operation": "new_body",
    })];
    for (i, h) in holes.iter().enumerate() {
        let p = format!("hole{}_diameter", i + 1);
        let d = h["diameter"].as_f64().unwrap_or(0.0);
        params.push(json!({"name": p, "expr": format!("{} mm", mm(d))}));
        let z = h["z"].as_f64().unwrap_or(0.0);
        let height = h["height"].as_f64().unwrap_or(0.0);
        features.push(json!({
            "id": i + 2, "name": format!("Hole{}", i + 1), "suppressed": false, "body_names": [],
            "param_names": [], "type": "cylinder",
            "base": [mm(h["x"].as_f64().unwrap_or(0.0)), mm(h["y"].as_f64().unwrap_or(0.0)), mm(z)],
            "axis": [0.0, 0.0, 1.0], "radius": format!("{p} / 2"), "height": mm(height),
            "operation": "cut",
        }));
    }
    let next_id = features.len() + 1;
    let design = json!({
        "format": FORMAT, "name": name, "units": "mm", "params": params, "features": features,
        "next_id": next_id, "reserved_body_names": ["Plate"], "next_component_color_index": 0,
    });

    let through = holes.iter().filter(|h| h["kind"] == "through").count();
    let summary = format!(
        "A {} x {} x {} mm plate with {} hole{} ({} through), from the scan's {} faces and {} round features",
        mm(len),
        mm(wid),
        mm(thick),
        holes.len(),
        if holes.len() == 1 { "" } else { "s" },
        through,
        planes.len(),
        cylinders.len()
    );
    let report = json!({
        "summary": summary,
        "items": losses.iter().map(Loss::json).collect::<Vec<_>>(),
        "details": {
            "base": {"area_mm2": base.area, "sigma": base.plane.sigma, "form_error": base.plane.form_error},
            "plate": {"length": len, "width": wid, "thickness": thick},
            "holes": holes,
            "frame": {"origin": o, "u": u, "v": v, "normal": n, "outline_min": lo, "z0": z0},
        },
    });
    if !len.is_finite() || !wid.is_finite() || !thick.is_finite() || length(n) == 0.0 {
        return Err(NotRebuilt("the fitted base is not finite".into()));
    }
    Ok(Rebuilt { design, report })
}
