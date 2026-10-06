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
pub(crate) mod lp_api;
pub(crate) mod problem;
pub(crate) mod domain;
pub(crate) mod queue;
pub(crate) mod pseudocost;
pub(crate) mod solver;
pub(crate) mod heuristics;
pub(crate) mod cuts;
pub(crate) mod separation;

use crate::solver::solve_lp;
use crate::types::{ConstraintRow, LpOptions, MipSettings, MipSummary, Objective, RootSolver, SolveResult, Status, VarType, VariableData};
use problem::MipProblem;
use solver::{MipParams, MipStatus};

/// (混合) 整数計画問題を解く。整数変数が 1 つもなければ `solve_lp` を 1 回呼ぶだけ。
pub fn solve_mip(
    variables: &[VariableData],
    objective: &Objective,
    constraints: &[ConstraintRow],
    root_solver: RootSolver,
    opts: LpOptions,
    settings: MipSettings,
) -> SolveResult {
    let has_discrete = variables.iter().any(|v| v.vtype != VarType::Continuous);
    if !has_discrete {
        return solve_lp(variables, objective, constraints, root_solver, opts);
    }
    let p = MipProblem::from_model(variables, objective, constraints);
    // 既定値 < 環境変数 < 引数の順に優先する。
    let env_f64 = |v: Option<&str>| v.and_then(|s| s.parse::<f64>().ok());
    let d = MipParams::default();
    let params = MipParams {
        verbose: env_str!("ENOMOTO_MIP_LOG").is_some(),
        rel_gap: settings.rel_gap.or(env_f64(env_str!("ENOMOTO_MIP_REL_GAP"))).unwrap_or(d.rel_gap),
        time_limit: settings.time_limit.or(env_f64(env_str!("ENOMOTO_MIP_TIME_LIMIT"))).unwrap_or(d.time_limit),
        node_limit: settings.node_limit.or(env_f64(env_str!("ENOMOTO_MIP_NODE_LIMIT")).map(|v| v as u64)).unwrap_or(d.node_limit),
        ..d
    };
    // 前処理 (整数を考慮した縮約) をかけて解き、解を元の空間に戻す。戻した解が元の問題で
    // 実行可能でなければ (前処理の誤りの安全網)、前処理なしで解き直す。
    let presolved = if env_str!("ENOMOTO_MIP_NO_PRESOLVE").is_some() { None } else { presolve_mip(variables, &p) };
    let r = match presolved {
        Some(Presolved::Infeasible) => solver::MipResult { status: MipStatus::Infeasible, x: None, objective: None, best_bound: f64::INFINITY, nodes: 0, lp_iterations: 0 },
        Some(Presolved::Reduced { prob, postsolve, scaling }) => {
            if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_SOL") {
                let x0: Vec<f64> = std::fs::read_to_string(f).unwrap().lines().map(|l| l.trim().parse().unwrap()).collect();
                let xd: Vec<f64> = (0..prob.n).map(|j| if prob.col_lo[j] == prob.col_up[j] { prob.col_lo[j] } else { x0[j] }).collect();
                eprintln!("MIP_DEBUG_SOL: reduced objective {} feasible {}", prob.objective(&xd), prob.is_feasible(&xd, 1e-6));
                solver::DEBUG_SOL.with(|d| *d.borrow_mut() = Some(xd));
            }
            let mut r = solver::solve(&prob, params);
            let mut ok = true;
            if let Some(xr) = r.x.take() {
                let mut x = xr.clone();
                for step in postsolve.iter().rev() {
                    step.apply(&mut x);
                }
                let x = crate::presolve::scaling::unscale_x(&scaling, &x);
                if p.is_feasible(&x, 1e-6) {
                    let z_orig = p.objective(&x);
                    let shift = z_orig - prob.objective(&xr);
                    r.best_bound += shift;
                    r.objective = Some(z_orig);
                    r.x = Some(x);
                } else {
                    if params.verbose {
                        eprintln!("MIP: the postsolved solution is infeasible in the original problem; solving without presolve");
                    }
                    ok = false;
                }
            } else if r.status != MipStatus::Infeasible {
                // 暫定解がないと目的値の定数のずれが分からないので下界は報告しない
                r.best_bound = f64::NEG_INFINITY;
            }
            if ok { r } else { solver::solve(&p, params) }
        }
        None => solver::solve(&p, params),
    };
    let s = p.sense_sign;
    let objective_value = r.objective.map(|z| s * z);
    let gap = match r.objective {
        Some(z) if r.best_bound.is_finite() => ((z - r.best_bound).max(0.0)) / z.abs().max(1.0),
        Some(_) if r.status == MipStatus::Optimal => 0.0,
        _ => f64::INFINITY,
    };
    let summary = MipSummary { best_bound: s * r.best_bound, gap, nodes: r.nodes, lp_iterations: r.lp_iterations };
    let status = match r.status {
        MipStatus::Optimal => Status::Optimal,
        MipStatus::Infeasible => Status::Infeasible,
        MipStatus::Unbounded => Status::Unbounded,
        MipStatus::InfeasibleOrUnbounded => Status::InfeasibleOrUnbounded,
        MipStatus::TimeLimit => Status::TimeLimit,
        MipStatus::NodeLimit => Status::NodeLimit,
        MipStatus::NotSolved => Status::NotSolved,
    };
    SolveResult { status, objective: objective_value, x: r.x, node_limit_hit: r.status == MipStatus::NodeLimit, mip: Some(summary) }
}

/// 前処理の結果。
enum Presolved {
    Infeasible,
    Reduced { prob: MipProblem, postsolve: Vec<crate::presolve::PostsolveStep>, scaling: crate::presolve::scaling::Scaling },
}

/// 整数を考慮した前処理をかけた問題を作る。前処理が何も減らさなければ `None` (元の問題で解く)。
fn presolve_mip(variables: &[VariableData], p: &MipProblem) -> Option<Presolved> {
    use crate::params::simplex::{PRESOLVE_ROUNDS, PROPAGATION_PASSES, ROWSINGLETON_COLSINGLETON_INNER_ROUNDS};
    let n = p.n;
    // 元の制約 (境界は丸め済みの MipProblem の値を使う)
    let vars: Vec<VariableData> = (0..n).map(|j| VariableData { vtype: variables[j].vtype, lb: p.col_lo[j], ub: p.col_up[j] }).collect();
    let mut cons: Vec<ConstraintRow> = Vec::with_capacity(p.m);
    for i in 0..p.m {
        let expr = crate::types::LinearExpr { coeffs: p.rows[i].iter().cloned().collect(), constant: 0.0 };
        let (lo, up) = (p.row_lo[i], p.row_up[i]);
        if lo == up {
            cons.push(ConstraintRow { expr, sense: crate::types::RowSense::Eq, rhs: lo });
        } else {
            if lo.is_finite() {
                cons.push(ConstraintRow { expr: expr.clone(), sense: crate::types::RowSense::Ge, rhs: lo });
            }
            if up.is_finite() {
                cons.push(ConstraintRow { expr, sense: crate::types::RowSense::Le, rhs: up });
            }
        }
    }
    let (a, b, g, h) = crate::presolve::build_a_g(&vars, &cons);
    let pre = crate::presolve::run_extended_mip(n, &a, &b, &g, &h, &p.cost, &p.is_int, PROPAGATION_PASSES, PRESOLVE_ROUNDS, ROWSINGLETON_COLSINGLETON_INNER_ROUNDS);
    if pre.infeasible {
        return Some(Presolved::Infeasible);
    }
    if pre.unbounded {
        return None;
    }
    let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut row_lo = Vec::new();
    let mut row_up = Vec::new();
    for (i, r) in crate::sparse::csr_rows(&pre.a).into_iter().enumerate() {
        if r.is_empty() {
            continue;
        }
        rows.push(r);
        row_lo.push(pre.b[i]);
        row_up.push(pre.b[i]);
    }
    for (r, &rhs) in pre.real_rows.iter().zip(&pre.real_rhs) {
        if r.is_empty() {
            continue;
        }
        rows.push(r.clone());
        row_lo.push(f64::NEG_INFINITY);
        row_up.push(rhs);
    }
    let fixed = (0..n).filter(|&j| pre.lb[j] == pre.ub[j]).count();
    if env_str!("ENOMOTO_MIP_LOG").is_some() {
        eprintln!("MIP: presolve: rows {} -> {}, free columns {} -> {}, postsolve steps {}", p.m, rows.len(), (0..n).filter(|&j| p.col_lo[j] < p.col_up[j]).count(), n - fixed, pre.postsolve_log.len());
    }
    // 代入消去で目的関数に生じた定数を offset に入れる (後処理は縮約後の点について
    // アフィンなので、任意の 1 点で元の目的値との差を測ればよい)。定数が抜けていると
    // 相対ギャップによる終了判定や目的値の刻みによる打ち切りが誤ったスケールで働く。
    let x_red: Vec<f64> = (0..n).map(|j| 0.0f64.clamp(pre.lb[j].min(pre.ub[j]), pre.ub[j].max(pre.lb[j]))).collect();
    let mut x_orig = x_red.clone();
    for step in pre.postsolve_log.iter().rev() {
        step.apply(&mut x_orig);
    }
    let x_orig = crate::presolve::scaling::unscale_x(&pre.scaling, &x_orig);
    let shift = p.objective(&x_orig) - (pre.c.iter().zip(&x_red).map(|(c, x)| c * x).sum::<f64>() + p.offset);
    let offset = if shift.is_finite() { p.offset + shift } else { p.offset };
    let prob = MipProblem::from_rows(pre.lb.clone(), pre.ub.clone(), pre.c.clone(), offset, p.sense_sign, p.is_int.clone(), rows, row_lo, row_up);
    Some(Presolved::Reduced { prob, postsolve: pre.postsolve_log, scaling: pre.scaling })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 代入消去で生じる目的関数の定数が縮約後の offset に入ること (binkar10_1 の誤答の回帰テスト:
    /// 定数が抜けると相対ギャップの判定が誤ったスケールで働き、最適でない解で終了していた)。
    #[test]
    fn presolve_keeps_objective_constant() {
        // y + x1 + x2 = 1000 (y は連続で 1 行にしか現れないので消去される)、x1 + x2 <= 15、
        // min 1000 y + x1 + 2 x2 (y を消去すると定数 1e6 が生じる)
        let variables = vec![
            VariableData { vtype: VarType::Continuous, lb: 0.0, ub: f64::INFINITY },
            VariableData { vtype: VarType::Integer, lb: 0.0, ub: 10.0 },
            VariableData { vtype: VarType::Integer, lb: 0.0, ub: 10.0 },
        ];
        let rows = vec![vec![(0, 1.0), (1, 1.0), (2, 1.0)], vec![(1, 1.0), (2, 1.0)]];
        let p = MipProblem::from_rows(vec![0.0, 0.0, 0.0], vec![f64::INFINITY, 10.0, 10.0], vec![1000.0, 1.0, 2.0], 0.0, 1.0, vec![false, true, true], rows, vec![1000.0, f64::NEG_INFINITY], vec![1000.0, 15.0]);
        let Some(Presolved::Reduced { prob, postsolve, scaling }) = presolve_mip(&variables, &p) else { panic!("expected a reduced problem") };
        for (x1, x2) in [(3.0, 4.0), (10.0, 5.0), (0.0, 0.0)] {
            let mut xr: Vec<f64> = (0..prob.n).map(|j| prob.col_lo[j].max(0.0).min(prob.col_up[j])).collect();
            for (j, v) in [(1, x1), (2, x2)] {
                if prob.col_lo[j] < prob.col_up[j] {
                    xr[j] = v;
                }
            }
            let mut x = xr.clone();
            for step in postsolve.iter().rev() {
                step.apply(&mut x);
            }
            let x = crate::presolve::scaling::unscale_x(&scaling, &x);
            let (zr, z) = (prob.objective(&xr), p.objective(&x));
            assert!((zr - z).abs() <= 1e-6 * z.abs().max(1.0), "reduced objective {zr} != original {z}");
        }
    }
}
