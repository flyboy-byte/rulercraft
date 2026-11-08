// SPDX-License-Identifier: MIT OR Apache-2.0
//! The command layer the CLI and the MCP server share (docs/specs/commands.md): typed requests in,
//! typed results out. No JSON here; front ends translate.

use crate::fit::{
    Clip, CylinderFit, CylinderOptions, FitError, PlaneFit, SphereFit, fit_cylinder, fit_plane,
    fit_sphere,
};
use crate::graph::MeshGraph;
use crate::math::Vec3;
use crate::mesh::Mesh;
use crate::select::{SelectError, facing, in_box, near};
use std::fmt;

/// Which vertices a command works on. Every given filter applies; none means the whole mesh.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Selection {
    /// Inclusive box (min, max), mm.
    pub in_box: Option<(Vec3, Vec3)>,
    /// Within a radius (centre, radius), mm.
    pub near: Option<(Vec3, f64)>,
    /// Facing a direction within an angle (direction, radians).
    pub facing: Option<(Vec3, f64)>,
}

/// What to fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FitKind {
    /// A plane.
    Plane,
    /// A sphere.
    Sphere,
    /// A cylinder.
    Cylinder,
}

/// A fit command.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FitRequest {
    /// Element kind.
    pub kind: FitKind,
    /// The vertices to fit.
    pub selection: Selection,
    /// Outlier cut-off.
    pub clip: Clip,
    /// Sphere: hold this diameter, mm. Cylinder: the designed (assumed) diameter.
    pub known_diameter: Option<f64>,
}

/// A fitted element.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Fitted {
    /// A plane.
    Plane(PlaneFit),
    /// A sphere.
    Sphere(SphereFit),
    /// A cylinder.
    Cylinder(CylinderFit),
}

/// The result of a fit command.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FitOutcome {
    /// Vertices the selection picked.
    pub selected: usize,
    /// The fit.
    pub fitted: Fitted,
}

/// Why a command failed. `Display` gives a message a user can act on.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandError {
    /// The selection was invalid.
    Select(SelectError),
    /// The fit failed.
    Fit(FitError),
    /// The request does not make sense as given.
    Invalid(&'static str),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::Select(e) => write!(f, "selection: {e}"),
            CommandError::Fit(e) => write!(f, "fit: {e}"),
            CommandError::Invalid(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for CommandError {}

/// The vertices `selection` picks, ascending.
pub fn select(
    mesh: &Mesh,
    graph: &MeshGraph,
    selection: &Selection,
) -> Result<Vec<u32>, CommandError> {
    let mut picked: Vec<u32> = match selection.in_box {
        Some((min, max)) => in_box(&mesh.positions, min, max).map_err(CommandError::Select)?,
        None => (0..mesh.positions.len() as u32).collect(),
    };
    if let Some((center, radius)) = selection.near {
        let close = near(&mesh.positions, center, radius).map_err(CommandError::Select)?;
        // Both filters return ascending vertex IDs, so merge them in one pass.
        let mut close = close.into_iter().peekable();
        picked.retain(|i| {
            while close.peek().is_some_and(|candidate| candidate < i) {
                close.next();
            }
            close.peek() == Some(i)
        });
    }
    if let Some((direction, within)) = selection.facing {
        picked = facing(&picked, &graph.vertex_normals, direction, within)
            .map_err(CommandError::Select)?;
    }
    Ok(picked)
}

/// Runs a fit command on a mesh and its graph.
pub fn fit(
    mesh: &Mesh,
    graph: &MeshGraph,
    request: &FitRequest,
) -> Result<FitOutcome, CommandError> {
    if request.known_diameter.is_some() && request.kind == FitKind::Plane {
        return Err(CommandError::Invalid(
            "a known diameter only applies to a sphere or cylinder fit",
        ));
    }
    let picked = select(mesh, graph, &request.selection)?;
    let fitted = match request.kind {
        FitKind::Plane => Fitted::Plane(
            fit_plane(
                &mesh.positions,
                Some(&graph.vertex_normals),
                &picked,
                request.clip,
            )
            .map_err(CommandError::Fit)?,
        ),
        FitKind::Sphere => Fitted::Sphere(
            fit_sphere(
                &mesh.positions,
                &picked,
                request.clip,
                request.known_diameter,
            )
            .map_err(CommandError::Fit)?,
        ),
        FitKind::Cylinder => {
            let options = CylinderOptions {
                clip: request.clip,
                assumed_diameter: request.known_diameter,
                ..Default::default()
            };
            Fitted::Cylinder(
                fit_cylinder(
                    &mesh.positions,
                    Some(&graph.vertex_normals),
                    &picked,
                    options,
                )
                .map_err(CommandError::Fit)?,
            )
        }
    };
    Ok(FitOutcome {
        selected: picked.len(),
        fitted,
    })
}
