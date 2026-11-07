// SPDX-License-Identifier: MIT OR Apache-2.0
//! 3-vector helpers on plain arrays.

/// A 3-vector.
pub type Vec3 = [f64; 3];

/// Dot product.
pub fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Cross product.
pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// a + b.
pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// a - b.
pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// s * a.
pub fn scale(a: Vec3, s: f64) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// Euclidean length, without overflow or underflow for extreme components.
pub fn length(a: Vec3) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}

/// `a` scaled to unit length, or `None` when its length is zero or not finite.
pub fn normalize(a: Vec3) -> Option<Vec3> {
    let l = length(a);
    (l > 0.0 && l.is_finite()).then(|| scale(a, 1.0 / l))
}

/// Unsigned angle between two directions, radians in [0, π]; `None` if either is zero.
///
/// Uses atan2(|a×b|, a·b), which stays accurate near 0 and π where acos of a dot product does not.
pub fn angle_between(a: Vec3, b: Vec3) -> Option<f64> {
    normalize(a)?;
    normalize(b)?;
    Some(length(cross(a, b)).atan2(dot(a, b)))
}

/// Signed angle from `a` to `b` around `axis`, radians in (-π, π]; `None` if `a`, `b` or `axis` is zero.
pub fn signed_angle(a: Vec3, b: Vec3, axis: Vec3) -> Option<f64> {
    let n = normalize(axis)?;
    normalize(a)?;
    normalize(b)?;
    let angle = dot(cross(a, b), n).atan2(dot(a, b));
    // atan2 gives [-π, π]; report the half-turn as +π so the range is (-π, π].
    Some(if angle == -std::f64::consts::PI {
        std::f64::consts::PI
    } else {
        angle
    })
}
