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
