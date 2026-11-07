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

/// How well fitted planes and cylinders explain a mesh.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CheckReport {
    /// Vertices checked.
    pub vertices: usize,
    /// Vertices within the tolerance of some feature.
    pub explained: usize,
    /// `explained / vertices` (1 for an empty mesh).
    pub explained_fraction: f64,
    /// RMS distance of the explained vertices to their nearest feature, mm.
    pub rms: f64,
    /// Largest distance among the explained vertices, mm.
    pub max: f64,
    /// Ids of vertices no feature explains, ascending.
    pub unexplained: Vec<u32>,
    /// Explained vertices per feature (each counted for its nearest feature), in input order,
    /// planes first.
    pub per_feature: Vec<(String, usize)>,
}

/// Checks the recipe's features against the mesh: each vertex is matched to the nearest feature it lies on
/// and the distance recorded. A plane covers vertices within a span of its centre across the plane; a
/// cylinder covers vertices within its measured length (plus its display extensions) along its axis. A
/// vertex further than `tolerance` from every feature is unexplained. This is a coverage check on the
/// fitted primitives, not a comparison with a solid built in a CAD program (that is the SolidCore
/// projection still to come). Refuses a tolerance that is not finite and above 0.
pub fn check(
    mesh: &crate::Mesh,
    planes: &[Named<PlaneFit>],
    cylinders: &[Named<CylinderFit>],
    tolerance: f64,
) -> Result<CheckReport, crate::deviation::DeviationError> {
    use crate::math::{length, scale};
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(crate::deviation::DeviationError(
            "tolerance must be a finite number above 0".into(),
        ));
    }
    let mut per = vec![0usize; planes.len() + cylinders.len()];
    let (mut sum2, mut max, mut explained) = (0.0, 0.0f64, 0usize);
    let mut unexplained = vec![];
    for (id, p) in mesh.positions.iter().enumerate() {
        let p = p.map(f64::from);
        let mut best: Option<(usize, f64)> = None;
        for (k, (_, f)) in planes.iter().enumerate() {
            let d = sub(p, f.center);
            let (u, v) = (dot(d, f.basis_u).abs(), dot(d, f.basis_v).abs());
            let off = dot(d, f.normal).abs();
            if u <= f.span_u + tolerance
                && v <= f.span_v + tolerance
                && best.is_none_or(|b| off < b.1)
            {
                best = Some((k, off));
            }
        }
        for (k, (_, c)) in cylinders.iter().enumerate() {
            let d = sub(p, c.axis_point);
            let along = dot(d, c.axis);
            let reach = c.length / 2.0 + c.extend_before.max(c.extend_after) + tolerance;
            if along.abs() > reach {
                continue;
            }
            let off = (length(sub(d, scale(c.axis, along))) - c.radius).abs();
            if best.is_none_or(|b| off < b.1) {
                best = Some((planes.len() + k, off));
            }
        }
        match best {
            Some((k, off)) if off <= tolerance => {
                explained += 1;
                per[k] += 1;
                sum2 += off * off;
                max = max.max(off);
            }
            _ => unexplained.push(id as u32),
        }
    }
    let n = mesh.positions.len();
    Ok(CheckReport {
        vertices: n,
        explained,
        explained_fraction: if n == 0 {
            1.0
        } else {
            explained as f64 / n as f64
        },
        rms: if explained == 0 {
            0.0
        } else {
            (sum2 / explained as f64).sqrt()
        },
        max,
        unexplained,
        per_feature: planes
            .iter()
            .map(|(n, _)| n.clone())
            .chain(cylinders.iter().map(|(n, _)| n.clone()))
            .zip(per)
            .collect(),
    })
}
