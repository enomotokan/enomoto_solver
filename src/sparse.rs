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

/// A fixed-length, single-allocation ragged array of `(index, value)`
/// pairs — the CSR/CSC "offsets + flat entries" layout, kept generic over
/// which axis it indexes (row-major or column-major) since `simplex.rs`
/// builds one of each from the same triplets. Unlike `Vec<Vec<(usize,
/// f64)>>` (one heap allocation per outer row/column, each grown by
/// repeated `push`), this is exactly two allocations total — `offsets` and
/// `entries` — built once and never mutated afterward, which is the point:
/// `simplex.rs`'s `StdForm` is frozen the moment presolve hands it off (see
/// that struct's own docs), so there is no reason for its row/column
/// storage to still pay for growable `Vec`s.
#[derive(Clone)]
pub struct FixedRows {
    offsets: Vec<usize>,
    entries: Vec<(usize, f64)>,
}

impl FixedRows {
    /// Flattens `rows` (already grouped by outer index) into one `entries`
    /// buffer, `offsets[i]..offsets[i+1]` marking row `i`'s slice.
    pub fn from_rows(rows: &[Vec<(usize, f64)>]) -> Self {
        let mut offsets = Vec::with_capacity(rows.len() + 1);
        offsets.push(0);
        let mut entries = Vec::with_capacity(rows.iter().map(|r| r.len()).sum());
        for row in rows {
            entries.extend_from_slice(row);
            offsets.push(entries.len());
        }
        FixedRows { offsets, entries }
    }

    /// Builds the transpose of `rows` (`n_outer` = the column count of
    /// `rows`, i.e. this call's own outer index count) directly into the
    /// flat layout via a counting sort — one pass to size each outer
    /// index's slice, one pass to fill it — rather than transposing into
    /// `n_outer` separate growable `Vec`s first and flattening those
    /// second.
    pub fn from_transpose(rows: &[Vec<(usize, f64)>], n_outer: usize) -> Self {
        let mut counts = vec![0usize; n_outer];
        for row in rows {
            for &(j, _) in row {
                counts[j] += 1;
            }
        }
        let mut offsets = Vec::with_capacity(n_outer + 1);
        offsets.push(0);
        for &c in &counts {
            offsets.push(offsets.last().unwrap() + c);
        }
        let mut entries = vec![(0usize, 0.0f64); offsets[n_outer]];
        let mut cursor = offsets.clone();
        for (i, row) in rows.iter().enumerate() {
            for &(j, v) in row {
                entries[cursor[j]] = (i, v);
                cursor[j] += 1;
            }
        }
        FixedRows { offsets, entries }
    }

    /// Outer index `i`'s `(inner_index, value)` pairs — a plain slice into
    /// the shared flat buffer, no per-call allocation.
    #[inline]
    pub fn row(&self, i: usize) -> &[(usize, f64)] {
        &self.entries[self.offsets[i]..self.offsets[i + 1]]
    }
}

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
