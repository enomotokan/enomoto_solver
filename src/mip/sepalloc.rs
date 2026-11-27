//! 試作 `ENOMOTO_MIP_SEPALLOC` (既定にせず閉じた (2026-11-21)。スイッチは既定オフで残す。analysis/sepalloc_20261121/): 根のカットのループで、分離器ごとの伸びと手間を
//! その求解の中で測り、伸びの取り分に比例して手間を配る (`a`)。ループを止める判断も同じ物差し (手間あたりの伸び) で行う (`s`)。
//! 値は文字の並び (例 `as`)。
//!
//! `a` (配る): 対象は zero-half・経路集約 (path_highs・path)・tableau の 4 つ (analysis/sepeff_20261120/ で手間の大半を使い、
//! 効き目が問題ごとに散るもの)。
//! - 材料は `ENOMOTO_MIP_SEPSTAT` と同じ計測 ([`super::seplearn`]): 根の累積の伸び G_k (|双対値| × 違反での按分) と、分離の段階の
//!   決定的な手間 (診断の手間を含む [`crate::ticks::total_with_diag`])。`SEPALLOC` を使うと計測も入る。
//! - 1 ラウンドの枠 B = 4 つの「今の配り方 (倍率 1) で 1 回動かしたときの手間」u_k の和 (u_k は倍率 1 で直近に動いたときの手間)。
//! - 取り分 s_k = G_k / ΣG。伸びが 0 の種類にも、伸びが 0 でない種類の中で一番小さい取り分を残し、和が 1 になるよう割り直す。
//! - 倍率 f_k = s_k × B / u_k。f_k ≥ 1 なら上限 (tableau の行の数・経路集約の始点の数・zero-half の本数) を倍率に掛けて毎ラウンド動かす
//!   (path_highs は全部の行を始点にするので上限がなく、1 倍と同じ)。倍率は初め f_k、続けて倍率 > 1 で動いたなら
//!   「そのときの手間が枠 s_k × B に合う」よう掛け直す (手間が倍率に比例しない分離器で膨らみ続けない)。f_k < 1 なら、ラウンドごとに f_k を貯め、1 以上になったラウンドだけ
//!   動かす (平均して f_k 回に 1 回)。
//! - 1 ラウンド目と、ΣG = 0 の間は今の配り方 (全部 1 倍)。伸びは目的値の 1e-9 倍 (数値の誤差) 以下を 0 と見る。
//!
//! `s` (止める): 今の停滞の判定 (SCIP 式の 10 回) の代わりに、直近の「根の LP 1 回分の手間」の窓の伸びの速さ M と、根の LP からの
//! 全体の速さ A (= (今の下界 − ループの始めの下界) / (根の LP の手間 + ループの手間)) を比べ、M < A で止める。
//! - 窓 = ループの手間で根の LP の手間 1 回分以上さかのぼった直近のラウンドからの伸び / その間の手間 (窓に満たない間は今までの判定)。
//! - 下界が一度も動いていない間は今までの判定のまま。ラウンドの上限・時間の上限・目的値が整数刻みの判定はそのまま効く。
//! - 新しい閾値は置かない (窓の長さは根の LP 1 回の手間で、比べる相手は全体の速さ)。
//!
//! 203 問の根だけの比べ (analysis/sepalloc_20261121/README.md): `s` は伸びが次第に鈍る問題で早く止まりすぎ (悪い 98 問・良い 7 問)、閉じた。
//! `a` は同じ手間では伸びる (良い 64 問・悪い 18 問) が、最後の下界は少し低い (悪い 64 問・良い 30 問)。
//!
//! 外側の主の根だけ (サブ MIP・裏のカットの写し・ノードでは使わない)。状態はスレッドごと。

use std::cell::RefCell;

use super::seplearn;
use super::separation::sepprof;

/// `ENOMOTO_MIP_SEPALLOC` に文字 `c` が入っているか
pub(crate) fn on(c: char) -> bool {
    env_str!("ENOMOTO_MIP_SEPALLOC").is_some_and(|v| v.contains(c))
}

/// 配る対象 (seplearn の区切りの番号)
const KINDS: [usize; 4] = [sepprof::SRC_ZH as usize, sepprof::SRC_PATHH as usize, sepprof::SRC_PATH as usize, sepprof::SRC_TAB as usize];
const NAMES: [&str; 4] = ["zerohalf", "path_highs", "path", "tableau"];

#[derive(Default)]
struct State {
    /// 今のラウンドに動かすか・倍率
    run: [bool; 4],
    scale: [f64; 4],
    /// 倍率 1 で 1 回動かしたときの手間 (直近)
    unit: [f64; 4],
    credit: [f64; 4],
    /// 前のラウンドの初めの累積の手間
    last_ticks: [f64; 4],
    rounds: usize,
    /// 動かしたラウンドの数・飛ばしたラウンドの数 (ログ)
    nrun: [usize; 4],
    nskip: [usize; 4],
    /// 候補がなくてやり直したラウンドの数
    redo: usize,
    /// 直前のラウンドがやり直しだった (手間が 2 回分なので u_k の測り直しに使わない)
    redone: bool,
}

thread_local! {
    static ST: RefCell<Option<State>> = const { RefCell::new(None) };
    /// 外側の根のカットのループの始めの手間 (`ENOMOTO_MIP_CUTSTOP_LOG` の `tk_loop` 用)
    static LOOP_TK0: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
}

/// 外側の根のカットのループの始め (診断の手間の起点)
pub(super) fn mark_loop_start() {
    LOOP_TK0.with(|c| c.set(crate::ticks::total_with_diag()));
}

/// 外側の根のカットのループの始めからの手間 (診断の手間を含む)
pub(super) fn loop_ticks() -> f64 {
    crate::ticks::total_with_diag() - LOOP_TK0.with(|c| c.get())
}

/// 根のカットのループの初めと終わり (外側の主の根だけ)
pub(super) fn activate(on: bool) {
    ST.with(|s| *s.borrow_mut() = on.then(|| State { scale: [1.0; 4], run: [true; 4], ..Default::default() }));
}

pub(super) fn active() -> bool {
    ST.with(|s| s.borrow().is_some())
}

/// 分離器 `src` をこのラウンドに動かすか (対象でない分離器・使っていないときは真)
pub(super) fn run(src: u8) -> bool {
    let Some(i) = KINDS.iter().position(|&k| k == src as usize) else { return true };
    ST.with(|s| s.borrow().as_ref().is_none_or(|st| st.run[i]))
}

/// 分離器 `src` の上限の倍率 (1 以上。対象でない・使っていないときは 1)
pub(super) fn scale(src: u8) -> f64 {
    let Some(i) = KINDS.iter().position(|&k| k == src as usize) else { return 1.0 };
    ST.with(|s| s.borrow().as_ref().map_or(1.0, |st| if st.run[i] { st.scale[i].max(1.0) } else { 1.0 }))
}

/// ラウンドの初め (分離の前): 根の累積の伸び・手間 `st` ([`seplearn::SepLearn::st`] の根の欄) と今の LP の値 `obj` から、
/// このラウンドの配り方を決める
pub(super) fn begin_round(st: &[seplearn::Stat; seplearn::NSRC], obj: f64, verbose: bool) {
    ST.with(|s| {
        let mut b = s.borrow_mut();
        let Some(a) = b.as_mut() else { return };
        // 前のラウンドの手間: 倍率 1 で動いたなら u_k、倍率 > 1 で動いたならそのときの手間 (倍率の直しに使う)
        let mut last_d = [0.0f64; 4];
        for i in 0..4 {
            let t = st[KINDS[i]].ticks;
            let d = t - a.last_ticks[i];
            a.last_ticks[i] = t;
            if a.rounds > 0 && a.run[i] && d > 0.0 && !a.redone {
                if a.scale[i] <= 1.0 {
                    a.unit[i] = d;
                } else {
                    last_d[i] = d;
                }
            }
        }
        a.redone = false;
        a.rounds += 1;
        // 伸びは数値の誤差 (目的値の 1e-9 倍) 以下を 0 と見る
        let tol = 1e-9 * obj.abs().max(1.0);
        let g: [f64; 4] = std::array::from_fn(|i| if st[KINDS[i]].gain > tol { st[KINDS[i]].gain } else { 0.0 });
        let gsum: f64 = g.iter().sum();
        let bsum: f64 = a.unit.iter().sum();
        if a.rounds == 1 || gsum <= 0.0 || bsum <= 0.0 {
            a.run = [true; 4];
            a.scale = [1.0; 4];
        } else {
            let floor = g.iter().filter(|&&v| v > 0.0).map(|&v| v / gsum).fold(f64::INFINITY, f64::min);
            // 手間が付いていない種類 (既定で切ってある zero-half など) は取り分を持たない (枠 B にも入っていない)
            let mut sh: [f64; 4] = std::array::from_fn(|i| if a.unit[i] > 0.0 { (g[i] / gsum).max(floor) } else { 0.0 });
            let ssum: f64 = sh.iter().sum::<f64>().max(f64::MIN_POSITIVE);
            for v in sh.iter_mut() {
                *v /= ssum;
            }
            for i in 0..4 {
                if a.unit[i] <= 0.0 {
                    // 手間が付いていない (動いていないか対象の行がない): 今のまま
                    a.run[i] = true;
                    a.scale[i] = 1.0;
                    continue;
                }
                let bud = sh[i] * bsum;
                let f = bud / a.unit[i];
                if f >= 1.0 {
                    // 倍率: 前のラウンドも倍率 > 1 で動いたなら、そのときの手間が枠に合うよう掛け直す (手間が倍率に比例しない
                    // 分離器でも膨らみ続けない)。初めは f
                    a.scale[i] = if a.scale[i] > 1.0 && last_d[i] > 0.0 { (a.scale[i] * bud / last_d[i]).max(1.0) } else { f };
                    a.run[i] = true;
                    a.credit[i] = 0.0;
                } else {
                    a.credit[i] += f;
                    a.run[i] = a.credit[i] >= 1.0;
                    if a.run[i] {
                        a.credit[i] -= 1.0;
                    }
                    a.scale[i] = 1.0;
                }
            }
        }
        for i in 0..4 {
            if a.run[i] { a.nrun[i] += 1 } else { a.nskip[i] += 1 }
        }
        if verbose {
            let mut msg = format!("MIP: SEPALLOC round {}:", a.rounds);
            for i in 0..4 {
                msg += &format!(" {} g {:.3e} u {:.3e} {}", NAMES[i], g[i], a.unit[i], if a.run[i] { format!("x{:.2}", a.scale[i]) } else { "skip".to_string() });
            }
            eprintln!("{msg}");
        }
    });
}

/// このラウンドに飛ばした分離器があれば、全部を倍率 1 で動かすように直して真を返す (候補が 1 本もなかったラウンドのやり直し用)
pub(super) fn run_all_if_skipped() -> bool {
    ST.with(|s| {
        let mut b = s.borrow_mut();
        let Some(a) = b.as_mut() else { return false };
        if a.run.iter().all(|&r| r) {
            return false;
        }
        for i in 0..4 {
            if !a.run[i] {
                a.nskip[i] -= 1;
                a.nrun[i] += 1;
            }
        }
        a.run = [true; 4];
        a.scale = [1.0; 4];
        a.redo += 1;
        a.redone = true;
        true
    })
}

/// ループの終わりのまとめ (ログの 1 行。使っていなければ空)
pub(super) fn summary() -> Option<String> {
    ST.with(|s| {
        s.borrow().as_ref().map(|a| {
            let mut msg = format!("SEPALLOC rounds {} redo {}", a.rounds, a.redo);
            for i in 0..4 {
                msg += &format!(" {} run {} skip {}", NAMES[i], a.nrun[i], a.nskip[i]);
            }
            msg
        })
    })
}

/// `s` の判定の状態 (根のカットのループ 1 回分)
pub(super) struct StopRule {
    first_obj: f64,
    /// 根の LP の手間 (窓の長さ)
    tlp: f64,
    t0: f64,
    /// (ループの始めからの手間, 目的値)。初めに (0, first_obj)
    hist: Vec<(f64, f64)>,
    /// 直近の (窓の速さ, 全体の速さ) (ログ用)
    pub(super) last: (f64, f64),
}

impl StopRule {
    pub(super) fn new(first_obj: f64, root_lp_ticks: f64) -> Option<Self> {
        (root_lp_ticks > 0.0).then(|| StopRule { first_obj, tlp: root_lp_ticks, t0: crate::ticks::total_with_diag(), hist: vec![(0.0, first_obj)], last: (f64::NAN, f64::NAN) })
    }

    /// ラウンドの後の目的値 (前処理の定数を除く) を入れる。答えは [`super::rootstop::Rate`] と同じ形
    pub(super) fn record(&mut self, obj: f64) -> super::rootstop::Rate {
        use super::rootstop::Rate;
        let tau = crate::ticks::total_with_diag() - self.t0;
        self.hist.push((tau, obj));
        let tiny = 1e-9 * obj.abs().max(1.0);
        if obj - self.first_obj <= tiny || tau < self.tlp {
            return Rate::Undecided;
        }
        // 窓の始め: 手間が tau - tlp 以下の最後の記録
        let from = tau - self.tlp;
        let (t0w, o0) = self.hist.iter().rev().find(|h| h.0 <= from).map_or((0.0, self.first_obj), |h| (h.0, h.1));
        let m = (obj - o0) / (tau - t0w).max(self.tlp);
        let a = (obj - self.first_obj) / (self.tlp + tau);
        self.last = (m, a);
        if m < a { Rate::Stop } else { Rate::Continue }
    }
}
