//! Builds the QP standard form PIQP's algorithm expects,
//!
//!   minimize   0.5 x^T P x + c^T x
//!   subject to A x = b
//!              G x <= h
//!
//! directly from the model's variables/objective/constraints — no
//! shift/split preprocessing (that was only ever needed by the simplex
//! method's ">= 0" requirement; the interior point method below handles
//! arbitrary bounds natively). Variable bounds become extra rows of `G`.
//! `A` and `G` are assembled as faer's native CSR (`SparseRowMat`) and stay
//! sparse all the way into the solver — no dense conversion.

use super::kkt::{csr_from_rows, Csr};
use crate::types::{ConstraintRow, RowSense, VariableData};

pub struct QpStd {
    pub n: usize,
    pub c: Vec<f64>,
    pub a: Csr,
    pub b: Vec<f64>,
    pub g: Csr,
    pub h: Vec<f64>,
}

pub fn build(variables: &[VariableData], obj_coeffs_for_min: &[(usize, f64)], constraints: &[ConstraintRow]) -> QpStd {
    let n = variables.len();

    let mut c = vec![0.0; n];
    for &(j, v) in obj_coeffs_for_min {
        c[j] += v;
    }

    let mut a_rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut b: Vec<f64> = Vec::new();
    let mut g_rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut h: Vec<f64> = Vec::new();

    for row in constraints {
        let terms: Vec<(usize, f64)> = row.expr.coeffs.iter().map(|(&j, &v)| (j, v)).collect();
        let rhs = row.rhs - row.expr.constant;
        match row.sense {
            RowSense::Eq => {
                a_rows.push(terms);
                b.push(rhs);
            }
            RowSense::Le => {
                g_rows.push(terms);
                h.push(rhs);
            }
            RowSense::Ge => {
                g_rows.push(terms.into_iter().map(|(j, v)| (j, -v)).collect());
                h.push(-rhs);
            }
        }
    }

    for (j, v) in variables.iter().enumerate() {
        if v.ub.is_finite() {
            g_rows.push(vec![(j, 1.0)]);
            h.push(v.ub);
        }
        if v.lb.is_finite() {
            g_rows.push(vec![(j, -1.0)]);
            h.push(-v.lb);
        }
    }

    let a = csr_from_rows(&a_rows, n);
    let g = csr_from_rows(&g_rows, n);

    QpStd { n, c, a, b, g, h }
}
