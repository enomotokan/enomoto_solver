//! 試作 `ENOMOTO_MIP_ROOTLP3=3` (既定では使わない、analysis/rootlp3_20261111.md 段階 2): 基底がある根の LP を、基底からの単体法
//! (warm)・全スラック基底からの二段解法 (cold)・内点法 + クロスオーバー (ipm) の 3 本で同時に解き、最初に結論を出したものを採る。
//! warm・cold は 1 スレッドずつ、内点法は残りの `threads - 2` スレッド。下界 ([`crate::cancel::SharedBound`]) は 3 本で共有する。
//! cold と ipm は `ENOMOTO_T_MIP_ROOTLP3_HEAD` 秒 (既定 0) 遅れて始める (warm に先行時間を与える)。

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

enum Msg {
    Simplex(&'static str, LpStatus, Box<TwoStageLp>, u8),
    Ipm(Option<RaceWon>),
}

thread_local! {
    /// 直近の [`TwoStageLp::solve_race3`] で勝った側 (`warm`・`cold`・`ipm`、どれも結論を出さなければ `none`)
    static WINNER: std::cell::Cell<&'static str> = const { std::cell::Cell::new("") };
}

/// 直近の 3 本同時で勝った側を取り出す (なければ空)。
pub(crate) fn take_race3_winner() -> &'static str {
    WINNER.with(|w| w.replace(""))
}

/// `head` 秒待つ (`cancel` が立てばすぐ戻る)。待ち終えたら真、止められたら偽。
fn head_wait(head: f64, cancel: &AtomicBool) -> bool {
    let end = std::time::Instant::now() + std::time::Duration::from_secs_f64(head.max(0.0));
    while std::time::Instant::now() < end {
        if cancel.load(Ordering::SeqCst) {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    !cancel.load(Ordering::SeqCst)
}

impl TwoStageLp {
    /// 3 本同時に解く (このモジュールの説明)。基底がない・`threads` < 3 なら [`Self::solve`]。
    pub fn solve_race3(&mut self, lim: &SolveLimits, threads: usize) -> LpStatus {
        if threads < 3 || !self.has_warm_basis() || crate::cancel::current().is_some() {
            return self.solve(lim);
        }
        let build = |k: usize, name: &'static str| rayon::ThreadPoolBuilder::new().num_threads(k).thread_name(move |i| format!("enomoto-mip-r3{name}-{i}")).build();
        let (Ok(pool_w), Ok(pool_c), Ok(pool_i)) = (build(1, "warm"), build(1, "cold"), build(threads - 2, "ipm")) else {
            return self.solve(lim);
        };
        let debug = env_str!("ENOMOTO_DEBUG_RACE").is_some();
        let head = tunable!("ENOMOTO_T_MIP_ROOTLP3_HEAD", 0.0, f64);
        let t0 = std::time::Instant::now();
        let cancel = Arc::new(AtomicBool::new(false));
        let bound = crate::cancel::SharedBound::new(self.std.n_total, self.std.n_rows);
        let (tx, rx) = mpsc::channel::<Msg>();
        // 内点法 + クロスオーバー
        {
            let (std, cancel, bound, tx) = (Arc::new(self.std.clone()), cancel.clone(), bound.clone(), tx.clone());
            let sc = self.ipm_scale();
            let deadline = lim.deadline;
            std::thread::spawn(move || {
                if !head_wait(head, &cancel) {
                    let _ = tx.send(Msg::Ipm(None));
                    return;
                }
                let r = pool_i.install(|| {
                    sid::request_duals(true);
                    let r = crate::cancel::with_deadline(deadline, || {
                        crate::cancel::with_token(Some(cancel.clone()), || crate::cancel::with_bound(Some(bound), || ipm_crossover_run(&std, sc.as_deref())))
                    });
                    let duals = sid::take_duals();
                    sid::request_duals(false);
                    r.filter(|r| r.status == Status::Optimal).and_then(|r| r.x).map(|x| (x, duals))
                });
                let _ = tx.send(Msg::Ipm(if cancel.load(Ordering::SeqCst) { None } else { r }));
            });
        }
        // 単体法 2 本 (warm はこの LP の写し、cold は基底を捨てた写し)
        for (kind, pool) in [("warm", pool_w), ("cold", pool_c)] {
            let mut lp = Box::new(self.clone());
            if kind == "cold" {
                lp.drop_basis();
            }
            let (cancel, bound, tx, lim) = (cancel.clone(), bound.clone(), tx.clone(), *lim);
            let flags = sid::tl_flags();
            std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn(move || {
                    if kind == "cold" && !head_wait(head, &cancel) {
                        let _ = tx.send(Msg::Simplex(kind, LpStatus::IterationLimit, lp, flags));
                        return;
                    }
                    let (st, flags) = pool.install(|| {
                        sid::set_tl_flags(flags);
                        let st = crate::cancel::with_token(Some(cancel.clone()), || crate::cancel::with_bound(Some(bound), || lp.solve_once(&lim)));
                        (st, sid::tl_flags())
                    });
                    let _ = tx.send(Msg::Simplex(kind, st, lp, flags));
                })
                .expect("spawn");
        }
        drop(tx);
        let recv = |rx: &mpsc::Receiver<Msg>| match lim.deadline {
            Some(d) => rx.recv_timeout(d.saturating_duration_since(std::time::Instant::now())).ok(),
            None => rx.recv().ok(),
        };
        let mut failed = 0;
        loop {
            let Some(msg) = recv(&rx) else {
                cancel.store(true, Ordering::SeqCst);
                if failed >= 3 || lim.deadline.is_none() {
                    break;
                }
                WINNER.with(|w| w.set("none"));
                return LpStatus::TimeLimit;
            };
            let secs = t0.elapsed().as_secs_f64();
            match msg {
                Msg::Simplex(kind, st, lp, flags) => {
                    let conclusive = matches!(st, LpStatus::Optimal | LpStatus::Infeasible | LpStatus::Unbounded | LpStatus::ObjectiveBound | LpStatus::TimeLimit);
                    if !conclusive || cancel.swap(true, Ordering::SeqCst) {
                        failed += 1;
                        if failed >= 3 {
                            break;
                        }
                        continue;
                    }
                    if debug {
                        eprintln!("MIP root race3: {kind} simplex won at {secs:.3}s ({st:?})");
                    }
                    *self = *lp;
                    sid::set_tl_flags(flags);
                    WINNER.with(|w| w.set(kind));
                    user_log!("Root LP: the {kind} simplex finished first ({secs:.2}s)");
                    return st;
                }
                Msg::Ipm(None) => {
                    failed += 1;
                    if failed >= 3 {
                        break;
                    }
                }
                Msg::Ipm(Some((x, duals))) => {
                    if cancel.swap(true, Ordering::SeqCst) {
                        failed += 1;
                        if failed >= 3 {
                            break;
                        }
                        continue;
                    }
                    if debug {
                        eprintln!("MIP root race3: ipm_crossover won at {secs:.3}s");
                    }
                    WINNER.with(|w| w.set("ipm"));
                    self.scale_once = false;
                    user_log!("Root LP: the interior-point method + crossover finished first ({secs:.2}s)");
                    let st = self.adopt_ipm_vertex(&x, duals, lim, debug, t0);
                    self.status = Some(st);
                    return st;
                }
            }
        }
        // どれも結論を出さなかった: 今まで通り 1 本で解く
        WINNER.with(|w| w.set("none"));
        self.solve_once(lim)
    }
}
