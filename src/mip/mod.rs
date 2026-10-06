//! 整数変数を含むモデルのための、単純な深さ優先の分枝限定法。
//! 各ノードでは変数境界を締めた LP 緩和問題を最初から解き直す (ウォームスタートなし)。


/// 分枝切除法のための、状態を保持する LP エンジン。
pub(crate) mod lp;
use crate::solver::solve_lp;
use crate::types::{ConstraintRow, LpOptions, Objective, RootSolver, Sense, SolveResult, Status, VarType, VariableData};
use std::collections::HashMap;
use crate::params::mip::{BOUND_INFEAS_TOL, INT_TOL, MAX_NODES, OBJ_EPS};

/// 分枝限定木の 1 ノード: 元の問題のうち一部の変数の境界をさらに締めたもの。
struct Node {
    /// 変数番号 `j` → そのノードで追加する境界 `(lo, hi)`。処理時に元の境界と
    /// 交差を取る。エントリのない変数は元の境界のまま。
    overrides: HashMap<usize, (f64, f64)>,
}

/// `candidate` が `current` より `OBJ_EPS` を超えて厳密に良い (`sense` の向きで) なら true。
fn is_better(sense: Sense, candidate: f64, current: f64) -> bool {
    match sense {
        Sense::Minimize => candidate < current - OBJ_EPS,
        Sense::Maximize => candidate > current + OBJ_EPS,
    }
}

/// 緩和解 `x` で最も小数的な (半整数に最も近い) 整数変数の番号を返す。
/// すべての整数変数が整数から `INT_TOL` 以内なら `None` (= この緩和解はそのまま
/// MIP の実行可能解)。最も小数的な変数で分枝すると探索空間が速く縮みやすい。
fn most_fractional(variables: &[VariableData], x: &[f64]) -> Option<usize> {
    // これまでの最良候補 (変数番号, 小数度)
    let mut best: Option<(usize, f64)> = None;
    for (j, v) in variables.iter().enumerate() {
        if v.vtype == VarType::Continuous {
            continue;
        }
        let val = x[j];
        let frac = val - val.floor();
        // 小数度 = 最も近い整数までの距離 (0 〜 0.5)
        let score = frac.min(1.0 - frac);
        if score > INT_TOL {
            if best.map_or(true, |(_, bs)| score > bs) {
                best = Some((j, score));
            }
        }
    }
    best.map(|(j, _)| j)
}

/// (混合) 整数計画問題を深さ優先の分枝限定法で解く。整数変数が 1 つもなければ
/// `solve_lp` を 1 回呼ぶだけ。そうでなければ、ノードを取り出して LP 緩和を解き、
/// 緩和が実行不能か暫定解より良くならなければ枝刈り、全整数変数が整数なら暫定解として
/// 採用、そうでなければ最も小数的な変数で `floor`/`ceil` の 2 つの子ノードに分枝する。
///
/// ノード数が `MAX_NODES` を超えたらその時点の最良暫定解を `node_limit_hit: true` で返す。
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

    // 未処理ノードのスタック (深さ優先)
    let mut stack: Vec<Node> = vec![Node {
        overrides: HashMap::new(),
    }];
    // 暫定解 (目的関数値, 解ベクトル)
    let mut incumbent: Option<(f64, Vec<f64>)> = None;
    let mut node_count = 0usize;
    let mut node_limit_hit = false;

    while let Some(node) = stack.pop() {
        node_count += 1;
        if node_count > MAX_NODES {
            node_limit_hit = true;
            break;
        }

        // このノードでの実効的な変数境界
        let mut vars_eff = variables.to_vec();
        let mut bounds_infeasible = false;
        for (&j, &(lo, hi)) in node.overrides.iter() {
            let new_lb = vars_eff[j].lb.max(lo);
            let new_ub = vars_eff[j].ub.min(hi);
            if new_lb > new_ub + BOUND_INFEAS_TOL {
                bounds_infeasible = true;
                break;
            }
            vars_eff[j].lb = new_lb;
            vars_eff[j].ub = new_ub;
        }
        if bounds_infeasible {
            continue;
        }

        // `Optimal` 以外の緩和はどれも枝刈りするので、実行不能/非有界を区別しない
        // 安価な既定オプションで十分。
        let relax = solve_lp(&vars_eff, objective, constraints, root_solver, LpOptions::default());
        if relax.status != Status::Optimal {
            continue;
        }
        let relax_obj = relax.objective.unwrap();
        if let Some((inc_obj, _)) = &incumbent {
            if !is_better(objective.sense, relax_obj, *inc_obj) {
                continue;
            }
        }

        let x = relax.x.unwrap();
        match most_fractional(variables, &x) {
            None => {
                let better = incumbent
                    .as_ref()
                    .map_or(true, |(inc, _)| is_better(objective.sense, relax_obj, *inc));
                if better {
                    // `most_fractional` は整数から INT_TOL 以内であることしか保証しないので、
                    // 暫定解として記録する前に整数変数を丸める。
                    let mut x_rounded = x;
                    for (j, v) in variables.iter().enumerate() {
                        if v.vtype != VarType::Continuous {
                            x_rounded[j] = x_rounded[j].round();
                        }
                    }
                    let obj_rounded = objective.expr.constant
                        + objective
                            .expr
                            .coeffs
                            .iter()
                            .map(|(&j, &coeff)| coeff * x_rounded[j])
                            .sum::<f64>();
                    incumbent = Some((obj_rounded, x_rounded));
                }
            }
            Some(j) => {
                let val = x[j];
                let floor_v = val.floor();
                let ceil_v = val.ceil();

                // このノードでの変数 j の現在の境界: 既に上書きがあればそれ、
                // この経路で初めて j を分枝するなら元の境界。
                let current_bounds = |m: &HashMap<usize, (f64, f64)>| {
                    *m.get(&j).unwrap_or(&(variables[j].lb, variables[j].ub))
                };

                // 子ノード 1: x_j <= floor(val)
                let mut floor_overrides = node.overrides.clone();
                let (lo, hi) = current_bounds(&floor_overrides);
                floor_overrides.insert(j, (lo, hi.min(floor_v)));

                // 子ノード 2: x_j >= ceil(val)
                let mut ceil_overrides = node.overrides.clone();
                let (lo, hi) = current_bounds(&ceil_overrides);
                ceil_overrides.insert(j, (lo.max(ceil_v), hi));

                stack.push(Node { overrides: floor_overrides });
                stack.push(Node { overrides: ceil_overrides });
            }
        }
    }

    match incumbent {
        Some((obj, x)) => SolveResult {
            status: Status::Optimal,
            objective: Some(obj),
            x: Some(x),
            node_limit_hit,
        },
        // 整数解が 1 つも見つからなかった。理由を分類するため、制限なしの根の緩和を
        // もう一度解く: 根が実行不能/非有界なら MIP も同じ (境界を締めても実行可能域は
        // 縮むだけ)。根の緩和が最適なのに整数解に届かなかったなら、MIP は整数実行
        // 可能解を持たない (実行不能) と報告する。
        None => {
            let root = solve_lp(variables, objective, constraints, root_solver, opts);
            match root.status {
                Status::Infeasible => root,
                Status::Unbounded => root,
                Status::InfeasibleOrUnbounded => root,
                Status::NotSolved => root,
                Status::Optimal => SolveResult {
                    status: Status::Infeasible,
                    objective: None,
                    x: None,
                    node_limit_hit,
                },
            }
        }
    }
}
