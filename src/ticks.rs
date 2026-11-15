//! 手間の単位 (決定的な時計、analysis/timedep_20261114.md の 11 節)。
//!
//! スレッドごとに、部分 (LP・伝播・分離・前処理・ヒューリスティクス・ノード) 別の手間を数える。数えは常に行う (スレッドローカルの
//! 足し算だけで、各部分の外側のループで 1 回)。重み付きの合計 [`total`] を、秒の代わりの上限 (手間の期限 [`with_deadline`]) に使う。
//! 重みは `ENOMOTO_T_TICKS_W_<部分>` (既定は時間の規則のログから当てはめた値で、合計がおよそ CPU のナノ秒になる)。
//! 期限は既定ではない (`u64::MAX`) ので、既定の道筋は変わらない。

use std::cell::Cell;

pub const LP: usize = 0;
pub const PROP: usize = 1;
pub const SEP: usize = 2;
pub const PRE: usize = 3;
pub const HEUR: usize = 4;
pub const NODE: usize = 5;
/// LP の 1 反復ごとに 1 (反復の決まった手間の分)
pub const LPIT: usize = 6;
/// LU の分解・求解・更新 (`FtLu` の tick と同じ数え)
pub const LU: usize = 7;
pub const NK: usize = 8;
pub const NAMES: [&str; NK] = ["lp", "prop", "sep", "pre", "heur", "node", "lpit", "lu"];

thread_local! {
    static RAW: [Cell<u64>; NK] = const { [const { Cell::new(0) }; NK] };
    static TOTAL: Cell<f64> = const { Cell::new(0.0) };
    static DEADLINE: Cell<f64> = const { Cell::new(f64::INFINITY) };
}

fn weights() -> &'static [f64; NK] {
    static W: std::sync::OnceLock<[f64; NK]> = std::sync::OnceLock::new();
    W.get_or_init(|| {
        // 既定は 58 問の 1 スレッドの時間の規則のログ (analysis/timedep_20261114/tool/ticks_fit.py、cal4) で当てはめた重み x 1e9 (1 手間 ≒ 1 ns)
        let mut w = [3.48, 5.55, 31.7, 84.0, 16.6, 235.0, 9063.0, 2.11];
        for (k, n) in NAMES.iter().enumerate() {
            if let Ok(v) = std::env::var(format!("ENOMOTO_T_TICKS_W_{}", n.to_uppercase())) {
                if let Ok(x) = v.parse::<f64>() {
                    w[k] = x;
                }
            }
        }
        w
    })
}

/// 部分 `k` の手間を `n` 足す。
#[inline]
pub fn add(k: usize, n: u64) {
    RAW.with(|r| r[k].set(r[k].get() + n));
    TOTAL.with(|t| t.set(t.get() + weights()[k] * n as f64));
}

/// このスレッドの重み付きの手間の合計。
#[inline]
pub fn total() -> f64 {
    TOTAL.with(|t| t.get())
}

/// このスレッドの部分ごとの手間 (診断用)。
pub fn raw() -> [u64; NK] {
    RAW.with(|r| std::array::from_fn(|k| r[k].get()))
}

/// このスレッドの手間の期限 (重み付きの合計) を過ぎたか (期限がなければ偽)。
#[inline]
pub fn over() -> bool {
    total() >= DEADLINE.with(|d| d.get())
}

/// 今から `budget` (重み付き) を手間の期限にして `f` を実行し、元に戻す (今の期限より早いときだけ縮める)。
pub fn with_budget<R>(budget: f64, f: impl FnOnce() -> R) -> R {
    let prev = DEADLINE.with(|d| d.get());
    DEADLINE.with(|d| d.set(prev.min(total() + budget)));
    let r = f();
    DEADLINE.with(|d| d.set(prev));
    r
}

/// 今から `budget` (重み付き) を手間の期限にする (今の期限より早いときだけ縮める)。戻り値の前の期限を [`pop_budget`] に渡して戻す。
pub fn push_budget(budget: f64) -> f64 {
    let prev = DEADLINE.with(|d| d.get());
    DEADLINE.with(|d| d.set(prev.min(total() + budget)));
    prev
}

/// [`push_budget`] の前の期限に戻す。
pub fn pop_budget(prev: f64) {
    DEADLINE.with(|d| d.set(prev));
}
