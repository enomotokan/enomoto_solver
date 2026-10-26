//! 繰り返しの Local Branching (`ENOMOTO_MIP_LB2`) と中心を移す Proximity Search (`ENOMOTO_MIP_PROX2`)。
//! `analysis/heur_sub_20261023.md` の試作 (LB2・PROX2 の改訂 2 = v3) の作り直し (試作のソースは残っていなかった)。
//!
//! - [`Solver::local_branching2`] (Fischetti & Lodi 2003、SCIP heur_localbranching の形): 暫定解から 2 値列を k 個以内しか
//!   変えない行を足したサブ MIP を、打ち切り値 `z - 0.01 (z - lb)` (SCIP の minimprove) で解く。良くなればその解を中心に
//!   やり直す。解き切って解がなければ k を 1.5 倍、制限で止まって解がなければ半分 (初め k = 18)。全体 残りの 0.2・最大
//!   20 秒、1 回 8 秒・1000 ノード、10 回まで、3 回続けて駄目なら止める。
//! - [`Solver::proximity2`] (Fischetti & Monaci 2014): 目的関数を暫定解からの 2 値列の Hamming 距離にし、元の目的値
//!   ≤ z − θ の行を足したサブ MIP を、最初の実行可能解で止めて解く。解が出たら中心を移してやり直す。解がなければ θ を
//!   半分に (刻みより小さくなれば止める)。θ の初め = max(目的関数の刻み, 0.05 (z − lb))。時間は LB2 と同じ形 (20 回まで)。
//!
//! 呼ぶ条件 ([`Solver::hsub_root_ok`]・[`Solver::hsub_tree_ok`]): 根は再スタート前の根で、暫定解と根の下界の差が相対
//! `ENOMOTO_T_MIP_HSUB_MIN_GAP` (0.01) を超え、外側の求解の始まりから `ENOMOTO_T_MIP_HSUB_ROOT_MIN_SECS` (10) 秒たっているとき。
//! 木は前に呼んだときより良い暫定解があり、差が 1% を超え、外側の始まりから `ENOMOTO_T_MIP_HSUB_TREE_MIN_SECS` (15) 秒たち、
//! 木で使った時間が経過時間の `ENOMOTO_T_MIP_HSUB_TREE_FRAC` (0.1) 倍未満のとき。
//!
//! 試作との違い: 試作のサブ MIP の根の設定 (カット 25 ラウンド、Pump・ダイビングを使わず入れ子の RENS) は入れていない
//! (サブ MIP は通常のサブ MIP の設定で解く)。

use std::time::Instant;

use super::lp_api::MipLp;
use super::problem::MipProblem;
use super::solver::{MipStatus, Solver};

/// LB2 を使うか (`ENOMOTO_MIP_LB2=1|root|tree`) と、`where_` (`"root"`/`"tree"`) で使うか。
pub(super) fn lb2_on(where_: &str) -> bool {
    env_str!("ENOMOTO_MIP_LB2").is_some_and(|s| s == "1" || s == where_)
}

/// PROX2 を使うか (`ENOMOTO_MIP_PROX2=1|root|tree`)。
pub(super) fn prox2_on(where_: &str) -> bool {
    env_str!("ENOMOTO_MIP_PROX2").is_some_and(|s| s == "1" || s == where_)
}

/// 外側の求解の始まりからの秒。
fn outer_elapsed() -> f64 {
    super::RUN_STATS.lock().ok().and_then(|s| s.t0).map_or(0.0, |t| t.elapsed().as_secs_f64())
}

/// 改善の段の状態 (LB2・PROX2)。
#[derive(Default)]
pub(super) struct HsubState {
    /// LB2 の近傍の大きさ k (0 ならまだ決めていない)。
    pub lb_k: f64,
    /// 木で前に呼んだときの暫定解の値。
    pub tree_last_z: Option<f64>,
    /// 木で使った秒。
    pub tree_secs: f64,
    /// 呼んだ回数と暫定解を出した回数 (LB2, PROX2) — 診断用。
    pub calls: [u32; 2],
    pub wins: [u32; 2],
}

impl<'a, L: MipLp> Solver<'a, L> {
    fn hsub_gap_ok(&self, lb: f64) -> bool {
        let Some((z, _)) = self.incumbent.as_ref() else { return false };
        (z - lb) / z.abs().max(1.0) > tunable!("ENOMOTO_T_MIP_HSUB_MIN_GAP", 0.01, f64)
    }

    /// 根で LB2・PROX2 を呼ぶか (`lb` は根の下界、暫定解と同じ単位)。
    pub(super) fn hsub_root_ok(&self, lb: f64) -> bool {
        !self.params.submip && self.params.restarts == 0 && self.hsub_gap_ok(lb) && outer_elapsed() >= tunable!("ENOMOTO_T_MIP_HSUB_ROOT_MIN_SECS", 10.0, f64)
    }

    /// 木で LB2・PROX2 を呼ぶか。
    pub(super) fn hsub_tree_ok(&self, lb: f64) -> bool {
        if self.params.submip || !self.hsub_gap_ok(lb) {
            return false;
        }
        let z = self.incumbent.as_ref().unwrap().0;
        if self.hsub.tree_last_z.is_some_and(|z0| z >= z0 - 1e-9 * z0.abs().max(1.0)) {
            return false;
        }
        outer_elapsed() >= tunable!("ENOMOTO_T_MIP_HSUB_TREE_MIN_SECS", 15.0, f64)
            && self.hsub.tree_secs < tunable!("ENOMOTO_T_MIP_HSUB_TREE_FRAC", 0.1, f64) * self.start.elapsed().as_secs_f64()
    }

    /// 木で LB2・PROX2 を (使うものだけ) 呼ぶ。条件は呼ぶ側で [`Self::hsub_tree_ok`] を確かめる。
    pub(super) fn hsub_tree(&mut self, lb: f64) {
        let t = Instant::now();
        self.hsub.tree_last_z = self.incumbent.as_ref().map(|(z, _)| *z);
        if lb2_on("tree") {
            self.heur_label = "tree local branching 2";
            self.local_branching2(lb);
        }
        if prox2_on("tree") && self.hsub_gap_ok(lb) {
            self.heur_label = "tree proximity 2";
            self.proximity2(lb);
        }
        self.heur_label = "";
        self.hsub.tree_secs += t.elapsed().as_secs_f64();
        // 自分の改善で呼んだ後の暫定解を基準にする (同じ解でもう一度呼ばない)
        self.hsub.tree_last_z = self.incumbent.as_ref().map(|(z, _)| *z);
    }

    /// 全体の時間の上限 (残りの `frac`、最大 `max` 秒)。
    fn hsub_total_cap(&self, frac: f64, max: f64) -> f64 {
        let remaining = self.deadline.map_or(f64::INFINITY, |d| d.saturating_duration_since(Instant::now()).as_secs_f64());
        (frac * remaining).min(max)
    }

    /// サブ MIP を、1 回の時間 `secs`・ノード `nodes`・相対ギャップ `gap` (None なら通常) で解く。
    fn hsub_solve(&mut self, sub: MipProblem, secs: f64, nodes: u64, cutoff: Option<f64>, gap: Option<f64>) -> Option<(MipStatus, bool)> {
        let keep = (self.submip_time_frac, self.submip_time_max, self.submip_gap);
        self.submip_time_frac = 1.0;
        self.submip_time_max = secs;
        self.submip_gap = gap;
        let r = self.solve_submip_problem(sub, nodes, cutoff);
        (self.submip_time_frac, self.submip_time_max, self.submip_gap) = keep;
        r
    }

    /// 繰り返しの Local Branching (モジュールの説明を参照)。`lb` は下界 (暫定解と同じ単位)。暫定解を良くしたら真。
    pub(super) fn local_branching2(&mut self, lb: f64) -> bool {
        if self.params.submip || self.incumbent.is_none() {
            return false;
        }
        let t0 = Instant::now();
        let cap = self.hsub_total_cap(tunable!("ENOMOTO_T_MIP_LB2_TIME_FRAC", 0.2, f64), tunable!("ENOMOTO_T_MIP_LB2_TIME_MAX", 20.0, f64));
        if self.hsub.lb_k == 0.0 {
            self.hsub.lb_k = tunable!("ENOMOTO_T_MIP_LB2_K", 18.0, f64);
        }
        let (mut fails, mut any) = (0, false);
        for _ in 0..tunable!("ENOMOTO_T_MIP_LB2_CALLS", 10usize, usize) {
            let left = cap - t0.elapsed().as_secs_f64();
            if left < 0.1 || self.time_up() {
                break;
            }
            let (z, inc) = self.incumbent.as_ref().map(|(z, x)| (*z, x.clone())).unwrap();
            let (row, ones) = self.hamming_row(&inc);
            if row.len() < 10 {
                break;
            }
            let k = self.hsub.lb_k.clamp(2.0, (row.len() as f64 / 2.0).max(2.0)).round();
            let p = self.p;
            let mut rows = p.rows.clone();
            let mut row_lo = p.row_lo.clone();
            let mut row_up = p.row_up.clone();
            rows.push(row);
            row_lo.push(f64::NEG_INFINITY);
            row_up.push(k - ones);
            let sub = MipProblem::from_rows(self.dom.global_lo.clone(), self.dom.global_up.clone(), p.cost.clone(), p.offset, p.sense_sign, p.is_int.clone(), rows, row_lo, row_up);
            // SCIP の minimprove: 暫定解より差の 1% 以上良い解だけを探す
            let cutoff = (z - 0.01 * (z - lb).max(0.0)).min(self.prune_limit());
            let secs = left.min(tunable!("ENOMOTO_T_MIP_LB2_CALL_SECS", 8.0, f64));
            self.hsub.calls[0] += 1;
            let Some((st, acc)) = self.hsub_solve(sub, secs, 1000, Some(cutoff), None) else { break };
            if self.params.verbose {
                eprintln!("MIP:   local branching 2 k={k}: {st:?}, improved {acc} ({:.2}s)", t0.elapsed().as_secs_f64());
            }
            if acc {
                self.hsub.wins[0] += 1;
                any = true;
                fails = 0;
                continue;
            }
            fails += 1;
            match st {
                MipStatus::Optimal | MipStatus::Infeasible => self.hsub.lb_k *= 1.5,
                _ => self.hsub.lb_k /= 2.0,
            }
            self.hsub.lb_k = self.hsub.lb_k.clamp(2.0, 1000.0);
            if fails >= 3 {
                break;
            }
        }
        any
    }

    /// 中心を移す Proximity Search (モジュールの説明を参照)。`lb` は下界。暫定解を良くしたら真。
    pub(super) fn proximity2(&mut self, lb: f64) -> bool {
        if self.params.submip || self.incumbent.is_none() {
            return false;
        }
        let t0 = Instant::now();
        let cap = self.hsub_total_cap(tunable!("ENOMOTO_T_MIP_PROX2_TIME_FRAC", 0.2, f64), tunable!("ENOMOTO_T_MIP_PROX2_TIME_MAX", 20.0, f64));
        let z0 = self.incumbent.as_ref().unwrap().0;
        let step = self.obj_step.unwrap_or(0.0);
        let min_theta = step.max(1e-6 * z0.abs().max(1.0));
        let mut theta = step.max(tunable!("ENOMOTO_T_MIP_PROX2_THETA", 0.05, f64) * (z0 - lb).max(0.0)).max(min_theta);
        let (mut fails, mut any) = (0, false);
        for _ in 0..tunable!("ENOMOTO_T_MIP_PROX2_CALLS", 20usize, usize) {
            let left = cap - t0.elapsed().as_secs_f64();
            if left < 0.1 || self.time_up() {
                break;
            }
            let (z, inc) = self.incumbent.as_ref().map(|(z, x)| (*z, x.clone())).unwrap();
            if z - theta < lb - 1e-9 * lb.abs().max(1.0) {
                theta = 0.5 * (z - lb);
                if theta < min_theta {
                    break;
                }
            }
            let (row, ones) = self.hamming_row(&inc);
            if row.len() < 10 {
                break;
            }
            let p = self.p;
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
            let sub = MipProblem::from_rows(self.dom.global_lo.clone(), self.dom.global_up.clone(), cost, ones, p.sense_sign, p.is_int.clone(), rows, row_lo, row_up);
            let secs = left.min(tunable!("ENOMOTO_T_MIP_PROX2_CALL_SECS", 8.0, f64));
            self.hsub.calls[1] += 1;
            // 最初の実行可能解で止める (ギャップを 1e30 に)
            let Some((st, acc)) = self.hsub_solve(sub, secs, 1000, Some(f64::INFINITY), Some(1e30)) else { break };
            if self.params.verbose {
                eprintln!("MIP:   proximity 2 theta={theta:.6e}: {st:?}, improved {acc} ({:.2}s)", t0.elapsed().as_secs_f64());
            }
            if acc {
                self.hsub.wins[1] += 1;
                any = true;
                fails = 0;
                continue;
            }
            fails += 1;
            theta *= 0.5;
            if theta < min_theta || fails >= 3 {
                break;
            }
        }
        any
    }
}
