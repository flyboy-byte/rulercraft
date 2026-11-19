// SPDX-License-Identifier: MIT OR Apache-2.0
//! Cone fit (docs/specs/fit-shapes.md): a few axis starts, then geometric least squares on the distance of each point
//! from the cone's surface, with clipping as for the other fits.
use super::{Clip, FitError};
use crate::math::{
    LmOptions, Vec3, add, covariance, cross, dot, length, levenberg_marquardt, normalize, scale,
    sub, symmetric_eigen3,
};

/// A measured cone, lengths in millimetres, angles in degrees.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ConeFit {
    /// Point on the axis at the middle of the measured axial span.
    pub axis_point: Vec3,
    /// Unit axis pointing from the narrow end to the wide end.
    pub axis: Vec3,
    /// Half of the cone's opening angle (the angle between its surface and its axis).
    pub half_angle_deg: f64,
    /// Radius at `axis_point`.
    pub radius: f64,
    /// Diameter at the narrow end of the measured span.
    pub diameter_small: f64,
    /// Diameter at the wide end of the measured span.
    pub diameter_large: f64,
    /// The apex, where the surface meets the axis.
    pub apex: Vec3,
    /// Axial extent of the kept points.
    pub length: f64,
    /// RMS distance of the kept points from the surface.
    pub sigma: f64,
    /// Maximum minus minimum signed distance.
    pub form_error: f64,
    /// Most negative signed distance (inside).
    pub min_residual: f64,
    /// Most positive signed distance (outside).
    pub max_residual: f64,
    /// Points kept.
    pub used: usize,
    /// Points offered.
    pub offered: usize,
    /// Fits including clipping rounds.
    pub rounds: usize,
}

/// The cone's parameters: a point on the axis, the unit axis, the radius at that point and the slope of the radius
/// along the axis (tan of the half-angle).
#[derive(Debug, Clone, Copy)]
struct Cone {
    p: Vec3,
    d: Vec3,
    r: f64,
    t: f64,
}

/// Signed distance from the surface, outside positive.
fn residual(q: Vec3, c: &Cone) -> f64 {
    let w = sub(q, c.p);
    let h = dot(w, c.d);
    let rho = length(sub(w, scale(c.d, h)));
    (rho - (c.r + c.t * h)) / (1.0 + c.t * c.t).sqrt()
}

fn basis(a: Vec3) -> Option<(Vec3, Vec3)> {
    let k = (0..3).min_by(|&i, &j| a[i].abs().total_cmp(&a[j].abs()))?;
    let mut seed = [0.0; 3];
    seed[k] = 1.0;
    let u = normalize(cross(a, seed))?;
    Some((u, cross(a, u)))
}

/// A starting cone for an axis direction: through the centroid, the radius a straight line in height.
fn start(points: &[Vec3], d: Vec3) -> Option<Cone> {
    let n = points.len() as f64;
    let c = scale(points.iter().fold([0.0; 3], |a, &p| add(a, p)), 1.0 / n);
    // The axis point across the axis: where the points' circle in the cross-section plane is centred (a cone's
    // points projected along its axis fill an annulus round that centre, so their mean is it, near enough).
    let hs: Vec<(f64, f64)> = points
        .iter()
        .map(|&q| {
            let w = sub(q, c);
            let h = dot(w, d);
            (h, length(sub(w, scale(d, h))))
        })
        .collect();
    let (sh, sr) = hs.iter().fold((0.0, 0.0), |(a, b), &(h, r)| (a + h, b + r));
    let (mh, mr) = (sh / n, sr / n);
    let (num, den) = hs.iter().fold((0.0, 0.0), |(a, b), &(h, r)| {
        (a + (h - mh) * (r - mr), b + (h - mh) * (h - mh))
    });
    let t = if den > 0.0 { num / den } else { 0.0 };
    (mr > 0.0 && t.is_finite()).then_some(Cone {
        p: add(c, scale(d, mh)),
        d,
        r: mr,
        t,
    })
}

/// Geometric refinement of all six parameters from a start.
fn refine(points: &[Vec3], s: Cone, max_iterations: usize) -> Option<(Cone, f64)> {
    let (e1, e2) = basis(s.d)?;
    let build = |x: &[f64]| -> Option<Cone> {
        let d = normalize(add(s.d, add(scale(e1, x[0]), scale(e2, x[1]))))?;
        let p = add(s.p, add(scale(e1, x[2]), scale(e2, x[3])));
        Some(Cone {
            p,
            d,
            r: x[4],
            t: x[5],
        })
    };
    let eval = |x: &[f64], r: &mut [f64], jac: Option<&mut [f64]>| -> bool {
        let Some(c) = build(x) else { return false };
        for (k, &q) in points.iter().enumerate() {
            r[k] = residual(q, &c);
        }
        if let Some(j) = jac {
            let n = x.len();
            for col in 0..n {
                let mut y = x.to_vec();
                let step = 1e-7 * (1.0 + x[col].abs());
                y[col] += step;
                let Some(c2) = build(&y) else { return false };
                for (k, &q) in points.iter().enumerate() {
                    j[k * n + col] = (residual(q, &c2) - r[k]) / step;
                }
            }
        }
        r.iter().all(|v| v.is_finite())
    };
    let opts = LmOptions {
        max_iterations,
        ..LmOptions::default()
    };
    let report = levenberg_marquardt(&[0.0, 0.0, 0.0, 0.0, s.r, s.t], points.len(), opts, eval)?;
    Some((build(&report.x)?, report.cost))
}

/// Starting axis directions: across the normals when given (a cone's normals all make one angle with its axis, so the
/// axis is the direction they vary least along), then the points' three principal directions.
fn axis_starts(points: &[Vec3], normals: &[Vec3]) -> Vec<Vec3> {
    let mut out = Vec::new();
    if normals.len() >= 6 {
        if let Some(e) =
            covariance(normals.iter().copied()).and_then(|c| symmetric_eigen3(c.matrix))
        {
            out.push(e.vectors[0]);
        }
    }
    if let Some(e) = covariance(points.iter().copied()).and_then(|c| symmetric_eigen3(c.matrix)) {
        out.extend(e.vectors.iter().rev().copied());
    }
    out
}

/// Fits a cone to the `selection` of `positions`. `normals`, when given (one per vertex), help find the axis; zero
/// normals are ignored.
pub fn fit_cone(
    positions: &[[f32; 3]],
    normals: Option<&[[f32; 3]]>,
    selection: &[u32],
    clip: Clip,
) -> Result<ConeFit, FitError> {
    if selection.len() < 7 {
        return Err(FitError::TooFewPoints {
            found: selection.len(),
            needed: 7,
        });
    }
    let k = match clip {
        Clip::None => None,
        Clip::Sigma(k @ 1..=3) => Some(f64::from(k)),
        Clip::Sigma(k) => return Err(FitError::BadClip(k)),
    };
    let mut points = Vec::with_capacity(selection.len());
    let mut ns = Vec::new();
    for &i in selection {
        let p = positions.get(i as usize).ok_or(FitError::NoSuchVertex {
            index: i,
            count: positions.len(),
        })?;
        if p.iter().any(|v| !v.is_finite()) {
            return Err(FitError::NotFinite);
        }
        points.push(p.map(f64::from));
        if let Some(n) = normals.and_then(|n| n.get(i as usize)) {
            let n = n.map(f64::from);
            if n.iter().all(|v| v.is_finite()) && length(n) > 0.5 {
                ns.push(n);
            }
        }
    }
    const NOT_A_CONE: FitError = FitError::Degenerate("the points do not determine a cone");
    let fit_all = |pts: &[Vec3]| -> Option<Cone> {
        axis_starts(pts, &ns)
            .into_iter()
            .filter_map(|d| start(pts, d))
            .filter_map(|s| refine(pts, s, 200))
            .filter(|(c, _)| c.r.is_finite() && c.t.is_finite())
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(c, _)| c)
    };
    let mut cone = fit_all(&points).ok_or(NOT_A_CONE)?;
    let mut kept: Vec<Vec3> = points.clone();
    let mut rounds = 1;
    if let Some(k) = k {
        // The cut-off is fixed from the fit of every point, so it cannot creep inward (docs/specs/fit-plane.md).
        let rms = (points
            .iter()
            .map(|&q| residual(q, &cone).powi(2))
            .sum::<f64>()
            / points.len() as f64)
            .sqrt();
        let cut = k * rms;
        while rounds < 10 && cut > 0.0 {
            let next: Vec<Vec3> = points
                .iter()
                .copied()
                .filter(|&q| residual(q, &cone).abs() <= cut)
                .collect();
            if next.len() == kept.len() || next.len() < 7 {
                break;
            }
            kept = next;
            cone = refine(&kept, cone, 200).map(|(c, _)| c).ok_or(NOT_A_CONE)?;
            rounds += 1;
        }
    }
    // A cone that hardly tapers is a cylinder; one that is almost flat is a plane.
    let half = cone.t.abs().atan().to_degrees();
    if !(0.5..=85.0).contains(&half) {
        return Err(FitError::Degenerate(if half < 0.5 {
            "the surface hardly tapers: fit a cylinder"
        } else {
            "the surface is almost flat: fit a plane"
        }));
    }
    // Point the axis toward the wide end, and report at the middle of the kept span.
    if cone.t < 0.0 {
        cone = Cone {
            d: scale(cone.d, -1.0),
            t: -cone.t,
            ..cone
        };
    }
    let hs: Vec<f64> = kept.iter().map(|&q| dot(sub(q, cone.p), cone.d)).collect();
    let (h0, h1) = hs
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &h| {
            (a.min(h), b.max(h))
        });
    let mid = (h0 + h1) / 2.0;
    let res: Vec<f64> = kept.iter().map(|&q| residual(q, &cone)).collect();
    let (lo, hi) = res
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
            (a.min(v), b.max(v))
        });
    let radius = cone.r + cone.t * mid;
    let out = ConeFit {
        axis_point: add(cone.p, scale(cone.d, mid)),
        axis: cone.d,
        half_angle_deg: half,
        radius,
        diameter_small: 2.0 * (cone.r + cone.t * h0),
        diameter_large: 2.0 * (cone.r + cone.t * h1),
        apex: add(cone.p, scale(cone.d, -cone.r / cone.t)),
        length: h1 - h0,
        sigma: (res.iter().map(|v| v * v).sum::<f64>() / res.len() as f64).sqrt(),
        form_error: hi - lo,
        min_residual: lo,
        max_residual: hi,
        used: kept.len(),
        offered: points.len(),
        rounds,
    };
    let finite = [out.radius, out.half_angle_deg, out.sigma, out.length]
        .iter()
        .chain(out.axis_point.iter())
        .chain(out.apex.iter())
        .all(|v| v.is_finite());
    if !finite || out.diameter_small < 0.0 || radius <= 0.0 {
        return Err(NOT_A_CONE);
    }
    Ok(out)
}
