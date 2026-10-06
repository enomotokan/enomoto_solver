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
    pub(super) fn propagate_rounding(&mut self, target: &[f64], order: &[usize], mut rounded: Option<&mut Vec<f64>>) -> bool {
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
        for &j in &all {
            if !p.is_int[j] {
                continue;
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
            let mut target = x.to_vec();
            for &j in &order {
                let th = if t == 0 { 0.5 } else { 0.1 + 0.8 * self.rand() };
                target[j] = (x[j] + 1.0 - th).floor();
            }
            if self.fix_and_propagate(&target, &order) {
                return true;
            }
            if self.time_up() {
                break;
            }
        }
        false
    }

    /// 分数ダイビング: LP 解で整数に最も近い分数の列をその側に丸めて固定し、伝播して LP を解き直す、を
    /// 整数解になるまで繰り返す。固定で実行不能になったら 1 回だけ反対側を試す。`budget` は LP 反復の上限。
    /// 定義域と LP は呼ぶ前の状態に戻す。
    pub(super) fn fractional_dive(&mut self, budget: u64) -> bool {
        let p = self.p;
        let saved = self.lp.save_state();
        let pos = self.dom.stack_len();
        let it_start = self.lp.total_iterations();
        let mut found = false;
        for _depth in 0..(2 * p.n) {
            if self.time_up() || self.lp.total_iterations() - it_start > budget {
                if self.params.verbose {
                    eprintln!("MIP:   dive ran out of budget at depth {_depth} ({} iterations)", self.lp.total_iterations() - it_start);
                }
                break;
            }
            let x = self.lp.col_values();
            let frac = self.fractional(&x);
            if frac.is_empty() {
                found = self.try_lp_solution();
                if self.params.verbose {
                    eprintln!("MIP:   dive reached an integral LP at depth {_depth} (accepted {found}, objective {})", self.lp.objective() + p.offset);
                }
                break;
            }
            // 整数に最も近い列 (同点は lock の少ない側に丸めやすい列)
            let &(j, v) = frac
                .iter()
                .min_by(|a, b| {
                    let fa = (a.1 - a.1.round()).abs();
                    let fb = (b.1 - b.1.round()).abs();
                    fa.total_cmp(&fb)
                })
                .unwrap();
            let up_first = v - v.floor() >= 0.5;
            let mut ok = false;
            for up in [up_first, !up_first] {
                let before = self.dom.stack_len();
                if up {
                    self.dom.tighten_lower(p, j, v.ceil());
                } else {
                    self.dom.tighten_upper(p, j, v.floor());
                }
                if self.dom.propagate(p) {
                    self.sync_lp();
                    let it0 = self.lp.total_iterations();
                    let lim = budget.saturating_sub(self.lp.total_iterations() - it_start).max(1000);
                    let st = self.lp.solve(&self.limits(lim));
                    self.heur_iters += self.lp.total_iterations() - it0;
                    if st == LpStatus::Optimal {
                        ok = true;
                        break;
                    }
                    if self.params.verbose {
                        eprintln!("MIP:   dive LP {st:?} after {} iterations (col {j}, up {up})", self.lp.total_iterations() - it0);
                    }
                }
                self.dom.backtrack_to(p, before);
                self.sync_lp();
            }
            if !ok {
                if self.params.verbose {
                    eprintln!("MIP:   dive stopped at depth {_depth} with {} fractional", frac.len());
                }
                break;
            }
        }
        self.dom.backtrack_to(p, pos);
        self.sync_lp();
        self.lp.restore_state(&saved);
        found
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
        let p = self.p;
        let (n, m) = (p.n, p.m);
        if n == 0 {
            return false;
        }
        let t_end = Instant::now() + Duration::from_secs_f64(time_cap);
        let lo = self.dom.lo.clone();
        let up = self.dom.up.clone();
        // 初期点: 0 に最も近い境界内の値
        let mut x: Vec<f64> = (0..n)
            .map(|j| {
                let v = 0.0f64.clamp(lo[j], up[j]);
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
            if step % 256 == 0 && Instant::now() >= t_end {
                break;
            }
            if vset.is_empty() {
                return self.try_incumbent(x);
            }
            let i = vset[(self.rand() * vset.len() as f64) as usize % vset.len()];
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
                    let v0 = cand[k].clamp(lo[j], up[j]);
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
        false
    }
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 一部の整数列を固定した (境界を締めた) サブ MIP を、ノード数を制限して解く。
    /// 見つかった解は暫定解の候補にする。`lo`/`up` はサブ MIP の列の境界。
    fn solve_submip(&mut self, lo: Vec<f64>, up: Vec<f64>, node_limit: u64) -> bool {
        let p = self.p;
        let mut sub = p.clone();
        sub.col_lo = lo;
        sub.col_up = up;
        let remaining = match self.deadline {
            Some(d) => d.saturating_duration_since(Instant::now()).as_secs_f64(),
            None => f64::INFINITY,
        };
        let params = super::solver::MipParams {
            time_limit: (0.1 * remaining).min(10.0),
            node_limit,
            rel_gap: self.params.rel_gap,
            abs_gap: self.params.abs_gap,
            verbose: false,
            submip: true,
            cutoff: self.prune_limit(),
        };
        let r = super::solve_problem(&sub, params, env_str!("ENOMOTO_MIP_SUBMIP_NO_PRESOLVE").is_none());
        if self.params.verbose {
            let nfree = (0..p.n).filter(|&j| sub.col_lo[j] < sub.col_up[j]).count();
            eprintln!("MIP:   sub-MIP: {nfree} free columns of {}, status {:?}, nodes {}, objective {:?}", p.n, r.status, r.nodes, r.objective);
        }
        self.heur_iters += r.lp_iterations;
        match r.x {
            Some(x) => self.try_incumbent(x),
            None => false,
        }
    }

    /// RENS: LP 解で整数値の整数列をその値に固定し、他の整数列を LP 値の前後の整数に制限したサブ MIP を解く。
    pub(super) fn rens(&mut self, x: &[f64]) -> bool {
        let p = self.p;
        if self.params.submip {
            return false;
        }
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

    /// RINS: 暫定解と LP 解で値が一致する整数列を固定したサブ MIP を解く。
    pub(super) fn rins(&mut self, x: &[f64]) -> bool {
        let p = self.p;
        if self.params.submip {
            return false;
        }
        let Some((_, inc)) = self.incumbent.as_ref() else { return false };
        let inc = inc.clone();
        let mut lo = self.dom.lo.clone();
        let mut up = self.dom.up.clone();
        let (mut nint, mut nfix) = (0usize, 0usize);
        for j in 0..p.n {
            if !p.is_int[j] {
                continue;
            }
            nint += 1;
            if (x[j] - inc[j]).abs() <= FEASTOL && inc[j] >= self.dom.lo[j] && inc[j] <= self.dom.up[j] {
                lo[j] = inc[j];
                up[j] = inc[j];
                nfix += 1;
            }
        }
        if nint == 0 || (nfix as f64) < 0.5 * nint as f64 {
            return false;
        }
        self.solve_submip(lo, up, 500)
    }
}
