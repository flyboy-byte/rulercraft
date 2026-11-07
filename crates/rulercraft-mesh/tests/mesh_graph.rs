// SPDX-License-Identifier: MIT OR Apache-2.0
//! mesh.graph checks: shapes whose counts are known exactly (docs/specs/mesh-model.md).
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::graph::MeshGraph;
use rulercraft_mesh::math::dot;
use rulercraft_mesh::{Limits, Mesh, Unit, read_stl};

/// A 2 mm cube read through the STL reader, wound so its normals point outward.
fn cube() -> Mesh {
    let c = |i: u32| {
        [
            (i & 1) as f32 * 2.0,
            ((i >> 1) & 1) as f32 * 2.0,
            ((i >> 2) & 1) as f32 * 2.0,
        ]
    };
    // Each face's corners counter-clockwise seen from outside.
    let faces = [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ];
    let mut s = String::from("solid c\n");
    for q in faces {
        for t in [[q[0], q[1], q[2]], [q[0], q[2], q[3]]] {
            s += "facet normal 0 0 0\nouter loop\n";
            for k in t {
                let v = c(k);
                s += &format!("vertex {} {} {}\n", v[0], v[1], v[2]);
            }
            s += "endloop\nendfacet\n";
        }
    }
    read_stl(s.as_bytes(), Unit::Mm, Limits::default()).unwrap()
}

fn mesh(positions: Vec<[f32; 3]>, triangles: Vec<[u32; 3]>) -> Mesh {
    Mesh {
        positions,
        triangles,
        degenerate_dropped: 0,
    }
}

#[test]
fn cube_counts_and_outward_normals() {
    let m = cube();
    let g = MeshGraph::build(&m);
    assert_eq!(
        (m.positions.len(), g.edges.len(), m.triangles.len()),
        (8, 18, 12)
    );
    assert_eq!(g.component_sizes, vec![8]);
    assert!(g.boundary_loops().is_empty(), "a cube is closed");
    assert_eq!(g.non_manifold_edges(), 0);
    assert_eq!(g.crease_edges(30f64.to_radians()).len(), 12);
    for (v, n) in g.vertex_normals.iter().enumerate() {
        let out = m.positions[v].map(|x| f64::from(x) - 1.0);
        assert!(
            dot(n.map(f64::from), out) > 0.5,
            "vertex {v} normal {n:?} does not point outward"
        );
        assert!((dot(n.map(f64::from), n.map(f64::from)) - 1.0).abs() < 1e-6);
    }
    let total: usize = (0..8).map(|v| g.neighbours_of(v).len()).sum();
    assert_eq!(
        total,
        2 * g.edges.len(),
        "every edge appears in both its vertices' neighbour lists"
    );
    let triangle_corners: usize = (0..8).map(|v| g.triangles_of(v).len()).sum();
    assert_eq!(triangle_corners, 36);
    for e in &g.edges {
        assert!(g.neighbours_of(e.a).contains(&e.b) && g.neighbours_of(e.b).contains(&e.a));
    }
}

#[test]
fn open_square_has_one_boundary_loop_of_four() {
    let g = MeshGraph::build(&mesh(
        vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        vec![[0, 1, 2], [0, 2, 3]],
    ));
    assert_eq!(g.edges.len(), 5);
    let loops = g.boundary_loops();
    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0].len(), 4);
    assert!(
        g.crease_edges(1f64.to_radians()).is_empty(),
        "a flat square has no creases"
    );
    assert_eq!(g.vertex_normals[2], [0.0, 0.0, 1.0]);
}

#[test]
fn open_tube_has_two_boundary_loops() {
    let (n, mut p, mut t) = (12u32, vec![], vec![]);
    for ring in 0..2 {
        for i in 0..n {
            let a = f64::from(i) * std::f64::consts::TAU / f64::from(n);
            p.push([a.cos() as f32, a.sin() as f32, ring as f32 * 3.0]);
        }
    }
    for i in 0..n {
        let j = (i + 1) % n;
        t.push([i, j, n + j]);
        t.push([i, n + j, n + i]);
    }
    let g = MeshGraph::build(&mesh(p, t));
    let mut sizes: Vec<usize> = g.boundary_loops().iter().map(Vec::len).collect();
    sizes.sort_unstable();
    assert_eq!(sizes, vec![12, 12]);
    assert_eq!(g.component_sizes, vec![24]);
}

#[test]
fn pieces_non_manifold_edges_zero_area_and_unused_vertices() {
    let two = MeshGraph::build(&mesh(
        vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [5., 0., 0.],
            [6., 0., 0.],
            [5., 1., 0.],
            [9., 9., 9.],
        ],
        vec![[0, 1, 2], [3, 4, 5]],
    ));
    assert_eq!(two.component_sizes, vec![3, 3]);
    assert_eq!(
        two.component[6],
        u32::MAX,
        "an unused vertex belongs to no component"
    );
    assert_eq!(two.vertex_normals[6], [0.0; 3]);

    let fin = MeshGraph::build(&mesh(
        vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., -1., 0.],
            [0., 0., 1.],
        ],
        vec![[0, 1, 2], [0, 3, 1], [0, 1, 4]],
    ));
    assert_eq!(fin.non_manifold_edges(), 1);

    let flat = MeshGraph::build(&mesh(
        vec![[0., 0., 0.], [1., 0., 0.], [2., 0., 0.], [0., 1., 0.]],
        vec![[0, 1, 2], [0, 1, 3]],
    ));
    assert_eq!(
        flat.face_normals[0], [0.0; 3],
        "a zero-area triangle has no normal"
    );
    assert!(
        flat.crease_edges(1f64.to_radians()).is_empty(),
        "zero-area triangles make no creases"
    );
}
