//! 主ヒューリスティクス (実行可能解を探す)。HiGHS の `HighsPrimalHeuristics` と
//! Feasibility Jump (Luteberger & Sartor 2023) を簡略化したもの。
//!
//! - [`Solver::simple_rounding`]: LP 解の小数の整数列を、制約を悪化させない向き (lock のない向き) に丸める。
//! - [`Solver::fix_and_propagate`]: 整数列を目標値に固定しながら伝播し、最後に連続部分を LP で解く。
//! - [`Solver::randomized_rounding`]: LP 解をしきい値を乱数にして丸め、fix_and_propagate に渡す。
//! - [`Solver::feasibility_pump`]: 丸めた点への L1 距離を目的にした LP を繰り返す。
//! - [`Solver::feasibility_jump`]: LP を使わない重み付き局所探索 (根の LP の前に使う)。

use super::domain::FEASTOL;
use super::lp::{LpStatus, SolveLimits};
use super::problem::MipProblem;
use super::solver::Solver;
use super::lp_api::MipLp;
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// 列ごとの lock 数 (下げると違反しうる行の数, 上げると違反しうる行の数)。
pub fn compute_locks(p: &MipProblem) -> Vec<(u32, u32)> {
    let mut locks = vec![(0u32, 0u32); p.n];
    for j in 0..p.n {
        for &(i, a) in &p.cols[j] {
            let (lo, up) = (p.row_lo[i].is_finite(), p.row_up[i].is_finite());
            let down = if a > 0.0 { lo } else { up };
            let upl = if a > 0.0 { up } else { lo };
            locks[j].0 += down as u32;
            locks[j].1 += upl as u32;
        }
    }
    locks
}

impl<'a, L: MipLp> Solver<'a, L> {
    pub(super) fn rand(&mut self) -> f64 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64) / ((1u64 << 53) as f64)
    }

    /// 単純丸め。小数の整数列をすべて lock のない向きに丸められれば、その点を試す。
    pub(super) fn simple_rounding(&mut self, x: &[f64]) -> bool {
        let mut y = x.to_vec();
        for j in 0..self.p.n {
            if !self.p.is_int[j] {
                continue;
            }
            let v = x[j];
            if (v - v.round()).abs() <= FEASTOL {
                y[j] = v.round();
                continue;
            }
            let (dl, ul) = self.locks[j];
            if dl == 0 {
                y[j] = v.floor();
            } else if ul == 0 {
                y[j] = v.ceil();
            } else {
                return false;
            }
        }
        self.try_incumbent(y)
    }

    /// 整数列を `order` の順に `target` の値 (丸めて定義域に収めたもの) に固定しながら伝播する。
    /// 矛盾したら 1 つ隣の値を試す。全部固定できたら、連続変数があれば LP で解いて試す。
    /// 定義域と LP は呼ぶ前の状態に戻す。
    pub(super) fn fix_and_propagate(&mut self, target: &[f64], order: &[usize]) -> bool {
        self.propagate_rounding(target, order, None)
    }

    /// [`Self::fix_and_propagate`] の本体。`rounded` を渡すと、伝播を使った丸め点 (Feasibility Pump 2.0
    /// の丸め) を書き込む: 固定できた列はその値、矛盾が出た後の列は (そこで伝播を止めて) その時点の
    /// 定義域に収めた `target` の丸め。
    pub(super) fn propagate_rounding(&mut self, target: &[f64], order: &[usize], rounded: Option<&mut Vec<f64>>) -> bool {
        self.propagate_rounding_from(target, order, rounded, None)
    }

    /// [`Self::propagate_rounding`] で、連続部分の LP を基底 `start` (列番号, その基底の点) から始めるもの
    /// (クロスオーバーの頂点の丸めで、その基底を使う。基底は双対実行可能でなくてよい)。
    pub(super) fn propagate_rounding_from(&mut self, target: &[f64], order: &[usize], mut rounded: Option<&mut Vec<f64>>, start: Option<(&[usize], &[f64])>) -> bool {
        let p = self.p;
        let pos = self.dom.stack_len();
        let mut ok = true;
        let mut all: Vec<usize> = order.to_vec();
        // order に含まれない整数列も最後に固定する
        let mut in_order = vec![false; p.n];
        for &j in order {
            in_order[j] = true;
        }
        all.extend((0..p.n).filter(|&j| p.is_int[j] && !in_order[j]));
        let work0 = self.dom.debug_work();
        let work_cap = 20 * p.rows.iter().map(|r| r.len() as u64).sum::<u64>() + 1_000_000;
        for (cnt, &j) in all.iter().enumerate() {
            if !p.is_int[j] {
                continue;
            }
            // 時間切れ、または伝播の手間 (非零数の 20 倍) を使い切ったら諦める
            if cnt % 64 == 0 && (self.time_up() || self.dom.debug_work() - work0 > work_cap) {
                ok = false;
                if rounded.is_none() {
                    break;
                }
            }
            let t = target[j];
            let v = t.round().clamp(self.dom.lo[j], self.dom.up[j]);
            if !ok {
                // 矛盾の後: 伝播せずに今の定義域で丸めるだけ
                if let Some(r) = rounded.as_deref_mut() {
                    r[j] = v;
                }
                continue;
            }
            if self.dom.is_fixed(j) {
                if let Some(r) = rounded.as_deref_mut() {
                    r[j] = self.dom.lo[j];
                }
                continue;
            }
            let before = self.dom.stack_len();
            self.dom.tighten_lower(p, j, v);
            self.dom.tighten_upper(p, j, v);
            if self.dom.propagate(p) {
                if let Some(r) = rounded.as_deref_mut() {
                    r[j] = v;
                }
                continue;
            }
            // 隣の値を試す
            self.dom.backtrack_to(p, before);
            let v2 = if t > v { v + 1.0 } else { v - 1.0 };
            if v2 >= self.dom.lo[j] && v2 <= self.dom.up[j] {
                self.dom.tighten_lower(p, j, v2);
                self.dom.tighten_upper(p, j, v2);
                if self.dom.propagate(p) {
                    if let Some(r) = rounded.as_deref_mut() {
                        r[j] = v2;
                    }
                    continue;
                }
                self.dom.backtrack_to(p, before);
            }
            ok = false;
            if let Some(r) = rounded.as_deref_mut() {
                r[j] = v.clamp(self.dom.lo[j], self.dom.up[j]);
            } else {
                break;
            }
        }
        let mut found = false;
        if ok {
            if p.is_int.iter().all(|&b| b) {
                let x: Vec<f64> = self.dom.lo.clone();
                found = self.try_incumbent(x);
            } else {
                let saved = self.lp.save_state();
                for j in 0..p.n {
                    let (l, u) = self.lp.col_bounds(j);
                    if l != self.dom.lo[j] || u != self.dom.up[j] {
                        self.lp.set_col_bounds(j, self.dom.lo[j], self.dom.up[j]);
                    }
                }
                if let Some((b, xb)) = start {
                    self.lp.set_basis_cols(b, Some(xb));
                }
                let it0 = self.lp.total_iterations();
                let lim = (2 * self.avg_node_iters()).max(1000);
                let st = self.lp.solve(&self.limits(lim));
                self.heur_iters += self.lp.total_iterations() - it0;
                if st == LpStatus::Optimal {
                    let x = self.lp.col_values();
                    found = self.try_incumbent(x);
                }
                self.lp.restore_state(&saved);
            }
        }
        self.dom.backtrack_to(p, pos);
        found
    }

    /// Shift-and-Propagate (SCIP の `heur_shiftandpropagate` の簡略版)。LP を使わず、各列を定義域の端の点
    /// (下限、なければ上限、なければ 0) に置いた点から始め、違反している行の整数列のうち、その列の行の違反数を
    /// 最も減らす値 (行を満たすのに要るずらし量から候補を作る) に固定して伝播する、を繰り返す。違反行が
    /// なくなったら残りの整数列を今の値に固定する。行き詰まったら直前の固定の次の候補に戻る (上限あり)。
    /// 全部固定できたら連続部分を LP で解いて試す。定義域と LP は戻す。
    pub(super) fn shift_and_propagate(&mut self) -> bool {
        let p = self.p;
        let n = p.n;
        if !p.is_int.iter().any(|&b| b) {
            return false;
        }
        self.sync_lp();
        let pos = self.dom.stack_len();
        // 点は定義域の関数: 下限、なければ上限、なければ 0 (固定された列はその値)
        let val = |lo: f64, up: f64| if lo.is_finite() { lo } else if up.is_finite() { up } else { 0.0 };
        let mut x: Vec<f64> = (0..n).map(|j| val(self.dom.lo[j], self.dom.up[j])).collect();
        let mut act = vec![0.0f64; p.m];
        for (i, r) in p.rows.iter().enumerate() {
            act[i] = r.iter().map(|&(j, a)| a * x[j]).sum();
        }
        let tol = |b: f64| 1e-6 * (1.0 + b.abs());
        let viol = |i: usize, a: f64| (a < p.row_lo[i] - tol(p.row_lo[i])) as u32 + (a > p.row_up[i] + tol(p.row_up[i])) as u32;
        let work0 = self.dom.debug_work();
        let work_cap = 20 * p.rows.iter().map(|r| r.len() as u64).sum::<u64>() + 1_000_000;
        let max_bt = tunable!("ENOMOTO_T_MIP_SHIFTPROP_BACKTRACKS", 20usize, usize);
        let _ = self.dom.take_changed();
        // 決定の記録: (列, 残りの候補, 決定前の定義域の記録位置)
        let mut decisions: Vec<(usize, Vec<f64>, usize)> = Vec::new();
        let mut backtracks = 0usize;
        let mut row_ptr = 0usize;
        let mut ok = true;
        let mut steps = 0usize;
        // 次に固定する列と候補 (違反数の少ない順) を選ぶ
        let pick = |s: &Self, x: &[f64], act: &[f64], row_ptr: &mut usize| -> Option<(usize, Vec<f64>)> {
            let cands_of = |j: usize| -> Vec<(u32, f64, f64)> {
                let (lo, up) = (s.dom.lo[j], s.dom.up[j]);
                let mut cands: Vec<f64> = vec![x[j].clamp(lo, up).round()];
                if lo.is_finite() {
                    cands.push(lo);
                }
                if up.is_finite() {
                    cands.push(up);
                }
                for &(i, a) in &p.cols[j] {
                    let need = if act[i] < p.row_lo[i] - tol(p.row_lo[i]) {
                        p.row_lo[i] - act[i]
                    } else if act[i] > p.row_up[i] + tol(p.row_up[i]) {
                        p.row_up[i] - act[i]
                    } else {
                        continue;
                    };
                    let d = need / a;
                    let v = x[j] + if (need > 0.0) == (a > 0.0) { (d - 1e-9).ceil() } else { (d + 1e-9).floor() };
                    if v.is_finite() {
                        cands.push(v.round().clamp(lo, up));
                    }
                }
                cands.sort_by(|a, b| a.total_cmp(b));
                cands.dedup();
                let mut scored: Vec<(u32, f64, f64)> = cands
                    .iter()
                    .map(|&v| {
                        let dv = v - x[j];
                        let nv: u32 = p.cols[j].iter().map(|&(i, a)| viol(i, act[i] + a * dv)).sum();
                        (nv, p.cost[j] * v, v)
                    })
                    .collect();
                scored.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
                scored
            };
            // 違反している行 (前回の位置から巡回) の未固定の整数列のうち、違反数を最も減らすもの
            for k in 0..p.m {
                let i = (*row_ptr + k) % p.m;
                if viol(i, act[i]) == 0 {
                    continue;
                }
                let mut best: Option<(i64, usize, Vec<(u32, f64, f64)>)> = None;
                for &(j, _) in &p.rows[i] {
                    if !p.is_int[j] || s.dom.is_fixed(j) {
                        continue;
                    }
                    let sc = cands_of(j);
                    let now: u32 = p.cols[j].iter().map(|&(i2, _)| viol(i2, act[i2])).sum();
                    let gain = now as i64 - sc[0].0 as i64;
                    if best.as_ref().is_none_or(|b| gain > b.0) {
                        best = Some((gain, j, sc));
                    }
                }
                if let Some((_, j, sc)) = best {
                    *row_ptr = i;
                    return Some((j, sc.iter().take(4).map(|t| t.2).collect()));
                }
            }
            // 違反行に未固定の整数列がない: 残りの整数列を今の値に
            (0..n).find(|&j| p.is_int[j] && !s.dom.is_fixed(j)).map(|j| (j, cands_of(j).iter().take(4).map(|t| t.2).collect()))
        };
        'outer: loop {
            steps += 1;
            if steps % 64 == 0 && (self.time_up() || self.dom.debug_work() - work0 > work_cap) {
                ok = false;
                break;
            }
            let Some((j, cands)) = pick(self, &x, &act, &mut row_ptr) else { break };
            let mut pending: Option<(usize, Vec<f64>, usize)> = Some((j, cands, self.dom.stack_len()));
            // 候補を順に試す。全部駄目なら前の決定の次の候補へ戻る
            loop {
                let (dj, mut rest, dpos) = pending.take().unwrap();
                let mut fixed = false;
                while !rest.is_empty() {
                    let v = rest.remove(0);
                    self.dom.tighten_lower(p, dj, v);
                    self.dom.tighten_upper(p, dj, v);
                    if self.dom.propagate(p) {
                        fixed = true;
                        break;
                    }
                    self.dom.backtrack_to(p, dpos);
                }
                if fixed {
                    decisions.push((dj, rest, dpos));
                    break;
                }
                if backtracks >= max_bt {
                    ok = false;
                    break 'outer;
                }
                let Some(d) = decisions.pop() else {
                    ok = false;
                    break 'outer;
                };
                backtracks += 1;
                self.dom.backtrack_to(p, d.2);
                pending = Some(d);
            }
            // 境界の変わった列の点と行の活動量を更新する
            for c in self.dom.take_changed() {
                let nv = val(self.dom.lo[c], self.dom.up[c]);
                if nv != x[c] {
                    let dv = nv - x[c];
                    for &(i, a) in &p.cols[c] {
                        act[i] += a * dv;
                    }
                    x[c] = nv;
                }
            }
        }
        if self.params.verbose && self.nodes <= 1 {
            let nviol = (0..p.m).filter(|&i| viol(i, act[i]) > 0).count();
            eprintln!("MIP:   shift-and-propagate: {} ({} decisions, {backtracks} backtracks, {nviol} rows violated before the LP)", if ok { "all integer columns fixed" } else { "gave up" }, decisions.len());
        }
        let target: Vec<f64> = (0..n).map(|j| if self.dom.is_fixed(j) { self.dom.lo[j] } else { x[j] }).collect();
        self.dom.backtrack_to(p, pos);
        if !ok {
            return false;
        }
        let ints: Vec<usize> = decisions.iter().map(|d| d.0).collect();
        self.fix_and_propagate(&target, &ints)
    }

    /// ランダム丸め: しきい値を乱数にして丸めた点を fix_and_propagate で試す (整数に近い列から固定)。
    pub(super) fn randomized_rounding(&mut self, x: &[f64], tries: usize) -> bool {
        let p = self.p;
        let mut order: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j]).collect();
        order.sort_by(|&a, &b| {
            let fa = (x[a] - x[a].round()).abs();
            let fb = (x[b] - x[b].round()).abs();
            fa.total_cmp(&fb)
        });
        for t in 0..tries {
            let t0 = std::time::Instant::now();
            let mut target = x.to_vec();
            for &j in &order {
                let th = if t == 0 { 0.5 } else { 0.1 + 0.8 * self.rand() };
                target[j] = (x[j] + 1.0 - th).floor();
            }
            let found = self.fix_and_propagate(&target, &order);
            if env_str!("ENOMOTO_MIP_DEBUG_RR").is_some() {
                eprintln!("RR try {t}: {:.3}s found {found} prop_work {}", t0.elapsed().as_secs_f64(), self.dom.debug_work());
            }
            if found {
                return true;
            }
            if self.time_up() {
                break;
            }
        }
        false
    }

    /// oneopt (SCIP の `heur_oneopt`): 実行可能解 `x` の整数列を 1 つずつ、目的値が良くなる向きに、どの行も
    /// 破らない最大の整数幅だけ動かす。改善の大きい列から順に適用し、何も動かなくなるまで繰り返す。
    /// 連続列は動かさないので、結果も実行可能。動かしたら真。
    pub(super) fn one_opt(&mut self, x: &mut [f64]) -> bool {
        let p = self.p;
        if self.params.submip || !p.is_int.iter().any(|&b| b) {
            return false;
        }
        let cols = self.ensure_col_rows();
        let mut act: Vec<f64> = p.rows.iter().map(|r| r.iter().map(|&(j, a)| a * x[j]).sum()).collect();
        // 列 j を向き dir (+1/-1) に動かせる最大の整数幅
        let max_shift = |j: usize, dir: f64, x: &[f64], act: &[f64], lo: &[f64], up: &[f64]| -> f64 {
            let mut sh = if dir > 0.0 { up[j] - x[j] } else { x[j] - lo[j] };
            if !sh.is_finite() {
                return 0.0;
            }
            for &(i, a) in &cols[j] {
                let da = a * dir; // 1 単位動かしたときの活動量の変化
                if da > 0.0 {
                    sh = sh.min((p.row_up[i] - act[i] + 1e-9) / da);
                } else if da < 0.0 {
                    sh = sh.min((act[i] - p.row_lo[i] + 1e-9) / -da);
                }
                if sh < 1.0 {
                    return 0.0;
                }
            }
            sh.floor().max(0.0)
        };
        let (lo, up) = (self.dom.global_lo.clone(), self.dom.global_up.clone());
        let mut improved = false;
        for _pass in 0..10 {
            let mut cands: Vec<(usize, f64, f64)> = Vec::new(); // (列, 向き, 改善量)
            for j in 0..p.n {
                if !p.is_int[j] || p.cost[j] == 0.0 {
                    continue;
                }
                let dir = if p.cost[j] > 0.0 { -1.0 } else { 1.0 };
                let sh = max_shift(j, dir, x, &act, &lo, &up);
                if sh >= 1.0 {
                    cands.push((j, dir, p.cost[j].abs() * sh));
                }
            }
            if cands.is_empty() {
                break;
            }
            cands.sort_by(|a, b| b.2.total_cmp(&a.2));
            let mut moved = false;
            for (j, dir, _) in cands {
                let sh = max_shift(j, dir, x, &act, &lo, &up);
                if sh < 1.0 {
                    continue;
                }
                x[j] += dir * sh;
                for &(i, a) in &cols[j] {
                    act[i] += a * dir * sh;
                }
                moved = true;
            }
            if !moved {
                break;
            }
            improved = true;
        }
        improved
    }

    /// 分数ダイビング: LP 解で整数に最も近い分数の列をその側に丸めて固定し、伝播して LP を解き直す、を
    /// 整数解になるまで繰り返す。固定で実行不能になったら 1 回だけ反対側を試す。`budget` は LP 反復の上限。
    /// 定義域と LP は呼ぶ前の状態に戻す。
    pub(super) fn fractional_dive(&mut self, budget: u64) -> bool {
        self.dive(DiveKind::Fractional, budget, f64::INFINITY)
    }

    /// 列ごとの (行, 係数) を (なければ) 作る。
    pub(super) fn ensure_col_rows(&mut self) -> std::rc::Rc<Vec<Vec<(usize, f64)>>> {
        if self.col_rows.is_none() {
            let mut cols: Vec<Vec<(usize, f64)>> = vec![Vec::new(); self.p.n];
            for (i, r) in self.p.rows.iter().enumerate() {
                for &(j, a) in r {
                    cols[j].push((i, a));
                }
            }
            self.col_rows = Some(std::rc::Rc::new(cols));
        }
        self.col_rows.clone().unwrap()
    }

    /// ダイビング (SCIP の `SCIPperformGenericDivingAlgorithm` の簡略版)。`kind` で丸める列と向きを選び、
    /// 固定 → 伝播 → LP の解き直しを整数解になるまで続ける。固定で実行不能 (または目的値が `search_bound` 以上)
    /// になったら反対側を試し、両側とも駄目なら前の決定に戻って反対側に変える (後戻り、最大
    /// `ENOMOTO_T_MIP_DIVE_BACKTRACKS` 回)。`budget` は LP 反復の上限。定義域と LP は戻す。
    pub(super) fn dive(&mut self, kind: DiveKind, budget: u64, search_bound: f64) -> bool {
        let p = self.p;
        let verbose = self.params.verbose && self.nodes <= 1;
        let cols = self.ensure_col_rows();
        let saved = self.lp.save_state();
        let pos = self.dom.stack_len();
        let it_start = self.lp.total_iterations();
        // 後戻りは根でだけ (ノードのダイビングは数が多く、後戻りの反復が探索の時間を食う)
        let max_bt = if self.nodes <= 1 { tunable!("ENOMOTO_T_MIP_DIVE_BACKTRACKS", 10usize, usize) } else { tunable!("ENOMOTO_T_MIP_NODE_DIVE_BACKTRACKS", 0usize, usize) };
        let guide = if kind == DiveKind::Guided { self.incumbent.as_ref().map(|(_, x)| x.clone()) } else { None };
        if kind == DiveKind::Guided && guide.is_none() {
            return false;
        }
        let cmax = p.cost.iter().fold(0.0f64, |m, c| m.max(c.abs()));
        // 衝突制約 (目的値に依存しない行 sum a x <= rhs) による lock: a > 0 なら上げると破れうる
        let clocks: Vec<(u32, u32)> = if kind == DiveKind::Conflict {
            let mut l = vec![(0u32, 0u32); p.n];
            for (c, _, obj) in &self.dual_proofs {
                if *obj {
                    continue;
                }
                for &(j, a) in c {
                    if a > 0.0 {
                        l[j].1 += 1;
                    } else {
                        l[j].0 += 1;
                    }
                }
            }
            l
        } else {
            Vec::new()
        };
        // 決定の記録: (列, 値, 上へ, 決定前の定義域の記録位置, 反対側を試し済みか)
        let mut decisions: Vec<(usize, f64, bool, usize, bool)> = Vec::new();
        let mut backtracks = 0usize;
        let mut found = false;
        let mut steps = 0usize;
        'dive: while steps < 4 * p.n + 10 {
            steps += 1;
            if self.time_up() || self.lp.total_iterations() - it_start > budget {
                if verbose {
                    eprintln!("MIP:   dive ran out of budget at depth {} ({} iterations, {backtracks} backtracks)", decisions.len(), self.lp.total_iterations() - it_start);
                }
                break;
            }
            let x = self.lp.col_values();
            let frac = self.fractional(&x);
            if frac.is_empty() {
                found = self.try_lp_solution();
                if verbose {
                    eprintln!("MIP:   dive reached an integral LP at depth {} (accepted {found}, objective {}, {backtracks} backtracks)", decisions.len(), self.lp.objective() + p.offset);
                }
                break;
            }
            // 丸める列と最初に試す向きを選ぶ (スコアの小さいもの)
            let mut pick: Option<(usize, f64, bool, f64)> = None; // (列, 値, 上へ, スコア)
            for &(j, v) in &frac {
                let f = v - v.floor();
                let (dl, ul) = self.locks[j];
                let binary = self.dom.global_lo[j] == 0.0 && self.dom.global_up[j] == 1.0;
                let (up, score) = match kind {
                    DiveKind::Fractional => {
                        // SCIP の fracdiving: 片側に自明に丸められる列は反対側へ (自明な側は後でいつでも選べる)。
                        // それ以外は近い整数へ。自明に丸められる列・2 値でない列は後回し。
                        let (may_down, may_up) = (dl == 0, ul == 0);
                        let up = if may_down && !may_up {
                            true
                        } else if may_up && !may_down {
                            false
                        } else {
                            f >= 0.5
                        };
                        let dist = if up { 1.0 - f } else { f };
                        (up, dist + if may_down || may_up { 1.0 } else { 0.0 } + if binary { 0.0 } else { 0.5 })
                    }
                    DiveKind::VectorLength => {
                        // SCIP の veclendiving: 目的値の悪くなる向きに丸め、(目的値の悪化) / (列の長さ + 1) の小さい列から
                        // (長い列を 1 にすると多くの行が満たされる。集合被覆・分割向け)
                        let up = p.cost[j] >= 0.0;
                        let dist = if up { 1.0 - f } else { f };
                        let delta = p.cost[j].abs() * dist + 1e-6 * dist;
                        (up, delta / (cols[j].len() as f64 + 1.0))
                    }
                    DiveKind::Coefficient => {
                        // SCIP の coefdiving: lock の少ない向きへ丸め、その lock 数の少ない列から (同数なら近いもの)。
                        // 自明に丸められる列 (lock 0) は後回し
                        let up = if dl != ul { ul < dl } else { f >= 0.5 };
                        let lk = if up { ul } else { dl };
                        let dist = if up { 1.0 - f } else { f };
                        let trivial = if dl == 0 || ul == 0 { 1e6 } else { 0.0 };
                        (up, trivial + lk as f64 + dist + if binary { 0.0 } else { 0.5 })
                    }
                    DiveKind::Pseudocost => {
                        // SCIP の pscostdiving: 端に近ければその側、そうでなければ擬費用の小さい側へ。
                        // (この側の費用) / (反対側の費用) の小さい、向きのはっきりした列から
                        let (cu, cd) = (self.pc.cost_up(j) * (1.0 - f), self.pc.cost_down(j) * f);
                        let up = if f < 0.3 {
                            false
                        } else if f > 0.7 {
                            true
                        } else {
                            cu < cd
                        };
                        let (this, other) = if up { (cu, cd) } else { (cd, cu) };
                        (up, (this + 1e-6) / (other + 1e-6) + if binary { 0.0 } else { 1.0 })
                    }
                    DiveKind::Guided => {
                        // SCIP の guideddiving: 暫定解の値の側へ、暫定解との差の小さい列から
                        let g = guide.as_ref().unwrap()[j];
                        let up = g >= v;
                        ((up), (g - v).abs() + if binary { 0.0 } else { 0.5 })
                    }
                    DiveKind::Farkas => {
                        // 目的値の良くなる向き (費用 0 なら近い側) へ、費用の絶対値の大きい列から
                        let c = p.cost[j];
                        let up = if c != 0.0 { c < 0.0 } else { f >= 0.5 };
                        let dist = if up { 1.0 - f } else { f };
                        (up, -c.abs() / (1.0 + cmax) + 0.01 * dist + if binary { 0.0 } else { 0.5 })
                    }
                    DiveKind::Conflict => {
                        // 衝突制約の lock の少ない向きへ (同数なら通常の lock、それも同数なら近い側)
                        let (cdl, cul) = clocks[j];
                        let up = if cdl != cul { cul < cdl } else if dl != ul { ul < dl } else { f >= 0.5 };
                        let lk = if up { cul } else { cdl };
                        let dist = if up { 1.0 - f } else { f };
                        (up, lk as f64 + dist + if binary { 0.0 } else { 0.5 })
                    }
                };
                if pick.is_none_or(|(_, _, _, s)| score < s) {
                    pick = Some((j, v, up, score));
                }
            }
            let (j, v, up_first, _) = pick.unwrap();
            let before = self.dom.stack_len();
            if self.dive_try(j, v, up_first, budget, it_start, search_bound) {
                decisions.push((j, v, up_first, before, false));
                continue;
            }
            if self.dive_try(j, v, !up_first, budget, it_start, search_bound) {
                decisions.push((j, v, !up_first, before, true));
                continue;
            }
            // 両側とも駄目: 反対側を試していない決定まで戻ってそちらに変える
            loop {
                if backtracks >= max_bt {
                    if verbose {
                        eprintln!("MIP:   dive stopped at depth {} with {} fractional ({backtracks} backtracks)", decisions.len(), frac.len());
                    }
                    break 'dive;
                }
                let Some((dj, dv, dup, dpos, tried)) = decisions.pop() else {
                    if verbose {
                        eprintln!("MIP:   dive exhausted ({backtracks} backtracks)");
                    }
                    break 'dive;
                };
                self.dom.backtrack_to(p, dpos);
                self.sync_lp();
                if tried {
                    continue;
                }
                backtracks += 1;
                if self.dive_try(dj, dv, !dup, budget, it_start, search_bound) {
                    decisions.push((dj, dv, !dup, dpos, true));
                    break;
                }
            }
        }
        self.dom.backtrack_to(p, pos);
        self.sync_lp();
        self.lp.restore_state(&saved);
        found
    }

    /// ダイビングの 1 つの決定: 列 `j` を `v` の上 (下) の整数に締め、伝播して LP を解く。LP が最適で目的値が
    /// `search_bound` と打ち切り値より小さければ真。駄目なら定義域を戻して偽。
    fn dive_try(&mut self, j: usize, v: f64, up: bool, budget: u64, it_start: u64, search_bound: f64) -> bool {
        let p = self.p;
        let before = self.dom.stack_len();
        if up {
            self.dom.tighten_lower(p, j, v.ceil());
        } else {
            self.dom.tighten_upper(p, j, v.floor());
        }
        let prop_ok = self.dom.propagate(p);
        if !prop_ok && env_str!("ENOMOTO_MIP_DIVE_CONFLICTS").is_some() {
            self.add_conflict();
        }
        if prop_ok {
            self.sync_lp();
            let it0 = self.lp.total_iterations();
            let lim = budget.saturating_sub(self.lp.total_iterations() - it_start).max(1000);
            let st = self.lp.solve(&self.limits(lim));
            self.heur_iters += self.lp.total_iterations() - it0;
            if st == LpStatus::Optimal && self.lp.objective() + p.offset < search_bound.min(self.prune_limit()) {
                return true;
            }
        }
        self.dom.backtrack_to(p, before);
        self.sync_lp();
        false
    }

    /// Feasibility Pump (根で暫定解がないときに使う)。LP は根の最適解の状態から始め、最後に戻す。
    pub(super) fn feasibility_pump(&mut self, root_iters: u64) -> bool {
        let p = self.p;
        let n = p.n;
        if !p.is_int.iter().any(|&b| b) {
            return false;
        }
        let saved = self.lp.save_state();
        let budget = 1000 + 5 * root_iters;
        let it_start = self.lp.total_iterations();
        let mut seen: HashSet<Vec<i64>> = HashSet::new();
        let ints: Vec<usize> = (0..n).filter(|&j| p.is_int[j]).collect();
        let mut found = false;
        // Objective Feasibility Pump: 距離関数に元の目的関数を重み alpha で混ぜ、反復ごとに減衰させる。
        let cnorm = p.cost.iter().map(|c| c * c).sum::<f64>().sqrt();
        let mut alpha = if cnorm > 0.0 { 1.0f64 } else { 0.0 };
        for _pass in 0..100 {
            if self.time_up() || self.lp.total_iterations() - it_start > budget {
                break;
            }
            let x = self.lp.col_values();
            if self.fractional(&x).is_empty() {
                // LP 解が整数: 元の費用でなくても実行可能解
                found = self.try_lp_solution();
                break;
            }
            let mut r = x.clone();
            for &j in &ints {
                let th = 0.4 + 0.2 * self.rand();
                r[j] = (x[j] + 1.0 - th).floor().clamp(self.dom.lo[j], self.dom.up[j]);
            }
            let mut key: Vec<i64> = ints.iter().map(|&j| r[j] as i64).collect();
            let mut cycles = 0;
            while seen.contains(&key) && cycles < 2 {
                // 循環: ランダムに 10 個を反転
                for _ in 0..10 {
                    let k = (self.rand() * ints.len() as f64) as usize % ints.len();
                    let j = ints[k];
                    let v = if r[j] > x[j] { x[j].floor() } else if r[j] < x[j] { x[j].ceil() } else if r[j] < self.dom.up[j] { self.dom.up[j] } else { self.dom.lo[j] };
                    r[j] = v.clamp(self.dom.lo[j], self.dom.up[j]);
                    key[k] = r[j] as i64;
                }
                cycles += 1;
            }
            if seen.contains(&key) {
                break;
            }
            seen.insert(key);
            // 丸めた点の固定と伝播で実行可能解になるか
            // 伝播を使った丸め (Feasibility Pump 2.0): 整数に近い列から固定して伝播し、後の列は締まった
            // 定義域の中で丸める。矛盾したらそこで伝播を止める。得た点を距離の目標にする。
            let mut order = ints.clone();
            order.sort_by(|&a, &b| (x[a] - r[a]).abs().total_cmp(&(x[b] - r[b]).abs()));
            let mut rp = r.clone();
            if self.propagate_rounding(&r, &order, Some(&mut rp)) {
                found = true;
                break;
            }
            r = rp;
            // 距離の目的
            let mut c = vec![0.0; n];
            for &j in &ints {
                let (dl, ul) = self.locks[j];
                if dl == 0 || ul == 0 {
                    continue;
                }
                let noise = 1e-4 * (self.rand() - 0.5);
                c[j] = if r[j] <= self.dom.lo[j] {
                    1.0
                } else if r[j] >= self.dom.up[j] {
                    -1.0
                } else if x[j] > r[j] {
                    1.0
                } else {
                    -1.0
                } + noise;
            }
            if alpha > 1e-3 {
                // 距離の項の大きさ (||Δ||) に目的関数の大きさを合わせて混ぜる
                let dnorm = c.iter().map(|v| v * v).sum::<f64>().sqrt();
                let w = alpha * dnorm.max(1.0) / cnorm;
                for j in 0..n {
                    c[j] = (1.0 - alpha) * c[j] + w * p.cost[j];
                }
            }
            alpha *= 0.9;
            self.lp.set_costs(&c);
            let it0 = self.lp.total_iterations();
            let lim = budget.saturating_sub(self.lp.total_iterations() - it_start).max(10);
            let st = self.lp.solve_primal(&SolveLimits { iteration_limit: lim, cutoff: f64::INFINITY, deadline: self.deadline });
            self.heur_iters += self.lp.total_iterations() - it0;
            if st != LpStatus::Optimal || self.lp.total_iterations() == it0 {
                break;
            }
        }
        self.lp.set_costs(&p.cost);
        self.lp.restore_state(&saved);
        found
    }

    /// Feasibility Jump (重み付きの局所探索、LP 不要)。`effort` は列の評価回数の目安。
    pub(super) fn feasibility_jump(&mut self, effort: u64, time_cap: f64) -> bool {
        let seed = self.rng ^ 0x9E37_79B9_7F4A_7C15;
        self.rand();
        match fj_search(self.p, &self.dom.lo, &self.dom.up, effort, time_cap, seed, None) {
            Some(x) => self.try_incumbent(x),
            None => false,
        }
    }
}

/// 局所探索 (Local-MIP・ViolationLS 系、Feasibility Jump の発展): 違反している行を重み付きで減らす手を選ぶ。
/// FJ との違い: 行の重みの平滑化 (重みは確率的に増やし、満たされた行の重みは減らす)、逆戻りの禁止 (タブー)、
/// 改善する手がなければランダムな手、何度かの再出発、実行可能解が見つかったら目的値をそれより良くする行を
/// 足して続ける。戻り値は (最良の実行可能解, 違反の合計が最小だった点とその違反)。
pub(super) fn local_search(p: &MipProblem, lo: &[f64], up: &[f64], time_cap: f64, max_steps: u64, seed: u64) -> (Option<Vec<f64>>, Option<(Vec<f64>, f64)>) {
    let (n, m) = (p.n, p.m);
    let mut rng = seed | 1;
    let mut rand = move || {
        rng ^= rng >> 12;
        rng ^= rng << 25;
        rng ^= rng >> 27;
        ((rng.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64) / ((1u64 << 53) as f64)
    };
    let t_end = Instant::now() + Duration::from_secs_f64(time_cap);
    if n == 0 {
        return (None, None);
    }
    // 目的値の行 (index m): c x <= obj_bound (実行可能解が見つかってから有効)
    let mut obj_bound = f64::INFINITY;
    let row_lo = |i: usize| if i < m { p.row_lo[i] } else { f64::NEG_INFINITY };
    let row_up = |i: usize, ob: f64| if i < m { p.row_up[i] } else { ob };
    let viol = |i: usize, a: f64, ob: f64| -> f64 {
        let (rl, ru) = (row_lo(i), row_up(i, ob));
        let tol = FEASTOL * (1.0 + a.abs().min(1e6));
        if a > ru + tol {
            a - ru
        } else if a < rl - tol {
            rl - a
        } else {
            0.0
        }
    };
    // 列ごとの (行, 係数)。目的値の行を含める
    let cols: Vec<Vec<(usize, f64)>> = (0..n)
        .map(|j| {
            let mut c = p.cols[j].clone();
            if p.cost[j] != 0.0 {
                c.push((m, p.cost[j]));
            }
            c
        })
        .collect();
    let start = |rand: &mut dyn FnMut() -> f64| -> Vec<f64> {
        (0..n)
            .map(|j| {
                let (l, u) = (lo[j], up[j]);
                let v = if rand() < 0.5 { 0.0f64.max(l).min(u.max(l)) } else if l.is_finite() && u.is_finite() && p.is_int[j] { (l + ((u - l + 1.0) * rand()).floor()).min(u) } else { 0.0f64.max(l).min(u.max(l)) };
                if v.is_finite() { v } else { 0.0 }
            })
            .collect()
    };
    let mut best_feas: Option<Vec<f64>> = None;
    let mut best_inf: Option<(Vec<f64>, f64)> = None;
    let mut steps_total = 0u64;
    'restart: for round in 0..1000 {
        let mut x = if round == 0 { (0..n).map(|j| { let v = 0.0f64.max(lo[j]).min(up[j].max(lo[j])); if v.is_finite() { v } else { 0.0 } }).collect() } else { start(&mut rand) };
        if let Some(b) = &best_feas {
            if rand() < 0.5 {
                x = b.clone();
            }
        }
        let mut act: Vec<f64> = (0..m).map(|i| p.rows[i].iter().map(|&(j, a)| a * x[j]).sum()).collect();
        act.push((0..n).map(|j| p.cost[j] * x[j]).sum());
        let mut w = vec![1.0f64; m + 1];
        let mut tabu_until = vec![0u64; n];
        let mut vset: Vec<usize> = Vec::new();
        let mut vpos = vec![usize::MAX; m + 1];
        for i in 0..=m {
            if viol(i, act[i], obj_bound) > 0.0 {
                vpos[i] = vset.len();
                vset.push(i);
            }
        }
        let mut stall = 0u64;
        let mut best_round = f64::INFINITY;
        let mut step = 0u64;
        loop {
            step += 1;
            steps_total += 1;
            if steps_total >= max_steps || (step % 128 == 0 && Instant::now() >= t_end) {
                break 'restart;
            }
            if vset.is_empty() {
                // 実行可能: 記録し、目的値をそれより良くする行を有効にして続ける
                let z = act[m];
                best_feas = Some(x.clone());
                let step_obj = 1e-6 * z.abs().max(1.0);
                obj_bound = z - step_obj;
                if viol(m, act[m], obj_bound) > 0.0 && vpos[m] == usize::MAX {
                    vpos[m] = vset.len();
                    vset.push(m);
                }
                if vset.is_empty() {
                    break 'restart;
                }
            }
            // 違反の合計 (目的値の行を除く) で最良の非実行可能点を記録
            let tv: f64 = vset.iter().filter(|&&i| i < m).map(|&i| viol(i, act[i], obj_bound)).sum();
            if tv < best_round - 1e-9 {
                best_round = tv;
                stall = 0;
                if best_inf.as_ref().is_none_or(|(_, v)| tv < *v) && tv > 0.0 {
                    best_inf = Some((x.clone(), tv));
                }
            } else {
                stall += 1;
                if stall > 20_000 {
                    continue 'restart;
                }
            }
            // 違反している行をいくつか標本にとり、その変数の「行を満たす値」への手を評価する
            let mut best: Option<(usize, f64, f64)> = None;
            for _ in 0..3.min(vset.len()) {
                let i = vset[(rand() * vset.len() as f64) as usize % vset.len()];
                let row: &[(usize, f64)] = if i < m { &p.rows[i] } else { &[] };
                let obj_row: Vec<(usize, f64)>;
                let row = if i == m {
                    obj_row = (0..n).filter(|&j| p.cost[j] != 0.0).map(|j| (j, p.cost[j])).collect();
                    &obj_row[..]
                } else {
                    row
                };
                for &(j, a) in row {
                    if lo[j] == up[j] || tabu_until[j] > step {
                        continue;
                    }
                    let (rl, ru) = (row_lo(i), row_up(i, obj_bound));
                    let need = if act[i] > ru { ru - act[i] } else { rl - act[i] };
                    let mut v = x[j] + need / a;
                    if p.is_int[j] {
                        v = if (need / a) > 0.0 { (v - 1e-9).ceil() } else { (v + 1e-9).floor() };
                    }
                    let v = v.clamp(lo[j], up[j]);
                    if v == x[j] || !v.is_finite() {
                        continue;
                    }
                    let d = v - x[j];
                    let mut gain = 0.0;
                    for &(r, b) in &cols[j] {
                        let before = viol(r, act[r], obj_bound);
                        let after = viol(r, act[r] + b * d, obj_bound);
                        gain += w[r] * (before - after);
                    }
                    if best.is_none_or(|(_, _, g)| gain > g) {
                        best = Some((j, v, gain));
                    }
                }
            }
            let mv = match best {
                Some((j, v, g)) if g > 1e-12 => Some((j, v)),
                _ => {
                    // 改善する手がない: 重みを更新し (平滑化: 確率 0.3 で満たされた行の重みを減らす)、ランダムな手
                    if rand() < 0.3 {
                        for r in 0..=m {
                            if vpos[r] == usize::MAX && w[r] > 1.0 {
                                w[r] -= 1.0;
                            }
                        }
                    } else {
                        for &r in &vset {
                            w[r] += 1.0;
                        }
                    }
                    best.map(|(j, v, _)| (j, v)).filter(|_| rand() < 0.5)
                }
            };
            let Some((j, v)) = mv else { continue };
            let d = v - x[j];
            x[j] = v;
            tabu_until[j] = step + 3 + (rand() * 10.0) as u64;
            for &(r, b) in &cols[j] {
                act[r] += b * d;
                let is_v = viol(r, act[r], obj_bound) > 0.0;
                if is_v && vpos[r] == usize::MAX {
                    vpos[r] = vset.len();
                    vset.push(r);
                } else if !is_v && vpos[r] != usize::MAX {
                    let k = vpos[r];
                    let last = *vset.last().unwrap();
                    vset.swap_remove(k);
                    if last != r {
                        vpos[last] = k;
                    }
                    vpos[r] = usize::MAX;
                }
            }
        }
    }
    (best_feas, best_inf)
}

/// Feasibility Jump の探索本体 (Solver の状態に依らないので別スレッドでも動かせる)。`lo`/`up` は列の境界、
/// `seed` は乱数の種、`stop` が立ったら止める。実行可能な点が見つかれば返す。
pub(super) fn fj_search(p: &MipProblem, lo: &[f64], up: &[f64], effort: u64, time_cap: f64, seed: u64, stop: Option<&std::sync::atomic::AtomicBool>) -> Option<Vec<f64>> {
    let mut rng = seed | 1;
    let mut rand = move || {
        rng ^= rng >> 12;
        rng ^= rng << 25;
        rng ^= rng >> 27;
        ((rng.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64) / ((1u64 << 53) as f64)
    };
    {
        let (n, m) = (p.n, p.m);
        if n == 0 {
            return None;
        }
        let t_end = Instant::now() + Duration::from_secs_f64(time_cap);
        let lo = lo.to_vec();
        let up = up.to_vec();
        // 初期点: 0 に最も近い境界内の値
        let mut x: Vec<f64> = (0..n)
            .map(|j| {
                let v = 0.0f64.max(lo[j]).min(up[j].max(lo[j]));
                if v.is_finite() { v } else { 0.0 }
            })
            .collect();
        let mut act: Vec<f64> = (0..m).map(|i| p.rows[i].iter().map(|&(j, a)| a * x[j]).sum()).collect();
        let mut w = vec![1.0f64; m];
        let viol = |i: usize, a: f64| -> f64 {
            let tol = FEASTOL * (1.0 + a.abs().min(1e6));
            if a > p.row_up[i] + tol {
                a - p.row_up[i]
            } else if a < p.row_lo[i] - tol {
                p.row_lo[i] - a
            } else {
                0.0
            }
        };
        // 違反している行の集合 (位置の索引つき)
        let mut vset: Vec<usize> = Vec::new();
        let mut vpos = vec![usize::MAX; m];
        for i in 0..m {
            if viol(i, act[i]) > 0.0 {
                vpos[i] = vset.len();
                vset.push(i);
            }
        }
        let mut last_move = vec![0u64; n];
        let mut work = 0u64;
        let mut step = 0u64;
        let mut cand: Vec<f64> = Vec::new();
        while work < effort {
            step += 1;
            if step % 256 == 0 && (Instant::now() >= t_end || stop.is_some_and(|f| f.load(std::sync::atomic::Ordering::Relaxed))) {
                break;
            }
            if vset.is_empty() {
                return Some(x);
            }
            let i = vset[(rand() * vset.len() as f64) as usize % vset.len()];
            // 行 i の各変数について、列の行の重み付き違反を最小にする値を探す
            let mut best: Option<(usize, f64, f64)> = None; // (列, 値, 改善量)
            for &(j, _) in &p.rows[i] {
                if lo[j] == up[j] || step - last_move[j] < 3 && last_move[j] != 0 {
                    continue;
                }
                cand.clear();
                for &(r, a) in &p.cols[j] {
                    for b in [p.row_lo[r], p.row_up[r]] {
                        if b.is_finite() {
                            cand.push(x[j] + (b - act[r]) / a);
                        }
                    }
                    if cand.len() > 64 {
                        break;
                    }
                }
                if lo[j].is_finite() {
                    cand.push(lo[j]);
                }
                if up[j].is_finite() {
                    cand.push(up[j]);
                }
                let cur: f64 = p.cols[j].iter().map(|&(r, _)| w[r] * viol(r, act[r])).sum();
                let nc = cand.len();
                for k in 0..nc {
                    let v0 = cand[k].max(lo[j]).min(up[j].max(lo[j]));
                    let vals: [f64; 2] = if p.is_int[j] { [v0.floor().max(lo[j]), v0.ceil().min(up[j])] } else { [v0, v0] };
                    for (q, &v) in vals.iter().enumerate() {
                        if q == 1 && vals[1] == vals[0] {
                            continue;
                        }
                        if !v.is_finite() || v == x[j] {
                            continue;
                        }
                        let d = v - x[j];
                        let mut new = 0.0;
                        for &(r, a) in &p.cols[j] {
                            new += w[r] * viol(r, act[r] + a * d);
                        }
                        work += p.cols[j].len() as u64;
                        let gain = cur - new - 1e-9 * p.cost[j] * d;
                        if best.is_none_or(|(_, _, g)| gain > g) {
                            best = Some((j, v, gain));
                        }
                    }
                }
            }
            match best {
                Some((j, v, g)) if g > 1e-12 => {
                    let d = v - x[j];
                    x[j] = v;
                    last_move[j] = step;
                    for &(r, a) in &p.cols[j] {
                        act[r] += a * d;
                        let is_v = viol(r, act[r]) > 0.0;
                        if is_v && vpos[r] == usize::MAX {
                            vpos[r] = vset.len();
                            vset.push(r);
                        } else if !is_v && vpos[r] != usize::MAX {
                            let k = vpos[r];
                            let last = *vset.last().unwrap();
                            vset.swap_remove(k);
                            if last != r {
                                vpos[last] = k;
                            }
                            vpos[r] = usize::MAX;
                        }
                    }
                }
                _ => {
                    // 局所最適: 違反している行の重みを上げる
                    for &r in &vset {
                        w[r] += 1.0;
                    }
                }
            }
        }
        None
    }
}

impl<'a, L: MipLp> Solver<'a, L> {
}

/// ダイビングの種類。
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DiveKind {
    /// 分数ダイビング (整数に近い列を近い側へ)。
    Fractional,
    /// ベクトル長ダイビング (目的値の悪化が列の長さの割に小さい列を、悪化する側へ)。
    VectorLength,
    /// 係数ダイビング (lock の少ない向きへ、lock の少ない列から)。
    Coefficient,
    /// 擬費用ダイビング (擬費用の小さい向きへ、向きのはっきりした列から)。
    Pseudocost,
    /// 誘導ダイビング (暫定解の値の側へ。暫定解があるときだけ)。
    Guided,
    /// Farkas ダイビング (SCIP の farkasdiving の簡略版: 目的値の良くなる向きへ、費用の絶対値の大きい列から)。
    Farkas,
    /// 衝突ダイビング (SCIP の conflictdiving の簡略版: 衝突制約の lock の少ない向きへ、その lock の少ない列から)。
    Conflict,
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 一部の整数列を固定した (境界を締めた) サブ MIP を、ノード数を制限して解く。
    /// 見つかった解は暫定解の候補にする。`lo`/`up` はサブ MIP の列の境界。
    pub(super) fn solve_submip(&mut self, lo: Vec<f64>, up: Vec<f64>, node_limit: u64) -> bool {
        let mut sub = self.p.clone();
        sub.col_lo = lo;
        sub.col_up = up;
        self.solve_submip_problem(sub, node_limit, None).is_some_and(|r| r.1)
    }

    /// 加工したサブ MIP (`sub`: 列は元と同じ並び) を解き、見つかった解を元の問題の暫定解の候補にする。
    /// `cutoff` はサブ MIP の目的値の打ち切り値 (`None` なら元の打ち切り値。目的関数を変えたサブ MIP では指定する)。
    /// 戻り値は (サブ MIP の状態, 解を受け入れたか)。
    fn solve_submip_problem(&mut self, mut sub: super::problem::MipProblem, node_limit: u64, cutoff: Option<f64>) -> Option<(super::solver::MipStatus, bool)> {
        let p = self.p;
        let (lo, up) = (std::mem::take(&mut sub.col_lo), std::mem::take(&mut sub.col_up));
        // 伝播の丸め誤差で下限 > 上限 (ごくわずか) になった列は 1 点に固定する
        let (mut lo, mut up) = (lo, up);
        for j in 0..p.n {
            if lo[j] > up[j] {
                let v = if p.is_int[j] { lo[j].round() } else { 0.5 * (lo[j] + up[j]) };
                lo[j] = v;
                up[j] = v;
            }
        }
        sub.col_lo = lo;
        sub.col_up = up;
        let remaining = match self.deadline {
            Some(d) => d.saturating_duration_since(Instant::now()).as_secs_f64(),
            None => f64::INFINITY,
        };
        let params = super::solver::MipParams {
            time_limit: (self.submip_time_frac * remaining).min(6.0),
            node_limit,
            rel_gap: self.params.rel_gap,
            abs_gap: self.params.abs_gap,
            verbose: false,
            submip: true,
            cutoff: cutoff.unwrap_or_else(|| self.prune_limit()),
            restarts: 0,
            skip_heurs: 0,
        };
        // 並列モード (根のヒューリスティクスの間): 別スレッドで解き始め、結果は後で受け取る
        if self.parallel_submips {
            self.spawn_submip(sub, params);
            return Some((super::solver::MipStatus::NotSolved, false));
        }
        let r = super::solve_problem(&sub, params, env_str!("ENOMOTO_MIP_SUBMIP_NO_PRESOLVE").is_none());
        if self.params.verbose {
            let nfree = (0..p.n).filter(|&j| sub.col_lo[j] < sub.col_up[j]).count();
            eprintln!("MIP:   sub-MIP: {nfree} free columns of {}, status {:?}, nodes {}, objective {:?}", p.n, r.status, r.nodes, r.objective);
        }
        self.heur_iters += r.lp_iterations;
        let acc = match r.x {
            Some(x) => self.try_incumbent(x),
            None => false,
        };
        Some((r.status, acc))
    }

    /// 固定 (列, 下限, 上限) を順に今の定義域に積み、1 つずつ伝播する。矛盾する固定は飛ばす (サブ MIP が丸ごと
    /// 実行不能になるのを防ぐ。HiGHS の RENS・RINS と同じ考え)。戻り値は (積んだ後の定義域の下限, 上限, 積めた数)。
    /// 定義域は戻す。
    fn propagated_fixings(&mut self, fixes: &[(usize, f64, f64)]) -> (Vec<f64>, Vec<f64>, usize) {
        let p = self.p;
        let pos = self.dom.stack_len();
        let mut applied = 0usize;
        if env_str!("ENOMOTO_MIP_SUBMIP_FIX_NO_PROP").is_some() {
            let (mut lo, mut up) = (self.dom.lo.clone(), self.dom.up.clone());
            for &(j, l, u) in fixes {
                lo[j] = lo[j].max(l);
                up[j] = up[j].min(u);
            }
            return (lo, up, fixes.len());
        }
        let work0 = self.dom.debug_work();
        let work_cap = 20 * p.rows.iter().map(|r| r.len() as u64).sum::<u64>() + 1_000_000;
        for (k, &(j, l, u)) in fixes.iter().enumerate() {
            if k % 64 == 0 && (self.time_up() || self.dom.debug_work() - work0 > work_cap) {
                break;
            }
            if self.dom.lo[j] >= l && self.dom.up[j] <= u {
                applied += 1;
                continue;
            }
            let before = self.dom.stack_len();
            self.dom.tighten_lower(p, j, l);
            self.dom.tighten_upper(p, j, u);
            if self.dom.propagate(p) {
                applied += 1;
            } else {
                self.dom.backtrack_to(p, before);
            }
        }
        let (lo, up) = (self.dom.lo.clone(), self.dom.up.clone());
        self.dom.backtrack_to(p, pos);
        (lo, up, applied)
    }

    /// RENS (HiGHS の `HighsPrimalHeuristics::RENS` に倣う): LP 解で整数値の整数列を、残りの自由な列の 1 割ずつ
    /// 伝播しながら固定し、そのたびに LP を解き直す (LP が実行不能になったら直前の段に戻る、最大 10 回)。整数値の列が
    /// なくなったら分数の列を丸めて固定する。整数列の固定率が目標 (成功・失敗した固定率から決める、初期 0.6) に
    /// 達したら、その定義域でサブ MIP を解く。`x` は使わない (今の LP 解から始める)。
    pub(super) fn rens(&mut self, _x: &[f64]) -> bool {
        let p = self.p;
        if self.params.submip {
            return false;
        }
        // 既定は旧来の全部固定 (ダイビング式は簡単な問題でサブ MIP に時間を取られ、40 問の幾何平均が
        // 29.91 -> 30.41 に悪化した)。ENOMOTO_MIP_RENS_DIVE でダイビング式
        if env_str!("ENOMOTO_MIP_RENS_DIVE").is_none() {
            return self.rens_old(_x);
        }
        let ints: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j] && self.dom.lo[j] < self.dom.up[j]).collect();
        if ints.is_empty() {
            return false;
        }
        // 目標の固定率
        let r0 = tunable!("ENOMOTO_T_MIP_RENS_RATE", 0.6, f64);
        let (mut low, mut high) = (r0, r0);
        if self.rens_infeas.1 > 0 {
            high = 0.9 * self.rens_infeas.0 / self.rens_infeas.1 as f64;
            low = low.min(high);
        }
        if self.rens_succ.1 > 0 {
            let r = self.rens_succ.0 / self.rens_succ.1 as f64;
            low = low.min(0.9 * r);
            high = high.max(1.1 * r);
        }
        let target = (low + (high - low) * self.rand()).clamp(0.05, 0.95);
        let saved = self.lp.save_state();
        let pos = self.dom.stack_len();
        let it_start = self.lp.total_iterations();
        let budget = 2 * self.avg_node_iters().max(1000) + 1000;
        let nint = ints.len() as f64;
        let rate = |s: &Self| ints.iter().filter(|&&j| s.dom.lo[j] == s.dom.up[j]).count() as f64 / nint;
        let mut backtracks = 0usize;
        // 段ごとの定義域の記録位置 (LP が実行不能になったら直前の段に戻る)
        let mut stage_pos: Vec<usize> = Vec::new();
        let mut order: Vec<usize> = ints.clone();
        for k in (1..order.len()).rev() {
            let r = (self.rand() * (k + 1) as f64) as usize;
            order.swap(k, r.min(k));
        }
        loop {
            if self.time_up() || self.lp.total_iterations() - it_start > budget {
                break;
            }
            let fr = rate(self);
            if fr >= target || backtracks >= 10 {
                break;
            }
            let x = self.lp.col_values();
            let stop_rate = (1.0 - (1.0 - fr) * 0.9).min(target);
            stage_pos.push(self.dom.stack_len());
            let mut branched = 0usize;
            let mut conflict = false;
            for &j in &order {
                if self.dom.lo[j] == self.dom.up[j] {
                    continue;
                }
                let v = x[j];
                if (v - v.round()).abs() > FEASTOL {
                    continue;
                }
                branched += 1;
                self.dom.tighten_lower(p, j, v.round());
                self.dom.tighten_upper(p, j, v.round());
                if !self.dom.propagate(p) {
                    conflict = true;
                    break;
                }
                if branched % 16 == 0 && rate(self) >= stop_rate {
                    break;
                }
            }
            if branched == 0 {
                // 整数値の列がない: 分数の列を、目的値の悪くなる向き (費用 0 なら近い側) に丸めて固定する
                // (丸め幅の小さい列から、丸め幅の合計が 0.5 に達するまで)
                let mut fr_cols: Vec<(f64, usize, f64)> = order
                    .iter()
                    .filter(|&&j| self.dom.lo[j] < self.dom.up[j])
                    .map(|&j| {
                        let v = x[j];
                        let fv = if p.cost[j] > 0.0 { v.ceil() } else if p.cost[j] < 0.0 { v.floor() } else { v.round() };
                        let fv = fv.clamp(self.dom.lo[j], self.dom.up[j]);
                        ((fv - v).abs(), j, fv)
                    })
                    .collect();
                fr_cols.sort_by(|a, b| a.0.total_cmp(&b.0));
                let mut change = 0.0;
                for (d, j, fv) in fr_cols {
                    branched += 1;
                    self.dom.tighten_lower(p, j, fv);
                    self.dom.tighten_upper(p, j, fv);
                    if !self.dom.propagate(p) {
                        conflict = true;
                        break;
                    }
                    if rate(self) >= target {
                        break;
                    }
                    change += d;
                    if change >= 0.5 {
                        break;
                    }
                }
            }
            if branched == 0 {
                break;
            }
            let mut ok = !conflict;
            if ok {
                self.sync_lp();
                let it0 = self.lp.total_iterations();
                let lim = budget.saturating_sub(self.lp.total_iterations() - it_start).max(500);
                let st = self.lp.solve(&self.limits(lim));
                self.heur_iters += self.lp.total_iterations() - it0;
                ok = st == LpStatus::Optimal;
            }
            if !ok {
                // 直前の段に戻る (この段の固定を捨てる)。戻った後は固定率をそこまでで打ち切る
                backtracks += 1;
                let sp = stage_pos.pop().unwrap();
                self.dom.backtrack_to(p, sp);
                self.sync_lp();
                break;
            }
        }
        let fr = rate(self);
        let (lo, up) = (self.dom.lo.clone(), self.dom.up.clone());
        self.dom.backtrack_to(p, pos);
        self.sync_lp();
        self.lp.restore_state(&saved);
        if self.params.verbose && self.nodes <= 1 {
            eprintln!("MIP:   RENS: fixing rate {fr:.2} (target {target:.2}, {backtracks} backtracks)");
        }
        if fr < 0.1 {
            return false;
        }
        let mut sub = p.clone();
        sub.col_lo = lo;
        sub.col_up = up;
        let r = self.solve_submip_problem(sub, 500, None);
        match r {
            Some((_, true)) => {
                self.rens_succ.0 += fr;
                self.rens_succ.1 += 1;
                true
            }
            Some((super::solver::MipStatus::Infeasible, false)) => {
                self.rens_infeas.0 += fr;
                self.rens_infeas.1 += 1;
                false
            }
            _ => false,
        }
    }

    /// 旧来の RENS (LP 解で整数値の列を全部固定する。既定)。
    fn rens_old(&mut self, x: &[f64]) -> bool {
        let p = self.p;
        let mut lo = self.dom.lo.clone();
        let mut up = self.dom.up.clone();
        let (mut nint, mut nfix) = (0usize, 0usize);
        for j in 0..p.n {
            if !p.is_int[j] {
                continue;
            }
            nint += 1;
            let v = x[j];
            if (v - v.round()).abs() <= FEASTOL {
                lo[j] = v.round();
                up[j] = v.round();
                nfix += 1;
            } else {
                lo[j] = lo[j].max(v.floor());
                up[j] = up[j].min(v.ceil());
            }
        }
        if nint == 0 || (nfix as f64) < 0.5 * nint as f64 {
            return false;
        }
        self.solve_submip(lo, up, 500)
    }

    /// RINS: 暫定解と LP 解で値が一致する整数列を固定したサブ MIP を解く。固定は伝播しながら積み、矛盾するものは飛ばす。
    pub(super) fn rins(&mut self, x: &[f64]) -> bool {
        let p = self.p;
        if self.params.submip {
            return false;
        }
        let Some((_, inc)) = self.incumbent.as_ref() else { return false };
        let inc = inc.clone();
        let mut fixes: Vec<(usize, f64, f64)> = Vec::new();
        let mut nint = 0usize;
        for j in 0..p.n {
            if !p.is_int[j] {
                continue;
            }
            nint += 1;
            if (x[j] - inc[j]).abs() <= FEASTOL && inc[j] >= self.dom.lo[j] && inc[j] <= self.dom.up[j] {
                fixes.push((j, inc[j], inc[j]));
            }
        }
        if nint == 0 || (fixes.len() as f64) < 0.5 * nint as f64 {
            return false;
        }
        fixes.sort_by_key(|&(_, v, _)| (v == 0.0) as u8);
        if env_str!("ENOMOTO_MIP_RINS_PROP").is_none() {
            // 既定は旧来どおり (伝播せずに固定する)
            let (mut lo, mut up) = (self.dom.lo.clone(), self.dom.up.clone());
            for &(j, l, u) in &fixes {
                lo[j] = l;
                up[j] = u;
            }
            return self.solve_submip(lo, up, 500);
        }
        let (lo, up, _) = self.propagated_fixings(&fixes);
        self.solve_submip(lo, up, 500)
    }

    /// 暫定解の 2 値列を何個変えたか (Hamming 距離) の行: `sum_{inc=0} x_j - sum_{inc=1} x_j` と、その定数
    /// (`|{inc=1}|`)。距離 = 行の値 + 定数。2 値列 (大域的な境界が [0, 1]) だけを数える。
    fn hamming_row(&self, inc: &[f64]) -> (Vec<(usize, f64)>, f64) {
        let p = self.p;
        let mut row = Vec::new();
        let mut ones = 0.0;
        for j in 0..p.n {
            if !p.is_int[j] || self.dom.global_lo[j] != 0.0 || self.dom.global_up[j] != 1.0 {
                continue;
            }
            if inc[j] > 0.5 {
                row.push((j, -1.0));
                ones += 1.0;
            } else {
                row.push((j, 1.0));
            }
        }
        (row, ones)
    }

    /// Local Branching (Fischetti & Lodi 2003): 暫定解から 2 値列を `k` 個以内しか変えない、という制約を足した
    /// サブ MIP を、暫定解より良い解だけを探して解く。`k` は結果で調整する (改善なしで解き切ったら広げ、時間切れで
    /// 解がなければ狭める)。
    pub(super) fn local_branching(&mut self) -> bool {
        if self.params.submip {
            return false;
        }
        let Some((_, inc)) = self.incumbent.as_ref() else { return false };
        let inc = inc.clone();
        let (row, ones) = self.hamming_row(&inc);
        if row.len() < 10 {
            return false;
        }
        let k = self.lb_k.clamp(2.0, row.len() as f64 / 2.0).round();
        let mut rows = self.p.rows.clone();
        let mut row_lo = self.p.row_lo.clone();
        let mut row_up = self.p.row_up.clone();
        rows.push(row);
        row_lo.push(f64::NEG_INFINITY);
        row_up.push(k - ones);
        let p = self.p;
        let sub = super::problem::MipProblem::from_rows(self.dom.global_lo.clone(), self.dom.global_up.clone(), p.cost.clone(), p.offset, p.sense_sign, p.is_int.clone(), rows, row_lo, row_up);
        let Some((st, acc)) = self.solve_submip_problem(sub, 1000, None) else { return false };
        if self.params.verbose {
            eprintln!("MIP:   local branching k={k}: {st:?}, improved {acc}");
        }
        match (st, acc) {
            (_, true) => {}
            (super::solver::MipStatus::Optimal | super::solver::MipStatus::Infeasible, false) => self.lb_k *= 1.5,
            _ => self.lb_k /= 1.5,
        }
        self.lb_k = self.lb_k.clamp(2.0, 1000.0);
        acc
    }

    /// Proximity Search (Fischetti & Monaci 2014): 目的関数を暫定解からの Hamming 距離に置き換え、元の目的値を
    /// 暫定解より `theta` 以上良くする制約を足したサブ MIP を解く (どの実行可能解も暫定解の改善)。
    pub(super) fn proximity_search(&mut self) -> bool {
        if self.params.submip {
            return false;
        }
        let Some((z, inc)) = self.incumbent.as_ref() else { return false };
        let (z, inc) = (*z, inc.clone());
        let (row, ones) = self.hamming_row(&inc);
        if row.len() < 10 {
            return false;
        }
        let p = self.p;
        // 元の目的値の改善の幅: 目的値が整数刻みならその刻み、そうでなければ相対 1e-4
        let theta = match self.obj_step {
            Some(step) => step,
            None => 1e-4 * z.abs().max(1.0),
        };
        let mut cost = vec![0.0; p.n];
        for &(j, c) in &row {
            cost[j] = c;
        }
        let mut rows = p.rows.clone();
        let mut row_lo = p.row_lo.clone();
        let mut row_up = p.row_up.clone();
        rows.push((0..p.n).filter(|&j| p.cost[j] != 0.0).map(|j| (j, p.cost[j])).collect());
        row_lo.push(f64::NEG_INFINITY);
        row_up.push(z - theta - p.offset);
        let sub = super::problem::MipProblem::from_rows(self.dom.global_lo.clone(), self.dom.global_up.clone(), cost, ones, p.sense_sign, p.is_int.clone(), rows, row_lo, row_up);
        let Some((st, acc)) = self.solve_submip_problem(sub, 500, Some(f64::INFINITY)) else { return false };
        if self.params.verbose {
            eprintln!("MIP:   proximity search: {st:?}, improved {acc}");
        }
        acc
    }

    /// Mutation (SCIP): 暫定解の値で整数列の一定割合 (乱数で選ぶ) を固定したサブ MIP を解く。
    pub(super) fn mutation(&mut self) -> bool {
        if self.params.submip {
            return false;
        }
        let Some((_, inc)) = self.incumbent.as_ref() else { return false };
        let inc = inc.clone();
        let p = self.p;
        let mut lo = self.dom.global_lo.clone();
        let mut up = self.dom.global_up.clone();
        let rate = tunable!("ENOMOTO_T_MIP_MUTATION_RATE", 0.7, f64);
        let mut nfix = 0usize;
        for j in 0..p.n {
            if p.is_int[j] && self.rand() < rate && inc[j] >= lo[j] && inc[j] <= up[j] {
                lo[j] = inc[j];
                up[j] = inc[j];
                nfix += 1;
            }
        }
        if nfix == 0 {
            return false;
        }
        let mut sub = p.clone();
        sub.col_lo = lo;
        sub.col_up = up;
        let r = self.solve_submip_problem(sub, 500, None);
        if self.params.verbose {
            eprintln!("MIP:   mutation ({nfix} fixed): {:?}", r);
        }
        r.is_some_and(|r| r.1)
    }

    /// 適応的な大近傍探索 (SCIP の ALNS の簡略版): RINS・Local Branching・Proximity Search・Mutation・Crossover・DINS から、
    /// 改善できた割合と試した回数で UCB1 により 1 つ選んで使う。`x` は今のノードの LP 解 (RINS 用)。
    pub(super) fn alns(&mut self, x: &[f64]) -> bool {
        if self.incumbent.is_none() {
            return false;
        }
        const ARMS: usize = 6;
        let total: f64 = self.alns_count.iter().sum::<u32>() as f64;
        let arm = match (0..ARMS).find(|&a| self.alns_count[a] == 0) {
            Some(a) => a,
            None => (0..ARMS)
                .max_by(|&a, &b| {
                    let ucb = |k: usize| self.alns_reward[k] / self.alns_count[k] as f64 + (2.0 * total.ln() / self.alns_count[k] as f64).sqrt() * 0.5;
                    ucb(a).total_cmp(&ucb(b))
                })
                .unwrap(),
        };
        let z0 = self.incumbent.as_ref().map(|(z, _)| *z).unwrap();
        let ok = match arm {
            0 => self.rins(x),
            1 => self.local_branching(),
            2 => self.proximity_search(),
            3 => self.mutation(),
            4 => self.crossover(),
            _ => self.dins(x),
        };
        // 報酬: 改善できたら 1 (改善の相対的な大きさで少し上乗せ)
        let z1 = self.incumbent.as_ref().map(|(z, _)| *z).unwrap();
        let reward = if ok && z1 < z0 { 1.0 + ((z0 - z1) / z0.abs().max(1.0)).min(1.0) } else { 0.0 };
        self.alns_count[arm] += 1;
        self.alns_reward[arm] += reward;
        if self.params.verbose {
            eprintln!("MIP:   ALNS arm {arm} reward {reward:.3} (counts {:?})", self.alns_count);
        }
        ok
    }

    /// 根の被約費用による固定 (HiGHS の `HighsPrimalHeuristics::rootReducedCost`): 根の LP で境界にいる整数列を、
    /// 被約費用の大きい順に (打ち切り値がそれ以下なら被約費用固定で固定できる列から) その境界に固定して伝播する
    /// (矛盾する固定は飛ばす)。整数列の固定率が 5 割に達したら止め、3 割に届かなければ何もしない。残った問題を
    /// サブ MIP として解く。LP は根の最適解の状態で呼ぶ。
    pub(super) fn root_reduced_cost(&mut self) -> bool {
        let p = self.p;
        if self.params.submip {
            return false;
        }
        let b = self.lp.basis();
        let d = self.lp.reduced_costs();
        let ints: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j] && self.dom.lo[j] < self.dom.up[j]).collect();
        if ints.is_empty() {
            return false;
        }
        // (|被約費用|, 列, 上限を締めるか (下限にいる列), 値)
        let mut lurking: Vec<(f64, usize, bool, f64)> = Vec::new();
        for &j in &ints {
            match b.col[j] {
                super::lp::VarStatus::Lower if d[j] > 1e-7 => lurking.push((d[j], j, true, self.dom.lo[j])),
                super::lp::VarStatus::Upper if d[j] < -1e-7 => lurking.push((-d[j], j, false, self.dom.up[j])),
                _ => {}
            }
        }
        // HiGHS は lurking bound が整数列の 1 割未満なら使わない
        if 10 * lurking.len() < ints.len() {
            return false;
        }
        lurking.sort_by(|a, b| b.0.total_cmp(&a.0));
        let pos = self.dom.stack_len();
        let nint = ints.len() as f64;
        let rate = |s: &Self| ints.iter().filter(|&&j| s.dom.lo[j] == s.dom.up[j]).count() as f64 / nint;
        let work0 = self.dom.debug_work();
        let work_cap = 20 * p.rows.iter().map(|r| r.len() as u64).sum::<u64>() + 1_000_000;
        let max_rate = tunable!("ENOMOTO_T_MIP_RRC_RATE", 0.5, f64);
        for (k, &(_, j, upper, v)) in lurking.iter().enumerate() {
            if k % 64 == 0 && (self.time_up() || self.dom.debug_work() - work0 > work_cap) {
                break;
            }
            let before = self.dom.stack_len();
            if upper {
                self.dom.tighten_upper(p, j, v);
            } else {
                self.dom.tighten_lower(p, j, v);
            }
            if !self.dom.propagate(p) {
                self.dom.backtrack_to(p, before);
                continue;
            }
            if k % 16 == 0 && rate(self) >= max_rate {
                break;
            }
        }
        let fr = rate(self);
        let (lo, up) = (self.dom.lo.clone(), self.dom.up.clone());
        self.dom.backtrack_to(p, pos);
        if self.params.verbose && self.nodes <= 1 {
            eprintln!("MIP:   root reduced cost: {} lurking bounds, fixing rate {fr:.2}", lurking.len());
        }
        if fr < tunable!("ENOMOTO_T_MIP_RRC_MIN_RATE", 0.3, f64) {
            return false;
        }
        // サブ MIP の時間の上限 (既定は通常と同じ残りの 7%。3% では 10teams のサブ MIP が解を見つける前に止まった)
        let keep = self.submip_time_frac;
        self.submip_time_frac = tunable!("ENOMOTO_T_MIP_RRC_TIME_FRAC", 0.07, f64);
        let r = self.solve_submip(lo, up, 500);
        self.submip_time_frac = keep;
        r
    }

    /// 自明な点 (HiGHS の trivial、SCIP の trivial・locks): 全部 0、全部下限、全部上限、lock の少ない側に寄せた点を
    /// 整数列の目標値にして固定・伝播し、連続部分は LP で解いて試す ([`Self::fix_and_propagate`])。
    pub(super) fn trivial(&mut self) -> bool {
        let p = self.p;
        let ints: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j]).collect();
        if ints.is_empty() {
            return false;
        }
        let clamp0 = |lo: f64, up: f64, v: f64| v.clamp(lo, up);
        for kind in 0..4 {
            if self.time_up() {
                break;
            }
            let target: Vec<f64> = (0..p.n)
                .map(|j| {
                    let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
                    let finite = |v: f64, alt: f64| if v.is_finite() { v } else { alt };
                    match kind {
                        0 => clamp0(lo, up, 0.0),
                        1 => finite(lo, clamp0(lo, up, 0.0)),
                        2 => finite(up, clamp0(lo, up, 0.0)),
                        _ => {
                            let (dl, ul) = self.locks[j];
                            if dl <= ul { finite(lo, clamp0(lo, up, 0.0)) } else { finite(up, clamp0(lo, up, 0.0)) }
                        }
                    }
                })
                .collect();
            if self.fix_and_propagate(&target, &ints) {
                if self.params.verbose && self.nodes <= 1 {
                    eprintln!("MIP:   trivial heuristic found a solution (kind {kind})");
                }
                return true;
            }
        }
        false
    }

    /// ZI round (Wallace 2010、HiGHS の ziRound の簡略版): LP 解の小数の整数列を、どの行も (今の点で) 破らない
    /// 範囲で動かせるなら、近い整数 (両方動かせれば目的値の良い方) へ動かす、を変化がなくなるまで繰り返す。
    /// 連続列は動かさない。全部整数になったら試す。
    pub(super) fn zi_round(&mut self, x: &[f64]) -> bool {
        let p = self.p;
        let mut x = x.to_vec();
        let mut act = vec![0.0f64; p.m];
        for (i, r) in p.rows.iter().enumerate() {
            act[i] = r.iter().map(|&(j, a)| a * x[j]).sum();
        }
        let tol = |b: f64| 1e-9 * (1.0 + b.abs());
        for _pass in 0..10 {
            let mut changed = false;
            let mut nfrac = 0usize;
            for j in 0..p.n {
                if !p.is_int[j] {
                    continue;
                }
                let v = x[j];
                let (fl, ce) = ((v + FEASTOL).floor(), (v - FEASTOL).ceil());
                if fl >= ce {
                    continue; // 整数
                }
                nfrac += 1;
                // 上へ・下へ動かせる最大の幅 (列の境界と各行の余裕)
                let (mut up_max, mut down_max) = (self.dom.up[j] - v, v - self.dom.lo[j]);
                for &(i, a) in &p.cols[j] {
                    let (rl, ru) = (p.row_lo[i], p.row_up[i]);
                    if a > 0.0 {
                        up_max = up_max.min((ru - act[i] + tol(ru)) / a);
                        down_max = down_max.min((act[i] - rl + tol(rl)) / a);
                    } else {
                        up_max = up_max.min((act[i] - rl + tol(rl)) / (-a));
                        down_max = down_max.min((ru - act[i] + tol(ru)) / (-a));
                    }
                }
                let can_up = up_max >= ce - v - 1e-12;
                let can_down = down_max >= v - fl - 1e-12;
                let nv = match (can_up, can_down) {
                    (true, true) => {
                        if p.cost[j] * (ce - v) <= p.cost[j] * (fl - v) { ce } else { fl }
                    }
                    (true, false) => ce,
                    (false, true) => fl,
                    _ => continue,
                };
                let dv = nv - v;
                for &(i, a) in &p.cols[j] {
                    act[i] += a * dv;
                }
                x[j] = nv;
                nfrac -= 1;
                changed = true;
            }
            if nfrac == 0 {
                return self.try_incumbent(x);
            }
            if !changed {
                break;
            }
        }
        false
    }
}
