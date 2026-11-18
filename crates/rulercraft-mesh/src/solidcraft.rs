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
/// The most corners a traced polygon outline may have; a more complex outline stays a rectangle.
const MAX_OUTLINE_CORNERS: usize = 200;
/// A loop needs at least this many points to count as a circle.
const MIN_CIRCLE_POINTS: usize = 12;

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

/// The boundary loops of a set of triangles in the plane, largest enclosed area first (the outline, then the
/// inner loops), or `None` when the boundary is not a set of simple closed chains.
fn boundary_loops(
    tris: &[u32],
    mesh: &Mesh,
    flat: &dyn Fn(u32) -> [f64; 2],
) -> Option<Vec<Vec<[f64; 2]>>> {
    use std::collections::{HashMap, HashSet};
    let mut directed: HashSet<(u32, u32)> = HashSet::new();
    for &t in tris {
        let [a, b, c] = mesh.triangles[t as usize];
        directed.extend([(a, b), (b, c), (c, a)]);
    }
    let mut next: HashMap<u32, u32> = HashMap::new();
    for &(a, b) in &directed {
        if !directed.contains(&(b, a)) && next.insert(a, b).is_some() {
            return None; // a vertex where two boundary chains touch
        }
    }
    let mut found: Vec<(f64, Vec<[f64; 2]>)> = Vec::new();
    let mut seen: HashSet<u32> = HashSet::new();
    let mut starts: Vec<u32> = next.keys().copied().collect();
    starts.sort_unstable();
    for s in starts {
        if seen.contains(&s) {
            continue;
        }
        let mut chain = vec![s];
        seen.insert(s);
        let mut at = *next.get(&s)?;
        while at != s {
            if !seen.insert(at) {
                return None;
            }
            chain.push(at);
            at = *next.get(&at)?;
        }
        let pts: Vec<[f64; 2]> = chain.iter().map(|&i| flat(i)).collect();
        found.push((signed_area(&pts).abs(), pts));
    }
    found.sort_by(|a, b| b.0.total_cmp(&a.0));
    Some(found.into_iter().map(|f| f.1).collect())
}

fn signed_area(p: &[[f64; 2]]) -> f64 {
    let n = p.len();
    (0..n)
        .map(|i| p[i][0] * p[(i + 1) % n][1] - p[(i + 1) % n][0] * p[i][1])
        .sum::<f64>()
        / 2.0
}

/// Distance from `q` to the segment `a`-`b`.
fn seg_dist(q: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (q[0] - a[0] - t * dx).hypot(q[1] - a[1] - t * dy)
}

/// A closed loop reduced to the corners that matter: every dropped point lies within `tol` of the result.
fn simplify_loop(p: &[[f64; 2]], tol: f64) -> Vec<[f64; 2]> {
    let n = p.len();
    if n < 4 {
        return p.to_vec();
    }
    // Split at the point farthest from the first, so neither half is a loop.
    let far = (1..n)
        .max_by(|&a, &b| {
            let d = |i: usize| (p[i][0] - p[0][0]).hypot(p[i][1] - p[0][1]);
            d(a).total_cmp(&d(b))
        })
        .unwrap_or(1);
    fn dp(p: &[[f64; 2]], lo: usize, hi: usize, tol: f64, keep: &mut Vec<usize>) {
        let (mut worst, mut at) = (0.0, None);
        for i in lo + 1..hi {
            let d = seg_dist(p[i], p[lo], p[hi % p.len()]);
            if d > worst {
                (worst, at) = (d, Some(i));
            }
        }
        if let (true, Some(i)) = (worst > tol, at) {
            dp(p, lo, i, tol, keep);
            keep.push(i);
            dp(p, i, hi, tol, keep);
        }
    }
    let mut keep = vec![0];
    dp(p, 0, far, tol, &mut keep);
    keep.push(far);
    dp(p, far, n, tol, &mut keep);
    let mut out: Vec<[f64; 2]> = keep.into_iter().map(|i| p[i]).collect();
    // The split points are arbitrary vertices, perhaps mid-edge: drop any corner that is not one.
    loop {
        let m = out.len();
        let flat = (0..m)
            .find(|&i| m > 3 && seg_dist(out[i], out[(i + m - 1) % m], out[(i + 1) % m]) <= tol);
        match flat {
            Some(i) => {
                out.remove(i);
            }
            None => break,
        }
    }
    out
}

/// The circle through a loop's points, when they all lie within `tol` of one: `(cx, cy, r)`.
fn circle_of(p: &[[f64; 2]], tol: f64) -> Option<(f64, f64, f64)> {
    if p.len() < MIN_CIRCLE_POINTS {
        return None; // four corners always lie on a circle: a rectangle is not one
    }
    let n = p.len() as f64;
    let (cx, cy) = (
        p.iter().map(|q| q[0]).sum::<f64>() / n,
        p.iter().map(|q| q[1]).sum::<f64>() / n,
    );
    // Kasa fit on the centred points.
    let (mut suu, mut svv, mut suv, mut suuu, mut svvv, mut suvv, mut svuu) =
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for q in p {
        let (u, v) = (q[0] - cx, q[1] - cy);
        suu += u * u;
        svv += v * v;
        suv += u * v;
        suuu += u * u * u;
        svvv += v * v * v;
        suvv += u * v * v;
        svuu += v * u * u;
    }
    let det = suu * svv - suv * suv;
    if det.abs() < 1e-12 {
        return None;
    }
    let (b1, b2) = ((suuu + suvv) / 2.0, (svvv + svuu) / 2.0);
    let (uc, vc) = ((b1 * svv - b2 * suv) / det, (suu * b2 - suv * b1) / det);
    let r = (uc * uc + vc * vc + (suu + svv) / n).sqrt();
    let worst = p
        .iter()
        .map(|q| ((q[0] - cx - uc).hypot(q[1] - cy - vc) - r).abs())
        .fold(0.0, f64::max);
    (worst <= tol && r.is_finite()).then_some((cx + uc, cy + vc, r))
}

fn point_in_loop(q: [f64; 2], p: &[[f64; 2]]) -> bool {
    let mut inside = false;
    let n = p.len();
    for i in 0..n {
        let (a, b) = (p[i], p[(i + 1) % n]);
        if (a[1] > q[1]) != (b[1] > q[1])
            && q[0] < (b[0] - a[0]) * (q[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}

/// The plate's outline, traced from the base face.
enum Outline {
    Rectangle,
    Circle { x: f64, y: f64, r: f64 },
    Polygon(Vec<[f64; 2]>),
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

    // The outline: the base face's outer boundary, as a circle or a polygon when it is not a rectangle.
    let part_xy = |i: u32| {
        let l = local(pos(i));
        [l[0] - lo[0], l[1] - lo[1]]
    };
    let mut outline = Outline::Rectangle;
    let mut outline_area = len * wid;
    let base_loops = boundary_loops(&base.triangles, mesh, &part_xy).unwrap_or_default();
    if let Some(ring) = base_loops.first() {
        let area = signed_area(ring).abs();
        if area < 0.98 * len * wid {
            if let Some((x, y, r)) = circle_of(ring, tol) {
                outline_area = std::f64::consts::PI * r * r;
                outline = Outline::Circle { x, y, r };
            } else {
                let mut poly = simplify_loop(ring, tol);
                let poly_area = signed_area(&poly);
                if (3..=MAX_OUTLINE_CORNERS).contains(&poly.len()) && poly_area.abs() > 0.0 {
                    if poly_area < 0.0 {
                        poly.reverse(); // counter-clockwise in the part frame
                    }
                    let short = (0..poly.len())
                        .filter(|&i| {
                            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
                            (b[0] - a[0]).hypot(b[1] - a[1]) < 6.0 * tol
                        })
                        .count();
                    if short >= 3 {
                        losses.push(Loss {
                            severity: "degraded",
                            title: "Curved outline built from straight edges",
                            subject: "base outline".into(),
                            message: format!("{short} very short edges: a curved edge of the outline is a run of straight ones"),
                            code: "outline_faceted",
                        });
                    }
                    outline_area = poly_area.abs();
                    outline = Outline::Polygon(poly);
                }
            }
        }
    }
    let inside_outline = |x: f64, y: f64, r: f64| match &outline {
        Outline::Rectangle => {
            !(x - r < -tol || y - r < -tol || x + r > len + tol || y + r > wid + tol)
        }
        Outline::Circle {
            x: cx,
            y: cy,
            r: big,
        } => (x - cx).hypot(y - cy) + r <= big + tol,
        Outline::Polygon(p) => {
            point_in_loop([x, y], p)
                && (0..p.len()).all(|i| seg_dist([x, y], p[i], p[(i + 1) % p.len()]) >= r - tol)
        }
    };

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
        if !inside_outline(x, y, r) {
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
            if !matches!(outline, Outline::Rectangle) {
                continue; // a wall of the traced outline: a curved edge, already part of the plate
            }
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

    // Cutouts through the plate that are not round: an inner loop of the base that the top face repeats.
    let mut slots: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut slot_area = 0.0;
    let top_loops = boundary_loops(&planes[ti].triangles, mesh, &part_xy).unwrap_or_default();
    let centroid = |p: &[[f64; 2]]| {
        let n = p.len() as f64;
        [
            p.iter().map(|q| q[0]).sum::<f64>() / n,
            p.iter().map(|q| q[1]).sum::<f64>() / n,
        ]
    };
    for inner in base_loops.iter().skip(1) {
        if circle_of(inner, tol).is_some() {
            continue; // a round hole: its wall is a round region, cut above
        }
        let area = signed_area(inner).abs();
        let c = centroid(inner);
        let matched = top_loops.iter().skip(1).any(|t| {
            let (tc, ta) = (centroid(t), signed_area(t).abs());
            (tc[0] - c[0]).hypot(tc[1] - c[1]) <= 2.0 * tol && (ta - area).abs() <= 0.03 * area
        });
        let poly = simplify_loop(inner, tol);
        if matched
            && (3..=MAX_OUTLINE_CORNERS).contains(&poly.len())
            && signed_area(&poly).abs() > 0.0
        {
            slot_area += signed_area(&poly).abs();
            slots.push(poly);
        }
    }

    // An outline that is not a full rectangle is simplified to its enclosing rectangle.
    let expected = outline_area - down_hole_area - slot_area;
    if base.area < 0.98 * expected {
        let rectangle = matches!(outline, Outline::Rectangle);
        losses.push(Loss {
            severity: "degraded",
            title: if rectangle {
                "Outline simplified to a rectangle"
            } else {
                "Cutouts in the base not rebuilt"
            },
            subject: "base outline".into(),
            message: format!(
                "the base face covers {:.1} mm² of the {:.1} mm² its {} leaves after holes",
                base.area,
                expected,
                if rectangle { "rectangle" } else { "outline" }
            ),
            code: if rectangle {
                "outline_rectangle"
            } else {
                "cutout_not_round"
            },
        });
    }

    // The design: parameters for the sizes a person would edit, then the features in build order.
    let param = |name: &str, v: f64| json!({"name": name, "expr": format!("{} mm", mm(v)), "unit": "mm", "model": false});
    let mut params = vec![
        param("length", len),
        param("width", wid),
        param("thickness", thick),
    ];
    let mut features: Vec<Value> = match &outline {
        Outline::Rectangle => vec![json!({
            "id": 1, "name": "Plate", "suppressed": false, "body_names": ["Plate"], "param_names": [],
            "type": "box", "corner": [0.0, 0.0, 0.0], "length": "length", "width": "width", "height": "thickness",
            "operation": "new_body",
        })],
        traced => {
            let (points, curves, loop_ids) = match traced {
                Outline::Circle { x, y, r } => (
                    vec![
                        json!({"fixed": true, "id": "origin", "pos": [0.0, 0.0]}),
                        json!({"fixed": false, "id": "c1.center", "pos": [round_um(*x), round_um(*y)]}),
                    ],
                    vec![
                        json!({"c": 1, "construction": false, "id": "c1", "r": round_um(*r), "type": "circle"}),
                    ],
                    vec!["c1".to_string()],
                ),
                Outline::Polygon(p) => {
                    let n = p.len();
                    let mut pts = vec![json!({"fixed": true, "id": "origin", "pos": [0.0, 0.0]})];
                    for (i, q) in p.iter().enumerate() {
                        pts.push(json!({"fixed": false, "id": format!("p{}", i + 1), "pos": [round_um(q[0]), round_um(q[1])]}));
                    }
                    let lines = (0..n)
                        .map(|i| json!({"a": 1 + i, "b": 1 + (i + 1) % n, "construction": false, "id": format!("l{}", i + 1), "type": "line"}))
                        .collect();
                    (pts, lines, (0..n).map(|i| format!("l{}", i + 1)).collect())
                }
                Outline::Rectangle => (vec![], vec![], vec![]),
            };
            let counters = match traced {
                Outline::Circle { .. } => json!({"c": 1, "k": 0}),
                _ => json!({"k": 0, "l": curves.len(), "p": points.len() - 1}),
            };
            vec![
                json!({"id": 1, "name": "Outline", "suppressed": false, "param_names": [], "type": "sketch",
                    "plane": {"name": "XY", "type": "origin"},
                    "sketch": {"constraints": [], "counters": counters, "curves": curves, "points": points}}),
                json!({"id": 2, "name": "Plate", "suppressed": false, "body_names": ["Plate"], "param_names": [],
                    "type": "extrude", "sketch": 1, "operation": "new_body",
                    "profiles": {"type": "curves", "loops": [loop_ids]},
                    "extent": {"convention": "total_distance", "direction": "positive", "distance": "thickness"}}),
            ]
        }
    };
    for (i, poly) in slots.iter().enumerate() {
        let n = poly.len();
        let sid = features.len() + 1;
        let mut pts = vec![json!({"fixed": true, "id": "origin", "pos": [0.0, 0.0]})];
        for (k, q) in poly.iter().enumerate() {
            pts.push(json!({"fixed": false, "id": format!("p{}", k + 1), "pos": [round_um(q[0]), round_um(q[1])]}));
        }
        let lines: Vec<Value> = (0..n)
            .map(|k| json!({"a": 1 + k, "b": 1 + (k + 1) % n, "construction": false, "id": format!("l{}", k + 1), "type": "line"}))
            .collect();
        let ids: Vec<String> = (0..n).map(|k| format!("l{}", k + 1)).collect();
        features.push(json!({"id": sid, "name": format!("Cutout{}_Outline", i + 1), "suppressed": false, "param_names": [],
            "type": "sketch", "plane": {"name": "XY", "type": "origin"},
            "sketch": {"constraints": [], "counters": {"k": 0, "l": n, "p": n}, "curves": lines, "points": pts}}));
        features.push(json!({"id": sid + 1, "name": format!("Cutout{}", i + 1), "suppressed": false, "param_names": [],
            "type": "extrude", "sketch": sid, "operation": "cut", "targets": ["Plate"],
            "profiles": {"type": "curves", "loops": [ids]},
            "extent": {"convention": "total_distance", "direction": "positive", "distance": format!("thickness + {}", mm(margin))}}));
    }
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
            "cutouts": slots, "holes": holes, "bosses": bosses, "pockets": pockets, "pads": pads,
            "frame": {"origin": o, "u": u, "v": v, "normal": n, "outline_min": lo, "z0": 0.0},
        },
    });
    Ok(Rebuilt { design, report })
}

#[cfg(test)]
#[allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn area_of_an_l_is_1600() {
        let l = [
            [0.0, 0.0],
            [60.0, 0.0],
            [60.0, 20.0],
            [20.0, 20.0],
            [20.0, 40.0],
            [0.0, 40.0],
        ];
        assert!((signed_area(&l) - 1600.0).abs() < 1e-9);
        let rev: Vec<_> = l.iter().rev().copied().collect();
        assert!((signed_area(&rev) + 1600.0).abs() < 1e-9);
    }

    #[test]
    fn midpoints_on_edges_are_not_corners() {
        // A 10 x 4 rectangle with a point every 1 mm along each edge: four corners remain.
        let mut p = Vec::new();
        for x in 0..10 {
            p.push([f64::from(x), 0.0]);
        }
        for y in 0..4 {
            p.push([10.0, f64::from(y)]);
        }
        for x in (1..=10).rev() {
            p.push([f64::from(x), 4.0]);
        }
        for y in (1..=4).rev() {
            p.push([0.0, f64::from(y)]);
        }
        let s = simplify_loop(&p, 0.01);
        assert_eq!(s.len(), 4, "{s:?}");
        assert!((signed_area(&s).abs() - 40.0).abs() < 1e-9);
    }

    #[test]
    fn a_circle_of_radius_25_is_found_and_a_square_is_not() {
        let c: Vec<[f64; 2]> = (0..90)
            .map(|i| {
                let a = f64::from(i) * std::f64::consts::TAU / 90.0;
                [3.0 + 25.0 * a.cos(), -4.0 + 25.0 * a.sin()]
            })
            .collect();
        let (x, y, r) = circle_of(&c, 1e-6).unwrap();
        assert!((x - 3.0).abs() < 1e-6 && (y + 4.0).abs() < 1e-6 && (r - 25.0).abs() < 1e-6);
        let sq: Vec<[f64; 2]> = (0..40)
            .map(|i| match i / 10 {
                0 => [f64::from(i % 10), 0.0],
                1 => [10.0, f64::from(i % 10)],
                2 => [10.0 - f64::from(i % 10), 10.0],
                _ => [0.0, 10.0 - f64::from(i % 10)],
            })
            .collect();
        assert!(circle_of(&sq, 0.05).is_none());
    }

    #[test]
    fn a_hole_near_an_inner_corner_is_outside_the_l() {
        let l = [
            [0.0, 0.0],
            [60.0, 0.0],
            [60.0, 20.0],
            [20.0, 20.0],
            [20.0, 40.0],
            [0.0, 40.0],
        ];
        assert!(point_in_loop([10.0, 30.0], &l));
        assert!(!point_in_loop([40.0, 30.0], &l));
        assert!((seg_dist([10.0, 30.0], l[3], l[4]) - 10.0).abs() < 1e-9);
    }
}
