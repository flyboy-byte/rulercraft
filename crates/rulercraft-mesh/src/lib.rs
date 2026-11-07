// SPDX-License-Identifier: MIT OR Apache-2.0
//! RulerCraft's mesh engine: reading meshes, and (to come) fitting and measuring their features.
//! Behaviour is specified in `docs/specs/`.

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
pub mod segment;
pub mod select;
pub mod stl;

pub use mesh::{Bounds, Mesh, MeshBuilder, Unit};
pub use stl::{Limits, ReadError, read_stl};

pub use io::{ImportLimits, read_mesh};
