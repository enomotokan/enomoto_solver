//! Removes redundant equality-constraint rows from `(A, b)`, run once (on
//! the Ruiz-scaled problem) before either engine's main loop starts —
//! shared by `interior_point.rs` and `simplex.rs` via `presolve::run_extended`.
//! [`reduce_inequalities`]'s own duplicate-row pass over `(G, h)` is the
//! exception: cheap enough (a single hash scan, no linear algebra) that
//! `run_extended`'s own round loop calls it again at the end of every outer
//! round, not just once here — see that call site's own docs for why (in
//! short: `doubleton`/`colsingleton` substitution can turn two originally-
//! distinct inequality rows into duplicates only *after* this pre-loop call
//! already ran).
//!
//! Two passes:
//!  1. **Direct duplicate detection**: a row that is an exact or
//!     scalar-multiple duplicate of an already-kept row (same coefficient
//!     pattern up to one scalar, including the right-hand side) is dropped.
//!     This is a cheap hash comparison, no linear algebra.
//!  2. **Rank-revealing elimination**, via *either* of two implementations
//!     chosen per-call by [`reduce_equalities`] (see its own docs for the
//!     dispatch rule): dense column-pivoted QR ([`drop_linearly_dependent`])
//!     or sparse Gaussian elimination ([`drop_linearly_dependent_sparse_blocked`],
//!     picking at each step the column carrying the most numerical weight
//!     and, within it, the largest-magnitude row as pivot, mirroring dense
//!     QR's own strategy). Both answer the identical question — a row
//!     whose residual after eliminating every previously-kept row's pivot
//!     is negligible relative to its own original norm is a linear
//!     combination of the others — just via different arithmetic paths,
//!     each cheap in the regime the other is expensive in. The sparse path
//!     is itself a thin wrapper ([`drop_linearly_dependent_sparse_blocked`])
//!     around the core per-block algorithm ([`drop_linearly_dependent_sparse`]):
//!     a Dulmage-Mendelsohn-style block-triangularization pre-pass
//!     ([`dulmage_mendelsohn_blocks`], via a maximum bipartite matching plus
//!     Tarjan strongly-connected-components) first splits the system into
//!     sub-problems — some real Netlib instances decompose into dozens of
//!     near-identical-size blocks this way (one per vessel/route/period in
//!     a multi-period scheduling LP), and instances that share no exploitable
//!     structure by plain column-disjointness alone still often decompose
//!     into hundreds of much smaller blocks once the matching's dependency
//!     structure is taken into account — which are then solved, and above a
//!     total-size threshold dispatched via `rayon`: unlike using this same
//!     decomposition for LU factorization or solving, redundancy detection
//!     only ever asks a *local* per-row question ("is this row exactly some
//!     combination of these specific other rows?"), which holds
//!     unconditionally once verified — so every block here is safe to check
//!     independently and in any order, including concurrently, with no
//!     triangular ordering dependency to respect (see
//!     [`dulmage_mendelsohn_blocks`]'s own docs for the full argument).
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
#[cfg(test)]
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};

use faer::dyn_stack::{GlobalPodBuffer, PodStack};
use faer::linalg::qr::col_pivoting::compute as colpiv_qr;
use faer::{Mat, Parallelism};

use crate::presolve::smallcoeff;
use crate::sparse::{Csr, CsrRowBuilder, csr_from_rows, csr_is_canonical, csr_row_iter, csr_row_vec};
/// Measurement counters answering "would a dedicated block-triangularization
/// pre-pass (Dulmage-Mendelsohn / BTF, exposing structurally-forced 1x1
/// pivots before elimination starts, the way `simplex::lu`'s own
/// `PROF_TOTAL_STEPS`/`PROF_TRIVIAL_STEPS` counters investigated for the
/// basis LU) help [`drop_linearly_dependent_sparse`] the same way it was
/// found *not* to help there — see this module's own `ENOMOTO_PROF_REDUNDANCY`
/// diagnostic (`presolve.rs`'s `reduce_equalities` call site) for the
/// answer. A step is "trivial" under the identical definition
/// `simplex::lu::MarkowitzState::find_best_pivot` uses: the winning
/// `(row_degree - 1) * (col_degree - 1)` Markowitz score is `0`, i.e. a
/// structurally forced pivot a BTF pre-pass would also have found for free.
pub(crate) static PROF_TOTAL_STEPS: AtomicUsize = AtomicUsize::new(0);
pub(crate) static PROF_TRIVIAL_STEPS: AtomicUsize = AtomicUsize::new(0);

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
///
/// `lb`/`ub` are used only by the sparse path's own
/// [`dulmage_mendelsohn_blocks`] block-decomposition pre-pass, to decide
/// which structural edges a negligible coefficient should be left out of
/// (see that function's own docs) — never to touch `a`/`b` themselves.
/// Dropping an edge here is safe regardless of how accurate `lb`/`ub` are
/// (see that same function's docs on why it only costs recall, never
/// soundness), so passing the model's raw, not-yet-propagated bounds —
/// this runs before presolve's own bound-tightening rounds start — is
/// fine: staler/wider bounds just make the negligibility test fire less
/// often, i.e. a more conservative (coarser, never incorrect) split than
/// the fully-tightened bounds would give.
pub fn reduce_equalities(a: &Csr, b: &[f64], n: usize, lb: &[f64], ub: &[f64]) -> (Csr, Vec<f64>) {
    let p = a.nrows();
    if p == 0 {
        return (csr_from_rows(&[], n), Vec::new());
    }

    let rows: Vec<(Vec<(usize, f64)>, f64)> = (0..p)
        .map(|i| {
            let row: Vec<(usize, f64)> = csr_row_vec(a, i);
            (row, b[i])
        })
        .collect();

    let deduped = dedupe_rows(rows);
    let nnz: usize = deduped.iter().map(|(row, _)| row.len()).sum();
    let density = nnz as f64 / (deduped.len() as f64 * n.max(1) as f64);
    let keep = if density > tunable!("ENOMOTO_T_DENSE_DENSITY_THRESHOLD", DENSE_DENSITY_THRESHOLD, f64) {
        drop_linearly_dependent(&deduped, n)
    } else {
        drop_linearly_dependent_sparse_blocked(&deduped, n, lb, ub)
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
    // Same decisions as keying a `HashSet` on the normalized signature
    // `[(j, bits(v/pivot))..., (usize::MAX, bits(rhs/pivot))]` (kept below
    // as `dedupe_rows_reference` for the equivalence test), but without
    // materializing each signature as its own `Vec` and SipHash-ing it:
    // the signature is hashed on the fly (the same multiplicative mix
    // `reduce_inequalities` uses) and a hash hit is confirmed by
    // recomputing the kept row's signature — normalization is
    // deterministic — and comparing it entry by entry.
    #[inline]
    fn mix(hash: u64, x: u64) -> u64 {
        (hash.rotate_left(5) ^ x).wrapping_mul(0x517c_c1b7_2722_0a95)
    }
    let mut heads: HashMap<u64, usize, std::hash::BuildHasherDefault<IdentityU64Hasher>> = HashMap::with_capacity_and_hasher(rows.len(), Default::default());
    // Per kept row: its inverse pivot and the next kept row in the same
    // hash chain.
    let mut chain: Vec<(f64, usize)> = Vec::with_capacity(rows.len());
    let mut kept: Vec<(Vec<(usize, f64)>, f64)> = Vec::with_capacity(rows.len());
    // `chain` is indexed like `kept` except for empty rows, which never
    // enter a chain; `slot_of_chain[k]` maps chain entry -> `kept` index.
    let mut slot_of_chain: Vec<usize> = Vec::with_capacity(rows.len());
    for (row, rhs) in rows {
        if row.is_empty() {
            if rhs != 0.0 {
                kept.push((row, rhs));
            }
            continue;
        }
        let pivot = row[0].1;
        let inv = 1.0 / pivot;
        let mut hash = row.len() as u64;
        for &(j, v) in &row {
            hash = mix(mix(hash, j as u64), (v * inv).to_bits());
        }
        let rhs_bits = (rhs * inv).to_bits();
        hash = mix(hash, rhs_bits);
        let head = heads.get(&hash).copied().unwrap_or(usize::MAX);
        let mut cur = head;
        let mut dup = false;
        while cur != usize::MAX {
            let (k_inv, next) = chain[cur];
            let (k_row, k_rhs) = &kept[slot_of_chain[cur]];
            if k_row.len() == row.len()
                && (k_rhs * k_inv).to_bits() == rhs_bits
                && k_row.iter().zip(&row).all(|(&(kj, kv), &(j, v))| kj == j && (kv * k_inv).to_bits() == (v * inv).to_bits())
            {
                dup = true;
                break;
            }
            cur = next;
        }
        if !dup {
            heads.insert(hash, chain.len());
            chain.push((inv, head));
            slot_of_chain.push(kept.len());
            kept.push((row, rhs));
        }
    }
    kept
}

#[cfg(test)]
fn dedupe_rows_reference(rows: Vec<(Vec<(usize, f64)>, f64)>) -> Vec<(Vec<(usize, f64)>, f64)> {
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

    // Column-pivoted QR in place, explicitly sequential. `ColPivQr::new`
    // would read faer's *global* parallelism (`Rayon(0)` with the `rayon`
    // feature on), which for the sizes seen here (at most a few thousand x a
    // few hundred) buys nothing: on a fresh process it is what first spins up
    // rayon's global pool (4 thread spawns + per-thread arenas, ~0.5 ms), every
    // later call pays the hand-off to the pool, and the parallel reduction
    // order made `wood1p`'s dropped-row set vary from run to run. Only the
    // diagonal of R (= the diagonal of the in-place factors) and the column
    // permutation are needed below.
    let size = aug_n.min(p);
    let blocksize = colpiv_qr::recommended_blocksize::<f64>(aug_n, p);
    let mut householder = Mat::<f64>::zeros(blocksize, size);
    let mut col_perm = vec![0usize; p];
    let mut col_perm_inv = vec![0usize; p];
    let params = Default::default();
    colpiv_qr::qr_in_place(
        m.as_mut(),
        householder.as_mut(),
        &mut col_perm,
        &mut col_perm_inv,
        Parallelism::None,
        PodStack::new(&mut GlobalPodBuffer::new(
            colpiv_qr::qr_in_place_req::<usize, f64>(aug_n, p, blocksize, Parallelism::None, params).unwrap(),
        )),
        params,
    );
    let r = &m; // upper triangle holds R
    let fwd = &col_perm;

    let rank_dim = size;

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
        if r[(k, k)].abs() <= tunable!("ENOMOTO_T_REDEQ_QR_RANK_TOL", 1e-9, f64) * col_norm(orig).max(1e-300) {
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
    // Lazy maintenance of `heap`: a column's key is only guaranteed to be
    // an *upper bound* on its true current max-abs entry; `col_exact[j]`
    // records whether it is known to be exactly equal. Removals / shrinking
    // updates that may have lowered the max only clear `col_exact[j]` (O(1))
    // instead of rescanning the whole column; the rescan happens only when
    // that column actually reaches the top of `heap` (validation loop at the
    // start of every step). Since every key is >= its column's true max, the
    // first *exact* top is the true global max — bit-identical to the eager
    // version, which rescanned every touched column (the rhs column `n`
    // and other long columns included) after every single step.
    let mut col_exact = vec![true; aug_n];
    // `old_abs`: the magnitude an entry of column `j` had before being
    // removed/changed (`0.0` if it did not exist); `new_abs`: its magnitude
    // afterwards (`0.0` if removed).
    fn note_change(heap: &mut BTreeSet<(u64, usize)>, col_bits: &mut [Option<u64>], col_exact: &mut [bool], j: usize, old_abs: f64, new_abs: f64) {
        let key = col_bits[j].map_or(0.0, f64::from_bits);
        if new_abs > key {
            match col_bits[j].take() {
                Some(old) => {
                    heap.remove(&(old, j));
                }
                // Not in `heap` means the column's true max was 0, so the
                // new entry is now exactly its max.
                None => col_exact[j] = true,
            }
            let bits = new_abs.to_bits();
            heap.insert((bits, j));
            col_bits[j] = Some(bits);
            // exactness unchanged: if the old key was exact, the new entry
            // is now the strict maximum; if it was only an upper bound, the
            // new key still is one.
        } else if old_abs >= key {
            col_exact[j] = false;
        }
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
        let gmax = loop {
            match heap.iter().next_back() {
                Some(&(bits, j)) => {
                    if col_exact[j] {
                        break Some(f64::from_bits(bits));
                    }
                    refresh_col(&col_rows, &rows, &mut heap, &mut col_bits, j);
                    col_exact[j] = true;
                }
                None => break None,
            }
        };
        let Some(gmax) = gmax else {
            break; // every active column is entirely zero
        };
        let threshold = tunable!("ENOMOTO_T_REDEQ_PIVOT_STABILITY", PIVOT_STABILITY, f64) * gmax;

        // Ascending-degree bucket scan over columns, exactly like
        // `find_best_pivot`, but a candidate is only ever recorded into
        // `best` once it passes the *global* `threshold` above — see this
        // function's own docs for why that, not local column-relative
        // magnitude, is what actually determines a correct rank here.
        PROF_TOTAL_STEPS.fetch_add(1, Ordering::Relaxed);
        let mut best: Option<(usize, usize)> = None;
        let mut best_score = usize::MAX;
        let mut best_abs = 0.0f64;
        'search: for deg_col in 1..col_buckets.len() {
            for &j in &col_buckets[deg_col] {
                if col_used[j] {
                    continue;
                }
                // `col_bits[j]` is an upper bound on column `j`'s largest
                // active |entry| (see the lazy-heap notes above; `None` =
                // column is entirely zero). Below `threshold`, every entry
                // would fail the `v.abs() < threshold` test in the scan
                // below, so the scan could not record a candidate — skip it
                // in O(1). Same pivot choice, bit for bit; this is what keeps
                // long runs of low-degree, tiny-valued columns (`dfl001`)
                // from being rescanned on every single step.
                if col_bits[j].map_or(true, |bits| f64::from_bits(bits) < threshold) {
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
                    PROF_TRIVIAL_STEPS.fetch_add(1, Ordering::Relaxed);
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
            if !col_used[j] {
                let old_abs = rows[pi].get(&j).map_or(0.0, |v| v.abs());
                note_change(&mut heap, &mut col_bits, &mut col_exact, j, old_abs, 0.0);
            }
        }

        // Dependency test: is what's left of this row, at the point it
        // was chosen, negligible relative to its own *original* scale?
        if pivot_val.abs() <= tunable!("ENOMOTO_T_REDEQ_DEP_TOL", DEP_TOL, f64) * row_orig_norm[pi].max(1e-300) {
            // Dependent: drop it (leave `keep[pi] = false`) without
            // eliminating — it contributes no independent structure to
            // scatter into the other rows. This branch never reaches the
            // post-elimination refresh pass below, so `pi_cols` must be
            // refreshed here instead — otherwise a stale, too-high entry
            // could survive in `heap` for a column whose recorded max
            // came only from `pi`, wrongly gating out a genuinely valid
            // pivot elsewhere via an inflated `gmax` on a later step.
            // (Lazy heap: `pi_cols`' possible max decrease was already
            // recorded via `note_change` just above.)
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
                        let old_val = *e.get();
                        let new_val = old_val - mult * v;
                        if !col_used[j] {
                            note_change(&mut heap, &mut col_bits, &mut col_exact, j, old_val.abs(), new_val.abs());
                        }
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
                            if !col_used[j] {
                                note_change(&mut heap, &mut col_bits, &mut col_exact, j, 0.0, new_val.abs());
                            }
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

        // (Lazy heap: every value change above was already recorded via
        // `note_change`; no per-column rescan here.)
    }

    (0..p).filter(|&i| keep[i]).collect()
}

/// Partitions `rows` into blocks via a Dulmage-Mendelsohn-style
/// decomposition — thin wrapper around the shared [`crate::graph`]
/// implementation (maximum bipartite matching plus Tarjan
/// strongly-connected-components; see that module's own docs for the
/// algorithm), passing just each row's own nonzero-column pattern as
/// adjacency. Strictly finer than a plain connected-components partition
/// of the same bipartite graph would be — two rows sharing no column at
/// all can never end up in the same SCC either, since the matching
/// graph's edges are themselves derived from real nonzeros — so this
/// never *loses* the block-diagonal structure a simpler decomposition
/// would already find; real Netlib instances that show as a single
/// connected component by raw column-sharing alone (`shell`, `scsd8`,
/// `fit1p`, `ganges`) decompose into hundreds of much smaller SCCs this
/// way instead (`shell`: 531 blocks from 534 rows; `ganges`: 998 from
/// 1284), most of them singletons.
///
/// **Why checking each block in isolation is sound here, unlike using
/// this same decomposition for LU factorization/solving**: LU needs
/// blocks processed in dependency order because it *propagates computed
/// values* forward through the matrix (well, needs it for *solving* —
/// see `crate::graph::dulmage_mendelsohn_blocks`'s own docs on why even
/// LU *factorization* itself, as opposed to solving, turns out not to
/// need that ordering after all, since off-diagonal spillover entries
/// are carried through unchanged rather than requiring elimination).
/// Redundancy detection asks a different, purely *local* question per
/// row — "is row `R` exactly equal to some linear combination of these
/// specific other rows?" — and that identity, once verified using the
/// rows' full, untruncated content (not just their entries in the
/// block's own columns), holds unconditionally regardless of what any
/// other block contains. So every block found here can be checked
/// independently, in any order, including concurrently — the same
/// parallel dispatch [`drop_linearly_dependent_sparse_blocked`] applies
/// unchanged. The only cost of this independence is *recall*, not
/// soundness: a redundancy whose witnessing combination genuinely spans
/// multiple blocks (possible here, unlike with disjoint-column
/// components, since an earlier block's row can still have nonzeros
/// reaching into a later block's own columns) goes undetected and that
/// row is conservatively kept — never the reverse (an independent row is
/// never wrongly dropped), so this trades a little reduction
/// *aggressiveness* for a lot more exploitable structure, the same trade
/// already accepted for the rhs-augmentation edge case below.
/// Builds the bipartite adjacency from each row's *structural* nonzeros —
/// except a coefficient [`smallcoeff::clean_row`] judges negligible for
/// that row's own worst-case activity (Achterberg, Bixby, Gu, Rothberg &
/// Weninger, "Presolve Reductions in Mixed Integer Programming", §3.1) is
/// left out of the edge set entirely, so two rows linked only by such a
/// coefficient are no longer forced into the same block by it.
///
/// **Non-destructive**: `rows` itself — what every block's own
/// [`drop_linearly_dependent_sparse`] call actually eliminates against —
/// is untouched; only which edges this decomposition *sees* changes.
/// [`smallcoeff`]'s own module docs record two prior attempts at wiring
/// its reduction into the live pipeline, each reverted after it
/// numerically destabilized a real instance (`perold` newly crashing,
/// `beale_cycling_example_terminates_correctly` newly failing its IPM
/// cross-check) — both traced to the reduction *mutating* a row/rhs a
/// later stage then solved against. Using the exact same negligibility
/// test only to decide which edges feed a graph algorithm carries none of
/// that risk: per this module's own docs on why every block found here is
/// sound to check independently, dropping an edge (even a "real" one)
/// only costs *recall* — a redundancy whose witness spans two blocks this
/// now separates goes undetected and that row is conservatively kept,
/// never the reverse — the identical trade-off this decomposition's own
/// matching-vs-plain-connected-components choice already accepts.
///
/// **Measured (instrumented directly, not inferred from timing) against
/// real Netlib instances already known to exercise this decomposition**:
/// the filter is far from a no-op on some of them — `shell` drops 500 of
/// 3550 structural edges (14%), `25fv47` 72 of 3609, `sierra` 40 of 3973
/// — but the resulting block *count* barely moves either way (`shell`
/// 529 -> 524, `sierra` 438 -> 438 unchanged, `scfxm3` 326 -> 329,
/// `25fv47` 247 -> 241): most negligible coefficients turn out to sit
/// inside a block the matching would have kept together anyway on other,
/// non-negligible edges, not to be the sole bridge between two blocks.
/// `25fv47` landing on *fewer* blocks after filtering (not more) is not a
/// soundness concern — a maximum bipartite matching is generally
/// non-unique, so removing an edge can steer the matcher to a different
/// one with its own, differently-shaped SCC condensation; every block
/// either matching produces is independently sound per this function's
/// own docs above, just not guaranteed monotonic in *count* the way plain
/// connected components would be. A full-Netlib wall-clock A/B (73
/// in-scope instances, 3 repeats each side) showed no aggregate
/// difference distinguishable from this machine's own run-to-run noise
/// (both sides landed in the same ~4.1-5.1s band) — consistent with the
/// small, block-count-neutral effect measured directly above.
fn dulmage_mendelsohn_blocks(rows: &[(Vec<(usize, f64)>, f64)], n: usize, lb: &[f64], ub: &[f64]) -> Vec<Vec<usize>> {
    let adj: Vec<Vec<usize>> = rows
        .iter()
        .map(|(row, _)| smallcoeff::clean_row(row, 0.0, lb, ub).0.into_iter().map(|(j, _)| j).collect())
        .collect();
    crate::graph::dulmage_mendelsohn_blocks(&adj, n)
}

/// Below this total row count across a decomposition's non-trivial
/// (size > 1) components, [`drop_linearly_dependent_sparse_blocked`] runs
/// them sequentially rather than via `rayon` — see that function's own
/// docs for why, unlike every *other* `rayon` call site in this crate
/// (all gated by `RAYON_SIZE_THRESHOLD`-style raw *problem* size, per
/// `simplex.rs`'s own docs on measured per-element dispatch overhead),
/// the right threshold here is total row count *within the blocks
/// actually being split*, since a component's own elimination is real,
/// non-trivial work per row (unlike a cheap per-element scan) — a modest
/// absolute row count here still comfortably pays for `rayon`'s task
/// dispatch.
const PARALLEL_DECOMPOSE_ROW_THRESHOLD: usize = 64;

/// Below this many equality rows, [`drop_linearly_dependent_sparse_blocked`]
/// skips [`dulmage_mendelsohn_blocks`] entirely and calls
/// [`drop_linearly_dependent_sparse`] directly, rather than always paying
/// for the bipartite-matching-plus-SCC pass (and, if it does find multiple
/// blocks, the per-block `HashMap`-based column remapping and fresh
/// `BTreeMap`/bucket/heap scaffolding for each one). Measured directly (with
/// the earlier, coarser connected-components version of this same
/// decomposition, before it was replaced by the finer Dulmage-Mendelsohn
/// one — the size/regression picture below is unaffected by that swap,
/// since both pay similar decomposition overhead on tiny inputs): every
/// real Netlib win from decomposition (`ship12s` `p=1045`, `ship08s`
/// `p=698`, `ship04l`/`ship04s` `p=354`, `sierra` `p=528`) has `p` well
/// above this; every case that regressed when decomposition ran
/// unconditionally (`sc105` `p=45`, `scorpion` `p=280`, `sc205` `p=91`,
/// `capri` `p=142`, `standgub`/`standata` `p=160`, `recipe` `p=67`,
/// `bore3d` `p=214`) sits below it — all by a comfortable margin, so `300`
/// is not a tight cutoff. Every one of those regressions was itself only
/// a fraction of a millisecond in absolute terms (these are already
/// sub-10ms problems), but with nothing to gain there either — the
/// decomposition's benefit scales with how much per-row elimination work
/// it *avoids* doing across blocks, which is negligible when the whole
/// problem is this small to begin with.
const MIN_ROWS_FOR_BLOCK_DECOMPOSE: usize = 300;

/// Wraps [`drop_linearly_dependent_sparse`] with a Dulmage-Mendelsohn-style
/// block-triangularization pre-pass (see [`dulmage_mendelsohn_blocks`]):
/// the equality system is first split into blocks via a maximum bipartite
/// matching plus strongly-connected-components search, each solved by
/// calling the same core algorithm on just that block's rows with columns
/// remapped to a compact local index range (`0..local_n`) — without that
/// remapping, every block's call would still pay for `aug_n`-sized scratch
/// arrays (`col_rows`, `col_buckets`, `heap`/`col_bits`) proportional to
/// the *whole* problem's `n`, defeating the point of splitting at all.
///
/// Real Netlib multi-vessel/multi-period scheduling LPs (`ship12s`,
/// `ship08s`, `ship04l`, `ship04s`, `sierra`) decompose into dozens of
/// blocks of *nearly identical size* this way (`ship12s`: 12 blocks of
/// exactly 78 rows each, plus 109 size-1 singletons) — one instance per
/// vessel/route/period. A first version of this decomposition used plain
/// connected components (disjoint column support only) and stopped there,
/// since it correctly found *those* instances but left several others
/// (`shell`, `scsd8`, `fit1p`, `ganges`) showing as a single, fully-coupled
/// component with nothing to split. Replacing it with the full
/// Dulmage-Mendelsohn matching-plus-SCC decomposition finds much finer
/// structure in exactly those remaining instances too (`shell`: 531 blocks
/// from 534 rows; `ganges`: 998 from 1284; `fit1p`: 605 from 627) — the
/// matching exploits a *directional* dependency structure (a block's rows
/// can still reach into a later block's own columns) that pure
/// column-disjointness can never see, since two rows sharing a column can
/// still end up in different SCCs as long as the dependency isn't mutual.
/// `wood1p` stays on the *dense* path entirely (density dispatch above)
/// and is unaffected either way.
///
/// **Why every block found here is still sound to check independently,
/// unlike using this same decomposition for LU factorization or
/// solving**: LU needs strict block order because it propagates *computed
/// values* forward — a later block's solve genuinely depends on an earlier
/// block's result. Redundancy detection instead asks, per row, "is this
/// row exactly equal to some linear combination of these specific other
/// rows?" — an identity that, once verified using the rows' full,
/// untruncated content, holds unconditionally regardless of what any other
/// block contains. So unlike a general block-triangular form's usual
/// sequential constraint, every block here can be checked in any order,
/// including concurrently — the only cost is *recall*, not soundness: a
/// redundancy whose witnessing combination genuinely spans multiple blocks
/// (possible here, since an earlier block's row can still reach into a
/// later block's columns — impossible with plain connected components,
/// where blocks share no column at all) goes undetected and that row is
/// conservatively kept, never the reverse. See
/// [`dulmage_mendelsohn_blocks`]'s own docs for the full argument.
///
/// **The rhs-augmentation edge case**: [`drop_linearly_dependent_sparse`]
/// augments each row with the equation's rhs as one extra shared column
/// (index `n`), used to distinguish genuine redundancy from an
/// inconsistency (Farkas infeasibility witness) — but
/// [`dulmage_mendelsohn_blocks`] deliberately does not treat that column as
/// a graph edge, so two *originally* all-zero-coefficient rows with
/// different nonzero rhs (`0 = 5`, `0 = 3`) land in separate singleton
/// blocks here, whereas the un-decomposed algorithm's single shared rhs
/// column would link them and drop one as "dependent" on the other. Both
/// outcomes are correct — each such row is already its own infeasibility
/// witness on its own, so dropping one loses no information the solver
/// needs — this function is just more conservative (keeps a
/// possibly-redundant-but-harmless extra row) in that one narrow,
/// degenerate edge case. The same reasoning applies to a row that only
/// reduces to a pure rhs residual *during* elimination (the general Farkas
/// case): that reduction happens entirely from real columns within one
/// block, so it is still caught correctly and entirely locally.
fn drop_linearly_dependent_sparse_blocked(rows_in: &[(Vec<(usize, f64)>, f64)], n: usize, lb: &[f64], ub: &[f64]) -> Vec<usize> {
    if rows_in.len() < tunable!("ENOMOTO_T_MIN_ROWS_FOR_BLOCK_DECOMPOSE", MIN_ROWS_FOR_BLOCK_DECOMPOSE, usize) {
        return drop_linearly_dependent_sparse(rows_in, n);
    }
    let components = dulmage_mendelsohn_blocks(rows_in, n, lb, ub);
    if components.len() <= 1 {
        return drop_linearly_dependent_sparse(rows_in, n);
    }

    let mut kept: Vec<usize> = Vec::new();
    let mut nontrivial: Vec<Vec<usize>> = Vec::with_capacity(components.len());
    for comp in components {
        if comp.len() == 1 {
            // A row with zero real-column edges to anything else cannot
            // be a linear combination of any other row's real
            // coefficients — the only way it could be "dependent" is by
            // literally being the zero vector including its rhs, which
            // `dedupe_rows` already drops as a trivial `0 = 0` row before
            // this function ever sees it. Always kept, no elimination
            // machinery needed at all.
            kept.push(comp[0]);
        } else {
            nontrivial.push(comp);
        }
    }

    // Longest-processing-time-first: the biggest components are hardest
    // to load-balance, so dispatching them first gives `rayon`'s
    // work-stealing scheduler the best chance of not stranding two large
    // blocks on the same thread behind a run of smaller ones.
    nontrivial.sort_by_key(|c| std::cmp::Reverse(c.len()));

    let solve_component = |comp: &[usize]| -> Vec<usize> {
        let mut col_map: HashMap<usize, usize> = HashMap::new();
        let local_rows: Vec<(Vec<(usize, f64)>, f64)> = comp
            .iter()
            .map(|&i| {
                let (row, rhs) = &rows_in[i];
                let local_row = row
                    .iter()
                    .map(|&(j, v)| {
                        let next_id = col_map.len();
                        let lj = *col_map.entry(j).or_insert(next_id);
                        (lj, v)
                    })
                    .collect();
                (local_row, *rhs)
            })
            .collect();
        let local_n = col_map.len();
        drop_linearly_dependent_sparse(&local_rows, local_n)
            .into_iter()
            .map(|local_idx| comp[local_idx])
            .collect::<Vec<usize>>()
    };

    let total_nontrivial_rows: usize = nontrivial.iter().map(|c| c.len()).sum();
    if nontrivial.len() > 1 && total_nontrivial_rows >= tunable!("ENOMOTO_T_PARALLEL_DECOMPOSE_ROW_THRESHOLD", PARALLEL_DECOMPOSE_ROW_THRESHOLD, usize) {
        use rayon::prelude::*;
        kept.extend(nontrivial.par_iter().flat_map(|c| solve_component(c)).collect::<Vec<usize>>());
    } else {
        kept.extend(nontrivial.iter().flat_map(|c| solve_component(c)));
    }

    kept.sort_unstable();
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hashed/in-place `reduce_inequalities` must make exactly the
    /// decisions of the straightforward reference version and return a
    /// bit-identical `(G, h)` — on inputs with exact and scaled duplicates,
    /// sign-flipped near-duplicates, empty rows and unsorted input.
    #[test]
    fn reduce_inequalities_matches_reference_bit_for_bit() {
        let mut state: u64 = 0x1234_5678_9abc_def0;
        let mut rnd = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for trial in 0..200 {
            let n = 6 + (trial % 5);
            let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
            let mut h: Vec<f64> = Vec::new();
            let base_count = 3 + (rnd() % 6) as usize;
            for _ in 0..base_count {
                let len = (rnd() % 4) as usize;
                let mut row: Vec<(usize, f64)> = Vec::new();
                for _ in 0..len {
                    let j = (rnd() % n as u64) as usize;
                    if row.iter().all(|&(k, _)| k != j) {
                        row.push((j, ((rnd() % 7) as f64 - 3.0) * 0.5 + 0.25));
                    }
                }
                rows.push(row);
                h.push((rnd() % 11) as f64 - 5.0);
            }
            // Scaled copies (positive and negative factors) of random rows.
            for _ in 0..base_count {
                let src = (rnd() % base_count as u64) as usize;
                let f = [1.0, 2.0, 0.5, 3.0, -1.0, 1.0 / 3.0][(rnd() % 6) as usize];
                rows.push(rows[src].iter().map(|&(j, v)| (j, v * f)).collect());
                h.push(h[src] * f + ((rnd() % 3) as f64 - 1.0));
            }
            let g = csr_from_rows(&rows, n);
            let (g_ref, h_ref) = reduce_inequalities_reference(&g, &h, n);
            let (g_new, h_new) = reduce_inequalities(&g, &h, n);
            assert_eq!(g_new.as_ref().row_ptrs(), g_ref.as_ref().row_ptrs(), "trial {trial}");
            assert_eq!(g_new.as_ref().col_indices(), g_ref.as_ref().col_indices(), "trial {trial}");
            let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(g_new.as_ref().values()), bits(g_ref.as_ref().values()), "trial {trial}");
            assert_eq!(bits(&h_new), bits(&h_ref), "trial {trial}");
        }
    }

    /// `dedupe_rows` (on-the-fly u64 hash + chain) must keep exactly the
    /// rows the `HashSet<Vec<_>>` reference keeps, in the same order.
    #[test]
    fn dedupe_rows_matches_reference_bit_for_bit() {
        let mut state: u64 = 0x0fed_cba9_8765_4321;
        let mut rnd = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for trial in 0..300 {
            let n = 4 + (trial % 5);
            let mut rows: Vec<(Vec<(usize, f64)>, f64)> = Vec::new();
            let base_count = 2 + (rnd() % 6) as usize;
            for _ in 0..base_count {
                let len = (rnd() % 4) as usize;
                let mut row: Vec<(usize, f64)> = Vec::new();
                for _ in 0..len {
                    let j = (rnd() % n as u64) as usize;
                    if row.iter().all(|&(k, _)| k != j) {
                        row.push((j, ((rnd() % 7) as f64 - 3.0) * 0.5 + 0.25));
                    }
                }
                rows.push((row, [0.0, 1.0, -2.0, 0.5][(rnd() % 4) as usize]));
            }
            for _ in 0..base_count {
                let src = (rnd() % base_count as u64) as usize;
                let f = [1.0, 2.0, 0.5, 3.0, -1.0, 1.0 / 3.0][(rnd() % 6) as usize];
                let (r, b) = rows[src].clone();
                let db = [0.0, 0.0, 1.0][(rnd() % 3) as usize];
                rows.push((r.iter().map(|&(j, v)| (j, v * f)).collect(), b * f + db));
            }
            let got = dedupe_rows(rows.clone());
            let want = dedupe_rows_reference(rows);
            assert_eq!(got.len(), want.len(), "trial {trial}");
            for ((gr, gb), (wr, wb)) in got.iter().zip(&want) {
                assert_eq!(gb.to_bits(), wb.to_bits(), "trial {trial}");
                assert_eq!(gr.len(), wr.len(), "trial {trial}");
                for (&(gj, gv), &(wj, wv)) in gr.iter().zip(wr) {
                    assert_eq!((gj, gv.to_bits()), (wj, wv.to_bits()), "trial {trial}");
                }
            }
        }
    }

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

    /// Two mutually-referencing (genuine 2-cycle in the matching graph,
    /// however the matching happens to pick columns) blocks — rows
    /// `{0,1}` over columns `{0,1}`, rows `{2,3}` over columns `{2,3}` —
    /// plus one truly isolated row (`{4}`, column `{4}`) must land in
    /// exactly three blocks, each in ascending row order, sorted by first
    /// row: a case where the finer Dulmage-Mendelsohn decomposition must
    /// still agree with what plain column-disjointness alone would find.
    #[test]
    fn dulmage_mendelsohn_blocks_splits_disjoint_cyclic_groups() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 1.0), (1, 2.0)], 3.0),
            (vec![(0, 2.0), (1, 1.0)], 4.0),
            (vec![(2, 1.0), (3, 1.0)], 1.0),
            (vec![(2, 1.0), (3, 2.0)], 2.0),
            (vec![(4, 5.0)], 5.0),
        ];
        let comps = dulmage_mendelsohn_blocks(&rows, 5, &[f64::NEG_INFINITY; 5], &[f64::INFINITY; 5]);
        assert_eq!(comps, vec![vec![0, 1], vec![2, 3], vec![4]], "comps={comps:?}");
    }

    /// A pure dependency *chain* (row `i` and `i+1` always share a column,
    /// but never cyclically — row `i` never depends back on row `i+1`)
    /// stays one connected component under plain column-sharing alone, but
    /// has *no* genuine cycles in the matching graph, so the finer
    /// Dulmage-Mendelsohn decomposition must split it into `n` singleton
    /// blocks — exactly the structure real instances like `fit1p`/`ganges`
    /// showed (hundreds of singleton SCCs) despite looking like one
    /// fully-coupled component by column-sharing alone.
    #[test]
    fn dulmage_mendelsohn_blocks_splits_a_pure_chain_into_singletons() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 1.0), (1, 1.0)], 1.0),
            (vec![(1, 1.0), (2, 1.0)], 1.0),
            (vec![(2, 1.0), (3, 1.0)], 1.0),
        ];
        let comps = dulmage_mendelsohn_blocks(&rows, 4, &[f64::NEG_INFINITY; 4], &[f64::INFINITY; 4]);
        assert_eq!(comps, vec![vec![0], vec![1], vec![2]], "comps={comps:?}");
    }

    /// The blocked wrapper must match the un-decomposed sparse algorithm's
    /// rank count on a case that is all *one* component (no decomposition
    /// possible) — exercises the `components.len() <= 1` direct-fallback
    /// path specifically.
    #[test]
    fn drop_linearly_dependent_sparse_blocked_matches_unblocked_on_a_single_component() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 1.0), (1, 2.0)], 5.0),
            (vec![(1, 1.0), (2, 3.0)], 7.0),
            (vec![(0, 2.0), (2, 1.0)], 4.0),
        ];
        let keep = drop_linearly_dependent_sparse_blocked(&rows, 3, &[f64::NEG_INFINITY; 3], &[f64::INFINITY; 3]);
        assert_eq!(keep, vec![0, 1, 2], "keep={keep:?}");
    }

    /// Two independent blocks, each internally rank-deficient by exactly
    /// one row, stitched together in a single call — the redundant row in
    /// block A must not affect block B's own (independent) redundant row
    /// and vice versa, and the combined result must be exactly rank
    /// `2 + 2 = 4` out of the 6 rows the two blocks contribute. Each
    /// block's own 3 rows form a genuine cycle in the matching graph (row
    /// 2 touches all of columns {0,1,2}, so whichever column the maximum
    /// matching assigns it, tracing back through the other two rows'
    /// matches closes a cycle), so [`dulmage_mendelsohn_blocks`] keeps
    /// each block together as one SCC — and the two blocks share no
    /// column at all, so they land in different SCCs from each other.
    /// Padded with 300 trivially-independent singleton rows (each its own
    /// isolated column, always kept — see
    /// [`drop_linearly_dependent_sparse_blocked`]'s own docs) purely to
    /// clear [`MIN_ROWS_FOR_BLOCK_DECOMPOSE`] and actually exercise
    /// [`dulmage_mendelsohn_blocks`] rather than that size gate's direct
    /// fallback — the singletons are otherwise inert and checked only in
    /// aggregate.
    #[test]
    fn drop_linearly_dependent_sparse_blocked_handles_independent_blocks_separately() {
        let mut rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            // Block A: columns {0,1,2}, row 2 = row0 + row1 (x0+2x1+x2=7).
            (vec![(0, 1.0), (1, 1.0)], 3.0),
            (vec![(1, 1.0), (2, 1.0)], 4.0),
            (vec![(0, 1.0), (1, 2.0), (2, 1.0)], 7.0),
            // Block B: columns {3,4,5}, row 5 = 2*row3 - row4 (2x3+x4-x5=-1).
            (vec![(3, 1.0), (4, 1.0)], 2.0),
            (vec![(4, 1.0), (5, 1.0)], 5.0),
            (vec![(3, 2.0), (4, 1.0), (5, -1.0)], -1.0),
        ];
        let filler_count = MIN_ROWS_FOR_BLOCK_DECOMPOSE;
        for k in 0..filler_count {
            rows.push((vec![(6 + k, 1.0)], 1.0));
        }
        let n = 6 + filler_count;
        assert!(rows.len() >= MIN_ROWS_FOR_BLOCK_DECOMPOSE, "test must actually exercise dulmage_mendelsohn_blocks");

        let keep = drop_linearly_dependent_sparse_blocked(&rows, n, &vec![f64::NEG_INFINITY; n], &vec![f64::INFINITY; n]);
        assert_eq!(keep.len(), 4 + filler_count, "expected rank 2+2+{filler_count} singletons; keep={keep:?}");
        let keep_a = keep.iter().filter(|&&i| i < 3).count();
        let keep_b = keep.iter().filter(|&&i| (3..6).contains(&i)).count();
        let keep_filler = keep.iter().filter(|&&i| i >= 6).count();
        assert_eq!(keep_a, 2, "block A must independently reduce to rank 2; keep={keep:?}");
        assert_eq!(keep_b, 2, "block B must independently reduce to rank 2; keep={keep:?}");
        assert_eq!(keep_filler, filler_count, "every isolated singleton row must survive; keep={keep:?}");
    }

    /// Same as the previous test but with enough repeated blocks to push
    /// `drop_linearly_dependent_sparse_blocked` past both
    /// `MIN_ROWS_FOR_BLOCK_DECOMPOSE` (so it doesn't take the small-input
    /// direct-fallback path at all) and `PARALLEL_DECOMPOSE_ROW_THRESHOLD`
    /// (so it actually dispatches via `rayon` rather than iterating
    /// sequentially) — every block is an independent copy of the same
    /// rank-2 (out of 3 rows) pattern on disjoint columns, so the correct
    /// answer is mechanically checkable (rank `2 * block_count`)
    /// regardless of which thread processes which block.
    #[test]
    fn drop_linearly_dependent_sparse_blocked_matches_sequential_result_under_parallel_dispatch() {
        let block_count = 120; // 120 * 3 = 360 rows, clears both thresholds above.
        let mut rows: Vec<(Vec<(usize, f64)>, f64)> = Vec::new();
        for b in 0..block_count {
            let base = b * 3;
            rows.push((vec![(base, 1.0), (base + 1, 1.0)], 3.0));
            rows.push((vec![(base + 1, 1.0), (base + 2, 1.0)], 4.0));
            rows.push((vec![(base, 1.0), (base + 1, 2.0), (base + 2, 1.0)], 7.0)); // = row0 + row1
        }
        let n = block_count * 3;
        assert!(rows.len() >= MIN_ROWS_FOR_BLOCK_DECOMPOSE, "test must clear the small-input fallback gate");
        assert!(rows.len() >= PARALLEL_DECOMPOSE_ROW_THRESHOLD, "test must actually exercise the parallel path");
        let keep = drop_linearly_dependent_sparse_blocked(&rows, n, &vec![f64::NEG_INFINITY; n], &vec![f64::INFINITY; n]);
        assert_eq!(keep.len(), block_count * 2, "expected rank 2 per block; keep={keep:?}");
        for b in 0..block_count {
            let in_block = keep.iter().filter(|&&i| i / 3 == b).count();
            assert_eq!(in_block, 2, "block {b} must independently reduce to rank 2; keep={keep:?}");
        }
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

    // Same decisions as the straightforward version (kept below as
    // `reduce_inequalities_reference` for the equivalence test), but
    // without materializing every row and every normalized signature as
    // its own `Vec` and SipHash-ing it: rows are read straight from the CSR
    // slices, each signature is hashed on the fly with a cheap
    // multiplicative mix, and a hash hit is confirmed by recomputing the
    // class representative's signature (bit-for-bit the one the reference
    // version would have stored as the key, since normalization is
    // deterministic) and comparing it entry by entry.
    #[inline]
    fn mix(hash: u64, x: u64) -> u64 {
        (hash.rotate_left(5) ^ x).wrapping_mul(0x517c_c1b7_2722_0a95)
    }
    struct Class {
        rep: usize,
        rep_inv: f64,
        kept_idx: usize,
        kept_h: f64,
        next: usize,
    }
    // Single-entry rows (the box-bound rows `rebuild_g` emits, usually the
    // bulk of `g`) are not hashed: their class chain starts at
    // `unit_head[j]` instead, keyed by their only column (a class's
    // signature still decides membership, exactly as for a hashed row —
    // rows of different length never share a class, so splitting the
    // lookup this way changes no decision). Only multi-entry rows go
    // through `heads`, sized up front.
    let multi_rows = (0..m).filter(|&i| gr.col_indices_of_row_raw(i).len() > 1).count();
    let mut heads: HashMap<u64, usize, std::hash::BuildHasherDefault<IdentityU64Hasher>> = HashMap::with_capacity_and_hasher(multi_rows, Default::default());
    let mut unit_head: Vec<usize> = vec![usize::MAX; n];
    let mut classes: Vec<Class> = Vec::with_capacity(m);
    let mut keep = vec![true; m];
    let mut any_dropped = false;
    for idx in 0..m {
        let cols = gr.col_indices_of_row_raw(idx);
        let vals = gr.values_of_row(idx);
        if cols.is_empty() {
            continue;
        }
        let hv = h[idx];
        let scale = vals[0].abs();
        let inv = 1.0 / scale;
        let unit = cols.len() == 1;
        let mut hash = cols.len() as u64;
        if !unit {
            for (&j, &v) in cols.iter().zip(vals) {
                hash = mix(mix(hash, j as u64), (v * inv).to_bits());
            }
        }
        let normalized_h = hv * inv;
        let same_sig = |c: &Class| -> bool {
            let rc = gr.col_indices_of_row_raw(c.rep);
            if rc.len() != cols.len() {
                return false;
            }
            let rv = gr.values_of_row(c.rep);
            rc.iter().zip(rv).zip(cols.iter().zip(vals)).all(|((&rj, &rvv), (&j, &v))| rj == j && (rvv * c.rep_inv).to_bits() == (v * inv).to_bits())
        };
        let mut found: Option<usize> = None;
        let head = if unit {
            Some(unit_head[cols[0]]).filter(|&h| h != usize::MAX)
        } else {
            heads.get(&hash).copied()
        };
        let mut cur = head.unwrap_or(usize::MAX);
        while cur != usize::MAX {
            if same_sig(&classes[cur]) {
                found = Some(cur);
                break;
            }
            cur = classes[cur].next;
        }
        match found {
            None => {
                let id = classes.len();
                classes.push(Class { rep: idx, rep_inv: inv, kept_idx: idx, kept_h: normalized_h, next: head.unwrap_or(usize::MAX) });
                if unit {
                    unit_head[cols[0]] = id;
                } else {
                    heads.insert(hash, id);
                }
            }
            Some(ci) => {
                any_dropped = true;
                let c = &mut classes[ci];
                if normalized_h < c.kept_h {
                    keep[c.kept_idx] = false;
                    c.kept_idx = idx;
                    c.kept_h = normalized_h;
                } else {
                    keep[idx] = false;
                }
            }
        }
    }

    // Nothing dropped and `g` already in canonical form (no stored zeros,
    // strictly increasing columns per row — what `csr_from_rows` would
    // produce anyway): the rebuilt matrix would be `g` itself.
    if !any_dropped {
        if csr_is_canonical(g) {
            return (g.clone(), h.to_vec());
        }
    }
    let kept_rows: Vec<usize> = (0..m).filter(|&i| keep[i]).collect();
    let nnz: usize = kept_rows.iter().map(|&i| gr.col_indices_of_row_raw(i).len()).sum();
    let mut builder = CsrRowBuilder::with_capacity(n, kept_rows.len(), nnz);
    let mut row_buf: Vec<(usize, f64)> = Vec::new();
    for &i in &kept_rows {
        row_buf.clear();
        row_buf.extend(csr_row_iter(g, i));
        if !builder.push_row(&row_buf) {
            let rows: Vec<Vec<(usize, f64)>> = kept_rows.iter().map(|&i| csr_row_vec(g, i)).collect();
            return (csr_from_rows(&rows, n), kept_rows.iter().map(|&i| h[i]).collect());
        }
    }
    (builder.finish(), kept_rows.iter().map(|&i| h[i]).collect())
}

/// Pass-through hasher for keys that already are well-mixed 64-bit hashes.
#[derive(Default)]
pub(crate) struct IdentityU64Hasher(u64);

impl std::hash::Hasher for IdentityU64Hasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 << 8) | b as u64;
        }
    }
    fn write_u64(&mut self, x: u64) {
        self.0 = x;
    }
}

#[cfg(test)]
fn reduce_inequalities_reference(g: &Csr, h: &[f64], n: usize) -> (Csr, Vec<f64>) {
    let m = g.nrows();
    if m == 0 {
        return (csr_from_rows(&[], n), Vec::new());
    }

    let rows: Vec<(Vec<(usize, f64)>, f64)> = (0..m)
        .map(|i| {
            let row: Vec<(usize, f64)> = csr_row_vec(g, i);
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
