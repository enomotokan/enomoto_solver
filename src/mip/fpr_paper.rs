//! 試作 `ENOMOTO_MIP_FPR` の文字 `p`: 論文 (Salvagnin, Roberti, Fischetti 2025) に寄せた fix-propagate-repair の核。
//!
//! 論文の本文は読めず、論文を HiGHS に移した公開実装 (Spoorendonk 2026「Presolve heuristics in HiGHS」arXiv 2609.22938 の Algorithm 2、
//! github.com/spoorendonk/mip-heuristics の `fpr_core.cpp`・`repair_walk.cpp`・`fpr_strategies.h`) に書かれた論文の記述に合わせた:
//! - 1 回の試み: 整数列を順に、値の規則で決め、伝播する深さ優先探索 (兄弟 = もう一方の端、0-1 列なら 1 - v)。節点の数は n + 1 まで。
//!   形 (mode): `dfs` (伝播あり・repair なし・後戻りあり)、`dfsrep` (伝播・repair・後戻り)、`dive` (伝播なし・repair は最後だけ・後戻りなし)、
//!   `diveprop` (伝播・repair・後戻りなし)。論文の `repairsearch` は `dfsrep` で代える。
//! - 最初に自明に丸められる整数列 (上向きの lock が 0 なら上限、下向きが 0 なら下限) を固定して伝播する。
//! - repair (RepairWalk、論文 5 節): 部分割り当て (今の定義域) の上で、行の活動量の範囲 [m, M] と行の範囲の距離を違反とし、違反行を無作為に選び、
//!   その行を満たす最小のずらし (整数列は丸め、元の境界に収まるよう切る) で列の定義域ごと平行移動する。行の違反を減らさない列は候補にしない。
//!   被害 = 他の行の違反の増えた分の和 (減った分は数えない)。被害 0 の手があれば貪欲に、なければ確率 0.75 で無作為、残りは被害最小。
//!   タブーは直近 3 列。10 手ごとに最良の状態 (違反の和が最小) に戻す。200 手まで。伝播は使わない。
//! - 整数列を全部決めたら、残りの列を埋める (費用のある連続列は良い向きの端、費用 0 は 0、無限の端は ±1e5 で切る。整数列は値の規則) →
//!   違反が残れば全列の点の上の WalkSAT (200 手、雑音 0.75、最良の状態に戻す) → 全行を確かめる。
//! - LP を使わない 8 通り (論文 6.3 節の Class 1、`kInitialFprConfigs`): (type, badobj, dfs)・(locks, loosedyn, dfs)・(locks, loosedyn, dive)・
//!   (LR, loosedyn, dfsrep)・(type, badobj, dfsrep)・(type 内で無作為, random, diveprop)・(LR, loosedyn, repairsearch → dfsrep)・
//!   (domainsize, loosedyn, dfs)。2 回目からは順の先頭 30% を混ぜる。`type` は論文では 0-1 列のクリーク被覆を先に置く (`typecl`) が、ここでは型の順だけ。
//!
//! ENOMOTO の定義域 (`Domain`) は境界を締めることしかできないので、repair で決めた列の値が変わったら、定義域を最初に戻して決めた列を
//! 新しい値で固定し直して伝播する (論文・移植は平行移動した定義域のまま探索を続ける)。やり直しで矛盾したら、その repair は失敗とする。
//! 固定し直した後は、それより前の兄弟には戻らない。

use super::domain::{Domain, FEASTOL};
use super::problem::MipProblem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum VarRule {
    /// 列の番号の順 (論文の LR)
    Lr,
    /// 型の順 (0-1、一般整数)
    Type,
    /// 型の中で無作為
    RandomType,
    /// 型の中で max(上向きの lock, 下向きの lock) の大きい順
    Locks,
    /// 節点ごとに定義域の最も狭い列 (動的)
    DomSize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ValRule {
    /// 目的の悪い向きの端 (費用 0 なら下限)
    BadObj,
    /// 目的の良い向きの端 (費用 0 なら下限)
    GoodObj,
    /// 今の定義域の活動量で数えた lock の少ない向きの端 (論文の loosedyn)
    LooseDyn,
    /// 定義域の中で一様
    Random,
    /// 参照点 (LP 解) を、小数部分の確率で切り上げ
    Lp,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Mode {
    Dfs,
    DfsRep,
    Dive,
    DiveProp,
}

impl Mode {
    fn prop(self) -> bool {
        self != Mode::Dive
    }
    fn rep(self) -> bool {
        matches!(self, Mode::DfsRep | Mode::Dive | Mode::DiveProp)
    }
    fn bt(self) -> bool {
        matches!(self, Mode::Dfs | Mode::DfsRep)
    }
}

/// 論文 6.3 節の Class 1 (LP を使わない 8 通り)。
pub(crate) const CLASS1: [(VarRule, ValRule, Mode); 8] = [
    (VarRule::Type, ValRule::BadObj, Mode::Dfs),
    (VarRule::Locks, ValRule::LooseDyn, Mode::Dfs),
    (VarRule::Locks, ValRule::LooseDyn, Mode::Dive),
    (VarRule::Lr, ValRule::LooseDyn, Mode::DfsRep),
    (VarRule::Type, ValRule::BadObj, Mode::DfsRep),
    (VarRule::RandomType, ValRule::Random, Mode::DiveProp),
    (VarRule::Lr, ValRule::LooseDyn, Mode::DfsRep),
    (VarRule::DomSize, ValRule::LooseDyn, Mode::Dfs),
];

/// LP 解を使う組み合わせ (論文の Class 2・3 の形。どの組を使うかは未確認なので、移植の `kStratLp` (typecl/lp) と形を組み合わせた)。
pub(crate) const WITH_LP: [(VarRule, ValRule, Mode); 4] = [
    (VarRule::Type, ValRule::Lp, Mode::DfsRep),
    (VarRule::Type, ValRule::Lp, Mode::Dive),
    (VarRule::Locks, ValRule::Lp, Mode::DiveProp),
    (VarRule::Lr, ValRule::Lp, Mode::Dfs),
];

const NOISE: f64 = 0.75;
const WALK_STEPS: usize = 200;
const TABU: usize = 3;
const RESTART: usize = 10;

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

fn tol(b: f64) -> f64 {
    FEASTOL * (1.0 + b.abs())
}

/// 区間 [m, mx] と行の範囲の距離。
fn range_viol(p: &MipProblem, i: usize, m: f64, mx: f64) -> f64 {
    let mut v = 0.0;
    if p.row_up[i].is_finite() && m > p.row_up[i] + tol(p.row_up[i]) {
        v += m - p.row_up[i];
    }
    if p.row_lo[i].is_finite() && mx < p.row_lo[i] - tol(p.row_lo[i]) {
        v += p.row_lo[i] - mx;
    }
    v
}

/// 部分割り当て (定義域 `l`・`u`) の上の RepairWalk。成功すれば真 (`l`・`u` は直した定義域)。
struct RangeWalk<'p> {
    p: &'p MipProblem,
    col_lo: &'p [f64],
    col_up: &'p [f64],
    l: Vec<f64>,
    u: Vec<f64>,
    minf: Vec<f64>,
    mini: Vec<u32>,
    maxf: Vec<f64>,
    maxi: Vec<u32>,
}

impl<'p> RangeWalk<'p> {
    fn new(p: &'p MipProblem, col_lo: &'p [f64], col_up: &'p [f64], l: Vec<f64>, mut u: Vec<f64>) -> Self {
        // 矛盾で交差した定義域は下の端に潰す
        for j in 0..p.n {
            if l[j] > u[j] {
                u[j] = l[j];
            }
        }
        let mut s = RangeWalk { p, col_lo, col_up, l, u, minf: vec![0.0; p.m], mini: vec![0; p.m], maxf: vec![0.0; p.m], maxi: vec![0; p.m] };
        for i in 0..p.m {
            for &(j, a) in &p.rows[i] {
                let (lo, hi) = if a > 0.0 { (a * s.l[j], a * s.u[j]) } else { (a * s.u[j], a * s.l[j]) };
                if lo.is_finite() {
                    s.minf[i] += lo;
                } else {
                    s.mini[i] += 1;
                }
                if hi.is_finite() {
                    s.maxf[i] += hi;
                } else {
                    s.maxi[i] += 1;
                }
            }
        }
        s
    }
    fn range(&self, i: usize) -> (f64, f64) {
        (if self.mini[i] > 0 { f64::NEG_INFINITY } else { self.minf[i] }, if self.maxi[i] > 0 { f64::INFINITY } else { self.maxf[i] })
    }
    fn viol(&self, i: usize) -> f64 {
        let (m, mx) = self.range(i);
        range_viol(self.p, i, m, mx)
    }
    fn shift(&mut self, j: usize, s: f64) {
        self.l[j] += s;
        self.u[j] += s;
        for &(i, a) in &self.p.cols[j] {
            self.minf[i] += a * s;
            self.maxf[i] += a * s;
        }
    }

    fn walk(&mut self, rng: &mut Rng, work: &mut u64, work_cap: u64) -> bool {
        let p = self.p;
        let mut violated: Vec<usize> = (0..p.m).filter(|&i| self.viol(i) > 0.0).collect();
        let total = |s: &Self, v: &[usize]| v.iter().map(|&i| s.viol(i)).sum::<f64>();
        let mut moves: Vec<(usize, f64)> = Vec::new();
        let mut best = (total(self, &violated), 0usize);
        let mut tabu: Vec<usize> = Vec::new();
        let mut since = 0usize;
        let mut cand: Vec<(usize, f64, f64)> = Vec::new();
        for _ in 0..WALK_STEPS {
            violated.retain(|&i| self.viol(i) > 0.0);
            if violated.is_empty() {
                return true;
            }
            if *work > work_cap {
                break;
            }
            let row = violated[rng.below(violated.len())];
            let (m, mx) = self.range(row);
            let cur = range_viol(p, row, m, mx);
            let above = p.row_up[row].is_finite() && m > p.row_up[row] + tol(p.row_up[row]);
            let (target, anchor) = if above { (p.row_up[row], m) } else { (p.row_lo[row], mx) };
            cand.clear();
            let mut best_dmg = f64::INFINITY;
            *work += p.rows[row].len() as u64;
            for &(j, a) in &p.rows[row] {
                if a.abs() < 1e-15 || tabu.contains(&j) {
                    continue;
                }
                let (cl, cu) = (self.l[j], self.u[j]);
                if !cl.is_finite() || !cu.is_finite() {
                    continue;
                }
                let mut s = (target - anchor) / a;
                if p.is_int[j] {
                    s = if above == (a > 0.0) { (s + FEASTOL).floor() } else { (s - FEASTOL).ceil() };
                }
                s = s.max(self.col_lo[j] - cl).min(self.col_up[j] - cu);
                if p.is_int[j] {
                    s = if s > 0.0 { s.floor() } else { s.ceil() };
                }
                if s.abs() < 1e-15 || !s.is_finite() {
                    continue;
                }
                if range_viol(p, row, m + a * s, mx + a * s) >= cur - FEASTOL {
                    continue;
                }
                *work += p.cols[j].len() as u64;
                let mut dmg = 0.0;
                for &(i2, a2) in &p.cols[j] {
                    if i2 == row {
                        continue;
                    }
                    let (m2, mx2) = self.range(i2);
                    let d = a2 * s;
                    let dv = range_viol(p, i2, m2 + d, mx2 + d) - range_viol(p, i2, m2, mx2);
                    if dv > 0.0 {
                        dmg += dv;
                    }
                }
                best_dmg = best_dmg.min(dmg);
                cand.push((j, s, dmg));
            }
            if cand.is_empty() {
                continue;
            }
            let k = if best_dmg > FEASTOL && rng.f() < NOISE {
                rng.below(cand.len())
            } else {
                let th = best_dmg + FEASTOL;
                let idx: Vec<usize> = (0..cand.len()).filter(|&c| cand[c].2 <= th).collect();
                idx[rng.below(idx.len())]
            };
            let (j, s, _) = cand[k];
            self.shift(j, s);
            moves.push((j, s));
            for &(i, _) in &p.cols[j] {
                if self.viol(i) > 0.0 && !violated.contains(&i) {
                    violated.push(i);
                }
            }
            tabu.push(j);
            if tabu.len() > TABU {
                tabu.remove(0);
            }
            violated.retain(|&i| self.viol(i) > 0.0);
            let t = total(self, &violated);
            if t < best.0 - FEASTOL {
                best = (t, moves.len());
            }
            since += 1;
            if since >= RESTART {
                since = 0;
                if t > best.0 + FEASTOL {
                    // 最良の状態に戻す (タブーは残す)
                    while moves.len() > best.1 {
                        let (j, s) = moves.pop().unwrap();
                        self.shift(j, -s);
                    }
                    violated = (0..p.m).filter(|&i| self.viol(i) > 0.0).collect();
                    *work += p.m as u64;
                }
            }
        }
        violated.retain(|&i| self.viol(i) > 0.0);
        if !violated.is_empty() && total(self, &violated) > best.0 + FEASTOL {
            // 最良の状態に戻す
            while moves.len() > best.1 {
                let (j, s) = moves.pop().unwrap();
                self.shift(j, -s);
            }
            violated = (0..p.m).filter(|&i| self.viol(i) > 0.0).collect();
        }
        violated.is_empty()
    }
}

/// 全列の点の上の WalkSAT (論文の最後の段)。200 手、雑音 0.75、被害は増えた違反の和、最良の点に戻す。
fn walksat_point(p: &MipProblem, col_lo: &[f64], col_up: &[f64], x: &mut [f64], rng: &mut Rng, work: &mut u64, work_cap: u64) {
    let mut w = RangeWalk::new(p, col_lo, col_up, x.to_vec(), x.to_vec());
    // 最良の状態は walk の終わりで戻してある
    w.walk(rng, work, work_cap);
    x.copy_from_slice(&w.l);
}

pub(crate) struct PaperOut {
    /// 全列が決まった実行可能解 (`complete`)、または整数列の割り当て (連続列は意味なし)
    pub x: Option<Vec<f64>>,
    pub complete: bool,
    pub nodes: usize,
    pub repairs: u64,
    pub repairs_ok: u64,
}

/// 1 回の試み (論文の Algorithm、移植の `fpr_attempt`)。`dom` は呼んだときの状態に戻す。`ints_only` なら、整数列を全部決めたところで
/// 整数列の割り当てを返す (連続部分は呼ぶ側が LP で解く)。
#[allow(clippy::too_many_arguments)]
pub(crate) fn fpr_attempt(
    p: &MipProblem,
    dom: &mut Domain,
    locks: &[(u32, u32)],
    lp_x: Option<&[f64]>,
    cfg: (VarRule, ValRule, Mode),
    attempt: u64,
    ints_only: bool,
    seed: u64,
    deadline: Option<Instant>,
    stop: Option<&AtomicBool>,
) -> PaperOut {
    let (vr, val, mode) = cfg;
    let mut out = PaperOut { x: None, complete: false, nodes: 0, repairs: 0, repairs_ok: 0 };
    let n = p.n;
    let pos0 = dom.stack_len();
    let col_lo: Vec<f64> = dom.lo.clone();
    let col_up: Vec<f64> = dom.up.clone();
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let work0 = dom.debug_work();
    let work_cap = 100 * p.nnz as u64 + 1_000_000;
    let mut work = 0u64;
    let is_bin = |j: usize| p.is_int[j] && col_lo[j] == 0.0 && col_up[j] == 1.0;
    // 列の順 (静的なもの)
    let mut order: Vec<usize> = (0..n).filter(|&j| p.is_int[j]).collect();
    let typ = |j: usize| if is_bin(j) { 0 } else { 1 };
    match vr {
        VarRule::Lr | VarRule::DomSize => {}
        VarRule::Type => order.sort_by_key(|&j| typ(j)),
        VarRule::RandomType => {
            for k in (1..order.len()).rev() {
                let r = rng.below(k + 1);
                order.swap(k, r);
            }
            order.sort_by_key(|&j| typ(j));
        }
        VarRule::Locks => order.sort_by_key(|&j| (typ(j), std::cmp::Reverse(locks[j].0.max(locks[j].1)))),
    }
    if attempt > 0 && !order.is_empty() {
        let k = (3 * order.len() / 10).max(1);
        for t in (1..k).rev() {
            let r = rng.below(t + 1);
            order.swap(t, r);
        }
    }
    let lpx = |j: usize| lp_x.map_or(0.0, |x| x[j]);
    // 値の規則
    let choose = |j: usize, dom: &Domain, rng: &mut Rng| -> f64 {
        let (lo, up) = (dom.lo[j], dom.up[j]);
        let end = |down: bool| {
            let v = if down { lo } else { up };
            if v.is_finite() { v } else { 0.0f64.clamp(lo, up) }
        };
        let good = || if p.cost[j].abs() < 1e-15 { end(true) } else { end(p.cost[j] > 0.0) };
        let v = match val {
            ValRule::GoodObj => good(),
            ValRule::BadObj => {
                if p.cost[j].abs() < 1e-15 {
                    end(true)
                } else {
                    end(p.cost[j] < 0.0)
                }
            }
            ValRule::Random => {
                if lo.is_finite() && up.is_finite() {
                    (lo + rng.f() * (up - lo)).round()
                } else {
                    end(rng.f() < 0.5)
                }
            }
            ValRule::Lp => {
                let c = lpx(j).clamp(lo, up);
                let f = c - c.floor();
                if f < 1e-10 {
                    c.round()
                } else if rng.f() < f {
                    c.ceil()
                } else {
                    c.floor()
                }
            }
            ValRule::LooseDyn => {
                // 今の定義域の活動量で、上げると / 下げると満たせなくなりうる行の数 (既に満たされている行・既に満たせない行は数えない)
                let col = &p.cols[j];
                if col.iter().map(|&(i, _)| p.rows[i].len()).sum::<usize>() > 5000 {
                    // 密な列は静的な lock で代える
                    let (dl, ul) = locks[j];
                    if ul < dl {
                        end(false)
                    } else if dl < ul {
                        end(true)
                    } else {
                        good()
                    }
                } else {
                    let (mut ul, mut dl) = (0usize, 0usize);
                    for &(i, a) in col {
                        let (mut m, mut mx) = (0.0f64, 0.0f64);
                        for &(k, b) in &p.rows[i] {
                            let (x1, x2) = if b > 0.0 { (b * dom.lo[k], b * dom.up[k]) } else { (b * dom.up[k], b * dom.lo[k]) };
                            m += x1;
                            mx += x2;
                        }
                        let (hu, hl) = (p.row_up[i].is_finite(), p.row_lo[i].is_finite());
                        if (hu && m > p.row_up[i]) || (hl && mx < p.row_lo[i]) {
                            continue;
                        }
                        if (!hl || m >= p.row_lo[i]) && (!hu || mx <= p.row_up[i]) {
                            continue;
                        }
                        if (hu && a > 0.0) || (hl && a < 0.0) {
                            ul += 1;
                        }
                        if (hu && a < 0.0) || (hl && a > 0.0) {
                            dl += 1;
                        }
                    }
                    if ul < dl {
                        end(false)
                    } else if dl < ul {
                        end(true)
                    } else {
                        good()
                    }
                }
            }
        };
        let v = if p.is_int[j] { v.round() } else { v };
        v.clamp(lo, up)
    };
    let sibling = |j: usize, v: f64, dom: &Domain| -> f64 {
        if is_bin(j) {
            1.0 - v
        } else if (v - dom.lo[j]).abs() < 0.5 {
            dom.up[j]
        } else {
            dom.lo[j]
        }
    };
    let timeout = |dom: &Domain| deadline.is_some_and(|d| Instant::now() >= d) || stop.is_some_and(|s| s.load(Ordering::Relaxed)) || dom.debug_work() - work0 > work_cap;
    // Phase 1: 自明に丸められる整数列を固定して伝播
    for j in 0..n {
        if !p.is_int[j] || dom.lo[j] == dom.up[j] {
            continue;
        }
        let (dl, ul) = locks[j];
        if ul == 0 && dl > 0 && dom.up[j].is_finite() {
            dom.tighten_lower(p, j, dom.up[j]);
        } else if dl == 0 && ul > 0 && dom.lo[j].is_finite() {
            dom.tighten_upper(p, j, dom.lo[j]);
        }
    }
    if mode.prop() && !dom.propagate(p) {
        dom.backtrack_to(p, pos0);
        return out;
    }
    // 次の決めていない整数列 (静的な順は位置 `from` から、domainsize は最も狭いもの)
    let next = |dom: &Domain, from: usize| -> Option<(usize, usize)> {
        if vr == VarRule::DomSize {
            order.iter().filter(|&&j| dom.lo[j] < dom.up[j]).min_by(|&&a, &&b| (dom.up[a] - dom.lo[a]).total_cmp(&(dom.up[b] - dom.lo[b]))).map(|&j| (j, 0))
        } else {
            (from..order.len()).find(|&k| dom.lo[order[k]] < dom.up[order[k]]).map(|k| (order[k], k))
        }
    };
    // 節点: (列, 値, 節点の前の定義域の記録位置, 順の位置)
    let mut stack: Vec<(usize, f64, usize, usize)> = Vec::new();
    let mut complete = false;
    // 決めた列 (repair の後の固定し直し用): (列, 値)
    let mut decided: Vec<(usize, f64)> = Vec::new();
    let mut dec_at: Vec<usize> = Vec::new(); // 節点の前の decided の長さ (記録位置ごと)
    match next(dom, 0) {
        None => complete = true,
        Some((k, at)) => {
            let v = choose(k, dom, &mut rng);
            let mark = dom.stack_len();
            if mode.bt() {
                stack.push((k, sibling(k, v, dom), mark, at));
                dec_at.push(decided.len());
            }
            stack.push((k, v, mark, at));
            dec_at.push(decided.len());
        }
    }
    while let Some((j, v, mark, at)) = stack.pop() {
        let dlen = dec_at.pop().unwrap_or(0);
        if out.nodes > n || complete {
            break;
        }
        out.nodes += 1;
        if out.nodes % 16 == 0 && timeout(dom) {
            break;
        }
        dom.backtrack_to(p, mark);
        decided.truncate(dlen);
        let mut repaired = false;
        let mut infeas = v < dom.lo[j] - 1e-9 || v > dom.up[j] + 1e-9;
        if !infeas {
            dom.tighten_lower(p, j, v);
            dom.tighten_upper(p, j, v);
            decided.push((j, v));
            if mode.prop() {
                infeas = !dom.propagate(p);
            }
        }
        if infeas && mode.rep() && mode.prop() {
            // 部分割り当ての上の RepairWalk (伝播で矛盾した状態の定義域で)
            out.repairs += 1;
            let mut w = RangeWalk::new(p, &col_lo, &col_up, dom.lo.clone(), dom.up.clone());
            if !decided.iter().any(|&(k, _)| k == j) {
                // 値が定義域の外だった: その値に置いてから直す
                w.l[j] = v;
                w.u[j] = v;
                w = RangeWalk::new(p, &col_lo, &col_up, w.l.clone(), w.u.clone());
                decided.push((j, v));
            }
            if w.walk(&mut rng, &mut work, work_cap) {
                // 決めた列の新しい値で、最初から固定し直す
                let vals: Vec<(usize, f64)> = decided.iter().map(|&(k, _)| (k, w.l[k])).collect();
                dom.backtrack_to(p, pos0);
                let mut ok = dom.propagate(p);
                for &(k, x) in &vals {
                    if !ok {
                        break;
                    }
                    if x < dom.lo[k] - 1e-9 || x > dom.up[k] + 1e-9 {
                        ok = false;
                        break;
                    }
                    dom.tighten_lower(p, k, x);
                    dom.tighten_upper(p, k, x);
                    ok = dom.propagate(p);
                }
                if ok {
                    out.repairs_ok += 1;
                    decided = vals;
                    // それより前の兄弟には戻らない
                    repaired = true;
                    stack.clear();
                    dec_at.clear();
                    infeas = false;
                } else {
                    dom.backtrack_to(p, pos0);
                    // 前の状態には戻れないので、この試みはここまで
                    break;
                }
            }
        }
        if infeas && mode.bt() {
            continue;
        }
        match next(dom, if repaired { 0 } else { at }) {
            None => {
                complete = true;
                break;
            }
            Some((k, at2)) => {
                let v = choose(k, dom, &mut rng);
                let mark2 = dom.stack_len();
                if mode.bt() {
                    stack.push((k, sibling(k, v, dom), mark2, at2));
                    dec_at.push(decided.len());
                }
                stack.push((k, v, mark2, at2));
                dec_at.push(decided.len());
            }
        }
    }
    if complete {
        // Phase 2.5: 残りの列を埋める
        let mut x: Vec<f64> = (0..n)
            .map(|j| {
                let (lo, up) = (dom.lo[j], dom.up[j]);
                if lo == up {
                    lo
                } else if p.is_int[j] {
                    choose(j, dom, &mut rng)
                } else {
                    let v = if p.cost[j] > 0.0 {
                        lo
                    } else if p.cost[j] < 0.0 {
                        up
                    } else {
                        0.0
                    };
                    let v = if v.is_finite() { v } else { 0.0 };
                    v.clamp(lo.max(-1e5), up.min(1e5)).clamp(lo, up)
                }
            })
            .collect();
        if ints_only && p.is_int.iter().any(|&b| !b) {
            out.x = Some(x);
        } else {
            if !p.is_feasible(&x, FEASTOL) && mode.rep() {
                walksat_point(p, &col_lo, &col_up, &mut x, &mut rng, &mut work, work_cap);
            }
            if p.is_feasible(&x, FEASTOL) {
                out.complete = true;
                out.x = Some(x);
            } else if p.is_int.iter().any(|&b| !b) && dom.lo.iter().zip(&dom.up).enumerate().all(|(j, (l, u))| !p.is_int[j] || l == u) {
                // 整数列は伝播と矛盾せずに全部決まった: 連続部分は呼ぶ側が LP で解く
                out.x = Some((0..n).map(|j| if p.is_int[j] { dom.lo[j] } else { 0.0 }).collect());
            }
        }
    }
    dom.backtrack_to(p, pos0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// fpr.rs の単体テストと同じ小さな 0-1 問題で、Class 1 の 8 通りが実行可能解を返し、定義域を元に戻すこと。
    #[test]
    fn paper_kernel_finds_feasible_points() {
        let n = 6;
        let rows = vec![
            vec![(0, 1.0), (1, 1.0), (2, 1.0)],
            vec![(3, 1.0), (4, 1.0), (5, 1.0)],
            vec![(0, 1.0), (3, 1.0)],
            vec![(0, 1.0), (4, -1.0)],
            vec![(2, 1.0), (5, 1.0)],
        ];
        let p = MipProblem::from_rows(
            vec![0.0; n],
            vec![1.0; n],
            vec![1.0; n],
            0.0,
            1.0,
            vec![true; n],
            rows,
            vec![1.0, 1.0, f64::NEG_INFINITY, 0.0, 1.0],
            vec![1.0, 1.0, 1.0, f64::INFINITY, f64::INFINITY],
        );
        let mut dom = Domain::new(&p);
        assert!(dom.propagate(&p));
        let locks = super::super::heuristics::compute_locks(&p);
        let mut found = 0;
        for (k, &cfg) in CLASS1.iter().enumerate() {
            for a in 0..4u64 {
                let r = fpr_attempt(&p, &mut dom, &locks, None, cfg, a, false, 3 + 11 * a + k as u64, None, None);
                if r.complete {
                    let x = r.x.unwrap();
                    assert!(p.is_feasible(&x, 1e-9), "{cfg:?}: {x:?}");
                    found += 1;
                }
            }
        }
        assert!(dom.lo.iter().all(|&v| v == 0.0) && dom.up.iter().all(|&v| v == 1.0));
        assert!(found >= CLASS1.len() * 4 / 2, "found {found}");
    }
}
