//! ENOMOTO-Solver の Rust コア (Python 拡張モジュール `enomoto_solver._core`)。
//!
//! 線形計画問題 (LP) と混合整数計画問題 (MIP) を解く。モジュール構成:
//!
//!   - `types` — 各層で共有するデータ型 (変数・目的関数・制約・求解結果など)。
//!   - `model` — PyO3 の入口 `PyModel`。Python 側 `Model` からの入力を検証して保持する。
//!   - `solver` — LP を選択されたエンジンへ振り分け、目的関数値を復元する薄い層。
//!   - `mip` — 整数変数を含む問題のための深さ優先の分枝限定法 (`solver` の上に乗る)。
//!   - `simplex` (+ `simplex::lu`, `simplex::slope_intercept_dual`) — 既定の LP エンジン。
//!     現在は傾き・切片二段解法のエンジンだけを含む (古典的な双対単体法は削除済み)。
//!   - `presolve` (+ `presolve/*`) — 前処理 (スケーリング、冗長行の除去、制約伝播、
//!     各種の変数消去) と、その後処理 (消去した変数の値の復元)。
//!   - `interior_point` (+ `qp`, `kkt`) — IP-PMM 内点法。既定では使われず、
//!     `Model.solve(root_solver="interior")` を指定したときだけ呼ばれる。
//!   - `sparse` — 疎行列 (CSR/CSC)・疎ベクトル・疎アキュムレータなど疎データ構造の一式。
//!   - `graph` — 二部マッチング・強連結成分分解などのグラフアルゴリズム。
//!   - `params` — 閾値・許容誤差・反復上限などの調整用定数をすべて集約したもの。
//!   - `phase_timing` — 求解の節目 (前処理の終わり、段階 A/B の結論) の時刻の記録 (計測用)。
//!
//! `src/legacy/` は最初期の実装の残骸で、コンパイル対象外 (`mod` 宣言なし)。
//! 開発経緯は `docs/improvement_history.md` などを参照。

/// 数値の調整用パラメータを環境変数から読む (プロセスごとに 1 回だけ読み、
/// `OnceLock` にキャッシュする)。未設定・解釈不能なら `$default` を返す。
/// 再ビルドなしで閾値の A/B 比較をするためのもの。既定値は調整済みの値。
macro_rules! tunable {
    ($name:literal, $default:expr, $t:ty) => {{
        // 呼び出し箇所ごとの値のキャッシュ
        static V: std::sync::OnceLock<$t> = std::sync::OnceLock::new();
        *V.get_or_init(|| std::env::var($name).ok().and_then(|s| s.parse::<$t>().ok()).unwrap_or($default))
    }};
}

/// 環境変数 (フラグや文字列設定) をプロセスごとに 1 回だけ読み、`OnceLock` に
/// キャッシュして `Option<&'static str>` で返す。求解中に何度も参照される
/// `ENOMOTO_*` フラグの読み取りコストを避けるため。
/// 初回読み取り後に `std::env::set_var` で変えても反映されない
/// (フラグはプロセス起動前に設定すること)。
macro_rules! env_str {
    ($name:literal) => {{
        // 呼び出し箇所ごとの値のキャッシュ
        static V: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
        V.get_or_init(|| std::env::var($name).ok()).as_deref()
    }};
}

mod graph;
mod interior_point;
mod mip;
mod model;
mod params;
mod phase_timing;
mod presolve;
mod simplex;
mod solver;
mod sparse;
mod types;

use pyo3::prelude::*;
use crate::params::alloc::MIMALLOC_SIZE_LIMIT;

/// Rust 側のグローバルアロケータ。小さいブロック (`MIMALLOC_SIZE_LIMIT` バイト未満) は
/// mimalloc、大きいブロックはシステムアロケータに割り当てる。
///
/// 前処理が短命の小さな確保を大量に行うため、それを mimalloc で安くしつつ、
/// 単体法の大きな密作業ベクトルはシステムアロケータのままにしている。
/// 振り分けはレイアウトのサイズだけで決まるので、`dealloc` は必ず確保した側に届く。
/// しきい値をまたぐ `realloc` は、新しい側で確保 → コピー → 古い側で解放する。
/// 数値結果はアドレスに依存しないので影響しない。
struct SplitAlloc;

unsafe impl std::alloc::GlobalAlloc for SplitAlloc {
    /// サイズに応じて mimalloc かシステムアロケータで確保する。
    #[inline]
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        if layout.size() < MIMALLOC_SIZE_LIMIT {
            mimalloc::MiMalloc.alloc(layout)
        } else {
            std::alloc::System.alloc(layout)
        }
    }
    /// ゼロ初期化付きの確保。振り分けは `alloc` と同じ。
    #[inline]
    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        if layout.size() < MIMALLOC_SIZE_LIMIT {
            mimalloc::MiMalloc.alloc_zeroed(layout)
        } else {
            std::alloc::System.alloc_zeroed(layout)
        }
    }
    /// 解放。確保時と同じ基準でアロケータを選ぶので、必ず確保した側に返る。
    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        if layout.size() < MIMALLOC_SIZE_LIMIT {
            mimalloc::MiMalloc.dealloc(ptr, layout)
        } else {
            std::alloc::System.dealloc(ptr, layout)
        }
    }
    /// 再確保。新旧サイズが同じ側なら、そのアロケータの `realloc` をそのまま使う。
    /// しきい値をまたぐ場合は新しい側で確保し、内容をコピーして古い側で解放する。
    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        // 旧/新ブロックがそれぞれ mimalloc 側 (小) かどうか
        let old_small = layout.size() < MIMALLOC_SIZE_LIMIT;
        let new_small = new_size < MIMALLOC_SIZE_LIMIT;
        if old_small && new_small {
            return mimalloc::MiMalloc.realloc(ptr, layout, new_size);
        }
        if !old_small && !new_small {
            return std::alloc::System.realloc(ptr, layout, new_size);
        }
        let new_layout = std::alloc::Layout::from_size_align_unchecked(new_size, layout.align());
        let new_ptr = self.alloc(new_layout);
        if !new_ptr.is_null() {
            std::ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size));
            self.dealloc(ptr, layout);
        }
        new_ptr
    }
}

/// このクレート全体で使うグローバルアロケータ (`SplitAlloc`)。
#[global_allocator]
static GLOBAL: SplitAlloc = SplitAlloc;

use model::PyModel;

/// 直近の LP 求解の節目の時刻 `[(名前, 開始からの秒), ...]` (`phase_timing` 参照)。
#[pyfunction]
fn last_solve_events() -> Vec<(&'static str, f64)> {
    phase_timing::events()
}

/// Python 拡張モジュール `enomoto_solver._core` の初期化。`PyModel` クラスと
/// `last_solve_events` を登録する。
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyModel>()?;
    m.add_function(wrap_pyfunction!(last_solve_events, m)?)?;
    Ok(())
}
