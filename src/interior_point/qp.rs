//! Builds the QP standard form PIQP's algorithm expects,
//!
//!   minimize   0.5 x^T P x + c^T x
//!   subject to A x = b
//!              G x <= h
//!
//! directly from the model's variables/objective/constraints — no
//! shift/split preprocessing (that was only ever needed by the simplex
//! method's ">= 0" requirement; the interior point method below handles
//! arbitrary bounds natively). Variable bounds become extra rows of `G`
//! (via the shared [`crate::presolve::build_a_g`], also used by
//! `simplex.rs`'s own presolve entry point). `A` and `G` are assembled as
//! faer's native CSR (`SparseRowMat`) and stay sparse all the way into the
//! solver — no dense conversion.

use super::kkt::Csr;
use crate::presolve::build_a_g;
use crate::types::{ConstraintRow, VariableData};

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

    let (a, b, g, h) = build_a_g(variables, constraints);

    QpStd { n, c, a, b, g, h }
}
