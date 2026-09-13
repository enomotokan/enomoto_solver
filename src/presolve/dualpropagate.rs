//! DualPropagate: [`propagate`](super::propagate)'s own row-activity bound
//! tightening, run a second time on the *transpose* of the constraint
//! system, to derive implied bounds on each real row's own dual variable
//! (shadow price) — and, from those, to recognize when an inequality row
//! is *provably tight in every optimal solution*, even though nothing in
//! the primal system alone (its own activity range, `propagate`'s own
//! §3.1 forcing-row check) would show that.
//!
//! ## Why this exists
//!
//! `colsingleton`'s own docs note that a column singleton's *inequality*
//! case "needs a sign-based case analysis of whether the row is guaranteed
//! to bind" and is deferred — and a naive per-column version of that case
//! analysis (checking only that one column's own objective sign against
//! its one row) turns out to require a convex epigraph reformulation that
//! *grows* the problem rather than shrinking it (a variable+row swapped
//! for a variable+two rows), because a single column's own cost sign says
//! nothing about whether the *row* itself must bind. HiGHS's own
//! `HPresolve.cpp` (`isDualImpliedFree`, `updateRowDualImpliedBounds`)
//! resolves this the sound way: instead of asking one column in isolation,
//! it propagates the *entire* dual feasibility system (every column's own
//! reduced-cost identity, across every row that column touches) and asks
//! whether *that* proves the row's dual variable can never be zero. This
//! module is that same idea, implemented by reusing this crate's existing
//! primal propagation code on a transposed problem instead of writing a
//! second, parallel bound-tightening algorithm from scratch.
//!
//! ## The mechanics
//!
//! For `minimize c^T x` subject to `A x = b` (dual `lambda`, free sign)
//! and `G x <= h` (dual `mu >= 0`), stationarity for column `j` reads
//! (writing `d_i` for whichever of `lambda_i`/`mu_i` multiplies row `i`,
//! and `M_ij` for that row's own coefficient on column `j`, exactly as
//! stored — no sign flip needed since every row here is already in `<=`
//! form):
//!
//!   r_j := c_j + sum_i d_i * M_ij
//!
//! and complementary slackness ties `r_j`'s sign to which of `x_j`'s own
//! bounds is active: `r_j >= 0` is consistent with `x_j` sitting at a
//! finite lower bound, `r_j <= 0` with a finite upper bound, and (since a
//! variable can never sit at a bound that doesn't exist) an *infinite*
//! bound on one side rules out the matching sign outright:
//!
//!   - `ub_j = +inf` => `r_j` can never be negative => `r_j >= 0` forced
//!     => `sum_i d_i * M_ij >= -c_j` (a lower bound on that weighted sum).
//!   - `lb_j = -inf` => `r_j` can never be positive => `r_j <= 0` forced
//!     => `sum_i d_i * M_ij <= -c_j` (an upper bound on that same sum).
//!   - both finite: neither sign is ruled out, so column `j` contributes
//!     nothing usable — skipped outright, the same way `propagate`'s own
//!     activity sums skip a term that can't tighten anything.
//!   - both infinite (a genuinely free `x_j`): *both* rules apply at once,
//!     pinning `sum_i d_i * M_ij` to *exactly* `-c_j` — the classic "free
//!     column's reduced cost is zero" fact, expressed here as an equality
//!     (represented, like every two-sided bound elsewhere in this crate,
//!     as two opposing `<=` rows rather than a separate equality system).
//!
//! Each column `j` with at least one infinite bound therefore contributes
//! one (or, if fully free, two) row(s) to a *new* `<=` system whose own
//! "variables" are the original problem's row duals `d_i` — exactly
//! [`propagate::propagate`]'s own input shape, just built from the
//! transpose. `G`-row duals additionally get their sign-constraint row
//! (`-mu_i <= 0`, i.e. `mu_i >= 0`, folded in as a box row exactly like
//! [`build_a_g`](super::build_a_g) folds a variable's own bound); `A`-row
//! duals get none (free sign, matching a variable with no box row at all
//! under [`propagate::extract_bounds`]'s "no row => infinite" convention).
//! Running [`propagate::propagate`] on this transposed system tightens
//! each `mu_i`'s own `[lb, ub]` the same way it would tighten any
//! primal variable's — and a `G`-row whose tightened `mu_i` lower bound
//! comes back strictly positive can *never* have `mu_i = 0` in any dual-
//! feasible point, so by complementary slackness that row must be tight
//! (`(Gx)_i = h_i`) at *every* primal optimum: a real, provable implied
//! equality, safe to migrate into the `A` system outright and pick up
//! everything `doubleton`/`colsingleton`/`rowsingleton` already know how
//! to do with a true equality — no new substitution logic needed.
//!
//! **"Infinite" means *strictly implied*, not just literally `+/-inf`**:
//! a column's bound only rules out one of `r_j`'s signs when `x_j` can
//! *genuinely never* sit there — true not only for a literal `+/-inf`
//! bound, but for any bound `propagate`'s own §3.2 activity tightening
//! has since proven strictly *tighter* than (that original bound is then
//! provably unreachable too, exactly HiGHS's own `isLowerStrictlyImplied`/
//! `isUpperStrictlyImplied` distinction between a column's *explicit*
//! bound and its separately-tracked *implied* one). This crate's
//! `propagate`/`dualfix`/`colsingleton` overwrite `lb`/`ub` in place
//! rather than keeping that distinction as a separate pair the way HiGHS
//! does, but the same fact is recoverable without extra bookkeeping:
//! since every tightening pass only ever narrows a bound, never widens
//! it, `ub[j] < orig_ub[j]` (the model's own, never-tightened value,
//! captured once as a frozen `orig_lb`/`orig_ub` pair before
//! `run_extended`'s round loop starts) already *means* something besides
//! `orig_ub[j]` proved tighter — so `orig_ub[j]` itself can never again be
//! `x_j`'s actual value, the same as if it had been `+inf` all along.
//! `lb[j] == ub[j]` (a column fixed outright — a genuine decision by
//! `dualfix`/`colsingleton`/`doubleton`, not a mere activity-derived
//! tightening) is excluded either way: a fixed column is a constant, not
//! a live unknown, and correctly contributes nothing regardless of what
//! its original bounds were.
//!
//! Single-pass, non-cascading, decided from the input snapshot (mirrors
//! every other pass in this pipeline): a row promoted to an implied
//! equality this call is picked up by `doubleton`/`colsingleton` starting
//! next round, not immediately re-examined within this same call.

use crate::presolve::propagate;
use crate::sparse::{csr_from_rows, Csr};

const TOL: f64 = 1e-9;

/// Returns the indices into `real_g_rows` of every row proven tight in
/// every optimal solution (an *implied* equality) by propagating the
/// dual feasibility system derived from `c`/`orig_lb`/`orig_ub` (see the
/// module docs for why the *original*, not the current round's, bounds)
/// and both `a` and `real_g_rows`'s own coefficients. `lb`/`ub` are the
/// current round's bounds, consulted only to recognize a column already
/// fixed to a point (`lb[j] == ub[j]`), which contributes nothing
/// regardless of its original bounds. `a`'s rows contribute their own
/// (always-free-sign) duals to the propagated system but are never
/// themselves returned — they are already part of the equality system
/// the caller maintains, with nothing left to promote.
pub fn find_implied_equalities(n: usize, a: &Csr, real_g_rows: &[Vec<(usize, f64)>], c: &[f64], lb: &[f64], ub: &[f64], orig_lb: &[f64], orig_ub: &[f64], passes: usize) -> Vec<usize> {
    let ar = a.as_ref();
    let num_a = ar.nrows();
    let num_g = real_g_rows.len();
    let num_duals = num_a + num_g;
    if num_duals == 0 {
        return Vec::new();
    }

    // Transpose: `col_terms[j]` collects every (dual-variable index, its
    // own coefficient on column `j`) pair, `A`'s rows numbered `0..num_a`
    // and `real_g_rows`'s numbered `num_a..num_a+num_g` right after them.
    let mut col_terms: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for i in 0..num_a {
        for (j, &v) in ar.col_indices_of_row(i).zip(ar.values_of_row(i)) {
            if v != 0.0 {
                col_terms[j].push((i, v));
            }
        }
    }
    for (gi, row) in real_g_rows.iter().enumerate() {
        let i = num_a + gi;
        for &(j, v) in row {
            if v != 0.0 {
                col_terms[j].push((i, v));
            }
        }
    }

    let mut t_rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut t_h: Vec<f64> = Vec::new();

    for j in 0..n {
        if col_terms[j].is_empty() || lb[j] == ub[j] {
            continue;
        }
        // `ub_j` unreachable (literally `+inf`, or `propagate` has since
        // proven a strictly tighter bound than the model's own — see the
        // module docs) => `r_j` can never be negative => `r_j >= 0`
        // forced => `sum d_i*M_ij >= -c_j` => `-sum d_i*M_ij <= c_j`.
        if ub[j] == f64::INFINITY || orig_ub[j] > ub[j] + TOL {
            t_rows.push(col_terms[j].iter().map(|&(i, v)| (i, -v)).collect());
            t_h.push(c[j]);
        }
        // Symmetric case on the lower side.
        if lb[j] == f64::NEG_INFINITY || orig_lb[j] < lb[j] - TOL {
            t_rows.push(col_terms[j].clone());
            t_h.push(-c[j]);
        }
    }

    // Sign constraint on every `G`-row's own dual: `mu_i >= 0`, i.e.
    // `-mu_i <= 0` — folded in as a box row exactly like a variable's own
    // bound, per `build_a_g`'s convention. `A`-row duals get no such row
    // (free sign), matching `extract_bounds`'s "no row => infinite" rule.
    for gi in 0..num_g {
        t_rows.push(vec![(num_a + gi, -1.0)]);
        t_h.push(0.0);
    }

    let t_g = csr_from_rows(&t_rows, num_duals);
    let result = propagate::propagate(num_duals, &t_g, &t_h, passes);
    if result.infeasible {
        // A genuinely infeasible dual system here would mean the primal
        // is unbounded or infeasible outright — a real finding, but too
        // strong a conclusion to act on from this single, partial (box-
        // bound-driven) slice of the full dual system alone. Left for
        // whichever existing pass (`propagate` on the primal side, the
        // simplex/interior-point solve itself) is actually responsible
        // for that determination; this call simply finds nothing usable.
        return Vec::new();
    }

    (0..num_g).filter(|&gi| result.lb[num_a + gi] > TOL).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    #[test]
    fn free_hub_column_forces_its_only_inequality_tight() {
        // minimize -x1 (x1 free: [0,+inf)) s.t. x0 + x1 <= 10, x0 in
        // [0,10]. x1's own reduced cost must be 0 (cost -1, and x1 has no
        // upper bound so its dual-row is genuinely an equality: -1 + mu*1
        // = 0 => mu = 1 > 0), so the row is proven tight in every optimal
        // solution even though nothing about its own primal activity
        // range (x0+x1 can range from 0 to +inf, no §3.1 forcing-row
        // shape at all) would show that.
        let n = 2;
        let a = csr_from_rows(&[], n);
        let real_g_rows = vec![vec![(0usize, 1.0), (1usize, 1.0)]];
        let c = vec![0.0, -1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, f64::INFINITY];
        let implied = find_implied_equalities(n, &a, &real_g_rows, &c, &lb, &ub, &lb, &ub, 2);
        assert_eq!(implied, vec![0]);
    }

    #[test]
    fn bounded_hub_column_proves_nothing() {
        // Same shape, but x1 now has a finite upper bound too — neither
        // of its own bounds is infinite, so it contributes no dual
        // constraint at all, and nothing here can prove the row tight.
        let n = 2;
        let a = csr_from_rows(&[], n);
        let real_g_rows = vec![vec![(0usize, 1.0), (1usize, 1.0)]];
        let c = vec![0.0, -1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 10.0];
        let implied = find_implied_equalities(n, &a, &real_g_rows, &c, &lb, &ub, &lb, &ub, 2);
        assert!(implied.is_empty());
    }

    #[test]
    fn unfavorable_cost_sign_alone_proves_nothing() {
        // x1 is free ([0,+inf)) but its cost now favors making the row
        // *slack* (c1 = +1, minimizing wants x1 small, i.e. 0), so mu is
        // forced to 0, not away from it — dualfix's own favorable-sign
        // case would already fix x1 = 0 upstream of this pass; here,
        // taken in isolation, this pass must correctly find nothing.
        let n = 2;
        let a = csr_from_rows(&[], n);
        let real_g_rows = vec![vec![(0usize, 1.0), (1usize, 1.0)]];
        let c = vec![0.0, 1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, f64::INFINITY];
        let implied = find_implied_equalities(n, &a, &real_g_rows, &c, &lb, &ub, &lb, &ub, 2);
        assert!(implied.is_empty());
    }

    #[test]
    fn two_free_columns_sharing_a_row_still_forces_it() {
        // Neither column alone has a *strictly* one-sided-favorable cost
        // (both are free, i.e. both bounds infinite on the unconstrained
        // side, each pinning the row's dual to its own exact cost value)
        // — the row is forced tight regardless, and both free columns'
        // own equalities must be mutually consistent with the same mu.
        let n = 2;
        let a = csr_from_rows(&[], n);
        let real_g_rows = vec![vec![(0usize, 1.0), (1usize, 1.0)]];
        let c = vec![-2.0, -2.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY];
        let implied = find_implied_equalities(n, &a, &real_g_rows, &c, &lb, &ub, &lb, &ub, 2);
        assert_eq!(implied, vec![0]);
    }

    #[test]
    fn no_real_rows_is_a_no_op() {
        let n = 1;
        let a = csr_from_rows(&[], n);
        let real_g_rows: Vec<Vec<(usize, f64)>> = Vec::new();
        let c = vec![0.0];
        let lb = vec![0.0];
        let ub = vec![1.0];
        assert!(find_implied_equalities(n, &a, &real_g_rows, &c, &lb, &ub, &lb, &ub, 2).is_empty());
    }
}
