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

use crate::sparse::{csr_from_rows, Csr};

const EPS: f64 = 1e-9;

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
        let row: Vec<(usize, f64)> = gr.col_indices_of_row(i).zip(gr.values_of_row(i)).map(|(j, &v)| (j, v)).collect();
        if row.len() == 1 {
            let (j, v) = row[0];
            let bound = h[i] / v;
            if v > 0.0 {
                if bound < ub[j] {
                    ub[j] = bound;
                }
            } else if bound > lb[j] {
                lb[j] = bound;
            }
        } else {
            rows.push(row);
            rhs.push(h[i]);
        }
    }
    (lb, ub, rows, rhs)
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

pub fn propagate(n: usize, g: &Csr, h: &[f64], passes: usize) -> PropagateResult {
    let (mut lb, mut ub, mut rows, mut rhs) = extract_bounds(n, g, h);

    if bounds_inconsistent(n, &lb, &ub) {
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

    let mut infeasible = false;
    for _pass in 0..passes {
        if infeasible {
            break;
        }
        let mut kept_rows = Vec::with_capacity(rows.len());
        let mut kept_rhs = Vec::with_capacity(rhs.len());

        for (row, &b) in rows.iter().zip(rhs.iter()) {
            let mut finite_sum_inf = 0.0f64;
            let mut finite_sum_sup = 0.0f64;
            let mut inf_unbounded: Vec<usize> = Vec::new();
            let mut sup_unbounded: Vec<usize> = Vec::new();

            for &(j, v) in row {
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

            let true_inf = if inf_unbounded.is_empty() { finite_sum_inf } else { f64::NEG_INFINITY };
            let true_sup = if sup_unbounded.is_empty() { finite_sum_sup } else { f64::INFINITY };

            if true_inf > b + EPS {
                infeasible = true;
                break;
            }
            if true_sup <= b + EPS {
                // Row can never be violated: redundant, drop it (§3.1).
                continue;
            }

            // Bound strengthening (§3.2): for each x_k in the row, l_iS is
            // the row's minimal activity excluding x_k's own contribution.
            // Only computable (finite) when no *other* variable is the
            // source of an unbounded contribution.
            for &(k, aik) in row {
                let l_s = if inf_unbounded.is_empty() {
                    let contrib_k = if aik > 0.0 { aik * lb[k] } else { aik * ub[k] };
                    finite_sum_inf - contrib_k
                } else if inf_unbounded.len() == 1 && inf_unbounded[0] == k {
                    finite_sum_inf
                } else {
                    continue;
                };
                if !l_s.is_finite() {
                    continue;
                }
                if aik > 0.0 {
                    let candidate = (b - l_s) / aik;
                    if candidate < ub[k] - EPS {
                        ub[k] = candidate;
                    }
                } else if aik < 0.0 {
                    let candidate = (b - l_s) / aik;
                    if candidate > lb[k] + EPS {
                        lb[k] = candidate;
                    }
                }
            }

            kept_rows.push(row.clone());
            kept_rhs.push(b);
        }

        rows = kept_rows;
        rhs = kept_rhs;
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

    let (new_g, new_h) = rebuild_g(n, rows.clone(), rhs.clone(), &lb, &ub);
    PropagateResult { g: new_g, h: new_h, lb, ub, real_rows: rows, real_rhs: rhs, infeasible: false }
}

/// Rebuilds `G x <= h` from a set of "real" (multi-variable) rows plus a
/// fresh pair of single-variable bound rows per variable with a finite
/// bound — the inverse of [`extract_bounds`]. Shared by [`propagate`]'s
/// own ending and by `dualfix`, which also needs to fold freshly-fixed
/// bounds back into `G` the same way.
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
