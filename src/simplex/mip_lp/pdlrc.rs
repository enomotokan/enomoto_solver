//! 試作 `ENOMOTO_MIP_PDLRC` (analysis/pdlrc_20261111.md、既定では使わない): 根の LP (最初・再スタートの後) を解き始めるときに、
//! その LP の写しを別のスレッド (1 スレッドのプール) の PDLP で解き、確認点ごとに近似の双対 `y` からラグランジュ下界
//! `L(y) = b·y + Σ_j min_{l_j <= x_j <= u_j} (c - A^T y)_j x_j` と構造列の被約費用を作って、下界の一番良いものを控える
//! ([`PdlrcJob::best`])。MIP の側は根の終わり (再スタートの判断・木に入る前) にそれを被約費用による固定に使う (src/mip/xolazy.rs)。
//!
//! 正しさ: どの `y` でも `L(y)` は LP の下界 (`lagrange_std` がスラック列の符号を射影し、浮動小数の和の誤差の余裕を引く)。写しは
//! 根の LP を解き始めた時点の行と境界のもので、後で境界が締まり行 (カット) が増えても、緩い緩和の下界なのでそのまま成り立つ。

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// [`PdlrcJob`] が控える一番良い下界
#[derive(Clone)]
pub struct PdlrcBest {
    /// ラグランジュ下界 (元の列の単位の目的値、定数項 `offset` は含まない。余裕を引いた値)
    pub lb: f64,
    /// 構造列の被約費用 (最小化形)
    pub d: Vec<f64>,
    /// その双対を得た PDLP の反復と、始めからの秒
    pub iters: usize,
    pub secs: f64,
}

/// 裏で回る PDLP の仕事
pub struct PdlrcJob {
    pub best: Mutex<Option<PdlrcBest>>,
    /// 立てると PDLP が次の確認点で止まる
    pub stop: Arc<AtomicBool>,
    /// 確認点の数 (`L(y)` を計算した回数) と `L = -inf` だった回数
    pub checks: std::sync::atomic::AtomicUsize,
    pub neg_inf: std::sync::atomic::AtomicUsize,
}

impl PdlrcJob {
    /// 一番良い下界の写し
    pub fn best(&self) -> Option<PdlrcBest> {
        self.best.lock().ok().and_then(|b| b.clone())
    }
}

impl Drop for PdlrcJob {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

impl TwoStageLp {
    /// 今の LP の写しで PDLP を裏のスレッドで始める。PDLP は `ENOMOTO_T_MIP_PDLRC_SECS` (既定 30) 秒・`stop` まで回る。
    /// `L(y)` は `ENOMOTO_T_MIP_PDLRC_EVERY` (既定 1) 回目の確認点 (64 反復) ごとに計算する
    pub fn pdlrc_start(&self) -> Option<Arc<PdlrcJob>> {
        let std = Arc::new(self.std.clone());
        let n = self.n;
        let shift_c: f64 = (0..n).map(|j| self.std.c[j] * self.shift[j]).sum();
        let job = Arc::new(PdlrcJob {
            best: Mutex::new(None),
            stop: Arc::new(AtomicBool::new(false)),
            checks: Default::default(),
            neg_inf: Default::default(),
        });
        let secs = tunable!("ENOMOTO_T_MIP_PDLRC_SECS", 30.0, f64);
        let every = tunable!("ENOMOTO_T_MIP_PDLRC_EVERY", 1usize, usize).max(1);
        let jw = Arc::downgrade(&job);
        let stop = job.stop.clone();
        let pool = rayon::ThreadPoolBuilder::new().num_threads(1).thread_name(|i| format!("enomoto-mip-pdlrc-{i}")).build().ok()?;
        std::thread::spawn(move || {
            pool.install(|| {
                let t0 = std::time::Instant::now();
                let a = std.rows.to_faer();
                let mut k = 0usize;
                let std2 = std.clone();
                let hook: crate::interior_point::pdlp::CheckHook = Box::new(move |iters, y| {
                    k += 1;
                    if k % every != 0 {
                        return;
                    }
                    let Some(job) = jw.upgrade() else { return };
                    job.checks.fetch_add(1, Ordering::Relaxed);
                    match lagrange_std(&std2, n, y) {
                        Some((l, d)) if l.is_finite() => {
                            let lb = l + shift_c;
                            if let Ok(mut b) = job.best.lock() {
                                if b.as_ref().is_none_or(|b| lb > b.lb) {
                                    *b = Some(PdlrcBest { lb, d, iters, secs: t0.elapsed().as_secs_f64() });
                                }
                            }
                        }
                        _ => {
                            job.neg_inf.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
                let opts = crate::interior_point::pdlp::PdlpOptions { eps: 1e-9, max_iters: usize::MAX, time_limit: secs, max_restarts: usize::MAX };
                crate::cancel::with_token(Some(stop), || {
                    crate::interior_point::pdlp::with_check_hook(hook, || {
                        let _ = crate::interior_point::pdlp::solve_pdlp(&a, &std.b, &std.c, &std.lb, &std.ub, &opts);
                    })
                });
            });
        });
        Some(job)
    }
}
