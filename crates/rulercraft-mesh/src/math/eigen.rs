// SPDX-License-Identifier: MIT OR Apache-2.0
//! Eigen decomposition of a symmetric 3x3 matrix by cyclic Jacobi rotations.
//!
//! Each rotation zeroes one off-diagonal entry; sweeping the three pairs repeatedly converges
//! quadratically for symmetric matrices. Simple, and accurate to a few ulps on the sizes we use.

use super::vec3::Vec3;

/// Eigenvalues in ascending order with their unit eigenvectors (`vectors[i]` belongs to `values[i]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eigen3 {
    /// Eigenvalues, ascending.
    pub values: [f64; 3],
    /// Orthonormal eigenvectors, each with its largest-magnitude component positive.
    pub vectors: [Vec3; 3],
}

type M3 = [[f64; 3]; 3];

fn mul(a: &M3, b: &M3) -> M3 {
    let mut c = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            c[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    c
}

fn transpose(a: &M3) -> M3 {
    let mut t = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            t[i][j] = a[j][i];
        }
    }
    t
}

/// Decomposes the symmetric matrix `m` (only its upper triangle is read). `None` if an entry is not
/// finite or the iteration fails to converge.
pub fn symmetric_eigen3(m: [[f64; 3]; 3]) -> Option<Eigen3> {
    let mut a = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            a[i][j] = if j >= i { m[i][j] } else { m[j][i] };
            if !a[i][j].is_finite() {
                return None;
            }
        }
    }
    let mut v: M3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let scale = a.iter().flatten().fold(0.0f64, |s, x| s.max(x.abs()));
    if scale == 0.0 {
        return Some(finish(a, v));
    }
    for _sweep in 0..64 {
        let off = a[0][1] * a[0][1] + a[0][2] * a[0][2] + a[1][2] * a[1][2];
        if off <= (f64::EPSILON * scale).powi(2) * 1e-4 {
            return Some(finish(a, v));
        }
        for (p, q) in [(0, 1), (0, 2), (1, 2)] {
            let apq = a[p][q];
            if apq.abs() <= f64::MIN_POSITIVE {
                continue;
            }
            // Rotation angle that zeroes a[p][q]: t = tan(angle), the smaller root of t² + 2θt − 1 = 0.
            let theta = (a[q][q] - a[p][p]) / (2.0 * apq);
            let t = if theta.abs() > 1e150 {
                0.5 / theta
            } else {
                theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt())
            };
            let c = 1.0 / (t * t + 1.0).sqrt();
            let s = t * c;
            let mut j: M3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
            j[p][p] = c;
            j[q][q] = c;
            j[p][q] = s;
            j[q][p] = -s;
            a = mul(&transpose(&j), &mul(&a, &j));
            a[p][q] = 0.0;
            a[q][p] = 0.0;
            v = mul(&v, &j);
        }
    }
    None
}

fn finish(a: M3, v: M3) -> Eigen3 {
    let mut order = [0usize, 1, 2];
    order.sort_by(|&i, &j| a[i][i].total_cmp(&a[j][j]));
    let mut out = Eigen3 {
        values: [0.0; 3],
        vectors: [[0.0; 3]; 3],
    };
    for (slot, &k) in order.iter().enumerate() {
        out.values[slot] = a[k][k];
        let mut col = [v[0][k], v[1][k], v[2][k]];
        let big = (0..3)
            .max_by(|&i, &j| col[i].abs().total_cmp(&col[j].abs()))
            .unwrap_or(0);
        if col[big] < 0.0 {
            col = col.map(|x| -x);
        }
        out.vectors[slot] = col;
    }
    out
}
