// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cylinder fit from deterministic axis starts and geometric radial least squares.
use super::{Clip, FitError};
use crate::math::{
    LmOptions, Vec3, add, covariance, cross, dot, length, levenberg_marquardt, normalize, scale,
    solve_dense, sub, symmetric_eigen3,
};
/// Caller-selected bounds on cylinder fitting work.
#[derive(Debug, Clone, Copy)]
pub struct CylinderLimits {
    /// Maximum selected points.
    pub max_points: usize,
    /// Maximum nonlinear solver iterations per fit.
    pub max_iterations: usize,
    /// Maximum fits including clipping rounds.
    pub max_rounds: usize,
    /// Maximum candidate axes scored per fit.
    pub max_starts: usize,
}
impl Default for CylinderLimits {
    fn default() -> Self {
        Self {
            max_points: 1_000_000,
            max_iterations: 100,
            max_rounds: 10,
            max_starts: 8,
        }
    }
}
/// Fit controls and carried display metadata.
#[derive(Debug, Clone, Copy)]
pub struct CylinderOptions {
    /// Outlier clipping.
    pub clip: Clip,
    /// Designed diameter metadata, not a measurement constraint.
    pub assumed_diameter: Option<f64>,
    /// Display extension before the measured span, mm.
    pub extend_before: f64,
    /// Display extension after the measured span, mm.
    pub extend_after: f64,
    /// Work bounds.
    pub limits: CylinderLimits,
}
impl Default for CylinderOptions {
    fn default() -> Self {
        Self {
            clip: Clip::None,
            assumed_diameter: None,
            extend_before: 0.,
            extend_after: 0.,
            limits: CylinderLimits::default(),
        }
    }
}
/// Measured cylinder, lengths in millimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct CylinderFit {
    /// Point on axis at midpoint of measured axial span.
    pub axis_point: Vec3,
    /// Unit axis, largest absolute component positive.
    pub axis: Vec3,
    /// Measured radius.
    pub radius: f64,
    /// Twice the measured radius.
    pub diameter: f64,
    /// Axial extent of kept points.
    pub length: f64,
    /// Angular extent, 360 minus widest sampled gap, degrees.
    pub coverage_deg: f64,
    /// RMS radial residual.
    pub sigma: f64,
    /// Maximum minus minimum radial residual.
    pub form_error: f64,
    /// Minimum signed radial residual.
    pub min_residual: f64,
    /// Maximum signed radial residual.
    pub max_residual: f64,
    /// Number of kept points.
    pub used: usize,
    /// Number of offered points.
    pub offered: usize,
    /// Fits including clipping rounds.
    pub rounds: usize,
    /// Carried designed diameter, not used in fitting.
    pub assumed_diameter: Option<f64>,
    /// Carried display extension before measured span.
    pub extend_before: f64,
    /// Carried display extension after measured span.
    pub extend_after: f64,
}
const NO_AXIS: FitError =
    FitError::Degenerate("the marked surface curves too little to determine a cylinder axis");
fn basis(a: Vec3) -> Option<(Vec3, Vec3)> {
    let k = (0..3).min_by(|&i, &j| a[i].abs().total_cmp(&a[j].abs()))?;
    let mut seed = [0.; 3];
    seed[k] = 1.;
    let u = normalize(cross(a, seed))?;
    Some((u, cross(a, u)))
}
#[derive(Clone)]
struct Cylinder {
    c: Vec3,
    a: Vec3,
    r: f64,
}
fn residual(p: Vec3, s: &Cylinder) -> f64 {
    let d = sub(p, s.c);
    length(sub(d, scale(s.a, dot(d, s.a)))) - s.r
}
fn rms(points: &[Vec3], kept: &[usize], s: &Cylinder) -> f64 {
    (kept
        .iter()
        .map(|&i| residual(points[i], s).powi(2))
        .sum::<f64>()
        / kept.len() as f64)
        .sqrt()
}
/// Fits a cylinder to selected positions using optional corresponding per-vertex normals.
pub fn fit_cylinder(
    positions: &[[f32; 3]],
    normals: Option<&[[f32; 3]]>,
    selection: &[u32],
    options: CylinderOptions,
) -> Result<CylinderFit, FitError> {
    let lim = options.limits;
    if selection.len() < 6 {
        return Err(FitError::TooFewPoints {
            found: selection.len(),
            needed: 6,
        });
    }
    if selection.len() > lim.max_points
        || lim.max_iterations == 0
        || lim.max_rounds == 0
        || lim.max_starts == 0
    {
        return Err(FitError::Degenerate(
            "cylinder fit work limit exceeded or zero iteration/start limit",
        ));
    }
    if options
        .assumed_diameter
        .is_some_and(|d| !d.is_finite() || d <= 0.)
        || [options.extend_before, options.extend_after]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.)
    {
        return Err(FitError::Degenerate(
            "assumed diameter must be positive and end extensions nonnegative finite numbers",
        ));
    }
    let k = match options.clip {
        Clip::None => None,
        Clip::Sigma(k @ 1..=3) => Some(f64::from(k)),
        Clip::Sigma(k) => return Err(FitError::BadClip(k)),
    };
    let points = super::gather(positions, selection)?;
    // Stop clipping once residuals are below the uncertainty of the stored f32 coordinates.
    let quantization = points.iter().flatten().fold(0.0_f64, |m, v| m.max(v.abs()))
        * f64::from(f32::EPSILON)
        * 3.0_f64.sqrt();
    let mut ns = Vec::new();
    if let Some(normals) = normals {
        for &i in selection {
            let n = normals.get(i as usize).ok_or(FitError::Degenerate(
                "normals are missing for a selected vertex",
            ))?;
            if n.iter().any(|v| !v.is_finite()) {
                return Err(FitError::NotFinite);
            }
            ns.push(normalize(n.map(f64::from)));
        }
    }
    let mut kept: Vec<_> = (0..points.len()).collect();
    let mut s = solve(&points, &ns, &kept, lim)?;
    let mut rounds = 1;
    if let Some(k) = k {
        while rounds < lim.max_rounds {
            let sigma = rms(&points, &kept, &s);
            if sigma <= quantization {
                break;
            }
            let next: Vec<_> = (0..points.len())
                .filter(|&i| residual(points[i], &s).abs() <= k * sigma)
                .collect();
            if next == kept || next.len() < 6 {
                break;
            }
            let Ok(n) = solve(&points, &ns, &next, lim) else {
                break;
            };
            s = n;
            kept = next;
            rounds += 1;
        }
    }
    let axis_index = (0..3)
        .max_by(|&i, &j| s.a[i].abs().total_cmp(&s.a[j].abs()))
        .ok_or(NO_AXIS)?;
    if s.a[axis_index] < 0. {
        s.a = scale(s.a, -1.);
    }
    let (u, v) = basis(s.a).ok_or(NO_AXIS)?;
    let (mut lo, mut hi, mut min, mut max) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    let mut angles = Vec::new();
    for &i in &kept {
        let d = sub(points[i], s.c);
        let z = dot(d, s.a);
        lo = lo.min(z);
        hi = hi.max(z);
        angles.push(dot(d, v).atan2(dot(d, u)));
        let r = residual(points[i], &s);
        min = min.min(r);
        max = max.max(r);
    }
    angles.sort_by(f64::total_cmp);
    let mut gap = angles[0] + std::f64::consts::TAU - angles[angles.len() - 1];
    for pair in angles.windows(2) {
        gap = gap.max(pair[1] - pair[0]);
    }
    if hi - lo < s.r * 1e-8 {
        return Err(NO_AXIS);
    }
    Ok(CylinderFit {
        axis_point: add(s.c, scale(s.a, (lo + hi) / 2.)),
        axis: s.a,
        radius: s.r,
        diameter: 2. * s.r,
        length: hi - lo,
        coverage_deg: (std::f64::consts::TAU - gap).to_degrees(),
        sigma: rms(&points, &kept, &s),
        form_error: max - min,
        min_residual: min,
        max_residual: max,
        used: kept.len(),
        offered: selection.len(),
        rounds,
        assumed_diameter: options.assumed_diameter,
        extend_before: options.extend_before,
        extend_after: options.extend_after,
    })
}
fn solve(
    points: &[Vec3],
    normals: &[Option<Vec3>],
    kept: &[usize],
    lim: CylinderLimits,
) -> Result<Cylinder, FitError> {
    let cov = covariance(kept.iter().map(|&i| points[i])).ok_or(NO_AXIS)?;
    let eigen = symmetric_eigen3(cov.matrix).ok_or(NO_AXIS)?;
    if eigen.values[2] <= 0. || eigen.values[0] <= 1e-10 * eigen.values[2] {
        return Err(NO_AXIS);
    }
    let mut axes = Vec::new();
    let ns: Vec<_> = kept
        .iter()
        .filter_map(|&i| normals.get(i).copied().flatten())
        .collect();
    if let Some(ncov) = covariance(ns.iter().copied()) {
        if let Some(e) = symmetric_eigen3(ncov.matrix) {
            if e.values[1] > 1e-6 * e.values[2] {
                axes.push(e.vectors[0]);
            }
        }
    }
    // Pair samples provide a deterministic fallback when covariance is unstable.
    for i in 0..ns.len().min(16) {
        let a = ns[i * ns.len() / ns.len().min(16)];
        let b = ns[((i + 7) % ns.len().min(16)) * ns.len() / ns.len().min(16)];
        let c = cross(a, b);
        if length(c) > 1e-4 {
            if let Some(axis) = normalize(c) {
                axes.push(axis);
            }
        }
    }
    axes.extend(eigen.vectors);
    let o = cov.centroid;
    let local: Vec<_> = kept.iter().map(|&i| sub(points[i], o)).collect();
    let mut best: Option<(f64, Vec3, Vec3, Vec3, Vec<f64>)> = None;
    for a in axes.into_iter().take(lim.max_starts) {
        let Some((u, v)) = basis(a) else {
            continue;
        };
        let mut ata = vec![0.; 9];
        let mut atb = vec![0.; 3];
        for &p in &local {
            let x = dot(p, u);
            let y = dot(p, v);
            let row = [2. * x, 2. * y, 1.];
            let rhs = x * x + y * y;
            for i in 0..3 {
                atb[i] += row[i] * rhs;
                for j in 0..3 {
                    ata[i * 3 + j] += row[i] * row[j];
                }
            }
        }
        let Some(x) = solve_dense(ata, atb) else {
            continue;
        };
        let r = (x[2] + x[0] * x[0] + x[1] * x[1]).sqrt();
        if !r.is_finite() || r <= 0. {
            continue;
        }
        let s = Cylinder {
            c: add(scale(u, x[0]), scale(v, x[1])),
            a,
            r,
        };
        let cost = local.iter().map(|&p| residual(p, &s).powi(2)).sum::<f64>();
        if best.as_ref().is_none_or(|b| cost < b.0) {
            best = Some((cost, a, u, v, vec![0., 0., x[0], x[1], r]));
        }
    }
    let (_, a, u, v, start) = best.ok_or(NO_AXIS)?;
    let report = levenberg_marquardt(
        &start,
        local.len(),
        LmOptions {
            max_iterations: lim.max_iterations,
            ..Default::default()
        },
        |x, res, jac| {
            let raw = add(a, add(scale(u, x[0]), scale(v, x[1])));
            let len = length(raw);
            let Some(axis) = normalize(raw) else {
                return false;
            };
            if !x[4].is_finite() || x[4] <= 0. {
                return false;
            }
            let center = add(scale(u, x[2]), scale(v, x[3]));
            let mut jac = jac;
            let du = scale(sub(u, scale(axis, dot(axis, u))), 1. / len);
            let dv = scale(sub(v, scale(axis, dot(axis, v))), 1. / len);
            for (i, &p) in local.iter().enumerate() {
                let d = sub(p, center);
                let h = dot(d, axis);
                let q = sub(d, scale(axis, h));
                let rho = length(q);
                if rho == 0. || !rho.is_finite() {
                    return false;
                }
                res[i] = rho - x[4];
                if let Some(j) = jac.as_deref_mut() {
                    j[i * 5] = -h * dot(d, du) / rho;
                    j[i * 5 + 1] = -h * dot(d, dv) / rho;
                    j[i * 5 + 2] = -dot(q, u) / rho;
                    j[i * 5 + 3] = -dot(q, v) / rho;
                    j[i * 5 + 4] = -1.;
                }
            }
            true
        },
    )
    .ok_or(FitError::Degenerate("the cylinder fit did not converge"))?;
    if !report.converged {
        return Err(FitError::Degenerate(
            "the cylinder fit reached its iteration limit or stalled",
        ));
    }
    let x = report.x;
    let axis = normalize(add(a, add(scale(u, x[0]), scale(v, x[1])))).ok_or(NO_AXIS)?;
    let center = add(o, add(scale(u, x[2]), scale(v, x[3])));
    if !x[4].is_finite() || x[4] <= 0. || center.iter().any(|v| !v.is_finite()) {
        return Err(FitError::NotFinite);
    }
    Ok(Cylinder {
        a: axis,
        c: center,
        r: x[4],
    })
}
