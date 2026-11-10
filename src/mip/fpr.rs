//! 試作 `ENOMOTO_MIP_FPR`: fix-propagate-repair (Salvagnin, Roberti, Fischetti 2025「A fix-propagate-repair heuristic for
//! Mixed Integer Programming」, Math. Prog. Comp. 17(1)) の簡略版。
//!
//! 列を順に固定して伝播し (fix-and-propagate)、伝播が矛盾しても捨てずに、その場で WalkSAT 風の局所探索 (repair) で
//! 直してから固定を続ける。repair は伝播を使わず、「決めた列はその値、まだ決めていない列は (最初の伝播の後の) 定義域」
//! で見た行の活動量の範囲が行の範囲と交わらない行 (違反行) を 1 つ選び、その行の決めた列のうち、違反を減らすように
//! 値をずらす手を、重みつきの違反の増減 (被害) が最小のもの (確率 `noise` で無作為) から選ぶ (タブーつき)。違反行が
//! なくなったら、値の変わった最初の決定まで定義域を巻き戻して固定・伝播をやり直す (やり直しで矛盾した列は「まだ
//! 決めていない」に戻し、後でその値の近くからもう一度決める)。列の順の規則と値の選び方を何通りも持ち、ポートフォリオで回す。
//!
//! 使う場所 (`ENOMOTO_MIP_FPR` の文字):
//! - `t` (既定): 根の LP の前に別スレッドで、LP を使わない組み合わせを回す (全列を決める。連続列も最後に決める。連続列で
//!   行き詰まったら、整数列の割り当てを主のスレッドに渡し、主が整数列を固定して LP で連続部分を解く)。
//! - `r` (既定): 根のヒューリスティクス (暫定解がなければ。LP 解を値・順に使う。整数列だけ決め、連続部分は LP)。
//! - `b` (既定): 暫定解がない間の木の中のヒューリスティクスの腕 (バンディット `ENOMOTO_MIP_NOINC_BANDIT` の腕にもなる)。
//!
//! 論文からの違い・未確認の点 (論文の本文は読めていない。検索の抜粋による): 論文の repair の被害の物差し・重み・タブーの
//! 長さ・雑音の確率、列の順・値の規則の正確な一覧は未確認。ここでは FJ (Luteberger & Sartor 2023) 風の重み (行き詰まると
//! 違反行の重みを 1 増やす) とした。

use super::domain::Domain;
use super::problem::MipProblem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// `ENOMOTO_MIP_FPR` の文字 `c` を使うか (値が空・`1` なら `t`・`r`・`b` 全部)。
pub(crate) fn fpr_on(c: char) -> bool {
    match env_str!("ENOMOTO_MIP_FPR") {
        None => false,
        Some(v) if v.is_empty() || v == "1" => matches!(c, 't' | 'r' | 'b'),
        Some(v) => v.contains(c),
    }
}

/// 列を決める順の規則。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Order {
    /// 列の番号の順
    Input,
    /// 無作為
    Random,
    /// 列の非零の多い順 (多くの行に関わる列を先に)
    Degree,
    /// 定義域の狭い順 (0-1 列が先)、同じなら非零の多い順
    Domain,
    /// LP 解の整数に近い順 (LP 解があるときだけ)
    LpFrac,
}

/// 値の選び方。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Value {
    /// 目的の良い向きの端 (費用 0 なら lock の少ない向き)
    Obj,
    /// lock の少ない向きの端
    Locks,
    /// 無作為 (定義域が広ければ端のどちらか)
    Random,
    /// LP 解の丸め (LP 解があるときだけ)
    Lp,
    /// 0 に近い値
    Zero,
}

pub(crate) struct FprOut {
    /// 見つけた点 (`complete` なら全列が決まった実行可能解、そうでなければ整数列の割り当て (連続列は意味なし))
    pub x: Option<Vec<f64>>,
    pub complete: bool,
    pub repairs: u64,
    pub moves: u64,
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn f(&mut self) -> f64 {
        ((self.next() >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn below(&mut self, k: usize) -> usize {
        (self.next() % k.max(1) as u64) as usize
    }
}

fn row_tol(b: f64) -> f64 {
    1e-6 * (1.0 + b.abs())
}

/// 1 回の fix-propagate-repair の状態。`x` は全列の点 (決めた列はその値、まだ決めていない列は今の定義域の中の仮の値)。
struct Fpr<'p> {
    p: &'p MipProblem,
    /// 最初の伝播の後の定義域 (repair がずらす値の範囲)
    lo0: Vec<f64>,
    up0: Vec<f64>,
    decided: Vec<bool>,
    x: Vec<f64>,
    /// 行の活動量 (点 `x` での値)
    act: Vec<f64>,
    w: Vec<f64>,
    /// 違反行の集合 (位置の表つき)
    viol: Vec<usize>,
    vpos: Vec<usize>,
    tabu: Vec<u64>,
    step: u64,
    rng: Rng,
    moved: Vec<usize>,
    moved_mark: Vec<bool>,
}

impl<'p> Fpr<'p> {
    fn new(p: &'p MipProblem, lo0: Vec<f64>, up0: Vec<f64>, x: Vec<f64>, seed: u64) -> Self {
        let m = p.m;
        let mut s = Fpr {
            p,
            lo0,
            up0,
            decided: vec![false; p.n],
            x,
            act: vec![0.0; m],
            w: vec![1.0; m],
            viol: Vec::new(),
            vpos: vec![usize::MAX; m],
            tabu: vec![0; p.n],
            step: 0,
            rng: Rng(seed | 1),
            moved: Vec::new(),
            moved_mark: vec![false; p.n],
        };
        s.recompute();
        s
    }

    fn recompute(&mut self) {
        let p = self.p;
        for i in 0..p.m {
            self.act[i] = p.rows[i].iter().map(|&(j, a)| a * self.x[j]).sum();
        }
        self.viol.clear();
        self.vpos.iter_mut().for_each(|v| *v = usize::MAX);
        for i in 0..p.m {
            self.update_viol(i);
        }
    }

    /// 行 `i` の違反量 (行の大きさで割ったもの。0 なら違反なし)。`d` は活動量の仮の変化。
    fn viol_amount(&self, i: usize, d: f64) -> f64 {
        let p = self.p;
        let a = self.act[i] + d;
        let e = if a > p.row_up[i] + row_tol(p.row_up[i]) {
            (a - p.row_up[i]) / (1.0 + p.row_up[i].abs())
        } else if a < p.row_lo[i] - row_tol(p.row_lo[i]) {
            (p.row_lo[i] - a) / (1.0 + p.row_lo[i].abs())
        } else {
            0.0
        };
        // 違反の量だけでなく違反の有無も数える (WalkSAT の「満たされない節の数」に当たる部分)
        if e > 0.0 { 1.0 + e.min(1.0) } else { 0.0 }
    }

    fn update_viol(&mut self, i: usize) {
        let bad = self.viol_amount(i, 0.0) > 0.0;
        let inset = self.vpos[i] != usize::MAX;
        if bad && !inset {
            self.vpos[i] = self.viol.len();
            self.viol.push(i);
        } else if !bad && inset {
            let k = self.vpos[i];
            let last = *self.viol.last().unwrap();
            self.viol.swap_remove(k);
            if last != i {
                self.vpos[last] = k;
            }
            self.vpos[i] = usize::MAX;
        }
    }

    /// 列 `j` の値を `v` にし、行の活動量と違反行を更新する。
    fn set_val(&mut self, j: usize, v: f64) {
        let d = v - self.x[j];
        if d == 0.0 {
            return;
        }
        self.x[j] = v;
        let p = self.p;
        for &(i, a) in &p.cols[j] {
            self.act[i] += a * d;
            self.update_viol(i);
        }
    }

    /// 列 `j` を `nv` に動かしたときの、重みつきの違反の増減 (被害)。
    fn damage(&self, j: usize, nv: f64) -> f64 {
        let d = nv - self.x[j];
        let mut s = 0.0;
        for &(i, a) in &self.p.cols[j] {
            let old = self.viol_amount(i, 0.0);
            let new = self.viol_amount(i, a * d);
            s += self.w[i] * (new - old);
        }
        s
    }

    /// 違反行 `i` を直す向きに、列 `j` (係数 `a`) をずらした値 (動かないなら None)。
    fn shift_value(&self, i: usize, j: usize, a: f64) -> Option<f64> {
        let p = self.p;
        let need = if self.act[i] > p.row_up[i] + row_tol(p.row_up[i]) {
            p.row_up[i] - self.act[i]
        } else if self.act[i] < p.row_lo[i] - row_tol(p.row_lo[i]) {
            p.row_lo[i] - self.act[i]
        } else {
            return None;
        };
        let dv = need / a;
        let mut nv = self.x[j] + dv;
        if p.is_int[j] {
            nv = if dv > 0.0 { (nv - 1e-9).ceil() } else { (nv + 1e-9).floor() };
        }
        nv = nv.clamp(self.lo0[j], self.up0[j]);
        if !nv.is_finite() || (nv - self.x[j]).abs() <= 1e-9 {
            return None;
        }
        Some(nv)
    }

    /// WalkSAT 風の repair。違反行がなくなれば真。
    fn repair(&mut self, max_steps: u64, noise: f64, moves: &mut u64) -> bool {
        let p = self.p;
        let mut k = 0u64;
        let mut cands: Vec<(usize, f64)> = Vec::new();
        while !self.viol.is_empty() {
            if k >= max_steps {
                return false;
            }
            k += 1;
            self.step += 1;
            let i = self.viol[self.rng.below(self.viol.len())];
            // 候補: 行 i の列 (長い行は無作為な位置から 64 本まで見る)
            let row = &p.rows[i];
            cands.clear();
            let lim = 64usize;
            let start = if row.len() > lim { self.rng.below(row.len()) } else { 0 };
            for t in 0..row.len().min(lim) {
                let (j, a) = row[(start + t) % row.len()];
                if let Some(nv) = self.shift_value(i, j, a) {
                    cands.push((j, nv));
                }
            }
            if cands.is_empty() {
                self.w[i] += 1.0;
                continue;
            }
            let (j, nv) = if self.rng.f() < noise {
                cands[self.rng.below(cands.len())]
            } else {
                let mut best: Option<(f64, usize, f64)> = None;
                for &(j, nv) in &cands {
                    // まだ決めていない列を動かす方を少し好む (決め直しが要らない)
                    let dmg = self.damage(j, nv) + if self.decided[j] { 1e-3 } else { 0.0 };
                    // タブーの列は被害が負 (良くなる) のときだけ
                    if self.tabu[j] > self.step && dmg >= 0.0 {
                        continue;
                    }
                    let tie = self.rng.f() * 1e-9;
                    if best.is_none_or(|(b, _, _)| dmg + tie < b) {
                        best = Some((dmg + tie, j, nv));
                    }
                }
                match best {
                    Some((dmg, j, nv)) => {
                        if dmg >= 0.0 {
                            // 行き詰まり: 違反行の重みを増やす
                            for t in 0..self.viol.len() {
                                let r = self.viol[t];
                                self.w[r] += 1.0;
                            }
                        }
                        (j, nv)
                    }
                    None => {
                        self.w[i] += 1.0;
                        continue;
                    }
                }
            };
            self.set_val(j, nv);
            self.tabu[j] = self.step + 3 + self.rng.below(8) as u64;
            if self.decided[j] && !self.moved_mark[j] {
                self.moved_mark[j] = true;
                self.moved.push(j);
            }
            *moves += 1;
        }
        true
    }
}

/// 1 回の fix-propagate-repair。`dom` は呼んだときの状態に戻す。`ints_only` なら整数列だけ決める
/// (repair で全行を満たせなかったときに返す点は整数列の割り当て)。`lp_x` は LP 解 (値・順に使う)。
#[allow(clippy::too_many_arguments)]
pub(crate) fn fpr_run(
    p: &MipProblem,
    dom: &mut Domain,
    locks: &[(u32, u32)],
    lp_x: Option<&[f64]>,
    order: Order,
    value: Value,
    ints_only: bool,
    seed: u64,
    deadline: Option<Instant>,
    stop: Option<&AtomicBool>,
) -> FprOut {
    let mut out = FprOut { x: None, complete: false, repairs: 0, moves: 0 };
    let n = p.n;
    let pos0 = dom.stack_len();
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let lpx = |j: usize| lp_x.map_or(0.0, |x| x[j]);
    let choose = |j: usize, lo: f64, up: f64, rng: &mut Rng| -> f64 {
        let cl = |v: f64| if v.is_finite() { v.clamp(lo, up) } else if v > 0.0 { up } else { lo };
        let zero = || 0.0f64.clamp(lo, up);
        let end = |down: bool| {
            let v = if down { lo } else { up };
            if v.is_finite() { v } else { zero() }
        };
        let (dl, ul) = locks.get(j).copied().unwrap_or((0, 0));
        let lockdir = || if dl != ul { dl < ul } else { p.cost[j] >= 0.0 };
        let v = match value {
            Value::Obj => {
                if p.cost[j] > 0.0 {
                    end(true)
                } else if p.cost[j] < 0.0 {
                    end(false)
                } else {
                    end(lockdir())
                }
            }
            Value::Locks => end(lockdir()),
            Value::Random => {
                if lo.is_finite() && up.is_finite() && up - lo <= 16.0 {
                    if p.is_int[j] {
                        lo + rng.below((up - lo) as usize + 1) as f64
                    } else {
                        lo + rng.f() * (up - lo)
                    }
                } else {
                    end(rng.f() < 0.5)
                }
            }
            Value::Lp => cl(lpx(j)),
            Value::Zero => zero(),
        };
        let v = if p.is_int[j] { v.round() } else { v };
        cl(v)
    };
    // 仮の点: 連続列は LP 解があればその値、なければ同じ規則
    let x0: Vec<f64> = (0..n)
        .map(|j| {
            if !p.is_int[j] && lp_x.is_some() {
                lpx(j).clamp(dom.lo[j], dom.up[j])
            } else {
                choose(j, dom.lo[j], dom.up[j], &mut rng)
            }
        })
        .collect();
    let mut st = Fpr::new(p, dom.lo.clone(), dom.up.clone(), x0, seed);
    // 列の順 (整数列、続けて連続列)
    let mut ints: Vec<usize> = (0..n).filter(|&j| p.is_int[j]).collect();
    let mut conts: Vec<usize> = if ints_only { Vec::new() } else { (0..n).filter(|&j| !p.is_int[j]).collect() };
    let shuffle = |v: &mut Vec<usize>, rng: &mut Rng| {
        for k in (1..v.len()).rev() {
            let r = rng.below(k + 1);
            v.swap(k, r);
        }
    };
    match order {
        Order::Input => {}
        Order::Random => shuffle(&mut ints, &mut rng),
        Order::Degree => {
            shuffle(&mut ints, &mut rng);
            ints.sort_by_key(|&j| std::cmp::Reverse(p.cols[j].len()));
        }
        Order::Domain => {
            shuffle(&mut ints, &mut rng);
            ints.sort_by(|&a, &b| {
                let wa = dom.up[a] - dom.lo[a];
                let wb = dom.up[b] - dom.lo[b];
                wa.total_cmp(&wb).then(p.cols[b].len().cmp(&p.cols[a].len()))
            });
        }
        Order::LpFrac => {
            shuffle(&mut ints, &mut rng);
            ints.sort_by(|&a, &b| {
                let fa = (lpx(a) - lpx(a).round()).abs();
                let fb = (lpx(b) - lpx(b).round()).abs();
                fa.total_cmp(&fb)
            });
        }
    }
    conts.sort_by_key(|&j| std::cmp::Reverse(p.cols[j].len()));
    let mut queue: std::collections::VecDeque<(usize, bool)> = ints.into_iter().chain(conts).map(|j| (j, false)).collect();
    // 決定の列 (列, 決定前の定義域の記録位置)
    let mut decs: Vec<(usize, usize)> = Vec::new();
    let mut dec_idx = vec![usize::MAX; n];
    let mut requeued = vec![0u8; n];
    let work0 = dom.debug_work();
    let work_cap = 30 * p.nnz as u64 + 2_000_000;
    let max_moves_total = 20 * n as u64 + 200_000;
    let noise = tunable!("ENOMOTO_T_MIP_FPR_NOISE", 0.1, f64);
    let max_requeue = tunable!("ENOMOTO_T_MIP_FPR_REQUEUE", 3u8, u8);
    let mut ok = true;
    let mut cnt = 0usize;
    // 伝播で変わった定義域に、まだ決めていない列の仮の値を収める
    let sync = |st: &mut Fpr, dom: &Domain, pos: usize| {
        let ch: Vec<usize> = dom.changes_since(pos).map(|c| c.0).collect();
        for k in ch {
            if !st.decided[k] {
                let (lo, up) = (dom.lo[k], dom.up[k]);
                let v = st.x[k];
                if v < lo || v > up {
                    st.set_val(k, v.clamp(lo, up));
                }
            }
        }
    };
    while let Some((j, again)) = queue.pop_front() {
        cnt += 1;
        if cnt % 32 == 0 && (deadline.is_some_and(|d| Instant::now() >= d) || stop.is_some_and(|s| s.load(Ordering::Relaxed)) || dom.debug_work() - work0 > work_cap || out.moves > max_moves_total) {
            ok = false;
            break;
        }
        if st.decided[j] {
            continue;
        }
        let (lo, up) = (dom.lo[j], dom.up[j]);
        let v = if lo == up {
            lo
        } else if again || lp_x.is_none() || matches!(value, Value::Lp) {
            // 仮の値 (repair が動かしたものを含む) を今の定義域に収めたもの
            let v = if p.is_int[j] { st.x[j].round() } else { st.x[j] };
            v.clamp(lo, up)
        } else {
            choose(j, lo, up, &mut rng)
        };
        let pos = dom.stack_len();
        if lo < up {
            dom.tighten_lower(p, j, v);
            dom.tighten_upper(p, j, v);
        }
        st.decided[j] = true;
        st.set_val(j, v);
        dec_idx[j] = decs.len();
        decs.push((j, pos));
        if dom.propagate(p) {
            sync(&mut st, dom, pos);
            continue;
        }
        dom.backtrack_to(p, pos);
        // 矛盾: 捨てずに j = v と決めたまま、全列の点の上で repair する
        out.repairs += 1;
        for &k in &st.moved {
            st.moved_mark[k] = false;
        }
        st.moved.clear();
        st.moved_mark[j] = true;
        st.moved.push(j);
        let steps = tunable!("ENOMOTO_T_MIP_FPR_REPAIR_STEPS", 200u64, u64) + 10 * st.viol.len() as u64;
        if st.repair(steps, noise, &mut out.moves) {
            // 全行を満たした: その点が解 (整数列は整数、列は最初の定義域の中)
            let x = st.x.clone();
            if p.is_feasible(&x, super::domain::FEASTOL) {
                out.x = Some(x);
                out.complete = true;
                dom.backtrack_to(p, pos0);
                return out;
            }
        }
        // 値の変わった最初の決定まで巻き戻して固定・伝播をやり直す
        let first = st.moved.iter().map(|&k| dec_idx[k]).min().unwrap_or(decs.len() - 1);
        dom.backtrack_to(p, decs[first].1);
        let old: Vec<(usize, usize)> = decs.drain(first..).collect();
        for &(k, _) in &old {
            dec_idx[k] = usize::MAX;
        }
        let mut dropped: Vec<usize> = Vec::new();
        for (k, _) in old {
            let v = st.x[k];
            let pos = dom.stack_len();
            if v >= dom.lo[k] - 1e-9 && v <= dom.up[k] + 1e-9 {
                if dom.lo[k] < dom.up[k] {
                    dom.tighten_lower(p, k, v);
                    dom.tighten_upper(p, k, v);
                }
                if dom.propagate(p) {
                    dec_idx[k] = decs.len();
                    decs.push((k, pos));
                    continue;
                }
                dom.backtrack_to(p, pos);
            }
            dropped.push(k);
        }
        // やり直しで矛盾した列は「まだ決めていない」に戻して、先頭に積み直す (repair の値の近くから決め直す)
        for k in dropped.into_iter().rev() {
            st.decided[k] = false;
            if requeued[k] >= max_requeue {
                ok = false;
                break;
            }
            requeued[k] += 1;
            queue.push_front((k, true));
        }
        if !ok {
            break;
        }
        // まだ決めていない列の仮の値を今の定義域に収める
        for k in 0..n {
            if !st.decided[k] && (st.x[k] < dom.lo[k] || st.x[k] > dom.up[k]) {
                st.set_val(k, st.x[k].clamp(dom.lo[k], dom.up[k]));
            }
        }
    }
    if ok && queue.is_empty() {
        // 全部 (ints_only なら整数列を全部) 決めて伝播が矛盾しなかった
        let x: Vec<f64> = (0..n).map(|k| if dom.lo[k] == dom.up[k] { dom.lo[k] } else { st.x[k].clamp(dom.lo[k], dom.up[k]) }).collect();
        if p.is_feasible(&x, super::domain::FEASTOL) {
            out.complete = true;
            out.x = Some(x);
        } else if ints_only || p.is_int.iter().any(|&b| !b) {
            out.x = Some(x);
        }
    }
    dom.backtrack_to(p, pos0);
    out
}

/// ポートフォリオの組み合わせ (LP を使わないもの)。
pub(crate) const LP_FREE: [(Order, Value); 8] = [
    (Order::Degree, Value::Locks),
    (Order::Domain, Value::Obj),
    (Order::Random, Value::Random),
    (Order::Input, Value::Obj),
    (Order::Degree, Value::Obj),
    (Order::Random, Value::Locks),
    (Order::Domain, Value::Zero),
    (Order::Random, Value::Obj),
];

/// LP 解を使う組み合わせ (根・木の中)。
pub(crate) const WITH_LP: [(Order, Value); 4] = [(Order::LpFrac, Value::Lp), (Order::Degree, Value::Lp), (Order::Random, Value::Lp), (Order::LpFrac, Value::Locks)];

/// 別スレッドの FPR (`t`)。`mailbox` に (点, 全列が決まった解か) を入れる。全列の解は良くなったものだけ。
pub(crate) fn fpr_thread(p: &MipProblem, cap: f64, seed: u64, stop: &AtomicBool, mailbox: &std::sync::Mutex<Vec<(Vec<f64>, bool)>>) {
    let t0 = Instant::now();
    let deadline = Some(t0 + std::time::Duration::from_secs_f64(cap));
    let mut dom = Domain::new(p);
    if !dom.propagate(p) {
        return;
    }
    let locks = super::heuristics::compute_locks(p);
    let mut best = f64::INFINITY;
    let mut partial_sent = 0;
    let mut runs = 0u64;
    let mut first: Option<f64> = None;
    let mut after_first = 0;
    let verbose = env_str!("ENOMOTO_MIP_LOG").is_some();
    while Instant::now() < deadline.unwrap() && !stop.load(Ordering::Relaxed) {
        let (o, v) = LP_FREE[runs as usize % LP_FREE.len()];
        let s = seed.wrapping_add(runs.wrapping_mul(0xD1B5_4A32_D192_ED03));
        let r = fpr_run(p, &mut dom, &locks, None, o, v, false, s, deadline, Some(stop));
        runs += 1;
        if let Some(x) = r.x {
            if r.complete {
                let z = p.objective(&x);
                if first.is_none() {
                    first = Some(t0.elapsed().as_secs_f64());
                }
                if z < best - 1e-9 * best.abs().max(1.0) {
                    best = z;
                    mailbox.lock().unwrap().push((x, true));
                }
            } else if partial_sent < 4 {
                partial_sent += 1;
                mailbox.lock().unwrap().push((x, false));
            }
        }
        if verbose && env_str!("ENOMOTO_MIP_DEBUG_FPR").is_some() {
            eprintln!("MIP: FPR run {runs} {o:?}/{v:?}: complete {} repairs {} moves {} ({:.2}s)", r.complete, r.repairs, r.moves, t0.elapsed().as_secs_f64());
        }
        // 解を見つけた後はポートフォリオを 1 周だけ回して止める
        if first.is_some() || partial_sent > 0 {
            after_first += 1;
            if after_first > LP_FREE.len() {
                break;
            }
        }
    }
    if verbose {
        eprintln!(
            "MIP: FPR thread: {runs} runs, first complete solution at {:?}s, best {}, partial {partial_sent}, {:.2}s",
            first.map(|v| (v * 100.0).round() / 100.0),
            if best.is_finite() { format!("{best:.6e}") } else { "none".to_string() },
            t0.elapsed().as_secs_f64()
        );
    }
}

/// 主のスレッドが持つ FPR の状態。
#[derive(Default)]
pub(crate) struct FprState {
    pub thread: Option<(std::thread::JoinHandle<()>, std::sync::Arc<AtomicBool>)>,
    pub mailbox: Option<std::sync::Arc<std::sync::Mutex<Vec<(Vec<f64>, bool)>>>>,
    /// 主のスレッドの FPR (根・木) の呼び出しの回数 (組み合わせを回す)
    pub calls: u64,
    /// 別スレッドの整数列の割り当てを LP で仕上げた回数
    pub partial_tries: u32,
}

impl Drop for FprState {
    fn drop(&mut self) {
        if let Some((_, stop)) = &self.thread {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

use super::lp_api::MipLp;
use super::solver::Solver;

impl<'a, L: MipLp> Solver<'a, L> {
    /// `t`: 別スレッドの FPR を始める (`pc` は FJ と同じ、対称性の行を足す前の問題)。
    pub(super) fn spawn_fpr_thread(&mut self, pc: std::sync::Arc<MipProblem>, seed: u64) {
        if !fpr_on('t') || self.fpr.thread.is_some() {
            return;
        }
        let cap = if self.params.time_limit.is_finite() { (tunable!("ENOMOTO_T_MIP_FPR_TIME_FRAC", 0.1, f64) * self.params.time_limit).min(10.0) } else { 10.0 };
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let mb: std::sync::Arc<std::sync::Mutex<Vec<(Vec<f64>, bool)>>> = Default::default();
        let mb2 = mb.clone();
        let h = std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || fpr_thread(&pc, cap, seed, &stop2, &mb2))
            .expect("failed to spawn the FPR thread");
        self.fpr.thread = Some((h, stop));
        self.fpr.mailbox = Some(mb);
    }

    /// 別スレッドの FPR の解を受け取る。全列の解は良い順に試し、整数列の割り当ては (暫定解がなければ) 整数列を固定して LP で仕上げる。
    pub(super) fn poll_fpr(&mut self) {
        let Some(mb) = self.fpr.mailbox.clone() else { return };
        let got: Vec<(Vec<f64>, bool)> = std::mem::take(&mut *mb.lock().unwrap());
        let mut done = false;
        for (x, complete) in got.into_iter().rev() {
            if x.len() != self.p.n {
                continue;
            }
            if complete {
                if done {
                    continue;
                }
                self.heur_label = "FPR (thread)";
                let ok = self.try_incumbent(x);
                self.heur_label = "";
                if self.params.verbose {
                    eprintln!("MIP: FPR thread solution accepted {ok} ({:.2}s)", self.start.elapsed().as_secs_f64());
                }
                done |= ok;
            } else if self.incumbent.is_none() && self.fpr.partial_tries < 4 && !self.time_up() {
                self.fpr.partial_tries += 1;
                let p = self.p;
                let ints: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j]).collect();
                self.sync_lp();
                self.heur_label = "FPR (thread, LP)";
                let ok = self.fix_and_propagate(&x, &ints);
                self.heur_label = "";
                if self.params.verbose {
                    eprintln!("MIP: FPR thread integer assignment completed by the LP: {ok} ({:.2}s)", self.start.elapsed().as_secs_f64());
                }
            }
        }
        // 暫定解ができたら別スレッドを止める (最初の解を探すためのもの)
        if self.incumbent.is_some() {
            if let Some((_, stop)) = &self.fpr.thread {
                stop.store(true, Ordering::Relaxed);
            }
        }
        if self.fpr.thread.as_ref().is_some_and(|(h, _)| h.is_finished()) {
            let (h, _) = self.fpr.thread.take().unwrap();
            let _ = h.join();
        }
    }

    /// 主のスレッドの FPR (根の `r`・木の中の `b`)。LP 解 `x` を値・順に使う組み合わせと LP を使わない組み合わせを、
    /// `runs` 回 (呼ぶたびに続きから) 回す。整数列を決め終えたら整数列を固定して連続部分を LP で解く。
    pub(super) fn fpr_heur(&mut self, x: &[f64], runs: usize, secs: f64) -> bool {
        let p = self.p;
        if !p.is_int.iter().any(|&b| b) {
            return false;
        }
        self.sync_lp();
        let t0 = Instant::now();
        let mut deadline = t0 + std::time::Duration::from_secs_f64(secs);
        if let Some(d) = self.deadline {
            deadline = deadline.min(d);
        }
        let ints: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j]).collect();
        let (mut nrep, mut nmov, mut nint_ok) = (0u64, 0u64, 0u32);
        let mut found = false;
        for _ in 0..runs {
            if self.time_up() || Instant::now() >= deadline {
                break;
            }
            let k = self.fpr.calls as usize;
            self.fpr.calls += 1;
            // LP 解を使う組み合わせ 2 回に LP を使わない組み合わせ 1 回
            let (o, v) = if k % 3 == 2 { LP_FREE[(k / 3) % LP_FREE.len()] } else { WITH_LP[(k - k / 3) % WITH_LP.len()] };
            let seed = self.rng ^ (k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
            let locks = std::mem::take(&mut self.locks);
            let r = fpr_run(p, &mut self.dom, &locks, Some(x), o, v, true, seed, Some(deadline), None);
            self.locks = locks;
            nrep += r.repairs;
            nmov += r.moves;
            if let Some(y) = r.x {
                nint_ok += 1;
                if r.complete {
                    self.heur_label = "FPR";
                    let ok = self.try_incumbent(y.clone());
                    self.heur_label = "";
                    if ok {
                        found = true;
                        break;
                    }
                }
                if self.fix_and_propagate(&y, &ints) {
                    found = true;
                    break;
                }
            }
        }
        if self.params.verbose && (self.nodes <= 1 || env_str!("ENOMOTO_MIP_DEBUG_FPR").is_some()) {
            eprintln!("MIP:   FPR: found {found}, integer assignments {nint_ok}, repairs {nrep}, moves {nmov} ({:.2}s)", t0.elapsed().as_secs_f64());
        }
        found
    }
}
