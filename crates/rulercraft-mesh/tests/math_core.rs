// SPDX-License-Identifier: MIT OR Apache-2.0
//! math.core checks: inputs whose answers are known exactly (docs/specs/math.md).
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::math::*;
use std::f64::consts::PI;

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn rotation(axis: Vec3, angle: f64) -> [[f64; 3]; 3] {
    let [x, y, z] = normalize(axis).unwrap();
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    [
        [t * x * x + c, t * x * y - s * z, t * x * z + s * y],
        [t * x * y + s * z, t * y * y + c, t * y * z - s * x],
        [t * x * z - s * y, t * y * z + s * x, t * z * z + c],
    ]
}

fn mat_mul(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut c = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            c[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    c
}

#[test]
fn eigen_of_a_diagonal_matrix_sorts_values_with_their_axes() {
    let e = symmetric_eigen3([[3.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 2.0]]).unwrap();
    assert_eq!(e.values, [1.0, 2.0, 3.0]);
    assert_eq!(
        e.vectors,
        [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]]
    );
}

#[test]
fn eigen_recovers_a_rotated_diagonal_to_1e_12() {
    for (axis, angle) in [
        ([1.0, 2.0, 3.0], 0.7),
        ([-0.3, 0.1, 0.9], 2.9),
        ([0.0, 0.0, 1.0], 1e-7),
    ] {
        let r = rotation(axis, angle);
        let d = [[5.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 0.5]];
        let rt = [
            [r[0][0], r[1][0], r[2][0]],
            [r[0][1], r[1][1], r[2][1]],
            [r[0][2], r[1][2], r[2][2]],
        ];
        let m = mat_mul(mat_mul(r, d), rt);
        let e = symmetric_eigen3(m).unwrap();
        for (got, want) in e.values.iter().zip([0.5, 2.0, 5.0]) {
            assert!(close(*got, want, 1e-12), "{:?}", e.values);
        }
        // Column k of r is the eigenvector of d[k][k]; signs may differ.
        for (slot, col) in [(0, 2), (1, 1), (2, 0)] {
            let want = [r[0][col], r[1][col], r[2][col]];
            assert!(close(dot(e.vectors[slot], want).abs(), 1.0, 1e-12));
        }
        for i in 0..3 {
            assert!(close(length(e.vectors[i]), 1.0, 1e-14));
            for j in i + 1..3 {
                assert!(dot(e.vectors[i], e.vectors[j]).abs() < 1e-13);
            }
        }
    }
}

#[test]
fn eigen_handles_repeated_values_zero_and_bad_input() {
    let e = symmetric_eigen3([[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 2.0]]).unwrap();
    assert_eq!(e.values, [2.0; 3]);
    let z = symmetric_eigen3([[0.0; 3]; 3]).unwrap();
    assert_eq!(z.values, [0.0; 3]);
    assert!(symmetric_eigen3([[f64::NAN, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]).is_none());
}

#[test]
fn covariance_far_from_the_origin_keeps_the_plane_normal() {
    // A tilted grid on the plane through (1e6, 1e6, 1e6) with normal (1, 1, 1)/√3.
    let n = normalize([1.0, 1.0, 1.0]).unwrap();
    let u = normalize(cross(n, [1.0, 0.0, 0.0])).unwrap();
    let v = cross(n, u);
    let mut points = vec![];
    for i in -10..=10 {
        for j in -10..=10 {
            points.push(add(
                [1e6; 3],
                add(scale(u, f64::from(i)), scale(v, f64::from(j))),
            ));
        }
    }
    let c = covariance(points.iter().copied()).unwrap();
    assert_eq!(c.count, 441);
    for k in 0..3 {
        assert!(close(c.centroid[k], 1e6, 1e-6));
    }
    let e = symmetric_eigen3(c.matrix).unwrap();
    assert!(close(dot(e.vectors[0], n).abs(), 1.0, 1e-9));
    assert!(e.values[0].abs() < 1e-9);
    assert!(covariance(std::iter::empty()).is_none());
}

#[test]
fn angles_stay_accurate_near_zero_and_half_turns() {
    let tiny = angle_between([1.0, 0.0, 0.0], [1.0, 1e-9, 0.0]).unwrap();
    assert!(close(tiny, 1e-9, 1e-20));
    let opposite = angle_between([1.0, 0.0, 0.0], [-1.0, 1e-9, 0.0]).unwrap();
    assert!(close(opposite, PI - 1e-9, 1e-15));
    let z = [0.0, 0.0, 1.0];
    assert!(close(
        signed_angle([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], z).unwrap(),
        PI / 2.0,
        1e-15
    ));
    assert!(close(
        signed_angle([1.0, 0.0, 0.0], [0.0, -1.0, 0.0], z).unwrap(),
        -PI / 2.0,
        1e-15
    ));
    assert_eq!(
        signed_angle([1.0, 0.0, 0.0], [-1.0, -0.0, 0.0], z),
        Some(PI)
    );
    assert!(normalize([0.0; 3]).is_none());
    assert!(angle_between([0.0; 3], z).is_none());
    assert!(close(length([3e200, 4e200, 0.0]), 5e200, 1e188));
}

#[test]
fn levenberg_marquardt_fits_a_circle_from_a_bad_start() {
    let (cx, cy, r) = (3.0, -2.0, 7.5);
    let pts: Vec<[f64; 2]> = (0..40)
        .map(|i| {
            let t = f64::from(i) * 0.15;
            [cx + r * t.cos(), cy + r * t.sin()]
        })
        .collect();
    let report = levenberg_marquardt(
        &[0.0, 0.0, 1.0],
        pts.len(),
        LmOptions::default(),
        |x, res, mut jac| {
            for (i, p) in pts.iter().enumerate() {
                let (dx, dy) = (p[0] - x[0], p[1] - x[1]);
                let d = dx.hypot(dy);
                if d == 0.0 {
                    return false;
                }
                res[i] = d - x[2];
                if let Some(j) = jac.as_deref_mut() {
                    j[i * 3] = -dx / d;
                    j[i * 3 + 1] = -dy / d;
                    j[i * 3 + 2] = -1.0;
                }
            }
            true
        },
    )
    .unwrap();
    assert!(report.converged);
    for (got, want) in report.x.iter().zip([cx, cy, r]) {
        assert!(close(*got, want, 1e-10), "{:?}", report);
    }
    assert!(report.cost < 1e-20);
}

#[test]
fn levenberg_marquardt_reaches_the_rosenbrock_minimum() {
    let report = levenberg_marquardt(&[-1.2, 1.0], 2, LmOptions::default(), |x, res, jac| {
        res[0] = 10.0 * (x[1] - x[0] * x[0]);
        res[1] = 1.0 - x[0];
        if let Some(j) = jac {
            j.copy_from_slice(&[-20.0 * x[0], 10.0, -1.0, 0.0]);
        }
        true
    })
    .unwrap();
    assert!(
        close(report.x[0], 1.0, 1e-8) && close(report.x[1], 1.0, 1e-8),
        "{report:?}"
    );
    assert!(levenberg_marquardt(&[0.0, 0.0], 1, LmOptions::default(), |_, _, _| true).is_none());
    assert!(solve_dense(vec![1.0, 2.0, 2.0, 4.0], vec![1.0, 2.0]).is_none());
}
