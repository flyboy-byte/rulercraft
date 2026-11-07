// SPDX-License-Identifier: MIT OR Apache-2.0
//! Small linear algebra shared by the fits (docs/specs/math.md). Everything is `f64`.
#![allow(clippy::needless_range_loop)] // index loops are the clearest form for 3x3 matrix code

pub mod eigen;
pub mod lsq;
pub mod stats;
pub mod vec3;

pub use eigen::{Eigen3, symmetric_eigen3};
pub use lsq::{LmOptions, LmReport, levenberg_marquardt, solve_dense};
pub use stats::{Covariance, centroid, covariance};
pub use vec3::{Vec3, add, angle_between, cross, dot, length, normalize, scale, signed_angle, sub};
