//! 試作 `ENOMOTO_MIP_TREEFIX` (既定では使わない。analysis/treefix_20261111.md、docs/backlog.md の「木の中で暫定解が良くなったら、
//! 大域的な伝播・打ち切り値つきのプロービングで範囲を締める仕事を回す」): 木を分け合う探索で共有の最良値が下がったら、
//! ヒューリスティクス役のスレッド (番号の最も大きい従、[`super::heur_role_count`] が 1 以上のとき) が、大近傍探索の合間か
//! 木の潜りの切れ目 (潜りの子がないとき) に、根の定義域で大域的な範囲を締める仕事を 1 回だけ回す。専用のスレッドは立てない。
//!
//! 文字で部品を選ぶ (`1` は `pcur`):
//! - `p`: 大域的な範囲での伝播 (行と、打ち切り値による目的関数の行。[`Solver::global_obj_prop`] の本数を問わない形)。
//!   主の根の被約費用による固定 (`root_redcost_fixing`) で締まった列から、制約を通して他の列を締める。
//! - `c`: 根の定義域 (締めた大域的な範囲) とカットの行で LP を解き直し、その被約費用で締める。
//! - `u`: 打ち切り値つきの LP のプロービング (GAP1 の `u`、[`Solver::lp_probe_cutoff`]) を手間の上限つきで回す。
//! - `r`: 根のカットの各ラウンドの LP の被約費用も主の被約費用による固定に使う (`ENOMOTO_MIP_TREE4_RCUNION` と同じ。主が暫定解の
//!   改善のたびに当てる)。
//! - `R`: 仕事で新たに固定した整数列があれば、主に木の中の再スタートの判定を頼む (`ENOMOTO_MIP_TREE_RESTART=inc` と同じ判定、
//!   ただし主のノード数の上限は見ない。固定の割合 `ENOMOTO_T_MIP_TREE_RESTART_FRAC` (0.05) 以上で再スタート)。
//!
//! 締めた範囲は自分の大域的な範囲と共有の範囲 ([`Solver::sync_global_bounds`]) に入れ、ほかのスレッドは 64 ノードおきの突き合わせ
//! (決まった点) で取り込む。仕事の時間の合計は、木の始まりからの経過時間の `ENOMOTO_T_MIP_TREEFIX_FRAC` (0.2) 倍まで
//! (1 スレッドの時間に対する割合)。LP のプロービングは 1 回の仕事で `ENOMOTO_T_MIP_TREEFIX_PROBE_SECS` 秒 (既定は時間上限の 0.02 倍) まで、
//! 相対ギャップが `ENOMOTO_T_MIP_TREEFIX_PROBE_MAX_GAP` (0.01) を越えれば試さない。
//! 根の伝播・LP で矛盾した (打ち切り値より良い解がない) ときは、主が次の木のループの頭で最適として終える。

use super::{Solver, WorkPool};
use crate::mip::lp::{LpStatus, VarStatus};
use crate::mip::lp_api::MipLp;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// `ENOMOTO_MIP_TREEFIX` の値 (`1` は `pcur`)
fn flags() -> &'static str {
    static V: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    V.get_or_init(|| match env_str!("ENOMOTO_MIP_TREEFIX") {
        Some("1") => "pcur".to_string(),
        Some(v) if v != "0" && v != "off" => v.to_string(),
        _ => String::new(),
    })
}

/// `ENOMOTO_MIP_TREEFIX` に文字 `c` が入っているか
pub(super) fn on(c: char) -> bool {
    flags().contains(c)
}

/// 仕事を回す部品 (`p`・`c`・`u`) のどれかが入っているか
fn job_on() -> bool {
    on('p') || on('c') || on('u')
}

/// 統計の添字
const JOBS: usize = 0;
const PROP: usize = 1;
const RC: usize = 2;
const PROBED: usize = 3;
const PROBE_LPS: usize = 4;
const PROBE_FIX: usize = 5;
const NEW_FIX: usize = 6;
const TIGHT: usize = 7;
const DEAD: usize = 8;
const RESTART_REQ: usize = 9;
const CLOSED: usize = 10;
const LP_FAIL: usize = 11;
const BUDGET_SKIP: usize = 12;
const NSTAT: usize = 13;

#[derive(Default)]
struct State {
    /// 最後に仕事を回した (または木に入って最初に見た) 共有の最良値
    seen: Option<f64>,
    /// 仕事に使った秒の合計
    secs: f64,
    /// 根で矛盾した仕事の打ち切り値 (主が見て終える)
    closed: Option<f64>,
    /// 予算で見送った最良値 (同じ値で何度も数えない)
    skipped_at: Option<f64>,
}

/// 木を分け合うスレッドで共有する状態 ([`WorkPool`] に持たせる)
#[derive(Default)]
pub(super) struct Shared {
    st: Mutex<State>,
    /// 締めた仕事の数 (世代)。木のスレッドは変わったときだけ待ち行列の死んだノードを数える
    gen: AtomicU64,
    /// 主に木の中の再スタートの判定を頼む合図 (`R`)
    restart_req: AtomicBool,
    stats: [AtomicU64; NSTAT],
}

impl Shared {
    fn add(&self, k: usize, v: usize) {
        if v > 0 {
            self.stats[k].fetch_add(v as u64, Ordering::Relaxed);
        }
    }
}

thread_local! {
    /// 木のスレッドが最後に死んだノードを数えた世代 (共有の状態の番地と組)
    static SEEN_GEN: std::cell::Cell<(usize, u64)> = const { std::cell::Cell::new((0, 0)) };
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// このスレッドが今、仕事を回すか: 木を分け合う従のうち番号の最も大きいもの (ヒューリスティクス役を受け持つスレッド) で、
    /// 共有の最良値が前に見た値より下がっていて、時間の予算が残っているとき。木に入って最初に見た値では回さない。
    pub(super) fn treefix_due(&mut self) -> bool {
        if !job_on() || self.params.submip || self.snap.is_none() || !self.race_done {
            return false;
        }
        let Some(pool) = self.pool.clone() else { return false };
        if pool.heur_k == 0 || self.pool_tid as usize + 1 != pool.threads {
            return false;
        }
        let best = self.shared.as_ref().map_or(f64::INFINITY, |s| s.best());
        if !best.is_finite() || self.time_up() {
            return false;
        }
        let mut st = pool.treefix.st.lock().unwrap();
        match st.seen {
            None => {
                st.seen = Some(best);
                false
            }
            Some(s) if best < s - 1e-9 * s.abs().max(1.0) => {
                let el = pool.t0.elapsed().as_secs_f64();
                if st.secs > tunable!("ENOMOTO_T_MIP_TREEFIX_FRAC", 0.2, f64) * el {
                    if st.skipped_at != Some(best) {
                        st.skipped_at = Some(best);
                        pool.treefix.add(BUDGET_SKIP, 1);
                    }
                    return false;
                }
                true
            }
            _ => false,
        }
    }

    /// 仕事を 1 回回す ([`Self::treefix_due`] が真のとき。潜りの子がないとき・大近傍探索の合間に呼ぶ)。定義域は根に戻し、LP は根の
    /// 範囲に合わせて返す (潜りの兄弟の一覧は捨てる。兄弟は待ち行列に残る)。
    pub(super) fn treefix_job(&mut self) {
        let p = self.p;
        let Some(pool) = self.pool.clone() else { return };
        let t0 = Instant::now();
        let best = self.shared.as_ref().map_or(f64::INFINITY, |s| s.best());
        let lim = self.prune_limit();
        if !lim.is_finite() {
            return;
        }
        let tf = &pool.treefix;
        tf.add(JOBS, 1);
        self.dive_stack.clear();
        self.dom.reset_to_root(p);
        // 主の被約費用による固定などを先に取り込む
        self.sync_global_bounds();
        let (lo0, up0) = (self.dom.global_lo.clone(), self.dom.global_up.clone());
        let mut closed = self.dom.infeasible;
        let (mut prop, mut rc, mut probe) = (0usize, 0usize, [0usize; 3]);
        if !closed && on('p') {
            let r = self.global_obj_prop(true, "TREEFIX p");
            prop = r[0];
            closed = r[3] == 1;
        }
        if !closed && (on('c') || on('u')) {
            self.dom.reset_to_root(p);
            if self.dom.infeasible || !self.dom.propagate(p) {
                closed = true;
            } else {
                self.sync_lp();
                let it0 = self.lp.total_iterations();
                let st = self.lp.solve(&self.limits(tunable!("ENOMOTO_T_MIP_TREEFIX_ROOT_ITERS", 50000u64, u64)));
                let root_its = self.lp.total_iterations() - it0;
                match st {
                    LpStatus::Optimal => {
                        if on('c') {
                            rc = self.treefix_redcost(lim);
                        }
                        if on('u') && !self.time_up() {
                            let tl = if self.params.time_limit.is_finite() { self.params.time_limit } else { 300.0 };
                            let tmax = tunable!("ENOMOTO_T_MIP_TREEFIX_PROBE_SECS", 0.02 * tl, f64);
                            let max_gap = tunable!("ENOMOTO_T_MIP_TREEFIX_PROBE_MAX_GAP", 0.01, f64);
                            let itlim = (tunable!("ENOMOTO_T_MIP_TREEFIX_PROBE_ITERMULT", 0.5, f64) * root_its as f64).max(100.0) as u64;
                            probe = self.lp_probe_cutoff(tmax, max_gap, itlim, "TREEFIX u");
                            closed = self.dom.infeasible;
                        }
                    }
                    LpStatus::Infeasible | LpStatus::ObjectiveBound => closed = true,
                    _ => tf.add(LP_FAIL, 1),
                }
            }
        }
        if !closed {
            // 根の伝播 (締めた大域的な範囲から) で締まった整数列の範囲も大域的にする
            for j in 0..p.n {
                if !p.is_int[j] {
                    continue;
                }
                if self.dom.lo[j] > self.dom.global_lo[j] {
                    self.dom.tighten_global(p, j, false, self.dom.lo[j]);
                }
                if self.dom.up[j] < self.dom.global_up[j] {
                    self.dom.tighten_global(p, j, true, self.dom.up[j]);
                }
            }
        }
        let (mut tight, mut newfix) = (0usize, 0usize);
        for j in 0..p.n {
            if !p.is_int[j] {
                continue;
            }
            let (gl, gu) = (self.dom.global_lo[j], self.dom.global_up[j]);
            if gl > lo0[j] || gu < up0[j] {
                tight += 1;
                if gl >= gu && lo0[j] < up0[j] {
                    newfix += 1;
                }
            }
        }
        // 締めた範囲を共有の範囲に入れる (ほかのスレッドは 64 ノードおきの突き合わせで取り込む)
        if tight > 0 && !closed {
            self.sync_global_bounds();
            tf.gen.fetch_add(1, Ordering::Relaxed);
        }
        self.dom.reset_to_root(p);
        self.sync_lp();
        tf.add(PROP, prop);
        tf.add(RC, rc);
        tf.add(PROBED, probe[0]);
        tf.add(PROBE_LPS, probe[1]);
        tf.add(PROBE_FIX, probe[2]);
        tf.add(TIGHT, tight);
        tf.add(NEW_FIX, newfix);
        if on('R') && newfix > 0 && !closed {
            tf.restart_req.store(true, Ordering::Relaxed);
            tf.add(RESTART_REQ, 1);
        }
        let dt = t0.elapsed().as_secs_f64();
        {
            let mut st = tf.st.lock().unwrap();
            st.seen = Some(best);
            st.secs += dt;
            if closed {
                st.closed = Some(st.closed.map_or(lim, |c| c.min(lim)));
            }
        }
        if closed {
            tf.add(CLOSED, 1);
        }
        if self.params.verbose || env_str!("ENOMOTO_MIP_TREEFIX_LOG").is_some() {
            eprintln!(
                "TREEFIX: job at {:.2}s (tid {}, node {}): cutoff {lim:.10e}, propagation {prop}, reduced costs {rc}, probing {probe:?} (columns, LPs, fixed), integer columns tightened {tight} (newly fixed {newfix}), closed {closed} ({dt:.3}s)",
                pool.t0.elapsed().as_secs_f64(),
                self.pool_tid,
                self.nodes
            );
        }
    }

    /// 解き直した根の LP (最適、打ち切り値 `lim`) の被約費用で整数列を大域的に締める。締めた列の数を返す。
    fn treefix_redcost(&mut self, lim: f64) -> usize {
        let p = self.p;
        let z = self.lp.objective() + p.offset;
        let gap = lim - z;
        if !(gap.is_finite() && gap >= 0.0) {
            return 0;
        }
        let list: Vec<(usize, bool, f64, f64)> = {
            let b = self.lp.basis_ref();
            let d = self.lp.reduced_costs_ref();
            (0..p.n)
                .filter(|&j| p.is_int[j] && self.dom.lo[j] < self.dom.up[j])
                .filter_map(|j| match b.col[j] {
                    VarStatus::Lower if d[j] > 1e-7 => Some((j, true, d[j], self.dom.lo[j])),
                    VarStatus::Upper if d[j] < -1e-7 => Some((j, false, d[j], self.dom.up[j])),
                    _ => None,
                })
                .collect()
        };
        let mut k = 0;
        for (j, at_lower, d, bound) in list {
            let (gl, gu) = (self.dom.global_lo[j], self.dom.global_up[j]);
            if at_lower {
                self.dom.tighten_global(p, j, true, bound + gap / d);
            } else {
                self.dom.tighten_global(p, j, false, bound - gap / (-d));
            }
            k += (self.dom.global_lo[j] > gl || self.dom.global_up[j] < gu) as usize;
        }
        k
    }

    /// 木のスレッド: 仕事の世代が変わっていれば、手元の待ち行列で大域的な範囲に反する分枝を持つ (取り出せばすぐ刈られる) ノードを
    /// 数える ([`Self::sync_global_bounds`] の後に呼ぶ。数えるだけ)。
    pub(super) fn treefix_count_dead(&self) {
        if !job_on() {
            return;
        }
        let Some(pool) = self.pool.as_ref() else { return };
        let key = std::sync::Arc::as_ptr(pool) as usize;
        let g = pool.treefix.gen.load(Ordering::Relaxed);
        let (k0, g0) = SEEN_GEN.with(|c| c.get());
        if k0 == key && g0 == g {
            return;
        }
        SEEN_GEN.with(|c| c.set((key, g)));
        if g == 0 {
            return;
        }
        let (lo, up) = (&self.dom.global_lo, &self.dom.global_up);
        let dead = self
            .queue
            .iter()
            .filter(|n| n.changes.iter().any(|c| if c.upper { c.value < lo[c.col] - 1e-6 } else { c.value > up[c.col] + 1e-6 }))
            .count();
        pool.treefix.add(DEAD, dead);
    }

    /// 主: 仕事が根で矛盾を見つけていて、自分の打ち切り値がその打ち切り値以下なら、暫定解の値を返す (最適として終える)。
    /// `R` の合図があれば、木の中の再スタートの判定 (`tree_restart_check`) を立てる。
    pub(super) fn treefix_main_tick(&mut self) -> Option<f64> {
        if !job_on() || self.pool_tid != 0 || self.snap.is_some() || self.params.submip {
            return None;
        }
        let pool = self.pool.clone()?;
        if on('R') && pool.treefix.restart_req.swap(false, Ordering::Relaxed) {
            self.sync_global_bounds();
            self.tree_restart_check = true;
        }
        let c = pool.treefix.st.lock().unwrap().closed?;
        let z = self.incumbent.as_ref().map(|(z, _)| *z)?;
        if self.prune_limit() <= c + 1e-9 * c.abs().max(1.0) {
            if self.params.verbose || env_str!("ENOMOTO_MIP_TREEFIX_LOG").is_some() {
                eprintln!("TREEFIX: the root under the cutoff {c:.10e} is infeasible; the incumbent {z:.10e} is optimal");
            }
            return Some(z);
        }
        None
    }

    /// 主: `R` の合図で立てた再スタートの判定か (主のノード数の上限を見ない)
    pub(super) fn treefix_restart_mode(&self) -> bool {
        on('R') && self.pool.is_some()
    }

    /// 終わりの集計の行 (`TREEFIX:`)。主が求解の終わりに出す
    pub(super) fn treefix_summary(&self, pool: &WorkPool) {
        if !job_on() && !on('r') {
            return;
        }
        let s: Vec<u64> = pool.treefix.stats.iter().map(|a| a.load(Ordering::Relaxed)).collect();
        let secs = pool.treefix.st.lock().unwrap().secs;
        eprintln!(
            "TREEFIX: summary jobs {} ({secs:.2}s, skipped by the budget {}), propagation {}, reduced costs {}, probing columns {} LPs {} fixed {}, integer columns tightened {} (newly fixed {}), dead open nodes {}, restart requests {}, closed {}, root LP not solved {}",
            s[JOBS], s[BUDGET_SKIP], s[PROP], s[RC], s[PROBED], s[PROBE_LPS], s[PROBE_FIX], s[TIGHT], s[NEW_FIX], s[DEAD], s[RESTART_REQ], s[CLOSED], s[LP_FAIL]
        );
    }
}
