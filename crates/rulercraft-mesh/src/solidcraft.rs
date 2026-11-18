// SPDX-License-Identifier: MIT OR Apache-2.0
//! A scan as an editable SolidCraft part (docs/specs/mesh-to-cad.md, "SolidCraft design").
//!
//! The mesh is segmented into flat and round regions; the largest flat region is the base. The part is
//! rebuilt as a plate (a box over the base outline's smallest enclosing rectangle, as thick as the largest
//! parallel face) with pads and bosses joined, pockets and steps cut, and round holes cut square to it,
//! every size a named parameter, in a frame where the plate's corner is the origin. What does not fit
//! that shape is listed in a loss report, never guessed. The output is the plain JSON
//! design SolidCraft opens (its book, "Writing a plugin", "An importer"); nothing here talks to SolidCraft.
use crate::Mesh;
use crate::graph::MeshGraph;
use crate::math::{cross, dot, normalize, scale, sub};
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

/// A length rounded to a micrometre (no negative zero).
fn round_um(v: f64) -> f64 {
    let r = (v * 1000.0).round() / 1000.0;
    if r == 0.0 { 0.0 } else { r }
}

/// A length as SolidCraft expression text, to a micrometre.
fn mm(v: f64) -> String {
    format!("{}", round_um(v))
}

fn acute_deg(a: [f64; 3], b: [f64; 3]) -> f64 {
    let c = dot(a, b).abs().min(1.0);
    c.acos().to_degrees()
}

/// Rotation (radians, in [0, π/2)) of the smallest-area rectangle around `pts`, by rotating calipers over
/// the convex hull. Ties keep the smaller angle, so the result never depends on point order.
fn min_rect_angle(pts: &[[f64; 2]]) -> f64 {
    let mut p: Vec<[f64; 2]> = pts
        .iter()
        .copied()
        .filter(|q| q.iter().all(|v| v.is_finite()))
        .collect();
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    p.dedup();
    if p.len() < 3 {
        return 0.0;
    }
    let turn = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    let mut hull: Vec<[f64; 2]> = Vec::new();
    for pass in 0..2 {
        let start = hull.len();
        let it: Box<dyn Iterator<Item = &[f64; 2]>> = if pass == 0 {
            Box::new(p.iter())
        } else {
            Box::new(p.iter().rev())
        };
        for &q in it {
            while hull.len() >= start + 2
                && turn(hull[hull.len() - 2], hull[hull.len() - 1], q) <= 0.0
            {
                hull.pop();
            }
            hull.push(q);
        }
        hull.pop();
    }
    let mut best = (f64::INFINITY, 0.0);
    for i in 0..hull.len() {
        let (a, b) = (hull[i], hull[(i + 1) % hull.len()]);
        let ang = (b[1] - a[1])
            .atan2(b[0] - a[0])
            .rem_euclid(std::f64::consts::FRAC_PI_2);
        let (c, s) = (ang.cos(), ang.sin());
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for q in &hull {
            let r = [c * q[0] + s * q[1], -s * q[0] + c * q[1]];
            for k in 0..2 {
                lo[k] = lo[k].min(r[k]);
                hi[k] = hi[k].max(r[k]);
            }
        }
        let area = (hi[0] - lo[0]) * (hi[1] - lo[1]);
        if area < best.0 * (1.0 - 1e-9) || (area <= best.0 * (1.0 + 1e-9) && ang < best.1) {
            best = (area, ang);
        }
    }
    best.1
}

/// A flat face parallel to the base, in the part frame.
struct Level {
    index: usize,
    height: f64,
    up: bool,
    area: f64,
    lo: [f64; 2],
    hi: [f64; 2],
    aligned: bool,
}

/// Rebuilds `mesh` as a plate with its pads, bosses, pockets and holes. `name` names the design (usually
/// the file name).
pub fn design(mesh: &Mesh, name: &str) -> Result<Rebuilt, NotRebuilt> {
    let graph = MeshGraph::build(mesh);
    let planes = segment_planes(mesh, &graph, Default::default()).regions;
    let cylinders = segment_cylinders(mesh, &graph, Default::default()).regions;
    let pos = |i: u32| mesh.positions[i as usize].map(f64::from);

    // The base: the largest flat region.
    let (bi, base) = planes
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.area.total_cmp(&b.1.area))
        .ok_or_else(|| NotRebuilt("the mesh has no flat face to build on".into()))?;
    let o = base.plane.center;
    let mut n = base.plane.normal;
    let u0 = base.plane.basis_u;
    let mut v0 = if dot(cross(u0, base.plane.basis_v), n) < 0.0 {
        scale(base.plane.basis_v, -1.0)
    } else {
        base.plane.basis_v
    };

    // The opposite face: the largest other face parallel to the base. The frame's normal points from the
    // base into the part, so the base is at height 0 and the opposite face at the thickness.
    let parallel: Vec<usize> = (0..planes.len())
        .filter(|&k| k != bi && acute_deg(planes[k].plane.normal, n) <= SQUARE_DEG)
        .collect();
    let ti = parallel
        .iter()
        .copied()
        .max_by(|&a, &b| planes[a].area.total_cmp(&planes[b].area))
        .ok_or_else(|| {
            NotRebuilt("no face is parallel to the base, so the part's thickness is unknown".into())
        })?;
    if dot(sub(planes[ti].plane.center, o), n) < 0.0 {
        n = scale(n, -1.0);
        v0 = scale(v0, -1.0); // keeps (u, v, n) right-handed
    }

    // In-plane axes along the outline's edges: the base face's smallest enclosing rectangle.
    let flat: Vec<[f64; 2]> = base
        .vertices
        .iter()
        .map(|&i| {
            let d = sub(pos(i), o);
            [dot(d, u0), dot(d, v0)]
        })
        .collect();
    let a = min_rect_angle(&flat);
    let (c, s) = (a.cos(), a.sin());
    let u = [0, 1, 2].map(|k| c * u0[k] + s * v0[k]);
    let v = [0, 1, 2].map(|k| -s * u0[k] + c * v0[k]);
    let local = |p: [f64; 3]| {
        let d = sub(p, o);
        [dot(d, u), dot(d, v), dot(d, n)]
    };
    let extent = |verts: &[u32]| {
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for &i in verts {
            let l = local(pos(i));
            for k in 0..2 {
                lo[k] = lo[k].min(l[k]);
                hi[k] = hi[k].max(l[k]);
            }
        }
        (lo, hi)
    };
    let (lo, hi) = extent(&base.vertices);
    let (len, wid) = (hi[0] - lo[0], hi[1] - lo[1]);
    if !(len > 0.0 && wid > 0.0 && len.is_finite() && wid.is_finite()) {
        return Err(NotRebuilt("the base face has no extent".into()));
    }
    let thick = dot(sub(planes[ti].plane.center, o), n);
    if !(thick > 0.0 && thick.is_finite()) {
        return Err(NotRebuilt(
            "the opposite face is not apart from the base".into(),
        ));
    }
    let to_part = |l: [f64; 3]| [l[0] - lo[0], l[1] - lo[1], l[2]];
    let tol = (0.02 * thick).max(0.05);
    let margin = (0.1 * thick).max(0.5);
    let mut losses: Vec<Loss> = Vec::new();

    // Every flat face parallel to the base, with its height, side and extent in the part frame.
    let levels: Vec<Level> = parallel
        .iter()
        .map(|&k| {
            let p = &planes[k];
            let (l, h) = extent(&p.vertices);
            let face_flat: Vec<[f64; 2]> = p
                .vertices
                .iter()
                .map(|&i| {
                    let q = local(pos(i));
                    [q[0], q[1]]
                })
                .collect();
            let ang = min_rect_angle(&face_flat).to_degrees();
            Level {
                index: k,
                height: dot(sub(p.plane.center, o), n),
                up: dot(p.plane.normal, n) > 0.0,
                area: p.area,
                lo: [l[0] - lo[0], l[1] - lo[1]],
                hi: [h[0] - lo[0], h[1] - lo[1]],
                aligned: ang <= 1.0 || ang >= 89.0,
            }
        })
        .collect();
    let up_heights: Vec<f64> = levels.iter().filter(|l| l.up).map(|l| l.height).collect();
    let down_heights: Vec<f64> = std::iter::once(0.0)
        .chain(levels.iter().filter(|l| !l.up).map(|l| l.height))
        .collect();
    let near = |hs: &[f64], h: f64| hs.iter().any(|&x| (x - h).abs() <= tol);

    for (k, p) in planes.iter().enumerate() {
        let ang = acute_deg(p.plane.normal, n);
        if k != bi && ang > SQUARE_DEG && ang < 90.0 - SQUARE_DEG {
            losses.push(Loss {
                severity: "dropped",
                title: "Slanted faces not rebuilt",
                subject: format!("face {}", k + 1),
                message: format!("a {:.2} mm² face at {ang:.1}° to the base (a chamfer or draft) is not part of the rebuild", p.area),
                code: "slanted_face",
            });
        }
    }

    // Round regions square to the base: holes (surface facing the axis) and bosses (facing away).
    let mut holes: Vec<Value> = Vec::new();
    let mut bosses: Vec<Value> = Vec::new();
    // Flat faces a round feature explains: a boss's top, a blind hole's floor.
    let mut explained: Vec<usize> = Vec::new();
    let mark = |explained: &mut Vec<usize>, x: f64, y: f64, r: f64, h: f64| {
        for l in &levels {
            let (cx, cy) = ((l.lo[0] + l.hi[0]) / 2.0, (l.lo[1] + l.hi[1]) / 2.0);
            if (l.height - h).abs() <= tol
                && (cx - x).hypot(cy - y) <= r * 0.5
                && l.area <= std::f64::consts::PI * r * r * 1.05
            {
                explained.push(l.index);
            }
        }
    };
    let mut down_hole_area = 0.0;
    for (k, cy) in cylinders.iter().enumerate() {
        let f = &cy.cylinder;
        let subject = format!("round {} (Ø{:.3} mm)", k + 1, f.diameter);
        let drop = |losses: &mut Vec<Loss>, title, code, message: String| {
            losses.push(Loss {
                severity: "dropped",
                title,
                subject: subject.clone(),
                message,
                code,
            });
        };
        if acute_deg(f.axis, n) > SQUARE_DEG {
            drop(
                &mut losses,
                "Round features not square to the base",
                "tilted_round",
                format!(
                    "its axis is {:.1}° off the base normal",
                    acute_deg(f.axis, n)
                ),
            );
            continue;
        }
        let Some(ax) = normalize(f.axis) else {
            continue;
        };
        let mut facing = 0.0;
        let (mut t0, mut t1) = (f64::INFINITY, f64::NEG_INFINITY);
        for &i in &cy.vertices {
            let p = pos(i);
            let d = sub(p, f.axis_point);
            if let Some(r) = normalize(sub(d, scale(ax, dot(d, ax)))) {
                facing += dot(graph.vertex_normals[i as usize].map(f64::from), r);
            }
            let z = local(p)[2];
            t0 = t0.min(z);
            t1 = t1.max(z);
        }
        let at = to_part(local(f.axis_point));
        let (x, y, r) = (at[0], at[1], f.diameter / 2.0);
        if x - r < -tol || y - r < -tol || x + r > len + tol || y + r > wid + tol {
            drop(
                &mut losses,
                "Round features outside the plate",
                "round_outside",
                "it does not lie wholly inside the base outline".into(),
            );
            continue;
        }
        if facing < 0.0 {
            let (top, bottom) = (near(&up_heights, t1), near(&down_heights, t0));
            let (z, height, kind) = match (bottom, top) {
                (true, true) => (t0 - margin, t1 - t0 + 2.0 * margin, "through"),
                (false, true) => (t0, t1 - t0 + margin, "blind from above"),
                (true, false) => (t0 - margin, t1 - t0 + margin, "blind from below"),
                (false, false) => {
                    drop(
                        &mut losses,
                        "Holes that open on no face",
                        "hole_internal",
                        format!(
                            "its wall runs from {t0:.3} to {t1:.3} mm and reaches no flat face"
                        ),
                    );
                    continue;
                }
            };
            if bottom && t0.abs() <= tol {
                down_hole_area += std::f64::consts::PI * r * r;
            }
            if !bottom {
                mark(&mut explained, x, y, r, t0); // the floor of a hole blind from above
            } else if !top {
                mark(&mut explained, x, y, r, t1); // the end of a hole blind from below
            }
            holes.push(json!({"x": x, "y": y, "z": z, "height": height, "diameter": f.diameter,
                              "kind": kind, "from": t0, "to": t1, "sigma": f.sigma, "coverage_deg": f.coverage_deg}));
        } else if near(&up_heights, t0) && t0 > tol {
            // Standing on an upward face (the top, a pocket floor or a pad); joined from just inside it.
            mark(&mut explained, x, y, r, t1);
            bosses.push(
                json!({"x": x, "y": y, "z": t0 - margin, "height": t1 - t0 + margin,
                               "standing_height": t1 - t0, "diameter": f.diameter, "on": t0,
                               "sigma": f.sigma, "coverage_deg": f.coverage_deg}),
            );
        } else if t0.abs() <= tol && (t1 - thick).abs() <= tol {
            drop(&mut losses, "Round outlines not rebuilt", "round_outline",
                 "its outer wall runs the full thickness: the outline is round, and the plate uses its enclosing rectangle".into());
        } else {
            drop(
                &mut losses,
                "Bosses and pins not rebuilt",
                "boss",
                format!(
                    "it faces away from its axis but does not stand on an upward face (it runs from {t0:.3} to {t1:.3} mm)"
                ),
            );
        }
    }

    // The other parallel faces: pocket and step floors (cut), pads (joined), undercuts (reported).
    let mut pockets: Vec<Value> = Vec::new();
    let mut pads: Vec<Value> = Vec::new();
    for l in &levels {
        if l.index == ti || explained.contains(&l.index) {
            continue;
        }
        let subject = format!("face {} at {:.3} mm", l.index + 1, l.height);
        let (w, d) = (l.hi[0] - l.lo[0], l.hi[1] - l.lo[1]);
        let rect = json!({"x": l.lo[0], "y": l.lo[1], "length": w, "width": d, "height": l.height, "area_mm2": l.area});
        let simplified = l.area < 0.98 * w * d;
        if l.up && l.height > tol && l.height < thick - tol {
            if !l.aligned {
                losses.push(Loss {
                    severity: "dropped",
                    title: "Pockets turned to the outline not rebuilt",
                    subject,
                    message: "its edges are not square to the plate's".into(),
                    code: "pocket_rotated",
                });
                continue;
            }
            if simplified {
                losses.push(Loss {
                    severity: "degraded",
                    title: "Pocket outlines simplified to a rectangle",
                    subject: subject.clone(),
                    message: format!(
                        "the floor covers {:.1} mm² of its {:.1} mm² rectangle",
                        l.area,
                        w * d
                    ),
                    code: "pocket_rectangle",
                });
            }
            pockets.push(rect);
        } else if l.up && l.height > thick + tol {
            if !l.aligned {
                losses.push(Loss {
                    severity: "dropped",
                    title: "Pads turned to the outline not rebuilt",
                    subject,
                    message: "its edges are not square to the plate's".into(),
                    code: "pad_rotated",
                });
                continue;
            }
            if simplified {
                losses.push(Loss {
                    severity: "degraded",
                    title: "Pad outlines simplified to a rectangle",
                    subject: subject.clone(),
                    message: format!(
                        "the top covers {:.1} mm² of its {:.1} mm² rectangle",
                        l.area,
                        w * d
                    ),
                    code: "pad_rectangle",
                });
            }
            pads.push(rect);
        } else if !l.up && l.height > tol && l.height < thick - tol {
            losses.push(Loss {
                severity: "dropped",
                title: "Undercuts not rebuilt",
                subject,
                message: "a downward face inside the plate (an overhang or undercut)".into(),
                code: "undercut",
            });
        } else if (l.up && (l.height - thick).abs() > tol) || (!l.up && l.height.abs() > tol) {
            losses.push(Loss {
                severity: "dropped",
                title: "Faces below the base not rebuilt",
                subject,
                message: "a flat face outside the plate's thickness on the base side".into(),
                code: "below_base",
            });
        }
    }

    // An outline that is not a full rectangle is simplified to its enclosing rectangle.
    let expected = len * wid - down_hole_area;
    if base.area < 0.98 * expected {
        losses.push(Loss {
            severity: "degraded",
            title: "Outline simplified to a rectangle",
            subject: "base outline".into(),
            message: format!(
                "the base face covers {:.1} mm² of the {:.1} mm² its rectangle leaves after holes",
                base.area, expected
            ),
            code: "outline_rectangle",
        });
    }

    // The design: parameters for the sizes a person would edit, then the features in build order.
    let param = |name: &str, v: f64| json!({"name": name, "expr": format!("{} mm", mm(v)), "unit": "mm", "model": false});
    let mut params = vec![
        param("length", len),
        param("width", wid),
        param("thickness", thick),
    ];
    let mut features: Vec<Value> = vec![json!({
        "id": 1, "name": "Plate", "suppressed": false, "body_names": ["Plate"], "param_names": [],
        "type": "box", "corner": [0.0, 0.0, 0.0], "length": "length", "width": "width", "height": "thickness",
        "operation": "new_body",
    })];
    let f64of = |v: &Value, k: &str| v[k].as_f64().unwrap_or(0.0);
    let mut add = |mut f: Value| {
        f["id"] = json!(features.len() + 1);
        f["suppressed"] = json!(false);
        f["param_names"] = json!([]);
        features.push(f);
    };
    for (i, p) in pads.iter().enumerate() {
        let h = format!("pad{}_height", i + 1);
        params.push(param(&h, f64of(p, "height") - thick));
        add(json!({"name": format!("Pad{}", i + 1), "type": "box",
            "corner": [mm(f64of(p, "x")), mm(f64of(p, "y")), format!("thickness - {}", mm(margin))],
            "length": mm(f64of(p, "length")), "width": mm(f64of(p, "width")),
            "height": format!("{h} + {}", mm(margin)), "operation": "join"}));
    }
    for (i, b) in bosses.iter().enumerate() {
        let (dp, hp) = (
            format!("boss{}_diameter", i + 1),
            format!("boss{}_height", i + 1),
        );
        params.push(param(&dp, f64of(b, "diameter")));
        params.push(param(&hp, f64of(b, "standing_height")));
        add(json!({"name": format!("Boss{}", i + 1), "type": "cylinder",
            "base": [round_um(f64of(b, "x")), round_um(f64of(b, "y")), round_um(f64of(b, "z"))],
            "axis": [0.0, 0.0, 1.0], "radius": format!("{dp} / 2"), "height": format!("{hp} + {}", mm(margin)),
            "operation": "join"}));
    }
    for (i, p) in pockets.iter().enumerate() {
        let dp = format!("pocket{}_depth", i + 1);
        params.push(param(&dp, thick - f64of(p, "height")));
        add(json!({"name": format!("Pocket{}", i + 1), "type": "box",
            "corner": [mm(f64of(p, "x")), mm(f64of(p, "y")), format!("thickness - {dp}")],
            "length": mm(f64of(p, "length")), "width": mm(f64of(p, "width")),
            "height": format!("{dp} + {}", mm(margin)), "operation": "cut"}));
    }
    for (i, h) in holes.iter().enumerate() {
        let p = format!("hole{}_diameter", i + 1);
        params.push(param(&p, f64of(h, "diameter")));
        add(json!({"name": format!("Hole{}", i + 1), "type": "cylinder",
            "base": [round_um(f64of(h, "x")), round_um(f64of(h, "y")), round_um(f64of(h, "z"))],
            "axis": [0.0, 0.0, 1.0], "radius": format!("{p} / 2"), "height": mm(f64of(h, "height")),
            "operation": "cut"}));
    }
    let next_id = features.len() + 1;
    let design = json!({
        "format": FORMAT, "name": name, "units": "mm", "params": params, "features": features,
        "next_id": next_id, "next_component_color_index": 0,
    });

    let count =
        |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
    let mut parts = vec![count(holes.len(), "hole", "holes")];
    for (n, one, many) in [
        (bosses.len(), "boss", "bosses"),
        (pockets.len(), "pocket", "pockets"),
        (pads.len(), "pad", "pads"),
    ] {
        if n > 0 {
            parts.push(count(n, one, many));
        }
    }
    let summary = format!(
        "A {} x {} x {} mm plate with {}, from the scan's {} flat and {} round regions",
        mm(len),
        mm(wid),
        mm(thick),
        parts.join(", "),
        planes.len(),
        cylinders.len()
    );
    let report = json!({
        "summary": summary,
        "items": losses.iter().map(Loss::json).collect::<Vec<_>>(),
        "details": {
            "base": {"area_mm2": base.area, "sigma": base.plane.sigma, "form_error": base.plane.form_error},
            "plate": {"length": len, "width": wid, "thickness": thick},
            "holes": holes, "bosses": bosses, "pockets": pockets, "pads": pads,
            "frame": {"origin": o, "u": u, "v": v, "normal": n, "outline_min": lo, "z0": 0.0},
        },
    });
    Ok(Rebuilt { design, report })
}
