//! Constraint propagation over inequality rows (Achterberg, Bixby, Gu,
//! Rothberg, Weninger, *"Presolve Reductions in Mixed Integer
//! Programming"*, ZIB Report 16-44, §3.1-3.2): tightens variable bounds
//! using row activity bounds, and detects rows that are always satisfied
//! (redundant, dropped) or always violated (the problem is infeasible).
//!
//! For a row `sum_j a_ij x_j <= b_i`, the paper defines the minimal/maximal
//! *activity*
//!
//!   inf{A_i. x} = sum_{a_ij>0} a_ij * lb_j + sum_{a_ij<0} a_ij * ub_j
//!   sup{A_i. x} = sum_{a_ij>0} a_ij * ub_j + sum_{a_ij<0} a_ij * lb_j
//!
//! (§2, eq. 2.2/2.3). A row is *redundant* (always satisfied) if
//! `sup <= b + eps`, and the problem is *infeasible* if `inf > b + eps`
//! (§3.1). Otherwise, for each variable `x_k` in the row's support, the
//! activity bound `l_iS` of the row excluding `x_k` gives a tighter bound
//! on `x_k` (§3.2, eq. 3.4/3.5).
//!
//! Variable bounds are not a separate vector in this codebase's `G`/`A`
//! representation — they are folded into `G`/`h` as single-variable rows
//! by `presolve::build_a_g`. So this module first pulls those out into an
//! explicit `lb`/`ub` pair via [`extract_bounds`] (which doubles as this
//! pass's working representation of "the bounds"), iterates §3.1/§3.2 over
//! the remaining multi-variable rows for `passes` rounds (each round
//! re-derives activities from whatever bounds were tightened so far — the
//! paper itself only does one round per presolve pass to keep the process
//! finite, see §3.2's `x1 - a*x2 = 0` example), and rebuilds `G`/`h` from
//! the surviving rows plus fresh bound rows for `interior_point`'s sake
//! (which wants bounds folded into `G` throughout). [`PropagateResult`]
//! *also* carries the already-split `lb`/`ub`/`real_rows`/`real_rhs`
//! directly, so `simplex.rs` (which wants bounds as `StdForm`'s own
//! explicit `lb`/`ub`, not folded into a row with its own slack) can use
//! those as-is instead of calling [`extract_bounds`] a second time on the
//! just-rebuilt `g`/`h` to undo the very folding this function just did.
//!
//! **Parallelization**: [`extract_bounds`] is a scatter (any row can
//! tighten any variable's bound), so a naive per-row-parallel write would
//! race; a rayon fold/reduce would avoid the race (like
//! `scaling::compute`'s column-norm accumulation once did) but, per
//! profiling on this crate's target problem sizes, costs more in
//! dispatch overhead than this scan itself — so it runs as a single
//! sequential pass instead (see `simplex.rs`'s `solve_lp_dual_on` module
//! docs for the same finding elsewhere). The main §3.1/§3.2 pass loop in
//! [`propagate`] is sequential for an unrelated, non-negotiable reason
//! regardless of problem size: it is Gauss-Seidel by design (each row
//! reads whatever `lb`/`ub` the *previous* rows in the *same* pass already
//! tightened), so parallelizing across rows would change which bounds are
//! visible to which row and alter the pass's convergence behavior, not
//! just its speed.

use crate::sparse::{Csr, CsrRowBuilder, csr_from_rows, csr_row_iter, csr_row_vec};
const EPS: f64 = 1e-9;

#[allow(dead_code)] // the pipeline uses `PropagateSplit`; kept for tests / G-form callers
pub struct PropagateResult {
    pub g: Csr,
    pub h: Vec<f64>,
    /// The same bounds already folded into `g`/`h` as single-variable
    /// rows, pulled back out — see the module docs for why this saves
    /// callers like `simplex.rs` a redundant `extract_bounds` call.
    pub lb: Vec<f64>,
    pub ub: Vec<f64>,
    /// The final surviving multi-variable rows, *not* re-folded with the
    /// bound rows the way `g`/`h` are — i.e. `g`/`h` minus its
    /// single-variable rows, equivalently `extract_bounds(n, &g, &h)`'s
    /// 3rd/4th return values, computed once here instead of twice.
    pub real_rows: Vec<Vec<(usize, f64)>>,
    pub real_rhs: Vec<f64>,
    pub infeasible: bool,
}

/// Splits `G x <= h` into its explicit single-variable bound rows
/// (returned as `lb`/`ub`, `f64::NEG_INFINITY`/`f64::INFINITY` where no
/// such row exists for a variable) and its remaining "real" (multi-
/// variable) rows (returned as sparse `rows`/`rhs`). Recognized purely by
/// row shape (`row.len() == 1`), not by position — works on `G` before or
/// after propagation, or on `G` built directly by `build_a_g`.
pub fn extract_bounds(n: usize, g: &Csr, h: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<Vec<(usize, f64)>>, Vec<f64>) {
    let mut lb = vec![f64::NEG_INFINITY; n];
    let mut ub = vec![f64::INFINITY; n];
    let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut rhs: Vec<f64> = Vec::new();

    let gr = g.as_ref();
    for i in 0..gr.nrows() {
        // Singleton (bound) rows are read straight from the CSR slices —
        // only the real rows need an owned copy.
        let cols = gr.col_indices_of_row_raw(i);
        if cols.len() == 1 {
            let (j, v) = (cols[0], gr.values_of_row(i)[0]);
            let bound = h[i] / v;
            if v > 0.0 {
                if bound < ub[j] {
                    ub[j] = bound;
                }
            } else if bound > lb[j] {
                lb[j] = bound;
            }
        } else {
            rows.push(csr_row_vec(g, i));
            rhs.push(h[i]);
        }
    }
    (lb, ub, rows, rhs)
}

/// The `lb`/`ub` half of [`extract_bounds`] alone — identical values,
/// without copying G's real rows out.
pub fn extract_bounds_only(n: usize, g: &Csr, h: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut lb = vec![f64::NEG_INFINITY; n];
    let mut ub = vec![f64::INFINITY; n];
    let gr = g.as_ref();
    for i in 0..gr.nrows() {
        let cols = gr.col_indices_of_row_raw(i);
        if cols.len() == 1 {
            let (j, v) = (cols[0], gr.values_of_row(i)[0]);
            let bound = h[i] / v;
            if v > 0.0 {
                if bound < ub[j] {
                    ub[j] = bound;
                }
            } else if bound > lb[j] {
                lb[j] = bound;
            }
        }
    }
    (lb, ub)
}

/// A variable's own two folded-in bound rows can contradict each other —
/// e.g. a single-variable `x >= 10` constraint folding in against an
/// `x <= 5` box bound (both single-variable rows are indistinguishable to
/// [`extract_bounds`], which just keeps the tighter of the two on each
/// side). That is a real infeasibility, not a representational quirk, and
/// must be caught explicitly before `lb`/`ub` are trusted for anything
/// else — every caller (`simplex.rs`'s `Tableau`, `interior_point`'s `G`
/// rows, `dualfix`'s own fixing rule) assumes every variable's bounds are
/// at least self-consistent, and `dualfix` in particular would otherwise
/// "fix" an already-inconsistent variable to one of its two contradictory
/// bounds, silently discarding the other and erasing the infeasibility.
pub fn bounds_inconsistent(n: usize, lb: &[f64], ub: &[f64]) -> bool {
    (0..n).any(|j| lb[j] > ub[j] + EPS)
}

/// [`propagate`]'s outcome without the re-folded `g`/`h` — what every
/// caller inside the presolve pipeline actually reads (`run_extended`
/// works on the split `lb`/`ub`/real-row form, `dualpropagate` only reads
/// the propagated box), so building the CSR there was pure overhead.
pub struct PropagateSplit {
    pub lb: Vec<f64>,
    pub ub: Vec<f64>,
    pub real_rows: Vec<Vec<(usize, f64)>>,
    pub real_rhs: Vec<f64>,
    pub infeasible: bool,
}

impl PropagateSplit {
    fn infeasible() -> Self {
        PropagateSplit { lb: Vec::new(), ub: Vec::new(), real_rows: Vec::new(), real_rhs: Vec::new(), infeasible: true }
    }
}

#[allow(dead_code)]
pub fn propagate(n: usize, g: &Csr, h: &[f64], passes: usize) -> PropagateResult {
    let r = propagate_nog(n, g, h, passes);
    if r.infeasible {
        return PropagateResult {
            g: csr_from_rows(&[], n),
            h: Vec::new(),
            lb: Vec::new(),
            ub: Vec::new(),
            real_rows: Vec::new(),
            real_rhs: Vec::new(),
            infeasible: true,
        };
    }
    let (new_g, new_h) = rebuild_g_ref(n, &r.real_rows, &r.real_rhs, &r.lb, &r.ub);
    PropagateResult { g: new_g, h: new_h, lb: r.lb, ub: r.ub, real_rows: r.real_rows, real_rhs: r.real_rhs, infeasible: false }
}

/// [`propagate`] without rebuilding `g`/`h` at the end (same `lb`/`ub`/
/// real rows, bit for bit).
pub fn propagate_nog(n: usize, g: &Csr, h: &[f64], passes: usize) -> PropagateSplit {
    let (lb, ub, rows, rhs) = extract_bounds(n, g, h);
    propagate_split(n, lb, ub, rows, rhs, passes)
}

/// [`propagate_nog`] on an already split `G` — `lb`/`ub` and the real
/// rows exactly as [`extract_bounds`] would return them for that `G`.
pub fn propagate_split(n: usize, mut lb: Vec<f64>, mut ub: Vec<f64>, mut rows: Vec<Vec<(usize, f64)>>, mut rhs: Vec<f64>, passes: usize) -> PropagateSplit {
    // `ENOMOTO_T_PROP_RELTOL` (default 0 = off, the historical absolute-EPS
    // rule only): HiGHS-style, a finite bound is only tightened when the
    // improvement also exceeds `reltol * (1 + |bound|)` — stops the
    // geometric shaving of bounds over cyclic row structures that keeps
    // the outer presolve rounds from reaching a fixpoint.
    let reltol = tunable!("ENOMOTO_T_PROP_RELTOL", 0.0, f64);

    if bounds_inconsistent(n, &lb, &ub) {
        return PropagateSplit::infeasible();
    }

    let mut infeasible = false;
    for _pass in 0..passes {
        if infeasible {
            break;
        }
        // A pass that neither drops a row nor writes any bound leaves the
        // state (`rows`, `rhs`, `lb`, `ub`) exactly as it found it, so every
        // further pass would repeat it verbatim — stop instead.
        let mut changed = false;
        let n_rows_before = rows.len();
        let mut kept_rows = Vec::with_capacity(rows.len());
        let mut kept_rhs = Vec::with_capacity(rhs.len());
        for (row, b) in std::mem::take(&mut rows).into_iter().zip(std::mem::take(&mut rhs)) {
            let mut finite_sum_inf = 0.0f64;
            let mut finite_sum_sup = 0.0f64;
            // Only "how many" and "which one, if exactly one" are ever
            // read, so no per-row allocation is needed.
            let mut inf_unbounded_count = 0usize;
            let mut inf_unbounded_first = usize::MAX;
            let mut sup_unbounded_count = 0usize;

            for &(j, v) in &row {
                if v > 0.0 {
                    if lb[j].is_finite() {
                        finite_sum_inf += v * lb[j];
                    } else {
                        {
                            if inf_unbounded_count == 0 {
                                inf_unbounded_first = j;
                            }
                            inf_unbounded_count += 1;
                        }
                    }
                    if ub[j].is_finite() {
                        finite_sum_sup += v * ub[j];
                    } else {
                        sup_unbounded_count += 1;
                    }
                } else {
                    if ub[j].is_finite() {
                        finite_sum_inf += v * ub[j];
                    } else {
                        {
                            if inf_unbounded_count == 0 {
                                inf_unbounded_first = j;
                            }
                            inf_unbounded_count += 1;
                        }
                    }
                    if lb[j].is_finite() {
                        finite_sum_sup += v * lb[j];
                    } else {
                        sup_unbounded_count += 1;
                    }
                }
            }

            let true_inf = if inf_unbounded_count == 0 { finite_sum_inf } else { f64::NEG_INFINITY };
            let true_sup = if sup_unbounded_count == 0 { finite_sum_sup } else { f64::INFINITY };

            if true_inf > b + EPS {
                infeasible = true;
                break;
            }
            if true_sup <= b + EPS {
                // Row can never be violated: redundant, drop it (§3.1).
                continue;
            }

            // Forcing row (§3.1's sibling case to the redundant/infeasible
            // checks just above, HiGHS's `HPresolve::rowPresolve` calls the
            // same thing): if the row's own minimum achievable value
            // already equals `b` (`true_inf` finite, `== b`), then
            // `sum <= b` combined with `sum >= true_inf == b` leaves sum
            // exactly one point, `b` — achievable only when every term
            // sits at whichever bound produced that minimum (a positive
            // coefficient at its lower bound, a negative one at its upper
            // bound). That fixes every variable in the row outright, which
            // in turn makes the row itself trivially satisfied — drop it,
            // the same as the redundant case above, rather than running
            // the (now moot) per-variable bound strengthening below on a
            // row with no remaining freedom at all. `inf_unbounded.is_empty()`
            // guarantees every bound this loop is about to read is finite
            // (that emptiness is exactly what made `true_inf` a real
            // number rather than `NEG_INFINITY` above).
            if inf_unbounded_count == 0 && (finite_sum_inf - b).abs() <= EPS {
                for &(j, v) in &row {
                    if v > 0.0 {
                        ub[j] = lb[j];
                    } else {
                        lb[j] = ub[j];
                    }
                }
                continue;
            }

            // Bound strengthening (§3.2): for each x_k in the row, l_iS is
            // the row's minimal activity excluding x_k's own contribution.
            // Only computable (finite) when no *other* variable is the
            // source of an unbounded contribution.
            for &(k, aik) in &row {
                let l_s = if inf_unbounded_count == 0 {
                    let contrib_k = if aik > 0.0 { aik * lb[k] } else { aik * ub[k] };
                    finite_sum_inf - contrib_k
                } else if inf_unbounded_count == 1 && inf_unbounded_first == k {
                    finite_sum_inf
                } else {
                    continue;
                };
                if !l_s.is_finite() {
                    continue;
                }
                if aik > 0.0 {
                    let candidate = (b - l_s) / aik;
                    if candidate < ub[k] - EPS && (reltol == 0.0 || !ub[k].is_finite() || candidate < ub[k] - reltol * (1.0 + ub[k].abs())) {
                        ub[k] = candidate;
                        changed = true;
                    }
                } else if aik < 0.0 {
                    let candidate = (b - l_s) / aik;
                    if candidate > lb[k] + EPS && (reltol == 0.0 || !lb[k].is_finite() || candidate > lb[k] + reltol * (1.0 + lb[k].abs())) {
                        lb[k] = candidate;
                        changed = true;
                    }
                }
            }

            kept_rows.push(row);
            kept_rhs.push(b);
        }

        if kept_rows.len() != n_rows_before {
            changed = true;
        }
        rows = kept_rows;
        rhs = kept_rhs;
        if !changed {
            break;
        }
    }

    // Bound strengthening above tightens `lb[k]`/`ub[k]` independently
    // from each row's own activity check, so a pass can drive the two
    // past each other even when no single row was individually flagged
    // infeasible (e.g. two different rows each push towards the other
    // side) — check once more after every pass has run.
    if !infeasible && bounds_inconsistent(n, &lb, &ub) {
        infeasible = true;
    }

    if infeasible {
        return PropagateSplit::infeasible();
    }

    PropagateSplit { lb, ub, real_rows: rows, real_rhs: rhs, infeasible: false }
}

/// `G x <= h` as a pass reads it: either a materialized CSR, or the split
/// form `(rows, rhs, lb, ub)` standing for exactly
/// `rebuild_g_ref(rows, rhs, lb, ub)` — only used when
/// [`split_is_canonical`] holds, so that `extract_bounds` of that matrix
/// would hand back `lb`/`ub`/`rows`/`rhs` themselves, bit for bit, and a
/// pass can read the split form directly instead of building the CSR (the
/// "bounds as rows" round trip, analysis/presolve_pipeline_20260924 C13).
#[derive(Clone, Copy)]
pub enum GView<'a> {
    Mat { g: &'a Csr, h: &'a [f64] },
    Split { rows: &'a [Vec<(usize, f64)>], rhs: &'a [f64], lb: &'a [f64], ub: &'a [f64] },
}

impl GView<'_> {
    /// Row count of the matrix this view stands for.
    pub fn nrows(&self) -> usize {
        match *self {
            GView::Mat { g, .. } => g.nrows(),
            GView::Split { rows, lb, ub, .. } => rows.len() + lb.iter().filter(|v| v.is_finite()).count() + ub.iter().filter(|v| v.is_finite()).count(),
        }
    }

    /// `extract_bounds_only` of the matrix this view stands for.
    pub fn bounds(&self, n: usize) -> (std::borrow::Cow<'_, [f64]>, std::borrow::Cow<'_, [f64]>) {
        match *self {
            GView::Mat { g, h } => {
                let (lb, ub) = extract_bounds_only(n, g, h);
                (std::borrow::Cow::Owned(lb), std::borrow::Cow::Owned(ub))
            }
            GView::Split { lb, ub, .. } => (std::borrow::Cow::Borrowed(lb), std::borrow::Cow::Borrowed(ub)),
        }
    }
}

/// Whether `rebuild_g_ref(rows, _, lb, ub)` round-trips exactly through
/// [`extract_bounds`]: every row has at least two entries, no stored zero
/// and strictly increasing in-range columns (so `rebuild_g_ref` stores it
/// verbatim and `extract_bounds` does not fold it into a bound), and every
/// bound is finite or infinite on its own side (a `-inf` upper / `+inf`
/// lower bound / NaN would emit no row and come back as the opposite
/// infinity).
pub fn split_is_canonical(n: usize, rows: &[Vec<(usize, f64)>], lb: &[f64], ub: &[f64]) -> bool {
    lb.iter().all(|&v| v.is_finite() || v == f64::NEG_INFINITY)
        && ub.iter().all(|&v| v.is_finite() || v == f64::INFINITY)
        && rows.iter().all(|r| r.len() >= 2 && r.iter().all(|&(j, v)| v != 0.0 && j < n) && r.windows(2).all(|w| w[0].0 < w[1].0))
}

/// Rebuilds `G x <= h` from a set of "real" (multi-variable) rows plus a
/// fresh pair of single-variable bound rows per variable with a finite
/// bound — the inverse of [`extract_bounds`]. Shared by [`propagate`]'s
/// own ending and by `dualfix`, which also needs to fold freshly-fixed
/// bounds back into `G` the same way.
/// [`rebuild_g`] without taking ownership of (and so without the caller
/// having to clone) `rows`/`rhs`, and without allocating one `Vec` per
/// bound row: builds the CSR directly. Produces a bit-identical `(G, h)`
/// (see `sparse::CsrRowBuilder`); falls back to [`rebuild_g`] itself in
/// the rare case a real row holds a duplicate column index.
pub fn rebuild_g_ref(n: usize, rows: &[Vec<(usize, f64)>], rhs: &[f64], lb: &[f64], ub: &[f64]) -> (Csr, Vec<f64>) {
    let n_bounds = (0..n).filter(|&j| ub[j].is_finite()).count() + (0..n).filter(|&j| lb[j].is_finite()).count();
    let nnz: usize = rows.iter().map(|r| r.len()).sum::<usize>() + n_bounds;
    let mut builder = CsrRowBuilder::with_capacity(n, rows.len() + n_bounds, nnz);
    for row in rows {
        if !builder.push_row(row) {
            return rebuild_g(n, rows.to_vec(), rhs.to_vec(), lb, ub);
        }
    }
    let mut new_rhs = Vec::with_capacity(rhs.len() + n_bounds);
    new_rhs.extend_from_slice(rhs);
    for j in 0..n {
        if ub[j].is_finite() {
            builder.push_singleton(j, 1.0);
            new_rhs.push(ub[j]);
        }
        if lb[j].is_finite() {
            builder.push_singleton(j, -1.0);
            new_rhs.push(-lb[j]);
        }
    }
    (builder.finish(), new_rhs)
}

pub fn rebuild_g(n: usize, mut rows: Vec<Vec<(usize, f64)>>, mut rhs: Vec<f64>, lb: &[f64], ub: &[f64]) -> (Csr, Vec<f64>) {
    for j in 0..n {
        if ub[j].is_finite() {
            rows.push(vec![(j, 1.0)]);
            rhs.push(ub[j]);
        }
        if lb[j].is_finite() {
            rows.push(vec![(j, -1.0)]);
            rhs.push(-lb[j]);
        }
    }
    (csr_from_rows(&rows, n), rhs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    #[test]
    fn forcing_row_fixes_every_variable_to_its_minimum_bound() {
        // x0 in [1,3], x1 in [2,4] (folded in as their own box-bound
        // rows), plus the real row x0 + x1 <= 3. That row's own minimum
        // achievable activity (1*lb[x0] + 1*lb[x1] = 1 + 2 = 3) already
        // equals its RHS, so it's a forcing row: satisfying `<= 3` at all
        // requires x0=1, x1=2 exactly (either one any larger would push
        // the sum past 3 with no room for the other to compensate, since
        // both coefficients are positive).
        let g = csr_from_rows(
            &[
                vec![(0, 1.0)],
                vec![(0, -1.0)],
                vec![(1, 1.0)],
                vec![(1, -1.0)],
                vec![(0, 1.0), (1, 1.0)],
            ],
            2,
        );
        let h = vec![3.0, -1.0, 4.0, -2.0, 3.0];
        let result = propagate(2, &g, &h, 1);
        assert!(!result.infeasible);
        assert!((result.lb[0] - 1.0).abs() < 1e-9, "lb={:?}", result.lb);
        assert!((result.ub[0] - 1.0).abs() < 1e-9, "ub={:?}", result.ub);
        assert!((result.lb[1] - 2.0).abs() < 1e-9, "lb={:?}", result.lb);
        assert!((result.ub[1] - 2.0).abs() < 1e-9, "ub={:?}", result.ub);
        // The forcing row itself, now trivially satisfied, is dropped
        // from the surviving multi-variable rows.
        assert!(result.real_rows.is_empty(), "real_rows={:?}", result.real_rows);
    }

    #[test]
    fn forcing_row_with_mixed_signs_uses_the_matching_bound_per_term() {
        // x0 in [0,10], x1 in [0,10], row x0 - x1 <= -4. Minimum activity:
        // coefficient of x0 is positive -> use lb[x0]=0; coefficient of
        // x1 is negative -> use ub[x1]=10. inf = 0 - 10 = -10 -- not equal
        // to -4, so *this* row isn't forcing; instead pick bounds so the
        // minimum lands exactly on the RHS: x0 in [2,10], x1 in [0,6],
        // inf = 1*2 + (-1)*6 = -4 = b. Forces x0=2 (lb, positive coeff),
        // x1=6 (ub, negative coeff).
        let g = csr_from_rows(
            &[
                vec![(0, 1.0)],
                vec![(0, -1.0)],
                vec![(1, 1.0)],
                vec![(1, -1.0)],
                vec![(0, 1.0), (1, -1.0)],
            ],
            2,
        );
        let h = vec![10.0, -2.0, 6.0, 0.0, -4.0];
        let result = propagate(2, &g, &h, 1);
        assert!(!result.infeasible);
        assert!((result.lb[0] - 2.0).abs() < 1e-9, "lb={:?}", result.lb);
        assert!((result.ub[0] - 2.0).abs() < 1e-9, "ub={:?}", result.ub);
        assert!((result.lb[1] - 6.0).abs() < 1e-9, "lb={:?}", result.lb);
        assert!((result.ub[1] - 6.0).abs() < 1e-9, "ub={:?}", result.ub);
        assert!(result.real_rows.is_empty(), "real_rows={:?}", result.real_rows);
    }
}

/// Activity-based propagation over the *equality* system `A x = b`, which
/// [`propagate`] above never sees (it is handed only the inequality system
/// `G x <= h`). Mirrors this module's own §3.1/§3.2 reductions, applied to
/// an equality row's two implied inequalities `A_i x <= b_i` and
/// `A_i x >= b_i` instead of one: forcing-row detection on *both* sides
/// (activity can only reach `b_i` with every term pinned at one bound) and
/// bound strengthening from whichever side is finite. Only `lb`/`ub` are
/// mutated; the rows themselves are left for `foldfixed` (fixed terms) and
/// the round loop's own rowsingleton/doubleton/colsingleton passes to
/// shrink.
///
/// Without this, a column that appears only in equality rows and has no
/// finite bound anywhere else reaches `extended_dual`'s dual simplex
/// unbounded on that side, which forces its far more expensive "M-side"
/// bookkeeping for every such column. On `greenbea` (92% equality rows)
/// that was 3,569 columns and an 11x blow-up in iteration count; running
/// this pass drops it to a few hundred columns and brings the iteration
/// count within 2-3x of HiGHS's own presolved problem. See
/// `analysis/greenbea_20260921_230908.md` for the full measurement.
pub struct EqPropagateResult {
    pub infeasible: bool,
    pub forcing_rows: usize,
    pub fixed_cols: usize,
    pub tightened: usize,
}

pub fn propagate_equalities(a: &Csr, b: &[f64], lb: &mut [f64], ub: &mut [f64], passes: usize) -> EqPropagateResult {
    let ar = a.as_ref();
    // A finite bound is only replaced when the change exceeds `EPS`; an
    // infinite bound is always replaced by a finite one.
    // `ENOMOTO_T_EQPROP_RELTOL` (default 0 = off): additionally require a
    // finite bound to move by more than `reltol * (1 + |old|)`.
    let reltol = tunable!("ENOMOTO_T_EQPROP_RELTOL", 0.0, f64);
    let improves = |old: f64, new: f64| -> bool {
        if !old.is_finite() {
            return true;
        }
        (old - new).abs() > EPS && (reltol == 0.0 || (old - new).abs() > reltol * (1.0 + old.abs()))
    };
    let mut res = EqPropagateResult { infeasible: false, forcing_rows: 0, fixed_cols: 0, tightened: 0 };
    let mut forcing_seen = vec![false; ar.nrows()];
    for _pass in 0..passes {
        let mut changed = 0usize;
        for i in 0..ar.nrows() {
            let bi = b[i];
            let mut finite_sum_inf = 0.0f64;
            let mut finite_sum_sup = 0.0f64;
            let mut inf_unbounded: Vec<usize> = Vec::new();
            let mut sup_unbounded: Vec<usize> = Vec::new();
            let mut live = 0usize;
            for (j, v) in csr_row_iter(a, i) {
                if v == 0.0 {
                    continue;
                }
                if lb[j] < ub[j] {
                    live += 1;
                }
                if v > 0.0 {
                    if lb[j].is_finite() {
                        finite_sum_inf += v * lb[j];
                    } else {
                        inf_unbounded.push(j);
                    }
                    if ub[j].is_finite() {
                        finite_sum_sup += v * ub[j];
                    } else {
                        sup_unbounded.push(j);
                    }
                } else {
                    if ub[j].is_finite() {
                        finite_sum_inf += v * ub[j];
                    } else {
                        inf_unbounded.push(j);
                    }
                    if lb[j].is_finite() {
                        finite_sum_sup += v * lb[j];
                    } else {
                        sup_unbounded.push(j);
                    }
                }
            }
            if live == 0 {
                continue;
            }
            let true_inf = if inf_unbounded.is_empty() { finite_sum_inf } else { f64::NEG_INFINITY };
            let true_sup = if sup_unbounded.is_empty() { finite_sum_sup } else { f64::INFINITY };
            if true_inf > bi + EPS || true_sup < bi - EPS {
                res.infeasible = true;
                return res;
            }
            // Forcing on the lower side: activity can only reach `b` with
            // every term at its inf-bound.
            if inf_unbounded.is_empty() && (finite_sum_inf - bi).abs() <= EPS {
                if !forcing_seen[i] {
                    forcing_seen[i] = true;
                    res.forcing_rows += 1;
                }
                for (j, v) in csr_row_iter(a, i) {
                    if v == 0.0 {
                        continue;
                    }
                    if lb[j] < ub[j] {
                        res.fixed_cols += 1;
                        changed += 1;
                    }
                    if v > 0.0 {
                        ub[j] = lb[j];
                    } else {
                        lb[j] = ub[j];
                    }
                }
                continue;
            }
            // Forcing on the upper side: symmetric, at every term's sup-bound.
            if sup_unbounded.is_empty() && (finite_sum_sup - bi).abs() <= EPS {
                if !forcing_seen[i] {
                    forcing_seen[i] = true;
                    res.forcing_rows += 1;
                }
                for (j, v) in csr_row_iter(a, i) {
                    if v == 0.0 {
                        continue;
                    }
                    if lb[j] < ub[j] {
                        res.fixed_cols += 1;
                        changed += 1;
                    }
                    if v > 0.0 {
                        lb[j] = ub[j];
                    } else {
                        ub[j] = lb[j];
                    }
                }
                continue;
            }
            for (k, aik) in csr_row_iter(a, i) {
                if aik == 0.0 || lb[k] == ub[k] {
                    continue;
                }
                // `A_i x <= b`: bound from the row's min activity excluding k.
                let l_s = if inf_unbounded.is_empty() {
                    let contrib = if aik > 0.0 { aik * lb[k] } else { aik * ub[k] };
                    Some(finite_sum_inf - contrib)
                } else if inf_unbounded.len() == 1 && inf_unbounded[0] == k {
                    Some(finite_sum_inf)
                } else {
                    None
                };
                if let Some(l_s) = l_s {
                    let candidate = (bi - l_s) / aik;
                    if aik > 0.0 {
                        if candidate < ub[k] - EPS && improves(ub[k], candidate) {
                            ub[k] = candidate;
                            changed += 1;
                            res.tightened += 1;
                        }
                    } else if candidate > lb[k] + EPS && improves(lb[k], candidate) {
                        lb[k] = candidate;
                        changed += 1;
                        res.tightened += 1;
                    }
                }
                // `A_i x >= b`: bound from the row's max activity excluding k.
                let u_s = if sup_unbounded.is_empty() {
                    let contrib = if aik > 0.0 { aik * ub[k] } else { aik * lb[k] };
                    Some(finite_sum_sup - contrib)
                } else if sup_unbounded.len() == 1 && sup_unbounded[0] == k {
                    Some(finite_sum_sup)
                } else {
                    None
                };
                if let Some(u_s) = u_s {
                    let candidate = (bi - u_s) / aik;
                    if aik > 0.0 {
                        if candidate > lb[k] + EPS && improves(lb[k], candidate) {
                            lb[k] = candidate;
                            changed += 1;
                            res.tightened += 1;
                        }
                    } else if candidate < ub[k] - EPS && improves(ub[k], candidate) {
                        ub[k] = candidate;
                        changed += 1;
                        res.tightened += 1;
                    }
                }
                if lb[k] > ub[k] + EPS {
                    res.infeasible = true;
                    return res;
                }
                if lb[k] > ub[k] {
                    // Within EPS: snap to a single point.
                    lb[k] = ub[k];
                }
            }
        }
        if changed == 0 {
            break;
        }
    }
    res
}

#[cfg(test)]
mod eqprop_tests {
    use super::*;

    #[test]
    fn forcing_row_fixes_every_term_at_its_matching_bound() {
        // x0 in [0,10], x1 in [0,10], row x0 - x1 = -4. Minimum activity
        // with x0 at lb=0, x1 at ub=10 is -10 (not -4, not forcing on the
        // lower side); maximum activity with x0 at ub=10, x1 at lb=0 is 10
        // (not -4 either). Pick bounds so the minimum lands exactly on the
        // RHS instead: x0 in [2,10], x1 in [0,6], inf = 1*2 + (-1)*6 = -4 = b.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, -1.0)]], 2);
        let b = vec![-4.0];
        let mut lb = vec![2.0, 0.0];
        let mut ub = vec![10.0, 6.0];
        let res = propagate_equalities(&a, &b, &mut lb, &mut ub, 1);
        assert!(!res.infeasible);
        assert_eq!(res.forcing_rows, 1);
        assert_eq!(res.fixed_cols, 2);
        assert!((lb[0] - 2.0).abs() < 1e-9 && (ub[0] - 2.0).abs() < 1e-9, "x0={:?}/{:?}", lb[0], ub[0]);
        assert!((lb[1] - 6.0).abs() < 1e-9 && (ub[1] - 6.0).abs() < 1e-9, "x1={:?}/{:?}", lb[1], ub[1]);
    }

    #[test]
    fn free_column_gets_a_finite_bound_from_an_equality_row() {
        // x0 free, x1 in [0,5], row x0 + x1 = 3. x0's only bound comes from
        // this equality: x0 = 3 - x1 in [3-5, 3-0] = [-2, 3].
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let b = vec![3.0];
        let mut lb = vec![f64::NEG_INFINITY, 0.0];
        let mut ub = vec![f64::INFINITY, 5.0];
        let res = propagate_equalities(&a, &b, &mut lb, &mut ub, 2);
        assert!(!res.infeasible);
        assert!(res.tightened >= 2, "tightened={}", res.tightened);
        assert!((lb[0] - (-2.0)).abs() < 1e-9, "lb[0]={}", lb[0]);
        assert!((ub[0] - 3.0).abs() < 1e-9, "ub[0]={}", ub[0]);
    }

    #[test]
    fn contradictory_equality_row_is_infeasible() {
        // x0 in [0,1], x1 in [0,1], row x0 + x1 = 5: max activity is 2 < 5.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let b = vec![5.0];
        let mut lb = vec![0.0, 0.0];
        let mut ub = vec![1.0, 1.0];
        let res = propagate_equalities(&a, &b, &mut lb, &mut ub, 1);
        assert!(res.infeasible);
    }
}
