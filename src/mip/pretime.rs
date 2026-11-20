//! 診断 `ENOMOTO_MIP_DEBUG_PRETIME` (analysis/prepar_20261107.md): 前処理の段ごとの秒を数える。値が `stop` なら根の前
//! (`solver::solve` を呼ぶ直前) で表を出してプロセスを終える。ほかの値なら表を出して続ける。数えるのは最初の求解
//! (サブ MIP・再スタートではない) を呼んだスレッドだけ (スレッドごとの表)。かけ直し (入れ子の前処理) の中の秒は
//! 段の名前に `@re` を付けて別に数える。
//! 段ごとに、プロセス全体の CPU 時間 (全スレッドの和) の増えも数え、`段=秒/回/コア` (コア = CPU 時間 / 経過時間、4 スレッドで
//! 1 本だけ働いていれば 1.0) を出す (analysis/objtarget_20261119/ の 4、前処理の間の空きコア)。段は入れ子になりうる (外側の段は内側を含む)。

use std::cell::{Cell, RefCell};
use std::time::Instant;

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    static T0: Cell<Option<Instant>> = const { Cell::new(None) };
    static NEST: Cell<u32> = const { Cell::new(0) };
    static TAB: RefCell<Vec<(String, f64, u32, f64)>> = const { RefCell::new(Vec::new()) };
    static CPU0: Cell<f64> = const { Cell::new(0.0) };
    static MARKS: RefCell<Vec<(String, f64)>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn env_on() -> bool {
    env_str!("ENOMOTO_MIP_DEBUG_PRETIME").is_some()
}

/// 診断 `ENOMOTO_MIP_DEBUG_PRETIME_RESTART`: 再スタートの後の前処理の表も出す (analysis/objtarget_20261119/ の 10 節)
pub(crate) fn restart_on() -> bool {
    env_on() && env_str!("ENOMOTO_MIP_DEBUG_PRETIME_RESTART").is_some()
}

/// 最初の求解の前処理の始めに呼ぶ (表を空にして数え始める)
pub(crate) fn start() {
    if !env_on() {
        return;
    }
    ON.with(|o| o.set(true));
    T0.with(|t| t.set(Some(Instant::now())));
    CPU0.with(|c| c.set(process_cpu()));
    NEST.with(|n| n.set(0));
    TAB.with(|t| t.borrow_mut().clear());
    MARKS.with(|t| t.borrow_mut().clear());
}

/// プロセス全体の CPU 時間 (秒、全スレッドの和。Linux 以外は 0)
pub(crate) fn process_cpu() -> f64 {
    #[cfg(target_os = "linux")]
    {
        #[repr(C)]
        struct Timespec {
            tv_sec: i64,
            tv_nsec: i64,
        }
        extern "C" {
            fn clock_gettime(clk: i32, tp: *mut Timespec) -> i32;
        }
        const CLOCK_PROCESS_CPUTIME_ID: i32 = 2;
        let mut ts = Timespec { tv_sec: 0, tv_nsec: 0 };
        // SAFETY: `ts` は有効な書き込み先で、clock_gettime はそこに書くだけ
        if unsafe { clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &mut ts) } == 0 {
            return ts.tv_sec as f64 + ts.tv_nsec as f64 * 1e-9;
        }
    }
    0.0
}

pub(crate) fn add(key: &str, secs: f64) {
    add_cpu(key, secs, 0.0);
}

fn add_cpu(key: &str, secs: f64, cpu: f64) {
    if !ON.with(|o| o.get()) {
        return;
    }
    let re = NEST.with(|n| n.get()) > 1;
    let k = if re { format!("{key}@re") } else { key.to_string() };
    TAB.with(|t| {
        let mut t = t.borrow_mut();
        if let Some(e) = t.iter_mut().find(|e| e.0 == k) {
            e.1 += secs;
            e.2 += 1;
            e.3 += cpu;
        } else {
            t.push((k, secs, 1, cpu));
        }
    });
}

/// 時刻の印 (前処理の始めからの秒)
pub(crate) fn mark(key: &str) {
    if !ON.with(|o| o.get()) {
        return;
    }
    let s = T0.with(|t| t.get()).map_or(0.0, |t| t.elapsed().as_secs_f64());
    MARKS.with(|m| m.borrow_mut().push((key.to_string(), s)));
}

/// 区間を測る (落とすときに足す)
pub(crate) struct Guard(&'static str, Instant, f64);
impl Drop for Guard {
    fn drop(&mut self) {
        // 試作 `ENOMOTO_MIP_TPRE` の `t`: 前処理の段の終わりごとに、並列の FJ・Local-MIP に主の手間まで許す
        super::tpre::pt_grant();
        super::tpre::gated_grant();
        if ON.with(|o| o.get()) {
            add_cpu(self.0, self.1.elapsed().as_secs_f64(), process_cpu() - self.2);
        }
    }
}
pub(crate) fn guard(key: &'static str) -> Guard {
    Guard(key, Instant::now(), if ON.with(|o| o.get()) { process_cpu() } else { 0.0 })
}

/// 前処理の入れ子 (presolve_mip_depth の呼び出し) を数える
pub(crate) struct Nest;
impl Drop for Nest {
    fn drop(&mut self) {
        NEST.with(|n| n.set(n.get().saturating_sub(1)));
    }
}
pub(crate) fn nest() -> Nest {
    NEST.with(|n| n.set(n.get() + 1));
    Nest
}

/// 表を出す (`stop` ならプロセスを終える)。`m`・`n`・`nnz`・`free`: 前処理の後の問題の大きさ
pub(crate) fn report(m: usize, n: usize, nnz: usize, free: usize) {
    if !ON.with(|o| o.replace(false)) {
        return;
    }
    let total = T0.with(|t| t.get()).map_or(0.0, |t| t.elapsed().as_secs_f64());
    let cpu = process_cpu() - CPU0.with(|c| c.get());
    let mut s = format!("PRETIME total={total:.3} cores={:.2} m={m} n={n} nnz={nnz} free={free}", if total > 0.0 { cpu / total } else { 0.0 });
    TAB.with(|t| {
        for (k, v, c, u) in t.borrow().iter() {
            if *u > 0.0 && *v > 0.0 {
                s.push_str(&format!(" {k}={v:.3}/{c}/{:.2}", u / v));
            } else {
                s.push_str(&format!(" {k}={v:.3}/{c}"));
            }
        }
    });
    MARKS.with(|t| {
        for (k, v) in t.borrow().iter() {
            s.push_str(&format!(" @{k}={v:.3}"));
        }
    });
    eprintln!("{s}");
    if env_str!("ENOMOTO_MIP_DEBUG_PRETIME") == Some("stop") {
        std::process::exit(0);
    }
}
