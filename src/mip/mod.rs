//! 整数変数を含むモデルのための分枝限定法 (計画は `docs/branch_and_cut_plan.md`)。
//!
//! - [`lp`]: 状態を保持する LP エンジン (境界の変更・行の追加削除・warm start)。
//! - [`problem`]: 最小化形の問題データ。
//! - [`domain`]: 変数の定義域、変更の記録と巻き戻し、制約伝播。
//! - [`queue`]: 未処理ノードの待ち行列。
//! - [`pseudocost`]: pseudocost と分枝スコア。
//! - [`solver`]: 探索の本体 (reliability 分岐、plunge、暫定解による枝刈り)。

/// 分枝切除法のための、状態を保持する LP エンジン。
pub(crate) mod lp;
pub(crate) mod problem;
pub(crate) mod domain;
pub(crate) mod queue;
pub(crate) mod pseudocost;
pub(crate) mod solver;

use crate::solver::solve_lp;
use crate::types::{ConstraintRow, LpOptions, Objective, RootSolver, SolveResult, Status, VarType, VariableData};
use problem::MipProblem;
use solver::{MipParams, MipStatus};

/// (混合) 整数計画問題を解く。整数変数が 1 つもなければ `solve_lp` を 1 回呼ぶだけ。
pub fn solve_mip(
    variables: &[VariableData],
    objective: &Objective,
    constraints: &[ConstraintRow],
    root_solver: RootSolver,
    opts: LpOptions,
) -> SolveResult {
    let has_discrete = variables.iter().any(|v| v.vtype != VarType::Continuous);
    if !has_discrete {
        return solve_lp(variables, objective, constraints, root_solver, opts);
    }
    let p = MipProblem::from_model(variables, objective, constraints);
    let env_f64 = |v: Option<&str>| v.and_then(|s| s.parse::<f64>().ok());
    let d = MipParams::default();
    let params = MipParams {
        verbose: env_str!("ENOMOTO_MIP_LOG").is_some(),
        rel_gap: env_f64(env_str!("ENOMOTO_MIP_REL_GAP")).unwrap_or(d.rel_gap),
        time_limit: env_f64(env_str!("ENOMOTO_MIP_TIME_LIMIT")).unwrap_or(d.time_limit),
        node_limit: env_f64(env_str!("ENOMOTO_MIP_NODE_LIMIT")).map(|v| v as u64).unwrap_or(d.node_limit),
        ..d
    };
    let r = solver::solve(&p, params);
    let objective_value = r.objective.map(|z| p.sense_sign * z);
    match r.status {
        MipStatus::Optimal => SolveResult { status: Status::Optimal, objective: objective_value, x: r.x, node_limit_hit: false },
        MipStatus::NodeLimit | MipStatus::TimeLimit if r.x.is_some() => {
            SolveResult { status: Status::Optimal, objective: objective_value, x: r.x, node_limit_hit: true }
        }
        MipStatus::Infeasible => SolveResult { status: Status::Infeasible, objective: None, x: None, node_limit_hit: false },
        MipStatus::Unbounded => SolveResult { status: Status::Unbounded, objective: None, x: None, node_limit_hit: false },
        MipStatus::InfeasibleOrUnbounded => {
            SolveResult { status: Status::InfeasibleOrUnbounded, objective: None, x: None, node_limit_hit: false }
        }
        _ => SolveResult { status: Status::NotSolved, objective: None, x: None, node_limit_hit: matches!(r.status, MipStatus::NodeLimit) },
    }
}
