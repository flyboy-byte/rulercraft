// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fitting geometric elements to marked vertices (docs/specs/fit-*.md).

pub mod cylinder;
pub mod plane;
pub mod robust;
pub mod sphere;

pub use cylinder::{CylinderFit, CylinderLimits, CylinderOptions, fit_cylinder};
pub use plane::{Clip, FitError, PlaneFit, fit_plane};
pub use robust::{
    RobustCylinder, RobustOptions, RobustPlane, RobustSphere, fit_cylinder_robust,
    fit_plane_robust, fit_sphere_robust,
};
pub use sphere::{SphereFit, fit_sphere};

use crate::math::Vec3;

/// The selected positions as f64 points, refusing missing vertices and non-finite coordinates.
pub(crate) fn gather(positions: &[[f32; 3]], selection: &[u32]) -> Result<Vec<Vec3>, FitError> {
    selection
        .iter()
        .map(|&i| {
            let p = positions.get(i as usize).ok_or(FitError::NoSuchVertex {
                index: i,
                count: positions.len(),
            })?;
            if p.iter().all(|v| v.is_finite()) {
                Ok(p.map(f64::from))
            } else {
                Err(FitError::NotFinite)
            }
        })
        .collect()
}
