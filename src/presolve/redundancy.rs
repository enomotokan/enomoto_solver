//! Removes redundant equality-constraint rows from `(A, b)`, run once (on
//! the Ruiz-scaled problem) before either engine's main loop starts —
//! shared by `interior_point.rs` and `simplex.rs` via `presolve::run_extended`.
//!
//! Two passes:
//!  1. **Direct duplicate detection**: a row that is an exact or
//!     scalar-multiple duplicate of an already-kept row (same coefficient
//!     pattern up to one scalar, including the right-hand side) is dropped.
//!     This is a cheap hash comparison, no linear algebra.
//!  2. **Rank-revealing elimination**, via *either* of two implementations
//!     chosen per-call by [`reduce_equalities`] (see its own docs for the
//!     dispatch rule): dense column-pivoted QR ([`drop_linearly_dependent`])
//!     or sparse Gaussian elimination ([`drop_linearly_dependent_sparse`],
//!     picking at each step the column carrying the most numerical weight
//!     and, within it, the largest-magnitude row as pivot, mirroring dense
//!     QR's own strategy). Both answer the identical question — a row
//!     whose residual after eliminating every previously-kept row's pivot
//!     is negligible relative to its own original norm is a linear
//!     combination of the others — just via different arithmetic paths,
//!     each cheap in the regime the other is expensive in.
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
//! between runs.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

use faer::linalg::solvers::ColPivQr;
use faer::Mat;

use crate::sparse::{csr_from_rows, Csr};

/// Above this fraction of nonzero coefficients (`nnz / (p * n)`, over the
/// deduplicated equality rows), [`reduce_equalities`] uses
/// [`drop_linearly_dependent`] (dense QR) instead of
/// [`drop_linearly_dependent_sparse`].
///
/// Density, not `p` or a dense-QR flop-count estimate, is what actually
/// separates the two regimes — calibrated directly against measured
/// Netlib instances, not derived analytically. The first cut at this
/// dispatch rule used a pure cost estimate (`(n+1) * p^2`, dense QR's own
/// flop order, thresholded so `wood1p`'s small `p` routed to dense): it
/// fixed `wood1p` but *also* routed `standmps` (`p=268`, density 0.96%)
/// and `fffff800` (`p=350`, density 1.6%) to dense even though the sparse
/// method was already faster for both there (their low density means
/// elimination stays close to its own nonzero count, with none of the
/// fill-in blowup a size-based estimate implicitly worries about) —
/// measured regressions of roughly 2-4x on both after that first cut.
/// `wood1p` itself is the outlier that actually needs dense: 11.1% row
/// density, roughly 7-30x denser than every other measured instance
/// (`fffff800` 1.6%, `standmps` 0.96%, `sierra` 0.37%, `ganges` 0.31%,
/// `modszk1` 0.28%, `stocfor2` 0.21%) — dense QR there stayed a bounded
/// ~30ms while the sparse method's fill-in blew up to 962ms. `3%` sits
/// with comfortable margin above every instance that must stay sparse and
/// below `wood1p`'s own density.
const DENSE_DENSITY_THRESHOLD: f64 = 0.03;

/// Returns a reduced `(A, b)` with duplicate/linearly-dependent equality
/// rows removed.
///
/// Dispatches to whichever of [`drop_linearly_dependent`] (dense
/// column-pivoted QR) or [`drop_linearly_dependent_sparse`] (sparse
/// Gaussian elimination) is expected to be cheaper for this problem's
/// shape — see [`DENSE_DENSITY_THRESHOLD`]'s own docs for the rule and the
/// real Netlib instances that motivated it.
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
    let nnz: usize = deduped.iter().map(|(row, _)| row.len()).sum();
    let density = nnz as f64 / (deduped.len() as f64 * n.max(1) as f64);
    let keep = if density > DENSE_DENSITY_THRESHOLD {
        drop_linearly_dependent(&deduped, n)
    } else {
        drop_linearly_dependent_sparse(&deduped, n)
    };

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

/// Rank-revealing alternative to [`drop_linearly_dependent_sparse`]:
/// returns the indices into `rows` of a maximal linearly independent
/// subset, found via column-pivoted QR of the dense `(n+1) x p` matrix
/// whose columns are `rows`' own coefficients (plus one extra row for the
/// RHS, at index `n`). Cost is a fixed `O((n+1) * p^2)` regardless of how
/// dense or sparse `rows` actually are — see [`DENSE_DENSITY_THRESHOLD`]'s
/// own docs for when [`reduce_equalities`] picks this over the sparse
/// method instead.
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

/// Numerical-stability floor a candidate pivot must clear, relative to the
/// current **global** maximum active entry anywhere in the matrix (not
/// just its own column's) — see [`drop_linearly_dependent_sparse`]'s own
/// docs for why "global" here, not "local to the column", is what makes
/// this a correct rank-revealing criterion instead of merely a safe-enough
/// pivot for solving. A different, narrower purpose than `DEP_TOL` below:
/// this only gates which *candidates* the fill-minimizing search is
/// allowed to accept, the same role `simplex::lu`'s own `STABILITY`
/// constant plays for the (unrelated) basis factorization.
const PIVOT_STABILITY: f64 = 0.1;
/// Dependency floor, relative to a row's own *original* norm (computed
/// once, before any elimination) — this is the actual redundancy
/// criterion. `DEP_TOL * row_orig_norm` plays the same role here that
/// `1e-9 * col_norm(orig)` plays against `|R[k,k]|` in
/// [`drop_linearly_dependent`] (see that function's own docs for why a
/// per-row-relative, not global, threshold matters — the same reasoning
/// applies here).
const DEP_TOL: f64 = 1e-9;

/// Returns the indices into `rows` of a maximal linearly independent
/// subset, via sparse Gaussian elimination over the `p` rows treated as
/// sparse rows over `n+1` columns (the row's own coefficients plus one
/// extra "virtual" column at index `n` for its RHS).
///
/// Columns are `[row coefficients ; rhs]` (`n+1` entries), not just the
/// coefficients: a row whose *coefficients* are a linear combination of
/// other rows' coefficients but whose *rhs* breaks that same combination
/// is an inconsistency (the system is infeasible), not redundancy, and
/// must not be dropped here — appending rhs as an extra coordinate makes
/// such a row linearly independent in the augmented sense, so this
/// correctly keeps it (the existing Farkas-certificate infeasibility
/// detection downstream is what reports it).
///
/// This exists alongside [`drop_linearly_dependent`] (dense QR), not in
/// place of it: dense QR's own docs used to assume "`p` is expected to be
/// small relative to `n`" and had no fallback when several real Netlib
/// instances violate that badly (`ganges`: `n=1681`, `p=1284` equality
/// rows — almost every constraint is an equality), which paid for it
/// directly — dense QR there measured at >99% of `ganges`'s *entire*
/// presolve time, dwarfing every other technique in this pipeline
/// combined. This function instead treats each of the `p` rows as a
/// native sparse row over `n+1` columns (the same RHS-augmentation trick
/// dense QR uses, via a genuinely sparse map entry instead of a dense
/// matrix row), so cost scales with actual nonzero fill rather than
/// `n * p` — but that same fill-dependence is a liability of its own on a
/// matrix that isn't actually sparse to begin with (`wood1p`: only
/// `p=243` but 11% row density, an order of magnitude denser than every
/// other measured instance — fill-in during elimination blew up to
/// 962ms there, while dense QR's cost bound doesn't care about density at
/// all). [`reduce_equalities`] picks between the two per call based on
/// row density — see [`DENSE_DENSITY_THRESHOLD`]'s own docs.
///
/// **Pivot selection is a hybrid of dense `ColPivQr`'s numerical strategy
/// and `simplex::lu::MarkowitzState`'s fill-minimizing one, not purely
/// either**: candidates are still found via an ascending-Markowitz-degree
/// bucket scan exactly like `find_best_pivot` (for fill control — see that
/// function's own docs for why this keeps the scan sub-`O(m)` per step in
/// the common case), but a candidate is only *acceptable* if its magnitude
/// is within `PIVOT_STABILITY` of the current **global** maximum active
/// entry anywhere in the matrix (tracked incrementally via `heap`/
/// `col_bits` below, not rescanned from scratch each step) — not, as an
/// earlier version of this function tried, relative only to its *own
/// column's* current maximum.
///
/// That earlier, purely-local-threshold version is what `simplex::lu`'s
/// own Markowitz factorization uses (appropriate for *solving*, where any
/// non-negligible pivot is fine), but it does not work for *rank
/// revelation*: a column's own max is trivially satisfied by its own max
/// entry, so on real data (`ganges`) a low-degree column holding only
/// small, easily-corrupted-by-cancellation entries got chosen as a pivot
/// purely because it had few nonzeros, ahead of a column that was
/// numerically dominant *matrix-wide* — three genuinely independent rows
/// were misclassified as dependent as a result (confirmed by direct
/// comparison against the dense reference; raising the local threshold
/// had *no* effect, proving the bug was about which column got selected,
/// not how strong the pivot was once one was chosen). A purely
/// global-norm-maximizing version (no degree preference at all, matching
/// dense QR's own strategy exactly) fixed that but gave up fill control
/// entirely, causing severe fill-in blowups on several *other* real
/// instances (`modszk1`, `standmps`, `wood1p`, `fffff800` all measured
/// 3-10x slower). Gating the *same* degree-ascending search with a
/// *global*, not local, acceptance threshold gets both: fill-minimizing
/// order is still preferred among candidates that are numerically safe,
/// and a candidate that is only locally-large-but-globally-negligible
/// (the actual `ganges` failure mode) is skipped in favor of continuing
/// the search until a genuinely significant pivot is found — worst case,
/// the column realizing the global maximum itself, which always trivially
/// passes its own threshold.
///
/// A row here is classified *dependent*, not treated as a hard elimination
/// failure the way a singular basis would be: after eliminating every
/// previously-kept row's pivot column out of it, a row whose largest
/// remaining active entry is negligible relative to its own *original*
/// norm (`DEP_TOL`, same role [`drop_linearly_dependent`]'s own
/// `col_norm(orig)` check plays) is dropped, and elimination simply continues
/// with whatever rows/columns remain — exactly the "residual after
/// projecting out earlier pivots" meaning `|R[k,k]|` carries in the dense
/// QR, arrived at here via direct Gaussian elimination instead. No `L`/`U`
/// factors are kept (nothing downstream needs to *solve* against this
/// matrix) — only which rows got a pivot at all.
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
    let mut row_degree = vec![0usize; p];
    for (i, row) in rows.iter().enumerate() {
        row_degree[i] = row.len();
        for &j in row.keys() {
            col_rows[j].insert(i);
        }
    }
    let mut col_degree: Vec<usize> = col_rows.iter().map(|s| s.len()).collect();

    // Tracks which column currently holds the largest active entry
    // anywhere in the matrix, without an `O(aug_n)` rescan every step:
    // `heap` is ordered by each active column's current max-abs entry
    // (`f64::to_bits` preserves ordering for non-negative values, so
    // sorting the bit pattern sorts by magnitude directly), and
    // `col_bits[j]` records which entry (if any) column `j` currently has
    // in `heap` so it can be removed before a fresh one is inserted.
    // `heap.last()` (the true global max) then costs `O(log aug_n)`.
    let mut heap: BTreeSet<(u64, usize)> = BTreeSet::new();
    let mut col_bits: Vec<Option<u64>> = vec![None; aug_n];
    fn refresh_col(col_rows: &[BTreeSet<usize>], rows: &[BTreeMap<usize, f64>], heap: &mut BTreeSet<(u64, usize)>, col_bits: &mut [Option<u64>], j: usize) {
        if let Some(old) = col_bits[j].take() {
            heap.remove(&(old, j));
        }
        let new_max = col_rows[j].iter().filter_map(|&i| rows[i].get(&j).map(|v| v.abs())).fold(0.0f64, f64::max);
        if new_max > 0.0 {
            let bits = new_max.to_bits();
            heap.insert((bits, j));
            col_bits[j] = Some(bits);
        }
    }
    for j in 0..aug_n {
        refresh_col(&col_rows, &rows, &mut heap, &mut col_bits, j);
    }

    // Column bucket arrays for the ascending-Markowitz-degree scan — see
    // `simplex::lu::MarkowitzState::find_best_pivot`'s own docs for why
    // this (rather than a full active-submatrix scan) is what keeps this
    // sub-`O(m)` per step. Unlike that function, only columns are bucketed
    // here: nothing below scans rows by degree bucket, `row_degree` alone
    // (updated in place) is enough to evaluate the Markowitz score.
    let mut col_buckets: Vec<VecDeque<usize>> = vec![VecDeque::new(); p + 1];
    let mut col_bucket_pos: Vec<Option<usize>> = vec![None; aug_n];
    for j in 0..aug_n {
        col_bucket_pos[j] = Some(col_buckets[col_degree[j]].len());
        col_buckets[col_degree[j]].push_back(j);
    }
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

    let mut col_used = vec![false; aug_n];
    let mut row_used = vec![false; p];
    let mut keep = vec![false; p];

    // `min(p, aug_n)` is the maximum possible rank — no point searching
    // further once that many rows have been kept.
    let max_steps = p.min(aug_n);
    for _step in 0..max_steps {
        let gmax = match heap.iter().next_back() {
            Some(&(bits, _)) => f64::from_bits(bits),
            None => break, // every active column is entirely zero
        };
        let threshold = PIVOT_STABILITY * gmax;

        // Ascending-degree bucket scan over columns, exactly like
        // `find_best_pivot`, but a candidate is only ever recorded into
        // `best` once it passes the *global* `threshold` above — see this
        // function's own docs for why that, not local column-relative
        // magnitude, is what actually determines a correct rank here.
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
                    if v == 0.0 || v.abs() < threshold {
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
            // No column has an entry within `PIVOT_STABILITY` of the
            // current global maximum — since the column *realizing* that
            // maximum always trivially passes its own threshold, this
            // cannot happen while any active column remains nonzero; take
            // it as "rank exhausted" defensively.
            break;
        };

        let pivot_val = *rows[pi].get(&pj).unwrap();
        row_used[pi] = true;
        col_used[pj] = true;
        remove_from_bucket(&mut col_buckets, &mut col_bucket_pos, col_degree[pj], pj);
        if let Some(old) = col_bits[pj].take() {
            heap.remove(&(old, pj));
        }

        // Row `pi` is retiring (whether it ends up kept or dependent,
        // decided just below) — drop it from every *other* column it
        // still touches so a later column's scan never has to consider an
        // inactive row's stale membership. `heap`/`col_bits` are refreshed
        // separately below, once, in whichever of the two branches is
        // actually taken — not here — since the kept branch's own
        // elimination pass touches these same columns' *values* again
        // right after this, and refreshing twice per step is a real,
        // measured cost on matrices with high average column degree
        // (`wood1p`: doing it unconditionally here as well as after
        // elimination roughly doubled `reduce_equalities`' time).
        let pi_cols: Vec<usize> = rows[pi].keys().copied().filter(|&j| j != pj).collect();
        for &j in &pi_cols {
            col_rows[j].remove(&pi);
            let new_deg = col_rows[j].len();
            move_bucket(&mut col_buckets, &mut col_bucket_pos, new_deg + 1, new_deg, j, &col_used);
            col_degree[j] = new_deg;
        }

        // Dependency test: is what's left of this row, at the point it
        // was chosen, negligible relative to its own *original* scale?
        if pivot_val.abs() <= DEP_TOL * row_orig_norm[pi].max(1e-300) {
            // Dependent: drop it (leave `keep[pi] = false`) without
            // eliminating — it contributes no independent structure to
            // scatter into the other rows. This branch never reaches the
            // post-elimination refresh pass below, so `pi_cols` must be
            // refreshed here instead — otherwise a stale, too-high entry
            // could survive in `heap` for a column whose recorded max
            // came only from `pi`, wrongly gating out a genuinely valid
            // pivot elsewhere via an inflated `gmax` on a later step.
            for &j in &pi_cols {
                if !col_used[j] {
                    refresh_col(&col_rows, &rows, &mut heap, &mut col_bits, j);
                }
            }
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
                            let new_deg = col_rows[j].len();
                            move_bucket(&mut col_buckets, &mut col_bucket_pos, new_deg + 1, new_deg, j, &col_used);
                            col_degree[j] = new_deg;
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
            row_degree[i] = new_row_degree;
        }
        col_rows[pj].clear();

        // Refresh the true current max for every column touched by this
        // elimination step, from that column's own (now-updated)
        // `col_rows` set — O(that column's current degree).
        for &(j, _) in &pivot_row_snapshot {
            if j == pj || col_used[j] {
                continue;
            }
            refresh_col(&col_rows, &rows, &mut heap, &mut col_bits, j);
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

    /// Same scenario, checked against [`drop_linearly_dependent_sparse`] —
    /// rows 0, 2, 3 form a rank-2 dependent set (`r3 = r0 + r2`, so *any* 2
    /// of the 3 are a valid independent basis for it — which 2 survive is
    /// a legitimate, pivot-order-dependent choice both implementations are
    /// free to make differently), hence checking the *count* and row 1's
    /// survival rather than the exact index set, matching the dense test's
    /// own already order-agnostic assertions.
    #[test]
    fn drop_linearly_dependent_sparse_matches_dense_on_the_same_case() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 3.0)], 3.0),
            (vec![(1, 10.0)], 10.0),
            (vec![(2, 5.0)], 5.0),
            (vec![(0, 3.0), (2, 5.0)], 8.0),
        ];
        let keep = drop_linearly_dependent_sparse(&rows, 3);
        assert_eq!(keep.len(), 3, "keep={keep:?}");
        assert!(keep.contains(&1), "independent row 1 must survive; keep={keep:?}");
        assert_eq!(keep.iter().filter(|&&i| i != 1).count(), 2, "keep={keep:?}");
    }

    /// [`reduce_equalities`] itself must route a high-density case
    /// (mirroring `wood1p`'s own shape — small `p` but 11% row density) to
    /// dense QR, and every genuinely sparse case to the sparse method
    /// regardless of `p`, per [`DENSE_DENSITY_THRESHOLD`]'s own docs —
    /// checked indirectly here (both paths are independently tested for
    /// correctness above) by confirming the *density arithmetic*
    /// [`reduce_equalities`] uses picks the expected side for `(n, p,
    /// nnz)` shapes drawn from real measured instances. `standmps` and
    /// `fffff800` are the cases that specifically ruled out a pure
    /// size-based cost estimate in an earlier version of this rule (see
    /// [`DENSE_DENSITY_THRESHOLD`]'s own docs) — both have modest `p` but
    /// low density, and must stay on the sparse path despite that.
    #[test]
    fn dense_density_threshold_routes_known_instances_correctly() {
        let density = |n: usize, p: usize, nnz: usize| nnz as f64 / (p as f64 * n as f64);
        // wood1p: p=243, n=2594, nnz=70214 (11.1% density) -- must go dense.
        assert!(density(2594, 243, 70214) > DENSE_DENSITY_THRESHOLD);
        // standmps: p=268, n=1075, nnz=2776 (0.96% density) -- must stay sparse.
        assert!(density(1075, 268, 2776) <= DENSE_DENSITY_THRESHOLD);
        // fffff800: p=350, n=854, nnz=4775 (1.6% density) -- must stay sparse.
        assert!(density(854, 350, 4775) <= DENSE_DENSITY_THRESHOLD);
        // ganges: p=1284, n=1681, nnz=6612 (0.31% density) -- must stay sparse.
        assert!(density(1681, 1284, 6612) <= DENSE_DENSITY_THRESHOLD);
    }

    /// A row whose *coefficients* are a linear combination of others' but
    /// whose *rhs* breaks that same combination is an inconsistency
    /// (infeasible system), not redundancy — must NOT be dropped, exactly
    /// why this function's own docs augment with `rhs` as an extra
    /// coordinate. `r0 - r1`'s
    /// coefficients, `(1,0,-1)`, match `r2`'s exactly, but `3-4=-1 != 7`.
    #[test]
    fn drop_linearly_dependent_sparse_keeps_rhs_inconsistent_rows() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 1.0), (1, 1.0)], 3.0),
            (vec![(1, 1.0), (2, 1.0)], 4.0),
            (vec![(0, 1.0), (2, -1.0)], 7.0),
        ];
        let keep = drop_linearly_dependent_sparse(&rows, 3);
        assert_eq!(keep, vec![0, 1, 2], "an rhs-inconsistent row must survive; keep={keep:?}");
    }

    /// No dependency anywhere: every row must survive.
    #[test]
    fn drop_linearly_dependent_sparse_keeps_everything_when_independent() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 1.0), (1, 2.0)], 5.0),
            (vec![(1, 1.0), (2, 3.0)], 7.0),
            (vec![(0, 2.0), (2, 1.0)], 4.0),
        ];
        let keep = drop_linearly_dependent_sparse(&rows, 3);
        assert_eq!(keep, vec![0, 1, 2], "keep={keep:?}");
    }

    /// An exact duplicate row (same coefficients *and* rhs) is genuinely
    /// redundant — unambiguous, so the exact surviving index is checked
    /// too, not just the count.
    #[test]
    fn drop_linearly_dependent_sparse_drops_an_exact_duplicate() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 1.0), (1, 1.0)], 5.0),
            (vec![(0, 1.0), (1, 1.0)], 5.0),
            (vec![(1, 1.0), (2, 1.0)], 3.0),
        ];
        let keep = drop_linearly_dependent_sparse(&rows, 3);
        assert_eq!(keep, vec![0, 2], "keep={keep:?}");
    }

    /// A larger synthetic system with a known rank deficiency (row 4 is a
    /// linear combination of rows 0-3, constructed with non-trivial
    /// coefficients so no single pairwise/duplicate shortcut could catch
    /// it) — checks the sparse path finds the correct *rank* (4
    /// survivors) on a case too large to eyeball by hand, without pinning
    /// down which specific row is dropped (multiple valid bases exist
    /// here too).
    #[test]
    fn drop_linearly_dependent_sparse_finds_the_right_rank_in_a_larger_system() {
        let n = 6;
        let base: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 2.0), (1, 1.0)], 5.0),
            (vec![(1, 3.0), (2, 1.0)], 8.0),
            (vec![(2, 1.0), (3, 4.0)], 2.0),
            (vec![(3, 1.0), (4, 2.0), (5, 1.0)], 6.0),
        ];
        // row 4 = 2*row0 - row1 + 3*row2 (coefficients and rhs both
        // combined the same way, so this is genuinely redundant, not
        // inconsistent).
        let mut combo: BTreeMap<usize, f64> = BTreeMap::new();
        let mut rhs = 0.0;
        for (mult, (row, r)) in [(2.0, &base[0]), (-1.0, &base[1]), (3.0, &base[2])] {
            for &(j, v) in row {
                *combo.entry(j).or_insert(0.0) += mult * v;
            }
            rhs += mult * r;
        }
        let mut rows = base.clone();
        rows.push((combo.into_iter().collect(), rhs));

        let keep = drop_linearly_dependent_sparse(&rows, n);
        assert_eq!(keep.len(), 4, "expected rank 4 out of 5 rows; keep={keep:?}");
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
