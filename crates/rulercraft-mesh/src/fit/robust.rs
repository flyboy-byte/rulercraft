// SPDX-License-Identifier: MIT OR Apache-2.0
//! Robust plane fit (docs/specs/fit-plane.md, "Robust fit"): random three-point planes are scored by how
//! many selected points lie within a distance threshold; the largest consensus set is then fitted by the
//! ordinary total least squares fit. Far outliers cannot tilt it. The sampling is a fixed pseudo-random
//! sequence from `seed`, so the same input always gives the same answer.
use super::{FitError, PlaneFit, fit_plane, gather};
use crate::fit::Clip;
use crate::math::{cross, dot, length, sub};

/// Most trials a call may ask for.
pub const MAX_ITERATIONS: usize = 100_000;

/// Options for [`fit_plane_robust`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RobustOptions {
    /// A point counts as agreeing when it is within this distance of a trial plane, mm.
    pub threshold: f64,
    /// Number of trial planes, 1 to [`MAX_ITERATIONS`].
    pub iterations: usize,
    /// Seed of the sampling sequence.
    pub seed: u64,
}
impl Default for RobustOptions {
    fn default() -> Self {
        Self {
            threshold: 0.1,
            iterations: 500,
            seed: 1,
        }
    }
}

/// A robust plane and the vertices it agreed with.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustPlane {
    /// Total least squares plane through the agreeing vertices.
    pub fit: PlaneFit,
    /// The agreeing vertex ids, in selection order.
    pub inliers: Vec<u32>,
}

fn next(s: &mut u64) -> u64 {
    *s = s
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *s >> 33
}

/// Fits a plane to `selection` ignoring far outliers. Refuses a threshold that is not a positive finite
/// number, zero or too many iterations, fewer than 3 points, and a selection where no three points make
/// a plane that at least 3 points agree with.
pub fn fit_plane_robust(
    positions: &[[f32; 3]],
    normals: Option<&[[f32; 3]]>,
    selection: &[u32],
    options: RobustOptions,
) -> Result<RobustPlane, FitError> {
    if !(options.threshold.is_finite() && options.threshold > 0.0)
        || options.iterations == 0
        || options.iterations > MAX_ITERATIONS
    {
        return Err(FitError::Degenerate(
            "robust fit needs a positive threshold and 1 to 100000 iterations",
        ));
    }
    let pts = gather(positions, selection)?;
    let n = pts.len();
    if n < 3 {
        return Err(FitError::TooFewPoints {
            found: n,
            needed: 3,
        });
    }
    let mut seed = options.seed;
    let mut best: Vec<usize> = vec![];
    for _ in 0..options.iterations {
        let (a, b, c) = (
            next(&mut seed) as usize % n,
            next(&mut seed) as usize % n,
            next(&mut seed) as usize % n,
        );
        if a == b || b == c || a == c {
            continue;
        }
        let nv = cross(sub(pts[b], pts[a]), sub(pts[c], pts[a]));
        let len = length(nv);
        if len < 1e-12 {
            continue;
        }
        let inl: Vec<usize> = (0..n)
            .filter(|&i| (dot(sub(pts[i], pts[a]), nv) / len).abs() <= options.threshold)
            .collect();
        if inl.len() > best.len() {
            best = inl;
        }
    }
    if best.len() < 3 {
        return Err(FitError::Degenerate(
            "no plane that at least 3 selected points agree with",
        ));
    }
    let inliers: Vec<u32> = best.iter().map(|&i| selection[i]).collect();
    let fit = fit_plane(positions, normals, &inliers, Clip::None)?;
    Ok(RobustPlane { fit, inliers })
}

/// A robust sphere and the vertices it agreed with.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustSphere {
    /// Fit through the agreeing vertices.
    pub fit: super::SphereFit,
    /// The agreeing vertex ids, in selection order.
    pub inliers: Vec<u32>,
}

/// A robust cylinder and the vertices it agreed with.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustCylinder {
    /// Fit through the agreeing vertices.
    pub fit: super::CylinderFit,
    /// The agreeing vertex ids, in selection order.
    pub inliers: Vec<u32>,
}

fn check_options(o: &RobustOptions) -> Result<(), FitError> {
    if !(o.threshold.is_finite() && o.threshold > 0.0)
        || o.iterations == 0
        || o.iterations > MAX_ITERATIONS
    {
        return Err(FitError::Degenerate(
            "robust fit needs a positive threshold and 1 to 100000 iterations",
        ));
    }
    Ok(())
}

/// Random subset of `k` distinct selection indices.
fn subset(seed: &mut u64, n: usize, k: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = vec![];
    while idx.len() < k {
        let i = next(seed) as usize % n;
        if !idx.contains(&i) {
            idx.push(i);
        }
    }
    idx
}

/// Fits a sphere to `selection` ignoring far outliers: trial spheres are fitted to random 6-point subsets
/// and scored by how many selected points lie within `threshold` of the sphere's surface; the best
/// agreeing set is then fitted without clipping. The same rules and refusals as [`fit_plane_robust`];
/// needs at least 6 selected points. A trial that cannot be fitted is skipped.
pub fn fit_sphere_robust(
    positions: &[[f32; 3]],
    selection: &[u32],
    options: RobustOptions,
) -> Result<RobustSphere, FitError> {
    const K: usize = 6;
    check_options(&options)?;
    let pts = gather(positions, selection)?;
    let n = pts.len();
    if n < K {
        return Err(FitError::TooFewPoints {
            found: n,
            needed: K,
        });
    }
    let mut seed = options.seed;
    let mut best: Vec<usize> = vec![];
    for _ in 0..options.iterations {
        let ids: Vec<u32> = subset(&mut seed, n, K)
            .iter()
            .map(|&i| selection[i])
            .collect();
        let Ok(t) = super::fit_sphere(positions, &ids, Clip::None, None) else {
            continue;
        };
        let inl: Vec<usize> = (0..n)
            .filter(|&i| (length(sub(pts[i], t.center)) - t.radius).abs() <= options.threshold)
            .collect();
        if inl.len() > best.len() {
            best = inl;
        }
    }
    if best.len() < K {
        return Err(FitError::Degenerate(
            "no sphere that at least 6 selected points agree with",
        ));
    }
    let inliers: Vec<u32> = best.iter().map(|&i| selection[i]).collect();
    let fit = super::fit_sphere(positions, &inliers, Clip::None, None)?;
    Ok(RobustSphere { fit, inliers })
}

/// Fits a cylinder to `selection` ignoring far outliers: trial cylinders are fitted to random 12-point
/// subsets and scored by how many selected points lie within `threshold` of the cylinder's surface; the
/// best agreeing set is then fitted without clipping. Needs at least 12 selected points; `iterations`
/// should be small (default options are fine for a few hundred points) because each trial is a full fit.
pub fn fit_cylinder_robust(
    positions: &[[f32; 3]],
    selection: &[u32],
    options: RobustOptions,
) -> Result<RobustCylinder, FitError> {
    const K: usize = 12;
    check_options(&options)?;
    let pts = gather(positions, selection)?;
    let n = pts.len();
    if n < K {
        return Err(FitError::TooFewPoints {
            found: n,
            needed: K,
        });
    }
    let opts = super::CylinderOptions {
        clip: Clip::None,
        ..Default::default()
    };
    let mut seed = options.seed;
    let mut best: Vec<usize> = vec![];
    for _ in 0..options.iterations {
        let ids: Vec<u32> = subset(&mut seed, n, K)
            .iter()
            .map(|&i| selection[i])
            .collect();
        let Ok(t) = super::fit_cylinder(positions, None, &ids, opts) else {
            continue;
        };
        let inl: Vec<usize> = (0..n)
            .filter(|&i| {
                let d = sub(pts[i], t.axis_point);
                let radial = length(sub(d, crate::math::scale(t.axis, dot(d, t.axis))));
                (radial - t.radius).abs() <= options.threshold
            })
            .collect();
        if inl.len() > best.len() {
            best = inl;
        }
    }
    if best.len() < K {
        return Err(FitError::Degenerate(
            "no cylinder that at least 12 selected points agree with",
        ));
    }
    let inliers: Vec<u32> = best.iter().map(|&i| selection[i]).collect();
    let fit = super::fit_cylinder(positions, None, &inliers, opts)?;
    Ok(RobustCylinder { fit, inliers })
}
