//! 分枝限定法の本体 (HiGHS の `HighsMipSolver` / `HighsSearch` を簡略化したもの)。
//!
//! 流れ:
//! 1. 根: 制約伝播 → LP 緩和を解く。
//! 2. 待ち行列からノードを取り出し、根まで戻した定義域にノードの分枝を積み直して伝播し、
//!    LP の境界を合わせて (保存した親の基底から) 解く。
//! 3. LP 解が整数なら暫定解の候補、そうでなければ reliability 分岐で変数を選び、
//!    片方の子へそのまま潜り (plunge、LP は直前の基底から warm start)、もう片方を待ち行列へ入れる。
//! 4. 打ち切り値 (暫定解から決まる) 以上の下界のノードは捨てる。

use super::domain::{Domain, FEASTOL};
use super::lp::{LpEngine, LpStatus, SolveLimits, VarStatus};
use super::lp_api::MipLp;
use super::problem::MipProblem;
use super::pseudocost::Pseudocost;
use super::queue::{BoundChange, NodeQueue, OpenNode};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// 求解の設定。
#[derive(Debug, Clone, Copy)]
pub struct MipParams {
    /// 時間上限 [秒]。
    pub time_limit: f64,
    /// ノード数上限。
    pub node_limit: u64,
    /// 相対ギャップ `(上界 - 下界) / max(|上界|, 1)` がこれ以下なら終了する。
    pub rel_gap: f64,
    /// 絶対ギャップがこれ以下なら終了する。
    pub abs_gap: f64,
    /// 進捗を標準エラーに出す。
    pub verbose: bool,
    /// サブ MIP (RENS/RINS の中) として解いているか (サブ MIP の中ではサブ MIP を作らない)。
    pub submip: bool,
    /// 目的値の打ち切り値 (これ以上の解は要らない。最小化形、定数項込み)。
    pub cutoff: f64,
    /// これまでに根で再スタートした回数。
    pub restarts: u32,
}

impl Default for MipParams {
    fn default() -> Self {
        MipParams { time_limit: f64::INFINITY, node_limit: u64::MAX, rel_gap: 1e-4, abs_gap: 1e-6, verbose: false, submip: false, cutoff: f64::INFINITY, restarts: 0 }
    }
}

/// 求解の結果の状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MipStatus {
    Optimal,
    Infeasible,
    Unbounded,
    InfeasibleOrUnbounded,
    TimeLimit,
    NodeLimit,
    NotSolved,
}

/// 求解の結果 (目的値・下界は最小化形、定数項込み)。
#[derive(Debug, Clone)]
pub struct MipResult {
    pub status: MipStatus,
    pub x: Option<Vec<f64>>,
    pub objective: Option<f64>,
    pub best_bound: f64,
    pub nodes: u64,
    pub lp_iterations: u64,
}

thread_local! {
    /// 診断用: 既知の最適解 (縮約後の空間、`ENOMOTO_MIP_DEBUG_SOL`)。この解を含むノードが
    /// 枝刈りされたら理由を表示する (SCIP の debug solution と同様)。
    pub(crate) static DEBUG_SOL: std::cell::RefCell<Option<Vec<f64>>> = const { std::cell::RefCell::new(None) };
}

/// 分枝の決定。
enum BranchAction {
    /// 列 `col` を `value` (LP 値) で分枝する。
    Branch { col: usize, value: f64 },
    /// 強分岐で境界が締まったので LP を解き直す。
    Resolve,
    /// ノードが実行不能 (強分岐の両側が打ち切り)。
    Prune,
}

pub(super) struct Solver<'a, L: MipLp> {
    pub(super) p: &'a MipProblem,
    pub(super) params: MipParams,
    pub(super) dom: Domain,
    pub(super) lp: L,
    pub(super) queue: NodeQueue,
    pub(super) pc: Pseudocost,
    pub(super) incumbent: Option<(f64, Vec<f64>)>,
    pub(super) obj_step: Option<f64>,
    pub(super) start: Instant,
    pub(super) deadline: Option<Instant>,
    pub(super) nodes: u64,
    /// 強分岐に使った LP 反復数。
    pub(super) sb_iters: u64,
    /// ヒューリスティクスに使った LP 反復数。
    pub(super) heur_iters: u64,
    /// 列ごとの lock 数 (下げると違反しうる行の数, 上げると違反しうる行の数)。
    pub(super) locks: Vec<(u32, u32)>,
    /// 擬似乱数の状態。
    pub(super) rng: u64,
    /// 根の LP の目的値と、非基底の構造変数の (列, 下限側か, 被約費用)。暫定解が良くなるたびに
    /// 大域的な被約費用固定に使う。
    pub(super) root_redcost: Option<(f64, Vec<(usize, bool, f64, f64)>)>,
    /// 最後に RINS を試したノード番号。
    pub(super) last_rins: u64,
    /// ノードの LP (強分岐以外) に使った反復数と回数。
    node_iters: u64,
    node_lps: u64,
    /// 解けなかったノード (LP が失敗し、分枝もできなかった)。最適性を主張できなくなる。
    unresolved: bool,
    last_log: Instant,
    /// カット生成に使う変数上下限 (最初の分離で作る)。
    pub(super) vbounds: Option<Rc<super::cuts::VarBounds>>,
    /// 根で作ったカット (係数, 右辺, ノルム)。大域的に成り立つ。ノードで違反していれば LP に戻す。
    pub(super) cut_pool: Vec<(Vec<(usize, f64)>, f64, f64)>,
    /// 列ごとの (行, 係数) (oneopt 用、最初に使うときに作る)。
    pub(super) col_rows: Option<Rc<Vec<Vec<(usize, f64)>>>>,
    /// 求解の開始時 (根の伝播の後) に固定されていた整数列の数 (再スタートの判定用)。
    root_fixed0: usize,
    /// ノードでのダイビングの LP 反復数・呼び出し回数・解を見つけた回数。
    dive_iters: u64,
    dive_calls: u64,
    dive_succ: u64,
}

/// 分枝限定法で解く。
pub fn solve(p: &MipProblem, params: MipParams) -> MipResult {
    // LP の実装: 既定は傾き・切片二段解法 (`ENOMOTO_MIP_LP=own` で分枝限定法専用の単体法)。
    if env_str!("ENOMOTO_MIP_LP").is_some_and(|v| v == "own") {
        solve_with::<LpEngine>(p, params)
    } else {
        solve_with::<crate::simplex::mip_lp::TwoStageLp>(p, params)
    }
}

fn solve_with<L: MipLp>(p: &MipProblem, params: MipParams) -> MipResult {
    let start = Instant::now();
    let deadline = if params.time_limit.is_finite() { Some(start + Duration::from_secs_f64(params.time_limit.max(0.0))) } else { None };
    let mut dom = Domain::new(p);
    let fail = |status, nodes| MipResult { status, x: None, objective: None, best_bound: f64::INFINITY, nodes, lp_iterations: 0 };
    if !dom.propagate(p) {
        return fail(MipStatus::Infeasible, 0);
    }
    dom.commit_root();
    dom.take_changed();
    let lp = L::new(&dom.lo, &dom.up, &p.cost, &p.rows, &p.row_lo, &p.row_up);
    let mut s: Solver<L> = Solver {
        p,
        params,
        dom,
        lp,
        queue: NodeQueue::new(),
        pc: Pseudocost::new(p.n),
        incumbent: None,
        obj_step: p.objective_step(),
        start,
        deadline,
        nodes: 0,
        sb_iters: 0,
        heur_iters: 0,
        locks: super::heuristics::compute_locks(p),
        rng: 0x2545_F491_4F6C_DD1D,
        root_redcost: None,
        last_rins: 0,
        node_iters: 0,
        node_lps: 0,
        unresolved: false,
        last_log: start,
        vbounds: None,
        cut_pool: Vec::new(),
        col_rows: None,
        root_fixed0: 0,
        dive_iters: 0,
        dive_calls: 0,
        dive_succ: 0,
    };
    s.run()
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 診断用: 現在の定義域がデバッグ解を含み、かつその目的値が打ち切り値未満なら `why` を表示する。
    pub(super) fn dbg_lost(&self, why: &str, contained: bool) {
        if self.params.submip || !contained {
            return;
        }
        DEBUG_SOL.with(|d| {
            if let Some(x) = d.borrow().as_ref() {
                let z = self.p.objective(x);
                if z < self.prune_limit() - 1e-6 {
                    eprintln!("MIP_DEBUG_SOL lost at node {}: {why} (debug obj {z}, prune limit {})", self.nodes, self.prune_limit());
                }
            }
        });
    }

    /// 診断用: ノードの分枝がデバッグ解を含むか。
    fn dbg_changes(&self, changes: &[BoundChange]) -> bool {
        if self.params.submip {
            return false;
        }
        DEBUG_SOL.with(|d| d.borrow().as_ref().is_some_and(|x| changes.iter().all(|c| if c.upper { x[c.col] <= c.value + 1e-6 } else { x[c.col] >= c.value - 1e-6 })))
    }

    /// 診断用: 現在の定義域がデバッグ解を含むか (デバッグ解がなければ偽)。
    pub(super) fn dbg_contains(&self) -> bool {
        if self.params.submip {
            return false;
        }
        DEBUG_SOL.with(|d| d.borrow().as_ref().is_some_and(|x| (0..self.p.n).all(|j| x[j] >= self.dom.lo[j] - 1e-6 && x[j] <= self.dom.up[j] + 1e-6)))
    }

    /// 診断用: LP の行 (カットを含む) がデバッグ解で満たされているか調べる。
    pub(super) fn dbg_check_rows(&self, label: &str) {
        if self.params.submip {
            return;
        }
        DEBUG_SOL.with(|d| {
            if let Some(x) = d.borrow().as_ref() {
                for i in 0..self.lp.num_rows() {
                    let act: f64 = self.lp.row(i).iter().map(|&(j, v)| v * x[j]).sum();
                    let (lo, up) = self.lp.row_bounds(i);
                    if act < lo - 1e-6 * (1.0 + lo.abs()) || act > up + 1e-6 * (1.0 + up.abs()) {
                        eprintln!("MIP_DEBUG_SOL {label}: LP row {i} violated: {lo} <= {act} <= {up} (orig rows {})", self.p.m);
                    }
                }
            }
        });
    }

    pub(super) fn time_up(&self) -> bool {
        self.deadline.is_some_and(|d| Instant::now() >= d)
    }

    pub(super) fn limits(&self, iteration_limit: u64) -> SolveLimits {
        SolveLimits { iteration_limit, cutoff: self.prune_limit() - self.p.offset, deadline: self.deadline }
    }

    /// この値以上の下界のノードは捨ててよい (最小化形、定数項込み)。
    pub(super) fn prune_limit(&self) -> f64 {
        match &self.incumbent {
            None => self.params.cutoff,
            Some((z, _)) => {
                let strict = match self.obj_step {
                    Some(step) => z - step + (1e-6 * z.abs().max(1.0)).min(0.5 * step),
                    None => z - 1e-9 * z.abs().max(1.0),
                };
                let gap = z - (self.params.rel_gap * z.abs().max(1.0)).max(self.params.abs_gap);
                strict.min(gap).min(self.params.cutoff)
            }
        }
    }

    pub(super) fn sync_lp(&mut self) {
        for j in self.dom.take_changed() {
            self.lp.set_col_bounds(j, self.dom.lo[j], self.dom.up[j]);
        }
    }

    /// 解 `x` (整数列は丸める) を暫定解として試す。採用したら真。
    pub(super) fn try_incumbent(&mut self, mut x: Vec<f64>) -> bool {
        for j in 0..self.p.n {
            if self.p.is_int[j] {
                x[j] = x[j].round();
            }
        }
        if !self.p.is_feasible(&x, FEASTOL) {
            return false;
        }
        let z = self.p.objective(&x);
        let better = self.incumbent.as_ref().is_none_or(|(inc, _)| z < *inc - 1e-9 * inc.abs().max(1.0));
        if better {
            // oneopt (SCIP の heur_oneopt): 整数列を 1 つずつ目的値の良くなる向きに、行を破らない範囲で動かす
            let mut x = x;
            let mut z = z;
            if env_str!("ENOMOTO_MIP_NO_ONEOPT").is_none() && self.one_opt(&mut x) {
                let z2 = self.p.objective(&x);
                if z2 < z && self.p.is_feasible(&x, FEASTOL) {
                    if self.params.verbose {
                        eprintln!("MIP: oneopt improved {z:.10e} -> {z2:.10e}");
                    }
                    z = z2;
                }
            }
            self.incumbent = Some((z, x));
            self.root_redcost_fixing();
            let lim = self.prune_limit();
            self.queue.prune(lim);
            if self.params.verbose {
                eprintln!("MIP: new incumbent {:.10e} at node {} ({:.2}s)", z, self.nodes, self.start.elapsed().as_secs_f64());
            }
        }
        better
    }

    /// LP 解の整数列を丸めた点を試し、行で少しはみ出るなら整数を固定して連続部分を LP で解き直す。
    pub(super) fn try_lp_solution(&mut self) -> bool {
        let x = self.lp.col_values();
        if self.try_incumbent(x.clone()) {
            return true;
        }
        if !self.p.is_int.iter().any(|&b| !b) {
            return false;
        }
        // 整数を固定して連続変数を解き直す
        let saved = self.lp.save_state();
        for j in 0..self.p.n {
            if self.p.is_int[j] {
                let v = x[j].round();
                self.lp.set_col_bounds(j, v, v);
            }
        }
        let st = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
        let ok = st == LpStatus::Optimal && {
            let y = self.lp.col_values();
            self.try_incumbent(y)
        };
        self.lp.restore_state(&saved);
        ok
    }

    /// 現在の LP 解で整数でない整数列の (列, 値) の一覧。
    pub(super) fn fractional(&self, x: &[f64]) -> Vec<(usize, f64)> {
        (0..self.p.n)
            .filter(|&j| self.p.is_int[j] && (x[j] - x[j].round()).abs() > FEASTOL && self.dom.lo[j] < self.dom.up[j])
            .map(|j| (j, x[j]))
            .collect()
    }

    fn log(&mut self, force: bool) {
        if !self.params.verbose {
            return;
        }
        if !force && self.last_log.elapsed().as_secs_f64() < 2.0 {
            return;
        }
        self.last_log = Instant::now();
        let ub = self.incumbent.as_ref().map(|(z, _)| *z).unwrap_or(f64::INFINITY);
        eprintln!(
            "MIP: {:8.2}s nodes {:8} open {:7} lb {:.8e} ub {:.8e} lp_iters {} sb_iters {}",
            self.start.elapsed().as_secs_f64(),
            self.nodes,
            self.queue.len(),
            self.queue.best_lower_bound(),
            ub,
            self.lp.total_iterations(),
            self.sb_iters
        );
    }

    fn run(&mut self) -> MipResult {
        // LP を使わない局所探索 (Feasibility Jump) で最初の実行可能解を探す。
        {
            let nnz: usize = self.p.rows.iter().map(|r| r.len()).sum();
            let cap = if self.params.time_limit.is_finite() { (0.05 * self.params.time_limit).min(5.0) } else { 5.0 };
            if !self.params.submip && self.feasibility_jump((50 * nnz as u64).clamp(100_000, 50_000_000), cap) && self.params.verbose {
                eprintln!("MIP: feasibility jump found a solution ({:.2}s)", self.start.elapsed().as_secs_f64());
            }
        }
        // 根の LP
        self.root_fixed0 = (0..self.p.n).filter(|&j| self.p.is_int[j] && self.dom.global_lo[j] == self.dom.global_up[j]).count();
        let root_st = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
        match root_st {
            LpStatus::Optimal => {}
            LpStatus::Infeasible => return self.finish(MipStatus::Infeasible, f64::INFINITY),
            LpStatus::Unbounded => return self.finish(MipStatus::InfeasibleOrUnbounded, f64::NEG_INFINITY),
            LpStatus::TimeLimit => return self.finish(MipStatus::TimeLimit, f64::NEG_INFINITY),
            _ => return self.finish(MipStatus::NotSolved, f64::NEG_INFINITY),
        }
        let mut root_obj = self.lp.objective() + self.p.offset;
        let root_iters = self.lp.total_iterations();
        if self.params.verbose {
            eprintln!("MIP: root LP {:.10e} ({} iters, {:.2}s)", root_obj, self.lp.total_iterations(), self.start.elapsed().as_secs_f64());
        }
        // カットを加える前の根の LP 解 (RENS の 2 つ目の近傍に使う)
        let x_root0 = self.lp.col_values();
        // 根の切除平面
        {
            let x = self.lp.col_values();
            if !self.fractional(&x).is_empty() {
                self.simple_rounding(&x);
                if env_str!("ENOMOTO_MIP_NO_CUTS").is_none() && !self.root_cut_loop(root_iters) {
                    return self.finish(MipStatus::NotSolved, root_obj);
                }
                root_obj = self.lp.objective() + self.p.offset;
            }
        }
        // 根のヒューリスティクス
        {
            let x = self.lp.col_values();
            if self.fractional(&x).is_empty() {
                self.try_lp_solution();
            } else {
                macro_rules! heur {
                    ($name:expr, $e:expr) => {{
                        let t = Instant::now();
                        let found = $e;
                        if self.params.verbose {
                            eprintln!("MIP: root heuristic {}: {} ({:.2}s)", $name, if found { "found" } else { "none" }, t.elapsed().as_secs_f64());
                        }
                    }};
                }
                heur!("simple rounding", self.simple_rounding(&x));
                heur!("randomized rounding", self.randomized_rounding(&x, 3));
                heur!("RENS", self.rens(&x));
                if self.incumbent.is_none() && !self.fractional(&x_root0).is_empty() {
                    heur!("RENS (LP before cuts)", self.rens(&x_root0));
                }
                if self.incumbent.is_none() {
                    heur!("feasibility pump", self.feasibility_pump(root_iters));
                }
                if self.incumbent.is_none() {
                    heur!("fractional diving", self.fractional_dive(2 * root_iters + 1000));
                }
                if self.incumbent.is_none() {
                    heur!("vector length diving", self.dive(super::heuristics::DiveKind::VectorLength, 2 * root_iters + 1000, f64::INFINITY));
                }
                // 暫定解 (Feasibility Jump・pump・丸めなどで得たもの) を根の LP 解との RINS で磨く
                if self.incumbent.is_some() && env_str!("ENOMOTO_MIP_NO_ROOT_RINS").is_none() {
                    heur!("RINS", self.rins(&x));
                }
                if self.params.verbose {
                    eprintln!("MIP: after root heuristics: incumbent {:?} ({:.2}s)", self.incumbent.as_ref().map(|(z, _)| *z), self.start.elapsed().as_secs_f64());
                }
            }
        }
        if env_str!("ENOMOTO_MIP_DEBUG_BOUNDS").is_some() {
            DEBUG_SOL.with(|d| {
                if let Some(x) = d.borrow().as_ref() {
                    for j in 0..self.p.n {
                        eprintln!("BND x{j} = {} global [{}, {}] local [{}, {}]", x[j], self.dom.global_lo[j], self.dom.global_up[j], self.dom.lo[j], self.dom.up[j]);
                    }
                }
            });
        }
        self.dbg_check_rows("after root cuts");
        self.dbg_lost("root domain (before redcost fixing)", !self.dbg_contains());
        let dbg_root = self.dbg_contains();
        self.store_root_redcost(root_obj);
        self.root_redcost_fixing();
        if dbg_root && !self.dbg_contains() {
            self.dbg_lost("root redcost fixing", true);
        }
        if let Some(r) = self.maybe_restart() {
            return r;
        }
        self.queue.push(OpenNode { changes: Vec::new(), lower_bound: root_obj, estimate: root_obj, depth: 0, basis: Some(Rc::new(self.lp.basis())), branch: None });
        // 根のノードは LP を解いた状態のままなので、最初の取り出しでは定義域・LP を作り直さない。
        let mut first = true;
        // 潜っている子ノード (定義域に分枝を積んだ状態で次に処理する)。
        let mut plunge: Option<OpenNode> = None;
        let mut plunge_depth = 0usize;
        loop {
            if self.time_up() {
                return self.finish_limit(MipStatus::TimeLimit, plunge.as_ref());
            }
            if self.nodes >= self.params.node_limit {
                return self.finish_limit(MipStatus::NodeLimit, plunge.as_ref());
            }
            let node = match plunge.take() {
                Some(n) => n,
                None => {
                    plunge_depth = 0;
                    let lim = self.prune_limit();
                    self.queue.prune(lim);
                    let Some(n) = self.queue.pop() else { break };
                    if !first {
                        // 根まで戻してノードの分枝を積み直す。
                        self.dom.reset_to_root(self.p);
                        let mut ok = true;
                        for c in &n.changes {
                            if c.upper {
                                self.dom.tighten_upper(self.p, c.col, c.value);
                            } else {
                                self.dom.tighten_lower(self.p, c.col, c.value);
                            }
                            if self.dom.infeasible {
                                ok = false;
                                break;
                            }
                        }
                        if ok {
                            if let Some(b) = &n.basis {
                                let mr = self.lp.num_rows();
                                if b.row.len() == mr {
                                    self.lp.set_basis(b);
                                } else if b.row.len() < mr {
                                    // 後から加えたカットの行は論理変数を基底にする
                                    let mut b2 = (**b).clone();
                                    b2.row.resize(mr, VarStatus::Basic);
                                    self.lp.set_basis(&b2);
                                }
                            }
                        }
                        if !ok {
                            self.dbg_lost("propagation while replaying branches", self.dbg_changes(&n.changes));
                            self.nodes += 1;
                            continue;
                        }
                    }
                    n
                }
            };
            first = false;
            self.nodes += 1;
            self.log(false);
            let dn = self.dbg_changes(&node.changes);
            if node.lower_bound >= self.prune_limit() {
                self.dbg_lost(&format!("node lower bound {}", node.lower_bound), dn);
                self.pc.objlim_leaves += 1;
                continue;
            }
            let stack_before_prop = self.dom.stack_len();
            if dn && !self.dbg_contains() {
                self.dbg_lost("domain excludes the debug solution before node propagation", dn);
            }
            let prop_ok = self.dom.propagate(self.p);
            if let Some((j, up, _, _)) = node.branch {
                self.pc.add_inference(j, up, self.dom.stack_len().saturating_sub(stack_before_prop) as f64);
            }
            if !prop_ok {
                self.pc.infeasible_leaves += 1;
                self.dbg_lost("node propagation", dn);
                if let Some((j, up, _, _)) = node.branch {
                    self.pc.add_cutoff(j, up);
                }
                continue;
            }
            self.sync_lp();
            // ノードの LP を解く (強分岐で境界が締まったら解き直す)。
            let mut node_obj;
            let mut resolves = 0;
            let action = loop {
                let it0 = self.lp.total_iterations();
                let iter_limit = (10 * self.avg_node_iters()).max(20_000);
                let mut st = self.lp.solve(&self.limits(iter_limit));
                if matches!(st, LpStatus::Error | LpStatus::IterationLimit) {
                    // 全論理変数基底から解き直してみる
                    let b = self.lp.basis();
                    let slack = super::lp::Basis { col: b.col.iter().map(|_| VarStatus::Lower).collect(), row: vec![VarStatus::Basic; b.row.len()] };
                    self.lp.set_basis(&slack);
                    st = self.lp.solve(&self.limits(u64::MAX));
                }
                self.node_iters += self.lp.total_iterations() - it0;
                self.node_lps += 1;
                match st {
                    LpStatus::Optimal => {}
                    LpStatus::Infeasible | LpStatus::ObjectiveBound => {
                        if st == LpStatus::Infeasible {
                            self.pc.infeasible_leaves += 1;
                        } else {
                            self.pc.objlim_leaves += 1;
                        }
                        self.dbg_lost(&format!("node LP {st:?}"), dn && self.dbg_contains());
                        if let Some((j, up, _, _)) = node.branch {
                            self.pc.add_cutoff(j, up);
                        }
                        break None;
                    }
                    LpStatus::TimeLimit => return self.finish_limit(MipStatus::TimeLimit, Some(&node)),
                    _ => {
                        // LP が解けない: LP 値なしで分枝する (正しさは保たれる)
                        break Some(self.branch_without_lp());
                    }
                }
                node_obj = self.lp.objective() + self.p.offset;
                if resolves == 0 {
                    if let Some((j, up, parent_val, parent_obj)) = node.branch {
                        let x = self.lp.col_value(j);
                        let delta = (x - parent_val).abs().max(if up { parent_val.ceil() - parent_val } else { parent_val - parent_val.floor() });
                        self.pc.add_observation(j, up, delta, node_obj - parent_obj);
                    }
                }
                if node_obj >= self.prune_limit() {
                    self.pc.objlim_leaves += 1;
                    self.dbg_lost(&format!("node LP objective {node_obj}"), dn && self.dbg_contains());
                    break None;
                }
                let x = self.lp.col_values();
                let frac = self.fractional(&x);
                if frac.is_empty() {
                    self.try_lp_solution();
                    break None;
                }
                // 被約費用による局所的な固定。LP 解が新しい境界から外れたら解き直す。
                if self.incumbent.is_some() && self.local_redcost_fixing(node_obj) {
                    if dn && !self.dbg_contains() {
                        self.dbg_lost("local redcost fixing", true);
                    }
                    if !self.dom.propagate(self.p) {
                        self.dbg_lost("propagation after local redcost fixing", dn && self.dbg_contains());
                        break None;
                    }
                    self.sync_lp();
                    let x2 = self.lp.col_values();
                    let moved = (0..self.p.n).any(|j| x2[j] < self.dom.lo[j] - FEASTOL || x2[j] > self.dom.up[j] + FEASTOL);
                    if moved {
                        resolves += 1;
                        continue;
                    }
                }
                // ノードのヒューリスティクス (安価な単純丸めは毎回、ランダム丸めは予算内で待ち行列から取り出したノードのみ)
                if resolves == 0 {
                    self.simple_rounding(&x);
                    let budget = self.lp.total_iterations() / 20 + 10_000;
                    if node.depth > 0 && plunge_depth == 0 && self.heur_iters < budget {
                        if self.incumbent.is_some() && self.nodes >= self.last_rins + 100 {
                            self.last_rins = self.nodes;
                            self.rins(&x);
                        } else {
                            self.randomized_rounding(&x, 1);
                        }
                    }
                    // ダイビング (SCIP の fracdiving / veclendiving: 深さ 10 ごと、ずらし 3 / 7)
                    if node.depth % 10 == 3 || node.depth % 10 == 7 {
                        let quota = (0.05 * (self.dive_succ + 1) as f64 / (self.dive_calls + 1) as f64 * self.node_iters as f64) as u64 + 1000;
                        if self.dive_iters < quota && env_str!("ENOMOTO_MIP_NO_NODE_DIVE").is_none() {
                            let kind = if node.depth % 10 == 3 { super::heuristics::DiveKind::Fractional } else { super::heuristics::DiveKind::VectorLength };
                            let lb = self.queue.best_lower_bound().min(node_obj);
                            let cutoff = self.prune_limit();
                            let quot = if self.incumbent.is_some() { 0.8 } else { 0.1 };
                            let bound = if cutoff.is_finite() { lb + quot * (cutoff - lb) } else { f64::INFINITY };
                            let it0 = self.lp.total_iterations();
                            self.dive_calls += 1;
                            if self.dive(kind, quota - self.dive_iters, bound) {
                                self.dive_succ += 1;
                            }
                            let used = self.lp.total_iterations() - it0;
                            self.dive_iters += used;
                            self.heur_iters += used;
                        }
                    }
                    if node_obj >= self.prune_limit() {
                        break None;
                    }
                }
                // ノードでの切除平面 (待ち行列から取り出したノードで 1 回)。カットを足すたびに LP を作り直すので
                // 今は遅くなる問題が多く、既定では行わない (ENOMOTO_MIP_NODE_CUTS=1 で有効)。
                // カットプールからのカット (待ち行列から取り出したノードで最大 2 回、各 10 本まで)
                if resolves < 2 && node.depth > 0 && plunge_depth == 0 && env_str!("ENOMOTO_MIP_NO_POOL_CUTS").is_none() && self.pool_cut_round(&x, 10) {
                    resolves += 1;
                    continue;
                }
                if resolves == 0 && node.depth > 0 && plunge_depth == 0 && env_str!("ENOMOTO_MIP_NODE_CUTS").is_some() && self.node_cut_round(&x) {
                    resolves += 1;
                    continue;
                }
                let before_sb = dn && self.dbg_contains();
                let sel = self.select_branch(&frac, node_obj, node.depth);
                if before_sb && !self.dbg_contains() {
                    self.dbg_lost("strong branching tightened bounds", true);
                }
                if before_sb && matches!(sel, BranchAction::Prune) {
                    self.dbg_lost("strong branching pruned the node", true);
                }
                match sel {
                    BranchAction::Resolve => {
                        resolves += 1;
                        if !self.dom.propagate(self.p) {
                            break None;
                        }
                        self.sync_lp();
                        continue;
                    }
                    BranchAction::Prune => break None,
                    BranchAction::Branch { col, value } => break Some(Some((col, value, node_obj, frac))),
                }
            };
            let Some(branch) = action else { continue };
            let Some((col, value, node_obj, frac)) = branch else {
                // LP なしの分枝 (branch_without_lp が子ノードを積んだ)
                continue;
            };
            // 子ノードを作る
            let est_gain: f64 = frac.iter().map(|&(j, v)| self.pc.estimate_gain(j, v - v.floor())).sum();
            let estimate = node_obj + est_gain;
            let f = value - value.floor();
            let down = BoundChange { col, upper: true, value: value.floor() };
            let up = BoundChange { col, upper: false, value: value.ceil() };
            let prefer_up = {
                let cu = self.pc.cost_up(col) * (1.0 - f);
                let cd = self.pc.cost_down(col) * f;
                if (cu - cd).abs() <= 1e-12 * (1.0 + cu.abs()) { f >= 0.5 } else { cu < cd }
            };
            let (first_c, second_c) = if prefer_up { (up, down) } else { (down, up) };
            let basis = Rc::new(self.lp.basis());
            let mk = |c: BoundChange| {
                let mut ch = node.changes.clone();
                ch.push(c);
                OpenNode {
                    changes: ch,
                    lower_bound: node_obj,
                    estimate,
                    depth: node.depth + 1,
                    basis: Some(basis.clone()),
                    branch: Some((col, !c.upper, value, node_obj)),
                }
            };
            self.queue.push(mk(second_c));
            let child = mk(first_c);
            plunge_depth += 1;
            if plunge_depth <= 200 {
                // 潜る: 定義域に分枝を積む (LP は今の基底から続ける)
                if first_c.upper {
                    self.dom.tighten_upper(self.p, first_c.col, first_c.value);
                } else {
                    self.dom.tighten_lower(self.p, first_c.col, first_c.value);
                }
                plunge = Some(child);
            } else {
                self.queue.push(child);
            }
            if self.queue.len() > 200_000 {
                self.queue.drop_bases();
            }
        }
        // 待ち行列が空: 探索終了
        if self.unresolved {
            let lb = self.incumbent.as_ref().map(|(z, _)| *z).unwrap_or(f64::NEG_INFINITY);
            return self.finish(MipStatus::NotSolved, lb);
        }
        match &self.incumbent {
            Some((z, _)) => {
                let z = *z;
                self.finish(MipStatus::Optimal, z)
            }
            None => self.finish(MipStatus::Infeasible, f64::INFINITY),
        }
    }

    /// 根の LP の被約費用を保存する。
    fn store_root_redcost(&mut self, root_obj: f64) {
        let b = self.lp.basis();
        let d = self.lp.reduced_costs();
        let mut v = Vec::new();
        for j in 0..self.p.n {
            if !self.p.is_int[j] {
                continue;
            }
            match b.col[j] {
                VarStatus::Lower if d[j] > 1e-7 => v.push((j, true, d[j], self.dom.lo[j])),
                VarStatus::Upper if d[j] < -1e-7 => v.push((j, false, d[j], self.dom.up[j])),
                _ => {}
            }
        }
        self.root_redcost = Some((root_obj, v));
    }

    /// 根の被約費用と現在の打ち切り値から、大域的に境界を締める。
    fn root_redcost_fixing(&mut self) {
        let Some((z, list)) = self.root_redcost.take() else { return };
        let gap = self.prune_limit() - z;
        if gap.is_finite() && gap >= 0.0 {
            for &(j, at_lower, d, bound) in &list {
                // 根の LP で変数がいた境界 `bound` を基準にする: z(x_j) >= z + d (x_j - bound)
                if at_lower {
                    self.dom.tighten_global(self.p, j, true, bound + gap / d);
                } else {
                    self.dom.tighten_global(self.p, j, false, bound - gap / (-d));
                }
            }
        }
        self.root_redcost = Some((z, list));
    }

    /// 現在の LP の被約費用で、このノード以下で有効な境界を締める。締めたら真。
    fn local_redcost_fixing(&mut self, node_obj: f64) -> bool {
        let gap = self.prune_limit() - node_obj;
        if !(gap.is_finite() && gap >= 0.0) {
            return false;
        }
        let b = self.lp.basis();
        let d = self.lp.reduced_costs();
        let mut changed = false;
        for j in 0..self.p.n {
            if self.dom.is_fixed(j) {
                continue;
            }
            match b.col[j] {
                VarStatus::Lower if d[j] > 1e-7 => {
                    let lo = self.dom.lo[j];
                    changed |= self.dom.tighten_upper(self.p, j, lo + gap / d[j]);
                }
                VarStatus::Upper if d[j] < -1e-7 => {
                    let up = self.dom.up[j];
                    changed |= self.dom.tighten_lower(self.p, j, up - gap / (-d[j]));
                }
                _ => {}
            }
        }
        changed
    }

    pub(super) fn avg_node_iters(&self) -> u64 {
        if self.node_lps == 0 { 1000 } else { self.node_iters / self.node_lps + 1 }
    }

    /// LP が解けなかったノードを、定義域の広い整数列の中点で分ける。分けられる列がなければ未解決として印を付ける。
    fn branch_without_lp(&mut self) -> Option<(usize, f64, f64, Vec<(usize, f64)>)> {
        let mut best: Option<(usize, f64)> = None;
        for j in 0..self.p.n {
            if self.p.is_int[j] && self.dom.lo[j] < self.dom.up[j] {
                let w = self.dom.up[j] - self.dom.lo[j];
                if best.is_none_or(|(_, bw)| w > bw) {
                    best = Some((j, w));
                }
            }
        }
        let Some((j, _)) = best else {
            self.unresolved = true;
            return None;
        };
        let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
        let mid = if lo.is_finite() && up.is_finite() {
            ((lo + up) / 2.0).floor() + 0.5
        } else if lo.is_finite() {
            lo + 0.5
        } else if up.is_finite() {
            up - 0.5
        } else {
            0.5
        };
        let lb = self.queue.best_lower_bound().min(f64::INFINITY);
        let _ = lb;
        // 呼び出し側で子ノードを作るため、値 mid (小数部 0.5) で分枝として返す
        Some((j, mid, f64::NEG_INFINITY, vec![(j, mid)]))
    }

    /// reliability 分岐: 信頼できない候補は強分岐で評価し、スコア最大の列を選ぶ。
    /// 双対退化の度合い (SCIP の `SCIPgetLPDualDegeneracy` の簡略版): 非基底の構造列のうち被約費用が 0 の
    /// 割合と、(基底列 + 被約費用 0 の非基底列) / 行数。
    fn dual_degeneracy(&self) -> (f64, f64) {
        let b = self.lp.basis();
        let d = self.lp.reduced_costs();
        let (mut nb, mut zero, mut basic) = (0usize, 0usize, 0usize);
        for j in 0..self.p.n {
            if self.dom.lo[j] == self.dom.up[j] {
                continue;
            }
            if b.col[j] == VarStatus::Basic {
                basic += 1;
            } else {
                nb += 1;
                if d[j].abs() <= 1e-9 * (1.0 + self.p.cost[j].abs()) {
                    zero += 1;
                }
            }
        }
        let deg = if nb == 0 { 0.0 } else { zero as f64 / nb as f64 };
        let ratio = (basic + zero) as f64 / self.lp.num_rows().max(1) as f64;
        (deg, ratio)
    }

    fn select_branch(&mut self, frac: &[(usize, f64)], node_obj: f64, depth: usize) -> BranchAction {
        if env_str!("ENOMOTO_MIP_SB_OLD").is_some() {
            return self.select_branch_old(frac, node_obj);
        }
        // SCIP の relpscost に倣った強分岐 (branch_relpscost.c `branchExecRelpscost`)。
        let hybrid = env_str!("ENOMOTO_MIP_NO_HYBRID_SCORE").is_none();
        let score_of = |s: &Self, j: usize, v: f64| if hybrid { s.pc.hybrid_score(j, v - v.floor()) } else { s.pc.score(j, v - v.floor()) };
        let mut cands: Vec<(usize, f64, f64)> = frac.iter().map(|&(j, v)| (j, v, score_of(self, j, v))).collect();
        cands.sort_by(|a, b| b.2.total_cmp(&a.2));
        // 予算: ノード LP の反復数 (強分岐を除く) の 0.125 倍 + 25000
        let quot = 0.125 * self.node_iters as f64;
        let maxsb = quot + 25_000.0;
        let sb = self.sb_iters as f64;
        // 双対退化の強いノードでは強分岐をしない (根以外)
        let degenerate = depth > 0 && {
            let (deg, ratio) = self.dual_degeneracy();
            deg >= 0.8 || ratio >= 2.0
        };
        let allow_sb = sb <= maxsb && !degenerate;
        // 信頼度のしきい値: 予算の残りに応じて 1..5
        let mut prio = ((maxsb - sb) / (sb + 1.0)).min(1.0);
        prio = prio.max((quot - sb) / (sb + 1.0)).max(0.0);
        let reliable_thr = (1.0 - prio) * 1.0 + prio * 5.0;
        // 強分岐 LP の反復上限
        let sb_iter_limit = ((2.0 * self.avg_node_iters() as f64 * (1.0 + 20.0 / self.nodes.max(1) as f64)) as u64).clamp(10, 500);
        let unreliable: usize = cands.iter().filter(|&&(j, _, _)| (self.pc.min_observations(j) as f64) < reliable_thr).count();
        let lookahead = (9.0 * (1.0 + unreliable as f64 / cands.len().max(1) as f64)) as usize;
        let mut best: Option<(usize, f64, f64)> = None; // (列, 値, スコア)
        let mut no_improve = 0.0f64;
        let mut tried = 0usize;
        let mut bound_changes = 0usize;
        for &(j, v, pscore) in &cands {
            let is_reliable = (self.pc.min_observations(j) as f64) >= reliable_thr;
            if is_reliable || !allow_sb || tried >= 100 || no_improve >= lookahead as f64 || self.time_up() || (self.sb_iters as f64) > maxsb {
                if best.is_none_or(|(_, _, s)| pscore > s) {
                    best = Some((j, v, pscore));
                }
                continue;
            }
            tried += 1;
            // 強分岐
            let saved = self.lp.save_state();
            let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
            let mut gains = [0.0f64; 2];
            let mut cut = [false; 2];
            for (side, is_up) in [(0usize, false), (1usize, true)] {
                if is_up {
                    self.lp.set_col_bounds(j, v.ceil(), up);
                } else {
                    self.lp.set_col_bounds(j, lo, v.floor());
                }
                let it0 = self.lp.total_iterations();
                let st = self.lp.solve(&self.limits(sb_iter_limit));
                self.sb_iters += self.lp.total_iterations() - it0;
                match st {
                    LpStatus::Optimal => {
                        let o = self.lp.objective() + self.p.offset;
                        gains[side] = (o - node_obj).max(0.0);
                        let delta = if is_up { v.ceil() - v } else { v - v.floor() };
                        self.pc.add_observation(j, is_up, delta, gains[side]);
                        if o >= self.prune_limit() {
                            cut[side] = true;
                        } else {
                            let x = self.lp.col_values();
                            if self.fractional(&x).is_empty() {
                                self.try_lp_solution();
                            }
                        }
                    }
                    LpStatus::Infeasible | LpStatus::ObjectiveBound => {
                        cut[side] = true;
                        self.pc.add_cutoff(j, is_up);
                    }
                    _ => {
                        // 反復上限: 途中の目的値 (双対単体法なので下界) を弱い観測として使う
                        gains[side] = (self.lp.objective() + self.p.offset - node_obj).max(0.0);
                    }
                }
                self.lp.restore_state(&saved);
            }
            match (cut[0], cut[1]) {
                (true, true) => return BranchAction::Prune,
                (true, false) => {
                    self.dom.tighten_lower(self.p, j, v.ceil());
                    bound_changes += 1;
                }
                (false, true) => {
                    self.dom.tighten_upper(self.p, j, v.floor());
                    bound_changes += 1;
                }
                _ => {}
            }
            // 境界の変更は最大 5 個まとめてから解き直す
            if bound_changes >= 5 {
                return BranchAction::Resolve;
            }
            if cut[0] || cut[1] {
                continue;
            }
            let score = if hybrid { self.pc.hybrid_score_with_gains(j, gains[0], gains[1]) } else { gains[0].max(1e-6) * gains[1].max(1e-6) };
            if best.is_none_or(|(_, _, s)| score > s) {
                best = Some((j, v, score));
                no_improve = 0.0;
            } else if best.is_some_and(|(_, _, s)| score >= s * (1.0 - 1e-9)) {
                no_improve += 0.5;
            } else {
                no_improve += 1.0;
            }
        }
        if bound_changes > 0 {
            return BranchAction::Resolve;
        }
        match best {
            Some((col, value, _)) => BranchAction::Branch { col, value },
            None => BranchAction::Prune,
        }
    }

    fn select_branch_old(&mut self, frac: &[(usize, f64)], node_obj: f64) -> BranchAction {
        let mut cands: Vec<(usize, f64, f64)> = frac.iter().map(|&(j, v)| (j, v, self.pc.score(j, v - v.floor()))).collect();
        cands.sort_by(|a, b| b.2.total_cmp(&a.2));
        let total = self.lp.total_iterations();
        let budget = total / 2 + 100_000;
        let mut best: Option<(usize, f64, f64)> = None; // (列, 値, スコア)
        let mut no_improve = 0;
        let sb_cap = tunable!("ENOMOTO_T_MIP_SB_ITER_CAP", 2_000u64, u64);
        let sb_floor = tunable!("ENOMOTO_T_MIP_SB_ITER_FLOOR", 200u64, u64);
        let sb_iter_limit = (2 * self.avg_node_iters()).clamp(sb_floor, sb_cap.max(sb_floor));
        let lookahead = tunable!("ENOMOTO_T_MIP_SB_LOOKAHEAD", 8usize, usize);
        for &(j, v, pscore) in &cands {
            let reliable = self.pc.is_reliable(j);
            if reliable || self.sb_iters > budget || no_improve >= lookahead || self.time_up() {
                if best.is_none_or(|(_, _, s)| pscore > s) {
                    best = Some((j, v, pscore));
                }
                continue;
            }
            // 強分岐
            let saved = self.lp.save_state();
            let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
            let mut gains = [0.0f64; 2];
            let mut cut = [false; 2];
            for (side, is_up) in [(0usize, false), (1usize, true)] {
                if is_up {
                    self.lp.set_col_bounds(j, v.ceil(), up);
                } else {
                    self.lp.set_col_bounds(j, lo, v.floor());
                }
                let it0 = self.lp.total_iterations();
                let st = self.lp.solve(&self.limits(sb_iter_limit));
                self.sb_iters += self.lp.total_iterations() - it0;
                if env_str!("ENOMOTO_MIP_DEBUG_SB").is_some() {
                    eprintln!("SB j={j} up={is_up} st={st:?} iters={} limit={sb_iter_limit}", self.lp.total_iterations() - it0);
                }
                match st {
                    LpStatus::Optimal => {
                        let o = self.lp.objective() + self.p.offset;
                        gains[side] = (o - node_obj).max(0.0);
                        let delta = if is_up { v.ceil() - v } else { v - v.floor() };
                        self.pc.add_observation(j, is_up, delta, gains[side]);
                        if o >= self.prune_limit() {
                            cut[side] = true;
                        } else {
                            // 強分岐の LP 解が整数なら暫定解の候補
                            let x = self.lp.col_values();
                            if self.fractional(&x).is_empty() {
                                self.try_lp_solution();
                            }
                        }
                    }
                    LpStatus::Infeasible | LpStatus::ObjectiveBound => {
                        cut[side] = true;
                        self.pc.add_cutoff(j, is_up);
                    }
                    _ => {
                        // 反復上限など: 途中の目的値を目安にする
                        gains[side] = (self.lp.objective() + self.p.offset - node_obj).max(0.0);
                    }
                }
                self.lp.restore_state(&saved);
            }
            match (cut[0], cut[1]) {
                (true, true) => return BranchAction::Prune,
                (true, false) => {
                    // 下側が打ち切り → 上側に固定
                    self.dom.tighten_lower(self.p, j, v.ceil());
                    return BranchAction::Resolve;
                }
                (false, true) => {
                    self.dom.tighten_upper(self.p, j, v.floor());
                    return BranchAction::Resolve;
                }
                _ => {}
            }
            let score = gains[0].max(1e-6) * gains[1].max(1e-6);
            if best.is_none_or(|(_, _, s)| score > s) {
                best = Some((j, v, score));
                no_improve = 0;
            } else {
                no_improve += 1;
            }
        }
        match best {
            Some((col, value, _)) => BranchAction::Branch { col, value },
            None => BranchAction::Prune,
        }
    }

    fn finish_limit(&mut self, status: MipStatus, current: Option<&OpenNode>) -> MipResult {
        let mut lb = self.queue.best_lower_bound();
        if let Some(n) = current {
            lb = lb.min(n.lower_bound);
        }
        if let Some((z, _)) = &self.incumbent {
            lb = lb.min(*z);
        }
        self.finish(status, lb)
    }

    /// 根での再スタート (SCIP の `restartfac`): 根の処理で新たに大域固定された整数列が全体の 2.5% を超えたら、
    /// 大域的な境界と LP に残ったカットを持って前処理からやり直す。暫定解は打ち切り値として渡し、
    /// やり直しで良い解が見つからなければそれを返す。最大 2 回。
    fn maybe_restart(&mut self) -> Option<MipResult> {
        let p = self.p;
        if self.params.submip || self.params.restarts >= 2 || env_str!("ENOMOTO_MIP_NO_RESTART").is_some() || self.time_up() {
            return None;
        }
        let nint = p.is_int.iter().filter(|&&b| b).count();
        let fixed = (0..p.n).filter(|&j| p.is_int[j] && self.dom.global_lo[j] == self.dom.global_up[j]).count();
        let newly = fixed.saturating_sub(self.root_fixed0);
        if nint == 0 || (newly as f64) <= tunable!("ENOMOTO_T_MIP_RESTART_FAC", 0.025, f64) * nint as f64 {
            return None;
        }
        // 新しい問題: 大域的な境界 + 元の行 + LP に残ったカット (どれも大域的に成り立つ)
        let mut rows = p.rows.clone();
        let mut row_lo = p.row_lo.clone();
        let mut row_up = p.row_up.clone();
        for i in p.m..self.lp.num_rows() {
            let (l, u) = self.lp.row_bounds(i);
            rows.push(self.lp.row(i));
            row_lo.push(l);
            row_up.push(u);
        }
        // 伝播の丸め誤差で下限 > 上限 (ごくわずか) になった列は 1 点に固定する
        let mut glo = self.dom.global_lo.clone();
        let mut gup = self.dom.global_up.clone();
        for j in 0..p.n {
            if glo[j] > gup[j] {
                let v = if p.is_int[j] { glo[j].round() } else { 0.5 * (glo[j] + gup[j]) };
                glo[j] = v;
                gup[j] = v;
            }
        }
        let newp = MipProblem::from_rows(glo, gup, p.cost.clone(), p.offset, p.sense_sign, p.is_int.clone(), rows, row_lo, row_up);
        let remaining = match self.deadline {
            Some(d) => d.saturating_duration_since(Instant::now()).as_secs_f64(),
            None => f64::INFINITY,
        };
        let params = MipParams {
            time_limit: remaining,
            node_limit: self.params.node_limit.saturating_sub(self.nodes),
            cutoff: self.prune_limit().min(self.params.cutoff),
            restarts: self.params.restarts + 1,
            ..self.params
        };
        if self.params.verbose {
            eprintln!(
                "MIP: restart {} ({} of {} integer columns newly fixed at the root, {} cuts kept) ({:.2}s)",
                params.restarts,
                newly,
                nint,
                self.lp.num_rows() - p.m,
                self.start.elapsed().as_secs_f64()
            );
        }
        let r = super::solve_problem(&newp, params, true);
        let nodes = self.nodes + r.nodes;
        let iters = self.lp.total_iterations() + r.lp_iterations;
        // 良い方の解を採る
        let (x, objective) = match (&r.x, r.objective, &self.incumbent) {
            (Some(x2), Some(z2), Some((z, x))) => {
                if z2 < *z {
                    (Some(x2.clone()), Some(z2))
                } else {
                    (Some(x.clone()), Some(*z))
                }
            }
            (Some(x2), Some(z2), None) => (Some(x2.clone()), Some(z2)),
            (_, _, Some((z, x))) => (Some(x.clone()), Some(*z)),
            _ => (None, None),
        };
        let (status, best_bound) = match r.status {
            // 打ち切り値より良い解が無い/最適: 暫定解があれば最適
            MipStatus::Optimal | MipStatus::Infeasible => match objective {
                Some(z) => (MipStatus::Optimal, z),
                None => (MipStatus::Infeasible, f64::INFINITY),
            },
            s => (s, r.best_bound.min(objective.unwrap_or(f64::INFINITY))),
        };
        Some(MipResult { status, x, objective, best_bound, nodes, lp_iterations: iters })
    }

    fn finish(&mut self, status: MipStatus, best_bound: f64) -> MipResult {
        self.log(true);
        let (x, objective) = match &self.incumbent {
            Some((z, x)) => (Some(x.clone()), Some(*z)),
            None => (None, None),
        };
        // 実行不能と報告する前に暫定解があればそれは最適
        let status = match (status, &self.incumbent) {
            (MipStatus::Infeasible, Some(_)) => MipStatus::Optimal,
            (s, _) => s,
        };
        MipResult { status, x, objective, best_bound, nodes: self.nodes, lp_iterations: self.lp.total_iterations() }
    }
}
