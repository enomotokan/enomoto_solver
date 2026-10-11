//! 利用者向けのログ (`Model.solve(log=True)`)。求解の要点 (前処理の結果、根の LP、切除平面、探索の経過) を、Python の
//! `sys.stdout` に 1 行ずつ書く (Jupyter などでも見えるよう、Rust の標準出力・標準エラーには書かない)。
//!
//! 書き先は求解するスレッドごとに持つ ([`with_sink`])。求解は `Model.solve` の間 GIL を手放して動くので、1 行ごとに
//! GIL を取って書く。サブ MIP (呼び出し側が [`suspend`] する) や木の探索の従のスレッド (書き先がない) は出さない。
//! 開発用の詳しい経過 (`ENOMOTO_MIP_LOG` などの標準エラーへの出力) とは別。

use pyo3::prelude::*;
use std::cell::RefCell;

thread_local! {
    /// このスレッドのログの書き先 (`sys.stdout`)。`None` なら出さない。
    static SINK: RefCell<Option<Py<PyAny>>> = const { RefCell::new(None) };
}

/// このスレッドの書き先を `sink` にして `f` を実行し、元に戻す。
pub(crate) fn with_sink<R>(sink: Option<Py<PyAny>>, f: impl FnOnce() -> R) -> R {
    let prev = SINK.with(|s| s.replace(sink));
    let r = f();
    let mine = SINK.with(|s| s.replace(prev));
    if let Some(obj) = mine {
        Python::with_gil(|_| drop(obj));
    }
    r
}

/// このスレッドの書き先を外して `f` を実行し、元に戻す (サブ MIP など、利用者に見せない求解)。
pub(crate) fn suspend<R>(f: impl FnOnce() -> R) -> R {
    let prev = SINK.with(|s| s.take());
    let r = f();
    SINK.with(|s| *s.borrow_mut() = prev);
    r
}

/// このスレッドでログを出すか。
pub(crate) fn enabled() -> bool {
    SINK.with(|s| s.borrow().is_some())
}

/// 1 行書く (書き先がなければ何もしない)。
pub(crate) fn line(text: &str) {
    SINK.with(|s| {
        if let Some(out) = s.borrow().as_ref() {
            Python::with_gil(|py| {
                let _ = out.call_method1(py, "write", (format!("{text}\n"),));
                let _ = out.call_method0(py, "flush");
            });
        }
    });
}
