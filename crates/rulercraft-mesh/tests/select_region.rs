// SPDX-License-Identifier: MIT OR Apache-2.0
//! select.region checks: selections whose members are known exactly (docs/specs/selection.md).
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::fit::{Clip, fit_plane};
use rulercraft_mesh::graph::MeshGraph;
use rulercraft_mesh::select::{SelectError, explicit, facing, in_box, near, thinned};
use rulercraft_mesh::{Mesh, Unit, read_mesh};

fn grid(n: i32) -> Vec<[f32; 3]> {
    (0..n * n)
        .map(|i| [(i % n) as f32, (i / n) as f32, 0.0])
        .collect()
}

#[test]
fn box_and_near_are_inclusive_and_exact() {
    let p = grid(10);
    let b = in_box(&p, [2.0, 3.0, -1.0], [4.0, 3.0, 1.0]).unwrap();
    assert_eq!(b, vec![32, 33, 34]);
    assert_eq!(in_box(&p, [-1.0; 3], [100.0; 3]).unwrap().len(), 100);
    assert!(in_box(&p, [50.0; 3], [60.0; 3]).unwrap().is_empty());
    let n = near(&p, [5.0, 5.0, 0.0], 1.0).unwrap();
    assert_eq!(n, vec![45, 54, 55, 56, 65]);
    assert_eq!(near(&p, [5.0, 5.0, 0.0], 0.0).unwrap(), vec![55]);
}

#[test]
fn facing_picks_one_face_of_a_cube_and_feeds_a_plane_fit() {
    // A cube with its corners split so each face has its own vertices and flat normals: built as an
    // OBJ with per-face vertices.
    let mut obj = String::new();
    let faces: [[[f32; 3]; 4]; 6] = [
        [[0., 0., 0.], [0., 1., 0.], [1., 1., 0.], [1., 0., 0.]],
        [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]],
        [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
        [[0., 1., 0.], [0., 1., 1.], [1., 1., 1.], [1., 1., 0.]],
        [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]],
        [[1., 0., 0.], [1., 1., 0.], [1., 1., 1.], [1., 0., 1.]],
    ];
    // Shrink each face slightly toward its centre so its vertices are distinct from the next face's.
    for (fi, f) in faces.iter().enumerate() {
        let c = [0, 1, 2].map(|k| f.iter().map(|v| v[k]).sum::<f32>() / 4.0);
        for v in f {
            let q = [0, 1, 2].map(|k| c[k] + (v[k] - c[k]) * 0.9);
            obj += &format!("v {} {} {}\n", q[0], q[1], q[2]);
        }
        let b = fi * 4 + 1;
        obj += &format!("f {} {} {} {}\n", b, b + 1, b + 2, b + 3);
    }
    let mesh: Mesh = read_mesh("c.obj", obj.as_bytes(), Unit::Mm, Default::default()).unwrap();
    let g = MeshGraph::build(&mesh);
    let all: Vec<u32> = (0..mesh.positions.len() as u32).collect();
    let up = facing(&all, &g.vertex_normals, [0.0, 0.0, 5.0], 30f64.to_radians()).unwrap();
    assert_eq!(up.len(), 4);
    assert!(up.iter().all(|&i| mesh.positions[i as usize][2] == 1.0));
    let fit = fit_plane(
        &mesh.positions,
        Some(&g.vertex_normals),
        &up,
        Clip::Sigma(3),
    )
    .unwrap();
    assert_eq!(fit.normal, [0.0, 0.0, 1.0]);
    // Every face's normal is within 90°+ of +z except the bottom: 180° keeps everything.
    assert_eq!(
        facing(
            &all,
            &g.vertex_normals,
            [0.0, 0.0, 1.0],
            std::f64::consts::PI
        )
        .unwrap()
        .len(),
        24
    );
    assert_eq!(
        facing(&all, &g.vertex_normals, [0.0, 0.0, 1.0], 90f64.to_radians())
            .unwrap()
            .len(),
        20
    );
}

#[test]
fn explicit_lists_and_thinning() {
    assert_eq!(explicit(10, &[5, 2, 5, 9]).unwrap(), vec![2, 5, 9]);
    assert_eq!(
        explicit(10, &[3, 10]),
        Err(SelectError::NoSuchVertex {
            index: 10,
            count: 10
        })
    );
    let s: Vec<u32> = (0..1000).collect();
    let (few, step) = thinned(&s, 100);
    assert_eq!((few.len(), step, few[1]), (100, 10, 10));
    let (some, step) = thinned(&s, 300);
    assert_eq!((step, some.len()), (4, 250));
    assert_eq!(thinned(&s[..5], 100), (vec![0, 1, 2, 3, 4], 1));
    assert_eq!(thinned(&s, 0), (vec![], 0));
}

#[test]
fn refusals_say_what_is_wrong() {
    let p = grid(3);
    assert_eq!(
        in_box(&p, [0.0, 2.0, 0.0], [1.0, 1.0, 1.0]),
        Err(SelectError::InvertedBox(1))
    );
    assert_eq!(
        in_box(&p, [f64::NAN, 0.0, 0.0], [1.0; 3]),
        Err(SelectError::NotFinite)
    );
    assert_eq!(near(&p, [0.0; 3], -1.0), Err(SelectError::BadRadius));
    let n = vec![[0.0f32, 0.0, 1.0]; 9];
    assert_eq!(
        facing(&[0], &n, [0.0; 3], 1.0),
        Err(SelectError::ZeroDirection)
    );
    assert_eq!(
        facing(&[0], &n, [0.0, 0.0, 1.0], 0.0),
        Err(SelectError::BadAngle)
    );
    assert_eq!(
        facing(&[20], &n, [0.0, 0.0, 1.0], 1.0),
        Err(SelectError::NoSuchVertex {
            index: 20,
            count: 9
        })
    );
    assert!(SelectError::InvertedBox(2).to_string().ends_with("on z"));
}

#[test]
fn finite_extreme_queries_do_not_square_to_infinity_or_zero() {
    let points = [[0.0, 0.0, 0.0]];
    assert!(
        rulercraft_mesh::select::near(&points, [1e200, 0., 0.], 1e199)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        rulercraft_mesh::select::near(&points, [1e200, 0., 0.], 1e201).unwrap(),
        [0]
    );
    assert!(
        rulercraft_mesh::select::near(&points, [1e-200, 0., 0.], 0.)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        rulercraft_mesh::select::facing(&[0], &[[0., 0., 1.]], [0., 0., 1e-320], 0.1).unwrap(),
        [0]
    );
}
