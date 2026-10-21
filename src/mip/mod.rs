//! 整数変数を含むモデルのための分枝限定法 (計画は `docs/branch_and_cut_plan.md`)。
//!
//! - [`lp`]・[`lp_api`]: LP の基底・結果の型と、分枝限定法が LP に求める操作 (実装は `simplex::mip_lp::TwoStageLp`)。
//! - [`problem`]: 最小化形の問題データ。
//! - [`domain`]: 変数の定義域、変更の記録と巻き戻し、制約伝播。
//! - [`queue`]: 未処理ノードの待ち行列。
//! - [`pseudocost`]: pseudocost と分枝スコア。
//! - [`solver`]: 探索の本体 (reliability 分岐、plunge、暫定解による枝刈り)。

/// 分枝切除法の LP とやり取りする型 (基底・結果・打ち切り条件)。
pub(crate) mod lp;
pub(crate) mod lp_api;
pub(crate) mod problem;
pub(crate) mod domain;
pub(crate) mod conflict_pool;
pub(crate) mod queue;
pub(crate) mod pseudocost;
pub(crate) mod solver;
pub(crate) mod fj;
mod heuristics;
pub(crate) mod heur_scip;
pub(crate) mod heur_ipm;
pub(crate) mod symmetry;
pub(crate) mod cuts;
pub(crate) mod separation;
pub(crate) mod zerohalf;
pub(crate) mod clique;

use crate::solver::solve_lp;
use crate::types::{ConstraintRow, LpOptions, MipSettings, MipSummary, Objective, RootSolver, SolveResult, Status, VarType, VariableData};
use problem::MipProblem;
use solver::{MipParams, MipStatus};

/// 時間で決める上限 (根のカットのループ・探り) に使う時計 [秒]: このスレッドが使った CPU 時間 (Linux)。ほかの
/// プロセスに CPU を取られている間は進まないので、マシンの負荷で打ち切りの位置が変わりにくい (nw04: 負荷で根のカットの
/// 1 ラウンドが 1.9 -> 4.0 秒になり、時間の上限で 2 ラウンドで止まって根で閉じず 9 -> 17 秒)。Linux 以外と
/// `ENOMOTO_MIP_WALL_CAPS` では経過時間。差だけを使う。
pub(crate) fn cap_clock() -> f64 {
    #[cfg(target_os = "linux")]
    {
        if env_str!("ENOMOTO_MIP_WALL_CAPS").is_none() {
            #[repr(C)]
            struct Timespec {
                tv_sec: i64,
                tv_nsec: i64,
            }
            extern "C" {
                fn clock_gettime(clk: i32, tp: *mut Timespec) -> i32;
            }
            const CLOCK_THREAD_CPUTIME_ID: i32 = 3;
            let mut ts = Timespec { tv_sec: 0, tv_nsec: 0 };
            // SAFETY: `ts` は有効な書き込み先で、clock_gettime はそこに書くだけ
            if unsafe { clock_gettime(CLOCK_THREAD_CPUTIME_ID, &mut ts) } == 0 {
                return ts.tv_sec as f64 + ts.tv_nsec as f64 * 1e-9;
            }
        }
    }
    static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    EPOCH.get_or_init(std::time::Instant::now).elapsed().as_secs_f64()
}

/// 求解の経過の記録 (ベンチマークの指標用: 最初の暫定解の時刻と目的値、根の下界)。最小化形・定数項込みの値で、
/// サブ MIP の中のものは記録しない。プロセスで 1 つなので、同じプロセスで同時に複数の求解をすると混ざる (診断用)。
pub(crate) struct RunStats {
    pub(crate) t0: Option<std::time::Instant>,
    /// 最初の暫定解 (求解の開始からの秒, 目的値)。
    pub(crate) first_sol: Option<(f64, f64)>,
    /// 根の切除平面ループを終えた時点の LP の目的値 (再スタート後の根があれば最後のもの)。
    pub(crate) root_bound: Option<f64>,
}

pub(crate) static RUN_STATS: std::sync::Mutex<RunStats> = std::sync::Mutex::new(RunStats { t0: None, first_sol: None, root_bound: None });

/// 暫定解の記録 ([`RUN_STATS`])。最初のものだけを残す。
pub(crate) fn record_incumbent(z: f64) {
    if let Ok(mut s) = RUN_STATS.lock() {
        if s.first_sol.is_none() {
            let t = s.t0.map_or(0.0, |t0| t0.elapsed().as_secs_f64());
            s.first_sol = Some((t, z));
        }
    }
}

/// 根の下界の記録 ([`RUN_STATS`])。
pub(crate) fn record_root_bound(z: f64) {
    if let Ok(mut s) = RUN_STATS.lock() {
        s.root_bound = Some(z);
    }
}

/// 木の探索に使えるスレッドの数の上限: このプロセスが使える CPU の数 (Linux では CPU affinity と cgroup の割り当てを
/// 反映する `std::thread::available_parallelism`)。環境変数 `ENOMOTO_MIP_MAX_THREADS` でさらに小さくできる。
pub fn max_threads() -> usize {
    // 診断用: `ENOMOTO_MIP_DIAG_HW_THREADS` で使える CPU の数を上書きする (CPU の少ない計算機で多スレッドの振る舞いを再現する)
    let hw = env_str!("ENOMOTO_MIP_DIAG_HW_THREADS")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1));
    let cap = env_str!("ENOMOTO_MIP_MAX_THREADS").and_then(|v| v.parse::<usize>().ok()).filter(|&v| v >= 1).unwrap_or(usize::MAX);
    hw.min(cap).max(1)
}

/// スレッドの数を指定しないときの既定値: このプロセスが使える CPU ([`max_threads`]) の数 + 1 の半分 (HiGHS の
/// `threads = 0` と同じ。論理コアが 4 なら 2、8 なら 4)。
pub fn default_threads() -> usize {
    max_threads().div_ceil(2).max(1)
}

/// (混合) 整数最適化問題を解く。整数変数が 1 つもなければ `solve_lp` を 1 回呼ぶだけ。
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
        // 制限時間: 期限を過ぎると各解法は結論なしで戻る ([`crate::cancel::with_deadline`])。そのときは `TimeLimit`
        let deadline = settings.time_limit.filter(|t| t.is_finite()).map(|t| std::time::Instant::now() + std::time::Duration::from_secs_f64(t.max(0.0)));
        let mut r = crate::cancel::with_deadline(deadline, || solve_lp(variables, objective, constraints, root_solver, opts));
        if r.status == Status::NotSolved && deadline.is_some_and(|d| std::time::Instant::now() >= d) {
            r.status = Status::TimeLimit;
        }
        return r;
    }
    let p = MipProblem::from_model(variables, objective, constraints);
    if let Ok(mut st) = RUN_STATS.lock() {
        *st = RunStats { t0: Some(std::time::Instant::now()), first_sol: None, root_bound: None };
    }
    // 既定値 < 環境変数 < 引数の順に優先する。
    let env_f64 = |v: Option<&str>| v.and_then(|s| s.parse::<f64>().ok());
    let d = MipParams::default();
    let params = MipParams {
        verbose: env_str!("ENOMOTO_MIP_LOG").is_some(),
        rel_gap: settings.rel_gap.or(env_f64(env_str!("ENOMOTO_MIP_REL_GAP"))).unwrap_or(d.rel_gap),
        time_limit: settings.time_limit.or(env_f64(env_str!("ENOMOTO_MIP_TIME_LIMIT"))).unwrap_or(d.time_limit),
        node_limit: settings.node_limit.or(env_f64(env_str!("ENOMOTO_MIP_NODE_LIMIT")).map(|v| v as u64)).unwrap_or(d.node_limit),
        // 未指定なら `opts.threads` (Python の `Model.solve` は未指定のとき [`default_threads`] を渡す)
        threads: settings.threads.or(env_f64(env_str!("ENOMOTO_MIP_TREE_THREADS")).map(|v| v as usize)).unwrap_or(opts.threads.max(d.threads)).clamp(1, max_threads()),
        distinguish: opts.distinguish_infeasible_unbounded,
        // 診断用 (`ENOMOTO_MIP_CUTOFF`): 打ち切り値を元の目的の向きで与える (この値以上の下界のノードを捨てる)。
        // 最適値が分かっている問題で、暫定解を見つける運の良し悪しと証明の時間を切り分けるために使う
        // (pk1 の揺れの調査。最適値 11 の整数目的なら 10.5 を与えると、暫定解 11 を持って始めたのと同じ木になる)
        cutoff: env_f64(env_str!("ENOMOTO_MIP_CUTOFF")).map_or(d.cutoff, |v| p.sense_sign * v),
        ..d
    };
    // 基底のない LP の求解 (根・再スタート後の根) を `threads` 本で同時実行にする (木の探索の従のスレッドを始めたら、
    // またヒューリスティクスのサブ MIP の中では 1)
    // 診断用 (`ENOMOTO_MIP_START_FILE`): 最初の暫定解を与える (元の問題の列の順に 1 行 1 値)。再スタートで暫定解を
    // 引き継ぐ経路 (`RESTART_SOL`) に置くので、前処理後の問題で実行可能なときだけ使われる。主側 (解を見つける) と
    // 双対側 (証明) の切り分けに使う (mzzv42z・neos-960392 の調査)。
    if let Some(f) = env_str!("ENOMOTO_MIP_START_FILE") {
        match std::fs::read_to_string(f) {
            Ok(s) => {
                let x: Vec<f64> = s.lines().filter_map(|l| l.trim().parse().ok()).collect();
                let ok = x.len() == p.n;
                eprintln!(
                    "MIP_START_FILE: {} values (n = {}), objective {} feasible {}",
                    x.len(),
                    p.n,
                    if ok { p.sense_sign * p.objective(&x) } else { f64::NAN },
                    ok && p.is_feasible(&x, 1e-6)
                );
                if ok {
                    RESTART_SOL.with(|s| *s.borrow_mut() = Some(x));
                }
            }
            Err(e) => eprintln!("MIP_START_FILE: cannot read {f}: {e}"),
        }
    }
    let prev_race = crate::simplex::mip_lp::set_cold_race_threads(params.threads);
    // 根のサブ MIP・Feasibility Jump を別スレッドで回すかなどの判定 ([`solver::mip_threads`]) も、`ENOMOTO_MIP_THREADS` が
    // なければこのスレッド数にする
    let prev_mt = solver::MIP_THREADS_DEFAULT.swap(params.threads, std::sync::atomic::Ordering::Relaxed);
    let r = if env_str!("ENOMOTO_MIP_PORTFOLIO").is_some() { solve_portfolio(&p, params) } else { solve_problem(&p, params, env_str!("ENOMOTO_MIP_NO_PRESOLVE").is_none()) };
    crate::simplex::mip_lp::set_cold_race_threads(prev_race);
    solver::MIP_THREADS_DEFAULT.store(prev_mt, std::sync::atomic::Ordering::Relaxed);
    if env_str!("ENOMOTO_MIP_XPROF").is_some() {
        crate::simplex::slope_intercept_dual::xprof("tail");
        for (l, ns) in crate::simplex::slope_intercept_dual::xprof_take() {
            eprintln!("XPROF {l:10} {:10.1} ms", ns as f64 / 1e6);
        }
    }
    let s = p.sense_sign;
    let mut r = r;
    // 返す解を研磨し、目的値を誤差補償つきの和で計算し直す ([`polish_solution`])
    if r.status != MipStatus::Unbounded {
        if let Some(x) = r.x.as_mut() {
            let z = polish_solution(variables, objective, constraints, &p, x);
            if r.status == MipStatus::Optimal {
                r.best_bound = z;
            } else {
                r.best_bound = r.best_bound.min(z);
            }
            r.objective = Some(z);
        }
    }
    // 非有界なら目的値・解は返さない (見つけた実行可能解は最適ではない)
    let unbounded = r.status == MipStatus::Unbounded;
    let objective_value = if unbounded { None } else { r.objective.map(|z| s * z) };
    let gap = match r.objective {
        Some(z) if r.best_bound.is_finite() => ((z - r.best_bound).max(0.0)) / z.abs().max(1.0),
        Some(_) if r.status == MipStatus::Optimal => 0.0,
        _ => f64::INFINITY,
    };
    let (first_sol, root_bound) = RUN_STATS.lock().map(|st| (st.first_sol, st.root_bound)).unwrap_or((None, None));
    let summary = MipSummary {
        best_bound: s * r.best_bound,
        gap,
        nodes: r.nodes,
        lp_iterations: r.lp_iterations,
        root_bound: root_bound.map_or(f64::NAN, |z| s * z),
        first_solution_time: first_sol.map_or(f64::NAN, |(t, _)| t),
        first_solution_objective: first_sol.map_or(f64::NAN, |(_, z)| s * z),
    };
    let status = match r.status {
        MipStatus::Optimal => Status::Optimal,
        MipStatus::Infeasible => Status::Infeasible,
        MipStatus::Unbounded => Status::Unbounded,
        MipStatus::InfeasibleOrUnbounded => Status::InfeasibleOrUnbounded,
        MipStatus::TimeLimit => Status::TimeLimit,
        MipStatus::NodeLimit => Status::NodeLimit,
        MipStatus::NotSolved => Status::NotSolved,
    };
    SolveResult { status, objective: objective_value, x: if unbounded { None } else { r.x }, node_limit_hit: r.status == MipStatus::NodeLimit, mip: Some(summary) }
}

/// 誤差補償つき (Neumaier) の和。
fn exact_sum(it: impl Iterator<Item = f64>) -> f64 {
    let (mut sum, mut comp) = (0.0f64, 0.0f64);
    for v in it {
        let t = sum + v;
        comp += if sum.abs() >= v.abs() { (sum - t) + v } else { (v - t) + sum };
        sum = t;
    }
    sum + comp
}

/// 元の問題での点 `x` の境界・行の違反の最大値 (行の活動量は誤差補償つきの和)。
fn max_violation(p: &MipProblem, x: &[f64]) -> f64 {
    let mut v = 0.0f64;
    for j in 0..p.n {
        v = v.max(p.col_lo[j] - x[j]).max(x[j] - p.col_up[j]);
    }
    for (i, row) in p.rows.iter().enumerate() {
        let a = exact_sum(row.iter().map(|&(j, c)| c * x[j]));
        v = v.max(p.row_lo[i] - a).max(a - p.row_up[i]);
    }
    v
}

/// 最小化形の目的値 (定数項込み) を誤差補償つきの和で計算する。
fn exact_objective(p: &MipProblem, x: &[f64]) -> f64 {
    exact_sum(std::iter::once(p.offset).chain((0..p.n).map(|j| p.cost[j] * x[j])))
}

/// 返す解の研磨: 整数変数をちょうどの整数に丸め、連続変数があれば整数変数をその値に固定した元の問題の LP を
/// 解き直して連続変数の値を取り替える。分枝限定法の中の LP は前処理・スケーリング・境界の摂動・許容誤差の
/// 範囲の解を返すので、そのままでは目的値に 1e-12〜1e-9 程度の相対誤差が残る (khb05250, binkar10_1)。
/// 解き直した解は、元の問題の違反 (境界・行) が増えず目的値が悪くならないとき、または違反が真に減って目的値の悪化が
/// 相対 1e-6 以下のときに採る。最小化形の目的値を返す
/// (誤差補償つきの和)。`ENOMOTO_MIP_NO_POLISH` なら丸めと目的値の計算し直しだけ。
fn polish_solution(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow], p: &MipProblem, x: &mut Vec<f64>) -> f64 {
    for j in 0..p.n {
        if p.is_int[j] {
            x[j] = x[j].round();
        }
    }
    let z = exact_objective(p, x);
    if p.is_int.iter().all(|&b| b) || env_str!("ENOMOTO_MIP_NO_POLISH").is_some() {
        return z;
    }
    let fixed: Vec<VariableData> = variables
        .iter()
        .enumerate()
        .map(|(j, v)| if p.is_int[j] { VariableData { vtype: VarType::Continuous, lb: x[j], ub: x[j] } } else { v.clone() })
        .collect();
    let r = solve_lp(&fixed, objective, constraints, RootSolver::Simplex, LpOptions::default());
    if let (Status::Optimal, Some(mut xn)) = (r.status, r.x) {
        for j in 0..p.n {
            if p.is_int[j] {
                xn[j] = x[j];
            }
        }
        let zn = exact_objective(p, &xn);
        let (vo, vn) = (max_violation(p, x), max_violation(p, &xn));
        // 違反が増えず目的値も (丸め誤差を除いて) 悪くならなければ採る。違反が真に減るなら、目的値が少し悪くなっても採る:
        // 許容誤差の範囲の違反で目的値が実際の最適値より良く見えていた分 (双対値が大きい行では 1e-9 の違反が 1e-8 程度の
        // 相対差になる。binkar10_1) を正すため
        // 違反の比較には丸め誤差の分 (1e-9) の余裕を持たせる: 元の解の違反がちょうど 0 だと、解き直した解の 1e-15 程度の
        // 丸め誤差だけで捨てていた (bppc4-08 で目的値 59.994 -> 57 の研磨を捨てていた)
        let scale = z.abs().max(1.0);
        if (vn <= vo.max(1e-9) && zn <= z + 1e-9 * scale) || (vn < vo && zn <= z + 1e-6 * scale) {
            *x = xn;
            return zn;
        }
    }
    z
}

/// ポートフォリオ並列 (`ENOMOTO_MIP_PORTFOLIO`): 設定の違う 2 つの求解を別スレッドで同時に走らせる
/// (0: 既定、1: 以前の形 = DSE 重みを引き継がず最適面の頂点も使わない。得意な問題が分かれる)。
/// 見つけた解の目的値を共有して互いの打ち切りに使い、片方が探索を終えたら (最適・実行不能) もう片方を止める。
/// 片方が「共有の最良値より良い解はない」と示して終われば、もう片方の最良の解が最適。
fn solve_portfolio(p: &MipProblem, params: MipParams) -> solver::MipResult {
    use std::sync::atomic::Ordering;
    let shared = std::sync::Arc::new(solver::Portfolio::new());
    let use_presolve = env_str!("ENOMOTO_MIP_NO_PRESOLVE").is_none();
    // 探索を終えたスレッドが示した「これより良い解はない」値 (そのスレッドの打ち切りに使った値)
    let proven: std::sync::Mutex<Option<f64>> = std::sync::Mutex::new(None);
    // `ENOMOTO_MIP_PORTFOLIO_BISECT`: 3 本目のスレッドで目的値の二分探索を回す ([`bisection_worker`])
    // `ENOMOTO_MIP_BISECT_FRACS`: 二分探索のスレッドごとの最初の目標値の位置 (下界と最良値の間の割合、コンマ区切り。
    // 既定 "0.5,0.25" の 2 本。木の探索のスレッド (割合 1 にあたる) と合わせて 1, 1/2, 1/4 の 3 本で始める)。
    // 示した下界と見つけた解はスレッドの間で共有し、区間が変わったら二分探索のスレッドは新しい区間の同じ割合で解き直す
    let bisect = env_str!("ENOMOTO_MIP_PORTFOLIO_BISECT").is_some();
    let fracs: Vec<f64> = if bisect {
        env_str!("ENOMOTO_MIP_BISECT_FRACS").map(|s| s.split(',').filter_map(|v| v.trim().parse::<f64>().ok()).filter(|v| *v > 0.0 && *v < 1.0).collect()).unwrap_or_else(|| vec![0.5, 0.25])
    } else {
        Vec::new()
    };
    // `ENOMOTO_MIP_PORTFOLIO_TREES`: 木の探索のスレッドの数 (1 なら既定の設定だけ、2 なら以前の形も。
    // 既定は二分探索のスレッドがあれば 1、なければ 2)
    let ntrees = env_str!("ENOMOTO_MIP_PORTFOLIO_TREES").and_then(|v| v.parse::<u8>().ok()).unwrap_or(if bisect { 1 } else { 2 }).clamp(1, 2);
    let bis_result: std::sync::Mutex<Vec<(Option<solver::MipResult>, f64)>> = std::sync::Mutex::new(Vec::new());
    let results: Vec<solver::MipResult> = std::thread::scope(|sc| {
        for &f0 in &fracs {
            let shared = shared.clone();
            let bis_result = &bis_result;
            sc.spawn(move || {
                let r = bisection_worker(p, params, use_presolve, shared, f0);
                bis_result.lock().unwrap().push(r);
            });
        }
        let hs: Vec<_> = (0..ntrees)
            .map(|prof| {
                let shared = shared.clone();
                let proven = &proven;
                let params = MipParams { verbose: params.verbose && prof == 0, ..params };
                sc.spawn(move || {
                    crate::simplex::slope_intercept_dual::set_mip_profile(prof);
                    solver::set_thread_portfolio(Some(shared.clone()));
                    let r = solve_problem(p, params, use_presolve);
                    solver::set_thread_portfolio(None);
                    let stopped = shared.stop.load(Ordering::Relaxed);
                    if !stopped && matches!(r.status, MipStatus::Optimal | MipStatus::Infeasible) {
                        // このスレッドは探索を終えた: 自分の解か共有の最良値より良い解はない
                        let own = r.objective.unwrap_or(f64::INFINITY);
                        *proven.lock().unwrap() = Some(own.min(shared.best()));
                        shared.stop.store(true, Ordering::Relaxed);
                    }
                    r
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut proven = *proven.lock().unwrap();
    // 問題全体の下界は、木の探索のスレッドの下界と、二分探索のスレッドが示した厳密な下界からだけ作る
    // (二分探索の求解が返す下界は打ち切り値で絞った問題のもので、全体の下界ではない)
    let tree_bound = results.iter().map(|r| r.best_bound).fold(f64::NEG_INFINITY, f64::max).max(shared.proven());
    let mut results = results;
    let mut plb = f64::NEG_INFINITY;
    for (r, v) in bis_result.into_inner().unwrap() {
        if let Some(r) = r {
            results.push(r);
        }
        plb = plb.max(v);
    }
    plb = plb.max(shared.proven());
    {
        // 二分探索のスレッドが示した「plb 未満の解はない」: 最良の解がそれに届いていれば最適
        let bz = results.iter().filter_map(|r| r.objective).fold(f64::INFINITY, f64::min);
        if bz.is_finite() && plb >= bz - (params.rel_gap * bz.abs().max(1.0)).max(params.abs_gap) {
            proven = Some(proven.map_or(plb, |v: f64| v.max(plb)));
        }
    }
    // 最良の解
    let mut best: Option<&solver::MipResult> = None;
    for r in &results {
        if let Some(z) = r.objective {
            if best.is_none_or(|b| z < b.objective.unwrap()) {
                best = Some(r);
            }
        }
    }
    let nodes = results.iter().map(|r| r.nodes).sum();
    let lp_iterations = results.iter().map(|r| r.lp_iterations).sum();
    let mut bound = tree_bound;
    match (best, proven) {
        (Some(b), Some(_)) => {
            // 探索を終えたスレッドは、共有の最良値 (= この解以下) より良い解がないことを示した
            let z = b.objective.unwrap();
            solver::MipResult { status: MipStatus::Optimal, x: b.x.clone(), objective: Some(z), best_bound: z, nodes, lp_iterations }
        }
        (None, Some(_)) => {
            // どちらも解がなく、片方が探索を終えた
            let r = results.iter().find(|r| matches!(r.status, MipStatus::Infeasible | MipStatus::Optimal)).unwrap();
            solver::MipResult { status: r.status, x: None, objective: None, best_bound: r.best_bound, nodes, lp_iterations }
        }
        (Some(b), None) => {
            let z = b.objective.unwrap();
            bound = bound.min(z);
            let status = results.iter().map(|r| r.status).find(|s| *s != MipStatus::Optimal).unwrap_or(b.status);
            solver::MipResult { status, x: b.x.clone(), objective: Some(z), best_bound: bound, nodes, lp_iterations }
        }
        (None, None) => {
            let r = &results[0];
            solver::MipResult { status: r.status, x: None, objective: None, best_bound: bound, nodes, lp_iterations }
        }
    }
}

/// ポートフォリオ並列の目的値の二分探索のスレッド: 共有の最良値 `ub` と木の探索のスレッドの下界 `lb` の間の目標値
/// `t = lb + f (ub - lb)` を打ち切り値にして問題全体を (時間を区切って) 解く。
/// - 解が見つかれば共有の最良値が良くなる (求解の中で知らせる。f は 0.5 に戻す)
/// - 探索を終えて解がなければ「t (と共有の最良値の小さい方) 未満の解はない」と示せたので、下界をそこへ上げる
///   (f は 0.5 に戻す)。最適で終われば、その値未満の解はない (見つけた解が最適)
/// - 時間で終わったら f を暫定解の側へ寄せる
/// 下界が最良値に届いたら全スレッドを止める。戻り値は (見つけた最良の解, 示した下界)。
fn bisection_worker(p: &MipProblem, params: MipParams, use_presolve: bool, shared: std::sync::Arc<solver::Portfolio>, frac0: f64) -> (Option<solver::MipResult>, f64) {
    use std::sync::atomic::Ordering;
    // `ENOMOTO_MIP_BISECT_NO_HEUR`: ヒューリスティクスを走らせず、下界の証明だけに集中する
    // `ENOMOTO_MIP_BISECT_ROOT_HEUR`: 根のヒューリスティクスだけ走らせる (見つけた解は共有の最良値になる)
    crate::simplex::slope_intercept_dual::set_mip_profile(if env_str!("ENOMOTO_MIP_BISECT_NO_HEUR").is_some() {
        2
    } else if env_str!("ENOMOTO_MIP_BISECT_ROOT_HEUR").is_some() {
        3
    } else {
        0
    });
    solver::set_thread_portfolio(Some(shared.clone()));
    let t0 = std::time::Instant::now();
    let tl = params.time_limit;
    let mut frac = frac0;
    let mut proven = f64::NEG_INFINITY;
    let mut best: Option<solver::MipResult> = None;
    let verbose = params.verbose && env_str!("ENOMOTO_MIP_BISECT_VERBOSE").is_some();
    // `ENOMOTO_MIP_BISECT_SNAPSHOT`: 2 回目からの求解は、木の探索のスレッドの探索木の写し ([`solver::TreeSnapshot`]) の
    // 未処理のノードから始める (根の LP・カット・ヒューリスティクスを解き直さない)。写しのノードの部分木を探索し終えるたびに、
    // 木の探索のスレッドはそのノードの子孫の下界を上げる
    let use_snap = env_str!("ENOMOTO_MIP_BISECT_SNAPSHOT").is_some();
    let mut solves = 0u64;
    let mut last_serial = 0u64;
    loop {
        let el = t0.elapsed().as_secs_f64();
        if shared.stop.load(Ordering::Relaxed) || el >= tl - 0.5 {
            break;
        }
        let ub = shared.best();
        proven = proven.max(shared.proven());
        let lb = shared.lb().max(proven);
        if !ub.is_finite() || !lb.is_finite() {
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        }
        let tol = (params.rel_gap * ub.abs().max(1.0)).max(params.abs_gap);
        if lb >= ub - tol {
            // 共有の下界は木の探索のスレッドの待ち行列からの目安で、厳密ではない (処理中のノードを含まない)。
            // 止める合図には使わず、自分で示した下界 (`proven`) だけで判断する
            if proven >= ub - tol {
                shared.stop.store(true, Ordering::Relaxed);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        }
        let t = lb + frac * (ub - lb);
        // 既定 (区間が変わったら解き直す): 時間は区切らず、最良値か厳密な下界が変わったら (どのスレッドが変えても)
        // 求解を打ち切って、新しい区間で同じ割合の目標値から解き直す。`ENOMOTO_MIP_BISECT_SLICED` なら以前の形
        // (時間で区切り、決着がつかなければ割合を最良値の側へ寄せる)
        let sliced = env_str!("ENOMOTO_MIP_BISECT_SLICED").is_some();
        let slice = if sliced { (tunable!("ENOMOTO_T_MIP_BISECT_SLICE", 0.1, f64) * tl).max(2.0).min(tl - el) } else { tl - el };
        let pr = MipParams { time_limit: slice, cutoff: t, verbose: false, ..params };
        // 木の探索のスレッドが根で作ったカットを、この求解の最初に LP に入れる (成り立つ範囲が目標値を含むときだけ)
        if let Some(c) = shared.cuts().filter(|c| t <= c.1 && env_str!("ENOMOTO_MIP_BISECT_SHARE_CUTS").is_some()) {
            RESTART_CUTS.with(|s| *s.borrow_mut() = Some(c.0.clone()));
        }
        if !sliced {
            // 打ち切るのは、厳密な下界が自分の目標値に届いたとき (自分の区間に解がないとほかのスレッドが示した) だけ。
            // 最良値が下がっても打ち切らずに同じ木を続ける (枝刈りの値は共有の最良値まで下がるので、続けても
            // 「min(t, 最良値) 未満の解はない」の証明として正しい。解き直すと、それまでの木を捨てることになる)。
            // `ENOMOTO_MIP_BISECT_ABORT_ON_UB` なら以前の形 (最良値が区間の 1/2 の点を下回ったら打ち切る)
            let trig = if env_str!("ENOMOTO_MIP_BISECT_ABORT_ON_UB").is_some() { lb + 0.5 * (ub - lb) } else { f64::NEG_INFINITY };
            solver::set_thread_watch(Some((trig.min(t), t)));
        }
        let snap = if use_snap && solves > 0 {
            // 新しい写しを求めて待つ (木の探索のスレッドは待ち行列の頭で作る)。来なければ前の写しを使う (古くても
            // 打ち切り値が成り立つ範囲に入っていれば正しい)
            shared.snap_req.store(true, Ordering::Release);
            let w0 = std::time::Instant::now();
            let wait = tunable!("ENOMOTO_T_MIP_BISECT_SNAP_WAIT", 1.0, f64);
            let mut got = None;
            while w0.elapsed().as_secs_f64() < wait && !shared.stop.load(Ordering::Relaxed) && t0.elapsed().as_secs_f64() < tl - 0.5 {
                if let Some(sn) = shared.snapshot().filter(|sn| sn.serial > last_serial) {
                    got = Some(sn);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            got.or_else(|| shared.snapshot()).filter(|sn| t <= sn.valid_below)
        } else {
            None
        };
        solves += 1;
        let from_snap = snap.is_some();
        let snap_info = snap.as_ref().map(|sn| (sn.serial, sn.nodes.len()));
        let r = match snap {
            Some(sn) => {
                last_serial = last_serial.max(sn.serial);
                // 解は写しの問題の空間なので返さない (木の探索のスレッドが受け取る)
                let mut r = solver::solve_snapshot(sn, pr);
                r.x = None;
                r
            }
            None => solve_problem(p, pr, use_presolve),
        };
        solver::set_thread_watch(None);
        let used_cut = t.min(shared.best());
        if verbose {
            eprintln!("MIP: bisection thread: lb {lb:.10e} ub {ub:.10e} target {t:.10e} -> {:?} {:?} bound {:.10e} nodes {} snapshot {:?} ({:.1}s)", r.status, r.objective, r.best_bound, r.nodes, snap_info, t0.elapsed().as_secs_f64());
        }
        // 写しから解いた結果は解 (x) を持たないので、最良の解の候補にしない
        let r = if from_snap { solver::MipResult { objective: r.objective.filter(|&z| z < used_cut), ..r } } else { r };
        let keep = |best: &mut Option<solver::MipResult>, r: solver::MipResult| {
            if let (false, Some(z)) = (from_snap, r.objective) {
                if best.as_ref().is_none_or(|b| z < b.objective.unwrap()) {
                    *best = Some(r);
                }
            }
        };
        match (r.status, r.objective) {
            (MipStatus::Optimal, Some(z)) if z < used_cut => {
                // 打ち切り値未満で最良の解 = 全体の最適 (打ち切り値以上の解はヒューリスティクスが打ち切り値に関係なく
                // 見つけたもので、探索を終えても「t 未満の解はない」ことしか示さない: 下の腕で扱う)
                proven = proven.max(z);
                shared.publish_proven(proven);
                keep(&mut best, r);
                shared.stop.store(true, Ordering::Relaxed);
                break;
            }
            (MipStatus::Infeasible, _) | (MipStatus::Optimal, _) => {
                keep(&mut best, r);
                proven = proven.max(used_cut);
                shared.publish_proven(proven);
                frac = frac0;
            }
            (_, Some(_)) => {
                keep(&mut best, r);
                frac = frac0;
            }
            _ => {
                if sliced {
                    frac = (0.5 * (1.0 + frac)).min(0.9);
                }
            }
        }
    }
    solver::set_thread_portfolio(None);
    (best, proven)
}

/// `p` を (`use_presolve` なら MIP 前処理をかけてから) 分枝限定法で解き、解を `p` の空間に戻す。
/// 戻した解が `p` で実行可能でなければ (前処理の誤りの安全網)、前処理なしで解き直す。
/// サブ MIP (RENS/RINS) もこれで解く。
thread_local! {
    /// 次の [`solve_problem`] に渡す最初の暫定解 (`p` の空間)。開始時に取り出し、前処理後の空間に移して
    /// [`solver::START_SOL`] に置く (再スタートで暫定解を引き継ぐ)。
    pub(crate) static RESTART_SOL: std::cell::RefCell<Option<Vec<f64>>> = const { std::cell::RefCell::new(None) };
    /// 再スタート前のカット (`係数 x <= 右辺`)。再スタートが [`solve_problem`] に渡す問題の空間で置き、前処理後の空間に
    /// 移して置き直し、求解の開始時にカットプールに入れる (static TLS の領域が足りないので 1 つで済ませる)。
    pub(crate) static RESTART_CUTS: std::cell::RefCell<Option<Vec<(Vec<(usize, f64)>, f64)>>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn solve_problem(p: &MipProblem, params: MipParams, use_presolve: bool) -> solver::MipResult {
    let outer = !params.submip && params.restarts == 0;
    if outer {
        ORBITOPES.with(|t| *t.borrow_mut() = None);
        SYM_ORIG.with(|t| *t.borrow_mut() = None);
        SYM_LEX_GENS.with(|t| *t.borrow_mut() = None);
    }
    let r = solve_problem_inner(p, params, use_presolve);
    if outer {
        ORBITOPES.with(|t| *t.borrow_mut() = None);
        SYM_ORIG.with(|t| *t.borrow_mut() = None);
        SYM_LEX_GENS.with(|t| *t.borrow_mut() = None);
        solver::ROOT_BASIS.with(|t| *t.borrow_mut() = None);
    }
    r
}

fn solve_problem_inner(p: &MipProblem, params: MipParams, use_presolve: bool) -> solver::MipResult {
    let start = RESTART_SOL.with(|s| s.borrow_mut().take());
    let t_pre = std::time::Instant::now();
    // 時間上限は前処理・対称性の検出の時間も含める (分枝限定法の時計は `solver::solve` の開始から測るので、渡す前に引く)
    let remaining = |params: MipParams| MipParams { time_limit: (params.time_limit - t_pre.elapsed().as_secs_f64()).max(0.0), ..params };
    // 前処理の LP による固定 ([`lp_probe`]) は最初の求解だけ (サブ MIP・再スタートの後は使わない)
    let lp_probe_secs = (env_str!("ENOMOTO_MIP_NO_LP_PROBE").is_none() && !params.submip && params.restarts == 0)
        .then(|| if params.time_limit.is_finite() { tunable!("ENOMOTO_T_MIP_LP_PROBE_TIME_FRAC", 0.05, f64) * params.time_limit } else { 10.0 });
    let presolved = if use_presolve { presolve_mip(p, params.verbose, params.submip, lp_probe_secs) } else { None };
    // `ENOMOTO_MIP_SPARSIFY_LATE`: HiGHS と同じく前処理 (プロービング・同値の代入の後) の最後に 1 回だけ疎化し
    // ([`sparsify_rows_highs`]、`=old` なら [`sparsify_rows`])、書き換えたら前処理をもう一度かける (その中では疎化しない)
    let presolved = match presolved {
        Some(Presolved::Reduced { prob, postsolve, scaling }) if env_str!("ENOMOTO_MIP_SPARSIFY_LATE").is_some() && !params.submip && params.restarts == 0 => {
            let MipProblem { col_lo, col_up, cost, offset, sense_sign, is_int, mut rows, mut row_lo, mut row_up, .. } = prob.clone();
            let k = if env_str!("ENOMOTO_MIP_SPARSIFY_LATE").is_some_and(|v| v == "old") {
                sparsify_rows(&mut rows, &mut row_lo, &mut row_up, &is_int, prob.n)
            } else {
                sparsify_rows_highs(&mut rows, &mut row_lo, &mut row_up, &is_int, &col_lo, &col_up, prob.n)
            };
            if params.verbose {
                eprintln!("MIP: presolve: sparsified {k} rows (late)");
            }
            if k == 0 {
                Some(Presolved::Reduced { prob, postsolve, scaling })
            } else {
                let q = MipProblem::from_rows(col_lo, col_up, cost, offset, sense_sign, is_int, rows, row_lo, row_up);
                SPARSIFY_REPRESOLVING.with(|f| f.set(true));
                let r = presolve_mip(&q, params.verbose, params.submip, None);
                SPARSIFY_REPRESOLVING.with(|f| f.set(false));
                match r {
                    Some(Presolved::Infeasible) => Some(Presolved::Infeasible),
                    Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) if sc2.d.iter().chain(&scaling.d).all(|&d| d == 1.0) => {
                        let mut postsolve = postsolve;
                        postsolve.extend(post2);
                        Some(Presolved::Reduced { prob: prob2, postsolve, scaling })
                    }
                    // かけ直しが何も減らさない (または尺度が掛かった): 疎化した問題をそのまま使う
                    _ if scaling.d.iter().all(|&d| d == 1.0) => Some(Presolved::Reduced { prob: q, postsolve, scaling }),
                    _ => Some(Presolved::Reduced { prob, postsolve, scaling }),
                }
            }
        }
        other => other,
    };
    // 利用者向けのログ (サブ MIP の前処理は出さない)
    if !params.submip {
        let free = |q: &MipProblem| (0..q.n).filter(|&j| q.col_lo[j] < q.col_up[j]).count();
        match &presolved {
            Some(Presolved::Reduced { prob, .. }) => {
                user_log!("Presolve: {} rows, {} columns -> {} rows, {} columns ({:.2}s)", p.m, free(p), prob.m, free(prob), t_pre.elapsed().as_secs_f64())
            }
            Some(Presolved::Infeasible) => user_log!("Presolve: the problem is infeasible ({:.2}s)", t_pre.elapsed().as_secs_f64()),
            None => {}
        }
    }
    // 前処理は列の番号を保ち、列の尺度 d を掛ける (x = d x')。前処理後の空間に移して実行可能なら渡す
    // (代入で消えた列などで合わなければ渡さない)
    match (&presolved, start) {
        (Some(Presolved::Reduced { prob, scaling, .. }), Some(x)) if x.len() == prob.n => {
            let xr: Vec<f64> = x.iter().zip(&scaling.d).map(|(v, d)| v / d).collect();
            if prob.is_feasible(&xr, 1e-6) {
                solver::START_SOL.with(|s| *s.borrow_mut() = Some(xr));
            } else if params.verbose || env_str!("ENOMOTO_MIP_START_FILE").is_some() {
                eprintln!("MIP: the start solution is not feasible in the presolved problem; dropped");
            }
        }
        (None, Some(x)) if x.len() == p.n => solver::START_SOL.with(|s| *s.borrow_mut() = Some(x)),
        _ => {}
    }
    // 再スタート前のカット: 前処理は列の番号を保ち x = d x' と尺度を変えるので係数に d を掛ける。前処理で新たに
    // 固定された (消された・代入された) 列を含むカットは捨てる (代入された列の値は前処理後の境界からは分からない)
    let cuts = RESTART_CUTS.with(|s| s.borrow_mut().take());
    match (&presolved, cuts) {
        (Some(Presolved::Reduced { prob, scaling, .. }), Some(cuts)) if prob.n == p.n => {
            let mapped: Vec<(Vec<(usize, f64)>, f64)> = cuts
                .into_iter()
                .filter(|(c, _)| c.iter().all(|&(j, _)| j < p.n && (p.col_lo[j] == p.col_up[j] || prob.col_lo[j] < prob.col_up[j])))
                .map(|(c, r)| (c.into_iter().map(|(j, v)| (j, v * scaling.d[j])).collect(), r))
                .collect();
            RESTART_CUTS.with(|s| *s.borrow_mut() = Some(mapped));
        }
        (None, Some(cuts)) => RESTART_CUTS.with(|s| *s.borrow_mut() = Some(cuts)),
        _ => {}
    }
    match presolved {
        Some(Presolved::Infeasible) => {
            solver::START_SOL.with(|s| *s.borrow_mut() = None);
            RESTART_CUTS.with(|s| *s.borrow_mut() = None);
            solver::MipResult { status: MipStatus::Infeasible, x: None, objective: None, best_bound: f64::INFINITY, nodes: 0, lp_iterations: 0 }
        }
        Some(Presolved::Reduced { mut prob, postsolve, scaling }) => {
            if (env_str!("ENOMOTO_MIP_DUAL_UB").is_some() && !params.submip) || crate::fj_p3_force() {
                let k = dual_bound_tightening(&mut prob);
                if params.verbose {
                    eprintln!("MIP: presolve: tightened {k} continuous column bounds by dual arguments");
                }
            }
            if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED").filter(|_| !params.submip && params.restarts == 0) {
                dump_mps(&prob, &f);
            }
            let prob = if env_str!("ENOMOTO_MIP_NO_CLIQUE_EXTEND").is_none() && (!params.submip || env_str!("ENOMOTO_MIP_CLIQUE_EXTEND_SUBMIP").is_some()) { extend_clique_rows(prob, params.verbose) } else { prob };
            // 診断用: クリーク行の拡張の後の問題 (`ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED2=ファイル`)
            if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED2").filter(|_| !params.submip && params.restarts == 0) {
                dump_mps(&prob, &f);
            }
            let prob = if !params.submip && params.restarts == 0 && env_str!("ENOMOTO_MIP_NO_SYMMETRY").is_none() { add_symmetry_rows(prob, params.verbose) } else { prob };
            if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_SOL").filter(|_| !params.submip) {
                let x0: Vec<f64> = std::fs::read_to_string(f).unwrap().lines().map(|l| l.trim().parse().unwrap()).collect();
                let xd: Vec<f64> = (0..prob.n).map(|j| if prob.col_lo[j] == prob.col_up[j] { prob.col_lo[j] } else { x0[j] }).collect();
                eprintln!("MIP_DEBUG_SOL: reduced objective {} feasible {}", prob.objective(&xd), prob.is_feasible(&xd, 1e-6));
                solver::DEBUG_SOL.with(|d| *d.borrow_mut() = Some(xd));
            }
            let outer_level = !params.submip && params.restarts == 0;
            if outer_level {
                solver::set_thread_scale(Some(scaling.d.clone()));
            }
            let mut r = solver::solve(&prob, remaining(params));
            if outer_level {
                solver::set_thread_scale(None);
            }
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
            if ok { r } else { solver::solve(p, remaining(params)) }
        }
        None => solver::solve(p, remaining(params)),
    }
}

thread_local! {
    /// [`presolve_mip`] の 行をまとめた後のかけ直し (`ENOMOTO_MIP_NO_MERGE_REPRESOLVE` で使わない) の 2 回目の前処理の中か。
    static MERGE_REPRESOLVING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// [`presolve_mip`] の `ENOMOTO_MIP_DOMCOL` のかけ直しの中か。
    static DOMCOL_REPRESOLVING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// [`presolve_mip`] の `ENOMOTO_MIP_SPARSIFY` のかけ直しの中か。
    static SPARSIFY_REPRESOLVING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// [`probe`] が見つけた同値な 0-1 列の組 (j, k, 否定か) (`ENOMOTO_MIP_NO_PROBE_EQUIV` で使わない)。
    static PROBE_EQUIV: std::cell::RefCell<Vec<(usize, usize, bool)>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 同値な列による前処理のかけ直しの深さ。
    static PROBE_EQUIV_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

thread_local! {
    /// 今の求解 (再スタートを含む) で使う完全オービトープ ([`add_symmetry_rows`] が置き、外側の求解の終わりに消す)。
    pub(crate) static ORBITOPES: std::cell::RefCell<Option<std::rc::Rc<Vec<symmetry::Orbitope>>>> = const { std::cell::RefCell::new(None) };
    /// 対称性の行・固定を加える前の問題 (同じく外側の求解の終わりに消す)。
    pub(crate) static SYM_ORIG: std::cell::RefCell<Option<std::rc::Rc<MipProblem>>> = const { std::cell::RefCell::new(None) };
    /// lex-leader の行を作った生成元 (`ENOMOTO_MIP_SYM_LEX_CANON` でヒューリスティクスの解を写すのに使う。同じく消す)。
    pub(crate) static SYM_LEX_GENS: std::cell::RefCell<Option<std::rc::Rc<Vec<Vec<usize>>>>> = const { std::cell::RefCell::new(None) };
}

/// 対称性を探し、対称性を崩す不等式 `x_i - x_k >= 0` ([`symmetry::lex_leader_pairs`]) を行として加えた問題を返す。
fn add_symmetry_rows(prob: MipProblem, verbose: bool) -> MipProblem {
    let t0 = std::time::Instant::now();
    // 対称性の行・固定を加える前の問題 (ヒューリスティクスはこちらで解を探し、並べ替えてから使う)
    SYM_ORIG.with(|t| *t.borrow_mut() = Some(std::rc::Rc::new(prob.clone())));
    let time_cap = tunable!("ENOMOTO_T_MIP_SYM_TIME", 0.3, f64);
    let Some(sym) = symmetry::detect(&prob, time_cap, tunable!("ENOMOTO_T_MIP_SYM_TRIES", 2000usize, usize)) else {
        if verbose {
            eprintln!("MIP: symmetry: none ({:.2}s)", t0.elapsed().as_secs_f64());
        }
        return prob;
    };
    // 完全オービトープは探索中の固定 (orbitopal fixing) で扱い、残りの生成元だけ lex-leader の行にする
    let (orbs, used) = if env_str!("ENOMOTO_MIP_NO_ORBITOPE").is_none() { symmetry::find_orbitopes(&prob, &sym) } else { (Vec::new(), vec![false; sym.generators.len()]) };
    let mut in_orb = vec![false; prob.n];
    for o in &orbs {
        for line in &o.vars {
            for &j in line {
                in_orb[j] = true;
            }
        }
    }
    let pairs = symmetry::lex_leader_pairs_except(&sym, &used, &in_orb);
    SYM_LEX_GENS.with(|t| *t.borrow_mut() = Some(std::rc::Rc::new(symmetry::lex_leader_generators_except(&sym, &used, &in_orb))));
    // パッキング・分割の行 (係数が全て 1、右辺 <= 1) に含まれるオービトープの行だけで作ったオービトープに置き換える
    // (列の入れ替えは全体の対称性なので、その部分行列の列を辞書式に並べた解が必ずある)。パッキングの行が半分以上の
    // ときだけ。すると列 c の最初の 1 は行 c 以降なので、x_{i,c} = 0 (c > i) を根で固定できる
    let mut orbs = orbs;
    let mut prob = prob;
    let mut static_fix = 0usize;
    // 既定は動的な orbitopal fixing (分枝した行をその順に使う、HiGHS と同じ): 行ごとのパッキングの印だけ付ける。
    // 根での固定 (x_{i,c} = 0, c > i) は行の固定の順に依るので、動的な固定とは混ぜられない
    let static_orbitope = env_str!("ENOMOTO_MIP_ORBITOPE_STATIC").is_some();
    for o in orbs.iter_mut() {
        let flags: Vec<bool> = o
            .vars
            .iter()
            .map(|line| {
                let set: std::collections::HashSet<usize> = line.iter().copied().collect();
                prob.cols[line[0]].iter().any(|&(i, _)| {
                    let r = &prob.rows[i];
                    prob.row_up[i] <= 1.0 + 1e-9 && r.iter().all(|&(k, a)| a == 1.0 && prob.col_lo[k] >= 0.0) && r.iter().filter(|&&(k, _)| set.contains(&k)).count() == line.len()
                })
            })
            .collect();
        o.row_packing = flags;
        o.packing = o.row_packing.iter().all(|&b| b);
    }
    if static_orbitope && env_str!("ENOMOTO_MIP_NO_PACKING_ORBITOPE").is_none() {
        for o in orbs.iter_mut() {
            let packing_rows: Vec<Vec<usize>> = o
                .vars
                .iter()
                .filter(|line| {
                    let set: std::collections::HashSet<usize> = line.iter().copied().collect();
                    let j0 = line[0];
                    prob.cols[j0].iter().any(|&(i, _)| {
                        let r = &prob.rows[i];
                        prob.row_up[i] <= 1.0 + 1e-9 && r.iter().all(|&(k, a)| a == 1.0 && prob.col_lo[k] >= 0.0) && r.iter().filter(|&&(k, _)| set.contains(&k)).count() == line.len()
                    })
                })
                .cloned()
                .collect();
            if 2 * packing_rows.len() >= o.vars.len() && !packing_rows.is_empty() {
                for (i, line) in packing_rows.iter().enumerate() {
                    for (c, &j) in line.iter().enumerate() {
                        if c > i && prob.col_up[j] > 0.0 {
                            prob.col_up[j] = 0.0;
                            static_fix += 1;
                        }
                    }
                }
                let k = packing_rows.len();
                o.vars = packing_rows;
                o.packing = true;
                o.row_packing = vec![true; k];
            }
        }
    }
    if verbose && static_fix > 0 {
        eprintln!("MIP: symmetry: packing orbitopes fixed {static_fix} columns to 0");
    }
    if env_str!("ENOMOTO_MIP_SYM_DEBUG").is_some() {
        // オービトープの各行に、その行の列だけからなる (他の列も含んでよい) 集合分割・パッキング行があるか
        for o in &orbs {
            let mut packing = 0;
            for line in &o.vars {
                let set: std::collections::HashSet<usize> = line.iter().copied().collect();
                let has = prob.rows.iter().enumerate().any(|(i, r)| prob.row_up[i] == 1.0 && r.iter().all(|&(_, a)| a == 1.0) && line.iter().all(|j| r.iter().any(|&(k, _)| k == *j)) && r.iter().filter(|&&(k, _)| set.contains(&k)).count() == line.len());
                packing += has as usize;
            }
            eprintln!("SYM orbitope {}x{}: {} of {} rows lie in a packing/partitioning row", o.vars.len(), o.vars[0].len(), packing, o.vars.len());
        }
    }
    if verbose && !orbs.is_empty() {
        let dims: Vec<String> = orbs.iter().map(|o| format!("{}x{}{}", o.vars.len(), o.vars[0].len(), if o.packing { " packing" } else { "" })).collect();
        eprintln!("MIP: symmetry: {} full orbitopes ({})", orbs.len(), dims.join(", "));
    }
    ORBITOPES.with(|t| *t.borrow_mut() = Some(std::rc::Rc::new(orbs)));
    if env_str!("ENOMOTO_MIP_SYM_DEBUG").is_some() {
        for (k, g) in sym.generators.iter().enumerate() {
            // 巡回の長さの分布
            let mut seen = vec![false; g.len()];
            let mut lens: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
            for j in 0..g.len() {
                if seen[j] || g[j] == j {
                    continue;
                }
                let mut l = 0;
                let mut x = j;
                while !seen[x] {
                    seen[x] = true;
                    x = g[x];
                    l += 1;
                }
                *lens.entry(l).or_insert(0) += 1;
            }
            let moved: Vec<usize> = (0..g.len()).filter(|&j| g[j] != j).take(12).collect();
            eprintln!("SYM gen {k}: cycles {lens:?} first moved {moved:?} -> {:?}", moved.iter().map(|&j| g[j]).collect::<Vec<_>>());
        }
    }
    if verbose {
        let moved = sym.orbit.iter().filter(|&&o| o != usize::MAX).count();
        eprintln!("MIP: symmetry: {} generators, {} orbits on {moved} columns, {} lex-leader rows ({:.2}s)", sym.generators.len(), sym.num_orbits, pairs.len(), t0.elapsed().as_secs_f64());
    }
    if pairs.is_empty() || env_str!("ENOMOTO_MIP_SYM_DETECT_ONLY").is_some() {
        return prob;
    }
    let MipProblem { col_lo, col_up, cost, offset, sense_sign, is_int, mut rows, mut row_lo, mut row_up, conflict_edges, implied_bounds, .. } = prob;
    for (i, k) in pairs {
        rows.push(vec![(i.min(k), if i < k { 1.0 } else { -1.0 }), (i.max(k), if i < k { -1.0 } else { 1.0 })]);
        row_lo.push(0.0);
        row_up.push(f64::INFINITY);
    }
    // プロービングの含意は行を足しても成り立つので引き継ぐ (以前は落としていた)
    let mut q = MipProblem::from_rows(col_lo, col_up, cost, offset, sense_sign, is_int, rows, row_lo, row_up);
    q.conflict_edges = conflict_edges;
    q.implied_bounds = implied_bounds;
    q
}

/// 診断用 (`ENOMOTO_MIP_DEBUG_DUMP_PRESOLVED=ファイル`): 前処理後の問題 (最小化形、offset は目的の定数列 `OBJCONST` に入れない) を
/// 固定形式でない MPS に書き出す。列は `C<番号>`、行は `R<番号>`。
fn dump_mps(p: &MipProblem, f: &str) {
    use std::fmt::Write as _;
    let mut s = String::new();
    let _ = writeln!(s, "NAME presolved\nROWS\n N OBJ");
    for i in 0..p.m {
        let t = if p.row_lo[i] == p.row_up[i] { "E" } else if p.row_up[i].is_finite() { "L" } else { "G" };
        let _ = writeln!(s, " {t} R{i}");
    }
    let _ = writeln!(s, "COLUMNS");
    let mut in_int = false;
    for j in 0..p.n {
        if p.is_int[j] != in_int {
            let _ = writeln!(s, "    M{j} 'MARKER' {}", if p.is_int[j] { "'INTORG'" } else { "'INTEND'" });
            in_int = p.is_int[j];
        }
        if p.cost[j] != 0.0 {
            let _ = writeln!(s, "    C{j} OBJ {:e}", p.cost[j]);
        }
        for &(i, a) in &p.cols[j] {
            let _ = writeln!(s, "    C{j} R{i} {a:e}");
        }
        if p.cost[j] == 0.0 && p.cols[j].is_empty() {
            let _ = writeln!(s, "    C{j} OBJ 0");
        }
    }
    if in_int {
        let _ = writeln!(s, "    MEND 'MARKER' 'INTEND'");
    }
    let _ = writeln!(s, "RHS");
    for i in 0..p.m {
        let r = if p.row_up[i].is_finite() { p.row_up[i] } else { p.row_lo[i] };
        if r != 0.0 {
            let _ = writeln!(s, "    RHS R{i} {r:e}");
        }
    }
    let _ = writeln!(s, "RANGES");
    for i in 0..p.m {
        if p.row_lo[i] != p.row_up[i] && p.row_lo[i].is_finite() && p.row_up[i].is_finite() {
            let _ = writeln!(s, "    RNG R{i} {:e}", p.row_up[i] - p.row_lo[i]);
        }
    }
    let _ = writeln!(s, "BOUNDS");
    for j in 0..p.n {
        let (l, u) = (p.col_lo[j], p.col_up[j]);
        if l == u {
            let _ = writeln!(s, " FX BND C{j} {l:e}");
            continue;
        }
        if l == f64::NEG_INFINITY {
            let _ = writeln!(s, " MI BND C{j}");
        } else if l != 0.0 {
            let _ = writeln!(s, " LO BND C{j} {l:e}");
        }
        if u.is_finite() {
            let _ = writeln!(s, " UP BND C{j} {u:e}");
        } else if p.is_int[j] {
            let _ = writeln!(s, " PL BND C{j}");
        }
    }
    let _ = writeln!(s, "ENDATA");
    let _ = std::fs::write(f, s);
    eprintln!("MIP: presolved problem written to {f} (objective offset {:e})", p.offset);
}

/// クリーク行の拡張 (既定で使う。`ENOMOTO_MIP_NO_CLIQUE_EXTEND` で使わない。HiGHS の前処理の probing の "lifted nonzeros" に当たる):
/// 0-1 列だけで「文字の和 <= 1」の形の片側の行 (集合詰め込み行、`1 - x` の文字を含む) について、行のすべての文字と衝突する
/// (衝突グラフ = 行から作る組 + プロービングの含意 [`MipProblem::conflict_edges`]) 文字を行に加える。加えた文字どうしも
/// 衝突するものだけ足すので、行は大きなクリークのままで整数解は変わらず、LP の緩和だけが締まる。
/// 等式の行 (文字の和 = 1) では、すべての文字と衝突する文字は偽に固定できる (`ENOMOTO_MIP_CLIQUE_EXTEND_FIX`)。
/// 1 行に加える文字は `ENOMOTO_T_MIP_CLIQUE_EXTEND_MAX` 個まで (既定 1000)。拡張でまったく同じになった行は 1 本にする。
fn extend_clique_rows(prob: MipProblem, verbose: bool) -> MipProblem {
    let t0 = std::time::Instant::now();
    let n = prob.n;
    let binary: Vec<bool> = (0..n).map(|j| prob.is_int[j] && prob.col_lo[j] == 0.0 && prob.col_up[j] == 1.0).collect();
    let g = clique::CliqueGraph::build_with(n, &prob.rows, &prob.row_lo, &prob.row_up, &binary, &prob.col_lo, &prob.col_up, &prob.conflict_edges);
    let max_add = tunable!("ENOMOTO_T_MIP_CLIQUE_EXTEND_MAX", 1000usize, usize);
    let want_fix = env_str!("ENOMOTO_MIP_CLIQUE_EXTEND_FIX").is_some();
    let MipProblem { mut col_lo, mut col_up, cost, offset, sense_sign, is_int, mut rows, mut row_lo, mut row_up, conflict_edges, implied_bounds, .. } = prob;
    let (mut ext_rows, mut added, mut fixed) = (0usize, 0usize, 0usize);
    let mut fix_lits: Vec<u32> = Vec::new();
    // 片側の集合詰め込み行の文字の集合 (拡張の後)。ほかの片側の集合詰め込み行に含まれる行は冗長なので消す
    // (`ENOMOTO_MIP_CLIQUE_EXTEND_KEEP_DOMINATED` で消さない)。HiGHS のクリークの統合も含まれるクリークを消す
    let mut clique_rows: Vec<(usize, Vec<u32>)> = Vec::new();
    for i in 0..rows.len() {
        let (l, u) = (row_lo[i], row_up[i]);
        let eq = l == u;
        // sg * row <= b の向き (等式は <= の向きで見る)
        let sg = if u.is_finite() && (eq || !l.is_finite()) {
            1.0
        } else if l.is_finite() && !u.is_finite() {
            -1.0
        } else {
            continue;
        };
        let b = if sg > 0.0 { u } else { -l };
        let r = &rows[i];
        if r.len() < 2 || !r.iter().all(|&(j, a)| binary[j] && ((sg * a) - 1.0).abs() <= 1e-12 || binary[j] && ((sg * a) + 1.0).abs() <= 1e-12) {
            continue;
        }
        // 文字の和 <= b + (負の係数の数)
        let nneg = r.iter().filter(|&&(_, a)| sg * a < 0.0).count() as f64;
        if ((b + nneg) - 1.0).abs() > 1e-9 {
            continue;
        }
        let lits: Vec<u32> = r.iter().map(|&(j, a)| if sg * a > 0.0 { 2 * j as u32 } else { 2 * j as u32 + 1 }).collect();
        // 候補: 隣接の最も少ない文字の隣接から
        let &l0 = lits.iter().min_by_key(|&&x| g.neighbors(x).len()).unwrap();
        let in_row: std::collections::HashSet<usize> = r.iter().map(|&(j, _)| j).collect();
        let mut add: Vec<u32> = Vec::new();
        for &c in g.neighbors(l0) {
            if add.len() >= max_add {
                break;
            }
            let jc = (c / 2) as usize;
            if in_row.contains(&jc) || col_lo[jc] == col_up[jc] {
                continue;
            }
            if lits.iter().all(|&x| x == l0 || g.adjacent(x, c)) && add.iter().all(|&y| (y / 2) as usize != jc && g.adjacent(y, c)) {
                add.push(c);
            }
        }
        if !eq {
            let mut all: Vec<u32> = lits.iter().chain(&add).copied().collect();
            all.sort_unstable();
            clique_rows.push((i, all));
        }
        if add.is_empty() {
            continue;
        }
        if eq {
            // 等式: 行のどれかが真なので、全員と衝突する文字は偽
            if want_fix {
                fix_lits.extend(add.iter().copied());
            }
            continue;
        }
        let mut b2 = b;
        for &c in &add {
            let jc = (c / 2) as usize;
            if c % 2 == 0 {
                rows[i].push((jc, sg));
            } else {
                rows[i].push((jc, -sg));
                b2 -= 1.0;
            }
        }
        rows[i].sort_unstable_by_key(|e| e.0);
        if sg > 0.0 {
            row_up[i] = b2;
        } else {
            row_lo[i] = -b2;
        }
        ext_rows += 1;
        added += add.len();
    }
    for c in fix_lits {
        let j = (c / 2) as usize;
        if col_lo[j] < col_up[j] {
            // 文字 x_j (c 偶数) が偽なら x_j = 0、文字 1 - x_j が偽なら x_j = 1
            if c % 2 == 0 {
                col_up[j] = 0.0;
            } else {
                col_lo[j] = 1.0;
            }
            fixed += 1;
        }
    }
    // 拡張でまったく同じになった行を 1 本にする
    let mut seen: std::collections::HashMap<(Vec<(usize, u64)>, u64, u64), ()> = std::collections::HashMap::new();
    let mut keep = vec![true; rows.len()];
    let mut dup = 0usize;
    let mut dominated = 0usize;
    if env_str!("ENOMOTO_MIP_CLIQUE_EXTEND_KEEP_DOMINATED").is_none() {
        // 文字 -> それを含むクリーク行 (clique_rows の添字)
        let mut by_lit: std::collections::HashMap<u32, Vec<usize>> = std::collections::HashMap::new();
        for (c, (_, ls)) in clique_rows.iter().enumerate() {
            for &l in ls {
                by_lit.entry(l).or_default().push(c);
            }
        }
        let mut order: Vec<usize> = (0..clique_rows.len()).collect();
        order.sort_by_key(|&c| clique_rows[c].1.len());
        let mut gone = vec![false; clique_rows.len()];
        for &c in &order {
            let (i, ls) = &clique_rows[c];
            // 含む行の候補: 文字のうち含む行の最も少ないものから
            let Some(l0) = ls.iter().min_by_key(|l| by_lit.get(l).map_or(0, |v| v.len())) else { continue };
            let found = by_lit[l0].iter().any(|&d| {
                d != c && !gone[d] && {
                    let ld = &clique_rows[d].1;
                    (ld.len() > ls.len() || (ld.len() == ls.len() && d < c)) && ls.iter().all(|x| ld.binary_search(x).is_ok())
                }
            });
            if found {
                gone[c] = true;
                keep[*i] = false;
                dominated += 1;
            }
        }
    }
    for i in 0..rows.len() {
        if !keep[i] {
            continue;
        }
        let key = (rows[i].iter().map(|&(j, a)| (j, a.to_bits())).collect::<Vec<_>>(), row_lo[i].to_bits(), row_up[i].to_bits());
        if seen.insert(key, ()).is_some() {
            keep[i] = false;
            dup += 1;
        }
    }
    if dup + dominated > 0 {
        let mut k = 0;
        rows.retain(|_| {
            k += 1;
            keep[k - 1]
        });
        let mut k = 0;
        row_lo.retain(|_| {
            k += 1;
            keep[k - 1]
        });
        let mut k = 0;
        row_up.retain(|_| {
            k += 1;
            keep[k - 1]
        });
    }
    if verbose {
        eprintln!("MIP: clique extension: {ext_rows} rows extended, {added} nonzeros added, {fixed} columns fixed, {dup} duplicate and {dominated} dominated rows removed ({} edges, {:.2}s)", g.num_edges(), t0.elapsed().as_secs_f64());
    }
    let mut q = MipProblem::from_rows(col_lo, col_up, cost, offset, sense_sign, is_int, rows, row_lo, row_up);
    q.conflict_edges = conflict_edges;
    q.implied_bounds = implied_bounds;
    q
}

/// 前処理の結果。
enum Presolved {
    Infeasible,
    Reduced { prob: MipProblem, postsolve: Vec<crate::presolve::PostsolveStep>, scaling: crate::presolve::scaling::Scaling },
}

/// 整数を考慮した前処理をかけた問題を作る。前処理が何も減らさなければ `None` (元の問題で解く)。
fn presolve_mip(p: &MipProblem, verbose: bool, submip: bool, lp_probe_secs: Option<f64>) -> Option<Presolved> {
    presolve_mip_depth(p, verbose, submip, lp_probe_secs, 0)
}

fn presolve_mip_depth(p: &MipProblem, verbose: bool, submip: bool, lp_probe_secs: Option<f64>, depth: u32) -> Option<Presolved> {
    use crate::params::simplex::{PRESOLVE_ROUNDS, PROPAGATION_PASSES, ROWSINGLETON_COLSINGLETON_INNER_ROUNDS};
    // 前の呼び出しのプロービングが残した同値な列の組は使わない (列の番号が違いうる)
    PROBE_EQUIV.with(|e| e.borrow_mut().clear());
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
        let tightened = tighten_coefficients(&mut rows, &mut row_lo, &mut row_up, &pre.lb, &pre.ub, &is_int_now, env_str!("ENOMOTO_MIP_COEF_TIGHTEN_GENERAL").is_some());
        let normalized = normalize_integer_rows(&mut rows, &mut row_lo, &mut row_up, &is_int_now);
        if verbose && (tightened > 0 || normalized > 0) {
            eprintln!("MIP: presolve: tightened {tightened} coefficients, normalized {normalized} integer rows");
        }
    }
    // 係数が定数倍の関係にある行をまとめる (上限側と下限側の組は範囲制約 1 本に、同じ向きはきつい方に)。
    // 前処理は範囲制約を 2 本の不等式で表すので、そのままだと LP の行が倍になる
    let eq0 = (0..rows.len()).filter(|&i| row_lo[i] == row_up[i]).count();
    let merged = if env_str!("ENOMOTO_MIP_NO_ROW_MERGE").is_none() { merge_parallel_rows(&mut rows, &mut row_lo, &mut row_up) } else { 0 };
    // 行をまとめて新しくできた等式 (`x - y <= 0` と `x - y >= 0` から `x = y` など) の数
    let new_eq = (0..rows.len()).filter(|&i| row_lo[i] == row_up[i]).count().saturating_sub(eq0);
    if verbose && merged > 0 {
        eprintln!("MIP: presolve: merged {merged} parallel rows ({new_eq} new equations)");
    }
    // `ENOMOTO_MIP_SPARSIFY`: 等式の倍数を引いて行を疎にする ([`sparsify_rows`])。書き換えたら前処理をかけ直す (下の
    // MERGE_REPRESOLVE と同じ形。疎になった行が冗長・強制になったり、列が支配されたりする)
    let sparsified = if env_str!("ENOMOTO_MIP_SPARSIFY").is_some() && !submip && !SPARSIFY_REPRESOLVING.with(|f| f.get()) {
        let k = if env_str!("ENOMOTO_MIP_SPARSIFY").is_some_and(|v| v == "highs") {
            sparsify_rows_highs(&mut rows, &mut row_lo, &mut row_up, &p.is_int, &pre.lb, &pre.ub, n)
        } else {
            sparsify_rows(&mut rows, &mut row_lo, &mut row_up, &p.is_int, n)
        };
        if verbose {
            eprintln!("MIP: presolve: sparsified {k} rows");
        }
        k
    } else {
        0
    };
    let fixed = (0..n).filter(|&j| pre.lb[j] == pre.ub[j]).count();
    if verbose {
        let nnz0 = p.nnz;
        let nnz1: usize = rows.iter().map(|r| r.len()).sum();
        eprintln!("MIP: presolve: rows {} -> {}, free columns {} -> {}, nonzeros {nnz0} -> {nnz1}, postsolve steps {}", p.m, rows.len(), (0..n).filter(|&j| p.col_lo[j] < p.col_up[j]).count(), n - fixed, pre.postsolve_log.len());
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
    let mut prob = MipProblem::from_rows(pre.lb.clone(), pre.ub.clone(), pre.c.clone(), offset, p.sense_sign, p.is_int.clone(), rows, row_lo, row_up);
    // `ENOMOTO_MIP_KEEP_IMPLICATIONS`: 元の問題が持っていた含意 (サブ MIP なら親のプロービングの結果) を引き継ぐ。前処理は
    // 列の番号を変えないので、両端が前処理後も尺度 1 の 0-1 列のままの組だけ残す (HiGHS もサブ MIP に含意とクリーク表を渡す)
    if env_str!("ENOMOTO_MIP_KEEP_IMPLICATIONS").is_some() && !p.conflict_edges.is_empty() {
        let bin = |j: usize| prob.is_int[j] && prob.col_lo[j] == 0.0 && prob.col_up[j] == 1.0 && pre.scaling.d.get(j).is_none_or(|&d| d == 1.0);
        prob.conflict_edges = p.conflict_edges.iter().copied().filter(|&(a, b)| bin((a / 2) as usize) && bin((b / 2) as usize)).collect();
    }
    // 行をまとめた後のかけ直し (既定で使う。`ENOMOTO_MIP_NO_MERGE_REPRESOLVE` で使わない): 行をまとめて等式ができたら、前処理をもう一度かける (1 回だけ)。前処理の中では向きの逆の
    // 不等式の組は等式として見えないので、`x = y`・`x + y = 1` の代入 (二項等式の消去) がされずに残る。physiciansched6-2 は
    // こうしてできた 0-1 列どうしの `x - y = 0` が 3482 本、`x + y = 1` が 46 本残り (HiGHS の前処理後は等式 283 本)、
    // Feasibility Jump が解を見つけられない。後処理は内側の手順の後に外側の手順を並べる (LP による固定と同じ)
    let merge_rp = new_eq > 0 && env_str!("ENOMOTO_MIP_NO_MERGE_REPRESOLVE").is_none() && !MERGE_REPRESOLVING.with(|f| f.get());
    if merge_rp || sparsified > 0 {
        // 疎化のかけ直しの中では疎化しない (1 回だけ)
        let (was_m, was_s) = (MERGE_REPRESOLVING.with(|f| f.get()), SPARSIFY_REPRESOLVING.with(|f| f.get()));
        MERGE_REPRESOLVING.with(|f| f.set(was_m || merge_rp));
        SPARSIFY_REPRESOLVING.with(|f| f.set(was_s || sparsified > 0));
        let r = presolve_mip(&prob, verbose, submip, lp_probe_secs);
        MERGE_REPRESOLVING.with(|f| f.set(was_m));
        SPARSIFY_REPRESOLVING.with(|f| f.set(was_s));
        match r {
            Some(Presolved::Infeasible) => return Some(Presolved::Infeasible),
            Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) if sc2.d.iter().chain(&pre.scaling.d).all(|&d| d == 1.0) => {
                let mut postsolve = pre.postsolve_log;
                postsolve.extend(post2);
                return Some(Presolved::Reduced { prob: prob2, postsolve, scaling: pre.scaling });
            }
            // 何も減らない (または尺度が掛かった): このまま続ける
            _ => {}
        }
    }
    // LP による固定は含意を集めるプロービングの前に行う (固定をプロービングの伝播に使い、固定した列の含意を作らない)。
    // 固定したら前処理をもう一度かける (固定した列と、それで冗長になった行を消し、縮約を続ける。プロービングもそちらで行う)。
    // 前処理は列の番号を保ち、整数最適化では列の尺度をかけない (d = 1) ので、後処理は内側の手順の後に外側の手順を並べればよい
    if let Some(secs) = lp_probe_secs {
        match lp_probe(&mut prob, verbose, secs) {
            None => return Some(Presolved::Infeasible),
            Some(fixed) if fixed > 0 && env_str!("ENOMOTO_MIP_LP_PROBE_NO_REPRESOLVE").is_none() => {
                let mut postsolve = pre.postsolve_log;
                return match presolve_mip_depth(&prob, verbose, submip, None, depth) {
                    Some(Presolved::Infeasible) => Some(Presolved::Infeasible),
                    Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) => {
                        debug_assert!(sc2.d.iter().chain(&pre.scaling.d).all(|&d| d == 1.0));
                        postsolve.extend(post2);
                        Some(Presolved::Reduced { prob: prob2, postsolve, scaling: pre.scaling })
                    }
                    // 2 回目の前処理が何も減らさない: 1 回目の結果にプロービングだけかける
                    None => {
                        if env_str!("ENOMOTO_MIP_NO_PROBING").is_none() {
                            match probe(&mut prob, verbose, submip) {
                                Some(false) => return Some(Presolved::Infeasible),
                                Some(true) => return represolve_after_probe(prob, postsolve, pre.scaling, verbose, submip, depth),
                                None => {}
                            }
                        }
                        Some(Presolved::Reduced { prob, postsolve, scaling: pre.scaling })
                    }
                };
            }
            Some(_) => {}
        }
    }
    if env_str!("ENOMOTO_MIP_NO_PROBING").is_none() {
        match probe(&mut prob, verbose, submip) {
            Some(false) => return Some(Presolved::Infeasible),
            Some(true) => return represolve_after_probe(prob, pre.postsolve_log, pre.scaling, verbose, submip, depth),
            None => {}
        }
    }
    // `ENOMOTO_MIP_DOMCOL`: 支配される 0-1 列を固定し ([`dominated_binary_fixings`])、前処理をかけ直す (LP による固定と同じ形、1 回だけ)
    if env_str!("ENOMOTO_MIP_DOMCOL").is_some() && !submip && !DOMCOL_REPRESOLVING.with(|f| f.get()) {
        let t0 = std::time::Instant::now();
        let fix = dominated_binary_fixings(&prob);
        if verbose {
            eprintln!("MIP: presolve: {} dominated binaries fixed ({:.2}s)", fix.len(), t0.elapsed().as_secs_f64());
        }
        for &(j, v) in &fix {
            prob.col_lo[j] = v;
            prob.col_up[j] = v;
        }
        // 固定が自由な列の `ENOMOTO_T_MIP_DOMCOL_REPRESOLVE_FRAC` (既定 5%) 未満なら、かけ直さずに境界だけ固定する
        // (nw04 は 87482 列中 372 列の固定に前処理 1 回 (約 1.8 秒) をかけ直していた)
        let nfree = (0..n).filter(|&j| prob.col_lo[j] < prob.col_up[j]).count() + fix.len();
        if !fix.is_empty() && fix.len() as f64 >= tunable!("ENOMOTO_T_MIP_DOMCOL_REPRESOLVE_FRAC", 0.05, f64) * nfree as f64 {
            DOMCOL_REPRESOLVING.with(|f| f.set(true));
            let r = presolve_mip(&prob, verbose, submip, None);
            DOMCOL_REPRESOLVING.with(|f| f.set(false));
            match r {
                Some(Presolved::Infeasible) => return Some(Presolved::Infeasible),
                Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) if sc2.d.iter().chain(&pre.scaling.d).all(|&d| d == 1.0) => {
                    let mut postsolve = pre.postsolve_log;
                    postsolve.extend(post2);
                    return Some(Presolved::Reduced { prob: prob2, postsolve, scaling: pre.scaling });
                }
                _ => {}
            }
        }
    }
    // 同値な 0-1 列の代入 (既定で使う。`ENOMOTO_MIP_NO_PROBE_EQUIV` で使わない): プロービングで見つけた同値な 0-1 列を等式 (`x_k - x_j = 0` / `x_k + x_j = 1`) として足し、
    // 前処理をもう一度かける (二項等式の消去が列を代入で消す)。かけ直しは `ENOMOTO_T_MIP_PROBE_EQUIV_ROUNDS` 回まで (既定 2)
    let equiv = PROBE_EQUIV.with(|e| std::mem::take(&mut *e.borrow_mut()));
    let depth = PROBE_EQUIV_DEPTH.with(|d| d.get());
    if !equiv.is_empty() && depth < tunable!("ENOMOTO_T_MIP_PROBE_EQUIV_ROUNDS", 2usize, usize) {
        let MipProblem { col_lo, col_up, cost, offset, sense_sign, is_int, mut rows, mut row_lo, mut row_up, .. } = prob.clone();
        for &(j, k, neg) in &equiv {
            let (a, b) = (j.min(k), j.max(k));
            rows.push(vec![(a, 1.0), (b, if neg { 1.0 } else { -1.0 })]);
            let r = if neg { 1.0 } else { 0.0 };
            row_lo.push(r);
            row_up.push(r);
        }
        let q = MipProblem::from_rows(col_lo, col_up, cost, offset, sense_sign, is_int, rows, row_lo, row_up);
        PROBE_EQUIV_DEPTH.with(|d| d.set(depth + 1));
        let r = presolve_mip(&q, verbose, submip, None);
        PROBE_EQUIV_DEPTH.with(|d| d.set(depth));
        match r {
            Some(Presolved::Infeasible) => return Some(Presolved::Infeasible),
            Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) if sc2.d.iter().chain(&pre.scaling.d).all(|&d| d == 1.0) => {
                let mut postsolve = pre.postsolve_log;
                postsolve.extend(post2);
                return Some(Presolved::Reduced { prob: prob2, postsolve, scaling: pre.scaling });
            }
            _ => {}
        }
    }
    Some(Presolved::Reduced { prob, postsolve: pre.postsolve_log, scaling: pre.scaling })
}

/// プロービングの後の前処理のかけ直し (試作、`ENOMOTO_MIP_PROBE_REPRESOLVE` で使う。既定では使わない): プロービングが
/// 境界を締めた (固定した) とき、締めた境界で前処理をもう一度かける (`ENOMOTO_T_MIP_PROBE_REPRESOLVE_MAX` 回まで、既定 2)。
/// プロービングは連続列の境界も締める (drayage-25-23 では時刻の列の境界) が、その境界で冗長になった行・強められる係数は
/// 前処理をかけ直さないと残る。drayage-25-23 では 3212 行のまま根を解き、再スタートの前処理で初めて 3873 -> 1150 行になる
/// (HiGHS は前処理の中で 732 行まで減らす)。かけ直した前処理は新しい問題でプロービングをやり直す (含意はそちらで集め直す)。
/// 前処理は列の番号を保ち、整数最適化では列の尺度をかけない (d = 1) ので、後処理は内側の手順の後に外側の手順を並べればよい
fn represolve_after_probe(prob: MipProblem, mut postsolve: Vec<crate::presolve::PostsolveStep>, scaling: crate::presolve::scaling::Scaling, verbose: bool, submip: bool, depth: u32) -> Option<Presolved> {
    let max = tunable!("ENOMOTO_T_MIP_PROBE_REPRESOLVE_MAX", 2u32, u32);
    if env_str!("ENOMOTO_MIP_PROBE_REPRESOLVE").is_none() || depth >= max || (submip && env_str!("ENOMOTO_MIP_PROBE_REPRESOLVE_SUBMIP").is_none()) {
        return Some(Presolved::Reduced { prob, postsolve, scaling });
    }
    if verbose {
        eprintln!("MIP: presolve again after probing (round {})", depth + 1);
    }
    match presolve_mip_depth(&prob, verbose, submip, None, depth + 1) {
        Some(Presolved::Infeasible) => Some(Presolved::Infeasible),
        Some(Presolved::Reduced { prob: prob2, postsolve: post2, scaling: sc2 }) => {
            debug_assert!(sc2.d.iter().chain(&scaling.d).all(|&d| d == 1.0));
            postsolve.extend(post2);
            Some(Presolved::Reduced { prob: prob2, postsolve, scaling })
        }
        // 何も減らない: プロービングの結果 (含意・締めた境界) を持つ元の問題を使う
        None => Some(Presolved::Reduced { prob, postsolve, scaling }),
    }
}

/// LP による固定 (既定で使う。`ENOMOTO_MIP_NO_LP_PROBE` で使わない。前処理の最後、含意を集めるプロービングの前): LP 緩和を解き、LP 解で分数値の整数列
/// x_j = v ごとに、上の子 (x_j >= ceil v) と下の子 (x_j <= floor v) の LP を反復上限つきで解く。片方が実行不能なら逆側に
/// 固定し (根の強分枝で子が実行不能になったときの固定と同じ。どの整数解でも成り立つ)、両方なら問題は実行不能。固定したら
/// LP を解き直し、新しく分数値になった列で繰り返す (`ENOMOTO_T_MIP_LP_PROBE_PASSES` 回まで)。手間の上限は LP の反復の合計
/// (`ENOMOTO_T_MIP_LP_PROBE_ITER_MULT` x 最初の LP の反復 + 5000) と時間 `secs`。最後の LP の基底は根の LP に渡す
/// ([`solver::ROOT_BASIS`])。
///
/// app1-1 では 0-1 列 1030 個のうち 925 個が「1 にすると LP が実行不能」で、LP 解の分数値の 19 個を固定するだけで LP 値が
/// −20.96 → −3.98 (目的関数は整数なので下界 −3 = 最適値) になる。HiGHS は経路集約・tableau のカットで根を −4 にし、こちらは
/// 根のカットのループで −7〜−8.6 にしかならず、木の根の強分枝で初めて −3.99 に届いていた。
/// 固定した列の数を返す。実行不能と分かったら `None`。
fn lp_probe(prob: &mut MipProblem, verbose: bool, secs: f64) -> Option<usize> {
    use lp::{LpStatus, SolveLimits};
    let t0 = std::time::Instant::now();
    let deadline = Some(t0 + std::time::Duration::from_secs_f64(secs.max(0.0)));
    let mut lp = crate::simplex::mip_lp::TwoStageLp::new(&prob.col_lo, &prob.col_up, &prob.cost, &prob.rows, &prob.row_lo, &prob.row_up);
    match lp.solve(&SolveLimits { deadline, ..Default::default() }) {
        LpStatus::Optimal => {}
        LpStatus::Infeasible => return None,
        _ => return Some(0),
    }
    let z0 = lp.objective() + prob.offset;
    let root_secs = t0.elapsed().as_secs_f64();
    let root_iters = lp.total_iterations();
    let iter_budget = (tunable!("ENOMOTO_T_MIP_LP_PROBE_ITER_MULT", 2.0, f64) * root_iters as f64) as u64 + 5000;
    let per_lp = (root_iters / 4).clamp(200, 2000);
    let passes = tunable!("ENOMOTO_T_MIP_LP_PROBE_PASSES", 3usize, usize);
    // 最初の `ENOMOTO_T_MIP_LP_PROBE_GIVEUP` 列で 1 つも固定できなければやめる (固定が出る問題では最初からすぐ出る:
    // app1-1 は最初の 24 列で 21 個。40 問で固定が出るのは 5 問で、出ない問題に 1 問平均 0.5 秒、最大 3 秒かかっていた)
    let giveup = tunable!("ENOMOTO_T_MIP_LP_PROBE_GIVEUP", 20usize, usize);
    // 固定が 1 つもないまま、最初の LP の `ENOMOTO_T_MIP_LP_PROBE_GIVEUP_TIME_MULT` 倍 (+ 0.1 秒) の時間を使ってもやめる
    // (子の LP が重い問題: cvs16r128-89 は 5 列で 3 秒、根の LP は 0.9 秒)
    let giveup_secs = tunable!("ENOMOTO_T_MIP_LP_PROBE_GIVEUP_TIME_MULT", 1.0, f64) * root_secs + 0.1;
    let budget_left = |lp: &crate::simplex::mip_lp::TwoStageLp| lp.total_iterations() - root_iters < iter_budget && t0.elapsed().as_secs_f64() < secs;
    let (mut n_lps, mut n_fixed, mut n_tried) = (0usize, 0usize, 0usize);
    let mut tried: std::collections::HashSet<(usize, i64)> = std::collections::HashSet::new();
    let mut ok = true;
    for pass in 0..passes {
        let x = lp.col_values();
        let mut cands: Vec<(usize, f64)> = (0..prob.n)
            .filter(|&j| prob.is_int[j] && prob.col_lo[j] < prob.col_up[j] && (x[j] - x[j].round()).abs() > domain::FEASTOL && !tried.contains(&(j, x[j].floor() as i64)))
            .map(|j| (j, x[j]))
            .collect();
        // 分数部分が 1/2 に近い順
        cands.sort_by(|a, b| {
            let fa = a.1 - a.1.floor();
            let fb = b.1 - b.1.floor();
            (fb * (1.0 - fb)).total_cmp(&(fa * (1.0 - fa)))
        });
        if cands.is_empty() || !budget_left(&lp) {
            break;
        }
        let saved = lp.save_state();
        // (列, 上限を締めるか, 値)
        let mut fixes: Vec<(usize, bool, f64)> = Vec::new();
        for &(j, v) in &cands {
            if !budget_left(&lp) || (n_fixed + fixes.len() == 0 && (n_tried >= giveup || t0.elapsed().as_secs_f64() > giveup_secs)) {
                break;
            }
            tried.insert((j, v.floor() as i64));
            n_tried += 1;
            let (lo, up) = (prob.col_lo[j], prob.col_up[j]);
            let mut cut = [false; 2];
            for (side, is_up) in [(0usize, true), (1usize, false)] {
                if is_up {
                    lp.set_col_bounds(j, v.ceil(), up);
                } else {
                    lp.set_col_bounds(j, lo, v.floor());
                }
                cut[side] = lp.solve(&SolveLimits { iteration_limit: per_lp, deadline, ..Default::default() }) == LpStatus::Infeasible;
                n_lps += 1;
                lp.restore_state(&saved);
            }
            match (cut[0], cut[1]) {
                (true, true) => {
                    ok = false;
                    break;
                }
                // 上の子が実行不能なら上限を floor に、下の子なら下限を ceil に
                (true, false) => fixes.push((j, true, v.floor())),
                (false, true) => fixes.push((j, false, v.ceil())),
                _ => {}
            }
        }
        if verbose {
            eprintln!(
                "MIP: LP probing pass {}: {} candidates, {} fixings{} ({} LPs, {} iters, {:.2}s)",
                pass + 1,
                cands.len(),
                fixes.len(),
                if ok { "" } else { ", both sides infeasible" },
                n_lps,
                lp.total_iterations() - root_iters,
                t0.elapsed().as_secs_f64()
            );
        }
        if !ok || fixes.is_empty() {
            break;
        }
        for &(j, upper, val) in &fixes {
            if upper {
                prob.col_up[j] = prob.col_up[j].min(val);
            } else {
                prob.col_lo[j] = prob.col_lo[j].max(val);
            }
            lp.set_col_bounds(j, prob.col_lo[j], prob.col_up[j]);
        }
        n_fixed += fixes.len();
        match lp.solve(&SolveLimits { deadline, ..Default::default() }) {
            LpStatus::Optimal => {}
            LpStatus::Infeasible => {
                ok = false;
                break;
            }
            _ => break,
        }
    }
    if verbose && n_tried > 0 {
        eprintln!(
            "MIP: LP probing: tried {} columns, fixed {}, LP {:.10e} -> {:.10e} ({} LPs, {} iters, {:.2}s)",
            n_tried,
            n_fixed,
            z0,
            lp.objective() + prob.offset,
            n_lps,
            lp.total_iterations() - root_iters,
            t0.elapsed().as_secs_f64()
        );
    }
    if !ok {
        return None;
    }
    // 固定したときだけ最後の LP の基底を根の LP に渡す (固定がなければ根の LP は前処理の LP と同じ問題で、基底を渡すと
    // 着く頂点が変わり、カット・探索の経路が変わるだけ: qiu は固定 0 で 40 問の 2 回とも遅くなった)。ただし最初の LP に
    // `ENOMOTO_T_MIP_LP_PROBE_BASIS_SECS` 秒 (既定 1 秒) 以上かかったら、根で解き直す手間を省くために渡す
    if n_fixed > 0 || root_secs >= tunable!("ENOMOTO_T_MIP_LP_PROBE_BASIS_SECS", 1.0, f64) {
        let b = lp.basis().col;
        solver::ROOT_BASIS.with(|s| *s.borrow_mut() = Some(b));
    }
    Some(n_fixed)
}

/// プロービング (SCIP・HiGHS の前処理と同じ): 0-1 列 `x_j` を 0 にして伝播した結果と 1 にして伝播した結果を比べる。
///
/// - 両方矛盾すれば問題は実行不能 (`Some(false)` を返す)。
/// - 片方だけ矛盾すれば、もう片方の値に固定する。
/// - どちらも矛盾しなければ、各列の境界を両方の結果の弱い方 (下限は小さい方、上限は大きい方) まで締める
///   (どちらの値でも成り立つので、大域的に成り立つ)。
///
/// 列の値は変えない (境界を締めるだけ) ので後処理は要らない。手間は伝播で走査した行の長さの合計
/// (非零数の `ENOMOTO_T_MIP_PROBE_WORK` 倍、既定 20 倍) と時間 (`ENOMOTO_T_MIP_PROBE_TIME` 秒、既定 0.5 秒。固定・締め付けが
/// 出ていれば `ENOMOTO_T_MIP_PROBE_TIME_FOUND` 秒、既定 0.5 秒。長い行の多い問題 (nw04・air03) は 1 回が重く何も得られない) で打ち切る。
/// 締めた境界があれば `Some(true)`、何もなければ `None`。
fn probe(prob: &mut MipProblem, verbose: bool, submip: bool) -> Option<bool> {
    use domain::Domain;
    let t0 = std::time::Instant::now();
    let c0 = cap_clock();
    let n = prob.n;
    let nnz = prob.nnz;
    // 旧来の上限 (サブ MIP と `ENOMOTO_MIP_NO_PROBE_ADAPTIVE`): 手間は非零数の 20 倍、時間は 0.5 秒 (固定・締め付けが出ていれば
    // `ENOMOTO_T_MIP_PROBE_TIME_FOUND` 秒)。
    // 大きくすると (2000 倍・1 秒・3 秒) 30n20b8 は 7971 列を全部試して 3241 列を固定し根の LP が 43.3 -> 122.8
    // (HiGHS 123.0)、neos-860300 も 2035 -> 2210 になるが、40 問では 19 問 28.14 -> 18 問 28.08 (neos-860300 が
    // 2 回とも時間切れ、30n20b8 はそれでも解が見つからない) で既定にはしない
    let work_cap = (tunable!("ENOMOTO_T_MIP_PROBE_WORK", 20.0, f64) * nnz as f64) as u64 + 100_000;
    let time_cap = tunable!("ENOMOTO_T_MIP_PROBE_TIME", 0.5, f64);
    let time_cap_found = tunable!("ENOMOTO_T_MIP_PROBE_TIME_FOUND", 0.5, f64);
    let mut dom = Domain::new(prob);
    if !dom.propagate(prob) {
        return Some(false);
    }
    // 行の多い列から
    let mut cands: Vec<usize> = (0..n).filter(|&j| prob.is_int[j] && dom.lo[j] == 0.0 && dom.up[j] == 1.0).collect();
    cands.sort_by_key(|&j| std::cmp::Reverse(prob.cols[j].len()));
    let work0 = dom.debug_work();
    let (mut probed, mut fixed, mut tightened) = (0usize, 0usize, 0usize);
    // 含意 (`ENOMOTO_MIP_NO_PROBE_IMPLICATIONS` で集めない): x_j = v で 0-1 列 x_k が固定されれば、
    // 「x_j = v かつ x_k が固定と逆の値」は成り立たない (衝突の組)。上限は非零数の 10 倍
    let want_impl = env_str!("ENOMOTO_MIP_NO_PROBE_IMPLICATIONS").is_none();
    let edge_cap = 10 * nnz + 10_000;
    let bin0: Vec<bool> = (0..n).map(|k| prob.is_int[k] && prob.col_lo[k] == 0.0 && prob.col_up[k] == 1.0).collect();
    // 引き継いだ含意 ([`presolve_mip`] の `ENOMOTO_MIP_KEEP_IMPLICATIONS`) に足していく
    let mut edges: Vec<(u32, u32)> = std::mem::take(&mut prob.conflict_edges);
    // 含意による境界 (implied bound カット用): x_j = v で x_k の境界が締まるもの。上限は非零数の 10 倍
    let mut implied: Vec<(u32, bool, u32, bool, f64)> = Vec::new();
    let mut add_implied = |j: usize, v: bool, ch: &[(usize, bool, f64)], dom: &Domain, implied: &mut Vec<(u32, bool, u32, bool, f64)>| {
        let mut best: std::collections::HashMap<(usize, bool), f64> = std::collections::HashMap::new();
        for &(k, upper, val) in ch {
            if k == j {
                continue;
            }
            let e = best.entry((k, upper)).or_insert(val);
            *e = if upper { e.min(val) } else { e.max(val) };
        }
        for ((k, upper), val) in best {
            if implied.len() >= edge_cap {
                return;
            }
            let (l, u) = (dom.lo[k], dom.up[k]);
            if !l.is_finite() || !u.is_finite() {
                continue;
            }
            let gain = if upper { u - val } else { val - l };
            if gain > 1e-6 * (1.0 + val.abs()) {
                implied.push((j as u32, v, k as u32, upper, val));
            }
        }
    };
    let mut add_edges = |j: usize, v: u32, ch: &[(usize, bool, f64)], edges: &mut Vec<(u32, u32)>| {
        let lj = 2 * j as u32 + if v == 1 { 0 } else { 1 };
        for &(k, upper, val) in ch {
            if k == j || !bin0[k] || edges.len() >= edge_cap {
                continue;
            }
            // x_k <= 0 なら x_k = 1 (文字 2k) と衝突、x_k >= 1 なら x_k = 0 (文字 2k+1) と衝突
            if upper && val <= 0.5 {
                edges.push((lj, 2 * k as u32));
            } else if !upper && val >= 0.5 {
                edges.push((lj, 2 * k as u32 + 1));
            }
        }
    };
    // 成果連動の予算 (既定。`ENOMOTO_MIP_NO_PROBE_ADAPTIVE` で旧来の上限に戻す。サブ MIP 以外): HiGHS の `runProbing` と同じく、試す回数の予算を成果で伸ばす。
    // 予算は候補の数から始め、1 回の試行で固定・締め付けた数だけ増やす。候補を一巡して
    // 固定が候補の 1% 以上出ていれば、残った候補でもう一巡する (最大 `ENOMOTO_T_MIP_PROBE_PASSES` 巡、既定 10。
    // HiGHS は前処理のループで probing を繰り返す)。手間 (`ENOMOTO_T_MIP_PROBE_ADAPT_WORK` 倍、既定 2000 倍) と時間
    // (`ENOMOTO_T_MIP_PROBE_ADAPT_TIME` 秒、既定 3 秒) は安全網。30n20b8 は既定の上限 (非零数の 20 倍) では 7971 本中 160 本で
    // 止まり 1 本も固定できないが、HiGHS は probing を 7 巡 (約 7 秒) 回して列を 8033 -> 4733 本にする
    // サブ MIP でも使うのは `ENOMOTO_MIP_PROBE_ADAPTIVE_SUBMIP` のとき (時間は `ENOMOTO_T_MIP_PROBE_ADAPT_TIME_SUB` 秒、既定 0.5 秒。
    // HiGHS はサブ MIP の前処理でも probing を回す)
    // 40 問・300 秒・4 回 (旧来 / 成果連動): 平均時間 65.71/66.13/67.21/62.22 と 66.33/66.94/66.80/63.82、解けた数
    // 23/22/22/23 と 23/23/23/23 (pk1 が 4 回とも解ける。旧来は 2 回)、主ギャップの和は 2.89/3.03/3.10/2.83 と
    // 2.39/3.02/2.63/2.76。neos-860300 は 31-50 秒 -> 45-49 秒と遅くなる
    let adaptive = env_str!("ENOMOTO_MIP_NO_PROBE_ADAPTIVE").is_none() && (!submip || env_str!("ENOMOTO_MIP_PROBE_ADAPTIVE_SUBMIP").is_some());
    let (work_cap, time_cap, time_cap_found) = if adaptive {
        let t = if submip { tunable!("ENOMOTO_T_MIP_PROBE_ADAPT_TIME_SUB", 0.5, f64) } else { tunable!("ENOMOTO_T_MIP_PROBE_ADAPT_TIME", 3.0, f64) };
        // 何も固定・締め付けられない間は `ENOMOTO_T_MIP_PROBE_ADAPT_TIME0` 秒 (既定 1 秒) で止める。3 秒のままだと長い行の
        // 多い問題で何も得られずに使い切る (40 問・3 回で air03 0.9 -> 4.0 秒、nw04 9.3 -> 11.9 秒)。30n20b8 は最初の固定まで
        // 約 0.6 秒
        let t0 = tunable!("ENOMOTO_T_MIP_PROBE_ADAPT_TIME0", 1.0, f64).min(t);
        ((tunable!("ENOMOTO_T_MIP_PROBE_ADAPT_WORK", 2000.0, f64) * nnz as f64) as u64 + 100_000, t0, t)
    } else {
        (work_cap, time_cap, time_cap_found)
    };
    // 予算の初期値は候補の数 (全部を一巡できる)。`ENOMOTO_T_MIP_PROBE_BASE` > 0 ならその回数。HiGHS の 1000 回は含意の多い列
    // から試す順番と組み合わせたもので、行の多い列から試すこちらの順番では、30n20b8 の固定できる列 (3 行の短い列、3161 本)
    // が候補の後ろ (4730 番目以降) に並び、1000 回では 1 本も固定できない (全部を試しても 0.96 秒で 3305 本固定)
    let base = tunable!("ENOMOTO_T_MIP_PROBE_BASE", 0.0, f64);
    let mut contingent = if adaptive { if base > 0.0 { base } else { cands.len() as f64 } } else { f64::INFINITY };
    let max_pass = if adaptive { tunable!("ENOMOTO_T_MIP_PROBE_PASSES", 10usize, usize).max(1) } else { 1 };
    let want_equiv = env_str!("ENOMOTO_MIP_NO_PROBE_EQUIV").is_none() && !submip;
    // `ENOMOTO_MIP_PROBE_CLIQUE`: 見つけた含意を後の列のプロービングの伝播に使う ([`domain::Domain::add_implications`])
    let want_pclique = env_str!("ENOMOTO_MIP_PROBE_CLIQUE").is_some() && !submip;
    let mut equiv: Vec<(usize, usize, bool)> = Vec::new();
    let mut passes = 0usize;
    'passes: for _pass in 0..max_pass {
        passes += 1;
        let fixed_pass0 = fixed;
        for (cnt, &j) in cands.iter().enumerate() {
            let cap = if fixed + tightened > 0 { time_cap_found } else { time_cap };
            if cnt % 16 == 0 && (dom.debug_work() - work0 > work_cap || cap_clock() - c0 > cap) {
                break 'passes;
            }
            if dom.lo[j] == dom.up[j] {
                continue;
            }
            if probed as f64 >= contingent {
                break 'passes;
            }
            let found0 = fixed + tightened;
            probed += 1;
            let pos = dom.stack_len();
            // x_j = 0 (変わった境界だけ覚える: 全列の境界を写すと列の多い問題で重い)
            dom.tighten_upper(prob, j, 0.0);
            let ok0 = dom.propagate(prob);
            let ch0: Vec<_> = if ok0 { dom.changes_since(pos).collect() } else { Vec::new() };
            dom.backtrack_to(prob, pos);
            // x_j = 1
            dom.tighten_lower(prob, j, 1.0);
            let ok1 = dom.propagate(prob);
            let ch1: Vec<_> = if ok1 { dom.changes_since(pos).collect() } else { Vec::new() };
            dom.backtrack_to(prob, pos);
            match (ok0, ok1) {
                (false, false) => return Some(false),
                (false, true) | (true, false) => {
                    let v = if ok1 { 1.0 } else { 0.0 };
                    dom.tighten_lower(prob, j, v);
                    dom.tighten_upper(prob, j, v);
                    if !dom.propagate(prob) {
                        return Some(false);
                    }
                    fixed += 1;
                    if env_str!("ENOMOTO_MIP_DEBUG_PROBE").is_some() {
                        eprintln!("MIP:   probing fixed column {j} at candidate {cnt} (pass {passes}, {} rows)", prob.cols[j].len());
                    }
                }
                (true, true) => {
                    if want_impl {
                        let e0 = edges.len();
                        add_edges(j, 0, &ch0, &mut edges);
                        add_edges(j, 1, &ch1, &mut edges);
                        if want_pclique {
                            dom.add_implications(n, &edges[e0..]);
                        }
                        // implied bound カットを使うときだけ集める (既定は相手が連続変数のものだけ使う。`ENOMOTO_MIP_NO_IMPLBD_CONT` で使わない)
                        if env_str!("ENOMOTO_MIP_IMPLBD_CUTS").is_some() || env_str!("ENOMOTO_MIP_NO_IMPLBD_CONT").is_none() {
                            add_implied(j, false, &ch0, &dom, &mut implied);
                            add_implied(j, true, &ch1, &dom, &mut implied);
                        }
                    }
                    // 両方の結果で変わった (列, 側) だけが締まりうる。値は両方の弱い方
                    let mut first: std::collections::HashMap<(usize, bool), f64> = std::collections::HashMap::new();
                    for &(k, upper, v) in &ch0 {
                        first.insert((k, upper), v);
                    }
                    let mut last1: std::collections::HashMap<(usize, bool), f64> = std::collections::HashMap::new();
                    for &(k, upper, v) in &ch1 {
                        last1.insert((k, upper), v);
                    }
                    // 同値な 0-1 列 (既定で集める。`ENOMOTO_MIP_NO_PROBE_EQUIV` で集めない): x_j = 0 で x_k = 0、x_j = 1 で x_k = 1 なら x_k = x_j。
                    // 逆向きなら x_k = 1 - x_j。等式の行として足して前処理をかけ直す ([`presolve_mip`]、HiGHS の probing の代入)
                    if want_equiv {
                        for (&(k, upper), &v0) in &first {
                            if k == j || !bin0[k] || dom.lo[k] == dom.up[k] {
                                continue;
                            }
                            let Some(&v1) = last1.get(&(k, !upper)) else { continue };
                            if upper && v0 <= 0.5 && v1 >= 0.5 {
                                equiv.push((j, k, false));
                            } else if !upper && v0 >= 0.5 && v1 <= 0.5 {
                                equiv.push((j, k, true));
                            }
                        }
                    }
                    let mut any = false;
                    for (&(k, upper), &v0) in &first {
                        if k == j {
                            continue;
                        }
                        let Some(&v1) = last1.get(&(k, upper)) else { continue };
                        if upper {
                            let u = v0.max(v1);
                            if u < dom.up[k] - 1e-9 * (1.0 + u.abs()) {
                                dom.tighten_upper(prob, k, u);
                                any = true;
                                tightened += 1;
                            }
                        } else {
                            let l = v0.min(v1);
                            if l > dom.lo[k] + 1e-9 * (1.0 + l.abs()) {
                                dom.tighten_lower(prob, k, l);
                                any = true;
                                tightened += 1;
                            }
                        }
                    }
                    if any && !dom.propagate(prob) {
                        return Some(false);
                    }
                }
            }
            contingent += (fixed + tightened - found0) as f64;
        }
        // もう一巡するのは、この巡で候補の 1% 以上を固定したときだけ
        if ((fixed - fixed_pass0) as f64) < 0.01 * cands.len() as f64 {
            break;
        }
    }
    // 固定した列を含む組は捨てる (固定後は意味がない)
    edges.retain(|&(a, b)| dom.lo[(a / 2) as usize] < dom.up[(a / 2) as usize] && dom.lo[(b / 2) as usize] < dom.up[(b / 2) as usize]);
    edges.sort_unstable();
    edges.dedup();
    if verbose {
        eprintln!("MIP: probing: {probed} of {} binaries probed ({passes} passes), {fixed} fixed, {tightened} bounds tightened, {} implications ({:.2}s)", cands.len(), edges.len(), t0.elapsed().as_secs_f64());
    }
    prob.conflict_edges = edges;
    if want_equiv {
        let mut seen = std::collections::HashSet::new();
        equiv.retain(|&(j, k, _)| dom.lo[j] < dom.up[j] && dom.lo[k] < dom.up[k] && seen.insert((j.min(k), j.max(k))));
        if verbose {
            eprintln!("MIP: probing: {} equivalent binary pairs", equiv.len());
        }
        PROBE_EQUIV.with(|e| *e.borrow_mut() = std::mem::take(&mut equiv));
    }
    // 固定した列を含む含意は捨てる
    implied.retain(|&(j, _, k, _, _)| dom.lo[j as usize] < dom.up[j as usize] && dom.lo[k as usize] < dom.up[k as usize]);
    prob.implied_bounds = implied;
    if fixed == 0 && tightened == 0 {
        return None;
    }
    // 締めた境界を問題に入れる (連続列の境界は伝播の誤差の分だけ緩めてある)
    for k in 0..n {
        if dom.lo[k] > prob.col_lo[k] {
            prob.col_lo[k] = dom.lo[k];
        }
        if dom.up[k] < prob.col_up[k] {
            prob.col_up[k] = dom.up[k];
        }
    }
    Some(true)
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
fn tighten_coefficients(rows: &mut [Vec<(usize, f64)>], row_lo: &mut [f64], row_up: &mut [f64], lo: &[f64], up: &[f64], is_int: &[bool], general: bool) -> usize {
    // 対象の列: 既定は幅 1 の整数列 (0-1 型)。`ENOMOTO_MIP_COEF_TIGHTEN_GENERAL` なら両側の境界が有限な整数列すべて。
    // x_j ∈ [l, u] (整数) で a > 0 なら、x_j <= u - 1 で行が冗長 (M - a <= b) のとき d = b - (M - a) だけ縮め、
    // 右辺を d u 減らす (x_j = u では元と同じ、x_j <= u - 1 では残りの最大活動量で成り立つ)。a < 0 なら右辺を d l 増やす。
    // 幅 1 (0-1 列) はその特別な場合。一般整数まで広げる (`ENOMOTO_MIP_COEF_TIGHTEN_GENERAL`) と gt2 の前処理後の LP が
    // 13460 -> 20146.8 (HiGHS と同じ) になるが、40 問では mik-250-20-75-4 が 2 回とも遅くなり (31.7 -> 56-60 秒) 既定にはしない
    let bin_only = !general;
    let unit = |j: usize| {
        is_int[j]
            && lo[j].is_finite()
            && up[j].is_finite()
            && up[j] - lo[j] >= 1.0 - 1e-9
            && (lo[j] - lo[j].round()).abs() <= 1e-9
            && (up[j] - up[j].round()).abs() <= 1e-9
            && (!bin_only || (up[j] - lo[j] - 1.0).abs() <= 1e-9)
    };
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
                    // 右辺は d u_j 減る (0-1 型なら u_j = l_j + 1)。最大活動量も x_j の上限での寄与の分 d u_j 減る
                    b -= d * up[j];
                    maxact -= d * up[j];
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

/// 双対の推論による連続列の境界の締め付け (`ENOMOTO_MIP_DUAL_UB`、既定では使わない): 費用が非負 (最小化) の連続列 x_j が、
/// x_j を増やすと楽になる行 (`<=` の向きで係数が負) では、ほかの列が最悪の値 (その行の最大活動) でも行を満たす値
/// `u* = max_i (maxact_i(j を除く) - b_i) / |a_ij|` を超える必要はない。x_j を増やすと苦しくなる向き (等式を含む) には出ない
/// ときだけ使う。u* より大きい解は x_j を u* に下げても行は破れず目的も悪くならないので、上限を `max(lb, u*)` にできる。
/// 費用が非正なら向きを逆にして下限を締める。physiciansched6-2 の罰則の列 (`100 Σx + 600 z - w <= 600` だけに出る費用 1 の w、
/// 416 本) は ENOMOTO では [0, ∞) のまま残り、HiGHS の前処理後は [0, 100〜600] になる。上限がないと Feasibility Jump が
/// 解を見つけられない (同じ問題で上限を入れるだけで 20 回中 0 回 → 19 回)。締めた境界の数を返す (1 回だけ、連鎖しない)。
/// Feasibility Jump だけに渡す問題 (`ENOMOTO_MIP_FJ_P3`、試作、既定では使わない): 本体の前処理後の問題 `pc` に、このスレッドだけ
/// `ENOMOTO_MIP_INEQ_SINGLETON`・`ENOMOTO_MIP_CS_OBJ_PIVOT` を有効にした前処理をもう一度かけ、双対の推論で連続列の境界を締める
/// ([`dual_bound_tightening`])。本体の問題は変えない。前処理は列の番号を保つので、FJ の解は `postsolve` を逆順に当てて尺度を
/// 戻せば `pc` の空間の解になる ([`fj_p3_solution`])。physiciansched6-2 では FJ が解を見つけるのにこの 3 つと FJ の LP 修復の
/// 4 つが全部要る (1 つでも外すと 20 秒で解なし) が、p5s として本体の前処理に入れると beasleyC3・uccase12 で証明が遅れる
pub(super) fn fj_p3_problem(pc: &MipProblem) -> Option<(MipProblem, Vec<crate::presolve::PostsolveStep>, Option<crate::presolve::scaling::Scaling>)> {
    crate::set_fj_p3_force(true);
    let r = presolve_mip(pc, false, false, None);
    crate::set_fj_p3_force(false);
    let (mut prob, post, sc) = match r {
        Some(Presolved::Reduced { prob, postsolve, scaling }) if prob.n == pc.n => (prob, postsolve, Some(scaling)),
        Some(Presolved::Reduced { .. }) | Some(Presolved::Infeasible) => return None,
        None => (pc.clone(), Vec::new(), None),
    };
    let k = dual_bound_tightening(&mut prob);
    if env_str!("ENOMOTO_MIP_LOG").is_some() {
        eprintln!("MIP: FJ problem (FJ_P3): rows {} -> {}, free columns {} -> {}, {k} continuous bounds tightened", pc.m, prob.m, (0..pc.n).filter(|&j| pc.col_lo[j] < pc.col_up[j]).count(), (0..prob.n).filter(|&j| prob.col_lo[j] < prob.col_up[j]).count());
    }
    Some((prob, post, sc))
}

/// [`fj_p3_problem`] の問題での解 `x` を元の問題 `pc` の空間に戻す。
pub(super) fn fj_p3_solution(x: Vec<f64>, post: &[crate::presolve::PostsolveStep], sc: &Option<crate::presolve::scaling::Scaling>) -> Vec<f64> {
    let mut x = x;
    for step in post.iter().rev() {
        step.apply(&mut x);
    }
    match sc {
        Some(sc) => crate::presolve::scaling::unscale_x(sc, &x),
        None => x,
    }
}

fn dual_bound_tightening(prob: &mut MipProblem) -> usize {
    let m = prob.rows.len();
    let (mut mn, mut mx) = (vec![0.0f64; m], vec![0.0f64; m]);
    let (mut mn_inf, mut mx_inf) = (vec![0u32; m], vec![0u32; m]);
    for (i, r) in prob.rows.iter().enumerate() {
        for &(j, a) in r {
            let (l, u) = (prob.col_lo[j], prob.col_up[j]);
            let (lo_c, up_c) = if a > 0.0 { (a * l, a * u) } else { (a * u, a * l) };
            if lo_c.is_finite() { mn[i] += lo_c } else { mn_inf[i] += 1 }
            if up_c.is_finite() { mx[i] += up_c } else { mx_inf[i] += 1 }
        }
    }
    let mut count = 0;
    let cols = prob.cols.clone();
    for j in 0..prob.n {
        if prob.is_int[j] || prob.col_lo[j] == prob.col_up[j] || cols[j].is_empty() {
            continue;
        }
        // d = +1: 上限を締める (費用 >= 0)、d = -1: 下限を締める (費用 <= 0)。費用 0 は両方を試す
        for d in [1.0f64, -1.0] {
            if d * prob.cost[j] < 0.0 {
                continue;
            }
            let cur = if d > 0.0 { prob.col_up[j] } else { -prob.col_lo[j] };
            let other = if d > 0.0 { prob.col_lo[j] } else { -prob.col_up[j] };
            if !other.is_finite() {
                continue;
            }
            // y = d x_j を増やすと楽になる行だけか。楽になる行ごとに必要な y の値の最大
            let mut need = other;
            let mut ok = true;
            for &(i, a) in &cols[j] {
                let ay = d * a; // y の係数
                for (side, b) in [(1.0f64, prob.row_up[i]), (-1.0, -prob.row_lo[i])] {
                    if !b.is_finite() {
                        continue;
                    }
                    // side 向きの行: side * row <= b。y の係数 side * ay
                    let c = side * ay;
                    if c > 0.0 {
                        ok = false; // y を増やすと苦しくなる向きがある
                        break;
                    }
                    // 楽になる向き: c y <= b - (ほかの列の活動)。ほかの列の最悪 = side 向きの最大活動
                    let (act, inf) = if side > 0.0 { (mx[i], mx_inf[i]) } else { (-mn[i], mn_inf[i]) };
                    let own = {
                        let ax = side * a;
                        if ax > 0.0 { ax * prob.col_up[j] } else { ax * prob.col_lo[j] }
                    };
                    let own_inf = !own.is_finite();
                    if inf - own_inf as u32 > 0 {
                        ok = false;
                        break;
                    }
                    let rest = if own_inf { act } else { act - own };
                    // c y <= b - rest かつ c < 0: y >= (rest - b) / |c|
                    need = need.max((rest - b) / (-c));
                }
                if !ok {
                    break;
                }
            }
            if !ok || !need.is_finite() {
                continue;
            }
            // 少し余裕を持たせる (丸め誤差で最適解を落とさない)
            let nb = need + 1e-9 * (1.0 + need.abs());
            if nb < cur - 1e-6 * (1.0 + cur.abs().min(1e9)) {
                if d > 0.0 {
                    prob.col_up[j] = nb;
                } else {
                    prob.col_lo[j] = -nb;
                }
                count += 1;
            }
        }
    }
    count
}

/// 支配される 0-1 列の固定 (`ENOMOTO_MIP_DOMCOL`、既定では使わない。HiGHS の前処理の "dominated columns"、Gamrath ら 2015):
/// 0-1 列の文字 (`y = x` か `y = 1 - x`) で考える。文字 y_j・y_k で、費用が `c(y_j) <= c(y_k)`、すべての行で y_j の係数が y_k の
/// 係数より行を破りにくい (上限のある向きで `a_ij <= a_ik`、下限のある向きで `a_ij >= a_ik`、等式は等しい) とき、y_j は y_k を
/// 支配する: y_k = 1 の解から y_k を 0 に、y_j を 1 増やすと、行はどれも破れず目的も悪くならない。よって
/// - y_j と y_k が衝突する (同時に 1 にならない。衝突グラフの辺) なら y_k = 0 の最適解がある
/// - y_j の上限 1 が行から導ける (ほかの列の境界と 1 本の行で `y_j <= 1` が出る。増やしても境界を越えない) なら y_k = 0
/// のどちらかで y_k を 0 に固定する。候補は、衝突グラフの y_j の隣と、(上限が導けるとき) y_j の最も短い行の列。
/// 固定した列を含む組は使わない (固定は順に、それまでの固定をした問題で正しい。支配と「導ける」は境界を締めても崩れない)。
/// 調べる組は `ENOMOTO_T_MIP_DOMCOL_MAX_PAIRS` まで (既定 100 万)。固定 (列, 値) を返す。
fn dominated_binary_fixings(prob: &MipProblem) -> Vec<(usize, f64)> {
    dominated_binary_fixings_with(prob, env_str!("ENOMOTO_MIP_DOMCOL_PRED").is_some())
}

/// [`dominated_binary_fixings`] の本体。`pred` は `ENOMOTO_MIP_DOMCOL_PRED` (予測の境界の (c) も使う)。
fn dominated_binary_fixings_with(prob: &MipProblem, pred: bool) -> Vec<(usize, f64)> {
    let n = prob.n;
    let binary: Vec<bool> = (0..n).map(|j| prob.is_int[j] && prob.col_lo[j] == 0.0 && prob.col_up[j] == 1.0).collect();
    let g = clique::CliqueGraph::build_with(n, &prob.rows, &prob.row_lo, &prob.row_up, &binary, &prob.col_lo, &prob.col_up, &prob.conflict_edges);
    let max_pairs = tunable!("ENOMOTO_T_MIP_DOMCOL_MAX_PAIRS", 1_000_000usize, usize);
    let mut cols: Vec<Vec<(usize, f64)>> = prob.cols.clone();
    for c in cols.iter_mut() {
        c.sort_unstable_by_key(|e| e.0);
    }
    // 行ごとの最小・最大活動 (有限部分と無限の項の数)
    let m = prob.rows.len();
    let (mut mn, mut mx) = (vec![0.0f64; m], vec![0.0f64; m]);
    let (mut mn_inf, mut mx_inf) = (vec![0u32; m], vec![0u32; m]);
    for (i, r) in prob.rows.iter().enumerate() {
        for &(j, a) in r {
            let (l, u) = (prob.col_lo[j], prob.col_up[j]);
            let (lo_c, up_c) = if a > 0.0 { (a * l, a * u) } else { (a * u, a * l) };
            if lo_c.is_finite() { mn[i] += lo_c } else { mn_inf[i] += 1 }
            if up_c.is_finite() { mx[i] += up_c } else { mx_inf[i] += 1 }
        }
    }
    // 文字 (j, s) の上限 1 が行から導けるか (s = +1: x_j <= 1、s = -1: x_j >= 0)。0-1 列の寄与は有限
    let implied_up = |j: usize, s: f64| -> bool {
        cols[j].iter().any(|&(i, a)| {
            // 向き d (上限側 +1: a x <= up、下限側 -1: -a x <= -lo) ごとに、文字の係数が正になる向きで
            [(1.0, prob.row_up[i]), (-1.0, -prob.row_lo[i])].into_iter().any(|(d, b)| {
                let al = d * a * s; // 文字 y の係数 (x = y か 1 - y)
                if !b.is_finite() || al <= 0.0 {
                    return false;
                }
                // d 向きの最小活動 (j を除く)
                let (act, inf) = if d > 0.0 { (mn[i], mn_inf[i]) } else { (-mx[i], mx_inf[i]) };
                if inf > 0 {
                    return false;
                }
                let aj = d * a;
                let own = if aj > 0.0 { aj * prob.col_lo[j] } else { aj * prob.col_up[j] };
                let rest = act - own;
                // 文字 y の項: s = +1 なら aj x = al y、s = -1 なら aj x = aj - al y (定数 aj は右辺へ)
                let bb = if s > 0.0 { b } else { b - aj };
                (bb - rest) / al <= 1.0 + 1e-9
            })
        })
    };
    // 文字 (j, sj) が (k, sk) を支配するか
    let dominates = |j: usize, sj: f64, k: usize, sk: f64| -> bool {
        if sj * prob.cost[j] > sk * prob.cost[k] + 1e-9 * (1.0 + prob.cost[k].abs()) {
            return false;
        }
        let (cj, ck) = (&cols[j], &cols[k]);
        let (mut a, mut b) = (0, 0);
        while a < cj.len() || b < ck.len() {
            let (i, aj, ak) = if b >= ck.len() || (a < cj.len() && cj[a].0 < ck[b].0) {
                a += 1;
                (cj[a - 1].0, sj * cj[a - 1].1, 0.0)
            } else if a >= cj.len() || ck[b].0 < cj[a].0 {
                b += 1;
                (ck[b - 1].0, 0.0, sk * ck[b - 1].1)
            } else {
                a += 1;
                b += 1;
                (cj[a - 1].0, sj * cj[a - 1].1, sk * ck[b - 1].1)
            };
            let tol = 1e-9 * (1.0 + aj.abs().max(ak.abs()));
            if prob.row_up[i].is_finite() && aj > ak + tol {
                return false;
            }
            if prob.row_lo[i].is_finite() && aj < ak - tol {
                return false;
            }
        }
        true
    };
    // `ENOMOTO_MIP_DOMCOL_PRED`: HiGHS の予測の境界 (Gamrath ら 2015 の定理 3 (vi)) に当たる (c) も使う: y_j が y_k を支配し
    // c(y_k) >= 0 のとき、y_j = 1・y_k = 0 にして、y_k を下げると破れうる行 (y_k の係数が上限側で負・下限側で正) が、ほかの
    // 列の最悪の値でも満たされるなら y_k = 0 にできる (支配の補題で y_j = 1 か y_k = 0 の最適解があり、y_j = 1 の側でも
    // y_k を 0 に下げて行は破れず目的も悪くならない)。physiciansched6-2 には係数も費用も同じ 0-1 列が 3 本ずつの組が 13 あり、
    // HiGHS は各組の 2 本をこれで 0 に固定する (3 本の和が 1 を超えても得をしない)
    // 列 x の値 (文字 (j, s) の値 v に当たる)
    let xval = |s: f64, v: f64| if s > 0.0 { v } else { 1.0 - v };
    let worst_case_ok = |j: usize, sj: f64, k: usize, sk: f64| -> bool {
        if sk * prob.cost[k] < -1e-9 * (1.0 + prob.cost[k].abs()) {
            return false;
        }
        let (xj, xk) = (xval(sj, 1.0), xval(sk, 0.0));
        cols[k].iter().all(|&(i, ak)| {
            // y_k を下げると (x_k の向き -sk) 行の活動は -sk * ak だけ動く
            let dk = -sk * ak;
            let aj = cols[j].binary_search_by_key(&i, |e| e.0).map(|p| cols[j][p].1).unwrap_or(0.0);
            // j・k を除いたほかの列の最大・最小活動
            let contrib = |jj: usize, a: f64| if a > 0.0 { (a * prob.col_lo[jj], a * prob.col_up[jj]) } else { (a * prob.col_up[jj], a * prob.col_lo[jj]) };
            let (jl, ju) = contrib(j, aj);
            let (kl, ku) = contrib(k, ak);
            let fixed_act = aj * xj + ak * xk;
            // 上限側: 活動が増える向き (dk > 0) なら最大活動で確かめる
            if prob.row_up[i].is_finite() && dk > 0.0 {
                if mx_inf[i] > 0 {
                    return false;
                }
                if mx[i] - ju - ku + fixed_act > prob.row_up[i] + 1e-9 * (1.0 + prob.row_up[i].abs()) {
                    return false;
                }
            }
            if prob.row_lo[i].is_finite() && dk < 0.0 {
                if mn_inf[i] > 0 {
                    return false;
                }
                if mn[i] - jl - kl + fixed_act < prob.row_lo[i] - 1e-9 * (1.0 + prob.row_lo[i].abs()) {
                    return false;
                }
            }
            true
        })
    };
    let mut fixed = vec![false; n];
    let mut out = Vec::new();
    let mut pairs = 0usize;
    let (mut pred_pairs, max_pred_pairs) = (0usize, tunable!("ENOMOTO_T_MIP_DOMCOL_PRED_MAX_PAIRS", 1_000_000usize, usize));
    let mut cand: Vec<(usize, f64, bool)> = Vec::new(); // (k, sk, 衝突しているか)
    'outer: for j in 0..n {
        if !binary[j] {
            continue;
        }
        for sj in [1.0, -1.0] {
            if fixed[j] {
                continue 'outer;
            }
            let lj = if sj > 0.0 { 2 * j as u32 } else { 2 * j as u32 + 1 };
            cand.clear();
            for &lk in g.neighbors(lj) {
                cand.push(((lk / 2) as usize, if lk % 2 == 0 { 1.0 } else { -1.0 }, true));
            }
            // (b) の候補 (y_j の最も短い行)
            if implied_up(j, sj) {
                if let Some(&(i0, _)) = cols[j].iter().min_by_key(|&&(i, _)| prob.rows[i].len()) {
                    for &(k, _) in &prob.rows[i0] {
                        cand.push((k, 1.0, false));
                        cand.push((k, -1.0, false));
                    }
                }
            }
            // (c) の候補: HiGHS の bestRowPlus (文字 y_j の係数が行を締める向き (上限側で正、上限がなく下限側で負) の最も
            // 短い行)。y_j が y_k を支配するなら y_k はその行にある。(a)(b) の候補の後に、別の上限
            // `ENOMOTO_T_MIP_DOMCOL_PRED_MAX_PAIRS` (既定 100 万) の中で調べる (同じ上限にすると長い行で (a)(b) の手間を使い切る。
            // blp-ar98 で固定が 1539 → 16 になった)
            let n_ab = cand.len();
            if pred && pred_pairs < max_pred_pairs {
                let best = cols[j].iter().filter(|&&(i, a)| (if prob.row_up[i].is_finite() { 1.0 } else { -1.0 }) * a * sj > 0.0).min_by_key(|&&(i, _)| prob.rows[i].len());
                if let Some(&(i0, _)) = best {
                    for &(k, _) in &prob.rows[i0] {
                        cand.push((k, 1.0, false));
                        cand.push((k, -1.0, false));
                    }
                }
            }
            let up_j = pred && implied_up(j, sj);
            for (ci, &(k, sk, conf)) in cand.iter().enumerate() {
                if k == j || !binary[k] || fixed[k] {
                    continue;
                }
                if ci < n_ab {
                    pairs += 1;
                    if pairs > max_pairs {
                        break 'outer;
                    }
                } else {
                    pred_pairs += 1;
                    if pred_pairs > max_pred_pairs {
                        break;
                    }
                }
                if dominates(j, sj, k, sk) && (!pred || conf || up_j || worst_case_ok(j, sj, k, sk)) {
                    // y_k = 0: sk = +1 なら x_k = 0、sk = -1 なら x_k = 1
                    fixed[k] = true;
                    out.push((k, if sk > 0.0 { 0.0 } else { 1.0 }));
                }
            }
        }
    }
    out
}

/// 行の疎化 (`ENOMOTO_MIP_SPARSIFY`、既定では使わない。HiGHS の前処理の Sparsify の、フィルインのない半分): 等式の行 `e`
/// (2 列以上) の列がすべて行 `r` に含まれるとき、`r` から `s e` (`s` は消せる列の最も多い倍率) を引いて非零を減らす。
/// 等式の倍数を足し引きするだけなので実行可能領域は変わらず、後処理も要らない。整数列だけの行で係数が整数なら `s` も整数に限る
/// (係数を整数のまま保つ。係数の強化・カットの前提)。書き換えた行は、この呼び出しでは等式として使わず、等式として使った行は
/// 書き換えない (相互に引き合うと系が壊れる)。1 行から複数の等式を引いてよい。手間は 1 本の等式あたり、その最も出現の少ない
/// 列を含む行の数まで。
///
/// physiciansched6-2 では、勤務の数の長い行 (77 列の `Σ ≤ 26`) から同じ列の部分集合の等式 (66 列の `Σ = 18`) を引くと
/// 11 列の `Σ ≤ 8` になる (66 列は互いに素な 18 本の等式 `Σ = 1`)。HiGHS の前処理は 4 本しか書き換えないが、これを切ると
/// Feasibility Jump が解を見つけられない。
/// 書き換えた行の数を返す。
fn sparsify_rows(rows: &mut [Vec<(usize, f64)>], row_lo: &mut [f64], row_up: &mut [f64], is_int: &[bool], n: usize) -> usize {
    let m = rows.len();
    let max_len = tunable!("ENOMOTO_T_MIP_SPARSIFY_MAX_EQ_LEN", 1000usize, usize);
    let mut col_rows: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, r) in rows.iter().enumerate() {
        for &(j, _) in r {
            col_rows[j].push(i);
        }
    }
    // 長い等式から (長い行ほど減らせる非零が多い)
    let mut eqs: Vec<usize> = (0..m).filter(|&i| row_lo[i] == row_up[i] && rows[i].len() >= 2 && rows[i].len() <= max_len).collect();
    eqs.sort_by_key(|&i| std::cmp::Reverse(rows[i].len()));
    let mut changed = vec![false; m];
    let mut pivot = vec![false; m];
    let mut count = 0usize;
    for e in eqs {
        if changed[e] {
            continue;
        }
        let re = rows[e].clone();
        let be = row_lo[e];
        let Some(&(anchor, _)) = re.iter().min_by_key(|&&(j, _)| col_rows[j].len()) else { continue };
        let cand = col_rows[anchor].clone();
        for r in cand {
            if r == e || pivot[r] || rows[r].len() <= re.len() {
                continue;
            }
            let rr = &rows[r];
            // r の係数 (列は昇順でないこともあるので探す表を作る)
            let coef: std::collections::HashMap<usize, f64> = rr.iter().copied().collect();
            if !re.iter().all(|(j, _)| coef.contains_key(j)) {
                continue;
            }
            // 倍率の候補: 各列の a_r / a_e。最も多く出るもの
            let mut ratios: Vec<f64> = re.iter().map(|&(j, a)| coef[&j] / a).collect();
            ratios.sort_by(|a, b| a.total_cmp(b));
            let (mut best, mut best_cnt) = (0.0, 0usize);
            let mut k = 0;
            while k < ratios.len() {
                let mut l = k;
                while l < ratios.len() && (ratios[l] - ratios[k]).abs() <= 1e-12 * (1.0 + ratios[k].abs()) {
                    l += 1;
                }
                if l - k > best_cnt {
                    best_cnt = l - k;
                    best = ratios[k];
                }
                k = l;
            }
            if best_cnt == 0 || best == 0.0 {
                continue;
            }
            // 整数列だけで係数が整数の行は、倍率も整数に限る
            let int_row = rr.iter().all(|&(j, a)| is_int[j] && a == a.round());
            if int_row && (best != best.round() || !re.iter().all(|&(_, a)| a == a.round())) {
                continue;
            }
            let ecoef: std::collections::HashMap<usize, f64> = re.iter().copied().collect();
            let mut nr: Vec<(usize, f64)> = Vec::with_capacity(rr.len());
            for &(j, a) in rr {
                let v = match ecoef.get(&j) {
                    Some(&ae) => {
                        let v = a - best * ae;
                        if v.abs() <= 1e-12 * (1.0 + a.abs()) { 0.0 } else { v }
                    }
                    None => a,
                };
                if v != 0.0 {
                    nr.push((j, v));
                }
            }
            if nr.len() >= rr.len() {
                continue;
            }
            let shift = best * be;
            if env_str!("ENOMOTO_MIP_DEBUG_SPARSIFY").is_some() {
                eprintln!("MIP:   sparsify row {r} (len {} -> {}, [{}, {}]) by equation {e} (len {}, = {be}) x {best}", rr.len(), nr.len(), row_lo[r], row_up[r], re.len());
            }
            if row_lo[r].is_finite() {
                row_lo[r] -= shift;
            }
            if row_up[r].is_finite() {
                row_up[r] -= shift;
            }
            rows[r] = nr;
            if !changed[r] {
                count += 1;
            }
            changed[r] = true;
            pivot[e] = true;
        }
    }
    count
}

/// 行の疎化を HiGHS 1.15.1 の `HPresolve::sparsify` と同じ選び方でする (`ENOMOTO_MIP_SPARSIFY=highs`、既定では使わない)。
/// [`sparsify_rows`] との違い:
/// - 等式は **短い順** (HiGHS の `equations` は (行の長さ, 行) の順の集合で、呼び出しの初めの写しを順に使う)。書き換えた等式も
///   その後に (その時の中身で) 等式として使う
/// - 候補の行は等式の最も出現の少ない列を含む行。フィルインを 1 列まで許す (等式が連続列だけで候補の行に整数列があれば 0。
///   一般整数列 (幅 1.5 超) のフィルインは許さない)。加えて、最も出現の少ない列が一般整数でなければ、2 番目に少ない列を含み
///   最も少ない列を含まない行 (等式のほかの列はすべて含む) も候補にする (等式が連続列だけなら整数列のある行は除く)
/// - 倍率は `|s| <= 1e3` で、消える非零の最も多いもの (同数なら |s| の小さい方)。倍率どうしが `sqrt(1e-6)/|a|` より近いときの
///   扱いも HiGHS と同じ。消える数がフィルインより多ければ書き換える。整数の倍率に限らない
/// - 書き換えで絶対値 1e-9 以下になった係数は消す (`small_matrix_value`)
/// 書き換えた行の数を返す。
fn sparsify_rows_highs(rows: &mut [Vec<(usize, f64)>], row_lo: &mut [f64], row_up: &mut [f64], is_int: &[bool], col_lo: &[f64], col_up: &[f64], n: usize) -> usize {
    use std::collections::{BTreeSet, HashMap};
    let m = rows.len();
    let small = 1e-9;
    let min_nz = 1e-6f64.sqrt();
    let general = |j: usize| is_int[j] && col_up[j] - col_lo[j] > 1.5;
    let mut r: Vec<HashMap<usize, f64>> = rows.iter().map(|row| row.iter().copied().filter(|&(_, a)| a != 0.0).collect()).collect();
    let mut col_rows: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    for (i, row) in r.iter().enumerate() {
        for &j in row.keys() {
            col_rows[j].insert(i);
        }
    }
    let nint = |row: &HashMap<usize, f64>| row.keys().filter(|&&j| is_int[j]).count();
    let mut eqs: Vec<usize> = (0..m).filter(|&i| row_lo[i] == row_up[i] && r[i].len() >= 2).collect();
    eqs.sort_by_key(|&i| (r[i].len(), i));
    let mut changed = vec![false; m];
    let debug = env_str!("ENOMOTO_MIP_DEBUG_SPARSIFY").is_some();
    // possibleScales: 倍率の昇順の (倍率, 数)。数 -1 は数値的に悪い倍率
    let add_scale = |ps: &mut Vec<(f64, i64)>, scale: f64, eqv: f64, candv: f64| {
        let tol = min_nz / eqv.abs();
        let pos = ps.partition_point(|&(s, _)| s < scale - tol);
        if pos < ps.len() && (ps[pos].0 - scale).abs() <= tol {
            if ps[pos].1 == -1 {
                return;
            }
            if (ps[pos].0 * eqv + candv).abs() <= small {
                ps[pos].1 += 1;
            } else {
                ps[pos].1 = -1;
            }
        } else {
            ps.insert(pos, (scale, 1));
        }
    };
    let best_scale = |ps: &[(f64, i64)], misses: i64| -> (f64, i64) {
        let (mut scale, mut num) = (0.0f64, 0i64);
        for &(s, c) in ps {
            if c <= misses {
                continue;
            }
            if c > num || (c == num && s.abs() < scale.abs()) {
                scale = s;
                num = c;
            }
        }
        (scale, num)
    };
    let mut count = 0usize;
    for e in eqs {
        if r[e].len() < 2 || row_lo[e] != row_up[e] {
            continue;
        }
        let mut re: Vec<(usize, f64)> = r[e].iter().map(|(&j, &a)| (j, a)).collect();
        re.sort_unstable_by_key(|x| x.0);
        let rhs = row_lo[e];
        let eq_int = re.iter().filter(|x| is_int[x.0]).count();
        // 最も出現の少ない列と 2 番目
        let (mut c1, mut c2) = (usize::MAX, usize::MAX);
        for &(j, _) in &re {
            if c1 == usize::MAX || col_rows[j].len() < col_rows[c1].len() {
                c2 = c1;
                c1 = j;
            } else if c2 == usize::MAX || col_rows[j].len() < col_rows[c2].len() {
                c2 = j;
            }
        }
        let mut targets: Vec<(usize, f64)> = Vec::new();
        let mut ps: Vec<(f64, i64)> = Vec::new();
        for &cand in &col_rows[c1] {
            if cand == e {
                continue;
            }
            ps.clear();
            let cr = &r[cand];
            let mut misses = 0i64;
            let max_misses = if eq_int == 0 && nint(cr) != 0 { 0 } else { 1 };
            for &(j, a) in &re {
                let Some(&cv) = cr.get(&j) else {
                    if general(j) {
                        misses = 2;
                        break;
                    }
                    misses += 1;
                    if misses > max_misses {
                        break;
                    }
                    continue;
                };
                let scale = -cv / a;
                if scale.abs() > 1e3 {
                    continue;
                }
                add_scale(&mut ps, scale, a, cv);
            }
            if misses > max_misses || ps.is_empty() {
                continue;
            }
            let (scale, num) = best_scale(&ps, misses);
            if num > misses {
                targets.push((cand, scale));
            }
        }
        if !general(c1) && c2 != usize::MAX {
            for &cand in &col_rows[c2] {
                if cand == e || r[cand].contains_key(&c1) {
                    continue;
                }
                let cr = &r[cand];
                if eq_int == 0 && nint(cr) != 0 {
                    continue;
                }
                ps.clear();
                let mut skip = false;
                for &(j, a) in &re {
                    if j == c1 {
                        continue;
                    }
                    let Some(&cv) = cr.get(&j) else {
                        skip = true;
                        break;
                    };
                    let scale = -cv / a;
                    if scale.abs() > 1e3 {
                        continue;
                    }
                    add_scale(&mut ps, scale, a, cv);
                }
                if skip || ps.is_empty() {
                    continue;
                }
                let (scale, num) = best_scale(&ps, 1);
                if num > 1 {
                    targets.push((cand, scale));
                }
            }
        }
        for (row, scale) in targets {
            if debug {
                eprintln!("MIP:   sparsify(highs) row {row} (len {}, [{}, {}]) by equation {e} (len {}, = {rhs}) x {scale}", r[row].len(), row_lo[row], row_up[row], re.len());
            }
            if row_lo[row].is_finite() {
                row_lo[row] += scale * rhs;
            }
            if row_up[row].is_finite() {
                row_up[row] += scale * rhs;
            }
            for &(j, a) in &re {
                let v = r[row].get(&j).copied().unwrap_or(0.0) + scale * a;
                if v.abs() <= small {
                    if r[row].remove(&j).is_some() {
                        col_rows[j].remove(&row);
                    }
                } else if r[row].insert(j, v).is_none() {
                    col_rows[j].insert(row);
                }
            }
            if !changed[row] {
                count += 1;
                changed[row] = true;
            }
        }
    }
    for (i, row) in r.into_iter().enumerate() {
        if changed[i] {
            let mut v: Vec<(usize, f64)> = row.into_iter().collect();
            v.sort_unstable_by_key(|x| x.0);
            rows[i] = v;
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

    /// クリーク行の拡張: `x0 + x1 <= 1` に、x0・x1 の両方と衝突する x2 (`x0 + x2 <= 1`・`x1 + x2 <= 1`) を加え、
    /// 拡張した行に含まれる 2 本の行は消す。x3 は x1 とだけ衝突するので加えない。否定の文字 (`x4 - x5 <= 0` は
    /// `x4 + (1 - x5) <= 1`) も扱う
    #[test]
    fn extend_clique_rows_adds_common_neighbors() {
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, 1.0)], vec![(1, 1.0), (2, 1.0)], vec![(1, 1.0), (3, 1.0)], vec![(4, 1.0), (5, -1.0)], vec![(4, 1.0), (6, 1.0)], vec![(5, -1.0), (6, 1.0)]];
        let m = rows.len();
        let p = MipProblem::from_rows(vec![0.0; 7], vec![1.0; 7], vec![-1.0; 7], 0.0, 1.0, vec![true; 7], rows, vec![f64::NEG_INFINITY; m], vec![1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 0.0]);
        let q = extend_clique_rows(p, false);
        let has = |r: &[(usize, f64)], up: f64| (0..q.m).any(|i| q.rows[i] == r && q.row_up[i] == up);
        assert!(has(&[(0, 1.0), (1, 1.0), (2, 1.0)], 1.0));
        assert!(has(&[(1, 1.0), (3, 1.0)], 1.0));
        // x4 + (1 - x5) + x6 <= 1  <=>  x4 - x5 + x6 <= 0
        assert!(has(&[(4, 1.0), (5, -1.0), (6, 1.0)], 0.0));
        assert_eq!(q.m, 3);
        // 整数解は変わらない: 元の行を満たす点は拡張後も満たす (全列挙)
        for mask in 0u32..128 {
            let x: Vec<f64> = (0..7).map(|j| ((mask >> j) & 1) as f64).collect();
            let orig_ok = [x[0] + x[1], x[0] + x[2], x[1] + x[2], x[1] + x[3], x[4] + x[6]].iter().all(|&v| v <= 1.0) && x[4] - x[5] <= 0.0 && x[6] - x[5] <= 0.0;
            assert_eq!(orig_ok, q.is_feasible(&x, 1e-9), "{x:?}");
        }
    }

    /// 上限側と下限側 (符号の逆の倍) の行が範囲制約 1 本にまとまること
    #[test]
    fn tighten_coefficients_saturates_cover_row() {
        // sum 21 y >= 2 (0-1) -> 2 y0 + 2 y1 + 2 y2 >= 2
        let mut rows = vec![vec![(0, 21.0), (1, 21.0), (2, 21.0)]];
        let (mut lo, mut up) = (vec![2.0], vec![f64::INFINITY]);
        let n = tighten_coefficients(&mut rows, &mut lo, &mut up, &[0.0; 3], &[1.0; 3], &[true; 3], false);
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
            // 整数列の幅は 1-3 (0-1 型と一般整数)
            for j in 0..nb {
                lo[j] = (rnd() * 3.0).floor() - 1.0;
                up[j] = lo[j] + 1.0 + (rnd() * 3.0).floor();
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
            total += tighten_coefficients(&mut rows, &mut rlo, &mut rup, &lo, &up, &is_int, true);
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
            // 整数列のすべての値の組 (各列 up - lo + 1 通り) を数え上げる
            let widths: Vec<usize> = (0..nb).map(|j| (up[j] - lo[j]) as usize + 1).collect();
            let combos: usize = widths.iter().product();
            for mut code in 0..combos {
                let y: Vec<f64> = (0..nb)
                    .map(|j| {
                        let v = lo[j] + (code % widths[j]) as f64;
                        code /= widths[j];
                        v
                    })
                    .collect();
                assert_eq!(feasible(&row, rl, ru, &y), feasible(&rows[0], rlo[0], rup[0], &y), "row {row:?} [{rl}, {ru}] -> {:?} [{}, {}], y {y:?}", rows[0], rlo[0], rup[0]);
            }
        }
        assert!(total > 100, "only {total} coefficients tightened");
    }

    #[test]
    fn sparsify_rows_subtracts_disjoint_equations() {
        // x0 + x1 = 1、x2 + x3 = 1 を x0 + x1 + x2 + x3 + x4 <= 3 から引くと x4 <= 1。等式の行は書き換えない
        let mut rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(2, 1.0), (3, 1.0)], vec![(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0), (4, 1.0)], vec![(0, 2.0), (1, 3.0), (5, 1.0)]];
        let mut lo = vec![1.0, 1.0, f64::NEG_INFINITY, f64::NEG_INFINITY];
        let mut up = vec![1.0, 1.0, 3.0, 4.0];
        let k = sparsify_rows(&mut rows, &mut lo, &mut up, &[true; 6], 6);
        assert_eq!(rows[2], vec![(4, 1.0)]);
        assert_eq!(up[2], 1.0);
        assert_eq!(rows[0].len() + rows[1].len(), 4);
        // 2 x0 + 3 x1 + x5 <= 4: 倍率 2 で x0 を、3 で x1 を消せる (どちらも 1 列)。整数の倍率で 1 列減る
        assert_eq!(rows[3].len(), 2);
        assert_eq!(k, 2);
        // 整数の行で整数でない倍率は使わない: 2 x0 + 2 x1 + x5 <= 3 と 2 x0 + 2 x1 = 2 (倍率 1) は使い、
        // x0 + x1 + x5 <= 1 と 2 x0 + 2 x1 = 2 (倍率 1/2) は使わない
        let mut rows = vec![vec![(0, 2.0), (1, 2.0)], vec![(0, 1.0), (1, 1.0), (5, 1.0)]];
        let mut lo = vec![2.0, f64::NEG_INFINITY];
        let mut up = vec![2.0, 1.0];
        assert_eq!(sparsify_rows(&mut rows, &mut lo, &mut up, &[true; 6], 6), 0);
    }

    /// 支配による固定は最適値を変えない (小さな 0-1 問題を全部数えて比べる)
    #[test]
    fn dominated_binary_fixings_keep_optimum() {
        let mut seed = 12345u64;
        let mut rnd = |k: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % k
        };
        let mut total_fixed = [0usize; 2];
        for _ in 0..2000 {
            let n = 6;
            let m = 1 + rnd(4) as usize;
            let mut rows = Vec::new();
            let (mut rlo, mut rup) = (Vec::new(), Vec::new());
            for _ in 0..m {
                let mut r: Vec<(usize, f64)> = Vec::new();
                for j in 0..n {
                    let (take, a) = (rnd(2) == 0, rnd(5) as f64 - 2.0);
                    if take && a != 0.0 {
                        r.push((j, a));
                    }
                }
                if r.len() < 2 {
                    r = vec![(0, 1.0), (1 + rnd(5) as usize, 1.0)];
                }
                let b = rnd(3) as f64;
                match rnd(3) {
                    0 => {
                        rlo.push(f64::NEG_INFINITY);
                        rup.push(b);
                    }
                    1 => {
                        rlo.push(b - 1.0);
                        rup.push(f64::INFINITY);
                    }
                    _ => {
                        rlo.push(b - 1.0);
                        rup.push(b + 1.0);
                    }
                }
                rows.push(r);
            }
            // 衝突を作るための集合詰め込み行
            rows.push(vec![(rnd(3) as usize, 1.0), (3 + rnd(3) as usize, 1.0)]);
            rlo.push(f64::NEG_INFINITY);
            rup.push(1.0);
            let cost: Vec<f64> = (0..n).map(|_| rnd(5) as f64 - 2.0).collect();
            let p = MipProblem::from_rows(vec![0.0; n], vec![1.0; n], cost, 0.0, 1.0, vec![true; n], rows, rlo, rup);
            let best = |lo: &[f64], up: &[f64]| -> f64 {
                let mut z = f64::INFINITY;
                for mask in 0..(1u32 << n) {
                    let x: Vec<f64> = (0..n).map(|j| ((mask >> j) & 1) as f64).collect();
                    if (0..n).all(|j| x[j] >= lo[j] && x[j] <= up[j]) && p.is_feasible(&x, 1e-9) {
                        z = z.min(p.objective(&x));
                    }
                }
                z
            };
            for pred in [false, true] {
                let fix = dominated_binary_fixings_with(&p, pred);
                let (mut lo, mut up) = (p.col_lo.clone(), p.col_up.clone());
                for &(j, v) in &fix {
                    lo[j] = v;
                    up[j] = v;
                }
                total_fixed[pred as usize] += fix.len();
                assert_eq!(best(&p.col_lo, &p.col_up), best(&lo, &up), "fixings {fix:?} (pred {pred}) change the optimum: rows {:?} lo {:?} up {:?} cost {:?}", p.rows, p.row_lo, p.row_up, p.cost);
            }
        }
        // 予測の境界 (pred) は固定を増やす
        assert!(total_fixed[0] > 0 && total_fixed[1] > total_fixed[0], "{total_fixed:?}");
    }

    /// 双対の推論による連続列の上限 ([`dual_bound_tightening`]) は最適解を落とさない: 0-1 列 x と罰則の連続列 w
    /// (`a x - w <= b`、費用は正) の問題で、総当たりの最適解 (w = max(0, a x - b) の最小) が締めた境界の中にある
    #[test]
    fn dual_bound_tightening_keeps_optimum() {
        let mut seed = 777u64;
        let mut rnd = |k: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % k
        };
        let mut tightened = 0;
        for _ in 0..500 {
            let (nb, nw) = (5usize, 2usize);
            let n = nb + nw;
            let mut rows = Vec::new();
            let (mut rlo, mut rup) = (Vec::new(), Vec::new());
            for w in 0..nw {
                // 罰則の行: Σ a x - w <= b (w を増やすと楽になる)
                let mut r: Vec<(usize, f64)> = Vec::new();
                for j in 0..nb {
                    if rnd(2) == 0 {
                        let a = 1.0 + rnd(4) as f64;
                        r.push((j, a));
                    }
                }
                r.push((nb + w, -1.0));
                rows.push(r);
                rlo.push(f64::NEG_INFINITY);
                rup.push(rnd(4) as f64);
            }
            // 0-1 列だけの行 (被覆・詰め込み)
            let mut cover: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
            cover.insert(0, 1.0);
            for j in 0..nb {
                if rnd(2) == 0 {
                    cover.insert(j, 1.0);
                }
            }
            rows.push(cover.into_iter().collect());
            rlo.push(1.0);
            rup.push(f64::INFINITY);
            let cost: Vec<f64> = (0..n).map(|j| if j < nb { rnd(5) as f64 - 2.0 } else { 1.0 + rnd(3) as f64 }).collect();
            let is_int: Vec<bool> = (0..n).map(|j| j < nb).collect();
            let up0: Vec<f64> = (0..n).map(|j| if j < nb { 1.0 } else { f64::INFINITY }).collect();
            let p = MipProblem::from_rows(vec![0.0; n], up0, cost, 0.0, 1.0, is_int, rows, rlo, rup);
            let mut q = p.clone();
            tightened += dual_bound_tightening(&mut q);
            // 総当たり: x を決めると最適な w は各罰則の行の max(0, a x - b)
            let (mut zbest, mut xbest) = (f64::INFINITY, Vec::new());
            for mask in 0..(1u32 << nb) {
                let mut x: Vec<f64> = (0..nb).map(|j| ((mask >> j) & 1) as f64).collect();
                for w in 0..nw {
                    let act: f64 = p.rows[w].iter().filter(|e| e.0 < nb).map(|&(j, a)| a * x[j]).sum();
                    x.push((act - p.row_up[w]).max(0.0));
                }
                if p.is_feasible(&x, 1e-9) && p.objective(&x) < zbest {
                    zbest = p.objective(&x);
                    xbest = x;
                }
            }
            if xbest.is_empty() {
                continue;
            }
            assert!(q.is_feasible(&xbest, 1e-9) && (0..n).all(|j| xbest[j] >= q.col_lo[j] - 1e-9 && xbest[j] <= q.col_up[j] + 1e-9), "tightened bounds cut off the optimum {xbest:?}: {:?} {:?}", q.col_lo, q.col_up);
        }
        assert!(tightened > 0);
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
        let Some(Presolved::Reduced { prob, postsolve, scaling }) = presolve_mip(&p, false, false, None) else { panic!("expected a reduced problem") };
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
