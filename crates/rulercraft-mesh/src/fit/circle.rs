// SPDX-License-Identifier: MIT OR Apache-2.0
//! Circle fit in 3D (docs/specs/fit-shapes.md): the plane of the points, then the circle in that plane, refined
//! geometrically, with clipping as for the other fits. For points on a section, an edge or a hole's rim.
use super::{Clip, FitError, fit_plane};
use crate::math::{LmOptions, Vec3, add, dot, levenberg_marquardt, scale, sub};

/// A measured circle, lengths in millimetres.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CircleFit {
    /// Centre.
    pub center: Vec3,
    /// Unit normal of its plane.
    pub normal: Vec3,
    /// Radius.
    pub radius: f64,
    /// Diameter.
    pub diameter: f64,
    /// Angular extent of the kept points, 360 minus the widest gap, degrees.
    pub coverage_deg: f64,
    /// RMS radial distance of the kept points from the circle, in its plane.
    pub sigma: f64,
    /// Maximum minus minimum signed radial distance.
    pub form_error: f64,
    /// Most negative signed radial distance (inside).
    pub min_residual: f64,
    /// Most positive signed radial distance (outside).
    pub max_residual: f64,
    /// RMS distance of the kept points from its plane (how flat the circle is).
    pub flatness_sigma: f64,
    /// Points kept.
    pub used: usize,
    /// Points offered.
    pub offered: usize,
    /// Fits including clipping rounds.
    pub rounds: usize,
}

/// Algebraic circle through 2D points (Kasa), then geometric refinement.
fn circle_2d(p: &[[f64; 2]]) -> Option<(f64, f64, f64)> {
    let n = p.len() as f64;
    let (mx, my) = (
        p.iter().map(|q| q[0]).sum::<f64>() / n,
        p.iter().map(|q| q[1]).sum::<f64>() / n,
    );
    let (mut suu, mut svv, mut suv, mut suuu, mut svvv, mut suvv, mut svuu) =
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for q in p {
        let (u, v) = (q[0] - mx, q[1] - my);
        suu += u * u;
        svv += v * v;
        suv += u * v;
        suuu += u * u * u;
        svvv += v * v * v;
        suvv += u * v * v;
        svuu += v * u * u;
    }
    let det = suu * svv - suv * suv;
    if det.is_nan() || det.abs() <= 1e-18 * (suu + svv).powi(2) {
        return None;
    }
    let (b1, b2) = ((suuu + suvv) / 2.0, (svvv + svuu) / 2.0);
    let (uc, vc) = ((b1 * svv - b2 * suv) / det, (suu * b2 - suv * b1) / det);
    let r0 = (uc * uc + vc * vc + (suu + svv) / n).sqrt();
    let eval = |x: &[f64], r: &mut [f64], jac: Option<&mut [f64]>| -> bool {
        let mut j = jac;
        for (k, q) in p.iter().enumerate() {
            let (dx, dy) = (q[0] - x[0], q[1] - x[1]);
            let d = dx.hypot(dy);
            r[k] = d - x[2];
            if let Some(j) = j.as_deref_mut() {
                if d > 0.0 {
                    j[k * 3] = -dx / d;
                    j[k * 3 + 1] = -dy / d;
                } else {
                    j[k * 3] = 0.0;
                    j[k * 3 + 1] = 0.0;
                }
                j[k * 3 + 2] = -1.0;
            }
        }
        r.iter().all(|v| v.is_finite())
    };
    let report = levenberg_marquardt(&[mx + uc, my + vc, r0], p.len(), LmOptions::default(), eval)?;
    let x = report.x;
    (x[2] > 0.0 && x.iter().all(|v| v.is_finite())).then_some((x[0], x[1], x[2]))
}

/// Fits a circle to the `selection` of `positions`: the points' plane, then the circle in it.
pub fn fit_circle(
    positions: &[[f32; 3]],
    selection: &[u32],
    clip: Clip,
) -> Result<CircleFit, FitError> {
    if selection.len() < 3 {
        return Err(FitError::TooFewPoints {
            found: selection.len(),
            needed: 3,
        });
    }
    let k = match clip {
        Clip::None => None,
        Clip::Sigma(k @ 1..=3) => Some(f64::from(k)),
        Clip::Sigma(k) => return Err(FitError::BadClip(k)),
    };
    let plane = fit_plane(positions, None, selection, Clip::None)?;
    let (o, u, v, nrm) = (plane.center, plane.basis_u, plane.basis_v, plane.normal);
    let flat: Vec<[f64; 2]> = selection
        .iter()
        .map(|&i| {
            let d = sub(positions[i as usize].map(f64::from), o);
            [dot(d, u), dot(d, v)]
        })
        .collect();
    const NOT_A_CIRCLE: FitError =
        FitError::Degenerate("the points do not determine a circle (all on a line?)");
    let (mut cx, mut cy, mut r) = circle_2d(&flat).ok_or(NOT_A_CIRCLE)?;
    let mut kept = flat.clone();
    let mut rounds = 1;
    let off = |q: &[f64; 2], cx: f64, cy: f64, r: f64| (q[0] - cx).hypot(q[1] - cy) - r;
    if let Some(k) = k {
        let rms = (flat.iter().map(|q| off(q, cx, cy, r).powi(2)).sum::<f64>() / flat.len() as f64)
            .sqrt();
        let cut = k * rms;
        while rounds < 10 && cut > 0.0 {
            let next: Vec<[f64; 2]> = flat
                .iter()
                .copied()
                .filter(|q| off(q, cx, cy, r).abs() <= cut)
                .collect();
            if next.len() == kept.len() || next.len() < 3 {
                break;
            }
            kept = next;
            (cx, cy, r) = circle_2d(&kept).ok_or(NOT_A_CIRCLE)?;
            rounds += 1;
        }
    }
    let res: Vec<f64> = kept.iter().map(|q| off(q, cx, cy, r)).collect();
    let (lo, hi) = res
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &x| {
            (a.min(x), b.max(x))
        });
    // Coverage: 360 minus the widest gap between the points' angles, the wrap-around gap included.
    let mut angles: Vec<f64> = kept
        .iter()
        .map(|q| (q[1] - cy).atan2(q[0] - cx).to_degrees())
        .collect();
    angles.sort_by(f64::total_cmp);
    let mut gap = angles
        .first()
        .zip(angles.last())
        .map_or(360.0, |(a, b)| a + 360.0 - b);
    for w in angles.windows(2) {
        gap = gap.max(w[1] - w[0]);
    }
    let center = add(o, add(scale(u, cx), scale(v, cy)));
    Ok(CircleFit {
        center,
        normal: nrm,
        radius: r,
        diameter: 2.0 * r,
        coverage_deg: 360.0 - gap,
        sigma: (res.iter().map(|x| x * x).sum::<f64>() / res.len() as f64).sqrt(),
        form_error: hi - lo,
        min_residual: lo,
        max_residual: hi,
        flatness_sigma: plane.sigma,
        used: kept.len(),
        offered: flat.len(),
        rounds,
    })
}
