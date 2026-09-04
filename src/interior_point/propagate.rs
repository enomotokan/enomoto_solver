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
//! Variable bounds are not a separate vector in this codebase — they are
//! already folded into `G`/`h` as single-variable rows by `qp.rs`. So this
//! module first pulls those out into an explicit `lb`/`ub` pair (which
//! doubles as this pass's working representation of "the bounds"),
//! iterates §3.1/§3.2 over the remaining multi-variable rows for `passes`
//! rounds (each round re-derives activities from whatever bounds were
//! tightened so far — the paper itself only does one round per presolve
//! pass to keep the process finite, see §3.2's `x1 - a*x2 = 0` example),
//! and rebuilds `G`/`h` from the surviving rows plus fresh bound rows.

use super::kkt::{csr_from_rows, Csr};

const EPS: f64 = 1e-9;

pub struct PropagateResult {
    pub g: Csr,
    pub h: Vec<f64>,
    pub infeasible: bool,
}

pub fn propagate(n: usize, g: &Csr, h: &[f64], passes: usize) -> PropagateResult {
    let mut lb = vec![f64::NEG_INFINITY; n];
    let mut ub = vec![f64::INFINITY; n];
    let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut rhs: Vec<f64> = Vec::new();

    // Recover the explicit variable bounds folded into single-variable
    // rows; everything else is a "real" (multi-variable) row.
    let gr = g.as_ref();
    for i in 0..gr.nrows() {
        let row: Vec<(usize, f64)> = gr
            .col_indices_of_row(i)
            .zip(gr.values_of_row(i))
            .map(|(j, &v)| (j, v))
            .collect();
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

    if infeasible {
        return PropagateResult {
            g: csr_from_rows(&[], n),
            h: Vec::new(),
            infeasible: true,
        };
    }

    let mut new_rows = rows;
    let mut new_h = rhs;
    for j in 0..n {
        if ub[j].is_finite() {
            new_rows.push(vec![(j, 1.0)]);
            new_h.push(ub[j]);
        }
        if lb[j].is_finite() {
            new_rows.push(vec![(j, -1.0)]);
            new_h.push(-lb[j]);
        }
    }

    let new_g = csr_from_rows(&new_rows, n);
    PropagateResult { g: new_g, h: new_h, infeasible: false }
}
