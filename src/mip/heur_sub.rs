//! サブ MIP を使う主ヒューリスティクスの試作 (analysis/heur_sub_20261023.md。どれも既定では使わない):
//! zero-objective (SCIP の heur_zeroobj)・Alternating Criteria Search (Munguía ほか 2018)・
//! 繰り返しの Local Branching (Fischetti & Lodi 2003、SCIP の heur_localbranching)・
//! 中心を移す Proximity Search (Fischetti & Monaci 2014)。

use super::lp_api::MipLp;
use super::problem::MipProblem;
use super::solver::{MipParams, MipResult, MipStatus, Solver};
use std::time::Instant;

/// サブ MIP の設定 ([`Solver::solve_sub_raw`])。
#[derive(Clone, Copy, Debug)]
pub(super) struct SubCfg {
    /// 時間の上限 [秒] (外側の残り時間でも頭打ち)。
    pub time: f64,
    pub nodes: u64,
    /// 打ち切り値 (サブ MIP の目的関数で、定数項込み)。
    pub cutoff: f64,
    pub rel_gap: f64,
    /// 根のカットのループを通常の求解と同じ 25 ラウンドにする (`fc_submip` の設定を流用)。
    pub full_cuts: bool,
}

/// 試作のヒューリスティクスの時間の上限: 残りの `frac` 倍、最大 `max` 秒。
fn time_cap(remaining: f64, frac: f64, max: f64) -> f64 {
    (frac * remaining).min(max)
}

/// `ENOMOTO_MIP_HSUB_NOCUTS`: 試作のサブ MIP の根のカットをサブ MIP の既定 (5 ラウンド) に戻す。
fn full_cuts_on() -> bool {
    env_str!("ENOMOTO_MIP_HSUB_NOCUTS").is_none()
}

thread_local! {
    /// このスレッドで試作のサブ MIP ([`Solver::solve_sub_raw`]) を解いている間だけ立てる印。
    static HSUB_SUB: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 試作のサブ MIP の根では Feasibility Pump・ダイビングを使わない (`ENOMOTO_MIP_HSUB_FULL_HEUR` で使う)。
/// mzzv42z の根の Local Branching の近傍では Pump が 2 秒使い、3 秒のサブ MIP が 0 ノードで終わっていた。
pub(super) fn hsub_light() -> bool {
    HSUB_SUB.with(|h| h.get()) && env_str!("ENOMOTO_MIP_HSUB_FULL_HEUR").is_none()
}

/// 試作のサブ MIP の中では入れ子の RENS を使う (HiGHS のサブ MIP と同じ。`ENOMOTO_MIP_HSUB_NO_RENS` で使わない。
/// 深さの上限は `ENOMOTO_T_MIP_SUBMIP_RENS_DEPTH`)。
pub(super) fn hsub_rens() -> bool {
    HSUB_SUB.with(|h| h.get()) && env_str!("ENOMOTO_MIP_HSUB_NO_RENS").is_none()
}

impl<'a, L: MipLp> Solver<'a, L> {
    fn remaining_secs(&self) -> f64 {
        match self.deadline {
            Some(d) => d.saturating_duration_since(Instant::now()).as_secs_f64(),
            None => f64::INFINITY,
        }
    }

    /// 加工した問題 `sub` (列の数は元と違ってよい) を、設定 `cfg` のサブ MIP として 1 スレッドで解き、結果をそのまま返す
    /// (暫定解の候補にはしない)。入れ子の深さ ([`super::heuristics::SUBMIP_DEPTH`]) を数える。
    pub(super) fn solve_sub_raw(&mut self, sub: &MipProblem, cfg: SubCfg) -> MipResult {
        let params = MipParams {
            time_limit: cfg.time.min(self.remaining_secs()).max(0.0),
            node_limit: cfg.nodes,
            rel_gap: cfg.rel_gap,
            abs_gap: self.params.abs_gap.max(1e-6),
            verbose: self.params.verbose && env_str!("ENOMOTO_MIP_SUBMIP_VERBOSE").is_some(),
            submip: true,
            cutoff: cfg.cutoff,
            restarts: 0,
            skip_heurs: 0,
            threads: 1,
            distinguish: false,
            fc_submip: cfg.full_cuts,
        };
        let depth = super::heuristics::SUBMIP_DEPTH.with(|d| d.get());
        super::heuristics::SUBMIP_DEPTH.with(|d| d.set(depth + 1));
        let was = HSUB_SUB.with(|h| h.replace(true));
        let r = super::solve_problem(sub, params, env_str!("ENOMOTO_MIP_SUBMIP_NO_PRESOLVE").is_none());
        HSUB_SUB.with(|h| h.set(was));
        super::heuristics::SUBMIP_DEPTH.with(|d| d.set(depth));
        self.heur_iters += r.lp_iterations;
        r
    }

    /// 試作のヒューリスティクスの 1 回の記録 (表を作るための行)。
    fn hsub_log(&self, name: &str, what: &str, found: bool, t: Instant) {
        if self.params.verbose {
            eprintln!("MIP: hsub {name}: {what} -> found {found} ({:.2}s, at {:.2}s)", t.elapsed().as_secs_f64(), self.start.elapsed().as_secs_f64());
        }
    }

    /// zero-objective (SCIP の heur_zeroobj): 目的関数を 0 にした問題 (境界は今の大域的な境界) を、ノード数を絞った
    /// サブ MIP で解く。目的値が 0 なので最初の実行可能解で下界と一致して止まる。暫定解があれば元の目的値の
    /// 打ち切りを行として加える (SCIP と同じ)。`ENOMOTO_T_MIP_ZEROOBJ_NODES` (1000)・`_TIME_FRAC` (0.25)・`_TIME_MAX` (20 秒)。
    pub(super) fn zero_objective(&mut self) -> bool {
        if self.params.submip {
            return false;
        }
        let t = Instant::now();
        let p = self.p;
        let mut rows = p.rows.clone();
        let mut row_lo = p.row_lo.clone();
        let mut row_up = p.row_up.clone();
        if self.incumbent.is_some() {
            let lim = self.prune_limit();
            rows.push((0..p.n).filter(|&j| p.cost[j] != 0.0).map(|j| (j, p.cost[j])).collect());
            row_lo.push(f64::NEG_INFINITY);
            row_up.push(lim - p.offset);
        }
        let mut sub = MipProblem::from_rows(self.dom.global_lo.clone(), self.dom.global_up.clone(), vec![0.0; p.n], 0.0, p.sense_sign, p.is_int.clone(), rows, row_lo, row_up);
        sub.conflict_edges = p.conflict_edges.clone();
        sub.implied_bounds = p.implied_bounds.clone();
        let cfg = SubCfg {
            time: time_cap(self.remaining_secs(), tunable!("ENOMOTO_T_MIP_ZEROOBJ_TIME_FRAC", 0.25, f64), tunable!("ENOMOTO_T_MIP_ZEROOBJ_TIME_MAX", 20.0, f64)),
            nodes: tunable!("ENOMOTO_T_MIP_ZEROOBJ_NODES", 1000u64, u64),
            cutoff: f64::INFINITY,
            rel_gap: 1e-4,
            full_cuts: full_cuts_on(),
        };
        let r = self.solve_sub_raw(&sub, cfg);
        let found = r.x.is_some_and(|x| self.try_incumbent(x));
        self.hsub_stats[0].0 += 1;
        self.hsub_stats[0].1 += found as u32;
        self.hsub_log("zeroobj", &format!("{:?}, {} nodes, limit {:.1}s", r.status, r.nodes, cfg.time), found, t);
        found
    }

    /// Alternating Criteria Search (Munguía, Ahmed, Bader, Nemhauser, Shao 2018 の逐次版): 整数の点 `x̂` から、整数列の
    /// 一部を `x̂` に固定したサブ MIP を
    /// - 実行可能性の段: 各行に違反量の列を足し、違反量の合計を最小にする (元の目的値を前の段の値以下に保つ)
    /// - 最適性の段: 元の目的値を最小にする (違反量の合計を前の段の値以下に保つ。0 なら元の問題)
    ///
    /// で交互に解き、`x̂` を結果で置き換える。既定では違反量が 0 になるまでは実行可能性の段だけを回し (暫定解探しに
    /// 使うため)、0 になったら最適性の段だけを回す (`ENOMOTO_MIP_ACS_ALT=1` で原典どおり毎回交互)。固定する列は
    /// 乱数 (各整数列を固定率で) と区間 (整数列の並びの連続した区間) を交互に選ぶ (`ENOMOTO_MIP_ACS_FIX=rand|block`、`viol` なら
    /// 違反行の近くを自由にする選び方も混ぜて 3 つを順に)。
    /// 固定率は初め `ENOMOTO_T_MIP_ACS_FIX_RATE` (0.7)、サブ MIP が解き切って良くならなければ 0.1 下げ (最小 `_MIN_RATE` 0.1)、
    /// 制限で止まって良くならなければ 0.05 上げる (最大 0.95)。`start` は出発点 (なければ LP 解 `xlp` を丸めたもの)。
    /// 回数 `ENOMOTO_T_MIP_ACS_ROUNDS` (30)、全体の時間 残りの `_TIME_FRAC` (0.3)・最大 `_TIME_MAX` (30 秒)、
    /// 1 回のサブ MIP は `_CALL_MAX` (5 秒)・`_NODES` (500)。解が出た後の最適性の段は `_OPT_PATIENCE` (2) 回続けて
    /// 良くならなければ止める。暫定解があれば使わない (`ENOMOTO_MIP_ACS=2` なら暫定解を出発点にして最適性の段から)。
    pub(super) fn acs(&mut self, xlp: &[f64], start: Option<Vec<f64>>) -> bool {
        if self.params.submip || self.p.m == 0 {
            return false;
        }
        let p = self.p;
        let n = p.n;
        let t0 = Instant::now();
        let total = time_cap(self.remaining_secs(), tunable!("ENOMOTO_T_MIP_ACS_TIME_FRAC", 0.3, f64), tunable!("ENOMOTO_T_MIP_ACS_TIME_MAX", 30.0, f64));
        let rounds = tunable!("ENOMOTO_T_MIP_ACS_ROUNDS", 30usize, usize);
        let call_max = tunable!("ENOMOTO_T_MIP_ACS_CALL_MAX", 5.0, f64);
        let nodes = tunable!("ENOMOTO_T_MIP_ACS_NODES", 500u64, u64);
        let patience = tunable!("ENOMOTO_T_MIP_ACS_OPT_PATIENCE", 2usize, usize);
        let alt = env_str!("ENOMOTO_MIP_ACS_ALT").is_some();
        let fix_mode = env_str!("ENOMOTO_MIP_ACS_FIX").unwrap_or("alt");
        let mut rate = tunable!("ENOMOTO_T_MIP_ACS_FIX_RATE", 0.7, f64);
        let min_rate = tunable!("ENOMOTO_T_MIP_ACS_MIN_RATE", 0.1, f64);
        let (glo, gup) = (self.dom.global_lo.clone(), self.dom.global_up.clone());
        let ints: Vec<usize> = (0..n).filter(|&j| p.is_int[j] && glo[j] < gup[j]).collect();
        if ints.is_empty() {
            return false;
        }
        // 出発点: 整数列は丸めて境界に収める
        let base = match (&self.incumbent, start) {
            (Some((_, x)), _) => x.clone(),
            (None, Some(x)) => x,
            (None, None) => xlp.to_vec(),
        };
        let mut xh: Vec<f64> = (0..n).map(|j| if p.is_int[j] { base[j].round().clamp(glo[j], gup[j]) } else { base[j].clamp(glo[j], gup[j]) }).collect();
        // 行の大きさ (違反量の列の係数。min relaxation と同じ)
        let scale: Vec<f64> = p.rows.iter().map(|r| r.iter().fold(0.0f64, |m, &(_, a)| m.max(a.abs())).max(1e-9)).collect();
        let viol_of = |x: &[f64]| -> f64 {
            let mut v = 0.0;
            for i in 0..p.m {
                let a: f64 = p.rows[i].iter().map(|&(j, c)| c * x[j]).sum();
                if a < p.row_lo[i] {
                    v += (p.row_lo[i] - a) / scale[i];
                } else if a > p.row_up[i] {
                    v += (a - p.row_up[i]) / scale[i];
                }
            }
            v
        };
        let mut viol = if self.incumbent.is_some() { 0.0 } else { viol_of(&xh) };
        let mut zcur = if viol <= 1e-9 { p.objective(&xh) } else { f64::INFINITY };
        let mut found_any = false;
        let mut opt_fails = 0usize;
        let mut it = 0usize;
        let mut feas_phase = viol > 1e-9;
        if self.params.verbose {
            eprintln!("MIP: hsub ACS: start violation {viol:.4e} ({} integer columns, time {total:.1}s)", ints.len());
        }
        while it < rounds && !self.time_up() && t0.elapsed().as_secs_f64() < total {
            it += 1;
            let ts = Instant::now();
            // 固定する整数列
            // 固定の選び方: 0 = 乱数、1 = 区間、2 = 違反行の近く (`viol`: 違反している行の整数列と、そこから行を 1 段
            // たどった整数列 (短い行から) を自由にし、自由な列の数が (1 - 固定率) x 整数列の数に届くまで足す。
            // 原典にはない。違反行がなければ乱数)
            let kind = match fix_mode {
                "rand" => 0,
                "block" => 1,
                "viol" => [0, 1, 2][it % 3],
                _ => it % 2,
            };
            let mut fixed = vec![false; n];
            let mut kind = kind;
            if kind == 2 {
                let nfree = ((1.0 - rate) * ints.len() as f64).ceil() as usize;
                let act: Vec<f64> = p.rows.iter().map(|r| r.iter().map(|&(j, c)| c * xh[j]).sum()).collect();
                let tol = |b: f64| 1e-6 * (1.0 + b.abs());
                let viol_rows: Vec<usize> = (0..p.m).filter(|&i| act[i] < p.row_lo[i] - tol(p.row_lo[i]) || act[i] > p.row_up[i] + tol(p.row_up[i])).collect();
                if viol_rows.is_empty() {
                    kind = 0;
                } else {
                    let mut free = vec![false; n];
                    let mut cnt = 0usize;
                    let mut frontier: Vec<usize> = Vec::new();
                    for &i in &viol_rows {
                        for &(j, _) in &p.rows[i] {
                            if p.is_int[j] && glo[j] < gup[j] && !free[j] {
                                free[j] = true;
                                cnt += 1;
                                frontier.push(j);
                            }
                        }
                    }
                    let mut seen_row = vec![false; p.m];
                    for &i in &viol_rows {
                        seen_row[i] = true;
                    }
                    // 1 段ずつ広げる (行は短い順)
                    while cnt < nfree && !frontier.is_empty() {
                        let mut rws: Vec<usize> = Vec::new();
                        for &j in &frontier {
                            for &(i, _) in &p.cols[j] {
                                if !seen_row[i] {
                                    seen_row[i] = true;
                                    rws.push(i);
                                }
                            }
                        }
                        rws.sort_by_key(|&i| p.rows[i].len());
                        let mut next = Vec::new();
                        'outer: for i in rws {
                            for &(j, _) in &p.rows[i] {
                                if cnt >= nfree {
                                    break 'outer;
                                }
                                if p.is_int[j] && glo[j] < gup[j] && !free[j] {
                                    free[j] = true;
                                    cnt += 1;
                                    next.push(j);
                                }
                            }
                        }
                        frontier = next;
                    }
                    for &j in &ints {
                        fixed[j] = !free[j];
                    }
                }
            }
            if kind == 1 {
                let nfree = ((1.0 - rate) * ints.len() as f64).ceil() as usize;
                let s = (self.rand() * ints.len() as f64) as usize;
                for (k, &j) in ints.iter().enumerate() {
                    // 区間 [s, s + nfree) (巡回) の外を固定する
                    let d = (k + ints.len() - s) % ints.len();
                    fixed[j] = d >= nfree;
                }
            } else if kind == 0 {
                for &j in &ints {
                    fixed[j] = self.rand() < rate;
                }
            }
            let mut col_lo = glo.clone();
            let mut col_up = gup.clone();
            for &j in &ints {
                if fixed[j] {
                    col_lo[j] = xh[j];
                    col_up[j] = xh[j];
                }
            }
            let remain_total = (total - t0.elapsed().as_secs_f64()).max(0.0);
            let mut cfg = SubCfg { time: call_max.min(remain_total), nodes, cutoff: f64::INFINITY, rel_gap: 1e-4, full_cuts: full_cuts_on() };
            let mut rows = p.rows.clone();
            let mut row_lo = p.row_lo.clone();
            let mut row_up = p.row_up.clone();
            let cost_row: Vec<(usize, f64)> = (0..n).filter(|&j| p.cost[j] != 0.0).map(|j| (j, p.cost[j])).collect();
            let do_feas = feas_phase;
            let with_slack = do_feas || viol > 1e-9;
            let mut cost = if do_feas { vec![0.0; n] } else { p.cost.clone() };
            let mut is_int = p.is_int.clone();
            let mut slack_cols = Vec::new();
            if with_slack {
                for i in 0..p.m {
                    for (bound, sign) in [(p.row_lo[i], 1.0), (p.row_up[i], -1.0)] {
                        if bound.is_finite() {
                            rows[i].push((col_lo.len(), sign * scale[i]));
                            slack_cols.push(col_lo.len());
                            col_lo.push(0.0);
                            col_up.push(f64::INFINITY);
                            cost.push(if do_feas { 1.0 } else { 0.0 });
                            is_int.push(false);
                        }
                    }
                }
            }
            if do_feas {
                // 元の目的値を今の値以下に保つ
                if zcur.is_finite() {
                    rows.push(cost_row.clone());
                    row_lo.push(f64::NEG_INFINITY);
                    row_up.push(zcur - p.offset);
                }
                // 違反量を今より小さくする
                if viol.is_finite() && it > 1 {
                    cfg.cutoff = viol - 1e-7 * viol.max(1.0);
                }
            } else {
                if with_slack {
                    rows.push(slack_cols.iter().map(|&k| (k, 1.0)).collect());
                    row_lo.push(f64::NEG_INFINITY);
                    row_up.push(viol);
                }
                cfg.cutoff = if found_any { self.prune_limit() } else { zcur - 1e-9 * zcur.abs().max(1.0) };
            }
            let sub = MipProblem::from_rows(col_lo, col_up, cost, if do_feas { 0.0 } else { p.offset }, if do_feas { 1.0 } else { p.sense_sign }, is_int, rows, row_lo, row_up);
            let r = self.solve_sub_raw(&sub, cfg);
            let mut improved = false;
            if let Some(xs) = r.x.as_ref() {
                let x: Vec<f64> = xs[..n].iter().enumerate().map(|(j, &v)| if p.is_int[j] { v.round() } else { v }).collect();
                let v2 = viol_of(&x);
                let z2 = p.objective(&x);
                if do_feas {
                    improved = v2 < viol - 1e-9 * viol.max(1.0) || viol.is_infinite();
                } else {
                    improved = z2 < zcur - 1e-9 * zcur.abs().max(1.0) && v2 <= viol + 1e-9;
                }
                if improved {
                    xh = x.clone();
                    viol = v2;
                    zcur = if do_feas && v2 > 1e-9 { if zcur.is_finite() { z2.min(zcur) } else { f64::INFINITY } } else { z2 };
                }
                if v2 <= 1e-9 && self.try_incumbent(x) {
                    found_any = true;
                    zcur = self.incumbent.as_ref().map_or(zcur, |(z, _)| *z);
                }
            }
            let solved = matches!(r.status, MipStatus::Optimal | MipStatus::Infeasible);
            if !improved {
                if solved {
                    rate = (rate - 0.1).max(min_rate);
                } else {
                    rate = (rate + 0.05).min(0.95);
                }
            }
            if self.params.verbose {
                eprintln!(
                    "MIP: hsub ACS round {it} ({} {}, fix rate {rate:.2}): {:?}, {} nodes, violation {viol:.4e}, objective {zcur:.6e}, improved {improved} ({:.2}s)",
                    if do_feas { "feasibility" } else { "optimality" },
                    ["random", "block", "violated rows"][kind],
                    r.status,
                    r.nodes,
                    ts.elapsed().as_secs_f64()
                );
            }
            // 次の段
            if viol <= 1e-9 {
                feas_phase = false;
                if !do_feas {
                    opt_fails = if improved { 0 } else { opt_fails + 1 };
                    if opt_fails >= patience {
                        break;
                    }
                }
            } else {
                feas_phase = if alt { !do_feas } else { true };
            }
        }
        self.hsub_stats[1].0 += 1;
        self.hsub_stats[1].1 += found_any as u32;
        self.hsub_log("ACS", &format!("{it} rounds, final violation {viol:.4e}"), found_any, t0);
        found_any
    }

    /// 暫定解の 2 値列の Hamming 距離の行 ([`Self::hamming_row`] と同じ。こちらの段で使う写し)。
    fn hamming_row2(&self, inc: &[f64]) -> (Vec<(usize, f64)>, f64) {
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

    /// 繰り返しの Local Branching (Fischetti & Lodi 2003 を SCIP の heur_localbranching の形で): 暫定解から 2 値列を
    /// `k` (初め `ENOMOTO_T_MIP_LB2_K` = 18) 個以内しか変えない行を足したサブ MIP を、打ち切り値
    /// `z - minimprove (z - lb)` (`_MINIMPROVE` 0.01) で解く。良くなればその解を中心にやり直し、解き切って解がなければ
    /// k を 1.5 倍、制限で止まって解がなければ k を半分にする。`lb` は下界。全体の時間は残りの `_TIME_FRAC` (0.2)・
    /// 最大 `_TIME_MAX` (20 秒)、1 回は `_CALL_MAX` (8 秒)・`_NODES` (1000)、回数 `_ROUNDS` (10)、良くならない回が
    /// `_PATIENCE` (3) 回続けば止める。
    pub(super) fn local_branching2(&mut self, lb: f64, frac_scale: f64) -> bool {
        if self.params.submip || self.incumbent.is_none() {
            return false;
        }
        let p = self.p;
        let t0 = Instant::now();
        let total = frac_scale * time_cap(self.remaining_secs(), tunable!("ENOMOTO_T_MIP_LB2_TIME_FRAC", 0.2, f64), tunable!("ENOMOTO_T_MIP_LB2_TIME_MAX", 20.0, f64));
        let call_max = tunable!("ENOMOTO_T_MIP_LB2_CALL_MAX", 8.0, f64);
        let minimp = tunable!("ENOMOTO_T_MIP_LB2_MINIMPROVE", 0.01, f64);
        let rounds = tunable!("ENOMOTO_T_MIP_LB2_ROUNDS", 10usize, usize);
        let patience = tunable!("ENOMOTO_T_MIP_LB2_PATIENCE", 3usize, usize);
        let mut fails = 0usize;
        let mut any = false;
        let mut it = 0usize;
        while it < rounds && fails < patience && !self.time_up() && t0.elapsed().as_secs_f64() < total {
            it += 1;
            let ts = Instant::now();
            let (z, inc) = self.incumbent.as_ref().map(|(z, x)| (*z, x.clone())).unwrap();
            let (row, ones) = self.hamming_row2(&inc);
            if row.len() < 10 {
                break;
            }
            let k = self.lb2_k.clamp(2.0, row.len() as f64 / 2.0).round();
            let mut rows = p.rows.clone();
            let mut row_lo = p.row_lo.clone();
            let mut row_up = p.row_up.clone();
            rows.push(row);
            row_lo.push(f64::NEG_INFINITY);
            row_up.push(k - ones);
            let mut sub = MipProblem::from_rows(self.dom.global_lo.clone(), self.dom.global_up.clone(), p.cost.clone(), p.offset, p.sense_sign, p.is_int.clone(), rows, row_lo, row_up);
            sub.conflict_edges = p.conflict_edges.clone();
            sub.implied_bounds = p.implied_bounds.clone();
            let cutoff = if lb.is_finite() && lb < z { (z - minimp * (z - lb)).min(self.prune_limit()) } else { self.prune_limit() };
            let cfg = SubCfg {
                time: call_max.min((total - t0.elapsed().as_secs_f64()).max(0.0)),
                nodes: tunable!("ENOMOTO_T_MIP_LB2_NODES", 1000u64, u64),
                cutoff,
                rel_gap: 1e-4,
                full_cuts: full_cuts_on(),
            };
            let r = self.solve_sub_raw(&sub, cfg);
            let acc = r.x.is_some_and(|x| self.try_incumbent(x));
            let solved = matches!(r.status, MipStatus::Optimal | MipStatus::Infeasible);
            if acc {
                fails = 0;
                any = true;
            } else {
                fails += 1;
                if solved {
                    self.lb2_k *= 1.5;
                } else {
                    self.lb2_k *= 0.5;
                }
                self.lb2_k = self.lb2_k.clamp(2.0, 1000.0);
            }
            self.hsub_stats[2].0 += 1;
            self.hsub_stats[2].1 += acc as u32;
            self.hsub_log("LB2", &format!("k={k}, {:?}, {} nodes", r.status, r.nodes), acc, ts);
        }
        any
    }

    /// 中心を移す Proximity Search (Fischetti & Monaci 2014): 目的関数を暫定解からの Hamming 距離 (2 値列) にし、
    /// 元の目的値 `<= z - theta` の行を足したサブ MIP を、最初の実行可能解で止めて解く (`ENOMOTO_MIP_PROX2_FULL` なら
    /// 距離の最小まで)。解が出たら暫定解にして中心を移しやり直す。解き切って解がなければ (z - theta より良い解が
    /// ない) theta を半分にし、刻みより小さくなれば止める。制限で止まれば theta を半分にする。theta の初めは
    /// `max(目的関数の刻み, ENOMOTO_T_MIP_PROX2_THETA (0.05) * (z - lb))` (下界がなければ 相対 1e-3)。
    /// 時間・回数は Local Branching と同じ形 (`ENOMOTO_T_MIP_PROX2_*`)。
    pub(super) fn proximity2(&mut self, lb: f64, frac_scale: f64) -> bool {
        if self.params.submip || self.incumbent.is_none() {
            return false;
        }
        let p = self.p;
        let t0 = Instant::now();
        let total = frac_scale * time_cap(self.remaining_secs(), tunable!("ENOMOTO_T_MIP_PROX2_TIME_FRAC", 0.2, f64), tunable!("ENOMOTO_T_MIP_PROX2_TIME_MAX", 20.0, f64));
        let call_max = tunable!("ENOMOTO_T_MIP_PROX2_CALL_MAX", 8.0, f64);
        let rounds = tunable!("ENOMOTO_T_MIP_PROX2_ROUNDS", 20usize, usize);
        let patience = tunable!("ENOMOTO_T_MIP_PROX2_PATIENCE", 3usize, usize);
        let full = env_str!("ENOMOTO_MIP_PROX2_FULL").is_some();
        let step = self.obj_step.unwrap_or(0.0);
        let z0 = self.incumbent.as_ref().unwrap().0;
        let mut theta = if lb.is_finite() && lb < z0 { tunable!("ENOMOTO_T_MIP_PROX2_THETA", 0.05, f64) * (z0 - lb) } else { 1e-3 * z0.abs().max(1.0) };
        theta = theta.max(step).max(1e-6 * z0.abs().max(1.0));
        let mut fails = 0usize;
        let mut any = false;
        let mut it = 0usize;
        while it < rounds && fails < patience && !self.time_up() && t0.elapsed().as_secs_f64() < total {
            it += 1;
            let ts = Instant::now();
            let (z, inc) = self.incumbent.as_ref().map(|(z, x)| (*z, x.clone())).unwrap();
            let (row, ones) = self.hamming_row2(&inc);
            if row.len() < 10 {
                break;
            }
            let mut cost = vec![0.0; p.n];
            for &(j, c) in &row {
                cost[j] = c;
            }
            let mut rows = p.rows.clone();
            let mut row_lo = p.row_lo.clone();
            let mut row_up = p.row_up.clone();
            rows.push((0..p.n).filter(|&j| p.cost[j] != 0.0).map(|j| (j, p.cost[j])).collect());
            row_lo.push(f64::NEG_INFINITY);
            // 目的値の刻みがあれば、刻みに合わせた上限 (z - theta) を少し緩めて丸め誤差で落とさない
            let target = z - theta;
            row_up.push(target - p.offset + 1e-9 * z.abs().max(1.0));
            let mut sub = MipProblem::from_rows(self.dom.global_lo.clone(), self.dom.global_up.clone(), cost, ones, 1.0, p.is_int.clone(), rows, row_lo, row_up);
            sub.conflict_edges = p.conflict_edges.clone();
            sub.implied_bounds = p.implied_bounds.clone();
            let cfg = SubCfg {
                time: call_max.min((total - t0.elapsed().as_secs_f64()).max(0.0)),
                nodes: tunable!("ENOMOTO_T_MIP_PROX2_NODES", 1000u64, u64),
                cutoff: f64::INFINITY,
                // 最初の解で止める: ギャップを大きくすれば最初の暫定解で終わる
                rel_gap: if full { 1e-4 } else { 1e30 },
                full_cuts: full_cuts_on(),
            };
            let r = self.solve_sub_raw(&sub, cfg);
            let acc = r.x.is_some_and(|x| self.try_incumbent(x));
            let proven = !acc && r.status == MipStatus::Infeasible;
            if acc {
                fails = 0;
                any = true;
            } else {
                fails += 1;
                if proven && theta <= step.max(1e-6 * z.abs().max(1.0)) * 1.0001 {
                    // 刻みだけ良い解もない: 暫定解は (この近傍の定義で) 最適
                    self.hsub_stats[3].0 += 1;
                    self.hsub_log("PROX2", &format!("theta={theta:.4e}, {:?}, {} nodes (optimal)", r.status, r.nodes), false, ts);
                    break;
                }
                theta = (0.5 * theta).max(step).max(1e-6 * z.abs().max(1.0));
            }
            self.hsub_stats[3].0 += 1;
            self.hsub_stats[3].1 += acc as u32;
            self.hsub_log("PROX2", &format!("theta={theta:.4e}, {:?}, {} nodes", r.status, r.nodes), acc, ts);
        }
        any
    }
}
