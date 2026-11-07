// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cross-checks that need no expected answer: moving, turning or rescaling a part must move its fits
//! exactly with it and leave their sizes and errors unchanged.
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::fit::{Clip, CylinderOptions, fit_cylinder, fit_plane, fit_sphere};
use rulercraft_mesh::math::{Vec3, dot, normalize};

struct Noise(u64);
impl Noise {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 52) as f64 - 1.0
    }
}

/// A rigid motion: rotation about `axis` by `angle`, then a shift.
struct Motion {
    r: [[f64; 3]; 3],
    t: Vec3,
}
impl Motion {
    fn new(axis: Vec3, angle: f64, t: Vec3) -> Self {
        let [x, y, z] = normalize(axis).unwrap();
        let (s, c) = angle.sin_cos();
        let k = 1.0 - c;
        Motion {
            r: [
                [k * x * x + c, k * x * y - s * z, k * x * z + s * y],
                [k * x * y + s * z, k * y * y + c, k * y * z - s * x],
                [k * x * z - s * y, k * y * z + s * x, k * z * z + c],
            ],
            t,
        }
    }
    fn dir(&self, v: Vec3) -> Vec3 {
        [0, 1, 2].map(|i| self.r[i][0] * v[0] + self.r[i][1] * v[1] + self.r[i][2] * v[2])
    }
    fn point(&self, v: Vec3) -> Vec3 {
        let d = self.dir(v);
        [d[0] + self.t[0], d[1] + self.t[1], d[2] + self.t[2]]
    }
}

fn f32s(v: &[Vec3]) -> Vec<[f32; 3]> {
    v.iter().map(|p| p.map(|x| x as f32)).collect()
}

fn near(a: Vec3, b: Vec3, tol: f64) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() <= tol)
}

/// A noisy patch, ball cap and tube, each with its outward normals.
fn shapes() -> (Vec<Vec3>, Vec<Vec3>, Vec<Vec3>, Vec<Vec3>) {
    let mut n = Noise(42);
    let plane: Vec<Vec3> = (0..400)
        .map(|i| [f64::from(i % 20), f64::from(i / 20), 0.02 * n.next()])
        .collect();
    let ball: Vec<Vec3> = (0..500)
        .map(|i| {
            let z = 1.0 - 1.2 * (f64::from(i) + 0.5) / 500.0;
            let s = (1.0 - z * z).sqrt();
            let t = 2.399963 * f64::from(i);
            let r = 9.0 + 0.02 * n.next();
            [r * s * t.cos(), r * s * t.sin(), r * z]
        })
        .collect();
    let (mut tube, mut tube_n) = (vec![], vec![]);
    for ring in 0..8 {
        for i in 0..30 {
            let a = std::f64::consts::TAU * f64::from(i) / 30.0;
            let r = 4.0 + 0.01 * n.next();
            tube.push([r * a.cos(), r * a.sin(), f64::from(ring) * 2.5]);
            tube_n.push([a.cos(), a.sin(), 0.0]);
        }
    }
    (plane, ball, tube, tube_n)
}

#[test]
fn rigid_motion_moves_every_fit_and_keeps_its_numbers() {
    let (plane, ball, tube, tube_n) = shapes();
    for m in [
        Motion::new([1.0, 2.0, 3.0], 0.9, [40.0, -25.0, 10.0]),
        Motion::new([-0.4, 0.1, 0.7], 2.6, [-300.0, 120.0, 55.0]),
    ] {
        let sel = |n: usize| (0..n as u32).collect::<Vec<u32>>();
        let a = fit_plane(&f32s(&plane), None, &sel(plane.len()), Clip::None).unwrap();
        let moved: Vec<Vec3> = plane.iter().map(|p| m.point(*p)).collect();
        let b = fit_plane(&f32s(&moved), None, &sel(plane.len()), Clip::None).unwrap();
        assert!(
            dot(m.dir(a.normal), b.normal).abs() > 1.0 - 1e-9,
            "plane normal"
        );
        assert!(near(m.point(a.center), b.center, 2e-4), "plane centre");
        assert!(
            (a.sigma - b.sigma).abs() < 1e-5 && (a.form_error - b.form_error).abs() < 1e-4,
            "plane numbers"
        );

        let a = fit_sphere(&f32s(&ball), &sel(ball.len()), Clip::None, None).unwrap();
        let moved: Vec<Vec3> = ball.iter().map(|p| m.point(*p)).collect();
        let b = fit_sphere(&f32s(&moved), &sel(ball.len()), Clip::None, None).unwrap();
        assert!(
            (a.radius - b.radius).abs() < 1e-4,
            "sphere radius {} {}",
            a.radius,
            b.radius
        );
        assert!(near(m.point(a.center), b.center, 2e-4), "sphere centre");
        assert!(
            (a.cap_angle_deg - b.cap_angle_deg).abs() < 1e-3,
            "sphere coverage"
        );

        let opts = CylinderOptions::default();
        let a = fit_cylinder(&f32s(&tube), Some(&f32s(&tube_n)), &sel(tube.len()), opts).unwrap();
        let moved: Vec<Vec3> = tube.iter().map(|p| m.point(*p)).collect();
        let moved_n: Vec<Vec3> = tube_n.iter().map(|v| m.dir(*v)).collect();
        let b = fit_cylinder(&f32s(&moved), Some(&f32s(&moved_n)), &sel(tube.len()), opts).unwrap();
        assert!(
            (a.radius - b.radius).abs() < 1e-4 && (a.length - b.length).abs() < 1e-3,
            "cylinder size"
        );
        assert!(
            dot(m.dir(a.axis), b.axis).abs() > 1.0 - 1e-9,
            "cylinder axis"
        );
        assert!((a.sigma - b.sigma).abs() < 1e-5, "cylinder sigma");
    }
}

#[test]
fn rescaling_scales_lengths_and_keeps_angles() {
    let (plane, ball, tube, tube_n) = shapes();
    let k = 25.4;
    let scale = |v: &[Vec3]| v.iter().map(|p| p.map(|x| x * k)).collect::<Vec<Vec3>>();
    let sel = |n: usize| (0..n as u32).collect::<Vec<u32>>();
    let a = fit_plane(&f32s(&plane), None, &sel(plane.len()), Clip::None).unwrap();
    let b = fit_plane(&f32s(&scale(&plane)), None, &sel(plane.len()), Clip::None).unwrap();
    assert!(dot(a.normal, b.normal).abs() > 1.0 - 1e-9 && (b.sigma / a.sigma - k).abs() < 1e-3);
    let a = fit_sphere(&f32s(&ball), &sel(ball.len()), Clip::None, None).unwrap();
    let b = fit_sphere(&f32s(&scale(&ball)), &sel(ball.len()), Clip::None, None).unwrap();
    assert!(
        (b.radius / a.radius - k).abs() < 1e-6 && (a.cap_angle_deg - b.cap_angle_deg).abs() < 1e-6
    );
    let opts = CylinderOptions::default();
    let a = fit_cylinder(&f32s(&tube), Some(&f32s(&tube_n)), &sel(tube.len()), opts).unwrap();
    for k2 in [2.0, 5.0, 10.0, 25.4, 100.0] {
        let t: Vec<Vec3> = tube.iter().map(|p| p.map(|x| x * k2)).collect();
        eprintln!(
            "DBG scale {k2}: {:?}",
            fit_cylinder(&f32s(&t), Some(&f32s(&tube_n)), &sel(tube.len()), opts).map(|c| c.radius)
        );
    }
    let b = fit_cylinder(
        &f32s(&scale(&tube)),
        Some(&f32s(&tube_n)),
        &sel(tube.len()),
        opts,
    )
    .unwrap();
    eprintln!(
        "DBG cyl r {} {} ratio {} len {} {} ratio {}",
        a.radius,
        b.radius,
        b.radius / a.radius,
        a.length,
        b.length,
        b.length / a.length
    );
    assert!((b.radius / a.radius - k).abs() < 1e-6 && (b.length / a.length - k).abs() < 1e-6);
}

#[test]
fn selection_order_does_not_matter() {
    let (plane, ball, _, _) = shapes();
    let fwd: Vec<u32> = (0..ball.len() as u32).collect();
    let rev: Vec<u32> = fwd.iter().rev().copied().collect();
    let (a, b) = (
        fit_sphere(&f32s(&ball), &fwd, Clip::Sigma(3), None).unwrap(),
        fit_sphere(&f32s(&ball), &rev, Clip::Sigma(3), None).unwrap(),
    );
    assert!((a.radius - b.radius).abs() < 1e-9 && a.used == b.used);
    let fwd: Vec<u32> = (0..plane.len() as u32).collect();
    let rev: Vec<u32> = fwd.iter().rev().copied().collect();
    let (a, b) = (
        fit_plane(&f32s(&plane), None, &fwd, Clip::Sigma(2)).unwrap(),
        fit_plane(&f32s(&plane), None, &rev, Clip::Sigma(2)).unwrap(),
    );
    assert!((a.sigma - b.sigma).abs() < 1e-9 && a.used == b.used);
}

#[test]
fn every_fit_works_from_a_hundredth_to_a_thousand_times_the_size() {
    // A fit must not depend on absolute size: 25.4x (mm to inches) once made the cylinder fit stall.
    let (plane, ball, tube, tube_n) = shapes();
    let sel = |n: usize| (0..n as u32).collect::<Vec<u32>>();
    for k in [0.01, 0.1, 0.5, 2.0, 7.3, 25.4, 39.37, 100.0, 1000.0] {
        let scale = |v: &[Vec3]| v.iter().map(|p| p.map(|x| x * k)).collect::<Vec<Vec3>>();
        let p = fit_plane(
            &f32s(&scale(&plane)),
            None,
            &sel(plane.len()),
            Clip::Sigma(3),
        );
        assert!(p.is_ok(), "plane at {k}x: {p:?}");
        let b = fit_sphere(&f32s(&scale(&ball)), &sel(ball.len()), Clip::Sigma(3), None);
        assert!(
            b.as_ref()
                .is_ok_and(|b| (b.radius / (9.0 * k) - 1.0).abs() < 1e-3),
            "sphere at {k}x: {b:?}"
        );
        let c = fit_cylinder(
            &f32s(&scale(&tube)),
            Some(&f32s(&tube_n)),
            &sel(tube.len()),
            CylinderOptions::default(),
        );
        assert!(
            c.as_ref()
                .is_ok_and(|c| (c.radius / (4.0 * k) - 1.0).abs() < 1e-3),
            "cylinder at {k}x: {c:?}"
        );
    }
}

#[test]
fn tilted_axis_spacing_survives_rigid_motion_and_pair_order() {
    use rulercraft_mesh::measure::{Element, relate};
    let theta = 0.5f64.to_radians();
    let make = |point, axis| Element::Cylinder {
        point,
        axis,
        radius: 2.,
    };
    let (p, a, q, b) = (
        [0.; 3],
        [0., 0., 1.],
        [10., 3., 0.],
        [theta.sin(), 0., theta.cos()],
    );
    let expected = relate(&make(p, a), &make(q, b));
    assert!((expected.distance.unwrap() - 3.).abs() < 1e-9); // exact skew-line separation, f64 roundoff
    let motion = Motion::new([1., 2., 3.], 0.9, [40., -25., 10.]);
    let first = make(motion.point(p), motion.dir(a));
    let second = make(motion.point(q), motion.dir(b));
    for r in [relate(&first, &second), relate(&second, &first)] {
        assert!((r.distance.unwrap() - 3.).abs() < 1e-9);
        assert!((r.angle_deg.unwrap() - 0.5).abs() < 1e-9);
    }
}
