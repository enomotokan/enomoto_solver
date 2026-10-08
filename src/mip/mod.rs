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
pub(crate) mod heur_scip;
pub(crate) mod heur_ipm;
pub(crate) mod cuts;
pub(crate) mod separation;
pub(crate) mod zerohalf;
pub(crate) mod clique;

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
    let r = solve_problem(&p, params, env_str!("ENOMOTO_MIP_NO_PRESOLVE").is_none());
    if env_str!("ENOMOTO_MIP_XPROF").is_some() {
        crate::simplex::slope_intercept_dual::xprof("tail");
        for (l, ns) in crate::simplex::slope_intercept_dual::xprof_take() {
            eprintln!("XPROF {l:10} {:10.1} ms", ns as f64 / 1e6);
        }
    }
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

/// `p` を (`use_presolve` なら MIP 前処理をかけてから) 分枝限定法で解き、解を `p` の空間に戻す。
/// 戻した解が `p` で実行可能でなければ (前処理の誤りの安全網)、前処理なしで解き直す。
/// サブ MIP (RENS/RINS) もこれで解く。
thread_local! {
    /// 次の [`solve_problem`] に渡す最初の暫定解 (`p` の空間)。開始時に取り出し、前処理後の空間に移して
    /// [`solver::START_SOL`] に置く (再スタートで暫定解を引き継ぐ)。
    pub(crate) static RESTART_SOL: std::cell::RefCell<Option<Vec<f64>>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn solve_problem(p: &MipProblem, params: MipParams, use_presolve: bool) -> solver::MipResult {
    let start = RESTART_SOL.with(|s| s.borrow_mut().take());
    let presolved = if use_presolve { presolve_mip(p, params.verbose) } else { None };
    // 前処理は列の番号を保ち、列の尺度 d を掛ける (x = d x')。前処理後の空間に移して実行可能なら渡す
    // (代入で消えた列などで合わなければ渡さない)
    match (&presolved, start) {
        (Some(Presolved::Reduced { prob, scaling, .. }), Some(x)) if x.len() == prob.n => {
            let xr: Vec<f64> = x.iter().zip(&scaling.d).map(|(v, d)| v / d).collect();
            if prob.is_feasible(&xr, 1e-6) {
                solver::START_SOL.with(|s| *s.borrow_mut() = Some(xr));
            } else if params.verbose {
                eprintln!("MIP: the start solution is not feasible in the presolved problem; dropped");
            }
        }
        (None, Some(x)) if x.len() == p.n => solver::START_SOL.with(|s| *s.borrow_mut() = Some(x)),
        _ => {}
    }
    match presolved {
        Some(Presolved::Infeasible) => solver::MipResult { status: MipStatus::Infeasible, x: None, objective: None, best_bound: f64::INFINITY, nodes: 0, lp_iterations: 0 },
        Some(Presolved::Reduced { prob, postsolve, scaling }) => {
            if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_SOL").filter(|_| !params.submip) {
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
            if ok { r } else { solver::solve(p, params) }
        }
        None => solver::solve(p, params),
    }
}

/// 前処理の結果。
enum Presolved {
    Infeasible,
    Reduced { prob: MipProblem, postsolve: Vec<crate::presolve::PostsolveStep>, scaling: crate::presolve::scaling::Scaling },
}

/// 整数を考慮した前処理をかけた問題を作る。前処理が何も減らさなければ `None` (元の問題で解く)。
fn presolve_mip(p: &MipProblem, verbose: bool) -> Option<Presolved> {
    use crate::params::simplex::{PRESOLVE_ROUNDS, PROPAGATION_PASSES, ROWSINGLETON_COLSINGLETON_INNER_ROUNDS};
    let n = p.n;
    // 元の制約 (境界は丸め済みの MipProblem の値を使う)
    let vars: Vec<VariableData> = (0..n).map(|j| VariableData { vtype: if p.is_int[j] { VarType::Integer } else { VarType::Continuous }, lb: p.col_lo[j], ub: p.col_up[j] }).collect();
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
    // 係数の強化 (片側の行の 0-1 列の係数を、整数解を変えずに LP の緩和が締まる値に縮める)
    if env_str!("ENOMOTO_MIP_NO_COEF_TIGHTEN").is_none() {
        let is_int_now: Vec<bool> = p.is_int.clone();
        let tightened = tighten_coefficients(&mut rows, &mut row_lo, &mut row_up, &pre.lb, &pre.ub, &is_int_now);
        let normalized = normalize_integer_rows(&mut rows, &mut row_lo, &mut row_up, &is_int_now);
        if verbose && (tightened > 0 || normalized > 0) {
            eprintln!("MIP: presolve: tightened {tightened} coefficients, normalized {normalized} integer rows");
        }
    }
    // 係数が定数倍の関係にある行をまとめる (上限側と下限側の組は範囲制約 1 本に、同じ向きはきつい方に)。
    // 前処理は範囲制約を 2 本の不等式で表すので、そのままだと LP の行が倍になる
    let merged = if env_str!("ENOMOTO_MIP_NO_ROW_MERGE").is_none() { merge_parallel_rows(&mut rows, &mut row_lo, &mut row_up) } else { 0 };
    if verbose && merged > 0 {
        eprintln!("MIP: presolve: merged {merged} parallel rows");
    }
    let fixed = (0..n).filter(|&j| pre.lb[j] == pre.ub[j]).count();
    if verbose {
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

/// 係数の強化 (coefficient tightening、SCIP・HiGHS の前処理と同じ): 片側の行 `sum a_j x_j <= b` (`>=` の行は符号を
/// 反転して扱う) の最大活動量 `M` (有限のとき) について、幅 1 の整数列 `x_j` (`[l, l + 1]`) の係数を縮める。
///
/// - `a_j > 0` で `M - a_j < b` (x_j が下限なら行は常に満たされる): `δ = b - (M - a_j)` だけ `a_j` と `b` を減らす。
///   x_j が上限なら他の列への制約は変わらず、下限なら行は `M - a_j <= b - δ` で常に満たされる。
/// - `a_j < 0` で `M + a_j < b` (x_j が上限なら常に満たされる): `δ = b - (M + a_j)` だけ `a_j` を増やす (`b` はそのまま)。
///
/// どちらも整数解の集合を変えず、LP の緩和だけを締める (例: 0-1 列の `sum 21 y >= 2` は `sum 2 y >= 2`)。
/// 列の値は変えないので後処理は要らない。縮めた係数の数を返す。
fn tighten_coefficients(rows: &mut [Vec<(usize, f64)>], row_lo: &mut [f64], row_up: &mut [f64], lo: &[f64], up: &[f64], is_int: &[bool]) -> usize {
    let unit = |j: usize| is_int[j] && lo[j].is_finite() && up[j].is_finite() && (up[j] - lo[j] - 1.0).abs() <= 1e-9 && (lo[j] - lo[j].round()).abs() <= 1e-9;
    let mut count = 0;
    for i in 0..rows.len() {
        let (l, u) = (row_lo[i], row_up[i]);
        // 片側の行だけ (等式・範囲制約は両側の条件が絡むので扱わない)
        let sg = if u.is_finite() && !l.is_finite() {
            1.0
        } else if l.is_finite() && !u.is_finite() {
            -1.0
        } else {
            continue;
        };
        if !rows[i].iter().any(|&(j, _)| unit(j)) {
            continue;
        }
        // sg * row <= b
        let mut b = if sg > 0.0 { u } else { -l };
        let mut maxact = 0.0;
        let mut finite = true;
        for &(j, a) in &rows[i] {
            let a = sg * a;
            let v = if a > 0.0 { up[j] } else { lo[j] };
            if !v.is_finite() {
                finite = false;
                break;
            }
            maxact += a * v;
        }
        if !finite {
            continue;
        }
        let tol = 1e-9 * (1.0 + b.abs()).max(maxact.abs());
        let mut changed = false;
        for e in rows[i].iter_mut() {
            let j = e.0;
            if !unit(j) {
                continue;
            }
            let a = sg * e.1;
            if a > 0.0 {
                let d = b - (maxact - a);
                if d > tol && d < a {
                    e.1 = sg * (a - d);
                    b -= d;
                    // x_j の上限の寄与 a (u_j) が (a - d) (u_j) になり、右辺も d 減る。下限の分の移動は l_j が 0 でなければ
                    // 係数の変化 d に l_j を掛けた分だけ活動量が動くので、最大活動量は d * u_j 減る
                    maxact -= d * up[j];
                    b -= d * lo[j];
                    count += 1;
                    changed = true;
                }
            } else if a < 0.0 {
                let d = b - (maxact + a);
                if d > tol && d < -a {
                    e.1 = sg * (a + d);
                    // 最大活動量は x_j の下限での寄与 a l_j が (a + d) l_j になる
                    maxact += d * lo[j];
                    b += d * lo[j];
                    count += 1;
                    changed = true;
                }
            }
        }
        if changed {
            if sg > 0.0 {
                row_up[i] = b;
            } else {
                row_lo[i] = -b;
            }
        }
    }
    count
}

/// 整数列だけで係数が整数の片側の行を係数の最大公約数で割り、右辺を整数に丸める (`<=` は切り下げ、`>=` は切り上げ。
/// 整数解では左辺が整数なので成り立つ)。例: `2 y0 + 2 y1 >= 2` -> `y0 + y1 >= 1`。割った (または丸めた) 行の数を返す。
fn normalize_integer_rows(rows: &mut [Vec<(usize, f64)>], row_lo: &mut [f64], row_up: &mut [f64], is_int: &[bool]) -> usize {
    fn gcd(a: u64, b: u64) -> u64 {
        if b == 0 { a } else { gcd(b, a % b) }
    }
    let mut count = 0;
    for i in 0..rows.len() {
        let (l, u) = (row_lo[i], row_up[i]);
        if l.is_finite() == u.is_finite() || rows[i].is_empty() {
            continue;
        }
        if !rows[i].iter().all(|&(j, a)| is_int[j] && (a - a.round()).abs() <= 1e-9 && a.abs() < 1e9) {
            continue;
        }
        let g = rows[i].iter().fold(0u64, |g, &(_, a)| gcd(g, a.round().abs() as u64)) as f64;
        if g < 1.0 {
            continue;
        }
        let (nl, nu) = if u.is_finite() { (l, (u / g + 1e-9).floor()) } else { ((l / g - 1e-9).ceil(), u) };
        if g > 1.0 || (u.is_finite() && nu != u) || (l.is_finite() && nl != l) {
            for e in rows[i].iter_mut() {
                e.1 = (e.1 / g).round();
            }
            row_lo[i] = nl;
            row_up[i] = nu;
            count += 1;
        }
    }
    count
}

/// 係数が定数倍の関係にある行をまとめる。各行を最初の係数で割った形で分類し、同じ形の行の
/// `base x` の範囲の共通部分を、最初の行 (元の倍率) の範囲制約 1 本にする。共通部分が空 (実行不能) なら
/// その組はまとめない (伝播・LP が検出する)。まとめた (消した) 行の数を返す。
fn merge_parallel_rows(rows: &mut Vec<Vec<(usize, f64)>>, row_lo: &mut Vec<f64>, row_up: &mut Vec<f64>) -> usize {
    use std::collections::HashMap;
    let m = rows.len();
    let key_of = |r: &[(usize, f64)]| -> Option<(Vec<(usize, i64)>, f64)> {
        let s = r.first()?.1;
        if s == 0.0 {
            return None;
        }
        let mut k: Vec<(usize, i64)> = r.iter().map(|&(j, a)| (j, ((a / s) * 1e12).round() as i64)).collect();
        k.sort_unstable();
        Some((k, s))
    };
    // 形 -> (最初の行, その倍率, base x の下限, 上限)
    let mut groups: HashMap<Vec<(usize, i64)>, (usize, f64, f64, f64)> = HashMap::new();
    let mut keep = vec![true; m];
    let mut merged = 0usize;
    for i in 0..m {
        let Some((k, s)) = key_of(&rows[i]) else { continue };
        // row_lo <= s (base x) <= row_up -> base x の範囲
        let (bl, bu) = if s > 0.0 { (row_lo[i] / s, row_up[i] / s) } else { (row_up[i] / s, row_lo[i] / s) };
        match groups.get_mut(&k) {
            None => {
                groups.insert(k, (i, s, bl, bu));
            }
            Some(g) => {
                let (nl, nu) = (g.2.max(bl), g.3.min(bu));
                if nl > nu + 1e-9 * (1.0 + nl.abs().max(nu.abs())) {
                    continue; // 実行不能な組はまとめない
                }
                g.2 = nl;
                g.3 = nu;
                keep[i] = false;
                merged += 1;
            }
        }
    }
    if merged == 0 {
        return 0;
    }
    for (_, (i, s, bl, bu)) in groups {
        let (lo, up) = if s > 0.0 { (bl * s, bu * s) } else { (bu * s, bl * s) };
        row_lo[i] = lo;
        row_up[i] = up;
    }
    let mut k = 0;
    for i in 0..m {
        if keep[i] {
            rows.swap(k, i);
            row_lo.swap(k, i);
            row_up.swap(k, i);
            k += 1;
        }
    }
    rows.truncate(k);
    row_lo.truncate(k);
    row_up.truncate(k);
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 上限側と下限側 (符号の逆の倍) の行が範囲制約 1 本にまとまること
    #[test]
    fn tighten_coefficients_saturates_cover_row() {
        // sum 21 y >= 2 (0-1) -> 2 y0 + 2 y1 + 2 y2 >= 2
        let mut rows = vec![vec![(0, 21.0), (1, 21.0), (2, 21.0)]];
        let (mut lo, mut up) = (vec![2.0], vec![f64::INFINITY]);
        let n = tighten_coefficients(&mut rows, &mut lo, &mut up, &[0.0; 3], &[1.0; 3], &[true; 3]);
        assert_eq!(n, 3);
        assert_eq!(rows[0], vec![(0, 2.0), (1, 2.0), (2, 2.0)]);
        assert_eq!(lo[0], 2.0);
        // 最大公約数で割って y0 + y1 + y2 >= 1
        assert_eq!(normalize_integer_rows(&mut rows, &mut lo, &mut up, &[true; 3]), 1);
        assert_eq!(rows[0], vec![(0, 1.0), (1, 1.0), (2, 1.0)]);
        assert_eq!(lo[0], 1.0);
        // 3 x0 + 6 x1 <= 7 -> x0 + 2 x1 <= 2
        let mut r2 = vec![vec![(0, 3.0), (1, 6.0)]];
        let (mut l2, mut u2) = (vec![f64::NEG_INFINITY], vec![7.0]);
        assert_eq!(normalize_integer_rows(&mut r2, &mut l2, &mut u2, &[true; 2]), 1);
        assert_eq!(r2[0], vec![(0, 1.0), (1, 2.0)]);
        assert_eq!(u2[0], 2.0);
    }

    /// 乱数の片側の行 (幅 1 の整数列と有界な連続列 1 本) で、係数の強化の前後で整数列の値の組ごとの
    /// 実行可能性 (連続列を最も有利な値に置いたとき) が変わらないことを総当たりで確かめる。
    #[test]
    fn tighten_coefficients_keeps_integer_feasible_set() {
        let mut seed = 99u64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut total = 0;
        for _ in 0..3000 {
            let nb = 1 + (rnd() * 5.0) as usize;
            let n = nb + 1;
            let mut lo = vec![0.0; n];
            let mut up = vec![1.0; n];
            let mut is_int = vec![true; n];
            for j in 0..nb {
                lo[j] = (rnd() * 3.0).floor() - 1.0;
                up[j] = lo[j] + 1.0;
            }
            is_int[nb] = false;
            lo[nb] = -2.0 * rnd();
            up[nb] = lo[nb] + 3.0 * rnd();
            let row: Vec<(usize, f64)> = (0..n).map(|j| (j, ((rnd() - 0.4) * 20.0).round())).filter(|&(_, a)| a != 0.0).collect();
            if row.is_empty() {
                continue;
            }
            let ge = rnd() < 0.5;
            let rhs = ((rnd() - 0.5) * 30.0).round();
            let (rl, ru) = if ge { (rhs, f64::INFINITY) } else { (f64::NEG_INFINITY, rhs) };
            let mut rows = vec![row.clone()];
            let (mut rlo, mut rup) = (vec![rl], vec![ru]);
            total += tighten_coefficients(&mut rows, &mut rlo, &mut rup, &lo, &up, &is_int);
            let feasible = |r: &[(usize, f64)], l: f64, u: f64, y: &[f64]| -> bool {
                // 連続列は [lo, up] の好きな値を取れる: 活動量の取りうる範囲が [l, u] と交わるか
                let mut a0 = 0.0;
                let mut ac = 0.0;
                for &(j, a) in r {
                    if j < nb {
                        a0 += a * y[j];
                    } else {
                        ac = a;
                    }
                }
                let (c1, c2) = (a0 + ac * lo[nb], a0 + ac * up[nb]);
                let (mn, mx) = (c1.min(c2), c1.max(c2));
                mx >= l - 1e-9 && mn <= u + 1e-9
            };
            for mask in 0..(1u32 << nb) {
                let y: Vec<f64> = (0..nb).map(|j| lo[j] + ((mask >> j) & 1) as f64).collect();
                assert_eq!(feasible(&row, rl, ru, &y), feasible(&rows[0], rlo[0], rup[0], &y), "row {row:?} [{rl}, {ru}] -> {:?} [{}, {}], y {y:?}", rows[0], rlo[0], rup[0]);
            }
        }
        assert!(total > 100, "only {total} coefficients tightened");
    }

    #[test]
    fn merge_parallel_rows_makes_ranged_row() {
        let mut rows = vec![vec![(0, 1.0), (1, 2.0)], vec![(0, -2.0), (1, -4.0)], vec![(0, 1.0), (1, 1.0)]];
        let mut lo = vec![f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        let mut up = vec![5.0, -2.0, 3.0];
        // x0 + 2 x1 <= 5 と -2 x0 - 4 x1 <= -2 (x0 + 2 x1 >= 1)
        let k = merge_parallel_rows(&mut rows, &mut lo, &mut up);
        assert_eq!(k, 1);
        assert_eq!(rows.len(), 2);
        let i = rows.iter().position(|r| r[1].1 == 2.0).unwrap();
        assert!((lo[i] - 1.0).abs() < 1e-12 && (up[i] - 5.0).abs() < 1e-12);
    }

    /// 整数列の整数でない境界は問題を作るときに丸められること (伝播の走査省略やカットは整数の境界を前提にする。
    /// 丸めないと不正なカットが出て最適解を切っていた)。
    #[test]
    fn integer_bounds_are_rounded() {
        let p = MipProblem::from_rows(vec![-0.5, 0.0], vec![1.9665, 2.5], vec![1.0, 1.0], 0.0, 1.0, vec![true, false], vec![vec![(0, 1.0), (1, 1.0)]], vec![f64::NEG_INFINITY], vec![3.0]);
        assert_eq!((p.col_lo[0], p.col_up[0]), (0.0, 1.0));
        assert_eq!((p.col_lo[1], p.col_up[1]), (0.0, 2.5));
    }

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
        let Some(Presolved::Reduced { prob, postsolve, scaling }) = presolve_mip(&p, false) else { panic!("expected a reduced problem") };
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
