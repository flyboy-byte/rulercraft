// SPDX-License-Identifier: MIT OR Apache-2.0
//! Mesh-to-mesh deviation (docs/specs/deviation.md): per-vertex signed distance from a mesh to a
//! reference mesh, with statistics. Distance is exact (closest point on the reference surface, found
//! with a bounding-box tree). The sign comes from the face normal of the closest reference triangle:
//! positive on the side that normal points to. Beside an edge or corner the sign follows whichever
//! triangle was closest; the size of the distance never depends on it. Lengths in mm.
use crate::Mesh;
use crate::math::{Vec3, add, cross, dot, length, scale, sub};

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

struct Node {
    min: Vec3,
    max: Vec3,
    /// Triangle range for a leaf, or children for an inner node.
    kind: Kind,
}
enum Kind {
    Leaf(usize, usize),
    Inner(usize, usize),
}

struct Tree<'a> {
    tris: Vec<[Vec3; 3]>,
    order: Vec<usize>,
    nodes: Vec<Node>,
    _mesh: &'a Mesh,
}

fn box_dist2(p: Vec3, lo: Vec3, hi: Vec3) -> f64 {
    (0..3)
        .map(|i| {
            let d = (lo[i] - p[i]).max(p[i] - hi[i]).max(0.0);
            d * d
        })
        .sum()
}

impl<'a> Tree<'a> {
    fn build(mesh: &'a Mesh) -> Self {
        let tris: Vec<[Vec3; 3]> = mesh
            .triangles
            .iter()
            .map(|t| t.map(|i| v3(mesh.positions[i as usize])))
            .collect();
        let mut t = Tree {
            order: (0..tris.len()).collect(),
            tris,
            nodes: vec![],
            _mesh: mesh,
        };
        let n = t.order.len();
        t.node(0, n);
        t
    }
    fn node(&mut self, lo: usize, hi: usize) -> usize {
        let (mut mn, mut mx) = ([f64::MAX; 3], [f64::MIN; 3]);
        for &i in &self.order[lo..hi] {
            for p in self.tris[i] {
                for k in 0..3 {
                    mn[k] = mn[k].min(p[k]);
                    mx[k] = mx[k].max(p[k]);
                }
            }
        }
        let id = self.nodes.len();
        self.nodes.push(Node {
            min: mn,
            max: mx,
            kind: Kind::Leaf(lo, hi),
        });
        if hi - lo > 4 {
            let axis = (0..3)
                .max_by(|&a, &b| (mx[a] - mn[a]).total_cmp(&(mx[b] - mn[b])))
                .unwrap_or(0);
            let tris = &self.tris;
            let centre = |i: &usize| tris[*i][0][axis] + tris[*i][1][axis] + tris[*i][2][axis];
            self.order[lo..hi].sort_by(|a, b| centre(a).total_cmp(&centre(b)));
            let mid = lo + (hi - lo) / 2;
            let l = self.node(lo, mid);
            let r = self.node(mid, hi);
            self.nodes[id].kind = Kind::Inner(l, r);
        }
        id
    }
    /// Closest point and the index of its triangle.
    fn closest(&self, p: Vec3) -> (Vec3, usize) {
        let (mut best, mut best_d2, mut best_t) = ([0.0; 3], f64::MAX, 0);
        let mut stack = vec![0usize];
        while let Some(n) = stack.pop() {
            let node = &self.nodes[n];
            if box_dist2(p, node.min, node.max) >= best_d2 {
                continue;
            }
            match node.kind {
                Kind::Leaf(lo, hi) => {
                    for &i in &self.order[lo..hi] {
                        let [a, b, c] = self.tris[i];
                        let q = closest_point_on_triangle(p, a, b, c);
                        let d = sub(p, q);
                        let d2 = dot(d, d);
                        if d2 < best_d2 {
                            (best, best_d2, best_t) = (q, d2, i);
                        }
                    }
                }
                Kind::Inner(l, r) => {
                    let (dl, dr) = (
                        box_dist2(p, self.nodes[l].min, self.nodes[l].max),
                        box_dist2(p, self.nodes[r].min, self.nodes[r].max),
                    );
                    // Visit the nearer child first: push it last.
                    if dl <= dr {
                        stack.extend([r, l]);
                    } else {
                        stack.extend([l, r]);
                    }
                }
            }
        }
        (best, best_t)
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
    let tree = Tree::build(reference);
    let distances: Vec<f64> = mesh
        .positions
        .iter()
        .map(|&p| {
            let p = v3(p);
            let (q, t) = tree.closest(p);
            let [a, b, c] = tree.tris[t];
            let n = cross(sub(b, a), sub(c, a));
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
