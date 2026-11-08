//! 診断 `ENOMOTO_MIP_DEBUG_PRETIME` (analysis/prepar_20261107.md): 前処理の段ごとの秒を数える。値が `stop` なら根の前
//! (`solver::solve` を呼ぶ直前) で表を出してプロセスを終える。ほかの値なら表を出して続ける。数えるのは最初の求解
//! (サブ MIP・再スタートではない) を呼んだスレッドだけ (スレッドごとの表)。かけ直し (入れ子の前処理) の中の秒は
//! 段の名前に `@re` を付けて別に数える。

use std::cell::{Cell, RefCell};
use std::time::Instant;

thread_local! {
    static ON: Cell<bool> = const { Cell::new(false) };
    static T0: Cell<Option<Instant>> = const { Cell::new(None) };
    static NEST: Cell<u32> = const { Cell::new(0) };
    static TAB: RefCell<Vec<(String, f64, u32)>> = const { RefCell::new(Vec::new()) };
    static MARKS: RefCell<Vec<(String, f64)>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn env_on() -> bool {
    env_str!("ENOMOTO_MIP_DEBUG_PRETIME").is_some()
}

/// 最初の求解の前処理の始めに呼ぶ (表を空にして数え始める)
pub(crate) fn start() {
    if !env_on() {
        return;
    }
    ON.with(|o| o.set(true));
    T0.with(|t| t.set(Some(Instant::now())));
    NEST.with(|n| n.set(0));
    TAB.with(|t| t.borrow_mut().clear());
    MARKS.with(|t| t.borrow_mut().clear());
}

pub(crate) fn add(key: &str, secs: f64) {
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
        } else {
            t.push((k, secs, 1));
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
pub(crate) struct Guard(&'static str, Instant);
impl Drop for Guard {
    fn drop(&mut self) {
        add(self.0, self.1.elapsed().as_secs_f64());
    }
}
pub(crate) fn guard(key: &'static str) -> Guard {
    Guard(key, Instant::now())
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
    let mut s = format!("PRETIME total={total:.3} m={m} n={n} nnz={nnz} free={free}");
    TAB.with(|t| {
        for (k, v, c) in t.borrow().iter() {
            s.push_str(&format!(" {k}={v:.3}/{c}"));
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
