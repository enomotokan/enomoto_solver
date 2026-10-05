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
