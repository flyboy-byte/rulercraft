// SPDX-License-Identifier: MIT OR Apache-2.0
//! A CAD recipe: fitted planes and cylinders turned into plain build steps (a base face, holes or bosses
//! placed on it, parallel faces at a height) that an agent can replay over a CAD program's tool server.
//! Nothing here talks to any CAD program. Lengths in mm, angles in degrees.
use crate::fit::{CylinderFit, PlaneFit};
use crate::math::{angle_between, dot, sub};

/// A cylinder is "square to the base" when its axis is within this many degrees of the base normal.
pub const SQUARE_DEG: f64 = 5.0;

/// A named fit.
pub type Named<T> = (String, T);

/// One build step.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum Step {
    /// The base face: the largest plane. Its two in-plane axes carry the positions below.
    Base {
        /// Name of the fitted plane.
        name: String,
        /// Face size along `u` and `v`, mm.
        size: [f64; 2],
    },
    /// A round feature square to the base, positioned in the base's own frame.
    Round {
        /// Name of the fitted cylinder.
        name: String,
        /// Position along the base's `u` axis, mm.
        u: f64,
        /// Position along the base's `v` axis, mm.
        v: f64,
        /// Height of the axis midpoint above the base plane (signed along its normal), mm.
        height: f64,
        /// Diameter, mm.
        diameter: f64,
        /// Measured axial length, mm.
        length: f64,
    },
    /// A face parallel to the base, at a height above it (signed along the base normal).
    ParallelFace {
        /// Name of the fitted plane.
        name: String,
        /// Height, mm.
        height: f64,
    },
    /// Anything that fits none of the above, reported rather than guessed.
    Unplaced {
        /// Name of the fit.
        name: String,
        /// Why it was left out.
        reason: String,
    },
}

/// The recipe: steps in build order.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Recipe {
    /// Steps, base first.
    pub steps: Vec<Step>,
}

fn acute(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = angle_between(a, b).unwrap_or(0.0).to_degrees();
    if d > 90.0 { 180.0 - d } else { d }
}

/// Builds the recipe. With no planes there is no base, so every cylinder is `Unplaced`.
pub fn recipe(planes: &[Named<PlaneFit>], cylinders: &[Named<CylinderFit>]) -> Recipe {
    let area = |p: &PlaneFit| p.span_u * p.span_v;
    let base = planes
        .iter()
        .enumerate()
        .max_by(|a, b| area(&a.1.1).total_cmp(&area(&b.1.1)))
        .map(|(i, _)| i);
    let mut steps = vec![];
    let Some(bi) = base else {
        steps.extend(cylinders.iter().map(|(n, _)| Step::Unplaced {
            name: n.clone(),
            reason: "no plane to use as a base".into(),
        }));
        return Recipe { steps };
    };
    let b = &planes[bi].1;
    steps.push(Step::Base {
        name: planes[bi].0.clone(),
        size: [b.span_u, b.span_v],
    });
    for (i, (n, p)) in planes.iter().enumerate() {
        if i == bi {
            continue;
        }
        steps.push(if acute(p.normal, b.normal) <= SQUARE_DEG {
            Step::ParallelFace {
                name: n.clone(),
                height: dot(sub(p.center, b.center), b.normal),
            }
        } else {
            Step::Unplaced {
                name: n.clone(),
                reason: "plane is not parallel to the base".into(),
            }
        });
    }
    for (n, c) in cylinders {
        let d = sub(c.axis_point, b.center);
        steps.push(if acute(c.axis, b.normal) <= SQUARE_DEG {
            Step::Round {
                name: n.clone(),
                u: dot(d, b.basis_u),
                v: dot(d, b.basis_v),
                height: dot(d, b.normal),
                diameter: c.diameter,
                length: c.length,
            }
        } else {
            Step::Unplaced {
                name: n.clone(),
                reason: "axis is not square to the base".into(),
            }
        });
    }
    Recipe { steps }
}
