// SPDX-License-Identifier: MIT OR Apache-2.0
//! Connectivity and normals built once from a [`Mesh`] (docs/specs/mesh-model.md).

use crate::math::{Vec3, cross, dot, length, sub};
use crate::mesh::Mesh;

/// An undirected edge `a < b` with the triangles that use it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    /// Lower vertex index.
    pub a: u32,
    /// Higher vertex index.
    pub b: u32,
    /// First triangle using the edge.
    pub t0: u32,
    /// Second triangle, if any.
    pub t1: Option<u32>,
    /// More than two triangles use this edge.
    pub non_manifold: bool,
}

/// A mesh with its normals and connectivity.
#[derive(Debug, Clone)]
pub struct MeshGraph {
    /// Unit triangle normals from the winding (zero for a zero-area triangle).
    pub face_normals: Vec<Vec3>,
    /// Area-weighted unit vertex normals; `[0, 0, 0]` where the adjacent triangles cancel or none exist.
    pub vertex_normals: Vec<[f32; 3]>,
    /// Every edge once, sorted by (a, b).
    pub edges: Vec<Edge>,
    /// Component number of each vertex; `u32::MAX` for a vertex no triangle uses.
    pub component: Vec<u32>,
    /// Vertex count of each component, in component-number order.
    pub component_sizes: Vec<usize>,
    vertex_triangle_start: Vec<u32>,
    vertex_triangles: Vec<u32>,
    vertex_neighbour_start: Vec<u32>,
    vertex_neighbours: Vec<u32>,
}

impl MeshGraph {
    /// Builds the graph. Linear in mesh size apart from one sort of the edges.
    pub fn build(mesh: &Mesh) -> MeshGraph {
        let nv = mesh.positions.len();
        let p = |i: u32| mesh.positions[i as usize].map(f64::from);

        let mut face_normals = Vec::with_capacity(mesh.triangles.len());
        let mut sums = vec![[0.0f64; 3]; nv];
        for t in &mesh.triangles {
            let n = cross(sub(p(t[1]), p(t[0])), sub(p(t[2]), p(t[0])));
            for &v in t {
                let s = &mut sums[v as usize];
                *s = [s[0] + n[0], s[1] + n[1], s[2] + n[2]];
            }
            let l = length(n);
            face_normals.push(if l > 0.0 {
                [n[0] / l, n[1] / l, n[2] / l]
            } else {
                [0.0; 3]
            });
        }
        let vertex_normals = sums
            .iter()
            .map(|s| {
                let l = length(*s);
                if l > 0.0 {
                    s.map(|x| (x / l) as f32)
                } else {
                    [0.0; 3]
                }
            })
            .collect();

        let (vertex_triangle_start, vertex_triangles) = csr(
            nv,
            mesh.triangles
                .iter()
                .enumerate()
                .flat_map(|(ti, t)| t.iter().map(move |&v| (v, ti as u32))),
        );

        let mut half: Vec<(u32, u32, u32)> = Vec::with_capacity(mesh.triangles.len() * 3);
        for (ti, t) in mesh.triangles.iter().enumerate() {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                half.push((a.min(b), a.max(b), ti as u32));
            }
        }
        half.sort_unstable();
        let mut edges: Vec<Edge> = Vec::new();
        for (a, b, t) in half {
            match edges.last_mut() {
                Some(e) if e.a == a && e.b == b => {
                    if e.t1.is_none() {
                        e.t1 = Some(t);
                    } else {
                        e.non_manifold = true;
                    }
                }
                _ => edges.push(Edge {
                    a,
                    b,
                    t0: t,
                    t1: None,
                    non_manifold: false,
                }),
            }
        }
        let (vertex_neighbour_start, vertex_neighbours) =
            csr(nv, edges.iter().flat_map(|e| [(e.a, e.b), (e.b, e.a)]));

        // Components by union-find over triangle corners.
        let mut parent: Vec<u32> = (0..nv as u32).collect();
        fn root(parent: &mut [u32], mut i: u32) -> u32 {
            while parent[i as usize] != i {
                parent[i as usize] = parent[parent[i as usize] as usize];
                i = parent[i as usize];
            }
            i
        }
        for t in &mesh.triangles {
            for k in 1..3 {
                let (x, y) = (root(&mut parent, t[0]), root(&mut parent, t[k]));
                if x != y {
                    parent[x.max(y) as usize] = x.min(y);
                }
            }
        }
        let used: Vec<bool> = (0..nv)
            .map(|v| vertex_triangle_start[v] != vertex_triangle_start[v + 1])
            .collect();
        let mut number = vec![u32::MAX; nv];
        let mut component = vec![u32::MAX; nv];
        let mut component_sizes = Vec::new();
        for v in 0..nv {
            if !used[v] {
                continue;
            }
            let r = root(&mut parent, v as u32) as usize;
            if number[r] == u32::MAX {
                number[r] = component_sizes.len() as u32;
                component_sizes.push(0);
            }
            component[v] = number[r];
            component_sizes[number[r] as usize] += 1;
        }

        MeshGraph {
            face_normals,
            vertex_normals,
            edges,
            component,
            component_sizes,
            vertex_triangle_start,
            vertex_triangles,
            vertex_neighbour_start,
            vertex_neighbours,
        }
    }

    /// Triangles that use vertex `v`.
    pub fn triangles_of(&self, v: u32) -> &[u32] {
        let (s, e) = (
            self.vertex_triangle_start[v as usize],
            self.vertex_triangle_start[v as usize + 1],
        );
        &self.vertex_triangles[s as usize..e as usize]
    }

    /// Vertices joined to `v` by an edge, ascending.
    pub fn neighbours_of(&self, v: u32) -> &[u32] {
        let (s, e) = (
            self.vertex_neighbour_start[v as usize],
            self.vertex_neighbour_start[v as usize + 1],
        );
        &self.vertex_neighbours[s as usize..e as usize]
    }

    /// Edges used by more than two triangles.
    pub fn non_manifold_edges(&self) -> usize {
        self.edges.iter().filter(|e| e.non_manifold).count()
    }

    /// Closed loops of open-boundary edges (edges with exactly one triangle), as vertex sequences.
    /// Open chains on non-manifold boundaries are not returned. To check that no
    /// boundary exists, inspect `edges` for entries with `t1 == None` instead.
    pub fn boundary_loops(&self) -> Vec<Vec<u32>> {
        let open: Vec<&Edge> = self.edges.iter().filter(|e| e.t1.is_none()).collect();
        let nv = self.component.len();
        let (start, next) = csr(nv, open.iter().flat_map(|e| [(e.a, e.b), (e.b, e.a)]));
        let mut used = vec![false; open.len()];
        let index = |a: u32, b: u32| {
            open.binary_search_by(|e| (e.a, e.b).cmp(&(a.min(b), a.max(b))))
                .ok()
        };
        let mut loops = Vec::new();
        for first in 0..open.len() {
            if used[first] {
                continue;
            }
            used[first] = true;
            let (begin, mut at) = (open[first].a, open[first].b);
            let mut chain = vec![begin];
            while at != begin {
                chain.push(at);
                let around = &next[start[at as usize] as usize..start[at as usize + 1] as usize];
                let step = around
                    .iter()
                    .copied()
                    .find(|&w| index(at, w).is_some_and(|i| !used[i]));
                let Some(w) = step else { break };
                if let Some(i) = index(at, w) {
                    used[i] = true;
                }
                at = w;
            }
            if at == begin {
                loops.push(chain);
            }
        }
        loops
    }

    /// Edges whose two triangles meet at more than `angle` radians (sharp edges).
    pub fn crease_edges(&self, angle: f64) -> Vec<usize> {
        let limit = angle.cos();
        self.edges
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.t1.is_some_and(|t1| {
                    let (n0, n1) = (
                        self.face_normals[e.t0 as usize],
                        self.face_normals[t1 as usize],
                    );
                    length(n0) > 0.0 && length(n1) > 0.0 && dot(n0, n1) < limit
                })
            })
            .map(|(i, _)| i)
            .collect()
    }
}

/// Compressed adjacency lists from (key, value) pairs, values ascending within each key.
fn csr(n: usize, pairs: impl Iterator<Item = (u32, u32)> + Clone) -> (Vec<u32>, Vec<u32>) {
    let mut start = vec![0u32; n + 1];
    for (k, _) in pairs.clone() {
        start[k as usize + 1] += 1;
    }
    for i in 0..n {
        start[i + 1] += start[i];
    }
    let mut fill = start.clone();
    let mut values = vec![0u32; start[n] as usize];
    for (k, v) in pairs {
        values[fill[k as usize] as usize] = v;
        fill[k as usize] += 1;
    }
    for i in 0..n {
        values[start[i] as usize..start[i + 1] as usize].sort_unstable();
    }
    (start, values)
}
