// SPDX-License-Identifier: MIT OR Apache-2.0
//! Sphere fit: algebraic start, geometric Levenberg-Marquardt refinement, k-sigma clipping
//! (docs/specs/fit-sphere.md).
//!
//! The algebraic start uses |p|² = 2c·p + (r² − |c|²), which is linear in (c, r² − |c|²). Points are
//! shifted to their centroid first so the linear system stays well conditioned far from the origin.

use super::plane::{Clip, FitError};
use crate::math::{
    LmOptions, Vec3, covariance, dot, levenberg_marquardt, solve_dense, symmetric_eigen3,
};

/// A fitted sphere. Lengths in mm, angles in degrees.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SphereFit {
    /// Centre.
    pub center: Vec3,
    /// Radius (the known one when a diameter was held).
    pub radius: f64,
    /// 2 × radius.
    pub diameter: f64,
    /// RMS radial residual of the kept points.
    pub sigma: f64,
    /// Max minus min radial residual.
    pub form_error: f64,
    /// Most negative radial residual (inside the surface).
    pub min_residual: f64,
    /// Most positive radial residual.
    pub max_residual: f64,
    /// Points kept by the clipping.
    pub used: usize,
    /// Points offered.
    pub offered: usize,
    /// Clipping rounds taken.
    pub rounds: usize,
    /// Cap angle the kept points cover, degrees (90 = hemisphere, 180 = whole ball).
    pub cap_angle_deg: f64,
    /// With a known diameter: the diameter the same points fit freely.
    pub free_diameter: Option<f64>,
}

const MAX_ROUNDS: usize = 10;

/// Fits a sphere to the `selection` of `positions`. With `known_diameter`, only the centre is fitted.
pub fn fit_sphere(
    positions: &[[f32; 3]],
    selection: &[u32],
    clip: Clip,
    known_diameter: Option<f64>,
) -> Result<SphereFit, FitError> {
    if selection.len() < 4 {
        return Err(FitError::TooFewPoints {
            found: selection.len(),
            needed: 4,
        });
    }
    let k = match clip {
        Clip::None => None,
        Clip::Sigma(k @ 1..=3) => Some(f64::from(k)),
        Clip::Sigma(k) => return Err(FitError::BadClip(k)),
    };
    if let Some(d) = known_diameter
        && !(d.is_finite() && d > 0.0)
    {
        return Err(FitError::Degenerate(
            "a known diameter must be a positive number",
        ));
    }
    let points = super::gather(positions, selection)?;

    let mut kept: Vec<usize> = (0..points.len()).collect();
    let mut s = solve(&points, &kept, None)?;
    let mut rounds = 1;
    if let Some(k) = k {
        // The cut-off is fixed from the all-points fit; shrinking it every round creeps on heavy tails.
        let cut = k * rms(&points, &kept, &s);
        while rounds < MAX_ROUNDS && cut > 0.0 {
            let next: Vec<usize> = (0..points.len())
                .filter(|&i| residual(&points[i], &s).abs() <= cut)
                .collect();
            if next == kept || next.len() < 4 {
                break;
            }
            match solve(&points, &next, None) {
                Ok(n) => {
                    s = n;
                    kept = next;
                    rounds += 1;
                }
                Err(_) => break,
            }
        }
    }

    let mut free_diameter = None;
    if let Some(d) = known_diameter {
        let free = 2.0 * s.radius;
        if (free - d).abs() > (0.1 * d).max(1.0) {
            return Err(FitError::Degenerate(
                "the marked surface fits a sphere far from the known diameter, so it is probably not that ball",
            ));
        }
        free_diameter = Some(free);
        s = solve(&points, &kept, Some(d / 2.0))?;
    }

    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut mean_dir = [0.0; 3];
    let mut dirs = Vec::with_capacity(kept.len());
    for &i in &kept {
        let r = residual(&points[i], &s);
        lo = lo.min(r);
        hi = hi.max(r);
        let d = sub(points[i], s.center);
        let l = dot(d, d).sqrt();
        if l > 0.0 {
            let u = [d[0] / l, d[1] / l, d[2] / l];
            dirs.push(u);
            mean_dir = [mean_dir[0] + u[0], mean_dir[1] + u[1], mean_dir[2] + u[2]];
        }
    }
    let cap = match crate::math::normalize(mean_dir) {
        // Points all round the ball: their directions cancel out.
        None => 180.0,
        Some(m) => dirs
            .iter()
            .map(|u| dot(*u, m).clamp(-1.0, 1.0).acos())
            .fold(0.0, f64::max)
            .to_degrees(),
    };
    Ok(SphereFit {
        center: s.center,
        radius: s.radius,
        diameter: 2.0 * s.radius,
        sigma: rms(&points, &kept, &s),
        form_error: hi - lo,
        min_residual: lo,
        max_residual: hi,
        used: kept.len(),
        offered: selection.len(),
        rounds,
        cap_angle_deg: cap,
        free_diameter,
    })
}

struct Sphere {
    center: Vec3,
    radius: f64,
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn residual(p: &Vec3, s: &Sphere) -> f64 {
    let d = sub(*p, s.center);
    dot(d, d).sqrt() - s.radius
}

fn rms(points: &[Vec3], kept: &[usize], s: &Sphere) -> f64 {
    let sum: f64 = kept.iter().map(|&i| residual(&points[i], s).powi(2)).sum();
    (sum / kept.len() as f64).sqrt()
}

/// Algebraic start then geometric refinement on the kept points; `held` fixes the radius.
fn solve(points: &[Vec3], kept: &[usize], held: Option<f64>) -> Result<Sphere, FitError> {
    let cov = covariance(kept.iter().map(|&i| points[i])).ok_or(FitError::TooFewPoints {
        found: 0,
        needed: 4,
    })?;
    let e = symmetric_eigen3(cov.matrix).ok_or(FitError::NotFinite)?;
    if e.values[2] == 0.0 || e.values[0] <= 1e-10 * e.values[2] {
        return Err(FitError::Degenerate(
            "the marked points lie (nearly) in one plane, so no sphere fits them",
        ));
    }
    let o = cov.centroid;
    let local: Vec<Vec3> = kept.iter().map(|&i| sub(points[i], o)).collect();

    // Normal equations of the linear model [2x 2y 2z 1]·(c, d) = |p|².
    let mut ata = vec![0.0; 16];
    let mut atb = vec![0.0; 4];
    for p in &local {
        let row = [2.0 * p[0], 2.0 * p[1], 2.0 * p[2], 1.0];
        let b = dot(*p, *p);
        for a in 0..4 {
            atb[a] += row[a] * b;
            for c in 0..4 {
                ata[a * 4 + c] += row[a] * row[c];
            }
        }
    }
    let x = solve_dense(ata, atb).ok_or(FitError::Degenerate(
        "the marked points do not determine a sphere",
    ))?;
    let c0 = [x[0], x[1], x[2]];
    let r0 = (x[3] + dot(c0, c0)).sqrt();
    if !(r0.is_finite() && r0 > 0.0) {
        return Err(FitError::Degenerate(
            "the marked points do not determine a sphere",
        ));
    }

    let n = local.len();
    let report = match held {
        None => levenberg_marquardt(
            &[c0[0], c0[1], c0[2], r0],
            n,
            LmOptions::default(),
            |x, res, jac| radial(&local, [x[0], x[1], x[2]], x[3], res, jac, true),
        ),
        Some(r) => levenberg_marquardt(
            &[c0[0], c0[1], c0[2]],
            n,
            LmOptions::default(),
            |x, res, jac| radial(&local, [x[0], x[1], x[2]], r, res, jac, false),
        ),
    }
    .ok_or(FitError::Degenerate("the sphere fit did not converge"))?;
    let radius = held.unwrap_or(report.x.get(3).copied().unwrap_or(f64::NAN));
    let center = [report.x[0] + o[0], report.x[1] + o[1], report.x[2] + o[2]];
    if !(radius.is_finite() && radius > 0.0) || !center.iter().all(|v| v.is_finite()) {
        return Err(FitError::Degenerate("the sphere fit did not converge"));
    }
    Ok(Sphere { center, radius })
}

/// Residuals |p − c| − r and their derivatives in (c, r) (or c alone when the radius is held).
fn radial(
    local: &[Vec3],
    c: Vec3,
    r: f64,
    res: &mut [f64],
    jac: Option<&mut [f64]>,
    with_r: bool,
) -> bool {
    let cols = if with_r { 4 } else { 3 };
    let mut jac = jac;
    for (i, p) in local.iter().enumerate() {
        let d = sub(*p, c);
        let l = dot(d, d).sqrt();
        if l == 0.0 {
            return false;
        }
        res[i] = l - r;
        if let Some(j) = jac.as_deref_mut() {
            for a in 0..3 {
                j[i * cols + a] = -d[a] / l;
            }
            if with_r {
                j[i * cols + 3] = -1.0;
            }
        }
    }
    true
}
