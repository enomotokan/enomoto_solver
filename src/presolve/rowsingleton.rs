//! RowSingleton: an equality row with exactly one nonzero variable,
//! `coeff * x_j = rhs`, directly fixes `x_j = rhs / coeff` — no
//! substitution bookkeeping needed (unlike `colsingleton`/`doubleton`,
//! there are no *other* variables in the row to express anything in terms
//! of). Distinct from `dualfix`'s cost-sign-based fixing and from
//! `propagate::extract_bounds`'s single-variable-row handling, which only
//! ever looks at `G`'s (inequality/bound) rows, never `A`'s equalities.
//!
//! A fixed value outside the variable's current `[lb, ub]` is a genuine
//! infeasibility (an equality forcing a value the variable's own bounds
//! already rule out), reported directly rather than silently overriding
//! one of the two bounds — same discipline as
//! `propagate::bounds_inconsistent`/`dualfix`.

use crate::sparse::{Csr, csr_from_rows, csr_is_canonical, csr_row_iter};
const TOL: f64 = 1e-9;

pub struct RowSingletonResult {
    pub a: Csr,
    pub b: Vec<f64>,
    /// `(j, value)` for every variable fixed this pass.
    pub fixes: Vec<(usize, f64)>,
    pub infeasible: bool,
}

/// One non-cascading pass over `A`'s rows (mirrors `colsingleton`'s own
/// "computed once, not re-checked after each fix" scope — a later call on
/// this pass's own output catches anything that becomes a singleton row
/// only afterward).
pub fn fix_singleton_equalities(n: usize, a: &Csr, b: &[f64], lb: &[f64], ub: &[f64]) -> RowSingletonResult {
    let ar = a.as_ref();
    // No singleton row at all (the common case after the first round):
    // the rebuilt A would be `a` itself when it is already canonical.
    if csr_is_canonical(a) && (0..ar.nrows()).all(|i| ar.col_indices_of_row_raw(i).len() != 1) {
        return RowSingletonResult { a: a.clone(), b: b[..ar.nrows()].to_vec(), fixes: Vec::new(), infeasible: false };
    }
    let mut new_a_rows = Vec::with_capacity(ar.nrows());
    let mut new_b = Vec::with_capacity(b.len());
    let mut fixes = Vec::new();

    for i in 0..ar.nrows() {
        let row: Vec<(usize, f64)> =
            csr_row_iter(a, i).filter(|&(_, v)| v != 0.0).collect();
        if row.len() == 1 {
            let (j, coeff) = row[0];
            let value = b[i] / coeff;
            if value < lb[j] - TOL || value > ub[j] + TOL {
                return RowSingletonResult { a: csr_from_rows(&[], n), b: Vec::new(), fixes: Vec::new(), infeasible: true };
            }
            fixes.push((j, value));
            continue;
        }
        new_a_rows.push(row);
        new_b.push(b[i]);
    }

    RowSingletonResult { a: csr_from_rows(&new_a_rows, n), b: new_b, fixes, infeasible: false }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixes_a_singleton_row_and_drops_it() {
        // 2*x0 = 6 (row 0, singleton) ; x0 + x1 = 5 (row 1, not singleton)
        let a = csr_from_rows(&[vec![(0, 2.0)], vec![(0, 1.0), (1, 1.0)]], 2);
        let b = vec![6.0, 5.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 10.0];
        let result = fix_singleton_equalities(2, &a, &b, &lb, &ub);
        assert!(!result.infeasible);
        assert_eq!(result.fixes, vec![(0, 3.0)]);
        assert_eq!(result.a.nrows(), 1);
        assert_eq!(result.b, vec![5.0]);
    }

    #[test]
    fn reports_infeasible_when_forced_value_outside_bounds() {
        let a = csr_from_rows(&[vec![(0, 2.0)]], 1);
        let b = vec![100.0]; // x0 = 50, but ub is 10
        let lb = vec![0.0];
        let ub = vec![10.0];
        let result = fix_singleton_equalities(1, &a, &b, &lb, &ub);
        assert!(result.infeasible);
    }
}
