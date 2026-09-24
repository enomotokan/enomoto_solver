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
//! **"Infinite" means literally `+/-inf`, not merely tighter than the
//! model's own bound.** An earlier version of this module also fired a
//! column's t-row whenever `propagate`'s own activity tightening had
//! proven a bound strictly inside the model's original one (mirroring
//! HiGHS's `isLowerStrictlyImplied`/`isUpperStrictlyImplied`), on the
//! theory that `x_j` could then never reach that original bound either.
//! That theory silently assumed the row which justified the tightening
//! was still part of the row set (`real_g_rows`/`a`) this function is
//! handed — but `propagate` itself deletes a row as redundant right
//! after using it to tighten a bound, so the assumption frequently
//! doesn't hold, and the resulting t-row can be false in every optimal
//! dual solution (found on Netlib `80bau3b`: it fixed 106 columns off a
//! dual box with no actual optimal point, moving the objective by
//! +1531). `orig_lb`/`orig_ub` (the model's own, never-tightened bounds,
//! captured once before `run_extended`'s round loop starts) are kept as
//! parameters for whichever future fix re-derives this reduction with
//! the row/bound bookkeeping it actually needs, but this module no
//! longer reads them.
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
//!
//! ## Column fixing (HiGHS's "dominated column", reached this module's way)
//!
//! The same propagated `[dlo_i, dhi_i]` box on every real row's dual `d_i`
//! that [`run`] reads out for row promotion also bounds every column's own
//! reduced cost `r_j = c_j + sum_i d_i * M_ij`, by plain interval
//! arithmetic over `M_ij`'s already-collected `col_terms[j]` entries — not
//! just the columns whose own infinite bound helped build the box in the
//! first place, *any* column sharing a row with one of those. The same
//! box-constrained-Lagrangian argument the module docs above use to derive
//! `r_j`'s forced sign for an infinite-bound column (minimizing
//! `c^T x + d^T(\text{row terms})` over a box `[lb,ub]` independently per
//! coordinate: `r_j > 0` forces `x_j` down to `lb_j`, `r_j < 0` up to
//! `ub_j`) applies unconditionally, since it only assumes `d` is *some*
//! dual-optimal value — which the propagated box always contains, no
//! matter which of its own bounds happen to be finite. So: if the box's
//! own worst case still leaves `r_j` strictly one-signed (`rlo_j > 0` or
//! `rhi_j < 0`, computed by interval arithmetic over the box), that sign
//! holds at the *true* optimal `d` too, and the column can be fixed
//! outright — reading out the same fixed point [`run`] already computed,
//! no extra propagation pass. This is HiGHS's `HPresolve.cpp` "dominated
//! column" reduction (`impliedDualRowBounds` feeding `isDominatedCol`),
//! *not* Andersen & Andersen's column-vs-column comparison
//! ([`super::dominatedcol`], a different, complementary technique reached
//! from an entirely different angle) — this crate's own name collision is
//! coincidental, not a claim the two modules do the same thing.
//!
//! This is exactly what closes the gap this crate's own presolve leaves on
//! network-shaped models dense with equality-row column singletons (e.g.
//! Netlib's `seba`: HiGHS reduces it to 2 rows / 8 columns, largely via
//! this reduction's own long fix/substitute cascade — see this crate's
//! project memory for the full investigation): such a column's cost-0,
//! one-sided-bound "slack" is exactly the case
//! [`find_implied_equalities`]'s own infinite-bound test already builds a
//! dual-sign constraint from, but the *box* variable sharing that same
//! equality row (finite on both sides, so invisible to that test's own
//! column selection) is precisely the kind of column only this read-out
//! can fix — `dualfix`'s simple lock-counting can't reach it either,
//! since it explicitly disqualifies any column touching an equality row.
//! Once fixed, the row shrinks by one variable, which is exactly the
//! "row now short enough to be implied-free" condition `aggregator`
//! needs to fire, cascading into everything downstream that already knows
//! what to do with a fixed column or a shorter equality row (no new
//! substitution logic needed here either).

use crate::presolve::propagate;
use crate::sparse::{Csr, CscMat, csr_from_rows, csr_row_iter};
use crate::params::presolve::TOL;

/// Bundles both reductions [`run`] reads out of one dual-feasibility
/// propagation pass: `implied_equalities` (indices into `real_g_rows`
/// proven tight in every optimum — see the module docs) and
/// `fixed_columns` (`(column index, bound value)` pairs — always one of
/// that column's own `lb[j]`/`ub[j]`, whichever side its reduced cost is
/// proven pinned to; see the module docs' "Column fixing" section).
#[derive(Default)]
pub struct DualReductions {
    pub implied_equalities: Vec<usize>,
    pub fixed_columns: Vec<(usize, f64)>,
}

/// Propagates the dual feasibility system derived from `c` and both `a`
/// and `real_g_rows`'s own coefficients, and reads out both reductions
/// the resulting propagated dual box proves — see [`DualReductions`] and
/// the module docs' "Column fixing" section for the second half. `lb`/
/// `ub` are the current round's bounds, consulted to recognize a column
/// already fixed to a point (`lb[j] == ub[j]`), which contributes nothing
/// to the propagated system and is never a candidate for (re-)fixing
/// here, and to test literal `+/-inf`. `a`'s rows contribute their own
/// (always-free-sign) duals to the propagated system but are never
/// themselves candidates for row promotion — they are already part of
/// the equality system the caller maintains, with nothing left to
/// promote. `orig_lb`/`orig_ub` are unused (see the module docs' "means
/// literally `+/-inf`" note) and kept only so callers don't need to
/// change; a future fix may need them again.
pub fn run(n: usize, a: &Csr, real_g_rows: &[Vec<(usize, f64)>], c: &[f64], lb: &[f64], ub: &[f64], _orig_lb: &[f64], _orig_ub: &[f64], passes: usize) -> DualReductions {
    let ar = a.as_ref();
    let num_a = ar.nrows();
    let num_g = real_g_rows.len();
    let num_duals = num_a + num_g;
    if num_duals == 0 {
        return DualReductions::default();
    }

    // Transpose: `col_terms.col(j)` holds every (dual-variable index, its
    // own coefficient on column `j`) pair, `A`'s rows numbered `0..num_a`
    // and `real_g_rows`'s numbered `num_a..num_a+num_g` right after them.
    // Streamed straight into the compressed column form (two allocations,
    // one counting sort) rather than `n` growable per-column `Vec`s — see
    // `CscMat::from_entry_stream`'s own docs; the two row blocks never have
    // to be concatenated first.
    let col_terms = CscMat::from_entry_stream(num_duals, n, |emit| {
        for i in 0..num_a {
            for (j, v) in csr_row_iter(a, i) {
                if v != 0.0 {
                    emit(i, j, v);
                }
            }
        }
        for (gi, row) in real_g_rows.iter().enumerate() {
            for &(j, v) in row {
                if v != 0.0 {
                    emit(num_a + gi, j, v);
                }
            }
        }
    });

    let mut t_rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut t_h: Vec<f64> = Vec::new();

    for j in 0..n {
        if col_terms.col(j).is_empty() || lb[j] == ub[j] {
            continue;
        }
        // `ub_j` unreachable (literally `+inf`) => `r_j` can never be
        // negative => `r_j >= 0` forced => `sum d_i*M_ij >= -c_j` =>
        // `-sum d_i*M_ij <= c_j`.
        //
        // Deliberately *not* also firing on `orig_ub[j] > ub[j] + TOL`
        // (a bound `propagate` tightened below the model's own): that
        // reasoning silently assumed the row that justified the
        // tightening is still part of `real_g_rows`/`a` by the time this
        // runs, but `propagate` itself drops a row as redundant right
        // after using it to tighten a bound (`propagate.rs`'s own
        // redundant-row elimination) — so the row whose dual this t-row
        // would need is frequently already gone, making the t-row's
        // implicit "no other constraint keeps `x_j` off `orig_ub[j]`"
        // premise false in the *current* row set and unsound in general
        // (confirmed on Netlib `80bau3b`: it fixed 106 columns off a
        // dual box containing no actual optimal dual solution, moving
        // the objective by +1531).
        if ub[j] == f64::INFINITY {
            t_rows.push(col_terms.col(j).iter().map(|&(i, v)| (i, -v)).collect());
            t_h.push(c[j]);
        }
        // Symmetric case on the lower side.
        if lb[j] == f64::NEG_INFINITY {
            t_rows.push(col_terms.col(j).to_vec());
            t_h.push(-c[j]);
        }
    }

    // Sign constraint on every `G`-row's own dual: `mu_i >= 0`, i.e.
    // `-mu_i <= 0` — folded in as a box row exactly like a variable's own
    // bound, per `build_a_g`'s convention. `A`-row duals get no such row
    // (free sign), matching `extract_bounds`'s "no row => infinite" rule.
    //
    // No column-derived row at all (no column with a literally infinite
    // bound): the system is only those sign rows, which `propagate` would
    // read straight back as the dual box (`-mu_i <= 0` -> `lb = 0.0 /
    // -1.0`, everything else unbounded) without any propagation — built
    // directly here instead of via the CSR build + `propagate` round trip.
    let result = if t_rows.is_empty() {
        let mut lb = vec![f64::NEG_INFINITY; num_duals];
        for gi in 0..num_g {
            lb[num_a + gi] = 0.0 / -1.0;
        }
        propagate::PropagateSplit { lb, ub: vec![f64::INFINITY; num_duals], real_rows: Vec::new(), real_rhs: Vec::new(), infeasible: false }
    } else {
        for gi in 0..num_g {
            t_rows.push(vec![(num_a + gi, -1.0)]);
            t_h.push(0.0);
        }
        let t_g = csr_from_rows(&t_rows, num_duals);
        propagate::propagate_nog(num_duals, &t_g, &t_h, passes)
    };
    if result.infeasible {
        // A genuinely infeasible dual system here would mean the primal
        // is unbounded or infeasible outright — a real finding, but too
        // strong a conclusion to act on from this single, partial (box-
        // bound-driven) slice of the full dual system alone. Left for
        // whichever existing pass (`propagate` on the primal side, the
        // simplex/interior-point solve itself) is actually responsible
        // for that determination; this call simply finds nothing usable.
        return DualReductions::default();
    }

    let implied_equalities = (0..num_g).filter(|&gi| result.lb[num_a + gi] > TOL).collect();

    // Column fixing (see the module docs' "Column fixing" section): for
    // every column with at least one real-row appearance and not already
    // fixed to a point, compute `r_j`'s range over the just-propagated
    // dual box by plain interval arithmetic on `col_terms[j]` — reusing
    // the same fixed point `implied_equalities` just read from, not a
    // second propagation pass. `v > 0.0`'s branch pairs `dlo_i` with the
    // range's low end and `dhi_i` with its high end (a positive
    // coefficient preserves order); `v < 0.0` flips both pairings (matches
    // interval multiplication by a negative scalar) — `col_terms` entries
    // are never zero (filtered when built above), so no third case.
    let mut fixed_columns = Vec::new();
    for j in 0..n {
        let terms = col_terms.col(j);
        if terms.is_empty() || lb[j] == ub[j] {
            continue;
        }
        let mut rlo = c[j];
        let mut rhi = c[j];
        for &(i, v) in terms {
            let (dlo, dhi) = (result.lb[i], result.ub[i]);
            let (tlo, thi) = if v > 0.0 { (v * dlo, v * dhi) } else { (v * dhi, v * dlo) };
            rlo += tlo;
            rhi += thi;
        }
        if rlo.is_nan() || rhi.is_nan() {
            // Only reachable if the propagated box itself is degenerate in
            // a way `result.infeasible` didn't already catch (e.g. an
            // `inf - inf` cancellation across two terms) — treat as
            // "nothing proven" rather than risk acting on a bogus sign.
            continue;
        }
        // `r_j > 0` everywhere the box allows => forced to `lb_j` (needs
        // `lb_j` finite: an infinite one can never be "fixed" to, and by
        // the same argument this module's row-promotion half already
        // relies on, a genuinely infinite `lb_j` would instead have
        // contributed its *own* `r_j <= 0` constraint above, making
        // `rlo > TOL` here self-contradictory in practice).
        if rlo > TOL && lb[j] > f64::NEG_INFINITY {
            fixed_columns.push((j, lb[j]));
        } else if rhi < -TOL && ub[j] < f64::INFINITY {
            fixed_columns.push((j, ub[j]));
        }
    }

    DualReductions { implied_equalities, fixed_columns }
}

/// Thin wrapper over [`run`] for callers wanting only the row-promotion
/// half (and this module's own tests, written before [`DualReductions`]
/// existed) — see [`run`]'s own docs for the shared derivation and the
/// module docs' "Column fixing" section for the other half.
pub fn find_implied_equalities(n: usize, a: &Csr, real_g_rows: &[Vec<(usize, f64)>], c: &[f64], lb: &[f64], ub: &[f64], orig_lb: &[f64], orig_ub: &[f64], passes: usize) -> Vec<usize> {
    run(n, a, real_g_rows, c, lb, ub, orig_lb, orig_ub, passes).implied_equalities
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
