//! Removes redundant equality-constraint rows from `(A, b)`, run once (on
//! the Ruiz-scaled problem) before either engine's main loop starts —
//! shared by `interior_point.rs` and `simplex.rs` via `presolve::run`.
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
//!
//! **Parallelization**: extracting each row's coefficients out of the CSR
//! `A`/`G` (below) is independent per row, but runs sequentially rather
//! than via rayon — profiling on this crate's target problem sizes found
//! rayon's per-call dispatch overhead exceeding the cost of this simple
//! scan (the same finding as `scaling.rs`'s and `simplex.rs`'s own
//! per-iteration loops; see `simplex.rs`'s `solve_lp_dual_on` module
//! docs). Step 1 (`dedupe_rows`) is a single sequential scan over a
//! shared `HashSet` by design regardless — which duplicate of an equal
//! pair survives depends on scan order, so parallelizing it would make
//! that choice (immaterial to correctness, since the kept row is an
//! exact/scalar-multiple of the dropped one either way) nondeterministic
//! between runs. Step 2's dense QR is faer-internal and, per the module
//! docs above, already a cheap one-time cost given `p` is expected to be
//! small.

use std::collections::HashMap;
use std::collections::HashSet;

use faer::linalg::solvers::ColPivQr;
use faer::Mat;

use crate::sparse::{csr_from_rows, Csr};

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

/// Removes duplicate / positive-scalar-multiple rows from `(G, h)` —
/// PaPILO's "ParallelRows" (Achterberg et al. 2019, §4.4), the `<=`-sense
/// analogue of [`reduce_equalities`]'s duplicate detection (step 1 only —
/// there is no inequality analogue of step 2's rank-revealing QR: a
/// *positive* combination of several `<=` rows can imply another one, but
/// detecting that in general is Fourier-Motzkin elimination, well beyond
/// a cheap presolve pass, so only pairwise duplicates are caught here).
///
/// Sign matters here in a way it doesn't for equalities: `a.x <= h` and
/// `(-a).x <= h'` are *not* the same constraint (that would be
/// `a.x >= -h'`), so a row is normalized by dividing by `|row[0].1]`
/// (never flipping any sign) rather than by the signed first coefficient
/// the way `dedupe_rows` does. When two rows normalize to the identical
/// coefficient pattern, they bound the same linear combination from
/// above and only the tighter (smaller normalized `h`) is kept.
pub fn reduce_inequalities(g: &Csr, h: &[f64], n: usize) -> (Csr, Vec<f64>) {
    let m = g.nrows();
    if m == 0 {
        return (csr_from_rows(&[], n), Vec::new());
    }

    let gr = g.as_ref();
    let rows: Vec<(Vec<(usize, f64)>, f64)> = (0..m)
        .map(|i| {
            let row: Vec<(usize, f64)> = gr.col_indices_of_row(i).zip(gr.values_of_row(i)).map(|(j, &v)| (j, v)).collect();
            (row, h[i])
        })
        .collect();

    // (normalized sig) -> (index into `rows` currently kept, its normalized h)
    let mut best: HashMap<Vec<(usize, u64)>, (usize, f64)> = HashMap::new();
    let mut keep = vec![true; m];
    for (idx, (row, hv)) in rows.iter().enumerate() {
        if row.is_empty() {
            // `0 <= h`: either always true (drop) or a certificate of
            // infeasibility (`propagate`'s activity check catches that) —
            // neither is a "duplicate" in the sense this pass looks for.
            continue;
        }
        let scale = row[0].1.abs();
        let inv = 1.0 / scale;
        let sig: Vec<(usize, u64)> = row.iter().map(|&(j, v)| (j, (v * inv).to_bits())).collect();
        let normalized_h = hv * inv;
        match best.get_mut(&sig) {
            None => {
                best.insert(sig, (idx, normalized_h));
            }
            Some((kept_idx, kept_h)) => {
                if normalized_h < *kept_h {
                    keep[*kept_idx] = false;
                    *kept_idx = idx;
                    *kept_h = normalized_h;
                } else {
                    keep[idx] = false;
                }
            }
        }
    }

    let mut new_rows = Vec::new();
    let mut new_h = Vec::new();
    for (idx, (row, hv)) in rows.into_iter().enumerate() {
        if keep[idx] {
            new_rows.push(row);
            new_h.push(hv);
        }
    }
    (csr_from_rows(&new_rows, n), new_h)
}
