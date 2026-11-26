//! 試作 `ENOMOTO_MIP_ALNS` (既定では使わない。analysis/alns_20261126/README.md): OR-Tools CP-SAT の「難しさを自動で合わせる大近傍探索」
//! (ortools/sat/cp_model_lns.{h,cc}、ortools/util/adaptative_parameter_value.h) を、木の大近傍探索 ([`Solver::alns`]) の中で使う。
//!
//! - 近傍 3 本: `rins` (LP 解と暫定解の違う整数列を先に外す)、`random` (乱数)、`graph` (制約グラフで近い整数列、目的に効く列から幅優先)。
//!   どれも外す整数列の数 = ⌈d × 動ける整数列の数⌉ で、外さない整数列は暫定解の値に固定する (連続列は元の境界のまま)。
//! - 難しさ d (近傍ごと): CP-SAT の `AdaptiveParameterValue` の移植。初め 0.5、サブ MIP を解き切った (最適・実行不能) ら Increase、
//!   時間・ノードの上限で止まったら Decrease。係数 f = 1 + 1/√(変更回数 + 1) の乗算で、新しい閾値はない。上げ下げが d と 1 − d で
//!   対称なので、成功と失敗が釣り合う所 (成功率 50%: 二値の結果が最も不確かで、1 回の試行から得る情報が最も多い所) に落ち着く。
//! - 選び方: CP-SAT の `GetUCBScore` と同じ UCB1 (10 回までは無限大、以後 平均利得 + √(2 ln N / n))。利得 = 相対改善 /
//!   (1 + 決定的な時間 (tick 秒))。100 回までは単純平均、以後は 0.9 / 0.1 の指数平均。
//! - 呼ぶ時期・予算 (`ENOMOTO_T_MIP_LNS_FREQ`・`LNS_TIME_*`・HEUR_ADAPT など) と、4 スレッドのヒューリスティクス役 (tid 3 の
//!   `heur_role_run`) は今の木の大近傍探索のものをそのまま使う (`ENOMOTO_MIP_ALNS=1` の間は今の 6 本の腕の代わりにこの 3 本を使う)。

use super::lp_api::MipLp;
use super::solver::{MipStatus, Solver};

pub(crate) fn on() -> bool {
    env_str!("ENOMOTO_MIP_ALNS").is_some_and(|v| v != "0")
}

/// CP-SAT の `AdaptiveParameterValue`
#[derive(Clone, Copy)]
pub(crate) struct Adaptive {
    pub value: f64,
    changes: u64,
}

impl Default for Adaptive {
    fn default() -> Self {
        Adaptive { value: 0.5, changes: 0 }
    }
}

impl Adaptive {
    fn factor(&mut self) -> f64 {
        self.changes += 1;
        1.0 + 1.0 / ((self.changes + 1) as f64).sqrt()
    }
    pub fn increase(&mut self) {
        let f = self.factor();
        self.value = (1.0 - (1.0 - self.value) / f).min(self.value * f);
    }
    pub fn decrease(&mut self) {
        let f = self.factor();
        self.value = (self.value / f).max(1.0 - (1.0 - self.value) * f);
    }
}

pub(crate) const NAMES: [&str; 3] = ["rins", "random", "graph"];

/// 近傍ごとの状態 (スレッドごとの `Solver` に持つ)
#[derive(Default, Clone)]
pub(crate) struct AlnsCp {
    pub d: [Adaptive; 3],
    pub calls: [u64; 3],
    pub avg: [f64; 3],
    pub solved: [u64; 3],
    pub improved: [u64; 3],
}

impl AlnsCp {
    /// CP-SAT の `GetUCBScore`
    pub fn ucb(&self, k: usize, total: u64) -> f64 {
        if self.calls[k] <= 10 {
            return f64::INFINITY;
        }
        self.avg[k] + (2.0 * (total.max(1) as f64).ln() / self.calls[k] as f64).sqrt()
    }
    pub fn pick(&self) -> usize {
        let total: u64 = self.calls.iter().sum();
        (0..3).max_by(|&a, &b| self.ucb(a, total).total_cmp(&self.ucb(b, total)).then(b.cmp(&a))).unwrap()
    }
    pub fn add_gain(&mut self, k: usize, gain: f64) {
        self.calls[k] += 1;
        if self.calls[k] <= 100 {
            self.avg[k] += (gain - self.avg[k]) / self.calls[k] as f64;
        } else {
            self.avg[k] = 0.9 * self.avg[k] + 0.1 * gain;
        }
    }
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 乱数の並べ替え (Fisher–Yates、[`Solver::rand`] を使うので種で決まる)
    fn alns_cp_shuffle(&mut self, v: &mut [usize]) {
        for i in (1..v.len()).rev() {
            let j = ((self.rand() * (i + 1) as f64) as usize).min(i);
            v.swap(i, j);
        }
    }

    /// 近傍 `k` で外す整数列 (大域的に動ける整数列 `free` の中から `target` 個)
    fn alns_cp_relax(&mut self, k: usize, free: &[usize], target: usize, x: &[f64], inc: &[f64]) -> Vec<usize> {
        let p = self.p;
        match k {
            0 => {
                // rins: LP 解と暫定解の違う列を先に (どちらも乱数の順)
                let (mut diff, mut same): (Vec<usize>, Vec<usize>) = free.iter().partition(|&&j| (x[j] - inc[j]).abs() > 1e-6);
                self.alns_cp_shuffle(&mut diff);
                self.alns_cp_shuffle(&mut same);
                diff.extend(same);
                diff.truncate(target);
                diff
            }
            1 => {
                let mut v = free.to_vec();
                self.alns_cp_shuffle(&mut v);
                v.truncate(target);
                v
            }
            _ => {
                // graph: 目的の係数のある動ける整数列から乱数で始め、同じ行に出る列を幅優先 (行は 1 度だけ読む、各段で混ぜる)
                let mut is_free = vec![false; p.n];
                for &j in free {
                    is_free[j] = true;
                }
                let objv: Vec<usize> = free.iter().copied().filter(|&j| p.cost[j] != 0.0).collect();
                let pool = if objv.is_empty() { free } else { &objv[..] };
                let first = pool[((self.rand() * pool.len() as f64) as usize).min(pool.len() - 1)];
                let mut visited = vec![false; p.n];
                let mut scanned = vec![false; p.m];
                let mut order = vec![first];
                visited[first] = true;
                let mut out = vec![first];
                let mut i = 0;
                while i < order.len() && out.len() < target {
                    let mut next: Vec<usize> = Vec::new();
                    for &(r, _) in &p.cols[order[i]] {
                        if scanned[r] {
                            continue;
                        }
                        scanned[r] = true;
                        for &(j, _) in &p.rows[r] {
                            if !visited[j] {
                                visited[j] = true;
                                next.push(j);
                            }
                        }
                    }
                    self.alns_cp_shuffle(&mut next);
                    for j in next {
                        order.push(j);
                        if is_free[j] && out.len() < target {
                            out.push(j);
                        }
                    }
                    i += 1;
                }
                // 連結でつながらない分は乱数で足す
                if out.len() < target {
                    let mut rest: Vec<usize> = free.iter().copied().filter(|&j| !out.contains(&j)).collect();
                    self.alns_cp_shuffle(&mut rest);
                    rest.truncate(target - out.len());
                    out.extend(rest);
                }
                out
            }
        }
    }

    /// 近傍 `k` の 1 回 ([`Solver::alns`] から)。改善したら真
    pub(super) fn alns_cp_run(&mut self, k: usize, x: &[f64]) -> bool {
        let Some((z0, inc)) = self.incumbent.clone() else { return false };
        let p = self.p;
        let (glo, gup) = (self.dom.global_lo.clone(), self.dom.global_up.clone());
        let free: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j] && glo[j] < gup[j]).collect();
        if free.is_empty() {
            return false;
        }
        let d = self.alns_cp.d[k].value;
        let target = ((d * free.len() as f64).ceil() as usize).clamp(1, free.len());
        let relax = self.alns_cp_relax(k, &free, target, x, &inc);
        let mut keep = vec![false; p.n];
        for &j in &relax {
            keep[j] = true;
        }
        let (mut lo, mut up) = (glo, gup);
        for &j in &free {
            if !keep[j] {
                let v = inc[j].round().clamp(lo[j], up[j]);
                lo[j] = v;
                up[j] = v;
            }
        }
        let mut sub = p.clone();
        sub.col_lo = lo;
        sub.col_up = up;
        let t0 = crate::ticks::total();
        let w0 = std::time::Instant::now();
        let r = self.solve_submip_problem(sub, 500, None);
        let dt = (crate::ticks::total() - t0) / 1e9;
        let z1 = self.incumbent.as_ref().map_or(z0, |(z, _)| *z);
        let Some((st, acc)) = r else { return false };
        let solved = matches!(st, MipStatus::Optimal | MipStatus::Infeasible);
        let s = &mut self.alns_cp;
        if solved {
            s.d[k].increase();
            s.solved[k] += 1;
        } else {
            s.d[k].decrease();
        }
        let gain = ((z0 - z1) / z0.abs().max(1.0)).max(0.0) / (1.0 + dt);
        if z1 < z0 {
            s.improved[k] += 1;
        }
        s.add_gain(k, gain);
        if self.params.verbose {
            let s = &self.alns_cp;
            eprintln!(
                "MIP:   ALNS-CP {} d {d:.3} -> {:.3} relaxed {}/{} {st:?} improved {acc} z {z0:.6e} -> {z1:.6e} dtick {dt:.3} ({:.2}s) calls {:?} solved {:?} improved {:?}",
                NAMES[k],
                s.d[k].value,
                relax.len(),
                free.len(),
                w0.elapsed().as_secs_f64(),
                s.calls,
                s.solved,
                s.improved
            );
        }
        acc && z1 < z0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_moves_like_cpsat() {
        let mut a = Adaptive::default();
        a.increase();
        // f = 1 + 1/√2、d = min(1 - 0.5/f, 0.5 f)
        let f = 1.0 + 1.0 / 2f64.sqrt();
        assert!((a.value - (1.0 - 0.5 / f).min(0.5 * f)).abs() < 1e-12);
        let mut b = Adaptive::default();
        b.decrease();
        assert!((b.value - (0.5 / f).max(1.0 - 0.5 * f)).abs() < 1e-12);
        // 上げ下げは d と 1 − d で対称 (成功と失敗を入れ替えた列を 1 − d から始めると、いつも 1 − d の位置にいる)。この対称性のために、
        // 成功率が 50% になる d で釣り合う (d = 0.5 に戻るのではない: 係数が回ごとに小さくなるので、成功・失敗を交互に与えると
        // 初めの向きの側 (Increase からなら 0.6 付近) に落ち着く)
        let mut c = Adaptive::default();
        let mut m = Adaptive::default();
        let mut x = 0x9e37_79b9_7f4a_7c15u64;
        for i in 0..300 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            if x % 2 == 0 {
                c.increase();
                m.decrease();
            } else {
                c.decrease();
                m.increase();
            }
            assert!((c.value + m.value - 1.0).abs() < 1e-9, "step {i}: {} + {} != 1", c.value, m.value);
            assert!((0.0..=1.0).contains(&c.value));
        }
        let mut a = Adaptive::default();
        for i in 0..200 {
            if i % 2 == 0 { a.increase() } else { a.decrease() }
        }
        assert!(a.value > 0.5 && a.value < 0.75, "alternating from Increase settles above 0.5: {}", a.value);
        // 値は [0, 1] から出ない
        let mut e = Adaptive::default();
        for _ in 0..100 {
            e.increase();
        }
        assert!(e.value <= 1.0 && e.value > 0.9);
    }
}
