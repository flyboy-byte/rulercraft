// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reverse-engineering guesses (docs/specs/reverse-engineering.md): hole patterns and design values.
//! Every guess carries the evidence for it so a person can accept or edit it. Lengths in mm.
use crate::math::{
    Vec3, add, angle_between, cross, dot, length, normalize, scale, solve_dense, sub,
};

/// A round hole or boss as the pattern finder sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Hole {
    /// Name of the fitted cylinder.
    pub name: String,
    /// A point on the axis.
    pub point: Vec3,
    /// Unit axis.
    pub axis: Vec3,
    /// Diameter.
    pub diameter: f64,
}

/// Limits for [`hole_patterns`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatternOptions {
    /// Diameters closer than this count as equal, mm.
    pub diameter_tolerance: f64,
    /// Axes closer than this count as parallel, degrees.
    pub axis_tolerance_deg: f64,
    /// A hole is on a line or circle when within this distance of it, mm.
    pub position_tolerance: f64,
    /// Spacings along a row closer than this count as equal, mm.
    pub spacing_tolerance: f64,
    /// Angular steps closer than this count as equal, degrees.
    pub spacing_deg: f64,
}
impl Default for PatternOptions {
    fn default() -> Self {
        Self {
            diameter_tolerance: 0.05,
            axis_tolerance_deg: 2.0,
            position_tolerance: 0.1,
            spacing_tolerance: 0.1,
            spacing_deg: 1.0,
        }
    }
}

/// A pattern of equal, parallel holes.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "pattern", rename_all = "snake_case")]
pub enum Pattern {
    /// Holes in a row.
    Line {
        /// Member names in order along the row.
        members: Vec<String>,
        /// Common diameter (mean), mm.
        diameter: f64,
        /// Mean distance between neighbours, mm.
        spacing: f64,
        /// Whether every gap is within the spacing tolerance of the mean.
        equal_spacing: bool,
        /// Largest distance of any hole from the row, mm.
        worst_offset: f64,
    },
    /// Holes on a circle.
    Circle {
        /// Member names in order around the circle.
        members: Vec<String>,
        /// Common diameter (mean), mm.
        diameter: f64,
        /// Circle centre.
        center: Vec3,
        /// Circle radius, mm.
        radius: f64,
        /// Smallest angle between neighbours, degrees.
        step_deg: f64,
        /// Largest distance of any hole from the circle, mm.
        worst_offset: f64,
    },
    /// Holes on a regular lattice (rows and columns, not necessarily square).
    Grid {
        /// Member names, row by row.
        members: Vec<String>,
        /// Common diameter (mean), mm.
        diameter: f64,
        /// Number of lattice positions along the first direction.
        columns: usize,
        /// Number of lattice positions along the second direction.
        rows: usize,
        /// Spacing along the first direction, mm.
        spacing_a: f64,
        /// Spacing along the second direction, mm.
        spacing_b: f64,
        /// Angle between the two directions, degrees (90 for a rectangular grid).
        angle_deg: f64,
        /// Whether every lattice position inside the rows and columns holds a hole.
        full: bool,
        /// Largest distance of any hole from its lattice position, mm.
        worst_offset: f64,
    },
}

fn parallel(a: Vec3, b: Vec3, limit_deg: f64) -> bool {
    angle_between(a, b).is_some_and(|angle| {
        let d = angle.to_degrees();
        d.min(180.0 - d) <= limit_deg
    })
}

/// Finds lines, circles and grids of equal parallel holes. A square of four holes is reported as a circle
/// (it is one); a grid needs at least 2 rows and 2 columns and 4 holes. A circle is reported only when every gap between
/// neighbours (including the closing one) is a whole number of the smallest gap: a bolt circle, possibly
/// with holes missing. Any three holes lie on some circle, so irregular ones are not called patterns. Holes are grouped by diameter and axis direction
/// (each group needs 3 or more). A group is reported only when all of it lies on one line or one circle;
/// a group that is partly a pattern is left alone rather than split by guesswork.
pub fn hole_patterns(holes: &[Hole], options: PatternOptions) -> Vec<Pattern> {
    let mut used = vec![false; holes.len()];
    let mut out = vec![];
    for i in 0..holes.len() {
        if used[i]
            || normalize(holes[i].axis).is_none()
            || !holes[i].diameter.is_finite()
            || holes[i].diameter <= 0.0
            || holes[i].point.iter().any(|v| !v.is_finite())
        {
            continue;
        }
        let group: Vec<usize> = (i..holes.len())
            .filter(|&j| {
                !used[j]
                    && holes[j].point.iter().all(|v| v.is_finite())
                    && holes[j].diameter.is_finite()
                    && holes[j].diameter > 0.0
                    && (holes[j].diameter - holes[i].diameter).abs() <= options.diameter_tolerance
                    && parallel(holes[j].axis, holes[i].axis, options.axis_tolerance_deg)
            })
            .collect();
        for &j in &group {
            used[j] = true;
        }
        if group.len() < 3 {
            continue;
        }
        if let Some(p) = line(holes, &group, &options)
            .or_else(|| circle(holes, &group, &options))
            .or_else(|| grid(holes, &group, &options))
        {
            out.push(p);
        }
    }
    out
}

fn line(holes: &[Hole], g: &[usize], o: &PatternOptions) -> Option<Pattern> {
    let pts: Vec<Vec3> = g.iter().map(|&i| holes[i].point).collect();
    // Direction between the two farthest holes, found against the first.
    let a = pts[0];
    let far = pts
        .iter()
        .copied()
        .max_by(|p, q| length(sub(*p, a)).total_cmp(&length(sub(*q, a))))?;
    let b = pts
        .iter()
        .copied()
        .max_by(|p, q| length(sub(*p, far)).total_cmp(&length(sub(*q, far))))?;
    let u = normalize(sub(b, far))?;
    // Use the closer endpoint to retain small offsets near either end of a long row.
    let worst = pts.iter().try_fold(0.0f64, |worst, &p| {
        let from_far = sub(p, far);
        let from_b = sub(p, b);
        let delta = if length(from_far) <= length(from_b) {
            from_far
        } else {
            from_b
        };
        let distance = length(delta);
        // If rounding of the displacement itself can exceed the requested tolerance,
        // do not report an apparently exact line on uncertain evidence.
        if !distance.is_finite() || distance * f64::EPSILON * 8.0 > o.position_tolerance {
            return None;
        }
        Some(worst.max(length(cross(delta, u))))
    })?;
    if worst > o.position_tolerance {
        return None;
    }
    let mut order: Vec<usize> = (0..g.len()).collect();
    // The largest direction component gives a stable total order without subtracting
    // a distant endpoint from every coordinate. The adjacent projections below
    // still verify that off-line tolerances did not reverse that order.
    let axis = (0..3).max_by(|&a, &b| u[a].abs().total_cmp(&u[b].abs()))?;
    order.sort_by(|&p, &q| {
        let cmp = pts[p][axis].total_cmp(&pts[q][axis]);
        if u[axis] < 0.0 { cmp.reverse() } else { cmp }
    });
    let gaps: Vec<f64> = order
        .windows(2)
        .map(|w| dot(sub(pts[w[1]], pts[w[0]]), u))
        .collect();
    // Refuse the row if off-line offsets make coordinate order disagree with
    // direct adjacent projections along the fitted line.
    if gaps.iter().any(|gap| !gap.is_finite() || *gap < 0.0) {
        return None;
    }
    let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
    Some(Pattern::Line {
        members: order.iter().map(|&k| holes[g[k]].name.clone()).collect(),
        diameter: g.iter().map(|&i| holes[i].diameter).sum::<f64>() / g.len() as f64,
        spacing: mean,
        equal_spacing: gaps.iter().all(|x| (x - mean).abs() <= o.spacing_tolerance),
        worst_offset: worst,
    })
}

fn grid(holes: &[Hole], g: &[usize], o: &PatternOptions) -> Option<Pattern> {
    if g.len() < 4 {
        return None;
    }
    let z = normalize(holes[g[0]].axis)?;
    let seed = if z[0].abs() < 0.9 {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let x = normalize(cross(z, seed))?;
    let y = cross(z, x);
    let origin = holes[g[0]].point;
    let uv: Vec<[f64; 2]> = g
        .iter()
        .map(|&i| {
            let d = sub(holes[i].point, origin);
            [dot(d, x), dot(d, y)]
        })
        .collect();
    let len = |v: [f64; 2]| v[0].hypot(v[1]);
    let min_step = o.position_tolerance * 2.0;
    // First lattice direction: the nearest other hole; second: the nearest not along the first.
    let mut others: Vec<[f64; 2]> = uv[1..]
        .iter()
        .copied()
        .filter(|&v| len(v) > min_step)
        .collect();
    others.sort_by(|p, q| len(*p).total_cmp(&len(*q)));
    let a = *others.first()?;
    let b = *others
        .iter()
        .find(|&&v| (a[0] * v[1] - a[1] * v[0]).abs() > 0.1 * len(a) * len(v))?;
    let det = a[0] * b[1] - a[1] * b[0];
    let mut cells: Vec<(i64, i64, usize)> = vec![];
    let mut worst = 0.0f64;
    for (k, q) in uv.iter().enumerate() {
        let i = (q[0] * b[1] - q[1] * b[0]) / det;
        let j = (a[0] * q[1] - a[1] * q[0]) / det;
        let (ri, rj) = (i.round(), j.round());
        // Float-to-integer casts saturate; refuse unrepresentable cells rather than
        // inventing coordinates or overflowing the grid dimensions below.
        if !ri.is_finite()
            || !rj.is_finite()
            || ri < i64::MIN as f64
            || ri >= i64::MAX as f64
            || rj < i64::MIN as f64
            || rj >= i64::MAX as f64
        {
            return None;
        }
        let off = len([q[0] - ri * a[0] - rj * b[0], q[1] - ri * a[1] - rj * b[1]]);
        if !off.is_finite()
            || off > o.position_tolerance
            || cells.iter().any(|c| c.0 == ri as i64 && c.1 == rj as i64)
        {
            return None;
        }
        worst = worst.max(off);
        cells.push((ri as i64, rj as i64, k));
    }
    let (i0, i1) = (
        cells.iter().map(|c| c.0).min()?,
        cells.iter().map(|c| c.0).max()?,
    );
    let (j0, j1) = (
        cells.iter().map(|c| c.1).min()?,
        cells.iter().map(|c| c.1).max()?,
    );
    let columns = usize::try_from(i1.checked_sub(i0)?.checked_add(1)?).ok()?;
    let rows = usize::try_from(j1.checked_sub(j0)?.checked_add(1)?).ok()?;
    if columns < 2 || rows < 2 {
        return None;
    }
    cells.sort_by_key(|c| (c.1, c.0));
    let cos = (dot([a[0], a[1], 0.0], [b[0], b[1], 0.0]) / (len(a) * len(b))).clamp(-1.0, 1.0);
    Some(Pattern::Grid {
        members: cells.iter().map(|c| holes[g[c.2]].name.clone()).collect(),
        diameter: g.iter().map(|&i| holes[i].diameter).sum::<f64>() / g.len() as f64,
        columns,
        rows,
        spacing_a: len(a),
        spacing_b: len(b),
        angle_deg: cos.acos().to_degrees(),
        full: columns
            .checked_mul(rows)
            .is_some_and(|area| cells.len() == area),
        worst_offset: worst,
    })
}

fn circle(holes: &[Hole], g: &[usize], o: &PatternOptions) -> Option<Pattern> {
    let z = normalize(holes[g[0]].axis)?;
    let seed = if z[0].abs() < 0.9 {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let x = normalize(cross(z, seed))?;
    let y = cross(z, x);
    let origin = holes[g[0]].point;
    let uv: Vec<(f64, f64)> = g
        .iter()
        .map(|&i| {
            let d = sub(holes[i].point, origin);
            (dot(d, x), dot(d, y))
        })
        .collect();
    // Algebraic circle: u² + v² + a u + b v + c = 0 (least squares over all holes).
    let mut m = vec![0.0; 9];
    let mut r = vec![0.0; 3];
    for &(u, v) in &uv {
        let row = [u, v, 1.0];
        let rhs = -(u * u + v * v);
        for p in 0..3 {
            for q in 0..3 {
                m[p * 3 + q] += row[p] * row[q];
            }
            r[p] += row[p] * rhs;
        }
    }
    let s = solve_dense(m, r)?;
    let (cu, cv) = (-s[0] / 2.0, -s[1] / 2.0);
    let r2 = cu * cu + cv * cv - s[2];
    if r2.is_nan() || r2 <= 0.0 {
        return None;
    }
    let radius = r2.sqrt();
    let dist = |&(u, v): &(f64, f64)| ((u - cu).hypot(v - cv) - radius).abs();
    let worst = uv.iter().map(dist).fold(0.0, f64::max);
    if worst > o.position_tolerance {
        return None;
    }
    let mut order: Vec<usize> = (0..g.len()).collect();
    let ang = |k: usize| (uv[k].1 - cv).atan2(uv[k].0 - cu);
    order.sort_by(|&p, &q| ang(p).total_cmp(&ang(q)));
    let n = order.len();
    let gaps: Vec<f64> = (0..n)
        .map(|k| {
            let d = ang(order[(k + 1) % n]) - ang(order[k]);
            (if d <= 0.0 {
                d + std::f64::consts::TAU
            } else {
                d
            })
            .to_degrees()
        })
        .collect();
    let step = gaps.iter().copied().fold(f64::MAX, f64::min);
    let regular = gaps.iter().all(|&gap| {
        let k = (gap / step).round().max(1.0);
        (gap - k * step).abs() <= o.spacing_deg
    });
    if !regular {
        return None;
    }
    let centre = add(origin, add(scale(x, cu), scale(y, cv)));
    Some(Pattern::Circle {
        members: order.iter().map(|&k| holes[g[k]].name.clone()).collect(),
        diameter: g.iter().map(|&i| holes[i].diameter).sum::<f64>() / n as f64,
        center: centre,
        radius,
        step_deg: step,
        worst_offset: worst,
    })
}

/// One guess at a design value.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Guess {
    /// The round number, in mm.
    pub value: f64,
    /// Which family of round numbers it comes from.
    pub basis: &'static str,
    /// Measured minus guessed, mm (the evidence).
    pub error: f64,
}

const STEPS: [(&str, f64); 6] = [
    ("whole mm", 1.0),
    ("half mm", 0.5),
    ("tenth mm", 0.1),
    ("eighth inch", 25.4 / 8.0),
    ("sixteenth inch", 25.4 / 16.0),
    ("thirty-second inch", 25.4 / 32.0),
];

/// Round numbers a measured length could have been designed as. For each family, the nearest round
/// number is offered when the measurement is within `tolerance` of it. Millimetre families come first,
/// coarse to fine, then inch families. Returns nothing for a value or tolerance
/// that is not finite, or a tolerance of 0 or less.
pub fn design_guesses(measured: f64, tolerance: f64) -> Vec<Guess> {
    if !measured.is_finite() || !tolerance.is_finite() || tolerance <= 0.0 {
        return vec![];
    }
    STEPS
        .iter()
        .filter_map(|&(basis, step)| {
            let value = (measured / step).round() * step;
            let error = measured - value;
            (error.abs() <= tolerance && value != 0.0).then_some(Guess {
                value,
                basis,
                error,
            })
        })
        .collect()
}

/// How well a mesh mirrors across one candidate plane.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Symmetry {
    /// A point on the plane (the vertex centroid).
    pub center: Vec3,
    /// Unit normal (a principal axis of the vertex cloud).
    pub normal: Vec3,
    /// RMS distance from the mirrored vertices to the surface, mm.
    pub rms: f64,
    /// Largest such distance, mm.
    pub max: f64,
    /// Whether the largest distance is within the tolerance.
    pub symmetric: bool,
}

/// Tests mirror symmetry across the three planes through the vertex centroid whose normals are the
/// principal axes of the vertex cloud. Each vertex is reflected through the plane and its distance to the
/// mesh surface measured; the plane is called a symmetry plane when every one is within `tolerance`.
/// Limits: only these three candidate planes are tried, and a part whose spreads are nearly equal along
/// two axes (a cube, a round part) has no well-defined principal axes there, so a true symmetry plane can
/// be missed. Vertices are sampled, not surface points. Refuses a mesh without triangles, fewer than 4
/// vertices, and a tolerance that is not a finite number above 0.
pub fn symmetry_planes(
    mesh: &crate::Mesh,
    tolerance: f64,
) -> Result<Vec<Symmetry>, crate::deviation::DeviationError> {
    use crate::deviation::{DeviationError, Reference};
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(DeviationError(
            "tolerance must be a finite number above 0".into(),
        ));
    }
    if mesh.positions.len() < 4 {
        return Err(DeviationError("symmetry needs at least 4 vertices".into()));
    }
    let tree = Reference::new(mesh)?;
    let pts: Vec<Vec3> = mesh.positions.iter().map(|p| p.map(f64::from)).collect();
    let cov = crate::math::covariance(pts.iter().copied())
        .ok_or_else(|| DeviationError("no vertices".into()))?;
    let eig = crate::math::symmetric_eigen3(cov.matrix)
        .ok_or_else(|| DeviationError("the vertex cloud has no principal axes".into()))?;
    Ok(eig
        .vectors
        .iter()
        .map(|&n| {
            let (mut sum2, mut max) = (0.0, 0.0f64);
            for &p in &pts {
                let off = dot(sub(p, cov.centroid), n);
                let mirrored = sub(p, scale(n, 2.0 * off));
                let (q, _) = tree.closest(mirrored);
                let d = length(sub(mirrored, q));
                sum2 += d * d;
                max = max.max(d);
            }
            Symmetry {
                center: cov.centroid,
                normal: n,
                rms: (sum2 / pts.len() as f64).sqrt(),
                max,
                symmetric: max <= tolerance,
            }
        })
        .collect())
}
