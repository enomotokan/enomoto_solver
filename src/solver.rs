//! Top-level orchestration: dispatch an LP to whichever engine
//! `root_solver` selects (`types::RootSolver` — `Model.solve`'s
//! Python-facing `root_solver` argument) and reconstruct the objective
//! value. `Simplex` (`simplex::solve_lp_dual`, the bounded-variable dual
//! revised simplex) is the default, having replaced the original IP-PMM
//! (PIQP-style interior point) path as the primary engine; `Interior`
//! (`interior_point::solve_lp`) is kept fully reachable rather than
//! deleted, both as a fallback and so the two independent implementations
//! can be run against the same input and compared directly.

use crate::interior_point;
use crate::simplex;
use crate::types::{ConstraintRow, LpOptions, Objective, RootSolver, SolveResult, Status, VariableData};

pub fn solve_lp(
    variables: &[VariableData],
    objective: &Objective,
    constraints: &[ConstraintRow],
    root_solver: RootSolver,
    opts: LpOptions,
) -> SolveResult {
    let (status, x) = match root_solver {
        RootSolver::Simplex => {
            let result = simplex::solve_lp_dual_with(variables, objective, constraints, opts);
            (result.status, result.x)
        }
        RootSolver::Interior => {
            let result = interior_point::solve_lp(variables, objective, constraints);
            (result.status, result.x)
        }
    };

    match status {
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
        Status::InfeasibleOrUnbounded => SolveResult {
            status: Status::InfeasibleOrUnbounded,
            objective: None,
            x: None,
            node_limit_hit: false,
        },
        Status::NotSolved => SolveResult {
            status: Status::NotSolved,
            objective: None,
            x: None,
            node_limit_hit: false,
        },
        Status::Optimal => {
            let x = x.unwrap();
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
