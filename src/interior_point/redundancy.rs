//! Removes redundant equality-constraint rows from `(A, b)`, run once (on
//! the Ruiz-scaled problem) before the interior-point loop starts.
//!
//! Two passes:
//!  1. **Direct duplicate detection**: a row that is an exact or
//!     scalar-multiple duplicate of an already-kept row (same coefficient
//!     pattern up to one scalar, including the right-hand side) is dropped.
//!     This is a cheap hash comparison, no linear algebra.
//!  2. **Rank-revealing QR**: on whatever survives step 1, build the dense
//!     `n x p` matrix `A^T` and run a column-pivoted QR
//!     (`faer::linalg::solvers::ColPivQr`, `A^T P^T = QR`). `R`'s diagonal
//!     reveals rank: a pivot position with `|R[k,k]|` negligible relative
//!     to the leading pivot means the corresponding original row (via the
//!     column permutation) is a linear combination of the others, so it is
//!     dropped. `p` is expected to be small relative to `n` for realistic
//!     LPs, so a dense factorization here is a cheap one-time cost.

use std::collections::HashSet;

use faer::linalg::solvers::ColPivQr;
use faer::Mat;

use super::kkt::{csr_from_rows, Csr};

/// Returns a reduced `(A, b)` with duplicate/linearly-dependent equality
/// rows removed.
pub fn reduce_equalities(a: &Csr, b: &[f64], n: usize) -> (Csr, Vec<f64>) {
    let p = a.nrows();
    if p == 0 {
        return (csr_from_rows(&[], n), Vec::new());
    }

    let ar = a.as_ref();
    let rows: Vec<(Vec<(usize, f64)>, f64)> = (0..p)
        .map(|i| {
            let row: Vec<(usize, f64)> = ar
                .col_indices_of_row(i)
                .zip(ar.values_of_row(i))
                .map(|(j, &v)| (j, v))
                .collect();
            (row, b[i])
        })
        .collect();

    let deduped = dedupe_rows(rows);
    let keep = drop_linearly_dependent(&deduped, n);

    let mut new_rows = Vec::with_capacity(keep.len());
    let mut new_b = Vec::with_capacity(keep.len());
    for &idx in &keep {
        new_rows.push(deduped[idx].0.clone());
        new_b.push(deduped[idx].1);
    }
    (csr_from_rows(&new_rows, n), new_b)
}

/// Step 1: drops exact or scalar-multiple duplicate rows, by normalizing
/// each row (and its RHS) by its first coefficient and hashing the bit
/// pattern of the result. A structurally empty row (`0 = rhs`) is dropped
/// outright when `rhs` is also (bit-exactly) zero — a trivially redundant
/// `0 = 0` row; a nonzero RHS on an empty row is kept so the Farkas
/// infeasibility certificate downstream still sees (and reports) it.
fn dedupe_rows(rows: Vec<(Vec<(usize, f64)>, f64)>) -> Vec<(Vec<(usize, f64)>, f64)> {
    let mut seen: HashSet<Vec<(usize, u64)>> = HashSet::new();
    let mut kept = Vec::with_capacity(rows.len());
    for (row, rhs) in rows {
        if row.is_empty() {
            if rhs != 0.0 {
                kept.push((row, rhs));
            }
            continue;
        }
        let pivot = row[0].1;
        let inv = 1.0 / pivot;
        let mut sig: Vec<(usize, u64)> = row.iter().map(|&(j, v)| (j, (v * inv).to_bits())).collect();
        sig.push((usize::MAX, (rhs * inv).to_bits()));
        if seen.insert(sig) {
            kept.push((row, rhs));
        }
    }
    kept
}

/// Step 2: returns the indices into `rows` of a maximal linearly
/// independent subset, found via column-pivoted QR of the dense `n x p`
/// matrix whose columns are the rows of `rows` (i.e. the transpose).
fn drop_linearly_dependent(rows: &[(Vec<(usize, f64)>, f64)], n: usize) -> Vec<usize> {
    let p = rows.len();
    if p == 0 {
        return Vec::new();
    }

    // Columns are [row coefficients ; rhs] (n+1 entries), not just the
    // coefficients: a row whose *coefficients* are a linear combination of
    // other rows' coefficients but whose *rhs* breaks that same
    // combination is an inconsistency (the system is infeasible), not
    // redundancy, and must not be dropped here — appending rhs as an extra
    // coordinate makes such a row linearly independent in the augmented
    // sense, so QR correctly keeps it (the existing Farkas-certificate
    // infeasibility detection downstream is what reports it).
    let aug_n = n + 1;
    let mut m = Mat::<f64>::zeros(aug_n, p);
    for (i, (row, rhs)) in rows.iter().enumerate() {
        for &(j, v) in row {
            m[(j, i)] = v;
        }
        m[(n, i)] = *rhs;
    }

    let qr = ColPivQr::new(m.as_ref());
    let r = qr.compute_thin_r(); // min(aug_n, p) x p
    let perm = qr.col_permutation();
    let (fwd, _inv) = perm.arrays();

    let rank_dim = r.nrows().min(r.ncols());
    let r_max = (0..rank_dim).map(|k| r[(k, k)].abs()).fold(0.0_f64, f64::max);
    let tol = 1e-10 * (aug_n.max(p) as f64) * r_max.max(1.0);

    let mut keep = vec![true; p];
    for k in 0..rank_dim {
        if r[(k, k)].abs() <= tol {
            keep[fwd[k]] = false;
        }
    }
    // If p > aug_n, there can be at most aug_n independent rows: every
    // pivot position beyond rank_dim never received a diagonal entry at
    // all, so its row is necessarily redundant too.
    for &orig in &fwd[rank_dim..] {
        keep[orig] = false;
    }

    (0..p).filter(|&i| keep[i]).collect()
}
