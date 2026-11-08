// SPDX-License-Identifier: MIT OR Apache-2.0
//! Mesh-to-mesh deviation (docs/specs/deviation.md): per-vertex signed distance from a mesh to a
//! reference mesh, with statistics. Distance is exact (closest point on the reference surface, found
//! with a bounding-box tree). The sign comes from the face normal of the closest reference triangle:
//! positive on the side that normal points to. Beside an edge or corner the sign follows whichever
//! triangle was closest; the size of the distance never depends on it. Lengths in mm.
use crate::Mesh;
use crate::math::{Vec3, add, cross, dot, length, scale, sub};
use crate::spatial::{Bvh, Closest};

/// Why a deviation could not be computed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviationError(pub String);
impl std::fmt::Display for DeviationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for DeviationError {}

/// Summary of a deviation map.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DeviationStats {
    /// Vertices measured.
    pub count: usize,
    /// Mean signed distance, mm.
    pub mean: f64,
    /// Root mean square of the distances, mm.
    pub rms: f64,
    /// Largest positive distance (0 when none are positive), mm.
    pub max_outside: f64,
    /// Most negative distance as a positive number (0 when none are negative), mm.
    pub max_inside: f64,
    /// Median of the absolute distances, mm.
    pub p50_abs: f64,
    /// 95th percentile of the absolute distances (nearest rank), mm.
    pub p95_abs: f64,
    /// Vertices whose absolute distance exceeds the tolerance.
    pub beyond_tolerance: usize,
}

/// A deviation map.
#[derive(Debug, Clone, PartialEq)]
pub struct Deviation {
    /// Signed distance per vertex of the measured mesh, mm.
    pub distances: Vec<f64>,
    /// Summary.
    pub stats: DeviationStats,
}

fn v3(p: [f32; 3]) -> Vec3 {
    p.map(f64::from)
}

/// Closest point to `p` on triangle `abc`, and its barycentric weights are not needed.
pub fn closest_point_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return add(a, scale(ab, d1 / (d1 - d3)));
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return add(a, scale(ac, d2 / (d2 - d6)));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return add(b, scale(sub(c, b), w));
    }
    let denom = 1.0 / (va + vb + vc);
    add(a, add(scale(ab, vb * denom), scale(ac, vc * denom)))
}

/// A reference mesh indexed for closest-point queries (a [`Bvh`] that refuses an empty mesh).
pub struct Reference {
    tree: Bvh,
}

impl Reference {
    /// Indexes `mesh`. Fails when it has no triangles.
    pub fn new(mesh: &Mesh) -> Result<Self, DeviationError> {
        if mesh.triangles.is_empty() {
            return Err(DeviationError("the reference mesh has no triangles".into()));
        }
        Ok(Self {
            tree: Bvh::new(mesh),
        })
    }
    /// The shared tree underneath.
    pub fn tree(&self) -> &Bvh {
        &self.tree
    }
    /// The closest point on the surface to `p`, and the (not unit) face normal of its triangle.
    pub fn closest(&self, p: Vec3) -> (Vec3, Vec3) {
        // Not empty: `new` refused a mesh without triangles.
        let hit = self.tree.closest(p).unwrap_or(Closest {
            point: p,
            triangle: 0,
            distance2: 0.0,
        });
        let [a, b, c] = self.tree.triangle(hit.triangle);
        (hit.point, cross(sub(b, a), sub(c, a)))
    }
}

/// Signed distance of every vertex of `mesh` to `reference`, with statistics. `tolerance` (mm) only
/// sets the `beyond_tolerance` count. Fails when the reference has no triangles or `tolerance` is not
/// a finite non-negative number.
pub fn deviation(
    mesh: &Mesh,
    reference: &Mesh,
    tolerance: f64,
) -> Result<Deviation, DeviationError> {
    if reference.triangles.is_empty() {
        return Err(DeviationError("the reference mesh has no triangles".into()));
    }
    if !tolerance.is_finite() || tolerance < 0.0 {
        return Err(DeviationError(
            "tolerance must be a finite number, 0 or more".into(),
        ));
    }
    let tree = Reference::new(reference)?;
    let distances: Vec<f64> = mesh
        .positions
        .iter()
        .map(|&p| {
            let p = v3(p);
            let (q, n) = tree.closest(p);
            let d = sub(p, q);
            let side = if dot(d, n) < 0.0 { -1.0 } else { 1.0 };
            side * length(d)
        })
        .collect();
    let count = distances.len();
    let mut abs: Vec<f64> = distances.iter().map(|d| d.abs()).collect();
    abs.sort_by(f64::total_cmp);
    let rank = |q: f64| {
        if abs.is_empty() {
            0.0
        } else {
            abs[((q * abs.len() as f64).ceil() as usize).clamp(1, abs.len()) - 1]
        }
    };
    let n = count.max(1) as f64;
    let stats = DeviationStats {
        count,
        mean: distances.iter().sum::<f64>() / n,
        rms: (distances.iter().map(|d| d * d).sum::<f64>() / n).sqrt(),
        max_outside: distances.iter().fold(0.0, |m, &d| m.max(d)),
        max_inside: distances.iter().fold(0.0, |m, &d| m.max(-d)),
        p50_abs: rank(0.5),
        p95_abs: rank(0.95),
        beyond_tolerance: abs.iter().filter(|&&d| d > tolerance).count(),
    };
    Ok(Deviation { distances, stats })
}

/// A connected group of vertices on the same side of the reference, all beyond the tolerance.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Patch {
    /// Vertex ids, ascending.
    pub vertices: Vec<u32>,
    /// True when the patch stands outside the reference (positive distances).
    pub outside: bool,
    /// Mean signed distance of the patch, mm.
    pub mean: f64,
    /// Largest absolute distance in the patch, mm.
    pub max_abs: f64,
}

/// Groups vertices whose absolute distance exceeds `tolerance` into patches: vertices joined by a mesh
/// edge and on the same side. Largest first; equal sizes by lowest vertex id. `distances` must have one
/// entry per vertex of the graph's mesh.
pub fn patches(
    graph: &crate::graph::MeshGraph,
    distances: &[f64],
    tolerance: f64,
) -> Result<Vec<Patch>, DeviationError> {
    if !tolerance.is_finite() || tolerance < 0.0 {
        return Err(DeviationError(
            "tolerance must be a finite number, 0 or more".into(),
        ));
    }
    let n = distances.len();
    let bad = |i: usize| distances[i].abs() > tolerance && distances[i].is_finite();
    let mut seen = vec![false; n];
    let mut out = vec![];
    for s in 0..n {
        if seen[s] || !bad(s) {
            continue;
        }
        let side = distances[s] > 0.0;
        let mut stack = vec![s as u32];
        seen[s] = true;
        let mut vertices = vec![];
        while let Some(v) = stack.pop() {
            vertices.push(v);
            for &u in graph.neighbours_of(v) {
                let u = u as usize;
                if u < n && !seen[u] && bad(u) && (distances[u] > 0.0) == side {
                    seen[u] = true;
                    stack.push(u as u32);
                }
            }
        }
        vertices.sort_unstable();
        let k = vertices.len() as f64;
        out.push(Patch {
            outside: side,
            mean: vertices.iter().map(|&v| distances[v as usize]).sum::<f64>() / k,
            max_abs: vertices
                .iter()
                .fold(0.0, |m, &v| f64::max(m, distances[v as usize].abs())),
            vertices,
        });
    }
    out.sort_by(|a, b| {
        b.vertices
            .len()
            .cmp(&a.vertices.len())
            .then(a.vertices[0].cmp(&b.vertices[0]))
    });
    Ok(out)
}
