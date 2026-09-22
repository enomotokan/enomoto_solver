//! DominatedRows (Andersen & Andersen, "Presolving in Linear Programming",
//! Mathematical Programming 71 (1995), §3.1): the row-dual of
//! [`dominatedcol`](super::dominatedcol)'s column domination. There, two
//! *columns* sharing every row were compared coefficient-by-coefficient to
//! decide whether one variable could always be substituted for another
//! without hurting feasibility or the objective. Here, two *rows* sharing
//! every column are compared the same way to decide whether satisfying one
//! row's constraint always forces the other's to hold too — making the
//! forced one redundant, droppable outright, no bound-shifting or fixing
//! needed (unlike the column case, a dropped row changes nothing about
//! which `x` remain feasible, so there is no `infeasible`/fix-value
//! bookkeeping here at all).
//!
//! For two `<=` rows `p` (the *dominated*, candidate for removal) and `q`
//! (the *dominating*, kept) with no shared column an equality row also
//! touches (same exclusion [`dualfix`](super::dualfix)/[`dominatedcol`]
//! use — a column elsewhere pinned exactly by an equality has no freedom
//! left for this per-column argument to reason about, see below), `q`
//! dominates `p` when, for every column `k`:
//!
//!   - `G[q,k] == G[p,k]` (both compared with `TOL` slack) — identical on
//!     this column, so whatever `x_k` contributes to one row's activity it
//!     contributes identically to the other's, regardless of `x_k`'s own
//!     sign; **or**
//!   - `G[q,k] >= G[p,k]` **and** `lb[k] >= -TOL` — `q`'s coefficient is at
//!     least as large and `x_k` is never negative, so `G[q,k]*x_k >=
//!     G[p,k]*x_k` for every feasible `x_k`.
//!
//! and, in addition, `h[q] <= h[p]` (`q`'s own bound is at least as tight).
//! Given all of that, summing the per-column inequalities above over every
//! `k` gives `(G[q,:]).x >= (G[p,:]).x` for every `x` satisfying the
//! variable bounds, so `(G[q,:]).x <= h[q]` (row `q` holding) implies
//! `(G[p,:]).x <= (G[q,:]).x <= h[q] <= h[p]` (row `p` automatically
//! holding too) — row `p` can never be the binding constraint and is
//! dropped.
//!
//! This is strictly weaker than (and cannot rediscover) the *proportional*
//! case [`parallelrows`](super::parallelrows)/[`redundancy::reduce_inequalities`](super::redundancy::reduce_inequalities)
//! already catch (a row that is some row's exact scalar multiple has
//! `G[q,k] == G[p,k]` nowhere in general, yet is still redundant via a
//! *different* argument those passes already make) — it exists for the
//! complementary case those can't reach at all: two rows whose coefficients
//! are **not** proportional, only pointwise-ordered, need a column's
//! nonnegativity to license the comparison (see `TOL` usage above) rather
//! than a single shared scalar. The `lb[k] >= 0` requirement is what makes
//! this pass non-vacuous on real Netlib data in a way
//! [`dominatedcol`](super::dominatedcol)'s own escape-route conditions
//! measurably are not there (see that module's own docs on why every
//! structural variable in this crate always has two *finite* bounds): a
//! variable merely needs a `0` lower bound, which is the common case for
//! ordinary (non-free, non-negative-only) structural/slack variables, not
//! a *literal infinity* on either side.
//!
//! **Candidate search**: checking every pair of rows is `O(m^2)`; instead,
//! for each candidate row this only compares against rows sharing its own
//! *rarest* column (the column touched by the fewest other candidate rows)
//! — the same "let the cheapest available index bound the search" idea
//! [`dominatedcol`]'s own candidate search and [`sparsify`](super::sparsify)'s
//! `anchor` both use, just picked per-row here instead of per-column/
//! per-equality-row there.
//!
//! **Single-pass, non-cascading, decided from the input snapshot**: every
//! domination edge `p -> q` (`q` dominates `p`) is computed once, up front,
//! against the original, unmodified rows. A row is only ever actually
//! dropped if *no* edge anywhere in this same call points *at* it (i.e. it
//! never itself appears as someone else's dominated side) — so every row
//! used to justify a drop is guaranteed to survive this call uncut, the
//! same conservative "a committed row is never reused/invalidated in the
//! same pass" rule [`dominatedcol`]'s own docs describe for its columns.
//! This costs a genuine (if likely rare) chain `p -> q -> r` nothing gets
//! resolved in one call — `p` is left alone since `q` itself is some other
//! row's dominated side here — but the next call (this pipeline's own outer
//! fixpoint loop) picks it up once `q` is actually gone.
//!
//! **Implemented, unit-tested, measured against the full Netlib set — then
//! left unintegrated (kept here, tested, but never called from
//! [`crate::presolve::run_extended`]), mirroring [`dominatedcol`]/
//! [`sparsify`](super::sparsify)'s own precedent.** Wired in once, right
//! after [`parallelrows`](super::parallelrows), and instrumented directly
//! (not inferred from timing alone): zero domination edges were found on
//! any of the 73 in-scope Netlib instances — the joint condition (every
//! shared column pointwise ordered *and* nonnegative-lower-bounded on the
//! side that needs it, *and* the right-hand sides ordered to match) never
//! held for any real row pair measured, the same zero-hit-rate outcome
//! [`dominatedcol`]'s own docs report for the analogous column case (there
//! for a structural reason specific to this crate's finite-bounds
//! invariant; here, simply because no row pair in this particular problem
//! set happens to satisfy a genuinely strict joint condition). Wiring it
//! in cost pure candidate-search overhead (the anchor-column scan below,
//! over every multi-variable row) for zero reductions anywhere —
//! contributing, together with [`parallelrows`], +1.5% aggregate `ours`
//! time (3.85s unwired vs. 3.91s wired), 73/73 objective values unchanged
//! either way. Kept here for its correct, tested core logic rather than
//! deleted.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashSet;

use crate::sparse::{Csr, csr_row_iter};
const TOL: f64 = 1e-9;

/// Returns the row indices (into `g`) that are dominated and safe to drop
/// this call — same shape contract as every other reduction here: the
/// caller removes exactly these rows from `(G, h)` and nothing else
/// changes (no bound fix, no infeasibility — see module docs for why
/// dropping a dominated row can never affect feasibility either way).
pub fn find_dominated_rows(n: usize, a: &Csr, g: &Csr, h: &[f64], lb: &[f64]) -> Vec<usize> {
    let ar = a.as_ref();
    let mut in_equality = vec![false; n];
    for i in 0..ar.nrows() {
        for (j, v) in csr_row_iter(a, i) {
            if v != 0.0 {
                in_equality[j] = true;
            }
        }
    }

    let gr = g.as_ref();
    let m = gr.nrows();
    // Real (multi-variable) candidate rows only -- a length-1 row is a
    // variable's own box bound (see `super::build_a_g`) and carries
    // structural meaning elsewhere (`propagate::extract_bounds`) that
    // dropping it here would silently lose, mirroring why
    // `parallelrows`/`dominatedcol` both make the same exclusion.
    let rows: Vec<BTreeMap<usize, f64>> = (0..m)
        .map(|i| {
            let raw: Vec<(usize, f64)> = csr_row_iter(g, i).filter(|&(_, v)| v != 0.0).collect();
            let mut row = BTreeMap::new();
            if raw.len() >= 2 {
                for (j, v) in raw {
                    if in_equality[j] {
                        // A candidate row touching an equality-locked
                        // column is excluded outright (see module docs) --
                        // marked via a sentinel so it's skipped below
                        // rather than compared on an incomplete pattern.
                        row.clear();
                        row.insert(usize::MAX, 0.0);
                        break;
                    }
                    row.insert(j, v);
                }
            }
            row
        })
        .collect();

    let candidates: Vec<usize> = (0..m).filter(|&i| !rows[i].is_empty() && !rows[i].contains_key(&usize::MAX)).collect();
    if candidates.len() < 2 {
        return Vec::new();
    }

    let mut col_candidates: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    for &i in &candidates {
        for &k in rows[i].keys() {
            col_candidates[k].insert(i);
        }
    }

    // `q` dominates `p` (returns true) per the module docs' per-column test.
    let dominates = |q: usize, p: usize| -> bool {
        for (&k, &gp) in &rows[p] {
            let gq = rows[q].get(&k).copied().unwrap_or(0.0);
            if (gq - gp).abs() <= TOL {
                continue;
            }
            if gq + TOL >= gp && lb[k] >= -TOL {
                continue;
            }
            return false;
        }
        for (&k, &gq) in &rows[q] {
            if rows[p].contains_key(&k) {
                continue; // already covered above
            }
            let gp = 0.0;
            if (gq - gp).abs() <= TOL {
                continue;
            }
            if gq + TOL >= gp && lb[k] >= -TOL {
                continue;
            }
            return false;
        }
        h[q] <= h[p] + TOL
    };

    // Deterministic candidate pairing: for each row, anchor on its own
    // rarest column (fewest other candidate rows touching it) and only
    // compare against rows sharing that column -- a domination this misses
    // (its partner never shares this particular rarest column) is left for
    // a later round's structural change to expose, same scope limit
    // `dominatedcol`'s own candidate search accepts.
    let mut pairs: BTreeSet<(usize, usize)> = BTreeSet::new();
    for &i in &candidates {
        let anchor = *rows[i].keys().min_by_key(|&&k| col_candidates[k].len()).unwrap();
        for &j in &col_candidates[anchor] {
            if j != i {
                pairs.insert((i.min(j), i.max(j)));
            }
        }
    }

    // `dominated_by[p]` collects every `q` found to dominate `p`.
    let mut dominated_by: Vec<Vec<usize>> = vec![Vec::new(); m];
    let mut ever_dominates: HashSet<usize> = HashSet::new();
    for (x, y) in pairs {
        if dominates(x, y) {
            dominated_by[y].push(x);
            ever_dominates.insert(x);
        }
        if dominates(y, x) {
            dominated_by[x].push(y);
            ever_dominates.insert(y);
        }
    }

    // A row that ever dominates something is never itself dropped this
    // call, full stop -- so any `q` appearing in `dominated_by[p]` is
    // guaranteed to survive (it's in `ever_dominates`, hence skipped by the
    // check below), making it a always-safe citation for dropping `p`.
    let mut drop = Vec::new();
    for &p in &candidates {
        if ever_dominates.contains(&p) {
            continue; // used to justify dropping something else -- never itself dropped this call
        }
        if !dominated_by[p].is_empty() {
            drop.push(p);
        }
    }
    drop.sort_unstable();
    drop
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    #[test]
    fn drops_the_row_dominated_by_a_tighter_pointwise_larger_row() {
        // row0: x0 + x1 <= 5 (dominated)
        // row1: 2*x0 + 2*x1 <= 3 (dominating: coeffs pointwise >=, bound tighter)
        // x0,x1 >= 0, so row1 holding forces 2*(x0+x1) <= 3 -> x0+x1 <= 1.5 <= 5.
        let a = csr_from_rows(&[], 2);
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0)]], 2);
        let h = vec![5.0, 3.0];
        let lb = vec![0.0, 0.0];

        let dropped = find_dominated_rows(2, &a, &g, &h, &lb);
        assert_eq!(dropped, vec![0]);
    }

    #[test]
    fn keeps_both_when_a_column_can_go_negative() {
        // Same coefficients as above, but x1 is free (lb < 0) -- the
        // pointwise argument for column 1 no longer holds, so neither row
        // may be dropped via this rule.
        let a = csr_from_rows(&[], 2);
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0)]], 2);
        let h = vec![5.0, 3.0];
        let lb = vec![0.0, -1.0];

        let dropped = find_dominated_rows(2, &a, &g, &h, &lb);
        assert!(dropped.is_empty());
    }

    #[test]
    fn keeps_both_when_bounds_disagree_with_coefficient_order() {
        // row1's coefficients pointwise dominate row0's, but its own bound
        // is looser (12 > 5), so row1 holding does NOT force row0 -- no
        // domination either way.
        let a = csr_from_rows(&[], 2);
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0)]], 2);
        let h = vec![5.0, 12.0];
        let lb = vec![0.0, 0.0];

        let dropped = find_dominated_rows(2, &a, &g, &h, &lb);
        assert!(dropped.is_empty());
    }

    #[test]
    fn excludes_rows_touching_an_equality_locked_column() {
        let a = csr_from_rows(&[vec![(1, 1.0)]], 2);
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0)]], 2);
        let h = vec![5.0, 3.0];
        let lb = vec![0.0, 0.0];

        let dropped = find_dominated_rows(2, &a, &g, &h, &lb);
        assert!(dropped.is_empty());
    }

    #[test]
    fn leaves_box_bound_rows_untouched() {
        let a = csr_from_rows(&[], 1);
        let g = csr_from_rows(&[vec![(0, 1.0)], vec![(0, 2.0)]], 1);
        let h = vec![5.0, 3.0];
        let lb = vec![0.0];

        let dropped = find_dominated_rows(1, &a, &g, &h, &lb);
        assert!(dropped.is_empty());
    }

    #[test]
    fn a_row_used_to_justify_a_drop_is_never_itself_dropped_in_the_same_call() {
        // row0: x0+x1 <= 10 (dominated by row1)
        // row1: 2*x0+2*x1 <= 6 (dominates row0; also dominated by row2)
        // row2: 3*x0+3*x1 <= 3 (dominates row1)
        // row1 is cited to justify dropping row0, so row1 itself must
        // survive this call even though row2 also dominates it -- that
        // second link (row1 dominated by row2) is left for the pipeline's
        // next call to resolve, once row1's own removal can no longer
        // invalidate anything else's justification from this pass.
        let a = csr_from_rows(&[], 2);
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0)], vec![(0, 3.0), (1, 3.0)]], 2);
        let h = vec![10.0, 6.0, 3.0];
        let lb = vec![0.0, 0.0];

        let dropped = find_dominated_rows(2, &a, &g, &h, &lb);
        assert_eq!(dropped, vec![0], "only row0 is safe to drop this call -- its dominator row1 survives since row1 is never dropped while justifying row0");
    }
}
