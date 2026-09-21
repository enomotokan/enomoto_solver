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
//! docs, and [`RowActivity`]'s for how it stays cheap), but deliberately
//! keeps the *safety* condition row-local rather than the cross-row
//! aggregate this doc comment originally described: a column is only
//! eliminated via a specific row `R` when `R`'s *own* activity (using every
//! other column's current bound, nothing aggregated in from any other row)
//! already proves `x_j`'s box bound redundant. A cross-row aggregate
//! version (intersecting every row's own implication before checking
//! against `[lb_j, ub_j]`) was tried and reverted: it eliminated `stocfor2`
//! correctly, matching an ablation of HiGHS's own Aggregator to within a
//! few percent, but produced a false `Unbounded` on `shell` — a real
//! correctness bug this crate's benchmark objective-check caught before it
//! shipped. The row-local version above never showed this failure across
//! the full 73-problem Netlib set in an earlier form of this module (the
//! same check, just recomputed less efficiently) and is the one actually
//! used now; the cross-row aggregate's exact defect was not root-caused
//! before reverting to the version already known safe — see this crate's
//! own project memory for that measurement's full writeup if revisiting
//! the aggregate again.
//!
//! Re-validated at *elimination* time against the pivot row's *current*
//! content (not just at candidate-generation time against the initial
//! snapshot): a fold performed earlier in this same call can rewrite a row
//! that is also some other column's pivot candidate, and the row-local
//! check must hold for whatever content that row actually has when used,
//! not merely what it had when candidates were first collected.
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
//! are never used to justify an elimination even indirectly, precisely
//! because of the cross-row aggregate's own reverted history above); `G`'s
//! real rows still get folded like any other row referencing an eliminated
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
use crate::sparse::{csr_from_rows, Csr};

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

/// `row - factor * pivot`, dropping `drop_col` outright and any entry that
/// lands within `TOL` of zero — identical to `freevar::axpy_row` (see its
/// own docs on the `BTreeMap` merge and why it exists).
fn axpy_row(row: &[(usize, f64)], pivot: &[(usize, f64)], factor: f64, drop_col: usize) -> Vec<(usize, f64)> {
    let mut map: std::collections::BTreeMap<usize, f64> = row.iter().copied().collect();
    for &(k, v) in pivot {
        *map.entry(k).or_insert(0.0) -= factor * v;
    }
    map.remove(&drop_col);
    map.retain(|_, v| v.abs() > TOL);
    map.into_iter().collect()
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

/// Whether equality row `sum + coeff*x_j = rhs`'s *own* activity (using
/// every other column's current bound, via `activity` —
/// `compute_row_activity`'s summary of this same row, nothing aggregated in
/// from any other row `j` might also appear in) already proves `x_j`'s box
/// bound `[lb[j], ub[j]]` redundant. See the module docs for why this stays
/// row-local rather than aggregating across every row `j` appears in (a
/// cross-row aggregate version was tried and reverted after producing a
/// false `Unbounded` on a real Netlib instance).
fn row_implies_own_bound(activity: &RowActivity, j: usize, coeff: f64, rhs: f64, lb: &[f64], ub: &[f64]) -> bool {
    let (s_lo, s_hi) = residual_range(activity, j, coeff, lb, ub);
    let v1 = (rhs - s_hi) / coeff;
    let v2 = (rhs - s_lo) / coeff;
    let (lo, hi) = (v1.min(v2), v1.max(v2));
    (lb[j] == f64::NEG_INFINITY || lo >= lb[j] - TOL) && (ub[j] == f64::INFINITY || hi <= ub[j] + TOL)
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
pub fn eliminate_implied_free_columns(n: usize, a: &Csr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64]) -> AggregatorResult {
    let ar = a.as_ref();
    let mut a_rows: Vec<Vec<(usize, f64)>> = (0..ar.nrows()).map(|i| ar.col_indices_of_row(i).zip(ar.values_of_row(i)).map(|(j, &v)| (j, v)).collect()).collect();
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
    for (i, row) in a_rows.iter().enumerate() {
        let activity = compute_row_activity(row, lb, ub);
        for &(j, v) in row {
            if v != 0.0 && col_a_count[j] >= 2 && !is_free(j) && row_implies_own_bound(&activity, j, v, b[i], lb, ub) {
                candidates.push((i, j));
            }
        }
    }
    candidates.sort_by_key(|&(i, j)| {
        let rowlen = a_rows[i].len();
        let collen = col_a_count[j];
        let min_len = rowlen.min(collen);
        (min_len != 2, rowlen * collen, min_len, i, j)
    });

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
        if coeff.abs() < SUBSTITUTION_PIVOT_RATIO * row_max {
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
        if fillin > MAX_FILLIN {
            consecutive_fillin_failures += 1;
            if consecutive_fillin_failures >= MAX_CONSECUTIVE_FILLIN_FAILURES {
                break;
            }
            continue;
        }
        consecutive_fillin_failures = 0;

        let rhs_i = b[row_idx];
        for &i2 in &other_a {
            let a_i2j = a_rows[i2].iter().find(|&&(k, _)| k == j).unwrap().1;
            let factor = a_i2j / coeff;
            a_rows[i2] = axpy_row(&a_rows[i2], &pivot_row, factor, j);
            b[i2] -= factor * rhs_i;
        }
        for &i2 in &other_g {
            let a_i2j = real_rows[i2].iter().find(|&&(k, _)| k == j).unwrap().1;
            let factor = a_i2j / coeff;
            real_rows[i2] = axpy_row(&real_rows[i2], &pivot_row, factor, j);
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
}
