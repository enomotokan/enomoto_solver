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

/// 証明 `sum a_j x_j` の境界 `lo`/`up` での最小活動量 (有限部分, 無限の項の数) と `max_j |a_j| (u_j - l_j)`
/// (余裕がこれ以上ならどの境界も締まらない。境界は締まるだけなので以後も上界)。
fn proof_min_activity(coefs: &[(usize, f64)], lo: &[f64], up: &[f64]) -> (f64, u32, f64) {
    let (mut m, mut ninf, mut cap) = (0.0f64, 0u32, 0.0f64);
    for &(j, a) in coefs {
        let v = if a > 0.0 { a * lo[j] } else { a * up[j] };
        if v.is_finite() {
            m += v;
        } else {
            ninf += 1;
        }
        cap = cap.max(a.abs() * (up[j] - lo[j]));
    }
    (m, ninf, cap)
}

/// 診断用の区間計時 (`ENOMOTO_MIP_XPROF`)。
fn xp(label: &'static str) {
    crate::simplex::slope_intercept_dual::xprof(label);
}
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
    /// 根で呼ばないヒューリスティクス ([`heur_bit`] のビット)。再スタート前の根で解を見つけなかったもの
    /// (問題は固定が増えただけでほぼ同じなので、同じ結果になりやすく時間だけかかる)。
    pub skip_heurs: u64,
}

/// 根のヒューリスティクスの名前に対応するビット (再スタートで飛ばすものの印)。暫定解に依る RINS と、
/// 一覧にないものは 0 (飛ばさない)。
pub(super) fn heur_bit(name: &str) -> u64 {
    const NAMES: [&str; 20] = [
        "simple rounding", "ZI round", "trivial", "randomized rounding", "interior rounding", "root reduced cost", "LP face", "RENS",
        "RENS (LP before cuts)", "feasibility pump", "fractional diving", "vector length diving", "coefficient diving", "shift-and-propagate",
        "locks", "clique", "vbounds (loose)", "vbounds (tight)", "min relaxation", "repair",
    ];
    NAMES.iter().position(|&n| n == name).map_or(0, |k| 1u64 << k)
}

impl Default for MipParams {
    fn default() -> Self {
        MipParams { time_limit: f64::INFINITY, node_limit: u64::MAX, rel_gap: 1e-4, abs_gap: 1e-6, verbose: false, submip: false, cutoff: f64::INFINITY, restarts: 0, skip_heurs: 0 }
    }
}

/// LP の行の変更 ([`Solver::row_log`])。
#[derive(Debug, Clone)]
pub(super) enum RowEdit {
    /// 末尾に k 行加えた。
    Add(usize),
    /// 印のついた行を消した (長さは消す前の行数)。
    Delete(Vec<bool>),
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
    /// 次に始める求解 ([`solve`]) の最初の暫定解 (その問題の空間)。再スタートで前の暫定解を引き継ぐのに使う。
    /// 求解の開始時に取り出す (同じスレッドで後から始まるサブ MIP には渡らない)。
    pub(crate) static START_SOL: std::cell::RefCell<Option<Vec<f64>>> = const { std::cell::RefCell::new(None) };
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
    /// 強分岐に使った時間 (秒) と、強分岐で解いた LP の数。
    pub(super) sb_secs: f64,
    pub(super) sb_lps: u64,
    /// 直前の分枝の選択で、選んだ列の強分岐の子の LP 値 (列, 下の子, 上の子)。最適まで解けなかった側は -inf。
    pub(super) last_sb: Option<(usize, f64, f64)>,
    /// clique カット用の 2 値列の衝突グラフ (根で作る)。
    pub(super) clique_graph: Option<std::rc::Rc<super::clique::CliqueGraph>>,
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
    /// Local Branching の近傍の大きさ (結果で調整する)。
    pub(super) lb_k: f64,
    /// ノードで回す改善ヒューリスティクスの順番。
    pub(super) improve_turn: u64,
    /// ALNS の腕ごとの報酬の合計と試した回数。
    pub(super) alns_reward: [f64; 6],
    pub(super) alns_count: [u32; 6],
    /// 実行可能解のプール (目的値の良い順、Crossover 用)。
    pub(super) sol_pool: Vec<(f64, Vec<f64>)>,
    /// Repair の出発点 (局所探索で違反の合計が最小だった点)。
    pub(super) repair_start: Option<Vec<f64>>,
    /// 大近傍探索 (サブ MIP) に使った時間の合計 (秒)。
    pub(super) lns_secs: f64,
    /// サブ MIP の時間の上限 (残り時間に対する割合)。
    pub(super) submip_time_frac: f64,
    /// 並列モードで、サブ MIP を別スレッドで解くか (根のヒューリスティクスの間だけ真)。
    pub(super) parallel_submips: bool,
    /// 別スレッドで解いているサブ MIP (出した順)。
    pub(super) pending_submips: std::collections::VecDeque<std::thread::JoinHandle<MipResult>>,
    /// 別スレッドの Feasibility Jump とその停止の印。
    fj_thread: Option<(std::thread::JoinHandle<Option<Vec<f64>>>, std::sync::Arc<std::sync::atomic::AtomicBool>)>,
    /// RENS の固定率の記録 (成功したときの固定率の合計と回数、サブ MIP が実行不能だったときの合計と回数)。
    pub(super) rens_succ: (f64, u32),
    pub(super) rens_infeas: (f64, u32),
    /// ノードの LP (強分岐以外) に使った反復数と回数。
    node_iters: u64,
    /// ノードの最初の LP (カット・強分岐後の解き直しを除く) の反復数
    node_iters_first: u64,
    /// 基底の復元の回数、そのうち基底の数が行数と合わなかった回数、復元後の最初の LP の反復数 (診断用)
    restore_stats: (u64, u64, u64),
    restored_now: bool,
    node_lps: u64,
    /// ノードの LP にかかった時間の合計 (秒)。1 回の LP の時間の上限に使う。
    node_lp_secs: f64,
    /// 解けなかったノード (LP が失敗し、分枝もできなかった)。最適性を主張できなくなる。
    unresolved: bool,
    last_log: Instant,
    /// カット生成に使う変数上下限 (最初の分離で作る)。
    pub(super) vbounds: Option<Rc<super::cuts::VarBounds>>,
    /// 根で解を見つけなかったヒューリスティクス ([`heur_bit`] のビット)。再スタートで引き継ぐ。
    failed_heurs: u64,
    /// LP の行の追加・削除の記録 (待ち行列のノードの基底を、保存した後の行の変化に合わせるのに使う)。
    pub(super) row_log: Vec<RowEdit>,
    /// LP のカットの行 (元の行より後ろ) の年齢: 続けて効いていなかったノードの LP の数 ([`Self::age_cuts`])。
    pub(super) cut_age: Vec<u32>,
    /// 証明から作った衝突の数。
    proof_conflicts: u64,
    /// 完全オービトープ (orbitopal fixing に使う。サブ MIP では空)。
    orbitopes: Rc<Vec<super::symmetry::Orbitope>>,
    /// 対称性の行・固定を加える前の問題 (なければ `None`)。ヒューリスティクスの解の判定・局所探索に使う。
    pub(super) orig: Option<Rc<MipProblem>>,
    /// オービトープの列 -> (オービトープの番号, 行)。
    orb_col: std::collections::HashMap<usize, (usize, usize)>,
    /// 今の潜りで待ち行列に入れた兄弟ノード ((枠, 世代)、古い順)。潜りが枝刈りで終わったらここから戻る。
    dive_stack: Vec<(usize, u64)>,
    sibling_backtracks: u64,
    /// 元の問題では実行可能で、オービトープの列の並べ替えで使えるようにした解の数と、それでも使えなかった数。
    sym_canon: (u64, u64),
    /// orbitopal fixing で固定した数と、それで枝刈りしたノードの数。
    orbitope_fixings: u64,
    orbitope_prunes: u64,
    /// カット生成で整数として扱う列 (整数列と暗黙の整数列、[`MipProblem::implied_integers`])。
    pub(super) cut_int: Vec<bool>,
    /// 根で作ったカット (係数, 右辺, ノルム)。大域的に成り立つ。ノードで違反していれば LP に戻す。
    pub(super) cut_pool: Vec<(Vec<(usize, f64)>, f64, f64)>,
    /// 双対証明 (衝突分析): `sum coefs x <= U - konst` (U は打ち切り値から定数項を引いたもの、最新の値を使う)。
    pub(super) dual_proofs: std::collections::VecDeque<(Vec<(usize, f64)>, f64, bool)>,
    pub(super) dual_proof_nnz: usize,
    /// 双対証明の差分評価用 ([`Self::apply_dual_proofs`]): 先頭の証明の通し番号、各証明の基準の最小活動量
    /// (`proof_snap` の境界での有限部分と無限の項の数)、列から (証明の通し番号, 係数) への索引、基準の境界
    /// (根の状態) とそれを取ったノード数、作業領域。
    proof_first_id: u64,
    proof_base: std::collections::VecDeque<(f64, u32, f64)>,
    proof_col_index: Vec<Vec<(u64, f64)>>,
    proof_snap: Option<(Vec<f64>, Vec<f64>)>,
    proof_snap_nodes: u64,
    /// 基準を取ったときの打ち切り値 (暫定解が良くなったら取り直す)。
    proof_snap_limit: f64,
    proof_delta: Vec<(f64, i32)>,
    proof_touched: Vec<usize>,
    proof_in_touched: Vec<bool>,
    /// 根の状態の境界でも締め付けが起こりうる (余裕が `max |a_j| (u_j - l_j)` 未満の) 証明の通し番号。
    /// 変えた列を含まなくても毎回調べる。
    proof_always: Vec<u64>,
    /// `proof_delta` が記録のこの位置までを反映しているか。`proof_dirty` なら作り直す。
    /// `proof_delta` が反映している各列の境界 (この値から今の境界への差を次に足す)。
    proof_val_lo: Vec<f64>,
    proof_val_up: Vec<f64>,
    proof_dirty: bool,
    /// 作った衝突制約の数。
    conflicts_added: u64,
    /// 作った実行不能の証明の数。
    farkas_added: u64,
    /// 双対証明で枝刈りしたノード数と、締めた境界の数 (表示用)。
    pub(super) proof_prunes: u64,
    pub(super) proof_tightenings: u64,
    /// カットプールの重複判定用のハッシュと、プールの非零数。
    pub(super) cut_pool_keys: std::collections::HashSet<u64>,
    pub(super) cut_pool_nnz: usize,
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
/// 並列モードのスレッド数 (`ENOMOTO_MIP_THREADS`、既定 1 = 並列にしない)。
pub(crate) fn mip_threads() -> usize {
    tunable!("ENOMOTO_MIP_THREADS", 1usize, usize).max(1)
}

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
        sb_secs: 0.0,
        sb_lps: 0,
        last_sb: None,
        clique_graph: None,
        heur_iters: 0,
        locks: super::heuristics::compute_locks(p),
        rng: 0x2545_F491_4F6C_DD1D,
        root_redcost: None,
        last_rins: 0,
        lb_k: 18.0,
        improve_turn: 0,
        alns_reward: [0.0; 6],
        alns_count: [0; 6],
        sol_pool: Vec::new(),
        repair_start: None,
        lns_secs: 0.0,
        submip_time_frac: 0.07,
        parallel_submips: false,
        pending_submips: std::collections::VecDeque::new(),
        fj_thread: None,
        rens_succ: (0.0, 0),
        rens_infeas: (0.0, 0),
        node_iters: 0,
        node_iters_first: 0,
        restore_stats: (0, 0, 0),
        restored_now: false,
        node_lps: 0,
        node_lp_secs: 0.0,
        unresolved: false,
        last_log: start,
        vbounds: None,
        failed_heurs: 0,
        cut_age: Vec::new(),
        proof_conflicts: 0,
        orbitopes: if params.submip { Rc::new(Vec::new()) } else { super::ORBITOPES.with(|t| t.borrow().clone()).unwrap_or_default() },
        orbitope_fixings: 0,
        orig: if params.submip || env_str!("ENOMOTO_MIP_SYM_NO_ORIG").is_some() { None } else { super::SYM_ORIG.with(|t| t.borrow().clone()) },
        sym_canon: (0, 0),
        orb_col: std::collections::HashMap::new(),
        dive_stack: Vec::new(),
        sibling_backtracks: 0,
        orbitope_prunes: 0,
        row_log: Vec::new(),
        cut_int: if env_str!("ENOMOTO_MIP_NO_IMPLINT").is_some() { p.is_int.clone() } else { p.implied_integers() },
        cut_pool: Vec::new(),
        dual_proofs: std::collections::VecDeque::new(),
        dual_proof_nnz: 0,
        proof_first_id: 0,
        proof_base: std::collections::VecDeque::new(),
        proof_col_index: Vec::new(),
        proof_snap: None,
        proof_snap_nodes: 0,
        proof_snap_limit: f64::INFINITY,
        proof_delta: Vec::new(),
        proof_touched: Vec::new(),
        proof_in_touched: Vec::new(),
        proof_always: Vec::new(),
        proof_val_lo: Vec::new(),
        proof_val_up: Vec::new(),
        proof_dirty: true,
        conflicts_added: 0,
        farkas_added: 0,
        proof_prunes: 0,
        proof_tightenings: 0,
        cut_pool_keys: std::collections::HashSet::new(),
        cut_pool_nnz: 0,
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
    /// このノードで新しいカットを分離するか (SCIP の separator の freq と同じく、深さが `freq` の倍数のノードだけ。
    /// `ENOMOTO_T_MIP_NODE_CUT_FREQ`、既定 10、0 なら分離しない。`ENOMOTO_MIP_NODE_CUTS` なら毎回)。
    fn node_cuts_due(&self, depth: usize) -> bool {
        if env_str!("ENOMOTO_MIP_NODE_CUTS").is_some() {
            return true;
        }
        let freq = tunable!("ENOMOTO_T_MIP_NODE_CUT_FREQ", 10usize, usize);
        freq > 0 && depth % freq == 0
    }

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
            // 対称性の行・固定だけに反する解: 元の問題で実行可能なら、オービトープの列を辞書式の順に並べ替える
            // (対称性で写すので実行可能性と目的値は変わらない)
            let Some(orig) = self.orig.clone() else { return false };
            if !orig.is_feasible(&x, FEASTOL) {
                return false;
            }
            for o in self.orbitopes.iter() {
                o.canonicalize(&mut x);
            }
            if !self.p.is_feasible(&x, FEASTOL) {
                self.sym_canon.1 += 1;
                return false;
            }
            self.sym_canon.0 += 1;
        }
        let z = self.p.objective(&x);
        self.pool_add(z, &x);
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
            self.pool_add(z, &x);
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
            "MIP: {:8.2}s nodes {:8} open {:7} lb {:.8e} ub {:.8e} lp_iters {} sb_iters {} sb_lps {} sb_secs {:.2} node_iters {} (first {}, lps {}) heur_iters {} dive_iters {} lp_rows {}",
            self.start.elapsed().as_secs_f64(),
            self.nodes,
            self.queue.len(),
            self.queue.best_lower_bound(),
            ub,
            self.lp.total_iterations(),
            self.sb_iters,
            self.sb_lps,
            self.sb_secs,
            self.node_iters,
            self.node_iters_first,
            self.node_lps,
            self.heur_iters,
            self.dive_iters,
            self.lp.num_rows()
        );
    }

    fn run(&mut self) -> MipResult {
        // 再スタート前の暫定解 (実行可能性は try_incumbent が確かめる)
        if let Some(cuts) = super::RESTART_CUTS.with(|s| s.borrow_mut().take()) {
            let n0 = self.cut_pool.len();
            for (c, r) in &cuts {
                if c.iter().all(|&(j, _)| j < self.p.n) {
                    self.add_to_pool(c, *r);
                }
            }
            if self.params.verbose {
                eprintln!("MIP: {} cuts from before the restart put in the cut pool", self.cut_pool.len() - n0);
            }
        }
        if let Some(x) = START_SOL.with(|s| s.borrow_mut().take()) {
            if x.len() == self.p.n {
                let ok = self.try_incumbent(x);
                if self.params.verbose {
                    eprintln!("MIP: start solution from before the restart: accepted {ok}");
                }
            }
        }
        // LP を使わない局所探索 (Feasibility Jump) で最初の実行可能解を探す。
        {
            let nnz: usize = self.p.rows.iter().map(|r| r.len()).sum();
            let cap = if self.params.time_limit.is_finite() { (0.05 * self.params.time_limit).min(5.0) } else { 5.0 };
            let effort = (50 * nnz as u64).clamp(100_000, 50_000_000);
            if !self.params.submip && mip_threads() > 1 {
                // 並列モード: 別スレッドで根の LP・切除平面と同時に回す (根のヒューリスティクスの前に受け取る)
                // 対称性の行・固定を加える前の問題で探す (見つけた解は try_incumbent で並べ替える)
                let (pc, lo, up) = match &self.orig {
                    Some(o) => (std::sync::Arc::new((**o).clone()), o.col_lo.clone(), o.col_up.clone()),
                    None => (std::sync::Arc::new(self.p.clone()), self.dom.lo.clone(), self.dom.up.clone()),
                };
                let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let stop2 = stop.clone();
                let seed = self.rng ^ 0x9E37_79B9_7F4A_7C15;
                let h = std::thread::spawn(move || super::heuristics::fj_search(&pc, &lo, &up, effort, cap, seed, Some(&stop2)));
                self.fj_thread = Some((h, stop));
            } else if !self.params.submip && self.feasibility_jump(effort, cap) && self.params.verbose {
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
        self.join_fj_thread();
        // 根のヒューリスティクス
        {
            let x = self.lp.col_values();
            // 並列モード: サブ MIP を使うヒューリスティクスは別スレッドで解かせる
            self.parallel_submips = mip_threads() > 1 && !self.params.submip;
            if self.fractional(&x).is_empty() {
                self.try_lp_solution();
            } else {
                macro_rules! heur {
                    ($name:expr, $e:expr) => {{
                        let bit = heur_bit($name);
                        if self.params.skip_heurs & bit != 0 && env_str!("ENOMOTO_MIP_RESTART_ALL_HEURS").is_none() {
                            if self.params.verbose {
                                eprintln!("MIP: root heuristic {}: skipped (no solution before the restart)", $name);
                            }
                        } else {
                            let t = Instant::now();
                            let found = $e;
                            if !found {
                                self.failed_heurs |= bit;
                            }
                            if self.params.verbose {
                                eprintln!("MIP: root heuristic {}: {} ({:.2}s)", $name, if found { "found" } else { "none" }, t.elapsed().as_secs_f64());
                            }
                        }
                    }};
                }
                heur!("simple rounding", self.simple_rounding(&x));
                if env_str!("ENOMOTO_MIP_NO_ZIROUND").is_none() {
                    heur!("ZI round", self.zi_round(&x));
                }
                if self.incumbent.is_none() && env_str!("ENOMOTO_MIP_NO_TRIVIAL").is_none() {
                    heur!("trivial", self.trivial());
                }
                heur!("randomized rounding", self.randomized_rounding(&x, 3));
                if env_str!("ENOMOTO_MIP_NO_IPM_HEUR").is_none() {
                    heur!("interior rounding", self.interior_rounding(&x));
                }
                if env_str!("ENOMOTO_MIP_NO_ROOT_REDCOST_HEUR").is_none() {
                    heur!("root reduced cost", self.root_reduced_cost());
                }
                if env_str!("ENOMOTO_MIP_NO_LPFACE").is_none() {
                    heur!("LP face", self.lp_face());
                }
                heur!("RENS", self.rens(&x));
                if self.incumbent.is_none() && !self.fractional(&x_root0).is_empty() {
                    heur!("RENS (LP before cuts)", self.rens(&x_root0));
                }
                if self.incumbent.is_none() {
                    heur!("feasibility pump", self.feasibility_pump(root_iters));
                }
                // ダイビングの予算: 最初の根の LP の 2 倍に、カットのループを含めた根全体の反復数の一定割合を足す
                let dive_budget = 2 * root_iters + 1000 + (tunable!("ENOMOTO_T_MIP_ROOT_DIVE_FRAC", 0.0, f64) * self.lp.total_iterations() as f64) as u64;
                if self.incumbent.is_none() {
                    heur!("fractional diving", self.fractional_dive(dive_budget));
                }
                if self.incumbent.is_none() {
                    heur!("vector length diving", self.dive(super::heuristics::DiveKind::VectorLength, dive_budget, f64::INFINITY));
                }
                if self.incumbent.is_none() && env_str!("ENOMOTO_MIP_NO_COEF_DIVE").is_none() {
                    heur!("coefficient diving", self.dive(super::heuristics::DiveKind::Coefficient, dive_budget, f64::INFINITY));
                }
                // LP を使わない最後の手段 (先に使うと質の悪い解で他の暫定解探しを止めてしまう)
                if self.incumbent.is_none() && env_str!("ENOMOTO_MIP_NO_SHIFTPROP").is_none() {
                    heur!("shift-and-propagate", self.shift_and_propagate());
                }
                // SCIP 由来 (locks・clique・vbounds)。暫定解がないか、根の LP との差が 1% 以上なら使う
                let scip_heur_ok = |s: &Self| s.incumbent.as_ref().is_none_or(|(z, _)| *z - root_obj > 0.01 * root_obj.abs().max(1.0));
                if scip_heur_ok(self) && env_str!("ENOMOTO_MIP_NO_LOCKS_HEUR").is_none() {
                    heur!("locks", self.locks_heur());
                }
                if scip_heur_ok(self) && env_str!("ENOMOTO_MIP_NO_CLIQUE_HEUR").is_none() {
                    heur!("clique", self.clique_heur(&x));
                }
                if scip_heur_ok(self) && env_str!("ENOMOTO_MIP_NO_VBOUNDS_HEUR").is_none() {
                    heur!("vbounds (loose)", self.vbounds_heur(true));
                    if scip_heur_ok(self) {
                        heur!("vbounds (tight)", self.vbounds_heur(false));
                    }
                }
                // 並列モード: 別スレッドのサブ MIP の結果を出した順に受け取る
                self.parallel_submips = false;
                self.join_submips(0);
                // 局所探索と Repair (暫定解がなければ)
                if self.incumbent.is_none() && env_str!("ENOMOTO_MIP_NO_LOCAL_SEARCH").is_none() {
                    heur!("local search", self.local_search_heur());
                }
                if self.incumbent.is_none() && env_str!("ENOMOTO_MIP_NO_REPAIR").is_none() {
                    heur!("repair", self.repair(&x));
                }
                // 違反量を最小にする補助 MIP (最後の手段、既定では使わない: 30n20b8・neos-1456979 で違反 0 の点が
                // 見つからず根の時間を 3-5 s 使うだけだった。ENOMOTO_MIP_MINREL=1 で使う)
                if self.incumbent.is_none() && env_str!("ENOMOTO_MIP_MINREL").is_some() {
                    heur!("min relaxation", self.min_relaxation());
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
        // 並列モードは根のヒューリスティクスの間だけ (LP 解が整数で途中を飛ばした場合も戻す)
        self.parallel_submips = false;
        self.join_submips(0);
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
        self.queue.push(OpenNode { changes: Vec::new(), lower_bound: root_obj, estimate: root_obj, depth: 0, basis: Some(Rc::new(self.lp.basis())), basis_epoch: self.row_log.len(), branch: None });
        // 根のノードは LP を解いた状態のままなので、最初の取り出しでは定義域・LP を作り直さない。
        let mut first = true;
        // 潜っている子ノード (定義域に分枝を積んだ状態で次に処理する)。
        let mut plunge: Option<OpenNode> = None;
        let mut plunge_depth = 0usize;
        let mut plunge_start = 0u64;
        loop {
            if self.time_up() {
                return self.finish_limit(MipStatus::TimeLimit, plunge.as_ref());
            }
            if self.nodes >= self.params.node_limit {
                return self.finish_limit(MipStatus::NodeLimit, plunge.as_ref());
            }
            // 潜りが枝刈りで終わったら、潜った道の兄弟ノード (新しいものから) に戻る (HiGHS の backtrackPlunge)。
            // 潜りの打ち切りの条件 (下界が 全体の下界 + q (打ち切り値 - 全体の下界) 以下) を満たすものだけ
            let sibling = if plunge.is_none()
                && !first
                && env_str!("ENOMOTO_MIP_NO_SIBLING_BACKTRACK").is_none()
                && !(self.params.submip && env_str!("ENOMOTO_MIP_NO_SIBLING_BACKTRACK_SUB").is_some())
                // 1 回の潜り (兄弟への戻りを含む) のノード数の上限 (HiGHS: min(1000, ノード数 / 10))
                && self.nodes - plunge_start < (self.nodes / tunable!("ENOMOTO_T_MIP_PLUNGE_NODES_DIV", 10u64, u64)).min(1000)
            {
                let cutoff = self.prune_limit();
                let glb = self.queue.best_lower_bound();
                let limit = if cutoff.is_finite() { glb + tunable!("ENOMOTO_T_MIP_SIBLING_QUOT", 0.25, f64) * (cutoff - glb) } else { f64::INFINITY };
                let mut got = None;
                while let Some((id, gen)) = self.dive_stack.pop() {
                    if let Some(n) = self.queue.take(id, gen, limit.min(cutoff)) {
                        got = Some(n);
                        break;
                    }
                }
                got
            } else {
                None
            };
            let from_sibling = sibling.is_some();
            let node = match plunge.take().or(sibling) {
                Some(n) if !from_sibling => n,
                Some(n) => {
                    // 兄弟ノード: 定義域を根から積み直す (下の共通の処理と同じ)
                    self.sibling_backtracks += 1;
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
                    if !ok {
                        self.nodes += 1;
                        continue;
                    }
                    if let Some(b) = &n.basis {
                        self.restore_node_basis(b, n.basis_epoch);
                    }
                    n
                }
                None => {
                    plunge_depth = 0;
                    plunge_start = self.nodes;
                    self.dive_stack.clear();
                    let lim = self.prune_limit();
                    self.queue.prune(lim);
                    // 下界最小のノードを選ぶ頻度: 暫定解があれば上げる (下界を押し上げる)
                    let bb_every = if self.incumbent.is_some() { tunable!("ENOMOTO_T_MIP_BB_EVERY_INC", 4u64, u64) } else { tunable!("ENOMOTO_T_MIP_BB_EVERY", 10u64, u64) };
                    let Some(n) = self.queue.pop(bb_every) else { break };
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
                                self.restore_node_basis(b, n.basis_epoch);
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
            xp("m_pop");
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
            if !prop_ok {
                self.add_conflict();
            }
            if let Some((j, up, _, _)) = node.branch {
                self.pc.add_inference(j, up, self.dom.stack_len().saturating_sub(stack_before_prop) as f64);
            }
            xp("m_prop");
            // 双対証明による枝刈りと境界の締め付け
            let prop_ok = prop_ok && {
                let ok = self.apply_dual_proofs();
                if !ok {
                    self.proof_prunes += 1;
                    self.dbg_lost("dual proof", dn && self.dbg_contains());
                }
                ok
            };
            xp("m_proof");
            // 完全オービトープの固定 (列を辞書式で減少に並べた解だけを残す)
            let prop_ok = prop_ok && {
                let ok = self.orbitope_propagate(&node.changes);
                if !ok {
                    self.orbitope_prunes += 1;
                }
                ok
            };
            if !prop_ok {
                self.pc.infeasible_leaves += 1;
                self.dbg_lost("node propagation", dn);
                if let Some((j, up, _, _)) = node.branch {
                    self.pc.add_cutoff(j, up);
                }
                continue;
            }
            self.sync_lp();
            xp("m_sync");
            // ノードの LP を解く (強分岐で境界が締まったら解き直す)。
            let mut node_obj;
            let mut resolves = 0;
            let action = loop {
                let it0 = self.lp.total_iterations();
                let iter_limit = (10 * self.avg_node_iters()).max(20_000);
                // 1 回の LP の時間の上限 (平均の 50 倍、最低 0.5 秒): 数値的に悪条件の LP が再分解を繰り返して
                // 残り時間を使い切るのを防ぐ。超えたら全論理変数基底から解き直し、それも超えたら LP なしで分枝する
                let t_lp = Instant::now();
                let cap = Duration::from_secs_f64((50.0 * self.node_lp_secs / self.node_lps.max(1) as f64).max(0.5));
                let capped = |s: &Self, it: u64| {
                    let mut l = s.limits(it);
                    l.deadline = Some(match l.deadline {
                        Some(d) => d.min(Instant::now() + cap),
                        None => Instant::now() + cap,
                    });
                    l
                };
                let mut st = self.lp.solve(&capped(self, iter_limit));
                if st == LpStatus::TimeLimit && !self.time_up() {
                    st = LpStatus::IterationLimit;
                }
                if matches!(st, LpStatus::Error | LpStatus::IterationLimit) {
                    // 全論理変数基底から解き直してみる
                    let b = self.lp.basis();
                    let slack = super::lp::Basis { col: b.col.iter().map(|_| VarStatus::Lower).collect(), row: vec![VarStatus::Basic; b.row.len()] };
                    self.lp.set_basis(&slack);
                    st = self.lp.solve(&capped(self, u64::MAX));
                    if st == LpStatus::TimeLimit && !self.time_up() {
                        st = LpStatus::Error;
                    }
                }
                self.node_lp_secs += t_lp.elapsed().as_secs_f64();
                self.node_iters += self.lp.total_iterations() - it0;
                if resolves == 0 {
                    self.node_iters_first += self.lp.total_iterations() - it0;
                    if std::mem::take(&mut self.restored_now) {
                        self.restore_stats.2 += self.lp.total_iterations() - it0;
                    }
                }
                self.node_lps += 1;
                match st {
                    LpStatus::Optimal => {}
                    LpStatus::Infeasible | LpStatus::ObjectiveBound => {
                        if st == LpStatus::Infeasible {
                            self.pc.infeasible_leaves += 1;
                            self.add_farkas_proof();
                        } else {
                            self.pc.objlim_leaves += 1;
                            self.add_dual_proof();
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
                xp("m_lpstat");
                node_obj = self.lp.objective() + self.p.offset;
                if !self.params.submip && env_str!("ENOMOTO_MIP_NO_CUT_AGING").is_none() {
                    self.age_cuts();
                    // 年齢の上限を超えたカットを外す (行の削除は LP を作り直すので、ある程度まとめて)
                    if self.nodes % tunable!("ENOMOTO_T_MIP_CUT_AGING_EVERY", 20u64, u64) == 0 {
                        self.remove_aged_cuts();
                    }
                }
                if resolves == 0 {
                    if let Some((j, up, parent_val, parent_obj)) = node.branch.filter(|b| b.3.is_finite()) {
                        let x = self.lp.col_value(j);
                        let delta = (x - parent_val).abs().max(if up { parent_val.ceil() - parent_val } else { parent_val - parent_val.floor() });
                        self.pc.add_observation(j, up, delta, node_obj - parent_obj);
                    }
                }
                if node_obj >= self.prune_limit() {
                    self.add_dual_proof();
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
                xp("m_frac");
                // 被約費用による局所的な固定。LP 解が新しい境界から外れたら解き直す。
                if self.incumbent.is_some() && self.local_redcost_fixing(node_obj) {
                    if dn && !self.dbg_contains() {
                        self.dbg_lost("local redcost fixing", true);
                    }
                    xp("r_fix");
                    if !self.dom.propagate(self.p) {
                        self.add_conflict();
                        self.dbg_lost("propagation after local redcost fixing", dn && self.dbg_contains());
                        break None;
                    }
                    xp("r_prop");
                    self.sync_lp();
                    xp("r_sync");
                    let x2 = self.lp.col_values();
                    let moved = (0..self.p.n).any(|j| x2[j] < self.dom.lo[j] - FEASTOL || x2[j] > self.dom.up[j] + FEASTOL);
                    if moved {
                        resolves += 1;
                        continue;
                    }
                }
                xp("m_redcost");
                // ノードのヒューリスティクス (安価な単純丸めは毎回、ランダム丸めは予算内で待ち行列から取り出したノードのみ)
                if resolves == 0 {
                    self.simple_rounding(&x);
                    if env_str!("ENOMOTO_MIP_NO_ZIROUND").is_none() {
                        self.zi_round(&x);
                    }
                    let budget = self.lp.total_iterations() / 20 + 10_000;
                    // 大近傍探索 (サブ MIP) は反復の予算とは別に、経過時間の一定割合までの時間の予算で呼ぶ
                    // (サブ MIP の反復を共通の予算に数えると、1 回で使い切ってしばらく呼べなくなる)
                    // 割合は改善できた割合に連動させる (改善しない問題では 2%、改善するほど最大 10%)
                    // (成功 1 回の報酬は 1-2 なので、報酬の合計の半分を成功回数の目安にする)
                    let lns_calls: f64 = self.alns_count.iter().sum::<u32>() as f64;
                    let succ_rate = (0.5 * self.alns_reward.iter().sum::<f64>() + 1.0) / (lns_calls + 2.0);
                    let frac = tunable!("ENOMOTO_T_MIP_LNS_TIME_MIN", 0.02, f64) + tunable!("ENOMOTO_T_MIP_LNS_TIME_FRAC", 0.08, f64) * succ_rate;
                    // 既定はヒューリスティクス共通の反復の予算の中で 100 ノードおき (時間の予算を与えると、改善しない
                    // 問題で解ける問題を遅くした: 40 問で 15 -> 14 問)。ENOMOTO_MIP_LNS_TIME で時間の予算にする
                    let lns_time = env_str!("ENOMOTO_MIP_LNS_TIME").is_some();
                    let lns_ok = if lns_time { self.lns_secs < frac * self.start.elapsed().as_secs_f64() } else { self.heur_iters < budget };
                    let lns_freq = if lns_time { tunable!("ENOMOTO_T_MIP_LNS_FREQ", 50u64, u64) } else { 100 };
                    if node.depth > 0 && plunge_depth == 0 && self.incumbent.is_some() && self.nodes >= self.last_rins + lns_freq && lns_ok {
                        self.last_rins = self.nodes;
                        let t_lns = Instant::now();
                        // 改善ヒューリスティクス: ALNS で選ぶ (ENOMOTO_MIP_NO_ALNS なら RINS → Local Branching →
                        // Proximity Search を順に回す)
                        let turn = if env_str!("ENOMOTO_MIP_ONLY_RINS").is_some() {
                            0
                        } else if env_str!("ENOMOTO_MIP_NO_ALNS").is_none() {
                            3
                        } else {
                            self.improve_turn % 3
                        };
                        self.improve_turn += 1;
                        match turn {
                            0 => {
                                self.rins(&x);
                            }
                            1 => {
                                self.local_branching();
                            }
                            2 => {
                                self.proximity_search();
                            }
                            _ => {
                                self.alns(&x);
                            }
                        }
                        self.lns_secs += t_lns.elapsed().as_secs_f64();
                    } else if node.depth > 0 && plunge_depth == 0 && self.heur_iters < budget {
                        self.randomized_rounding(&x, 1);
                    }
                    xp("m_heur");
                    // ダイビング (SCIP の fracdiving / veclendiving: 深さ 10 ごと、ずらし 3 / 7)
                    if node.depth % 10 == 3 || node.depth % 10 == 7 {
                        let quota = (0.05 * (self.dive_succ + 1) as f64 / (self.dive_calls + 1) as f64 * self.node_iters as f64) as u64 + 1000;
                        if self.dive_iters < quota && env_str!("ENOMOTO_MIP_NO_NODE_DIVE").is_none() {
                            // 種類を順に回す (SCIP の各ダイビングに相当。誘導は暫定解があるときだけ)
                            use super::heuristics::DiveKind as K;
                            let kinds: &[K] = if env_str!("ENOMOTO_MIP_DIVE_OLD_KINDS").is_some() {
                                &[K::Fractional, K::VectorLength]
                            } else if env_str!("ENOMOTO_MIP_NO_SCIP_DIVES").is_some() {
                                if self.incumbent.is_some() {
                                    &[K::Fractional, K::VectorLength, K::Coefficient, K::Pseudocost, K::Guided]
                                } else {
                                    &[K::Fractional, K::VectorLength, K::Coefficient, K::Pseudocost]
                                }
                            } else if self.incumbent.is_some() {
                                &[K::Fractional, K::VectorLength, K::Coefficient, K::Pseudocost, K::Guided, K::Farkas, K::Conflict]
                            } else {
                                &[K::Fractional, K::VectorLength, K::Coefficient, K::Pseudocost, K::Farkas, K::Conflict]
                            };
                            let kind = kinds[self.dive_calls as usize % kinds.len()];
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
                xp("m_dive");
                // ノードでの切除平面 (待ち行列から取り出したノードで 1 回)。カットを足すたびに LP を作り直すので
                // 今は遅くなる問題が多く、既定では行わない (ENOMOTO_MIP_NODE_CUTS=1 で有効)。
                // カットプールからのカット (待ち行列から取り出したノードで最大 2 回、各 10 本まで)
                if resolves < 2 && node.depth > 0 && plunge_depth == 0 && env_str!("ENOMOTO_MIP_NO_POOL_CUTS").is_none() && self.pool_cut_round(&x, 10) {
                    resolves += 1;
                    continue;
                }
                if resolves == 0 && node.depth > 0 && plunge_depth == 0 && self.node_cuts_due(node.depth) && self.node_cut_round(&x) {
                    resolves += 1;
                    continue;
                }
                xp("m_pool");
                let before_sb = dn && self.dbg_contains();
                let sel = self.select_branch(&frac, node_obj, node.depth);
                if before_sb && !self.dbg_contains() {
                    self.dbg_lost("strong branching tightened bounds", true);
                }
                if before_sb && matches!(sel, BranchAction::Prune) {
                    self.dbg_lost("strong branching pruned the node", true);
                }
                xp("m_sb");
                match sel {
                    BranchAction::Resolve => {
                        resolves += 1;
                        if !self.dom.propagate(self.p) {
                            self.add_conflict();
                            break None;
                        }
                        self.sync_lp();
                        continue;
                    }
                    BranchAction::Prune => break None,
                    BranchAction::Branch { col, value } => {
                        let (col, value) = self.orbitope_branching_column(col, value);
                        break Some(Some((col, value, node_obj, frac)));
                    }
                }
            };
            xp("m_endlp");
            let Some(branch) = action else { continue };
            let Some((col, value, node_obj, frac)) = branch else {
                // LP なしの分枝 (branch_without_lp が子ノードを積んだ)
                continue;
            };
            // 子ノードを作る
            // LP が解けずに分けたノード (node_obj = -inf) でも、親の下界は子でも成り立つ
            let lp_obj = node_obj;
            let node_obj = node_obj.max(node.lower_bound);
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
            let basis_epoch = self.row_log.len();
            // 強分岐で子の LP が最適まで解けていれば、その値を子の下界にする
            let sb_bounds = self.last_sb.filter(|&(j, _, _)| j == col && env_str!("ENOMOTO_MIP_NO_SB_CHILD_BOUND").is_none());
            let mk = |c: BoundChange| {
                let mut ch = node.changes.clone();
                ch.push(c);
                let child_lb = match sb_bounds {
                    Some((_, dn, upb)) => node_obj.max(if c.upper { dn } else { upb }),
                    None => node_obj,
                };
                OpenNode {
                    changes: ch,
                    lower_bound: child_lb,
                    estimate,
                    depth: node.depth + 1,
                    basis: Some(basis.clone()),
                    basis_epoch,
                    branch: Some((col, !c.upper, value, lp_obj)),
                }
            };
            let sib = self.queue.push(mk(second_c));
            self.dive_stack.push(sib);
            let child = mk(first_c);
            plunge_depth += 1;
            // 潜りの打ち切り (SCIP の maxplungequot): 子の下界が 全体の下界 + q (打ち切り値 - 全体の下界) を超えたら
            // 潜らずに待ち行列から選び直す
            let plunge_ok = {
                let cutoff = self.prune_limit();
                if cutoff.is_finite() && env_str!("ENOMOTO_MIP_NO_PLUNGE_ABORT").is_none() {
                    let glb = self.queue.best_lower_bound().min(node_obj);
                    child.lower_bound <= glb + tunable!("ENOMOTO_T_MIP_PLUNGE_QUOT", 0.25, f64) * (cutoff - glb)
                } else {
                    true
                }
            };
            if plunge_depth <= 200 && plunge_ok {
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
            xp("m_child");
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
        xp("r_pre");
        let b = self.lp.basis();
        xp("r_basis");
        let d = self.lp.reduced_costs();
        xp("r_d");
        let cont_frac = tunable!("ENOMOTO_T_MIP_REDCOST_CONT_FRAC", 0.1, f64);
        let mut changed = false;
        for j in 0..self.p.n {
            if self.dom.is_fixed(j) {
                continue;
            }
            // 連続列はわずかな締め付けでも活動量の更新と伝播が走るので、幅を一定割合以上縮めるときだけ締める
            let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
            let min_cut = if self.p.is_int[j] { 0.0 } else { cont_frac * (up - lo) };
            match b.col[j] {
                VarStatus::Lower if d[j] > 1e-7 => {
                    let v = lo + gap / d[j];
                    if !(up - v >= min_cut) {
                        continue;
                    }
                    changed |= self.dom.tighten_upper(self.p, j, v);
                }
                VarStatus::Upper if d[j] < -1e-7 => {
                    let v = up - gap / (-d[j]);
                    if !(v - lo >= min_cut) {
                        continue;
                    }
                    changed |= self.dom.tighten_lower(self.p, j, v);
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
        let t0 = Instant::now();
        self.last_sb = None;
        let r = self.select_branch_inner(frac, node_obj, depth);
        self.sb_secs += t0.elapsed().as_secs_f64();
        r
    }

    fn select_branch_inner(&mut self, frac: &[(usize, f64)], node_obj: f64, depth: usize) -> BranchAction {
        if env_str!("ENOMOTO_MIP_SB_OLD").is_some() {
            return self.select_branch_old(frac, node_obj);
        }
        // SCIP の relpscost に倣った強分岐 (branch_relpscost.c `branchExecRelpscost`)。
        let hybrid = env_str!("ENOMOTO_MIP_NO_HYBRID_SCORE").is_none();
        let score_of = |s: &Self, j: usize, v: f64| if hybrid { s.pc.hybrid_score(j, v - v.floor()) } else { s.pc.score(j, v - v.floor()) };
        let mut cands: Vec<(usize, f64, f64)> = frac.iter().map(|&(j, v)| (j, v, score_of(self, j, v))).collect();
        cands.sort_by(|a, b| b.2.total_cmp(&a.2));
        // 予算: ノード LP の反復数 (強分岐を除く) の 0.125 倍 + 25000
        let quot = tunable!("ENOMOTO_T_MIP_SB_QUOT", 0.125, f64) * self.node_iters as f64;
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
        let sb_iter_limit = ((tunable!("ENOMOTO_T_MIP_SB_ITERMULT", 2.0, f64) * self.avg_node_iters() as f64 * (1.0 + 20.0 / self.nodes.max(1) as f64)) as u64).clamp(10, 500);
        let unreliable: usize = cands.iter().filter(|&&(j, _, _)| (self.pc.min_observations(j) as f64) < reliable_thr).count();
        let lookahead = (tunable!("ENOMOTO_T_MIP_SB_LOOKAHEAD", 9.0, f64) * (1.0 + unreliable as f64 / cands.len().max(1) as f64)) as usize;
        let max_tried = tunable!("ENOMOTO_T_MIP_SB_MAXCAND", 100usize, usize);
        let mut best: Option<(usize, f64, f64)> = None; // (列, 値, スコア)
        self.last_sb = None;
        let mut best_sb: Option<(usize, f64, f64)> = None; // best が強分岐で評価した列なら (列, 下の子の値, 上の子の値)
        let mut no_improve = 0.0f64;
        let mut tried = 0usize;
        let mut bound_changes = 0usize;
        for &(j, v, pscore) in &cands {
            let is_reliable = (self.pc.min_observations(j) as f64) >= reliable_thr;
            if is_reliable || !allow_sb || tried >= max_tried || no_improve >= lookahead as f64 || self.time_up() || (self.sb_iters as f64) > maxsb {
                if best.is_none_or(|(_, _, s)| pscore > s) {
                    best = Some((j, v, pscore));
                    best_sb = None;
                }
                continue;
            }
            tried += 1;
            // 強分岐
            let saved = self.lp.save_state();
            let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
            let mut gains = [0.0f64; 2];
            let mut child_obj = [f64::NEG_INFINITY; 2];
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
                self.sb_lps += 1;
                match st {
                    LpStatus::Optimal => {
                        let o = self.lp.objective() + self.p.offset;
                        gains[side] = (o - node_obj).max(0.0);
                        child_obj[side] = o;
                        let delta = if is_up { v.ceil() - v } else { v - v.floor() };
                        self.pc.add_observation(j, is_up, delta, gains[side]);
                        if o >= self.prune_limit() {
                            cut[side] = true;
                            self.sb_child_proof(j, is_up, v, false);
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
                        self.sb_child_proof(j, is_up, v, st == LpStatus::Infeasible);
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
                best_sb = Some((j, child_obj[0], child_obj[1]));
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
            Some((col, value, _)) => {
                self.last_sb = best_sb.filter(|&(j, _, _)| j == col);
                BranchAction::Branch { col, value }
            }
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
                self.sb_lps += 1;
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

    /// 双対証明を作ってプールに入れる (HiGHS の dual proof、衝突分析の LP 版)。LP は最適で、目的値が
    /// 打ち切り値以上のときに呼ぶ。双対値 y で行を足し、目的関数の上限 `c x <= U` と組み合わせると
    /// `(c - y^T A) x <= U - sum_i y_i b_i` (`b_i` は y_i > 0 なら行の下限、y_i < 0 なら上限) が全ての
    /// 改善解で成り立つ (行は大域的に成り立つ元の行かカット)。今のノードの境界ではこれが破れている。
    /// 係数の小さい列と大域的に固定された列は大域的な境界で右辺に移す。密すぎるものは捨てる。
    pub(super) fn add_dual_proof(&mut self) {
        if self.params.submip || env_str!("ENOMOTO_MIP_NO_DUAL_PROOF").is_some() {
            return;
        }
        let y = self.lp.row_duals();
        self.add_proof_from(&y, true);
    }

    /// 完全オービトープの固定と伝播を、固定が出なくなるまで (最大 10 回) 繰り返す。矛盾したら偽。
    /// 待ち行列のノードの基底を LP に置く (保存した後の行の追加・削除を当てはめる)。
    fn restore_node_basis(&mut self, b: &super::lp::Basis, epoch: usize) {
        let mr = self.lp.num_rows();
        if epoch == self.row_log.len() && b.row.len() == mr {
            self.restore_stats.0 += 1;
            self.restored_now = true;
            self.lp.set_basis(b);
            return;
        }
        let mut b2 = b.clone();
        if epoch <= self.row_log.len() && env_str!("ENOMOTO_MIP_NO_ROW_LOG").is_none() {
            for e in &self.row_log[epoch..] {
                match e {
                    RowEdit::Add(k) => b2.row.extend(std::iter::repeat_n(VarStatus::Basic, *k)),
                    RowEdit::Delete(mask) => {
                        let mut i = 0;
                        b2.row.retain(|_| {
                            let keep = !mask.get(i).copied().unwrap_or(false);
                            i += 1;
                            keep
                        });
                    }
                }
            }
        }
        b2.row.resize(mr, VarStatus::Basic);
        let nb = b2.col.iter().chain(b2.row.iter()).filter(|&&s| s == VarStatus::Basic).count();
        self.restore_stats.0 += 1;
        if nb != mr {
            self.restore_stats.1 += 1;
            // 保存した基底で非基底だった (効いていた) カットの行が削除されていると基底変数が多すぎる。
            // set_basis の修復 (構造変数を後ろから外す) は双対実行可能性を壊し、数百反復かかる
            // (neos-911970: 復元の 9 割)。今の LP の基底 (直前のノードの最適基底、双対実行可能) のまま解く
            if env_str!("ENOMOTO_MIP_RESTORE_MISMATCHED_BASIS").is_none() {
                return;
            }
        }
        self.restored_now = true;
        self.lp.set_basis(&b2);
    }

    /// 分枝する列がパッキング・オービトープの行にあれば、その行で最も左の固定されていない列で分枝する
    /// (HiGHS の `getBranchingColumn`。動的な orbitopal fixing の行の順が早く決まり、固定がよく効く)。
    /// 0-1 列なので分枝の値は 0.5 (LP 値が整数でも子の境界が変わるように)。
    fn orbitope_branching_column(&mut self, col: usize, value: f64) -> (usize, f64) {
        if self.orbitopes.is_empty() || env_str!("ENOMOTO_MIP_ORBITOPE_STATIC").is_some() || env_str!("ENOMOTO_MIP_NO_ORBITOPE_BRANCH").is_some() {
            return (col, value);
        }
        if self.orb_col.is_empty() {
            for (k, o) in self.orbitopes.iter().enumerate() {
                for (i, line) in o.vars.iter().enumerate() {
                    for &j in line {
                        self.orb_col.insert(j, (k, i));
                    }
                }
            }
        }
        let Some(&(k, i)) = self.orb_col.get(&col) else { return (col, value) };
        let o = &self.orbitopes[k];
        if !o.row_packing[i] {
            return (col, value);
        }
        for &j in &o.vars[i] {
            if j == col {
                break;
            }
            if self.dom.lo[j] < self.dom.up[j] {
                let v = self.lp.col_value(j);
                let v = if (v - v.round()).abs() <= 1e-6 { 0.5 } else { v };
                return (j, v);
            }
        }
        (col, value)
    }

    /// `changes` はこのノードまでの分枝 (根から順)。動的な固定 (既定) では、分枝した列を含むオービトープの行を
    /// 分枝した順に並べた行列で固定する (HiGHS・Bendotti らの dynamic orbitopal fixing。順は道ごとに決まるので道の上で
    /// 一貫している)。`ENOMOTO_MIP_ORBITOPE_STATIC` なら全ての行を固定の順で使う。
    fn orbitope_propagate(&mut self, changes: &[BoundChange]) -> bool {
        if self.orbitopes.is_empty() {
            return true;
        }
        let orbs = self.orbitopes.clone();
        let dynamic = env_str!("ENOMOTO_MIP_ORBITOPE_STATIC").is_none();
        if dynamic && self.orb_col.is_empty() {
            for (k, o) in orbs.iter().enumerate() {
                for (i, line) in o.vars.iter().enumerate() {
                    for &j in line {
                        self.orb_col.insert(j, (k, i));
                    }
                }
            }
        }
        // オービトープごとの分枝した行 (分枝した順)
        let subs: Vec<super::symmetry::Orbitope> = if dynamic {
            let mut rows: Vec<Vec<usize>> = vec![Vec::new(); orbs.len()];
            for c in changes {
                if let Some(&(k, i)) = self.orb_col.get(&c.col) {
                    if !rows[k].contains(&i) {
                        rows[k].push(i);
                    }
                }
            }
            rows.iter().enumerate().filter(|(_, r)| !r.is_empty()).map(|(k, r)| orbs[k].sub_rows(r)).collect()
        } else {
            orbs.iter().cloned().collect()
        };
        if subs.is_empty() {
            return true;
        }
        for _ in 0..10 {
            let mut fixes: Vec<(usize, f64)> = Vec::new();
            for o in subs.iter() {
                match super::symmetry::orbitopal_fixing(o, &self.dom.lo, &self.dom.up) {
                    None => return false,
                    Some(f) => fixes.extend(f),
                }
            }
            if fixes.is_empty() {
                return true;
            }
            self.orbitope_fixings += fixes.len() as u64;
            for (j, v) in fixes {
                self.dom.tighten_lower(self.p, j, v);
                self.dom.tighten_upper(self.p, j, v);
            }
            if self.dom.infeasible || !self.dom.propagate(self.p) {
                return false;
            }
        }
        true
    }

    /// 強分岐の子 (列 `j` を `v` の上/下に分けた側) が打ち切られたときの証明 (`infeasible` なら Farkas、そうでなければ
    /// 双対証明)。強分岐は子の境界を LP にだけ置くので、証明が今の定義域で破れているかを正しく調べ、衝突解析がその
    /// 境界を決定として使えるよう、その間だけ定義域にも置く (伝播はしない)。
    fn sb_child_proof(&mut self, j: usize, is_up: bool, v: f64, infeasible: bool) {
        if env_str!("ENOMOTO_MIP_SB_PROOF_OLD").is_some() {
            if !infeasible {
                self.add_dual_proof();
            }
            return;
        }
        let pos = self.dom.stack_len();
        if is_up {
            self.dom.tighten_lower(self.p, j, v.ceil());
        } else {
            self.dom.tighten_upper(self.p, j, v.floor());
        }
        if !self.dom.infeasible {
            if infeasible {
                self.add_farkas_proof();
            } else {
                self.add_dual_proof();
            }
        }
        self.dom.backtrack_to(self.p, pos);
    }

    /// LP が実行不能だったときの双対射線 `y` から実行不能の証明 (Farkas) を作ってプールに入れる。
    /// 行を `y` で足した `y^T A x >= sum_i y_i b_i` (`b_i` は y_i > 0 なら行の下限、y_i < 0 なら上限) はどの `y` でも
    /// 成り立つので、射線の数値誤差は正しさに影響しない (今のノードで破れていなければ捨てる)。符号は両方試す。
    pub(super) fn add_farkas_proof(&mut self) {
        if self.params.submip || env_str!("ENOMOTO_MIP_NO_FARKAS").is_some() {
            return;
        }
        let Some(ray) = self.lp.farkas_ray() else { return };
        if !self.add_proof_from(&ray, false) {
            let neg: Vec<f64> = ray.iter().map(|v| -v).collect();
            self.add_proof_from(&neg, false);
        }
    }

    /// 行の重み `y` から証明 `d x <= rhs` を作り、今のノードで破れていればプールに入れる (入れたら真)。
    /// `obj` なら双対証明 (`d = c - y^T A`、`rhs = U - konst`)、そうでなければ実行不能の証明 (`d = -y^T A`、
    /// `rhs = -konst`)。
    fn add_proof_from(&mut self, y: &[f64], obj: bool) -> bool {
        let p = self.p;
        let mut d = if obj { p.cost.clone() } else { vec![0.0; p.n] };
        let mut konst = 0.0;
        for (i, &yi) in y.iter().enumerate().take(self.lp.num_rows()) {
            if yi.abs() <= 1e-12 {
                continue;
            }
            let (lo, up) = self.lp.row_bounds(i);
            let b = if yi > 0.0 { lo } else { up };
            if !b.is_finite() {
                continue; // その行は使わない (y_i = 0 とみなす。d もそれに合わせて作る)
            }
            konst += yi * b;
            for (j, a) in self.lp.row(i) {
                d[j] -= yi * a;
            }
        }
        let dmax = d.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        if dmax <= 1e-9 {
            return false;
        }
        let mut coefs: Vec<(usize, f64)> = Vec::new();
        for (j, &dj) in d.iter().enumerate() {
            if dj == 0.0 {
                continue;
            }
            let (gl, gu) = (self.dom.global_lo[j], self.dom.global_up[j]);
            let fixed = gl == gu;
            // ノードでの境界が大域的な境界と同じ列は、どのノードでも最小値が同じなので右辺に移しても
            // このノードで破れたまま (分枝・伝播で締まった列だけが残る短い証明になる)
            let unchanged = self.dom.lo[j] == gl && self.dom.up[j] == gu && env_str!("ENOMOTO_MIP_PROOF_NO_REDUCE").is_none();
            if fixed || unchanged || dj.abs() <= 1e-9 * dmax {
                // d_j x_j >= d_j * (下の側の大域的な境界) で右辺に移す: sum_{他} <= U - konst - d_j x_j <= U - (konst + min)
                let m = if dj > 0.0 { dj * gl } else { dj * gu };
                if !m.is_finite() {
                    if fixed {
                        continue;
                    }
                    coefs.push((j, dj));
                    continue;
                }
                konst += m;
                continue;
            }
            coefs.push((j, dj));
        }
        let dbg = env_str!("ENOMOTO_MIP_DEBUG_PROOF").is_some();
        if coefs.is_empty() || coefs.len() > tunable!("ENOMOTO_T_MIP_PROOF_DENSITY", 0.5, f64).mul_add(p.n as f64, 20.0) as usize {
            if dbg {
                eprintln!("PROOF reject dense {} of {}", coefs.len(), p.n);
            }
            return false;
        }
        // 今のノードで破れていることを確かめる (数値誤差で破れていない証明は使わない)
        let rhs = if obj { self.prune_limit() - p.offset - konst } else { -konst };
        let minact: f64 = coefs.iter().map(|&(j, a)| if a > 0.0 { a * self.dom.lo[j] } else { a * self.dom.up[j] }).sum();
        if !minact.is_finite() || minact <= rhs + 1e-6 * (1.0 + rhs.abs()) {
            if dbg {
                eprintln!("PROOF reject not violated: minact {minact} rhs {rhs}");
            }
            return false;
        }
        // 診断用: デバッグ解を切っていないか
        DEBUG_SOL.with(|dd| {
            if let Some(x) = dd.borrow().as_ref() {
                if !obj || self.p.objective(x) < self.prune_limit() - 1e-6 {
                    let act: f64 = coefs.iter().map(|&(j, a)| a * x[j]).sum();
                    if act > rhs + 1e-6 * (1.0 + rhs.abs()) {
                        eprintln!("MIP_DEBUG_SOL: dual proof cuts off the debug solution ({act} > {rhs})");
                    }
                }
            }
        });
        // 証明を作った境界の変更を分枝の決定まで辿った短い衝突も作る (証明そのものは長いことが多い)
        self.add_proof_conflict(&coefs);
        if obj {
            self.push_pool_row(coefs, konst, true);
        } else {
            self.farkas_added += 1;
            self.push_pool_row(coefs, -konst, false);
        }
        true
    }

    /// 双対証明・衝突制約のプールに行 `sum coefs x <= rhs` を加える (`obj` なら `rhs = U - konst`、そうでなければ
    /// `rhs = konst`)。上限を超えたら古いものから捨てる。
    pub(super) fn push_pool_row(&mut self, coefs: Vec<(usize, f64)>, konst: f64, obj: bool) {
        let p = self.p;
        let nnz_cap = 10 * p.rows.iter().map(|r| r.len()).sum::<usize>() + 100_000;
        let max_rows = tunable!("ENOMOTO_T_MIP_PROOF_POOL", 2000usize, usize);
        while !self.dual_proofs.is_empty() && (self.dual_proofs.len() >= max_rows || self.dual_proof_nnz + coefs.len() > nnz_cap) {
            let (c, _, _) = self.dual_proofs.pop_front().unwrap();
            self.dual_proof_nnz -= c.len();
            self.proof_base.pop_front();
            self.proof_first_id += 1;
            self.proof_dirty = true;
        }
        self.dual_proof_nnz += coefs.len();
        let id = self.proof_first_id + self.dual_proofs.len() as u64;
        if self.proof_col_index.len() != p.n {
            self.proof_col_index = vec![Vec::new(); p.n];
        }
        for &(j, a) in &coefs {
            self.proof_col_index[j].push((id, a));
        }
        let base = match &self.proof_snap {
            Some((lo, up)) => proof_min_activity(&coefs, lo, up),
            None => (0.0, 0, f64::INFINITY),
        };
        self.proof_base.push_back(base);
        self.proof_dirty = true;
        self.dual_proofs.push_back((coefs, konst, obj));
        let k = self.dual_proofs.len() - 1;
        if self.proof_snap.is_some() && self.proof_root_tight(base, self.proof_rhs(k)) {
            self.proof_always.push(id);
        }
    }

    /// 直前の伝播の矛盾から衝突制約を作ってプールに入れる (定義域を巻き戻す前に呼ぶ)。衝突が 2 値列の固定だけで
    /// できていれば `sum_{x_j=1 の固定} x_j - sum_{x_j=0 の固定} x_j <= |{x_j=1}| - 1` (どれか 1 つは逆の値)。
    pub(super) fn add_conflict(&mut self) {
        if self.params.submip && env_str!("ENOMOTO_MIP_SUBMIP_CONFLICTS").is_none() || env_str!("ENOMOTO_MIP_NO_CONFLICTS").is_some() {
            return;
        }
        let p = self.p;
        let max_len = self.conflict_max_len();
        let Some(lits) = self.dom.analyze_conflict(p, max_len) else { return };
        if !self.add_conflict_lits(&lits) && env_str!("ENOMOTO_MIP_NO_DECISION_CONFLICTS").is_none() {
            // 0-1 列以外の境界を含む: 決定だけの組にして試す
            if let Some(lits) = self.dom.analyze_conflict_decisions(p, max_len) {
                self.add_conflict_lits(&lits);
            }
        }
    }

    /// 衝突解析の上限の長さ。
    fn conflict_max_len(&self) -> usize {
        // 長い衝突は弱いわりに評価が重い (eil33-2: 上限 460 で 678 本作ると 1 ノードの処理が重くなり解けなくなった)
        (tunable!("ENOMOTO_T_MIP_CONFLICT_LEN", 0.1, f64).mul_add(self.p.n as f64, 10.0) as usize).min(tunable!("ENOMOTO_T_MIP_CONFLICT_MAXLEN", 50usize, usize))
    }

    /// 破れている証明 (`coefs`、今のノードの境界で最小活動量が右辺を超える) から衝突を作ってプールに入れる。
    fn add_proof_conflict(&mut self, coefs: &[(usize, f64)]) {
        if env_str!("ENOMOTO_MIP_NO_PROOF_CONFLICTS").is_some() {
            return;
        }
        let max_len = self.conflict_max_len();
        let mut ok = false;
        if let Some(lits) = self.dom.analyze_proof_conflict(self.p, coefs, max_len, false) {
            ok = self.add_conflict_lits(&lits);
        }
        if !ok && env_str!("ENOMOTO_MIP_NO_DECISION_CONFLICTS").is_none() {
            if let Some(lits) = self.dom.analyze_proof_conflict(self.p, coefs, max_len, true) {
                ok = self.add_conflict_lits(&lits);
            }
        }
        if ok {
            self.proof_conflicts += 1;
        }
        if env_str!("ENOMOTO_MIP_DEBUG_PROOF").is_some() {
            let p = self.p;
            let r = self.dom.analyze_proof_conflict(p, coefs, usize::MAX, true);
            match r {
                None => eprintln!("PCONF none (proof len {})", coefs.len()),
                Some(l) => {
                    let bin = l.iter().filter(|&&(j, _, _)| p.is_int[j] && self.dom.global_lo[j] == 0.0 && self.dom.global_up[j] == 1.0).count();
                    let int = l.iter().filter(|&&(j, _, _)| p.is_int[j]).count();
                    eprintln!("PCONF ok {ok} len {} bin {bin} genint {} cont {} (proof len {})", l.len(), int - bin, l.len() - int, coefs.len());
                }
            }
        }
    }

    /// 衝突 (同時には成り立たない境界の組) が 0-1 列だけなら、`sum_{x_j >= 1} x_j - sum_{x_j <= 0} x_j <= |{x_j >= 1}| - 1`
    /// の行にしてプールに入れる (入れたら真)。
    fn add_conflict_lits(&mut self, lits: &[(usize, bool, f64)]) -> bool {
        let p = self.p;
        // 行にできなくても、衝突に現れた列は分枝の衝突スコアに数える
        self.pc.add_conflict(lits);
        let mut coefs: Vec<(usize, f64)> = Vec::with_capacity(lits.len());
        // 各リテラルの「真の度合い」s (偽なら <= 0、真なら > 0、常に <= 1) の和 <= k - 1 (k はリテラルの数) にする。
        // 0-1 列は s = x (x >= 1) / 1 - x (x <= 0)。一般整数列 (大域的な境界 [L, U]) は x >= v なら
        // s = (x - v + 1) / (U - v + 1)、x <= v なら s = (v + 1 - x) / (v + 1 - L)。真のとき s は 1 未満になりうるので、
        // 全部真の点を切るには一般整数のリテラルは 1 つまで (2 つあると、1 つ偽でも和が k - 1 を超えうる)。
        let mut ones = lits.len() as f64 - 1.0; // 右辺 (定数項を移していく)
        let mut general = 0usize;
        for &(j, upper, v) in lits {
            if !p.is_int[j] {
                return false;
            }
            let (gl, gu) = (self.dom.global_lo[j], self.dom.global_up[j]);
            if gl == 0.0 && gu == 1.0 {
                if !upper && v == 1.0 {
                    coefs.push((j, 1.0));
                } else if upper && v == 0.0 {
                    // s = 1 - x
                    coefs.push((j, -1.0));
                    ones -= 1.0;
                } else {
                    return false;
                }
                continue;
            }
            general += 1;
            if general > 1 || env_str!("ENOMOTO_MIP_NO_GENINT_CONFLICTS").is_some() {
                return false;
            }
            if !upper {
                // x >= v (v > L)
                if !gu.is_finite() || v <= gl || v > gu {
                    return false;
                }
                let w = gu - v + 1.0;
                coefs.push((j, 1.0 / w));
                ones += (v - 1.0) / w;
            } else {
                // x <= v (v < U)
                if !gl.is_finite() || v >= gu || v < gl {
                    return false;
                }
                let w = v + 1.0 - gl;
                coefs.push((j, -1.0 / w));
                ones -= (v + 1.0) / w;
            }
        }
        let ones = ones + 1.0; // 下で `ones - 1.0` を右辺にする
        coefs.sort_by_key(|&(j, _)| j);
        coefs.dedup_by_key(|&mut (j, _)| j);
        if coefs.len() != lits.len() {
            return false; // 同じ列の両側 (それ自体で矛盾) は使わない
        }
        // 診断用: デバッグ解を切っていないか
        DEBUG_SOL.with(|dd| {
            if let Some(x) = dd.borrow().as_ref() {
                let act: f64 = coefs.iter().map(|&(j, a)| a * x[j]).sum();
                if act > ones - 1.0 + 1e-6 {
                    eprintln!("MIP_DEBUG_SOL: conflict cuts off the debug solution ({act} > {})", ones - 1.0);
                }
            }
        });
        self.conflicts_added += 1;
        self.push_pool_row(coefs, ones - 1.0, false);
        true
    }

    /// プールの `k` 番目の行の右辺: 双対証明 (`obj`) は `U - konst` (`U` は打ち切り値、暫定解がなければ無限)、
    /// 衝突制約は `konst`。
    fn proof_rhs(&self, k: usize) -> f64 {
        let (_, konst, obj) = &self.dual_proofs[k];
        if *obj {
            self.prune_limit() - self.p.offset - konst
        } else {
            *konst
        }
    }

    /// 根の状態の境界 (基準) で、証明が破れているか締め付けが起こりうるか。
    fn proof_root_tight(&self, base: (f64, u32, f64), rhs: f64) -> bool {
        let (m, ninf, cap) = base;
        ninf == 0 && rhs.is_finite() && rhs - m < cap
    }

    /// 双対証明と衝突制約で今の定義域を調べる: どれかが破れていれば偽 (枝刈り)。そうでなければ整数列の
    /// 境界を締め、締めたら伝播し直す (その結果矛盾すれば偽)。
    pub(super) fn apply_dual_proofs(&mut self) -> bool {
        if self.dual_proofs.is_empty() {
            return true;
        }
        let p = self.p;
        let mut tightened = false;
        // 基準 (根の状態の境界での最小活動量) を取り直す: 最初と、ノード 200 個ごと (根の境界は締まるだけなので
        // 古い基準でも見積もりは真の最小活動量以下で正しい。取り直すと見積もりが強くなる)
        if self.proof_snap.is_none() || self.nodes >= self.proof_snap_nodes + 200 || self.prune_limit() < self.proof_snap_limit || env_str!("ENOMOTO_MIP_PROOF_FULL_SCAN").is_some() {
            let (lo, up) = self.dom.root_bounds();
            self.proof_base = self.dual_proofs.iter().map(|(c, _, _)| proof_min_activity(c, &lo, &up)).collect();
            self.proof_col_index = vec![Vec::new(); p.n];
            for (k, (c, _, _)) in self.dual_proofs.iter().enumerate() {
                let id = self.proof_first_id + k as u64;
                for &(j, a) in c {
                    self.proof_col_index[j].push((id, a));
                }
            }
            self.proof_snap = Some((lo, up));
            self.proof_snap_nodes = self.nodes;
            self.proof_snap_limit = self.prune_limit();
            self.proof_dirty = true;
            self.proof_always.clear();
            for k in 0..self.dual_proofs.len() {
                if self.proof_root_tight(self.proof_base[k], self.proof_rhs(k)) {
                    self.proof_always.push(self.proof_first_id + k as u64);
                }
            }
        }
        xp("p_refresh");
        // 根から変えた列に関わる証明だけを調べる: 最小活動量 = 基準 + (変えた列の寄与の差)。他の証明の最小活動量は
        // 根の状態の境界でのもの (基準以上) で、根で破れない限りこのノードでも破れない
        let full = env_str!("ENOMOTO_MIP_PROOF_FULL_SCAN").is_some();
        let npool = self.dual_proofs.len();
        // 差分は各列の境界 `proof_val_*` までを反映している。前回から境界が変わった列だけを足す (ノードを移っても
        // 戻した列・積み直した列だけで済む)。証明や基準が変わったら作り直す
        let cols: Vec<usize> = if self.proof_dirty || self.proof_delta.len() != npool {
            self.proof_delta.clear();
            self.proof_delta.resize(npool, (0.0, 0));
            for &k in &self.proof_touched {
                if k < self.proof_in_touched.len() {
                    self.proof_in_touched[k] = false;
                }
            }
            self.proof_touched.clear();
            self.proof_in_touched.clear();
            self.proof_in_touched.resize(npool, false);
            self.proof_dirty = false;
            let (slo, sup) = self.proof_snap.as_ref().unwrap();
            self.proof_val_lo.clone_from(slo);
            self.proof_val_up.clone_from(sup);
            let _ = self.dom.take_changed_for_proofs();
            (0..p.n).filter(|&j| self.dom.lo[j] != slo[j] || self.dom.up[j] != sup[j]).collect()
        } else {
            self.dom.take_changed_for_proofs()
        };
        if full {
            self.proof_touched.clear();
            self.proof_touched.extend(0..npool);
        } else {
            self.proof_absorb(&cols);
        }
        if !full {
            let first = self.proof_first_id;
            self.proof_always.retain(|&id| id >= first);
            for &id in &self.proof_always {
                let k = (id - first) as usize;
                if k < npool && !self.proof_in_touched[k] {
                    self.proof_in_touched[k] = true;
                    self.proof_touched.push(k);
                }
            }
        }
        xp("p_delta");
        // 調べる証明: 最初は差分のある証明すべて。締め付けが起きたら、その記録を差分に取り込んで影響を受けた
        // 証明だけを調べ直す (締め付けの連鎖。全走査の旧方式で先の証明の締め付けが後の証明に効いていたのと同じ)
        let mut scan: Vec<usize> = self.proof_touched.clone();
        for pass in 0..8 {
            let mut tightened_now = false;
            for &k in &scan {
                let rhs = self.proof_rhs(k);
                if !rhs.is_finite() {
                    continue;
                }
                let (minact, ninf) = if full {
                    let (mut minact, mut ninf) = (0.0f64, 0usize);
                    for &(j, a) in &self.dual_proofs[k].0 {
                        let v = if a > 0.0 { a * self.dom.lo[j] } else { a * self.dom.up[j] };
                        if v.is_finite() {
                            minact += v;
                        } else {
                            ninf += 1;
                        }
                    }
                    (minact, ninf)
                } else {
                    let (bm, bi, _) = self.proof_base[k];
                    let (dm, di) = self.proof_delta[k];
                    (bm + dm, (bi as i64 + di as i64).max(0) as usize)
                };
                if ninf > 0 {
                    continue;
                }
                let slack = rhs - minact;
                if slack < -1e-6 * (1.0 + rhs.abs()) {
                    return false;
                }
                if !full && slack >= self.proof_base[k].2 {
                    continue; // どの境界も締まらない
                }
                // 整数列の境界を締める: a > 0 なら x_j <= l_j + slack / a
                let mut changes: Vec<(usize, bool, f64)> = Vec::new();
                for &(j, a) in &self.dual_proofs[k].0 {
                    if !p.is_int[j] || self.dom.lo[j] == self.dom.up[j] {
                        continue;
                    }
                    let range = self.dom.up[j] - self.dom.lo[j];
                    if a.abs() * range <= slack + 1e-9 {
                        continue;
                    }
                    if a > 0.0 {
                        changes.push((j, true, self.dom.lo[j] + slack / a + 1e-6));
                    } else {
                        changes.push((j, false, self.dom.up[j] + slack / a - 1e-6));
                    }
                }
                for (j, upper, v) in changes {
                    let ch = if upper { self.dom.tighten_upper(p, j, v) } else { self.dom.tighten_lower(p, j, v) };
                    if ch {
                        tightened = true;
                        tightened_now = true;
                        self.proof_tightenings += 1;
                    }
                    if self.dom.infeasible {
                        return false;
                    }
                }
            }
            if full || !tightened_now || pass == 7 {
                break;
            }
            let cols = self.dom.take_changed_for_proofs();
            scan = self.proof_absorb(&cols);
            if scan.is_empty() {
                break;
            }
        }
        xp("p_scan");
        if tightened {
            return self.dom.propagate(p);
        }
        true
    }

    /// 列 `cols` の境界の変化 (`proof_val_*` から今の境界へ) を証明の差分 (`proof_delta`) に足し、差分が変わった
    /// 証明を返す (`proof_touched` にも加える)。基準の境界より根の状態が締まっている列は作り直しのときにその差も
    /// 入るので、見積もりは真の最小活動量以下のまま。
    fn proof_absorb(&mut self, cols: &[usize]) -> Vec<usize> {
        let mut changed: Vec<usize> = Vec::new();
        let mut in_changed: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for &j in cols {
            for upper in [false, true] {
                let (old_v, new_v) = if upper { (self.proof_val_up[j], self.dom.up[j]) } else { (self.proof_val_lo[j], self.dom.lo[j]) };
                if old_v == new_v {
                    continue;
                }
                if upper {
                    self.proof_val_up[j] = new_v;
                } else {
                    self.proof_val_lo[j] = new_v;
                }
                for &(id, a) in &self.proof_col_index[j] {
                    if id < self.proof_first_id || (a > 0.0) == upper {
                        continue; // 古い証明、またはこの側は最小活動量に効かない
                    }
                    let k = (id - self.proof_first_id) as usize;
                    let (old, new) = (a * old_v, a * new_v);
                    if !self.proof_in_touched[k] {
                        self.proof_in_touched[k] = true;
                        self.proof_touched.push(k);
                    }
                    if in_changed.insert(k) {
                        changed.push(k);
                    }
                    let e = &mut self.proof_delta[k];
                    if old.is_finite() {
                        e.0 -= old;
                    } else {
                        e.1 -= 1;
                    }
                    if new.is_finite() {
                        e.0 += new;
                    } else {
                        e.1 += 1;
                    }
                }
            }
        }
        changed
    }

    /// 根での再スタート (SCIP の `restartfac`): 根の処理で新たに大域固定された整数列が全体の 2.5% を超えたら、
    /// 大域的な境界と LP に残ったカットを持って前処理からやり直す。暫定解は打ち切り値として渡し、
    /// やり直しで良い解が見つからなければそれを返す。最大 2 回。
    fn maybe_restart(&mut self) -> Option<MipResult> {
        let p = self.p;
        if self.params.submip || self.params.restarts >= tunable!("ENOMOTO_T_MIP_MAX_RESTARTS", 4u32, u32) || env_str!("ENOMOTO_MIP_NO_RESTART").is_some() || self.time_up() {
            return None;
        }
        let nint = p.is_int.iter().filter(|&&b| b).count();
        let fixed = (0..p.n).filter(|&j| p.is_int[j] && self.dom.global_lo[j] == self.dom.global_up[j]).count();
        let newly = fixed.saturating_sub(self.root_fixed0);
        // HiGHS と同じく、初回は固定された整数列が 1 本でもあれば、2 回目以降は 2.5% 以上で再スタートする
        let fac = if self.params.restarts == 0 { tunable!("ENOMOTO_T_MIP_RESTART_FIRST_FAC", 0.0, f64) } else { tunable!("ENOMOTO_T_MIP_RESTART_FAC", 0.025, f64) };
        if nint == 0 || newly == 0 || (newly as f64) < fac * nint as f64 {
            return None;
        }
        // 新しい問題: 大域的な境界 + 元の行 + LP に残ったカット (どれも大域的に成り立つ)
        let mut rows = p.rows.clone();
        let mut row_lo = p.row_lo.clone();
        let mut row_up = p.row_up.clone();
        // カットは根の LP で効いている (活動量が上限にある) ものだけ残す。全部残すと再スタートのたびに行が増え続け
        // (neos-911970: 4 回で 107 行 -> 約 415 行)、ノードの LP が重くなる
        let keep_all = env_str!("ENOMOTO_MIP_RESTART_KEEP_ALL_CUTS").is_some();
        // `ENOMOTO_MIP_RESTART_CUTS_TO_POOL`: カットを問題の行にせず、新しい求解のカットプールに渡す (HiGHS と同じ。
        // 行が増えずノードの LP は軽いが、こちらの分離はカットの行の上にカットを作れなくなるので弱くなる:
        // 対称性の乱数問題で根の下界が閉じず木が数万ノードになる)
        let cuts_as_rows = env_str!("ENOMOTO_MIP_RESTART_CUTS_TO_POOL").is_none() || keep_all;
        let mut carry: Vec<(Vec<(usize, f64)>, f64)> = Vec::new();
        let act = self.lp.row_activities();
        for i in p.m..self.lp.num_rows() {
            let (l, u) = self.lp.row_bounds(i);
            if !cuts_as_rows {
                let r = self.lp.row(i);
                if u.is_finite() {
                    carry.push((r.clone(), u));
                }
                if l.is_finite() {
                    carry.push((r.iter().map(|&(j, v)| (j, -v)).collect(), -l));
                }
                continue;
            }
            let tight = (u.is_finite() && act[i] >= u - 1e-6 * (1.0 + u.abs())) || (l.is_finite() && act[i] <= l + 1e-6 * (1.0 + l.abs()));
            if !keep_all && !tight {
                continue;
            }
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
            skip_heurs: self.params.skip_heurs | self.failed_heurs,
            ..self.params
        };
        if self.params.verbose {
            eprintln!(
                "MIP: restart {} ({} of {} integer columns newly fixed at the root, {} cuts kept) ({:.2}s)",
                params.restarts,
                newly,
                nint,
                newp.m - p.m,
                self.start.elapsed().as_secs_f64()
            );
        }
        if !cuts_as_rows {
            carry.extend(self.cut_pool.iter().map(|(c, r, _)| (c.clone(), *r)));
            super::RESTART_CUTS.with(|s| *s.borrow_mut() = Some(carry));
        }
        // 暫定解を引き継ぐ (新しい問題は列が同じ)
        if env_str!("ENOMOTO_MIP_RESTART_NO_INCUMBENT").is_none() {
            super::RESTART_SOL.with(|s| *s.borrow_mut() = self.incumbent.as_ref().map(|(_, x)| x.clone()));
        }
        let r = super::solve_problem(&newp, params, true);
        super::RESTART_SOL.with(|s| *s.borrow_mut() = None);
        super::RESTART_CUTS.with(|s| *s.borrow_mut() = None);
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

    /// 別スレッドでサブ MIP を解き始める (同時に走らせるのはスレッド数 - 1 本まで。超えたら古いものを待つ)。
    pub(super) fn spawn_submip(&mut self, sub: MipProblem, params: MipParams) {
        let lim = mip_threads().saturating_sub(1).max(1);
        if self.pending_submips.len() >= lim {
            self.join_submips(lim - 1);
        }
        let presolve = env_str!("ENOMOTO_MIP_SUBMIP_NO_PRESOLVE").is_none();
        self.pending_submips.push_back(std::thread::spawn(move || super::solve_problem(&sub, params, presolve)));
    }

    /// 別スレッドのサブ MIP を、残りが `keep` 本になるまで出した順に待って結果を受け取る。
    pub(super) fn join_submips(&mut self, keep: usize) {
        while self.pending_submips.len() > keep {
            let h = self.pending_submips.pop_front().unwrap();
            if let Ok(r) = h.join() {
                self.heur_iters += r.lp_iterations;
                if self.params.verbose {
                    eprintln!("MIP:   parallel sub-MIP: status {:?}, nodes {}, objective {:?}", r.status, r.nodes, r.objective);
                }
                if let Some(x) = r.x {
                    self.try_incumbent(x);
                }
            }
        }
    }

    /// 別スレッドの Feasibility Jump を止めて結果を受け取る。
    fn join_fj_thread(&mut self) {
        if let Some((h, stop)) = self.fj_thread.take() {
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            if let Ok(Some(x)) = h.join() {
                if self.try_incumbent(x) && self.params.verbose {
                    eprintln!("MIP: feasibility jump (parallel) found a solution ({:.2}s)", self.start.elapsed().as_secs_f64());
                }
            }
        }
    }

    fn finish(&mut self, status: MipStatus, best_bound: f64) -> MipResult {
        // 並列モードの別スレッドを止める・待つ
        if let Some((_, stop)) = &self.fj_thread {
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.parallel_submips = false;
        self.join_submips(0);
        self.log(true);
        if self.params.verbose {
            eprintln!("MIP: dual proofs and conflicts {} (conflicts added {} (from proofs {}), Farkas {}, pruned {} nodes, tightened {} bounds)", self.dual_proofs.len(), self.conflicts_added, self.proof_conflicts, self.farkas_added, self.proof_prunes, self.proof_tightenings);
            eprintln!("MIP: sibling backtracks {}; basis restores {} (basic count mismatch {}), first LP iterations after a restore {}", self.sibling_backtracks, self.restore_stats.0, self.restore_stats.1, self.restore_stats.2);
            if !self.orbitopes.is_empty() {
                eprintln!("MIP: orbitopes {}: fixed {} bounds, pruned {} nodes", self.orbitopes.len(), self.orbitope_fixings, self.orbitope_prunes);
            }
            if self.sym_canon != (0, 0) {
                eprintln!("MIP: symmetry: {} heuristic solutions mapped to the symmetry-reduced problem, {} could not be", self.sym_canon.0, self.sym_canon.1);
            }
        }
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
