// SPDX-License-Identifier: MIT OR Apache-2.0
//! fit.robust checks: a plane that far outliers drag off when fitted the ordinary way.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::fit::{Clip, RobustOptions, fit_plane, fit_plane_robust};

fn lcg(s: &mut u64) -> f32 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*s >> 33) as f32 / (1u64 << 31) as f32) * 2.0 - 1.0
}
/// 300 points on z = 5 (x, y in 0..40, noise 0.01) and 100 far outliers, one-sided and heavy.
fn cloud() -> Vec<[f32; 3]> {
    let mut s = 3u64;
    let mut p: Vec<[f32; 3]> = (0..300)
        .map(|_| {
            [
                20.0 + 20.0 * lcg(&mut s),
                20.0 + 20.0 * lcg(&mut s),
                5.0 + 0.01 * lcg(&mut s),
            ]
        })
        .collect();
    p.extend((0..100).map(|_| {
        [
            20.0 + 20.0 * lcg(&mut s),
            20.0 + 20.0 * lcg(&mut s),
            25.0 + 30.0 * lcg(&mut s).abs(),
        ]
    }));
    p
}
fn all(p: &[[f32; 3]]) -> Vec<u32> {
    (0..p.len() as u32).collect()
}

#[test]
fn far_outliers_tilt_the_ordinary_fit_but_not_the_robust_one() {
    let p = cloud();
    let sel = all(&p);
    let plain = fit_plane(&p, None, &sel, Clip::Sigma(3)).unwrap();
    assert!(
        plain.normal[2].abs() < 0.9999,
        "the ordinary fit should be pulled off: {:?}",
        plain.normal
    );
    let r = fit_plane_robust(&p, None, &sel, RobustOptions::default()).unwrap();
    assert!(
        (r.fit.normal[2].abs() - 1.0).abs() < 1e-4,
        "{:?}",
        r.fit.normal
    );
    assert!((r.fit.center[2] - 5.0).abs() < 0.01);
    assert!(r.fit.sigma < 0.02);
    assert!(
        (290..=300).contains(&r.inliers.len()),
        "{}",
        r.inliers.len()
    );
    assert!(r.inliers.iter().all(|&i| i < 300), "an outlier was kept");
}

#[test]
fn same_input_same_answer_and_turning_the_cloud_turns_the_plane() {
    let p = cloud();
    let sel = all(&p);
    let o = RobustOptions::default();
    assert_eq!(
        fit_plane_robust(&p, None, &sel, o).unwrap(),
        fit_plane_robust(&p, None, &sel, o).unwrap()
    );
    // Swap x and z: the plane z = 5 becomes x = 5.
    let q: Vec<[f32; 3]> = p.iter().map(|v| [v[2], v[1], v[0]]).collect();
    let r = fit_plane_robust(&q, None, &sel, o).unwrap();
    assert!((r.fit.normal[0].abs() - 1.0).abs() < 1e-4 && (r.fit.center[0] - 5.0).abs() < 0.01);
}

#[test]
fn bad_input_is_refused_plainly() {
    let p = cloud();
    let sel = all(&p);
    let o = RobustOptions::default();
    for bad in [
        RobustOptions {
            threshold: 0.0,
            ..o
        },
        RobustOptions {
            threshold: f64::NAN,
            ..o
        },
        RobustOptions { iterations: 0, ..o },
        RobustOptions {
            iterations: 100_001,
            ..o
        },
    ] {
        assert!(fit_plane_robust(&p, None, &sel, bad).is_err());
    }
    assert!(fit_plane_robust(&p, None, &[0, 1], o).is_err());
    assert!(fit_plane_robust(&p, None, &[0, 1, 9999], o).is_err());
    // All on one line: no plane.
    let line = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
    ];
    assert!(fit_plane_robust(&line, None, &[0, 1, 2, 3], o).is_err());
}

use rulercraft_mesh::fit::{Clip as C2, fit_cylinder_robust, fit_sphere, fit_sphere_robust};

fn sphere_cloud() -> Vec<[f32; 3]> {
    let mut s = 11u64;
    let mut p: Vec<[f32; 3]> = vec![];
    while p.len() < 200 {
        let v = [lcg(&mut s), lcg(&mut s), lcg(&mut s)];
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if !(0.3..=1.0).contains(&n) {
            continue;
        }
        // Upper cap only (z above 0.3): a patch, as a scan would give.
        if v[2] / n < 0.3 {
            continue;
        }
        let r = 10.0 + 0.01 * lcg(&mut s);
        p.push([5.0 + r * v[0] / n, -3.0 + r * v[1] / n, 2.0 + r * v[2] / n]);
    }
    p.extend((0..60).map(|_| {
        [
            5.0 + 30.0 * lcg(&mut s),
            -3.0 + 30.0 * lcg(&mut s),
            2.0 + 30.0 * lcg(&mut s),
        ]
    }));
    p
}

#[test]
fn a_robust_sphere_ignores_scattered_points() {
    let p = sphere_cloud();
    let sel = all(&p);
    let plain = fit_sphere(&p, &sel, C2::None, None).unwrap();
    assert!(
        (plain.radius - 10.0).abs() > 0.5,
        "the plain fit should be pulled off: {}",
        plain.radius
    );
    let r = fit_sphere_robust(&p, &sel, RobustOptions::default()).unwrap();
    assert!((r.fit.radius - 10.0).abs() < 0.02, "{}", r.fit.radius);
    assert!(
        (r.fit.center[0] - 5.0).abs() < 0.05
            && (r.fit.center[1] + 3.0).abs() < 0.05
            && (r.fit.center[2] - 2.0).abs() < 0.05,
        "{:?}",
        r.fit.center
    );
    assert!(
        r.inliers.iter().all(|&i| i < 200),
        "a scattered point was kept"
    );
    assert!(r.inliers.len() >= 190);
}

#[test]
fn a_robust_cylinder_ignores_scattered_points() {
    // Radius 6 about the line through (1, 2, 3) along (0, 0.6, 0.8); 250 surface points, 80 scattered.
    let (axis, u) = ([0.0f64, 0.6, 0.8], [1.0f64, 0.0, 0.0]);
    let w = [
        axis[1] * u[2] - axis[2] * u[1],
        axis[2] * u[0] - axis[0] * u[2],
        axis[0] * u[1] - axis[1] * u[0],
    ];
    let mut s = 5u64;
    let mut p: Vec<[f32; 3]> = (0..250)
        .map(|_| {
            let (a, h) = (
                std::f64::consts::PI * f64::from(lcg(&mut s)) * 0.9,
                15.0 * f64::from(lcg(&mut s)),
            );
            let r = 6.0 + 0.01 * f64::from(lcg(&mut s));
            [0, 1, 2].map(|k| {
                ([1.0, 2.0, 3.0][k] + h * axis[k] + r * (a.cos() * u[k] + a.sin() * w[k])) as f32
            })
        })
        .collect();
    p.extend((0..80).map(|_| {
        [
            1.0 + 25.0 * lcg(&mut s),
            2.0 + 25.0 * lcg(&mut s),
            3.0 + 25.0 * lcg(&mut s),
        ]
    }));
    let sel = all(&p);
    let o = RobustOptions {
        iterations: 60,
        threshold: 0.1,
        seed: 2,
    };
    let r = fit_cylinder_robust(&p, &sel, o).unwrap();
    assert!((r.fit.radius - 6.0).abs() < 0.03, "{}", r.fit.radius);
    assert!(
        r.fit.axis[0].abs() < 0.01
            && (r.fit.axis[1] - 0.6).abs() < 0.01
            && (r.fit.axis[2] - 0.8).abs() < 0.01,
        "{:?}",
        r.fit.axis
    );
    // A scattered point can land inside the 0.1 mm band by chance; a handful at most.
    let strays = r.inliers.iter().filter(|&&i| i >= 250).count();
    assert!(
        strays <= 3 && r.inliers.len() >= 240,
        "{strays} strays, {} kept",
        r.inliers.len()
    );
}

#[test]
fn round_robust_fits_refuse_bad_input() {
    let p = sphere_cloud();
    let sel = all(&p);
    let o = RobustOptions::default();
    assert!(
        fit_sphere_robust(
            &p,
            &sel,
            RobustOptions {
                threshold: -1.0,
                ..o
            }
        )
        .is_err()
    );
    assert!(fit_sphere_robust(&p, &sel[..5], o).is_err());
    assert!(fit_sphere_robust(&p, &[0, 1, 2, 3, 4, 99999], o).is_err());
    assert!(fit_cylinder_robust(&p, &sel[..11], o).is_err());
    assert!(fit_cylinder_robust(&p, &sel, RobustOptions { iterations: 0, ..o }).is_err());
}
