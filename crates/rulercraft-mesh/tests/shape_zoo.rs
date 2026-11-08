// SPDX-License-Identifier: MIT OR Apache-2.0
//! Analytic, deterministic shape-zoo samples and their known dimensions.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::fit::{
    Clip,
    cylinder::{CylinderOptions, fit_cylinder},
    fit_sphere,
};
use std::f64::consts::{PI, TAU};

#[test]
fn known_spheres_at_multiple_sizes_and_densities() {
    for radius in [0.1_f64, 1.0, 100.0] {
        for count in [64_usize, 257] {
            let center = [2.0 * radius, -3.0 * radius, radius];
            let points: Vec<[f32; 3]> = (0..count)
                .map(|i| {
                    let z = 1.0 - 2.0 * (i as f64 + 0.5) / count as f64;
                    let phi = i as f64 * PI * (3.0 - 5.0_f64.sqrt());
                    let radial = (1.0 - z * z).sqrt();
                    [
                        center[0] + radius * radial * phi.cos(),
                        center[1] + radius * radial * phi.sin(),
                        center[2] + radius * z,
                    ]
                    .map(|x| x as f32)
                })
                .collect();
            let ids: Vec<_> = (0..count as u32).collect();
            let result = fit_sphere(&points, &ids, Clip::None, None).unwrap();
            // Input coordinates are f32, so the allowance scales with the part.
            let tolerance = radius * 2e-5;
            assert!(
                (result.radius - radius).abs() < tolerance,
                "r={radius} n={count}: {result:?}"
            );
            for (actual, expected) in result.center.into_iter().zip(center) {
                assert!(
                    (actual - expected).abs() < tolerance,
                    "r={radius} n={count}: {result:?}"
                );
            }
        }
    }
}

#[test]
fn known_cylinders_at_multiple_sizes_and_densities() {
    for radius in [0.1_f64, 1.0, 100.0] {
        for count in [16_usize, 48] {
            let center = [2.0 * radius, -3.0 * radius, radius];
            let mut points = Vec::new();
            let mut normals = Vec::new();
            for height in [-2.0, 0.0, 2.0] {
                for i in 0..count {
                    let angle = TAU * i as f64 / count as f64;
                    let normal = [angle.cos(), angle.sin(), 0.0];
                    points.push(
                        [
                            center[0] + radius * normal[0],
                            center[1] + radius * normal[1],
                            center[2] + height * radius,
                        ]
                        .map(|x| x as f32),
                    );
                    normals.push(normal.map(|x| x as f32));
                }
            }
            let ids: Vec<_> = (0..points.len() as u32).collect();
            let result =
                fit_cylinder(&points, Some(&normals), &ids, CylinderOptions::default()).unwrap();
            let tolerance = radius * 2e-5; // f32 coordinates and fit rounding.
            assert!(
                (result.radius - radius).abs() < tolerance,
                "r={radius} n={count}: {result:?}"
            );
            assert!(
                (result.length - 4.0 * radius).abs() < tolerance,
                "r={radius} n={count}: {result:?}"
            );
            assert!(result.axis[2].abs() > 1.0 - 1e-6, "{result:?}");
        }
    }
}

#[test]
fn partial_surfaces_with_bounded_radial_noise() {
    // The noise is bounded by 0.1% of the radius; a 1% error allowance is
    // fixed in advance for fitting incomplete surfaces from quantized f32 points.
    for radius in [1.0_f64, 10.0, 100.0] {
        for seed in [1_u64, 7, 25] {
            let mut state = seed;
            let mut noise = || {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                2.0 * ((state >> 11) as f64 / (1_u64 << 53) as f64) - 1.0
            };
            let center = [2.0 * radius, -3.0 * radius, radius];
            let count = 256;
            let mut sphere = Vec::new();
            for i in 0..count {
                // Uniform z on the cap within 60 degrees of +z.
                let z = 1.0 - 0.5 * (i as f64 + 0.5) / count as f64;
                let phi = i as f64 * PI * (3.0 - 5.0_f64.sqrt());
                let radial = (1.0 - z * z).sqrt();
                let r = radius * (1.0 + 0.001 * noise());
                sphere.push(
                    [
                        center[0] + r * radial * phi.cos(),
                        center[1] + r * radial * phi.sin(),
                        center[2] + r * z,
                    ]
                    .map(|x| x as f32),
                );
            }
            let ids: Vec<_> = (0..count as u32).collect();
            let fit = fit_sphere(&sphere, &ids, Clip::None, None).unwrap();
            assert!(
                (fit.radius - radius).abs() < radius * 0.01,
                "r={radius} seed={seed}: {fit:?}"
            );
            for (actual, expected) in fit.center.into_iter().zip(center) {
                assert!(
                    (actual - expected).abs() < radius * 0.01,
                    "r={radius} seed={seed}: {fit:?}"
                );
            }
            let mut tube = Vec::new();
            let mut normals = Vec::new();
            for height in [-2.0, 0.0, 2.0] {
                for i in 0..64 {
                    let angle = -PI / 3.0 + (2.0 * PI / 3.0) * i as f64 / 63.0;
                    let normal = [angle.cos(), angle.sin(), 0.0];
                    let r = radius * (1.0 + 0.001 * noise());
                    tube.push(
                        [
                            center[0] + r * normal[0],
                            center[1] + r * normal[1],
                            center[2] + height * radius,
                        ]
                        .map(|x| x as f32),
                    );
                    normals.push(normal.map(|x| x as f32));
                }
            }
            let ids: Vec<_> = (0..tube.len() as u32).collect();
            let fit =
                fit_cylinder(&tube, Some(&normals), &ids, CylinderOptions::default()).unwrap();
            assert!(
                (fit.radius - radius).abs() < radius * 0.01,
                "r={radius} seed={seed}: {fit:?}"
            );
            assert!(
                fit.axis[2].abs() > 0.9999,
                "r={radius} seed={seed}: {fit:?}"
            );
        }
    }
}
