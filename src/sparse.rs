//! Shared sparse-matrix plumbing: a thin CSR alias plus the handful of
//! allocation-conscious helpers built on it. Used by both the KKT assembly
//! in `interior_point/kkt.rs` (native CSR all the way into the Cholesky
//! solve) and the shared presolve pipeline (`presolve.rs`) that now feeds
//! both the simplex and interior-point engines — putting it here, rather
//! than inside `interior_point`, is what lets `presolve.rs` and `simplex.rs`
//! use it without depending on the (otherwise inactive) interior-point
//! module tree.
//!
//! **Parallelization**: `mat_vec_into` writes one output entry per row,
//! independently, so it parallelizes via a plain `par_iter_mut` over the
//! already-allocated `out` slice — no allocation inside the call, which
//! matters since `interior_point.rs`'s Newton loop calls it every
//! iteration and its own docs promise that loop never allocates.
//! `mat_t_vec_into`, by contrast, is a *scatter* over `out` (every row can
//! touch any column of the transpose), which would need either atomics or
//! a fold/reduce with a fresh per-thread buffer — the latter being exactly
//! the allocation this function is called from a no-allocation loop to
//! avoid — so it stays sequential.

use rayon::prelude::*;

pub type Csr = faer::sparse::SparseRowMat<usize, f64>;

/// Builds a `Csr` from a dense list of sparse rows (each a `(col, value)`
/// list), dropping exact-zero entries. `n_cols` is the matrix's column
/// count; the row count is `rows.len()`.
pub fn csr_from_rows(rows: &[Vec<(usize, f64)>], n_cols: usize) -> Csr {
    let mut triplets = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 {
                triplets.push((i, j, v));
            }
        }
    }
    Csr::try_new_from_triplets(rows.len(), n_cols, &triplets).expect("valid CSR triplets")
}

/// Writes `mat * x` into `out` (length `mat.nrows()`). No allocation.
pub fn mat_vec_into(mat: &Csr, x: &[f64], out: &mut [f64]) {
    let r = mat.as_ref();
    out.par_iter_mut().enumerate().for_each(|(i, o)| {
        *o = r.col_indices_of_row(i).zip(r.values_of_row(i)).map(|(j, &v)| v * x[j]).sum();
    });
}

/// Writes `mat^T * y` into `out` (length `mat.ncols()`). No allocation.
pub fn mat_t_vec_into(mat: &Csr, y: &[f64], out: &mut [f64]) {
    for v in out.iter_mut() {
        *v = 0.0;
    }
    let r = mat.as_ref();
    for i in 0..r.nrows() {
        let yi = y[i];
        if yi == 0.0 {
            continue;
        }
        for (j, &v) in r.col_indices_of_row(i).zip(r.values_of_row(i)) {
            out[j] += v * yi;
        }
    }
}

/// Allocating wrapper around `mat_vec_into` — `mat * x` as a fresh `Vec`.
pub fn mat_vec(mat: &Csr, x: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; mat.nrows()];
    mat_vec_into(mat, x, &mut out);
    out
}

/// Allocating wrapper around `mat_t_vec_into` — `mat^T * y` as a fresh `Vec`.
pub fn mat_t_vec(mat: &Csr, n_cols: usize, y: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; n_cols];
    mat_t_vec_into(mat, y, &mut out);
    out
}
