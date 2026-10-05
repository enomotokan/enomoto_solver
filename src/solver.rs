//! LP 求解の最上位の振り分け。`root_solver` (`types::RootSolver`、Python の
//! `Model.solve(root_solver=...)`) で選ばれたエンジンに LP を渡し、
//! 得られた解から目的関数値 (定数項込み) を計算して `SolveResult` にまとめる。
//! 既定は `Simplex` (傾き・切片二段解法)。`Interior` (IP-PMM 内点法) は明示指定時のみ。

use crate::interior_point;
use crate::simplex;
use crate::types::{ConstraintRow, LpOptions, Objective, RootSolver, SolveResult, Status, VariableData};

/// LP を 1 回解き、状態・目的関数値・解ベクトルを `SolveResult` で返す。
/// `Optimal` 以外では `objective`/`x` は `None`。`node_limit_hit` は常に `false`
/// (MIP の打ち切りは `mip::solve_mip` 側で設定する)。
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
        RootSolver::IpmCrossover => {
            let result = simplex::solve_lp_dual_with(variables, objective, constraints, LpOptions { ipm_crossover: true, ..opts });
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
            // 目的関数値 = 定数項 + Σ c_j x_j (元の変数空間で計算)
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
