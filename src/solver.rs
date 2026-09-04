//! Top-level orchestration: dispatch an LP straight to the bounded-variable
//! dual revised simplex (`simplex::solve_lp_dual`) and reconstruct the
//! objective value. This replaces the original IP-PMM (PIQP-style
//! interior point) path now that the from-scratch simplex engine (Stages
//! 1-3: Markowitz/Forrest-Tomlin sparse LU, EXPAND anti-cycling, primal
//! and dual steepest-edge pricing) is implemented and verified —
//! `interior_point` (+ its submodules) remains in the crate, unused, in
//! case that path is wanted again later.

use crate::simplex;
use crate::types::{ConstraintRow, Objective, SolveResult, Status, VariableData};

pub fn solve_lp(
    variables: &[VariableData],
    objective: &Objective,
    constraints: &[ConstraintRow],
) -> SolveResult {
    let result = simplex::solve_lp_dual(variables, objective, constraints);

    match result.status {
        Status::Infeasible => SolveResult {
            status: Status::Infeasible,
            objective: None,
            x: None,
            node_limit_hit: false,
        },
        Status::Unbounded => SolveResult {
            status: Status::Unbounded,
            objective: None,
            x: None,
            node_limit_hit: false,
        },
        Status::Optimal => {
            let x = result.x.unwrap();
            let obj_val = objective.expr.constant
                + objective
                    .expr
                    .coeffs
                    .iter()
                    .map(|(&j, &c)| c * x[j])
                    .sum::<f64>();
            SolveResult {
                status: Status::Optimal,
                objective: Some(obj_val),
                x: Some(x),
                node_limit_hit: false,
            }
        }
    }
}
