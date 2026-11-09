//! 担当 C の試作 (analysis/submip_impr_20261106/C.md、既定では使わない): 暫定解を改善するサブ MIP (RENS・RINS・被約費用) の
//! 固定の仕方を HiGHS (v1.15.1 の `HighsPrimalHeuristics::RENS`・`RINS`・`determineTargetFixingRate`・`solveSubMip`) に寄せる。
//!
//! `ENOMOTO_MIP_SUBFIX=<文字の並び>` (値 `1` は `ritc`):
//! - `r`: 外側の根の RENS (暫定解があるとき。今の既定は LP で整数値の列を全部固定する `rens_old`) を [`Solver::subfix_dive`] にする。
//!   整数列を lock の積の大きい順 (HiGHS の `setupIntCols`) に、LP 解の切り下げ・切り上げの範囲に締め (整数値なら固定)、残りの自由な列の
//!   1 割ずつ (`1 − (1 − 今の率) × 0.9`) 進めては LP を解き直し、目標の固定率で止める。締める列が尽きたら分数の列を `getFixVal`
//!   (根の LP 解からの変化が 0.4 以上ならその向き、でなければ費用の悪くなる向き、費用 0 なら近い整数) で丸め幅の小さい順に (丸め幅の和 0.5 まで) 固定する。
//! - `i`: 外側の根の RINS を同じダイブにする (LP 解と暫定解が一致する列を固定。尽きて固定率が 0.25 未満なら分数の列を `getFixVal` で足す)。
//! - `t`: 木の中の RINS (ALNS の RINS の腕) を同じダイブにする。
//! - `h`: 根の呼び方を HiGHS に寄せる: 外側の根の RINS を呼ばない (根の RENS は暫定解があっても呼ぶ。`r` と合わせて使う)。
//! - `c`: 根の被約費用のサブ MIP の固定率と結果も固定率の記録に入れる (HiGHS は RENS・RINS・被約費用で共通)。
//! - `n`: SUBFIX のサブ MIP のノード上限を HiGHS と同じ `200 + 外側のノード数 / 20` にする (既定の 500 の代わり)。
//! - `p`: 根の RENS の 1 回目は既定どおり並列のサブ MIP (別スレッド) で出し、受け取ったときに根 (1 ノード以下) で実行不能と分かったら、
//!   根のヒューリスティクスの並列のサブ MIP を受け取った後 ([`Solver::subfix_par_retry`]) に、目標を「達した固定率 × 0.5」にしたダイブを
//!   主のスレッドで解き直す (深さの半分まで戻る代わりに、今の LP 解から新しくダイブする)。
//! - `q` (保険、11-08): `r` を暫定解があるときだけにする (暫定解がなければ既定の RENS。`r` だけだと、暫定解のない根の 2 回目の RENS
//!   (`RENS (LP before cuts)`) もダイブにしてしまう)。根で実行不能のやり直しは、目標が `ENOMOTO_T_MIP_SUBFIX_RETRY_MIN` (0.2) 未満になるならしない。
//!   momentum1 (cand1107) では暫定解のない根で 0.58 → 0.29 → 0.15 → 0.09 とやり直し、最後のサブ MIP (自由な整数列 1039/1203) が 130 秒の時間切れで根を
//!   ふさぎ、既定で解を出す shift-and-propagate (85 秒) が回らなかった
//! - `f` (保険、11-08): 根の RENS のダイブが解を出さなければ、既定の RENS (LP で整数値の列を全部固定、並列のサブ MIP) も 1 回回す
//!
//! 目標の固定率は HiGHS の `determineTargetFixingRate` (初め 0.6、実行不能だった固定率の平均 × 0.9 を上の端、解を見つけた固定率の平均の
//! 0.9〜1.1 倍を含む範囲の乱数)。記録は外側の求解の全スレッドで共通 ([`STATS`]、求解の始めに [`reset`])。
//! ダイブの LP が実行不能か打ち切り値以上になったら、その段の固定を捨てて段の幅を半分にしてやり直す (最大 10 回。HiGHS は最後の分枝を
//! 逆にする深さ優先の戻り)。
//! 空振りのやり直し: サブ MIP が根 (1 ノード以下) で実行不能なら、目標を「達した固定率 × 0.5」にして、ダイブの深さ (締めた回数) の半分まで
//! 戻ってやり直す (深さの半分が 1 以下になるか、`ENOMOTO_T_MIP_SUBFIX_TRIES` (既定 5) 回でやめる)。
//! 固定率が 0.1 未満ならサブ MIP にしない (HiGHS は深さ優先の探索 10 回。ここでは何もしない)。

use super::domain::FEASTOL;
use super::lp::LpStatus;
use super::lp_api::MipLp;
use super::solver::{MipStatus, Solver};
use std::sync::Mutex;

/// `ENOMOTO_MIP_SUBFIX` の文字 `c` が入っているか
pub(crate) fn on(c: char) -> bool {
    static V: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let v = V.get_or_init(|| env_def!("ENOMOTO_MIP_SUBFIX", "rtcqf").map_or(String::new(), |v| if v == "1" { "ritc".to_string() } else { v.to_string() }));
    v.contains(c)
}

/// `p` で並列に出した RENS のサブ MIP のラベル (受け取りのときにやり直しの印を付けるため)
pub(crate) const SUBFIX_PAR_LABEL: &str = "SUBFIX RENS";

/// `ENOMOTO_MIP_SUBFIX` が何か付いているか
pub(crate) fn any() -> bool {
    env_def!("ENOMOTO_MIP_SUBFIX", "rtcqf").is_some_and(|v| !v.is_empty() && v != "0")
}

/// 外側の求解で共通の記録: (成功した固定率の和, 回数, 実行不能だった固定率の和, 回数) と、根の LP 解 (`getFixVal` の基準)
pub(crate) struct Shared {
    pub(crate) succ: (f64, u32),
    pub(crate) infeas: (f64, u32),
    pub(crate) root_x: Option<std::sync::Arc<Vec<f64>>>,
    /// ログの終わりの集計: [ダイブの回数, サブ MIP の回数, 解を受け入れた回数, 根で実行不能のやり直し, 固定率 < 0.1 で諦めた回数]
    pub(crate) count: [u64; 5],
}

pub(crate) static STATS: Mutex<Shared> = Mutex::new(Shared { succ: (0.0, 0), infeas: (0.0, 0), root_x: None, count: [0; 5] });

/// 外側の求解の始めに記録を消す
pub(crate) fn reset() {
    if let Ok(mut s) = STATS.lock() {
        *s = Shared { succ: (0.0, 0), infeas: (0.0, 0), root_x: None, count: [0; 5] };
    }
}

/// 固定率の記録に入れる ([`Solver::note_fix_rate`] から。外側の求解だけ)
pub(crate) fn note(fr: f64, infeasible: bool, improved: bool) {
    if let Ok(mut s) = STATS.lock() {
        if improved {
            s.succ.0 += fr;
            s.succ.1 += 1;
        } else if infeasible {
            s.infeas.0 += fr;
            s.infeas.1 += 1;
        }
    }
}

fn count(k: usize) {
    if let Ok(mut s) = STATS.lock() {
        s.count[k] += 1;
    }
}

/// ログの終わりに出す集計の文字列
pub(crate) fn summary() -> String {
    let s = STATS.lock().map(|s| (s.succ, s.infeas, s.count)).unwrap_or(((0.0, 0), (0.0, 0), [0; 5]));
    let avg = |a: (f64, u32)| if a.1 > 0 { a.0 / a.1 as f64 } else { f64::NAN };
    format!(
        "dives {} sub-MIPs {} accepted {} root-infeasible retries {} below 0.1 {}; success {} (mean rate {:.2}), infeasible {} (mean rate {:.2})",
        s.2[0], s.2[1], s.2[2], s.2[3], s.2[4], s.0 .1, avg(s.0), s.1 .1, avg(s.1)
    )
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 共通の記録からの目標の固定率 (HiGHS の `determineTargetFixingRate`)
    pub(super) fn subfix_target(&mut self) -> f64 {
        let (succ, infeas) = STATS.lock().map(|s| (s.succ, s.infeas)).unwrap_or(((0.0, 0), (0.0, 0)));
        let r0 = tunable!("ENOMOTO_T_MIP_SUBFIX_RATE0", 0.6, f64);
        let (mut low, mut high) = (r0, r0);
        if infeas.1 > 0 {
            high = 0.9 * infeas.0 / infeas.1 as f64;
            low = low.min(high);
        }
        if succ.1 > 0 {
            let r = succ.0 / succ.1 as f64;
            low = low.min(0.9 * r);
            high = high.max(1.1 * r);
        }
        (low + (high - low) * self.rand()).clamp(0.05, 0.95)
    }

    /// 外側の根なら、`getFixVal` の基準の根の LP 解を覚える
    pub(super) fn subfix_note_root_x(&self, x: &[f64]) {
        if !self.params.submip && self.nodes <= 1 && self.snap.is_none() {
            if let Ok(mut s) = STATS.lock() {
                s.root_x = Some(std::sync::Arc::new(x.to_vec()));
            }
        }
    }

    /// RENS (`inc` が `None`)・RINS (`inc` が暫定解) のダイブ ([モジュールの説明](self) を参照)。今の LP 解から始める。
    /// 解を受け入れたら真。定義域と LP は元に戻す。
    pub(super) fn subfix_dive(&mut self, inc: Option<&[f64]>, label: &'static str) -> bool {
        self.subfix_dive_with(inc, label, None)
    }

    /// `p` の並列の RENS が根で実行不能だったときのやり直し: 根の並列のサブ MIP を受け取った後に呼ぶ。主のスレッドで、目標を
    /// 「達した固定率 × 0.5」にしたダイブを解く (その後の空振りのやり直しも主で)。
    pub(super) fn subfix_par_retry(&mut self) -> bool {
        let pending = std::mem::take(&mut self.subfix_retry);
        let mut found = false;
        for fr in pending {
            if self.time_up() || 0.5 * fr < 0.1 {
                continue;
            }
            count(3);
            let keep = self.parallel_submips;
            self.parallel_submips = false;
            if self.params.verbose {
                eprintln!("MIP:   SUBFIX RENS (parallel) was infeasible at the root at fixing rate {fr:.2}; retrying on the main thread with target {:.2}", 0.5 * fr);
            }
            found |= self.subfix_dive_with(None, "RENS retry", Some(0.5 * fr));
            self.parallel_submips = keep;
        }
        found
    }

    fn subfix_dive_with(&mut self, inc: Option<&[f64]>, label: &'static str, target0: Option<f64>) -> bool {
        let p = self.p;
        let mut ints: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j] && self.dom.lo[j] < self.dom.up[j]).collect();
        if ints.is_empty() {
            return false;
        }
        count(0);
        let t0 = std::time::Instant::now();
        let verbose = self.params.verbose;
        // 列の順: lock の積 ((ε + 下) × (ε + 上)) の大きい順、同点は乱数 (HiGHS の `setupIntCols`)
        for k in (1..ints.len()).rev() {
            let r = ((self.rand() * (k + 1) as f64) as usize).min(k);
            ints.swap(k, r);
        }
        let score = |j: usize| {
            let (dl, ul) = self.locks[j];
            (1e-6 + dl as f64) * (1e-6 + ul as f64)
        };
        let sc: Vec<f64> = ints.iter().map(|&j| score(j)).collect();
        let mut idx: Vec<usize> = (0..ints.len()).collect();
        idx.sort_by(|&a, &b| sc[b].total_cmp(&sc[a]));
        let order: Vec<usize> = idx.into_iter().map(|k| ints[k]).collect();
        let nint = ints.len() as f64;
        let rate = |s: &Self| ints.iter().filter(|&&j| s.dom.lo[j] == s.dom.up[j]).count() as f64 / nint;
        let root_x = STATS.lock().ok().and_then(|s| s.root_x.clone()).filter(|r| r.len() == p.n);
        let fixval = |j: usize, v: f64, lo: f64, up: f64| -> f64 {
            let rc = root_x.as_ref().map_or(0.0, |r| v - r[j]);
            let c = p.cost[j];
            let f = if rc >= 0.4 {
                v.ceil()
            } else if rc <= -0.4 {
                v.floor()
            } else if c > 0.0 {
                v.ceil()
            } else if c < 0.0 {
                v.floor()
            } else {
                (v + 0.5).floor()
            };
            f.clamp(lo, up)
        };
        let saved = self.lp.save_state();
        let pos0 = self.dom.stack_len();
        let it_start = self.lp.total_iterations();
        // ダイブの LP 反復の上限 (やり直しを含む全体): ノードの平均の `ENOMOTO_T_MIP_SUBFIX_ITERS_MULT` (20) 倍 + 1 万
        let budget = (tunable!("ENOMOTO_T_MIP_SUBFIX_ITERS_MULT", 20.0, f64) * self.avg_node_iters().max(100) as f64) as u64 + 10_000;
        let max_tries = tunable!("ENOMOTO_T_MIP_SUBFIX_TRIES", 5usize, usize);
        let mut target = target0.unwrap_or_else(|| self.subfix_target());
        // 締めるたびの記録の位置 (ダイブの深さ = この長さ)
        let mut branch_pos: Vec<usize> = Vec::new();
        // 段の始めの `branch_pos` の長さ
        let mut stages: Vec<usize> = Vec::new();
        let mut step = 0.1f64;
        let mut found = false;
        let mut tries = 0usize;
        let mut need_lp = false;
        'retry: loop {
            let mut nback = 0usize;
            loop {
                if self.time_up() || self.lp.total_iterations() - it_start > budget {
                    break;
                }
                if need_lp {
                    self.sync_lp();
                    let it0 = self.lp.total_iterations();
                    let lim = budget.saturating_sub(self.lp.total_iterations() - it_start).max(500);
                    let st = self.lp.solve(&self.limits_k(lim, 2));
                    self.heur_iters += self.lp.total_iterations() - it0;
                    need_lp = false;
                    let cut = self.prune_limit();
                    let ok = st == LpStatus::Optimal && !(self.lp.objective() + p.offset >= cut - 1e-9 * cut.abs().max(1.0));
                    if !ok {
                        // 段の固定を捨てて、段の幅を半分にする
                        nback += 1;
                        let Some(s0) = stages.pop() else { break };
                        self.dom.backtrack_to(p, branch_pos[s0]);
                        branch_pos.truncate(s0);
                        step *= 0.5;
                        need_lp = true;
                        if nback >= 10 {
                            self.sync_lp();
                            let _ = self.lp.solve(&self.limits_k(budget.saturating_sub(self.lp.total_iterations() - it_start).max(500), 2));
                            need_lp = false;
                            break;
                        }
                        continue;
                    }
                }
                let fr = rate(self);
                if fr >= target {
                    break;
                }
                let x = self.lp.col_values();
                let stop_rate = (1.0 - (1.0 - fr) * (1.0 - step)).min(target);
                stages.push(branch_pos.len());
                let mut branched = 0usize;
                let mut conflict = false;
                for &j in &order {
                    let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
                    if lo == up {
                        continue;
                    }
                    let v = x[j];
                    let (nl, nu) = match inc {
                        None => ((v + FEASTOL).floor().min(up), (v - FEASTOL).ceil().max(lo)),
                        Some(inc) => {
                            if (v - inc[j]).abs() > FEASTOL || inc[j] < lo || inc[j] > up {
                                continue;
                            }
                            (inc[j], inc[j])
                        }
                    };
                    if nl <= lo && nu >= up {
                        continue;
                    }
                    branch_pos.push(self.dom.stack_len());
                    branched += 1;
                    if nl > lo {
                        self.dom.tighten_lower(p, j, nl);
                    }
                    if nu < up {
                        self.dom.tighten_upper(p, j, nu);
                    }
                    if !self.dom.propagate(p) {
                        conflict = true;
                        break;
                    }
                    if branched % 16 == 0 && rate(self) >= stop_rate {
                        break;
                    }
                }
                if branched == 0 && (inc.is_none() || fr < 0.25) {
                    // 締める列が尽きた: 分数の列を `getFixVal` で丸め幅の小さい順に固定する (丸め幅の和 0.5 まで)
                    let mut cands: Vec<(f64, usize, f64)> = order
                        .iter()
                        .filter(|&&j| self.dom.lo[j] < self.dom.up[j] && (x[j] - x[j].round()).abs() > FEASTOL)
                        .map(|&j| {
                            let f = fixval(j, x[j], self.dom.lo[j], self.dom.up[j]);
                            ((f - x[j]).abs(), j, f)
                        })
                        .collect();
                    cands.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                    let mut change = 0.0;
                    for (d, j, f) in cands {
                        if self.dom.lo[j] == self.dom.up[j] {
                            continue;
                        }
                        branch_pos.push(self.dom.stack_len());
                        branched += 1;
                        self.dom.tighten_lower(p, j, f);
                        self.dom.tighten_upper(p, j, f);
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
                    stages.pop();
                    break;
                }
                if conflict {
                    // 伝播の矛盾: 最後に締めた列だけ戻す (その段の残りは LP で確かめる)
                    let bp = branch_pos.pop().unwrap();
                    self.dom.backtrack_to(p, bp);
                    if branch_pos.len() == *stages.last().unwrap() {
                        stages.pop();
                        nback += 1;
                        if nback >= 10 {
                            break;
                        }
                        // この段で何も残らない: 幅を半分にして続ける
                        step *= 0.5;
                        continue;
                    }
                }
                need_lp = true;
            }
            if need_lp {
                // ダイブの最後の段の LP をまだ解いていない (時間・反復の上限): そのまま使う
                need_lp = false;
            }
            let fr = rate(self);
            let depth = branch_pos.len();
            let nfree = ints.iter().filter(|&&j| self.dom.lo[j] < self.dom.up[j]).count();
            if fr < 0.1 {
                count(4);
                if verbose && !self.params.submip {
                    eprintln!("MIP:   SUBFIX {label} try {}: fixing rate {fr:.2} below 0.1 (target {target:.2}, depth {depth}, {nback} backtracks); no sub-MIP", tries + 1);
                }
                break;
            }
            // 外側の根の RINS は、既定の `ENOMOTO_MIP_S4_RINSRATE` (0.5) と同じく固定率がそれ未満ならサブ MIP にしない
            // (1 回目の計測で cod105 の根の RINS が固定率 0.32 (一致する列が尽きた) で 87 秒回った。既定はこの判定で飛ばす)
            if inc.is_some() && self.nodes <= 1 {
                if let Some(r) = env_def!("ENOMOTO_MIP_S4_RINSRATE", "0.5").map(|v| v.parse::<f64>().unwrap_or(0.5)) {
                    if fr < r {
                        if verbose {
                            eprintln!("MIP:   SUBFIX {label} try {}: fixing rate {fr:.2} below {r:.2} (S4_RINSRATE); no sub-MIP", tries + 1);
                        }
                        break;
                    }
                }
            }
            let mut sub = p.clone();
            sub.col_lo = self.dom.lo.clone();
            sub.col_up = self.dom.up.clone();
            self.submip_fix_rate = Some(fr);
            let ts = std::time::Instant::now();
            count(1);
            // `n`: HiGHS のノード上限 (200 + 外側のノード数 / 20)
            let nl = if on('n') { 200 + self.nodes / 20 } else { 500 };
            // `p`: 並列で出すサブ MIP は、受け取りのときに根で実行不能ならやり直しの印を付ける ([`Solver::take_submip`])
            let keep_label = self.heur_label;
            if self.parallel_submips {
                self.heur_label = SUBFIX_PAR_LABEL;
            }
            let r = self.solve_submip_problem(sub, nl, None);
            self.heur_label = keep_label;
            self.submip_fix_rate = None;
            let nodes = self.last_submip_nodes;
            if verbose && !self.params.submip {
                eprintln!(
                    "MIP:   SUBFIX {label} try {}: target {target:.2}, fixing rate {fr:.2}, depth {depth}, {nback} backtracks, {nfree} free integer columns of {}, status {:?}, nodes {nodes}, accepted {}, {:.2}s (dive+sub {:.2}s)",
                    tries + 1,
                    ints.len(),
                    r.as_ref().map(|r| r.0),
                    r.as_ref().is_some_and(|r| r.1),
                    ts.elapsed().as_secs_f64(),
                    t0.elapsed().as_secs_f64()
                );
            }
            match r {
                Some((_, true)) => {
                    count(2);
                    found = true;
                    break;
                }
                // `q`: 目標が `ENOMOTO_T_MIP_SUBFIX_RETRY_MIN` (0.2) 未満になるやり直しはしない
                Some((MipStatus::Infeasible, false)) if nodes <= 1 && tries < max_tries && !self.time_up() && !(on('q') && 0.5 * fr < tunable!("ENOMOTO_T_MIP_SUBFIX_RETRY_MIN", 0.2, f64)) => {
                    // 根で実行不能: 目標を達した固定率の半分にして、深さの半分まで戻ってやり直す
                    let td = depth / 2;
                    if td <= 1 {
                        break;
                    }
                    count(3);
                    tries += 1;
                    target = 0.5 * fr;
                    self.dom.backtrack_to(p, branch_pos[td]);
                    branch_pos.truncate(td);
                    stages.retain(|&s| s < td);
                    step = 0.1;
                    need_lp = true;
                    continue 'retry;
                }
                _ => break,
            }
        }
        self.dom.backtrack_to(p, pos0);
        self.sync_lp();
        self.lp.restore_state(&saved);
        found
    }
}
