//! Aggregator (HiGHS's own name for this technique, `HPresolve::aggregator`
//! in its `HPresolve.cpp`): generalizes [`crate::presolve::colsingleton`]'s
//! "exactly one row" restriction to "any number of rows", for columns whose
//! box bound is *implied* — already forced by the constraint system as a
//! whole, so the column can be eliminated via any one of its equality rows
//! without needing `colsingleton`'s own `extra_g_rows` bound-preservation
//! mechanism at all (the eliminated variable's true value, recovered via
//! `Substitution::value` after solving, is *mathematically guaranteed* to
//! land inside its own box bound — nothing needs to re-impose that as a
//! separate row).
//!
//! ## Why the first version of this module was reverted, and what changed
//!
//! A first version eliminated *every* non-free bounded column (any bound,
//! not just an implied one) appearing in >= 2 equality rows, unconditionally
//! emitting `colsingleton`-style bound-preservation rows. Measured on the
//! full 73-problem Netlib set: +162% aggregate time, and a follow-up that
//! added a *pivot-row-only* skip (a `row_implied_bound` helper, since
//! removed — its logic now lives in [`row_implies_own_bound`]) for those extra
//! rows still measured +175% (worse). Both were wrongly blamed (in an
//! earlier version of this doc comment, and of `presolve.rs`'s own wiring
//! comment) on fill-in from the elimination fold itself slowing down the
//! simplex loop — that was never profiled and was wrong. `ENOMOTO_PROF_PRESOLVE`
//! showed the real cost was this *pass's own* running time: the eligibility
//! test ("any non-free bounded column") let through thousands of columns
//! with no realistic elimination (`wood1p`: 2592 "eligible" columns, 0
//! actually eliminated, 3260ms spent finding that out; HiGHS's own
//! `isImpliedFree` gate lets through only 2 columns on that same model),
//! each costing a fresh full-matrix rescan.
//!
//! This version fixes the *performance* defect (the eligibility test is
//! now [`row_implies_own_bound`], gating on a real implied-bound check
//! rather than "any non-free bounded column" — see that function's own
//! docs, and [`RowActivity`]'s for how it stays cheap), and keeps the
//! *safety* condition row-local: [`eliminate_implied_free_columns`] only
//! eliminates a column via a specific row `R` when `R`'s *own* activity
//! (using every other column's current bound, nothing aggregated in from
//! any other row) already proves `x_j`'s box bound redundant. Re-validated
//! at *elimination* time against the pivot row's *current* content (not
//! just at candidate-generation time against the initial snapshot): a fold
//! performed earlier in this same call can rewrite a row that is also some
//! other column's pivot candidate, and the row-local check must hold for
//! whatever content that row actually has when used, not merely what it
//! had when candidates were first collected.
//!
//! ## Cross-row aggregation (the default since 2026-09-22)
//!
//! A first cross-row aggregate version (intersecting every row's own
//! implication for a column before checking against `[lb_j, ub_j]`,
//! catching a column no *single* row alone justifies but several together
//! do) was tried and reverted early on: it eliminated `stocfor2` correctly,
//! matching an ablation of HiGHS's own Aggregator to within a few percent,
//! but produced a false `Unbounded` on `shell` — a real correctness bug
//! this crate's benchmark objective-check caught before it shipped, root-
//! caused only much later (its exact mechanism was unknown at revert time,
//! and [`eliminate_implied_free_columns`] above shipped instead as the
//! version already known safe).
//!
//! [`eliminate_implied_free_columns_xrow`] is the fixed reattempt: a
//! justifying row for a column can be *deleted* — consumed as a *different*
//! column's own pivot earlier in the same call — and a justification
//! computed once at candidate-generation time can go stale exactly that
//! way (confirmed as `shell`'s own actual mechanism: two columns there
//! mutually justify each other through one shared row; eliminating one
//! consumes that row, then a stale check wrongly still treats it as
//! justifying the other). The fix is to always recompute a candidate's
//! justification from *live* rows with *current* content immediately
//! before that specific elimination is committed, never trusting the
//! snapshot — see that function's own (considerably longer) docs for the
//! full argument and the numeric trace.
//!
//! **Stays opt-in** (`ENOMOTO_XROW_AGGREGATOR` in `presolve.rs`), row-local
//! stays this module's default: a first 93-problem-Netlib measurement
//! (2026-09-22) was accidentally taken on a stale feature branch 44 commits
//! behind `main`, where `greenbea` was pathologically slow for unrelated
//! reasons (missing this crate's own `propagate_equalities` wiring, not
//! anything cross-row-specific) and dominated the aggregate enough to show
//! a spurious ~12% win. Re-measured on actual `main` (3 reps each way,
//! `greenbea` corrected): row-local and cross-row land within about a
//! percent of each other, overlapping ranges — a wash, not a reproducible
//! win, on this benchmark set. [`eliminate_implied_free_columns`] stays the
//! default; [`eliminate_implied_free_columns_xrow`] stays available (its
//! own correctness fix is real and independently regression-tested) and
//! still shares this module's helpers ([`compute_row_activity`],
//! [`residual_range`], [`implied_range`], [`fillin_cost`],
//! [`crate::sparse::axpy_row`]).
//!
//! ## Default since 2026-09-23: [`eliminate_implied_free_columns_v2`]
//!
//! The paragraph above is superseded for the default path: on `stocfor2`
//! the row-local gate left 704 of 1652 surviving columns in exactly the
//! shape "one equality row + one or more inequality rows", which neither
//! this gate (it needs >= 2 equality rows) nor `colsingleton` (it counts
//! the inequality rows too) can remove — HiGHS with only its Aggregator
//! switched off reproduces our old 1766x1652 presolved size almost exactly.
//! [`eliminate_implied_free_columns_v2`] keeps the cross-row version's
//! live re-validation (the `shell` fix), additionally uses one-sided
//! implied bounds from real inequality rows, admits single-equality-row
//! columns, and uses HiGHS's net fill-in with the size-2 exemption.
//! `ENOMOTO_ROWLOCAL_AGGREGATOR` / `ENOMOTO_XROW_AGGREGATOR` select the older
//! versions. Measurements: `analysis/stocfor2_presolve_20260923.md`.
//!
//! ## Candidate order and fill-in
//!
//! Mirrors `HPresolve::aggregator`'s own design (`HPresolve.cpp:6688`): all
//! `(row, col)` candidate pairs are collected once, sorted cheapest-first
//! (row-length-2-or-column-length-2 pairs before anything else, then by
//! `rowlen * collen` ascending — the same fill-in proxy HiGHS's own
//! `pdqsort` comparator uses), then processed in that order with a
//! `SUBSTITUTION_PIVOT_RATIO` numerical guard and a `MAX_FILLIN` cap (HiGHS's
//! own registered default, `presolve_substitution_maxfillin = 10`) — a
//! candidate whose fold would exceed it is left for a later call rather than
//! forced through. Three consecutive fill-in failures abort the rest of this
//! call's candidate list outright (mirrors HiGHS's own `nfail == 3` cutoff:
//! "indicates the rows/columns are becoming too dense for substitutions").
//!
//! ## One non-cascading call; repetition is the caller's job
//!
//! Like `colsingleton`'s own single pass, this computes implied bounds and
//! candidates once from an input snapshot; it does not loop internally to a
//! fixpoint. `presolve.rs`'s own round loop is what should call this
//! repeatedly (HiGHS itself calls its `aggregator` once per outer main-loop
//! iteration, right after its fast singleton/doubleton inner loop converges,
//! re-entering that inner loop whenever `aggregator` shrinks the problem —
//! `HPresolve.cpp:5901-5917`) so that a column exposed as implied-free only
//! *after* an earlier fold, or after `rowsingleton`/`colsingleton` tighten a
//! bound, gets caught on the next call rather than never.
//!
//! Only `A`'s equality rows are used both as elimination pivots *and* as
//! the implied-bound justification (an inequality row can't be solved for
//! one variable in terms of the others the same way, and `G`'s real rows
//! are never used to justify an elimination even indirectly — the same
//! staleness hazard the cross-row version's own fix above addresses for
//! `A`'s rows would need its own analogous argument to extend safely to
//! `G`, not yet made); `G`'s real rows still get folded like any other row
//! referencing an eliminated
//! column, exactly as `colsingleton`/`freevar` already do — they just never
//! contribute to deciding *whether* to eliminate. Like every other pass in
//! this crate, only appearances in `A`'s own rows (plus `G`'s real,
//! multi-variable rows, for fold purposes only) count — a variable's own
//! box-bound rows in `G` never count (same convention
//! `dualfix`/`colsingleton`/`freevar` use). Free variables (`lb == -inf &&
//! ub == inf`) are left to [`crate::presolve::freevar`], which needs no
//! implied-bound justification at all since a free variable's bound is
//! already vacuous.

use crate::presolve::colsingleton::Substitution;
use crate::sparse::{Csr, SparseAccum, axpy_row, csr_from_rows, csr_is_canonical, csr_rows};
const TOL: f64 = 1e-9;
/// Mirrors `colsingleton`/`freevar`'s own pivot guard exactly (same value,
/// same purpose — see either module's own docs on `SUBSTITUTION_PIVOT_RATIO`).
const SUBSTITUTION_PIVOT_RATIO: f64 = 1e-2;
/// Mirrors HiGHS's own `presolve_substitution_maxfillin` default (registered
/// range `[0, 10]`, default `10`, `HighsOptions.h`): total new nonzeros a
/// single column's elimination may introduce across every row it folds
/// into, above which the column is left for a later call instead of
/// risking a dense-equality-system blowup.
const MAX_FILLIN: usize = 10;
/// Mirrors HiGHS's own `nfail == 3` cutoff in `HPresolve::aggregator`: after
/// this many *consecutive* fill-in rejections, stop trying the rest of this
/// call's candidate list outright rather than keep paying for the fill-in
/// check on an already-too-dense region.
const MAX_CONSECUTIVE_FILLIN_FAILURES: usize = 3;

pub struct AggregatorResult {
    pub a: Csr,
    pub b: Vec<f64>,
    pub c: Vec<f64>,
    pub substitutions: Vec<Substitution>,
    /// `real_rows`/`real_rhs` with every fold this pass performed already
    /// applied — the caller must use these, not its own original copies,
    /// for anything downstream (mirrors `freevar::FreeVarResult`'s own
    /// fields of these names).
    pub real_rows: Vec<Vec<(usize, f64)>>,
    pub real_rhs: Vec<f64>,
}

/// One row's activity, summarized so that any single column's *residual*
/// range (the row's achievable range with that one column's own term
/// removed) can be recovered in O(1) via [`residual_range`]. An earlier
/// version of this module instead recomputed each column's residual from
/// scratch (an `O(row_len)` activity scan over the row's *other* terms,
/// once per nonzero), making the caller `O(row_len^2)` per row; on a real
/// Netlib instance (`wood1p`, a max row length of 2592 out of 2594
/// columns) that cost ~225ms per call for zero eliminations, this crate's
/// `ENOMOTO_PROF_PRESOLVE` showed — exactly the kind of self-inflicted cost
/// the module docs warn a naive candidate scan can hide. Rather than a
/// per-column subtraction of a possibly-infinite running sum (which risks
/// the `inf - inf` case outright), this tracks how many terms are
/// unbounded on each side and, when there is exactly one, which —
/// mirroring HiGHS's own `getNumInfSumUpperOrig`/`getResidualSumLowerOrig`
/// pattern in `HPresolve.cpp` for the same reason.
#[derive(Clone, Copy)]
struct RowActivity {
    lo_finite_sum: f64,
    lo_inf_count: usize,
    hi_finite_sum: f64,
    hi_inf_count: usize,
}

fn compute_row_activity(row: &[(usize, f64)], lb: &[f64], ub: &[f64]) -> RowActivity {
    let mut lo_finite_sum = 0.0f64;
    let mut lo_inf_count = 0usize;
    let mut hi_finite_sum = 0.0f64;
    let mut hi_inf_count = 0usize;
    for &(k, v) in row {
        let (klo, khi) = if v > 0.0 { (lb[k], ub[k]) } else { (ub[k], lb[k]) };
        if klo.is_finite() {
            lo_finite_sum += v * klo;
        } else {
            lo_inf_count += 1;
        }
        if khi.is_finite() {
            hi_finite_sum += v * khi;
        } else {
            hi_inf_count += 1;
        }
    }
    RowActivity { lo_finite_sum, lo_inf_count, hi_finite_sum, hi_inf_count }
}

/// The row's own achievable `[s_lo, s_hi]` range with column `j`'s term
/// (coefficient `j_coeff`) excluded — O(1) given `activity`, `compute_row_activity`'s
/// one-pass-per-row summary of every term including `j`'s own.
fn residual_range(activity: &RowActivity, j: usize, j_coeff: f64, lb: &[f64], ub: &[f64]) -> (f64, f64) {
    let (jlo, jhi) = if j_coeff > 0.0 { (lb[j], ub[j]) } else { (ub[j], lb[j]) };
    let s_lo = if jlo.is_finite() {
        if activity.lo_inf_count == 0 {
            activity.lo_finite_sum - j_coeff * jlo
        } else {
            f64::NEG_INFINITY
        }
    } else if activity.lo_inf_count == 1 {
        // `j` itself was the row's only lo-unbounded term -- removing it
        // leaves the (already-finite) rest.
        activity.lo_finite_sum
    } else {
        f64::NEG_INFINITY
    };
    let s_hi = if jhi.is_finite() {
        if activity.hi_inf_count == 0 {
            activity.hi_finite_sum - j_coeff * jhi
        } else {
            f64::INFINITY
        }
    } else if activity.hi_inf_count == 1 {
        activity.hi_finite_sum
    } else {
        f64::INFINITY
    };
    (s_lo, s_hi)
}

/// Row `sum + coeff*x_j = rhs`'s own implied range for `x_j` alone (its
/// achievable range with every *other* column at its current bound) —
/// factored out of [`row_implies_own_bound`] so [`eliminate_implied_free_columns_xrow`]
/// can intersect this same per-row computation across several rows instead
/// of checking just one (see that function's own docs for why the
/// intersection needs this, not the boolean `row_implies_own_bound` itself).
fn implied_range(activity: &RowActivity, j: usize, coeff: f64, rhs: f64, lb: &[f64], ub: &[f64]) -> (f64, f64) {
    let (s_lo, s_hi) = residual_range(activity, j, coeff, lb, ub);
    let v1 = (rhs - s_hi) / coeff;
    let v2 = (rhs - s_lo) / coeff;
    (v1.min(v2), v1.max(v2))
}

fn range_within_box(j: usize, lo: f64, hi: f64, lb: &[f64], ub: &[f64]) -> bool {
    (lb[j] == f64::NEG_INFINITY || lo >= lb[j] - TOL) && (ub[j] == f64::INFINITY || hi <= ub[j] + TOL)
}

/// Whether equality row `sum + coeff*x_j = rhs`'s *own* activity (using
/// every other column's current bound, via `activity` —
/// `compute_row_activity`'s summary of this same row, nothing aggregated in
/// from any other row `j` might also appear in) already proves `x_j`'s box
/// bound `[lb[j], ub[j]]` redundant. See the module docs for why this stays
/// row-local rather than aggregating across every row `j` appears in (a
/// cross-row aggregate version was tried and reverted after producing a
/// false `Unbounded` on a real Netlib instance — see
/// [`eliminate_implied_free_columns_xrow`] for the root-caused, fixed
/// reattempt).
fn row_implies_own_bound(activity: &RowActivity, j: usize, coeff: f64, rhs: f64, lb: &[f64], ub: &[f64]) -> bool {
    let (lo, hi) = implied_range(activity, j, coeff, rhs, lb, ub);
    range_within_box(j, lo, hi, lb, ub)
}

/// Total nonzeros `pivot_terms` would newly introduce (not already present)
/// across every row in `targets` — the fill-in a fold into all of them at
/// once would cost.
fn fillin_cost(pivot_terms: &[(usize, f64)], targets: &[&Vec<(usize, f64)>]) -> usize {
    let mut cost = 0usize;
    for target in targets {
        let existing: std::collections::BTreeSet<usize> = target.iter().map(|&(k, _)| k).collect();
        cost += pivot_terms.iter().filter(|&&(k, _)| !existing.contains(&k)).count();
    }
    cost
}

/// Eliminates every *implied-free* structural column — one whose box bound
/// is already forced by the constraint system as a whole (see the module
/// docs) — reachable through two or more of `A`'s own equality rows (a
/// single-row appearance is `colsingleton`'s own, strictly cheaper case,
/// with no implied-bound computation needed since removing the column's
/// *only* row would otherwise lose its bound outright). One non-cascading
/// call from a snapshot; see the module docs for why repetition is the
/// caller's own job, not this function's.
#[cfg_attr(not(test), allow(dead_code))]
pub fn eliminate_implied_free_columns(n: usize, a: &Csr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64]) -> AggregatorResult {
    eliminate_implied_free_columns_if_any(n, a, b, c, lb, ub, real_rows, real_rhs).unwrap_or_else(|| AggregatorResult {
        a: if csr_is_canonical(a) { a.clone() } else { csr_from_rows(&csr_rows(a), n) },
        b: b.to_vec(),
        c: c.to_vec(),
        substitutions: Vec::new(),
        real_rows: real_rows.to_vec(),
        real_rhs: real_rhs.to_vec(),
    })
}

/// [`eliminate_implied_free_columns`], returning `None` instead of an
/// unchanged copy of the whole problem when nothing is eliminated — the
/// common case on most rounds of most problems. The candidate search runs
/// straight off `a`'s CSR rows, and only when it finds at least one
/// candidate is the problem copied into the mutable row-list form the
/// elimination loop works in, so an empty call costs one activity pass
/// over `A` and no allocation proportional to `A`/`real_rows`. The result,
/// when `Some`, is bit-identical to what the unconditional-copy version
/// produced (the candidate list is computed from exactly the same row
/// contents, in the same order).
pub fn eliminate_implied_free_columns_if_any(n: usize, a: &Csr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64]) -> Option<AggregatorResult> {
    let ar = a.as_ref();
    let p = ar.nrows();

    let mut col_a_count = vec![0usize; n];
    for i in 0..p {
        for (j, &v) in ar.col_indices_of_row(i).zip(ar.values_of_row(i)) {
            if v != 0.0 {
                col_a_count[j] += 1;
            }
        }
    }

    // All (row, col) candidate pairs where `row`'s *own* activity already
    // proves col's box bound redundant (`row_implies_own_bound`) and col
    // appears in >= 2 equality rows, sorted cheapest-first: size-2 pairs
    // (row length or column length exactly 2 -- fill-in can never be
    // problematic there) before anything else, then by `rowlen * collen`
    // ascending (a fill-in proxy) -- mirrors HiGHS's own `pdqsort`
    // comparator in `HPresolve::aggregator` exactly (`HPresolve.cpp:6703`).
    // One `compute_row_activity` call per row (not per nonzero) keeps this
    // `O(nnz)` overall rather than `O(row_len)` per nonzero -- see
    // `RowActivity`'s own docs on why that distinction matters.
    // Truly free columns (`lb == -inf && ub == inf`) trivially satisfy
    // `row_implies_own_bound` from *any* row (both sides of its check
    // short-circuit true) -- correct on its own terms, but left to
    // `freevar`'s own dedicated pivot selection instead, matching the
    // module docs' division of labor.
    let is_free = |j: usize| lb[j] == f64::NEG_INFINITY && ub[j] == f64::INFINITY;
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    let mut row: Vec<(usize, f64)> = Vec::new();
    for i in 0..p {
        row.clear();
        row.extend(ar.col_indices_of_row(i).zip(ar.values_of_row(i)).map(|(j, &v)| (j, v)));
        let activity = compute_row_activity(&row, lb, ub);
        for &(j, v) in &row {
            if v != 0.0 && col_a_count[j] >= 2 && !is_free(j) && row_implies_own_bound(&activity, j, v, b[i], lb, ub) {
                candidates.push((i, j));
            }
        }
    }
    if candidates.is_empty() {
        return None;
    }
    candidates.sort_by_key(|&(i, j)| {
        let rowlen = ar.col_indices_of_row(i).len();
        let collen = col_a_count[j];
        let min_len = rowlen.min(collen);
        (min_len != 2, rowlen * collen, min_len, i, j)
    });

    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);
    // One sparse accumulator for every row fold this pass performs —
    // see `crate::sparse::SparseAccum`'s own docs for why the merge is
    // not a per-row `BTreeMap`.
    let mut accum = SparseAccum::new(n);
    let mut b: Vec<f64> = b.to_vec();
    let mut c: Vec<f64> = c.to_vec();
    let mut real_rows: Vec<Vec<(usize, f64)>> = real_rows.to_vec();
    let mut real_rhs: Vec<f64> = real_rhs.to_vec();

    // Column -> row indices for `a_rows` and `real_rows`, so finding the
    // rows that contain the eliminated column costs O(column length)
    // instead of a scan over every row of both matrices per candidate.
    // Lists are a *superset* (entries can cancel to zero or rows get
    // deleted); each query re-verifies membership exactly as the plain
    // scan did and sorts/dedups, so the resulting row lists — and every
    // decision downstream — are identical to the full-scan version.
    let mut a_col_idx: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, row) in a_rows.iter().enumerate() {
        for &(k, v) in row {
            if v != 0.0 {
                a_col_idx[k].push(i);
            }
        }
    }
    let mut g_col_idx: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, row) in real_rows.iter().enumerate() {
        for &(k, v) in row {
            if v != 0.0 {
                g_col_idx[k].push(i);
            }
        }
    }

    let mut row_deleted = vec![false; a_rows.len()];
    let mut col_eliminated = vec![false; n];
    let mut substitutions = Vec::new();
    let mut consecutive_fillin_failures = 0usize;

    for (row_idx, j) in candidates {
        if row_deleted[row_idx] || col_eliminated[j] {
            continue;
        }
        let pivot_row = a_rows[row_idx].clone();
        let coeff = match pivot_row.iter().find(|&&(k, _)| k == j) {
            Some(&(_, v)) if v != 0.0 => v,
            _ => continue,
        };
        // Re-validate against the row's *current* content: an earlier
        // elimination this same call can have folded into this row (if it
        // was some other column's "other_a"), changing what it implies
        // about `j` since candidate generation ran on the initial snapshot
        // — see the module docs on why this re-check, not just the
        // generation-time one, is what soundness actually depends on.
        let activity = compute_row_activity(&pivot_row, lb, ub);
        if !row_implies_own_bound(&activity, j, coeff, b[row_idx], lb, ub) {
            continue;
        }
        let row_max = pivot_row.iter().map(|&(_, v)| v.abs()).fold(0.0f64, f64::max);
        if coeff.abs() < tunable!("ENOMOTO_T_SUBSTITUTION_PIVOT_RATIO", SUBSTITUTION_PIVOT_RATIO, f64) * row_max {
            // This specific (row, col) pair fails the numerical guard --
            // unlike a plain eligibility failure, a *different* row for the
            // same column may still be a later candidate in this same list
            // (mirrors HiGHS's own per-pair, not per-column, processing).
            continue;
        }
        let terms: Vec<(usize, f64)> = pivot_row.iter().filter(|&&(k, _)| k != j).copied().collect();
        if terms.is_empty() {
            // A genuine row singleton slipped through (rowsingleton should
            // already have caught this earlier in the same round).
            continue;
        }

        // Every *other* row (in `A` or `real_rows`) still referencing `j`,
        // scanned fresh from the current state -- cheap now that
        // `row_implies_own_bound` has already cut the candidate set down to
        // what HiGHS's own gate would (the reverted first version's
        // regression was exactly this scan running on thousands of
        // never-eliminable candidates; see the module docs).
        let other_a: Vec<usize> = {
            let mut v = a_col_idx[j].clone();
            v.sort_unstable();
            v.dedup();
            v.retain(|&i2| i2 != row_idx && !row_deleted[i2] && a_rows[i2].iter().any(|&(k, v)| k == j && v != 0.0));
            v
        };
        let other_g: Vec<usize> = {
            let mut v = g_col_idx[j].clone();
            v.sort_unstable();
            v.dedup();
            v.retain(|&i2| real_rows[i2].iter().any(|&(k, v)| k == j && v != 0.0));
            v
        };

        let fillin = {
            let mut targets: Vec<&Vec<(usize, f64)>> = Vec::with_capacity(other_a.len() + other_g.len());
            for &i2 in &other_a {
                targets.push(&a_rows[i2]);
            }
            for &i2 in &other_g {
                targets.push(&real_rows[i2]);
            }
            fillin_cost(&terms, &targets)
        };
        if fillin > tunable!("ENOMOTO_T_MAX_FILLIN", MAX_FILLIN, usize) {
            consecutive_fillin_failures += 1;
            if consecutive_fillin_failures >= tunable!("ENOMOTO_T_MAX_CONSECUTIVE_FILLIN_FAILURES", MAX_CONSECUTIVE_FILLIN_FAILURES, usize) {
                break;
            }
            continue;
        }
        consecutive_fillin_failures = 0;

        let rhs_i = b[row_idx];
        for &i2 in &other_a {
            let a_i2j = a_rows[i2].iter().find(|&&(k, _)| k == j).unwrap().1;
            let factor = a_i2j / coeff;
            a_rows[i2] = axpy_row(&mut accum, &a_rows[i2], &pivot_row, factor, j, TOL);
            // Fill-in can only land in the pivot row's other columns.
            for &(k, _) in &terms {
                a_col_idx[k].push(i2);
            }
            b[i2] -= factor * rhs_i;
        }
        for &i2 in &other_g {
            let a_i2j = real_rows[i2].iter().find(|&&(k, _)| k == j).unwrap().1;
            let factor = a_i2j / coeff;
            real_rows[i2] = axpy_row(&mut accum, &real_rows[i2], &pivot_row, factor, j, TOL);
            for &(k, _) in &terms {
                g_col_idx[k].push(i2);
            }
            real_rhs[i2] -= factor * rhs_i;
        }

        let cj = c[j];
        if cj != 0.0 {
            let factor = cj / coeff;
            for &(k, a_ik) in &terms {
                c[k] -= factor * a_ik;
            }
            c[j] = 0.0;
        }

        // No bound-preservation row: `j` is implied-free, so its true
        // (postsolve-recovered) value is guaranteed inside `[lb[j], ub[j]]`
        // without one -- see the module docs.
        substitutions.push(Substitution { var: j, terms, rhs: rhs_i, coeff });
        col_eliminated[j] = true;
        row_deleted[row_idx] = true;
    }
    // Nothing accepted: no row was touched (every fold above happens only
    // on acceptance), so the problem is unchanged.
    if substitutions.is_empty() {
        return None;
    }

    let mut final_a_rows = Vec::with_capacity(a_rows.len());
    let mut final_b = Vec::with_capacity(b.len());
    for (i, row) in a_rows.into_iter().enumerate() {
        if !row_deleted[i] {
            final_a_rows.push(row);
            final_b.push(b[i]);
        }
    }

    Some(AggregatorResult {
        a: csr_from_rows(&final_a_rows, n),
        b: final_b,
        c,
        substitutions,
        real_rows,
        real_rhs,
    })
}

/// Cross-row generalization of [`eliminate_implied_free_columns`]: a
/// column's implied range is the *intersection* of every one of its own
/// live equality rows' local implied range ([`implied_range`]), not just
/// one — catching a column no *single* row alone proves implied-free but
/// several together do (this module's own docs' `stocfor2` motivation: an
/// earlier attempt at exactly this matched an ablation of HiGHS's own
/// Aggregator there to within a few percent). Not wired into
/// [`crate::presolve::run_extended`]'s default pipeline — reachable only
/// via `presolve.rs`'s own `ENOMOTO_XROW_AGGREGATOR` opt-in gate, pending a
/// full-Netlib reach/cost measurement — because reproducing this
/// generalization faithfully surfaced a real, previously un-root-caused
/// correctness bug (see below) rather than the "aggregate the intersection
/// once and go" shape the module docs' own history describes; this
/// function is the fixed reattempt, not a resurrection of the original.
///
/// **The correctness-critical difference from the row-local version, and
/// from every earlier cross-row attempt**: a candidate's justification is
/// *recomputed from live rows only, with their current content*,
/// immediately before that specific elimination is committed — never
/// trusted from the snapshot candidate-generation pass below, and never
/// merely re-validated against the one row chosen as pivot (contrast the
/// row-local version's own single `pivot_row` re-check at its own call
/// site, sound there only because the row-local version's pivot row *is*
/// its sole justification, so `row_deleted[row_idx]` alone is enough to
/// catch a stale candidate). A justifying row for column `j` can itself be
/// *deleted* — consumed as the pivot row of an *earlier* elimination
/// within this same call — while a snapshot-only computation still "sees"
/// it as live and unchanged. Confirmed as the actual mechanism behind the
/// historical false `Unbounded` on Netlib `shell` (root-caused directly,
/// not inferred): columns 32 and 52 there mutually justify each other
/// through one shared length-2 row (`0.236*x32 - 0.983*x52 = 0`);
/// eliminating column 32 first consumes that row as its own pivot, and a
/// snapshot-only justification for column 52 then wrongly treats its own
/// bound as still implied by a row that is already gone, dropping *both*
/// variables' boxes with nothing left to enforce either — `x52`'s own
/// nonzero objective coefficient then drives it to `-inf` under plain
/// simplex, an entirely soundness bug in this presolve reduction itself,
/// not anything downstream. Re-scanning every currently-live row for `j`
/// fresh (rather than trusting the snapshot's `col_rows[j]` list, which
/// this function still uses for cheap candidate *generation* and ordering
/// only) is what closes this gap: a stale or deleted justifying row simply
/// no longer contributes to the recomputed intersection, so a candidate
/// whose justification depended on it is correctly rejected instead of
/// silently eliminated.
///
/// A closely related question — can a similarly-shaped bug hide even when
/// only a *single* row justifies a column, if that row (not just a
/// multi-row intersection) is the one that gets consumed by an earlier
/// elimination? — turns out to already be answered by the row-local
/// version's own design: there, the pivot row *is* the sole justifying
/// row, so `row_deleted[row_idx]` (checked before any re-validation even
/// runs) already catches exactly that case. The bug specific to a
/// cross-row version is a justifying row surviving deletion of some *other*
/// row that happened to be a *different* column's pivot while still being
/// treated, by a stale computation, as if it still backed `j`'s own
/// elimination — a distinction that only exists once more than one row can
/// jointly justify a single column.
pub fn eliminate_implied_free_columns_xrow(n: usize, a: &Csr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64]) -> AggregatorResult {
    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);
    // One sparse accumulator for every row fold this pass performs —
    // see `crate::sparse::SparseAccum`'s own docs for why the merge is
    // not a per-row `BTreeMap`.
    let mut accum = SparseAccum::new(n);
    let mut b: Vec<f64> = b.to_vec();
    let mut c: Vec<f64> = c.to_vec();
    let mut real_rows: Vec<Vec<(usize, f64)>> = real_rows.to_vec();
    let mut real_rhs: Vec<f64> = real_rhs.to_vec();

    let mut col_a_count = vec![0usize; n];
    for row in &a_rows {
        for &(j, v) in row {
            if v != 0.0 {
                col_a_count[j] += 1;
            }
        }
    }
    let is_free = |j: usize| lb[j] == f64::NEG_INFINITY && ub[j] == f64::INFINITY;

    // One `RowActivity` per row, computed lazily and reused across every
    // column that shares it — `None` means "stale or never computed",
    // forcing a fresh `compute_row_activity` next time it's read.
    // Essential, not just an optimization: without this, a row with many
    // nonzeros gets its `O(row_len)` activity recomputed once per column
    // that references it, both here and again at elimination time below —
    // `O(row_len)` per column times up to `row_len` columns sharing one row
    // is exactly the `O(row_len^2)` blowup `RowActivity`'s own docs
    // describe an *earlier* version of the row-local pass paying on
    // Netlib `wood1p` (a single row with 2592 of 2594 columns nonzero) —
    // confirmed to reproduce here too (0.13s -> 0.46s on the full Netlib
    // set) before this cache was added. A row's cached entry is
    // invalidated (`None`) the moment a fold changes its content (see the
    // elimination loop below), so a cache hit always reflects that row's
    // *current* state, never a stale one — the fold sites are the only
    // places `a_rows[i]` changes after this point.
    let mut row_activity: Vec<Option<RowActivity>> = vec![None; a_rows.len()];

    // Intersects every row in `rows` that still carries a nonzero `j` term
    // (in `a_rows`'s *current* content — always read fresh here, only the
    // per-row `RowActivity` summary is cached) into one combined implied
    // range; `None` if no such row remains.
    fn aggregate_range(j: usize, rows: &[usize], a_rows: &[Vec<(usize, f64)>], b: &[f64], lb: &[f64], ub: &[f64], row_activity: &mut [Option<RowActivity>]) -> Option<(f64, f64)> {
        let mut lo = f64::NEG_INFINITY;
        let mut hi = f64::INFINITY;
        let mut any = false;
        for &i in rows {
            let row = &a_rows[i];
            let Some(&(_, coeff)) = row.iter().find(|&&(k, _)| k == j) else { continue };
            if coeff == 0.0 {
                continue;
            }
            let activity = *row_activity[i].get_or_insert_with(|| compute_row_activity(row, lb, ub));
            let (rlo, rhi) = implied_range(&activity, j, coeff, b[i], lb, ub);
            lo = lo.max(rlo);
            hi = hi.min(rhi);
            any = true;
        }
        any.then_some((lo, hi))
    }

    // Snapshot candidate generation: column -> its own equality rows at
    // call-input time, used only to decide *which columns are worth
    // trying* and in what order — the elimination loop below never trusts
    // this list's own membership or the rows' snapshot content, only uses
    // it (still live rows filtered back in) as a candidate-ordering hint;
    // see this function's own docs for why the actual justification is
    // always recomputed fresh from live rows at elimination time instead.
    let mut col_rows: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, row) in a_rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 {
                col_rows[j].push(i);
            }
        }
    }

    let mut candidates: Vec<usize> = Vec::new();
    for j in 0..n {
        if col_a_count[j] < 2 || is_free(j) {
            continue;
        }
        if let Some((lo, hi)) = aggregate_range(j, &col_rows[j], &a_rows, &b, lb, ub, &mut row_activity) {
            if range_within_box(j, lo, hi, lb, ub) {
                candidates.push(j);
            }
        }
    }
    // Cheapest-first, mirroring the row-local version's own fill-in proxy
    // (`rowlen * collen`, size-2 pairs first): estimated via this column's
    // own least-cost row, the one an actual elimination would most likely
    // pivot through.
    candidates.sort_by_key(|&j| {
        let collen = col_a_count[j];
        let min_rowlen = col_rows[j].iter().map(|&i| a_rows[i].len()).min().unwrap_or(0);
        (min_rowlen.min(collen) != 2, min_rowlen * collen, min_rowlen.min(collen), j)
    });

    let mut row_deleted = vec![false; a_rows.len()];
    let mut col_eliminated = vec![false; n];
    let mut substitutions = Vec::new();
    let mut consecutive_fillin_failures = 0usize;

    for j in candidates {
        if col_eliminated[j] {
            continue;
        }
        // The fix: recompute from *every currently-live* row referencing
        // `j` (not just `col_rows[j]`'s snapshot list — a different
        // column's own fold can have introduced a fresh `j` term into a
        // row that had none at snapshot time; omitting such a row here
        // only widens the intersection, i.e. makes this check *more*
        // conservative, never unsound) and with *current* row content, not
        // the snapshot's. See this function's own docs for why this,
        // rather than a pivot-row-only re-check, is what soundness
        // actually depends on here.
        let live_rows: Vec<usize> = a_rows
            .iter()
            .enumerate()
            .filter(|&(i, row)| !row_deleted[i] && row.iter().any(|&(k, v)| k == j && v != 0.0))
            .map(|(i, _)| i)
            .collect();
        let Some((lo, hi)) = aggregate_range(j, &live_rows, &a_rows, &b, lb, ub, &mut row_activity) else {
            continue;
        };
        if !range_within_box(j, lo, hi, lb, ub) {
            continue;
        }

        // Pivot through whichever still-live justifying row is cheapest
        // (least fill-in) and clears the numerical pivot-ratio guard —
        // any of them is equally valid algebraically now that the
        // intersection above has already confirmed the *combined*
        // justification holds.
        let mut sorted_live = live_rows.clone();
        sorted_live.sort_by_key(|&i| a_rows[i].len());
        let mut chosen: Option<(usize, f64)> = None;
        for &i in &sorted_live {
            let row = &a_rows[i];
            let Some(&(_, coeff)) = row.iter().find(|&&(k, _)| k == j) else { continue };
            let row_max = row.iter().map(|&(_, v)| v.abs()).fold(0.0f64, f64::max);
            if coeff.abs() >= tunable!("ENOMOTO_T_SUBSTITUTION_PIVOT_RATIO", SUBSTITUTION_PIVOT_RATIO, f64) * row_max {
                chosen = Some((i, coeff));
                break;
            }
        }
        let Some((row_idx, coeff)) = chosen else {
            continue;
        };

        let pivot_row = a_rows[row_idx].clone();
        let terms: Vec<(usize, f64)> = pivot_row.iter().filter(|&&(k, _)| k != j).copied().collect();
        if terms.is_empty() {
            continue;
        }

        let other_a: Vec<usize> = a_rows
            .iter()
            .enumerate()
            .filter(|&(i2, row2)| i2 != row_idx && !row_deleted[i2] && row2.iter().any(|&(k, v)| k == j && v != 0.0))
            .map(|(i2, _)| i2)
            .collect();
        let other_g: Vec<usize> = real_rows.iter().enumerate().filter(|(_, row2)| row2.iter().any(|&(k, v)| k == j && v != 0.0)).map(|(i2, _)| i2).collect();

        let fillin = {
            let mut targets: Vec<&Vec<(usize, f64)>> = Vec::with_capacity(other_a.len() + other_g.len());
            for &i2 in &other_a {
                targets.push(&a_rows[i2]);
            }
            for &i2 in &other_g {
                targets.push(&real_rows[i2]);
            }
            fillin_cost(&terms, &targets)
        };
        if fillin > tunable!("ENOMOTO_T_MAX_FILLIN", MAX_FILLIN, usize) {
            consecutive_fillin_failures += 1;
            if consecutive_fillin_failures >= tunable!("ENOMOTO_T_MAX_CONSECUTIVE_FILLIN_FAILURES", MAX_CONSECUTIVE_FILLIN_FAILURES, usize) {
                break;
            }
            continue;
        }
        consecutive_fillin_failures = 0;

        let rhs_i = b[row_idx];
        for &i2 in &other_a {
            let a_i2j = a_rows[i2].iter().find(|&&(k, _)| k == j).unwrap().1;
            let factor = a_i2j / coeff;
            a_rows[i2] = axpy_row(&mut accum, &a_rows[i2], &pivot_row, factor, j, TOL);
            b[i2] -= factor * rhs_i;
            // This row's content just changed — its cached `RowActivity`
            // (if any) is now stale; see `row_activity`'s own docs.
            row_activity[i2] = None;
        }
        for &i2 in &other_g {
            let a_i2j = real_rows[i2].iter().find(|&&(k, _)| k == j).unwrap().1;
            let factor = a_i2j / coeff;
            real_rows[i2] = axpy_row(&mut accum, &real_rows[i2], &pivot_row, factor, j, TOL);
            real_rhs[i2] -= factor * rhs_i;
        }

        let cj = c[j];
        if cj != 0.0 {
            let factor = cj / coeff;
            for &(k, a_ik) in &terms {
                c[k] -= factor * a_ik;
            }
            c[j] = 0.0;
        }

        substitutions.push(Substitution { var: j, terms, rhs: rhs_i, coeff });
        col_eliminated[j] = true;
        row_deleted[row_idx] = true;
    }

    let mut final_a_rows = Vec::with_capacity(a_rows.len());
    let mut final_b = Vec::with_capacity(b.len());
    for (i, row) in a_rows.into_iter().enumerate() {
        if !row_deleted[i] {
            final_a_rows.push(row);
            final_b.push(b[i]);
        }
    }

    AggregatorResult {
        a: csr_from_rows(&final_a_rows, n),
        b: final_b,
        c,
        substitutions,
        real_rows,
        real_rhs,
    }
}

/// Which of [`eliminate_implied_free_columns_v2`]'s generalisations over
/// [`eliminate_implied_free_columns_xrow`] are switched on — each one is an
/// independent countermeasure (see `analysis/stocfor2_presolve_20260923.md`)
/// and is kept separately switchable (the `ENOMOTO_AGG_*` opt-outs in
/// [`AggOptions::from_env`]) so each can be A/B-measured on its own.
#[derive(Clone, Copy, Debug)]
pub struct AggOptions {
    /// Also intersect one-sided implied bounds from `G`'s real inequality
    /// rows into a column's implied range (HiGHS `isImpliedFree` uses the
    /// tightest implied bound from *any* row, not just equalities).
    pub use_ineq: bool,
    /// Minimum number of equality rows a candidate column must appear in:
    /// 2 is the old gate; 1 additionally admits a column in exactly one
    /// equality row plus >= 1 inequality row (which `colsingleton` rejects
    /// too, so nothing else in the pipeline can remove it).
    pub min_a_count: usize,
    /// HiGHS's net fill-in (`new nonzeros - (rowlen + collen - 1)`), with the
    /// check skipped outright when the pivot row or the column has length 2.
    pub net_fillin: bool,
    /// Abort the rest of the candidate list after
    /// `MAX_CONSECUTIVE_FILLIN_FAILURES` consecutive fill-in rejections.
    pub fillin_break: bool,
}

impl AggOptions {
    pub fn from_env() -> Self {
        AggOptions {
            use_ineq: env_str!("ENOMOTO_AGG_NOINEQ").is_none(),
            min_a_count: if env_str!("ENOMOTO_AGG_MINACNT2").is_some() { 2 } else { 1 },
            net_fillin: env_str!("ENOMOTO_AGG_GROSSFILL").is_none(),
            fillin_break: env_str!("ENOMOTO_AGG_NOBREAK").is_none(),
        }
    }
}

/// Generalised cross-row aggregator: [`eliminate_implied_free_columns_xrow`]'s
/// live-recomputed cross-row justification (the `shell` fix — every
/// candidate's implied range is recomputed from *live* rows with *current*
/// content immediately before its elimination is committed), extended by
/// [`AggOptions`].
///
/// Soundness of the inequality-row justification: a `G` row is never deleted
/// here, only folded, so the row that implied `x_j`'s bound keeps being
/// enforced after `x_j` is substituted out, and every other column it used
/// keeps its box (a column eliminated earlier no longer appears in any live
/// row, so justifications only ever depend on boxes that are still
/// enforced — the eliminations form a DAG, never a cycle).
/// [`eliminate_implied_free_columns_v2`], returning `None` (the problem is
/// unchanged) without copying anything when its candidate list would be
/// empty — the v2 counterpart of [`eliminate_implied_free_columns_if_any`]
/// (most rounds of most problems find no candidate at all). The pre-check
/// [`v2_has_candidate`] evaluates exactly v2's own candidate test, in the
/// same floating-point order, straight off `a`'s CSR slices and the
/// borrowed `real_rows`, so `Some` results are bit-identical to calling
/// v2 directly and `None` is returned only where v2 would have eliminated
/// nothing.
pub fn eliminate_implied_free_columns_v2_if_any(n: usize, a: &Csr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], opts: AggOptions) -> Option<AggregatorResult> {
    if !v2_has_candidate(n, a, b, lb, ub, real_rows, real_rhs, opts) {
        return None;
    }
    Some(eliminate_implied_free_columns_v2(n, a, b, c, lb, ub, real_rows, real_rhs, opts))
}

/// Whether [`eliminate_implied_free_columns_v2`]'s candidate list is
/// non-empty. Mirrors its candidate generation exactly: the same
/// eligibility test, each row's activity summed over the row sorted by
/// column (v2 sorts unsorted rows before anything else), and, per column,
/// the same sequence of `max`/`min` folds (equality rows in ascending
/// order, then inequality rows in ascending order).
pub fn v2_has_candidate(n: usize, a: &Csr, b: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], opts: AggOptions) -> bool {
    let ar = a.as_ref();
    let p = ar.nrows();
    let is_free = |j: usize| lb[j] == f64::NEG_INFINITY && ub[j] == f64::INFINITY;
    let mut col_a_count = vec![0usize; n];
    let mut col_g_count = vec![0usize; n];
    for i in 0..p {
        for (&j, &v) in ar.col_indices_of_row_raw(i).iter().zip(ar.values_of_row(i)) {
            if v != 0.0 {
                col_a_count[j] += 1;
            }
        }
    }
    for row in real_rows {
        for &(j, v) in row {
            if v != 0.0 {
                col_g_count[j] += 1;
            }
        }
    }
    let min_a = opts.min_a_count.max(1);
    let eligible: Vec<bool> = (0..n).map(|j| !(col_a_count[j] < min_a || col_a_count[j] + col_g_count[j] < 2 || is_free(j))).collect();
    if !eligible.iter().any(|&e| e) {
        return false;
    }
    let mut lo = vec![f64::NEG_INFINITY; n];
    let mut hi = vec![f64::INFINITY; n];
    let mut row_buf: Vec<(usize, f64)> = Vec::new();
    // `row` itself when already sorted by column, else a sorted copy in
    // `row_buf` (v2 sorts before computing any activity).
    fn sorted<'r>(row: &'r [(usize, f64)], buf: &'r mut Vec<(usize, f64)>) -> &'r [(usize, f64)] {
        if row.windows(2).all(|w| w[0].0 < w[1].0) {
            row
        } else {
            buf.clear();
            buf.extend_from_slice(row);
            buf.sort_unstable_by_key(|&(k, _)| k);
            buf
        }
    }
    let mut a_buf: Vec<(usize, f64)> = Vec::new();
    for i in 0..p {
        let cols = ar.col_indices_of_row_raw(i);
        let vals = ar.values_of_row(i);
        if !cols.iter().zip(vals).any(|(&j, &v)| v != 0.0 && eligible[j]) {
            continue;
        }
        a_buf.clear();
        a_buf.extend(cols.iter().copied().zip(vals.iter().copied()));
        let row = sorted(&a_buf, &mut row_buf);
        let act = compute_row_activity(row, lb, ub);
        for &(j, coeff) in row {
            if coeff == 0.0 || !eligible[j] {
                continue;
            }
            let (rlo, rhi) = implied_range(&act, j, coeff, b[i], lb, ub);
            lo[j] = lo[j].max(rlo);
            hi[j] = hi[j].min(rhi);
        }
    }
    if opts.use_ineq {
        for (i, row) in real_rows.iter().enumerate() {
            if !row.iter().any(|&(j, v)| v != 0.0 && eligible[j]) {
                continue;
            }
            let row = sorted(row, &mut row_buf);
            let act = compute_row_activity(row, lb, ub);
            for &(j, coeff) in row {
                if coeff == 0.0 || !eligible[j] {
                    continue;
                }
                let (s_lo, _) = residual_range(&act, j, coeff, lb, ub);
                if s_lo.is_finite() {
                    let v = (real_rhs[i] - s_lo) / coeff;
                    if coeff > 0.0 {
                        hi[j] = hi[j].min(v);
                    } else {
                        lo[j] = lo[j].max(v);
                    }
                }
            }
        }
    }
    (0..n).any(|j| eligible[j] && range_within_box(j, lo[j], hi[j], lb, ub))
}

pub fn eliminate_implied_free_columns_v2(n: usize, a: &Csr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], opts: AggOptions) -> AggregatorResult {
    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);
    let mut accum = SparseAccum::new(n);
    let mut b: Vec<f64> = b.to_vec();
    let mut c: Vec<f64> = c.to_vec();
    let mut real_rows: Vec<Vec<(usize, f64)>> = real_rows.to_vec();
    let mut real_rhs: Vec<f64> = real_rhs.to_vec();
    let is_free = |j: usize| lb[j] == f64::NEG_INFINITY && ub[j] == f64::INFINITY;
    // Rows are kept sorted by column so a coefficient lookup is a binary
    // search, not an O(row length) scan: dense rows (`fit2d`: ~10^4 per row)
    // otherwise make every lookup-per-(column, row) pass quadratic.
    for row in a_rows.iter_mut().chain(real_rows.iter_mut()) {
        if !row.windows(2).all(|w| w[0].0 < w[1].0) {
            row.sort_unstable_by_key(|&(k, _)| k);
        }
    }
    let coef_of = |row: &[(usize, f64)], j: usize| match row.binary_search_by_key(&j, |&(k, _)| k) {
        Ok(p) => row[p].1,
        Err(_) => 0.0,
    };

    // Column counts first, so every per-column list below is allocated
    // once at its final size (building them by `push` alone reallocated
    // each one log2(count) times — ~6% of `stocfor1`'s solve).
    let mut col_a_count = vec![0usize; n];
    let mut col_g_count = vec![0usize; n];
    for row in &a_rows {
        for &(j, v) in row {
            if v != 0.0 {
                col_a_count[j] += 1;
            }
        }
    }
    for row in &real_rows {
        for &(j, v) in row {
            if v != 0.0 {
                col_g_count[j] += 1;
            }
        }
    }
    let mut a_col_idx: Vec<Vec<usize>> = col_a_count.iter().map(|&k| Vec::with_capacity(k)).collect();
    let mut g_col_idx: Vec<Vec<usize>> = col_g_count.iter().map(|&k| Vec::with_capacity(k)).collect();
    // Initial `(row, coeff)` lists, used only for candidate generation
    // (every row is still pristine then): column-compressed, column `j`'s
    // list is `a_col0[a_col0_ptr[j]..a_col0_ptr[j + 1]]` (same entries, same
    // order as one `Vec` per column filled row by row).
    let col_ptr = |count: &[usize]| {
        let mut ptr = Vec::with_capacity(n + 1);
        let mut acc = 0usize;
        ptr.push(0);
        for &k in count {
            acc += k;
            ptr.push(acc);
        }
        ptr
    };
    let a_col0_ptr = col_ptr(&col_a_count);
    let g_col0_ptr = col_ptr(&col_g_count);
    let mut a_col0: Vec<(usize, f64)> = vec![(0, 0.0); a_col0_ptr[n]];
    let mut g_col0: Vec<(usize, f64)> = vec![(0, 0.0); g_col0_ptr[n]];
    // `a_col_idx[j]`/`g_col_idx[j]` need the `sort_unstable + dedup` below
    // only once they hold an out-of-order or repeated row id: built row by
    // row they are ascending, repeating an id only for a row with a
    // duplicate column; a fold appends, so it marks the column dirty.
    // Sorting/deduping an ascending duplicate-free list is a no-op, so
    // skipping it for clean columns changes nothing.
    let mut a_dirty = vec![false; n];
    let mut g_dirty = vec![false; n];
    for (i, row) in a_rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 {
                if a_col_idx[j].last() == Some(&i) {
                    a_dirty[j] = true;
                }
                a_col0[a_col0_ptr[j] + a_col_idx[j].len()] = (i, v);
                a_col_idx[j].push(i);
            }
        }
    }
    for (i, row) in real_rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 {
                if g_col_idx[j].last() == Some(&i) {
                    g_dirty[j] = true;
                }
                g_col0[g_col0_ptr[j] + g_col_idx[j].len()] = (i, v);
                g_col_idx[j].push(i);
            }
        }
    }
    let mut row_max_a: Vec<Option<f64>> = vec![None; a_rows.len()];
    let mut stamp = vec![usize::MAX; n];
    let mut stamp_id = 0usize;
    let mut act_a: Vec<Option<RowActivity>> = vec![None; a_rows.len()];
    let mut act_g: Vec<Option<RowActivity>> = vec![None; real_rows.len()];
    let mut row_deleted = vec![false; a_rows.len()];

    // Implied range of `x_j` from the given (live) equality rows `la` and,
    // with `use_ineq`, inequality rows `lg` (`(row, coeff)` pairs).
    let implied = |j: usize, la: &[(usize, f64)], lg: &[(usize, f64)], a_rows: &[Vec<(usize, f64)>], b: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], act_a: &mut [Option<RowActivity>], act_g: &mut [Option<RowActivity>]| -> (f64, f64) {
        let mut lo = f64::NEG_INFINITY;
        let mut hi = f64::INFINITY;
        for &(i, coeff) in la {
            let act = *act_a[i].get_or_insert_with(|| compute_row_activity(&a_rows[i], lb, ub));
            let (rlo, rhi) = implied_range(&act, j, coeff, b[i], lb, ub);
            lo = lo.max(rlo);
            hi = hi.min(rhi);
        }
        if opts.use_ineq {
            for &(i, coeff) in lg {
                let act = *act_g[i].get_or_insert_with(|| compute_row_activity(&real_rows[i], lb, ub));
                let (s_lo, _) = residual_range(&act, j, coeff, lb, ub);
                if s_lo.is_finite() {
                    let v = (real_rhs[i] - s_lo) / coeff;
                    if coeff > 0.0 {
                        hi = hi.min(v);
                    } else {
                        lo = lo.max(v);
                    }
                }
            }
        }
        (lo, hi)
    };

    let min_a = opts.min_a_count.max(1);
    let mut candidates: Vec<usize> = Vec::new();
    for j in 0..n {
        if col_a_count[j] < min_a || col_a_count[j] + col_g_count[j] < 2 || is_free(j) {
            continue;
        }
        let (la, lg) = (&a_col0[a_col0_ptr[j]..a_col0_ptr[j + 1]], &g_col0[g_col0_ptr[j]..g_col0_ptr[j + 1]]);
        let (lo, hi) = implied(j, la, lg, &a_rows, &b, &real_rows, &real_rhs, &mut act_a, &mut act_g);
        if range_within_box(j, lo, hi, lb, ub) {
            candidates.push(j);
        }
    }
    candidates.sort_by_key(|&j| {
        let collen = col_a_count[j] + col_g_count[j];
        let min_rowlen = a_col_idx[j].iter().map(|&i| a_rows[i].len()).min().unwrap_or(0);
        (min_rowlen.min(collen) != 2, min_rowlen * collen, min_rowlen.min(collen), j)
    });

    let mut substitutions = Vec::new();
    let mut consecutive_fillin_failures = 0usize;
    for j in candidates {
        // Live rows containing `j`, re-verified against current content.
        if a_dirty[j] {
            a_col_idx[j].sort_unstable();
            a_col_idx[j].dedup();
            a_dirty[j] = false;
        }
        let la: Vec<(usize, f64)> = a_col_idx[j].iter().filter(|&&i| !row_deleted[i]).map(|&i| (i, coef_of(&a_rows[i], j))).filter(|&(_, v)| v != 0.0).collect();
        if la.is_empty() {
            continue;
        }
        if g_dirty[j] {
            g_col_idx[j].sort_unstable();
            g_col_idx[j].dedup();
            g_dirty[j] = false;
        }
        let lg: Vec<(usize, f64)> = g_col_idx[j].iter().map(|&i| (i, coef_of(&real_rows[i], j))).filter(|&(_, v)| v != 0.0).collect();
        let (lo, hi) = implied(j, &la, &lg, &a_rows, &b, &real_rows, &real_rhs, &mut act_a, &mut act_g);
        if !range_within_box(j, lo, hi, lb, ub) {
            continue;
        }
        // Pivot: shortest live equality row passing the pivot-ratio guard.
        let mut by_len = la.clone();
        by_len.sort_by_key(|&(i, _)| a_rows[i].len());
        let Some((row_idx, coeff)) = by_len.into_iter().find(|&(i, coeff)| {
            let row_max = *row_max_a[i].get_or_insert_with(|| a_rows[i].iter().map(|&(_, v)| v.abs()).fold(0.0f64, f64::max));
            coeff.abs() >= tunable!("ENOMOTO_T_SUBSTITUTION_PIVOT_RATIO", SUBSTITUTION_PIVOT_RATIO, f64) * row_max
        }) else {
            continue;
        };
        let pivot_row = a_rows[row_idx].clone();
        let terms: Vec<(usize, f64)> = pivot_row.iter().filter(|&&(k, _)| k != j).copied().collect();
        if terms.is_empty() {
            continue;
        }
        let other_a: Vec<usize> = la.iter().map(|&(i, _)| i).filter(|&i| i != row_idx).collect();
        let other_g: Vec<usize> = lg.iter().map(|&(i, _)| i).collect();
        let n_other = other_a.len() + other_g.len();
        let size2 = pivot_row.len() == 2 || n_other + 1 == 2;
        if !(opts.net_fillin && size2) {
            // New nonzeros the folds would create, counted with a stamp
            // array (O(row length) per target, no per-target set).
            let mut gross = 0i64;
            for target in other_a.iter().map(|&i| &a_rows[i]).chain(other_g.iter().map(|&i| &real_rows[i])) {
                stamp_id += 1;
                for &(k, _) in target.iter() {
                    stamp[k] = stamp_id;
                }
                gross += terms.iter().filter(|&&(k, _)| stamp[k] != stamp_id).count() as i64;
            }
            let fillin = if opts.net_fillin { gross - (pivot_row.len() + n_other) as i64 } else { gross };
            if fillin > tunable!("ENOMOTO_T_MAX_FILLIN", MAX_FILLIN, usize) as i64 {
                consecutive_fillin_failures += 1;
                if opts.fillin_break && consecutive_fillin_failures >= tunable!("ENOMOTO_T_MAX_CONSECUTIVE_FILLIN_FAILURES", MAX_CONSECUTIVE_FILLIN_FAILURES, usize) {
                    break;
                }
                continue;
            }
        }
        consecutive_fillin_failures = 0;
        let rhs_i = b[row_idx];
        for &i2 in &other_a {
            let factor = coef_of(&a_rows[i2], j) / coeff;
            a_rows[i2] = axpy_row(&mut accum, &a_rows[i2], &pivot_row, factor, j, TOL);
            b[i2] -= factor * rhs_i;
            act_a[i2] = None;
            row_max_a[i2] = None;
            for &(k, _) in &terms {
                a_col_idx[k].push(i2);
                a_dirty[k] = true;
            }
        }
        for &i2 in &other_g {
            let factor = coef_of(&real_rows[i2], j) / coeff;
            real_rows[i2] = axpy_row(&mut accum, &real_rows[i2], &pivot_row, factor, j, TOL);
            real_rhs[i2] -= factor * rhs_i;
            act_g[i2] = None;
            for &(k, _) in &terms {
                g_col_idx[k].push(i2);
                g_dirty[k] = true;
            }
        }
        let cj = c[j];
        if cj != 0.0 {
            let factor = cj / coeff;
            for &(k, a_ik) in &terms {
                c[k] -= factor * a_ik;
            }
            c[j] = 0.0;
        }
        substitutions.push(Substitution { var: j, terms, rhs: rhs_i, coeff });
        row_deleted[row_idx] = true;
    }

    let mut final_a_rows = Vec::with_capacity(a_rows.len());
    let mut final_b = Vec::with_capacity(b.len());
    for (i, row) in a_rows.into_iter().enumerate() {
        if !row_deleted[i] {
            final_a_rows.push(row);
            final_b.push(b[i]);
        }
    }
    AggregatorResult { a: csr_from_rows(&final_a_rows, n), b: final_b, c, substitutions, real_rows, real_rhs }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csr(rows: &[Vec<(usize, f64)>], n: usize) -> Csr {
        csr_from_rows(rows, n)
    }

    #[test]
    fn no_implied_free_columns_is_a_no_op() {
        // x0 free (freevar's own domain, not implied-free-checked here);
        // x1/x2 bounded but each appears in only one row (colsingleton's
        // domain). Neither x1 nor x2 is implied free by their one row
        // anyway (row0 alone gives x1 in [-inf,inf] since x0 is free).
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![3.0, 1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0, 0.0];
        let ub = vec![f64::INFINITY, 10.0, 10.0];
        let r = eliminate_implied_free_columns(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(r.substitutions.is_empty());
        assert_eq!(r.b, b);
        assert_eq!(r.c, c);
    }

    #[test]
    fn row_activity_matches_hand_derivation() {
        let row = vec![(0, 1.0), (1, -1.0)];
        let lb = vec![0.0, -5.0];
        let ub = vec![10.0, 5.0];
        let activity = compute_row_activity(&row, &lb, &ub);
        // Term 0 (coeff +1, bound [0,10]): contributes 0 to lo, 10 to hi.
        // Term 1 (coeff -1, bound [-5,5]): its lo-side uses ub=5 -> -5; its
        // hi-side uses lb=-5 -> +5. Totals: lo=0-5=-5, hi=10+5=15.
        assert_eq!(activity.lo_finite_sum, -5.0);
        assert_eq!(activity.lo_inf_count, 0);
        assert_eq!(activity.hi_finite_sum, 15.0);
        assert_eq!(activity.hi_inf_count, 0);
    }

    #[test]
    fn residual_range_excludes_only_the_named_columns_own_term() {
        // Row [x0 + x1], x0 in [0,10] (finite), x1 in [-inf,inf] (the row's
        // only unbounded contributor on both sides).
        let row = vec![(0, 1.0), (1, 1.0)];
        let lb = vec![0.0, f64::NEG_INFINITY];
        let ub = vec![10.0, f64::INFINITY];
        let activity = compute_row_activity(&row, &lb, &ub);
        assert_eq!(activity.lo_inf_count, 1);
        assert_eq!(activity.hi_inf_count, 1);
        // Excluding col0 (finite, not the unbounded one) still leaves col1's
        // own unboundedness in the residual.
        assert_eq!(residual_range(&activity, 0, 1.0, &lb, &ub), (f64::NEG_INFINITY, f64::INFINITY));
        // Excluding col1 (the row's *only* unbounded contributor) leaves
        // exactly col0's own finite range: [0,10].
        assert_eq!(residual_range(&activity, 1, 1.0, &lb, &ub), (0.0, 10.0));
    }

    #[test]
    fn eliminates_an_implied_free_column_appearing_in_two_rows() {
        // x0 in [0,10]; x1 in [-5,5] specifically makes row0 alone prove
        // x0's bound redundant (x0 = 5-x1 in [0,10] for any x1 in [-5,5]),
        // so x0 is implied-free and gets eliminated with *no* extra row.
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![3.0, 1.0, 1.0];
        let lb = vec![0.0, -5.0, f64::NEG_INFINITY];
        let ub = vec![10.0, 5.0, f64::INFINITY];
        let r = eliminate_implied_free_columns(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert_eq!(r.substitutions.len(), 1);
        assert_eq!(r.substitutions[0].var, 0);
        assert_eq!(r.a.nrows(), 1);
        // Row 1 (x0 - x2 = 1) folds to -x1 - x2 = -4 (pivot row0, x0's only
        // >=2-appearance candidate row here, matches with itself).
        assert_eq!(r.c[1], 1.0 - 3.0);
        assert_eq!(r.c[2], 1.0);
        // Recover x0 given x1=2, x2=1: x0 = 5 - 2 = 3.
        let x = vec![3.0, 2.0, 1.0];
        assert!((r.substitutions[0].value(&x) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn non_implied_free_column_is_left_alone() {
        // Same shape, but x1's bound [0,100] is loose enough that row0
        // alone implies x0 in [-95,5], not contained in [0,10] -- x0 is
        // *not* implied-free (no other row helps either), so this pass
        // must leave it untouched (unlike the reverted version, which
        // would have eliminated it anyway with an explicit bound row).
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![3.0, 1.0, 1.0];
        let lb = vec![0.0, 0.0, f64::NEG_INFINITY];
        let ub = vec![10.0, 100.0, f64::INFINITY];
        let r = eliminate_implied_free_columns(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(r.substitutions.is_empty());
        assert_eq!(r.a.nrows(), 2);
    }

    #[test]
    fn neither_row_alone_proving_it_leaves_the_column_un_eliminated() {
        // x0's own bound is [0,10]. row0 (x0+x1=5, x1 in [-100,100]) alone
        // implies x0 in [-95,105] -- not contained in [0,10]. row1
        // (x0-x2=1, x2 in [-100,-4]) alone implies x0 = 1+x2 in
        // [1-100, 1-(-4)] = [-99,-3] -- also not contained. This module's
        // check is deliberately row-local (see the module docs on why a
        // cross-row aggregate was tried and reverted), so even though a
        // human could intersect both rows' own true implications to a
        // tighter [-95,-3] (still not enough here, but illustrating the
        // gap this design accepts), this pass only ever asks a single row
        // at a time and finds neither sufficient -- x0 remains
        // un-eliminated by this pass (though `A`'s equality rows are equal
        // to `x0`'s own true value regardless, so nothing is *lost*, just
        // not caught by this particular column-elimination technique).
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![0.0, 0.0, 0.0];
        let lb = vec![0.0, -100.0, -100.0];
        let ub = vec![10.0, 100.0, -4.0];
        let r = eliminate_implied_free_columns(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(r.substitutions.is_empty());
    }

    #[test]
    fn fold_reaches_a_shared_real_row_too() {
        // x0 in [0,10], x1 in [-5,5] (implied-free via row0). One real_rows
        // inequality also references x0.
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![0.0, 0.0, 0.0];
        let lb = vec![0.0, -5.0, f64::NEG_INFINITY];
        let ub = vec![10.0, 5.0, f64::INFINITY];
        let real_rows = vec![vec![(0, 1.0), (2, 1.0)]];
        let real_rhs = vec![7.0];
        let r = eliminate_implied_free_columns(3, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert_eq!(r.substitutions.len(), 1);
        // real_rows must no longer mention column 0: x0 + x2 <= 7 becomes
        // (5 - x1) + x2 <= 7 => -x1 + x2 <= 2.
        assert_eq!(r.real_rows.len(), 1);
        assert_eq!(r.real_rows[0], vec![(1, -1.0), (2, 1.0)]);
        assert_eq!(r.real_rhs, vec![2.0]);
    }

    #[test]
    fn cascading_elimination_within_one_call() {
        // x0 in [0,10] implied-free via row0 given x1 in [-5,5]. Once x0 is
        // eliminated, row1 (x0+x3=3, originally x1... ) -- construct so
        // that x1 *also* becomes implied-free and eliminable within this
        // same call once its own candidacy is checked against the
        // *original* snapshot's bounds (this pass doesn't recompute
        // implied bounds mid-call, but a column already implied-free up
        // front and appearing in >=2 rows is eliminated regardless of
        // fold order).
        // Row0: x0 + x1 = 5. Row1: x0 - x2 = 1. Row2: x1 + x3 = 3.
        // x1 in [-2,2] is implied-free via row2 alone (x3 in [1,5] ->
        // x1 = 3-x3 in [-2,2], matching its own bound exactly).
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)], vec![(1, 1.0), (3, 1.0)]], 4);
        let b = vec![5.0, 1.0, 3.0];
        let c = vec![0.0, 0.0, 0.0, 0.0];
        let lb = vec![0.0, -2.0, f64::NEG_INFINITY, 1.0];
        let ub = vec![10.0, 2.0, f64::INFINITY, 5.0];
        // x0 implied-free via row0 needs x1 in [-5,5] to cover [0,10] via
        // x0=5-x1; x1's own bound here is [-2,2] (tighter), giving x0's
        // row0-implied range [3,7] -- still inside [0,10], so x0 is
        // implied-free too. Both x0 and x1 appear in >= 2 A-rows apiece
        // once row0's own two occurrences count (x0 in rows 0,1; x1 in
        // rows 0,2), so both are candidates.
        let r = eliminate_implied_free_columns(4, &a, &b, &c, &lb, &ub, &[], &[]);
        assert_eq!(r.substitutions.len(), 2);
        let vars: std::collections::BTreeSet<usize> = r.substitutions.iter().map(|s| s.var).collect();
        assert_eq!(vars, [0usize, 1usize].into_iter().collect());
        assert_eq!(r.a.nrows(), 1);
    }

    #[test]
    fn fillin_cost_counts_only_genuinely_new_columns() {
        let terms = vec![(1, 1.0), (2, 1.0), (3, 1.0)];
        let existing_row = vec![(2, 5.0), (4, 1.0)];
        let targets: Vec<&Vec<(usize, f64)>> = vec![&existing_row];
        // Columns 1 and 3 are new to `existing_row`; column 2 already there.
        assert_eq!(fillin_cost(&terms, &targets), 2);
    }

    // --- eliminate_implied_free_columns_xrow --------------------------

    #[test]
    fn xrow_eliminates_a_column_no_single_row_alone_justifies() {
        // x0 in [0,10]. Row0 (x0+x1=5), x1 in [-3,8]: x0 = 5-x1 ranges over
        // [-3,8] -- pokes below the box (-3 < 0), not within [0,10] alone.
        // Row1 (x0-x2=1), x2 in [1,12]: x0 = 1+x2 ranges over [2,13] --
        // pokes above the box (13 > 10), also not within [0,10] alone.
        // Neither row alone suffices, but the two defects are on opposite
        // sides: intersected, [-3,8] n [2,13] = [2,8], comfortably inside
        // [0,10] -- genuinely needs both rows together.
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![0.0, 0.0, 0.0];
        let lb = vec![0.0, -3.0, 1.0];
        let ub = vec![10.0, 8.0, 12.0];
        // Row-local (`eliminate_implied_free_columns`) must find nothing:
        // neither row alone proves x0's bound.
        let row_local = eliminate_implied_free_columns(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(row_local.substitutions.is_empty());
        // The cross-row version must catch it via the intersection.
        let xrow = eliminate_implied_free_columns_xrow(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert_eq!(xrow.substitutions.len(), 1);
        assert_eq!(xrow.substitutions[0].var, 0);
    }

    #[test]
    fn xrow_rejects_a_justification_whose_shared_row_was_already_consumed() {
        // Regression test for the root-caused Netlib `shell` false-`Unbounded`
        // bug (see `eliminate_implied_free_columns_xrow`'s own docs): two
        // columns (x0, x1) mutually justify each other through one shared
        // row (row0), but x0 also has an independent (if individually
        // insufficient) second row, so x0 gets processed and eliminated
        // first, consuming row0 as *its own* pivot. x1's only other row
        // (row2) is, alone, nowhere near tight enough (`x1 + x3 = 0`, `x3`
        // in [-1000,1000]) -- x1's *only* real justification was row0,
        // which is gone by the time x1's own turn comes. A version that
        // trusts row0's snapshot-time contribution here would wrongly
        // eliminate x1 too, dropping its box entirely; the fixed version
        // must instead leave x1 exactly as it was.
        //
        // x0 in [0,10], x1 in [0,10], x2 free (makes row1 -- x0's second,
        // longer row -- individually vacuous so it never gets picked as
        // x0's pivot ahead of the shorter, shared row0), x3 in
        // [-1000,1000] (makes row2 -- x1's second row -- individually far
        // too loose), x4 in [0,1] (padding so row1 is strictly longer than
        // row0, biasing pivot selection toward row0).
        let a = csr(
            &[
                vec![(0, 1.0), (1, -1.0)],           // row0 (shared): x0 - x1 = 0
                vec![(0, 1.0), (2, 1.0), (4, 1.0)],  // row1 (x0's own, vacuous): x0 + x2 + x4 = 5
                vec![(1, 1.0), (3, 1.0)],             // row2 (x1's own, too loose): x1 + x3 = 0
            ],
            5,
        );
        let b = vec![0.0, 5.0, 0.0];
        let c = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let lb = vec![0.0, 0.0, f64::NEG_INFINITY, -1000.0, 0.0];
        let ub = vec![10.0, 10.0, f64::INFINITY, 1000.0, 1.0];

        let xrow = eliminate_implied_free_columns_xrow(5, &a, &b, &c, &lb, &ub, &[], &[]);
        assert_eq!(xrow.substitutions.len(), 1, "expected only x0 eliminated, got {:?}", xrow.substitutions.iter().map(|s| s.var).collect::<Vec<_>>());
        assert_eq!(xrow.substitutions[0].var, 0);
        assert!(xrow.substitutions.iter().all(|s| s.var != 1), "x1 must not be eliminated via a since-deleted justifying row");
    }

    fn all_on() -> AggOptions {
        AggOptions { use_ineq: true, min_a_count: 1, net_fillin: true, fillin_break: true }
    }

    #[test]
    fn v2_rejects_a_justification_whose_shared_row_was_already_consumed() {
        // Same `shell` regression shape as the xrow test above, through v2.
        let a = csr(
            &[
                vec![(0, 1.0), (1, -1.0)],
                vec![(0, 1.0), (2, 1.0), (4, 1.0)],
                vec![(1, 1.0), (3, 1.0)],
            ],
            5,
        );
        let b = vec![0.0, 5.0, 0.0];
        let c = vec![0.0; 5];
        let lb = vec![0.0, 0.0, f64::NEG_INFINITY, -1000.0, 0.0];
        let ub = vec![10.0, 10.0, f64::INFINITY, 1000.0, 1.0];
        let r = eliminate_implied_free_columns_v2(5, &a, &b, &c, &lb, &ub, &[], &[], all_on());
        assert!(r.substitutions.iter().all(|s| s.var != 1), "x1 must not be eliminated via a since-deleted justifying row");
    }

    #[test]
    fn v2_uses_an_inequality_row_to_justify_the_other_side() {
        // x0 - x1 = 0 (x1 >= 0) implies x0 >= 0 but no upper bound; the real
        // inequality x0 + x2 <= 5 (x2 in [0,1]) implies x0 <= 5. Together
        // they cover x0's box [0,5], so x0 (one equality row + one
        // inequality row — a shape the old gate rejected) is eliminated and
        // the inequality row is folded to x1 + x2 <= 5.
        let a = csr(&[vec![(0, 1.0), (1, -1.0)]], 3);
        let b = vec![0.0];
        let c = vec![1.0, 0.0, 0.0];
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![5.0, f64::INFINITY, 1.0];
        let g = vec![vec![(0, 1.0), (2, 1.0)]];
        let h = vec![5.0];
        let r = eliminate_implied_free_columns_v2(3, &a, &b, &c, &lb, &ub, &g, &h, all_on());
        assert_eq!(r.substitutions.len(), 1);
        assert_eq!(r.substitutions[0].var, 0);
        assert_eq!(r.a.nrows(), 0);
        assert_eq!(r.real_rows, vec![vec![(1, 1.0), (2, 1.0)]]);
        assert_eq!(r.real_rhs, vec![5.0]);
        assert_eq!(r.c, vec![0.0, 1.0, 0.0]);

        // Without the inequality-row justification the upper side is unproven.
        let off = AggOptions { use_ineq: false, ..all_on() };
        let r = eliminate_implied_free_columns_v2(3, &a, &b, &c, &lb, &ub, &g, &h, off);
        assert!(r.substitutions.is_empty());
    }

    #[test]
    fn v2_min_a_count_two_keeps_the_old_gate() {
        let a = csr(&[vec![(0, 1.0), (1, -1.0)]], 3);
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![5.0, f64::INFINITY, 1.0];
        let opts = AggOptions { min_a_count: 2, ..all_on() };
        let r = eliminate_implied_free_columns_v2(3, &a, &[0.0], &[0.0; 3], &lb, &ub, &[vec![(0, 1.0), (2, 1.0)]], &[5.0], opts);
        assert!(r.substitutions.is_empty());
    }

    /// `v2_has_candidate == false` must imply v2 eliminates nothing, and the
    /// `_if_any` wrapper must return v2's own result whenever it is `Some`.
    #[test]
    fn v2_has_candidate_is_consistent_with_v2() {
        let mut state: u64 = 0x2545_f491_4f6c_dd1d;
        let mut rnd = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut seen_none = 0;
        let mut seen_some = 0;
        for trial in 0..400 {
            let n = 3 + (trial % 5);
            let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
            for _ in 0..(1 + rnd() % 4) {
                let mut row: Vec<(usize, f64)> = Vec::new();
                for _ in 0..(1 + rnd() % 3) {
                    let j = (rnd() % n as u64) as usize;
                    if row.iter().all(|&(k, _)| k != j) {
                        row.push((j, ((rnd() % 5) as f64 - 2.0) + 0.5));
                    }
                }
                rows.push(row);
            }
            let b: Vec<f64> = rows.iter().map(|_| (rnd() % 7) as f64 - 3.0).collect();
            let mut g: Vec<Vec<(usize, f64)>> = Vec::new();
            for _ in 0..(rnd() % 3) {
                // Deliberately unsorted inequality rows.
                let j1 = (rnd() % n as u64) as usize;
                let j0 = (rnd() % n as u64) as usize;
                if j0 != j1 {
                    g.push(vec![(j1.max(j0), 1.0), (j1.min(j0), -0.5)]);
                }
            }
            let h: Vec<f64> = g.iter().map(|_| (rnd() % 9) as f64).collect();
            let bnd = |x: u64| [f64::NEG_INFINITY, -5.0, 0.0, 1.0][(x % 4) as usize];
            let lb: Vec<f64> = (0..n).map(|_| bnd(rnd())).collect();
            let ub: Vec<f64> = lb.iter().map(|&l| if rnd() % 4 == 0 { f64::INFINITY } else { l.max(0.0) + (rnd() % 10) as f64 + 1.0 }).collect();
            let c: Vec<f64> = (0..n).map(|_| (rnd() % 3) as f64).collect();
            let a = csr(&rows, n);
            let full = eliminate_implied_free_columns_v2(n, &a, &b, &c, &lb, &ub, &g, &h, all_on());
            match eliminate_implied_free_columns_v2_if_any(n, &a, &b, &c, &lb, &ub, &g, &h, all_on()) {
                None => {
                    seen_none += 1;
                    assert!(full.substitutions.is_empty(), "trial {trial}");
                }
                Some(r) => {
                    seen_some += 1;
                    assert_eq!(r.substitutions.len(), full.substitutions.len(), "trial {trial}");
                    assert_eq!(r.real_rows, full.real_rows, "trial {trial}");
                    assert_eq!(r.c, full.c, "trial {trial}");
                }
            }
        }
        assert!(seen_none > 0 && seen_some > 0, "none={seen_none} some={seen_some}");
    }
}
