//! 求解の節目の時刻の記録 (論文用ベンチマークの計測用、`scripts/paper_bench/`)。
//!
//! LP の求解の開始 ([`start`]、前処理の直前) からの経過秒を、節目ごとに [`mark`] で記録する。
//! Python からは `_core.last_solve_events()` で直近の求解の記録を `[(名前, 秒), ...]` として
//! 読める。記録するのは求解 1 回あたり数回だけ (段階の切り替わりと結論の時点) なので、
//! 常に有効にしておいても求解時間には影響しない。
//!
//! 記録はプロセス全体で 1 つ: 複数のスレッドから同時に別の問題を解くと混ざる (連結成分の並列
//! 求解は同じ問題の中なので問題ない。どの成分の記録かは区別しない)。
//!
//! 節目の名前:
//!   - `presolve_end`: 前処理が終わり、単体法に渡す直前。
//!   - `presolve_infeasible` / `presolve_no_finite_optimum`: 前処理が結論を出した。
//!   - `stage_a_skipped`: crash がどの列も `M` 側に置かなかったので段階 A を飛ばした。
//!   - `stage_a_end`: 段階 A (傾き問題) が最適になり、`z^1 >= 0` (有限最適か実行不能) だった。
//!   - `stage_a_no_finite_optimum`: 段階 A の終わりで `z^1 < 0` (実行不能か非有界) を検出した。
//!   - `stage_b_infeasible`: 段階 B (または polish) が実行不能を結論した。
//!   - `finish_unbounded`: 段階 B の後の終了判定で非有界を結論した。
//!   - `polish_unbounded`: 仕上げ (polish) の主単体法への引き継ぎが非有界を結論した。
//!   - `trivial_unbounded`: 制約の無い成分で非有界を結論した。
//!   - `retry`: 数値的破綻から安全モードで最初から解き直した (以後の記録は解き直しのもの)。
//!   - `simplex_end`: 単体法 (全成分) が終わり、後処理の直前。

use std::sync::Mutex;
use std::time::Instant;

/// 求解の開始時刻と、それ以降に記録した節目 (名前, 開始からの秒)。
static STATE: Mutex<Option<(Instant, Vec<(&'static str, f64)>)>> = Mutex::new(None);

/// 記録を消して、計時の起点を今にする。
pub fn start() {
    if let Ok(mut s) = STATE.lock() {
        *s = Some((Instant::now(), Vec::new()));
    }
}

/// 節目 `name` を、起点からの経過秒とともに記録する ([`start`] 前なら何もしない)。
pub fn mark(name: &'static str) {
    if let Ok(mut s) = STATE.lock() {
        if let Some((t0, events)) = s.as_mut() {
            events.push((name, t0.elapsed().as_secs_f64()));
        }
    }
}

/// 直近の求解の記録の写し。
pub fn events() -> Vec<(&'static str, f64)> {
    STATE.lock().ok().and_then(|s| s.as_ref().map(|(_, e)| e.clone())).unwrap_or_default()
}
