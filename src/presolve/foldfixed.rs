//! FoldFixed: drops every already-fixed column's own term from a set of
//! rows, folding `coeff * value` into that row's own right-hand side
//! instead — the "plug in a known constant" half of ordinary variable
//! elimination that merely *fixing* a variable's bound (`dualfix`,
//! [`super::dualpropagate`]'s column fixing, `rowsingleton`) leaves
//! undone on its own: setting `lb[j] = ub[j] = value` only tells later
//! passes *what* `x_j` is, not that a row still carrying its literal term
//! has one fewer *live* variable than its own length says. Left undone, a
//! row that starts with (say) 16 terms and has 14 of them fixed away
//! elsewhere still *looks* like a 16-variable row to `rowsingleton` (wants
//! exactly 1 live term), `doubleton` (wants exactly 2), and `aggregator`'s
//! implied-free gate — none of which can fire on it until something drops
//! those 14 dead terms and shrinks it down to the 2 genuinely live ones.
//! (`doubleton`/`colsingleton`/`aggregator` themselves never leave this
//! kind of residue behind for their *own* eliminated variable — each
//! rewrites every row it touches directly — so this module exists purely
//! to clean up after the bound-only fixers, which don't touch `A`/`G` at
//! all.)
//!
//! Run once per outer round, right after whichever passes did the fixing
//! for that round (mirrors this crate's own "single pass, caller repeats
//! via the round loop" idiom used throughout this pipeline — a column
//! fixed by *this* round's own `rowsingleton` is picked up by *next*
//! round's call, not re-scanned within this same one, since
//! `ROWSINGLETON_COLSINGLETON_INNER_ROUNDS` defaults to a single inner
//! pass anyway).

use crate::types::RowSense;

const TOL: f64 = 1e-9;

pub struct FoldFixedResult {
    pub rows: Vec<Vec<(usize, f64)>>,
    pub rhs: Vec<f64>,
    pub infeasible: bool,
}

/// Drops every term whose column is fixed (`lb[j] == ub[j]`) from each of
/// `rows`, subtracting `coeff * lb[j]` from that row's own `rhs` entry so
/// the row stays algebraically identical with one fewer live variable —
/// works identically for `A`'s equality rows and `G`'s real inequality
/// rows, since folding a constant into the right-hand side needs no sign
/// case analysis either way. `sense` only matters for a row that loses
/// *every* term this way: [`RowSense::Eq`] needs the folded constant to
/// already equal `rhs` (within `TOL`, else the row is a genuine
/// infeasibility, same check `rowsingleton`'s own fixed-value case
/// makes), [`RowSense::Le`] only needs `0 <= rhs'` (the row's own leftover
/// slack must still be nonnegative, the same forcing-row-style check
/// `propagate` already makes elsewhere) — [`RowSense::Ge`] is never
/// actually passed (this crate's `G` rows are always pre-normalized to
/// `<=`) but handled the mirror-image way for completeness rather than
/// left to panic. A row that survives with zero live terms and passes
/// this check is simply dropped (it now says nothing `rhs`'s own folded
/// value doesn't already guarantee).
pub fn fold_fixed_columns(rows: &[Vec<(usize, f64)>], rhs: &[f64], lb: &[f64], ub: &[f64], sense: RowSense) -> FoldFixedResult {
    let mut new_rows = Vec::with_capacity(rows.len());
    let mut new_rhs = Vec::with_capacity(rhs.len());
    for (row, &r) in rows.iter().zip(rhs.iter()) {
        let mut live = Vec::with_capacity(row.len());
        let mut folded = r;
        for &(j, v) in row {
            if v == 0.0 {
                continue;
            }
            if lb[j] == ub[j] {
                folded -= v * lb[j];
            } else {
                live.push((j, v));
            }
        }
        if live.is_empty() {
            let ok = match sense {
                RowSense::Eq => folded.abs() <= TOL,
                RowSense::Le => folded >= -TOL,
                RowSense::Ge => folded <= TOL,
            };
            if !ok {
                return FoldFixedResult { rows: Vec::new(), rhs: Vec::new(), infeasible: true };
            }
            continue;
        }
        new_rows.push(live);
        new_rhs.push(folded);
    }
    FoldFixedResult { rows: new_rows, rhs: new_rhs, infeasible: false }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_fixed_terms_and_folds_their_value_into_rhs() {
        // x0 + 2*x1 + x2 = 10, x1 fixed at 3 => x0 + x2 = 4.
        let rows = vec![vec![(0, 1.0), (1, 2.0), (2, 1.0)]];
        let rhs = vec![10.0];
        let lb = vec![0.0, 3.0, 0.0];
        let ub = vec![10.0, 3.0, 10.0];
        let result = fold_fixed_columns(&rows, &rhs, &lb, &ub, RowSense::Eq);
        assert!(!result.infeasible);
        assert_eq!(result.rows, vec![vec![(0, 1.0), (2, 1.0)]]);
        assert_eq!(result.rhs, vec![4.0]);
    }

    #[test]
    fn leaves_a_row_with_no_fixed_columns_untouched() {
        let rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let rhs = vec![5.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 10.0];
        let result = fold_fixed_columns(&rows, &rhs, &lb, &ub, RowSense::Eq);
        assert!(!result.infeasible);
        assert_eq!(result.rows, rows);
        assert_eq!(result.rhs, rhs);
    }

    #[test]
    fn a_row_reduced_to_zero_live_terms_vanishes_when_consistent() {
        // x0 + x1 = 5, both fixed at 2 and 3 => 0 = 0, row drops entirely.
        let rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let rhs = vec![5.0];
        let lb = vec![2.0, 3.0];
        let ub = vec![2.0, 3.0];
        let result = fold_fixed_columns(&rows, &rhs, &lb, &ub, RowSense::Eq);
        assert!(!result.infeasible);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn a_row_reduced_to_zero_live_terms_is_infeasible_when_inconsistent() {
        let rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let rhs = vec![5.0];
        let lb = vec![2.0, 2.0];
        let ub = vec![2.0, 2.0];
        let result = fold_fixed_columns(&rows, &rhs, &lb, &ub, RowSense::Eq);
        assert!(result.infeasible);
    }

    #[test]
    fn an_inequality_row_reduced_to_zero_live_terms_only_needs_nonnegative_slack() {
        // x0 <= 5, x0 fixed at 3 => 0 <= 2, satisfied, row drops.
        let rows = vec![vec![(0, 1.0)]];
        let rhs = vec![5.0];
        let lb = vec![3.0];
        let ub = vec![3.0];
        let result = fold_fixed_columns(&rows, &rhs, &lb, &ub, RowSense::Le);
        assert!(!result.infeasible);
        assert!(result.rows.is_empty());

        // x0 <= 5, x0 fixed at 7 => 0 <= -2, violated.
        let lb2 = vec![7.0];
        let ub2 = vec![7.0];
        let result2 = fold_fixed_columns(&rows, &rhs, &lb2, &ub2, RowSense::Le);
        assert!(result2.infeasible);
    }

    #[test]
    fn explicit_zero_coefficients_are_dropped_without_touching_rhs() {
        let rows = vec![vec![(0, 0.0), (1, 1.0)]];
        let rhs = vec![5.0];
        let lb = vec![100.0, 0.0]; // x0's own bound is irrelevant: its coefficient here is 0
        let ub = vec![100.0, 10.0];
        let result = fold_fixed_columns(&rows, &rhs, &lb, &ub, RowSense::Eq);
        assert!(!result.infeasible);
        assert_eq!(result.rows, vec![vec![(1, 1.0)]]);
        assert_eq!(result.rhs, vec![5.0]);
    }
}
