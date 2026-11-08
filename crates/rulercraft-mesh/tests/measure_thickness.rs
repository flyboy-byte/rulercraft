// SPDX-License-Identifier: MIT OR Apache-2.0
//! Directional wall measurements on our 2 x 4 x 6 mm box design.
#![allow(clippy::unwrap_used)]
use rulercraft_mesh::graph::MeshGraph;
use rulercraft_mesh::measure::thickness::{ThicknessOptions, wall_thickness};
use rulercraft_mesh::{Mesh, MeshBuilder, Unit};
fn box_design() -> (Mesh, Vec<u32>, Vec<f64>) {
    let mut b = MeshBuilder::new(Unit::Mm);
    let mut centers = Vec::new();
    let mut expected = Vec::new();
    let corner = |i: u32| {
        [
            (i & 1) as f32 * 2.,
            ((i >> 1) & 1) as f32 * 4.,
            ((i >> 2) & 1) as f32 * 6.,
        ]
    };
    for (q, d) in [
        ([0, 2, 3, 1], 6.),
        ([4, 5, 7, 6], 6.),
        ([0, 1, 5, 4], 4.),
        ([2, 6, 7, 3], 4.),
        ([0, 4, 6, 2], 2.),
        ([1, 3, 7, 5], 2.),
    ] {
        let center = [0, 1, 2].map(|k| q.iter().map(|i| corner(*i)[k]).sum::<f32>() / 4.);
        let c = b.vertex(center).unwrap();
        centers.push(c);
        expected.push(d);
        for j in 0..4 {
            let a = b.vertex(corner(q[j])).unwrap();
            let z = b.vertex(corner(q[(j + 1) % 4])).unwrap();
            b.triangle([c, a, z]);
        }
    }
    (b.finish(), centers, expected)
}
#[test]
fn opposite_faces_recover_each_design_dimension() {
    let (m, sel, expected) = box_design();
    let g = MeshGraph::build(&m);
    let samples = wall_thickness(&m, &g.vertex_normals, &sel, Default::default()).unwrap();
    for ((s, id), d) in samples.iter().zip(&sel).zip(expected) {
        assert_eq!(s.vertex, *id);
        assert!((s.distance.unwrap() - d).abs() < 1e-10);
    } // integer design and axis-aligned rays; f64 roundoff only
    let mut shifted = m.clone();
    for p in &mut shifted.positions {
        *p = [p[2] * 10. + 100., p[0] * 10. + 100., p[1] * 10. + 100.];
    }
    let rotated_normals: Vec<_> = g
        .vertex_normals
        .iter()
        .map(|n| [n[2], n[0], n[1]])
        .collect();
    let moved = wall_thickness(&shifted, &rotated_normals, &sel, Default::default()).unwrap();
    for (a, b) in samples.iter().zip(moved) {
        assert!((a.distance.unwrap() * 10. - b.distance.unwrap()).abs() < 1e-10);
    }
}
#[test]
fn missing_hits_limits_and_invalid_inputs() {
    let (m, sel, _) = box_design();
    let g = MeshGraph::build(&m);
    let short = ThicknessOptions {
        max_distance: 1.,
        ..Default::default()
    };
    assert!(
        wall_thickness(&m, &g.vertex_normals, &sel, short)
            .unwrap()
            .iter()
            .all(|s| s.distance.is_none())
    );
    let reversed: Vec<_> = g.vertex_normals.iter().map(|n| n.map(|v| -v)).collect();
    assert!(
        wall_thickness(&m, &reversed, &sel, Default::default())
            .unwrap()
            .iter()
            .all(|s| s.distance.is_none())
    );
    assert!(
        wall_thickness(
            &m,
            &g.vertex_normals,
            &sel,
            ThicknessOptions {
                max_triangle_tests: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(wall_thickness(&m, &[], &sel, Default::default()).is_err());
    assert!(wall_thickness(&m, &g.vertex_normals, &[999], Default::default()).is_err());
    assert!(
        wall_thickness(
            &m,
            &g.vertex_normals,
            &sel,
            ThicknessOptions {
                exclusion: f64::NAN,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut bad = m.clone();
    bad.triangles[0][0] = 999;
    assert!(wall_thickness(&bad, &g.vertex_normals, &sel, Default::default()).is_err());
    assert!(
        wall_thickness(&m, &g.vertex_normals, &[], Default::default())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn nearest_wall_wins_over_a_further_surface() {
    let (mut m, sel, _) = box_design();
    let mut normals = MeshGraph::build(&m).vertex_normals;
    let start = m.positions.len() as u32;
    m.positions
        .extend([[-100., -100., 10.], [100., -100., 10.], [0., 100., 10.]]);
    m.triangles.push([start, start + 1, start + 2]);
    normals.extend([[0., 0., 1.]; 3]);
    let samples = wall_thickness(&m, &normals, &sel[..1], Default::default()).unwrap();
    assert!((samples[0].distance.unwrap() - 6.).abs() < 1e-10); // box top z=6 before extra plane z=10
}

#[test]
fn a_ray_just_past_the_opposite_walls_edge_still_counts() {
    // The start vertex sits 5e-12 mm outside the far triangle's edge x = 0, inside the hit test's edge
    // allowance. The tree's boxes must be widened so it is still tested.
    let m = Mesh {
        positions: vec![[0., 0., 0.], [0., 10., 0.], [10., 0., 0.], [-5e-12, 2., 5.]],
        triangles: vec![[0, 1, 2]],
        degenerate_dropped: 0,
    };
    let normals = [[0., 0., 1.]; 4];
    let s = wall_thickness(&m, &normals, &[3], Default::default()).unwrap();
    assert_eq!(s[0].distance, Some(5.0));
}

#[test]
fn a_reused_tree_gives_the_same_samples_and_a_foreign_one_is_refused() {
    use rulercraft_mesh::measure::thickness::wall_thickness_indexed;
    use rulercraft_mesh::spatial::Bvh;
    let m = Mesh {
        positions: vec![[0., 0., 0.], [0., 10., 0.], [10., 0., 0.], [2., 2., 5.]],
        triangles: vec![[0, 1, 2]],
        degenerate_dropped: 0,
    };
    let normals = [[0., 0., 1.]; 4];
    let tree = Bvh::new(&m);
    let fresh = wall_thickness(&m, &normals, &[3], Default::default()).unwrap();
    let reused =
        wall_thickness_indexed(&m, Some(&tree), &normals, &[3], Default::default()).unwrap();
    assert_eq!(fresh, reused);
    assert_eq!(reused[0].distance, Some(5.0));
    let other = Mesh {
        triangles: vec![[0, 1, 2], [0, 2, 1]],
        ..m.clone()
    };
    assert!(
        wall_thickness_indexed(
            &m,
            Some(&Bvh::new(&other)),
            &normals,
            &[3],
            Default::default()
        )
        .is_err()
    );
}
