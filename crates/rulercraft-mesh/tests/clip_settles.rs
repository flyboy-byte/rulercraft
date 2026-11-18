// SPDX-License-Identifier: MIT OR Apache-2.0
//! Sigma clipping settles on heavy-tailed residuals (docs/findings/2026-11-07-clipping-on-real-scans.md).
//! The cut-off is k times the sigma of the fit to all offered points, fixed for every round, so the kept
//! set stops changing instead of shrinking until the round cap (docs/specs/fit-plane.md).
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::fit::{Clip, CylinderOptions, fit_cylinder, fit_plane, fit_sphere};

/// Deterministic uniform in (0, 1).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
    /// Laplace-distributed magnitude with scale `b`: heavier tails than Gaussian noise, like a real scan.
    fn laplace(&mut self, b: f64) -> f64 {
        -b * self.next().ln()
    }
}

/// Points on z = 0 in mirrored pairs (x, y, +e) and (x, y, -e), so the best plane is exactly z = 0 and
/// the right kept set can be counted without running any fit.
#[test]
fn plane_keeps_exactly_the_points_within_k_first_sigma() {
    let mut r = Lcg(7);
    let mut p: Vec<[f32; 3]> = vec![];
    for i in 0..60 {
        for j in 0..60 {
            let e = r.laplace(0.05) as f32;
            p.push([i as f32 * 0.5, j as f32 * 0.5, e]);
            p.push([i as f32 * 0.5, j as f32 * 0.5, -e]);
        }
    }
    let sel: Vec<u32> = (0..p.len() as u32).collect();
    let z: Vec<f64> = p.iter().map(|q| f64::from(q[2])).collect();
    let sigma0 = (z.iter().map(|v| v * v).sum::<f64>() / z.len() as f64).sqrt();
    let all = fit_plane(&p, None, &sel, Clip::None).unwrap();
    assert!(
        (all.sigma - sigma0).abs() < 1e-9,
        "all-points sigma {}",
        all.sigma
    );
    for k in 1..=3u8 {
        let cut = f64::from(k) * sigma0;
        let want = z.iter().filter(|v| v.abs() <= cut).count();
        let f = fit_plane(&p, None, &sel, Clip::Sigma(k)).unwrap();
        assert_eq!(f.used, want, "k={k}: kept count");
        // One fit on all points, one on the clipped set, then the set no longer changes.
        assert_eq!(f.rounds, 2, "k={k}: rounds");
        assert!(
            f.normal[2].abs() > 1.0 - 1e-12,
            "k={k}: normal {:?}",
            f.normal
        );
    }
}

/// For sphere and cylinder there is no closed-form kept set, so check the defining property: the kept
/// points are exactly those within k times the all-points sigma of the reported fit, reached before the cap.
#[test]
fn sphere_settles_on_heavy_tails() {
    let mut r = Lcg(11);
    let mut p: Vec<[f32; 3]> = vec![];
    for i in 0..40 {
        for j in 0..80 {
            let th = (f64::from(i) + 0.5) / 40.0 * std::f64::consts::PI;
            let ph = f64::from(j) / 80.0 * std::f64::consts::TAU;
            let e = r.laplace(0.05);
            let sign = if r.next() < 0.5 { -1.0 } else { 1.0 };
            let rad = 10.0 + sign * e;
            p.push([
                (5.0 + rad * th.sin() * ph.cos()) as f32,
                (-3.0 + rad * th.sin() * ph.sin()) as f32,
                (2.0 + rad * th.cos()) as f32,
            ]);
        }
    }
    let sel: Vec<u32> = (0..p.len() as u32).collect();
    let sigma0 = fit_sphere(&p, &sel, Clip::None, None).unwrap().sigma;
    let f = fit_sphere(&p, &sel, Clip::Sigma(3), None).unwrap();
    assert!(f.rounds < 10, "hit the round cap: {}", f.rounds);
    let res = |q: &[f32; 3]| {
        let d = [0, 1, 2].map(|a| f64::from(q[a]) - f.center[a]);
        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() - f.radius
    };
    let within = p.iter().filter(|q| res(q).abs() <= 3.0 * sigma0).count();
    assert_eq!(f.used, within);
    assert!((f.radius - 10.0).abs() < 0.01, "radius {}", f.radius);
}

#[test]
fn cylinder_settles_on_heavy_tails() {
    let mut r = Lcg(13);
    let (mut p, mut n): (Vec<[f32; 3]>, Vec<[f32; 3]>) = (vec![], vec![]);
    for i in 0..30 {
        for j in 0..90 {
            let a = f64::from(j) / 90.0 * std::f64::consts::TAU;
            let e = r.laplace(0.05);
            let sign = if r.next() < 0.5 { -1.0 } else { 1.0 };
            let rad = 8.0 + sign * e;
            p.push([(rad * a.cos()) as f32, (rad * a.sin()) as f32, i as f32]);
            n.push([a.cos() as f32, a.sin() as f32, 0.0]);
        }
    }
    let sel: Vec<u32> = (0..p.len() as u32).collect();
    let sigma0 = fit_cylinder(&p, Some(&n), &sel, CylinderOptions::default())
        .unwrap()
        .sigma;
    let opts = CylinderOptions {
        clip: Clip::Sigma(3),
        ..CylinderOptions::default()
    };
    let f = fit_cylinder(&p, Some(&n), &sel, opts).unwrap();
    assert!(f.rounds < 10, "hit the round cap: {}", f.rounds);
    let res = |q: &[f32; 3]| {
        let d = [0, 1, 2].map(|a| f64::from(q[a]) - f.axis_point[a]);
        let t = d[0] * f.axis[0] + d[1] * f.axis[1] + d[2] * f.axis[2];
        let o = [0, 1, 2].map(|a| d[a] - t * f.axis[a]);
        (o[0] * o[0] + o[1] * o[1] + o[2] * o[2]).sqrt() - f.radius
    };
    let within = p.iter().filter(|q| res(q).abs() <= 3.0 * sigma0).count();
    assert_eq!(f.used, within);
    assert!((f.radius - 8.0).abs() < 0.01, "radius {}", f.radius);
}
