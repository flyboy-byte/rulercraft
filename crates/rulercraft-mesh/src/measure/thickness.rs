// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bounded directional wall thickness, not a shortest-distance or solid-classification method.
use crate::Mesh;
use crate::math::{Vec3, cross, dot, normalize, scale, sub};
use crate::spatial::Bvh;
use std::fmt;

/// Limits and tolerances for normal rays; lengths in mm.
#[derive(Debug, Clone, Copy)]
pub struct ThicknessOptions {
    /// Largest selection count times triangle count permitted.
    pub max_triangle_tests: usize,
    /// Furthest accepted wall hit, mm.
    pub max_distance: f64,
    /// Ignore intersections this close to the starting point, mm.
    pub exclusion: f64,
}
impl Default for ThicknessOptions {
    fn default() -> Self {
        Self {
            max_triangle_tests: 5_000_000,
            max_distance: 1_000_000.,
            exclusion: 1e-6,
        }
    }
}
/// One vertex's inward normal-ray measurement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThicknessSample {
    /// Starting vertex index.
    pub vertex: u32,
    /// Nearest opposite outward-facing wall distance, mm; None if no hit within limits.
    pub distance: Option<f64>,
}
/// An actionable input or work-limit refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThicknessError(pub String);
impl fmt::Display for ThicknessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ThicknessError {}
fn error(message: impl ToString) -> ThicknessError {
    ThicknessError(message.to_string())
}

// Solve origin+t*direction = a+u*(b-a)+v*(c-a) with scalar triple products.
fn hit(origin: Vec3, direction: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f64> {
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let normal = normalize(cross(e1, e2))?;
    if dot(normal, direction) <= 1e-12 {
        return None;
    }
    let perpendicular = cross(direction, e2);
    let determinant = dot(e1, perpendicular);
    let from_a = sub(origin, a);
    let u = dot(from_a, perpendicular) / determinant;
    let q = cross(from_a, e1);
    let v = dot(direction, q) / determinant;
    // Only an edge-roundoff allowance, fixed independently of the part size.
    if u < -1e-12 || v < -1e-12 || u + v > 1. + 1e-12 {
        return None;
    }
    let t = dot(e2, q) / determinant;
    t.is_finite().then_some(t)
}
/// Cast inward normal rays to the nearest opposite wall. No topology repair or winding inference.
///
/// Normals correspond one-to-one to positions and point outward. Invalid inputs fail before any
/// result is returned; empty selections return no samples. See docs/specs/measure.md for conventions.
pub fn wall_thickness(
    mesh: &Mesh,
    outward_normals: &[[f32; 3]],
    selection: &[u32],
    options: ThicknessOptions,
) -> Result<Vec<ThicknessSample>, ThicknessError> {
    wall_thickness_indexed(mesh, None, outward_normals, selection, options)
}

/// [`wall_thickness`] with a tree the caller already built for `mesh` (for example once per open mesh,
/// reused across pages). `tree` must have been built from this same `mesh`; with `None` one is built.
/// Fails when the tree's triangle count does not match the mesh.
pub fn wall_thickness_indexed(
    mesh: &Mesh,
    tree: Option<&Bvh>,
    outward_normals: &[[f32; 3]],
    selection: &[u32],
    options: ThicknessOptions,
) -> Result<Vec<ThicknessSample>, ThicknessError> {
    if !options.max_distance.is_finite()
        || !options.exclusion.is_finite()
        || options.exclusion <= 0.
        || options.max_distance <= options.exclusion
    {
        return Err(error(
            "thickness requires finite max distance > exclusion > 0 mm",
        ));
    }
    if selection
        .len()
        .checked_mul(mesh.triangles.len())
        .is_none_or(|n| n > options.max_triangle_tests)
    {
        return Err(error(
            "thickness triangle-test limit exceeded; reduce the selection or raise the explicit limit",
        ));
    }
    if outward_normals.len() != mesh.positions.len() {
        return Err(error("normal count must match mesh vertices"));
    }
    for (i, p) in mesh.positions.iter().enumerate() {
        if p.iter().any(|v| !v.is_finite()) {
            return Err(error(format!("vertex {i}: nonfinite coordinate")));
        }
    }
    for (i, t) in mesh.triangles.iter().enumerate() {
        if t.iter().any(|v| *v as usize >= mesh.positions.len()) {
            return Err(error(format!("triangle {i}: vertex index out of range")));
        }
    }
    if tree.is_some_and(|t| t.len() != mesh.triangles.len()) {
        return Err(error("the spatial index was built from a different mesh"));
    }
    let built;
    let tree = match tree {
        Some(t) => t,
        None => {
            built = Bvh::new(mesh);
            &built
        }
    };
    // The hit test accepts points up to 1e-12 (in barycentric terms) outside a triangle; widening every
    // box by far more than that, scaled to the part, keeps the tree from skipping any such hit.
    let reach = mesh
        .positions
        .iter()
        .flat_map(|p| p.iter().map(|v| f64::from(v.abs())))
        .fold(0.0, f64::max);
    let pad = 1e-9 * (1.0 + reach);
    // Check every selected vertex first, in order, so the first bad one is the error reported; then cast
    // the rays on all cores. Each ray is independent, so the samples do not depend on the thread count.
    let mut rays = Vec::with_capacity(selection.len());
    for &vertex in selection {
        let index = vertex as usize;
        let origin = mesh
            .positions
            .get(index)
            .ok_or_else(|| error(format!("vertex {vertex}: index out of range")))?
            .map(f64::from);
        let normal = outward_normals[index].map(f64::from);
        let direction = normalize(normal)
            .filter(|n| n.iter().all(|v| v.is_finite()))
            .map(|n| scale(n, -1.))
            .ok_or_else(|| {
                error(format!(
                    "vertex {vertex}: normal must be finite and nonzero"
                ))
            })?;
        rays.push((vertex, origin, direction));
    }
    let samples = crate::par::map(rays.len(), |k| {
        let (vertex, origin, direction) = rays[k];
        let nearest = tree
            .ray_nearest(
                origin,
                direction,
                options.exclusion,
                options.max_distance,
                pad,
                |t, [a, b, c]| {
                    if mesh.triangles[t].contains(&vertex) {
                        return None;
                    }
                    hit(origin, direction, a, b, c)
                },
            )
            .map(|(distance, _)| distance);
        ThicknessSample {
            vertex,
            distance: nearest,
        }
    });
    Ok(samples)
}
