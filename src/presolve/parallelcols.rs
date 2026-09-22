//! ParallelColumns (HiGHS's own name, bundled into its "Parallel rows and
//! columns" presolve-log category; Andersen & Andersen, "Presolving in
//! Linear Programming", Mathematical Programming 71 (1995), transpose
//! counterpart of the row-duplicate technique [`parallelrows`](super::parallelrows)
//! already implements): two columns whose constraint-matrix entries are
//! exact scalar multiples of one another across every real row they
//! appear in — `A[:,j] = s * A[:,k]` for a single nonzero scalar `s` — move
//! every row's own activity in that same fixed `s : 1` ratio no matter how
//! `x_j`/`x_k` are individually split, so *both* can be replaced by a
//! single merged variable `z = x_k + s*x_j`, occupying column `k`'s own
//! position with `k`'s own bounds widened to `z`'s reachable range. `j` is
//! dropped entirely — from every row and from the objective.
//!
//! ## When this is sound without any further case analysis
//!
//! Substituting `x_k = z - s*x_j` turns the pair's own objective
//! contribution `c_j*x_j + c_k*x_k` into `c_k*z + (c_j - s*c_k)*x_j` — a
//! term in `z` alone plus a residual term in `x_j` alone. When
//! `c_j == s*c_k` (checked with the same relative `TOL` every reduction in
//! this crate uses), that residual term vanishes identically: the
//! objective no longer depends on how `z` is actually split between
//! `x_j`/`x_k`, only on `z` itself, so *every* split respecting both
//! original box bounds is equally optimal. This module only ever merges in
//! that exact-proportional-cost case — the "objective strictly prefers one
//! over the other" case is [`dominatedcol`](super::dominatedcol)'s job (a
//! strict, `<=`-row-only relationship, deliberately excluding equality-row
//! columns — see that module's own docs), not duplicated here.
//!
//! Given that indifference, a valid split is derived purely algebraically
//! in [`Substitution::apply`] — it never depends on which particular
//! vertex the simplex method actually lands on, since every split is
//! equally optimal by construction. See that method's own docs for the
//! derivation (a clamp of `x_j` into its own original range from `z`'s
//! recovered value, general to either sign of `s`).
//!
//! ## Scope
//!
//! Every column's own *lower* bound must be finite, checked before `s` is
//! ever computed — that finite `lb` is the fixed anchor
//! [`Substitution::apply`]'s recovery formula clamps around (see its own
//! docs), so a lower-unbounded column (surplus-variable-shaped, `lb=-inf`)
//! is left untouched entirely — no mirrored upper-anchor formula is
//! implemented, since no Netlib instance in this crate's own benchmark set
//! has ever needed one (see the "Candidate search" section below for the
//! instance that *does* need the upper side left open). The *upper* bound,
//! by contrast, may be finite or genuinely infinite: a real Netlib
//! instance (`standgub`) has a 108-column GUB block that is exactly this
//! shape (`lb=0`, `ub=+inf`, `cost=0` on every member) and is otherwise
//! untouched by every other presolve pass in this pipeline. A column with
//! *both* sides infinite (genuinely free) is excluded outright — `lb`
//! already failing the finiteness check above catches it, so it's left for
//! [`super::freevar`] instead, matching that module's own scope. This
//! reduction otherwise never interacts with the free-variable / extended-
//! dual / `BIG_M` machinery `solve_lp_dual`'s own unbounded-structural
//! routing exists for (see its own docs): a `kept` column that already had
//! `ub=+inf` before any merge keeps exactly that same one-sided shape
//! after, just with a wider finite `lb` contribution folded in — nothing
//! about *which* side is unbounded ever changes. A column already fixed
//! (`lb == ub`, from an earlier reduction this same round, possibly not
//! yet folded out of `real_rows`/`a` by `foldfixed`) is skipped the same
//! way — merging into or out of a phantom fixed slot serves no purpose.
//!
//! **Implemented, unit-tested, measured against the full Netlib set —
//! regressed the aggregate 73-problem `ours` time 8.6% when first measured,
//! concentrated on the same degenerate/shape-sensitive instances every
//! other structural presolve extension in this crate's history has hit,
//! but turned on by default anyway (2026-09-21, `presolve.rs`'s own
//! `ENOMOTO_DISABLE_PARALLELCOLS` opt-out) to make forward progress on the
//! actual structural win** (`standgub`'s own 108-column GUB block shrinks
//! as expected) while leaving the regression itself as deliberately
//! deferred future work — see `presolve.rs`'s own call site for the full
//! measurement history, and the `parallelcols-regression-mechanism` memory
//! for what the regression actually turned out to be (mostly extra
//! `XB_DRIFT_REL_TOL`-triggered refactorizations, not more simplex iterations).
//! A separate correctness bug (a merge that could produce a genuinely free
//! column, degenerate enough after `simplex.rs`'s own free-variable split
//! to blow `extended_dual`'s iteration budget and fall back to a false
//! `Infeasible` via the unreliable classical `BIG_M` path) was found and
//! fixed first — see the merge loop's own comment below and the
//! `parallelcols-greenbea-false-infeasible` memory.
//!
//! ## Candidate search
//!
//! Mirrors `parallelrows`'s own signature-grouping trick, transposed: each
//! column is normalized by dividing every entry — across every real row it
//! appears in, `a`'s own rows plus `real_rows`'s multi-variable ones,
//! combined into one row-id space local to this call — by its own *signed*
//! first entry, so two columns proportional by *either* sign of scalar
//! land on the identical signature (exactly `parallelrows`'s own reasoning
//! for why it divides by the signed, not absolute, leading value). Within
//! one signature group, the lowest-indexed column becomes that group's
//! single merge target for this call (`kept`); every other eligible member
//! (cost-proportional too, both fully bounded, neither already claimed)
//! folds into it, in ascending index order, each with its own correctly-
//! chained [`Substitution`] — `kept_lb` is `kept`'s own lower bound at the
//! moment *that specific* member is folded in, capturing whatever
//! cumulative widening every earlier fold in the same call already
//! applied, so postsolve's reverse-discovery-order unwind (see
//! `Substitution::apply`'s own docs) peels a large group back apart one
//! layer at a time, in the opposite order it was folded together.
//!
//! Unlike `parallelrows` (one merge per call, full stop — its own docs
//! explain why: repeated calls across this pipeline's own outer-round
//! fixpoint loop pick up whatever a single call leaves behind), a single
//! call here can absorb an entire group at once. That is deliberate, not
//! merely an optimization: a real Netlib instance in this crate's own
//! benchmark set (`standgub`) has one 108-column parallel group (a GUB
//! block of genuinely interchangeable decision variables), and capping
//! this module at one merge per call would need as many outer rounds as a
//! group has members to fully collapse it — far more than `run_extended`'s
//! own round cap ever runs.

use crate::sparse::{Csr, CscMat, csr_rows};
const TOL: f64 = 1e-9;

/// Recovers both `x[var]` and `x[kept]`'s own true values from `x[kept]`'s
/// current, merged-`z` value — see the module docs for why any such split
/// is optimal, and why this can't be expressed as
/// [`super::colsingleton::Substitution`]'s single linear formula (that
/// writes to exactly one index from *other*, already-resolved ones; this
/// writes to *two*, one of which — `kept` — is also its own input).
pub struct Substitution {
    pub var: usize,
    pub kept: usize,
    pub s: f64,
    pub lb: f64,
    pub ub: f64,
    /// `kept`'s own lower bound at the moment this fold was performed —
    /// *not* necessarily `kept`'s original problem bound, when several
    /// merges chained onto the same `kept` in one call (see the module
    /// docs' "Candidate search" section).
    pub kept_lb: f64,
}

impl Substitution {
    /// `z = x[kept]` currently holds `x_kept_true + s * x[var]` (see the
    /// module docs for the derivation of `z`). Recovering the true split:
    /// `raw = (z - kept_lb) / s` is `var`'s own value *were `kept` sitting
    /// exactly at `kept_lb`* — always within `[lb, ub]` when `z` is
    /// genuinely reachable and unclamped, and clamping to whichever end of
    /// `[lb, ub]` `raw` overshoots is provably still feasible for `kept`
    /// (worked out in full, both signs of `s`, in this module's own
    /// commit/test history — `merges_and_recovers_*` below exercise every
    /// case): `x[kept] = z - s*x[var]` then lands back in `kept`'s own
    /// `[kept_lb, ...]` range by construction.
    pub fn apply(&self, x: &mut [f64]) {
        let z = x[self.kept];
        let raw = (z - self.kept_lb) / self.s;
        let xj = raw.clamp(self.lb, self.ub);
        x[self.var] = xj;
        x[self.kept] = z - self.s * xj;
    }
}

pub struct ParallelColsResult {
    pub a: Csr,
    pub c: Vec<f64>,
    pub lb: Vec<f64>,
    pub ub: Vec<f64>,
    pub real_rows: Vec<Vec<(usize, f64)>>,
    pub substitutions: Vec<Substitution>,
}

/// One non-cascading pass: candidates are found from the input snapshot
/// alone (mirrors every other pass in this pipeline) — a column that only
/// becomes parallel to another *after* this call's own folds take effect
/// is left for the next call (`run_extended`'s own outer-round fixpoint
/// loop already re-invokes every stage until nothing changes).
pub fn merge_parallel_columns(n: usize, a: &Csr, real_rows: &[Vec<(usize, f64)>], c: &[f64], lb: &[f64], ub: &[f64]) -> ParallelColsResult {
    let ar = a.as_ref();
    let n_a_rows = ar.nrows();
    let a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);

    // One combined row-id space, local to this call: `a`'s own rows first,
    // `real_rows`'s multi-variable ones after — meaningless outside this
    // function, but stable within it, which is all the signature grouping
    // below needs.
    //
    // Streamed straight into the compressed column form rather than `n`
    // growable per-column `Vec`s — see `CscMat::from_entry_stream`'s own
    // docs; the two row blocks never have to be concatenated first. Both
    // blocks are emitted in row order and `a`'s ids all precede
    // `real_rows`'s, so each column comes out already ascending by row id
    // — which is what the signature scan below needs, and what the
    // per-column `sort_unstable_by_key` it used to run was (redundantly)
    // producing.
    let columns = CscMat::from_entry_stream(n_a_rows + real_rows.len(), n, |emit| {
        for (i, row) in a_rows.iter().enumerate() {
            for &(j, v) in row {
                if v != 0.0 {
                    emit(i, j, v);
                }
            }
        }
        for (gi, row) in real_rows.iter().enumerate() {
            for &(j, v) in row {
                if v != 0.0 {
                    emit(n_a_rows + gi, j, v);
                }
            }
        }
    });

    let mut groups: std::collections::HashMap<Vec<(usize, u64)>, Vec<usize>> = std::collections::HashMap::new();
    for j in 0..n {
        let col = columns.col(j);
        // `lb[j]` must be finite (the fixed anchor `Substitution::apply`
        // clamps around — see the module docs' "Scope" section); `ub[j]`
        // may be finite or `+inf` (a one-sided-unbounded column, e.g. the
        // `standgub` GUB block those same docs describe, is still eligible
        // — only a *fully* free column, `lb=-inf`, fails this check).
        if col.is_empty() || !lb[j].is_finite() || (ub[j] - lb[j]).abs() < TOL {
            continue;
        }
        let inv = 1.0 / col[0].1;
        let sig: Vec<(usize, u64)> = col.iter().map(|&(row_id, v)| (row_id, (v * inv).to_bits())).collect();
        groups.entry(sig).or_default().push(j);
    }

    let mut group_keys: Vec<&Vec<usize>> = groups.values().collect();
    group_keys.sort_by_key(|v| v[0]);

    let mut used = vec![false; n];
    let mut new_lb = lb.to_vec();
    let mut new_ub = ub.to_vec();
    let mut new_c = c.to_vec();
    let mut substitutions = Vec::new();
    let mut eliminated: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();

    for members in group_keys {
        if members.len() < 2 {
            continue;
        }
        let mut members = members.clone();
        members.sort_unstable();
        let kept = members[0];
        if used[kept] {
            continue;
        }
        let kept_lead = columns.col(kept)[0].1;
        let mut cur_lb = new_lb[kept];
        let mut cur_ub = new_ub[kept];
        let mut any_merged = false;
        for &var in &members[1..] {
            if used[var] {
                continue;
            }
            let var_lead = columns.col(var)[0].1;
            let s = var_lead / kept_lead;
            let predicted_c_var = s * new_c[kept];
            let tol = TOL * (1.0 + new_c[var].abs().max(predicted_c_var.abs()));
            if (new_c[var] - predicted_c_var).abs() > tol {
                continue;
            }
            let s_lb = s * new_lb[var];
            let s_ub = s * new_ub[var];
            let (lo, hi) = (s_lb.min(s_ub), s_lb.max(s_ub));
            // `var`'s own candidacy only required *its* `lb` to be finite
            // (see this module's own "Scope" docs) — nothing there stops a
            // *negative* `s` (opposite-signed leading entry) paired with
            // `var`'s `ub = +inf` from making `lo` itself `-inf`, which
            // would push `kept`'s own merged lower bound to `-inf`, turning
            // it into a genuinely free (`lb=-inf` *and* `ub=+inf`, since
            // this exact shape's `ub` is already `+inf`) structural column.
            // `simplex.rs::build_std_form_presolved` *does* still handle
            // that correctly on its own — it splits any surviving doubly-
            // infinite column into `x_j = x_j^+ - x_j^-` (two `[0, inf)`
            // slots) before `extended_dual` ever sees it, exactly the
            // documented fallback for `presolve::freevar`'s own residual
            // case — so this is not unsound, just a shape nothing upstream
            // was ever exercised against. Measured directly on a real
            // Netlib instance (`greenbea`, eleven `s=-1`/`ub=+inf` pairs):
            // the split's own two halves are forced to occupy *exactly* the
            // same rows with exactly opposite coefficients (`x_j^+`/`x_j^-`
            // both M-flagged, perfectly anti-parallel by construction) —
            // apparently degenerate enough, stacked onto an already-large
            // M-flagged column count, to run `extended_dual`'s main loop
            // out of its own `MAX_ITERS` budget (confirmed via
            // `ENOMOTO_DEBUG_EXT_ITERS`: `DEBUG_EXT_BAILOUT: MAX_ITERS
            // exhausted`), which then falls back to the classical `BIG_M`
            // path (`solve_lp_dual`'s own documented "should be
            // unreachable" fallback) — and *that* path is the one that
            // actually reports the false `Infeasible` (see
            // [[bigm-fallback-invalid-reference]] memory: already known
            // unreliable independent of this module). Skip this particular
            // fold outright rather than manufacture that worst-case shape;
            // `var` is simply left unmerged (eligible for a future call
            // once its own shape changes, same as any other unpicked
            // candidate) — cheaper than teaching either solver path to cope
            // with an adversarially anti-parallel column pair.
            if lo == f64::NEG_INFINITY {
                continue;
            }
            substitutions.push(Substitution { var, kept, s, lb: new_lb[var], ub: new_ub[var], kept_lb: cur_lb });
            cur_lb += lo;
            cur_ub += hi;
            used[var] = true;
            any_merged = true;
            eliminated.insert(var);
        }
        if any_merged {
            new_lb[kept] = cur_lb;
            new_ub[kept] = cur_ub;
            used[kept] = true;
        }
    }

    if eliminated.is_empty() {
        return ParallelColsResult { a: a.clone(), c: new_c, lb: new_lb, ub: new_ub, real_rows: real_rows.to_vec(), substitutions };
    }

    for &j in &eliminated {
        new_c[j] = 0.0;
        new_lb[j] = 0.0;
        new_ub[j] = 0.0;
    }

    let new_a_rows: Vec<Vec<(usize, f64)>> = a_rows.into_iter().map(|row| row.into_iter().filter(|&(j, _)| !eliminated.contains(&j)).collect()).collect();
    let new_real_rows: Vec<Vec<(usize, f64)>> = real_rows.iter().map(|row| row.iter().copied().filter(|&(j, _)| !eliminated.contains(&j)).collect()).collect();

    ParallelColsResult { a: crate::sparse::csr_from_rows(&new_a_rows, n), c: new_c, lb: new_lb, ub: new_ub, real_rows: new_real_rows, substitutions }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;
    use crate::sparse::csr_row_vec;

    #[test]
    fn merges_two_identical_columns_with_equal_cost() {
        // x0 + x1 + x2 = 10, cost x0 and x1 identical (s=1) -- must merge
        // into one, x2 left alone (different pattern: only x2 appears in
        // the second row).
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0), (2, 1.0)], vec![(2, 1.0)]], 3);
        let c = vec![5.0, 5.0, 2.0];
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![4.0, 6.0, 3.0];
        let r = merge_parallel_columns(3, &a, &[], &c, &lb, &ub);
        assert_eq!(r.substitutions.len(), 1);
        let sub = &r.substitutions[0];
        assert_eq!(sub.kept, 0);
        assert_eq!(sub.var, 1);
        assert!((sub.s - 1.0).abs() < 1e-9);
        assert!((r.lb[0] - 0.0).abs() < 1e-9);
        assert!((r.ub[0] - 10.0).abs() < 1e-9); // 4 + 6
        assert_eq!(r.lb[1], 0.0);
        assert_eq!(r.ub[1], 0.0);
        assert_eq!(r.c[1], 0.0);
        // Column 1 dropped from the row.
        let row0 = csr_row_vec(&r.a, 0);
        assert!(!row0.iter().any(|&(j, _)| j == 1));
    }

    #[test]
    fn recovers_a_feasible_split_at_every_extreme_and_interior_point() {
        let a = csr_from_rows(&[vec![(0, 2.0), (1, 3.0)]], 2);
        let c = vec![4.0, 6.0]; // s = 2/3 col-wise; c0 = s*c1 => 4 = (2/3)*6 OK
        let lb = vec![1.0, 0.5];
        let ub = vec![3.0, 4.0];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert_eq!(r.substitutions.len(), 1);
        let sub = &r.substitutions[0];
        assert_eq!(sub.kept, 0);
        assert_eq!(sub.var, 1);

        // z range: lb0 + min(s*lb1,s*ub1) .. ub0 + max(...)
        let z_lo = r.lb[0];
        let z_hi = r.ub[0];
        for frac in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let z = z_lo + frac * (z_hi - z_lo);
            let mut x = vec![0.0; 2];
            x[0] = z;
            sub.apply(&mut x);
            let x1 = x[sub.var];
            let x0 = x[sub.kept];
            assert!(x1 >= lb[1] - 1e-9 && x1 <= ub[1] + 1e-9, "x1={x1} out of [{}, {}]", lb[1], ub[1]);
            assert!(x0 >= lb[0] - 1e-9 && x0 <= ub[0] + 1e-9, "x0={x0} out of [{}, {}]", lb[0], ub[0]);
            assert!((x0 + sub.s * x1 - z).abs() < 1e-9);
        }
    }

    #[test]
    fn handles_negative_scale() {
        let a = csr_from_rows(&[vec![(0, 1.0), (1, -2.0)]], 2);
        let lb = vec![0.0, 0.0];
        let ub = vec![5.0, 5.0];
        // col0 leading=1.0, col1 leading=-2.0 => s = -2.0/1.0 = -2.0 for
        // var=1 relative to kept=0. Cost check needs c[1] == s*c[0]: pick
        // c0=-3.0 => predicted c1 = -2.0*-3.0 = 6.0.
        let c = vec![-3.0, 6.0];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert_eq!(r.substitutions.len(), 1);
        let sub = &r.substitutions[0];
        assert!((sub.s - (-2.0)).abs() < 1e-9);
        let z_lo = r.lb[0];
        let z_hi = r.ub[0];
        for frac in [0.0, 0.33, 0.5, 0.9, 1.0] {
            let z = z_lo + frac * (z_hi - z_lo);
            let mut x = vec![0.0; 2];
            x[0] = z;
            sub.apply(&mut x);
            let x1 = x[sub.var];
            let x0 = x[sub.kept];
            assert!(x1 >= lb[1] - 1e-9 && x1 <= ub[1] + 1e-9, "x1={x1}");
            assert!(x0 >= lb[0] - 1e-9 && x0 <= ub[0] + 1e-9, "x0={x0}");
            assert!((x0 + sub.s * x1 - z).abs() < 1e-9);
        }
    }

    #[test]
    fn leaves_unequal_cost_ratio_untouched() {
        // Same pattern, but cost isn't proportional -- dominatedcol's job,
        // not this module's.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let c = vec![1.0, 2.0]; // s=1, but c1 != s*c0
        let lb = vec![0.0, 0.0];
        let ub = vec![5.0, 5.0];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert!(r.substitutions.is_empty());
    }

    #[test]
    fn merges_an_upper_unbounded_column_the_standgub_shape() {
        // lb=0, ub=+inf, cost=0 on both -- the exact shape of `standgub`'s
        // own 108-column GUB block this module was written for. Only the
        // *lower* bound needs to be finite; the merged `z`'s own upper
        // bound must come out `+inf` too, and postsolve must still recover
        // a feasible split at an arbitrarily large (but finite, as any
        // real solved value is) `z`.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let c = vec![0.0, 0.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![5.0, f64::INFINITY];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert_eq!(r.substitutions.len(), 1);
        assert!(r.ub[0].is_infinite() && r.ub[0] > 0.0);
        let sub = &r.substitutions[0];
        for &z in &[0.0, 3.0, 1_000_000.0] {
            let mut x = vec![0.0; 2];
            x[0] = z;
            sub.apply(&mut x);
            let x1 = x[sub.var];
            let x0 = x[sub.kept];
            assert!(x1 >= lb[1] - 1e-9 && x1 <= ub[1] + 1e-9);
            assert!(x0 >= lb[0] - 1e-9 && x0 <= ub[0] + 1e-9);
            assert!((x0 + sub.s * x1 - z).abs() < 1e-6);
        }
    }

    #[test]
    fn leaves_a_fully_free_column_untouched() {
        // Both sides infinite -- `freevar`'s job, not this module's (see
        // the module docs' "Scope" section).
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let c = vec![3.0, 3.0];
        let lb = vec![0.0, f64::NEG_INFINITY];
        let ub = vec![5.0, f64::INFINITY];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert!(r.substitutions.is_empty());
    }

    #[test]
    fn refuses_a_merge_that_would_make_the_result_lower_unbounded() {
        // `standgub`-shaped columns (`lb=0`, `ub=+inf`, `cost=0`) but with
        // *opposite-signed* leading matrix entries (`s=-1`) -- the exact
        // shape a real Netlib instance (`greenbea`) hit: both columns are
        // individually candidacy-eligible (finite `lb`), but folding `var`
        // into `kept` at `s=-1` would send `kept`'s own merged lower bound
        // to `-inf` (`lo = min(s*lb[var], s*ub[var]) = min(0, -inf) =
        // -inf`), producing a genuinely free (`lb=-inf` *and* `ub=+inf`)
        // structural column downstream -- exactly what
        // `presolve::freevar`/`simplex::extended_dual`'s own preconditions
        // require never exists past this point (see this loop's own
        // comment). Must be refused outright, not merged.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, -1.0)]], 2);
        let c = vec![0.0, 0.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert!(r.substitutions.is_empty());
        assert!(r.lb[0].is_finite());
    }

    #[test]
    fn merges_a_whole_group_in_one_call() {
        // Five identical columns, one call absorbs all four duplicates
        // into column 0.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0), (4, 1.0)]], 5);
        let c = vec![7.0; 5];
        let lb = vec![0.0; 5];
        let ub = vec![2.0; 5];
        let r = merge_parallel_columns(5, &a, &[], &c, &lb, &ub);
        assert_eq!(r.substitutions.len(), 4);
        assert!((r.ub[0] - 10.0).abs() < 1e-9); // 5 * 2.0
        for j in 1..5 {
            assert_eq!(r.lb[j], 0.0);
            assert_eq!(r.ub[j], 0.0);
        }
    }

    #[test]
    fn ignores_columns_with_no_real_row_appearance() {
        let a = csr_from_rows(&[], 2);
        let c = vec![1.0, 1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![5.0, 5.0];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert!(r.substitutions.is_empty());
    }
}
