// SPDX-License-Identifier: MIT OR Apache-2.0
//! RulerCraft's mesh engine: read a mesh, find its flat and round features, measure them, and describe
//! the part as a CAD build recipe. Everything works in millimetres and degrees. Behaviour is specified in
//! `docs/specs/`.
//!
//! The usual flow (this example is run as a test):
//!
//! ```
//! use rulercraft_mesh::prelude::*;
//!
//! // A 10 x 10 mm square plate in the z = 5 plane, as an ASCII PLY.
//! let ply = "ply\nformat ascii 1.0\nelement vertex 4\nproperty float x\nproperty float y\n\
//! property float z\nelement face 2\nproperty list uchar int vertex_indices\nend_header\n\
//! 0 0 5\n10 0 5\n10 10 5\n0 10 5\n3 0 1 2\n3 0 2 3\n";
//! let mesh = read_mesh("plate.ply", ply.as_bytes(), Unit::Mm, ImportLimits::default())?;
//! let graph = MeshGraph::build(&mesh);
//!
//! let request = FitRequest {
//!     kind: FitKind::Plane,
//!     selection: Selection::default(), // every vertex
//!     clip: Clip::None,
//!     known_diameter: None,
//! };
//! let outcome = commands::fit(&mesh, &graph, &request)?;
//! let Fitted::Plane(plane) = outcome.fitted else { unreachable!() };
//! assert!((plane.center[2] - 5.0).abs() < 1e-9);
//! assert!(plane.normal[2].abs() > 0.999_999);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Where things are
//!
//! - Reading: [`read_mesh`] (STL, PLY, OBJ, 3MF), [`ImportLimits`], [`Mesh`].
//! - Marking and fitting: [`commands::select`], [`commands::fit`], and the fits in [`fit`] (plane, sphere,
//!   cylinder, and `*_robust` for noisy scans).
//! - Measuring: [`measure::relate`] (distance and angle between fitted elements),
//!   [`measure::thickness`](measure) for walls, [`deviation`] for mesh-to-mesh comparison, [`align`] for
//!   moving a scan onto a reference.
//! - Finding features automatically: [`segment`] (planes, cylinders, joined strips) and [`reverse`] (hole
//!   patterns, symmetry, round-number guesses).
//! - Spatial queries: [`spatial::Bvh`], one tree for closest point and ray casts, built once.
//! - Building: [`cad::recipe`] turns fitted features into plain CAD steps.
//! - Saving: [`project`].
//!
//! Every public function refuses bad input with an error rather than panicking, and results are
//! deterministic.

pub mod align;
pub mod cad;
pub mod commands;
pub mod deviation;
pub mod fit;
pub mod graph;
pub mod io;
pub mod math;
pub mod measure;
pub mod mesh;
pub mod project;
pub mod reverse;
pub mod segment;
pub mod select;
pub mod spatial;
pub mod stl;

pub use mesh::{Bounds, Mesh, MeshBuilder, Unit};
pub use stl::{Limits, ReadError, read_stl};

pub use io::{ImportLimits, read_mesh};

/// The names most programs need, in one `use`.
pub mod prelude {
    pub use crate::commands::{self, FitKind, FitRequest, Fitted, Selection};
    pub use crate::fit::{Clip, CylinderFit, PlaneFit, SphereFit};
    pub use crate::graph::MeshGraph;
    pub use crate::measure::{Element, Relation, relate};
    pub use crate::{ImportLimits, Mesh, ReadError, Unit, read_mesh};
}
