// SPDX-License-Identifier: MIT OR Apache-2.0
//! Known-answer robust sphere fits with increasing outlier rates and part scales.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::fit::{RobustOptions, fit_sphere_robust};
use std::f64::consts::PI;

#[test]
fn robust_sphere_recovers_dimensions_at_multiple_scales_and_outlier_rates() {
    let good = 180_usize;
    for radius in [1.0_f64, 10.0, 100.0] {
        let center = [2.0 * radius, -3.0 * radius, radius];
        for extra in [0_usize, 45, 90] {
            for seed in [3_u64, 11] {
                let mut state = seed;
                let mut random = || {
                    state = state
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407);
                    (state >> 11) as f64 / (1_u64 << 53) as f64
                };
                let mut points = Vec::new();
                for i in 0..good {
                    let z = 1.0 - 2.0 * (i as f64 + 0.5) / good as f64;
                    let phi = i as f64 * PI * (3.0 - 5.0_f64.sqrt());
                    let xy = (1.0 - z * z).sqrt();
                    let r = radius * (1.0 + 0.0001 * (2.0 * random() - 1.0));
                    points.push(
                        [
                            center[0] + r * xy * phi.cos(),
                            center[1] + r * xy * phi.sin(),
                            center[2] + r * z,
                        ]
                        .map(|x| x as f32),
                    );
                }
                for _ in 0..extra {
                    // Far outside the unit sphere, in a box on one side of the part.
                    points.push(
                        [
                            center[0] + radius * (3.0 + random()),
                            center[1] + radius * (3.0 + random()),
                            center[2] + radius * (3.0 + random()),
                        ]
                        .map(|x| x as f32),
                    );
                }
                let ids: Vec<_> = (0..points.len() as u32).collect();
                let fit = fit_sphere_robust(
                    &points,
                    &ids,
                    RobustOptions {
                        threshold: radius * 0.005,
                        iterations: 100,
                        seed,
                    },
                )
                .unwrap();
                // A 0.2% allowance is fixed ahead of the run for the 0.01% radial noise and f32 positions.
                let tolerance = radius * 0.002;
                assert!(
                    (fit.fit.radius - radius).abs() < tolerance,
                    "radius={radius} extra={extra} seed={seed}: {fit:?}"
                );
                for (actual, expected) in fit.fit.center.into_iter().zip(center) {
                    assert!(
                        (actual - expected).abs() < tolerance,
                        "radius={radius} extra={extra} seed={seed}: {fit:?}"
                    );
                }
                assert_eq!(
                    fit.inliers.len(),
                    good,
                    "radius={radius} extra={extra} seed={seed}: {fit:?}"
                );
                assert!(fit.inliers.iter().all(|&i| (i as usize) < good));
            }
        }
    }
}
