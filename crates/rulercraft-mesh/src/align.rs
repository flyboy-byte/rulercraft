// SPDX-License-Identifier: MIT OR Apache-2.0
//! Point-to-plane ICP (docs/specs/alignment.md): moves a scan onto a reference mesh by a rigid
//! transform (a turn then a move, no scaling). Each round pairs scan vertices with their closest points
//! on the reference, then solves for the small turn and move that shrinks the distances along the
//! reference's surface normals. Lengths in mm.
use crate::Mesh;
use crate::deviation::Reference;
use crate::math::{Vec3, add, cross, dot, length, normalize, solve_dense, sub};

/// Options for [`align_icp`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IcpOptions {
    /// Most rounds.
    pub max_iterations: usize,
    /// Stop when a round improves the RMS distance by less than this fraction of it.
    pub tolerance: f64,
    /// Pairs farther apart than this are ignored (for scans that only partly overlap), mm.
    pub max_pair_distance: f64,
    /// At most this many scan vertices are used per round (evenly spaced).
    pub max_points: usize,
}
impl Default for IcpOptions {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            tolerance: 1e-6,
            max_pair_distance: f64::INFINITY,
            max_points: 20_000,
        }
    }
}

/// A rigid transform: `p' = rotation · p + translation`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct Rigid {
    /// Row-major 3x3 rotation.
    pub rotation: [[f64; 3]; 3],
    /// Translation, mm.
    pub translation: Vec3,
}
impl Rigid {
    /// The do-nothing transform.
    pub const IDENTITY: Rigid = Rigid {
        rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        translation: [0.; 3],
    };
    /// Applies the transform to a point.
    pub fn apply(&self, p: Vec3) -> Vec3 {
        let r = &self.rotation;
        add([dot(r[0], p), dot(r[1], p), dot(r[2], p)], self.translation)
    }
}

/// Result of [`align_icp`].
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct IcpResult {
    /// Takes the scan's coordinates onto the reference's.
    pub transform: Rigid,
    /// RMS distance of the pairs before the first round, mm.
    pub rms_before: f64,
    /// RMS distance of the pairs at the start of the last round, mm.
    pub rms_after: f64,
    /// Rounds taken.
    pub iterations: usize,
    /// Pairs used in the last round.
    pub pairs: usize,
    /// Whether the tolerance was met (otherwise the round limit or a stall stopped it).
    pub converged: bool,
}

/// Why an alignment could not be done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcpError(pub String);
impl std::fmt::Display for IcpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for IcpError {}

/// Rotation for the turn vector `w` (axis times angle), by Rodrigues' formula.
fn rodrigues(w: Vec3) -> [[f64; 3]; 3] {
    let a = length(w);
    let (k, s, c) = match normalize(w) {
        Some(k) => (k, a.sin(), a.cos()),
        None => return Rigid::IDENTITY.rotation,
    };
    let (x, y, z) = (k[0], k[1], k[2]);
    let v = 1.0 - c;
    [
        [c + x * x * v, x * y * v - z * s, x * z * v + y * s],
        [y * x * v + z * s, c + y * y * v, y * z * v - x * s],
        [z * x * v - y * s, z * y * v + x * s, c + z * z * v],
    ]
}
fn mul(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut m = [[0.0; 3]; 3];
    for (i, row) in m.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    m
}

/// Aligns `scan` onto `reference`, starting from `start` (use [`Rigid::IDENTITY`] when the two are
/// already roughly in place; ICP only finds the nearest fit and cannot recover a large misplacement).
pub fn align_icp(
    scan: &Mesh,
    reference: &Mesh,
    start: Rigid,
    options: IcpOptions,
) -> Result<IcpResult, IcpError> {
    if !(options.tolerance.is_finite() && options.tolerance >= 0.0)
        || options.max_iterations == 0
        || options.max_points < 6
        || options.max_pair_distance.is_nan()
        || options.max_pair_distance <= 0.0
    {
        return Err(IcpError("bad ICP options".into()));
    }
    let tree = Reference::new(reference).map_err(|e| IcpError(e.0))?;
    if scan.positions.len() < 6 {
        return Err(IcpError("the scan needs at least 6 vertices".into()));
    }
    let step = scan.positions.len().div_ceil(options.max_points);
    let src: Vec<Vec3> = scan
        .positions
        .iter()
        .step_by(step)
        .map(|p| p.map(f64::from))
        .collect();
    let mut t = start;
    let (mut rms_before, mut last_rms, mut pairs, mut converged, mut rounds) =
        (f64::NAN, f64::NAN, 0, false, 0);
    for round in 0..options.max_iterations {
        let mut a = vec![0.0; 36];
        let mut b = vec![0.0; 6];
        let (mut sum2, mut n) = (0.0, 0usize);
        // Pairing runs in parallel; the sums below are taken in scan order on one thread, so the
        // result does not depend on the thread count.
        let paired = crate::par::map(src.len(), |k| {
            let p = t.apply(src[k]);
            let (q, normal) = tree.closest(p);
            let (Some(nu), d) = (normalize(normal), sub(q, p)) else {
                return None;
            };
            if length(d) > options.max_pair_distance {
                return None;
            }
            let c = cross(p, nu);
            Some(([c[0], c[1], c[2], nu[0], nu[1], nu[2]], dot(nu, d)))
        });
        for (row, r) in paired.into_iter().flatten() {
            sum2 += r * r;
            n += 1;
            for i in 0..6 {
                for j in 0..6 {
                    a[i * 6 + j] += row[i] * row[j];
                }
                b[i] += row[i] * r;
            }
        }
        if n < 6 {
            return Err(IcpError(
                "fewer than 6 scan vertices have a reference point within the pair distance".into(),
            ));
        }
        let rms = (sum2 / n as f64).sqrt();
        if round == 0 {
            rms_before = rms;
        }
        pairs = n;
        rounds = round + 1;
        let improved = last_rms.is_nan() || (last_rms - rms) > options.tolerance * last_rms;
        last_rms = rms;
        if !improved {
            converged = true;
            break;
        }
        let Some(x) = solve_dense(a, b) else {
            return Err(IcpError(
                "the surface is too featureless to align (a flat or round scan can slide)".into(),
            ));
        };
        let dr = rodrigues([x[0], x[1], x[2]]);
        t = Rigid {
            rotation: mul(&dr, &t.rotation),
            translation: add(
                [
                    dot(dr[0], t.translation),
                    dot(dr[1], t.translation),
                    dot(dr[2], t.translation),
                ],
                [x[3], x[4], x[5]],
            ),
        };
    }
    Ok(IcpResult {
        transform: t,
        rms_before,
        rms_after: last_rms,
        iterations: rounds,
        pairs,
        converged,
    })
}

/// Why a datum frame could not be made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatumError(pub &'static str);
impl std::fmt::Display for DatumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for DatumError {}

/// A plane must be at least this many degrees off the one before it to count as a different direction.
pub const DATUM_MIN_ANGLE_DEG: f64 = 10.0;

/// The transform that takes a part's own coordinates onto its datum frame, from three fitted planes
/// given as (point, unit normal) in the order primary, secondary, tertiary. The origin is where the
/// three planes meet. Z is the primary normal; X is the secondary normal with its Z part removed; Y is
/// Z × X. The tertiary plane only fixes the origin. Normals set the axis directions, so give them the
/// same side each time you align a batch of parts. Refuses planes within [`DATUM_MIN_ANGLE_DEG`] of
/// parallel, or whose normals are not independent enough to meet at one point.
pub fn datum_frame(
    primary: (Vec3, Vec3),
    secondary: (Vec3, Vec3),
    tertiary: (Vec3, Vec3),
) -> Result<Rigid, DatumError> {
    let unit = |n: Vec3| normalize(n).ok_or(DatumError("a plane normal has no length"));
    let (n1, n2, n3) = (unit(primary.1)?, unit(secondary.1)?, unit(tertiary.1)?);
    let min_sin = DATUM_MIN_ANGLE_DEG.to_radians().sin();
    if length(cross(n1, n2)) < min_sin {
        return Err(DatumError(
            "the primary and secondary planes are nearly parallel",
        ));
    }
    let z = n1;
    let x = unit(sub(n2, crate::math::scale(z, dot(n2, z))))?;
    let y = cross(z, x);
    // Meeting point: n_i · o = n_i · p_i for the three planes.
    let a = [n1, n2, n3].iter().flatten().copied().collect::<Vec<f64>>();
    let b = vec![
        dot(n1, primary.0),
        dot(n2, secondary.0),
        dot(n3, tertiary.0),
    ];
    let det = dot(n1, cross(n2, n3)).abs();
    if det < min_sin * min_sin {
        return Err(DatumError("the three planes do not meet at one point"));
    }
    let o = solve_dense(a, b).ok_or(DatumError("the three planes do not meet at one point"))?;
    let rotation = [x, y, z];
    let translation = [
        -dot(x, [o[0], o[1], o[2]]),
        -dot(y, [o[0], o[1], o[2]]),
        -dot(z, [o[0], o[1], o[2]]),
    ];
    Ok(Rigid {
        rotation,
        translation,
    })
}
