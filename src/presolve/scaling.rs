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

//! **Parallelization**: every per-row/per-column loop in this module
//! (`compute`'s column-norm accumulation and row/column normalization,
//! `apply`'s rescaling, `unscale_x`) is embarrassingly parallel in
//! principle, but all run sequentially — profiling on this crate's target
//! problem sizes (~1000 columns, a couple thousand rows across `A`/`G`)
//! found rayon's per-call dispatch overhead exceeding the arithmetic
//! itself at every one of these call sites (`compute`'s loop alone used
//! to make six rayon calls per Ruiz iteration, 60 total across the
//! default 10 iterations) — the same finding, and the same fix, as
//! `simplex.rs`'s per-pivot loop (see `solve_lp_dual_on`'s module docs).

use crate::sparse::{csr_from_rows, Csr};

pub struct Scaling {
    pub d: Vec<f64>,
    pub e_a: Vec<f64>,
    pub e_g: Vec<f64>,
}

/// The per-row-subset half of the column-norm fold: folds `rows` of `mat`
/// (scaled by `d` and that row's own `e`) into a length-`n` buffer via
/// elementwise max. Shared by the `A` and `G` accumulations in `compute`.
/// Sequential, not rayon — see `compute`'s own docs.
fn col_norm_fold(mat: faer::sparse::SparseRowMatRef<usize, f64>, d: &[f64], e: &[f64], rows: std::ops::Range<usize>, acc: &mut [f64]) {
    for i in rows {
        for (j, &v) in mat.col_indices_of_row(i).zip(mat.values_of_row(i)) {
            let cand = (v * d[j] * e[i]).abs();
            if cand > acc[j] {
                acc[j] = cand;
            }
        }
    }
}

pub fn compute(n: usize, a: &Csr, g: &Csr, c: &[f64], iters: usize) -> Scaling {
    let p = a.nrows();
    let m = g.nrows();
    let mut d = vec![1.0; n];
    let mut e_a = vec![1.0; p];
    let mut e_g = vec![1.0; m];

    let ar = a.as_ref();
    let gr = g.as_ref();

    let mut col_norm = vec![0.0f64; n];

    // Deliberately sequential, not rayon: profiling this pass on this
    // crate's target problem sizes (~1000 columns, a couple thousand
    // total rows across `A`/`G`, 10 Ruiz iterations) found that the six
    // rayon dispatches this loop used to make per iteration (two
    // fold/reduce column-norm accumulations plus four `par_iter_mut`
    // elementwise passes — 60 dispatches across all 10 iterations) cost
    // more in fixed per-call overhead than the actual arithmetic — same
    // finding, same fix, as the simplex per-iteration loop's own
    // `into_par_iter()` calls (see `simplex.rs`'s `solve_lp_dual_on`
    // module docs).
    for _ in 0..iters {
        col_norm.fill(0.0);
        col_norm_fold(ar, &d, &e_a, 0..p, &mut col_norm);
        col_norm_fold(gr, &d, &e_g, 0..m, &mut col_norm);
        for j in 0..n {
            let cj = (c[j] * d[j]).abs();
            if cj > col_norm[j] {
                col_norm[j] = cj;
            }
        }

        for j in 0..n {
            if col_norm[j] > 1e-12 {
                d[j] /= col_norm[j].sqrt();
            }
        }

        // Row-norm updates: row `i`'s own coefficients only, writing only
        // `e_a[i]`/`e_g[i]`.
        for (i, e) in e_a.iter_mut().enumerate() {
            let old_e = *e;
            let mut row_norm = 0.0f64;
            for (j, &v) in ar.col_indices_of_row(i).zip(ar.values_of_row(i)) {
                row_norm = row_norm.max((v * d[j] * old_e).abs());
            }
            if row_norm > 1e-12 {
                *e = old_e / row_norm.sqrt();
            }
        }
        for (i, e) in e_g.iter_mut().enumerate() {
            let old_e = *e;
            let mut row_norm = 0.0f64;
            for (j, &v) in gr.col_indices_of_row(i).zip(gr.values_of_row(i)) {
                row_norm = row_norm.max((v * d[j] * old_e).abs());
            }
            if row_norm > 1e-12 {
                *e = old_e / row_norm.sqrt();
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

    // Each row's rescaled entries are independent of every other row —
    // sequential nonetheless, per this module's own parallelization note.
    let a_rows: Vec<Vec<(usize, f64)>> = (0..p)
        .map(|i| {
            ar.col_indices_of_row(i)
                .zip(ar.values_of_row(i))
                .map(|(j, &v)| (j, v * scaling.e_a[i] * scaling.d[j]))
                .collect()
        })
        .collect();
    let g_rows: Vec<Vec<(usize, f64)>> = (0..m)
        .map(|i| {
            gr.col_indices_of_row(i)
                .zip(gr.values_of_row(i))
                .map(|(j, &v)| (j, v * scaling.e_g[i] * scaling.d[j]))
                .collect()
        })
        .collect();

    let a_scaled = csr_from_rows(&a_rows, n);
    let g_scaled = csr_from_rows(&g_rows, n);
    let b_scaled: Vec<f64> = b.iter().zip(&scaling.e_a).map(|(v, e)| v * e).collect();
    let h_scaled: Vec<f64> = h.iter().zip(&scaling.e_g).map(|(v, e)| v * e).collect();
    let c_scaled: Vec<f64> = c.iter().zip(&scaling.d).map(|(v, d)| v * d).collect();

    (a_scaled, g_scaled, b_scaled, h_scaled, c_scaled)
}

/// Recovers the original-problem solution `x = diag(d) x'` from a solve
/// performed on the scaled problem's `x'` — the inverse of what `apply`
/// did to the variables, applied once at the very end after either
/// engine's main loop converges (not needed at any point during the loop
/// itself, which works entirely in scaled space).
pub fn unscale_x(scaling: &Scaling, x: &[f64]) -> Vec<f64> {
    x.iter().zip(&scaling.d).map(|(v, d)| v * d).collect()
}
