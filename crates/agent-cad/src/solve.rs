//! Bounded deterministic Levenberg–Marquardt with numerical rank diagnostics.
//! This solver reports local rank; it never claims a globally unique solution.
use crate::*;
#[derive(Debug, Serialize)]
pub struct Solution {
    pub values: Vec<f64>,
    pub iterations: usize,
    pub rank: usize,
    pub degrees_of_freedom: usize,
    pub redundant_equations: usize,
    pub max_residual: f64,
}
fn max_abs(a: &[f64]) -> f64 {
    a.iter().fold(0_f64, |m, v| m.max(v.abs()))
}
fn squared(a: &[f64]) -> f64 {
    a.iter().map(|v| v * v).sum()
}
fn jacobian(
    x: &[f64],
    f: &impl Fn(&[f64]) -> Result<Vec<f64>>,
    rows: usize,
) -> Result<Vec<Vec<f64>>> {
    let mut j = vec![vec![0.; x.len()]; rows];
    let mut q = x.to_vec();
    for c in 0..x.len() {
        let h = 1e-5 * (1. + x[c].abs()).sqrt();
        q[c] = x[c] + h;
        let a = f(&q)?;
        q[c] = x[c] - h;
        let b = f(&q)?;
        q[c] = x[c];
        if a.len() != rows || b.len() != rows {
            return Err(CadError::new("SOLVER_ERROR", "residual size changed"));
        }
        for r in 0..rows {
            j[r][c] = (a[r] - b[r]) / (2. * h);
        }
    }
    Ok(j)
}
fn linear(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for c in 0..n {
        let pivot = (c..n).max_by(|&i, &j| a[i][c].abs().total_cmp(&a[j][c].abs()))?;
        if a[pivot][c].abs() < 1e-24 {
            return None;
        }
        a.swap(c, pivot);
        b.swap(c, pivot);
        let pivot_row = a[c].clone();
        for r in c + 1..n {
            let s = a[r][c] / a[c][c];
            for (value, pivot) in a[r].iter_mut().zip(&pivot_row).skip(c + 1) {
                *value -= s * pivot;
            }
            b[r] -= s * b[c];
        }
    }
    let mut x = vec![0.; n];
    for c in (0..n).rev() {
        x[c] = (b[c] - (c + 1..n).map(|j| a[c][j] * x[j]).sum::<f64>()) / a[c][c];
    }
    Some(x)
}
fn rank(mut a: Vec<Vec<f64>>, cols: usize) -> usize {
    if a.is_empty() {
        return 0;
    }
    let scale = a.iter().flatten().fold(0_f64, |m, v| m.max(v.abs()));
    let mut r = 0;
    for c in 0..cols {
        let Some(pivot) = (r..a.len()).max_by(|&i, &j| a[i][c].abs().total_cmp(&a[j][c].abs()))
        else {
            break;
        };
        if a[pivot][c].abs() <= scale * 1e-7 {
            continue;
        }
        a.swap(r, pivot);
        let pivot_row = a[r].clone();
        for row in a.iter_mut().skip(r + 1) {
            let s = row[c] / pivot_row[c];
            for (value, pivot) in row.iter_mut().zip(&pivot_row).skip(c + 1) {
                *value -= s * pivot;
            }
        }
        r += 1;
    }
    r
}
pub fn least_squares(
    mut x: Vec<f64>,
    f: impl Fn(&[f64]) -> Result<Vec<f64>>,
    tolerance: f64,
) -> Result<Solution> {
    if x.is_empty() || x.len() > 128 || !x.iter().all(|x| x.is_finite()) {
        return Err(CadError::new(
            "SOLVER_BUDGET",
            "requires 1..128 finite variables",
        ));
    }
    let mut residual = f(&x)?;
    if residual.len() > 512 {
        return Err(CadError::new(
            "SOLVER_BUDGET",
            "at most 512 residual equations",
        ));
    }
    let mut lambda = 1e-3;
    let mut iterations = 0;
    for i in 0..160 {
        iterations = i;
        if !residual.iter().all(|x| x.is_finite()) {
            return Err(CadError::new("SOLVER_DEGENERATE", "nonfinite residual"));
        }
        if max_abs(&residual) <= tolerance {
            break;
        }
        let j = jacobian(&x, &f, residual.len())?;
        let n = x.len();
        let mut a = vec![vec![0.; n]; n];
        let mut g = vec![0.; n];
        for r in 0..residual.len() {
            for c in 0..n {
                g[c] -= j[r][c] * residual[r];
                for d in 0..n {
                    a[c][d] += j[r][c] * j[r][d];
                }
            }
        }
        for (c, row) in a.iter_mut().enumerate() {
            row[c] += lambda * row[c].max(1.);
        }
        let Some(step) = linear(a, g) else {
            lambda *= 10.;
            continue;
        };
        let candidate = x.iter().zip(step).map(|(a, b)| a + b).collect::<Vec<_>>();
        if !candidate.iter().all(|x| x.is_finite() && x.abs() <= 1e6) {
            lambda *= 10.;
            continue;
        }
        let next = f(&candidate)?;
        if squared(&next) < squared(&residual) {
            x = candidate;
            residual = next;
            lambda = (lambda / 3.).max(1e-12);
        } else {
            lambda = (lambda * 10.).min(1e16);
        }
    }
    let rank = rank(jacobian(&x, &f, residual.len())?, x.len());
    Ok(Solution {
        degrees_of_freedom: x.len() - rank,
        redundant_equations: residual.len().saturating_sub(rank),
        rank,
        iterations,
        max_residual: max_abs(&residual),
        values: x,
    })
}
