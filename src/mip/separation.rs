//! 根での切除平面のループ (HiGHS の `HighsSeparation` / `HighsTableauSeparator` を簡略化したもの)。
//!
//! 各ラウンドで、元の行 1 本ずつと、基底にある小数の整数変数の tableau 行 (基底逆行列の行で LP 行を
//! 集約したもの、Gomory 相当) を CMIR に通して切除平面を作り、効き目の大きい順に、ほぼ平行なものを
//! 除いて LP に加えて解き直す。目的値がほとんど動かなくなったら止める。最後に効いていない (論理変数が
//! 基底にある) カット行を LP から外す。カットは大域的な境界から作るので、木全体で有効。

use super::cuts::{cmir, extended_cover, generate_cuts, lifted_cover, lifted_flow_cover, CutVars, RawCut, VarBounds};
use super::problem::MipProblem;
use super::domain::FEASTOL;
use super::lp::{LpStatus, SolveLimits, VarStatus};
use super::solver::Solver;

thread_local! {
    /// `ENOMOTO_MIP_CLIQUE_CUTS=auto` の、最初の求解 (再スタート前) の判定。再スタートの後の求解が引き継ぐ
    static CLIQUE_AUTO_FIRST: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
    /// `ENOMOTO_MIP_CLIQUE_CUTS=fallback` で clique カットを加えて回す 2 回目のループの中か (時間の上限をループの始めから数える)
    static CLIQUE_FB_SECOND: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// [`Solver::probe_cut_mode`] で、目的値が動かないラウンドでも止めずに k ラウンド回すか (試作 `ENOMOTO_MIP_CLIQUE_CUTS=probe` の比べ。
    /// neos-1456979 はクリークカットを入れても 3 ラウンド目までは目的値が 154 のままで、4 ラウンド目から伸びる)
    static PROBE_NO_FLAT_STOP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// [`Solver::probe_cut_mode`] で選んだカットのうちクリークの分離器から来た本数 (試作 `ENOMOTO_MIP_CLIQUE_CUTS=probe_s` の数え)
    static PROBE_CLIQUE_ROWS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// `ENOMOTO_MIP_CUT_EXTEND` で根のカットの時間の上限を延ばした後か (延ばした後は面の頂点での分離を止める)
    static CUT_EXTENDED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
use super::lp_api::MipLp;

/// LP に加える前のカット (構造変数の係数、右辺、効き目)。
/// lifted flow cover を使うか (`ENOMOTO_MIP_NO_FLOWCOVER` で無効)。
/// HiGHS の lifted cover を CMIR と並べて試すか (`ENOMOTO_MIP_LIFTED_COVER` で有効)。qnet1 では根の下界が
/// 15627 -> 15924 (HiGHS 16022) と上がったが、40 問では 19 -> 17 問 (sgeomean 27.91 -> 29.06) と悪化した
/// (misc07・neos5・binkar10_1・neos-860300 が遅くなった) ので既定では使わない。
/// clique カットの使い方 (`ENOMOTO_MIP_CLIQUE_CUTS`)。既定は `fallback` (根のループを clique なしで回し、ほかのカットで伸びず
/// clique カットで伸びる問題でだけ根の続きで使う、[`Solver::clique_fallback_trial`])。`off` (か `0`) で使わない。
/// 40 問 + 16 問 (4 スレッド・60 秒、`ENOMOTO_T_MIP_CLIQUE_FB_FRAC=0.9` と組んで 2 組) で drayage-25-23 が 2 組とも解け、
/// 40 問は解けた数・不一致とも変わらない。手元 16 スレッドでも確かめて既定にした
pub(super) fn clique_cuts_mode() -> Option<&'static str> {
    match env_str!("ENOMOTO_MIP_CLIQUE_CUTS") {
        Some("off") | Some("0") => None,
        Some(m) => Some(m),
        None => Some("fallback"),
    }
}

/// 試作 `ENOMOTO_MIP_HO5_CUTLP_GAIN` = g (既定では使わない、analysis/highs_only_20261105/G5.md): 主の根のカットのループで、カットを足した LP が
/// 1 回の LP の反復の上限 (`ENOMOTO_T_MIP_CUT_LP_WORK`、最低 1000) の後の進みの判定 (`ENOMOTO_MIP_CUT_LP_CAP_ROBUST` の `p`) で止まったとき、
/// 出発の目的値からの伸びが g x max(|出発の目的値|, 1) 以上なら、カットを外さずに止まったところから続けて解き切る (反復の上限は
/// `ENOMOTO_T_MIP_HO5_CUTLP_GAIN_ITER_MULT` (50) x 上限と行の数の 2 倍の大きい方、時間は 1 回の LP の時間の上限の
/// `ENOMOTO_T_MIP_HO5_CUTLP_GAIN_TIME_MULT` (3) 倍)。neos-4722843-widden (11 万行) は根の LP 8220 反復で上限 1000 になり、最初のカットの LP が
/// 1250 反復で目的値を 472.9 から 9128 まで上げている途中で外され、`ENOMOTO_MIP_CUT_LP_CAP_STOP` で主のループが 0 本で止まっていた。
/// 値なし (`1` など数でない値) は g = 1
fn ho5_cutlp_gain() -> f64 {
    match env_str!("ENOMOTO_MIP_HO5_CUTLP_GAIN") {
        None => 0.0,
        Some(v) => v.parse::<f64>().ok().filter(|g| *g > 0.0).unwrap_or(1.0),
    }
}

/// `ENOMOTO_MIP_CUTSTALL_GAP` (試作、既定では使わない。analysis/cutloop_stop_20261106/B.md): 根 (サブ MIP でない。裏のカットの写しを含む) の
/// カットのループの停滞の判定を、目的値の大きさではなく **ギャップの閉じ方** でも測り、どちらの尺度でも伸びていないラウンドだけを停滞と数える。
/// - 尺度は min(カットで上げた分 (今の値 - ループの始めの値), 暫定解があれば打ち切り値との残りの差)。前者は HiGHS の `evaluateRootNode` の
///   「今までの伸びの 0.1%」と同じ尺度、後者は根のギャップが閉じかけているときに少しの伸びでも続けるため
/// - ラウンドの伸びが割合 × 尺度以下なら新しい尺度で停滞。今までの判定 (目的値の相対変化 1e-4) でも停滞のときだけ停滞と数える (今までより早く止まることはない)
/// - 裏のカットの写しの窓の判定 ([`super::solver::CutJob::record_round`]) も同じ尺度で、窓の伸びが窓のラウンド数 × 割合 × 尺度を超えていれば停滞としない
/// - ラウンドの上限に当たったとき、直近 5 ラウンドの伸びが新しい尺度で停滞でなければ (5 ラウンドの伸び > 5 × 割合 × 尺度) 25 ラウンド延ばす
///   (`ENOMOTO_T_MIP_CUTSTALL_MAX_ROUNDS`、既定 100 まで)。時間の上限 (`ENOMOTO_T_MIP_CUT_TIME_FRAC`) はそのまま効く
///
/// 値: 数 (0 < t < 1) ならその割合、ほかは 1e-3。末尾に `n` を付けるとラウンドの延長をしない (例 `1n`)。
/// unitcal_7 は目的値 1.96e7 に対し 1 ラウンドの伸びが 300〜1000 で、今までの判定では相対変化 1.5e-5〜5e-5 の「停滞」になり 17 ラウンド前後で止まる
/// `ENOMOTO_MIP_CUTSTALL_GAP` の「カットで上げた分」の基準: 外側の最初の根のカットの前の LP の値 (前処理の定数項を足した元の問題の目的値)。
/// 再スタートの後の根や裏のカットの写しも、この値からの伸びを尺度にする (再スタートの後の根の LP の値を基準にすると、伸びが小さい分だけ
/// 少しの伸びでも停滞と数えなくなる。neos-3083819-nubu で再スタートの後の根が 25 ラウンドを延長して回った)。値がなければ NaN
pub(super) static CUTSTALL_BASE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);

pub(super) fn cutstall_base() -> f64 {
    f64::from_bits(CUTSTALL_BASE.load(std::sync::atomic::Ordering::Relaxed))
}

pub(super) fn cutstall_gap() -> Option<(f64, bool)> {
    env_str!("ENOMOTO_MIP_CUTSTALL_GAP").map(|v| {
        let ext = !v.ends_with('n');
        let t = v.trim_end_matches('n').parse::<f64>().ok().filter(|t| *t > 0.0 && *t < 1.0).unwrap_or(1e-3);
        (t, ext)
    })
}

/// `ENOMOTO_MIP_HO6_STALL` (試作、既定では使わない): 根のカットのループの停滞を、カットで上げた分に対する割合で判定する
/// (根のカットのループ `root_cut_loop` の停滞の判定)。値が数なら割合、ほかの値なら 1e-3。あわせて、ラウンドの上限に当たったときの延長
/// (`ENOMOTO_MIP_CUT_ADAPT_ROUNDS` と同じ、上限 `ENOMOTO_T_MIP_CUT_ADAPT_ROUNDS_CAP`) を、直近 5 ラウンドの伸びが今までの伸びの
/// `ENOMOTO_T_MIP_HO6_STALL_EXT` (既定 0.01) 以上なら行う
pub(super) fn ho6_stall_frac() -> Option<f64> {
    env_str!("ENOMOTO_MIP_HO6_STALL").map(|v| v.parse::<f64>().ok().filter(|t| *t > 0.0 && *t < 1.0).unwrap_or(1e-3))
}

fn use_lifted_cover() -> bool {
    env_str!("ENOMOTO_MIP_LIFTED_COVER").is_some()
}

fn use_flow_cover() -> bool {
    env_str!("ENOMOTO_MIP_NO_FLOWCOVER").is_none()
}

/// 旧来の生成 (CMIR・flow cover・cover を別々に作って全て候補に入れる) に戻す (`ENOMOTO_MIP_GEN_OLD`)。
/// 既定は HiGHS と同じく、集約行 1 本 (の向き 1 つ) ごとに `generate_cuts` の候補から効き目の最も大きい 1 本だけ残す。
fn gen_old() -> bool {
    env_str!("ENOMOTO_MIP_GEN_OLD").is_some()
}

/// カットの質 (Wesselmann・Suhl "Implementing cutting plane management and selection techniques" (2012) で最も良かった
/// 重み付き和): 距離 (効き目) + 0.1 × 目的関数との平行度 + 0.1 × 整数台の割合 (カットの非零のうち整数列の割合)。
/// カット選択 (`select_cuts_hybrid`) と、集約行 1 本から作った候補のうち 1 本を残すとき (`keep_best`) に使う。
fn cut_quality(c: &Candidate, p: &MipProblem, cnorm: f64) -> f64 {
    let nc = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
    let objpar = if cnorm > 0.0 && nc > 0.0 { c.coefs.iter().map(|&(j, v)| v * p.cost[j]).sum::<f64>().abs() / (cnorm * nc) } else { 0.0 };
    let intsup = c.coefs.iter().filter(|&&(j, _)| p.is_int[j]).count() as f64 / c.coefs.len().max(1) as f64;
    c.efficacy + 0.1 * objpar + 0.1 * intsup
}

/// `cands[n0..]` のうち質 ([`cut_quality`]、`ENOMOTO_MIP_KEEP_BEST_EFF` なら効き目だけ) の最も大きい 1 本だけ残す。
fn keep_best(cands: &mut Vec<Candidate>, n0: usize, p: &MipProblem) {
    if cands.len() <= n0 + 1 {
        return;
    }
    let eff_only = env_str!("ENOMOTO_MIP_KEEP_BEST_EFF").is_some();
    let cnorm = p.cost.iter().map(|c| c * c).sum::<f64>().sqrt();
    let q = |c: &Candidate| if eff_only { c.efficacy } else { cut_quality(c, p, cnorm) };
    let mut bi = n0;
    let mut bq = q(&cands[n0]);
    for i in n0 + 1..cands.len() {
        let qi = q(&cands[i]);
        if qi > bq {
            bi = i;
            bq = qi;
        }
    }
    cands.swap(n0, bi);
    cands.truncate(n0 + 1);
}

/// `ENOMOTO_MIP_CUTSEL_PROBE_KEEP` の比べの片側の終わりの状態 (足した行・LP の状態・分離器の状態・回ったラウンド数)
/// 試作 `ENOMOTO_MIP_OPTFACE_PARCUTS` の `k`: 勝ち残りの取り決めに、まだ届けていなければループを抜けたときの下界を届ける
struct RaceReport {
    race: Option<(std::sync::Arc<super::solver::CutRace>, usize)>,
    obj: f64,
}

impl Drop for RaceReport {
    fn drop(&mut self) {
        if let Some((r, id)) = self.race.take() {
            r.report(id, self.obj);
        }
    }
}

pub(super) struct ProbeEnd<S> {
    pub(super) rows: Vec<(Vec<(usize, f64)>, f64, f64)>,
    lp: S,
    rng: u64,
    tab: (std::collections::HashMap<usize, u64>, u8, (u64, u64), u64, u64),
    face: (f64, usize, bool, f64, f64),
    sel: (f64, f64),
    iters: u64,
    rounds: usize,
    /// 終わりの LP 解の分数の列の数 (試作 `ENOMOTO_MIP_CLIQUE_CUTS=probe` の判定に使う)
    nfrac: usize,
}

thread_local! {
    /// 診断 `ENOMOTO_MIP_INTVUB_CHECK` の数 (デバッグ解を切らない, 切る (デバッグ解が大域的な境界の中), 切る (境界の外))
    static INTVUB_CHECK: std::cell::RefCell<[u64; 3]> = const { std::cell::RefCell::new([0; 3]) };
}

#[derive(Clone)]
struct Candidate {
    coefs: Vec<(usize, f64)>,
    rhs: f64,
    efficacy: f64,
    /// 根の選択を通さずに LP に入れる (implied bound カット)。
    forced: bool,
    /// どの分離器から来たか (診断 `ENOMOTO_MIP_DEBUG_SEPPROF` の集計用。[`sepprof`] の `SRC_*`、面の頂点なら + `SRC_FACE`)
    src: u8,
}

/// 診断 (`ENOMOTO_MIP_DEBUG_SEPPROF`): 根のカットのループの 1 ラウンドの時間を段階ごとに集計する (スレッドごと)。
/// 根のループ (裏のループ・サブ MIP を除く) の終わりに、段階ごとの合計とラウンドあたりの時間、分離器ごとの候補と採った本数を出す
/// `ENOMOTO_MIP_CUTSEL_PROBE` の根の比べの結果 (0 = まだ、1 = 既定、2 = POOL)。外側の最初の根で決め、背景のカットの求解 (別のスレッド) と
/// 再スタートの後の根も同じ値を見る
pub(super) static CUT_PROBE_CHOICE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub(super) mod sepprof {
    use std::cell::RefCell;
    pub const SRC_EXTRA: u8 = 0;
    pub const SRC_ROWS: u8 = 1;
    pub const SRC_ZH: u8 = 2;
    pub const SRC_CLIQUE: u8 = 3;
    pub const SRC_IMPLBD: u8 = 4;
    pub const SRC_PATHH: u8 = 5;
    pub const SRC_PATH: u8 = 6;
    pub const SRC_TAB: u8 = 7;
    pub const SRC_POOL: u8 = 8;
    pub const SRC_MCF: u8 = 9;
    /// 試作 `ENOMOTO_MIP_MODKVB`・`ENOMOTO_MIP_MUSHAGG` の候補 (以前は `SRC_ROWS` に数えていた)
    pub const SRC_MODK: u8 = 10;
    pub const NSRC: usize = 11;
    pub const SRC_FACE: u8 = 16;
    pub const SRC_NAMES: [&str; NSRC] = ["extra", "rows", "zerohalf", "clique", "implbd", "path_highs", "path", "tableau", "pool", "mcf", "modk"];
    /// 段階 (時間)。separate() の中の段階は今の頂点 (0..) と面の頂点 (+ NSEP) を分ける
    pub const T_SETUP: usize = 0;
    pub const T_ROWS: usize = 1;
    pub const T_ZH: usize = 2;
    pub const T_CLIQUE: usize = 3;
    pub const T_IMPLBD: usize = 4;
    pub const T_PATHH: usize = 5;
    pub const T_PATH: usize = 6;
    pub const T_TABSEL: usize = 7;
    pub const T_TABAGG: usize = 8;
    pub const T_TABGEN: usize = 9;
    pub const T_MCF: usize = 10;
    pub const T_FINISH: usize = 11;
    pub const NSEP: usize = 12;
    pub const T_FACE_PREP: usize = 2 * NSEP;
    pub const T_FACE_LP: usize = 2 * NSEP + 1;
    pub const T_POOL: usize = 2 * NSEP + 2;
    pub const T_SELECT: usize = 2 * NSEP + 3;
    pub const T_ADDROWS: usize = 2 * NSEP + 4;
    pub const T_LP: usize = 2 * NSEP + 5;
    pub const T_REMOVE: usize = 2 * NSEP + 6;
    pub const T_ROUND: usize = 2 * NSEP + 7;
    pub const NT: usize = 2 * NSEP + 8;
    pub const SEP_NAMES: [&str; NSEP] = ["setup", "rows", "zerohalf", "clique", "implbd", "path_highs", "path", "tab_select", "tab_btran_agg", "tab_gen", "mcf", "finish(in)"];
    #[derive(Default)]
    pub struct Prof {
        pub t: [f64; NT],
        /// 候補と採った本数 [面か][分離器]
        pub cands: [[usize; NSRC]; 2],
        pub chosen: [[usize; NSRC]; 2],
        /// 生のカットの数 (finish_cut に渡した数) と通った数 (今の頂点・面の頂点)
        pub raw: [usize; 2],
        pub fin: [usize; 2],
        pub face_solves: usize,
        pub face_used: usize,
        pub rounds: usize,
    }
    thread_local! {
        pub static PROF: RefCell<Prof> = RefCell::new(Prof::default());
        /// 今 separate() が面の頂点で動いているか
        pub static AT_FACE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    pub fn on() -> bool {
        env_str!("ENOMOTO_MIP_DEBUG_SEPPROF").is_some()
    }
    pub fn add(k: usize, secs: f64) {
        PROF.with(|p| p.borrow_mut().t[k] += secs);
    }
    /// separate() の中の段階 `k` に、`lap` からの時間を足して `lap` を今にする
    pub fn lap(k: usize, lap: &mut std::time::Instant) {
        // `ENOMOTO_MIP_SEPSTAT`・`ENOMOTO_MIP_SEPLEARN` も分離器ごとの時間を数える ([`super::super::seplearn`])
        if on() || super::super::seplearn::stat_on() {
            let now = std::time::Instant::now();
            let f = if AT_FACE.with(|c| c.get()) { NSEP } else { 0 };
            add(k + f, (now - *lap).as_secs_f64());
            if super::super::seplearn::stat_on() {
                super::super::seplearn::add_stage_time(k, (now - *lap).as_secs_f64());
            }
            *lap = now;
        }
    }
    pub fn report(tag: &str) {
        if !on() {
            return;
        }
        let p = PROF.with(|p| std::mem::take(&mut *p.borrow_mut()));
        let r = p.rounds.max(1) as f64;
        let ms = |v: f64| 1e3 * v / r;
        let mut s = format!("SEPPROF {tag} rounds {} (ms/round): round {:.1} | face prep {:.1} face LP {:.1} ({} solves, {} used) | pool {:.1} select {:.1} add_rows {:.1} LP {:.1} remove {:.1}", p.rounds, ms(p.t[T_ROUND]), ms(p.t[T_FACE_PREP]), ms(p.t[T_FACE_LP]), p.face_solves, p.face_used, ms(p.t[T_POOL]), ms(p.t[T_SELECT]), ms(p.t[T_ADDROWS]), ms(p.t[T_LP]), ms(p.t[T_REMOVE]));
        for (f, name) in [(0usize, "cur"), (1, "face")] {
            let tot: f64 = (0..NSEP - 1).map(|k| p.t[k + f * NSEP]).sum();
            s += &format!("\nSEPPROF {tag}   sep@{name} {:.1}:", ms(tot));
            for k in 0..NSEP {
                s += &format!(" {} {:.1}", SEP_NAMES[k], ms(p.t[k + f * NSEP]));
            }
            s += &format!(" | raw {} fin {}", p.raw[f], p.fin[f]);
        }
        let (gc, gt) = super::super::cuts::GEN_PROF.with(|g| std::mem::take(&mut *g.borrow_mut()));
        s += &format!("\nSEPPROF {tag}   generate_cuts (this thread) per round: calls {:.0} mean nnz {:.0} substituted {:.0} raw {:.0} | subst {:.1} cmir {:.1} extcover {:.1} flowcover {:.1}", gc[0] as f64 / r, gc[1] as f64 / gc[0].max(1) as f64, gc[2] as f64 / r, gc[3] as f64 / r, ms(gt[0]), ms(gt[1]), ms(gt[2]), ms(gt[3]));
        s += &format!("\nSEPPROF {tag}   cands/chosen per round:");
        for i in 0..NSRC {
            let (c0, c1, h0, h1) = (p.cands[0][i], p.cands[1][i], p.chosen[0][i], p.chosen[1][i]);
            if c0 + c1 > 0 {
                s += &format!(" {} {:.1}/{:.1} (face {:.1}/{:.1})", SRC_NAMES[i], c0 as f64 / r, h0 as f64 / r, c1 as f64 / r, h1 as f64 / r);
            }
        }
        eprintln!("{s}");
    }
}

/// `ENOMOTO_MIP_SEP_ADAPTIVE` (試作、既定では使わない): 分離器ごとの適応的な飛ばし (CBC の `CbcModel::serialCuts` と
/// `CbcCutAdaptiveSkipSettings` (CbcModel.hpp) の移植)。根で、3 ラウンド以上試して 3 回続けて外れた (その分離器の候補が
/// 1 本も選択に残らなかった) 分離器は 5 ラウンド飛ばし、飛ばした後にまた外れたら飛ばす長さを倍にする (最大 20)。採られたら
/// 元に戻す。tableau 行 (Gomory) の分離器には使わない (選択の質の閾値で落ちるラウンドがあっても効く。
/// analysis/cutloop_hl_20261022.md §1)。また根で 1 本も採られなかった分離器はノードの分離で使わない (`node_off`)。
/// 列が 500 本未満の問題では使わない
#[derive(Default, Clone)]
pub(super) struct SepAdapt {
    /// 分離器 ([`sepprof`] の `SRC_*`) ごとの [試したラウンド数, 続けて外れた回数, 残りの飛ばすラウンド数, 今の飛ばす長さ, 根で採られた本数の合計]
    st: [[u32; 5]; sepprof::NSRC],
    /// このラウンドに動いた分離器 (ビット)
    ran: u16,
    /// 根で一度は試して 1 本も採られなかったので、ノードの分離で使わない分離器 (ビット。根のループの終わりに決め、写しで木の
    /// 探索のスレッドへ引き継ぐ)
    pub(super) node_off: u16,
}

impl SepAdapt {
    /// ノードで使わない分離器 `node_off` (写しから引き継いだもの) だけを持つ初めの状態
    pub(super) fn with_node_off(node_off: u16) -> Self {
        SepAdapt { node_off, ..Default::default() }
    }
}

/// 適応的な飛ばしを使う分離器 (tableau 行・一時的な行・プールは使わない)
const SEP_ADAPT_SRCS: [u8; 7] = [sepprof::SRC_ROWS, sepprof::SRC_ZH, sepprof::SRC_CLIQUE, sepprof::SRC_IMPLBD, sepprof::SRC_PATHH, sepprof::SRC_PATH, sepprof::SRC_MCF];

/// `ENOMOTO_MIP_SEP_NO_ROWCLONE` (試作、既定では使わない): [`Solver::separate`] で使う LP の行の写しと、行ごとの「整数の行か」
/// (係数と列がすべて整数) を、LP の行列の識別子 ([`MipLp::matrix_key`]) が同じ間は使い回す (毎回すべての行を複製し、整数の行かを
/// 数え直していた)。整数の行かは `cut_int` にもよるので、それが同じときだけ使う
pub(super) struct SepRowsCache {
    key: u64,
    rows: std::rc::Rc<Vec<Vec<(usize, f64)>>>,
    cut_int: Vec<bool>,
    integral: std::rc::Rc<Vec<bool>>,
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// `ENOMOTO_MIP_SEP_ADAPTIVE` を使うか (根の外側の求解・背景のカットの求解。サブ MIP と 500 列未満の問題では使わない)
    fn sep_adapt_on(&self) -> bool {
        env_str!("ENOMOTO_MIP_SEP_ADAPTIVE").is_some() && !self.params.submip && self.p.n >= 500
    }

    /// 分離器 `src` をこの呼び出しで使うか (`ENOMOTO_MIP_SEP_ADAPTIVE`。根では飛ばしている間は使わず、ノード (`light`) では
    /// `node_off` なら使わない)。使うなら、このラウンドに動いた印を付ける
    fn sep_src_on(&mut self, src: u8, light: bool) -> bool {
        // 試作 `ENOMOTO_MIP_SEPLEARN`: この呼び出しで使わないと決めた分離器 ([`super::seplearn::SepLearn::begin_call`])
        if self.sl.learn && self.sl.off & (1 << src) != 0 {
            return false;
        }
        if self.sep_src_skipped(src, light) {
            return false;
        }
        if !light && self.sep_adapt_on() && SEP_ADAPT_SRCS.contains(&src) {
            self.sep_adapt.ran |= 1 << src;
        }
        true
    }

    /// [`Self::sep_src_on`] の判定だけ (印を付けない)。飛ばしているなら真
    fn sep_src_skipped(&self, src: u8, light: bool) -> bool {
        if !self.sep_adapt_on() || !SEP_ADAPT_SRCS.contains(&src) {
            return false;
        }
        let k = src as usize;
        if light { self.sep_adapt.node_off & (1 << k) != 0 } else { self.sep_adapt.st[k][2] > 0 }
    }

    /// 根のラウンドの選択の後に、分離器ごとの採られた本数 `acc` で飛ばしの状態を進める (`ENOMOTO_MIP_SEP_ADAPTIVE`)
    fn sep_adapt_update(&mut self, acc: &[usize; sepprof::NSRC], round: usize) {
        if !self.sep_adapt_on() {
            return;
        }
        let ran = std::mem::take(&mut self.sep_adapt.ran);
        for &s in &SEP_ADAPT_SRCS {
            let k = s as usize;
            let e = &mut self.sep_adapt.st[k];
            if ran & (1 << k) != 0 {
                e[0] += 1;
                e[4] += acc[k] as u32;
                if acc[k] > 0 {
                    e[1] = 0;
                    e[3] = 0;
                } else {
                    e[1] += 1;
                    if e[0] >= 3 && e[1] >= 3 {
                        e[3] = if e[3] == 0 { 5 } else { (2 * e[3]).min(20) };
                        e[2] = e[3];
                        if self.params.verbose {
                            eprintln!("MIP: cut round {round}: separator {} missed {} rounds in a row; skipped for {} rounds", sepprof::SRC_NAMES[k], e[1], e[3]);
                        }
                    }
                }
            } else if e[2] > 0 {
                e[2] -= 1;
            }
        }
    }

    /// 根のカットのループの終わりに、一度は試して 1 本も採られなかった分離器をノードで使わないことにする (`ENOMOTO_MIP_SEP_ADAPTIVE`)
    fn sep_adapt_finish_root(&mut self) {
        if !self.sep_adapt_on() {
            return;
        }
        let mut off = 0u16;
        for &s in &SEP_ADAPT_SRCS {
            let e = &self.sep_adapt.st[s as usize];
            if e[0] > 0 && e[4] == 0 {
                off |= 1 << s;
            }
        }
        self.sep_adapt.node_off = off;
        if self.params.verbose {
            let names: Vec<&str> = SEP_ADAPT_SRCS.iter().filter(|&&s| off & (1 << s) != 0).map(|&s| sepprof::SRC_NAMES[s as usize]).collect();
            let stats: Vec<String> = SEP_ADAPT_SRCS.iter().map(|&s| format!("{} {}/{}", sepprof::SRC_NAMES[s as usize], self.sep_adapt.st[s as usize][4], self.sep_adapt.st[s as usize][0])).collect();
            eprintln!("MIP: adaptive separators: accepted/tried rounds at the root: {}; not used at nodes: {:?}", stats.join(", "), names);
        }
    }

    /// `ENOMOTO_MIP_SEPSTAT`・`ENOMOTO_MIP_SEPLEARN`: 分離の後の候補を数え、学習なら分離器ごとの候補の上限をかける ([`super::seplearn`])
    fn sl_after_separate(&mut self, lists: &mut [&mut Vec<Candidate>], budget: usize) {
        let mut srcs: Vec<Vec<(u8, f64, usize)>> = lists.iter().map(|v| v.iter().map(|c| (c.src, c.efficacy, c.coefs.len())).collect()).collect();
        let keep = self.sl.after_separate(&mut srcs, budget);
        for (v, k) in lists.iter_mut().zip(keep) {
            if k.iter().any(|b| !b) {
                let mut it = k.into_iter();
                v.retain(|_| it.next().unwrap_or(true));
            }
        }
    }

    /// 選んだカットを LP に加える直前に、加える前の目的値と分離器ごとの違反を覚える ([`super::seplearn::SepLearn::prepare`])
    fn sl_prepare(&mut self, chosen: &[Candidate], x: &[f64]) {
        let obj0 = self.lp.objective() + self.p.offset;
        let items: Vec<(u8, f64)> = chosen
            .iter()
            .map(|c| {
                let act: f64 = c.coefs.iter().map(|&(j, a)| a * x[j]).sum();
                (c.src, act - c.rhs)
            })
            .collect();
        for c in chosen {
            self.sl.add_chosen_nnz(c.src, c.coefs.len());
        }
        self.sl.prepare(obj0, items);
    }

    /// LP を解いた後に、加えたカットの伸びと生き残りを数える ([`super::seplearn::SepLearn::after_lp`])
    pub(super) fn sl_after_lp(&mut self, st: LpStatus) {
        let obj = (st == LpStatus::Optimal).then(|| self.lp.objective() + self.p.offset);
        let bound = matches!(st, LpStatus::Infeasible | LpStatus::ObjectiveBound);
        let lim = self.prune_limit();
        self.sync_cut_age();
        let m0 = self.p.m;
        let cut_ids = &self.cut_ids;
        let lp = &self.lp;
        let mut duals: Option<Vec<f64>> = None;
        let mut pos: Option<std::collections::HashMap<u64, usize>> = None;
        let mut f = |q: &[u64]| -> Vec<Option<f64>> {
            let pos = pos.get_or_insert_with(|| cut_ids.iter().enumerate().map(|(i, &id)| (id, i)).collect());
            let y = duals.get_or_insert_with(|| lp.row_duals());
            q.iter().map(|id| pos.get(id).and_then(|&i| y.get(m0 + i)).map(|v| v.abs())).collect()
        };
        self.sl.after_lp(obj, bound, lim, &mut f);
    }

    /// HiGHS のカット選択 (`HighsCutPool::separate`): スコア = 違反量 / (効いている非零の数 x 効いている列だけのノルム)
    /// (効いている = LP 解が境界から離れている列)。これまでに見た最良のスコアの `min_factor` 倍以上を残し
    /// (残りが少なすぎれば上位半分、全部残れば係数を下げる)、採ったカットとの平行度が 0.1 を超えるものは捨てる。
    fn select_cuts_highs(&mut self, cands: Vec<Candidate>, max_cuts: usize, x: &[f64], maxpar: f64) -> Vec<Candidate> {
        let tol = FEASTOL;
        let mut sc: Vec<(f64, Candidate)> = Vec::new();
        for c in cands {
            let act: f64 = c.coefs.iter().map(|&(j, a)| a * x[j]).sum();
            let viol = act - c.rhs;
            if viol <= tol {
                continue;
            }
            let mut norm = 0.0;
            let mut nact = 0usize;
            for &(j, a) in &c.coefs {
                let active = if a > 0.0 { x[j] > self.dom.global_lo[j] + tol } else { x[j] < self.dom.global_up[j] - tol };
                if active {
                    norm += a * a;
                    nact += 1;
                }
            }
            if nact == 0 {
                continue;
            }
            sc.push((viol / (nact as f64 * norm.sqrt()), c));
        }
        if sc.is_empty() {
            return Vec::new();
        }
        sc.sort_by(|a, b| b.0.total_cmp(&a.0));
        self.cutsel_best = self.cutsel_best.max(sc[0].0);
        let min_score = self.cutsel_factor * self.cutsel_best;
        let mut keep = sc.partition_point(|s| s.0 >= min_score);
        let lower = sc.len() / 20;
        let upper = sc.len() - 1;
        if keep <= lower {
            keep = (sc.len() / 2).max(1);
            self.cutsel_factor = sc[keep - 1].0 / self.cutsel_best;
        } else if keep > upper {
            self.cutsel_factor = sc[upper].0 / self.cutsel_best;
        }
        sc.truncate(keep);
        let mut chosen: Vec<(f64, Candidate)> = Vec::new();
        for (_, c) in sc {
            if chosen.len() >= max_cuts {
                break;
            }
            let nc = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
            if chosen.iter().any(|(nd, d)| sparse_dot(&c.coefs, &d.coefs).abs() > maxpar * nc * nd) {
                continue;
            }
            chosen.push((nc, c));
        }
        chosen.into_iter().map(|(_, c)| c).collect()
    }

    /// `ENOMOTO_MIP_CUT_CLIQUE_EXTEND` (既定 1、`0` で外す。計測 23 で既定化): 根のカットのループで LP に入れるカットのうち集合詰め込みの形
    /// (0-1 列だけ、係数の絶対値がそろい、文字の和 <= 1) のものを、衝突グラフ ([`Self::clique_graph_get`]、元の行 + プロービングの含意)
    /// と前のラウンドまでに入れた同じ形のカットの文字の組で、極大なクリークまで延ばす (前処理のクリーク行の拡張
    /// [`super::extend_clique_rows`] と同じ規則。足す文字は LP 解の値の大きい順)。physiciansched6-2 では根のカットの大半が非零 2 の
    /// 衝突のカットで、根のループは 49224 で止まるが、再スタートの前処理がそのカットの行を延ばすと (834 行に 1488 非零) 根の LP が
    /// そのまま 49324 (最適値) になる。延ばした後で同じになったカットは 1 本にする
    fn extend_clique_cuts(&mut self, chosen: &mut Vec<Candidate>, x: &[f64], round: usize) {
        let p = self.p;
        // 集合詰め込みの形のカットがなければ何もしない (衝突グラフを作らない。作ると後の clique カットの判定が使うグラフが変わり、
        // drayage-25-23 では延ばすカットが 0 本なのに再スタートが 1 回増えて 10 -> 22 秒になっていた)
        let any_sp = {
            let is_bin = |j: usize| p.is_int[j] && self.dom.global_lo[j] == 0.0 && self.dom.global_up[j] == 1.0;
            let sp_form = |c: &Candidate| {
                let s = c.coefs.first().map_or(0.0, |e| e.1.abs());
                c.coefs.len() >= 2
                    && s > 0.0
                    && c.coefs.iter().all(|&(j, a)| is_bin(j) && (a.abs() - s).abs() <= 1e-9 * s)
                    && ((c.rhs / s + c.coefs.iter().filter(|&&(_, a)| a < 0.0).count() as f64) - 1.0).abs() <= 1e-9
            };
            chosen.iter().filter(|c| sp_form(c)).count()
        };
        let (any_sp, any_sp_n) = (any_sp > 0, any_sp);
        // `ENOMOTO_T_MIP_CUT_CLIQUE_EXTEND_MIN_FRAC` = f (既定 0.5、計測 23 で既定化): 集合詰め込みの形のカットが選んだカットの f 未満のラウンドでは延ばさない
        // (physiciansched6-2 はほぼ全部 (44/44・266/314)。drayage-25-23 は 100 本中 1〜21 本で、延ばすと再スタートの経路が変わり 10 -> 14〜22 秒)
        let min_frac = tunable!("ENOMOTO_T_MIP_CUT_CLIQUE_EXTEND_MIN_FRAC", 0.5, f64);
        if !any_sp || (min_frac > 0.0 && (any_sp_n as f64) < min_frac * chosen.len() as f64) {
            return;
        }
        let g = self.clique_graph_get();
        let max_add = tunable!("ENOMOTO_T_MIP_CUT_CLIQUE_EXTEND_MAX", 1000usize, usize);
        let is_bin = |j: usize| p.is_int[j] && self.dom.global_lo[j] == 0.0 && self.dom.global_up[j] == 1.0;
        let val = |l: u32| -> f64 {
            let j = (l / 2) as usize;
            if l % 2 == 0 { x[j] } else { 1.0 - x[j] }
        };
        let (mut ext, mut added, mut sp) = (0usize, 0usize, 0usize);
        let mut seen: std::collections::HashSet<Vec<u32>> = std::collections::HashSet::new();
        let mut keep = vec![true; chosen.len()];
        for (ci, c) in chosen.iter_mut().enumerate() {
            if c.coefs.len() < 2 {
                continue;
            }
            let s = c.coefs[0].1.abs();
            if !(s > 0.0) || !c.coefs.iter().all(|&(j, a)| is_bin(j) && ((a.abs() - s).abs() <= 1e-9 * s)) {
                continue;
            }
            let nneg = c.coefs.iter().filter(|&&(_, a)| a < 0.0).count() as f64;
            if ((c.rhs / s + nneg) - 1.0).abs() > 1e-9 {
                continue;
            }
            sp += 1;
            let mut lits: Vec<u32> = c.coefs.iter().map(|&(j, a)| if a > 0.0 { 2 * j as u32 } else { 2 * j as u32 + 1 }).collect();
            let adj = |a: u32, b: u32| g.adjacent(a, b) || self.cut_clique_adj.get(&a).is_some_and(|v| v.contains(&b));
            // 候補: 隣接の最も少ない文字の隣接 (グラフ + 前のカット)
            let &l0 = lits.iter().min_by_key(|&&l| g.neighbors(l).len() + self.cut_clique_adj.get(&l).map_or(0, |v| v.len())).unwrap();
            let in_cut: std::collections::HashSet<usize> = c.coefs.iter().map(|&(j, _)| j).collect();
            let mut cands: Vec<u32> = g.neighbors(l0).iter().copied().chain(self.cut_clique_adj.get(&l0).into_iter().flatten().copied()).filter(|&l| {
                let j = (l / 2) as usize;
                !in_cut.contains(&j) && is_bin(j)
            }).collect();
            cands.sort_unstable();
            cands.dedup();
            cands.sort_by(|&a, &b| val(b).total_cmp(&val(a)).then(a.cmp(&b)));
            let n0 = lits.len();
            for l in cands {
                if lits.len() - n0 >= max_add {
                    break;
                }
                if lits.iter().any(|&m| m / 2 == l / 2) {
                    continue;
                }
                if lits.iter().all(|&m| adj(m, l)) {
                    lits.push(l);
                }
            }
            let mut key = lits.clone();
            key.sort_unstable();
            if !seen.insert(key) {
                keep[ci] = false;
                continue;
            }
            // カットの組を前のカットの辺として覚える (次のラウンドから使う)
            if lits.len() > n0 {
                ext += 1;
                added += lits.len() - n0;
                let mut coefs: Vec<(usize, f64)> = Vec::with_capacity(lits.len());
                let mut rhs = 1.0;
                for &l in &lits {
                    let j = (l / 2) as usize;
                    if l % 2 == 0 {
                        coefs.push((j, 1.0));
                    } else {
                        coefs.push((j, -1.0));
                        rhs -= 1.0;
                    }
                }
                coefs.sort_unstable_by_key(|e| e.0);
                c.coefs = coefs;
                c.rhs = rhs;
            }
            if lits.len() <= 64 {
                for a in 0..lits.len() {
                    for b in 0..lits.len() {
                        if a != b && !g.adjacent(lits[a], lits[b]) {
                            let v = self.cut_clique_adj.entry(lits[a]).or_default();
                            if !v.contains(&lits[b]) {
                                v.push(lits[b]);
                            }
                        }
                    }
                }
            }
        }
        let mut k = 0;
        chosen.retain(|_| {
            k += 1;
            keep[k - 1]
        });
        if self.params.verbose {
            eprintln!("MIP: cut round {round}: clique extension of cuts: {sp} set packing cuts, {ext} extended by {added} literals ({} literals with cut edges)", self.cut_clique_adj.len());
        }
    }

    /// clique カット用の衝突グラフ (根で一度だけ作って使い回す)。
    fn clique_graph_get(&mut self) -> std::rc::Rc<super::clique::CliqueGraph> {
        let p = self.p;
        if self.clique_graph.is_none() {
            let binary: Vec<bool> = (0..p.n).map(|j| p.is_int[j] && self.dom.global_lo[j] >= 0.0 && self.dom.global_up[j] <= 1.0).collect();
            let g = super::clique::CliqueGraph::build_with(p.n, &p.rows, &p.row_lo, &p.row_up, &binary, &self.dom.global_lo, &self.dom.global_up, &p.conflict_edges);
            if self.params.verbose && !self.params.submip {
                let g0 = super::clique::CliqueGraph::build(p.n, &p.rows, &p.row_lo, &p.row_up, &binary, &self.dom.global_lo, &self.dom.global_up);
                eprintln!("MIP: clique graph: {} edges ({} from rows, probing implications {})", g.num_edges(), g0.num_edges(), p.conflict_edges.len());
            }
            self.clique_graph = Some(std::rc::Rc::new(g));
        }
        self.clique_graph.clone().unwrap()
    }

    /// `ENOMOTO_MIP_CLIQUE_CUTS=auto` の判定 (試作): 根の LP 解で破れる clique カットだけを LP に入れて解き直し、目的値の伸びが
    /// `ENOMOTO_T_MIP_CLIQUE_AUTO_GAIN` (既定 1e-3) x max(1, |目的値|) を超えれば使う。試した行は外し、LP の状態を戻す。
    /// drayage-25-23 では 99718.79 -> 100121.99 (4e-3、HiGHS の根の下界と同じ)。clique カットを常に使うと misc07 の根は
    /// 1535.54 -> 1536.21 しか上がらないのに木が変わって 7 秒 -> 10 秒に遅くなる
    fn clique_trial(&mut self, root_iters: u64) -> bool {
        let t0 = std::time::Instant::now();
        let Some((obj0, obj1, nfrac0, nfrac1, ncuts)) = self.clique_trial_lp(root_iters) else {
            if self.params.verbose {
                eprintln!("MIP: clique trial: no violated clique cuts; not used ({:.2}s)", t0.elapsed().as_secs_f64());
            }
            return false;
        };
        let thr = tunable!("ENOMOTO_T_MIP_CLIQUE_AUTO_GAIN", 1e-3, f64) * obj0.abs().max(1.0);
        // `ENOMOTO_T_MIP_CLIQUE_AUTO_FRAC` = r (> 0): 分数の列の数が r 倍以下に減るときも使う (目的値が既に最適値で動かない
        // drayage-100-23 でも、clique カットで LP 解が整数に近づき根の RENS が最適解を見つける)
        let frac_r = tunable!("ENOMOTO_T_MIP_CLIQUE_AUTO_FRAC", 0.0, f64);
        let on = obj1 - obj0 > thr || (frac_r > 0.0 && (nfrac1 as f64) <= frac_r * nfrac0 as f64);
        if self.params.verbose {
            eprintln!("MIP: clique trial: {ncuts} cuts, LP {obj0:.10e} -> {obj1:.10e}, fractional {nfrac0} -> {nfrac1}; {} ({:.2}s)", if on { "used" } else { "not used" }, t0.elapsed().as_secs_f64());
        }
        on
    }

    /// 今の LP 解で破れる clique カットだけを LP に入れて解き直し、(目的値, 入れた後の目的値, 分数の列の数, 入れた後の数, カットの数)
    /// を返す (破れるカットがなければ `None`)。試した行は外し、LP の状態を戻す
    fn clique_trial_lp(&mut self, root_iters: u64) -> Option<(f64, f64, usize, usize, usize)> {
        let g = self.clique_graph_get();
        let x = self.lp.col_values();
        let cuts = g.separate(&x, tunable!("ENOMOTO_T_MIP_CLIQUE_MAX", 100usize, usize));
        if cuts.is_empty() {
            return None;
        }
        let obj0 = self.lp.objective();
        let nfrac0 = self.fractional(&x).len();
        let saved = self.lp.save_state();
        let m0 = self.lp.num_rows();
        let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = cuts.iter().map(|c| (c.coefs.clone(), f64::NEG_INFINITY, c.rhs)).collect();
        self.lp.add_rows(&rows);
        // `ENOMOTO_MIP_TREE3_CLIQUECAP` (試作、既定では使わない。analysis/tree_close3_20261104.md): 試しの LP に時間の上限
        // max(`ENOMOTO_T_MIP_TREE3_CLIQUECAP_MIN` (1 秒), `ENOMOTO_T_MIP_TREE3_CLIQUECAP_MULT` (2) x 根の LP の秒) を置く (超えたら使わない)。
        // neos-3216931-puriri ではこの LP 1 回が 285〜388 秒 (根の LP は 11.7 秒) かかり、伸びは 0 で使われなかった
        let deadline = if env_str!("ENOMOTO_MIP_TREE3_CLIQUECAP").is_some_and(|v| v != "0") {
            let secs = tunable!("ENOMOTO_T_MIP_TREE3_CLIQUECAP_MIN", 1.0, f64).max(tunable!("ENOMOTO_T_MIP_TREE3_CLIQUECAP_MULT", 2.0, f64) * self.root_lp_secs);
            let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(secs.min(1e6));
            Some(self.deadline.map_or(d, |g| g.min(d)))
        } else {
            self.deadline
        };
        let st = self.lp.solve(&SolveLimits { iteration_limit: 10 * root_iters.max(100) + 10_000, cutoff: f64::INFINITY, deadline });
        if self.params.verbose && st != LpStatus::Optimal && deadline != self.deadline {
            eprintln!("MIP: clique fallback: the trial LP stopped ({st:?}, ENOMOTO_MIP_TREE3_CLIQUECAP)");
        }
        let obj1 = if st == LpStatus::Optimal { self.lp.objective() } else { obj0 };
        let nfrac1 = if st == LpStatus::Optimal { self.fractional(&self.lp.col_values()).len() } else { nfrac0 };
        let mut remove = vec![false; self.lp.num_rows()];
        for r in remove.iter_mut().skip(m0) {
            *r = true;
        }
        self.lp.delete_rows(&remove);
        self.lp.restore_state(&saved);
        Some((obj0, obj1, nfrac0, nfrac1, cuts.len()))
    }

    /// 試作 `ENOMOTO_MIP_CLIQUE_CUTS=probe` の判定 (課題 CLIQFB、analysis/cliqfb_20261111.md): 根のカットのループの 0 ラウンド目の始めで、
    /// クリークカットなし・ありのそれぞれで `ENOMOTO_T_MIP_CLIQUE_PROBE_ROUNDS` (5) ラウンドずつ回し ([`Self::probe_cut_mode`]、選び方は
    /// カットの選び方の比べの結果に従う。目的値が動かないラウンドでも止めない)、下界の伸び (g0, g1) と時間 (t0, t1) を比べる。使うのは
    /// g1 - g0 > `ENOMOTO_T_MIP_CLIQUE_PROBE_GAIN` (1e-4) x max(1, |目的値|) かつ時間あたりの伸び g1 / t1 が
    /// `ENOMOTO_T_MIP_CLIQUE_PROBE_RATE` (1.0) x g0 / t0 以上のとき (時間は下限 `ENOMOTO_T_MIP_CLIQUE_PROBE_TMIN` (0.5 秒) で測る)。どちらの伸びも 1e-9 x max(1, |目的値|) 以下なら、終わりの LP 解の分数の列の数が
    /// `ENOMOTO_T_MIP_CLIQUE_PROBE_FRAC` (0.9、0 で使わない) 倍以下に減るときも使う (今の fallback の drayage-100-23 の規則と同じ考え)。
    /// 比べの時間は全体の `ENOMOTO_T_MIP_CLIQUE_PROBE_TIME_FRAC` (0.05) 倍まで。勝った側の終わりの状態 (足したカット・LP の状態) に戻し、
    /// 負けた側のカットはプールに入れる。(使うか, 勝った側で回ったラウンド数) を返し、`self.clique_auto` を決める
    fn clique_probe(&mut self) -> (bool, usize) {
        let t_all = std::time::Instant::now();
        let k = tunable!("ENOMOTO_T_MIP_CLIQUE_PROBE_ROUNDS", 5usize, usize).max(1);
        let tfrac = tunable!("ENOMOTO_T_MIP_CLIQUE_PROBE_TIME_FRAC", 0.05, f64);
        let budget = if tfrac > 0.0 && self.params.time_limit.is_finite() { tfrac * self.params.time_limit } else { f64::INFINITY };
        let deadline = t_all + std::time::Duration::from_secs_f64(budget.min(1e6));
        let pool = self.probe_pool_on();
        let never = |_: usize, _: f64, _: f64| false;
        let obj = self.lp.objective();
        let scale = obj.abs().max(1.0);
        let nfrac_start = self.fractional(&self.lp.col_values()).len();
        // `probe_s` (試作 `probe` の余分を減らす形): (1) 始めの LP 解で破れるクリークカットがなければ比べない (使わない)。(2) 「あり」の側を
        // 先に回し、選ばれたクリークカットが 0 本なら「なし」の側は同じ道筋になるので回さない (その終わりの状態から使わずに続ける)。
        // 58 問の根だけの回では、既定でクリークカットの破れが 0 本の 42 問でも比べを回し、nw04 5.8 秒・mzzv42z 4.7 秒を使っていた
        let skip = matches!(clique_cuts_mode(), Some("probe_s") | Some("probe_r"));
        // `probe_r` (`probe_s` に足す形): 使わないと決めたら、どちらの側の状態も捨てて比べの前の状態のまま続け、負けた側のカットもプールに
        // 入れない (既定のループと同じ道筋にする。比べの時間は呼び出し側で時間の上限の起点をずらす)。roll3000 は `probe_s` で使わない判定なのに、
        // 比べの「なし」の側の状態とプールに入れた「あり」の側のカットで道筋が変わり、種 1〜3 とも 23〜33 秒 -> 49〜60 秒に遅くなった
        let rest = clique_cuts_mode() == Some("probe_r");
        // `probe_r`: 使わないと決めたら、比べの中で作った衝突グラフも捨てる (グラフは既定でもカットのクリーク拡張 `extend_clique_cuts` が
        // 初めて要るときに、その時の大域的な境界で作る。比べで早く作ると拡張が変わり道筋が変わる。neos859080 は `probe_s` で破れなしの判定なのに
        // 根の下界が 1.87 -> 2 に変わった)
        let graph_was = self.clique_graph.is_some();
        if skip && self.clique_graph_get().separate(&self.lp.col_values(), 1).is_empty() {
            self.clique_auto = Some(false);
            if rest && !graph_was {
                self.clique_graph = None;
            }
            if self.params.verbose {
                eprintln!("MIP: clique probe: no violated clique cuts; not used ({:.2}s)", self.start.elapsed().as_secs_f64());
            }
            return (false, 0);
        }
        PROBE_NO_FLAT_STOP.with(|c| c.set(true));
        // `probe` (v2、分担の計測の形) は「なし」→「あり」の順
        let first0 = if skip {
            None
        } else {
            self.clique_auto = Some(false);
            Some(self.probe_cut_mode(pool, k, deadline, &never, true))
        };
        PROBE_CLIQUE_ROWS.with(|c| c.set(0));
        self.clique_auto = Some(true);
        let (h1, n1, e1) = self.probe_cut_mode(pool, k, deadline, &never, true);
        let nclq = PROBE_CLIQUE_ROWS.with(|c| c.get());
        if skip && nclq == 0 {
            PROBE_NO_FLAT_STOP.with(|c| c.set(false));
            let mut nr = 0;
            if let Some(e) = e1.filter(|_| !rest) {
                nr = e.rounds;
                self.probe_restore(e);
            }
            self.clique_auto = Some(false);
            if rest && !graph_was {
                self.clique_graph = None;
            }
            if self.params.verbose {
                eprintln!("MIP: clique probe ({k} rounds): no clique cuts chosen in {} rounds ({n1} cuts); not used (kept {nr} rounds, probe {:.2}s, {:.2}s)", h1.len(), t_all.elapsed().as_secs_f64(), self.start.elapsed().as_secs_f64());
            }
            return (false, nr);
        }
        let (h0, n0, e0) = match first0 {
            Some(v) => v,
            None => {
                self.clique_auto = Some(false);
                self.probe_cut_mode(pool, k, deadline, &never, true)
            }
        };
        PROBE_NO_FLAT_STOP.with(|c| c.set(false));
        let (g0, t0) = h0.last().copied().unwrap_or((0.0, 0.0));
        let (g1, t1) = h1.last().copied().unwrap_or((0.0, 0.0));
        let f0 = e0.as_ref().map_or(nfrac_start, |e| if e.rounds > 0 { e.nfrac } else { nfrac_start });
        let f1 = e1.as_ref().map_or(nfrac_start, |e| if e.rounds > 0 { e.nfrac } else { nfrac_start });
        // 時間の下限 `ENOMOTO_T_MIP_CLIQUE_PROBE_TMIN` (0.5 秒): 比べが短い問題では時間の差は誤差なので伸びだけで比べる
        // (rococoC10-001000: なし 0.10 秒で 15.6%、あり 0.11 秒で 17.0%)
        let tmin = tunable!("ENOMOTO_T_MIP_CLIQUE_PROBE_TMIN", 0.5, f64).max(1e-3);
        let rate = |g: f64, t: f64| g.max(0.0) / t.max(tmin);
        let gain_thr = tunable!("ENOMOTO_T_MIP_CLIQUE_PROBE_GAIN", 1e-4, f64) * scale;
        let rate_r = tunable!("ENOMOTO_T_MIP_CLIQUE_PROBE_RATE", 1.0, f64);
        let frac_r = tunable!("ENOMOTO_T_MIP_CLIQUE_PROBE_FRAC", 0.9, f64);
        let by_gain = g1 - g0 > gain_thr && rate(g1, t1) >= rate_r * rate(g0, t0);
        let flat = g0 <= 1e-9 * scale && g1 <= 1e-9 * scale;
        let by_frac = frac_r > 0.0 && flat && (f1 as f64) <= frac_r * f0 as f64 && f1 < f0;
        let on = by_gain || by_frac;
        let (win, lose) = if on { (e1, e0) } else { (e0, e1) };
        let (win, lose) = if rest && !on { (None, None) } else { (win, lose) };
        if rest && !on && !graph_was {
            self.clique_graph = None;
        }
        if let Some(e) = lose {
            for (c, _, r) in &e.rows {
                self.add_to_pool(c, *r);
            }
        }
        let mut nr = 0;
        if let Some(e) = win {
            nr = e.rounds;
            self.probe_restore(e);
        }
        self.clique_auto = Some(on);
        if self.params.verbose {
            eprintln!(
                "MIP: clique probe ({k} rounds): without {n0} cuts gain {:.3e} ({t0:.2}s, {} rounds, fractional {f0}), with {n1} cuts ({nclq} clique) gain {:.3e} ({t1:.2}s, {} rounds, fractional {f1}) relative; {} (kept {nr} rounds, LP {:.10e}, probe {:.2}s, {:.2}s)",
                g0 / scale,
                h0.len(),
                g1 / scale,
                h1.len(),
                if on { if by_gain { "used" } else { "used (fractional)" } } else { "not used" },
                self.lp.objective(),
                t_all.elapsed().as_secs_f64(),
                self.start.elapsed().as_secs_f64()
            );
        }
        (on, nr)
    }

    /// `ENOMOTO_MIP_CLIQUE_CUTS=fallback` の判定 (試作): 根の切除平面ループを clique カットなしで回し終えた LP で、破れる clique
    /// カットだけを入れた LP の伸び g_c を測り、ループ全体の伸び g_d (clique なしのカットでの伸び) と比べる。
    /// g_c > `ENOMOTO_T_MIP_CLIQUE_FB_GAIN` (既定 1e-4) x max(1, |目的値|) かつ g_c > `ENOMOTO_T_MIP_CLIQUE_FB_RATIO` (既定 0.5) x g_d
    /// なら使う (ほかのカットで伸びない問題でだけ使う)。drayage-25-23 は g_d = 0、30n20b8 は g_d = 15 に対し g_c は 1 程度
    fn clique_fallback_trial(&mut self, root_iters: u64, first_obj: f64) -> bool {
        let t0 = std::time::Instant::now();
        let Some((obj0, obj1, nfrac0, nfrac1, ncuts)) = self.clique_trial_lp(root_iters) else {
            if self.params.verbose {
                eprintln!("MIP: clique fallback: no violated clique cuts; not used ({:.2}s)", t0.elapsed().as_secs_f64());
            }
            return false;
        };
        let (gc, gd) = (obj1 - obj0, obj0 - first_obj);
        let scale = obj0.abs().max(1.0);
        let on = gc > tunable!("ENOMOTO_T_MIP_CLIQUE_FB_GAIN", 1e-4, f64) * scale && gc > tunable!("ENOMOTO_T_MIP_CLIQUE_FB_RATIO", 0.5, f64) * gd;
        // `ENOMOTO_T_MIP_CLIQUE_FB_FRAC` = r (> 0、既定 0.9。0 で使わない): ほかのカットで目的値がまったく動かず (g_d <= 1e-9)、clique カットで分数の列の数が
        // r 倍以下に減るときも使う (drayage-100-23 は根の LP 値が最適値で g_c = 0。clique カットで LP 解が整数に近づき根の RENS が効く)
        let frac_r = tunable!("ENOMOTO_T_MIP_CLIQUE_FB_FRAC", 0.9, f64);
        let on = on || (frac_r > 0.0 && gd <= 1e-9 * scale && (nfrac1 as f64) <= frac_r * nfrac0 as f64);
        if self.params.verbose {
            eprintln!(
                "MIP: clique fallback: {ncuts} cuts, LP {obj0:.10e} -> {obj1:.10e} (g_c {:.3e}, g_d {:.3e} relative), fractional {nfrac0} -> {nfrac1}; {} ({:.2}s)",
                gc / scale,
                gd / scale,
                if on { "used" } else { "not used" },
                t0.elapsed().as_secs_f64()
            );
        }
        on
    }

    /// `ENOMOTO_MIP_ZH_TRIAL` の判定 (試作、[`super::zerohalf::zh_trial_mode`]): 根のカットのループを zerohalf なしで回し終えた LP で、
    /// zerohalf だけのカット (破れの大きい順に 1 ラウンド最大 `ENOMOTO_T_MIP_ZH_TRIAL_CUTS` (100) 本) を加えて解き直すのを
    /// `ENOMOTO_T_MIP_ZH_TRIAL_ROUNDS` (3) ラウンド回し、伸び g_z を測る。g_z が根のギャップ (打ち切り値があれば打ち切り値 - 目的値、なければ
    /// ループの伸び g_d) の `ENOMOTO_T_MIP_ZH_TRIAL_FRAC` (既定 0.1) 以上、かつ g_z > `ENOMOTO_T_MIP_ZH_TRIAL_GAIN` (1e-4) x max(1, |目的値|) なら使う。
    /// 試した行は外し、LP の状態を戻す
    pub(super) fn zh_trial(&mut self, root_iters: u64, first_obj: f64) -> bool {
        let t0 = std::time::Instant::now();
        let p = self.p;
        let obj0 = self.lp.objective();
        let x0 = self.lp.col_values();
        let nfrac0 = self.fractional(&x0).len();
        let saved = self.lp.save_state();
        let m0 = self.lp.num_rows();
        let it0 = self.lp.total_iterations();
        let rounds = tunable!("ENOMOTO_T_MIP_ZH_TRIAL_ROUNDS", 3usize, usize);
        let max_cuts = tunable!("ENOMOTO_T_MIP_ZH_TRIAL_CUTS", 100usize, usize);
        // 期限: 求解の期限と、根の LP の `ENOMOTO_T_MIP_ZH_TRIAL_LP_MULT` (3) 倍 + 1 秒の早い方
        let lim = std::time::Duration::from_secs_f64((tunable!("ENOMOTO_T_MIP_ZH_TRIAL_LP_MULT", 3.0, f64) * self.root_lp_secs + 1.0).min(1e6));
        let dl = t0 + lim;
        let deadline = Some(self.deadline.map_or(dl, |g| g.min(dl)));
        let mut obj = obj0;
        let mut x = x0;
        let (mut ncuts, mut nnz, mut sep_secs) = (0usize, 0usize, 0.0f64);
        let mut objs = Vec::new();
        let mut ok = true;
        for _ in 0..rounds {
            let ts = std::time::Instant::now();
            let zhd = if super::zerohalf::zh_deadline_on() { deadline } else { None };
            let raw = super::zerohalf::with_zh_deadline(zhd, || {
                super::hnfcut::map_rows(&p.rows, &p.row_lo, &p.row_up, &self.cut_int, &self.dom.global_lo, &self.dom.global_up, &x, |rw, rl, ru, ii, lo, up, xx| super::zerohalf::zerohalf_cuts(rw, rl, ru, ii, lo, up, xx, 100))
            });
            sep_secs += ts.elapsed().as_secs_f64();
            let mut cs: Vec<(f64, super::cuts::RawCut)> = raw
                .into_iter()
                .filter_map(|c| {
                    let act: f64 = c.coefs.iter().map(|&(j, a)| a * x[j]).sum();
                    let norm = c.coefs.iter().map(|&(_, a)| a * a).sum::<f64>().sqrt();
                    let viol = act - c.rhs;
                    (norm > 0.0 && viol > 1e-6 * c.rhs.abs().max(1.0)).then(|| (viol / norm, c))
                })
                .collect();
            if cs.is_empty() {
                break;
            }
            cs.sort_by(|a, b| b.0.total_cmp(&a.0));
            cs.truncate(max_cuts);
            ncuts += cs.len();
            nnz += cs.iter().map(|(_, c)| c.coefs.len()).sum::<usize>();
            let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = cs.into_iter().map(|(_, c)| (c.coefs, f64::NEG_INFINITY, c.rhs)).collect();
            self.lp.add_rows(&rows);
            let st = self.lp.solve(&SolveLimits { iteration_limit: 10 * root_iters.max(100) + 10_000, cutoff: f64::INFINITY, deadline });
            if st != LpStatus::Optimal {
                ok = false;
                break;
            }
            obj = self.lp.objective();
            objs.push(obj);
            x = self.lp.col_values();
            if self.fractional(&x).is_empty() || std::time::Instant::now() >= dl {
                break;
            }
        }
        let iters = self.lp.total_iterations() - it0;
        let nfrac1 = self.fractional(&x).len();
        let mut remove = vec![false; self.lp.num_rows()];
        for r in remove.iter_mut().skip(m0) {
            *r = true;
        }
        self.lp.delete_rows(&remove);
        self.lp.restore_state(&saved);
        let scale = obj0.abs().max(1.0);
        let (gz, gd) = (obj - obj0, obj0 - first_obj);
        let cut = self.prune_limit();
        let gap = if cut.is_finite() && cut > obj0 { cut - obj0 } else { f64::INFINITY };
        // 尺度: 根のギャップ (打ち切り値があれば)、なければループの伸び
        let base = if gap.is_finite() { gap } else { gd.max(1e-9 * scale) };
        let frac = tunable!("ENOMOTO_T_MIP_ZH_TRIAL_FRAC", 0.1, f64);
        let on = ok && gz > tunable!("ENOMOTO_T_MIP_ZH_TRIAL_GAIN", 1e-4, f64) * scale && gz >= frac * base;
        if self.params.verbose {
            let row_nnz = p.rows.iter().map(|r| r.len()).sum::<usize>() as f64 / p.m.max(1) as f64;
            // zerohalf の元になれる行 (整数列だけ・整数係数) の割合
            let zrows = p.rows.iter().filter(|r| r.iter().all(|&(j, a)| self.cut_int[j] && (a - a.round()).abs() < 1e-9)).count();
            eprintln!(
                "MIP: zh trial: {} rounds, {ncuts} cuts (avg {:.1} nonzeros, rows avg {row_nnz:.1}, zh rows {zrows}/{}), LP {obj0:.10e} -> {obj:.10e} [{}], g_z {:.3e} g_d {:.3e} gap {:.3e} (relative to |obj|), g_z/gap {:.3} g_z/g_d {:.3}, fractional {nfrac0} -> {nfrac1}, {iters} LP iters (root LP {root_iters}), sep {sep_secs:.2}s, root LP {:.2}s; {} ({:.2}s)",
                objs.len(),
                nnz as f64 / ncuts.max(1) as f64,
                p.m,
                objs.iter().map(|o| format!("{:.6e}", o + p.offset)).collect::<Vec<_>>().join(" "),
                gz / scale,
                gd / scale,
                gap / scale,
                gz / gap,
                gz / gd.max(1e-12),
                self.root_lp_secs,
                if on { "used" } else { "not used" },
                t0.elapsed().as_secs_f64()
            );
        }
        on
    }

    /// 根の切除平面ループ。LP は根の最適解の状態で呼ぶこと。終わったときも LP は最適 (でなければ偽)。
    /// `ENOMOTO_MIP_EQUIV2_CUTS` (試作、既定では使わない): 前処理が `ENOMOTO_MIP_PROBE_EQUIV2` で同値な 0-1 列を多数代入した問題
    /// ([`super::EQUIV2_USED`]) だけ、根のカットのループの予算を広げる: 面の頂点での分離を止め、根のカットの時間の上限を全体の
    /// `ENOMOTO_T_MIP_EQUIV2_CUT_TIME_FRAC` (既定 0.3)、1 回の LP の上限を `ENOMOTO_T_MIP_EQUIV2_CUT_LP_TIME_FRAC` (既定 0.05) にし、
    /// LP が上限で止まってもカットを外さずに解き続ける (`ENOMOTO_MIP_CUT_LP_CONTINUE` と同じ)。physiciansched6-2 は代入の後に
    /// tableau のカットで根の下界が伸び続けるが (HiGHS も Probing の代入の後のカットで根を閉じる)、既定の予算では 1〜2 ラウンドで止まる
    pub(super) fn equiv2_cuts(&self) -> bool {
        !self.params.submip && env_str!("ENOMOTO_MIP_EQUIV2_CUTS").is_some() && super::EQUIV2_USED.with(|f| f.get())
    }

    /// `ENOMOTO_MIP_CUT_AGG` (試作、既定では使わない): 根のカットのループで、集約した行からのカットの枠
    /// ([`pick_agg_quota`]、`ENOMOTO_T_MIP_CUTSEL_AGG_QUOTA` を指定しなければ 50) を使い、それが回るだけの予算にする:
    /// 面の頂点での分離をしない、1 回の LP の上限を全体の 5% 以上にする、伸びている間は根のカットの時間の上限を延ばす
    /// (`ENOMOTO_MIP_CUT_EXTEND` と同じ)、1 ラウンドの上限を 1000 本に広げない (`ENOMOTO_MIP_NO_CUT_WIDEN` と同じ)。
    /// physiciansched6-2 では HiGHS の根の下界を上げているのが一般係数の tableau・経路集約のカットで、ENOMOTO の候補にも
    /// 似たものがあるのに、元の行・zerohalf の効き目の大きい 2〜3 非零のカットに質の閾値で負けて 1 本も採られていなかった
    /// (analysis/physcmp_20261023.md)
    /// 値が `1` (など下の文字を含まない値) なら全部。文字の並びなら、その部分だけ: `q` 枠、`l` 1 回の LP の上限、`e` 延長、
    /// `w` 上限を広げない、`f` 面の頂点の分離をしない (例 `qlw`。analysis/cand9bisect_20261024.md)
    pub(super) fn cut_agg(&self) -> bool {
        // `ENOMOTO_MIP_SUBMIP_CUT_AGG` (試作、既定では使わない): サブ MIP の根のカットのループでも使う (部分は `ENOMOTO_MIP_CUT_AGG` の値から)
        (!self.params.submip && env_def!("ENOMOTO_MIP_CUT_AGG", "f").is_some())
            || (self.params.submip && (env_str!("ENOMOTO_MIP_SUBMIP_CUT_AGG").is_some() || super::submip_highs('c')))
    }

    /// [`Self::cut_agg`] の部分 `c` を使うか
    fn cut_agg_has(&self, c: char) -> bool {
        self.cut_agg()
            && match env_def!("ENOMOTO_MIP_CUT_AGG", "f") {
                Some(v) if v.chars().any(|d| "qlewf".contains(d)) => v.contains(c),
                _ => true,
            }
    }

    /// `ENOMOTO_MIP_ROOT_CUT_HIGHS` (試作、既定では使わない): 外側の根のカットのループを HiGHS の回し方に寄せる。
    /// (1) 時間の上限をループの始めから数え、`ENOMOTO_T_MIP_CUT_MIN_ROUNDS` (10) ラウンドまでは時間の上限で止めない
    /// (`ENOMOTO_MIP_CUT_CAP_FROM_LOOP` と同じ)、(2) 1 ラウンドに採るカットの上限を `ENOMOTO_T_MIP_ROOT_CUT_HIGHS_MAX_CUTS` (500)・平行度の上限を `..._MAXPAR` (0.9)、
    /// (3) 1 回の LP の時間の上限を全体の `ENOMOTO_T_MIP_ROOT_CUT_HIGHS_LP_TIME_FRAC` (0.1) 以上、上限で止まった LP は解き続ける
    /// (`ENOMOTO_MIP_CUT_LP_CONTINUE` と同じ)、(4) 動かないラウンドで 1 ラウンドの上限を行の数まで広げる規則を使わない。
    /// physiciansched6-2 では HiGHS の根は tableau のカットを 1 ラウンドに 45〜157 本入れて 5〜8 ラウンドで下界を 49324 にする
    /// (tableau の分離を外すと 13 ラウンドで 47960)。ENOMOTO の既定は前景のカットが時間の上限で 0 ラウンド、上限を外しても 50 本ずつで
    /// 伸びず、広げると 1000 本の LP が 1.2 秒の上限に当たって捨てられていた (analysis/highstree_phys_20261024.md)
    /// `ENOMOTO_MIP_CUTSEL_PROBE` の比べ: 既定 (`pool` = false: [`select_cuts`]、50 本) または POOL (HiGHS の点数、1000 本・平行度 0.1、
    /// 2 ラウンド目からはプールで違反しているカットも候補) の選び方で `k` ラウンド (毎ラウンド分離し直す) 回し、下界の伸び・時間・足したカットの本数を返す。終わったら足した行を外し、LP の状態 (基底・LU) と HiGHS の点数の状態を元に戻す。
    /// 戻り値の 1 つ目はラウンドごとの (そこまでの伸び, そこまでの時間)。`stop` が真を返したら (ラウンドの番号, 伸び, 時間)、または
    /// `deadline` を過ぎたら、`k` ラウンドを待たずにやめる。
    /// `keep` なら終わりの状態 ([`ProbeEnd`]) も返す (`ENOMOTO_MIP_CUTSEL_PROBE_KEEP`)。
    pub(super) fn probe_cut_mode(&mut self, pool: bool, k: usize, deadline: std::time::Instant, stop: &dyn Fn(usize, f64, f64) -> bool, keep: bool) -> (Vec<(f64, f64)>, usize, Option<ProbeEnd<L::State>>) {
        let p = self.p;
        let t0 = std::time::Instant::now();
        let obj0 = self.lp.objective();
        let st0 = self.lp.save_state();
        let m0 = self.lp.num_rows();
        let keep_sel = (self.cutsel_best, self.cutsel_factor);
        // 比べの分離の副作用を残さない: 乱数・tableau の記憶と予算の数え・面の頂点の時間の勘定・並列の分離の CPU 時間・LP の反復の累計を
        // 保存して最後に戻す (noswot では比べの後の 1 ラウンド目の候補が 120 -> 118 本にずれ、既定を選んでも探索が変わっていた)
        let keep_rng = self.rng;
        let keep_tab = (self.tab_memo.clone(), self.tab_memo_mode, self.tab_memo_stats, self.tab_calls, self.tab_tries);
        let keep_face = (self.face_secs, self.face_used, self.face_off_loop, self.sep_par_cpu, self.last_cut_lp_secs);
        let keep_iters = self.lp.total_iterations();
        let cap = if self.params.time_limit.is_finite() { (self.cut_lp_time_frac() * self.params.time_limit).max(1.0) } else { f64::INFINITY };
        // 試作 `ENOMOTO_MIP_LOSE6_2_TIE`: 同点の比べ直しでは 1 回の LP の上限も `ENOMOTO_T_MIP_LOSE6_2_TIE_BUDGET` 倍にする
        let cap = cap * LOSE6_2_PROBE_CAP_MULT.with(|c| c.get());
        let mut gain = 0.0;
        let mut hist: Vec<(f64, f64)> = Vec::new();
        let mut nadd = 0usize;
        let mut all_rows: Vec<(Vec<(usize, f64)>, f64, f64)> = Vec::new();
        let alt_has = |c: char| env_def!("ENOMOTO_MIP_CUTSEL_PROBE_ALT", "NS").is_none_or(|v| v.contains(c));
        for r in 0..k {
            if self.time_up() || (r > 0 && std::time::Instant::now() >= deadline) {
                break;
            }
            let (cands, forced_r): (Vec<Candidate>, Vec<Candidate>) = {
                let x = self.lp.col_values();
                let (mut c, _) = self.separate_face_split(&x, false);
                if pool && r > 0 && alt_has('P') {
                    for (eff, kk) in self.pool_violated(&x) {
                        let (cc, rr, _) = &self.cut_pool[kk];
                        c.push(Candidate { coefs: cc.clone(), rhs: *rr, efficacy: eff, forced: false, src: sepprof::SRC_POOL });
                    }
                }
                c.into_iter().partition(|c| !c.forced)
            };
            let x = self.lp.col_values();
            // `ENOMOTO_MIP_CUTSEL_PROBE_ALT`: POOL の側は挙げた部品 (S: 点数、N: 本数、P: プールから戻す) だけ使う ([`Self::pool_part`])
            let nmax = if pool && alt_has('N') { 1000usize } else { tunable!("ENOMOTO_T_MIP_ROOT_MAX_CUTS", 50usize, usize) }.min(p.m.max(50));
            let chosen = if pool && alt_has('S') {
                self.select_cuts_highs(cands, nmax, &x, tunable!("ENOMOTO_T_CUTSEL_HIGHS_MAXPAR", 0.1, f64))
            } else if let Some(fl) = super::cutsel_ens::flags(self.params.submip) {
                select_cuts_ens(cands, &forced_r, nmax, p, fl)
            } else {
                select_cuts(cands, nmax, p)
            };
            PROBE_CLIQUE_ROWS.with(|c| c.set(c.get() + chosen.iter().chain(forced_r.iter()).filter(|c| c.src & !sepprof::SRC_FACE == sepprof::SRC_CLIQUE).count()));
            let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = chosen.iter().chain(forced_r.iter()).map(|c| (c.coefs.clone(), f64::NEG_INFINITY, c.rhs)).collect();
            if rows.is_empty() {
                break;
            }
            nadd += rows.len();
            let before = self.lp.objective();
            self.lp.add_rows(&rows);
            if keep {
                all_rows.extend(rows.iter().cloned());
            }
            let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(cap.min(1e6));
            let d = Some(self.deadline.map_or(d, |g| g.min(d)));
            let st = self.lp.solve(&SolveLimits { iteration_limit: 10 * self.avg_node_iters().max(1000) + 10_000, cutoff: f64::INFINITY, deadline: d });
            if st != LpStatus::Optimal {
                break;
            }
            gain = self.lp.objective() - obj0;
            hist.push((gain, t0.elapsed().as_secs_f64()));
            if stop(r, gain, t0.elapsed().as_secs_f64()) {
                break;
            }
            if self.lp.objective() <= before + 1e-9 * before.abs().max(1.0) && r > 0 && !PROBE_NO_FLAT_STOP.with(|c| c.get()) {
                break;
            }
        }
        let end = keep.then(|| ProbeEnd {
            rows: std::mem::take(&mut all_rows),
            lp: self.lp.save_state(),
            rng: self.rng,
            tab: (self.tab_memo.clone(), self.tab_memo_mode, self.tab_memo_stats, self.tab_calls, self.tab_tries),
            face: (self.face_secs, self.face_used, self.face_off_loop, self.sep_par_cpu, self.last_cut_lp_secs),
            sel: (self.cutsel_best, self.cutsel_factor),
            iters: self.lp.total_iterations(),
            rounds: hist.len(),
            nfrac: self.fractional(&self.lp.col_values()).len(),
        });
        let mut remove = vec![false; self.lp.num_rows()];
        for rr in remove.iter_mut().skip(m0) {
            *rr = true;
        }
        self.lp.delete_rows(&remove);
        self.lp.restore_state(&st0);
        (self.cutsel_best, self.cutsel_factor) = keep_sel;
        self.rng = keep_rng;
        (self.tab_memo, self.tab_memo_mode, self.tab_memo_stats, self.tab_calls, self.tab_tries) = keep_tab;
        (self.face_secs, self.face_used, self.face_off_loop, self.sep_par_cpu, self.last_cut_lp_secs) = keep_face;
        self.lp.set_total_iterations(keep_iters);
        let _ = gain;
        (hist, nadd, end)
    }

    /// `ENOMOTO_MIP_CUTSEL_PROBE_KEEP`: 比べの勝った側の終わりの状態に戻す (足した行を LP に入れ直し、LP・分離器の状態をその時のものにする)。
    /// 行は普通のカットと同じく LP の行の記録 ([`Self::add_cut_rows`]) とプールに入れる
    fn probe_restore(&mut self, e: ProbeEnd<L::State>) {
        if !e.rows.is_empty() {
            for (c, _, r) in &e.rows {
                self.add_to_pool(c, *r);
            }
            self.add_cut_rows(&e.rows);
        }
        self.lp.restore_state(&e.lp);
        self.rng = e.rng;
        (self.tab_memo, self.tab_memo_mode, self.tab_memo_stats, self.tab_calls, self.tab_tries) = e.tab;
        (self.face_secs, self.face_used, self.face_off_loop, self.sep_par_cpu, self.last_cut_lp_secs) = e.face;
        (self.cutsel_best, self.cutsel_factor) = e.sel;
        self.lp.set_total_iterations(e.iters);
    }

    /// `ENOMOTO_MIP_CUTSEL_PROBE` (既定 `rate`、`0` で外す。計測 23 で既定化) で、根の 1 ラウンド目の比べで POOL の選び方が選ばれたか
    /// (外側の求解の中だけ。背景のカットの求解と再スタートの後の根も同じ選択に従う)。
    pub(super) fn probe_pool_on(&self) -> bool {
        !self.params.submip && env_def!("ENOMOTO_MIP_CUTSEL_PROBE", "rate").is_some() && CUT_PROBE_CHOICE.load(std::sync::atomic::Ordering::Relaxed) == 2
    }

    /// `ENOMOTO_MIP_CUTSEL_HIGHS_GUARD` のもとで固定費用のネットワーク型か: そこでは根のカットのプールの選び直しの部品
    /// (`ENOMOTO_MIP_ROOT_CUT_HIGHS`・`ENOMOTO_MIP_CUTSEL_HIGHS`・`ENOMOTO_MIP_CUTSEL_HIGHS_POOL`・`ENOMOTO_T_MIP_ROOT_CUT_AGE`) を使わない
    /// (mc11: 根の下界 11684 -> 11500〜11644)
    pub(super) fn cutsel_guard_fc(&self) -> bool {
        self.fc_network && !self.params.submip && (env_str!("ENOMOTO_MIP_CUTSEL_HIGHS_GUARD").is_some() || env_def!("ENOMOTO_MIP_CUTSEL_PROBE", "rate").is_some())
    }

    pub(super) fn root_cut_highs(&self) -> bool {
        // `ENOMOTO_MIP_CUTSEL_HIGHS_GUARD` なら固定費用のネットワーク型では使わない (mc11: 1 ラウンド 1000 本で根のループが 1.2 -> 13 秒)
        (env_str!("ENOMOTO_MIP_ROOT_CUT_HIGHS").is_some() || self.probe_pool_on()) && !self.params.submip && !self.cutsel_guard_fc()
    }

    /// `ENOMOTO_MIP_CUTSEL_PROBE_ALT=<文字>` (既定 `NS`。計測 23 で既定化): `ENOMOTO_MIP_CUTSEL_PROBE` の 2 つ目の選び方 (POOL 一式) のうち、
    /// 文字で挙げた部品だけを使う (挙げない部品は既定のまま)。`ENOMOTO_MIP_CUTSEL_PROBE_ALT=0` なら全部 (今までの POOL 一式)。
    /// S: HiGHS の点数での選択 (平行度 0.1、[`Self::select_cuts_highs`]) / N: 1 ラウンド 1000 本 / P: プールで違反しているカットを候補に戻す /
    /// G: 効かないラウンドが 10 続いたカットだけ外す / T: 時間の上限をループの始めから数え、最低 10 ラウンド / L: 上限で止まった LP を解き続ける /
    /// F: 1 回の LP の時間の上限を全体の 10% 以上 / W: 1 ラウンドの上限を広げる規則を使わない
    pub(super) fn pool_part(&self, c: char) -> bool {
        self.probe_pool_on() && env_def!("ENOMOTO_MIP_CUTSEL_PROBE_ALT", "NS").is_none_or(|v| v.contains(c))
    }

    /// [`Self::root_cut_highs`] の部品ごとの形 (`ENOMOTO_MIP_ROOT_CUT_HIGHS` なら全部、`ENOMOTO_MIP_CUTSEL_PROBE` で POOL を選んだら [`Self::pool_part`])
    fn rch_part(&self, c: char) -> bool {
        (env_str!("ENOMOTO_MIP_ROOT_CUT_HIGHS").is_some() || self.pool_part(c)) && !self.params.submip && !self.cutsel_guard_fc()
    }

    /// 根のカットのループの 1 回の LP の時間の上限 (全体に対する割合)。
    fn cut_lp_time_frac(&self) -> f64 {
        let f = self.cut_lp_time_frac_base();
        if self.rch_part('F') { f.max(tunable!("ENOMOTO_T_MIP_ROOT_CUT_HIGHS_LP_TIME_FRAC", 0.1, f64)) } else { f }
    }

    fn cut_lp_time_frac_base(&self) -> f64 {
        if self.cut_agg_has('l') {
            let f = if self.equiv2_cuts() { tunable!("ENOMOTO_T_MIP_EQUIV2_CUT_LP_TIME_FRAC", 0.05, f64) } else { tunable!("ENOMOTO_T_MIP_CUT_LP_TIME_FRAC", 0.02, f64) };
            f.max(tunable!("ENOMOTO_T_MIP_CUT_AGG_LP_TIME_FRAC", 0.05, f64))
        } else if self.equiv2_cuts() {
            tunable!("ENOMOTO_T_MIP_EQUIV2_CUT_LP_TIME_FRAC", 0.05, f64)
        } else {
            tunable!("ENOMOTO_T_MIP_CUT_LP_TIME_FRAC", 0.02, f64)
        }
    }

    pub(super) fn root_cut_loop(&mut self, root_iters: u64) -> bool {
        let p = self.p;
        // 固定費用のネットワーク型か (`root_cut_highs` がループの初めから使うので先に決める。下でも同じ値を入れ直す)
        self.fc_network = !self.params.submip && fixed_charge_network(p);
        // 診断 (analysis/cutmix_20261111.md): 型の割合と 2 択の判定を再スタートごとに出す
        if !self.params.submip && super::cutmix::want_root() {
            super::cutmix::at_root(p, Some(&self.lp.col_values()), self.params.restarts, self.fc_network, fixed_charge_network_for_nodes(p));
        }
        // `ENOMOTO_MIP_CUTSEL_PROBE`: 外側の最初の根では前の求解の選択を消す (0 ラウンド目の比べで決め直す)
        // (試作 `ENOMOTO_MIP_GAP2_PROBE1` では clique の fallback の 2 回目のループで消さない。下の `probe_pending` を参照)
        if !self.params.submip && self.snap.is_none() && self.params.restarts == 0 && self.cut_cont.is_none() && env_def!("ENOMOTO_MIP_CUTSEL_PROBE", "rate").is_some()
            && !(CLIQUE_FB_SECOND.with(|c| c.get()) && env_str!("ENOMOTO_MIP_GAP2_PROBE1").is_some_and(|v| v != "0"))
        {
            CUT_PROBE_CHOICE.store(0, std::sync::atomic::Ordering::Relaxed);
        }
        let n = p.n;
        let mut stall = 0;
        let mut intobj_stall = 0usize;
        // 整数値に固定された連続列を整数列と扱って目的値が整数刻みか (`ENOMOTO_MIP_CUT_INTOBJ_STALL` の判定に使う)
        let intobj_step = env_def!("ENOMOTO_MIP_CUT_INTOBJ_STALL", "1").is_some() && p.objective_step_bounds(Some((&self.dom.lo, &self.dom.up))).is_some();
        // 試作 `ENOMOTO_MIP_GAP2_FLAT=k` (既定では使わない。analysis/gap2_rootcost_20261109.md): 目的値が整数刻みでない問題でも、
        // `ENOMOTO_MIP_CUT_INTOBJ_STALL` と同じ「一度上がった後に k ラウンド (既定 3) まったく動かない」で根のカットのループを止める。
        // physiciansched6-2 (連続列 9 本に費用があり整数刻みと判定されない) は 4 ラウンド目で下界が最適値 49324 に届いた後、分数の列の
        // 数が減り続けるので停滞と数えられず、下界が全く動かないまま 11 ラウンド (約 4.8 秒) 回っていた
        let gap2_flat = (!intobj_step && !self.params.submip)
            .then(|| env_str!("ENOMOTO_MIP_GAP2_FLAT").filter(|v| *v != "0").map(|v| v.parse::<usize>().unwrap_or(3).max(1)))
            .flatten();
        let mut prev_obj = self.lp.objective();
        let mut prev_nfrac = self.fractional(&self.lp.col_values()).len();
        let first_obj = prev_obj;
        // `ENOMOTO_MIP_CUTSTALL_GAP` ([`cutstall_gap`]): 外側の根 (裏のカットの写しを含む) だけ
        let cs_gap = cutstall_gap().filter(|_| !self.params.submip);
        // 試作 `ENOMOTO_MIP_ROOTSTOP` の `r` ([`super::rootstop::RateStop`]): 外側の主の根だけ
        let mut rate_stop = (super::rootstop::on('r') && !self.params.submip && self.snap.is_none() && self.cut_cont.is_none()).then(|| super::rootstop::RateStop::new(first_obj, self.params.restarts));
        if cs_gap.is_some() && self.params.restarts == 0 && self.snap.is_none() && self.cut_cont.is_none() && first_obj.is_finite() {
            CUTSTALL_BASE.store((first_obj + p.offset).to_bits(), std::sync::atomic::Ordering::Relaxed);
        }
        let cs_first = {
            let b = cutstall_base() - p.offset;
            if cs_gap.is_some() && b.is_finite() { b.min(first_obj) } else { first_obj }
        };
        if cs_gap.is_some() && self.params.verbose && self.snap.is_none() {
            eprintln!("MIP: cut loop (ENOMOTO_MIP_CUTSTALL_GAP): base {cs_first:.10e} (LP {first_obj:.10e}, offset {:.6e}, prune limit {:.10e})", p.offset, self.prune_limit());
        }
        // 尺度 = min(カットで上げた分, 暫定解との残りの差)。どちらかの割合を超えて伸びたラウンドは停滞と数えない
        let cs_scale = |slf: &Self, obj: f64| -> f64 {
            let cut = slf.prune_limit();
            let mut s = obj - cs_first;
            if cut.is_finite() {
                s = s.min(cut - obj);
            }
            s.max(1e-9 * obj.abs().max(1.0))
        };
        if clique_cuts_mode() == Some("auto") && self.clique_auto.is_none() && !self.params.submip {
            // 再スタートの後は最初の求解の判定を引き継ぐ (再スタートの根は前の根のカットを持つので、試しても伸びない)
            let on = match CLIQUE_AUTO_FIRST.with(|c| c.get()) {
                Some(v) if self.params.restarts > 0 => v,
                _ => self.clique_trial(root_iters),
            };
            if self.params.restarts == 0 {
                CLIQUE_AUTO_FIRST.with(|c| c.set(Some(on)));
            }
            self.clique_auto = Some(on);
        }
        // `fallback`: ループの後で判定する ([`Self::clique_fallback_trial`])。再スタートの後は最初の求解の判定を引き継ぐ
        let mut clique_fb = clique_cuts_mode() == Some("fallback") && self.clique_auto.is_none() && !self.params.submip;
        if clique_fb && self.params.restarts > 0 {
            if let Some(v) = CLIQUE_AUTO_FIRST.with(|c| c.get()) {
                self.clique_auto = Some(v);
                clique_fb = false;
            }
        }
        // 試作 `probe` (課題 CLIQFB、analysis/cliqfb_20261111.md): ループの始めの比べで判定する ([`Self::clique_probe`])。再スタートの後は最初の判定を引き継ぐ
        let mut clique_probe_pending = matches!(clique_cuts_mode(), Some("probe") | Some("probe_s") | Some("probe_r")) && self.clique_auto.is_none() && !self.params.submip;
        if clique_probe_pending && self.params.restarts > 0 {
            if let Some(v) = CLIQUE_AUTO_FIRST.with(|c| c.get()) {
                self.clique_auto = Some(v);
                clique_probe_pending = false;
            }
        }
        // HiGHS の停滞判定 (`ENOMOTO_MIP_CUT_STALL_HIGHS`, HighsMipSolverData::evaluateRootNode): 最初の LP 解からの
        // 移動方向の平均と今の移動の内積 (進み具合) を平滑化し、それが 1% 以上伸びず、かつ目的値の伸びが前のラウンドまでの
        // 伸びの 0.1% 以下なら停滞。3 回続けて停滞したら止める (目的値が動かなくても LP 解が動いている間は続ける)。
        // 40 問では 19 問 27.87 -> 17 問 28.49 (最大 100 ラウンドにすると 16 問 29.77): 根の下界は上がる (neos-1456979
        // 154 -> 163) が、こちらの 1 ラウンドは HiGHS の数倍重く、根に時間がかかる問題 (qnet1・10teams・misc07) が遅くなる
        let highs_stall = env_str!("ENOMOTO_MIP_CUT_STALL_HIGHS").is_some() && !self.params.submip;
        let first_x = self.lp.col_values();
        let mut avgdir = vec![0.0f64; n];
        let mut smooth = 0.0f64;
        let mut hstall = 0usize;
        let mut hrounds = 0usize;
        // サブ MIP (RENS/RINS) では分離に時間をかけない
        // `ENOMOTO_T_MIP_SUBMIP_CUT_ROUNDS` / `..._STALL` (試作、既定 5 / 3): サブ MIP の根のカットのラウンドの上限と停滞の回数。
        // beasleyC3 の根の RENS の近傍 (最適値 763・770) は、通常の求解 (25 ラウンド) なら根の下界が 760〜762 に上がって根で
        // 解けるが、5 ラウンドのサブ MIP は 500 ノードで止まり 773〜779 だった。HiGHS のサブ MIP は通常のカットのループを回し、
        // 同じ形の近傍を 1 ノードで解く (analysis/beasleyC3_r3_20261020.md)
        let fc_sub = self.params.submip && self.params.fc_submip;
        // `ENOMOTO_MIP_SUBMIP_CUT_AGG` のサブ MIP: ラウンドの上限 `ENOMOTO_T_MIP_SUBMIP_CUT_AGG_ROUNDS` (25)、時間の上限はサブ MIP の時間の
        // `ENOMOTO_T_MIP_SUBMIP_CUT_AGG_TIME_FRAC` (0.5) 倍。physiciansched6-2 の根の RINS のサブ MIP は、HiGHS ならカットだけで下界が
        // 49324 に上がって 1 ノードで解ける (2.8 秒、分離の LP 4019 反復) が、ENOMOTO の既定 (5 ラウンド・時間の 1 割・枠なし) では
        // 49257 で止まって 20 秒でも解なし。枠・25 ラウンド・5 割では 6.4 秒・0 ノードで 49324 (analysis/physprimal_20261024.md)
        let sub_agg = self.params.submip && !fc_sub && self.cut_agg();
        // `ENOMOTO_MIP_SUBMIP_HIGHS` の `c`: 根のカットを親と同じだけ回す (ラウンドの上限 25・停滞 10 回。時間の上限は `sub_agg` と同じ
        // サブ MIP の時間の 5 割。HiGHS の `solveSubMip` はカットの設定を親から引き継ぐ)
        let sub_hc = self.params.submip && !fc_sub && super::submip_highs('c');
        // `ENOMOTO_T_MIP_SUBMIP_HC_CUT_ROUNDS` (試作、既定は `ENOMOTO_T_MIP_CUT_ROUNDS` と同じ): `sub_hc` のサブ MIP のラウンドの上限だけを
        // 外側と別にする。`ENOMOTO_T_MIP_CUT_ROUNDS=3` は外側の根 (neos-960392 で 3 ラウンド目の密なカットを入れない) と
        // サブ MIP の中 (physiciansched6-2 の根の RINS が速くなる) の両方を変えるので、その切り分け用 (analysis/neos960392_20261101.md の 3 節)
        let mut max_rounds = if sub_hc { tunable!("ENOMOTO_T_MIP_SUBMIP_HC_CUT_ROUNDS", tunable!("ENOMOTO_T_MIP_CUT_ROUNDS", 25usize, usize), usize) } else if fc_sub { tunable!("ENOMOTO_T_MIP_FC_SUBMIP_CUT_ROUNDS", 25usize, usize) } else if sub_agg { tunable!("ENOMOTO_T_MIP_SUBMIP_CUT_AGG_ROUNDS", 25usize, usize) } else if self.params.submip { tunable!("ENOMOTO_T_MIP_SUBMIP_CUT_ROUNDS", 5usize, usize) } else { tunable!("ENOMOTO_T_MIP_CUT_ROUNDS", 25usize, usize) };
        // `ENOMOTO_MIP_TREEEFF` の `s` (試作): 木の中の再スタートの後の根はカットのループを短くする (カットは行として持ち込んでいる)
        if !self.params.submip && self.params.restarts > 0 && super::solver::treeeff('s') && super::solver::TREE_RESTART_CHILD.with(|c| c.get()) {
            max_rounds = max_rounds.min(tunable!("ENOMOTO_T_MIP_TREEEFF_ROUNDS", 5usize, usize));
        }
        let eq2c = self.equiv2_cuts();
        let cut_frac = if sub_agg { tunable!("ENOMOTO_T_MIP_SUBMIP_CUT_AGG_TIME_FRAC", 0.5, f64) } else if eq2c { tunable!("ENOMOTO_T_MIP_EQUIV2_CUT_TIME_FRAC", 0.3, f64) } else { tunable!("ENOMOTO_T_MIP_CUT_TIME_FRAC", 0.1, f64) };
        let mut time_cap = if self.params.time_limit.is_finite() { cut_frac * self.params.time_limit } else { f64::INFINITY };
        // 根のヒューリスティクスの間にカットを強める求解: 止めの合図まで続ける (ラウンド数・時間の上限なし)
        let job = super::solver::CUT_JOB.with(|j| j.borrow().clone()).filter(|_| self.snap.is_some());
        // `ENOMOTO_MIP_SYM_CUTS` ([`super::sym_cuts`]): 根のカットの像を作る生成元と、列ごとにその列を動かす生成元の番号
        let sym_gens = if !self.params.submip && self.params.restarts == 0 && self.snap.is_none() && job.is_none() {
            super::SYM_CUT_GENS.with(|t| t.borrow().clone()).filter(|g| g.iter().all(|perm| perm.len() == n))
        } else {
            None
        };
        let sym_moving: Vec<Vec<u32>> = match &sym_gens {
            Some(gens) => {
                let mut m = vec![Vec::new(); n];
                for (k, g) in gens.iter().enumerate() {
                    for j in 0..n {
                        if g[j] != j {
                            m[j].push(k as u32);
                        }
                    }
                }
                m
            }
            None => Vec::new(),
        };
        let sym_cap = tunable!("ENOMOTO_T_MIP_SYM_CUTS_MAX", 5000usize, usize);
        let mut sym_images = 0usize;
        // 像として新たにプールに入ったカットの番号 (候補に戻すのはこれだけ。ほかのプールのカットは今まで通り)
        let mut sym_pool: std::collections::HashSet<usize> = std::collections::HashSet::new();
        if job.is_some() {
            max_rounds = tunable!("ENOMOTO_T_MIP_BG_CUTS_MAX_ROUNDS", 1000usize, usize);
            time_cap = f64::INFINITY;
        }
        let heavy = job.as_ref().is_some_and(|j| j.heavy);
        // 止めの合図か、試作 `ENOMOTO_MIP_BG_CUTS_DETERM` で裏のカットが自分で終わると決めた ([`super::solver::CutJob::stopped`])
        let stopped = |j: &Option<std::sync::Arc<super::solver::CutJob>>| j.as_ref().is_some_and(|j| j.stopped());
        let mut total_added = 0usize;
        let mut lp_failures = 0usize;
        // `ENOMOTO_MIP_CUT_PHASE2`: 既定の選択 (`select_cuts_hybrid`: 質が最良の 50% 未満のカットは採らない) で停滞して
        // 止まるところで、HiGHS のスコア (違反量 / (効いている非零の数 x そのノルム)、疎なカットを好む。ラウンドをまたいで
        // 最良スコアを覚え、その一定割合以上を採る) に切り替えてもう一度続ける第 2 段階。neos-911970 では既定の選択は根の
        // 下界 47.26 で止まり (100 ラウンド回しても動かない)、HiGHS のスコアなら 51.8 まで上がる (HiGHS は 38 ラウンドで
        // 52.1)。最初から HiGHS のスコアを使うと misc07・mik-250・neos5 が 60 秒で解けなくなった (木が数倍になる) ので、
        // 既定の選択が止まった後にだけ使う。第 2 段階は目的値が `ENOMOTO_T_MIP_CUT_PHASE2_STALL` ラウンド続けて
        // 動かなければ止め、ラウンド数は `ENOMOTO_T_MIP_CUT_PHASE2_ROUNDS` まで (根のカットの時間上限はそのまま)。
        // 40 問 (同時に測った既定 19 問 27.81 に対し) 18 問 28.64: nw04 33 -> 21 秒、h80x6320d の上界は良くなるが、
        // neos5 が時間切れ、10teams 4.2 -> 9.1 秒、qnet1 9.4 -> 14.7 秒、misc07 13.7 -> 18.8 秒 (根の LP 解が変わって木が
        // 変わる)。neos-911970 も 60 秒では解けない (根は 52.1 になるが暫定解が悪くなる) ので既定では使わない
        // 試作 `ENOMOTO_MIP_BG_DETERM_ROBUST` の `p` ([`super::solver::CutJob::robust_phase2`]): 裏のカットのスレッドでも第 2 段階を使う
        let phase2_on = (heavy || env_str!("ENOMOTO_MIP_CUT_PHASE2").is_some() || (self.highs_cutmgmt() && env_str!("ENOMOTO_MIP_HCM_NO_PHASE2").is_none()) || job.as_ref().is_some_and(|j| j.robust_phase2())) && !self.params.submip;
        // 試作 `ENOMOTO_MIP_BG_DETERM_ROBUST` の `o` ([`super::solver::CutJob::veto_own_stall`]): 今までの停滞の判定で止まるところで、試作の判定が停滞と言わなければ続ける
        let veto_own = |j: &Option<std::sync::Arc<super::solver::CutJob>>| j.as_ref().is_some_and(|j| j.veto_own_stall());
        let mut phase2 = false;
        // 試作 `ENOMOTO_MIP_MC11FIX2` の `c` ([`super::solver::mc11fix2`]、[`Solver::cut_cont_p2`]): 主が続けるループを初めから第 2 段階で決まったラウンド数だけ回す
        let cont_p2 = if self.cut_cont.is_some() { self.cut_cont_p2 } else { 0 };
        if cont_p2 > 0 {
            phase2 = true;
            max_rounds = cont_p2;
        }
        // `ENOMOTO_MIP_S4_TINYCUT` = k (試作、既定では使わない。analysis/highs_slow_20261106/S4.md): 再スタートの後の根 (主のスレッド) で、
        // 暫定解と根の LP 値の相対ギャップが `ENOMOTO_T_MIP_S4_TINYGAP_REL` (1e-5) 以下なら、主のカットのループを k ラウンド (既定 1) までにする
        // (残りは裏のカットに任せる)。neos-957323 は再スタートの後の根でカットのループが 6 ラウンド・約 9 秒回るが、目的値が動くのは
        // 1 ラウンド目だけ (-237.75707531 -> -237.75697464、その後は同じ値) で、これを 4 回の再スタートで繰り返していた
        if let Some(k) = env_str!("ENOMOTO_MIP_S4_TINYCUT").map(|v| v.parse::<usize>().unwrap_or(1)) {
            if !self.params.submip && job.is_none() && self.params.restarts >= 1 {
                if let Some((z, _)) = self.incumbent.as_ref() {
                    let lp_z = self.lp.objective() + self.p.offset;
                    let rel = (*z - lp_z).max(0.0) / z.abs().max(lp_z.abs()).max(1.0);
                    if rel <= tunable!("ENOMOTO_T_MIP_S4_TINYGAP_REL", 1e-5, f64) && max_rounds > k {
                        if self.params.verbose {
                            eprintln!("MIP: S4_TINYCUT: cut rounds {max_rounds} -> {k} (relative gap {rel:.3e})");
                        }
                        max_rounds = k;
                    }
                }
            }
        }
        // 第 2 段階の時間の上限 (第 1 段階に使った時間の `ENOMOTO_T_MIP_CUT_PHASE2_TIME_MULT` 倍。qnet1 では再スタート後の
        // 第 2 段階が 88 ラウンド回って根が 6 秒伸び、9.2 秒 -> 17.2 秒になった)
        // 上限はこのスレッドの CPU 時間で測る ([`super::cap_clock`]。求解の開始からの分は `cpu0` から)
        let t_loop0 = super::cap_clock();
        // fallback の 2 回目のループ: 1 回目で根のカットの時間の上限 (`ENOMOTO_T_MIP_CUT_TIME_FRAC`) を使い切っていても回るよう、
        // 上限をこのループの始めから数える (drayage-25-23 では 1 回目に 6.9 秒かかり、2 回目が 1 ラウンドも回らなかった)
        // `ENOMOTO_MIP_CUT_CAP_FROM_LOOP` (既定で使う。`0` で止める。2026-11-01 に cs2 として既定化、analysis/halve_submip_20261030.md の bench12・
        // analysis/cs2_check_20261101.md): 外側の根のカットの時間の上限を、求解の開始からではなくこのループの
        // 始めから数える。また `ENOMOTO_T_MIP_CUT_MIN_ROUNDS` (既定 0) ラウンドまでは時間の上限で止めない (全体の時間切れでは止める)。
        // physiciansched6-2 (60 秒) は前処理と根の LP で 6〜7 秒かかり、上限 (開始から全体の 1 割 = 6 秒) を超えていて前景のカットが
        // 0 ラウンドだった。HiGHS は根のカットを 11〜13 ラウンド回して下界を 49324 (最適値) にする (analysis/highstree_phys_20261024.md)
        let cap_from_loop = (env_def!("ENOMOTO_MIP_CUT_CAP_FROM_LOOP", "1").is_some() || self.rch_part('T')) && !self.params.submip;
        let mut min_rounds = if cap_from_loop { tunable!("ENOMOTO_T_MIP_CUT_MIN_ROUNDS", if self.rch_part('T') { 10usize } else { 0 }, usize) } else { 0 };
        // `ENOMOTO_MIP_CUTSEL_PROBE`: 比べを 0 ラウンド目の始めに行う (時間の上限の判定より前なので、既定では前景のカットが 0 ラウンドの
        // physiciansched6-2 でも比べる)
        let min_rounds0 = min_rounds;
        let probe_pending = !self.params.submip && self.snap.is_none() && self.params.restarts == 0 && self.cut_cont.is_none() && !self.fc_network && env_def!("ENOMOTO_MIP_CUTSEL_PROBE", "rate").is_some();
        // 試作 `ENOMOTO_MIP_GAP2_PROBE1` (既定では使わない。analysis/gap2_rootcost_20261109.md): clique の fallback (と zerohalf の試し) で
        // 続ける 2 回目のループでは、カットの選び方の比べをやり直さず 1 回目の選択 (`CUT_PROBE_CHOICE`) をそのまま使う。
        // neos-960392 では 2 回目の比べに 5.9 秒 (1 ラウンドの分離が 2〜3 秒) かかり、選択は 1 回目と同じ (既定) だった
        let probe_pending = probe_pending && !(CLIQUE_FB_SECOND.with(|c| c.get()) && env_str!("ENOMOTO_MIP_GAP2_PROBE1").is_some_and(|v| v != "0") && CUT_PROBE_CHOICE.load(std::sync::atomic::Ordering::Relaxed) != 0);
        // 試作 `ENOMOTO_MIP_GAP4` の `p` (既定では使わない。analysis/gap4_rootstuck_mem_20261109.md): 根の LP の秒が比べの時間の予算
        // (全体の `ENOMOTO_T_MIP_CUTSEL_PROBE_TIME_FRAC` 倍) の `ENOMOTO_T_MIP_GAP4_PROBE_FRAC` (0.25) 倍を超えるなら比べない。
        // neos-3216931-puriri (根の LP 10 秒) では比べが 1 ラウンドの LP も終えられず予算 29.7 秒を超えて 43.5 秒使い、0 ラウンドで比べて捨てていた
        let probe_pending = probe_pending
            && !(super::solver::gap4('p')
                && self.params.time_limit.is_finite()
                && self.root_lp_secs > tunable!("ENOMOTO_T_MIP_GAP4_PROBE_FRAC", 0.25, f64) * tunable!("ENOMOTO_T_MIP_CUTSEL_PROBE_TIME_FRAC", 0.05, f64) * self.params.time_limit
                && {
                    if self.params.verbose {
                        eprintln!("MIP: GAP4 p: cut selection probe skipped (root LP {:.2}s)", self.root_lp_secs);
                    }
                    true
                });
        let mut cap_base = if (CLIQUE_FB_SECOND.with(|c| c.get()) && env_str!("ENOMOTO_MIP_CLIQUE_FB_SHARED_CAP").is_none()) || cap_from_loop { t_loop0 } else { self.cpu0 };
        let mut phase2_deadline = f64::INFINITY;
        let mut round_ctr = 0usize;
        // 診断 (`ENOMOTO_MIP_DEBUG_FACETIME`): このループの分離・面の頂点の分離・LP の時間の合計
        let face0 = (self.face_secs, self.face_used);
        let (mut sep_tot, mut lp_tot, mut nrounds) = (0.0f64, 0.0f64, 0usize);
        let t_loop_wall = std::time::Instant::now();
        let prof = sepprof::on() && !self.params.submip && job.is_none();
        self.face_off_loop = false;
        self.last_cut_lp_secs = f64::INFINITY;
        // 試作 `ENOMOTO_MIP_MPSEP` (既定では使わない、analysis/mpsep_20261106.md): 多点分離の設定と、in-out の内点 (ループで 1 回だけ求める)
        let mpsep = mpsep_flags(self.params.submip, job.is_some());
        let mut mp_center: Option<Vec<f64>> = None;
        let mut mp_center_tried = false;
        // 絞った形 (値に `g`・`s`・`h`): 止めたか (`g` で最適面が広くなくなったら、このループの残りでは使わない)、前のラウンドの目的値
        let mut mp_off = false;
        let mut mp_last_obj: Option<f64> = None;
        // `h`: 根の LP が重い問題 (根の LP の秒 ≥ `ENOMOTO_T_MIP_MPSEP_GATE_SECS` (0.5)、または元の行の数 ≥ `ENOMOTO_T_MIP_MPSEP_GATE_ROWS` (5000)) だけ
        if mpsep.as_ref().is_some_and(|f| f.contains('h'))
            && self.root_lp_secs < tunable!("ENOMOTO_T_MIP_MPSEP_GATE_SECS", 0.5, f64)
            && p.m < tunable!("ENOMOTO_T_MIP_MPSEP_GATE_ROWS", 5000usize, usize)
        {
            mp_off = true;
            if self.params.verbose {
                eprintln!("MIP: mpsep gate: light root LP ({:.3}s, {} rows); not used in this cut loop", self.root_lp_secs, p.m);
            }
        }
        // 計測用 `ENOMOTO_MIP_MPSEP_WIDTH_ONLY`: 外側の最初の主の根のカットのループの始めで、最適面の広さ ([`Solver::mpsep_width`]) を 1 行の JSON で出して終わる
        if env_str!("ENOMOTO_MIP_MPSEP_WIDTH_ONLY").is_some() && !self.params.submip && self.snap.is_none() && self.params.restarts == 0 && job.is_none() {
            let w = self.mpsep_width();
            eprintln!(
                "MPSEP_WIDTH {{\"width\": {w}, \"rows\": {}, \"cols\": {}, \"lp_rows\": {}, \"root_lp_secs\": {:.4}, \"nfrac\": {}}}",
                p.m,
                p.n,
                self.lp.num_rows(),
                self.root_lp_secs,
                self.fractional(&self.lp.col_values()).len()
            );
            std::process::exit(0);
        }
        // (多点分離をしたラウンド, 足した点, 足した候補, 選んだカットのうち x* を切らないもの, 選んだカット, 時間)
        let mut mp_stats = (0usize, 0usize, 0usize, 0usize, 0usize, 0.0f64);
        if prof {
            sepprof::PROF.with(|pp| *pp.borrow_mut() = Default::default());
            super::cuts::GEN_PROF.with(|g| *g.borrow_mut() = Default::default());
        }
        // 固定費用ネットワーク型 (連続変数のほぼ全てに VUB `x <= u y` の 2 非零の行があり、その行が大半) では、根のカット選択を
        // 緩める (質の閾値なし、平行度の上限 0.9)。flow cover 系のカットは同じ y の係数が支配的で互いに平行と判定されやすく、
        // 既定の選択では 1 ラウンドに数本しか入らない (sp150x300d: 候補 456 本 -> 31 本、根の下界 56.9 (HiGHS 68.7))。
        // 固定費用型 18 問 (beasleyC1-3, mc7/8/11, h50x2450, h80x6320d, sp150x300d, p200x1188c, aflow30a/40b, rout など、60 秒) で
        // SGM 29.16 -> 27.00、29.09 -> 27.49 (2 回)、解けた問題 9 -> 10。`ENOMOTO_MIP_NO_CUTSEL_FC` で無効
        let fc_net = !self.params.submip && fixed_charge_network(p);
        self.fc_network = fc_net;
        // 試作 `ENOMOTO_MIP_CUTMIX=f` (analysis/cutmix_20261111.md): 緩めの重み (既定では今の 2 択で 1 か 0)
        let fc_w = super::cutmix::fc_weight(p, fc_net);
        let fc_relax = env_str!("ENOMOTO_MIP_NO_CUTSEL_FC").is_none() && fc_w > 0.0;
        if fc_relax && self.params.verbose {
            eprintln!("MIP: fixed-charge network structure: relaxed root cut selection");
        }
        // `ENOMOTO_MIP_CUT_ADAPT_MAX` (実験的): 採ったカットが 1 ラウンドの上限に達したら、次のラウンドの上限を 2 倍にする
        // (`ENOMOTO_T_MIP_CUT_ADAPT_MAX_CAP` 本まで)。同じ形のブロックが多数ある問題 (neos-3381206-awhea: 475 個の箱) では
        // 毎ラウンド候補が数千本あり、50 本ずつでは LP が切られていない別のブロックに移るだけで下界が動かない
        let adapt_max = env_str!("ENOMOTO_MIP_CUT_ADAPT_MAX").is_some() && !self.params.submip;
        let mut adapt_max_cuts = 0usize;
        // `ENOMOTO_MIP_CUT_LP_HALVE` (試作): カットを足した LP が 1 回の LP の時間の上限に当たったら、そのラウンドのカットは今まで通り
        // 外すが、失敗と数えずに、以後のラウンドの本数の上限をそのラウンドの本数の半分にする (`ENOMOTO_T_MIP_CUT_LP_HALVE_MIN` 本 (既定 20)
        // 以下で当たったときだけ失敗と数える)。physiciansched6-2 (4 コアの機械) では、目的値が動かず 1 ラウンド 1000 本までに広げた後の
        // 247 本ほどのカットを足した LP が上限 (60 秒の 2% = 1.2 秒) に当たり、2 回続けて全部外してループが終わり、根の下界が 48724 で
        // 止まっていた (1 回の LP の上限を 6 秒にすると 5 ラウンドで 49324 に届く。analysis/hl4_20261030.md)
        let lp_halve = env_str!("ENOMOTO_MIP_CUT_LP_HALVE").is_some() && !self.params.submip;
        // `ENOMOTO_MIP_BG_CUTS_LP_WORK` = f[h] (既定 `1`、`=0` で使わない。bench18 の canddet として既定にした: analysis/canddet_default_20261102.md。analysis/bg_lp_work_20261102.md): 裏のカットの写しでは、カットの LP 1 回の
        // 上限を秒ではなく反復 (主の根の LP の反復 (`BG_ROOT_ITERS`) の f 倍、少なくとも `ENOMOTO_T_MIP_BG_CUTS_LP_WORK_MIN` (既定 2000)、
        // 最適まで解けたカットの LP の反復の最大の `ENOMOTO_T_MIP_CUT_LP_WORK_PREV` 倍) で決める。時間の上限は今の上限の
        // `ENOMOTO_T_MIP_BG_CUTS_LP_WORK_TIME_MULT` 倍 (既定 5) にして安全網としてだけ残す。写しの 1 回の LP の上限は
        // `max(0.02 x time_limit, 1.0)` 秒で、機械の速さや重みの質で当たり方が変わり、2 回当たると写しのループを抜けていた
        // (遅い機械の physiciansched6-2 で写しが 0 ラウンド、単位重みの neos-960392・neos-933966。analysis/bgx_check1_20261102.md・bgx_check2_20261102.md)。
        // 値の末尾に `h` を付ける (`h` だけなら f = 1) と、写しのカットの LP が上限 (反復・時間) に当たったとき、失敗と数えずに
        // カットを外して以後の本数の上限を半分にして続ける (`ENOMOTO_MIP_CUT_LP_HALVE` と同じ形、`ENOMOTO_T_MIP_CUT_LP_HALVE_MIN` 本以下で当たったら失敗と数える)
        let bg_work: Option<(f64, bool)> = if job.is_some() {
            env_def!("ENOMOTO_MIP_BG_CUTS_LP_WORK", "1").map(|v| {
                let h = v.ends_with('h');
                let f = v.trim_end_matches('h').parse::<f64>().ok().filter(|f| *f > 0.0).unwrap_or(1.0);
                (f, h)
            })
        } else {
            None
        };
        let bg_work_lim0 = bg_work.map_or(0u64, |(f, _)| ((f * super::solver::BG_ROOT_ITERS.with(|c| c.get()).max(100) as f64) as u64).max(tunable!("ENOMOTO_T_MIP_BG_CUTS_LP_WORK_MIN", 2000u64, u64)));
        let bg_halve = bg_work.is_some_and(|w| w.1);
        if let (Some((f, h)), true) = (bg_work, self.params.verbose) {
            eprintln!("MIP: background cut LP work cap: {bg_work_lim0} iters (f {f}, main root LP {} iters){}", super::solver::BG_ROOT_ITERS.with(|c| c.get()), if h { ", halve on cap" } else { "" });
        }
        let lp_halve_min = tunable!("ENOMOTO_T_MIP_CUT_LP_HALVE_MIN", 20usize, usize);
        let mut lp_cap_cuts = usize::MAX;
        // `ENOMOTO_T_MIP_CUT_LP_CAP_MULT` = m (> 0、試作、既定 0 で使わない): 1 回の LP の時間の上限を、このループで最適まで解けた
        // カットの LP の時間の最大の m 倍より短くしない。physiciansched6-2 (4 コアの機械) では、カット 77 本でも LP が 1〜1.5 秒かかり、
        // 上限 1.2 秒 (60 秒の 2%) に当たり続けた (根の LP 自体が 3.4 秒。上限は LP の大きさに合っていない)
        let lp_cap_mult = if self.params.submip { 0.0 } else { tunable!("ENOMOTO_T_MIP_CUT_LP_CAP_MULT", 0.0, f64) };
        let mut ok_lp_secs = 0.0f64;
        // 上限まで採っても目的値が動かないラウンドが来たら、1 ラウンドの上限を行の数 (`ENOMOTO_T_MIP_CUT_WIDE_MAX` 本まで) に
        // 広げ、ラウンドの上限を `ENOMOTO_T_MIP_CUT_WIDE_ROUNDS` 以上にし、それ以降はまったく進まないラウンドも停滞 1 回と数える
        // (`ENOMOTO_MIP_NO_CUT_WIDEN` で無効)。同じ形のブロックが多数ある問題 (neos-3381206-awhea: 容量 100 の箱 475 個) では
        // 毎ラウンド候補が数千本あり、50 本ずつ切っても LP が切られていない別の箱に移るだけで下界が 415.24 から動かず、
        // 停滞判定で 5 ラウンドで止まっていた。下界が上がっている問題では起きないので、上限を一律に広げる
        // (40 問で悪化した) のと違い他の問題は変わらない
        let mut widen_on = env_str!("ENOMOTO_MIP_NO_CUT_WIDEN").is_none() && !self.params.submip && !self.cut_agg_has('w') && !self.rch_part('W');
        let mut widened = false;
        // 既定 (2026-10-30、`ENOMOTO_MIP_CUT_WIDEN_DOUBLE=0` で以前の一気に広げる形): 下の 2 倍ずつ広げる形。58 問 × 2 回 (4 スレッド・60 秒) で
        // SGM -2.0%・-4.5%、解けた数 ±0・+2 (physiciansched6-2・qiu)、解けなくなった問題なし (analysis/halve_submip_20261030.md)
        let widen_double_on = env_def!("ENOMOTO_MIP_CUT_WIDEN_DOUBLE", "1").is_some();
        // `ENOMOTO_MIP_CUT_ADAPT_ROUNDS` (実験的): ラウンドの上限に達したとき、直近 5 ラウンドの目的値の伸びがそれまでの伸びの
        // `ENOMOTO_T_MIP_CUT_ADAPT_ROUNDS_FRAC` 以上なら上限を 25 ラウンド延ばす (`ENOMOTO_T_MIP_CUT_ADAPT_ROUNDS_CAP` まで)
        let adapt_rounds = (env_str!("ENOMOTO_MIP_CUT_ADAPT_ROUNDS").is_some() || ho6_stall_frac().is_some()) && !self.params.submip;
        let adapt_frac = if ho6_stall_frac().is_some() && env_str!("ENOMOTO_MIP_CUT_ADAPT_ROUNDS").is_none() { tunable!("ENOMOTO_T_MIP_HO6_STALL_EXT", 0.01, f64) } else { tunable!("ENOMOTO_T_MIP_CUT_ADAPT_ROUNDS_FRAC", 0.05, f64) };
        let mut obj_hist: Vec<f64> = vec![prev_obj];
        // カットを加える前の根の LP の値 (`ENOMOTO_MIP_CUT_STRONG_AUTO` の判定用)
        let cut_lp0 = prev_obj;
        // `ENOMOTO_T_MIP_CUT_PROG_WIN` = W (> 0): 直近 W ラウンドの目的値の伸びが、ループ全体の伸びの
        // `ENOMOTO_T_MIP_CUT_PROG_EPS` 倍未満なら止める (進み具合による停滞判定。伸びが続く間はラウンド数の上限まで回す)
        let prog_win = if self.params.submip { 0 } else { tunable!("ENOMOTO_T_MIP_CUT_PROG_WIN", 0usize, usize) };
        let prog_eps = tunable!("ENOMOTO_T_MIP_CUT_PROG_EPS", 0.01, f64);
        let mut obj_hist: Vec<f64> = Vec::new();
        // `ENOMOTO_MIP_CUT_STRONG_WIN5` (試作、既定では使わない): `obj_hist` は 1 ラウンドに 2 回積まれる (ラウンドの記録の後と
        // 停滞判定の後) ので、`CUT_STRONG_AUTO` の「直近 5 ラウンド」の伸びは実際には約 2.5 ラウンド分になっている。
        // これを 1 ラウンドに 1 回だけ積む `round_obj` で本当に 5 ラウンド分の伸びにする。beasleyC3 は 25 ラウンド目でも
        // 5 ラウンドで 10〜24 伸びている (閾値 4.7) のに、2.5 ラウンド分 (3.7〜9.4) では判定が回ごとに分かれ、
        // 第 2 段階に入らない回は根の下界が 706〜715 (入る回は 745) で、時間切れの回が出る (analysis/proofvar_20261023.md)
        // 値が `nofc` なら、固定費用のネットワーク (`fc_relax`、カットの選択を緩める問題) では使わない (今までの判定のまま)。
        // mc11 は WIN5 で強いカットの判定が通って第 2 段階 (0.9 -> 3.4 秒) と HiGHS 流のカット管理に入り、背景のカットの
        // 後の根の下界が 11637〜11667 -> 11626〜11639 に下がって時間切れになる (analysis/mc11bisect_20261023.md)。
        // beasleyC3 は最初の根では fc_relax にならない
        let strong_win5 = match env_def!("ENOMOTO_MIP_CUT_STRONG_WIN5", "nofc") {
            Some("nofc") => !fc_relax,
            v => v.is_some(),
        };
        let mut round_obj: Vec<f64> = Vec::new();
        let extend_on = (env_str!("ENOMOTO_MIP_CUT_EXTEND").is_some() || self.cut_agg_has('e')) && !self.params.submip && job.is_none() && self.params.time_limit.is_finite();
        let extend_max = tunable!("ENOMOTO_T_MIP_CUT_EXTEND_MAX", 0.3, f64) * self.params.time_limit;
        CUT_EXTENDED.with(|c| c.set(false));
        // ラウンドごとの目的値 (`ENOMOTO_MIP_CUT_EXTEND` の判定用。最初は根の LP の値)
        let mut ext_hist: Vec<f64> = vec![first_obj];
        self.sep_pass = 0;
        self.sep_last_gain = f64::INFINITY;
        // `ENOMOTO_MIP_CUT_KEEP_LU` (試作、既定では使わない): ラウンドの終わりに効いていないカットを外す
        // ([`Self::remove_inactive_cuts`]) のをやめ、次のラウンドの分離の後、新しいカットを加えるときに一緒に外す
        // ([`MipLp::delete_and_add_rows`]、標準形の作り直しは 1 回)。行を消すと標準形を作り直して最適基底の LU を捨てるので、
        // 次のラウンドの tableau 行の分離 (`basis_inverse_row`) が基底を分解し直していた (カットを加えた後の LP でもう一度分解する)。
        // 外す行は論理変数が基底にあるので、基底逆行列の行のその行の重みは 0 で、tableau 行の集約は外す前と同じ (経路集約の始点にも
        // ならない)。CBC (`CbcModel::solveWithCuts`) も効いていないカットを外すのは新しいカットを加えるときにまとめて行う
        let keep_lu = env_str!("ENOMOTO_MIP_CUT_KEEP_LU").is_some();
        let mut keep_lu_pending = false;
        // `ENOMOTO_MIP_CUT_FAIL_RESTORE` (試作、既定では使わない): カットを加えた LP が最適に解けなかったとき (1 回の LP の時間の上限など)、
        // カットを外した後に、加える前の LP の状態 (最適基底・その LU・DSE 重み、[`MipLp::save_state`]) を戻してから解き直す
        // (今までは基底を加える前のものに戻すだけで LU と DSE 重みを捨て、解き直しが基底の分解からになっていた)
        let fail_restore = env_str!("ENOMOTO_MIP_CUT_FAIL_RESTORE").is_some();
        // `ENOMOTO_MIP_ROOT_CUT_CUTOFF` (試作、既定では使わない): 根のカットの LP に打ち切り値 (刈り込みの上限 - 定数項) を渡し
        // (今までは +inf)、LP の下界が打ち切り値に届いたら (`ObjectiveBound`) 根を閉じたものとしてループを終える
        // ([`Solver::root_gap_closed`] で閉じられるときだけ渡す。外側の求解だけ、背景のカットの求解では使わない)。CBC も根の
        // カットの LP を打ち切り値つきで解く
        let root_cutoff = env_str!("ENOMOTO_MIP_ROOT_CUT_CUTOFF").is_some() && !self.params.submip && job.is_none() && self.snap.is_none();
        let mut closed_by_cutoff = false;
        // 際どい時間の判定に頼らない止め方の試作 (外側の根だけ、既定では使わない。analysis/neos960392_cutstop_20261101.md)。
        // neos-960392 は根の LP の値が最初から最適値で、カットは下界を動かさないのに、3 ラウンド目に入るかどうかが根のカットの
        // 時間の上限 (CPU 時間) の際で決まり、根の頂点 (根の RENS の出発点) が回ごとに変わっていた。
        // - `ENOMOTO_T_MIP_CUT_FLAT_STOP` = k (> 0): 根の LP の値から (相対 `ENOMOTO_T_MIP_CUT_FLAT_EPS`、既定 1e-9 以下しか) 動かない
        //   ラウンドが最初から k 回続いたら止める (一度動いたら数えない。LP を外したラウンドも動かないラウンドと数える。
        //   `ENOMOTO_MIP_CUT_FLAT_NO_FAIL` で数えない)。`intobj_stall` は「一度上がった後に動かない」側を受け持つ
        // - `ENOMOTO_MIP_CUT_LP_CAP_STOP` (既定 `flat`、2026-11-01 に cs2 として既定化。`0` で止める): カットを足した LP が 1 回の LP の上限 (時間・反復) に当たって外したら、そこで止める
        //   (2 回目を待たない)。値が `flat` なら、そこまで根の LP の値から動いていないときだけ
        // - `ENOMOTO_T_MIP_CUT_WORK_CAP` = w (> 0): 根のカットの上限を時間ではなく手間 (このループの LP の反復の合計 / 根の LP の反復
        //   + ラウンドの数 x `ENOMOTO_T_MIP_CUT_WORK_ROUND` (既定 0.05)) で測り、w を超えたら止める。時間の上限は
        //   `ENOMOTO_T_MIP_CUT_WORK_TIME_MULT` 倍 (既定 2) にして安全網として残す
        // - `ENOMOTO_T_MIP_CUT_LP_WORK` = m (> 0、既定 0.1、2026-11-01 に cs2 として既定化。`0` で使わない): 1 回の LP の上限を反復 (m x 根の LP の反復、最低 1000) で決める。時間の上限は
        //   `ENOMOTO_T_MIP_CUT_LP_WORK_TIME_MULT` 倍 (既定 5) にして安全網として残す (下の `ENOMOTO_T_MIP_CUT_LP_WORK_PREV` も見る)
        let cs_outer = !self.params.submip && job.is_none();
        let flat_stop = if cs_outer { tunable!("ENOMOTO_T_MIP_CUT_FLAT_STOP", 0usize, usize) } else { 0 };
        let flat_eps = tunable!("ENOMOTO_T_MIP_CUT_FLAT_EPS", 1e-9, f64);
        let flat_fail = env_str!("ENOMOTO_MIP_CUT_FLAT_NO_FAIL").is_none();
        let mut flat_rounds = 0usize;
        let mut moved = false;
        let lpcap_stop = if cs_outer { env_def!("ENOMOTO_MIP_CUT_LP_CAP_STOP", "flat").filter(|v| *v != "0") } else { None };
        let mut work_cap = if cs_outer { tunable!("ENOMOTO_T_MIP_CUT_WORK_CAP", 0.0, f64) } else { 0.0 };
        let mut work_round = tunable!("ENOMOTO_T_MIP_CUT_WORK_ROUND", 0.05, f64);
        // 試作 `ENOMOTO_MIP_BG_CUTS_DETERM` の `c`: 手間の上限は LP の反復だけで数える ([`Solver::cut_cont`])
        let mut obj_cont: Vec<f64> = Vec::new();
        // 試作 `ENOMOTO_MIP_BG_CUTS_DETERM` の裏のカットのスレッドでは、第 2 段階の時間の上限 (CPU 時間) で止めない (手間の上限で止まる)
        let determ_job = job.as_ref().is_some_and(|j| j.determ.is_some());
        // 試作 `ENOMOTO_MIP_OPTFACE_PARCUTS` の `s`: 他の写しのカットの読んだ位置と、このラウンドで受け取ったカットの印
        let mut share_cursor = 0usize;
        let mut share_keys: std::collections::HashSet<Vec<(usize, u64)>> = std::collections::HashSet::new();
        // `g` (利用者の案の 2 つ目): 渡すのは下界を上げたラウンドの後に LP で効いている (双対値が 0 でない) カットの行だけ、受け取るのは
        // `ENOMOTO_T_MIP_OPTFACE_SHARE_START` (既定 5) ラウンド目からで、直近 `ENOMOTO_T_MIP_OPTFACE_SHARE_STALL` (既定 2) ラウンドの伸びが相対
        // `ENOMOTO_T_MIP_OPTFACE_SHARE_EPS` (既定 1e-6) 以下 (停滞) の写しだけ、1 回に `ENOMOTO_T_MIP_OPTFACE_SHARE_MAX` (`g` では既定 5) 本まで
        let share_gated = env_str!("ENOMOTO_MIP_OPTFACE_PARCUTS").is_some_and(|v| v.contains('g'));
        let mut share_pub: std::collections::HashSet<Vec<(usize, u64)>> = std::collections::HashSet::new();
        let mut race_rep = RaceReport { race: job.as_ref().and_then(|j| j.race.lock().unwrap().clone()), obj: prev_obj };
        let race_rounds = tunable!("ENOMOTO_T_MIP_OPTFACE_MERGE_ROUNDS", 5usize, usize).max(1);
        // `x`: 勝った写しが次のラウンドの候補に足す、負けた写しのカット (自分の LP の解を切るもの)
        let race_x = env_str!("ENOMOTO_MIP_OPTFACE_PARCUTS").is_some_and(|v| v.contains('x'));
        // `y`: 勝った写しは、負けた写しのカットのうち自分の LP のカット (と取り込んだもの) との平行度が `ENOMOTO_T_MIP_OPTFACE_RACE_MAXPAR`
        // (既定 0.5) 以下のものを、自分の解を切るかによらず LP に直接足す (最大 `ENOMOTO_T_MIP_OPTFACE_RACE_MAX` 本)
        let race_y = env_str!("ENOMOTO_MIP_OPTFACE_PARCUTS").is_some_and(|v| v.contains('y'));
        let mut race_extra: Vec<(f64, Vec<(usize, f64)>, f64)> = Vec::new();
        let mut cont_stop = false;
        if let Some((w, _, _)) = self.cut_cont {
            work_cap = w as f64 / root_iters.max(1) as f64;
            work_round = 0.0;
        }
        if work_cap > 0.0 {
            time_cap *= tunable!("ENOMOTO_T_MIP_CUT_WORK_TIME_MULT", 2.0, f64);
        }
        let lp_work = if cs_outer { tunable!("ENOMOTO_T_MIP_CUT_LP_WORK", 0.1, f64) } else { 0.0 };
        // `ENOMOTO_T_MIP_CUT_LP_WORK_PREV` = q (> 0、既定 5、2026-11-01 に cs2 として既定化。`0` で使わない): `ENOMOTO_T_MIP_CUT_LP_WORK` の反復の上限を、このループで最適まで解けたカットの LP の
        // 反復の最大の q 倍より小さくしない (physiciansched6-2 は 0・1 ラウンドの LP が 900・808 反復で、2 ラウンド目の 1166 反復を
        // 上限 1000 で外していた。neos-960392 は 133・126 反復で、2 ラウンド目の密なカットの LP は 3447 反復)
        let lp_work_prev = tunable!("ENOMOTO_T_MIP_CUT_LP_WORK_PREV", 5.0, f64);
        // `ENOMOTO_MIP_CUT_LP_CAP_ROBUST` = 文字の並び [`:m` | `:b`] (既定 `p`、`=0` で使わない。bench20 で既定にした: analysis/robust_cap_default_20261103.md。analysis/robust_cap_20261102.md): 根のカットのループの
        // カットの LP 1 回の反復の上限 (上の `lim`) を、道の変化に強い形にする。上限は「このループで最適まで解けたカットの LP の反復の最大の
        // `ENOMOTO_T_MIP_CUT_LP_WORK_PREV` 倍」より小さくしないので、道が少し変わって短い LP が続いた後に長めの LP が来ると外れる
        // (mc11 で裏のカットの根の LP を厳密な DSE 重みにすると (`ENOMOTO_MIP_BG_CUTS_DSE=g`) 写しのカットの LP が 100〜240 反復で続き、
        // 32・33 ラウンド目が上限 2000 に当たって外され、写しは 32 ラウンドで止まる。analysis/bgdse_grow_20261102.md)。
        // - `s`: 問題の大きさの下限。上限を `ENOMOTO_T_MIP_CUT_LP_CAP_ROBUST_ROWS` (既定 2) x 今の LP の行の数と
        //   `ENOMOTO_T_MIP_CUT_LP_CAP_ROBUST_ROOT` (既定 2) x 根の LP の反復 (写しでは主の根の LP の反復) より小さくしない
        // - `d`: 上限に当たったら、外さずに止まったところから上限を倍にしてもう 1 回だけ続ける (外すのは 2 回目)
        // - `p`: 上限に達した後も、`ENOMOTO_T_MIP_CUT_LP_CAP_ROBUST_WIN` (既定 0.25) x 上限 (最低 50) 反復ごとに双対の目的値を見て、
        //   直近の窓の伸びが出発の目的値からの伸びの `ENOMOTO_T_MIP_CUT_LP_CAP_ROBUST_FRAC` (既定 0.01) 倍未満になるまで続ける
        //   (安全網の上限は `ENOMOTO_T_MIP_CUT_LP_CAP_ROBUST_HARD` (既定 10) x 上限)
        // 末尾の `:m` は主のカットのループだけ、`:b` は裏のカットのスレッド (写し) だけ (既定は両方)。時間の上限 (安全網) は変えない
        let robust = env_def!("ENOMOTO_MIP_CUT_LP_CAP_ROBUST", "p").filter(|_| !self.params.submip).and_then(|v| {
            let (letters, scope) = v.split_once(':').unwrap_or((v, ""));
            let on = match scope {
                "m" => job.is_none(),
                "b" => job.is_some(),
                _ => true,
            };
            on.then(|| (letters.contains('s'), letters.contains('d'), letters.contains('p')))
        });
        let (rob_s, rob_d, rob_p) = robust.unwrap_or((false, false, false));
        // 試作 `ENOMOTO_MIP_HO5_CUTLP_GAIN` (外側の主の根のカットのループだけ)
        let ho5_gain = if cs_outer { ho5_cutlp_gain() } else { 0.0 };
        if robust.is_some() && self.params.verbose {
            eprintln!("MIP: robust cut LP cap{}: size {rob_s}, double {rob_d}, progress {rob_p}", if job.is_some() { " (background)" } else { "" });
        }
        let rob_root = if job.is_some() { super::solver::BG_ROOT_ITERS.with(|c| c.get()) } else { root_iters };
        let mut ok_lp_iters = 0u64;
        // 試作 `ENOMOTO_MIP_CUTLPCAP` = 文字の並び (analysis/cutlpcap_20261107.md、既定では使わない。主の根のカットのループ (`cs_outer`) だけ):
        // - `a`: カットの LP が上限 (反復・時間・進みの判定) に当たったら、全部外す前に、そのラウンドのカットのうち効き目の大きい
        //   半分だけを残して加える前の状態から解き直す (`ENOMOTO_T_MIP_CUTLPCAP_AMIN` 本 (既定 5) 未満になるか最適になるまで繰り返す)。
        //   最適になればそのラウンドは成功として続け、以後の 1 ラウンドの本数の上限を残した本数にする
        // - `n`: 1 ラウンドに入れる本数 (選択を通さないカットも含む) を、0 ラウンド目は `ENOMOTO_T_MIP_CUTLPCAP_N0` 本 (既定 100)、
        //   以後は前に最適まで解けたラウンドの本数の `ENOMOTO_T_MIP_CUTLPCAP_NMULT` 倍 (既定 2) と N0 の大きい方までに、効き目の順に絞る
        // - `c`: 反復の上限を `ENOMOTO_T_MIP_CUTLPCAP_CMULT` (既定 1) x max(根の LP の反復, 前処理の LP の反復, 今の LP の行の数) より小さくしない
        // 値に `l` を含むか `ENOMOTO_MIP_CUTLPCAP_LOG` を置くと、ラウンドごとの上限と本数を出す (診断。動きは変えない)
        let cl = if cs_outer { env_def!("ENOMOTO_MIP_CUTLPCAP", "ac").filter(|v| *v != "0") } else { None };
        let cl_a = cl.is_some_and(|v| v.contains('a'));
        let cl_n = cl.is_some_and(|v| v.contains('n'));
        let cl_c = cl.is_some_and(|v| v.contains('c'));
        let cl_log = cs_outer && self.params.verbose && (cl.is_some_and(|v| v.contains('l')) || env_str!("ENOMOTO_MIP_CUTLPCAP_LOG").is_some());
        let mut cl_last_ok_cuts = 0usize;
        let mut loop_iters = 0u64;
        let mut work_rounds = 0usize;
        // `ENOMOTO_MIP_LOSE6_3_CUTRESTART` (試作、既定では使わない): 主の根のカットのループで、`ENOMOTO_T_MIP_LOSE6_3_CUTRESTART_MIN` (既定 3)
        // ラウンド目以降、暫定解による被約費用の固定が再スタートの判定を通るなら ([`Solver::lose6_3_cut_restart_due`])、ループを抜けて
        // 根のヒューリスティクスを飛ばし早めの再スタートへ進む (HiGHS は根の分離のラウンドごとに被約費用で固定し、再スタートを判定する)。
        // 値の文字: `h` = 根のヒューリスティクスは飛ばさない、`w` = 抜けた後に裏のカットを待たない
        let l63_cut = !self.params.submip && job.is_none() && self.snap.is_none() && env_str!("ENOMOTO_MIP_LOSE6_3_CUTRESTART").is_some() && super::early_restart_on();
        let l63_min = tunable!("ENOMOTO_T_MIP_LOSE6_3_CUTRESTART_MIN", 3usize, usize);
        loop {
            self.poll_fj_thread();
            if self.root_gap_closed() {
                break;
            }
            if l63_cut && round_ctr >= l63_min && !self.early_restart {
                if let Some(w) = self.lose6_3_cut_restart_due() {
                    // 値に `h` があれば根のヒューリスティクスは飛ばさない (その中の早めの再スタートの判定に任せる)
                    if !env_str!("ENOMOTO_MIP_LOSE6_3_CUTRESTART").is_some_and(|v| v.contains('h')) {
                        self.early_restart = true;
                        self.er_at.get_or_insert_with(std::time::Instant::now);
                    }
                    self.l63_cut_left = true;
                    if self.params.verbose {
                        eprintln!(
                            "MIP: early restart: {w} integer columns fixable by reduced costs at cut round {round_ctr} (newly fixed {}), leaving the cut loop (ENOMOTO_MIP_LOSE6_3_CUTRESTART, {:.2}s)",
                            self.newly_fixed_ints(),
                            self.start.elapsed().as_secs_f64()
                        );
                    }
                    break;
                }
            }
            if round_ctr >= max_rounds && cont_p2 > 0 {
                break;
            }
            if round_ctr >= max_rounds {
                let k = obj_hist.len();
                let extend = adapt_rounds
                    && max_rounds < tunable!("ENOMOTO_T_MIP_CUT_ADAPT_ROUNDS_CAP", 100usize, usize)
                    && k > 5
                    && obj_hist[k - 1] - obj_hist[k - 6] >= adapt_frac * (obj_hist[k - 1] - obj_hist[0]).max(1e-9);
                // `ENOMOTO_MIP_CUTSTALL_GAP`: 直近 5 ラウンドの伸びが新しい尺度で停滞でなければ 25 ラウンド延ばす
                let extend = extend
                    || cs_gap.is_some_and(|(t, ext)| {
                        let kr = round_obj.len();
                        ext && max_rounds < tunable!("ENOMOTO_T_MIP_CUTSTALL_MAX_ROUNDS", 100usize, usize) && kr > 5 && {
                            let o = round_obj[kr - 1];
                            o - round_obj[kr - 6] > 5.0 * t * cs_scale(self, o)
                        }
                    });
                // `ENOMOTO_MIP_CUT_PHASE2_AT_CAP`: 第 1 段階がラウンドの上限に当たって止まるときは (停滞していなくても)、
                // 第 2 段階 (HiGHS のスコア) へ進む (`ENOMOTO_MIP_CUT_PHASE2` がなくても。停滞で止まるときは既定のまま止める)。
                // timtab1 は既定の選択で 25 ラウンド目まで毎回 1-3% 伸び、停滞の判定に届かないので、`ENOMOTO_MIP_CUT_PHASE2`・
                // `ENOMOTO_MIP_HIGHS_CUTMGMT` でも第 2 段階に入らない (根の下界 495790、第 2 段階に入ると 598k-602k、HiGHS 569k-610k)
                let to_phase2 = !extend && !phase2 && !self.params.submip && env_str!("ENOMOTO_MIP_CUT_PHASE2_AT_CAP").is_some();
                // `ENOMOTO_MIP_CUT_STRONG_AUTO` (既定で使う。`ENOMOTO_MIP_NO_CUT_STRONG_AUTO` で止める): 第 1 段階がラウンドの上限に当たったとき、カットが根の LP の値を大きく
                // (`ENOMOTO_T_MIP_CUT_STRONG_FRAC` (既定 0.3) × max(|LP|, |今の値|) 以上) 上げていて、直近 5 ラウンドもまだ全体の伸びの
                // `ENOMOTO_T_MIP_CUT_STRONG_RECENT` (既定 0.01) 以上伸びているなら、第 2 段階へ進み (`CUT_PHASE2_AT_CAP` と同じ)、
                // 以後 HiGHS 流のカット管理 (`ENOMOTO_MIP_HIGHS_CUTMGMT`) を有効にする (値が `cap` なら第 2 段階だけ)。
                // timtab1 は 163277 -> 495790 (25 ラウンド目まで毎回伸びる)。40 問で上限に当たるほかの問題の伸びは 1 割未満
                let strong_env = env_str!("ENOMOTO_MIP_CUT_STRONG_AUTO");
                let strong_on = strong_env.is_some() || env_str!("ENOMOTO_MIP_NO_CUT_STRONG_AUTO").is_none();
                let (hist, kh) = if strong_win5 { (&round_obj, round_obj.len()) } else { (&obj_hist, k) };
                let strong = !extend && !phase2 && !self.params.submip && kh > 5 && strong_on && {
                    // 大きさは元の問題の目的値 (前処理の定数項を足した値) で測る
                    let (z0, z1) = (cut_lp0 + p.offset, hist[kh - 1] + p.offset);
                    let gain = z1 - z0;
                    z0.is_finite()
                        && gain >= tunable!("ENOMOTO_T_MIP_CUT_STRONG_FRAC", 0.3, f64) * z0.abs().max(z1.abs()).max(1.0)
                        && hist[kh - 1] - hist[kh - 6] >= tunable!("ENOMOTO_T_MIP_CUT_STRONG_RECENT", 0.01, f64) * gain
                };
                if strong {
                    if strong_env.is_none_or(|v| v != "cap") {
                        self.hcm_auto = true;
                    }
                    if self.params.verbose {
                        eprintln!("MIP: strong cuts at the round limit: LP {:.6e} -> {:.6e}, HiGHS cut management {}", cut_lp0 + p.offset, obj_hist[k - 1] + p.offset, self.hcm_auto);
                    }
                }
                let to_phase2 = to_phase2 || strong;
                if to_phase2 {
                    phase2 = true;
                    stall = 0;
                    max_rounds = round_ctr + tunable!("ENOMOTO_T_MIP_CUT_PHASE2_ROUNDS", 60usize, usize);
                    let t1 = super::cap_clock() - t_loop0;
                    phase2_deadline = t1 + (tunable!("ENOMOTO_T_MIP_CUT_PHASE2_TIME_MULT", 3.0, f64) * t1).max(0.2);
                    if self.params.verbose {
                        eprintln!("MIP: cut loop phase 2 (HiGHS cut score) at the round limit {round_ctr} ({:.2}s)", self.start.elapsed().as_secs_f64());
                    }
                } else if !extend {
                    break;
                } else {
                    max_rounds += 25;
                    if self.params.verbose {
                        eprintln!("MIP: cut loop extended to {max_rounds} rounds");
                    }
                }
            }
            let round = round_ctr;
            round_ctr += 1;
            // 試作 `ENOMOTO_MIP_OPTFACE_PARCUTS` の `k`: `ENOMOTO_T_MIP_OPTFACE_MERGE_ROUNDS` (既定 5) ラウンドで下界を届けて全部の写しを待ち、
            // 一番高い写しだけが写し全部の分のスレッドで続ける (負けた写しはここで止まる)
            race_rep.obj = prev_obj;
            if round == race_rounds {
                if let Some((r, id)) = race_rep.race.take() {
                    // `x`・`y`: 届ける前に自分の LP のカットの行を置き場に入れる
                    if race_x || race_y {
                        let mut v = r.rows.lock().unwrap();
                        for i in self.p.m..self.lp.num_rows() {
                            let (_, u) = self.lp.row_bounds(i);
                            if u.is_finite() {
                                v.push((id, self.lp.row(i), u));
                            }
                        }
                    }
                    r.report(id, prev_obj);
                    let tw = std::time::Instant::now();
                    let w = loop {
                        let w = r.winner.load(std::sync::atomic::Ordering::SeqCst);
                        if w >= 0 || stopped(&job) || self.time_up() {
                            break w;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(2));
                    };
                    if self.params.verbose {
                        eprintln!("MIP: cut round {round}: background copy {id}: obj {:.10e}, winner {w} (waited {:.3}s)", prev_obj + p.offset, tw.elapsed().as_secs_f64());
                    }
                    if w != id as isize {
                        break;
                    }
                    if let Some(j) = &job {
                        j.boost.store(r.total_threads, std::sync::atomic::Ordering::Relaxed);
                    }
                    // `x`: 負けた写しのカットのうち、勝った写し (自分) の今の LP の解を切るものを、効き目の大きい順に
                    // `ENOMOTO_T_MIP_OPTFACE_RACE_MAX` (既定 200) 本まで次のラウンドの候補に足す (選ぶのは既定の選び方)
                    if race_x {
                        let xw = self.lp.col_values();
                        let own: std::collections::HashSet<Vec<(usize, u64)>> = (self.p.m..self.lp.num_rows()).map(|i| self.lp.row(i).iter().map(|&(j, a)| (j, a.to_bits())).collect()).collect();
                        let v = r.rows.lock().unwrap();
                        let mut seen = std::collections::HashSet::new();
                        let (mut nall, mut nviol) = (0usize, 0usize);
                        for (from, c, rhs) in v.iter() {
                            if *from == id {
                                continue;
                            }
                            nall += 1;
                            let k: Vec<(usize, u64)> = c.iter().map(|&(j, a)| (j, a.to_bits())).collect();
                            if own.contains(&k) || !seen.insert(k) {
                                continue;
                            }
                            let act: f64 = c.iter().map(|&(j, a)| a * xw[j]).sum();
                            let norm = c.iter().map(|&(_, a)| a * a).sum::<f64>().sqrt();
                            let eff = (act - rhs) / norm.max(1e-300);
                            if act - rhs > 1e-6 * (1.0 + rhs.abs()) && eff >= 1e-5 {
                                nviol += 1;
                                race_extra.push((eff, c.clone(), *rhs));
                            }
                        }
                        race_extra.sort_by(|a, b| b.0.total_cmp(&a.0));
                        race_extra.truncate(tunable!("ENOMOTO_T_MIP_OPTFACE_RACE_MAX", 200usize, usize));
                        if let Some(j) = &job {
                            j.share_in.fetch_add(race_extra.len(), std::sync::atomic::Ordering::Relaxed);
                        }
                        if self.params.verbose || env_str!("ENOMOTO_MIP_OPTFACE_RACE_LOG").is_some() {
                            eprintln!("MIP: cut round {round}: background copy {id} won: {nall} cut rows of the other copies, {nviol} cut off this LP point, {} added to the candidates", race_extra.len());
                        }
                    }
                    if race_y {
                        let maxpar = tunable!("ENOMOTO_T_MIP_OPTFACE_RACE_MAXPAR", 0.5, f64);
                        let lim = tunable!("ENOMOTO_T_MIP_OPTFACE_RACE_MAX", 200usize, usize);
                        let mut have: Vec<Vec<(usize, f64)>> = (self.p.m..self.lp.num_rows()).map(|i| self.lp.row(i)).collect();
                        // `x` で候補にしたカットは除く
                        let cand_keys: std::collections::HashSet<Vec<(usize, u64)>> = race_extra.iter().map(|(_, c, _)| c.iter().map(|&(j, a)| (j, a.to_bits())).collect()).collect();
                        let v = r.rows.lock().unwrap();
                        let mut add: Vec<(Vec<(usize, f64)>, f64, f64)> = Vec::new();
                        let mut nall = 0usize;
                        for (from, c, rhs) in v.iter() {
                            if *from == id || add.len() >= lim {
                                continue;
                            }
                            nall += 1;
                            let k: Vec<(usize, u64)> = c.iter().map(|&(j, a)| (j, a.to_bits())).collect();
                            if cand_keys.contains(&k) {
                                continue;
                            }
                            if have.iter().all(|h| cut_parallelism(c, h) <= maxpar) {
                                have.push(c.clone());
                                add.push((c.clone(), f64::NEG_INFINITY, *rhs));
                            }
                        }
                        drop(v);
                        let mut st_ok = true;
                        if !add.is_empty() {
                            let m_before = self.lp.num_rows();
                            for (c, _, rr) in &add {
                                self.add_to_pool(c, *rr);
                            }
                            self.add_cut_rows(&add);
                            let st = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
                            if st != LpStatus::Optimal {
                                // 解けなければ足した行を外して解き直す
                                let mut remove = vec![false; self.lp.num_rows()];
                                for rr in remove.iter_mut().skip(m_before) {
                                    *rr = true;
                                }
                                self.delete_lp_rows(&remove);
                                let _ = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
                                st_ok = false;
                            }
                        }
                        if self.params.verbose || env_str!("ENOMOTO_MIP_OPTFACE_RACE_LOG").is_some() {
                            eprintln!(
                                "MIP: cut round {round}: background copy {id} won: {nall} cut rows of the other copies, {} with parallelism <= {maxpar} added to the LP{}, obj {:.10e} -> {:.10e}",
                                add.len(),
                                if st_ok { "" } else { " (LP failed; removed)" },
                                prev_obj + p.offset,
                                self.lp.objective() + p.offset
                            );
                        }
                        if st_ok && !add.is_empty() {
                            if let Some(j) = &job {
                                j.share_used.fetch_add(add.len(), std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                    }
                }
            }
            // 試作 `ENOMOTO_MIP_OPTFACE_PARCUTS` の `m`: 裏のカットの写しは決まったラウンド数だけ回して止まる (主がまとめて続ける)
            if let Some(cap) = job.as_ref().map(|j| j.round_cap.load(std::sync::atomic::Ordering::Relaxed)).filter(|&c| c > 0) {
                if round >= cap {
                    if self.params.verbose {
                        eprintln!("MIP: cut round {round}: background cuts: round cap {cap} reached (OPTFACE_PARCUTS m); stop");
                    }
                    break;
                }
            }
            // `ENOMOTO_MIP_CUTSEL_PROBE` (既定 `rate`、`0` で外す。計測 23 で既定化): 外側の最初の根 (再スタートの前) の 0 ラウンド目の始め (時間の上限の判定の前)
            // で、既定の選び方 (50 本) と POOL の選び方 (HiGHS の点数、1000 本・平行度 0.1) のそれぞれで k ラウンド回して下界の伸びを比べ
            // ([`Solver::probe_cut_mode`]、終わったら LP と分離器の状態を戻す)、伸びの大きい方を以後の回し方 (時間の上限・本数・プールの戻し・
            // 年齢) ごと使う ([`Solver::probe_pool_on`])。既定を選んだら、最低のラウンド数を元に戻し、時間の上限の起点を比べにかかった時間だけ
            // 後ろにずらす (比べがなかったのと同じ判定になる)。固定費用のネットワーク型では比べない (既定のまま)。計測 15 では POOL が roll3000・
            // h80x6320d・binkar10_1・physiciansched6-2 で得、noswot・neos-960392・hypothyroid-k1・dcmulti で損で、どちらも同じ機械 4 回で再現した
            // (analysis/highstree_phys_20261024.md §9・§10)
            if round == 0 && probe_pending {
                let c0 = super::cap_clock();
                let k = tunable!("ENOMOTO_T_MIP_CUTSEL_PROBE_ROUNDS", 6usize, usize).max(1);
                let rule = env_def!("ENOMOTO_MIP_CUTSEL_PROBE", "rate").unwrap_or("1");
                let tol = 1e-6 * self.lp.objective().abs().max(1.0);
                // 早い打ち切り (`rate` のとき): 時間あたりの伸びの比が `ENOMOTO_T_MIP_CUTSEL_PROBE_RATIO` (3) 倍を超えるか、両方とも伸びが 0 なら
                // 決める。比べの合計時間の上限は全体の `ENOMOTO_T_MIP_CUTSEL_PROBE_TIME_FRAC` (0.05)。既定 3 (計測 23 で既定化)、0 で打ち切らない (§10 の形)
                let ratio = tunable!("ENOMOTO_T_MIP_CUTSEL_PROBE_RATIO", 3.0, f64);
                let tfrac = tunable!("ENOMOTO_T_MIP_CUTSEL_PROBE_TIME_FRAC", if ratio > 0.0 { 0.05 } else { 0.0 }, f64);
                let budget = if tfrac > 0.0 && self.params.time_limit.is_finite() { tfrac * self.params.time_limit } else { f64::INFINITY };
                let t_probe0 = std::time::Instant::now();
                let deadline = t_probe0 + std::time::Duration::from_secs_f64(budget.min(1e6));
                let rate = |g: f64, t: f64| g.max(0.0) / t.max(1e-3);
                let decisive = |a: (f64, f64), b: (f64, f64)| -> bool {
                    if ratio <= 0.0 {
                        return false;
                    }
                    if a.0 <= tol && b.0 <= tol {
                        return true;
                    }
                    let (ra, rb) = (rate(a.0, a.1), rate(b.0, b.1));
                    ra.max(rb) > ratio * ra.min(rb).max(1e-12)
                };
                let never = |_: usize, _: f64, _: f64| false;
                let mut early = "";
                // `ENOMOTO_MIP_CUTSEL_PROBE_KEEP` (試作、既定では使わない): 比べの両方の側の終わりの状態を取っておき、勝った側の状態 (足したカット・
                // LP の基底・分離器の状態) からループを続ける (比べで回ったラウンドはラウンドの数と時間の上限の勘定に入れる)。負けた側のカットは
                // LP には入れず、カットのプールに入れる。最初の 1 ラウンドの比べの後に k ラウンドで比べ直したときは、比べ直しの終わりの状態を使う
                // (1 ラウンドの比べのカットは両方の側ともプールに入れる)
                let keep = env_str!("ENOMOTO_MIP_CUTSEL_PROBE_KEEP").is_some();
                let mut to_pool: Vec<(Vec<(usize, f64)>, f64)> = Vec::new();
                let (mut hd, mut nd, mut ed) = self.probe_cut_mode(false, if ratio > 0.0 { 1 } else { k }, deadline, &never, keep);
                let (mut hp, mut np, mut ep) = self.probe_cut_mode(true, if ratio > 0.0 { 1 } else { k }, deadline, &never, keep);
                if ratio > 0.0 && k > 1 {
                    let first_ok = !hd.is_empty() && !hp.is_empty();
                    if first_ok && decisive(hd[0], hp[0]) {
                        early = "after round 1";
                    } else if std::time::Instant::now() >= deadline {
                        early = "time budget";
                    } else if first_ok && lose6_2_tie() && (hd[0].0 - hp[0].0).abs() <= tol + 1e-9 * hd[0].0.abs().max(hp[0].0.abs()) {
                        // 試作 `ENOMOTO_MIP_LOSE6_2_TIE` ([`lose6_2_tie`]): 1 ラウンド目の伸びが同じなら、両方を同じラウンド数
                        // (`ENOMOTO_T_MIP_LOSE6_2_TIE_ROUNDS`、既定 2) だけ回して比べ直す (時間の予算は比べの始めから元の
                        // `ENOMOTO_T_MIP_LOSE6_2_TIE_BUDGET` (既定 4) 倍)
                        let r2 = tunable!("ENOMOTO_T_MIP_LOSE6_2_TIE_ROUNDS", 2usize, usize).clamp(2, k);
                        let bmult = tunable!("ENOMOTO_T_MIP_LOSE6_2_TIE_BUDGET", 4.0, f64).max(1.0);
                        let dl2 = t_probe0 + std::time::Duration::from_secs_f64((bmult * budget).min(1e6));
                        LOSE6_2_PROBE_CAP_MULT.with(|c| c.set(bmult));
                        let (hd2, nd2, ed2) = self.probe_cut_mode(false, r2, dl2, &never, keep);
                        let (hp2, np2, ep2) = self.probe_cut_mode(true, r2, dl2, &never, keep);
                        LOSE6_2_PROBE_CAP_MULT.with(|c| c.set(1.0));
                        for e in [ed.take(), ep.take()].into_iter().flatten() {
                            to_pool.extend(e.rows.into_iter().map(|(c, _, r)| (c, r)));
                        }
                        (hd, nd, hp, np, ed, ep) = (hd2, nd2, hp2, np2, ed2, ep2);
                        early = if hd.len().min(hp.len()) < r2 { "tie, stopped early" } else { "tie" };
                    } else {
                        let (hd2, nd2, ed2) = self.probe_cut_mode(false, k, deadline, &never, keep);
                        let hd_ref = hd2.clone();
                        let stop = move |r: usize, g: f64, t: f64| r < hd_ref.len() && decisive(hd_ref[r], (g, t));
                        let (hp2, np2, ep2) = self.probe_cut_mode(true, k, deadline, &stop, keep);
                        for e in [ed.take(), ep.take()].into_iter().flatten() {
                            to_pool.extend(e.rows.into_iter().map(|(c, _, r)| (c, r)));
                        }
                        (hd, nd, hp, np, ed, ep) = (hd2, nd2, hp2, np2, ed2, ep2);
                        early = if hp.len() < k { "pool stopped early" } else { "full" };
                    }
                }
                // 両方がそろっている同じラウンド数の値で比べる
                let m = hd.len().min(hp.len());
                let (d_def, t_def) = if m > 0 { hd[m - 1] } else { (0.0, 0.0) };
                let (d_pool, t_pool) = if m > 0 { hp[m - 1] } else { (0.0, 0.0) };
                let (n_def, n_pool) = (nd, np);
                // `ENOMOTO_T_MIP_CUTSEL_PROBE_MIN_GAIN` = g (既定 1e-6、計測 23 で既定化。0 で外す): POOL の伸びが g x max(1, |根の LP|) 以下なら既定を選ぶ
                // (drayage-25-23 では両方の伸びが ±1.5e-11 で、`rate` が誤差で POOL を選び 10 -> 26 秒 (ALT=NS のとき) になっていた)
                let min_gain = tunable!("ENOMOTO_T_MIP_CUTSEL_PROBE_MIN_GAIN", 1e-6, f64) * self.lp.objective().abs().max(1.0);
                let pick_pool = d_pool > min_gain
                    && match rule {
                        // 時間あたりの伸び
                        "rate" => d_pool / t_pool.max(1e-3) > d_def / t_def.max(1e-3) + 1e-12,
                        _ => d_pool > d_def + tol,
                    };
                if self.params.verbose && ratio > 0.0 {
                    eprintln!("MIP: cut selection probe: compared at {m} rounds ({early}), probe wall {:.2}s (budget {budget:.1}s)", t_probe0.elapsed().as_secs_f64());
                }
                CUT_PROBE_CHOICE.store(if pick_pool { 2 } else { 1 }, std::sync::atomic::Ordering::Relaxed);
                if self.params.verbose {
                    eprintln!("MIP: cut selection probe ({k} rounds): default {n_def} cuts gain {d_def:.6e} ({t_def:.2}s), pool {n_pool} cuts gain {d_pool:.6e} ({t_pool:.2}s) -> {} (rule {rule}, probe {:.2}s in total, {:.2}s)", if pick_pool { "pool" } else { "default" }, t_def + t_pool, self.start.elapsed().as_secs_f64());
                }
                if pick_pool {
                    // 回し方も POOL に合わせる (ループの始めに決めた時間の上限の数え方・最低のラウンド数・広げる規則)
                    // (`ENOMOTO_MIP_CUTSEL_PROBE_ALT` では挙げた部品だけ)
                    if self.rch_part('T') {
                        cap_base = t_loop0;
                        min_rounds = tunable!("ENOMOTO_T_MIP_CUT_MIN_ROUNDS", 10usize, usize);
                    } else {
                        min_rounds = min_rounds0;
                        if !keep {
                            cap_base += super::cap_clock() - c0;
                        }
                    }
                    if self.rch_part('W') {
                        widen_on = false;
                    }
                } else {
                    min_rounds = min_rounds0;
                    if !keep {
                        cap_base += super::cap_clock() - c0;
                    }
                }
                if keep {
                    let (win, lose) = if pick_pool { (ep, ed) } else { (ed, ep) };
                    if let Some(e) = lose {
                        to_pool.extend(e.rows.into_iter().map(|(c, _, r)| (c, r)));
                    }
                    for (c, r) in &to_pool {
                        self.add_to_pool(c, *r);
                    }
                    if let Some(e) = win {
                        let (nr, nrows) = (e.rounds, e.rows.len());
                        self.probe_restore(e);
                        round_ctr += nr;
                        if self.params.verbose {
                            eprintln!("MIP: cut selection probe: kept the {} side ({nr} rounds, {nrows} rows, LP {:.10e}); {} cuts of the other side to the pool", if pick_pool { "pool" } else { "default" }, self.lp.objective(), to_pool.len());
                        }
                    }
                }
            }
            // 試作 `ENOMOTO_MIP_CLIQUE_CUTS=probe` (課題 CLIQFB): 0 ラウンド目の始め (カットの選び方の比べの後) に、クリークカットの
            // あり・なしで k ラウンドずつ回して比べ、勝った側の終わりの状態からループを続ける ([`Self::clique_probe`])
            if round == 0 && clique_probe_pending && job.is_none() && self.snap.is_none() && self.cut_cont.is_none() && !self.time_up() {
                clique_probe_pending = false;
                let c0 = super::cap_clock();
                let (on, nr) = self.clique_probe();
                // `probe_r` で使わない (比べの状態を捨てた) ときは、時間の上限の起点を比べにかかった時間だけ後ろにずらす (比べがなかったのと同じ判定)
                if !on && nr == 0 && clique_cuts_mode() == Some("probe_r") {
                    cap_base += super::cap_clock() - c0;
                }
                if self.params.restarts == 0 {
                    CLIQUE_AUTO_FIRST.with(|c| c.set(Some(on)));
                }
                round_ctr += nr;
            }
            let now = super::cap_clock();
            let now = if env_str!("ENOMOTO_MIP_SEP_COUNT_CPU").is_some() { now + self.sep_par_cpu } else { now };
            // `ENOMOTO_MIP_CUT_EXTEND` (試作、既定では使わない): 根のカットの時間の上限に来ても、直近 3 ラウンドの目的値の伸びが
            // (3 ラウンドに満たなければ回った分) |目的値| の `ENOMOTO_T_MIP_CUT_EXTEND_MIN_GAIN` 倍 (既定 1e-3) 以上なら、上限を全体の `ENOMOTO_T_MIP_CUT_EXTEND_STEP`
            // (0.05) ずつ、合わせて `ENOMOTO_T_MIP_CUT_EXTEND_MAX` (0.3) まで延ばす。延ばした後は面の頂点での分離 (LP を 2 回解く)
            // を止める。また最初から、1 回の LP の時間の上限で止まった LP は外さずに解き続ける (`ENOMOTO_MIP_CUT_LP_CONTINUE` と同じ)。
            // physiciansched6-2 (同値の代入の後) では根のカットが 1 ラウンド 1〜2 秒で 48587 -> 49124 (10 ラウンド) と伸び続けるが、
            // 既定の上限 (全体の 10%) で 1〜2 ラウンドで止まっていた
            if extend_on && now - cap_base > time_cap && !self.time_up() && !phase2 && !stopped(&job) && time_cap + 1e-9 < extend_max {
                let k = ext_hist.len();
                let obj = self.lp.objective();
                // `ENOMOTO_T_MIP_CUT_EXTEND_WIN` (既定 3): 伸びを見るラウンドの数。physiciansched6-2 では 2〜3 ラウンド伸びない平らな所
                // (48633・48757) の後でまた伸びる (HiGHS も同じ) ので、3 ラウンドでは平らな所で止まる回がある
                let w = (k.saturating_sub(1)).min(tunable!("ENOMOTO_T_MIP_CUT_EXTEND_WIN", 3usize, usize).max(1));
                if w >= 1 && ext_hist[k - 1] - ext_hist[k - 1 - w] >= tunable!("ENOMOTO_T_MIP_CUT_EXTEND_MIN_GAIN", 1e-3, f64) * obj.abs().max(1.0) {
                    let step = tunable!("ENOMOTO_T_MIP_CUT_EXTEND_STEP", 0.05, f64) * self.params.time_limit;
                    // `ENOMOTO_MIP_CUT_EXTEND_CATCHUP` (試作、既定では使わない): 延ばした上限が今の経過に届かなければ、経過 + 1 段まで
                    // 延ばす (上限 `ENOMOTO_T_MIP_CUT_EXTEND_MAX` は同じ)。physiciansched6-2 (`ENOMOTO_MIP_CUT_AGG`) では、ときどき 1 ラウンドの
                    // LP が 7000 反復・6 秒と重くなり、伸びている判定が通って 1 段 (3 秒) 延ばしても経過がそれを超えたままで、ループが
                    // 48757 で止まる回があった (延ばさずに続いた回は 49099〜49324。analysis/physprimal_20261024.md)
                    let catchup = if env_str!("ENOMOTO_MIP_CUT_EXTEND_CATCHUP").is_some() { now - cap_base + step } else { 0.0 };
                    time_cap = (time_cap + step).max(catchup).min(extend_max);
                    CUT_EXTENDED.with(|c| c.set(true));
                    if self.params.verbose {
                        eprintln!("MIP: cut round {round}: still improving ({:.6e} in {w} rounds); root cut time cap extended to {time_cap:.1}s", ext_hist[k - 1] - ext_hist[k - 1 - w]);
                    }
                }
            }
            // 診断のログ (振る舞いは変えない): 根のカットの時間の上限の判定に使う時計 (このスレッドの CPU 時間) と上限。
            // neos-960392 では 2 ラウンド目の後の判定が 6.0 秒の上限の際にあり、3 ラウンド目を回すかどうかが回ごとに変わって、
            // 根の RENS の出発点の頂点 (最適値を出すか) を分けていた (analysis/neos960392_20261101.md の 1 節)
            if self.params.verbose && !self.params.submip && job.is_none() && round > 0 {
                eprintln!(
                    "MIP: cut round {round}: root cut time cap check: clock {:.3}s of cap {time_cap:.2}s (min rounds {min_rounds}){}",
                    now - cap_base,
                    if now - cap_base > time_cap && round >= min_rounds { " -> stop" } else { "" }
                );
            }
            if self.time_up() || (now - cap_base > time_cap && round >= min_rounds) || (phase2 && now - t_loop0 > phase2_deadline && !heavy && !determ_job) || stopped(&job) || cont_stop {
                break;
            }
            // `ENOMOTO_T_MIP_CUT_WORK_CAP`: 手間の上限
            if work_cap > 0.0 && round >= min_rounds {
                let work = loop_iters as f64 / root_iters.max(1) as f64 + work_round * work_rounds as f64;
                if self.params.verbose && round > 0 {
                    eprintln!("MIP: cut round {round}: root cut work cap check: work {work:.3} ({loop_iters} LP iters, {work_rounds} rounds) of cap {work_cap:.3}{}", if work > work_cap { " -> stop" } else { "" });
                }
                if work > work_cap {
                    break;
                }
            }
            self.bg_cut_rounds = round_ctr;
            // `ENOMOTO_T_MIP_CLIQUE_FB_EARLY` = k (> 0): fallback の判定を、k ラウンド回っても目的値がまったく動かないときは
            // k ラウンド目で行い、使うならこのループの続きで clique カットを加える (drayage-25-23 では clique なしの 6 ラウンドに
            // 6.5 秒かかる)。ここで使わないと決まらなければループの後でもう一度判定する
            let fb_early = tunable!("ENOMOTO_T_MIP_CLIQUE_FB_EARLY", 0usize, usize);
            if clique_fb && fb_early > 0 && round == fb_early && job.is_none() && self.lp.objective() - first_obj <= 1e-9 * self.lp.objective().abs().max(1.0) {
                if self.clique_fallback_trial(root_iters, first_obj) {
                    if self.params.restarts == 0 {
                        CLIQUE_AUTO_FIRST.with(|c| c.set(Some(true)));
                    }
                    self.clique_auto = Some(true);
                    clique_fb = false;
                    stall = 0;
                }
            }
            let x = self.lp.col_values();
            if self.fractional(&x).is_empty() {
                break;
            }
            let t_sep = std::time::Instant::now();
            let t_round0 = t_sep;
            // `ENOMOTO_T_MIP_FACE_MAX_CUTS` = k (> 0): 面の頂点からのカットは別枠で最大 k 本選ぶ (今の頂点のカットとは
            // 競わせない)。0 なら同じ枠で選ぶ。40 問: 0 (既定) 19 問 27.70、10 は 17 問 28.14、25 は 17 問 27.83 (rout・neos5 が
            // 時間切れ、mik-250 47.9 -> 33-40 秒)。既定では使わない
            let face_quota = tunable!("ENOMOTO_T_MIP_FACE_MAX_CUTS", 0usize, usize);
            self.sep_pass = round;
            self.sep_adapt.ran = 0;
            // `ENOMOTO_MIP_SEPSTAT`・`ENOMOTO_MIP_SEPLEARN`: 分離器ごとの計測と、この呼び出しで使う分離器 ([`super::seplearn`])
            let sl_on = self.sl.stat && !self.params.submip && job.is_none();
            if sl_on {
                self.sl.begin_call(false);
            }
            let (mut cands, mut fcands) = self.separate_face_split(&x, false);
            if sl_on {
                // 予算は 1 ラウンドの本数の上限の目安 (正確な上限はこの後で決まる)
                let b = if adapt_max_cuts > 0 { adapt_max_cuts } else { tunable!("ENOMOTO_T_MIP_ROOT_MAX_CUTS", 50usize, usize).min(p.m.max(50)) };
                self.sl_after_separate(&mut [&mut cands, &mut fcands], b);
            }
            // 試作 `ENOMOTO_MIP_MPSEP`: 今の頂点 x* のほかの点 (最適面の上の別の頂点、x* と内点を結ぶ線分上の点) でも分離して候補に足す
            // 足した点の候補は `mp_cands` に分けておく (値に `P` を含まなければカットプールに入れない。`q` でなければ選択の前に候補に混ぜる)
            let mut mp_cands: Vec<Candidate> = Vec::new();
            // 絞った形: `g` は最適面が広い (被約費用 0 の非基底の数 ≥ max(`ENOMOTO_T_MIP_MPSEP_GATE_K` (20), 元の行の数 × `ENOMOTO_T_MIP_MPSEP_GATE_R` (0.5)))
            // ラウンドだけ (広くなくなったら残りは止める)、`s` は前のラウンドで目的値が相対 `ENOMOTO_T_MIP_MPSEP_STALL_EPS` (1e-4) 以下しか上がらなかったラウンドだけ
            let mp_go = match &mpsep {
                Some(f) if !mp_off => {
                    let obj = self.lp.objective();
                    let stalled = mp_last_obj.is_some_and(|o| obj - o <= tunable!("ENOMOTO_T_MIP_MPSEP_STALL_EPS", 1e-4, f64) * obj.abs().max(1.0));
                    mp_last_obj = Some(obj);
                    let mut go = !f.contains('s') || stalled;
                    if go && f.contains('g') {
                        let w = self.mpsep_width();
                        let th = (tunable!("ENOMOTO_T_MIP_MPSEP_GATE_K", 20usize, usize) as f64).max(tunable!("ENOMOTO_T_MIP_MPSEP_GATE_R", 0.5, f64) * p.m as f64);
                        if (w as f64) < th {
                            mp_off = true;
                            go = false;
                        }
                        if self.params.verbose && (mp_off || mp_stats.0 == 0) {
                            eprintln!("MIP: cut round {round}: mpsep gate: optimal face width {w} (threshold {th:.0}, {} rows) -> {}", p.m, if mp_off { "narrow; stop" } else { "wide" });
                        }
                    }
                    go
                }
                _ => false,
            };
            let mp_pts: Vec<Vec<f64>> = match &mpsep {
                Some(f) if mp_go => self.mpsep_extra(f, &x, round, &mut mp_cands, &mut mp_center, &mut mp_center_tried, &mut mp_stats),
                _ => Vec::new(),
            };
            let mp_quota = mpsep.as_ref().is_some_and(|f| f.contains('q'));
            if mpsep.as_ref().is_some_and(|f| f.contains('P')) {
                for c in &mp_cands {
                    self.add_to_pool(&c.coefs, c.rhs);
                }
            }
            // 分離の間に求解の期限を過ぎたら、このラウンドを捨てて抜ける (ラウンドの頭の `time_up` で抜けるのと同じ状態)。
            // 裏のカットの求解のラウンド (rail507 で 1 回 3〜9 秒) の候補のプールへの追加・選択が期限の後も回り、主がその終わりを
            // 待って時間の上限を 5 秒過ぎていた (analysis/overrun_crypt_20261101.md)
            if sep_deadline(self.deadline).is_some_and(|d| std::time::Instant::now() >= d) {
                if self.params.verbose {
                    eprintln!("MIP: cut round {round}: the time limit passed during the separation ({:.2}s); round dropped", t_sep.elapsed().as_secs_f64());
                }
                break;
            }
            // `ENOMOTO_MIP_CUTSEL_FACE_ANCHOR` (実験的): 面の頂点からのカットの効き目はその頂点で測ったもので、今の LP 解は
            // 切らないことがある。それが最良のスコアになると、質の閾値 (最良の 50%) で今の頂点を切るカットが全部落ち、
            // 選んだ面のカットを足しても LP が動かず (効いていないので外され、次のラウンドでまた選ばれる) 停滞判定で止まる
            // (roll3000: 今の解での活動量 0.068 の x818 + x831 <= 1 が効き目 0.38 で毎ラウンド 1 本だけ選ばれる)。
            // 1: 質の閾値の基準 (最良のスコア) を今の頂点の候補だけで決める。2: 加えて面の候補の効き目を今の解で測り直し、
            // 今の解を切らないものは捨てる
            let face_anchor = if self.params.submip || face_quota > 0 { 0 } else { tunable!("ENOMOTO_MIP_CUTSEL_FACE_ANCHOR", 0usize, usize) };
            if face_anchor >= 2 {
                fcands.retain_mut(|c| {
                    let act: f64 = c.coefs.iter().map(|&(j, v)| v * x[j]).sum();
                    let norm = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
                    c.efficacy = (act - c.rhs) / norm.max(1e-300);
                    act - c.rhs > 1e-6 * (1.0 + c.rhs.abs()) && c.efficacy >= 1e-5
                });
            }
            let anchor = if face_anchor >= 1 {
                let cnorm = p.cost.iter().map(|c| c * c).sum::<f64>().sqrt();
                cands.iter().filter(|c| !c.forced).map(|c| cut_score(c, p, cnorm)).fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v))))
            } else {
                None
            };
            let mut face_cands: Vec<Candidate> = Vec::new();
            if face_quota > 0 {
                face_cands = fcands;
            } else {
                cands.extend(fcands);
            }
            // 再スタート前のカットをプールで引き継いだとき (`ENOMOTO_MIP_RESTART_CUTS_TO_POOL`): プールで違反しているものも候補にする
            // `ENOMOTO_MIP_SYM_CUTS`: 前のラウンドのカットの像 (プールにある) で違反しているものも候補にする
            if !sym_pool.is_empty() {
                for (eff, k) in self.pool_violated(&x) {
                    if !sym_pool.contains(&k) {
                        continue;
                    }
                    let (c, r, _) = &self.cut_pool[k];
                    cands.push(Candidate { coefs: c.clone(), rhs: *r, efficacy: eff, forced: false, src: sepprof::SRC_POOL });
                }
            }
            if ((self.params.restarts > 0 && env_str!("ENOMOTO_MIP_RESTART_CUTS_TO_POOL").is_some()) || ((env_str!("ENOMOTO_MIP_CUTSEL_HIGHS_POOL").is_some() || self.pool_part('P')) && !self.cutsel_guard_fc())) && round < tunable!("ENOMOTO_T_MIP_ROOT_POOL_ROUNDS", if self.pool_part('P') { 1000usize } else { 5 }, usize) && env_str!("ENOMOTO_MIP_NO_ROOT_POOL").is_none() {
                for (eff, k) in self.pool_violated(&x) {
                    let (c, r, _) = &self.cut_pool[k];
                    cands.push(Candidate { coefs: c.clone(), rhs: *r, efficacy: eff, forced: false, src: sepprof::SRC_POOL });
                }
            }
            // 試作 `ENOMOTO_MIP_OPTFACE_PARCUTS` の `s`: 他の写し (別の頂点から始めた) が作ったカットを `ENOMOTO_T_MIP_OPTFACE_SHARE_EVERY`
            // (既定 2) ラウンドごとに読み、今の LP の点で破られているものを効き目の大きい順に `ENOMOTO_T_MIP_OPTFACE_SHARE_MAX` (既定 20) 本まで
            // 候補に足す (LP にそのまま入れず、既定の選び方 (効き目・平行度・本数の上限) に任せる)
            share_keys.clear();
            // `x`: 勝ったときに集めた負けた写しのカットを、このラウンドの候補に足す (1 回だけ)
            for (eff, c, r) in race_extra.drain(..) {
                share_keys.insert(c.iter().map(|&(j, a)| (j, a.to_bits())).collect());
                cands.push(Candidate { coefs: c, rhs: r, efficacy: eff, forced: false, src: sepprof::SRC_POOL });
            }
            if let Some((sh, id)) = job.as_ref().and_then(|j| j.share.lock().unwrap().clone()) {
                let every = tunable!("ENOMOTO_T_MIP_OPTFACE_SHARE_EVERY", 2usize, usize).max(1);
                let due = if share_gated {
                    let w = tunable!("ENOMOTO_T_MIP_OPTFACE_SHARE_STALL", 2usize, usize).max(1);
                    let eps = tunable!("ENOMOTO_T_MIP_OPTFACE_SHARE_EPS", 1e-6, f64);
                    let k = obj_hist.len();
                    round >= tunable!("ENOMOTO_T_MIP_OPTFACE_SHARE_START", 5usize, usize) && k > w && obj_hist[k - 1] - obj_hist[k - 1 - w] <= eps * obj_hist[k - 1].abs().max(1.0)
                } else {
                    round > 0 && round % every == 0
                };
                if due {
                    let lim = tunable!("ENOMOTO_T_MIP_OPTFACE_SHARE_MAX", if share_gated { 5usize } else { 20 }, usize);
                    let mut got: Vec<(f64, Vec<(usize, f64)>, f64)> = Vec::new();
                    {
                        let v = sh.cuts.lock().unwrap();
                        for (from, c, r) in v[share_cursor..].iter() {
                            if *from == id {
                                continue;
                            }
                            let act: f64 = c.iter().map(|&(j, a)| a * x[j]).sum();
                            let norm = c.iter().map(|&(_, a)| a * a).sum::<f64>().sqrt();
                            let eff = (act - r) / norm.max(1e-300);
                            if act - r > 1e-6 * (1.0 + r.abs()) && eff >= 1e-5 {
                                got.push((eff, c.clone(), *r));
                            }
                        }
                        share_cursor = v.len();
                    }
                    got.sort_by(|a, b| b.0.total_cmp(&a.0));
                    got.truncate(lim);
                    if let Some(j) = &job {
                        j.share_in.fetch_add(got.len(), std::sync::atomic::Ordering::Relaxed);
                    }
                    for (eff, c, r) in got {
                        let k: Vec<(usize, u64)> = c.iter().map(|&(j, a)| (j, a.to_bits())).collect();
                        // 受け取ったカットは送り返さない (`g`)
                        share_pub.insert(k.clone());
                        share_keys.insert(k);
                        cands.push(Candidate { coefs: c, rhs: r, efficacy: eff, forced: false, src: sepprof::SRC_POOL });
                    }
                }
            }
            // 試作 `ENOMOTO_MIP_MPSEP`: 候補の効き目を、x* だけでなく全部の点での効き目 (最大、値に `u` を含めば正の部分の和) にする。
            // x* を切る候補の最良の点数を質の閾値の基準にする (x* を切らない候補だけが採られて LP が動かなくなるのを避ける)
            let mut mp_anchor: Option<f64> = None;
            // `q`: x* での既定の選択はそのまま (候補も効き目も変えない) にし、別枠で、全部の点での効き目の順に
            // `ENOMOTO_T_MIP_MPSEP_EXTRA` (既定 10) 本まで足す候補 (足した点の候補と、x* の候補の全部。効き目は全部の点での値)
            let mut mp_extra_pool: Vec<Candidate> = Vec::new();
            // (点を足さなかったラウンドでは何もしない。前の版は `q` のとき点がなくても x* の候補から別枠を足していた)
            if mp_pts.is_empty() {
                // 何もしない
            } else if mp_quota {
                mp_extra_pool = std::mem::take(&mut mp_cands);
                mp_extra_pool.extend(cands.iter().filter(|c| !c.forced).cloned());
            } else {
                cands.append(&mut mp_cands);
            }
            if !mp_pts.is_empty() && mpsep.as_ref().is_some_and(|f| !f.contains('p')) {
                let sum = mpsep.as_ref().is_some_and(|f| f.contains('u'));
                let cnorm = p.cost.iter().map(|c| c * c).sum::<f64>().sqrt();
                let target = if mp_quota { &mut mp_extra_pool } else { &mut cands };
                for c in target.iter_mut().filter(|c| !c.forced) {
                    let norm = c.coefs.iter().map(|&(_, a)| a * a).sum::<f64>().sqrt().max(1e-300);
                    let ev = |pt: &[f64]| (c.coefs.iter().map(|&(j, a)| a * pt[j]).sum::<f64>() - c.rhs) / norm;
                    let e0 = ev(&x);
                    let (mut emax, mut esum) = (e0, e0.max(0.0));
                    for pt in &mp_pts {
                        let e = ev(pt);
                        emax = emax.max(e);
                        esum += e.max(0.0);
                    }
                    c.efficacy = if sum { esum } else { emax };
                    if e0 > 1e-6 && !mp_quota {
                        let sc = cut_score(c, p, cnorm);
                        mp_anchor = Some(mp_anchor.map_or(sc, |m: f64| m.max(sc)));
                    }
                }
                target.retain(|c| c.forced || c.efficacy > 1e-6);
            }
            let ncands = cands.len() + face_cands.len();
            if ncands == 0 {
                if self.params.verbose && job.is_some() {
                    eprintln!("MIP: cut round {round}: background cuts: no candidates; stop");
                }
                break;
            }
            let dbg_sep = env_str!("ENOMOTO_MIP_DEBUG_SEP").is_some() && !self.params.submip;
            let t_sel0 = std::time::Instant::now();
            if !self.params.submip {
                self.age_pool();
            }
            // 候補はすべてカットプールに入れる (選択で落ちたものも、ノードで違反すれば使う)
            if !self.params.submip && env_str!("ENOMOTO_MIP_POOL_SELECTED_ONLY").is_none() {
                for c in cands.iter().chain(face_cands.iter()) {
                    self.add_to_pool(&c.coefs, c.rhs);
                }
            }
            let pool_secs = t_sel0.elapsed().as_secs_f64();
            if prof {
                sepprof::add(sepprof::T_POOL, pool_secs);
                sepprof::PROF.with(|pp| {
                    let mut pp = pp.borrow_mut();
                    for c in cands.iter().chain(face_cands.iter()) {
                        pp.cands[(c.src >= sepprof::SRC_FACE) as usize][(c.src % sepprof::SRC_FACE) as usize] += 1;
                    }
                });
            }
            let cand_nnz: usize = cands.iter().map(|c| c.coefs.len()).sum();
            // 診断 (`ENOMOTO_MIP_DEBUG_DUMP_CANDS=ディレクトリ`): 根のこのラウンドの候補を `<ディレクトリ>/cands_r<ラウンド>.txt` に
            // 1 行 1 本 (`分離器の番号 効き目 右辺 列:係数 ...`、`sum a x <= 右辺`) で書く。選んだものは `chosen_r<ラウンド>.txt`
            let dump_cands = env_str!("ENOMOTO_MIP_DEBUG_DUMP_CANDS").filter(|_| !self.params.submip && job.is_none());
            if let Some(dir) = &dump_cands {
                let txt: String = cands.iter().chain(face_cands.iter()).map(|c| format!("{} {:.6e} {:.17e} {}\n", c.src, c.efficacy, c.rhs, c.coefs.iter().map(|&(j, a)| format!("{j}:{a:.17e}")).collect::<Vec<_>>().join(" "))).collect();
                let _ = std::fs::write(format!("{dir}/cands_r{round}.txt"), txt);
            }
            // 効き目の大きい順に、平行なものを除いて選ぶ
            let max_cuts = if self.rch_part('N') { tunable!("ENOMOTO_T_MIP_ROOT_CUT_HIGHS_MAX_CUTS", if self.pool_part('N') { 1000usize } else { 500 }, usize) } else { tunable!("ENOMOTO_T_MIP_ROOT_MAX_CUTS", 50usize, usize) }.min(p.m.max(50));
            let max_cuts = if adapt_max_cuts > 0 { adapt_max_cuts } else { max_cuts };
            let max_cuts = max_cuts.min(lp_cap_cuts);
            let (forced, cands): (Vec<Candidate>, Vec<Candidate>) = cands.into_iter().partition(|c| c.forced);
            // `ENOMOTO_T_MIP_CUTSEL_AGG_QUOTA` = q (> 0、試作): 集約した行からのカット (tableau・経路集約) を、分離器ごとに
            // 質の上位 q 本まで、全体の質の閾値 (最良の `ENOMOTO_T_CUTSEL_MIN_REL` 倍) と関係なく先に採る ([`pick_agg_quota`])
            let agg_quota = if (self.params.submip && !self.cut_agg()) || phase2 {
                0
            } else {
                let q = tunable!("ENOMOTO_T_MIP_CUTSEL_AGG_QUOTA", 0usize, usize);
                if q == 0 && (self.cut_agg_has('q') || (self.params.submip && super::submip_highs('q'))) { 50 } else { q }
            };
            let (quota_cuts, cands) = if agg_quota > 0 { pick_agg_quota(cands, agg_quota, p) } else { (Vec::new(), cands) };
            let chosen = if phase2 {
                let maxpar = tunable!("ENOMOTO_T_MIP_CUT_PHASE2_MAXPAR", 0.3, f64);
                self.select_cuts_highs(cands, max_cuts, &x, maxpar)
            } else if (env_str!("ENOMOTO_MIP_CUTSEL_HIGHS").is_some() || self.pool_part('S')) && !self.cutsel_guard_fc() {
                let maxpar = tunable!("ENOMOTO_T_CUTSEL_HIGHS_MAXPAR", 0.1, f64);
                // `ENOMOTO_MIP_CUTSEL_HIGHS_GUARD` (試作、既定では使わない): HiGHS の点数での選択と既定の選択 (`select_cuts`) の両方を作り、
                // 和を採る。固定費用のネットワーク型 (`fc_relax`) では HiGHS の点数を使わず、緩めた既定の選択のまま (上の条件。
                // `ENOMOTO_MIP_ROOT_CUT_HIGHS` の 1 ラウンドの本数・時間の設定もそこでは使わない、[`Solver::root_cut_highs`])。
                // 根のカットのプールの選び直し (計測 14 の POOL) では、hypothyroid-k1 で HiGHS の点数が 1 ラウンド 5 本しか採らず
                // (既定は 50 本) 根の下界が -2889.7 -> -2902.2 に下がって再スタートが起き、mc11 では固定費用型の緩めた選択を上書きして
                // 背景のカットの後の下界が 11668 -> 11500 に下がっていた (analysis/highstree_phys_20261024.md §8)
                if env_str!("ENOMOTO_MIP_CUTSEL_HIGHS_GUARD").is_some_and(|v| v != "fc") {
                    // HiGHS の点数での選択に、既定の選択の上位 `ENOMOTO_T_MIP_CUTSEL_HIGHS_GUARD_K` (20) 本を足す (同じ係数のカットは 1 本。
                    // 値が `fc` なら足さず、固定費用型で外すだけ)。hypothyroid-k1 の根の 0 ラウンド目では既定の選択が密なカット 4 本 (+9.5)、
                    // HiGHS の点数が 2 非零のカット 6 本 (+0.1) を採る。既定の選択を全部 (最大 1000 本) 足すと physiciansched6-2 で 1 ラウンド
                    // 300〜460 本になり LP が時間の上限に当たる
                    let mut hy = select_cuts(cands.clone(), tunable!("ENOMOTO_T_MIP_CUTSEL_HIGHS_GUARD_K", 20usize, usize).min(max_cuts), p);
                    let hs = self.select_cuts_highs(cands, max_cuts, &x, maxpar);
                    for c in hs {
                        if !hy.iter().any(|d| d.coefs == c.coefs && d.rhs == c.rhs) {
                            hy.push(c);
                        }
                    }
                    hy
                } else {
                    self.select_cuts_highs(cands, max_cuts, &x, maxpar)
                }
            } else if let Some(fl) = super::cutsel_ens::flags(self.params.submip).filter(|_| !fc_relax) {
                // 試作 `ENOMOTO_MIP_CUTSEL_ENS` (既定では使わない): SCIP の cutsel_ensemble の点数・間引き・止め方 ([`super::cutsel_ens`])
                select_cuts_ens(cands, &forced, max_cuts, p, fl)
            } else if fc_relax && fc_w < 1.0 {
                let (mr, mp) = super::cutmix::fc_sel_params(fc_w);
                select_cuts_hybrid_with(cands, max_cuts, p, mr, mp)
            } else if fc_relax {
                select_cuts_hybrid_with(cands, max_cuts, p, tunable!("ENOMOTO_T_CUTSEL_FC_MIN_REL", 0.0, f64), tunable!("ENOMOTO_T_CUTSEL_FC_MAXPAR", 0.9, f64))
            } else if env_str!("ENOMOTO_MIP_ROOT_CUT_HIGHS").is_some() && self.root_cut_highs() && job.is_none() {
                // `ENOMOTO_MIP_ROOT_CUT_HIGHS`: 平行度の上限を `ENOMOTO_T_MIP_ROOT_CUT_HIGHS_MAXPAR` (0.9) に緩める。physiciansched6-2 の根の
                // 1 ラウンド目の候補 (4090 本) を同じ LP に足すと、既定の選択 (150 本・0.3) では 48583 -> 48588、500 本・0.9 なら 48774
                // (候補全部 48790)。最適面が退化していて、短いカットを少しずつ足しても LP が別の頂点に移るだけ
                select_cuts_hybrid_with(cands, max_cuts, p, tunable!("ENOMOTO_T_CUTSEL_MIN_REL", 0.5, f64), tunable!("ENOMOTO_T_MIP_ROOT_CUT_HIGHS_MAXPAR", 0.9, f64))
            } else if mp_anchor.is_some() && env_str!("ENOMOTO_MIP_CUTSEL_OLD").is_none() {
                select_cuts_hybrid_anchor(cands, max_cuts, p, tunable!("ENOMOTO_T_CUTSEL_MIN_REL", 0.5, f64), tunable!("ENOMOTO_T_CUTSEL_MAXPAR", 0.3, f64), mp_anchor)
            } else if face_anchor >= 1 && env_str!("ENOMOTO_MIP_CUTSEL_OLD").is_none() {
                select_cuts_hybrid_anchor(cands, max_cuts, p, tunable!("ENOMOTO_T_CUTSEL_MIN_REL", 0.5, f64), tunable!("ENOMOTO_T_CUTSEL_MAXPAR", 0.3, f64), anchor)
            } else {
                select_cuts(cands, max_cuts, p)
            };
            let chosen = if face_cands.is_empty() {
                chosen
            } else {
                // 面の頂点のカットは別枠で選び、今の頂点で選んだカットとほぼ平行なものは除く
                let mut chosen = chosen;
                let extra = select_cuts(std::mem::take(&mut face_cands), face_quota, p);
                for c in extra {
                    if !chosen.iter().any(|d| cut_parallelism(&c.coefs, &d.coefs) > 0.99) {
                        chosen.push(c);
                    }
                }
                chosen
            };
            let mut chosen = chosen;
            // 試作 `ENOMOTO_MIP_ROOTCAP` の `d` ([`super::solver::rootcap`]): 根の LP が重い (根の LP の秒 (前処理の LP の最初の LP を含む) が
            // `ENOMOTO_T_MIP_ROOTCAP_DENSE_LP` (2 秒) 以上) 外側の根のカットのループ (裏のカットの写しも) では、1 ラウンドに足すカットの非零の和を
            // max(`ENOMOTO_T_MIP_ROOTCAP_DENSE_FRAC` (0.05) x 元の行列の非零, 1000) までにする (選んだ順に、最初の 1 本は必ず残す)。
            // neos-873061 はカットの行の平均非零が 2700〜4400 (元の行は 3.8) で、1 ラウンドの LP が 17〜37 秒かかっていた
            if super::solver::rootcap('d') && !self.params.submip && chosen.len() > 1 {
                let t_lp = self.root_lp_secs.max(f64::from_bits(super::solver::ROOTCAP_PROBE_SECS.load(std::sync::atomic::Ordering::Relaxed)));
                if t_lp >= tunable!("ENOMOTO_T_MIP_ROOTCAP_DENSE_LP", 2.0, f64) {
                    let cap = (tunable!("ENOMOTO_T_MIP_ROOTCAP_DENSE_FRAC", 0.05, f64) * p.nnz as f64).max(1000.0);
                    let n0 = chosen.len();
                    let nnz0: usize = chosen.iter().map(|c| c.coefs.len()).sum();
                    let mut acc = 0usize;
                    let mut k = 0usize;
                    chosen.retain(|c| {
                        let keep = k == 0 || (acc + c.coefs.len()) as f64 <= cap;
                        k += 1;
                        if keep {
                            acc += c.coefs.len();
                        }
                        keep
                    });
                    if self.params.verbose && chosen.len() < n0 {
                        eprintln!("MIP: cut round {round}: ROOTCAP d: kept {} of {n0} cuts (nonzeros {acc} of {nnz0}, cap {cap:.0}){}", chosen.len(), if job.is_some() { " (background)" } else { "" });
                    }
                }
            }
            if !quota_cuts.is_empty() {
                if self.params.verbose && env_str!("ENOMOTO_MIP_DEBUG_AGG_QUOTA").is_some() {
                    eprintln!("MIP: cut round {round}: {} cuts by the aggregation quota, {} by the selection", quota_cuts.len(), chosen.len());
                }
                chosen.extend(quota_cuts);
            }
            // 診断 (`ENOMOTO_MIP_DEBUG_TOPCUTS` = k): 選んだカットの最初の k 本を、列の LP 値・境界・整数かと並べて表示する
            let topk = tunable!("ENOMOTO_MIP_DEBUG_TOPCUTS", 0usize, usize);
            if topk > 0 && !self.params.submip {
                for c in chosen.iter().take(topk) {
                    let terms: Vec<String> = c.coefs.iter().take(8).map(|&(j, a)| format!("{a:+.3}*x{j}[{:.3} {}..{} {}]", x[j], self.dom.global_lo[j], self.dom.global_up[j], if p.is_int[j] { "I" } else { "C" })).collect();
                    eprintln!("TOPCUT round {round} eff {:.4} nnz {} <= {:.3}: {}", c.efficacy, c.coefs.len(), c.rhs, terms.join(" "));
                }
            }
            if mp_quota && !mp_extra_pool.is_empty() {
                // 別枠: 選んだカットと同じもの・平行度が `ENOMOTO_T_MIP_MPSEP_MAXPAR` (既定 0.3) を超えるものを除き、全部の点での効き目の順に
                let q = tunable!("ENOMOTO_T_MIP_MPSEP_EXTRA", 5usize, usize);
                let maxpar = tunable!("ENOMOTO_T_MIP_MPSEP_MAXPAR", 0.3, f64);
                let cnorm = p.cost.iter().map(|c| c * c).sum::<f64>().sqrt();
                let mut ex: Vec<(f64, Candidate)> = mp_extra_pool.drain(..).map(|c| (cut_score(&c, p, cnorm), c)).collect();
                ex.sort_by(|a, b| b.0.total_cmp(&a.0));
                let mut added = 0usize;
                for (_, c) in ex {
                    if added >= q {
                        break;
                    }
                    if chosen.iter().any(|d| d.coefs == c.coefs && d.rhs == c.rhs || cut_parallelism(&c.coefs, &d.coefs) > maxpar) {
                        continue;
                    }
                    chosen.push(c);
                    added += 1;
                }
            }
            if !mp_pts.is_empty() {
                let off = chosen.iter().filter(|c| c.coefs.iter().map(|&(j, a)| a * x[j]).sum::<f64>() - c.rhs <= 1e-6 * (1.0 + c.rhs.abs())).count();
                mp_stats.3 += off;
                mp_stats.4 += chosen.len();
                if self.params.verbose && mpsep.as_ref().is_some_and(|f| f.contains('v')) {
                    eprintln!("MIP: cut round {round}: mpsep: chosen {} cuts, {off} of them do not cut x*", chosen.len());
                }
            }
            // 選択の上限まで採ったか (選択を通さないカットは数えない)
            let quota_full = chosen.len() >= max_cuts && ncands >= 2 * max_cuts;
            chosen.extend(forced);
            // 診断 (`ENOMOTO_MIP_REDSPLIT_LOG`): 試作 `ENOMOTO_MIP_REDSPLIT` の縮約の行から作った候補のうち採られた数
            if super::redsplit::redsplit_mode().1 && env_str!("ENOMOTO_MIP_REDSPLIT_LOG").is_some() && !self.params.submip {
                let (k, total) = super::redsplit::count_chosen(chosen.iter().map(|c| super::redsplit::fingerprint(&c.coefs, c.rhs)));
                eprintln!("REDSPLIT chosen round {round}: {k} of {} chosen (candidates {total})", chosen.len());
            }
            // 試作 `ENOMOTO_MIP_NODECNT` の `r`: 根で採ったカットの指紋と出どころ (経路集約か) を覚える ([`Self::nc_root_decide`])
            if self.nc_path_forced && super::solver::nodecnt('r') && !self.params.submip {
                for c in &chosen {
                    let is_path = matches!(c.src % sepprof::SRC_FACE, sepprof::SRC_PATH | sepprof::SRC_PATHH);
                    self.nc_root_cnt[0] += 1;
                    self.nc_root_cnt[1] += is_path as u64;
                    self.nc_root_fp.insert(cut_fingerprint(&c.coefs, c.rhs), is_path);
                }
            }
            if self.sep_adapt_on() {
                let mut acc = [0usize; sepprof::NSRC];
                for c in &chosen {
                    acc[(c.src % sepprof::SRC_FACE) as usize] += 1;
                }
                self.sep_adapt_update(&acc, round);
            }
            if let Some(dir) = &dump_cands {
                let txt: String = chosen.iter().map(|c| format!("{} {:.6e} {:.17e} {}\n", c.src, c.efficacy, c.rhs, c.coefs.iter().map(|&(j, a)| format!("{j}:{a:.17e}")).collect::<Vec<_>>().join(" "))).collect();
                let _ = std::fs::write(format!("{dir}/chosen_r{round}.txt"), txt);
            }
            let sep_secs = t_sep.elapsed().as_secs_f64();
            sep_tot += sep_secs;
            nrounds += 1;
            let sel_secs = t_sel0.elapsed().as_secs_f64() - pool_secs;
            if prof {
                sepprof::add(sepprof::T_SELECT, sel_secs);
                sepprof::PROF.with(|pp| {
                    let mut pp = pp.borrow_mut();
                    pp.rounds += 1;
                    for c in &chosen {
                        pp.chosen[(c.src >= sepprof::SRC_FACE) as usize][(c.src % sepprof::SRC_FACE) as usize] += 1;
                    }
                });
            }
            if super::cmirdelta::on() && !self.params.submip {
                for c in &chosen {
                    super::cmirdelta::note_chosen(&c.coefs, c.rhs, c.efficacy);
                }
            }
            if sl_on && chosen.is_empty() {
                self.sl.end_call_without_add();
            }
            if chosen.is_empty() {
                if self.params.verbose && job.is_some() {
                    eprintln!("MIP: cut round {round}: background cuts: no cuts chosen; stop");
                }
                break;
            }
            // 上限まで採ったラウンドは停滞として数えない (候補が上限を超えて残っている間は LP を締め続ける)
            let saturated = adapt_max && chosen.len() >= max_cuts;
            if saturated {
                adapt_max_cuts = (2 * max_cuts).min(tunable!("ENOMOTO_T_MIP_CUT_ADAPT_MAX_CAP", 500usize, usize)).max(max_cuts);
            }
            // 試作 `ENOMOTO_MIP_CUTLPCAP` の `n`: 1 ラウンドの本数を前のラウンドに合わせて絞る
            if cl_n {
                let n0 = tunable!("ENOMOTO_T_MIP_CUTLPCAP_N0", 100usize, usize).max(1);
                let lim_n = if cl_last_ok_cuts == 0 { n0 } else { n0.max((tunable!("ENOMOTO_T_MIP_CUTLPCAP_NMULT", 2.0, f64) * cl_last_ok_cuts as f64) as usize) };
                if chosen.len() > lim_n {
                    let before = chosen.len();
                    chosen.sort_by(|a, b| b.efficacy.partial_cmp(&a.efficacy).unwrap_or(std::cmp::Ordering::Equal));
                    chosen.truncate(lim_n);
                    if self.params.verbose {
                        eprintln!("MIP: cut round {round}: CUTLPCAP n: {before} cuts -> {lim_n} (last solved round {cl_last_ok_cuts} cuts)");
                    }
                }
            }
            if !self.params.submip && env_def!("ENOMOTO_MIP_CUT_CLIQUE_EXTEND", "1").is_some() {
                self.extend_clique_cuts(&mut chosen, &x, round);
            }
            if sl_on {
                self.sl_prepare(&chosen, &x);
            }
            let row_effs: Vec<f64> = if cl_a { chosen.iter().map(|c| c.efficacy).collect() } else { Vec::new() };
            let mut rows: Vec<(Vec<(usize, f64)>, f64, f64)> = chosen.into_iter().map(|c| (c.coefs, f64::NEG_INFINITY, c.rhs)).collect();
            total_added += rows.len();
            // `x` (共有の置き場がないとき): 負けた写しから足したカットのうち選ばれた数を数える
            if race_x && !share_keys.is_empty() && job.as_ref().is_some_and(|j| j.share.lock().unwrap().is_none()) {
                let used = rows.iter().filter(|(c, _, _)| share_keys.contains(&c.iter().map(|&(j, a)| (j, a.to_bits())).collect::<Vec<_>>())).count();
                if let Some(j) = &job {
                    j.share_used.fetch_add(used, std::sync::atomic::Ordering::Relaxed);
                }
            }
            // 試作 `ENOMOTO_MIP_OPTFACE_PARCUTS` の `s`: 選んだカットを他の写しに渡す (受け取ったカットのうち選ばれた数も数える)
            if let Some((sh, id)) = job.as_ref().and_then(|j| j.share.lock().unwrap().clone()) {
                let mut used = 0usize;
                let publish = !share_gated;
                let mut v = sh.cuts.lock().unwrap();
                for (c, _, r) in &rows {
                    let k: Vec<(usize, u64)> = c.iter().map(|&(j, a)| (j, a.to_bits())).collect();
                    if share_keys.contains(&k) {
                        used += 1;
                    } else if publish {
                        v.push((id, c.clone(), *r));
                    }
                }
                if let Some(j) = &job {
                    j.share_used.fetch_add(used, std::sync::atomic::Ordering::Relaxed);
                }
            }
            // カットプールに残す (ノードで違反していれば LP に戻す。候補として入れ済みなら重複は除かれる)
            if !self.params.submip {
                for (c, _, r) in &rows {
                    self.add_to_pool(c, *r);
                }
                // `ENOMOTO_MIP_SYM_CUTS`: 選んだカットを、その列を動かす生成元で写した像もプールに入れる (上限 `ENOMOTO_T_MIP_SYM_CUTS_MAX` 本)
                if let Some(gens) = &sym_gens {
                    let before = sym_images;
                    let mut ks: Vec<u32> = Vec::new();
                    'rows: for (c, _, r) in &rows {
                        ks.clear();
                        for &(j, _) in c {
                            ks.extend_from_slice(&sym_moving[j]);
                        }
                        ks.sort_unstable();
                        ks.dedup();
                        for &k in &ks {
                            if sym_images >= sym_cap {
                                break 'rows;
                            }
                            let g = &gens[k as usize];
                            let mut img: Vec<(usize, f64)> = c.iter().map(|&(j, a)| (g[j], a)).collect();
                            img.sort_unstable_by_key(|e| e.0);
                            let len0 = self.cut_pool.len();
                            self.add_to_pool(&img, *r);
                            if self.cut_pool.len() > len0 {
                                sym_pool.insert(len0);
                            }
                            sym_images += 1;
                        }
                    }
                    if self.params.verbose && sym_images > before {
                        eprintln!("MIP: cut round {round}: {} symmetric images of the chosen cuts put in the pool", sym_images - before);
                    }
                }
            }
            // `ENOMOTO_MIP_CUT_KEEP_LU`: 前のラウンドで外さなかった効いていないカットを、ここで新しいカットと一緒に外す
            // (`ENOMOTO_MIP_CUT_FAIL_RESTORE` も使うときは、加える前の状態を取っておくために先に外す)
            let keep_lu_remove = if std::mem::take(&mut keep_lu_pending) { self.inactive_cut_mask() } else { None };
            let keep_lu_remove = match keep_lu_remove {
                Some(rm) if fail_restore => {
                    self.delete_lp_rows(&rm);
                    None
                }
                r => r,
            };
            let saved_rows = self.lp.num_rows() - keep_lu_remove.as_ref().map_or(0, |rm| rm.iter().filter(|&&r| r).count());
            let saved_state = fail_restore.then(|| self.lp.save_state());
            // 試作 `ENOMOTO_MIP_CUT_LP_CAP_ROBUST` の `d`: 外すときに戻す加える前の状態 (止まった基底を残させるので)
            // 試作 `ENOMOTO_MIP_HO4_GAIN_RESTORE` (analysis/highs_only_20261105/G4.md 10 節、既定では使わない): `ENOMOTO_MIP_HO5_CUTLP_GAIN` は
            // 続けないときも止まった LP の状態を残させる (`FORCE_RESUME_ONCE`) ので、カットを外した後の解き直しが加える前の最適基底ではなく
            // 止まった基底から始まり、根の頂点が変わる (neos-960392 で解き直し 1 -> 7208 反復、根の RENS が実行不能になり 60 秒で解けない)。
            // `d` と同じく加える前の状態を取っておき、外すときに戻す
            let ho4_gr = ho5_gain > 0.0 && env_str!("ENOMOTO_MIP_HO4_GAIN_RESTORE").is_some();
            let rob_saved = ((rob_d || ho4_gr) && !fail_restore && keep_lu_remove.is_none()).then(|| self.lp.save_state());
            // 試作 `ENOMOTO_MIP_CUTLPCAP` の `a`: 解き直しのために加える前の状態を取っておく
            let cl_saved = (cl_a && !fail_restore && keep_lu_remove.is_none() && rows.len() >= 2 * tunable!("ENOMOTO_T_MIP_CUTLPCAP_AMIN", 5usize, usize)).then(|| self.lp.save_state());
            // 試作 `ENOMOTO_MIP_CUT0_RETRY` (analysis/cutloop_stop_20261106/A.md、既定では使わない): `HO4_CUT0=moved` の改良。
            // 1 回目は既定とまったく同じに解き、進みの判定で止まったとき、次の 3 つを満たすときだけ同じカットの LP を解き直す。
            // (1) 下界がまだ根の LP の値から動いていない (`!moved`) 序盤のラウンド (`ENOMOTO_T_MIP_CUT0_RETRY_ROUNDS` 未満、既定 2)。
            // (2) 止まった LP の目的値が出発から相対 `ENOMOTO_T_MIP_CUT0_RETRY_TOL` (既定 1e-6) を超えて伸びていた (平らな窓は双対退化の
            //     途中であって「伸びない」のではない。neos-960392 は根の LP の値が最適値で伸びが -1e-10 なので外れ、既定と同じ道)。
            // (3) 解き直しの反復の上限は MULT (既定 2) x max(根の LP の反復, 前処理の LP の最初の LP の反復, 行の数) (neos-873061 は根の LP が
            //     前処理の LP の基底から 1 反復で終わり、根の LP の反復だけを基準にすると上限が最低値 1000 に張り付く)。
            // 値に `c` を含むと、解き直しが根の LP の時間を超えても終わらないとき、残りを全スラック基底から解く (neos-3216931-puriri は
            // 根の最適基底から出発すると照合の不一致による再分解が 9 反復に 1 回続き 1 反復 2〜5 ms で 2 万反復を超えても終わらず、
            // 全スラック基底からなら 0.33 ms/反復・22850 反復・7.4 秒で最適)。値に `w` を含むと、解き直しにも長い窓の進みの判定
            // (上限の半分を過ぎてから、窓は上限の 1/4、伸びが出発からの伸びの `ENOMOTO_T_MIP_CUT0_RETRY_FRAC` (既定 1e-3) 倍未満なら止める) を使う。
            // 解き直しが最適にならなければ、加える前の状態に戻してから既定どおりカットを外す
            // (`c0r_saved` は解き直しが最適にならなかったときに戻す、加える前の状態)
            let c0r = if cs_outer && !moved { env_str!("ENOMOTO_MIP_CUT0_RETRY").filter(|v| *v != "0") } else { None };
            let c0r = c0r.filter(|_| (round as usize) < tunable!("ENOMOTO_T_MIP_CUT0_RETRY_ROUNDS", 2usize, usize));
            let c0r_saved = (c0r.is_some() && !fail_restore && keep_lu_remove.is_none()).then(|| self.lp.save_state());
            let t_add0 = std::time::Instant::now();
            let dse_check = env_str!("ENOMOTO_MIP_DEBUG_DSE_CHECK").is_some() && !self.params.submip;
            if dse_check {
                self.lp.debug_dse_check(&format!("round {round} before add_rows"));
            }
            if let Some(rm) = &keep_lu_remove {
                if dbg_sep {
                    eprintln!("SEP round {round}: removed {} inactive cuts together with the new cuts", rm.iter().filter(|&&r| r).count());
                }
                self.replace_cut_rows(rm, &rows);
            } else {
                self.add_cut_rows(&rows);
            }
            if sl_on {
                self.sl.bind(self.next_cut_id);
            }
            if dse_check {
                self.lp.debug_dse_check(&format!("round {round} after add_rows ({} rows)", rows.len()));
            }
            let add_secs = t_add0.elapsed().as_secs_f64();
            if prof {
                sepprof::add(sepprof::T_ADDROWS, add_secs);
            }
            let t_lp0 = std::time::Instant::now();
            let it0 = self.lp.total_iterations();
            // `ENOMOTO_T_MIP_CUT_LP_WORK`: 1 回の LP の上限を反復で決める (時間の上限は安全網)
            let lim = if bg_work.is_some() {
                bg_work_lim0.max((lp_work_prev * ok_lp_iters as f64) as u64)
            } else if lp_work > 0.0 { ((lp_work * root_iters.max(100) as f64) as u64).max(1000).max((lp_work_prev * ok_lp_iters as f64) as u64) } else { 10 * root_iters.max(100) + 10_000 };
            // 試作 `ENOMOTO_MIP_CUT_LP_CAP_ROBUST` の `s`: 問題の大きさの下限
            let lim = if rob_s {
                let a = tunable!("ENOMOTO_T_MIP_CUT_LP_CAP_ROBUST_ROWS", 2.0, f64) * self.lp.num_rows() as f64;
                let b = tunable!("ENOMOTO_T_MIP_CUT_LP_CAP_ROBUST_ROOT", 2.0, f64) * rob_root as f64;
                lim.max(a.max(b) as u64)
            } else {
                lim
            };
            // 試作 `ENOMOTO_MIP_HO4_CUT0` (analysis/highs_only_20261105/G4.md、既定では使わない): 主の根のカットのループで、下界がまだ
            // 根の LP の値から動いていない間 (多くは 0 ラウンド目) のカットの LP は、反復の上限を `ENOMOTO_T_MIP_HO4_CUT0_MULT` (既定 2)
            // x 根の LP の反復より小さくせず、進みの判定 (`p`) で打ち切らない (時間の上限は安全網として残す)。
            // neos-3216931-puriri・satellites2-60-fs・neos-873061 は 0 ラウンド目のカットの LP が進みの判定 (目的値が平らな窓) で
            // 外され、`ENOMOTO_MIP_CUT_LP_CAP_STOP` でループが止まって根の下界が根の LP のまま。HiGHS は最初の分離のラウンドに
            // 根の LP の 0.7〜1.6 倍の反復を使って下界を上げる
            let ho4_cut0 = cs_outer && !moved && env_str!("ENOMOTO_MIP_HO4_CUT0").is_some_and(|v| v != "moved");
            // `ENOMOTO_MIP_HO4_CUT0=moved` (G4.md 10 節): 1 回目は既定と同じ (上限・進みの判定・止まった状態を残さない) に解き、進みの判定で
            // 止まったときに、その LP の目的値が出発から相対 `ENOMOTO_T_MIP_HO4_CUT0_MOVED_TOL` (既定 1e-6) を超えて伸びていたときだけ、同じ
            // カットの LP を上限 `ENOMOTO_T_MIP_HO4_CUT0_MULT` x 根の LP の反復 (前処理の LP の最初の LP と大きい方) で進みの判定なしに解き直す
            // (止まった所からは続けない: 止まった状態を残すと、続けないときにも外した後の解き直しの頂点が変わるため。HO5_CUTLP_GAIN を見よ)
            let ho4m = cs_outer && !moved && env_str!("ENOMOTO_MIP_HO4_CUT0") == Some("moved");
            // 根の LP が前処理の LP の基底から数反復で終わった問題 (neos-873061 は 1 反復) では、前処理の LP の反復を基準にする
            let ho4_base = root_iters.max(super::HO4_PROBE_ITERS.with(|c| c.get())).max(100);
            let lim = if ho4_cut0 { lim.max((tunable!("ENOMOTO_T_MIP_HO4_CUT0_MULT", 2.0, f64) * ho4_base as f64) as u64) } else { lim };
            let lim = if cl_c {
                let b = (self.lp.num_rows() as u64).max(ho4_base);
                lim.max((tunable!("ENOMOTO_T_MIP_CUTLPCAP_CMULT", 1.0, f64) * b as f64) as u64)
            } else {
                lim
            };
            // 試作 `ENOMOTO_MIP_CUTLPCAP` の検査用 (`ENOMOTO_T_MIP_CUTLPCAP_TESTCAP` = k > 0): 反復の上限を k にして `a` の解き直しを必ず通す (誤答の検査だけに使う)
            let lim = match tunable!("ENOMOTO_T_MIP_CUTLPCAP_TESTCAP", 0u64, u64) {
                k if k > 0 && cl.is_some() => k,
                _ => lim,
            };
            if cl_log {
                eprintln!("MIP: cut round {round}: CUTLPCAP cap {lim} cuts {} rows {} root_iters {root_iters} ok_iters {ok_lp_iters}", rows.len(), self.lp.num_rows());
            }
            if ho4_cut0 && self.params.verbose {
                eprintln!("MIP: cut round {round}: ENOMOTO_MIP_HO4_CUT0: iteration cap {lim}, no progress check");
            }
            // 1 回の LP の時間の上限 (全体の 2%、最低 1 秒): 数値的に悪条件の LP が残り時間を使い切るのを防ぐ
            let cap = if self.params.time_limit.is_finite() { (self.cut_lp_time_frac() * self.params.time_limit).max(1.0) } else { f64::INFINITY };
            let cap = cap.max(lp_cap_mult * ok_lp_secs);
            let cap = if lp_work > 0.0 { cap * tunable!("ENOMOTO_T_MIP_CUT_LP_WORK_TIME_MULT", 5.0, f64) } else { cap };
            // 裏のカットの写しの 1 回の LP の上限 (analysis/bgx_check1_20261102.md)。写しの `time_limit` は主の残り時間なので、
            // 上限は主 (60 秒の 2% = 1.2 秒) より短い 1.0 秒 (最低値) になる。physiciansched6-2 の遅い機械では、写しのカットの LP
            // (主と同じ頂点から 1200〜1700 反復) がこれに当たって 2 ラウンド続けてカットを全部外し、失敗 2 回でループを抜けていた
            // (ラウンドの記録 0、下界は上がらない)。
            // `ENOMOTO_MIP_BG_CUTS_LP_CAP` = m (試作、既定では使わない): 写しでは上限を主の根の LP の時間の m 倍より短くしない。
            // `ENOMOTO_T_MIP_BG_CUTS_LP_CAP_SECS` = s (診断、既定 0 で使わない): 写しの上限を s 秒にする (遅い機械の真似)
            let cap = if job.is_some() {
                let s = tunable!("ENOMOTO_T_MIP_BG_CUTS_LP_CAP_SECS", 0.0, f64);
                let m = env_str!("ENOMOTO_MIP_BG_CUTS_LP_CAP").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                let c = if s > 0.0 { s } else { cap };
                let c = c.max(m * super::solver::BG_ROOT_LP_SECS.with(|c| c.get()));
                // `ENOMOTO_MIP_BG_CUTS_LP_WORK`: 時間の上限は安全網 (上の反復の上限が本来の上限)
                if bg_work.is_some() { c * tunable!("ENOMOTO_T_MIP_BG_CUTS_LP_WORK_TIME_MULT", 5.0, f64) } else { c }
            } else {
                cap
            };
            // 試作 `ENOMOTO_MIP_S6_CUTLP_TCAP` = k (analysis/highs_slow_20261106/S6.md、既定では使わない): 主の根のカットのループ (外側の求解、
            // 裏のカットの写しでない) で、1 回のカットの LP の時間の上限を、このループで最適まで解けたカットの LP の最長の秒の k 倍
            // (最低 `ENOMOTO_T_MIP_S6_CUTLP_TCAP_MIN` 秒、既定 2) に押さえる (最適まで解けた LP がまだないうちは押さえない)。上限に当たった LP は
            // 既定と同じく外す。nexp-150-20-8-5 (600 秒) では普段のカットの LP は 1 秒未満なのに、ときどき 1 回が 4〜5 万反復・60 秒
            // (全体の 2% x 5 の安全網) に当たって外され、その 60 秒でループが根のカットの時間の上限に届いて止まっていた
            let s6_tcap = if job.is_none() && !self.params.submip && self.snap.is_none() && ok_lp_secs > 0.0 {
                env_str!("ENOMOTO_MIP_S6_CUTLP_TCAP").and_then(|v| v.parse::<f64>().ok()).filter(|k| *k > 0.0)
            } else {
                None
            };
            let cap = match s6_tcap {
                Some(k) => cap.min((k * ok_lp_secs).max(tunable!("ENOMOTO_T_MIP_S6_CUTLP_TCAP_MIN", 2.0, f64))),
                None => cap,
            };
            if job.is_some() && self.params.verbose && round <= 1 {
                eprintln!("MIP: cut round {round}: background cut LP time cap {cap:.2}s{}", if bg_work.is_some() { format!(", iteration cap {lim}") } else { String::new() });
            }
            work_rounds += 1;
            let lp_deadline = if cap.is_finite() {
                let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(cap);
                Some(self.deadline.map_or(d, |g| g.min(d)))
            } else {
                self.deadline
            };
            // 診断 (`ENOMOTO_MIP_DEBUG_DUMP_CUT_LP=ディレクトリ`): カットを足した LP と出発の基底を、求解の前の状態で
            // `<ディレクトリ>/cutlp_r<ラウンド>_<結果>.mps/.bas` に書き出す (analysis/lp_hang_20261021.md)
            // 試作 `ENOMOTO_MIP_CUT_HARRIS2` = `ENOMOTO_SIMPLEX_HARRIS2` の値 (`1`・`t`・`r`。既定では使わない、analysis/puriri_cutlp_20261106.md):
            // 根のカットのループ (サブ MIP でないもの。裏のカットの写しも) のこのラウンドの LP だけ、双対単体法の比の判定に前向きの Harris を使う。
            // カットを足した直後の基底から既定の後ろ向きの窓 (比で 1e-7) で進むと、puriri では 100 反復で基底の条件数が 4.5e6 から 3e11 に
            // 悪くなり、照合の不一致と悪条件の再分解が 9 反復に 1 回起きて 1 反復 5 ms になる
            let _h2_scope = crate::simplex::slope_intercept_dual::Harris2Scope::new(
                env_str!("ENOMOTO_MIP_CUT_HARRIS2").filter(|v| *v != "0" && !self.params.submip),
            );
            let dump_dir = env_str!("ENOMOTO_MIP_DEBUG_DUMP_CUT_LP").filter(|_| !self.params.submip && job.is_none());
            let dump_basis = dump_dir.as_ref().map(|_| (self.lp.basis_cols(), self.lp.dse_weights()));
            // `ENOMOTO_MIP_CUT_DSE_EXACT` (試作、既定では使わない): カットを足した LP を、引き継いだ DSE 重みではなく出発の基底の
            // 厳密な重み (行ごとに BTRAN 1 回) から解く。引き継いだ重みは求解をまたいで更新を重ねるうちに厳密な値から大きく
            // 外れ (physiciansched6-2 では 1e8〜1e9 の重みが数百行)、カットを足した LP の反復が数倍〜十数倍に増えることがある
            // (analysis/lp_hang_20261021.md)
            // 試作 `ENOMOTO_MIP_BG_CUTS_DSE=x`: 裏のカットの写しの最初のカットの LP だけ同じく厳密な重みから (analysis/bgdse_20261102.md)
            let bg_dse_exact = job.is_some() && super::solver::BG_DSE_EXACT_FIRST.with(|c| c.replace(false));
            if bg_dse_exact && self.params.verbose {
                eprintln!("MIP: cut round {round}: background cut LP from the exact DSE weights");
            }
            let cut_dse_exact = (env_str!("ENOMOTO_MIP_CUT_DSE_EXACT").is_some() && !self.params.submip) || bg_dse_exact;
            // 試作 `ENOMOTO_MIP_BG_CUTS_DSE=q`・`=1q` (LP の始め)・`=qa`・`=1qa` (`ENOMOTO_T_MIP_BG_CUTS_DSE_AFTER` 反復目): 写しの最初のカットの LP の
            // 重みを `ENOMOTO_T_MIP_BG_CUTS_DSE_SAMPLE` (既定 32) 行の標本で厳密な値と比べ、判定値 (既定は 4 倍を超えてずれた行の割合) が
            // `ENOMOTO_T_MIP_BG_CUTS_DSE_DEV` (既定 0.1) を超えたときだけ全部の行を厳密に。既定の道筋には入れていない (analysis/bgdse_quality_20261102.md)
            let bg_dse_sample_mode = if job.is_some() && !cut_dse_exact { super::solver::BG_DSE_SAMPLE_FIRST.with(|c| c.replace(0)) } else { 0 };
            let bg_dse_sample = bg_dse_sample_mode > 0;
            if bg_dse_sample {
                let k = tunable!("ENOMOTO_T_MIP_BG_CUTS_DSE_SAMPLE", 32usize, usize);
                let dev = tunable!("ENOMOTO_T_MIP_BG_CUTS_DSE_DEV", 0.1, f64);
                // 判定に使う統計 (`ENOMOTO_T_MIP_BG_CUTS_DSE_STAT`): 0 = 全体の |ln| の中央値、1 = 全体のばらつき、2 = 主実行不能な行のばらつき、
                // 3 = 全体の 4 倍を超えてずれた行の割合 (既定)、4 = 主実行不能な行の同じ割合
                let stat = tunable!("ENOMOTO_T_MIP_BG_CUTS_DSE_STAT", 3u8, u8);
                crate::simplex::slope_intercept_dual::DSE_SAMPLE_RESULT.with(|c| c.set(None));
                let after = if bg_dse_sample_mode == 2 { tunable!("ENOMOTO_T_MIP_BG_CUTS_DSE_AFTER", 500usize, usize).max(1) } else { 0 };
                crate::simplex::slope_intercept_dual::DSE_SAMPLE_ONCE.with(|c| c.set(Some((k, dev, stat, after))));
            }
            if cut_dse_exact {
                crate::simplex::slope_intercept_dual::DSE_EXACT_ONCE.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            // `ENOMOTO_MIP_ROOT_CUT_CUTOFF`: 打ち切り値 (根を閉じられるときだけ)
            let cut_cutoff = if root_cutoff && self.root_gap_closed_at(f64::INFINITY) { self.prune_limit() - p.offset } else { f64::INFINITY };
            // 試作 `ENOMOTO_MIP_CUT_LP_CAP_ROBUST` の `d`・`p`: `d` は止まったところから続けるので、止まった基底を残させ、外すときのために
            // 加える前の状態を取っておく。`p` は上限の後を目的値の進みで打ち切る (反復の上限は安全網)
            let prev_lp_obj = self.lp.objective();
            let solve_lim = if rob_p && !ho4_cut0 {
                let w = ((tunable!("ENOMOTO_T_MIP_CUT_LP_CAP_ROBUST_WIN", 0.25, f64) * lim as f64) as usize).max(50);
                let frac = tunable!("ENOMOTO_T_MIP_CUT_LP_CAP_ROBUST_FRAC", 0.01, f64);
                crate::simplex::mip_lp::PROGRESS_STOP_ONCE.with(|c| c.set(Some((lim as usize, w, frac, prev_lp_obj))));
                ((tunable!("ENOMOTO_T_MIP_CUT_LP_CAP_ROBUST_HARD", 10.0, f64) * lim as f64) as u64).max(lim)
            } else {
                lim
            };
            if rob_d || ho5_gain > 0.0 {
                crate::simplex::mip_lp::FORCE_RESUME_ONCE.with(|c| c.set(true));
            }
            let mut st = self.lp.solve(&SolveLimits { iteration_limit: solve_lim, cutoff: cut_cutoff, deadline: lp_deadline });
            // 試作 `ENOMOTO_MIP_HO5_CUTLP_GAIN` ([`ho5_cutlp_gain`]): 上限 (進みの判定) で止まったカットの LP が、出発の目的値から
            // 大きく伸びていたら、外さずに止まったところから続けて解き切る
            if ho5_gain > 0.0 && rob_p && matches!(st, LpStatus::IterationLimit) && !self.time_up() && !stopped(&job) {
                if let Some((it, _gain, total)) = crate::simplex::slope_intercept_dual::ext_progress_stopped() {
                    if total >= ho5_gain * prev_lp_obj.abs().max(1.0) {
                        let it1 = self.lp.total_iterations();
                        let mult = tunable!("ENOMOTO_T_MIP_HO5_CUTLP_GAIN_TIME_MULT", 3.0, f64);
                        let d = if cap.is_finite() {
                            let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(mult * cap);
                            Some(self.deadline.map_or(d, |g| g.min(d)))
                        } else {
                            self.deadline
                        };
                        let lim2 = ((tunable!("ENOMOTO_T_MIP_HO5_CUTLP_GAIN_ITER_MULT", 50.0, f64) * lim as f64) as u64).max(2 * self.lp.num_rows() as u64);
                        let st0 = st;
                        st = self.lp.solve(&SolveLimits { iteration_limit: lim2, cutoff: cut_cutoff, deadline: d });
                        if self.params.verbose {
                            eprintln!(
                                "MIP: cut round {round}: HO5 cut LP gain {total:.3e} >= {ho5_gain} x |{prev_lp_obj:.6e}| at iter {it} ({st0:?}); continued (cap {lim2}): {st:?} after {} more iters, obj {:.10e}",
                                self.lp.total_iterations() - it1,
                                self.lp.objective()
                            );
                        }
                    }
                }
            }
            crate::simplex::mip_lp::FORCE_RESUME_ONCE.with(|c| c.set(false));
            crate::simplex::mip_lp::PROGRESS_STOP_ONCE.with(|c| c.set(None));
            if rob_p && !ho4_cut0 && self.params.verbose {
                if let Some((it, gain, total)) = crate::simplex::slope_intercept_dual::ext_progress_stopped() {
                    eprintln!("MIP: cut round {round}: cut LP stopped by the progress check at iter {it} (cap {lim}): window gain {gain:.3e}, gain from the start {total:.3e} ({} iters)", self.lp.total_iterations() - it0);
                } else if self.lp.total_iterations() - it0 > lim {
                    eprintln!("MIP: cut round {round}: cut LP went past the cap {lim} on progress: {st:?} after {} iters", self.lp.total_iterations() - it0);
                }
            }
            if ho4m && st == LpStatus::IterationLimit && !self.time_up() && !stopped(&job) {
                if let Some((it, _gain, total)) = crate::simplex::slope_intercept_dual::ext_progress_stopped() {
                    let thr = tunable!("ENOMOTO_T_MIP_HO4_CUT0_MOVED_TOL", 1e-6, f64) * prev_lp_obj.abs().max(1.0);
                    if total.is_finite() && total > thr {
                        let base = root_iters.max(super::HO4_PROBE_ITERS.with(|c| c.get())).max(100);
                        let lim2 = (tunable!("ENOMOTO_T_MIP_HO4_CUT0_MULT", 2.0, f64) * base as f64) as u64;
                        let it1 = self.lp.total_iterations();
                        st = self.lp.solve(&SolveLimits { iteration_limit: lim2, cutoff: cut_cutoff, deadline: lp_deadline });
                        if self.params.verbose {
                            eprintln!("MIP: cut round {round}: ENOMOTO_MIP_HO4_CUT0=moved: stopped at iter {it} after a gain of {total:.3e} from the start; re-solved with the cap {lim2}: {st:?} after {} iters", self.lp.total_iterations() - it1);
                        }
                    } else if self.params.verbose {
                        eprintln!("MIP: cut round {round}: ENOMOTO_MIP_HO4_CUT0=moved: stopped at iter {it}, gain from the start {total:.3e}; not moved, dropped as by default");
                    }
                }
            }
            let mut c0r_failed = false;
            if let (Some(mode), Some(ss)) = (c0r, c0r_saved.as_ref()) {
                if rob_p && st == LpStatus::IterationLimit && !self.time_up() && !stopped(&job) {
                    if let Some((it, _gain, total)) = crate::simplex::slope_intercept_dual::ext_progress_stopped() {
                        let thr = tunable!("ENOMOTO_T_MIP_CUT0_RETRY_TOL", 1e-6, f64) * prev_lp_obj.abs().max(1.0);
                        let it_first = self.lp.total_iterations() - it0;
                        let secs_first = t_lp0.elapsed().as_secs_f64();
                        if total.is_finite() && total > thr {
                            let base = root_iters.max(super::HO4_PROBE_ITERS.with(|c| c.get())).max(self.lp.num_rows() as u64).max(100);
                            let lim2 = (tunable!("ENOMOTO_T_MIP_CUT0_RETRY_MULT", 2.0, f64) * base as f64) as u64;
                            // `c`: 解き直しを `ENOMOTO_T_MIP_CUT0_RETRY_CHUNK` 反復 (既定は上限の 1/40、最低 500) ずつに区切り (止まった所から続ける)、
                            // 解き直しにかかった時間が根の LP の時間の `ENOMOTO_T_MIP_CUT0_RETRY_COLD` 倍 (既定 1) を超えても終わらなければ、残りを
                            // 全スラック基底から解く (全スラック基底から解く費用の目安が根の LP の時間なので、合わせて高々その 2 倍ほどで済む。
                            // 根の LP が 1000 反復未満なら見ない)
                            let root_rate = if root_iters >= 1000 && self.root_lp_secs > 0.0 { self.root_lp_secs / root_iters as f64 } else { 0.0 };
                            let rate = secs_first / it_first.max(1) as f64;
                            let chunked = mode.contains('c') && root_rate > 0.0;
                            let cold_secs = tunable!("ENOMOTO_T_MIP_CUT0_RETRY_COLD", 1.0, f64) * self.root_lp_secs;
                            let chunk = if chunked { tunable!("ENOMOTO_T_MIP_CUT0_RETRY_CHUNK", (lim2 / 40).max(500), u64) } else { lim2 };
                            if mode.contains('w') && !chunked {
                                let w = (lim2 / 4).max(50) as usize;
                                let frac = tunable!("ENOMOTO_T_MIP_CUT0_RETRY_FRAC", 1e-3, f64);
                                crate::simplex::mip_lp::PROGRESS_STOP_ONCE.with(|c| c.set(Some(((lim2 / 2) as usize, w, frac, prev_lp_obj))));
                            }
                            let it1 = self.lp.total_iterations();
                            let t1 = std::time::Instant::now();
                            let mut slow: Option<(u64, f64)> = None;
                            let mut cold_it0 = it1;
                            loop {
                                // 全スラック基底に切り替えた後は、反復の上限を切り替えたところから数え直す
                                let done = self.lp.total_iterations() - cold_it0;
                                let left = lim2.saturating_sub(done);
                                if left == 0 {
                                    break;
                                }
                                let k = if chunked && slow.is_none() { chunk.min(left) } else { left };
                                if chunked && slow.is_none() {
                                    crate::simplex::mip_lp::FORCE_RESUME_ONCE.with(|c| c.set(true));
                                }
                                let (i2, t2) = (self.lp.total_iterations(), std::time::Instant::now());
                                st = self.lp.solve(&SolveLimits { iteration_limit: k, cutoff: cut_cutoff, deadline: lp_deadline });
                                crate::simplex::mip_lp::FORCE_RESUME_ONCE.with(|c| c.set(false));
                                if !chunked || slow.is_some() || st != LpStatus::IterationLimit || self.time_up() || stopped(&job) {
                                    break;
                                }
                                let r2 = t2.elapsed().as_secs_f64() / (self.lp.total_iterations() - i2).max(1) as f64;
                                if t1.elapsed().as_secs_f64() > cold_secs {
                                    slow = Some((self.lp.total_iterations() - it1, r2));
                                    // 新しい LP と同じ出発にする: 全スラック基底を、点 x = 0 から非基底の置き場所を推す形で渡す
                                    // (`set_basis` は直前の点から推すので、温かい道の途中の点が残り、puriri では全スラック基底からでも
                                    // 30568 反復かかった。x = 0 なら 20299 反復)。止まった所の置き場所 (`resume_nb`) も消える
                                    let b = self.lp.basis();
                                    let (n, m) = (b.col.len(), b.row.len());
                                    let cols: Vec<usize> = (n..n + m).collect();
                                    self.lp.set_basis_cols(&cols, Some(&vec![0.0; n]));
                                    cold_it0 = self.lp.total_iterations();
                                }
                            }
                            crate::simplex::mip_lp::PROGRESS_STOP_ONCE.with(|c| c.set(None));
                            if self.params.verbose {
                                eprintln!(
                                    "MIP: cut round {round}: ENOMOTO_MIP_CUT0_RETRY: stopped at iter {it} after a gain of {total:.3e} from the start ({it_first} iters {secs_first:.2}s, {:.3} ms/iter vs root LP {:.3}); re-solved with the cap {lim2}{}: {st:?} after {} iters {:.2}s, obj {:.10e}",
                                    1e3 * rate,
                                    1e3 * root_rate,
                                    slow.map_or(String::new(), |(i, r)| format!(" (past {cold_secs:.2}s at {i} iters, last {:.3} ms/iter; the rest from the slack basis)", 1e3 * r)),
                                    self.lp.total_iterations() - it1,
                                    t1.elapsed().as_secs_f64(),
                                    self.lp.objective()
                                );
                            }
                            c0r_failed = st != LpStatus::Optimal;
                        } else if self.params.verbose {
                            eprintln!("MIP: cut round {round}: ENOMOTO_MIP_CUT0_RETRY: stopped at iter {it}, gain from the start {total:.3e}; not moved, dropped as by default");
                        }
                    }
                }
                let _ = ss;
            }
            // `d`: 上限 (反復・時間) に当たったら、上限を倍にして止まったところからもう 1 回だけ続ける
            if rob_d && matches!(st, LpStatus::IterationLimit | LpStatus::TimeLimit) && !self.time_up() && !stopped(&job) {
                let it1 = self.lp.total_iterations();
                let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(2.0 * cap);
                let d = if cap.is_finite() { Some(self.deadline.map_or(d, |g| g.min(d))) } else { self.deadline };
                let st0 = st;
                st = self.lp.solve(&SolveLimits { iteration_limit: 2 * solve_lim, cutoff: cut_cutoff, deadline: d });
                if self.params.verbose {
                    eprintln!("MIP: cut round {round}: cut LP hit the cap ({st0:?}, {} iters); continued with twice the cap {}: {st:?} after {} more iters", it1 - it0, 2 * solve_lim, self.lp.total_iterations() - it1);
                }
            }
            crate::simplex::slope_intercept_dual::DSE_EXACT_ONCE.store(false, std::sync::atomic::Ordering::Relaxed);
            if bg_dse_sample {
                crate::simplex::slope_intercept_dual::DSE_SAMPLE_ONCE.with(|c| c.set(None));
                crate::simplex::slope_intercept_dual::DSE_SAMPLE_LATE.with(|c| c.set(None));
                if self.params.verbose {
                    match crate::simplex::slope_intercept_dual::DSE_SAMPLE_RESULT.with(|c| c.take()) {
                        Some(r) => eprintln!(
                            "MIP: cut round {round}: background DSE sample: {} / {} rows, median |ln(exact/cur)| {:.3}, mean ln {:.3}, max {:.3}, spread {:.3}, frac>4x {:.3}; infeasible {} / {} rows, median |ln| {:.3}, mean ln {:.3}, spread {:.3}, frac>4x {:.3}; value {:.3} at iter {} ({:.3}s); exact weights {} ({:.3}s)",
                            r.all.n, r.m, r.all.med_abs, r.all.mean, r.all.max_abs, r.all.spread, r.all.frac, r.inf.n, r.n_inf, r.inf.med_abs, r.inf.mean, r.inf.spread, r.inf.frac, r.value, r.iter, r.sample_secs,
                            match r.full { 1 => "computed", 2 => "stopped", _ => "not computed" }, r.full_secs
                        ),
                        None => eprintln!("MIP: cut round {round}: background DSE sample: not taken (LP finished before the check, {} iters, or not a warm start)", self.lp.total_iterations() - it0),
                    }
                }
            }
            if let Some(dir) = &dump_dir {
                let prefix = format!("{dir}/cutlp_r{round}_{st:?}");
                let (b, w) = dump_basis.as_ref().unwrap();
                let r = self.lp.dump_lp(&prefix, b.as_deref(), w.as_deref());
                eprintln!("MIP: cut round {round}: LP {st:?} after {} iters ({:.3}s), written to {prefix}.mps/.bas ({r:?})", self.lp.total_iterations() - it0, t_lp0.elapsed().as_secs_f64());
                // 診断 (`ENOMOTO_MIP_DEBUG_CUT_LP_RESOLVE`): 上限で止まった LP を、同じスレッドで新しい LP として解き直す
                if st == LpStatus::TimeLimit && env_str!("ENOMOTO_MIP_DEBUG_CUT_LP_RESOLVE").is_some() {
                    self.lp.resolve_fresh_debug(b.as_deref(), w.as_deref());
                }
            }
            // `ENOMOTO_MIP_CUT_LP_CONTINUE` (試作、既定では使わない): 1 回の LP の時間の上限で止まったとき、カットを外さずに
            // 同じ上限でもう `ENOMOTO_T_MIP_CUT_LP_CONTINUE_MAX` 回 (既定 2) まで解き続ける (根のカットの時間の上限を過ぎていても)。
            // physiciansched6-2 (同値の代入の後) では 1 ラウンドに数百本のカットを入れた LP が上限 (3 秒) を少し超え、外したカットの
            // 分の反復が無駄になり、2 回で根のカットのループが止まっていた
            // `ENOMOTO_MIP_EQUIV2_CUTS` では `ENOMOTO_T_MIP_EQUIV2_CUT_LP_CONTINUE` (既定 1) のときだけ解き続ける
            let eq2_cont = eq2c && tunable!("ENOMOTO_T_MIP_EQUIV2_CUT_LP_CONTINUE", 1usize, usize) > 0;
            if st == LpStatus::TimeLimit && (env_str!("ENOMOTO_MIP_CUT_LP_CONTINUE").is_some() || extend_on || eq2_cont || self.rch_part('L')) && !self.params.submip {
                for _ in 0..tunable!("ENOMOTO_T_MIP_CUT_LP_CONTINUE_MAX", 2usize, usize) {
                    if self.time_up() || stopped(&job) {
                        break;
                    }
                    let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(cap);
                    let d = Some(self.deadline.map_or(d, |g| g.min(d)));
                    st = self.lp.solve(&SolveLimits { iteration_limit: lim, cutoff: cut_cutoff, deadline: d });
                    if self.params.verbose {
                        eprintln!("MIP: cut round {round}: LP continued after the time limit: {st:?}");
                    }
                    if st != LpStatus::TimeLimit {
                        break;
                    }
                }
            }
            // `ENOMOTO_T_MIP_CUT_LP_RETRY` = k (> 0、試作): 1 回の LP の時間の上限で止まったら、カットを外す前に同じ基底から
            // もう k 倍の時間だけ解き続ける。mzzv42z (10460 行) に implied bound カット 300 本余りを一度に入れると LP が
            // 1700 反復ほどかかり、上限 1.2 秒 (60 秒の 2%) に当たる回と当たらない回があって、当たると全部捨てて根の下界が
            // -21534 のまま終わる (当たらなければ -21203)
            // `ENOMOTO_MIP_GR_AUTO` (試作): 一般整数の列がある問題だけ 4 倍 (G のカットを入れた LP を捨てないため)
            let mut retry = tunable!("ENOMOTO_T_MIP_CUT_LP_RETRY", 0.0, f64);
            if retry == 0.0 && st == LpStatus::TimeLimit && super::gr_auto() && super::implbd_genint_for(&p.is_int, &p.col_lo, &p.col_up) {
                retry = 4.0;
            }
            if st == LpStatus::TimeLimit && retry > 0.0 && cap.is_finite() && !self.time_up() {
                let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(retry * cap);
                let d = Some(self.deadline.map_or(d, |g| g.min(d)));
                st = self.lp.solve(&SolveLimits { iteration_limit: lim, cutoff: cut_cutoff, deadline: d });
                if self.params.verbose {
                    eprintln!("MIP: cut round {round}: LP hit the time cap; continued for up to {:.2}s more: {st:?}", retry * cap);
                }
            }
            // 試作 `ENOMOTO_MIP_CUTLPCAP` の `a`: 上限に当たったら、効き目の大きい半分を残して加える前の状態から解き直す
            if let Some(ss) = cl_saved.as_ref().filter(|_| matches!(st, LpStatus::IterationLimit | LpStatus::TimeLimit)) {
                let amin = tunable!("ENOMOTO_T_MIP_CUTLPCAP_AMIN", 5usize, usize).max(1);
                let mut order: Vec<usize> = (0..rows.len()).collect();
                order.sort_by(|&i, &j| row_effs[j].partial_cmp(&row_effs[i]).unwrap_or(std::cmp::Ordering::Equal).then(i.cmp(&j)));
                let mut keep = rows.len();
                while matches!(st, LpStatus::IterationLimit | LpStatus::TimeLimit) && keep / 2 >= amin && !self.time_up() && !stopped(&job) {
                    keep /= 2;
                    let mut remove = vec![false; self.lp.num_rows()];
                    for r in remove.iter_mut().skip(saved_rows) {
                        *r = true;
                    }
                    self.delete_lp_rows(&remove);
                    if self.lp.num_rows() != saved_rows {
                        break;
                    }
                    self.lp.restore_state(ss);
                    let mut idx: Vec<usize> = order[..keep].to_vec();
                    idx.sort_unstable();
                    let sub: Vec<(Vec<(usize, f64)>, f64, f64)> = idx.iter().map(|&i| rows[i].clone()).collect();
                    self.add_cut_rows(&sub);
                    if rob_p {
                        let w = ((tunable!("ENOMOTO_T_MIP_CUT_LP_CAP_ROBUST_WIN", 0.25, f64) * lim as f64) as usize).max(50);
                        let frac = tunable!("ENOMOTO_T_MIP_CUT_LP_CAP_ROBUST_FRAC", 0.01, f64);
                        crate::simplex::mip_lp::PROGRESS_STOP_ONCE.with(|c| c.set(Some((lim as usize, w, frac, prev_lp_obj))));
                    }
                    let d = if cap.is_finite() {
                        let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(cap);
                        Some(self.deadline.map_or(d, |g| g.min(d)))
                    } else {
                        self.deadline
                    };
                    let it1 = self.lp.total_iterations();
                    st = self.lp.solve(&SolveLimits { iteration_limit: solve_lim, cutoff: cut_cutoff, deadline: d });
                    crate::simplex::mip_lp::PROGRESS_STOP_ONCE.with(|c| c.set(None));
                    if self.params.verbose {
                        eprintln!(
                            "MIP: cut round {round}: CUTLPCAP a: kept {keep} of {} cuts by efficacy and re-solved: {st:?} after {} iters, obj {:.10e}",
                            rows.len(),
                            self.lp.total_iterations() - it1,
                            self.lp.objective() + p.offset
                        );
                    }
                    if st == LpStatus::Optimal {
                        rows = idx.iter().map(|&i| rows[i].clone()).collect();
                        lp_cap_cuts = lp_cap_cuts.min(keep.max(amin));
                    }
                }
            }
            self.sb_iters += 0;
            loop_iters += self.lp.total_iterations() - it0;
            lp_tot += t_lp0.elapsed().as_secs_f64();
            self.last_cut_lp_secs = t_lp0.elapsed().as_secs_f64();
            if prof {
                sepprof::add(sepprof::T_LP, t_lp0.elapsed().as_secs_f64());
            }
            let _ = it0;
            if sl_on {
                self.sl_after_lp(st);
            }
            // `ENOMOTO_MIP_ROOT_CUT_CUTOFF`: LP の下界が打ち切り値に届いた (カットは大域的に成り立つので問題全体の下界)。根を閉じて終える
            if st == LpStatus::ObjectiveBound && cut_cutoff.is_finite() {
                self.root_cut_lb = self.root_cut_lb.max(cut_cutoff + p.offset);
                if self.root_gap_closed() {
                    if self.params.verbose {
                        eprintln!("MIP: cut round {round}: the cut LP reached the cutoff {:.10e}; root closed", cut_cutoff + p.offset);
                    }
                    closed_by_cutoff = true;
                    break;
                }
            }
            if st != LpStatus::Optimal {
                // 数値的に困ったら今回のカットを外して戻す
                let mut remove = vec![false; self.lp.num_rows()];
                for r in remove.iter_mut().skip(saved_rows) {
                    *r = true;
                }
                self.delete_lp_rows(&remove);
                // `ENOMOTO_MIP_CUT_FAIL_RESTORE`: 加える前の最適基底・LU・DSE 重みを戻す (行は加える前と同じ)
                if let Some(ss) = &saved_state {
                    self.lp.restore_state(ss);
                }
                // 試作 `ENOMOTO_MIP_CUT0_RETRY`: 解き直しが最適にならなかったら、加える前の状態に戻す
                if c0r_failed {
                    if let Some(ss) = &c0r_saved {
                        if self.lp.num_rows() == saved_rows {
                            self.lp.restore_state(ss);
                        }
                    }
                }
                // 試作 `ENOMOTO_MIP_CUT_LP_CAP_ROBUST` の `d`: 止まった基底からの解き直しにせず、加える前の最適基底に戻す
                if let Some(ss) = &rob_saved {
                    if self.lp.num_rows() == saved_rows {
                        self.lp.restore_state(ss);
                    }
                }
                let it2 = self.lp.total_iterations();
                let t_re = std::time::Instant::now();
                let lp_secs_failed = t_lp0.elapsed().as_secs_f64();
                let st2 = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
                if self.params.verbose && saved_state.is_some() {
                    eprintln!("MIP: cut round {round}: re-solve from the saved state: {} iters", self.lp.total_iterations() - it2);
                }
                if self.params.verbose {
                    eprintln!(
                        "MIP: cut round {round}: LP status {st:?}, removed the cuts ({st2:?}; cut LP {} iters {lp_secs_failed:.2}s, re-solve {} iters {:.2}s, {:.2}s)",
                        it2 - it0,
                        self.lp.total_iterations() - it2,
                        t_re.elapsed().as_secs_f64(),
                        self.start.elapsed().as_secs_f64()
                    );
                }
                if self.params.verbose && st == LpStatus::TimeLimit {
                    if let Some(k) = s6_tcap {
                        eprintln!("MIP: cut round {round}: ENOMOTO_MIP_S6_CUTLP_TCAP={k}: cut LP time cap {cap:.2}s (longest solved cut LP {ok_lp_secs:.2}s)");
                    }
                }
                let capped = matches!(st, LpStatus::TimeLimit | LpStatus::IterationLimit);
                // `ENOMOTO_MIP_CUT_LP_HALVE`: 時間の上限に当たっただけなら、本数を半分にして続ける
                // (`ENOMOTO_MIP_BG_CUTS_LP_WORK=..h`: 写しでは反復の上限に当たったときも)
                let halve_now = (lp_halve && st == LpStatus::TimeLimit) || (bg_halve && capped);
                if halve_now && st2 == LpStatus::Optimal && rows.len() > lp_halve_min && !self.time_up() && !stopped(&job) {
                    lp_cap_cuts = (rows.len() / 2).max(lp_halve_min);
                    if self.params.verbose {
                        eprintln!("MIP: cut round {round}: the LP with {} cuts hit the time cap ({st:?}); up to {lp_cap_cuts} cuts per round from now", rows.len());
                    }
                    continue;
                }
                loop_iters += self.lp.total_iterations() - it2;
                // `ENOMOTO_MIP_CUT_LP_CAP_STOP`: 1 回の LP の上限に当たったら止める (`flat` なら根の LP の値から動いていないときだけ)
                if capped && st2 == LpStatus::Optimal && lpcap_stop.is_some_and(|v| v != "flat" || !moved) {
                    if self.params.verbose {
                        eprintln!("MIP: cut round {round}: the cut LP hit the per-LP cap ({st:?}); stop (ENOMOTO_MIP_CUT_LP_CAP_STOP)");
                    }
                    break;
                }
                // `ENOMOTO_T_MIP_CUT_FLAT_STOP`: 外したラウンドも動かないラウンドと数える
                if flat_stop > 0 && !moved && flat_fail && st2 == LpStatus::Optimal {
                    flat_rounds += 1;
                    if flat_rounds >= flat_stop {
                        if self.params.verbose {
                            eprintln!("MIP: cut loop: LP bound unchanged from the root LP for {flat_rounds} rounds (incl. a dropped round); stop (ENOMOTO_T_MIP_CUT_FLAT_STOP)");
                        }
                        break;
                    }
                }
                // 1 回の失敗ではやめない (そのラウンドのカットだけ捨てて続ける。2 回目でやめる)
                lp_failures += 1;
                // `ENOMOTO_MIP_EQUIV2_CUTS` では `ENOMOTO_T_MIP_EQUIV2_CUT_LP_FAILS` 回目 (既定 2) でやめる
                let max_fail = if eq2c { tunable!("ENOMOTO_T_MIP_EQUIV2_CUT_LP_FAILS", 2usize, usize) } else { 2 };
                if st2 != LpStatus::Optimal || lp_failures >= max_fail || env_str!("ENOMOTO_MIP_CUT_LP_FAIL_STOP").is_some() {
                    return st2 == LpStatus::Optimal;
                }
                continue;
            }
            ok_lp_secs = ok_lp_secs.max(t_lp0.elapsed().as_secs_f64());
            ok_lp_iters = ok_lp_iters.max(self.lp.total_iterations() - it0);
            cl_last_ok_cuts = rows.len();
            let obj = self.lp.objective();
            // 最適に解けたカットの LP の値は問題全体の下界 (根を閉じる判定 [`Solver::root_gap_closed`] に使う)
            if job.is_none() && self.snap.is_none() {
                self.root_cut_lb = self.root_cut_lb.max(obj + p.offset);
                // `ENOMOTO_MIP_TREE4_RCUNION` (試作): このラウンドの LP の被約費用を根の被約費用による固定に足す
                self.tree4_rc_add(obj + p.offset);
            }
            // `ENOMOTO_MIP_CUT_WIDEN_DOUBLE` (既定): 一気に行の数 (1000 本まで) に広げるのでなく、採り切っても動かないラウンドのたびに
            // 2 倍にする (50 -> 100 -> 200 -> ... -> 1000)。physiciansched6-2 では 50 本から一気に広げたラウンドで 247〜308 本が入り、
            // LP が 1 回の時間の上限に当たって全部外されていた (`ENOMOTO_MIP_CUT_LP_HALVE` の半分化と対になる)
            let widen_now = widen_on && (!widened || widen_double_on) && quota_full && (obj - prev_obj) / obj.abs().max(prev_obj.abs()).max(1.0) <= 1e-4;
            if widen_now {
                let wide_max = p.m.min(tunable!("ENOMOTO_T_MIP_CUT_WIDE_MAX", 1000usize, usize));
                if !widened {
                    max_rounds = max_rounds.max(round_ctr + tunable!("ENOMOTO_T_MIP_CUT_WIDE_ROUNDS", 100usize, usize));
                }
                widened = true;
                adapt_max_cuts = if widen_double_on { (2 * max_cuts).min(wide_max) } else { wide_max }.max(max_cuts);
                if self.params.verbose {
                    eprintln!("MIP: cut round {round}: {max_cuts} cuts did not move the objective; up to {adapt_max_cuts} cuts per round from now");
                }
            }
            // 試作 `ENOMOTO_MIP_OPTFACE_PARCUTS` の `g`: 下界を上げたラウンドの後、LP で効いているカットの行 (まだ渡していないもの) を渡す
            if share_gated && obj > prev_obj + 1e-9 * obj.abs().max(1.0) {
                if let Some((sh, id)) = job.as_ref().and_then(|j| j.share.lock().unwrap().clone()) {
                    let duals = self.lp.row_duals();
                    let mut v = sh.cuts.lock().unwrap();
                    for i in self.p.m..self.lp.num_rows().min(duals.len()) {
                        if duals[i].abs() <= 1e-9 {
                            continue;
                        }
                        let c = self.lp.row(i);
                        let (_, u) = self.lp.row_bounds(i);
                        if !u.is_finite() {
                            continue;
                        }
                        let k: Vec<(usize, u64)> = c.iter().map(|&(j, a)| (j, a.to_bits())).collect();
                        if share_keys.contains(&k) || !share_pub.insert(k) {
                            continue;
                        }
                        v.push((id, c, u));
                    }
                }
            }
            if let Some(j) = &job {
                // 試作 `ENOMOTO_MIP_ERWAIT` の `k`: このラウンドの LP (最適) のカットの行と基底を取っておく (主が途中で止めたときに使う)
                if super::solver::erwait('b') && super::solver::erwait('k') {
                    let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = (p.m..self.lp.num_rows())
                        .map(|i| {
                            let (l, u) = self.lp.row_bounds(i);
                            (self.lp.row(i), l, u)
                        })
                        .collect();
                    *j.last_ok.lock().unwrap() = Some((rows, obj + p.offset, round_ctr, self.lp.basis()));
                }
                if let Some(why) = j.record_round(obj + p.offset, self.lp.col_values(), self.lp.total_iterations(), phase2_on && !phase2) {
                    if self.params.verbose {
                        eprintln!("MIP: cut round {round}: background cuts stop by themselves ({why}, {} LP iters, obj {:.10e})", self.lp.total_iterations(), obj + p.offset);
                    }
                }
            }
            // 試作 `ENOMOTO_MIP_BG_CUTS_DETERM` の `c` (主が裏のカットの続きを回す): 直近の窓の伸びで停滞を決める
            if let Some((_, win, eps)) = self.cut_cont {
                obj_cont.push(obj + p.offset);
                if super::solver::window_stalled(&obj_cont, win, eps) {
                    if self.params.verbose {
                        eprintln!("MIP: cut round {round}: continued background cuts stalled (obj {:.10e})", obj + p.offset);
                    }
                    cont_stop = true;
                }
            }
            if dbg_sep {
                eprintln!(
                    "SEP round {round}: cands {ncands} (nnz {cand_nnz}) pool {pool_secs:.3}s select {sel_secs:.3}s add_rows {add_secs:.3}s LP {:.3}s ({} iters, {:.1} us/iter)",
                    t_lp0.elapsed().as_secs_f64(),
                    self.lp.total_iterations() - it0,
                    1e6 * t_lp0.elapsed().as_secs_f64() / (self.lp.total_iterations() - it0).max(1) as f64
                );
            }
            if env_str!("ENOMOTO_MIP_DEBUG_FC").is_some() && !self.params.submip {
                super::cuts::FC_STATS.with(|s| eprintln!("FC stats (calls, cuts, no SNF): {:?}", s.borrow()));
            }
            if self.params.verbose {
                let cut_nnz: usize = (p.m..self.lp.num_rows()).map(|i| self.lp.row_ref(i).len()).sum();
                let ncut_rows = self.lp.num_rows() - p.m;
                eprintln!(
                    "MIP: cut round {round}: {} cuts (of {ncands}, sep {sep_secs:.2}s, LP {} iters), LP rows {} (cut rows mean nnz {:.0}), obj {:.10e} ({:.2}s)",
                    rows.len(),
                    self.lp.total_iterations() - it0,
                    self.lp.num_rows(),
                    cut_nnz as f64 / ncut_rows.max(1) as f64,
                    obj + p.offset,
                    self.start.elapsed().as_secs_f64()
                );
            }
            // `ENOMOTO_MIP_RR_EACH_ROUND` (試作、既定では使わない): サブ MIP の中か暫定解がないときは、各ラウンドの LP 解で乱択丸め
            // ([`Self::randomized_rounding`]) を `ENOMOTO_T_MIP_RR_EACH_ROUND_TRIES` (1) 回行う (HiGHS の `rootSeparationRound` と同じ)。
            // physiciansched6-2 の根の RENS の近傍のサブ MIP は、HiGHS ならラウンドの途中の LP 解の乱択丸めで解が出るが、ENOMOTO は
            // ループの後の LP 解でしか丸めない (analysis/physhighs_20261024.md)。`ENOMOTO_MIP_RR_HIGHS` と合わせて使う (1 回目から乱数の閾値)
            if env_str!("ENOMOTO_MIP_RR_EACH_ROUND").is_some() && job.is_none() && (self.params.submip || self.incumbent.is_none()) {
                let xr = self.lp.col_values();
                if !self.fractional(&xr).is_empty() {
                    let found = self.randomized_rounding(&xr, tunable!("ENOMOTO_T_MIP_RR_EACH_ROUND_TRIES", 1usize, usize));
                    if found && self.params.verbose {
                        eprintln!("MIP: cut round {round}: randomized rounding of the round's LP solution found {:?}", self.incumbent.as_ref().map(|(z, _)| z + 0.0));
                    }
                }
            }
            obj_hist.push(obj);
            round_obj.push(obj);
            // `ENOMOTO_T_MIP_CUT_FLAT_STOP`: 根の LP の値から動かないラウンドが最初から k 回続いたら止める
            // (`moved` は `ENOMOTO_MIP_CUT_LP_CAP_STOP=flat` も使う)
            if !moved && obj - first_obj > flat_eps * first_obj.abs().max(1.0) {
                moved = true;
            }
            if flat_stop > 0 && !moved {
                flat_rounds += 1;
                if flat_rounds >= flat_stop {
                    if self.params.verbose {
                        eprintln!("MIP: cut loop: LP bound unchanged from the root LP for {flat_rounds} rounds; stop (ENOMOTO_T_MIP_CUT_FLAT_STOP)");
                    }
                    break;
                }
            }
            if prog_win > 0 && obj_hist.len() > prog_win {
                let total = obj - first_obj;
                let recent = obj - obj_hist[obj_hist.len() - 1 - prog_win];
                if total > 1e-9 * obj.abs().max(1.0) && recent < prog_eps * total {
                    if self.params.verbose {
                        eprintln!("MIP: cut loop: progress of the last {prog_win} rounds {recent:.3e} < {prog_eps} x total {total:.3e}; stop");
                    }
                    break;
                }
            }
            if highs_stall {
                hrounds += 1;
                let xs = self.lp.col_values();
                let cur: Vec<f64> = (0..n).map(|j| first_x[j] - xs[j]).collect();
                let nrm = cur.iter().map(|v| v * v).sum::<f64>().sqrt();
                let scale = if nrm > 0.0 { 1.0 / nrm } else { 0.0 };
                let (mut sq, mut dot) = (0.0, 0.0);
                for j in 0..n {
                    avgdir[j] = (scale * cur[j] - avgdir[j]) / hrounds as f64;
                    sq += avgdir[j] * avgdir[j];
                    dot += avgdir[j] * cur[j];
                }
                let progress = if sq > 0.0 { dot / sq.sqrt() } else { 0.0 };
                if hrounds == 1 {
                    smooth = progress;
                } else {
                    let next = (2.0 / 3.0) * smooth + progress / 3.0;
                    if next < smooth * 1.01 && obj - first_obj <= (prev_obj - first_obj) * 1.001 {
                        hstall += 1;
                        if hstall >= 3 {
                            break;
                        }
                    } else {
                        hstall = 0;
                    }
                    smooth = next;
                }
            } else if env_str!("ENOMOTO_MIP_STALL_OLD").is_some() {
                // 停滞判定: 改善が初回からの改善量のわずかな割合なら停滞
                let gain = obj - prev_obj;
                let scale = (obj - first_obj).abs().max(1e-6 * obj.abs().max(1.0));
                if gain <= 1e-3 * scale || gain <= 1e-9 * obj.abs().max(1.0) {
                    stall += 1;
                    if stall >= 3 {
                        break;
                    }
                } else {
                    stall = 0;
                }
            } else {
                // SCIP の停滞判定 (solve.c): 目的値の相対変化が 1e-4 以下で、分数の列の数も十分に減っていなければ
                // 停滞。根では 10 回分で止める (サブ MIP では 3 回)。目的値も分数の列の数もまったく減らない
                // ラウンドは 2 回分と数える (何も進まない問題で長く回さない。分数の列が減っている間は続ける)。
                let nfrac = self.fractional(&self.lp.col_values()).len();
                let reldiff = (obj - prev_obj) / obj.abs().max(prev_obj.abs()).max(1.0);
                // `ENOMOTO_MIP_HO6_STALL=割合` (試作、既定では使わない。`1` なら 1e-3): 根 (サブ MIP でない) の停滞の判定の
                // 「目的値の相対変化」を、目的値の大きさではなくカットで上げた分 (今の値 - カットの前の値) で測る (HiGHS の
                // 「下界の伸びが今までの伸びの 0.1% 以下」と同じ尺度)。unitcal_7 は目的値 1.96e7 に対して根のギャップが 0.35% しかなく、
                // 1 ラウンド 300〜1000 (今までの伸びの 0.3〜1%) 伸びていても相対変化 1e-4 未満で停滞と数えられ、17 ラウンド前後で止まる
                // (HiGHS は 47 ラウンド)。この割合以下の伸びのラウンドを停滞とする (分数の列の数の条件は今までどおり)
                let reldiff = match ho6_stall_frac() {
                    Some(t) if !self.params.submip => {
                        let tot = (obj - first_obj).max(1e-9 * obj.abs().max(1.0));
                        if obj - prev_obj <= t * tot { 0.0 } else { 1.0 }
                    }
                    _ => reldiff,
                };
                // `ENOMOTO_MIP_CUTSTALL_GAP`: 今までの判定 (相対変化 1e-4) で停滞でも、ギャップの尺度で伸びていれば停滞と数えない
                let reldiff = match cs_gap {
                    Some((t, _)) if reldiff <= 1e-4 => {
                        let gain = obj - prev_obj;
                        if gain > t * cs_scale(self, obj) && gain > 1e-9 * obj.abs().max(1.0) { 1.0 } else { reldiff }
                    }
                    _ => reldiff,
                };
                let rs = rate_stop.as_mut().map(|r| r.record(obj, self.lp.total_iterations() - it0));
                if rs == Some(super::rootstop::Rate::Stop) {
                    if self.params.verbose {
                        let (r, r0) = rate_stop.as_ref().map_or((0.0, 0.0), |r| r.last);
                        eprintln!("MIP: cut round {round}: ROOTSTOP r: rate {r:.3e} < {} x first {r0:.3e}; stop (obj {:.10e})", tunable!("ENOMOTO_T_MIP_ROOTSTOP_RATE", 0.05, f64), obj + p.offset);
                    }
                    break;
                }
                let reldiff = if rs == Some(super::rootstop::Rate::Continue) { 1.0 } else { reldiff };
                if phase2 {
                    // 第 2 段階: 目的値だけで停滞を数える
                    if reldiff <= 1e-4 {
                        stall += 1;
                        if stall >= tunable!("ENOMOTO_T_MIP_CUT_PHASE2_STALL", 10usize, usize) {
                            if veto_own(&job) {
                                if self.params.verbose {
                                    eprintln!("MIP: cut round {round}: phase-2 stall vetoed (ENOMOTO_MIP_BG_DETERM_ROBUST)");
                                }
                                stall = 0;
                            } else {
                                break;
                            }
                        }
                    } else {
                        stall = 0;
                    }
                } else {
                    // `ENOMOTO_MIP_CUT_STALL_ONE`: まったく進まないラウンドも 1 回分と数える (neos-1456979 では最初の
                    // 4 ラウンドは下界が動かず、続ければ上がる。HiGHS も止めずに 66 ラウンド回して 154 -> 171)
                    // `ENOMOTO_MIP_CUT_INTOBJ_STALL` (試作、既定では使わない): 目的値が整数刻みの問題で、目的値が一度上がった後に
                    // まったく動かないラウンドは、分数の列の数が減っていても停滞と数え、`ENOMOTO_T_MIP_CUT_INTOBJ_STALL` 回 (既定 3) で
                    // 止める。10teams は 0.5 秒 (ラウンド 0) で 917 -> 924 (最適値) になった後、分数の列の数が上下するので停滞と
                    // 数えられず、14 ラウンド (約 6 秒) 回してから根のヒューリスティクスに進んでいた。最初から動かない
                    // ラウンド (neos-1456979) は数えない。整数刻みかは整数値に固定された連続列を整数列と扱って判定する (10teams は費用のある固定の連続列
                    // 225 本のために `obj_step` が無い)
                    let intobj_flat = (intobj_step || gap2_flat.is_some())
                        && !self.params.submip
                        && obj - first_obj > 1e-6 * obj.abs().max(1.0)
                        && (obj - prev_obj).abs() <= 1e-9 * obj.abs().max(1.0)
                        && env_def!("ENOMOTO_MIP_CUT_INTOBJ_STALL", "1").is_some();
                    if intobj_flat {
                        intobj_stall += 1;
                        if intobj_stall >= gap2_flat.unwrap_or_else(|| tunable!("ENOMOTO_T_MIP_CUT_INTOBJ_STALL", 3usize, usize)) {
                            if self.params.verbose {
                                eprintln!("MIP: cut loop: {}, LP bound unchanged for {intobj_stall} rounds; stop", if gap2_flat.is_some() { "GAP2_FLAT" } else { "integral objective" });
                            }
                            break;
                        }
                    } else {
                        intobj_stall = 0;
                    }
                    // 試作 `ENOMOTO_MIP_GAP2_ZEROCAP=m` (既定では使わない。analysis/gap2_rootcost_20261109.md): 外側の根で、ループの始めから
                    // 目的値がまったく動かないまま、ループの経過 (壁時計) が max(m x 根の LP の秒, `ENOMOTO_T_MIP_GAP2_ZEROCAP_MIN` (1 秒)) を
                    // 超えたら止める。neos-960392 は根の LP の値 -238 が最適値で、1 ラウンドの分離が 1〜6 秒 (密なカット) のまま
                    // 下界が全く動かないラウンドを 6〜14 回 (13〜58 秒) 回していた (HiGHS は 5 秒ほどで抜ける)
                    if !self.params.submip && job.is_none() && (obj - first_obj).abs() <= 1e-9 * obj.abs().max(1.0) {
                        if let Some(m) = env_str!("ENOMOTO_MIP_GAP2_ZEROCAP").and_then(|v| v.parse::<f64>().ok()).filter(|&m| m > 0.0) {
                            let lim = (m * self.root_lp_secs).max(tunable!("ENOMOTO_T_MIP_GAP2_ZEROCAP_MIN", 1.0, f64));
                            let el = t_loop_wall.elapsed().as_secs_f64();
                            if el > lim {
                                if self.params.verbose {
                                    eprintln!("MIP: cut loop: GAP2_ZEROCAP, LP bound unchanged since the loop start for {el:.2}s (limit {lim:.2}s, root LP {:.2}s); stop", self.root_lp_secs);
                                }
                                break;
                            }
                        }
                    }
                    if reldiff <= 1e-4 && nfrac as f64 >= (0.9 - 0.1 * stall as f64) * prev_nfrac as f64 && !saturated && !widen_now {
                        stall += if nfrac >= prev_nfrac && env_str!("ENOMOTO_MIP_CUT_STALL_ONE").is_none() && !widened { 2 } else { 1 };
                        if stall >= if fc_sub { tunable!("ENOMOTO_T_MIP_FC_SUBMIP_CUT_STALL", 10usize, usize) } else if self.params.submip && !sub_hc { tunable!("ENOMOTO_T_MIP_SUBMIP_CUT_STALL", 3usize, usize) } else if heavy { tunable!("ENOMOTO_T_MIP_BG_CUTS_STALL", 20usize, usize) } else { tunable!("ENOMOTO_T_MIP_CUT_STALL", 10usize, usize) } {
                            if !phase2_on && veto_own(&job) {
                                if self.params.verbose {
                                    eprintln!("MIP: cut round {round}: own stall vetoed (ENOMOTO_MIP_BG_DETERM_ROBUST)");
                                }
                                stall = 0;
                            } else if !phase2_on {
                                if cs_gap.is_some() && self.params.verbose {
                                    eprintln!("MIP: cut round {round}: stall stop (ENOMOTO_MIP_CUTSTALL_GAP, obj {obj:.10e})");
                                }
                                break;
                            } else {
                            // 第 2 段階へ (ラウンド数の上限を延ばす)
                            phase2 = true;
                            stall = 0;
                            max_rounds = round_ctr + tunable!("ENOMOTO_T_MIP_CUT_PHASE2_ROUNDS", 60usize, usize);
                            let t1 = super::cap_clock() - t_loop0;
                            phase2_deadline = t1 + (tunable!("ENOMOTO_T_MIP_CUT_PHASE2_TIME_MULT", 3.0, f64) * t1).max(0.2);
                            if self.params.verbose {
                                eprintln!("MIP: cut loop phase 2 (HiGHS cut score) from round {} ({:.2}s)", round + 1, self.start.elapsed().as_secs_f64());
                            }
                            }
                        }
                    } else {
                        stall = 0;
                    }
                }
                prev_nfrac = nfrac;
            }
            self.sep_last_gain = (obj - prev_obj) / obj.abs().max(prev_obj.abs()).max(1.0);
            prev_obj = obj;
            obj_hist.push(obj);
            ext_hist.push(obj);
            // 効いていないカットを外す (論理変数が基底にあり、行が緩んでいるもの)。`ENOMOTO_T_MIP_CUT_REMOVE_EVERY` ラウンドごと
            // `ENOMOTO_T_MIP_ROOT_CUT_AGE` > 0: HiGHS と同じく、効いていないラウンドが続いたカットだけを外す
            // (HiGHS の mip_lp_age_limit = 10)
            let root_age = if self.cutsel_guard_fc() { 0 } else { tunable!("ENOMOTO_T_MIP_ROOT_CUT_AGE", if self.pool_part('G') { 10u32 } else { 0 }, u32) };
            let t_rm0 = std::time::Instant::now();
            let rows_before = self.lp.num_rows();
            if root_age > 0 {
                self.age_cuts();
                let removed = self.remove_cuts_older_than(root_age);
                if env_str!("ENOMOTO_MIP_DEBUG_DSE_CHECK").is_some() && !self.params.submip && removed > 0 {
                    self.lp.debug_dse_check(&format!("round {round} after removing {removed} old cuts"));
                }
            } else if (round + 1) % tunable!("ENOMOTO_T_MIP_CUT_REMOVE_EVERY", 1usize, usize).max(1) == 0 {
                if keep_lu {
                    // `ENOMOTO_MIP_CUT_KEEP_LU`: 次のラウンドでカットを加えるときに外す (ループを出たら下で外す)
                    keep_lu_pending = true;
                } else {
                    self.remove_inactive_cuts();
                }
            }
            if dbg_sep {
                eprintln!("SEP round {round}: removed {} inactive cuts in {:.3}s", rows_before - self.lp.num_rows(), t_rm0.elapsed().as_secs_f64());
            }
            if prof {
                sepprof::add(sepprof::T_REMOVE, t_rm0.elapsed().as_secs_f64());
                sepprof::add(sepprof::T_ROUND, t_round0.elapsed().as_secs_f64());
            }
        }
        if prof {
            sepprof::report(&format!("restarts {}", self.params.restarts));
        }
        if job.is_none() {
            self.sep_adapt_finish_root();
            if self.sl.stat && !self.params.submip && self.params.verbose {
                eprint!("{}", self.sl.report(&format!("root restarts {}", self.params.restarts)));
            }
        }
        if closed_by_cutoff {
            // `ENOMOTO_MIP_ROOT_CUT_CUTOFF`: 根を閉じた (呼び出し側が [`Solver::root_gap_closed`] で終える)。LP は最適ではないが、
            // 終わった後に LP を使わない
            user_log!("Root cuts: the cut LP reached the cutoff ({:.2}s)", self.start.elapsed().as_secs_f64());
            return true;
        }
        self.remove_inactive_cuts();
        if mpsep.is_some() && self.params.verbose {
            eprintln!(
                "MIP: mpsep{}: {} rounds with extra points, {} points, {} extra candidates, chosen {} cuts ({} do not cut x*), {:.3}s",
                if job.is_some() { " (background cuts)" } else { "" },
                mp_stats.0,
                mp_stats.1,
                mp_stats.2,
                mp_stats.4,
                mp_stats.3,
                mp_stats.5
            );
        }
        if env_str!("ENOMOTO_MIP_DEBUG_FACETIME").is_some() && !self.params.submip {
            eprintln!(
                "FACETIME {}restarts {} rounds {nrounds}: sep {sep_tot:.2}s (face {:.2}s, {} vertices), LP {lp_tot:.2}s, loop {:.2}s, obj {:.10e}",
                if job.is_some() { "bg " } else { "root " },
                self.params.restarts,
                self.face_secs - face0.0,
                self.face_used - face0.1,
                t_loop_wall.elapsed().as_secs_f64(),
                self.lp.objective() + p.offset
            );
        }
        let st = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
        if clique_fb && st == LpStatus::Optimal && job.is_none() && !self.time_up() {
            let on = self.clique_fallback_trial(root_iters, first_obj);
            if self.params.restarts == 0 {
                CLIQUE_AUTO_FIRST.with(|c| c.set(Some(on)));
            }
            self.clique_auto = Some(on);
            if on {
                // clique カットを加えてループを続ける (2 回目のループは判定しない)
                CLIQUE_FB_SECOND.with(|c| c.set(true));
                let ok = self.root_cut_loop(root_iters);
                CLIQUE_FB_SECOND.with(|c| c.set(false));
                return ok;
            }
        }
        // `ENOMOTO_MIP_ZH_TRIAL` (試作): zerohalf なしで回し終えた根で zerohalf を試し、使うなら zerohalf を加えてループを続ける
        // (2 回目のループの時間の上限は clique の fallback と同じくループの始めから数える)。再スタートの後は最初の判定を引き継ぐ
        if let Some(zm) = super::zerohalf::zh_trial_mode() {
            if st == LpStatus::Optimal && job.is_none() && !self.params.submip && !super::zerohalf::zh_trial_decided() && env_def!("ENOMOTO_MIP_NO_ZEROHALF", "1").is_some() && !self.time_up() {
                // `r`: 試しの LP をせずに使う (根の続きと裏のカットだけ。木では [`super::zerohalf::zh_trial_used`] が偽)
                let on = if zm == "r" { true } else { self.zh_trial(root_iters, first_obj) && zm == "on" };
                if zm == "r" && self.params.verbose {
                    eprintln!("MIP: zh trial (r): continuing the root cut loop with zerohalf ({:.2}s)", self.start.elapsed().as_secs_f64());
                }
                super::zerohalf::zh_trial_set(on);
                if on {
                    CLIQUE_FB_SECOND.with(|c| c.set(true));
                    let ok = self.root_cut_loop(root_iters);
                    CLIQUE_FB_SECOND.with(|c| c.set(false));
                    return ok;
                }
            }
        }
        if !self.params.submip && st == LpStatus::Optimal && job.is_none() {
            super::record_root_bound(self.lp.objective() + p.offset);
        }
        user_log!(
            "Root cuts: objective {:.10e} -> {:.10e}, {} cuts in the LP ({:.2}s)",
            p.sense_sign * (first_obj + p.offset),
            p.sense_sign * (self.lp.objective() + p.offset),
            self.lp.num_rows() - p.m,
            self.start.elapsed().as_secs_f64()
        );
        if self.params.verbose && super::cmirdelta::on() && !self.params.submip {
            eprintln!("{}", super::cmirdelta::summary());
        }
        if self.params.verbose && super::hnfcut::on() && !self.params.submip {
            eprintln!("MIP: root {}", super::hnfcut::stats_line());
        }
        if self.params.verbose {
            eprintln!(
                "MIP: cut loop done: {} cuts added, {} in LP, pool {} cuts ({} nonzeros), obj {:.10e} -> {:.10e}",
                total_added,
                self.lp.num_rows() - p.m,
                self.cut_pool.len(),
                self.cut_pool_nnz,
                first_obj + p.offset,
                self.lp.objective() + p.offset
            );
        }
        st == LpStatus::Optimal
    }

    /// カットをプールに加える。係数と右辺が (丸めて) 同じものは加えない。プールの非零数は問題の非零数の
    /// 20 倍 (最低 20 万) までにする。
    pub(super) fn add_to_pool(&mut self, coefs: &[(usize, f64)], rhs: f64) {
        let nnz_cap = (tunable!("ENOMOTO_T_MIP_CUT_POOL_NNZ_MULT", 20usize, usize) * self.p.nnz).max(200_000);
        // 最大係数 1 に揃えて 1e-9 の格子で丸めたもののハッシュ
        use std::hash::{Hash, Hasher};
        let cmax = coefs.iter().fold(0.0f64, |m, &(_, v)| m.max(v.abs())).max(1e-300);
        let mut hs = std::collections::hash_map::DefaultHasher::new();
        for &(j, v) in coefs {
            j.hash(&mut hs);
            ((v / cmax) * 1e9).round().to_bits().hash(&mut hs);
        }
        ((rhs / cmax) * 1e9).round().to_bits().hash(&mut hs);
        let key = hs.finish();
        if let Some(&k) = self.cut_pool_keys.get(&key) {
            // 既にある: 使われたので年齢を戻す
            self.pool_age[k] = 0;
            return;
        }
        if self.cut_pool_nnz + coefs.len() > nnz_cap && !self.pool_nocap {
            if env_str!("ENOMOTO_MIP_POOL_EVICT").is_none() {
                return;
            }
            self.evict_pool(nnz_cap);
            if self.cut_pool_nnz + coefs.len() > nnz_cap {
                return;
            }
        }
        self.cut_pool_keys.insert(key, self.cut_pool.len());
        self.pool_hash.push(key);
        self.pool_age.push(0);
        let norm = coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
        self.cut_pool_nnz += coefs.len();
        self.cut_pool.push((coefs.to_vec(), rhs, norm));
        for &(j, v) in coefs {
            self.pool_idx.push(j as u32);
            self.pool_val.push(v);
        }
        self.pool_start.push(self.pool_idx.len() as u32);
        // 木を分け合う: 新しいカットをほかのスレッドにも渡す (取り込み中のものは渡し返さない)
        if !self.importing && self.snap.is_some() && super::solver::tree_share_cuts() {
            if let Some(pool) = &self.pool {
                super::solver::learned_push(pool, self.pool_tid, super::solver::LearnedItem::Cut(coefs.to_vec(), rhs));
            }
        }
    }

    /// プールの年齢を 1 進める。
    pub(super) fn age_pool(&mut self) {
        for a in self.pool_age.iter_mut() {
            *a = a.saturating_add(1);
        }
    }

    /// プールが上限 `nnz_cap` に達したとき、年齢の大きい (長く使われていない) カットから非零数が上限の
    /// `ENOMOTO_T_MIP_POOL_EVICT_TO` 倍 (既定 0.8) になるまで捨てる (HiGHS の `HighsCutPool` の agelim に相当)。
    /// 年齢 0 (今のラウンドで加えた・使った) のカットは残す。
    fn evict_pool(&mut self, nnz_cap: usize) {
        let target = (tunable!("ENOMOTO_T_MIP_POOL_EVICT_TO", 0.8, f64) * nnz_cap as f64) as usize;
        let mut ord: Vec<usize> = (0..self.cut_pool.len()).filter(|&k| self.pool_age[k] > 0).collect();
        // 年齢の大きい順、同じなら古い (番号の小さい) 順
        ord.sort_by(|&a, &b| self.pool_age[b].cmp(&self.pool_age[a]).then(a.cmp(&b)));
        let mut drop = vec![false; self.cut_pool.len()];
        let mut nnz = self.cut_pool_nnz;
        for k in ord {
            if nnz <= target {
                break;
            }
            drop[k] = true;
            nnz -= self.cut_pool[k].0.len();
        }
        if !drop.iter().any(|&d| d) {
            return;
        }
        // カットの番号が変わるので、カットプールの伝播の差分評価を作り直させる
        self.cpp.valid = false;
        let old = std::mem::take(&mut self.cut_pool);
        let (hash, age) = (std::mem::take(&mut self.pool_hash), std::mem::take(&mut self.pool_age));
        self.cut_pool_keys.clear();
        self.pool_idx.clear();
        self.pool_val.clear();
        self.pool_start.clear();
        self.pool_start.push(0);
        self.cut_pool_nnz = 0;
        for (k, c) in old.into_iter().enumerate() {
            if drop[k] {
                continue;
            }
            self.cut_pool_keys.insert(hash[k], self.cut_pool.len());
            self.pool_hash.push(hash[k]);
            self.pool_age.push(age[k]);
            self.cut_pool_nnz += c.0.len();
            for &(j, v) in &c.0 {
                self.pool_idx.push(j as u32);
                self.pool_val.push(v);
            }
            self.pool_start.push(self.pool_idx.len() as u32);
            self.cut_pool.push(c);
        }
    }

    /// カットプールのうち LP 解 `x` が違反するカットの (効き目, 番号)。平坦な複製 (`pool_idx` など) を走査する。
    pub(super) fn pool_violated(&self, x: &[f64]) -> Vec<(f64, usize)> {
        let mut viol: Vec<(f64, usize)> = Vec::new();
        for k in 0..self.cut_pool.len() {
            let (s, e) = (self.pool_start[k] as usize, self.pool_start[k + 1] as usize);
            let mut act = 0.0;
            for t in s..e {
                act += self.pool_val[t] * x[self.pool_idx[t] as usize];
            }
            let r = self.cut_pool[k].1;
            let eff = (act - r) / self.cut_pool[k].2.max(1e-12);
            if eff > 1e-4 && act - r > 1e-6 * (1.0 + r.abs()) {
                viol.push((eff, k));
            }
        }
        viol
    }

    /// カットプールから、LP 解 `x` が違反するカットを効き目の大きい順に最大 `max_cuts` 本 LP に加える
    /// (違反しているので今の LP にはないカット)。LP の行が増えすぎていれば先に効いていないカットを外す。加えたら真。
    pub(super) fn pool_cut_round(&mut self, x: &[f64], max_cuts: usize) -> bool {
        if self.cut_pool.is_empty() {
            return false;
        }
        let mut viol = self.pool_violated(x);
        if viol.is_empty() {
            return false;
        }
        // 効き目の大きい順、同じなら番号の小さい順 (全順序なので、上位 `max_cuts` 本を選んでから並べても全体を
        // 安定に並べたのと同じ結果になる)
        let ord = |a: &(f64, usize), b: &(f64, usize)| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1));
        if max_cuts > 0 && viol.len() > max_cuts {
            viol.select_nth_unstable_by(max_cuts - 1, ord);
        }
        viol.truncate(max_cuts);
        viol.sort_unstable_by(ord);
        // `ENOMOTO_T_MIP_POOL_REL` = r (> 0): 効き目が最大のカットの r 倍に満たないカットは戻さない
        let rel = tunable!("ENOMOTO_T_MIP_POOL_REL", 0.0, f64);
        if rel > 0.0 {
            let top = viol[0].0;
            viol.retain(|&(e, _)| e >= rel * top);
        }
        let cap = self.p.m + (2 * self.cut_pool.len()).clamp(100, 2000);
        if self.lp.num_rows() + viol.len() > cap {
            self.remove_inactive_cuts();
        }
        let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = viol.iter().map(|&(_, k)| (self.cut_pool[k].0.clone(), f64::NEG_INFINITY, self.cut_pool[k].1)).collect();
        self.age_pool();
        for &(_, k) in &viol {
            self.pool_age[k] = 0;
        }
        self.add_cut_rows(&rows);
        true
    }

    /// 試作 `ENOMOTO_MIP_NODECNT` の `r`: ノードで経路集約を使うかを根の情報で決める (決めたら以後は同じ)。主のスレッドは、根のカットの
    /// ループ (再スタートの後を含む) で採った経路集約のカットが `ENOMOTO_T_MIP_NODECNT_ROOT_PATH` (既定 5) 本以上なら使う。診断に、
    /// 根で採ったカット ([`Solver::nc_root_fp`]) のうち最初に木で分離するノードの LP で効いている (双対値が 0 でない) 行と、そのうち経路集約の
    /// ものを数えて出す。決めた値は共有の待ち行列 ([`WorkPool::nc_rdec`]) に置き、
    /// 従のスレッド (根のカットのループを回さない) はそれを読む。まだ決まっていなければ使わない
    pub(super) fn nc_root_decide(&mut self) -> bool {
        use std::sync::atomic::Ordering;
        if self.nc_rdec == 0 {
            if let Some(d) = self.pool.as_ref().map(|pl| pl.nc_rdec.load(Ordering::Relaxed)).filter(|&d| d != 0) {
                self.nc_rdec = d;
            } else if self.nc_root_cnt[0] > 0 {
                let m = self.lp.num_rows();
                let ncut = self.cut_ids.len().min(m);
                let y = self.lp.row_duals();
                let (mut tight, mut tight_path, mut matched) = (0u64, 0u64, 0u64);
                for i in (m - ncut)..m {
                    let (_, up) = self.lp.row_bounds(i);
                    let Some(&is_path) = self.nc_root_fp.get(&cut_fingerprint(&self.lp.row(i), up)) else { continue };
                    matched += 1;
                    if y.get(i).is_some_and(|v| v.abs() > 1e-9) {
                        tight += 1;
                        tight_path += is_path as u64;
                    }
                }
                let share = tight_path as f64 / tight.max(1) as f64;
                // 判定は根で採った経路集約のカットの本数 (効いている行の割合は診断に出すだけ: 効く neos-1456979 で 0、効かない tr12-30 で 0.71 と分けられない)
                let on = self.nc_root_cnt[1] >= tunable!("ENOMOTO_T_MIP_NODECNT_ROOT_PATH", 5u64, u64);
                self.nc_rdec = if on { 1 } else { 2 };
                if let Some(pl) = &self.pool {
                    pl.nc_rdec.store(self.nc_rdec, Ordering::Relaxed);
                }
                if env_str!("ENOMOTO_MIP_LOG").is_some() {
                    eprintln!(
                        "MIP: NODECNT root gate: cut rows {ncut}, matched root cuts {matched}, tight {tight}, tight path {tight_path} (share {share:.3}); root chosen {} (path {}); node path aggregation {}",
                        self.nc_root_cnt[0],
                        self.nc_root_cnt[1],
                        if on { "on" } else { "off" }
                    );
                }
            }
        }
        self.nc_rdec == 1
    }

    /// ノードでの分離 (1 ラウンド)。カットを加えたら LP を解き直し、真を返す。LP の行数の上限を超えたら何もしない。
    pub(super) fn node_cut_round(&mut self, x: &[f64]) -> bool {
        let p = self.p;
        let max_rows = p.m + (2 * p.m).max(500);
        if self.lp.num_rows() >= max_rows {
            return false;
        }
        // ノードでは軽い分離 (経路集約の始点・tableau 行を減らす) で、追加するカットも少なくする
        // `ENOMOTO_MIP_NODE_FACE_SEP`: ノードでも最適面の頂点で分離する ([`Self::separate_face`])
        // 40 問: 使わない 18 問 27.20、使う 18 問 27.36 (neos5 時間切れ -> 59.7 秒、neos-860300 54.2 秒 -> 時間切れ。木での分離の
        // 時間はもともと小さく、差はほぼない)。既定では使わない
        let sl_on = self.sl.stat && !self.params.submip;
        if sl_on {
            self.sl.begin_call(true);
        }
        let mut cands = if env_str!("ENOMOTO_MIP_NODE_FACE_SEP").is_some() { self.separate_face(x, true) } else { self.separate(x, true) };
        self.node_cut_maxlen(&mut cands);
        let room = max_rows - self.lp.num_rows();
        let max_node = room.min(tunable!("ENOMOTO_T_MIP_NODE_MAX_CUTS", 20usize, usize));
        if sl_on {
            self.sl_after_separate(&mut [&mut cands], max_node);
        }
        if cands.is_empty() {
            if sl_on {
                self.sl.end_call_without_add();
            }
            return false;
        }
        let chosen = select_cuts(cands, max_node, p);
        // 試作 `ENOMOTO_MIP_NODECNT` の `p`・`h`: ノードで採った経路集約のカットを数え、`g` なら採られないときにやめる
        if self.nc_path_forced && self.tree_sep_fc {
            let np = chosen.iter().filter(|c| matches!(c.src & 0xF, sepprof::SRC_PATH | sepprof::SRC_PATHH)).count() as u64;
            let st = &mut self.nc_path_stats;
            st[0] += 1;
            st[1] += (np > 0) as u64;
            st[2] += np;
            st[3] += chosen.len() as u64;
            if super::solver::nodecnt('g')
                && st[0] >= tunable!("ENOMOTO_T_MIP_NODECNT_GATE_N", 30u64, u64)
                && (st[1] as f64) < tunable!("ENOMOTO_T_MIP_NODECNT_GATE", 0.3, f64) * st[0] as f64
            {
                self.tree_sep_fc = false;
            }
        }
        if chosen.is_empty() {
            if sl_on {
                self.sl.end_call_without_add();
            }
            return false;
        }
        if sl_on {
            self.sl_prepare(&chosen, x);
        }
        let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = chosen.into_iter().map(|c| (c.coefs, f64::NEG_INFINITY, c.rhs)).collect();
        // 加えるカットの非零の数の平均を効き目で続けるノードの分離の密度の判定 ([`Solver::nca_dense`]) に入れる
        let d = rows.iter().map(|r| r.0.len()).sum::<usize>() as f64 / rows.len() as f64;
        self.nca_cut_nnz = Some(self.nca_cut_nnz.map_or(d, |e| 0.9 * e + 0.1 * d));
        // カットはプールにも入れ (他のノードで違反すれば戻る)、このノードで効いていないカットは外してから加える
        // (HiGHS の aging と同じく LP を小さく保つ。外したカットもプールにあれば戻せる)
        if env_str!("ENOMOTO_MIP_NODE_CUTS_KEEP").is_none() {
            for (c, _, r) in &rows {
                self.add_to_pool(c, *r);
            }
            self.remove_aged_cuts();
        }
        self.add_cut_rows(&rows);
        if sl_on {
            self.sl.bind(self.next_cut_id);
        }
        true
    }

    /// HiGHS 流のカット管理 (`Solver::highs_cutmgmt`) でのノードでの分離 1 ラウンド: プールで違反しているカットと、
    /// 新しく分離したカット (軽い分離) を合わせて選び (最大 `ENOMOTO_T_MIP_HCM_NODE_MAX_CUTS` 本)、年齢の上限を超えたカットを
    /// 外してから加える。加えたら真 (呼び出し側が LP を解き直す)。LP の行数の上限を超えていれば何もしない。
    pub(super) fn hcm_node_sep_round(&mut self, x: &[f64], fresh: bool) -> bool {
        let p = self.p;
        let max_rows = p.m + (2 * p.m).max(500);
        if self.lp.num_rows() >= max_rows {
            return false;
        }
        // `fresh` でなければプールのカットだけ (安価。年齢で外れた根のカットを戻すのが主な役目)
        crate::simplex::slope_intercept_dual::xprof("hcm_pre");
        let sl_on = self.sl.stat && !self.params.submip;
        if sl_on {
            self.sl.begin_call(true);
        }
        let mut cands = if fresh { self.separate(x, true) } else { Vec::new() };
        self.node_cut_maxlen(&mut cands);
        if sl_on {
            let b = tunable!("ENOMOTO_T_MIP_HCM_NODE_MAX_CUTS", 100usize, usize);
            self.sl_after_separate(&mut [&mut cands], b);
        }
        crate::simplex::slope_intercept_dual::xprof("hcm_sep");
        for (eff, k) in self.pool_violated(x) {
            let (c, r, _) = &self.cut_pool[k];
            cands.push(Candidate { coefs: c.clone(), rhs: *r, efficacy: eff, forced: false, src: sepprof::SRC_POOL });
        }
        crate::simplex::slope_intercept_dual::xprof("hcm_pool");
        if cands.is_empty() {
            return false;
        }
        let room = max_rows - self.lp.num_rows();
        // 1 ラウンドの本数は根と同じ程度まで許す (HiGHS はプールの違反カットを平行度だけで絞る)。年齢で外れたカットが多い
        // ノードでは、20 本ずつでは根で効いていたカットが戻りきらず、ノードの LP が根より弱くなって木の下界が動かなかった
        let chosen = select_cuts(cands, room.min(tunable!("ENOMOTO_T_MIP_HCM_NODE_MAX_CUTS", 100usize, usize)), p);
        if chosen.is_empty() {
            if sl_on {
                self.sl.end_call_without_add();
            }
            return false;
        }
        if sl_on {
            self.sl_prepare(&chosen, x);
        }
        let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = chosen.into_iter().map(|c| (c.coefs, f64::NEG_INFINITY, c.rhs)).collect();
        for (c, _, r) in &rows {
            self.add_to_pool(c, *r);
        }
        self.remove_aged_cuts();
        self.add_cut_rows(&rows);
        if sl_on {
            self.sl.bind(self.next_cut_id);
        }
        true
    }

    /// `ENOMOTO_MIP_NODE_CUT_MAXLEN` (試作、既定では使わない): ノードの分離で作ったカットのうち、非零の数が max(50, 列 / 40) を
    /// 超えるものを捨てる (CglGomory の `dynamicLimitInTree_`。プールから戻すカットは捨てない)。加えるカットの非零の平均
    /// (ノードの分離を続けるかの密度の判定 [`Solver::nca_dense`] が見る `nca_cut_nnz`) は残ったカットで数える
    fn node_cut_maxlen(&self, cands: &mut Vec<Candidate>) {
        if env_str!("ENOMOTO_MIP_NODE_CUT_MAXLEN").is_none() {
            return;
        }
        let maxlen = tunable!("ENOMOTO_T_MIP_NODE_CUT_MAXLEN_MIN", 50usize, usize).max(self.p.n / tunable!("ENOMOTO_T_MIP_NODE_CUT_MAXLEN_DIV", 40usize, usize).max(1));
        cands.retain(|c| c.coefs.len() <= maxlen);
    }

    /// LP にカットの行を加える (年齢 0)。
    pub(super) fn add_cut_rows(&mut self, rows: &[(Vec<(usize, f64)>, f64, f64)]) {
        self.debug_check_cut_rows(rows);
        self.sync_cut_age();
        self.lp.add_rows(rows);
        self.cut_age.extend(std::iter::repeat_n(0, rows.len()));
        for _ in 0..rows.len() {
            self.cut_ids.push(self.next_cut_id);
            self.next_cut_id += 1;
        }
        self.row_log.push(super::solver::RowEdit::Add(rows.len()));
    }

    /// 診断用 (デバッグ解): LP に加えるカットの行がデバッグ解を切っていれば表示する ([`Self::add_cut_rows`])。
    fn debug_check_cut_rows(&self, rows: &[(Vec<(usize, f64)>, f64, f64)]) {
        // 診断用 (デバッグ解): どのスレッドでも、LP に加えるカットの行がデバッグ解を切っていれば表示する (打ち切り値より良いときだけ)
        if self.dbg_on {
            super::solver::DEBUG_SOL.with(|d| {
                if let Some(x) = d.borrow().as_ref() {
                    if self.p.objective(x) < self.prune_limit() - 1e-6 {
                        for (k, (c, lo, up)) in rows.iter().enumerate() {
                            let act: f64 = c.iter().map(|&(j, a)| a * x[j]).sum();
                            if act < lo - 1e-6 * (1.0 + lo.abs()) || act > up + 1e-6 * (1.0 + up.abs()) {
                                let terms: Vec<String> = c.iter().take(8).map(|&(j, a)| format!("{a:+.4}*x{j}[{:.4}]", x[j])).collect();
                                eprintln!("MIP_DEBUG_SOL: node {} thread {}: cut row {k} of {} added to the LP (as row {}) is violated by the debug solution: {lo} <= {act:.10e} <= {up}, len {}: {}{}", self.nodes, self.pool_tid, rows.len(), self.lp.num_rows() + k, c.len(), terms.join(" "), if c.len() > 8 { " ..." } else { "" });
                            }
                        }
                    }
                }
            });
        }
    }

    /// LP の行 `remove` を消してからカットの行 `rows` を加える ([`Self::delete_lp_rows`] の後に [`Self::add_cut_rows`] と同じ。
    /// LP の標準形の作り直しは 1 回、[`MipLp::delete_and_add_rows`]。`ENOMOTO_MIP_CUT_KEEP_LU`)。
    fn replace_cut_rows(&mut self, remove: &[bool], rows: &[(Vec<(usize, f64)>, f64, f64)]) {
        self.debug_check_cut_rows(rows);
        self.sync_cut_age();
        let m0 = self.p.m;
        let mut k = m0;
        self.cut_age.retain(|_| {
            let keep = !remove[k];
            k += 1;
            keep
        });
        let mut k = m0;
        self.cut_ids.retain(|_| {
            let keep = !remove[k];
            k += 1;
            keep
        });
        self.row_log.push(super::solver::RowEdit::delete(remove));
        self.lp.delete_and_add_rows(remove, rows);
        self.cut_age.extend(std::iter::repeat_n(0, rows.len()));
        for _ in 0..rows.len() {
            self.cut_ids.push(self.next_cut_id);
            self.next_cut_id += 1;
        }
        self.row_log.push(super::solver::RowEdit::Add(rows.len()));
    }

    /// LP の行を消す (カットの年齢も合わせて消す)。
    pub(super) fn delete_lp_rows(&mut self, remove: &[bool]) {
        self.sync_cut_age();
        let m0 = self.p.m;
        let mut k = m0;
        self.cut_age.retain(|_| {
            let keep = !remove[k];
            k += 1;
            keep
        });
        let mut k = m0;
        let track = !self.ts_cut_ids.is_empty();
        let mut ntree = 0u64;
        let mut nother = 0u64;
        self.cut_ids.retain(|id| {
            let keep = !remove[k];
            k += 1;
            if !keep && track {
                if self.ts_cut_ids.contains(id) { ntree += 1 } else { nother += 1 }
            }
            keep
        });
        self.tsd[1] += ntree;
        self.tsd[2] += nother;
        self.lp.delete_rows(remove);
        self.row_log.push(super::solver::RowEdit::delete(remove));
    }

    /// カットの年齢の長さを LP のカットの行の数に合わせる (足りなければ 0 で埋める)。
    pub(super) fn sync_cut_age(&mut self) {
        let want = self.lp.num_rows().saturating_sub(self.p.m);
        self.cut_age.resize(want, 0);
        if self.cut_ids.len() > want {
            self.cut_ids.truncate(want);
        }
        while self.cut_ids.len() < want {
            self.cut_ids.push(self.next_cut_id);
            self.next_cut_id += 1;
        }
    }

    /// 今の LP の基底で効いている (論理変数が非基底の) カットの番号 (待ち行列に入れるノードに持たせ、その間は年齢で
    /// 外さない。取り出したときに保存した基底が必ず合う)。既定では使わない (`ENOMOTO_MIP_PROTECT_QUEUE_CUTS` で使う)。
    /// 以前は既定で使っていた (40 問: 19 問 27.04 -> 19 問 26.89、保護して年齢の上限を一律 30 にすると 17 問 27.55) が、待ち行列の
    /// ノードが数千あると、それぞれが入れた時点で効いていたカットの和でほぼ全てのカットが外せなくなり LP が膨らむ (pk1 は元 45 行が
    /// 約 545 行になり、ノードの LP が 2 倍遅い)。その計測は役の復帰で `idle` が下に回り待ち行列がほぼ空になる不具合のある状態で
    /// 取ったもの (analysis/pk1_variance_20261017.md)。HiGHS も保護せず、基底が行と合わなければ今の基底から解く
    pub(super) fn basis_protected_cuts(&mut self) -> Option<std::rc::Rc<Vec<u64>>> {
        // HiGHS 流のカット管理では保護しない (HiGHS と同じく、取り出したノードの基底が LP の行と合わなければ今の基底から解く。
        // 保護すると待ち行列のノードが数千あるときほぼ全てのカットが外せず、neos-911970 では LP が行数の上限 607 に張り付いた)
        if env_str!("ENOMOTO_MIP_PROTECT_QUEUE_CUTS").is_none() || env_str!("ENOMOTO_MIP_NO_PROTECT_QUEUE_CUTS").is_some() || self.params.submip || (self.highs_cutmgmt() && env_str!("ENOMOTO_MIP_HCM_PROTECT").is_none()) {
            return None;
        }
        self.sync_cut_age();
        let m0 = self.p.m;
        let mr = self.lp.num_rows();
        if mr <= m0 {
            return None;
        }
        let b = self.lp.basis_ref();
        let v: Vec<u64> = (m0..mr).filter(|&i| b.row[i] != VarStatus::Basic).map(|i| self.cut_ids[i - m0]).collect();
        if v.is_empty() { None } else { Some(std::rc::Rc::new(v)) }
    }

    /// ノードの LP の後に呼ぶ (HiGHS の LP の aging): 効いていない (論理変数が基底で行に余裕がある) カットの年齢を
    /// 1 増やし、効いているカットは 0 に戻す。
    pub(super) fn age_cuts(&mut self) {
        self.sync_cut_age();
        if self.cut_age.is_empty() {
            return;
        }
        let m0 = self.p.m;
        let b = self.lp.basis_ref();
        let act = self.lp.row_activities_ref();
        // 相対的な基準 (既定では使わない):
        // `ENOMOTO_T_MIP_CUT_SLACK_REL` = s (> 0): 余裕のあるカットのうち、正規化した余裕 (余裕 / 係数の 2 ノルム) が
        //   カットの中で最大のものの s 倍に満たないもの (ほぼ効いている) は年齢を増やさない
        // `ENOMOTO_T_MIP_CUT_DUAL_REL` = d (> 0): 効いている (論理変数が非基底の) カットのうち、|双対値| がカットの中で
        //   最大のものの d 倍に満たないもの (退化して効いているだけ) は年齢を増やす
        // 40 問 (19 問は同じ): 既定 25.28 に対し s=0.1 は 24.96 だったが、もう一度並べると 25.37 (s=0.05 25.31、
        // s=0.3 25.28)。d=0.01 は 25.35、プールの r=0.1 は 25.26。差はばらつきの範囲なので、どれも既定では使わない
        let srel = tunable!("ENOMOTO_T_MIP_CUT_SLACK_REL", 0.0, f64);
        let drel = tunable!("ENOMOTO_T_MIP_CUT_DUAL_REL", 0.0, f64);
        let ncut = self.cut_age.len();
        // 正規化した余裕 (`srel > 0` のときだけ作る)
        let mut nslack = Vec::new();
        let mut max_slack = 0.0f64;
        if srel > 0.0 {
            nslack = vec![0.0f64; ncut];
            for k in 0..ncut {
                let i = m0 + k;
                let (_, up) = self.lp.row_bounds(i);
                if b.row[i] == VarStatus::Basic && act[i] < up - 1e-6 * (1.0 + up.abs()) {
                    let id = self.cut_ids[k];
                    let norm = match self.cut_norm.get(&id) {
                        Some(&v) => v,
                        None => {
                            let v = self.lp.row_ref(i).iter().map(|&(_, a)| a * a).sum::<f64>().sqrt().max(1e-12);
                            self.cut_norm.insert(id, v);
                            v
                        }
                    };
                    nslack[k] = (up - act[i]) / norm;
                    max_slack = max_slack.max(nslack[k]);
                }
            }
            if self.cut_norm.len() > 4 * ncut + 1000 {
                let live: std::collections::HashSet<u64> = self.cut_ids.iter().copied().collect();
                self.cut_norm.retain(|id, _| live.contains(id));
            }
        }
        let mut dual = Vec::new();
        let mut max_dual = 0.0f64;
        if drel > 0.0 {
            dual = self.lp.row_duals();
            for k in 0..ncut {
                if b.row[m0 + k] != VarStatus::Basic {
                    max_dual = max_dual.max(dual[m0 + k].abs());
                }
            }
        }
        // `ENOMOTO_MIP_CUT_AGE_HIGHS`: HiGHS の `HighsLpRelaxation::performAging` と同じく、論理変数が基底のカットは余裕が 0
        // (退化して効いている形) でも年齢を増やし、非基底でも双対値が 0 なら増やす。0 に戻すのは双対値が 0 でないものだけ。
        // 既定の数え方では退化した基底の行の年齢が 0 に戻り続け、上限を下げても LP のカットの行が減らない (mc11 で上限 50 でも 3770 行)
        if env_str!("ENOMOTO_MIP_CUT_AGE_HIGHS").is_some() {
            let y = self.lp.row_duals();
            for (k, age) in self.cut_age.iter_mut().enumerate() {
                let i = m0 + k;
                if b.row[i] == VarStatus::Basic || y[i].abs() <= 1e-9 {
                    *age = age.saturating_add(1);
                } else {
                    *age = 0;
                }
            }
        } else {
        // `ENOMOTO_MIP_TREESEP2` の a: 木の分離で加えたカットは HiGHS の数え方 (退化して基底にある行・双対値 0 の行も年齢を増やす)
        let ts2a = self.ts2_has('a') && self.ts2_id0 != u64::MAX;
        let y2 = if ts2a { self.lp.row_duals() } else { Vec::new() };
        for (k, age) in self.cut_age.iter_mut().enumerate() {
            let i = m0 + k;
            let (_, up) = self.lp.row_bounds(i);
            if ts2a && self.cut_ids[k] >= self.ts2_id0 {
                // HiGHS の `performAging` と同じく、双対値 0 の非基底の行は年齢をそのままにする (外すのは基底の行だけ。
                // 非基底の行を外すと基底が崩れ、大きな LP (uccase12・unitcal_7) でノードの LP の反復が 6〜7 倍になった)
                if b.row[i] == VarStatus::Basic {
                    *age = age.saturating_add(1);
                } else if y2[i].abs() > 1e-9 {
                    *age = 0;
                }
                continue;
            }
            if b.row[i] == VarStatus::Basic && act[i] < up - 1e-6 * (1.0 + up.abs()) {
                if srel > 0.0 && nslack[k] < srel * max_slack {
                    continue;
                }
                *age = age.saturating_add(1);
            } else if drel > 0.0 && b.row[i] != VarStatus::Basic && dual[i].abs() < drel * max_dual {
                *age = age.saturating_add(1);
            } else {
                *age = 0;
            }
        }
        }
        if !self.merged_cuts.is_empty() {
            for k in 0..ncut {
                if self.merged_cuts.contains(&self.cut_ids[k]) {
                    self.cut_age[k] = 0;
                }
            }
        }
    }

    /// 年齢が上限 (`ENOMOTO_T_MIP_CUT_AGE_LIMIT`、既定 300。HiGHS の `mip_lp_age_limit` は 10 だが、10 では binkar10_1 で効くカットまで外れた) を超えたカットを LP から外す
    /// (プールにあるカットは、違反すればまた戻る)。外した数を返す。
    /// 年齢は今のノードで効いていない回数なので、待ち行列のノードで効いているカットまで外れうる。そのノードを取り出すと
    /// 保存した基底が合わず LP がほぼ解き直しになる (binkar10_1: 上限 30 で 1 ノード平均 272 反復、外さなければ 37)。
    /// 40 問: 上限 30 は 17 問 27.21、100 は 19 問 27.05、300 は 19 問 26.64 (外さないと neos5 で LP が重くなる)
    ///
    /// `ENOMOTO_T_MIP_CUT_AGE_PER_ROW` = f (> 0): 上限を f × (元の行の数) にして [`ENOMOTO_T_MIP_CUT_AGE_MIN`, 上の上限] に収める
    /// (行の少ない問題ではカットが LP を何倍にもする: markshare_4_0 は 4 行で、上限 300 だと遅くなった)。
    /// `ENOMOTO_T_MIP_CUT_ROWS_CAP` = r (> 0): カットの行が max(r × 元の行の数, 200) を超えたら、年齢の高い順に外して収める。
    pub(super) fn remove_aged_cuts(&mut self) -> usize {
        let base = tunable!("ENOMOTO_T_MIP_CUT_AGE_LIMIT", 300u32, u32);
        // 既定は f = 0.5 (40 問: 一律 300 の 19 問 26.72 に対し 19 問 26.41。markshare_4_0 (4 行) 25 -> 14 秒、
        // neos5 (63 行) は 42 -> 48 秒。カットの行の総数で抑える形 (`ENOMOTO_T_MIP_CUT_ROWS_CAP=2`) は 26.61)
        let f = tunable!("ENOMOTO_T_MIP_CUT_AGE_PER_ROW", 0.5, f64);
        // HiGHS 流のカット管理では上限を `ENOMOTO_T_MIP_HCM_AGE_LIMIT` (HiGHS の mip_lp_age_limit は 10) にする。
        // 30 では neos-911970 の木の下界が根から動かず (潜りの間に根で効いていたカットが外れ、ノードの LP が根より弱くなる)、
        // 100 で下界 54.70 / 上界 54.76 (最適値)。mik-250 は 30 でも 100 でも 60 秒で解けない (既定は 35 秒)
        let limit = if self.highs_cutmgmt() {
            tunable!("ENOMOTO_T_MIP_HCM_AGE_LIMIT", 100u32, u32)
        } else if f > 0.0 {
            let lo = tunable!("ENOMOTO_T_MIP_CUT_AGE_MIN", 30u32, u32);
            ((f * self.p.m as f64) as u32).clamp(lo.min(base), base)
        } else {
            base
        };
        let mut removed = self.remove_cuts_older_than(limit);
        // `ENOMOTO_T_MIP_TREE_SEP_CUT_AGE` = a (> 0): `ENOMOTO_MIP_TREE_SEP` のラウンドで加えたカットは年齢 a で外す
        // (根のカットは上の上限のまま)。ノードで加えたカットの行が残ってノードの LP が重くなるのを抑える
        let ts_age = if self.ts2_has('a') { tunable!("ENOMOTO_T_MIP_TREESEP2_AGE", 10u32, u32) } else { tunable!("ENOMOTO_T_MIP_TREE_SEP_CUT_AGE", 0u32, u32) };
        let ts2a = self.ts2_has('a');
        if ts_age > 0 && (!self.ts_cut_ids.is_empty() || ts2a) {
            self.sync_cut_age();
            let m0 = self.p.m;
            let mut remove = vec![false; self.lp.num_rows()];
            let mut cnt = 0;
            for (k, &age) in self.cut_age.iter().enumerate() {
                let tree = if ts2a { self.cut_ids[k] >= self.ts2_id0 && self.lp.basis_ref().row[m0 + k] == VarStatus::Basic } else { self.ts_cut_ids.contains(&self.cut_ids[k]) };
                if age > ts_age && tree && !self.queue.is_protected(self.cut_ids[k]) {
                    remove[m0 + k] = true;
                    cnt += 1;
                }
            }
            if cnt > 0 {
                self.delete_lp_rows(&remove);
                removed += cnt;
                self.tsd[5] += cnt as u64;
            }
            if self.ts_cut_ids.len() > 4 * self.cut_ids.len() + 1000 {
                let live: std::collections::HashSet<u64> = self.cut_ids.iter().copied().collect();
                self.ts_cut_ids.retain(|id| live.contains(id));
            }
        }
        // `ENOMOTO_MIP_TREESEP2` の d: 木の分離で加えたカットの行が上限を超えたら、年齢の高い (今効いていない) ものから外す
        if self.ts2_has('d') && self.ts2_id0 != u64::MAX {
            self.sync_cut_age();
            let cap = self.ts2_rows_cap();
            let mut tree: Vec<(u32, usize)> = (0..self.cut_ids.len()).filter(|&k| self.cut_ids[k] >= self.ts2_id0 && !self.queue.is_protected(self.cut_ids[k])).map(|k| (self.cut_age[k], k)).collect();
            // 外すのは基底の (効いていない) 行だけ (上の a と同じ)
            let basic_of: std::collections::HashMap<usize, bool> = { let b = self.lp.basis_ref(); tree.iter().map(|&(_, k)| (k, b.row[self.p.m + k] == VarStatus::Basic)).collect() };
            if tree.len() > cap {
                tree.sort_by(|a, b| b.cmp(a));
                let m0 = self.p.m;
                let mut remove = vec![false; self.lp.num_rows()];
                let mut cnt = 0;
                for &(_, k) in tree.iter().filter(|&&(a, k)| a > 0 && basic_of[&k]).take(tree.len() - cap) {
                    remove[m0 + k] = true;
                    cnt += 1;
                }
                if cnt > 0 {
                    self.delete_lp_rows(&remove);
                    removed += cnt;
                    self.tsd[5] += cnt as u64;
                }
            }
        }
        let r = tunable!("ENOMOTO_T_MIP_CUT_ROWS_CAP", 0.0, f64);
        if r > 0.0 {
            self.sync_cut_age();
            let cap = ((r * self.p.m as f64) as usize).max(200);
            let ncut = self.cut_age.len();
            if ncut > cap {
                // 年齢の高い順に (ncut - cap) 本外す (年齢 0 = 今効いているものは外さない)
                let mut order: Vec<(u32, usize)> = self.cut_age.iter().enumerate().filter(|&(_, &a)| a > 0).map(|(k, &a)| (a, k)).collect();
                order.sort_by(|a, b| b.cmp(a));
                let m0 = self.p.m;
                let mut remove = vec![false; self.lp.num_rows()];
                let mut cnt = 0;
                for &(_, k) in order.iter().filter(|&&(_, k)| !self.queue.is_protected(self.cut_ids[k])).take(ncut - cap) {
                    remove[m0 + k] = true;
                    cnt += 1;
                }
                if cnt > 0 {
                    self.delete_lp_rows(&remove);
                    removed += cnt;
                }
            }
        }
        removed
    }

    /// `ENOMOTO_MIP_TREE_SEP` のラウンドで加えたカット (番号が `id0` 以上) を覚える (`ENOMOTO_T_MIP_TREE_SEP_CUT_AGE` > 0、
    /// `ENOMOTO_MIP_TREESEP2`・`ENOMOTO_MIP_TREESEP_DIAG` のときだけ)。以前は加える前の行の数 `r0` から後ろを覚えていたが、
    /// `node_cut_round` は加える前に年齢で行を外すので、外した分だけ前の (根の) カットまで木のカットとして覚えていた
    pub(super) fn mark_tree_sep_cuts(&mut self, id0: u64) {
        if tunable!("ENOMOTO_T_MIP_TREE_SEP_CUT_AGE", 0u32, u32) == 0 && env_str!("ENOMOTO_MIP_TREESEP_DIAG").is_none() && env_str!("ENOMOTO_MIP_TREESEP2").is_none() {
            return;
        }
        self.sync_cut_age();
        for &id in self.cut_ids.iter().rev() {
            if id < id0 {
                break;
            }
            self.ts_cut_ids.insert(id);
        }
    }

    /// 年齢が `limit` を超えたカットを LP から外す。外した数を返す。
    pub(super) fn remove_cuts_older_than(&mut self, limit: u32) -> usize {
        self.sync_cut_age();
        let m0 = self.p.m;
        let old = |k: usize, age: u32| age > limit && !self.queue.is_protected(self.cut_ids[k]);
        // 外すカットがなければ印の配列を作らない
        if !self.cut_age.iter().enumerate().any(|(k, &age)| old(k, age)) {
            return 0;
        }
        let mut remove = vec![false; self.lp.num_rows()];
        let mut cnt = 0;
        for (k, &age) in self.cut_age.iter().enumerate() {
            if old(k, age) {
                remove[m0 + k] = true;
                cnt += 1;
            }
        }
        self.delete_lp_rows(&remove);
        cnt
    }

    /// 今の LP で効いている (双対値が 0 でない) カットを、|双対値| を重みに足して 1 本の行にまとめて加える
    /// (各カットは `a_i x <= b_i` なので、非負の重みの和 `sum w_i a_i x <= sum w_i b_i` も妥当)。今の LP の双対解は
    /// まとめた行に重み 1 を置いても実行可能なので、元のカットが年齢で外れても今の LP の目的値は保たれる
    /// (子のノードでは個々のカットより弱い)。まとめた行は年齢で外さない。加えたら真。
    pub(super) fn merge_active_cuts(&mut self) -> bool {
        let m0 = self.p.m;
        let n = self.p.n;
        let mr = self.lp.num_rows();
        if mr <= m0 {
            return false;
        }
        let y = self.lp.row_duals();
        let mut wmax = 0.0f64;
        for i in m0..mr {
            let (l, u) = self.lp.row_bounds(i);
            if l == f64::NEG_INFINITY && u.is_finite() {
                wmax = wmax.max(y[i].abs());
            }
        }
        if wmax <= 1e-12 {
            return false;
        }
        let mut dense = vec![0.0f64; n];
        let mut rhs = 0.0f64;
        let mut used = 0usize;
        for i in m0..mr {
            let (l, u) = self.lp.row_bounds(i);
            if l != f64::NEG_INFINITY || !u.is_finite() {
                continue;
            }
            let w = y[i].abs() / wmax;
            if w <= 1e-9 {
                continue;
            }
            used += 1;
            for &(j, a) in self.lp.row_ref(i) {
                dense[j] += w * a;
            }
            rhs += w * u;
        }
        if used < 2 {
            return false;
        }
        let cmax = dense.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let mut coefs: Vec<(usize, f64)> = Vec::new();
        for (j, &c) in dense.iter().enumerate() {
            if c == 0.0 {
                continue;
            }
            if c.abs() <= 1e-9 * cmax {
                // 小さい係数は大域的な境界で右辺に移す (c x_j >= min(c lo, c up))
                let m = if c > 0.0 { c * self.dom.global_lo[j] } else { c * self.dom.global_up[j] };
                if m.is_finite() {
                    rhs -= m;
                    continue;
                }
            }
            coefs.push((j, c));
        }
        let max_nnz = tunable!("ENOMOTO_T_MIP_MERGE_MAX_NNZ_FRAC", 1.0, f64);
        if coefs.is_empty() || coefs.len() as f64 > max_nnz * n as f64 {
            return false;
        }
        let nnz = coefs.len();
        self.add_cut_rows(&[(coefs, f64::NEG_INFINITY, rhs)]);
        if let Some(&id) = self.cut_ids.last() {
            self.merged_cuts.insert(id);
        }
        let st = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
        if self.params.verbose {
            eprintln!("MIP: merged {used} active cuts into one row ({nnz} nonzeros), LP {st:?} obj {:.10e}", self.lp.objective() + self.p.offset);
        }
        true
    }

    /// LP の行のうち、元の行より後ろ (カット) で論理変数が基底にあるものを外す。
    pub(super) fn remove_inactive_cuts(&mut self) {
        if let Some(remove) = self.inactive_cut_mask() {
            self.delete_lp_rows(&remove);
        }
    }

    /// [`Self::remove_inactive_cuts`] で外す行の印 (外す行がなければ `None`)。
    fn inactive_cut_mask(&mut self) -> Option<Vec<bool>> {
        let m0 = self.p.m;
        let mr = self.lp.num_rows();
        if mr <= m0 {
            return None;
        }
        self.sync_cut_age();
        let b = self.lp.basis_ref();
        let act = self.lp.row_activities_ref();
        let mut remove = vec![false; mr];
        let mut any = false;
        for i in m0..mr {
            if b.row[i] == VarStatus::Basic {
                let (_, up) = self.lp.row_bounds(i);
                if act[i] < up - 1e-6 * (1.0 + up.abs()) && !self.merged_cuts.contains(&self.cut_ids[i - m0]) {
                    remove[i] = true;
                    any = true;
                }
            }
        }
        any.then_some(remove)
    }

    /// HiGHS の `HighsPathSeparator` (path mixing cut を除く) の移植。
    /// - 行の型: 等式 / LP で効いている側 (`<=` か `>=`) / 効いていない (使わない)
    /// - 連続変数 (境界から離れているもの) ごとに、係数の符号で「入る」行と「出る」行に分ける (`<=` の行で負なら入る、
    ///   `>=` の行で正なら入る、等式は両方)。集約に残った連続変数を消すときは、係数が負なら入る行、正なら出る行から選ぶ
    /// - 連続変数がそれ 1 つだけの等式は代入用にとっておき、始点・相手には使わない
    /// - 効いている行それぞれを始点に、最大 6 段。各段で集約行とその符号を反転したものから CMIR / flow cover を作り、
    ///   カットが出たら伸ばさない
    fn path_aggregation_highs<F>(&mut self, vars: &CutVars, lp_rows: &[Vec<(usize, f64)>], cands: &mut Vec<Candidate>, push: &mut F)
    where
        F: FnMut(Option<RawCut>, &mut Vec<Candidate>, &Solver<L>),
    {
        const MAX_PATH_LEN: usize = 6;
        let n = self.p.n;
        let m = lp_rows.len();
        let feastol = PATHH_FEASTOL;
        let bound_dist = |j: usize| -> f64 { pathh_bound_dist(vars, j) };
        let (rtype, cont, col_rows, subst) = pathh_row_types(vars, lp_rows, n);
        // 連続変数ごとの入る行・出る行
        let mut in_arcs: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        let mut out_arcs: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        for j in 0..n {
            if !cont[j] || subst[j].is_some() {
                continue;
            }
            for &(i, a) in &col_rows[j] {
                match rtype[i] {
                    0 => {}
                    1 => {
                        if a < 0.0 { in_arcs[j].push((i, a)) } else { out_arcs[j].push((i, a)) }
                    }
                    -1 => {
                        if a > 0.0 { in_arcs[j].push((i, a)) } else { out_arcs[j].push((i, a)) }
                    }
                    _ => {
                        in_arcs[j].push((i, a));
                        out_arcs[j].push((i, a));
                    }
                }
            }
        }
        let mut agg = vec![0.0f64; n + m];
        let mut touched: Vec<usize> = Vec::new();
        let mut in_t = vec![false; n + m];
        let add_row = |i: usize, w: f64, agg: &mut Vec<f64>, touched: &mut Vec<usize>, in_t: &mut Vec<bool>| {
            for &(j, a) in &lp_rows[i] {
                if !in_t[j] {
                    in_t[j] = true;
                    touched.push(j);
                }
                agg[j] += w * a;
            }
            let k = n + i;
            if !in_t[k] {
                in_t[k] = true;
                touched.push(k);
            }
            agg[k] -= w;
        };
        // 診断 `ENOMOTO_MIP_GAP4_PATHDIAG` (既定では出さない): [始点, 試した向き, 生成の試み, 成功, 消す連続変数なしで止まる, 相手の行なしで止まる, 段の上限]
        let pdiag = env_str!("ENOMOTO_MIP_GAP4_PATHDIAG").is_some();
        let mut pd = [0u64; 7];
        let pd_out = |pd: &[u64; 7], m: usize| {
            eprintln!("GAP4 PATHDIAG: rows {m} starts {} orientations {} attempts {} success {} stop_nocol {} stop_norow {} stop_len {}", pd[0], pd[1], pd[2], pd[3], pd[4], pd[5], pd[6]);
        };
        for start in 0..m {
            if start % 64 == 0 && self.time_up() {
                if pdiag {
                    pd_out(&pd, m);
                }
                return;
            }
            let scales: [f64; 2] = match rtype[start] {
                0 => continue,
                1 | 2 => [1.0, -1.0],
                _ => [-1.0, 1.0],
            };
            pd[0] += 1;
            for &scale in &scales {
                pd[1] += 1;
                for &k in &touched {
                    agg[k] = 0.0;
                    in_t[k] = false;
                }
                touched.clear();
                add_row(start, scale, &mut agg, &mut touched, &mut in_t);
                let mut path: Vec<usize> = vec![start];
                let mut try_neg = false;
                let mut success = false;
                let mut guard = 0;
                while path.len() < MAX_PATH_LEN {
                    guard += 1;
                    if guard > 50 {
                        break;
                    }
                    let in_path = |r: usize, path: &Vec<usize>| path.contains(&r);
                    // 代入できる連続変数があれば先に代入して数え直す
                    let mut substituted = false;
                    let snapshot: Vec<(usize, f64)> = touched.iter().filter(|&&k| k < n && agg[k].abs() > 1e-9).map(|&k| (k, agg[k])).collect();
                    for &(j, v) in &snapshot {
                        if !cont[j] {
                            continue;
                        }
                        if let Some((r, a)) = subst[j] {
                            add_row(r, -v / a, &mut agg, &mut touched, &mut in_t);
                            agg[j] = 0.0;
                            substituted = true;
                        }
                    }
                    if substituted {
                        continue;
                    }
                    let plen = path.len();
                    let mut skip_col = |j: usize, arcs: &Vec<Vec<(usize, f64)>>, other: &Vec<Vec<(usize, f64)>>, try_neg: &mut bool, path: &Vec<usize>| -> bool {
                        if plen == 1 && !*try_neg {
                            if arcs[j].len() <= plen {
                                if arcs[j].iter().any(|&(r, _)| r != start) {
                                    *try_neg = true;
                                }
                            } else {
                                *try_neg = true;
                            }
                        }
                        if other[j].is_empty() {
                            return true;
                        }
                        if other[j].len() <= plen {
                            return other[j].iter().all(|&(r, _)| in_path(r, path));
                        }
                        false
                    };
                    let mut best_out: Option<(usize, f64, f64)> = None;
                    let mut best_in: Option<(usize, f64, f64)> = None;
                    for &(j, v) in &snapshot {
                        if !cont[j] {
                            continue;
                        }
                        let d = bound_dist(j);
                        if v < 0.0 {
                            if skip_col(j, &out_arcs, &in_arcs, &mut try_neg, &path) {
                                continue;
                            }
                            if best_out.is_none_or(|(_, _, bd)| d > bd) {
                                best_out = Some((j, v, d));
                            }
                        } else {
                            if skip_col(j, &in_arcs, &out_arcs, &mut try_neg, &path) {
                                continue;
                            }
                            if best_in.is_none_or(|(_, _, bd)| d > bd) {
                                best_in = Some((j, v, d));
                            }
                        }
                    }
                    // カットを作る (集約行と、その符号を反転したもの)
                    let n_before = cands.len();
                    let amax = touched.iter().fold(0.0f64, |mx, &k| mx.max(agg[k].abs()));
                    let base: Vec<(usize, f64)> = touched.iter().filter(|&&k| agg[k].abs() > 1e-9 * amax.max(1.0) || k >= n && agg[k] != 0.0).map(|&k| (k, agg[k])).collect();
                    let neg: Vec<(usize, f64)> = base.iter().map(|&(k, a)| (k, -a)).collect();
                    if gen_old() {
                        push(cmir(vars, &base, 0.0), cands, self);
                        if use_lifted_cover() {
                            push(lifted_cover(vars, &base, 0.0), cands, self);
                        }
                        if use_flow_cover() {
                            push(lifted_flow_cover(vars, &base, 0.0), cands, self);
                        }
                        push(cmir(vars, &neg, 0.0), cands, self);
                        if use_lifted_cover() {
                            push(lifted_cover(vars, &neg, 0.0), cands, self);
                        }
                        if use_flow_cover() {
                            push(lifted_flow_cover(vars, &neg, 0.0), cands, self);
                        }
                    } else {
                        for b in [&base, &neg] {
                            let n0 = cands.len();
                            for r in generate_cuts(vars, b, 0.0, use_flow_cover(), false) {
                                push(Some(r), cands, self);
                            }
                            keep_best(cands, n0, self.p);
                        }
                    }
                    success = cands.len() > n_before;
                    // 診断 `ENOMOTO_MIP_GAP4_PATHDIAG=2`: 等式の始点で 1 段目にカットが出なかった行を 6 本まで、列ごとに (列, 係数, LP 値, 下限, 上限, 整数か, VUB) を出す
                    if pdiag && !success && path.len() == 1 && base.len() >= 3 && env_str!("ENOMOTO_MIP_GAP4_PATHDIAG") == Some("2") && base.iter().any(|&(k, _)| k < n && !vars.is_int[k] && vars.vb.is_some_and(|vb| vb.vub[k].iter().any(|&(y, _, _)| vars.x[y] > 1e-6 && vars.x[y] < 1.0 - 1e-6))) {
                        let desc: Vec<String> = base
                            .iter()
                            .map(|&(k, a)| {
                                if k < n {
                                    let vub = vars.vb.and_then(|vb| vb.vub.get(k)).map(|v| format!("{v:?}")).unwrap_or_default();
                                    format!("({k} a={a:.4} x={:.6} [{},{}] int={} vub={vub})", vars.x[k], vars.lo[k], vars.up[k], vars.is_int[k])
                                } else {
                                    format!("(row{} a={a:.4} x={:.6} [{},{}])", k - n, vars.x[k], vars.lo[k], vars.up[k])
                                }
                            })
                            .collect();
                        eprintln!("GAP4 PATHROW start {start} scale {scale}: {}", desc.join(" "));
                    }
                    pd[2] += 1;
                    if success {
                        pd[3] += 1;
                    } else if best_out.is_none() && best_in.is_none() {
                        pd[4] += 1;
                    } else if path.len() + 1 >= MAX_PATH_LEN {
                        pd[6] += 1;
                    }
                    if success || (best_out.is_none() && best_in.is_none()) {
                        break;
                    }
                    // 消す連続変数と相手の行 (乱数の位置から、経路にない行で重みが範囲内のもの)
                    let find_row = |j: usize, v: f64, arcs: &Vec<(usize, f64)>, s: &mut Self, path: &Vec<usize>| -> Option<(usize, f64)> {
                        if arcs.is_empty() {
                            return None;
                        }
                        let k0 = ((s.rand() * arcs.len() as f64) as usize).min(arcs.len() - 1);
                        for t in 0..arcs.len() {
                            let (r, a) = arcs[(k0 + t) % arcs.len()];
                            let w = -v / a;
                            if !in_path(r, path) && w.abs() <= 1.0 / feastol && w.abs() >= feastol {
                                let _ = j;
                                return Some((r, w));
                            }
                        }
                        None
                    };
                    let pick = match (best_out, best_in) {
                        (Some(o), i) if i.is_none_or(|i| o.2 >= i.2 - feastol) => find_row(o.0, o.1, &in_arcs[o.0], self, &path).or_else(|| i.and_then(|i| find_row(i.0, i.1, &out_arcs[i.0], self, &path))),
                        (_, Some(i)) => find_row(i.0, i.1, &out_arcs[i.0], self, &path),
                        _ => None,
                    };
                    let Some((r, w)) = pick else {
                        pd[5] += 1;
                        break;
                    };
                    add_row(r, w, &mut agg, &mut touched, &mut in_t);
                    path.push(r);
                }
                let _ = success;
                if !try_neg {
                    break;
                }
            }
        }
        if pdiag {
            pd_out(&pd, m);
        }
    }

    /// 経路集約: 元の行を 1 本選び、集約行に残った連続変数のうち LP 値が境界 (変数上下限を含む) から
    /// 最も離れたものを、それを含む別の元の行で打ち消す、を最大 `PATH_MAX_LEN` 段繰り返し、各段で CMIR を試す
    /// (Marchand & Wolsey の集約ヒューリスティクス)。行 i の活動量を変数 `n + i` として
    /// `sum_i w_i (a_i x - r_i) = 0` の形で集約するので、等式・不等式・範囲行を区別せずに扱える
    /// (`r_i` の置き換えは CMIR が行の上下限で行う)。
    fn path_aggregation<F>(&mut self, vars: &CutVars, lp_rows: &[Vec<(usize, f64)>], cands: &mut Vec<Candidate>, push: &mut F, max_starts: usize)
    where
        F: FnMut(Option<RawCut>, &mut Vec<Candidate>, &Solver<L>),
    {
        const PATH_MAX_LEN: usize = 6;
        const MAX_ROW_LEN: usize = 500;
        let p = self.p;
        let n = p.n;
        // 元の行だけでなく LP に入っているカットの行も集約に使う (HiGHS と同じ)。再スタート後はカットが元の行になり
        // 経路の候補が大きく増えて根の下界が伸びたので、再スタートを待たずに使う
        let m = if env_str!("ENOMOTO_MIP_PATH_ORIG_ONLY").is_some() { p.m } else { lp_rows.len() };
        // 連続変数の LP 値の、最も近い境界 (単純な上下限・変数上下限) までの距離
        let bound_dist = |j: usize| -> f64 {
            let xj = vars.x[j];
            let mut d = (xj - vars.lo[j]).min(vars.up[j] - xj);
            if let Some(vb) = vars.vb {
                for &(y, a, e) in &vb.vub[j] {
                    d = d.min(a * vars.x[y] + e - xj);
                }
                for &(y, a, e) in &vb.vlb[j] {
                    d = d.min(xj - a * vars.x[y] - e);
                }
            }
            d.max(0.0)
        };
        // 列 → 元の行 (短い行だけ)
        let mut col_rows: Vec<Vec<usize>> = vec![Vec::new(); n];
        for i in 0..m {
            if lp_rows[i].len() <= MAX_ROW_LEN {
                for &(j, _) in &lp_rows[i] {
                    if !vars.is_int[j] {
                        col_rows[j].push(i);
                    }
                }
            }
        }
        // 行が LP で効いているか (活動量が上下限に近い)
        let tight = |i: usize| -> bool {
            let r = vars.x[n + i];
            let (l, u) = (vars.lo[n + i], vars.up[n + i]);
            (r - l).abs() <= 1e-6 * (1.0 + l.abs()) || (u - r).abs() <= 1e-6 * (1.0 + u.abs())
        };
        let mut agg = vec![0.0f64; n];
        let mut in_agg = vec![false; n];
        let mut touched: Vec<usize> = Vec::new();
        let mut used_row = vec![false; m];
        let mut starts = 0usize;
        // 始点の行 (既定は元の行だけ。カットの行は相手としてだけ使う)
        let m_start = if env_str!("ENOMOTO_MIP_PATH_START_CUTS").is_some() { m } else { p.m.min(m) };
        // `ENOMOTO_MIP_PATH_TIGHT_STARTS`: 始点は効いている行だけ (HiGHS)。`ENOMOTO_MIP_PATH_ROTATE`: 始点の行の順を
        // 毎回ずらす。`ENOMOTO_MIP_PATH_BOTH_SIGNS`: 始点の行を両方の向きで使う (HiGHS)。h80x6320d ではどれも根の
        // 下界を上げず (両方の向きは候補が増えすぎて LP が詰まり悪化)、既定では使わない
        let tight_only = env_str!("ENOMOTO_MIP_PATH_TIGHT_STARTS").is_some();
        let offset = if m_start > 0 && env_str!("ENOMOTO_MIP_PATH_ROTATE").is_some() { self.path_offset % m_start } else { 0 };
        let both_signs = env_str!("ENOMOTO_MIP_PATH_BOTH_SIGNS").is_some();
        let stop_on_cut = env_str!("ENOMOTO_MIP_PATH_STOP_ON_CUT").is_some();
        let mut last = offset;
        for sidx in 0..m_start * if both_signs { 2 } else { 1 } {
            let start = (offset + sidx / if both_signs { 2 } else { 1 }) % m_start;
            let sign = if both_signs && sidx % 2 == 1 { -1.0 } else { 1.0 };
            if starts >= max_starts || (sidx % 64 == 0 && self.time_up()) {
                break;
            }
            last = start;
            let row = &lp_rows[start];
            if row.len() < 2 || row.len() > MAX_ROW_LEN {
                continue;
            }
            if tight_only && !tight(start) {
                continue;
            }
            // 境界から離れた連続変数がなければ 1 行の CMIR と同じなので飛ばす
            if !row.iter().any(|&(j, _)| !vars.is_int[j] && bound_dist(j) > 1e-6) {
                continue;
            }
            starts += 1;
            for &j in &touched {
                agg[j] = 0.0;
                in_agg[j] = false;
            }
            touched.clear();
            let mut weights: Vec<(usize, f64)> = vec![(start, sign)];
            used_row[start] = true;
            for &(j, a) in row {
                if !in_agg[j] {
                    in_agg[j] = true;
                    touched.push(j);
                }
                agg[j] += sign * a;
            }
            for step in 0..PATH_MAX_LEN {
                if step > 0 {
                    let n_before = cands.len();
                    let mut base: Vec<(usize, f64)> = Vec::with_capacity(touched.len() + weights.len());
                    let amax = touched.iter().fold(0.0f64, |mx, &j| mx.max(agg[j].abs()));
                    for &j in &touched {
                        if agg[j].abs() > 1e-9 * amax.max(1.0) {
                            base.push((j, agg[j]));
                        }
                    }
                    for &(i, w) in &weights {
                        base.push((n + i, -w));
                    }
                    // 並列の分離: 集約した行を集めるだけにする (カットは呼び出し側でまとめて作る)
                    if let Some(col) = self.path_collect.as_mut() {
                        col.push(base);
                    } else {
                        let neg: Vec<(usize, f64)> = base.iter().map(|&(k, a)| (k, -a)).collect();
                        if gen_old() {
                            push(cmir(vars, &base, 0.0), cands, self);
                            if use_lifted_cover() {
                                push(lifted_cover(vars, &base, 0.0), cands, self);
                            }
                            push(cmir(vars, &neg, 0.0), cands, self);
                            if use_lifted_cover() {
                                push(lifted_cover(vars, &neg, 0.0), cands, self);
                            }
                            if use_flow_cover() {
                                push(lifted_flow_cover(vars, &base, 0.0), cands, self);
                                push(lifted_flow_cover(vars, &neg, 0.0), cands, self);
                            }
                        } else {
                            for b in [&base, &neg] {
                                let n0 = cands.len();
                                for r in generate_cuts(vars, b, 0.0, use_flow_cover(), false) {
                                    push(Some(r), cands, self);
                                }
                                keep_best(cands, n0, self.p);
                            }
                        }
                        // HiGHS と同じく、カットが出たら経路を伸ばさない (`ENOMOTO_MIP_PATH_STOP_ON_CUT`)
                        if cands.len() > n_before && stop_on_cut {
                            break;
                        }
                    }
                }
                // 打ち消す連続変数: 境界から最も離れたもの
                let amax = touched.iter().fold(0.0f64, |mx, &j| mx.max(agg[j].abs()));
                let mut best: Option<(usize, f64)> = None;
                for &j in &touched {
                    if vars.is_int[j] || agg[j].abs() <= 1e-9 * amax.max(1.0) {
                        continue;
                    }
                    let d = bound_dist(j);
                    if d > 1e-6 && best.is_none_or(|(_, bd)| d > bd) {
                        best = Some((j, d));
                    }
                }
                let Some((j, _)) = best else { break };
                // 相手の行: 未使用で j を含むもの。効いている行を優先し、同じ組では乱数で選ぶ
                let mut pick: Option<(usize, f64)> = None;
                for &i in &col_rows[j] {
                    if used_row[i] {
                        continue;
                    }
                    let score = if tight(i) { 2.0 } else { 1.0 } + 0.5 * self.rand();
                    if pick.is_none_or(|(_, sc)| score > sc) {
                        pick = Some((i, score));
                    }
                }
                let Some((r, _)) = pick else { break };
                let arj = lp_rows[r].iter().find(|&&(k, _)| k == j).map_or(0.0, |&(_, a)| a);
                if arj.abs() < 1e-9 {
                    break;
                }
                let w = -agg[j] / arj;
                // 重みの比が大きすぎる集約は数値的に危ないので止める
                let wmax = weights.iter().fold(w.abs(), |mx, &(_, v)| mx.max(v.abs()));
                let wmin = weights.iter().fold(w.abs(), |mn, &(_, v)| mn.min(v.abs()));
                if wmax / wmin > 1e4 {
                    break;
                }
                used_row[r] = true;
                weights.push((r, w));
                for &(k, a) in &lp_rows[r] {
                    if !in_agg[k] {
                        in_agg[k] = true;
                        touched.push(k);
                    }
                    agg[k] += w * a;
                }
                agg[j] = 0.0;
            }
            for &(i, _) in &weights {
                used_row[i] = false;
            }
        }
        self.path_offset = last + 1;
    }

    /// 試作 `ENOMOTO_MIP_MPSEP` の絞った形: 今の LP の最適面の広さの目安 = 被約費用が 0 (相対 1e-9) で固定されていない非基底の構造列と、
    /// 双対値が 0 の非基底の (等式でない) 行の数 (`TwoStageLp::optface_move` の候補の数に近い)。
    fn mpsep_width(&self) -> usize {
        let p = self.p;
        let d = self.lp.reduced_costs();
        let b = self.lp.basis();
        let y = self.lp.row_duals();
        let mut k = 0usize;
        for j in 0..p.n.min(b.col.len()) {
            if b.col[j] == VarStatus::Basic {
                continue;
            }
            let (l, u) = self.lp.col_bounds(j);
            if l < u && d[j].abs() <= 1e-9 * (1.0 + p.cost[j].abs()) {
                k += 1;
            }
        }
        for i in 0..b.row.len().min(y.len()) {
            if b.row[i] == VarStatus::Basic {
                continue;
            }
            let (l, u) = self.lp.row_bounds(i);
            if l < u && y[i].abs() <= 1e-9 {
                k += 1;
            }
        }
        k
    }

    /// 試作 `ENOMOTO_MIP_MPSEP` (既定では使わない、analysis/mpsep_20261106.md): 根のカットのループの 1 ラウンドで、今の頂点 `x` (x*) の
    /// ほかの点でも分離して `cands` に足し、足した点を返す。
    /// - 値に `o`: 最適面の上の別の最適基底 ([`Solver::optface_alt_ext`]、被約費用 0 の非基底を入れる軸の入れ替え) を
    ///   `ENOMOTO_T_MIP_MPSEP_NV` (既定 2) 個作り、それぞれの基底と頂点で分離する (tableau 系 (GMI・MIR の tableau 行) は別の基底の行、
    ///   それ以外 (cover・clique・flow cover・行の MIR) は点だけが変わる)。1 個の移動は `ENOMOTO_T_MIP_MPSEP_MOVE_SECS`
    ///   (既定は根の LP の時間の 0.25 倍を 0.01〜0.1 秒に収めたもの) まで、入れ替えは `ENOMOTO_T_MIP_MPSEP_PIVOTS` (既定 1000) 回まで。
    /// - 値に `c` (費用つき = 最適面の中心付近) か `a` (費用 0 = 実行可能領域の中心付近): ループで 1 回だけ内点法で内点 x_c を求め
    ///   (`ENOMOTO_T_MIP_MPSEP_IPM_SECS`、既定は根の LP の時間の 3 倍を 0.2 秒〜時間の上限の 3% に収めたもの)、x* と x_c を結ぶ線分上の点
    ///   x(α) = x* + α (x_c − x*) (`ENOMOTO_T_MIP_MPSEP_ALPHA`、既定 0.5) で分離する (行の活動量はその点から計算する)。
    ///   x(α) を切る候補がなければ x_c ← x(α) (Ben-Ameur・Neto の更新)。
    /// - 値に `l`: 足す点では軽い分離 (tableau 行・経路集約の始点を減らす)。
    /// - `ENOMOTO_T_MIP_MPSEP_START` (既定 0) ラウンド目から `ENOMOTO_T_MIP_MPSEP_ROUNDS` (既定 1000) ラウンドだけ。
    #[allow(clippy::too_many_arguments)]
    fn mpsep_extra(&mut self, f: &str, x: &[f64], round: usize, cands: &mut Vec<Candidate>, center: &mut Option<Vec<f64>>, center_tried: &mut bool, stats: &mut (usize, usize, usize, usize, usize, f64)) -> Vec<Vec<f64>> {
        let r0 = tunable!("ENOMOTO_T_MIP_MPSEP_START", 0usize, usize);
        let nr = tunable!("ENOMOTO_T_MIP_MPSEP_ROUNDS", 1000usize, usize);
        if round < r0 || round >= r0 + nr || self.time_up() {
            return Vec::new();
        }
        let t0 = std::time::Instant::now();
        let n = self.p.n;
        let light = f.contains('l');
        let n0 = cands.len();
        let mut pts: Vec<Vec<f64>> = Vec::new();
        let mut nmoved = 0usize;
        if f.contains('o') {
            let nv = tunable!("ENOMOTO_T_MIP_MPSEP_NV", 1usize, usize);
            let cap = tunable!("ENOMOTO_T_MIP_MPSEP_MOVE_SECS", (0.25 * self.root_lp_secs).clamp(0.01, 0.1), f64);
            let piv = tunable!("ENOMOTO_T_MIP_MPSEP_PIVOTS", 1000usize, usize);
            let mode = if f.contains('d') { 'd' } else if f.contains('r') { 'r' } else { 'f' };
            let dbg = f.contains('v') && self.params.verbose;
            for k in 0..nv {
                if self.time_up() {
                    break;
                }
                let seed = 7919 * (round as u64 + 1) + k as u64 + 1;
                let (alt, _) = self.optface_alt_ext("mpsep", mode, seed, cap, piv, dbg);
                let Some(mut alt) = alt else { continue };
                let xa = alt.col_values();
                if xa.len() != n {
                    continue;
                }
                std::mem::swap(&mut self.lp, &mut alt);
                let mut cv = self.separate(&xa, light);
                std::mem::swap(&mut self.lp, &mut alt);
                cands.append(&mut cv);
                pts.push(xa);
                nmoved += 1;
            }
        }
        let mut io = false;
        if f.contains('c') || f.contains('a') {
            if center.is_none() && !*center_tried {
                *center_tried = true;
                let tl = if self.params.time_limit.is_finite() { 0.03 * self.params.time_limit } else { 10.0 };
                let secs = tunable!("ENOMOTO_T_MIP_MPSEP_IPM_SECS", (3.0 * self.root_lp_secs).clamp(0.2, tl.max(0.2)), f64);
                let t1 = std::time::Instant::now();
                let dl = std::time::Instant::now() + std::time::Duration::from_secs_f64(secs);
                let dl = self.deadline.map_or(dl, |g| g.min(dl));
                let ip = self.lp.interior_point(!f.contains('c'), false, dl);
                *center = ip.map(|ip| ip.x).filter(|v| v.len() == n);
                if self.params.verbose {
                    let diff = center.as_ref().map_or(0, |c| (0..n).filter(|&j| (c[j] - x[j]).abs() > 1e-6).count());
                    let nfc = center.as_ref().map_or(0, |c| self.fractional(c).len());
                    eprintln!("MIP: mpsep: interior point ({}) {} in {:.3}s: {diff} columns differ from x*, {nfc} fractional (x*: {})", if f.contains('c') { "optimal face" } else { "analytic" }, if center.is_some() { "found" } else { "not found" }, t1.elapsed().as_secs_f64(), self.fractional(x).len());
                }
            }
            if let Some(xc) = center.clone() {
                let al = tunable!("ENOMOTO_T_MIP_MPSEP_ALPHA", 0.5, f64);
                let xm: Vec<f64> = (0..n).map(|j| x[j] + al * (xc[j] - x[j])).collect();
                let (rows, _) = self.sep_lp_rows();
                let act: Vec<f64> = rows.iter().map(|r| r.iter().map(|&(j, a)| a * xm[j]).sum()).collect();
                self.sep_act_override = Some(act);
                let mut cv = self.separate(&xm, light);
                self.sep_act_override = None;
                let viol = cv.iter().any(|c| c.coefs.iter().map(|&(j, a)| a * xm[j]).sum::<f64>() - c.rhs > 1e-6 * (1.0 + c.rhs.abs()));
                if !viol {
                    *center = Some(xm.clone());
                }
                cands.append(&mut cv);
                pts.push(xm);
                io = true;
            }
        }
        let secs = t0.elapsed().as_secs_f64();
        if !pts.is_empty() {
            stats.0 += 1;
            stats.1 += pts.len();
            stats.2 += cands.len() - n0;
        }
        stats.5 += secs;
        if self.params.verbose && f.contains('v') {
            eprintln!("MIP: cut round {round}: mpsep: {nmoved} moved vertices, in-out {io}, +{} candidates ({:.3}s)", cands.len() - n0, secs);
        }
        pts
    }

    /// 根の分離 (退化に強い形、`ENOMOTO_MIP_FACE_SEP`)。LP が双対退化している (非基底で被約費用が 0 の列がある) と
    /// 最適解の集合 (最適面) に頂点が多く、どの頂点に着くかは DSE 重みなど解き方の履歴で変わる。カットは着いた頂点から
    /// 作るので、頂点が変わるとカットも根の下界も変わり、また 1 つの頂点だけを切っても LP は同じ目的値の別の頂点へ移るだけで
    /// 下界が動かない。そこで最適面の上で決まった頂点を選んで分離する:
    /// 被約費用が 0 でない非基底列を今の値に固定した LP (の複製) を、費用 c + ε c2 で解き直す (ε c2 は小さいので
    /// 主の目的値は変わらず、最適面の中で c2 が最小の頂点に移る)。モード:
    /// - `canon`: c2 は列ごとに決まった擬似乱数。その頂点だけで分離する (着いた頂点によらず同じ点から分離する)
    /// - `both`: c2 は同じ擬似乱数。今の頂点と面の頂点の両方で分離して候補を合わせる
    /// - `int`: c2 は分数の整数列を近い整数の側へ押す向き。両方で分離する
    /// 面の頂点の真の目的値が今の値から離れたら (ε が大きすぎた) 今の頂点だけで分離する。
    /// 既定は `int` (`off` で使わない)。40 問 (DSE 重みを全ての LP で引き継ぐ既定のもとで): 使わない 18 問 28.07、
    /// `int` 19 問 27.04 (misc07 時間切れ -> 40.9 秒、neos5 時間切れ -> 59.5 秒、mik-250 42.4 -> 16.9 秒、nw04 20.5 -> 8.7 秒、
    /// rout 57.8 秒 -> 時間切れ)、`canon` 17 問 27.66。既定は `int2` (同時に測った `int` 18 問 27.94 に対し 20 問 27.70:
    /// rout 時間切れ -> 31.2 秒、neos5 時間切れ -> 53.2 秒、misc07 39.6 -> 54.1 秒)
    fn separate_face(&mut self, x: &[f64], light: bool) -> Vec<Candidate> {
        let (mut a, mut b) = self.separate_face_split(x, light);
        a.append(&mut b);
        a
    }

    /// [`Self::separate_face`] の候補を、今の頂点からのもの と 面の頂点からのもの に分けて返す。
    fn separate_face_split(&mut self, x: &[f64], light: bool) -> (Vec<Candidate>, Vec<Candidate>) {
        if crate::simplex::slope_intercept_dual::mip_profile() == 1 {
            return (self.separate(x, light), Vec::new());
        }
        if (CUT_EXTENDED.with(|c| c.get()) || self.equiv2_cuts()) && !self.params.submip {
            return (self.separate(x, light), Vec::new());
        }
        // 試作 (inv/gfrsplit-20261021): `ENOMOTO_MIP_FACE_SEP_BG=off` なら背景のカットの求解 (`CUT_JOB`) だけ面の頂点の分離を止める
        // (根の手前のカット・ヒューリスティクスの経路は既定のまま)。`ENOMOTO_MIP_GR_AUTO=2` なら一般整数の列がある問題だけ止める
        // (G が働く問題。mzzv42z は G と面の分離を止めることの両方で根の RENS が崩れなくなる)
        let in_bg = self.snap.is_some() && super::solver::CUT_JOB.with(|j| j.borrow().is_some());
        if in_bg && env_str!("ENOMOTO_MIP_FACE_SEP_BG") == Some("off") {
            return (self.separate(x, light), Vec::new());
        }
        if super::gr_auto_face_off() && super::implbd_genint_for(&self.p.is_int, &self.p.col_lo, &self.p.col_up) {
            return (self.separate(x, light), Vec::new());
        }
        // `ENOMOTO_MIP_FACE_SEP_FC=off` (2026-10-21 から既定。`=on` で止めない): 固定費用ネットワーク型 (根のカット選択を緩める問題) では面の頂点の分離を止める。
        // mc11: 止めると根の手前のカットが 2.6 -> 1.1〜1.5 秒で 11069 (止めなければ 11108) に着き、背景のカットが 23 -> 50〜56
        // ラウンド回って 11455 -> 11672〜11674 (最適 11689) になり 60 秒で解ける (背景のカットだけ止めても 11549〜11604 で解けない)
        // `ENOMOTO_T_MIP_FACE_SEP_FC_MAXCOLS` (既定 5000): 列 (前処理の後) がこの数を超える問題では止めない。
        // h80x6320d (12640 列) では F で根の頂点が変わると根の RINS が 9 回中 5 回 6644.8 しか出さず (既定は 6 回とも
        // 6388〜6418)、最初の再スタートで固まる列が 4473〜5126 -> 996 本になって 2 回目の再スタートが要り 54〜60 秒かかる
        // (analysis/qf_narrow_20261021.md)。mc11 は 3034 列
        if self.fc_network && env_str!("ENOMOTO_MIP_FACE_SEP_FC") != Some("on") && self.p.n <= tunable!("ENOMOTO_T_MIP_FACE_SEP_FC_MAXCOLS", 5000usize, usize) {
            return (self.separate(x, light), Vec::new());
        }
        // `ENOMOTO_MIP_FACE_ADAPT` (試作): 面の LP が最適に解けなかったら、このループの残りでは面の頂点の分離をしない
        // (physiciansched6-2 では面の LP 2 回が時間の上限 1.2 秒に当たって使われず、1 ラウンド 3.3 秒のうち 2.3 秒を占めた)。
        // また面の LP の時間の上限を、直前のラウンドのカットの LP の時間の `ENOMOTO_T_MIP_FACE_ADAPT_MULT` 倍 (既定 2、最低 0.05 秒) にする
        let face_adapt = env_str!("ENOMOTO_MIP_FACE_ADAPT").is_some() && !self.params.submip;
        // `ENOMOTO_MIP_CUT_AGG` の `f` は、列が行の `ENOMOTO_T_MIP_CUT_AGG_F_MAX_COLS_PER_ROW` 倍 (既定 100) を超える問題では使わない:
        // nw04 (36 行・87482 列) は面の頂点のカットで根を 4 ラウンドで閉じ (8 秒)、止めると 8 ラウンドで 16776 止まり (22 秒)。
        // h80x6320d・drayage-100-23・10teams は止めると速くなる (analysis/cand9bisect_20261024.md)
        let agg_face_off = self.cut_agg_has('f') && (self.p.n as f64) <= tunable!("ENOMOTO_T_MIP_CUT_AGG_F_MAX_COLS_PER_ROW", 100.0, f64) * self.p.m.max(1) as f64;
        if (face_adapt && self.face_off_loop) || agg_face_off {
            return (self.separate(x, light), Vec::new());
        }
        let mode = match env_str!("ENOMOTO_MIP_FACE_SEP") {
            Some(m) if m == "off" => return (self.separate(x, light), Vec::new()),
            Some(m) if !self.params.submip => m,
            None if !self.params.submip => "int2",
            _ => return (self.separate(x, light), Vec::new()),
        };
        let t_prep0 = std::time::Instant::now();
        let p = self.p;
        let n = p.n;
        let d = self.lp.reduced_costs();
        let b = self.lp.basis();
        let cmax = p.cost.iter().fold(0.0f64, |m, c| m.max(c.abs())).max(1.0);
        let mut face = self.lp.clone();
        let mut ndeg = 0usize;
        for j in 0..n {
            if b.col[j] == VarStatus::Basic {
                continue;
            }
            let (l, u) = face.col_bounds(j);
            if l == u {
                continue;
            }
            if d[j].abs() > 1e-7 * (1.0 + p.cost[j].abs()) {
                face.set_col_bounds(j, x[j], x[j]);
            } else {
                ndeg += 1;
            }
        }
        // `ENOMOTO_MIP_FACE_BASIS`: 双対値が 0 で非基底の行 (スラック) による退化も数え、面の LP で点が動かなくても
        // 基底が変わっていればその基底で分離する (主退化で最適基底が複数ある場合。tableau 行が変わるので別のカットが出る)
        // 40 問 (`int2` のもとで): 使わない 20 問 27.89、使う 18 問 28.07 (misc07 57.3 -> 45.6 秒だが、neos5・neos-860300 が
        // 時間切れ、nw04 10.5 -> 16.4 秒)。既定では使わない
        let face_basis = env_str!("ENOMOTO_MIP_FACE_BASIS").is_some();
        if face_basis {
            let y = self.lp.row_duals();
            for i in 0..b.row.len() {
                if b.row[i] != VarStatus::Basic && y[i].abs() <= 1e-9 {
                    ndeg += 1;
                }
            }
        }
        if ndeg == 0 {
            return (self.separate(x, light), Vec::new());
        }
        let eps = tunable!("ENOMOTO_T_MIP_FACE_EPS", 1e-5, f64) * cmax;
        let hash = |j: usize| -> f64 {
            let mut h = (j as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
            h ^= h >> 31;
            h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            h ^= h >> 29;
            (h >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
        };
        // 副目的の向き (`int2`: 近い整数の側へ押す頂点と、遠い側へ押す頂点の 2 つ)
        let near: Vec<f64> = (0..n)
            .map(|j| {
                let f = x[j] - x[j].floor();
                if self.cut_int[j] && f > 1e-6 && f < 1.0 - 1e-6 {
                    if f < 0.5 { 1.0 } else { -1.0 }
                } else {
                    0.0
                }
            })
            .collect();
        let dirs: Vec<Vec<f64>> = match mode {
            "int" => vec![near],
            "int2" => {
                let far: Vec<f64> = near.iter().map(|v| -v).collect();
                vec![near, far]
            }
            _ => vec![(0..n).map(hash).collect()],
        };
        let z0: f64 = (0..n).map(|j| p.cost[j] * x[j]).sum();
        self.tab_memo.clear();
        self.tab_memo_mode = 1;
        let prof = sepprof::on() && !self.params.submip;
        if prof {
            sepprof::add(sepprof::T_FACE_PREP, t_prep0.elapsed().as_secs_f64());
        }
        let cands = if mode == "canon" { Vec::new() } else { self.separate(x, light) };
        self.tab_memo_mode = 2;
        let t_face0 = std::time::Instant::now();
        let mut fcands: Vec<Candidate> = Vec::new();
        let mut used = 0usize;
        // 面の LP 1 回の時間の上限 (カットを加えた LP と同じ、全体の 2%、最低 1 秒)。neos-3656078-kumeu・neos-933966 では
        // 根の最初のラウンドの面の LP が数値的に破綻して解き直しを繰り返し、残り時間 (50 秒以上) を使い切っていた
        let cap = if self.params.time_limit.is_finite() { (self.cut_lp_time_frac() * self.params.time_limit).max(1.0) } else { f64::INFINITY };
        // 最初のラウンド (直前のカットの LP がない) は `ENOMOTO_T_MIP_FACE_ADAPT_FIRST` 秒 (既定 0.25) まで
        let cap = if !face_adapt {
            cap
        } else if self.last_cut_lp_secs.is_finite() {
            cap.min((tunable!("ENOMOTO_T_MIP_FACE_ADAPT_MULT", 2.0, f64) * self.last_cut_lp_secs).max(0.05))
        } else {
            cap.min(tunable!("ENOMOTO_T_MIP_FACE_ADAPT_FIRST", 0.25, f64))
        };
        for c2 in &dirs {
            let t_fl0 = std::time::Instant::now();
            let mut f2 = face.clone();
            let cost2: Vec<f64> = (0..n).map(|j| p.cost[j] + eps * c2[j]).collect();
            f2.set_costs(&cost2);
            let deadline = if cap.is_finite() {
                let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(cap);
                Some(self.deadline.map_or(d, |g| g.min(d)))
            } else {
                self.deadline
            };
            let st = f2.solve(&SolveLimits { iteration_limit: if light { (p.m + n) as u64 + 100 } else { 10 * (p.m + n) as u64 + 1000 }, cutoff: f64::INFINITY, deadline });
            if prof {
                sepprof::add(sepprof::T_FACE_LP, t_fl0.elapsed().as_secs_f64());
                sepprof::PROF.with(|pp| pp.borrow_mut().face_solves += 1);
            }
            if st != LpStatus::Optimal {
                if face_adapt {
                    self.face_off_loop = true;
                    if self.params.verbose {
                        eprintln!("MIP: face LP {st:?}; no face separation for the rest of this cut loop");
                    }
                    break;
                }
                continue;
            }
            let xv = f2.col_values();
            let zv: f64 = (0..n).map(|j| p.cost[j] * xv[j]).sum();
            let moved = (0..n).filter(|&j| (xv[j] - x[j]).abs() > 1e-6).count();
            if env_str!("ENOMOTO_MIP_DEBUG_FACE").is_some() {
                eprintln!("FACE: degenerate nonbasic {ndeg}, moved {moved} cols, obj {z0:.9} -> {zv:.9}, frac {} -> {}", self.fractional(x).len(), self.fractional(&xv).len());
            }
            if zv > z0 + 1e-7 * (1.0 + z0.abs()) {
                continue;
            }
            if moved == 0 {
                // 点は同じ。基底が変わっていればその基底で分離する (`ENOMOTO_MIP_FACE_BASIS`)
                let fb = f2.basis();
                let same = fb.col.iter().zip(&b.col).all(|(u, v)| (*u == VarStatus::Basic) == (*v == VarStatus::Basic))
                    && fb.row.iter().zip(&b.row).all(|(u, v)| (*u == VarStatus::Basic) == (*v == VarStatus::Basic));
                if !face_basis || same {
                    continue;
                }
            }
            // `ENOMOTO_MIP_FACE_LIGHT`: 面の頂点では軽い分離 (経路集約の始点・tableau 行を減らす) にする
            let lt = light || env_str!("ENOMOTO_MIP_FACE_LIGHT").is_some();
            std::mem::swap(&mut self.lp, &mut f2);
            sepprof::AT_FACE.with(|c| c.set(true));
            let mut cv = self.separate(&xv, lt);
            sepprof::AT_FACE.with(|c| c.set(false));
            std::mem::swap(&mut self.lp, &mut f2);
            if prof {
                sepprof::PROF.with(|pp| pp.borrow_mut().face_used += 1);
            }
            fcands.append(&mut cv);
            used += 1;
        }
        self.tab_memo_mode = 0;
        self.face_secs += t_face0.elapsed().as_secs_f64();
        self.face_used += used;
        if env_str!("ENOMOTO_MIP_DEBUG_FACE").is_some() {
            eprintln!("FACE tab memo: checked {} rows, same as at the current vertex {}", self.tab_memo_stats.0, self.tab_memo_stats.1);
        }
        if mode == "canon" && used == 0 {
            return (self.separate(x, light), Vec::new());
        }
        (cands, fcands)
    }

    /// [`fixed_charge_network`] の判定 (`ENOMOTO_MIP_SEPA_MCF=auto` 用。問題ごとに一度だけ数える)
    fn mcf_fc_network(&mut self) -> bool {
        *self.mcf_fc.get_or_insert_with(|| fixed_charge_network(self.p))
    }

    /// 多品種フローの網 ([`super::mcf::extract`]、最初に呼ばれたときに一度だけ探す)。`ENOMOTO_MIP_DEBUG_MCF` で検出の結果を出す
    pub(super) fn mcf_networks(&mut self) -> std::rc::Rc<Vec<super::mcf::McfNetwork>> {
        if let Some(nets) = &self.mcf_nets {
            return nets.clone();
        }
        let t0 = std::time::Instant::now();
        let (nets, st) = super::mcf::extract(self.p);
        if env_str!("ENOMOTO_MIP_DEBUG_MCF").is_some() || env_str!("ENOMOTO_MIP_LOG").is_some() {
            let desc: Vec<String> = nets.iter().map(|g| format!("{}n/{}a({}uncap)/{}k{}", g.nnodes, g.narcs(), g.nuncap, g.ncom, if g.directed { "" } else { "/undir" })).collect();
            eprintln!(
                "MCF detect: rows {} cols {} | flow cands {} flow rows {} commodities {} cap cands {} arcs {} nodes {} incons {:.2} {} | fail '{}' | networks [{}] fc_network {} | {:.3}s",
                self.p.m,
                self.p.n,
                st.flowcands,
                st.flowrows,
                st.commodities,
                st.capcands,
                st.arcs,
                st.nodes,
                st.inconsistencies,
                if st.directed { "directed" } else { "undirected" },
                st.fail,
                desc.join(" "),
                fixed_charge_network(self.p),
                t0.elapsed().as_secs_f64()
            );
        }
        let nets = std::rc::Rc::new(nets);
        self.mcf_nets = Some(nets.clone());
        nets
    }

    /// 現在の LP 解 `x` を切る候補を作る。
    /// `light` (ノード用) なら経路集約の始点と tableau 行の数を絞る。
    /// `ENOMOTO_MIP_ORIGROW_TIGHT`: 元の行 1 本ずつの分離で、行 `i` の (`<=` 側, `>=` 側) それぞれに試すもの (0 = 試さない,
    /// 1 = 全部 (`generate_cuts`), 2 = extended cover だけ)。`None` なら全部試す (既定と同じ)。
    /// - 根 (HiGHS の経路集約 [`Self::path_aggregation_highs`] を使うとき): 経路集約の始点になる行 (LP で効いている行と等式、
    ///   [`pathh_row_types`] の型が 0 でない行) は、経路集約が最初の段で同じ 1 行 (その行と符号を反転したもの) から CMIR・
    ///   flow cover を作るので、ここでは extended cover だけ試す (経路集約は extended cover を作らない)。それ以外の行は全部試す
    /// - ノード (`light`): 余裕 / 係数の 2 ノルム が `ENOMOTO_T_MIP_ORIGROW_NODE_SLACK` (既定 0.1) 未満の向きだけ試す
    ///   (CglKnapsackCover も効いている行だけを使う。ノードでは経路集約を使わない)
    fn orig_row_modes(&self, vars: &CutVars, lp_rows: &[Vec<(usize, f64)>], act: &[f64], light: bool) -> Option<Vec<[u8; 2]>> {
        let p = self.p;
        if light {
            let thr = tunable!("ENOMOTO_T_MIP_ORIGROW_NODE_SLACK", 0.1, f64);
            return Some(
                (0..p.m)
                    .map(|i| {
                        let norm = p.rows[i].iter().map(|&(_, a)| a * a).sum::<f64>().sqrt().max(1e-12);
                        let a = act[i];
                        [((p.row_up[i] - a) / norm < thr) as u8, ((a - p.row_lo[i]) / norm < thr) as u8]
                    })
                    .collect(),
            );
        }
        // HiGHS の経路集約を使うか ([`Self::separate`] の経路集約の条件。根では `old_only` は `ENOMOTO_MIP_PATH_OLD` だけ)
        let pathh_runs = env_str!("ENOMOTO_MIP_NO_PATH_AGG").is_none() && env_str!("ENOMOTO_MIP_PATH_OLD").is_none() && !self.sep_src_skipped(sepprof::SRC_PATHH, light);
        if !pathh_runs {
            return None;
        }
        let (rtype, _, _, _) = pathh_row_types(vars, lp_rows, p.n);
        Some((0..p.m).map(|i| if rtype[i] != 0 { [2, 2] } else { [1, 1] }).collect())
    }

    /// [`Self::separate`] で使う LP の行の写しと、(`ENOMOTO_MIP_SEP_NO_ROWCLONE` なら) 行ごとの整数の行の印。既定では毎回
    /// すべての行を複製し、印は `None` (呼び出し側が数える)
    fn sep_lp_rows(&mut self) -> (std::rc::Rc<Vec<Vec<(usize, f64)>>>, Option<std::rc::Rc<Vec<bool>>>) {
        let mr = self.lp.num_rows();
        let key = if env_str!("ENOMOTO_MIP_SEP_NO_ROWCLONE").is_some() { self.lp.matrix_key() } else { None };
        let Some(key) = key else {
            return (std::rc::Rc::new((0..mr).map(|i| self.lp.row(i)).collect()), None);
        };
        if let Some(c) = &self.sep_rows_cache {
            if c.key == key && c.rows.len() == mr && c.cut_int == self.cut_int {
                return (c.rows.clone(), Some(c.integral.clone()));
            }
        }
        let rows: Vec<Vec<(usize, f64)>> = (0..mr).map(|i| self.lp.row(i)).collect();
        let integral: Vec<bool> = rows.iter().map(|r| r.iter().all(|&(j, a)| self.cut_int[j] && (a - a.round()).abs() <= 1e-9)).collect();
        let (rows, integral) = (std::rc::Rc::new(rows), std::rc::Rc::new(integral));
        self.sep_rows_cache = Some(SepRowsCache { key, rows: rows.clone(), cut_int: self.cut_int.clone(), integral: integral.clone() });
        (rows, Some(integral))
    }

    fn separate(&mut self, x: &[f64], light: bool) -> Vec<Candidate> {
        let mut lap = std::time::Instant::now();
        // 試作 `ENOMOTO_MIP_HNFCUT`: 合同式の表を分離の間だけ置く ([`super::hnfcut`])
        let _hnfcut = if super::hnfcut::on() { super::hnfcut::Guard::new(super::hnfcut::state_for(self.dom.cong.clone(), 'a')) } else { super::hnfcut::Guard::none() };
        if let (true, Some(t)) = (super::hnfcut::on() && !self.params.submip, self.dom.cong.as_deref()) {
            super::hnfcut::note_lp_point(t, x, &self.dom.global_lo, &self.dom.global_up);
        }
        let fidx = if sepprof::AT_FACE.with(|c| c.get()) { 1 } else { 0 };
        let src_off = if fidx == 1 { sepprof::SRC_FACE } else { 0 };
        let tag = |cands: &mut [Candidate], src: u8| {
            for c in cands.iter_mut() {
                c.src = src + src_off;
            }
        };
        let p = self.p;
        let n = p.n;
        let mr = self.lp.num_rows();
        // 変数: 構造変数 0..n と、LP 行の活動量 n..n+mr
        // `ENOMOTO_MIP_SEP_NO_ROWCLONE`: 行の写しと整数の行の印を使い回す ([`SepRowsCache`])
        let (lp_rows_rc, row_integral) = self.sep_lp_rows();
        let lp_rows: &[Vec<(usize, f64)>] = &lp_rows_rc;
        // 試作 `ENOMOTO_MIP_MPSEP` の in-out の点 (LP の頂点でない点) では、その点での行の活動量を使う
        let act = match self.sep_act_override.take() {
            Some(a) if a.len() == mr => a,
            _ => self.lp.row_activities(),
        };
        // カットは木全体で有効にするため、大域的な境界から作る
        let mut lo = self.dom.global_lo.clone();
        let mut up = self.dom.global_up.clone();
        let mut is_int = self.cut_int.clone();
        // 暗黙の整数列の境界も整数に丸める (整数列は丸め済み)
        for j in 0..n {
            if is_int[j] && !p.is_int[j] {
                lo[j] = (lo[j] - FEASTOL).ceil();
                up[j] = (up[j] + FEASTOL).floor();
            }
        }
        let mut xv = x.to_vec();
        for i in 0..mr {
            let (l, u) = self.lp.row_bounds(i);
            let integral = match &row_integral {
                Some(v) => v[i],
                None => lp_rows[i].iter().all(|&(j, a)| self.cut_int[j] && (a - a.round()).abs() <= 1e-9),
            };
            let (l, u) = if integral { ((l - FEASTOL).ceil(), (u + FEASTOL).floor()) } else { (l, u) };
            lo.push(l);
            up.push(u);
            is_int.push(integral);
            xv.push(act[i]);
        }
        if self.vbounds.is_none() {
            let mut vbs = VarBounds::from_rows(n, &p.is_int, &p.rows, &p.row_lo, &p.row_up);
            // 試作 `ENOMOTO_MIP_INTVUB`: 一般整数列と 0-1 列の変数上下限も集める (`ENOMOTO_MIP_INTVUB_IMPL` ならプロービングの含意も)
            if super::cuts::intvub_mode() > 0 || mushagg_mode().is_some() || super::modkvb::modkvb_mode().is_some() {
                let implied: &[(u32, bool, u32, bool, f64)] = if env_str!("ENOMOTO_MIP_INTVUB_IMPL").is_some() { &p.implied_bounds } else { &[] };
                vbs.add_int_vbs(&self.cut_int[..n], &self.dom.global_lo[..n], &self.dom.global_up[..n], &p.rows, &p.row_lo, &p.row_up, implied);
                if env_str!("ENOMOTO_MIP_INTVUB_LOG").is_some() && !self.params.submip {
                    let (nu, nl) = (vbs.ivub.iter().map(|v| v.len()).sum::<usize>(), vbs.ivlb.iter().map(|v| v.len()).sum::<usize>());
                    let ncol = vbs.ivub.iter().zip(&vbs.ivlb).filter(|(a, b)| !a.is_empty() || !b.is_empty()).count();
                    eprintln!("INTVUB: {nu} vub, {nl} vlb on {ncol} integer columns (implied {})", implied.len());
                }
            }
            self.vbounds = Some(std::rc::Rc::new(vbs));
        }
        let vb = self.vbounds.clone();
        // 試作 `ENOMOTO_MIP_IMPLINT_TU2` の `a` ([`Solver::cut_imp`]): TU2 で足した暗に整数の列は連続の印に戻し、`imp` で渡す
        if !self.cut_imp.is_empty() {
            for j in 0..n {
                if self.cut_imp[j] {
                    is_int[j] = false;
                }
            }
        }
        let cut_imp = self.cut_imp.clone();
        let imp = if cut_imp.is_empty() { None } else { Some(&cut_imp[..]) };
        let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &xv, vb: if env_str!("ENOMOTO_MIP_NO_VB").is_some() { None } else { vb.as_deref() }, imp };
        // 試作 `ENOMOTO_MIP_HNFCUT` の `l`: 論理変数 (LP の行の活動量) の合同式を表に足す ([`super::hnfcut::extend_logicals`])
        if super::hnfcut::on() {
            super::hnfcut::extend_logicals(lp_rows, &is_int, &lo, &up, n);
        }
        // 診断用 (`ENOMOTO_MIP_DEBUG_XFILE`): HiGHS の書き出し (`HIGHS_CUT_DUMP`) の LP 解で経路集約だけを試し、カットの数と
        // 効き目を表示する (前処理なし `ENOMOTO_MIP_NO_PRESOLVE` で、列の並びを HiGHS と揃えて使う)
        if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_XFILE") {
            if !self.debug_x_done && !self.params.submip {
                self.debug_x_done = true;
                let xh: Vec<f64> = std::fs::read_to_string(&f).unwrap().lines().filter(|l| l.starts_with("C ")).map(|l| l.split_whitespace().nth(4).unwrap().parse().unwrap()).collect();
                if xh.len() == n {
                    let mut xvh = xh.clone();
                    for row in lp_rows {
                        xvh.push(row.iter().map(|&(j, a)| a * xh[j]).sum());
                    }
                    let vars_h = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &xvh, vb: vb.as_deref(), imp: None };
                    let mut ch: Vec<Candidate> = Vec::new();
                    let mut scratch_h = vec![0.0f64; n];
                    let mut push_h = |raw: Option<RawCut>, cands: &mut Vec<Candidate>, s: &Solver<L>| {
                        if let Some(raw) = raw {
                            if let Some(c) = finish_cut(raw, n, lp_rows, &s.cut_int, &s.dom.global_lo, &s.dom.global_up, &xh, None, &mut scratch_h) {
                                cands.push(c);
                            }
                        }
                    };
                    if env_str!("ENOMOTO_MIP_PATH_OLD").is_none() {
                        self.path_aggregation_highs(&vars_h, lp_rows, &mut ch, &mut push_h);
                    } else {
                        self.path_aggregation(&vars_h, lp_rows, &mut ch, &mut push_h, usize::MAX);
                    }
                    let mut e: Vec<f64> = ch.iter().map(|c| c.efficacy).collect();
                    e.sort_by(|a, b| b.total_cmp(a));
                    let mean = e.iter().sum::<f64>() / e.len().max(1) as f64;
                    eprintln!(
                        "DEBUG_XFILE path cuts {} mean eff {:.4} max {:.4} top10 {:?} >0.1: {} >0.01: {}",
                        e.len(),
                        mean,
                        e.first().copied().unwrap_or(0.0),
                        e.iter().take(10).map(|v| (v * 1e4).round() / 1e4).collect::<Vec<_>>(),
                        e.iter().filter(|&&v| v > 0.1).count(),
                        e.iter().filter(|&&v| v > 0.01).count()
                    );
                } else {
                    eprintln!("DEBUG_XFILE: {} columns in the file, {} in the problem", xh.len(), n);
                }
            }
        }
        let mut cands: Vec<Candidate> = Vec::new();
        // 診断用: finish_cut に使った時間の累計 (秒)
        let t_finish = std::cell::Cell::new(0.0f64);
        let (n_raw, n_fin) = (std::cell::Cell::new(0usize), std::cell::Cell::new(0usize));
        let mut finish_scratch = vec![0.0f64; n];
        // 診断 (`ENOMOTO_MIP_INTVUB_CHECK` とデバッグ解 `ENOMOTO_MIP_DEBUG_SOL`): 分離の候補をすべてデバッグ解で確かめ、切るものを数えて表示する
        // (LP に入れるカットだけを打ち切り値より良いときに確かめる既存の確かめより広い。大域的な境界がデバッグ解を含まなくなった後は
        // 境界の締め付けのせいで切ることもあるので、そのときは別に数える)
        let dbg_cand_check = self.dbg_on && env_str!("ENOMOTO_MIP_INTVUB_CHECK").is_some();
        let dbg_x: Option<Vec<f64>> = if dbg_cand_check { super::solver::DEBUG_SOL.with(|d| d.borrow().clone()) } else { None };
        let dbg_in_bounds = dbg_x.as_ref().is_some_and(|xd| (0..n).all(|j| xd[j] >= self.dom.global_lo[j] - 1e-6 && xd[j] <= self.dom.global_up[j] + 1e-6));
        let mut push = |raw: Option<RawCut>, cands: &mut Vec<Candidate>, s: &Solver<L>| {
            if let Some(raw) = raw {
                let t0 = std::time::Instant::now();
                n_raw.set(n_raw.get() + 1);
                if let Some(c) = finish_cut(raw, n, lp_rows, &s.cut_int, &s.dom.global_lo, &s.dom.global_up, x, s.incumbent.as_ref().map(|(_, v)| v.as_slice()), &mut finish_scratch) {
                    if let Some(xd) = dbg_x.as_ref() {
                        let act: f64 = c.coefs.iter().map(|&(j, a)| a * xd[j]).sum();
                        let viol = act - c.rhs > 1e-6 * (1.0 + c.rhs.abs());
                        let k = if !viol { 0 } else if dbg_in_bounds { 1 } else { 2 };
                        INTVUB_CHECK.with(|v| v.borrow_mut()[k] += 1);
                        if viol && dbg_in_bounds {
                            eprintln!("MIP_DEBUG_SOL: INTVUB_CHECK: candidate cut (src {}) violated by the debug solution: {act:.10e} > {:.10e}, len {}", c.src, c.rhs, c.coefs.len());
                        }
                    }
                    cands.push(c);
                    n_fin.set(n_fin.get() + 1);
                }
                t_finish.set(t_finish.get() + t0.elapsed().as_secs_f64());
            }
        };
        sepprof::lap(sepprof::T_SETUP, &mut lap);
        let sep_t0 = std::time::Instant::now();
        let dbg_sep = env_str!("ENOMOTO_MIP_DEBUG_SEP").is_some();
        // 一時的に加えた行 (目的関数の下界の行など、`Solver::extra_sep_rows`)
        if !self.extra_sep_rows.is_empty() {
            let extra = self.extra_sep_rows.clone();
            for (b, r) in &extra {
                let n0 = cands.len();
                for c in generate_cuts(&vars, b, *r, use_flow_cover(), true) {
                    push(Some(c), &mut cands, self);
                }
                keep_best(&mut cands, n0, self.p);
            }
        }
        tag(&mut cands, sepprof::SRC_EXTRA);
        let mut nsrc0 = cands.len();
        // 元の行 1 本ずつ (診断用 `ENOMOTO_MIP_DEBUG_NO_ROW_CUTS`: 使わない)
        let row_cuts_m = if env_str!("ENOMOTO_MIP_DEBUG_NO_ROW_CUTS").is_some() { 0 } else { p.m };
        // `ENOMOTO_MIP_SEP_ADAPTIVE`: 飛ばしている分離器は使わない ([`SepAdapt`])
        let row_cuts_m = if row_cuts_m > 0 && !self.sep_src_on(sepprof::SRC_ROWS, light) { 0 } else { row_cuts_m };
        // `ENOMOTO_MIP_ORIGROW_TIGHT` (試作、既定では使わない): 元の行 1 本ずつの分離で、行の向きごとに試すものを絞る
        // ([`orig_row_modes`]。0 = 試さない, 1 = 全部 (`generate_cuts`), 2 = extended cover だけ)。既定は毎ラウンド・毎ノード、
        // 余裕と関係なくすべての行の両側を試していた
        let row_modes = if row_cuts_m > 0 && env_str!("ENOMOTO_MIP_ORIGROW_TIGHT").is_some() && !gen_old() && mr >= p.m { self.orig_row_modes(&vars, lp_rows, &act, light) } else { None };
        // `ENOMOTO_MIP_ZH_TRIAL` (試作): 根の試しで使うと決まった問題では使う ([`super::zerohalf::zh_trial_mode`])
        // 診断用 `ENOMOTO_MIP_ZH_ONLY` = `root` / `tree`: zerohalf を使うとき、根 (`light` でない分離) だけ / ノードだけで使う
        let zh_where = match env_str!("ENOMOTO_MIP_ZH_ONLY") {
            Some("root") => !light,
            Some("tree") => light,
            _ => true,
        };
        let zh_on = (env_def!("ENOMOTO_MIP_NO_ZEROHALF", "1").is_none() || super::zerohalf::zh_trial_used(light) || super::cutmix::zh_on()) && zh_where && self.sep_src_on(sepprof::SRC_ZH, light);
        // clique カットの使い方は下の説明を参照
        let clique_on = clique_cuts_mode().is_some_and(|m| {
            let probe = m == "probe" || m == "probe_s" || m == "probe_r";
            let judged = m == "auto" || m == "fallback" || probe;
            !(light && (m == "root" || ((m == "fallback" || probe) && env_str!("ENOMOTO_MIP_CLIQUE_FB_NODES").is_none()))) && (!judged || self.clique_auto == Some(true))
        });
        let clique_on = clique_on && self.sep_src_on(sepprof::SRC_CLIQUE, light);
        // `ENOMOTO_MIP_SEP_PARALLEL` (試作、既定では使わない): 根の分離 (木を分け合う前、サブ MIP でない) で、元の行 1 本ずつ・zerohalf・
        // clique の分離を複数のスレッドで行う ([`sep_parallel_parts`])。行は連続した塊に分けて塊の順に、zerohalf・clique はそれぞれ
        // 1 つの仕事にし、結果を逐次と同じ順につなぐので候補は逐次と同じ (時間切れで止まったときだけ違う)。implied bound の分離は
        // 含意の表を 1 回なめるだけで軽いので逐次のまま。tableau 行 (`gen_cuts_parallel` と同じ形の並列) や経路集約の並列と同じく
        // スレッド数は [`Self::sep_budget_threads`]
        let par_sep = env_str!("ENOMOTO_MIP_SEP_PARALLEL").is_some()
            && self.params.threads > 1
            && !light
            && !self.params.submip
            && self.pool.is_none()
            && !gen_old()
            && env_str!("ENOMOTO_MIP_SEP_SEQ").is_none();
        let mut par_parts: Option<SepParts> = None;
        if par_sep {
            let g = if clique_on { Some(self.clique_graph_get()) } else { None };
            let inc = self.incumbent.as_ref().map(|(_, v)| v.as_slice());
            let parts = sep_parallel_parts(
                &SepParCtx { vars: &vars, p, lp_rows, cut_int: &self.cut_int, glo: &self.dom.global_lo, gup: &self.dom.global_up, x, inc, deadline: self.deadline },
                row_cuts_m,
                row_modes.as_deref(),
                zh_on,
                g.as_deref(),
                sep_threads(self.sep_budget_threads()),
            );
            self.sep_par_cpu += parts.cpu;
            par_parts = Some(parts);
        }
        if let Some(pp) = par_parts.as_mut() {
            cands.append(&mut pp.rows);
            if pp.timed_out || self.time_up() {
                return cands;
            }
        }
        // 診断 (`ENOMOTO_MIP_INTVUB_DUMP=N`): 一般整数列の変数上下限を持つ列を含む行を N 本、置き換え後の項と CMIR の結果を表示する
        if let Some(nd) = env_str!("ENOMOTO_MIP_INTVUB_DUMP").and_then(|v| v.parse::<usize>().ok()) {
            if !self.params.submip && !self.intvub_dumped {
                self.intvub_dumped = true;
                let vbr = vars.vb;
                let mut cnt = 0;
                for i in 0..p.m {
                    let row = &p.rows[i];
                    if row.len() < 3 || !row.iter().any(|&(j, _)| vbr.is_some_and(|v| j < v.ivub.len() && !(v.ivub[j].is_empty() && v.ivlb[j].is_empty()))) {
                        continue;
                    }
                    let neg: Vec<(usize, f64)> = row.iter().map(|&(j, a)| (j, -a)).collect();
                    if p.row_up[i].is_finite() {
                        eprintln!("INTVUB_DUMP {i} up {}", super::cuts::intvub_debug_row(&vars, row, p.row_up[i]));
                    }
                    if p.row_lo[i].is_finite() {
                        eprintln!("INTVUB_DUMP {i} lo {}", super::cuts::intvub_debug_row(&vars, &neg, -p.row_lo[i]));
                    }
                    cnt += 1;
                    if cnt >= nd {
                        break;
                    }
                }
            }
        }
        for i in 0..if par_parts.is_some() { 0 } else { row_cuts_m } {
            if i % 64 == 0 && self.time_up() {
                return cands;
            }
            let row = &p.rows[i];
            if row.len() < 2 {
                continue;
            }
            if !gen_old() {
                let neg: Vec<(usize, f64)> = row.iter().map(|&(j, a)| (j, -a)).collect();
                for (side, (b, r)) in [(row.as_slice(), p.row_up[i]), (neg.as_slice(), -p.row_lo[i])].into_iter().enumerate() {
                    let md = row_modes.as_ref().map_or(1, |rm| rm[i][side]);
                    if r.is_finite() && md != 0 {
                        let n0 = cands.len();
                        if md == 1 {
                            for c in generate_cuts(&vars, b, r, use_flow_cover(), true) {
                                push(Some(c), &mut cands, self);
                            }
                        } else if !has_cont_col(&vars, b) {
                            push(extended_cover(&vars, b, r), &mut cands, self);
                        }
                        keep_best(&mut cands, n0, self.p);
                    }
                }
                continue;
            }
            if p.row_up[i].is_finite() {
                if use_flow_cover() {
                    push(lifted_flow_cover(&vars, row, p.row_up[i]), &mut cands, self);
                }
                if use_lifted_cover() {
                    push(lifted_cover(&vars, row, p.row_up[i]), &mut cands, self);
                }
                let r = cmir(&vars, row, p.row_up[i]);
                push(r, &mut cands, self);
                let r = extended_cover(&vars, row, p.row_up[i]);
                push(r, &mut cands, self);
            }
            if p.row_lo[i].is_finite() {
                let neg: Vec<(usize, f64)> = row.iter().map(|&(j, a)| (j, -a)).collect();
                if use_flow_cover() {
                    push(lifted_flow_cover(&vars, &neg, -p.row_lo[i]), &mut cands, self);
                }
                if use_lifted_cover() {
                    push(lifted_cover(&vars, &neg, -p.row_lo[i]), &mut cands, self);
                }
                let r = cmir(&vars, &neg, -p.row_lo[i]);
                push(r, &mut cands, self);
                let r = extended_cover(&vars, &neg, -p.row_lo[i]);
                push(r, &mut cands, self);
            }
        }
        if dbg_cand_check {
            INTVUB_CHECK.with(|v| {
                let v = v.borrow();
                eprintln!("MIP_DEBUG_SOL: INTVUB_CHECK: candidates checked {} violated {} (debug solution within global bounds) {} (outside)", v[0] + v[1] + v[2], v[1], v[2]);
            });
        }
        if dbg_sep {
            eprintln!("SEP rows {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
            if super::cuts::intvub_mode() > 0 {
                let st: Vec<u64> = super::cuts::INTVUB_STATS.iter().map(|a| a.load(std::sync::atomic::Ordering::Relaxed)).collect();
                eprintln!("INTVUB stats (calls, narrow, at bound, fractional, not tight, substituted) {st:?}");
            }
        }
        tag(&mut cands[nsrc0..], sepprof::SRC_ROWS);
        nsrc0 = cands.len();
        sepprof::lap(sepprof::T_ROWS, &mut lap);
        // zerohalf ({0, 1/2}-CG) カット (元の行から)
        if let Some(pp) = par_parts.as_mut() {
            cands.append(&mut pp.zh);
        } else if zh_on {
            let zhd = if super::zerohalf::zh_deadline_on() { sep_deadline(self.deadline) } else { None };
            let zh = super::zerohalf::with_zh_deadline(zhd, || {
                super::hnfcut::map_rows(&p.rows, &p.row_lo, &p.row_up, &self.cut_int, &self.dom.global_lo, &self.dom.global_up, x, |rw, rl, ru, ii, lo, up, xx| super::zerohalf::zerohalf_cuts(rw, rl, ru, ii, lo, up, xx, 100))
            });
            for raw in zh {
                push(Some(raw), &mut cands, self);
            }
        }
        // 試作 `ENOMOTO_MIP_SNFMODK` (既定では使わない): 余裕 0 の行の群全体からの CG カット ([`super::snfmodk::snfmodk_cg_cuts`])
        if super::snfmodk::snfmodk_has('c') && (!light || super::snfmodk::snfmodk_has('t')) && !self.params.submip {
            let t0 = std::time::Instant::now();
            let mut dl = t0 + std::time::Duration::from_secs_f64(tunable!("ENOMOTO_T_MIP_SNFMODK_TIME", 0.3f64, f64));
            if let Some(d) = sep_deadline(self.deadline) {
                dl = dl.min(d);
            }
            // 試作 `ENOMOTO_MIP_HNFCUT` の `b`: 合同式の列を写した空間で作って戻す ([`super::hnfcut::map_rows`])
            let mut side = None;
            let cs = super::hnfcut::map_rows(&p.rows, &p.row_lo, &p.row_up, &self.cut_int, &self.dom.global_lo, &self.dom.global_up, x, |rw, rl, ru, ii, lo, up, xx| {
                let (cs, gr, st) = super::snfmodk::snfmodk_cg_cuts(rw, rl, ru, ii, lo, up, xx, tunable!("ENOMOTO_T_MIP_SNFMODK_CUTS", 50usize, usize), dl);
                side = Some((gr, st));
                cs
            });
            let (gr, st) = side.expect("snfmodk");
            let c0 = cands.len();
            for raw in cs {
                push(Some(raw), &mut cands, self);
            }
            if dbg_sep || env_str!("ENOMOTO_MIP_SNFMODK_LOG").is_some() {
                eprintln!(
                    "SEP snfmodk {:.3}s rows {} cols {} rank {} unit {} core {}x{} gaveup {} divs {} elems {} cuts_p {} cuts_new {} eff_p {:.4} eff_new {:.4} cands {} single {} new_multi {} eff_new_multi {:.4}",
                    t0.elapsed().as_secs_f64(),
                    st.rows,
                    st.cols,
                    gr.rank,
                    gr.unit_piv,
                    gr.core.0,
                    gr.core.1,
                    gr.gave_up,
                    super::snfmodk::group_summary(&gr),
                    st.elems,
                    st.cuts_p,
                    st.cuts_new,
                    st.best_eff_p,
                    st.best_eff_new,
                    cands.len() - c0,
                    st.cuts_single,
                    st.cuts_new_multi,
                    st.best_eff_new_multi
                );
            }
        }
        if dbg_sep {
            eprintln!("SEP zerohalf {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        tag(&mut cands[nsrc0..], sepprof::SRC_ZH);
        nsrc0 = cands.len();
        sepprof::lap(sepprof::T_ZH, &mut lap);
        // clique カット (2 値列の衝突グラフ。根で一度だけ作って使い回す)。既定では使わない: 40 問で根の下界は
        // ほとんど変わらず、10teams では根の LP 解が変わって根の被約費用ヒューリスティクスが解を見つけられなくなった
        // (13 問 / 幾何平均 30.57 に悪化)。ENOMOTO_MIP_CLIQUE_CUTS=1 で使う。プロービングの含意を辺に加えても (misc07 の辺は
        // 2198 -> 4760、他の問題はほぼ増えない) 40 問で 20 問 28.11 -> 17 問 30.13 (10teams・rout・misc07 が時間切れ)
        // `ENOMOTO_MIP_CLIQUE_CUTS=root` (試作): 根の分離だけで使う (ノードの分離 `light` では使わない)。drayage-25-23 の根の
        // 下界は 1 ラウンド目で 99718.79 -> 100121.99 (HiGHS と同じ。HiGHS も clique と implied bound の分離を止めると根で動かない)
        // `ENOMOTO_MIP_CLIQUE_CUTS=auto` (試作): 根で一度だけ clique カットだけを入れた LP を解き、下界が上がる問題でだけ使う
        // ([`Self::clique_trial`])
        // `ENOMOTO_MIP_CLIQUE_CUTS=fallback` (既定、[`clique_cuts_mode`]): 根のループを clique なしで回し、ほかのカットで伸びず clique カットで伸びる問題でだけ
        // 根の続きのラウンドで使う ([`Self::clique_fallback_trial`])。ノードでは使わない (`ENOMOTO_MIP_CLIQUE_FB_NODES` で使う)
        // (判定は上の `clique_on`)
        if let Some(pp) = par_parts.as_mut() {
            cands.append(&mut pp.clique);
        } else if clique_on {
            let g = self.clique_graph_get();
            for raw in g.separate(x, tunable!("ENOMOTO_T_MIP_CLIQUE_MAX", 100usize, usize)) {
                push(Some(raw), &mut cands, self);
            }
        }
        if dbg_sep {
            eprintln!("SEP clique {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        tag(&mut cands[nsrc0..], sepprof::SRC_CLIQUE);
        nsrc0 = cands.len();
        sepprof::lap(sepprof::T_CLIQUE, &mut lap);
        // implied bound カット (HiGHS の `HighsImplications::separateImpliedBounds`): プロービングの含意
        // `y = v ⇒ x_k <= b` から `x_k <= u_k - (u_k - b) y` (v = 1) / `x_k <= b + (u_k - b) y` (v = 0) (下限も同様)。
        // u_k は今の大域的な上限。LP 解で違反するものだけ
        // 既定では含意の相手が連続変数のものだけを使う (`ENOMOTO_MIP_IMPLBD_CONT`、施設配置の分解した不等式 `x_ij <= d_i y_j`:
        // pg は容量の行 `sum_i x_ij <= C y_j` だけでは緩和が弱く、これで 60 秒で解けない -> 25 秒)。`ENOMOTO_MIP_IMPLBD_CUTS` なら
        // 0-1 変数どうしの含意も使う (全問で使うと 40 問で eil33-2・mik-250 が遅くなり SGM +2.7%)。`ENOMOTO_MIP_NO_IMPLBD_CONT` で無効
        let ib_all = env_str!("ENOMOTO_MIP_IMPLBD_CUTS").is_some();
        // 既定 (連続変数のみ) は根だけ (`light` はノードの分離)。ノードでも作ると木の経過が変わり rout などが揺れる
        let ib_cont = !ib_all && !light && env_str!("ENOMOTO_MIP_NO_IMPLBD_CONT").is_none();
        // `ENOMOTO_MIP_IMPLBD_GENINT` (試作): 含意の相手が一般整数 (0-1 でない整数) の列も使う (HiGHS の
        // `separateImpliedBounds` と同じく 0-1 でない列全部)。mzzv42z (全部整数、一般整数 235 列) は連続変数がないので
        // 既定では 1 本も出ないが、HiGHS の根の分離で LP に入るカットの 8 割がこれで、止めると根の下界が -20829 -> -21209 に
        // 落ちる。`=nodes` ならノードの分離 (`light`) でも使う
        let ib_gen_mode = env_str!("ENOMOTO_MIP_IMPLBD_GENINT");
        // `ENOMOTO_MIP_GR_AUTO` (試作): 一般整数の列があるときだけ根で使う ([`super::implbd_genint_for`])
        let ib_gen_auto = ib_gen_mode.is_none() && !light && super::gr_auto() && super::implbd_genint_for(&p.is_int, &p.col_lo, &p.col_up);
        let ib_gen = !ib_all && ((ib_gen_mode.is_some() && (!light || ib_gen_mode == Some("nodes"))) || ib_gen_auto);
        let ib_cont = ib_cont || ib_gen;
        if (ib_all || ib_cont) && !p.implied_bounds.is_empty() && self.sep_src_on(sepprof::SRC_IMPLBD, light) {
            let (glo, gup) = (&self.dom.global_lo, &self.dom.global_up);
            let max_ib = tunable!("ENOMOTO_T_MIP_IMPLBD_MAX", 5000usize, usize);
            let mut viol: Vec<(f64, RawCut)> = Vec::new();
            for &(y, v, k, upper, b) in &p.implied_bounds {
                let (y, k) = (y as usize, k as usize);
                if ib_cont && p.is_int[k] && !(ib_gen && !(glo[k] >= 0.0 && gup[k] <= 1.0)) {
                    continue;
                }
                if glo[y] == gup[y] || glo[k] == gup[k] || glo[y] != 0.0 || gup[y] != 1.0 {
                    continue;
                }
                // 列を上限側にそろえる: upper なら x_k、下限なら -x_k の上限 (-l_k、含意の値は -b)
                let (sk, bound, imp) = if upper { (1.0, gup[k], b) } else { (-1.0, -glo[k], -b) };
                if !bound.is_finite() || imp >= bound - 1e-9 * (1.0 + bound.abs()) {
                    continue;
                }
                let d = bound - imp;
                // sk x_k + cy y <= rhs
                let (cy, rhs) = if v { (d, bound) } else { (-d, imp) };
                let act = sk * x[k] + cy * x[y];
                let nrm = (1.0 + cy * cy).sqrt();
                if act - rhs > 1e-6 * nrm * (1.0 + rhs.abs()) {
                    let coefs = if y < k { vec![(y, cy), (k, sk)] } else { vec![(k, sk), (y, cy)] };
                    viol.push(((act - rhs) / nrm, RawCut { coefs, rhs }));
                }
            }
            viol.sort_by(|a, b| b.0.total_cmp(&a.0));
            let n0 = cands.len();
            for (_, raw) in viol.into_iter().take(max_ib) {
                push(Some(raw), &mut cands, self);
            }
            // 2 非零で 1 本ずつ別の (y, x) の組を締めるので、根では選択 (質の閾値・平行度) を通さずに全部入れる (HiGHS も
            // implied bound カットはプールに直接入れる)。pg: 選択を通すと根 -9027、通さなければ -8680 (HiGHS -8680) で 60 秒で解けない -> 7.9 秒
            if env_str!("ENOMOTO_MIP_IMPLBD_SELECT").is_none() {
                for c in &mut cands[n0..] {
                    c.forced = true;
                }
            }
            if dbg_sep {
                eprintln!("SEP implied bounds {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
            }
        }
        tag(&mut cands[nsrc0..], sepprof::SRC_IMPLBD);
        nsrc0 = cands.len();
        sepprof::lap(sepprof::T_IMPLBD, &mut lap);
        // 試作 `ENOMOTO_MIP_MUSHAGG` (既定では使わない): 変数上下限を持たない一般整数列を消す 2 本の行の集約 ([`mush_agg_cuts`])。根だけ
        if let Some(mode) = mushagg_mode() {
            if !light && !self.params.submip {
                let t0 = std::time::Instant::now();
                let (groups, npairs, ncols, ntight) = mush_agg_cuts(&vars, p, x, mode);
                let c0 = cands.len();
                for g in groups {
                    let n0 = cands.len();
                    for r in g {
                        push(Some(r), &mut cands, self);
                    }
                    keep_best(&mut cands, n0, self.p);
                }
                if dbg_sep || env_str!("ENOMOTO_MIP_MUSHAGG_LOG").is_some() {
                    eprintln!("SEP mushagg {:.3}s tight {ntight} cols {ncols} pairs {npairs} cands {}", t0.elapsed().as_secs_f64(), cands.len() - c0);
                }
            }
        }
        // 試作 `ENOMOTO_MIP_MODKVB` (既定では使わない): 変数上下限で置き換えた空間の mod-k の行の組 ([`super::modkvb::modkvb_cuts`])。根だけ
        if let Some(mode) = super::modkvb::modkvb_mode() {
            if !light && !self.params.submip && self.sep_src_on(sepprof::SRC_MODK, light) {
                let t0 = std::time::Instant::now();
                let tl = tunable!("ENOMOTO_T_MIP_MODKVB_TIME", 0.3f64, f64);
                let mut dl = t0 + std::time::Duration::from_secs_f64(tl);
                if let Some(d) = sep_deadline(self.deadline) {
                    dl = dl.min(d);
                }
                // 試作 `ENOMOTO_MIP_HNFCUT` の `b`: 合同式の列を写した空間で連立を作って戻す ([`super::hnfcut::map_cutvars`])
                let (groups, st) = match super::hnfcut::map_cutvars(&vars, lp_rows, n) {
                    Some(mv) => {
                        let (gs, st) = super::hnfcut::with_cmir(false, || super::modkvb::modkvb_cuts(&mv.vars(), &mv.rows, n, p.m, mode, use_flow_cover(), dl));
                        (gs.into_iter().map(|g| g.into_iter().filter_map(|c| mv.unmap(c)).collect::<Vec<_>>()).collect::<Vec<_>>(), st)
                    }
                    None => super::modkvb::modkvb_cuts(&vars, lp_rows, n, p.m, mode, use_flow_cover(), dl),
                };
                let c0 = cands.len();
                for g in groups {
                    let n0 = cands.len();
                    for r in g {
                        push(Some(r), &mut cands, self);
                    }
                    keep_best(&mut cands, n0, self.p);
                }
                // `ENOMOTO_T_MIP_MODKVB_MAXCANDS` (既定 0 = 上限なし): 候補を効き目の大きい順にこの数まで残す (physiciansched6-2 では
                // 0 ラウンド目の 450 本で根のカットの LP が反復の上限に当たり、根のカットのループが止まった)
                let maxc = tunable!("ENOMOTO_T_MIP_MODKVB_MAXCANDS", 0usize, usize);
                if maxc > 0 && cands.len() - c0 > maxc {
                    cands[c0..].sort_by(|a, b| b.efficacy.total_cmp(&a.efficacy));
                    cands.truncate(c0 + maxc);
                }
                if dbg_sep || env_str!("ENOMOTO_MIP_MODKVB_LOG").is_some() {
                    eprintln!(
                        "SEP modkvb {:.3}s tight {} sys {} cols {} vbsub {}/{} k {} sols {} solrows {} dense {} timeout {} cands {} snf {}",
                        t0.elapsed().as_secs_f64(),
                        st.tight,
                        st.sys,
                        st.cols,
                        st.vbsub.0,
                        st.vbsub.1,
                        st.k,
                        st.sols,
                        st.sol_rows,
                        st.dense,
                        st.timed_out as u8,
                        cands.len() - c0,
                        st.snf
                    );
                }
            }
        }
        tag(&mut cands[nsrc0..], sepprof::SRC_MODK);
        nsrc0 = cands.len();
        if super::seplearn::stat_on() {
            let now = std::time::Instant::now();
            super::seplearn::add_src_time(sepprof::SRC_MODK, (now - lap).as_secs_f64());
            lap = now;
        }
        // 経路集約 (path aggregation、HiGHS の `HighsPathSeparator`)
        if env_str!("ENOMOTO_MIP_NO_PATH_AGG").is_none() {
            // 固定費用ネットワーク型ではノードでも経路集約の始点を使う (`ENOMOTO_T_MIP_NODE_FC_PATH_STARTS`、[`Solver::node_cuts_due`])
            let max_starts = if !light {
                tunable!("ENOMOTO_T_MIP_PATH_STARTS", 1000usize, usize)
            } else if self.fc_node_sep || (self.tree_sep_fc && !(self.nc_path_forced && super::solver::nodecnt('r') && !self.nc_root_decide())) {
                tunable!("ENOMOTO_T_MIP_NODE_FC_PATH_STARTS", 1000usize, usize)
            } else {
                tunable!("ENOMOTO_T_MIP_NODE_PATH_STARTS", 0usize, usize)
            };
            // 既定は HiGHS の経路集約の移植 (`ENOMOTO_MIP_PATH_OLD` で従来の経路集約)
            // 経路集約: 従来のもの (`ENOMOTO_MIP_PATH_OLD`)、HiGHS の移植 (`ENOMOTO_MIP_PATH_HIGHS_ONLY`)、既定は両方
            // (h80x6320d・rout は移植で根の下界が上がり、mik-250 は従来のものの方が強い)
            // `ENOMOTO_MIP_FC_NODE_PATH_HIGHS`: 固定費用ネットワーク型の毎ノードの分離では、ノードでも HiGHS の移植を使う
            // 試作 `ENOMOTO_MIP_NODECNT` の `h`: 木の中の分離 (`tree_sep_fc`) でもノードで HiGHS の移植を使う
            let node_highs = light && ((self.fc_node_sep && env_str!("ENOMOTO_MIP_FC_NODE_PATH_HIGHS").is_some()) || (self.tree_sep_fc && super::solver::nodecnt('h') && !(self.nc_path_forced && super::solver::nodecnt('r') && self.nc_rdec != 1)));
            let old_only = env_str!("ENOMOTO_MIP_PATH_OLD").is_some() || (light && !node_highs);
            let highs_only = env_str!("ENOMOTO_MIP_PATH_HIGHS_ONLY").is_some();
            if !old_only && self.sep_src_on(sepprof::SRC_PATHH, light) {
                self.path_aggregation_highs(&vars, lp_rows, &mut cands, &mut push);
            }
            tag(&mut cands[nsrc0..], sepprof::SRC_PATHH);
            nsrc0 = cands.len();
            sepprof::lap(sepprof::T_PATHH, &mut lap);
            if (!highs_only || old_only) && self.sep_src_on(sepprof::SRC_PATH, light) {
                // 根で並列の分離なら、集約した行を順に集めてからカットの生成と仕上げを並列に行う (集約は逐次と同じ順で、
                // カットの有無で経路を変えないので結果は逐次と同じ。`ENOMOTO_MIP_PATH_STOP_ON_CUT` では逐次)
                let par_path = self.params.threads > 1
                    && !light
                    && !self.params.submip
                    && self.pool.is_none()
                    && !gen_old()
                    && env_str!("ENOMOTO_MIP_PATH_STOP_ON_CUT").is_none()
                    && env_str!("ENOMOTO_MIP_SEP_SEQ").is_none()
                    && env_str!("ENOMOTO_MIP_PATH_SEQ").is_none();
                if par_path {
                    self.path_collect = Some(Vec::new());
                    self.path_aggregation(&vars, lp_rows, &mut cands, &mut push, max_starts);
                    let bases = self.path_collect.take().unwrap_or_default();
                    let inc = self.incumbent.as_ref().map(|(_, v)| v.as_slice());
                    let (by_row, cpu) = gen_cuts_parallel(&bases, sep_threads(self.sep_budget_threads()), &vars, use_flow_cover(), n, lp_rows, &self.cut_int, &self.dom.global_lo, &self.dom.global_up, x, inc, self.p, sep_deadline(self.deadline));
                    self.sep_par_cpu += cpu;
                    for cs in by_row {
                        cands.extend(cs);
                    }
                } else {
                    self.path_aggregation(&vars, lp_rows, &mut cands, &mut push, max_starts);
                }
            }
        }
        if dbg_sep {
            eprintln!("SEP path {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        tag(&mut cands[nsrc0..], sepprof::SRC_PATH);
        nsrc0 = cands.len();
        sepprof::lap(sepprof::T_PATH, &mut lap);
        // 多品種フローの cut-set の集約 (SCIP の `sepa_mcf.c` の移植、[`super::mcf`])。既定は `auto` (2026-10-29、analysis/mergeall_20261028.md)。
        // `ENOMOTO_MIP_SEPA_MCF=0` (`off`) で使わない。`ENOMOTO_MIP_SEPA_MCF=1` (`root`) で根だけ、
        // `nodes` でノードの分離でも使う。`auto` (`auto-nodes`) は固定費用ネットワーク型 ([`fixed_charge_network`]) でない問題でだけ使う
        // (VUB の行が大半の 1 品種の網 mc11・n3700 系などでは、1 節点の cut-set は経路集約と flow cover のカットとほぼ同じで、
        // 根の下界は揺れの範囲でしか変わらない。analysis/mcf_cuts_20261028.md)。網は最初に呼ばれたときに元の行から一度だけ探す
        let mcf_mode = Some(env_str!("ENOMOTO_MIP_SEPA_MCF").unwrap_or("auto")).filter(|&m| m != "0" && m != "off");
        let mcf_on = mcf_mode.is_some_and(|m| !light || m.ends_with("nodes"))
            && (!mcf_mode.is_some_and(|m| m.starts_with("auto")) || !self.mcf_fc_network())
            && !self.params.submip
            && mr >= p.m;
        if mcf_on && self.sep_src_on(sepprof::SRC_MCF, light) {
            let nets = self.mcf_networks();
            if !nets.is_empty() {
                let t0 = std::time::Instant::now();
                let rlo = &lo[n..];
                let rup = &up[n..];
                let duals = self.lp.row_duals();
                let max_aggs = tunable!("ENOMOTO_T_MIP_MCF_MAX_AGGS", 400usize, usize);
                let single = env_str!("ENOMOTO_MIP_MCF_NO_SINGLE").is_none();
                let aggs = super::mcf::aggregations(&nets, lp_rows, &act, rlo, rup, &duals, tunable!("ENOMOTO_T_MIP_MCF_CLUSTERS", 5usize, usize), max_aggs, single);
                let max_cuts = tunable!("ENOMOTO_T_MIP_MCF_MAX_CUTS", 200usize, usize);
                let mut agg = vec![0.0f64; n];
                let mut in_agg = vec![false; n];
                let mut touched: Vec<usize> = Vec::new();
                let n_start = cands.len();
                for wts in &aggs {
                    if cands.len() - n_start >= max_cuts || self.time_up() {
                        break;
                    }
                    for &j in &touched {
                        agg[j] = 0.0;
                        in_agg[j] = false;
                    }
                    touched.clear();
                    for &(i, w) in wts {
                        for &(j, a) in &lp_rows[i] {
                            if !in_agg[j] {
                                in_agg[j] = true;
                                touched.push(j);
                            }
                            agg[j] += w * a;
                        }
                    }
                    touched.sort_unstable();
                    let amax = touched.iter().fold(0.0f64, |mx, &j| mx.max(agg[j].abs()));
                    let mut base: Vec<(usize, f64)> = touched.iter().filter(|&&j| agg[j].abs() > 1e-9 * amax.max(1.0)).map(|&j| (j, agg[j])).collect();
                    let mut slk: Vec<(usize, f64)> = wts.iter().map(|&(i, w)| (n + i, -w)).collect();
                    slk.sort_unstable_by_key(|&(k, _)| k);
                    base.extend(slk);
                    let n0 = cands.len();
                    for c in generate_cuts(&vars, &base, 0.0, use_flow_cover(), true) {
                        push(Some(c), &mut cands, self);
                    }
                    keep_best(&mut cands, n0, self.p);
                }
                if env_str!("ENOMOTO_MIP_DEBUG_MCF").is_some() {
                    eprintln!("MCF sep: {} aggregations, {} cands, {:.3}s", aggs.len(), cands.len() - n_start, t0.elapsed().as_secs_f64());
                }
            }
        }
        tag(&mut cands[nsrc0..], sepprof::SRC_MCF);
        nsrc0 = cands.len();
        sepprof::lap(sepprof::T_MCF, &mut lap);
        // tableau 行 (診断用 `ENOMOTO_MIP_DEBUG_NO_TAB_CUTS`: 使わない)
        let mut basics: Vec<(usize, f64)> = Vec::new();
        let tab_mr = if env_str!("ENOMOTO_MIP_DEBUG_NO_TAB_CUTS").is_some() { 0 } else { mr };
        for s in 0..tab_mr {
            let k = self.lp.basic_var(s);
            if k < n && self.cut_int[k] {
                let v = x[k];
                let f = v - v.floor();
                if f > 1e-3 && f < 1.0 - 1e-3 {
                    basics.push((s, f * (1.0 - f) / self.lp.dse_weight(s).max(1e-8)));
                }
            }
        }
        let nint = p.nint;
        // 根の tableau 行の数の上限: `ENOMOTO_T_MIP_TAB_ROWS_BASE` (200) + `ENOMOTO_T_MIP_TAB_ROWS_FRAC` (0.1) x min(行, 整数列)
        // (試作で変えられるようにしたもの。mzzv42z では 1200 行から候補 1295 本を作り、選択で採るのは 27 本)
        let limit = if light {
            tunable!("ENOMOTO_T_MIP_NODE_TAB_ROWS", 50usize, usize)
        } else {
            tunable!("ENOMOTO_T_MIP_TAB_ROWS_BASE", 200usize, usize) + (tunable!("ENOMOTO_T_MIP_TAB_ROWS_FRAC", 0.1, f64) * (mr.min(nint)) as f64) as usize
        };
        // `ENOMOTO_MIP_TAB_HIGHS_RULES`: HiGHS の HighsTableauSeparator と同じ行の選び方。`dse_weight` は 1 を返すだけなので
        // 既定のスコアは f(1-f) で、密な行 (基底逆行列の行の非零が多い) が不利にならない。HiGHS は
        // (1) 基底逆行列の行 w を先に全部計算し、スコア = f(1-f) / ||w||^2 (w_i は行 i の最大係数で拡大縮小) で並べる、
        // (2) w の最大/最小の比が 1e4 を超える行は使わない、(3) 集約で増える非零 (集約行の長さ - w の非零) の 10 倍が
        // 10000 + 列数 を超える行は使わない (fill の予算)、(4) 最初にカットが出た行のスコアを基準に、その 0.0025 倍
        // (カットが 50 本を超えたら 0.01 倍) を下回ったら止める。30n20b8 では集約行の平均非零が 4400-5000 で、200 行の
        // カット生成 + finish_cut に 1 ラウンド 0.45 秒かかっていた (HiGHS は tableau のカットを 1 本も作らず 0.002-0.012 秒)。
        // 実測 (根のループ全体の時間 / 最後の下界): neos-1456979 0.58 -> 0.38 秒 (154 のまま)、qnet1 0.54 -> 0.33 秒
        // (15654 -> 15745)、neos-911970 0.42 -> 0.28 秒 (47.26 のまま)、h80x6320d 2.00 -> 1.84 秒 (5936 -> 5926) だが、
        // 30n20b8 は 2.41 -> 2.08 秒で下界 151 -> 88.5 (fill の予算なしでも 102: 停止規則が密なカットを切り、こちらは
        // mod-k などの代わりに tableau 行の密なカットで下界を得ている)。既定では使わない
        // 試作 `ENOMOTO_MIP_GAP4` の `t` (既定では使わない。analysis/gap4mem_20261111.md): LP の元の行が `ENOMOTO_T_MIP_GAP4_TAB_M` (20000) 以上の
        // 問題でだけ HiGHS の選び方を使う。neos-873061 (9.3 万行) では既定の密な tableau カット (1 本 約 4300 非零) で 1 ラウンドの LP が重く、
        // HiGHS の選び方で根の下界が 104.7 → 109.5・111.4 に上がる。害の出た 30n20b8・roll3000・nw04 はどれも 2300 行以下
        let tab_rules = env_str!("ENOMOTO_MIP_TAB_HIGHS_RULES").is_some() || (super::solver::gap4('t') && p.m >= tunable!("ENOMOTO_T_MIP_GAP4_TAB_M", 20000usize, usize));
        let mut w_cache: Vec<Vec<f64>> = Vec::new();
        if tab_rules {
            // 行の最大係数 (LP 行、スラック列を含まない)
            let row_max: Vec<f64> = lp_rows.iter().map(|r| r.iter().fold(0.0f64, |m, &(_, a)| m.max(a.abs()))).collect();
            let mut scored: Vec<(usize, f64, Vec<f64>)> = Vec::new();
            for &(s, _) in &basics {
                let w = self.lp.basis_inverse_row(s);
                let (mut norm2, mut wmin, mut wmax, mut cnt) = (0.0f64, f64::INFINITY, 0.0f64, 0usize);
                for i in 0..mr {
                    let sw = row_max[i] * w[i].abs();
                    if sw <= 1e-6 {
                        continue;
                    }
                    wmin = wmin.min(sw);
                    wmax = wmax.max(sw);
                    norm2 += sw * sw;
                    cnt += 1;
                }
                if cnt <= 1 || wmax / wmin > 1e4 {
                    continue;
                }
                let k = self.lp.basic_var(s);
                let f = x[k] - x[k].floor();
                scored.push((s, f * (1.0 - f) / norm2, w));
            }
            scored.sort_by(|a, b| b.1.total_cmp(&a.1));
            scored.truncate(limit);
            basics = scored.iter().map(|&(s, sc, _)| (s, sc)).collect();
            w_cache = scored.into_iter().map(|(_, _, w)| w).collect();
        } else {
            basics.sort_by(|a, b| b.1.total_cmp(&a.1));
        }
        // `ENOMOTO_T_MIP_TAB_NNZ_BUDGET` = B (>0 で有効): 1 ラウンドの集約の手間 (w の非零の行の長さの合計) の上限を
        // B * (列数 + LP の非零) にする。BTRAN の直後に手間が分かるので、上限を超える (密な) 行は集約せずに飛ばし、
        // 後ろの疎な行で埋める (候補は limit の 3 倍まで見て、使う行は limit まで)。既定では使わない (0):
        // 30n20b8 は w の非零が平均 5 しかなく、LP の行そのものが密 (集約行 4400-7700 非零) で、根の最初のラウンドの
        // 下界 43 -> 81 は密な行からの 1 本のカットによる。予算をかけると分離は 0.3 -> 0.1 秒/ラウンドになるが、
        // 根の下界が 151 -> 92 (B=2), 109 (B=5), 105 (B=20) に下がる。neos-911970 は B=2,5 で根のラウンドが減り
        // 木の下界 53.1 -> 51 に悪化、qnet1 は 4.5 -> 3.0 秒と速くなり、neos-1456979・h80x6320d はほぼ変わらない
        let lp_nnz: usize = lp_rows.iter().map(|r| r.len()).sum();
        let nnz_budget_mult = tunable!("ENOMOTO_T_MIP_TAB_NNZ_BUDGET", 0.0, f64);
        let nnz_budget = if nnz_budget_mult > 0.0 { nnz_budget_mult * (n + lp_nnz) as f64 } else { f64::INFINITY };
        let mut work_used = 0.0f64;
        let mut n_budget_skip = 0usize;
        // `ENOMOTO_MIP_TAB_NNZ_CBC` (既定で使う、2026-10-30、analysis/cbcimpl_20261029.md。`0` か `off` で使わない): CglGomory (CglGomory.cpp の `nReasonable`) と同じく、行をスコアの順に試し、
        // 作ったカットの非零の数の合計が予算に届いたら止める (届いた行のカットは入れる。密な行を飛ばして後ろの行で埋めることはしない)。
        // 予算は根で max(5n, nnz/8 + min(10000, nnz + 2m)) (最初のラウンドは min を取らず 10000、さらに + nnz/2)、ノード (`light`) で
        // max(2n, nnz/8 + 200) (n は列、m は LP の行、nnz は LP の非零)。`ENOMOTO_T_MIP_TAB_NNZ_BUDGET`・`ENOMOTO_MIP_TAB_HIGHS_RULES`
        // と一緒には使わない。以前の似た試み (その 2 つ) は 30n20b8 の根の下界を下げた。
        // 値が `root` なら根だけ、`node` ならノードだけに予算をかける (切り分け用)
        let tab_cbc_mode = Some(env_str!("ENOMOTO_MIP_TAB_NNZ_CBC").unwrap_or("1")).filter(|&m| m != "0" && m != "off");
        let tab_cbc_on = match tab_cbc_mode {
            None => false,
            Some("root") => !light,
            Some("node") => light,
            Some(_) => true,
        };
        let mut tab_cbc_left: Option<i64> = if tab_cbc_on && !tab_rules && !nnz_budget.is_finite() {
            let (nz, nn, mm) = (lp_nnz as i64, n as i64, mr as i64);
            Some(if light {
                (2 * nn).max(nz / 8 + 200)
            } else {
                let first = self.sep_pass == 0;
                let add = if first { 10_000 } else { 10_000i64.min(nz + 2 * mm) };
                (5 * nn).max(nz / 8 + add) + if first { nz / 2 } else { 0 }
            })
        } else {
            None
        };
        // 根の予算の効きすぎ (mc11・ns1208400 で解けなくなる) を抑える (0 で無効):
        // - `ENOMOTO_T_MIP_TAB_NNZ_GAIN` = tol (既定 1e-4、2026-10-30): 根で直前のラウンドの目的値の相対的な伸びが tol を超える (最初のラウンドを含む)
        //   あいだは予算をかけない。下界が止まってから予算をかける。58 問・Mittelmann で既定 (予算なし) とほぼ同じ (SGM +1.4%・−0.2%、
        //   解けた数 ±0・+1)。予算だけ (0) では mc11・ns1208400 が解けなくなる
        // - `ENOMOTO_T_MIP_TAB_NNZ_LATE` = R (試作、既定 0): 根のループの R ラウンド目からは予算をかけない (mc11 は後半のラウンドで候補が 3〜5 割減り、下界が止まる)
        // - `ENOMOTO_T_MIP_TAB_NNZ_CUTCAP` = k (試作、既定 0): カット 1 本が予算から引く非零を 予算 / k までにする (ns1208400 は 1 本約 1500 非零で、
        //   予算で 1 ラウンド約 13 行しか使えない)
        if !light && tab_cbc_left.is_some() {
            let late = tunable!("ENOMOTO_T_MIP_TAB_NNZ_LATE", 0usize, usize);
            let gain_tol = tunable!("ENOMOTO_T_MIP_TAB_NNZ_GAIN", 1e-4, f64);
            if (late > 0 && self.sep_pass >= late) || (gain_tol > 0.0 && self.sep_last_gain > gain_tol) {
                tab_cbc_left = None;
            }
        }
        let tab_cut_cap: i64 = {
            let k = tunable!("ENOMOTO_T_MIP_TAB_NNZ_CUTCAP", 0.0, f64);
            match tab_cbc_left {
                Some(b) if k > 0.0 => ((b as f64 / k).ceil() as i64).max(1),
                _ => i64::MAX,
            }
        };
        let tab_cbc_init = tab_cbc_left;
        if !tab_rules {
            basics.truncate(if nnz_budget.is_finite() { 3 * limit } else { limit });
        }
        // `ENOMOTO_MIP_TAB_BUDGET` (試作): HiGHS の `HighsTableauSeparator` と同じく、試す tableau 行の数の累計に上限
        // 5000 + 50 x (呼び出し回数) + (LP の反復の累計) / 10 を設け、超えたら tableau の分離をしない (1 回に試す行は
        // 残りの予算と上の limit の小さい方)。mzzv42z では毎ラウンド 1200 行を試していた (HiGHS は数ラウンドで予算を使い切る)。
        // また基底逆行列の行の非零が 1 つの行 (元の行そのもの、行ごとの分離と同じ) は飛ばす
        let tab_budget = env_str!("ENOMOTO_MIP_TAB_BUDGET").is_some() && !self.params.submip;
        if tab_budget {
            let max_tries = 5000 + 50 * self.tab_calls as i64 + (self.lp.total_iterations() / 10) as i64 - self.tab_tries as i64;
            self.tab_calls += 1;
            let keep = if max_tries <= 0 { 0 } else { (max_tries as usize).min(basics.len()) };
            basics.truncate(keep);
            self.tab_tries += basics.len().min(limit) as u64;
        }
        sepprof::lap(sepprof::T_TABSEL, &mut lap);
        let mut agg = vec![0.0; n];
        let tab_fc = use_flow_cover() && env_str!("ENOMOTO_MIP_TAB_FC").is_some();
        // 診断 (`ENOMOTO_MIP_DEBUG_SEP`): tableau 行の内訳の時間 (BTRAN, 集約, カット生成 + finish_cut) と行の非零数
        let (mut t_btran, mut t_agg, mut t_gen) = (0.0f64, 0.0f64, 0.0f64);
        let (mut n_rows_used, mut w_nnz_total, mut base_nnz_total, mut n_fill_skip) = (0usize, 0usize, 0usize, 0usize);
        let cands_before_tab = cands.len();
        // 試作 `ENOMOTO_MIP_BRCUT`: 行 (元の基底の列) ごとの候補のカットの効き目の最大 ([`super::brcut::record`])
        let brcut_on = super::brcut::on() && !self.params.submip;
        let mut brcut_round: Vec<(usize, f64)> = Vec::new();
        let mut best_score = -1.0f64;
        // 根の分離 (木を分け合う前、サブ MIP でない) で既定の行の選び方なら、tableau 行のカットの生成と仕上げ (分離の時間の
        // 大半: 30n20b8・blp-ar98・graph20-20-1rand で 8 割) を複数のスレッドで行う (`ENOMOTO_MIP_SEP_SEQ` で逐次)。
        // 行の選択と集約は逐次で今までと同じ順に行い、行ごとの候補 (keep_best 後) を同じ順につなぐので結果は逐次と同じ
        let par_threads = sep_threads(self.sep_budget_threads());
        let par = par_threads > 1
            && !light
            && !self.params.submip
            && self.pool.is_none()
            && !gen_old()
            && !tab_rules
            && !nnz_budget.is_finite()
            && env_str!("ENOMOTO_MIP_SEP_SEQ").is_none();
        // 試作 `ENOMOTO_MIP_REDSPLIT` ([`super::redsplit`]、既定では使わない。根だけ): `a` = k-cuts (δ = 1/k の CMIR も候補に足す)、
        // `b` = reduce-and-split (分離に使った行を集めておき、下で縮約してからカットを作る)
        let (rs_a, rs_b) = if light || self.params.submip { (false, false) } else { super::redsplit::redsplit_mode() };
        super::hnfcut::set_kind('c');
        let hnfcut_st = super::hnfcut::snapshot();
        let rs_deltas = super::redsplit::deltas(rs_a);
        let mut rs_bases: Vec<Vec<(usize, f64)>> = Vec::new();
        if par {
            let mut bases: Vec<Vec<(usize, f64)>> = Vec::new();
            let mut brcut_src: Vec<usize> = Vec::new();
            for &(s, _) in basics.iter() {
                if self.time_up() || n_rows_used >= limit {
                    break;
                }
                let w = self.lp.basis_inverse_row(s);
                let wmax = w.iter().fold(0.0f64, |m, v| m.max(v.abs()));
                if wmax == 0.0 {
                    continue;
                }
                let wmin_keep = 1e-9 * wmax;
                let mut touched: Vec<usize> = Vec::new();
                let mut base: Vec<(usize, f64)> = Vec::new();
                let mut wnz_min = f64::INFINITY;
                for i in 0..mr {
                    let wi = w[i];
                    if wi.abs() <= wmin_keep {
                        continue;
                    }
                    wnz_min = wnz_min.min(wi.abs());
                    for &(j, a) in &lp_rows[i] {
                        if agg[j] == 0.0 {
                            touched.push(j);
                        }
                        agg[j] += wi * a;
                        if agg[j] == 0.0 {
                            agg[j] = 1e-300;
                        }
                    }
                    base.push((n + i, -wi));
                }
                let ok = wmax / wnz_min <= 1e6 && !(tab_budget && base.len() <= 1);
                for &j in &touched {
                    let v = agg[j];
                    if v.abs() > 1e-11 && ok {
                        base.push((j, v));
                    }
                    agg[j] = 0.0;
                }
                if !ok {
                    continue;
                }
                // 最適面の頂点での分離の照合 (逐次の経路と同じ)
                if self.tab_memo_mode != 0 {
                    use std::hash::{Hash, Hasher};
                    let q = |v: f64| -> i64 { (v * 1e9).round() as i64 };
                    let mut hs = std::collections::hash_map::DefaultHasher::new();
                    for &(j, a) in &base {
                        j.hash(&mut hs);
                        q(a).hash(&mut hs);
                        q(vars.x[j]).hash(&mut hs);
                    }
                    let h = hs.finish();
                    let k = self.lp.basic_var(s);
                    if self.tab_memo_mode == 1 {
                        self.tab_memo.insert(k, h);
                    } else {
                        self.tab_memo_stats.0 += 1;
                        if self.tab_memo.get(&k) == Some(&h) {
                            self.tab_memo_stats.1 += 1;
                            if env_str!("ENOMOTO_MIP_FACE_TAB_SKIP").is_some() {
                                continue;
                            }
                        }
                    }
                }
                n_rows_used += 1;
                base_nnz_total += base.len();
                bases.push(base);
                if brcut_on {
                    brcut_src.push(self.lp.basic_var(s));
                }
            }
            sepprof::lap(sepprof::T_TABAGG, &mut lap);
            let t0 = std::time::Instant::now();
            let (cut_int, glo, gup) = (&self.cut_int[..], &self.dom.global_lo[..], &self.dom.global_up[..]);
            let inc = self.incumbent.as_ref().map(|(_, v)| v.as_slice());
            let (pp, lp_rows_ref, vars_ref, bases_ref) = (self.p, lp_rows, &vars, &bases);
            let rs_deltas_ref = &rs_deltas;
            let hnfcut_ref = &hnfcut_st;
            let nb = bases.len();
            let nth = par_threads.min(nb.max(1));
            let deadline = sep_deadline(self.deadline);
            // 行をスレッドに交互に割り当てる (行はスコア順で、重さが偏らないように)
            let parts: Vec<(Vec<(usize, Vec<Candidate>)>, f64)> = std::thread::scope(|sc| {
                let hs: Vec<_> = (0..nth)
                    .map(|t| {
                        sc.spawn(move || {
                            let _hc = super::hnfcut::Guard::new(hnfcut_ref.clone());
                            let c0 = super::cap_clock();
                            let mut scratch = vec![0.0f64; n];
                            let mut out = Vec::new();
                            let mut k = t;
                            // 求解の期限 ([`sep_deadline`]) を過ぎたら残りの行は作らない (逐次の形は行ごとに `time_up` を見る)
                            while k < nb && !deadline.is_some_and(|d| std::time::Instant::now() >= d) {
                                let base = &bases_ref[k];
                                let neg: Vec<(usize, f64)> = base.iter().map(|&(j, a)| (j, -a)).collect();
                                let mut cs: Vec<Candidate> = Vec::new();
                                for b in [base, &neg] {
                                    let n0 = cs.len();
                                    for raw in generate_cuts(vars_ref, b, 0.0, tab_fc, false) {
                                        if let Some(c) = finish_cut(raw, n, lp_rows_ref, cut_int, glo, gup, x, inc, &mut scratch) {
                                            cs.push(c);
                                        }
                                    }
                                    if rs_a {
                                        if let Some(raw) = super::cuts::cmir_with_deltas(vars_ref, b, 0.0, rs_deltas_ref) {
                                            if let Some(c) = finish_cut(raw, n, lp_rows_ref, cut_int, glo, gup, x, inc, &mut scratch) {
                                                cs.push(c);
                                            }
                                        }
                                    }
                                    keep_best(&mut cs, n0, pp);
                                }
                                out.push((k, cs));
                                k += nth;
                            }
                            (out, super::cap_clock() - c0)
                        })
                    })
                    .collect();
                hs.into_iter().map(|h| h.join().unwrap()).collect()
            });
            let mut by_row: Vec<Vec<Candidate>> = (0..nb).map(|_| Vec::new()).collect();
            for (part, cpu) in parts {
                self.sep_par_cpu += cpu;
                for (k, cs) in part {
                    by_row[k] = cs;
                }
            }
            if brcut_on {
                for (k, cs) in by_row.iter().enumerate() {
                    brcut_round.push((brcut_src[k], cs.iter().fold(0.0f64, |m, c| m.max(c.efficacy))));
                }
            }
            for cs in by_row {
                // `ENOMOTO_MIP_TAB_NNZ_CBC`: 逐次と同じく、予算に届いたらそれより後ろの行のカットは入れない
                if let Some(left) = tab_cbc_left.as_mut() {
                    if *left <= 0 {
                        break;
                    }
                    *left -= cs.iter().map(|c| (c.coefs.len() as i64).min(tab_cut_cap)).sum::<i64>();
                }
                cands.extend(cs);
            }
            if dbg_sep {
                t_gen += t0.elapsed().as_secs_f64();
            }
            if rs_b {
                rs_bases = bases;
            }
        } else {
            for (bi, &(s, score)) in basics.iter().enumerate() {
                if self.time_up() {
                    break;
                }
                if tab_rules && best_score >= 0.0 {
                    let made = cands.len() - cands_before_tab;
                    let fac = if made >= 50 { 0.01 } else { 0.0025 };
                    if score < fac * best_score {
                        break;
                    }
                }
                if n_rows_used >= limit {
                    break;
                }
                if tab_cbc_left.is_some_and(|left| left <= 0) {
                    break;
                }
                let t0 = std::time::Instant::now();
                let w = if tab_rules { std::mem::take(&mut w_cache[bi]) } else { self.lp.basis_inverse_row(s) };
                if dbg_sep {
                    t_btran += t0.elapsed().as_secs_f64();
                }
                let t0 = std::time::Instant::now();
                let wmax = w.iter().fold(0.0f64, |m, v| m.max(v.abs()));
                if wmax == 0.0 {
                    continue;
                }
                let wmin_keep = 1e-9 * wmax;
                if nnz_budget.is_finite() {
                    let work: usize = (0..mr).filter(|&i| w[i].abs() > wmin_keep).map(|i| lp_rows[i].len() + 1).sum();
                    if work_used + work as f64 > nnz_budget {
                        n_budget_skip += 1;
                        continue;
                    }
                    work_used += work as f64;
                }
                // 集約: sum_i w_i (a_i x - r_i) = 0
                let mut touched: Vec<usize> = Vec::new();
                let mut base: Vec<(usize, f64)> = Vec::new();
                let mut ok = true;
                let mut wnz_min = f64::INFINITY;
                for i in 0..mr {
                    let wi = w[i];
                    if wi.abs() <= wmin_keep {
                        continue;
                    }
                    wnz_min = wnz_min.min(wi.abs());
                    for &(j, a) in &lp_rows[i] {
                        if agg[j] == 0.0 {
                            touched.push(j);
                        }
                        agg[j] += wi * a;
                        if agg[j] == 0.0 {
                            agg[j] = 1e-300;
                        }
                    }
                    base.push((n + i, -wi));
                }
                // 重みの比が大きすぎる集約は数値的に危ないので使わない
                if wmax / wnz_min > 1e6 || (tab_budget && base.len() <= 1) {
                    ok = false;
                }
                for &j in &touched {
                    let v = agg[j];
                    if v.abs() > 1e-11 && ok {
                        base.push((j, v));
                    }
                    agg[j] = 0.0;
                }
                let w_nnz = w.iter().filter(|v| v.abs() > wmin_keep).count();
                if dbg_sep {
                    t_agg += t0.elapsed().as_secs_f64();
                    w_nnz_total += w_nnz;
                }
                if !ok {
                    continue;
                }
                // HiGHS の fill の予算: 集約で増えた非零の 10 倍が 10000 + 列数 を超える行は使わない
                // (`ENOMOTO_T_MIP_TAB_FILL_MULT` 倍に緩める。0 なら予算なし。30n20b8 では HiGHS と同じ予算だと全ての行が外れ、
                // 根の下界が 151 -> 88.5 に下がった: HiGHS は mod-k など他の分離で下界を得るが、こちらは tableau 行に頼っている)
                let fill_mult = tunable!("ENOMOTO_T_MIP_TAB_FILL_MULT", 1.0, f64);
                if tab_rules && fill_mult > 0.0 && 10.0 * base.len().saturating_sub(w_nnz) as f64 > fill_mult * (10_000 + n) as f64 {
                    n_fill_skip += 1;
                    continue;
                }
                // 最適面の頂点での分離: 今の頂点で同じ基底変数の行が、集約した行もその列の LP 値も同じなら、作るカットも
                // 同じなので飛ばす (`ENOMOTO_MIP_FACE_TAB_SKIP`。照合と数えるのは常に)。同じ行の割合は misc07 で 25% (1323 行中 325)、
                // 10teams ではほぼ 0 (2088 行中 2: tableau が密で、1 回のピボットでほぼ全ての行が変わる)。分離の時間はほとんど
                // 変わらないので既定では飛ばさない
                if self.tab_memo_mode != 0 {
                    use std::hash::{Hash, Hasher};
                    let q = |v: f64| -> i64 { (v * 1e9).round() as i64 };
                    let mut hs = std::collections::hash_map::DefaultHasher::new();
                    for &(j, a) in &base {
                        j.hash(&mut hs);
                        q(a).hash(&mut hs);
                        q(vars.x[j]).hash(&mut hs);
                    }
                    let h = hs.finish();
                    let k = self.lp.basic_var(s);
                    if self.tab_memo_mode == 1 {
                        self.tab_memo.insert(k, h);
                    } else {
                        self.tab_memo_stats.0 += 1;
                        if self.tab_memo.get(&k) == Some(&h) {
                            self.tab_memo_stats.1 += 1;
                            if env_str!("ENOMOTO_MIP_FACE_TAB_SKIP").is_some() {
                                continue;
                            }
                        }
                    }
                }
                n_rows_used += 1;
                base_nnz_total += base.len();
                if rs_b {
                    rs_bases.push(base.clone());
                }
                sepprof::lap(sepprof::T_TABAGG, &mut lap);
                if !gen_old() {
                    let t0 = std::time::Instant::now();
                    let n_before = cands.len();
                    let neg: Vec<(usize, f64)> = base.iter().map(|&(k, a)| (k, -a)).collect();
                    // 診断 (`ENOMOTO_MIP_INTVUB_DUMP=N` で `ENOMOTO_MIP_INTVUB_DUMP_TAB`): 最初の N 本の tableau の集約行を表示する
                    if env_str!("ENOMOTO_MIP_INTVUB_DUMP_TAB").is_some() && !self.params.submip {
                        let nd = env_str!("ENOMOTO_MIP_INTVUB_DUMP").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
                        if n_rows_used <= nd {
                            for (si, b) in [&base, &neg].into_iter().enumerate() {
                                eprintln!("INTVUB_DUMP_TAB {n_rows_used}.{si} {}", super::cuts::intvub_debug_row(&vars, b, 0.0));
                            }
                        }
                    }
                    for b in [&base, &neg] {
                        let n0 = cands.len();
                        for c in generate_cuts(&vars, b, 0.0, tab_fc, false) {
                            push(Some(c), &mut cands, self);
                        }
                        if rs_a {
                            push(super::cuts::cmir_with_deltas(&vars, b, 0.0, &rs_deltas), &mut cands, self);
                        }
                        keep_best(&mut cands, n0, self.p);
                    }
                    if brcut_on {
                        brcut_round.push((self.lp.basic_var(s), cands[n_before..].iter().fold(0.0f64, |m, c| m.max(c.efficacy))));
                    }
                    if let Some(left) = tab_cbc_left.as_mut() {
                        *left -= cands[n_before..].iter().map(|c| (c.coefs.len() as i64).min(tab_cut_cap)).sum::<i64>();
                    }
                    if dbg_sep {
                        t_gen += t0.elapsed().as_secs_f64();
                    }
                    sepprof::lap(sepprof::T_TABGEN, &mut lap);
                    if tab_rules && best_score < 0.0 && cands.len() > n_before {
                        best_score = score;
                    }
                    continue;
                }
                let r = cmir(&vars, &base, 0.0);
                push(r, &mut cands, self);
                let neg: Vec<(usize, f64)> = base.iter().map(|&(k, a)| (k, -a)).collect();
                let r = cmir(&vars, &neg, 0.0);
                push(r, &mut cands, self);
                if use_lifted_cover() {
                    push(lifted_cover(&vars, &base, 0.0), &mut cands, self);
                    push(lifted_cover(&vars, &neg, 0.0), &mut cands, self);
                }
                if tab_fc {
                    push(lifted_flow_cover(&vars, &base, 0.0), &mut cands, self);
                    push(lifted_flow_cover(&vars, &neg, 0.0), &mut cands, self);
                }
            }
        }
        // 試作 `ENOMOTO_MIP_REDSPLIT` の `b`: 集めた行を縮約し、短くなった行から δ の候補 ([`super::redsplit::deltas`]) で CMIR を作る。
        // 改良 (11-08、どれも既定では効かない値): `ENOMOTO_T_MIP_REDSPLIT_ROUNDS` = R (> 0) なら根のループの最初の R ラウンドだけ、
        // `ENOMOTO_T_MIP_REDSPLIT_TFRAC` = r (> 0) なら縮約の時間の累計が求解の経過時間の r 倍を超えたら止める、
        // `ENOMOTO_T_MIP_REDSPLIT_RATIO` = q (< 1) なら連続の部分のノルムの 2 乗が q 倍以下に縮んだ行だけ使う、
        // `ENOMOTO_T_MIP_REDSPLIT_DENS` = d (> 0) なら元の行のカットの非零の d 倍を超えるカットは捨てる、
        // 値に `c` があれば元の行のカット (同じ δ の候補) より効き目が大きいカットだけ使う
        let rs_rounds = tunable!("ENOMOTO_T_MIP_REDSPLIT_ROUNDS", 0usize, usize);
        let rs_tfrac = tunable!("ENOMOTO_T_MIP_REDSPLIT_TFRAC", 0.0f64, f64);
        let rs_go = rs_b
            && !rs_bases.is_empty()
            && (rs_rounds == 0 || self.sep_pass < rs_rounds)
            && (rs_tfrac <= 0.0 || super::redsplit::time_used(self.start) <= rs_tfrac * self.start.elapsed().as_secs_f64());
        if rs_go {
            let t0 = std::time::Instant::now();
            let tl = tunable!("ENOMOTO_T_MIP_REDSPLIT_TIME", 0.5f64, f64);
            let dl = t0 + std::time::Duration::from_secs_f64(tl);
            let dl = match sep_deadline(self.deadline) {
                Some(d) if d < dl => d,
                _ => dl,
            };
            let ratio_max = tunable!("ENOMOTO_T_MIP_REDSPLIT_RATIO", 1.0f64, f64);
            let dens = tunable!("ENOMOTO_T_MIP_REDSPLIT_DENS", 0.0f64, f64);
            let cmp_orig = super::redsplit::compare_mode();
            let rs_log = dbg_sep || env_str!("ENOMOTO_MIP_REDSPLIT_LOG").is_some();
            let (rows, st) = super::redsplit::reduce(&rs_bases, &is_int, n, Some(dl));
            let c0 = cands.len();
            let (mut n_ratio_skip, mut n_dens_skip, mut n_cmp_skip) = (0usize, 0usize, 0usize);
            let mut tmp: Vec<Candidate> = Vec::new();
            for (idx, ratio, row) in &rows {
                // 縮約とカットの生成を合わせて 1 回 `ENOMOTO_T_MIP_REDSPLIT_TIME` 秒まで (blp-ar98 で密な行のカットの生成が重かった)
                if self.time_up() || std::time::Instant::now() >= dl {
                    break;
                }
                if *ratio > ratio_max {
                    n_ratio_skip += 1;
                    continue;
                }
                tmp.clear();
                let neg: Vec<(usize, f64)> = row.iter().map(|&(k, a)| (k, -a)).collect();
                for b in [row, &neg] {
                    let n0 = tmp.len();
                    push(super::cuts::cmir_with_deltas(&vars, b, 0.0, &rs_deltas), &mut tmp, self);
                    keep_best(&mut tmp, n0, self.p);
                }
                if tmp.is_empty() {
                    continue;
                }
                // 元の行のカット (効き目の最大と非零の数の最大)
                if cmp_orig || dens > 0.0 {
                    let mut orig: Vec<Candidate> = Vec::new();
                    let base = &rs_bases[*idx];
                    let negb: Vec<(usize, f64)> = base.iter().map(|&(k, a)| (k, -a)).collect();
                    for b in [base, &negb] {
                        let n0 = orig.len();
                        push(super::cuts::cmir_with_deltas(&vars, b, 0.0, &rs_deltas), &mut orig, self);
                        keep_best(&mut orig, n0, self.p);
                    }
                    let e_o = orig.iter().fold(0.0f64, |m, c| m.max(c.efficacy));
                    let nnz_o = orig.iter().map(|c| c.coefs.len()).max();
                    tmp.retain(|c| {
                        if cmp_orig && c.efficacy <= e_o * (1.0 + 1e-3) {
                            n_cmp_skip += 1;
                            return false;
                        }
                        if dens > 0.0 && nnz_o.is_some_and(|k| c.coefs.len() as f64 > dens * k.max(1) as f64) {
                            n_dens_skip += 1;
                            return false;
                        }
                        true
                    });
                }
                if rs_log {
                    for c in &tmp {
                        super::redsplit::remember(super::redsplit::fingerprint(&c.coefs, c.rhs));
                    }
                }
                cands.append(&mut tmp);
            }
            let dt = t0.elapsed().as_secs_f64();
            super::redsplit::add_time(self.start, dt);
            if rs_log {
                let effs: Vec<f64> = cands[c0..].iter().map(|c| c.efficacy).collect();
                let emax = effs.iter().fold(0.0f64, |m, &v| m.max(v));
                let nnz = cands[c0..].iter().map(|c| c.coefs.len()).sum::<usize>() as f64 / effs.len().max(1) as f64;
                eprintln!(
                    "REDSPLIT pass {} rows {} (cont {}) reduced {} (mean norm2 ratio {:.3}) rejected {} skipped ratio/dens/cmp {}/{}/{} cuts {} (max eff {:.4}, mean nnz {:.0}) {:.3}s",
                    self.sep_pass,
                    rs_bases.len(),
                    st.rows,
                    st.reduced,
                    st.ratio_sum / st.reduced.max(1) as f64,
                    st.rejected,
                    n_ratio_skip,
                    n_dens_skip,
                    n_cmp_skip,
                    effs.len(),
                    emax,
                    nnz,
                    dt
                );
            }
        }
        if dbg_sep && tab_cbc_init.is_some() {
            eprintln!("SEP tableau CBC budget {:?} left {:?} light {} cands {}", tab_cbc_init, tab_cbc_left, light, cands.len() - cands_before_tab);
        }
        if dbg_sep {
            eprintln!(
                "SEP tableau {:.3}s cands {} (rows {} of {} candidates, fill-skipped {}, budget-skipped {} (work {:.0} / {:.0}), btran {:.3}s agg {:.3}s gen+finish {:.3}s (finish total this call {:.3}s), mean w nnz {:.0}, mean agg nnz {:.0}, LP rows {} nnz {})",
                sep_t0.elapsed().as_secs_f64(),
                cands.len(),
                n_rows_used,
                basics.len(),
                n_fill_skip,
                n_budget_skip,
                work_used,
                nnz_budget,
                t_btran,
                t_agg,
                t_gen,
                t_finish.get(),
                w_nnz_total as f64 / basics.len().max(1) as f64,
                base_nnz_total as f64 / n_rows_used.max(1) as f64,
                mr,
                lp_rows.iter().map(|r| r.len()).sum::<usize>()
            );
        }
        if brcut_on {
            super::brcut::record(&brcut_round);
        }
        super::hnfcut::set_kind('a');
        // 並列の経路では tableau 行の生成は 1 回にまとまる (逐次の経路では行ごとに足し済みで、ここは端数)
        sepprof::lap(sepprof::T_TABGEN, &mut lap);
        tag(&mut cands[nsrc0..], sepprof::SRC_TAB);
        if sepprof::on() {
            sepprof::add(sepprof::T_FINISH + fidx * sepprof::NSEP, t_finish.get());
            sepprof::PROF.with(|pp| {
                let mut pp = pp.borrow_mut();
                pp.raw[fidx] += n_raw.get();
                pp.fin[fidx] += n_fin.get();
            });
        }
        cands
    }
}

/// 集約行 `base` に連続列 (境界が幅を持つもの) があるか ([`generate_cuts`] が extended cover を試さない条件)
/// 試作 `ENOMOTO_MIP_MUSHAGG` の動き方 (既定では `None` = 使わない)。`cover` = [`super::cuts::vb_cover_cut`] だけ、`gen` = 集約行に
/// 一般整数列の変数上下限の置き換えを使った CMIR など([`generate_cuts`]、置き換えのモードは `ENOMOTO_MIP_MUSHAGG_IV`、既定 2) だけ、
/// ほかの値 (`1` など) は両方
pub(crate) fn mushagg_mode() -> Option<u8> {
    static M: std::sync::OnceLock<Option<u8>> = std::sync::OnceLock::new();
    *M.get_or_init(|| match env_def!("ENOMOTO_MIP_MUSHAGG", "cover") {
        None | Some("0") => None,
        Some("cover") => Some(1),
        Some("gen") => Some(2),
        Some(_) => Some(3),
    })
}

/// 試作 `ENOMOTO_MIP_MUSHAGG` (HiGHS の `HighsModkSeparator` が mushroom-best で作るカットの形を真似たもの): LP 解で等号になっている
/// 元の行 2 本を、変数上下限を持たない一般整数列 (LP 値が上下限の内側) の係数が打ち消し合う重みで足し、その列を消した行から
/// カットを作る。HiGHS は mod-k の連立 (GF(k) の消去) で「LP 値が 0 でない整数列の係数が k で割り切れる」行の組を探し、その集約行を
/// `HighsCutGeneration::generateCut` (変数上下限の置き換え + lifted cover / CMIR) に渡す。mushroom-best ではその組のほとんどが
/// 「x0 の符号が逆のデータの行 2 本 (+ LP 値が 0 でない w を消す w-z の 2 項の行)」で、できるカットは `sum_S z >= 1` の形
/// (`analysis/mushroom_cuts_20261106.md`)。ここでは mod-k を解かずに、消す列ごとに符号が逆の行の組を直接作る。
/// - 消す列: 整数 (0-1 でない)、LP 値が上下限から feastol より内側、一般整数列の変数上下限を持たない。等号の行に多く出る順に
///   `ENOMOTO_T_MIP_MUSHAGG_COLS` (既定 4) 本。
/// - 行の組: 消す列の係数が正の行と負の行 (それぞれ `ENOMOTO_T_MIP_MUSHAGG_ROWS`、既定 200 本まで) の組のうち、足した行の非零の数
///   (共通の列を引いた概算) が小さい順に全体で `ENOMOTO_T_MIP_MUSHAGG_PAIRS` (既定 3000) 組。
/// - 集約行 `row_p / |a_pj| + row_q / |a_qj|` (列 j は 0 にして除く) から、[`super::cuts::vb_cover_cut`] と (モード 2・3)
///   一般整数列の変数上下限の置き換えを強制した [`generate_cuts`] でカットを作る。組ごとに返す。
/// 返り値は (組ごとのカット, 試した組の数, 消す列の数, 等号の行の数)
#[allow(clippy::too_many_arguments)]
pub(crate) fn mush_agg_cuts(vars: &CutVars, p: &MipProblem, x: &[f64], mode: u8) -> (Vec<Vec<RawCut>>, usize, usize, usize) {
    let n = p.n;
    let Some(vb) = vars.vb else { return (Vec::new(), 0, 0, 0) };
    let ft = FEASTOL;
    let has_ivb = |j: usize| j < vb.ivub.len() && !(vb.ivub[j].is_empty() && vb.ivlb[j].is_empty());
    // 消してよい列
    let elim = |j: usize| -> bool { j < n && vars.is_int[j] && vars.up[j] - vars.lo[j] > 1.5 && x[j] - vars.lo[j] > ft && vars.up[j] - x[j] > ft && !has_ivb(j) };
    // 等号の行の向き: (行, 符号) 符号 +1 は `row <= up`、-1 は `-row <= -lo`
    let mut tight: Vec<(usize, f64)> = Vec::new();
    for i in 0..p.m {
        let r = &p.rows[i];
        if r.len() < 2 {
            continue;
        }
        let a: f64 = r.iter().map(|&(j, c)| c * x[j]).sum();
        if p.row_up[i].is_finite() && p.row_up[i] - a <= 1e-6 * (1.0 + p.row_up[i].abs()) {
            tight.push((i, 1.0));
        } else if p.row_lo[i].is_finite() && a - p.row_lo[i] <= 1e-6 * (1.0 + p.row_lo[i].abs()) {
            tight.push((i, -1.0));
        }
    }
    // 列ごとの (等号の行の番号 in tight, 向きを付けた係数)
    let mut occ: std::collections::HashMap<usize, (Vec<(usize, f64)>, Vec<(usize, f64)>)> = std::collections::HashMap::new();
    for (t, &(i, sg)) in tight.iter().enumerate() {
        for &(j, c) in &p.rows[i] {
            if c.abs() > 1e-9 && elim(j) {
                let e = occ.entry(j).or_default();
                if sg * c > 0.0 {
                    e.0.push((t, sg * c));
                } else {
                    e.1.push((t, sg * c));
                }
            }
        }
    }
    let mut cols: Vec<(usize, usize)> = occ.iter().filter(|(_, v)| !v.0.is_empty() && !v.1.is_empty()).map(|(&j, v)| (v.0.len().min(v.1.len()), j)).collect();
    cols.sort_by(|a, b| b.cmp(a));
    cols.truncate(tunable!("ENOMOTO_T_MIP_MUSHAGG_COLS", 4usize, usize));
    let max_rows = tunable!("ENOMOTO_T_MIP_MUSHAGG_ROWS", 200usize, usize);
    let max_pairs = tunable!("ENOMOTO_T_MIP_MUSHAGG_PAIRS", 3000usize, usize);
    // 組の候補 (非零の概算, 消す列, 正の側, 負の側)
    let mut pairs: Vec<(usize, usize, (usize, f64), (usize, f64))> = Vec::new();
    let mut mark = vec![false; n];
    for &(_, j) in &cols {
        let (pos, neg) = &occ[&j];
        for &pp in pos.iter().take(max_rows) {
            let rp = &p.rows[tight[pp.0].0];
            for &(k, _) in rp {
                mark[k] = true;
            }
            for &qq in neg.iter().take(max_rows) {
                let rq = &p.rows[tight[qq.0].0];
                let common = rq.iter().filter(|&&(k, _)| mark[k]).count();
                pairs.push((rp.len() + rq.len() - 2 * common, j, pp, qq));
            }
            for &(k, _) in rp {
                mark[k] = false;
            }
        }
    }
    pairs.sort_by_key(|t| t.0);
    pairs.truncate(max_pairs);
    let iv = env_str!("ENOMOTO_MIP_MUSHAGG_IV").and_then(|v| v.parse::<u8>().ok()).unwrap_or(2);
    let mut out = Vec::new();
    let mut acc: Vec<f64> = vec![0.0; n];
    let mut touched: Vec<usize> = Vec::new();
    for &(_, j, (tp, ap), (tq, aq)) in &pairs {
        let (ip, sp) = tight[tp];
        let (iq, sq) = tight[tq];
        // 向きを付けた行 sp * row_ip <= rhs_p を |ap| で割る (ap は向きを付けた係数で > 0、aq < 0)
        let (wp, wq) = (sp / ap, sq / -aq);
        let rhs_p = if sp > 0.0 { p.row_up[ip] } else { p.row_lo[ip] };
        let rhs_q = if sq > 0.0 { p.row_up[iq] } else { p.row_lo[iq] };
        let rhs = wp * rhs_p + wq * rhs_q;
        for &(i, w) in &[(ip, wp), (iq, wq)] {
            for &(k, c) in &p.rows[i] {
                if acc[k] == 0.0 {
                    touched.push(k);
                }
                acc[k] += w * c;
                if acc[k] == 0.0 {
                    acc[k] = 1e-300;
                }
            }
        }
        let mut base: Vec<(usize, f64)> = Vec::with_capacity(touched.len());
        for &k in &touched {
            let v = acc[k];
            if k != j && v.abs() > 1e-9 {
                base.push((k, v));
            }
            acc[k] = 0.0;
        }
        touched.clear();
        if base.is_empty() {
            continue;
        }
        let mut cs = Vec::new();
        if mode & 1 != 0 {
            if let Some(c) = super::cuts::vb_cover_cut(vars, &base, rhs) {
                cs.push(c);
            }
        }
        if mode & 2 != 0 {
            cs.extend(super::cuts::with_intvub_mode(iv, || generate_cuts(vars, &base, rhs, use_flow_cover(), true)));
        }
        if !cs.is_empty() {
            out.push(cs);
        }
    }
    (out, pairs.len(), cols.len(), tight.len())
}

fn has_cont_col(vars: &CutVars, base: &[(usize, f64)]) -> bool {
    base.iter().any(|&(k, a)| a != 0.0 && !vars.is_int[k] && vars.lo[k] < vars.up[k])
}

/// [`sep_parallel_parts`] に渡す分離の入力
struct SepParCtx<'a> {
    vars: &'a CutVars<'a>,
    p: &'a MipProblem,
    lp_rows: &'a [Vec<(usize, f64)>],
    cut_int: &'a [bool],
    glo: &'a [f64],
    gup: &'a [f64],
    x: &'a [f64],
    inc: Option<&'a [f64]>,
    deadline: Option<std::time::Instant>,
}

/// [`sep_parallel_parts`] の結果 (分離器ごとの候補 (逐次と同じ順)、時間切れで止まったか、スレッドの CPU 時間の合計)
struct SepParts {
    rows: Vec<Candidate>,
    zh: Vec<Candidate>,
    clique: Vec<Candidate>,
    timed_out: bool,
    cpu: f64,
}

/// `ENOMOTO_MIP_SEP_PARALLEL`: 元の行 1 本ずつ (最初の `row_m` 行、向きごとの `modes` は [`Solver::orig_row_modes`])・zerohalf
/// (`zh_on`)・clique (`clique`) の分離を `nth` 本のスレッドで行う。仕事は zerohalf・clique・行の連続した塊 (スレッド数の 8 倍まで)
/// で、空いたスレッドが次の仕事を取り、結果は仕事の順に並べるので逐次 ([`Solver::separate`]) と同じ候補になる
fn sep_parallel_parts(c: &SepParCtx, row_m: usize, modes: Option<&[[u8; 2]]>, zh_on: bool, clique: Option<&super::clique::CliqueGraph>, nth: usize) -> SepParts {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    let p = c.p;
    let n = p.n;
    let nchunks = if row_m == 0 { 0 } else { (8 * nth).min(row_m) };
    let chunk = if nchunks == 0 { 0 } else { row_m.div_ceil(nchunks) };
    let ntask = 2 + nchunks;
    let next = AtomicUsize::new(0);
    let timed = AtomicBool::new(false);
    let nth = nth.clamp(1, ntask);
    let hnfcut_st = super::hnfcut::snapshot();
    let parts: Vec<(Vec<(usize, Vec<Candidate>)>, f64)> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..nth)
            .map(|_| {
                sc.spawn(|| {
                    let _hc = super::hnfcut::Guard::new(hnfcut_st.clone());
                    let c0 = super::cap_clock();
                    let mut scratch = vec![0.0f64; n];
                    let mut out: Vec<(usize, Vec<Candidate>)> = Vec::new();
                    loop {
                        let t = next.fetch_add(1, Ordering::Relaxed);
                        if t >= ntask {
                            break;
                        }
                        let mut cs: Vec<Candidate> = Vec::new();
                        let mut fin = |raw: RawCut, cs: &mut Vec<Candidate>| {
                            if let Some(cd) = finish_cut(raw, n, c.lp_rows, c.cut_int, c.glo, c.gup, c.x, c.inc, &mut scratch) {
                                cs.push(cd);
                            }
                        };
                        match t {
                            0 => {
                                if zh_on {
                                    let zhd = if super::zerohalf::zh_deadline_on() { c.deadline } else { None };
                                    for raw in super::zerohalf::with_zh_deadline(zhd, || {
                                        super::hnfcut::map_rows(&p.rows, &p.row_lo, &p.row_up, c.cut_int, c.glo, c.gup, c.x, |rw, rl, ru, ii, lo, up, xx| super::zerohalf::zerohalf_cuts(rw, rl, ru, ii, lo, up, xx, 100))
                                    }) {
                                        fin(raw, &mut cs);
                                    }
                                }
                            }
                            1 => {
                                if let Some(g) = clique {
                                    for raw in g.separate(c.x, tunable!("ENOMOTO_T_MIP_CLIQUE_MAX", 100usize, usize)) {
                                        fin(raw, &mut cs);
                                    }
                                }
                            }
                            _ => {
                                let k = t - 2;
                                for i in k * chunk..((k + 1) * chunk).min(row_m) {
                                    if i % 64 == 0 && (timed.load(Ordering::Relaxed) || c.deadline.is_some_and(|d| std::time::Instant::now() >= d)) {
                                        timed.store(true, Ordering::Relaxed);
                                        break;
                                    }
                                    let row = &p.rows[i];
                                    if row.len() < 2 {
                                        continue;
                                    }
                                    let neg: Vec<(usize, f64)> = row.iter().map(|&(j, a)| (j, -a)).collect();
                                    for (side, (b, r)) in [(row.as_slice(), p.row_up[i]), (neg.as_slice(), -p.row_lo[i])].into_iter().enumerate() {
                                        let md = modes.map_or(1, |rm| rm[i][side]);
                                        if r.is_finite() && md != 0 {
                                            let n0 = cs.len();
                                            if md == 1 {
                                                for raw in generate_cuts(c.vars, b, r, use_flow_cover(), true) {
                                                    fin(raw, &mut cs);
                                                }
                                            } else if !has_cont_col(c.vars, b) {
                                                if let Some(raw) = extended_cover(c.vars, b, r) {
                                                    fin(raw, &mut cs);
                                                }
                                            }
                                            keep_best(&mut cs, n0, p);
                                        }
                                    }
                                }
                            }
                        }
                        out.push((t, cs));
                    }
                    (out, super::cap_clock() - c0)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut by_task: Vec<Vec<Candidate>> = (0..ntask).map(|_| Vec::new()).collect();
    let mut cpu = 0.0;
    for (part, t) in parts {
        cpu += t;
        for (k, cs) in part {
            by_task[k] = cs;
        }
    }
    let mut it = by_task.into_iter();
    let zh = it.next().unwrap_or_default();
    let clique = it.next().unwrap_or_default();
    let rows: Vec<Candidate> = it.flatten().collect();
    SepParts { rows, zh, clique, timed_out: timed.load(Ordering::Relaxed), cpu }
}

/// [`Solver::path_aggregation_highs`] の許容誤差
const PATHH_FEASTOL: f64 = 1e-6;

/// [`Solver::path_aggregation_highs`] の、列 `j` の LP 値の境界 (変数上下限を含む) からの距離 (許容誤差以下は 0)
fn pathh_bound_dist(vars: &CutVars, j: usize) -> f64 {
    let feastol = PATHH_FEASTOL;
    let xj = vars.x[j];
    let mut blo = vars.lo[j];
    let mut bup = vars.up[j];
    if let Some(vb) = vars.vb {
        for &(y, a, e) in &vb.vub[j] {
            bup = bup.min(a * vars.x[y] + e);
        }
        for &(y, a, e) in &vb.vlb[j] {
            blo = blo.max(a * vars.x[y] + e);
        }
    }
    let mut dl = xj - blo;
    let mut du = bup - xj;
    if dl <= feastol {
        dl = 0.0;
    }
    if du <= feastol {
        du = 0.0;
    }
    dl.min(du)
}

/// [`Solver::path_aggregation_highs`] の行の型 (0 使わない, 1 `<=`, -1 `>=`, 2 等式。代入用の等式は 0)、連続変数か
/// (境界から離れているもの)、連続変数ごとの行、代入用の等式 (連続変数ごとの (行, 係数))。型が 0 でない行が経路の始点になる
#[allow(clippy::type_complexity)]
fn pathh_row_types(vars: &CutVars, lp_rows: &[Vec<(usize, f64)>], n: usize) -> (Vec<i8>, Vec<bool>, Vec<Vec<(usize, f64)>>, Vec<Option<(usize, f64)>>) {
    let m = lp_rows.len();
    let feastol = PATHH_FEASTOL;
    let bound_dist = |j: usize| -> f64 { pathh_bound_dist(vars, j) };
    // 行の型: 0 使わない, 1 <=, -1 >=, 2 等式
    let mut rtype = vec![0i8; m];
    for i in 0..m {
        let (l, u) = (vars.lo[n + i], vars.up[n + i]);
        if l == u {
            rtype[i] = 2;
            continue;
        }
        let r = vars.x[n + i];
        let ls = if l.is_finite() { r - l } else { f64::INFINITY };
        let us = if u.is_finite() { u - r } else { f64::INFINITY };
        rtype[i] = if ls > feastol && us > feastol {
            0
        } else if ls < us {
            -1
        } else {
            1
        };
    }
    let cont: Vec<bool> = (0..n).map(|j| !vars.is_int[j] && bound_dist(j) > 0.0).collect();
    let mut num_cont = vec![0usize; m];
    let mut col_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for i in 0..m {
        for &(j, a) in &lp_rows[i] {
            if cont[j] {
                num_cont[i] += 1;
                col_rows[j].push((i, a));
            }
        }
    }
    // 代入用の等式 (連続変数が 1 つだけ)
    let mut subst: Vec<Option<(usize, f64)>> = vec![None; n];
    for i in 0..m {
        if rtype[i] != 2 || num_cont[i] != 1 {
            continue;
        }
        let Some(&(j, a)) = lp_rows[i].iter().find(|&&(j, _)| cont[j]) else { continue };
        if subst[j].is_some() {
            continue;
        }
        subst[j] = Some((i, a));
        rtype[i] = 0;
    }
    (rtype, cont, col_rows, subst)
}

/// 分離を期限で打ち切るときの期限 (求解の期限 `deadline` そのもの)。`ENOMOTO_MIP_NO_SEP_DEADLINE` なら `None` (切り分け用、直す前と同じ)。
/// 期限を過ぎた後だけ効くので、時間内の分離は変わらない
fn sep_deadline(deadline: Option<std::time::Instant>) -> Option<std::time::Instant> {
    if env_str!("ENOMOTO_MIP_NO_SEP_DEADLINE").is_some() {
        None
    } else {
        deadline
    }
}

/// 集約した行 `bases` ごとに (その行と符号を反転した行から) カットを作って仕上げ、行ごとに keep_best で絞った候補を
/// 行の順に返す (`nth` 本のスレッドに交互に割り当てる。結果は逐次と同じ)。2 つ目の戻り値はスレッドの CPU 時間の合計。
#[allow(clippy::too_many_arguments)]
fn gen_cuts_parallel(
    bases: &[Vec<(usize, f64)>],
    nth: usize,
    vars: &CutVars,
    flow_cover: bool,
    n: usize,
    lp_rows: &[Vec<(usize, f64)>],
    cut_int: &[bool],
    glo: &[f64],
    gup: &[f64],
    x: &[f64],
    inc: Option<&[f64]>,
    p: &MipProblem,
    deadline: Option<std::time::Instant>,
) -> (Vec<Vec<Candidate>>, f64) {
    let nb = bases.len();
    let nth = nth.min(nb.max(1));
    let hnfcut_st = super::hnfcut::snapshot();
    let hnfcut_ref = &hnfcut_st;
    let parts: Vec<(Vec<(usize, Vec<Candidate>)>, f64)> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..nth)
            .map(|t| {
                sc.spawn(move || {
                    let _hc = super::hnfcut::Guard::new(hnfcut_ref.clone());
                    let c0 = super::cap_clock();
                    let mut scratch = vec![0.0f64; n];
                    let mut out = Vec::new();
                    let mut k = t;
                    // 求解の期限 ([`sep_deadline`]) を過ぎたら残りの行は作らない
                    while k < nb && !deadline.is_some_and(|d| std::time::Instant::now() >= d) {
                        let base = &bases[k];
                        let neg: Vec<(usize, f64)> = base.iter().map(|&(j, a)| (j, -a)).collect();
                        let mut cs: Vec<Candidate> = Vec::new();
                        for b in [base, &neg] {
                            let n0 = cs.len();
                            for raw in generate_cuts(vars, b, 0.0, flow_cover, false) {
                                if let Some(c) = finish_cut(raw, n, lp_rows, cut_int, glo, gup, x, inc, &mut scratch) {
                                    cs.push(c);
                                }
                            }
                            keep_best(&mut cs, n0, p);
                        }
                        out.push((k, cs));
                        k += nth;
                    }
                    (out, super::cap_clock() - c0)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut by_row: Vec<Vec<Candidate>> = (0..nb).map(|_| Vec::new()).collect();
    let mut cpu = 0.0;
    for (part, c) in parts {
        cpu += c;
        for (k, cs) in part {
            by_row[k] = cs;
        }
    }
    (by_row, cpu)
}

/// 生のカット (構造変数と行の活動量の変数) を構造変数だけの式にし、整え、検査する。
/// 試作 `ENOMOTO_MIP_GAP4` の `l`: [`finish_cut`] で長さの上限まで短くしたカットの数 (全スレッド)
pub(super) static GAP4_CUT_SHORTENED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn finish_cut(
    raw: RawCut,
    n: usize,
    lp_rows: &[Vec<(usize, f64)>],
    is_int: &[bool],
    lo: &[f64],
    up: &[f64],
    x: &[f64],
    incumbent: Option<&[f64]>,
    scratch: &mut [f64],
) -> Option<Candidate> {
    // 試作 `ENOMOTO_MIP_CMIRDELTA` の診断: 新しい δ で作った CMIR か
    let cmirdelta_new = super::cmirdelta::on() && super::cmirdelta::is_new_raw(&raw.coefs, raw.rhs);
    // 構造変数ごとの係数を密な作業領域 (呼び出し側が長さ n の 0 で用意し、ここで 0 に戻す) に集める。
    // 以前は BTreeMap で、集約行の非零が数千のときは finish_cut が tableau 行のカット生成と同程度の時間を使っていた
    let mut touched: Vec<usize> = Vec::with_capacity(raw.coefs.len());
    for &(k, c) in &raw.coefs {
        if k < n {
            if scratch[k] == 0.0 {
                touched.push(k);
            }
            scratch[k] += c;
            if scratch[k] == 0.0 {
                scratch[k] = 1e-300;
            }
        } else {
            for &(j, a) in &lp_rows[k - n] {
                if scratch[j] == 0.0 {
                    touched.push(j);
                }
                scratch[j] += c * a;
                if scratch[j] == 0.0 {
                    scratch[j] = 1e-300;
                }
            }
        }
    }
    touched.sort_unstable();
    let mut dense: Vec<(usize, f64)> = Vec::with_capacity(touched.len());
    for &j in &touched {
        let v = scratch[j];
        scratch[j] = 0.0;
        dense.push((j, if v == 1e-300 { 0.0 } else { v }));
    }
    let mut rhs = raw.rhs;
    let cmax = dense.iter().fold(0.0f64, |m, &(_, v)| m.max(v.abs()));
    if cmax <= 0.0 || !rhs.is_finite() {
        return None;
    }
    // 小さい係数は境界で吸収して消す (HiGHS の `postprocessCut` は 100·feastol·max(cmax, 1e-3) ≈ 1e-4·cmax 以下を消す)。
    // カットは Σ c x <= rhs の向きなので、c_j x_j >= min(c_j lo_j, c_j up_j) (c_j > 0 なら下限、c_j < 0 なら上限) から
    // Σ_{k≠j} c_k x_k <= rhs - c_j x_j <= rhs - min(...) となり、右辺を緩める向きにだけ動く。
    // 閾値は `ENOMOTO_T_MIP_CUT_SMALL_REL` (既定 1e-4、1e-9 で以前と同じ)。その境界が無限のときは、以前の閾値 1e-9 以下なら
    // カットを捨て、それより大きければ係数を残す (以前より多くは捨てない)
    let small_rel = tunable!("ENOMOTO_T_MIP_CUT_SMALL_REL", 1e-4, f64);
    let mut coefs: Vec<(usize, f64)> = Vec::with_capacity(dense.len());
    for (j, c) in dense {
        if c.abs() <= small_rel * cmax {
            let m = if c > 0.0 { c * lo[j] } else { c * up[j] };
            if m.is_finite() {
                rhs -= m;
                continue;
            }
            if c.abs() <= 1e-9 * cmax {
                return None;
            }
        }
        coefs.push((j, c));
    }
    if coefs.is_empty() {
        return None;
    }
    // 係数の締め付け (HiGHS の `HighsDomain::tightenCoefficients`): 最大活動量が右辺を delta 超えるなら、
    // 係数の絶対値が delta を超える整数列の係数を delta に下げる (x がその境界から 1 以上離れれば、残りの項の
    // 最大活動量だけで成り立つので、右辺も同じだけ動かしてよい)。`ENOMOTO_MIP_NO_CUT_TIGHTEN` で無効。
    if env_str!("ENOMOTO_MIP_NO_CUT_TIGHTEN").is_none() {
        let mut maxact = 0.0;
        let mut fin = true;
        for &(j, c) in &coefs {
            let m = if c > 0.0 { c * up[j] } else { c * lo[j] };
            if !m.is_finite() {
                fin = false;
                break;
            }
            maxact += m;
        }
        let delta = maxact - rhs;
        if fin && delta > 1e-6 * (1.0 + rhs.abs()) {
            for (j, c) in coefs.iter_mut() {
                if is_int[*j] && c.abs() > delta + 1e-9 {
                    if *c > 0.0 {
                        rhs -= (*c - delta) * up[*j];
                        *c = delta;
                    } else {
                        rhs += (-*c - delta) * lo[*j];
                        *c = -delta;
                    }
                }
            }
        }
    }
    // 試作 `ENOMOTO_MIP_GAP4` の `l` (既定では使わない。analysis/gap4_rootstuck_mem_20261109.md): HiGHS の `HighsCutGeneration::postprocessCut`
    // と同じく、カットの非零を 100 + 0.15 × 列 までにする。超えた分は、LP の解が大域の境界にある列の項 (係数が正なら下限、負なら上限) を
    // 係数の小さい順に落とし、右辺を `c × 境界` だけ直す (落とした項は境界で下から押さえるので妥当なまま、LP の解での違反は変わらない)。
    // 落とせる項が足りなければ捨てる。neos-957323・tbfp-network の根のカットは 1 本 3.5〜4 万非零で、木の各スレッドのプール・LP が大きかった
    if super::solver::gap4('l') {
        let maxlen = 100 + (0.15 * n as f64) as usize;
        if coefs.len() > maxlen {
            let need = coefs.len() - maxlen;
            let tol = 1e-9;
            let mut cancel: Vec<usize> = (0..coefs.len())
                .filter(|&t| {
                    let (j, c) = coefs[t];
                    if c > 0.0 { lo[j].is_finite() && x[j] <= lo[j] + tol } else { up[j].is_finite() && x[j] >= up[j] - tol }
                })
                .collect();
            if cancel.len() < need {
                return None;
            }
            cancel.sort_by(|&a, &b| coefs[a].1.abs().total_cmp(&coefs[b].1.abs()));
            let mut drop = vec![false; coefs.len()];
            for &t in &cancel[..need] {
                let (j, c) = coefs[t];
                rhs -= c * if c > 0.0 { lo[j] } else { up[j] };
                drop[t] = true;
            }
            let mut k = 0;
            coefs.retain(|_| {
                k += 1;
                !drop[k - 1]
            });
            GAP4_CUT_SHORTENED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    // 係数の幅 (dynamism) が大きすぎるカットは数値的に危ないので捨てる (小さい係数を吸収した後で判定する)
    let cmax = coefs.iter().fold(0.0f64, |m, &(_, c)| m.max(c.abs()));
    let cmin = coefs.iter().fold(f64::INFINITY, |m, &(_, c)| m.min(c.abs()));
    if cmax / cmin > 1e6 {
        return None;
    }
    // 最大係数 1 にスケール
    let s = 1.0 / cmax;
    for c in coefs.iter_mut() {
        c.1 *= s;
    }
    rhs *= s;
    let act: f64 = coefs.iter().map(|&(j, c)| c * x[j]).sum();
    let norm: f64 = coefs.iter().map(|&(_, c)| c * c).sum::<f64>().sqrt();
    let viol = act - rhs;
    if viol <= 1e-6 * (1.0 + rhs.abs()) {
        return None;
    }
    let efficacy = viol / norm;
    if efficacy < 1e-5 {
        return None;
    }
    // 安全確認: 暫定解を切るカットは (生成の誤りなので) 捨てる
    if let Some(inc) = incumbent {
        let a: f64 = coefs.iter().map(|&(j, c)| c * inc[j]).sum();
        if a > rhs + 1e-6 * (1.0 + rhs.abs()) {
            if env_str!("ENOMOTO_MIP_LOG").is_some() {
                eprintln!("MIP: warning: a generated cut cuts off the incumbent (rejected)");
            }
            return None;
        }
    }
    if cmirdelta_new {
        super::cmirdelta::note_new_cand(&coefs, rhs);
    }
    Some(Candidate { coefs, rhs, efficacy, forced: false, src: 0 })
}

/// 効き目の大きい順に、既に選んだものとほぼ平行 (|cos| > 0.99) なものを除いて選ぶ。
/// 試作 `ENOMOTO_MIP_MPSEP` (既定では使わない、analysis/mpsep_20261106.md) の値。サブ MIP では使わない。値に `m` を含めば主の根のループだけ、
/// `b` を含めば裏のカットの写しだけ。
fn mpsep_flags(submip: bool, in_job: bool) -> Option<String> {
    let v = env_def!("ENOMOTO_MIP_MPSEP", "oqmgs")?;
    if submip || (v.contains('m') && in_job) || (v.contains('b') && !in_job) {
        return None;
    }
    Some(v.to_string())
}

/// 試作 `ENOMOTO_MIP_CUTSEL_ENS` の選択 ([`super::cutsel_ens::select`])。候補が多すぎて使わない (`c`) なら [`select_cuts`]
fn select_cuts_ens(cands: Vec<Candidate>, forced: &[Candidate], max_cuts: usize, p: &MipProblem, fl: &str) -> Vec<Candidate> {
    let cs: Vec<(&[(usize, f64)], f64)> = cands.iter().map(|c| (c.coefs.as_slice(), c.efficacy)).collect();
    let fs: Vec<&[(usize, f64)]> = forced.iter().map(|c| c.coefs.as_slice()).collect();
    // `k`: 本数を既定の選び方が採る本数にそろえる
    let max_cuts = if fl.contains('k') { select_cuts(cands.clone(), max_cuts, p).len().max(1) } else { max_cuts };
    let Some(sel) = super::cutsel_ens::select(&cs, &fs, p, max_cuts, fl) else {
        return select_cuts(cands, max_cuts, p);
    };
    let mut slot: Vec<Option<Candidate>> = cands.into_iter().map(Some).collect();
    sel.into_iter().filter_map(|i| slot[i].take()).collect()
}

fn select_cuts(cands: Vec<Candidate>, max_cuts: usize, p: &MipProblem) -> Vec<Candidate> {
    if env_str!("ENOMOTO_MIP_CUTSEL_OLD").is_some() {
        return select_cuts_old(cands, max_cuts);
    }
    select_cuts_hybrid(cands, max_cuts, p)
}

/// 固定費用ネットワーク型か: 連続変数 1 つと整数変数 1 つからなる行 (VUB/VLB) が行の
/// `ENOMOTO_T_FC_ROW_FRAC` (0.7) 以上で、連続変数の `ENOMOTO_T_FC_COL_FRAC` (0.9) 以上がそのような行を持つ。
fn fixed_charge_network(p: &MipProblem) -> bool {
    fixed_charge_network_with(p, tunable!("ENOMOTO_T_FC_ROW_FRAC", 0.7, f64))
}

/// 木のノードの分離を強める固定費用ネットワーク型の判定 ([`Solver::node_cuts_due`]): [`fixed_charge_network`] と同じだが、
/// 行の割合のしきい値は `ENOMOTO_T_FC_NODE_ROW_FRAC` (0.5)。流量保存の行も数えるので、弧に対して節点の多いネットワークでは
/// VUB の行の割合が下がる (beasleyC3: 1473 行中 852 行 = 58%、連続変数は 852 本すべてが VUB を持つ)
pub(super) fn fixed_charge_network_for_nodes(p: &MipProblem) -> bool {
    fixed_charge_network_with(p, tunable!("ENOMOTO_T_FC_NODE_ROW_FRAC", 0.5, f64))
}

fn fixed_charge_network_with(p: &MipProblem, row_frac: f64) -> bool {
    // 前処理で消えた列 (行に現れない、固定された) は数えない
    let ncont = (0..p.n).filter(|&j| !p.is_int[j] && !p.cols[j].is_empty() && p.col_lo[j] < p.col_up[j]).count();
    if ncont == 0 || p.m == 0 {
        return false;
    }
    let mut has_vb = vec![false; p.n];
    let mut nrows = 0usize;
    for r in &p.rows {
        if r.len() != 2 {
            continue;
        }
        let (a, b) = (r[0].0, r[1].0);
        let c = match (p.is_int[a], p.is_int[b]) {
            (false, true) => a,
            (true, false) => b,
            _ => continue,
        };
        nrows += 1;
        has_vb[c] = true;
    }
    let ncol = has_vb.iter().filter(|&&v| v).count();
    if env_str!("ENOMOTO_MIP_LOG").is_some() {
        eprintln!("MIP: fixed-charge check: vb rows {nrows} of {}, continuous with vb {ncol} of {ncont}", p.m);
    }
    let col_frac = tunable!("ENOMOTO_T_FC_COL_FRAC", 0.9, f64);
    let raw = nrows as f64 >= row_frac * p.m as f64 && ncol as f64 >= col_frac * ncont as f64;
    // 試作 `ENOMOTO_MIP_CUTMIX=h`: 再スタートでの入れ替わりにヒステリシス (analysis/cutmix_20261111.md)
    super::cutmix::fc_hyst(raw, row_frac, col_frac, nrows as f64 / p.m as f64, ncol as f64 / ncont as f64)
}

/// 試作 `ENOMOTO_MIP_NODECNT` の `r`: カットの指紋 (係数・右辺のビットのハッシュ、非零だけ)
fn cut_fingerprint(coefs: &[(usize, f64)], rhs: f64) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for &(j, a) in coefs {
        if a != 0.0 {
            j.hash(&mut h);
            a.to_bits().hash(&mut h);
        }
    }
    rhs.to_bits().hash(&mut h);
    h.finish()
}

/// 疎なベクトル (列番号の昇順) の内積。
/// 2 本のカットの係数の向きの余弦 (絶対値)。
pub(super) fn cut_parallelism(a: &[(usize, f64)], b: &[(usize, f64)]) -> f64 {
    let na = a.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
    let nb = b.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    sparse_dot(a, b).abs() / (na * nb)
}

fn sparse_dot(a: &[(usize, f64)], b: &[(usize, f64)]) -> f64 {
    let (mut i, mut k, mut dot) = (0, 0, 0.0);
    while i < a.len() && k < b.len() {
        let (x, y) = (a[i].0, b[k].0);
        if x == y {
            dot += a[i].1 * b[k].1;
            i += 1;
            k += 1;
        } else if x < y {
            i += 1;
        } else {
            k += 1;
        }
    }
    dot
}

/// SCIP の `cutsel_hybrid` に倣ったカット選択: スコア = 効き目 + 0.1 × 目的関数との平行度 + 0.1 × 整数列の割合。
/// スコアの高い順に採り、採ったカットとの平行度 (|cos|) が 0.3 を超えるものは捨てる。ただしスコアが最良の
/// 0.9 倍以上の「良い」カットは平行度 0.7 まで許す (SCIP の既定は 0.1 / 0.5 だが、こちらはカットのラウンド数が
/// 少ないので緩めにした方が良かった)。
fn select_cuts_hybrid(cands: Vec<Candidate>, max_cuts: usize, p: &MipProblem) -> Vec<Candidate> {
    select_cuts_hybrid_with(cands, max_cuts, p, tunable!("ENOMOTO_T_CUTSEL_MIN_REL", 0.5, f64), tunable!("ENOMOTO_T_CUTSEL_MAXPAR", 0.3, f64))
}

/// [`select_cuts_hybrid`] の質の閾値 (最良の何倍未満を捨てるか) と平行度の上限を指定する版。
fn select_cuts_hybrid_with(cands: Vec<Candidate>, max_cuts: usize, p: &MipProblem, min_rel: f64, maxpar: f64) -> Vec<Candidate> {
    select_cuts_hybrid_anchor(cands, max_cuts, p, min_rel, maxpar, None)
}

/// [`select_cuts_hybrid`] のスコア。
fn cut_score(c: &Candidate, p: &MipProblem, cnorm: f64) -> f64 {
    // 密なカットを避ける (`ENOMOTO_T_CUTSEL_DENSITY_EXP` = α: スコアを 非零の数^α で割る。HiGHS は効いている
    // 非零の数で割る (α = 1 相当)。密なカットは LP の反復が増え、カットの行から作る次のカットも密になる)
    cut_quality(c, p, cnorm) / (c.coefs.len().max(1) as f64).powf(tunable!("ENOMOTO_T_CUTSEL_DENSITY_EXP", 0.0, f64))
}

/// [`select_cuts_hybrid_with`] の、質の閾値の基準のスコアを `anchor` で与えられる版 (`None` なら候補の最良のスコア)。
fn select_cuts_hybrid_anchor(cands: Vec<Candidate>, max_cuts: usize, p: &MipProblem, min_rel: f64, maxpar: f64, anchor: Option<f64>) -> Vec<Candidate> {
    let cnorm = p.cost.iter().map(|c| c * c).sum::<f64>().sqrt();
    let mut scored: Vec<(f64, f64, Candidate)> = cands
        .into_iter()
        .map(|c| {
            let nc = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
            (cut_score(&c, p, cnorm), nc, c)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let Some(best_score) = scored.first().map(|s| s.0) else { return Vec::new() };
    let best_score = anchor.map_or(best_score, |a| a.min(best_score));
    if env_str!("ENOMOTO_MIP_DEBUG_CUTSEL").is_some() {
        let above = scored.iter().filter(|s| s.0 >= min_rel * best_score).count();
        let top: Vec<String> = scored
            .iter()
            .take(6)
            .map(|(s, nc, c)| {
                let objpar = if cnorm > 0.0 && *nc > 0.0 { c.coefs.iter().map(|&(j, v)| v * p.cost[j]).sum::<f64>().abs() / (cnorm * nc) } else { 0.0 };
                format!("{s:.4}(eff {:.4} op {objpar:.2} nnz {})", c.efficacy, c.coefs.len())
            })
            .collect();
        let max_eff = scored.iter().map(|s| s.2.efficacy).fold(0.0, f64::max);
        eprintln!("CUTSEL: {} cands, {above} above {min_rel} x best, max eff {max_eff:.4}; top {}", scored.len(), top.join(" "));
    }
    // 質が最良の 0.9 倍以上の「良い」カットの平行度の上限 (`maxpar` より小さくてもそのまま使う。固定費用型の緩めた選択で
    // maxpar に合わせて 0.9 にすると sp150x300d の根の下界が 65.3 -> 58.0 に下がった)
    let (goodmaxpar, good) = (tunable!("ENOMOTO_T_CUTSEL_GOODMAXPAR", 0.7, f64), 0.9 * best_score);
    // `ENOMOTO_T_CUTSEL_MIN_REL` (既定 0.5): 質が最良の何倍未満のカットは採らない (Wesselmann・Suhl の MOPS と同じ
    // 最良の 50%)。弱いカットを LP に入れず、1 ラウンドの LP を軽くする (根の LP の反復: qnet1 1936 -> 1149、
    // h80x6320d 7202 -> 3378)。40 問 2 回: sgeomean 27.84 -> 26.72、28.10 -> 26.81 (misc07 36 -> 11 秒、mik-250 33 -> 25 秒)。
    // 0.3 は 27.13。0 なら以前と同じ (上限 50 本まで全部)
    let min_score = min_rel * best_score;
    // `ENOMOTO_T_CUTSEL_FILL_FRAC` = f (> 0): 相対閾値で採ったカットが max_cuts·f 本未満なら、閾値を下げて
    // (候補の上位半分まで) max_cuts·f 本になるまで足す (HiGHS の `HighsCutPool::separate` の「少なすぎれば上位半分」)。
    // p0201 は候補 130-480 本から 1-4 本しか採らず、25 ラウンドかけても根の下界が伸び切らなかった
    let fill = (tunable!("ENOMOTO_T_CUTSEL_FILL_FRAC", 0.0, f64) * max_cuts as f64).round() as usize;
    let half = scored.len().div_ceil(2);
    let dbg = env_str!("ENOMOTO_MIP_DEBUG_CUTSEL").is_some();
    let mut n_par = 0usize;
    let mut chosen: Vec<(f64, Candidate)> = Vec::new();
    let mut rest: Vec<(f64, f64, Candidate)> = Vec::new();
    for (k, (score, nc, c)) in scored.into_iter().enumerate() {
        if chosen.len() >= max_cuts {
            break;
        }
        if score < min_score {
            if fill > 0 && k < half {
                rest.push((score, nc, c));
                continue;
            }
            break;
        }
        let limit = if score >= good { goodmaxpar } else { maxpar };
        let parallel = chosen.iter().any(|(nd, d)| sparse_dot(&c.coefs, &d.coefs).abs() > limit * nc * nd);
        if !parallel {
            chosen.push((nc, c));
        } else {
            n_par += 1;
        }
    }
    if dbg {
        eprintln!("CUTSEL: chosen {} (parallel rejected {n_par})", chosen.len());
    }
    if chosen.len() < fill {
        for (_, nc, c) in rest {
            if chosen.len() >= fill {
                break;
            }
            let parallel = chosen.iter().any(|(nd, d)| sparse_dot(&c.coefs, &d.coefs).abs() > maxpar * nc * nd);
            if !parallel {
                chosen.push((nc, c));
            }
        }
    }
    chosen.into_iter().map(|(_, c)| c).collect()
}

/// 集約した行からのカットの枠 (`ENOMOTO_T_MIP_CUTSEL_AGG_QUOTA`、試作)。tableau・HiGHS 流の経路集約・経路集約の候補を分離器ごとに
/// [`cut_score`] の順に見て、すでに採ったものとの平行度が `ENOMOTO_T_CUTSEL_MAXPAR` (0.3) 以下なら採り、分離器ごとに `q` 本まで採る。
/// 返り値は (採ったもの、残り)。physiciansched6-2 では、元の行・zerohalf の 2〜3 非零のカット (効き目 0.5 前後、LP の値はほとんど
/// 動かない) が質の最良を決め、その 50% を下回る tableau のカット (効き目 0.1〜0.3) が 1 本も採られない。HiGHS が根の下界を上げる
/// のに使っている一般係数のカット (ENOMOTO の根の LP に 12 本足すと 48984 -> 49266) と余弦 0.8〜0.9 の候補が、その中にある
fn pick_agg_quota(cands: Vec<Candidate>, q: usize, p: &MipProblem) -> (Vec<Candidate>, Vec<Candidate>) {
    let cnorm = p.cost.iter().map(|c| c * c).sum::<f64>().sqrt();
    let maxpar = tunable!("ENOMOTO_T_CUTSEL_MAXPAR", 0.3, f64);
    let fam = |c: &Candidate| -> Option<usize> {
        match c.src % sepprof::SRC_FACE {
            sepprof::SRC_PATHH => Some(0),
            sepprof::SRC_PATH => Some(1),
            sepprof::SRC_TAB => Some(2),
            _ => None,
        }
    };
    let mut order: Vec<(f64, usize)> = cands.iter().enumerate().filter(|(_, c)| fam(c).is_some()).map(|(i, c)| (cut_score(c, p, cnorm), i)).collect();
    order.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut taken = vec![false; cands.len()];
    let mut picked: Vec<(f64, usize)> = Vec::new(); // (ノルム, 番号)
    let mut cnt = [0usize; 3];
    for (_, i) in order {
        let c = &cands[i];
        let f = fam(c).unwrap();
        if cnt[f] >= q {
            continue;
        }
        let nc = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
        if picked.iter().any(|&(nd, k)| sparse_dot(&c.coefs, &cands[k].coefs).abs() > maxpar * nc * nd) {
            continue;
        }
        picked.push((nc, i));
        taken[i] = true;
        cnt[f] += 1;
    }
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for (i, c) in cands.into_iter().enumerate() {
        if taken[i] {
            a.push(c);
        } else {
            b.push(c);
        }
    }
    (a, b)
}

fn select_cuts_old(mut cands: Vec<Candidate>, max_cuts: usize) -> Vec<Candidate> {
    cands.sort_by(|a, b| b.efficacy.total_cmp(&a.efficacy));
    let mut chosen: Vec<Candidate> = Vec::new();
    let mut norms: Vec<f64> = Vec::new();
    for c in cands {
        if chosen.len() >= max_cuts {
            break;
        }
        let nc: f64 = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
        let mut parallel = false;
        for (d, &nd) in chosen.iter().zip(&norms) {
            // 疎な内積 (両方とも列番号の昇順)
            let (mut i, mut k, mut dot) = (0, 0, 0.0);
            while i < c.coefs.len() && k < d.coefs.len() {
                let (a, b) = (c.coefs[i].0, d.coefs[k].0);
                if a == b {
                    dot += c.coefs[i].1 * d.coefs[k].1;
                    i += 1;
                    k += 1;
                } else if a < b {
                    i += 1;
                } else {
                    k += 1;
                }
            }
            if dot.abs() > 0.99 * nc * nd {
                parallel = true;
                break;
            }
        }
        if !parallel {
            norms.push(nc);
            chosen.push(c);
        }
    }
    chosen
}

/// 根のカットの生成を並列にするスレッドの数。`ENOMOTO_MIP_SEP_THREADS_M1` (試作) ならスレッド数 − 1 (FJ のスレッドの分を空ける。
/// 生成の間は生成のスレッドと FJ のスレッドが同時に動き、4 スレッドなら 5 本が 4 CPU を取り合う)
impl<'a, L: MipLp> Solver<'a, L> {
    /// 分離に使うスレッド数の元 (`ENOMOTO_MIP_THREAD_BUDGET` では、根 (写しでない・サブ MIP でない) では FJ のスレッドの分を 1 本引く)。
    fn sep_budget_threads(&self) -> usize {
        // 試作 `ENOMOTO_MIP_OPTFACE_PARCUTS` の `k`: 勝ち残った裏のカットの写しは、写し全部の分のスレッドで分離する
        let boost = super::solver::CUT_JOB.with(|j| j.borrow().as_ref().map_or(0, |j| j.boost.load(std::sync::atomic::Ordering::Relaxed)));
        if boost > 0 && self.snap.is_some() {
            return boost;
        }
        if super::solver::thread_budget_on() && self.snap.is_none() && !self.params.submip && self.params.threads > 1 { self.params.threads - 1 } else { self.params.threads }
    }
}

fn sep_threads(threads: usize) -> usize {
    if env_str!("ENOMOTO_MIP_SEP_THREADS_M1").is_some() { threads.saturating_sub(1).max(1) } else { threads.max(1) }
}

/// 試作 `ENOMOTO_MIP_LOSE6_2_TIE` (既定では使わない。`0` でも使わない。analysis/lose6_2_20261103.md): カット選択の試し (`ENOMOTO_MIP_CUTSEL_PROBE`) で、
/// 1 ラウンド目の伸びが既定の選び方と POOL で同じ (差が許容誤差以内) なら、比べ直しを「既定 6 ラウンド・POOL は予算の残りまで」ではなく、両方を同じ
/// `ENOMOTO_T_MIP_LOSE6_2_TIE_ROUNDS` (既定 2) ラウンドだけ回す形にする。physiciansched6-2 では 1 ラウンド目の伸びが両方 4.291667 で並び、
/// 比べ直しの予算を既定の 6 ラウンドが使い切って POOL が 1 ラウンドで打ち切られ、1 ラウンドの時間の揺れで既定を選んでいた
/// (POOL は 2 ラウンドで伸び 117、既定は 4.29。analysis/phys6_20261101.md の 2 節の 4)。
pub(crate) fn lose6_2_tie() -> bool {
    env_str!("ENOMOTO_MIP_LOSE6_2_TIE").is_some_and(|v| v != "0")
}

thread_local! {
    /// 試作 `ENOMOTO_MIP_LOSE6_2_TIE` ([`lose6_2_tie`]): 同点の比べ直しの間だけ、比べの 1 回の LP の上限に掛ける倍率
    static LOSE6_2_PROBE_CAP_MULT: std::cell::Cell<f64> = const { std::cell::Cell::new(1.0) };
}

/// 試作 `ENOMOTO_MIP_MUSHAGG` (x0 を消す 2 本の行の集約と変数上下限の 0-1 cover) の有効性の確かめ
#[cfg(test)]
mod mushagg_tests {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn f(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
        fn i(&mut self, lo: i64, hi: i64) -> i64 {
            lo + (self.f() * (hi - lo + 1) as f64).floor() as i64
        }
    }

    /// 列の値の全ての組 (整数列の上下限の中) のうち、全ての行を満たすもの
    fn feasible_points(lo: &[f64], up: &[f64], rows: &[Vec<(usize, f64)>], rlo: &[f64], rup: &[f64]) -> Vec<Vec<f64>> {
        let n = lo.len();
        let mut out = Vec::new();
        let mut q: Vec<f64> = lo.to_vec();
        loop {
            if rows.iter().zip(rlo.iter().zip(rup)).all(|(r, (&l, &u))| {
                let a: f64 = r.iter().map(|&(j, c)| c * q[j]).sum();
                a >= l - 1e-9 && a <= u + 1e-9
            }) {
                out.push(q.clone());
            }
            let mut k = 0;
            while k < n {
                if q[k] < up[k] {
                    q[k] += 1.0;
                    break;
                }
                q[k] = lo[k];
                k += 1;
            }
            if k == n {
                return out;
            }
        }
    }

    /// 乱数の小さい問題: x0 (変数上下限なし) と一般整数列 w (|w| <= d z などの 2 項の行)、0-1 列 z・y、x0 を含むデータの行。
    /// データの行は乱数の LP 点 x で等号になるように右辺を決める。[`mush_agg_cuts`] (cover と、置き換えを強制した CMIR など) と
    /// [`super::super::cuts::vb_cover_cut`] (データの行 1 本ずつ) のカットが全ての整数解で成り立つことを総当たりで確かめる
    #[test]
    fn mushagg_cuts_valid() {
        let mut rng = Rng(987654321);
        let (mut ncuts, mut ncover, mut nfeas) = (0usize, 0usize, 0usize);
        for _ in 0..3000 {
            let nw = rng.i(1, 3) as usize;
            let nz = nw;
            let ny = rng.i(0, 1) as usize;
            // 列: 0 = x0, 1..=nw = w, 次の nz = z, 最後の ny = y
            let n = 1 + nw + nz + ny;
            let mut lo = vec![0.0; n];
            let mut up = vec![1.0; n];
            let mut is_int = vec![true; n];
            lo[0] = -3.0;
            up[0] = 3.0;
            for k in 1..=nw {
                lo[k] = -(rng.i(2, 3) as f64);
                up[k] = rng.i(2, 3) as f64;
            }
            if rng.f() < 0.2 {
                // 連続列にすると集約の相手にならず、cover にも入らない
                is_int[n - 1] = ny == 0;
            }
            let mut x = vec![0.0; n];
            x[0] = rng.i(-2, 1) as f64 + 0.25 + 0.5 * rng.f();
            let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
            let (mut rlo, mut rup) = (Vec::new(), Vec::new());
            for k in 1..=nw {
                let z = nw + k;
                // w <= d z + e、-w <= d' z - e' (e, e' は 0 か 1)
                let (d, e) = (up[k] - rng.i(0, 1) as f64, rng.i(0, 1) as f64 * (rng.f() < 0.3) as i64 as f64);
                rows.push(vec![(k, 1.0), (z, -d)]);
                rlo.push(f64::NEG_INFINITY);
                rup.push(e);
                let d2 = -lo[k] - rng.i(0, 1) as f64;
                rows.push(vec![(k, -1.0), (z, -d2)]);
                rlo.push(f64::NEG_INFINITY);
                rup.push(0.0);
                // LP 点: 多くは w = 0, z = 0、ときどき小数
                if rng.f() < 0.3 {
                    x[z] = 0.2 + 0.6 * rng.f();
                    x[k] = (rng.f() * 2.0 - 1.0) * d.min(d2) * x[z];
                }
            }
            if ny > 0 {
                x[n - 1] = rng.f();
            }
            let nd = rng.i(2, 4) as usize;
            for t in 0..nd {
                let mut r = vec![(0, if t % 2 == 0 { 1.0 } else { -(rng.i(1, 2) as f64) })];
                for k in 1..=nw {
                    if rng.f() < 0.7 {
                        r.push((k, [-1.0, 1.0, 2.0, -2.0][rng.i(0, 3) as usize]));
                    }
                }
                if ny > 0 && rng.f() < 0.5 {
                    r.push((n - 1, rng.i(-2, 2) as f64));
                }
                let a: f64 = r.iter().map(|&(j, c)| c * x[j]).sum();
                // `<=` か `>=` で等号 (右辺は少し動かして整数解を減らす / 増やす)
                let sh = [0.0, -0.5, 0.5, -1.0][rng.i(0, 3) as usize];
                if rng.f() < 0.5 {
                    rows.push(r);
                    rlo.push(f64::NEG_INFINITY);
                    rup.push(a);
                } else {
                    rows.push(r);
                    rlo.push(a);
                    rup.push(f64::INFINITY);
                }
                let _ = sh;
            }
            let feas = feasible_points(&lo, &up, &rows, &rlo, &rup);
            nfeas += feas.len();
            let m = rows.len();
            let p = MipProblem::from_rows(lo.clone(), up.clone(), vec![0.0; n], 0.0, 1.0, is_int.clone(), rows.clone(), rlo.clone(), rup.clone());
            let _ = m;
            let mut vb = VarBounds::from_rows(n, &is_int, &rows, &rlo, &rup);
            vb.add_int_vbs(&is_int, &lo, &up, &rows, &rlo, &rup, &[]);
            let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &x, vb: Some(&vb), imp: None };
            let (groups, _, _, _) = mush_agg_cuts(&vars, &p, &x, 3);
            let mut all: Vec<RawCut> = groups.into_iter().flatten().collect();
            let n_agg = all.len();
            for (i, r) in rows.iter().enumerate() {
                if r.len() > 2 {
                    if rup[i].is_finite() {
                        all.extend(super::super::cuts::vb_cover_cut(&vars, r, rup[i]));
                    }
                    if rlo[i].is_finite() {
                        let neg: Vec<(usize, f64)> = r.iter().map(|&(j, c)| (j, -c)).collect();
                        all.extend(super::super::cuts::vb_cover_cut(&vars, &neg, -rlo[i]));
                    }
                }
            }
            ncover += all.len() - n_agg;
            for cut in &all {
                ncuts += 1;
                for q in &feas {
                    let a: f64 = cut.coefs.iter().map(|&(k, c)| c * q[k]).sum();
                    assert!(a <= cut.rhs + 1e-7 * (1.0 + cut.rhs.abs()), "integer point {q:?} cut off by {:?} <= {} (rows {rows:?} [{rlo:?}, {rup:?}], x {x:?}, lo {lo:?} up {up:?})", cut.coefs, cut.rhs);
                }
            }
        }
        eprintln!("mushagg_cuts_valid: cuts {ncuts}, single-row covers {ncover}, feasible points {nfeas}");
        assert!(ncuts > 300 && ncover > 20 && nfeas > 1000, "cuts {ncuts}, single-row covers {ncover}, feasible points {nfeas}");
    }

    /// [`super::super::cuts::vb_cover_cut`] の小さい例: x0 を消した `w1 - w2 <= -1` (|w| <= 3 z) から `z1 + z2 >= 1`
    #[test]
    fn vb_cover_small() {
        let is_int = vec![true; 4];
        let lo = vec![-3.0, -3.0, 0.0, 0.0];
        let up = vec![3.0, 3.0, 1.0, 1.0];
        let rows = vec![vec![(0, 1.0), (2, -3.0)], vec![(0, -1.0), (2, -3.0)], vec![(1, 1.0), (3, -3.0)], vec![(1, -1.0), (3, -3.0)]];
        let rlo = vec![f64::NEG_INFINITY; 4];
        let rup = vec![0.0; 4];
        let mut vb = VarBounds::default();
        vb.add_int_vbs(&is_int, &lo, &up, &rows, &rlo, &rup, &[]);
        let x = vec![0.0, 0.0, 0.0, 0.0];
        let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &x, vb: Some(&vb), imp: None };
        let c = super::super::cuts::vb_cover_cut(&vars, &[(0, 1.0), (1, -1.0)], -1.0).expect("cut");
        assert_eq!(c.coefs, vec![(2, -1.0), (3, -1.0)]);
        assert_eq!(c.rhs, -1.0);
        // 右辺が 0 なら (w = 0 で満たせる) カットはない
        assert!(super::super::cuts::vb_cover_cut(&vars, &[(0, 1.0), (1, -1.0)], 0.0).is_none());
    }
}
