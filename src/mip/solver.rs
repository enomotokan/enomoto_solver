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
use super::lp::{LpStatus, SolveLimits, VarStatus};
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
    /// 木の探索に使うスレッドの数 (1 なら並列にしない。[`max_threads`] で頭打ちにしてから渡す)。
    pub threads: usize,
    /// 根の緩和問題が非有界のとき、実行不能と非有界を区別するか (`Model.solve(distinguish_infeasible_unbounded=True)`)。
    /// 偽なら `InfeasibleOrUnbounded` を返す。真なら切片問題を整数に制限した問題を解く ([`Solver::resolve_unbounded_root`])。
    pub distinguish: bool,
    /// 固定費用ネットワーク型の問題のサブ MIP か (`ENOMOTO_MIP_FC_SUBMIP` の試作。親が [`Solver::fc_submip`] のときに立てる):
    /// 根のカットのラウンドを通常の求解と同じにする (分離の `ENOMOTO_T_MIP_FC_SUBMIP_CUT_ROUNDS`、既定 25)
    pub fc_submip: bool,
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
        MipParams { time_limit: f64::INFINITY, node_limit: u64::MAX, rel_gap: 0.0, abs_gap: 0.0, verbose: false, submip: false, cutoff: f64::INFINITY, restarts: 0, skip_heurs: 0, threads: 1, distinguish: false, fc_submip: false }
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
    /// 並列のダイビング (`ENOMOTO_MIP_PAR_DIVES`): このスレッドで写しから作る求解が根の後に行うヒューリスティクス
    /// (0 = しない、1 = Feasibility Pump、2 = fractional、3 = vector length、4 = coefficient diving) と、ダイビングの反復の予算、
    /// 根の LP の反復数 (Feasibility Pump の予算に使う)。
    static DIVE_ONLY: std::cell::Cell<(u8, u64, u64)> = const { std::cell::Cell::new((0, 0, 0)) };
    /// 根の解発見スレッド ([`find_thread_on`]) が解を見つけるたびに置く場所 (主が [`Solver::poll_find_thread`] で受け取る)
    static FIND_OUT: std::cell::RefCell<Option<std::sync::Arc<std::sync::Mutex<Vec<Vec<f64>>>>>> = const { std::cell::RefCell::new(None) };
    /// 根の解発見スレッドの止めの合図 (主の暫定解が質の悪い FJ の解でなくなったら主が立てる。[`Solver::time_up`] が見る)
    static FIND_STOP: std::cell::RefCell<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>> = const { std::cell::RefCell::new(None) };
    /// 根のヒューリスティクスの間にカットを強める求解 (`ENOMOTO_MIP_BG_CUTS`): このスレッドで写しから作る求解はカットの
    /// ループだけを行い、止めの合図まで続けて、最後の LP に残ったカットの行と目的値を返す。
    /// カットを強める求解の、主の根の LP の反復数 (カットの LP の反復上限に使う)
    pub(super) static BG_ROOT_ITERS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    pub(super) static CUT_JOB: std::cell::RefCell<Option<std::sync::Arc<CutJob>>> = const { std::cell::RefCell::new(None) };
    /// カットを強める求解の根の LP の初めの基底 (主の根の LP の最後の基底。既定で渡す。`ENOMOTO_MIP_NO_BG_CUTS_BASIS` で渡さない)
    static BG_ROOT_BASIS: std::cell::RefCell<Option<super::lp::Basis>> = const { std::cell::RefCell::new(None) };
    /// 診断用: 既知の最適解 (縮約後の空間、`ENOMOTO_MIP_DEBUG_SOL`)。この解を含むノードが
    /// 枝刈りされたら理由を表示する (SCIP の debug solution と同様)。
    pub(crate) static DEBUG_SOL: std::cell::RefCell<Option<Vec<f64>>> = const { std::cell::RefCell::new(None) };
    /// 次に始める求解 ([`solve`]) の最初の暫定解 (その問題の空間)。再スタートで前の暫定解を引き継ぐのに使う。
    /// 求解の開始時に取り出す (同じスレッドで後から始まるサブ MIP には渡らない)。
    pub(crate) static START_SOL: std::cell::RefCell<Option<Vec<f64>>> = const { std::cell::RefCell::new(None) };
    /// 次に始める求解の、暫定解にはできない参照解 (`ENOMOTO_MIP_RESTART_REF`、試作)。再スタート前の暫定解が、打ち切り値による
    /// 固定 (被約費用・根の目的値の探り) で再スタート後の問題では実行不能になったときに置き、RINS の相手に使う。
    pub(crate) static REF_SOL: std::cell::RefCell<Option<Vec<f64>>> = const { std::cell::RefCell::new(None) };
    /// 次に始めるサブ MIP の根の LP の初めの基底 (列の状態。`ENOMOTO_MIP_NO_SUBMIP_BASIS` でなければ)。呼び出し側の LP の基底を渡す
    /// (HiGHS のサブ MIP の `rootbasis`)。前処理は列の番号を保つので列の状態はそのまま使え、行は論理変数で埋めて修復する。
    pub(crate) static SUBMIP_BASIS: std::cell::RefCell<Option<Vec<super::lp::VarStatus>>> = const { std::cell::RefCell::new(None) };
    /// 次に始めるサブ MIP が根のサブ MIP なら、その成果連動の打ち切りの情報 ([`SubmipCtl`]、`ENOMOTO_MIP_SUBMIP_OUTCOME_STOP`)。
    /// サブ MIP の最初の求解が取り出す (入れ子のサブ MIP には渡らない)
    pub(crate) static ROOT_SUBMIP_CTL: std::cell::RefCell<Option<std::sync::Arc<SubmipCtl>>> = const { std::cell::RefCell::new(None) };
    /// 次に始める求解 (サブ MIP でないもの) の根の LP の初めの基底 (列の状態)。前処理の LP による固定
    /// ([`super::lp_probe`]) が解いた LP の基底を渡す (根の LP を解き直す手間を減らす)。
    pub(crate) static ROOT_BASIS: std::cell::RefCell<Option<super::lp::Basis>> = const { std::cell::RefCell::new(None) };
    /// [`ROOT_BASIS`] の DSE 重み (`ENOMOTO_MIP_LP_PROBE_FINISH` のときだけ。番号は構造列 j < n、行 n + i)
    pub(crate) static ROOT_DSE: std::cell::RefCell<Option<Vec<(u32, f32)>>> = const { std::cell::RefCell::new(None) };
    /// 再スタートの後の根の LP の初めの基底 (`ENOMOTO_MIP_RESTART_BASIS`、試作)。再スタートの直前の LP の列の状態と、
    /// 各行 (元の行とカットの行) の中身と論理変数の状態。前処理は列の番号を保つが行は消したり並べ替えたりするので、
    /// 行の状態は行の中身で照らし合わせて引き継ぐ ([`restart_basis_for`])
    pub(crate) static RESTART_BASIS: std::cell::RefCell<Option<(Vec<super::lp::VarStatus>, Vec<(Vec<(usize, f64)>, super::lp::VarStatus)>)>> = const { std::cell::RefCell::new(None) };
    /// `ENOMOTO_MIP_LP_PROBE_ROW_BASIS`・`ENOMOTO_MIP_LP_PROBE_FINISH`: [`ROOT_BASIS`] の行の状態を作った LP の行の指紋
    /// ([`super::rows_fingerprint`])。根の LP の始めの行が同じ (指紋が合う) ときだけ行の状態を使う。列の状態だけだと、基底の残りを
    /// 行 0, 1, 2, ... の論理変数で埋めることになり ([`crate::simplex::mip_lp::TwoStageLp::set_basis`] の修復)、最適基底とは遠い基底から
    /// 解き直す (uccase12 で根の LP が 40 秒終わらなかった。analysis/uccase12_20261023.md)
    pub(crate) static ROOT_BASIS_ROWS: std::cell::RefCell<Option<u64>> = const { std::cell::RefCell::new(None) };
    /// ヒューリスティクス役のスレッド (`ENOMOTO_MIP_HEUR_ROLE`、[`Solver::heur_role_run`]) の共有の待ち行列。このスレッドで
    /// 解くサブ MIP も、木の探索が終わるか止める合図が立ったら打ち切る ([`Solver::time_up`])。
    static ROLE_POOL: std::cell::RefCell<Option<std::sync::Arc<WorkPool>>> = const { std::cell::RefCell::new(None) };
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
    /// 求解の開始時の [`super::cap_clock`] (根のカットのループの上限に使う)。
    pub(super) cpu0: f64,
    /// 根の分離を並列にしたとき、ほかのスレッドが使った CPU 時間の合計 (`ENOMOTO_MIP_SEP_COUNT_CPU` なら根のカットの時間の
    /// 上限に含める。含めないと、主の CPU 時間で測る上限の中でラウンドが増え、実時間はかえって長くなる)。
    pub(super) sep_par_cpu: f64,
    /// 並列の分離で、経路集約の集約した行を集める先 (`Some` の間は [`Self::path_aggregation`] がカットを作らずにここへ入れる)。
    pub(super) path_collect: Option<Vec<Vec<(usize, f64)>>>,
    /// カットを強める求解で回したラウンド数
    pub(super) bg_cut_rounds: usize,
    /// 根のカットのループで固定費用ネットワーク型と判定したか (`ENOMOTO_MIP_FACE_SEP_FC`、試作)
    pub(super) fc_network: bool,
    /// 根の面の頂点の分離 (面の LP とその頂点での分離) に使った時間の合計と、使った面の頂点の数 (`ENOMOTO_MIP_DEBUG_FACETIME`)
    pub(super) face_secs: f64,
    pub(super) face_used: usize,
    /// `ENOMOTO_MIP_FACE_ADAPT`: このカットのループでは面の頂点の分離をもうしない (面の LP が解けなかった)
    pub(super) face_off_loop: bool,
    /// `ENOMOTO_MIP_TAB_BUDGET`: tableau の分離の呼び出し回数と、試した行の数の累計 (HiGHS の `numTries`)
    pub(super) tab_calls: u64,
    pub(super) tab_tries: u64,
    /// 直前のカットのラウンドの LP の時間 (秒、`ENOMOTO_MIP_FACE_ADAPT` で面の LP の時間の上限に使う。未知なら無限大)
    pub(super) last_cut_lp_secs: f64,
    /// 再スタート前の暫定解 (`START_SOL`) を受け取っている間か ([`Self::try_incumbent`] の `ENOMOTO_MIP_RESTART_REF` の判定を飛ばす)
    accepting_start: bool,
    /// 並列の強分岐 (`ENOMOTO_MIP_PAR_SB`) の [まとめた回数, 並列に解いた候補の数]。
    par_sb_stats: [u64; 2],
    pub(super) deadline: Option<Instant>,
    pub(super) nodes: u64,
    /// 強分岐に使った LP 反復数。
    pub(super) sb_iters: u64,
    /// 強分岐に使った時間 (秒) と、強分岐で解いた LP の数。
    pub(super) sb_secs: f64,
    pub(super) sb_lps: u64,
    /// 分枝の選択の回数と、そのうち双対退化で強分岐をしなかった回数 (診断用)。
    pub(super) sb_degen_stats: [u64; 2],
    /// `ENOMOTO_MIP_SB_AGREE` (試作): 直近の強分岐で、擬似費用の 1 位が強分岐の 1 位と (ほぼ) 一致したか ([`Solver::sb_agree_learned`])
    sb_agree: std::collections::VecDeque<bool>,
    /// `ENOMOTO_MIP_SB_AGREE`: 学べたと判断して強分岐を省いたノードの数 (間隔 `ENOMOTO_T_MIP_SB_AGREE_PROBE` ごとに強分岐を続ける)
    sb_agree_tick: u64,
    /// `ENOMOTO_MIP_SB_AGREE` の集計: [判定した回, 一致した回, 強分岐を省いたノード]
    sb_agree_stats: [u64; 3],
    /// 直前の分枝の選択で、選んだ列の強分岐の子の LP 値 (列, 下の子, 上の子)。最適まで解けなかった側は -inf。
    pub(super) last_sb: Option<(usize, f64, f64)>,
    /// clique カット用の 2 値列の衝突グラフ (根で作る)。
    pub(super) clique_graph: Option<std::rc::Rc<super::clique::CliqueGraph>>,
    /// `ENOMOTO_MIP_CUT_CLIQUE_EXTEND`: 根のカットのループで LP に入れた集合詰め込みの形のカットの文字の組 (衝突グラフに足す辺。文字 -> 隣の文字)
    pub(super) cut_clique_adj: std::collections::HashMap<u32, Vec<u32>>,
    /// `ENOMOTO_MIP_CLIQUE_CUTS=auto` の判定 (根で一度だけ試す。`None` は未判定)。
    pub(super) clique_auto: Option<bool>,
    /// ヒューリスティクスに使った LP 反復数。
    pub(super) heur_iters: u64,
    /// 根のヒューリスティクスを終えた時点の `heur_iters` (木の中の予算は、これを引いた分で数える)。
    pub(super) heur_iters_root: u64,
    /// 列ごとの lock 数 (下げると違反しうる行の数, 上げると違反しうる行の数)。
    pub(super) locks: Vec<(u32, u32)>,
    /// 擬似乱数の状態。
    pub(super) rng: u64,
    /// 根の LP の目的値と、非基底の構造変数の (列, 下限側か, 被約費用)。暫定解が良くなるたびに
    /// 大域的な被約費用固定に使う。
    pub(super) root_redcost: Option<(f64, Vec<(usize, bool, f64, f64)>)>,
    /// 固定費用ネットワーク型の問題で、木のノードの分離を強めるか ([`Solver::node_cuts_due`]、`ENOMOTO_MIP_FC_NODE_SEP` で使う。評価中)
    pub(super) fc_node_sep: bool,
    /// `ENOMOTO_MIP_FC_NODE_SEP_ADAPT`: [`Self::fc_node_sep`] の毎ノードの分離を、効き目 (分離後の解き直しで LP の値が
    /// 打ち切り値までの差のどれだけ上がったか) と時間で絞る ([`Solver::node_cuts_due`])。評価中
    pub(super) fc_adapt: bool,
    /// `ENOMOTO_MIP_FC_NODE_SEP2`: [`Self::fc_node_sep`] の分離を、待ち行列から取り出したノードでプールのカットを戻した後にも
    /// 行い、LP の値が伸びる間ラウンドを繰り返す (HiGHS の `HighsSeparation::separate`)。評価中
    pub(super) fc_sep2: bool,
    /// `ENOMOTO_MIP_FC_SUBMIP` (試作、既定では使わない。2026-10-21 に既定にしたが、同じ経路 (SUBMIP_CUTS + サブ MIP のカット 25 ラウンド) で mc11 が誤った最適値 11712 を返した回があったので戻した): 固定費用ネットワーク型 ([`super::separation::fixed_charge_network_for_nodes`])
    /// の問題で、ヒューリスティクスのサブ MIP に LP のカットの行を加え (`ENOMOTO_MIP_SUBMIP_CUTS` と同じ)、サブ MIP の根のカットの
    /// ラウンドを通常の求解と同じにする。analysis/beasleyC3_r3_20261020.md
    pub(super) fc_submip: bool,
    /// 木の中の分離の規則 ([`Solver::tree_sep_start`]、既定で使う。`ENOMOTO_MIP_NO_TREE_SEP` で使わない): 待ち行列から取り出した
    /// ノードでもプールのラウンドの後に分離し、LP の値が伸びる間ラウンドを繰り返す。40 問 + 16 問 (4 スレッド・60 秒、2 組) で
    /// neos-911970 が 2 組とも解け (既定は時間切れ)、40 問の解けた数 24/25 -> 26/26、目的値の不一致 0
    pub(super) tree_sep: bool,
    /// [`Self::tree_sep`] で、固定費用ネットワーク型ならノードの分離でも経路集約の始点を使う
    pub(super) tree_sep_fc: bool,
    /// ノードの分離の効き目の統計: (回数, 効き目の指数移動平均, 分離で枝が切れた回数, 深さの規則から外れて分離した回数)
    pub(super) fc_stats: (u64, f64, u64, u64),
    /// 効き目が低いときに、何回に 1 回は試し続けるかの数え
    pub(super) fc_probe: u64,
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
    /// ノードでの分離 (カットを加えた後の LP の解き直しを含む) に使った時間 [秒]。
    pub(super) node_cut_secs: f64,
    /// HiGHS 流のカット管理 (`ENOMOTO_MIP_HIGHS_CUTMGMT`) でのノードの分離の進み具合の基準にする根の LP の目的値
    /// (カットのループの後、定数項込み。HiGHS の `rootlpsolobj`)。
    pub(super) hcm_root_obj: f64,
    /// 効き目で続けるノードの分離 (`ENOMOTO_MIP_NODE_CUT_ADAPT`、[`Solver::node_cut_adapt_due`]): 結果待ちの分離の前の
    /// ノードの LP の値、効いた割合の指数移動平均、試した回数・効いた回数・効き目が低くて飛ばした回数。
    pub(super) nca_pending: Option<f64>,
    pub(super) nca_ema: f64,
    pub(super) nca_stats: [u64; 3],
    /// 診断用: ノードの LP の行数の合計 (`node_lps` で割ると平均)、木のノードでの新しい分離のラウンド数と加えたラウンド数
    pub(super) node_lp_rows: u64,
    pub(super) node_sep_stats: [u64; 2],
    /// 診断用: 効き目で続けるノードの分離の結果の内訳 (打ち切り、伸び/差が 0.02 未満、0.1 未満、0.3 未満、0.3 以上)
    pub(super) nca_hist: [u64; 5],
    /// ノードで加えたカットの非零の数の指数移動平均 (`ENOMOTO_T_MIP_NODE_CUT_ADAPT_DENSITY`、[`Solver::nca_dense`])。
    /// まだ測っていなければ None (最初に LP にある根のカットの行の平均で始める)
    pub(super) nca_cut_nnz: Option<f64>,
    /// 暫定解がない間の木の中のヒューリスティクス (RENS・Feasibility Pump) に使った時間 (秒) と呼んだ回数。
    pub(super) noinc_secs: f64,
    pub(super) noinc_calls: u64,
    /// 直前に呼んだヒューリスティクスが解にどれだけ近づいたか (0..1。解が見つからなかったときの報酬に使う)。
    /// ダイビングは分数の列の数の減り方、Feasibility Pump は丸めた点との距離の減り方、RENS は固定率。
    pub(super) heur_close: f64,
    /// 暫定解がない間の木の中のヒューリスティクスのバンディット (`ENOMOTO_MIP_NOINC_BANDIT`): 腕ごとの報酬の合計・
    /// 試した回数・使った時間 (秒)。腕は [`NOINC_ARMS`] の順。
    noinc_reward: [f64; NOINC_ARMS],
    noinc_count: [u32; NOINC_ARMS],
    noinc_cost: [f64; NOINC_ARMS],
    /// 暫定解がない間の呼び出しの間隔の調整 (`ENOMOTO_MIP_NOINC_BUDGET`): 近さの最大値が更新されなかった続けての回数、
    /// これまでの近さの最大値、次に呼んでよいノード番号。
    /// 根で同じヒューリスティクスを呼んだときの近さ (`ENOMOTO_MIP_NOINC_ROOT_PRIOR` でバンディットの初期値にする)。
    noinc_prior: [Option<f64>; NOINC_ARMS],
    /// 「木を進める」腕を選んで、木だけを進めている間の (始めた時刻 [秒], 終える時刻 [秒], その間のノードの LP 解の
    /// 分数の列の数の最小)。
    noinc_tree_win: Option<(f64, f64, usize)>,
    /// 根のヒューリスティクスを始めるときの LP 解の分数の列の数 (「木を進める」腕の近さの基準)。
    noinc_root_frac: usize,
    /// 早めの再スタート ([`super::early_restart_on`]): 根のヒューリスティクスの途中で再スタートの判定が通った (残りの根のヒューリスティクスを飛ばす)。
    early_restart: bool,
    /// `ENOMOTO_MIP_TREE_RESTART` (試作): 木の中で暫定解が良くなった (木のループの頭で再スタートを判定する)。
    tree_restart_check: bool,
    noinc_stall: u32,
    noinc_best_close: f64,
    noinc_next_node: u64,
    /// サブ MIP の時間の上限 (残り時間に対する割合)。
    pub(super) submip_time_frac: f64,
    /// サブ MIP の時間の上限 (秒)。
    pub(super) submip_time_max: f64,
    /// 暫定解がない根の RENS (ダイビング式・長めのサブ MIP) を行ったか。
    pub(super) noinc_rens_done: bool,
    /// サブ MIP のノード上限の倍率 (根の RENS のやり直しで一時的に上げる)。
    pub(super) submip_node_mult: f64,
    /// 直近の (旧来の) RENS のサブ MIP がノード上限で止まったか。
    pub(super) rens_node_limited: bool,
    /// RENS・RINS の固定を目標の固定率で止める (`ENOMOTO_MIP_FIX_CAP` と同じ) のを一時的に強制する
    /// (根の広い近傍のサブ MIP (`ENOMOTO_MIP_ROOT_WIDE_SUBMIP`) の広い近傍のやり直しの間だけ真)。
    pub(super) force_fix_cap: bool,
    /// 並列モードで、サブ MIP を別スレッドで解くか (根のヒューリスティクスの間だけ真)。
    pub(super) parallel_submips: bool,
    /// 別スレッドで解いているサブ MIP (出した順)。
    /// 並列モードで別スレッドに出したサブ MIP と、それが根の RENS のものか ([`Self::submip_is_rens`] を出すときに写す)。
    pub(super) pending_submips: std::collections::VecDeque<(std::thread::JoinHandle<MipResult>, bool, &'static str, std::time::Instant, Option<f64>, Option<std::sync::Arc<SubmipCtl>>)>,
    /// [`Self::add_proof_cmir`] の作業領域 (丸めの基準の点) と、試した数・足した数。
    proof_cmir_x: Option<Vec<f64>>,
    pub(super) proof_cmir_stats: [u64; 3],
    /// 次に出すサブ MIP の固定率 (RENS・RINS の固定率の記録 `rens_succ`・`rens_infeas` に結果を反映するため)。
    pub(super) submip_fix_rate: Option<f64>,
    /// いま走らせているヒューリスティクスの名前 (ログで暫定解の出どころを示すため)
    pub(super) heur_label: &'static str,
    /// 根のヒューリスティクス 1 回の時間の上限 (`ENOMOTO_T_MIP_ROOT_DIVE_TIME_FRAC`。[`Solver::time_up`] が見る)
    pub(super) heur_stop: Option<Instant>,
    /// 最初の根の LP の反復数 (丸め系のヒューリスティクスの連続部分の LP の反復上限に使う)
    pub(super) root_lp_iters: u64,
    /// 暫定解が Feasibility Jump で見つけたものだけか (`ENOMOTO_MIP_FJ2_KEEP_NOINC` の判定に使う)
    pub(super) inc_only_fj: bool,
    /// 暫定解がないときに RINS の相手に使う参照解 ([`REF_SOL`]、`ENOMOTO_MIP_RESTART_REF`)
    pub(super) ref_sol: Option<Vec<f64>>,
    /// 根の LP (カットのループの後は最後の LP) の目的値。`ENOMOTO_MIP_FJ_POOR_NOINC` の判定に使う (根の外では -inf)
    pub(super) fj_poor_lb: f64,
    /// サブ MIP の停滞の数え上げ (`ENOMOTO_T_MIP_SUBMIP_STALL`)
    submip_stall: u64,
    /// 根のサブ MIP の成果連動の打ち切りの情報 ([`ROOT_SUBMIP_CTL`]、サブ MIP の側)
    root_submip: Option<std::sync::Arc<SubmipCtl>>,
    /// 暫定解が最後に良くなったときのノード数 ([`submip_outcome_stop`])
    last_improve_node: Option<u64>,
    /// 次に別スレッドで出すサブ MIP の根の LP の初めの基底 ([`SUBMIP_BASIS`])
    pub(super) submip_basis: Option<Vec<super::lp::VarStatus>>,
    /// ヒューリスティクス役を務めている間か (ログで暫定解の出どころを示すため)
    pub(super) in_heur_role: bool,
    /// 受け取った解が別のスレッドのヒューリスティクスのものか (ログの表示用)
    other_thread_label: bool,
    /// 次に出すサブ MIP が根の RENS のものか (受け取ったときにノード上限で止まっていれば `rens_node_limited` を立てる)。
    pub(super) submip_is_rens: bool,
    /// `ENOMOTO_MIP_SUBMIP_HIGHS` の `i`: 根の最初の RINS の LP 解 (HiGHS の `rootlpsol` の代わり。固定の向きを決める)
    pub(super) rins_root_x: Option<Vec<f64>>,
    /// 別スレッドの Feasibility Jump とその停止の印。
    fj_thread: Option<(std::thread::JoinHandle<Option<Vec<f64>>>, std::sync::Arc<std::sync::atomic::AtomicBool>)>,
    /// `ENOMOTO_MIP_LOCALMIP` の別スレッド (試作)
    lmip_thread: Option<(std::thread::JoinHandle<Option<Vec<f64>>>, std::sync::Arc<std::sync::atomic::AtomicBool>)>,
    /// `ENOMOTO_MIP_LOCALMIP` のスレッドが良くなった解を入れる箱
    lmip_mailbox: Option<std::sync::Arc<std::sync::Mutex<Vec<Vec<f64>>>>>,
    /// `ENOMOTO_MIP_FJ_NOSOL_EXTEND`: 暫定解を持ったら立てて、FJ のスレッドの延長をやめさせる印
    fj_has_inc: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// 根のヒューリスティクスの最初に別スレッドで始めた局所探索 (`ENOMOTO_MIP_PAR_LOCAL_SEARCH`。結果は元の位置で受け取る)。
    pub(super) ls_thread: Option<std::thread::JoinHandle<super::heuristics::LocalSearchResult>>,
    /// 根の解発見スレッド ([`find_thread_on`])。根のヒューリスティクスの後と木の各ノードで、終わっていれば受け取る
    find_thread: Option<std::thread::JoinHandle<MipResult>>,
    find_out: Option<std::sync::Arc<std::sync::Mutex<Vec<Vec<f64>>>>>,
    find_stop: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
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
    /// 診断用 (`ENOMOTO_MIP_RESTORE_DIAG`): 直前に戻した基底の (保存後に足された行, 消えた行, 基底の数の不一致)
    restore_diag: (usize, usize, bool),
    /// 診断用: 共有の待ち行列から受け取ったノードの基底の、基底変数の余り (受け取ったときに数える)
    pool_excess: i64,
    /// `ENOMOTO_MIP_TREE_POOL_BASIS` で受け取った基底を使わなかった回数
    pool_basis_stats: u64,
    /// 直前に戻した基底に、保存した DSE 重みも当てはめたか (次の LP で引き継ぐ)。
    restored_dse: bool,
    node_lps: u64,
    /// ノードの LP にかかった時間の合計 (秒)。1 回の LP の時間の上限に使う。
    node_lp_secs: f64,
    /// 解けなかったノード (LP が失敗し、分枝もできなかった)。最適性を主張できなくなる。
    unresolved: bool,
    last_log: Instant,
    /// 利用者向けのログ ([`crate::log`]) の探索の経過の見出しを出したか。
    user_log_header: bool,
    /// 処理中 (最後に取り出した) のノードの下界 (根では根の LP の値)。待ち行列にないので全体の下界に足す。
    cur_lb: f64,
    /// 根の LP の値 (カットのループの後はその値)。この求解の問題全体の下界として常に正しいので、時間切れなどで終わるときに
    /// 返す下界はこれより下げない (従のスレッドの失敗・再スタート後の求解の結果などで下界が失われても、根の下界は返す)。
    root_lb: f64,
    /// 根のカットのループで最適に解けた LP の値の最大 (カットは大域的に成り立つので問題全体の下界。後でカットを外しても正しい)
    pub(super) root_cut_lb: f64,
    /// 終わるときの下界 ([`Self::finish`] が決めたもの)。あれば最後の経過の行はこれを使う。
    final_lb: Option<f64>,
    /// カット生成に使う変数上下限 (最初の分離で作る)。
    pub(super) vbounds: Option<Rc<super::cuts::VarBounds>>,
    /// 根で解を見つけなかったヒューリスティクス ([`heur_bit`] のビット)。再スタートで引き継ぐ。
    failed_heurs: u64,
    /// LP の行の追加・削除の記録 (待ち行列のノードの基底を、保存した後の行の変化に合わせるのに使う)。
    pub(super) row_log: Vec<RowEdit>,
    /// LP のカットの行 (元の行より後ろ) の年齢: 続けて効いていなかったノードの LP の数 ([`Self::age_cuts`])。
    pub(super) cut_age: Vec<u32>,
    /// LP のカットの行の番号 (追加順に振る、行を消しても変わらない)。`cut_age` と同じ並び。
    pub(super) cut_ids: Vec<u64>,
    pub(super) next_cut_id: u64,
    /// カットの行の係数の 2 ノルム (カットの番号ごと。相対的な余裕の基準 `ENOMOTO_T_MIP_CUT_SLACK_REL` で使う)。
    pub(super) cut_norm: std::collections::HashMap<u64, f64>,
    /// 効いていたカットを双対値でまとめた行の番号 ([`Self::merge_active_cuts`])。年齢で外さない。
    pub(super) merged_cuts: std::collections::HashSet<u64>,
    /// `ENOMOTO_MIP_TREE_SEP` のラウンドで加えたカットの番号 (`ENOMOTO_T_MIP_TREE_SEP_CUT_AGE` で年齢の上限を別にする)
    pub(super) ts_cut_ids: std::collections::HashSet<u64>,
    /// `ENOMOTO_MIP_TREE_SEP`: 最初に分離を考えた取り出したノードでの LP の行数 (`ENOMOTO_T_MIP_TREE_SEP_HEAVY` の基準)
    pub(super) ts_rows0: usize,
    /// 最適面の頂点での分離 (`separate_face_split`) で、今の頂点の tableau 行 (基底変数ごとの、集約した行とその列の
    /// LP 値のハッシュ)。`tab_memo_mode`: 0 = 使わない、1 = 記録する、2 = 照合する。
    pub(super) tab_memo: std::collections::HashMap<usize, u64>,
    pub(super) tab_memo_mode: u8,
    /// (照合した行の数, 同じだった行の数)
    pub(super) tab_memo_stats: (u64, u64),
    /// 目的値の二分探索 ([`Self::objective_bisection`]): 目的値が `bisect_lb` 未満の解はないと証明した値、
    /// 次の目標値の位置 (下界と暫定解の間の割合)、最後に呼んだノード数、使った時間、(呼び出し, 解, 証明) の回数。
    pub(super) bisect_lb: f64,
    pub(super) bisect_frac: f64,
    pub(super) bisect_last: u64,
    pub(super) bisect_secs: f64,
    pub(super) bisect_stats: [u64; 3],
    /// ポートフォリオ並列の共有値 (サブ MIP では持たない)。
    pub(super) shared: Option<std::sync::Arc<Portfolio>>,
    /// 二分探索のスレッドの求解なら、見張り (打ち切る最良値, 目標値)。最低 `ENOMOTO_T_MIP_BISECT_MIN_RUN` 秒走った後で打ち切る。
    pub(super) watch: Option<(f64, f64)>,
    /// 目的関数の下界のカットを最後に作ったときの厳密な下界 ([`Portfolio::proven`])。
    pub(super) obj_cut_p: f64,
    /// 分離に一時的に加える行 (`<=` の形 (係数, 右辺)。変数は構造変数 j < n と LP の行の活動量 n + i)。
    pub(super) extra_sep_rows: Vec<(Vec<(usize, f64)>, f64)>,
    /// (目的関数の下界のカットを試した回数, カットを加えた回数)
    pub(super) obj_cut_stats: [u64; 2],
    /// 木の写しから解く求解なら、その写し ([`TreeSnapshot`]) と、写しのノードごとの「部分木のノードがまだ残っているか」。
    pub(super) snap: Option<std::sync::Arc<TreeSnapshot>>,
    snap_alive: Vec<bool>,
    /// 木の探索のスレッド: 写しに入れる問題 (最初の写しで作る)、作った写しの通し番号、受け取った示した値・解の更新の回数、
    /// [作った写し, 下界を上げたノード, 受け取った解]
    snap_p: Option<std::sync::Arc<MipProblem>>,
    snap_serials: Vec<u64>,
    fb_seen: u64,
    inbox_seen: u64,
    snap_stats: [u64; 3],
    /// 木の探索を分け合う共有の待ち行列 ([`WorkPool`])、自分が待っていない側に数えられているか、主が始めた従のスレッド、
    /// 従の結果の合計 (ノード数, LP の反復数)、[ここへ渡したノード, ここから取ったノード]
    pub(super) pool: Option<std::sync::Arc<WorkPool>>,
    pool_busy: bool,
    /// ヒューリスティクス役 ([`Self::heur_role_run`]) を終えて木の探索に戻ったときの (時刻, 共有の最良値, 次に役に戻るまでの
    /// 秒数) (`ENOMOTO_MIP_HEUR_ROLE_REENTER` でなければ、戻ったら二度と役に就かない)、役に就いた回数、
    /// 役の間の [大近傍探索の呼び出し, 暫定解を良くした回数, 取り込んだ解の数]。
    heur_role_retired: Option<(Instant, f64, f64)>,
    heur_role_entries: u64,
    /// 起動の判定 (`ENOMOTO_MIP_HEUR_ROLE_TRIGGER`): 前回の (時刻, 木の完了率) と、完了率の進む速さ (毎秒、指数平滑)。
    trig_last: Option<(Instant, f64)>,
    trig_v: f64,
    /// 前回判定した時刻 (0.25 秒おきに判定する)。
    trig_eval_at: Option<Instant>,
    /// 根のヒューリスティクスのうち暫定解があるときに呼んだものの (縮めたギャップの割合, 秒) (主解への寄与の事前の値)。
    root_mu_obs: Vec<(f64, f64)>,
    heur_role_stats: [u64; 3],
    /// ヒューリスティクス役で ALNS と競わせる Local-MIP (`ENOMOTO_MIP_ROLE_LMIP`、試作)。役を離れても状態を持ち越す
    role_lm: Option<super::localmip::LocalMip<'a>>,
    /// Local-MIP を作れなかった・続けられない
    role_lm_off: bool,
    /// 腕 (0 = ALNS、1 = Local-MIP) ごとの使った秒と、暫定解を良くした回数
    role_arm_t: [f64; 2],
    role_arm_wins: [u64; 2],
    /// 先導 (直近に暫定解を良くした腕): 0 = なし、1 = ALNS、2 = Local-MIP
    role_leader: u8,
    /// `ENOMOTO_MIP_SYM_ORIG_CUT` (試作): 対称性の行への並べ替えに失敗したが元の問題で実行可能な解のうち最良のもの (目的値, 解)。
    /// 値は打ち切り値 (刈り込みの上限・共有の最良値) にだけ使い、暫定解 (ヒューリスティクスの近傍の中心・「暫定解なし」の判定) には
    /// しない。終わるときに暫定解より良ければ答えにする ([`Self::finish`])
    orig_best: Option<(f64, Vec<f64>)>,
    /// 主: 従を始めた後、ノードを渡し始めるまで待つ期限 (待ち行列のノードが `ENOMOTO_T_MIP_TREE_START_NODES` 個たまるか、
    /// この時刻を過ぎたら渡し始める)。従は先に始めて準備 (写しからの LP・根の LP) を主の探索と並行させる。
    pool_hold: Option<Instant>,
    /// 従が最後に見た共有の最良値。
    seen_best: f64,
    /// 擬似費用を前回合わせたときの自分の値 (この後に増えた観測を共有の擬似費用に足す)。
    pc_base: Option<Pseudocost>,
    /// 木を分け合うときの自分の番号、学習の記録をどこまで読んだか、ほかのスレッドの記録を取り込んでいる最中か、
    /// 取り込んだ [証明, 衝突] の数。
    pub(super) pool_tid: u32,
    /// racing ramp-up: このスレッドが競争の結果を処理したか、探索の多様化の種類 (bit 0: 子の順を逆に、bit 1: 分枝の
    /// スコアに揺らぎ) と、その乱数の状態。
    race_done: bool,
    race_div: u8,
    race_rng: u64,
    learned_cursor: u64,
    pub(super) importing: bool,
    learned_stats: [u64; 2],
    /// 診断用: デバッグ解の対称な像、最後の刈り込みの理由、直前に処理したノードが像を含んでいたか、像がまだ残っているか。
    dbg_imgs: Option<Vec<Vec<f64>>>,
    dbg_last: std::cell::RefCell<String>,
    dbg_prev_has: bool,
    dbg_alive: bool,
    /// 診断用: デバッグ解があり (サブ MIP でない) 診断を行うか (構築時に決める)。偽なら診断の文字列を作らない。
    pub(super) dbg_on: bool,
    workers: Vec<std::thread::JoinHandle<MipResult>>,
    worker_totals: (u64, u64),
    pool_stats: [u64; 3],
    /// 診断用 (`ENOMOTO_PROF_TREE_PAR`): 木の探索の内訳と、今のノードの種類 ([`TreeProf::kind_n`])。
    tprof: TreeProf,
    tp_kind: usize,
    lbd: LbNodeDiag,
    /// 証明から作った衝突の数。
    proof_conflicts: u64,
    /// 完全オービトープ (orbitopal fixing に使う。サブ MIP では空)。
    orbitopes: Rc<Vec<super::symmetry::Orbitope>>,
    /// lex-leader の行を作った生成元 ([`super::SYM_LEX_GENS`]。`ENOMOTO_MIP_SYM_LEX_CANON` で使う)
    lex_gens: Rc<Vec<Vec<usize>>>,
    /// 対称性の行・固定を加える前の問題 (なければ `None`)。ヒューリスティクスの解の判定・局所探索に使う。
    pub(super) orig: Option<Rc<MipProblem>>,
    /// オービトープの列 -> (オービトープの番号, 行)。
    orb_col: std::collections::HashMap<usize, (usize, usize)>,
    /// オービトープごとの、生成元が動かすオービトープの外の列 ([`Self::orbitope_propagate`])。
    orb_others: Option<Rc<Vec<std::collections::HashSet<usize>>>>,
    /// 今の潜りで待ち行列に入れた兄弟ノード ((枠, 世代, 親の LP の LU キャッシュ)、古い順)。潜りが枝刈りで終わったら
    /// ここから戻る。LU キャッシュは兄弟の始点の基底 (親の最適基底) の分解を省くのに使う
    /// ([`MipLp::adopt_lu_snapshot`]、`ENOMOTO_MIP_NO_SIBLING_LU` で持たない)。
    dive_stack: Vec<(usize, u64, Option<crate::simplex::mip_lp::LuSnapshot>)>,
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
    /// 証明ごとの、係数の並びに対応する締め付けの上界 `|a_j| (u_j - l_j)` (加えたときの大域的な境界。整数列でなければ 0)。
    /// 係数はこの大きい順に並べて持つ (締め付けでは、これが余裕以下になったら残りの列は締まらないので打ち切る)。
    proof_keys: std::collections::VecDeque<Vec<f64>>,
    /// 診断用 (`ENOMOTO_MIP_PROOF_VERIFY`): 自前の証明を作ったときに y_i != 0 だったカットの行の番号 (`cut_ids`)。取り込んだ行・
    /// 衝突の行は `None`。刈り込みの確認で LP 解が証明を破ったとき、使ったカットが全部 LP に残っていれば証明の誤りと断定できる。
    proof_cuts: std::collections::VecDeque<Option<Vec<u64>>>,
    /// 診断用: 次に `push_pool_row` で入れる自前の証明が使ったカットの行の番号 (`add_proof_from` が置く)。
    proof_pending_cuts: Option<Vec<u64>>,
    /// 証明ごとの状態 (`dual_proofs` と同じ並び、[`ProofState`])。
    proof_state: std::collections::VecDeque<ProofState>,
    /// 生きている証明の通し番号を古い順に、[自前, 取り込み] ごとに (上限を分けて古いものから捨てるため)。
    proof_fifo: [std::collections::VecDeque<u64>; 2],
    /// 捨てた (中身を空にした) が、まだ `dual_proofs` に残っている証明の数。
    proof_dead: usize,
    /// 証明ごとの、前に捨てる候補になってから刈り込み・締め付けに使われたかの印 (木を分け合うとき、`ENOMOTO_MIP_PROOF_NO_AGING` でなければ: 捨てるときに
    /// 印のあるものは印を消して後ろに回す。clock 法による「最近使われていないもの」の近似)。
    proof_ref: std::collections::VecDeque<bool>,
    /// 列ごとの、その列を含む証明 (通し番号, 係数)。[0] は係数が正のもの (下限が変わると最小活動量が変わる)、[1] は
    /// それ以外 (上限が変わると変わる)。境界の変わった側だけをたどるように分けて持つ。
    proof_col_index: [Vec<Vec<(u64, f64)>>; 2],
    /// [`Self::proof_absorb`] で、その呼び出しで差分が変わった証明の印 (`proof_chg_stamp` と同じなら印あり)。
    proof_chg_mark: Vec<u32>,
    proof_chg_stamp: u32,
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
    /// HiGHS 式の衝突解析 [呼び出し, 作った衝突, 入れた衝突, 長さの和]
    multi_stats: [u64; 4],
    multi_secs: f64,
    /// HiGHS 式のカット選択の状態 (これまでの最良のスコア、残す下限の係数)
    pub(super) cutsel_best: f64,
    pub(super) cutsel_factor: f64,
    /// 経路集約の始点の行の開始位置 (ラウンドごとにずらす)
    pub(super) path_offset: usize,
    pub(super) debug_x_done: bool,
    /// 目的関数の非零の係数 (目的関数の伝播用、最初に使うときに作る)
    obj_cols: Option<std::rc::Rc<Vec<(usize, f64)>>>,
    /// 目的関数の伝播 [刈ったノード, 締めた境界]、強分岐の子の伝播 [矛盾した子, 子の LP に渡した境界]
    obj_prop_stats: [u64; 2],
    /// カットプールの伝播 (`ENOMOTO_MIP_CUTPOOL_PROP`): 呼び出し回数、枝刈りしたノード数、締めた境界の数、時間 (ナノ秒)。
    cutpool_prop_stats: [u64; 4],
    /// 根のカットのループが HiGHS 流のカット管理を有効にしたか (`ENOMOTO_MIP_CUT_STRONG_AUTO`、[`Self::highs_cutmgmt`])。
    pub(super) hcm_auto: bool,
    /// [`Self::root_obj_probe`] の (試した列, 固定した列)
    root_obj_probe_stats: [u64; 2],
    sb_prop_stats: [u64; 2],
    /// 行にできない衝突 (連続列・一般整数列を含む) を境界リテラルのまま持つプール
    lit_conflicts: super::conflict_pool::LitConflictPool,
    /// [加えた数, 刈ったノード, 締めた境界]
    lit_stats: [u64; 3],
    /// リテラルの衝突で前回調べたときの定義域の記録の長さ
    lit_pos: usize,
    /// 診断用: 伝播の矛盾での衝突解析 [呼び出し, 1-UIP なし, 1-UIP を行にできない, 決定の組なし, 決定の組も行にできない]、
    /// 行にできなかった理由 [連続列, 一般整数 2 つ以上・範囲外, 同じ列の両側]
    conf_stats: [u64; 8],
    /// 作った実行不能の証明の数。
    farkas_added: u64,
    /// 双対証明で枝刈りしたノード数と、締めた境界の数 (表示用)。
    pub(super) proof_prunes: u64,
    /// 数値誤差の見積もりの範囲内でしか破れていないために捨てた証明の数 ([`Self::add_proof_from`])。
    pub(super) proof_numeric_rejects: u64,
    /// 診断用 (`ENOMOTO_MIP_PROOF_VERIFY`): 直前に [`Self::apply_dual_proofs`] がノードを刈ったときの証明の番号。
    proof_last_prune: Option<usize>,
    /// 診断用 (`ENOMOTO_MIP_PROOF_VERIFY`): 直前の刈り込みが、同じ呼び出しの中の証明による締め付け (整数の丸めを含む) の後に
    /// 起きたか (LP の値が打ち切り値未満でも正しい刈り込みでありうる)。
    proof_prune_after_tighten: bool,
    /// 診断用 (`ENOMOTO_MIP_PROOF_VERIFY`): 証明で刈ったノードの LP を解き直して確かめた数 [LP が打ち切り値以上か実行不能 (正しい),
    /// LP が打ち切り値未満 (証明が誤り)、確かめられなかった]。
    proof_verify_stats: [u64; 4],
    pub(super) proof_tightenings: u64,
    /// カットプールの重複判定用のハッシュと、プールの非零数。
    pub(super) cut_pool_keys: std::collections::HashMap<u64, usize>,
    pub(super) cut_pool_nnz: usize,
    /// プールの各カットのハッシュと年齢 (根のラウンド・プールからのラウンドごとに 1 増え、LP に入れたら 0 に戻す)。
    /// プールが上限に達したとき、年齢の大きいものから捨てる (`ENOMOTO_MIP_POOL_NO_EVICT` なら新しい候補を捨てる)。
    pub(super) pool_hash: Vec<u64>,
    pub(super) pool_age: Vec<u32>,
    /// カットプールの平坦な (CSR) 複製: 列番号・係数・各カットの開始位置 (長さ プール数 + 1)。違反の走査を 1 本の配列の
    /// 走査にする (`Vec<Vec<..>>` を辿る走査は mik-250 で 40 秒中 1.4-1.9 秒かかっていた)。`add_to_pool` が同期する。
    pub(super) pool_idx: Vec<u32>,
    pub(super) pool_val: Vec<f64>,
    pub(super) pool_start: Vec<u32>,
    /// 診断用: [`Self::add_to_pool`] で非零数の上限を見ない (`ENOMOTO_MIP_ORACLE_CUTS` を入れる間だけ真)
    pub(super) pool_nocap: bool,
    /// 求解の開始時 (根の伝播の後) に固定されていた整数列の数 (再スタートの判定用)。
    root_fixed0: usize,
    /// ノードでのダイビングの LP 反復数・呼び出し回数・解を見つけた回数。
    dive_iters: u64,
    dive_calls: u64,
    dive_succ: u64,
    /// 証明の段階の検出 (`ENOMOTO_MIP_PROOF_SWITCH`、[`Self::proof_phase_check`]): 木を分け合わないときの印、最初の
    /// 暫定解の目的値、最後に最良値が良くなった (と気づいた) 時刻とその値、前回調べた時刻。
    proof_phase: bool,
    pub(super) proof_first_z: f64,
    proof_last_best: (f64, Instant),
    proof_eval_at: Option<Instant>,
    /// カットの行のハッシュ ([`Self::refresh_row_hashes`]) と、作ったときの行の記録の位置。`cut_basis_stats` は共有の待ち行列に
    /// 出したノードに付けたカットの状態の数・受け取って当てはめた数 (`ENOMOTO_MIP_TREE_SHARE_CUT_BASIS`)。
    row_hash: Vec<u64>,
    row_hash_map: std::collections::HashMap<u64, usize>,
    row_hash_epoch: Option<usize>,
    cut_basis_stats: [u64; 2],
    /// 最後に待ち行列の半分を共有の待ち行列へ出した時刻 (`ENOMOTO_T_MIP_TREE_DONATE_HALF_SECS`)。
    donate_half_at: Option<Instant>,
}

/// 分枝限定法で解く。
/// 並列モードのスレッド数 (`ENOMOTO_MIP_THREADS`、既定 1 = 並列にしない)。
/// ポートフォリオ並列 (`ENOMOTO_MIP_PORTFOLIO`、[`super::solve_mip`]) のスレッド間で共有する値: これまでの最良の
/// 目的値 (最小化形、定数項込み。全てのスレッドで同じ空間) と、止める合図。
pub(crate) struct Portfolio {
    best: std::sync::atomic::AtomicU64,
    pub(crate) stop: std::sync::atomic::AtomicBool,
    /// 木の探索をしているスレッドの下界 (の最大)。目的値の二分探索のスレッドが目標値を決めるのに使う。
    lb: std::sync::atomic::AtomicU64,
    /// 目的値の二分探索のスレッドが示した厳密な下界 (この値未満の解はない。スレッドの間で共有する)。
    proven: std::sync::atomic::AtomicU64,
    /// 区間 (最良値・厳密な下界) が変わった回数。二分探索のスレッドはこれが変わったら目標値を決め直して解き直す。
    generation: std::sync::atomic::AtomicU64,
    /// 木の探索のスレッドが根で作ったカット (元の問題の空間) と、それが成り立つ範囲 (目的値がこの値未満の解)。
    /// 二分探索のスレッドが求解の最初に LP に入れる (`ENOMOTO_MIP_BISECT_SHARE_CUTS`)。
    cuts: std::sync::Mutex<Option<std::sync::Arc<(Vec<(Vec<(usize, f64)>, f64)>, f64)>>>,
    /// 木の写し (`ENOMOTO_MIP_BISECT_SNAPSHOT`): 二分探索のスレッドが写しを求める印、木の探索のスレッドが置いた最新の写し、
    /// 写しの通し番号。
    pub(crate) snap_req: std::sync::atomic::AtomicBool,
    snap: std::sync::Mutex<Option<std::sync::Arc<TreeSnapshot>>>,
    snap_serial: std::sync::atomic::AtomicU64,
    /// 写しのノードごとに二分探索のスレッドが示した値 (写しの通し番号 -> ノードごとの「部分木にこの値未満の解はない」)
    /// と、その更新の回数。
    fb: std::sync::Mutex<std::collections::HashMap<u64, Vec<f64>>>,
    fb_ver: std::sync::atomic::AtomicU64,
    /// 写しから解いた求解が見つけた解 (写しの通し番号, 写しの問題の空間の解)。写しを作った木の探索のスレッドが受け取る。
    inbox: std::sync::Mutex<Vec<(u64, Vec<f64>, &'static str, bool)>>,
    inbox_n: std::sync::atomic::AtomicU64,
}

/// 木の探索のスレッドの探索木の写し: 二分探索のスレッドが根を解き直さずに、未処理のノードから打ち切り値 t で探索を
/// 続けるのに使う。写しを作ったときの未処理のノードの全体が「打ち切り値 `valid_below` 未満の解が残りうる領域」の全部なので、
/// t <= `valid_below` なら、これらのノードを t で探索し終えれば「t 未満の解はない」。
pub(crate) struct TreeSnapshot {
    pub(crate) serial: u64,
    /// 木の探索のスレッドが解いている問題 (前処理・対称性の行を加えた後)。
    pub(crate) p: std::sync::Arc<MipProblem>,
    /// 大域的な境界、LP のカットの行 (係数, 下限, 上限)、カットプール (係数 x <= 右辺)、擬似費用。
    lo: Vec<f64>,
    up: Vec<f64>,
    rows: Vec<(Vec<(usize, f64)>, f64, f64)>,
    pool: Vec<(Vec<(usize, f64)>, f64)>,
    pc: Pseudocost,
    pub(crate) nodes: Vec<SnapNode>,
    pub(crate) valid_below: f64,
    /// 根で HiGHS 流のカット管理を有効にしたか (`ENOMOTO_MIP_CUT_STRONG_AUTO`)。
    hcm_auto: bool,
    /// 完全オービトープと、対称性の行・固定を加える前の問題 (写しから解く求解も orbitopal fixing をし、ヒューリスティクスの
    /// 解を並べ替えて使えるように)。
    orbitopes: Vec<super::symmetry::Orbitope>,
    orig: Option<std::sync::Arc<MipProblem>>,
    lex_gens: Vec<Vec<usize>>,
    /// 写しを作ったときの LP の基底 (`ENOMOTO_MIP_TREE_ROOT_BASIS`、試作。列と、元の行 + `rows` の行)
    root_basis: Option<super::lp::Basis>,
}

/// `ENOMOTO_MIP_TREE_ROOT_BASIS` (試作、inv/mzzv42z-r4-20261021、既定では使わない): 木を分け合う従が写しの根の LP を、主の LP の
/// 基底から解き始める (既定は基底なしで最初から解く)。mzzv42z (10800 行) では木に入った後の 20 秒で従 3 本が 1 ノードも
/// 処理しなかった (analysis/mzzv42z_r4_20261021.md。r4 の「従が根の LP で時間切れなら下界を +inf」は既定の
/// [`Solver::root_fail_bound`] に入っているので、inv/mzzv42z-20261023 でここに移したのは基底の受け渡しだけ)
fn tree_root_basis() -> bool {
    env_str!("ENOMOTO_MIP_TREE_ROOT_BASIS").is_some()
}

/// `ENOMOTO_MIP_RESTART_BASIS` (試作、既定では使わない): 再スタートの後の根の LP を、再スタートの直前の LP の基底から解き始める
/// ([`RESTART_BASIS`])。今までは再スタートの後の根の LP を毎回基底なしで解いていた (前処理の LP による固定は最初の求解だけで、
/// その基底も最初の根で使い切る)
pub(crate) fn restart_basis_on() -> bool {
    env_def!("ENOMOTO_MIP_RESTART_BASIS", "1").is_some()
}

/// `ENOMOTO_MIP_SB_AGREE` (試作、既定では使わない): 強分岐と擬似費用の選び方の一致を見て、十分に学べたら強分岐を省く
/// ([`Solver::sb_agree_learned`])。h80x6320d・roll3000 は強分岐の予算を増やすと木が小さくなる (`ENOMOTO_MIP_SB_BUDGET_HIGHS`) が、
/// 10teams・cvs16r128-89 などは予算を増やすと遅くなる。擬似費用で同じ列を選べる問題では強分岐を早く切り上げる
/// FJ のスレッドで FJ と Local-MIP + 連鎖の手に時間を分け合う ([`super::heuristics::fj_lmip_mix`])。既定で使う
/// (`ENOMOTO_MIP_NO_FJ_LMIP_MIX` で使わない)。`ENOMOTO_MIP_SYM_ORIG_CUT` ([`sym_orig_cut_on`]) と組で既定にした (mixoc)。
/// 40 問 + 16 問 (4 スレッド・60 秒、同じ機械で 2 組。ba7f18b・a261791・acaa6fa の 3 回の計測): 40 問の解けた数と
/// SGM 時間は揺れの範囲で並び、56 問の主ギャップの和は 1.39〜2.20 -> 0.21〜0.47。2 組とも同じ向きの差は、良い方が
/// neos-950242・30n20b8・cvs16r128-89・graph20-20-1rand など 4〜5 問、悪い方が timtab1 (a261791 の 1 回、偽の対称性の
/// 行が LP を重くしていた: analysis/timcmp_20261023.md) だけ (analysis/heur_ls_20261023/bench8_results.md)
pub(crate) fn fj_lmip_mix_on() -> bool {
    env_str!("ENOMOTO_MIP_NO_FJ_LMIP_MIX").is_none()
}

/// 対称性の行への並べ替えに失敗した解の値を、打ち切り値にだけ使う (`orig_best`)。既定で使う (`ENOMOTO_MIP_NO_SYM_ORIG_CUT` で
/// 使わない)。[`fj_lmip_mix_on`] と組で既定にした
pub(crate) fn sym_orig_cut_on() -> bool {
    env_str!("ENOMOTO_MIP_NO_SYM_ORIG_CUT").is_none()
}

fn sb_agree_on() -> bool {
    env_str!("ENOMOTO_MIP_SB_AGREE").is_some()
}

/// 行の中身を照らし合わせるための鍵: 列の番号の順に並べ、最初の係数の絶対値で割り、最初の係数が負なら符号を反転する
/// (反転したら真を返す。論理変数の下限・上限の状態を入れ替える)。係数は相対 1e-9 で丸める
fn row_key(r: &[(usize, f64)]) -> Option<(Vec<(usize, i64)>, bool)> {
    let mut r: Vec<(usize, f64)> = r.iter().copied().filter(|&(_, v)| v != 0.0).collect();
    if r.is_empty() {
        return None;
    }
    r.sort_by_key(|&(j, _)| j);
    let a = r[0].1;
    let flip = a < 0.0;
    let s = if flip { -1.0 / a.abs() } else { 1.0 / a.abs() };
    Some((r.iter().map(|&(j, v)| (j, (v * s * 1e9).round() as i64)).collect(), flip))
}

/// [`RESTART_BASIS`] を今の LP の行に当てはめた基底を作る。列の数が違えば `None`。行の中身が再スタートの直前の LP の
/// 行と同じなら (正規化して) その論理変数の状態を使い、見つからない行は論理変数を基底に入れる (基底変数の数の過不足は
/// `set_basis` の修復に任せる)。照らし合わせた行の数も返す
fn restart_basis_for<L: MipLp>(lp: &L, n: usize, saved: (Vec<super::lp::VarStatus>, Vec<(Vec<(usize, f64)>, super::lp::VarStatus)>)) -> Option<(super::lp::Basis, usize)> {
    use super::lp::VarStatus;
    let (col, rows) = saved;
    if col.len() != n {
        return None;
    }
    let swap = |st: VarStatus, flip: bool| match (st, flip) {
        (VarStatus::Lower, true) => VarStatus::Upper,
        (VarStatus::Upper, true) => VarStatus::Lower,
        (s, _) => s,
    };
    let mut map: std::collections::HashMap<Vec<(usize, i64)>, VarStatus> = std::collections::HashMap::new();
    for (r, st) in &rows {
        if let Some((k, flip)) = row_key(r) {
            map.insert(k, swap(*st, flip));
        }
    }
    let m = lp.num_rows();
    let mut matched = 0usize;
    let row: Vec<VarStatus> = (0..m)
        .map(|i| match row_key(&lp.row(i)).and_then(|(k, flip)| map.get(&k).map(|&st| swap(st, flip))) {
            Some(st) => {
                matched += 1;
                st
            }
            None => VarStatus::Basic,
        })
        .collect();
    Some((super::lp::Basis { col, row }, matched))
}

pub(crate) struct SnapNode {
    changes: Vec<BoundChange>,
    lower_bound: f64,
    estimate: f64,
    depth: usize,
    branch: Option<(usize, bool, f64, f64)>,
    /// 共有の待ち行列で渡すノードの親の LP の最適基底の、列と元の行の部分 (`ENOMOTO_MIP_TREE_NO_PASS_BASIS` でなければ)。カットの行は
    /// スレッドごとに違うので渡さない (受け取った側では基底にする)。
    basis: Option<(Vec<VarStatus>, Vec<VarStatus>)>,
    /// 親の LP の基底で非基底だったカットの行の (カットのハッシュ, 状態) (`ENOMOTO_MIP_TREE_SHARE_CUT_BASIS`)。受け取った側は同じ
    /// カットの行があればその状態にする ([`Solver::open_from_pool`])。
    cut_basis: Option<Vec<(u64, VarStatus)>>,
}

impl SnapNode {
    fn of(n: &OpenNode) -> Self {
        SnapNode { changes: n.changes.clone(), lower_bound: n.lower_bound, estimate: n.estimate, depth: n.depth, branch: n.branch, basis: None, cut_basis: None }
    }
    /// [`Self::of`] に、親の LP の基底の列と最初の `m` 行 (元の行) を付ける。
    fn of_with_basis(n: &OpenNode, m: usize) -> Self {
        let mut s = Self::of(n);
        if let Some(b) = &n.basis {
            if b.row.len() >= m {
                s.basis = Some((b.col.clone(), b.row[..m].to_vec()));
            }
        }
        s
    }
    fn into_open(self) -> OpenNode {
        OpenNode { changes: self.changes, lower_bound: self.lower_bound, estimate: self.estimate, depth: self.depth, basis: None, basis_epoch: 0, dse: None, branch: self.branch, prot: None, tags: None }
    }
}

/// 木の探索を n 本のスレッドで分け合うための共有の待ち行列 (`MipParams::threads` > 1)。根を解いたスレッド (主) が根の後の
/// 木の写しから残りのスレッド (従) を始め、各スレッドは自分の待ち行列で探索する。待ち行列が空になったスレッドはここから
/// ノードを取り、ほかのスレッドは待っているスレッドがいれば自分の待ち行列の下界の小さいノードをここへ渡す。
/// 全てのスレッドが待っていて、ここも空なら探索は終わり。
pub(crate) struct WorkPool {
    inner: std::sync::Mutex<PoolInner>,
    /// 止める合図 (主が終わるときに立てる)。
    stop: std::sync::atomic::AtomicBool,
    /// `stop` と同時に立てる合図 (`ENOMOTO_MIP_WORKER_HALT` なら従のスレッドの LP の反復が見る。[`crate::cancel::with_halt`])
    halt: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// 待っているスレッドの数と、ここにあるノードの数 (錠の外で読む目安。渡すかどうかの判断に使う)。
    idle: std::sync::atomic::AtomicUsize,
    len: std::sync::atomic::AtomicUsize,
    /// 解けなかったノードを捨てたスレッドがある (最適とは言えない)。
    unresolved: std::sync::atomic::AtomicBool,
    /// スレッドの間で共有する大域的な境界 (根の被約費用による固定・双対証明などの大域的な締め付けの和)。
    bounds: std::sync::Mutex<(Vec<f64>, Vec<f64>)>,
    /// スレッドの間で合わせた擬似費用 (各スレッドの観測の和)。
    pc: std::sync::Mutex<Pseudocost>,
    /// 各スレッドが作った双対証明・衝突の記録 (どのノードでも成り立つので、ほかのスレッドも取り込む)。
    learned: std::sync::Mutex<Learned>,
    /// スレッドの番号の払い出し (記録に自分の分を読み飛ばすために付ける)。
    next_tid: std::sync::atomic::AtomicU32,
    /// スレッドの数 (主を含む。木の中のヒューリスティクスを回すスレッドを決めるのに使う)。
    threads: usize,
    /// 各スレッド (`pool_tid`) の下界・待ち行列のノード数・処理したノード数 ([`Solver::note_current_lb`])。共有の
    /// 待ち行列のノードと合わせて全体の下界になる (利用者向けのログに使う)。
    slots: Vec<ThreadSlot>,
    /// racing ramp-up (`ENOMOTO_MIP_TREE_RACING`): 競争中か、勝者のスレッド番号 (決まるまで -1)。
    racing: std::sync::atomic::AtomicBool,
    race_winner: std::sync::atomic::AtomicI64,
    /// 診断用: 木の探索を始めた時刻と、終わったスレッドの [`TreeProf`]。
    t0: Instant,
    prof: std::sync::Mutex<Vec<TreeProf>>,
    /// ヒューリスティクス役 (`ENOMOTO_MIP_HEUR_ROLE`) のスレッドの数 (番号の大きい方から。0 なら役を分けない)。
    heur_k: usize,
    /// 全てのスレッドで最良の解 (目的値, 解)。ヒューリスティクス役のスレッドが近傍の中心に使う。`has_x` はそれがあるか。
    best_x: std::sync::Mutex<Option<(f64, std::sync::Arc<Vec<f64>>)>>,
    has_x: std::sync::atomic::AtomicBool,
    /// 木の探索のスレッドが最近解いたノードの LP 解 (ヒューリスティクス役のスレッドの RINS・DINS 用)。
    lp_x: std::sync::Mutex<Option<std::sync::Arc<Vec<f64>>>>,
    /// 木の探索が終わった (`PoolInner::done` を錠の外で読む目安)。
    fin: std::sync::atomic::AtomicBool,
    /// 今ヒューリスティクス役に就いているスレッドの数。
    in_role: std::sync::atomic::AtomicUsize,
    /// 大近傍探索の価値の指数移動平均 ([`MuStats`])。
    mu: std::sync::Mutex<MuStats>,
    /// 証明の段階に入った (`ENOMOTO_MIP_PROOF_SWITCH`、[`Solver::proof_phase_check`])。主が立て、各スレッドが読む。
    proof_phase: std::sync::atomic::AtomicBool,
}

/// 並列の強分岐 (`ENOMOTO_MIP_PAR_SB`) で別スレッドが解いた子の LP の結果。主が逐次と同じ順に当てはめる。
struct SbSide {
    st: LpStatus,
    /// 目的値 (定数項込み。反復上限で止まったときは途中の値)。
    obj: f64,
    iters: u64,
    /// 最適なら LP 解 (整数なら暫定解の候補)。
    x: Option<Vec<f64>>,
    /// 証明に使う行の重み (実行不能なら Farkas の射線、それ以外は双対)。
    y: Option<Vec<f64>>,
}

/// 双対証明・衝突制約のプールの行の状態。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ProofState {
    /// 自前で、まだほかのスレッドに知らせていない。刈り込み・締め付けに使われた回数を持つ (`ENOMOTO_T_MIP_TREE_SHARE_PROOF_USES`
    /// 回使われたら知らせる)。
    Own(u16),
    /// 自前で、知らせた (または知らせない) もの。
    OwnDone,
    /// ほかのスレッドから取り込んだもの。
    Imported,
    /// 捨てたもの (中身は空、右辺は +inf。先頭に来たら取り除く)。
    Dead,
}

/// 診断用 (`ENOMOTO_MIP_LBNODE_DIAG`): 取り出したノードの種類ごとの数・時間と、取り出したときに探索全体で下界最小だった数。
/// 種類: 0 = 潜りの子, 1 = 兄弟への戻り, 2 = 自分の待ち行列の下界最小, 3 = 自分の待ち行列の推定値で選んだもの (下界最小でない),
/// 4 = 共有の待ち行列から
#[derive(Default)]
struct LbNodeDiag {
    n: [u64; 5],
    at_min: [u64; 5],
    secs: [f64; 5],
    last: Option<(usize, Instant)>,
}

fn lbnode_diag() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| env_str!("ENOMOTO_MIP_LBNODE_DIAG").is_some())
}

/// 診断用: 木の探索のスレッドごとの時間・ノードの内訳 (`ENOMOTO_PROF_TREE_PAR` で終わりに表示する)。
#[derive(Default, Clone)]
pub(crate) struct TreeProf {
    tid: u32,
    /// 木の探索を始めた時刻 ([`WorkPool::t0`]) から、最初に共有の待ち行列からノードを取りに来るまで (従の準備)。
    ready_s: f64,
    /// 木の探索を始めた時刻から、このスレッドが終わるまで。
    end_s: f64,
    /// 共有の待ち行列でノードを待った時間と、待った回数 (すぐ取れなかった回数)。
    idle_s: f64,
    waits: u64,
    /// `idle_s` の内訳: 錠を取って調べていた時間 (錠の取り合い・ノードの走査) と、眠って待った時間。
    lock_s: f64,
    sleep_s: f64,
    /// 共有の待ち行列から取ったときの、待ち行列に残っていたノード数の合計 (平均の大きさを見る)。
    pool_len_sum: u64,
    /// 木の中の大近傍探索 (サブ MIP) に使った時間。
    lns_s: f64,
    /// 最後の待ち (探索の終わり・止める合図までノードが来なかった) の時間。
    tail_s: f64,
    /// スレッドの間の同期 (境界・学習・擬似費用・ノードを渡す) の時間。
    sync_s: f64,
    /// 処理したノードの数 (取り出した数)。
    nodes: u64,
    /// ノードの最初の LP を種類ごとに: 0 = 潜り・兄弟 (直前の LP の続き), 1 = 自分の待ち行列から, 2 = 共有の待ち行列から。
    kind_n: [u64; 3],
    kind_it: [u64; 3],
    kind_s: [f64; 3],
    /// 取り出したノードの下界 (最適値が分かってから刈れたかを数えるため)。
    lbs: Vec<f64>,
    /// 双対証明・衝突の効き目 ([自前, 取り込み] ごと)。証明: [調べた, ノードを刈った, 境界を締めた, 差分の更新 (費用の目安)]。
    /// 衝突は [`LitConflictPool::stats`] と同じ並び。終わりにプールに残っていた数 ([自前, 取り込み])。
    proof: [[u64; 4]; 2],
    lits: [[u64; 4]; 2],
    proof_live: [usize; 2],
    lits_live: [usize; 2],
    /// ほかのスレッドに知らせた自前の証明の数。
    shared: u64,
    /// 質の釣り合いで [取ったノード, 渡したノード]。
    balance: [u64; 2],
    /// 主: 求解の始めから、最初のノードを取り出したとき・従を始めたときまで。
    tree_start_s: f64,
    workers_s: f64,
}

impl TreeProf {
    fn proofs_line(&self) -> String {
        let p = &self.proof;
        let l = &self.lits;
        format!(
            "balance {}/{} shared {} proofs own/imp: live {}/{} eval {}/{} prune {}/{} tight {}/{} absorb {}/{}; lits own/imp: live {}/{} eval {}/{} prune {}/{} tight {}/{} aged {}/{}",
            self.balance[0], self.balance[1], self.shared, self.proof_live[0], self.proof_live[1], p[0][0], p[1][0], p[0][1], p[1][1], p[0][2], p[1][2], p[0][3], p[1][3],
            self.lits_live[0], self.lits_live[1], l[0][0], l[1][0], l[0][1], l[1][1], l[0][2], l[1][2], l[0][3], l[1][3]
        )
    }

    fn line(&self, zopt: Option<f64>) -> String {
        let waste = zopt.map_or(0, |z| self.lbs.iter().filter(|&&l| l >= z - 1e-9 * z.abs().max(1.0)).count());
        let per = |k: usize| if self.kind_n[k] > 0 { self.kind_it[k] as f64 / self.kind_n[k] as f64 } else { 0.0 };
        let pms = |k: usize| if self.kind_n[k] > 0 { 1e3 * self.kind_s[k] / self.kind_n[k] as f64 } else { 0.0 };
        format!(
            "{} | tid {} ready {:.2}s end {:.2}s idle {:.2}s ({} waits, lock {:.2}s sleep {:.2}s, pool mean {:.1}) lns {:.2}s tail {:.2}s sync {:.3}s nodes {} (lb>=opt {}) first LP [dive {} {:.1}it {:.2}ms | queue {} {:.1}it {:.2}ms | pool {} {:.1}it {:.2}ms]",
            self.proofs_line(),
            self.tid, self.ready_s, self.end_s, self.idle_s, self.waits, self.lock_s, self.sleep_s, self.pool_len_sum as f64 / self.kind_n[2].max(1) as f64, self.lns_s, self.tail_s, self.sync_s, self.nodes, waste,
            self.kind_n[0], per(0), pms(0), self.kind_n[1], per(1), pms(1), self.kind_n[2], per(2), pms(2)
        )
    }
}

/// `ENOMOTO_MIP_TREE_POOL_BASIS` (試作): 共有の待ち行列で渡すノードに非基底だったカットの行の状態も付け
/// (`ENOMOTO_MIP_TREE_SHARE_CUT_BASIS` と同じ)、受け取った側で基底変数が余るなら今の LP の基底のまま解く ([`Solver::open_from_pool`])。
fn tree_pool_basis() -> bool {
    env_str!("ENOMOTO_MIP_TREE_POOL_BASIS").is_some()
}

fn prof_tree_par() -> bool {
    env_str!("ENOMOTO_PROF_TREE_PAR").is_some()
}

/// 診断用 (`ENOMOTO_MIP_TREE_DIAG`): 木を分け合うスレッドの受け渡しと、optimal で終わるときの突き合わせを表示する
/// (analysis/partree_race_20261021.md)。
fn proof_diag() -> bool {
    env_str!("ENOMOTO_MIP_PROOF_DIAG").is_some()
}

/// 診断用 (`ENOMOTO_MIP_PROOF_VERIFY`): 双対証明・衝突制約がノードを刈るたびに、そのノードの LP を (証明による締め付けを戻して)
/// 解き直し、LP の値が打ち切り値未満なら証明が誤り (または LP から外れたカットに頼った証明) として表示する
/// (analysis/partree_race_20261021.md §9)。デバッグ解がなくても、どのスレッドでも働く。
fn proof_verify() -> bool {
    env_str!("ENOMOTO_MIP_PROOF_VERIFY").is_some()
}

fn tree_diag() -> bool {
    env_str!("ENOMOTO_MIP_TREE_DIAG").is_some()
}

/// `ENOMOTO_MIP_WORKER_HALT`: 主の止める合図を木の探索の従のスレッドの LP の反復にも伝える ([`crate::cancel::with_halt`])
fn worker_halt() -> bool {
    env_def!("ENOMOTO_MIP_WORKER_HALT", "1").is_some()
}

/// 診断用 `ENOMOTO_MIP_END_DIAG`: 終わり際の時刻 (従を待った時間、`finish` の始めと終わり) を `ENOMOTO_MIP_LOG` なしでも出す
pub(super) fn end_diag() -> bool {
    env_str!("ENOMOTO_MIP_END_DIAG").is_some()
}

/// スレッドの間で共有する学習の記録 (通し番号つき、古いものから捨てる)。
#[derive(Default)]
struct Learned {
    /// 先頭の記録の通し番号。
    first: u64,
    /// (作ったスレッド, 証明 (係数, 定数, 目的関数の証明か) か、衝突 (リテラルの組))
    items: std::collections::VecDeque<(u32, LearnedItem)>,
}

pub(super) enum LearnedItem {
    Proof(Vec<(usize, f64)>, f64, bool),
    Lits(Vec<(usize, bool, f64)>),
    /// カットプールのカット (係数 x <= 右辺)。`ENOMOTO_MIP_TREE_SHARE_CUTS`。
    Cut(Vec<(usize, f64)>, f64),
}

/// 木を分け合うスレッドごとの、利用者向けのログに使う値 (ノードごとに書くので錠を取らず、キャッシュラインを分ける)。
#[repr(align(128))]
#[derive(Default)]
struct ThreadSlot {
    /// 処理中のノードと自分の待ち行列の下界の最小 (f64 のビット)。ノードを持っていなければ +inf。
    lb: std::sync::atomic::AtomicU64,
    /// 自分の待ち行列のノード数。
    open: std::sync::atomic::AtomicUsize,
    /// 処理したノード数 (従のみ)。
    nodes: std::sync::atomic::AtomicU64,
    /// ヒューリスティクス役の起動の判定 (`ENOMOTO_MIP_HEUR_ROLE_TRIGGER`) 用: 自分の未処理のノードの重み (深さ d のノードは
    /// 2^-d) の和と、そのうち下界が「最良値 - δ (最良値 - 全体の下界)」以上のものの和 (f64 のビット。64 ノードおきに書く)。
    wopen: std::sync::atomic::AtomicU64,
    /// 下界が「最良値 - δ_i (最良値 - 全体の下界)」以上のノードの重みの和 (δ_i は [`TRIG_DELTAS`])。
    wcuts: [std::sync::atomic::AtomicU64; 8],
}

/// 刈れる重みの分布 ([`ThreadSlot::wcuts`]) の境の位置 (ギャップに対する割合)。
const TRIG_DELTAS: [f64; 8] = [0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0];

/// 刈れる重みの分布から、暫定解がギャップの `rel` だけ良くなったときに刈れる重みを線形補間で求める (`rel` <= 0 なら 0)。
fn cut_weight(wcuts: &[f64; 8], rel: f64) -> f64 {
    if rel <= 0.0 {
        return 0.0;
    }
    let (mut x0, mut y0) = (0.0, 0.0);
    for (i, &d) in TRIG_DELTAS.iter().enumerate() {
        if rel <= d {
            return y0 + (wcuts[i] - y0) * (rel - x0) / (d - x0);
        }
        (x0, y0) = (d, wcuts[i]);
    }
    y0
}

/// 大近傍探索の価値の指数移動平均 (`ENOMOTO_MIP_HEUR_ROLE_TRIGGER=mu`、[`Solver::mu_observe`])。全てのスレッドで共有する。
#[derive(Default, Clone, Copy, Debug)]
struct MuStats {
    /// 1 秒あたりに刈れる木の重み (証明への寄与) と、1 秒あたりに縮めたギャップの割合 (主解への寄与)。観測がなければ `None`。
    proof: Option<f64>,
    primal: Option<f64>,
    n: u64,
    /// 主解への寄与のうち、木の探索を始めてからの観測だけの平均と、その数 (根の実績は大きく出すぎるので、証明寄りの判定
    /// (`ENOMOTO_MIP_FOCUS` が primal 以外) ではこちらを使う)。
    primal_tree: Option<f64>,
    n_tree: u64,
    /// 主解への寄与のうち、ヒューリスティクス役のスレッドの観測だけの平均と、その数 (`ENOMOTO_MIP_HEUR_MU_ROLE_OBS`、[`Solver::mu_worth`])。
    primal_role: Option<f64>,
    n_role: u64,
}

impl MuStats {
    fn add(&mut self, proof: Option<f64>, primal: f64) {
        let l = tunable!("ENOMOTO_T_MIP_HEUR_MU_LAMBDA", 0.8, f64);
        if let Some(sp) = proof {
            self.proof = Some(self.proof.map_or(sp, |v| l * v + (1.0 - l) * sp));
        }
        self.primal = Some(self.primal.map_or(primal, |v| l * v + (1.0 - l) * primal));
        self.n += 1;
        // 根の実績 (証明への寄与なし) は木の探索の観測に数えない
        if proof.is_some() {
            self.primal_tree = Some(self.primal_tree.map_or(primal, |v| l * v + (1.0 - l) * primal));
            self.n_tree += 1;
        }
    }

    /// ヒューリスティクス役のスレッドの観測 ([`Self::add`] に加えて数える)。
    fn add_role(&mut self, primal: f64) {
        let l = tunable!("ENOMOTO_T_MIP_HEUR_MU_LAMBDA", 0.8, f64);
        self.primal_role = Some(self.primal_role.map_or(primal, |v| l * v + (1.0 - l) * primal));
        self.n_role += 1;
    }
}

struct PoolInner {
    nodes: PoolNodes,
    /// 待っていない (ノードを持っているか処理している) スレッドの数。
    busy: usize,
    done: bool,
}

/// 共有の待ち行列のノード。自分の待ち行列 ([`super::queue::NodeQueue`]) と同じく、下界の順と「下界と推定値の平均」の順の
/// 2 つの順序付き集合で索引する (以前は並べずに置き、下界最小のノードを毎回全体の走査で探していた。pk1 では共有の待ち行列が
/// 3〜9 万ノードになり、走査に錠の中でスレッドあたり 3〜8 秒使っていた。analysis/pk1_variance_20261017.md)。
struct PoolNodes {
    v: Vec<Option<SnapNode>>,
    free: Vec<usize>,
    by_lb: std::collections::BTreeSet<(super::queue::Key, usize)>,
    by_est: std::collections::BTreeSet<(super::queue::Key, super::queue::Key, usize)>,
    pops: u64,
}

impl PoolNodes {
    fn new() -> Self {
        PoolNodes { v: Vec::new(), free: Vec::new(), by_lb: std::collections::BTreeSet::new(), by_est: std::collections::BTreeSet::new(), pops: 0 }
    }
    fn len(&self) -> usize {
        self.by_lb.len()
    }
    fn is_empty(&self) -> bool {
        self.by_lb.is_empty()
    }
    fn iter(&self) -> impl Iterator<Item = &SnapNode> {
        self.v.iter().flatten()
    }
    fn est_key(n: &SnapNode) -> (super::queue::Key, super::queue::Key) {
        (super::queue::Key(0.5 * n.lower_bound + 0.5 * n.estimate), super::queue::Key(-(n.depth as f64)))
    }
    fn push(&mut self, n: SnapNode) {
        let lb = super::queue::Key(n.lower_bound);
        let (e, d) = Self::est_key(&n);
        let id = match self.free.pop() {
            Some(id) => {
                self.v[id] = Some(n);
                id
            }
            None => {
                self.v.push(Some(n));
                self.v.len() - 1
            }
        };
        self.by_lb.insert((lb, id));
        self.by_est.insert((e, d, id));
    }
    fn extend(&mut self, it: impl IntoIterator<Item = SnapNode>) {
        for n in it {
            self.push(n);
        }
    }
    fn remove(&mut self, id: usize) -> SnapNode {
        let n = self.v[id].take().unwrap();
        let (e, d) = Self::est_key(&n);
        self.by_lb.remove(&(super::queue::Key(n.lower_bound), id));
        self.by_est.remove(&(e, d, id));
        self.free.push(id);
        n
    }
    fn min_lower_bound(&self) -> f64 {
        self.by_lb.iter().next().map_or(f64::INFINITY, |&(k, _)| k.0)
    }
    /// 下界最小のノードを取り出す。
    fn pop_best(&mut self) -> Option<SnapNode> {
        let id = self.by_lb.iter().next().map(|&(_, id)| id)?;
        Some(self.remove(id))
    }
    /// 自分の待ち行列と同じ選び方で取り出す: `bb_every` 回に 1 回は下界最小、それ以外は「下界と推定値の平均」が最小。
    fn pop_hybrid(&mut self, bb_every: u64) -> Option<SnapNode> {
        self.pops += 1;
        let id = if self.pops % bb_every.max(1) == 0 { self.by_lb.iter().next().map(|&(_, id)| id) } else { self.by_est.iter().next().map(|&(_, _, id)| id) }?;
        Some(self.remove(id))
    }
}

/// 共有の待ち行列だけで探索するか (`ENOMOTO_MIP_TREE_SHARED_QUEUE`): 各スレッドは潜りの子だけを自分で持ち、兄弟などのノードは
/// 全て共有の待ち行列に出して、取るときは自分の待ち行列と同じ選び方 ([`PoolNodes::pop_hybrid`]) をする。
fn tree_shared_queue() -> bool {
    env_str!("ENOMOTO_MIP_TREE_SHARED_QUEUE").is_some()
}

/// 今の潜りの兄弟だけを手元に残すか (`ENOMOTO_MIP_TREE_KEEP_SIBLINGS`): 潜りが終わったときに親の基底・LU のまま安く戻れる兄弟
/// (`dive_stack`) だけを自分の待ち行列に残し、ほかのノードは全て共有の待ち行列に出す。取るときは自分の待ち行列と同じ選び方。
fn tree_keep_siblings() -> bool {
    env_str!("ENOMOTO_MIP_TREE_KEEP_SIBLINGS").is_some()
}

/// 木を分け合うスレッドの間で、カットプールに入れたカットを共有するか (`ENOMOTO_MIP_TREE_SHARE_CUTS`)。カットは大域的な
/// 境界で作るので、どのスレッドのどのノードでも成り立つ (被約費用による固定で締めた境界のものは、共有の打ち切り値未満の解で)。
pub(super) fn tree_share_cuts() -> bool {
    env_str!("ENOMOTO_MIP_TREE_SHARE_CUTS").is_some()
}

/// 学習の記録に加える (古いものから捨てて `ENOMOTO_T_MIP_TREE_LEARNED_MAX` 件まで残す)。
pub(super) fn learned_push(pool: &WorkPool, tid: u32, it: LearnedItem) {
    let max = tunable!("ENOMOTO_T_MIP_TREE_LEARNED_MAX", 20000usize, usize);
    let mut g = pool.learned.lock().unwrap();
    g.items.push_back((tid, it));
    while g.items.len() > max {
        g.items.pop_front();
        g.first += 1;
    }
}

/// このスレッドを待っていない側から外す (終わるスレッドが呼ぶ)。
fn pool_release(pool: &WorkPool) {
    let mut g = pool.inner.lock().unwrap();
    g.busy = g.busy.saturating_sub(1);
    if g.busy == 0 && g.nodes.is_empty() {
        g.done = true;
        pool.fin.store(true, std::sync::atomic::Ordering::Release);
    }
}

/// 双対証明に整数列を残すか (既定。`ENOMOTO_MIP_NO_PROOF_KEEP_INT` で以前どおり整数列も右辺に移す)。
/// neos-2978193-inde の木を HiGHS 並みにした組 (mink) の 1 つ (analysis/tree_replay_20261022.md)
fn proof_keep_int() -> bool {
    env_str!("ENOMOTO_MIP_NO_PROOF_KEEP_INT").is_none()
}

fn prop_fixpoint() -> bool {
    env_str!("ENOMOTO_MIP_PROP_FIXPOINT").is_some()
}

/// ノードでカットプールを伝播するか (`ENOMOTO_MIP_CUTPOOL_PROP`、試作。サブ MIP でも使う)。
fn cutpool_prop() -> bool {
    env_str!("ENOMOTO_MIP_CUTPOOL_PROP").is_some()
}

/// ヒューリスティクス役で ALNS と Local-MIP + 連鎖を競わせるか (`ENOMOTO_MIP_ROLE_LMIP`、試作)。
/// 根のヒューリスティクスの終わりに、暫定解を出発点に Local-MIP + 連鎖の手 ([`super::localmip`]) を回すか
/// (`ENOMOTO_MIP_ROOT_LMIP`、試作、既定では使わない)。FJ のスレッドの Local-MIP (`fj_lmip_mix`) は FJ 自身の解しか受け取らず、
/// 根のカットのループの終わりで止まるので、RENS・RINS などのサブ MIP の解からは磨かない
fn root_lmip_on() -> bool {
    env_str!("ENOMOTO_MIP_ROOT_LMIP").is_some()
}

fn role_lmip_on() -> bool {
    env_str!("ENOMOTO_MIP_ROLE_LMIP").is_some()
}

fn heur_role_verbose() -> bool {
    env_str!("ENOMOTO_MIP_HEUR_ROLE_VERBOSE").is_some()
}

/// ヒューリスティクス役を起動の判定で決めるか (既定。`ENOMOTO_MIP_HEUR_ROLE_TRIGGER=off` なら以前の形で、解ができたら就く)。
/// 証明の段階 (`ENOMOTO_MIP_PROOF_SWITCH`) でヒューリスティクス役も止めるか (`ENOMOTO_MIP_PROOF_KEEP_ROLE` なら止めない)。
fn proof_phase_stops_role() -> bool {
    env_str!("ENOMOTO_MIP_PROOF_KEEP_ROLE").is_none()
}

fn heur_trigger_mode() -> bool {
    env_str!("ENOMOTO_MIP_HEUR_ROLE_TRIGGER").is_none_or(|v| v != "off")
}

/// 起動の判定を、大近傍探索の限界効用の比較で行うか (既定、[`Solver::mu_worth`])。`ENOMOTO_MIP_HEUR_ROLE_TRIGGER=tree` なら
/// 以前の形 (木の完了の予測と刈れる割合のしきい値、[`TriggerEval`]) で判定する。
fn heur_mu_mode() -> bool {
    env_str!("ENOMOTO_MIP_HEUR_ROLE_TRIGGER").is_none_or(|v| v == "mu")
}

/// ヒューリスティクス役の起動の判定の材料 ([`Solver::heur_trigger_eval`])。
struct TriggerEval {
    /// 木の完了率 (1 - 未処理のノードの重みの和)、完了までの予測の秒数、残りの秒数、未処理のノードの重みのうち暫定解が
    /// δ (ギャップに対する割合) 良くなれば刈れるものの割合、判定してよいだけの時間・ノード数が経ったか。
    done: f64,
    est: f64,
    left: f64,
    frac: f64,
    warm: bool,
    /// 未処理のノードの重みの和、刈れる重みの分布、ギャップ (最良値 - 全体の下界)、木の完了率の進む速さ (毎秒)。
    w: f64,
    wcuts: [f64; 8],
    gap: f64,
    v: f64,
}

impl TriggerEval {
    fn line(&self) -> String {
        format!("tree done {:.4}, predicted {:.1}s vs left {:.1}s, cut fraction {:.3}", self.done, self.est, self.left, self.frac)
    }
}

/// 根の解発見スレッド (既定で使う。`ENOMOTO_MIP_NO_FIND_THREAD` で使わない): 根で暫定解が質の悪い FJ の解だけ ([`Solver::fj_poor`]) のとき、
/// 根の写しから作った求解を別スレッドで動かし、暫定解なし用のヒューリスティクス (局所探索・fractional / vector length / coefficient
/// diving・Feasibility Pump) を順に回す (主スレッドは暫定解があるのでこれらを回さない)。背景のカットの分離のスレッドを 1 本減らして
/// 空ける。時間の予算は残りの `ENOMOTO_T_MIP_FIND_THREAD_FRAC` (既定 0.3) 倍、最大 `ENOMOTO_T_MIP_FIND_THREAD_MAX` (既定 20) 秒。
/// 見つけた解は根のヒューリスティクスの後と木の各ノードで受け取る (待たない)。FJ2 (d1bfefd) の後、neos-950242 の根の局所探索
/// (4 = 最適値) と nursesched-sprint02 のダイビングが FJ の解のせいで回らなくなった
/// 主の暫定解が質の悪い FJ の解でなくなったら止める ([`Solver::poll_find_thread`])。40 問 + 16 問 (4 スレッド・60 秒、同じコンテナで
/// 既定と続けて測る対を順番を入れ替えて 2 組): 40 問は解けた数 25/25 -> 25/25・SGM 18.53/20.41 -> 18.16/20.59 秒・不一致 0、
/// 16 問は主ギャップの和 2.87/2.87 -> 2.41/2.41 (neos-950242 が 2 組とも 19 -> 6)
pub(super) fn find_thread_on() -> bool {
    env_str!("ENOMOTO_MIP_NO_FIND_THREAD").is_none()
}

/// 質の悪い FJ の解だけのときの暫定解なし用の RENS (既定で使う。`ENOMOTO_MIP_NO_FJ_POOR_BG_RENS` で使わない): 根で暫定解が Feasibility Jump の解だけで、根の LP から遠い
/// ([`Solver::fj_poor`]、相対ギャップ > `ENOMOTO_T_MIP_FJ_POOR_GAP`) とき、暫定解なし用のダイビング式 RENS を回し
/// (サブ MIP は根のヒューリスティクスの間の別スレッド、[`Solver::parallel_submips`])、背景のカットの分離のスレッドを 1 本減らす。
/// `ENOMOTO_MIP_FJ_POOR_NOINC` と違い、Pump・ダイビングなど主スレッドで回す暫定解なし用のヒューリスティクスは回さない
/// (根の時間を延ばさない)。FJ2 (d1bfefd) の後、hypothyroid-k1 では FJ の -3 が暫定解になり、RENS が -2851 (最適値) を出さなくなった。
/// 40 問 + 16 問 (4 スレッド・60 秒、同じコンテナの対 2 組): 40 問は解けた数 24/25 -> 25/24・SGM 21.52/19.25 -> 20.62/20.52 秒・不一致 0、
/// 16 問は解けた数 2/5 -> 4/4・主ギャップの和 4.71/3.02 -> 2.87/2.95 (hypothyroid-k1 は 2 組とも -2851、nursesched-sprint02 28.3 秒)
pub(super) fn fj_poor_bg_rens() -> bool {
    env_str!("ENOMOTO_MIP_NO_FJ_POOR_BG_RENS").is_none()
}

/// ヒューリスティクス役のスレッドの数 (既定で使う。`ENOMOTO_MIP_NO_HEUR_ROLE` で使わない)。スレッド数が
/// `ENOMOTO_T_MIP_HEUR_ROLE_MIN_THREADS` (既定 4) 以上なら、スレッド数 × `ENOMOTO_T_MIP_HEUR_ROLE_FRAC` (既定 0.25) の切り捨て、
/// 最低 1。主の分は残す。
/// MIPLIB 40 問 (4 スレッド、60 秒): 使わない 3 回 SGM 22.60/22.95/23.13 秒・解けた 24/23/23 問・主ギャップの和 3.08/3.07/3.09、
/// 使う (限界効用・証明の寄与で判定) 22.37 秒・24 問・2.45 (bppc4-08 0.117 -> 0.019、blp-ar98 0.126 -> 0.023、cvs16r128-89
/// 0.48 -> 0.10、qiu・noswot は時間内に解ける)
fn heur_role_count(threads: usize) -> usize {
    if env_str!("ENOMOTO_MIP_NO_HEUR_ROLE").is_some() || threads < tunable!("ENOMOTO_T_MIP_HEUR_ROLE_MIN_THREADS", 4usize, usize).max(2) {
        return 0;
    }
    let k = ((threads as f64 * tunable!("ENOMOTO_T_MIP_HEUR_ROLE_FRAC", 0.25, f64)).floor() as usize).clamp(1, threads - 1);
    // `ENOMOTO_MIP_TREE_PHYS_CAP` (試作): ノードを処理する木のスレッドを物理コア数までにし、残り (スレッド数 - 物理コア数) は
    // ヒューリスティクス役 (大近傍探索・Local-MIP) に回す。8 物理コア・16 論理の機械で 16 スレッドを指定すると、木の 16 本が
    // 1 コアに 2 本ずつ載り、1 本あたりが遅くなる (beasleyC3・mc11 は 8 スレッドが最速のことが多い)
    if env_str!("ENOMOTO_MIP_TREE_PHYS_CAP").is_some() {
        let phys = tunable!("ENOMOTO_T_MIP_PHYS_CORES", num_cpus::get_physical(), usize).max(1);
        return k.max(threads.saturating_sub(phys)).clamp(1, threads - 1);
    }
    k
}

impl WorkPool {
    fn new(threads: usize, lo: Vec<f64>, up: Vec<f64>, pc: Pseudocost) -> Self {
        WorkPool {
            learned: std::sync::Mutex::new(Learned::default()),
            next_tid: std::sync::atomic::AtomicU32::new(0),
            threads,
            racing: std::sync::atomic::AtomicBool::new(false),
            race_winner: std::sync::atomic::AtomicI64::new(-1),
            t0: Instant::now(),
            prof: std::sync::Mutex::new(Vec::new()),
            heur_k: heur_role_count(threads),
            best_x: std::sync::Mutex::new(None),
            has_x: std::sync::atomic::AtomicBool::new(false),
            lp_x: std::sync::Mutex::new(None),
            fin: std::sync::atomic::AtomicBool::new(false),
            in_role: std::sync::atomic::AtomicUsize::new(0),
            mu: std::sync::Mutex::new(MuStats::default()),
            proof_phase: std::sync::atomic::AtomicBool::new(false),
            pc: std::sync::Mutex::new(pc),
            bounds: std::sync::Mutex::new((lo, up)),
            inner: std::sync::Mutex::new(PoolInner { nodes: PoolNodes::new(), busy: threads, done: false }),
            slots: (0..threads).map(|_| ThreadSlot { lb: std::sync::atomic::AtomicU64::new(f64::INFINITY.to_bits()), ..Default::default() }).collect(),
            stop: std::sync::atomic::AtomicBool::new(false),
            halt: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            idle: std::sync::atomic::AtomicUsize::new(0),
            len: std::sync::atomic::AtomicUsize::new(0),
            unresolved: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

impl Portfolio {
    pub(crate) fn new() -> Self {
        Portfolio {
            best: std::sync::atomic::AtomicU64::new(f64::INFINITY.to_bits()),
            stop: std::sync::atomic::AtomicBool::new(false),
            lb: std::sync::atomic::AtomicU64::new(f64::NEG_INFINITY.to_bits()),
            proven: std::sync::atomic::AtomicU64::new(f64::NEG_INFINITY.to_bits()),
            generation: std::sync::atomic::AtomicU64::new(0),
            cuts: std::sync::Mutex::new(None),
            snap_req: std::sync::atomic::AtomicBool::new(false),
            snap: std::sync::Mutex::new(None),
            snap_serial: std::sync::atomic::AtomicU64::new(0),
            fb: std::sync::Mutex::new(std::collections::HashMap::new()),
            fb_ver: std::sync::atomic::AtomicU64::new(0),
            inbox: std::sync::Mutex::new(Vec::new()),
            inbox_n: std::sync::atomic::AtomicU64::new(0),
        }
    }
    /// 最新の木の写し。
    pub(crate) fn snapshot(&self) -> Option<std::sync::Arc<TreeSnapshot>> {
        self.snap.lock().unwrap().clone()
    }
    pub(crate) fn cuts(&self) -> Option<std::sync::Arc<(Vec<(Vec<(usize, f64)>, f64)>, f64)>> {
        self.cuts.lock().unwrap().clone()
    }
    fn set_cuts(&self, c: Vec<(Vec<(usize, f64)>, f64)>, valid_below: f64) {
        *self.cuts.lock().unwrap() = Some(std::sync::Arc::new((c, valid_below)));
    }
    pub(crate) fn generation(&self) -> u64 {
        self.generation.load(std::sync::atomic::Ordering::Relaxed)
    }
    pub(crate) fn proven(&self) -> f64 {
        f64::from_bits(self.proven.load(std::sync::atomic::Ordering::Relaxed))
    }
    /// 厳密な下界 `v` を知らせる (今の値より大きければ置き換える)。
    pub(crate) fn publish_proven(&self, v: f64) {
        use std::sync::atomic::Ordering;
        let mut cur = self.proven.load(Ordering::Relaxed);
        while v > f64::from_bits(cur) {
            match self.proven.compare_exchange_weak(cur, v.to_bits(), Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => {
                    self.generation.fetch_add(1, Ordering::Relaxed);
                    break;
                }
                Err(c) => cur = c,
            }
        }
    }
    pub(crate) fn lb(&self) -> f64 {
        f64::from_bits(self.lb.load(std::sync::atomic::Ordering::Relaxed))
    }
    /// 下界 `v` を知らせる (今の値より大きければ置き換える)。
    pub(crate) fn publish_lb(&self, v: f64) {
        use std::sync::atomic::Ordering;
        let mut cur = self.lb.load(Ordering::Relaxed);
        while v > f64::from_bits(cur) {
            match self.lb.compare_exchange_weak(cur, v.to_bits(), Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(c) => cur = c,
            }
        }
    }
    pub(crate) fn best(&self) -> f64 {
        f64::from_bits(self.best.load(std::sync::atomic::Ordering::Acquire))
    }
    /// 目的値 `z` の解を見つけたことを知らせる (今の最良より良ければ置き換える)。
    pub(crate) fn publish(&self, z: f64) {
        use std::sync::atomic::Ordering;
        let mut cur = self.best.load(Ordering::Relaxed);
        while z < f64::from_bits(cur) {
            match self.best.compare_exchange_weak(cur, z.to_bits(), Ordering::AcqRel, Ordering::Relaxed) {
                Ok(_) => {
                    self.generation.fetch_add(1, Ordering::Relaxed);
                    break;
                }
                Err(c) => cur = c,
            }
        }
    }
}

/// スレッドごとのポートフォリオの共有値 (スレッド局所の領域を増やさないよう、スレッドの番号で引く表に持つ)。
/// 3 つ目は二分探索のスレッドの見張り (打ち切る最良値, 自分の目標値): 共有の最良値が前者を下回るか、厳密な下界が
/// 自分の目標値に届いたら求解を打ち切る。
/// 4 つ目は、このスレッドの一番外の求解の前処理の尺度 (`x = d x'` の d。カットを元の問題の空間に戻すのに使う)。
type PortfolioEntry = (std::thread::ThreadId, std::sync::Arc<Portfolio>, Option<(f64, f64)>, Option<std::sync::Arc<Vec<f64>>>);
static PORTFOLIOS: std::sync::Mutex<Vec<PortfolioEntry>> = std::sync::Mutex::new(Vec::new());

pub(crate) fn set_thread_portfolio(p: Option<std::sync::Arc<Portfolio>>) {
    let id = std::thread::current().id();
    let mut t = PORTFOLIOS.lock().unwrap();
    t.retain(|(i, _, _, _)| *i != id);
    if let Some(p) = p {
        t.push((id, p, None, None));
    }
}

/// このスレッドの求解の見張りを設定する (`None` で見張らない)。
pub(crate) fn set_thread_watch(g: Option<(f64, f64)>) {
    let id = std::thread::current().id();
    if let Some(e) = PORTFOLIOS.lock().unwrap().iter_mut().find(|(i, _, _, _)| *i == id) {
        e.2 = g;
    }
}

/// このスレッドの一番外の求解の前処理の尺度を設定する (ポートフォリオのスレッドでなければ何もしない)。
pub(crate) fn set_thread_scale(d: Option<Vec<f64>>) {
    let id = std::thread::current().id();
    if let Some(e) = PORTFOLIOS.lock().unwrap().iter_mut().find(|(i, _, _, _)| *i == id) {
        e.3 = d.map(std::sync::Arc::new);
    }
}

fn thread_scale() -> Option<std::sync::Arc<Vec<f64>>> {
    let id = std::thread::current().id();
    PORTFOLIOS.lock().unwrap().iter().find(|(i, _, _, _)| *i == id).and_then(|e| e.3.clone())
}

fn thread_portfolio() -> Option<std::sync::Arc<Portfolio>> {
    let id = std::thread::current().id();
    PORTFOLIOS.lock().unwrap().iter().find(|(i, _, _, _)| *i == id).map(|(_, p, _, _)| p.clone())
}

fn thread_watch() -> Option<(f64, f64)> {
    let id = std::thread::current().id();
    PORTFOLIOS.lock().unwrap().iter().find(|(i, _, _, _)| *i == id).and_then(|e| e.2)
}

pub(crate) fn mip_threads() -> usize {
    let env = tunable!("ENOMOTO_MIP_THREADS", 0usize, usize);
    if env > 0 { env } else { MIP_THREADS_DEFAULT.load(std::sync::atomic::Ordering::Relaxed).max(1) }
}

/// `ENOMOTO_MIP_THREAD_BUDGET` (試作): 同時に動く計算のスレッドの合計を、指定のスレッド数 T (論理スレッド数で頭打ち済み) 以下に
/// 配る。根の LP の同時実行と根の分離は FJ のスレッドの分を引き、同時に走らせるサブ MIP は max(1, (T - 1) / 4) 本まで、
/// 解探索のスレッドは T >= 4 のときだけ、背景のカットは残り (0 なら回さない)、木は解探索のスレッドが生きている間 1 本減らす。
/// 論理スレッドの少ない計算機 (T = 1・2・4) でも合計が T を超えない
pub(crate) fn thread_budget_on() -> bool {
    env_str!("ENOMOTO_MIP_THREAD_BUDGET").is_some()
}

/// `ENOMOTO_MIP_SUBMIP_OUTCOME_STOP` (試作、既定では使わない): 根のサブ MIP を時間ではなく成果で打ち切る。
/// - サブ MIP が外側の暫定解より良い解を見つけた後、最後に良くなってからのノード数が、そこまでのノード数
///   (最小 `ENOMOTO_T_MIP_SUBMIP_OUTCOME_MIN_NODES`、既定 50) に達したら止める (見つけるのにかかった分だけ続けて見込みがなければやめる)。
///   解を 1 つも見つけていないサブ MIP は今まで通り (時間とノードの上限まで)。10teams の根の被約費用のサブ MIP は 248 ノード目に初めて
///   最適解を見つけるので、時間で一律に絞る `ENOMOTO_MIP_ROOT_SUBMIP_REL` はそれを切っていた
/// - 並列のサブ MIP には外側の刈り込みの上限 (ほかのサブ MIP・主のスレッドが見つけた解による) を打ち切り値として渡し続ける ([`SubmipCtl::outer_best`])
/// - 並列のサブ MIP は終わった順に受け取る (`ENOMOTO_MIP_SUBMIP_FIRST_DONE` と同じ)
pub(crate) fn submip_outcome_stop() -> bool {
    env_str!("ENOMOTO_MIP_SUBMIP_OUTCOME_STOP").is_some()
}

/// 根のサブ MIP と外側の求解の間で共有する情報 ([`submip_outcome_stop`])。
pub(crate) struct SubmipCtl {
    /// 外側の刈り込みの上限 (`prune_limit`、f64 のビット。なければ +inf)。サブ MIP の打ち切り値に使う
    pub(crate) outer_best: std::sync::atomic::AtomicU64,
}

impl SubmipCtl {
    pub(crate) fn new(z: f64) -> Self {
        SubmipCtl { outer_best: std::sync::atomic::AtomicU64::new(z.to_bits()) }
    }
    pub(crate) fn best(&self) -> f64 {
        f64::from_bits(self.outer_best.load(std::sync::atomic::Ordering::Relaxed))
    }
    pub(crate) fn offer(&self, z: f64) {
        let _ = self.outer_best.fetch_update(std::sync::atomic::Ordering::Relaxed, std::sync::atomic::Ordering::Relaxed, |b| (z < f64::from_bits(b)).then_some(z.to_bits()));
    }
}

/// 同時に走らせる別スレッドのサブ MIP の本数の上限。
pub(crate) fn submip_par_limit() -> usize {
    let t = mip_threads();
    if thread_budget_on() { (t.saturating_sub(1) / 4).max(1) } else { t.saturating_sub(1).max(1) }
}

/// [`mip_threads`] の既定 (`ENOMOTO_MIP_THREADS` がないとき)。[`super::solve_mip`] が求解の間だけ木の探索のスレッド数にする。
pub(crate) static MIP_THREADS_DEFAULT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

pub fn solve(p: &MipProblem, params: MipParams) -> MipResult {
    // ヒューリスティクスのサブ MIP は小さいことが多いので、基底のない求解も二段解法だけで解く (同時実行しない)
    let prev_race = params.submip.then(|| crate::simplex::mip_lp::set_cold_race_threads(1));
    // LP の実装は傾き・切片二段解法
    let run = || solve_with::<crate::simplex::mip_lp::TwoStageLp>(p, params, None, None);
    // サブ MIP の経過は利用者向けのログに出さない
    let r = if params.submip { crate::log::suspend(run) } else { run() };
    if let Some(k) = prev_race {
        crate::simplex::mip_lp::set_cold_race_threads(k);
    }
    r
}

/// 木の写し `snap` の未処理のノードから探索を続ける (根のカット・ヒューリスティクス・再スタートは行わない)。解は写しの
/// 問題 (`snap.p`) の空間。見つけた解は写しを作ったスレッドに渡す ([`Portfolio`] の inbox)。
pub(crate) fn solve_snapshot(snap: std::sync::Arc<TreeSnapshot>, params: MipParams) -> MipResult {
    solve_worker(snap, None, params)
}

/// 根のヒューリスティクスの間にカットを強める求解との受け渡し。
pub(super) struct CutJob {
    /// 止めの合図 (主のヒューリスティクスが終わった)
    pub(super) stop: std::sync::atomic::AtomicBool,
    /// 重い設定 (停滞したら HiGHS のスコアの第 2 段階に入る、停滞の判定を緩める)
    pub(super) heavy: bool,
    /// 結果: 最後の LP のカットの行 (係数, 下限, 上限)、目的値 (offset 込み)、ラウンド数
    pub(super) out: std::sync::Mutex<Option<(Vec<(Vec<(usize, f64)>, f64, f64)>, f64, usize)>>,
    /// 途中の LP 解 (ラウンドごとに更新)。`ENOMOTO_MIP_BG_CUTS_SHARE_X` なら主の根のヒューリスティクスが使う
    pub(super) latest_x: std::sync::Mutex<Option<Vec<f64>>>,
    /// 結果の LP の最後の基底 (既定で返す。`ENOMOTO_MIP_NO_BG_CUTS_BASIS` で返さない。主がカットの行を置き換えた後の LP に入れる)
    pub(super) basis: std::sync::Mutex<Option<super::lp::Basis>>,
}

/// カットを強める求解のスレッド。落とすと止めの合図を出して終わりを待つ (主が途中で戻っても残らないように)。
struct BgCuts {
    job: std::sync::Arc<CutJob>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl BgCuts {
    fn finish(&mut self) -> Option<(Vec<(Vec<(usize, f64)>, f64, f64)>, f64, usize)> {
        self.job.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        self.job.out.lock().unwrap().take()
    }
}

impl Drop for BgCuts {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

/// 木の写し `snap` から解く (`pool` があれば共有の待ち行列からノードを取る従のスレッドとして)。
fn solve_worker(snap: std::sync::Arc<TreeSnapshot>, pool: Option<std::sync::Arc<WorkPool>>, params: MipParams) -> MipResult {
    let p = snap.p.clone();
    solve_with::<crate::simplex::mip_lp::TwoStageLp>(&p, params, Some(snap), pool)
}

fn solve_with<L: MipLp>(p: &MipProblem, params: MipParams, snap: Option<std::sync::Arc<TreeSnapshot>>, pool: Option<std::sync::Arc<WorkPool>>) -> MipResult {
    let start = Instant::now();
    let deadline = if params.time_limit.is_finite() { Some(start + Duration::from_secs_f64(params.time_limit.max(0.0))) } else { None };
    let mut dom = Domain::new(p);
    let fail = |status, nodes| MipResult { status, x: None, objective: None, best_bound: f64::INFINITY, nodes, lp_iterations: 0 };
    if let Some(sn) = &snap {
        for j in 0..p.n {
            dom.tighten_lower(p, j, sn.lo[j]);
            dom.tighten_upper(p, j, sn.up[j]);
        }
    }
    if !dom.propagate(p) {
        if let Some(pl) = &pool {
            pool_release(pl);
        }
        return fail(MipStatus::Infeasible, 0);
    }
    dom.commit_root();
    dom.take_changed();
    let lp = L::new(&dom.lo, &dom.up, &p.cost, &p.rows, &p.row_lo, &p.row_up);
    let pool_tid = pool.as_ref().map_or(0, |p| p.next_tid.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    // racing ramp-up の競争中に始めた従か (主は start_workers で決める)
    let racing0 = pool.as_ref().is_some_and(|pl| pl.racing.load(std::sync::atomic::Ordering::Relaxed));
    if !params.submip {
        if let Some(x) = DEBUG_SOL.with(|d| d.borrow().clone()) {
            if x.len() == p.n {
                dom.dbg_sol = Some(std::sync::Arc::new(x));
            }
        }
    }
    // `ENOMOTO_MIP_OBJ_STEP_SKIP_FIXED` (試作、既定では使わない): 整数値に固定された連続列は整数列と同じに扱って
    // 刻みを判定する (10teams は固定された連続列 225 本が費用を持ち、刻みが無いことになっていた。HiGHS は前処理で消して
    // 「刻み 1」と見る)
    let obj_step = if env_str!("ENOMOTO_MIP_OBJ_STEP_SKIP_FIXED").is_some() { p.objective_step_bounds(Some((&dom.lo, &dom.up))) } else { p.objective_step() };
    let mut s: Solver<L> = Solver {
        p,
        params,
        dom,
        lp,
        queue: NodeQueue::new(),
        pc: snap.as_ref().map_or_else(|| Pseudocost::new(p.n), |sn| sn.pc.clone()),
        incumbent: None,
        obj_step,
        start,
        cpu0: super::cap_clock(),
        sep_par_cpu: 0.0,
        path_collect: None,
        bg_cut_rounds: 0,
        fc_network: false,
        face_secs: 0.0,
        face_used: 0,
        face_off_loop: false,
        tab_calls: 0,
        tab_tries: 0,
        last_cut_lp_secs: f64::INFINITY,
        accepting_start: false,
        par_sb_stats: [0; 2],
        deadline,
        nodes: 0,
        sb_iters: 0,
        sb_secs: 0.0,
        sb_lps: 0,
        sb_degen_stats: [0; 2],
        sb_agree: std::collections::VecDeque::new(),
        sb_agree_tick: 0,
        sb_agree_stats: [0; 3],
        last_sb: None,
        clique_graph: None,
        cut_clique_adj: std::collections::HashMap::new(),
        clique_auto: None,
        heur_iters: 0,
        heur_iters_root: 0,
        locks: super::heuristics::compute_locks(p),
        // 乱数の種は従のスレッドごとに変える (同じ暫定解から同じ近傍を重複して探さないように)。
        // 主 (番号 0)・1 スレッドは以前と同じ種。`ENOMOTO_MIP_TREE_HEUR_ALL` なら全スレッドが同じ種
        // `ENOMOTO_T_MIP_SEED` (既定 0 で今まで通り): 種をずらす (同じ問題を種を変えて流す計測用)
        rng: (if pool_tid == 0 || env_str!("ENOMOTO_MIP_TREE_HEUR_ALL").is_some() {
            0x2545_F491_4F6C_DD1D
        } else {
            (0x2545_F491_4F6C_DD1D ^ (pool_tid as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)).max(1)
        } ^ tunable!("ENOMOTO_T_MIP_SEED", 0u64, u64).wrapping_mul(0xD1B5_4A32_D192_ED03))
        .max(1),
        root_redcost: None,
        fc_node_sep: !params.submip
            && (env_str!("ENOMOTO_MIP_FC_NODE_SEP").is_some() || env_str!("ENOMOTO_MIP_FC_NODE_SEP_ADAPT").is_some() || env_str!("ENOMOTO_MIP_FC_NODE_SEP2").is_some())
            && super::separation::fixed_charge_network_for_nodes(p),
        fc_adapt: env_str!("ENOMOTO_MIP_FC_NODE_SEP_ADAPT").is_some(),
        fc_sep2: env_str!("ENOMOTO_MIP_FC_NODE_SEP2").is_some(),
        fc_submip: !params.submip && env_str!("ENOMOTO_MIP_FC_SUBMIP").is_some() && super::separation::fixed_charge_network_for_nodes(p),
        tree_sep: !params.submip && env_str!("ENOMOTO_MIP_NO_TREE_SEP").is_none(),
        tree_sep_fc: !params.submip && env_str!("ENOMOTO_MIP_NO_TREE_SEP").is_none() && env_str!("ENOMOTO_MIP_TREE_SEP_NO_FC").is_none() && super::separation::fixed_charge_network_for_nodes(p),
        fc_stats: (0, 0.0, 0, 0),
        fc_probe: 0,
        last_rins: 0,
        lb_k: 18.0,
        improve_turn: 0,
        alns_reward: [0.0; 6],
        alns_count: [0; 6],
        sol_pool: Vec::new(),
        repair_start: None,
        lns_secs: 0.0,
        node_cut_secs: 0.0,
        hcm_root_obj: f64::NEG_INFINITY,
        nca_pending: None,
        nca_ema: 1.0,
        nca_stats: [0; 3],
        node_lp_rows: 0,
        node_sep_stats: [0; 2],
        nca_hist: [0; 5],
        nca_cut_nnz: None,
        noinc_secs: 0.0,
        noinc_calls: 0,
        heur_close: 0.0,
        noinc_reward: [0.0; NOINC_ARMS],
        noinc_count: [0; NOINC_ARMS],
        noinc_cost: [0.0; NOINC_ARMS],
        noinc_prior: [None; NOINC_ARMS],
        noinc_tree_win: None,
        noinc_root_frac: 0,
        early_restart: false,
        tree_restart_check: false,
        noinc_stall: 0,
        noinc_best_close: 0.0,
        noinc_next_node: 0,
        submip_time_frac: tunable!("ENOMOTO_T_MIP_SUBMIP_TIME_FRAC", 0.07, f64),
        submip_time_max: tunable!("ENOMOTO_T_MIP_SUBMIP_TIME_MAX", 6.0, f64),
        noinc_rens_done: false,
        submip_node_mult: 1.0,
        rens_node_limited: false,
        force_fix_cap: false,
        parallel_submips: false,
        pending_submips: std::collections::VecDeque::new(),
        proof_cmir_x: None,
        proof_cmir_stats: [0; 3],
        heur_label: "",
        heur_stop: None,
        root_lp_iters: 0,
        inc_only_fj: false,
        ref_sol: None,
        fj_poor_lb: f64::NEG_INFINITY,
        submip_stall: 0,
        root_submip: if params.submip { ROOT_SUBMIP_CTL.with(|c| c.borrow_mut().take()) } else { None },
        last_improve_node: None,
        submip_basis: None,
        in_heur_role: false,
        other_thread_label: false,
        submip_fix_rate: None,
        submip_is_rens: false,
        rins_root_x: None,
        fj_thread: None,
        lmip_thread: None,
        lmip_mailbox: None,
        fj_has_inc: None,
        ls_thread: None,
        find_thread: None,
        find_out: None,
        find_stop: None,
        rens_succ: (0.0, 0),
        rens_infeas: (0.0, 0),
        node_iters: 0,
        node_iters_first: 0,
        restore_stats: (0, 0, 0),
        restored_now: false,
        restore_diag: (0, 0, false),
        pool_excess: 0,
        pool_basis_stats: 0,
        restored_dse: false,
        node_lps: 0,
        node_lp_secs: 0.0,
        unresolved: false,
        last_log: start,
        user_log_header: false,
        cur_lb: f64::NEG_INFINITY,
        root_lb: f64::NEG_INFINITY,
        root_cut_lb: f64::NEG_INFINITY,
        final_lb: None,
        vbounds: None,
        failed_heurs: 0,
        cut_age: Vec::new(),
        cut_ids: Vec::new(),
        cut_norm: std::collections::HashMap::new(),
        merged_cuts: std::collections::HashSet::new(),
        ts_cut_ids: std::collections::HashSet::new(),
        ts_rows0: 0,
        tab_memo: std::collections::HashMap::new(),
        tab_memo_mode: 0,
        tab_memo_stats: (0, 0),
        bisect_lb: f64::NEG_INFINITY,
        bisect_frac: 0.5,
        bisect_last: 0,
        bisect_secs: 0.0,
        bisect_stats: [0; 3],
        shared: if params.submip { None } else { thread_portfolio() },
        watch: if params.submip { None } else { thread_watch() },
        obj_cut_p: f64::NEG_INFINITY,
        extra_sep_rows: Vec::new(),
        obj_cut_stats: [0; 2],
        next_cut_id: 0,
        snap: snap.clone(),
        snap_alive: Vec::new(),
        snap_p: None,
        snap_serials: Vec::new(),
        fb_seen: 0,
        inbox_seen: 0,
        snap_stats: [0; 3],
        pool_busy: pool.is_some(),
        heur_role_retired: None,
        heur_role_entries: 0,
        trig_last: None,
        trig_v: -1.0,
        trig_eval_at: None,
        root_mu_obs: Vec::new(),
        heur_role_stats: [0; 3],
        role_lm: None,
        role_lm_off: false,
        role_arm_t: [0.0; 2],
        role_arm_wins: [0; 2],
        role_leader: 0,
        orig_best: None,
        pool_hold: None,
        seen_best: f64::INFINITY,
        pc_base: None,
        pool_tid,
        race_done: !racing0,
        race_div: if racing0 { (pool_tid % 4) as u8 } else { 0 },
        race_rng: 0x2545_F491_4F6C_DD1D ^ (pool_tid as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15),
        learned_cursor: 0,
        importing: false,
        learned_stats: [0; 2],
        dbg_imgs: None,
        dbg_last: std::cell::RefCell::new(String::new()),
        dbg_prev_has: false,
        dbg_alive: true,
        dbg_on: !params.submip && DEBUG_SOL.with(|d| d.borrow().is_some()),
        pool,
        workers: Vec::new(),
        worker_totals: (0, 0),
        pool_stats: [0; 3],
        tprof: TreeProf::default(),
        tp_kind: 0,
        lbd: LbNodeDiag::default(),
        proof_conflicts: 0,
        orbitopes: match &snap {
            Some(sn) => Rc::new(sn.orbitopes.clone()),
            None if params.submip => Rc::new(Vec::new()),
            None => super::ORBITOPES.with(|t| t.borrow().clone()).unwrap_or_default(),
        },
        orbitope_fixings: 0,
        lex_gens: match &snap {
            Some(sn) => Rc::new(sn.lex_gens.clone()),
            None if params.submip => Rc::new(Vec::new()),
            None => super::SYM_LEX_GENS.with(|t| t.borrow().clone()).unwrap_or_default(),
        },
        orig: match &snap {
            Some(sn) => sn.orig.as_ref().map(|o| Rc::new((**o).clone())),
            None if params.submip || env_str!("ENOMOTO_MIP_SYM_NO_ORIG").is_some() => None,
            None => super::SYM_ORIG.with(|t| t.borrow().clone()),
        },
        sym_canon: (0, 0),
        orb_col: std::collections::HashMap::new(),
        orb_others: None,
        dive_stack: Vec::new(),
        sibling_backtracks: 0,
        orbitope_prunes: 0,
        row_log: Vec::new(),
        cut_int: if env_str!("ENOMOTO_MIP_NO_IMPLINT").is_some() {
            p.is_int.clone()
        } else if env_str!("ENOMOTO_MIP_IMPLINT_TU").is_some() {
            let b = p.implied_integers();
            let t = p.implied_integers_tu(&b);
            if params.verbose && !params.submip {
                eprintln!("MIP: implied integers: {} by the row rules, {} with the TU rule (ENOMOTO_MIP_IMPLINT_TU)", b.iter().zip(&p.is_int).filter(|(a, i)| **a && !**i).count(), t.iter().zip(&p.is_int).filter(|(a, i)| **a && !**i).count());
            }
            t
        } else {
            p.implied_integers()
        },
        cut_pool: Vec::new(),
        dual_proofs: std::collections::VecDeque::new(),
        dual_proof_nnz: 0,
        proof_first_id: 0,
        proof_base: std::collections::VecDeque::new(),
        proof_keys: std::collections::VecDeque::new(),
        proof_cuts: std::collections::VecDeque::new(),
        proof_pending_cuts: None,
        proof_state: std::collections::VecDeque::new(),
        proof_fifo: [std::collections::VecDeque::new(), std::collections::VecDeque::new()],
        proof_dead: 0,
        proof_ref: std::collections::VecDeque::new(),
        proof_col_index: [Vec::new(), Vec::new()],
        proof_chg_mark: Vec::new(),
        proof_chg_stamp: 0,
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
        multi_stats: [0; 4],
        multi_secs: 0.0,
        cutsel_best: 0.0,
        cutsel_factor: 0.9,
        path_offset: 0,
        debug_x_done: false,
        obj_cols: None,
        obj_prop_stats: [0; 2],
        cutpool_prop_stats: [0; 4],
        hcm_auto: snap.as_ref().is_some_and(|sn| sn.hcm_auto),
        root_obj_probe_stats: [0; 2],
        sb_prop_stats: [0; 2],
        lit_conflicts: super::conflict_pool::LitConflictPool::new(p.n, tunable!("ENOMOTO_T_MIP_LIT_CONFLICT_POOL", 5000usize, usize)),
        lit_stats: [0; 3],
        lit_pos: 0,
        conf_stats: [0; 8],
        farkas_added: 0,
        proof_prunes: 0,
        proof_numeric_rejects: 0,
        proof_last_prune: None,
        proof_prune_after_tighten: false,
        proof_verify_stats: [0; 4],
        proof_tightenings: 0,
        cut_pool_keys: std::collections::HashMap::new(),
        pool_hash: Vec::new(),
        pool_age: Vec::new(),
        cut_pool_nnz: 0,
        pool_idx: Vec::new(),
        pool_val: Vec::new(),
        pool_start: vec![0],
        pool_nocap: false,
        root_fixed0: 0,
        dive_iters: 0,
        dive_calls: 0,
        dive_succ: 0,
        proof_phase: false,
        proof_first_z: f64::INFINITY,
        proof_last_best: (f64::INFINITY, Instant::now()),
        proof_eval_at: None,
        row_hash: Vec::new(),
        row_hash_map: std::collections::HashMap::new(),
        row_hash_epoch: None,
        cut_basis_stats: [0; 2],
        donate_half_at: None,
    };
    // DSE 重みは既定で全ての LP で引き継ぐ (根のカットのループ、潜りの子、強分岐、待ち行列のノードは基底と一緒に保存した
    // 重みを戻す)。`ENOMOTO_MIP_NO_DSE_ALL` で以前の形 (毎回 1 から) に戻す。サブ MIP の中から呼ばれても、終わったら元に戻す
    use std::sync::atomic::Ordering;
    let warm = &crate::simplex::slope_intercept_dual::WARM_DSE_ON;
    let prev = warm.load(Ordering::Relaxed);
    // `ENOMOTO_MIP_SUBMIP_NO_DSE`: サブ MIP (RENS など) の中では引き継がない (10teams: 引き継ぐと根の被約費用ヒューリスティクスの
    // サブ MIP が 0 ノード -> 366 ノード、0.6 -> 3.5 秒)
    let sub_off = s.params.submip && env_str!("ENOMOTO_MIP_SUBMIP_NO_DSE").is_some();
    if dse_all() && !sub_off {
        warm.store(true, Ordering::Relaxed);
    } else if sub_off {
        warm.store(false, Ordering::Relaxed);
    }
    let r = s.run();
    warm.store(prev, Ordering::Relaxed);
    r
}

/// 証明だけのスレッドか (ポートフォリオの二分探索のスレッドで `ENOMOTO_MIP_BISECT_NO_HEUR` のとき。ヒューリスティクスを
/// 一切走らせず、打ち切り値未満に解がないことを示す木の探索とカットに時間を使う)。サブ MIP には及ばない。
fn proof_only() -> bool {
    crate::simplex::slope_intercept_dual::mip_profile() == 2
}

/// 暫定解がない間の木の中のヒューリスティクスの腕の数 (0: RENS、1: Feasibility Pump、2..8: ダイビング
/// 分数・ベクトル長・係数・擬似費用・Farkas・衝突、8: 何もせず木を進める ([`NOINC_TREE_ARM`]))。
const NOINC_ARMS: usize = 9;

/// 「木を進める」腕 (`ENOMOTO_MIP_NOINC_TREE_ARM` のときだけ選ぶ)。
const NOINC_TREE_ARM: usize = 8;

/// 根のヒューリスティクスの名前に対応する、暫定解がない間の木の中のバンディットの腕 ([`NOINC_ARMS`] の番号)。
/// 擬似費用・Farkas・衝突のダイビングは根では使わない。
fn noinc_root_arm(name: &str) -> Option<usize> {
    match name {
        "RENS" => Some(0),
        "feasibility pump" => Some(1),
        "fractional diving" => Some(2),
        "vector length diving" => Some(3),
        "coefficient diving" => Some(4),
        _ => None,
    }
}

/// 木の中のヒューリスティクスを走らせないスレッドか (証明だけのスレッドと、根のヒューリスティクスだけ走らせる
/// 二分探索のスレッド `ENOMOTO_MIP_BISECT_ROOT_HEUR`)。
fn tree_heur_off() -> bool {
    matches!(crate::simplex::slope_intercept_dual::mip_profile(), 2 | 3)
}

/// `ENOMOTO_MIP_HIGHS_CONFLICT_SCORE`: 衝突スコアを HiGHS と同じ変更で上げるか ([`Solver::add_conflicts_multi`])。
fn highs_conflict_score() -> bool {
    env_str!("ENOMOTO_MIP_HIGHS_CONFLICT_SCORE").is_some()
}

/// `ENOMOTO_MIP_PROOF_SIDE_REDUCE` ([`Solver::add_proof_from`]): 0 使わない、1 常に (`=1` など)、2 密すぎる証明だけ (`=dense`)。
fn proof_side_reduce_mode() -> u8 {
    match env_str!("ENOMOTO_MIP_PROOF_SIDE_REDUCE") {
        None => 0,
        Some("dense") => 2,
        Some(_) => 1,
    }
}

/// `ENOMOTO_MIP_LP_INT_BOUNDS` ([`Solver::sync_lp`])。
pub(super) fn lp_int_bounds() -> bool {
    env_str!("ENOMOTO_MIP_LP_INT_BOUNDS").is_some()
}

/// 双対証明から CMIR カットを作るか ([`Solver::add_proof_cmir`]、既定。`ENOMOTO_MIP_NO_PROOF_CMIR` で使わない)。
fn proof_cmir() -> bool {
    env_str!("ENOMOTO_MIP_NO_PROOF_CMIR").is_none()
}

/// `ENOMOTO_MIP_PROOF_FIXPOINT` (ノードの伝播で証明を不動点まで調べ直す)。
fn proof_fixpoint() -> bool {
    env_str!("ENOMOTO_MIP_PROOF_FIXPOINT").is_some()
}

/// 診断用 (`ENOMOTO_MIP_TREE_LOG`): 木のノードの終わり方と分枝を 1 行ずつ標準エラーに出すか (サブ MIP では出さない)。
/// `ENODE d=深さ r=終わり方 lp=LP 値 rows=LP の行数` と `EBR d=深さ col=列 val=値 rows=行数`。HiGHS の計測用パッチの
/// `HNODE`・`HBR` と並べる (scripts/tree_log_compare.py)
fn tree_log() -> bool {
    env_str!("ENOMOTO_MIP_TREE_LOG").is_some()
}

/// DSE 重みを全ての LP で引き継ぎ、待ち行列のノードに保存するか (既定で真、`ENOMOTO_MIP_NO_DSE_ALL` で偽)。
fn dse_all() -> bool {
    env_str!("ENOMOTO_MIP_NO_DSE_ALL").is_none() && crate::simplex::slope_intercept_dual::mip_profile() != 1
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 診断用: 現在の定義域がデバッグ解を含み、かつその目的値が打ち切り値未満なら `why` を表示する。
    pub(super) fn dbg_lost(&self, why: &str, contained: bool) {
        if !self.dbg_on {
            return;
        }
        if !self.params.submip {
            self.dbg_last.replace(format!("{why} at node {}", self.nodes));
        }
        if self.params.submip || !contained {
            return;
        }
        DEBUG_SOL.with(|d| {
            if let Some(x) = d.borrow().as_ref() {
                let z = self.p.objective(x);
                if z < self.prune_limit() - 1e-6 {
                    eprintln!("MIP_DEBUG_SOL lost at node {} ({}): {why} (debug obj {z}, prune limit {})", self.nodes, if self.snap.is_some() { "helper thread" } else { "main thread" }, self.prune_limit());
                }
            }
        });
    }

    /// 待ち行列の下界が `lim` 以上のノードを捨てる。診断用: デバッグ解を含むノードを捨てたら知らせる (`queue.prune` の一括の
    /// 刈り込みは以前はデバッグ解の検査の外だった)。
    fn prune_queue(&mut self, lim: f64) {
        if self.dbg_on {
            let lb = self.queue.iter().filter(|n| n.lower_bound >= lim && self.dbg_changes(&n.changes)).map(|n| n.lower_bound).fold(f64::INFINITY, f64::min);
            if lb.is_finite() {
                self.dbg_lost(&format!("queue prune (node lower bound {lb}, limit {lim})"), true);
            }
        }
        self.queue.prune(lim);
    }

    /// 診断用: ノードの分枝がデバッグ解を含むか。
    fn dbg_changes(&self, changes: &[BoundChange]) -> bool {
        if !self.dbg_on {
            return false;
        }
        DEBUG_SOL.with(|d| d.borrow().as_ref().is_some_and(|x| changes.iter().all(|c| if c.upper { x[c.col] <= c.value + 1e-6 } else { x[c.col] >= c.value - 1e-6 })))
    }

    /// 診断用: デバッグ解の対称な像 (オービトープの生成元で閉じた集合) のどれかを、ノード `n` の分枝が含むか。
    fn dbg_node_has_image(&mut self, n: &OpenNode) -> bool {
        let Some(x) = DEBUG_SOL.with(|d| d.borrow().clone()) else { return false };
        if self.dbg_imgs.is_none() {
            let mut imgs = vec![x];
            let mut k = 0;
            while k < imgs.len() && imgs.len() < 5000 {
                for o in self.orbitopes.iter() {
                    for (_, _, perm) in &o.swaps {
                        let mut y = vec![0.0; imgs[k].len()];
                        for (j, &t) in perm.iter().enumerate() {
                            y[t] = imgs[k][j];
                        }
                        if !imgs.contains(&y) {
                            imgs.push(y);
                        }
                    }
                }
                k += 1;
            }
            eprintln!("MIP_DEBUG_SOL: {} symmetric images", imgs.len());
            self.dbg_imgs = Some(imgs);
        }
        let imgs = self.dbg_imgs.as_ref().unwrap();
        imgs.iter().any(|x| n.changes.iter().all(|c| if c.upper { x[c.col] <= c.value + 1e-6 } else { x[c.col] >= c.value - 1e-6 }))
    }

    /// 診断用: `ENOMOTO_MIP_DEBUG_NODE` のノードで、今の定義域がデバッグ解の像を含むかを表示する。
    fn dbg_trace(&self, stage: &str) {
        if !self.dbg_on {
            return;
        }
        let Some(k) = env_str!("ENOMOTO_MIP_DEBUG_NODE").and_then(|v| v.parse::<u64>().ok()) else { return };
        if self.nodes != k || self.params.submip {
            return;
        }
        let Some(imgs) = self.dbg_imgs.as_ref() else { return };
        let inside: Vec<usize> = (0..imgs.len()).filter(|&i| (0..self.p.n).all(|j| imgs[i][j] >= self.dom.lo[j] - 1e-6 && imgs[i][j] <= self.dom.up[j] + 1e-6)).collect();
        eprintln!("MIP_DEBUG_NODE {k} {stage}: images inside the domain {:?}", inside);
        if inside.is_empty() {
            // どの列の境界が像を外しているか (最初の像について)
            let x = &imgs[0];
            let bad: Vec<(usize, f64, f64, f64)> = (0..self.p.n).filter(|&j| x[j] < self.dom.lo[j] - 1e-6 || x[j] > self.dom.up[j] + 1e-6).map(|j| (j, x[j], self.dom.lo[j], self.dom.up[j])).take(10).collect();
            eprintln!("MIP_DEBUG_NODE {k} {stage}: image 0 violates {:?}", bad);
            for &(j, xj, lo, up) in &bad {
                let upper = xj > up + 1e-6;
                match self.dom.last_change_of(j, upper) {
                    Some((pos, reason, old, new)) => eprintln!("MIP_DEBUG_NODE {k} {stage}:   col {j} {} bound set by stack entry {pos} (reason {reason}: -1 decision, -2 external, -3 local redcost fixing, -4 strong branching fixing, -5 orbitope, -6 dual proof, -7 literal conflict, -8 objective propagation, -9 implication (with ENOMOTO_MIP_DEBUG_IMPL_TAG), >= 0 row) {old} -> {new}; stack len {}, global [{}, {}]", if upper { "upper" } else { "lower" }, self.dom.stack_len(), self.dom.global_lo[j], self.dom.global_up[j]),
                    None => eprintln!("MIP_DEBUG_NODE {k} {stage}:   col {j} {} bound is in the root state (not on the stack); global [{}, {}], root/current [{lo}, {up}]", if upper { "upper" } else { "lower" }, self.dom.global_lo[j], self.dom.global_up[j]),
                }
            }
        }
    }

    /// 診断用: 定義域がデバッグ解を外しているとき、外している列ごとに、その境界の出どころ (記録の位置・理由、ノードの分枝に
    /// 含まれるか、または根の状態) を表示する。
    fn dbg_explain_exclusion(&self, changes: &[BoundChange]) {
        DEBUG_SOL.with(|d| {
            let Some(x) = d.borrow().clone() else { return };
            if self.p.objective(&x) >= self.prune_limit() - 1e-6 {
                return;
            }
            for j in 0..self.p.n {
                let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
                if x[j] >= lo - 1e-6 && x[j] <= up + 1e-6 {
                    continue;
                }
                let upper = x[j] > up + 1e-6;
                let in_changes = changes.iter().any(|c| c.col == j && c.upper == upper);
                match self.dom.last_change_of(j, upper) {
                    Some((pos, reason, old, new)) => eprintln!("MIP_DEBUG_SOL:   node {} thread {}: col {j} (x_dbg {}) {} bound {old} -> {new} set by stack entry {pos} of {} (reason {reason}; in this node's branches: {in_changes}); global [{}, {}]", self.nodes, self.pool_tid, x[j], if upper { "upper" } else { "lower" }, self.dom.stack_len(), self.dom.global_lo[j], self.dom.global_up[j]),
                    None => eprintln!("MIP_DEBUG_SOL:   node {} thread {}: col {j} (x_dbg {}) {} bound [{lo}, {up}] is in the root state (not on the stack); global [{}, {}]", self.nodes, self.pool_tid, x[j], if upper { "upper" } else { "lower" }, self.dom.global_lo[j], self.dom.global_up[j]),
                }
            }
        });
    }

    /// 診断用: 現在の定義域がデバッグ解を含むか (デバッグ解がなければ偽)。
    pub(super) fn dbg_contains(&self) -> bool {
        if !self.dbg_on {
            return false;
        }
        DEBUG_SOL.with(|d| d.borrow().as_ref().is_some_and(|x| (0..self.p.n).all(|j| x[j] >= self.dom.lo[j] - 1e-6 && x[j] <= self.dom.up[j] + 1e-6)))
    }

    /// 診断用: LP の行 (カットを含む) がデバッグ解で満たされているか調べる。
    pub(super) fn dbg_check_rows(&self, label: &str) {
        if !self.dbg_on {
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

    /// HiGHS 流のカット管理をまとめて有効にする (`ENOMOTO_MIP_HIGHS_CUTMGMT`、サブ MIP では使わない):
    /// - 再スタートで効いているカットを問題の行にせず、外せるカットの行として戻す (`maybe_restart` / `run`)
    /// - 木の中のカットの年齢の上限を `ENOMOTO_T_MIP_HCM_AGE_LIMIT` にする (`remove_aged_cuts`)
    /// - 待ち行列から取り出したノードでは、プールと新しい分離を合わせたラウンドを、ノードの下界が伸びる間
    ///   (HiGHS の `HighsSeparation::separate` と同じ基準) 繰り返す。時間の予算 `ENOMOTO_T_MIP_HCM_NODE_SEP_FRAC` 付き
    /// - 根のカットのループの第 2 段階 (`ENOMOTO_MIP_CUT_PHASE2` と同じ。`ENOMOTO_MIP_HCM_NO_PHASE2` で外す)
    ///
    /// - 待ち行列のノードの基底によるカットの保護はせず、取り出したノードの基底が LP の行と合わなければ今の基底から解く
    /// - 1 回の潜り (兄弟への戻りを含む) は `ENOMOTO_T_MIP_HCM_PLUNGE_NODES` (既定 100) ノードまで
    ///
    /// neos-911970 では、既定の再スタートはカットを恒久的な行にして 107 行 -> 358 行 (木の LP は 405 -> 822 行、ノードの LP は
    /// 1 回平均 187 反復) になり、根のカットのループは 47.26 で止まり (HiGHS 52.1)、木での分離は深さ 10 ごとだけだった。
    /// この選択肢では根 52.1、木の LP は 200-430 行、ノードの LP は 1 回 36-75 反復で、単独実行では 51.5 秒で最適値 54.76 を
    /// 証明できた (ただし根のヒューリスティクスの暫定解が 60-68 と悪く、実行ごとのばらつきが大きい)。
    /// 40 問 (3 並列、同時に測った既定 18 問 26.92 に対し) 17 問 28.56: neos-860300 は解けるようになる (58 秒) が neos5・mik-250 が
    /// 時間切れ (年齢で外れたカットが戻りきらずノードの LP が弱くなる)、misc07 11.0 -> 18.7 秒、qnet1 7.9 -> 11.7 秒、
    /// binkar10_1 26.3 -> 38.8 秒。既定では使わない。
    /// 基底の扱いを変えても mik-250 は解けない (60 秒): 保護あり (`ENOMOTO_MIP_HCM_PROTECT`) 下界 -52500、保存基底の修復
    /// (`ENOMOTO_MIP_HCM_REPAIR`) -52656、どちらもなし -52483 (既定の設定は 31-35 秒で最適)。neos5 は修復版だけ 56.6 秒で解けた。
    /// mik-250 で HiGHS 流にすると LP の反復が 193k -> 390k (同じ 4517 ノード) に増え、単体法の主ループが 7.2 -> 16.4 秒。
    /// 行の追加・削除そのもの (標準形の作り直し + 次の求解での再分解) は 40 秒中 0.9 + 0.7 秒 (3192 回)、再分解は 0.1 秒未満、
    /// プールの違反の走査は 1.4-1.9 秒だった (走査は平坦な複製 `pool_idx` で安くした)
    pub(super) fn highs_cutmgmt(&self) -> bool {
        (env_str!("ENOMOTO_MIP_HIGHS_CUTMGMT").is_some() || self.hcm_auto) && !self.params.submip
    }

    pub(super) fn time_up(&self) -> bool {
        self.deadline.is_some_and(|d| Instant::now() >= d)
            || self.heur_stop.is_some_and(|d| Instant::now() >= d)
            || (self.snap.is_some() && FIND_STOP.with(|f| f.borrow().as_ref().is_some_and(|s| s.load(std::sync::atomic::Ordering::Relaxed))))
            || self.pool.as_ref().is_some_and(|p| p.stop.load(std::sync::atomic::Ordering::Relaxed))
            || ROLE_POOL.with(|r| r.borrow().as_ref().is_some_and(|p| p.stop.load(std::sync::atomic::Ordering::Relaxed) || p.fin.load(std::sync::atomic::Ordering::Relaxed)))
            || self.shared.as_ref().is_some_and(|s| {
                s.stop.load(std::sync::atomic::Ordering::Relaxed)
                    || self.watch.is_some_and(|(trig, target)| {
                        (s.best() < trig || s.proven() >= target) && self.start.elapsed().as_secs_f64() >= tunable!("ENOMOTO_T_MIP_BISECT_MIN_RUN", 1.0, f64)
                    })
            })
    }

    pub(super) fn limits(&self, iteration_limit: u64) -> SolveLimits {
        SolveLimits { iteration_limit, cutoff: self.prune_limit() - self.p.offset, deadline: self.deadline }
    }

    /// この値以上の下界のノードは捨ててよい (最小化形、定数項込み)。
    /// このノードで新しいカットを分離するか (SCIP の separator の freq と同じく、深さが `freq` の倍数のノードだけ。
    /// `ENOMOTO_T_MIP_NODE_CUT_FREQ`、既定 10、0 なら分離しない。`ENOMOTO_MIP_NODE_CUTS` なら毎回)。
    fn node_cuts_due(&mut self, depth: usize) -> bool {
        if env_str!("ENOMOTO_MIP_NODE_CUTS").is_some() {
            return true;
        }
        let freq = tunable!("ENOMOTO_T_MIP_NODE_CUT_FREQ", 10usize, usize);
        // 効き目と時間で絞る版 (`ENOMOTO_MIP_FC_NODE_SEP_ADAPT`): 最初の W 回は毎回分離し、その後は効き目の移動平均が
        // しきい値以上で、ノードの分離 (解き直しを含む) の時間が経過時間の f 倍以下の間だけ毎回分離する。外れたら深さの規則に
        // 戻るが、P 回に 1 回は試して効き目を測り続ける
        if self.fc_node_sep && self.fc_adapt {
            if freq > 0 && depth % freq == 0 {
                return true;
            }
            let warm = self.fc_stats.0 < tunable!("ENOMOTO_T_MIP_FC_SEP_WARM", 30u64, u64);
            let eff_ok = self.fc_stats.1 >= tunable!("ENOMOTO_T_MIP_FC_SEP_MIN_GAIN", 0.05, f64);
            let time_ok = self.node_cut_secs < tunable!("ENOMOTO_T_MIP_FC_SEP_TIME_FRAC", 0.3, f64) * self.start.elapsed().as_secs_f64();
            let go = if time_ok && (warm || eff_ok) {
                true
            } else {
                self.fc_probe += 1;
                time_ok && self.fc_probe % tunable!("ENOMOTO_T_MIP_FC_SEP_PROBE", 8u64, u64) == 0
            };
            if go {
                self.fc_stats.3 += 1;
            }
            return go;
        }
        // 固定費用ネットワーク型では、待ち行列から取り出したノードで毎回分離する (HiGHS も取り出したノードで毎回分離する)。
        // 分枝で開閉が決まった弧で LP 解が変わり、経路集約の flow cover 系のカットが効く: beasleyC3 は 4 スレッド・120 秒で
        // 時間切れ (木の下界 746.9、最適 754) -> 57 秒 (経路集約の始点も使う、[`Self::separate`])。一律に毎回分離すると
        // neos5・misc07 などが悪化する (下の注記)
        if self.fc_node_sep {
            return true;
        }
        // `ENOMOTO_T_MIP_NODE_CUT_TIME_FRAC` = f (> 0): HiGHS と同じく待ち行列から取り出したノードでは毎回分離するが、
        // ノードの分離 (LP の解き直しを含む) に使った時間が経過時間の f 倍を超えている間は深さの規則に戻す。
        // neos-911970 では毎回分離すると木の下界が 60 秒で 53.69 -> 54.68 (最適値 54.76) と大きく上がるが、
        // 無制限 (`ENOMOTO_MIP_NODE_CUTS`) では neos5 (63 行、6 万ノード) が時間切れになり misc07 も 12 -> 21 秒。
        // f = 0.1 でも 40 問では 17 問 28.67 (既定 19 問 27.81): neos5・neos-860300 が時間切れ、misc07 13.7 -> 22.7 秒、
        // mik-250 34.9 -> 48.6 秒 (加えたカットの行が残ってノードの LP が重くなる) ので既定では使わない
        let f = tunable!("ENOMOTO_T_MIP_NODE_CUT_TIME_FRAC", 0.0, f64);
        if f > 0.0 && self.node_cut_secs < f * self.start.elapsed().as_secs_f64() {
            return true;
        }
        freq > 0 && depth % freq == 0
    }

    /// `ENOMOTO_MIP_TREE_SEP` (木の中の分離の規則をまとめた試作): 待ち行列から取り出したノードで最初のラウンドの分離を
    /// するか。(1) 費用: ノードの分離 (解き直しを含む) の時間が経過時間の `ENOMOTO_T_MIP_TREE_SEP_TIME_FRAC` 倍 (既定 0.1) と
    /// ノードの LP の時間の `ENOMOTO_T_MIP_TREE_SEP_LP_FRAC` 倍 (既定 0.25、0 で判定しない) に収まる間だけ。カットが密
    /// ([`Self::nca_dense`]、`ENOMOTO_T_MIP_NODE_CUT_ADAPT_DENSITY`) なら深さの規則 ([`Self::node_cuts_due`] の既定の部分) だけ。
    /// (2) 頻度: 効いた割合の指数移動平均 (`nca_ema`、伸びが打ち切り値までの差の `ENOMOTO_T_MIP_NODE_CUT_ADAPT_GAIN` 倍以上か打ち切り)
    /// が `ENOMOTO_T_MIP_TREE_SEP_RATE` (既定 0.2) 以上なら毎回、下なら深さの規則と `ENOMOTO_T_MIP_TREE_SEP_PROBE` (既定 10)
    /// ノードに 1 回の試し
    fn tree_sep_start(&mut self, depth: usize) -> bool {
        let freq = tunable!("ENOMOTO_T_MIP_NODE_CUT_FREQ", 10usize, usize);
        let depth_rule = freq > 0 && depth % freq == 0;
        if !self.tree_sep_budget_ok() {
            return depth_rule;
        }
        if self.nca_dense() {
            return depth_rule;
        }
        // `ENOMOTO_T_MIP_TREE_SEP_HEAVY` = h (> 0): LP の行が最初の取り出したノードの h 倍を超えて重くなったら、毎回分離するのに
        // 要る効いた割合を `ENOMOTO_T_MIP_TREE_SEP_HEAVY_RATE` (既定 0.4) に上げる。noswot では分離そのものは軽い (経過時間の 3%) が、
        // 残ったカットでノードの LP が 420 -> 590 行・27 -> 35 反復と重くなり遅くなった (効いた割合 0.19〜0.32)。neos-911970 は
        // 行が 2 倍になるが割合 0.44〜0.57 で、重くなっても続ける価値がある
        if self.ts_rows0 == 0 {
            self.ts_rows0 = self.lp.num_rows().max(1);
        }
        let h = tunable!("ENOMOTO_T_MIP_TREE_SEP_HEAVY", 0.0, f64);
        let rate = if h > 0.0 && self.lp.num_rows() as f64 > h * self.ts_rows0 as f64 {
            tunable!("ENOMOTO_T_MIP_TREE_SEP_HEAVY_RATE", 0.4, f64)
        } else {
            tunable!("ENOMOTO_T_MIP_TREE_SEP_RATE", 0.2, f64)
        };
        if depth_rule || self.nca_ema >= rate {
            return true;
        }
        self.nca_stats[2] += 1;
        self.nca_stats[2] % tunable!("ENOMOTO_T_MIP_TREE_SEP_PROBE", 10u64, u64).max(1) == 0
    }

    /// [`Self::tree_sep_start`] の時間の予算 (2 ラウンド目以降もこれを見る)
    fn tree_sep_budget_ok(&self) -> bool {
        let f = tunable!("ENOMOTO_T_MIP_TREE_SEP_LP_FRAC", 0.25, f64);
        self.node_cut_secs < tunable!("ENOMOTO_T_MIP_TREE_SEP_TIME_FRAC", 0.1, f64) * self.start.elapsed().as_secs_f64()
            && (f <= 0.0 || self.node_cut_secs < f * self.node_lp_secs.max(0.05))
    }

    /// 効き目で続けるノードの分離 (`ENOMOTO_MIP_NODE_CUT_ADAPT`): 待ち行列から取り出したノードで、ノードの分離に使った時間が
    /// 経過時間の `ENOMOTO_T_MIP_NODE_CUT_ADAPT_FRAC` 倍に収まり、分離が効いた割合 (分離の後の LP の値の伸びが打ち切り値までの
    /// 差の `ENOMOTO_T_MIP_NODE_CUT_ADAPT_GAIN` 倍以上、または打ち切り) の指数移動平均が `ENOMOTO_T_MIP_NODE_CUT_ADAPT_RATE`
    /// 以上の間は毎回分離する。割合が下がったら `ENOMOTO_T_MIP_NODE_CUT_ADAPT_PROBE` ノードに 1 回だけ試して割合を測り直す。
    /// `ENOMOTO_T_MIP_NODE_CUT_TIME_FRAC` (一律に毎回分離) が neos5・misc07 などで遅くなったので、効かない問題では止める形にした。
    /// 伸びで数える効き目は qiu・misc07・mas76 でも 0.9 以上になり、遅くなる問題を止められない (neos-911970 は 0.3〜0.5)。
    /// そこで費用の側を 2 つ足した: カットが密なら止める ([`Self::nca_dense`])、分離の時間がノードの LP の時間に比べて
    /// 大きければ止める ([`Self::nca_lp_budget_ok`])。analysis/node_cut_adapt_fix_20261020.md
    fn node_cut_adapt_due(&mut self) -> bool {
        if self.node_cut_secs >= tunable!("ENOMOTO_T_MIP_NODE_CUT_ADAPT_FRAC", 0.1, f64) * self.start.elapsed().as_secs_f64() || !self.nca_lp_budget_ok() {
            return false;
        }
        if self.nca_ema >= tunable!("ENOMOTO_T_MIP_NODE_CUT_ADAPT_RATE", 0.3, f64) {
            return true;
        }
        self.nca_stats[2] += 1;
        self.nca_stats[2] % tunable!("ENOMOTO_T_MIP_NODE_CUT_ADAPT_PROBE", 20u64, u64).max(1) == 0
    }

    /// 効き目で続けるノードの分離の時間 (解き直しを含む) が、ノードの LP の時間の `ENOMOTO_T_MIP_NODE_CUT_ADAPT_LP_FRAC` 倍
    /// (既定 0.1、0 なら判定しない) に収まっているか。ノードの LP が軽く木が大きい問題 (mas76: 1 回 9 反復) では、分離の時間が
    /// ノードの LP の 0.2〜0.7 倍になり、伸びが打ち切り値までの差の 2% を超えても (効いた割合 0.95〜1.0) ノード数は減らない
    /// (misc07・mas76・eil33-2・mik-250 が 15〜30% 遅い)。neos-911970 では 0.04〜0.08 倍
    fn nca_lp_budget_ok(&self) -> bool {
        let f = tunable!("ENOMOTO_T_MIP_NODE_CUT_ADAPT_LP_FRAC", 0.1, f64);
        f <= 0.0 || self.node_cut_secs < f * self.node_lp_secs
    }

    /// 効き目で続けるノードの分離を止めるほどカットが密か: ノードで加えたカットの非零の数の指数移動平均が、元の行の平均の
    /// `ENOMOTO_T_MIP_NODE_CUT_ADAPT_DENSITY` 倍 (既定 10、0 なら判定しない) を超えるか。密なら効き目で続ける分離をやめ、
    /// 既定の深さの規則だけで分離する (ノードの分離はどれもこの平均を更新するので、疎に戻れば再び続ける)。密なカットは加えた後のすべてのノードの LP を
    /// 重くする (qiu: 元の行 3.5 非零に対しカット 300〜390 非零、rout: 7.3 に対し 140〜160。neos-911970 は 32 に対し 19)。
    /// 分離の時間 (`..._FRAC`) にはこの LP の重さが入らず、伸び (`..._GAIN`) は qiu の方が neos-911970 より大きいので、
    /// 費用の側をカットの密度で見る
    pub(super) fn nca_dense(&mut self) -> bool {
        let r = tunable!("ENOMOTO_T_MIP_NODE_CUT_ADAPT_DENSITY", 10.0, f64);
        if r <= 0.0 {
            return false;
        }
        let m0 = self.p.m;
        if self.nca_cut_nnz.is_none() && self.lp.num_rows() > m0 {
            let k = self.lp.num_rows() - m0;
            self.nca_cut_nnz = Some((m0..self.lp.num_rows()).map(|i| self.lp.row(i).len()).sum::<usize>() as f64 / k as f64);
        }
        let avg = (self.p.rows.iter().map(|r| r.len()).sum::<usize>() as f64 / m0.max(1) as f64).max(1.0);
        // `ENOMOTO_MIP_TREE_SEP` では上限を列の数の `ENOMOTO_T_MIP_TREE_SEP_DENSITY_COLS` 倍 (既定 0.05) より下げない: 元の行が
        // 非常に短い問題 (beasleyC3 は 1 行平均 2.8 非零) では、列の 1.7% のカット (29 非零) でも元の行の 10 倍を超えて止まった。
        // qiu (840 列、カット 110〜220 非零)・rout (556 列、150〜185) は 0.05 倍でも密と判定される
        let floor = if self.tree_sep { tunable!("ENOMOTO_T_MIP_TREE_SEP_DENSITY_COLS", 0.05, f64) * self.p.n as f64 } else { 0.0 };
        self.nca_cut_nnz.is_some_and(|d| d > (r * avg).max(floor))
    }

    /// ノードの分離の後の解き直しの結果を効き目の統計に入れる (`obj0`: 分離の前の LP の値、`obj`: 解き直した値。
    /// 枝が切れたら +inf)。効き目は打ち切り値までの差に対する上がり幅 (0..1)。暫定解がなければ |obj0| の 1% を差とする
    pub(super) fn fc_record(&mut self, obj0: f64, obj: f64) {
        if !self.fc_node_sep {
            return;
        }
        let lim = self.prune_limit();
        let gap = if lim.is_finite() && lim > obj0 { lim - obj0 } else { 1e-2 * obj0.abs().max(1.0) };
        let g = if obj.is_finite() { ((obj - obj0) / gap).clamp(0.0, 1.0) } else { 1.0 };
        if obj >= lim {
            self.fc_stats.2 += 1;
        }
        self.fc_stats.0 += 1;
        let a = (1.0 / self.fc_stats.0 as f64).max(tunable!("ENOMOTO_T_MIP_FC_SEP_EMA", 0.05, f64));
        self.fc_stats.1 += a * (g - self.fc_stats.1);
    }

    /// 根の LP が打ち切り値に届いたら根で終える (既定で使う。`ENOMOTO_MIP_NO_ROOT_GAP_EXIT` で使わない): 根 (写しから解く求解でない) で、根の LP の値 (`cur_lb`、カット前の値で常に正しい下界) が暫定解の打ち切り値 ([`Self::prune_limit`]) に
    /// 届いたか。届いていれば根のカット・ヒューリスティクスを続けずに最適で終える。ns1116954 (目的関数が全部 0) では FJ が
    /// 14.5 秒に 0 を見つけても、カットのループ・根のヒューリスティクス (内点法の丸め 17 秒)・裏のカットを回して 52 秒で終わっていた
    /// カットの後の根の LP の値 (`root_lb`) も下界に使う (既定。`ENOMOTO_MIP_NO_ROOT_CUT_GAP_EXIT` ならカット前の `cur_lb` だけ)。
    /// カット前の値だけを見ていたので、カットで根の下界が暫定解に届いても根で止まらず、根のヒューリスティクスが期限を
    /// 越えると木の最初の時間切れの判定 (根のノードを刈る前) で時間切れになっていた (physiciansched6-2: 暫定解・根の下界が
    /// どちらも 49324 で 60 秒の時間切れ。analysis/term_check_20261022.md)
    /// 暫定解がなくても、打ち切り値 (サブ MIP・再スタート後の求解に渡された値) があれば、下界がそれに届いたら閉じる
    /// (それより良い解はない。木で根のノードを刈ったときと同じく実行不能で終え、呼び出し側が自分の暫定解を最適とする)。
    /// physiciansched6-2 では再スタート前の暫定解 49324 が再スタート後の問題の前処理で捨てられ、再スタート後の根のカットで
    /// 下界が 49324 に届いても止まらず時間切れになっていた
    pub(super) fn root_gap_closed(&self) -> bool {
        let lb = if env_str!("ENOMOTO_MIP_NO_ROOT_CUT_GAP_EXIT").is_some() { self.cur_lb } else { self.cur_lb.max(self.root_lb).max(self.root_cut_lb) };
        self.root_gap_closed_at(lb)
    }

    /// 根で、下界 `lb` (根の LP の値。カットは大域的に成り立つので、カットの後の値も問題全体の下界) が打ち切り値に届いたか。
    /// 暫定解があれば、打ち切り値がほかのスレッドの解 (ポートフォリオ並列) から来ているときは除く (自分の暫定解を最適とは
    /// 言えない)。暫定解がなければ (`ENOMOTO_MIP_NO_ROOT_CUT_GAP_EXIT` でないとき) 打ち切り値が有限なら。閉じたら
    /// [`Self::finish_root_gap`] で終える
    pub(super) fn root_gap_closed_at(&self, lb: f64) -> bool {
        if env_str!("ENOMOTO_MIP_NO_ROOT_GAP_EXIT").is_some() || self.snap.is_some() {
            return false;
        }
        let ok = match self.incumbent.as_ref() {
            Some((z, _)) => self.shared.as_ref().is_none_or(|s| !(s.best() < *z)),
            None => env_str!("ENOMOTO_MIP_NO_ROOT_CUT_GAP_EXIT").is_none() && self.prune_limit().is_finite(),
        };
        ok && lb >= self.prune_limit()
    }

    /// 根の下界が打ち切り値に届いた ([`Self::root_gap_closed_at`]) ので終える: 暫定解があれば最適、なければ打ち切り値より良い解はない
    fn finish_root_gap(&mut self, what: &str, lb: f64) -> MipResult {
        if self.params.verbose {
            eprintln!("MIP: the {what} {lb:.10e} closed the gap to the prune limit {:.10e} (incumbent {:?}, {:.2}s)", self.prune_limit(), self.incumbent.as_ref().map(|(z, _)| *z), self.start.elapsed().as_secs_f64());
        }
        match self.incumbent.as_ref().map(|(z, _)| *z) {
            Some(z) => self.finish(MipStatus::Optimal, z),
            None => self.finish(MipStatus::Infeasible, f64::INFINITY),
        }
    }

    /// カットの後の根の LP の値 `root_obj` も使って根を閉じるか ([`Self::root_gap_closed_at`]。`ENOMOTO_MIP_NO_ROOT_CUT_GAP_EXIT` なら使わない)
    fn root_cut_gap_closed(&self, root_obj: f64) -> bool {
        env_str!("ENOMOTO_MIP_NO_ROOT_CUT_GAP_EXIT").is_none() && self.root_gap_closed_at(self.root_bound_now(root_obj))
    }

    /// 根で分かっている下界: カット前の LP、カットのループの LP の値の最大、カットの後の根の LP の値 `root_obj`
    fn root_bound_now(&self, root_obj: f64) -> f64 {
        root_obj.max(self.cur_lb).max(self.root_lb).max(self.root_cut_lb)
    }

    pub(super) fn prune_limit(&self) -> f64 {
        self.prune_limit_ex(true)
    }

    /// 刈り込みの上限。`with_orig` が偽なら `orig_best` (`ENOMOTO_MIP_SYM_ORIG_CUT`) を数えない (「暫定解なし」の判定に使う)
    pub(super) fn prune_limit_ex(&self, with_orig: bool) -> f64 {
        // ポートフォリオ並列: 他のスレッドの解の目的値も打ち切りに使う
        let inc = self.incumbent.as_ref().map(|(z, _)| *z);
        let ob = if with_orig { self.orig_best.as_ref().map(|(z, _)| *z) } else { None };
        let own = match (inc, ob) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let shared = self.shared.as_ref().map(|s| s.best()).filter(|v| v.is_finite());
        // 根のサブ MIP: 外側の刈り込みの上限も打ち切り値として使う ([`submip_outcome_stop`])。外側が計算した上限をそのまま使う
        // (暫定解の目的値として入れると、サブ MIP の相対ギャップ (1e-4 以上) で外側の暫定解から 1e-4 も離れた所で刈り込み、
        // neos-827175 では根の RINS が外側の暫定解 112.002 より 4.3e-6 だけ良い最適解 112.00152 を捨てていた)
        let outer = self.root_submip.as_ref().map_or(f64::INFINITY, |c| c.best());
        let eff = match (own, shared) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let lim = match eff.as_ref() {
            None => self.params.cutoff,
            Some(z) => {
                let strict = match self.obj_step {
                    Some(step) => z - step + (1e-6 * z.abs().max(1.0)).min(0.5 * step),
                    None => z - 1e-9 * z.abs().max(1.0),
                };
                let gap = z - (self.params.rel_gap * z.abs().max(1.0)).max(self.params.abs_gap);
                strict.min(gap).min(self.params.cutoff)
            }
        };
        lim.min(outer)
    }

    /// 暫定解があり、その値が下界 `lb` (の切り上げ) より目的関数の刻み (なければ相対 1e-6) 以上悪いか
    /// ([`Self::obj_support`] を呼ぶ条件)。
    pub(super) fn obj_support_gap(&self, lb: f64) -> bool {
        let Some((z, _)) = self.incumbent.as_ref() else { return false };
        match self.obj_step {
            Some(step) => *z - step * ((lb - self.p.offset) / step - 1e-6).ceil() - self.p.offset >= step - 1e-9,
            None => *z - lb > 1e-6 * z.abs().max(1.0),
        }
    }

    pub(super) fn sync_lp(&mut self) {
        // `ENOMOTO_MIP_LP_INT_BOUNDS`: HiGHS (`HighsLpRelaxation::flushDomain` が局所的な定義域では
        // `removeContinuousChangedCols` する) と同じく、連続列は LP に大域的な境界だけを渡す。伝播で締まった連続列の境界は
        // 行と整数列の境界から導かれるものが多く LP の値は (ほぼ) 変わらないが、LP に置くと双対証明・Farkas 証明が
        // その列の被約費用に乗って密で局所的なものになる (neos-2978193-inde: 閉じた施設の x を 0 に固定した 324 列)。
        // 置かなければ証明は行 (容量制約) を通して整数列に乗り、短い大域的な証明になる
        let int_only = lp_int_bounds();
        for j in self.dom.take_changed() {
            if int_only && !self.p.is_int[j] {
                self.lp.set_col_bounds(j, self.dom.global_lo[j], self.dom.global_up[j]);
            } else {
                self.lp.set_col_bounds(j, self.dom.lo[j], self.dom.up[j]);
            }
        }
    }

    /// 解 `x` (整数列は丸める) を暫定解として試す。採用したら真。
    pub(super) fn try_incumbent(&mut self, mut x: Vec<f64>) -> bool {
        for j in 0..self.p.n {
            if self.p.is_int[j] {
                x[j] = x[j].round();
            }
        }
        // 元の問題では実行可能だが、対称性の行・固定に反し、並べ替えても直らなかった解 (`ENOMOTO_MIP_SYM_ORIG_INC`)
        let mut orig_only = false;
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
            // `ENOMOTO_MIP_SYM_LEX_CANON` (試作): lex-leader の行に反する解も生成元で写して使う。9946555 (対称性の検出の近道)
            // で neos-950242 に lex-leader の行 3-4 本が付くようになり、根の局所探索・FJ の解 (4、22 など) が並べ替えられずに
            // 捨てられていた (以前は対称性なしで 4 を見つけていた)
            // FJ と Local-MIP を混ぜる形 (`ENOMOTO_MIP_FJ_LMIP_MIX`) の解には既定で使う: Local-MIP は対称性の行のない問題で探すので、
            // neos-950242 では最適解 4 が lex-leader の行に反して拒まれ 5〜8 で止まっていた (並べ替えると 3 回とも根で最適)
            let lex_canon = env_str!("ENOMOTO_MIP_SYM_LEX_CANON").is_some() || (fj_lmip_mix_on() && self.heur_label == "Local-MIP");
            if !self.lex_gens.is_empty() && !self.p.is_feasible(&x, FEASTOL) && lex_canon {
                super::symmetry::lex_canonicalize(&self.lex_gens, &mut x, 4 * self.lex_gens.len() * 64 + 1000);
            }
            if !self.p.is_feasible(&x, FEASTOL) {
                self.sym_canon.1 += 1;
                // `ENOMOTO_MIP_SYM_ORIG_INC` (試作): それでも暫定解にする。対称性の行は対称な解のうち 1 つを残す絞り込みで、
                // この解の像のどれかが対称性の行を満たし目的値は同じなので、目的値は打ち切り値に使え、解そのものは元の問題の
                // 解として返せる。対称性の行のある問題で解を磨く手順 (oneopt・連続変数の解き直し・解の溜め) には使わない
                // 再スタートの後の問題は前処理をかけ直していて (列の代入・固定・目的関数の定数)、`orig` (最初の回の対称性の行を
                // 加える前の問題) とは解の空間が違うので使わない
                if self.params.restarts > 0 || orig.n != self.p.n {
                    return false;
                }
                // `ENOMOTO_MIP_SYM_ORIG_CUT` (試作): 暫定解にはせず、値だけを打ち切り値に使う (ORIG_INC と違い、ヒューリスティクスと
                // 「暫定解なし」の判定は今までどおり動く)。暫定解より良い解だけを残す
                if sym_orig_cut_on() {
                    let z = orig.objective(&x);
                    let cur = self.prune_limit();
                    if z < cur - 1e-9 * z.abs().max(1.0) && self.orig_best.as_ref().is_none_or(|(b, _)| z < *b) {
                        if self.params.verbose {
                            eprintln!("MIP: a solution feasible only without the symmetry rows kept as a cutoff value {z:.10e} [{}]", self.heur_label);
                        }
                        self.orig_best = Some((z, x));
                        if let (None, Some(s)) = (&self.snap, &self.shared) {
                            s.publish(z);
                        }
                        if !self.params.submip {
                            super::record_incumbent(z);
                        }
                        let lim = self.prune_limit();
                        self.prune_queue(lim);
                    }
                    return false;
                }
                if env_str!("ENOMOTO_MIP_SYM_ORIG_INC").is_none() {
                    return false;
                }
                orig_only = true;
            } else {
                self.sym_canon.0 += 1;
            }
        }
        let z = self.p.objective(&x);
        // `ENOMOTO_MIP_RESTART_REF` (試作): 再スタート後の求解では、打ち切り値 (再スタート前の暫定解より良いこと) に届かない解を
        // 暫定解にしない。届かない解は外側では捨てられるのに、暫定解にすると RINS などの相手になり、暫定解なし用の
        // ヒューリスティクスも止める (mzzv42z: 打ち切り値 -19874 の再スタートの後で FJ の -680・RINS の -5770 を暫定解にした)
        // 再スタート前の暫定解そのもの (再スタート後の問題でも実行可能なもの) は打ち切り値より必ず悪いが、受け取る
        // (受け取らないと再スタート後の求解が暫定解なしになる。neos-911970: 62.30 を捨て、60 秒で解けない回が増えた)
        if self.incumbent.is_none() && self.params.restarts > 0 && !self.params.submip && !self.accepting_start && z > self.params.cutoff && super::restart_ref_for(self.p) {
            return false;
        }
        if !orig_only {
            self.pool_add(z, &x);
        }
        let better = self.incumbent.as_ref().is_none_or(|(inc, _)| z < *inc - 1e-9 * inc.abs().max(1.0));
        if better {
            // oneopt (SCIP の heur_oneopt): 整数列を 1 つずつ目的値の良くなる向きに、行を破らない範囲で動かす
            let mut x = x;
            let mut z = z;
            if orig_only && self.params.verbose {
                eprintln!("MIP: accepting a solution feasible in the problem without the symmetry rows ({z:.10e})");
            }
            if !orig_only && env_str!("ENOMOTO_MIP_NO_ONEOPT").is_none() && self.one_opt(&mut x) {
                let z2 = self.p.objective(&x);
                if z2 < z && self.p.is_feasible(&x, FEASTOL) {
                    if self.params.verbose {
                        eprintln!("MIP: oneopt improved {z:.10e} -> {z2:.10e}");
                    }
                    z = z2;
                }
            }
            if !orig_only {
                self.resolve_continuous(&mut x, &mut z);
                self.pool_add(z, &x);
            }
            if let (Some(snap), Some(s)) = (&self.snap, &self.shared) {
                // 木の写しから解く求解の解は写しの問題の空間なので、写しを作ったスレッドに渡す (共有の最良値はそのスレッドが
                // 受け取ったときに更新する。解を持たないまま最良値だけ下げると、その解を誰も返せないことがある)
                s.inbox.lock().unwrap().push((snap.serial, x.clone(), self.heur_label, self.in_heur_role));
                s.inbox_n.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                // 木を分け合う従のスレッドは、解を主に渡した後で最良値も知らせる (主は終わる前に必ず受け取る)
                if self.pool.is_some() {
                    s.publish(z);
                }
            } else if let Some(s) = &self.shared {
                s.publish(z);
            }
            if !self.params.submip {
                super::record_incumbent(z);
            }
            // 木を分け合う: 解そのものも共有する (ヒューリスティクス役のスレッドが近傍の中心に使う)
            if let Some(pool) = self.pool.as_ref().filter(|p| p.heur_k > 0) {
                let mut g = pool.best_x.lock().unwrap();
                if g.as_ref().is_none_or(|(zb, _)| z < *zb) {
                    *g = Some((z, std::sync::Arc::new(x.clone())));
                    pool.has_x.store(true, std::sync::atomic::Ordering::Release);
                }
            }
            // 根の解発見スレッドの解も FJ の解と同じに扱う (質の悪い解なら、暫定解なし用の RENS・解発見スレッドを続ける)。
            // 局所探索の刻みを直した後、hypothyroid-k1 では解発見スレッドの局所探索が -27 を返し、それで「FJ の解だけ」でなくなって
            // 暫定解なし用の RENS (-2851 = 最適値を出す) が回らなくなった
            // FJ のスレッドの Local-MIP (`ENOMOTO_MIP_FJ_LMIP_MIX`・`ENOMOTO_MIP_FJ_LOCALMIP`・`ENOMOTO_MIP_LOCALMIP`) の解も同じに扱う。
            // そうしないと、混ぜた形では FJ の解も受け渡し箱を通って「Local-MIP」の名で入り、hypothyroid-k1 で RENS が回らず -3 のまま終わった
            self.inc_only_fj = self.heur_label.starts_with("feasibility jump") || self.heur_label.starts_with("find thread") || self.heur_label == "Local-MIP";
            if !self.proof_first_z.is_finite() {
                self.proof_first_z = z;
            }
            self.incumbent = Some((z, x));
            self.last_improve_node = Some(self.nodes);
            // FJ のスレッド (と解発見スレッド) の解だけのうちは立てない (`=2` はその解の改善を続ける)
            if let Some(f) = self.fj_has_inc.as_ref().filter(|_| !self.inc_only_fj) {
                f.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            self.root_redcost_fixing();
            self.tree_restart_check = true;
            let lim = self.prune_limit();
            self.prune_queue(lim);
            if self.params.verbose {
                let other = if std::mem::take(&mut self.other_thread_label) { "other thread: " } else { "" };
                // 被約費用による大域的な固定の後に固定されている整数列の数 (根の LP を解く前に固定されていた数との差も)。
                // 木の途中で再スタートする価値があるかの目安
                let gfix = (0..self.p.n).filter(|&j| self.p.is_int[j] && self.dom.global_lo[j] == self.dom.global_up[j]).count();
                eprintln!("MIP:   global fixed integers {gfix} of {} (newly {} since the root LP)", self.p.nint, gfix.saturating_sub(self.root_fixed0));
                eprintln!("MIP: new incumbent {:.10e} at node {} ({:.2}s) [{other}{}]", z, self.nodes, self.start.elapsed().as_secs_f64(), if self.heur_label.is_empty() { "tree" } else { self.heur_label });
            }
        }
        better
    }

    /// 暫定解 `x` (目的値 `z`) の整数列を固定し、連続列を元の境界に戻した LP を解いて、連続列の値を取り替える
    /// (実行可能で目的値が良くなるときだけ)。ヒューリスティクスの解の連続列は最適とは限らない: 目的値の上限を制約にする
    /// Proximity Search の解は、連続の目的の列が上限に張り付いたまま返る (bppc4-08 は 0-1 の割り当てでは z = 57 にできる
    /// 解が 59.994 のまま暫定解になり、枝刈りもその値で行っていた)。`ENOMOTO_MIP_NO_INC_LP` で使わない。
    /// 今の LP の双対値で `x` の最適性が示せるなら解かない ([`Self::continuous_optimal_by_duals`])。解き直しの反復は
    /// LP の反復の累計に入れない (累計を基準にするヒューリスティクスの予算などを動かさないため)
    fn resolve_continuous(&mut self, x: &mut Vec<f64>, z: &mut f64) {
        if self.p.is_int.iter().all(|&b| b) || self.time_up() || env_str!("ENOMOTO_MIP_NO_INC_LP").is_some() {
            return;
        }
        // Feasibility Pump は LP の費用を距離に置き換えたまま解を試すので、元の費用に戻して解く
        let cost = self.lp.costs();
        if env_str!("ENOMOTO_MIP_INC_LP_NO_DUAL_CHECK").is_none() && self.continuous_optimal_by_duals(x, &cost) {
            return;
        }
        let saved = self.lp.save_state();
        let it0 = self.lp.total_iterations();
        let swap = cost != self.p.cost;
        if swap {
            self.lp.set_costs(&self.p.cost);
        }
        for j in 0..self.p.n {
            let (lo, up) = if self.p.is_int[j] { (x[j], x[j]) } else { (self.p.col_lo[j], self.p.col_up[j]) };
            self.lp.set_col_bounds(j, lo, up);
        }
        // `ENOMOTO_MIP_INC_LP_CAP` (試作、既定では使わない): この LP の時間の上限を全体の `ENOMOTO_T_MIP_INC_LP_TIME_FRAC`
        // (0.02、最低 1 秒) にする。uccase12 で DUAL_UB + CS_OBJ_PIVOT のとき、根の LP の直後に FJ の解 (865106) を受け取り、
        // この LP が 8.4 秒から 56 秒まで止まらず、根のカット・ヒューリスティクスが 1 つも回らなかった
        // `ENOMOTO_MIP_FJ_P3` のときも使う (FJ の解は境界を締めた問題のもので、uccase12 では同じくこの LP が長引く)
        let deadline = if (env_str!("ENOMOTO_MIP_INC_LP_CAP").is_some() || super::fj_p3_on()) && self.params.time_limit.is_finite() {
            let cap = (tunable!("ENOMOTO_T_MIP_INC_LP_TIME_FRAC", 0.02, f64) * self.params.time_limit).max(1.0);
            let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(cap);
            Some(self.deadline.map_or(d, |g| g.min(d)))
        } else {
            self.deadline
        };
        let st = self.lp.solve(&SolveLimits { deadline, ..Default::default() });
        if self.params.verbose && st != LpStatus::Optimal {
            eprintln!("MIP: fixed-integer LP: {st:?} ({} iters)", self.lp.total_iterations() - it0);
        }
        if st == LpStatus::Optimal {
            let mut y = self.lp.col_values();
            for j in 0..self.p.n {
                if self.p.is_int[j] {
                    y[j] = x[j];
                }
            }
            let zy = self.p.objective(&y);
            if zy < *z - 1e-9 * z.abs().max(1.0) && self.p.is_feasible(&y, FEASTOL) {
                if self.params.verbose {
                    eprintln!("MIP: fixed-integer LP improved incumbent {z:.10e} -> {zy:.10e}");
                }
                *x = y;
                *z = zy;
            }
        }
        if swap {
            self.lp.set_costs(&cost);
        }
        self.lp.restore_state(&saved);
        self.lp.set_total_iterations(it0);
    }

    /// 今の LP の双対値 `y` (費用 `lp_cost` で解いたもの) を使い、整数列を `x` に固定した LP で `x` が最適なことを
    /// 弱双対で示せるか。どの `y` についても、被約費用 `d = c - A^T y` (`c` は元の費用) から
    /// `L(y) = Σ_行 (y_i >= 0 なら y_i lo_i、負なら y_i up_i) + Σ_整数列 d_j x_j + Σ_連続列 (d_j >= 0 なら d_j lo_j、負なら d_j up_j)`
    /// (連続列は大域の境界) は固定した LP の下界なので、`c x - L(y)` が相対 1e-9 以下なら `x` の連続部分は (その誤差の範囲で)
    /// 最適。ノードの LP 解をそのまま暫定解にしたときは成り立つ。Proximity Search の解のように連続列が上限などに
    /// 張り付いて動かせる解では成り立たない
    fn continuous_optimal_by_duals(&self, x: &[f64], lp_cost: &[f64]) -> bool {
        const ZERO: f64 = 1e-11;
        let y = self.lp.row_duals();
        let mut d = self.lp.reduced_costs();
        if d.len() < self.p.n {
            return false;
        }
        let mut lower = 0.0;
        for (i, &yi) in y.iter().enumerate() {
            if yi.abs() <= ZERO {
                continue;
            }
            let (lo, up) = self.lp.row_bounds(i);
            let b = if yi > 0.0 { lo } else { up };
            if !b.is_finite() {
                return false;
            }
            lower += yi * b;
        }
        let mut cx = 0.0;
        for j in 0..self.p.n {
            d[j] += self.p.cost[j] - lp_cost[j];
            cx += self.p.cost[j] * x[j];
            if self.p.is_int[j] {
                lower += d[j] * x[j];
            } else if d[j].abs() > ZERO {
                let b = if d[j] > 0.0 { self.p.col_lo[j] } else { self.p.col_up[j] };
                if !b.is_finite() {
                    return false;
                }
                lower += d[j] * b;
            }
        }
        let ok = cx - lower <= 1e-9 * cx.abs().max(1.0);
        if self.params.verbose {
            eprintln!("MIP: fixed-integer LP dual check: c x {cx:.10e}, dual bound {lower:.10e} -> {}", if ok { "skip" } else { "solve" });
        }
        ok
    }

    /// LP 解の整数列を丸めた点を試し、行で少しはみ出るなら整数を固定して連続部分を LP で解き直す。
    pub(super) fn try_lp_solution(&mut self) -> bool {
        if self.try_incumbent(self.lp.col_values()) {
            return true;
        }
        if !self.p.is_int.iter().any(|&b| !b) {
            return false;
        }
        // 整数を固定して連続変数を解き直す (`try_incumbent` が偽なら LP には触れていないので、LP の値は上と同じ)
        let saved = self.lp.save_state();
        for j in 0..self.p.n {
            if self.p.is_int[j] {
                let v = self.lp.col_value(j).round();
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
        let user = crate::log::enabled();
        if !self.params.verbose && !user {
            return;
        }
        // 利用者向けのログだけなら 5 秒ごと (開発用の経過は 2 秒ごと)
        let every = if self.params.verbose { 2.0 } else { 5.0 };
        if !force && self.last_log.elapsed().as_secs_f64() < every {
            return;
        }
        self.last_log = Instant::now();
        let ub = self.incumbent.as_ref().map(|(z, _)| *z).unwrap_or(f64::INFINITY);
        if user {
            self.user_progress_line(ub);
        }
        if !self.params.verbose {
            return;
        }
        eprintln!(
            "MIP: {:8.2}s nodes {:8} open {:7} lb {:.8e} ub {:.8e} lp_iters {} sb_iters {} sb_lps {} sb_secs {:.2} node_iters {} (first {}, lps {}) heur_iters {} dive_iters {} lp_rows {} node_cut_secs {:.2}{}{}",
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
            self.lp.num_rows(),
            self.node_cut_secs,
            if self.fc_node_sep {
                format!(" fc_sep calls {} gain_ema {:.4} prunes {} extra {}", self.fc_stats.0, self.fc_stats.1, self.fc_stats.2, self.fc_stats.3)
            } else {
                String::new()
            },
            // 木を分け合うとき: 共有の待ち行列の大きさと待っているスレッドの数 (符号つきで出す。下に回っていれば負)
            self.pool.as_ref().map_or(String::new(), |pl| {
                use std::sync::atomic::Ordering;
                format!(" pool_len {} pool_idle {}", pl.len.load(Ordering::Relaxed), pl.idle.load(Ordering::Relaxed) as isize)
            }),
        );
    }

    /// 処理を始めたノードの下界を記録する。木を分け合うときは、このスレッドの下界 (処理中のノードと自分の待ち行列の
    /// 最小) を共有の領域にも書く ([`PoolInner::lbs`])。
    fn note_current_lb(&mut self, node_lb: f64) {
        self.cur_lb = node_lb;
        if let Some(pool) = &self.pool {
            let v = node_lb.min(self.queue.best_lower_bound());
            let t = self.pool_tid as usize;
            if let Some(sl) = pool.slots.get(t) {
                use std::sync::atomic::Ordering;
                sl.lb.store(v.to_bits(), Ordering::Relaxed);
                sl.open.store(self.queue.len(), Ordering::Relaxed);
                if self.snap.is_some() {
                    sl.nodes.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    /// 探索全体の下界 (最小化形): 自分の待ち行列・処理中のノード、木を分け合うときは共有の待ち行列と各スレッドの下界の最小。
    pub(super) fn global_lower_bound(&self) -> f64 {
        let mut lb = self.queue.best_lower_bound().min(self.cur_lb);
        if let Some(pool) = &self.pool {
            let g = pool.inner.lock().unwrap();
            lb = lb.min(g.nodes.min_lower_bound());
            for sl in &pool.slots {
                lb = lb.min(f64::from_bits(sl.lb.load(std::sync::atomic::Ordering::Relaxed)));
            }
        }
        lb
    }

    /// 探索全体の (処理したノード数, 未処理のノード数)。木を分け合うときは他のスレッドの分と共有の待ち行列も足す。
    fn global_node_counts(&self) -> (u64, usize) {
        let (mut done, mut open) = (self.nodes, self.queue.len());
        if let Some(pool) = &self.pool {
            use std::sync::atomic::Ordering;
            open += pool.inner.lock().unwrap().nodes.len();
            let t = self.pool_tid as usize;
            done += pool.slots.iter().map(|sl| sl.nodes.load(Ordering::Relaxed)).sum::<u64>();
            open += pool.slots.iter().enumerate().filter(|&(i, _)| i != t).map(|(_, sl)| sl.open.load(Ordering::Relaxed)).sum::<usize>();
        }
        (done, open)
    }

    /// 利用者向けのログの探索の経過の 1 行 (時間、処理したノード数、待ち行列のノード数、限界、暫定解、ギャップ)。
    /// 目的値は元の向き (最大化なら限界は上界)。
    fn user_progress_line(&mut self, ub: f64) {
        let s = self.p.sense_sign;
        if !self.user_log_header {
            self.user_log_header = true;
            user_log!("{:>9} {:>10} {:>9} {:>17} {:>17} {:>9}", "time", "nodes", "open", "best bound", "incumbent", "gap");
        }
        let lb = self.final_lb.unwrap_or_else(|| self.global_lower_bound()).min(ub);
        let fmt = |v: f64| if v.is_finite() { format!("{:.9e}", s * v) } else { "-".to_string() };
        let gap = if ub.is_finite() && lb.is_finite() { format!("{:.2}%", 100.0 * (ub - lb).max(0.0) / ub.abs().max(1.0)) } else { "-".to_string() };
        let (done, open) = if self.final_lb.is_some() { (self.nodes + self.worker_totals.0, 0) } else { self.global_node_counts() };
        user_log!("{:>8.1}s {:>10} {:>9} {:>17} {:>17} {:>9}", self.start.elapsed().as_secs_f64(), done, open, fmt(lb), fmt(ub), gap);
        // 診断用 (`ENOMOTO_MIP_DEBUG_SLOTS`): 全体の下界がどこにあるか (共有の待ち行列・各スレッドの下界と待ち行列のノード数)
        if env_str!("ENOMOTO_MIP_DEBUG_SLOTS").is_some() {
            if let Some(pool) = &self.pool {
                use std::sync::atomic::Ordering;
                let (gmin, glen) = {
                    let g = pool.inner.lock().unwrap();
                    (g.nodes.min_lower_bound(), g.nodes.len())
                };
                let sl: Vec<String> = pool
                    .slots
                    .iter()
                    .map(|sl| format!("{:.4}/{}/{}", f64::from_bits(sl.lb.load(Ordering::Relaxed)), sl.open.load(Ordering::Relaxed), sl.nodes.load(Ordering::Relaxed)))
                    .collect();
                eprintln!("SLOTS shared {gmin:.4}/{glen} own(tid {}) {:.4}/{} slots(lb/open/nodes) {}", self.pool_tid, self.queue.best_lower_bound().min(self.cur_lb), self.queue.len(), sl.join(" "));
            }
        }
    }

    fn run(&mut self) -> MipResult {
        // `ENOMOTO_MIP_SUBMIP_NO_SB`: サブ MIP では強分岐をしない (HiGHS のサブ MIP の `mip_pscost_minreliable = 0` と同じ)。
        // 30n20b8 の根の RENS のサブ MIP は、最初の 26 ノードで強分岐に 2.2 秒 (250 回の LP) 使う
        // 暫定解がない (打ち切り値が無限の) サブ MIP だけ: すべてのサブ MIP に使うと RINS などの改善のサブ MIP が弱まり、
        // 40 問 (300 秒) で timtab1 の主ギャップ 0.059 -> 0.121、bppc4-08 0.036 -> 0.070、binkar10_1 23.7 -> 35.4 秒
        if self.params.submip && ((self.params.cutoff.is_infinite() && env_str!("ENOMOTO_MIP_SUBMIP_NO_SB").is_some()) || super::submip_highs('s')) {
            self.pc.min_reliable = 0;
        }
        // 木の写しから解く: 写しを作ったときの LP のカットの行とカットプールを入れる
        if let Some(snap) = self.snap.clone() {
            for (c, r) in &snap.pool {
                self.add_to_pool(c, *r);
            }
            if !snap.rows.is_empty() {
                self.add_cut_rows(&snap.rows);
            }
        }
        // `ENOMOTO_MIP_RESTART_HCM_KEEP` (試作): 再スタート前の求解が選んだ HiGHS 流のカット管理を引き継ぐ (下の readd の判定にも効く)
        if super::RESTART_HCM.with(|f| f.replace(false)) && !self.params.submip {
            self.hcm_auto = true;
            if self.params.verbose {
                eprintln!("MIP: HiGHS cut management kept from before the restart");
            }
        }
        // 再スタート前の暫定解 (実行可能性は try_incumbent が確かめる)
        if let Some(cuts) = super::RESTART_CUTS.with(|s| s.borrow_mut().take()) {
            let n0 = self.cut_pool.len();
            for (c, r) in &cuts {
                if c.iter().all(|&(j, _)| j < self.p.n) {
                    self.add_to_pool(c, *r);
                }
            }
            // `ENOMOTO_MIP_RESTART_CUTS_READD`: 再スタート前に効いていたカットを、根の LP を解く前にカットの行として加える
            // (問題の行ではないので木の中で年齢により外せる。プールにもあるので外れても違反すれば戻る)
            let readd_mark = super::RESTART_CUTS_READD.with(|f| f.replace(false));
            if (env_str!("ENOMOTO_MIP_RESTART_CUTS_READD").is_some() || self.highs_cutmgmt() || self.watch.is_some() || super::gr_restart_readd_for(self.p) || readd_mark) && !self.params.submip {
                let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = cuts.iter().filter(|(c, _)| c.iter().all(|&(j, _)| j < self.p.n)).map(|(c, r)| (c.clone(), f64::NEG_INFINITY, *r)).collect();
                if !rows.is_empty() {
                    self.add_cut_rows(&rows);
                }
                if self.params.verbose {
                    eprintln!("MIP: {} cuts from before the restart added back to the LP as cut rows", rows.len());
                }
            } else if self.params.verbose {
                eprintln!("MIP: {} cuts from before the restart put in the cut pool", self.cut_pool.len() - n0);
            }
        }
        if let Some(x) = REF_SOL.with(|s| s.borrow_mut().take()) {
            if x.len() == self.p.n {
                self.ref_sol = Some(x);
            }
        }
        if let Some(x) = START_SOL.with(|s| s.borrow_mut().take()) {
            if x.len() == self.p.n {
                self.accepting_start = true;
                let ok = self.try_incumbent(x);
                self.accepting_start = false;
                if self.params.verbose || env_str!("ENOMOTO_MIP_START_FILE").is_some() {
                    eprintln!("MIP: start solution from before the restart: accepted {ok}");
                }
            }
        }
        // LP を使わない局所探索 (Feasibility Jump) で最初の実行可能解を探す。
        {
            let nnz = self.p.nnz;
            let cap = if self.params.time_limit.is_finite() { (0.05 * self.params.time_limit).min(5.0) } else { 5.0 };
            let effort = (50 * nnz as u64).clamp(100_000, 50_000_000);
            // 診断用 (`ENOMOTO_MIP_DEBUG_FJ_TRIALS=N`): 根の前に FJ2 を種を変えて N 回 (各回は手間の上限まで) 回し、解を見つけた回数と
            // 最小の違反行数を出す。FJ2 は決定的で、1 回の成否は行の並びなどで変わるので、前処理の形の比べ方に使う
            if let Some(k) = env_str!("ENOMOTO_MIP_DEBUG_FJ_TRIALS").and_then(|v| v.parse::<u64>().ok()).filter(|_| !self.params.submip && self.snap.is_none()) {
                let (pc, lo, up) = match &self.orig {
                    Some(o) => ((**o).clone(), o.col_lo.clone(), o.col_up.clone()),
                    None => (self.p.clone(), self.dom.lo.clone(), self.dom.up.clone()),
                };
                let mut found = 0;
                for t in 0..k {
                    if super::fj::fj2_search(&pc, &lo, &up, 30.0, 0x1234_5678u64.wrapping_add(t.wrapping_mul(0x9E37_79B9_7F4A_7C15)), None).is_some() {
                        found += 1;
                    }
                }
                eprintln!("MIP: FJ trials: found {found}/{k}");
            }
            // 診断用 (`ENOMOTO_MIP_DEBUG_FJ_VS_LMIP=秒`): 根の前に、FJ の問題 (FJ_P3) で FJ と Local-MIP + 連鎖をそれぞれ単独で回し、
            // 最初の実行可能解の時刻と値、時間内の最良の値を出す (`FJVSLM:` の行。目的値は FJ の問題の空間)
            if let Some(tl) = env_str!("ENOMOTO_MIP_DEBUG_FJ_VS_LMIP").and_then(|v| v.parse::<f64>().ok()).filter(|_| !self.params.submip && self.snap.is_none()) {
                let pc = match &self.orig {
                    Some(o) => (**o).clone(),
                    None => self.p.clone(),
                };
                let q = if super::fj_p3_on() { super::fj_p3_problem(&pc).map(|t| t.0).unwrap_or(pc) } else { pc };
                let never = std::sync::atomic::AtomicBool::new(false);
                let seed = self.rng ^ 0x9E37_79B9_7F4A_7C15;
                let obj = |x: &Option<Vec<f64>>| x.as_ref().map_or("none".to_string(), |x| format!("{:.10e}", q.objective(x)));
                crate::set_fj_p3_force(super::fj_p3_on());
                let t = Instant::now();
                let r1 = super::fj::fj2_search(&q, &q.col_lo, &q.col_up, tl, seed, Some(&never));
                let t1 = t.elapsed().as_secs_f64();
                let t = Instant::now();
                let r2 = super::heuristics::fj_search(&q, &q.col_lo, &q.col_up, effort, tl, seed, Some(&never));
                let t2 = t.elapsed().as_secs_f64();
                crate::set_fj_p3_force(false);
                let first: std::sync::Mutex<Option<(f64, f64)>> = Default::default();
                let t = Instant::now();
                let on_sol = |x: &[f64]| {
                    let mut f = first.lock().unwrap();
                    if f.is_none() {
                        *f = Some((t.elapsed().as_secs_f64(), q.objective(x)));
                    }
                };
                let lr = super::localmip::local_mip_opts(&q, &q.col_lo, &q.col_up, tl, seed ^ 0x6A09_E667_F3BC_C909, None, &on_sol, true);
                let f = *first.lock().unwrap();
                eprintln!(
                    "FJVSLM: fj1 found {} t {t1:.3} | fj ({tl}s) best {} t {t2:.3} | lm first {} t {} best {} least_viol {} steps {}",
                    obj(&r1),
                    obj(&r2),
                    f.map_or("none".to_string(), |v| format!("{:.10e}", v.1)),
                    f.map_or("-".to_string(), |v| format!("{:.3}", v.0)),
                    obj(&lr.best),
                    lr.least_viol,
                    lr.steps
                );
            }
            if self.snap.is_some() || env_str!("ENOMOTO_MIP_DEBUG_NO_FJ").is_some() {
                // 木の写しから解く: 解探しはしない (`ENOMOTO_MIP_DEBUG_NO_FJ` は診断用に FJ を使わない)
            } else if !self.params.submip && !proof_only() && mip_threads() > 1 {
                // 並列モード: 別スレッドで根の LP・切除平面と同時に回す (根のヒューリスティクスの前に受け取る)
                // 対称性の行・固定を加える前の問題で探す (見つけた解は try_incumbent で並べ替える)
                let (pc, lo, up) = match &self.orig {
                    // `ENOMOTO_MIP_FJ_RESTART_BOUNDS` (試作): 再スタートの後は、FJ・Local-MIP の問題 (対称性の行を足す前の、最初の求解の問題で、
                    // 再スタートでは作り直さない) の列の境界を、今の大域的な境界 (再スタートで固定・締めた境界) と重ねて狭める。前処理は列の
                    // 番号を保つので列は同じ。重ねると空になる列はそのまま (対称性の固定などで食い違うときのため)
                    Some(o) if self.params.restarts > 0 && o.n == self.p.n && env_str!("ENOMOTO_MIP_FJ_RESTART_BOUNDS").is_some() => {
                        let mut q = (**o).clone();
                        let mut k = 0usize;
                        for j in 0..q.n {
                            let (l, u) = (q.col_lo[j].max(self.dom.global_lo[j]), q.col_up[j].min(self.dom.global_up[j]));
                            if l <= u && (l > q.col_lo[j] || u < q.col_up[j]) {
                                q.col_lo[j] = l;
                                q.col_up[j] = u;
                                k += 1;
                            }
                        }
                        if self.params.verbose {
                            eprintln!("MIP: FJ problem after the restart: {k} column bounds tightened to the global bounds, free columns {}", (0..q.n).filter(|&j| q.col_lo[j] < q.col_up[j]).count());
                        }
                        let (lo, up) = (q.col_lo.clone(), q.col_up.clone());
                        (std::sync::Arc::new(q), lo, up)
                    }
                    Some(o) => (std::sync::Arc::new((**o).clone()), o.col_lo.clone(), o.col_up.clone()),
                    None => (std::sync::Arc::new(self.p.clone()), self.dom.lo.clone(), self.dom.up.clone()),
                };
                let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let stop2 = stop.clone();
                // `ENOMOTO_T_MIP_FJ_SEED` (診断用): FJ の種をずらす (FJ の当たり外れが種の運かを見る)
                let seed = (self.rng ^ 0x9E37_79B9_7F4A_7C15).wrapping_add(tunable!("ENOMOTO_T_MIP_FJ_SEED", 0u64, u64).wrapping_mul(0xD1B5_4A32_D192_ED03));
                // 実行可能性問題では FJ を根のカットのループの終わりまで (時間上限の半分まで) 回し続ける
                let cap = if super::heuristics::fj_feas_extend(&pc) && self.params.time_limit.is_finite() { tunable!("ENOMOTO_T_MIP_FJ_FEAS_FRAC", 0.5, f64) * self.params.time_limit } else { cap };
                // `ENOMOTO_MIP_FJ_P3` (既定、`ENOMOTO_MIP_NO_FJ_P3` で止める): FJ だけ、p5s の前処理の変更をかけ直した問題で探す ([`super::fj_p3_problem`])
                let fj_p3 = super::fj_p3_on();
                // `ENOMOTO_MIP_LOCALMIP` (試作): 1 = FJ と並べて別スレッドで Local-MIP 風の局所探索 ([`super::localmip`]) を回す、
                // 2 = FJ の代わりに Local-MIP だけ。時間の上限は `ENOMOTO_T_MIP_LOCALMIP_TIME_FRAC` (0.1) × 時間上限、最大 10 秒。
                // 結果は根のカットのループの各ラウンドと木の各ノードで (待たずに) 受け取る
                let lmip = env_str!("ENOMOTO_MIP_LOCALMIP").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
                if lmip > 0 {
                    let lcap = if self.params.time_limit.is_finite() { (tunable!("ENOMOTO_T_MIP_LOCALMIP_TIME_FRAC", 0.1, f64) * self.params.time_limit).min(10.0) } else { 10.0 };
                    let t_start = self.start;
                    let pl = pc.clone();
                    let (llo, lup) = (lo.clone(), up.clone());
                    let lstop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                    let lstop2 = lstop.clone();
                    let lseed = seed ^ 0x6A09_E667_F3BC_C909;
                    let mailbox: std::sync::Arc<std::sync::Mutex<Vec<Vec<f64>>>> = Default::default();
                    let mb2 = mailbox.clone();
                    self.lmip_mailbox = Some(mailbox);
                    let hl = std::thread::spawn(move || {
                        let post_sc = if fj_p3 { super::fj_p3_problem(&pl) } else { None };
                        let run = |q: &super::problem::MipProblem, lo: &[f64], up: &[f64]| {
                            let t = std::time::Instant::now();
                            // 良くなった解はその場で受け渡し箱に入れる (元の問題の空間に戻して)
                            let on_sol = |x: &[f64]| {
                                let y = match &post_sc {
                                    Some((_, post, sc)) => super::fj_p3_solution(x.to_vec(), post, sc),
                                    None => x.to_vec(),
                                };
                                mb2.lock().unwrap().push(y);
                            };
                            let lr = super::localmip::local_mip(q, lo, up, lcap, lseed, Some(&lstop2), &on_sol);
                            eprintln!(
                                "MIP: Local-MIP: {} steps, least violated rows {}, pair/chain moves {}, found {} (first at {:?}s of {:.2}s), started at {:.2}s",
                                lr.steps,
                                lr.least_viol,
                                lr.pair_moves,
                                lr.best.as_ref().map_or("none".to_string(), |x| format!("{:.6e}", q.objective(x))),
                                lr.first_time.map(|v| (v * 100.0).round() / 100.0),
                                t.elapsed().as_secs_f64(),
                                (t - t_start).as_secs_f64()
                            );
                            lr.best
                        };
                        match &post_sc {
                            Some((q, post, sc)) => run(q, &q.col_lo, &q.col_up).map(|x| super::fj_p3_solution(x, post, sc)),
                            None => run(&pl, &llo, &lup),
                        }
                    });
                    self.lmip_thread = Some((hl, lstop));
                }
                // `ENOMOTO_MIP_FJ_LOCALMIP` (試作): FJ のスレッドで、FJ2 が続けて解なしなら Local-MIP + 連鎖の手に切り替える
                // ([`super::heuristics::fj_search_fb`])。Local-MIP は時間の上限 (`ENOMOTO_T_MIP_LOCALMIP_TIME_FRAC` × 時間上限、最大 10 秒) か、
                // FJ のスレッドを止める合図 (根のカットのループの終わり) まで回り、良くなった解はその場で受け渡し箱に入れる
                let fjl = env_str!("ENOMOTO_MIP_FJ_LOCALMIP").is_some() && lmip == 0;
                // `ENOMOTO_MIP_FJ_LMIP_MIX` (試作): FJ のスレッドで FJ と Local-MIP + 連鎖に時間を分け合う ([`super::heuristics::fj_lmip_mix`])。
                // 時間の上限は Local-MIP と同じ。実行可能性問題 (目的が全部 0) では今までどおり
                let mix = fj_lmip_mix_on() && lmip == 0 && !fjl;
                let fjl_cap = if self.params.time_limit.is_finite() { (tunable!("ENOMOTO_T_MIP_LOCALMIP_TIME_FRAC", 0.1, f64) * self.params.time_limit).min(10.0) } else { 10.0 };
                let fjl_box: std::sync::Arc<std::sync::Mutex<Vec<Vec<f64>>>> = Default::default();
                if fjl || mix {
                    self.lmip_mailbox = Some(fjl_box.clone());
                }
                let t_start = self.start;
                // `ENOMOTO_MIP_FJ_NOSOL_EXTEND` (試作): 解が出ない間は、FJ の時間の予算 (5%・5 秒まで) を過ぎても、根のカットのループの
                // 終わり (止める合図) か `ENOMOTO_T_MIP_FJ_NOSOL_FRAC` (0.25) × 時間上限まで種を変えた FJ2 を続ける。主のスレッドが
                // FJ のスレッド以外から暫定解を得たら (`fj_has_inc`) 延長をやめる。`=2` は混ぜる形 (FJ_LMIP_MIX) で、FJ のスレッドの解しか
                // ない間は解があっても同じ秒数まで続ける (Local-MIP の改善)
                let nosol_cap = if env_def!("ENOMOTO_MIP_FJ_NOSOL_EXTEND", "2").is_some() && self.params.time_limit.is_finite() {
                    tunable!("ENOMOTO_T_MIP_FJ_NOSOL_FRAC", 0.25, f64) * self.params.time_limit - self.start.elapsed().as_secs_f64()
                } else {
                    0.0
                };
                let has_inc = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(self.incumbent.is_some() && !self.inc_only_fj));
                if nosol_cap > 0.0 {
                    self.fj_has_inc = Some(has_inc.clone());
                }
                let h = std::thread::spawn(move || {
                    if lmip == 2 {
                        return None;
                    }
                    super::heuristics::FJ_NOSOL_CAP.with(|c| c.set(nosol_cap.max(0.0)));
                    super::heuristics::FJ_HAS_INC.with(|f| *f.borrow_mut() = Some(has_inc));
                    // Local-MIP に切り替える探索。`map` で解を `pc` の空間に戻して受け渡し箱に入れる
                    let fb_run = |q: &super::problem::MipProblem, lo: &[f64], up: &[f64], map: &dyn Fn(Vec<f64>) -> Vec<f64>| -> Option<Vec<f64>> {
                        let t = std::time::Instant::now();
                        let on_sol = |x: &[f64]| {
                            fjl_box.lock().unwrap().push(map(x.to_vec()));
                        };
                        let lr = super::localmip::local_mip_opts(q, lo, up, fjl_cap, seed ^ 0x6A09_E667_F3BC_C909, Some(&stop2), &on_sol, true);
                        eprintln!(
                            "MIP: Local-MIP (in the FJ thread): {} steps, least violated rows {}, chain moves {}, found {} (first at {:?}s of {:.2}s), started at {:.2}s",
                            lr.steps,
                            lr.least_viol,
                            lr.pair_moves,
                            lr.best.as_ref().map_or("none".to_string(), |x| format!("{:.6e}", q.objective(x))),
                            lr.first_time.map(|v| (v * 100.0).round() / 100.0),
                            t.elapsed().as_secs_f64(),
                            (t - t_start).as_secs_f64()
                        );
                        lr.best
                    };
                    if fj_p3 {
                        if let Some((q, post, sc)) = super::fj_p3_problem(&pc) {
                            let map = |x: Vec<f64>| super::fj_p3_solution(x, &post, &sc);
                            let fb = |q2: &super::problem::MipProblem, lo: &[f64], up: &[f64]| {
                                crate::set_fj_p3_force(false);
                                fb_run(q2, lo, up, &map)
                            };
                            crate::set_fj_p3_force(true);
                            if mix && !super::heuristics::fj_feas_extend(&q) {
                                let on_sol = |x: &[f64]| fjl_box.lock().unwrap().push(map(x.to_vec()));
                                let r = super::heuristics::fj_lmip_mix(&q, &q.col_lo, &q.col_up, fjl_cap.max(cap), seed, &stop2, &on_sol);
                                crate::set_fj_p3_force(false);
                                return r.map(map);
                            }
                            let r = super::heuristics::fj_search_fb(&q, &q.col_lo, &q.col_up, effort, cap, seed, Some(&stop2), if fjl { Some(&fb) } else { None });
                            crate::set_fj_p3_force(false);
                            return r.map(map);
                        }
                    }
                    if mix && !super::heuristics::fj_feas_extend(&pc) {
                        let on_sol = |x: &[f64]| fjl_box.lock().unwrap().push(x.to_vec());
                        return super::heuristics::fj_lmip_mix(&pc, &lo, &up, fjl_cap.max(cap), seed, &stop2, &on_sol);
                    }
                    let fb = |q2: &super::problem::MipProblem, lo: &[f64], up: &[f64]| fb_run(q2, lo, up, &|x| x);
                    super::heuristics::fj_search_fb(&pc, &lo, &up, effort, cap, seed, Some(&stop2), if fjl { Some(&fb) } else { None })
                });
                self.fj_thread = Some((h, stop));
            } else if !self.params.submip && !proof_only() && self.feasibility_jump(effort, cap) && self.params.verbose {
                eprintln!("MIP: feasibility jump found a solution ({:.2}s)", self.start.elapsed().as_secs_f64());
            }
        }
        // 木を分け合う従: 準備の間に主が終わっていたら、根の LP を解かずに抜ける
        if self.pool.as_ref().is_some_and(|p| p.stop.load(std::sync::atomic::Ordering::Relaxed)) {
            return self.finish(MipStatus::NotSolved, f64::INFINITY);
        }
        // 根の LP
        self.root_fixed0 = (0..self.p.n).filter(|&j| self.p.is_int[j] && self.dom.global_lo[j] == self.dom.global_up[j]).count();
        // 基底のない求解なので、`threads` >= 2 なら二段解法と内点法 + クロスオーバーで同時に解かれる
        // (`TwoStageLp::solve`、`crate::simplex::mip_lp::set_cold_race_threads`)
        if self.params.submip {
            if let Some(col) = SUBMIP_BASIS.with(|s| s.borrow_mut().take()) {
                if col.len() == self.p.n {
                    self.lp.set_basis(&super::lp::Basis { col, row: Vec::new() });
                }
            }
        }
        let mut root_warm_dse_once = false;
        if !self.params.submip && self.snap.is_none() {
            let rb = ROOT_BASIS.with(|s| s.borrow_mut().take());
            // 行の状態は、前処理の LP の行と根の LP の始めの行が同じ (指紋が合う) ときだけ使う (`ENOMOTO_MIP_LP_PROBE_ROW_BASIS`。
            // 合わなければ列の状態だけにし、DSE 重みも捨てる)
            let fp = ROOT_BASIS_ROWS.with(|s| s.borrow_mut().take());
            let rb = rb.map(|mut b| {
                if !b.row.is_empty() && (b.row.len() > self.p.m || fp.is_none_or(|f| super::rows_fingerprint(&self.p.rows[..b.row.len()]) != f)) {
                    if self.params.verbose {
                        eprintln!("MIP: root LP: the rows changed after LP probing; the row statuses are not used");
                    }
                    b.row.clear();
                    ROOT_DSE.with(|s| *s.borrow_mut() = None);
                }
                b
            });
            if self.params.verbose {
                eprintln!(
                    "MIP: root LP basis from the presolve LP probing: {}",
                    match &rb {
                        Some(b) if b.col.len() == self.p.n && b.row.len() == self.lp.num_rows() => "used (with rows)",
                        Some(b) if b.col.len() == self.p.n && !b.row.is_empty() && b.row.len() < self.lp.num_rows() => "used (with rows; the added rows basic)",
                        Some(b) if b.col.len() == self.p.n => "used (columns only)",
                        Some(_) => "size mismatch",
                        None => "none",
                    }
                );
            }
            if let Some(mut b) = rb {
                if b.col.len() == self.p.n {
                    // 行の状態 (`ENOMOTO_MIP_LP_PROBE_FINISH`。既定は列の状態だけ): 前処理の後に末尾に足した行 (対称性の
                    // lex-leader の行など) は論理変数を基底に入れる。行が減っていたら使わない
                    if b.row.len() > self.lp.num_rows() {
                        b.row.clear();
                    } else if !b.row.is_empty() {
                        b.row.resize(self.lp.num_rows(), super::lp::VarStatus::Basic);
                    }
                    self.lp.set_basis(&b);
                    // DSE 重みも引き継ぐ (根の LP の 1 回だけ。重みなしで始めると、カットのループの LP の反復が増えた)
                    if let Some(w) = ROOT_DSE.with(|s| s.borrow_mut().take()) {
                        self.lp.set_dse_vars(&w);
                        root_warm_dse_once = true;
                    }
                }
            }
        }
        ROOT_DSE.with(|s| *s.borrow_mut() = None);
        // `ENOMOTO_MIP_RESTART_BASIS` (試作): 再スタートの後の根の LP は、再スタートの直前の LP の基底から解き始める
        let restart_basis_used = if !self.params.submip && self.snap.is_none() && self.params.restarts > 0 {
            match RESTART_BASIS.with(|s| s.borrow_mut().take()).and_then(|saved| restart_basis_for(&self.lp, self.p.n, saved)) {
                Some((b, matched)) => {
                    if self.params.verbose {
                        eprintln!("MIP: root LP basis from before the restart: {matched} of {} rows matched", self.lp.num_rows());
                    }
                    self.lp.set_basis(&b);
                    true
                }
                None => false,
            }
        } else {
            false
        };
        let root_iters0 = self.lp.total_iterations();
        if self.snap.is_some() && (CUT_JOB.with(|j| j.borrow().is_some()) || DIVE_ONLY.with(|d| d.get()).0 == 5) {
            if let Some(b) = BG_ROOT_BASIS.with(|s| s.borrow_mut().take()) {
                if b.col.len() == self.p.n && b.row.len() == self.lp.num_rows() {
                    self.lp.set_basis(&b);
                }
            }
        }
        // `ENOMOTO_MIP_TREE_ROOT_BASIS` (試作): 木を分け合う従は主の LP の基底から解き始める
        if self.pool.is_some() && tree_root_basis() {
            if let Some(b) = self.snap.as_ref().and_then(|s| s.root_basis.clone()) {
                if b.col.len() == self.p.n && b.row.len() == self.lp.num_rows() {
                    self.lp.set_basis(&b);
                }
            }
        }
        if self.params.verbose && self.snap.is_none() && !self.params.submip {
            eprintln!("MIP: solving the root LP ({:.2}s)", self.start.elapsed().as_secs_f64());
        }
        if root_warm_dse_once {
            crate::simplex::slope_intercept_dual::WARM_DSE_ONCE.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let mut root_st = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
        if root_warm_dse_once {
            crate::simplex::slope_intercept_dual::WARM_DSE_ONCE.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        if restart_basis_used && self.params.verbose {
            eprintln!("MIP: root LP after the restart: {root_st:?}, {} iters ({:.2}s)", self.lp.total_iterations() - root_iters0, self.start.elapsed().as_secs_f64());
        }
        // 診断用 (`ENOMOTO_MIP_DEBUG_RESTART_CUT_USE`): 再スタートの後の根の LP 解で、カットプールのカットと LP のカットの行が
        // 破られている・等号で効いている・緩んでいる数
        if self.params.restarts > 0 && env_str!("ENOMOTO_MIP_DEBUG_RESTART_CUT_USE").is_some() && root_st == LpStatus::Optimal {
            let x = self.lp.col_values();
            let (mut viol, mut tight, mut slack) = (0usize, 0usize, 0usize);
            for (c, r, _) in self.cut_pool.iter() {
                let a: f64 = c.iter().map(|&(j, v)| if j < x.len() { v * x[j] } else { 0.0 }).sum();
                let tol = 1e-6 * (1.0 + r.abs());
                if a > r + tol { viol += 1 } else if a >= r - tol { tight += 1 } else { slack += 1 }
            }
            let act = self.lp.row_activities();
            let (mut rt, mut rs) = (0usize, 0usize);
            for i in self.p.m..self.lp.num_rows() {
                let (l, u) = self.lp.row_bounds(i);
                let t = (u.is_finite() && act[i] >= u - 1e-6 * (1.0 + u.abs())) || (l.is_finite() && act[i] <= l + 1e-6 * (1.0 + l.abs()));
                if t { rt += 1 } else { rs += 1 }
            }
            eprintln!(
                "RESTART_CUT_USE: after restart {} root LP {:.10e}: cut pool {} (violated {viol}, tight {tight}, slack {slack}); LP cut rows {} (tight {rt}, slack {rs}); problem rows {}",
                self.params.restarts, self.lp.objective() + self.p.offset, self.cut_pool.len(), rt + rs, self.p.m
            );
        }
        if self.params.verbose && self.snap.is_some() {
            eprintln!("MIP: [snapshot] root LP {root_st:?} ({} iters, {:.2}s)", self.lp.total_iterations(), self.start.elapsed().as_secs_f64());
        }
        if matches!(root_st, LpStatus::Error | LpStatus::IterationLimit) && !self.time_up() {
            // 結論を出せなかった (数値的な破綻など): 内点法 + クロスオーバーだけで解き直す
            if self.params.verbose {
                eprintln!("MIP: root LP {root_st:?} ({:.2}s), re-solving with the interior-point method + crossover", self.start.elapsed().as_secs_f64());
            }
            root_st = self.lp.solve_rescue(&SolveLimits { deadline: self.deadline, ..Default::default() });
        }
        match root_st {
            LpStatus::Optimal => {
                self.cur_lb = self.lp.objective() + self.p.offset;
                self.root_lb = self.cur_lb;
                user_log!("Root LP: objective {:.10e} ({:.2}s)", self.p.sense_sign * (self.lp.objective() + self.p.offset), self.start.elapsed().as_secs_f64());
            }
            LpStatus::Infeasible => {
                user_log!("Root LP: infeasible ({:.2}s)", self.start.elapsed().as_secs_f64());
                return self.finish(MipStatus::Infeasible, f64::INFINITY);
            }
            LpStatus::Unbounded => {
                user_log!("Root LP: no finite optimum ({:.2}s)", self.start.elapsed().as_secs_f64());
                let st = self.resolve_unbounded_root();
                return self.finish(st, f64::NEG_INFINITY);
            }
            LpStatus::TimeLimit => {
                self.join_fj_thread();
                return self.finish(MipStatus::TimeLimit, self.root_fail_bound());
            }
            _ => {
                // 根の LP が解けない: LP を使わない Feasibility Jump の解だけは受け取る
                if self.params.verbose {
                    eprintln!("MIP: root LP failed ({root_st:?})");
                }
                self.join_fj_thread();
                let st = if self.time_up() { MipStatus::TimeLimit } else { MipStatus::NotSolved };
                return self.finish(st, self.root_fail_bound());
            }
        }
        let mut root_obj = self.lp.objective() + self.p.offset;
        if self.snap.is_none() {
            self.fj_poor_lb = root_obj;
        }
        // 根の下界 (カットのループが最後の LP で書き直す。ループが途中で終わっても根の LP の値は残す)
        if !self.params.submip && self.snap.is_none() {
            super::record_root_bound(root_obj);
        }
        let root_iters = self.lp.total_iterations();
        self.root_lp_iters = root_iters;
        if self.params.verbose {
            eprintln!("MIP: root LP {:.10e} ({} iters, {:.2}s)", root_obj, self.lp.total_iterations(), self.start.elapsed().as_secs_f64());
        }
        // カットを加える前の根の LP 解 (RENS の 2 つ目の近傍に使う)
        let x_root0 = self.lp.col_values();
        // `ENOMOTO_MIP_EARLY_RENS` (試作): 暫定解がない (か質の悪い FJ の解だけの) とき、暫定解なし用のダイビング式 RENS を
        // カットのループの前に (カット前の LP 解で、同じスレッドで) 回す。hypothyroid-k1 ではこの RENS のサブ MIP が
        // 0.2 秒で最適値 -2851 を見つけるが、既定ではカットのループ (6.7 秒) と根のヒューリスティクスの後になる
        if env_str!("ENOMOTO_MIP_EARLY_RENS").is_some() && !self.params.submip && self.snap.is_none() && !self.fractional(&x_root0).is_empty() {
            self.poll_fj_thread();
            if self.incumbent.is_none() || self.fj_poor() {
                let t = Instant::now();
                self.heur_label = "RENS (early)";
                let found = self.rens(&x_root0);
                self.heur_label = "";
                if self.params.verbose {
                    eprintln!("MIP: early RENS before the root cuts: {} ({:.2}s, incumbent {:?}, {:.2}s)", if found { "found" } else { "none" }, t.elapsed().as_secs_f64(), self.incumbent.as_ref().map(|(z, _)| *z), self.start.elapsed().as_secs_f64());
                }
            }
            // `ENOMOTO_MIP_EARLY_RENS_PROBE` (試作): その直後に打ち切り値つきの根のプロービング ([`Self::root_obj_probe`]) も
            // カットのループの前に回す (固定は大域的に入り、カットのループの後の 2 回目も回る)
            if env_str!("ENOMOTO_MIP_EARLY_RENS_PROBE").is_some() && env_str!("ENOMOTO_MIP_NO_ROOT_OBJ_PROBE").is_none() && self.root_obj_probe() {
                if let Some(z) = self.incumbent.as_ref().map(|(z, _)| *z) {
                    if self.shared.as_ref().is_none_or(|s| !(s.best() < z)) {
                        if self.params.verbose {
                            eprintln!("MIP: early root objective probing proved the incumbent {z:.10e} optimal ({:.2}s)", self.start.elapsed().as_secs_f64());
                        }
                        return self.finish(MipStatus::Optimal, z);
                    }
                }
            }
        }
        // 根の切除平面
        let cut_job = CUT_JOB.with(|j| j.borrow().clone()).filter(|_| self.snap.is_some());
        {
            let x = self.lp.col_values();
            if !self.fractional(&x).is_empty() && (self.snap.is_none() || cut_job.is_some()) {
                if cut_job.is_none() {
                    self.simple_rounding(&x);
                }
                // `ENOMOTO_MIP_ROOT_WARM_DSE`: 根のカットのループの間だけ DSE 重みを引き継ぐ (カットを足した後の LP の反復を減らす)。
                // 1 ラウンドの反復は 10-45% 減る (neos-1456979 357 -> 193) が、40 問では 19 -> 17 問 (sgeomean 27.64 -> 29.64):
                // 着く根の LP 解・カットが変わり、10teams (3.3 秒 -> 時間切れ)・neos-860300 が解けなくなった (mik-250 は 31.6 -> 19.4 秒)
                let root_warm_dse = env_str!("ENOMOTO_MIP_ROOT_WARM_DSE").is_some() && !self.params.submip;
                let warm_prev = crate::simplex::slope_intercept_dual::WARM_DSE_ON.load(std::sync::atomic::Ordering::Relaxed);
                if root_warm_dse {
                    crate::simplex::slope_intercept_dual::WARM_DSE_ON.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                let cut_ok = env_str!("ENOMOTO_MIP_NO_CUTS").is_some() || self.root_cut_loop(root_iters);
                if root_warm_dse {
                    crate::simplex::slope_intercept_dual::WARM_DSE_ON.store(warm_prev, std::sync::atomic::Ordering::Relaxed);
                }
                if !cut_ok {
                    // カットを外した LP も解けなかった。期限に当たったのなら時間切れ (カットを加える前の根の LP の値は下界)
                    self.join_fj_thread();
                    let st = if self.time_up() { MipStatus::TimeLimit } else { MipStatus::NotSolved };
                    return self.finish(st, root_obj);
                }
                root_obj = self.lp.objective() + self.p.offset;
                self.root_lb = root_obj;
                if self.snap.is_none() {
                    self.fj_poor_lb = root_obj;
                }
                // `ENOMOTO_MIP_MERGE_ROOT_CUTS`: 根の最後の LP で効いているカットを双対値で 1 本にまとめて加える
                // 40 問: 使わない 20 問 27.57、使う 18 問 28.23 (rout 30.1 秒・neos5 53.7 秒が時間切れ、neos-860300 54.0 -> 48.4 秒、
                // neos-911970 の下界は上がるが解けない)。既定では使わない
                if env_str!("ENOMOTO_MIP_MERGE_ROOT_CUTS").is_some() && !self.params.submip {
                    self.merge_active_cuts();
                }
            }
        }
        if let Some(job) = &cut_job {
            let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = (self.p.m..self.lp.num_rows())
                .map(|i| {
                    let (l, u) = self.lp.row_bounds(i);
                    (self.lp.row(i), l, u)
                })
                .collect();
            {
                if env_str!("ENOMOTO_MIP_NO_BG_CUTS_BASIS").is_none() {
                    *job.basis.lock().unwrap() = Some(self.lp.basis());
                }
                *job.out.lock().unwrap() = Some((rows, self.lp.objective() + self.p.offset, self.bg_cut_rounds));
            }
            return MipResult { status: MipStatus::NotSolved, x: None, objective: None, best_bound: f64::NEG_INFINITY, nodes: 0, lp_iterations: self.lp.total_iterations() };
        }
        self.join_fj_thread();
        if self.root_gap_closed() {
            let lb = if env_str!("ENOMOTO_MIP_NO_ROOT_CUT_GAP_EXIT").is_some() { self.cur_lb } else { self.root_bound_now(root_obj) };
            return self.finish_root_gap("root LP bound", lb);
        }
        // 並列モードで、根のヒューリスティクスの間に、今の根の写しから作った求解でカットのループを続ける (分離は残りの
        // スレッドで並列)。ヒューリスティクスが終わったら止め、その LP に残ったカット (効いているもの) で根の LP のカットを
        // 置き換える (目的値が上がったときだけ)。既定で使う (`ENOMOTO_MIP_NO_BG_CUTS` で使わない)。`ENOMOTO_MIP_BG_CUTS=heavy`
        // なら停滞の判定を緩め、停滞したら HiGHS のスコアの第 2 段階に入る。
        // 40 問 (各 2 回、threads=4): 使わない SGM 20.79・ギャップ 11.91%、使う 20.58 (−1.0%、2 回とも改善)・11.73%、heavy
        // 20.60・11.95%。根の下界は h80x6320d 5947 -> 6051、qnet1 15714 -> 15787、binkar10_1 6710.6 -> 6719.9 (heavy は
        // 停滞する問題でさらに上がる: timtab1 465655 -> 571329 だが、カットが増えて木が重くなり全体は同じ)
        let mut find_spawned = false;
        let light_root = self.params.restarts > 0 && self.params.cutoff.is_finite() && env_str!("ENOMOTO_MIP_LIGHT_RESTART_ROOT").is_some();
        if find_thread_on() && !light_root && self.snap.is_none() && !self.params.submip && self.params.threads > if thread_budget_on() { 3 } else { 2 } && self.fj_poor() && !self.fractional(&self.lp.col_values()).is_empty() {
            let snap = self.make_snapshot(&Portfolio::new(), &mut None, false);
            let remaining = self.deadline.map_or(f64::INFINITY, |d| d.saturating_duration_since(Instant::now()).as_secs_f64());
            let budget = (tunable!("ENOMOTO_T_MIP_FIND_THREAD_FRAC", 0.3, f64) * remaining).min(tunable!("ENOMOTO_T_MIP_FIND_THREAD_MAX", 20.0, f64));
            let params = MipParams { time_limit: budget, node_limit: u64::MAX, verbose: env_str!("ENOMOTO_MIP_FIND_THREAD_LOG").is_some(), threads: 1, ..self.params };
            let dive_budget = 2 * root_iters + 1000;
            let ri = root_iters;
            let basis = self.lp.basis();
            let out = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            self.find_out = Some(out.clone());
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            self.find_stop = Some(stop.clone());
            if self.params.verbose {
                eprintln!("MIP: find thread started (budget {budget:.1}s, {:.2}s)", self.start.elapsed().as_secs_f64());
            }
            self.find_thread = Some(
                std::thread::Builder::new()
                    .stack_size(64 << 20)
                    .spawn(move || {
                        DIVE_ONLY.with(|d| d.set((5, dive_budget, ri)));
                        FIND_OUT.with(|o| *o.borrow_mut() = Some(out));
                        FIND_STOP.with(|o| *o.borrow_mut() = Some(stop));
                        BG_ROOT_BASIS.with(|b| *b.borrow_mut() = Some(basis));
                        solve_worker(snap, None, params)
                    })
                    .expect("failed to spawn the find thread"),
            );
            find_spawned = true;
        }
        let mut bg_cuts: Option<BgCuts> = None;
        if self.snap.is_none() && !self.params.submip && self.params.threads > 1 && env_str!("ENOMOTO_MIP_NO_CUTS").is_none() && !self.fractional(&self.lp.col_values()).is_empty() {
            let bg_mode = env_str!("ENOMOTO_MIP_BG_CUTS").or(if env_str!("ENOMOTO_MIP_NO_BG_CUTS").is_none() { Some("1") } else { None });
            if let Some(mode) = bg_mode {
                let job = std::sync::Arc::new(CutJob { stop: std::sync::atomic::AtomicBool::new(false), heavy: mode == "heavy", out: std::sync::Mutex::new(None), latest_x: std::sync::Mutex::new(None), basis: std::sync::Mutex::new(None) });
                let snap = self.make_snapshot(&Portfolio::new(), &mut None, false);
                let remaining = self.deadline.map_or(f64::INFINITY, |d| d.saturating_duration_since(Instant::now()).as_secs_f64());
                let mut threads = tunable!("ENOMOTO_T_MIP_BG_CUTS_THREADS", (self.params.threads - 1).max(1), usize);
                // 暫定解が質の悪い FJ の解だけなら (`ENOMOTO_MIP_NO_FJ_POOR_BG_RENS` で使わない)、背景のカットの分離のスレッドを 1 本減らし、
                // 空いた分で暫定解なし用のダイビング式 RENS のサブ MIP を別スレッドで解く ([`fj_poor_bg_rens`])
                if find_spawned && threads > 1 {
                    threads -= 1;
                }
                if thread_budget_on() {
                    // 主 1・同時のサブ MIP・解探索のスレッド・暫定解なし用の RENS を引いた残り
                    let used = 1 + submip_par_limit() + find_spawned as usize + (fj_poor_bg_rens() && self.fj_poor()) as usize;
                    threads = self.params.threads.saturating_sub(used).max(1);
                }
                if !thread_budget_on() && fj_poor_bg_rens() && self.fj_poor() && threads > 1 {
                    threads -= 1;
                    if self.params.verbose {
                        eprintln!("MIP: the incumbent is a poor feasibility jump solution; background cuts use {threads} threads");
                    }
                }
                let params = MipParams { time_limit: remaining, node_limit: u64::MAX, verbose: env_str!("ENOMOTO_MIP_BG_CUTS_LOG").is_some(), threads, ..self.params };
                let j2 = job.clone();
                let ri = root_iters;
                // 既定で (`ENOMOTO_MIP_NO_BG_CUTS_BASIS` で渡さない) 主の根の LP の最後の基底 (写しの問題の行 + カットの行と同じ並び) を渡す。渡さないと
                // 写しの根の LP は基底なしで解き直しになり、uccase12 (88600 行) では 38 秒たっても終わらず、主がその終わりを
                // 待って 23〜27 秒止まっていた (LP は止めの合図を見ない)
                let bg_basis = if env_str!("ENOMOTO_MIP_NO_BG_CUTS_BASIS").is_none() { Some(self.lp.basis()) } else { None };
                let handle = std::thread::Builder::new()
                    .stack_size(64 << 20)
                    .spawn(move || {
                        CUT_JOB.with(|j| *j.borrow_mut() = Some(j2));
                        BG_ROOT_ITERS.with(|c| c.set(ri));
                        BG_ROOT_BASIS.with(|b| *b.borrow_mut() = bg_basis);
                        let _ = solve_worker(snap, None, params);
                    })
                    .expect("failed to spawn the cut thread");
                bg_cuts = Some(BgCuts { job, handle: Some(handle) });
            }
        }
        // 根のヒューリスティクス
        // `ENOMOTO_MIP_BG_CUTS_SHARE_X`: カットを強める求解の途中の LP 解を、各ヒューリスティクスの前に受け取って使う
        let share_x = bg_cuts.is_some() && env_str!("ENOMOTO_MIP_BG_CUTS_SHARE_X").is_some();
        let mut shared_x_used = 0usize;
        {
            #[allow(unused_mut)]
            let mut x = self.lp.col_values();
            self.noinc_root_frac = self.fractional(&x).len();
            // 根の被約費用の早めの記録 (既定で使う。`ENOMOTO_MIP_NO_EARLY_REDCOST` で使わない): 根の被約費用を、根のヒューリスティクスの前 (LP が根のカットの後の最適な状態のうち) に
            // 記録して固定する。既定では根のヒューリスティクスと裏のカットの後にしか記録しないので、根の途中で見つかった解
            // (FJ・Local-MIP・ヒューリスティクス) で被約費用による固定が進まない (根の終わりに 1 回だけ)。カットの後の LP も緩和なので固定は正しい
            if super::early_redcost_on() && !self.params.submip && self.snap.is_none() {
                self.store_root_redcost(root_obj);
                self.root_redcost_fixing();
                if self.params.verbose {
                    eprintln!("MIP: early root reduced costs stored ({} newly fixed integer columns, {:.2}s)", self.newly_fixed_ints(), self.start.elapsed().as_secs_f64());
                }
            }
            // 並列モード: サブ MIP を使うヒューリスティクスは別スレッドで解かせる
            self.parallel_submips = mip_threads() > 1 && !self.params.submip;
            if self.fractional(&x).is_empty() {
                self.try_lp_solution();
                // 並列のダイビングのための求解: 根の LP が整数なら、その解を返して終える (木は探索しない)
                if DIVE_ONLY.with(|d| d.get()).0 > 0 && self.snap.is_some() {
                    let (x, objective) = match &self.incumbent {
                        Some((z, x)) => (Some(x.clone()), Some(*z)),
                        None => (None, None),
                    };
                    return MipResult { status: MipStatus::NotSolved, x, objective, best_bound: f64::NEG_INFINITY, nodes: 0, lp_iterations: self.lp.total_iterations() };
                }
            } else if self.snap.is_some() {
                // 木の写しから解く: 根のヒューリスティクスは走らせない。並列のダイビングのための求解なら、指定のものだけを
                // 行って見つけた解を返す
                let (kind, budget, riters) = DIVE_ONLY.with(|d| d.get());
                if kind > 0 {
                    match kind {
                        1 => {
                            self.feasibility_pump(riters);
                        }
                        2 => {
                            self.fractional_dive(budget);
                        }
                        3 => {
                            self.dive(super::heuristics::DiveKind::VectorLength, budget, f64::INFINITY);
                        }
                        5 => {
                            // 根の解発見スレッド ([`find_thread_on`]): 暫定解なし用のヒューリスティクスを予算の中で順に回す
                            let z0 = self.incumbent.as_ref().map(|(z, _)| *z);
                            let steps: [(&'static str, u8); 5] = [("local search", 0), ("fractional diving", 2), ("vector length diving", 3), ("coefficient diving", 4), ("feasibility pump", 1)];
                            for (name, k) in steps {
                                if self.time_up() {
                                    break;
                                }
                                let t = Instant::now();
                                self.heur_label = name;
                                let found = match k {
                                    0 => env_str!("ENOMOTO_MIP_NO_LOCAL_SEARCH").is_none() && self.local_search_heur(),
                                    1 => self.feasibility_pump(riters),
                                    2 => self.fractional_dive(budget),
                                    3 => self.dive(super::heuristics::DiveKind::VectorLength, budget, f64::INFINITY),
                                    _ => env_str!("ENOMOTO_MIP_NO_COEF_DIVE").is_none() && self.dive(super::heuristics::DiveKind::Coefficient, budget, f64::INFINITY),
                                };
                                self.heur_label = "";
                                if found {
                                    if let (Some(o), Some((_, x))) = (FIND_OUT.with(|o| o.borrow().clone()), self.incumbent.as_ref()) {
                                        o.lock().unwrap().push(x.clone());
                                    }
                                }
                                if self.params.verbose {
                                    eprintln!("MIP: [find thread] {name}: {} ({:.2}s), incumbent {z0:?} -> {:?}", if found { "found" } else { "none" }, t.elapsed().as_secs_f64(), self.incumbent.as_ref().map(|(z, _)| *z));
                                }
                            }
                        }
                        _ => {
                            self.dive(super::heuristics::DiveKind::Coefficient, budget, f64::INFINITY);
                        }
                    }
                    let (x, objective) = match &self.incumbent {
                        Some((z, x)) => (Some(x.clone()), Some(*z)),
                        None => (None, None),
                    };
                    return MipResult { status: MipStatus::NotSolved, x, objective, best_bound: f64::NEG_INFINITY, nodes: 0, lp_iterations: self.lp.total_iterations() };
                }
            } else {
                // `ENOMOTO_T_MIP_ROOT_HEUR_TIME_FRAC` = f (> 0): 根のヒューリスティクス全体の時間の予算を時間上限の f 倍
                // (暫定解がなければ `ENOMOTO_T_MIP_ROOT_HEUR_TIME_FRAC_NOINC` 倍) にし、超えたら残りは飛ばす
                let rh_t0 = Instant::now();
                if !self.params.submip && self.params.restarts == 0 {
                    super::record_root_heur_start();
                }
                let rh_frac = tunable!("ENOMOTO_T_MIP_ROOT_HEUR_TIME_FRAC", 0.0, f64);
                let rh_frac_noinc = tunable!("ENOMOTO_T_MIP_ROOT_HEUR_TIME_FRAC_NOINC", 0.5, f64);
                let rh_over = |s: &Self| {
                    rh_frac > 0.0
                        && !s.params.submip
                        && s.params.time_limit.is_finite()
                        && rh_t0.elapsed().as_secs_f64() > if s.incumbent.is_some() { rh_frac } else { rh_frac_noinc } * s.params.time_limit
                };
                macro_rules! heur {
                    ($name:expr, $e:expr) => {{
                        let bit = heur_bit($name);
                        self.poll_lmip_thread();
                        if proof_only() && !self.params.submip {
                            // 証明だけのスレッド: 根のヒューリスティクスは走らせない
                        } else if self.time_up() {
                            // 期限を過ぎた: 残りのヒューリスティクスは走らせない (局所探索などは期限を見ずに数秒使う)
                        } else if self.root_cut_gap_closed(root_obj) {
                            // 暫定解がカットの後の根の下界に届いた: 残りのヒューリスティクスは走らせない (下で最適で終える)
                        } else if rh_over(self) {
                            if self.params.verbose {
                                eprintln!("MIP: root heuristic {}: skipped (root heuristic time budget)", $name);
                            }
                        } else if self.early_restart {
                            if self.params.verbose {
                                eprintln!("MIP: root heuristic {}: skipped (early restart)", $name);
                            }
                        } else if self.params.restarts > 0 && self.params.cutoff.is_finite() && env_str!("ENOMOTO_MIP_LIGHT_RESTART_ROOT").is_some()
                            && !matches!($name, "trivial" | "simple rounding" | "ZI round" | "randomized rounding" | "shift-and-propagate")
                        {
                            // 安い丸め (と自明な解・shift-and-propagate) は残し、サブ MIP・ダイビング・局所探索など重いものだけ飛ばす
                            if self.params.verbose {
                                eprintln!("MIP: root heuristic {}: skipped (light root after a restart)", $name);
                            }
                        } else if self.params.skip_heurs & bit != 0 && env_str!("ENOMOTO_MIP_RESTART_ALL_HEURS").is_none() {
                            if self.params.verbose {
                                eprintln!("MIP: root heuristic {}: skipped (no solution before the restart)", $name);
                            }
                        } else {
                            if share_x {
                                if let Some(nx) = bg_cuts.as_ref().and_then(|b| b.job.latest_x.lock().unwrap().take()) {
                                    if nx.len() == x.len() {
                                        x = nx;
                                        shared_x_used += 1;
                                    }
                                }
                            }
                            if self.find_out.is_some() {
                                self.poll_find_thread();
                            }
                            let t = Instant::now();
                            // 近さ ([`Solver::heur_close`]) を記録しない関数 (旧来の RENS など) を見分けるため負にしておく
                            self.heur_close = -1.0;
                            let z_before = self.incumbent.as_ref().map(|(z, _)| *z);
                            self.heur_label = $name;
                            // `ENOMOTO_T_MIP_ROOT_DIVE_TIME_FRAC`: Feasibility Pump・ダイビング 1 回の時間を時間上限のこの割合までにする
                            // (既定 0 は上限なし。trento1 では Pump が 1 回で 45 秒使い、その間に別スレッドの RENS が見つけた解も
                            // 受け取れなかった)
                            let cap = tunable!("ENOMOTO_T_MIP_ROOT_DIVE_TIME_FRAC", 0.0, f64);
                            if cap > 0.0 && !self.params.submip && self.params.time_limit.is_finite() && ($name == "feasibility pump" || $name.ends_with("diving")) {
                                self.heur_stop = Some(Instant::now() + std::time::Duration::from_secs_f64(cap * self.params.time_limit));
                            }
                            let found = $e;
                            self.heur_stop = None;
                            self.heur_label = "";
                            // 早めの再スタート (既定で使う。`ENOMOTO_MIP_NO_EARLY_RESTART` で使わない): このヒューリスティクスで暫定解が良くなり、根の被約費用による固定で
                            // 新たに固定された整数列が `ENOMOTO_T_MIP_EARLY_RESTART_FRAC` (0.025) 以上なら、残りの根のヒューリスティクスを飛ばして再スタートへ進む
                            if super::early_restart_on() && self.incumbent.as_ref().map(|(z, _)| *z) != z_before
                                && self.proto_restart_ok(tunable!("ENOMOTO_T_MIP_EARLY_RESTART_FRAC", 0.025, f64))
                            {
                                self.early_restart = true;
                                if self.params.verbose {
                                    eprintln!("MIP: early restart: {} newly fixed integer columns after {} ({:.2}s)", self.newly_fixed_ints(), $name, self.start.elapsed().as_secs_f64());
                                }
                            }
                            // `ENOMOTO_MIP_POLL_SUBMIPS`: 別スレッドのサブ MIP のうち終わったものの結果をすぐ受け取る (以前は根の
                            // ヒューリスティクスを一通り回し終えてから受け取ったので、見つかった解が後のヒューリスティクスに使われなかった)
                            if env_str!("ENOMOTO_MIP_POLL_SUBMIPS").is_some() {
                                self.poll_submips();
                            }
                            if !found {
                                self.failed_heurs |= bit;
                            }
                            if let Some(z0) = z_before {
                                let gap = z0 - root_obj;
                                let z1 = self.incumbent.as_ref().map_or(z0, |(z, _)| *z);
                                if gap > 0.0 {
                                    self.root_mu_obs.push((((z0 - z1) / gap).clamp(0.0, 1.0), t.elapsed().as_secs_f64().max(1e-3)));
                                }
                            }
                            // 木の中のバンディットの腕に当たるものは、近さを初期値として取っておく ([`Solver::noinc_prior`])。
                            // 並列モードの RENS はサブ MIP を別スレッドに出しただけなので記録しない
                            if let Some(arm) = noinc_root_arm($name) {
                                if self.heur_close >= 0.0 && !(arm == 0 && self.parallel_submips) {
                                    self.noinc_prior[arm] = Some(if found { 1.0 } else { self.heur_close.min(1.0) });
                                }
                            }
                            if self.params.verbose {
                                eprintln!("MIP: root heuristic {}: {} ({:.2}s)", $name, if found { "found" } else { "none" }, t.elapsed().as_secs_f64());
                            }
                        }
                    }};
                }
                // `ENOMOTO_MIP_PAR_LOCAL_SEARCH`: 並列モードで暫定解がなければ、局所探索を今から別スレッドで始める
                // (Feasibility Pump・ダイビングと重ねる。結果は元の位置 (下の "local search") で受け取る)
                if mip_threads() > 1 && !self.params.submip && self.incumbent.is_none() && env_str!("ENOMOTO_MIP_PAR_LOCAL_SEARCH").is_some() && env_str!("ENOMOTO_MIP_NO_LOCAL_SEARCH").is_none() {
                    self.spawn_local_search();
                }
                // 診断用 (`ENOMOTO_MIP_DEBUG_DUMP_ROOT_X=<接頭辞>`): 根のヒューリスティクスの前に、この求解の問題を `<接頭辞><k>.mps`、
                // カットの後の LP 解を `<接頭辞><k>_x.txt`、カットの前の LP 解を `<接頭辞><k>_x0.txt` に書く (列の順に 1 行 1 値。
                // k は再スタートごとに 0, 1, ...)。LP 解と最適解の離れ方の調査 (analysis/physlpdist_20261024.md)
                if let Some(pre) = env_str!("ENOMOTO_MIP_DEBUG_DUMP_ROOT_X").filter(|_| !self.params.submip) {
                    let k = self.params.restarts;
                    let f = format!("{pre}{k}");
                    let _ = self.p.write_mps(&format!("{f}.mps"));
                    let w = |name: &str, v: &[f64]| std::fs::write(format!("{f}{name}"), v.iter().map(|a| format!("{a:.17e}\n")).collect::<String>());
                    let _ = w("_x.txt", &x);
                    let _ = w("_x0.txt", &x_root0);
                    eprintln!("MIP: root LP solution written to {f}_x.txt (objective {:.6e}, {:.2}s)", self.lp.objective() + self.p.offset, self.start.elapsed().as_secs_f64());
                }
                // 診断用 (`ENOMOTO_MIP_DEBUG_LMIP_FROM_LP`): Local-MIP (連鎖の手つき) を、LP 解を四捨五入した点から始めたものと、
                // 既定の出発点 (各列の下限) から始めたものを、種 2 つずつ `ENOMOTO_T_MIP_DEBUG_LMIP_SECS` (20) 秒ずつ回して比べる
                // (主のスレッドで。見つけた解は使わない)。LP 解は `ENOMOTO_MIP_DEBUG_LMIP_X=<ファイル>` (列の数が合えば) か、今の LP 解
                if env_str!("ENOMOTO_MIP_DEBUG_LMIP_FROM_LP").is_some() && !self.params.submip {
                    let secs = tunable!("ENOMOTO_T_MIP_DEBUG_LMIP_SECS", 20.0, f64);
                    let p = self.p;
                    let xs: Vec<f64> = env_str!("ENOMOTO_MIP_DEBUG_LMIP_X")
                        .and_then(|f| std::fs::read_to_string(f).ok())
                        .map(|s| s.lines().filter_map(|l| l.trim().parse().ok()).collect::<Vec<f64>>())
                        .filter(|v| v.len() == p.n)
                        .unwrap_or_else(|| x.clone());
                    let nfrac = (0..p.n).filter(|&j| p.is_int[j] && (xs[j] - xs[j].round()).abs() > 1e-6).count();
                    eprintln!("MIP: [lmip from LP] LP solution: objective {:.4}, fractional {nfrac}", p.objective(&xs));
                    for (mode, k) in [("LP", 0u64), ("LP", 1), ("lower bounds", 0), ("lower bounds", 1)] {
                        let Some(mut lm) = super::localmip::LocalMip::new(p, &self.dom.lo, &self.dom.up, 12345 + k, true) else { break };
                        let v0 = if mode == "LP" { lm.set_start(&xs) } else { usize::MAX };
                        let t = Instant::now();
                        lm.run(t + std::time::Duration::from_secs_f64(secs), None, &|_| {});
                        let res = &lm.res;
                        let (z, d) = match &res.best {
                            Some(b) => (p.objective(b), (0..p.n).filter(|&j| p.is_int[j] && self.dom.lo[j] < self.dom.up[j]).map(|j| (b[j] - xs[j]).abs()).sum::<f64>()),
                            None => (f64::NAN, f64::NAN),
                        };
                        eprintln!(
                            "MIP: [lmip from {mode}] seed {k}: violated rows at the start {}, first feasible {:?}, best objective {z:.1}, L1 to the LP solution {d:.1}, least violated rows {}, steps {}, {:.1}s",
                            if v0 == usize::MAX { "-".to_string() } else { v0.to_string() },
                            res.first_time.map(|t| (t * 100.0).round() / 100.0),
                            res.least_viol,
                            res.steps,
                            t.elapsed().as_secs_f64()
                        );
                    }
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
                if self.incumbent.is_none() && self.ref_sol.is_none() && !self.fractional(&x_root0).is_empty() {
                    heur!("RENS (LP before cuts)", self.rens(&x_root0));
                }
                // `ENOMOTO_MIP_SUBMIP_LIGHT_HEUR`: サブ MIP の根では Feasibility Pump とダイビングを使わない。30n20b8 の根の RENS の
                // サブ MIP (942 列) では、この 4 つで 2.4 秒使って何も見つけず、入れ子の RENS のサブ MIP でも毎回同じだけかかる
                // (HiGHS のサブ MIP は根のヒューリスティクスのほとんどを入れ子の RENS に使い、1 回 0.1-1.6 秒で終わる)
                // (暫定解がない、つまり打ち切り値が無限のサブ MIP だけ。`ENOMOTO_MIP_SUBMIP_NO_SB` と同じ理由)
                let light = (self.params.submip && self.params.cutoff.is_infinite() && env_str!("ENOMOTO_MIP_SUBMIP_LIGHT_HEUR").is_some()) || self.submip_rens_cutoff();
                // ダイビングの予算: 最初の根の LP の 2 倍に、カットのループを含めた根全体の反復数の一定割合を足す
                let dive_budget = 2 * root_iters + 1000 + (tunable!("ENOMOTO_T_MIP_ROOT_DIVE_FRAC", 0.0, f64) * self.lp.total_iterations() as f64) as u64;
                // `ENOMOTO_MIP_PAR_DIVES`: 並列モードで暫定解がなければ、Feasibility Pump と 3 つのダイビングを、今の根の写しから
                // 作った求解でそれぞれ別スレッドで同時に行い、元の順の位置で結果を受け取る (先のもので暫定解が見つかっても、
                // 後のものの解も受け取る)
                let mut par_dives: Vec<Option<std::thread::JoinHandle<MipResult>>> = Vec::new();
                if self.incumbent.is_none() && !light && mip_threads() > 1 && !self.params.submip && env_str!("ENOMOTO_MIP_PAR_DIVES").is_some() {
                    let snap = self.make_snapshot(&Portfolio::new(), &mut None, false);
                    let remaining = self.deadline.map_or(f64::INFINITY, |d| d.saturating_duration_since(Instant::now()).as_secs_f64());
                    let params = MipParams { time_limit: remaining, node_limit: u64::MAX, verbose: false, threads: 1, ..self.params };
                    for kind in 1..=4u8 {
                        let on = kind != 4 || env_str!("ENOMOTO_MIP_NO_COEF_DIVE").is_none();
                        par_dives.push(on.then(|| {
                            let sn = snap.clone();
                            std::thread::Builder::new()
                                .stack_size(64 << 20)
                                .spawn(move || {
                                    DIVE_ONLY.with(|d| d.set((kind, dive_budget, root_iters)));
                                    solve_worker(sn, None, params)
                                })
                                .expect("failed to spawn a diving thread")
                        }));
                    }
                }
                let take_dive = |s: &mut Self, dives: &mut Vec<Option<std::thread::JoinHandle<MipResult>>>, k: usize| -> bool {
                    let Some(h) = dives.get_mut(k).and_then(|h| h.take()) else { return false };
                    match h.join() {
                        Ok(r) => r.x.is_some_and(|x| s.try_incumbent(x)),
                        Err(_) => false,
                    }
                };
                if !par_dives.is_empty() {
                    heur!("feasibility pump", take_dive(self, &mut par_dives, 0));
                    heur!("fractional diving", take_dive(self, &mut par_dives, 1));
                    heur!("vector length diving", take_dive(self, &mut par_dives, 2));
                    heur!("coefficient diving", take_dive(self, &mut par_dives, 3));
                } else {
                    if self.noinc_like() && !light && !(self.incumbent.is_some() && self.fj_poor() && env_str!("ENOMOTO_MIP_FJ_POOR_NOINC").is_some_and(|v| v == "nopump")) {
                        heur!("feasibility pump", self.feasibility_pump(root_iters));
                    }
                    if self.noinc_like() && !light {
                        heur!("fractional diving", self.fractional_dive(dive_budget));
                    }
                    if self.noinc_like() && !light {
                        heur!("vector length diving", self.dive(super::heuristics::DiveKind::VectorLength, dive_budget, f64::INFINITY));
                    }
                    if self.noinc_like() && !light && env_str!("ENOMOTO_MIP_NO_COEF_DIVE").is_none() {
                        heur!("coefficient diving", self.dive(super::heuristics::DiveKind::Coefficient, dive_budget, f64::INFINITY));
                    }
                }
                // LP を使わない最後の手段 (先に使うと質の悪い解で他の暫定解探しを止めてしまう)
                if self.noinc_like() && env_str!("ENOMOTO_MIP_NO_SHIFTPROP").is_none() {
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
                self.poll_find_thread();
                // 局所探索と Repair (暫定解がなければ)
                if self.noinc_like() && env_str!("ENOMOTO_MIP_NO_LOCAL_SEARCH").is_none() {
                    heur!("local search", self.local_search_heur());
                }
                if self.noinc_like() && env_str!("ENOMOTO_MIP_NO_REPAIR").is_none() {
                    heur!("repair", self.repair(&x));
                }
                // 違反量を最小にする補助 MIP (最後の手段、既定では使わない: 30n20b8・neos-1456979 で違反 0 の点が
                // 見つからず根の時間を 3-5 s 使うだけだった。ENOMOTO_MIP_MINREL=1 で使う)
                if self.incumbent.is_none() && env_str!("ENOMOTO_MIP_MINREL").is_some() {
                    heur!("min relaxation", self.min_relaxation());
                }
                // `ENOMOTO_MIP_PAR_LATE_SUBMIPS`: 並列モードでは、この後の RENS のやり直しと RINS のサブ MIP を同時に解く
                // (RINS はやり直しの結果を待たずに今の暫定解で作る)
                let par_late = mip_threads() > 1 && !self.params.submip && env_str!("ENOMOTO_MIP_PAR_LATE_SUBMIPS").is_some();
                self.parallel_submips = par_late;
                // 根の RENS のサブ MIP がノード上限で止まり、根のヒューリスティクスの後も差が大きい (既定 0.5% 超) なら、
                // ノード上限を 4 倍にして RENS をやり直す (`ENOMOTO_MIP_NO_RENS_MORE_NODES` で無効)。binkar10_1 は 500 ノードの
                // RENS が 7182 で止まり、根の暫定解 6822.81 から木が 60 秒進まない (被約費用固定が効かず再スタートもしない)。
                // 約 1600 ノードで RENS が 6754.85 を見つけ、再スタートして最適値 6742.2 に届く
                if self.rens_node_limited && env_str!("ENOMOTO_MIP_NO_RENS_MORE_NODES").is_none() {
                    let gap = self.incumbent.as_ref().map(|(z, _)| (z - (self.lp.objective() + self.p.offset)) / z.abs().max(1.0));
                    if let Some(gap) = gap {
                        if gap > tunable!("ENOMOTO_T_MIP_RENS_MORE_NODES_GAP", 0.005, f64) {
                            self.submip_node_mult = tunable!("ENOMOTO_T_MIP_RENS_MORE_NODES_MULT", 4.0, f64);
                            heur!("RENS (more nodes)", self.rens(&x));
                            self.submip_node_mult = 1.0;
                        }
                    }
                }
                // 暫定解 (Feasibility Jump・pump・丸めなどで得たもの) を根の LP 解との RINS で磨く
                if (self.incumbent.is_some() || self.ref_sol.is_some()) && env_str!("ENOMOTO_MIP_NO_ROOT_RINS").is_none() {
                    heur!("RINS", self.rins(&x));
                }
                // `ENOMOTO_MIP_ROOT_LMIP` (試作): 根のヒューリスティクスの暫定解 (RENS・RINS などのサブ MIP の解を含む) を出発点に
                // Local-MIP を回して磨く ([`Self::root_lmip`])
                if root_lmip_on() && !self.params.submip && self.incumbent.is_some() {
                    heur!("root Local-MIP", self.root_lmip());
                }
                // 目的関数の台のサブ MIP (`ENOMOTO_MIP_OBJ_SUPPORT`、試作、[`Solver::obj_support`])。暫定解が根の LP の値から
                // 目的関数の刻み以上離れているときだけ
                if env_str!("ENOMOTO_MIP_OBJ_SUPPORT").is_some() && self.obj_support_gap(root_obj) {
                    heur!("objective support", self.obj_support(&x));
                }
                if par_late {
                    self.parallel_submips = false;
                    self.join_submips(0);
                }
                // 根の広い近傍のサブ MIP (`ENOMOTO_MIP_ROOT_WIDE_SUBMIP` で使う。既定では使わない。h80x6320d を時間切れにするため): 根の RENS・RINS の後も暫定解と根の LP の差が大きい
                // (既定 0.5% 超) なら、固定を目標の固定率 (初期 0.6) で止めた広い近傍 (`ENOMOTO_MIP_FIX_CAP` と同じ) で、
                // サブ MIP に長めの時間 (残りの `ENOMOTO_T_MIP_WIDE_SUBMIP_FRAC` = 0.3、最大 `ENOMOTO_T_MIP_WIDE_SUBMIP_MAX` = 20 秒)
                // を与えて RENS と RINS をもう 1 回ずつ解く。既定の RENS・RINS は LP 解 (と暫定解) に合う列を全部固定するので、
                // neos-960392 (根の LP = 最適値 -238) では RENS が 294 列残して実行不能、RINS は 692 列のサブ MIP の最適値 -226
                // 止まりになる。固定率を止めても既定の時間 (残りの 7%、最大 6 秒) ではサブ MIP が根のヒューリスティクスの途中で
                // 時間切れになる (8391 列で 3 秒、0 ノード)
                if env_str!("ENOMOTO_MIP_ROOT_WIDE_SUBMIP").is_some() && !self.params.submip {
                    let gap_of = |s: &Self| s.incumbent.as_ref().map(|(z, _)| (z - (s.lp.objective() + s.p.offset)) / z.abs().max(1.0));
                    let min_gap = tunable!("ENOMOTO_T_MIP_WIDE_SUBMIP_GAP", 0.005, f64);
                    if gap_of(self).is_some_and(|g| g > min_gap) {
                        let keep = (self.submip_time_frac, self.submip_time_max);
                        self.submip_time_frac = tunable!("ENOMOTO_T_MIP_WIDE_SUBMIP_FRAC", 0.3, f64);
                        self.submip_time_max = tunable!("ENOMOTO_T_MIP_WIDE_SUBMIP_MAX", 20.0, f64);
                        self.force_fix_cap = true;
                        heur!("RENS (wide)", self.rens(&x));
                        if gap_of(self).is_some_and(|g| g > min_gap) {
                            heur!("RINS (wide)", self.rins(&x));
                        }
                        self.force_fix_cap = false;
                        (self.submip_time_frac, self.submip_time_max) = keep;
                    }
                }
                if self.params.verbose && share_x {
                    eprintln!("MIP: root heuristics used {shared_x_used} LP solutions from the background cuts");
                }
                if self.params.verbose {
                    eprintln!("MIP: after root heuristics: incumbent {:?} ({:.2}s)", self.incumbent.as_ref().map(|(z, _)| *z), self.start.elapsed().as_secs_f64());
                }
            }
        }
        // 木の中のヒューリスティクス (ランダム丸め) の反復の予算は、根で使った分を除いて数える (根のサブ MIP の反復が
        // h80x6320d・assign1-5-8・graph20 で 5-6 万あり、木の予算 (LP 反復の 1/20 + 1 万) を最初から超えて木の中で
        // 一度も動かなかった)。`ENOMOTO_MIP_HEUR_BUDGET_ALL` なら以前どおり根の分も数える
        if env_str!("ENOMOTO_MIP_HEUR_BUDGET_ALL").is_none() {
            self.heur_iters_root = self.heur_iters;
        }
        // 並列モードは根のヒューリスティクスの間だけ (LP 解が整数で途中を飛ばした場合も戻す)
        self.parallel_submips = false;
        self.join_submips(0);
        // カットを強める求解を止め、その LP のカットで根の LP のカットを置き換える
        if let Some(mut bg) = bg_cuts.take() {
            let t0 = Instant::now();
            let res = bg.finish();
            let wait = t0.elapsed().as_secs_f64();
            // 根の LP を解き直しておく (ヒューリスティクスの後で状態が戻っていることを確かめる)
            let it0 = self.lp.total_iterations();
            let st0 = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
            let cur = self.lp.objective() + self.p.offset;
            if self.params.verbose {
                eprintln!("MIP: background cuts: root LP re-solved before replacing the cuts: {st0:?}, {} iters ({:.2}s)", self.lp.total_iterations() - it0, self.start.elapsed().as_secs_f64());
            }
            match res {
                Some((rows, obj, rounds)) if obj > cur + 1e-9 * cur.abs().max(1.0) && st0 == LpStatus::Optimal => {
                    let m0 = self.p.m;
                    let nr = self.lp.num_rows();
                    let saved: Vec<(Vec<(usize, f64)>, f64, f64)> = (m0..nr)
                        .map(|i| {
                            let (l, u) = self.lp.row_bounds(i);
                            (self.lp.row(i), l, u)
                        })
                        .collect();
                    let mut remove = vec![false; nr];
                    for r in remove.iter_mut().skip(m0) {
                        *r = true;
                    }
                    self.delete_lp_rows(&remove);
                    self.add_cut_rows(&rows);
                    if let Some(b) = bg.job.basis.lock().unwrap().take() {
                        if b.col.len() == self.p.n && b.row.len() == self.lp.num_rows() {
                            self.lp.set_basis(&b);
                        }
                    }
                    for (c, l, u) in &rows {
                        if u.is_finite() && !l.is_finite() {
                            self.add_to_pool(c, *u);
                        }
                    }
                    let it1 = self.lp.total_iterations();
                    let st = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
                    let new = self.lp.objective() + self.p.offset;
                    if self.params.verbose {
                        eprintln!("MIP: background cuts: LP with the replaced cuts: {st:?}, {} iters ({:.2}s)", self.lp.total_iterations() - it1, self.start.elapsed().as_secs_f64());
                    }
                    if st == LpStatus::Optimal && new >= cur - 1e-9 * cur.abs().max(1.0) {
                        if self.params.verbose {
                            eprintln!("MIP: background cuts: {} rounds, {} cut rows (was {}), obj {:.10e} -> {:.10e} (waited {:.2}s, {:.2}s)", rounds, rows.len(), saved.len(), cur, new, wait, self.start.elapsed().as_secs_f64());
                        }
                        root_obj = new;
                        // 根の被約費用の早めの記録 (`ENOMOTO_MIP_NO_EARLY_REDCOST` で使わない): 裏のカットで強くなった LP の被約費用を記録し直して固定する
                        if super::early_redcost_on() && !self.params.submip && self.snap.is_none() {
                            self.store_root_redcost(root_obj);
                            self.root_redcost_fixing();
                            if self.params.verbose {
                                eprintln!("MIP: root reduced costs re-stored after the background cuts ({} newly fixed integer columns)", self.newly_fixed_ints());
                            }
                        }
                        if !self.params.submip {
                            super::record_root_bound(new);
                        }
                        // `ENOMOTO_MIP_BG_CUTS_REHEUR`: 置き換えた LP の解で軽いヒューリスティクスをもう一度回す (`1`: 丸めだけ、
                        // `rins`: 丸めと、暫定解があれば RINS も、`rens` (試作): 丸めと RENS と RINS)。`rens` は beasleyC3 のため:
                        // 根のヒューリスティクスの RENS は前景のカットの後の LP (712〜720) で 768〜787 しか出さないが、HiGHS は
                        // カットの後の根 (733) の部分 MIP で 1.7 秒に 756 を見つける。背景のカットの後の LP は 742〜748 と強い
                        // `ENOMOTO_MIP_GR_RESTART` (試作) では一般整数のある問題で `rins`
                        if let Some(mode) = env_str!("ENOMOTO_MIP_BG_CUTS_REHEUR").or_else(|| super::gr_restart_for(self.p).then_some("rins")) {
                            let x2 = self.lp.col_values();
                            if !self.fractional(&x2).is_empty() {
                                let t1 = Instant::now();
                                let z0 = self.incumbent.as_ref().map(|(z, _)| *z);
                                let a = self.simple_rounding(&x2);
                                let b = self.zi_round(&x2);
                                let r = mode == "rens" && self.rens(&x2);
                                let c = (mode == "rins" || mode == "rens") && self.incumbent.is_some() && self.rins(&x2);
                                if self.params.verbose {
                                    eprintln!(
                                        "MIP: background cuts: heuristics on the new LP solution: rounding {a} ZI {b} RENS {r} RINS {c}, incumbent {z0:?} -> {:?} ({:.2}s)",
                                        self.incumbent.as_ref().map(|(z, _)| *z),
                                        t1.elapsed().as_secs_f64()
                                    );
                                }
                            }
                        }
                    } else {
                        // 戻す
                        let nr = self.lp.num_rows();
                        let mut remove = vec![false; nr];
                        for r in remove.iter_mut().skip(m0) {
                            *r = true;
                        }
                        self.delete_lp_rows(&remove);
                        self.add_cut_rows(&saved);
                        let st2 = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
                        if self.params.verbose {
                            eprintln!("MIP: background cuts: LP {st:?} / obj {new:.10e} < {cur:.10e}; restored ({st2:?})");
                        }
                    }
                }
                Some((_, obj, rounds)) => {
                    if self.params.verbose {
                        eprintln!("MIP: background cuts: {rounds} rounds, obj {obj:.10e} not above {cur:.10e}; not used (waited {wait:.2}s)");
                    }
                }
                None => {
                    if self.params.verbose {
                        eprintln!("MIP: background cuts: no result (waited {wait:.2}s)");
                    }
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
        // `ENOMOTO_T_MIP_ROOT_LNS_REPEAT` = N (> 0、既定 0 = 使わない): 根のヒューリスティクスの後、暫定解があれば大近傍探索
        // ([`Self::alns`]、木の中・ヒューリスティクス役と同じ腕の選び方) を、改善しない呼び出しが `ENOMOTO_T_MIP_ROOT_LNS_PATIENCE`
        // (既定 2) 回続くか、N 回か、根の大近傍探索の時間が時間上限の `ENOMOTO_T_MIP_ROOT_LNS_TIME_FRAC` (既定 0.1) 倍を超えるまで
        // 繰り返す (暫定解が停滞するまで根で磨く)
        let lns_rep = tunable!("ENOMOTO_T_MIP_ROOT_LNS_REPEAT", 0u64, u64);
        if lns_rep > 0 && !self.params.submip && self.snap.is_none() && self.incumbent.is_some() {
            let patience = tunable!("ENOMOTO_T_MIP_ROOT_LNS_PATIENCE", 2u64, u64).max(1);
            let tmax = tunable!("ENOMOTO_T_MIP_ROOT_LNS_TIME_FRAC", 0.1, f64) * self.params.time_limit;
            let t0 = Instant::now();
            let x = self.lp.col_values();
            let mut fails = 0u64;
            for k in 0..lns_rep {
                if self.time_up() || t0.elapsed().as_secs_f64() >= tmax || fails >= patience {
                    break;
                }
                let z0 = self.incumbent.as_ref().map_or(f64::INFINITY, |(z, _)| *z);
                let ts = Instant::now();
                self.heur_label = "root LNS repeat";
                self.alns(&x);
                self.heur_label = "";
                self.lns_secs += ts.elapsed().as_secs_f64();
                let z1 = self.incumbent.as_ref().map_or(f64::INFINITY, |(z, _)| *z);
                if z1 < z0 - 1e-9 * z0.abs().max(1.0) {
                    fails = 0;
                } else {
                    fails += 1;
                }
                if self.params.verbose {
                    eprintln!("MIP: root LNS repeat {k}: {z0:.10e} -> {z1:.10e} ({:.2}s)", ts.elapsed().as_secs_f64());
                }
            }
        }
        self.dbg_check_rows("after root cuts");
        self.dbg_lost("root domain (before redcost fixing)", !self.dbg_contains());
        let dbg_root = self.dbg_contains();
        self.hcm_root_obj = root_obj;
        // `ENOMOTO_MIP_BISECT_SHARE_CUTS`: 木の探索のスレッドの一番外の求解は、根で LP に残ったカットを元の問題の空間に戻して
        // 共有する (二分探索のスレッドが求解の最初に入れる)。成り立つ範囲は今の打ち切り値未満 (根の被約費用による固定は
        // 打ち切り値を使うので、それで締めた境界から作ったカットはその範囲でだけ成り立つ)。対称性を崩す行・固定を根で
        // 加えた問題では共有しない (カットがそれに頼っていると、別の求解では成り立たないことがある。オービトープの固定は
        // 木の中で分枝してから効くので、根のカットには関わらない)
        // 40 問 (木 1 本 + 二分探索 2 本、二分探索は根だけヒューリスティクス): 共有しない 20 問 27.04、共有する 20 問 26.81
        // (neos-860300 59.8 -> 49.1 秒、mik-250 44.6 -> 41.2 秒、neos-911970 で最適値を発見。ほかはほぼ同じ)
        if env_str!("ENOMOTO_MIP_BISECT_SHARE_CUTS").is_some() && !self.params.submip && self.params.restarts == 0 && self.watch.is_none() && self.orig.as_ref().is_none_or(|o| o.m == self.p.m && o.col_lo == self.p.col_lo && o.col_up == self.p.col_up) {
            if let (Some(s), Some(d)) = (self.shared.clone(), thread_scale()) {
                let m0 = self.p.m;
                let mut out: Vec<(Vec<(usize, f64)>, f64)> = Vec::new();
                for i in m0..self.lp.num_rows() {
                    let (l, u) = self.lp.row_bounds(i);
                    let row = self.lp.row(i);
                    if row.iter().any(|&(j, _)| j >= d.len() || d[j] == 0.0) {
                        continue;
                    }
                    let mapped: Vec<(usize, f64)> = row.iter().map(|&(j, v)| (j, v / d[j])).collect();
                    if u.is_finite() {
                        out.push((mapped.clone(), u));
                    }
                    if l.is_finite() {
                        out.push((mapped.iter().map(|&(j, v)| (j, -v)).collect(), -l));
                    }
                }
                if self.params.verbose {
                    eprintln!("MIP: shared {} root cuts with the bisection threads", out.len());
                }
                s.set_cuts(out, self.prune_limit());
            }
        }
        // 根のヒューリスティクス・裏のカットの後: 暫定解がカットの後の根の下界に届いていれば最適で終える (木に入ると、
        // 根のノードを刈る前に時間切れの判定があるので、ヒューリスティクスが期限を越えると時間切れで返していた)
        if self.root_cut_gap_closed(root_obj) {
            let lb = self.root_bound_now(root_obj);
            return self.finish_root_gap("root LP bound after cuts and heuristics", lb);
        }
        self.store_root_redcost(root_obj);
        self.root_redcost_fixing();
        if dbg_root && !self.dbg_contains() {
            self.dbg_lost("root redcost fixing", true);
        }
        // `ENOMOTO_MIP_ROOT_OBJ_PROBE_SUBMIP`: サブ MIP (RENS・RINS など) の根でも行う。サブ MIP では渡された打ち切り値
        // (元の問題の暫定解) も使う
        let probe_sub = env_str!("ENOMOTO_MIP_ROOT_OBJ_PROBE_SUBMIP").is_some();
        if env_str!("ENOMOTO_MIP_NO_ROOT_OBJ_PROBE").is_none() && self.snap.is_none() && (!self.params.submip || probe_sub) {
            let proved = self.root_obj_probe();
            if dbg_root && !self.dbg_contains() {
                self.dbg_lost("root objective probing", true);
            }
            // 0-1 列のどちらの値でも打ち切り値より良い解がない: 自分の暫定解が最適 (打ち切り値がほかのスレッドの解から
            // 来ているときは除く)
            if proved {
                if let Some(z) = self.incumbent.as_ref().map(|(z, _)| *z) {
                    if self.shared.as_ref().is_none_or(|s| !(s.best() < z)) {
                        if self.params.verbose {
                            eprintln!("MIP: root objective probing proved the incumbent {z:.10e} optimal ({:.2}s)", self.start.elapsed().as_secs_f64());
                        }
                        return self.finish(MipStatus::Optimal, z);
                    }
                } else if self.params.submip {
                    // サブ MIP で自分の暫定解がない: 渡された打ち切り値より良い解はない
                    if self.params.verbose {
                        eprintln!("MIP: root objective probing: no solution better than the cutoff in the sub-MIP");
                    }
                    return self.finish(MipStatus::Infeasible, f64::INFINITY);
                }
            }
        }
        if self.snap.is_none() {
            if let Some(r) = self.maybe_restart() {
                return r;
            }
        }
        let prot = self.basis_protected_cuts();
        // 木の写しから解くときは、根のノードの代わりに写しのノードを入れる (定義域は取り出すたびに根から積み直す)
        let mut first = true;
        if let Some(snap) = self.snap.clone() {
            for (k, n) in snap.nodes.iter().enumerate() {
                let tags = Some(Rc::new(vec![snap.serial << 32 | k as u64]));
                self.queue.push(OpenNode { changes: n.changes.clone(), lower_bound: n.lower_bound.max(root_obj), estimate: n.estimate, depth: n.depth, basis: None, basis_epoch: 0, dse: None, branch: n.branch, prot: None, tags });
            }
            self.snap_alive = vec![true; snap.nodes.len()];
            first = false;
        } else {
            self.queue.push(OpenNode { changes: Vec::new(), lower_bound: root_obj, estimate: root_obj, depth: 0, basis: Some(Rc::new(self.lp.basis())), basis_epoch: self.row_log.len(), dse: if dse_all() { self.lp.dse_snapshot().map(Rc::new) } else { None }, branch: None, prot, tags: None });
        }
        // 根のノードは LP を解いた状態のままなので、最初の取り出しでは定義域・LP を作り直さない。
        // 木を分け合う従のスレッドは、未処理のノードが 2 つ以上になってから (根を分枝してから) 始める (根で解ける問題では
        // 始めない。nw04: 根で閉じるのに、従の準備 (8.7 万列の定義域・LP と根の LP) を待って 4 本で 2-3 秒遅かった)
        let parallel_tree = self.params.threads > 1 && !self.params.submip && self.watch.is_none() && self.snap.is_none();
        // 潜っている子ノード (定義域に分枝を積んだ状態で次に処理する)。
        let mut plunge: Option<OpenNode> = None;
        let mut plunge_depth = 0usize;
        let mut plunge_start = 0u64;
        // 診断用 (`ENOMOTO_MIP_BRANCH_REPLAY`): HiGHS の木の分枝をなぞる ([`super::replay`])
        let replay = super::replay::get().filter(|_| !self.params.submip && !parallel_tree && self.snap.is_none());
        if replay.is_some() {
            self.queue.prio_only = true;
        }
        // 診断用 (`ENOMOTO_MIP_ORACLE_CUTS=ファイル`): 他のソルバーが作ったカット (HiGHS の計装の `HCC 右辺 長さ 列:係数 ...` 行、
        // 列は元の問題の番号) を木の前にカットのプールに入れる (`ENOMOTO_MIP_CUTPOOL_PROP` と合わせてノードで伝播する)
        if let Some(f) = env_str!("ENOMOTO_MIP_ORACLE_CUTS").filter(|_| !self.params.submip && self.snap.is_none()) {
            let s = std::fs::read_to_string(f).unwrap_or_default();
            let mut k = 0;
            self.pool_nocap = true;
            for l in s.lines().filter(|l| l.starts_with("HCC ")) {
                let t: Vec<&str> = l.split_whitespace().collect();
                let rhs: f64 = t[1].parse().unwrap();
                let coefs: Vec<(usize, f64)> = t[3..].iter().map(|x| {
                    let (a, b) = x.split_once(':').unwrap();
                    (a.parse().unwrap(), b.parse().unwrap())
                }).collect();
                if coefs.iter().all(|&(j, _)| j < self.p.n) {
                    self.add_to_pool(&coefs, rhs);
                    k += 1;
                }
            }
            self.pool_nocap = false;
            if let Some((c, r, _)) = self.cut_pool.get(self.cut_pool.len().saturating_sub(k)) {
                eprintln!("MIP: oracle cuts: first in the pool {:?} <= {r}, is_int {:?}", c, c.iter().map(|&(j, _)| self.p.is_int[j]).collect::<Vec<_>>());
            }
            eprintln!("MIP: oracle cuts: {k} added to the cut pool from {f} (pool {})", self.cut_pool.len());
        }
        // [なぞったノード, ファイルにないノード, 指定の列で分枝, 自前で分枝, 指定の列が固定済み]
        let mut rp_stats = [0u64; 5];
        // `ENOMOTO_MIP_TREE_RESTART` (試作): 木に入る前の暫定解の印は消しておく (木の中で良くなったときだけ判定する)
        self.tree_restart_check = false;
        loop {
            // `ENOMOTO_MIP_TREE_RESTART` (試作): 木の中で暫定解が良くなり、根の被約費用で新たに固定された整数列が
            // `ENOMOTO_T_MIP_TREE_RESTART_FRAC` (0.05) 以上、主のノード数が `ENOMOTO_T_MIP_TREE_RESTART_MAX_NODES` (2000) 以下、
            // 残り時間が時間上限の `ENOMOTO_T_MIP_TREE_RESTART_MIN_LEFT` (0.1) 以上なら、従を止めて再スタートする
            if std::mem::take(&mut self.tree_restart_check) && env_str!("ENOMOTO_MIP_TREE_RESTART").is_some() && self.watch.is_none() {
                let left_ok = match self.deadline {
                    Some(d) => d.saturating_duration_since(Instant::now()).as_secs_f64() >= tunable!("ENOMOTO_T_MIP_TREE_RESTART_MIN_LEFT", 0.1, f64) * self.params.time_limit,
                    None => true,
                };
                if left_ok && self.nodes <= tunable!("ENOMOTO_T_MIP_TREE_RESTART_MAX_NODES", 2000u64, u64) && self.proto_restart_ok(tunable!("ENOMOTO_T_MIP_TREE_RESTART_FRAC", 0.05, f64)) {
                    if self.params.verbose {
                        eprintln!("MIP: tree restart: {} newly fixed integer columns at node {} ({:.2}s)", self.newly_fixed_ints(), self.nodes, self.start.elapsed().as_secs_f64());
                    }
                    self.join_workers();
                    // 従を止める合図は `time_up` にも効くので、合流させた後は下ろす (下ろさないと再スタートが期限切れとして断られる)
                    if let Some(pool) = self.pool.as_ref() {
                        pool.stop.store(false, std::sync::atomic::Ordering::Relaxed);
                        pool.halt.store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                    if let Some(r) = self.maybe_restart() {
                        return r;
                    }
                    // 判定は通したので普通はここに来ない。従は止めたので、主だけで続ける
                }
            }
            // ポートフォリオ並列: 下界を知らせる (二分探索のスレッドが目標値を決めるのに使う)
            // 二分探索のスレッドの求解の下界は打ち切り値で絞った問題のものなので、木の写しを使う形では知らせない
            if self.nodes % 64 == 0 && (self.watch.is_none() || env_str!("ENOMOTO_MIP_BISECT_SNAPSHOT").is_none()) {
                if let Some(s) = &self.shared {
                    let mut lb = self.queue.best_lower_bound();
                    if let Some(n) = plunge.as_ref() {
                        lb = lb.min(n.lower_bound);
                    }
                    if lb.is_finite() && self.queue.len() > 0 {
                        s.publish_lb(lb);
                    }
                }
            }
            // 木の写し: 木の探索のスレッドは、写しの求め・二分探索のスレッドが示した値・見つけた解を受け取る。写しから解く
            // 求解は、写しのノードの部分木を探索し終えたら知らせる
            if self.snap.is_some() {
                if self.nodes % 64 == 0 {
                    self.snap_report(plunge.as_ref());
                }
            } else if self.watch.is_none() && !self.params.submip && self.shared.is_some() {
                self.snap_sync(&mut plunge);
            }
            // 木を分け合う従: ほかのスレッドの解で共有の最良値が下がったら、根の被約費用による固定をやり直して待ち行列を刈る
            // (自分の暫定解が良くなったときと同じ。主は従の解を受け取るときに try_incumbent で行う)
            if self.pool.is_some() && self.snap.is_some() && env_str!("ENOMOTO_MIP_TREE_NO_SHARED_FIX").is_none() {
                let b = self.shared.as_ref().map_or(f64::INFINITY, |s| s.best());
                if b < self.seen_best {
                    self.seen_best = b;
                    self.root_redcost_fixing();
                    let lim = self.prune_limit();
                    self.prune_queue(lim);
                }
            }
            // 従を始める: 待ち行列のノードが `ENOMOTO_T_MIP_TREE_SPAWN_NODES` 個 (既定 2、根を分枝した後) たまったら始め、
            // ノードを渡し始めるのは `ENOMOTO_T_MIP_TREE_START_NODES` 個 (既定 8) たまるか `ENOMOTO_T_MIP_TREE_HOLD_SECS` 秒
            // (既定 1) 経ってから (従の準備を主の探索と並行させる)。`ENOMOTO_MIP_TREE_LATE_SPAWN` なら以前と同じく
            // START_NODES 個たまってから始めてすぐ渡す
            let late = env_str!("ENOMOTO_MIP_TREE_LATE_SPAWN").is_some();
            let spawn_at = if late { tunable!("ENOMOTO_T_MIP_TREE_START_NODES", 8usize, usize) } else { tunable!("ENOMOTO_T_MIP_TREE_SPAWN_NODES", 2usize, usize) };
            if parallel_tree && self.pool.is_none() && self.queue.len() + plunge.is_some() as usize >= spawn_at {
                self.start_workers(&mut plunge);
                if !late {
                    self.pool_hold = Some(Instant::now() + Duration::from_secs_f64(tunable!("ENOMOTO_T_MIP_TREE_HOLD_SECS", 1.0, f64)));
                }
            }
            if self.pool_hold.is_some_and(|t| Instant::now() >= t || self.queue.len() >= tunable!("ENOMOTO_T_MIP_TREE_START_NODES", 8usize, usize)) {
                self.pool_hold = None;
            }
            // racing ramp-up: 競争の終わり (主が決める) と、負けたスレッドの木を捨てる
            if self.pool.is_some() && !self.race_done && self.race_step(&mut plunge) {
                continue;
            }
            // 木を分け合う: 大域的な境界の締め付けを共有する (64 ノードおき)
            let t_sync = Instant::now();
            if self.pool.is_some() && self.nodes % 64 == 0 && env_str!("ENOMOTO_MIP_TREE_NO_SHARED_BOUNDS").is_none() {
                self.sync_global_bounds();
            }
            // 木を分け合う: ほかのスレッドの双対証明・衝突を取り込む (64 ノードおき)
            if self.pool.is_some() && self.nodes % 64 == 0 && env_str!("ENOMOTO_MIP_TREE_NO_SHARED_PROOFS").is_none() {
                self.sync_learned();
            }
            // ヒューリスティクス役の起動の判定のため、自分の未処理のノードの重みを知らせる (64 ノードおき)
            if self.pool.as_ref().is_some_and(|p| p.heur_k > 0) && heur_trigger_mode() && self.nodes % 64 == 0 {
                self.publish_tree_weight(plunge.as_ref());
            }
            // 木を分け合う: 擬似費用の観測を合わせる (`ENOMOTO_T_MIP_TREE_PC_SYNC` ノードおき、0 で合わせない)
            let pc_every = tunable!("ENOMOTO_T_MIP_TREE_PC_SYNC", 16u64, u64);
            if pc_every > 0 && self.nodes % pc_every == 0 {
                if let Some(pool) = self.pool.clone() {
                    let mut base = self.pc_base.take().unwrap_or_else(|| self.snap.as_ref().map_or_else(|| self.pc.clone(), |sn| sn.pc.clone()));
                    {
                        let mut g = pool.pc.lock().unwrap();
                        self.pc.merge_shared(&mut base, &mut g);
                    }
                    self.pc_base = Some(base);
                }
            }
            // 木を分け合う: 待っているスレッドがいれば、自分の待ち行列から下界の小さいノードを渡す
            if let Some(pool) = &self.pool {
                use std::sync::atomic::Ordering;
                let (idle, len) = (pool.idle.load(Ordering::Relaxed), pool.len.load(Ordering::Relaxed));
                if tree_shared_queue() && self.snap.is_some() && self.race_done {
                    // 共有の待ち行列だけで探索する: 自分の待ち行列のノードを全て出す
                    if !self.queue.is_empty() && self.pool_hold.is_none() {
                        self.pool_donate_all();
                    }
                } else if tree_keep_siblings() && self.race_done {
                    // 今の潜りの兄弟 (`dive_stack`) だけを手元に残し、ほかは全て共有の待ち行列に出す
                    if self.queue.len() > self.dive_stack.len() && self.pool_hold.is_none() {
                        self.pool_donate_except_siblings();
                    }
                } else if idle > len && self.queue.len() >= 2 && self.pool_hold.is_none() {
                    self.pool_donate(idle - len);
                }
                // `ENOMOTO_T_MIP_TREE_LOCAL_CAP` = K (既定 1000、0 で使わない): 自分の待ち行列を K ノードまでにし、超えた分は下界の
                // 小さい順に共有の待ち行列へ出す (手元の近いノードを続けて処理する利点を残したまま、手元が数千ノードに膨らむのを防ぎ、
                // 浅いノードをほかのスレッドに回す)。40 問 (2 回、`idle` の修正と組み): 解けた数 24.5 -> 25.0、SGM 20.58 -> 20.74 秒、
                // 主ギャップの和 2.006 -> 1.949、noswot が 2 回とも解ける (38.6・31.7 秒)。上限 2・8 (手元をほぼ空にする) は証明が主の
                // 問題で遅かった (h80x6320d・eil33-2・neos-860300)
                // `ENOMOTO_T_MIP_TREE_DONATE_HALF_SECS` = T (> 0、既定 0 = 使わない): T 秒おきに、自分の待ち行列の半分を下界の小さい順に
                // 共有の待ち行列へ出す (役の復帰で `idle` が下に回った状態では毎ノード半分を出していた。その間隔を決めて、役の有無によらず行う)
                let half_secs = tunable!("ENOMOTO_T_MIP_TREE_DONATE_HALF_SECS", 0.0, f64);
                if half_secs > 0.0 && self.race_done && self.queue.len() >= 2 && self.pool_hold.is_none() {
                    let due = self.donate_half_at.is_none_or(|t| t.elapsed().as_secs_f64() >= half_secs);
                    if due {
                        self.donate_half_at = Some(Instant::now());
                        self.pool_donate_n(self.queue.len() / 2);
                    }
                }
                let cap = tunable!("ENOMOTO_T_MIP_TREE_LOCAL_CAP", 1000usize, usize);
                if cap > 0 && self.race_done && self.queue.len() > cap && self.pool_hold.is_none() {
                    self.pool_donate_n(self.queue.len() - cap);
                }
                if !tree_shared_queue() && !tree_keep_siblings() && env_str!("ENOMOTO_MIP_TREE_NO_BALANCE").is_none() && self.nodes % tunable!("ENOMOTO_T_MIP_TREE_BALANCE_EVERY", 32u64, u64) == 0 {
                    self.pool_balance();
                }
                self.tprof.sync_s += t_sync.elapsed().as_secs_f64();
            }
            // ヒューリスティクス役 (`ENOMOTO_MIP_HEUR_ROLE`): 番号の大きい従は、解ができたら木を離れて大近傍探索を回し続ける
            if plunge.is_none() && !first && self.heur_role_mine() {
                match self.heur_role_run() {
                    Some(true) => break,
                    Some(false) => return self.finish_limit(MipStatus::TimeLimit, None),
                    None => continue,
                }
            }
            // 目的値の二分探索で、暫定解より良い解がないことを証明した (下界が打ち切り値に届いた)
            if self.incumbent.is_some() && self.bisect_lb >= self.prune_limit() {
                if self.params.verbose {
                    eprintln!("MIP: objective bisection closed the gap (lower bound {:.10e})", self.bisect_lb);
                }
                let z = self.incumbent.as_ref().map(|(z, _)| *z).unwrap();
                return self.finish(MipStatus::Optimal, z);
            }
            // 診断用: 直前のノードがデバッグ解の像を含んでいたのに、未処理のノードがどれも像を含まなくなったら知らせる
            if self.dbg_prev_has && self.dbg_alive && !self.params.submip {
                self.dbg_prev_has = false;
                let dz = DEBUG_SOL.with(|d| d.borrow().as_ref().map(|x| self.p.objective(x)));
                if dz.is_some_and(|z| z < self.prune_limit() - 1e-6) {
                    let nodes: Vec<OpenNode> = self.queue.iter().cloned().chain(plunge.iter().cloned()).collect();
                    let alive = nodes.iter().any(|n| self.dbg_node_has_image(n));
                    if !alive {
                        self.dbg_alive = false;
                        eprintln!("MIP_DEBUG_SOL: all symmetric images gone after node {} (last pruning: {})", self.nodes, self.dbg_last.borrow());
                    }
                }
            }
            if self.time_up() {
                return self.finish_limit(MipStatus::TimeLimit, plunge.as_ref());
            }
            if self.nodes >= self.params.node_limit {
                return self.finish_limit(MipStatus::NodeLimit, plunge.as_ref());
            }
            // 潜りが枝刈りで終わったら、潜った道の兄弟ノード (新しいものから) に戻る (HiGHS の backtrackPlunge)。
            // 潜りの打ち切りの条件 (下界が 全体の下界 + q (打ち切り値 - 全体の下界) 以下) を満たすものだけ
            let mut sib_lu = None;
            let sibling = if replay.is_some() && plunge.is_none() && !first {
                // なぞる: 兄弟が HiGHS の次のノードのときだけ戻る
                let mut got = None;
                if let (Some(top), Some((fid, fgen, _))) = (self.dive_stack.last(), self.queue.first_prio()) {
                    if top.0 == fid && top.1 == fgen {
                        let (id, gen, lu) = self.dive_stack.pop().unwrap();
                        got = self.queue.take(id, gen, f64::INFINITY);
                        sib_lu = lu;
                    }
                }
                got
            } else if plunge.is_none()
                && !first
                && env_str!("ENOMOTO_MIP_NO_SIBLING_BACKTRACK").is_none()
                && !(self.params.submip && env_str!("ENOMOTO_MIP_NO_SIBLING_BACKTRACK_SUB").is_some())
                // 1 回の潜り (兄弟への戻りを含む) のノード数の上限 (HiGHS: min(1000, ノード数 / 10))。
                // HiGHS 流のカット管理では HiGHS の processNodes の plungeLimit と同じ `ENOMOTO_T_MIP_HCM_PLUNGE_NODES` (既定 100):
                // 分離は待ち行列から取り出したノードでしか行わないので、潜りが 1000 ノード続くと分離がほとんど起きない
                // (neos-911970: 14000 ノード中、待ち行列からの取り出しは 70 回)
                && self.nodes - plunge_start < if self.highs_cutmgmt() { tunable!("ENOMOTO_T_MIP_HCM_PLUNGE_NODES", 100u64, u64) } else { (self.nodes / tunable!("ENOMOTO_T_MIP_PLUNGE_NODES_DIV", 10u64, u64)).min(1000) }
            {
                let cutoff = self.prune_limit();
                let glb = self.queue.best_lower_bound();
                let limit = if cutoff.is_finite() { glb + tunable!("ENOMOTO_T_MIP_SIBLING_QUOT", 0.25, f64) * (cutoff - glb) } else { f64::INFINITY };
                let mut got = None;
                while let Some((id, gen, lu)) = self.dive_stack.pop() {
                    if let Some(n) = self.queue.take(id, gen, limit.min(cutoff)) {
                        got = Some(n);
                        sib_lu = lu;
                        break;
                    }
                }
                got
            } else {
                None
            };
            let from_sibling = sibling.is_some();
            let from_plunge = plunge.is_some();
            let mut pre_qlb = f64::NAN;
            self.tp_kind = 0;
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
                        if replay.is_some() {
                            self.rp_log(&n, "tight", f64::NAN, f64::NAN, -1, &mut rp_stats);
                        }
                        continue;
                    }
                    if let Some(b) = &n.basis {
                        self.restore_node_basis(b, n.basis_epoch, n.dse.as_ref().map(|v| v.as_slice()));
                        // 置いた基底が親の LP の最適基底のままなら、その LU を使う (行が変わっていれば使わない)
                        if let Some(s) = &sib_lu {
                            if n.basis_epoch == self.row_log.len() {
                                self.lp.adopt_lu_snapshot(s);
                            }
                        }
                    }
                    n
                }
                None => {
                    plunge_depth = 0;
                    plunge_start = self.nodes;
                    self.dive_stack.clear();
                    let lim = self.prune_limit();
                    self.prune_queue(lim);
                    // 下界最小のノードを選ぶ頻度: 暫定解があれば上げる (下界を押し上げる)
                    self.proof_phase_check();
                    let bb_every = if self.in_proof_phase() {
                        tunable!("ENOMOTO_T_MIP_PROOF_BB_EVERY", 1u64, u64)
                    } else if self.has_cutoff_value() {
                        tunable!("ENOMOTO_T_MIP_BB_EVERY_INC", 4u64, u64)
                    } else {
                        tunable!("ENOMOTO_T_MIP_BB_EVERY", 10u64, u64)
                    };
                    pre_qlb = self.queue.best_lower_bound();
                    let n = match self.queue.pop(bb_every) {
                        Some(n) => {
                            self.tp_kind = 1;
                            // `ENOMOTO_T_MIP_SUBMIP_STALL` (サブ MIP の中だけ、既定 12、0 なら使わない): 暫定解があり、待ち行列から取ったノードの
                            // 推定値が打ち切り値以上のことが続けてこの回数あれば止める (HiGHS の `mip_max_stall_nodes`、サブ MIP では 12。
                            // 改善の見込みのないサブ MIP を早く切り、1 回の時間の上限を短くしなくて済むようにする)
                            // `ENOMOTO_MIP_SUBMIP_OUTCOME_STOP` (試作): 根のサブ MIP は、解を見つけてから見つけるまでと同じだけ進めて良くならなければ止める
                            if let (Some(_), Some(k)) = (self.root_submip.as_ref(), self.last_improve_node) {
                                if self.nodes.saturating_sub(k) >= k.max(tunable!("ENOMOTO_T_MIP_SUBMIP_OUTCOME_MIN_NODES", 50u64, u64)) {
                                    return self.finish_limit(MipStatus::NodeLimit, Some(&n));
                                }
                            }
                            let stall_lim = if self.params.submip { tunable!("ENOMOTO_T_MIP_SUBMIP_STALL", 12u64, u64) } else { 0 };
                            if stall_lim > 0 && self.incumbent.is_some() {
                                if n.estimate >= self.prune_limit() {
                                    self.submip_stall += 1;
                                    if self.submip_stall >= stall_lim {
                                        return self.finish_limit(MipStatus::NodeLimit, Some(&n));
                                    }
                                } else {
                                    self.submip_stall = 0;
                                }
                            }
                            n
                        }
                        // 木を分け合う: 共有の待ち行列から取る (全てのスレッドが待っていて空なら探索は終わり)
                        None => match self.pool_acquire() {
                            Ok(Some(n)) => {
                                self.tp_kind = 2;
                                n
                            }
                            Ok(None) => break,
                            Err(()) => return self.finish_limit(MipStatus::TimeLimit, None),
                        },
                    };
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
                                self.restore_node_basis(b, n.basis_epoch, n.dse.as_ref().map(|v| v.as_slice()));
                            }
                        }
                        if !ok {
                            self.dbg_lost("propagation while replaying branches", self.dbg_changes(&n.changes));
                            self.nodes += 1;
                            if replay.is_some() {
                                self.rp_log(&n, "tight", f64::NAN, f64::NAN, -1, &mut rp_stats);
                            }
                            continue;
                        }
                    }
                    n
                }
            };
            xp("m_pop");
            if lbnode_diag() {
                let k = if from_plunge {
                    0
                } else if from_sibling {
                    1
                } else if self.tp_kind == 2 {
                    4
                } else if node.lower_bound <= pre_qlb {
                    2
                } else {
                    3
                };
                // 取り出した後の、ほかのノード (自分の待ち行列・共有の待ち行列・ほかのスレッド) の下界の最小
                let mut other = self.queue.best_lower_bound();
                if let Some(pool) = &self.pool {
                    other = other.min(pool.inner.lock().unwrap().nodes.min_lower_bound());
                    for (i, sl) in pool.slots.iter().enumerate() {
                        if i != self.pool_tid as usize {
                            other = other.min(f64::from_bits(sl.lb.load(std::sync::atomic::Ordering::Relaxed)));
                        }
                    }
                }
                let now = Instant::now();
                if let Some((pk, t)) = self.lbd.last {
                    self.lbd.secs[pk] += now.duration_since(t).as_secs_f64();
                }
                self.lbd.last = Some((k, now));
                self.lbd.n[k] += 1;
                if node.lower_bound <= other + 1e-9 * other.abs().max(1.0) {
                    self.lbd.at_min[k] += 1;
                }
            }
            // 診断用 (`ENOMOTO_T_MIP_TREE_DIAG_WAITING_SLEEP_MS`): 待っている側に数えられたままノードを処理するスレッドを眠らせ、
            // その間にほかのスレッドが探索の終わりを判定する競合を起こしやすくする
            if self.pool.is_some() && !self.pool_busy {
                let ms = tunable!("ENOMOTO_T_MIP_TREE_DIAG_WAITING_SLEEP_MS", 0u64, u64);
                if tree_diag() {
                    eprintln!("MIP_TREE_DIAG: tid {} processes a node (lower bound {:.10e}) while counted as waiting", self.pool_tid, node.lower_bound);
                }
                if ms > 0 {
                    std::thread::sleep(Duration::from_millis(ms));
                }
            }
            if first {
                self.tprof.tree_start_s = self.start.elapsed().as_secs_f64();
            }
            first = false;
            self.nodes += 1;
            self.tprof.nodes += 1;
            if self.find_thread.is_some() {
                self.poll_find_thread();
            }
            if prof_tree_par() {
                self.tprof.lbs.push(node.lower_bound);
            }
            if self.dbg_alive && DEBUG_SOL.with(|d| d.borrow().is_some()) && !self.params.submip {
                self.dbg_prev_has = self.dbg_node_has_image(&node);
            }
            self.note_current_lb(node.lower_bound);
            self.log(false);
            let dn = self.dbg_changes(&node.changes);
            let rp_entry = replay.and_then(|_| super::replay::lookup(&node.changes));
            let mut rp_stage = "";
            let (mut rp_obj0, mut rp_obj) = (f64::NAN, f64::NAN);
            if node.lower_bound >= self.prune_limit() {
                if self.dbg_on {
                    self.dbg_lost(&format!("node lower bound {}", node.lower_bound), dn);
                }
                self.pc.objlim_leaves += 1;
                if replay.is_some() {
                    self.rp_log(&node, "lb", f64::NAN, f64::NAN, -1, &mut rp_stats);
                }
                if tree_log() && !self.params.submip {
                    eprintln!("ENODE d={} r=pop_lb lp={} rows={}", node.depth, node.lower_bound, self.lp.num_rows());
                }
                continue;
            }
            self.dbg_trace("after replaying the branches");
            let stack_before_prop = self.dom.stack_len();
            if dn && !self.dbg_contains() {
                self.dbg_lost("domain excludes the debug solution before node propagation", dn);
                self.dbg_explain_exclusion(&node.changes);
            }
            let prop_ok = self.dom.propagate(self.p);
            if !prop_ok {
                rp_stage = "p_dom";
            }
            self.dbg_trace("after propagation");
            if !prop_ok {
                self.add_conflict();
            }
            if let Some((j, up, _, _)) = node.branch {
                self.pc.add_inference(j, up, self.dom.stack_len().saturating_sub(stack_before_prop) as f64);
            }
            xp("m_prop");
            // 目的関数の伝播 (目的値 <= 打ち切り値)
            let prop_ok = prop_ok && self.propagate_objective();
            if !prop_ok && rp_stage.is_empty() {
                rp_stage = "p_obj";
            }
            self.dbg_trace("after objective propagation");
            // 双対証明による枝刈りと境界の締め付け
            let prop_ok = prop_ok && {
                let pos_pf = self.dom.stack_len();
                self.proof_last_prune = None;
                self.proof_prune_after_tighten = false;
                let mut ok = self.apply_dual_proofs();
                if ok && proof_fixpoint() {
                    for _ in 0..20 {
                        let before = self.dom.stack_len();
                        ok = self.apply_dual_proofs();
                        if !ok || self.dom.stack_len() == before {
                            break;
                        }
                    }
                }
                if !ok {
                    if self.dom.conflict.is_some() && env_str!("ENOMOTO_MIP_NO_EXTRA_CONFLICTS").is_none() {
                        self.add_conflict();
                    }
                    self.proof_prunes += 1;
                    self.dbg_lost("dual proof", dn && self.dbg_contains());
                    if proof_verify() {
                        self.proof_verify_lp(pos_pf, node.lower_bound, dn);
                    }
                }
                ok
            };
            if !prop_ok && rp_stage.is_empty() {
                rp_stage = "p_proof";
            }
            // 境界リテラルの衝突による伝播
            let prop_ok = prop_ok && {
                // 前回調べた後の変更だけを調べる (巻き戻しでそれより短くなっていればそこから、衝突が増えていれば最初から)
                let lw = self.dom.take_low_water();
                let start = if self.lit_conflicts.dirty || env_str!("ENOMOTO_MIP_LIT_FULL_SCAN").is_some() { 0 } else { self.lit_pos.min(lw) };
                self.lit_conflicts.dirty = false;
                let r = self.lit_conflicts.propagate(self.p, &mut self.dom, start);
                self.share_used_lits();
                self.lit_pos = self.dom.stack_len();
                self.dom.take_low_water();
                match r {
                    Ok(k) => {
                        self.lit_stats[2] += k as u64;
                        true
                    }
                    Err(()) => {
                        if self.dom.conflict.is_some() && env_str!("ENOMOTO_MIP_NO_EXTRA_CONFLICTS").is_none() {
                            self.add_conflict();
                        }
                        self.lit_stats[1] += 1;
                        self.dbg_lost("literal conflict", dn && self.dbg_contains());
                        false
                    }
                }
            };
            // カットプールの伝播 (`ENOMOTO_MIP_CUTPOOL_PROP`、[`Self::propagate_cut_pool`])
            if !prop_ok && rp_stage.is_empty() {
                rp_stage = "p_lit";
            }
            let prop_ok = prop_ok && (!cutpool_prop() || self.propagate_cut_pool());
            if !prop_ok && rp_stage.is_empty() {
                rp_stage = "p_cpool";
            }
            xp("m_proof");
            self.dbg_trace("after dual proofs and literal conflicts");
            // 完全オービトープの固定 (列を辞書式で減少に並べた解だけを残す)
            let prop_ok = prop_ok && {
                let ok = self.orbitope_propagate(&node.changes);
                if !ok {
                    if self.dom.conflict.is_some() && env_str!("ENOMOTO_MIP_NO_EXTRA_CONFLICTS").is_none() {
                        self.add_conflict();
                    }
                    self.orbitope_prunes += 1;
                }
                ok
            };
            self.dbg_trace("after orbitopal fixing");
            if !prop_ok && rp_stage.is_empty() {
                rp_stage = "p_orb";
            }
            // `ENOMOTO_MIP_PROP_FIXPOINT` (試作): オービトープの固定などで境界が変わったら、行・目的関数・双対証明・リテラルの衝突・
            // カットプール・オービトープの伝播を、境界が変わらなくなるまで (最大 `ENOMOTO_T_MIP_PROP_FIXPOINT_PASSES` 回) 回し直す
            // (HiGHS は対称性の固定・行・カットプール・衝突プールを 1 つの伝播の不動点まで回す。こちらは各段を 1 回ずつ順に回すので、
            // 後の段 (オービトープ) で締まった境界が前の段 (カットプール・証明) に戻らない)
            let prop_ok = prop_ok && (!prop_fixpoint() || {
                let ok = self.node_prop_fixpoint(&node.changes);
                if !ok && rp_stage.is_empty() {
                    rp_stage = "p_fix";
                }
                ok
            });
            if !prop_ok {
                if replay.is_some() {
                    self.rp_log(&node, rp_stage, f64::NAN, f64::NAN, -1, &mut rp_stats);
                }
                self.pc.infeasible_leaves += 1;
                if tree_log() && !self.params.submip {
                    eprintln!("ENODE d={} r=prop lp={} rows={}", node.depth, node.lower_bound, self.lp.num_rows());
                }
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
            // ノードの分離でカットを加えた後の解き直しか (その LP の時間を `node_cut_secs` に足す)
            let mut node_cut_resolve: Option<Instant> = None;
            // ノードの分離 ([`Solver::node_cuts_due`]) の前の LP の値 (解き直しの後に効き目を測る)
            let mut fc_obj0: Option<f64> = None;
            // HiGHS 流のカット管理: このノードでの分離のラウンド数と、前のラウンドの前の目的値
            let mut hcm_sep_rounds = 0usize;
            let mut hcm_sep_last_obj = f64::NEG_INFINITY;
            // 効き目で続けるノードの分離 (`ENOMOTO_MIP_NODE_CUT_ADAPT`): このノードでの分離のラウンド数と、直前のラウンドが効いたか
            let mut nca_rounds = 0usize;
            let mut nca_last_ok = false;
            // `ENOMOTO_MIP_FC_NODE_SEP2`: このノードでの分離のラウンド数 (分離しても加わらなければ打ち止め)、最初のラウンドの前と
            // 前のラウンドの前の LP の値
            let mut fc2_rounds = 0usize;
            let mut fc2_first = f64::NEG_INFINITY;
            let mut fc2_last = f64::NEG_INFINITY;
            // `ENOMOTO_MIP_TREE_SEP`: このノードでのラウンド数 (加わらなければ打ち止め)、最初のラウンドの前と前のラウンドの前の LP の値
            let mut ts_rounds = 0usize;
            let mut ts_first = f64::NEG_INFINITY;
            let mut ts_last = f64::NEG_INFINITY;
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
                // `ENOMOTO_MIP_RESTORE_EXACT_DSE`: 待ち行列から取り出して基底を復元したノードの最初の LP だけ、DSE 重みを
                // その基底の厳密な値から始める (HiGHS は新しい基底で重みを計算し直す。単位重みのままだと反復が多い)。
                // neos-911970: 取り出したノードの最初の LP は 1 回 300 -> 98 反復、60 秒のノード数 2190 -> 6424 だが、
                // 探索の順が変わって下界 54.23 -> 53.09。40 問: 19 問 25.05 -> 25.90 (binkar10_1 21 -> 33 秒、mik-250 22 -> 28 秒、
                // neos5 37 -> 49 秒: 行ごとの BTRAN m 回の分が重い)。既定では使わない
                if resolves == 0 && self.restored_now && env_str!("ENOMOTO_MIP_RESTORE_EXACT_DSE").is_some() {
                    crate::simplex::slope_intercept_dual::DSE_EXACT_ONCE.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                if std::mem::take(&mut self.restored_dse) && resolves == 0 {
                    crate::simplex::slope_intercept_dual::WARM_DSE_ONCE.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                let mut st = self.lp.solve(&capped(self, iter_limit));
                crate::simplex::slope_intercept_dual::WARM_DSE_ONCE.store(false, std::sync::atomic::Ordering::Relaxed);
                crate::simplex::slope_intercept_dual::DSE_EXACT_ONCE.store(false, std::sync::atomic::Ordering::Relaxed);
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
                if let Some(t) = node_cut_resolve.take() {
                    self.node_cut_secs += t.elapsed().as_secs_f64();
                }
                if let Some(o0) = self.nca_pending.take() {
                    let ok = match st {
                        LpStatus::Infeasible | LpStatus::ObjectiveBound => {
                            self.nca_hist[0] += 1;
                            true
                        }
                        LpStatus::Optimal => {
                            let o = self.lp.objective() + self.p.offset;
                            let lim = self.prune_limit();
                            let gap = if lim.is_finite() { (lim - o0).max(1e-9) } else { o0.abs().max(1.0) };
                            let r = (o - o0) / gap;
                            self.nca_hist[if o >= lim { 0 } else if r < 0.02 { 1 } else if r < 0.1 { 2 } else if r < 0.3 { 3 } else { 4 }] += 1;
                            o >= lim || o - o0 >= tunable!("ENOMOTO_T_MIP_NODE_CUT_ADAPT_GAIN", 0.02, f64) * gap
                        }
                        _ => false,
                    };
                    let a = tunable!("ENOMOTO_T_MIP_NODE_CUT_ADAPT_ALPHA", 0.05, f64);
                    self.nca_ema = (1.0 - a) * self.nca_ema + a * if ok { 1.0 } else { 0.0 };
                    self.nca_stats[0] += 1;
                    self.nca_stats[1] += ok as u64;
                    if ok {
                        nca_last_ok = true;
                    }
                }
                self.node_iters += self.lp.total_iterations() - it0;
                if resolves == 0 {
                    self.node_iters_first += self.lp.total_iterations() - it0;
                    let k = std::mem::take(&mut self.tp_kind);
                    self.tprof.kind_n[k] += 1;
                    self.tprof.kind_it[k] += self.lp.total_iterations() - it0;
                    self.tprof.kind_s[k] += t_lp.elapsed().as_secs_f64();
                    if std::mem::take(&mut self.restored_now) {
                        self.restore_stats.2 += self.lp.total_iterations() - it0;
                        if env_str!("ENOMOTO_MIP_RESTORE_DIAG").is_some() {
                            let (a, d, mm) = self.restore_diag;
                            eprintln!("RESTDIAG: tid {} kind {} added {a} deleted {d} mismatch {mm} excess {} rows {} iters {}", self.pool_tid, k, if k == 2 { self.pool_excess } else { 0 }, self.lp.num_rows(), self.lp.total_iterations() - it0);
                        }
                    }
                }
                self.node_lps += 1;
                self.node_lp_rows += self.lp.num_rows() as u64;
                match st {
                    LpStatus::Optimal => {}
                    LpStatus::Infeasible | LpStatus::ObjectiveBound => {
                        if let Some(o) = fc_obj0.take() {
                            self.fc_record(o, f64::INFINITY);
                        }
                        if self.dbg_on {
                            self.dbg_trace(&format!("LP {st:?} (resolve {resolves}, LP rows {})", self.lp.num_rows()));
                        }
                        rp_obj = f64::INFINITY;
                        if resolves == 0 {
                            rp_obj0 = f64::INFINITY;
                        }
                        if tree_log() && !self.params.submip {
                            eprintln!("ENODE d={} r={} lp={} rows={}", node.depth, if st == LpStatus::Infeasible { "lpinf" } else { "lpobj" }, self.lp.objective() + self.p.offset, self.lp.num_rows());
                        }
                        if st == LpStatus::Infeasible {
                            self.pc.infeasible_leaves += 1;
                            self.add_farkas_proof();
                        } else {
                            self.pc.objlim_leaves += 1;
                            self.add_dual_proof();
                        }
                        if self.dbg_on {
                            self.dbg_lost(&format!("node LP {st:?}"), dn && self.dbg_contains());
                        }
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
                rp_obj = node_obj;
                if resolves == 0 {
                    rp_obj0 = node_obj;
                }
                if let Some(o) = fc_obj0.take() {
                    self.fc_record(o, node_obj);
                }
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
                    if tree_log() && !self.params.submip {
                        eprintln!("ENODE d={} r=lpobj lp={node_obj} rows={}", node.depth, self.lp.num_rows());
                    }
                    self.add_dual_proof();
                    self.pc.objlim_leaves += 1;
                    if self.dbg_on {
                        self.dbg_lost(&format!("node LP objective {node_obj}"), dn && self.dbg_contains());
                    }
                    break None;
                }
                let x = self.lp.col_values();
                let frac = self.fractional(&x);
                if frac.is_empty() {
                    if tree_log() && !self.params.submip {
                        eprintln!("ENODE d={} r=int lp={node_obj} rows={}", node.depth, self.lp.num_rows());
                    }
                    if self.try_lp_solution() {
                        break None;
                    }
                    // 許容誤差の中で整数の LP 解が、丸めると行に反する (big-M の行 x <= M y で y = 1e-10、x = M y): ノードを捨てると
                    // その先の解を失う (beavma の再スタートの後の問題で 383285 を捨て 383738.6 を最適と返した)。整数からのずれが
                    // 最大の固定されていない整数列で分枝する
                    let mut xr = x.clone();
                    for j in 0..self.p.n {
                        if self.p.is_int[j] {
                            xr[j] = xr[j].round();
                        }
                    }
                    if !self.p.is_feasible(&xr, FEASTOL) {
                        let pick = (0..self.p.n)
                            .filter(|&j| self.p.is_int[j] && self.dom.lo[j] < self.dom.up[j] && x[j] != x[j].round())
                            .max_by(|&a, &b| (x[a] - x[a].round()).abs().total_cmp(&(x[b] - x[b].round()).abs()));
                        if let Some(j) = pick {
                            break Some(Some((j, x[j], node_obj, vec![(j, x[j])])));
                        }
                    }
                    break None;
                }
                xp("m_frac");
                // 被約費用による局所的な固定。LP 解が新しい境界から外れたら解き直す。
                if self.has_cutoff_value() && self.local_redcost_fixing(node_obj) {
                    if dn && !self.dbg_contains() {
                        self.dbg_lost("local redcost fixing", true);
                    }
                    xp("r_fix");
                    if !self.dom.propagate(self.p) {
                        if tree_log() && !self.params.submip {
                            eprintln!("ENODE d={} r=redcost lp={node_obj} rows={}", node.depth, self.lp.num_rows());
                        }
                        self.add_conflict();
                        self.dbg_lost("propagation after local redcost fixing", dn && self.dbg_contains());
                        break None;
                    }
                    xp("r_prop");
                    self.sync_lp();
                    xp("r_sync");
                    let x2 = self.lp.col_values();
                    let moved = (0..self.p.n).any(|j| (self.p.is_int[j] || !lp_int_bounds()) && (x2[j] < self.dom.lo[j] - FEASTOL || x2[j] > self.dom.up[j] + FEASTOL));
                    if moved {
                        resolves += 1;
                        continue;
                    }
                }
                xp("m_redcost");
                // ノードのヒューリスティクス (安価な単純丸めは毎回、ランダム丸めは予算内で待ち行列から取り出したノードのみ)
                if resolves == 0 && !(tree_heur_off() && !self.params.submip) {
                    self.heur_label = "tree rounding";
                    self.simple_rounding(&x);
                    if env_str!("ENOMOTO_MIP_NO_ZIROUND").is_none() {
                        self.zi_round(&x);
                    }
                    self.heur_label = "";
                    let budget = self.lp.total_iterations() / 20 + 10_000;
                    // 大近傍探索 (サブ MIP) は反復の予算とは別に、経過時間の一定割合までの時間の予算で呼ぶ
                    // (サブ MIP の反復を共通の予算に数えると、1 回で使い切ってしばらく呼べなくなる)
                    // 割合は改善できた割合に連動させる (改善しない問題では 2%、改善するほど最大 10%)
                    // (成功 1 回の報酬は 1-2 なので、報酬の合計の半分を成功回数の目安にする)
                    let lns_calls: f64 = self.alns_count.iter().sum::<u32>() as f64;
                    let succ_rate = (0.5 * self.alns_reward.iter().sum::<f64>() + 1.0) / (lns_calls + 2.0);
                    let frac = tunable!("ENOMOTO_T_MIP_LNS_TIME_MIN", 0.02, f64) + tunable!("ENOMOTO_T_MIP_LNS_TIME_FRAC", 0.08, f64) * succ_rate;
                    // 既定は時間の予算で 50 ノードおき。`ENOMOTO_MIP_LNS_ITERS` なら以前の、ヒューリスティクス共通の反復の
                    // 予算の中で 100 ノードおき (根のサブ MIP の反復を数えるので、根で数万反復使った問題では木の中で
                    // 一度も呼ばれなかった: h80x6320d・timtab1・assign1-5-8・graph20)
                    let lns_time = env_str!("ENOMOTO_MIP_LNS_ITERS").is_none();
                    let lns_avail = frac * self.start.elapsed().as_secs_f64() - self.lns_secs;
                    let lns_ok = if lns_time { lns_avail > 0.0 } else { self.heur_iters - self.heur_iters_root < budget };
                    let lns_freq = if lns_time { tunable!("ENOMOTO_T_MIP_LNS_FREQ", 50u64, u64) } else { 100 };
                    // 木を分け合うときは一部のスレッドだけが大近傍探索・ダイビングを回す ([`Solver::tree_heur_allowed`])
                    let heur_ok = self.tree_heur_allowed();
                    // ヒューリスティクス役のスレッドに、待ち行列から取ったノードの LP 解を渡す (`ENOMOTO_T_MIP_HEUR_ROLE_LPX_EVERY` ノードおき)
                    if let Some(pool) = self.pool.as_ref().filter(|p| p.heur_k > 0) {
                        if node.depth > 0 && plunge_depth == 0 && self.nodes % tunable!("ENOMOTO_T_MIP_HEUR_ROLE_LPX_EVERY", 8u64, u64).max(1) == 0 {
                            *pool.lp_x.lock().unwrap() = Some(std::sync::Arc::new(x.clone()));
                        }
                    }
                    if heur_ok && node.depth > 0 && plunge_depth == 0 && self.incumbent.is_some() && self.nodes >= self.last_rins + lns_freq && lns_ok {
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
                        // 1 回のサブ MIP の時間は時間の予算の残りに合わせる (残りの 2 倍、最低
                        // `ENOMOTO_T_MIP_LNS_CALL_MIN` 秒)。1 回で 6 秒使うと、割合 2% の予算では以後ほぼ呼べなくなる
                        // (`ENOMOTO_MIP_LNS_NO_CAP` で上限なし)
                        let keep_max = self.submip_time_max;
                        if lns_time && env_str!("ENOMOTO_MIP_LNS_NO_CAP").is_none() {
                            let cap = (tunable!("ENOMOTO_T_MIP_LNS_CALL_MULT", 2.0, f64) * lns_avail).max(tunable!("ENOMOTO_T_MIP_LNS_CALL_MIN", 0.3, f64));
                            self.submip_time_max = self.submip_time_max.min(cap);
                        }
                        let mu0 = if self.pool.as_ref().is_some_and(|p| p.heur_k > 0) && heur_mu_mode() {
                            let z0 = self.shared.as_ref().map_or(f64::INFINITY, |s| s.best());
                            self.heur_trigger_eval().map(|ev| (ev, z0))
                        } else {
                            None
                        };
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
                        self.submip_time_max = keep_max;
                        self.lns_secs += t_lns.elapsed().as_secs_f64();
                        if let Some((ev, z0)) = mu0 {
                            let z1 = self.shared.as_ref().map_or(z0, |s| s.best()).min(self.incumbent.as_ref().map_or(z0, |(z, _)| *z));
                            self.mu_observe(&ev, z0, z1, t_lns.elapsed().as_secs_f64(), false);
                        }
                    } else if env_str!("ENOMOTO_MIP_OBJ_BISECT").is_some()
                        && !self.params.submip
                        && plunge_depth == 0
                        && self.incumbent.is_some()
                        && self.nodes >= self.bisect_last + tunable!("ENOMOTO_T_MIP_BISECT_FREQ", 500u64, u64)
                        && self.bisect_secs < tunable!("ENOMOTO_T_MIP_BISECT_TIME_FRAC", 0.25, f64) * self.start.elapsed().as_secs_f64()
                    {
                        self.bisect_last = self.nodes;
                        let lb = self.queue.best_lower_bound().min(node_obj);
                        let t_b = Instant::now();
                        self.objective_bisection(lb);
                        self.bisect_secs += t_b.elapsed().as_secs_f64();
                    } else if node.depth > 0 && plunge_depth == 0 && self.heur_iters - self.heur_iters_root < budget {
                        self.heur_label = "tree randomized rounding";
                        self.randomized_rounding(&x, 1);
                        self.heur_label = "";
                    }
                    // 暫定解がない間は木の中でも解探しを強める (`ENOMOTO_MIP_NOINC_HEUR` で有効。根で暫定解のない 2 問
                    // (neos-1456979・30n20b8) でも解は見つからなかった: HiGHS はどちらも根のカットで下界を大きく上げた後の
                    // 根のサブ MIP で最初の解を得ている。木の中のダイビングは深く潜っても行き詰まる): 待ち行列から取り出した
                    // ノードで、経過時間の `ENOMOTO_T_MIP_NOINC_TIME_FRAC` (既定 0.3) 倍までの時間を使い、ノードの定義域と
                    // LP 解から RENS (固定率を調整する版)・Feasibility Pump・ダイビング (1 回 `ENOMOTO_T_MIP_NOINC_DIVE_ITERS`
                    // 反復まで、根と同じく後戻りあり) を順に回す (RENS・pump は根だけで使っていたもの。深いノードでは固定が
                    // 進み部分問題が小さい)。
                    let noinc = heur_ok && self.incumbent.is_none() && node.depth > 0 && plunge_depth == 0 && env_str!("ENOMOTO_MIP_NOINC_HEUR").is_some();
                    // `ENOMOTO_MIP_NOINC_BUDGET`: 時間の割合を木の様子で変える。実行不能で刈れた葉の割合が低い
                    // (暫定解がないと限定操作で刈れず、木が進まない) ほど上限 (既定 0.6) に、高いほど下限 (既定 0.15) に寄せる。
                    // さらに近さ ([`Solver::heur_close`]) の最大値が更新されない回が続くと、呼ぶ間隔を倍々にする (最大 64 ノード)
                    // 40 問 (90 秒、threads=1) では NOINC_HEUR だけ・BANDIT・BUDGET・両方とも 20 問で、sgeomean は基準 2 回
                    // (34.10・33.59) の揺れの範囲 (33.94・33.54・33.72・33.74)。暫定解がないまま終わるのは 30n20b8・neos-1456979
                    // の 2 問だけで、どの設定でも見つからない: 根に 27-50 秒かかり、木の中で呼べるのは 3-9 回なので、8 本の腕を
                    // 一通り試すところで時間が尽きる (neos-1456979 の近さは擬似費用ダイビング 0.79・分数 0.65、FP 0.01)。既定では使わない
                    // 300 秒 (根の近さの引き継ぎ ROOT_PRIOR を含む): sgeomean は基準 66.91、NOINC_HEUR 67.55、BANDIT+BUDGET 67.41、
                    // +ROOT_PRIOR 66.06、どれも 21 問。30n20b8 はどの設定でも解なし。neos-1456979 の最初の解は基準 118 秒
                    // (木の探索そのものがノード 4731 で見つける)、NOINC_HEUR 119 秒、BANDIT+BUDGET 解なし、+ROOT_PRIOR 211 秒:
                    // 木の中のヒューリスティクスは 22 回呼んでも見つけず (近さは 0.7-0.89 で止まる)、時間の 6 割を取って木が
                    // 同じノードに着くのを遅らせただけだった (実行不能の葉が少ないので BUDGET が上限 0.6 にする)
                    let adapt_budget = env_str!("ENOMOTO_MIP_NOINC_BUDGET").is_some();
                    let time_frac = if adapt_budget {
                        let infeas = self.pc.infeasible_leaves as f64 / self.nodes.max(1) as f64;
                        let (lo, hi) = (tunable!("ENOMOTO_T_MIP_NOINC_FRAC_LO", 0.15, f64), tunable!("ENOMOTO_T_MIP_NOINC_FRAC_HI", 0.6, f64));
                        lo + (hi - lo) * (1.0 - infeas.clamp(0.0, 1.0))
                    } else {
                        tunable!("ENOMOTO_T_MIP_NOINC_TIME_FRAC", 0.3, f64)
                    };
                    let spaced = !adapt_budget || self.nodes >= self.noinc_next_node;
                    // 「木を進める」腕 (`ENOMOTO_MIP_NOINC_TREE_ARM`、バンディットのとき): 選ばれたら、腕の 1 回の平均時間だけ
                    // ヒューリスティクスを呼ばずに木を進める。その間のノードの LP 解の分数の列の数の最小を根の LP 解の数と
                    // 比べたものを近さ (報酬) にする (ダイビングの近さと同じ物差し)。木の方が解に近づく問題 (neos-1456979 は
                    // 木の探索そのものが最初の解を見つける) では、ヒューリスティクスより木に時間が回る
                    // 300 秒・40 問 (1 回ずつ): sgeomean は基準 65.42、BANDIT+BUDGET+ROOT_PRIOR 65.58、+TREE_ARM 66.29、
                    // TREE_ARM で BUDGET なし 66.47、どれも 23 問。neos-1456979 の最初の解は基準 134 秒、ROOT_PRIOR 221 秒、
                    // +TREE_ARM 136 秒 (基準並みに戻る)、BUDGET なし 250 秒。30n20b8 はどれも解なし。既定では使わない
                    let mut tree_busy = false;
                    if let Some((t_start, t_end, fmin)) = self.noinc_tree_win {
                        let fmin = fmin.min(self.fractional(&x).len());
                        let now = self.start.elapsed().as_secs_f64();
                        if self.incumbent.is_some() || now >= t_end {
                            let found = self.incumbent.is_some();
                            let close = if found { 1.0 } else if self.noinc_root_frac > 0 { (1.0 - fmin as f64 / self.noinc_root_frac as f64).max(0.0) } else { 0.0 };
                            self.noinc_update_arm(NOINC_TREE_ARM, if found { 2.0 } else { close }, now - t_start);
                            self.noinc_tree_win = None;
                            if self.params.verbose && env_str!("ENOMOTO_MIP_DEBUG_NOINC").is_some() {
                                eprintln!("MIP:   no-incumbent heuristic arm {NOINC_TREE_ARM} (tree): close {close:.3} found {found} ({:.2}s, node {})", now - t_start, self.nodes);
                            }
                        } else {
                            self.noinc_tree_win = Some((t_start, t_end, fmin));
                            tree_busy = true;
                        }
                    }
                    if noinc && spaced && !tree_busy && self.noinc_secs < time_frac * self.start.elapsed().as_secs_f64() {
                        let t0 = Instant::now();
                        let it0 = self.lp.total_iterations();
                        // 腕を選ぶ。`ENOMOTO_MIP_NOINC_BANDIT` なら UCB (報酬は解が見つかれば 2、見つからなければ近さ、
                        // それを使った時間で割り引く)、そうでなければ RENS → Feasibility Pump → ダイビング (種類を順に) を回す
                        let bandit = env_str!("ENOMOTO_MIP_NOINC_BANDIT").is_some();
                        let arm = if bandit {
                            self.noinc_select_arm()
                        } else {
                            match self.noinc_calls % 3 {
                                0 => 0,
                                1 => 1,
                                _ => 2 + (self.noinc_calls / 3) as usize % 6,
                            }
                        };
                        let inc0 = self.incumbent.as_ref().map(|(z, _)| *z);
                        self.heur_close = 0.0;
                        if arm == NOINC_TREE_ARM {
                            let calls: u32 = self.noinc_count.iter().enumerate().filter(|&(a, _)| a != NOINC_TREE_ARM).map(|(_, &c)| c).sum();
                            let cost: f64 = (0..NOINC_ARMS).filter(|&a| a != NOINC_TREE_ARM).map(|a| self.noinc_cost[a]).sum();
                            let len = if calls > 0 { cost / calls as f64 } else { 5.0 };
                            let now = self.start.elapsed().as_secs_f64();
                            self.noinc_tree_win = Some((now, now + len.max(0.5), usize::MAX));
                        }
                        match arm {
                            NOINC_TREE_ARM => {}
                            0 => {
                                self.rens_dive();
                            }
                            1 => {
                                let r = 4 * self.avg_node_iters();
                                self.feasibility_pump(r);
                            }
                            _ => {
                                use super::heuristics::DiveKind as K;
                                let kinds = [K::Fractional, K::VectorLength, K::Coefficient, K::Pseudocost, K::Farkas, K::Conflict];
                                let kind = kinds[arm - 2];
                                let it = self.lp.total_iterations();
                                self.dive_calls += 1;
                                if self.dive(kind, tunable!("ENOMOTO_T_MIP_NOINC_DIVE_ITERS", 50_000u64, u64), f64::INFINITY) {
                                    self.dive_succ += 1;
                                }
                                self.dive_iters += self.lp.total_iterations() - it;
                            }
                        }
                        let found = self.incumbent.as_ref().map(|(z, _)| *z) != inc0;
                        let close = if found { 1.0 } else { self.heur_close.clamp(0.0, 1.0) };
                        let secs = t0.elapsed().as_secs_f64();
                        self.noinc_calls += 1;
                        self.heur_iters += self.lp.total_iterations() - it0;
                        self.noinc_secs += secs;
                        if bandit && arm != NOINC_TREE_ARM {
                            self.noinc_update_arm(arm, if found { 2.0 } else { close }, secs);
                        }
                        // 「木を進める」腕は窓を始めただけ (結果は窓を閉じるときに記録する)
                        if adapt_budget && arm != NOINC_TREE_ARM {
                            if close > self.noinc_best_close + 1e-3 {
                                self.noinc_best_close = close;
                                self.noinc_stall = 0;
                            } else {
                                self.noinc_stall = (self.noinc_stall + 1).min(6);
                            }
                            self.noinc_next_node = self.nodes + (1u64 << self.noinc_stall);
                        }
                        if self.params.verbose && env_str!("ENOMOTO_MIP_DEBUG_NOINC").is_some() && arm != NOINC_TREE_ARM {
                            eprintln!("MIP:   no-incumbent heuristic arm {arm}: close {close:.3} found {found} ({secs:.2}s, node {}, frac {time_frac:.2}, stall {})", self.nodes, self.noinc_stall);
                        }
                    }
                    xp("m_heur");
                    // ダイビング (SCIP の fracdiving / veclendiving: 深さ 10 ごと、ずらし 3 / 7)
                    if heur_ok && !noinc && (node.depth % 10 == 3 || node.depth % 10 == 7) {
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
                            let quot = if self.has_cutoff_value() { 0.8 } else { 0.1 };
                            let bound = if cutoff.is_finite() { lb + quot * (cutoff - lb) } else { f64::INFINITY };
                            let it0 = self.lp.total_iterations();
                            self.dive_calls += 1;
                            self.heur_label = "tree diving";
                            if self.dive(kind, quota - self.dive_iters, bound) {
                                self.dive_succ += 1;
                            }
                            self.heur_label = "";
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
                // HiGHS 流のカット管理 (`highs_cutmgmt`): 待ち行列から取り出したノードで、プールと新しい分離を合わせた
                // ラウンドを、根の LP からの伸びが前のラウンドまでの伸びの 1.01 倍を超えている間 (HiGHS の
                // `HighsSeparation::separate`) 繰り返す。最初のラウンドはプールのカットだけ (予算に数えない。既定のプールの
                // ラウンドに相当し、年齢で外れた根のカットを戻す)、2 ラウンド目からは新しい分離も合わせ、ノードの分離に使った
                // 時間が経過時間の `ENOMOTO_T_MIP_HCM_NODE_SEP_FRAC` 倍に収まる間だけ行う
                let hcm_sep = self.highs_cutmgmt() && node.depth > 0 && plunge_depth == 0;
                if hcm_sep {
                    let feastol = 1e-6 * self.hcm_root_obj.abs().max(1.0);
                    let fresh = hcm_sep_rounds > 0;
                    let budget_ok = !fresh || self.node_cut_secs < tunable!("ENOMOTO_T_MIP_HCM_NODE_SEP_FRAC", 0.1, f64) * self.start.elapsed().as_secs_f64();
                    let go = budget_ok
                        && hcm_sep_rounds < tunable!("ENOMOTO_T_MIP_HCM_NODE_SEP_ROUNDS", 10usize, usize)
                        && (hcm_sep_rounds == 0 || node_obj - self.hcm_root_obj > 1.01 * (hcm_sep_last_obj - self.hcm_root_obj).max(feastol));
                    if go {
                        let t_nc = Instant::now();
                        let rows0 = self.lp.num_rows();
                        let added = self.hcm_node_sep_round(&x, fresh);
                        if fresh {
                            self.node_cut_secs += t_nc.elapsed().as_secs_f64();
                        }
                        if env_str!("ENOMOTO_MIP_DEBUG_HCM").is_some() {
                            eprintln!("HCM node {} depth {} round {} obj {:.6} rows {} -> {} added {} pool {}", self.nodes, node.depth, hcm_sep_rounds, node_obj, rows0, self.lp.num_rows(), added, self.cut_pool.len());
                        }
                        if added {
                            hcm_sep_rounds += 1;
                            hcm_sep_last_obj = node_obj;
                            resolves += 1;
                            if fresh {
                                node_cut_resolve = Some(Instant::now());
                            }
                            continue;
                        }
                        // プールだけのラウンドで何も加わらなければ、新しい分離のラウンドへ進む
                        if !fresh {
                            hcm_sep_rounds = 1;
                            hcm_sep_last_obj = node_obj;
                            if self.node_cut_secs < tunable!("ENOMOTO_T_MIP_HCM_NODE_SEP_FRAC", 0.1, f64) * self.start.elapsed().as_secs_f64() {
                                let t_nc = Instant::now();
                                let added = self.hcm_node_sep_round(&x, true);
                                self.node_cut_secs += t_nc.elapsed().as_secs_f64();
                                if added {
                                    hcm_sep_rounds = 2;
                                    resolves += 1;
                                    node_cut_resolve = Some(Instant::now());
                                    continue;
                                }
                            }
                        }
                    }
                // `ENOMOTO_T_MIP_POOL_CUTS_PER_NODE`: 待ち行列から取り出したノードで 1 ラウンドにプールから戻すカットの本数。
                // 10 本ずつだと、年齢で外れた (または他の部分木で入った) 根のカットが戻りきらず、ノードの LP が根より弱くなる。
                // 100 本 (HiGHS はプールの違反カットを平行度だけで絞って全部戻す) にすると mik-250 35 -> 25 秒、nw04 34 -> 19 秒、
                // binkar10_1 31 -> 28 秒、misc07 14 -> 12 秒、neos-860300 60 -> 50 秒 (単独)。1000 本は 100 本と同等かやや遅い。
                // 40 問 (同時測定) 1 回目: 既定 10 本 18 問 27.75 に対し 100 本 18 問 26.86、2 回目: 19 問 27.19 に対し 18 問 27.63
                // (neos-860300 が 58.7 秒 -> 時間切れ、nw04 17.7 -> 36.1 秒 (経路の揺れ)、misc07 11.3 -> 14.5 秒。mik-250・binkar10_1 は
                // 2 回とも速い)。2 回の合計で解けた数が 37 -> 36 と減るので既定は 10 本のまま
                } else if resolves < 2 && node.depth > 0 && plunge_depth == 0 && env_str!("ENOMOTO_MIP_NO_POOL_CUTS").is_none() && {
                    let r0 = self.lp.num_rows();
                    let added = self.pool_cut_round(&x, tunable!("ENOMOTO_T_MIP_POOL_CUTS_PER_NODE", 10usize, usize));
                    if added && tree_log() && !self.params.submip {
                        eprintln!("EPOOL d={} rows {} -> {} lp={node_obj}", node.depth, r0, self.lp.num_rows());
                    }
                    added
                } {
                    resolves += 1;
                    continue;
                }
                // `ENOMOTO_MIP_OBJ_BOUND_CUTS`: ポートフォリオで厳密な下界 P (二分探索のスレッドが示した「P 未満の解はない」)
                // が上がり、このノードの LP の値より上なら、1 回だけ「目的値 >= P」の行 (そのままの形と、被約費用と行の双対値で
                // 書き直した形) を分離の材料に加えて分離する。目的関数の行そのものは LP に入れない (強く退化する)
                // 40 問 (木 1 本 + 二分探索 2 本、二分探索は根だけヒューリスティクス): 使わない 20 問 27.04、使う 20 問 27.37
                // (加えたのは 15 問で 1-3 回ずつ。neos-911970 で最適値を見つけたが解き切れず、misc07 47.8 -> 55.1 秒)。既定では使わない
                if resolves == 0 && self.watch.is_none() && env_str!("ENOMOTO_MIP_OBJ_BOUND_CUTS").is_some() {
                    let pv = self.shared.as_ref().map_or(f64::NEG_INFINITY, |s| s.proven());
                    let tol = 1e-6 * (1.0 + pv.abs());
                    if pv.is_finite() && pv > self.obj_cut_p + tol && pv > node_obj + tol {
                        self.obj_cut_p = pv;
                        let n = self.p.n;
                        let rhs = -(pv - self.p.offset);
                        // c x >= P - offset  ->  -c x <= -(P - offset)
                        let r1: Vec<(usize, f64)> = (0..n).filter(|&j| self.p.cost[j] != 0.0).map(|j| (j, -self.p.cost[j])).collect();
                        // c x = d x + y^T (A x) (d: 被約費用、y: 行の双対値、A x: 行の活動量 = 分離の変数 n + i)
                        // d は y から計算し直す (LP が返す被約費用は許容誤差の分だけ c - A^T y とずれうる。ずれると
                        // 書き直した行が c x >= P と同値でなくなり、妥当でないカットになりうる)
                        let y = self.lp.row_duals();
                        let mut d = self.p.cost.clone();
                        for (i, &yi) in y.iter().enumerate() {
                            if yi != 0.0 {
                                for (j, a) in self.lp.row(i) {
                                    d[j] -= yi * a;
                                }
                            }
                        }
                        let mut r2: Vec<(usize, f64)> = (0..n).filter(|&j| d[j].abs() > 1e-12).map(|j| (j, -d[j])).collect();
                        r2.extend((0..y.len()).filter(|&i| y[i].abs() > 1e-12).map(|i| (n + i, -y[i])));
                        self.extra_sep_rows = vec![(r1, rhs), (r2, rhs)];
                        let t_nc = Instant::now();
                        let added = self.node_cut_round(&x);
                        self.extra_sep_rows.clear();
                        self.node_cut_secs += t_nc.elapsed().as_secs_f64();
                        self.obj_cut_stats[0] += 1;
                        if added {
                            self.obj_cut_stats[1] += 1;
                            resolves += 1;
                            node_cut_resolve = Some(Instant::now());
                            continue;
                        }
                    }
                }
                // `ENOMOTO_MIP_FC_NODE_SEP2` (固定費用ネットワーク型): 待ち行列から取り出したノードでは、プールのカットを戻した
                // 解き直しの後でも分離する (上のプールのラウンドが `resolves` を進めるので、`resolves == 0` の条件では
                // プールに違反カットがあるノードで分離が飛ばされていた)。LP の値が前のラウンドまでの伸びの 1.01 倍を超えて
                // 伸びる間、`ENOMOTO_T_MIP_FC_SEP_ROUNDS` ラウンドまで繰り返す (HiGHS の `HighsSeparation::separate`)。
                // ノードの分離の時間 (解き直しを含む) が経過時間の `ENOMOTO_T_MIP_FC_SEP2_TIME_FRAC` 倍 (既定 0.1) を超えている間は
                // 行わない (0.3 では ran14x18-disj-8 の 60 秒の下界が 3720.8 -> 3664.5、rout 7.6 -> 21.0 秒。0.1 で 3704.6、9.0 秒)
                let fc2_due = self.fc_node_sep && self.fc_sep2 && !hcm_sep && node.depth > 0 && plunge_depth == 0 && {
                    let feastol = 1e-6 * node_obj.abs().max(1.0);
                    fc2_rounds < tunable!("ENOMOTO_T_MIP_FC_SEP_ROUNDS", 5usize, usize)
                        && (fc2_rounds == 0 || node_obj - fc2_first > 1.01 * (fc2_last - fc2_first).max(feastol))
                        && self.node_cut_secs < tunable!("ENOMOTO_T_MIP_FC_SEP2_TIME_FRAC", 0.1, f64) * self.start.elapsed().as_secs_f64()
                };
                // `ENOMOTO_MIP_TREE_SEP`: 取り出したノードでは、プールのラウンドの後でも分離し ([`Solver::tree_sep_start`] で始めるか
                // を決める)、LP の値が前のラウンドまでの伸びの 1.01 倍を超えて伸び、時間の予算に収まる間
                // `ENOMOTO_T_MIP_TREE_SEP_ROUNDS` (既定 5) ラウンドまで繰り返す。分離しても加わらなければそのノードでは打ち止め
                let ts_due = self.tree_sep && !hcm_sep && node.depth > 0 && plunge_depth == 0 && ts_rounds < tunable!("ENOMOTO_T_MIP_TREE_SEP_ROUNDS", 5usize, usize) && {
                    let feastol = 1e-6 * node_obj.abs().max(1.0);
                    if ts_rounds == 0 {
                        self.tree_sep_start(node.depth)
                    } else {
                        node_obj - ts_first > 1.01 * (ts_last - ts_first).max(feastol) && self.tree_sep_budget_ok() && !self.nca_dense()
                    }
                };
                if ts_due {
                    if ts_rounds == 0 {
                        ts_first = node_obj;
                    }
                    ts_last = node_obj;
                    let t_nc = Instant::now();
                    let r0 = self.lp.num_rows();
                    let added = self.node_cut_round(&x);
                    if added {
                        self.mark_tree_sep_cuts(r0);
                    }
                    self.node_cut_secs += t_nc.elapsed().as_secs_f64();
                    self.node_sep_stats[0] += 1;
                    self.node_sep_stats[1] += added as u64;
                    ts_rounds = if added { ts_rounds + 1 } else { usize::MAX };
                    if added {
                        self.nca_pending = Some(node_obj);
                        resolves += 1;
                        node_cut_resolve = Some(Instant::now());
                        continue;
                    }
                }
                let nca = !self.tree_sep && env_str!("ENOMOTO_MIP_NODE_CUT_ADAPT").is_some() && !self.params.submip && !self.nca_dense();
                if nca && !hcm_sep && node.depth > 0 && plunge_depth == 0 && (nca_rounds == 0 || (nca_last_ok && self.nca_lp_budget_ok())) && nca_rounds < tunable!("ENOMOTO_T_MIP_NODE_CUT_ADAPT_ROUNDS", 3usize, usize) && (self.node_cuts_due(node.depth) || self.node_cut_adapt_due()) {
                    nca_rounds += 1;
                    nca_last_ok = false;
                    let t_nc = Instant::now();
                    let added = self.node_cut_round(&x);
                    self.node_cut_secs += t_nc.elapsed().as_secs_f64();
                    self.node_sep_stats[0] += 1;
                    self.node_sep_stats[1] += added as u64;
                    if added {
                        self.nca_pending = Some(node_obj);
                        resolves += 1;
                        node_cut_resolve = Some(Instant::now());
                        continue;
                    }
                } else if fc2_due || (!(self.fc_node_sep && self.fc_sep2) && !self.tree_sep && !nca && !hcm_sep && resolves == 0 && node.depth > 0 && plunge_depth == 0 && self.node_cuts_due(node.depth)) {
                    let t_nc = Instant::now();
                    let added = self.node_cut_round(&x);
                    self.node_cut_secs += t_nc.elapsed().as_secs_f64();
                    self.node_sep_stats[0] += 1;
                    self.node_sep_stats[1] += added as u64;
                    if fc2_due {
                        if fc2_rounds == 0 {
                            fc2_first = node_obj;
                        }
                        fc2_rounds = if added { fc2_rounds + 1 } else { usize::MAX };
                        fc2_last = node_obj;
                    }
                    if added {
                        fc_obj0 = Some(node_obj);
                        resolves += 1;
                        // 解き直しの時間もノードの分離の時間に数える
                        node_cut_resolve = Some(Instant::now());
                        continue;
                    }
                }
                xp("m_pool");
                let before_sb = dn && self.dbg_contains();
                let rp_force = rp_entry.filter(|e| e.col >= 0 && (e.col as usize) < self.p.n).map(|e| {
                    let c = e.col as usize;
                    let v = frac.iter().find(|&&(j, _)| j == c).map_or(self.dom.lo[c].floor() + 0.5, |&(_, v)| v);
                    (c, v)
                });
                let sel = if let Some((col, value)) = rp_force {
                    rp_stats[2] += 1;
                    if self.dom.lo[col] >= self.dom.up[col] {
                        rp_stats[4] += 1;
                    }
                    BranchAction::Branch { col, value }
                } else {
                    if replay.is_some() {
                        rp_stats[3] += 1;
                    }
                    self.select_branch(&frac, node_obj, node.depth)
                };
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
                            if tree_log() && !self.params.submip {
                                eprintln!("ENODE d={} r=sbprop lp={node_obj} rows={}", node.depth, self.lp.num_rows());
                            }
                            self.add_conflict();
                            break None;
                        }
                        self.sync_lp();
                        continue;
                    }
                    BranchAction::Prune => {
                        if tree_log() && !self.params.submip {
                            eprintln!("ENODE d={} r=sb lp={node_obj} rows={}", node.depth, self.lp.num_rows());
                        }
                        break None;
                    }
                    BranchAction::Branch { col, value } => {
                        let (col, value) = if rp_force.is_some() { (col, value) } else { self.orbitope_branching_column(col, value) };
                        if tree_log() && !self.params.submip {
                            eprintln!("EBR d={} col={col} val={value} lp={node_obj} rows={} resolves={resolves}", node.depth, self.lp.num_rows());
                        }
                        break Some(Some((col, value, node_obj, frac)));
                    }
                }
            };
            xp("m_endlp");
            let Some(branch) = action else {
                if replay.is_some() {
                    self.rp_log(&node, "lp", rp_obj0, rp_obj, -1, &mut rp_stats);
                }
                continue;
            };
            if replay.is_some() {
                self.rp_log(&node, "br", rp_obj0, rp_obj, branch.as_ref().map_or(-1, |b| b.0 as i64), &mut rp_stats);
            }
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
            // `ENOMOTO_MIP_CHILD_HIGHS`: HiGHS の子の選び方 (`ChildSelectionRule::kHybridInferenceCost`): (1 + 推論の平均) /
            // 擬似費用の大きい側。`=up` なら常に上の子
            let prefer_up = match env_str!("ENOMOTO_MIP_CHILD_HIGHS") {
                Some("up") => true,
                Some(_) => {
                    let eps = 1e-6 * (1.0 + node_obj.abs());
                    let us = (1.0 + self.pc.inference(col, true)) / (self.pc.cost_up(col) * (1.0 - f) + eps);
                    let ds = (1.0 + self.pc.inference(col, false)) / (self.pc.cost_down(col) * f + eps);
                    us >= ds
                }
                None => prefer_up,
            };
            let prefer_up = prefer_up ^ (self.race_div & 1 == 1 && !self.race_done);
            // なぞる: 子の優先度は HiGHS の処理順 (ファイルにない子は親の直後)
            let rp_parent = rp_entry.map_or(node.estimate, |e| e.seq as f64);
            let rp_prio = |c: BoundChange| -> f64 {
                let mut ch = node.changes.clone();
                ch.push(c);
                super::replay::lookup(&ch).map_or(rp_parent + 1e-4, |e| e.seq as f64)
            };
            let prefer_up = if replay.is_some() { rp_prio(up) < rp_prio(down) } else { prefer_up };
            let (first_c, second_c) = if prefer_up { (up, down) } else { (down, up) };
            let basis = Rc::new(self.lp.basis());
            let basis_epoch = self.row_log.len();
            // `ENOMOTO_MIP_NODE_DSE`: 基底と一緒に DSE 重みも保存し、取り出したノードの最初の LP で使う
            let dse = if env_str!("ENOMOTO_MIP_NODE_DSE").is_some() || dse_all() { self.lp.dse_snapshot().map(Rc::new) } else { None };
            let prot = self.basis_protected_cuts();
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
                    dse: dse.clone(),
                    branch: Some((col, !c.upper, value, lp_obj)),
                    prot: prot.clone(),
                    tags: node.tags.clone(),
                }
            };
            let mut sec = mk(second_c);
            if replay.is_some() {
                sec.estimate = rp_prio(second_c);
            }
            let sib = self.queue.push(sec);
            // 兄弟へ戻ったときのために親の LP の LU を一緒に持つ (共有なので写さない)
            let sib_lu = if env_str!("ENOMOTO_MIP_NO_SIBLING_LU").is_none() { self.lp.lu_snapshot() } else { None };
            self.dive_stack.push((sib.0, sib.1, sib_lu));
            let mut child = mk(first_c);
            if replay.is_some() {
                child.estimate = rp_prio(first_c);
            }
            plunge_depth += 1;
            // 潜りの打ち切り (SCIP の maxplungequot): 子の下界が 全体の下界 + q (打ち切り値 - 全体の下界) を超えたら
            // 潜らずに待ち行列から選び直す
            let plunge_ok = if replay.is_some() {
                child.estimate <= self.queue.first_prio().map_or(f64::INFINITY, |f| f.2)
            } else {
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
        if replay.is_some() {
            eprintln!("MIP: branch replay: nodes in the file {}, not in the file {}; branched on the file's column {} (already fixed {}), own choice {}", rp_stats[0], rp_stats[1], rp_stats[2], rp_stats[4], rp_stats[3]);
        }
        if self.snap.is_some() {
            self.snap_report(None);
        }
        // 木を分け合う主: 従を待ち、従が見つけた解を受け取ってから結果を決める
        let wlb = self.join_workers();
        self.take_inbox();
        // 従を止めた後に、止められた従が返したノード・共有の待ち行列に残ったノード (下界 `wlb`) が打ち切り値より小さければ、
        // 探索し残した部分木がある (最適とは言えない)。また共有の最良値が主の暫定解より良ければ、その解が主に届いていない
        if self.pool.is_some() && self.snap.is_none() {
            let lim = self.prune_limit();
            let z = self.incumbent.as_ref().map_or(f64::INFINITY, |(z, _)| *z).min(self.orig_best.as_ref().map_or(f64::INFINITY, |(z, _)| *z));
            let sb = self.shared.as_ref().map_or(f64::INFINITY, |s| s.best());
            if tree_diag() {
                eprintln!("MIP_TREE_DIAG: end of tree search: incumbent {z:.10e} shared best {sb:.10e} prune limit {lim:.10e} workers/pool bound {wlb:.10e} unresolved {}", self.unresolved);
            }
            // 共有の最良値を公表した解が主に届いていない (受信箱で拒まれた、など)。木はその値から作った打ち切り値で刈っているので、
            // 主の暫定解を最適とは言えない (以前は警告だけ出して最適と報告していた。analysis/h80x6320d_warning_20261021.md)
            if sb < z - 1e-9 * z.abs().max(1.0) {
                eprintln!("MIP: WARNING: the shared best value {sb:.10e} is better than the main thread's incumbent {z:.10e} at the end of the tree search; not reporting optimality");
                if !self.unresolved && env_str!("ENOMOTO_MIP_TREE_NO_END_CHECK").is_none() {
                    return self.finish(MipStatus::NotSolved, wlb.min(lim));
                }
            }
            if self.dbg_on {
                let dz = DEBUG_SOL.with(|d| d.borrow().as_ref().map(|x| self.p.objective(x)));
                if let Some(dz) = dz.filter(|&dz| dz < lim - 1e-6) {
                    eprintln!("MIP_DEBUG_SOL: the tree search ended with prune limit {lim} above the debug solution's objective {dz} (incumbent {z})");
                }
            }
            // `ENOMOTO_MIP_TREE_NO_END_CHECK` (診断用): 以前と同じく確かめずに最適と報告する (誤りの再現用)
            if wlb < lim && !self.unresolved && env_str!("ENOMOTO_MIP_TREE_NO_END_CHECK").is_none() {
                eprintln!("MIP: WARNING: nodes with lower bound {wlb:.10e} below the prune limit {lim:.10e} remained when the tree search threads were stopped; not reporting optimality");
                return self.finish(MipStatus::NotSolved, wlb.min(z));
            }
        }
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
        // 診断用: 根の被約費用を取る LP の状態が `root_obj` の LP と同じか (ヒューリスティクスが費用・境界を変えたまま戻していないと、
        // 被約費用が根の LP のものでなくなり、打ち切り値による大域的な固定が誤る)
        let lp_obj = self.lp.objective() + self.p.offset;
        if (lp_obj - root_obj).abs() > 1e-6 * (1.0 + root_obj.abs()) {
            eprintln!("MIP: WARNING: root reduced costs are taken from an LP whose objective {lp_obj:.10e} differs from the root objective {root_obj:.10e} (rows {}, thread {})", self.lp.num_rows(), self.pool_tid);
        }
        if self.dbg_on && self.dbg_contains() {
            self.dbg_local_redcost_check(root_obj);
        }
        let b = self.lp.basis_ref();
        let d = self.lp.reduced_costs_ref();
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

    /// 診断用 (デバッグ解): 強分枝の子 (列 `j` を `v` の上/下に分けた側) が打ち切られた (`st`: 実行不能・打ち切り値以上) とき、
    /// 今のノードがデバッグ解を含み、デバッグ解がその側にあれば、打ち切りは誤り。子の LP を打ち切り値なしで解き直して
    /// 真の値を表示する (LP の状態は戻す)。
    fn dbg_sb_cut_check(&mut self, j: usize, is_up: bool, v: f64, st: LpStatus, obj: f64) {
        if !self.dbg_on || !self.dbg_contains() {
            return;
        }
        let Some(x) = DEBUG_SOL.with(|d| d.borrow().clone()) else { return };
        let p = self.p;
        let dz = p.objective(&x);
        if dz >= self.prune_limit() - 1e-6 {
            return;
        }
        let on_side = if is_up { x[j] >= v.ceil() - 1e-6 } else { x[j] <= v.floor() + 1e-6 };
        if !on_side {
            return;
        }
        let saved = self.lp.save_state();
        let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
        for c in 0..p.n {
            self.lp.set_col_bounds(c, self.dom.lo[c], self.dom.up[c]);
        }
        if is_up {
            self.lp.set_col_bounds(j, v.ceil(), up);
        } else {
            self.lp.set_col_bounds(j, lo, v.floor());
        }
        let st2 = self.lp.solve(&SolveLimits { iteration_limit: 1_000_000, cutoff: f64::INFINITY, deadline: self.deadline });
        let o2 = self.lp.objective() + p.offset;
        let act = self.lp.row_activities();
        let mut viol = 0usize;
        for i in 0..self.lp.num_rows() {
            let a: f64 = self.lp.row(i).iter().map(|&(c, w)| w * x[c]).sum();
            let (l, u) = self.lp.row_bounds(i);
            if a < l - 1e-6 * (1.0 + l.abs()) || a > u + 1e-6 * (1.0 + u.abs()) {
                viol += 1;
            }
        }
        let _ = act;
        eprintln!(
            "MIP_DEBUG_SOL: node {} thread {}: strong branching cut off the child x{j} {} {} that contains the debug solution (x_dbg {}, debug obj {dz}, prune limit {:.10e}): child LP {st:?} obj {obj:.10e}; re-solved without cutoff: {st2:?} obj {o2:.10e}; LP rows {} of which the debug solution violates {viol}; node dom [{lo}, {up}]",
            self.nodes, self.pool_tid, if is_up { ">=" } else { "<=" }, if is_up { v.ceil() } else { v.floor() }, x[j], self.prune_limit(), self.lp.num_rows()
        );
        self.lp.restore_state(&saved);
    }

    /// 診断用 (デバッグ解): [`Self::local_redcost_fixing`] の根拠 `c x >= z + sum_{非基底 j} d_j (x_j - 境界_j)` をデバッグ解で確かめ、
    /// 破れていれば内訳 (z、c x、非基底の項の和、符号の合わない被約費用、寄与の大きい列) を表示する。
    fn dbg_local_redcost_check(&self, node_obj: f64) {
        DEBUG_SOL.with(|dd| {
            let Some(x) = dd.borrow().clone() else { return };
            let p = self.p;
            let dz = p.objective(&x);
            if dz >= self.prune_limit() - 1e-6 {
                return;
            }
            let b = self.lp.basis_ref();
            let d = self.lp.reduced_costs_ref();
            let mut sum = 0.0f64;
            let mut terms: Vec<(f64, usize, &'static str, f64, f64, f64)> = Vec::new();
            let (mut wrong_sign, mut max_wrong, mut max_basic) = (0usize, 0.0f64, 0.0f64);
            for j in 0..p.n {
                let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
                let (st, bound) = match b.col[j] {
                    VarStatus::Lower => ("Lower", lo),
                    VarStatus::Upper => ("Upper", up),
                    VarStatus::Basic => {
                        max_basic = max_basic.max(d[j].abs());
                        continue;
                    }
                    _ => ("Other", 0.0),
                };
                if (st == "Lower" && d[j] < -1e-7) || (st == "Upper" && d[j] > 1e-7) {
                    wrong_sign += 1;
                    max_wrong = max_wrong.max(d[j].abs());
                }
                if !bound.is_finite() {
                    continue;
                }
                let t = d[j] * (x[j] - bound);
                sum += t;
                if t.abs() > 1e-9 {
                    terms.push((t, j, st, d[j], x[j], bound));
                }
            }
            let lhs = dz;
            let rhs = node_obj + sum;
            if lhs < rhs - 1e-6 * (1.0 + rhs.abs()) {
                terms.sort_by(|a, b| b.0.total_cmp(&a.0));
                eprintln!(
                    "MIP_DEBUG_SOL: node {} thread {}: redcost fixing: the dual bound is violated at the debug solution: c x = {lhs:.10e} < z + sum d_j (x_j - bound_j) = {node_obj:.10e} + {sum:.10e} = {rhs:.10e} (lp obj now {:.10e}, lp rows {}, prune limit {:.10e}; nonbasic with wrong-sign d: {wrong_sign} (max {max_wrong:.3e}), max |d| on basic {max_basic:.3e})",
                    self.nodes, self.pool_tid, self.lp.objective() + p.offset, self.lp.num_rows(), self.prune_limit()
                );
                for &(t, j, st, dj, xj, bound) in terms.iter().take(8) {
                    let (ll, lu) = self.lp.col_bounds(j);
                    eprintln!("MIP_DEBUG_SOL:   col {j} ({st}, int {}): d {dj:.6e}, x_dbg {xj}, bound {bound}, dom [{}, {}], lp [{ll}, {lu}], global [{}, {}], term {t:.6e}", p.is_int[j], self.dom.lo[j], self.dom.up[j], self.dom.global_lo[j], self.dom.global_up[j]);
                }
            }
        });
    }

    /// 現在の LP の被約費用で、このノード以下で有効な境界を締める。締めたら真。
    /// 打ち切り値に使える解の目的値があるか (自分の暫定解、または木を分け合うときはほかのスレッドの解の共有の最良値)。
    /// 目的値しか使わない処理 (被約費用による固定・ノードの選び方) の条件に使う。木を分け合わないときは自分の暫定解だけ
    /// (ポートフォリオ並列の挙動は変えない)。
    fn has_cutoff_value(&self) -> bool {
        // `ENOMOTO_MIP_TREE_OWN_INC_ONLY`: 以前の形 (自分の暫定解があるときだけ)
        self.incumbent.is_some() || (self.pool.is_some() && env_str!("ENOMOTO_MIP_TREE_OWN_INC_ONLY").is_none() && self.shared.as_ref().is_some_and(|s| s.best().is_finite()))
    }

    /// 証明の段階に入ったか ([`Self::proof_phase_check`])。
    fn in_proof_phase(&self) -> bool {
        match &self.pool {
            Some(pool) => pool.proof_phase.load(std::sync::atomic::Ordering::Relaxed),
            None => self.proof_phase,
        }
    }

    /// 証明の段階の検出 (`ENOMOTO_MIP_PROOF_SWITCH`): 最適解かそれに近い解が早く見つかり、残りは最適性の証明だけの問題
    /// (noswot・mas76・binkar10_1 など) で、木の中の主側の処理を止めて下界最小のノード選択に切り替える。0.5 秒おきに
    /// 主 (木を分け合わないときは自分) が調べ、次を全て満たしたら入る (戻らない):
    /// - 今のギャップ (最良値 - 全体の下界) が、最初の暫定解のときのギャップ (最初の暫定解 - 根の下界) の
    ///   `ENOMOTO_T_MIP_PROOF_GAP_FRAC` (既定 0.1) 倍以下か、相対ギャップが `ENOMOTO_T_MIP_PROOF_REL_GAP` (既定 0 = 見ない) 以下
    /// - 相対ギャップが `ENOMOTO_T_MIP_PROOF_REL_GAP_MAX` (既定 0.05) 以下
    /// - 最良値が (ギャップの 2% を超えて) 良くなってからの時間が、経過時間の `ENOMOTO_T_MIP_PROOF_STALL_FRAC` (既定 0.5) 倍以上、かつ
    ///   `ENOMOTO_T_MIP_PROOF_STALL_MIN` (既定 2) 秒以上
    fn proof_phase_check(&mut self) {
        if self.params.submip || self.pool_tid != 0 || self.in_proof_phase() || env_str!("ENOMOTO_MIP_PROOF_SWITCH").is_none() {
            return;
        }
        if self.proof_eval_at.is_some_and(|t| t.elapsed().as_secs_f64() < 0.5) {
            return;
        }
        let now = Instant::now();
        self.proof_eval_at = Some(now);
        let z = match &self.shared {
            Some(s) => s.best(),
            None => self.incumbent.as_ref().map_or(f64::INFINITY, |(z, _)| *z),
        };
        if !z.is_finite() {
            return;
        }
        if !self.proof_first_z.is_finite() {
            self.proof_first_z = z;
        }
        // 最良値の改善は、そのときのギャップの `ENOMOTO_T_MIP_PROOF_IMPROVE` (既定 0.02) 倍を超えるときだけ数える
        // (qiu では 1e-9 程度の改善が続き、停滞の時間がそのたびに 0 に戻っていた)
        let lb_now = self.global_lower_bound();
        let tol = if self.proof_last_best.0.is_finite() && lb_now.is_finite() {
            tunable!("ENOMOTO_T_MIP_PROOF_IMPROVE", 0.02, f64) * (self.proof_last_best.0 - lb_now).max(0.0)
        } else {
            0.0
        };
        if z < self.proof_last_best.0 - tol.max(1e-9 * z.abs().max(1.0)) {
            self.proof_last_best = (z, now);
            return;
        }
        let el = self.start.elapsed().as_secs_f64();
        let stall = now.duration_since(self.proof_last_best.1).as_secs_f64();
        if stall < tunable!("ENOMOTO_T_MIP_PROOF_STALL_FRAC", 0.5, f64) * el || stall < tunable!("ENOMOTO_T_MIP_PROOF_STALL_MIN", 2.0, f64) {
            if env_str!("ENOMOTO_MIP_PROOF_DEBUG").is_some() {
                let lb = self.global_lower_bound();
                eprintln!("PROOFDBG: {el:.1}s stall {stall:.1}s (short) gap {:.3e} gap0 {:.3e}", z - lb, self.proof_first_z - self.hcm_root_obj);
            }
            return;
        }
        let lb = self.global_lower_bound();
        if !lb.is_finite() {
            return;
        }
        let gap = (z - lb).max(0.0);
        let gap0 = (self.proof_first_z - self.hcm_root_obj).max(0.0);
        let rel = gap / z.abs().max(lb.abs()).max(1e-9);
        // 相対ギャップが `ENOMOTO_T_MIP_PROOF_REL_GAP_MAX` (既定 0.05) を超えるときは入らない (最初の暫定解が悪いと
        // ギャップの比はすぐ小さくなる: radiationm18-12-05 は相対ギャップ 10%、rococoC10-001000 は 22% で入っていた)
        let ok = ((gap0 > 0.0 && gap <= tunable!("ENOMOTO_T_MIP_PROOF_GAP_FRAC", 0.1, f64) * gap0) || rel <= tunable!("ENOMOTO_T_MIP_PROOF_REL_GAP", 0.0, f64))
            && rel <= tunable!("ENOMOTO_T_MIP_PROOF_REL_GAP_MAX", 0.05, f64);
        if env_str!("ENOMOTO_MIP_PROOF_DEBUG").is_some() {
            eprintln!("PROOFDBG: {el:.1}s stall {stall:.1}s gap {gap:.3e} gap0 {gap0:.3e} ratio {:.3} rel {rel:.4} ok {ok}", gap / gap0.max(1e-300));
        }
        if !ok {
            return;
        }
        match &self.pool {
            Some(pool) => pool.proof_phase.store(true, std::sync::atomic::Ordering::Relaxed),
            None => self.proof_phase = true,
        }
        if self.params.verbose {
            eprintln!(
                "MIP: proof phase at node {} ({el:.2}s): best {z:.10e} bound {lb:.10e} gap {gap:.3e} (first-incumbent gap {gap0:.3e}), no improvement for {stall:.1}s",
                self.nodes
            );
        }
    }

    /// 木の中の大近傍探索・ダイビングをこのスレッドで回すか。木を分け合うときは HiGHS (`HighsMipSolver::run`) と同じく、
    /// 暫定解があればスレッドの 1/4、なければ 1/2 (切り上げ、主を含む) だけが回す (全スレッドが同じ予算で回すと、同じ
    /// 暫定解から同じ近傍のサブ MIP を重複して解く)。主 (番号 0)・1 スレッドは常に回す。
    /// `ENOMOTO_MIP_TREE_HEUR_ALL` なら以前と同じく全スレッドが回す
    fn tree_heur_allowed(&self) -> bool {
        // 証明の段階では木の中の大近傍探索・ダイビングを止める (単純丸め・ZI round は続ける)
        if self.in_proof_phase() {
            return false;
        }
        let Some(pool) = &self.pool else { return true };
        // ヒューリスティクス役を分けるとき: 解があれば (役のスレッドが大近傍探索を回すので) 木の探索のスレッドは回さない
        // (`ENOMOTO_MIP_HEUR_ROLE_TREE_LNS` なら以前と同じく回す)
        // (起動の判定 `ENOMOTO_MIP_HEUR_ROLE_TRIGGER` を使うときは、役に就いているスレッドがある間だけ止める)
        let role_on = if heur_trigger_mode() { pool.in_role.load(std::sync::atomic::Ordering::Relaxed) > 0 } else { pool.has_x.load(std::sync::atomic::Ordering::Acquire) };
        if pool.heur_k > 0 && role_on && env_str!("ENOMOTO_MIP_HEUR_ROLE_TREE_LNS").is_none() {
            return false;
        }
        if self.pool_tid == 0 || env_str!("ENOMOTO_MIP_TREE_HEUR_ALL").is_some() {
            return true;
        }
        let k = if self.has_cutoff_value() { pool.threads.div_ceil(tunable!("ENOMOTO_T_MIP_TREE_HEUR_DIV_INC", 2usize, usize).max(1)) } else { pool.threads.div_ceil(tunable!("ENOMOTO_T_MIP_TREE_HEUR_DIV_NOINC", 2usize, usize).max(1)) };
        (self.pool_tid as usize) < k
    }

    /// このスレッドがヒューリスティクス役に就くか: 役を分けていて、番号が大きい方から `heur_k` 本の従で、racing ramp-up を
    /// 終え、共有の解があるとき。
    fn heur_role_mine(&mut self) -> bool {
        let Some(pool) = &self.pool else { return false };
        if !(pool.heur_k > 0 && self.snap.is_some() && self.race_done && (self.pool_tid as usize) >= pool.threads - pool.heur_k && pool.has_x.load(std::sync::atomic::Ordering::Acquire)) {
            return false;
        }
        if proof_phase_stops_role() && pool.proof_phase.load(std::sync::atomic::Ordering::Relaxed) {
            return false;
        }
        // 起動の判定 (`ENOMOTO_MIP_HEUR_ROLE_TRIGGER`): 0.25 秒おきに、木の証明が時間内に終わりそうになく、暫定解が良くなれば
        // 木の多くを刈れるときだけ役に就く ([`Self::heur_trigger_eval`])。木の探索に戻った後は決めた秒数が過ぎるまで就かない
        if heur_trigger_mode() {
            if self.trig_eval_at.is_some_and(|t| t.elapsed().as_secs_f64() < 0.25) || self.heur_role_retired.is_some_and(|(t, _, cool)| t.elapsed().as_secs_f64() < cool) {
                return false;
            }
            self.trig_eval_at = Some(Instant::now());
            self.publish_tree_weight(None);
            let Some(ev) = self.heur_trigger_eval() else { return false };
            let go = if heur_mu_mode() {
                ev.warm && self.mu_worth(&ev, false)
            } else {
                ev.warm && ev.est > tunable!("ENOMOTO_T_MIP_HEUR_TRIG_START", 1.5, f64) * ev.left && ev.frac >= tunable!("ENOMOTO_T_MIP_HEUR_TRIG_FRAC", 0.3, f64)
            };
            if env_str!("ENOMOTO_MIP_HEUR_TRIG_DEBUG").is_some() {
                eprintln!("HEURTRIG: tid {} eval: {} warm {} v {:.3e}", self.pool_tid, ev.line(), ev.warm, self.trig_v);
            }
            if go && heur_role_verbose() {
                eprintln!("HEURTRIG: tid {} start: {}", self.pool_tid, ev.line());
            }
            return go;
        }
        // 木の探索に戻った後: `ENOMOTO_MIP_HEUR_ROLE_REENTER` なら、ほかのスレッドが最良値を良くしたか (近傍の中心が変わった)、
        // 戻ってから決めた秒数が過ぎたら役に戻る (64 ノードおきに調べる)
        match self.heur_role_retired {
            None => true,
            Some((t, best, cool)) => {
                env_str!("ENOMOTO_MIP_HEUR_ROLE_REENTER").is_some()
                    && self.nodes % 64 == 0
                    && (self.shared.as_ref().is_some_and(|s| s.best() < best - 1e-9 * best.abs().max(1.0)) || t.elapsed().as_secs_f64() >= cool)
            }
        }
    }

    /// 刈れる重みの分布の境: 「最良値 - δ_i (最良値 - 全体の下界)」(δ_i は [`TRIG_DELTAS`])。最良値か全体の下界がなければ +inf。
    fn trigger_cut_thresholds(&self) -> [f64; 8] {
        let z = self.shared.as_ref().map_or(f64::INFINITY, |s| s.best());
        let lb = self.global_lower_bound();
        if !(z.is_finite() && lb.is_finite()) {
            return [f64::INFINITY; 8];
        }
        TRIG_DELTAS.map(|d| z - d * (z - lb).max(0.0))
    }

    /// 自分の未処理のノード (待ち行列と潜っている子) の重みの和と、刈れる割合の分を [`ThreadSlot`] に書く。
    fn publish_tree_weight(&self, plunge: Option<&OpenNode>) {
        use std::sync::atomic::Ordering;
        let Some(pool) = &self.pool else { return };
        let thr = self.trigger_cut_thresholds();
        let mut w = 0.0f64;
        let mut wc = [0.0f64; 8];
        for n in self.queue.iter().chain(plunge) {
            let wt = 0.5f64.powi(n.depth.min(2000) as i32);
            w += wt;
            for i in 0..8 {
                if n.lower_bound >= thr[i] {
                    wc[i] += wt;
                }
            }
        }
        if let Some(sl) = pool.slots.get(self.pool_tid as usize) {
            sl.wopen.store(w.to_bits(), Ordering::Relaxed);
            for i in 0..8 {
                sl.wcuts[i].store(wc[i].to_bits(), Ordering::Relaxed);
            }
        }
    }

    /// 起動の判定 (案 1: 木の重み、案 2: 暫定解が良くなれば刈れる割合)。未処理のノード (全てのスレッドと共有の待ち行列) の
    /// 重み w の和から木の完了率 1 - w を出し、その進む速さ v (0.5 秒以上空けて測り、指数平滑) から完了までを w / v 秒と
    /// 予測する。残りの秒数は期限まで (期限がなければ、木の探索の経過時間 × `ENOMOTO_T_MIP_HEUR_TRIG_NOLIMIT_MULT`、既定 10)。
    /// 判定は木の探索を始めてから 1 秒以上・全体の時間の `ENOMOTO_T_MIP_HEUR_TRIG_WARM_FRAC` (既定 5%) 以上経ち、
    /// `ENOMOTO_T_MIP_HEUR_TRIG_NODES` (既定 100) ノード以上処理し、速さを 1 回以上測ってからにする。
    fn heur_trigger_eval(&mut self) -> Option<TriggerEval> {
        use std::sync::atomic::Ordering;
        let pool = self.pool.clone()?;
        let thr = self.trigger_cut_thresholds();
        let zb = self.shared.as_ref().map_or(f64::INFINITY, |s| s.best());
        let gap = if thr[7].is_finite() { (zb - thr[7]).max(0.0) } else { f64::INFINITY };
        let mut w = 0.0f64;
        let mut wcuts = [0.0f64; 8];
        {
            let g = pool.inner.lock().unwrap();
            for n in g.nodes.iter() {
                let wt = 0.5f64.powi(n.depth.min(2000) as i32);
                w += wt;
                for i in 0..8 {
                    if n.lower_bound >= thr[i] {
                        wcuts[i] += wt;
                    }
                }
            }
        }
        for sl in &pool.slots {
            w += f64::from_bits(sl.wopen.load(Ordering::Relaxed));
            for i in 0..8 {
                wcuts[i] += f64::from_bits(sl.wcuts[i].load(Ordering::Relaxed));
            }
        }
        let wc = cut_weight(&wcuts, tunable!("ENOMOTO_T_MIP_HEUR_TRIG_DELTA", 0.1, f64));
        let done = 1.0 - w;
        let now = Instant::now();
        match self.trig_last {
            Some((t0, d0)) => {
                let dt = now.duration_since(t0).as_secs_f64();
                if dt >= 0.5 {
                    let rate = ((done - d0) / dt).max(0.0);
                    let a = tunable!("ENOMOTO_T_MIP_HEUR_TRIG_ALPHA", 0.3, f64);
                    self.trig_v = if self.trig_v < 0.0 { rate } else { a * rate + (1.0 - a) * self.trig_v };
                    self.trig_last = Some((now, done));
                }
            }
            None => self.trig_last = Some((now, done)),
        }
        // 完了までの予測は、直近の速さ (平滑) と木の探索を始めてからの平均の速さの、短い方 (完了率は進みが不揃いで、
        // 直近の速さだけでは止まって見えた後に一気に進む問題 (qiu) で起動してしまう)
        let tree_el = pool.t0.elapsed().as_secs_f64();
        let v_avg = if tree_el > 0.0 { done.max(0.0) / tree_el } else { 0.0 };
        let v = self.trig_v.max(v_avg);
        let est = if v > 1e-12 { w.max(0.0) / v } else { f64::INFINITY };
        let left = match self.deadline {
            Some(d) => d.saturating_duration_since(now).as_secs_f64(),
            None => tunable!("ENOMOTO_T_MIP_HEUR_TRIG_NOLIMIT_MULT", 10.0, f64) * tree_el,
        };
        let frac = if w > 0.0 { (wc / w).min(1.0) } else { 0.0 };
        let nodes = self.global_node_counts().0;
        let warm = self.trig_v >= 0.0
            && tree_el >= 1.0
            && tree_el >= tunable!("ENOMOTO_T_MIP_HEUR_TRIG_WARM_FRAC", 0.05, f64) * (tree_el + left)
            && nodes >= tunable!("ENOMOTO_T_MIP_HEUR_TRIG_NODES", 100u64, u64);
        Some(TriggerEval { done, est, left, frac, warm, w, wcuts, gap, v })
    }

    /// 大近傍探索 1 回の結果を価値の指数移動平均 ([`MuStats`]) に入れる。`ev` は呼ぶ前の木の状態、`z0`/`z1` は呼ぶ前と後の
    /// 最良値、`dt` はかかった秒数。証明への寄与 = 暫定解の改善で刈れる木の重み / 秒、主解への寄与 = 縮めたギャップの割合 / 秒。
    fn mu_observe(&self, ev: &TriggerEval, z0: f64, z1: f64, dt: f64, role: bool) {
        let Some(pool) = &self.pool else { return };
        let dt = dt.max(1e-3);
        let rel = if ev.gap.is_finite() && ev.gap > 0.0 && z0.is_finite() { ((z0 - z1) / ev.gap).clamp(0.0, 1.0) } else { 0.0 };
        let proof = ev.gap.is_finite().then(|| cut_weight(&ev.wcuts, rel) / dt);
        {
            let mut mu = pool.mu.lock().unwrap();
            mu.add(proof, rel / dt);
            if role {
                mu.add_role(rel / dt);
            }
        }
        if env_str!("ENOMOTO_MIP_HEUR_TRIG_DEBUG").is_some() {
            eprintln!("HEURMU: tid {} obs rel {rel:.4} dt {dt:.2} proof {:?} -> {:?}", self.pool_tid, proof, *pool.mu.lock().unwrap());
        }
    }

    /// 大近傍探索を専用のスレッドで回す価値があるか (限界効用の比較)。木の証明が時間内に終わりそうにない (案 1 の予測 >
    /// 残り時間) なら主解への寄与を `ENOMOTO_T_MIP_HEUR_MU_PRIMAL` (毎秒のギャップの割合、既定 0.002) と比べ、終わりそうなら
    /// 証明への寄与を木の探索の 1 スレッドあたりの完了率の速さ × `ENOMOTO_T_MIP_HEUR_MU_KAPPA` (既定 1) と比べる。役に
    /// 就いている間 (`in_role`) は、しきい値を `ENOMOTO_T_MIP_HEUR_MU_HYST` 倍 (既定 0.5) に下げる (起動と停止を繰り返さないように)。
    /// 観測がなければ、主解の側は価値ありとし (試して測る)、証明の側は価値なしとする。
    /// どちらの寄与で判定するかは `ENOMOTO_MIP_FOCUS` で決める: 既定 (proof) は常に証明への寄与 (木の完了率が測れるほど
    /// 進まない問題では木の側の値が 0 になり、近傍探索の寄与が 0 でも「0 以上」として就く。bppc4-08・blp-ar98 など。木が
    /// 進んでいて近傍探索で何も刈れない問題 (qiu) では就かない)、primal は常に主解への寄与 (良い暫定解を優先)、auto は
    /// 案 1 の予測で切り替える。
    fn mu_worth(&self, ev: &TriggerEval, in_role: bool) -> bool {
        let Some(pool) = &self.pool else { return false };
        let mu = *pool.mu.lock().unwrap();
        let h = if in_role { tunable!("ENOMOTO_T_MIP_HEUR_MU_HYST", 0.5, f64) } else { 1.0 };
        let focus = env_str!("ENOMOTO_MIP_FOCUS");
        let primal_side = match focus.as_deref() {
            Some("primal") => true,
            Some("auto") => ev.est > ev.left,
            _ => false,
        };
        let thr_p = h * tunable!("ENOMOTO_T_MIP_HEUR_MU_PRIMAL", 0.002, f64);
        if primal_side && focus.as_deref() == Some("primal") {
            // 良い暫定解を優先: 根の実績も事前の値に使い、観測がなければ試す
            mu.primal.is_none_or(|v| v >= thr_p)
        } else if primal_side {
            // 既定 (証明寄り): 木の探索を始めてからの大近傍探索の観測が `ENOMOTO_T_MIP_HEUR_MU_MIN_OBS` (既定 3) 回以上あり、
            // その平均がしきい値以上のときだけ (木のノードが暫定解を良くしていて大近傍探索が効かない問題 (qiu) では就かない)
            mu.n_tree >= tunable!("ENOMOTO_T_MIP_HEUR_MU_MIN_OBS", 3u64, u64) && mu.primal_tree.is_some_and(|v| v >= thr_p)
        } else {
            // 既定 (proof): (a) 大近傍探索の証明への寄与が正で、木の探索の 1 スレッドあたりの完了率の速さ × κ 以上か、
            // (b) 木が停滞している (完了の予測が残り時間の `ENOMOTO_T_MIP_HEUR_MU_STAGNANT` 倍 (既定 4) 以上。役の間は
            // その `ENOMOTO_T_MIP_HEUR_MU_HYST` 倍) ときに、木の中の大近傍探索の観測がまだ少ないか、主解への寄与がしきい値以上なら
            // 就く。以前は (a) を「0 以上」で判定していて、木の完了率が測れるほど進まないときだけ 0 >= 0 で就いていた (少しでも
            // 進むと就かず、cvs16r128-89 で 2 回目は一度も就かなかった)。qiu は予測が残り時間の約 2 倍で (b) に当たらない
            let tree = (pool.threads - pool.in_role.load(std::sync::atomic::Ordering::Relaxed)).max(1) as f64;
            let vt = ev.v / tree;
            let proof_ok = mu.proof.is_some_and(|v| v > 0.0 && v >= h * tunable!("ENOMOTO_T_MIP_HEUR_MU_KAPPA", 1.0, f64) * vt);
            let stagnant = ev.est >= h * tunable!("ENOMOTO_T_MIP_HEUR_MU_STAGNANT", 4.0, f64) * ev.left;
            let primal_ok = mu.n_tree < tunable!("ENOMOTO_T_MIP_HEUR_MU_MIN_OBS", 3u64, u64) || mu.primal_tree.is_some_and(|v| v >= thr_p);
            // `ENOMOTO_MIP_HEUR_MU_ROLE_OBS` (試作): 役のスレッド自身の観測が `ENOMOTO_T_MIP_HEUR_MU_ROLE_MIN_OBS` (既定 10) 回に満たないか、
            // その平均がしきい値以上なら、主解の側も価値ありとする。木のスレッドの大近傍探索は 1 回の時間が短く (最低 0.3 秒)、
            // 暫定解が最初から良いと失敗が続いて木の側の平均が 0 になり、役が 1 回 (0.01 秒) で退いてそれきり就かない
            // (bppc4-08 の mixoc は Local-MIP の 55 から。既定は 62 から役の Proximity Search が 1 ずつ下げて 54。analysis/bppc408_20261023.md)
            let primal_ok = primal_ok
                || (env_def!("ENOMOTO_MIP_HEUR_MU_ROLE_OBS", "1").is_some()
                    && (mu.n_role < tunable!("ENOMOTO_T_MIP_HEUR_MU_ROLE_MIN_OBS", 10u64, u64) || mu.primal_role.is_some_and(|v| v >= thr_p)));
            proof_ok || (stagnant && primal_ok)
        }
    }

    /// 共有の最良の解が自分の暫定解より良ければ取り込む (解の空間はどのスレッドも同じ。共有の最良値は既に下がっている)。
    fn heur_role_import(&mut self, pool: &WorkPool) {
        let got = pool.best_x.lock().unwrap().clone();
        if let Some((z, x)) = got {
            if self.incumbent.as_ref().is_none_or(|(zi, _)| z < *zi - 1e-9 * zi.abs().max(1.0)) {
                self.pool_add(z, &x);
                self.incumbent = Some((z, (*x).clone()));
                self.heur_role_stats[2] += 1;
            }
        }
    }

    /// ヒューリスティクス役 (`ENOMOTO_MIP_HEUR_ROLE`): 自分の待ち行列のノードを全て共有の待ち行列に渡し (待っていない側からも
    /// 外れる)、共有の最良の解と木の探索のスレッドが渡したノードの LP 解から ALNS の大近傍探索を回し続ける。見つけた解は
    /// 通常どおり主に渡す ([`Self::try_incumbent`])。`ENOMOTO_T_MIP_HEUR_ROLE_PATIENCE` = P (既定 20、0 なら戻らない) 回続けて暫定解を良く
    /// できなければ木の探索に戻る (`None`)。木の探索が終わったら `Some(true)`、時間切れ・止める合図なら `Some(false)`。
    /// ヒューリスティクス役の次の 1 回を Local-MIP にするか (`ENOMOTO_MIP_ROLE_LMIP`)。使った時間の割合で決める: どちらも暫定解を
    /// 良くしていないうちは半々、良くした腕 (先導) に `ENOMOTO_T_MIP_ROLE_LMIP_LEAD` (0.75)、もう一方に残り。最初は ALNS
    fn role_pick_lmip(&self) -> bool {
        if !role_lmip_on() || self.role_lm_off {
            return false;
        }
        let lead = tunable!("ENOMOTO_T_MIP_ROLE_LMIP_LEAD", 0.75, f64).clamp(0.0, 1.0);
        let share = match self.role_leader {
            0 => 0.5,
            2 => lead,
            _ => 1.0 - lead,
        };
        let [ta, tl] = self.role_arm_t;
        tl < share * (ta + tl) - 1e-9
    }

    /// Local-MIP を `ENOMOTO_T_MIP_ROLE_LMIP_SLICE` (0.5) 秒だけ続きから回す。暫定解の方が良ければ先に渡して、そこから磨かせる。
    /// 良い解が出れば暫定解にする
    fn role_lmip_slice(&mut self) {
        let p = self.p;
        if self.role_lm.is_none() {
            let seed = self.rng ^ 0x6A09_E667_F3BC_C909 ^ u64::from(self.pool_tid);
            self.role_lm = super::localmip::LocalMip::new(p, &self.dom.global_lo, &self.dom.global_up, seed, true);
            if self.role_lm.is_none() {
                self.role_lm_off = true;
                return;
            }
        }
        if let Some((_, x)) = self.incumbent.clone() {
            self.role_lm.as_mut().unwrap().inject(&x);
        }
        let slice = tunable!("ENOMOTO_T_MIP_ROLE_LMIP_SLICE", 0.5, f64).max(0.01);
        let mut t_end = Instant::now() + Duration::from_secs_f64(slice);
        if let Some(d) = self.deadline {
            t_end = t_end.min(d);
        }
        let lm = self.role_lm.as_mut().unwrap();
        lm.run(t_end, None, &|_| {});
        if lm.done {
            self.role_lm_off = true;
        }
        let cand = lm.res.best.clone();
        if let Some(x) = cand {
            let z = p.objective(&x);
            if self.incumbent.as_ref().is_none_or(|(zi, _)| z < *zi - 1e-9 * zi.abs().max(1.0)) {
                self.heur_label = "Local-MIP (heuristic role)";
                self.try_incumbent(x);
                self.heur_label = "";
            }
        }
    }

    /// `ENOMOTO_MIP_ROOT_LMIP`: 暫定解を Local-MIP に渡し、`ENOMOTO_T_MIP_ROOT_LMIP_SECS` (2) 秒と時間上限の
    /// `ENOMOTO_T_MIP_ROOT_LMIP_FRAC` (0.03) 倍の小さい方だけ回す (lift move と目的の行を締める探索で磨く)。良い解が出れば暫定解にする
    fn root_lmip(&mut self) -> bool {
        let p = self.p;
        let Some((z0, x0)) = self.incumbent.clone() else { return false };
        let seed = self.rng ^ 0x5851_F42D_4C95_7F2D;
        let Some(mut lm) = super::localmip::LocalMip::new(p, &self.dom.global_lo, &self.dom.global_up, seed, true) else { return false };
        if !lm.inject(&x0) {
            return false;
        }
        let mut secs = tunable!("ENOMOTO_T_MIP_ROOT_LMIP_SECS", 2.0, f64);
        if self.params.time_limit.is_finite() {
            secs = secs.min(tunable!("ENOMOTO_T_MIP_ROOT_LMIP_FRAC", 0.03, f64) * self.params.time_limit);
        }
        let mut t_end = Instant::now() + Duration::from_secs_f64(secs.max(0.0));
        if let Some(d) = self.deadline {
            t_end = t_end.min(d);
        }
        lm.run(t_end, None, &|_| {});
        let Some(x) = lm.res.best.clone() else { return false };
        let z = p.objective(&x);
        if z < z0 - 1e-9 * z0.abs().max(1.0) {
            self.try_incumbent(x);
            return self.incumbent.as_ref().is_some_and(|(zi, _)| *zi < z0 - 1e-9 * z0.abs().max(1.0));
        }
        false
    }

    fn heur_role_run(&mut self) -> Option<bool> {
        use std::sync::atomic::Ordering;
        let pool = self.pool.clone()?;
        let t_role = Instant::now();
        self.heur_role_entries += 1;
        self.in_heur_role = true;
        pool.in_role.fetch_add(1, Ordering::Relaxed);
        let mut out = Vec::with_capacity(self.queue.len());
        while let Some(n) = self.queue.pop_best() {
            out.push(self.snap_of(&n));
        }
        self.dive_stack.clear();
        {
            let mut g = pool.inner.lock().unwrap();
            g.nodes.extend(out);
            pool.len.store(g.nodes.len(), Ordering::Relaxed);
            if self.pool_busy {
                g.busy -= 1;
                self.pool_busy = false;
                // 待っているスレッドの数 (`idle`) も合わせる (`ENOMOTO_MIP_NO_ROLE_IDLE_FIX` なら以前と同じく合わせない)。合わせないと、役から木に戻って
                // 共有の待ち行列からノードを取るときに `idle` を 1 減らして usize の下に回り (`pool_busy` が偽のまま取るので
                // 増やした分がない)、以後どのスレッドも「待っているスレッドがいる」と見て毎ノード自分の待ち行列の半分を
                // 共有の待ち行列に渡し続ける (pk1 の速い山はこの状態: 自分の待ち行列がほぼ空になりカットが保護されず LP が
                // 小さいままになる。analysis/pk1_variance_20261017.md)。合わせるだけでは 93 問 (2 回) で主ギャップの和が 12.99 -> 15.26 と
                // 悪くなったが、手元の待ち行列の上限 (`ENOMOTO_T_MIP_TREE_LOCAL_CAP`、既定 1000) と組むと 40 問で既定と同等以上になったので、
                // 役の有無で探索が変わらないこの組を既定にした
                if env_str!("ENOMOTO_MIP_NO_ROLE_IDLE_FIX").is_none() {
                    pool.idle.fetch_add(1, Ordering::Relaxed);
                }
            }
            if g.busy == 0 && g.nodes.is_empty() {
                g.done = true;
                pool.fin.store(true, Ordering::Release);
            }
        }
        if let Some(sl) = pool.slots.get(self.pool_tid as usize) {
            sl.lb.store(f64::INFINITY.to_bits(), Ordering::Relaxed);
            sl.open.store(0, Ordering::Relaxed);
        }
        self.dom.reset_to_root(self.p);
        self.sync_lp();
        ROLE_POOL.with(|r| *r.borrow_mut() = Some(pool.clone()));
        let patience = tunable!("ENOMOTO_T_MIP_HEUR_ROLE_PATIENCE", 20u64, u64);
        // 1 回のサブ MIP の時間の上限 (秒)。木の探索が終わったらサブ MIP も止まる ([`Self::time_up`])
        let call_max = tunable!("ENOMOTO_T_MIP_HEUR_ROLE_CALL_MAX", 5.0, f64);
        let mut fails = 0u64;
        let res = loop {
            if pool.fin.load(Ordering::Acquire) {
                break Some(true);
            }
            if self.time_up() {
                break Some(false);
            }
            // 証明の段階に入ったら木の探索に戻る (役には戻らない、[`Self::heur_role_mine`])
            if proof_phase_stops_role() && pool.proof_phase.load(Ordering::Relaxed) {
                break None;
            }
            self.heur_role_import(&pool);
            let lpx = pool.lp_x.lock().unwrap().clone();
            let x = match (lpx, &self.incumbent) {
                (Some(x), _) => (*x).clone(),
                (None, Some((_, x))) => x.clone(),
                (None, None) => break None,
            };
            let z0 = self.incumbent.as_ref().map_or(f64::INFINITY, |(z, _)| *z);
            let ev0 = if heur_mu_mode() { self.heur_trigger_eval() } else { None };
            let keep = self.submip_time_max;
            self.submip_time_max = keep.min(call_max);
            let t = Instant::now();
            // `ENOMOTO_MIP_ROLE_LMIP` (試作): ALNS と Local-MIP + 連鎖を競わせる ([`Self::role_pick_lmip`])
            let use_lm = self.role_pick_lmip();
            if use_lm {
                self.role_lmip_slice();
            } else {
                self.alns(&x);
            }
            self.submip_time_max = keep;
            let dt = t.elapsed().as_secs_f64();
            if end_diag() && self.time_up() {
                eprintln!("MIP: heuristic role (tid {}): {} call took {dt:.2}s and ended after the stop ({:.2}s)", self.pool_tid, if use_lm { "Local-MIP" } else { "ALNS" }, pool.t0.elapsed().as_secs_f64());
            }
            if role_lmip_on() {
                let arm = usize::from(use_lm);
                self.role_arm_t[arm] += dt;
                if self.incumbent.as_ref().is_some_and(|(z, _)| *z < z0) {
                    self.role_arm_wins[arm] += 1;
                    self.role_leader = arm as u8 + 1;
                }
            }
            if let Some(ev0) = &ev0 {
                let z1 = self.incumbent.as_ref().map_or(z0, |(z, _)| *z);
                self.mu_observe(ev0, z0, z1, dt, true);
            }
            self.lns_secs += dt;
            self.heur_role_stats[0] += 1;
            if self.incumbent.as_ref().is_some_and(|(z, _)| *z < z0) {
                self.heur_role_stats[1] += 1;
                fails = 0;
            } else {
                fails += 1;
                if patience > 0 && fails >= patience {
                    // 木の探索に戻る。役に戻るまでの秒数は戻るたびに倍にする (`ENOMOTO_T_MIP_HEUR_ROLE_COOLDOWN`、既定 5 秒から)
                    let cool = self.heur_role_retired.map_or(tunable!("ENOMOTO_T_MIP_HEUR_ROLE_COOLDOWN", 5.0, f64), |(_, _, c)| 2.0 * c);
                    let best = self.shared.as_ref().map_or(f64::INFINITY, |s| s.best());
                    self.heur_role_retired = Some((Instant::now(), best, cool));
                    break None;
                }
            }
            // 起動の判定を使うとき: 木の証明が時間内に終わりそうになったら木の探索に戻る
            // (予測の残り時間 < `ENOMOTO_T_MIP_HEUR_TRIG_STOP` (既定 0.7) × 残り時間)
            if heur_trigger_mode() {
                if let Some(ev) = self.heur_trigger_eval() {
                    let stop = if heur_mu_mode() { ev.warm && !self.mu_worth(&ev, true) } else { ev.warm && ev.est < tunable!("ENOMOTO_T_MIP_HEUR_TRIG_STOP", 0.7, f64) * ev.left };
                    if stop {
                        if heur_role_verbose() {
                            eprintln!("HEURTRIG: tid {} stop: {}", self.pool_tid, ev.line());
                        }
                        let cool = self.heur_role_retired.map_or(tunable!("ENOMOTO_T_MIP_HEUR_ROLE_COOLDOWN", 5.0, f64), |(_, _, c)| 2.0 * c);
                        let best = self.shared.as_ref().map_or(f64::INFINITY, |s| s.best());
                        self.heur_role_retired = Some((Instant::now(), best, cool));
                        break None;
                    }
                }
            }
            // 近傍が作れずすぐ戻る呼び出しが続くと空回りするので少し待つ
            if dt < 1e-3 {
                std::thread::sleep(Duration::from_millis(1));
            }
        };
        ROLE_POOL.with(|r| *r.borrow_mut() = None);
        pool.in_role.fetch_sub(1, Ordering::Relaxed);
        self.in_heur_role = false;
        if prof_tree_par() || heur_role_verbose() {
            eprintln!(
                "HEURROLE: tid {} entry {} {:.2}s in role ({}), {} LNS calls, {} improved, {} imported, ALNS counts {:?}",
                self.pool_tid,
                self.heur_role_entries,
                t_role.elapsed().as_secs_f64(),
                match res {
                    Some(true) => "tree done",
                    Some(false) => "stopped",
                    None => "back to tree",
                },
                self.heur_role_stats[0],
                self.heur_role_stats[1],
                self.heur_role_stats[2],
                self.alns_count
            );
        }
        if role_lmip_on() && (prof_tree_par() || heur_role_verbose() || env_str!("ENOMOTO_MIP_FJ_DEBUG").is_some()) {
            eprintln!(
                "HEURROLE: tid {} ALNS {:.2}s ({} improvements) vs Local-MIP {:.2}s ({} improvements{}), leader {}",
                self.pool_tid,
                self.role_arm_t[0],
                self.role_arm_wins[0],
                self.role_arm_t[1],
                self.role_arm_wins[1],
                if self.role_lm_off { ", stopped" } else { "" },
                ["none", "ALNS", "Local-MIP"][self.role_leader as usize]
            );
        }
        res
    }

    fn local_redcost_fixing(&mut self, node_obj: f64) -> bool {
        let gap = self.prune_limit() - node_obj;
        if !(gap.is_finite() && gap >= 0.0) {
            return false;
        }
        xp("r_pre");
        // 診断用 (デバッグ解): このノードがデバッグ解を含むなら、固定の根拠の弱双対の不等式 c x >= z + sum_{非基底 j} d_j (x_j - 境界_j) を
        // デバッグ解で確かめる (破れていれば z・d・境界のどれかが今の LP と食い違っている)
        if self.dbg_on && self.dbg_contains() {
            self.dbg_local_redcost_check(node_obj);
        }
        let b = self.lp.basis_ref();
        xp("r_basis");
        let d = self.lp.reduced_costs_ref();
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
                    self.dom.set_external_tag(1);
                    changed |= self.dom.tighten_upper(self.p, j, v);
                    self.dom.set_external(false);
                }
                VarStatus::Upper if d[j] < -1e-7 => {
                    let v = up - gap / (-d[j]);
                    if !(v - lo >= min_cut) {
                        continue;
                    }
                    self.dom.set_external_tag(1);
                    changed |= self.dom.tighten_lower(self.p, j, v);
                    self.dom.set_external(false);
                }
                _ => {}
            }
        }
        changed
    }

    /// 暫定解がない間の木の中のヒューリスティクスの腕を UCB で選ぶ (まだ試していない腕を先に)。平均の報酬は、
    /// その腕の 1 回の平均時間が全体の平均より長いほど割り引く (時間の比の平方根で割る)。
    /// `ENOMOTO_MIP_NOINC_ROOT_PRIOR` なら、根で同じヒューリスティクスを呼んだときの近さを重み
    /// `ENOMOTO_T_MIP_NOINC_PRIOR_W` (既定 1) 回分の試行として足す (1 未満にすると UCB の探索の項が木で 1 回試した腕
    /// より大きくなり、初期値のある腕も結局一通り試してしまう: neos-1456979 で 0.5 は 8 本を全部回した)。
    /// 初期値のある腕は「まだ試していない腕」として先に回さない (根で近づけなかった腕に、木で 1 回を使わない)。
    /// 初期値だけの腕の時間は全体の平均とみなす
    fn noinc_select_arm(&self) -> usize {
        let w = if env_str!("ENOMOTO_MIP_NOINC_ROOT_PRIOR").is_some() { tunable!("ENOMOTO_T_MIP_NOINC_PRIOR_W", 1.0, f64) } else { 0.0 };
        let prior = |a: usize| if w > 0.0 { self.noinc_prior[a] } else { None };
        let n_arms = if env_str!("ENOMOTO_MIP_NOINC_TREE_ARM").is_some() { NOINC_ARMS } else { NOINC_TREE_ARM };
        if let Some(a) = (0..n_arms).find(|&a| self.noinc_count[a] == 0 && prior(a).is_none()) {
            return a;
        }
        let n_eff = |a: usize| self.noinc_count[a] as f64 + if prior(a).is_some() { w } else { 0.0 };
        let total: f64 = (0..NOINC_ARMS).map(n_eff).sum();
        let calls: u32 = self.noinc_count.iter().sum();
        let mean_cost = if calls > 0 { self.noinc_cost.iter().sum::<f64>() / calls as f64 } else { 0.0 } + 1e-3;
        let c = tunable!("ENOMOTO_T_MIP_NOINC_UCB_C", 0.5, f64);
        let score = |a: usize| {
            let n = n_eff(a);
            if n <= 0.0 {
                return f64::NEG_INFINITY;
            }
            let cost = if self.noinc_count[a] > 0 { self.noinc_cost[a] / self.noinc_count[a] as f64 + 1e-3 } else { mean_cost };
            let reward = self.noinc_reward[a] + w * prior(a).unwrap_or(0.0);
            reward / n / (cost / mean_cost).sqrt().max(1.0) + c * (2.0 * total.max(1.0).ln() / n).sqrt()
        };
        (0..n_arms).max_by(|&a, &b| score(a).total_cmp(&score(b))).unwrap()
    }

    fn noinc_update_arm(&mut self, arm: usize, reward: f64, secs: f64) {
        self.noinc_reward[arm] += reward;
        self.noinc_count[arm] += 1;
        self.noinc_cost[arm] += secs;
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
        let b = self.lp.basis_ref();
        let d = self.lp.reduced_costs_ref();
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

    /// 診断用 (`ENOMOTO_MIP_BRANCH_REPLAY`): なぞったノードの終わり方を数え、`ENOMOTO_MIP_REPLAY_LOG` なら
    /// `ER 通し番号 HiGHSのseq(なければ-1) 深さ 終わり方 最初のLP値 最後のLP値 分枝した列 固定された整数列の数 LPの行数 HiGHSの結果 HiGHSのLP値 HiGHSの固定数 経路` を出す。
    fn rp_log(&self, node: &OpenNode, out: &str, obj0: f64, obj: f64, col: i64, st: &mut [u64; 5]) {
        let e = super::replay::lookup(&node.changes);
        st[if e.is_some() { 0 } else { 1 }] += 1;
        if !super::replay::log_on() {
            return;
        }
        if let (Some(e), Some(lim)) = (e, env_str!("ENOMOTO_MIP_REPLAY_DETAIL").and_then(|v| v.parse::<u64>().ok())) {
            if e.seq < lim {
                eprintln!("ERD {}{}", e.seq, self.dom.int_changes_str(self.p));
            }
        }
        let nfix = (0..self.p.n).filter(|&j| self.p.is_int[j] && self.dom.lo[j] >= self.dom.up[j]).count();
        let path: Vec<String> = node.changes.iter().map(|c| format!("{}{}{}", c.col, if c.upper { 'd' } else { 'u' }, c.value)).collect();
        eprintln!(
            "ER {} {} {} {} {:.10} {:.10} {} {} {} {} {:.10} {} {} {}",
            self.nodes,
            e.map_or(-1, |e| e.seq as i64),
            node.depth,
            out,
            obj0,
            obj,
            col,
            nfix,
            self.lp.num_rows(),
            e.map_or(-1, |e| e.res),
            e.map_or(f64::NAN, |e| e.lp),
            e.map_or(-1, |e| e.nfix),
            path.join(" "),
            self.dom.reason_counts_int(self.p)
        );
    }

    /// `ENOMOTO_MIP_SB_AGREE`: 強分岐した候補のうち擬似費用のスコア (強分岐の前の予想) が 1 位の候補が、強分岐のスコアでも
    /// 1 位か、1 位との差が `ENOMOTO_T_MIP_SB_AGREE_TOL` (0.1) 以内なら「一致」として、直近 `ENOMOTO_T_MIP_SB_AGREE_WIN` (20) 回を覚える。
    /// 差の小さい組は、どちらを選んでも木がほとんど変わらないので一致に数える
    fn sb_agree_record(&mut self, ev: &[(f64, f64)]) {
        let best_sb = ev.iter().map(|e| e.1).fold(f64::NEG_INFINITY, f64::max);
        let pc_pick = ev.iter().max_by(|a, b| a.0.total_cmp(&b.0)).map_or(f64::NEG_INFINITY, |e| e.1);
        let tol = tunable!("ENOMOTO_T_MIP_SB_AGREE_TOL", 0.1, f64);
        let agree = pc_pick >= best_sb - tol * best_sb.abs();
        let win = tunable!("ENOMOTO_T_MIP_SB_AGREE_WIN", 20usize, usize).max(1);
        self.sb_agree.push_back(agree);
        while self.sb_agree.len() > win {
            self.sb_agree.pop_front();
        }
        self.sb_agree_stats[0] += 1;
        self.sb_agree_stats[1] += agree as u64;
    }

    /// `ENOMOTO_MIP_SB_AGREE`: 直近の窓が埋まっていて、一致の割合が `ENOMOTO_T_MIP_SB_AGREE_THR` (0.8) 以上か
    fn sb_agree_learned(&self) -> bool {
        if !sb_agree_on() {
            return false;
        }
        let win = tunable!("ENOMOTO_T_MIP_SB_AGREE_WIN", 20usize, usize).max(1);
        self.sb_agree.len() >= win && self.sb_agree.iter().filter(|&&a| a).count() as f64 >= tunable!("ENOMOTO_T_MIP_SB_AGREE_THR", 0.8, f64) * win as f64
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
        // racing ramp-up の多様化: 擬似費用のスコアに ±10% の揺らぎを加える
        if self.race_div & 2 == 2 && !self.race_done {
            for c in cands.iter_mut() {
                self.race_rng ^= self.race_rng << 13;
                self.race_rng ^= self.race_rng >> 7;
                self.race_rng ^= self.race_rng << 17;
                let u = (self.race_rng >> 11) as f64 / (1u64 << 53) as f64;
                c.2 *= 0.9 + 0.2 * u;
            }
        }
        cands.sort_by(|a, b| b.2.total_cmp(&a.2));
        // 予算: ノード LP の反復数 (強分岐を除く) の 0.125 倍 + 25000
        // `ENOMOTO_MIP_SB_BUDGET_HIGHS` (試作): HiGHS (`HighsSearch::selectBranchingCandidate`) と同じ大きさの予算
        // 100000 + 0.5 × (強分岐・ヒューリスティクス以外の LP 反復)。既定の 25000 + 0.125 倍では、h80x6320d の木の主のスレッドが
        // 100 ノードほどで予算に当たり、残りは擬似費用だけで分枝して木が大きくなる (1 スレッドなら 9 ノード、4 スレッドで 200 ノード前後)
        // サブ MIP (RINS など) では使わない (根のヒューリスティクスのサブ MIP が重くなり、h80x6320d の根が 4 秒ほど延びた)
        let sb_highs = !self.params.submip && env_str!("ENOMOTO_MIP_SB_BUDGET_HIGHS").is_some();
        let quot = if sb_highs { tunable!("ENOMOTO_T_MIP_SB_HIGHS_QUOT", 0.5, f64) } else { tunable!("ENOMOTO_T_MIP_SB_QUOT", 0.125, f64) } * self.node_iters as f64;
        let maxsb = quot + if sb_highs { tunable!("ENOMOTO_T_MIP_SB_HIGHS_BASE", 100_000.0, f64) } else { 25_000.0 };
        let sb = self.sb_iters as f64;
        // 双対退化の強いノードでは強分岐をしない (根以外)
        // `ENOMOTO_MIP_SB_NO_DEGEN_SKIP`: 双対退化でも強分岐を止めない
        let degenerate = depth > 0 && env_str!("ENOMOTO_MIP_SB_NO_DEGEN_SKIP").is_none() && {
            let (deg, ratio) = self.dual_degeneracy();
            deg >= 0.8 || ratio >= 2.0
        };
        self.sb_degen_stats[0] += 1;
        self.sb_degen_stats[1] += degenerate as u64;
        // `ENOMOTO_MIP_SB_AGREE` (試作): 擬似費用の選び方が強分岐と十分に一致するようになったら (根以外)、強分岐を省いて
        // 擬似費用だけで分枝する。学び続けるため、省いたノードの `ENOMOTO_T_MIP_SB_AGREE_PROBE` 回に 1 回は強分岐する
        let agree_skip = depth > 0 && self.sb_agree_learned() && {
            self.sb_agree_tick += 1;
            self.sb_agree_tick % tunable!("ENOMOTO_T_MIP_SB_AGREE_PROBE", 10u64, u64).max(1) != 0
        };
        self.sb_agree_stats[2] += agree_skip as u64;
        let allow_sb = sb <= maxsb && !degenerate && !agree_skip;
        // 信頼度のしきい値: 予算の残りに応じて 1..5
        let mut prio = ((maxsb - sb) / (sb + 1.0)).min(1.0);
        prio = prio.max((quot - sb) / (sb + 1.0)).max(0.0);
        // `ENOMOTO_MIP_SUBMIP_HIGHS` の `s`: サブ MIP ではどの候補も信頼できるとみなす (HiGHS の `mip_pscost_minreliable = 0`。
        // `pc.min_reliable` はこの選び方では使われないので、`ENOMOTO_MIP_SUBMIP_NO_SB` だけでは強分岐は止まらない)
        let reliable_thr = if self.params.submip && super::submip_highs('s') { 0.0 } else { (1.0 - prio) * 1.0 + prio * 5.0 };
        // 強分岐 LP の反復上限 (平均ノード反復の `ENOMOTO_T_MIP_SB_ITERMULT` 倍、10〜500)。倍率 2 -> 4 で 40 問 90 秒の SGM が
        // 5 台のマシン (各 2 回、マシンごとの基準と比較) で -2.7, -1.0, -2.1, +0.2, -2.8% (平均 -1.7%) なので 4 にした
        // (analysis/mip_opt_eval_20261011.md)
        let sb_iter_limit = ((tunable!("ENOMOTO_T_MIP_SB_ITERMULT", 4.0, f64) * self.avg_node_iters() as f64 * (1.0 + 20.0 / self.nodes.max(1) as f64)) as u64).clamp(10, 500);
        let unreliable: usize = cands.iter().filter(|&&(j, _, _)| (self.pc.min_observations(j) as f64) < reliable_thr).count();
        let lookahead = (tunable!("ENOMOTO_T_MIP_SB_LOOKAHEAD", 9.0, f64) * (1.0 + unreliable as f64 / cands.len().max(1) as f64)) as usize;
        let max_tried = tunable!("ENOMOTO_T_MIP_SB_MAXCAND", 100usize, usize);
        let mut best: Option<(usize, f64, f64)> = None; // (列, 値, スコア)
        self.last_sb = None;
        let mut best_sb: Option<(usize, f64, f64)> = None; // best が強分岐で評価した列なら (列, 下の子の値, 上の子の値)
        let mut no_improve = 0.0f64;
        let mut tried = 0usize;
        let mut bound_changes = 0usize;
        // 強分岐の前の LP の状態 (最初に強分岐する候補で 1 回だけ保存する)。各候補の子の LP は解いた後でこの状態に戻し、
        // 候補の間では LP に触れない (境界の締め付けは定義域だけ) ので、候補ごとに保存し直しても同じ状態になる
        let mut sb_saved: Option<L::State> = None;
        // `ENOMOTO_MIP_PAR_SB`: 木を分け合う前 (主だけで探索している間) か、ほかの木のスレッドが全て待っているときは、強分岐する
        // 候補をスレッド数ずつまとめて、LP の複製で子の LP を並列に解く。結果は逐次と同じ順に当てはめる (打ち切ったら残りは
        // 捨てる)。逐次との違い: まとめた間に暫定解が良くなっても打ち切り値は解き始めたときのもの、子の LP 解が整数で暫定解に
        // ならないときの連続列の解き直しはしない
        let par_sb = self.params.threads > 1
            && !self.params.submip
            && env_str!("ENOMOTO_MIP_SB_PROP").is_none()
            && env_str!("ENOMOTO_MIP_PAR_SB").is_some()
            && self.pool.as_ref().is_none_or(|pl| pl.idle.load(std::sync::atomic::Ordering::Relaxed) + 1 >= pl.threads);
        let mut pre: std::collections::HashMap<usize, [SbSide; 2]> = std::collections::HashMap::new();
        // このノードで強分岐した候補の (擬似費用のスコア (強分岐の前), 強分岐のスコア)。`ENOMOTO_MIP_SB_AGREE` の一致の判定に使う
        let mut sb_eval: Vec<(f64, f64)> = Vec::new();
        for (ci, &(j, v, pscore)) in cands.iter().enumerate() {
            let is_reliable = (self.pc.min_observations(j) as f64) >= reliable_thr;
            if is_reliable || !allow_sb || tried >= max_tried || no_improve >= lookahead as f64 || self.time_up() || (self.sb_iters as f64) > maxsb {
                if best.is_none_or(|(_, _, s)| pscore > s) {
                    best = Some((j, v, pscore));
                    best_sb = None;
                }
                continue;
            }
            tried += 1;
            // 根の解発見スレッドが動いている間は、強分岐の候補ごとにその解を受け取る (既定で使う。`ENOMOTO_MIP_NO_FIND_POLL_SB` で使わない)。
            // neos-950242 では解発見スレッドの局所探索が 12.5 秒に 4 (= 根の LP の値) を出すが、主は最初のノードの強分岐
            // (約 13 秒) の間受け取らず、25.5 秒まで暫定解が 19 のままだった。暫定解でこのノードが刈れるなら強分岐をやめる
            // (候補の間は LP に触れないので、そのまま返してよい)
            if self.find_thread.is_some() && env_str!("ENOMOTO_MIP_NO_FIND_POLL_SB").is_none() {
                self.poll_find_thread();
                if node_obj >= self.prune_limit() {
                    return BranchAction::Prune;
                }
            }
            // 強分岐
            let saved = &*sb_saved.get_or_insert_with(|| self.lp.save_state());
            let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
            let mut gains = [0.0f64; 2];
            let mut child_obj = [f64::NEG_INFINITY; 2];
            let mut cut = [false; 2];
            if par_sb && !pre.contains_key(&j) {
                // この候補から、強分岐するはずの候補 (信頼できないもの) をスレッド数まで集めて並列に解く
                let nth = self.params.threads;
                let mut batch: Vec<(usize, f64, f64, f64)> = vec![(j, v, lo, up)];
                for &(j2, v2, _) in &cands[ci + 1..] {
                    if batch.len() >= nth || tried + batch.len() > max_tried {
                        break;
                    }
                    if (self.pc.min_observations(j2) as f64) < reliable_thr {
                        batch.push((j2, v2, self.dom.lo[j2], self.dom.up[j2]));
                    }
                }
                let lim = self.limits(sb_iter_limit);
                let offset = self.p.offset;
                let lp0 = &self.lp;
                let saved0 = saved;
                let batch_ref = &batch;
                let outs: Vec<Vec<(usize, [SbSide; 2])>> = std::thread::scope(|sc| {
                    let hs: Vec<_> = (0..nth.min(batch.len()))
                        .map(|t| {
                            let mut lpc = lp0.clone();
                            let saved_c = saved0.clone();
                            let lim_c = lim.clone();
                            sc.spawn(move || {
                                let mut out = Vec::new();
                                let mut k = t;
                                while k < batch_ref.len() {
                                    let (jj, vv, l0, u0) = batch_ref[k];
                                    let mut sides: Vec<SbSide> = Vec::with_capacity(2);
                                    for is_up in [false, true] {
                                        if is_up {
                                            lpc.set_col_bounds(jj, vv.ceil(), u0);
                                        } else {
                                            lpc.set_col_bounds(jj, l0, vv.floor());
                                        }
                                        let it0 = lpc.total_iterations();
                                        let st = lpc.solve(&lim_c);
                                        let iters = lpc.total_iterations() - it0;
                                        let obj = lpc.objective() + offset;
                                        let x = (st == LpStatus::Optimal).then(|| lpc.col_values());
                                        let y = match st {
                                            LpStatus::Infeasible => lpc.farkas_ray(),
                                            LpStatus::Optimal | LpStatus::ObjectiveBound => Some(lpc.row_duals()),
                                            _ => None,
                                        };
                                        sides.push(SbSide { st, obj, iters, x, y });
                                        lpc.restore_state(&saved_c);
                                    }
                                    let up_side = sides.pop().unwrap();
                                    let down_side = sides.pop().unwrap();
                                    out.push((jj, [down_side, up_side]));
                                    k += nth;
                                }
                                out
                            })
                        })
                        .collect();
                    hs.into_iter().map(|h| h.join().unwrap()).collect()
                });
                for o in outs {
                    for (jj, r) in o {
                        pre.insert(jj, r);
                    }
                }
                self.par_sb_stats[0] += 1;
                self.par_sb_stats[1] += batch.len() as u64;
            }
            if let Some(rs) = pre.remove(&j) {
                for (side, (is_up, r)) in [false, true].into_iter().zip(rs).enumerate() {
                    self.sb_iters += r.iters;
                    self.sb_lps += 1;
                    match r.st {
                        LpStatus::Optimal => {
                            let o = r.obj;
                            gains[side] = (o - node_obj).max(0.0);
                            child_obj[side] = o;
                            let delta = if is_up { v.ceil() - v } else { v - v.floor() };
                            self.pc.add_observation(j, is_up, delta, gains[side]);
                            if o >= self.prune_limit() {
                                cut[side] = true;
                                if self.dbg_on {
                                    self.dbg_sb_cut_check(j, is_up, v, r.st, o);
                                }
                                if let Some(y) = &r.y {
                                    self.sb_child_proof_y(j, is_up, v, false, y);
                                }
                            } else if let Some(x) = r.x {
                                if self.fractional(&x).is_empty() {
                                    self.try_incumbent(x);
                                }
                            }
                        }
                        LpStatus::Infeasible | LpStatus::ObjectiveBound => {
                            cut[side] = true;
                            self.pc.add_cutoff(j, is_up);
                            if self.dbg_on {
                                self.dbg_sb_cut_check(j, is_up, v, r.st, r.obj);
                            }
                            if let Some(y) = &r.y {
                                self.sb_child_proof_y(j, is_up, v, r.st == LpStatus::Infeasible, y);
                            }
                        }
                        _ => {
                            gains[side] = (r.obj - node_obj).max(0.0);
                        }
                    }
                }
            } else {
            for (side, is_up) in [(0usize, false), (1usize, true)] {
                // 子の定義域を LP の前に伝播する (HiGHS): 矛盾すれば LP を解かずに切る (衝突解析もする)。矛盾しなければ
                // 伝播で締まった境界も子の LP に渡す
                // 既定では無効 (`ENOMOTO_MIP_SB_PROP` で有効): neos-911970 では最適解が見つかったが、misc07・binkar10_1 で
                // 子の LP を省いた分の評価の違いなどから大きく悪化した
                let sb_prop = env_str!("ENOMOTO_MIP_SB_PROP").is_some();
                let pos = self.dom.stack_len();
                if sb_prop {
                    if is_up {
                        self.dom.tighten_lower(self.p, j, v.ceil());
                    } else {
                        self.dom.tighten_upper(self.p, j, v.floor());
                    }
                    let ok = !self.dom.infeasible && self.dom.propagate(self.p) && self.propagate_objective() && {
                        match self.lit_conflicts.propagate(self.p, &mut self.dom, pos) {
                            Ok(_) => true,
                            Err(()) => false,
                        }
                    };
                    if !ok {
                        if self.dom.conflict.is_some() {
                            self.add_conflict();
                        }
                        self.dom.backtrack_to(self.p, pos);
                        cut[side] = true;
                        self.pc.add_cutoff(j, is_up);
                        self.sb_prop_stats[0] += 1;
                        continue;
                    }
                    if env_str!("ENOMOTO_MIP_SB_PROP_NO_LP").is_none() {
                        for (c, _, _) in self.dom.changes_since(pos) {
                            self.lp.set_col_bounds(c, self.dom.lo[c], self.dom.up[c]);
                            self.sb_prop_stats[1] += 1;
                        }
                    }
                }
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
                            if self.dbg_on {
                                self.dbg_sb_cut_check(j, is_up, v, st, o);
                            }
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
                        if self.dbg_on {
                            let o = self.lp.objective() + self.p.offset;
                            self.dbg_sb_cut_check(j, is_up, v, st, o);
                        }
                        self.sb_child_proof(j, is_up, v, st == LpStatus::Infeasible);
                    }
                    _ => {
                        // 反復上限: 途中の目的値 (双対単体法なので下界) を弱い観測として使う
                        gains[side] = (self.lp.objective() + self.p.offset - node_obj).max(0.0);
                    }
                }
                self.lp.restore_state(saved);
                if sb_prop {
                    self.dom.backtrack_to(self.p, pos);
                }
            }
            }
            match (cut[0], cut[1]) {
                (true, true) => return BranchAction::Prune,
                (true, false) => {
                    let had = self.dbg_on && self.dbg_contains();
                    self.dom.set_external_tag(2);
                    self.dom.tighten_lower(self.p, j, v.ceil());
                    self.dom.set_external(false);
                    if had && !self.dbg_contains() {
                        self.dbg_lost(&format!("strong branching fixing x{j} >= {}", v.ceil()), true);
                    }
                    bound_changes += 1;
                }
                (false, true) => {
                    let had = self.dbg_on && self.dbg_contains();
                    self.dom.set_external_tag(2);
                    self.dom.tighten_upper(self.p, j, v.floor());
                    self.dom.set_external(false);
                    if had && !self.dbg_contains() {
                        self.dbg_lost(&format!("strong branching fixing x{j} <= {}", v.floor()), true);
                    }
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
            sb_eval.push((pscore, score));
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
        if sb_agree_on() && sb_eval.len() >= 2 {
            self.sb_agree_record(&sb_eval);
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
                            if self.dbg_on {
                                self.dbg_sb_cut_check(j, is_up, v, st, o);
                            }
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
                        if self.dbg_on {
                            let o = self.lp.objective() + self.p.offset;
                            self.dbg_sb_cut_check(j, is_up, v, st, o);
                        }
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
                    self.dom.set_external_tag(2);
                    self.dom.tighten_lower(self.p, j, v.ceil());
                    self.dom.set_external(false);
                    return BranchAction::Resolve;
                }
                (false, true) => {
                    self.dom.set_external_tag(2);
                    self.dom.tighten_upper(self.p, j, v.floor());
                    self.dom.set_external(false);
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

    /// 根の LP が解けずに終わるときの下界: 木を分け合う従のスレッドは、共有の待ち行列からノードを取る前なので探索し残した
    /// 部分木がなく、主の下界を下げない (+inf)。主 (ほかの求解) は根の下界がない (-inf。カット前の根の LP が解けていれば
    /// [`Self::finish`] が `root_lb` で抑える)。
    fn root_fail_bound(&self) -> f64 {
        if self.pool.is_some() && self.snap.is_some() { f64::INFINITY } else { f64::NEG_INFINITY }
    }

    fn finish_limit(&mut self, status: MipStatus, current: Option<&OpenNode>) -> MipResult {
        // 木を分け合う従: 残ったノードを共有の待ち行列に返す (主がそれも含めて下界を出す)
        if let (Some(pool), true) = (self.pool.clone(), self.snap.is_some()) {
            let mut back: Vec<SnapNode> = self.queue.iter().map(SnapNode::of).collect();
            if let Some(n) = current {
                back.push(SnapNode::of(n));
            }
            if self.dbg_on && back.iter().any(|n| self.dbg_changes(&n.changes)) {
                eprintln!("MIP_DEBUG_SOL: tree thread {} stopped ({status:?}) holding a node that contains the debug solution (returned to the shared queue)", self.pool_tid);
            }
            if tree_diag() && !back.is_empty() {
                eprintln!("MIP_TREE_DIAG: tid {} stopped ({status:?}) holding {} nodes (min lower bound {:.10e}, counted as {})", self.pool_tid, back.len(), back.iter().map(|n| n.lower_bound).fold(f64::INFINITY, f64::min), if self.pool_busy { "busy" } else { "waiting" });
            }
            let mut g = pool.inner.lock().unwrap();
            g.nodes.extend(back);
            pool.len.store(g.nodes.len(), std::sync::atomic::Ordering::Relaxed);
        }
        let mut lb = self.queue.best_lower_bound();
        if let Some(n) = current {
            lb = lb.min(n.lower_bound);
        }
        if let Some((z, _)) = &self.incumbent {
            lb = lb.min(*z);
        }
        // 目的値の二分探索で証明した下界
        lb = lb.max(self.bisect_lb.min(self.incumbent.as_ref().map_or(f64::INFINITY, |(z, _)| *z)));
        self.finish(status, lb)
    }

    /// 双対証明を作ってプールに入れる (HiGHS の dual proof、衝突分析の LP 版)。LP は最適で、目的値が
    /// 打ち切り値以上のときに呼ぶ。双対値 y で行を足し、目的関数の上限 `c x <= U` と組み合わせると
    /// `(c - y^T A) x <= U - sum_i y_i b_i` (`b_i` は y_i > 0 なら行の下限、y_i < 0 なら上限) が全ての
    /// 改善解で成り立つ (行は大域的に成り立つ元の行かカット)。今のノードの境界ではこれが破れている。
    /// 係数の小さい列と大域的に固定された列は大域的な境界で右辺に移す。密すぎるものは捨てる。
    pub(super) fn add_dual_proof(&mut self) {
        // サブ MIP では既定で使わない (`ENOMOTO_MIP_SUBMIP_PROOFS` で使う。HiGHS のサブ MIP は本体と同じく使う)
        if self.params.submip && env_str!("ENOMOTO_MIP_SUBMIP_PROOFS").is_none() && !super::submip_highs('f') || env_str!("ENOMOTO_MIP_NO_DUAL_PROOF").is_some() {
            return;
        }
        let y = self.lp.row_duals();
        self.add_proof_from(&y, true);
    }

    /// 完全オービトープの固定と伝播を、固定が出なくなるまで (最大 10 回) 繰り返す。矛盾したら偽。
    /// 待ち行列のノードの基底を LP に置く (保存した後の行の追加・削除を当てはめる)。
    fn restore_node_basis(&mut self, b: &super::lp::Basis, epoch: usize, dse: Option<&[(u32, f32)]>) {
        let mr = self.lp.num_rows();
        if env_str!("ENOMOTO_MIP_RESTORE_DIAG").is_some() && epoch <= self.row_log.len() {
            let (mut a, mut d) = (0usize, 0usize);
            for e in &self.row_log[epoch..] {
                match e {
                    RowEdit::Add(k) => a += *k,
                    RowEdit::Delete(mask) => d += mask.iter().filter(|&&x| x).count(),
                }
            }
            self.restore_diag = (a, d, false);
        }
        // 調査用: 保存した基底を使わず、今の LP の基底 (直前のノードの最適基底) のまま解く
        if env_str!("ENOMOTO_MIP_RESTORE_NEVER").is_some() {
            return;
        }
        if epoch == self.row_log.len() && b.row.len() == mr {
            self.restore_stats.0 += 1;
            self.restored_now = true;
            self.lp.set_basis(b);
            if let Some(w) = dse {
                self.lp.set_dse_vars(w);
                self.restored_dse = true;
            }
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
        // `ENOMOTO_MIP_HCM_REPAIR`: HiGHS 流のカット管理でも今の基底のままにせず、この修復で保存した基底を使う
        let hcm_repair = self.highs_cutmgmt() && env_str!("ENOMOTO_MIP_HCM_REPAIR").is_some();
        if nb > mr && (env_str!("ENOMOTO_MIP_RESTORE_SMART_REPAIR").is_some() || hcm_repair) {
            // 基底変数が多すぎる (保存した基底で効いていたカットの行が消えた): 余分を、今の LP の基底 (直前のノードの
            // 最適基底) で非基底のもの (境界の近くにありそう) から、今の LP と同じ側の境界で非基底にして減らす
            let cur = self.lp.basis();
            let mut excess = nb - mr;
            for j in 0..b2.col.len().min(cur.col.len()) {
                if excess == 0 {
                    break;
                }
                if b2.col[j] == VarStatus::Basic && cur.col[j] != VarStatus::Basic {
                    b2.col[j] = cur.col[j];
                    excess -= 1;
                }
            }
            for i in 0..b2.row.len().min(cur.row.len()) {
                if excess == 0 {
                    break;
                }
                if b2.row[i] == VarStatus::Basic && cur.row[i] != VarStatus::Basic {
                    b2.row[i] = cur.row[i];
                    excess -= 1;
                }
            }
        }
        let nb = b2.col.iter().chain(b2.row.iter()).filter(|&&s| s == VarStatus::Basic).count();
        if nb != mr {
            self.restore_diag.2 = true;
            self.restore_stats.1 += 1;
            // 保存した基底で非基底だった (効いていた) カットの行が削除されていると基底変数が多すぎる。既定では
            // set_basis の修復 (構造変数を後ろから外す) に任せる。`ENOMOTO_MIP_RESTORE_KEEP_CURRENT` なら今の LP の
            // 基底 (直前のノードの最適基底) のまま解く (HiGHS と同じ。neos-911970 では 1 ノードの反復が大きく減るが、
            // misc07 などでは遠いノードの基底から解くことになり悪化した: 40 問で 19 -> 18 問)
            // HiGHS 流のカット管理 (`highs_cutmgmt`) でも今の基底のまま解く (neos-911970: 保存した基底を修復して使うと
            // ノードの LP は 1 回 75 反復で木の下界が根から動かず、今の基底なら 36 反復で 17000 ノード、下界 54.05・最適解 54.76 を発見)
            // `ENOMOTO_MIP_FC_NODE_SEP2` でも今の基底のまま解く: 取り出したノードごとに分離するとカットの行の出入りが多く、
            // 保存した基底の修復からの解き直しが重い (mc11、4 スレッド・60 秒: 取り出した後の最初の LP が 1 回平均 1400 反復
            // (118 回、うち不一致 36) -> 74 反復 (231 回、うち不一致 131))。`ENOMOTO_MIP_FC_SEP2_REPAIR` で従来の修復
            let fc2_keep = (self.fc_node_sep && self.fc_sep2 && env_str!("ENOMOTO_MIP_FC_SEP2_REPAIR").is_none())
                || (self.tree_sep && env_str!("ENOMOTO_MIP_TREE_SEP_REPAIR").is_none());
            if env_str!("ENOMOTO_MIP_RESTORE_KEEP_CURRENT").is_some() || (self.highs_cutmgmt() && !hcm_repair) || fc2_keep {
                return;
            }
        }
        self.restored_now = true;
        self.lp.set_basis(&b2);
        if let Some(w) = dse {
            // 保存したときの行の番号を今の行の番号に直す (行の追加・削除の記録を当てはめる)
            let n = b.col.len();
            let m_saved = b.row.len();
            let mut orig: Vec<usize> = (0..m_saved).collect();
            if epoch <= self.row_log.len() {
                for e in &self.row_log[epoch..] {
                    match e {
                        RowEdit::Add(k) => orig.extend(std::iter::repeat_n(usize::MAX, *k)),
                        RowEdit::Delete(mask) => {
                            let mut i = 0;
                            orig.retain(|_| {
                                let keep = !mask.get(i).copied().unwrap_or(false);
                                i += 1;
                                keep
                            });
                        }
                    }
                }
                let mut cur_of = vec![u32::MAX; m_saved];
                for (ci, &o) in orig.iter().enumerate() {
                    if o < m_saved {
                        cur_of[o] = (n + ci) as u32;
                    }
                }
                let mapped: Vec<(u32, f32)> = w
                    .iter()
                    .filter_map(|&(v, x)| {
                        let v = v as usize;
                        if v < n {
                            Some((v as u32, x))
                        } else if v - n < m_saved && cur_of[v - n] != u32::MAX {
                            Some((cur_of[v - n], x))
                        } else {
                            None
                        }
                    })
                    .collect();
                self.lp.set_dse_vars(&mapped);
                self.restored_dse = true;
            }
        }
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
        // オービトープの生成元が動かす、オービトープの外の列 (連続列・一般整数列など)。これに分枝した道では、ノードの
        // 領域が列の入れ替えで閉じなくなり、分枝した行の順の辞書式で最大の代表がどのノードにも残らないことがある
        // (rout: 一般整数列への分枝の後の固定で最適解の像が全て消え、1079.19 を最適と報告した)。その道ではその
        // オービトープの固定をしない (`ENOMOTO_MIP_ORBITOPE_IGNORE_OTHERS` で以前の形)
        if self.orb_others.is_none() {
            let others: Vec<std::collections::HashSet<usize>> = orbs
                .iter()
                .map(|o| {
                    let inside: std::collections::HashSet<usize> = o.vars.iter().flatten().copied().collect();
                    let mut set = std::collections::HashSet::new();
                    for (_, _, perm) in &o.swaps {
                        for (j, &t) in perm.iter().enumerate() {
                            if t != j && !inside.contains(&j) {
                                set.insert(j);
                            }
                        }
                    }
                    set
                })
                .collect();
            self.orb_others = Some(Rc::new(others));
        }
        let others = self.orb_others.clone().unwrap();
        let check_others = env_str!("ENOMOTO_MIP_ORBITOPE_IGNORE_OTHERS").is_none();
        // オービトープごとの分枝した行 (分枝した順)
        let subs: Vec<super::symmetry::Orbitope> = if dynamic {
            let mut rows: Vec<Vec<usize>> = vec![Vec::new(); orbs.len()];
            let mut broken = vec![false; orbs.len()];
            for c in changes {
                if let Some(&(k, i)) = self.orb_col.get(&c.col) {
                    if !rows[k].contains(&i) {
                        rows[k].push(i);
                    }
                }
                if check_others {
                    for (k, o) in others.iter().enumerate() {
                        if o.contains(&c.col) {
                            broken[k] = true;
                        }
                    }
                }
            }
            rows.iter().enumerate().filter(|(k, r)| !r.is_empty() && !broken[*k]).map(|(k, r)| orbs[k].sub_rows(r)).collect()
        } else {
            orbs.iter().cloned().collect()
        };
        if subs.is_empty() {
            return true;
        }
        for _ in 0..10 {
            let mut fixes: Vec<(usize, f64, f64)> = Vec::new();
            for o in subs.iter() {
                // 0-1 でない整数の行を含むオービトープは整数の値で比べる形で固定する
                let int_rows = o.vars.iter().any(|line| line.iter().any(|&j| self.p.col_lo[j] < 0.0 || self.p.col_up[j] > 1.0));
                if int_rows {
                    match super::symmetry::orbitopal_fixing_int(o, &self.dom.lo, &self.dom.up) {
                        None => return false,
                        Some(f) => fixes.extend(f),
                    }
                } else {
                    match super::symmetry::orbitopal_fixing(o, &self.dom.lo, &self.dom.up) {
                        None => return false,
                        Some(f) => fixes.extend(f.into_iter().map(|(j, v)| (j, v, v))),
                    }
                }
            }
            if fixes.is_empty() {
                return true;
            }
            self.orbitope_fixings += fixes.len() as u64;
            for (j, l, u) in fixes {
                self.dom.set_external_tag(3);
                self.dom.tighten_lower(self.p, j, l);
                self.dom.set_external(false);
                self.dom.set_external_tag(3);
                self.dom.tighten_upper(self.p, j, u);
                self.dom.set_external(false);
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
    /// [`Self::sb_child_proof`] と同じだが、子の LP の行の重み `y` (別スレッドで解いたもの) から作る。
    fn sb_child_proof_y(&mut self, j: usize, is_up: bool, v: f64, infeasible: bool, y: &[f64]) {
        let dual_ok = !(self.params.submip && env_str!("ENOMOTO_MIP_SUBMIP_PROOFS").is_none() && !super::submip_highs('f') || env_str!("ENOMOTO_MIP_NO_DUAL_PROOF").is_some());
        let farkas_ok = !(self.params.submip && env_str!("ENOMOTO_MIP_SUBMIP_PROOFS").is_none() && !super::submip_highs('f') || env_str!("ENOMOTO_MIP_NO_FARKAS").is_some());
        if env_str!("ENOMOTO_MIP_SB_PROOF_OLD").is_some() {
            if !infeasible && dual_ok {
                self.add_proof_from(y, true);
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
                if farkas_ok && !self.add_proof_from(y, false) {
                    let neg: Vec<f64> = y.iter().map(|v| -v).collect();
                    self.add_proof_from(&neg, false);
                }
            } else if dual_ok {
                self.add_proof_from(y, true);
            }
        }
        self.dom.backtrack_to(self.p, pos);
    }

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
        if self.params.submip && env_str!("ENOMOTO_MIP_SUBMIP_PROOFS").is_none() && !super::submip_highs('f') || env_str!("ENOMOTO_MIP_NO_FARKAS").is_some() {
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
        // 実行不能の証明 (Farkas) は斉次なので、射線を最大の成分で 1 に正規化する (下の数値誤差の見積もりを小さく保つ)
        let y_norm: Vec<f64>;
        let y: &[f64] = if obj {
            y
        } else {
            let m = y.iter().take(self.lp.num_rows()).fold(0.0f64, |m, v| m.max(v.abs()));
            if m > 0.0 && m.is_finite() {
                y_norm = y.iter().map(|v| v / m).collect();
                &y_norm
            } else {
                y
            }
        };
        let mut d = if obj { p.cost.clone() } else { vec![0.0; p.n] };
        let mut konst = 0.0;
        let mut skipped = 0usize;
        // 数値誤差の見積もり: 行の重み y_i と行の境界、係数と列の境界の大きさに比例する項の和 (下で破れの幅と比べる)
        let mut num_err = 0.0f64;
        for (i, &yi) in y.iter().enumerate().take(self.lp.num_rows()) {
            if yi.abs() <= 1e-12 {
                continue;
            }
            let (lo, up) = self.lp.row_bounds(i);
            let b = if yi > 0.0 { lo } else { up };
            if !b.is_finite() {
                skipped += 1;
                continue; // その行は使わない (y_i = 0 とみなす。d もそれに合わせて作る)
            }
            konst += yi * b;
            num_err += yi.abs() * (1.0 + b.abs());
            for (j, a) in self.lp.row(i) {
                d[j] -= yi * a;
            }
        }
        let dmax = d.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        for (j, &dj) in d.iter().enumerate() {
            if dj != 0.0 {
                let b = if dj > 0.0 { self.dom.lo[j] } else { self.dom.up[j] };
                let b = if b.is_finite() { b } else if dj > 0.0 { self.dom.global_lo[j] } else { self.dom.global_up[j] };
                if b.is_finite() {
                    num_err += dj.abs() * (1.0 + b.abs());
                }
            }
        }
        // 行の重みが極端に大きい (強分枝の子が実行不能に近く、双対単体法が双対の射線に沿って進む途中で打ち切り値を越えて止まった
        // ときなど、|y| が 1e20 になった例がある) と、konst = Σ y_i b_i と d = c - y^T A は桁落ちで意味を失い、行を 1e-13 しか破らない
        // 最適解をも切る「証明」になる (mc11 の誤った最適値 11728 の原因。analysis/partree_race_20261021.md §9.6)。どの y でも数学的には
        // 正しいが、浮動小数では |y| × (行・境界の許容誤差) の分だけ破れの幅に余裕が要る
        num_err *= tunable!("ENOMOTO_T_MIP_PROOF_NUM_TOL", 1e-9, f64);
        if dmax <= 1e-9 {
            return false;
        }
        // `ENOMOTO_MIP_PROOF_SIDE_REDUCE=dense`: 側別に移すのは、移さないと密すぎて捨てる証明か、移すと大きく縮む証明だけ (作り直す)。
        // 常に移すと (=1) 証明が大域的に弱くなる: 移した列は、他のノードでその列の境界が締まっても証明に効かない
        // (mas76 で 11 秒 -> 35 秒)
        let side_mode = proof_side_reduce_mode();
        let konst_rows = konst;
        let dense_len = tunable!("ENOMOTO_T_MIP_PROOF_DENSITY", 0.5, f64).mul_add(p.n as f64, 20.0) as usize;
        let mut side = side_mode == 1;
        let mut coefs: Vec<(usize, f64)>;
        loop {
        konst = konst_rows;
        coefs = Vec::new();
        for (j, &dj) in d.iter().enumerate() {
            if dj == 0.0 {
                continue;
            }
            let (gl, gu) = (self.dom.global_lo[j], self.dom.global_up[j]);
            let fixed = gl == gu;
            // ノードでの境界が大域的な境界と同じ列は、どのノードでも最小値が同じなので右辺に移しても
            // このノードで破れたまま (分枝・伝播で締まった列だけが残る短い証明になる)
            let unchanged = self.dom.lo[j] == gl && self.dom.up[j] == gu && env_str!("ENOMOTO_MIP_PROOF_NO_REDUCE").is_none();
            // `ENOMOTO_MIP_PROOF_SIDE_REDUCE`: 最小活動量を決める側の境界 (d_j > 0 なら下限、d_j < 0 なら上限) だけが大域と
            // 同じ列も移す (反対側の境界が締まっていても、このノードの最小活動量は変わらないので破れの幅は同じ)。
            // 容量制約の施設型 (x_ij <= U y_j の集約) で、閉じた施設の x を 0 に固定した列が証明に残って密になるのを防ぐ
            let unchanged = unchanged || (side && if dj > 0.0 { self.dom.lo[j] == gl } else { self.dom.up[j] == gu });
            // `ENOMOTO_MIP_PROOF_KEEP_INT` (試作): HiGHS の双対証明と同じく、大域的に固定されていない整数列は境界が大域と
            // 同じでも残し、連続列だけを (最小活動量を決める側の境界が大域と同じなら) 右辺に移す。整数列を移すと、証明が
            // 「このノードで固定した列」だけの局所的なものになり、CMIR で強めても弱いカットにしかならない
            // (neos-2978193-inde の最初の葉: HiGHS は y 8 本の「5 本以上開く」、移すと固定した 4 本の「2 本以上」)
            let (unchanged, keep) = if proof_keep_int() {
                if p.is_int[j] {
                    (false, !fixed && dj.abs() > 1e-9 * dmax)
                } else {
                    (if dj > 0.0 { self.dom.lo[j] == gl } else { self.dom.up[j] == gu }, false)
                }
            } else {
                (unchanged, false)
            };
            if keep {
                coefs.push((j, dj));
                continue;
            }
            if fixed || unchanged || dj.abs() <= 1e-9 * dmax {
                // d_j x_j >= d_j * (下の側の大域的な境界) で右辺に移す: sum_{他} <= U - konst - d_j x_j <= U - (konst + min)
                let m = if dj > 0.0 { dj * gl } else { dj * gu };
                if !m.is_finite() {
                    // 無限の境界の側に向く極小の係数 (行を足した丸め誤差、neos-911970 で 1e-15 程度) は 0 とみなす
                    // (残すと最小活動量が -inf になり証明全体が捨てられる。HiGHS も極小の値は無視する)
                    if fixed || (dj.abs() <= 1e-9 * dmax && env_str!("ENOMOTO_MIP_PROOF_KEEP_TINY").is_none()) {
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
        // 密すぎる証明か、側別に移すと 1/10 未満に縮む証明 (このノードで締まった列のほとんどが最小活動量に効かない側だけ
        // 締まっている。neos-2978193-inde: 約 5000 列 -> 約 9 列) は作り直す
        if side_mode == 2 && !side && coefs.len() > 1 && {
            let rest = coefs.iter().filter(|&&(j, a)| !(if a > 0.0 { self.dom.lo[j] == self.dom.global_lo[j] } else { self.dom.up[j] == self.dom.global_up[j] })).count();
            coefs.len() > dense_len || 10 * rest < coefs.len()
        } {
            side = true;
            continue;
        }
        break;
        }
        // `ENOMOTO_MIP_PROOF_SHORTEN`: 分枝・伝播で締まった列も、このノードで破れている範囲で右辺に移して短くする。列 j を
        // 大域的な境界で移すと、このノードの最小活動量は d_j (l_j - L_j) (d_j < 0 なら d_j (u_j - U_j)) だけ下がる (損)。
        // 損の小さい順に、破れの幅 (最小活動量 - 右辺) の半分を超えない範囲で移す
        if env_str!("ENOMOTO_MIP_PROOF_SHORTEN").is_some() && coefs.len() > 1 {
            let rhs0 = if obj { self.prune_limit() - p.offset - konst } else { -konst };
            let minact0: f64 = coefs.iter().map(|&(j, a)| if a > 0.0 { a * self.dom.lo[j] } else { a * self.dom.up[j] }).sum();
            let margin = minact0 - rhs0;
            if minact0.is_finite() && rhs0.is_finite() && margin > 0.0 {
                let budget = 0.5 * margin;
                let mut cand: Vec<(f64, usize)> = coefs
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &(j, a))| {
                        let (gl, gu) = (self.dom.global_lo[j], self.dom.global_up[j]);
                        let loss = if a > 0.0 { a * (self.dom.lo[j] - gl) } else { a * (self.dom.up[j] - gu) };
                        loss.is_finite().then_some((loss.max(0.0), i))
                    })
                    .collect();
                cand.sort_by(|x, y| x.0.total_cmp(&y.0));
                let mut used = 0.0;
                let mut drop = vec![false; coefs.len()];
                let mut kept = coefs.len();
                for (loss, i) in cand {
                    if used + loss > budget || kept <= 1 {
                        break;
                    }
                    let (j, a) = coefs[i];
                    let m = if a > 0.0 { a * self.dom.global_lo[j] } else { a * self.dom.global_up[j] };
                    used += loss;
                    konst += m;
                    drop[i] = true;
                    kept -= 1;
                }
                let mut i = 0;
                coefs.retain(|_| {
                    let keep = !drop[i];
                    i += 1;
                    keep
                });
            }
        }
        let dbg = env_str!("ENOMOTO_MIP_DEBUG_PROOF").is_some();
        if coefs.is_empty() {
            return false;
        }
        let dense = coefs.len() > tunable!("ENOMOTO_T_MIP_PROOF_DENSITY", 0.5, f64).mul_add(p.n as f64, 20.0) as usize;
        if dense && env_str!("ENOMOTO_MIP_DENSE_PROOF_NO_CONFLICT").is_some() {
            if dbg {
                eprintln!("PROOF reject dense {} of {}", coefs.len(), p.n);
            }
            return false;
        }
        // 今のノードで破れていることを確かめる (数値誤差で破れていない証明は使わない)
        let rhs = if obj { self.prune_limit() - p.offset - konst } else { -konst };
        let minact: f64 = coefs.iter().map(|&(j, a)| if a > 0.0 { a * self.dom.lo[j] } else { a * self.dom.up[j] }).sum();
        if minact.is_finite() && minact > rhs + 1e-6 * (1.0 + rhs.abs()) && minact - rhs <= num_err {
            // 破れているが、数値誤差の見積もりの範囲内: 証明として信用しない
            self.proof_numeric_rejects += 1;
            if dbg || self.params.verbose && env_str!("ENOMOTO_MIP_PROOF_NUM_LOG").is_some() {
                let ymax = y.iter().take(self.lp.num_rows()).fold(0.0f64, |m, v| m.max(v.abs()));
                eprintln!("PROOF reject numerically unreliable: minact - rhs {:.3e} <= error estimate {num_err:.3e} (max |y| {ymax:.3e}, dmax {dmax:.3e}, konst {konst:.3e}, obj {obj}, len {})", minact - rhs, coefs.len());
            }
            return false;
        }
        if !minact.is_finite() || minact <= rhs + 1e-6 * (1.0 + rhs.abs()) {
            if dbg {
                eprintln!("PROOF reject not violated: minact {minact} rhs {rhs} obj {obj} lp_obj {} limit {} len {} rows_skipped {}", self.lp.objective() + p.offset, self.prune_limit(), coefs.len(), skipped);
                for &(j, a) in &coefs {
                    let b = if a > 0.0 { self.dom.lo[j] } else { self.dom.up[j] };
                    if !b.is_finite() {
                        eprintln!("PROOF   inf col {j} coef {a:e} int {} dmax {dmax:e} lp lo/up {:?}", p.is_int[j], self.lp.col_bounds(j));
                    }
                }
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
                        // 破れの分解: 破れ = (目的値 - U) - sum_i y_i (A_i x - b_i) + sum_{移した列 j} d_j (g_j - x_j)。正しい行なら
                        // y_i (A_i x - b_i) >= 0、正しい大域的な境界なら d_j (g_j - x_j) <= 0 なので、破れの原因になる行・列を出す
                        let ymax = y.iter().take(self.lp.num_rows()).fold(0.0f64, |m, v| m.max(v.abs()));
                        let mut bad_rows: Vec<(f64, usize, f64, f64, f64, f64)> = Vec::new();
                        let mut sum_rows = 0.0f64;
                        for (i, &yi) in y.iter().enumerate().take(self.lp.num_rows()) {
                            if yi.abs() <= 1e-12 {
                                continue;
                            }
                            let (lo, up) = self.lp.row_bounds(i);
                            let b = if yi > 0.0 { lo } else { up };
                            if !b.is_finite() {
                                continue;
                            }
                            let ax: f64 = self.lp.row(i).iter().map(|&(j, a)| a * x[j]).sum();
                            let t = yi * (ax - b);
                            sum_rows += t;
                            if t < -1e-7 * (1.0 + b.abs()) {
                                bad_rows.push((t, i, yi, ax, lo, up));
                            }
                        }
                        let mut kept = vec![false; p.n];
                        for &(j, _) in &coefs {
                            kept[j] = true;
                        }
                        let mut bad_cols: Vec<(f64, usize, f64, f64, f64, f64)> = Vec::new();
                        let mut sum_cols = 0.0f64;
                        for (j, &dj) in d.iter().enumerate() {
                            if dj == 0.0 || kept[j] {
                                continue;
                            }
                            let g = if dj > 0.0 { self.dom.global_lo[j] } else { self.dom.global_up[j] };
                            if !g.is_finite() {
                                continue;
                            }
                            let t = dj * (g - x[j]);
                            sum_cols += t;
                            if t > 1e-7 * (1.0 + g.abs()) {
                                bad_cols.push((t, j, dj, x[j], self.dom.global_lo[j], self.dom.global_up[j]));
                            }
                        }
                        bad_rows.sort_by(|a, b| a.0.total_cmp(&b.0));
                        bad_cols.sort_by(|a, b| b.0.total_cmp(&a.0));
                        eprintln!(
                            "MIP_DEBUG_SOL: proof decomposition at node {} thread {}: debug obj - U = {:.6e}, -sum_i y_i (A_i x - b_i) = {:.6e}, sum_moved d_j (g_j - x_j) = {:.6e}; max |y| {ymax:.3e}, lp rows {} (orig {}), lp obj {:.10e}, proof len {} dmax {dmax:.3e}; rows the debug solution violates (weighted): {}, moved columns outside the global bounds: {}",
                            self.nodes, self.pool_tid, self.p.objective(x) - self.prune_limit(), -sum_rows, sum_cols, self.lp.num_rows(), p.m, self.lp.objective() + p.offset, coefs.len(), bad_rows.len(), bad_cols.len()
                        );
                        for &(t, i, yi, ax, lo, up) in bad_rows.iter().take(8) {
                            let row = self.lp.row(i);
                            let terms: Vec<String> = row.iter().take(6).map(|&(j, a)| format!("{a:+.4}*x{j}[{:.4}]", x[j])).collect();
                            eprintln!("MIP_DEBUG_SOL:   row {i} ({}): y {yi:.6e}, {lo} <= activity {ax:.10e} <= {up}, contribution {t:.6e}, len {}: {}{}", if i < p.m { "original" } else { "cut" }, row.len(), terms.join(" "), if row.len() > 6 { " ..." } else { "" });
                        }
                        for &(t, j, dj, xj, gl, gu) in bad_cols.iter().take(8) {
                            eprintln!("MIP_DEBUG_SOL:   moved col {j}: d {dj:.6e}, x {xj}, global [{gl}, {gu}], node [{}, {}], contribution {t:.6e}", self.dom.lo[j], self.dom.up[j]);
                        }
                    }
                }
            }
        });
        // 証明を作った境界の変更を分枝の決定まで辿った短い衝突も作る (証明そのものは長いことが多い)
        self.add_proof_conflict(&coefs, rhs);
        // 密な証明はプールに入れない (伝播が重い)。衝突だけ使う
        if dense {
            if dbg {
                eprintln!("PROOF dense {} of {}: conflict only", coefs.len(), p.n);
            }
            return false;
        }
        if proof_verify() {
            let m0 = p.m;
            let ids: Vec<u64> = y
                .iter()
                .enumerate()
                .take(self.lp.num_rows())
                .filter(|&(i, &yi)| {
                    if i < m0 || yi.abs() <= 1e-12 {
                        return false;
                    }
                    let (lo, up) = self.lp.row_bounds(i);
                    (if yi > 0.0 { lo } else { up }).is_finite()
                })
                .filter_map(|(i, _)| self.cut_ids.get(i - m0).copied())
                .collect();
            self.proof_pending_cuts = Some(ids);
        }
        if dbg {
            eprintln!("PROOF ok len {} int {} obj {obj}", coefs.len(), coefs.iter().filter(|&&(j, _)| p.is_int[j]).count());
        }
        if proof_cmir() {
            self.add_proof_cmir(&coefs, rhs);
        }
        if obj {
            self.push_pool_row(coefs, konst, true);
        } else {
            self.farkas_added += 1;
            self.push_pool_row(coefs, -konst, false);
        }
        self.proof_pending_cuts = None;
        true
    }

    /// `ENOMOTO_MIP_PROOF_CMIR`: 破れている証明 `coefs x <= rhs` を HiGHS の `HighsCutGeneration::generateConflict` と同じく
    /// CMIR で強めたカットを、右辺を固定した行として証明のプールに足す (証明そのものも残す)。境界は大域的なものを使い、
    /// 丸めの基準の点はこのノードの最小活動量の点 (大域の境界からの距離) を証明の超平面まで縮めたもの (HiGHS と同じ)。
    /// 目的の証明から作ったカットは今の打ち切り値で正しく、打ち切り値は下がるだけなので以後も正しい
    fn add_proof_cmir(&mut self, coefs: &[(usize, f64)], rhs: f64) {
        let p = self.p;
        let (gl, gu) = (&self.dom.global_lo, &self.dom.global_up);
        let mut x = self.proof_cmir_x.take().unwrap_or_default();
        x.resize(p.n, 0.0);
        let mut act = 0.0;
        let mut r = rhs;
        for &(j, a) in coefs {
            if a > 0.0 {
                if !gl[j].is_finite() {
                    self.proof_cmir_x = Some(x);
                    return;
                }
                r -= a * gl[j];
                act += a * (self.dom.lo[j].max(gl[j]) - gl[j]);
            } else {
                if !gu[j].is_finite() {
                    self.proof_cmir_x = Some(x);
                    return;
                }
                r -= a * gu[j];
                act += -a * (gu[j] - self.dom.up[j].min(gu[j]));
            }
        }
        let sc = if act > r && act > 0.0 && r > 0.0 { r / act } else { 1.0 };
        for &(j, a) in coefs {
            x[j] = if a > 0.0 { gl[j] + sc * (self.dom.lo[j].max(gl[j]) - gl[j]) } else { gu[j] - sc * (gu[j] - self.dom.up[j].min(gu[j])) };
        }
        let vars = super::cuts::CutVars { lo: gl, up: gu, is_int: &p.is_int, x: &x, vb: None };
        let cut = super::cuts::cmir(&vars, coefs, rhs);
        self.proof_cmir_x = Some(x);
        self.proof_cmir_stats[0] += 1;
        let Some(c) = cut else { return };
        if c.coefs.is_empty() || !c.rhs.is_finite() {
            return;
        }
        // 診断用: デバッグ解を切っていないか
        DEBUG_SOL.with(|dd| {
            if let Some(xd) = dd.borrow().as_ref() {
                if self.p.objective(xd) < self.prune_limit() - 1e-6 {
                    let a: f64 = c.coefs.iter().map(|&(j, v)| v * xd[j]).sum();
                    if a > c.rhs + 1e-6 * (1.0 + c.rhs.abs()) {
                        eprintln!("MIP_DEBUG_SOL: proof CMIR cuts off the debug solution ({a} > {})", c.rhs);
                    }
                }
            }
        });
        if env_str!("ENOMOTO_MIP_DEBUG_PROOF").is_some() {
            eprintln!("PROOF cmir len {} -> {}", coefs.len(), c.coefs.len());
            if env_str!("ENOMOTO_MIP_DEBUG_PROOF") == Some("2") {
                eprintln!("PROOF src {:?} <= {rhs}", coefs);
                eprintln!("PROOF cut {:?} <= {}", c.coefs, c.rhs);
            }
        }
        self.proof_cmir_stats[1] += 1;
        // `ENOMOTO_MIP_PROOF_CMIR_POOL`: カットのプールにも入れ、待ち行列から取り出したノードで破れていれば LP に戻す
        // (HiGHS は衝突カットをカットプールに入れ、ノードの分離のラウンドで LP に加える)
        // 既定は `nocap` (プールの非零数の上限を見ずに入れる。根の密なカットで上限に達していると、短い衝突カットが 1 本も
        // 入らない。neos-2978193-inde)。`ENOMOTO_MIP_PROOF_CMIR_POOL=off` で入れない、それ以外の値で上限を見て入れる
        let pool_mode = env_str!("ENOMOTO_MIP_PROOF_CMIR_POOL").unwrap_or("nocap");
        if pool_mode != "off" && !self.params.submip {
            self.pool_nocap = pool_mode == "nocap";
            let n0 = self.cut_pool.len();
            self.add_to_pool(&c.coefs, c.rhs);
            self.pool_nocap = false;
            self.proof_cmir_stats[2] += (self.cut_pool.len() > n0) as u64;
        }
        self.push_pool_row(c.coefs, c.rhs, false);
    }

    /// 分類 `cat` の証明を 1 本捨てる: 古いものから。木を分け合うとき (`ENOMOTO_MIP_PROOF_NO_AGING` でなければ)、前回から使われた印のあるものは印を
    /// 消して後ろに回す (clock 法)。
    fn proof_evict(&mut self, cat: usize) {
        let aging = self.pool.is_some() && env_str!("ENOMOTO_MIP_PROOF_NO_AGING").is_none();
        let mut turns = self.proof_fifo[cat].len();
        while let Some(id) = self.proof_fifo[cat].pop_front() {
            let k = (id - self.proof_first_id) as usize;
            if aging && turns > 0 && self.proof_ref[k] {
                self.proof_ref[k] = false;
                self.proof_fifo[cat].push_back(id);
                turns -= 1;
                continue;
            }
            self.proof_kill(id);
            return;
        }
    }

    /// 通し番号 `id` の証明を捨てる: 中身を空にして右辺を +inf にする (調べる処理は右辺が無限の行を飛ばす)。列の索引に
    /// 残った項目は、索引を作り直すまで差分を足すだけで害はない。
    fn proof_kill(&mut self, id: u64) {
        let k = (id - self.proof_first_id) as usize;
        let c = std::mem::take(&mut self.dual_proofs[k].0);
        self.dual_proof_nnz -= c.len();
        self.dual_proofs[k].1 = f64::INFINITY;
        self.dual_proofs[k].2 = false;
        self.proof_state[k] = ProofState::Dead;
        self.proof_keys[k] = Vec::new();
        self.proof_cuts[k] = None;
        self.proof_dead += 1;
    }

    /// 捨てた証明を取り除いて詰め直す。通し番号は付け直す (古い番号より大きい番号から) ので、列の索引・基準は次の
    /// [`Self::apply_dual_proofs`] で作り直す。
    fn proof_compact(&mut self) {
        let old_first = self.proof_first_id;
        let new_first = old_first + self.dual_proofs.len() as u64;
        let mut map = std::collections::HashMap::new();
        let (mut dp, mut base, mut st, mut keys) = (std::collections::VecDeque::new(), std::collections::VecDeque::new(), std::collections::VecDeque::new(), std::collections::VecDeque::new());
        let mut refs = std::collections::VecDeque::new();
        let mut cuts = std::collections::VecDeque::new();
        let it = std::mem::take(&mut self.dual_proofs).into_iter().zip(std::mem::take(&mut self.proof_base)).zip(std::mem::take(&mut self.proof_state)).zip(std::mem::take(&mut self.proof_keys)).zip(std::mem::take(&mut self.proof_ref)).zip(std::mem::take(&mut self.proof_cuts));
        for (k, (((((row, b), s), ky), rf), pc)) in it.enumerate() {
            if s != ProofState::Dead {
                map.insert(old_first + k as u64, new_first + dp.len() as u64);
                dp.push_back(row);
                base.push_back(b);
                st.push_back(s);
                keys.push_back(ky);
                refs.push_back(rf);
                cuts.push_back(pc);
            }
        }
        self.proof_cuts = cuts;
        self.proof_ref = refs;
        self.dual_proofs = dp;
        self.proof_base = base;
        self.proof_state = st;
        self.proof_keys = keys;
        for f in self.proof_fifo.iter_mut() {
            for id in f.iter_mut() {
                *id = map[id];
            }
        }
        self.proof_first_id = new_first;
        self.proof_dead = 0;
        self.proof_always.clear();
        self.proof_snap = None;
        self.proof_dirty = true;
    }

    /// `k` 番目の証明が刈り込み・締め付けに使われた: 自前でまだ知らせていなければ、ほかのスレッドに知らせる
    /// (長さが `ENOMOTO_T_MIP_TREE_SHARE_PROOF_LEN` 以下のものだけ)。
    fn proof_used(&mut self, k: usize) {
        self.proof_ref[k] = true;
        let ProofState::Own(uses) = self.proof_state[k] else { return };
        let uses = uses.saturating_add(1);
        if (uses as u64) < tunable!("ENOMOTO_T_MIP_TREE_SHARE_PROOF_USES", 3u64, u64) {
            self.proof_state[k] = ProofState::Own(uses);
            return;
        }
        self.proof_state[k] = ProofState::OwnDone;
        let Some(pool) = self.pool.clone() else { return };
        if env_str!("ENOMOTO_MIP_TREE_NO_SHARED_PROOFS").is_some() {
            return;
        }
        let (c, konst, obj) = &self.dual_proofs[k];
        // 長さの上限: 絶対 (`ENOMOTO_T_MIP_TREE_SHARE_PROOF_LEN`) と列数に対する割合 (`ENOMOTO_T_MIP_TREE_SHARE_LEN_FRAC`、0 なら無制限)
        let frac = tunable!("ENOMOTO_T_MIP_TREE_SHARE_LEN_FRAC", 0.0, f64);
        if c.len() > tunable!("ENOMOTO_T_MIP_TREE_SHARE_PROOF_LEN", usize::MAX, usize) || (frac > 0.0 && c.len() as f64 > frac * self.p.n as f64) {
            return;
        }
        learned_push(&pool, self.pool_tid, LearnedItem::Proof(c.clone(), *konst, *obj));
        self.tprof.shared += 1;
    }

    /// 双対証明・衝突制約のプールに行 `sum coefs x <= rhs` を加える (`obj` なら `rhs = U - konst`、そうでなければ
    /// `rhs = konst`)。上限を超えたら古いものから捨てる。
    pub(super) fn push_pool_row(&mut self, coefs: Vec<(usize, f64)>, konst: f64, obj: bool) {
        // 係数を締め付けの上界 |a_j| (u_j - l_j) (大域的な境界) の大きい順に並べる
        let (coefs, keys) = {
            let (glo, gup) = (&self.dom.global_lo, &self.dom.global_up);
            let mut ck: Vec<((usize, f64), f64)> = coefs.into_iter().map(|(j, a)| ((j, a), if self.p.is_int[j] { a.abs() * (gup[j] - glo[j]) } else { 0.0 })).collect();
            ck.sort_by(|x, y| y.1.total_cmp(&x.1));
            ck.into_iter().unzip::<_, _, Vec<_>, Vec<_>>()
        };
        // 木を分け合う: 自前の証明は、このスレッドで初めて刈り込み・締め付けに使われたときにほかのスレッドに知らせる
        // ([`Self::proof_used`])。`ENOMOTO_MIP_TREE_SHARE_PROOFS_AT_ONCE` なら以前と同じく作ったときに知らせる
        let mut state = if self.importing { ProofState::Imported } else { ProofState::Own(0) };
        if state == ProofState::Own(0) && env_str!("ENOMOTO_MIP_TREE_SHARE_PROOFS_AT_ONCE").is_some() {
            if let Some(pool) = &self.pool {
                if env_str!("ENOMOTO_MIP_TREE_NO_SHARED_PROOFS").is_none() {
                    learned_push(pool, self.pool_tid, LearnedItem::Proof(coefs.clone(), konst, obj));
                }
            }
            state = ProofState::OwnDone;
        }
        let p = self.p;
        // 診断用 (デバッグ解): プールに入る行 (自前・取り込み) がデバッグ解を切っていないか (目的の証明は、デバッグ解が打ち切り値より
        // 良いときだけ)
        if self.dbg_on {
            DEBUG_SOL.with(|d| {
                if let Some(x) = d.borrow().as_ref() {
                    let dz = p.objective(x);
                    let rhs = if obj { self.prune_limit() - p.offset - konst } else { konst };
                    if (!obj || dz < self.prune_limit() - 1e-6) && rhs.is_finite() {
                        let act: f64 = coefs.iter().map(|&(j, a)| a * x[j]).sum();
                        if act > rhs + 1e-6 * (1.0 + rhs.abs()) {
                            eprintln!("MIP_DEBUG_SOL: node {} thread {}: pool row ({state:?}, obj {obj}, len {}, konst {konst:.10e}) is violated by the debug solution: activity {act:.10e} > rhs {rhs:.10e} (debug obj {dz}, prune limit {})", self.nodes, self.pool_tid, coefs.len(), self.prune_limit());
                        }
                    }
                }
            });
        }
        let nnz_cap = 10 * p.nnz + 100_000;
        // 上限は自前と取り込みで別 (取り込みが自前の証明を押し出さないように)。取り込みの上限
        // `ENOMOTO_T_MIP_PROOF_POOL_IMPORTED`
        let cat = (state == ProofState::Imported) as usize;
        let cap = if cat == 0 { tunable!("ENOMOTO_T_MIP_PROOF_POOL", 2000usize, usize) } else { tunable!("ENOMOTO_T_MIP_PROOF_POOL_IMPORTED", 500usize, usize) };
        // 木を分け合うとき、`ENOMOTO_T_MIP_PROOF_POOL_PER_COL` = c (> 0、既定 4): 上限を max(200, c * 列数) 以下にする (小さい問題で証明のプールが問題より
        // ずっと大きくなり、ノードごとの伝播が重くなるのを避ける)
        let per_col = tunable!("ENOMOTO_T_MIP_PROOF_POOL_PER_COL", 4usize, usize);
        let cap = if per_col > 0 && self.pool.is_some() { cap.min((per_col * self.p.n).max(200)) } else { cap };
        if cap == 0 {
            return;
        }
        while self.proof_fifo[cat].len() >= cap {
            self.proof_evict(cat);
        }
        // 非零の上限: 取り込みから、なければ自前の古いものから捨てる
        while self.dual_proof_nnz + coefs.len() > nnz_cap {
            let c = if !self.proof_fifo[1].is_empty() { 1 } else if !self.proof_fifo[0].is_empty() { 0 } else { break };
            self.proof_evict(c);
        }
        // 捨てたものが多ければ詰め直す (先頭から取り除くと証明の番号がずれて差分を作り直すことになるので、まとめて行う)
        if self.proof_dead > self.dual_proofs.len() / 2 + 256 {
            self.proof_compact();
        }
        self.dual_proof_nnz += coefs.len();
        let id = self.proof_first_id + self.dual_proofs.len() as u64;
        if self.proof_col_index[0].len() != p.n {
            self.proof_col_index = [vec![Vec::new(); p.n], vec![Vec::new(); p.n]];
        }
        for &(j, a) in &coefs {
            self.proof_col_index[(a <= 0.0) as usize][j].push((id, a));
        }
        let base = match &self.proof_snap {
            Some((lo, up)) => proof_min_activity(&coefs, lo, up),
            None => (0.0, 0, f64::INFINITY),
        };
        self.proof_base.push_back(base);
        self.proof_state.push_back(state);
        self.proof_fifo[cat].push_back(id);
        // 差分が今の基準で揃っていれば、この証明の差分だけを足す (全部を作り直さない)。基準の境界と、差分に反映済みの
        // 境界 (`proof_val_*`) の違いを足す ([`Self::proof_absorb`] と同じ計算)
        if !self.proof_dirty && self.proof_delta.len() == self.dual_proofs.len() && self.proof_val_lo.len() == p.n {
            let (slo, sup) = self.proof_snap.as_ref().unwrap();
            let mut e = (0.0f64, 0i32);
            for &(j, a) in &coefs {
                let (s0, v) = if a > 0.0 { (slo[j], self.proof_val_lo[j]) } else { (sup[j], self.proof_val_up[j]) };
                if s0 == v {
                    continue;
                }
                let (old, new) = (a * s0, a * v);
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
            let k = self.dual_proofs.len();
            self.proof_delta.push(e);
            self.proof_in_touched.push(false);
            if e != (0.0, 0) {
                self.proof_in_touched[k] = true;
                self.proof_touched.push(k);
            }
        } else {
            self.proof_dirty = true;
        }
        self.proof_keys.push_back(keys);
        self.proof_cuts.push_back(if state == ProofState::Imported { None } else { self.proof_pending_cuts.take() });
        self.proof_ref.push_back(false);
        self.dual_proofs.push_back((coefs, konst, obj));
        let k = self.dual_proofs.len() - 1;
        if self.proof_snap.is_some() && self.proof_root_tight(base, self.proof_rhs(k)) {
            self.proof_always.push(id);
        }
    }

    /// 直前の伝播の矛盾から衝突制約を作ってプールに入れる (定義域を巻き戻す前に呼ぶ)。衝突が 2 値列の固定だけで
    /// できていれば `sum_{x_j=1 の固定} x_j - sum_{x_j=0 の固定} x_j <= |{x_j=1}| - 1` (どれか 1 つは逆の値)。
    pub(super) fn add_conflict(&mut self) {
        if self.params.submip && env_str!("ENOMOTO_MIP_SUBMIP_CONFLICTS").is_none() && env_str!("ENOMOTO_MIP_SUBMIP_PROOFS").is_none() && !super::submip_highs('f') || env_str!("ENOMOTO_MIP_NO_CONFLICTS").is_some() {
            return;
        }
        let p = self.p;
        self.conf_stats[0] += 1;
        if let Some(src) = self.dom.conflict {
            if self.add_conflicts_multi(super::domain::ConflictStart::Src(src)) > 0 {
                return;
            }
        }
        let max_len = self.conflict_max_len();
        let first = self.dom.analyze_conflict(p, max_len);
        let ok = match &first {
            None => {
                self.conf_stats[1] += 1;
                false
            }
            Some(lits) => {
                let ok = self.add_conflict_lits(lits);
                if !ok {
                    self.conf_stats[2] += 1;
                }
                ok
            }
        };
        if !ok && env_str!("ENOMOTO_MIP_NO_DECISION_CONFLICTS").is_none() {
            // 0-1 列以外の境界を含む (または 1-UIP が長すぎる): 決定だけの組にして試す
            match self.dom.analyze_conflict_decisions(p, max_len) {
                None => self.conf_stats[3] += 1,
                Some(lits) => {
                    if !self.add_conflict_lits(&lits) {
                        self.conf_stats[4] += 1;
                    }
                }
            }
        }
    }

    /// 衝突解析の上限の長さ。
    /// HiGHS 式の衝突解析での衝突の長さの上限 (HiGHS: 10 長さ > 1000 + 3 整数列数 なら捨てる)。
    fn conflict_max_len_multi(&self) -> usize {
        let nint = self.p.nint;
        (tunable!("ENOMOTO_T_MIP_CONFLICT_LEN_INT", 0.3, f64).mul_add(nint as f64, tunable!("ENOMOTO_T_MIP_CONFLICT_LEN_BASE", 100.0, f64)) as usize).max(1)
    }

    /// HiGHS 式の衝突解析で衝突を作って入れる (入れた数)。`ENOMOTO_MIP_CONFLICT_OLD` なら使わない (0)。
    fn add_conflicts_multi(&mut self, start: super::domain::ConflictStart) -> usize {
        if env_str!("ENOMOTO_MIP_CONFLICT_OLD").is_some() {
            return 0;
        }
        // 解析の時間が経過時間の 1 割を超えたら休む
        if self.multi_secs > tunable!("ENOMOTO_T_MIP_CONFLICT_TIME_FRAC", 0.1, f64) * self.start.elapsed().as_secs_f64() + 0.05 {
            self.multi_stats[0] += 1;
            return 1;
        }
        let max_len = self.conflict_max_len_multi();
        let reconv = env_str!("ENOMOTO_MIP_NO_RECONV_CONFLICTS").is_none();
        let t0 = Instant::now();
        // `ENOMOTO_MIP_HIGHS_CONFLICT_SCORE`: 分枝の衝突スコアを HiGHS と同じく、解析で説明に使った境界の変更全部の列で
        // 上げる (作れた衝突のリテラルでは上げない)
        let hcs = highs_conflict_score();
        let mut bumps: Vec<(usize, bool)> = Vec::new();
        let confs = self.dom.analyze_multi_bumps(self.p, start, max_len, reconv, hcs.then_some(&mut bumps));
        if hcs && !bumps.is_empty() {
            self.pc.bump_conflict(&bumps);
        }
        self.multi_secs += t0.elapsed().as_secs_f64();
        let mut n = 0;
        // 既定ではリテラルの衝突プールに入れる (行にして証明のプールに入れると、数が多いので双対証明を追い出す)
        let rows = env_str!("ENOMOTO_MIP_MULTI_CONFLICT_ROWS").is_some();
        for c in &confs {
            let ok = if rows {
                self.add_conflict_lits(c)
            } else {
                if !hcs {
                    self.pc.add_conflict(c);
                }
                self.add_lit_conflict(c)
            };
            if ok {
                n += 1;
            }
        }
        self.multi_stats[0] += 1;
        self.multi_stats[1] += confs.len() as u64;
        self.multi_stats[2] += n as u64;
        self.multi_stats[3] += confs.iter().map(|c| c.len() as u64).sum::<u64>();
        // 何も作れなければ従来の解析も試す (呼び出し側)
        if confs.is_empty() { 0 } else { n.max(1) }
    }

    fn conflict_max_len(&self) -> usize {
        // 長い衝突は弱いわりに評価が重い (eil33-2: 上限 460 で 678 本作ると 1 ノードの処理が重くなり解けなくなった)
        (tunable!("ENOMOTO_T_MIP_CONFLICT_LEN", 0.1, f64).mul_add(self.p.n as f64, 10.0) as usize).min(tunable!("ENOMOTO_T_MIP_CONFLICT_MAXLEN", 50usize, usize))
    }

    /// 破れている証明 (`coefs`、今のノードの境界で最小活動量が右辺を超える) から衝突を作ってプールに入れる。
    fn add_proof_conflict(&mut self, coefs: &[(usize, f64)], rhs: f64) {
        if env_str!("ENOMOTO_MIP_NO_PROOF_CONFLICTS").is_some() {
            return;
        }
        if self.add_conflicts_multi(super::domain::ConflictStart::Proof(coefs, rhs)) > 0 {
            self.proof_conflicts += 1;
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

    /// 打ち切り値つきの根のプロービング (既定で使う。`ENOMOTO_MIP_NO_ROOT_OBJ_PROBE` で使わない): 暫定解があるとき、
    /// 根の LP 解で分数値の 0-1 列 x_j ごとに x_j = 0 と x_j = 1 を試し、行の伝播と目的関数の伝播 (`目的値 <= 打ち切り値`)
    /// で矛盾する側があれば、逆の値に大域的に固定する (打ち切り値より良い解はその側にない)。固定はすぐ定義域に入れるので、
    /// 後の列の試しにも効く。HiGHS は根の分離の 1 回目 (`HighsImplications::separateImpliedBounds`) で、まだ試していない
    /// 分数値の 0-1 列を全てプロービングする (このときの定義域の伝播は打ち切り値つきの目的関数の伝播を含む)。
    /// hypothyroid-k1 では HiGHS は暫定解 -2692 (打ち切り -2693) で 2153 列を試して 968 列を固定し、LP 値が
    /// -2902.85 -> -2851.08 になって根で解き終わる。こちらは前処理のプロービング (打ち切り値なし) しかなく、木の根の強分枝に
    /// 時間を使い切っていた。固定が出れば根の再スタート ([`Self::maybe_restart`]) が固定を入れて前処理からやり直す。
    /// 時間は時間上限の `ENOMOTO_T_MIP_ROOT_OBJ_PROBE_TIME_FRAC` 倍 (既定 0.2) まで。最初の
    /// `ENOMOTO_T_MIP_ROOT_OBJ_PROBE_GIVEUP` 列 (既定 200) で 1 つも固定できなければやめる。
    /// [`Self::root_obj_probe`] の 1 回の試し: `x_j <= v` (`upper`) か `x_j >= v` を置いて、行・目的関数・被約費用の行・
    /// カットの行で伝播し、矛盾したら真。定義域は元に戻す。
    fn probe_bound_infeasible(&mut self, j: usize, upper: bool, v: f64, rc_row: Option<&(Vec<(usize, f64)>, f64)>, cut_rows: &[(Vec<(usize, f64)>, f64)]) -> bool {
        let p = self.p;
        let pos = self.dom.stack_len();
        if upper {
            self.dom.tighten_upper(p, j, v);
        } else {
            self.dom.tighten_lower(p, j, v);
        }
        let ok = !self.dom.infeasible
            && self.dom.propagate(p)
            && self.propagate_objective_with(false)
            && rc_row.is_none_or(|(r, u)| self.propagate_le_row(r, *u, false))
            && cut_rows.iter().all(|(r, u)| self.propagate_le_row(r, *u, false));
        self.dom.backtrack_to(p, pos);
        !ok
    }

    /// どちらの値でも矛盾する列が見つかった (打ち切り値より良い解がない) なら真。
    fn root_obj_probe(&mut self) -> bool {
        let p = self.p;
        let sub_cutoff = self.params.submip && self.params.cutoff.is_finite();
        if !(self.has_cutoff_value() || sub_cutoff) || !self.prune_limit().is_finite() || self.dom.infeasible || self.time_up() {
            return false;
        }
        let t0 = Instant::now();
        let tmax = tunable!("ENOMOTO_T_MIP_ROOT_OBJ_PROBE_TIME_FRAC", 0.2, f64) * if self.params.time_limit.is_finite() { self.params.time_limit } else { 300.0 };
        let giveup = tunable!("ENOMOTO_T_MIP_ROOT_OBJ_PROBE_GIVEUP", 200usize, usize);
        let x = self.lp.col_values();
        let cands: Vec<usize> = (0..p.n)
            .filter(|&j| p.is_int[j] && self.dom.lo[j] == 0.0 && self.dom.up[j] == 1.0 && x[j] > FEASTOL && x[j] < 1.0 - FEASTOL)
            .collect();
        // 効きそうなときだけ回す: 暫定解 (打ち切り値) と根の LP 値の相対ギャップが `ENOMOTO_T_MIP_ROOT_OBJ_PROBE_MAX_GAP`
        // 以下 (既定 0.05) で、候補 (分数値の 0-1 列) が `ENOMOTO_T_MIP_ROOT_OBJ_PROBE_MIN_CANDS` 本以上 (既定 500)
        // のときだけ。ギャップ = (打ち切り値 - LP 値) / max(|打ち切り値|, |LP 値|, 1)
        let lp_z = self.lp.objective() + p.offset;
        let cut_z = self.incumbent.as_ref().map_or(self.prune_limit(), |(z, _)| *z);
        let rel_gap = (cut_z - lp_z).max(0.0) / cut_z.abs().max(lp_z.abs()).max(1.0);
        let max_gap = tunable!("ENOMOTO_T_MIP_ROOT_OBJ_PROBE_MAX_GAP", 0.05, f64);
        let min_cands = tunable!("ENOMOTO_T_MIP_ROOT_OBJ_PROBE_MIN_CANDS", 500usize, usize);
        let log = self.params.verbose || (self.params.submip && env_str!("ENOMOTO_MIP_ROOT_OBJ_PROBE_LOG").is_some());
        if rel_gap > max_gap || cands.len() < min_cands {
            if log {
                eprintln!(
                    "MIP: {}root objective probing: skipped (gap {:.3e} (LP {lp_z:.10e}, incumbent {cut_z:.10e}), {} fractional binaries)",
                    if self.params.submip { "sub-MIP " } else { "" },
                    rel_gap,
                    cands.len()
                );
            }
            return false;
        }
        let (mut probed, mut fixed, mut both) = (0usize, 0usize, false);
        let learn = env_str!("ENOMOTO_MIP_ROOT_OBJ_PROBE_NO_IMPL").is_none();
        // `ENOMOTO_MIP_ROOT_OBJ_PROBE_REDCOST`: 根の LP の被約費用の行 `z + sum d_j (x_j - b_j) <= U` (b_j は LP 解で列が
        // いた境界。双対の実行可能性から、LP の行 (カットを含む) を満たす全ての x で z(x) >= z + sum d_j (x_j - b_j)) も
        // 伝播する。被約費用による固定は列 1 本ずつだが、この行は試しで動いた列の分を足し合わせるので、LP の情報を持った
        // 打ち切りになる
        let rc_row: Option<(Vec<(usize, f64)>, f64)> = (env_str!("ENOMOTO_MIP_ROOT_OBJ_PROBE_REDCOST").is_some())
            .then(|| self.root_redcost.as_ref().map(|(z, list)| {
                let u = self.prune_limit() - z;
                let row: Vec<(usize, f64)> = list.iter().map(|&(j, _, d, _)| (j, d)).collect();
                let rhs = u + list.iter().map(|&(_, _, d, b)| d * b).sum::<f64>();
                (row, rhs)
            }))
            .flatten()
            .filter(|(_, u)| u.is_finite());
        // カットの行 (既定で使う、`ENOMOTO_MIP_ROOT_OBJ_PROBE_NO_CUTS` で使わない): LP に入っているカットの行も伝播する (定義域の伝播は元の行だけなので、
        // 再スタートでカットが行になるまで効かなかった。HiGHS はカットプールのカットも伝播する)。行ごとに 1 回ずつ
        let mut cut_rows: Vec<(Vec<(usize, f64)>, f64)> = Vec::new();
        if env_str!("ENOMOTO_MIP_ROOT_OBJ_PROBE_NO_CUTS").is_none() {
            for i in p.m..self.lp.num_rows() {
                let (l, u) = self.lp.row_bounds(i);
                let r = self.lp.row(i);
                if r.iter().any(|&(j, _)| j >= p.n) {
                    continue;
                }
                if u.is_finite() {
                    cut_rows.push((r.clone(), u));
                }
                if l.is_finite() {
                    cut_rows.push((r.iter().map(|&(j, v)| (j, -v)).collect(), -l));
                }
            }
        }
        let mut new_edges: Vec<(u32, u32)> = Vec::new();
        let mut n_edges = 0usize;
        // `ENOMOTO_T_MIP_ROOT_OBJ_PROBE_PASSES` (試作、既定 1): 候補を一回りして、固定が出たのに証明に届かなければ、同じ候補を
        // もう一回り試す (前の回りの固定と含意が後の回りの伝播に効く)。hypothyroid-k1 では根のカットの組によって、
        // 1 回りで 351〜482 列を試して両側矛盾で証明する回と、全 2300〜2440 列を試して 17〜37 列を固定するだけで終わり、
        // 再スタートして 15〜35 秒遅れる回がある
        let passes = tunable!("ENOMOTO_T_MIP_ROOT_OBJ_PROBE_PASSES", 3usize, usize).max(1);
        let mut npass = 0usize;
        for _ in 0..passes {
            npass += 1;
            let fixed0 = fixed;
            for &j in &cands {
                if self.time_up() || t0.elapsed().as_secs_f64() > tmax || (fixed == 0 && probed >= giveup) {
                    break;
                }
                if self.dom.lo[j] == self.dom.up[j] {
                    continue;
                }
                probed += 1;
                let mut bad = [false; 2];
                for (k, v) in [0.0f64, 1.0].into_iter().enumerate() {
                    let pos = self.dom.stack_len();
                    if v == 0.0 {
                        self.dom.tighten_upper(p, j, 0.0);
                    } else {
                        self.dom.tighten_lower(p, j, 1.0);
                    }
                    bad[k] = !(!self.dom.infeasible
                        && self.dom.propagate(p)
                        && self.propagate_objective_with(false)
                        && rc_row.as_ref().is_none_or(|(r, u)| self.propagate_le_row(r, *u, false))
                        && cut_rows.iter().all(|(r, u)| self.propagate_le_row(r, *u, false)));
                    // 含意 (既定で使う、`ENOMOTO_MIP_ROOT_OBJ_PROBE_NO_IMPL` で使わない): 矛盾しなかった側で固定された 0-1 列を含意 (x_j = v => x_c = w) として
                    // 定義域に足し、後の列の試しの伝播に使う (HiGHS はプロービングの含意をクリーク表に入れる)。打ち切り値を
                    // 使って得た含意なので、打ち切り値より良い解でだけ成り立つ (大域的な固定と同じ)
                    if !bad[k] && learn {
                        let lj = 2 * j as u32 + if v == 1.0 { 0 } else { 1 };
                        for (c, upper, val) in self.dom.changes_since(pos + 1) {
                            if c != j && p.is_int[c] && self.dom.lo[c] == self.dom.up[c] && (val == 0.0 || val == 1.0) && self.dom.global_lo[c] == 0.0 && self.dom.global_up[c] == 1.0 {
                                // x_c = val (上限を 0 にした / 下限を 1 にした) => 文字 (c = 1 - val) と衝突
                                let _ = upper;
                                let lc = 2 * c as u32 + if val == 1.0 { 1 } else { 0 };
                                new_edges.push((lj, lc));
                            }
                        }
                    }
                    self.dom.backtrack_to(p, pos);
                }
                if !new_edges.is_empty() {
                    new_edges.sort_unstable();
                    new_edges.dedup();
                    n_edges += new_edges.len();
                    self.dom.add_implications(p.n, &new_edges);
                    new_edges.clear();
                }
                match bad {
                    [true, true] => {
                        // どちらの値でも打ち切り値より良い解はない: 暫定解が最適。ここでは固定せず、根の LP に任せる
                        both = true;
                        break;
                    }
                    [true, false] | [false, true] => {
                        let v = if bad[0] { 1.0 } else { 0.0 };
                        self.dom.tighten_global(p, j, v == 0.0, v);
                        if v == 0.0 {
                            self.dom.tighten_upper(p, j, 0.0);
                        } else {
                            self.dom.tighten_lower(p, j, 1.0);
                        }
                        fixed += 1;
                        if !self.dom.propagate(p) {
                            both = true;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if both || fixed == fixed0 || self.time_up() || t0.elapsed().as_secs_f64() > tmax {
                break;
            }
        }
        // `ENOMOTO_MIP_ROOT_OBJ_PROBE_GENINT`: 0-1 でない有界の整数列 (幅 2 以上) でも、境界の側ごとに二分探索で試す。
        // 上の側: `x >= m` が矛盾する最小の m を探し (矛盾は m について単調とみなす)、上限を m - 1 にする。下の側も同じ。
        // 試しは片側 ceil(log2(上限 - 下限)) 回程度。下限の側 (`x >= 下限`) から矛盾すれば打ち切り値より良い解はない
        let (mut gi_probed, mut gi_tight, mut gi_probes) = (0usize, 0usize, 0usize);
        if env_str!("ENOMOTO_MIP_ROOT_OBJ_PROBE_GENINT").is_some() && !both {
            let gcands: Vec<usize> = (0..p.n)
                .filter(|&j| p.is_int[j] && self.dom.lo[j].is_finite() && self.dom.up[j].is_finite() && self.dom.up[j] - self.dom.lo[j] >= 2.0)
                .collect();
            'cols: for &j in &gcands {
                if self.time_up() || t0.elapsed().as_secs_f64() > tmax {
                    break;
                }
                gi_probed += 1;
                // upper = false: x >= v を試して上限を探す。upper = true: x <= v を試して下限を探す
                for upper in [false, true] {
                    let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
                    if up - lo < 1.0 {
                        break;
                    }
                    // 端 (x >= up / x <= lo) が矛盾しなければ締まらない
                    let edge = if upper { lo } else { up };
                    gi_probes += 1;
                    if !self.probe_bound_infeasible(j, upper, edge, rc_row.as_ref(), &cut_rows) {
                        continue;
                    }
                    // a: 矛盾しない側の値 (現在の反対の端は矛盾しないとみなす)、b: 矛盾する値
                    let (mut a, mut b) = if upper { (up, lo) } else { (lo, up) };
                    let mut whole = false;
                    // 反対の端そのものが矛盾するか (区間全体が矛盾)
                    gi_probes += 1;
                    if self.probe_bound_infeasible(j, upper, a, rc_row.as_ref(), &cut_rows) {
                        whole = true;
                    }
                    if whole {
                        both = true;
                        break 'cols;
                    }
                    while (b - a).abs() > 1.0 {
                        let m = ((a + b) / 2.0).floor();
                        let m = if m == a { a + (b - a).signum() } else { m };
                        gi_probes += 1;
                        if self.probe_bound_infeasible(j, upper, m, rc_row.as_ref(), &cut_rows) {
                            b = m;
                        } else {
                            a = m;
                        }
                        if self.time_up() {
                            break;
                        }
                    }
                    // x >= b (upper=false) が矛盾 => 上限 b - 1。x <= b (upper=true) が矛盾 => 下限 b + 1
                    if upper {
                        self.dom.tighten_global(p, j, false, b + 1.0);
                        self.dom.tighten_lower(p, j, b + 1.0);
                    } else {
                        self.dom.tighten_global(p, j, true, b - 1.0);
                        self.dom.tighten_upper(p, j, b - 1.0);
                    }
                    gi_tight += 1;
                    if !self.dom.propagate(p) {
                        both = true;
                        break 'cols;
                    }
                }
            }
            if self.params.verbose || (self.params.submip && env_str!("ENOMOTO_MIP_ROOT_OBJ_PROBE_LOG").is_some()) {
                eprintln!(
                    "MIP: {}root objective probing (general integers): {} columns, {gi_probed} probed ({gi_probes} probes), {gi_tight} bounds tightened{} ({:.2}s)",
                    if self.params.submip { "sub-MIP " } else { "" },
                    gcands.len(),
                    if both { ", infeasible" } else { "" },
                    t0.elapsed().as_secs_f64()
                );
            }
        }
        self.root_obj_probe_stats = [probed as u64, fixed as u64];
        if self.params.verbose || (self.params.submip && env_str!("ENOMOTO_MIP_ROOT_OBJ_PROBE_LOG").is_some()) {
            eprintln!(
                "MIP: {}root objective probing: gap {:.3e}, {} fractional binaries, {probed} probed ({npass} passes), {fixed} fixed, {n_edges} implications{} (cutoff {:.10e}, redcost row {}, {} cut rows, {:.2}s)",
                if self.params.submip { "sub-MIP " } else { "" },
                rel_gap,
                cands.len(),
                if both { ", both sides infeasible" } else { "" },
                self.prune_limit(),
                rc_row.as_ref().map_or(0, |(r, _)| r.len()),
                cut_rows.len(),
                t0.elapsed().as_secs_f64()
            );
        }
        both
    }

    /// 衝突 (同時には成り立たない境界の組) が 0-1 列だけなら、`sum_{x_j >= 1} x_j - sum_{x_j <= 0} x_j <= |{x_j >= 1}| - 1`
    /// の行にしてプールに入れる (入れたら真)。
    /// 目的関数の伝播 (HiGHS の objective propagation): `sum c_j x_j <= U - 定数項` (`U` は打ち切り値) を 1 本の行として
    /// 伝播する。最小活動量が右辺を超えれば矛盾 (この行を証明として衝突解析する)。そうでなければ各列の境界を
    /// 余裕の分だけ締める (締めた境界は外から来た変更として記録)。矛盾なら偽。
    pub(super) fn propagate_objective(&mut self) -> bool {
        // 既定では無効 (`ENOMOTO_MIP_OBJ_PROP` で有効): neos-911970 の下界・暫定解は良くなるが、40 問では 2 回の比較とも
        // わずかに悪化した (sgeomean +0.2〜0.3)
        if env_str!("ENOMOTO_MIP_OBJ_PROP").is_none() {
            return !self.dom.infeasible;
        }
        self.propagate_objective_with(true)
    }

    /// [`Self::propagate_objective`] の本体 (環境変数を見ない)。`conflicts` が偽なら矛盾のときに衝突を足さない
    /// (根のプロービング [`Self::root_obj_probe`] は列ごとに矛盾しうるので足さない)。
    fn propagate_objective_with(&mut self, conflicts: bool) -> bool {
        if self.dom.infeasible {
            return false;
        }
        let p = self.p;
        let u = self.prune_limit() - p.offset;
        if !u.is_finite() {
            return true;
        }
        let obj = match &self.obj_cols {
            Some(o) => o.clone(),
            None => {
                let o = std::rc::Rc::new((0..p.n).filter(|&j| p.cost[j] != 0.0).map(|j| (j, p.cost[j])).collect::<Vec<_>>());
                self.obj_cols = Some(o.clone());
                o
            }
        };
        if obj.is_empty() {
            return true;
        }
        self.propagate_le_row(&obj, u, conflicts)
    }

    /// [`prop_fixpoint`] の本体: ノードの伝播の各段を、境界の変更の記録が伸びなくなるまで回し直す。矛盾なら偽。
    fn node_prop_fixpoint(&mut self, changes: &[BoundChange]) -> bool {
        let passes = tunable!("ENOMOTO_T_MIP_PROP_FIXPOINT_PASSES", 10usize, usize);
        let mut last = usize::MAX;
        for _ in 0..passes {
            let s0 = self.dom.stack_len();
            if s0 == last {
                break;
            }
            last = s0;
            if !self.dom.propagate(self.p) {
                self.add_conflict();
                return false;
            }
            if !self.propagate_objective() {
                return false;
            }
            if !self.apply_dual_proofs() {
                if self.dom.conflict.is_some() {
                    self.add_conflict();
                }
                self.proof_prunes += 1;
                return false;
            }
            let lw = self.dom.take_low_water();
            let start = if self.lit_conflicts.dirty { 0 } else { self.lit_pos.min(lw) };
            self.lit_conflicts.dirty = false;
            let r = self.lit_conflicts.propagate(self.p, &mut self.dom, start);
            self.share_used_lits();
            self.lit_pos = self.dom.stack_len();
            self.dom.take_low_water();
            if r.is_err() {
                if self.dom.conflict.is_some() {
                    self.add_conflict();
                }
                self.lit_stats[1] += 1;
                return false;
            }
            if cutpool_prop() && !self.propagate_cut_pool() {
                return false;
            }
            if !self.orbitope_propagate(changes) {
                if self.dom.conflict.is_some() {
                    self.add_conflict();
                }
                self.orbitope_prunes += 1;
                return false;
            }
            if self.dom.stack_len() == s0 {
                break;
            }
        }
        true
    }

    /// カットプールの伝播 (`ENOMOTO_MIP_CUTPOOL_PROP`、試作): プールのカット (大域的に成り立つ `sum a x <= rhs`) を
    /// ノードの定義域で伝播する (HiGHS は `HighsDomain::CutpoolPropagation` で全ノードでプールを伝播する)。整数列は丸めて
    /// 締め、連続列は幅の `ENOMOTO_T_MIP_CUTPOOL_PROP_CONT` (既定 0.1) 倍以上縮むときだけ締める。境界が変わったら行の伝播を
    /// 挟んで最大 `ENOMOTO_T_MIP_CUTPOOL_PROP_PASSES` (既定 2) 回まで繰り返す。矛盾ならそのカットから衝突を作って偽。
    /// timtab1 はカットが短く (非零 2-4) 一般整数 (0-5) を含み、HiGHS は 7k ノードで証明するがこちらは 5 万ノードでも閉じない
    fn propagate_cut_pool(&mut self) -> bool {
        self.dom.set_external_tag(7);
        let r = self.propagate_cut_pool_inner();
        self.dom.set_external(false);
        r
    }

    fn propagate_cut_pool_inner(&mut self) -> bool {
        let p = self.p;
        let t0 = Instant::now();
        self.cutpool_prop_stats[0] += 1;
        if env_str!("ENOMOTO_MIP_DEBUG_CPP").is_some() && !self.params.submip && self.cutpool_prop_stats[0] < 20 {
            let nint = self.cut_pool.iter().filter(|(c, _, _)| c.iter().all(|&(j, _)| self.p.is_int[j])).count();
            let first = self.cut_pool.iter().position(|(c, _, _)| c.iter().all(|&(j, _)| self.p.is_int[j]));
            eprintln!("CPP enter submip {} nodes {} pool {} start {} int-only {nint} first {first:?} {:?}", self.params.submip, self.nodes, self.cut_pool.len(), self.pool_start.len(), first.map(|k| (&self.cut_pool[k].0, self.cut_pool[k].1, self.pool_start[k], self.pool_start[k + 1])));
        }
        let passes = tunable!("ENOMOTO_T_MIP_CUTPOOL_PROP_PASSES", 2usize, usize);
        let cont_frac = tunable!("ENOMOTO_T_MIP_CUTPOOL_PROP_CONT", 0.1, f64);
        let mut bad: Option<usize> = None;
        'outer: for _ in 0..passes {
            let mut changed = false;
            for k in 0..self.cut_pool.len() {
                let (s0, e0) = (self.pool_start[k] as usize, self.pool_start[k + 1] as usize);
                let rhs = self.cut_pool[k].1;
                let mut minact = 0.0f64;
                let mut ninf = 0usize;
                let mut infj = usize::MAX;
                for t in s0..e0 {
                    let (j, c) = (self.pool_idx[t] as usize, self.pool_val[t]);
                    let b = if c > 0.0 { self.dom.lo[j] } else { self.dom.up[j] };
                    if b.is_finite() {
                        minact += c * b;
                    } else {
                        ninf += 1;
                        infj = j;
                    }
                }
                if env_str!("ENOMOTO_MIP_DEBUG_CPP").is_some() && !self.params.submip && self.cutpool_prop_stats[0] < 12 && k + 738 == self.cut_pool.len() {
                    eprintln!("CPP call {} cut {k} minact {minact} rhs {rhs} ninf {ninf} vals {:?}", self.cutpool_prop_stats[0], (s0..e0).map(|t| (self.pool_idx[t], self.pool_val[t], self.dom.lo[self.pool_idx[t] as usize], self.dom.up[self.pool_idx[t] as usize])).collect::<Vec<_>>());
                }
                if ninf >= 2 {
                    continue;
                }
                if ninf == 0 && minact > rhs + 1e-6 * (1.0 + rhs.abs()) {
                    bad = Some(k);
                    break 'outer;
                }
                let gap = rhs - minact;
                for t in s0..e0 {
                    let (j, c) = (self.pool_idx[t] as usize, self.pool_val[t]);
                    if ninf == 1 && j != infj {
                        continue;
                    }
                    let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
                    if ninf == 0 && c.abs() * (up - lo) <= gap + 1e-9 * (1.0 + gap.abs()) {
                        continue;
                    }
                    let tight = if c > 0.0 {
                        let v = if ninf == 0 { lo + gap / c } else { gap / c };
                        let need = if p.is_int[j] { 1e-6 * (1.0 + up.abs().min(1e6)) } else { (cont_frac * (up - lo)).max(1e-6 * (1.0 + up.abs().min(1e6))) };
                        v < up - need && self.dom.tighten_upper(p, j, v)
                    } else {
                        let v = if ninf == 0 { up + gap / c } else { gap / c };
                        let need = if p.is_int[j] { 1e-6 * (1.0 + lo.abs().min(1e6)) } else { (cont_frac * (up - lo)).max(1e-6 * (1.0 + lo.abs().min(1e6))) };
                        v > lo + need && self.dom.tighten_lower(p, j, v)
                    };
                    if tight {
                        changed = true;
                        self.cutpool_prop_stats[2] += 1;
                    }
                    if self.dom.infeasible {
                        bad = Some(k);
                        break 'outer;
                    }
                }
            }
            if !changed {
                break;
            }
            let ok = self.dom.propagate(p);
            self.dom.set_external_tag(7);
            if !ok {
                self.cutpool_prop_stats[1] += 1;
                self.cutpool_prop_stats[3] += t0.elapsed().as_nanos() as u64;
                if env_str!("ENOMOTO_MIP_NO_EXTRA_CONFLICTS").is_none() {
                    self.add_conflict();
                }
                return false;
            }
        }
        let ok = match bad {
            None => true,
            Some(k) => {
                self.cutpool_prop_stats[1] += 1;
                if self.dom.infeasible {
                    if self.dom.conflict.is_some() && env_str!("ENOMOTO_MIP_NO_EXTRA_CONFLICTS").is_none() {
                        self.add_conflict();
                    }
                } else if env_str!("ENOMOTO_MIP_NO_EXTRA_CONFLICTS").is_none() && !self.params.submip {
                    let (coefs, rhs) = (self.cut_pool[k].0.clone(), self.cut_pool[k].1);
                    self.add_conflicts_multi(super::domain::ConflictStart::Proof(&coefs, rhs));
                }
                false
            }
        };
        self.cutpool_prop_stats[3] += t0.elapsed().as_nanos() as u64;
        ok
    }

    /// 1 本の行 `sum a_j x_j <= u` を定義域で伝播する ([`Self::propagate_objective_with`] の本体。根のプロービングの
    /// 被約費用の行にも使う)。矛盾なら偽。
    fn propagate_le_row(&mut self, obj: &[(usize, f64)], u: f64, conflicts: bool) -> bool {
        let p = self.p;
        for _ in 0..3 {
            let mut minact = 0.0f64;
            let mut ninf = 0usize;
            let mut infj = usize::MAX;
            for &(j, c) in obj.iter() {
                let b = if c > 0.0 { self.dom.lo[j] } else { self.dom.up[j] };
                if b.is_finite() {
                    minact += c * b;
                } else {
                    ninf += 1;
                    infj = j;
                }
            }
            if ninf >= 2 {
                return true;
            }
            if ninf == 0 && minact > u + 1e-6 * (1.0 + u.abs()) {
                self.obj_prop_stats[0] += 1;
                if conflicts && env_str!("ENOMOTO_MIP_NO_EXTRA_CONFLICTS").is_none() && !self.params.submip {
                    self.add_conflicts_multi(super::domain::ConflictStart::Proof(obj, u));
                }
                return false;
            }
            let gap = u - minact;
            let mut changed = false;
            self.dom.set_external_tag(6);
            for &(j, c) in obj.iter() {
                let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
                let rest_gap = if ninf == 0 {
                    // c x_j <= gap + c * (x_j の最小の側の境界)
                    if c.abs() * (up - lo) <= gap + 1e-9 * (1.0 + gap.abs()) {
                        continue;
                    }
                    gap
                } else if j == infj {
                    u - minact
                } else {
                    continue;
                };
                let tight = if c > 0.0 {
                    let v = if ninf == 0 { lo + rest_gap / c } else { rest_gap / c };
                    v < up - 1e-6 * (1.0 + up.abs().min(1e6)) && self.dom.tighten_upper(p, j, v)
                } else {
                    let v = if ninf == 0 { up + rest_gap / c } else { rest_gap / c };
                    v > lo + 1e-6 * (1.0 + lo.abs().min(1e6)) && self.dom.tighten_lower(p, j, v)
                };
                if tight {
                    changed = true;
                    self.obj_prop_stats[1] += 1;
                }
                if self.dom.infeasible {
                    break;
                }
            }
            self.dom.set_external(false);
            if self.dom.infeasible {
                if conflicts && self.dom.conflict.is_some() && env_str!("ENOMOTO_MIP_NO_EXTRA_CONFLICTS").is_none() {
                    self.add_conflict();
                }
                return false;
            }
            if !changed {
                break;
            }
            if !self.dom.propagate(p) {
                if conflicts {
                    self.add_conflict();
                }
                return false;
            }
        }
        true
    }

    /// 行にできない衝突を境界リテラルのまま衝突プールに入れる (`ENOMOTO_MIP_NO_LIT_CONFLICTS` で無効)。入れたら真。
    /// 使われた自前の衝突をほかのスレッドに知らせる (`ENOMOTO_MIP_TREE_SHARE_LITS_AT_ONCE` でなければ)。
    fn share_used_lits(&mut self) {
        let used = self.lit_conflicts.take_used_unshared();
        if used.is_empty() {
            return;
        }
        if let Some(pool) = &self.pool {
            if env_str!("ENOMOTO_MIP_TREE_NO_SHARED_PROOFS").is_none() {
                let max_len = tunable!("ENOMOTO_T_MIP_TREE_SHARE_LITS_LEN", usize::MAX, usize);
                let frac = tunable!("ENOMOTO_T_MIP_TREE_SHARE_LEN_FRAC", 0.0, f64);
                for l in used {
                    if l.len() <= max_len && (frac <= 0.0 || l.len() as f64 <= frac * self.p.n as f64) {
                        learned_push(pool, self.pool_tid, LearnedItem::Lits(l));
                    }
                }
            }
        }
    }

    fn add_lit_conflict(&mut self, lits: &[(usize, bool, f64)]) -> bool {
        if env_str!("ENOMOTO_MIP_NO_LIT_CONFLICTS").is_some() {
            return false;
        }
        // 大域的に成り立っているリテラルは除く (どのノードでも成り立つ)。全部そうなら使わない
        let gl = &self.dom.global_lo;
        let gu = &self.dom.global_up;
        let l: Vec<(usize, bool, f64)> = lits.iter().copied().filter(|&(j, upper, v)| if upper { gu[j] > v } else { gl[j] < v }).collect();
        if l.is_empty() {
            return false;
        }
        let ok = self.lit_conflicts.add(&l, self.importing);
        if ok {
            self.lit_stats[0] += 1;
            // 作ったときには知らせず、刈り込み・締め付けに使われたときに知らせる (`ENOMOTO_MIP_TREE_SHARE_LITS_AT_ONCE` なら作ったときに)
            // ([`Self::share_used_lits`])
            if !self.importing && env_str!("ENOMOTO_MIP_TREE_SHARE_LITS_AT_ONCE").is_some() {
                if let Some(pool) = &self.pool {
                    if env_str!("ENOMOTO_MIP_TREE_NO_SHARED_PROOFS").is_none() {
                        learned_push(pool, self.pool_tid, LearnedItem::Lits(l));
                    }
                }
            }
        }
        ok
    }

    /// 木を分け合う: ほかのスレッドが作った双対証明・衝突を取り込む。
    fn sync_learned(&mut self) {
        let Some(pool) = self.pool.clone() else { return };
        let got: Vec<LearnedItem> = {
            let g = pool.learned.lock().unwrap();
            let start = self.learned_cursor.max(g.first);
            let end = g.first + g.items.len() as u64;
            let out = (start..end)
                .filter_map(|k| {
                    let (tid, it) = &g.items[(k - g.first) as usize];
                    if *tid == self.pool_tid {
                        return None;
                    }
                    Some(match it {
                        LearnedItem::Proof(c, k, o) => LearnedItem::Proof(c.clone(), *k, *o),
                        LearnedItem::Lits(l) => LearnedItem::Lits(l.clone()),
                        LearnedItem::Cut(c, r) => LearnedItem::Cut(c.clone(), *r),
                    })
                })
                .collect();
            self.learned_cursor = end;
            out
        };
        self.importing = true;
        for it in got {
            match it {
                LearnedItem::Proof(c, k, o) => {
                    self.push_pool_row(c, k, o);
                    self.learned_stats[0] += 1;
                }
                LearnedItem::Lits(l) => {
                    if self.add_lit_conflict(&l) {
                        self.learned_stats[1] += 1;
                    }
                }
                LearnedItem::Cut(c, r) => self.add_to_pool(&c, r),
            }
        }
        self.importing = false;
    }

    fn add_conflict_lits(&mut self, lits: &[(usize, bool, f64)]) -> bool {
        let p = self.p;
        // 行にできなくても、衝突に現れた列は分枝の衝突スコアに数える
        self.pc.add_conflict(lits);
        // 長い衝突は行 (証明のプール、活動量で伝播) にせずリテラルのまま持つ
        if lits.len() > tunable!("ENOMOTO_T_MIP_CONFLICT_ROW_MAXLEN", 50usize, usize) {
            return self.add_lit_conflict(lits);
        }
        let mut coefs: Vec<(usize, f64)> = Vec::with_capacity(lits.len());
        // 各リテラルの「真の度合い」s (偽なら <= 0、真なら > 0、常に <= 1) の和 <= k - 1 (k はリテラルの数) にする。
        // 0-1 列は s = x (x >= 1) / 1 - x (x <= 0)。一般整数列 (大域的な境界 [L, U]) は x >= v なら
        // s = (x - v + 1) / (U - v + 1)、x <= v なら s = (v + 1 - x) / (v + 1 - L)。真のとき s は 1 未満になりうるので、
        // 全部真の点を切るには一般整数のリテラルは 1 つまで (2 つあると、1 つ偽でも和が k - 1 を超えうる)。
        let mut ones = lits.len() as f64 - 1.0; // 右辺 (定数項を移していく)
        let mut general = 0usize;
        for &(j, upper, v) in lits {
            if !p.is_int[j] {
                self.conf_stats[5] += 1;
                return self.add_lit_conflict(lits);
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
                    return self.add_lit_conflict(lits);
                }
                continue;
            }
            general += 1;
            if general > 1 || env_str!("ENOMOTO_MIP_NO_GENINT_CONFLICTS").is_some() {
                self.conf_stats[6] += 1;
                return self.add_lit_conflict(lits);
            }
            if !upper {
                // x >= v (v > L)
                if !gu.is_finite() || v <= gl || v > gu {
                    return self.add_lit_conflict(lits);
                }
                let w = gu - v + 1.0;
                coefs.push((j, 1.0 / w));
                ones += (v - 1.0) / w;
            } else {
                // x <= v (v < U)
                if !gl.is_finite() || v >= gu || v < gl {
                    return self.add_lit_conflict(lits);
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
            self.conf_stats[7] += 1;
            return self.add_lit_conflict(lits); // 同じ列の両側 (それ自体で矛盾) は使わない
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
            self.proof_col_index = [vec![Vec::new(); p.n], vec![Vec::new(); p.n]];
            for (k, (c, _, _)) in self.dual_proofs.iter().enumerate() {
                let id = self.proof_first_id + k as u64;
                for &(j, a) in c {
                    self.proof_col_index[(a <= 0.0) as usize][j].push((id, a));
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
        // 目的の証明の右辺 `U - konst` の U (打ち切り値。調べる間は変わらないので 1 回だけ読む: 共有の最良値の読み込みを
        // 証明ごとにしない)
        let obj_lim = self.prune_limit() - self.p.offset;
        for pass in 0..8 {
            let mut tightened_now = false;
            for &k in &scan {
                let rhs = {
                    let (_, konst, obj) = &self.dual_proofs[k];
                    if *obj {
                        obj_lim - konst
                    } else {
                        *konst
                    }
                };
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
                let org = (self.proof_state[k] == ProofState::Imported) as usize;
                self.tprof.proof[org][0] += 1;
                if slack < -1e-6 * (1.0 + rhs.abs()) {
                    if self.dbg_on || proof_diag() {
                        self.proof_diag_prune(k, rhs, minact, full);
                    }
                    self.tprof.proof[org][1] += 1;
                    self.proof_used(k);
                    self.proof_last_prune = Some(k);
                    self.proof_prune_after_tighten = tightened;
                    return false;
                }
                if !full && slack >= self.proof_base[k].2 {
                    continue; // どの境界も締まらない
                }
                // 整数列の境界を締める: a > 0 なら x_j <= l_j + slack / a
                let mut changes: Vec<(usize, bool, f64)> = Vec::new();
                let keys = &self.proof_keys[k];
                for (i, &(j, a)) in self.dual_proofs[k].0.iter().enumerate() {
                    // 係数は締め付けの上界の大きい順: 上界が余裕以下なら、この列も残りの列も締まらない (`ENOMOTO_MIP_PROOF_NO_EARLY_STOP` で打ち切らない)
                    if keys[i] <= slack + 1e-9 && env_str!("ENOMOTO_MIP_PROOF_NO_EARLY_STOP").is_none() {
                        break;
                    }
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
                let mut used = false;
                for (j, upper, v) in changes {
                    if self.dbg_on {
                        self.proof_diag_tighten(k, j, upper, v, rhs, minact);
                    }
                    self.dom.set_external_tag(4);
                    let ch = if upper { self.dom.tighten_upper(p, j, v) } else { self.dom.tighten_lower(p, j, v) };
                    self.dom.set_external(false);
                    if ch {
                        tightened = true;
                        tightened_now = true;
                        used = true;
                        self.proof_tightenings += 1;
                        self.tprof.proof[org][2] += 1;
                    }
                    if self.dom.infeasible {
                        self.tprof.proof[org][1] += 1;
                        self.proof_used(k);
                        self.proof_last_prune = Some(k);
                        self.proof_prune_after_tighten = true;
                        return false;
                    }
                }
                if used {
                    self.proof_used(k);
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

    /// 診断用 (`ENOMOTO_MIP_PROOF_VERIFY`): 証明がこのノードを刈った直後に、証明による締め付けを `pos` まで戻してノードの LP を
    /// 解き直し、LP の値 (定数項込み) が打ち切り値未満なら `PROOF_VERIFY: ... WRONG` と表示する (証明が誤りか、証明が頼った
    /// カットが LP から外れている)。LP 解での証明の活動量も出す (右辺を超えていれば、証明は今の LP の行から導けない)。
    fn proof_verify_lp(&mut self, pos: usize, node_lb: f64, dn: bool) {
        let p = self.p;
        self.dom.backtrack_to(p, pos);
        self.sync_lp();
        let st = self.lp.solve(&SolveLimits { iteration_limit: 200_000, cutoff: f64::INFINITY, deadline: self.deadline });
        let lim = self.prune_limit();
        let k = self.proof_last_prune;
        let desc = |s: &Self| match k {
            Some(k) => {
                let (c, konst, obj) = &s.dual_proofs[k];
                let rhs = s.proof_rhs(k);
                let exact: f64 = c.iter().map(|&(j, a)| if a > 0.0 { a * s.dom.lo[j] } else { a * s.dom.up[j] }).sum();
                format!("proof {k} ({:?}, obj {obj}, len {}, konst {konst:.10e}, rhs {rhs:.10e}, min activity at the node {exact:.10e})", s.proof_state[k], c.len())
            }
            None => "proof ? (pruned by propagation after tightening)".to_string(),
        };
        match st {
            LpStatus::Optimal => {
                let obj = self.lp.objective() + p.offset;
                if obj < lim - 1e-6 * (1.0 + lim.abs()) {
                    self.proof_verify_stats[1] += 1;
                    let x = self.lp.col_values();
                    let act = k.map(|k| self.dual_proofs[k].0.iter().map(|&(j, a)| a * x[j]).sum::<f64>());
                    // LP 解が証明を破るとき: 証明が使ったカットが全部まだ LP にあれば、証明は今の LP の行から導けるはずなので誤りと断定できる
                    let verdict = match (k, act) {
                        (Some(k), Some(a)) if a > self.proof_rhs(k) + 1e-6 * (1.0 + self.proof_rhs(k).abs()) => match &self.proof_cuts[k] {
                            Some(ids) => {
                                let present: std::collections::HashSet<u64> = self.cut_ids.iter().copied().collect();
                                let missing = ids.iter().filter(|id| !present.contains(id)).count();
                                if missing == 0 {
                                    format!("DEFINITE (the LP solution violates the proof and all {} cut rows the proof used are still in the LP)", ids.len())
                                } else {
                                    format!("undecided ({missing} of {} cut rows the proof used were removed from the LP)", ids.len())
                                }
                            }
                            None => "undecided (the LP solution violates the proof; imported or conflict row, used cuts unknown)".to_string(),
                        },
                        _ => "undecided (the LP solution satisfies the proof: pruned through integer tightening)".to_string(),
                    };
                    if verdict.starts_with("DEFINITE") {
                        self.proof_verify_stats[3] += 1;
                    }
                    eprintln!(
                        "PROOF_VERIFY: WRONG: node {} thread {}: {} pruned a node whose LP objective is {obj:.10e} < prune limit {lim:.10e} (node lower bound {node_lb:.10e}, proof activity at the LP solution {act:?}, after tightening {}, debug solution in node {dn}, lp rows {}): {verdict}",
                        self.nodes, self.pool_tid, desc(self), self.proof_prune_after_tighten, self.lp.num_rows()
                    );
                } else {
                    self.proof_verify_stats[0] += 1;
                }
            }
            LpStatus::Infeasible => self.proof_verify_stats[0] += 1,
            _ => {
                self.proof_verify_stats[2] += 1;
                eprintln!("PROOF_VERIFY: node {} thread {}: {} pruned a node; LP {st:?} (not verified)", self.nodes, self.pool_tid, desc(self));
            }
        }
    }

    /// 診断用 (`ENOMOTO_MIP_PROOF_DIAG` またはデバッグ解): 証明 `k` がこのノードを刈るとき、差分で求めた最小活動量
    /// `minact` を今の境界で全部計算し直したものと比べ、デバッグ解があれば、証明そのものがデバッグ解を切っていないか
    /// (証明が誤り) と、ノードがデバッグ解を含んでいたか (含んでいて証明が正しければ差分の誤り) を表示する。
    fn proof_diag_prune(&self, k: usize, rhs: f64, minact: f64, full: bool) {
        let (c, konst, obj) = &self.dual_proofs[k];
        let exact: f64 = c.iter().map(|&(j, a)| if a > 0.0 { a * self.dom.lo[j] } else { a * self.dom.up[j] }).sum();
        let tol = 1e-6 * (1.0 + rhs.abs());
        let origin = format!("{:?}", self.proof_state[k]);
        if !full && (exact - minact).abs() > 1e-6 * (1.0 + exact.abs()) && exact <= rhs + tol {
            eprintln!("PROOF_DIAG: node {} thread {}: proof {k} ({origin}, obj {obj}, len {}) prunes with incremental min activity {minact:.10e} but the exact one is {exact:.10e} (rhs {rhs:.10e})", self.nodes, self.pool_tid, c.len());
        }
        if self.dbg_on {
            DEBUG_SOL.with(|d| {
                if let Some(x) = d.borrow().as_ref() {
                    let dz = self.p.objective(x);
                    if dz < self.prune_limit() - 1e-6 {
                        let act: f64 = c.iter().map(|&(j, a)| a * x[j]).sum();
                        let inside = (0..self.p.n).all(|j| x[j] >= self.dom.lo[j] - 1e-6 && x[j] <= self.dom.up[j] + 1e-6);
                        if act > rhs + tol {
                            eprintln!("MIP_DEBUG_SOL: node {} thread {}: proof {k} ({origin}, obj {obj}, len {}, konst {konst:.10e}) is violated by the debug solution: activity {act:.10e} > rhs {rhs:.10e} (debug obj {dz}, prune limit {})", self.nodes, self.pool_tid, c.len(), self.prune_limit());
                        } else if inside {
                            eprintln!("MIP_DEBUG_SOL: node {} thread {}: proof {k} ({origin}) prunes a node that contains the debug solution although the debug solution satisfies it (activity {act:.10e} <= rhs {rhs:.10e}; min activity incremental {minact:.10e} exact {exact:.10e})", self.nodes, self.pool_tid);
                        }
                    }
                }
            });
        }
    }

    /// 診断用 (デバッグ解): 証明 `k` による締め付け `x_j <= v` (`upper`) / `x_j >= v` が、今の定義域が含むデバッグ解を切るなら表示する。
    fn proof_diag_tighten(&self, k: usize, j: usize, upper: bool, v: f64, rhs: f64, minact: f64) {
        DEBUG_SOL.with(|d| {
            if let Some(x) = d.borrow().as_ref() {
                let cuts = if upper { x[j] > v + 1e-6 } else { x[j] < v - 1e-6 };
                if !cuts || self.p.objective(x) >= self.prune_limit() - 1e-6 || !(0..self.p.n).all(|i| x[i] >= self.dom.lo[i] - 1e-6 && x[i] <= self.dom.up[i] + 1e-6) {
                    return;
                }
                let (c, _, obj) = &self.dual_proofs[k];
                let act: f64 = c.iter().map(|&(i, a)| a * x[i]).sum();
                let exact: f64 = c.iter().map(|&(i, a)| if a > 0.0 { a * self.dom.lo[i] } else { a * self.dom.up[i] }).sum();
                eprintln!("MIP_DEBUG_SOL: node {} thread {}: proof {k} ({:?}, obj {obj}) tightens col {j} {} {v} cutting the debug solution (x {}); proof activity at the debug solution {act:.10e}, rhs {rhs:.10e}, min activity incremental {minact:.10e} exact {exact:.10e}", self.nodes, self.pool_tid, self.proof_state[k], if upper { "<=" } else { ">=" }, x[j]);
            }
        });
    }

    /// 列 `cols` の境界の変化 (`proof_val_*` から今の境界へ) を証明の差分 (`proof_delta`) に足し、差分が変わった
    /// 証明を返す (`proof_touched` にも加える)。基準の境界より根の状態が締まっている列は作り直しのときにその差も
    /// 入るので、見積もりは真の最小活動量以下のまま。
    fn proof_absorb(&mut self, cols: &[usize]) -> Vec<usize> {
        let mut changed: Vec<usize> = Vec::new();
        if self.proof_chg_mark.len() < self.proof_delta.len() {
            self.proof_chg_mark.resize(self.proof_delta.len(), 0);
        }
        self.proof_chg_stamp = self.proof_chg_stamp.wrapping_add(1);
        if self.proof_chg_stamp == 0 {
            self.proof_chg_mark.iter_mut().for_each(|m| *m = 0);
            self.proof_chg_stamp = 1;
        }
        let stamp = self.proof_chg_stamp;
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
                // この側の境界が最小活動量に効く証明だけをたどる (下限は係数が正、上限は係数が正でないもの)
                for &(id, a) in &self.proof_col_index[upper as usize][j] {
                    if id < self.proof_first_id {
                        continue; // 古い証明
                    }
                    let k = (id - self.proof_first_id) as usize;
                    self.tprof.proof[(self.proof_state[k] == ProofState::Imported) as usize][3] += 1;
                    let (old, new) = (a * old_v, a * new_v);
                    if !self.proof_in_touched[k] {
                        self.proof_in_touched[k] = true;
                        self.proof_touched.push(k);
                    }
                    if self.proof_chg_mark[k] != stamp {
                        self.proof_chg_mark[k] = stamp;
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

    /// 根の LP を解く前から新たに大域固定された整数列の数 ([`Self::maybe_restart`] の判定と同じ数え方)。
    fn newly_fixed_ints(&self) -> usize {
        let p = self.p;
        let fixed = (0..p.n).filter(|&j| p.is_int[j] && self.dom.global_lo[j] == self.dom.global_up[j]).count();
        fixed.saturating_sub(self.root_fixed0)
    }

    /// 早めの再スタート ([`super::early_restart_on`])・木の中の再スタート (`ENOMOTO_MIP_TREE_RESTART`、試作) の判定: 新たな固定が、根の始めに固定されて
    /// いなかった整数列の `frac` 以上 (かつ [`Self::maybe_restart`] の判定 (2 回目以降 2.5%) も通る) で、再スタートの回数と時間が残っているか。
    fn proto_restart_ok(&self, frac: f64) -> bool {
        if self.params.submip || self.snap.is_some() || self.time_up() || env_str!("ENOMOTO_MIP_NO_RESTART").is_some() {
            return false;
        }
        if self.params.restarts >= tunable!("ENOMOTO_T_MIP_MAX_RESTARTS", 4u32, u32) {
            return false;
        }
        let nint = self.p.nint;
        let free = nint.saturating_sub(self.root_fixed0);
        let newly = self.newly_fixed_ints();
        let fac = if self.params.restarts == 0 { tunable!("ENOMOTO_T_MIP_RESTART_FIRST_FAC", 0.0, f64) } else { tunable!("ENOMOTO_T_MIP_RESTART_FAC", 0.025, f64) };
        newly > 0 && newly as f64 >= frac * free as f64 && newly as f64 >= fac * nint as f64
    }

    /// 根での再スタート (SCIP の `restartfac`): 根の処理で新たに大域固定された整数列が全体の 2.5% を超えたら、
    /// 大域的な境界と LP に残ったカットを持って前処理からやり直す。暫定解は打ち切り値として渡し、
    /// やり直しで良い解が見つからなければそれを返す。最大 2 回。
    fn maybe_restart(&mut self) -> Option<MipResult> {
        let p = self.p;
        if self.params.submip || self.params.restarts >= tunable!("ENOMOTO_T_MIP_MAX_RESTARTS", 4u32, u32) || env_str!("ENOMOTO_MIP_NO_RESTART").is_some() || self.time_up() {
            return None;
        }
        let nint = p.nint;
        let fixed = (0..p.n).filter(|&j| p.is_int[j] && self.dom.global_lo[j] == self.dom.global_up[j]).count();
        let newly = fixed.saturating_sub(self.root_fixed0);
        // HiGHS と同じく、初回は固定された整数列が 1 本でもあれば、2 回目以降は 2.5% 以上で再スタートする
        let fac = if self.params.restarts == 0 { tunable!("ENOMOTO_T_MIP_RESTART_FIRST_FAC", 0.0, f64) } else { tunable!("ENOMOTO_T_MIP_RESTART_FAC", 0.025, f64) };
        // `ENOMOTO_MIP_RESTART_FRAC_FREE` (試作): 割合の分母を、この根の始めに固定されていなかった整数列の数にする (HiGHS の
        // `percentageInactiveIntegers` は再スタート後の模型の整数列に対する割合)。再スタートの後の問題は前に固定した列を
        // 固定した列として持ったままなので、分母が全体 (h80x6320d では 6320 本) だと、自由な整数列 (約 1000 本) の 16% 近くを
        // 固定しないと 2 回目の再スタートが起きない
        // `ENOMOTO_MIP_GR_RESTART=ff` も同じ ([`super::gr_restart_ff_for`])
        let denom = if env_str!("ENOMOTO_MIP_RESTART_FRAC_FREE").is_some() || super::gr_restart_ff_for(p) { nint.saturating_sub(self.root_fixed0) } else { nint };
        if self.params.verbose && self.params.restarts > 0 {
            eprintln!("MIP: restart check: newly fixed {newly}, integer columns {nint}, free at the root {}, threshold {:.1}", nint.saturating_sub(self.root_fixed0), fac * denom as f64);
        }
        if nint == 0 || newly == 0 || (newly as f64) < fac * denom as f64 {
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
        // `ENOMOTO_MIP_RESTART_CUTS_READD`: 効いているカットを問題の行にはせず、新しい求解の最初に (根の LP を解く前に)
        // カットの行として加え直す (HiGHS の `separateLpCutsAfterRestart`)。根の LP の下界は行にしたときと同じだが、
        // 木の中では年齢で外せる普通のカットになる。行にすると neos-911970 では 4 回の再スタートで 107 行 -> 358 行になり
        // (効いているものだけ残しても)、木の LP は 405 -> 822 行、ノードの LP は 1 回平均 187 反復 (行が 190 程度なら 47)。
        // neos-911970 (60 秒) では暫定解 56.23 -> 54.83 (最適値 54.76)・下界 53.69 -> 53.62 で解けるには至らず、再スタート後の
        // カットのループが進まなくなる (47.26 のまま。カットの行の上に作る次のカットが変わる) ので既定では使わない
        let readd = (env_str!("ENOMOTO_MIP_RESTART_CUTS_READD").is_some() || self.highs_cutmgmt() || super::gr_restart_readd_for(p)) && !keep_all;
        // `ENOMOTO_MIP_RESTART_CUTS_ROWS` (試作、既定では使わない): 再スタートで新たに固定される整数列が、根の始めに自由だった整数列の
        // `ENOMOTO_T_MIP_RESTART_CUTS_ROWS_FRAC` (0.3) 以上なら、readd でも問題の行として持ち込む (前処理が列を別の列で置き換えると、
        // カットを写せずに捨てていた頃の回避。今は [`super::restart_cuts_subst`] で写すので要らず、前処理が効かなくなる分遅くなりうる)
        let big_restart = super::restart_cuts_rows()
            && self.newly_fixed_ints() as f64 >= tunable!("ENOMOTO_T_MIP_RESTART_CUTS_ROWS_FRAC", 0.3, f64) * p.nint.saturating_sub(self.root_fixed0) as f64;
        let switched_to_rows = readd && big_restart;
        let readd = readd && !big_restart;
        let cuts_as_rows = (env_str!("ENOMOTO_MIP_RESTART_CUTS_TO_POOL").is_none() && !readd) || keep_all;
        let mut carry: Vec<(Vec<(usize, f64)>, f64)> = Vec::new();
        let act = self.lp.row_activities();
        for i in p.m..self.lp.num_rows() {
            let (l, u) = self.lp.row_bounds(i);
            let tight = (u.is_finite() && act[i] >= u - 1e-6 * (1.0 + u.abs())) || (l.is_finite() && act[i] <= l + 1e-6 * (1.0 + l.abs()));
            if !cuts_as_rows {
                if readd && !tight {
                    continue;
                }
                let r = self.lp.row(i);
                if u.is_finite() {
                    carry.push((r.clone(), u));
                }
                if l.is_finite() {
                    carry.push((r.iter().map(|&(j, v)| (j, -v)).collect(), -l));
                }
                continue;
            }
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
        // 幅が許容誤差以下になった列 (伝播は連続列の境界を誤差の分だけ緩めて締める) も 1 点に固定する ([`super::fix_tiny_width_columns`])
        let tiny = super::fix_tiny_width_columns(&rows, &mut row_lo, &mut row_up, &mut glo, &mut gup, &p.is_int);
        // [`super::restart_cuts_fix`] の (2) (`ENOMOTO_MIP_NO_RESTART_CUTS_FIX` で使わない): プールの経路のカットは、大域的に固定された列を代入して右辺に移す (前処理で固定された
        // 列を含むカットは写すときに捨てられるので)。代入して項が無くなったカットは捨てる
        if super::restart_cuts_fix() && !cuts_as_rows {
            let before = carry.len();
            carry = carry
                .into_iter()
                .filter_map(|(c, r)| {
                    let mut rr = r;
                    let cc: Vec<(usize, f64)> = c.into_iter().filter(|&(j, v)| if glo[j] == gup[j] { rr -= v * glo[j]; false } else { true }).collect();
                    (!cc.is_empty()).then_some((cc, rr))
                })
                .collect();
            if self.params.verbose {
                eprintln!("MIP: restart: substituted the fixed columns into the carried cuts ({} -> {} cuts)", before, carry.len());
            }
        }
        // 診断用 (`ENOMOTO_MIP_DEBUG_RESTART_CUT_USE`): 持ち込むカットの経路と数、新しい大域的な境界の下で完全に冗長になったものの数、
        // 固定された列を含むものの数
        if env_str!("ENOMOTO_MIP_DEBUG_RESTART_CUT_USE").is_some() {
            let path = if switched_to_rows { "rows (readd switched by the large restart)" } else if cuts_as_rows { "rows" } else if readd { "readd (pool + cut rows)" } else { "pool" };
            let cuts: Vec<(Vec<(usize, f64)>, f64)> = if cuts_as_rows {
                (p.m..rows.len()).flat_map(|i| {
                    let mut v = Vec::new();
                    if row_up[i].is_finite() { v.push((rows[i].clone(), row_up[i])); }
                    if row_lo[i].is_finite() { v.push((rows[i].iter().map(|&(j, a)| (j, -a)).collect(), -row_lo[i])); }
                    v
                }).collect()
            } else {
                carry.clone()
            };
            let (mut redundant, mut with_fixed, mut fixed_terms, mut terms) = (0usize, 0usize, 0usize, 0usize);
            for (c, r) in &cuts {
                let maxact: f64 = c.iter().map(|&(j, a)| if a > 0.0 { a * gup[j] } else { a * glo[j] }).sum();
                if maxact <= r + 1e-9 * (1.0 + r.abs()) {
                    redundant += 1;
                }
                let nf = c.iter().filter(|&&(j, _)| glo[j] == gup[j]).count();
                if nf > 0 {
                    with_fixed += 1;
                }
                fixed_terms += nf;
                terms += c.len();
            }
            eprintln!(
                "RESTART_CUT_USE: restart {} path {path}: carried {} (cut pool {}), redundant under the new global bounds {redundant}, with fixed columns {with_fixed} (fixed terms {fixed_terms} of {terms})",
                self.params.restarts + 1, cuts.len(), self.cut_pool.len()
            );
        }
        if self.params.verbose && tiny > 0 {
            eprintln!("MIP: restart: fixed {tiny} columns of width within tolerance");
        }
        // 診断用 (`ENOMOTO_MIP_DEBUG_RESTART_CUTS`): 残したカットの行を、整数列だけのものとそうでないものに分けて数え、整数列だけの行を出す
        if env_str!("ENOMOTO_MIP_DEBUG_RESTART_CUTS").is_some() {
            let (mut ni, mut nm) = (0, 0);
            for i in p.m..rows.len() {
                let nc = rows[i].iter().filter(|&&(j, _)| !p.is_int[j]).count();
                if nc == 0 {
                    ni += 1;
                    let t: Vec<String> = rows[i].iter().map(|&(j, v)| format!("{j}:{v:.6}")).collect();
                    eprintln!("RCUT int-only len {} lo {:.6} up {:.6} : {}", rows[i].len(), row_lo[i], row_up[i], t.join(" "));
                } else {
                    nm += 1;
                }
            }
            let carry_int = carry.iter().filter(|(r, _)| r.iter().all(|&(j, _)| j < p.n && p.is_int[j])).count();
            eprintln!("RCUT summary: kept cut rows {} (integer-only {ni}, with continuous {nm}); carried to the pool {} (integer-only {carry_int}); cut pool {}", rows.len() - p.m, carry.len(), self.cut_pool.len());
            let pool_int = self.cut_pool.iter().filter(|(r, _, _)| r.iter().all(|&(j, _)| j < p.n && p.is_int[j])).count();
            eprintln!("RCUT cut pool integer-only {pool_int}");
        }
        let newp = MipProblem::from_rows(glo, gup, p.cost.clone(), p.offset, p.sense_sign, p.is_int.clone(), rows, row_lo, row_up);
        // 診断用 (`ENOMOTO_MIP_DEBUG_DUMP_RESTART=接頭辞`): 再スタートで渡す問題を `接頭辞_r回数.mps` に書く
        if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_DUMP_RESTART").filter(|_| !self.params.submip) {
            let f = format!("{f}_r{}.mps", self.params.restarts + 1);
            super::dump_mps(&newp, &f);
            eprintln!("MIP: restart: dumped the problem to {f} (offset {}, cutoff {}, incumbent {:?})", newp.offset, self.prune_limit().min(self.params.cutoff), self.incumbent.as_ref().map(|(z, _)| *z));
        }
        let remaining = match self.deadline {
            Some(d) => d.saturating_duration_since(Instant::now()).as_secs_f64(),
            None => f64::INFINITY,
        };
        let params = MipParams {
            time_limit: remaining,
            node_limit: self.params.node_limit.saturating_sub(self.nodes),
            cutoff: self.prune_limit().min(self.params.cutoff),
            restarts: self.params.restarts + 1,
            // `ENOMOTO_MIP_RESTART_NOINC_HEURS`: 暫定解がなければ飛ばさない (再スタートで固定が増えて問題が締まった後なので、
            // 同じヒューリスティクスでも解が見つかりうる)。60 秒・1 スレッドの 40 問では暫定解のないまま再スタートする問題が
            // なく効果を測れないので既定では使わない
            skip_heurs: if self.incumbent.is_none() && env_str!("ENOMOTO_MIP_RESTART_NOINC_HEURS").is_some() { 0 } else { self.params.skip_heurs | self.failed_heurs },
            ..self.params
        };
        user_log!("Restart {}: {} of {} integer columns newly fixed at the root ({:.2}s)", params.restarts, newly, nint, self.start.elapsed().as_secs_f64());
        if self.params.verbose {
            eprintln!(
                "MIP: restart {} ({} of {} integer columns newly fixed at the root, {} cuts kept) ({:.2}s)",
                params.restarts,
                newly,
                nint,
                if readd { carry.len() } else { newp.m - p.m },
                self.start.elapsed().as_secs_f64()
            );
        }
        if !cuts_as_rows {
            if !readd {
                carry.extend(self.cut_pool.iter().map(|(c, r, _)| (c.clone(), *r)));
            }
            super::RESTART_CUTS.with(|s| *s.borrow_mut() = Some(carry));
            super::RESTART_CUTS_READD.with(|f| f.set(readd && super::restart_cuts_fix()));
        }
        // 暫定解を引き継ぐ (新しい問題は列が同じ)
        if env_str!("ENOMOTO_MIP_RESTART_NO_INCUMBENT").is_none() {
            // `ENOMOTO_MIP_RESTART_REF` (試作): 暫定解がなければ参照解を渡す (2 回目以降の再スタートでも参照解を失わない。
            // qnet1 は 3 回再スタートし、渡さないと 2 回目の後の求解が暫定解なしで局所探索を 2.7〜2.9 秒ずつ回していた)
            let ref_x = if self.incumbent.is_none() && super::restart_ref_for(p) { self.ref_sol.clone() } else { None };
            super::RESTART_SOL.with(|s| *s.borrow_mut() = self.incumbent.as_ref().map(|(_, x)| x.clone()).or(ref_x));
        }
        // `ENOMOTO_MIP_RESTART_BASIS` (試作): 今の LP (根のカットの後) の基底を、再スタートの後の根の LP に渡す
        if restart_basis_on() {
            let b = self.lp.basis();
            let rows: Vec<(Vec<(usize, f64)>, super::lp::VarStatus)> = (0..self.lp.num_rows().min(b.row.len())).map(|i| (self.lp.row(i), b.row[i])).collect();
            RESTART_BASIS.with(|s| *s.borrow_mut() = Some((b.col, rows)));
        }
        super::RESTART_HCM.with(|f| f.set(self.hcm_auto && super::restart_hcm_keep()));
        let r = super::solve_problem(&newp, params, true);
        super::RESTART_HCM.with(|f| f.set(false));
        super::RESTART_CUTS_READD.with(|f| f.set(false));
        RESTART_BASIS.with(|s| *s.borrow_mut() = None);
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
            // 再スタート後の求解の下界が失われていても (暫定解なしで終わった、など)、再スタート前の根の下界は正しい
            s => (s, r.best_bound.max(self.root_lb).min(objective.unwrap_or(f64::INFINITY))),
        };
        Some(MipResult { status, x, objective, best_bound, nodes, lp_iterations: iters })
    }

    /// 別スレッドでサブ MIP を解き始める (同時に走らせるのはスレッド数 - 1 本まで。超えたら古いものを待つ)。
    pub(super) fn spawn_submip(&mut self, sub: MipProblem, params: MipParams) {
        let lim = submip_par_limit();
        if self.pending_submips.len() >= lim {
            // `ENOMOTO_MIP_SUBMIP_FIRST_DONE` (試作): 空きを待つとき、出した順 (いちばん古いもの) ではなく先に終わったものから受け取る
            // ([`Self::join_submips`])。nu25-pr12 では LP face のサブ MIP が 0.7 秒で最適解を見つけて終わっていたのに、古い根の被約費用の
            // サブ MIP (時間上限 4.1 秒) を待つ間、その解を 3 秒以上受け取れなかった
            self.join_submips(lim - 1);
        }
        let presolve = env_str!("ENOMOTO_MIP_SUBMIP_NO_PRESOLVE").is_none();
        let is_rens = std::mem::take(&mut self.submip_is_rens);
        if self.params.verbose {
            let nfree = (0..sub.n).filter(|&j| sub.col_lo[j] < sub.col_up[j]).count();
            eprintln!("MIP:   parallel sub-MIP started ({}): {nfree} free columns of {}", self.heur_label, sub.n);
        }
        let label = self.heur_label;
        let fix_rate = self.submip_fix_rate.take();
        let basis = self.submip_basis.take();
        let ctl = (submip_outcome_stop() && !self.params.submip && self.snap.is_none()).then(|| std::sync::Arc::new(SubmipCtl::new(self.prune_limit())));
        let ctl2 = ctl.clone();
        self.pending_submips.push_back((
            std::thread::spawn(move || {
                // 別スレッドで解くサブ MIP も入れ子の深さ 1 として数える ([`super::heuristics::SUBMIP_DEPTH`])
                super::heuristics::SUBMIP_DEPTH.with(|d| d.set(1));
                if let Some(c) = ctl2 {
                    ROOT_SUBMIP_CTL.with(|s| *s.borrow_mut() = Some(c));
                }
                if let Some(b) = basis {
                    SUBMIP_BASIS.with(|s| *s.borrow_mut() = Some(b));
                }
                // `ENOMOTO_MIP_FJ_P3=submip` (試作): サブ MIP の前処理でも p5s の前処理の変更を有効にする
                if env_str!("ENOMOTO_MIP_FJ_P3") == Some("submip") {
                    crate::set_fj_p3_force(true);
                }
                super::solve_problem(&sub, params, presolve)
            }),
            is_rens,
            label,
            Instant::now(),
            fix_rate,
            ctl,
        ));
    }

    /// 別スレッドのサブ MIP を、残りが `keep` 本になるまで出した順に待って結果を受け取る。
    pub(super) fn join_submips(&mut self, keep: usize) {
        if env_str!("ENOMOTO_MIP_SUBMIP_FIRST_DONE").is_some() || submip_outcome_stop() {
            // 終わったものから受け取る (どれも終わっていなければ 1 ms 待って見直す)。受け取って良くなった暫定解は残りのサブ MIP に渡す
            while self.pending_submips.len() > keep {
                let z = self.prune_limit();
                for it in &self.pending_submips {
                    if let Some(c) = &it.5 {
                        c.offer(z);
                    }
                }
                match self.pending_submips.iter().position(|it| it.0.is_finished()) {
                    Some(k) => {
                        let item = self.pending_submips.remove(k).unwrap();
                        self.take_submip(item);
                    }
                    None => std::thread::sleep(std::time::Duration::from_millis(1)),
                }
            }
            return;
        }
        while self.pending_submips.len() > keep {
            let item = self.pending_submips.pop_front().unwrap();
            self.take_submip(item);
        }
    }

    /// 別スレッドのサブ MIP のうち終わっているものの結果を受け取る (終わっていないものは待たない)。
    pub(super) fn poll_submips(&mut self) {
        let mut k = 0;
        while k < self.pending_submips.len() {
            if self.pending_submips[k].0.is_finished() {
                let item = self.pending_submips.remove(k).unwrap();
                self.take_submip(item);
            } else {
                k += 1;
            }
        }
    }

    /// 別スレッドのサブ MIP 1 本の結果を受け取る。
    fn take_submip(&mut self, item: (std::thread::JoinHandle<MipResult>, bool, &'static str, std::time::Instant, Option<f64>, Option<std::sync::Arc<SubmipCtl>>)) {
        let (h, is_rens, label, t0, fix_rate, _) = item;
        {
            if let Ok(r) = h.join() {
                self.heur_iters += r.lp_iterations;
                // 根の RENS がノード上限で止まったら、ノード上限を増やしてやり直す判断に使う (逐次のときは rens が立てる。
                // 並列モードでは結果をここで受け取るので、ここで立てないとやり直しが行われない: binkar10_1 で 4 スレッドが
                // 6754.85 を見つけられず、木が 1 スレッドの 10 倍に増えていた)
                if is_rens && r.status == MipStatus::NodeLimit && env_str!("ENOMOTO_MIP_NO_PARALLEL_RENS_RETRY").is_none() {
                    self.rens_node_limited = true;
                }
                if self.params.verbose {
                    eprintln!("MIP:   parallel sub-MIP ({label}): status {:?}, nodes {}, objective {:?}, {:.2}s", r.status, r.nodes, r.objective, t0.elapsed().as_secs_f64());
                }
                let mut acc = false;
                let infeasible = r.status == MipStatus::Infeasible;
                if let Some(x) = r.x {
                    let keep = std::mem::replace(&mut self.heur_label, label);
                    acc = self.try_incumbent(x);
                    self.heur_label = keep;
                }
                if let Some(fr) = fix_rate {
                    self.note_fix_rate(fr, infeasible, acc);
                }
            }
        }
    }

    /// 根の「暫定解がなければ回す」ヒューリスティクス (Feasibility Pump・ダイビング・Shift-and-Propagate・局所探索・Repair) を
    /// 回すか。暫定解がなければ真。`ENOMOTO_MIP_FJ2_KEEP_NOINC` なら、暫定解が Feasibility Jump のものだけの間も真
    /// (FJ の質の悪い解のせいで、より良い解を見つける根の局所探索などを飛ばさないように。ns1830653 は根の局所探索が
    /// 26622 を見つけるが、FJ の 43808 があると回らなかった)。
    pub(super) fn noinc_like(&self) -> bool {
        // 参照解 (`ENOMOTO_MIP_RESTART_REF`) があれば、再スタート前の暫定解が打ち切り値として生きているので暫定解なし扱いにしない
        (self.incumbent.is_none() && self.ref_sol.is_none())
            || (self.inc_only_fj && env_str!("ENOMOTO_MIP_FJ2_KEEP_NOINC").is_some())
            || (self.fj_poor() && env_str!("ENOMOTO_MIP_FJ_POOR_NOINC").is_some_and(|v| v != "rens"))
    }

    /// `ENOMOTO_MIP_FJ_POOR_NOINC` (試作): 根で、暫定解が Feasibility Jump のものだけで、その目的値が根の LP の値から大きく
    /// 離れている (相対ギャップ (z - lb) / max(|z|, |lb|, 1) が `ENOMOTO_T_MIP_FJ_POOR_GAP` (既定 0.5) を超える) か。
    /// 真なら、根の暫定解なし用のヒューリスティクス ([`Self::noinc_like`]、値が `rens` なら回さない) と暫定解なし用の
    /// ダイビング式 RENS を、暫定解がないときと同じく回す (FJ の解は暫定解として残し、打ち切り値に使う)。
    /// d1bfefd (FJ2 の既定化) で、FJ2 が数秒で出す質の悪い解 (hypothyroid-k1 の -3、nursesched-sprint02 の 251) が、
    /// 以前は最適値近くを見つけていた暫定解なし用の RENS (hypothyroid-k1 -2851) やベクトル長ダイビング (nursesched 69)
    /// を止めていた。FJ の解が根の LP に近い問題 (KEEP_NOINC で悪化した C の問題を想定) では今までどおり回さない
    pub(super) fn fj_poor(&self) -> bool {
        let Some((z, _)) = self.incumbent.as_ref() else { return false };
        if !self.inc_only_fj || self.nodes > 1 || self.params.submip || !self.fj_poor_lb.is_finite() || (env_str!("ENOMOTO_MIP_FJ_POOR_NOINC").is_none() && !fj_poor_bg_rens()) {
            return false;
        }
        let lb = self.fj_poor_lb;
        (z - lb) / z.abs().max(lb.abs()).max(1.0) > tunable!("ENOMOTO_T_MIP_FJ_POOR_GAP", 0.5, f64)
    }

    /// 別スレッドの Feasibility Jump が終わっていれば結果を受け取る (`ENOMOTO_MIP_NO_FJ2` でないとき。根のカットのループの各ラウンドで
    /// 呼ぶ。HiGHS 流の FJ は多くの問題で 1 秒以内に終わるので、カットのループの終わりまで待たずに暫定解にする)。
    pub(super) fn poll_fj_thread(&mut self) {
        self.poll_lmip_thread();
        if self.fj_thread.as_ref().is_some_and(|(h, _)| h.is_finished()) && env_str!("ENOMOTO_MIP_NO_FJ2").is_none() {
            self.join_fj_thread();
        }
    }

    /// `ENOMOTO_MIP_LOCALMIP` の別スレッドが終わっていれば結果を受け取る (待たない)。
    pub(super) fn poll_lmip_thread(&mut self) {
        // 受け渡し箱には良くなった解を全部ためてあり、新しい (良い) 順に試して、受け付けられたらそこで止める。最良の解が受け付け
        // られなくても (対称性の行に反するなど。neos-950242 で最良の解だけを残していたときは全部失った)、前の解が暫定解になれる。
        // 古い順に全部試すと、uccase12 では受け付けるたびの oneopt・連続列の解き直しに 1 回 1 秒かかり、根で 20 秒を失った
        let got: Vec<Vec<f64>> = self.lmip_mailbox.as_ref().map(|mb| std::mem::take(&mut *mb.lock().unwrap())).unwrap_or_default();
        for x in got.into_iter().rev() {
            self.heur_label = "Local-MIP";
            let ok = self.try_incumbent(x);
            self.heur_label = "";
            if self.params.verbose || env_str!("ENOMOTO_MIP_FJ_DEBUG").is_some() {
                eprintln!("MIP: Local-MIP solution accepted {ok} ({:.2}s)", self.start.elapsed().as_secs_f64());
            }
            if ok {
                break;
            }
        }
        if self.lmip_thread.as_ref().is_some_and(|(h, _)| h.is_finished()) {
            let (h, _) = self.lmip_thread.take().unwrap();
            if let Ok(Some(x)) = h.join() {
                self.heur_label = "Local-MIP";
                let ok = self.try_incumbent(x);
                self.heur_label = "";
                eprintln!("MIP: Local-MIP solution accepted {ok} ({:.2}s)", self.start.elapsed().as_secs_f64());
            }
        }
    }

    /// 別スレッドの Feasibility Jump を止めて結果を受け取る。
    fn join_fj_thread(&mut self) {
        if let Some((h, stop)) = self.fj_thread.take() {
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            let r = h.join();
            // 受け渡し箱に残った解 (FJ のスレッドの Local-MIP) を先に受け取る
            self.poll_lmip_thread();
            if let Ok(Some(x)) = r {
                self.heur_label = "feasibility jump (parallel)";
                let ok = self.try_incumbent(x);
                self.heur_label = "";
                if ok && self.params.verbose {
                    eprintln!("MIP: feasibility jump (parallel) found a solution ({:.2}s)", self.start.elapsed().as_secs_f64());
                }
            }
        }
    }

    /// 木の探索のスレッド: 写しから解いた求解が見つけた解を受け取る (自分が作った写しの問題の空間のものだけ)。良くなったら真。
    fn take_inbox(&mut self) -> bool {
        use std::sync::atomic::Ordering;
        let Some(s) = self.shared.clone() else { return false };
        if self.snap.is_some() || self.snap_serials.is_empty() {
            return false;
        }
        let n = s.inbox_n.load(Ordering::Acquire);
        if n == self.inbox_seen {
            return false;
        }
        self.inbox_seen = n;
        let got: Vec<(Vec<f64>, &'static str, bool)> = {
            let mut ib = s.inbox.lock().unwrap();
            let mut keep = Vec::new();
            let mut mine = Vec::new();
            for (serial, x, label, role) in ib.drain(..) {
                if self.snap_serials.contains(&serial) {
                    mine.push((x, label, role));
                } else {
                    keep.push((serial, x, label, role));
                }
            }
            *ib = keep;
            mine
        };
        let mut better = false;
        for (x, label, role) in got {
            let zx = if x.len() == self.p.n { self.p.objective(&x) } else { f64::NAN };
            self.heur_label = match (role, label.is_empty()) {
                (true, true) => "other thread (heuristic role)",
                (true, false) => label,
                (false, true) => "other thread (tree)",
                (false, false) => label,
            };
            self.other_thread_label = !label.is_empty() || role;
            let ok = x.len() == self.p.n && self.try_incumbent(x);
            self.heur_label = "";
            self.other_thread_label = false;
            if ok {
                self.snap_stats[2] += 1;
                better = true;
            } else if self.incumbent.as_ref().is_none_or(|(z, _)| zx < *z - 1e-9 * z.abs().max(1.0)) {
                // 届いた解を受け付けられなかった (従で実行可能と判定した解が主で通らない)。最良値だけが下がり解が失われる
                eprintln!("MIP: WARNING: a solution from another thread was rejected (objective {zx:.10e}, len {})", self.p.n);
            }
        }
        better
    }

    /// 木の探索のスレッドの、待ち行列の頭での木の写しの処理: 写しから解いた求解の解を受け取り、二分探索のスレッドが示した
    /// 値でノードの下界を上げ (64 ノードおき)、写しを求められていれば作る。
    fn snap_sync(&mut self, plunge: &mut Option<OpenNode>) {
        use std::sync::atomic::Ordering;
        self.poll_lmip_thread();
        let Some(s) = self.shared.clone() else { return };
        self.take_inbox();
        if self.nodes % 64 == 0 && !self.snap_serials.is_empty() {
            let v = s.fb_ver.load(Ordering::Acquire);
            if v != self.fb_seen {
                self.fb_seen = v;
                let fb = s.fb.lock().unwrap().clone();
                let raise = |n: &mut OpenNode, cnt: &mut u64| {
                    if let Some(tags) = &n.tags {
                        let mut best = n.lower_bound;
                        for &t in tags.iter() {
                            if let Some(vals) = fb.get(&(t >> 32)) {
                                if let Some(&w) = vals.get((t & 0xffff_ffff) as usize) {
                                    best = best.max(w);
                                }
                            }
                        }
                        if best > n.lower_bound {
                            n.lower_bound = best;
                            n.estimate = n.estimate.max(best);
                            *cnt += 1;
                        }
                    }
                };
                let mut cnt = 0u64;
                let mut nodes = self.queue.take_all();
                for n in nodes.iter_mut() {
                    raise(n, &mut cnt);
                }
                for n in nodes {
                    self.queue.push(n);
                }
                if let Some(n) = plunge.as_mut() {
                    raise(n, &mut cnt);
                }
                self.dive_stack.clear();
                self.snap_stats[1] += cnt;
            }
        }
        if s.snap_req.load(Ordering::Acquire) && !self.unresolved {
            s.snap_req.store(false, Ordering::Release);
            self.make_snapshot(&s, plunge, true);
        }
    }

    /// 木の写しを作って共有する。待ち行列のノードと潜っている子に写しの中の番号を付ける (前の写しの番号は 3 つまで残す)。
    /// 根の解発見スレッド ([`find_thread_on`]) が終わっていれば、その解を受け取る (待たない)。
    fn poll_find_thread(&mut self) {
        // 主の暫定解が質の悪い FJ の解でなくなったら (FJ 以外の解が入った、または根の LP との相対ギャップが
        // `ENOMOTO_T_MIP_FIND_THREAD_STOP_GAP` (既定は `ENOMOTO_T_MIP_FJ_POOR_GAP` と同じ 0.5) 以下)、解発見スレッドを止める
        // (`ENOMOTO_MIP_FIND_THREAD_NO_STOP` で止めない)
        if self.find_thread.is_some() && env_str!("ENOMOTO_MIP_FIND_THREAD_NO_STOP").is_none() {
            let lb = self.fj_poor_lb;
            let thr = tunable!("ENOMOTO_T_MIP_FIND_THREAD_STOP_GAP", tunable!("ENOMOTO_T_MIP_FJ_POOR_GAP", 0.5, f64), f64);
            let poor = self.incumbent.as_ref().is_some_and(|(z, _)| self.inc_only_fj && lb.is_finite() && (z - lb) / z.abs().max(lb.abs()).max(1.0) > thr);
            if let Some(st) = self.find_stop.as_ref().filter(|_| !poor) {
                if !st.swap(true, std::sync::atomic::Ordering::Relaxed) && self.params.verbose {
                    eprintln!("MIP: find thread asked to stop (incumbent {:?}, {:.2}s)", self.incumbent.as_ref().map(|(z, _)| *z), self.start.elapsed().as_secs_f64());
                }
            }
        }
        let got: Vec<Vec<f64>> = self.find_out.as_ref().map(|o| std::mem::take(&mut *o.lock().unwrap())).unwrap_or_default();
        for x in got {
            let z0 = self.incumbent.as_ref().map(|(z, _)| *z);
            let label = std::mem::replace(&mut self.heur_label, "find thread");
            let acc = self.try_incumbent(x);
            self.heur_label = label;
            if self.params.verbose {
                eprintln!("MIP: find thread solution: accepted {acc} (incumbent {z0:?}) ({:.2}s, node {})", self.start.elapsed().as_secs_f64(), self.nodes);
            }
        }
        if !self.find_thread.as_ref().is_some_and(|h| h.is_finished()) {
            return;
        }
        let Some(h) = self.find_thread.take() else { return };
        if let Ok(r) = h.join() {
            let z0 = self.incumbent.as_ref().map(|(z, _)| *z);
            let label = std::mem::replace(&mut self.heur_label, "find thread");
            let acc = r.x.is_some_and(|x| self.try_incumbent(x));
            self.heur_label = label;
            if self.params.verbose {
                eprintln!("MIP: find thread done: objective {:?}, accepted {acc} (incumbent {z0:?}) ({:.2}s, node {})", r.objective, self.start.elapsed().as_secs_f64(), self.nodes);
            }
        }
    }

    fn make_snapshot(&mut self, s: &Portfolio, plunge: &mut Option<OpenNode>, with_nodes: bool) -> std::sync::Arc<TreeSnapshot> {
        use std::sync::atomic::Ordering;
        let serial = s.snap_serial.fetch_add(1, Ordering::AcqRel) + 1;
        let p = match &self.snap_p {
            Some(p) => p.clone(),
            None => {
                let p = std::sync::Arc::new(self.p.clone());
                self.snap_p = Some(p.clone());
                p
            }
        };
        let mut nodes = if with_nodes { self.queue.take_all() } else { Vec::new() };
        let mut snodes = Vec::with_capacity(nodes.len() + 1);
        let retag = |n: &mut OpenNode, k: usize| {
            let mut t = vec![serial << 32 | k as u64];
            if let Some(old) = &n.tags {
                t.extend(old.iter().take(3));
            }
            n.tags = Some(Rc::new(t));
        };
        for n in nodes.iter_mut() {
            retag(n, snodes.len());
            snodes.push(SnapNode { changes: n.changes.clone(), lower_bound: n.lower_bound, estimate: n.estimate, depth: n.depth, branch: n.branch, cut_basis: None, basis: None });
        }
        for n in nodes {
            self.queue.push(n);
        }
        if let Some(n) = plunge.as_mut().filter(|_| with_nodes) {
            retag(n, snodes.len());
            snodes.push(SnapNode { changes: n.changes.clone(), lower_bound: n.lower_bound, estimate: n.estimate, depth: n.depth, branch: n.branch, cut_basis: None, basis: None });
        }
        if with_nodes {
            self.dive_stack.clear();
        }
        let m0 = self.p.m;
        let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = (m0..self.lp.num_rows())
            .map(|i| {
                let (l, u) = self.lp.row_bounds(i);
                (self.lp.row(i), l, u)
            })
            .collect();
        let pool: Vec<(Vec<(usize, f64)>, f64)> = self.cut_pool.iter().map(|(c, r, _)| (c.clone(), *r)).collect();
        let orig = self.orig.as_ref().map(|o| std::sync::Arc::new((**o).clone()));
        let root_basis = if tree_root_basis() { Some(self.lp.basis()) } else { None };
        let snap = TreeSnapshot { serial, p, lo: self.dom.global_lo.clone(), up: self.dom.global_up.clone(), rows, pool, pc: self.pc.clone(), nodes: snodes, valid_below: self.prune_limit(), hcm_auto: self.hcm_auto, orbitopes: (*self.orbitopes).clone(), orig, lex_gens: (*self.lex_gens).clone(), root_basis };
        if self.params.verbose && env_str!("ENOMOTO_MIP_BISECT_VERBOSE").is_some() {
            eprintln!("MIP: tree snapshot {serial}: {} nodes, {} cut rows, valid below {:.10e} ({:.1}s)", snap.nodes.len(), snap.rows.len(), snap.valid_below, self.start.elapsed().as_secs_f64());
        }
        let snap = std::sync::Arc::new(snap);
        if with_nodes {
            *s.snap.lock().unwrap() = Some(snap.clone());
            self.snap_stats[0] += 1;
        }
        self.snap_serials.push(serial);
        snap
    }

    /// 木を分け合う従のスレッドを始める (根の処理を終えて木の探索に入るところで、主が呼ぶ)。従は根の後の写し (ノードなし)
    /// から始め、共有の待ち行列からノードを取る。見つけた解は主に渡す。
    fn start_workers(&mut self, plunge: &mut Option<OpenNode>) {
        // `ENOMOTO_MIP_THREAD_BUDGET`: 解探索のスレッドが生きている間は、木のスレッドを 1 本減らす
        let tt = if thread_budget_on() && self.find_out.is_some() && self.params.threads > 2 { self.params.threads - 1 } else { self.params.threads };
        let n = tt - 1;
        // 以後は木の探索が全てのスレッドを使うので、このスレッドの基底のない求解は同時実行にしない
        crate::simplex::mip_lp::set_cold_race_threads(1);
        let shared = match &self.shared {
            Some(s) => s.clone(),
            None => {
                let s = std::sync::Arc::new(Portfolio::new());
                if let Some((z, _)) = &self.incumbent {
                    s.publish(*z);
                }
                self.shared = Some(s.clone());
                s
            }
        };
        let pool = std::sync::Arc::new(WorkPool::new(tt, self.dom.global_lo.clone(), self.dom.global_up.clone(), self.pc.clone()));
        if let Some((z, x)) = &self.incumbent {
            *pool.best_x.lock().unwrap() = Some((*z, std::sync::Arc::new(x.clone())));
            pool.has_x.store(true, std::sync::atomic::Ordering::Release);
        }
        if pool.heur_k > 0 && !self.root_mu_obs.is_empty() {
            let mut g = pool.mu.lock().unwrap();
            for &(rel, dt) in &self.root_mu_obs {
                g.add(None, rel / dt);
            }
        }
        if self.params.verbose && pool.heur_k > 0 {
            eprintln!("MIP: {} of {} tree threads take the heuristic role once a solution exists", pool.heur_k, self.params.threads);
        }
        self.tprof.workers_s = self.start.elapsed().as_secs_f64();
        if prof_tree_par() {
            eprintln!(
                "TREEPAR: spawn at {:.2}s tree start {:.2}s nodes {} sb {:.2}s ({} LPs, {} iters) node LP {:.2}s ({} LPs, {} iters)",
                self.tprof.workers_s, self.tprof.tree_start_s, self.nodes, self.sb_secs, self.sb_lps, self.sb_iters, self.node_lp_secs, self.node_lps, self.node_iters
            );
        }
        self.pool = Some(pool.clone());
        self.pool_busy = true;
        self.pc_base = Some(self.pc.clone());
        self.pool_tid = pool.next_tid.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // racing ramp-up: 従は今の未処理のノードを全部持って始め、各スレッドが同じ木を別の順で探索する
        let racing = env_str!("ENOMOTO_MIP_TREE_RACING").is_some();
        pool.racing.store(racing, std::sync::atomic::Ordering::Relaxed);
        self.race_done = !racing;
        let snap = if racing { self.make_snapshot(&shared, plunge, true) } else { self.make_snapshot(&shared, &mut None, false) };
        // 従は主が止める (止める合図)。時間の上限は主より少し後にして、主より先に止まらないようにする。
        // 従の LP (ノードの LP、ヒューリスティクス役のサブ MIP の LP) は期限だけを見るので、主が止める合図を出しても期限
        // (主の 2 秒後) まで解き続けることがあり、主はそれを待つ (時間の上限の超過がほぼ 2.0 秒: physiciansched6-2・mzzv42z。
        // analysis/overrun_20261023.md)。`ENOMOTO_MIP_WORKER_HALT` なら止める合図を従の LP の反復にも伝える
        let halt = worker_halt().then(|| pool.halt.clone());
        let remaining = self.deadline.map_or(f64::INFINITY, |d| d.saturating_duration_since(Instant::now()).as_secs_f64() + 2.0);
        let params = MipParams { time_limit: remaining, node_limit: u64::MAX, verbose: false, threads: 1, ..self.params };
        let prof = crate::simplex::slope_intercept_dual::mip_profile();
        let dbg = DEBUG_SOL.with(|d| d.borrow().clone());
        for _ in 0..n {
            let (snap, pool, shared, dbg, halt) = (snap.clone(), pool.clone(), shared.clone(), dbg.clone(), halt.clone());
            let h = std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn(move || {
                    crate::simplex::slope_intercept_dual::set_mip_profile(prof);
                    set_thread_portfolio(Some(shared));
                    DEBUG_SOL.with(|d| *d.borrow_mut() = dbg);
                    let r = crate::cancel::with_halt(halt, || solve_worker(snap, Some(pool), params));
                    set_thread_portfolio(None);
                    r
                })
                .expect("failed to spawn a tree search thread");
            self.workers.push(h);
        }
        if self.params.verbose {
            eprintln!("MIP: tree search with {} threads ({:.2}s)", tt, self.start.elapsed().as_secs_f64());
        }
    }

    /// 自分の大域的な境界と共有の境界を突き合わせ、締まっている方に揃える (自分の方が締まっていれば共有の側を、共有の方が
    /// 締まっていれば自分の側を、大域的な締め付けとして)。締め付けは「打ち切り値未満の解はこれを満たす」もので、どのスレッドの
    /// 打ち切り値も共有の最良値から作るので、ほかのスレッドの締め付けで除かれるのは最良値を良くしない解だけ。
    fn sync_global_bounds(&mut self) {
        let Some(pool) = self.pool.clone() else { return };
        let mut pulled = Vec::new();
        {
            let mut g = pool.bounds.lock().unwrap();
            let (lo, up) = &mut *g;
            for j in 0..self.p.n {
                let (ml, mu) = (self.dom.global_lo[j], self.dom.global_up[j]);
                if ml > lo[j] {
                    lo[j] = ml;
                } else if lo[j] > ml {
                    pulled.push((j, false, lo[j]));
                }
                if mu < up[j] {
                    up[j] = mu;
                } else if up[j] < mu {
                    pulled.push((j, true, up[j]));
                }
            }
        }
        self.pool_stats[2] += pulled.len() as u64;
        for (j, upper, v) in pulled {
            self.dom.tighten_global(self.p, j, upper, v);
        }
    }

    /// 待っているスレッド `want` 本に、自分の待ち行列の下界の小さいノードを渡す (自分の待ち行列の半分まで)。
    fn pool_donate(&mut self, want: usize) {
        use std::sync::atomic::Ordering;
        let Some(pool) = self.pool.clone() else { return };
        let k = want.min(self.queue.len() / 2).max(1);
        let mut out = Vec::with_capacity(k);
        for _ in 0..k {
            match self.queue.pop_best() {
                Some(n) => out.push(self.snap_of(&n)),
                None => break,
            }
        }
        self.pool_stats[0] += out.len() as u64;
        let mut g = pool.inner.lock().unwrap();
        g.nodes.extend(out);
        pool.len.store(g.nodes.len(), Ordering::Relaxed);
    }

    /// 自分の待ち行列から下界の小さい順に `k` ノードを共有の待ち行列に出す (`ENOMOTO_T_MIP_TREE_LOCAL_CAP`)。
    fn pool_donate_n(&mut self, k: usize) {
        use std::sync::atomic::Ordering;
        let Some(pool) = self.pool.clone() else { return };
        let mut out = Vec::with_capacity(k);
        for _ in 0..k {
            match self.queue.pop_best() {
                Some(n) => out.push(self.snap_of(&n)),
                None => break,
            }
        }
        self.pool_stats[0] += out.len() as u64;
        let mut g = pool.inner.lock().unwrap();
        g.nodes.extend(out);
        pool.len.store(g.nodes.len(), Ordering::Relaxed);
    }

    /// 今の潜りの兄弟 (`dive_stack`) 以外のノードを共有の待ち行列に出す (`ENOMOTO_MIP_TREE_KEEP_SIBLINGS`)。
    fn pool_donate_except_siblings(&mut self) {
        use std::sync::atomic::Ordering;
        let Some(pool) = self.pool.clone() else { return };
        let keep: Vec<(usize, u64)> = self.dive_stack.iter().map(|&(id, gen, _)| (id, gen)).collect();
        let nodes = self.queue.take_all_except(&keep);
        if nodes.is_empty() {
            return;
        }
        let out: Vec<SnapNode> = nodes.iter().map(|n| self.snap_of(n)).collect();
        self.pool_stats[0] += out.len() as u64;
        let mut g = pool.inner.lock().unwrap();
        g.nodes.extend(out);
        pool.len.store(g.nodes.len(), Ordering::Relaxed);
    }

    /// 自分の待ち行列のノードを全て共有の待ち行列に出す (`ENOMOTO_MIP_TREE_SHARED_QUEUE`)。
    fn pool_donate_all(&mut self) {
        use std::sync::atomic::Ordering;
        let Some(pool) = self.pool.clone() else { return };
        let mut out = Vec::with_capacity(self.queue.len());
        while let Some(n) = self.queue.pop_best() {
            out.push(self.snap_of(&n));
        }
        self.pool_stats[0] += out.len() as u64;
        let mut g = pool.inner.lock().unwrap();
        g.nodes.extend(out);
        pool.len.store(g.nodes.len(), Ordering::Relaxed);
    }

    /// 共有の待ち行列から取ったノードを自分のノードにする。親の基底が付いていれば (`ENOMOTO_MIP_TREE_NO_PASS_BASIS` でなければ付ける)、列と元の行は
    /// その状態、このスレッドのカットの行は基底にした基底を付ける (基底変数の数が合わなければ [`Self::restore_node_basis`] の
    /// 修復に任せる)。
    fn open_from_pool(&mut self, mut sn: SnapNode) -> OpenNode {
        let basis = sn.basis.take();
        let cut_basis = sn.cut_basis.take();
        let mut n = sn.into_open();
        if let Some((col, mut row)) = basis {
            let mr = self.lp.num_rows();
            if col.len() == self.p.n && row.len() <= mr {
                row.resize(mr, VarStatus::Basic);
                // 渡した側で非基底だったカットと同じカットの行があれば、その状態にする (`ENOMOTO_MIP_TREE_SHARE_CUT_BASIS`)
                if let Some(cb) = cut_basis {
                    self.refresh_row_hashes();
                    let mut hit = 0u64;
                    for (h, st) in cb {
                        if let Some(&i) = self.row_hash_map.get(&h) {
                            if i < mr {
                                row[i] = st;
                                hit += 1;
                            }
                        }
                    }
                    self.cut_basis_stats[1] += hit;
                }
                self.pool_excess = col.iter().chain(row.iter()).filter(|&&s| s == VarStatus::Basic).count() as i64 - mr as i64;
                // `ENOMOTO_MIP_TREE_POOL_BASIS` (試作): 渡した側で非基底だったカットの行がこちらに無く、基底変数が余るときは、
                // 受け取った基底を使わず今の LP の基底 (直前のノードの最適基底。双対実行可能) から解く。余った基底を
                // set_basis の修復 (構造変数を後ろから外す) に任せると、最初の LP が根から解くのと同じくらいかかる
                // (roll3000、4 スレッド・60 秒: 余りが 1 以上の 21 回は 1 回 2000〜5600 反復、余りが 0 なら 54〜198 反復)
                if self.pool_excess > 0 && tree_pool_basis() {
                    self.pool_basis_stats += 1;
                    return n;
                }
                n.basis = Some(Rc::new(super::lp::Basis { col, row }));
                n.basis_epoch = self.row_log.len();
            }
        }
        n
    }

    /// 共有の待ち行列に出すノード ([`SnapNode`])。親の基底の列と元の行 (`ENOMOTO_MIP_TREE_NO_PASS_BASIS` でなければ) と、
    /// `ENOMOTO_MIP_TREE_SHARE_CUT_BASIS` なら非基底だったカットの行の (ハッシュ, 状態) を付ける。
    fn snap_of(&mut self, n: &OpenNode) -> SnapNode {
        if env_str!("ENOMOTO_MIP_TREE_NO_PASS_BASIS").is_some() {
            return SnapNode::of(n);
        }
        let mut s = SnapNode::of_with_basis(n, self.p.m);
        if s.basis.is_some() && (env_str!("ENOMOTO_MIP_TREE_SHARE_CUT_BASIS").is_some() || tree_pool_basis()) {
            if let Some(b) = n.basis.clone() {
                // 保存したときの行を今の行に合わせる (行の追加・削除の記録を当てはめる。[`Self::restore_node_basis`] と同じ)
                let mut row = b.row.clone();
                if n.basis_epoch <= self.row_log.len() {
                    for e in &self.row_log[n.basis_epoch..] {
                        match e {
                            RowEdit::Add(k) => row.extend(std::iter::repeat_n(VarStatus::Basic, *k)),
                            RowEdit::Delete(mask) => {
                                let mut i = 0;
                                row.retain(|_| {
                                    let keep = !mask.get(i).copied().unwrap_or(false);
                                    i += 1;
                                    keep
                                });
                            }
                        }
                    }
                }
                let mr = self.lp.num_rows();
                if row.len() == mr {
                    self.refresh_row_hashes();
                    let cb: Vec<(u64, VarStatus)> = (self.p.m..mr).filter(|&i| row[i] != VarStatus::Basic).map(|i| (self.row_hash[i], row[i])).collect();
                    self.cut_basis_stats[0] += cb.len() as u64;
                    s.cut_basis = Some(cb);
                }
            }
        }
        s
    }

    /// LP のカットの行のハッシュ (係数と右辺を最大の係数で割って丸めたもの) と、ハッシュから行への表を、行が変わっていれば作り直す。
    fn refresh_row_hashes(&mut self) {
        use std::hash::{Hash, Hasher};
        let mr = self.lp.num_rows();
        if self.row_hash_epoch == Some(self.row_log.len()) && self.row_hash.len() == mr {
            return;
        }
        self.row_hash.clear();
        self.row_hash_map.clear();
        for i in 0..mr {
            if i < self.p.m {
                self.row_hash.push(0);
                continue;
            }
            let coefs = self.lp.row(i);
            let (lo, up) = self.lp.row_bounds(i);
            let cmax = coefs.iter().fold(0.0f64, |m, &(_, v)| m.max(v.abs())).max(1e-300);
            let mut hs = std::collections::hash_map::DefaultHasher::new();
            for &(j, v) in &coefs {
                j.hash(&mut hs);
                ((v / cmax) * 1e9).round().to_bits().hash(&mut hs);
            }
            for v in [lo, up] {
                if v.is_finite() { ((v / cmax) * 1e9).round().to_bits() } else { v.to_bits() }.hash(&mut hs);
            }
            let h = hs.finish();
            self.row_hash.push(h);
            self.row_hash_map.entry(h).or_insert(i);
        }
        self.row_hash_epoch = Some(self.row_log.len());
    }

    /// racing ramp-up の進行 (競争中のスレッドがノードごとに呼ぶ)。主は競争の時間 (`ENOMOTO_T_MIP_TREE_RACE_SECS`) が過ぎたら、
    /// 自分の木の下界 (未処理のノードの最小) が最も大きいスレッドを勝者にする。自分の木を探索し終えたスレッドは (全体を
    /// 解き終えたので) 自分を勝者にする。勝者が決まったら、負けたスレッドは自分の木を捨てて共有の待ち行列から取る (勝者は
    /// 待っているスレッドにノードを渡す)。木を捨てたら真。
    fn race_step(&mut self, plunge: &mut Option<OpenNode>) -> bool {
        use std::sync::atomic::Ordering;
        let Some(pool) = self.pool.clone() else { return false };
        let t = self.pool_tid as i64;
        if pool.race_winner.load(Ordering::Acquire) < 0 {
            let exhausted = self.queue.len() == 0 && plunge.is_none();
            let timeup = self.snap.is_none() && pool.t0.elapsed().as_secs_f64() >= tunable!("ENOMOTO_T_MIP_TREE_RACE_SECS", 5.0, f64);
            if exhausted {
                let _ = pool.race_winner.compare_exchange(-1, t, Ordering::AcqRel, Ordering::Acquire);
            } else if timeup {
                let mut best = (f64::NEG_INFINITY, t);
                for (i, sl) in pool.slots.iter().enumerate() {
                    let v = f64::from_bits(sl.lb.load(Ordering::Relaxed));
                    if v > best.0 {
                        best = (v, i as i64);
                    }
                }
                let _ = pool.race_winner.compare_exchange(-1, best.1, Ordering::AcqRel, Ordering::Acquire);
            } else {
                return false;
            }
        }
        let w = pool.race_winner.load(Ordering::Acquire);
        if prof_tree_par() {
            eprintln!("TREEPAR: race tid {t} sees winner {w} at {:.2}s (own lb {:.6e}, open {})", pool.t0.elapsed().as_secs_f64(), self.queue.best_lower_bound(), self.queue.len() + plunge.is_some() as usize);
        }
        self.race_done = true;
        pool.racing.store(false, Ordering::Relaxed);
        // 写しのノードの部分木を探索し終えた知らせ ([`Self::snap_report`]) を止める: この後は木を捨てたり (負け)、ノードを
        // ほかのスレッドに渡したり (勝ち) して、待ち行列から写しのノードの子孫が消えても探索し終えたとは限らない
        // (知らせると主がその部分木を刈ってしまう。neos-860300 で 3479 を最適と誤って報告した)
        self.snap_alive.iter_mut().for_each(|a| *a = false);
        if w == t {
            return false;
        }
        // 負け: 自分の木を捨てる (勝者の木が全体を覆っている)
        let _ = self.queue.take_all();
        *plunge = None;
        self.dive_stack.clear();
        if let Some(sl) = pool.slots.get(t as usize) {
            sl.lb.store(f64::INFINITY.to_bits(), Ordering::Relaxed);
            sl.open.store(0, Ordering::Relaxed);
        }
        true
    }

    /// 質の釣り合い (既定。`ENOMOTO_MIP_TREE_NO_BALANCE` で無効): 共有の待ち行列に自分の下界より十分小さいノードがあれば取って自分の待ち行列に
    /// 入れる。ほかのスレッドの下界が自分の待ち行列の最良より十分大きく、共有の待ち行列が少なければ、最良のノードを渡す。
    /// 「十分」は q (`ENOMOTO_T_MIP_TREE_BALANCE_Q`) × (打ち切り値 − 全体の下界) (打ち切り値がなければ |下界|)。
    fn pool_balance(&mut self) {
        use std::sync::atomic::Ordering;
        let Some(pool) = self.pool.clone() else { return };
        if !self.race_done {
            return;
        }
        let t = self.pool_tid as usize;
        let mine = self.queue.best_lower_bound();
        let me = mine.min(self.cur_lb);
        let mut others_min = f64::INFINITY;
        let mut others_max = f64::NEG_INFINITY;
        for (i, sl) in pool.slots.iter().enumerate() {
            if i == t {
                continue;
            }
            let v = f64::from_bits(sl.lb.load(Ordering::Relaxed));
            if v.is_finite() {
                others_min = others_min.min(v);
                others_max = others_max.max(v);
            }
        }
        let cutoff = self.prune_limit();
        let mut g = pool.inner.lock().unwrap();
        let pool_min = g.nodes.min_lower_bound();
        let all_min = me.min(others_min).min(pool_min);
        if !all_min.is_finite() {
            return;
        }
        let width = if cutoff.is_finite() { (cutoff - all_min).max(1e-9) } else { all_min.abs().max(1.0) };
        let thr = tunable!("ENOMOTO_T_MIP_TREE_BALANCE_Q", 0.2, f64) * width;
        if let Some(sn) = if pool_min < me - thr { g.nodes.pop_best() } else { None } {
            pool.len.store(g.nodes.len(), Ordering::Relaxed);
            // 待っている側に数えられたまま (ヒューリスティクス役から木に戻った直後) ノードを取るときは、待っていない側に戻す
            // ([`Self::pool_acquire`] と同じ)。戻さないと、このスレッドがノードを持っている間にほかのスレッドが「全員が待っていて
            // 共有の待ち行列も空」と判定して探索を終え、このスレッドの部分木が失われる (mc11 で 11712 を最適と誤って報告した
            // 経路の候補。analysis/partree_race_20261021.md)
            if !self.pool_busy {
                if tree_diag() {
                    eprintln!("MIP_TREE_DIAG: tid {} took a node (lower bound {:.10e}) by balancing while counted as waiting (busy {}, pool {})", self.pool_tid, sn.lower_bound, g.busy, g.nodes.len());
                }
                if env_str!("ENOMOTO_MIP_TREE_NO_BALANCE_BUSY_FIX").is_none() {
                    g.busy += 1;
                    pool.idle.fetch_sub(1, Ordering::Relaxed);
                    self.pool_busy = true;
                }
            }
            drop(g);
            if let Some(sl) = pool.slots.get(t) {
                sl.lb.store(me.min(sn.lower_bound).to_bits(), Ordering::Relaxed);
            }
            let n = self.open_from_pool(sn);
            self.queue.push(n);
            self.pool_stats[1] += 1;
            self.tprof.balance[0] += 1;
            return;
        }
        let few = g.nodes.len() < pool.threads;
        // `ENOMOTO_T_MIP_TREE_BALANCE_KEEP` = K (> 0): UG の collect mode のように、共有の待ち行列に下界の良い (全体の下界 +
        // しきい値以下の) ノードを K 個は置いておく (良いノードを持つスレッドが出す)
        let keep = tunable!("ENOMOTO_T_MIP_TREE_BALANCE_KEEP", 0usize, usize);
        let good = if keep > 0 { g.nodes.iter().filter(|n| n.lower_bound <= all_min + thr).count() } else { 0 };
        drop(g);
        let donate = if keep > 0 { good < keep && mine <= all_min + thr } else { few && others_max > mine + thr };
        if donate && self.queue.len() >= 2 && self.pool_hold.is_none() {
            self.pool_donate(1);
            self.tprof.balance[1] += 1;
        }
    }

    /// 自分の待ち行列が空のとき、共有の待ち行列からノードを取る。なければ待つ。全てのスレッドが待っていて空なら探索は終わり
    /// (`Ok(None)`)。待っている間に時間切れ・止める合図なら `Err`。
    fn pool_acquire(&mut self) -> Result<Option<OpenNode>, ()> {
        use std::sync::atomic::Ordering;
        let Some(pool) = self.pool.clone() else { return Ok(None) };
        let t_wait = Instant::now();
        if self.tprof.ready_s == 0.0 && self.snap.is_some() {
            self.tprof.ready_s = pool.t0.elapsed().as_secs_f64();
        }
        let mut waited = false;
        // 共有の待ち行列だけで探索するときは、自分の待ち行列と同じ選び方で取る
        let shared_sel = (tree_shared_queue() && self.snap.is_some() || tree_keep_siblings()) && self.race_done;
        let bb_every = if self.in_proof_phase() {
            tunable!("ENOMOTO_T_MIP_PROOF_BB_EVERY", 1u64, u64)
        } else if self.has_cutoff_value() {
            tunable!("ENOMOTO_T_MIP_BB_EVERY_INC", 4u64, u64)
        } else {
            tunable!("ENOMOTO_T_MIP_BB_EVERY", 10u64, u64)
        };
        let r = loop {
            {
                let t_lock = Instant::now();
                let mut g = pool.inner.lock().unwrap();
                self.tprof.pool_len_sum += g.nodes.len() as u64;
                let sl = pool.slots.get(self.pool_tid as usize);
                if let Some(sl) = sl {
                    sl.open.store(0, Ordering::Relaxed);
                }
                let got = if shared_sel { g.nodes.pop_hybrid(bb_every) } else { g.nodes.pop_best() };
                if let Some(sn) = got {
                    // 受け取ったノードの下界を同じ錠の中で書く (共有の待ち行列から消えた後も全体の下界に入るように)
                    if let Some(sl) = sl {
                        sl.lb.store(sn.lower_bound.to_bits(), Ordering::Relaxed);
                    }
                    pool.len.store(g.nodes.len(), Ordering::Relaxed);
                    if !self.pool_busy {
                        g.busy += 1;
                        pool.idle.fetch_sub(1, Ordering::Relaxed);
                        self.pool_busy = true;
                    }
                    self.pool_stats[1] += 1;
                    let n = self.open_from_pool(sn);
                    self.tprof.lock_s += t_lock.elapsed().as_secs_f64();
                    break Ok(Some(n));
                }
                self.tprof.lock_s += t_lock.elapsed().as_secs_f64();
                // ノードを持っていない
                if let Some(sl) = sl {
                    sl.lb.store(f64::INFINITY.to_bits(), Ordering::Relaxed);
                }
                if self.pool_busy {
                    g.busy -= 1;
                    pool.idle.fetch_add(1, Ordering::Relaxed);
                    self.pool_busy = false;
                }
                if g.busy == 0 {
                    g.done = true;
                    pool.fin.store(true, Ordering::Release);
                }
                if g.done {
                    break Ok(None);
                }
            }
            if self.time_up() {
                break Err(());
            }
            waited = true;
            let t_sleep = Instant::now();
            std::thread::sleep(Duration::from_micros(200));
            self.tprof.sleep_s += t_sleep.elapsed().as_secs_f64();
        };
        let w = t_wait.elapsed().as_secs_f64();
        if matches!(r, Ok(Some(_))) {
            self.tprof.idle_s += w;
            self.tprof.waits += waited as u64;
        } else {
            self.tprof.tail_s += w;
        }
        r
    }

    /// 主: 従を止めて待ち、結果を合わせる。戻り値は従の下界と、共有の待ち行列に残ったノードの下界の最小 (なければ +inf)。
    fn join_workers(&mut self) -> f64 {
        use std::sync::atomic::Ordering;
        let Some(pool) = self.pool.clone() else { return f64::INFINITY };
        if self.workers.is_empty() {
            return f64::INFINITY;
        }
        pool.stop.store(true, Ordering::Relaxed);
        pool.halt.store(true, Ordering::Relaxed);
        let mut lb = f64::INFINITY;
        // 従が終わったときの解 (従の問題は主の問題の写しなので、主の解としてそのまま使える)
        let mut ended: Vec<(f64, Option<Vec<f64>>)> = Vec::new();
        let t_stop = Instant::now();
        for (k, h) in std::mem::take(&mut self.workers).into_iter().enumerate() {
            let jr = h.join();
            if self.params.verbose || end_diag() {
                eprintln!("MIP: tree thread {k} joined {:.3}s after the stop signal ({:.2}s)", t_stop.elapsed().as_secs_f64(), self.start.elapsed().as_secs_f64());
            }
            match jr {
                Ok(r) => {
                    if tree_diag() {
                        eprintln!("MIP_TREE_DIAG: worker ended: status {:?} objective {:?} bound {:.10e} nodes {}", r.status, r.objective, r.best_bound, r.nodes);
                    }
                    if let Some(zw) = r.objective {
                        ended.push((zw, r.x));
                    }
                    self.worker_totals.0 += r.nodes;
                    self.worker_totals.1 += r.lp_iterations;
                    if !matches!(r.status, MipStatus::Optimal | MipStatus::Infeasible) {
                        lb = lb.min(r.best_bound);
                    }
                }
                Err(_) => {
                    // 従が異常終了した: 探索し残した部分があるかもしれない
                    self.unresolved = true;
                    lb = f64::NEG_INFINITY;
                }
            }
        }
        if pool.unresolved.load(Ordering::Relaxed) {
            self.unresolved = true;
        }
        // 従が見つけた解は受信箱 (`take_inbox`) で主に届く。主が共有の待ち行列を待っている間 (`pool_acquire`) は受信箱を見ないので、
        // 探索の終わり際に従 (ヒューリスティクス役の従の ALNS など) が見つけた解はここまで受信箱に残っている。従の解と主の
        // 暫定解を比べるのは受信箱を受け取ってから (以前はその前に比べていて、正しく届く解に「従が主より良い解で終わった」の
        // 警告を出していた: h80x6320d、analysis/h80x6320d_warning_20261021.md)。受け取っても従の方が良ければ (受信箱で解が
        // 拒まれた・失われた)、従が返した解そのものを暫定解に試し、それでも悪ければ警告する
        if self.take_inbox() && tree_diag() {
            eprintln!("MIP_TREE_DIAG: a better solution was taken from the inbox after joining the workers (incumbent {:?})", self.incumbent.as_ref().map(|(z, _)| *z));
        }
        for (zw, x) in ended {
            let better = |inc: &Option<(f64, Vec<f64>)>| inc.as_ref().is_none_or(|(z, _)| zw < *z - 1e-6 * z.abs().max(1.0));
            if !better(&self.incumbent) {
                continue;
            }
            if let Some(x) = x {
                self.heur_label = "other thread (tree, at the end)";
                let ok = self.try_incumbent(x);
                self.heur_label = "";
                if ok && !better(&self.incumbent) {
                    continue;
                }
            }
            let z = self.incumbent.as_ref().map_or(f64::INFINITY, |(z, _)| *z);
            eprintln!("MIP: WARNING: a tree thread ended with a better solution {zw:.10e} than the main thread's {z:.10e}");
        }
        let g = pool.inner.lock().unwrap();
        lb.min(g.nodes.min_lower_bound())
    }

    /// 木の写しから解く求解: 部分木を探索し終えた写しのノード (待ち行列にも潜っている子にも、そのノードの子孫が残って
    /// いない) に、今の打ち切り値 (打ち切り値は下がるだけなので、それまでに刈ったノードもこの値未満の解を持たない) を
    /// 知らせる。待ち行列の頭 (処理中のノードがないとき) で呼ぶ。
    fn snap_report(&mut self, plunge: Option<&OpenNode>) {
        use std::sync::atomic::Ordering;
        let (Some(snap), Some(s)) = (self.snap.clone(), self.shared.clone()) else { return };
        if self.unresolved {
            return;
        }
        let k = snap.nodes.len();
        let mut alive = vec![false; k];
        let mark = |n: &OpenNode, alive: &mut Vec<bool>| {
            if let Some(t) = n.tags.as_ref().and_then(|t| t.first()) {
                if t >> 32 == snap.serial {
                    let i = (t & 0xffff_ffff) as usize;
                    if i < alive.len() {
                        alive[i] = true;
                    }
                }
            }
        };
        for n in self.queue.iter() {
            mark(n, &mut alive);
        }
        if let Some(n) = plunge {
            mark(n, &mut alive);
        }
        let v = self.prune_limit();
        let done: Vec<usize> = (0..k).filter(|&i| self.snap_alive[i] && !alive[i]).collect();
        if done.is_empty() || !v.is_finite() {
            return;
        }
        {
            let mut fb = s.fb.lock().unwrap();
            let e = fb.entry(snap.serial).or_insert_with(|| vec![f64::NEG_INFINITY; k]);
            for &i in &done {
                e[i] = e[i].max(v);
                self.snap_alive[i] = false;
            }
        }
        s.fb_ver.fetch_add(1, Ordering::AcqRel);
    }

    /// 根の緩和問題が非有界 (二段解法の段階 A が `z^1 < 0`、段階 B で実行可能) のときの整数最適化の結論。区別を求められて
    /// いなければ `InfeasibleOrUnbounded`。求められていれば、暫定解がすでにあれば非有界 (実行可能解と、緩和問題の目的値を
    /// 下げる有理数の方向 `x^1` がある)。なければ切片問題 (論文の式 (8)。傾き `x^1` を固定し、その傾きの境界上にない側の
    /// 境界を捨てた問題) を整数に制限して解く: 実行可能解 `x^0` があれば `x^0 + M x^1` (`M` は `x^1` の整数列の分母の倍数)
    /// が目的値をいくらでも下げる整数解なので非有界、実行不能なら元の整数最適化も実行不能 (元の実行可能解 `x` から
    /// `x - M x^1` が切片問題の整数解になる)。切片問題の緩和は段階 B の双対実行可能な基底から解けるので下に有界。
    /// 切片問題は実行可能性だけを問うので、目的を 0 にして解く (最初の整数解で終わる)。
    fn resolve_unbounded_root(&mut self) -> MipStatus {
        if !self.params.distinguish || self.params.submip {
            return MipStatus::InfeasibleOrUnbounded;
        }
        if self.incumbent.is_some() {
            return MipStatus::Unbounded;
        }
        let p = self.p;
        let Some((col_lo, col_up, row_lo, row_up)) = self.lp.intercept_bounds().filter(|b| b.0.len() == p.n && b.2.len() >= p.m) else {
            return MipStatus::InfeasibleOrUnbounded;
        };
        let mut sub = p.clone();
        for j in 0..p.n {
            let (mut lo, mut up) = (col_lo[j], col_up[j]);
            if p.is_int[j] {
                lo = (lo - 1e-9).ceil();
                up = (up + 1e-9).floor();
            }
            if lo > up {
                return MipStatus::Infeasible;
            }
            sub.col_lo[j] = lo;
            sub.col_up[j] = up;
        }
        sub.row_lo.copy_from_slice(&row_lo[..p.m]);
        sub.row_up.copy_from_slice(&row_up[..p.m]);
        // 知りたいのは整数の実行可能解があるかだけなので、目的を 0 にして実行可能性の問題として解く (最初の整数解で
        // 下界 0 = 暫定解 0 となり探索が終わる。元の目的のままだと最適性を示すまで木を探索していた)。
        // `ENOMOTO_MIP_INTERCEPT_KEEP_OBJ` なら以前どおり元の目的で解く
        if env_str!("ENOMOTO_MIP_INTERCEPT_KEEP_OBJ").is_none() {
            sub.cost.iter_mut().for_each(|c| *c = 0.0);
            sub.offset = 0.0;
        }
        let remaining = self.deadline.map_or(f64::INFINITY, |d| d.saturating_duration_since(Instant::now()).as_secs_f64());
        let params = MipParams { time_limit: remaining, distinguish: false, restarts: 0, skip_heurs: 0, cutoff: f64::INFINITY, ..self.params };
        if self.params.verbose {
            eprintln!("MIP: root relaxation unbounded; solving the intercept problem with integrality ({:.2}s)", self.start.elapsed().as_secs_f64());
        }
        user_log!("Solving the intercept problem with integer variables to tell infeasible from unbounded");
        let r = crate::log::suspend(|| super::solve_problem(&sub, params, env_str!("ENOMOTO_MIP_NO_PRESOLVE").is_none()));
        if self.params.verbose {
            eprintln!("MIP: intercept problem {:?} ({:.2}s)", r.status, self.start.elapsed().as_secs_f64());
        }
        match r.status {
            _ if r.x.is_some() => MipStatus::Unbounded,
            MipStatus::Infeasible => MipStatus::Infeasible,
            MipStatus::TimeLimit => MipStatus::TimeLimit,
            _ => MipStatus::InfeasibleOrUnbounded,
        }
    }

    fn finish(&mut self, status: MipStatus, best_bound: f64) -> MipResult {
        if (self.params.verbose || end_diag()) && !self.params.submip && self.snap.is_none() {
            eprintln!("MIP: finish start {status:?} ({:.2}s)", self.start.elapsed().as_secs_f64());
        }
        // `ENOMOTO_MIP_SYM_ORIG_CUT`: 打ち切り値にだけ使った解が暫定解より良ければ、それを答えにする (元の問題で実行可能)
        if let Some((zo, xo)) = self.orig_best.take() {
            if self.incumbent.as_ref().is_none_or(|(z, _)| zo < *z) {
                if self.params.verbose {
                    eprintln!("MIP: returning the solution feasible only without the symmetry rows ({zo:.10e})");
                }
                self.incumbent = Some((zo, xo));
            }
        }
        if lbnode_diag() && !self.params.submip && self.lbd.n.iter().sum::<u64>() > 0 {
            if let Some((pk, t)) = self.lbd.last.take() {
                self.lbd.secs[pk] += t.elapsed().as_secs_f64();
            }
            let d = &self.lbd;
            eprintln!(
                "LBNODE: tid {} restarts {} (kind: nodes/at global min/secs) plunge {}/{}/{:.1} sibling {}/{}/{:.1} queue-min {}/{}/{:.1} queue-est {}/{}/{:.1} pool {}/{}/{:.1}",
                self.pool_tid, self.params.restarts, d.n[0], d.at_min[0], d.secs[0], d.n[1], d.at_min[1], d.secs[1], d.n[2], d.at_min[2], d.secs[2], d.n[3], d.at_min[3], d.secs[3], d.n[4], d.at_min[4], d.secs[4]
            );
        }
        // 診断用: 従は自分の内訳を主に渡す (主は従を待った後に表示する)
        if prof_tree_par() {
            self.tprof.lits = self.lit_conflicts.stats;
            self.tprof.lits_live = self.lit_conflicts.live_by_origin();
            self.tprof.proof_live = [self.proof_fifo[0].len(), self.proof_fifo[1].len()];
        }
        if let (Some(pool), true) = (self.pool.clone(), self.snap.is_some() && prof_tree_par()) {
            self.tprof.tid = self.pool_tid;
            self.tprof.end_s = pool.t0.elapsed().as_secs_f64();
            self.tprof.lns_s = self.lns_secs;
            if self.cut_basis_stats[0] + self.cut_basis_stats[1] > 0 {
                eprintln!("TREEPAR: tid {} cut basis: sent {} matched {} (backtracks to siblings {}), received bases not used {}", self.pool_tid, self.cut_basis_stats[0], self.cut_basis_stats[1], self.sibling_backtracks, self.pool_basis_stats);
            }
            pool.prof.lock().unwrap().push(std::mem::take(&mut self.tprof));
        }
        // 木を分け合う主: 従を止めて待ち、従と共有の待ち行列の下界を合わせる。従: 待っていない側から外れる
        let wlb = self.join_workers();
        // 時間切れなどで終わるとき、下界は根の LP の値 (`root_lb`、この問題全体の下界として常に正しい) より下げない。
        // 従のスレッドが写しの根の LP で時間切れになって -inf を返す・再スタート後の求解が暫定解なしで終わるなどで
        // 下界を失い、根の下界が分かっているのに -inf を返していた (mzzv42z・physiciansched6-2 の 4 スレッド・60 秒。
        // analysis/bound_inf_20261021.md)。暫定解があればその値で上から抑える (根の下界が暫定解の値をわずかに越えうる)
        // 上限で終わるときの下界 (暫定解で抑える前。下の上限で終わるときの判定に使う)
        let raw_bound = best_bound.min(wlb).max(self.root_lb).max(self.root_cut_lb);
        let best_bound = if matches!(status, MipStatus::Optimal | MipStatus::Infeasible) {
            best_bound
        } else {
            best_bound.min(wlb).max(self.root_lb).min(self.incumbent.as_ref().map_or(f64::INFINITY, |(z, _)| *z))
        };
        if let Some(pool) = self.pool.clone() {
            if self.unresolved && self.snap.is_some() {
                pool.unresolved.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            if self.pool_busy {
                pool_release(&pool);
                self.pool_busy = false;
            }
            if self.params.verbose {
                eprintln!("MIP: tree search threads: worker nodes {}, nodes given away {}, taken {}, global bounds taken from other threads {}, proofs {} and conflicts {} taken from other threads", self.worker_totals.0, self.pool_stats[0], self.pool_stats[1], self.pool_stats[2], self.learned_stats[0], self.learned_stats[1]);
            }
        }
        // 木の写しから解いた求解が見つけた解を受け取る (止める合図で終わるときも)
        let best_bound = if self.take_inbox() { best_bound.min(self.incumbent.as_ref().map_or(f64::INFINITY, |(z, _)| *z)) } else { best_bound };
        if prof_tree_par() && self.snap.is_none() && !self.params.submip && self.watch.is_none() {
            let z = self.incumbent.as_ref().map(|(z, _)| *z);
            let total = self.start.elapsed().as_secs_f64();
            let mut me = std::mem::take(&mut self.tprof);
            me.tid = self.pool_tid;
            let mut rows = Vec::new();
            if let Some(pool) = &self.pool {
                me.end_s = pool.t0.elapsed().as_secs_f64();
                rows = std::mem::take(&mut *pool.prof.lock().unwrap());
            }
            eprintln!("TREEPAR: total {total:.2}s tree start {:.2}s workers start {:.2}s threads {} status {status:?} obj {z:?} bound {best_bound:.10e}", me.tree_start_s, me.workers_s, self.params.threads);
            rows.insert(0, me);
            rows.sort_by_key(|r| r.tid);
            for r in &rows {
                eprintln!("TREEPAR: {}", r.line(z));
            }
        }
        if self.params.verbose && self.snap_stats[0] > 0 {
            eprintln!("MIP: tree snapshots {}: node lower bounds raised {}, solutions received {}", self.snap_stats[0], self.snap_stats[1], self.snap_stats[2]);
        }
        // 並列モードの別スレッドを止める・待つ
        if let Some((_, stop)) = &self.fj_thread {
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.parallel_submips = false;
        self.join_submips(0);
        self.final_lb = Some(best_bound);
        self.log(true);
        if self.params.verbose {
            eprintln!("MIP: dual proofs and conflicts {} (conflicts added {} (from proofs {}), Farkas {}, pruned {} nodes, tightened {} bounds, rejected as numerically unreliable {})", self.dual_proofs.len(), self.conflicts_added, self.proof_conflicts, self.farkas_added, self.proof_prunes, self.proof_tightenings, self.proof_numeric_rejects);
            if proof_verify() {
                eprintln!("MIP: proof prunes verified by re-solving the node LP: correct {}, WRONG {} (DEFINITE {}), not verified {}", self.proof_verify_stats[0], self.proof_verify_stats[1], self.proof_verify_stats[3], self.proof_verify_stats[2]);
            }
            eprintln!("MIP: propagation conflicts: analysed {}, no 1-UIP {}, 1-UIP not a row {}, no decision set {}, decision set not a row {}; not a row because continuous {}, general integer {}, both sides {}", self.conf_stats[0], self.conf_stats[1], self.conf_stats[2], self.conf_stats[3], self.conf_stats[4], self.conf_stats[5], self.conf_stats[6], self.conf_stats[7]);
            if cutpool_prop() {
                eprintln!("MIP: cut pool propagation: calls {}, pruned {} nodes, tightened {} bounds, {:.2}s, pool {} cuts", self.cutpool_prop_stats[0], self.cutpool_prop_stats[1], self.cutpool_prop_stats[2], self.cutpool_prop_stats[3] as f64 * 1e-9, self.cut_pool.len());
            }
            eprintln!("MIP: objective propagation: pruned {} nodes, tightened {} bounds; strong branching children infeasible by propagation {}, propagated bounds passed to child LPs {}", self.obj_prop_stats[0], self.obj_prop_stats[1], self.sb_prop_stats[0], self.sb_prop_stats[1]);
            eprintln!("MIP: literal conflicts: added {}, in pool {} (aged out {}), pruned {} nodes, tightened {} bounds", self.lit_stats[0], self.lit_conflicts.len(), self.lit_conflicts.removed, self.lit_stats[1], self.lit_stats[2]);
            if proof_cmir() {
                eprintln!("MIP: proof CMIR cuts: tried {}, added {}, entered the cut pool {}", self.proof_cmir_stats[0], self.proof_cmir_stats[1], self.proof_cmir_stats[2]);
            }
            if self.obj_cut_stats[0] > 0 {
                eprintln!("MIP: objective bound cuts: tried {}, added {}", self.obj_cut_stats[0], self.obj_cut_stats[1]);
            }
            eprintln!("MIP: conflict analysis (HiGHS style): calls {}, conflicts {}, added {}, mean length {:.1}, max length {}, {:.2}s", self.multi_stats[0], self.multi_stats[1], self.multi_stats[2], self.multi_stats[3] as f64 / self.multi_stats[1].max(1) as f64, self.conflict_max_len_multi(), self.multi_secs);
            let cf: Vec<u64> = super::domain::CONFLICT_FAIL.iter().map(|a| a.load(std::sync::atomic::Ordering::Relaxed)).collect();
            eprintln!("MIP: conflict analysis failures (all calls): no decision {}, too long midway {}, reason row lacks column {}, empty {}, too long at the end {} (total length {}), max length {}", cf[0], cf[1], cf[2], cf[3], cf[4], cf[5], self.conflict_max_len());
            if self.par_sb_stats[0] > 0 {
                eprintln!("MIP: parallel strong branching: {} batches, {} candidates", self.par_sb_stats[0], self.par_sb_stats[1]);
            }
            eprintln!("MIP: branching: {} selections, {} without strong branching for dual degeneracy; sb LPs {} iters {}", self.sb_degen_stats[0], self.sb_degen_stats[1], self.sb_lps, self.sb_iters);
            if sb_agree_on() {
                eprintln!("MIP: branching: strong branching vs pseudocost: {} of {} agreed, {} nodes without strong branching", self.sb_agree_stats[1], self.sb_agree_stats[0], self.sb_agree_stats[2]);
            }
            eprintln!("MIP: sibling backtracks {}; basis restores {} (basic count mismatch {}), first LP iterations after a restore {}", self.sibling_backtracks, self.restore_stats.0, self.restore_stats.1, self.restore_stats.2);
            eprintln!("MIP: node diag: nodes {} node LPs {} mean rows {:.1} iters {} ({:.1}/LP) LP secs {:.2} sep rounds {} added {} node cut secs {:.2} final rows {}", self.nodes, self.node_lps, self.node_lp_rows as f64 / self.node_lps.max(1) as f64, self.node_iters, self.node_iters as f64 / self.node_lps.max(1) as f64, self.node_lp_secs, self.node_sep_stats[0], self.node_sep_stats[1], self.node_cut_secs, self.lp.num_rows());
            if env_str!("ENOMOTO_MIP_NODE_CUT_ADAPT").is_some() || self.tree_sep {
                eprintln!("MIP: adaptive node cuts: rounds {} effective {} skipped {} (rate {:.3}), node cut secs {:.2}, outcomes cutoff/<0.02/<0.1/<0.3/>=0.3 {:?}, cut nnz {:?} (rows mean {:.1})", self.nca_stats[0], self.nca_stats[1], self.nca_stats[2], self.nca_ema, self.node_cut_secs, self.nca_hist, self.nca_cut_nnz, self.p.rows.iter().map(|r| r.len()).sum::<usize>() as f64 / self.p.m.max(1) as f64);
            }
            if !self.orbitopes.is_empty() {
                eprintln!("MIP: orbitopes {}: fixed {} bounds, pruned {} nodes", self.orbitopes.len(), self.orbitope_fixings, self.orbitope_prunes);
            }
            if self.sym_canon != (0, 0) {
                eprintln!("MIP: symmetry: {} heuristic solutions mapped to the symmetry-reduced problem, {} could not be", self.sym_canon.0, self.sym_canon.1);
            }
        }
        // `ENOMOTO_MIP_NCA_DIAG`: 木の従のスレッドの分も出す (診断用)
        if !self.params.verbose && !self.params.submip && env_str!("ENOMOTO_MIP_NCA_DIAG").is_some() {
            eprintln!("MIP: node diag (thread {}): nodes {} node LPs {} mean rows {:.1} iters {} ({:.1}/LP) LP secs {:.2} sep rounds {} added {} node cut secs {:.2} final rows {}; adaptive rounds {} effective {} cut nnz {:?}", self.pool_tid, self.nodes, self.node_lps, self.node_lp_rows as f64 / self.node_lps.max(1) as f64, self.node_iters, self.node_iters as f64 / self.node_lps.max(1) as f64, self.node_lp_secs, self.node_sep_stats[0], self.node_sep_stats[1], self.node_cut_secs, self.lp.num_rows(), self.nca_stats[0], self.nca_stats[1], self.nca_cut_nnz);
        }
        let (x, objective) = match &self.incumbent {
            Some((z, x)) => (Some(x.clone()), Some(*z)),
            None => (None, None),
        };
        // 時間・ノードの上限で終わるが、下界 (待ち行列・従・根の LP) が打ち切り値に届いている: 探索し残したものはない
        // (木の時間切れの判定は根のノードを刈るより前にあり、根で下界が打ち切り値に届いたまま時間切れになると、暫定解 = 下界の
        // 時間切れを返していた。physiciansched6-2)。暫定解があれば最適、なければ打ち切り値より良い解はない (実行不能)。
        // 木の写しから解く求解・取りこぼしたノードがあるときは使わない。`ENOMOTO_MIP_NO_LIMIT_GAP_EXIT` で使わない
        let (status, best_bound) = if matches!(status, MipStatus::TimeLimit | MipStatus::NodeLimit)
            && self.snap.is_none()
            && !self.unresolved
            && raw_bound.is_finite()
            && env_str!("ENOMOTO_MIP_NO_LIMIT_GAP_EXIT").is_none()
            && self.incumbent.as_ref().map_or(self.prune_limit().is_finite(), |(z, _)| self.shared.as_ref().is_none_or(|s| !(s.best() < *z)))
            && raw_bound >= self.prune_limit()
        {
            if self.params.verbose {
                eprintln!("MIP: stopped by {status:?} with the lower bound {raw_bound:.10e} at the prune limit {:.10e}; reporting the search as complete", self.prune_limit());
            }
            match &self.incumbent {
                Some((z, _)) => (MipStatus::Optimal, *z),
                None => (MipStatus::Infeasible, f64::INFINITY),
            }
        } else {
            (status, best_bound)
        };
        // 実行不能と報告する前に暫定解があればそれは最適
        let status = match (status, &self.incumbent) {
            (MipStatus::Infeasible, Some(_)) => MipStatus::Optimal,
            (s, _) => s,
        };
        // 最後の守り (全ての終わり方): 共有の最良値が返す解より良ければ、その良い解が主に届いていない。木や根はその値で刈って
        // いるので、返す解を最適とは言えない (木の探索の終わりの確かめを通らない根で終わる道筋で、h80x6320d が 6387.76 (正しくは
        // 6382.10) を最適と報告した。analysis/heur_ls_20261023.md 追記 5)。共有の最良値より良い解はないことは示せているので、
        // 下界はその値にする。`ENOMOTO_MIP_NO_FINAL_SHARED_CHECK` で使わない (診断用)
        let (status, best_bound) = match (status, objective, self.shared.as_ref().map(|s| s.best())) {
            (MipStatus::Optimal, Some(z), Some(sb)) if sb < z - 1e-9 * z.abs().max(1.0) && self.snap.is_none() && env_str!("ENOMOTO_MIP_NO_FINAL_SHARED_CHECK").is_none() => {
                eprintln!("MIP: WARNING: the shared best value {sb:.10e} is better than the returned solution {z:.10e} at the end; not reporting optimality");
                (MipStatus::NotSolved, sb)
            }
            (s, _, _) => (s, best_bound),
        };
        if (self.params.verbose || end_diag()) && !self.params.submip && self.snap.is_none() {
            eprintln!("MIP: finish done ({:.2}s)", self.start.elapsed().as_secs_f64());
        }
        MipResult { status, x, objective, best_bound, nodes: self.nodes + self.worker_totals.0, lp_iterations: self.lp.total_iterations() + self.worker_totals.1 }
    }
}
