//! 分枝限定法から傾き・切片二段解法 ([`super::slope_intercept_dual`]) を使うためのラッパー。
//!
//! 前処理を通さずに、MIP の LP (`row_lo <= a_i x <= row_up`, `lo <= x <= up`) を標準形
//! `σ_i a_i x + s_i = b_i` (スラック `s_i >= 0`、等式行は `s_i = 0`、`>=` だけの行は `σ_i = -1`) に
//! 直して [`StdForm`] として持つ。構造列は「有限の境界のうち下側 (なければ上側) が 0」になるよう
//! 平行移動する (`x = x' + shift`、二段解法の前提)。境界の変更は平行移動量と右辺 `b` の差分更新で
//! 反映するので、行列は作り直さない。行の追加・削除と費用の変更のときだけ作り直す。
//!
//! 毎回の求解は、前回の最適基底 (基底列の集合) を渡して二段解法の warm start
//! (`solve_slope_intercept_dual_from_basis`) で解く。打ち切り条件 (反復上限・目的値・時刻) は
//! [`super::slope_intercept_dual::set_ext_control`] で渡す。

use super::slope_intercept_dual::{self as sid, ExtControl, ExtStop};
use super::{freeze_std_matrices, StdForm};
use crate::mip::lp::{Basis, LpStatus, SolveLimits, VarStatus};
use crate::types::{LpOptions, Status};
use std::cell::OnceCell;
use std::sync::Arc;

// 試作 `ENOMOTO_MIP_ROOTLP3=3` の 3 本同時 (analysis/rootlp3_20261111.md)
mod race3;
pub(crate) use race3::take_race3_winner;

/// 内点法 ([`TwoStageLp::interior_point`]) を打ち切る追加の条件 (見張りのスレッドが 5 ミリ秒ごとに呼ぶ)。
pub type IpmAbort = Arc<dyn Fn() -> bool + Send + Sync>;

/// 検査 `ENOMOTO_MIP_BINV_ROW_DENSE=check` の [照合した回数, ビット一致しなかった要素の数]
pub(crate) static BINV_ROW_CHECK: [std::sync::atomic::AtomicU64; 2] = [const { std::sync::atomic::AtomicU64::new(0) }; 2];

thread_local! {
    /// [`TwoStageLp::basis_inverse_row`] の超疎の単位 BTRAN の作業領域 (次数, 作業, 出力, `e_tilde`)。出力は作業の非零の記録と対で使い回す
    static BINV_ROW_WORK: std::cell::RefCell<Option<(usize, super::lu::UnitBtranWork, Vec<f64>, Vec<f64>)>> = const { std::cell::RefCell::new(None) };
}

thread_local! {
    static IPM_ABORT: std::cell::RefCell<Option<IpmAbort>> = const { std::cell::RefCell::new(None) };
}

/// `abort` をこのスレッドの内点法の追加の打ち切り条件にして `f` を実行し、元に戻す
/// (試作 `ENOMOTO_MIP_S1_IPMFIND`: 解発見スレッドが解を出したら根の内点法の丸めを止める)。
pub fn with_ipm_abort<R>(abort: Option<IpmAbort>, f: impl FnOnce() -> R) -> R {
    let prev = IPM_ABORT.with(|a| std::mem::replace(&mut *a.borrow_mut(), abort));
    let r = f();
    IPM_ABORT.with(|a| *a.borrow_mut() = prev);
    r
}

/// 最適基底とその LU、最終の被約費用 ([`TwoStageLp::lu_cache`])。状態の保存 ([`TwoStageLp::save_state`]) と
/// [`TwoStageLp::lu`] は `Arc` で共有し、求解で LU を渡すときだけ (共有中なら) 複製する。
struct LuCache {
    basis: Vec<usize>,
    lu: Arc<super::lu::FtLu>,
    d: Option<Vec<f64>>,
}

/// [`TwoStageLp::optface_move`] の結果 (試作 `ENOMOTO_MIP_OPTFACE_MOVE`)。
#[derive(Clone, Debug, Default)]
pub struct OptFaceStats {
    /// 最初の基底での候補 (被約費用 0 で動ける非基底) の数 (双対退化の目安)
    pub cands0: usize,
    /// 軸の入れ替え・境界の入れ替え・そのうち退化 (歩幅 0) の数
    pub pivots: usize,
    pub flips: usize,
    pub degenerate: usize,
    /// FTRAN の回数・動けなかった候補の数
    pub ftran: usize,
    pub dead: usize,
    /// 値の変わった構造列の数・基底の違う変数の数
    pub moved_cols: usize,
    pub basis_diff: usize,
    /// 確かめの解き直しの反復・目的値の差・最適で目的値が同じか
    pub resolve_iters: u64,
    pub obj_diff: f64,
    pub ok: bool,
    pub stopped: bool,
    pub singular: bool,
    /// 移動だけの秒・確かめを含む秒
    pub secs_move: f64,
    pub secs: f64,
}

/// 二段解法を使う MIP 用の LP。
#[derive(Clone)]
pub struct TwoStageLp {
    /// 直近の求解の最後の DSE 重み (基底位置順) と、そのときの基底。次の求解の基底が同じなら引き継ぐ
    /// (行を加えたら新しい行の重みを 1 で足す)。状態の保存と共有する。
    dse_cache: Option<Arc<(Vec<usize>, Vec<f64>)>>,
    /// [`Self::dse_cache`] の重みの育ち具合 (重みを初期化してからの反復の数、厳密な値で初期化したら `u64::MAX`。
    /// [`sid::last_dse_age`])。外から当てはめた重み ([`Self::set_dse_vars`]・[`Self::adopt_lu_snapshot`]) は分からないので 0
    dse_age: u64,
    n: usize,
    /// 元の行 (列番号の昇順) と行の境界。
    rows: Vec<Vec<(usize, f64)>>,
    row_lo: Vec<f64>,
    row_up: Vec<f64>,
    /// 元の費用と列の境界。
    cost: Vec<f64>,
    lo: Vec<f64>,
    up: Vec<f64>,
    /// 行の符号 σ_i (`>=` だけの行は -1)。
    sigma: Vec<f64>,
    /// 列の平行移動量。
    shift: Vec<f64>,
    std: StdForm,
    /// 基底 (基底列の番号、長さ m。構造列 j < n、スラック n + i)。`None` なら全スラック基底から。
    basis: Option<Vec<usize>>,
    /// 直近の解 (元の空間の構造列の値)、双対 (標準形の行)、状態。
    x: Vec<f64>,
    y: Vec<f64>,
    status: Option<LpStatus>,
    /// 直近に分解した基底の LU (tableau 行用、基底が変わったら捨てる)。`lu_cache` の LU と共有することがある。
    lu: Option<Arc<super::lu::FtLu>>,
    /// 直近の最適基底とその LU (次の warm start で分解を省く)。
    lu_cache: Option<Arc<LuCache>>,
    iters: u64,
    /// 制約行列の識別子 (作り直すたびに新しい値。PRICE 用の行列の使い回しに使う)。
    price_key: u64,
    /// 直前の求解が実行不能だったときの双対射線 (元の行の向き)。
    ray: Option<Vec<f64>>,
    /// 直前の求解が非有界だったときの切片問題の境界 (標準形の列ごとの下限・上限、[`Self::intercept_bounds`])。
    intercept: Option<(Vec<Option<f64>>, Vec<Option<f64>>)>,
    /// 直近の状態についての [`Self::basis`]・[`Self::reduced_costs`] の結果 (1 ノードで何度も呼ばれるので覚えておく)。
    /// 境界・行・費用・基底・解・双対を変えたら捨てる ([`Self::clear_memo`])。
    basis_memo: OnceCell<Basis>,
    rc_memo: OnceCell<Vec<f64>>,
    /// 打ち切られた求解の続きを解くときの非基底列の置き場所 (`ENOMOTO_MIP_LP_RESUME`、[`Self::solve_inner`])。
    /// 行・境界・基底が変わったら捨てる ([`Self::clear_memo`])。
    resume_nb: Option<Vec<i8>>,
    /// 試作 (`ENOMOTO_MIP_BASIS_NB`、[`Self::set_basis_nb`]): 保存した [`Basis`] の上限・下限から作った、次の求解の非基底列の
    /// 置き場所の希望 (標準形の列ごとに -1 下限、+1 上限、0 は今の推し量り [`Self::nb_hint`] のまま)。次の求解で使い切る。
    /// 行・解が変わったら捨てる ([`Self::clear_memo`])。列の境界の変更では捨てず、求解のときに今の境界と照らす
    basis_nb: Option<Vec<i8>>,
    /// 診断用 (`ENOMOTO_MIP_NB_DIAG`): 直前の [`Self::set_basis`] に渡された置き場所 (次の求解で `nb_hint` と比べる)
    nb_diag: Option<Box<super::nb_diag::Pending>>,
    /// 直近の解 `x` についての [`Self::row_activities`] の結果 (行と `x` にだけよるので、`x` か行を変えたら捨てる。
    /// 列の境界・基底を変えただけなら残し、次の求解の [`Self::nb_hint`] でも使う)。
    act_memo: OnceCell<Vec<f64>>,
    /// `ENOMOTO_MIP_ROOTLP_SCALE` (既定で使う) ([`Self::request_scale_once`]): 次の求解 1 回だけ、基底があっても行と列の尺度を調整した写しで解く
    /// ([`Self::solve_scaled`])。求解の始めに使い切る
    scale_once: bool,
    /// 試作 `ENOMOTO_MIP_LPSCALE` (analysis/mip_lp_scale_20261106.md): 行と列の尺度をかけた双子 (係数 `r_i a_ij c_j`)。
    /// 本体を変える操作は双子にも写し、求解は双子で解いてから本体 (元の尺度) で確かめる ([`Self::solve_tw`])。
    tw: Option<Box<TwoStageLp>>,
    /// 双子の行の尺度 `r_i` (行と同じ並び) と列の尺度 `c_j` (2 の冪)。
    tw_rs: Vec<f64>,
    tw_cs: Arc<Vec<f64>>,
    /// 双子の基底・解が次の求解の出発点として使えるか (直前の求解を双子で解き、その後に本体の基底が外から置かれていない)。
    /// 偽なら次の求解の前に本体から写す ([`Self::tw_pull`])
    tw_sync: bool,
    /// 試作 `ENOMOTO_MIP_LPSCALE=h`: 尺度なしの求解が重かったら双子を作る ([`Self::solve_once`])
    tw_lazy: bool,
}

/// 試作 `ENOMOTO_MIP_LPSCALE` の双子の状態 ([`TwoStageState`]・[`LuSnapshot`] に付ける)。
#[derive(Clone)]
struct TwState {
    basis: Option<Vec<usize>>,
    x: Vec<f64>,
    lu_cache: Option<Arc<LuCache>>,
    dse_cache: Option<Arc<(Vec<usize>, Vec<f64>)>>,
    dse_age: u64,
    sync: bool,
}

/// 試作 `ENOMOTO_MIP_LPSCALE` の数 (求解、双子の反復、仕上げの反復、仕上げで基底が動いた回数、打ち切りを採った回数、実行不能を採った回数、
/// 本体で解き直した回数、時間・反復の上限、双子を作った LP の数、仕上げの秒 (マイクロ秒)、双子の秒 (マイクロ秒))
/// 、確かめで仕上げを省いた回数、確かめで外れて仕上げた回数、確かめの秒 (マイクロ秒))
/// 、`h` で重い求解を見て双子に切り替えた回数)
pub(crate) static LPSCALE_STATS: [std::sync::atomic::AtomicU64; 15] = [const { std::sync::atomic::AtomicU64::new(0) }; 15];

/// `ENOMOTO_MIP_LPSCALE` の数を表示する (`ENOMOTO_MIP_LPSCALE_LOG`、MIP の終わりに呼ぶ)
pub(crate) fn lpscale_report() {
    if env_def!("ENOMOTO_MIP_LPSCALE", "h").is_none_or(|v| v == "0") {
        return;
    }
    let g = |k: usize| LPSCALE_STATS[k].load(std::sync::atomic::Ordering::Relaxed);
    eprintln!(
        "LPSCALE: lps {} solves {} tw_iters {} polish_iters {} polish_moved {} objbound {} infeas {} fallback {} limit {} polish_s {:.2} tw_s {:.2} check_ok {} check_fail {} check_s {:.2} switched {}",
        g(8), g(0), g(1), g(2), g(3), g(4), g(5), g(6), g(7), g(9) as f64 * 1e-6, g(10) as f64 * 1e-6, g(11), g(12), g(13) as f64 * 1e-6, g(14)
    );
}

/// 試作 `ENOMOTO_MIP_LPSCALE=h` (analysis/mip_lp_scale_20261106.md の改良 (11-08)): 尺度なしの LP の求解が重かった (1 回が
/// `ENOMOTO_T_MIP_LPSCALE_HEAVY_SECS` 秒 (既定 1) 以上、または数値の破綻) のを見て立てる印。立った後に作る LP (サブ MIP・再スタート・
/// 潜りの写しなど) は作るときに双子を持つ。外側の求解の始めに下ろす ([`lpscale_disarm`])
static LPSCALE_ARMED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(crate) fn lpscale_disarm() {
    LPSCALE_ARMED.store(false, std::sync::atomic::Ordering::Relaxed);
}

/// `ENOMOTO_MIP_LPSCALE` の形: 0 使わない、1 作るときから (条件 [`TwoStageLp::attach_tw`])、2 重い求解を見てから (`h`)
fn lpscale_mode() -> u8 {
    match env_def!("ENOMOTO_MIP_LPSCALE", "h") {
        None | Some("0") => 0,
        Some("h") => 2,
        Some(_) => 1,
    }
}

fn lpscale_add(k: usize, v: u64) {
    LPSCALE_STATS[k].fetch_add(v, std::sync::atomic::Ordering::Relaxed);
}

/// 試作 `ENOMOTO_MIP_BASIS_NB` ([`TwoStageLp::set_basis_nb`]) を、場所 `site` (`t` 木のノードの基底の復元、`s` サブ MIP の根、
/// `b` 裏のカットの写しの根 (と解探索のスレッドの根)、`w` 主の根の LP が 1 秒以上の問題の裏のカットの写しの根だけ、`r` 再スタートの後の根、`p` 前処理の LP による固定の基底からの根 (root_probe)) で使うか。`1` なら全部、`t,s` のようにカンマ区切りで場所を選ぶ
pub fn basis_nb_on(site: char) -> bool {
    match env_def!("ENOMOTO_MIP_BASIS_NB", "t") {
        None | Some("0") | Some("off") => false,
        Some("1") => true,
        Some(v) => v.split(',').any(|s| s.trim().starts_with(site)),
    }
}

/// [`basis_nb_on`] のどれかの場所で使うか。
pub fn basis_nb_any() -> bool {
    matches!(env_def!("ENOMOTO_MIP_BASIS_NB", "t"), Some(v) if v != "0" && v != "off")
}

/// 診断: [`TwoStageLp::set_basis_nb`] の希望を使った求解の数、今の境界で有効だった希望の数、推し量りから変えた数。
pub static BASIS_NB_STATS: [std::sync::atomic::AtomicU64; 3] = [const { std::sync::atomic::AtomicU64::new(0) }; 3];

/// [`TwoStageLp::price_key`] の発行元。
static NEXT_PRICE_KEY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn new_price_key() -> u64 {
    NEXT_PRICE_KEY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

thread_local! {
    /// 試作 `ENOMOTO_MIP_CUT_LP_CAP_ROBUST` (根のカットのループ、`crate::mip::separation`): 次の 1 回の求解だけ、時間・反復の上限で
    /// 止まったら止まった基底・DSE 重み・置き場所を残す (`ENOMOTO_MIP_LP_RESUME` と同じ。`d` で上限を倍にして続けるため)
    pub(crate) static FORCE_RESUME_ONCE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// 同じく `p`: 次の 1 回の求解だけ、反復が `from` に達した後は双対の目的値の進みで打ち切る ((from, window, frac, 出発の目的値 (元の座標)))
    pub(crate) static PROGRESS_STOP_ONCE: std::cell::Cell<Option<(usize, usize, f64, f64)>> = const { std::cell::Cell::new(None) };
    /// このスレッドの基底のない求解を同時実行にするときのスレッド数 ([`TwoStageLp::solve`]。1 なら同時実行しない)。
    static COLD_RACE_THREADS: std::cell::Cell<usize> = const { std::cell::Cell::new(1) };
}

thread_local! {
    /// 試作 `ENOMOTO_MIP_ROOTLP_BOTH`: 次の基底のない同時実行 1 回だけ、負けた側を止めずに続ける ((待ちの比 k, 待ちの絶対の上限 (秒), 待ちの下限 (秒)))
    static BOTH_REQ: std::cell::Cell<Option<(f64, f64, f64)>> = const { std::cell::Cell::new(None) };
    /// 同じく、続けている負けた側 ([`TwoStageLp::solve_race_both`] が置き、[`take_both_loser`] で取り出す)
    static BOTH_LOSER: std::cell::RefCell<Option<BothLoser>> = const { std::cell::RefCell::new(None) };
}

/// 試作 `ENOMOTO_MIP_ROOTLP_BOTH`: このスレッドの次の基底のない同時実行 1 回だけ、勝った側の後も負けた側を止めずに続ける。
/// 負けた側は勝った側の時間の `ratio` 倍 (少なくとも `min_secs`、多くとも `max_secs`) まで続け、それを過ぎたら止める。
pub(crate) fn request_both(ratio: f64, max_secs: f64, min_secs: f64) {
    BOTH_REQ.with(|c| c.set(Some((ratio, max_secs, min_secs))));
    BOTH_LOSER.with(|c| *c.borrow_mut() = None);
}

/// [`request_both`] の取り消し (同時実行が起きなかったときの残りも消す) と、続けている負けた側の取り出し。
pub(crate) fn take_both_loser() -> Option<BothLoser> {
    BOTH_REQ.with(|c| c.set(None));
    BOTH_LOSER.with(|c| c.borrow_mut().take())
}

type RaceWon = (Vec<f64>, Option<(Vec<f64>, Vec<Option<usize>>)>);

enum BothMsg {
    /// 内点法 + クロスオーバーの最適な頂点 (標準形の空間) と仕上げの記録。得られなければ `None`
    Ipm(Option<RaceWon>),
    /// 二段解法で解いた LP の写し
    Simplex(LpStatus, Box<TwoStageLp>, u8),
}

/// 試作 `ENOMOTO_MIP_ROOTLP_BOTH`: 根の LP の同時実行で負けた側 (裏で続いている)。落とすと止める。
pub struct BothLoser {
    /// 負けた側 ("ipm" または "simplex")
    pub kind: &'static str,
    rx: std::sync::mpsc::Receiver<BothMsg>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
    /// 内点法側の頂点を当てはめる元の LP (基底のない写し)
    base: Option<Box<TwoStageLp>>,
    lim: SolveLimits,
    /// 勝った側の時間 (同時実行の始めから)
    pub win_secs: f64,
    /// 負けた側を止める時刻
    pub give_up: std::time::Instant,
    t0: std::time::Instant,
}

impl Drop for BothLoser {
    fn drop(&mut self) {
        self.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

impl BothLoser {
    /// 負けた側の結果を `until` (と止める時刻の早い方) まで待つ。最適に解けていれば、その頂点を基底解にした LP と、
    /// 同時実行の始めから届くまでの時間を返す。待ちきれない・解けなかったら `None` (負けた側は止める)。
    pub fn wait(mut self, until: std::time::Instant) -> Option<(TwoStageLp, f64)> {
        let end = until.min(self.give_up);
        let msg = self.rx.recv_timeout(end.saturating_duration_since(std::time::Instant::now())).ok();
        self.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        let secs = self.t0.elapsed().as_secs_f64();
        match msg? {
            BothMsg::Simplex(st, lp, _) => (st == LpStatus::Optimal).then(|| (*lp, secs)),
            BothMsg::Ipm(w) => {
                let (x, duals) = w?;
                let mut lp = self.base.take()?;
                let st = lp.adopt_ipm_vertex(&x, duals, &self.lim, false, self.t0);
                lp.status = Some(st);
                (st == LpStatus::Optimal).then(|| (*lp, secs))
            }
        }
    }
}

/// このスレッドの基底のない求解の同時実行のスレッド数を設定し、前の値を返す ([`crate::mip::solve_mip`] が入口で
/// `threads` に、木の探索の従のスレッドを始めるときに 1 にする)。
pub(crate) fn set_cold_race_threads(k: usize) -> usize {
    COLD_RACE_THREADS.with(|c| c.replace(k.max(1)))
}

/// このスレッドの [`set_cold_race_threads`] の値。
pub(crate) fn cold_race_threads() -> usize {
    COLD_RACE_THREADS.with(|c| c.get())
}

/// 診断用 [`TwoStageLp::diag_both`] の結果。
pub struct BothDiag {
    /// 二段解法で解いた写しと状態、時間 (始めからの秒)。
    pub lp_s: TwoStageLp,
    pub st_s: LpStatus,
    pub t_simplex: f64,
    /// 内点法 + クロスオーバーの頂点を採った写しと状態 (最適な頂点が得られなければ `None`)。
    pub lp_i: TwoStageLp,
    pub st_i: Option<LpStatus>,
    /// 内点法が終わった時刻・クロスオーバーまで終わった時刻 (始めからの秒)、頂点を採る手間 (秒)。
    pub t_ipm: Option<f64>,
    pub t_xo: f64,
    pub t_adopt: f64,
}

/// [`TwoStageLp::interior_point`] の結果 (元の空間の構造列の値)。
pub struct InteriorPoint {
    /// 内点法の解。
    pub x: Vec<f64>,
    /// クロスオーバーの押し出しで作った (基底 (標準形の列番号), 頂点 (主実行可能でなければ `None`))。
    /// 基底は双対実行可能とは限らない。
    pub vertex: Option<(Vec<usize>, Option<Vec<f64>>)>,
}

/// 状態の保存 (強分岐用)。
#[derive(Clone)]
pub struct TwoStageState {
    lo: Vec<f64>,
    up: Vec<f64>,
    basis: Option<Vec<usize>>,
    x: Vec<f64>,
    y: Vec<f64>,
    status: Option<LpStatus>,
    lu_cache: Option<Arc<LuCache>>,
    dse_cache: Option<Arc<(Vec<usize>, Vec<f64>)>>,
    dse_age: u64,
    /// 試作 `ENOMOTO_MIP_LPSCALE` の双子の状態
    tw: Option<Box<TwState>>,
}

/// 取り出した LU キャッシュと DSE 重み ([`TwoStageLp::lu_snapshot`]。どちらも `Arc` の共有)。分枝で潜らない側の
/// 兄弟ノードと一緒に持ち、兄弟へ戻って親の基底を置いたときに [`TwoStageLp::adopt_lu_snapshot`] で戻して分解を省く。
#[derive(Clone)]
pub struct LuSnapshot {
    /// 本体の LU (試作 `ENOMOTO_MIP_LPSCALE` で本体の仕上げを省いたときは `None`、双子の LU だけ)。
    lu: Option<Arc<LuCache>>,
    /// `lu` の基底の DSE 重み (基底位置の順)。
    dse: Option<Arc<(Vec<usize>, Vec<f64>)>>,
    /// 取り出したときの行列の識別子 ([`TwoStageLp::price_key`]。行の追加・削除で変わる)。
    price_key: u64,
    /// 試作 `ENOMOTO_MIP_LPSCALE` の双子の LU
    tw: Option<Box<LuSnapshot>>,
}

/// 試作 `ENOMOTO_XO_MIP_SCALE=1` (既定では使わない。analysis/rootlp_ipm_fail_20261105.md): MIP の LP の内点法 + クロスオーバー
/// (根の LP の競争・解き直し) を、行と列の尺度をそろえた写し ([`scale_factors`]、2 の冪) の標準形で解く。解は `x = c x'`、双対は
/// `y = r y'` で元の尺度に戻す (基底は尺度によらない)。目的値は尺度によらないので、二段解法と共有する下界もそのまま使える。
/// momentum1 の根の LP: 元の尺度では双対の乗数が 1e7〜1e12 で、内点法が停滞し、押し出しの基底からの仕上げの単体法も破綻した
/// (LP 単独の入口 (前処理で尺度をそろえる) では 14.7 秒で解ける)。
struct IpmScale {
    std: StdForm,
    /// 仕上げに非基底の向きを渡さないか (係数の幅が `ENOMOTO_T_XO_MIP_SCALE_NOHINT_RANGE` (既定 1e8) 以上。momentum1 (幅 7.9e9) は
    /// 渡すと仕上げが 1.5 → 15.5 秒、satellites2-40 (6.6e5) は渡さないと 7.3 → 59.9 秒。この 2 問だけで決めた値)
    no_hint: bool,
    rs: Vec<f64>,
    cs: Vec<f64>,
    sc_shift: Vec<f64>,
    shift: Vec<f64>,
}


/// 内点法 + クロスオーバーを (`sc` があれば尺度をそろえた写しで) 解き、解 (標準形の構造列の空間) と記録した双対を元の尺度に戻す。
/// 試作 `ENOMOTO_MIP_IPMRC` (analysis/ipmrc_20261111.md、既定では使わない) に文字 `c` が入っているか。`u`: 根の LP の同時実行で
/// 内点法の点が手に入ったら、その双対のラグランジュ下界と被約費用を被約費用による固定に使う (`src/mip/xolazy.rs` の
/// `ipmrc_store`)。`w`: 二段解法が先に最適で終わっても内点法の点が届くまで待つ (採るのは二段解法の結果)
pub(crate) fn ipmrc_has(c: char) -> bool {
    env_str!("ENOMOTO_MIP_IPMRC").is_some_and(|v| v.contains(c))
}

/// 診断 `ENOMOTO_MIP_XOLAZY_LOG` (analysis/xolazy_20261111.md、振る舞いは変えない) が立っているか
pub(crate) fn xolazy_log() -> bool {
    env_str!("ENOMOTO_MIP_XOLAZY_LOG").is_some()
}

/// 基底のない LP の同時実行 ([`TwoStageLp::solve_race`]) の記録 (診断 `ENOMOTO_MIP_XOLAZY_LOG`)。秒はどれも同時実行の始めから。
#[derive(Clone, Debug, Default)]
pub struct RaceDiag {
    /// 勝った側: `simplex` 二段解法・`ipm` 内点法 + クロスオーバー (最適基底をそのまま採った)・`ipm_resolve` (頂点から解き直した)・
    /// `none` (内点法側も結論なし、二段解法で解き直した)
    pub winner: &'static str,
    /// 勝ち負けが決まった時刻 (二段解法が勝ったときはその終わり、内点法が勝ったときは頂点が届いた時刻)
    pub t_win: f64,
    /// 勝ち負けが決まった時点までに内点法が終わっていればその時刻
    pub t_ipm: Option<f64>,
    /// 同じく押し出しが終わっていれば (仕上げの単体法の始め) その時刻
    pub t_push: Option<f64>,
    /// 同じく仕上げが終わっていればその時刻
    pub t_xo: Option<f64>,
    /// 同時実行を抜けた時刻 (内点法が勝ったときは頂点を採る・解き直しを含む)
    pub t_end: f64,
    /// 内点法が勝った (最適で終わった) ときの、ラグランジュ緩和の下界 `L(y)` の最適値との差 (最適な双対の `L` − その双対の `L`、
    /// 0 以上なら弱い)。`simplex` は止められた二段解法が最後に下界を計算した途中の基底の双対、`ipm` は内点法の双対 (押し出しの前)。
    /// 無限の境界の向きに被約費用がある列があれば `inf`
    pub gap_simplex: Option<f64>,
    pub gap_ipm: Option<f64>,
    /// 最適な双対 (この LP の双対) のラグランジュ下界と主の目的値の差 (双対の精度と余裕の分)
    pub gap_opt: Option<f64>,
    /// 内点法の双対のラグランジュ下界 (元の列の単位の目的値、定数項 `offset` は含まない。浮動小数の誤差の余裕を引いた値)
    pub lb_ipm: Option<f64>,
    /// 同時実行で共有した下界 (二段解法の途中の基底からの値) の最適値との差
    pub gap_shared: Option<f64>,
    /// 構造列の被約費用 (最小化形): 最適な双対・途中の基底の双対・内点法の双対
    pub d_opt: Option<Vec<f64>>,
    pub d_simplex: Option<Vec<f64>>,
    pub d_ipm: Option<Vec<f64>>,
    /// 診断 `ENOMOTO_MIP_IPMRC_WAIT`: 二段解法が勝った後、内点法の点を待った秒
    pub t_wait: Option<f64>,
}

thread_local! {
    static RACE_DIAG: std::cell::RefCell<Option<RaceDiag>> = const { std::cell::RefCell::new(None) };
}

/// このスレッドで最後に記録した同時実行の記録を取り出す (診断 `ENOMOTO_MIP_XOLAZY_LOG`)
pub fn take_race_diag() -> Option<RaceDiag> {
    RACE_DIAG.with(|d| d.borrow_mut().take())
}

fn xolazy_record(winner: &'static str, t0: std::time::Instant, stamp: Option<&super::crossover::XoStamp>, t_win: Option<f64>) {
    let Some(stamp) = stamp else { return };
    let t_end = t0.elapsed().as_secs_f64();
    let a = stamp.lock().map(|a| a.t).unwrap_or([None; 3]);
    let f = |t: Option<std::time::Instant>| t.map(|t| t.saturating_duration_since(t0).as_secs_f64());
    let d = RaceDiag { winner, t_win: t_win.unwrap_or(t_end), t_ipm: f(a[0]), t_push: f(a[1]), t_xo: f(a[2]), t_end, ..Default::default() };
    RACE_DIAG.with(|r| *r.borrow_mut() = Some(d));
}

/// 標準形の行の双対 `y` (どの `y` でもよい) からラグランジュ緩和の下界 `L(y) = b·y + Σ_j min_{l_j <= x_j <= u_j} (c - A^T y)_j x_j`
/// (標準形の目的の単位、浮動小数の誤差の余裕を引いた値) と構造列の被約費用を返す。無限の境界の向きに被約費用がある列があれば `L = -inf`。
fn lagrange_std(std: &StdForm, n: usize, y: &[f64]) -> Option<(f64, Vec<f64>)> {
    if y.len() != std.n_rows || y.iter().any(|v| !v.is_finite()) {
        return None;
    }
    // 近似の双対 (内点法) はスラック列 (片側だけ有限の境界) の被約費用の符号がわずかに逆になり `L = -inf` になりやすいので、
    // スラック列の被約費用が境界の無限の向きに 0 となるよう行の双対を射影する (射影した `y` でも `L(y)` は正しい下界)
    let mut y = y.to_vec();
    for j in n..std.n_total {
        let col = std.cols.col(j);
        if col.len() != 1 {
            continue;
        }
        let (i, a) = col[0];
        let d = std.c[j] - a * y[i];
        if (d > 0.0 && !std.lb[j].is_finite()) || (d < 0.0 && !std.ub[j].is_finite()) {
            y[i] = std.c[j] / a;
        }
    }
    let y = &y[..];
    let mut lb: f64 = std.b.iter().zip(y).map(|(b, y)| b * y).sum();
    // 浮動小数の和の誤差の余裕に使う、項の絶対値の和
    let mut abs: f64 = std.b.iter().zip(y).map(|(b, y)| (b * y).abs()).sum();
    let mut dn = vec![0.0; n];
    for j in 0..std.n_total {
        let mut d = std.c[j];
        for &(i, a) in std.cols.col(j) {
            d -= a * y[i];
        }
        if j < n {
            dn[j] = d;
        }
        if d > 0.0 {
            lb += if std.lb[j].is_finite() { d * std.lb[j] } else { f64::NEG_INFINITY };
            abs += (d * std.lb[j]).abs();
        } else if d < 0.0 {
            lb += if std.ub[j].is_finite() { d * std.ub[j] } else { f64::NEG_INFINITY };
            abs += (d * std.ub[j]).abs();
        }
    }
    // 余裕: 和の丸めの誤差 (項の絶対値の和の 1e-9 倍) と、値の大きさの 1e-9 倍を引いた値を下界とする
    let lb = lb - 1e-9 * abs - 1e-9 * (1.0 + lb.abs());
    Some((lb, dn))
}

fn ipm_crossover_run(std: &StdForm, sc: Option<&IpmScale>) -> Option<super::SimplexResult> {
    let Some(s) = sc else {
        return super::crossover::solve_ipm_crossover(std);
    };
    let prev = super::crossover::XO_NO_CLEANUP_HINT.with(|f| f.replace(s.no_hint));
    let r = super::crossover::solve_ipm_crossover(&s.std);
    super::crossover::XO_NO_CLEANUP_HINT.with(|f| f.set(prev));
    if sid::duals_requested() {
        if let Some((y, bp)) = sid::take_duals() {
            let y2: Vec<f64> = y.iter().zip(&s.rs).map(|(v, r)| v * r).collect();
            sid::put_duals(y2, bp);
        }
    }
    r.map(|mut r| {
        if let Some(x) = r.x.as_mut() {
            let n = s.cs.len().min(x.len());
            for j in 0..n {
                x[j] = (x[j] + s.sc_shift[j]) * s.cs[j] - s.shift[j];
            }
        }
        r
    })
}

/// 試作 `ENOMOTO_SIMPLEX_SCALE=1` (既定では使わない。analysis/rootlp_simplex_fail_20261105.md): 基底のない求解
/// ([`TwoStageLp::solve_once`]、MIP の根の LP・LP プロービングの最初の LP など) を、行と列の尺度を調整した写しで解く
/// ([`TwoStageLp::solve_scaled`])。HiGHS は尺度を調整してから解き、uccase12 の根の LP は調整なし (`simplex_scale_strategy=0`) だと
/// 120 秒で終わらない (調整ありは 2 秒)。行の数が `ENOMOTO_T_SIMPLEX_SCALE_MIN_M` (既定 10000、2026-11-09 から) 未満、または係数の幅 ([`coef_range`]) が
/// `ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE` (既定 1e4、2026-11-09 から) 未満なら使わない
fn scale_cold_on() -> bool {
    env_def!("ENOMOTO_SIMPLEX_SCALE", "1").is_some_and(|v| v != "0")
}

/// 試作 `ENOMOTO_SIMPLEX_SCALE` の行と列の尺度 (`r_i`, `s_j`、2 の冪): 調整した係数は `r_i a_ij s_j`。幾何平均の調整
/// (行と列を交互に `1 / sqrt(min |a| max |a|)`、最大 `ENOMOTO_T_SIMPLEX_SCALE_PASSES` 回 (既定 4)) の後、列の最大の絶対値を 1 に近づける。
/// 尺度は 2^-20..2^20 に収める (HiGHS の `allowed_matrix_scale_factor` と同じ)。
fn scale_factors(n: usize, rows: &[Vec<(usize, f64)>]) -> (Vec<f64>, Vec<f64>) {
    let m = rows.len();
    let mut rs = vec![1.0f64; m];
    let mut cs = vec![1.0f64; n];
    let passes = tunable!("ENOMOTO_T_SIMPLEX_SCALE_PASSES", 4usize, usize);
    let lim = 2f64.powi(20);
    let clamp = |v: f64| if v.is_finite() && v > 0.0 { v.clamp(1.0 / lim, lim) } else { 1.0 };
    for _ in 0..passes {
        // 行
        for (i, r) in rows.iter().enumerate() {
            let (mut mn, mut mx) = (f64::INFINITY, 0.0f64);
            for &(j, v) in r {
                let a = (v * cs[j]).abs();
                if a > 0.0 {
                    mn = mn.min(a);
                    mx = mx.max(a);
                }
            }
            if mx > 0.0 {
                rs[i] = clamp(1.0 / (mn * mx).sqrt());
            }
        }
        // 列
        let mut mn = vec![f64::INFINITY; n];
        let mut mx = vec![0.0f64; n];
        for (i, r) in rows.iter().enumerate() {
            for &(j, v) in r {
                let a = (v * rs[i]).abs();
                if a > 0.0 {
                    mn[j] = mn[j].min(a);
                    mx[j] = mx[j].max(a);
                }
            }
        }
        for j in 0..n {
            if mx[j] > 0.0 {
                cs[j] = clamp(1.0 / (mn[j] * mx[j]).sqrt());
            }
        }
    }
    // 列の最大を 1 に
    let mut mx = vec![0.0f64; n];
    for (i, r) in rows.iter().enumerate() {
        for &(j, v) in r {
            mx[j] = mx[j].max((v * rs[i] * cs[j]).abs());
        }
    }
    for j in 0..n {
        if mx[j] > 0.0 {
            cs[j] = clamp(cs[j] / mx[j]);
        }
    }
    let p2 = |v: f64| 2f64.powi(v.log2().round() as i32);
    (rs.into_iter().map(p2).collect(), cs.into_iter().map(p2).collect())
}

/// 試作 `ENOMOTO_SIMPLEX_SCALE` の条件 `ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE` (既定 0 = いつも): 係数の絶対値の最大 / 最小。
/// 根の LP の 9 問では、尺度の調整が効いたのは 4e4 以上 (uccase12・snp-02-004-104・uccase9)、悪くしたのは 4.2e3 (supportcase42)・1.2e3 (sing326)
fn coef_range(rows: &[Vec<(usize, f64)>]) -> f64 {
    let (mut mn, mut mx) = (f64::INFINITY, 0.0f64);
    for r in rows {
        for &(_, v) in r {
            let a = v.abs();
            if a > 0.0 {
                mn = mn.min(a);
                mx = mx.max(a);
            }
        }
    }
    if mx > 0.0 { mx / mn } else { 1.0 }
}

fn shift_of(lo: f64, up: f64) -> f64 {
    if lo.is_finite() {
        lo
    } else if up.is_finite() {
        up
    } else {
        0.0
    }
}

impl TwoStageLp {
    /// MIP の LP を作る。試作 `ENOMOTO_MIP_LPSCALE=1` で条件 ([`Self::attach_tw`]) に当たれば、尺度をかけた双子も持つ。
    pub fn new(col_lo: &[f64], col_up: &[f64], cost: &[f64], rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64]) -> Self {
        let mut lp = Self::new_raw(col_lo, col_up, cost, rows, row_lo, row_up);
        match lpscale_mode() {
            1 => lp.attach_tw(),
            2 if LPSCALE_ARMED.load(std::sync::atomic::Ordering::Relaxed) => lp.attach_tw(),
            2 => lp.tw_lazy = true,
            _ => {}
        }
        lp
    }

    /// 試作 `ENOMOTO_MIP_LPSCALE`: 行の数が `ENOMOTO_T_MIP_LPSCALE_MIN_M` (既定 0) 以上、係数の幅が `ENOMOTO_T_MIP_LPSCALE_MIN_RANGE`
    /// (既定 0) 以上なら、行と列の尺度 ([`scale_factors`]) を決めて双子を作る。尺度がすべて 1 なら作らない
    fn attach_tw(&mut self) {
        if self.rows.len() < tunable!("ENOMOTO_T_MIP_LPSCALE_MIN_M", 0usize, usize) {
            return;
        }
        let range = coef_range(&self.rows);
        if range < tunable!("ENOMOTO_T_MIP_LPSCALE_MIN_RANGE", 1e4f64, f64) {
            return;
        }
        let (rs, cs) = scale_factors(self.n, &self.rows);
        if rs.iter().chain(&cs).all(|&v| v == 1.0) {
            return;
        }
        let rows: Vec<Vec<(usize, f64)>> = self.rows.iter().enumerate().map(|(i, r)| r.iter().map(|&(j, v)| (j, v * rs[i] * cs[j])).collect()).collect();
        let row_lo: Vec<f64> = (0..rs.len()).map(|i| self.row_lo[i] * rs[i]).collect();
        let row_up: Vec<f64> = (0..rs.len()).map(|i| self.row_up[i] * rs[i]).collect();
        let lo: Vec<f64> = (0..self.n).map(|j| self.lo[j] / cs[j]).collect();
        let up: Vec<f64> = (0..self.n).map(|j| self.up[j] / cs[j]).collect();
        let cost: Vec<f64> = (0..self.n).map(|j| self.cost[j] * cs[j]).collect();
        let tw = TwoStageLp::new_raw(&lo, &up, &cost, &rows, &row_lo, &row_up);
        if env_str!("ENOMOTO_MIP_LPSCALE_LOG").is_some() {
            eprintln!("LPSCALE: scaled twin for {} rows x {} cols (range {range:.3e} -> {:.3e})", self.rows.len(), self.n, coef_range(&tw.rows));
        }
        lpscale_add(8, 1);
        self.tw = Some(Box::new(tw));
        self.tw_rs = rs;
        self.tw_cs = Arc::new(cs);
        self.tw_sync = false;
    }

    /// 双子を作らずに LP を作る (尺度をかけた写し・双子そのもの)
    fn new_raw(col_lo: &[f64], col_up: &[f64], cost: &[f64], rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64]) -> Self {
        let n = col_lo.len();
        let mut sorted: Vec<Vec<(usize, f64)>> = rows.iter().map(|r| {
            let mut r: Vec<(usize, f64)> = r.iter().filter(|&&(_, v)| v != 0.0).cloned().collect();
            r.sort_unstable_by_key(|&(j, _)| j);
            r
        }).collect();
        // 同じ列の重複を足し合わせる
        for r in sorted.iter_mut() {
            let mut out: Vec<(usize, f64)> = Vec::with_capacity(r.len());
            for &(j, v) in r.iter() {
                match out.last_mut() {
                    Some(last) if last.0 == j => last.1 += v,
                    _ => out.push((j, v)),
                }
            }
            *r = out;
        }
        let m = sorted.len();
        let shift: Vec<f64> = (0..n).map(|j| shift_of(col_lo[j], col_up[j])).collect();
        let dummy = Self::build_std(n, &sorted, row_lo, row_up, cost, col_lo, col_up, &shift);
        TwoStageLp {
            n,
            rows: sorted,
            row_lo: row_lo.to_vec(),
            row_up: row_up.to_vec(),
            cost: cost.to_vec(),
            lo: col_lo.to_vec(),
            up: col_up.to_vec(),
            sigma: dummy.1,
            shift,
            std: dummy.0,
            basis: None,
            x: vec![0.0; n],
            y: vec![0.0; m],
            status: None,
            lu: None,
            lu_cache: None,
            dse_cache: None,
            dse_age: 0,
            iters: 0,
            price_key: new_price_key(),
            ray: None,
            intercept: None,
            basis_memo: OnceCell::new(),
            rc_memo: OnceCell::new(),
            act_memo: OnceCell::new(),
            scale_once: false,
            tw: None,
            tw_rs: Vec::new(),
            tw_cs: Arc::new(Vec::new()),
            tw_sync: false,
            tw_lazy: false,
            resume_nb: None,
            basis_nb: None,
            nb_diag: None,
        }
    }

    /// 覚えておいた基底・被約費用を捨てる (列の境界・基底・費用・双対が変わったとき)。行の活動量は残す。
    fn clear_state_memo(&mut self) {
        self.basis_memo.take();
        self.rc_memo.take();
    }

    /// 覚えておいた基底・被約費用・行の活動量をすべて捨てる (解 `x` か行が変わったとき)。
    fn clear_memo(&mut self) {
        self.clear_state_memo();
        self.act_memo.take();
        self.resume_nb = None;
        self.basis_nb = None;
        self.tw_sync = false;
    }

    /// 標準形を作る。戻り値は (標準形, 行の符号)。
    #[allow(clippy::too_many_arguments)]
    fn build_std(n: usize, rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64], cost: &[f64], lo: &[f64], up: &[f64], shift: &[f64]) -> (StdForm, Vec<f64>) {
        let m = rows.len();
        let n_total = n + m;
        let mut std_rows: Vec<Vec<(usize, f64)>> = Vec::with_capacity(m);
        let mut b = Vec::with_capacity(m);
        let mut slb = vec![0.0; n_total];
        let mut sub = vec![0.0; n_total];
        let mut sigma = Vec::with_capacity(m);
        for j in 0..n {
            slb[j] = lo[j] - shift[j];
            sub[j] = up[j] - shift[j];
        }
        for i in 0..m {
            let (l, u) = (row_lo[i], row_up[i]);
            // (σ, 右辺, スラックの上限)
            let (sg, rhs, s_up, empty) = if l.is_finite() && u.is_finite() {
                (1.0, u, u - l, false)
            } else if u.is_finite() {
                (1.0, u, f64::INFINITY, false)
            } else if l.is_finite() {
                (-1.0, -l, f64::INFINITY, false)
            } else {
                // 制約のない行: 空の行 (0 = 0) にする
                (1.0, 0.0, 0.0, true)
            };
            let mut r: Vec<(usize, f64)> = Vec::with_capacity(rows[i].len() + 1);
            let mut rhs = rhs;
            if !empty {
                for &(j, v) in &rows[i] {
                    r.push((j, sg * v));
                    rhs -= sg * v * shift[j];
                }
            }
            r.push((n + i, 1.0));
            std_rows.push(r);
            b.push(rhs);
            slb[n + i] = 0.0;
            sub[n + i] = s_up.max(0.0);
            sigma.push(sg);
        }
        let mut c = vec![0.0; n_total];
        c[..n].copy_from_slice(cost);
        let (rows_m, cols_m) = freeze_std_matrices(&std_rows, n_total);
        (StdForm { n_total, n_rows: m, c, rows: rows_m, cols: cols_m, b, lb: slb, ub: sub }, sigma)
    }

    fn rebuild(&mut self) {
        sid::xcount("n_rebuild");
        self.lu_cache = None;
        let (std, sigma) = Self::build_std(self.n, &self.rows, &self.row_lo, &self.row_up, &self.cost, &self.lo, &self.up, &self.shift);
        self.std = std;
        self.sigma = sigma;
        self.price_key = new_price_key();
        self.lu = None;
    }

    pub fn num_cols(&self) -> usize {
        self.n
    }

    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    pub fn total_iterations(&self) -> u64 {
        self.iters
    }

    pub fn set_total_iterations(&mut self, v: u64) {
        self.iters = v;
    }

    pub fn col_bounds(&self, j: usize) -> (f64, f64) {
        (self.lo[j], self.up[j])
    }

    pub fn row_bounds(&self, i: usize) -> (f64, f64) {
        (self.row_lo[i], self.row_up[i])
    }

    pub fn row(&self, i: usize) -> Vec<(usize, f64)> {
        self.rows[i].clone()
    }

    pub fn row_ref(&self, i: usize) -> &[(usize, f64)] {
        &self.rows[i]
    }

    pub fn costs(&self) -> Vec<f64> {
        self.cost.clone()
    }

    pub fn set_col_bounds(&mut self, j: usize, lo: f64, up: f64) {
        if self.lo[j] == lo && self.up[j] == up {
            return;
        }
        self.lo[j] = lo;
        self.up[j] = up;
        let ns = shift_of(lo, up);
        let d = ns - self.shift[j];
        if d != 0.0 {
            for &(i, v) in self.std.cols.col(j) {
                self.std.b[i] -= v * d;
            }
            self.shift[j] = ns;
        }
        self.std.lb[j] = lo - ns;
        self.std.ub[j] = up - ns;
        self.status = None;
        self.clear_state_memo();
        if let Some(t) = self.tw.as_mut() {
            let c = self.tw_cs[j];
            t.set_col_bounds(j, lo / c, up / c);
        }
    }

    pub fn set_row_bounds(&mut self, i: usize, lo: f64, up: f64) {
        self.row_lo[i] = lo;
        self.row_up[i] = up;
        self.rebuild();
        self.status = None;
        let sync = self.tw_sync;
        self.clear_memo();
        if let Some(t) = self.tw.as_mut() {
            let r = self.tw_rs[i];
            t.set_row_bounds(i, lo * r, up * r);
            self.tw_sync = sync;
        }
    }

    pub fn set_costs(&mut self, c: &[f64]) {
        self.cost.copy_from_slice(c);
        self.std.c[..self.n].copy_from_slice(c);
        self.status = None;
        self.clear_state_memo();
        if let Some(t) = self.tw.as_mut() {
            let cs: Vec<f64> = c.iter().zip(self.tw_cs.iter()).map(|(v, s)| v * s).collect();
            t.set_costs(&cs);
        }
    }

    /// 行を加える。標準形を作り直し (`rebuild`: 凍結した行列を作り直し、PRICE 用の行列の識別子を変え、最適基底の LU の
    /// キャッシュを捨てる)、新しい行のスラックを基底に入れる。次の求解は基底を分解し直す (HiGHS は
    /// `kExtendInvertWhenAddingRows` で INVERT を拡張する)。コストの実測 (mik-250、HiGHS 流のカット管理で 40 秒、3192 回の
    /// 追加 + 削除): この関数 0.9 秒、`delete_rows` 0.7 秒、再分解 0.1 秒未満。LP の反復 (16 秒) に比べ小さいので、
    /// LU の拡張は実装していない (`ENOMOTO_MIP_XPROF` の `lp_add_rows` / `lp_del_rows` で測れる)。
    /// 行を加える。既存の基底に新しい行のスラックを足し、DSE 重みも引き継ぐ (新しい行の重みは 1)。
    /// `ENOMOTO_MIP_ROWS_DSE_EXACT` (試作、既定では使わない): 新しい行の重みを 1 ではなく「未知」(`NaN`) にし、次の求解が
    /// 分解した直後にその基底での厳密な値を BTRAN で埋める ([`super::DseState::fill_missing`]、行ごとに BTRAN 1 回)。
    /// 新しい行 k (スラックが基底) の `B^-1` の行は `[-a_k(B) B_old^-1, 1]` なので重みは `1 + |B_old^-T a_k(B)|^2` で、
    /// 1 からかけ離れる (physiciansched6-2 のカットの行で最大 1e3〜4e3)。古い行の `B^-1` の行は変わらないので引き継いだ
    /// 重みはそのまま正しい。重み 1 のままだと、カットを足した LP の行の選択が Dantzig 規則に近くなって反復が 10 倍以上に
    /// 増え (HiGHS の 2224 反復に対し 30948 反復)、根のカットのループの 1 回の LP の時間の上限で止まっていた
    /// (analysis/lp_hang_20261021.md)
    pub fn add_rows(&mut self, new_rows: &[(Vec<(usize, f64)>, f64, f64)]) {
        sid::xprof("lp_pre_add");
        self.add_rows_inner(new_rows, true);
        sid::xprof("lp_add_rows");
    }

    /// [`Self::add_rows`] の本体。`rebuild` が偽なら標準形を作り直さない (呼び出し側が後で 1 回だけ作り直す、
    /// [`Self::delete_and_add_rows`])。
    fn add_rows_inner(&mut self, new_rows: &[(Vec<(usize, f64)>, f64, f64)], rebuild: bool) {
        let m0 = self.rows.len();
        let new_w = if env_str!("ENOMOTO_MIP_ROWS_DSE_EXACT").is_some() { f64::NAN } else { 1.0 };
        for (r, l, u) in new_rows {
            let mut r: Vec<(usize, f64)> = r.iter().filter(|&&(_, v)| v != 0.0).cloned().collect();
            r.sort_unstable_by_key(|&(j, _)| j);
            self.rows.push(r);
            self.row_lo.push(*l);
            self.row_up.push(*u);
        }
        if rebuild {
            self.rebuild();
        }
        // 新しい行のスラックを基底に入れる (スラックの番号は n + i なので、既存の基底は番号が変わらない)
        let same = matches!((&self.basis, self.dse_cache.as_deref()), (Some(b), Some((cb, _))) if b == cb);
        if let Some(b) = self.basis.as_mut() {
            for i in m0..self.rows.len() {
                b.push(self.n + i);
            }
        }
        // DSE 重みも引き継ぎ、新しい行の重みは 1 (スラックが基底の行)
        if same {
            if let (Some(b), Some(c)) = (&self.basis, self.dse_cache.as_mut()) {
                let (cb, w) = Arc::make_mut(c);
                cb.clone_from(b);
                w.resize(b.len(), new_w);
            }
        } else {
            self.dse_cache = None;
        }
        self.y.resize(self.rows.len(), 0.0);
        self.status = None;
        let sync = self.tw_sync;
        self.clear_memo();
        if self.tw.is_some() {
            // 新しい行の尺度: 列の尺度をかけた係数の幾何平均 (`1 / sqrt(min max)`、2 の冪)。列の尺度は変えない
            let lim = 2f64.powi(20);
            let cs = self.tw_cs.clone();
            let mut srows: Vec<(Vec<(usize, f64)>, f64, f64)> = Vec::with_capacity(new_rows.len());
            for i in m0..self.rows.len() {
                let (mut mn, mut mx) = (f64::INFINITY, 0.0f64);
                for &(j, v) in &self.rows[i] {
                    let a = (v * cs[j]).abs();
                    mn = mn.min(a);
                    mx = mx.max(a);
                }
                let r = if mx > 0.0 && mn.is_finite() { (1.0 / (mn * mx).sqrt()).clamp(1.0 / lim, lim) } else { 1.0 };
                let r = if r.is_finite() && r > 0.0 { 2f64.powi(r.log2().round() as i32) } else { 1.0 };
                self.tw_rs.push(r);
                srows.push((self.rows[i].iter().map(|&(j, v)| (j, v * r * cs[j])).collect(), self.row_lo[i] * r, self.row_up[i] * r));
            }
            self.tw.as_mut().unwrap().add_rows_inner(&srows, rebuild);
            self.tw_sync = sync;
        }
    }

    pub fn delete_rows(&mut self, remove: &[bool]) {
        sid::xprof("lp_pre_del");
        self.delete_rows_inner(remove, true);
        sid::xprof("lp_del_rows");
    }

    /// 行を消してから行を加える ([`Self::delete_rows`] の後に [`Self::add_rows`] と同じ結果)。標準形の作り直し
    /// (`rebuild`) は最後に 1 回だけ行う (`ENOMOTO_MIP_CUT_KEEP_LU`: 根のカットのループで効いていないカットを外すのと
    /// 新しいカットを加えるのを 1 回にまとめる)。
    pub fn delete_and_add_rows(&mut self, remove: &[bool], new_rows: &[(Vec<(usize, f64)>, f64, f64)]) {
        sid::xprof("lp_pre_del");
        self.delete_rows_inner(remove, false);
        self.add_rows_inner(new_rows, true);
        sid::xprof("lp_del_add_rows");
    }

    /// 行列 (行の集合) の識別子 ([`Self::price_key`]。行の追加・削除・行の境界の変更で変わる。写しは同じ値を持つ)。
    pub fn matrix_key(&self) -> u64 {
        self.price_key
    }

    /// [`Self::delete_rows`] の本体。`rebuild` が偽なら標準形を作り直さない (直後に [`Self::add_rows_inner`] が作り直す)。
    /// 基底の修復 ([`Self::repair_basis`]) は標準形を見ないので、作り直しの前に行ってよい。
    fn delete_rows_inner(&mut self, remove: &[bool], rebuild: bool) {
        let m = self.rows.len();
        let mut new_index = vec![usize::MAX; m];
        let mut k = 0;
        for i in 0..m {
            if !remove[i] {
                new_index[i] = k;
                k += 1;
            }
        }
        let keep = |v: &mut Vec<f64>| {
            let mut i = 0;
            v.retain(|_| {
                i += 1;
                !remove[i - 1]
            });
        };
        keep(&mut self.row_lo);
        keep(&mut self.row_up);
        keep(&mut self.y);
        let mut i = 0;
        self.rows.retain(|_| {
            i += 1;
            !remove[i - 1]
        });
        let n = self.n;
        let cache = self.dse_cache.take();
        if env_str!("ENOMOTO_MIP_DEBUG_DSE_CHECK").is_some() {
            let nonbasic_removed = (0..m).filter(|&i| remove[i] && self.basis.as_ref().is_some_and(|b| !b.contains(&(n + i)))).count();
            eprintln!("DSE_CHECK delete_rows: removing {} rows, {} of them with a nonbasic slack", remove.iter().filter(|&&r| r).count(), nonbasic_removed);
        }
        if let Some(b) = self.basis.take() {
            let same = matches!(cache.as_deref(), Some((cb, w)) if *cb == b && w.len() == b.len());
            let mut nb: Vec<usize> = Vec::with_capacity(k);
            let mut nw: Vec<f64> = Vec::with_capacity(k);
            for (p, &v) in b.iter().enumerate() {
                let keep = v < n || !remove[v - n];
                if keep {
                    nb.push(if v < n { v } else { n + new_index[v - n] });
                    if same {
                        nw.push(cache.as_ref().unwrap().1[p]);
                    }
                }
            }
            if same {
                self.dse_cache = Some(Arc::new((nb.clone(), nw)));
            }
            self.basis = Some(nb);
        }
        if rebuild {
            self.rebuild();
        }
        self.repair_basis();
        // 基底を直したら重みは使わない
        if !matches!((&self.basis, self.dse_cache.as_deref()), (Some(b), Some((cb, _))) if b == cb) {
            self.dse_cache = None;
        }
        self.status = None;
        let sync = self.tw_sync;
        self.clear_memo();
        if let Some(t) = self.tw.as_mut() {
            let mut i = 0;
            self.tw_rs.retain(|_| {
                i += 1;
                !remove[i - 1]
            });
            t.delete_rows_inner(remove, rebuild);
            self.tw_sync = sync;
        }
    }

    /// 基底列の数を行数に合わせる (多ければ構造列を外し、足りなければ非基底の行のスラックを入れる)。
    fn repair_basis(&mut self) {
        let m = self.rows.len();
        let n = self.n;
        let Some(b) = self.basis.as_mut() else { return };
        let mut used = vec![false; n + m];
        b.retain(|&v| v < n + m && !std::mem::replace(&mut used[v], true));
        while b.len() > m {
            if let Some(pos) = b.iter().rposition(|&v| v < n) {
                let v = b.remove(pos);
                used[v] = false;
            } else {
                b.pop();
            }
        }
        let mut i = 0;
        while b.len() < m && i < m {
            if !used[n + i] {
                used[n + i] = true;
                b.push(n + i);
            }
            i += 1;
        }
    }

    pub fn basis(&self) -> Basis {
        self.basis_ref().clone()
    }

    /// [`Self::basis`] の参照 (同じ状態で何度呼んでも 1 回だけ作る)。
    pub fn basis_ref(&self) -> &Basis {
        self.basis_memo.get_or_init(|| self.compute_basis())
    }

    fn compute_basis(&self) -> Basis {
        let n = self.n;
        let m = self.rows.len();
        let mut col = vec![VarStatus::Lower; n];
        let mut row = vec![VarStatus::Lower; m];
        let mut is_basic = vec![false; n + m];
        if let Some(b) = &self.basis {
            for &v in b {
                is_basic[v] = true;
            }
        } else {
            for i in 0..m {
                is_basic[n + i] = true;
            }
        }
        for j in 0..n {
            col[j] = if is_basic[j] {
                VarStatus::Basic
            } else if self.lo[j].is_finite() && (self.x[j] - self.lo[j]).abs() <= 1e-9 * (1.0 + self.lo[j].abs()) {
                VarStatus::Lower
            } else if self.up[j].is_finite() && (self.x[j] - self.up[j]).abs() <= 1e-9 * (1.0 + self.up[j].abs()) {
                VarStatus::Upper
            } else if !self.lo[j].is_finite() && !self.up[j].is_finite() {
                VarStatus::Zero
            } else if self.lo[j].is_finite() {
                VarStatus::Lower
            } else {
                VarStatus::Upper
            };
        }
        // `ENOMOTO_MIP_BASIS_NB` (試作) を使うときは、範囲行でスラックが上限 (行の幅) にある行を反対の側 (σ = +1 なら下限) とする
        // (既定はスラックの位置を見ずに、スラック 0 の側とする)
        let ranged_side = basis_nb_any();
        for i in 0..m {
            // スラック 0 は σ = +1 なら行が上限、σ = -1 なら下限にある
            let at_su = ranged_side && !is_basic[n + i] && {
                let su = self.std.ub[n + i];
                su.is_finite() && su > 0.0 && {
                    let act = self.row_activities_ref()[i];
                    let sl = if self.sigma[i] > 0.0 { self.row_up[i] - act } else { act - self.row_lo[i] };
                    (sl - su).abs() <= 1e-9 * (1.0 + su.abs())
                }
            };
            row[i] = if is_basic[n + i] {
                VarStatus::Basic
            } else if (self.sigma[i] > 0.0) != at_su {
                VarStatus::Upper
            } else {
                VarStatus::Lower
            };
        }
        Basis { col, row }
    }

    pub fn set_basis(&mut self, b: &Basis) {
        let n = self.n;
        let mut list: Vec<usize> = (0..n).filter(|&j| b.col[j] == VarStatus::Basic).collect();
        list.extend((0..b.row.len().min(self.rows.len())).filter(|&i| b.row[i] == VarStatus::Basic).map(|i| n + i));
        self.basis = Some(list);
        self.repair_basis();
        self.lu = None;
        self.status = None;
        self.basis_nb = None;
        self.tw_sync = false;
        self.clear_state_memo();
        if super::nb_diag::on() {
            self.nb_diag = Some(Box::new(super::nb_diag::Pending::new(&b.col, &b.row, &self.sigma, n, self.rows.len())));
        }
    }

    /// 試作 (`ENOMOTO_MIP_BASIS_NB`): [`Self::set_basis`] に加えて、`b` の非基底の列・行の上限・下限 (Upper / Lower) を次の
    /// 求解の非基底列の置き場所の希望 ([`sid::set_warm_nb`]、[`Self::resume_nb`] と同じ -1 / +1) として渡す。既定の
    /// [`Self::set_basis`] は基底に入る列の集合だけを使い、置き場所は直前に解いた点 `x` からの推し量り ([`Self::nb_hint`]) で決める。
    /// `skip` の列 (分枝で固定・締めた列など、保存のときから境界の変わった列) と、Basic の列 (修復で基底から外れた列を含む)、
    /// 求解のときに指す側の境界が無限・固定 (下限 = 上限) の列は 0 (推し量りのまま) にする
    pub fn set_basis_nb(&mut self, b: &Basis, skip: &[usize]) {
        self.set_basis(b);
        self.resume_nb = None;
        let n = self.n;
        let m = self.rows.len();
        let mut h = vec![0i8; n + m];
        for j in 0..n.min(b.col.len()) {
            h[j] = match b.col[j] {
                VarStatus::Lower => -1,
                VarStatus::Upper => 1,
                _ => 0,
            };
        }
        for &j in skip {
            if j < n {
                h[j] = 0;
            }
        }
        // 行: σ = +1 ならスラック 0 (下限) は行が上限、σ = -1 なら行が下限
        for i in 0..m.min(b.row.len()) {
            let pos = self.sigma[i] > 0.0;
            h[n + i] = match (b.row[i], pos) {
                (VarStatus::Upper, true) | (VarStatus::Lower, false) => -1,
                (VarStatus::Lower, true) | (VarStatus::Upper, false) => 1,
                _ => 0,
            };
        }
        if let Some(bs) = &self.basis {
            for &v in bs {
                h[v] = 0;
            }
        }
        self.basis_nb = Some(h);
    }

    /// [`Self::basis_nb`] の希望を今の境界と照らして推し量り `h` に重ねる。変えた数を数える ([`BASIS_NB_STATS`])。
    fn apply_basis_nb(&self, h: &mut [i8], w: &[i8]) {
        let n = self.n;
        let (mut given, mut changed) = (0u64, 0u64);
        for k in 0..h.len().min(w.len()) {
            if w[k] == 0 {
                continue;
            }
            let (lo, up) = (self.std.lb[k], self.std.ub[k]);
            let ok = if k < n {
                lo < up && if w[k] < 0 { lo.is_finite() } else { up.is_finite() }
            } else {
                // スラックの下限は 0。等式行 (上限 0) は置き場所が意味を持たない
                up > 0.0 && (w[k] < 0 || up.is_finite())
            };
            if ok {
                given += 1;
                if h[k] != w[k] {
                    changed += 1;
                    h[k] = w[k];
                }
            }
        }
        use std::sync::atomic::Ordering::Relaxed;
        BASIS_NB_STATS[0].fetch_add(1, Relaxed);
        BASIS_NB_STATS[1].fetch_add(given, Relaxed);
        BASIS_NB_STATS[2].fetch_add(changed, Relaxed);
    }

    /// 今の LP (行・列の境界・費用) を内点法で解いた点 (元の空間の構造列の値) を返す。`zero_cost` なら費用 0
    /// (実行可能領域の中心付近の点)。`want_vertex` なら、その点からクロスオーバーの押し出しだけで作った頂点と
    /// 基底 (標準形の列番号: 構造列 j < n、スラック n + i) も返す。この基底は**双対実行可能とは限らない**ので、
    /// warm start ([`Self::set_basis_cols`]) にだけ使い、最適基底 (下界・被約費用) として使ってはいけない。
    /// `deadline` を過ぎたら打ち切って `None`。
    pub fn interior_point(&self, zero_cost: bool, want_vertex: bool, deadline: std::time::Instant) -> Option<InteriorPoint> {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let token = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicBool::new(false));
        let abort = IPM_ABORT.with(|a| a.borrow().clone());
        // 時間切れ (か、呼び手が渡した打ち切りの条件 [`with_ipm_abort`]) でトークンを立てる見張り (内点法・クロスオーバーはトークンを見て止まる)
        let watch = {
            let (token, done) = (token.clone(), done.clone());
            std::thread::spawn(move || {
                while !done.load(Ordering::Relaxed) {
                    if std::time::Instant::now() >= deadline || abort.as_ref().is_some_and(|f| f()) {
                        token.store(true, Ordering::Relaxed);
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            })
        };
        let r = crate::cancel::with_token(Some(token.clone()), || super::crossover::interior_point(&self.std, zero_cost, want_vertex));
        done.store(true, Ordering::Relaxed);
        let _ = watch.join();
        if token.load(Ordering::Relaxed) {
            return None;
        }
        let r = r?;
        let n = self.n;
        let to_orig = |z: &[f64]| -> Vec<f64> { (0..n).map(|j| z[j] + self.shift[j]).collect() };
        let x = to_orig(&r.x);
        let vertex = r.vertex.map(|(b, xv)| (b, xv.map(|v| to_orig(&v))));
        Some(InteriorPoint { x, vertex })
    }

    /// 基底を標準形の列番号 (長さ m) で直接与える (特異なら修復する)。`x` を渡すと、次の求解の非基底列の
    /// 置き場所の希望 ([`Self::nb_hint`]) をその点から作る。基底は双対実行可能でなくてよい (二段解法の warm start は
    /// 非基底列を被約費用の符号の側 (無限なら記号的な M) に置くか費用をずらして始め、最後に真の費用で仕上げる)。
    pub fn set_basis_cols(&mut self, b: &[usize], x: Option<&[f64]>) {
        if b.len() != self.rows.len() {
            return;
        }
        self.basis = Some(b.to_vec());
        self.repair_basis();
        if let Some(x) = x {
            self.x.copy_from_slice(&x[..self.n]);
        }
        self.lu = None;
        self.status = None;
        self.clear_memo();
    }

    pub fn basic_var(&self, s: usize) -> usize {
        match &self.basis {
            Some(b) => b[s],
            None => self.n + s,
        }
    }

    /// 今の基底の DSE 重み (直前の求解の最後の値。基底が変わっていなければ) を、基底変数の番号 (構造列 j < n、
    /// 行 n + i) との組で返す。待ち行列のノードに基底と一緒に保存する。
    pub fn dse_snapshot(&self) -> Option<Vec<(u32, f32)>> {
        // 双子で解いた後は双子の重み (基底変数の番号との組なので基底の並びによらない。本体の仕上げで基底が動いていても、
        // 戻すときに [`Self::set_dse_vars`] が今の基底にない変数を捨て、ない変数は 1 にする)。本体の重みは仕上げ (単位重みから始めて
        // ふつう 0 ピボット) のもので、保存すると戻したノードの LP が単位重みで始まる (uccase12 で 1 回 1 万反復超)
        if let Some(t) = self.tw.as_deref().filter(|_| self.tw_sync) {
            return t.dse_snapshot();
        }
        match (&self.basis, self.dse_cache.as_deref()) {
            (Some(b), Some((cb, w))) if b == cb && w.len() == b.len() => Some(b.iter().zip(w).map(|(&v, &x)| (v as u32, x as f32)).collect()),
            _ => None,
        }
    }

    /// [`Self::dse_snapshot`] の重みを今の基底に当てはめる (番号は今の LP の行に合わせてあること)。基底にない
    /// 変数の重みは捨て、重みのない基底変数は 1。次の求解が引き継ぐかは [`sid::WARM_DSE_ONCE`] などで決まる。
    pub fn set_dse_vars(&mut self, w: &[(u32, f32)]) {
        let Some(b) = &self.basis else { return };
        // 渡されなかった重みは `INFINITY` の印 (1 にする)。渡された `NaN` (未知、求解の始めに厳密な値を埋める) はそのまま
        let mut map = vec![f32::INFINITY; self.n + self.rows.len()];
        for &(v, x) in w {
            if (v as usize) < map.len() {
                map[v as usize] = x;
            }
        }
        // 試作 `ENOMOTO_MIP_DSE_INIT` の `m` ([`sid::dse_init_has`]): 重みのない基底変数は 1 でなく未知 (求解の始めに厳密な値で埋める)
        let missing = if sid::dse_init_has('m') { f64::NAN } else { 1.0 };
        let ws: Vec<f64> = b.iter().map(|&v| if map[v].is_infinite() { missing } else if map[v].is_nan() { f64::NAN } else { (map[v] as f64).max(1e-8) }).collect();
        self.dse_cache = Some(Arc::new((b.clone(), ws)));
        self.dse_age = 0;
    }

    /// 今の基底の DSE 重み ([`Self::dse_cache`]) の育ち具合 (重みを初期化してからの反復の数、厳密な値なら `u64::MAX`)。
    /// 重みがない (次の求解は単位重みから始まる) なら `None`。
    pub fn dse_age(&self) -> Option<u64> {
        if let Some(t) = self.tw_same() {
            return t.dse_age();
        }
        match (&self.basis, self.dse_cache.as_deref()) {
            (Some(b), Some((cb, w))) if b == cb && w.len() == b.len() => Some(self.dse_age),
            _ => None,
        }
    }

    /// 試作 `ENOMOTO_MIP_BG_CUTS_DSE=g`: 今の基底 (直前の最適基底) の厳密な DSE 重みを、手元の LU キャッシュから行ごとの
    /// BTRAN で計算し、次の求解に引き継ぐ重みにする。`stop` は 32 行ごとに見て、真ならやめて今の重みを残す。
    /// 計算したら `Some(true)`、やめたら `Some(false)`、同じ基底の LU がなければ `None`。
    pub fn recompute_dse_exact(&mut self, stop: &dyn Fn() -> bool) -> Option<bool> {
        if self.tw_same().is_some() {
            return self.tw.as_mut().unwrap().recompute_dse_exact(stop);
        }
        let b = self.basis.as_ref()?;
        let c = self.lu_cache.as_ref()?;
        if c.basis != *b || b.len() != self.rows.len() {
            return None;
        }
        match super::exact_dse_weights_stoppable(b.len(), &c.lu, stop) {
            Some(w) => {
                self.dse_cache = Some(Arc::new((b.clone(), w)));
                self.dse_age = u64::MAX;
                Some(true)
            }
            None => Some(false),
        }
    }

    /// 今の基底の LU キャッシュ (直前の最適基底のもので、基底がそのままなら) を DSE 重みと一緒に取り出す。
    pub fn lu_snapshot(&self) -> Option<LuSnapshot> {
        let tw = self.tw_same().and_then(|t| t.lu_snapshot()).map(Box::new);
        let c = match self.lu_cache.as_ref() {
            Some(c) if matches!(&self.basis, Some(b) if *b == c.basis) => c,
            // 本体の LU がない (双子の確かめで仕上げを省いた) なら双子の LU だけ
            _ => return tw.map(|tw| LuSnapshot { lu: None, dse: None, price_key: self.price_key, tw: Some(tw) }),
        };
        // `ENOMOTO_MIP_NO_SIBLING_LU_DSE`: DSE 重みは持たない (戻したときは置いた基底の重みがあればそれを使う)
        let dse = match &self.dse_cache {
            Some(d) if d.0 == c.basis && d.1.len() == c.basis.len() && env_str!("ENOMOTO_MIP_NO_SIBLING_LU_DSE").is_none() => Some(d.clone()),
            _ => None,
        };
        Some(LuSnapshot { lu: Some(c.clone()), dse, price_key: self.price_key, tw })
    }

    /// [`Self::lu_snapshot`] を戻す。行列が取り出したときのままで、今の基底が LU の基底と同じ集合のときだけ、基底を
    /// LU の基底位置の順に並べ直して LU キャッシュと DSE 重みを差し替え、真を返す ([`Self::set_basis`] は番号順に
    /// 並べるので、並びを合わせないと次の求解が LU を使わない)。それ以外は何もしない。
    pub fn adopt_lu_snapshot(&mut self, s: &LuSnapshot) -> bool {
        // 試作 `ENOMOTO_MIP_LPSCALE`: 双子の LU があれば先に双子に戻す (次の求解は双子で解くので、本体の LU はなくてよい)
        if let (Some(stw), true) = (s.tw.as_deref(), self.tw.is_some()) {
            if s.price_key != self.price_key {
                return false;
            }
            let self_ok = s.lu.is_some() && self.adopt_lu_snapshot_own(s);
            if !self_ok {
                self.tw_sync = false;
            }
            // 本体の基底を双子に写してから (同じ集合のときだけ) 双子の LU の並びに直す
            self.tw_pull();
            if self.tw.as_mut().unwrap().adopt_lu_snapshot(stw) {
                if !self_ok {
                    self.basis = self.tw.as_ref().unwrap().basis.clone();
                    self.lu = None;
                    self.status = None;
                    self.clear_state_memo();
                }
                self.tw_sync = true;
                return true;
            }
            self.tw_sync = false;
            return self_ok;
        }
        if s.lu.is_none() {
            return false;
        }
        self.adopt_lu_snapshot_own(s)
    }

    /// [`Self::adopt_lu_snapshot`] の本体の LU の分 (`s.lu` があること)
    fn adopt_lu_snapshot_own(&mut self, s: &LuSnapshot) -> bool {
        let m = self.rows.len();
        let nt = self.n + m;
        let Some(slu) = s.lu.as_ref() else { return false };
        let cb = &slu.basis;
        if s.price_key != self.price_key || cb.len() != m {
            return false;
        }
        let Some(b) = &self.basis else { return false };
        if b.len() != m {
            return false;
        }
        // 同じ集合か (どちらも重複のない m 個)
        let mut pos = vec![usize::MAX; nt];
        for (k, &v) in b.iter().enumerate() {
            if v < nt {
                pos[v] = k;
            }
        }
        if cb.iter().any(|&v| v >= nt || pos[v] == usize::MAX) {
            return false;
        }
        // DSE 重み: 取り出したときの重み、なければ今の基底の重み (ノードに保存した重みなど) を並べ直す
        let dse: Option<Arc<(Vec<usize>, Vec<f64>)>> = match (&s.dse, self.dse_cache.as_deref()) {
            (Some(d), _) if d.1.len() == m => Some(d.clone()),
            (_, Some((db, w))) if db == b && w.len() == m => Some(Arc::new((cb.clone(), cb.iter().map(|&v| w[pos[v]]).collect::<Vec<f64>>()))),
            _ => None,
        };
        sid::xcount("n_sib_lu");
        self.basis = Some(cb.clone());
        self.dse_cache = dse;
        self.dse_age = 0;
        self.lu_cache = Some(slu.clone());
        self.lu = None;
        self.status = None;
        self.tw_sync = false;
        self.clear_state_memo();
        true
    }

    /// 双子が本体と同じ基底を持ち、次の求解の出発点になっているなら双子
    fn tw_same(&self) -> Option<&TwoStageLp> {
        let t = self.tw.as_deref()?;
        if !self.tw_sync {
            return None;
        }
        let (Some(a), Some(b)) = (t.basis.as_ref(), self.basis.as_ref()) else { return None };
        if a.len() != b.len() {
            return None;
        }
        if a == b {
            return Some(t);
        }
        // 同じ集合か (並びは仕上げの分解で変わりうる)
        let mut seen = vec![false; self.n + self.rows.len()];
        for &v in a {
            if v < seen.len() {
                seen[v] = true;
            }
        }
        b.iter().all(|&v| v < seen.len() && seen[v]).then_some(t)
    }

    /// 双子を本体に合わせる (本体の基底が外から置かれた後、双子で解く前): 基底・解 (`x / c`)・非基底列の置き場所の希望・
    /// (同じ基底の) DSE 重み。双子の LU キャッシュは双子の中で基底と照らして使う
    fn tw_pull(&mut self) {
        let Some(t) = self.tw.as_mut() else { return };
        t.clear_memo();
        t.basis = self.basis.clone();
        for j in 0..self.n {
            t.x[j] = self.x[j] / self.tw_cs[j];
        }
        t.resume_nb = self.resume_nb.clone();
        t.basis_nb = self.basis_nb.clone();
        t.dse_cache = self.dse_cache.clone();
        t.dse_age = self.dse_age;
        t.lu = None;
        t.status = None;
        t.ray = None;
        t.intercept = None;
        self.tw_sync = true;
    }

    pub fn dse_weight(&self, _s: usize) -> f64 {
        1.0
    }

    pub fn last_status(&self) -> Option<LpStatus> {
        self.status
    }

    /// LU と DSE 重みは `Arc` の共有 (複製しない)。
    pub fn save_state(&self) -> TwoStageState {
        let tw = self.tw.as_deref().map(|t| Box::new(TwState { basis: t.basis.clone(), x: t.x.clone(), lu_cache: t.lu_cache.clone(), dse_cache: t.dse_cache.clone(), dse_age: t.dse_age, sync: self.tw_sync }));
        TwoStageState { lo: self.lo.clone(), up: self.up.clone(), basis: self.basis.clone(), x: self.x.clone(), y: self.y.clone(), status: self.status, lu_cache: self.lu_cache.clone(), dse_cache: self.dse_cache.clone(), dse_age: self.dse_age, tw }
    }

    pub fn restore_state(&mut self, s: &TwoStageState) {
        for j in 0..self.n {
            if self.lo[j] != s.lo[j] || self.up[j] != s.up[j] {
                self.set_col_bounds(j, s.lo[j], s.up[j]);
            }
        }
        if self.basis != s.basis {
            self.lu = None;
        }
        self.basis = s.basis.clone();
        self.lu_cache = s.lu_cache.clone();
        self.dse_cache = s.dse_cache.clone();
        self.dse_age = s.dse_age;
        self.x.clone_from(&s.x);
        self.y.clone_from(&s.y);
        self.status = s.status;
        self.clear_memo();
        // 双子も保存したときの状態に戻す (列の境界は `set_col_bounds` が写した)
        if let (Some(t), Some(ts)) = (self.tw.as_mut(), s.tw.as_deref()) {
            if t.basis != ts.basis {
                t.lu = None;
            }
            t.basis = ts.basis.clone();
            t.lu_cache = ts.lu_cache.clone();
            t.dse_cache = ts.dse_cache.clone();
            t.dse_age = ts.dse_age;
            t.x.clone_from(&ts.x);
            t.status = None;
            t.clear_memo();
            self.tw_sync = ts.sync;
        }
    }

    /// LP を解く。基底がなく (warm start できない)、このスレッドの [`set_cold_race_threads`] が 2 以上なら、
    /// 二段解法と内点法 + クロスオーバーで同時に解く ([`Self::solve_race`])。それ以外は二段解法だけ ([`Self::solve_once`])。
    pub fn solve(&mut self, lim: &SolveLimits) -> LpStatus {
        let cold = !matches!(&self.basis, Some(b) if b.len() == self.rows.len());
        if cold && crate::cancel::current().is_none() {
            let k = COLD_RACE_THREADS.with(|c| c.get());
            if k >= 2 {
                // 試作 `ENOMOTO_MIP_MC11FIX1_RACE` (既定では使わない、analysis/mc11_wait1_20261103.md): 小さい LP (行 + 列 + 非零の数が
                // `ENOMOTO_T_MIP_MC11FIX1_RACE_SIZE` (既定 50000) 未満) では、勝ち負けを時間で決めない。`s` は同時実行をやめて二段解法だけ、
                // `i` は二段解法が先に最適で終わっても内点法 + クロスオーバーの頂点を待って採る。mc11 (大きさ 約 1.1 万) では両方が 0.04〜0.06 秒で
                // ほぼ同着になり、どちらが勝つかで根の頂点が変わって、根のカットと裏のカットの道が 11683 (二段解法、約 15 秒) と
                // 11666 (内点法、約 30 秒) に分かれていた
                if let Some(v) = env_str!("ENOMOTO_MIP_MC11FIX1_RACE") {
                    let nnz: usize = self.rows.iter().map(|r| r.len()).sum();
                    let size = self.std.n_total + nnz;
                    let small = (size as f64) < tunable!("ENOMOTO_T_MIP_MC11FIX1_RACE_SIZE", 50000.0, f64);
                    if env_str!("ENOMOTO_DEBUG_RACE").is_some() {
                        eprintln!("MIP root race: ENOMOTO_MIP_MC11FIX1_RACE={v}: size {size} ({})", if small { "small" } else { "race as usual" });
                    }
                    if small && v.contains('s') {
                        return self.solve_once(lim);
                    }
                    if small && v.contains('i') {
                        return self.solve_race_ext(lim, k, true);
                    }
                }
                if let Some(cfg) = BOTH_REQ.with(|c| c.take()) {
                    return self.solve_race_both(lim, k, cfg);
                }
                return self.solve_race(lim, k);
            }
        }
        self.solve_once(lim)
    }

    /// 二段解法で 1 回解く。
    fn solve_once(&mut self, lim: &SolveLimits) -> LpStatus {
        if self.iters == 0 && env_str!("ENOMOTO_MIP_XPROF").is_some() {
            sid::xprof_enable(true);
        }
        let t0 = std::time::Instant::now();
        let it0 = self.iters;
        let pol0 = sid::ext_polish_iterations();
        let scale_once = std::mem::take(&mut self.scale_once);
        // 調べ用 (`ENOMOTO_SIMPLEX_SCALE_LOG`): 根の LP・最初の LP の大きさと係数の幅、条件に当たったか (analysis/rootlp_scale_20261106.md)
        if scale_once && env_str!("ENOMOTO_SIMPLEX_SCALE_LOG").is_some() {
            let warm = matches!(&self.basis, Some(b) if b.len() == self.rows.len());
            eprintln!("ROOTLP_SCALE: rows {} cols {} range {:.3e} warm {warm} applies {}", self.rows.len(), self.n, coef_range(&self.rows), self.scale_once_applies());
        }
        let st = if self.tw.is_some() {
            self.solve_tw(lim)
        } else if (scale_once && self.scale_once_applies())
            || (scale_cold_on()
                && !matches!(&self.basis, Some(b) if b.len() == self.rows.len())
                && self.rows.len() >= tunable!("ENOMOTO_T_SIMPLEX_SCALE_MIN_M", 10000usize, usize)
                && coef_range(&self.rows) >= tunable!("ENOMOTO_T_SIMPLEX_SCALE_MIN_RANGE", 1e4, f64))
        {
            self.solve_scaled(lim)
        } else {
            self.solve_inner(lim)
        };
        // 試作 `ENOMOTO_MIP_LPSCALE=h`: 尺度なしの求解が重かった (または数値の破綻) なら、この LP に双子を作って以後は双子で解き、
        // 以後に作る LP も双子を持つ印を立てる。破綻ならすぐ双子で解き直す
        let mut st = st;
        if self.tw_lazy && self.tw.is_none() {
            let secs = t0.elapsed().as_secs_f64();
            let failed = st == LpStatus::Error;
            if failed || secs >= tunable!("ENOMOTO_T_MIP_LPSCALE_HEAVY_SECS", 1.0f64, f64) {
                self.tw_lazy = false;
                self.attach_tw();
                if self.tw.is_some() {
                    LPSCALE_ARMED.store(true, std::sync::atomic::Ordering::Relaxed);
                    lpscale_add(14, 1);
                    if env_str!("ENOMOTO_MIP_LPSCALE_LOG").is_some() {
                        eprintln!("LPSCALE: heavy unscaled solve ({st:?}, {} iters, {secs:.2}s, {} rows); switching to the scaled twin", self.iters - it0, self.rows.len());
                    }
                    if failed {
                        st = self.solve_tw(lim);
                    }
                }
            }
        }
        if super::nb_diag::on() {
            super::nb_diag::record(self.nb_diag.take().map(|p| *p), self.iters - it0, t0.elapsed().as_secs_f64());
        }
        if env_str!("ENOMOTO_MIP_DEBUG_LP").is_some() {
            eprintln!("TSLP solve st={st:?} iters={} polish={} us={} warm={} dse_cache={} lu_cache={}", self.iters - it0, sid::ext_polish_iterations() - pol0, t0.elapsed().as_micros(), self.basis.is_some(), self.dse_cache.is_some(), self.lu_cache.is_some());
        }
        self.status = Some(st);
        st
    }

    /// 試作 `ENOMOTO_MIP_LPSCALE` の求解 (analysis/mip_lp_scale_20261106.md 1.2 節): 双子 (尺度をかけた LP) で解き、元の尺度で確かめる。
    /// 最適なら双子の基底と解から本体を warm start で解き直す (仕上げ)。打ち切り・実行不能は双子の双対・射線を元の尺度に戻して本体の
    /// 標準形で確かめて採り、確かめられなければ本体で (双子に渡す前の基底から) 解き直す。非有界・数値の破綻も本体で解き直す。
    /// 時間・反復の上限はそのまま返す (本体は変えない)
    fn solve_tw(&mut self, lim: &SolveLimits) -> LpStatus {
        let t0 = std::time::Instant::now();
        if !self.tw_sync {
            self.tw_pull();
        }
        lpscale_add(0, 1);
        let t = self.tw.as_mut().unwrap();
        let it0 = t.iters;
        let st_t = t.solve_inner(lim);
        let it_t = t.iters - it0;
        t.status = Some(st_t);
        self.iters += it_t;
        lpscale_add(1, it_t);
        lpscale_add(10, t0.elapsed().as_micros() as u64);
        let debug = env_str!("ENOMOTO_MIP_LPSCALE_LOG").is_some_and(|v| v == "2");
        let n = self.n;
        let m = self.rows.len();
        // 双子で解いた後は、双子の基底・解を次の求解の出発点にする (本体で解き直したら `solve_inner0` が偽にする)
        let fall_back = |me: &mut Self, why: &str| -> LpStatus {
            lpscale_add(6, 1);
            if debug {
                eprintln!("LPSCALE: twin {st_t:?} ({it_t} iters): {why}; re-solving the unscaled LP");
            }
            let left = lim.iteration_limit.saturating_sub(it_t);
            me.solve_inner(&SolveLimits { iteration_limit: left, ..*lim })
        };
        match st_t {
            LpStatus::Optimal => {
                let t = self.tw.as_ref().unwrap();
                let Some(b) = t.basis.clone().filter(|b| b.len() == m) else {
                    return fall_back(self, "no basis");
                };
                let xo: Vec<f64> = (0..n).map(|j| t.x[j] * self.tw_cs[j]).collect();
                let yo: Vec<f64> = if t.y.len() == m { (0..m).map(|i| t.y[i] * self.tw_rs[i]).collect() } else { Vec::new() };
                // 元の尺度で主・双対の実行可能性を確かめ、満たせば本体の仕上げ (LU の分解) を省く
                // (`ENOMOTO_MIP_LPSCALE_POLISH=1` ならいつも仕上げる)
                let tc = std::time::Instant::now();
                let ok = env_str!("ENOMOTO_MIP_LPSCALE_POLISH").is_none_or(|v| v == "0") && yo.len() == m && self.tw_check(&xo, &yo, &b);
                lpscale_add(13, tc.elapsed().as_micros() as u64);
                if ok {
                    lpscale_add(11, 1);
                    self.x = xo;
                    self.y = yo;
                    self.basis = Some(b);
                    self.lu = None;
                    self.lu_cache = None;
                    self.dse_cache = None;
                    self.ray = None;
                    self.intercept = None;
                    self.clear_memo();
                    self.tw_sync = true;
                    return LpStatus::Optimal;
                }
                lpscale_add(12, 1);
                self.x = xo;
                self.basis = Some(b);
                self.lu = None;
                self.lu_cache = None;
                self.dse_cache = None;
                self.status = None;
                self.clear_memo();
                let t1 = std::time::Instant::now();
                let it1 = self.iters;
                // 仕上げの反復の上限: 残り (双子が使い切っていても仕上げは少しは回す)
                let left = lim.iteration_limit.saturating_sub(it_t).max(50);
                let st = self.solve_inner(&SolveLimits { iteration_limit: left, ..*lim });
                let it_p = self.iters - it1;
                lpscale_add(2, it_p);
                // 反復の数は最適の確かめの 1 回を含む (ピボットなしでも 1)
                if it_p > 1 {
                    lpscale_add(3, 1);
                }
                lpscale_add(9, t1.elapsed().as_micros() as u64);
                if debug {
                    eprintln!("LPSCALE: twin Optimal {it_t} iters {:.3}s; unscaled polish {st:?} {it_p} iters {:.3}s", t1.duration_since(t0).as_secs_f64(), t1.elapsed().as_secs_f64());
                }
                self.tw_sync = true;
                st
            }
            LpStatus::ObjectiveBound => {
                let t = self.tw.as_ref().unwrap();
                if t.y.len() != m {
                    return fall_back(self, "no duals");
                }
                let y: Vec<f64> = (0..m).map(|i| t.y[i] * self.tw_rs[i]).collect();
                let const_obj: f64 = (0..n).map(|j| self.cost[j] * self.shift[j]).sum();
                let lb = self.std_lower_bound(&y) + const_obj;
                if !(lb + 1e-12 * (1.0 + lim.cutoff.abs()) >= lim.cutoff) {
                    return fall_back(self, &format!("bound {lb:e} below the cutoff {:e} in the original scale", lim.cutoff));
                }
                lpscale_add(4, 1);
                self.y = y;
                self.ray = None;
                self.intercept = None;
                self.clear_state_memo();
                self.tw_sync = true;
                LpStatus::ObjectiveBound
            }
            LpStatus::Infeasible => {
                let t = self.tw.as_ref().unwrap();
                let Some(ray) = t.ray.as_ref().filter(|r| r.len() == m) else {
                    return fall_back(self, "no ray");
                };
                // 元の行の向きの射線 (双子の行は r 倍)
                let ray_o: Vec<f64> = (0..m).map(|i| ray[i] * self.tw_rs[i]).collect();
                let ystd: Vec<f64> = (0..m).map(|i| ray_o[i] * self.sigma[i]).collect();
                if !self.std_farkas_ok(&ystd) {
                    return fall_back(self, "the ray does not prove infeasibility in the original scale");
                }
                lpscale_add(5, 1);
                self.ray = Some(ray_o);
                self.intercept = None;
                self.clear_state_memo();
                self.tw_sync = true;
                LpStatus::Infeasible
            }
            LpStatus::TimeLimit | LpStatus::IterationLimit => {
                lpscale_add(7, 1);
                self.tw_sync = true;
                st_t
            }
            _ => fall_back(self, "not conclusive"),
        }
    }

    /// 双子の最適な基底解を元の尺度に戻した `x` (構造列)・`y` (標準形の行の双対)・基底 `b` が、本体の標準形で最適と言えるか:
    /// 非基底の列が境界にあり (`1e-9 (1 + |境界|)`)、被約費用の符号が置き場所と合い (`1e-7 (1 + |c|)`、[`Self::adopt_basic_solution`] と同じ)、
    /// 基底の列とスラックが境界の内側にある (`1e-9 (1 + max(|境界|, 行の項の大きさ))`)
    fn tw_check(&self, x: &[f64], y: &[f64], b: &[usize]) -> bool {
        let n = self.n;
        let m = self.rows.len();
        let nt = n + m;
        let mut basic = vec![false; nt];
        for &v in b {
            if v >= nt || std::mem::replace(&mut basic[v], true) {
                return false;
            }
        }
        let dbg = env_str!("ENOMOTO_MIP_LPSCALE_LOG").is_some_and(|v| v == "3");
        for v in 0..nt {
            let (z, mag) = if v < n {
                (x[v] - self.shift[v], 0.0)
            } else {
                let i = v - n;
                let mut act = 0.0;
                let mut mag = 0.0f64;
                for &(j, a) in &self.rows[i] {
                    act += a * x[j];
                    mag += (a * x[j]).abs();
                }
                let sl = if self.sigma[i] > 0.0 { self.row_up[i] - act } else { act - self.row_lo[i] };
                (sl, mag)
            };
            let (l, u) = (self.std.lb[v], self.std.ub[v]);
            let tl = 1e-9 * (1.0 + l.abs().max(mag));
            let tu = 1e-9 * (1.0 + u.abs().max(mag));
            if basic[v] {
                if (l.is_finite() && z < l - tl) || (u.is_finite() && z > u + tu) || !z.is_finite() {
                    if dbg {
                        eprintln!("LPSCALE check: basic {v} (n={n}) infeasible: z={z:e} l={l:e} u={u:e}");
                    }
                    return false;
                }
                continue;
            }
            let at_l = l.is_finite() && (z - l).abs() <= tl;
            let at_u = u.is_finite() && (z - u).abs() <= tu;
            let free = !l.is_finite() && !u.is_finite() && z.abs() <= 1e-9;
            if !(at_l || at_u || free) {
                if dbg {
                    eprintln!("LPSCALE check: nonbasic {v} (n={n}) off its bounds: z={z:e} l={l:e} u={u:e}");
                }
                return false;
            }
            if at_l && at_u {
                continue;
            }
            let d = self.std.c[v] - self.std.cols.col(v).iter().map(|&(i, a)| a * y[i]).sum::<f64>();
            let sc = 1e-7 * (1.0 + self.std.c[v].abs());
            let ok = if at_l { d >= -sc } else if at_u { d <= sc } else { d.abs() <= sc };
            if !ok {
                if dbg {
                    eprintln!("LPSCALE check: nonbasic {v} (n={n}) dual infeasible: d={d:e} at_l={at_l} at_u={at_u}");
                }
                return false;
            }
        }
        true
    }

    /// 本体の標準形での下界 `L(y) = b·y + Σ_j min_{lb_j <= x_j <= ub_j} (c - A^T y)_j x_j` (標準形の空間、定数なし。
    /// `slope_intercept_dual::lower_bound_from_basis` と同じ式・同じ小さい値の扱い)
    fn std_lower_bound(&self, y: &[f64]) -> f64 {
        let std = &self.std;
        let ymax = y.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let tiny = 1e-9 * (1.0 + ymax);
        let mut lb: f64 = std.b.iter().zip(y).map(|(b, y)| b * y).sum();
        for j in 0..std.n_total {
            let mut d = std.c[j];
            for &(i, a) in std.cols.col(j) {
                d -= a * y[i];
            }
            if d > 0.0 {
                if std.lb[j].is_finite() {
                    lb += d * std.lb[j];
                } else if d > tiny {
                    return f64::NEG_INFINITY;
                }
            } else if d < 0.0 {
                if std.ub[j].is_finite() {
                    lb += d * std.ub[j];
                } else if -d > tiny {
                    return f64::NEG_INFINITY;
                }
            }
        }
        lb
    }

    /// 本体の標準形 `A x = b, lb <= x <= ub` で、行の乗数 `y` が実行不能を示すか (Farkas: `y·b` が箱の上の `(A^T y)·x` の範囲の外)。
    /// 無限の境界の向きの `A^T y` の成分は `1e-9 (1 + max |y|)` 以下なら 0 とみなす ([`Self::std_lower_bound`] と同じ)
    fn std_farkas_ok(&self, y: &[f64]) -> bool {
        let std = &self.std;
        let ymax = y.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        if !(ymax > 0.0) || !ymax.is_finite() {
            return false;
        }
        let tiny = 1e-9 * (1.0 + ymax);
        let yb: f64 = std.b.iter().zip(y).map(|(b, y)| b * y).sum();
        let (mut zlo, mut zhi, mut mag) = (0.0f64, 0.0f64, yb.abs());
        let (mut lo_inf, mut hi_inf) = (false, false);
        for j in 0..std.n_total {
            let mut z = 0.0;
            for &(i, a) in std.cols.col(j) {
                z += a * y[i];
            }
            if z == 0.0 {
                continue;
            }
            let (l, u) = (std.lb[j], std.ub[j]);
            // z x の最小と最大
            let (vmin, vmax) = if z > 0.0 { (z * l, z * u) } else { (z * u, z * l) };
            let small = z.abs() <= tiny;
            if vmin.is_finite() {
                zlo += vmin;
                mag += vmin.abs();
            } else if !small {
                lo_inf = true;
            }
            if vmax.is_finite() {
                zhi += vmax;
                mag += vmax.abs();
            } else if !small {
                hi_inf = true;
            }
        }
        let tol = 1e-9 * (1.0 + mag);
        (!hi_inf && yb > zhi + tol) || (!lo_inf && yb < zlo - tol)
    }

    /// 試作 `ENOMOTO_XO_MIP_SCALE` の写し ([`IpmScale`])。環境変数がなければ `None`。
    fn ipm_scale(&self) -> Option<Arc<IpmScale>> {
        if !env_str!("ENOMOTO_XO_MIP_SCALE").is_some_and(|v| v != "0") {
            return None;
        }
        // 係数の幅 (最大 / 最小の絶対値) が `ENOMOTO_T_XO_MIP_SCALE_MIN_RANGE` (既定 1e5) 未満なら使わない (opm2-z10-s4 (幅 4.1e3) は
        // 尺度をそろえた写しで基底の選択が 600 秒戻らなかった。momentum1 は幅 7.9e9、satellites2-40 は 6.6e5)
        let (mut amin, mut amax) = (f64::INFINITY, 0.0f64);
        for r in &self.rows {
            for &(_, v) in r {
                let a = v.abs();
                if a > 0.0 {
                    amin = amin.min(a);
                    amax = amax.max(a);
                }
            }
        }
        if !(amax / amin >= tunable!("ENOMOTO_T_XO_MIP_SCALE_MIN_RANGE", 1e5f64, f64)) {
            return None;
        }
        let (rs, cs) = scale_factors(self.n, &self.rows);
        let rows: Vec<Vec<(usize, f64)>> = self.rows.iter().enumerate().map(|(i, r)| r.iter().map(|&(j, v)| (j, v * rs[i] * cs[j])).collect()).collect();
        let row_lo: Vec<f64> = (0..rs.len()).map(|i| self.row_lo[i] * rs[i]).collect();
        let row_up: Vec<f64> = (0..rs.len()).map(|i| self.row_up[i] * rs[i]).collect();
        let lo: Vec<f64> = (0..self.n).map(|j| self.lo[j] / cs[j]).collect();
        let up: Vec<f64> = (0..self.n).map(|j| self.up[j] / cs[j]).collect();
        let cost: Vec<f64> = (0..self.n).map(|j| self.cost[j] * cs[j]).collect();
        let sc = TwoStageLp::new_raw(&lo, &up, &cost, &rows, &row_lo, &row_up);
        let no_hint = amax / amin >= tunable!("ENOMOTO_T_XO_MIP_SCALE_NOHINT_RANGE", 1e8f64, f64);
        Some(Arc::new(IpmScale { std: sc.std, no_hint, rs, cs, sc_shift: sc.shift, shift: self.shift.clone() }))
    }

    /// `ENOMOTO_MIP_ROOTLP_SCALE` (既定で使う) (analysis/momentum1_rootlp_20261106.md): 次の求解 1 回 (MIP の根の LP・前処理の LP による固定の
    /// 最初の LP) だけ、基底の有無によらず尺度を調整した写しで解く ([`Self::solve_scaled`])。行の数が `ENOMOTO_T_MIP_ROOTLP_SCALE_MIN_M`
    /// (既定 10000) 未満、または係数の幅 ([`coef_range`]) が `ENOMOTO_T_MIP_ROOTLP_SCALE_MIN_RANGE` (既定 1e4) 未満なら何もしない
    pub fn request_scale_once(&mut self) {
        self.scale_once = true;
    }

    fn scale_once_applies(&self) -> bool {
        self.rows.len() >= tunable!("ENOMOTO_T_MIP_ROOTLP_SCALE_MIN_M", 10000usize, usize)
            && coef_range(&self.rows) >= tunable!("ENOMOTO_T_MIP_ROOTLP_SCALE_MIN_RANGE", 1e4f64, f64)
    }

    /// 根の LP の安全網 ([`crate::mip`] の根の LP が `Error`・`IterationLimit` で終わったとき): 尺度を調整した写しを基底なしで解き、
    /// 最適ならその基底から元の LP を warm start で解き直す ([`Self::solve_scaled`])。係数の幅が `ENOMOTO_T_MIP_ROOTLP_RESCUE_MIN_RANGE`
    /// (既定 1e4) 未満なら `None` (試さない)
    pub fn solve_scaled_rescue(&mut self, lim: &SolveLimits) -> Option<LpStatus> {
        if coef_range(&self.rows) < tunable!("ENOMOTO_T_MIP_ROOTLP_RESCUE_MIN_RANGE", 1e4f64, f64) {
            return None;
        }
        self.basis = None;
        self.lu = None;
        self.lu_cache = None;
        self.dse_cache = None;
        self.resume_nb = None;
        self.basis_nb = None;
        self.status = None;
        self.clear_memo();
        let st = self.solve_scaled(lim);
        self.status = Some(st);
        Some(st)
    }

    /// 試作 `ENOMOTO_SIMPLEX_SCALE` ([`scale_cold_on`]) の本体: 行と列の尺度を調整した写し ([`scale_factors`]) を基底なしで解き、
    /// 最適ならその基底 (尺度によらない) から元の LP を warm start で解き直す。写しが最適で終わらなければ、時間・反復の上限はそのまま返し、
    /// それ以外 (実行不能・非有界・数値の破綻など) は元の LP を基底なしで解き直す (証明・双対を元の尺度で得るため)。
    fn solve_scaled(&mut self, lim: &SolveLimits) -> LpStatus {
        let t0 = std::time::Instant::now();
        let (rs, cs) = scale_factors(self.n, &self.rows);
        let rows: Vec<Vec<(usize, f64)>> = self.rows.iter().enumerate().map(|(i, r)| r.iter().map(|&(j, v)| (j, v * rs[i] * cs[j])).collect()).collect();
        let row_lo: Vec<f64> = (0..rs.len()).map(|i| self.row_lo[i] * rs[i]).collect();
        let row_up: Vec<f64> = (0..rs.len()).map(|i| self.row_up[i] * rs[i]).collect();
        let lo: Vec<f64> = (0..self.n).map(|j| self.lo[j] / cs[j]).collect();
        let up: Vec<f64> = (0..self.n).map(|j| self.up[j] / cs[j]).collect();
        let cost: Vec<f64> = (0..self.n).map(|j| self.cost[j] * cs[j]).collect();
        let mut sc = TwoStageLp::new_raw(&lo, &up, &cost, &rows, &row_lo, &row_up);
        // 基底があれば (`ENOMOTO_MIP_ROOTLP_SCALE` の warm start) 写しに渡す。基底と非基底の置き場所は尺度によらない。
        // 解 `x` は写しの尺度に直す (`x' = x / c`)。DSE 重みは尺度による (元の LP の重みは写しでは使わない)
        let warm = matches!(&self.basis, Some(b) if b.len() == self.rows.len());
        if warm {
            sc.basis = self.basis.clone();
            for j in 0..self.n {
                sc.x[j] = self.x[j] / cs[j];
            }
            sc.resume_nb = self.resume_nb.clone();
            sc.basis_nb = self.basis_nb.take();
        }
        let st_s = sc.solve_inner(lim);
        let t_s = t0.elapsed().as_secs_f64();
        self.iters += sc.iters;
        let debug = env_str!("ENOMOTO_SIMPLEX_SCALE_LOG").is_some();
        let st = match (st_s, sc.basis.take()) {
            (LpStatus::Optimal, Some(b)) if b.len() == self.rows.len() => {
                for j in 0..self.n {
                    self.x[j] = sc.x[j] * cs[j];
                }
                self.basis = Some(b);
                self.lu_cache = None;
                self.dse_cache = None;
                self.resume_nb = None;
                self.clear_memo();
                let it1 = self.iters;
                let st = self.solve_inner(lim);
                if debug {
                    eprintln!("SIMPLEX_SCALE: scaled{} {st_s:?} {} iters {t_s:.3}s; unscaled warm {st:?} {} iters {:.3}s", if warm { " (from the basis)" } else { "" }, sc.iters, self.iters - it1, t0.elapsed().as_secs_f64() - t_s);
                }
                st
            }
            (LpStatus::TimeLimit | LpStatus::IterationLimit, _) => {
                if debug {
                    eprintln!("SIMPLEX_SCALE: scaled {st_s:?} {} iters {t_s:.3}s", sc.iters);
                }
                st_s
            }
            _ => {
                let it1 = self.iters;
                let st = self.solve_inner(lim);
                if debug {
                    eprintln!("SIMPLEX_SCALE: scaled {st_s:?} {} iters {t_s:.3}s; unscaled cold {st:?} {} iters {:.3}s", sc.iters, self.iters - it1, t0.elapsed().as_secs_f64() - t_s);
                }
                st
            }
        };
        st
    }

    /// 診断用 (`ENOMOTO_MIP_ROOTLP_SIMPLEX_DIAG`): 同時実行をせず二段解法だけで 1 回解く ([`Self::solve_once`])。
    pub fn solve_simplex_only(&mut self, lim: &SolveLimits) -> LpStatus {
        self.solve_once(lim)
    }

    pub fn solve_primal(&mut self, lim: &SolveLimits) -> LpStatus {
        self.solve(lim)
    }

    /// 基底のない LP の同時実行 (`threads` >= 2): 二段解法を 1 スレッドの専用プールで ([`Self::solve_once`])、内点法 +
    /// クロスオーバーを `threads - 1` スレッドの専用プールで同時に解き、先に結論を出した側の基底解を採る (LP の
    /// [`super::race`] と同じ配分。始める前にプールを分けるので、二段解法の並列区間 (LU・行列ベクトル積) と内点法の
    /// 分解がスレッドを取り合わない)。内点法側が採るのは最適な頂点が得られたときだけ。そのときは二段解法を打ち切り、クロスオーバーの仕上げ
    /// (二段解法の主ループ) が記録した最適基底と双対をそのまま使う ([`Self::adopt_basic_solution`])。仕上げの記録が
    /// 使えない (双対化した問題を解いた経路など) ときだけ、頂点で境界の内側にある列を基底にして warm start で解き直す。
    /// 二段解法が結論を出せずに終わった (`Error`・`IterationLimit`) ときは勝ちとせず、期限まで内点法側の結果を待つ。
    fn solve_race(&mut self, lim: &SolveLimits, threads: usize) -> LpStatus {
        self.solve_race_ext(lim, threads, false)
    }

    /// [`Self::solve_race`] の本体。`force_ipm` なら二段解法が先に最適で終わっても内点法 + クロスオーバーの頂点を待つ
    /// (`ENOMOTO_MIP_ROOT_LP_IPM` と同じ。試作 `ENOMOTO_MIP_MC11FIX1_RACE=i`)
    fn solve_race_ext(&mut self, lim: &SolveLimits, threads: usize, force_ipm: bool) -> LpStatus {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::{mpsc, Arc};
        if threads < 2 {
            return self.solve_once(lim);
        }
        let build = |k: usize, name: &'static str| rayon::ThreadPoolBuilder::new().num_threads(k).thread_name(move |i| format!("enomoto-mip-{name}-{i}")).build();
        let (Ok(pool_s), Ok(pool)) = (build(1, "simplex"), build(threads - 1, "ipm")) else {
            return self.solve_once(lim);
        };
        let debug = env_str!("ENOMOTO_DEBUG_RACE").is_some();
        let t0 = std::time::Instant::now();
        let cancel = Arc::new(AtomicBool::new(false));
        let bound = crate::cancel::SharedBound::new(self.std.n_total, self.std.n_rows);
        type Won = (Vec<f64>, Option<(Vec<f64>, Vec<Option<usize>>)>);
        let (tx, rx) = mpsc::channel::<Option<Won>>();
        // 診断 `ENOMOTO_MIP_XOLAZY_LOG` (振る舞いは変えない): 内点法側の時刻を記録させる
        let stamp: Option<super::crossover::XoStamp> = (xolazy_log() || ipmrc_has('u')).then(Default::default);
        {
            let (std, cancel, bound) = (Arc::new(self.std.clone()), cancel.clone(), bound.clone());
            let sc = self.ipm_scale();
            let deadline = lim.deadline;
            let stamp = stamp.clone();
            std::thread::spawn(move || {
                let r = pool.install(|| {
                    super::crossover::XO_STAMP.with(|s| *s.borrow_mut() = stamp);
                    // 仕上げの二段解法の最適基底と双対を記録させる
                    sid::request_duals(true);
                    let r = crate::cancel::with_deadline(deadline, || {
                        crate::cancel::with_token(Some(cancel.clone()), || crate::cancel::with_bound(Some(bound), || ipm_crossover_run(&std, sc.as_deref())))
                    });
                    let duals = sid::take_duals();
                    sid::request_duals(false);
                    super::crossover::XO_STAMP.with(|s| *s.borrow_mut() = None);
                    r.filter(|r| r.status == Status::Optimal).and_then(|r| r.x).map(|x| (x, duals))
                });
                // 最適な頂点が得られ、二段解法より先なら合図を立てて二段解法を止める
                let won = r.is_some() && !cancel.swap(true, Ordering::SeqCst);
                let _ = tx.send(if won { r } else { None });
            });
        }
        // 二段解法は 1 スレッドのプールで解く。スレッドごとの設定 (MIP のポートフォリオの設定など) を持ち込み、持ち帰る
        let flags = sid::tl_flags();
        let shared_lb = bound.clone();
        let (st, flags, y_mid) = pool_s.install(|| {
            sid::set_tl_flags(flags);
            let _ = stamp.is_some().then(sid::take_bound_duals);
            let st = crate::cancel::with_token(Some(cancel.clone()), || crate::cancel::with_bound(Some(bound), || self.solve_once(lim)));
            // 診断 `ENOMOTO_MIP_XOLAZY_LOG`: 二段解法が最後に下界を計算した途中の基底の双対
            let y_mid = stamp.is_some().then(sid::take_bound_duals);
            (st, sid::tl_flags(), y_mid)
        });
        sid::set_tl_flags(flags);
        let conclusive = matches!(st, LpStatus::Optimal | LpStatus::Infeasible | LpStatus::Unbounded | LpStatus::ObjectiveBound | LpStatus::TimeLimit);
        // `ENOMOTO_MIP_ROOT_LP_IPM` (診断用、既定では使わない): 二段解法が先に最適で終わっても内点法 + クロスオーバーの頂点を待って採る。
        // beasleyC3 では、どちらが先かで根の頂点が変わり、背景のカットの後の下界が 741.96 (二段解法) と 746.3 (内点法) に分かれ、
        // 解けるかどうかがほぼこれで決まった (analysis/beasleyC3_r3_20261020.md)
        let wait_ipm = st == LpStatus::Optimal && (force_ipm || env_str!("ENOMOTO_MIP_ROOT_LP_IPM").is_some());
        // 診断 `ENOMOTO_MIP_IPMRC_WAIT` (analysis/ipmrc_20261111.md、採る結果は変えない): 二段解法が先に最適で終わっても、内点法の点
        // (押し出しの前の双対) が届くか内点法側が終わるまで待ってから止める (その間に内点法側が頂点で勝っても、二段解法の結果を採る)。
        // 待った時間は [`RaceDiag::t_wait`]
        let mut t_wait = None;
        if conclusive && !wait_ipm && st == LpStatus::Optimal && stamp.is_some() && (env_str!("ENOMOTO_MIP_IPMRC_WAIT").is_some() || ipmrc_has('w')) {
            let tw = std::time::Instant::now();
            let cap = tunable!("ENOMOTO_T_MIP_IPMRC_WAIT_MAX", 60.0, f64);
            let sp = stamp.as_ref().unwrap();
            loop {
                // 内点法の点が届いた・内点法側が終わった (頂点で勝った、または失敗した)・待つ上限・期限
                if sp.lock().map(|a| a.y_ipm.is_some()).unwrap_or(true) || cancel.load(Ordering::SeqCst) || rx.try_recv().is_ok() || tw.elapsed().as_secs_f64() >= cap || lim.deadline.is_some_and(|d| std::time::Instant::now() >= d) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            t_wait = Some(tw.elapsed().as_secs_f64());
        }
        let won = if conclusive && !wait_ipm {
            if t_wait.is_some() || !cancel.swap(true, Ordering::SeqCst) {
                if t_wait.is_some() {
                    // 待つ間に内点法側が頂点で勝っていたら (合図を立てた後)、その結果は受け取って捨てる
                    cancel.store(true, Ordering::SeqCst);
                }
                // 二段解法が先 (内点法側は確認点で止まる。待たない)
                if debug {
                    eprintln!("MIP root race: simplex won at {:.3}s ({st:?})", t0.elapsed().as_secs_f64());
                }
                crate::phase_timing::mark("race_simplex_won");
                user_log!("Root LP: the two-phase method finished first ({:.2}s)", t0.elapsed().as_secs_f64());
                xolazy_record("simplex", t0, stamp.as_ref(), None);
                if let Some(sp) = stamp.as_ref() {
                    RACE_DIAG.with(|r| {
                        if let Some(d) = r.borrow_mut().as_mut() {
                            d.t_wait = t_wait;
                        }
                    });
                    if st == LpStatus::Optimal {
                        self.xolazy_duals(sp, y_mid.as_deref(), &shared_lb);
                    }
                }
                return st;
            }
            rx.recv().ok().flatten()
        } else {
            // 二段解法は結論を出せなかった (数値的な破綻など): 内点法側を止めずに、期限まで結果を待つ。leo1・leo2 では
            // 二段解法が 0.1 秒で `Error` を返して勝ちとされ、内点法側を止めて MIP が `NotSolved` で終わっていた
            if debug {
                eprintln!("MIP root race: simplex ended without a conclusion at {:.3}s ({st:?}), waiting for ipm_crossover", t0.elapsed().as_secs_f64());
            }
            let r = match lim.deadline {
                Some(d) => rx.recv_timeout(d.saturating_duration_since(std::time::Instant::now())).ok(),
                None => rx.recv().ok(),
            };
            cancel.store(true, Ordering::SeqCst);
            match r.flatten() {
                Some(w) => Some(w),
                None => return if lim.deadline.is_some_and(|d| std::time::Instant::now() >= d) { LpStatus::TimeLimit } else { st },
            }
        };
        let Some((x, duals)) = won else {
            xolazy_record("none", t0, stamp.as_ref(), None);
            return self.solve_once(lim);
        };
        crate::phase_timing::mark("race_ipm_crossover_won");
        user_log!("Root LP: the interior-point method + crossover finished first ({:.2}s)", t0.elapsed().as_secs_f64());
        let t_won = t0.elapsed().as_secs_f64();
        let st = self.adopt_ipm_vertex(&x, duals, lim, debug, t0);
        xolazy_record(if st == LpStatus::Optimal { "ipm" } else { "ipm_resolve" }, t0, stamp.as_ref(), Some(t_won));
        if let (Some(stamp), LpStatus::Optimal) = (stamp.as_ref(), st) {
            self.xolazy_duals(stamp, y_mid.as_deref(), &shared_lb);
        }
        st
    }

    /// 診断 `ENOMOTO_MIP_XOLAZY_LOG`: 最適な双対・途中の基底の双対・内点法の双対のラグランジュ下界と被約費用を [`RACE_DIAG`] に足す
    /// (時刻を測った後に呼ぶ。最適で終わったときだけ)
    fn xolazy_duals(&self, stamp: &super::crossover::XoStamp, y_mid: Option<&[f64]>, shared_lb: &crate::cancel::SharedBound) {
        let n = self.n;
        let Some((l_opt0, d_opt)) = lagrange_std(&self.std, n, &self.y) else { return };
        // 下界の差の基準は主の目的値 (標準形の単位、`x' = x - shift`)。最適な双対の `L` を基準にすると、その双対が少しでも
        // 最適でないとき他の双対の下界を高く見積もる (khb05250 で内点法の下界が LP の値を 0.05 越えて見えた)
        let l_opt = (0..n).map(|j| self.std.c[j] * (self.x[j] - self.shift[j])).sum::<f64>();
        let y_ipm = stamp.lock().ok().and_then(|mut a| a.y_ipm.take());
        let ls = y_mid.and_then(|y| lagrange_std(&self.std, n, y));
        let li = y_ipm.as_deref().and_then(|y| lagrange_std(&self.std, n, y));
        let sh = shared_lb.get();
        RACE_DIAG.with(|r| {
            if let Some(d) = r.borrow_mut().as_mut() {
                d.gap_simplex = ls.as_ref().map(|v| l_opt - v.0);
                d.gap_ipm = li.as_ref().map(|v| l_opt - v.0);
                d.gap_shared = sh.is_finite().then(|| l_opt - sh);
                d.gap_opt = Some(l_opt - l_opt0);
                // 標準形の目的は平行移動した列 `x' = x - shift` のもの: 元の単位は `c·shift` を足す
                d.lb_ipm = li.as_ref().map(|v| v.0 + (0..n).map(|j| self.std.c[j] * self.shift[j]).sum::<f64>());
                d.d_opt = Some(d_opt);
                d.d_simplex = ls.map(|v| v.1);
                d.d_ipm = li.map(|v| v.1);
            }
        });
    }

    /// 診断用 (`ENOMOTO_MIP_ROOTLP_BOTH_DIAG`、既定では呼ばない。analysis/rootlp_both_diff_20261105.md): この LP の写しを 2 つ作り、
    /// [`Self::solve_race`] と同じ配分 (二段解法は 1 スレッド、内点法 + クロスオーバーは `threads - 1` スレッド、下界は共有) で同時に、
    /// どちらも止めずに最後まで解く。返す写しは、二段解法の写しと、内点法の頂点を採った写し (競争で内点法が勝ったときと同じ
    /// [`Self::adopt_ipm_vertex`])。自分は変えない。
    pub fn diag_both(&self, lim: &SolveLimits, threads: usize) -> BothDiag {
        use std::sync::mpsc;
        let threads = threads.max(2);
        let build = |k: usize, name: &'static str| rayon::ThreadPoolBuilder::new().num_threads(k).thread_name(move |i| format!("enomoto-mip-diag-{name}-{i}")).build().unwrap();
        let (pool_s, pool) = (build(1, "simplex"), build(threads - 1, "ipm"));
        let bound = crate::cancel::SharedBound::new(self.std.n_total, self.std.n_rows);
        let t0 = std::time::Instant::now();
        type Got = (Option<(Vec<f64>, Option<(Vec<f64>, Vec<Option<usize>>)>)>, f64, Option<f64>);
        let (tx, rx) = mpsc::channel::<Got>();
        {
            let (std, bound) = (Arc::new(self.std.clone()), bound.clone());
            let sc = self.ipm_scale();
            let deadline = lim.deadline;
            std::thread::spawn(move || {
                let r = pool.install(|| {
                    super::crossover::XO_IPM_END.with(|t| t.set(None));
                    sid::request_duals(true);
                    let r = crate::cancel::with_deadline(deadline, || crate::cancel::with_bound(Some(bound), || ipm_crossover_run(&std, sc.as_deref())));
                    let duals = sid::take_duals();
                    sid::request_duals(false);
                    let t_ipm = super::crossover::XO_IPM_END.with(|t| t.get()).map(|e| e.saturating_duration_since(t0).as_secs_f64());
                    (r.filter(|r| r.status == Status::Optimal).and_then(|r| r.x).map(|x| (x, duals)), t0.elapsed().as_secs_f64(), t_ipm)
                });
                let _ = tx.send(r);
            });
        }
        let mut lp_s = self.clone();
        // 診断 `ENOMOTO_MIP_ROOTLP_IPM_ONLY=1` (analysis/rootlp_ipm_fail_20261105.md): 二段解法を走らせず、内点法 + クロスオーバーだけを解く
        // (二段解法の下界の共有もない。`st_s` は `IterationLimit`)
        let ipm_only = env_str!("ENOMOTO_MIP_ROOTLP_IPM_ONLY").is_some();
        let flags = sid::tl_flags();
        let (st_s, flags) = pool_s.install(|| {
            sid::set_tl_flags(flags);
            let st = if ipm_only { LpStatus::IterationLimit } else { crate::cancel::with_bound(Some(bound), || lp_s.solve_once(lim)) };
            (st, sid::tl_flags())
        });
        sid::set_tl_flags(flags);
        let t_simplex = t0.elapsed().as_secs_f64();
        let (got, t_xo, t_ipm) = rx.recv().unwrap_or((None, t0.elapsed().as_secs_f64(), None));
        let mut lp_i = self.clone();
        let t1 = std::time::Instant::now();
        let st_i = match got {
            Some((x, duals)) => Some(lp_i.adopt_ipm_vertex(&x, duals, lim, false, t1)),
            None => None,
        };
        let t_adopt = t1.elapsed().as_secs_f64();
        if let Some(st) = st_i {
            lp_i.status = Some(st);
        }
        BothDiag { lp_s, st_s, t_simplex, lp_i, st_i, t_ipm, t_xo, t_adopt }
    }

    /// 試作 `ENOMOTO_MIP_ROOTLP_BOTH` の同時実行 ([`request_both`])。[`Self::solve_race_ext`] と同じく二段解法 (1 スレッド) と
    /// 内点法 + クロスオーバー (`threads - 1` スレッド) で解くが、二段解法もこの LP の写しで別のスレッドに解かせ、先に最適で
    /// 終わった側を採った後、負けた側を止めずに続ける ([`BothLoser`]、[`take_both_loser`] で取り出す)。負けた側は勝った側の時間の
    /// `ratio` 倍 (`min_secs` 以上 `max_secs` 以下) の後に見張りのスレッドが止める。
    fn solve_race_both(&mut self, lim: &SolveLimits, threads: usize, cfg: (f64, f64, f64)) -> LpStatus {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::mpsc;
        let build = |k: usize, name: &'static str| rayon::ThreadPoolBuilder::new().num_threads(k).thread_name(move |i| format!("enomoto-mip-{name}-{i}")).build();
        let (Ok(pool_s), Ok(pool)) = (build(1, "simplex"), build(threads - 1, "ipm")) else {
            return self.solve_once(lim);
        };
        let debug = env_str!("ENOMOTO_DEBUG_RACE").is_some();
        let t0 = std::time::Instant::now();
        let cancel_i = Arc::new(AtomicBool::new(false));
        let cancel_s = Arc::new(AtomicBool::new(false));
        let bound = crate::cancel::SharedBound::new(self.std.n_total, self.std.n_rows);
        let (tx, rx) = mpsc::channel::<BothMsg>();
        let base = Box::new(self.clone());
        {
            let (std, cancel, bound, tx) = (Arc::new(self.std.clone()), cancel_i.clone(), bound.clone(), tx.clone());
            let sc = self.ipm_scale();
            let deadline = lim.deadline;
            std::thread::spawn(move || {
                let r = pool.install(|| {
                    sid::request_duals(true);
                    let r = crate::cancel::with_deadline(deadline, || {
                        crate::cancel::with_token(Some(cancel.clone()), || crate::cancel::with_bound(Some(bound), || ipm_crossover_run(&std, sc.as_deref())))
                    });
                    let duals = sid::take_duals();
                    sid::request_duals(false);
                    r.filter(|r| r.status == Status::Optimal).and_then(|r| r.x).map(|x| (x, duals))
                });
                let _ = tx.send(BothMsg::Ipm(if cancel.load(Ordering::SeqCst) { None } else { r }));
            });
        }
        {
            let (mut lp, cancel, bound, tx, lim) = (Box::new(self.clone()), cancel_s.clone(), bound, tx, *lim);
            let flags = sid::tl_flags();
            std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn(move || {
                    let (st, flags) = pool_s.install(|| {
                        sid::set_tl_flags(flags);
                        let st = crate::cancel::with_token(Some(cancel.clone()), || crate::cancel::with_bound(Some(bound), || lp.solve_once(&lim)));
                        (st, sid::tl_flags())
                    });
                    let _ = tx.send(BothMsg::Simplex(st, lp, flags));
                })
                .expect("spawn");
        }
        let recv = |rx: &mpsc::Receiver<BothMsg>| match lim.deadline {
            Some(d) => rx.recv_timeout(d.saturating_duration_since(std::time::Instant::now())).ok(),
            None => rx.recv().ok(),
        };
        let mut ipm_failed = false;
        let mut simplex_failed: Option<LpStatus> = None;
        loop {
            let Some(msg) = recv(&rx) else {
                cancel_i.store(true, Ordering::SeqCst);
                cancel_s.store(true, Ordering::SeqCst);
                return LpStatus::TimeLimit;
            };
            let win_secs = t0.elapsed().as_secs_f64();
            let (ratio, max_secs, min_secs) = cfg;
            let wait = (ratio * win_secs).max(min_secs).min(max_secs);
            let give_up = std::time::Instant::now() + std::time::Duration::from_secs_f64(wait);
            match msg {
                BothMsg::Simplex(st, lp, flags) => {
                    let conclusive = matches!(st, LpStatus::Optimal | LpStatus::Infeasible | LpStatus::Unbounded | LpStatus::ObjectiveBound | LpStatus::TimeLimit);
                    if !conclusive {
                        if ipm_failed {
                            return st;
                        }
                        simplex_failed = Some(st);
                        if debug {
                            eprintln!("MIP root race (both): simplex ended without a conclusion at {win_secs:.3}s ({st:?}), waiting for ipm_crossover");
                        }
                        continue;
                    }
                    *self = *lp;
                    sid::set_tl_flags(flags);
                    if debug {
                        eprintln!("MIP root race (both): simplex won at {win_secs:.3}s ({st:?}); ipm_crossover continues for up to {wait:.3}s");
                    }
                    crate::phase_timing::mark("race_simplex_won");
                    user_log!("Root LP: the two-phase method finished first ({:.2}s)", win_secs);
                    if st == LpStatus::Optimal && !ipm_failed {
                        let c = cancel_i.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_secs_f64(wait));
                            c.store(true, Ordering::SeqCst);
                        });
                        BOTH_LOSER.with(|b| *b.borrow_mut() = Some(BothLoser { kind: "ipm", rx, cancel: cancel_i, base: Some(base), lim: *lim, win_secs, give_up, t0 }));
                    } else {
                        cancel_i.store(true, Ordering::SeqCst);
                    }
                    return st;
                }
                BothMsg::Ipm(None) => {
                    ipm_failed = true;
                    if let Some(st) = simplex_failed {
                        return st;
                    }
                    continue;
                }
                BothMsg::Ipm(Some((x, duals))) => {
                    crate::phase_timing::mark("race_ipm_crossover_won");
                    user_log!("Root LP: the interior-point method + crossover finished first ({:.2}s)", win_secs);
                    let st = self.adopt_ipm_vertex(&x, duals, lim, debug, t0);
                    if debug {
                        eprintln!("MIP root race (both): ipm_crossover won at {win_secs:.3}s ({st:?}); simplex continues for up to {wait:.3}s");
                    }
                    if st == LpStatus::Optimal && simplex_failed.is_none() {
                        let c = cancel_s.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_secs_f64(wait));
                            c.store(true, Ordering::SeqCst);
                        });
                        BOTH_LOSER.with(|b| *b.borrow_mut() = Some(BothLoser { kind: "simplex", rx, cancel: cancel_s, base: None, lim: *lim, win_secs, give_up, t0 }));
                    } else {
                        cancel_s.store(true, Ordering::SeqCst);
                    }
                    self.status = Some(st);
                    return st;
                }
            }
        }
    }

    /// 内点法 + クロスオーバーで単独に解き (このスレッドで、期限は `lim.deadline`)、得られた最適な頂点をこの LP の解として採る
    /// ([`Self::solve_race`] の内点法側と同じ)。二段解法が結論を出せなかった LP (MIP の根) の解き直しに使う。
    pub fn solve_ipm_rescue(&mut self, lim: &SolveLimits) -> LpStatus {
        let t0 = std::time::Instant::now();
        let std = self.std.clone();
        let sc = self.ipm_scale();
        sid::request_duals(true);
        let r = crate::cancel::with_deadline(lim.deadline, || ipm_crossover_run(&std, sc.as_deref()));
        let duals = sid::take_duals();
        sid::request_duals(false);
        let Some(x) = r.filter(|r| r.status == Status::Optimal).and_then(|r| r.x) else {
            return if lim.deadline.is_some_and(|d| std::time::Instant::now() >= d) { LpStatus::TimeLimit } else { LpStatus::Error };
        };
        let st = self.adopt_ipm_vertex(&x, duals, lim, env_str!("ENOMOTO_DEBUG_RACE").is_some(), t0);
        self.status = Some(st);
        st
    }

    /// 内点法 + クロスオーバーの最適な頂点 `x` (標準形の空間) と仕上げの記録 `duals` を、この LP の解として採る。
    fn adopt_ipm_vertex(&mut self, x: &[f64], duals: Option<(Vec<f64>, Vec<Option<usize>>)>, lim: &SolveLimits, debug: bool, t0: std::time::Instant) -> LpStatus {
        let n = self.n;
        let xo: Vec<f64> = (0..n).map(|j| x.get(j).copied().unwrap_or(0.0) + self.shift[j]).collect();
        if let Some((y, bpos)) = duals {
            let bpos_keep = env_str!("ENOMOTO_XO_MIP_SCALE").is_some_and(|v| v != "0").then(|| bpos.clone());
            if self.adopt_basic_solution(&xo, y, &bpos) {
                if debug {
                    eprintln!("MIP root race: ipm_crossover won at {:.3}s, its optimal basis adopted", t0.elapsed().as_secs_f64());
                }
                return LpStatus::Optimal;
            }
            // 試作 `ENOMOTO_XO_MIP_SCALE`: 尺度をそろえた写しの最適基底は元の尺度では丸めで判定に通らないことがある。その基底から
            // warm start で解き直す (最適基底なのでほとんど反復しない)
            let m = self.rows.len();
            let b: Option<Vec<usize>> = bpos_keep.filter(|bp| bp.len() == n + m).and_then(|bp| {
                let mut b = vec![usize::MAX; m];
                for (v, p) in bp.iter().enumerate() {
                    if let Some(p) = *p {
                        if p >= m {
                            return None;
                        }
                        b[p] = v;
                    }
                }
                (!b.contains(&usize::MAX)).then_some(b)
            });
            if let Some(b) = b {
                self.basis = Some(b);
                self.x.copy_from_slice(&xo);
                self.lu = None;
                self.lu_cache = None;
                self.dse_cache = None;
                self.status = None;
                self.clear_memo();
                let it0 = self.iters;
                let st = self.solve_once(lim);
                if debug || env_str!("ENOMOTO_DEBUG_ADOPT").is_some() {
                    eprintln!("MIP root race: ipm_crossover (scaled) basis -> warm re-solve {st:?} in {} iters", self.iters - it0);
                }
                return st;
            }
        }
        // 仕上げの記録が使えない: 頂点で境界の内側にある列 (足りなければスラック) を基底にして warm start で解き直す
        let m = self.rows.len();
        let inside = |v: f64, l: f64, u: f64| v > l + 1e-9 * (1.0 + l.abs()) && v < u - 1e-9 * (1.0 + u.abs());
        let mut b: Vec<usize> = (0..n).filter(|&j| inside(xo[j], self.lo[j], self.up[j])).collect();
        for i in 0..m {
            if inside(self.slack_value(i, &xo), 0.0, self.std.ub[n + i]) {
                b.push(n + i);
            }
        }
        self.basis = Some(b);
        self.repair_basis();
        self.x.copy_from_slice(&xo);
        self.lu = None;
        self.lu_cache = None;
        self.dse_cache = None;
        self.status = None;
        self.clear_memo();
        let it0 = self.iters;
        let st = self.solve_once(lim);
        if debug {
            eprintln!("MIP root race: ipm_crossover won at {:.3}s, no usable basis -> warm re-solve {st:?} in {} iters", t0.elapsed().as_secs_f64(), self.iters - it0);
        }
        st
    }

    /// 直前の求解が非有界 (段階 A が `z^1 < 0`、段階 B で実行可能) だったときの切片問題 (論文の式 (8)) の境界を、元の
    /// 空間の (列の下限, 列の上限, 行の下限, 行の上限) で返す。行列は同じ。切片問題の実行可能解 `x^0` と段階 A の傾き
    /// `x^1` (`A x^1 = 0`、目的値を下げる) から `x^0 + M x^1` (M → ∞) が元の問題の実行可能解になるので、切片問題を整数に
    /// 制限した問題が実行可能なら元の整数最適化は非有界、実行不能なら実行不能。
    pub fn intercept_bounds(&self) -> Option<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> {
        let (lo, up) = self.intercept.as_ref()?;
        let n = self.n;
        let m = self.rows.len();
        if lo.len() != n + m || up.len() != n + m {
            return None;
        }
        let inf = f64::INFINITY;
        // 構造列: 標準形の値 x' = x - shift
        let col_lo: Vec<f64> = (0..n).map(|j| lo[j].map_or(-inf, |b| b + self.shift[j])).collect();
        let col_up: Vec<f64> = (0..n).map(|j| up[j].map_or(inf, |b| b + self.shift[j])).collect();
        // スラック s_i (σ_i a_i x + s_i = b_i): σ = +1 なら a_i x = 上限 - s、σ = -1 なら a_i x = 下限 + s
        let mut row_lo = vec![-inf; m];
        let mut row_up = vec![inf; m];
        for i in 0..m {
            let (sl, su) = (lo[n + i].unwrap_or(-inf), up[n + i].unwrap_or(inf));
            if self.sigma[i] > 0.0 {
                row_lo[i] = self.row_up[i] - su;
                row_up[i] = self.row_up[i] - sl;
            } else {
                row_lo[i] = self.row_lo[i] + sl;
                row_up[i] = self.row_lo[i] + su;
            }
        }
        Some((col_lo, col_up, row_lo, row_up))
    }

    /// 行 `i` のスラック (標準形の値、`σ_i a_i x + s_i = b_i`) を元の空間の点 `x` から求める。
    fn slack_value(&self, i: usize, x: &[f64]) -> f64 {
        let act: f64 = self.rows[i].iter().map(|&(j, a)| a * x[j]).sum();
        if self.sigma[i] > 0.0 {
            self.row_up[i] - act
        } else {
            act - self.row_lo[i]
        }
    }

    /// 別のスレッドで求めた基底解 (元の空間の点 `x`、標準形の双対 `y`、列ごとの基底位置 `bpos`) を、この LP の最適解と
    /// して採る。大きさが合い、非基底の列が境界にあり、被約費用 `c - Aᵀy` の符号が非基底の列の位置と合う (双対実行可能)
    /// ときだけ採って真を返す。LU と DSE 重みは持たない (次の求解で分解し直す)。
    fn adopt_basic_solution(&mut self, x: &[f64], y: Vec<f64>, bpos: &[Option<usize>]) -> bool {
        let n = self.n;
        let m = self.rows.len();
        // 診断 `ENOMOTO_DEBUG_ADOPT=1` (analysis/rootlp_ipm_fail_20261105.md): 採らなかった理由を stderr に出す
        let dbg = env_str!("ENOMOTO_DEBUG_ADOPT").is_some();
        if y.len() != m || bpos.len() != n + m {
            if dbg {
                eprintln!("ADOPT: size mismatch y={} (m={m}) bpos={} (n+m={})", y.len(), bpos.len(), n + m);
            }
            return false;
        }
        let mut b = vec![usize::MAX; m];
        for (v, p) in bpos.iter().enumerate() {
            if let Some(p) = *p {
                if p >= m || b[p] != usize::MAX {
                    return false;
                }
                b[p] = v;
            }
        }
        if b.iter().any(|&v| v == usize::MAX) {
            if dbg {
                eprintln!("ADOPT: basis positions incomplete");
            }
            return false;
        }
        let tol = 1e-7;
        let row_scale_tol = env_str!("ENOMOTO_MIP_ADOPT_ROW_SCALE").is_some_and(|v| v != "0");
        for v in 0..n + m {
            if bpos[v].is_some() {
                continue;
            }
            // 標準形の値 (平行移動した空間) と境界
            let z = if v < n { x[v] - self.shift[v] } else { self.slack_value(v - n, x) };
            let (l, u) = (self.std.lb[v], self.std.ub[v]);
            // 試作 `ENOMOTO_MIP_ADOPT_ROW_SCALE=1` (既定では使わない。analysis/rootlp_ipm_fail_20261105.md): スラックの値は `x` から行の
            // 活動量を計算し直すので、丸めの誤差は行の項の大きさ `Σ|a_ij x_j|` と右辺に比例する。許容をそれに比例させる
            // (opm2-z10-s4 の根の LP: 右辺・活動量が 1e6 級の行で、最適基底のスラック -5.6e-9 が許容 2e-9 を超えて捨てられ、解き直しに 85 秒)。
            let mag = if v >= n && row_scale_tol {
                let i = v - n;
                self.rows[i].iter().map(|&(j, a)| (a * x[j]).abs()).sum::<f64>().max(self.row_lo[i].abs().min(1e300)).max(if self.row_up[i].is_finite() { self.row_up[i].abs() } else { 0.0 })
            } else {
                0.0
            };
            let at_l = l.is_finite() && (z - l).abs() <= 1e-9 * (1.0 + l.abs().max(mag)) + 1e-9;
            let at_u = u.is_finite() && (z - u).abs() <= 1e-9 * (1.0 + u.abs().max(mag)) + 1e-9;
            let free = !l.is_finite() && !u.is_finite() && z.abs() <= 1e-9;
            if !(at_l || at_u || free) {
                if dbg {
                    eprintln!("ADOPT: nonbasic {v} (n={n}) off its bounds: z={z:e} l={l:e} u={u:e}");
                }
                return false;
            }
            if at_l && at_u {
                continue; // 固定列は被約費用の符号を問わない
            }
            let d = self.std.c[v] - self.std.cols.col(v).iter().map(|&(i, a)| a * y[i]).sum::<f64>();
            let scale = tol * (1.0 + self.std.c[v].abs());
            let ok = if at_l { d >= -scale } else if at_u { d <= scale } else { d.abs() <= scale };
            if !ok {
                if dbg {
                    eprintln!("ADOPT: nonbasic {v} (n={n}) dual infeasible: d={d:e} at_l={at_l} at_u={at_u} tol={scale:e}");
                }
                return false;
            }
        }
        self.x.copy_from_slice(&x[..n]);
        self.y = y;
        self.basis = Some(b);
        self.lu = None;
        self.lu_cache = None;
        self.dse_cache = None;
        self.ray = None;
        self.status = Some(LpStatus::Optimal);
        self.clear_memo();
        true
    }

    fn solve_inner(&mut self, lim: &SolveLimits) -> LpStatus {
        // 試作 `ENOMOTO_MIP_NODECUT_LIVE`・診断 `ENOMOTO_MIP_NODECUT_STAT` ([`set_nodecut_src`])。既定では入れない
        let src = if NC_SRC_ON.with(|c| c.get()) { NC_SRC.with(|c| c.borrow().as_ref().filter(|s| s.cutoff_bits == lim.cutoff.to_bits()).map(|s| (s.f.clone(), s.kind))) } else { None };
        let Some((f, kind)) = src else { return self.solve_inner0(lim) };
        let n = self.n;
        let const_obj: f64 = (0..n).map(|j| self.cost[j] * self.shift[j]).sum();
        let apply = nodecut_live();
        let every = tunable!("ENOMOTO_T_MIP_NODECUT_LIVE_EVERY", 32usize, usize).max(1);
        sid::set_ext_live(Some(sid::ExtLive { f: f.clone(), shift: const_obj, every, apply }));
        let it0 = sid::ext_iterations();
        let st = self.solve_inner0(lim);
        let r = sid::ext_nc_rec();
        sid::set_ext_live(None);
        if nodecut_stat() {
            let it = sid::ext_iterations() - it0;
            let k = 32 * (kind as usize).min(3);
            let end = f().min(lim.cutoff);
            let add = |i: usize, v: u64| NODECUT_STATS[k + i].fetch_add(v, std::sync::atomic::Ordering::Relaxed);
            add(0, 1);
            add(1, it);
            if !lim.cutoff.is_finite() {
                add(20, 1);
            }
            if r.lower_at.is_some() {
                add(11, 1);
            }
            add(24, r.checks as u64);
            add(25, r.inf_checks as u64);
            // 標準形の空間の値を元に戻す (`ExtLive::shift` = const_obj)
            let shadow_hit = r.shadow_hit;
            match st {
                LpStatus::ObjectiveBound => {
                    add(2, 1);
                    add(3, it);
                    if r.lower_at.is_some() {
                        add(12, 1);
                        add(13, it);
                    }
                    if let Some(h) = shadow_hit {
                        add(14, 1);
                        add(15, it.saturating_sub(h as u64));
                    }
                }
                LpStatus::Optimal => {
                    let o = self.objective();
                    add(18, 1);
                    add(19, it);
                    if o >= lim.cutoff {
                        add(4, 1);
                        add(5, it);
                        match r.first_b {
                            None => {
                                add(6, 1);
                                add(7, it);
                            }
                            Some(b) => {
                                if env_str!("ENOMOTO_MIP_NODECUT_TRACE").is_some() {
                                    eprintln!("NODECUT_TRACE: kind {kind} iters {it} first_b {b} checks {} last_check {:?} cutoff {:.10e} obj {:.10e} (std cutoff {:.10e})", r.checks, r.last_check, lim.cutoff, o, lim.cutoff - const_obj);
                                }
                                add(16, it.saturating_sub(b as u64));
                                if r.last_check.is_some_and(|(_, v)| v == f64::NEG_INFINITY) {
                                    add(26, 1);
                                    add(27, it);
                                }
                                if let Some(h) = r.relaxed_hit {
                                    add(28, 1);
                                    add(29, it.saturating_sub(h as u64));
                                }
                                if r.checks == 0 {
                                    add(17, 1);
                                }
                            }
                        }
                    } else if o >= end {
                        add(8, 1);
                        add(9, it);
                        if let Some(h) = shadow_hit {
                            add(10, it.saturating_sub(h as u64));
                        }
                        if r.first_b.is_none() {
                            add(21, 1);
                        }
                    }
                }
                LpStatus::Infeasible => {
                    add(22, 1);
                    add(23, it);
                }
                _ => {}
            }
        }
        st
    }

    fn solve_inner0(&mut self, lim: &SolveLimits) -> LpStatus {
        let n = self.n;
        // 目的値の定数 (平行移動の分): c·x = c·x' + c·shift
        let const_obj: f64 = (0..n).map(|j| self.cost[j] * self.shift[j]).sum();
        let ctrl = ExtControl {
            iteration_limit: usize::try_from(lim.iteration_limit).unwrap_or(usize::MAX),
            cutoff: lim.cutoff - const_obj,
            deadline: lim.deadline,
        };
        let opts = LpOptions { distinguish_infeasible_unbounded: true, ..Default::default() };
        let it0 = sid::ext_iterations();
        sid::xprof("between");
        sid::set_ext_control(Some(ctrl));
        // `ENOMOTO_MIP_LP_RESUME` (試作、既定では使わない): 時間・反復の上限で止まった求解の基底・DSE 重み・非基底列の
        // 置き場所を残し、次の求解をそこから始める (続きを解く)。従来は打ち切られた求解の進みを捨て、次の求解は前の最適
        // 基底から単位重みでやり直していた (根のカットのループの「LP を続ける」がやり直しになっていた、
        // analysis/lp_hang_20261021.md)
        let resume_on = env_str!("ENOMOTO_MIP_LP_RESUME").is_some() || FORCE_RESUME_ONCE.with(|c| c.replace(false));
        sid::request_stop_state(resume_on);
        sid::set_ext_progress(PROGRESS_STOP_ONCE.with(|c| c.take()).map(|(from, window, frac, start)| sid::ExtProgress { from, window, frac, start: start - const_obj }));
        sid::set_fast_reopt(env_str!("ENOMOTO_MIP_NO_FAST_REOPT").is_none());
        sid::request_duals(true);
        sid::request_lu(true);
        sid::request_intercept(true);
        sid::set_price_key(Some(self.price_key));
        let r = match self.basis.clone() {
            Some(b) if b.len() == self.rows.len() => {
                // 同じ基底の LU が手元にあれば渡す (分解を省く)。LU は求解で更新されるので所有権を渡す
                // (状態の保存や `self.lu` と共有中なら複製する)
                self.lu = None;
                match self.lu_cache.take() {
                    // Forrest-Tomlin の更新が積み重なった LU は FTRAN/BTRAN が遅いので、分解し直させる
                    // (主ループの上限 3m に任せると、再分解は減るがノードの処理数はかえって減った)
                    Some(c) if c.basis == b && c.lu.update_count() < tunable!("ENOMOTO_T_MIP_LU_MAX_UPD", 64usize, usize) => {
                        let warm_d = env_str!("ENOMOTO_MIP_WARM_D").is_some();
                        let (lu, d) = match Arc::try_unwrap(c) {
                            Ok(c) => (c.lu, c.d),
                            Err(c) => (c.lu.clone(), if warm_d { c.d.clone() } else { None }),
                        };
                        sid::set_warm_lu(Some(Arc::try_unwrap(lu).unwrap_or_else(|lu| (*lu).clone())));
                        // 前回の求解の最終の被約費用は使わない: 主ループが増分で保つ値は求解をまたぐと誤差が積もり
                        // (相対 1e-4 程度)、価格付けしない固定列の値は更新されないので、双対実行可能性の判定を誤る
                        if warm_d {
                            sid::set_warm_d(d);
                        }
                    }
                    Some(c) if c.basis == b => {
                        sid::xcount("n_upd64");
                        sid::set_warm_lu(None)
                    }
                    Some(_) => {
                        sid::xcount("n_diffb");
                        sid::set_warm_lu(None)
                    }
                    None => {
                        sid::xcount("n_nocache");
                        sid::set_warm_lu(None)
                    }
                }
                // 打ち切られた求解の続き (`resume_nb`) なら止まったときの置き場所、それ以外は前回の解から
                if let Some(h) = self.resume_nb.take() {
                    sid::set_warm_nb(Some(h));
                } else if env_str!("ENOMOTO_MIP_NO_NB_HINT").is_none() {
                    let mut h = self.nb_hint();
                    if let Some(p) = self.nb_diag.as_deref_mut() {
                        p.compare(&h, &b, &self.std.lb, &self.std.ub);
                    }
                    // 試作 `ENOMOTO_MIP_BASIS_NB`: 保存した基底の上限・下限の置き場所を重ねる
                    if let Some(w) = self.basis_nb.take() {
                        self.apply_basis_nb(&mut h, &w);
                    }
                    sid::set_warm_nb(Some(h));
                }
                // 前回の最後の DSE 重み (同じ基底のとき)
                if let Some((cb, w)) = self.dse_cache.as_deref() {
                    if *cb == b && w.len() == b.len() {
                        sid::set_warm_dse(Some(w.clone()));
                        sid::set_warm_dse_age(self.dse_age);
                    } else if env_str!("ENOMOTO_MIP_DEBUG_LP").is_some() {
                        eprintln!("TSLP solve: dse_cache basis differs (len {} vs {})", cb.len(), b.len());
                    }
                }
                sid::solve_slope_intercept_dual_from_basis(&self.std, &opts, b)
            }
            _ => sid::solve_slope_intercept_dual(&self.std, &opts),
        };
        sid::xprof("wrapper");
        // 解・双対・基底が変わる (非最適で終わっても双対は変わりうる)
        self.clear_memo();
        sid::set_warm_lu(None);
        sid::set_warm_d(None);
        sid::set_warm_nb(None);
        sid::set_warm_dse(None);
        sid::set_warm_dse_age(0);
        let last_dse = sid::take_last_dse();
        let last_dse_age = sid::last_dse_age();
        self.dse_cache = None;
        sid::set_price_key(None);
        let stop = sid::ext_stop();
        sid::set_ext_control(None);
        sid::clear_ext_progress();
        sid::set_fast_reopt(false);
        let duals = sid::take_duals();
        sid::request_duals(false);
        if r.is_some() {
            sid::request_stop_state(false);
        }
        let last_lu = sid::take_last_lu();
        let last_d = sid::take_last_d();
        let ray = sid::take_last_ray();
        self.ray = None;
        let intercept = sid::take_intercept();
        sid::request_intercept(false);
        self.intercept = None;
        sid::request_lu(false);
        self.lu_cache = None;
        self.iters += sid::ext_iterations() - it0;
        self.lu = None;
        sid::xprof("after");
        match r {
            Some(res) => match res.status {
                Status::Optimal => {
                    let xs = res.x.unwrap_or_default();
                    for j in 0..n {
                        self.x[j] = xs.get(j).copied().unwrap_or(0.0) + self.shift[j];
                    }
                    if let Some((y, bpos)) = duals {
                        self.y = y;
                        let mut b = vec![usize::MAX; self.rows.len()];
                        for (v, p) in bpos.iter().enumerate() {
                            if let Some(p) = *p {
                                if p < b.len() {
                                    b[p] = v;
                                }
                            }
                        }
                        if b.iter().all(|&v| v != usize::MAX) {
                            if let Some(lu) = last_lu {
                                self.lu_cache = Some(Arc::new(LuCache { basis: b.clone(), lu: Arc::new(lu), d: last_d }));
                            }
                            if let Some(w) = last_dse.filter(|w| w.len() == b.len()) {
                                self.dse_cache = Some(Arc::new((b.clone(), w)));
                                self.dse_age = last_dse_age;
                            }
                            self.basis = Some(b);
                            if env_str!("ENOMOTO_MIP_DEBUG_DSE_CHECK").is_some() {
                                self.debug_dse_check(&format!("after solve ({} iters, polish {})", sid::ext_iterations() - it0, sid::ext_polish_iterations()));
                            }
                        }
                    }
                    LpStatus::Optimal
                }
                Status::Infeasible => {
                    // 標準形の行は元の行に符号 σ を掛けたもの
                    self.ray = ray.filter(|r| r.len() >= self.rows.len()).map(|r| (0..self.rows.len()).map(|i| r[i] * self.sigma[i]).collect());
                    LpStatus::Infeasible
                }
                Status::Unbounded | Status::InfeasibleOrUnbounded => {
                    if res.status == Status::Unbounded {
                        self.intercept = intercept;
                    }
                    LpStatus::Unbounded
                }
                _ => LpStatus::Error,
            },
            None => {
                if resume_on && matches!(stop, ExtStop::TimeLimit | ExtStop::IterationLimit) {
                    if let Some((b, w, h)) = sid::take_stop_state() {
                        let m = self.rows.len();
                        let nt = n + m;
                        let mut seen = vec![false; nt];
                        let ok = b.len() == m && w.len() == m && h.len() == nt && b.iter().all(|&v| v < nt && !std::mem::replace(&mut seen[v], true));
                        if ok {
                            if env_str!("ENOMOTO_MIP_DEBUG_LP").is_some() {
                                let changed = self.basis.as_ref().map_or(m, |ob| b.iter().zip(ob).filter(|(a, c)| a != c).count());
                                eprintln!("TSLP solve: {stop:?}; keeping the basis reached so far ({changed} positions changed) for the next solve");
                            }
                            self.basis = Some(b.clone());
                            self.dse_cache = Some(Arc::new((b, w)));
                            self.dse_age = last_dse_age;
                            self.resume_nb = Some(h);
                        }
                    }
                }
                sid::request_stop_state(false);
                match stop {
                ExtStop::IterationLimit => LpStatus::IterationLimit,
                ExtStop::ObjectiveBound => {
                    // 打ち切ったときの双対 (双対証明に使う)
                    let yb = sid::take_bound_duals();
                    if yb.len() == self.rows.len() {
                        self.y = yb;
                    }
                    LpStatus::ObjectiveBound
                }
                ExtStop::TimeLimit => LpStatus::TimeLimit,
                ExtStop::None => LpStatus::Error,
                }
            }
        }
    }

    /// 前回の解での非基底列の位置 (標準形の列ごと: -1 下限、+1 上限、0 不明)。
    fn nb_hint(&self) -> Vec<i8> {
        let n = self.n;
        let m = self.rows.len();
        let mut h = vec![0i8; n + m];
        let at = |v: f64, b: f64| b.is_finite() && (v - b).abs() <= 1e-9 * (1.0 + b.abs());
        for j in 0..n {
            let (v, lo, up) = (self.x[j], self.lo[j], self.up[j]);
            h[j] = if at(v, lo) { -1 } else if at(v, up) { 1 } else { 0 };
        }
        // スラック s_i = b_i - σ_i a_i x (元の空間で): 0 なら下限、上限 (範囲行の幅) なら上限
        let acts = self.row_activities_ref();
        for i in 0..m {
            let act = acts[i];
            let (l, u) = (self.row_lo[i], self.row_up[i]);
            let sl = if self.sigma[i] > 0.0 { u - act } else { act - l };
            let su = self.std.ub[n + i];
            h[n + i] = if sl.abs() <= 1e-9 * (1.0 + act.abs()) { -1 } else if su.is_finite() && (sl - su).abs() <= 1e-9 * (1.0 + su.abs()) { 1 } else { 0 };
        }
        h
    }

    pub fn objective(&self) -> f64 {
        (0..self.n).map(|j| self.cost[j] * self.x[j]).sum()
    }

    /// 今の基底 (基底列の番号、長さ m。構造列 j < n、スラック n + i)。`None` なら全スラック基底。
    pub fn basis_cols(&self) -> Option<Vec<usize>> {
        self.basis.clone()
    }

    /// 診断用 (`ENOMOTO_MIP_DEBUG_CUT_LP_RESOLVE`): 今の LP (行・境界・費用) から新しい LP を作り、`basis`・今の解 `x`・
    /// `weights` を与えて上限なしで解き直し、反復数を表示する。同じプロセス・同じスレッドで、LP の内部状態 (列の平行移動量、
    /// LU のピボット閾値など) だけを新しくしたときに求解の経路が変わるかを見る (analysis/lp_hang_20261021.md)。
    pub fn resolve_fresh_debug(&self, basis: Option<&[usize]>, weights: Option<&[f64]>) {
        let Some(b) = basis else { return };
        let mut lp = TwoStageLp::new(&self.lo, &self.up, &self.cost, &self.rows, &self.row_lo, &self.row_up);
        lp.set_basis_cols(b, Some(&self.x));
        if let Some(w) = weights.filter(|w| w.len() == b.len()) {
            let pairs: Vec<(u32, f32)> = b.iter().zip(w).map(|(&v, &x)| (v as u32, x as f32)).collect();
            lp.set_dse_vars(&pairs);
        }
        let t0 = std::time::Instant::now();
        let st = lp.solve(&SolveLimits::default());
        eprintln!("TSLP resolve_fresh_debug: {st:?} after {} iters ({:.3}s), obj {:.10e}", lp.total_iterations(), t0.elapsed().as_secs_f64(), lp.objective());
    }

    /// 基底を捨てる (次の求解は基底のない求解になり、[`Self::solve`] の同時実行に入る)。試作 `ENOMOTO_MIP_PROBE_BASIS_GUARD`
    /// (analysis/probe_basis_20261106.md) の切り替え (`x`) で、warm start が反復の上限に当たったときに使う。
    pub fn drop_basis(&mut self) {
        self.basis = None;
        self.lu = None;
        self.lu_cache = None;
        self.dse_cache = None;
        self.resume_nb = None;
        self.basis_nb = None;
        self.status = None;
        self.clear_memo();
    }

    /// 試作 `ENOMOTO_MIP_ROOTLP3` (analysis/rootlp3_20261111.md): 次の求解が warm start する基底 (行の数の大きさ) を持つか
    /// ([`Self::solve`] の `cold` の否定)。
    pub fn has_warm_basis(&self) -> bool {
        matches!(&self.basis, Some(b) if b.len() == self.rows.len())
    }

    /// 同: 今の基底の LU が特異か (warm start が全スラック基底からの解き直しになるか。単体法の `refactorize` と同じ分解)。
    /// 基底がなければ偽。
    pub fn warm_basis_singular(&self) -> bool {
        let Some(b) = self.basis.as_ref().filter(|b| b.len() == self.rows.len()) else { return false };
        let mut pos = vec![None; self.std.n_total];
        for (p, &v) in b.iter().enumerate() {
            if v >= pos.len() || pos[v].is_some() {
                return true;
            }
            pos[v] = Some(p);
        }
        super::basis_kernel::factorize_basis(&self.std, &pos, None).is_none()
    }

    /// [`Self::solve_race`] を基底があっても呼ぶ (試作 `ENOMOTO_MIP_PROBE_BASIS_GUARD` の `r`): 二段解法側は今の基底から warm start、
    /// 内点法 + クロスオーバー側は基底なしで同時に解く。`threads` < 2 なら二段解法だけ。
    pub fn solve_race_warm(&mut self, lim: &SolveLimits, threads: usize) -> LpStatus {
        self.solve_race(lim, threads)
    }

    /// 診断用 (`ENOMOTO_MIP_PROBE_BASIS_DIAG`・試作 `ENOMOTO_MIP_PROBE_BASIS_GUARD`): 今の基底 ([`Self::set_basis`] の後、修復済み) の
    /// 出発の点の性質。基底の LU が特異なら `None`。特異でなければ (主実行不能な基底変数の数, その量の和 (1 + |境界| で割った相対値))。
    /// 非基底の構造列は `col` (渡した列の状態) の側 (Upper なら上限、それ以外は有限な下限、なければ上限、なければ 0)、
    /// 非基底のスラックは下限 0 (行がどちらかの境界) に置く。
    pub fn basis_start_info(&self, col: &[VarStatus]) -> Option<(usize, f64)> {
        let b = self.basis.as_ref()?;
        let m = self.rows.len();
        if b.len() != m {
            return None;
        }
        let nt = self.std.n_total;
        let mut pos = vec![None; nt];
        for (p, &v) in b.iter().enumerate() {
            pos[v] = Some(p);
        }
        let lu = super::basis_kernel::factorize_basis(&self.std, &pos, None)?;
        let mut rhs = self.std.b.clone();
        for j in 0..nt {
            if pos[j].is_some() {
                continue;
            }
            let (lb, ub) = (self.std.lb[j], self.std.ub[j]);
            let up = j < self.n && col.get(j) == Some(&VarStatus::Upper);
            let v = if up && ub.is_finite() {
                ub
            } else if lb.is_finite() {
                lb
            } else if ub.is_finite() {
                ub
            } else {
                0.0
            };
            if v != 0.0 {
                for &(i, a) in self.std.cols.col(j) {
                    rhs[i] -= a * v;
                }
            }
        }
        let mut xb = vec![0.0; m];
        let mut scratch = vec![0.0; m];
        lu.solve_into(&rhs, &mut scratch, &mut xb);
        let (mut cnt, mut sum) = (0usize, 0.0f64);
        for (&j, &v) in b.iter().zip(&xb) {
            let (lb, ub) = (self.std.lb[j], self.std.ub[j]);
            let d = if v < lb - 1e-7 {
                (lb - v) / (1.0 + lb.abs())
            } else if v > ub + 1e-7 {
                (v - ub) / (1.0 + ub.abs())
            } else {
                0.0
            };
            if d > 0.0 {
                cnt += 1;
                sum += d;
            }
        }
        Some((cnt, sum))
    }

    /// 診断用: 今の基底を分解して、その厳密な DSE 重み (基底位置順、[`super::DseState::from_basis`]) を返す。
    /// 基底がない・特異なら `None`。
    pub fn exact_dse_weights(&self) -> Option<Vec<f64>> {
        let b = self.basis.as_ref()?;
        let m = self.rows.len();
        if b.len() != m {
            return None;
        }
        let mut pos = vec![None; self.n + m];
        for (p, &v) in b.iter().enumerate() {
            pos[v] = Some(p);
        }
        let lu = super::basis_kernel::factorize_basis(&self.std, &pos, None)?;
        Some(super::DseState::from_basis(m, &lu).w)
    }

    /// 診断用 (`ENOMOTO_MIP_DEBUG_DSE_CHECK`): 今の基底について、次の求解に渡す DSE 重みと厳密な重みを比べて表示する
    /// (`tag` は場面)。重みがなければその旨。比が 10 倍以上・1/10 以下の行の数と最大・最小の比、比が最大の 3 行を出す。
    pub fn debug_dse_check(&self, tag: &str) {
        let Some(b) = &self.basis else { return };
        let m = self.rows.len();
        let Some(w) = self.dse_weights() else {
            eprintln!("DSE_CHECK {tag}: rows {m}, no weights to carry");
            return;
        };
        let Some(ex) = self.exact_dse_weights() else {
            eprintln!("DSE_CHECK {tag}: rows {m}, basis singular");
            return;
        };
        let mut ratios: Vec<(f64, usize)> = (0..m).filter(|&i| !w[i].is_nan()).map(|i| (w[i] / ex[i], i)).collect();
        ratios.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let k = ratios.len();
        let (over, under) = (ratios.iter().filter(|r| r.0 >= 10.0).count(), ratios.iter().filter(|r| r.0 <= 0.1).count());
        let top: Vec<String> = ratios.iter().rev().take(3).map(|&(r, i)| format!("pos {i} var {} w {:.2e} exact {:.2e} ratio {:.1e}", b[i], w[i], ex[i], r)).collect();
        let bot: Vec<String> = ratios.iter().take(2).map(|&(r, i)| format!("pos {i} var {} w {:.2e} exact {:.2e} ratio {:.1e}", b[i], w[i], ex[i], r)).collect();
        eprintln!("DSE_CHECK {tag}: rows {m} (nan {}), ratio >=10: {over}, <=0.1: {under}, max {:.1e}, min {:.1e} | {} | {}", m - k, ratios.last().map_or(0.0, |r| r.0), ratios.first().map_or(0.0, |r| r.0), top.join("; "), bot.join("; "));
    }

    /// 次の求解に渡す DSE 重み (基底位置順。今の基底のものがなければ `None`)。
    pub fn dse_weights(&self) -> Option<Vec<f64>> {
        match (&self.basis, self.dse_cache.as_deref()) {
            (Some(b), Some((cb, w))) if b == cb && w.len() == b.len() => Some(w.clone()),
            _ => None,
        }
    }

    /// 診断用 (`ENOMOTO_MIP_DEBUG_DUMP_CUT_LP`): 今の LP (行・列の境界・費用、カットの行を含む。最小化形、整数の印なし) を
    /// `<prefix>.mps` (自由 MPS、列 `c<番号>`・行 `r<番号>`) に、出発の基底を `<prefix>.bas` に書き出す。`.bas` は
    /// 1 行目 `n m`、2 行目 基底列の番号 (m 個。空なら全スラック基底)、3 行目 直近の解 `x` (n 個。非基底列の置き場所の希望
    /// [`Self::nb_hint`] の元)、4 行目 次の求解に渡す DSE 重み (基底位置順。引き継ぐ重みがなければ空行)。`basis` を渡せばその基底を書く (求解の前に取り出した出発の基底)。
    /// 読み戻しは `cut_lp_debug` テスト (ENOMOTO) と `analysis/lp_hang_20261021/solve_dump.py` (HiGHS)。
    pub fn dump_lp(&self, prefix: &str, basis: Option<&[usize]>, weights: Option<&[f64]>) -> std::io::Result<()> {
        use std::fmt::Write as _;
        let n = self.n;
        let m = self.rows.len();
        let p = crate::mip::problem::MipProblem::from_rows(self.lo.clone(), self.up.clone(), self.cost.clone(), 0.0, 1.0, vec![false; n], self.rows.clone(), self.row_lo.clone(), self.row_up.clone());
        p.write_mps(&format!("{prefix}.mps"))?;
        let mut o = String::new();
        let _ = writeln!(o, "{n} {m}");
        let b: Vec<String> = basis.map(|b| b.to_vec()).or_else(|| self.basis.clone()).unwrap_or_default().iter().map(|v| v.to_string()).collect();
        let _ = writeln!(o, "{}", b.join(" "));
        let xs: Vec<String> = self.x.iter().map(|v| format!("{v:.17e}")).collect();
        let _ = writeln!(o, "{}", xs.join(" "));
        // 4 行目: 次の求解に渡す DSE 重み (基底位置順、[`Self::dse_cache`] が書く基底のものなら。なければ空行)
        let ws: Vec<String> = weights.map(|w| w.to_vec()).or_else(|| self.dse_weights()).unwrap_or_default().iter().map(|v| format!("{v:.17e}")).collect();
        let _ = writeln!(o, "{}", ws.join(" "));
        std::fs::write(format!("{prefix}.bas"), o)
    }

    pub fn col_values(&self) -> Vec<f64> {
        self.x.clone()
    }

    pub fn col_value(&self, j: usize) -> f64 {
        self.x[j]
    }

    pub fn row_activities(&self) -> Vec<f64> {
        self.row_activities_ref().to_vec()
    }

    /// [`Self::row_activities`] の参照 (同じ解で何度呼んでも 1 回だけ計算する)。
    pub fn row_activities_ref(&self) -> &[f64] {
        self.act_memo.get_or_init(|| self.rows.iter().map(|r| r.iter().map(|&(j, a)| a * self.x[j]).sum()).collect())
    }

    /// 被約費用 `c - A^T y` (元の空間。標準形の行の符号を考慮)。
    pub fn reduced_costs(&self) -> Vec<f64> {
        self.reduced_costs_ref().to_vec()
    }

    /// [`Self::reduced_costs`] の参照 (同じ状態で何度呼んでも 1 回だけ計算する)。
    pub fn reduced_costs_ref(&self) -> &[f64] {
        self.rc_memo.get_or_init(|| self.compute_reduced_costs())
    }

    fn compute_reduced_costs(&self) -> Vec<f64> {
        let mut d = self.cost.clone();
        for (i, r) in self.rows.iter().enumerate() {
            let yi = self.y.get(i).copied().unwrap_or(0.0) * self.sigma[i];
            if yi != 0.0 {
                for &(j, a) in r {
                    d[j] -= yi * a;
                }
            }
        }
        d
    }

    /// 行の双対値 (元の行の向き)。
    pub fn farkas_ray(&self) -> Option<Vec<f64>> {
        self.ray.clone()
    }

    pub fn row_duals(&self) -> Vec<f64> {
        (0..self.rows.len()).map(|i| self.y.get(i).copied().unwrap_or(0.0) * self.sigma[i]).collect()
    }

    /// 基底位置 `s` の `e_s^T B^{-1}` を、元の行に掛ける重みとして返す
    /// (`sum_i w_i (a_i x - r_i) = 0` が基底変数 `basic_var(s)` についての tableau 行)。
    /// 試作 `ENOMOTO_MIP_OPTFACE_MOVE`・`ENOMOTO_MIP_OPTFACE_PARCUTS` (既定では呼ばない。analysis/optface_cuts_20261105.md): 最適な
    /// 今の基底から、被約費用が (許容誤差で) 0 の非基底変数を入れる主単体法の軸の入れ替え (双対退化の向き) を最大 `k` 回行い、
    /// 目的値を変えずに最適面の上の別の最適基底 (頂点) へ移る。比の判定は Harris の 2 段 (主の実行可能性を保つ)。入れる変数は
    /// 候補から最大 `sample` 本を引いて FTRAN し、`mode` で選ぶ: `r` 乱数 (最初に引いた有効な候補)、`f` 分数の値の整数の基底列の
    /// tableau の行を変える数 (α ≠ 0) が多い候補、`d` 動く距離 (構造列の値の変化の和) が大きい候補 (退化でない入れ替えを優先)。
    /// 一度基底から出た変数は入れ直さない (戻りを避ける)。終わったら今の基底と値をこの LP に入れ、warm start で解き直して最適と
    /// 目的値を確かめる (解き直しの反復も返す)。最適でない・基底がない・分解できないなら `None`。
    pub fn optface_move(&mut self, k: usize, sample: usize, mode: char, seed: u64, is_int: &[bool], deadline: Option<std::time::Instant>) -> Option<OptFaceStats> {
        let t0 = std::time::Instant::now();
        if self.status != Some(LpStatus::Optimal) {
            return None;
        }
        let n = self.n;
        let m = self.rows.len();
        let nt = n + m;
        let mut basis = self.basis.clone().filter(|b| b.len() == m)?;
        if self.y.len() != m || self.std.n_total != nt {
            return None;
        }
        let obj0 = self.objective();
        let mut z: Vec<f64> = (0..nt).map(|v| if v < n { self.x[v] - self.shift[v] } else { self.slack_value(v - n, &self.x) }).collect();
        let x_start = z.clone();
        let mut pos: Vec<Option<usize>> = vec![None; nt];
        for (p, &v) in basis.iter().enumerate() {
            pos[v] = Some(p);
        }
        let mut lu = super::basis_kernel::factorize_basis(&self.std, &pos, None)?;
        let (lb, ub, c) = (&self.std.lb, &self.std.ub, &self.std.c);
        let btran = |lu: &super::lu::FtLu, basis: &[usize]| -> Vec<f64> {
            let cb: Vec<f64> = basis.iter().map(|&v| c[v]).collect();
            let mut scratch = vec![0.0; m];
            let mut out = vec![0.0; m];
            lu.solve_transpose_into(&cb, &mut scratch, &mut out);
            out
        };
        let mut y = btran(&lu, &basis);
        let mut rng = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            rng
        };
        let mut tabu = vec![false; nt];
        let mut st = OptFaceStats::default();
        let ptol = 1e-9;
        let frac = |v: usize, zv: f64| -> bool {
            if v >= n || !is_int.get(v).copied().unwrap_or(false) {
                return false;
            }
            let xv = zv + self.shift[v];
            (xv - xv.round()).abs() > 1e-6
        };
        while st.pivots + st.flips < k {
            if deadline.is_some_and(|d| std::time::Instant::now() >= d) {
                st.stopped = true;
                break;
            }
            // 被約費用 0 の非基底の候補 (向き付き)
            let mut cands: Vec<(usize, f64)> = Vec::new();
            for v in 0..nt {
                if pos[v].is_some() || tabu[v] || lb[v] >= ub[v] {
                    continue;
                }
                let d = c[v] - self.std.cols.col(v).iter().map(|&(i, a)| a * y[i]).sum::<f64>();
                if d.abs() > 1e-9 * (1.0 + c[v].abs()) {
                    continue;
                }
                let at_l = lb[v].is_finite() && (z[v] - lb[v]).abs() <= 1e-9 * (1.0 + lb[v].abs());
                let at_u = ub[v].is_finite() && (z[v] - ub[v]).abs() <= 1e-9 * (1.0 + ub[v].abs());
                let dir = if at_l {
                    1.0
                } else if at_u {
                    -1.0
                } else if !lb[v].is_finite() && !ub[v].is_finite() {
                    if next() & 1 == 0 { 1.0 } else { -1.0 }
                } else {
                    continue;
                };
                cands.push((v, dir));
            }
            if st.pivots + st.flips == 0 {
                st.cands0 = cands.len();
            }
            if cands.is_empty() {
                break;
            }
            // 候補を引いて FTRAN と比の判定
            let s = sample.max(1).min(cands.len());
            for i in 0..s {
                let j = i + (next() as usize) % (cands.len() - i);
                cands.swap(i, j);
            }
            // (候補, 向き, 歩幅 t, 出る位置 (None は境界の入れ替え), α, 点数)
            let mut best: Option<(usize, f64, f64, Option<usize>, Vec<f64>, f64)> = None;
            for &(q, dir) in &cands[..s] {
                let mut rhs = vec![0.0; m];
                for &(i, a) in self.std.cols.col(q).iter() {
                    rhs[i] = a;
                }
                let alpha = lu.solve(&rhs);
                st.ftran += 1;
                // Harris の 1 段目: 許容誤差つきの上限
                let mut tmax = if lb[q].is_finite() && ub[q].is_finite() { ub[q] - lb[q] } else { f64::INFINITY };
                for (p, &a) in alpha.iter().enumerate() {
                    if a.abs() <= ptol {
                        continue;
                    }
                    let v = basis[p];
                    let delta = -dir * a;
                    let lim = if delta < 0.0 {
                        if !lb[v].is_finite() { continue }
                        (z[v] - lb[v] + 1e-9 * (1.0 + lb[v].abs())) / -delta
                    } else {
                        if !ub[v].is_finite() { continue }
                        (ub[v] - z[v] + 1e-9 * (1.0 + ub[v].abs())) / delta
                    };
                    tmax = tmax.min(lim);
                }
                if !tmax.is_finite() {
                    continue; // 最適面の中の半直線 (動かない)
                }
                // 2 段目: 上限以下の中で |α| が最大の行
                let flip = if lb[q].is_finite() && ub[q].is_finite() { ub[q] - lb[q] } else { f64::INFINITY };
                let mut r: Option<usize> = None;
                let mut ra = 0.0;
                let mut t = flip;
                for (p, &a) in alpha.iter().enumerate() {
                    if a.abs() <= ptol {
                        continue;
                    }
                    let v = basis[p];
                    let delta = -dir * a;
                    let lim = if delta < 0.0 {
                        if !lb[v].is_finite() { continue }
                        (z[v] - lb[v]) / -delta
                    } else {
                        if !ub[v].is_finite() { continue }
                        (ub[v] - z[v]) / delta
                    };
                    if lim <= tmax && a.abs() > ra {
                        ra = a.abs();
                        r = Some(p);
                        t = lim.max(0.0);
                    }
                }
                if r.is_some() && flip <= t {
                    r = None;
                    t = flip;
                }
                if r.is_none() && !flip.is_finite() {
                    continue;
                }
                let noise = (next() % 1024) as f64 / 1024.0 * 1e-3;
                let score = match mode {
                    'f' => alpha.iter().enumerate().filter(|&(p, &a)| a.abs() > ptol && Some(p) != r && frac(basis[p], z[basis[p]])).count() as f64 + noise,
                    'd' => {
                        let mv: f64 = alpha.iter().enumerate().filter(|&(p, _)| basis[p] < n).map(|(_, a)| a.abs()).sum::<f64>() + if q < n { 1.0 } else { 0.0 };
                        (t * mv).min(1e12) + if t > 1e-9 { 1e6 } else { 0.0 } + noise
                    }
                    _ => noise,
                };
                if best.as_ref().is_none_or(|b| score > b.5) {
                    best = Some((q, dir, t, r, alpha, score));
                }
                if mode == 'r' && best.is_some() {
                    break;
                }
            }
            let Some((q, dir, t, r, alpha, _)) = best else {
                // 引いた候補がどれも動けない: 引いた分を除いて続ける
                for &(q, _) in &cands[..s] {
                    tabu[q] = true;
                }
                st.dead += s;
                continue;
            };
            // 値の更新
            if t > 0.0 {
                z[q] += dir * t;
                for (p, &a) in alpha.iter().enumerate() {
                    if a != 0.0 {
                        let v = basis[p];
                        z[v] -= dir * a * t;
                    }
                }
            }
            match r {
                None => {
                    // 境界の入れ替え (基底は同じ、点が動く)
                    z[q] = if dir > 0.0 { ub[q] } else { lb[q] };
                    st.flips += 1;
                    tabu[q] = true;
                }
                Some(p) => {
                    let v = basis[p];
                    z[v] = if -dir * alpha[p] < 0.0 { lb[v] } else { ub[v] };
                    basis[p] = q;
                    pos[v] = None;
                    pos[q] = Some(p);
                    match super::basis_kernel::factorize_basis(&self.std, &pos, None) {
                        Some(l) => lu = l,
                        None => {
                            // 特異: 戻してやめる
                            basis[p] = v;
                            pos[q] = None;
                            pos[v] = Some(p);
                            st.singular = true;
                            break;
                        }
                    }
                    tabu[v] = true;
                    st.pivots += 1;
                    if t <= 1e-12 {
                        st.degenerate += 1;
                    }
                    y = btran(&lu, &basis);
                }
            }
        }
        // 基底の値を今の LU で計算し直す (誤差を消す): x_B = B^-1 (b - N x_N)
        let mut rhs = self.std.b.clone();
        for v in 0..nt {
            if pos[v].is_none() && z[v] != 0.0 {
                for &(i, a) in self.std.cols.col(v).iter() {
                    rhs[i] -= a * z[v];
                }
            }
        }
        let xb = lu.solve(&rhs);
        for (p, &v) in basis.iter().enumerate() {
            z[v] = xb[p];
        }
        st.moved_cols = (0..n).filter(|&j| (z[j] - x_start[j]).abs() > 1e-7 * (1.0 + x_start[j].abs())).count();
        let b0: std::collections::HashSet<usize> = self.basis.as_ref().unwrap().iter().copied().collect();
        st.basis_diff = basis.iter().filter(|v| !b0.contains(v)).count();
        st.secs_move = t0.elapsed().as_secs_f64();
        if st.pivots + st.flips == 0 {
            return Some(st);
        }
        // この LP に入れて解き直す (最適の確かめ。双対の許容誤差の分だけ数反復かかることがある)
        for j in 0..n {
            self.x[j] = z[j] + self.shift[j];
        }
        self.basis = Some(basis);
        self.lu = None;
        self.lu_cache = None;
        self.dse_cache = None;
        self.status = None;
        self.ray = None;
        self.clear_memo();
        // 確かめの解き直しは移動の期限 (もう過ぎていることがある) ではなく反復の上限で縛る (ふつうは 0〜数反復)
        let it0 = self.iters;
        let stt = self.solve(&SolveLimits { iteration_limit: (1000 + 2 * m) as u64, ..Default::default() });
        st.resolve_iters = self.iters - it0;
        st.obj_diff = self.objective() - obj0;
        st.ok = stt == LpStatus::Optimal && st.obj_diff.abs() <= 1e-7 * obj0.abs().max(1.0);
        st.secs = t0.elapsed().as_secs_f64();
        Some(st)
    }

    pub fn basis_inverse_row(&mut self, s: usize) -> Vec<f64> {
        let m = self.rows.len();
        if self.lu.is_none() {
            // 最適基底の LU を共有する (解くだけなので複製しない)
            if let (Some(c), Some(b)) = (&self.lu_cache, &self.basis) {
                if c.basis == *b {
                    self.lu = Some(c.lu.clone());
                }
            }
        }
        if self.lu.is_none() {
            let n_total = self.n + m;
            let mut pos: Vec<Option<usize>> = vec![None; n_total];
            match &self.basis {
                Some(b) => {
                    for (k, &v) in b.iter().enumerate() {
                        pos[v] = Some(k);
                    }
                }
                None => {
                    for i in 0..m {
                        pos[self.n + i] = Some(i);
                    }
                }
            }
            self.lu = super::basis_kernel::factorize_basis(&self.std, &pos, None).map(Arc::new);
        }
        let Some(lu) = &self.lu else { return vec![0.0; m] };
        // σ_i a_i x + s_i = b_i で s_i = b_i - σ_i r_i なので、重みは ρ_i σ_i
        // (スラックが基底の行では、基底変数 s_i の係数は 1 で、r_i については -σ_i)
        let k = self.basic_var(s);
        let sc = if k >= self.n { -self.sigma[k - self.n] } else { 1.0 };
        // 既定: 反復の中と同じ超疎の単位 BTRAN (値は密な版とビット一致)。作業領域はスレッドごとに使い回し、LU の決定的演算量
        // カウンタ (双対単体法の再分解の判断が読む) には残さない。`ENOMOTO_MIP_BINV_ROW_DENSE=1` で従来の密な BTRAN
        if env_str!("ENOMOTO_MIP_BINV_ROW_DENSE").is_none_or(|v| v == "0" || v == "check") {
            let tick0 = lu.synth_tick();
            let r = BINV_ROW_WORK.with(|c| {
                let mut c = c.borrow_mut();
                if c.as_ref().is_none_or(|w| w.0 != m) {
                    *c = Some((m, super::lu::UnitBtranWork::new(m), vec![0.0; m], vec![0.0; m]));
                }
                let (_, work, out, et) = c.as_mut().unwrap();
                work.set_density(0.0);
                lu.solve_transpose_unit_work_sparse(s, out, et, work, None);
                let mut r = vec![0.0; m];
                match work.nonzero_rows() {
                    Some(rows) => {
                        for &i in rows {
                            r[i] = out[i] * self.sigma[i] / sc;
                        }
                    }
                    None => {
                        for i in 0..m {
                            r[i] = out[i] * self.sigma[i] / sc;
                        }
                    }
                }
                r
            });
            lu.restore_synth_tick(tick0);
            // 検査 `ENOMOTO_MIP_BINV_ROW_DENSE=check`: 密な版と照合し、ビット一致しない要素の数を数える
            if env_str!("ENOMOTO_MIP_BINV_ROW_DENSE") == Some("check") {
                let mut out = vec![0.0; m];
                let mut scratch = vec![0.0; m];
                lu.solve_transpose_unit(s, &mut scratch, &mut out);
                let bad = (0..m).filter(|&i| (out[i] * self.sigma[i] / sc).to_bits() != r[i].to_bits() && !(r[i] == 0.0 && out[i] == 0.0)).count();
                BINV_ROW_CHECK[0].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                BINV_ROW_CHECK[1].fetch_add(bad as u64, std::sync::atomic::Ordering::Relaxed);
                lu.restore_synth_tick(tick0);
            }
            return r;
        }
        let mut out = vec![0.0; m];
        let mut scratch = vec![0.0; m];
        lu.solve_transpose_unit(s, &mut scratch, &mut out);
        (0..m).map(|i| out[i] * self.sigma[i] / sc).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_lp_and_bound_change() {
        // min -x - y  s.t. x + 2y <= 4, 3x + y <= 6, x, y in [0, 10]
        let mut lp = TwoStageLp::new(&[0.0, 0.0], &[10.0, 10.0], &[-1.0, -1.0], &[vec![(0, 1.0), (1, 2.0)], vec![(0, 3.0), (1, 1.0)]], &[f64::NEG_INFINITY; 2], &[4.0, 6.0]);
        assert_eq!(lp.solve(&SolveLimits::default()), LpStatus::Optimal);
        assert!((lp.objective() - (-2.8)).abs() < 1e-9, "{}", lp.objective());
        lp.set_col_bounds(0, 0.0, 1.0);
        assert_eq!(lp.solve(&SolveLimits::default()), LpStatus::Optimal);
        assert!((lp.objective() - (-2.5)).abs() < 1e-9, "{}", lp.objective());
        lp.set_col_bounds(0, 3.0, 10.0);
        assert_eq!(lp.solve(&SolveLimits::default()), LpStatus::Infeasible);
    }

    #[test]
    fn optface_move_keeps_objective() {
        // min -x - y  s.t. x + y <= 4, x, y in [0, 3]: 最適面は (1, 3)-(3, 1) の線分 (双対退化)。最適面の上を動いて
        // 目的値 -4 のまま別の頂点に移る
        let mut lp = TwoStageLp::new(&[0.0, 0.0], &[3.0, 3.0], &[-1.0, -1.0], &[vec![(0, 1.0), (1, 1.0)]], &[f64::NEG_INFINITY], &[4.0]);
        assert_eq!(lp.solve(&SolveLimits::default()), LpStatus::Optimal);
        let x0 = lp.col_values();
        for mode in ['f', 'd', 'r'] {
            let mut alt = lp.clone();
            let st = alt.optface_move(10, 4, mode, 1, &[true, true], None).expect("optimal basis");
            assert!(st.cands0 >= 1, "{st:?}");
            assert!(st.ok, "{st:?}");
            assert!((alt.objective() - (-4.0)).abs() < 1e-9, "{}", alt.objective());
            let x = alt.col_values();
            assert!(x[0] + x[1] <= 4.0 + 1e-9 && x.iter().all(|&v| (-1e-9..=3.0 + 1e-9).contains(&v)), "{x:?}");
            assert!(st.pivots + st.flips >= 1 && (x[0] - x0[0]).abs() > 1e-6, "mode {mode}: {st:?} {x0:?} -> {x:?}");
        }
    }

    #[test]
    fn interior_point_and_crossover_vertex() {
        // min -x - y  s.t. x + y <= 4, x, y in [0, 3]: 最適面は (1, 3)-(3, 1) の線分。内点法の点はその内側、
        // 押し出しの頂点は実行可能な頂点
        let lp = TwoStageLp::new(&[0.0, 0.0], &[3.0, 3.0], &[-1.0, -1.0], &[vec![(0, 1.0), (1, 1.0)]], &[f64::NEG_INFINITY], &[4.0]);
        let far = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let ip = lp.interior_point(false, true, far).expect("interior point");
        assert!((ip.x[0] + ip.x[1] - 4.0).abs() < 1e-5, "{:?}", ip.x);
        assert!(ip.x[0] > 1.0 + 1e-3 && ip.x[0] < 3.0 - 1e-3, "not interior to the optimal face: {:?}", ip.x);
        let (b, xv) = ip.vertex.expect("vertex");
        assert_eq!(b.len(), 1);
        let xv = xv.expect("primal feasible vertex");
        assert!(xv[0] + xv[1] <= 4.0 + 1e-9 && xv.iter().all(|&v| (-1e-9..=3.0 + 1e-9).contains(&v)), "{xv:?}");
        // 費用 0: 実行可能領域の中心付近
        let ac = lp.interior_point(true, false, far).expect("analytic center");
        assert!(ac.x[0] + ac.x[1] < 4.0 - 1e-3 && ac.x.iter().all(|&v| v > 1e-3 && v < 3.0 - 1e-3), "{:?}", ac.x);
    }

    #[test]
    fn warm_start_from_dual_infeasible_basis() {
        // min -x - 2y  s.t. x + y <= 4, x, y in [0, 3]。基底 {x} (y、スラックは非基底) は被約費用
        // d_y = -2 - (-1) = -1 < 0 で双対実行可能でない。そこから始めても最適値 -7 (x = 1, y = 3) に達する
        let mut lp = TwoStageLp::new(&[0.0, 0.0], &[3.0, 3.0], &[-1.0, -2.0], &[vec![(0, 1.0), (1, 1.0)]], &[f64::NEG_INFINITY], &[4.0]);
        lp.set_basis_cols(&[0], Some(&[3.0, 0.0]));
        assert_eq!(lp.solve(&SolveLimits::default()), LpStatus::Optimal);
        assert!((lp.objective() - (-7.0)).abs() < 1e-9, "{}", lp.objective());
    }
}

/// 試作 `ENOMOTO_MIP_NODECUT_LIVE`・診断 `ENOMOTO_MIP_NODECUT_STAT` (analysis/nodecutoff_20261106.md) の打ち切り値の読み直し口。
/// 分枝限定法が `limits()` で入れ、打ち切り値 (`SolveLimits::cutoff`) がビット単位で同じ求解だけが使う
/// (サブ MIP など、別の求解の値を取り違えないため)。`kind` は 0 = ノードの LP、1 = 強分枝の子、2 = 潜り、3 = その他
pub(crate) struct NodecutSrc {
    pub cutoff_bits: u64,
    pub f: std::rc::Rc<dyn Fn() -> f64>,
    pub kind: u8,
}

thread_local! {
    static NC_SRC_ON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static NC_SRC: std::cell::RefCell<Option<NodecutSrc>> = const { std::cell::RefCell::new(None) };
}

/// [`NodecutSrc`] を入れる (`None` で外す)。
pub(crate) fn set_nodecut_src(s: Option<NodecutSrc>) {
    NC_SRC_ON.with(|c| c.set(s.is_some()));
    NC_SRC.with(|c| *c.borrow_mut() = s);
}

/// `ENOMOTO_MIP_NODECUT_LIVE` (試作、既定では使わない): 双対単体法の途中で共有の最良値を読み直して打ち切り値を下げる。
pub(crate) fn nodecut_live() -> bool {
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| env_str!("ENOMOTO_MIP_NODECUT_LIVE").is_some_and(|v| v != "0"))
}

/// `ENOMOTO_MIP_NODECUT_STAT` (診断、既定では出さない): 打ち切り値による LP の打ち切りの計数をログの終わりに出す。
pub(crate) fn nodecut_stat() -> bool {
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| env_str!("ENOMOTO_MIP_NODECUT_STAT").is_some_and(|v| v != "0"))
}

/// [`nodecut_stat`] の計数 (種類ごとに 32 個、プロセス全体)。添字は [`nodecut_report`] を見よ。
pub(crate) static NODECUT_STATS: [std::sync::atomic::AtomicU64; 128] = [const { std::sync::atomic::AtomicU64::new(0) }; 128];

/// [`nodecut_stat`] の計数を出す。
pub(crate) fn nodecut_report() {
    let names = ["node", "sb", "dive", "other"];
    for (k, name) in names.iter().enumerate() {
        let v: Vec<u64> = (0..32).map(|i| NODECUT_STATS[32 * k + i].load(std::sync::atomic::Ordering::Relaxed)).collect();
        if v[0] == 0 {
            continue;
        }
        eprintln!(
            "MIP: nodecut {name}: lps {} iters {} | objbound stops {} iters {} | optimal {} iters {} | optimal >= start cutoff {} iters {} (no stage B {} iters {}; stage B iters after entering B {}; stage B but never compared {}) | optimal < start but >= cutoff re-read at end {} iters {} est saved {} (no stage B {}) | cutoff lowered during LP {} (objbound after lowering {} iters {}) | objbound where re-read cutoff hit earlier {} est saved {} | infeasible {} iters {} | start cutoff inf {} | bound checks {} of which -inf {} | optimal >= start with the last bound -inf {} iters {} | -inf bound whose relaxed value (skipping the -inf columns) passed the cutoff {} est saved {}",
            v[0], v[1], v[2], v[3], v[18], v[19], v[4], v[5], v[6], v[7], v[16], v[17], v[8], v[9], v[10], v[21], v[11], v[12], v[13], v[14], v[15], v[22], v[23], v[20], v[24], v[25], v[26], v[27], v[28], v[29]
        );
    }
}
