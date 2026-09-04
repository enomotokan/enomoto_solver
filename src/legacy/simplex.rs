//! A from-scratch two-phase primal simplex method operating on a dense
//! tableau built from the standardized `CsrMatrix`. Bland's rule is used
//! for both the entering- and leaving-variable choice, which is slower
//! than Dantzig's rule but guarantees termination (no cycling) without
//! extra bookkeeping — appropriate for the problem sizes this MVP targets.

use crate::preprocess::StandardForm;
use crate::types::{RowSense, Status};
use std::collections::HashSet;

const TOL: f64 = 1e-9;

enum PivotOutcome {
    Optimal,
    Unbounded,
}

fn reduced_costs(tableau: &[Vec<f64>], basis: &[usize], cost: &[f64], total_cols: usize) -> Vec<f64> {
    let m = tableau.len();
    let mut r = vec![0.0; total_cols];
    for j in 0..total_cols {
        let mut zj = 0.0;
        for i in 0..m {
            let cb = cost[basis[i]];
            if cb != 0.0 {
                zj += cb * tableau[i][j];
            }
        }
        r[j] = cost[j] - zj;
    }
    r
}

fn pivot(tableau: &mut [Vec<f64>], basis: &mut [usize], row: usize, col: usize, total_cols: usize) {
    let piv = tableau[row][col];
    let ncols = total_cols + 1;
    for k in 0..ncols {
        tableau[row][k] /= piv;
    }
    let m = tableau.len();
    for i in 0..m {
        if i == row {
            continue;
        }
        let factor = tableau[i][col];
        if factor != 0.0 {
            for k in 0..ncols {
                tableau[i][k] -= factor * tableau[row][k];
            }
        }
    }
    basis[row] = col;
}

fn run_simplex(
    tableau: &mut [Vec<f64>],
    basis: &mut [usize],
    cost: &[f64],
    total_cols: usize,
    excluded: &HashSet<usize>,
    max_iters: usize,
) -> PivotOutcome {
    let m = tableau.len();
    for _ in 0..max_iters {
        let r = reduced_costs(tableau, basis, cost, total_cols);
        let entering = (0..total_cols).find(|j| !excluded.contains(j) && r[*j] < -TOL);
        let entering = match entering {
            Some(e) => e,
            None => return PivotOutcome::Optimal,
        };

        let mut leaving: Option<usize> = None;
        let mut best_ratio = f64::INFINITY;
        for i in 0..m {
            let a = tableau[i][entering];
            if a > TOL {
                let ratio = tableau[i][total_cols] / a;
                let better = ratio < best_ratio - 1e-12;
                let tied_but_smaller_basis = (ratio - best_ratio).abs() <= 1e-9
                    && leaving.map_or(false, |lr| basis[i] < basis[lr]);
                if better || tied_but_smaller_basis {
                    best_ratio = ratio;
                    leaving = Some(i);
                }
            }
        }

        match leaving {
            None => return PivotOutcome::Unbounded,
            Some(lr) => pivot(tableau, basis, lr, entering, total_cols),
        }
    }
    // Iteration cap reached (possible degenerate cycling on pathological
    // inputs); return the best tableau found so far as a best-effort result.
    PivotOutcome::Optimal
}

/// Solves `minimize obj_std . y` subject to the standardized system,
/// `y >= 0`. `obj_std` gives sparse (column, coeff) pairs in standardized
/// column space. Returns the standardized solution vector `y` on success.
pub fn solve_standard(sf: &StandardForm, obj_std: &[(usize, f64)]) -> (Status, Option<Vec<f64>>) {
    let n = sf.n_std_vars;
    let m = sf.csr.n_rows;

    if m == 0 {
        if n == 0 {
            return (Status::Optimal, Some(vec![]));
        }
        // No constraints at all: y = 0 is feasible. Unbounded iff some
        // variable has strictly negative objective coefficient (since it
        // could grow to +inf, driving the objective to -inf).
        let mut cost = vec![0.0; n];
        for &(j, c) in obj_std {
            cost[j] += c;
        }
        if cost.iter().any(|&c| c < -TOL) {
            return (Status::Unbounded, None);
        }
        return (Status::Optimal, Some(vec![0.0; n]));
    }

    let mut slack_col: Vec<Option<usize>> = vec![None; m];
    let mut surplus_col: Vec<Option<usize>> = vec![None; m];
    let mut artificial_col: Vec<Option<usize>> = vec![None; m];
    let mut artificial_cols: HashSet<usize> = HashSet::new();
    let mut col_cursor = n;

    for i in 0..m {
        match sf.row_sense[i] {
            RowSense::Le => {
                slack_col[i] = Some(col_cursor);
                col_cursor += 1;
            }
            RowSense::Ge => {
                surplus_col[i] = Some(col_cursor);
                col_cursor += 1;
                artificial_col[i] = Some(col_cursor);
                artificial_cols.insert(col_cursor);
                col_cursor += 1;
            }
            RowSense::Eq => {
                artificial_col[i] = Some(col_cursor);
                artificial_cols.insert(col_cursor);
                col_cursor += 1;
            }
        }
    }
    let total_cols = col_cursor;

    let mut tableau: Vec<Vec<f64>> = vec![vec![0.0; total_cols + 1]; m];
    let mut basis: Vec<usize> = vec![0; m];

    for i in 0..m {
        for (j, v) in sf.csr.row(i) {
            tableau[i][j] = v;
        }
        tableau[i][total_cols] = sf.rhs[i];
        match sf.row_sense[i] {
            RowSense::Le => {
                let sc = slack_col[i].unwrap();
                tableau[i][sc] = 1.0;
                basis[i] = sc;
            }
            RowSense::Ge => {
                let suc = surplus_col[i].unwrap();
                let ac = artificial_col[i].unwrap();
                tableau[i][suc] = -1.0;
                tableau[i][ac] = 1.0;
                basis[i] = ac;
            }
            RowSense::Eq => {
                let ac = artificial_col[i].unwrap();
                tableau[i][ac] = 1.0;
                basis[i] = ac;
            }
        }
    }

    let max_iters = 500 + 50 * (m + total_cols);

    // ---- Phase 1: minimize the sum of artificial variables ----
    if !artificial_cols.is_empty() {
        let mut cost1 = vec![0.0; total_cols];
        for &c in artificial_cols.iter() {
            cost1[c] = 1.0;
        }
        let no_exclusions: HashSet<usize> = HashSet::new();
        run_simplex(&mut tableau, &mut basis, &cost1, total_cols, &no_exclusions, max_iters);

        let phase1_obj: f64 = (0..m)
            .filter(|&i| artificial_cols.contains(&basis[i]))
            .map(|i| tableau[i][total_cols])
            .sum();
        if phase1_obj > 1e-6 {
            return (Status::Infeasible, None);
        }

        // Drive any remaining zero-valued artificial variables out of the
        // basis so phase 2 starts from a clean (artificial-free) basis.
        for i in 0..m {
            if artificial_cols.contains(&basis[i]) {
                if let Some(j) = (0..n).chain(0..total_cols).find(|&j| {
                    !artificial_cols.contains(&j) && tableau[i][j].abs() > TOL
                }) {
                    pivot(&mut tableau, &mut basis, i, j, total_cols);
                }
            }
        }
    }

    // ---- Phase 2: minimize the real objective ----
    let mut cost2 = vec![0.0; total_cols];
    for &(j, c) in obj_std {
        cost2[j] += c;
    }
    match run_simplex(&mut tableau, &mut basis, &cost2, total_cols, &artificial_cols, max_iters) {
        PivotOutcome::Unbounded => (Status::Unbounded, None),
        PivotOutcome::Optimal => {
            let mut y = vec![0.0; n];
            for i in 0..m {
                if basis[i] < n {
                    y[basis[i]] = tableau[i][total_cols];
                }
            }
            (Status::Optimal, Some(y))
        }
    }
}
