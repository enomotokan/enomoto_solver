//! 求解の途中打ち切り (同時実行の負けた側を止める) のためのトークン。
//!
//! `RootSolver::Auto` は前処理後の大きな問題を傾き・切片双対二段解法と内点法 + クロスオーバーで同時に解き、
//! 先に結論を出した側を採用する (`simplex::race`)。負けた側を止めるため、各解法の反復ループは
//! [`is_cancelled`] を確かめ、真なら結論なしで戻る (`None` / `NotSolved`)。
//!
//! トークンはスレッドローカルに置く ([`with_token`])。求解の関数の引数 (`LpOptions` など) を変えずに
//! 済み、トークンを持たないスレッド (通常の求解) では常に偽。rayon で作業を別スレッドに分ける箇所
//! (連結成分の並列求解) は [`current`] で取り出したトークンを各タスクで [`with_token`] し直す。

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

thread_local! {
    static TOKEN: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) };
}

/// このスレッドの求解が打ち切りを求められているか。
#[inline]
pub fn is_cancelled() -> bool {
    TOKEN.with(|t| t.borrow().as_ref().is_some_and(|a| a.load(Ordering::Relaxed)))
}

/// このスレッドのトークン (なければ `None`)。
pub fn current() -> Option<Arc<AtomicBool>> {
    TOKEN.with(|t| t.borrow().clone())
}

/// `token` をこのスレッドのトークンにして `f` を実行し、元に戻す。
pub fn with_token<R>(token: Option<Arc<AtomicBool>>, f: impl FnOnce() -> R) -> R {
    let prev = TOKEN.with(|t| std::mem::replace(&mut *t.borrow_mut(), token));
    let r = f();
    TOKEN.with(|t| *t.borrow_mut() = prev);
    r
}

// ---- 同時実行で二段解法から内点法へ渡す、元の問題の最適値の下界 ----
//
// 二段解法は段階 B (双対実行可能) の間、今の基底の双対 `y` から元の問題の最適値の厳密な下界
// `L(y) = b·y + Σ_j min_{l_j <= x_j <= u_j} (c - A^T y)_j x_j` を計算して共有の値に書き (大きくなったときだけ)、
// 内点法は自分の主目的値とこの下界の差 (真の双対ギャップの上界) でクロスオーバーに渡す時期を決める。
// 値は `f64` のビットで持つ (`-inf` は未設定)。どの問題の下界か (列数・行数) を一緒に持ち、別の形の問題
// (連結成分ごと、双対化した問題など) を解いている間は書かない。

use std::sync::atomic::AtomicU64;

/// 共有の下界と、その対象の問題の (列数, 行数)。
#[derive(Clone)]
pub struct SharedBound {
    pub value: Arc<AtomicU64>,
    pub n_total: usize,
    pub n_rows: usize,
}

impl SharedBound {
    pub fn new(n_total: usize, n_rows: usize) -> Self {
        SharedBound { value: Arc::new(AtomicU64::new(f64::NEG_INFINITY.to_bits())), n_total, n_rows }
    }

    /// 今の下界 (未設定なら `-inf`)。
    pub fn get(&self) -> f64 {
        f64::from_bits(self.value.load(Ordering::Relaxed))
    }

    /// `v` が今の下界より大きければ書く。
    pub fn raise(&self, v: f64) {
        if !v.is_finite() {
            return;
        }
        let mut cur = self.value.load(Ordering::Relaxed);
        while v > f64::from_bits(cur) {
            match self.value.compare_exchange_weak(cur, v.to_bits(), Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(c) => cur = c,
            }
        }
    }
}

thread_local! {
    static BOUND: RefCell<Option<SharedBound>> = const { RefCell::new(None) };
}

/// このスレッドの共有の下界 (なければ `None`)。
pub fn bound() -> Option<SharedBound> {
    BOUND.with(|b| b.borrow().clone())
}

/// `bound` をこのスレッドの共有の下界にして `f` を実行し、元に戻す。
pub fn with_bound<R>(bound: Option<SharedBound>, f: impl FnOnce() -> R) -> R {
    let prev = BOUND.with(|b| std::mem::replace(&mut *b.borrow_mut(), bound));
    let r = f();
    BOUND.with(|b| *b.borrow_mut() = prev);
    r
}
