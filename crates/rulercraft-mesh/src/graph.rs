// SPDX-License-Identifier: MIT OR Apache-2.0
//! Derived normals and undirected topology for an indexed mesh.
use crate::{
    Mesh,
    math::vec3::{add, angle_between, cross, normalize, sub},
};
use std::{collections::BTreeMap, fmt};

/// Bounds on graph construction.
#[derive(Debug, Clone, Copy)]
pub struct GraphLimits {
    /// Maximum input vertices.
    pub max_vertices: usize,
    /// Maximum input triangles.
    pub max_triangles: usize,
}
impl Default for GraphLimits {
    fn default() -> Self {
        Self {
            max_vertices: 50_000_000,
            max_triangles: 50_000_000,
        }
    }
}
/// Invalid mesh data, work limit or angle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphError(String);
impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for GraphError {}
/// One undirected edge and all its incident triangle ids.
#[derive(Debug, Clone)]
pub struct Edge {
    /// Endpoints in increasing order.
    pub vertices: [u32; 2],
    /// Incident triangle ids in increasing order.
    pub triangles: Vec<usize>,
}
/// A component connected through shared vertices, including isolated vertices.
#[derive(Debug, Clone)]
pub struct Component {
    /// Vertex ids, sorted.
    pub vertices: Vec<u32>,
    /// Triangle ids, sorted.
    pub triangles: Vec<usize>,
}
/// Data derived once from a mesh, with the mesh's original ids preserved.
#[derive(Debug, Clone)]
pub struct MeshGraph {
    /// Unit triangle normals; None for zero area.
    pub triangle_normals: Vec<Option<[f64; 3]>>,
    /// Area-weighted unit vertex normals; None for zero sum.
    pub vertex_normals: Vec<Option<[f64; 3]>>,
    /// Sorted neighbouring vertex ids per vertex.
    pub vertex_neighbours: Vec<Vec<u32>>,
    /// Sorted incident triangle ids per vertex.
    pub vertex_triangles: Vec<Vec<usize>>,
    /// Edges sorted lexicographically by endpoints.
    pub edges: Vec<Edge>,
    /// Components ordered by lowest vertex id.
    pub components: Vec<Component>,
    /// Edge ids with exactly one incident triangle.
    pub boundary_edges: Vec<usize>,
    /// Closed degree-two boundary components, without repeated start vertices.
    pub boundary_loops: Vec<Vec<u32>>,
    /// Count of edges with more than two incident triangles.
    pub non_manifold_edges: usize,
}
impl MeshGraph {
    /// Builds derived data in linear work aside from sorting, refusing invalid input or limits.
    pub fn build(mesh: &Mesh, limits: GraphLimits) -> Result<Self, GraphError> {
        let nv = mesh.positions.len();
        if nv > limits.max_vertices
            || nv > u32::MAX as usize
            || mesh.triangles.len() > limits.max_triangles
        {
            return Err(GraphError(
                "mesh graph vertex or triangle limit exceeded".into(),
            ));
        }
        for (i, p) in mesh.positions.iter().enumerate() {
            if p.iter().any(|v| !v.is_finite()) {
                return Err(GraphError(format!("vertex {i}: position is not finite")));
            }
        }
        let mut sums = vec![[0.; 3]; nv];
        let mut vertex_triangles = vec![Vec::new(); nv];
        let mut triangle_normals = Vec::new();
        let mut edges: BTreeMap<[u32; 2], Vec<usize>> = BTreeMap::new();
        for (i, t) in mesh.triangles.iter().enumerate() {
            if t.iter().any(|v| *v as usize >= nv) || t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
                return Err(GraphError(format!(
                    "triangle {i}: invalid or repeated vertex index"
                )));
            }
            let [a, b, c] = t.map(|v| mesh.positions[v as usize].map(f64::from));
            let area = cross(sub(b, a), sub(c, a));
            triangle_normals.push(normalize(area));
            for &v in t {
                sums[v as usize] = add(sums[v as usize], area);
                vertex_triangles[v as usize].push(i);
            }
            for [a, b] in [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]] {
                edges.entry([a.min(b), a.max(b)]).or_default().push(i);
            }
        }
        let edges: Vec<_> = edges
            .into_iter()
            .map(|(vertices, triangles)| Edge {
                vertices,
                triangles,
            })
            .collect();
        let mut vertex_neighbours = vec![Vec::new(); nv];
        let mut boundary = vec![Vec::new(); nv];
        let mut boundary_edges = Vec::new();
        let mut non_manifold_edges = 0;
        for (i, e) in edges.iter().enumerate() {
            let [a, b] = e.vertices;
            vertex_neighbours[a as usize].push(b);
            vertex_neighbours[b as usize].push(a);
            if e.triangles.len() == 1 {
                boundary_edges.push(i);
                boundary[a as usize].push(b);
                boundary[b as usize].push(a);
            }
            non_manifold_edges += usize::from(e.triangles.len() > 2);
        }
        for v in &mut vertex_neighbours {
            v.sort_unstable();
        }
        for v in &mut boundary {
            v.sort_unstable();
        }
        let mut seen = vec![false; nv];
        let mut components = Vec::new();
        let mut triangle_seen = vec![false; mesh.triangles.len()];
        for start in 0..nv {
            if seen[start] {
                continue;
            }
            let mut stack = vec![start as u32];
            seen[start] = true;
            let mut vertices = Vec::new();
            let mut triangles = Vec::new();
            while let Some(v) = stack.pop() {
                vertices.push(v);
                for &t in &vertex_triangles[v as usize] {
                    if !triangle_seen[t] {
                        triangle_seen[t] = true;
                        triangles.push(t);
                    }
                }
                for &w in &vertex_neighbours[v as usize] {
                    if !seen[w as usize] {
                        seen[w as usize] = true;
                        stack.push(w);
                    }
                }
            }
            vertices.sort_unstable();
            triangles.sort_unstable();
            components.push(Component {
                vertices,
                triangles,
            });
        }
        // A closed boundary component has degree two at every vertex. Branches remain edges.
        let mut boundary_loops = Vec::new();
        seen.fill(false);
        for start in 0..nv {
            if seen[start] || boundary[start].is_empty() {
                continue;
            }
            let mut stack = vec![start as u32];
            seen[start] = true;
            let mut regular = true;
            while let Some(v) = stack.pop() {
                regular &= boundary[v as usize].len() == 2;
                for &w in &boundary[v as usize] {
                    if !seen[w as usize] {
                        seen[w as usize] = true;
                        stack.push(w);
                    }
                }
            }
            if !regular {
                continue;
            }
            let mut loop_vertices = vec![start as u32];
            let mut previous = start as u32;
            let mut current = boundary[start][0];
            while current != start as u32 {
                loop_vertices.push(current);
                let next = &boundary[current as usize];
                let following = if next[0] == previous {
                    next[1]
                } else {
                    next[0]
                };
                previous = current;
                current = following;
            }
            boundary_loops.push(loop_vertices);
        }
        Ok(Self {
            triangle_normals,
            vertex_normals: sums.into_iter().map(normalize).collect(),
            vertex_neighbours,
            vertex_triangles,
            edges,
            components,
            boundary_edges,
            boundary_loops,
            non_manifold_edges,
        })
    }
    /// True when no edge has exactly one incident triangle; non-manifold edges are separate.
    pub fn is_closed(&self) -> bool {
        self.boundary_edges.is_empty()
    }
    /// Edge ids where two nonzero triangle normals differ by more than `angle` radians.
    pub fn crease_edges(&self, angle: f64) -> Result<Vec<usize>, GraphError> {
        if !angle.is_finite() || !(0.0..=std::f64::consts::PI).contains(&angle) {
            return Err(GraphError(
                "crease angle must be finite radians from 0 through pi".into(),
            ));
        }
        Ok(self.creases(angle))
    }
    /// Crease edge ids at the default 30 degree threshold.
    pub fn default_crease_edges(&self) -> Vec<usize> {
        self.creases(std::f64::consts::PI / 6.)
    }
    fn creases(&self, angle: f64) -> Vec<usize> {
        self.edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let [a, b] = e.triangles.as_slice() else {
                    return None;
                };
                let a = self.triangle_normals[*a]?;
                let b = self.triangle_normals[*b]?;
                (angle_between(a, b)? > angle).then_some(i)
            })
            .collect()
    }
}
