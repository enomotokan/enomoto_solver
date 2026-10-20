//! Feasibility Jump (Luteberget & Sartor 2023) の移植。HiGHS が使う実装 (`extern/FeasibilityJump/FeasibilityJump.hpp`、
//! MIT ライセンス) と同じ手順にしてある:
//!
//! - 各変数について「跳び先の値」(その変数だけを動かしたときに、重み付きの違反の和が最小になる値) と、そこへ動かしたときの
//!   スコア (重み付きの違反の減り + 目的の重み × 目的値の減り) を持ち、変数を動かすたびに関わる行の変数のスコアを増分で直す。
//! - スコアが正の変数 (良い変数) の集合から 25 個を乱数で選び、スコアの最大のものを動かす。良い変数がなければ (局所最適)
//!   違反している行の重みを 1 上げ (違反がなければ目的の重みを上げ)、違反している行を 1 本選んでその中で最良の変数を動かす。
//! - 違反がなくなるたびに、目的値が相対 1e-4 以上良くなっていれば解として記録し、続ける。
//! - 手間 (触った非零の数) の合計が非零の数 × 1024、最後の改善からの手間が非零の数 × 256 を超えたら止める。
//!
//! 等式の行は `>=` と `<=` の 2 本にする (HiGHS と同じ)。

use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq)]
enum Sense {
    Lte,
    Gte,
}

struct Cons {
    sense: Sense,
    rhs: f64,
    coeffs: Vec<(u32, f64)>,
    weight: f64,
    lhs: f64,
    viol_idx: i32,
}

impl Cons {
    /// 行の違反の負 (満たしていれば 0)。
    #[inline]
    fn score(&self, lhs: f64) -> f64 {
        match self.sense {
            Sense::Lte => -(lhs - self.rhs).max(0.0),
            Sense::Gte => -(self.rhs - lhs).max(0.0),
        }
    }
}

#[derive(Clone, Copy)]
struct Move {
    value: f64,
    score: f64,
}

struct Fj {
    lb: Vec<f64>,
    ub: Vec<f64>,
    is_int: Vec<bool>,
    obj: Vec<f64>,
    vcols: Vec<Vec<(u32, f64)>>,
    cons: Vec<Cons>,
    x: Vec<f64>,
    objective: f64,
    violated: Vec<u32>,
    moves: Vec<Move>,
    good: Vec<u32>,
    good_idx: Vec<i32>,
    obj_weight: f64,
    weight_inc: f64,
    effort: u64,
    eq_tol: f64,
    viol_tol: f64,
    rng: u64,
    shift_buf: Vec<(f64, f64)>,
}

impl Fj {
    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    #[inline]
    fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64)
    }

    fn reset_incumbent(&mut self) {
        self.objective = (0..self.x.len()).map(|j| self.obj[j] * self.x[j]).sum();
        self.violated.clear();
        for (ci, c) in self.cons.iter_mut().enumerate() {
            c.lhs = c.coeffs.iter().map(|&(j, a)| a * self.x[j as usize]).sum();
            if c.score(c.lhs) < -self.viol_tol {
                c.viol_idx = self.violated.len() as i32;
                self.violated.push(ci as u32);
            } else {
                c.viol_idx = -1;
            }
        }
    }

    /// 変数 `v` の跳び先の値を計算し直す (HiGHS の `JumpMove::updateValue`)。
    fn update_value(&mut self, v: usize) {
        self.shift_buf.clear();
        let xi = self.x[v];
        let (lower, upper) = (self.lb[v], self.ub[v]);
        let mut cur_value = lower;
        let mut slope = 0.0;
        for &(ci, a) in &self.vcols[v] {
            let c = &self.cons[ci as usize];
            let (blo, bup) = match c.sense {
                Sense::Lte => (f64::NEG_INFINITY, c.rhs),
                Sense::Gte => (c.rhs, f64::INFINITY),
            };
            let resid = c.lhs - a * xi;
            let mut r = ((1.0 / a) * (blo - resid), (1.0 / a) * (bup - resid));
            // 負の係数で割ると区間の向きが逆になる (参照実装 sintef/feasibilityjump と同じ。HiGHS 1.15.1 の写しにはこの
            // 入れ替えがなく、負の係数の行は空の区間として飛ばされる)
            if a < 0.0 {
                r = (r.1, r.0);
            }
            if self.is_int[v] {
                r = ((r.0 - self.eq_tol).ceil(), (r.1 + self.eq_tol).floor());
            }
            if r.0 > r.1 {
                continue;
            }
            if r.0 > cur_value {
                slope -= c.weight;
                if r.0 < upper {
                    self.shift_buf.push((r.0, c.weight));
                }
            }
            if r.1 <= cur_value {
                slope += c.weight;
            } else if r.1 < upper {
                self.shift_buf.push((r.1, c.weight));
            }
        }
        if lower.is_finite() {
            self.shift_buf.push((lower, 0.0));
        }
        if upper.is_finite() {
            self.shift_buf.push((upper, 0.0));
        }
        self.shift_buf.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let mut score = 0.0;
        cur_value = if self.shift_buf.is_empty() { xi } else { self.shift_buf[0].0 };
        let mut best_score = score;
        let mut best_value = cur_value;
        let eq = |a: f64, b: f64, t: f64| (a - b).abs() < t;
        for &(val, w) in &self.shift_buf {
            score += (val - cur_value) * slope;
            slope += w;
            cur_value = val;
            if eq(best_value, xi, self.eq_tol) || (!eq(cur_value, xi, self.eq_tol) && score < best_score) {
                best_score = score;
                best_value = cur_value;
            }
            if !eq(best_value, xi, self.eq_tol) && slope >= 0.0 {
                break;
            }
        }
        self.moves[v].value = best_value;
    }

    fn update_good(&mut self, v: usize) {
        let good = self.moves[v].score > 0.0;
        if good && self.good_idx[v] == -1 {
            self.good_idx[v] = self.good.len() as i32;
            self.good.push(v as u32);
        } else if !good && self.good_idx[v] != -1 {
            let k = self.good_idx[v] as usize;
            let last = *self.good.last().unwrap();
            self.good.swap_remove(k);
            if last as usize != v {
                self.good_idx[last as usize] = k as i32;
            }
            self.good_idx[v] = -1;
        }
    }

    fn reset_moves(&mut self, v: usize) {
        self.effort += self.vcols[v].len() as u64;
        self.update_value(v);
        let mv = self.moves[v].value;
        let d = mv - self.x[v];
        let mut s = self.obj_weight * self.obj[v] * d;
        for &(ci, a) in &self.vcols[v] {
            let c = &self.cons[ci as usize];
            s += c.weight * (c.score(c.lhs + a * d) - c.score(c.lhs));
        }
        self.moves[v].score = s;
        self.update_good(v);
    }

    fn update_weights(&mut self) {
        let mut rescale = false;
        let mut dt = 0u64;
        let inc = self.weight_inc;
        if self.violated.is_empty() {
            self.obj_weight += inc;
            if self.obj_weight > 1e20 {
                rescale = true;
            }
            dt += self.x.len() as u64;
            for v in 0..self.x.len() {
                self.moves[v].score += inc * self.obj[v] * (self.moves[v].value - self.x[v]);
            }
        } else {
            let viol = self.violated.clone();
            for ci in viol {
                let ci = ci as usize;
                self.cons[ci].weight += inc;
                if self.cons[ci].weight > 1e20 {
                    rescale = true;
                }
                dt += self.cons[ci].coeffs.len() as u64;
                for k in 0..self.cons[ci].coeffs.len() {
                    let (j, a) = self.cons[ci].coeffs[k];
                    let j = j as usize;
                    let c = &self.cons[ci];
                    let cand = c.lhs + a * (self.moves[j].value - self.x[j]);
                    self.moves[j].score += inc * (c.score(cand) - c.score(c.lhs));
                    self.update_good(j);
                }
            }
        }
        if rescale {
            self.weight_inc *= 1e-20;
            self.obj_weight *= 1e-20;
            for c in self.cons.iter_mut() {
                c.weight *= 1e-20;
            }
            dt += self.cons.len() as u64;
            for v in 0..self.x.len() {
                self.reset_moves(v);
            }
        }
        self.effort += dt;
    }

    fn select_variable(&mut self) -> usize {
        if !self.good.is_empty() {
            if self.uniform() < 0.001 {
                let k = (self.next_u64() % self.good.len() as u64) as usize;
                return self.good[k] as usize;
            }
            let sample = self.good.len().min(25);
            self.effort += sample as u64;
            let mut best = (f64::NEG_INFINITY, usize::MAX);
            for _ in 0..sample {
                let k = (self.next_u64() % self.good.len() as u64) as usize;
                let v = self.good[k] as usize;
                if self.moves[v].score > best.0 {
                    best = (self.moves[v].score, v);
                }
            }
            return best.1;
        }
        self.update_weights();
        if !self.violated.is_empty() {
            let k = (self.next_u64() % self.violated.len() as u64) as usize;
            let ci = self.violated[k] as usize;
            if self.uniform() < 0.01 {
                let k = (self.next_u64() % self.cons[ci].coeffs.len() as u64) as usize;
                return self.cons[ci].coeffs[k].0 as usize;
            }
            let mut best = (f64::NEG_INFINITY, usize::MAX);
            for &(j, _) in &self.cons[ci].coeffs {
                let s = self.moves[j as usize].score;
                if s > best.0 {
                    best = (s, j as usize);
                }
            }
            return best.1;
        }
        (self.next_u64() % self.x.len() as u64) as usize
    }

    fn do_move(&mut self, v: usize) {
        let new_value = self.moves[v].value;
        let old = self.x[v];
        let delta = new_value - old;
        self.x[v] = new_value;
        self.objective += self.obj[v] * delta;
        for k in 0..self.vcols[v].len() {
            let (ci, a) = self.vcols[v][k];
            let ci = ci as usize;
            let old_lhs = self.cons[ci].lhs;
            let new_lhs = old_lhs + a * delta;
            self.cons[ci].lhs = new_lhs;
            let new_cost = self.cons[ci].score(new_lhs);
            if new_cost < -self.viol_tol && self.cons[ci].viol_idx == -1 {
                self.cons[ci].viol_idx = self.violated.len() as i32;
                self.violated.push(ci as u32);
            }
            if new_cost >= -self.viol_tol && self.cons[ci].viol_idx != -1 {
                let k2 = self.cons[ci].viol_idx as usize;
                let last = *self.violated.last().unwrap() as usize;
                self.violated.swap_remove(k2);
                if last != ci {
                    self.cons[last].viol_idx = k2 as i32;
                }
                self.cons[ci].viol_idx = -1;
            }
            self.effort += self.cons[ci].coeffs.len() as u64;
            for kk in 0..self.cons[ci].coeffs.len() {
                let (j, b) = self.cons[ci].coeffs[kk];
                let j = j as usize;
                if j == v {
                    continue;
                }
                // 行 ci の左辺の変化を、変数 j の手のスコアに反映する (HiGHS の `modifyMove`)
                let c = &self.cons[ci];
                let mv = self.moves[j].value - self.x[j];
                let old_term = c.weight * (c.score(old_lhs + b * mv) - c.score(old_lhs));
                let new_term = c.weight * (c.score(new_lhs + b * mv) - c.score(new_lhs));
                self.moves[j].score += new_term - old_term;
                self.update_good(j);
            }
        }
        self.reset_moves(v);
    }
}

/// Feasibility Jump で実行可能な点を探す。`lo`/`up` は列の境界、`time_cap` は秒、`stop` が立ったら止める。
/// 見つけた実行可能な点のうち目的値の最良のものを返す。
pub(super) fn fj2_search(p: &super::problem::MipProblem, lo: &[f64], up: &[f64], time_cap: f64, seed: u64, stop: Option<&std::sync::atomic::AtomicBool>) -> Option<Vec<f64>> {
    let n = p.n;
    if n == 0 {
        return None;
    }
    let t_end = Instant::now() + Duration::from_secs_f64(time_cap.max(0.0));
    let feastol = 1e-6;
    let mut lb = vec![0.0; n];
    let mut ub = vec![0.0; n];
    for j in 0..n {
        let (mut l, mut u) = (lo[j], up[j]);
        if p.is_int[j] {
            l = (l - feastol).ceil();
            u = (u + feastol).floor();
        }
        if !(l <= u) || l == f64::INFINITY || u == f64::NEG_INFINITY {
            return None;
        }
        lb[j] = l;
        ub[j] = u;
    }
    let x: Vec<f64> = (0..n).map(|j| if lb[j].is_finite() { lb[j] } else if ub[j].is_finite() { ub[j] } else { 0.0 }).collect();
    let mut cons: Vec<Cons> = Vec::new();
    let mut vcols: Vec<Vec<(u32, f64)>> = vec![Vec::new(); n];
    let mut nnz = 0u64;
    for i in 0..p.m {
        let row: Vec<(u32, f64)> = p.rows[i].iter().filter(|&&(_, a)| a != 0.0).map(|&(j, a)| (j as u32, a)).collect();
        for (sense, rhs) in [(Sense::Gte, p.row_lo[i]), (Sense::Lte, p.row_up[i])] {
            if !rhs.is_finite() {
                continue;
            }
            if row.is_empty() {
                let ok = match sense {
                    Sense::Lte => 0.0 <= rhs + 1e-9,
                    Sense::Gte => 1e-9 >= rhs,
                };
                if !ok {
                    return None;
                }
                continue;
            }
            let ci = cons.len() as u32;
            for &(j, a) in &row {
                vcols[j as usize].push((ci, a));
            }
            nnz += row.len() as u64;
            cons.push(Cons { sense, rhs, coeffs: row.clone(), weight: 1.0, lhs: 0.0, viol_idx: -1 });
        }
    }
    let mut s = Fj {
        lb,
        ub,
        is_int: p.is_int.clone(),
        obj: p.cost.clone(),
        vcols,
        cons,
        x,
        objective: 0.0,
        violated: Vec::new(),
        moves: vec![Move { value: 0.0, score: 0.0 }; n],
        good: Vec::new(),
        good_idx: vec![-1; n],
        obj_weight: 0.0,
        weight_inc: 1.0,
        effort: 0,
        eq_tol: 1e-9,
        viol_tol: feastol,
        rng: seed | 1,
        shift_buf: Vec::new(),
    };
    s.reset_incumbent();
    s.effort += nnz;
    for v in 0..n {
        s.reset_moves(v);
    }
    // 手間の上限 (非零の数 × 1024、最後の改善から × 256)。`ENOMOTO_T_MIP_FJ_EFFORT_MULT` 倍にする (診断用、既定 1)
    let mult = tunable!("ENOMOTO_T_MIP_FJ_EFFORT_MULT", 1u64, u64).max(1);
    let max_total = (nnz.max(1) << 10) * mult;
    let max_since = (nnz.max(1) << 8) * mult;
    let mut best_viol = usize::MAX;
    let mut best_obj = f64::INFINITY;
    let mut best_x: Option<Vec<f64>> = None;
    let mut effort_last_improve = 0u64;
    let mut effort_last_check = 0u64;
    let t0 = Instant::now();
    let mut why = "select";
    let mut steps = 0u64;
    loop {
        steps += 1;
        if s.violated.len() < best_viol {
            effort_last_improve = s.effort;
            best_viol = s.violated.len();
        }
        let mut check = s.effort - effort_last_check > 500_000;
        if s.violated.is_empty() && s.objective < best_obj {
            let rel = (best_obj - s.objective) / s.objective.abs().max(1.0);
            if rel > 1e-4 {
                effort_last_improve = s.effort;
                best_obj = s.objective;
                best_x = Some(s.x.clone());
                check = true;
            }
        }
        if check {
            effort_last_check = s.effort;
            if s.effort - effort_last_improve > max_since || s.effort > max_total {
                why = "effort";
                break;
            }
            if Instant::now() >= t_end || stop.is_some_and(|f| f.load(std::sync::atomic::Ordering::Relaxed)) {
                why = "time/stop";
                break;
            }
        }
        let v = s.select_variable();
        if v == usize::MAX {
            break;
        }
        s.do_move(v);
    }
    if env_str!("ENOMOTO_MIP_FJ_DEBUG").is_some_and(|v| v == "2") && best_x.is_none() {
        // 終わったときに違反している行の形 (向き・長さ・連続列の数・右辺) を数える
        let mut kinds: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        for &ci in &s.violated {
            let c = &s.cons[ci as usize];
            let nc = c.coeffs.iter().filter(|&&(j, _)| !s.is_int[j as usize]).count();
            let k = format!("{} len {} cont {} rhs {:.4}", if c.sense == Sense::Lte { "<=" } else { ">=" }, c.coeffs.len(), nc, c.rhs);
            *kinds.entry(k).or_insert(0) += 1;
        }
        eprintln!("MIP:   FJ2 violated rows at the end: {kinds:?}");
    }
    if env_str!("ENOMOTO_MIP_FJ_DEBUG").is_some() {
        eprintln!(
            "MIP:   FJ2: {} rows (as inequalities), {} nonzeros, {steps} steps, effort {} (max {max_total}), least violated rows {best_viol}, found {} (objective {best_obj:.6e}), stopped by {why}, {:.2}s",
            s.cons.len(),
            nnz,
            s.effort,
            best_x.is_some(),
            t0.elapsed().as_secs_f64()
        );
    }
    best_x
}
