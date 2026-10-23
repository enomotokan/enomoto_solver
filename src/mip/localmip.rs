//! Local-MIP (Lin・Zou・Cai, CP 2024「Local-MIP: Efficient Local Search for Mixed Integer Programming」) 風の局所探索
//! (`ENOMOTO_MIP_LOCALMIP`、試作・既定では使わない)。論文は手元にないので、覚えている範囲の手順で組んだ。論文どおりの所:
//!
//! - **tight move** (mixed tight move): 違反している行を 1 本とり、その行の列 1 本を「その行をちょうど満たす境界 (等式なら右辺)」まで
//!   動かす。整数列は満たす向きに丸める。FJ の「跳び先」(その列の全部の行の重み付き違反の和を最小にする値) とは違い、1 本の行だけを見る。
//! - **スコア**: 動かした列の各行について、違反が減れば +重み、増えれば -重み (量ではなく向きで数える)。暫定解があれば目的の行
//!   (`c x <= 暫定解の値 - 刻み`) も 1 本の行として同じに数える。同点は breakthrough bonus (動かした後の目的値が暫定解より良い)
//!   で崩す。
//! - **BMS (best from multiple selections)**: 違反行を `SAMPLE_ROWS` 本選び、その行の手から最良のものを選ぶ。
//! - **タブー**: 列を増やしたら一定の手数は減らさない (逆も)。
//! - **重み**: 正の手がなければ、確率 `sp` で満たされた行の重みを 1 減らし (平滑化)、そうでなければ違反行の重みを 1 増やす。
//!   そのあと違反行を 1 本乱数で選び、その行の手の最良のもの (タブーを無視) を動かす (random walk)。
//! - **lift move**: 違反がなければ、行を全部満たしたまま目的が良くなる方向へ列を動かせる幅 (lift の幅) の端まで動かす。
//!   良くなる lift move がなくなったら、その点を暫定解として目的の行を付け (breakthrough)、違反のある状態から続ける。
//! - **再出発**: 一定の手数よくならなければ、暫定解 (なければ乱数で崩した出発点) から重みを戻してやり直す。
//!
//! 論文と違う (と思う) 所: lift move は「最良の 1 手」ではなく、費用のある列を乱数の順に見て良くなる手をその場で当てる掃引にした。
//! 論文の robust bonus (満たされた行の余裕を広げる手の加点) は入れていない。満たされた行からの tight move も入れていない。
//!
//! `ENOMOTO_MIP_LOCALMIP_PAIR` (試作): 違反行を満たす列 `j` の tight move が別の行 `r` を新たに破るとき、`r` を元に戻す
//! 2 本目の列 `k` (`r` に現れ、違反行には現れない) を同時に動かす組の手も候補に入れる (1 列ずつの探索では抜けられない形への手当て)。

use std::time::{Duration, Instant};

use super::problem::MipProblem;

const TOL: f64 = 1e-6;

/// [`local_mip`] の結果。
pub(super) struct LocalMipResult {
    /// 目的値の最良の実行可能解
    pub best: Option<Vec<f64>>,
    /// 違反行の数が最も少なかった点の違反行の数 (実行可能解がなければ)
    pub least_viol: usize,
    /// 最初の実行可能解を見つけた時刻 (秒、この関数の始めから)
    pub first_time: Option<f64>,
    pub steps: u64,
    pub pair_moves: u64,
    /// 連鎖の手で動かした列の数の合計
    pub chain_len: u64,
}

struct St<'a> {
    p: &'a MipProblem,
    lo: Vec<f64>,
    up: Vec<f64>,
    x: Vec<f64>,
    /// 行の左辺 (index m は目的の行)
    act: Vec<f64>,
    /// 行の重み (index m は目的の行)
    w: Vec<f64>,
    /// 目的の行の上限 (暫定解の値 - 刻み、暫定解がなければ +inf)
    obj_up: f64,
    vset: Vec<usize>,
    vpos: Vec<usize>,
    /// 列 j を減らす (0)・増やす (1) のを禁じる手数の終わり
    tabu: Vec<[u64; 2]>,
    rng: u64,
    /// 目的の行の列 (費用の非零)
    obj_cols: Vec<(usize, f64)>,
}

impl ChainView for St<'_> {
    fn prob(&self) -> &MipProblem {
        self.p
    }
    fn x(&self, j: usize) -> f64 {
        self.x[j]
    }
    fn lo(&self, j: usize) -> f64 {
        self.lo[j]
    }
    fn up(&self, j: usize) -> f64 {
        self.up[j]
    }
    fn act(&self, r: usize) -> f64 {
        self.act[r]
    }
    fn weight(&self, r: usize) -> f64 {
        self.w[r]
    }
    fn viol_at(&self, r: usize, a: f64) -> f64 {
        St::viol_at(self, r, a)
    }
    fn for_col(&self, j: usize, f: &mut dyn FnMut(usize, f64)) {
        St::for_col(self, j, |r, b| f(r, b))
    }
    fn row(&self, r: usize) -> &[(usize, f64)] {
        St::row(self, r)
    }
    fn tight_value(&self, i: usize, j: usize, a: f64) -> Option<f64> {
        St::tight_value(self, i, j, a)
    }
    fn below(&mut self, k: usize) -> usize {
        St::below(self, k)
    }
    fn magnitude(&self) -> bool {
        false
    }
}

impl St<'_> {
    #[inline]
    fn rand_u64(&mut self) -> u64 {
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    #[inline]
    fn uniform(&mut self) -> f64 {
        ((self.rand_u64() >> 11) as f64) / ((1u64 << 53) as f64)
    }
    #[inline]
    fn below(&mut self, k: usize) -> usize {
        (self.rand_u64() % k.max(1) as u64) as usize
    }

    #[inline]
    fn bounds(&self, i: usize) -> (f64, f64) {
        if i < self.p.m { (self.p.row_lo[i], self.p.row_up[i]) } else { (f64::NEG_INFINITY, self.obj_up) }
    }

    #[inline]
    fn viol_at(&self, i: usize, a: f64) -> f64 {
        let (l, u) = self.bounds(i);
        // 許容誤差は `MipProblem::is_feasible` (1e-6 + 1e-9 |b|) より少し狭くする
        if a > u + 0.5 * TOL + 1e-10 * u.abs() {
            a - u
        } else if a < l - 0.5 * TOL - 1e-10 * l.abs() {
            l - a
        } else {
            0.0
        }
    }

    #[inline]
    fn row(&self, i: usize) -> &[(usize, f64)] {
        if i < self.p.m { &self.p.rows[i] } else { &self.obj_cols }
    }

    fn set_viol(&mut self, r: usize) {
        let v = self.viol_at(r, self.act[r]) > 0.0;
        if v && self.vpos[r] == usize::MAX {
            self.vpos[r] = self.vset.len();
            self.vset.push(r);
        } else if !v && self.vpos[r] != usize::MAX {
            let k = self.vpos[r];
            let last = *self.vset.last().unwrap();
            self.vset.swap_remove(k);
            if last != r {
                self.vpos[last] = k;
            }
            self.vpos[r] = usize::MAX;
        }
    }

    fn reset(&mut self) {
        let m = self.p.m;
        for i in 0..m {
            self.act[i] = self.p.rows[i].iter().map(|&(j, a)| a * self.x[j]).sum();
        }
        self.act[m] = self.obj_cols.iter().map(|&(j, c)| c * self.x[j]).sum();
        self.vset.clear();
        self.vpos.iter_mut().for_each(|v| *v = usize::MAX);
        for i in 0..=m {
            self.set_viol(i);
        }
    }

    /// 列 j の (行, 係数) を目的の行を含めて回す
    fn for_col(&self, j: usize, mut f: impl FnMut(usize, f64)) {
        for &(r, b) in &self.p.cols[j] {
            f(r, b);
        }
        let c = self.p.cost[j];
        if c != 0.0 && self.obj_up.is_finite() {
            f(self.p.m, c);
        }
    }

    /// 違反行 i の列 j (係数 a) の tight move の新しい値。動けなければ None
    fn tight_value(&self, i: usize, j: usize, a: f64) -> Option<f64> {
        let (l, u) = self.bounds(i);
        let act = self.act[i];
        let target = if act > u { u } else if act < l { l } else { return None };
        let mut d = (target - act) / a;
        if self.p.is_int[j] {
            d = if d > 0.0 { (d - 1e-9).ceil() } else { (d + 1e-9).floor() };
        }
        let v = (self.x[j] + d).clamp(self.lo[j], self.up[j]);
        (v.is_finite() && (v - self.x[j]).abs() > 1e-12).then_some(v)
    }

    /// 手 (列 j を d だけ動かす) のスコア: (向きで数えた重み付きの進み, 目的の行が満たされるか)
    fn score(&self, j: usize, d: f64) -> f64 {
        let mut s = 0.0;
        self.for_col(j, |r, b| {
            let before = self.viol_at(r, self.act[r]);
            let after = self.viol_at(r, self.act[r] + b * d);
            if after < before - 1e-12 {
                s += self.w[r];
            } else if after > before + 1e-12 {
                s -= self.w[r];
            }
        });
        s
    }

    /// 2 列の手のスコア (同じ行に両方現れる場合も正しく数える)
    fn score2(&self, j: usize, d: f64, k: usize, e: f64, buf: &mut Vec<(usize, f64)>) -> f64 {
        buf.clear();
        self.for_col(j, |r, b| buf.push((r, b * d)));
        self.for_col(k, |r, b| buf.push((r, b * e)));
        buf.sort_unstable_by_key(|t| t.0);
        let mut s = 0.0;
        let mut q = 0;
        while q < buf.len() {
            let r = buf[q].0;
            let mut delta = 0.0;
            while q < buf.len() && buf[q].0 == r {
                delta += buf[q].1;
                q += 1;
            }
            let before = self.viol_at(r, self.act[r]);
            let after = self.viol_at(r, self.act[r] + delta);
            if after < before - 1e-12 {
                s += self.w[r];
            } else if after > before + 1e-12 {
                s -= self.w[r];
            }
        }
        s
    }

    fn breakthrough(&self, j: usize, d: f64) -> bool {
        self.obj_up.is_finite() && self.act[self.p.m] + self.p.cost[j] * d <= self.obj_up + TOL
    }

    fn apply(&mut self, j: usize, v: f64, step: u64) {
        let d = v - self.x[j];
        if d == 0.0 {
            return;
        }
        self.x[j] = v;
        let tt = step + 3 + self.below(10) as u64;
        // 増やしたら減らすのを、減らしたら増やすのを禁じる
        self.tabu[j][if d > 0.0 { 0 } else { 1 }] = tt;
        let m = self.p.m;
        for k in 0..self.p.cols[j].len() {
            let (r, b) = self.p.cols[j][k];
            self.act[r] += b * d;
            self.set_viol(r);
        }
        let c = self.p.cost[j];
        if c != 0.0 {
            self.act[m] += c * d;
            self.set_viol(m);
        }
    }

    #[inline]
    fn tabu_ok(&self, j: usize, d: f64, step: u64) -> bool {
        self.tabu[j][if d > 0.0 { 1 } else { 0 }] <= step
    }

    /// 連鎖の手 ([`chain_move`]) を、向きで数えるスコアで作る。
    fn chain_move(&mut self, i: usize, max_depth: usize, max_cand: usize, set_of: &[Vec<u32>]) -> Option<(f64, Vec<(usize, f64)>)> {
        chain_move(self, i, max_depth, max_cand, set_of).map(|(sc, mv, _)| (sc, mv))
    }

    /// lift move の掃引: 費用のある列を乱数の順に見て、全部の行を満たしたまま目的が良くなる幅の端まで動かす。動かした数を返す
    fn lift_sweep(&mut self, order: &mut [usize], step: u64) -> usize {
        let n = order.len();
        for k in (1..n).rev() {
            let r = self.below(k + 1);
            order.swap(k, r);
        }
        let mut moved = 0;
        for &j in order.iter() {
            let c = self.p.cost[j];
            if c == 0.0 || self.lo[j] == self.up[j] {
                continue;
            }
            let (mut dl, mut du) = (self.lo[j] - self.x[j], self.up[j] - self.x[j]);
            for &(r, b) in &self.p.cols[j] {
                let (l, u) = (self.p.row_lo[r], self.p.row_up[r]);
                let (sl, su) = (l - self.act[r], u - self.act[r]);
                let (a1, a2) = if b > 0.0 { (sl / b, su / b) } else { (su / b, sl / b) };
                dl = dl.max(a1);
                du = du.min(a2);
            }
            if self.p.is_int[j] {
                dl = (dl - 1e-9).ceil();
                du = (du + 1e-9).floor();
            }
            let d = if c > 0.0 { dl.min(0.0) } else { du.max(0.0) };
            if !d.is_finite() || d.abs() < 1e-9 || dl > du {
                continue;
            }
            let v = self.x[j] + d;
            self.apply(j, v, step);
            moved += 1;
        }
        moved
    }
}

/// Local-MIP 風の局所探索で実行可能な点を探す。`lo`/`up` は列の境界、`time_cap` は秒、`stop` が立ったら止める。
/// `on_sol` は実行可能解が良くなるたびに呼ぶ (別スレッドから本体へ渡す)。
pub(super) fn local_mip(p: &MipProblem, lo: &[f64], up: &[f64], time_cap: f64, seed: u64, stop: Option<&std::sync::atomic::AtomicBool>, on_sol: &dyn Fn(&[f64])) -> LocalMipResult {
    local_mip_opts(p, lo, up, time_cap, seed, stop, on_sol, env_str!("ENOMOTO_MIP_LOCALMIP_CHAIN").is_some())
}

/// [`local_mip`] で、連鎖の手を使うかを `chain` で決めるもの。
#[allow(clippy::too_many_arguments)]
pub(super) fn local_mip_opts(p: &MipProblem, lo: &[f64], up: &[f64], time_cap: f64, seed: u64, stop: Option<&std::sync::atomic::AtomicBool>, on_sol: &dyn Fn(&[f64]), chain: bool) -> LocalMipResult {
    let (n, m) = (p.n, p.m);
    let t0 = Instant::now();
    let t_end = t0 + Duration::from_secs_f64(time_cap.max(0.0));
    let mut res = LocalMipResult { best: None, least_viol: usize::MAX, first_time: None, steps: 0, pair_moves: 0, chain_len: 0 };
    if n == 0 {
        return res;
    }
    let mut lb = lo.to_vec();
    let mut ub = up.to_vec();
    for j in 0..n {
        if p.is_int[j] {
            lb[j] = (lb[j] - 1e-6).ceil();
            ub[j] = (ub[j] + 1e-6).floor();
        }
        if !(lb[j] <= ub[j]) {
            return res;
        }
    }
    let start: Vec<f64> = (0..n).map(|j| if lb[j].is_finite() { lb[j] } else if ub[j].is_finite() { ub[j] } else { 0.0 }).collect();
    let obj_cols: Vec<(usize, f64)> = (0..n).filter(|&j| p.cost[j] != 0.0).map(|j| (j, p.cost[j])).collect();
    let mut s = St {
        p,
        lo: lb,
        up: ub,
        x: start.clone(),
        act: vec![0.0; m + 1],
        w: vec![1.0; m + 1],
        obj_up: f64::INFINITY,
        vset: Vec::new(),
        vpos: vec![usize::MAX; m + 1],
        tabu: vec![[0, 0]; n],
        rng: seed | 1,
        obj_cols,
    };
    s.reset();
    let sample_rows = tunable!("ENOMOTO_T_MIP_LOCALMIP_SAMPLE_ROWS", 3usize, usize).max(1);
    let sample_cols = tunable!("ENOMOTO_T_MIP_LOCALMIP_SAMPLE_COLS", 64usize, usize).max(1);
    let sp = tunable!("ENOMOTO_T_MIP_LOCALMIP_SP", 0.0003, f64);
    let restart_after = tunable!("ENOMOTO_T_MIP_LOCALMIP_RESTART", 300_000u64, u64);
    let pair = env_str!("ENOMOTO_MIP_LOCALMIP_PAIR").is_some();
    let chain_depth = tunable!("ENOMOTO_T_MIP_LOCALMIP_CHAIN_DEPTH", 32usize, usize);
    let mut chain_tries = 0u64;
    let set_of: Vec<Vec<u32>> = if chain {
        set_rows(p, &s.lo, &s.up)
    } else {
        Vec::new()
    };
    let pair_cols = tunable!("ENOMOTO_T_MIP_LOCALMIP_PAIR_COLS", 8usize, usize).max(1);
    let debug = env_str!("ENOMOTO_MIP_LOCALMIP_DEBUG").is_some();
    let mut lift_order: Vec<usize> = (0..n).filter(|&j| p.cost[j] != 0.0).collect();
    let mut best_obj = f64::INFINITY;
    let mut since_improve = 0u64;
    let mut round_best = usize::MAX;
    let mut cand: Vec<(usize, f64)> = Vec::new();
    let mut buf: Vec<(usize, f64)> = Vec::new();
    let mut restarts = 0u64;
    let mut step = 0u64;
    loop {
        step += 1;
        if step % 256 == 0 && (Instant::now() >= t_end || stop.is_some_and(|f| f.load(std::sync::atomic::Ordering::Relaxed))) {
            if debug {
                eprintln!("MIP:   [localmip] time/stop at step {step}");
            }
            break;
        }
        // 実行可能 (目的の行を除いて違反なし)
        let nviol = s.vset.len() - usize::from(s.vpos[m] != usize::MAX);
        if nviol == 0 {
            // lift move で目的を良くし、良くならなくなったら暫定解として記録して目的の行を締める (breakthrough)
            while s.lift_sweep(&mut lift_order, step) > 0 {}
            if s.vset.iter().any(|&r| r < m) {
                // 丸めの誤差で違反が出た (起きないはず): そのまま探索を続ける
                continue;
            }
            let z = s.act[m];
            if !p.is_feasible(&s.x, 1e-6) {
                // 左辺の誤差で違反を見落とした: 数え直して、それでも違反がなければ止める
                s.reset();
                if s.vset.iter().any(|&r| r < m) {
                    continue;
                }
                if debug {
                    eprintln!("MIP:   [localmip] a point without violated rows is not feasible: stopped");
                }
                break;
            }
            if !best_obj.is_finite() || z < best_obj - 1e-9 * best_obj.abs().max(1.0) {
                best_obj = z;
                res.best = Some(s.x.clone());
                on_sol(&s.x);
                res.first_time.get_or_insert(t0.elapsed().as_secs_f64());
                since_improve = 0;
                if debug {
                    eprintln!("MIP:   [localmip] feasible {z:.6e} at step {step} ({:.2}s)", t0.elapsed().as_secs_f64());
                }
            }
            let ostep = super::heuristics::ls_obj_step_pub(p, best_obj);
            s.obj_up = best_obj - ostep;
            if s.obj_cols.is_empty() {
                break;
            }
            s.set_viol(m);
            if s.vset.is_empty() {
                if debug {
                    eprintln!("MIP:   [localmip] objective row not violated after a feasible point (obj_up {:.6e}, z {z:.6e}): stopped", s.obj_up);
                }
                break;
            }
        }
        if nviol < res.least_viol {
            res.least_viol = nviol;
        }
        if nviol < round_best {
            round_best = nviol;
            since_improve = 0;
        } else {
            since_improve += 1;
        }
        if since_improve > restart_after {
            // 再出発: 暫定解 (なければ出発点を乱数で崩した点) から、重みを戻す
            restarts += 1;
            s.x = match &res.best {
                Some(b) => b.clone(),
                None => {
                    let mut x = start.clone();
                    for j in 0..n {
                        if p.is_int[j] && s.lo[j].is_finite() && s.up[j].is_finite() && s.uniform() < 0.1 {
                            x[j] = (s.lo[j] + ((s.up[j] - s.lo[j] + 1.0) * s.uniform()).floor()).min(s.up[j]);
                        }
                    }
                    x
                }
            };
            s.w.iter_mut().for_each(|w| *w = 1.0);
            s.tabu.iter_mut().for_each(|t| *t = [0, 0]);
            s.reset();
            since_improve = 0;
            round_best = usize::MAX;
            continue;
        }
        // BMS: 違反行を何本か選び、その行の tight move (タブーでないもの) の最良を選ぶ
        let mut best: Option<(f64, bool, usize, f64, Option<(usize, f64)>)> = None;
        for _ in 0..sample_rows.min(s.vset.len()) {
            let kk = s.below(s.vset.len());
            let i = s.vset[kk];
            let len = s.row(i).len();
            let k = len.min(sample_cols);
            for t in 0..k {
                let q = if len <= sample_cols { t } else { s.below(len) };
                let (j, a) = s.row(i)[q];
                let Some(v) = s.tight_value(i, j, a) else { continue };
                let d = v - s.x[j];
                if !s.tabu_ok(j, d, step) {
                    continue;
                }
                let sc = s.score(j, d);
                let bt = s.breakthrough(j, d);
                if best.as_ref().is_none_or(|b| sc > b.0 || (sc == b.0 && bt && !b.1)) {
                    best = Some((sc, bt, j, v, None));
                }
            }
        }
        // 連鎖の手: 最良の 1 列の手が正でなければ
        if chain && best.as_ref().is_none_or(|b| b.0 <= 0.0) && !s.vset.is_empty() {
            let kk = s.below(s.vset.len());
            let i = s.vset[kk];
            chain_tries += 1;
            if let Some((_, mv)) = s.chain_move(i, chain_depth, sample_cols, &set_of) {
                for &(k, v) in &mv {
                    s.apply(k, v, step);
                }
                res.pair_moves += 1;
                res.chain_len += mv.len() as u64;
                continue;
            }
        }
        // 組の手: 最良の 1 列の手が正でなければ、違反行の列の手が新たに破る行を戻す 2 本目の列を探す
        if pair && best.as_ref().is_none_or(|b| b.0 <= 0.0) && !s.vset.is_empty() {
            let kk = s.below(s.vset.len());
            let i = s.vset[kk];
            cand.clear();
            let len = s.row(i).len();
            for t in 0..len.min(sample_cols) {
                let q = if len <= sample_cols { t } else { s.below(len) };
                let (j, a) = s.row(i)[q];
                if let Some(v) = s.tight_value(i, j, a) {
                    cand.push((j, v));
                }
            }
            for &(j, v) in cand.iter().take(pair_cols) {
                let d = v - s.x[j];
                // j を動かして新たに破れる行 (目的の行は除く) を 1 本選ぶ
                let mut broken: Option<(usize, f64)> = None;
                for &(r, b) in &p.cols[j] {
                    if r != i && s.viol_at(r, s.act[r] + b * d) > s.viol_at(r, s.act[r]) + 1e-12 {
                        broken = Some((r, b));
                        break;
                    }
                }
                let Some((r, b)) = broken else { continue };
                let (l, u) = s.bounds(r);
                let na = s.act[r] + b * d;
                let target = if na > u { u } else { l };
                for &(k, c) in p.rows[r].iter() {
                    if k == j || s.lo[k] == s.up[k] {
                        continue;
                    }
                    let mut e = (target - na) / c;
                    if p.is_int[k] {
                        e = if e > 0.0 { (e - 1e-9).ceil() } else { (e + 1e-9).floor() };
                    }
                    let vk = (s.x[k] + e).clamp(s.lo[k], s.up[k]);
                    let e = vk - s.x[k];
                    if e.abs() < 1e-12 || !s.tabu_ok(k, e, step) {
                        continue;
                    }
                    let sc = s.score2(j, d, k, e, &mut buf);
                    if best.as_ref().is_none_or(|bb| sc > bb.0) {
                        best = Some((sc, false, j, v, Some((k, vk))));
                    }
                }
            }
        }
        match best {
            Some((sc, _, j, v, second)) if sc > 0.0 => {
                s.apply(j, v, step);
                if let Some((k, vk)) = second {
                    s.apply(k, vk, step);
                    res.pair_moves += 1;
                }
            }
            _ => {
                // 正の手がない: 重みを更新して、違反行 1 本の手の最良を動かす (random walk)
                if s.uniform() < sp {
                    for r in 0..=m {
                        if s.vpos[r] == usize::MAX && s.w[r] > 1.0 {
                            s.w[r] -= 1.0;
                        }
                    }
                } else {
                    for q in 0..s.vset.len() {
                        let r = s.vset[q];
                        s.w[r] += 1.0;
                    }
                }
                if s.vset.is_empty() {
                    continue;
                }
                let kk = s.below(s.vset.len());
            let i = s.vset[kk];
                let mut bw: Option<(f64, usize, f64)> = None;
                let len = s.row(i).len();
                for t in 0..len.min(sample_cols) {
                    let q = if len <= sample_cols { t } else { s.below(len) };
                    let (j, a) = s.row(i)[q];
                    let Some(v) = s.tight_value(i, j, a) else { continue };
                    let sc = s.score(j, v - s.x[j]);
                    if bw.is_none_or(|b| sc > b.0) {
                        bw = Some((sc, j, v));
                    }
                }
                if let Some((_, j, v)) = bw {
                    s.apply(j, v, step);
                }
            }
        }
        if step % 100_000 == 0 {
            // 左辺の誤差をためないよう数え直す
            s.reset();
        }
    }
    res.steps = step;
    if debug {
        eprintln!(
            "MIP:   [localmip] {step} steps, {restarts} restarts, least violated rows {}, pair/chain moves {} (chain tries {chain_tries}, columns {}), found {} (first {:?}), {:.2}s",
            res.least_viol,
            res.pair_moves,
            res.chain_len,
            res.best.as_ref().map(|x| p.objective(x)).map_or("none".to_string(), |z| format!("{z:.6e}")),
            res.first_time,
            t0.elapsed().as_secs_f64()
        );
    }
    res
}

/// 集合分割の行 (0-1 列の和 = 1) を使った境界の締め (`ENOMOTO_MIP_FJ_SP_BOUNDS`、試作): 0-1 列が全部 1 本の集合分割 (か ≤ 1 の
/// 集合パッキング) の行に入っている行では、0-1 の部分の値は「どれか 1 本の係数」(か 0) しか取らないので、残りの 0-1 でない
/// 列が 1 本ならその境界を締める (30n20b8 の `sum_t t y_t - s = 0` で、t = 79〜81 の y が 0 に固定なら s >= 82)。
/// そのあと短い行 (10 列以下) の活動度で伝播する。締めた列の数を返す (矛盾したら何も変えず 0)。
pub(super) fn sp_implied_bounds(p: &mut MipProblem) -> usize {
    let (n, m) = (p.n, p.m);
    let free_bin = |p: &MipProblem, j: usize| p.is_int[j] && p.col_lo[j] == 0.0 && p.col_up[j] == 1.0;
    // 集合の行と、0-1 列ごとの集合の行の一覧
    let mut set_of: Vec<Vec<u32>> = vec![Vec::new(); n];
    let mut set_info: Vec<(bool, usize)> = Vec::new(); // (分割か, 自由な列の数)
    for i in 0..m {
        if p.row_up[i] != 1.0 || !(p.row_lo[i] == 1.0 || p.row_lo[i] <= 0.0) {
            continue;
        }
        let ok = p.rows[i].iter().all(|&(j, a)| a == 1.0 && (free_bin(p, j) || (p.col_lo[j] == 0.0 && p.col_up[j] == 0.0)));
        if !ok {
            continue;
        }
        let k = set_info.len() as u32;
        let nf = p.rows[i].iter().filter(|&&(j, _)| free_bin(p, j)).count();
        set_info.push((p.row_lo[i] == 1.0, nf));
        for &(j, _) in &p.rows[i] {
            if free_bin(p, j) {
                set_of[j].push(k);
            }
        }
    }
    if set_info.is_empty() {
        return 0;
    }
    let (mut lo, mut up) = (p.col_lo.clone(), p.col_up.clone());
    let tighten = |lo: &mut [f64], up: &mut [f64], j: usize, l: f64, u: f64, is_int: bool| -> bool {
        let (l, u) = if is_int { ((l - 1e-6).ceil(), (u + 1e-6).floor()) } else { (l, u) };
        let mut ch = false;
        if l > lo[j] + 1e-9 {
            lo[j] = l;
            ch = true;
        }
        if u < up[j] - 1e-9 {
            up[j] = u;
            ch = true;
        }
        ch
    };
    for i in 0..m {
        let mut bins: Vec<(usize, f64)> = Vec::new();
        let mut other: Option<(usize, f64)> = None;
        let mut n_other = 0;
        let mut c = 0.0;
        for &(j, a) in &p.rows[i] {
            if p.col_lo[j] == p.col_up[j] {
                c += a * p.col_lo[j];
            } else if free_bin(p, j) {
                bins.push((j, a));
            } else {
                n_other += 1;
                other = Some((j, a));
            }
        }
        if n_other != 1 || bins.len() < 2 {
            continue;
        }
        // 0-1 列を全部含む集合の行
        let cand = &set_of[bins[0].0];
        let Some(&s) = cand.iter().find(|&&s| bins.iter().all(|&(j, _)| set_of[j].contains(&s))) else { continue };
        let (part, nf) = set_info[s as usize];
        let mut mn = bins.iter().map(|t| t.1).fold(f64::INFINITY, f64::min);
        let mut mx = bins.iter().map(|t| t.1).fold(f64::NEG_INFINITY, f64::max);
        if !part || nf > bins.len() {
            mn = mn.min(0.0);
            mx = mx.max(0.0);
        }
        let (j, a) = other.unwrap();
        // row_lo <= c + bin + a x_j <= row_up
        let (rl, ru) = (p.row_lo[i] - c, p.row_up[i] - c);
        let (sl, su) = (rl - mx, ru - mn); // a x_j の範囲
        let (l, u) = if a > 0.0 { (sl / a, su / a) } else { (su / a, sl / a) };
        tighten(&mut lo, &mut up, j, l, u, p.is_int[j]);
    }
    // 短い行の活動度で伝播
    for _ in 0..20 {
        let mut ch = false;
        for i in 0..m {
            let r = &p.rows[i];
            if r.len() > 10 {
                continue;
            }
            for &(j, a) in r {
                let (mut mn, mut mx) = (0.0, 0.0);
                for &(k, b) in r {
                    if k == j {
                        continue;
                    }
                    let (x1, x2) = if b > 0.0 { (b * lo[k], b * up[k]) } else { (b * up[k], b * lo[k]) };
                    mn += x1;
                    mx += x2;
                }
                let (sl, su) = (p.row_lo[i] - mx, p.row_up[i] - mn);
                let (l, u) = if a > 0.0 { (sl / a, su / a) } else { (su / a, sl / a) };
                let (l, u) = (if l.is_nan() { f64::NEG_INFINITY } else { l }, if u.is_nan() { f64::INFINITY } else { u });
                if tighten(&mut lo, &mut up, j, l, u, p.is_int[j]) {
                    ch = true;
                }
            }
        }
        if !ch {
            break;
        }
    }
    if (0..n).any(|j| lo[j] > up[j] + 1e-9) {
        return 0;
    }
    let k = (0..n).filter(|&j| lo[j] != p.col_lo[j] || up[j] != p.col_up[j]).count();
    p.col_lo = lo;
    p.col_up = up;
    k
}

/// 連鎖の手を作るのに要る探索の状態 (Local-MIP と FJ で共有する)。行の番号は問題の行 (`0..m`)、Local-MIP では `m` が目的の行。
pub(super) trait ChainView {
    fn prob(&self) -> &MipProblem;
    fn x(&self, j: usize) -> f64;
    fn lo(&self, j: usize) -> f64;
    fn up(&self, j: usize) -> f64;
    /// 行の左辺
    fn act(&self, r: usize) -> f64;
    fn weight(&self, r: usize) -> f64;
    /// 左辺が `a` のときの行 `r` の違反量
    fn viol_at(&self, r: usize, a: f64) -> f64;
    /// 列 `j` の (行, 係数)
    fn for_col(&self, j: usize, f: &mut dyn FnMut(usize, f64));
    fn row(&self, r: usize) -> &[(usize, f64)];
    /// 違反行 `i` の列 `j` (係数 `a`) をその行をちょうど満たす値へ動かす値
    fn tight_value(&self, i: usize, j: usize, a: f64) -> Option<f64>;
    fn below(&mut self, k: usize) -> usize;
    /// スコアを違反量の重み付きの減りで数えるか (FJ)。偽なら向き (良くなれば +重み、悪くなれば -重み) で数える (Local-MIP)
    fn magnitude(&self) -> bool;
}

/// 連鎖の手 (`ENOMOTO_MIP_LOCALMIP_CHAIN`・`ENOMOTO_MIP_FJ_CHAIN`、試作): 違反行 `i` の tight move から始め、その手で新たに
/// 悪くなった行 (等式を先に、同じなら重みの大きい方) を元に戻す手 (その行の 1 列の tight move か、その行の 1 の 0-1 列を
/// 集合分割の行の中の 0 の列と入れ替える 2 列の手) を次々に足す (ejection chain)。途中でスコアの合計が最良だった所までの
/// 手を返す (正のときだけ)。`set_of` は 0-1 列ごとの集合分割 (パッキング) の行。3 つ目は評価で触った非零の数 (手間)。
/// 30n20b8 では 1 列ずつの探索が抜けられない「0-1 の時刻の選択と時刻の整数列を結ぶ等式 + precedence」の鎖をこれで直せる
pub(super) fn chain_move<V: ChainView + ?Sized>(v: &mut V, i: usize, max_depth: usize, max_cand: usize, set_of: &[Vec<u32>]) -> Option<(f64, Vec<(usize, f64)>, u64)> {
    use std::collections::HashMap;
    let m = v.prob().m;
    let mag = v.magnitude();
    let mut effort = 0u64;
    let mut newv: HashMap<usize, f64> = HashMap::new();
    let mut dl: HashMap<usize, f64> = HashMap::new();
    let mut moves: Vec<(usize, f64)> = Vec::new();
    // 行 r の、左辺の変化 d に対するスコアへの寄与
    let contrib = |v: &V, r: usize, d: f64| -> f64 {
        let before = v.viol_at(r, v.act(r));
        let after = v.viol_at(r, v.act(r) + d);
        if mag {
            v.weight(r) * (before - after)
        } else if after < before - 1e-12 {
            v.weight(r)
        } else if after > before + 1e-12 {
            -v.weight(r)
        } else {
            0.0
        }
    };
    // 手 ks を今の連鎖に足したときのスコアの変化
    let gain = |v: &V, dl: &HashMap<usize, f64>, newv: &HashMap<usize, f64>, ks: &[(usize, f64)], effort: &mut u64| -> f64 {
        let mut loc: HashMap<usize, f64> = HashMap::new();
        for &(k, val) in ks {
            let d = val - newv.get(&k).copied().unwrap_or(v.x(k));
            v.for_col(k, &mut |r, b| *loc.entry(r).or_insert(0.0) += b * d);
        }
        *effort += loc.len() as u64;
        let mut g = 0.0;
        for (&r, &dd) in &loc {
            let d0 = dl.get(&r).copied().unwrap_or(0.0);
            g += contrib(v, r, d0 + dd) - contrib(v, r, d0);
        }
        g
    };
    let push = |v: &V, dl: &mut HashMap<usize, f64>, newv: &mut HashMap<usize, f64>, moves: &mut Vec<(usize, f64)>, k: usize, val: f64| {
        let d = val - newv.get(&k).copied().unwrap_or(v.x(k));
        v.for_col(k, &mut |r, b| *dl.entry(r).or_insert(0.0) += b * d);
        newv.insert(k, val);
        moves.push((k, val));
    };
    // 始めの手: 行 i の tight move の最良
    let mut first: Option<(f64, usize, f64)> = None;
    let len = v.row(i).len();
    for t in 0..len.min(max_cand) {
        let q = if len <= max_cand { t } else { v.below(len) };
        let (j, a) = v.row(i)[q];
        if let Some(val) = v.tight_value(i, j, a) {
            let g = gain(v, &dl, &newv, &[(j, val)], &mut effort);
            if first.is_none_or(|f| g > f.0) {
                first = Some((g, j, val));
            }
        }
    }
    let (mut score, j0, v0) = first?;
    push(v, &mut dl, &mut newv, &mut moves, j0, v0);
    let mut best = (score, moves.len());
    let mut tried_rows: Vec<usize> = Vec::new();
    for _ in 0..max_depth {
        let p = v.prob();
        // 新たに悪くなった行
        let mut pick: Option<(bool, f64, usize, f64)> = None;
        for (&r, &d) in &dl {
            if r >= m || tried_rows.contains(&r) {
                continue;
            }
            if v.viol_at(r, v.act(r) + d) > v.viol_at(r, v.act(r)) + 1e-12 {
                let eq = p.row_lo[r] == p.row_up[r];
                let key = (eq, v.weight(r));
                if pick.is_none_or(|pk| key > (pk.0, pk.1) || (key == (pk.0, pk.1) && r < pk.2)) {
                    pick = Some((eq, v.weight(r), r, d));
                }
            }
        }
        let Some((_, _, r, d)) = pick else { break };
        tried_rows.push(r);
        let na = v.act(r) + d;
        let (need_lo, need_up) = (p.row_lo[r] - na, p.row_up[r] - na); // 左辺の変化の範囲
        let mut cands: Vec<Vec<(usize, f64)>> = Vec::new();
        let rlen = p.rows[r].len();
        for t in 0..rlen.min(max_cand) {
            let q = if rlen <= max_cand { t } else { v.below(rlen) };
            let p = v.prob();
            let (k, c) = p.rows[r][q];
            if newv.contains_key(&k) || v.lo(k) == v.up(k) {
                continue;
            }
            let xk = v.x(k);
            // 1 列の tight move
            let target = if need_lo > 0.0 { need_lo } else { need_up };
            let mut e = target / c;
            if p.is_int[k] {
                e = if e > 0.0 { (e - 1e-9).ceil() } else { (e + 1e-9).floor() };
            }
            let vk = (xk + e).clamp(v.lo(k), v.up(k));
            if (vk - xk).abs() > 1e-12 {
                cands.push(vec![(k, vk)]);
            }
            // 入れ替え: k が 1 の 0-1 列なら、同じ集合分割の行の 0 の列 k2 を 1 にして、行 r の変化が範囲に入るもの
            if xk == 1.0 {
                for &sr in &set_of[k] {
                    for &(k2, _) in &p.rows[sr as usize] {
                        if k2 == k || v.x(k2) != 0.0 || v.up(k2) < 1.0 || newv.contains_key(&k2) {
                            continue;
                        }
                        let c2 = p.rows[r].binary_search_by_key(&k2, |t| t.0).map(|ix| p.rows[r][ix].1).unwrap_or(0.0);
                        let ch = c2 - c;
                        if ch >= need_lo - 1e-9 && ch <= need_up + 1e-9 {
                            cands.push(vec![(k, 0.0), (k2, 1.0)]);
                        }
                    }
                }
            }
        }
        if cands.is_empty() {
            break;
        }
        let mut bc: Option<(f64, usize)> = None;
        for (ci, c) in cands.iter().enumerate() {
            let g = gain(v, &dl, &newv, c, &mut effort);
            if bc.is_none_or(|b| g > b.0) {
                bc = Some((g, ci));
            }
        }
        let (g, ci) = bc.unwrap();
        for &(k, val) in &cands[ci] {
            push(v, &mut dl, &mut newv, &mut moves, k, val);
        }
        score += g;
        if score > best.0 {
            best = (score, moves.len());
        }
    }
    moves.truncate(best.1);
    (best.0 > 1e-12).then_some((best.0, moves, effort))
}

/// 0-1 列ごとの集合分割 (パッキング) の行: 係数が全部 1、上限 1、0-1 列と 0 固定の列だけの行。
pub(super) fn set_rows(p: &MipProblem, lo: &[f64], up: &[f64]) -> Vec<Vec<u32>> {
    let fb = |j: usize| p.is_int[j] && lo[j] == 0.0 && up[j] == 1.0;
    let mut so = vec![Vec::new(); p.n];
    for i in 0..p.m {
        if p.row_up[i] == 1.0 && p.rows[i].iter().all(|&(j, a)| a == 1.0 && (fb(j) || up[j] <= 0.0)) {
            for &(j, _) in &p.rows[i] {
                if fb(j) {
                    so[j].push(i as u32);
                }
            }
        }
    }
    so
}
