// SPDX-License-Identifier: MIT OR Apache-2.0
//! fit.sphere checks: points on spheres whose centre and radius are known (docs/specs/fit-sphere.md).
#![allow(clippy::unwrap_used, missing_docs)]
use rulercraft_mesh::fit::{Clip, FitError, fit_sphere};

/// Deterministic uniform noise in [-1, 1).
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

/// `n` evenly spread points on the cap within `cap_deg` of +z, on a sphere at `c` of radius `r`,
/// each moved radially by `noise * u` with u uniform in [-1, 1).
fn cap(c: [f64; 3], r: f64, cap_deg: f64, n: usize, noise: f64, seed: u64) -> Vec<[f32; 3]> {
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    let zmin = cap_deg.to_radians().cos();
    let mut rng = Noise(seed);
    (0..n)
        .map(|i| {
            let z = 1.0 - (1.0 - zmin) * (i as f64 + 0.5) / n as f64;
            let s = (1.0 - z * z).sqrt();
            let t = golden * i as f64;
            let rr = r + noise * rng.next();
            [
                c[0] + rr * s * t.cos(),
                c[1] + rr * s * t.sin(),
                c[2] + rr * z,
            ]
            .map(|v| v as f32)
        })
        .collect()
}

fn all(p: &[[f32; 3]]) -> Vec<u32> {
    (0..p.len() as u32).collect()
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[test]
fn whole_ball_gives_centre_and_radius() {
    let p = cap([3.0, -4.0, 5.0], 12.7, 180.0, 500, 0.0, 1);
    let f = fit_sphere(&p, &all(&p), Clip::Sigma(3), None).unwrap();
    assert!((f.radius - 12.7).abs() < 1e-5, "{f:?}");
    assert!(dist(f.center, [3.0, -4.0, 5.0]) < 1e-5);
    assert!((f.diameter - 25.4).abs() < 2e-5);
    assert!(f.sigma < 1e-5 && f.cap_angle_deg > 170.0);
}

#[test]
fn shallow_cap_still_gives_the_radius() {
    let p = cap([0.0, 0.0, 0.0], 12.7, 40.0, 300, 0.0, 1);
    let f = fit_sphere(&p, &all(&p), Clip::None, None).unwrap();
    assert!((f.radius - 12.7).abs() < 1e-4, "{f:?}");
    assert!(
        (f.cap_angle_deg - 40.0).abs() < 1.0,
        "cap {}",
        f.cap_angle_deg
    );
    let half = fit_sphere(
        &cap([0.0; 3], 5.0, 90.0, 300, 0.0, 1),
        &all(&p),
        Clip::None,
        None,
    )
    .unwrap();
    assert!(
        (half.cap_angle_deg - 90.0).abs() < 1.0,
        "hemisphere {}",
        half.cap_angle_deg
    );
}

#[test]
fn noisy_ball_far_from_the_origin() {
    let a = 0.0173; // uniform half-width; standard deviation a/√3 ≈ 0.01
    let c = [2.0e4, -1.5e4, 3.0e3];
    let p = cap(c, 10.0, 120.0, 2000, a, 9);
    let f = fit_sphere(&p, &all(&p), Clip::None, None).unwrap();
    assert!((f.radius - 10.0).abs() < 2e-3, "{f:?}");
    assert!(dist(f.center, c) < 3e-3);
    assert!(
        (f.sigma - a / 3f64.sqrt()).abs() < 2e-3,
        "sigma {}",
        f.sigma
    );
}

#[test]
fn clipping_drops_spikes_near_the_surface() {
    // What scans produce: spikes a millimetre or so off an otherwise clean surface.
    let mut p = cap([0.0; 3], 8.0, 100.0, 400, 0.005, 2);
    let good = p.len();
    p.extend(cap([0.0; 3], 9.5, 100.0, 12, 0.0, 5));
    let f = fit_sphere(&p, &all(&p), Clip::Sigma(3), None).unwrap();
    assert_eq!(f.used, good, "{f:?}");
    assert!((f.radius - 8.0).abs() < 1e-3, "{f:?}");
    let raw = fit_sphere(&p, &all(&p), Clip::None, None).unwrap();
    assert!(
        (raw.radius - 8.0).abs() > 0.02,
        "spikes should pull the unclipped fit: {}",
        raw.radius
    );
}

#[test]
fn far_strays_are_a_known_limit_of_sigma_clipping() {
    // A few points far off the surface pull the least-squares sphere until they sit closer to it than
    // the good points do, so sigma clipping cannot find them. Robust seeding (fit.robust, R3) is the fix;
    // this records the limit so nobody mistakes clipping for robustness (docs/specs/fit-sphere.md).
    let mut p = cap([0.0; 3], 8.0, 100.0, 400, 0.005, 2);
    for i in 0..6 {
        p.push([i as f32, 0.0, 30.0]);
    }
    let f = fit_sphere(&p, &all(&p), Clip::Sigma(3), None).unwrap();
    assert!(
        (f.radius - 8.0).abs() > 1.0,
        "if this starts passing, clipping got robust: update the spec"
    );
}

#[test]
fn known_diameter_places_the_centre_and_reports_the_free_fit() {
    let c = [1.0, 2.0, 3.0];
    let p = cap(c, 12.7, 60.0, 300, 0.01, 4);
    let f = fit_sphere(&p, &all(&p), Clip::None, Some(25.4)).unwrap();
    assert_eq!(f.radius, 12.7);
    assert!(dist(f.center, c) < 5e-3, "{f:?}");
    assert!((f.free_diameter.unwrap() - 25.4).abs() < 0.05);
    assert!(matches!(
        fit_sphere(&p, &all(&p), Clip::None, Some(40.0)),
        Err(FitError::Degenerate(_))
    ));
    assert!(matches!(
        fit_sphere(&p, &all(&p), Clip::None, Some(-1.0)),
        Err(FitError::Degenerate(_))
    ));
}

#[test]
fn refusals_say_what_is_wrong() {
    let p = cap([0.0; 3], 5.0, 90.0, 50, 0.0, 1);
    assert_eq!(
        fit_sphere(&p, &[0, 1, 2], Clip::None, None),
        Err(FitError::TooFewPoints {
            found: 3,
            needed: 4
        })
    );
    assert_eq!(
        fit_sphere(&p, &[0, 1, 2, 500], Clip::None, None),
        Err(FitError::NoSuchVertex {
            index: 500,
            count: 50
        })
    );
    let flat: Vec<[f32; 3]> = (0..25)
        .map(|i| [(i % 5) as f32, (i / 5) as f32, 0.0])
        .collect();
    assert!(matches!(
        fit_sphere(&flat, &all(&flat), Clip::None, None),
        Err(FitError::Degenerate(_))
    ));
    let mut bad = p.clone();
    bad[3][0] = f32::INFINITY;
    assert_eq!(
        fit_sphere(&bad, &all(&bad), Clip::None, None),
        Err(FitError::NotFinite)
    );
}

fn rms_at(p: &[[f32; 3]], c: [f64; 3], r: f64) -> f64 {
    let sum: f64 = p
        .iter()
        .map(|q| (dist([f64::from(q[0]), f64::from(q[1]), f64::from(q[2])], c) - r).powi(2))
        .sum();
    (sum / p.len() as f64).sqrt()
}

#[test]
fn result_is_the_least_squares_optimum_not_just_the_algebraic_start() {
    // On a noisy shallow cap the algebraic start is biased; the geometric fit must be a true minimum
    // of the RMS radial residual: nudging the centre or radius any way can only make it worse.
    let p = cap([0.0, 0.0, 0.0], 10.0, 35.0, 400, 0.3, 11);
    let f = fit_sphere(&p, &all(&p), Clip::None, None).unwrap();
    let best = rms_at(&p, f.center, f.radius);
    assert!((best - f.sigma).abs() < 1e-12);
    for h in [1e-3, -1e-3] {
        for k in 0..4 {
            let mut c = f.center;
            let mut r = f.radius;
            if k < 3 {
                c[k] += h;
            } else {
                r += h;
            }
            assert!(
                rms_at(&p, c, r) >= best - 1e-12,
                "nudge {k} by {h} improved the fit: {f:?}"
            );
        }
    }
    // The same for a held diameter: only the centre moves.
    let held = fit_sphere(&p, &all(&p), Clip::None, Some(2.0 * f.radius + 0.1)).unwrap();
    let best = rms_at(&p, held.center, held.radius);
    for h in [1e-3, -1e-3] {
        for k in 0..3 {
            let mut c = held.center;
            c[k] += h;
            assert!(
                rms_at(&p, c, held.radius) >= best - 1e-12,
                "held: nudge {k} by {h} improved it"
            );
        }
    }
}
