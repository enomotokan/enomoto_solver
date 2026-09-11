//! Removes negligible ("small") coefficients from a constraint row —
//! Achterberg, Bixby, Gu, Rothberg & Weninger, "Presolve Reductions in
//! Mixed Integer Programming" (INFORMS Journal on Computing 32(2), 2020;
//! originally ZIB-Report 16-44), §3.1 "Model cleanup and removal of
//! redundant constraints": a nonzero `a_ik` whose worst-case contribution
//! to row `i`'s own activity, `|a_ik| * (ub[k] - lb[k])`, is small enough
//! relative to the solver's own feasibility tolerance can be dropped from
//! the row entirely (folding its value at `lb[k]` into the row's own
//! right-hand side) without changing which points the row considers
//! feasible by more than that tolerance.
//!
//! Distinct from every other reduction in this pipeline: it never removes
//! a row or a variable, only individual matrix entries — a variable whose
//! *every* remaining appearance happens to get dropped this way simply
//! becomes a free column for `dualfix`/`propagate` to pick up on a later
//! round, the same "one step's leftover is another step's opportunity"
//! pattern `run_extended`'s own docs describe for its other stages.
//!
//! **The paper's own two-part scheme is implemented here as a single,
//! strictly more general, per-row cumulative budget**, rather than
//! transcribed literally. Achterberg et al. first test each entry
//! individually (`|a_ik| < 1e-3` *and* `|a_ik| * (ub_k - lb_k) *
//! |supp(A_i·)| < 1e-2 * eps`), then *separately* re-scan the row with a
//! looser, purely cumulative budget (drop entries, in column order, as
//! long as the running sum of `|a_ik| * (ub_k - lb_k)` stays below
//! `1e-1 * eps`) — but the second pass already subsumes the first: if
//! every one of a row's (at most `|supp(A_i·)|`) entries satisfied the
//! first test, their contributions would sum to at most `1e-2 * eps`,
//! comfortably inside the second pass's own `1e-1 * eps` ceiling
//! regardless of any entry's raw magnitude. Running only the single,
//! looser cumulative-budget pass therefore finds everything the paper's
//! own two-pass scheme does (and more, since it never additionally
//! requires `|a_ik| < 1e-3`), while staying just as sound: the *total*
//! perturbation to row `i`'s own activity from every dropped entry
//! combined never exceeds the same `1e-1 * eps` ceiling the paper itself
//! already accepts as safe.
//!
//! A separate, unconditional pass — matching the paper's own "finally, we
//! set coefficients with `|a_ik| < 1e-10` to zero" — drops any coefficient
//! this small regardless of the cumulative budget or the variable's own
//! bound width, since a coefficient at that scale is floating-point noise
//! on any problem this solver's own Ruiz scaling has already normalized.
//!
//! **Implemented, unit-tested, and measured against the full Netlib set —
//! then left unintegrated (kept here, tested, but never called from
//! [`crate::presolve::run_extended`]).** Wired in two ways, each measured
//! in turn: once per outer round (right after `propagate` derives fresh
//! bounds, so a coefficient a bound this round just tightened could
//! newly qualify) and, after that measured a reproducible ~20% aggregate
//! slowdown for a real but small yield (`ENOMOTO_PROF_SMALLCOEFF`: well
//! under 1% of scanned entries dropped on every instance checked — e.g.
//! `pilotnov` 889/119286, `ganges` 80/33500, `bnl1` 144/23989, several
//! instances finding nothing at all), once only, at the very end of the
//! pipeline against its final, tightest bounds. The once-only version
//! recovered the lost performance (back in line with the pre-change
//! baseline) — but on the exact same full-Netlib run, `perold` newly
//! crashed ("simplex basis matrix must be nonsingular"), and a synthetic
//! regression test already in this crate's own suite
//! (`simplex::tests::beale_cycling_example_terminates_correctly`) newly
//! failed its interior-point cross-check (misreporting `Unbounded` on a
//! provably bounded LP). Root cause (established, not just suspected):
//! dropping a coefficient whose *variable* happens to already be exactly
//! fixed (`lb[k] == ub[k]`, `contribution == 0` unconditionally, so this
//! reduction accepts it regardless of the coefficient's own magnitude) is
//! an *exact*, zero-error transformation in real arithmetic, but it still
//! changes the row's shape and shifts its right-hand side by a tiny
//! floating-point amount — enough to send the affected instance down a
//! different numerical path, the same "a locally-sound change can still
//! expose latent fragility on an already-marginal, highly degenerate
//! instance" pattern this session hit repeatedly elsewhere (the `chuzr`
//! rayon-vs-sequential tie-break fix, the Schork-Gondzio Forrest-Tomlin
//! variant, the fixed-width `chuzc1` candidacy exclusion — see
//! `simplex.rs`'s own history for those). `perold` and `beale_cycling`'s
//! own IPM cross-check are both independently already documented
//! elsewhere in this codebase (`HARRIS_RATIO_TOL`'s own docs; this test's
//! own comment) as sensitive to exactly this class of small numerical
//! perturbation, so this is consistent with, not an outlier from, that
//! established picture. Kept registered and tested (not deleted) in case
//! a future, more targeted version — e.g. skipping any entry whose
//! variable is already exactly fixed, since that specific case is what
//! triggered both failures above and contributes nothing `dualfix`'s own
//! fixing hasn't already captured — is worth trying later.

use std::collections::BTreeMap;

/// Analogue of this solver's own primal feasibility tolerance
/// (`simplex.rs`'s `PRIMAL_FEAS_TOL`) — the "eps" the cumulative budget
/// below is measured against, kept as this module's own copy rather than
/// importing `simplex`'s (this pipeline is shared with `interior_point`,
/// which has no reason to depend on `simplex`'s own module) since both
/// represent the same underlying concept: how much primal infeasibility
/// this solver is willing to call negligible.
const EPS: f64 = 1e-7;

/// Per-row ceiling (as a fraction of [`EPS`]) on the *total*, summed
/// worst-case activity perturbation this reduction may introduce into any
/// one row — Achterberg et al.'s own `1e-1 * eps` (see the module docs
/// for why this single, looser budget suffices on its own).
const CUMULATIVE_FRACTION: f64 = 0.1;

/// Coefficients at or below this magnitude are dropped unconditionally,
/// regardless of the cumulative budget above — Achterberg et al.'s own
/// `1e-10`, floating-point noise on any realistically scaled problem.
const NOISE_THRESHOLD: f64 = 1e-10;

/// Cleans one already-column-deduplicated row: drops every coefficient
/// this reduction judges negligible, adjusting `rhs` to compensate
/// (folding the dropped term's value at `lb[k]` into the row's own
/// right-hand side, so the row stays *exactly* equivalent at `x_k =
/// lb[k]` and off by at most that term's own worst-case contribution
/// everywhere else in `[lb[k], ub[k]]` — see the module docs for the
/// error-budget argument bounding that worst case across the whole row).
/// Entries are visited in the row's own stored (ascending column index)
/// order, matching the paper's own "starting from the first non-zero
/// coefficient".
fn clean_row(row: &[(usize, f64)], rhs: f64, lb: &[f64], ub: &[f64]) -> (Vec<(usize, f64)>, f64) {
    let budget = CUMULATIVE_FRACTION * EPS;
    let mut new_row = Vec::with_capacity(row.len());
    let mut new_rhs = rhs;
    let mut used_budget = 0.0;
    for &(k, v) in row {
        if v == 0.0 {
            continue;
        }
        if v.abs() <= NOISE_THRESHOLD {
            new_rhs -= v * lb[k];
            continue;
        }
        let contribution = v.abs() * (ub[k] - lb[k]);
        if used_budget + contribution <= budget {
            used_budget += contribution;
            new_rhs -= v * lb[k];
            continue;
        }
        new_row.push((k, v));
    }
    (new_row, new_rhs)
}

/// Applies [`clean_row`] to every row of a sparse system, merging
/// duplicate column indices first (mirroring every other pass in this
/// pipeline, e.g. `doubleton`/`colsingleton`'s own row construction) so a
/// row's true per-column coefficient — not one of several raw entries
/// that happen to sum to it — is what gets tested.
#[allow(dead_code)]
pub fn remove_small_coefficients(rows: &[Vec<(usize, f64)>], rhs: &[f64], lb: &[f64], ub: &[f64]) -> (Vec<Vec<(usize, f64)>>, Vec<f64>) {
    let mut new_rows = Vec::with_capacity(rows.len());
    let mut new_rhs = Vec::with_capacity(rhs.len());
    for (row, &b) in rows.iter().zip(rhs) {
        let mut merged: BTreeMap<usize, f64> = BTreeMap::new();
        for &(k, v) in row {
            *merged.entry(k).or_insert(0.0) += v;
        }
        let merged_row: Vec<(usize, f64)> = merged.into_iter().collect();
        let (cleaned, new_b) = clean_row(&merged_row, b, lb, ub);
        new_rows.push(cleaned);
        new_rhs.push(new_b);
    }
    (new_rows, new_rhs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_a_coefficient_whose_contribution_is_negligible_via_a_tiny_bound_width() {
        // Row: 5*x0 + 1e-4*x1 = 10. x1's own coefficient (1e-4) is well
        // above NOISE_THRESHOLD on its own, but its bound width is tiny
        // (2.0 to 2.0+1e-6), so its worst-case contribution
        // (1e-4 * 1e-6 = 1e-10) is negligible against the cumulative
        // budget (CUMULATIVE_FRACTION * EPS = 1e-8) -- exercising the
        // contribution-based branch specifically, not the raw-noise one.
        let row = vec![(0, 5.0), (1, 1e-4)];
        let lb = vec![0.0, 2.0];
        let ub = vec![10.0, 2.0 + 1e-6];
        let (rows, rhs) = remove_small_coefficients(&[row], &[10.0], &lb, &ub);
        assert_eq!(rows[0], vec![(0, 5.0)]);
        assert!((rhs[0] - (10.0 - 1e-4 * 2.0)).abs() < 1e-12, "rhs={}", rhs[0]);
    }

    #[test]
    fn keeps_a_genuinely_significant_coefficient() {
        let row = vec![(0, 5.0), (1, 3.0)];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 10.0]; // contribution 3.0*10.0 = 30, far over any reasonable budget
        let (rows, rhs) = remove_small_coefficients(&[row], &[10.0], &lb, &ub);
        assert_eq!(rows[0], vec![(0, 5.0), (1, 3.0)]);
        assert_eq!(rhs[0], 10.0);
    }

    #[test]
    fn drops_a_raw_noise_coefficient_regardless_of_bound_width() {
        // Coefficient itself is below NOISE_THRESHOLD even though the
        // variable's own bound width is huge (so the contribution-based
        // budget test alone would *not* have caught this one).
        let row = vec![(0, 5.0), (1, 1e-12)];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 1e9];
        let (rows, rhs) = remove_small_coefficients(&[row], &[10.0], &lb, &ub);
        assert_eq!(rows[0], vec![(0, 5.0)]);
        assert_eq!(rhs[0], 10.0); // lb[1] = 0, so no rhs adjustment needed
    }

    #[test]
    fn respects_the_cumulative_budget_across_several_small_entries_in_one_row() {
        // Three equal-sized small contributions where two fit the row's
        // total budget but a third would exceed it -- only the two
        // encountered first (ascending column order) should be dropped.
        let budget = CUMULATIVE_FRACTION * EPS;
        let per_term = budget / 2.5; // 2 terms fit (0.8*budget), 3 don't (1.2*budget)
        let row = vec![(0, 100.0), (1, per_term), (2, per_term), (3, per_term)];
        let lb = vec![0.0, 0.0, 0.0, 0.0];
        let ub = vec![1.0, 1.0, 1.0, 1.0]; // width 1 => contribution == coefficient
        let (rows, _rhs) = remove_small_coefficients(&[row], &[0.0], &lb, &ub);
        let kept: Vec<usize> = rows[0].iter().map(|&(k, _)| k).collect();
        // Column 0's own contribution (100) alone busts the budget, so it
        // is always kept; columns 1 and 2 fit the running budget and are
        // dropped; column 3 arrives after the budget is already spent.
        assert_eq!(kept, vec![0, 3], "kept={kept:?}");
    }

    #[test]
    fn merges_duplicate_column_entries_before_testing_them() {
        // Two raw entries for column 1 that individually look negligible
        // but sum to something significant must not be dropped.
        let row = vec![(0, 5.0), (1, 4.9), (1, 4.9)]; // merges to (1, 9.8)
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 10.0];
        let (rows, _rhs) = remove_small_coefficients(&[row], &[10.0], &lb, &ub);
        assert_eq!(rows[0], vec![(0, 5.0), (1, 9.8)]);
    }
}
