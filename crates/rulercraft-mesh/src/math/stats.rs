// SPDX-License-Identifier: MIT OR Apache-2.0
//! Centroid and covariance of point sets.

use super::vec3::Vec3;

/// Mean of the points, or `None` for an empty set.
pub fn centroid(points: impl IntoIterator<Item = Vec3>) -> Option<Vec3> {
    let mut sum = [0.0; 3];
    let mut n = 0usize;
    for p in points {
        for k in 0..3 {
            sum[k] += p[k];
        }
        n += 1;
    }
    (n > 0).then(|| sum.map(|s| s / n as f64))
}

/// A centroid with the 3x3 covariance (divided by n) of the points about it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Covariance {
    /// Mean of the points.
    pub centroid: Vec3,
    /// Row-major symmetric covariance matrix.
    pub matrix: [[f64; 3]; 3],
    /// Number of points.
    pub count: usize,
}

/// Two-pass covariance: the centroid first, then products of offsets from it, so points far from
/// the origin (a part placed at 10^6 mm) keep their precision. `None` for an empty set.
pub fn covariance<I>(points: I) -> Option<Covariance>
where
    I: IntoIterator<Item = Vec3>,
    I::IntoIter: Clone,
{
    let iter = points.into_iter();
    let c = centroid(iter.clone())?;
    let mut m = [[0.0; 3]; 3];
    let mut n = 0usize;
    for p in iter {
        let d = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
        for i in 0..3 {
            for j in i..3 {
                m[i][j] += d[i] * d[j];
            }
        }
        n += 1;
    }
    for i in 0..3 {
        for j in i..3 {
            m[i][j] /= n as f64;
            m[j][i] = m[i][j];
        }
    }
    Some(Covariance {
        centroid: c,
        matrix: m,
        count: n,
    })
}
