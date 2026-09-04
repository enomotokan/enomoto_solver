//! Modified Ruiz equilibration: a diagonal preconditioner that rescales
//! variables and constraint rows so that `[A; G]`'s rows and columns have
//! roughly unit infinity-norm, improving the conditioning of the KKT
//! systems `ipm.rs` solves. This is a one-time step per `ipm::solve()`
//! call (not per iteration), computed on the original problem data before
//! the interior-point loop starts.
//!
//! The scaled problem is `x = D x'`, `A' = diag(e_a) A diag(d)`,
//! `b' = diag(e_a) b`, `G' = diag(e_g) G diag(d)`, `h' = diag(e_g) h`,
//! `c' = diag(d) c`. Solving the scaled problem and recovering
//! `x = diag(d) x'` gives the same solution as solving the original
//! problem directly (a diagonal reparametrization changes neither
//! feasibility nor boundedness).

use super::kkt::{csr_from_rows, Csr};

pub struct Scaling {
    pub d: Vec<f64>,
    pub e_a: Vec<f64>,
    pub e_g: Vec<f64>,
}

pub fn compute(n: usize, a: &Csr, g: &Csr, c: &[f64], iters: usize) -> Scaling {
    let p = a.nrows();
    let m = g.nrows();
    let mut d = vec![1.0; n];
    let mut e_a = vec![1.0; p];
    let mut e_g = vec![1.0; m];

    let ar = a.as_ref();
    let gr = g.as_ref();

    for _ in 0..iters {
        let mut col_norm = vec![0.0f64; n];
        for i in 0..p {
            for (j, &v) in ar.col_indices_of_row(i).zip(ar.values_of_row(i)) {
                col_norm[j] = col_norm[j].max((v * d[j] * e_a[i]).abs());
            }
        }
        for i in 0..m {
            for (j, &v) in gr.col_indices_of_row(i).zip(gr.values_of_row(i)) {
                col_norm[j] = col_norm[j].max((v * d[j] * e_g[i]).abs());
            }
        }
        for j in 0..n {
            col_norm[j] = col_norm[j].max((c[j] * d[j]).abs());
        }
        for j in 0..n {
            if col_norm[j] > 1e-12 {
                d[j] /= col_norm[j].sqrt();
            }
        }

        for i in 0..p {
            let mut row_norm = 0.0f64;
            for (j, &v) in ar.col_indices_of_row(i).zip(ar.values_of_row(i)) {
                row_norm = row_norm.max((v * d[j] * e_a[i]).abs());
            }
            if row_norm > 1e-12 {
                e_a[i] /= row_norm.sqrt();
            }
        }
        for i in 0..m {
            let mut row_norm = 0.0f64;
            for (j, &v) in gr.col_indices_of_row(i).zip(gr.values_of_row(i)) {
                row_norm = row_norm.max((v * d[j] * e_g[i]).abs());
            }
            if row_norm > 1e-12 {
                e_g[i] /= row_norm.sqrt();
            }
        }
    }

    Scaling { d, e_a, e_g }
}

pub fn apply(scaling: &Scaling, a: &Csr, g: &Csr, b: &[f64], h: &[f64], c: &[f64]) -> (Csr, Csr, Vec<f64>, Vec<f64>, Vec<f64>) {
    let n = scaling.d.len();
    let p = a.nrows();
    let m = g.nrows();
    let ar = a.as_ref();
    let gr = g.as_ref();

    let mut a_rows = vec![Vec::new(); p];
    for i in 0..p {
        for (j, &v) in ar.col_indices_of_row(i).zip(ar.values_of_row(i)) {
            a_rows[i].push((j, v * scaling.e_a[i] * scaling.d[j]));
        }
    }
    let mut g_rows = vec![Vec::new(); m];
    for i in 0..m {
        for (j, &v) in gr.col_indices_of_row(i).zip(gr.values_of_row(i)) {
            g_rows[i].push((j, v * scaling.e_g[i] * scaling.d[j]));
        }
    }

    let a_scaled = csr_from_rows(&a_rows, n);
    let g_scaled = csr_from_rows(&g_rows, n);
    let b_scaled: Vec<f64> = b.iter().zip(&scaling.e_a).map(|(v, e)| v * e).collect();
    let h_scaled: Vec<f64> = h.iter().zip(&scaling.e_g).map(|(v, e)| v * e).collect();
    let c_scaled: Vec<f64> = c.iter().zip(&scaling.d).map(|(v, d)| v * d).collect();

    (a_scaled, g_scaled, b_scaled, h_scaled, c_scaled)
}

/// Recovers the original-problem solution `x = diag(d) x'` from a solve
/// performed on the scaled problem's `x'` — the inverse of what `apply`
/// did to the variables, applied once at the very end after the
/// interior-point loop converges (not needed at any point during the
/// loop itself, which works entirely in scaled space).
pub fn unscale_x(scaling: &Scaling, x: &[f64]) -> Vec<f64> {
    x.iter().zip(&scaling.d).map(|(v, d)| v * d).collect()
}
