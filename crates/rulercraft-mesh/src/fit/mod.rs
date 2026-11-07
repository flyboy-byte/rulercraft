// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fitting geometric elements to marked vertices (docs/specs/fit-*.md).

pub mod plane;

pub use plane::{Clip, FitError, PlaneFit, fit_plane};
