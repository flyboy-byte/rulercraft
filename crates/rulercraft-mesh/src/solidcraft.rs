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
use crate::segment::{segment_cylinders_seeded, segment_planes};
use serde_json::{Value, json};

/// The SolidCraft design format written. Later SolidCraft versions upgrade it when they open it.
pub const FORMAT: &str = "solidcraft/57";
/// A face or axis within this many degrees counts as parallel or square to the base.
pub const SQUARE_DEG: f64 = 5.0;
/// The most corners a traced polygon outline may have; a more complex outline stays a rectangle.
const MAX_OUTLINE_CORNERS: usize = 200;
/// At most this many of the scan's points are measured against the rebuilt part.
const MAX_CHECK_POINTS: usize = 200_000;
/// A loop needs at least this many points to count as a circle.
const MIN_CIRCLE_POINTS: usize = 12;
/// Flat regions with at least this many triangles seed no round region.
const BIG_FACE_TRIANGLES: usize = 200;
/// A round square to the base covering less of a circle than this is a rounded vertical edge, not a hole or boss.
const MIN_ROUND_COVERAGE_DEG: f64 = 200.0;

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
/// The direction of the walls square to the base, in the base's `(u0, v0)` axes: the area-weighted mean of their
/// facings taken four-fold, so walls at right angles agree. `None` when there are no such walls or fewer than half
/// their area agrees within 1° (a hexagon, say), and the enclosing rectangle decides instead. On a scan the base
/// face's ragged boundary can turn the smallest rectangle by a fraction of a degree; the walls do not.
fn wall_angle(
    planes: &[crate::segment::PlaneRegion],
    n: [f64; 3],
    u0: [f64; 3],
    v0: [f64; 3],
) -> Option<f64> {
    let square: Vec<&crate::segment::PlaneRegion> = planes
        .iter()
        .filter(|p| (acute_deg(p.plane.normal, n) - 90.0).abs() <= SQUARE_DEG)
        .collect();
    // Facets of round holes and bosses are small flat walls at every angle: only walls of a tenth of the largest count.
    let largest = square.iter().map(|p| p.area).fold(0.0, f64::max);
    let walls: Vec<(f64, f64)> = square
        .iter()
        .filter(|p| p.area >= 0.1 * largest)
        .map(|p| {
            let (x, y) = (dot(p.plane.normal, u0), dot(p.plane.normal, v0));
            (p.area, 4.0 * y.atan2(x))
        })
        .collect();
    let total: f64 = walls.iter().map(|w| w.0).sum();
    if walls.is_empty() || total <= 0.0 {
        return None;
    }
    let near = |a: f64, b: f64| (a - b).sin().atan2((a - b).cos()).abs() <= 4f64.to_radians();
    // The peak: the wall whose neighbours within 1° (4° four-fold) hold the most area.
    let peak = walls
        .iter()
        .map(|w| {
            (
                walls
                    .iter()
                    .filter(|o| near(o.1, w.1))
                    .map(|o| o.0)
                    .sum::<f64>(),
                w.1,
            )
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))?;
    if peak.0 < 0.5 * total {
        return None;
    }
    let (sx, sy) = walls
        .iter()
        .filter(|o| near(o.1, peak.1))
        .fold((0.0, 0.0), |(x, y), o| {
            (x + o.0 * o.1.cos(), y + o.0 * o.1.sin())
        });
    Some((sy.atan2(sx) / 4.0).rem_euclid(std::f64::consts::FRAC_PI_2))
}

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
    // Boundary edges by their start; a vertex where boundary chains touch (common on a noisy scan) has several, and
    // each is walked once, taking the unused edges in order.
    let mut next: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut edges: Vec<(u32, u32)> = directed
        .iter()
        .copied()
        .filter(|&(a, b)| !directed.contains(&(b, a)))
        .collect();
    edges.sort_unstable();
    for &(a, b) in &edges {
        next.entry(a).or_default().push(b);
    }
    let mut found: Vec<(f64, Vec<[f64; 2]>)> = Vec::new();
    for &(s, first) in &edges {
        if !next.get(&s).is_some_and(|v| v.contains(&first)) {
            continue; // already walked
        }
        let take = |next: &mut HashMap<u32, Vec<u32>>, from: u32, to: Option<u32>| -> Option<u32> {
            let v = next.get_mut(&from)?;
            let k = match to {
                Some(t) => v.iter().position(|&x| x == t)?,
                None => 0,
            };
            (k < v.len()).then(|| v.swap_remove(k))
        };
        let mut chain = vec![s];
        let mut at = take(&mut next, s, Some(first))?;
        let mut steps = 0usize;
        while at != s {
            chain.push(at);
            steps += 1;
            if steps > edges.len() {
                return None;
            }
            at = match take(&mut next, at, None) {
                Some(n) => n,
                None => break, // an open chain: not a loop
            };
        }
        if at == s && chain.len() >= 3 {
            let pts: Vec<[f64; 2]> = chain.iter().map(|&i| flat(i)).collect();
            found.push((signed_area(&pts).abs(), pts));
        }
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

/// The least-squares circle through points (Kasa): `(cx, cy, r)`.
fn fit_circle(p: &[[f64; 2]]) -> Option<(f64, f64, f64)> {
    let n = p.len() as f64;
    let (cx, cy) = (
        p.iter().map(|q| q[0]).sum::<f64>() / n,
        p.iter().map(|q| q[1]).sum::<f64>() / n,
    );
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
    if det.is_nan() || det.abs() < 1e-12 {
        return None;
    }
    let (b1, b2) = ((suuu + suvv) / 2.0, (svvv + svuu) / 2.0);
    let (uc, vc) = ((b1 * svv - b2 * suv) / det, (suu * b2 - suv * b1) / det);
    let r = (uc * uc + vc * vc + (suu + svv) / n).sqrt();
    r.is_finite().then_some((cx + uc, cy + vc, r))
}

/// The circle through a loop's points, when they all lie within `tol` of one: `(cx, cy, r)`.
fn circle_of(p: &[[f64; 2]], tol: f64) -> Option<(f64, f64, f64)> {
    if p.len() < MIN_CIRCLE_POINTS {
        return None; // four corners always lie on a circle: a rectangle is not one
    }
    let (cx, cy, r) = fit_circle(p)?;
    p.iter()
        .all(|q| ((q[0] - cx).hypot(q[1] - cy) - r).abs() <= tol)
        .then_some((cx, cy, r))
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

/// The rebuilt part as a signed-distance field in the part frame: negative inside the material.
/// Unions and cuts are combined by min and max, which is exact on faces and a slight underestimate
/// near the edges where two features meet; fine for judging how far a scan lies from the rebuild.
struct Solid {
    /// Parts of the body: a 2D shape extruded between two heights.
    body: Vec<(Shape, f64, f64)>,
    /// Material taken away.
    cuts: Vec<(Shape, f64, f64)>,
    /// Material taken away by cones (countersinks).
    cones: Vec<Cone>,
    /// Material taken away by bevels along top edges.
    chamfers: Vec<Chamfer>,
}

enum Shape {
    Circle(f64, f64, f64),
    Rect([f64; 2], [f64; 2]),
    Poly(Vec<[f64; 2]>),
}

impl Shape {
    fn dist(&self, q: [f64; 2]) -> f64 {
        match self {
            Shape::Circle(x, y, r) => (q[0] - x).hypot(q[1] - y) - r,
            Shape::Rect(lo, hi) => {
                let c = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
                let h = [(hi[0] - lo[0]) / 2.0, (hi[1] - lo[1]) / 2.0];
                let d = [(q[0] - c[0]).abs() - h[0], (q[1] - c[1]).abs() - h[1]];
                d[0].max(0.0).hypot(d[1].max(0.0)) + d[0].max(d[1]).min(0.0)
            }
            Shape::Poly(p) => {
                let e = (0..p.len())
                    .map(|i| seg_dist(q, p[i], p[(i + 1) % p.len()]))
                    .fold(f64::INFINITY, f64::min);
                if point_in_loop(q, p) { -e } else { e }
            }
        }
    }
}

impl Solid {
    fn extruded(shape: &Shape, z0: f64, z1: f64, p: [f64; 3]) -> f64 {
        let (d, dz) = (
            shape.dist([p[0], p[1]]),
            (p[2] - (z0 + z1) / 2.0).abs() - (z1 - z0) / 2.0,
        );
        d.max(dz).min(0.0) + d.max(0.0).hypot(dz.max(0.0))
    }

    fn distance(&self, p: [f64; 3]) -> f64 {
        let body = self
            .body
            .iter()
            .map(|(s, a, b)| Self::extruded(s, *a, *b, p))
            .fold(f64::INFINITY, f64::min);
        let cut = self
            .cuts
            .iter()
            .map(|(s, a, b)| Self::extruded(s, *a, *b, p))
            .fold(f64::INFINITY, f64::min)
            .min(
                self.cones
                    .iter()
                    .map(|c| c.dist(p))
                    .fold(f64::INFINITY, f64::min),
            )
            .min(
                self.chamfers
                    .iter()
                    .map(|c| c.dist(p))
                    .fold(f64::INFINITY, f64::min),
            );
        body.max(-cut)
    }
}

/// How far the scan's points lie from the rebuilt part.
fn scan_distances(solid: &Solid, points: impl Iterator<Item = [f64; 3]>) -> Vec<f64> {
    points.map(|p| solid.distance(p).abs()).collect()
}

fn check_report(mut d: Vec<f64>, tol: f64) -> Value {
    d.sort_by(f64::total_cmp);
    let n = d.len().max(1) as f64;
    let at = |q: f64| {
        d.get(((d.len() as f64 - 1.0) * q).round() as usize)
            .copied()
            .unwrap_or(0.0)
    };
    let within = d.iter().filter(|&&x| x <= tol).count() as f64 / n;
    json!({"points": d.len(), "mean_mm": d.iter().sum::<f64>() / n,
           "rms_mm": (d.iter().map(|x| x * x).sum::<f64>() / n).sqrt(),
           "p95_mm": at(0.95), "max_mm": d.last().copied().unwrap_or(0.0),
           "within_tolerance": within, "tolerance_mm": tol})
}

/// A cone square to the base: its axis at `(x, y)` in the part frame, radius `r0` at height `z0` and `r1` at `z1`.
#[derive(Debug, Clone, Copy)]
struct Cone {
    x: f64,
    y: f64,
    z0: f64,
    r0: f64,
    z1: f64,
    r1: f64,
}

impl Cone {
    /// Signed distance to the solid frustum (negative inside).
    fn dist(&self, p: [f64; 3]) -> f64 {
        let q = [
            (p[0] - self.x).hypot(p[1] - self.y),
            p[2] - (self.z0 + self.z1) / 2.0,
        ];
        let h = (self.z1 - self.z0) / 2.0;
        let (k1, k2) = ([self.r1, h], [self.r1 - self.r0, 2.0 * h]);
        let ca = [
            q[0] - q[0].min(if q[1] < 0.0 { self.r0 } else { self.r1 }),
            q[1].abs() - h,
        ];
        let d2 = |a: [f64; 2]| a[0] * a[0] + a[1] * a[1];
        let t = (((k1[0] - q[0]) * k2[0] + (k1[1] - q[1]) * k2[1]) / d2(k2)).clamp(0.0, 1.0);
        let cb = [q[0] - k1[0] + k2[0] * t, q[1] - k1[1] + k2[1] * t];
        let sign = if cb[0] < 0.0 && ca[1] < 0.0 {
            -1.0
        } else {
            1.0
        };
        sign * d2(ca).min(d2(cb)).sqrt()
    }
}

/// The cone square to the base that slanted triangles lie on, when they wrap around one axis.
/// `faces` holds each triangle's area, centroid across the base, and its normal's unit direction across the base;
/// `points` the triangles' vertices in the part frame.
fn cone_of(faces: &[(f64, [f64; 2], [f64; 2])], points: &[[f64; 3]], tol: f64) -> Option<Cone> {
    // The axis: the point nearest every line through a triangle along its normal.
    let (mut a, mut b) = ([[0.0; 2]; 2], [0.0; 2]);
    for &(w, c, h) in faces {
        let m = [
            [1.0 - h[0] * h[0], -h[0] * h[1]],
            [-h[0] * h[1], 1.0 - h[1] * h[1]],
        ];
        for i in 0..2 {
            for j in 0..2 {
                a[i][j] += w * m[i][j];
            }
            b[i] += w * (m[i][0] * c[0] + m[i][1] * c[1]);
        }
    }
    let (det, tr) = (a[0][0] * a[1][1] - a[0][1] * a[1][0], a[0][0] + a[1][1]);
    if det.is_nan() || det <= 0.05 * tr * tr {
        return None; // the normals all point one way: a flat bevel, not a cone
    }
    let x = (b[0] * a[1][1] - b[1] * a[0][1]) / det;
    let y = (a[0][0] * b[1] - a[1][0] * b[0]) / det;
    // Radius as a straight line in height.
    let n = points.len() as f64;
    let rz: Vec<[f64; 2]> = points
        .iter()
        .map(|p| [(p[0] - x).hypot(p[1] - y), p[2]])
        .collect();
    let (sz, sr) = (
        rz.iter().map(|q| q[1]).sum::<f64>(),
        rz.iter().map(|q| q[0]).sum::<f64>(),
    );
    let szz = rz.iter().map(|q| q[1] * q[1]).sum::<f64>();
    let szr = rz.iter().map(|q| q[1] * q[0]).sum::<f64>();
    let den = n * szz - sz * sz;
    if den.is_nan() || den <= 1e-12 * n * n {
        return None;
    }
    let slope = (n * szr - sz * sr) / den;
    let k = (sr - slope * sz) / n;
    if rz.iter().any(|q| (k + slope * q[1] - q[0]).abs() > tol) {
        return None;
    }
    // It must go all the way round.
    let mut bins = [false; 36];
    for p in points {
        let ang = (p[1] - y).atan2(p[0] - x).to_degrees().rem_euclid(360.0);
        bins[((ang / 10.0) as usize).min(35)] = true;
    }
    if bins.iter().filter(|&&f| f).count() < 32 {
        return None;
    }
    let (z0, z1) = rz
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), q| {
            (lo.min(q[1]), hi.max(q[1]))
        });
    let (r0, r1) = (k + slope * z0, k + slope * z1);
    (r0 > 0.0 && r1 > 0.0 && z1 > z0).then_some(Cone {
        x,
        y,
        z0,
        r0,
        z1,
        r1,
    })
}

/// A bevel along a straight top edge of the outline: the edge from `a` along unit `dir` for `len`, `out` its
/// outward normal across the base; the bevel cuts back `across` on the top face and `down` on the wall.
#[derive(Debug, Clone, Copy)]
struct Chamfer {
    a: [f64; 2],
    dir: [f64; 2],
    out: [f64; 2],
    len: f64,
    across: f64,
    down: f64,
    top: f64,
    /// A rounded edge (a fillet of radius `across`, equal to `down`), not a flat bevel.
    round: bool,
}

impl Chamfer {
    /// Signed distance to the material the bevel takes away (negative inside it), close enough for the check.
    fn dist(&self, p: [f64; 3]) -> f64 {
        let q = [p[0] - self.a[0], p[1] - self.a[1]];
        let (s, t) = (
            q[0] * self.out[0] + q[1] * self.out[1],
            q[0] * self.dir[0] + q[1] * self.dir[1],
        );
        let ends = (-t).max(t - self.len);
        if self.round {
            // Outside the quarter circle centred `across` in from the wall and down from the top.
            let (ds, dz) = (s + self.across, p[2] - (self.top - self.across));
            return (self.across - ds.hypot(dz)).max(-ds).max(-dz).max(ends);
        }
        // The bevel's plane in (s, z): through (0, top - down) and (-across, top); taken where outside it.
        let g = (1.0 / self.across).hypot(1.0 / self.down);
        let plane = -(s / self.across - (self.top - p[2]) / self.down + 1.0) / g;
        plane.max(ends)
    }
}

/// An edge of a traced polygon being moved onto a wall: a point and direction for its line, the wall it lies on (if
/// any), its length, and the index of its first corner.
type SnapLine = ([f64; 2], [f64; 2], Option<usize>, f64, usize);

/// A flat face square to the base, in the frame: unit normal and centre across the base, extent, heights, area.
struct Wall {
    n: [f64; 2],
    c: [f64; 2],
    lo: [f64; 2],
    hi: [f64; 2],
    zlo: f64,
    zhi: f64,
    area: f64,
}

/// A countersink around a hole of radius `r` at `(x, y)` whose wall stops at `t1`, below the top at `top`: the slanted
/// triangles around its axis between the two, their radius a straight line in height (fit, points off it dropped,
/// fit again), widening upward at a half-angle of 20° to 75°, going all the way round. `corners` gives a triangle's
/// corners in the part frame.
#[allow(clippy::too_many_arguments)]
fn countersink_around(
    x: f64,
    y: f64,
    r: f64,
    t1: f64,
    top: f64,
    reach: f64,
    tol: f64,
    slanted: &[bool],
    corners: &dyn Fn(u32) -> [[f64; 3]; 3],
) -> Option<(Cone, Vec<u32>)> {
    let widest = r + 4.0 * (top - t1) + reach;
    let mut tris = Vec::new();
    let mut rz: Vec<[f64; 2]> = Vec::new();
    for (t, &sl) in slanted.iter().enumerate() {
        if !sl {
            continue;
        }
        let q = corners(t as u32);
        // Facing up and in toward the axis, as a countersink's cone does (20° to 75° from the base's normal).
        let (e1, e2) = (
            [q[1][0] - q[0][0], q[1][1] - q[0][1], q[1][2] - q[0][2]],
            [q[2][0] - q[0][0], q[2][1] - q[0][1], q[2][2] - q[0][2]],
        );
        let f = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let fl = (f[0] * f[0] + f[1] * f[1] + f[2] * f[2]).sqrt();
        let c = [0, 1].map(|k| (q[0][k] + q[1][k] + q[2][k]) / 3.0);
        let inward = f[0] * (x - c[0]) + f[1] * (y - c[1]) > 0.0;
        let up = if fl > 0.0 {
            (f[2] / fl).clamp(-1.0, 1.0).acos().to_degrees()
        } else {
            90.0
        };
        let ok = inward
            && (20.0..=75.0).contains(&up)
            && q.iter().all(|p| {
                let rho = (p[0] - x).hypot(p[1] - y);
                rho >= r - reach && rho <= widest && p[2] >= t1 - reach && p[2] <= top + reach
            });
        if ok {
            tris.push(t as u32);
            rz.extend(q.iter().map(|p| [(p[0] - x).hypot(p[1] - y), p[2]]));
        }
    }
    let line = |pts: &[[f64; 2]]| {
        let n = pts.len() as f64;
        let (sz, sr) = (
            pts.iter().map(|q| q[1]).sum::<f64>(),
            pts.iter().map(|q| q[0]).sum::<f64>(),
        );
        let szz = pts.iter().map(|q| q[1] * q[1]).sum::<f64>();
        let szr = pts.iter().map(|q| q[1] * q[0]).sum::<f64>();
        let den = n * szz - sz * sz;
        (den > 0.0).then(|| {
            let m = (n * szr - sz * sr) / den;
            (m, (sr - m * sz) / n)
        })
    };
    if rz.len() < 12 {
        return None;
    }
    let (mut m, mut k) = line(&rz)?;
    for _ in 0..4 {
        let kept: Vec<[f64; 2]> = rz
            .iter()
            .copied()
            .filter(|q| (k + m * q[1] - q[0]).abs() <= 2.0 * tol)
            .collect();
        if kept.len() < 12 || kept.len() == rz.len() {
            break;
        }
        (m, k) = line(&kept)?;
        rz = kept;
    }
    let rms = (rz
        .iter()
        .map(|q| (k + m * q[1] - q[0]).powi(2))
        .sum::<f64>()
        / rz.len() as f64)
        .sqrt();
    if !(rms <= tol && m >= 20f64.to_radians().tan() && m <= 75f64.to_radians().tan()) {
        return None;
    }
    // Where the cone meets the hole's wall, and its radius at the top.
    let z0 = (r - k) / m;
    let r1 = k + m * top;
    if !(z0 > t1 - 2.0 * reach && z0 < top - tol && r1 > r) {
        return None;
    }
    let cone = Cone {
        x,
        y,
        z0,
        r0: r,
        z1: top,
        r1,
    };
    // All the way round.
    let mut bins = [false; 36];
    for &t in &tris {
        for p in corners(t) {
            let ang = (p[1] - y).atan2(p[0] - x).to_degrees().rem_euclid(360.0);
            bins[((ang / 10.0) as usize).min(35)] = true;
        }
    }
    (bins.iter().filter(|&&f| f).count() >= 32).then_some((cone, tris))
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
    // How rough the surface is: the median angle between neighbouring triangles (0 on a CAD mesh, whose flat faces
    // are flat; several degrees on a noisy scan, whose small triangles tilt every way). Faces grow across that
    // roughness; the distance limit still stops them at real edges.
    let rough = {
        let step = (graph.edges.len() / 50_000).max(1);
        let mut d: Vec<f64> = graph
            .edges
            .iter()
            .step_by(step)
            .filter_map(|e| {
                let t1 = e.t1?;
                let (a, b) = (
                    graph.face_normals[e.t0 as usize],
                    graph.face_normals[t1 as usize],
                );
                (dot(a, a) > 0.0 && dot(b, b) > 0.0).then(|| acute_deg(a, b))
            })
            .collect();
        d.sort_by(f64::total_cmp);
        d.get(d.len() / 2).copied().unwrap_or(0.0)
    };
    let plane_options = crate::segment::PlaneSegmentOptions {
        max_angle: (2.5 * rough).clamp(10.0, 30.0).to_radians(),
        ..Default::default()
    };
    // No round on a part is wider than the part: half its diagonal bounds the radius, which keeps the search from
    // growing near-flat "cylinders" across a noisy face only to drop them.
    let diagonal = {
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for p in &mesh.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(f64::from(p[k]));
                hi[k] = hi[k].max(f64::from(p[k]));
            }
        }
        let d = sub(hi, lo);
        dot(d, d).sqrt()
    };
    let cylinder_options = crate::segment::CylinderSegmentOptions {
        max_angle: (2.5 * rough).clamp(15.0, 30.0).to_radians(),
        max_radius: (diagonal / 2.0).max(1.0),
        ..Default::default()
    };
    let planes = segment_planes(mesh, &graph, plane_options).regions;
    // Triangles of large flat faces seed no round: on a noisy scan they are most of the mesh.
    let mut on_big_face = vec![false; mesh.triangles.len()];
    for p in planes
        .iter()
        .filter(|p| p.triangles.len() >= BIG_FACE_TRIANGLES)
    {
        for &t in &p.triangles {
            on_big_face[t as usize] = true;
        }
    }
    let cylinders =
        segment_cylinders_seeded(mesh, &graph, cylinder_options, |t| !on_big_face[t as usize])
            .regions;
    let pos = |i: u32| mesh.positions[i as usize].map(f64::from);

    // The base: the largest flat region.
    let (bi, base) = planes
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.area.total_cmp(&b.1.area))
        .ok_or_else(|| NotRebuilt("the mesh has no flat face to build on".into()))?;
    // A flat face's point: the area-weighted centre of its triangles lying within 1° of it. Region growing takes
    // triangles up to 10° off, the first facets of a fillet among them, which would pull the fitted centre off the face.
    let flat_center = |r: &crate::segment::PlaneRegion| {
        let (mut sum, mut w) = ([0.0; 3], 0.0);
        for &t in &r.triangles {
            if acute_deg(graph.face_normals[t as usize], r.plane.normal) > 1.0 {
                continue;
            }
            let [i, j, k] = mesh.triangles[t as usize];
            let (a, b, c) = (pos(i), pos(j), pos(k));
            let cr = cross(sub(b, a), sub(c, a));
            let area = dot(cr, cr).sqrt() / 2.0;
            for q in 0..3 {
                sum[q] += area * (a[q] + b[q] + c[q]) / 3.0;
            }
            w += area;
        }
        if w > 0.0 {
            scale(sum, 1.0 / w)
        } else {
            r.plane.center
        }
    };
    let o = flat_center(base);
    // A flat face's normal from the same triangles, area-weighted: the region's fit leans toward any fillet facets
    // it took in.
    let flat_normal = |r: &crate::segment::PlaneRegion| {
        let mut sum = [0.0; 3];
        for &t in &r.triangles {
            let f = graph.face_normals[t as usize];
            if acute_deg(f, r.plane.normal) > 1.0 {
                continue;
            }
            let [i, j, k] = mesh.triangles[t as usize];
            let (a, b, c) = (pos(i), pos(j), pos(k));
            let cr = cross(sub(b, a), sub(c, a));
            let w = dot(cr, cr).sqrt() / 2.0;
            let s = if dot(f, r.plane.normal) < 0.0 { -w } else { w };
            for q in 0..3 {
                sum[q] += s * f[q];
            }
        }
        normalize(sum).unwrap_or(r.plane.normal)
    };
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
    let a = wall_angle(&planes, n, u0, v0).map_or_else(
        || min_rect_angle(&flat),
        |w| {
            // The same direction as the rectangle's, a quarter turn at most: keeps length and width as before.
            let r = min_rect_angle(&flat);
            let q = std::f64::consts::FRAC_PI_2;
            [w - q, w, w + q]
                .into_iter()
                .min_by(|x, y| (x - r).abs().total_cmp(&(y - r).abs()))
                .unwrap_or(w)
        },
    );
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
    let (blo, bhi) = extent(&base.vertices);
    let (base_len, base_wid) = (bhi[0] - blo[0], bhi[1] - blo[1]);
    if !(base_len > 0.0 && base_wid > 0.0 && base_len.is_finite() && base_wid.is_finite()) {
        return Err(NotRebuilt("the base face has no extent".into()));
    }
    let thick = dot(sub(flat_center(&planes[ti]), o), n);
    if !(thick > 0.0 && thick.is_finite()) {
        return Err(NotRebuilt(
            "the opposite face is not apart from the base".into(),
        ));
    }
    let tol = (0.02 * thick).max(0.05);
    let margin = (0.1 * thick).max(0.5);
    // How much of its rectangle a face must cover to count as one: noise leaves small gaps in a scan's faces.
    let full = if rough > 2.0 { 0.93 } else { 0.98 };

    let square = if rough > 2.0 { 2.5 } else { 1.0 };
    // How far a scan's softened edge can pull a face's boundary or a round's end from where the edge really is:
    // three of the mesh's typical edge lengths, at most 1 mm, at least the tolerance. On a CAD mesh faces meet
    // exactly, so the nearest match within it is the exact one.
    let reach = {
        let step = (graph.edges.len() / 20_000).max(1);
        let mut e: Vec<f64> = graph
            .edges
            .iter()
            .step_by(step)
            .map(|e| {
                let d = sub(pos(e.a), pos(e.b));
                dot(d, d).sqrt()
            })
            .collect();
        e.sort_by(f64::total_cmp);
        let median = e.get(e.len() / 2).copied().unwrap_or(0.0);
        (3.0 * median).min(1.0).max(tol)
    };
    // Walls square to the base, in the frame: where faces' edges really are on a scan, whose faces stop short of
    // their softened edges.
    let walls: Vec<Wall> = planes
        .iter()
        .filter(|p| (acute_deg(p.plane.normal, n) - 90.0).abs() <= SQUARE_DEG)
        .filter_map(|p| {
            let wn = flat_normal(p);
            let (nx, ny) = (dot(wn, u), dot(wn, v));
            let l = nx.hypot(ny);
            let c = local(flat_center(p));
            let (lo, hi) = extent(&p.vertices);
            let (zlo, zhi) =
                p.vertices
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &i| {
                        let z = local(pos(i))[2];
                        (a.min(z), b.max(z))
                    });
            (l > 0.0).then_some(Wall {
                n: [nx / l, ny / l],
                c: [c[0], c[1]],
                lo,
                hi,
                zlo,
                zhi,
                area: p.area,
            })
        })
        .collect();
    // A rectangle's sides moved onto the walls that bound it: facing out of it for the plate and pads, into it for
    // pockets; each wall within 1° of the side's direction, within `reach` of it, and beside it in height and along it.
    let snap = |lo: [f64; 2], hi: [f64; 2], z0: f64, z1: f64, inward: bool| {
        let (mut lo2, mut hi2) = (lo, hi);
        for k in 0..2 {
            let o = 1 - k;
            for side in [-1.0f64, 1.0] {
                let at = if side < 0.0 { lo[k] } else { hi[k] };
                let want = if inward { -side } else { side };
                let best = walls
                    .iter()
                    .filter(|w| {
                        w.n[k] * want >= 1f64.to_radians().cos()
                            && (w.c[k] - at).abs() <= reach
                            && w.zhi > z0 + tol
                            && w.zlo < z1 - tol
                            && w.hi[o] > lo[o] + tol
                            && w.lo[o] < hi[o] - tol
                    })
                    .max_by(|a, b| a.area.total_cmp(&b.area));
                if let Some(w) = best {
                    if side < 0.0 {
                        lo2[k] = w.c[k];
                    } else {
                        hi2[k] = w.c[k];
                    }
                }
            }
        }
        (lo2, hi2)
    };
    let (lo, hi) = snap(blo, bhi, 0.0, thick, false);
    let (len, wid) = (hi[0] - lo[0], hi[1] - lo[1]);
    // How far a traced boundary may stray from a straight edge or a circle: a scan's face boundary is ragged.
    let ptol = if rough > 2.0 { reach } else { tol };
    let to_part = |l: [f64; 3]| [l[0] - lo[0], l[1] - lo[1], l[2]];
    // A counter-clockwise polygon in the part frame with each edge moved onto the wall that bounds it: its normal
    // within 10° of the side away from the material (a traced edge is ragged), within `reach` of the edge, beside it
    // along the edge. Corners are where the moved edges meet. `material_inside` for an outline, not for a window.
    let snap_poly = |poly: &[[f64; 2]], material_inside: bool| -> Vec<[f64; 2]> {
        let m = poly.len();
        // Each edge: a point and direction for its line, the wall it lies on (if any), its length, its first corner.
        let lines: Vec<SnapLine> = (0..m)
            .map(|i| {
                let (a, b) = (poly[i], poly[(i + 1) % m]);
                let l = (b[0] - a[0]).hypot(b[1] - a[1]);
                if l <= 0.0 {
                    return (a, [1.0, 0.0], None, 0.0, i);
                }
                let dir = [(b[0] - a[0]) / l, (b[1] - a[1]) / l];
                let out = [dir[1], -dir[0]];
                let want = if material_inside {
                    out
                } else {
                    [-out[0], -out[1]]
                };
                let best = walls
                    .iter()
                    .enumerate()
                    .filter(|(_, w)| {
                        let c = [w.c[0] - lo[0], w.c[1] - lo[1]];
                        let d = [c[0] - a[0], c[1] - a[1]];
                        // The wall's extent along the edge overlaps the edge.
                        let ts = [
                            [w.lo[0], w.lo[1]],
                            [w.hi[0], w.lo[1]],
                            [w.lo[0], w.hi[1]],
                            [w.hi[0], w.hi[1]],
                        ]
                        .map(|q| (q[0] - lo[0] - a[0]) * dir[0] + (q[1] - lo[1] - a[1]) * dir[1]);
                        let (t0, t1) = ts
                            .iter()
                            .fold((f64::INFINITY, f64::NEG_INFINITY), |(x, y), &t| {
                                (x.min(t), y.max(t))
                            });
                        w.n[0] * want[0] + w.n[1] * want[1] >= 10f64.to_radians().cos()
                            && (d[0] * out[0] + d[1] * out[1]).abs() <= reach
                            && t1 > -reach
                            && t0 < l + reach
                            && w.zhi - w.zlo > tol
                    })
                    .max_by(|x, y| x.1.area.total_cmp(&y.1.area));
                match best {
                    Some((k, w)) => {
                        let d = [-w.n[1], w.n[0]];
                        let s = if d[0] * dir[0] + d[1] * dir[1] < 0.0 {
                            -1.0
                        } else {
                            1.0
                        };
                        (
                            [w.c[0] - lo[0], w.c[1] - lo[1]],
                            [s * d[0], s * d[1]],
                            Some(k),
                            l,
                            i,
                        )
                    }
                    None => (a, dir, None, l, i),
                }
            })
            .collect();
        // Edges on the same wall as the one before are one edge; a short edge no wall bounds, between two that walls
        // do, is a softened corner, and its neighbours meet instead.
        let mut kept: Vec<SnapLine> = Vec::new();
        for i in 0..m {
            let (prev, this, next) = (lines[(i + m - 1) % m], lines[i], lines[(i + 1) % m]);
            let softened =
                this.2.is_none() && this.3 < 3.0 * reach && prev.2.is_some() && next.2.is_some();
            if softened || (this.2.is_some() && kept.last().is_some_and(|k| k.2 == this.2)) {
                continue;
            }
            kept.push(this);
        }
        if kept.len() > 1 && kept[0].2.is_some() && kept[0].2 == kept[kept.len() - 1].2 {
            kept.remove(0);
        }
        let k = kept.len();
        if k < 3 {
            return poly.to_vec();
        }
        (0..k)
            .map(|i| {
                let ((p1, d1, ..), (p2, d2, _, _, at)) = (kept[(i + k - 1) % k], kept[i]);
                let traced = poly[at];
                let cr = d1[0] * d2[1] - d1[1] * d2[0];
                if cr.abs() < 1e-6 {
                    return traced;
                }
                let w = [p2[0] - p1[0], p2[1] - p1[1]];
                let s = (w[0] * d2[1] - w[1] * d2[0]) / cr;
                let q = [p1[0] + s * d1[0], p1[1] + s * d1[1]];
                // A corner the walls move far (they did not bound this corner's edges) keeps its traced place.
                if (q[0] - traced[0]).hypot(q[1] - traced[1]) <= 3.0 * reach {
                    q
                } else {
                    traced
                }
            })
            .collect()
    };
    let mut losses: Vec<Loss> = Vec::new();

    // The outline: the base face's outer boundary, as a circle or a polygon when it is not a rectangle.
    let part_xy = |i: u32| {
        let l = local(pos(i));
        [l[0] - lo[0], l[1] - lo[1]]
    };
    let mut outline = Outline::Rectangle;
    let mut outline_area = base_len * base_wid;
    let base_loops = boundary_loops(&base.triangles, mesh, &part_xy).unwrap_or_default();
    if let Some(ring) = base_loops.first() {
        let area = signed_area(ring).abs();
        if area < 0.98 * base_len * base_wid {
            if let Some((x, y, r)) = circle_of(ring, ptol) {
                // The outer wall's own circle, when one was found round it: the base face stops short of it on a
                // scan.
                let (x, y, r) = cylinders
                    .iter()
                    .filter(|c| acute_deg(c.cylinder.axis, n) <= SQUARE_DEG)
                    .map(|c| {
                        let at = to_part(local(c.cylinder.axis_point));
                        (c.area, at[0], at[1], c.cylinder.diameter / 2.0)
                    })
                    .filter(|c| (c.1 - x).hypot(c.2 - y) <= reach && (c.3 - r).abs() <= reach)
                    .max_by(|a, b| a.0.total_cmp(&b.0))
                    .map_or((x, y, r), |c| (c.1, c.2, c.3));
                outline_area = std::f64::consts::PI * r * r;
                outline = Outline::Circle { x, y, r };
            } else {
                let mut poly = simplify_loop(ring, ptol);
                let poly_area = signed_area(&poly);
                if (3..=MAX_OUTLINE_CORNERS).contains(&poly.len()) && poly_area.abs() > 0.0 {
                    if poly_area < 0.0 {
                        poly.reverse(); // counter-clockwise in the part frame
                    }
                    let poly = snap_poly(&poly, true);
                    let poly_area = signed_area(&poly);
                    let short = (0..poly.len())
                        .filter(|&i| {
                            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
                            (b[0] - a[0]).hypot(b[1] - a[1]) < 6.0 * ptol
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
                height: dot(sub(flat_center(p), o), n),
                up: dot(p.plane.normal, n) > 0.0,
                area: p.area,
                lo: [l[0] - lo[0], l[1] - lo[1]],
                hi: [h[0] - lo[0], h[1] - lo[1]],
                // A scan's ragged face boundary turns its smallest rectangle a little: allow 2.5° on a rough mesh.
                aligned: ang <= square || ang >= 90.0 - square,
            }
        })
        .collect();
    let up_heights: Vec<f64> = levels.iter().filter(|l| l.up).map(|l| l.height).collect();
    let down_heights: Vec<f64> = std::iter::once(0.0)
        .chain(levels.iter().filter(|l| !l.up).map(|l| l.height))
        .collect();
    // The face height nearest a round's end, within `reach`: on a scan a round stops short of its faces.
    let nearest = |hs: &[f64], h: f64| {
        hs.iter()
            .copied()
            .filter(|&x| (x - h).abs() <= reach)
            .min_by(|a, b| (a - h).abs().total_cmp(&(b - h).abs()))
    };

    // Cones square to the base, from the slanted triangles: countersinks once matched to their holes.
    let slanted: Vec<bool> = graph
        .face_normals
        .iter()
        .map(|&f| {
            let a = acute_deg(f, n);
            a > SQUARE_DEG && a < 90.0 - SQUARE_DEG
        })
        .collect();
    let mut cones: Vec<(Cone, Vec<u32>)> = Vec::new();
    let mut bevel_groups: Vec<Vec<u32>> = Vec::new();
    {
        let mut parent: Vec<u32> = (0..slanted.len() as u32).collect();
        fn root(parent: &mut [u32], mut i: u32) -> u32 {
            while parent[i as usize] != i {
                parent[i as usize] = parent[parent[i as usize] as usize];
                i = parent[i as usize];
            }
            i
        }
        for e in &graph.edges {
            if let (Some(t1), false) = (e.t1, e.non_manifold) {
                if slanted[e.t0 as usize] && slanted[t1 as usize] {
                    let (a, b) = (root(&mut parent, e.t0), root(&mut parent, t1));
                    parent[a as usize] = b;
                }
            }
        }
        let mut groups: std::collections::BTreeMap<u32, Vec<u32>> = Default::default();
        for t in 0..slanted.len() as u32 {
            if slanted[t as usize] {
                groups.entry(root(&mut parent, t)).or_default().push(t);
            }
        }
        for tris in groups.into_values().filter(|g| g.len() >= 6) {
            let mut faces = Vec::with_capacity(tris.len());
            let mut points = Vec::with_capacity(3 * tris.len());
            for &t in &tris {
                let [i, j, k] = mesh.triangles[t as usize];
                let (pi, pj, pk) = (pos(i), pos(j), pos(k));
                let area = 0.5 * {
                    let c = cross(sub(pj, pi), sub(pk, pi));
                    dot(c, c).sqrt()
                };
                let f = graph.face_normals[t as usize];
                let Some(h) = ({
                    let (hx, hy) = (dot(f, u), dot(f, v));
                    let l = hx.hypot(hy);
                    (l > 0.0).then(|| [hx / l, hy / l])
                }) else {
                    continue;
                };
                let c = to_part(local(scale(
                    [0, 1, 2].map(|q| pi[q] + pj[q] + pk[q]),
                    1.0 / 3.0,
                )));
                faces.push((area, [c[0], c[1]], h));
                points.extend([pi, pj, pk].map(|q| to_part(local(q))));
            }
            if let Some(c) = cone_of(&faces, &points, tol) {
                cones.push((c, tris));
            } else {
                bevel_groups.push(tris);
            }
        }
    }
    let mut cone_used = vec![false; cones.len()];
    let mut countersink_tris: Vec<u32> = Vec::new();
    // Faces under a hundredth of the base's area go in one item: on a scan, the softened edges.
    let small = 0.01 * base.area;
    let (mut small_count, mut small_area) = (0usize, 0.0);
    let mut tilted: Vec<Loss> = Vec::new();
    let mut rounded_corners: Vec<f64> = Vec::new();
    let mut tilted_of: Vec<usize> = Vec::new();

    // Round regions square to the base: holes (surface facing the axis) and bosses (facing away).
    let mut holes: Vec<Value> = Vec::new();
    let mut bosses: Vec<Value> = Vec::new();
    // Flat faces a round feature explains: a boss's top, a blind hole's floor.
    let mut explained: Vec<usize> = Vec::new();
    let mark = |explained: &mut Vec<usize>, x: f64, y: f64, r: f64, h: f64| {
        let mut found: Option<f64> = None;
        for l in &levels {
            let (cx, cy) = ((l.lo[0] + l.hi[0]) / 2.0, (l.lo[1] + l.hi[1]) / 2.0);
            if (l.height - h).abs() <= reach
                && (cx - x).hypot(cy - y) <= r * 0.5
                && l.area <= std::f64::consts::PI * r * r * 1.05
            {
                explained.push(l.index);
                if found.is_none_or(|f| (l.height - h).abs() < (f - h).abs()) {
                    found = Some(l.height);
                }
            }
        }
        found
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
            // Held back: strips of a countersink's cone fit as tilted rounds, and only the cone knows.
            drop(
                &mut tilted,
                "Round features not square to the base",
                "tilted_round",
                format!(
                    "its axis is {:.1}° off the base normal",
                    acute_deg(f.axis, n)
                ),
            );
            tilted_of.push(k);
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
        if let Outline::Circle {
            x: cx,
            y: cy0,
            r: big,
        } = outline
        {
            if (x - cx).hypot(y - cy0) <= reach && (r - big).abs() <= reach {
                continue; // a piece of the round outline's own wall
            }
        }
        if !inside_outline(x, y, r) {
            if cy.area < small {
                small_count += 1;
                small_area += cy.area;
                continue;
            }
            drop(
                &mut losses,
                "Round features outside the plate",
                "round_outside",
                "it does not lie wholly inside the base outline".into(),
            );
            continue;
        }
        // Too thin to be a feature on this mesh, or a piece of a round already taken: the softened edges.
        let piece = holes.iter().chain(&bosses).any(|h| {
            let (hx, hy) = (
                h["x"].as_f64().unwrap_or(f64::NAN),
                h["y"].as_f64().unwrap_or(f64::NAN),
            );
            (hx - x).hypot(hy - y) <= reach
                && (h["diameter"].as_f64().unwrap_or(f64::NAN) - f.diameter).abs() <= 2.0 * reach
        });
        if r < reach / 2.0 || piece {
            small_count += 1;
            small_area += cy.area;
            continue;
        }
        if f.coverage_deg < MIN_ROUND_COVERAGE_DEG {
            // Less than most of a circle: a rounded vertical edge, not a hole or a boss.
            let outline_wall = t0.abs() <= reach && (t1 - thick).abs() <= reach;
            if !(outline_wall && !matches!(outline, Outline::Rectangle)) {
                rounded_corners.push(f.diameter / 2.0);
            }
            continue;
        }
        if facing < 0.0 {
            // A countersink: a cone widening from the hole's top to the plate's top face, on its axis.
            let csk = cones.iter().position(|(c, _)| {
                (c.x - x).hypot(c.y - y) <= tol
                    && (c.z0 - t1).abs() <= reach
                    && (c.r0 - r).abs() <= reach
                    && c.r1 > c.r0
                    && (c.z1 - thick).abs() <= reach
            });
            let mut countersink = csk.map(|ci| {
                cone_used[ci] = true;
                let c = cones[ci].0;
                t1 = thick;
                json!({"diameter": 2.0 * c.r1, "depth": c.z1 - c.z0,
                       "angle_deg": 2.0 * ((c.r1 - c.r0) / (c.z1 - c.z0)).atan().to_degrees()})
            });
            // On a scan the cone's facets are too noisy to find on their own: look around the hole instead, when its
            // wall stops short of the top.
            if countersink.is_none() && t1 < thick - reach {
                if let Some((c, tris)) =
                    countersink_around(x, y, r, t1, thick, reach, tol, &slanted, &|t| {
                        mesh.triangles[t as usize].map(|i| to_part(local(pos(i))))
                    })
                {
                    t1 = thick;
                    countersink_tris.extend(tris);
                    countersink = Some(json!({"diameter": 2.0 * c.r1, "depth": c.z1 - c.z0,
                        "angle_deg": 2.0 * ((c.r1 - c.r0) / (c.z1 - c.z0)).atan().to_degrees()}));
                }
            }
            let (top_h, bottom_h) = (nearest(&up_heights, t1), nearest(&down_heights, t0));
            let (top, bottom) = (top_h.is_some(), bottom_h.is_some());
            t1 = top_h.unwrap_or(t1);
            t0 = bottom_h.unwrap_or(t0);
            let kind = match (bottom, top) {
                (true, true) => "through",
                (false, true) => "blind from above",
                (true, false) => "blind from below",
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
                // The floor of a hole blind from above.
                if let Some(h) = mark(&mut explained, x, y, r, t0) {
                    t0 = h;
                }
            } else if !top {
                // The end of a hole blind from below.
                if let Some(h) = mark(&mut explained, x, y, r, t1) {
                    t1 = h;
                }
            }
            let (z, height) = match kind {
                "through" => (t0 - margin, t1 - t0 + 2.0 * margin),
                "blind from above" => (t0, t1 - t0 + margin),
                _ => (t0 - margin, t1 - t0 + margin),
            };
            holes.push(json!({"x": x, "y": y, "z": z, "height": height, "diameter": f.diameter,
                              "kind": kind, "from": t0, "to": t1, "sigma": f.sigma, "coverage_deg": f.coverage_deg,
                              "countersink": countersink}));
        } else if let Some(on) = nearest(&up_heights, t0).filter(|_| t0 > tol) {
            // Standing on an upward face (the top, a pocket floor or a pad); joined from just inside it.
            t0 = on;
            if let Some(h) = mark(&mut explained, x, y, r, t1) {
                t1 = h;
            }
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

    // Bevels along straight top edges of the outline: flat slanted faces from the top face down a wall.
    let edges: Vec<([f64; 2], [f64; 2])> = match &outline {
        Outline::Rectangle => vec![
            ([0.0, 0.0], [len, 0.0]),
            ([len, 0.0], [len, wid]),
            ([len, wid], [0.0, wid]),
            ([0.0, wid], [0.0, 0.0]),
        ],
        Outline::Polygon(p) => (0..p.len()).map(|i| (p[i], p[(i + 1) % p.len()])).collect(),
        Outline::Circle { .. } => Vec::new(),
    };
    let mut chamfers: Vec<(Chamfer, Vec<u32>)> = Vec::new();
    // Fillets: on each straight top edge, the slanted triangles facing straight out over it whose cross-section is a
    // circle touching both the top face and the wall.
    let mut rounded: std::collections::HashSet<u32> = Default::default();
    let bevel_tris: Vec<u32> = bevel_groups.iter().flatten().copied().collect();
    for &(a, b) in &edges {
        let l = (b[0] - a[0]).hypot(b[1] - a[1]);
        if l <= 0.0 {
            continue;
        }
        let dir = [(b[0] - a[0]) / l, (b[1] - a[1]) / l];
        let out = [dir[1], -dir[0]];
        let st = |p: [f64; 3]| {
            let q = [p[0] - a[0], p[1] - a[1]];
            [
                q[0] * out[0] + q[1] * out[1],
                q[0] * dir[0] + q[1] * dir[1],
                p[2],
            ]
        };
        // Triangles beside the edge facing partway between the wall and the top (15° to 75° from the base's normal,
        // across the base within 20° of the edge's outward side): the middle of a round, even with noise.
        let mut tris = Vec::new();
        let mut section: Vec<[f64; 2]> = Vec::new();
        for (t, &sl) in slanted.iter().enumerate() {
            if !sl {
                continue;
            }
            let f = graph.face_normals[t];
            let (hx, hy) = (dot(f, u), dot(f, v));
            let hl = hx.hypot(hy);
            let up = acute_deg(f, n);
            if hl <= 0.0
                || dot(f, n) <= 0.0
                || !(15.0..=75.0).contains(&up)
                || (hx * out[0] + hy * out[1]) / hl < 20f64.to_radians().cos()
            {
                continue;
            }
            let q = mesh.triangles[t].map(|i| st(to_part(local(pos(i)))));
            if q.iter().all(|q| {
                q[0] <= reach && q[0] >= -thick && q[2] >= 0.0 && q[1] >= -tol && q[1] <= l + tol
            }) {
                tris.push(t as u32);
                section.extend(q.iter().map(|q| [q[0], q[2]]));
            }
        }
        // A fillet's cross-section is a quarter circle at most: `circle_of` wants points round it, so fit it
        // here, dropping points off it (noise, a stray facet) and fitting again.
        if tris.len() < 4 {
            continue;
        }
        // A fillet touches the top and the wall, so its centre is `r` in from both and each point gives its own
        // radius: (s + r)² + (z - top + r)² = r². The median of those is the radius; stray points do not move it.
        let mut each: Vec<f64> = section
            .iter()
            .filter_map(|q| {
                let (a, b) = (q[0], q[1] - thick);
                (a < 0.0 && b < 0.0).then(|| -(a + b) + (2.0 * a * b).sqrt())
            })
            .collect();
        if each.len() < 6 {
            continue;
        }
        each.sort_by(f64::total_cmp);
        let r = each[each.len() / 2];
        let (cs, cz) = (-r, thick - r);
        let off = |q: &[f64; 2]| ((q[0] - cs).hypot(q[1] - cz) - r).abs();
        let inliers: Vec<[f64; 2]> = section
            .iter()
            .copied()
            .filter(|q| off(q) <= 2.0 * tol)
            .collect();
        let rms = (inliers.iter().map(|q| off(q).powi(2)).sum::<f64>()
            / inliers.len().max(1) as f64)
            .sqrt();
        // Most of its points on that circle, across a real arc (not a flat bevel, whose points give every radius).
        let span = {
            let angles: Vec<f64> = inliers
                .iter()
                .map(|q| (q[1] - cz).atan2(q[0] - cs).to_degrees())
                .collect();
            let (lo_a, hi_a) = angles
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(x, y), &a| {
                    (x.min(a), y.max(a))
                });
            hi_a - lo_a
        };
        // And its facets turn along it: a bevel's all face one way, however well a circle passes near its points.
        let turn = {
            let mut ups: Vec<f64> = tris
                .iter()
                .map(|&t| acute_deg(graph.face_normals[t as usize], n))
                .collect();
            ups.sort_by(f64::total_cmp);
            // Half or more within 6° of the middle facing: one face, a bevel with softened sides.
            let mid = ups[ups.len() / 2];
            let together = ups.iter().filter(|&&x| (x - mid).abs() <= 6.0).count();
            if together * 2 >= ups.len() {
                0.0
            } else {
                ups[ups.len() * 9 / 10] - ups[ups.len() / 10]
            }
        };
        let on = inliers.len() * 10 >= section.len() * 6
            && rms <= tol
            && span >= 30.0
            && turn >= 20.0
            && r > reach.max(tol);
        if on && (cs + r).abs() <= 2.0 * tol && (cz - (thick - r)).abs() <= 2.0 * tol && r < thick {
            rounded.extend(tris.iter().copied());
            chamfers.push((
                Chamfer {
                    a,
                    dir,
                    out,
                    len: l,
                    across: r,
                    down: r,
                    top: thick,
                    round: true,
                },
                tris,
            ));
        }
    }
    // The corners where two fillets meet: slanted triangles inside the round band of a filleted edge, past its ends.
    let mut fillet_corners: Vec<u32> = Vec::new();
    for &t in &bevel_tris {
        if rounded.contains(&t) {
            continue;
        }
        let q = mesh.triangles[t as usize].map(|i| to_part(local(pos(i))));
        let in_band = |c: &Chamfer| {
            q.iter().all(|p| {
                let d = [p[0] - c.a[0], p[1] - c.a[1]];
                let (s, w) = (
                    d[0] * c.out[0] + d[1] * c.out[1],
                    d[0] * c.dir[0] + d[1] * c.dir[1],
                );
                s <= tol
                    && s >= -c.across - tol
                    && p[2] >= c.top - c.across - tol
                    && w >= -c.across - tol
                    && w <= c.len + c.across + tol
            })
        };
        if chamfers.iter().any(|(c, _)| c.round && in_band(c)) {
            fillet_corners.push(t);
        }
    }
    rounded.extend(fillet_corners.iter().copied());
    // Each bevel is one of the flat regions, slanted to the base: grown regions stay together on a scan, where single
    // facets point every way.
    {
        let planes_of: Vec<([f64; 3], Vec<u32>, [f64; 3])> = planes
            .iter()
            .filter(|p| {
                let a = acute_deg(p.plane.normal, n);
                a > SQUARE_DEG
                    && a < 90.0 - SQUARE_DEG
                    // Not a fillet's: a bevel can share a few triangles with a fillet's corner where they meet.
                    && p.triangles.iter().filter(|t| rounded.contains(t)).count() * 2 < p.triangles.len()
            })
            .map(|p| {
                (
                    p.plane.normal,
                    p.triangles.clone(),
                    to_part(local(flat_center(p))),
                )
            })
            .collect();
        for (f, tris, center) in planes_of {
            if dot(f, n) <= 0.0 {
                continue; // facing down: a bevel under the plate, which changes the base's outline
            }
            let Some(h) = ({
                let (hx, hy) = (dot(f, u), dot(f, v));
                let l = hx.hypot(hy);
                (l > 0.0).then(|| [hx / l, hy / l])
            }) else {
                continue;
            };
            let pts: Vec<[f64; 3]> = tris
                .iter()
                .flat_map(|&t| mesh.triangles[t as usize])
                .map(|i| to_part(local(pos(i))))
                .collect();
            let (zlo, zhi) = pts
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
                    (a.min(p[2]), b.max(p[2]))
                });
            if (zhi - thick).abs() > tol || zlo <= tol {
                continue;
            }
            let found = edges.iter().find_map(|&(a, b)| {
                let l = (b[0] - a[0]).hypot(b[1] - a[1]);
                if l <= 0.0 {
                    return None;
                }
                let dir = [(b[0] - a[0]) / l, (b[1] - a[1]) / l];
                let out = [dir[1], -dir[0]]; // the outline runs counter-clockwise
                if out[0] * h[0] + out[1] * h[1] < SQUARE_DEG.to_radians().cos() {
                    return None;
                }
                let st: Vec<(f64, f64)> = pts
                    .iter()
                    .map(|p| {
                        let q = [p[0] - a[0], p[1] - a[1]];
                        (q[0] * out[0] + q[1] * out[1], q[0] * dir[0] + q[1] * dir[1])
                    })
                    .collect();
                let smax = st.iter().map(|x| x.0).fold(f64::NEG_INFINITY, f64::max);
                let smin = st.iter().map(|x| x.0).fold(f64::INFINITY, f64::min);
                let inside = st.iter().all(|x| x.1 >= -tol && x.1 <= l + tol);
                // It runs along the whole edge, short only where bevels on the next edges meet it.
                let (t0, t1) = st
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| {
                        (a.min(x.1), b.max(x.1))
                    });
                let along = t1 - t0 >= l + 2.0 * smin - 2.0 * reach;
                let size_ok = -smin < l / 2.0 && thick - zlo < thick;
                (smax.abs() <= reach.max(tol) && inside && smin < -tol && along && size_ok)
                    .then_some(Chamfer {
                        a,
                        dir,
                        out,
                        len: l,
                        // Where the bevel's own plane meets the top and the wall: its extent on a scan runs
                        // into the softened rims on both sides.
                        across: {
                            let (fs, fz) = (dot(f, u) * out[0] + dot(f, v) * out[1], dot(f, n));
                            let cs = (center[0] - a[0]) * out[0] + (center[1] - a[1]) * out[1];
                            let top_s = cs - fz * (thick - center[2]) / fs;
                            if top_s.is_finite() && top_s < 0.0 {
                                -top_s
                            } else {
                                -smin
                            }
                        },
                        down: {
                            let (fs, fz) = (dot(f, u) * out[0] + dot(f, v) * out[1], dot(f, n));
                            let cs = (center[0] - a[0]) * out[0] + (center[1] - a[1]) * out[1];
                            let wall_z = center[2] + fs * cs / fz;
                            if wall_z.is_finite() && wall_z < thick {
                                thick - wall_z
                            } else {
                                thick - zlo
                            }
                        },
                        top: thick,
                        round: false,
                    })
            });
            if let Some(c) = found {
                chamfers.push((c, tris));
            }
        }
    }

    // Slanted faces nothing above explains.
    let explained_tris: std::collections::HashSet<u32> = chamfers
        .iter()
        .flat_map(|(_, tris)| tris.iter().copied())
        .chain(countersink_tris.iter().copied())
        .chain(fillet_corners.iter().copied())
        .chain(
            cones
                .iter()
                .zip(&cone_used)
                .filter(|(_, used)| **used)
                .flat_map(|((_, tris), _)| tris.iter().copied()),
        )
        .collect();
    for (loss, k) in tilted.into_iter().zip(tilted_of) {
        // Mostly on faces already rebuilt (a bevel, a countersink): a strip fit across their softened sides.
        let tris = &cylinders[k].triangles;
        if tris
            .iter()
            .filter(|t| explained_tris.contains(t) || on_big_face[**t as usize])
            .count()
            * 2
            >= tris.len()
        {
            continue;
        }
        // Or lying along a rebuilt bevel or fillet, nearly all of it within that edge's band: its softened sides.
        let verts: Vec<[f64; 3]> = cylinders[k]
            .vertices
            .iter()
            .map(|&i| to_part(local(pos(i))))
            .collect();
        let along_edge = chamfers.iter().any(|(c, _)| {
            let inside = verts
                .iter()
                .filter(|p| {
                    let d = [p[0] - c.a[0], p[1] - c.a[1]];
                    let (s, t) = (
                        d[0] * c.out[0] + d[1] * c.out[1],
                        d[0] * c.dir[0] + d[1] * c.dir[1],
                    );
                    s <= reach
                        && s >= -c.across - reach
                        && p[2] >= c.top - c.down - reach
                        && t >= -reach
                        && t <= c.len + reach
                })
                .count();
            inside * 10 >= verts.len() * 9
        });
        if along_edge {
            continue;
        }
        // A thin round lying along an edge is that edge softened, however long.
        // So is one whose arc is no wider than that, whatever radius noise gives its fit.
        let c = &cylinders[k].cylinder;
        let arc = c.coverage_deg.to_radians() * c.diameter / 2.0;
        // And a shallow arc (under 45°) is a near-flat strip by an edge, not a round a person drew.
        if cylinders[k].area < small
            || c.diameter / 2.0 <= reach
            || arc <= 2.0 * reach
            || c.coverage_deg < 45.0
        {
            if !cylinders[k]
                .triangles
                .iter()
                .all(|t| explained_tris.contains(t) || !slanted[*t as usize])
            {
                small_count += 1;
                small_area += cylinders[k].area;
            }
            continue;
        }
        if !cylinders[k]
            .triangles
            .iter()
            .all(|t| explained_tris.contains(t) || !slanted[*t as usize])
        {
            losses.push(loss);
        }
    }
    for (k, p) in planes.iter().enumerate() {
        let ang = acute_deg(p.plane.normal, n);
        if k != bi
            && ang > SQUARE_DEG
            && ang < 90.0 - SQUARE_DEG
            // Explained when three quarters of it is (a rebuilt bevel's, round's or countersink's triangles, or not
            // slanted): on a scan a face spills over where two of them meet.
            && p
                .triangles
                .iter()
                .filter(|t| explained_tris.contains(t) || !slanted[**t as usize])
                .count()
                * 4
                < p.triangles.len() * 3
        {
            if p.area < small {
                small_count += 1;
                small_area += p.area;
                continue;
            }
            losses.push(Loss {
                severity: "dropped",
                title: "Slanted faces not rebuilt",
                subject: format!("face {}", k + 1),
                message: format!("a {:.2} mm² face at {ang:.1}° to the base (a chamfer or draft) is not part of the rebuild", p.area),
                code: "slanted_face",
            });
        }
    }

    // The other parallel faces: pocket and step floors (cut), pads (joined), undercuts (reported).
    let mut pockets: Vec<Value> = Vec::new();
    let mut pads: Vec<Value> = Vec::new();
    for l in &levels {
        if l.index == ti || explained.contains(&l.index) {
            continue;
        }
        let (lx, ly) = ((l.lo[0] + l.hi[0]) / 2.0, (l.lo[1] + l.hi[1]) / 2.0);
        let in_band = chamfers.iter().any(|(c, _)| {
            let d = [lx - c.a[0], ly - c.a[1]];
            let (s, t) = (
                d[0] * c.out[0] + d[1] * c.out[1],
                d[0] * c.dir[0] + d[1] * c.dir[1],
            );
            s <= reach
                && s >= -c.across - reach
                && l.height >= c.top - c.down - reach
                && t >= -c.across - reach
                && t <= c.len + c.across + reach
        });
        let in_countersink = holes.iter().any(|h| {
            let c = &h["countersink"];
            c.is_object()
                && (lx - h["x"].as_f64().unwrap_or(f64::NAN))
                    .hypot(ly - h["y"].as_f64().unwrap_or(f64::NAN))
                    <= c["diameter"].as_f64().unwrap_or(0.0) / 2.0 + reach
                && l.height >= thick - c["depth"].as_f64().unwrap_or(0.0) - reach
        });
        if l.area < small
            && (in_band
                || in_countersink
                || l.height.abs() <= reach
                || (l.height - thick).abs() <= reach)
        {
            // A sliver on the softened rim of the base or the top.
            small_count += 1;
            small_area += l.area;
            continue;
        }
        let subject = format!("face {} at {:.3} mm", l.index + 1, l.height);
        let (w, d) = (l.hi[0] - l.lo[0], l.hi[1] - l.lo[1]);
        let simplified = l.area < full * w * d;
        // Its sides on its walls (a scan's floor stops short of them), into it for a pocket, out of it for a pad.
        let (rlo, rhi) = {
            let at = |p: [f64; 2]| [p[0] + lo[0], p[1] + lo[1]];
            let (a, b) = if l.height < thick {
                snap(at(l.lo), at(l.hi), l.height, thick, true)
            } else {
                snap(at(l.lo), at(l.hi), thick, l.height, false)
            };
            ([a[0] - lo[0], a[1] - lo[1]], [b[0] - lo[0], b[1] - lo[1]])
        };
        let rect = json!({"x": rlo[0], "y": rlo[1], "length": rhi[0] - rlo[0], "width": rhi[1] - rlo[1],
                          "height": l.height, "area_mm2": l.area});
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

    if small_count > 0 {
        losses.push(Loss {
            severity: "degraded",
            title: "Small slanted and rounded faces not rebuilt",
            subject: format!("{small_count} faces"),
            message: format!(
                "{small_count} faces of {small_area:.1} mm² in all, each under {small:.1} mm²: softened edges, or small bevels and rounds"
            ),
            code: "small_faces",
        });
    }
    if !rounded_corners.is_empty() {
        let (a, b) = rounded_corners
            .iter()
            .fold((f64::INFINITY, 0.0f64), |(a, b), &r| (a.min(r), b.max(r)));
        losses.push(Loss {
            severity: "degraded",
            title: "Rounded vertical edges left sharp",
            subject: format!("{} edges", rounded_corners.len()),
            message: format!("rounds square to the base covering less than {MIN_ROUND_COVERAGE_DEG}° of a circle, radius {a:.2} to {b:.2} mm"),
            code: "rounded_edges",
        });
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
        if circle_of(inner, ptol).is_some() {
            continue; // a round hole: its wall is a round region, cut above
        }
        let area = signed_area(inner).abs();
        let c = centroid(inner);
        let same = if rough > 2.0 { 0.1 } else { 0.03 };
        let matched = top_loops.iter().skip(1).any(|t| {
            let (tc, ta) = (centroid(t), signed_area(t).abs());
            (tc[0] - c[0]).hypot(tc[1] - c[1]) <= 2.0 * ptol && (ta - area).abs() <= same * area
        });
        let mut poly = simplify_loop(inner, ptol);
        if signed_area(&poly) < 0.0 {
            poly.reverse();
        }
        let poly = snap_poly(&poly, false);
        if matched
            && area >= small
            && (3..=MAX_OUTLINE_CORNERS).contains(&poly.len())
            && signed_area(&poly).abs() > 0.0
        {
            slot_area += signed_area(&poly).abs();
            slots.push(poly);
        }
    }

    // An outline that is not a full rectangle is simplified to its enclosing rectangle.
    let expected = outline_area - down_hole_area - slot_area;
    if base.area < full * expected {
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

    // The check: the scan's own points against the rebuilt part.
    let mut solid = Solid {
        body: Vec::new(),
        cuts: Vec::new(),
        cones: Vec::new(),
        chamfers: chamfers.iter().map(|c| c.0).collect(),
    };
    solid.body.push((
        match &outline {
            Outline::Rectangle => Shape::Rect([0.0, 0.0], [len, wid]),
            Outline::Circle { x, y, r } => Shape::Circle(*x, *y, *r),
            Outline::Polygon(p) => Shape::Poly(p.clone()),
        },
        0.0,
        thick,
    ));
    let g = |v: &Value, k: &str| v[k].as_f64().unwrap_or(0.0);
    for b in &pads {
        let (x, y) = (g(b, "x"), g(b, "y"));
        solid.body.push((
            Shape::Rect([x, y], [x + g(b, "length"), y + g(b, "width")]),
            thick,
            g(b, "height"),
        ));
    }
    for b in &bosses {
        solid.body.push((
            Shape::Circle(g(b, "x"), g(b, "y"), g(b, "diameter") / 2.0),
            g(b, "on"),
            g(b, "on") + g(b, "standing_height"),
        ));
    }
    for h in &holes {
        solid.cuts.push((
            Shape::Circle(g(h, "x"), g(h, "y"), g(h, "diameter") / 2.0),
            g(h, "from"),
            g(h, "to"),
        ));
        if h["countersink"].is_object() {
            let (r0, d, rt) = (
                g(h, "diameter") / 2.0,
                g(&h["countersink"], "depth"),
                g(&h["countersink"], "diameter") / 2.0,
            );
            solid.cones.push(Cone {
                x: g(h, "x"),
                y: g(h, "y"),
                z0: thick - d,
                r0,
                z1: thick + margin,
                r1: rt + margin * (rt - r0) / d,
            });
        }
    }
    for b in &pockets {
        let (x, y) = (g(b, "x"), g(b, "y"));
        solid.cuts.push((
            Shape::Rect([x, y], [x + g(b, "length"), y + g(b, "width")]),
            g(b, "height"),
            thick,
        ));
    }
    for poly in &slots {
        solid.cuts.push((Shape::Poly(poly.clone()), 0.0, thick));
    }
    let vertex_count = mesh.positions.len();
    let check_count = vertex_count.min(MAX_CHECK_POINTS);
    let distances = scan_distances(
        &solid,
        (0..check_count).map(|i| {
            // One vertex per equal interval of the input sequence. Use u64 on wasm32, too.
            let index = (i as u64 * vertex_count as u64 / check_count as u64) as u32;
            to_part(local(pos(index)))
        }),
    );
    let check = check_report(distances, tol);
    let p95 = check["p95_mm"].as_f64().unwrap_or(0.0);
    if p95 > 3.0 * tol {
        losses.push(Loss {
            severity: "degraded",
            title: "The rebuilt part is off the scan",
            subject: "whole part".into(),
            message: format!("5 % of the scan lies {p95:.2} mm or more from the rebuilt part; look at what was dropped above"),
            code: "scan_deviation",
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
    // Bevels of one size share a feature and its sizes.
    let mut sizes: Vec<(f64, f64, Vec<Chamfer>)> = Vec::new();
    let mut radii: Vec<(f64, f64, Vec<Chamfer>)> = Vec::new();
    for (c, _) in &chamfers {
        let list = if c.round { &mut radii } else { &mut sizes };
        match list
            .iter_mut()
            .find(|s| (s.0 - c.across).abs() <= tol && (s.1 - c.down).abs() <= tol)
        {
            Some(s) => s.2.push(*c),
            None => list.push((c.across, c.down, vec![*c])),
        }
    }
    for (i, (r, _, cs)) in radii.iter().enumerate() {
        let edges: Vec<Value> = cs
            .iter()
            .map(|c| {
                let t = c.len / 2.0;
                json!([
                    round_um(c.a[0] + c.dir[0] * t),
                    round_um(c.a[1] + c.dir[1] * t),
                    round_um(thick)
                ])
            })
            .collect();
        let p = format!("fillet{}_radius", i + 1);
        params.push(param(&p, *r));
        add(
            json!({"name": format!("Fillet{}", i + 1), "type": "fillet", "edges": edges, "radius": p}),
        );
    }
    for (i, (across, down, cs)) in sizes.iter().enumerate() {
        let edges: Vec<Value> = cs
            .iter()
            .map(|c| {
                let t = c.len / 2.0;
                json!([
                    round_um(c.a[0] + c.dir[0] * t),
                    round_um(c.a[1] + c.dir[1] * t),
                    round_um(thick)
                ])
            })
            .collect();
        let mut f = json!({"name": format!("Chamfer{}", i + 1), "type": "chamfer", "edges": edges});
        if (across - down).abs() <= tol {
            let p = format!("chamfer{}_size", i + 1);
            params.push(param(&p, (across + down) / 2.0));
            f["distance"] = json!(p);
        } else {
            let (pa, pd) = (
                format!("chamfer{}_width", i + 1),
                format!("chamfer{}_height", i + 1),
            );
            params.push(param(&pa, *across));
            params.push(param(&pd, *down));
            f["distance"] = json!(pa);
            f["distance2"] = json!(pd);
        }
        add(f);
    }
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
        // A boss on the plate's own top face follows the thickness; one on a pad or pocket floor stays put.
        let z: Value = if (f64of(b, "on") - thick).abs() <= tol {
            json!(format!("thickness - {}", mm(margin)))
        } else {
            json!(round_um(f64of(b, "z")))
        };
        add(json!({"name": format!("Boss{}", i + 1), "type": "cylinder",
            "base": [round_um(f64of(b, "x")), round_um(f64of(b, "y")), z],
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
        // Holes that open on the top follow the thickness, so editing it never leaves a hole short.
        let (z, height): (Value, Value) = match h["kind"].as_str() {
            Some("through") => (
                json!(round_um(f64of(h, "z"))),
                json!(format!("thickness + {}", mm(2.0 * margin))),
            ),
            Some("blind from above") => {
                let dp = format!("hole{}_depth", i + 1);
                params.push(param(&dp, thick - f64of(h, "from")));
                (
                    json!(format!("thickness - {dp}")),
                    json!(format!("{dp} + {}", mm(margin))),
                )
            }
            _ => (
                json!(round_um(f64of(h, "z"))),
                json!(mm(f64of(h, "height"))),
            ),
        };
        add(json!({"name": format!("Hole{}", i + 1), "type": "cylinder",
            "base": [round_um(f64of(h, "x")), round_um(f64of(h, "y")), z],
            "axis": [0.0, 0.0, 1.0], "radius": format!("{p} / 2"), "height": height,
            "operation": "cut"}));
        if h["countersink"].is_object() {
            let c = &h["countersink"];
            let (cd, ck, m) = (
                format!("hole{}_csk_diameter", i + 1),
                format!("hole{}_csk_depth", i + 1),
                mm(margin),
            );
            params.push(param(&cd, f64of(c, "diameter")));
            params.push(param(&ck, f64of(c, "depth")));
            // From the hole's wall up past the top face, keeping its slope.
            add(
                json!({"name": format!("Hole{}_Countersink", i + 1), "type": "cone",
                "base": [round_um(f64of(h, "x")), round_um(f64of(h, "y")), format!("thickness - {ck}")],
                "axis": [0.0, 0.0, 1.0], "radius": format!("{p} / 2"),
                "top_radius": format!("{cd} / 2 + {m} * ({cd} - {p}) / (2 * {ck})"),
                "height": format!("{ck} + {m}"), "operation": "cut"}),
            );
        }
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
            "fillets": chamfers.iter().filter(|(c, _)| c.round).map(|(c, _)| json!({"edge_from": c.a,
                "edge_direction": c.dir, "edge_length": c.len, "radius": c.across})).collect::<Vec<_>>(),
            "chamfers": chamfers.iter().filter(|(c, _)| !c.round).map(|(c, _)| json!({"edge_from": c.a, "edge_direction": c.dir,
                "edge_length": c.len, "width": c.across, "height": c.down})).collect::<Vec<_>>(),
            "check": check, "cutouts": slots, "holes": holes, "bosses": bosses, "pockets": pockets, "pads": pads,
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
    fn distance_to_a_plate_with_a_hole_and_a_boss() {
        // 10 x 10 x 4 plate, Ø2 through hole at (5, 5), Ø2 boss 3 high at (2, 2).
        let solid = Solid {
            body: vec![
                (Shape::Rect([0.0, 0.0], [10.0, 10.0]), 0.0, 4.0),
                (Shape::Circle(2.0, 2.0, 1.0), 4.0, 7.0),
            ],
            cuts: vec![(Shape::Circle(5.0, 5.0, 1.0), 0.0, 4.0)],
            cones: vec![],
            chamfers: vec![],
        };
        let d = |p| solid.distance(p);
        assert!((d([8.0, 8.0, 7.0]) - 3.0).abs() < 1e-9, "above the top");
        assert!(
            (d([8.0, 8.0, 2.0]) + 2.0).abs() < 1e-9,
            "inside, 2 from the top"
        );
        assert!(
            (d([5.0, 5.0, 2.0]) - 1.0).abs() < 1e-9,
            "in the hole, 1 from its wall"
        );
        assert!((d([2.0, 2.0, 7.5]) - 0.5).abs() < 1e-9, "over the boss");
        assert!((d([12.0, 5.0, 2.0]) - 2.0).abs() < 1e-9, "beside the plate");
    }

    #[test]
    fn a_cone_is_found_from_its_facets_and_measured() {
        // A 90° countersink: radius 3 at height 7 to radius 6 at height 10, axis at (20, 15), 48 facets.
        let (mut faces, mut points) = (Vec::new(), Vec::new());
        let at = |a: f64, r: f64, z: f64| [20.0 + r * a.cos(), 15.0 + r * a.sin(), z];
        for i in 0..48 {
            let (a, b) = (
                f64::from(i) * std::f64::consts::TAU / 48.0,
                f64::from(i + 1) * std::f64::consts::TAU / 48.0,
            );
            let m = (a + b) / 2.0;
            let c = at(m, 4.5, 8.5);
            faces.push((1.0, [c[0], c[1]], [-m.cos(), -m.sin()]));
            points.extend([
                at(a, 3.0, 7.0),
                at(b, 3.0, 7.0),
                at(a, 6.0, 10.0),
                at(b, 6.0, 10.0),
            ]);
        }
        let c = cone_of(&faces, &points, 1e-6).unwrap();
        assert!(
            (c.x - 20.0).abs() < 1e-9 && (c.y - 15.0).abs() < 1e-9,
            "{c:?}"
        );
        assert!((c.r0 - 3.0).abs() < 1e-9 && (c.r1 - 6.0).abs() < 1e-9);
        assert!((c.z0 - 7.0).abs() < 1e-9 && (c.z1 - 10.0).abs() < 1e-9);
        // Half of it does not go round: no cone.
        assert!(cone_of(&faces[..24], &points[..96], 1e-6).is_none());
        // Distances: on the axis below its base, inside, and beside its slanted wall.
        assert!((c.dist([20.0, 15.0, 5.0]) - 2.0).abs() < 1e-9);
        assert!(c.dist([20.0, 15.0, 8.5]) < 0.0);
        let wall = c.dist([20.0 + 4.5 + 2.0_f64.sqrt(), 15.0, 8.5]);
        assert!((wall - 1.0).abs() < 1e-9, "{wall}");
    }

    #[test]
    fn distance_to_a_bevel() {
        // The edge along +x from the origin, 10 long, outward -y; 2 across the top at z = 8, 1 down the wall.
        let c = Chamfer {
            a: [0.0, 0.0],
            dir: [1.0, 0.0],
            out: [0.0, -1.0],
            len: 10.0,
            across: 2.0,
            down: 1.0,
            top: 8.0,
            round: false,
        };
        // The old corner is inside what it takes away; a point on the bevel's face is on it.
        assert!(c.dist([5.0, 0.0, 8.0]) < 0.0);
        assert!(c.dist([5.0, 1.0, 7.5]).abs() < 1e-12);
        // 1 mm below the bevel's lower edge on the wall, the nearest point is past the plane: positive.
        assert!(c.dist([5.0, 0.0, 6.0]) > 0.0);
        // Beyond the edge's ends nothing is taken.
        assert!(c.dist([12.0, 0.0, 8.0]) >= 2.0 - 1e-12);
    }

    #[test]
    fn distance_to_a_fillet() {
        // The edge along +x, outward -y, top at 8, rounded R2: the arc's centre is at y = 2, z = 6.
        let c = Chamfer {
            a: [0.0, 0.0],
            dir: [1.0, 0.0],
            out: [0.0, -1.0],
            len: 10.0,
            across: 2.0,
            down: 2.0,
            top: 8.0,
            round: true,
        };
        let on_arc = [
            5.0,
            2.0 - 2.0 * 45f64.to_radians().cos(),
            6.0 + 2.0 * 45f64.to_radians().sin(),
        ];
        assert!(c.dist(on_arc).abs() < 1e-12);
        let corner = c.dist([5.0, 0.0, 8.0]);
        assert!((corner + (8f64.sqrt() - 2.0)).abs() < 1e-12, "{corner}");
        assert!(c.dist([5.0, 1.0, 6.5]) > 0.0, "inside the arc stays");
    }

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
