// SPDX-License-Identifier: MIT OR Apache-2.0
//! Known-answer topology and normal checks; no measured reference numbers.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::{
    Mesh,
    graph::{GraphLimits, MeshGraph},
};
fn build(m: &Mesh) -> MeshGraph {
    MeshGraph::build(m, GraphLimits::default()).unwrap()
}
fn square() -> Mesh {
    Mesh {
        positions: vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        triangles: vec![[0, 1, 2], [0, 2, 3]],
        ..Default::default()
    }
}
fn cube() -> Mesh {
    let positions = (0..8)
        .map(|i| [(i & 1) as f32, ((i >> 1) & 1) as f32, ((i >> 2) & 1) as f32])
        .collect();
    let mut triangles = Vec::new();
    for q in [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ] {
        triangles.extend([[q[0], q[1], q[2]], [q[0], q[2], q[3]]]);
    }
    Mesh {
        positions,
        triangles,
        ..Default::default()
    }
}
#[test]
fn cube_has_known_topology_and_creases() {
    let g = build(&cube());
    assert_eq!(g.edges.len(), 18);
    assert_eq!(g.components.len(), 1);
    assert_eq!(
        (
            g.components[0].vertices.len(),
            g.components[0].triangles.len()
        ),
        (8, 12)
    );
    assert!(g.boundary_loops.is_empty());
    assert!(g.is_closed());
    assert_eq!(g.non_manifold_edges, 0);
    assert_eq!(g.default_crease_edges().len(), 12);
    assert_eq!(g.vertex_neighbours[0], vec![1, 2, 3, 4, 5, 6]);
    for n in g.vertex_normals.iter().flatten() {
        // f64 normalization roundoff only; integer coordinate inputs are exact.
        assert!((n.iter().map(|v| v * v).sum::<f64>() - 1.).abs() < 1e-14);
    }
}
#[test]
fn square_loop_and_disconnected_isolated_vertex() {
    let mut m = square();
    m.positions.push([9., 9., 9.]);
    let g = build(&m);
    assert_eq!(g.edges.len(), 5);
    assert_eq!(g.boundary_edges.len(), 4);
    assert_eq!(g.boundary_loops, vec![vec![0, 1, 2, 3]]);
    assert!(!g.is_closed());
    assert_eq!(g.components.len(), 2);
    assert_eq!(g.components[1].vertices, vec![4]);
    assert_eq!(g.vertex_normals[4], None);
    assert_eq!(g.vertex_triangles[0], vec![0, 1]);
    assert!(g.default_crease_edges().is_empty());
    assert_eq!(g.vertex_normals[0], Some([0., 0., 1.]));
}
#[test]
fn area_weighting_cancellation_and_nonmanifold() {
    let mut m = Mesh {
        positions: vec![[0., 0., 0.], [2., 0., 0.], [0., 2., 0.], [0., 0., 1.]],
        triangles: vec![[0, 1, 2], [0, 3, 1]],
        ..Default::default()
    };
    let n = build(&m).vertex_normals[0].unwrap();
    for (actual, expected) in n
        .into_iter()
        .zip([0., 2. / 20f64.sqrt(), 4. / 20f64.sqrt()])
    {
        assert!((actual - expected).abs() < 1e-14); // normalization roundoff only
    }
    m.triangles = vec![[0, 1, 2], [0, 2, 1]];
    assert_eq!(build(&m).vertex_normals[0], None);
    m.triangles.push([0, 1, 3]);
    assert_eq!(build(&m).non_manifold_edges, 1);
    m.positions[2] = [4., 0., 0.];
    m.triangles = vec![[0, 1, 2]];
    assert_eq!(build(&m).triangle_normals, vec![None]);
}
#[test]
fn invalid_mesh_limits_and_angles_refuse() {
    let m = square();
    for limits in [
        GraphLimits {
            max_vertices: 3,
            ..Default::default()
        },
        GraphLimits {
            max_triangles: 1,
            ..Default::default()
        },
    ] {
        assert!(
            MeshGraph::build(&m, limits)
                .unwrap_err()
                .to_string()
                .contains("limit")
        );
    }
    for t in [[0, 1, 99], [0, 0, 1]] {
        let mut bad = m.clone();
        bad.triangles[0] = t;
        assert!(
            MeshGraph::build(&bad, GraphLimits::default())
                .unwrap_err()
                .to_string()
                .contains("triangle 0")
        );
    }
    let mut bad = m.clone();
    bad.positions[0][0] = f32::NAN;
    assert!(MeshGraph::build(&bad, GraphLimits::default()).is_err());
    for a in [-1., f64::NAN, 4.] {
        assert!(build(&m).crease_edges(a).is_err());
    }
    assert!(build(&Mesh::default()).is_closed());
}
