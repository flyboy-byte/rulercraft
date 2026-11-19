// SPDX-License-Identifier: MIT OR Apache-2.0
//! fit.shapes: cone and circle fits against surfaces known exactly (docs/specs/fit-shapes.md).
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::fit::{Clip, fit_circle, fit_cone};

type V3 = [f64; 3];

fn norm(a: V3) -> V3 {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    a.map(|c| c / l)
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
/// Two unit vectors across `a`.
fn across(a: V3) -> (V3, V3) {
    let u = norm(cross(
        a,
        if a[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        },
    ));
    (u, cross(a, u))
}
/// A fixed sequence in [-1, 1].
fn wobble(i: usize) -> f64 {
    let x = (i as f64 * 12.9898).sin() * 43_758.545_3;
    2.0 * (x - x.floor()) - 1.0
}

/// Points on a cone with apex `apex`, axis `axis`, half-angle `half` (degrees), from `h0` to `h1` along the axis, over
/// `arc` degrees, `noise` on each along the surface's normal.
fn cone_points(
    apex: V3,
    axis: V3,
    half: f64,
    h: (f64, f64),
    arc: f64,
    noise: f64,
    n: usize,
) -> Vec<[f32; 3]> {
    let d = norm(axis);
    let (u, v) = across(d);
    let t = half.to_radians().tan();
    let normal_tilt = half.to_radians();
    (0..n)
        .map(|i| {
            let a = arc.to_radians() * (i % 37) as f64 / 37.0;
            let hh = h.0 + (h.1 - h.0) * ((i / 37) as f64 + 0.5) / (n as f64 / 37.0).ceil();
            let r = hh * t;
            let radial = [0, 1, 2].map(|k| a.cos() * u[k] + a.sin() * v[k]);
            // The surface normal leans back along the axis by the half-angle.
            let nrm = [0, 1, 2].map(|k| radial[k] * normal_tilt.cos() - d[k] * normal_tilt.sin());
            let e = noise * wobble(i);
            [0, 1, 2].map(|k| (apex[k] + d[k] * hh + radial[k] * r + nrm[k] * e) as f32)
        })
        .collect()
}

fn all(n: usize) -> Vec<u32> {
    (0..n as u32).collect()
}

fn dist(a: V3, b: V3) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[test]
fn an_exact_tilted_cone_is_recovered() {
    let (apex, axis) = ([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]);
    let p = cone_points(apex, axis, 30.0, (2.0, 10.0), 360.0, 0.0, 37 * 20);
    let f = fit_cone(&p, None, &all(p.len()), Clip::None).unwrap();
    assert!((f.half_angle_deg - 30.0).abs() < 1e-6, "{f:?}");
    assert!(dist(f.apex, apex) < 1e-5, "{f:?}");
    let d = norm(axis);
    assert!(
        (f.axis[0] * d[0] + f.axis[1] * d[1] + f.axis[2] * d[2] - 1.0).abs() < 1e-9,
        "points to the wide end"
    );
    let t = 30f64.to_radians().tan();
    // The points span 2 to 10 from the apex, at the middle of each of 20 rows: 2.2 to 9.8.
    assert!(
        (f.diameter_small - 2.0 * 2.2 * t).abs() < 1e-5
            && (f.diameter_large - 2.0 * 9.8 * t).abs() < 1e-5
    );
    assert!(f.sigma < 1e-6);
}

#[test]
fn a_countersink_shaped_half_cone_with_noise_and_outliers() {
    // A 90° countersink (45° half-angle), only 180° of it, 0.01 mm noise, every 25th point 0.5 mm off.
    let mut p = cone_points(
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        45.0,
        (3.0, 6.0),
        180.0,
        0.01,
        37 * 16,
    );
    for (i, q) in p.iter_mut().enumerate() {
        if i % 25 == 0 {
            q[2] += 0.5;
        }
    }
    let f = fit_cone(&p, None, &all(p.len()), Clip::Sigma(3)).unwrap();
    assert!((f.half_angle_deg - 45.0).abs() < 0.2, "{f:?}");
    assert!(dist(f.apex, [0.0; 3]) < 0.05, "{f:?}");
    assert!(f.used < f.offered, "the outliers are dropped");
}

#[test]
fn a_cylinder_or_a_plane_is_not_a_cone() {
    // Radius 5, no taper.
    let p: Vec<[f32; 3]> = (0..200)
        .map(|i| {
            let a = i as f64 * 0.31;
            [5.0 * a.cos(), 5.0 * a.sin(), (i % 20) as f64].map(|c| c as f32)
        })
        .collect();
    let e = fit_cone(&p, None, &all(200), Clip::None).unwrap_err();
    assert!(
        e.to_string().contains("cylinder") || e.to_string().contains("cone"),
        "{e}"
    );
    assert!(fit_cone(&p[..5], None, &all(5), Clip::None).is_err());
}

/// Points on a circle of radius `r` round `c`, in the plane across `n`, over `arc` degrees, `noise` radially.
fn circle_points(c: V3, n: V3, r: f64, arc: f64, noise: f64, count: usize) -> Vec<[f32; 3]> {
    let (u, v) = across(norm(n));
    (0..count)
        .map(|i| {
            let a = arc.to_radians() * i as f64 / count as f64;
            let rr = r + noise * wobble(i);
            [0, 1, 2].map(|k| (c[k] + rr * (a.cos() * u[k] + a.sin() * v[k])) as f32)
        })
        .collect()
}

#[test]
fn circles_full_partial_and_noisy() {
    let (c, n) = ([1.0, 2.0, 3.0], [0.2, -1.0, 0.5]);
    let f = fit_circle(
        &circle_points(c, n, 7.0, 360.0, 0.0, 90),
        &all(90),
        Clip::None,
    )
    .unwrap();
    assert!(
        // Positions are stored as f32: about 1e-6 mm here.
        (f.radius - 7.0).abs() < 1e-5 && dist(f.center, c) < 1e-5,
        "{f:?}"
    );
    assert!(f.coverage_deg > 355.0);
    let nn = norm(n);
    assert!((f.normal[0] * nn[0] + f.normal[1] * nn[1] + f.normal[2] * nn[2]).abs() > 1.0 - 1e-9);
    let f = fit_circle(
        &circle_points(c, n, 7.0, 90.0, 0.0, 40),
        &all(40),
        Clip::None,
    )
    .unwrap();
    assert!(
        (f.radius - 7.0).abs() < 1e-5 && (f.coverage_deg - 87.75).abs() < 0.01,
        "{f:?}"
    );
    let f = fit_circle(
        &circle_points(c, n, 7.0, 360.0, 0.02, 200),
        &all(200),
        Clip::Sigma(3),
    )
    .unwrap();
    assert!((f.radius - 7.0).abs() < 0.005, "{f:?}");
    // On a line: refused.
    let line: Vec<[f32; 3]> = (0..10).map(|i| [i as f32, 0.0, 0.0]).collect();
    assert!(fit_circle(&line, &all(10), Clip::None).is_err());
}
