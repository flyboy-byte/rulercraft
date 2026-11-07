// SPDX-License-Identifier: MIT OR Apache-2.0
//! Marking the vertices a fit runs on (docs/specs/selection.md).

use crate::math::{Vec3, dot, normalize};
use std::fmt;

/// Why a selection could not be made. `Display` gives a message a user can act on.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectError {
    /// A box whose minimum is past its maximum on an axis (0 = x, 1 = y, 2 = z).
    InvertedBox(usize),
    /// A radius that is negative or not finite.
    BadRadius,
    /// A facing direction of zero length or with a non-finite component.
    ZeroDirection,
    /// A facing angle outside (0°, 180°].
    BadAngle,
    /// A box corner or centre with a non-finite coordinate.
    NotFinite,
    /// An explicit vertex index past the end of the mesh.
    NoSuchVertex {
        /// The index.
        index: u32,
        /// Vertices in the mesh.
        count: usize,
    },
}

impl fmt::Display for SelectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectError::InvertedBox(axis) => {
                write!(
                    f,
                    "the box's minimum is past its maximum on {}",
                    ["x", "y", "z"][*axis]
                )
            }
            SelectError::BadRadius => write!(f, "the radius must be zero or a positive number"),
            SelectError::ZeroDirection => {
                write!(f, "the facing direction must be a non-zero vector")
            }
            SelectError::BadAngle => {
                write!(f, "the facing angle must be more than 0° and at most 180°")
            }
            SelectError::NotFinite => write!(
                f,
                "the box or point has a coordinate that is not a finite number"
            ),
            SelectError::NoSuchVertex { index, count } => {
                write!(
                    f,
                    "vertex {index} does not exist (the mesh has {count} vertices)"
                )
            }
        }
    }
}

impl std::error::Error for SelectError {}

fn finite(v: Vec3) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// Every vertex inside the axis-aligned box `min..=max` (mm, inclusive), ascending.
pub fn in_box(positions: &[[f32; 3]], min: Vec3, max: Vec3) -> Result<Vec<u32>, SelectError> {
    if !finite(min) || !finite(max) {
        return Err(SelectError::NotFinite);
    }
    if let Some(axis) = (0..3).find(|&k| min[k] > max[k]) {
        return Err(SelectError::InvertedBox(axis));
    }
    Ok((0..positions.len() as u32)
        .filter(|&i| {
            let p = positions[i as usize];
            (0..3).all(|k| (min[k]..=max[k]).contains(&f64::from(p[k])))
        })
        .collect())
}

/// Every vertex within `radius` mm of `center` (inclusive), ascending.
pub fn near(positions: &[[f32; 3]], center: Vec3, radius: f64) -> Result<Vec<u32>, SelectError> {
    if !finite(center) {
        return Err(SelectError::NotFinite);
    }
    if !(radius.is_finite() && radius >= 0.0) {
        return Err(SelectError::BadRadius);
    }
    let r2 = radius * radius;
    Ok((0..positions.len() as u32)
        .filter(|&i| {
            let p = positions[i as usize].map(f64::from);
            let d = [p[0] - center[0], p[1] - center[1], p[2] - center[2]];
            dot(d, d) <= r2
        })
        .collect())
}

/// The vertices of `selection` whose normal is within `within` radians of `direction` (any non-zero
/// length). Vertices without a normal (zero vector) are dropped.
pub fn facing(
    selection: &[u32],
    normals: &[[f32; 3]],
    direction: Vec3,
    within: f64,
) -> Result<Vec<u32>, SelectError> {
    let d = (finite(direction))
        .then(|| normalize(direction))
        .flatten()
        .ok_or(SelectError::ZeroDirection)?;
    if !(within > 0.0 && within <= std::f64::consts::PI) {
        return Err(SelectError::BadAngle);
    }
    let limit = within.cos();
    let mut out = Vec::with_capacity(selection.len());
    for &i in selection {
        let n = normals.get(i as usize).ok_or(SelectError::NoSuchVertex {
            index: i,
            count: normals.len(),
        })?;
        let n = n.map(f64::from);
        if let Some(n) = normalize(n)
            && dot(n, d) >= limit - 1e-12
        {
            out.push(i);
        }
    }
    Ok(out)
}

/// An explicit list of vertex indices, checked against the mesh size, sorted and without repeats.
pub fn explicit(count: usize, indices: &[u32]) -> Result<Vec<u32>, SelectError> {
    if let Some(&index) = indices.iter().find(|&&i| i as usize >= count) {
        return Err(SelectError::NoSuchVertex { index, count });
    }
    let mut v = indices.to_vec();
    v.sort_unstable();
    v.dedup();
    Ok(v)
}

/// At most `cap` of `selection`, evenly spread (every `step`-th), with the step, so a large selection
/// can be shown to an agent without flooding it. `cap` of 0 returns nothing with step 0.
pub fn thinned(selection: &[u32], cap: usize) -> (Vec<u32>, usize) {
    if cap == 0 {
        return (Vec::new(), 0);
    }
    let step = selection.len().div_ceil(cap).max(1);
    (selection.iter().copied().step_by(step).collect(), step)
}
