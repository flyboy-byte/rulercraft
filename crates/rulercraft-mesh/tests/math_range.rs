// SPDX-License-Identifier: MIT OR Apache-2.0
//! Public vector helpers remain useful for finite directions across the f64 range.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]
use rulercraft_mesh::math::{angle_between, normalize, signed_angle};
use std::f64::consts::PI;

fn close(a: f64, b: f64, tolerance: f64) {
    assert!((a - b).abs() <= tolerance, "{a} differs from {b}");
}

#[test]
fn angle_between_preserves_a_known_angle_across_scales() {
    let expected = PI / 6.0;
    for scale in [1.0, 1e200, 1e-200] {
        let a = [scale, 0.0, 0.0];
        let b = [scale * (3.0_f64.sqrt() / 2.0), scale * 0.5, 0.0];
        close(angle_between(a, b).unwrap(), expected, 2e-15);
    }
    let tiny = 1e-320;
    close(
        angle_between([tiny, 0.0, 0.0], [0.0, tiny, 0.0]).unwrap(),
        PI / 2.0,
        2e-15,
    );
}

#[test]
fn normalization_handles_large_and_subnormal_nonzero_directions() {
    for v in [[1e308, 1e308, 0.0], [f64::from_bits(1), 0.0, 0.0]] {
        let n = normalize(v).expect("finite nonzero direction should normalize");
        assert!(n.iter().all(|x| x.is_finite()));
        close(n[0].hypot(n[1]).hypot(n[2]), 1.0, 2e-15);
    }
    assert!(normalize([0.0; 3]).is_none());
    assert!(normalize([f64::INFINITY, 0.0, 0.0]).is_none());
    assert!(normalize([f64::NAN, 0.0, 0.0]).is_none());
}

#[test]
fn signed_angle_uses_direction_only_for_extreme_finite_vectors() {
    let angle = signed_angle([1e200, 0.0, 0.0], [0.0, 1e200, 0.0], [0.0, 0.0, 1e200]).unwrap();
    close(angle, PI / 2.0, 2e-15);
}
