//! 分離器ごとの効き目の計測 (analysis/seplearn_20261108.md)。
//!
//! 計測 (`ENOMOTO_MIP_SEPSTAT`。既定では使わない): 分離器 ([`sepprof`] の `SRC_*`) ごとに、
//! 作った候補の数・LP に入った本数・LP の目的値の伸びへの寄与・生き残り・使った時間を、根とノードに分けて数え、根のループの終わりと
//! 求解の終わりに `ENOMOTO_MIP_LOG` (verbose) で出す。
//! - 伸びへの寄与: カットを加えてから次の LP を解いたときの目的値の伸び Δ を、加えたカット i の |双対値 y_i| × (加える前の点での
//!   違反 v_i) で按分する (一次の近似で Δ ≈ Σ y_i v_i)。打ち切り・実行不能になったら Δ = 打ち切り値 - 加える前の値 (有限なら) を
//!   違反で按分する
//! - 生き残り: 加えてから `ENOMOTO_T_MIP_SEPLEARN_SURV_K` (既定 3) 回あとの LP (同じスレッドの根のラウンドかノードの LP) で、
//!   まだ LP にあり |双対値| > 1e-9 のカットの割合 (それまでに外れたものは生き残らなかったと数える)
//!
//! 得点 (表に出すだけ): 分離器ごとに 1 回の呼び出しあたりの伸び・仕事 (候補の非零の数 + `ENOMOTO_T_MIP_SEPLEARN_LPW` × LP に入れた
//! カットの非零の数)・生き残りの指数移動平均を持ち、0.5 × 伸び (最良を 1 にそろえる) + 0.3 × 伸び / 仕事 (同) + 0.2 × 生き残り とする。
//! 根の平均は木の探索の写しへ引き継ぐ ([`SepLearn::for_tree`])。
//! 得点で分離器を飛ばし候補に上限をかける学習の試作 (`ENOMOTO_MIP_SEPLEARN`) は、58 問で解けた数 85 → 81 と既定に勝たなかったので
//! 削除した (2026-11-18、analysis/seplearn_20261108.md・analysis/cleanup_20261118/)。計測の調整値 (`ENOMOTO_T_MIP_SEPLEARN_ALPHA`・
//! `_SURV_K`・`_LPW`) と表の行の頭 `SEPLEARN` は、記録の台本と合わせるため名前を変えずに残した。

use super::separation::sepprof;
use std::cell::RefCell;

/// 数える区切りの数: 分離器 (`SRC_*`) に、元の行 (`SRC_ROWS`) の中の種類 (持ち上げつきの被覆・拡張被覆・flow cover、
/// [`super::cuts::gen_kind`]) を足したもの。`SRC_ROWS` の欄は CMIR (と種類の分からないもの) だけになる (analysis/sepeff_20261120/)
pub(super) const NSRC: usize = sepprof::NSRC + 3;
const NAMES: [&str; NSRC] = ["extra", "rows_cmir", "zerohalf", "clique", "implbd", "path_highs", "path", "tableau", "pool", "mcf", "modk", "lcover", "ecover", "flowcover"];

/// カットの分離器 `src` (面の頂点の + `SRC_FACE` は外す) と元の行の中の種類 `sub` から、数える区切りの番号
pub(super) fn key(src: u8, sub: u8) -> usize {
    let k = (src % sepprof::SRC_FACE) as usize;
    if k == sepprof::SRC_ROWS as usize && sub > 0 {
        sepprof::NSRC + sub as usize - 1
    } else {
        k
    }
}

thread_local! {
    /// 分離器ごとの時間 (秒) の累計 (このスレッド、[`sepprof::lap`] が足す)
    static SLT: RefCell<[f64; NSRC]> = const { RefCell::new([0.0; NSRC]) };
    /// 分離器ごとの決定的な手間 ([`crate::ticks::total`]) の累計 (このスレッド、[`add_stage_ticks`] が足す)
    static SLK: RefCell<[f64; NSRC]> = const { RefCell::new([0.0; NSRC]) };
    /// [`add_stage_ticks`] が前に見た、元の行の種類ごとの手間 ([`super::cuts::GenRec::ticks`]) の累計
    static GEN_LAST: RefCell<[f64; super::cuts::gen_kind::N]> = const { RefCell::new([0.0; super::cuts::gen_kind::N]) };
}

/// `separate()` の段階の手間 `d` を分離器に足す (前処理 `T_SETUP` と、分離の外 (面の LP・プール) の段階は数えない)。
/// 元の行の段階は、種類ごとの手間を被覆・flow cover に分け、残りを CMIR (`SRC_ROWS`) にする
pub(super) fn add_stage_ticks(stage: usize, d: f64) {
    if stage == sepprof::T_SETUP || stage >= sepprof::NSEP {
        return;
    }
    let src = stage_src(stage) as usize;
    SLK.with(|t| {
        let mut t = t.borrow_mut();
        if src == sepprof::SRC_ROWS as usize {
            let now = super::cuts::GenRec::ticks();
            let last = GEN_LAST.with(|g| std::mem::replace(&mut *g.borrow_mut(), now));
            let mut rest = d;
            for k in 1..super::cuts::gen_kind::N {
                let dk = (now[k] - last[k]).clamp(0.0, rest.max(0.0));
                t[sepprof::NSRC + k - 1] += dk;
                rest -= dk;
            }
            t[src] += rest;
        } else {
            t[src] += d;
        }
    });
}

fn stage_src(k: usize) -> u8 {
    match k {
        sepprof::T_ROWS => sepprof::SRC_ROWS,
        sepprof::T_ZH => sepprof::SRC_ZH,
        sepprof::T_CLIQUE => sepprof::SRC_CLIQUE,
        sepprof::T_IMPLBD => sepprof::SRC_IMPLBD,
        sepprof::T_PATHH => sepprof::SRC_PATHH,
        sepprof::T_PATH => sepprof::SRC_PATH,
        sepprof::T_TABSEL | sepprof::T_TABAGG | sepprof::T_TABGEN => sepprof::SRC_TAB,
        sepprof::T_MCF => sepprof::SRC_MCF,
        _ => sepprof::SRC_EXTRA,
    }
}

/// `separate()` の段階 (`sepprof::T_*`、面の頂点は + NSEP) を分離器 (`SRC_*`) に対応させる
pub(super) fn add_stage_time(stage: usize, secs: f64) {
    let src = stage_src(stage % sepprof::NSEP);
    SLT.with(|t| t.borrow_mut()[src as usize] += secs);
}

/// 分離器 `src` の時間を足す (段階の時計に入らないもの)
pub(super) fn add_src_time(src: u8, secs: f64) {
    SLT.with(|t| t.borrow_mut()[src as usize] += secs);
}

/// 計測を使うか (`ENOMOTO_MIP_SEPSTAT`)
pub(super) fn stat_on() -> bool {
    // 試作 `ENOMOTO_MIP_SEPALLOC` ([`super::sepalloc`]) は伸びと手間をこの計測から読むので、計測も入れる
    env_str!("ENOMOTO_MIP_SEPSTAT").is_some() || env_str!("ENOMOTO_MIP_SEPALLOC").is_some()
}

#[derive(Clone, Copy, Default)]
pub(super) struct Stat {
    /// 動いた呼び出しの数 (候補が 1 本以上か時間が付いた)
    pub calls: u64,
    pub cands: u64,
    pub chosen: u64,
    pub secs: f64,
    /// 決定的な手間 ([`crate::ticks::total`] の差、分離の段階の分だけ)
    pub ticks: f64,
    /// 加えた直後の LP で |双対値| > 1e-9 だった本数 (効いている本数)
    pub act1: u64,
    pub work: f64,
    pub gain: f64,
    pub surv_n: u64,
    pub surv_ok: u64,
    /// (削除した学習の試作が飛ばした呼び出しの数・候補の上限で捨てた候補の数。今は 0)
    pub skipped: u64,
    pub capped: u64,
}

/// 伸びを待っている加えたカット
#[derive(Clone)]
struct Pending {
    obj0: f64,
    node: bool,
    /// (カットの番号、区切りの番号、加える前の点での違反)
    items: Vec<(u64, usize, f64)>,
}

#[derive(Clone)]
pub(super) struct SepLearn {
    pub stat: bool,
    /// [根, ノード][分離器]
    pub st: [[Stat; NSRC]; 2],
    /// 学習の指数移動平均 [根, ノード][分離器]: 伸び、仕事、生き残り、観測の数
    ema_gain: [[f64; NSRC]; 2],
    ema_work: [[f64; NSRC]; 2],
    ema_surv: [[f64; NSRC]; 2],
    nobs: [[u32; NSRC]; 2],
    /// 今が根かノードか
    cur_node: bool,
    /// この呼び出しの分離器ごとの候補の数・仕事・時間 (加えた後の伸びの観測に合わせて EMA に入れる)
    call_cands: [u64; NSRC],
    call_work: [f64; NSRC],
    call_secs: [f64; NSRC],
    call_ticks: [f64; NSRC],
    last_ticks: [f64; NSRC],
    pending_prep: Option<(f64, bool, Vec<(usize, f64)>)>,
    pending: Option<Pending>,
    /// 生き残りを見るカット (番号、分離器、加えたときの LP の通し番号、ノードか)
    track: std::collections::VecDeque<(u64, usize, u64, bool)>,
    lp_seq: u64,
    last_secs: [f64; NSRC],
}

impl Default for SepLearn {
    fn default() -> Self {
        SepLearn {
            stat: stat_on(),
            st: [[Stat::default(); NSRC]; 2],
            ema_gain: [[0.0; NSRC]; 2],
            ema_work: [[0.0; NSRC]; 2],
            ema_surv: [[0.5; NSRC]; 2],
            nobs: [[0; NSRC]; 2],
            cur_node: false,
            call_cands: [0; NSRC],
            call_work: [0.0; NSRC],
            call_secs: [0.0; NSRC],
            call_ticks: [0.0; NSRC],
            last_ticks: SLK.with(|t| *t.borrow()),
            pending_prep: None,
            pending: None,
            track: Default::default(),
            lp_seq: 0,
            last_secs: SLT.with(|t| *t.borrow()),
        }
    }
}

fn alpha() -> f64 {
    tunable!("ENOMOTO_T_MIP_SEPLEARN_ALPHA", 0.3, f64)
}

impl SepLearn {
    /// 木の探索の写し (別のスレッド) に引き継ぐ状態: 学んだ平均 (根の値をノードの初めの値にもする) だけを持つ
    pub(super) fn for_tree(&self) -> SepLearn {
        let mut s = SepLearn::default();
        s.ema_gain = self.ema_gain;
        s.ema_work = self.ema_work;
        s.ema_surv = self.ema_surv;
        s.nobs = self.nobs;
        for k in 0..NSRC {
            if s.nobs[1][k] == 0 {
                s.ema_gain[1][k] = self.ema_gain[0][k];
                s.ema_work[1][k] = self.ema_work[0][k];
                s.ema_surv[1][k] = self.ema_surv[0][k];
                s.nobs[1][k] = self.nobs[0][k];
            }
        }
        s
    }

    /// 得点 (0..1): 0.5 x 伸び (最良の分離器で 1) + 0.3 x 伸び / 仕事 (最良で 1) + 0.2 x 生き残り。観測がまだない分離器は `None`
    /// (伸び / 仕事 だけだと、伸びの小さい安い分離器 (元の行) が伸びの大部分を出す分離器より高くなった。enlight_hard のノードの zerohalf)
    fn scores(&self, w: usize) -> [Option<f64>; NSRC] {
        let mut eff = [0.0f64; NSRC];
        let (mut best_e, mut best_g) = (0.0f64, 0.0f64);
        for k in 0..NSRC {
            if self.nobs[w][k] > 0 {
                eff[k] = self.ema_gain[w][k].max(0.0) / self.ema_work[w][k].max(1e-12);
                best_e = best_e.max(eff[k]);
                best_g = best_g.max(self.ema_gain[w][k]);
            }
        }
        let mut out = [None; NSRC];
        for k in 0..NSRC {
            if self.nobs[w][k] > 0 {
                let e = if best_e > 0.0 { eff[k] / best_e } else { 1.0 };
                let g = if best_g > 0.0 { self.ema_gain[w][k].max(0.0) / best_g } else { 1.0 };
                out[k] = Some(0.5 * g + 0.3 * e + 0.2 * self.ema_surv[w][k]);
            }
        }
        out
    }

    /// 呼び出しの初め (根のラウンド・ノードの分離)
    pub(super) fn begin_call(&mut self, node: bool) {
        self.cur_node = node;
        self.call_cands = [0; NSRC];
        self.call_work = [0.0; NSRC];
        self.call_secs = [0.0; NSRC];
        self.call_ticks = [0.0; NSRC];
        self.last_secs = SLT.with(|t| *t.borrow());
        self.last_ticks = SLK.with(|t| *t.borrow());
    }

    /// 段階の時間・手間の累計のうち、前に見た後の分をこの呼び出しに足す
    fn take_lap(&mut self) {
        let now = SLT.with(|t| *t.borrow());
        let nowk = SLK.with(|t| *t.borrow());
        for k in 0..NSRC {
            self.call_secs[k] += now[k] - self.last_secs[k];
            self.call_ticks[k] += nowk[k] - self.last_ticks[k];
        }
        self.last_secs = now;
        self.last_ticks = nowk;
    }

    /// 分離の後の候補を数える (分離器ごと)
    pub(super) fn after_separate(&mut self, srcs: &[Vec<(usize, usize)>]) {
        // srcs: 候補の (区切りの番号 [`key`]、非零の数) の並び (候補の束ごと)
        let w = self.cur_node as usize;
        self.take_lap();
        let mut cnt = [0u64; NSRC];
        let mut nnz = [0f64; NSRC];
        for v in srcs.iter() {
            for &(k, z) in v {
                cnt[k] += 1;
                nnz[k] += z as f64;
            }
        }
        for k in 0..NSRC {
            self.call_cands[k] += cnt[k];
            self.call_work[k] += nnz[k];
            self.st[w][k].cands += cnt[k];
        }
    }

    /// 選んだカットを LP に加える直前: 加える前の目的値 `obj0` と、(分離器、違反) の並び
    pub(super) fn prepare(&mut self, obj0: f64, items: Vec<(usize, f64)>) {
        self.pending_prep = Some((obj0, self.cur_node, items));
    }

    /// 加えた直後: 加えたカットの番号は `next_id - k .. next_id` (k = 加えた本数)
    pub(super) fn bind(&mut self, next_id: u64) {
        let Some((obj0, node, items)) = self.pending_prep.take() else { return };
        let w = node as usize;
        let k = items.len() as u64;
        let id0 = next_id.saturating_sub(k);
        let mut nch = [0u64; NSRC];
        let its: Vec<(u64, usize, f64)> = items.iter().enumerate().map(|(i, &(s, v))| (id0 + i as u64, s, v)).collect();
        for &(id, s, _) in &its {
            nch[s] += 1;
            self.track.push_back((id, s, self.lp_seq, node));
        }
        for kk in 0..NSRC {
            self.st[w][kk].chosen += nch[kk];
        }
        self.pending = Some(Pending { obj0, node, items: its });
    }

    /// 選んだカットの非零の数を仕事に足す (LP を重くする分)
    pub(super) fn add_chosen_nnz(&mut self, k: usize, nnz: usize) {
        self.call_work[k] += tunable!("ENOMOTO_T_MIP_SEPLEARN_LPW", 5.0, f64) * nnz as f64;
    }

    /// LP を解いた後。`obj` は目的値 (最適のとき)、`bound_hit` は打ち切り・実行不能で、そのとき `lim` は打ち切り値。
    /// `dual_of` はカットの番号の並びに対し |双対値| (LP にないなら None) を返す
    pub(super) fn after_lp(&mut self, obj: Option<f64>, bound_hit: bool, lim: f64, dual_of: &mut dyn FnMut(&[u64]) -> Vec<Option<f64>>) {
        self.lp_seq += 1;
        if let Some(pd) = self.pending.take() {
            let w = pd.node as usize;
            let delta = match obj {
                Some(o) => (o - pd.obj0).max(0.0),
                None if bound_hit && lim.is_finite() => (lim - pd.obj0).max(0.0),
                _ => 0.0,
            };
            let ids: Vec<u64> = pd.items.iter().map(|e| e.0).collect();
            let ys = if obj.is_some() { dual_of(&ids) } else { vec![None; ids.len()] };
            for (e, y) in pd.items.iter().zip(&ys) {
                if y.is_some_and(|v| v > 1e-9) {
                    self.st[w][e.1].act1 += 1;
                }
            }
            let mut wts: Vec<f64> = pd.items.iter().zip(&ys).map(|(e, y)| y.unwrap_or(0.0) * e.2.max(0.0)).collect();
            let mut tot: f64 = wts.iter().sum();
            if tot <= 0.0 {
                wts = pd.items.iter().map(|e| e.2.max(0.0)).collect();
                tot = wts.iter().sum();
            }
            let mut g = [0f64; NSRC];
            if tot > 0.0 && delta > 0.0 {
                for (e, wt) in pd.items.iter().zip(&wts) {
                    g[e.1] += delta * wt / tot;
                }
            }
            for k in 0..NSRC {
                self.st[w][k].gain += g[k];
            }
            self.observe(w, &g);
        }
        // 生き残り
        let kk = tunable!("ENOMOTO_T_MIP_SEPLEARN_SURV_K", 3u64, u64);
        let due = self.track.iter().take_while(|e| self.lp_seq - e.2 >= kk).count();
        if due > 0 {
            let es: Vec<(u64, usize, u64, bool)> = self.track.drain(..due).collect();
            let ids: Vec<u64> = es.iter().map(|e| e.0).collect();
            let ys = if obj.is_some() { dual_of(&ids) } else { vec![None; ids.len()] };
            let a = alpha();
            for (e, y) in es.iter().zip(ys) {
                let w = e.3 as usize;
                let k = e.1;
                let ok = y.is_some_and(|v| v > 1e-9);
                self.st[w][k].surv_n += 1;
                self.st[w][k].surv_ok += ok as u64;
                self.ema_surv[w][k] = (1.0 - a * 0.2) * self.ema_surv[w][k] + a * 0.2 * if ok { 1.0 } else { 0.0 };
            }
        }
    }

    /// 呼び出しの観測 (伸び `g`) を EMA に入れる。候補を出した (か時間を使った) 分離器だけ
    fn observe(&mut self, w: usize, g: &[f64; NSRC]) {
        let a = alpha();
        for k in 0..NSRC {
            let ran = self.call_cands[k] > 0 || self.call_secs[k] > 0.0 || self.call_ticks[k] > 0.0;
            if !ran {
                continue;
            }
            let work = self.call_work[k] + 1.0;
            self.st[w][k].calls += 1;
            self.st[w][k].secs += self.call_secs[k];
            self.st[w][k].ticks += self.call_ticks[k];
            self.st[w][k].work += work;
            if self.nobs[w][k] == 0 {
                self.ema_gain[w][k] = g[k];
                self.ema_work[w][k] = work;
            } else {
                self.ema_gain[w][k] = (1.0 - a) * self.ema_gain[w][k] + a * g[k];
                self.ema_work[w][k] = (1.0 - a) * self.ema_work[w][k] + a * work;
            }
            self.nobs[w][k] += 1;
        }
        self.call_cands = [0; NSRC];
        self.call_work = [0.0; NSRC];
        self.call_secs = [0.0; NSRC];
        self.call_ticks = [0.0; NSRC];
    }

    /// 候補が 1 本も採られずに終わった呼び出し (加えなかった) を伸び 0 で観測に入れる
    pub(super) fn end_call_without_add(&mut self) {
        let w = self.cur_node as usize;
        let g = [0f64; NSRC];
        self.take_lap();
        self.observe(w, &g);
    }

    /// 表 (`ENOMOTO_MIP_LOG`)
    pub(super) fn report(&self, tag: &str) -> String {
        let mut s = String::new();
        for (w, wn) in [(0usize, "root"), (1, "node")] {
            let tg: f64 = (0..NSRC).map(|k| self.st[w][k].gain).sum();
            let ts: f64 = (0..NSRC).map(|k| self.st[w][k].secs).sum();
            let tk: f64 = (0..NSRC).map(|k| self.st[w][k].ticks).sum();
            if self.st[w].iter().all(|e| e.cands == 0 && e.chosen == 0) {
                continue;
            }
            s += &format!("SEPLEARN {tag} {wn}: gain {tg:.6e} sep secs {ts:.3} ticks {tk:.0}\n");
            let sc = self.scores(w);
            for k in 0..NSRC {
                let e = &self.st[w][k];
                if e.cands == 0 && e.chosen == 0 && e.skipped == 0 && e.ticks <= 0.0 && e.secs <= 0.0 {
                    continue;
                }
                s += &format!(
                    "SEPLEARN {tag} {wn} {:<10} calls {} cands {} chosen {} act1 {} gain {:.4e} ({:.1}%) surv {}/{} ({:.0}%) secs {:.3} ticks {:.0} work {:.0} skipped {} capped {} score {}\n",
                    NAMES[k],
                    e.calls,
                    e.cands,
                    e.chosen,
                    e.act1,
                    e.gain,
                    if tg > 0.0 { 100.0 * e.gain / tg } else { 0.0 },
                    e.surv_ok,
                    e.surv_n,
                    if e.surv_n > 0 { 100.0 * e.surv_ok as f64 / e.surv_n as f64 } else { 0.0 },
                    e.secs,
                    e.ticks,
                    e.work,
                    e.skipped,
                    e.capped,
                    sc[k].map_or("-".to_string(), |v| format!("{v:.2}"))
                );
            }
        }
        s
    }
}
