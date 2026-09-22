//! DominatedColumns (Andersen & Andersen, "Presolving in Linear
//! Programming", Mathematical Programming 71 (1995), §3.1): generalizes
//! [`dualfix`](super::dualfix)'s own zero-lock special case — there, a
//! variable is fixed only by comparing it implicitly against the all-zero
//! "column" (no real row resists moving it at all, i.e. its lock count on
//! one side is `0`); here, any *other* real column can play that role,
//! catching a variable `dualfix` alone never would because some row
//! genuinely does resist moving it — just not as much as it resists
//! moving some other column.
//!
//! For two variables `j`, `k`, neither appearing in any equality row (the
//! same disqualification `dualfix` uses — an equality-locked variable
//! can't be pushed either direction without a compensating move
//! elsewhere, which this pass doesn't attempt to find), `j` *dominates*
//! `k` when `c[j] <= c[k]` and, for every real inequality row `i`,
//! `G[i,j] <= G[i,k]` (both compared with `TOL` slack). Given that,
//! shifting mass from `k` to `j` (`x_j += delta, x_k -= delta`, any
//! `delta >= 0`) never increases any row's activity (each row's own change
//! is `delta * (G[i,j] - G[i,k]) <= 0`) and never increases the objective
//! (`delta * (c[j] - c[k]) <= 0`), so there is always an optimal solution
//! at one of the two extremes of how far that shift can run before a bound
//! stops it:
//!
//!   - if `j` has no finite upper bound, the shift can run until `k` hits
//!     its own lower bound (needs that bound finite, or there's nowhere
//!     for the shift to stop) — fix `k` there.
//!   - if `k` has no finite lower bound, the shift can instead run until
//!     `j` hits its own upper bound (needs that bound finite) — fix `j`
//!     there.
//!
//! (When *both* escape routes are open the shift is unbounded in the
//! direction that only helps the objective — a real unboundedness the LP
//! has regardless, not a reduction this pass should paper over by
//! guessing a side; when *neither* is open, only a partial bound
//! tightening is available, which — like `dualfix` and `doubleton` — this
//! pass leaves alone rather than folding a second reduction kind into one.)
//!
//! **Candidate search**: checking every pair of columns is `O(n^2)`;
//! instead, for each candidate column `j` this only compares against the
//! columns sharing `j`'s own *shortest* row (the row bounding the fewest
//! other candidates) — the same "let the cheapest available row bound the
//! search" idea [`sparsify`](super::sparsify)'s own `anchor` uses, just
//! picked per-column here instead of per-equality-row there. A domination
//! this misses (its partner column never shares that particular shortest
//! row) is left for a later round's structural change to expose, not
//! chased down exhaustively.
//!
//! Single-pass, non-cascading, decided from the input snapshot (mirrors
//! every other pass in this pipeline): a column committed as *either* side
//! of one fix this call — the one being fixed, or the one whose infinite
//! bound justified it — is never reused as either side of a second fix in
//! the same call, even though reusing it purely as an *anchor* again would
//! often still be sound (its own freedom to absorb a shift isn't used up
//! by lending it to one fix). The conservative blanket rule is simpler to
//! prove correct outright: once a column might itself end up fixed this
//! same pass, letting it also serve as the premise for fixing something
//! else risks the same kind of stale-assumption bug `doubleton`'s own
//! `claimed` guard exists to rule out (see that module's docs) — any
//! opportunity this costs is picked up by the next round instead.
//!
//! **Implemented, unit-tested, measured against the full Netlib set — then
//! left unintegrated (kept here, tested, but never called from
//! [`crate::presolve::run_extended`]), mirroring [`sparsify`](super::sparsify)'s
//! own precedent.** The two escape-route conditions above genuinely need a
//! *literal* infinity, not merely "a wide finite range" — a finite range,
//! however large, can always be defeated by an adversarial choice of the
//! *other* variable's own starting point within *its* range: e.g. with `j`
//! ranging over `[0,3]` and `k` over `[0,10]` (`k`'s range wider), an
//! optimal solution sitting at `x_j=0, x_k=0` has *zero* room to shift
//! either way, even though `range_k >= range_j` — a tempting but unsound
//! generalization this module's own history once tried and caught before
//! shipping. Only an unbounded side sidesteps this (infinity beats any
//! finite worst case unconditionally), which is also exactly why
//! `dualfix`'s own zero-lock case needs no escape route at all: comparing
//! against the *implicit* all-zero column never needs the zero column
//! itself to move, so there is nothing for an adversarial starting point to
//! defeat.
//!
//! That requirement collides with an invariant the rest of this crate
//! enforces outright: every variable must have two *finite* bounds
//! (`model.rs::add_variable` rejects `+/-inf` bounds at the API boundary;
//! see `presolve.rs`'s own "bounded-variable invariant" docs on
//! [`super::build_a_g`]) — so neither `ub[j] == f64::INFINITY` nor
//! `lb[k] == f64::NEG_INFINITY` can ever be true for any model this crate
//! can actually build, and this pass's two fix branches are unreachable in
//! practice, not merely rare. Confirmed empirically, not just argued:
//! instrumented and run across all 73 solvable Netlib/HiGHS-comparison
//! instances (`python/enomoto_solver/benchmark_highs.py`), this pass fixed
//! zero variables on every single one (consistent with every instance's
//! `+/-inf` bounds already having been substituted with a large finite
//! `BIG_M` before reaching this crate — itself downstream of the same
//! finite-bounds invariant), while still costing roughly 5-8% of this
//! crate's own total solve time in pure candidate-search overhead if wired
//! into [`crate::presolve::run_extended`] (aggregate Netlib total: 3.67s
//! unwired vs. 3.88s wired, `ours`-side only; the `ours`-vs-HiGHS ratio
//! itself stayed within run-to-run noise, ~2.02-2.03x either way). Kept
//! here for its correct, tested core logic — e.g. if this crate's
//! finite-bounds invariant is ever relaxed — rather than deleted outright.

use crate::sparse::{Csr, csr_row_iter};
use std::collections::{BTreeMap, BTreeSet, HashSet};

const TOL: f64 = 1e-9;

/// Returns `(j, value)` for every variable fixed outright — same shape as
/// [`dualfix::fix_dominated_variables`](super::dualfix::fix_dominated_variables).
pub fn fix_dominated_columns(n: usize, a: &Csr, real_g_rows: &[Vec<(usize, f64)>], c: &[f64], lb: &[f64], ub: &[f64]) -> Vec<(usize, f64)> {
    let mut in_equality = vec![false; n];
    let ar = a.as_ref();
    for i in 0..ar.nrows() {
        for (j, v) in csr_row_iter(a, i) {
            if v != 0.0 {
                in_equality[j] = true;
            }
        }
    }

    // Full column vectors (row index -> coefficient), one per non-equality
    // variable, restricted to `real_g_rows` (box-bound rows carry no
    // domination information of their own — they're the bounds being
    // compared against, not additional constraints).
    let mut col_rows: Vec<BTreeMap<usize, f64>> = vec![BTreeMap::new(); n];
    for (i, row) in real_g_rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 && !in_equality[j] {
                col_rows[j].insert(i, v);
            }
        }
    }

    // Deterministic iteration order matters here: which of two competing
    // candidates gets to claim a shared column first decides which fix
    // survives the `used` guard below, so candidate order must not depend
    // on hash-map iteration order.
    let mut candidates: BTreeSet<(usize, usize)> = BTreeSet::new();
    for j in 0..n {
        if in_equality[j] || col_rows[j].is_empty() {
            continue;
        }
        let anchor = *col_rows[j].keys().min_by_key(|&&i| real_g_rows[i].len()).unwrap();
        for &(k, _) in &real_g_rows[anchor] {
            if k != j && !in_equality[k] {
                candidates.insert((j.min(k), j.max(k)));
            }
        }
    }

    let mut used: HashSet<usize> = HashSet::new();
    let mut fixed: Vec<(usize, f64)> = Vec::new();
    for (p, q) in candidates {
        if used.contains(&p) || used.contains(&q) {
            continue;
        }
        let outcome = try_fix_pair(p, q, &col_rows, c, lb, ub).or_else(|| try_fix_pair(q, p, &col_rows, c, lb, ub));
        if let Some((var, value)) = outcome {
            used.insert(p);
            used.insert(q);
            fixed.push((var, value));
        }
    }
    fixed
}

/// Checks whether `j` dominates `k` per the module docs, returning the
/// resulting `(var, value)` fix — whichever side's infinite bound made it
/// possible — or `None` if the pointwise comparison fails or neither
/// escape route is open.
fn try_fix_pair(j: usize, k: usize, col_rows: &[BTreeMap<usize, f64>], c: &[f64], lb: &[f64], ub: &[f64]) -> Option<(usize, f64)> {
    if c[j] > c[k] + TOL {
        return None;
    }
    for (&i, &vk) in &col_rows[k] {
        let vj = col_rows[j].get(&i).copied().unwrap_or(0.0);
        if vj > vk + TOL {
            return None;
        }
    }
    for (&i, &vj) in &col_rows[j] {
        if col_rows[k].contains_key(&i) {
            continue; // already covered by the loop above
        }
        // `k`'s implicit coefficient on this row is `0.0`.
        if vj > TOL {
            return None;
        }
    }
    if ub[j] == f64::INFINITY && lb[k].is_finite() {
        Some((k, lb[k]))
    } else if lb[k] == f64::NEG_INFINITY && ub[j].is_finite() {
        Some((j, ub[j]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    #[test]
    fn fixes_the_dominated_column_to_its_lower_bound() {
        // Row: x0 + 2*x1 <= 10, so G[.,0]=1 <= G[.,1]=2 pointwise and
        // c0 <= c1 -- x0 dominates x1. x0 has no finite upper bound, x1
        // has a finite lower bound, so x1 should be fixed there.
        let a = csr_from_rows(&[], 2);
        let real_g_rows = vec![vec![(0, 1.0), (1, 2.0)]];
        let c = vec![1.0, 1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY];

        let fixed = fix_dominated_columns(2, &a, &real_g_rows, &c, &lb, &ub);
        assert_eq!(fixed, vec![(1, 0.0)]);
    }

    #[test]
    fn fixes_the_dominating_column_to_its_upper_bound_when_the_dominated_side_is_unbounded_below() {
        // Same pointwise relation (x0 dominates x1), but now x1 has no
        // finite lower bound while x0 has a finite upper bound -- the
        // shift runs the other way, pinning x0 at its own ceiling instead.
        let a = csr_from_rows(&[], 2);
        let real_g_rows = vec![vec![(0, 1.0), (1, 2.0)]];
        let c = vec![1.0, 1.0];
        let lb = vec![0.0, f64::NEG_INFINITY];
        let ub = vec![5.0, f64::INFINITY];

        let fixed = fix_dominated_columns(2, &a, &real_g_rows, &c, &lb, &ub);
        assert_eq!(fixed, vec![(0, 5.0)]);
    }

    #[test]
    fn does_nothing_when_neither_escape_route_is_open() {
        // Same pointwise relation, but both variables are finitely bounded
        // on the side that would need to be infinite -- no fix available,
        // only a partial tightening this pass intentionally leaves alone.
        let a = csr_from_rows(&[], 2);
        let real_g_rows = vec![vec![(0, 1.0), (1, 2.0)]];
        let c = vec![1.0, 1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![5.0, 5.0];

        let fixed = fix_dominated_columns(2, &a, &real_g_rows, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    #[test]
    fn leaves_a_non_dominated_pair_untouched() {
        // The two rows disagree about which column has the smaller
        // coefficient (row0: 1 vs 2; row1: 2 vs 1), so neither pointwise
        // comparison holds in either direction across the whole column.
        let a = csr_from_rows(&[], 2);
        let real_g_rows = vec![vec![(0, 1.0), (1, 2.0)], vec![(0, 2.0), (1, 1.0)]];
        let c = vec![1.0, 1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY];

        let fixed = fix_dominated_columns(2, &a, &real_g_rows, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    #[test]
    fn excludes_variables_appearing_in_any_equality_row() {
        // Same dominance relation as the first test, but x1 also appears
        // in an equality row -- disqualified, same as `dualfix`.
        let a = csr_from_rows(&[vec![(1, 1.0)]], 2);
        let real_g_rows = vec![vec![(0, 1.0), (1, 2.0)]];
        let c = vec![1.0, 1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY];

        let fixed = fix_dominated_columns(2, &a, &real_g_rows, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    #[test]
    fn never_reuses_a_committed_column_as_either_side_of_a_second_fix() {
        // x0 dominates both x1 and x2 on one shared row (coefficient 1 vs
        // 2 for each, equal cost). Only one of the two resulting fixes
        // should survive this single pass -- the other is left for the
        // next round, since committing both would pin x0 twice over.
        let a = csr_from_rows(&[], 3);
        let real_g_rows = vec![vec![(0, 1.0), (1, 2.0), (2, 2.0)]];
        let c = vec![1.0, 1.0, 1.0];
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY, f64::INFINITY];

        let fixed = fix_dominated_columns(3, &a, &real_g_rows, &c, &lb, &ub);
        assert_eq!(fixed.len(), 1, "exactly one of the two competing fixes should be committed this pass");
        assert!(fixed[0].0 == 1 || fixed[0].0 == 2);
        assert_eq!(fixed[0].1, 0.0);
    }
}
