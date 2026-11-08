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
