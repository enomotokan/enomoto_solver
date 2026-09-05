//! Modified Ruiz equilibration: a diagonal preconditioner that rescales
//! variables and constraint rows so that `[A; G]`'s rows and columns have
//! roughly unit infinity-norm, improving the conditioning of both the KKT
//! systems `interior_point.rs` solves and the LU systems `simplex.rs`
//! factorizes. This is a one-time step per `presolve::run()` call (not per
//! iteration), computed on the original problem data before either
//! engine's main loop starts.
//!
//! The scaled problem is `x = D x'`, `A' = diag(e_a) A diag(d)`,
//! `b' = diag(e_a) b`, `G' = diag(e_g) G diag(d)`, `h' = diag(e_g) h`,
//! `c' = diag(d) c`. Solving the scaled problem and recovering
//! `x = diag(d) x'` gives the same solution as solving the original
//! problem directly (a diagonal reparametrization changes neither
//! feasibility nor boundedness).

//! **Parallelization**: every per-row and per-column update below is
//! independent of every other row/column *within the same iteration*
//! (only the iteration count `iters` is a genuine sequential dependency,
//! each round refining the previous one's `d`/`e_a`/`e_g`), so each round
//! parallelizes freely via rayon — except the column-norm accumulation,
//! which is a scatter-with-max: every row of `A`/`G` can touch any
//! column, so two rows racing to update the same `col_norm[j]` would be a
//! data race under naive per-row parallelism. That one instead uses
//! rayon's fold/reduce: each thread folds its own subset of rows into a
//! private length-`n` buffer (no cross-thread writes), and the buffers
//! are merged with an elementwise max at the end.

use rayon::prelude::*;

use crate::sparse::{csr_from_rows, Csr};

pub struct Scaling {
    pub d: Vec<f64>,
    pub e_a: Vec<f64>,
    pub e_g: Vec<f64>,
}

/// Elementwise max of two same-length buffers, consuming `a` as the
/// accumulator — the `reduce` half of the column-norm fold/reduce.
fn elementwise_max(mut a: Vec<f64>, b: Vec<f64>) -> Vec<f64> {
    for (x, y) in a.iter_mut().zip(&b) {
        *x = x.max(*y);
    }
    a
}

/// The per-row-subset half of the column-norm fold: folds `rows` of `mat`
/// (scaled by `d` and that row's own `e`) into a length-`n` buffer via
/// elementwise max. Shared by the `A` and `G` accumulations in `compute`.
fn col_norm_fold(mat: faer::sparse::SparseRowMatRef<usize, f64>, n: usize, d: &[f64], e: &[f64], rows: std::ops::Range<usize>) -> Vec<f64> {
    rows.into_par_iter()
        .fold(
            || vec![0.0f64; n],
            |mut acc, i| {
                for (j, &v) in mat.col_indices_of_row(i).zip(mat.values_of_row(i)) {
                    acc[j] = acc[j].max((v * d[j] * e[i]).abs());
                }
                acc
            },
        )
        .reduce(|| vec![0.0f64; n], elementwise_max)
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
        let col_norm_a = col_norm_fold(ar, n, &d, &e_a, 0..p);
        let col_norm_g = col_norm_fold(gr, n, &d, &e_g, 0..m);

        let mut col_norm = col_norm_a;
        col_norm
            .par_iter_mut()
            .zip(col_norm_g.par_iter())
            .zip(d.par_iter())
            .zip(c.par_iter())
            .for_each(|(((cn, &cg), &dj), &cj)| {
                *cn = cn.max(cg).max((cj * dj).abs());
            });

        d.par_iter_mut().zip(col_norm.par_iter()).for_each(|(dj, &cn)| {
            if cn > 1e-12 {
                *dj /= cn.sqrt();
            }
        });

        // Row-norm updates: row `i`'s own coefficients only, writing only
        // `e_a[i]`/`e_g[i]` — disjoint per row, safe to parallelize
        // directly (unlike the column-norm scatter above).
        e_a.par_iter_mut().enumerate().for_each(|(i, e)| {
            let old_e = *e;
            let mut row_norm = 0.0f64;
            for (j, &v) in ar.col_indices_of_row(i).zip(ar.values_of_row(i)) {
                row_norm = row_norm.max((v * d[j] * old_e).abs());
            }
            if row_norm > 1e-12 {
                *e = old_e / row_norm.sqrt();
            }
        });
        e_g.par_iter_mut().enumerate().for_each(|(i, e)| {
            let old_e = *e;
            let mut row_norm = 0.0f64;
            for (j, &v) in gr.col_indices_of_row(i).zip(gr.values_of_row(i)) {
                row_norm = row_norm.max((v * d[j] * old_e).abs());
            }
            if row_norm > 1e-12 {
                *e = old_e / row_norm.sqrt();
            }
        });
    }

    Scaling { d, e_a, e_g }
}

pub fn apply(scaling: &Scaling, a: &Csr, g: &Csr, b: &[f64], h: &[f64], c: &[f64]) -> (Csr, Csr, Vec<f64>, Vec<f64>, Vec<f64>) {
    let n = scaling.d.len();
    let p = a.nrows();
    let m = g.nrows();
    let ar = a.as_ref();
    let gr = g.as_ref();

    // Each row's rescaled entries are independent of every other row, so
    // both matrices are built one `Vec` of `(col, value)` pairs per row,
    // in parallel, then handed to `csr_from_rows` in one piece.
    let a_rows: Vec<Vec<(usize, f64)>> = (0..p)
        .into_par_iter()
        .map(|i| {
            ar.col_indices_of_row(i)
                .zip(ar.values_of_row(i))
                .map(|(j, &v)| (j, v * scaling.e_a[i] * scaling.d[j]))
                .collect()
        })
        .collect();
    let g_rows: Vec<Vec<(usize, f64)>> = (0..m)
        .into_par_iter()
        .map(|i| {
            gr.col_indices_of_row(i)
                .zip(gr.values_of_row(i))
                .map(|(j, &v)| (j, v * scaling.e_g[i] * scaling.d[j]))
                .collect()
        })
        .collect();

    let a_scaled = csr_from_rows(&a_rows, n);
    let g_scaled = csr_from_rows(&g_rows, n);
    let b_scaled: Vec<f64> = b.par_iter().zip(scaling.e_a.par_iter()).map(|(v, e)| v * e).collect();
    let h_scaled: Vec<f64> = h.par_iter().zip(scaling.e_g.par_iter()).map(|(v, e)| v * e).collect();
    let c_scaled: Vec<f64> = c.par_iter().zip(scaling.d.par_iter()).map(|(v, d)| v * d).collect();

    (a_scaled, g_scaled, b_scaled, h_scaled, c_scaled)
}

/// Recovers the original-problem solution `x = diag(d) x'` from a solve
/// performed on the scaled problem's `x'` — the inverse of what `apply`
/// did to the variables, applied once at the very end after either
/// engine's main loop converges (not needed at any point during the loop
/// itself, which works entirely in scaled space).
pub fn unscale_x(scaling: &Scaling, x: &[f64]) -> Vec<f64> {
    x.par_iter().zip(scaling.d.par_iter()).map(|(v, d)| v * d).collect()
}
