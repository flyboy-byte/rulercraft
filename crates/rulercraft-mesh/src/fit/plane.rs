// SPDX-License-Identifier: MIT OR Apache-2.0
//! Plane fit by total least squares with k-sigma outlier clipping (docs/specs/fit-plane.md).
//!
//! The best plane through a point set passes through its centroid with the normal along the
//! direction of least spread: the smallest-eigenvalue eigenvector of the covariance.

use crate::math::{Vec3, covariance, cross, dot, symmetric_eigen3};
use std::fmt;

/// Outlier cut-off: keep points within `k` standard deviations of the plane, or all of them.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Clip {
    /// Use every offered point.
    None,
    /// Keep points within k·sigma, k in 1..=3.
    Sigma(u8),
}

/// A fitted plane. Lengths in mm.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlaneFit {
    /// Centroid of the kept points.
    pub center: Vec3,
    /// Unit normal.
    pub normal: Vec3,
    /// Unit in-plane direction of greatest spread.
    pub basis_u: Vec3,
    /// `normal × basis_u`.
    pub basis_v: Vec3,
    /// Extent of the kept points along `basis_u` (max − min).
    pub span_u: f64,
    /// Extent of the kept points along `basis_v`.
    pub span_v: f64,
    /// RMS signed distance of the kept points from the plane.
    pub sigma: f64,
    /// Max minus min signed distance of the kept points.
    pub form_error: f64,
    /// Most negative signed distance.
    pub min_distance: f64,
    /// Most positive signed distance.
    pub max_distance: f64,
    /// Points kept by the clipping.
    pub used: usize,
    /// Points offered.
    pub offered: usize,
    /// Clipping rounds taken (1 when nothing was clipped).
    pub rounds: usize,
}

/// Why a fit failed. `Display` gives a message a user can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FitError {
    /// Fewer points than the element needs.
    TooFewPoints {
        /// Points offered.
        found: usize,
        /// Points needed.
        needed: usize,
    },
    /// A vertex index past the end of the mesh.
    NoSuchVertex {
        /// The index.
        index: u32,
        /// Vertices in the mesh.
        count: usize,
    },
    /// A position or normal is NaN or infinite.
    NotFinite,
    /// The points do not determine the element (all on a line, all at one point, ...).
    Degenerate(&'static str),
    /// `Clip::Sigma` with k outside 1..=3.
    BadClip(u8),
}

impl fmt::Display for FitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FitError::TooFewPoints { found, needed } => {
                write!(f, "{found} points marked; this fit needs at least {needed}")
            }
            FitError::NoSuchVertex { index, count } => {
                write!(
                    f,
                    "vertex {index} does not exist (the mesh has {count} vertices)"
                )
            }
            FitError::NotFinite => write!(
                f,
                "a marked point has a coordinate that is not a finite number"
            ),
            FitError::Degenerate(why) => write!(f, "{why}"),
            FitError::BadClip(k) => write!(f, "outlier cut-off must be 1, 2 or 3 sigma, not {k}"),
        }
    }
}

impl std::error::Error for FitError {}

const MAX_ROUNDS: usize = 10;

/// Fits a plane to the `selection` of `positions`. `normals`, when given (one per vertex), orient the
/// result toward the side the selected vertices face.
pub fn fit_plane(
    positions: &[[f32; 3]],
    normals: Option<&[[f32; 3]]>,
    selection: &[u32],
    clip: Clip,
) -> Result<PlaneFit, FitError> {
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
    let mut points = Vec::with_capacity(selection.len());
    for &i in selection {
        let p = positions.get(i as usize).ok_or(FitError::NoSuchVertex {
            index: i,
            count: positions.len(),
        })?;
        if !p.iter().all(|v| v.is_finite()) {
            return Err(FitError::NotFinite);
        }
        points.push(p.map(f64::from));
    }

    let mut kept: Vec<usize> = (0..points.len()).collect();
    let mut plane = solve(&points, &kept)?;
    let mut rounds = 1;
    if let Some(k) = k {
        while rounds < MAX_ROUNDS {
            let sigma = rms(&points, &kept, &plane);
            if sigma == 0.0 {
                break;
            }
            let next: Vec<usize> = (0..points.len())
                .filter(|&i| distance(&points[i], &plane).abs() <= k * sigma)
                .collect();
            if next == kept || next.len() < 3 {
                break;
            }
            match solve(&points, &next) {
                Ok(p) => {
                    plane = p;
                    kept = next;
                    rounds += 1;
                }
                Err(_) => break,
            }
        }
    }

    let mut normal = plane.normal;
    let mut basis_u = plane.basis_u;
    if let Some(normals) = normals {
        let mut facing = 0.0;
        for &i in selection {
            let n = normals.get(i as usize).ok_or(FitError::NoSuchVertex {
                index: i,
                count: normals.len(),
            })?;
            if !n.iter().all(|v| v.is_finite()) {
                return Err(FitError::NotFinite);
            }
            facing += dot(normal, n.map(f64::from));
        }
        if facing < 0.0 {
            normal = normal.map(|x| -x);
            basis_u = basis_u.map(|x| -x);
        }
    }
    let plane = Plane {
        center: plane.center,
        normal,
        basis_u,
    };
    let basis_v = cross(normal, basis_u);

    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut ulo, mut uhi, mut vlo, mut vhi) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for &i in &kept {
        let d = distance(&points[i], &plane);
        lo = lo.min(d);
        hi = hi.max(d);
        let r = sub(points[i], plane.center);
        let (u, v) = (dot(r, basis_u), dot(r, basis_v));
        ulo = ulo.min(u);
        uhi = uhi.max(u);
        vlo = vlo.min(v);
        vhi = vhi.max(v);
    }
    Ok(PlaneFit {
        center: plane.center,
        normal,
        basis_u,
        basis_v,
        span_u: uhi - ulo,
        span_v: vhi - vlo,
        sigma: rms(&points, &kept, &plane),
        form_error: hi - lo,
        min_distance: lo,
        max_distance: hi,
        used: kept.len(),
        offered: selection.len(),
        rounds,
    })
}

struct Plane {
    center: Vec3,
    normal: Vec3,
    basis_u: Vec3,
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn distance(p: &Vec3, plane: &Plane) -> f64 {
    dot(sub(*p, plane.center), plane.normal)
}

fn rms(points: &[Vec3], kept: &[usize], plane: &Plane) -> f64 {
    let sum: f64 = kept
        .iter()
        .map(|&i| distance(&points[i], plane).powi(2))
        .sum();
    (sum / kept.len() as f64).sqrt()
}

fn solve(points: &[Vec3], kept: &[usize]) -> Result<Plane, FitError> {
    let c = covariance(kept.iter().map(|&i| points[i])).ok_or(FitError::TooFewPoints {
        found: 0,
        needed: 3,
    })?;
    let e = symmetric_eigen3(c.matrix).ok_or(FitError::NotFinite)?;
    if e.values[2] == 0.0 || e.values[1] <= 1e-12 * e.values[2] {
        return Err(FitError::Degenerate(
            "the marked points lie on one line or at one point, so no single plane fits them",
        ));
    }
    Ok(Plane {
        center: c.centroid,
        normal: e.vectors[0],
        basis_u: e.vectors[2],
    })
}
