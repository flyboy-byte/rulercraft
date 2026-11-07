// SPDX-License-Identifier: MIT OR Apache-2.0
//! Small dense Levenberg-Marquardt for nonlinear least squares with a handful of parameters.
//!
//! Minimises Σ rᵢ(x)² by solving (JᵀJ + λ·diag(JᵀJ)) δ = −Jᵀr each step, shrinking λ after a step
//! that lowers the cost and growing it after one that does not.

/// Iteration controls.
#[derive(Debug, Clone, Copy)]
pub struct LmOptions {
    /// Most accepted-or-rejected steps.
    pub max_iterations: usize,
    /// Stop when the step is smaller than this relative to the parameters.
    pub step_tolerance: f64,
    /// Stop when the cost improves by less than this fraction.
    pub cost_tolerance: f64,
    /// Stop when the residual is this close to orthogonal to every Jacobian column (the largest
    /// |cos| between them). Scale-free: it reads the same for a part in mm or in inches.
    pub gradient_tolerance: f64,
}

impl Default for LmOptions {
    fn default() -> Self {
        Self {
            max_iterations: 200,
            step_tolerance: 1e-12,
            cost_tolerance: 1e-15,
            gradient_tolerance: 1e-10,
        }
    }
}

/// The outcome of a solve.
#[derive(Debug, Clone, PartialEq)]
pub struct LmReport {
    /// Best parameters found.
    pub x: Vec<f64>,
    /// Sum of squared residuals at `x`.
    pub cost: f64,
    /// Steps taken.
    pub iterations: usize,
    /// Whether a tolerance was met (otherwise the iteration limit or a stall stopped it).
    pub converged: bool,
}

/// Solves `a x = b` for a small dense system by Gaussian elimination with partial pivoting.
pub fn solve_dense(mut a: Vec<f64>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    if a.len() != n * n {
        return None;
    }
    for col in 0..n {
        let pivot =
            (col..n).max_by(|&i, &j| a[i * n + col].abs().total_cmp(&a[j * n + col].abs()))?;
        if a[pivot * n + col].abs() < 1e-300 || !a[pivot * n + col].is_finite() {
            return None;
        }
        if pivot != col {
            for k in 0..n {
                a.swap(pivot * n + k, col * n + k);
            }
            b.swap(pivot, col);
        }
        for row in col + 1..n {
            let f = a[row * n + col] / a[col * n + col];
            for k in col..n {
                a[row * n + k] -= f * a[col * n + k];
            }
            b[row] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let mut s = b[row];
        for k in row + 1..n {
            s -= a[row * n + k] * x[k];
        }
        x[row] = s / a[row * n + row];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

/// Minimises the sum of squares of `m` residuals over the parameters, starting at `x0`.
///
/// `eval(x, residuals, jacobian)` fills `residuals` (length `m`) and, when given, `jacobian`
/// (row-major `m × x0.len()`, ∂rᵢ/∂xⱼ), and returns `false` when `x` is outside the model's domain.
/// Returns `None` if `x0` itself cannot be evaluated or `m < x0.len()`.
pub fn levenberg_marquardt<F>(
    x0: &[f64],
    m: usize,
    options: LmOptions,
    mut eval: F,
) -> Option<LmReport>
where
    F: FnMut(&[f64], &mut [f64], Option<&mut [f64]>) -> bool,
{
    let n = x0.len();
    if n == 0 || m < n {
        return None;
    }
    let mut x = x0.to_vec();
    let mut r = vec![0.0; m];
    let mut jac = vec![0.0; m * n];
    if !eval(&x, &mut r, Some(&mut jac)) {
        return None;
    }
    let mut cost: f64 = r.iter().map(|v| v * v).sum();
    if !cost.is_finite() {
        return None;
    }
    let mut lambda = 1e-3;
    let mut trial_r = vec![0.0; m];
    for iteration in 1..=options.max_iterations {
        let mut jtj = vec![0.0; n * n];
        let mut jtr = vec![0.0; n];
        for i in 0..m {
            let row = &jac[i * n..(i + 1) * n];
            for a in 0..n {
                jtr[a] += row[a] * r[i];
                for b in 0..n {
                    jtj[a * n + b] += row[a] * row[b];
                }
            }
        }
        let orthogonality = max_cosine(&jtj, &jtr, cost, n);
        if cost == 0.0 || orthogonality <= options.gradient_tolerance {
            return Some(LmReport {
                x,
                cost,
                iterations: iteration - 1,
                converged: true,
            });
        }
        let mut damped = jtj.clone();
        for a in 0..n {
            damped[a * n + a] += lambda * jtj[a * n + a].max(1e-12);
        }
        let step = solve_dense(damped, jtr.iter().map(|g| -g).collect());
        let candidate: Option<Vec<f64>> = step
            .as_ref()
            .map(|d| x.iter().zip(d).map(|(a, b)| a + b).collect());
        let improved = match &candidate {
            Some(c) if eval(c, &mut trial_r, None) => {
                let c_cost: f64 = trial_r.iter().map(|v| v * v).sum();
                c_cost.is_finite() && c_cost < cost
            }
            _ => false,
        };
        if improved {
            let (Some(c), Some(d)) = (candidate, step) else {
                return None;
            };
            let new_cost: f64 = trial_r.iter().map(|v| v * v).sum();
            let step_norm = d.iter().map(|v| v * v).sum::<f64>().sqrt();
            let x_norm = c.iter().map(|v| v * v).sum::<f64>().sqrt();
            let small_step =
                step_norm <= options.step_tolerance * (x_norm + options.step_tolerance);
            let small_gain = cost - new_cost <= options.cost_tolerance * cost;
            x = c;
            cost = new_cost;
            if !eval(&x, &mut r, Some(&mut jac)) {
                return None;
            }
            lambda = (lambda / 10.0).max(1e-15);
            if small_step || small_gain {
                return Some(LmReport {
                    x,
                    cost,
                    iterations: iteration,
                    converged: true,
                });
            }
        } else {
            lambda *= 10.0;
            if lambda > 1e16 {
                // No step lowers the cost any more. That is the optimum, to working precision, when
                // the residual is (nearly) orthogonal to the Jacobian; otherwise it is a real stall.
                return Some(LmReport {
                    x,
                    cost,
                    iterations: iteration,
                    converged: orthogonality <= 1e-6,
                });
            }
        }
    }
    Some(LmReport {
        x,
        cost,
        iterations: options.max_iterations,
        converged: false,
    })
}

/// The largest |cos| between the residual vector and a Jacobian column: |(Jᵀr)ⱼ| / (‖Jⱼ‖ ‖r‖).
fn max_cosine(jtj: &[f64], jtr: &[f64], cost: f64, n: usize) -> f64 {
    let r = cost.sqrt();
    if r == 0.0 {
        return 0.0;
    }
    (0..n)
        .map(|j| {
            let col = jtj[j * n + j].sqrt();
            if col == 0.0 {
                0.0
            } else {
                jtr[j].abs() / (col * r)
            }
        })
        .fold(0.0, f64::max)
}
