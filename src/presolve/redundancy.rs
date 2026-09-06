//! Removes redundant equality-constraint rows from `(A, b)`, run once (on
//! the Ruiz-scaled problem) before either engine's main loop starts —
//! shared by `interior_point.rs` and `simplex.rs` via `presolve::run_extended`.
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

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

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

    // Dependency is judged per row, relative to that row's *own* norm:
    // `|R[k,k]|` is exactly the residual norm of pivot column `k` after
    // projecting out every earlier pivot, so a row is (numerically) a
    // combination of the rows chosen before it iff that residual is
    // negligible *compared to the row itself*. An earlier version used one
    // global threshold, `1e-10 * max(n+1, p) * max_k |R[k,k]|` — on a
    // problem with ~1600 columns and a large appended-rhs coordinate that
    // came to ~1e-2 in absolute terms, and it dropped equality rows whose
    // genuine independent component was of that size (confirmed on Netlib
    // `modszk1`/`ganges`: the solves then ended at points *violating* the
    // dropped rows by ~1e-2, with objectives "better" than the true
    // optimum). A tiny absolute floor still catches exact zeros.
    let col_norm = |i: usize| -> f64 {
        let (row, rhs) = &rows[i];
        (row.iter().map(|&(_, v)| v * v).sum::<f64>() + rhs * rhs).sqrt()
    };
    let mut keep = vec![true; p];
    for k in 0..rank_dim {
        let orig = fwd[k];
        if r[(k, k)].abs() <= 1e-9 * col_norm(orig).max(1e-300) {
            keep[orig] = false;
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

/// Numerical-stability floor for the pivot chosen at each elimination
/// step (relative to the still-active *column*'s own current max
/// magnitude) — this is a different, narrower purpose than `DEP_TOL`
/// below: it only guards against dividing by an unreasonably tiny number
/// during the elimination arithmetic itself, the same role `simplex::lu`'s
/// own `STABILITY` constant plays for the (unrelated) basis factorization.
const PIVOT_STABILITY: f64 = 0.1;
/// Dependency floor, relative to a row's own *original* norm (computed
/// once, before any elimination) — this is the actual redundancy
/// criterion, directly analogous to `drop_linearly_dependent`'s own
/// `1e-9 * col_norm(orig)` test against `|R[k,k]|` (see that function's
/// docs for why a per-row-relative, not global, threshold matters).
const DEP_TOL: f64 = 1e-9;

/// Sparse, elimination-based alternative to [`drop_linearly_dependent`]:
/// identical contract (same inputs, same "maximal linearly independent
/// subset, by index into `rows`" output, same per-row-relative dependency
/// criterion) via sparse Gaussian elimination with degree-bucketed,
/// fill-minimizing pivot selection — the same technique `simplex::lu`'s
/// Markowitz factorization already uses for the simplex basis — instead
/// of densifying an `(n+1) x p` matrix for a dense `ColPivQr`.
///
/// [`drop_linearly_dependent`]'s own docs assume "`p` is expected to be
/// small relative to `n`"; several real Netlib instances violate that
/// badly (`ganges`: `n=1681`, `p=1284` equality rows — almost every
/// constraint is an equality) and paid for it directly: that dense QR
/// call measured at >99% of `ganges`'s *entire* presolve time, dwarfing
/// every other technique in this pipeline combined. This function treats
/// each of the `p` rows as a native sparse row over `n+1` columns (the
/// row's own coefficients plus one extra "virtual" column at index `n`
/// for its RHS — the same augmentation trick `drop_linearly_dependent`
/// uses, via a genuine matrix row there vs. a genuinely sparse map entry
/// here), so cost scales with actual nonzero fill rather than `n * p`.
///
/// Per-step pivot selection mirrors `simplex::lu::MarkowitzState::find_best_pivot`
/// (ascending-degree bucket scan, ties broken toward larger pivot
/// magnitude, `PIVOT_STABILITY`-gated candidates) — but a row here is
/// classified *dependent*, not treated as a hard elimination failure the
/// way a singular basis would be: after eliminating every previously-kept
/// row's pivot column out of it, a row whose largest remaining active
/// entry is negligible relative to its own *original* norm (`DEP_TOL`,
/// same role as `drop_linearly_dependent`'s `col_norm(orig)` check) is
/// dropped, and elimination simply continues with whatever rows/columns
/// remain — exactly the "residual after projecting out earlier pivots"
/// meaning `|R[k,k]|` carries in the dense QR, arrived at here via direct
/// Gaussian elimination instead. No `L`/`U` factors are kept (nothing
/// downstream needs to *solve* against this matrix) — only which rows got
/// a pivot at all.
fn drop_linearly_dependent_sparse(rows_in: &[(Vec<(usize, f64)>, f64)], n: usize) -> Vec<usize> {
    let p = rows_in.len();
    if p == 0 {
        return Vec::new();
    }
    let aug_n = n + 1;

    let mut rows: Vec<BTreeMap<usize, f64>> = Vec::with_capacity(p);
    let mut row_orig_norm = vec![0.0f64; p];
    for (i, (row, rhs)) in rows_in.iter().enumerate() {
        let mut m: BTreeMap<usize, f64> = BTreeMap::new();
        for &(j, v) in row {
            if v != 0.0 {
                *m.entry(j).or_insert(0.0) += v;
            }
        }
        m.retain(|_, v| *v != 0.0);
        if *rhs != 0.0 {
            m.insert(n, *rhs);
        }
        row_orig_norm[i] = m.values().map(|v| v * v).sum::<f64>().sqrt();
        rows.push(m);
    }

    let mut col_rows: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); aug_n];
    let mut col_max_abs = vec![0.0f64; aug_n];
    let mut row_degree = vec![0usize; p];
    for (i, row) in rows.iter().enumerate() {
        row_degree[i] = row.len();
        for (&j, &v) in row {
            col_rows[j].insert(i);
            col_max_abs[j] = col_max_abs[j].max(v.abs());
        }
    }
    let mut col_degree: Vec<usize> = col_rows.iter().map(|s| s.len()).collect();

    // Bucket arrays: a column's degree can be at most `p` (every row could
    // touch it); a row's degree can be at most `aug_n`.
    let mut col_buckets: Vec<VecDeque<usize>> = vec![VecDeque::new(); p + 1];
    let mut row_buckets: Vec<VecDeque<usize>> = vec![VecDeque::new(); aug_n + 1];
    let mut col_bucket_pos: Vec<Option<usize>> = vec![None; aug_n];
    let mut row_bucket_pos: Vec<Option<usize>> = vec![None; p];
    for j in 0..aug_n {
        col_bucket_pos[j] = Some(col_buckets[col_degree[j]].len());
        col_buckets[col_degree[j]].push_back(j);
    }
    for i in 0..p {
        row_bucket_pos[i] = Some(row_buckets[row_degree[i]].len());
        row_buckets[row_degree[i]].push_back(i);
    }

    let mut col_used = vec![false; aug_n];
    let mut row_used = vec![false; p];
    let mut keep = vec![false; p];

    // Local helpers mirroring `simplex::lu::MarkowitzState`'s own
    // bucket-maintenance methods exactly (swap-to-last-then-pop removal,
    // O(1) moves via the position index) — see that module's docs for why
    // this is what keeps bucket upkeep sub-`O(m)` per step.
    fn remove_from_bucket(buckets: &mut [VecDeque<usize>], pos: &mut [Option<usize>], degree: usize, idx: usize) {
        if let Some(p) = pos[idx] {
            let bucket = &mut buckets[degree];
            if p < bucket.len() {
                let last = bucket.pop_back().unwrap();
                if p < bucket.len() {
                    bucket[p] = last;
                    pos[last] = Some(p);
                }
            }
            pos[idx] = None;
        }
    }
    fn move_bucket(buckets: &mut [VecDeque<usize>], pos: &mut [Option<usize>], old_degree: usize, new_degree: usize, idx: usize, used: &[bool]) {
        if used[idx] || old_degree == new_degree {
            return;
        }
        remove_from_bucket(buckets, pos, old_degree, idx);
        let p = buckets[new_degree].len();
        pos[idx] = Some(p);
        buckets[new_degree].push_back(idx);
    }

    // `min(p, aug_n)` is the maximum possible rank — no point searching
    // further once that many rows have been kept.
    let max_steps = p.min(aug_n);
    for _step in 0..max_steps {
        // Ascending-degree bucket scan over columns, exactly like
        // `find_best_pivot` — see that function's own docs for why this
        // (rather than a full active-submatrix scan) is what keeps this
        // sub-`O(m)` per step, and why the per-degree-level early exit is
        // a standard, deliberately non-exhaustive relaxation.
        let mut best: Option<(usize, usize)> = None;
        let mut best_score = usize::MAX;
        let mut best_abs = 0.0f64;
        'search: for deg_col in 1..col_buckets.len() {
            for &j in &col_buckets[deg_col] {
                if col_used[j] {
                    continue;
                }
                for &i in &col_rows[j] {
                    if row_used[i] {
                        continue;
                    }
                    let Some(&v) = rows[i].get(&j) else { continue };
                    if v == 0.0 || v.abs() < PIVOT_STABILITY * col_max_abs[j] {
                        continue;
                    }
                    let score = (row_degree[i] - 1) * (col_degree[j] - 1);
                    if score < best_score || (score == best_score && v.abs() > best_abs) {
                        best_score = score;
                        best = Some((i, j));
                        best_abs = v.abs();
                    }
                }
                if best_score == 0 {
                    break 'search;
                }
            }
            if best.is_some() && best_score <= deg_col * deg_col {
                break;
            }
        }
        let Some((pi, pj)) = best else {
            // No numerically acceptable pivot remains anywhere: every
            // still-active row's remaining entries are too small, relative
            // to whatever column they sit in, to divide by safely — the
            // rank has been exhausted, and every row not yet kept is
            // dependent by construction (never selected).
            break;
        };

        let pivot_val = *rows[pi].get(&pj).unwrap();
        row_used[pi] = true;
        col_used[pj] = true;
        remove_from_bucket(&mut row_buckets, &mut row_bucket_pos, row_degree[pi], pi);
        remove_from_bucket(&mut col_buckets, &mut col_bucket_pos, col_degree[pj], pj);

        // Dependency test: is what's left of this row, at the point it
        // was chosen, negligible relative to its own *original* scale?
        // This — not `PIVOT_STABILITY` above, a different, columnwise
        // concept — is the actual redundancy criterion; see this
        // function's own docs.
        if pivot_val.abs() <= DEP_TOL * row_orig_norm[pi].max(1e-300) {
            // Dependent: drop it (leave `keep[pi] = false`) without
            // eliminating — it contributes no independent structure to
            // scatter into the other rows.
            continue;
        }
        keep[pi] = true;

        // Eliminate column `pj` from every other row that still has it —
        // the same scatter `simplex::lu::MarkowitzState::eliminate` does,
        // via a single `entry()` descent per touched `(row, col)` pair.
        let pivot_row_snapshot: Vec<(usize, f64)> = rows[pi].iter().map(|(&j, &v)| (j, v)).collect();
        let affected_rows: Vec<usize> = col_rows[pj].iter().copied().filter(|&i| i != pi).collect();
        for i in affected_rows {
            let Some(&aij) = rows[i].get(&pj) else { continue };
            if aij == 0.0 {
                continue;
            }
            let mult = aij / pivot_val;
            let old_degree = row_degree[i];
            for &(j, v) in &pivot_row_snapshot {
                if j == pj {
                    continue;
                }
                use std::collections::btree_map::Entry;
                match rows[i].entry(j) {
                    Entry::Occupied(mut e) => {
                        let new_val = *e.get() - mult * v;
                        if new_val == 0.0 {
                            e.remove();
                            col_rows[j].remove(&i);
                        } else {
                            *e.get_mut() = new_val;
                        }
                    }
                    Entry::Vacant(e) => {
                        let new_val = -mult * v;
                        if new_val != 0.0 {
                            e.insert(new_val);
                            col_rows[j].insert(i);
                            let new_deg = col_rows[j].len();
                            move_bucket(&mut col_buckets, &mut col_bucket_pos, new_deg - 1, new_deg, j, &col_used);
                            col_degree[j] = new_deg;
                        }
                    }
                }
            }
            rows[i].remove(&pj);
            let new_row_degree = rows[i].len();
            move_bucket(&mut row_buckets, &mut row_bucket_pos, old_degree, new_row_degree, i, &row_used);
            row_degree[i] = new_row_degree;
            // `col_max_abs`/degree for every column this row still touches
            // may have shrunk; recomputed lazily below via a full refresh
            // over the columns this elimination actually touched.
        }
        col_rows[pj].clear();
        // Recompute `col_max_abs` for every column touched by this
        // elimination step (both `pivot_row_snapshot`'s own columns and
        // any that lost an entry) — a direct rescan of each one's own
        // (now-updated) `col_rows` set, O(that column's current degree),
        // matching `MarkowitzState::refresh_column`'s own cost bound.
        for &(j, _) in &pivot_row_snapshot {
            if j == pj || col_used[j] {
                continue;
            }
            let new_max = col_rows[j].iter().filter_map(|&i| rows[i].get(&j).map(|v| v.abs())).fold(0.0, f64::max);
            col_max_abs[j] = new_max;
        }
    }

    (0..p).filter(|&i| keep[i]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pivot-position -> original-row mapping must be the right one of
    /// faer's two permutation arrays, and a test can only tell them apart
    /// when the pivot permutation is *not* its own inverse (a single swap
    /// is, so "largest-norm row placed last" proves nothing). Here rows 0,
    /// 2, 3 form the dependent set (`r3 = r0 + r2`), while row 1 is
    /// independent of everything. Column norms force pivot order
    /// `r1, r3, ...` — a 3-cycle-containing permutation — so the negligible
    /// pivot lands at position 2 or 3 and must map back to one of rows
    /// `{0, 2, 3}`; mapping through the wrong array instead drops row 1.
    #[test]
    fn drop_linearly_dependent_maps_pivot_positions_to_the_right_rows() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 3.0)], 3.0),
            (vec![(1, 10.0)], 10.0),
            (vec![(2, 5.0)], 5.0),
            (vec![(0, 3.0), (2, 5.0)], 8.0),
        ];
        let keep = drop_linearly_dependent(&rows, 3);
        assert_eq!(keep.len(), 3, "keep={keep:?}");
        assert!(keep.contains(&1), "independent row 1 must survive; keep={keep:?}");
        assert_eq!(keep.iter().filter(|&&i| i != 1).count(), 2, "keep={keep:?}");
    }
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
