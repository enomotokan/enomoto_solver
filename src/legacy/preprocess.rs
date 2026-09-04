//! Preprocessing: turns the user-facing model (variables with arbitrary
//! bounds, constraints in `<=`/`>=`/`==` form) into standard form suitable
//! for the simplex engine: every standardized variable is `>= 0`, and every
//! row has a non-negative right-hand side.

use crate::csr::CsrMatrix;
use crate::types::{ConstraintRow, LinearExpr, RowSense, VariableData};

/// How to recover the value of an original-space variable from the
/// standardized (all-non-negative) variable space.
#[derive(Debug, Clone, Copy)]
pub enum VarRecon {
    /// x = shift + y, y >= 0
    Shifted { col: usize, shift: f64 },
    /// x = base - y, y >= 0   (used when only an upper bound is finite)
    Flipped { col: usize, base: f64 },
    /// x = pos - neg, pos, neg >= 0  (free variable)
    Split { pos: usize, neg: usize },
}

impl VarRecon {
    pub fn value(&self, y: &[f64]) -> f64 {
        match *self {
            VarRecon::Shifted { col, shift } => shift + y[col],
            VarRecon::Flipped { col, base } => base - y[col],
            VarRecon::Split { pos, neg } => y[pos] - y[neg],
        }
    }
}

pub struct StandardForm {
    pub csr: CsrMatrix,
    pub rhs: Vec<f64>,
    pub row_sense: Vec<RowSense>,
    pub n_std_vars: usize,
    pub recon: Vec<VarRecon>,
}

/// Assigns standardized columns/reconstruction rules for every original
/// variable, and returns any extra rows needed to encode finite upper
/// bounds (`y <= ub - lb`) in variables that have both bounds finite.
fn build_recon(variables: &[VariableData]) -> (Vec<VarRecon>, usize, Vec<Vec<(usize, f64)>>, Vec<RowSense>, Vec<f64>) {
    let mut recon = Vec::with_capacity(variables.len());
    let mut next_col = 0usize;
    let mut extra_rows = Vec::new();
    let mut extra_sense = Vec::new();
    let mut extra_rhs = Vec::new();

    for v in variables {
        let lb_finite = v.lb.is_finite();
        let ub_finite = v.ub.is_finite();
        match (lb_finite, ub_finite) {
            (true, true) => {
                let col = next_col;
                next_col += 1;
                recon.push(VarRecon::Shifted { col, shift: v.lb });
                let width = v.ub - v.lb;
                extra_rows.push(vec![(col, 1.0)]);
                extra_sense.push(RowSense::Le);
                extra_rhs.push(width.max(0.0));
            }
            (true, false) => {
                let col = next_col;
                next_col += 1;
                recon.push(VarRecon::Shifted { col, shift: v.lb });
            }
            (false, true) => {
                let col = next_col;
                next_col += 1;
                recon.push(VarRecon::Flipped { col, base: v.ub });
            }
            (false, false) => {
                let pos = next_col;
                let neg = next_col + 1;
                next_col += 2;
                recon.push(VarRecon::Split { pos, neg });
            }
        }
    }

    (recon, next_col, extra_rows, extra_sense, extra_rhs)
}

/// Expands a linear expression given in original-variable space into the
/// standardized column space, returning the standardized coefficients and
/// the constant contribution that must be moved to the right-hand side.
pub fn expand_expr(expr: &LinearExpr, recon: &[VarRecon]) -> (Vec<(usize, f64)>, f64) {
    let mut out = Vec::with_capacity(expr.coeffs.len());
    let mut constant_shift = 0.0;
    for (&j, &a) in expr.coeffs.iter() {
        match recon[j] {
            VarRecon::Shifted { col, shift } => {
                out.push((col, a));
                constant_shift += a * shift;
            }
            VarRecon::Flipped { col, base } => {
                out.push((col, -a));
                constant_shift += a * base;
            }
            VarRecon::Split { pos, neg } => {
                out.push((pos, a));
                out.push((neg, -a));
            }
        }
    }
    (out, constant_shift)
}

pub fn standardize(variables: &[VariableData], constraints: &[ConstraintRow]) -> StandardForm {
    let (recon, n_std_vars, mut rows, mut sense, mut rhs) = build_recon(variables);

    for c in constraints {
        let (mut std_coeffs, constant_shift) = expand_expr(&c.expr, &recon);
        let mut row_rhs = c.rhs - c.expr.constant - constant_shift;
        let mut row_sense = c.sense;

        if row_rhs < 0.0 {
            for (_, v) in std_coeffs.iter_mut() {
                *v = -*v;
            }
            row_rhs = -row_rhs;
            row_sense = row_sense.flip();
        }

        rows.push(std_coeffs);
        sense.push(row_sense);
        rhs.push(row_rhs);
    }

    let csr = CsrMatrix::from_rows(&rows, n_std_vars);
    StandardForm {
        csr,
        rhs,
        row_sense: sense,
        n_std_vars,
        recon,
    }
}
