// SPDX-License-Identifier: MIT OR Apache-2.0
//! compare.deviation checks: distances with known answers, and the tree against brute force.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::Mesh;
use rulercraft_mesh::deviation::{closest_point_on_triangle, deviation};
use rulercraft_mesh::math::{length, sub};

fn mesh(positions: &[[f32; 3]], triangles: &[[u32; 3]]) -> Mesh {
    Mesh {
        positions: positions.to_vec(),
        triangles: triangles.to_vec(),
        degenerate_dropped: 0,
    }
}
/// Cube from (0,0,0) to (10,10,10), outward normals.
fn cube() -> Mesh {
    let p = [
        [0., 0., 0.],
        [10., 0., 0.],
        [10., 10., 0.],
        [0., 10., 0.],
        [0., 0., 10.],
        [10., 0., 10.],
        [10., 10., 10.],
        [0., 10., 10.],
    ];
    let t = [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [3, 7, 6],
        [3, 6, 2],
        [0, 4, 7],
        [0, 7, 3],
        [1, 2, 6],
        [1, 6, 5],
    ];
    mesh(&p, &t)
}
fn probe(points: &[[f32; 3]]) -> Mesh {
    mesh(points, &[])
}

#[test]
fn distances_to_a_cube_are_known_and_signed() {
    let m = probe(&[
        [5., 5., 10.5],     // 0.5 above the top face
        [5., 5., 9.7],      // 0.3 inside it
        [10.5, 10.5, 10.5], // off a corner: sqrt(3) * 0.5
        [5., 12., 5.],      // 2 off a side face
    ]);
    let d = deviation(&m, &cube(), 0.4).unwrap();
    let want = [0.5, -0.3, 3f64.sqrt() * 0.5, 2.0];
    for (g, w) in d.distances.iter().zip(want) {
        assert!((g - w).abs() < 1e-5, "{:?}", d.distances);
    }
    let s = &d.stats;
    assert_eq!(s.count, 4);
    assert!((s.max_outside - 2.0).abs() < 1e-5 && (s.max_inside - 0.3).abs() < 1e-5);
    assert!((s.mean - (0.5 - 0.3 + 3f64.sqrt() * 0.5 + 2.0) / 4.0).abs() < 1e-5);
    assert_eq!(s.beyond_tolerance, 3); // 0.5, 0.866 and 2.0 exceed 0.4; 0.3 does not
    assert!((s.p50_abs - 0.5).abs() < 1e-5 && (s.p95_abs - 2.0).abs() < 1e-5);
}

#[test]
fn a_mesh_on_its_own_reference_has_zero_deviation() {
    let c = cube();
    let d = deviation(&c, &c, 0.01).unwrap();
    assert!(d.stats.rms < 1e-9 && d.stats.beyond_tolerance == 0);
}

#[test]
fn turning_and_moving_both_meshes_changes_nothing() {
    let (a, b) = (0.9f64, [30.0, -4.0, 7.0]);
    let tr = |p: [f32; 3]| {
        let p = p.map(f64::from);
        [
            (p[0] * a.cos() - p[1] * a.sin() + b[0]) as f32,
            (p[0] * a.sin() + p[1] * a.cos() + b[1]) as f32,
            (p[2] + b[2]) as f32,
        ]
    };
    let pts = [
        [5., 5., 10.5],
        [10.5, 10.5, 10.5],
        [5., 12., 5.],
        [-3., 2., 1.],
    ];
    let base = deviation(&probe(&pts), &cube(), 1.0).unwrap();
    let mut c = cube();
    c.positions = c.positions.iter().map(|&p| tr(p)).collect();
    let moved = deviation(&probe(&pts.map(tr)), &c, 1.0).unwrap();
    for (x, y) in base.distances.iter().zip(&moved.distances) {
        assert!((x - y).abs() < 1e-4, "{x} vs {y}");
    }
}

fn lcg(s: &mut u64) -> f32 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*s >> 33) as f32 / (1u64 << 31) as f32) * 20.0 - 10.0
}

#[test]
fn the_tree_matches_brute_force_on_a_random_triangle_soup() {
    let mut s = 7u64;
    let positions: Vec<[f32; 3]> = (0..3 * 300)
        .map(|_| [lcg(&mut s), lcg(&mut s), lcg(&mut s)])
        .collect();
    let triangles: Vec<[u32; 3]> = (0..300).map(|i| [3 * i, 3 * i + 1, 3 * i + 2]).collect();
    let soup = mesh(&positions, &triangles);
    let pts: Vec<[f32; 3]> = (0..200)
        .map(|_| [lcg(&mut s) * 2.0, lcg(&mut s) * 2.0, lcg(&mut s) * 2.0])
        .collect();
    let d = deviation(&probe(&pts), &soup, 1.0).unwrap();
    for (p, got) in pts.iter().zip(&d.distances) {
        let p = p.map(f64::from);
        let best = triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| positions[i as usize].map(f64::from));
                length(sub(p, closest_point_on_triangle(p, a, b, c)))
            })
            .fold(f64::MAX, f64::min);
        assert!((got.abs() - best).abs() < 1e-9, "{got} vs {best}");
    }
}

#[test]
fn bad_input_is_refused_plainly() {
    assert!(deviation(&cube(), &Mesh::default(), 1.0).is_err());
    assert!(deviation(&cube(), &cube(), f64::NAN).is_err());
    assert!(deviation(&cube(), &cube(), -1.0).is_err());
    let none = deviation(&Mesh::default(), &cube(), 1.0).unwrap();
    assert_eq!(none.stats.count, 0);
    assert_eq!(none.stats.rms, 0.0);
}

/// A 10 x 10 vertex grid of unit squares in the x-y plane.
fn grid() -> Mesh {
    let mut p = vec![];
    for j in 0..10 {
        for i in 0..10 {
            p.push([i as f32, j as f32, 0.0]);
        }
    }
    let mut t = vec![];
    for j in 0..9u32 {
        for i in 0..9u32 {
            let a = j * 10 + i;
            t.push([a, a + 1, a + 11]);
            t.push([a, a + 11, a + 10]);
        }
    }
    mesh(&p, &t)
}

#[test]
fn bumps_become_separate_patches_by_side_and_size() {
    use rulercraft_mesh::deviation::patches;
    use rulercraft_mesh::graph::MeshGraph;
    let g = grid();
    let mut d = vec![0.0; 100];
    for v in [11, 12, 21, 22] {
        d[v] = 0.5;
    } // 4 vertices out
    for v in [77, 78] {
        d[v] = -0.4;
    } // 2 vertices in
    d[23] = -0.6; // touches the outside bump but is on the other side
    d[90] = 0.05; // under tolerance
    let p = patches(&MeshGraph::build(&g), &d, 0.1).unwrap();
    assert_eq!(p.len(), 3, "{p:?}");
    assert_eq!(p[0].vertices, vec![11, 12, 21, 22]);
    assert!(p[0].outside && (p[0].mean - 0.5).abs() < 1e-12 && (p[0].max_abs - 0.5).abs() < 1e-12);
    assert_eq!(p[1].vertices, vec![77, 78]);
    assert!(!p[1].outside && (p[1].mean + 0.4).abs() < 1e-12);
    assert_eq!(p[2].vertices, vec![23]);
    assert!(!p[2].outside && (p[2].max_abs - 0.6).abs() < 1e-12);
}

#[test]
fn patch_inputs_are_checked() {
    use rulercraft_mesh::deviation::patches;
    use rulercraft_mesh::graph::MeshGraph;
    let g = MeshGraph::build(&grid());
    assert!(patches(&g, &[0.0; 100], f64::NAN).is_err());
    assert!(patches(&g, &[0.0; 100], -1.0).is_err());
    assert!(patches(&g, &[0.0; 100], 0.1).unwrap().is_empty());
    let mut d = vec![0.0; 100];
    d[5] = f64::NAN;
    assert!(patches(&g, &d, 0.1).unwrap().is_empty());
    // Shorter distances than vertices must not panic.
    assert!(patches(&g, &[1.0; 10], 0.1).is_ok());
}
