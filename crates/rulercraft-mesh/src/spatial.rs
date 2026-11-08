// SPDX-License-Identifier: MIT OR Apache-2.0
//! One bounding-box tree over a mesh's triangles, built once and shared by closest-point queries
//! (deviation, ICP, symmetry) and ray casts (wall thickness, picking). Queries are exact: they return
//! the same answer as testing every triangle, only faster. Triangle numbers are the mesh's own.
use crate::Mesh;
use crate::math::{Vec3, dot, sub};

/// Closest point on triangle `abc` to `p` (exact, region by region).
pub use crate::deviation::closest_point_on_triangle;

struct Node {
    min: Vec3,
    max: Vec3,
    /// Leaf: a range of `order`. Inner: two child nodes.
    kind: Kind,
}
enum Kind {
    Leaf(usize, usize),
    Inner(usize, usize),
}

/// A bounding-box tree over the triangles of one mesh, in f64.
pub struct Bvh {
    tris: Vec<[Vec3; 3]>,
    order: Vec<usize>,
    nodes: Vec<Node>,
}

/// The closest surface point found by [`Bvh::closest`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Closest {
    /// The point on the surface, mm.
    pub point: Vec3,
    /// The mesh triangle it lies on. Ties go to whichever triangle the search met first.
    pub triangle: usize,
    /// Squared distance from the query point, mm².
    pub distance2: f64,
}

/// Largest triangle count in a leaf.
const LEAF: usize = 4;
/// Fewest triangles in a subtree worth handing to another thread.
const PAR_BUILD: usize = 32_768;

/// Builds the subtree over `order` (whose first entry is position `lo` of the whole order) and returns
/// its nodes in depth-first order, root first, children numbered within the returned list. Subtrees may
/// be built on up to `threads` threads; the nodes are the same for any count.
fn build(tris: &[[Vec3; 3]], order: &mut [usize], lo: usize, threads: usize) -> Vec<Node> {
    let n = order.len();
    let (mut mn, mut mx) = ([f64::MAX; 3], [f64::MIN; 3]);
    for &i in order.iter() {
        for p in tris[i] {
            for k in 0..3 {
                mn[k] = mn[k].min(p[k]);
                mx[k] = mx[k].max(p[k]);
            }
        }
    }
    if n <= LEAF {
        return vec![Node {
            min: mn,
            max: mx,
            kind: Kind::Leaf(lo, lo + n),
        }];
    }
    let axis = (0..3)
        .max_by(|&a, &b| (mx[a] - mn[a]).total_cmp(&(mx[b] - mn[b])))
        .unwrap_or(0);
    let centre = |i: usize| tris[i][0][axis] + tris[i][1][axis] + tris[i][2][axis];
    let mid = n / 2;
    // Median split; ties broken by triangle number so the tree never depends on sort details.
    order.select_nth_unstable_by(mid, |&a, &b| {
        centre(a).total_cmp(&centre(b)).then(a.cmp(&b))
    });
    let (left, right) = order.split_at_mut(mid);
    let (mut l, mut r) = if threads > 1 && n >= PAR_BUILD {
        let right_threads = threads - threads / 2;
        std::thread::scope(|s| {
            let h = s.spawn(move || build(tris, right, lo + mid, right_threads));
            let l = build(tris, left, lo, threads / 2);
            match h.join() {
                Ok(r) => (l, r),
                Err(e) => std::panic::resume_unwind(e),
            }
        })
    } else {
        (build(tris, left, lo, 1), build(tris, right, lo + mid, 1))
    };
    let (lo_off, ro_off) = (1, 1 + l.len());
    for (nodes, off) in [(&mut l, lo_off), (&mut r, ro_off)] {
        for node in nodes.iter_mut() {
            if let Kind::Inner(a, b) = node.kind {
                node.kind = Kind::Inner(a + off, b + off);
            }
        }
    }
    let mut out = Vec::with_capacity(1 + l.len() + r.len());
    out.push(Node {
        min: mn,
        max: mx,
        kind: Kind::Inner(lo_off, ro_off),
    });
    out.extend(l);
    out.extend(r);
    out
}

fn box_dist2(p: Vec3, lo: Vec3, hi: Vec3) -> f64 {
    (0..3)
        .map(|i| {
            let d = (lo[i] - p[i]).max(p[i] - hi[i]).max(0.0);
            d * d
        })
        .sum()
}

/// Entry distance of the ray into the box, or None when it misses or enters beyond `t_max`. Boxes are
/// widened by `pad` so that a triangle test with an edge allowance can never be pruned away.
fn box_entry(
    origin: Vec3,
    direction: Vec3,
    inv: Vec3,
    lo: Vec3,
    hi: Vec3,
    pad: f64,
    t_max: f64,
) -> Option<f64> {
    let (mut t0, mut t1) = (f64::NEG_INFINITY, t_max);
    for k in 0..3 {
        let (a, b) = (lo[k] - pad, hi[k] + pad);
        // Parallel rays on either boundary are inside the slab for both signs of zero.
        if direction[k] == 0.0 {
            if origin[k] < a || origin[k] > b {
                return None;
            }
            continue;
        }
        // A finite nonzero subnormal direction may also have an infinite reciprocal.
        // Division retains the exact zero for an origin on a boundary in that case.
        let (mut ta, mut tb) = if inv[k].is_infinite() {
            (
                (a - origin[k]) / direction[k],
                (b - origin[k]) / direction[k],
            )
        } else {
            ((a - origin[k]) * inv[k], (b - origin[k]) * inv[k])
        };
        if ta > tb {
            std::mem::swap(&mut ta, &mut tb);
        }
        t0 = t0.max(ta);
        t1 = t1.min(tb);
        if t0 > t1 {
            return None;
        }
    }
    // Wholly behind the start is handled by the caller's `t_min` filter; this only prunes.
    Some(t0)
}

impl Bvh {
    /// Builds the tree over every triangle of `mesh`. Triangle corners must index `mesh.positions`
    /// (a [`Mesh`] from the readers always does). A mesh without triangles gives an empty tree whose
    /// queries return nothing.
    pub fn new(mesh: &Mesh) -> Self {
        let tris: Vec<[Vec3; 3]> = mesh
            .triangles
            .iter()
            .map(|t| t.map(|i| mesh.positions[i as usize].map(f64::from)))
            .collect();
        let mut order: Vec<usize> = (0..tris.len()).collect();
        let nodes = if tris.is_empty() {
            vec![]
        } else {
            build(&tris, &mut order, 0, crate::par::threads())
        };
        Bvh { tris, order, nodes }
    }
    /// Number of tree nodes (leaves and inner).
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    /// Number of triangles indexed.
    pub fn len(&self) -> usize {
        self.tris.len()
    }
    /// True when there are no triangles.
    pub fn is_empty(&self) -> bool {
        self.tris.is_empty()
    }
    /// Corners of triangle `t`, in f64.
    pub fn triangle(&self, t: usize) -> [Vec3; 3] {
        self.tris[t]
    }
    /// The closest point on the surface to `p`, or None for an empty tree.
    pub fn closest(&self, p: Vec3) -> Option<Closest> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut best = Closest {
            point: [0.0; 3],
            triangle: 0,
            distance2: f64::INFINITY,
        };
        let mut stack = vec![0usize];
        while let Some(n) = stack.pop() {
            let node = &self.nodes[n];
            if box_dist2(p, node.min, node.max) >= best.distance2 {
                continue;
            }
            match node.kind {
                Kind::Leaf(lo, hi) => {
                    for &i in &self.order[lo..hi] {
                        let [a, b, c] = self.tris[i];
                        let q = closest_point_on_triangle(p, a, b, c);
                        let d = sub(p, q);
                        let d2 = dot(d, d);
                        if d2 < best.distance2 {
                            best = Closest {
                                point: q,
                                triangle: i,
                                distance2: d2,
                            };
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
        Some(best)
    }

    /// The smallest ray parameter in `(t_min, t_max]` that `hit` reports for any triangle the ray
    /// `origin + t * direction` can reach, with that triangle. `hit(triangle, corners)` decides what
    /// counts as a hit (facing, edge allowance, skipping the start triangle) and returns its `t`; the
    /// tree only skips triangles whose box, widened by `pad` mm, the ray cannot touch within the range.
    /// The answer equals testing every triangle with `hit` whenever `hit` only accepts points within
    /// `pad` of the triangle. `direction` need not be unit length; `t` is in its units.
    pub fn ray_nearest(
        &self,
        origin: Vec3,
        direction: Vec3,
        t_min: f64,
        t_max: f64,
        pad: f64,
        mut hit: impl FnMut(usize, [Vec3; 3]) -> Option<f64>,
    ) -> Option<(f64, usize)> {
        if self.nodes.is_empty() {
            return None;
        }
        let inv = direction.map(|d| 1.0 / d);
        let mut best: Option<(f64, usize)> = None;
        let mut stack = vec![0usize];
        while let Some(n) = stack.pop() {
            let node = &self.nodes[n];
            let limit = best.map_or(t_max, |b| b.0);
            if box_entry(origin, direction, inv, node.min, node.max, pad, limit).is_none() {
                continue;
            }
            match node.kind {
                Kind::Leaf(lo, hi) => {
                    for &i in &self.order[lo..hi] {
                        if let Some(t) = hit(i, self.tris[i]).filter(|t| *t > t_min && *t <= t_max)
                        {
                            // Equal distances keep the lower triangle number, whatever the visit order.
                            if best.is_none_or(|(bt, bi)| t < bt || (t == bt && i < bi)) {
                                best = Some((t, i));
                            }
                        }
                    }
                }
                Kind::Inner(l, r) => {
                    let el = box_entry(
                        origin,
                        direction,
                        inv,
                        self.nodes[l].min,
                        self.nodes[l].max,
                        pad,
                        limit,
                    );
                    let er = box_entry(
                        origin,
                        direction,
                        inv,
                        self.nodes[r].min,
                        self.nodes[r].max,
                        pad,
                        limit,
                    );
                    match (el, er) {
                        (Some(a), Some(b)) if a <= b => stack.extend([r, l]),
                        (Some(_), Some(_)) => stack.extend([l, r]),
                        (Some(_), None) => stack.push(l),
                        (None, Some(_)) => stack.push(r),
                        (None, None) => {}
                    }
                }
            }
        }
        best
    }
}
