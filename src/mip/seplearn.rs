//! 分離器ごとの効き目の計測と、効き目で分離器に配分を振る試作 (analysis/seplearn_20261108.md)。
//!
//! 計測 (`ENOMOTO_MIP_SEPSTAT`、または `ENOMOTO_MIP_SEPLEARN`。既定では使わない): 分離器 ([`sepprof`] の `SRC_*`) ごとに、
//! 作った候補の数・LP に入った本数・LP の目的値の伸びへの寄与・生き残り・使った時間を、根とノードに分けて数え、根のループの終わりと
//! 求解の終わりに `ENOMOTO_MIP_LOG` (verbose) で出す。
//! - 伸びへの寄与: カットを加えてから次の LP を解いたときの目的値の伸び Δ を、加えたカット i の |双対値 y_i| × (加える前の点での
//!   違反 v_i) で按分する (一次の近似で Δ ≈ Σ y_i v_i)。打ち切り・実行不能になったら Δ = 打ち切り値 - 加える前の値 (有限なら) を
//!   違反で按分する
//! - 生き残り: 加えてから `ENOMOTO_T_MIP_SEPLEARN_SURV_K` (既定 3) 回あとの LP (同じスレッドの根のラウンドかノードの LP) で、
//!   まだ LP にあり |双対値| > 1e-9 のカットの割合 (それまでに外れたものは生き残らなかったと数える)
//!
//! 学習 (`ENOMOTO_MIP_SEPLEARN`、既定では使わない): 分離器ごとに 1 回の呼び出しあたりの伸び・仕事 (決定的な量: 候補の非零の数 +
//! `ENOMOTO_T_MIP_SEPLEARN_LPW` × LP に入れたカットの非零の数)・生き残りの指数移動平均を持ち、得点 = 0.5 × 伸び (最良を 1 に
//! そろえる) + 0.3 × 伸び / 仕事 (同) + 0.2 × 生き残り とする。呼び出しごと (根のラウンド・ノードの分離) に、
//! (a) 呼ぶか: 最初の W 回 (根 `..._WARM` 3、ノードは根で学んだ値を引き継ぐ) は呼び、得点が `..._SKIP` (既定 0.1) 未満なら飛ばす。
//!     飛ばす長さは 2 から外れるたびに倍 (最大 `..._MAXSKIP` 16)、飛ばし終えたら 1 回試す (ε-greedy の試しを決定的にしたもの)。
//!     また仕事の割合が得点の割合の `..._WORKF` (既定 3) 倍 + 0.05 を超えた分離器も飛ばす (時間の上限の代わり。時刻は使わない)
//! (b) 候補の上限: 得点の割合 × 予算 (根は 1 ラウンドの本数の上限 × `..._CAPMULT` (既定 2)、ノードは 20 × 同) と `..._CAPMIN`
//!     (既定 20) の大きい方まで、効き目の大きい順に残す (得点が `..._NOCAP` (既定 0.5) 以上の分離器にはかけない)
//! 根で学んだ値を木の探索の写しへ引き継ぎ ([`SepLearn::for_tree`])、木の中ではノードの観測で更新する (スレッドごと)。
//! 時刻を判断に使わないので、スレッド数・種が同じなら判断は同じ (`ENOMOTO_MIP_SEPLEARN=t` なら仕事の代わりに時間を使う。決定的でない)。
//! 対象は元の行 (CMIR・flow cover・cover)・zerohalf・clique・implied bound・経路集約 (2 種)・mcf・mod-k (MODKVB)。
//! tableau 行 (Gomory) とプールは計測だけ (`ENOMOTO_MIP_SEPLEARN` の値に `g` を含めば tableau 行にも候補の上限をかける)。

use super::separation::sepprof::{self, NSRC};
use std::cell::RefCell;

thread_local! {
    /// 分離器ごとの時間 (秒) の累計 (このスレッド、[`sepprof::lap`] が足す)
    static SLT: RefCell<[f64; NSRC]> = const { RefCell::new([0.0; NSRC]) };
}

/// `separate()` の段階 (`sepprof::T_*`、面の頂点は + NSEP) を分離器 (`SRC_*`) に対応させる
pub(super) fn add_stage_time(stage: usize, secs: f64) {
    let k = stage % sepprof::NSEP;
    let src = match k {
        sepprof::T_ROWS => sepprof::SRC_ROWS,
        sepprof::T_ZH => sepprof::SRC_ZH,
        sepprof::T_CLIQUE => sepprof::SRC_CLIQUE,
        sepprof::T_IMPLBD => sepprof::SRC_IMPLBD,
        sepprof::T_PATHH => sepprof::SRC_PATHH,
        sepprof::T_PATH => sepprof::SRC_PATH,
        sepprof::T_TABSEL | sepprof::T_TABAGG | sepprof::T_TABGEN => sepprof::SRC_TAB,
        sepprof::T_MCF => sepprof::SRC_MCF,
        _ => sepprof::SRC_EXTRA,
    };
    SLT.with(|t| t.borrow_mut()[src as usize] += secs);
}

/// 分離器 `src` の時間を足す (段階の時計に入らないもの)
pub(super) fn add_src_time(src: u8, secs: f64) {
    SLT.with(|t| t.borrow_mut()[src as usize] += secs);
}

/// 計測を使うか (`ENOMOTO_MIP_SEPSTAT` か `ENOMOTO_MIP_SEPLEARN`)
pub(super) fn stat_on() -> bool {
    env_str!("ENOMOTO_MIP_SEPSTAT").is_some() || learn_mode().is_some()
}

/// 学習の値 (`ENOMOTO_MIP_SEPLEARN`、`0` は使わない)
pub(super) fn learn_mode() -> Option<&'static str> {
    env_str!("ENOMOTO_MIP_SEPLEARN").filter(|v| *v != "0")
}

/// 学習で呼ぶか・候補の上限を決める分離器 (tableau 行・プール・一時的な行は除く)
pub(super) const LEARN_SRCS: [u8; 9] = [sepprof::SRC_ROWS, sepprof::SRC_ZH, sepprof::SRC_CLIQUE, sepprof::SRC_IMPLBD, sepprof::SRC_PATHH, sepprof::SRC_PATH, sepprof::SRC_MCF, sepprof::SRC_MODK, sepprof::SRC_TAB];

#[derive(Clone, Copy, Default)]
pub(super) struct Stat {
    /// 動いた呼び出しの数 (候補が 1 本以上か時間が付いた)
    pub calls: u64,
    pub cands: u64,
    pub chosen: u64,
    pub secs: f64,
    pub work: f64,
    pub gain: f64,
    pub surv_n: u64,
    pub surv_ok: u64,
    /// 学習で飛ばした呼び出しの数・候補の上限で捨てた候補の数
    pub skipped: u64,
    pub capped: u64,
}

/// 伸びを待っている加えたカット
#[derive(Clone)]
struct Pending {
    obj0: f64,
    node: bool,
    /// (カットの番号、分離器、加える前の点での違反)
    items: Vec<(u64, u8, f64)>,
}

#[derive(Clone)]
pub(super) struct SepLearn {
    pub stat: bool,
    pub learn: bool,
    use_time: bool,
    tab: bool,
    /// [根, ノード][分離器]
    pub st: [[Stat; NSRC]; 2],
    /// 学習の指数移動平均 [根, ノード][分離器]: 伸び、仕事、生き残り、観測の数
    ema_gain: [[f64; NSRC]; 2],
    ema_work: [[f64; NSRC]; 2],
    ema_surv: [[f64; NSRC]; 2],
    nobs: [[u32; NSRC]; 2],
    /// 呼び出しの数 (飛ばしたものも含む) [根, ノード]
    ncalls: [u64; 2],
    skip_left: [[u32; NSRC]; 2],
    skip_len: [[u32; NSRC]; 2],
    /// この呼び出しで使わない分離器 (ビット) と、今が根かノードか
    pub off: u32,
    cur_node: bool,
    /// この呼び出しの分離器ごとの候補の数・仕事・時間 (加えた後の伸びの観測に合わせて EMA に入れる)
    call_cands: [u64; NSRC],
    call_work: [f64; NSRC],
    call_secs: [f64; NSRC],
    pending_prep: Option<(f64, bool, Vec<(u8, f64)>)>,
    pending: Option<Pending>,
    /// 生き残りを見るカット (番号、分離器、加えたときの LP の通し番号、ノードか)
    track: std::collections::VecDeque<(u64, u8, u64, bool)>,
    lp_seq: u64,
    last_secs: [f64; NSRC],
}

impl Default for SepLearn {
    fn default() -> Self {
        let m = learn_mode();
        SepLearn {
            stat: stat_on(),
            learn: m.is_some(),
            use_time: m.is_some_and(|v| v.contains('t')),
            tab: m.is_some_and(|v| v.contains('g')),
            st: [[Stat::default(); NSRC]; 2],
            ema_gain: [[0.0; NSRC]; 2],
            ema_work: [[0.0; NSRC]; 2],
            ema_surv: [[0.5; NSRC]; 2],
            nobs: [[0; NSRC]; 2],
            ncalls: [0; 2],
            skip_left: [[0; NSRC]; 2],
            skip_len: [[0; NSRC]; 2],
            off: 0,
            cur_node: false,
            call_cands: [0; NSRC],
            call_work: [0.0; NSRC],
            call_secs: [0.0; NSRC],
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
        // `ENOMOTO_T_MIP_SEPLEARN_NODE_WARM` > 0: ノードは根の値を引き継がず、自分の観測がこの数たまるまで飛ばさない
        if tunable!("ENOMOTO_T_MIP_SEPLEARN_NODE_WARM", 0u32, u32) > 0 {
            return s;
        }
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

    /// 呼び出しの初め (根のラウンド・ノードの分離): 使わない分離器 `off` を決める (学習のときだけ。決定的)
    pub(super) fn begin_call(&mut self, node: bool) {
        self.cur_node = node;
        self.call_cands = [0; NSRC];
        self.call_work = [0.0; NSRC];
        self.call_secs = [0.0; NSRC];
        self.last_secs = SLT.with(|t| *t.borrow());
        self.off = 0;
        if !self.learn {
            return;
        }
        let w = node as usize;
        self.ncalls[w] += 1;
        let nw = tunable!("ENOMOTO_T_MIP_SEPLEARN_NODE_WARM", 0u32, u32);
        let warm = if node && nw > 0 { nw } else { tunable!("ENOMOTO_T_MIP_SEPLEARN_WARM", 3u32, u32) };
        // `ENOMOTO_T_MIP_SEPLEARN_ROOT_SKIP` = 0: 根では飛ばさない (候補の上限だけ)。misc07 は根の最初の 3 ラウンドで zerohalf の伸びが 0 と
        // 出て飛ばし、根の下界が弱くなった (既定では根の伸びの 63% が zerohalf)
        let no_skip = !node && tunable!("ENOMOTO_T_MIP_SEPLEARN_ROOT_SKIP", 1u32, u32) == 0;
        let thr = tunable!("ENOMOTO_T_MIP_SEPLEARN_SKIP", 0.1, f64);
        let workf = tunable!("ENOMOTO_T_MIP_SEPLEARN_WORKF", 3.0, f64);
        let maxskip = tunable!("ENOMOTO_T_MIP_SEPLEARN_MAXSKIP", 16u32, u32);
        let sc = self.scores(w);
        let ssum: f64 = LEARN_SRCS.iter().filter_map(|&s| sc[s as usize]).sum();
        let wsum: f64 = LEARN_SRCS.iter().filter(|&&s| sc[s as usize].is_some()).map(|&s| self.ema_work[w][s as usize]).sum();
        for &s in &LEARN_SRCS {
            let k = s as usize;
            if s == sepprof::SRC_TAB {
                continue;
            }
            if self.skip_left[w][k] > 0 {
                self.skip_left[w][k] -= 1;
                self.off |= 1 << k;
                self.st[w][k].skipped += 1;
                continue;
            }
            let Some(score) = sc[k] else { continue };
            if self.nobs[w][k] < warm || no_skip {
                continue;
            }
            let wshare = if wsum > 0.0 { self.ema_work[w][k] / wsum } else { 0.0 };
            let sshare = if ssum > 0.0 { score / ssum } else { 0.0 };
            let bad = score < thr || wshare > workf * sshare + 0.05;
            if bad {
                // 飛ばす長さを倍にして (最初は 2)、この呼び出しから飛ばす。飛ばし終えた次の呼び出しで 1 回試す
                let l = if self.skip_len[w][k] == 0 { 2 } else { (2 * self.skip_len[w][k]).min(maxskip) };
                self.skip_len[w][k] = l;
                self.skip_left[w][k] = l - 1;
                self.off |= 1 << k;
                self.st[w][k].skipped += 1;
            } else {
                self.skip_len[w][k] = 0;
            }
        }
    }

    /// 分離の後の候補を数え (分離器ごと)、学習なら候補の上限をかける。`budget` は 1 ラウンドに採る本数の上限
    pub(super) fn after_separate(&mut self, srcs: &mut [Vec<(u8, f64, usize)>], budget: usize) -> Vec<Vec<bool>> {
        // srcs: 候補の (分離器、効き目、非零の数) の並び (候補の束ごと)。返り値は残す印
        let w = self.cur_node as usize;
        let now = SLT.with(|t| *t.borrow());
        for k in 0..NSRC {
            self.call_secs[k] += now[k] - self.last_secs[k];
        }
        self.last_secs = now;
        let mut cnt = [0u64; NSRC];
        let mut nnz = [0f64; NSRC];
        for v in srcs.iter() {
            for &(s, _, z) in v {
                let k = (s % sepprof::SRC_FACE) as usize;
                cnt[k] += 1;
                nnz[k] += z as f64;
            }
        }
        let mut keep: Vec<Vec<bool>> = srcs.iter().map(|v| vec![true; v.len()]).collect();
        if self.learn {
            let sc = self.scores(w);
            let ssum: f64 = LEARN_SRCS.iter().filter_map(|&s| sc[s as usize]).sum();
            let mult = tunable!("ENOMOTO_T_MIP_SEPLEARN_CAPMULT", 2.0, f64);
            let cmin = tunable!("ENOMOTO_T_MIP_SEPLEARN_CAPMIN", 20usize, usize);
            let warm = tunable!("ENOMOTO_T_MIP_SEPLEARN_WARM", 3u32, u32);
            for &s in &LEARN_SRCS {
                let k = s as usize;
                if s == sepprof::SRC_TAB && !self.tab {
                    continue;
                }
                let Some(score) = sc[k] else { continue };
                // 得点が `ENOMOTO_T_MIP_SEPLEARN_NOCAP` (既定 0.5) 以上の分離器には上限をかけない
                if self.nobs[w][k] < warm || ssum <= 0.0 || score >= tunable!("ENOMOTO_T_MIP_SEPLEARN_NOCAP", 0.5, f64) {
                    continue;
                }
                let cap = ((score / ssum) * mult * budget as f64).ceil().max(cmin as f64) as usize;
                if (cnt[k] as usize) <= cap {
                    continue;
                }
                // 効き目の大きい順 (同じなら並びの順) に cap 本残す
                let mut ord: Vec<(f64, usize, usize)> = Vec::new();
                for (b, v) in srcs.iter().enumerate() {
                    for (i, &(s2, e, _)) in v.iter().enumerate() {
                        if (s2 % sepprof::SRC_FACE) as usize == k {
                            ord.push((e, b, i));
                        }
                    }
                }
                ord.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
                for &(_, b, i) in &ord[cap..] {
                    keep[b][i] = false;
                    nnz[k] -= srcs[b][i].2 as f64;
                }
                self.st[w][k].capped += (ord.len() - cap) as u64;
                cnt[k] = cap as u64;
            }
        }
        for k in 0..NSRC {
            self.call_cands[k] += cnt[k];
            self.call_work[k] += nnz[k];
            self.st[w][k].cands += cnt[k];
        }
        keep
    }

    /// 選んだカットを LP に加える直前: 加える前の目的値 `obj0` と、(分離器、違反) の並び
    pub(super) fn prepare(&mut self, obj0: f64, items: Vec<(u8, f64)>) {
        self.pending_prep = Some((obj0, self.cur_node, items));
    }

    /// 加えた直後: 加えたカットの番号は `next_id - k .. next_id` (k = 加えた本数)
    pub(super) fn bind(&mut self, next_id: u64) {
        let Some((obj0, node, items)) = self.pending_prep.take() else { return };
        let w = node as usize;
        let k = items.len() as u64;
        let id0 = next_id.saturating_sub(k);
        let mut nch = [0u64; NSRC];
        let its: Vec<(u64, u8, f64)> = items.iter().enumerate().map(|(i, &(s, v))| (id0 + i as u64, s % sepprof::SRC_FACE, v)).collect();
        for &(id, s, _) in &its {
            nch[s as usize] += 1;
            self.track.push_back((id, s, self.lp_seq, node));
        }
        for kk in 0..NSRC {
            self.st[w][kk].chosen += nch[kk];
        }
        self.pending = Some(Pending { obj0, node, items: its });
    }

    /// 選んだカットの非零の数を仕事に足す (LP を重くする分)
    pub(super) fn add_chosen_nnz(&mut self, src: u8, nnz: usize) {
        let k = (src % sepprof::SRC_FACE) as usize;
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
            let mut wts: Vec<f64> = pd.items.iter().zip(&ys).map(|(e, y)| y.unwrap_or(0.0) * e.2.max(0.0)).collect();
            let mut tot: f64 = wts.iter().sum();
            if tot <= 0.0 {
                wts = pd.items.iter().map(|e| e.2.max(0.0)).collect();
                tot = wts.iter().sum();
            }
            let mut g = [0f64; NSRC];
            if tot > 0.0 && delta > 0.0 {
                for (e, wt) in pd.items.iter().zip(&wts) {
                    g[e.1 as usize] += delta * wt / tot;
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
            let es: Vec<(u64, u8, u64, bool)> = self.track.drain(..due).collect();
            let ids: Vec<u64> = es.iter().map(|e| e.0).collect();
            let ys = if obj.is_some() { dual_of(&ids) } else { vec![None; ids.len()] };
            let a = alpha();
            for (e, y) in es.iter().zip(ys) {
                let w = e.3 as usize;
                let k = e.1 as usize;
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
            let ran = self.call_cands[k] > 0 || self.call_secs[k] > 0.0;
            if !ran {
                continue;
            }
            let work = if self.use_time { self.call_secs[k] * 1e6 } else { self.call_work[k] } + 1.0;
            self.st[w][k].calls += 1;
            self.st[w][k].secs += self.call_secs[k];
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
    }

    /// 候補が 1 本も採られずに終わった呼び出し (加えなかった) を伸び 0 で観測に入れる
    pub(super) fn end_call_without_add(&mut self) {
        let w = self.cur_node as usize;
        let g = [0f64; NSRC];
        let now = SLT.with(|t| *t.borrow());
        for k in 0..NSRC {
            self.call_secs[k] += now[k] - self.last_secs[k];
        }
        self.last_secs = now;
        self.observe(w, &g);
    }

    /// 表 (`ENOMOTO_MIP_LOG`)
    pub(super) fn report(&self, tag: &str) -> String {
        let mut s = String::new();
        for (w, wn) in [(0usize, "root"), (1, "node")] {
            let tg: f64 = (0..NSRC).map(|k| self.st[w][k].gain).sum();
            let ts: f64 = (0..NSRC).map(|k| self.st[w][k].secs).sum();
            if self.st[w].iter().all(|e| e.cands == 0 && e.chosen == 0) {
                continue;
            }
            s += &format!("SEPLEARN {tag} {wn}: gain {tg:.6e} sep secs {ts:.3}\n");
            let sc = self.scores(w);
            for k in 0..NSRC {
                let e = &self.st[w][k];
                if e.cands == 0 && e.chosen == 0 && e.skipped == 0 {
                    continue;
                }
                s += &format!(
                    "SEPLEARN {tag} {wn} {:<10} calls {} cands {} chosen {} gain {:.4e} ({:.1}%) surv {}/{} ({:.0}%) secs {:.3} work {:.0} skipped {} capped {} score {}\n",
                    sepprof::SRC_NAMES[k],
                    e.calls,
                    e.cands,
                    e.chosen,
                    e.gain,
                    if tg > 0.0 { 100.0 * e.gain / tg } else { 0.0 },
                    e.surv_ok,
                    e.surv_n,
                    if e.surv_n > 0 { 100.0 * e.surv_ok as f64 / e.surv_n as f64 } else { 0.0 },
                    e.secs,
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
