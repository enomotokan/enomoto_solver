//! 基底行列 (正方行列) の疎 LU 分解と、その Forrest-Tomlin 更新・求解。
//!
//! - 分解: バケット方式で次数を管理する **Markowitz ピボット選択**。
//!   閾値ピボット (`|a_ij| >= stability * max_i' |a_i'j|`、i' は列 j の
//!   活性行) を満たす候補のうち、Markowitz 数 `(row_nnz - 1) * (col_nnz - 1)`
//!   が最小のものを選ぶ (数値的最大よりフィルイン抑制を優先)。
//! - 結果は `P_row B P_col = L U`。`L` は単位下三角、`U` は上三角で、
//!   どちらも *消去ステップ順* に格納される (ステップ `s` のピボット行・列は
//!   元の基底行列の添字で `row_perm[s]` / `col_perm[s]`)。
//! - 活性部分行列は行優先の値付きラン + 列優先の添字ミラー
//!   ([`KernelMatrix`]、HiGHS `HFactor` の `mc_*`/`mr_*` 相当のフラット配列)
//!   で保持し、次数バケット (`col_buckets[d]` / `row_buckets[d]`) と位置索引で
//!   O(1) の移動を行う。消去 (`eliminate`) は算術と次数・バケット更新を同時に
//!   行い、引退したピボット行は全列の活性行リストから除かれる。
//! - 1 ステップのコストはピボット列の次数 + 触れたフィルに比例 (`m` に
//!   比例しない)。ただし `find_best_pivot` の探索幅は最悪保証ではない。
//! - Forrest-Tomlin 更新 ([`FtLu::try_update`]) により、完全な再分解は
//!   時々だけで済む。
//! - 求解は基本的に `for s in 0..m` のゼロスキップ付き走査。FTRAN の
//!   入力列だけは Gilbert & Peierls (1988) 型の疎前進代入
//!   (`LuFactors::l_solve_sparse_into`, [`GpScratch`]) を使う。
//!
//! 開発経緯は `docs/improvement_history.md` を参照。

use crate::sparse::{CscBuilder, CscMat, CsrMat, EpochMarks};
use std::cell::Cell;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;
use crate::params::lu::{BORDER_MAX_COUNT, BORDER_MAX_FRACTION, BTRAN_HYPER_FRACTION, BTRAN_L_SCATTER_FRACTION, BUCKET_POOL_MAX, DENSE_COL_FRACTION, DENSE_ETA_FRACTION, DENSE_INPUT_FRACTION, DENSE_RHS_FRACTION, DENSE_SWITCH_CHECK_INTERVAL, DENSE_SWITCH_FRACTION, DENSE_SWITCH_MIN_ROWS, DENSITY_AVERAGE_MULTIPLIER, EXPECTED_DENSE_FRACTION, KERNEL_LINEAR_SCAN_MAX, KERNEL_MIN_RUN_CAP, KERNEL_RESERVE_EXTRA, KERNEL_RESERVE_MULT, PIVOT_ROW_SEARCH_MAX_DEGREE, PIVOT_SEARCH_LIMIT, PIVOT_THRESHOLD_FACTOR, PIVOT_THRESHOLD_MAX, PIVOT_THRESHOLD_MIN, R_SPARSE_MIN_ETAS, REBUILD_FILL_LIMIT, SPARSE_PATH_MIN_M, REBUILD_MIN_PIVOT, REUSE_BACKOFF_SHIFT_CAP, REUSE_MAX_BACKOFF, STABILITY, TAU_GP_FRACTION, TICK_BUILD_FLOP_COEF, TICK_BUILD_LU_COEF, TICK_BUILD_M_COEF, TICK_SOLVE_NNZ_COEF, TINY_DROP, U_HYPER_ABORT_FRACTION};

thread_local! {
    /// このスレッドで現在実行中の求解に適用されるピボット閾値。
    /// 初回読み出しまでは `None`、以後は [`pivot_threshold_base`] の値。
    ///
    /// 求解単位の設定なのでスレッドローカルにしている (HiGHS の
    /// `info_.factor_pivot_threshold` 相当)。`mip.rs` が rayon ワーカー上で
    /// 並行に LP を解くため、グローバルにすると互いの閾値を上げ合ってしまう。
    /// 各求解の入口は最初の分解の前に [`reset_pivot_threshold`] を呼ぶこと。
    static PIVOT_THRESHOLD: Cell<Option<f64>> = const { Cell::new(None) };
}

/// [`reset_pivot_threshold`] が戻す基準値。[`STABILITY`]、または環境変数
/// `ENOMOTO_PIVOT_THRESHOLD` が設定されていればその値
/// (`[PIVOT_THRESHOLD_MIN, PIVOT_THRESHOLD_MAX]` にクランプ)。
/// 環境変数はプロセスごとに 1 回だけ読む (求解ごとに読むとホットループで
/// `std::env::var` を払うため)。
fn pivot_threshold_base() -> f64 {
    // プロセス内で 1 回だけ計算される基準値のキャッシュ
    static BASE: OnceLock<f64> = OnceLock::new();
    *BASE.get_or_init(|| {
        env_str!("ENOMOTO_PIVOT_THRESHOLD")
            .and_then(|v| v.parse::<f64>().ok())
            .map(|v| v.clamp(tunable!("ENOMOTO_T_PIVOT_THRESHOLD_MIN", PIVOT_THRESHOLD_MIN, f64), PIVOT_THRESHOLD_MAX))
            .unwrap_or(tunable!("ENOMOTO_T_STABILITY", STABILITY, f64))
    })
}

/// 現在有効な閾値ピボットの下限比率を返す。ピボット候補は、活性部分行列の
/// その列に残る最大絶対値のこの割合以上でなければならない。
/// **分解ごとに 1 回だけ** 読むこと (`MarkowitzState::new`,
/// `factorize_reusing_order`)。消去ステップごとに読まないことで、1 回の分解の
/// ピボット列が 1 つのスカラーだけで決まり、再現性が保たれる。
pub fn pivot_threshold() -> f64 {
    PIVOT_THRESHOLD.with(|c| match c.get() {
        Some(v) => v,
        None => {
            let base = pivot_threshold_base();
            c.set(Some(base));
            base
        }
    })
}

/// ピボット閾値を [`pivot_threshold_base`] に戻す。各求解の入口で最初の分解の
/// 前に呼ぶ (引き上げは求解内で単調なので、求解をまたいで漏らさないため)。
pub fn reset_pivot_threshold() {
    PIVOT_THRESHOLD.with(|c| c.set(Some(pivot_threshold_base())));
}

/// ピボット閾値を [`PIVOT_THRESHOLD_FACTOR`] 倍に 1 段引き上げる
/// (上限 [`PIVOT_THRESHOLD_MAX`])。実際に値が変わったら `true` を返す。
///
/// 数値的トラブル (FT 更新の棄却、`x_B`/`d` のドリフト等) が続く求解では、
/// フィルインを払って安定性を買う方向 (閾値を上げる) に動かす。
/// 求解内では単調で、下げるのは [`reset_pivot_threshold`] のみ。
///
/// **既定では誰も呼ばない** (`ENOMOTO_PIVOT_ESCALATION_STEP` で有効化)。
pub fn escalate_pivot_threshold() -> bool {
    let cur = pivot_threshold();
    let next = (cur * PIVOT_THRESHOLD_FACTOR).min(PIVOT_THRESHOLD_MAX);
    if next > cur {
        PIVOT_THRESHOLD.with(|c| c.set(Some(next)));
        PROF_PIVOT_ESCALATIONS.fetch_add(1, Ordering::Relaxed);
        true
    } else {
        false
    }
}

/// 1 回の `find_best_pivot` が調べてよい候補列数の上限
/// ([`PIVOT_SEARCH_LIMIT`]、環境変数 `ENOMOTO_PIVOT_SEARCH_LIMIT` で上書き可、
/// `0` で無制限)。分解ごとに 1 回 (`MarkowitzState::new`) だけ読む。
fn pivot_search_limit() -> usize {
    env_str!("ENOMOTO_PIVOT_SEARCH_LIMIT")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(PIVOT_SEARCH_LIMIT)
}

// ---- プロファイル用カウンタ (診断出力でのみ読まれる) ----

/// 分解で実行した消去ステップの総数 (`ENOMOTO_PROF_TRIANGULAR` 診断用)。
pub(crate) static PROF_TOTAL_STEPS: AtomicUsize = AtomicUsize::new(0);
/// そのうち Markowitz 数 0 の「自明な」ピボットで済んだステップ数。
pub(crate) static PROF_TRIVIAL_STEPS: AtomicUsize = AtomicUsize::new(0);
/// `find_best_pivot` のバケット走査に費やした累計ナノ秒。
/// `ENOMOTO_PROF_PHASES_EXT` / `ENOMOTO_PROF_PHASES` / `ENOMOTO_PROF_TRIANGULAR`
/// のいずれかが設定されているときだけ計測する (ステップごとに
/// `Instant::now()` 2 回のコストがあるため)。
pub(crate) static PROF_BUCKET_SCAN_NS: AtomicUsize = AtomicUsize::new(0);
/// 残り候補がすべて `initially_dense` だったため `find_best_pivot(false)` に
/// フォールバックした消去ステップ数 (稠密列回避の発動頻度)。
pub(crate) static PROF_DENSE_FALLBACK_STEPS: AtomicUsize = AtomicUsize::new(0);
/// [`PIVOT_SEARCH_LIMIT`] によって早期終了した `find_best_pivot` 呼び出し数。
pub(crate) static PROF_SEARCH_LIMIT_STEPS: AtomicUsize = AtomicUsize::new(0);
/// 全 `find_best_pivot` 呼び出しで調べた候補列の総数
/// ([`PROF_TOTAL_STEPS`] で割るとステップあたりの平均探索幅)。
pub(crate) static PROF_SEARCH_CANDIDATES: AtomicUsize = AtomicUsize::new(0);

/// [`escalate_pivot_threshold`] が実際に閾値を動かした回数。
pub(crate) static PROF_PIVOT_ESCALATIONS: AtomicUsize = AtomicUsize::new(0);
/// `ensure_col_max_abs` が古くなった列最大値を再計算するために走査した
/// 要素数の累計 (再走査ごとに加算)。
pub(crate) static PROF_COLMAX_RESCAN_ENTRIES: AtomicUsize = AtomicUsize::new(0);

/// BTRAN の `L^{-T}` 段で行優先スキャッタ形式を使った回数
/// ([`BTRAN_L_SCATTER_FRACTION`] 参照)。
pub(crate) static PROF_BTRAN_L_SCATTER: AtomicUsize = AtomicUsize::new(0);
/// BTRAN の `L^{-T}` 段で列優先ギャザー形式にフォールバックした回数。
pub(crate) static PROF_BTRAN_L_GATHER: AtomicUsize = AtomicUsize::new(0);

/// Markowitz 消去が作業する活性部分行列のフラット格納 (HiGHS `HFactor` の
/// `mc_*`/`mr_*` 方式を行・列を入れ替えて採用)。
///
/// - 行側 (値を持つ唯一の真実): `row_idx[row_start[i] .. row_start[i] + row_len[i]]`
///   (列番号 `u32`) と同範囲の `row_val` が行 `i` の活性ラン。**列番号昇順**。
///   `row_cap[i]` はそのランがその場で伸びられる容量。
/// - 列側 (添字のみのミラー): `col_ent[col_start[j] .. col_start[j] + col_len[j]]`
///   が列 `j` の活性行リスト。**行番号昇順**。例外は列シングルトン前処理
///   ([`MarkowitzState::peel_column_singletons`]) の間だけで、そこでは引退した
///   ピボット行をその場で除かずに残し (遅延削除)、前処理の最後に一括で詰め直す。
///   列 `j` の活性行数 (= 列次数) は常に `col_act[j]`。
///
/// どちらのランも昇順に保つ。これにより Markowitz 数やピボット絶対値の同点を
/// 「最初に見つかったもの」で決める挙動、`L` 乗数の出力順などが順序付き
/// コンテナ版と完全に一致する (LP 基底は `±1` の同点だらけなので重要)。
struct KernelMatrix {
    /// 行ランの列番号 (`row_val` とオフセットを共有する構造体配列形式)。
    row_idx: Vec<u32>,
    /// 行ランの値。
    row_val: Vec<f64>,
    /// 行 `i` のランの開始位置。
    row_start: Vec<usize>,
    /// 行 `i` のランの長さ (活性要素数)。
    row_len: Vec<usize>,
    /// 行 `i` のランがその場で保持できる容量。
    row_cap: Vec<usize>,
    /// 列ミラーの行番号バッファ。
    col_ent: Vec<u32>,
    /// 列 `j` のランの開始位置。
    col_start: Vec<usize>,
    /// 列 `j` のランの長さ。
    col_len: Vec<usize>,
    /// 列 `j` のランがその場で保持できる容量。
    col_cap: Vec<usize>,
    /// 列 `j` のランのうち活性行 (引退していない行) の数。
    col_act: Vec<usize>,
}

/// 昇順ラン `idx` 内で `j` の位置を返す (無ければ `None`)。短いラン
/// ([`KERNEL_LINEAR_SCAN_MAX`] 以下) は早期終了付き線形探索、長いランは二分探索。
#[inline]
fn sorted_find(idx: &[u32], j: u32) -> Option<usize> {
    if idx.len() <= tunable!("ENOMOTO_T_KERNEL_LINEAR_SCAN_MAX", KERNEL_LINEAR_SCAN_MAX, usize) {
        for (p, &c) in idx.iter().enumerate() {
            if c >= j {
                return if c == j { Some(p) } else { None };
            }
        }
        None
    } else {
        idx.binary_search(&j).ok()
    }
}

impl KernelMatrix {
    /// 疎行 `(列, 値)` の並びから `m x m` の活性部分行列を構築する。
    /// 各行は列でソートし、重複座標は入力順に加算、厳密な 0 は落とす。
    fn new(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Self {
        assert!(m <= u32::MAX as usize, "kernel row index must fit in u32");
        assert_eq!(rows_in.len(), m, "kernel input must be square");
        // 入力の総非ゼロ数
        let total: usize = rows_in.iter().map(|r| r.len()).sum();
        // 容量だけ予約する (後の再配置を 1 回の確保内の `resize` で済ませるため)。
        let mut row_idx: Vec<u32> = Vec::with_capacity(KERNEL_RESERVE_MULT * total + KERNEL_RESERVE_EXTRA);
        let mut row_val: Vec<f64> = Vec::with_capacity(KERNEL_RESERVE_MULT * total + KERNEL_RESERVE_EXTRA);
        let mut row_start = Vec::with_capacity(m);
        let mut row_len = Vec::with_capacity(m);
        let mut row_cap = Vec::with_capacity(m);
        // 1 行分をソート・重複加算するための作業バッファ
        let mut buf: Vec<(usize, f64)> = Vec::new();

        for row in rows_in.iter() {
            buf.clear();
            buf.extend_from_slice(row);
            // 列で安定ソートし、重複ランを入力順に加算、厳密な 0 を除去する
            // (加算順序も含め旧 BTreeMap 構築とビット一致)。
            buf.sort_by_key(|&(j, _)| j);
            let start = row_idx.len();
            let mut k = 0;
            while k < buf.len() {
                let j = buf[k].0;
                let mut acc = 0.0f64;
                while k < buf.len() && buf[k].0 == j {
                    acc += buf[k].1;
                    k += 1;
                }
                if acc != 0.0 {
                    row_idx.push(j as u32);
                    row_val.push(acc);
                }
            }
            // 余白なし (`cap == len`)。フィル 1 個はピボット列要素の退出と
            // 相殺されるのでその場に収まり、2 個以上で初めて再配置される。
            let len = row_idx.len() - start;
            row_start.push(start);
            row_len.push(len);
            row_cap.push(len);
        }

        // 計数ソートで列ミラーを作る。`i` 昇順に詰めるので各列ランは自動的に昇順。
        let mut col_len = vec![0usize; m];
        for &j in &row_idx {
            col_len[j as usize] += 1;
        }
        let mut col_start = vec![0usize; m];
        let mut pos = 0usize;
        for j in 0..m {
            col_start[j] = pos;
            pos += col_len[j];
        }
        // 行側と同じ理由で余白なし・容量予約。
        let col_cap = col_len.clone();
        let mut col_ent: Vec<u32> = Vec::with_capacity(KERNEL_RESERVE_MULT * total + KERNEL_RESERVE_EXTRA);
        col_ent.resize(pos, 0);
        // 各列にすでに書き込んだ行数
        let mut fill = vec![0usize; m];
        for i in 0..m {
            let (s, l) = (row_start[i], row_len[i]);
            for &j in &row_idx[s..s + l] {
                let j = j as usize;
                col_ent[col_start[j] + fill[j]] = i as u32;
                fill[j] += 1;
            }
        }

        let col_act = col_len.clone();
        KernelMatrix { row_idx, row_val, row_start, row_len, row_cap, col_ent, col_start, col_len, col_cap, col_act }
    }

    /// 行 `i` の活性ラン (列番号昇順, 値) を返す。
    #[inline]
    fn row(&self, i: usize) -> (&[u32], &[f64]) {
        let s = self.row_start[i];
        let e = s + self.row_len[i];
        (&self.row_idx[s..e], &self.row_val[s..e])
    }

    /// 列 `j` の活性行リスト (行番号昇順) を返す。
    #[inline]
    fn col(&self, j: usize) -> &[u32] {
        let s = self.col_start[j];
        &self.col_ent[s..s + self.col_len[j]]
    }

    /// `(i, j)` の値を返す。その座標が活性でなければ `None`。
    #[inline]
    fn row_get(&self, i: usize, j: usize) -> Option<f64> {
        let s = self.row_start[i];
        let e = s + self.row_len[i];
        sorted_find(&self.row_idx[s..e], j as u32).map(|p| self.row_val[s + p])
    }

    /// 行 `i` のランが `need` 要素をその場で保持できなければ、容量を倍増して
    /// 行バッファ末尾へ再配置する。空いた旧領域は詰めずに放置する
    /// (死領域は活性総量で抑えられ、分解は短命なため)。
    fn ensure_row_cap(&mut self, i: usize, need: usize) {
        if need <= self.row_cap[i] {
            return;
        }
        let new_cap = need.max(self.row_cap[i] * 2).max(KERNEL_MIN_RUN_CAP);
        let (old_start, len) = (self.row_start[i], self.row_len[i]);
        let start = self.row_idx.len();
        self.row_idx.resize(start + new_cap, 0);
        self.row_val.resize(start + new_cap, 0.0);
        self.row_idx.copy_within(old_start..old_start + len, start);
        self.row_val.copy_within(old_start..old_start + len, start);
        self.row_start[i] = start;
        self.row_cap[i] = new_cap;
    }

    /// 列 `j` のランについての [`Self::ensure_row_cap`] 相当。
    fn ensure_col_cap(&mut self, j: usize, need: usize) {
        if need <= self.col_cap[j] {
            return;
        }
        let new_cap = need.max(self.col_cap[j] * 2).max(KERNEL_MIN_RUN_CAP);
        let (old_start, len) = (self.col_start[j], self.col_len[j]);
        let start = self.col_ent.len();
        self.col_ent.resize(start + new_cap, 0);
        self.col_ent.copy_within(old_start..old_start + len, start);
        self.col_start[j] = start;
        self.col_cap[j] = new_cap;
    }

    /// 行 `i` を列 `j` の活性行リストに昇順を保って挿入する。
    fn col_insert(&mut self, j: usize, i: usize) {
        let len = self.col_len[j];
        let pos = {
            let s = self.col_start[j];
            self.col_ent[s..s + len].partition_point(|&r| (r as usize) < i)
        };
        self.ensure_col_cap(j, len + 1);
        let s = self.col_start[j];
        self.col_ent.copy_within(s + pos..s + len, s + pos + 1);
        self.col_ent[s + pos] = i as u32;
        self.col_len[j] = len + 1;
        self.col_act[j] += 1;
    }

    /// 行 `i` を列 `j` の活性行リストから除く。無ければ何もしない
    /// (`col_clear` 済みの列にもピボット行の除去呼び出しが来るため必要)。
    fn col_remove(&mut self, j: usize, i: usize) {
        let (s, len) = (self.col_start[j], self.col_len[j]);
        let run = &self.col_ent[s..s + len];
        let pos = run.partition_point(|&r| (r as usize) < i);
        if pos >= len || self.col_ent[s + pos] as usize != i {
            return;
        }
        self.col_ent.copy_within(s + pos + 1..s + len, s + pos);
        self.col_len[j] = len - 1;
        self.col_act[j] -= 1;
    }

    /// 列 `j` の活性行が 1 つ引退したことだけを記録する (ランからは除かない遅延削除。
    /// 列シングルトン前処理専用で、最後に [`Self::compact_cols`] で詰め直す)。
    #[inline]
    fn col_retire(&mut self, j: usize) {
        self.col_act[j] -= 1;
    }

    /// 遅延削除で引退行が残っている列ランから、`row_used` の行を順序を保って除く
    /// (`O(列ミラーの総長)`)。以後は全列のランが活性行だけになる。
    fn compact_cols(&mut self, row_used: &[bool]) {
        for j in 0..self.col_len.len() {
            if self.col_len[j] == self.col_act[j] {
                continue;
            }
            let s = self.col_start[j];
            let mut w = s;
            for p in s..s + self.col_len[j] {
                let r = self.col_ent[p];
                if !row_used[r as usize] {
                    self.col_ent[w] = r;
                    w += 1;
                }
            }
            self.col_len[j] = w - s;
            debug_assert_eq!(self.col_len[j], self.col_act[j], "column run must hold exactly the active rows after compaction");
        }
    }

    /// 列 `j` の活性行リストを空にする (ピボット列の引退時)。
    #[inline]
    fn col_clear(&mut self, j: usize) {
        self.col_len[j] = 0;
        self.col_act[j] = 0;
    }
}

/// 消去ステップ用の作業バッファ。[`MarkowitzState`] が所有し、
/// [`MarkowitzState::eliminate`] の間だけ `mem::take` で貸し出す
/// (ステップごとの確保を避けるため)。
#[derive(Default)]
struct ElimScratch {
    /// まだ消去が必要なピボット列の行 (処理中に列リスト自体が変わるのでコピー)。
    affected: Vec<usize>,
    /// 一般マージ経路で書き換え中の行の出力 (列番号)。フォールバック時のみ使用。
    merged_idx: Vec<u32>,
    /// 同上の値。
    merged_val: Vec<f64>,
    /// 現在の影響行のフィルイン (列番号昇順)。
    fill_idx: Vec<u32>,
    /// 同上の値。
    fill_val: Vec<f64>,
    /// 現在の影響行が新たに加わる列 (行の借用解放後に列ミラーへ反映)。
    col_add: Vec<usize>,
    /// 現在の影響行が抜ける列。
    col_del: Vec<usize>,
    /// このステップの `L` 乗数 `(行, 乗数)` (影響行の順)。
    l_out: Vec<(usize, f64)>,
    /// 引退するピボット行の列 (そこから行を除くため)。
    pi_cols: Vec<usize>,
    /// ピボット行の非ピボット値を列位置に散布した密配列 (他は `0.0`。
    /// ピボット行は厳密な 0 を持たないので非ゼロ = 所属)。各ステップ末に
    /// 要素ごとに 0 に戻す。
    wval: Vec<f64>,
    /// 影響行ごとの列スタンプ。フィルインが出る行でのみ、その行が既に
    /// 持っていたピボット行の列を判定するのに使う。
    rmark: Vec<u32>,
    /// `rmark` の現在のスタンプ値。
    rstamp: u32,
}

impl ElimScratch {
    /// `m` 列分の密バッファを確保して作る。
    fn new(m: usize) -> Self {
        ElimScratch { wval: vec![0.0; m], rmark: vec![0; m], rstamp: 0, ..Default::default() }
    }

    /// 1 ステップの開始時に `l_out` を空にする。
    fn begin(&mut self) {
        self.l_out.clear();
    }

    /// 新しい `rmark` スタンプを返す。0 に一周したときだけ全スタンプを消す
    /// ([`GpScratch::bump_epoch`] と同じ手法)。
    #[inline]
    fn next_rstamp(&mut self) -> u32 {
        self.rstamp = self.rstamp.wrapping_add(1);
        if self.rstamp == 0 {
            self.rmark.iter_mut().for_each(|e| *e = 0);
            self.rstamp = 1;
        }
        self.rstamp
    }
}

/// Markowitz 消去中の活性部分行列と行・列次数 (バケット配列) を管理する。
/// 格納は [`KernelMatrix`]。消去算術と次数・バケット更新は同じ場所
/// (`eliminate`) で行う。
struct MarkowitzState {
    /// 行列の次数 `m` (未使用だが保持)。
    #[allow(dead_code)]
    m: usize,

    /// 活性部分行列 (行優先の値 + 列優先の行番号ミラー)。
    mat: KernelMatrix,

    /// 列 `j` の現在の次数 (活性非ゼロ数)。バケット配置と一致する。
    col_degree: Vec<usize>,
    /// 行 `i` の現在の次数。
    row_degree: Vec<usize>,

    /// 列バケット: `col_buckets[d]` = 現在次数ちょうど `d` の列。長さ `m + 1`。
    col_buckets: Vec<Vec<usize>>,
    /// 行バケット: `row_buckets[d]` = 現在次数ちょうど `d` の行。
    row_buckets: Vec<Vec<usize>>,

    /// `col_bucket_pos[j]` = 列 `j` の `col_buckets[col_degree[j]]` 内の位置。
    /// 使用済み (全バケットから除去済み) なら `None`。
    col_bucket_pos: Vec<Option<usize>>,
    /// 行版の `col_bucket_pos`。
    row_bucket_pos: Vec<Option<usize>>,

    /// 列がすでにピボットとして使われたか。
    col_used: Vec<bool>,
    /// 行がすでにピボットとして使われたか。
    row_used: Vec<bool>,

    /// 列の活性行における最大絶対値 (閾値ピボットの基準)。
    col_max_abs: Vec<f64>,

    /// `col_max_abs[j]` が古い (列の要素が縮小・退出したので上界に
    /// すぎない) ことを示す。再計算は `find_best_pivot` が実際に読む直前の
    /// `ensure_col_max_abs` まで遅延する。
    col_max_abs_dirty: Vec<bool>,

    /// 消去開始前の列 `j` の次数が `DENSE_COL_FRACTION * m` を超えていたか。
    /// 構築時に固定し更新しない (現在次数は他のピボットで行が抜けて縮むが、
    /// 構造的に稠密な列を踏むと全影響行へピボット行を散布する最も高価な
    /// ステップになるため、元の構造で判定する)。
    initially_dense: Vec<bool>,

    /// ステップ間で再利用する作業バッファ ([`ElimScratch`])。
    scratch: ElimScratch,

    /// [`pivot_search_limit`] の値 (分解ごとに 1 回解決)。`0` は無制限。
    search_limit: usize,

    /// [`pivot_threshold`] の値 (分解開始時に 1 回だけ取得)。
    threshold: f64,

    /// [`PROF_COLMAX_RESCAN_ENTRIES`] 用の非アトミック累計。`Drop` で 1 回だけ
    /// 反映する (再走査ごとの `fetch_add` を避けるため)。
    prof_colmax_rescan_entries: usize,
    /// `find_best_pivot` 内で列ごとの値をキャッシュする配列
    /// (遅延 `col_max_abs` 再計算用)。
    col_value_cache: Vec<Option<f64>>,
    /// `ENOMOTO_LU_LAZY_COLMAX` (既定 on): 遅延再計算を使うか。分解ごとに 1 回読む。
    lazy_colmax: bool,
    /// `ENOMOTO_LU_ROW_SINGLETON=<rel>` (**経路が変わる。既定 off = `-1`**)。
    /// `>= 0` のとき、列シングルトンが無ければ、列最大値の `rel` 倍以上の
    /// 要素を持つ最初の活性行シングルトンを即座に採る (`rel = 0` は HiGHS の
    /// 規則)。[`factorize_bordered`] の疎フェーズでは必ず無効
    /// (境界列要素がその乗数で Schur 補行列を通じて更新されるため)。
    row_singleton_rel: f64,
    /// `ENOMOTO_PIVOT_ROW_SEARCH` (**経路が変わる。既定 `0` = off**)。
    /// 列バケット `c` を走査した後、次数 `c` の行バケットも走査する
    /// (HiGHS `buildKernel` 流)。値は走査する最大の行次数 (`1` = 行
    /// シングルトンのみ)。走査した行も探索上限に数える。
    /// [`factorize_bordered`] の疎フェーズでは無効。
    row_search: usize,
    /// `find_best_pivot` が自身の時間を計測するか (プロファイル用環境変数が
    /// 設定されているときのみ。分解ごとに 1 回解決)。
    prof_timing: bool,
    /// `PROF_TOTAL_STEPS` 用の分解内累計 (`Drop` で反映)。
    prof_steps: usize,
    /// `PROF_TRIVIAL_STEPS` 用の分解内累計。
    prof_trivial: usize,
    /// `PROF_SEARCH_LIMIT_STEPS` 用の分解内累計。
    prof_search_limit_hits: usize,
    /// `PROF_SEARCH_CANDIDATES` 用の分解内累計。
    prof_candidates: usize,
    /// `PROF_BUCKET_SCAN_NS` 用の分解内累計。
    prof_scan_ns: usize,
    /// `ENOMOTO_LU_INPLACE_ELIM` (既定 on): `eliminate` の散布ベースの
    /// その場更新を使うか。`0` なら全行で単純な 2 ポインタマージ
    /// (因子はビット一致)。
    inplace_elim: bool,
}

thread_local! {
    /// 破棄された [`MarkowitzState`] の `col_buckets`/`row_buckets` を再利用する
    /// プール (外側 `Vec` と各バケットの確保ごと)。再利用前に中身を消し、
    /// 新品と同じ順で詰めるので観測可能な違いはない。
    static BUCKET_POOL: std::cell::RefCell<Vec<Vec<Vec<usize>>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// プールからバケット配列を 1 つ取り出し、`n` 個の空バケットにして返す。
fn take_buckets(n: usize) -> Vec<Vec<usize>> {
    let mut b = BUCKET_POOL.with(|p| p.borrow_mut().pop()).unwrap_or_default();
    b.truncate(n);
    for v in b.iter_mut() {
        v.clear();
    }
    b.resize_with(n, Vec::new);
    b
}

/// バケット配列をプールに返す (プールが [`BUCKET_POOL_MAX`] 個未満のときのみ)。
fn give_buckets(b: Vec<Vec<usize>>) {
    BUCKET_POOL.with(|p| {
        let mut p = p.borrow_mut();
        if p.len() < BUCKET_POOL_MAX {
            p.push(b);
        }
    });
}

impl Drop for MarkowitzState {
    /// バケットをプールへ返し、分解内のプロファイル累計を静的カウンタへ反映する。
    fn drop(&mut self) {
        give_buckets(std::mem::take(&mut self.col_buckets));
        give_buckets(std::mem::take(&mut self.row_buckets));
        PROF_COLMAX_RESCAN_ENTRIES.fetch_add(self.prof_colmax_rescan_entries, Ordering::Relaxed);
        PROF_TOTAL_STEPS.fetch_add(self.prof_steps, Ordering::Relaxed);
        PROF_TRIVIAL_STEPS.fetch_add(self.prof_trivial, Ordering::Relaxed);
        PROF_SEARCH_LIMIT_STEPS.fetch_add(self.prof_search_limit_hits, Ordering::Relaxed);
        PROF_SEARCH_CANDIDATES.fetch_add(self.prof_candidates, Ordering::Relaxed);
        PROF_BUCKET_SCAN_NS.fetch_add(self.prof_scan_ns, Ordering::Relaxed);
    }
}

impl MarkowitzState {
    /// `m x m` の疎行 `rows_in` から消去状態を初期化する
    /// (次数・バケット・列最大値・稠密列フラグ・各種環境変数設定)。
    fn new(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Self {
        let mat = KernelMatrix::new(m, rows_in);

        let mut col_max_abs = vec![0.0f64; m];
        let mut row_degree = vec![0usize; m];
        for i in 0..m {
            let (idx, val) = mat.row(i);
            row_degree[i] = idx.len();
            for (&j, &v) in idx.iter().zip(val) {
                let j = j as usize;
                col_max_abs[j] = col_max_abs[j].max(v.abs());
            }
        }
        let col_degree: Vec<usize> = (0..m).map(|j| mat.col(j).len()).collect();

        let mut col_buckets = take_buckets(m + 1);
        let mut row_buckets = take_buckets(m + 1);
        let mut col_bucket_pos = vec![None; m];
        let mut row_bucket_pos = vec![None; m];

        for j in 0..m {
            let deg = col_degree[j];
            col_bucket_pos[j] = Some(col_buckets[deg].len());
            col_buckets[deg].push(j);
        }
        for i in 0..m {
            let deg = row_degree[i];
            row_bucket_pos[i] = Some(row_buckets[deg].len());
            row_buckets[deg].push(i);
        }

        // この次数を超える列を「初期稠密」とみなす
        let dense_threshold = tunable!("ENOMOTO_T_DENSE_COL_FRACTION", DENSE_COL_FRACTION, f64) * m as f64;
        let initially_dense: Vec<bool> = col_degree.iter().map(|&d| d as f64 > dense_threshold).collect();

        MarkowitzState {
            m,
            mat,
            col_degree,
            row_degree,
            col_buckets,
            row_buckets,
            col_bucket_pos,
            row_bucket_pos,
            col_used: vec![false; m],
            row_used: vec![false; m],
            col_max_abs,
            col_max_abs_dirty: vec![false; m],
            initially_dense,
            scratch: ElimScratch::new(m),
            search_limit: pivot_search_limit(),
            threshold: pivot_threshold(),
            prof_colmax_rescan_entries: 0,
            col_value_cache: Vec::new(),
            inplace_elim: !matches!(env_str!("ENOMOTO_LU_INPLACE_ELIM"), Some("0")),
            prof_timing: env_str!("ENOMOTO_PROF_PHASES_EXT").is_some()
                || env_str!("ENOMOTO_PROF_PHASES").is_some()
                || env_str!("ENOMOTO_PROF_TRIANGULAR").is_some(),
            prof_steps: 0,
            prof_trivial: 0,
            prof_search_limit_hits: 0,
            prof_candidates: 0,
            prof_scan_ns: 0,
            row_singleton_rel: env_str!("ENOMOTO_LU_ROW_SINGLETON").and_then(|v| v.parse::<f64>().ok()).unwrap_or(-1.0),
            row_search: tunable!("ENOMOTO_PIVOT_ROW_SEARCH", PIVOT_ROW_SEARCH_MAX_DEGREE, usize),
            lazy_colmax: !matches!(env_str!("ENOMOTO_LU_LAZY_COLMAX"), Some("0")),
        }
    }

    /// 行 `i` の活性要素 `(列, 値)` を列昇順で返すイテレータ。
    #[inline]
    fn row(&self, i: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
        let (idx, val) = self.mat.row(i);
        idx.iter().zip(val).map(|(&j, &v)| (j as usize, v))
    }

    /// `(i, j)` の値 (活性でなければ `None`)。
    #[inline]
    fn value_at(&self, i: usize, j: usize) -> Option<f64> {
        self.mat.row_get(i, j)
    }

    /// 直前の [`Self::eliminate`] が `L` 用に出力した `(行, 乗数)` の並び
    /// (全ステップで 1 つのバッファを共有するためスクラッチに保持)。
    #[inline]
    fn l_out(&self) -> &[(usize, f64)] {
        &self.scratch.l_out
    }

    /// 列 `j` を現在の次数バケットから除く (末尾と入れ替えて pop)。
    fn remove_from_bucket_col(&mut self, j: usize) {
        if let Some(pos) = self.col_bucket_pos[j] {
            let deg = self.col_degree[j];
            let bucket = &mut self.col_buckets[deg];
            if pos < bucket.len() {
                let last_j = bucket.pop().unwrap();
                if pos < bucket.len() {
                    bucket[pos] = last_j;
                    self.col_bucket_pos[last_j] = Some(pos);
                }
            }
            self.col_bucket_pos[j] = None;
        }
    }

    /// 行 `i` を現在の次数バケットから除く (末尾と入れ替えて pop)。
    fn remove_from_bucket_row(&mut self, i: usize) {
        if let Some(pos) = self.row_bucket_pos[i] {
            let deg = self.row_degree[i];
            let bucket = &mut self.row_buckets[deg];
            if pos < bucket.len() {
                let last_i = bucket.pop().unwrap();
                if pos < bucket.len() {
                    bucket[pos] = last_i;
                    self.row_bucket_pos[last_i] = Some(pos);
                }
            }
            self.row_bucket_pos[i] = None;
        }
    }

    /// 列 `j` の次数を `new_deg` に更新し、必要ならバケットを移す
    /// (使用済み列や次数不変なら何もしない)。
    fn update_col_degree(&mut self, j: usize, new_deg: usize) {
        if self.col_used[j] || new_deg == self.col_degree[j] {
            return;
        }
        self.remove_from_bucket_col(j);
        self.col_degree[j] = new_deg;
        let pos = self.col_buckets[new_deg].len();
        self.col_bucket_pos[j] = Some(pos);
        self.col_buckets[new_deg].push(j);
    }

    /// 行 `i` の次数を `new_deg` に更新し、必要ならバケットを移す。
    fn update_row_degree(&mut self, i: usize, new_deg: usize) {
        if self.row_used[i] || new_deg == self.row_degree[i] {
            return;
        }
        self.remove_from_bucket_row(i);
        self.row_degree[i] = new_deg;
        let pos = self.row_buckets[new_deg].len();
        self.row_bucket_pos[i] = Some(pos);
        self.row_buckets[new_deg].push(i);
    }

    /// 列 `j` の次数・バケット位置を列ミラーの活性行数から更新し
    /// (O(1))、`col_max_abs[j]` を古い印にする (再計算は遅延)。
    fn refresh_column(&mut self, j: usize) {
        if self.col_used[j] {
            return;
        }
        let new_deg = self.mat.col_act[j];
        self.update_col_degree(j, new_deg);
        self.col_max_abs_dirty[j] = true;
    }

    /// `col_max_abs[j]` が古い印付きなら列ミラーから再計算する (そうでなければ何もしない)。
    fn ensure_col_max_abs(&mut self, j: usize) {
        if !self.col_max_abs_dirty[j] {
            return;
        }
        self.col_max_abs[j] = self.col_max_abs_rescan(j);
        self.col_max_abs_dirty[j] = false;
    }

    /// 列 `j` の活性要素の `max |a_ij|` を列ミラー経由で計算する。
    fn col_max_abs_rescan(&mut self, j: usize) -> f64 {
        let col = self.mat.col(j);
        self.prof_colmax_rescan_entries += col.len();
        let mut mx = 0.0f64;
        for &r in col {
            if let Some(v) = self.mat.row_get(r as usize, j) {
                mx = f64::max(mx, v.abs());
            }
        }
        mx
    }

    /// 次のピボット `(行, 列)` を選ぶ。活性列を次数の小さいバケット順に走査し、
    /// 各列ではその列の活性行 (列ミラー) だけを調べ、閾値ピボットを満たす
    /// 候補のうち Markowitz 数最小 (同点ならピボット絶対値最大) を選ぶ。
    /// 候補が無ければ `None`。
    ///
    /// - 次数レベルごとの早期終了 (`best_score <= deg_col^2`) は実用的緩和で、
    ///   大域最小は保証しない。
    /// - `skip_dense`: `true` なら `initially_dense` 列を無条件に飛ばす
    ///   (呼び出し側は見つからなければ `false` で再試行する)。
    /// - [`PIVOT_SEARCH_LIMIT`] 個の候補列で打ち切る (バケット途中でも)。
    ///   ただしピボットが既に見つかっている場合のみなので `Some` を `None` に
    ///   することはない。
    fn find_best_pivot(&mut self, skip_dense: bool) -> Option<(usize, usize)> {
        // プロファイル時のみ計測開始時刻を取る
        let prof_t0 = if self.prof_timing { Some(std::time::Instant::now()) } else { None };
        let mut best: Option<(usize, usize)> = None;
        let mut best_score = usize::MAX;
        let mut best_pivot_abs = 0.0f64;
        // この呼び出しで調べた候補列数 (`search_limit` の対象、HiGHS の `searchCount`)
        let mut searched = 0usize;
        // 終了理由: 0 = Markowitz 数 0 で終了, 1 = 探索上限で終了,
        // それ以外 = 次数レベルでの終了または全走査
        let mut exit = 3u8;

        // 列バケット走査を開始する次数 (行シングルトン採用時は走査を飛ばす)
        let mut scan_from = 1usize;
        // 活性行の最小次数 (必要になった時点で遅延計算)
        let mut rmin = usize::MAX;
        // 任意機能 (既定 off): 列シングルトンが無ければ行シングルトンを直接採る
        if self.row_singleton_rel >= 0.0 && self.col_buckets[1].is_empty() {
            if let Some(p) = self.try_row_singleton(skip_dense) {
                best = Some(p);
                exit = 0;
                scan_from = self.col_buckets.len();
            }
        }
        // フィールドを分けて借用し、候補ごとの参照を素のスライス経由にする
        // (遅延 `col_max_abs` 再計算は `col_max_abs`/`col_max_abs_dirty`/`col_value_cache`
        // だけを書く)。
        let col_buckets = &self.col_buckets;
        let mat = &self.mat;
        let row_degree = &self.row_degree[..];
        let col_degree = &self.col_degree[..];
        let initially_dense = &self.initially_dense[..];
        let threshold = self.threshold;
        let search_limit = self.search_limit;
        let lazy_colmax = self.lazy_colmax;
        'scan: for deg_col in scan_from..col_buckets.len() {
            // バケットの中身はこの走査中は変わらない。
            for &j in &col_buckets[deg_col] {
                // バケットには使用済み列は入らない。
                debug_assert!(!self.col_used[j], "bucketed column must be active");
                if skip_dense && initially_dense[j] {
                    continue;
                }
                let col_deg = col_degree[j];
                // 列次数 - 1 (Markowitz 数の列側因子)
                let cm1 = col_deg - 1;
                searched += 1;
                // 古い `col_max_abs` は、Markowitz 数の篩を通った最初の要素で
                // 遅延再計算する (`min_pivot` はそこでしか使わないため)。
                // 再計算で読んだ値は `col_value_cache` にキャッシュして同じ列の候補参照に
                // 再利用する。比較結果・選ばれるピボットは不変。
                if !lazy_colmax && self.col_max_abs_dirty[j] {
                    let mut mx = 0.0f64;
                    for &r in mat.col(j) {
                        if let Some(v) = mat.row_get(r as usize, j) {
                            mx = f64::max(mx, v.abs());
                        }
                    }
                    self.prof_colmax_rescan_entries += mat.col_len[j];
                    self.col_max_abs[j] = mx;
                    self.col_max_abs_dirty[j] = false;
                }
                // 閾値ピボットの下限 (NaN = 未計算の古い列)
                let mut min_pivot = if self.col_max_abs_dirty[j] { f64::NAN } else { threshold * self.col_max_abs[j] };
                // `col_value_cache` にこの列の値がキャッシュ済みか
                let mut cached = false;
                // 列ミラーは活性行だけを保持している。
                let col = mat.col(j);
                let mut k = 0usize;
                while k < col.len() {
                    // Markowitz 数は次数だけで決まるので、`best_score` に勝てない
                    // 候補は値を引かずに飛ばす。
                    let bs = best_score;
                    match col[k..].iter().position(|&r| (row_degree[r as usize] - 1) * cm1 <= bs) {
                        Some(off) => k += off,
                        None => break,
                    }
                    let i = col[k] as usize;
                    debug_assert!(!self.row_used[i], "column mirror holds only active rows");
                    let score = (row_degree[i] - 1) * cm1;
                    if min_pivot.is_nan() {
                        // 古い列で初めて閾値が必要になった: 全値をキャッシュしつつ再走査
                        let mut mx = 0.0f64;
                        self.col_value_cache.clear();
                        for &r in col {
                            let v = mat.row_get(r as usize, j);
                            if let Some(v) = v {
                                mx = f64::max(mx, v.abs());
                            }
                            self.col_value_cache.push(v);
                        }
                        self.prof_colmax_rescan_entries += col.len();
                        self.col_max_abs[j] = mx;
                        self.col_max_abs_dirty[j] = false;
                        min_pivot = threshold * mx;
                        cached = true;
                    }
                    let v = if cached { self.col_value_cache[k] } else { mat.row_get(i, j) };
                    k += 1;
                    let Some(v) = v else { continue };
                    if v == 0.0 || v.abs() < min_pivot {
                        continue;
                    }
                    if score < best_score || (score == best_score && v.abs() > best_pivot_abs) {
                        best_score = score;
                        best = Some((i, j));
                        best_pivot_abs = v.abs();
                    }
                }
                if best_score == 0 {
                    exit = 0;
                    break 'scan;
                }
                // 探索上限は列を途中で切らず、列単位で数える。
                if search_limit != 0 && searched >= search_limit && best.is_some() {
                    exit = 1;
                    break 'scan;
                }
            }
            if deg_col <= self.row_search && deg_col < self.row_buckets.len() {
                // 行探索 (B2(b)): 次数 `deg_col` の行を走査する (`row_search` 参照)。
                // 行次数 - 1 (Markowitz 数の行側因子)
                let rc1 = deg_col - 1;
                for &i in &self.row_buckets[deg_col] {
                    let s = mat.row_start[i];
                    let e = s + mat.row_len[i];
                    for k in s..e {
                        let j = mat.row_idx[k] as usize;
                        if skip_dense && initially_dense[j] {
                            continue;
                        }
                        let score = rc1 * (col_degree[j] - 1);
                        if score > best_score {
                            continue;
                        }
                        let v = mat.row_val[k];
                        // `best` を改善しうる候補だけ閾値判定 (と古い列の再走査) をする。
                        if v == 0.0 || !(score < best_score || v.abs() > best_pivot_abs) {
                            continue;
                        }
                        if self.col_max_abs_dirty[j] {
                            let mut mx = 0.0f64;
                            for &r in mat.col(j) {
                                if let Some(x) = mat.row_get(r as usize, j) {
                                    mx = f64::max(mx, x.abs());
                                }
                            }
                            self.prof_colmax_rescan_entries += mat.col_len[j];
                            self.col_max_abs[j] = mx;
                            self.col_max_abs_dirty[j] = false;
                        }
                        if v.abs() < threshold * self.col_max_abs[j] {
                            continue;
                        }
                        if score < best_score || (score == best_score && v.abs() > best_pivot_abs) {
                            best_score = score;
                            best = Some((i, j));
                            best_pivot_abs = v.abs();
                        }
                    }
                    searched += 1;
                    if best_score == 0 {
                        exit = 0;
                        break 'scan;
                    }
                    if search_limit != 0 && searched >= search_limit && best.is_some() {
                        exit = 1;
                        break 'scan;
                    }
                }
            }
            if best.is_some() && best_score <= deg_col * deg_col {
                exit = 2;
                break 'scan;
            }
            // 以降のバケットの列は次数 > `deg_col`、活性行の次数は >= `rmin`
            // なので、以降の候補は `(rmin - 1) * deg_col` 未満にならない。
            // 厳密な `<` なので同点処理は全走査と同じ (選ばれるピボットは不変)。
            if best.is_some() && tunable!("ENOMOTO_PIVOT_RMIN_CUTOFF", 1usize, usize) != 0 {
                if rmin == usize::MAX {
                    rmin = (1..self.row_buckets.len()).find(|&d| !self.row_buckets[d].is_empty()).unwrap_or(1);
                }
                if best_score < (rmin - 1) * deg_col {
                    exit = 2;
                    break 'scan;
                }
            }
        }

        // プロファイル累計 (`Drop` でまとめて静的カウンタへ反映)
        self.prof_steps += 1;
        self.prof_candidates += searched;
        match exit {
            0 => self.prof_trivial += 1,
            1 => self.prof_search_limit_hits += 1,
            _ => {}
        }
        if let Some(t0) = prof_t0 {
            self.prof_scan_ns += t0.elapsed().as_nanos() as usize;
        }
        best
    }

    /// 活性行シングルトン (`row_buckets[1]` の順、最大 `search_limit` 個) のうち、
    /// その唯一の要素が `row_singleton_rel * col_max_abs` 以上である最初のものを返す。
    fn try_row_singleton(&mut self, skip_dense: bool) -> Option<(usize, usize)> {
        let n = self.row_buckets[1].len();
        // 調べる行シングルトンの最大数
        let cap = if self.search_limit == 0 { n } else { n.min(self.search_limit) };
        for idx in 0..cap {
            let i = self.row_buckets[1][idx];
            let (j, v) = self.row(i).next().unwrap();
            if (skip_dense && self.initially_dense[j]) || v == 0.0 {
                continue;
            }
            if self.row_singleton_rel == 0.0 {
                return Some((i, j));
            }
            self.ensure_col_max_abs(j);
            if v.abs() >= self.row_singleton_rel * self.col_max_abs[j] {
                return Some((i, j));
            }
        }
        None
    }

    /// 列シングルトンの前処理 (HiGHS `buildSimple` 相当)。消去の先頭で
    /// `find_best_pivot(true)` + `eliminate` が列シングルトンを 1 本ずつ採るのと
    /// **同じピボット・同じバケット操作** を、探索と消去の汎用処理を通さずに行う。
    /// 採ったステップ数を返し、呼び出し側はその次のステップから通常の消去を続ける。
    ///
    /// - 対象: `col_buckets[1]` に `initially_dense` でない列がある間。
    ///   `find_best_pivot(true)` はその格納順で最初の列を選び (唯一の活性要素は
    ///   列最大値そのものなので閾値 `<= 1` なら必ず通り、Markowitz 数 0 で即終了)、
    ///   ここでも同じ列を採る。
    /// - 列シングルトンのピボットは他の行を消去しないので、行の値・行次数は
    ///   変わらず、`L` も出ない。変わるのはピボット行の各列の次数 (引退による
    ///   `-1`) だけで、それを `eliminate` と同じく列昇順に `refresh_column` する。
    /// - ピボット行の引退は列ミラーの遅延削除 ([`KernelMatrix::col_retire`]) で済ませ、
    ///   前処理の最後に 1 回だけ詰め直す (1 要素ごとの二分探索と詰め直しを省く)。
    ///   以後の通常の消去が見る列ミラーは従来どおり活性行だけになる。
    /// - 行シングルトン採用・行探索 (経路が変わる任意機能) が有効なら何もしない。
    fn peel_column_singletons(&mut self, row_perm: &mut [usize], col_perm: &mut [usize], u_entries: &mut Vec<(usize, usize, f64)>) -> usize {
        if self.row_singleton_rel >= 0.0 || self.row_search > 0 || !(self.threshold <= 1.0) {
            return 0;
        }
        // 採ったステップ数 (= 次のステップ番号)
        let mut step = 0usize;
        loop {
            let pj = {
                let dense = &self.initially_dense;
                match self.col_buckets[1].iter().find(|&&j| !dense[j]) {
                    Some(&j) => j,
                    None => break,
                }
            };
            // 列 `pj` の唯一の活性行 (列ミラーには引退行も残っている)
            let pi = {
                let row_used = &self.row_used;
                self.mat.col(pj).iter().map(|&r| r as usize).find(|&r| !row_used[r]).expect("degree-1 column must have an active row")
            };
            self.row_used[pi] = true;
            self.col_used[pj] = true;
            row_perm[step] = pi;
            col_perm[step] = pj;
            self.remove_from_bucket_row(pi);
            self.remove_from_bucket_col(pj);

            // `U` の行 = ピボット行のうちピボット列と未使用列の非ゼロ (通常経路のスナップショットと同一)。
            let (s, len) = (self.mat.row_start[pi], self.mat.row_len[pi]);
            for p in s..s + len {
                let (j, v) = (self.mat.row_idx[p] as usize, self.mat.row_val[p]);
                if v != 0.0 && (j == pj || !self.col_used[j]) {
                    u_entries.push((step, j, v));
                }
            }
            // `eliminate` の可観測な効果: ピボット列を空にし、ピボット行を引退させる
            // (他の列の次数・バケットを列昇順に更新)。
            self.mat.col_clear(pj);
            for p in s..s + len {
                let j = self.mat.row_idx[p] as usize;
                if j == pj {
                    continue;
                }
                if !self.col_used[j] {
                    self.mat.col_retire(j);
                }
                self.refresh_column(j);
            }

            // プロファイル累計も `find_best_pivot` の即決ステップと同じに数える。
            self.prof_steps += 1;
            self.prof_trivial += 1;
            self.prof_candidates += 1;
            step += 1;
        }
        if step > 0 {
            self.mat.compact_cols(&self.row_used);
        }
        step
    }

    /// ピボット `(pi, pj)` (値 `pivot_val`) で列 `pj` を他の全活性行から消去する。
    /// ガウス消去の算術と次数・バケット更新を 1 パスで同時に行い
    /// (分けると「行がまだ `pj` を含むか」の判定が壊れる)、行 `pi` を全列の
    /// 活性行リストから引退させる。`L` 用の `(行, 乗数)` は [`Self::l_out`] に残す。
    ///
    /// - `pivot_row_snapshot`: ピボット行の `(列, 値)` (列昇順) のコピー。
    /// - 通常経路はピボット行の密散布 (`ElimScratch::wval`) を使い、影響行の
    ///   全要素に `v - mult * wval[j]` を分岐なしで適用する。フィルインがある行だけ
    ///   後方から挿入する。`ENOMOTO_LU_INPLACE_ELIM=0` または乗数が非有限なら
    ///   単純マージ ([`Self::merge_row`])。どちらも因子はビット一致。
    /// - 値や次数が変わりうる列はピボット行の列だけなので、それらを昇順に
    ///   refresh すればよい。
    fn eliminate(&mut self, pi: usize, pj: usize, pivot_val: f64, pivot_row_snapshot: &[(usize, f64)]) {
        let mut sc = std::mem::take(&mut self.scratch);
        sc.begin();

        sc.affected.clear();
        sc.affected.extend(self.mat.col(pj).iter().map(|&r| r as usize).filter(|&i| i != pi));

        // ピボット行の非ピボット活性要素を散布する。
        // ピボット行の非ピボット要素数
        let mut p_act = 0usize;
        for &(j, v) in pivot_row_snapshot {
            if j != pj {
                sc.wval[j] = v;
                p_act += 1;
            }
        }
        for ai in 0..sc.affected.len() {
            let i = sc.affected[ai];
            let s0 = self.mat.row_start[i];
            let len = self.mat.row_len[i];
            // 行 `i` のラン内でのピボット列の位置
            let Some(p0) = sorted_find(&self.mat.row_idx[s0..s0 + len], pj as u32) else { continue };
            let aij = self.mat.row_val[s0 + p0];
            if aij == 0.0 {
                continue;
            }
            // 消去乗数 (= `L` の要素)
            let mult = aij / pivot_val;
            sc.l_out.push((i, mult));

            sc.col_add.clear();
            sc.col_del.clear();
            let new_len = if self.inplace_elim && mult.is_finite() {
                // 全要素をその場で更新: ピボット行の列なら消去、それ以外は
                // `wval[j] == 0.0` なので値は `v` のまま (ピボット列も同様で、
                // 直後に除去)。分岐なし・圧縮なし。
                let idx = &self.mat.row_idx[s0..s0 + len];
                let val = &mut self.mat.row_val[s0..s0 + len];
                let wval = &sc.wval[..];
                // この行が持っていたピボット行の列の数
                let mut found = 0usize;
                // 更新で厳密に 0 になった要素数
                let mut zeros = 0usize;
                debug_assert!(idx.iter().all(|&j| (j as usize) < wval.len()));
                for (v, &j) in val.iter_mut().zip(idx) {
                    // SAFETY: カーネル内の列番号はすべて `< m` (`KernelMatrix::new`
                    // が正方性を検査し、フィルインはピボット行の列のコピーのみ)、
                    // `wval` の長さは `m`。
                    let pv = unsafe { *wval.get_unchecked(j as usize) };
                    found += (pv != 0.0) as usize;
                    let nv = *v - mult * pv;
                    *v = nv;
                    zeros += (nv == 0.0) as usize;
                }
                // ピボット列の要素 (と稀な厳密相殺) を順序を保って除く。
                let w = if zeros == 0 {
                    self.mat.row_idx.copy_within(s0 + p0 + 1..s0 + len, s0 + p0);
                    self.mat.row_val.copy_within(s0 + p0 + 1..s0 + len, s0 + p0);
                    len - 1
                } else {
                    // 相殺された要素は行と列ミラーから抜ける (列昇順)。
                    let mut w = 0usize;
                    for a in 0..len {
                        let (j, v) = (self.mat.row_idx[s0 + a], self.mat.row_val[s0 + a]);
                        if a == p0 {
                            continue;
                        }
                        if v == 0.0 {
                            sc.col_del.push(j as usize);
                            continue;
                        }
                        self.mat.row_idx[s0 + w] = j;
                        self.mat.row_val[s0 + w] = v;
                        w += 1;
                    }
                    w
                };
                self.mat.row_len[i] = w;
                if found == p_act {
                    w
                } else {
                    // フィルイン: この行が持っていなかったピボット行の列
                    // (相殺された要素は持っていた扱い)。
                    let stamp = sc.next_rstamp();
                    for &j in &self.mat.row_idx[s0..s0 + w] {
                        sc.rmark[j as usize] = stamp;
                    }
                    for &j in &sc.col_del {
                        sc.rmark[j] = stamp;
                    }
                    sc.fill_idx.clear();
                    sc.fill_val.clear();
                    for &(jb, vb) in pivot_row_snapshot {
                        if jb != pj && sc.rmark[jb] != stamp {
                            let nv = -mult * vb;
                            if nv != 0.0 {
                                sc.fill_idx.push(jb as u32);
                                sc.fill_val.push(nv);
                                sc.col_add.push(jb);
                            }
                        }
                    }
                    // フィルイン数
                    let nf = sc.fill_idx.len();
                    if nf > 0 {
                        self.mat.ensure_row_cap(i, w + nf);
                        let s = self.mat.row_start[i];
                        let idx = &mut self.mat.row_idx[s..s + w + nf];
                        let val = &mut self.mat.row_val[s..s + w + nf];
                        // 最後のフィルインから順に、未移動の前半を二分探索して
                        // 挿入位置を決め、その後ろのブロックを一括で後ろへずらす
                        // (各要素の移動は高々 1 回)。
                        let mut a = w;
                        for f in (0..nf).rev() {
                            let fj = sc.fill_idx[f];
                            let pos = idx[..a].partition_point(|&c| c < fj);
                            idx.copy_within(pos..a, pos + f + 1);
                            val.copy_within(pos..a, pos + f + 1);
                            idx[pos + f] = fj;
                            val[pos + f] = sc.fill_val[f];
                            a = pos;
                        }
                        self.mat.row_len[i] = w + nf;
                    }
                    w + nf
                }
            } else {
                self.merge_row(&mut sc, i, pj, mult, pivot_row_snapshot)
            };

            for k in 0..sc.col_del.len() {
                self.mat.col_remove(sc.col_del[k], i);
            }
            for k in 0..sc.col_add.len() {
                self.mat.col_insert(sc.col_add[k], i);
            }
            self.update_row_degree(i, new_len);
        }
        for &(j, _) in pivot_row_snapshot {
            sc.wval[j] = 0.0;
        }
        self.mat.col_clear(pj);

        // 行 pi は新しいピボット行として引退する。まだ触れている他の列から除き、
        // それらの列の次数・バケットを列昇順で更新する (順序は観測可能:
        // `refresh_column` はバケット末尾に追加し、`find_best_pivot` は格納順に
        // 走査して同点を先勝ちで決める)。
        sc.pi_cols.clear();
        {
            let (idx, _) = self.mat.row(pi);
            sc.pi_cols.extend(idx.iter().map(|&j| j as usize).filter(|&j| j != pj));
        }
        for k in 0..sc.pi_cols.len() {
            let j = sc.pi_cols[k];
            self.mat.col_remove(j, pi);
            self.refresh_column(j);
        }

        self.scratch = sc;
    }

    /// 行 `i` とピボット行の単純な 2 ポインタマージ (`eliminate` の散布ループの
    /// 参照実装)。`ENOMOTO_LU_INPLACE_ELIM=0` 時と、乗数が非有限
    /// (`v - mult * 0.0` が `v` にならない) 時に使う。新しい行長を返す。
    fn merge_row(&mut self, sc: &mut ElimScratch, i: usize, pj: usize, mult: f64, pivot_row_snapshot: &[(usize, f64)]) -> usize {
        sc.merged_idx.clear();
        sc.merged_val.clear();
        let (idx, val) = self.mat.row(i);
        // a: 行 `i` 側の位置, b: ピボット行側の位置
        let (mut a, mut b) = (0usize, 0usize);
        while a < idx.len() && b < pivot_row_snapshot.len() {
            let (ja, va) = (idx[a] as usize, val[a]);
            let (jb, vb) = pivot_row_snapshot[b];
            if ja < jb {
                // この行にしかない列 (ピボット済み列を含む。そのまま残す)。
                sc.merged_idx.push(ja as u32);
                sc.merged_val.push(va);
                a += 1;
            } else if jb < ja {
                // フィルイン。
                if jb != pj {
                    let new_val = -mult * vb;
                    if new_val != 0.0 {
                        sc.merged_idx.push(jb as u32);
                        sc.merged_val.push(new_val);
                        sc.col_add.push(jb);
                    }
                }
                b += 1;
            } else {
                // ピボット列の要素はこの行から抜ける (ミラーは `col_clear(pj)` で一括除去)。
                if ja != pj {
                    let new_val = va - mult * vb;
                    if new_val == 0.0 {
                        sc.col_del.push(ja);
                    } else {
                        sc.merged_idx.push(ja as u32);
                        sc.merged_val.push(new_val);
                    }
                }
                a += 1;
                b += 1;
            }
        }
        while a < idx.len() {
            sc.merged_idx.push(idx[a]);
            sc.merged_val.push(val[a]);
            a += 1;
        }
        while b < pivot_row_snapshot.len() {
            let (jb, vb) = pivot_row_snapshot[b];
            if jb != pj {
                let new_val = -mult * vb;
                if new_val != 0.0 {
                    sc.merged_idx.push(jb as u32);
                    sc.merged_val.push(new_val);
                    sc.col_add.push(jb);
                }
            }
            b += 1;
        }
        let n = sc.merged_idx.len();
        self.mat.ensure_row_cap(i, n);
        let s = self.mat.row_start[i];
        self.mat.row_idx[s..s + n].copy_from_slice(&sc.merged_idx);
        self.mat.row_val[s..s + n].copy_from_slice(&sc.merged_val);
        self.mat.row_len[i] = n;
        n
    }
}

/// [`LuFactors::l_solve_sparse_into`] (Gilbert-Peierls 疎前進代入) 用の
/// 永続スクラッチ。定常状態では確保ゼロ。呼び出し側が専用の疎求解バッファと
/// 共に 1 つ保持し、毎回の FTRAN で再利用する ([`FtLu::solve_into`] の
/// `scratch` とは共有しないこと)。
pub struct GpScratch {
    /// 今回の呼び出しの到達集合に既に入ったステップの印 ([`EpochMarks`]、
    /// エポックを進めるだけで O(1) リセット、一周対策込み)。
    visited: EpochMarks,
    /// DFS の作業スタック (`m` が数千になりうるので再帰でなく反復)。
    stack: Vec<usize>,
    /// DFS の起点 (右辺の非ゼロに対応するステップ)。
    seeds: Vec<usize>,
    /// 今回の呼び出しで集めた到達集合 (後でソートされる)。
    reach: Vec<usize>,
    /// C5: 疎な入力列 FTRAN (`solve_sparse_into_capture` / `_pair_capture` /
    /// `_triple_capture`) の前に呼び出し側が設定し、その `U` 段を超疎に実行
    /// させる ([`FtLu::u_solve_hyper`])。既定 `false`。
    pub u_hyper: bool,
    /// 超疎 `U` 段で一覧に入れたスロットの印。
    u_marks: EpochMarks,
    /// 超疎 `U` 段 (DFS 版) の DFS スタック。
    u_stack: Vec<usize>,
    /// 超疎 `U` 段 (DFS 版) で適用すべき到達 `u_seq` 位置。
    u_pos: Vec<usize>,
    /// 超疎 `U` 段 (ヒープ版) の `u_seq` 位置の最大ヒープ。
    u_heap: BinaryHeap<u32>,
    /// 到達した全スロット (= `U` 後に非ゼロになりうるスロット)。
    u_list: Vec<usize>,
    /// 直前の `R` eta 適用で値が変わった行 (超疎 `U` 段の起点の候補。`reach` と合わせて
    /// `U` 段入力の非ゼロ位置をすべて含む)。`R` eta を適用する側が毎回作り直す。
    r_seeds: Vec<usize>,
    /// 疎な `R` eta 適用の作業領域 (追加策 R)。
    r_work: RSparseWork,
}

impl GpScratch {
    /// 次数 `m` 用のスクラッチを作る。
    pub fn new(m: usize) -> Self {
        GpScratch {
            visited: EpochMarks::new(m),
            stack: Vec::new(),
            seeds: Vec::new(),
            reach: Vec::new(),
            u_hyper: false,
            u_marks: EpochMarks::new(m),
            u_stack: Vec::new(),
            u_pos: Vec::new(),
            u_heap: BinaryHeap::new(),
            u_list: Vec::new(),
            r_seeds: Vec::new(),
            r_work: RSparseWork::default(),
        }
    }
}

/// [`FtLu::apply_r_sparse`] の作業領域 (eta 番号の印と最小ヒープ)。[`GpScratch`] が持つ。
#[derive(Default)]
struct RSparseWork {
    /// eta 番号ごとの印 (`stamp[k] == epoch` なら選択済み)。
    stamp: Vec<u32>,
    /// 現在のエポック。
    epoch: u32,
    /// 未処理の eta 番号の最小ヒープ。
    heap: BinaryHeap<Reverse<u32>>,
}

/// [`FtLu::u_solve_hyper`] の結果。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum UHyper {
    /// 試さなかった (`x` は未変更。呼び出し側が全走査する)。
    NotTried,
    /// 超疎に解いた (非ゼロになりうる位置は `gp.u_list`)。
    Hyper,
    /// 途中から全走査に切り替えて解き終えた (非ゼロ位置は不明)。
    Full,
}

/// 1 回の分解結果 `P_row B P_col = L U` (更新前の素の因子)。
/// すべて消去ステップ番号の空間で格納される。
#[derive(Clone)]
pub struct LuFactors {
    /// 行列の次数。
    pub m: usize,
    /// `l_col[s]`: `L` の列 `s` の対角より下の要素 `(row_step, 乗数)`。
    /// 分解後は不変で FTRAN/BTRAN の `L` 段で毎回読むため、フラットな
    /// [`crate::sparse::CscMat`] に [`crate::sparse::CscBuilder`] で直接追記して作る
    /// (各分解は `L` の列をステップ昇順に出力するので中間 `Vec<Vec>` もコピーも不要)。
    pub l_col: crate::sparse::CscMat,
    /// `u_row[s]`: `U` の行 `s` の要素 `(col_step, 値)` (`col_step >= s`、
    /// 対角 `col_step == s` を含む)。`FtLu::new` が `u_seq` を作るときに
    /// 1 回読むだけなので `Vec<Vec>` のまま。
    pub u_row: Vec<Vec<(usize, f64)>>,
    /// [`Self::l_col`] の行優先ミラー (HiGHS の `lr_start/lr_index/lr_value`)。
    /// `l_row.row(r)` は `L` の行 `r` の非ゼロ `(s, 乗数)` (すべて `s < r`)。
    /// BTRAN の `L^{-T}` をスキャッタ形式で解き、`w[s] == 0.0` のステップを
    /// 丸ごと飛ばすために使う。[`CscMat::to_csr`] の計数ソートで構築。
    pub l_row: CsrMat,
    /// `row_perm[s]` = ステップ `s` のピボット行 (元の行番号)。
    pub row_perm: Vec<usize>,
    /// `col_perm[s]` = ステップ `s` のピボット列 (元の基底スロット番号)。
    pub col_perm: Vec<usize>,
    /// `col_perm` の逆写像: `col_perm_inv[元の列] = ステップ`。
    pub col_perm_inv: Vec<usize>,
    /// `row_perm` の逆写像: `row_perm_inv[元の行] = ステップ`。疎な右辺から
    /// [`l_solve_sparse_into`] の到達集合探索を `row_perm` の O(m) 走査なしで
    /// 始めるのに使う。
    pub row_perm_inv: Vec<usize>,
}

/// 入力の非ゼロ密度が `DENSE_INPUT_FRACTION * m^2` を超えるか
/// (超えれば Markowitz をやめて稠密 LU [`factorize_dense_faer`] に回す)。
fn is_dense_input(m: usize, rows_in: &[Vec<(usize, f64)>]) -> bool {
    if m == 0 {
        return false;
    }
    let nnz: usize = rows_in.iter().map(|r| r.len()).sum();
    nnz as f64 > tunable!("ENOMOTO_T_DENSE_INPUT_FRACTION", DENSE_INPUT_FRACTION, f64) * (m as f64) * (m as f64)
}

/// `faer` による稠密部分ピボット LU (`PA = LU`、行ピボットのみなので
/// `col_perm` は恒等)。結果を [`LuFactors`] に変換するので、下流 (`FtLu`、
/// 各種求解、FT 更新) はどの経路で作られたかを意識しない。
/// 対角に厳密な 0 があれば `None` (特異)。
fn factorize_dense_faer(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Option<LuFactors> {
    // 稠密化した入力行列 (重複座標は加算)
    let mut a = faer::Mat::<f64>::zeros(m, m);
    for (i, row) in rows_in.iter().enumerate() {
        for &(j, v) in row {
            a[(i, j)] += v;
        }
    }

    let lu = faer::linalg::solvers::PartialPivLu::new(a.as_ref());
    let l = lu.compute_l();
    let u = lu.compute_u();
    let perm = lu.row_permutation();
    let (fwd, _inv) = perm.arrays();
    // `PA = LU`: `PA` の行 `step` は元の行 `fwd[step]` で、このモジュールの
    // `row_perm[step]` と同じ意味 (`l_solve_into` は `z[s] = rhs[row_perm[s]]`)。
    let row_perm: Vec<usize> = fwd.iter().map(|&idx| usize::from(idx)).collect();
    let col_perm: Vec<usize> = (0..m).collect();

    for step in 0..m {
        if u[(step, step)] == 0.0 {
            return None;
        }
    }

    let mut row_perm_inv = vec![0usize; m];
    let mut col_perm_inv = vec![0usize; m];
    for step in 0..m {
        row_perm_inv[row_perm[step]] = step;
        col_perm_inv[col_perm[step]] = step;
    }

    let mut l_build = CscBuilder::new(m);
    for step in 0..m {
        for row_step in (step + 1)..m {
            let v = l[(row_step, step)];
            if v != 0.0 {
                l_build.push(row_step, v);
            }
        }
        l_build.end_column();
    }
    let l_col = l_build.build();
    let mut u_row: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for step in 0..m {
        for col_step in step..m {
            let v = u[(step, col_step)];
            if v != 0.0 {
                u_row[step].push((col_step, v));
            }
        }
    }

    let l_row = build_l_row(&l_col, m);
    Some(LuFactors { m, l_col, l_row, u_row, row_perm, col_perm, col_perm_inv, row_perm_inv })
}

/// 診断用 (`ENOMOTO_DEBUG_BLOCK_SIZES`、本番経路では未使用)。この基底行列を
/// Dulmage-Mendelsohn SCC 分解したときのブロックサイズ分布を標準エラーに出す。
fn debug_print_block_sizes(m: usize, rows_in: &[Vec<(usize, f64)>]) {
    // 行ごとの列番号リスト (二部グラフの隣接)
    let adj: Vec<Vec<usize>> = rows_in.iter().map(|row| row.iter().map(|&(j, _)| j).collect()).collect();
    let decomp_t0 = std::time::Instant::now();
    let decomp = crate::graph::dulmage_mendelsohn_blocks_topological(&adj, m);
    let decomp_us = decomp_t0.elapsed().as_micros();
    match decomp {
        None => eprintln!("BLOCK_SIZES m={m} no-perfect-matching decomp_us={decomp_us}"),
        Some((blocks, _)) => {
            let mut sizes: Vec<usize> = blocks.iter().map(|b| b.len()).collect();
            sizes.sort_unstable();
            let n_blocks = sizes.len();
            let le10 = sizes.iter().filter(|&&s| s <= 10).count();
            let le50 = sizes.iter().filter(|&&s| s <= 50).count();
            let rows_le10: usize = sizes.iter().filter(|&&s| s <= 10).sum();
            let rows_le50: usize = sizes.iter().filter(|&&s| s <= 50).sum();
            let max = sizes.last().copied().unwrap_or(0);
            eprintln!(
                "BLOCK_SIZES m={m} n_blocks={n_blocks} max_block={max} blocks_le10={le10} blocks_le50={le50} rows_in_blocks_le10={rows_le10}({:.1}%) rows_in_blocks_le50={rows_le50}({:.1}%) decomp_us={decomp_us}",
                100.0 * rows_le10 as f64 / m as f64,
                100.0 * rows_le50 as f64 / m as f64,
            );
        }
    }
}

/// 非ゼロ数が `DENSE_COL_FRACTION * m` を超える列 (ほぼ稠密な「トレンド列」)
/// の一覧を返す。`factorize` が [`factorize_bordered`] を試すかの判定に使う
/// (`O(nnz)` の 1 パス)。
fn detect_border_columns(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Vec<usize> {
    let mut col_degree = vec![0usize; m];
    for row in rows_in {
        for &(j, v) in row {
            if v != 0.0 {
                col_degree[j] += 1;
            }
        }
    }
    let threshold = tunable!("ENOMOTO_T_DENSE_COL_FRACTION", DENSE_COL_FRACTION, f64) * m as f64;
    (0..m).filter(|&j| col_degree[j] as f64 > threshold).collect()
}

/// 対角基底行列を直接分解する (`L = I`, `U = B`, 置換なし)。初期の全スラック
/// 基底 (符号付き単位行列) 用。`rows_in` が厳密に対角 (各行に自分の列の
/// 非ゼロ 1 個だけ) でなければ `None` を返すので、呼び出し側は [`factorize`]
/// にフォールバックすること。
pub fn factorize_diagonal(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Option<LuFactors> {
    // 確保なしで先に形状を検査する (途中の再分解でも毎回試され、ほぼ失敗するため)。
    let diagonal = rows_in.iter().enumerate().all(|(i, row)| matches!(row.as_slice(), [(j, v)] if *j == i && *v != 0.0));
    if !diagonal {
        return None;
    }
    let u_row: Vec<Vec<(usize, f64)>> = rows_in.iter().enumerate().map(|(i, row)| vec![(i, row[0].1)]).collect();
    let identity: Vec<usize> = (0..m).collect();
    // `L` は単位行列: `m` 列すべて空。
    let l_col = CscMat::empty(m, m);
    let l_row = build_l_row(&l_col, m);
    Some(LuFactors {
        m,
        l_col,
        l_row,
        u_row,
        row_perm: identity.clone(),
        col_perm: identity.clone(),
        col_perm_inv: identity.clone(),
        row_perm_inv: identity,
    })
}

// ---------------------------------------------------------------------
// ピボット順の再利用 ("rebuild"、HiGHS `HFactorRefactor.cpp` 相当)
// ---------------------------------------------------------------------

/// [`factorize_reusing_order`] を試みた回数 (`ENOMOTO_PROF_PHASES_EXT` 診断用)。
pub(crate) static PROF_REBUILD_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);
/// そのうち使える分解が得られた (採用された) 回数。
pub(crate) static PROF_REBUILD_ACCEPTED: AtomicUsize = AtomicUsize::new(0);
/// 試行 (採用・棄却とも) に費やした累計ナノ秒。
pub(crate) static PROF_REBUILD_NS: AtomicUsize = AtomicUsize::new(0);
/// 棄却理由: 処理中の列の残り部分行列に数値的に使える要素が無かった回数。
pub(crate) static PROF_REBUILD_FAIL_SINGULAR: AtomicUsize = AtomicUsize::new(0);
/// 棄却理由: 因子が [`REBUILD_FILL_LIMIT`] を超えて膨らんだ回数。
pub(crate) static PROF_REBUILD_FAIL_FILL: AtomicUsize = AtomicUsize::new(0);
/// 記録されたピボット *行* が使えず選び直したステップ数
/// ([`factorize_reusing_order`] 参照)。
pub(crate) static PROF_REBUILD_ROW_REPICKS: AtomicUsize = AtomicUsize::new(0);
/// 棄却後のバックオフ ([`factorize_reusing`]) で再利用を試みなかった再分解の回数。
pub(crate) static PROF_REBUILD_BACKOFF_SKIPS: AtomicUsize = AtomicUsize::new(0);

/// 再利用の fill 上限倍率 ([`REBUILD_FILL_LIMIT`]、環境変数
/// `ENOMOTO_REUSE_FILL_LIMIT` で上書き可)。再分解ごとに 1 回読む。
fn reuse_fill_limit() -> f64 {
    env_str!("ENOMOTO_REUSE_FILL_LIMIT").and_then(|v| v.parse::<f64>().ok()).unwrap_or(REBUILD_FILL_LIMIT)
}

/// 採用された再利用分解の `L`+`U` 非ゼロ数の累計。
pub(crate) static PROF_REBUILD_ACCEPTED_NNZ: AtomicUsize = AtomicUsize::new(0);
/// 通常の (Markowitz) 分解の `L`+`U` 非ゼロ数の累計。
pub(crate) static PROF_FULL_NNZ: AtomicUsize = AtomicUsize::new(0);
/// 通常の (Markowitz) 分解の回数。
pub(crate) static PROF_FULL_COUNT: AtomicUsize = AtomicUsize::new(0);

/// ピボット順再利用が有効か (既定 有効、`ENOMOTO_REUSE_PIVOT_ORDER=0` で無効)。
/// 再分解ごとに 1 回読む。
fn reuse_pivot_order_enabled() -> bool {
    !matches!(env_str!("ENOMOTO_REUSE_PIVOT_ORDER"), Some("0") | Some("false"))
}

/// `rows_in` を、可能なら **`prev` のピボット順を再利用して** 再分解し、
/// だめなら通常の Markowitz [`factorize`] にフォールバックする
/// (HiGHS の `HFactor::build()` が `rebuild()` を先に試すのに相当)。
///
/// - `prev`: 直前の分解 (`None` なら普通に分解する)。
/// - 再利用の対象は通常の疎 Markowitz ケースのみ (稠密入力・境界付き経路は除外)。
/// - 棄却が続くと指数バックオフで一定回数だけ再利用を試みない
///   (`reuse_fail_streak` / `reuse_skips_left`)。
/// - 特異なら `None`。
pub fn factorize_reusing(m: usize, rows_in: &[Vec<(usize, f64)>], prev: Option<&FtLu>) -> Option<FtLu> {
    let Some(prev) = prev else {
        return factorize(m, rows_in).map(FtLu::new);
    };
    // 境界列と稠密判定はここで 1 回だけ計算して `factorize_routed` に渡す。
    let border = detect_border_columns(m, rows_in);
    let dense = is_dense_input(m, rows_in);
    // 再利用の対象になるか (有効かつ非稠密かつ境界付き経路でない)
    let eligible = reuse_pivot_order_enabled() && m > 0 && !dense && !border_wanted(m, border.len());
    if eligible && prev.reuse_skips_left == 0 {
        let t0 = std::time::Instant::now();
        PROF_REBUILD_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        // 再利用分解が許される因子の最大非ゼロ数
        let max_nnz = (reuse_fill_limit() * prev.fill_baseline as f64) as usize + m;
        let rebuilt = factorize_reusing_order(m, rows_in, &prev.base.col_perm, &prev.base.row_perm, max_nnz);
        PROF_REBUILD_NS.fetch_add(t0.elapsed().as_nanos() as usize, Ordering::Relaxed);
        if let Some(lu) = rebuilt {
            PROF_REBUILD_ACCEPTED.fetch_add(1, Ordering::Relaxed);
            let mut ft = FtLu::new(lu);
            PROF_REBUILD_ACCEPTED_NNZ.fetch_add(ft.fill_baseline, Ordering::Relaxed);
            // 基準 fill は最後の *通常* 分解のものを引き継ぐ
            // (再利用の連鎖で上限が少しずつ上がらないように)。
            ft.fill_baseline = prev.fill_baseline;
            return Some(ft);
        }
    }
    // 再利用が棄却されたかバックオフ中。通常の Markowitz 分解を行い、
    // その fill が新しい基準になる (`FtLu::new` が設定)。
    let mut ft = FtLu::new(factorize_routed(m, rows_in, &border, dense)?);
    PROF_FULL_NNZ.fetch_add(ft.fill_baseline, Ordering::Relaxed);
    PROF_FULL_COUNT.fetch_add(1, Ordering::Relaxed);
    if eligible {
        if prev.reuse_skips_left > 0 {
            ft.reuse_fail_streak = prev.reuse_fail_streak;
            ft.reuse_skips_left = prev.reuse_skips_left - 1;
        } else {
            // 棄却は固まって起きるので指数バックオフする (2, 4, 8, ... 回見送り、
            // 上限 `REUSE_MAX_BACKOFF`)。採用されれば新しい `FtLu` で 0 に戻る。
            ft.reuse_fail_streak = prev.reuse_fail_streak.saturating_add(1);
            ft.reuse_skips_left = (1u32 << ft.reuse_fail_streak.min(tunable!("ENOMOTO_T_REUSE_BACKOFF_SHIFT_CAP", REUSE_BACKOFF_SHIFT_CAP, u32))).min(tunable!("ENOMOTO_T_REUSE_MAX_BACKOFF", REUSE_MAX_BACKOFF, u32));
            PROF_REBUILD_BACKOFF_SKIPS.fetch_add(ft.reuse_skips_left as usize, Ordering::Relaxed);
        }
    }
    Some(ft)
}

/// [`factorize`] がこの入力を [`factorize_bordered`] に回すか (同じ判定を切り出したもの)。
/// [`factorize_reusing`] は境界付きになる基底には再利用を使わない。
#[allow(dead_code)]
fn wants_bordered(m: usize, rows_in: &[Vec<(usize, f64)>]) -> bool {
    border_wanted(m, detect_border_columns(m, rows_in).len())
}

/// 境界列数 `k` が分かっているときの [`wants_bordered`]
/// (`0 < k <= BORDER_MAX_COUNT` かつ `k <= BORDER_MAX_FRACTION * m`)。
#[inline]
fn border_wanted(m: usize, k: usize) -> bool {
    k > 0 && k <= BORDER_MAX_COUNT && (k as f64) <= tunable!("ENOMOTO_T_BORDER_MAX_FRACTION", BORDER_MAX_FRACTION, f64) * m as f64
}

/// `rows_in` を **与えられた列順** で分解する (探索しない)。ステップ `s` は列
/// `pivot_col[s]` を消去し、行 `pivot_row_hint[s]` がまだ数値的に使えればそれを、
/// 使えなければ閾値を満たす行のうち未処理列に残る要素数が最少のものを
/// ピボット行にする。
///
/// 左視 (left-looking、Gilbert-Peierls 形、HiGHS `rebuild()` と同形): 列を読み込み、
/// ここまでの `L` で前進代入し、ピボット済み行の要素を `U` の列 `s`、ピボット行の
/// 要素を対角、未ピボット行の要素を `L` の列 `s` に振り分ける。
/// 活性部分行列は保持しない。
///
/// 列順だけを再生し、行順は再生しない (FT 更新で入れ替わった列では記録された
/// ピボット行が数値的に空になるため)。
///
/// 次の場合 `None` (呼び出し側が [`factorize`] にフォールバック):
/// 順序が正しい置換でない / ある列の残り部分行列に [`REBUILD_MIN_PIVOT`] を
/// 超える要素が無い (特異・数値的に使い果たした基底) / 因子の非ゼロ数が
/// `max_nnz` を超えた。
fn factorize_reusing_order(
    m: usize,
    rows_in: &[Vec<(usize, f64)>],
    pivot_col: &[usize],
    pivot_row_hint: &[usize],
    max_nnz: usize,
) -> Option<LuFactors> {
    if m == 0 || pivot_col.len() != m || pivot_row_hint.len() != m || rows_in.len() != m {
        return None;
    }
    {
        let mut seen = vec![false; m];
        for &j in pivot_col {
            if j >= m || seen[j] {
                return None;
            }
            seen[j] = true;
        }
    }
    // `MarkowitzState::new` と同じく分解全体で 1 回だけ読む
    // (引き上げた閾値がこの経路にも届くように)。
    let threshold = pivot_threshold();

    // 入力 (行優先) の列優先コピー。基底スロット番号で添字付けする、分解後に捨てる
    // 一時配列 (`O(nnz)` の計数ソート)。
    let mut col_start = vec![0usize; m + 1];
    for row in rows_in.iter() {
        for &(j, _) in row {
            if j >= m {
                return None;
            }
            col_start[j + 1] += 1;
        }
    }
    for j in 0..m {
        col_start[j + 1] += col_start[j];
    }
    let mut col_row = vec![0usize; col_start[m]];
    let mut col_val = vec![0.0f64; col_start[m]];
    {
        let mut cursor = col_start.clone();
        for (i, row) in rows_in.iter().enumerate() {
            for &(j, v) in row {
                col_row[cursor[j]] = i;
                col_val[cursor[j]] = v;
                cursor[j] += 1;
            }
        }
    }

    // 現在の列を元の行番号で展開した密作業ベクトル
    let mut work = vec![0.0f64; m];
    // `work` で値を書いた行の一覧
    let mut touched: Vec<usize> = Vec::new();
    // 行が `touched` に入っているか
    let mut in_touched = vec![false; m];
    // 前進代入で処理すべき `L` のステップ (昇順に取り出す最小ヒープ)
    let mut heap: BinaryHeap<Reverse<usize>> = BinaryHeap::new();
    // ステップがヒープに入っているか
    let mut queued = vec![false; m];

    // 作業中の `L` の列 (元の行番号で保持し、最後にステップ番号へ一括変換する。
    // 行のステップはピボットに選ばれるまで分からないため)。
    let mut l_entries: Vec<(usize, f64)> = Vec::new();
    // `l_entries` における各列の開始オフセット
    let mut l_offsets: Vec<usize> = Vec::with_capacity(m + 1);
    l_offsets.push(0);
    let mut u_row: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    // これまでに作った `L`+`U` の非ゼロ数
    let mut lu_nnz = 0usize;

    // `row_step[r]`: 行 `r` がピボットに選ばれたステップ (未選択なら `usize::MAX`)。
    // 最後に `LuFactors::row_perm_inv` になる。
    let mut row_step = vec![usize::MAX; m];
    let mut row_perm = vec![0usize; m];

    // `row_remaining[r]`: 行 `r` の要素のうち、まだ到達していない列にあるものの数
    // (入力行列上の Markowitz 数の行側)。記録された行が使えないときの行選択で、
    // フィル抑制のために使う。
    let mut row_remaining: Vec<u32> = rows_in.iter().map(|row| row.len() as u32).collect();

    for s in 0..m {
        let pj = pivot_col[s];
        for p in col_start[pj]..col_start[pj + 1] {
            row_remaining[col_row[p]] = row_remaining[col_row[p]].saturating_sub(1);
        }

        for p in col_start[pj]..col_start[pj + 1] {
            let r = col_row[p];
            work[r] += col_val[p];
            if !in_touched[r] {
                in_touched[r] = true;
                touched.push(r);
            }
            // `L` の列が空のステップはヒープに積まない (取り出しても何もしないため。
            // ステップ `t < s` の `l_offsets[t + 1]` は確定済み)。
            let t = row_step[r];
            if t != usize::MAX && !queued[t] && l_offsets[t + 1] > l_offsets[t] {
                queued[t] = true;
                heap.push(Reverse(t));
            }
        }

        // ここまでの `L` の `s` 列に対して `L y = A[:, pj]` を前進代入する。
        // `L` の列 `t` はステップ `t` で未ピボットだった行 (ステップ > t) にしか
        // 書かないので、昇順が正しいトポロジカル順になる (グラフが構築中のため
        // DFS でなくヒープを使う)。
        while let Some(Reverse(t)) = heap.pop() {
            queued[t] = false;
            let y = work[row_perm[t]];
            if y == 0.0 {
                continue;
            }
            for idx in l_offsets[t]..l_offsets[t + 1] {
                let (r, mult) = l_entries[idx];
                if !in_touched[r] {
                    in_touched[r] = true;
                    touched.push(r);
                }
                work[r] -= mult * y;
                let t2 = row_step[r];
                if t2 != usize::MAX && !queued[t2] && l_offsets[t2 + 1] > l_offsets[t2] {
                    queued[t2] = true;
                    heap.push(Reverse(t2));
                }
            }
        }

        // ピボット選択: まず残り部分行列 (ステップ未割当の行) でのこの列の
        // 最大絶対値を求める。
        let mut best_r = usize::MAX;
        let mut best_abs = 0.0f64;
        for &r in &touched {
            if row_step[r] == usize::MAX {
                let a = work[r].abs();
                if a > best_abs {
                    best_abs = a;
                    best_r = r;
                }
            }
        }
        if best_abs < tunable!("ENOMOTO_T_REBUILD_MIN_PIVOT", REBUILD_MIN_PIVOT, f64) {
            PROF_REBUILD_FAIL_SINGULAR.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        let hint = pivot_row_hint[s];
        // 記録された行が未使用で閾値 (`threshold * best_abs`) を満たせばそれを使う。
        let pi = if hint < m && row_step[hint] == usize::MAX && work[hint].abs() >= threshold * best_abs {
            hint
        } else {
            // 記録された行が使えない: 閾値を満たす行のうち、未到達列に残る要素数が
            // 最少の行 (同数ならピボット絶対値が大きい方) を選び直す。
            PROF_REBUILD_ROW_REPICKS.fetch_add(1, Ordering::Relaxed);
            let floor = threshold * best_abs;
            let mut pick = best_r;
            let mut pick_deg = u32::MAX;
            let mut pick_abs = 0.0f64;
            for &r in &touched {
                if row_step[r] != usize::MAX {
                    continue;
                }
                let a = work[r].abs();
                if a < floor {
                    continue;
                }
                let deg = row_remaining[r];
                if deg < pick_deg || (deg == pick_deg && a > pick_abs) {
                    pick_deg = deg;
                    pick_abs = a;
                    pick = r;
                }
            }
            pick
        };
        let pivot = work[pi];
        row_step[pi] = s;
        row_perm[s] = pi;
        // `U` の行 `s` は入力行 `pi` の残りの列から埋まるので、その長さで予約する。
        u_row[s].reserve(rows_in[pi].len());

        for &r in &touched {
            let v = work[r];
            work[r] = 0.0;
            in_touched[r] = false;
            if v == 0.0 {
                continue;
            }
            let t = row_step[r];
            if t == usize::MAX {
                l_entries.push((r, v / pivot));
            } else {
                // ここでは `t <= s` (以前のピボット行か、今選んだピボット行)。
                u_row[t].push((s, v));
            }
            lu_nnz += 1;
        }
        touched.clear();
        l_offsets.push(l_entries.len());
        if lu_nnz > max_nnz {
            PROF_REBUILD_FAIL_FILL.fetch_add(1, Ordering::Relaxed);
            return None;
        }
    }

    let mut col_perm_inv = vec![0usize; m];
    for (s, &j) in pivot_col.iter().enumerate() {
        col_perm_inv[j] = s;
    }
    let mut l_build = CscBuilder::with_capacity(m, m, l_entries.len());
    for s in 0..m {
        for idx in l_offsets[s]..l_offsets[s + 1] {
            let (r, mult) = l_entries[idx];
            l_build.push(row_step[r], mult);
        }
        l_build.end_column();
    }
    let l_col = l_build.build();
    let l_row = l_col.to_csr();

    Some(LuFactors {
        m,
        l_col,
        l_row,
        u_row,
        row_perm,
        col_perm: pivot_col.to_vec(),
        col_perm_inv,
        row_perm_inv: row_step,
    })
}

/// `m x m` の疎行列 (疎行 `(列, 値)` の並び) を LU 分解する。数値的に特異
/// (あるステップで許容できるピボットが残っていない) なら `None`。
/// 境界列が適量あれば [`factorize_bordered`]、稠密なら [`factorize_dense_faer`]、
/// それ以外は Markowitz 消去に振り分ける。
pub fn factorize(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Option<LuFactors> {
    let border = detect_border_columns(m, rows_in);
    factorize_routed(m, rows_in, &border, is_dense_input(m, rows_in))
}

/// [`factorize`] の本体。振り分けに使う [`detect_border_columns`] の結果
/// (`border`) と [`is_dense_input`] の結果 (`dense`) を呼び出し側が計算済みで渡す。
fn factorize_routed(m: usize, rows_in: &[Vec<(usize, f64)>], border: &[usize], dense: bool) -> Option<LuFactors> {
    if env_str!("ENOMOTO_DEBUG_BLOCK_SIZES").is_some() {
        debug_print_block_sizes(m, rows_in);
    }
    // 境界付き分解は `is_dense_input` より *前に* 試す (境界列だけで全体密度の
    // 判定を超える入力でも、その範囲では境界付きが稠密 LU より速いため)。
    // 疎フェーズでピボットが揃わなければ `None` が返り、下の通常経路に落ちる。
    if border_wanted(m, border.len()) {
        if let Some(lu) = factorize_bordered(m, rows_in, border) {
            return Some(lu);
        }
    }
    factorize_flat_markowitz_routed(m, rows_in, dense)
}

/// 解析専用: 分解の入力 1 件を `<dir>/lu_dump.bin` に追記する (`m`、続いて
/// 各行の `len` と `(列, f64 のビット)` の組。すべてリトルエンディアン `u64`)。
/// `lu_kernel_bench` が同じ行列を再生するために使う。
fn dump_lu_input(dir: &std::path::Path, m: usize, rows_in: &[Vec<(usize, f64)>]) {
    use std::io::Write;
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(&(m as u64).to_le_bytes());
    for row in rows_in {
        buf.extend_from_slice(&(row.len() as u64).to_le_bytes());
        for &(j, v) in row {
            buf.extend_from_slice(&(j as u64).to_le_bytes());
            buf.extend_from_slice(&v.to_bits().to_le_bytes());
        }
    }
    let _ = std::fs::create_dir_all(dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("lu_dump.bin")) {
        let _ = f.write_all(&buf);
    }
}

/// 稠密判定を自分で行う [`factorize_flat_markowitz_routed`] (境界付き経路を通らない)。
#[cfg_attr(not(test), allow(dead_code))]
fn factorize_flat_markowitz(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Option<LuFactors> {
    factorize_flat_markowitz_routed(m, rows_in, is_dense_input(m, rows_in))
}

/// B3 診断: 残りを稠密分解に切り替えた分解の回数。
pub(crate) static PROF_DENSE_SWITCH: AtomicUsize = AtomicUsize::new(0);
/// B3 診断: 稠密分解に渡した残りブロックのサイズ `k` の累計。
pub(crate) static PROF_DENSE_SWITCH_ROWS: AtomicUsize = AtomicUsize::new(0);

/// Markowitz 消去による分解本体 (`dense` なら [`factorize_dense_faer`] に回す)。
/// 各ステップで稠密でないピボット列を優先し、無ければ制限なしで探す。
/// ピボットが見つからなければ `None` (特異)。
fn factorize_flat_markowitz_routed(m: usize, rows_in: &[Vec<(usize, f64)>], dense: bool) -> Option<LuFactors> {
    if dense {
        return factorize_dense_faer(m, rows_in);
    }
    if let Some(dir) = std::env::var_os("ENOMOTO_DUMP_LU_DIR") {
        dump_lu_input(std::path::Path::new(&dir), m, rows_in);
    }
    let mut state = MarkowitzState::new(m, rows_in);

    let mut row_perm = vec![0usize; m];
    let mut col_perm = vec![0usize; m];

    // `L` の要素 `(元の行, ピボットステップ, 乗数)`
    let mut l_entries: Vec<(usize, usize, f64)> = Vec::new();
    // `U` の要素 `(ピボットステップ, 元の列, 値)`
    let mut u_entries: Vec<(usize, usize, f64)> = Vec::new();
    // `ENOMOTO_DEBUG_ELIMINATE_COST`: 消去とスナップショットの時間を出力する
    let debug_eliminate_cost = env_str!("ENOMOTO_DEBUG_ELIMINATE_COST").is_some();
    let mut eliminate_ns: u128 = 0;
    let mut snapshot_ns: u128 = 0;

    // ピボット行のコピー (分解全体で 1 つのバッファを再利用)
    let mut pivot_row_snapshot: Vec<(usize, f64)> = Vec::new();
    // B3 (`ENOMOTO_LU_DENSE_SWITCH`、既定 `0` = off、経路が変わる): 活性部分行列の
    // 密度が `k^2` のこの割合に達したら (`k` = 残り行数、`ENOMOTO_LU_DENSE_SWITCH_MIN`
    // 以上)、残りの `k x k` ブロックを稠密分解する。
    let dense_switch = tunable!("ENOMOTO_LU_DENSE_SWITCH", DENSE_SWITCH_FRACTION, f64);
    let dense_switch_min = tunable!("ENOMOTO_LU_DENSE_SWITCH_MIN", DENSE_SWITCH_MIN_ROWS, usize);
    // 先頭の列シングルトンをまとめて採る (経路は下のループと同一)。B3 が有効なら
    // ステップ 0 の密度判定を先に行う必要があるので使わない。
    let peeled = if dense_switch > 0.0 { 0 } else { state.peel_column_singletons(&mut row_perm, &mut col_perm, &mut u_entries) };
    for step in peeled..m {
        if dense_switch > 0.0 && step % DENSE_SWITCH_CHECK_INTERVAL == 0 && m - step >= dense_switch_min {
            // 残りの行数
            let k = m - step;
            // 活性部分行列の非ゼロ数
            let active: usize = (0..m).filter(|&i| !state.row_used[i]).map(|i| state.mat.row_len[i]).sum();
            if active as f64 >= dense_switch * (k as f64) * (k as f64) {
                // 残りの行・列 (元の番号)
                let rows_r: Vec<usize> = (0..m).filter(|&i| !state.row_used[i]).collect();
                let cols_c: Vec<usize> = (0..m).filter(|&j| !state.col_used[j]).collect();
                debug_assert_eq!(rows_r.len(), k);
                debug_assert_eq!(cols_c.len(), k);
                // 元の列番号 -> 残りブロック内の列番号
                let mut col_local = vec![usize::MAX; m];
                for (l, &j) in cols_c.iter().enumerate() {
                    col_local[j] = l;
                }
                let sub: Vec<Vec<(usize, f64)>> = rows_r
                    .iter()
                    .map(|&i| state.row(i).filter(|&(j, v)| v != 0.0 && col_local[j] != usize::MAX).map(|(j, v)| (col_local[j], v)).collect())
                    .collect();
                let dlu = factorize_dense_faer(k, &sub)?;
                for s in 0..k {
                    debug_assert_eq!(dlu.col_perm[s], s);
                    row_perm[step + s] = rows_r[dlu.row_perm[s]];
                    col_perm[step + s] = cols_c[s];
                    for &(cs, val) in &dlu.u_row[s] {
                        u_entries.push((step + s, cols_c[cs], val));
                    }
                    for &(rs, mult) in dlu.l_col.col(s) {
                        l_entries.push((rows_r[dlu.row_perm[rs]], step + s, mult));
                    }
                }
                PROF_DENSE_SWITCH.fetch_add(1, Ordering::Relaxed);
                PROF_DENSE_SWITCH_ROWS.fetch_add(k, Ordering::Relaxed);
                break;
            }
        }
        // 稠密でないピボット列があれば Markowitz 数に関係なくそれを優先し、
        // 残りがすべて `initially_dense` のときだけ制限なしで探す
        // (真に特異なら後者が `None` を返す)。
        let (pi, pj) = match state.find_best_pivot(true) {
            Some(p) => p,
            None => {
                PROF_DENSE_FALLBACK_STEPS.fetch_add(1, Ordering::Relaxed);
                state.find_best_pivot(false)?
            }
        };

        state.row_used[pi] = true;
        state.col_used[pj] = true;
        row_perm[step] = pi;
        col_perm[step] = pj;

        state.remove_from_bucket_row(pi);
        state.remove_from_bucket_col(pj);

        let pivot_val = state.value_at(pi, pj).unwrap();
        let snap_t0 = if debug_eliminate_cost { Some(std::time::Instant::now()) } else { None };
        // ピボット行のうち、ピボット列と未使用列の非ゼロだけを写す。
        pivot_row_snapshot.clear();
        pivot_row_snapshot.extend(state.row(pi).filter(|&(j, v)| v != 0.0 && (j == pj || !state.col_used[j])));
        if let Some(t0) = snap_t0 {
            snapshot_ns += t0.elapsed().as_nanos();
        }

        for &(j, v) in &pivot_row_snapshot {
            u_entries.push((step, j, v));
        }

        let elim_t0 = if debug_eliminate_cost { Some(std::time::Instant::now()) } else { None };
        state.eliminate(pi, pj, pivot_val, &pivot_row_snapshot);
        for &(i, mult) in state.l_out() {
            l_entries.push((i, step, mult));
        }
        if let Some(t0) = elim_t0 {
            eliminate_ns += t0.elapsed().as_nanos();
        }
    }
    if debug_eliminate_cost {
        eprintln!(
            "ELIMINATE_COST m={m} eliminate_ms={:.3} snapshot_ms={:.3} l_nnz={} u_nnz={} avg_row_fill={:.1}",
            eliminate_ns as f64 / 1e6,
            snapshot_ns as f64 / 1e6,
            l_entries.len(),
            u_entries.len(),
            u_entries.len() as f64 / m as f64,
        );
    }

    let mut row_perm_inv = vec![0usize; m];
    let mut col_perm_inv = vec![0usize; m];
    for step in 0..m {
        row_perm_inv[row_perm[step]] = step;
        col_perm_inv[col_perm[step]] = step;
    }

    // `l_entries` はピボットステップ昇順にまとまっているので、最終の圧縮
    // バッファへ直接追記できる。
    debug_assert!(l_entries.windows(2).all(|w| w[0].1 <= w[1].1), "L entries must be grouped by ascending pivot step");
    let mut l_build = CscBuilder::with_capacity(m, m, l_entries.len());
    let mut next = 0usize;
    for step in 0..m {
        while next < l_entries.len() && l_entries[next].1 == step {
            let (orig_row, _, mult) = l_entries[next];
            l_build.push(row_perm_inv[orig_row], mult);
            next += 1;
        }
        l_build.end_column();
    }
    let l_col = l_build.build();
    // `u_entries` もステップ昇順にまとまっているので、各行をちょうどの容量で 1 回で作る。
    debug_assert!(u_entries.windows(2).all(|w| w[0].0 <= w[1].0), "U entries must be grouped by ascending pivot step");
    let mut u_row: Vec<Vec<(usize, f64)>> = Vec::with_capacity(m);
    let mut next = 0usize;
    for step in 0..m {
        let start = next;
        while next < u_entries.len() && u_entries[next].0 == step {
            next += 1;
        }
        u_row.push(u_entries[start..next].iter().map(|&(_, orig_col, val)| (col_perm_inv[orig_col], val)).collect());
    }

    let l_row = build_l_row(&l_col, m);
    Some(LuFactors { m, l_col, l_row, u_row, row_perm, col_perm, col_perm_inv, row_perm_inv })
}

/// 境界付き (Schur 補行列) 分解。`border` 列 ([`detect_border_columns`]) を
/// 通常の疎 Markowitz フェーズから完全に除外し (ピボット候補にもならず、
/// フィルも受けない)、最後に小さな稠密 `k x k` Schur 補行列で解決する。
///
/// ```text
/// A = [ A_SS  A_SD ]    L = [ L_SS   0  ]    U = [ U_SS  U_SD ]
///     [ A_DS  A_DD ]        [ L_DS  L_DD]        [  0    U_DD ]
/// ```
///
/// `S` は非境界列と疎フェーズが選んだ `m - k` 行、`D` は `k` 本の境界列と疎フェーズが
/// 触れない `k` 行。`L_SS`/`U_SS`/`L_DS` は境界列を除いた入力に対する 1 回の
/// Markowitz 消去で得られ、`U_SD = L_SS^{-1} A_SD` は境界列ごとの前進代入、
/// Schur 補行列 `A_DD - L_DS U_SD` は [`factorize_dense_faer`] で分解する。
///
/// 疎フェーズが非境界列すべてのピボットを見つける前に行き詰まった場合、または
/// Schur 補行列が数値的に特異な場合は `None` (呼び出し側が
/// [`factorize_flat_markowitz`] にフォールバックする)。
fn factorize_bordered(m: usize, rows_in: &[Vec<(usize, f64)>], border: &[usize]) -> Option<LuFactors> {
    // 境界列の本数
    let k = border.len();
    if k == 0 || k >= m {
        return None;
    }
    // 疎フェーズのステップ数
    let n_sparse = m - k;

    let mut is_border = vec![false; m];
    for &j in border {
        is_border[j] = true;
    }

    // Markowitz 状態を作る前に入力から境界列を完全に取り除く。要素 0 個の列は
    // バケット 0 にしか入らず、`find_best_pivot` の `for deg_col in 1..` は
    // そこを訪れないので、疎フェーズが境界列に散布することはない。
    let sparse_rows: Vec<Vec<(usize, f64)>> =
        rows_in.iter().map(|row| row.iter().copied().filter(|&(j, v)| v != 0.0 && !is_border[j]).collect()).collect();

    let mut state = MarkowitzState::new(m, &sparse_rows);
    // 境界付きの疎フェーズでは行シングルトン採用・行探索を必ず無効にする
    // (`row_singleton_rel` 参照)。
    state.row_singleton_rel = -1.0;
    state.row_search = 0;

    let mut row_perm = vec![usize::MAX; m];
    let mut col_perm = vec![usize::MAX; m];
    // 疎フェーズで `eliminate` が触れた全行 (境界行を含む) の
    // `(元の行, ピボットステップ, 乗数)` = `L_SS` と `L_DS` を合わせたもの。
    let mut l_entries: Vec<(usize, usize, f64)> = Vec::new();
    // `(ピボットステップ, 元の列, 値)` = `U_SS` の要素 (後で `U_SD` も追記)。
    let mut u_entries: Vec<(usize, usize, f64)> = Vec::new();

    // ピボット行のコピー (分解全体で 1 つのバッファを再利用)
    let mut pivot_row_snapshot: Vec<(usize, f64)> = Vec::new();
    // 先頭の列シングルトンをまとめて採る (経路は下のループと同一。境界列は
    // バケット 0 にいるので採られず、採れる数は高々 `n_sparse`)。
    let peeled = state.peel_column_singletons(&mut row_perm, &mut col_perm, &mut u_entries);
    for step in peeled..n_sparse {
        let (pi, pj) = state.find_best_pivot(true).or_else(|| state.find_best_pivot(false))?;

        state.row_used[pi] = true;
        state.col_used[pj] = true;
        row_perm[step] = pi;
        col_perm[step] = pj;
        state.remove_from_bucket_row(pi);
        state.remove_from_bucket_col(pj);

        let pivot_val = state.value_at(pi, pj).unwrap();
        pivot_row_snapshot.clear();
        pivot_row_snapshot.extend(state.row(pi).filter(|&(j, v)| v != 0.0 && (j == pj || !state.col_used[j])));
        for &(j, v) in &pivot_row_snapshot {
            u_entries.push((step, j, v));
        }
        state.eliminate(pi, pj, pivot_val, &pivot_row_snapshot);
        for &(i, mult) in state.l_out() {
            l_entries.push((i, step, mult));
        }
    }

    // 疎フェーズでピボットにならなかった `k` 行 (元の行番号)
    let border_rows: Vec<usize> = (0..m).filter(|&i| !state.row_used[i]).collect();
    assert_eq!(border_rows.len(), k, "sparse phase must leave exactly `k` rows unpivoted");

    // 疎フェーズ部分の `row_perm` の逆写像 (境界行は `usize::MAX`)
    let mut row_perm_inv = vec![usize::MAX; m];
    for step in 0..n_sparse {
        row_perm_inv[row_perm[step]] = step;
    }
    // 元の行番号 -> 境界行内の局所番号
    let mut border_row_local = vec![usize::MAX; m];
    for (local, &r) in border_rows.iter().enumerate() {
        border_row_local[r] = local;
    }

    // `l_col_ss[s]`: `L_SS` の列 `s` (行は疎フェーズのステップ番号)。
    // この関数内の前進代入専用の一時データ。
    let mut l_col_ss: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n_sparse];
    // `l_ds[local]`: 境界行 `local` の各疎ステップに対する乗数 (= `L_DS[local, :]`)。
    let mut l_ds: Vec<Vec<(usize, f64)>> = vec![Vec::new(); k];
    for &(orig_row, step, mult) in &l_entries {
        let ri = row_perm_inv[orig_row];
        if ri != usize::MAX {
            l_col_ss[step].push((ri, mult));
        } else {
            l_ds[border_row_local[orig_row]].push((step, mult));
        }
    }

    // 元の列番号 -> `border` 内の添字 (境界列でなければ `usize::MAX`)
    let mut border_index = vec![usize::MAX; m];
    for (idx, &j) in border.iter().enumerate() {
        border_index[j] = idx;
    }

    // `border_col_rows[idx]`: 境界列 `border[idx]` の元の `(行, 値)` 全部
    // (1 回の `O(nnz)` パスで集め、`A_SD` と `A_DD` の両方に使う)。
    let mut border_col_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); k];
    for (i, row) in rows_in.iter().enumerate() {
        for &(j, v) in row {
            if v == 0.0 {
                continue;
            }
            let idx = border_index[j];
            if idx != usize::MAX {
                border_col_rows[idx].push((i, v));
            }
        }
    }

    // Schur 補行列。まず `A_DD` (元の値) を入れ、下で `L_DS * U_SD` を引く
    // (標準の境界付き LU 恒等式そのもの)。
    let mut schur = vec![vec![0.0f64; k]; k];
    for (idx, rows_for_col) in border_col_rows.iter().enumerate() {
        for &(orig_row, v) in rows_for_col {
            let local_e = border_row_local[orig_row];
            if local_e != usize::MAX {
                schur[local_e][idx] += v;
            }
        }
    }

    for (idx, rows_for_col) in border_col_rows.iter().enumerate() {
        // この境界列の `A_SD` 部分 (疎ステップ空間)。前進代入後は `U_SD` の列になる。
        let mut y = vec![0.0f64; n_sparse];
        for &(orig_row, v) in rows_for_col {
            let s = row_perm_inv[orig_row];
            if s != usize::MAX {
                y[s] += v;
            }
        }
        // `L_SS y = y` をその場で前進代入 (単位下三角、ステップ順)。
        for s in 0..n_sparse {
            if y[s] == 0.0 {
                continue;
            }
            for &(row_step, mult) in &l_col_ss[s] {
                y[row_step] -= mult * y[s];
            }
        }
        for (s, &val) in y.iter().enumerate() {
            if val != 0.0 {
                u_entries.push((s, border[idx], val));
            }
        }
        for local_e in 0..k {
            if l_ds[local_e].is_empty() {
                continue;
            }
            let mut acc = 0.0;
            for &(step, mult) in &l_ds[local_e] {
                if y[step] != 0.0 {
                    acc += mult * y[step];
                }
            }
            schur[local_e][idx] -= acc;
        }
    }

    let schur_rows: Vec<Vec<(usize, f64)>> = schur
        .iter()
        .map(|row| row.iter().enumerate().filter(|&(_, &v)| v != 0.0).map(|(j, &v)| (j, v)).collect())
        .collect();
    // Schur 補行列の稠密 LU
    let border_lu = factorize_dense_faer(k, &schur_rows)?;

    for s in 0..k {
        debug_assert_eq!(border_lu.col_perm[s], s, "factorize_dense_faer's own col_perm is always identity");
        row_perm[n_sparse + s] = border_rows[border_lu.row_perm[s]];
        col_perm[n_sparse + s] = border[s];
    }

    let mut row_perm_inv_full = vec![0usize; m];
    let mut col_perm_inv_full = vec![0usize; m];
    for step in 0..m {
        row_perm_inv_full[row_perm[step]] = step;
        col_perm_inv_full[col_perm[step]] = step;
    }

    // `l_entries` はステップ `0..n_sparse` を昇順に、境界ブロックの列は
    // `n_sparse..m` を昇順に続けるので、`L` 全体を列順のまま圧縮バッファへ直接書ける。
    debug_assert!(l_entries.windows(2).all(|w| w[0].1 <= w[1].1), "L entries must be grouped by ascending pivot step");
    let mut l_build = CscBuilder::with_capacity(m, m, l_entries.len() + border_lu.l_col.nnz());
    let mut next = 0usize;
    for step in 0..n_sparse {
        while next < l_entries.len() && l_entries[next].1 == step {
            let (orig_row, _, mult) = l_entries[next];
            l_build.push(row_perm_inv_full[orig_row], mult);
            next += 1;
        }
        l_build.end_column();
    }
    for s in 0..k {
        for &(row_step, mult) in border_lu.l_col.col(s) {
            l_build.push(n_sparse + row_step, mult);
        }
        l_build.end_column();
    }
    let l_col = l_build.build();
    let mut u_row: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for (step, orig_col, val) in u_entries {
        u_row[step].push((col_perm_inv_full[orig_col], val));
    }
    for s in 0..k {
        for &(col_step, val) in &border_lu.u_row[s] {
            u_row[n_sparse + s].push((n_sparse + col_step, val));
        }
    }

    let l_row = build_l_row(&l_col, m);
    Some(LuFactors { m, l_col, l_row, u_row, row_perm, col_perm, col_perm_inv: col_perm_inv_full, row_perm_inv: row_perm_inv_full })
}

impl LuFactors {
    /// ステップ `step` の `U` 対角値 (無ければ `0.0`)。
    #[allow(dead_code)]
    fn u_diag(&self, step: usize) -> f64 {
        self.u_row[step]
            .iter()
            .find(|&&(c, _)| c == step)
            .map(|&(_, v)| v)
            .unwrap_or(0.0)
    }

    /// 2 本の右辺に対する [`Self::l_solve_into`] を `L` の 1 回の走査で行う
    /// ([`FtLu::solve_into_pair_capture`] 用)。各ベクトルが受ける演算と順序は
    /// 個別呼び出しと同じなので結果はビット一致。
    ///
    /// `active`: `L` の列が非空のステップ (昇順、[`FtLu::l_active`])。空列のステップは
    /// 何もしないので、それだけを訪れる。
    fn l_solve_into_pair(&self, active: &[u32], rhs_a: &[f64], rhs_b: &[f64], za: &mut [f64], zb: &mut [f64]) {
        let m = self.m;
        for s in 0..m {
            let r = self.row_perm[s];
            za[s] = rhs_a[r];
            zb[s] = rhs_b[r];
        }
        for &s in active {
            let s = s as usize;
            let xa = za[s];
            let xb = zb[s];
            match (xa != 0.0, xb != 0.0) {
                (false, false) => {}
                (true, true) => {
                    for &(row_step, mult) in self.l_col.col(s) {
                        za[row_step] -= mult * xa;
                        zb[row_step] -= mult * xb;
                    }
                }
                (true, false) => {
                    for &(row_step, mult) in self.l_col.col(s) {
                        za[row_step] -= mult * xa;
                    }
                }
                (false, true) => {
                    for &(row_step, mult) in self.l_col.col(s) {
                        zb[row_step] -= mult * xb;
                    }
                }
            }
        }
    }

    /// [`Self::l_solve_into_pair`] に 3 本目の右辺 `c` (BFRT の合成フリップ列、
    /// [`FtLu::solve_into_triple_capture`]) を加えたもの。3 本とも非ゼロの列は
    /// 1 回で処理し、それ以外は `a`/`b` を pair の論理、`c` を単独で処理する。
    /// 結果はそれぞれ個別呼び出しとビット一致。
    #[allow(clippy::too_many_arguments)]
    fn l_solve_into_triple(&self, active: &[u32], rhs_a: &[f64], rhs_b: &[f64], rhs_c: &[f64], za: &mut [f64], zb: &mut [f64], zc: &mut [f64]) {
        let m = self.m;
        for s in 0..m {
            let r = self.row_perm[s];
            za[s] = rhs_a[r];
            zb[s] = rhs_b[r];
            zc[s] = rhs_c[r];
        }
        for &s in active {
            let s = s as usize;
            let xa = za[s];
            let xb = zb[s];
            let xc = zc[s];
            if xa != 0.0 && xb != 0.0 && xc != 0.0 {
                for &(row_step, mult) in self.l_col.col(s) {
                    za[row_step] -= mult * xa;
                    zb[row_step] -= mult * xb;
                    zc[row_step] -= mult * xc;
                }
                continue;
            }
            match (xa != 0.0, xb != 0.0) {
                (false, false) => {}
                (true, true) => {
                    for &(row_step, mult) in self.l_col.col(s) {
                        za[row_step] -= mult * xa;
                        zb[row_step] -= mult * xb;
                    }
                }
                (true, false) => {
                    for &(row_step, mult) in self.l_col.col(s) {
                        za[row_step] -= mult * xa;
                    }
                }
                (false, true) => {
                    for &(row_step, mult) in self.l_col.col(s) {
                        zb[row_step] -= mult * xb;
                    }
                }
            }
            if xc != 0.0 {
                for &(row_step, mult) in self.l_col.col(s) {
                    zc[row_step] -= mult * xc;
                }
            }
        }
    }

    /// `L` だけを通す部分 FTRAN (ステップ空間): `L z = P_row rhs` を解き、
    /// 呼び出し側の `z` (長さ `m`) に書く。`z[s]` が厳密に 0 のステップは列ごと
    /// 飛ばす (Hall & McKinnon 2000 §4.2 の超疎 FTRAN)。
    /// `active` は [`Self::l_solve_into_pair`] と同じ。
    fn l_solve_into(&self, active: &[u32], rhs: &[f64], z: &mut [f64]) {
        let m = self.m;
        for s in 0..m {
            z[s] = rhs[self.row_perm[s]];
        }
        for &s in active {
            let s = s as usize;
            if z[s] == 0.0 {
                continue;
            }
            for &(row_step, mult) in self.l_col.col(s) {
                z[row_step] -= mult * z[s];
            }
        }
    }

    /// `L` を通す Gilbert-Peierls 疎前進代入。右辺の非ゼロ `(元の行, 値)` を
    /// 直接受け取り、`l_col` のステップ間辺を DFS して到達集合 (非ゼロになりうる
    /// ステップ) を求め、その集合に限って [`l_solve_into`] と同じ消去を行う。
    /// 右辺・到達集合が `m` に比べて小さいときに有利。
    ///
    /// `l_col[s]` の行ステップは常に `> s` なので、到達集合を昇順ソートする
    /// だけで正しいトポロジカル順になる。
    ///
    /// **前提条件**: 入口で `z` は全 0 であること (再ゼロ化は
    /// [`FtLu::solve_sparse_into`] が返却前に行う)。
    fn l_solve_sparse_into(&self, rhs_sparse: &[(usize, f64)], z: &mut [f64], scratch: &mut GpScratch) {
        scratch.seeds.clear();
        for &(orig_row, v) in rhs_sparse {
            if v == 0.0 {
                continue;
            }
            let s = self.row_perm_inv[orig_row];
            z[s] = v;
            scratch.seeds.push(s);
        }
        self.l_solve_gp_seeded(z, scratch);
    }

    /// 非ゼロの *ステップ* (`rhs[row_perm[s]] != 0.0` となる `s`) が分かっている
    /// 密な `rhs` に対する [`Self::l_solve_into`]。`steps` はちょうどそれらを
    /// (任意順で) 列挙すること。`z` は自分でクリアする (`clean` なら `z` は入口で全 0 と
    /// 分かっているのでクリアを省く)。到達集合版の消去を行い、
    /// 結果は `l_solve_into` と (ゼロの符号を除き) ビット一致。
    fn l_solve_steps_into(&self, rhs: &[f64], steps: &[usize], z: &mut [f64], scratch: &mut GpScratch) {
        z.fill(0.0);
        scratch.seeds.clear();
        for &s in steps {
            let v = rhs[self.row_perm[s]];
            debug_assert!(v != 0.0);
            z[s] = v;
            scratch.seeds.push(s);
        }
        self.l_solve_gp_seeded(z, scratch);
    }

    /// `clean` (`z` が入口で全 0 と分かっている) ならクリアを省く [`Self::l_solve_steps_into`]
    /// (融合 FTRAN の記録付き版用)。
    fn l_solve_steps_into_clean(&self, rhs: &[f64], steps: &[usize], z: &mut [f64], scratch: &mut GpScratch, clean: bool) {
        if clean {
            debug_assert!(z.iter().all(|&v| v == 0.0));
        } else {
            z.fill(0.0);
        }
        scratch.seeds.clear();
        for &s in steps {
            let v = rhs[self.row_perm[s]];
            debug_assert!(v != 0.0);
            z[s] = v;
            scratch.seeds.push(s);
        }
        self.l_solve_gp_seeded(z, scratch);
    }

    /// 上の 2 つの GP 入口が共有する DFS と昇順消去。`scratch.seeds` と、
    /// 起点位置の `z` は設定済みであること。
    fn l_solve_gp_seeded(&self, z: &mut [f64], scratch: &mut GpScratch) {
        scratch.visited.begin();
        scratch.reach.clear();
        for i in 0..scratch.seeds.len() {
            let seed = scratch.seeds[i];
            if scratch.visited.is_marked(seed) {
                continue;
            }
            scratch.visited.mark(seed);
            scratch.stack.push(seed);
            while let Some(node) = scratch.stack.pop() {
                scratch.reach.push(node);
                for &(next, _) in self.l_col.col(node) {
                    if !scratch.visited.is_marked(next) {
                        scratch.visited.mark(next);
                        scratch.stack.push(next);
                    }
                }
            }
        }
        scratch.reach.sort_unstable();

        for &s in &scratch.reach {
            if z[s] == 0.0 {
                continue;
            }
            for &(row_step, mult) in self.l_col.col(s) {
                z[row_step] -= mult * z[s];
            }
        }
    }

    /// BTRAN の仕上げ (列優先ギャザー形式): `U^{-T}`/`R` 適用済みのステップ空間
    /// ベクトル `w` に `L^{-T}` をその場で適用し、元の行番号に戻して `y` に書く。
    ///
    /// **`ENOMOTO_BTRAN_L_SCATTER=0` の A/B 用と単体テストの参照実装としてのみ
    /// 残している**。本番の BTRAN は [`Self::l_transpose_solve_scatter_into`] を使う。
    #[cfg_attr(not(test), allow(dead_code))]
    fn l_transpose_solve_gather_into(&self, w: &mut [f64], y: &mut [f64]) {
        self.l_transpose_gather_core(w);
        permute_btran_out(&self.row_perm, w, y);
    }

    /// [`Self::l_transpose_solve_gather_into`] から最後の置換を除いたもの。
    #[inline]
    fn l_transpose_gather_core(&self, w: &mut [f64]) {
        let m = self.m;
        for s in (0..m).rev() {
            for &(row_step, mult) in self.l_col.col(s) {
                if w[row_step] == 0.0 {
                    continue;
                }
                w[s] -= mult * w[row_step];
            }
        }
    }

    /// BTRAN の `L^{-T}` 段を行優先ミラー [`LuFactors::l_row`] 経由の
    /// スキャッタ形式で解き、元の行番号に戻して `y` に書く。`w[s]` が内側ループ
    /// 唯一の乗数なので、`w[s] == 0.0` ならステップ全体を飛ばせる (超疎化)。
    /// HiGHS の `btranL` と同じ手法。
    ///
    /// ギャザー形式とはビット一致しない (各 `w[s]` への積の加算順が逆のため、
    /// 最終 ulp が異なりうる)。
    #[cfg_attr(not(test), allow(dead_code))]
    fn l_transpose_solve_scatter_into(&self, w: &mut [f64], y: &mut [f64]) {
        self.l_transpose_scatter_core(w);
        permute_btran_out(&self.row_perm, w, y);
    }

    /// [`Self::l_transpose_solve_scatter_into`] から最後の置換を除いたもの。
    #[inline]
    fn l_transpose_scatter_core(&self, w: &mut [f64]) {
        let m = self.m;
        for s in (0..m).rev() {
            let ws = w[s];
            if ws == 0.0 {
                continue;
            }
            for &(k, mult) in self.l_row.row(s) {
                w[k] -= mult * ws;
            }
        }
    }

    /// 因子 (`P_row B P_col = LU`) を使って `B x = rhs` を解く (テスト・参照用)。
    #[allow(dead_code)]
    pub fn solve(&self, rhs: &[f64]) -> Vec<f64> {
        let m = self.m;
        // rhs' = P_row rhs
        let mut z: Vec<f64> = (0..m).map(|s| rhs[self.row_perm[s]]).collect();
        // 前進代入: L z = rhs' (単位下三角、ステップ順)
        for s in 0..m {
            for &(row_step, mult) in self.l_col.col(s) {
                z[row_step] -= mult * z[s];
            }
        }
        // 後退代入: U x' = z
        let mut xp = vec![0.0; m];
        for s in (0..m).rev() {
            let mut acc = z[s];
            for &(col_step, v) in &self.u_row[s] {
                if col_step != s {
                    acc -= v * xp[col_step];
                }
            }
            xp[s] = acc / self.u_diag(s);
        }
        // x[col_perm[s]] = xp[s]
        let mut x = vec![0.0; m];
        for s in 0..m {
            x[self.col_perm[s]] = xp[s];
        }
        x
    }

    /// `B^T y = rhs` を解く (テスト・参照用)。
    #[allow(dead_code)]
    pub fn solve_transpose(&self, rhs: &[f64]) -> Vec<f64> {
        let m = self.m;
        // rhs2 = P_col^-1 rhs、すなわち rhs2[s] = rhs[col_perm[s]]
        let mut z: Vec<f64> = (0..m).map(|s| rhs[self.col_perm[s]]).collect();
        // 前進代入: U^T z2 = rhs2 (ステップ順で下三角)。`U` は行格納なので
        // スキャッタで解く: ステップ `s` の時点で `z[s]` には `k < s` からの寄与が
        // すべて引かれているので、対角で割って確定させ、`col_step > s` へ散布する。
        for s in 0..m {
            z[s] /= self.u_diag(s);
            for &(col_step, v) in &self.u_row[s] {
                if col_step != s {
                    z[col_step] -= v * z[s];
                }
            }
        }
        // 後退代入: L^T w = z (ステップ順で単位上三角)
        let mut w = z;
        for s in (0..m).rev() {
            for &(row_step, mult) in self.l_col.col(s) {
                w[s] -= mult * w[row_step];
            }
        }
        // y[row_perm[s]] = w[s]
        let mut y = vec![0.0; m];
        for s in 0..m {
            y[self.row_perm[s]] = w[s];
        }
        y
    }

}

// ---------------------------------------------------------------------
// Forrest-Tomlin 更新
// ---------------------------------------------------------------------
//
// Forrest & Tomlin (1972) / Huangfu & Hall, ERGO-13-001 (2013) §2.1 に従う。
//
// 列置換 `B̄ = B + (a_q - B e_p) e_p^T` を固定の `B = LU` で
//   `L^{-1} B̄ = U + (ã_q - u_p) e_p^T = U'`   (`ã_q = L^{-1} a_q`、部分 FTRAN)
// と書き直す。これで `U` の列 `p` が「スパイク」になるので、行変換
// `R^{-1} = I - e_p r^T` (`r^T = ū_p^T U^{-1}`、部分 BTRAN `ẽ_p^T = e_p^T U^{-1}`
// から `r = -u_pp · ẽ_p`、第 `p` 成分は 0) で行 `p` を消して三角性を回復する。
//
// `L` は更新で変わらない。`U` はスロットごとの列 eta (ピボット + 非対角) の
// *列* として保持し、置換されたスロットの eta は元の位置から除いて末尾に追加する
// (更新後の FTRAN/BTRAN はこの順序に従う必要がある)。各更新は `R` 行 eta を
// 1 つ作り、`B_k^{-1} = U_k^{-1} R_k^{-1} ... R_1^{-1} L^{-1}` の順で適用する。
//
// eta の非対角部は、fill が `DENSE_ETA_FRACTION * m` 以下なら疎、超えたら長さ `m`
// の密配列で持つ ([`EtaFile`])。

/// 非対角要素を持たない `U` eta (対角ピボットだけ)。
#[derive(Clone, Copy)]
struct SEta {
    /// 基底スロット。
    slot: usize,
    /// 対角ピボット値。
    pivot: f64,
}

/// eta が密形式で格納されていることを示す `len` の番兵値 ([`EtaFile::dense`] 参照)。
const ETA_DENSE: u32 = u32::MAX;


/// [`EtaFile::iter`] が返す、生きている eta 1 つへの参照。
#[derive(Clone, Copy)]
struct EtaRef {
    /// ヘッダ番号 ([`EtaFile::nnz`] / `dot` / `axpy` の引数)。
    k: usize,
    /// eta のスロット (`U`) または行 `p` (`R`)。ピボット値は必要な場所
    /// (ゼロスキップ後) でだけ `EtaFile::pivot[k]` から読む。
    slot: usize,
}

/// フラットな eta ファイル (HiGHS `HFactor` 方式)。eta ごとのヘッダを並列配列
/// (`key`, `pivot`, `span`) で持ち、要素は 1 つの連続プール (`idx: u32`,
/// `val: f64`) に置く。fill が `DENSE_ETA_FRACTION * m` を超えた eta は密形式
/// (`len == ETA_DENSE`、`start` が `dense` の添字) で持つ。
///
/// `U` eta の置換 (FT 更新) はヘッダを並列配列から削除する ([`Self::remove`])。大きな問題では
/// 削除の代わりに「死んだ」ヘッダにする ([`Self::kill`]、策4: 後続ヘッダの位置がずれないので
/// `Vec::remove` の memmove と位置索引の付け直しが要らない)。要素と死んだヘッダはゴミとして残り、
/// 次の再分解で新しいファイルが作られるまで回収されない。
#[derive(Clone, Default)]
struct EtaFile {
    /// eta ごとのキー (`U` ならスロット、`R` なら行 `p`)。
    key: Vec<u32>,
    /// eta ごとのピボット値。
    pivot: Vec<f64>,
    /// `idx`/`val` への `(開始, 長さ)`、または `(dense の添字, ETA_DENSE)`。
    span: Vec<(u32, u32)>,
    /// 疎 eta の要素の添字プール。
    idx: Vec<u32>,
    /// 疎 eta の要素の値プール。
    val: Vec<f64>,
    /// 密 eta の本体 (長さ `m` の配列, 非ゼロ数)。
    dense: Vec<(Box<[f64]>, usize)>,
}

impl EtaFile {
    /// eta `etas` 個・要素 `entries` 個分の容量を予約して作る。
    fn with_capacity(etas: usize, entries: usize) -> Self {
        EtaFile {
            key: Vec::with_capacity(etas),
            pivot: Vec::with_capacity(etas),
            span: Vec::with_capacity(etas),
            idx: Vec::with_capacity(entries),
            val: Vec::with_capacity(entries),
            dense: Vec::new(),
        }
    }

    /// ヘッダ数。
    #[inline]
    fn n_headers(&self) -> usize {
        self.key.len()
    }

    /// eta を作成順に返す (逆順は `.rev()`)。死んだヘッダ ([`Self::kill`]) も返すが、ピボット 1・
    /// 要素なしなので `U` 段の処理 (0 判定・除算・axpy・tick) では何もしないのと同じになる。
    #[inline(always)]
    fn iter(&self) -> impl DoubleEndedIterator<Item = EtaRef> + '_ {
        self.key.iter().enumerate().map(|(k, &s)| EtaRef { k, slot: s as usize })
    }

    /// eta `k` の非ゼロ数。
    #[inline(always)]
    fn nnz(&self, k: usize) -> usize {
        let (start, len) = self.span[k];
        if len == ETA_DENSE {
            self.dense[start as usize].1
        } else {
            len as usize
        }
    }

    /// 疎 eta `k` の要素 (添字, 値) スライス。
    #[inline(always)]
    fn seg(&self, k: usize) -> (&[u32], &[f64]) {
        let (s, l) = self.span[k];
        let s = s as usize;
        let e = s + l as usize;
        (&self.idx[s..e], &self.val[s..e])
    }

    /// 内積 `eta_k · dense` を返す。
    #[inline(always)]
    fn dot(&self, k: usize, dense: &[f64]) -> f64 {
        let (start, len) = self.span[k];
        if len == ETA_DENSE {
            let data = &self.dense[start as usize].0;
            return data.iter().zip(dense.iter()).map(|(&v, &d)| v * d).sum();
        }
        let (idx, val) = self.seg(k);
        idx.iter().zip(val.iter()).map(|(&i, &v)| v * dense[i as usize]).sum()
    }

    /// `dense += alpha * eta_k`。
    #[inline(always)]
    fn axpy(&self, k: usize, alpha: f64, dense: &mut [f64]) {
        let (start, len) = self.span[k];
        if len == ETA_DENSE {
            let data = &self.dense[start as usize].0;
            for (d, &v) in dense.iter_mut().zip(data.iter()) {
                *d += alpha * v;
            }
            return;
        }
        let (idx, val) = self.seg(k);
        for (&i, &v) in idx.iter().zip(val.iter()) {
            dense[i as usize] += alpha * v;
        }
    }

    /// eta `k` の各格納要素に対して `f(添字, 値)` を呼ぶ (密形式では非ゼロのみ)。
    #[inline]
    fn for_each_entry(&self, k: usize, mut f: impl FnMut(usize, f64)) {
        let (start, len) = self.span[k];
        if len == ETA_DENSE {
            for (i, &x) in self.dense[start as usize].0.iter().enumerate() {
                if x != 0.0 {
                    f(i, x);
                }
            }
            return;
        }
        let (idx, val) = self.seg(k);
        for (&i, &v) in idx.iter().zip(val.iter()) {
            f(i as usize, v);
        }
    }

    /// eta `k` から添字 `index` の要素を (他の順序を保って) 除き、あったかを返す。
    fn remove_index(&mut self, k: usize, index: usize) -> bool {
        let (start, len) = self.span[k];
        if len == ETA_DENSE {
            let (data, nnz) = &mut self.dense[start as usize];
            if data[index] != 0.0 {
                data[index] = 0.0;
                *nnz -= 1;
                return true;
            }
            return false;
        }
        let s = start as usize;
        let e = s + len as usize;
        let Some(pos) = self.idx[s..e].iter().position(|&i| i as usize == index) else {
            return false;
        };
        let pos = s + pos;
        self.idx.copy_within(pos + 1..e, pos);
        self.val.copy_within(pos + 1..e, pos);
        self.span[k].1 -= 1;
        true
    }

    /// eta `k` が密形式か。
    #[inline(always)]
    fn is_dense(&self, k: usize) -> bool {
        self.span[k].1 == ETA_DENSE
    }

    /// ヘッダ `k` を削除する (後続ヘッダは 1 つ前にずれる)。
    fn remove(&mut self, k: usize) {
        self.key.remove(k);
        self.pivot.remove(k);
        self.span.remove(k);
    }

    /// ヘッダ `k` を「死んだ」ヘッダにする (策4): キー (スロット) は残し、ピボットを 1・要素を空にする。
    /// 後続ヘッダの位置は変わらない。`U` 段の走査はそのまま通してよい ([`Self::iter`])。`U^T` 段は
    /// スロットの行を散布するので飛ばす必要がある (`FtLu::slot_pos[スロット] != k` で判定)。
    fn kill(&mut self, k: usize) {
        self.pivot[k] = 1.0;
        self.span[k] = (0, 0);
    }

    /// 密ベクトル `src` から eta `{(i, scale * src[i]) : i != skip, 残すもの}` を
    /// 作ってプールへ直接詰め、ヘッダ番号を返す。[`tiny_drop`] 未満の値は落とす。
    /// 非ゼロ数が `dense_fraction * len` を超えれば密形式にする。
    ///
    /// - `key`: スロット (`U`) または行 (`R`)。`pivot`: ピボット値。
    /// - `skip`: 格納しない添字 (対角位置)。`scale`: 各値に掛ける係数。
    fn push_scaled_dense(&mut self, key: usize, pivot: f64, src: &[f64], skip: usize, scale: f64, dense_fraction: f64) -> usize {
        let len = src.len();
        // この絶対値未満は格納しない (0 なら厳密な 0 だけ落とす)
        let tiny = tiny_drop();
        let k = self.key.len();
        self.key.push(key as u32);
        self.pivot.push(pivot);
        // まず数える (直線的な 1 パス、`skip` 分は後で補正)、次に疎/密を決め、
        // 最後に詰める。
        let nnz = if tiny > 0.0 {
            let mut n = 0usize;
            for &x in src {
                n += usize::from((scale * x).abs() >= tiny);
            }
            n - usize::from((scale * src[skip]).abs() >= tiny)
        } else {
            let mut n = 0usize;
            for &x in src {
                n += usize::from(scale * x != 0.0);
            }
            n - usize::from(scale * src[skip] != 0.0)
        };
        if nnz as f64 > dense_fraction * len as f64 {
            let data: Vec<f64> = if tiny > 0.0 {
                src.iter().enumerate().map(|(i, &x)| if i != skip && (scale * x).abs() >= tiny { scale * x } else { 0.0 }).collect()
            } else {
                let mut data = src.to_vec();
                if scale != 1.0 {
                    for d in data.iter_mut() {
                        *d *= scale;
                    }
                }
                data[skip] = 0.0;
                data
            };
            self.span.push((self.dense.len() as u32, ETA_DENSE));
            self.dense.push((data.into_boxed_slice(), nnz));
        } else {
            let base = self.idx.len();
            self.idx.reserve(nnz);
            self.val.reserve(nnz);
            if tiny > 0.0 {
                for (i, &x) in src.iter().enumerate() {
                    let v = scale * x;
                    if i != skip && v.abs() >= tiny {
                        self.idx.push(i as u32);
                        self.val.push(v);
                    }
                }
            } else {
                for (i, &x) in src.iter().enumerate() {
                    let v = scale * x;
                    if i != skip && v != 0.0 {
                        self.idx.push(i as u32);
                        self.val.push(v);
                    }
                }
            }
            debug_assert_eq!(self.idx.len() - base, nnz);
            self.span.push((base as u32, nnz as u32));
        }
        k
    }

    /// [`Self::push_scaled_dense`] の疎入力版 (策3): `src` の非ゼロになりうる位置の一覧
    /// `list` (昇順・重複なし、`src` の非ゼロをすべて含む) だけを見て同じ eta を作る。
    /// 非ゼロ数・疎/密の判定・要素の順序 (添字昇順) が密版と同じなのでビット一致。
    /// 密形式になる場合だけ `src` 全体から作る。
    #[allow(clippy::too_many_arguments)]
    fn push_scaled_list(&mut self, key: usize, pivot: f64, src: &[f64], list: &[usize], skip: usize, scale: f64, dense_fraction: f64) -> usize {
        debug_assert!(list.windows(2).all(|w| w[0] < w[1]));
        let tiny = tiny_drop();
        // 格納する値か (密版の判定と同じ)
        let keep = |i: usize, v: f64| i != skip && if tiny > 0.0 { v.abs() >= tiny } else { v != 0.0 };
        let nnz = list.iter().filter(|&&i| keep(i, scale * src[i])).count();
        if nnz as f64 > dense_fraction * src.len() as f64 {
            return self.push_scaled_dense(key, pivot, src, skip, scale, dense_fraction);
        }
        let k = self.key.len();
        self.key.push(key as u32);
        self.pivot.push(pivot);
        let base = self.idx.len();
        self.idx.reserve(nnz);
        self.val.reserve(nnz);
        for &i in list {
            let v = scale * src[i];
            if keep(i, v) {
                self.idx.push(i as u32);
                self.val.push(v);
            }
        }
        debug_assert_eq!(self.idx.len() - base, nnz);
        self.span.push((base as u32, nnz as u32));
        k
    }

    /// 最後に追加した eta を取り除く (棄却された更新の `R` 用)。
    fn pop(&mut self) {
        self.key.pop();
        self.pivot.pop();
        let (s, l) = self.span.pop().unwrap();
        if l == ETA_DENSE {
            self.dense.pop();
        } else {
            self.idx.truncate(s as usize);
            self.val.truncate(s as usize);
        }
    }
}

/// 新しい疎経路 (策1〜4・12、追加策 R) を使う最小行数 ([`SPARSE_PATH_MIN_M`]、
/// `ENOMOTO_T_SPARSE_PATH_MIN_M` で上書き可)。これ未満の問題は元の経路のコードだけを通る。
pub(crate) fn sparse_path_min_m() -> usize {
    tunable!("ENOMOTO_T_SPARSE_PATH_MIN_M", SPARSE_PATH_MIN_M, usize)
}

/// 結果密度ゲートの閾値 ([`EXPECTED_DENSE_FRACTION`]、環境変数
/// `ENOMOTO_EXPECTED_DENSITY_GATE` で上書き可。`>= 1.0` でゲート無効)。
/// [`FtranDensity::new`] ごとに 1 回だけ読む。
fn expected_dense_gate() -> f64 {
    env_str!("ENOMOTO_EXPECTED_DENSITY_GATE")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(EXPECTED_DENSE_FRACTION)
}

/// [`LuFactors::l_row`] を作る。スキャッタ形式が無効
/// (`ENOMOTO_BTRAN_L_SCATTER=0`) なら、同じ `m` 行で要素なしの空行列を返す
/// (無効時に構築コストも払わないため。空でも `row(i)` は常に有効)。
fn build_l_row(l_col: &CscMat, m: usize) -> CsrMat {
    if btran_l_scatter_gate() <= 0.0 {
        return CscMat::empty(m, m).to_csr();
    }
    l_col.to_csr()
}

/// FTRAN/BTRAN の結果や新しい eta 要素を厳密な 0 とみなす絶対値の閾値
/// (環境変数 `ENOMOTO_TINY`、既定 [`TINY_DROP`] = 0 で切り捨てなし)。
#[inline]
pub(crate) fn tiny_drop() -> f64 {
    tunable!("ENOMOTO_TINY", TINY_DROP, f64)
}

/// BTRAN の最終段: ステップ空間の `w` を元の行順 `y[row_perm[s]] = w[s]` に戻す
/// ([`tiny_drop`] 未満は 0 にする)。
#[inline]
fn permute_btran_out(row_perm: &[usize], w: &[f64], y: &mut [f64]) {
    let tiny = tiny_drop();
    if tiny > 0.0 {
        for (s, &v) in w.iter().enumerate() {
            y[row_perm[s]] = if v.abs() < tiny { 0.0 } else { v };
        }
    } else {
        for (s, &v) in w.iter().enumerate() {
            y[row_perm[s]] = v;
        }
    }
}

/// BTRAN 結果の非ゼロ *ステップ* の記録。
/// [`FtLu::solve_transpose_unit_capture_steps`] が最終置換中に (追加コストなしで)
/// 記録し、同じベクトルに対する次の DSE `tau` FTRAN
/// ([`FtLu::solve_into_pair_capture`] 等) がそれを `L` 段の Gilbert-Peierls 起点
/// として使う (BTRAN の出力置換は FTRAN の `L` 段入力置換の逆なので、記録した
/// ステップがそのまま起点になる)。
///
/// 非ゼロが `limit_frac * m` を超えたら記録を放棄 (`valid = false`) し、FTRAN は
/// 通常の密 `L` 段を使う (結果はどちらでもビット一致)。記録は最初に読んだ FTRAN が
/// 消費 (無効化) するので、別のベクトルに誤用されることはない。
pub struct StepCapture {
    /// 記録した非ゼロのステップ。
    steps: Vec<usize>,
    /// `steps` が有効 (未消費かつ上限内) か。
    valid: bool,
    /// 記録を諦める非ゼロ率の上限 (`m` に対する割合)。
    limit_frac: f64,
    /// 消費側 FTRAN が使う Gilbert-Peierls スクラッチ。
    gp: GpScratch,
}

impl StepCapture {
    /// 次数 `m` 用の (無効状態の) 記録を作る。
    pub fn new(m: usize) -> Self {
        StepCapture {
            steps: Vec::new(),
            valid: false,
            limit_frac: tunable!("ENOMOTO_T_TAU_GP_FRACTION", TAU_GP_FRACTION, f64),
            gp: GpScratch::new(m),
        }
    }

    /// C5 (`tau` チャネル): この記録を消費する次の FTRAN で `tau` の `U` 段を
    /// 超疎に実行するか ([`FtLu::u_solve_hyper`]、起点はこの記録の `L` 到達集合)。
    pub fn set_u_hyper(&mut self, on: bool) {
        self.gp.u_hyper = on;
    }

    /// FTRAN 1 回分として記録を取り出す: 有効なら無効化して `Some` を返す。
    #[inline]
    fn take(cap: Option<&mut StepCapture>) -> Option<&mut StepCapture> {
        match cap {
            Some(c) if c.valid => {
                c.valid = false;
                Some(c)
            }
            _ => None,
        }
    }
}

/// 呼び出し側が持つ長さ `m` の出力ベクトルの「非ゼロになりうる位置」の記録
/// (stormG2 報告 §4 策1)。`full == false` の間、対応するベクトルは `idx` 以外の位置で
/// 全 `+0.0` (`idx` は重複なし・順不同で、値 0 の位置を含みうる)。疎な書き出しは
/// 前回の `idx` だけを 0 に戻してから書くので、`O(m)` の `fill` が要らない。
///
/// 記録の外でベクトル全体を書いた (密な置換出力など) ときは [`Self::set_full`] で
/// 無効化すること。無効な記録の次の疎書き出しは 1 回だけ全体を `fill` する。
#[derive(Clone, Debug)]
pub struct NzTrack {
    /// 非ゼロになりうる位置 (`full` の間は意味なし)。
    idx: Vec<usize>,
    /// 位置が分からない (ベクトル全体が非ゼロでありうる) か。
    full: bool,
}

impl NzTrack {
    /// 無効状態 (`full`) の記録を作る。
    pub fn new() -> Self {
        NzTrack { idx: Vec::new(), full: true }
    }

    /// 記録を無効化する (対応するベクトルを記録の外で書いた後に呼ぶ)。
    #[inline]
    pub fn set_full(&mut self) {
        self.full = true;
        self.idx.clear();
    }

    /// 有効なら非ゼロになりうる位置の一覧。
    #[inline]
    pub fn indices(&self) -> Option<&[usize]> {
        if self.full {
            None
        } else {
            Some(&self.idx)
        }
    }

    /// 前回の位置 (無効なら全体) を 0 に戻し、空の有効な記録にする。
    #[inline]
    fn reset(&mut self, v: &mut [f64]) {
        if self.full {
            v.fill(0.0);
        } else {
            for &i in &self.idx {
                v[i] = 0.0;
            }
        }
        self.idx.clear();
        self.full = false;
    }
}

impl Default for NzTrack {
    /// [`NzTrack::new`] と同じ。
    fn default() -> Self {
        Self::new()
    }
}

/// 融合 FTRAN ([`FtLu::solve_sparse_into_pair_capture`] 等) の出力の疎書き出し用の記録
/// (stormG2 報告 §4 策1・策6)。主ループが 1 つ持ち、毎反復同じものを渡す。
///
/// - `alpha`: 入る列の結果 (`out_a`)、`tau`: DSE の `tau` (`out_b`) の非ゼロ位置 (元の行番号)。
/// - `a_tilde`: FT 更新用の `a_tilde` の非ゼロ位置 (ステップ番号)。
/// - `b_scratch_clean`: `tau` 側スクラッチ (`scratch_b`) が全 0 か (真なら
///   Gilbert-Peierls `L` 段の入口の `fill` を省く)。
///
/// どれも値・演算順は変えない (書き出す位置の集合が同じ) ので結果はビット一致。
#[derive(Clone, Debug, Default)]
pub struct FtranTrack {
    /// 入る列の FTRAN 結果の記録。
    pub alpha: NzTrack,
    /// DSE `tau` の記録。
    pub tau: NzTrack,
    /// `a_tilde` の記録。
    pub a_tilde: NzTrack,
    /// `scratch_b` が全 0 か。
    b_scratch_clean: bool,
    /// 呼び出し側の要求: DSE `tau` を入る列の結果の非ゼロ行だけで求めてよい (部分 `tau`、策12)。
    pub partial_tau: bool,
    /// 直前の融合 FTRAN が `tau` を部分的に求めたか (真なら `tau` は入る列の結果の非ゼロ行でだけ正しい)。
    tau_partial: bool,
    /// 部分 `tau` の作業領域 (`(u_seq 位置, スロット, 値)`)。
    owners_tmp: Vec<(usize, usize, f64)>,
}

impl FtranTrack {
    /// 全記録が無効な状態で作る。
    pub fn new() -> Self {
        Self::default()
    }

    /// 直前の融合 FTRAN が `tau` を部分的に求めたか ([`Self::partial_tau`])。
    #[inline]
    pub fn tau_is_partial(&self) -> bool {
        self.tau_partial
    }
}

/// `track` の `scratch_b` 全 0 フラグを読み、これから `scratch_b` を書くので偽に戻す
/// (融合 FTRAN の `b` の `L` 段の入口で呼ぶ。部分 `tau` の結果フラグもここで偽に戻す)。
#[inline]
fn take_b_clean(track: &mut Option<&mut FtranTrack>) -> bool {
    match track {
        Some(t) => {
            t.tau_partial = false;
            std::mem::replace(&mut t.b_scratch_clean, false)
        }
        None => false,
    }
}

/// 融合 FTRAN の `R` eta 適用の前に、各ベクトルの `r_seeds` を空にする。
#[inline]
fn clear_r_seeds(gp_a: &mut Option<&mut GpScratch>, gp_b: &mut Option<&mut GpScratch>) {
    if let Some(g) = gp_a.as_deref_mut() {
        g.r_seeds.clear();
    }
    if let Some(g) = gp_b.as_deref_mut() {
        g.r_seeds.clear();
    }
}

/// 疎 FTRAN の後始末: `U` 段を超疎に行った (`hyper`) なら非ゼロになりうるのは
/// `gp.u_list` の位置だけなので、そこだけ 0 に戻す (策1)。そうでなければ全体を戻す。
#[inline]
fn clear_after_hyper(scratch: &mut [f64], hyper: bool, gp: &GpScratch) {
    if hyper {
        for &s in &gp.u_list {
            scratch[s] = 0.0;
        }
    } else {
        scratch.fill(0.0);
    }
}

/// 融合 FTRAN の `tau` 側スクラッチの後始末: 超疎 `U` 段が成功した (`b_list`) なら
/// その位置だけ 0 に戻して全 0 と記録する (次の Gilbert-Peierls `L` 段が `fill` を省く)。
#[inline]
fn finish_b_scratch(scratch_b: &mut [f64], b_list: Option<&[usize]>, track: Option<&mut FtranTrack>) {
    if let Some(t) = track {
        match b_list {
            Some(list) => {
                for &s in list {
                    scratch_b[s] = 0.0;
                }
                t.b_scratch_clean = true;
            }
            None => t.b_scratch_clean = false,
        }
    }
}

/// `w` の非ゼロステップを `cap` に記録しつつ行う [`permute_btran_out`]。
/// `ZERO_W` なら同じパスで `w` を全 0 に戻す ([`UnitBtranWork`] 用)。
#[inline]
fn permute_btran_out_capture<const ZERO_W: bool>(row_perm: &[usize], w: &mut [f64], y: &mut [f64], cap: &mut StepCapture) {
    let tiny = tiny_drop();
    // 記録できる非ゼロ数の上限
    let limit = (cap.limit_frac * w.len() as f64) as usize;
    cap.steps.clear();
    // 上限を超えたか
    let mut over = false;
    for (s, wv) in w.iter_mut().enumerate() {
        let v = *wv;
        if ZERO_W {
            *wv = 0.0;
        }
        let v = if tiny > 0.0 && v.abs() < tiny { 0.0 } else { v };
        y[row_perm[s]] = v;
        if v != 0.0 && !over {
            if cap.steps.len() < limit {
                cap.steps.push(s);
            } else {
                over = true;
            }
        }
    }
    cap.valid = !over;
}

/// `w` を全 0 に戻しながら行う出力置換 (`cap` があれば非ゼロステップも記録する)。
#[inline]
fn btran_full_out(row_perm: &[usize], w: &mut [f64], y: &mut [f64], cap: Option<&mut StepCapture>) {
    match cap {
        Some(c) => permute_btran_out_capture::<true>(row_perm, w, y, c),
        None => permute_btran_out_zeroing(row_perm, w, y),
    }
}

/// 同じパスで `w` を全 0 に戻す [`permute_btran_out`]。
#[inline]
fn permute_btran_out_zeroing(row_perm: &[usize], w: &mut [f64], y: &mut [f64]) {
    let tiny = tiny_drop();
    for (s, wv) in w.iter_mut().enumerate() {
        let v = *wv;
        *wv = 0.0;
        y[row_perm[s]] = if tiny > 0.0 && v.abs() < tiny { 0.0 } else { v };
    }
}

/// [`FtLu::solve_transpose_unit_work`] (ピボット行 BTRAN `rho_p = B^-T e_r` と
/// `e_tilde` の記録) 用に呼び出し側が持つ作業領域。本質的に必要なもの以外の
/// `O(m)` パスを除く。結果は [`FtLu::solve_transpose_unit_capture`] とビット一致。
///
/// - `w` は呼び出し間で **常に全 0** に保つ専用スクラッチ (出力置換が読みながら
///   0 に戻す) なので、`e_i` の設定は 1 回の書き込みで済む。
/// - `U^T` 掃引は 0 から非ゼロになった位置を `touch` に記録する (`m` 個まで)。
///   `e_tilde_out` はその位置だけ書き、前回書いた位置 (`e_touch`) を先に消す。
///   したがって `e_tilde_out` を書くのはこのメソッドだけであること。
/// - `touch.len()` + 次に適用する `R` eta の非ゼロ数は `L^T` 段入力の非ゼロ数の
///   上界なので、それがゲート内なら正確な数え上げを省く。
///
/// 超疎経路 (stormG2 報告 §4 策2、`ENOMOTO_SPARSE_BTRAN=0` で無効): `U^T`/`L^T` の両段を
/// 非ゼロ位置の優先度付きキューで駆動し (全スロット・全ステップの走査をしない)、出力
/// `out` も前回の非ゼロ位置だけを消して書く。処理するスロット・ステップの集合と順序は
/// 全走査と同じ (値 0 の位置を飛ばすだけ) なので、`out`・`e_tilde_out`・記録・tick は
/// ビット一致。非ゼロが [`BTRAN_HYPER_FRACTION`] `* m` を超えたらその位置から全走査に
/// 切り替え、結果の非ゼロ率の移動平均がその半分以上なら最初から全走査にする。
pub struct UnitBtranWork {
    /// ステップ空間の作業ベクトル (呼び出し間は全 0)。
    w: Vec<f64>,
    /// 今回 `U^T` 掃引で非ゼロになった位置 (超疎経路では `R`/`L^T` 段の分も追記する)。
    touch: Vec<usize>,
    /// 前回 `e_tilde_out` に書いた位置。
    e_touch: Vec<usize>,
    /// 前回 `e_tilde_out` を全体コピーした (位置記録が溢れた) か。
    e_full: bool,
    /// 超疎経路を使うか (`ENOMOTO_SPARSE_BTRAN`)。
    sparse: bool,
    /// 結果 (`out`) の非ゼロ率の移動平均 ([`FtranDensity`] と同じ重み)。
    /// [`BTRAN_HYPER_FRACTION`] の半分未満のときだけ超疎経路を試す (途中で全走査に落ちる無駄を避ける)。
    density: f64,
    /// 超疎経路の「一覧に入った」印 (`U^T` 段と `L^T` 段で別のエポックを使う)。
    marks: EpochMarks,
    /// `U^T` 段の最小ヒープ (キー = `U^T` 順の位置、[`FtLu::u_transpose_key`])。
    uheap: BinaryHeap<Reverse<u32>>,
    /// `L^T` 段の最大ヒープ (ステップ)。
    lheap: BinaryHeap<u32>,
    /// `out` の非ゼロ位置の記録 (元の行番号)。
    y: NzTrack,
    /// 今回の `out` の非ゼロ行 (昇順)。`rows_valid` の間だけ有効。
    rows: Vec<usize>,
    /// `rows` が今回の `out` の非ゼロ行を正確に表しているか。
    rows_valid: bool,
}

impl UnitBtranWork {
    /// 次数 `m` 用の作業領域を作る。
    pub fn new(m: usize) -> Self {
        UnitBtranWork {
            w: vec![0.0; m],
            touch: Vec::with_capacity(m),
            e_touch: Vec::with_capacity(m),
            e_full: false,
            sparse: env_str!("ENOMOTO_SPARSE_BTRAN").map_or(true, |v| v != "0"),
            density: 0.0,
            marks: EpochMarks::new(m),
            uheap: BinaryHeap::new(),
            lheap: BinaryHeap::new(),
            y: NzTrack::new(),
            rows: Vec::new(),
            rows_valid: false,
        }
    }

    /// 直前の [`FtLu::solve_transpose_unit_work`] の `out` の非ゼロ行 (昇順)。超疎経路で
    /// 求めたときだけ `Some` (全走査なら `None`、呼び出し側が自分で数えること)。
    #[inline]
    pub fn nonzero_rows(&self) -> Option<&[usize]> {
        if self.rows_valid {
            Some(&self.rows)
        } else {
            None
        }
    }

    /// `out` に渡すベクトルを [`FtLu::solve_transpose_unit_work`] の外で書いた後に呼ぶ
    /// (次回の超疎書き出しが全体を 0 に戻すようにする)。
    #[inline]
    pub fn invalidate_out(&mut self) {
        self.y.set_full();
        self.rows_valid = false;
    }
}

/// BTRAN `L^{-T}` 段のスキャッタ/ギャザー切替閾値 ([`BTRAN_L_SCATTER_FRACTION`]、
/// 環境変数 `ENOMOTO_BTRAN_L_SCATTER` で上書き可。`0` でスキャッタ無効、`1` で常時)。
/// 再分解ごとに 1 回読む。
fn btran_l_scatter_gate() -> f64 {
    env_str!("ENOMOTO_BTRAN_L_SCATTER").and_then(|v| v.parse::<f64>().ok()).unwrap_or(BTRAN_L_SCATTER_FRACTION)
}

/// [`FtLu::u_solve_into`] がスロットをピボットで割る *前に* 0 判定するか
/// (`ENOMOTO_FTRAN_U_ZERO_SKIP=0` で無条件に割る)。[`FtLu::new`] ごとに 1 回読む。
fn u_zero_skip_enabled() -> bool {
    env_str!("ENOMOTO_FTRAN_U_ZERO_SKIP").map(|v| v != "0").unwrap_or(true)
}

/// FTRAN の呼び出し箇所 (チャネル) ごとの **結果** 密度の移動平均。
/// [`FtLu::should_use_dense_solve_tracked`] が右辺の非ゼロ数と併せて
/// 密/疎の切替に使う (HiGHS の `expected_density` 相当)。
///
/// - 入力が疎でも結果が密になりうるので、入力だけでは判断できない。
/// - チャネルごとに 1 つ持つ (入力列 FTRAN、BFRT 合成フリップ FTRAN など)。
///   [`FtLu`] は再分解のたびに作り直されるので、インスタンスは求解ループ側に置く。
/// - 計測は両方の経路で行うので、密経路に張り付くことはない。
#[derive(Clone, Copy, Debug)]
pub struct FtranDensity {
    /// `result_nnz / m` の移動平均 (`[0, 1]`)。初期値 `0.0` (最も疎)。
    expected: f64,
    /// [`expected_dense_gate`] の値 (構築時に 1 回だけ取得)。
    gate: f64,
}

impl FtranDensity {
    /// 新しいチャネルを作る (平均 0、ゲートは環境変数から取得)。
    pub fn new() -> Self {
        Self { expected: 0.0, gate: expected_dense_gate() }
    }

    /// 求解 1 回の結果密度 (`result_nnz / m`) を平均に畳み込む。どちらの経路でも
    /// 求解が返した非ゼロ数を渡すこと。
    #[inline]
    pub fn record(&mut self, result_nnz: usize, m: usize) {
        if m == 0 {
            return;
        }
        // 今回の結果密度
        let local = result_nnz as f64 / m as f64;
        self.expected = (1.0 - tunable!("ENOMOTO_T_DENSITY_AVERAGE_MULTIPLIER", DENSITY_AVERAGE_MULTIPLIER, f64)) * self.expected + tunable!("ENOMOTO_T_DENSITY_AVERAGE_MULTIPLIER", DENSITY_AVERAGE_MULTIPLIER, f64) * local;
    }

    /// 移動平均の値 (`[0, 1]`)。診断出力 (`ENOMOTO_PROF_PHASES_EXT`) 用。
    #[inline]
    pub fn expected(&self) -> f64 {
        self.expected
    }

    /// このチャネルの履歴だけで密経路を選ぶべきか (`expected > gate`)。
    #[inline]
    pub fn predicts_dense(&self) -> bool {
        self.expected > self.gate
    }
}

impl Default for FtranDensity {
    /// [`FtranDensity::new`] と同じ。
    fn default() -> Self {
        Self::new()
    }
}

/// Forrest-Tomlin 更新付きの LU 因子。`B_k^{-1} = U_k^{-1} R_k^{-1} ... R_1^{-1} L^{-1}`
/// を保持し、FTRAN (`solve_*`) / BTRAN (`solve_transpose_*`) と列置換更新
/// ([`Self::try_update`]) を提供する。再分解のたびに作り直す。
#[derive(Clone)]
pub struct FtLu {
    /// 元の分解 (`L` と置換。`L` は更新で変わらない)。
    base: LuFactors,
    /// `U` の非シングルトン eta を **作成順** (= 三角順) に格納したもの。
    /// FTRAN/BTRAN の `U` 段はこれを順に走査する。
    u_seq: EtaFile,
    /// 分解直後の **シングルトン** `U` eta (非対角なし、対角ピボットのみ)。
    /// 自スロットしか読み書きしないので、`U` 求解では `u_seq` の後、`U^T` 掃引
    /// では前にまとめて割ってもビット一致。順序は無関係。置換されたら
    /// `swap_remove` で除く (`u_seq` から戻ってくることはない)。
    singles: Vec<SEta>,
    /// `singles_pos[slot]` = `slot` の `singles` 内の位置。`u_seq` 側なら `usize::MAX`。
    singles_pos: Vec<usize>,
    /// `single_piv[slot]` = `slot` が `singles` にある間はそのピボット、なければ `0.0`
    /// (ピボットは 0 にならない)。FTRAN の `U` 段のシングルトン除算を
    /// [`Self::permute_out`] の中で行うために使う。
    single_piv: Vec<f64>,
    /// `L` の列が非空のステップ (昇順)。密 `L` 段が訪れる必要があるのはこれだけ
    /// ([`LuFactors::l_solve_into_pair`] 参照)。
    l_active: Vec<u32>,
    /// `slot_pos[slot]` = `slot` の `u_seq` 内の現在位置 (`singles` 側なら `usize::MAX`)。
    /// `try_update` が並べ替えと同じ範囲で同期させる。
    slot_pos: Vec<usize>,
    /// 逆引き索引: `row_owners[r]` = 行ステップ `r` に非ゼロを持つ eta のスロットと値。
    /// `try_update` が行 `p` を他の全 eta から消すとき、該当する eta だけに触れる
    /// ために使う (Tomlin 1974, eq. 12)。
    row_owners: Vec<Vec<(usize, f64)>>,
    /// `R` 行 eta (作成順、`key` = 行 `p`、`pivot` は未使用)。
    r_etas: EtaFile,
    /// `R` eta の列方向索引 (追加策 R): `r_head[i]` は位置 `i` に要素を持つ `R` eta の要素 (プール位置) の
    /// 連結リストの先頭 (`u32::MAX` で空)、`r_next[e]` は次の要素。新しい eta ほど前に入る。
    /// 疎なベクトルに `R` eta を当てるとき、内積が 0 でありえない eta だけを選ぶのに使う
    /// ([`Self::apply_r_sparse`])。密形式の `R` eta は載せず `r_dense` に番号を持つ。
    r_head: Vec<u32>,
    /// [`Self::r_head`] の次要素。
    r_next: Vec<u32>,
    /// 要素 (プール位置) → その要素を持つ `R` eta の番号。
    r_owner: Vec<u32>,
    /// 密形式の `R` eta の番号。
    r_dense: Vec<u32>,
    /// 全 `R` eta の非ゼロ数の和 (tick 用)。
    r_nnz_total: usize,
    /// 列方向索引に載っている `R` eta の数 (先頭から。全部載っているときだけ疎な `R` 段を使う)。
    r_indexed: usize,
    /// 置換された `U` eta を削除せず死んだヘッダにするか (策4、`m >= SPARSE_PATH_MIN_M`)。
    lazy_remove: bool,
    /// 死んだヘッダの数 (0 でなければ `U^T` 掃引は死んだヘッダを飛ばす版を使う)。
    n_dead: usize,
    /// 疎な `R` 段を使う `R` eta 数の下限 ([`R_SPARSE_MIN_ETAS`]、[`Self::new`] で 1 回だけ読む)。
    r_sparse_min: usize,
    /// [`Self::try_update`] の再利用スクラッチ (`a_tilde` 用、長さ `m`、
    /// 返却前に必ず長さ `m` に戻す)。ピボットごとのヒープ確保を避ける。
    scratch_a_tilde: Vec<f64>,
    /// [`Self::try_update`] の再利用スクラッチ (`e_tilde` 用)。
    scratch_e_tilde: Vec<f64>,
    /// `CLOCK` 再分解トリガ用の決定的な演算量カウンタ (HiGHS の
    /// `total_synthetic_tick_` 相当)。非ゼロ数の加算だけで増えるので、同じ問題は
    /// 常に同じ反復で再分解される。`&self` の求解段から加算するので `Cell`。
    /// 新しい `FtLu` を作る (= 再分解する) と 0 から始まる。
    tick: Cell<u64>,
    /// `u_seq` と `r_etas` が現在保持する非対角 fill の合計
    /// ([`Self::fill_count`] の値)。[`Self::commit_update`] が差分で更新する。
    fill: usize,
    /// 最後の *通常* (Markowitz) 分解の `L`+`U` 非ゼロ数。[`factorize_reusing`] が
    /// 再利用分解の fill 上限の基準に使う ([`REBUILD_FILL_LIMIT`])。
    fill_baseline: usize,
    /// この分解までに連続して棄却されたピボット順再利用の回数
    /// ([`factorize_reusing`] のバックオフ)。
    reuse_fail_streak: u32,
    /// 次の再利用試行まで見送る残り再分解回数。
    reuse_skips_left: u32,
    /// この分解自体の構築コスト ([`Self::tick`] と同じ単位)。[`Self::new`] で
    /// `m` と `L`/`U` の非対角非ゼロ数から 1 回だけ計算する。`CLOCK` トリガは
    /// `tick` が `FACTOR * build_tick` に達したら再分解する。
    build_tick: u64,
    /// [`btran_l_scatter_gate`] の値 (再分解ごとに 1 回取得)。
    btran_l_scatter: f64,
    /// [`u_zero_skip_enabled`] の値 (再分解ごとに 1 回取得、[`Self::u_solve_into`] 参照)。
    u_zero_skip: bool,
}

impl FtLu {
    /// 分解結果 `base` から更新可能な因子を作る (`U` を eta ファイルとシングルトンに
    /// 分け、逆引き索引・構築 tick・fill 基準などを初期化する)。再分解のたびに呼ばれる。
    pub fn new(base: LuFactors) -> Self {
        let m = base.m;
        // 先にスロット・行ごとの正確な個数を数え、内側の `Vec` を最終サイズで 1 回だけ確保する。
        // スロット (= `U` の列ステップ) ごとの非対角要素数
        let mut off_count = vec![0usize; m];
        // 行ステップごとの非対角要素数 (`row_owners` の容量)
        let mut owner_count = vec![0usize; m];
        // スロットごとの対角ピボット
        let mut pivots = vec![0.0; m];
        for row_step in 0..m {
            for &(col_step, v) in &base.u_row[row_step] {
                if col_step != row_step {
                    off_count[col_step] += 1;
                    owner_count[row_step] += 1;
                } else {
                    pivots[col_step] = v;
                }
            }
        }
        let l_nnz: u64 = base.l_col.nnz() as u64;
        // `U` の非対角非ゼロ数 (`u_row[s]` は対角を 1 個含むので長さ - 1。HiGHS の
        // `u_countX` と同じく非対角だけを数える)。
        let u_off: u64 = base.u_row.iter().map(|v| v.len().saturating_sub(1) as u64).sum();
        let mut build_tick = tunable!("ENOMOTO_T_TICK_BUILD_M_COEF", TICK_BUILD_M_COEF, u64) * m as u64 + tunable!("ENOMOTO_T_TICK_BUILD_LU_COEF", TICK_BUILD_LU_COEF, u64) * (l_nnz + u_off);
        // S16 (既定 off): 消去の積和回数 `Σ_s |L 列 s| · |U 行 s の非対角|`
        // (完成した因子から復元した古典的 LU 演算数) を構築 tick に加える。
        let flop_coef = tunable!("ENOMOTO_T_TICK_BUILD_FLOP_COEF", TICK_BUILD_FLOP_COEF, u64);
        if flop_coef > 0 {
            let flops: u64 = (0..m).map(|s| base.l_col.col(s).len() as u64 * owner_count[s] as u64).sum();
            build_tick += flop_coef * flops;
        }
        // fill 基準は対角も含めた全格納要素数 (`factorize_reusing_order` の数え方と一致)。
        let fill_baseline = (l_nnz + u_off) as usize + m;
        let mut singles: Vec<SEta> = Vec::new();
        let mut slot_pos = vec![usize::MAX; m];
        let mut singles_pos = vec![usize::MAX; m];
        let mut single_piv = vec![0.0f64; m];
        let l_active: Vec<u32> = (0..m).filter(|&s| !base.l_col.col(s).is_empty()).map(|s| s as u32).collect();
        // `U` の eta をフラットファイルへ直接詰める: ヘッダはスロット順
        // (シングルトンは別)、疎 eta の要素は `row_step` 昇順。
        let dense_fraction = tunable!("ENOMOTO_T_DENSE_ETA_FRACTION", DENSE_ETA_FRACTION, f64);
        // 非シングルトン eta の数
        let n_live = off_count.iter().filter(|&&c| c > 0).count();
        // 非対角要素の総数
        let total_off: usize = off_count.iter().sum();
        let mut u_seq = EtaFile::with_capacity(n_live, total_off);
        // 疎 eta ごとの次の書き込み位置 (プール内)
        let mut cursor = vec![u32::MAX; m];
        // 密 eta の `dense` 内の添字 (疎なら `u32::MAX`)
        let mut dense_of = vec![u32::MAX; m];
        // 疎 eta 用に確保したプールの長さ
        let mut pool = 0usize;
        for slot in 0..m {
            let c = off_count[slot];
            if c == 0 {
                singles_pos[slot] = singles.len();
                single_piv[slot] = pivots[slot];
                singles.push(SEta { slot, pivot: pivots[slot] });
                continue;
            }
            slot_pos[slot] = u_seq.key.len();
            u_seq.key.push(slot as u32);
            u_seq.pivot.push(pivots[slot]);
            if c as f64 > dense_fraction * m as f64 {
                dense_of[slot] = u_seq.dense.len() as u32;
                u_seq.span.push((u_seq.dense.len() as u32, ETA_DENSE));
                u_seq.dense.push((vec![0.0; m].into_boxed_slice(), c));
            } else {
                cursor[slot] = pool as u32;
                u_seq.span.push((pool as u32, c as u32));
                pool += c;
            }
        }
        u_seq.idx.resize(pool, 0);
        u_seq.val.resize(pool, 0.0);
        for row_step in 0..m {
            for &(col_step, v) in &base.u_row[row_step] {
                if col_step == row_step {
                    continue;
                }
                let d = dense_of[col_step];
                if d != u32::MAX {
                    u_seq.dense[d as usize].0[row_step] = v;
                } else {
                    let c = cursor[col_step] as usize;
                    u_seq.idx[c] = row_step as u32;
                    u_seq.val[c] = v;
                    cursor[col_step] += 1;
                }
            }
        }
        let mut row_owners: Vec<Vec<(usize, f64)>> = owner_count.iter().map(|&c| Vec::with_capacity(c)).collect();
        for k in 0..u_seq.n_headers() {
            let slot = u_seq.key[k] as usize;
            u_seq.for_each_entry(k, |row_step, v| row_owners[row_step].push((slot, v)));
        }
        let fill = total_off;
        FtLu {
            base,
            u_seq,
            singles,
            singles_pos,
            single_piv,
            l_active,
            slot_pos,
            row_owners,
            r_etas: EtaFile::default(),
            r_head: vec![u32::MAX; m],
            r_next: Vec::new(),
            r_owner: Vec::new(),
            r_dense: Vec::new(),
            r_nnz_total: 0,
            r_sparse_min: tunable!("ENOMOTO_T_R_SPARSE_MIN_ETAS", R_SPARSE_MIN_ETAS, usize),
            r_indexed: 0,
            lazy_remove: m >= sparse_path_min_m(),
            n_dead: 0,
            scratch_a_tilde: vec![0.0; m],
            scratch_e_tilde: vec![0.0; m],
            fill,
            tick: Cell::new(0),
            build_tick,
            fill_baseline,
            reuse_fail_streak: 0,
            reuse_skips_left: 0,
            btran_l_scatter: btran_l_scatter_gate(),
            u_zero_skip: u_zero_skip_enabled(),
        }
    }

    /// BTRAN の `L^{-T}` 段。`w` の非ゼロを数え (上限を超えたら打ち切り)、疎なら
    /// 行優先スキャッタ形式、密なら列優先ギャザー形式で解き、元の行順で `y` に書く。
    fn l_transpose_solve_into(&self, w: &mut [f64], y: &mut [f64]) {
        self.l_transpose_solve_into_cap(w, y, None)
    }

    /// [`Self::l_transpose_solve_into`] に、結果の非ゼロステップ記録
    /// ([`StepCapture`]) を任意で付けたもの。
    fn l_transpose_solve_into_cap(&self, w: &mut [f64], y: &mut [f64], cap: Option<&mut StepCapture>) {
        self.l_transpose_solve_into_ext::<false>(w, y, cap, usize::MAX)
    }

    /// [`Self::l_transpose_solve_into_cap`] に [`UnitBtranWork`] 用の 2 機能を加えたもの。
    /// `nnz_bound`: `w` の非ゼロ数の上界 (ゲート内なら数え上げを省く)。
    /// `ZERO_W`: 出力パスで `w` を 0 に戻す。
    fn l_transpose_solve_into_ext<const ZERO_W: bool>(&self, w: &mut [f64], y: &mut [f64], cap: Option<&mut StepCapture>, nnz_bound: usize) {
        // スキャッタ形式を使う非ゼロ数の上限
        let limit = (self.btran_l_scatter * self.base.m as f64) as usize;
        let mut sparse = true;
        if nnz_bound > limit {
            let mut nnz = 0usize;
            for &v in w.iter() {
                nnz += (v != 0.0) as usize;
                if nnz > limit {
                    sparse = false;
                    break;
                }
            }
        }
        if sparse {
            PROF_BTRAN_L_SCATTER.fetch_add(1, Ordering::Relaxed);
            self.base.l_transpose_scatter_core(w);
        } else {
            PROF_BTRAN_L_GATHER.fetch_add(1, Ordering::Relaxed);
            self.base.l_transpose_gather_core(w);
        }
        match cap {
            Some(c) => permute_btran_out_capture::<ZERO_W>(&self.base.row_perm, w, y, c),
            None if ZERO_W => permute_btran_out_zeroing(&self.base.row_perm, w, y),
            None => permute_btran_out(&self.base.row_perm, w, y),
        }
    }

    /// 疎な `R` 段 (追加策 R) を使えるか: `R` eta が [`R_SPARSE_MIN_ETAS`] 個以上あり、全部が列方向索引に
    /// 載っている (すべて [`Self::try_update_tracked`] で追加された)。
    #[inline]
    fn r_sparse_ready(&self) -> bool {
        let n = self.r_etas.n_headers();
        n >= self.r_sparse_min && self.r_indexed == n
    }

    /// 行列の次数 `m`。
    #[inline]
    pub fn dim(&self) -> usize {
        self.base.m
    }

    /// 決定的演算量カウンタ ([`Self::tick`]) の現在値。`slope_intercept_dual.rs` の
    /// `CLOCK` 再分解トリガが読む。
    pub fn synth_tick(&self) -> u64 {
        self.tick.get()
    }

    /// この分解自体の構築コスト ([`Self::synth_tick`] と同じ単位)。
    pub fn build_tick(&self) -> u64 {
        self.build_tick
    }

    /// 求解段の非ゼロ数 `n` を tick に加える (係数 [`TICK_SOLVE_NNZ_COEF`])。
    #[inline]
    fn add_tick(&self, n: u64) {
        self.tick.set(self.tick.get() + TICK_SOLVE_NNZ_COEF * n);
    }

    /// 非ゼロ `rhs_nnz` 個の FTRAN 右辺が十分密で、[`Self::solve_sparse_into`] でなく
    /// 密な [`Self::solve_into`] を使うべきか (`rhs_nnz > DENSE_RHS_FRACTION * m`)。
    pub fn should_use_dense_solve(&self, rhs_nnz: usize) -> bool {
        let m = self.base.m;
        m > 0 && rhs_nnz as f64 > tunable!("ENOMOTO_T_DENSE_RHS_FRACTION", DENSE_RHS_FRACTION, f64) * m as f64
    }

    /// [`Self::should_use_dense_solve`] を呼び出しチャネルの結果密度履歴
    /// ([`FtranDensity`]) で広げたもの: 右辺が密 *または* 最近の結果が密なら密経路。
    /// 密経路側にしか動かさない (密な右辺では疎経路は勝てないため)。
    pub fn should_use_dense_solve_tracked(&self, rhs_nnz: usize, density: &FtranDensity) -> bool {
        self.should_use_dense_solve(rhs_nnz) || density.predicts_dense()
    }

    /// ステップ空間ベクトル `z` に `U_k^{-T}` をその場で適用する (eta 列を作成順に
    /// 処理、eq. (8))。[`Self::row_owners`] 上のスキャッタ形式なので、値 0 の
    /// スロットは判定 1 回で済む (超疎)。
    fn u_transpose_solve_into(&self, z: &mut [f64]) {
        self.u_transpose_sweep(z);
    }

    /// `z` の非ゼロがステップ `seed` の 1 個だけと分かっている場合の
    /// [`Self::u_transpose_solve_into`] (単位ベクトル右辺の BTRAN: ピボット行 `rho_p`、
    /// DSE の `rho`、`DseState::from_basis` の参照解など)。
    fn u_transpose_solve_seeded(&self, z: &mut [f64], seed: usize) {
        debug_assert!(z.iter().enumerate().all(|(s, &v)| s == seed || v == 0.0));
        self.u_transpose_sweep(z);
    }

    /// `U^{-T}` 掃引本体 (上の 2 つが共有)。スキャッタ形式 (HiGHS `HFactor::btranU`
    /// と同様): スロット `p` の値が確定したらピボットで割り、それを読む全 eta
    /// (`row_owners[p]`) へ散布する。
    fn u_transpose_sweep(&self, z: &mut [f64]) {
        if self.n_dead != 0 {
            return self.u_transpose_sweep_live(z);
        }
        // CLOCK トリガ用: 全スロット走査の `m` と、散布した各行の長さを tick に加える。
        self.add_tick(self.base.m as u64);
        // シングルトンが先 (そのスロットに書く eta は無いので値は掃引前に確定)。
        for (p, pivot) in self.u_transpose_order() {
            let zp = z[p];
            if zp == 0.0 {
                continue;
            }
            let zp = zp / pivot;
            z[p] = zp;
            let owners = &self.row_owners[p];
            self.add_tick(owners.len() as u64);
            for &(q, v) in owners {
                z[q] -= v * zp;
            }
        }
    }

    /// 全 `U` eta の `(スロット, ピボット)` を `U^T` 順 (シングルトン → `u_seq` の作成順) で返す。
    #[inline(always)]
    fn u_transpose_order(&self) -> impl Iterator<Item = (usize, f64)> + '_ {
        self.singles.iter().map(|e| (e.slot, e.pivot)).chain(self.u_seq.key.iter().zip(self.u_seq.pivot.iter()).map(|(&s, &v)| (s as usize, v)))
    }

    /// 0 から非ゼロに変えた位置を `touch` に追記しながら行う
    /// [`Self::u_transpose_sweep`] (同じ位置が複数回入ることもある)。`touch` が
    /// `m` 個を超えそうなら記録を止めて `false` を返す (呼び出し側は全位置を
    /// 非ゼロの可能性ありと扱うこと)。演算と順序・tick は非追跡版と同じ。
    fn u_transpose_sweep_track(&self, z: &mut [f64], touch: &mut Vec<usize>) -> bool {
        if self.n_dead != 0 {
            return self.u_transpose_sweep_track_live(z, touch);
        }
        // `touch` の上限
        let cap = self.base.m;
        // まだ記録を続けているか
        let mut ok = true;
        self.add_tick(self.base.m as u64);
        for (p, pivot) in self.u_transpose_order() {
            let zp = z[p];
            if zp == 0.0 {
                continue;
            }
            let zp = zp / pivot;
            z[p] = zp;
            let owners = &self.row_owners[p];
            self.add_tick(owners.len() as u64);
            if ok && touch.len() + owners.len() <= cap {
                for &(q, v) in owners {
                    let old = z[q];
                    z[q] = old - v * zp;
                    if old == 0.0 {
                        touch.push(q);
                    }
                }
            } else {
                ok = false;
                for &(q, v) in owners {
                    z[q] -= v * zp;
                }
            }
        }
        ok
    }

    /// 死んだヘッダ ([`EtaFile::kill`]) を飛ばす `U^T` 順 (策4 の大きな問題用)。
    #[inline(always)]
    fn u_transpose_order_live(&self) -> impl Iterator<Item = (usize, f64)> + '_ {
        let slot_pos = &self.slot_pos;
        self.singles.iter().map(|e| (e.slot, e.pivot)).chain(
            self.u_seq.key.iter().zip(self.u_seq.pivot.iter()).enumerate().filter(move |&(k, (&s, _))| slot_pos[s as usize] == k).map(|(_, (&s, &v))| (s as usize, v)),
        )
    }

    /// 死んだヘッダがあるときの [`Self::u_transpose_sweep`] (演算・順序・tick は同じ)。
    #[inline(never)]
    fn u_transpose_sweep_live(&self, z: &mut [f64]) {
        self.add_tick(self.base.m as u64);
        for (p, pivot) in self.u_transpose_order_live() {
            let zp = z[p];
            if zp == 0.0 {
                continue;
            }
            let zp = zp / pivot;
            z[p] = zp;
            let owners = &self.row_owners[p];
            self.add_tick(owners.len() as u64);
            for &(q, v) in owners {
                z[q] -= v * zp;
            }
        }
    }

    /// 死んだヘッダがあるときの [`Self::u_transpose_sweep_track`] (演算・順序・tick は同じ)。
    #[inline(never)]
    fn u_transpose_sweep_track_live(&self, z: &mut [f64], touch: &mut Vec<usize>) -> bool {
        let cap = self.base.m;
        let mut ok = true;
        self.add_tick(self.base.m as u64);
        for (p, pivot) in self.u_transpose_order_live() {
            let zp = z[p];
            if zp == 0.0 {
                continue;
            }
            let zp = zp / pivot;
            z[p] = zp;
            let owners = &self.row_owners[p];
            self.add_tick(owners.len() as u64);
            if ok && touch.len() + owners.len() <= cap {
                for &(q, v) in owners {
                    let old = z[q];
                    z[q] = old - v * zp;
                    if old == 0.0 {
                        touch.push(q);
                    }
                }
            } else {
                ok = false;
                for &(q, v) in owners {
                    z[q] -= v * zp;
                }
            }
        }
        ok
    }

    /// `u_seq[start..]` に限った [`Self::u_transpose_solve_into`] (ギャザー形式)。
    /// **前提条件**: 入口で `z` が `[0, start)` で全 0 であり、`u_seq` が分解直後
    /// (位置 = スロット) であること。`try_update` で並べ替わった後に単独で呼ばないこと。
    fn u_transpose_solve_from(&self, z: &mut [f64], start: usize) {
        for eta in self.u_seq.iter().skip(start) {
            let p = eta.slot;
            let y = self.u_seq.dot(eta.k, z);
            z[p] = (z[p] - y) / self.u_seq.pivot[eta.k];
        }
    }

    /// `x` に `U_k^{-1}` をその場で適用する (eta 列を逆順に処理、eq. (7))。
    /// `xp` が 0 の eta は内側ループ全体を飛ばす (超疎)。シングルトンの除算は
    /// ここではせず [`Self::permute_out`] が `single_piv` を使って行う
    /// (`u_zero_skip` off 時を除く)。
    ///
    /// 超疎版は [`Self::u_solve_hyper`] (任意機能)。
    fn u_solve_into(&self, x: &mut [f64]) {
        // CLOCK トリガ用: 全 eta の除算分として一律 `m`、非対角更新分は
        // `xp` がスキップされなかった eta についてだけ加える。
        self.add_tick(self.base.m as u64);
        if !self.u_zero_skip {
            // `ENOMOTO_FTRAN_U_ZERO_SKIP=0`: 以前のループそのまま (A/B 用)。
            for eta in self.u_seq.iter().rev() {
                let p = eta.slot;
                x[p] /= self.u_seq.pivot[eta.k];
                let xp = x[p];
                if xp == 0.0 {
                    continue;
                }
                self.add_tick(self.u_seq.nnz(eta.k) as u64);
                self.u_seq.axpy(eta.k, -xp, x);
            }
            for eta in &self.singles {
                x[eta.slot] /= eta.pivot;
            }
            return;
        }
        for eta in self.u_seq.iter().rev() {
            let p = eta.slot;
            // 除算の *前に* 0 判定する (0 のスロットへの無駄な除算とストアを省く)。
            // `pivot < 0` のとき `-0.0` の代わりに `+0.0` が残りうるが、結果の利用側は
            // すべて 0 かどうかで分岐し、0 の符号には依存しない。
            if x[p] == 0.0 {
                continue;
            }
            x[p] /= self.u_seq.pivot[eta.k];
            let xp = x[p];
            self.add_tick(self.u_seq.nnz(eta.k) as u64);
            // 密形式でも `data[p] == 0.0` (対角位置は格納しない) なので、
            // いま割った `x[p]` は変わらない。
            self.u_seq.axpy(eta.k, -xp, x);
        }
        // シングルトンは最後 (そのスロットへの書き込みは全て済んでいる)。
        // その除算 (同じゼロスキップ付き) は `permute_out` が `single_piv` で行う。
    }

    /// C5: FTRAN 1 本の `U` 段を、非ゼロから到達しうる eta だけに限って行う。
    /// `x` は `L`/`R` 適用後のベクトルで、非ゼロは `gp.reach` (`L` 段の到達集合) と
    /// `R` eta のスロットに含まれる。そこから `U` の eta グラフ (スロット `p` → その
    /// eta の非対角要素の行) を DFS して到達スロットを `gp.u_list` に集め、到達した
    /// `u_seq` eta を **`u_seq` 位置の降順** に適用する。[`Self::u_solve_into`] の
    /// 逆順走査の部分列そのものなので、値も tick もビット一致。
    ///
    /// 到達数が [`U_HYPER_ABORT_FRACTION`] `* m` を超えた場合、または `U` に密 eta が
    /// ある場合は `x` に触れずに `false` を返す (呼び出し側が全走査する)。
    fn u_solve_hyper(&self, x: &mut [f64], gp: &mut GpScratch) -> bool {
        let m = self.base.m;
        if !self.u_seq.dense.is_empty() {
            return false;
        }
        // 到達スロット数の上限
        let limit = (tunable!("ENOMOTO_T_U_HYPER_ABORT", U_HYPER_ABORT_FRACTION, f64) * m as f64) as usize;
        gp.u_marks.begin();
        gp.u_list.clear();
        gp.u_pos.clear();
        let n_reach = gp.reach.len();
        let n_r = self.r_etas.n_headers();
        for i in 0..n_reach + n_r {
            // DFS 起点候補: `L` 段の到達ステップ、続いて `R` eta の行
            let seed = if i < n_reach { gp.reach[i] } else { self.r_etas.key[i - n_reach] as usize };
            if x[seed] == 0.0 || gp.u_marks.is_marked(seed) {
                continue;
            }
            gp.u_marks.mark(seed);
            gp.u_stack.push(seed);
            while let Some(node) = gp.u_stack.pop() {
                gp.u_list.push(node);
                if gp.u_list.len() > limit {
                    gp.u_stack.clear();
                    return false;
                }
                let k = self.slot_pos[node];
                if k == usize::MAX {
                    continue;
                }
                gp.u_pos.push(k);
                let (idx, _) = self.u_seq.seg(k);
                for &r in idx {
                    let r = r as usize;
                    if !gp.u_marks.is_marked(r) {
                        gp.u_marks.mark(r);
                        gp.u_stack.push(r);
                    }
                }
            }
        }
        gp.u_pos.sort_unstable();
        for &k in gp.u_pos.iter().rev() {
            let p = self.u_seq.key[k] as usize;
            if x[p] == 0.0 {
                continue;
            }
            x[p] /= self.u_seq.pivot[k];
            let xp = x[p];
            self.add_tick(self.u_seq.nnz(k) as u64);
            self.u_seq.axpy(k, -xp, x);
        }
        true
    }

    /// C5 (大きな問題の融合 FTRAN 用): FTRAN 1 本の `U` 段を、非ゼロになった eta だけに限って行う。
    /// `x` は `L`/`R` 適用後のベクトルで、非ゼロは `gp.reach` (`L` 段の到達集合) と
    /// `gp.r_seeds` (`R` eta で値が変わった行) に含まれる。非ゼロになったスロットの `u_seq`
    /// 位置を最大ヒープに積み、**位置の降順** に取り出して適用する (スロット `p` に書く eta は
    /// すべて `p` の eta より後ろの位置にあるので、取り出した時点で `x[p]` は確定している)。
    /// 書いた行は `gp.u_list` に集める。[`Self::u_solve_into`] の逆順走査のうち `x[p] != 0` の
    /// eta だけを同じ順に適用するので、値も tick もビット一致 (策12: 以前の「DFS で構造的な
    /// 到達集合を集めてから位置をソートして適用」と同じ結果で、DFS・ソートを省く)。
    ///
    /// - 一覧が [`U_HYPER_ABORT_FRACTION`] `* m` を超えたら、残り (より前の位置) を全走査で
    ///   仕上げて [`UHyper::Full`] を返す (`x` は解き終わっているが非ゼロ位置は不明)。
    /// - `U` に密 eta がある場合は `x` に触れずに [`UHyper::NotTried`] を返す (呼び出し側が全走査する)。
    fn u_solve_hyper_heap(&self, x: &mut [f64], gp: &mut GpScratch) -> UHyper {
        let m = self.base.m;
        if !self.u_seq.dense.is_empty() {
            return UHyper::NotTried;
        }
        // 一覧の長さの上限
        let limit = (tunable!("ENOMOTO_T_U_HYPER_ABORT", U_HYPER_ABORT_FRACTION, f64) * m as f64) as usize;
        let GpScratch { reach, r_seeds, u_marks, u_heap, u_list, .. } = gp;
        u_marks.begin();
        u_list.clear();
        u_heap.clear();
        for &s in reach.iter().chain(r_seeds.iter()) {
            if x[s] == 0.0 || u_marks.is_marked(s) {
                continue;
            }
            u_marks.mark(s);
            u_list.push(s);
            let k = self.slot_pos[s];
            if k != usize::MAX {
                u_heap.push(k as u32);
            }
        }
        while let Some(k) = u_heap.pop() {
            let k = k as usize;
            let p = self.u_seq.key[k] as usize;
            if x[p] == 0.0 {
                continue;
            }
            x[p] /= self.u_seq.pivot[k];
            let alpha = -x[p];
            self.add_tick(self.u_seq.nnz(k) as u64);
            // `EtaFile::axpy` と同じ演算に、書いた行の登録を加えたもの。
            let (idx, val) = self.u_seq.seg(k);
            for (&r, &v) in idx.iter().zip(val.iter()) {
                let r = r as usize;
                x[r] += alpha * v;
                if !u_marks.is_marked(r) {
                    u_marks.mark(r);
                    u_list.push(r);
                    let kr = self.slot_pos[r];
                    if kr != usize::MAX {
                        u_heap.push(kr as u32);
                    }
                }
            }
            if u_list.len() > limit {
                // 残り (位置 `k` より前) は全走査で仕上げる (`u_solve_into` の `u_zero_skip` ループと同じ)。
                u_heap.clear();
                for kk in (0..k).rev() {
                    // 死んだヘッダ (`EtaFile::kill`) はピボット 1・要素なしなので処理しても値・tick は変わらない。
                    let p = self.u_seq.key[kk] as usize;
                    if x[p] == 0.0 {
                        continue;
                    }
                    x[p] /= self.u_seq.pivot[kk];
                    let xp = x[p];
                    self.add_tick(self.u_seq.nnz(kk) as u64);
                    self.u_seq.axpy(kk, -xp, x);
                }
                return UHyper::Full;
            }
        }
        UHyper::Hyper
    }

    /// 策12 (部分 `tau`): `U` 段の結果 `x` のうち、`a_list` 中で `alpha_x` が非ゼロのスロット
    /// (入る列の FTRAN 結果の非ゼロ = DSE 重み更新が `tau` を読む行) の値だけを求める。
    /// `x` は `L`/`R` 適用後の値。必要なスロットの依存閉包 (スロット `p` は `row_owners[p]` の
    /// 各 eta のスロットの最終値に依存する) を集め、`u_seq` 位置の降順 (シングルトンは最後) に、
    /// 各スロットへの寄与を `row_owners` から位置の降順に集めて加える。全体の `U` 段
    /// ([`Self::u_solve_into`]) が同じスロットに加える演算と同じ値・同じ順序なので、求めた位置の
    /// 値はビット一致 (シングルトンの除算は出力時)。求めたスロットの閉包は `gp.u_list` に残る。
    ///
    /// 閉包の辺数が [`U_HYPER_ABORT_FRACTION`] `* m` を超える、または `U` に密 eta がある場合は
    /// `x` に触れずに `false` を返す。CLOCK tick は集めた寄与の数だけ加える (全体の `U` 段とは違う)。
    fn u_solve_partial(&self, x: &mut [f64], alpha_x: &[f64], a_list: &[usize], gp: &mut GpScratch, tmp: &mut Vec<(usize, usize, f64)>) -> bool {
        let m = self.base.m;
        if !self.u_seq.dense.is_empty() {
            return false;
        }
        // 閉包の辺数の上限
        let limit = (tunable!("ENOMOTO_T_U_HYPER_ABORT", U_HYPER_ABORT_FRACTION, f64) * m as f64) as usize;
        let GpScratch { u_marks, u_list, .. } = gp;
        u_marks.begin();
        u_list.clear();
        for &s in a_list {
            if alpha_x[s] != 0.0 && !u_marks.is_marked(s) {
                u_marks.mark(s);
                u_list.push(s);
            }
        }
        // 依存閉包 (`u_list` を待ち行列として使う)
        let mut head = 0usize;
        let mut edges = 0usize;
        while head < u_list.len() {
            let p = u_list[head];
            head += 1;
            let owners = &self.row_owners[p];
            edges += owners.len();
            if edges > limit {
                return false;
            }
            for &(q, _) in owners {
                if !u_marks.is_marked(q) {
                    u_marks.mark(q);
                    u_list.push(q);
                }
            }
        }
        // `u_seq` 位置の降順、シングルトン (位置なし) は最後。
        let slot_pos = &self.slot_pos;
        u_list.sort_unstable_by_key(|&p| Reverse(if slot_pos[p] == usize::MAX { 0 } else { slot_pos[p] + 1 }));
        let mut work = 0u64;
        for &p in u_list.iter() {
            let mut xp = x[p];
            tmp.clear();
            tmp.extend(self.row_owners[p].iter().map(|&(q, v)| (slot_pos[q], q, v)));
            tmp.sort_unstable_by_key(|e| Reverse(e.0));
            work += tmp.len() as u64;
            for &(_, q, v) in tmp.iter() {
                let xq = x[q];
                if xq != 0.0 {
                    // 全体の `U` 段の `x[p] += (-x[q]) * v` と同じ演算。
                    xp += -xq * v;
                }
            }
            let k = slot_pos[p];
            if k != usize::MAX && xp != 0.0 {
                xp /= self.u_seq.pivot[k];
            }
            x[p] = xp;
        }
        self.add_tick(work);
        true
    }

    /// `list` のスロットだけを対象にした [`Self::permute_out`] (他のスロットの
    /// `scratch` は 0)。`out` を `fill` でクリアしてから、列挙した値をシングルトン除算
    /// 付きで散布する。非ゼロ数を返す。`u_zero_skip` on かつ微小値切捨てなしのときのみ使う。
    fn permute_list(&self, scratch: &[f64], out: &mut [f64], list: &[usize]) -> usize {
        out.fill(0.0);
        let mut nnz = 0usize;
        let col_perm = &self.base.col_perm;
        let piv = &self.single_piv;
        for &s in list {
            let mut v = scratch[s];
            let d = piv[s];
            if d != 0.0 && v != 0.0 {
                v /= d;
            }
            out[col_perm[s]] = v;
            nnz += (v != 0.0) as usize;
        }
        nnz
    }

    /// 前回の位置を消してから書き、書いた位置を `t` に記録する [`Self::permute_list`] (策1)。
    fn permute_list_tracked(&self, scratch: &[f64], out: &mut [f64], list: &[usize], t: &mut NzTrack) -> usize {
        t.reset(out);
        let mut nnz = 0usize;
        let col_perm = &self.base.col_perm;
        let piv = &self.single_piv;
        for &s in list {
            let mut v = scratch[s];
            let d = piv[s];
            if d != 0.0 && v != 0.0 {
                v /= d;
            }
            let i = col_perm[s];
            out[i] = v;
            t.idx.push(i);
            nnz += (v != 0.0) as usize;
        }
        nnz
    }

    /// 疎 FTRAN が超疎 `U` 段を使ってよいか (呼び出し側の `gp.u_hyper` 要求に加えて、
    /// `u_zero_skip` on かつ微小値切捨てなし)。
    #[inline]
    fn u_hyper_ok(&self, gp: &GpScratch) -> bool {
        gp.u_hyper && self.u_zero_skip && tiny_drop() <= 0.0
    }

    /// 元の行番号のベクトルに `R_k^{-1} ... R_1^{-1} L^{-1}` を適用し、ステップ空間で
    /// `z` に書く (`solve_into` から最後の `U_k^{-1}` を除いたもの)。新しい更新が
    /// `a_q` から `ã_q = (L R_1 ... R_{k-1})^{-1} a_q` (eq. (11)) を得るのにも使う。
    fn ftran_through_l_and_r_into(&self, rhs: &[f64], z: &mut [f64]) {
        self.base.l_solve_into(&self.l_active, rhs, z);
        // CLOCK トリガ用: 密 `L` 段は fill に関係なく `O(m)`。
        self.add_tick(self.base.m as u64);
        for reta in self.r_etas.iter() {
            let dot = self.r_etas.dot(reta.k, z);
            // `R` eta は `z` の疎性に関係なく全部訪れる (ゼロスキップなし) ので、
            // この項で tick が eta 列の長さに比例して増える。
            self.add_tick(self.r_etas.nnz(reta.k) as u64);
            z[reta.slot] -= dot;
        }
    }

    /// `B^-1 rhs` を `out` (長さ `m`) に書く (FTRAN)。`scratch` (長さ `m`) を作業領域に
    /// 使い、確保は行わない。戻り値は結果の非ゼロ数 ([`FtranDensity::record`] 用、
    /// 不要なら無視してよい)。
    pub fn solve_into(&self, rhs: &[f64], scratch: &mut [f64], out: &mut [f64]) -> usize {
        self.ftran_through_l_and_r_into(rhs, scratch);
        self.u_solve_into(scratch);
        self.permute_out(scratch, out)
    }

    /// [`Self::solve_into`] と同じだが、`L`/`R` 適用後・`U` 適用前の中間値
    /// (`(L R_1...R_{k-1})^-1 rhs`) を `a_tilde_out` (長さ `m`) にも書き出す。
    /// `rhs` がこの反復の入力列なら、これがそのまま
    /// [`Self::try_update_precomputed`] に渡す `a_tilde` になる。
    pub fn solve_into_capture(&self, rhs: &[f64], scratch: &mut [f64], out: &mut [f64], a_tilde_out: &mut [f64]) -> usize {
        self.ftran_through_l_and_r_into(rhs, scratch);
        a_tilde_out.copy_from_slice(scratch);
        self.u_solve_into(scratch);
        self.permute_out(scratch, out)
    }

    /// 同じ因子に対する 2 本の密 FTRAN を 1 回の走査で行う:
    /// `out_a = B^-1 rhs_a` ([`Self::solve_into_capture`] と同じ、`a_tilde` 記録込み) と
    /// `out_b = B^-1 rhs_b` ([`Self::solve_into`] と同じ)。入力列の FTRAN と DSE の
    /// `tau = B^-1 rho_p` FTRAN 用。各段 (`L`, `R`, `U`) の因子データを 1 回だけ
    /// 走査し、各ベクトルには単独求解と同じ演算列を適用するので、結果・非ゼロ数・
    /// tick すべて 2 回の個別呼び出しとビット一致。
    ///
    /// - `rho_cap`: `rhs_b` の BTRAN 時に記録した非ゼロステップ ([`StepCapture`])。
    ///   有効なら `rhs_b` の `L` 段を Gilbert-Peierls で行う。
    /// - `ENOMOTO_FTRAN_U_ZERO_SKIP=0` のときは 2 回の個別呼び出しに落ちる。
    #[allow(clippy::too_many_arguments)]
    pub fn solve_into_pair_capture(
        &self,
        rhs_a: &[f64],
        rhs_b: &[f64],
        scratch_a: &mut [f64],
        scratch_b: &mut [f64],
        out_a: &mut [f64],
        out_b: &mut [f64],
        a_tilde_out: &mut [f64],
        rho_cap: Option<&mut StepCapture>,
    ) -> (usize, usize) {
        let rho_cap = StepCapture::take(rho_cap);
        if !self.u_zero_skip {
            let na = self.solve_into_capture(rhs_a, scratch_a, out_a, a_tilde_out);
            let nb = self.solve_into(rhs_b, scratch_b, out_b);
            return (na, nb);
        }
        let m = self.base.m as u64;
        // `L` 段 (+ `ftran_through_l_and_r_into` と同じ一律 `m` の tick をベクトルごとに。
        // `rhs_b` が GP 経路でも同じなので CLOCK トリガは変わらない)。
        // `rhs_b` の `U` 段を超疎に行う場合のスクラッチ
        let hyper_b = if let Some(c) = rho_cap {
            self.base.l_solve_into(&self.l_active, rhs_a, scratch_a);
            self.base.l_solve_steps_into(rhs_b, &c.steps, scratch_b, &mut c.gp);
            if self.u_hyper_ok(&c.gp) {
                Some(&mut c.gp)
            } else {
                None
            }
        } else {
            self.base.l_solve_into_pair(&self.l_active, rhs_a, rhs_b, scratch_a, scratch_b);
            None
        };
        self.add_tick(2 * m);
        let (na, nb) = self.pair_r_u_permute(scratch_a, scratch_b, out_a, out_b, a_tilde_out, None, hyper_b);
        (na, nb)
    }

    /// 入力列を疎で受け取る [`Self::solve_into_pair_capture`]。`rhs_a` は
    /// [`Self::solve_sparse_into_capture`] と同じ Gilbert-Peierls `L` 段
    /// (`scratch_a`/`gp` の前提・事後条件も同じ)、`rhs_b` は密 `L` 段、その後は
    /// 共通の `R`/`U`/置換。`solve_sparse_into_capture(rhs_a, ..)` +
    /// `solve_into(rhs_b, ..)` と tick 込みでビット一致。
    #[allow(clippy::too_many_arguments)]
    pub fn solve_sparse_into_pair_capture(
        &self,
        rhs_a: &[(usize, f64)],
        rhs_b: &[f64],
        scratch_a: &mut [f64],
        gp: &mut GpScratch,
        scratch_b: &mut [f64],
        out_a: &mut [f64],
        out_b: &mut [f64],
        a_tilde_out: &mut [f64],
        rho_cap: Option<&mut StepCapture>,
    ) -> (usize, usize) {
        let rho_cap = StepCapture::take(rho_cap);
        if !self.u_zero_skip {
            let na = self.solve_sparse_into_capture(rhs_a, scratch_a, gp, out_a, a_tilde_out);
            let nb = self.solve_into(rhs_b, scratch_b, out_b);
            return (na, nb);
        }
        self.base.l_solve_sparse_into(rhs_a, scratch_a, gp);
        self.add_tick(gp.reach.len() as u64);
        let hyper_b = match rho_cap {
            Some(c) => {
                self.base.l_solve_steps_into(rhs_b, &c.steps, scratch_b, &mut c.gp);
                if self.u_hyper_ok(&c.gp) {
                    Some(&mut c.gp)
                } else {
                    None
                }
            }
            None => {
                self.base.l_solve_into(&self.l_active, rhs_b, scratch_b);
                None
            }
        };
        self.add_tick(self.base.m as u64);
        let hyper = if self.u_hyper_ok(gp) { Some(gp) } else { None };
        let (na, nb) = self.pair_r_u_permute(scratch_a, scratch_b, out_a, out_b, a_tilde_out, hyper, hyper_b);
        scratch_a.fill(0.0);
        (na, nb)
    }

    /// [`Self::solve_into_pair_capture`] に 3 本目の密 FTRAN `out_c = B^-1 rhs_c`
    /// (同じ反復の BFRT 合成フリップ列) を加え、`L`/`R`/`U` の同じ 1 回の走査を
    /// 共有する。`solve_into_capture(a)` + `solve_into(b)` + `solve_into(c)` と
    /// tick 込みでビット一致。3 つの結果の非ゼロ数を返す。
    #[allow(clippy::too_many_arguments)]
    pub fn solve_into_triple_capture(
        &self,
        rhs_a: &[f64],
        rhs_b: &[f64],
        rhs_c: &[f64],
        scratch_a: &mut [f64],
        scratch_b: &mut [f64],
        scratch_c: &mut [f64],
        out_a: &mut [f64],
        out_b: &mut [f64],
        out_c: &mut [f64],
        a_tilde_out: &mut [f64],
        rho_cap: Option<&mut StepCapture>,
    ) -> (usize, usize, usize) {
        let rho_cap = StepCapture::take(rho_cap);
        if !self.u_zero_skip {
            let na = self.solve_into_capture(rhs_a, scratch_a, out_a, a_tilde_out);
            let nb = self.solve_into(rhs_b, scratch_b, out_b);
            let nc = self.solve_into(rhs_c, scratch_c, out_c);
            return (na, nb, nc);
        }
        let m = self.base.m as u64;
        let hyper_b = if let Some(c) = rho_cap {
            self.base.l_solve_into_pair(&self.l_active, rhs_a, rhs_c, scratch_a, scratch_c);
            self.base.l_solve_steps_into(rhs_b, &c.steps, scratch_b, &mut c.gp);
            if self.u_hyper_ok(&c.gp) {
                Some(&mut c.gp)
            } else {
                None
            }
        } else {
            self.base.l_solve_into_triple(&self.l_active, rhs_a, rhs_b, rhs_c, scratch_a, scratch_b, scratch_c);
            None
        };
        self.add_tick(3 * m);
        self.triple_r_u_permute(scratch_a, scratch_b, scratch_c, out_a, out_b, out_c, a_tilde_out, None, hyper_b)
    }

    /// [`Self::solve_sparse_into_pair_capture`] に [`Self::solve_into_triple_capture`] の
    /// 密な 3 本目の右辺を加えたもの。`scratch_a`/`gp` の約束は pair 版と同じ。
    #[allow(clippy::too_many_arguments)]
    pub fn solve_sparse_into_triple_capture(
        &self,
        rhs_a: &[(usize, f64)],
        rhs_b: &[f64],
        rhs_c: &[f64],
        scratch_a: &mut [f64],
        gp: &mut GpScratch,
        scratch_b: &mut [f64],
        scratch_c: &mut [f64],
        out_a: &mut [f64],
        out_b: &mut [f64],
        out_c: &mut [f64],
        a_tilde_out: &mut [f64],
        rho_cap: Option<&mut StepCapture>,
    ) -> (usize, usize, usize) {
        let rho_cap = StepCapture::take(rho_cap);
        if !self.u_zero_skip {
            let na = self.solve_sparse_into_capture(rhs_a, scratch_a, gp, out_a, a_tilde_out);
            let nb = self.solve_into(rhs_b, scratch_b, out_b);
            let nc = self.solve_into(rhs_c, scratch_c, out_c);
            return (na, nb, nc);
        }
        self.base.l_solve_sparse_into(rhs_a, scratch_a, gp);
        self.add_tick(gp.reach.len() as u64);
        let hyper_b = if let Some(c) = rho_cap {
            self.base.l_solve_into(&self.l_active, rhs_c, scratch_c);
            self.base.l_solve_steps_into(rhs_b, &c.steps, scratch_b, &mut c.gp);
            if self.u_hyper_ok(&c.gp) {
                Some(&mut c.gp)
            } else {
                None
            }
        } else {
            self.base.l_solve_into_pair(&self.l_active, rhs_b, rhs_c, scratch_b, scratch_c);
            None
        };
        self.add_tick(2 * self.base.m as u64);
        let hyper = if self.u_hyper_ok(gp) { Some(gp) } else { None };
        let r = self.triple_r_u_permute(scratch_a, scratch_b, scratch_c, out_a, out_b, out_c, a_tilde_out, hyper, hyper_b);
        scratch_a.fill(0.0);
        r
    }

    /// 3 本目のベクトル `c` (記録なし) を加えた [`Self::pair_r_u_permute`]。
    #[allow(clippy::too_many_arguments)]
    fn triple_r_u_permute(
        &self,
        scratch_a: &mut [f64],
        scratch_b: &mut [f64],
        scratch_c: &mut [f64],
        out_a: &mut [f64],
        out_b: &mut [f64],
        out_c: &mut [f64],
        a_tilde_out: &mut [f64],
        hyper_a: Option<&mut GpScratch>,
        hyper_b: Option<&mut GpScratch>,
    ) -> (usize, usize, usize) {
        let m = self.base.m as u64;
        for reta in self.r_etas.iter() {
            let dot_a = self.r_etas.dot(reta.k, scratch_a);
            let dot_b = self.r_etas.dot(reta.k, scratch_b);
            let dot_c = self.r_etas.dot(reta.k, scratch_c);
            self.add_tick(3 * self.r_etas.nnz(reta.k) as u64);
            scratch_a[reta.slot] -= dot_a;
            scratch_b[reta.slot] -= dot_b;
            scratch_c[reta.slot] -= dot_c;
        }
        a_tilde_out.copy_from_slice(scratch_a);
        self.add_tick(3 * m);
        // C5: ベクトル `a` の `U` 段だけを超疎に行う (ベクトルは独立なので、
        // `a` を融合走査から外しても `b`/`c` の演算は変わらない)。
        // 超疎 `U` 段が成功したときの `a` の到達スロット一覧
        let a_list = match hyper_a {
            Some(gp) => {
                if self.u_solve_hyper(scratch_a, gp) {
                    Some(&gp.u_list)
                } else {
                    None
                }
            }
            None => None,
        };
        // `a` を融合走査で処理するか
        let a_in_scan = a_list.is_none();
        let b_list = match hyper_b {
            Some(gp) => {
                if self.u_solve_hyper(scratch_b, gp) {
                    Some(&gp.u_list)
                } else {
                    None
                }
            }
            None => None,
        };
        let b_in_scan = b_list.is_none();
        for eta in self.u_seq.iter().rev() {
            let p = eta.slot;
            if a_in_scan && scratch_a[p] != 0.0 {
                scratch_a[p] /= self.u_seq.pivot[eta.k];
                let xp = scratch_a[p];
                self.add_tick(self.u_seq.nnz(eta.k) as u64);
                self.u_seq.axpy(eta.k, -xp, scratch_a);
            }
            if b_in_scan && scratch_b[p] != 0.0 {
                scratch_b[p] /= self.u_seq.pivot[eta.k];
                let xp = scratch_b[p];
                self.add_tick(self.u_seq.nnz(eta.k) as u64);
                self.u_seq.axpy(eta.k, -xp, scratch_b);
            }
            if scratch_c[p] != 0.0 {
                scratch_c[p] /= self.u_seq.pivot[eta.k];
                let xp = scratch_c[p];
                self.add_tick(self.u_seq.nnz(eta.k) as u64);
                self.u_seq.axpy(eta.k, -xp, scratch_c);
            }
        }
        // シングルトンの除算は `permute_out` が行う (`single_piv` 参照)。
        let na = match a_list {
            Some(list) => self.permute_list(scratch_a, out_a, list),
            None => self.permute_out(scratch_a, out_a),
        };
        let nb = match b_list {
            Some(list) => self.permute_list(scratch_b, out_b, list),
            None => self.permute_out(scratch_b, out_b),
        };
        let nc = self.permute_out(scratch_c, out_c);
        (na, nb, nc)
    }

    /// 上の 2 つの pair 求解が共有する `L` 以降の処理: `R` eta、`a_tilde` 記録
    /// (ベクトル `a` のみ)、`U` (`u_zero_skip` 形式、`hyper_*` があれば超疎形式)、
    /// 出力置換。各ベクトルに単独求解と同じ演算を適用する。
    fn pair_r_u_permute(
        &self,
        scratch_a: &mut [f64],
        scratch_b: &mut [f64],
        out_a: &mut [f64],
        out_b: &mut [f64],
        a_tilde_out: &mut [f64],
        hyper_a: Option<&mut GpScratch>,
        hyper_b: Option<&mut GpScratch>,
    ) -> (usize, usize) {
        let m = self.base.m as u64;
        for reta in self.r_etas.iter() {
            let dot_a = self.r_etas.dot(reta.k, scratch_a);
            let dot_b = self.r_etas.dot(reta.k, scratch_b);
            self.add_tick(2 * self.r_etas.nnz(reta.k) as u64);
            scratch_a[reta.slot] -= dot_a;
            scratch_b[reta.slot] -= dot_b;
        }
        a_tilde_out.copy_from_slice(scratch_a);
        // `U` 段: ベクトルごとに `u_solve_into` の `u_zero_skip` ループ
        // (C5 なら超疎形式、`triple_r_u_permute` 参照)。
        self.add_tick(2 * m);
        let a_list = match hyper_a {
            Some(gp) => {
                if self.u_solve_hyper(scratch_a, gp) {
                    Some(&gp.u_list)
                } else {
                    None
                }
            }
            None => None,
        };
        let a_in_scan = a_list.is_none();
        let b_list = match hyper_b {
            Some(gp) => {
                if self.u_solve_hyper(scratch_b, gp) {
                    Some(&gp.u_list)
                } else {
                    None
                }
            }
            None => None,
        };
        let b_in_scan = b_list.is_none();
        for eta in self.u_seq.iter().rev() {
            let p = eta.slot;
            if a_in_scan && scratch_a[p] != 0.0 {
                scratch_a[p] /= self.u_seq.pivot[eta.k];
                let xp = scratch_a[p];
                self.add_tick(self.u_seq.nnz(eta.k) as u64);
                self.u_seq.axpy(eta.k, -xp, scratch_a);
            }
            if b_in_scan && scratch_b[p] != 0.0 {
                scratch_b[p] /= self.u_seq.pivot[eta.k];
                let xp = scratch_b[p];
                self.add_tick(self.u_seq.nnz(eta.k) as u64);
                self.u_seq.axpy(eta.k, -xp, scratch_b);
            }
        }
        // シングルトンの除算は `permute_out` が行う (`single_piv` 参照)。
        let na = match a_list {
            Some(list) => self.permute_list(scratch_a, out_a, list),
            None => self.permute_out(scratch_a, out_a),
        };
        let nb = match b_list {
            Some(list) => self.permute_list(scratch_b, out_b, list),
            None => self.permute_out(scratch_b, out_b),
        };
        (na, nb)
    }

    // ---- 以下、出力の非ゼロ位置の記録 (`FtranTrack`) 付きの融合 FTRAN (stormG2 報告 §4 策1・6・12、追加策 R) ----
    // 大きな問題の主ループ専用。小さな問題のホット経路 (上の基底版) のコード配置を変えないよう、
    // 別関数として `#[inline(never)]` にしている。

    /// 同じ因子に対する 2 本の密 FTRAN を 1 回の走査で行う:
    /// `out_a = B^-1 rhs_a` ([`Self::solve_into_capture`] と同じ、`a_tilde` 記録込み) と
    /// `out_b = B^-1 rhs_b` ([`Self::solve_into`] と同じ)。入力列の FTRAN と DSE の
    /// `tau = B^-1 rho_p` FTRAN 用。各段 (`L`, `R`, `U`) の因子データを 1 回だけ
    /// 走査し、各ベクトルには単独求解と同じ演算列を適用するので、結果・非ゼロ数・
    /// tick すべて 2 回の個別呼び出しとビット一致。
    ///
    /// - `rho_cap`: `rhs_b` の BTRAN 時に記録した非ゼロステップ ([`StepCapture`])。
    ///   有効なら `rhs_b` の `L` 段を Gilbert-Peierls で行う。
    /// - `track`: 出力の疎書き出し用の記録 ([`FtranTrack`])。`None` なら従来どおり。
    /// - `ENOMOTO_FTRAN_U_ZERO_SKIP=0` のときは 2 回の個別呼び出しに落ちる。
    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    pub fn solve_into_pair_capture_tracked(
        &self,
        rhs_a: &[f64],
        rhs_b: &[f64],
        scratch_a: &mut [f64],
        scratch_b: &mut [f64],
        out_a: &mut [f64],
        out_b: &mut [f64],
        a_tilde_out: &mut [f64],
        rho_cap: Option<&mut StepCapture>,
        mut track: Option<&mut FtranTrack>,
    ) -> (usize, usize) {
        let rho_cap = StepCapture::take(rho_cap);
        if !self.u_zero_skip {
            if let Some(t) = track {
                t.alpha.set_full();
                t.tau.set_full();
                t.a_tilde.set_full();
                t.b_scratch_clean = false;
                t.tau_partial = false;
            }
            let na = self.solve_into_capture(rhs_a, scratch_a, out_a, a_tilde_out);
            let nb = self.solve_into(rhs_b, scratch_b, out_b);
            return (na, nb);
        }
        let m = self.base.m as u64;
        // `L` 段 (+ `ftran_through_l_and_r_into` と同じ一律 `m` の tick をベクトルごとに。
        // `rhs_b` が GP 経路でも同じなので CLOCK トリガは変わらない)。
        // `rhs_b` の `L` 段を GP で行った場合のスクラッチ
        let gp_b = if let Some(c) = rho_cap {
            self.base.l_solve_into(&self.l_active, rhs_a, scratch_a);
            self.base.l_solve_steps_into_clean(rhs_b, &c.steps, scratch_b, &mut c.gp, take_b_clean(&mut track));
            Some(&mut c.gp)
        } else {
            self.base.l_solve_into_pair(&self.l_active, rhs_a, rhs_b, scratch_a, scratch_b);
            take_b_clean(&mut track);
            None
        };
        self.add_tick(2 * m);
        let (na, nb, _) = self.pair_r_u_permute_tracked(scratch_a, scratch_b, out_a, out_b, a_tilde_out, None, gp_b, track);
        (na, nb)
    }

    /// 入力列を疎で受け取る [`Self::solve_into_pair_capture`]。`rhs_a` は
    /// [`Self::solve_sparse_into_capture`] と同じ Gilbert-Peierls `L` 段
    /// (`scratch_a`/`gp` の前提・事後条件も同じ)、`rhs_b` は密 `L` 段、その後は
    /// 共通の `R`/`U`/置換。`solve_sparse_into_capture(rhs_a, ..)` +
    /// `solve_into(rhs_b, ..)` と tick 込みでビット一致。
    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    pub fn solve_sparse_into_pair_capture_tracked(
        &self,
        rhs_a: &[(usize, f64)],
        rhs_b: &[f64],
        scratch_a: &mut [f64],
        gp: &mut GpScratch,
        scratch_b: &mut [f64],
        out_a: &mut [f64],
        out_b: &mut [f64],
        a_tilde_out: &mut [f64],
        rho_cap: Option<&mut StepCapture>,
        mut track: Option<&mut FtranTrack>,
    ) -> (usize, usize) {
        let rho_cap = StepCapture::take(rho_cap);
        if !self.u_zero_skip {
            if let Some(t) = track {
                t.alpha.set_full();
                t.tau.set_full();
                t.a_tilde.set_full();
                t.b_scratch_clean = false;
                t.tau_partial = false;
            }
            let na = self.solve_sparse_into_capture(rhs_a, scratch_a, gp, out_a, a_tilde_out);
            let nb = self.solve_into(rhs_b, scratch_b, out_b);
            return (na, nb);
        }
        self.base.l_solve_sparse_into(rhs_a, scratch_a, gp);
        self.add_tick(gp.reach.len() as u64);
        let gp_b = match rho_cap {
            Some(c) => {
                self.base.l_solve_steps_into_clean(rhs_b, &c.steps, scratch_b, &mut c.gp, take_b_clean(&mut track));
                Some(&mut c.gp)
            }
            None => {
                self.base.l_solve_into(&self.l_active, rhs_b, scratch_b);
                take_b_clean(&mut track);
                None
            }
        };
        self.add_tick(self.base.m as u64);
        let (na, nb, a_hyper) = self.pair_r_u_permute_tracked(scratch_a, scratch_b, out_a, out_b, a_tilde_out, Some(&mut *gp), gp_b, track);
        clear_after_hyper(scratch_a, a_hyper, gp);
        (na, nb)
    }

    /// [`Self::solve_into_pair_capture`] に 3 本目の密 FTRAN `out_c = B^-1 rhs_c`
    /// (同じ反復の BFRT 合成フリップ列) を加え、`L`/`R`/`U` の同じ 1 回の走査を
    /// 共有する。`solve_into_capture(a)` + `solve_into(b)` + `solve_into(c)` と
    /// tick 込みでビット一致。3 つの結果の非ゼロ数を返す。
    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    pub fn solve_into_triple_capture_tracked(
        &self,
        rhs_a: &[f64],
        rhs_b: &[f64],
        rhs_c: &[f64],
        scratch_a: &mut [f64],
        scratch_b: &mut [f64],
        scratch_c: &mut [f64],
        out_a: &mut [f64],
        out_b: &mut [f64],
        out_c: &mut [f64],
        a_tilde_out: &mut [f64],
        rho_cap: Option<&mut StepCapture>,
        mut track: Option<&mut FtranTrack>,
    ) -> (usize, usize, usize) {
        let rho_cap = StepCapture::take(rho_cap);
        if !self.u_zero_skip {
            if let Some(t) = track {
                t.alpha.set_full();
                t.tau.set_full();
                t.a_tilde.set_full();
                t.b_scratch_clean = false;
                t.tau_partial = false;
            }
            let na = self.solve_into_capture(rhs_a, scratch_a, out_a, a_tilde_out);
            let nb = self.solve_into(rhs_b, scratch_b, out_b);
            let nc = self.solve_into(rhs_c, scratch_c, out_c);
            return (na, nb, nc);
        }
        let m = self.base.m as u64;
        let gp_b = if let Some(c) = rho_cap {
            self.base.l_solve_into_pair(&self.l_active, rhs_a, rhs_c, scratch_a, scratch_c);
            self.base.l_solve_steps_into_clean(rhs_b, &c.steps, scratch_b, &mut c.gp, take_b_clean(&mut track));
            Some(&mut c.gp)
        } else {
            self.base.l_solve_into_triple(&self.l_active, rhs_a, rhs_b, rhs_c, scratch_a, scratch_b, scratch_c);
            take_b_clean(&mut track);
            None
        };
        self.add_tick(3 * m);
        let (na, nb, nc, _) = self.triple_r_u_permute_tracked(scratch_a, scratch_b, scratch_c, out_a, out_b, out_c, a_tilde_out, None, gp_b, track);
        (na, nb, nc)
    }

    /// [`Self::solve_sparse_into_pair_capture`] に [`Self::solve_into_triple_capture`] の
    /// 密な 3 本目の右辺を加えたもの。`scratch_a`/`gp` の約束は pair 版と同じ。
    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    pub fn solve_sparse_into_triple_capture_tracked(
        &self,
        rhs_a: &[(usize, f64)],
        rhs_b: &[f64],
        rhs_c: &[f64],
        scratch_a: &mut [f64],
        gp: &mut GpScratch,
        scratch_b: &mut [f64],
        scratch_c: &mut [f64],
        out_a: &mut [f64],
        out_b: &mut [f64],
        out_c: &mut [f64],
        a_tilde_out: &mut [f64],
        rho_cap: Option<&mut StepCapture>,
        mut track: Option<&mut FtranTrack>,
    ) -> (usize, usize, usize) {
        let rho_cap = StepCapture::take(rho_cap);
        if !self.u_zero_skip {
            if let Some(t) = track {
                t.alpha.set_full();
                t.tau.set_full();
                t.a_tilde.set_full();
                t.b_scratch_clean = false;
                t.tau_partial = false;
            }
            let na = self.solve_sparse_into_capture(rhs_a, scratch_a, gp, out_a, a_tilde_out);
            let nb = self.solve_into(rhs_b, scratch_b, out_b);
            let nc = self.solve_into(rhs_c, scratch_c, out_c);
            return (na, nb, nc);
        }
        self.base.l_solve_sparse_into(rhs_a, scratch_a, gp);
        self.add_tick(gp.reach.len() as u64);
        let gp_b = if let Some(c) = rho_cap {
            self.base.l_solve_into(&self.l_active, rhs_c, scratch_c);
            self.base.l_solve_steps_into_clean(rhs_b, &c.steps, scratch_b, &mut c.gp, take_b_clean(&mut track));
            Some(&mut c.gp)
        } else {
            self.base.l_solve_into_pair(&self.l_active, rhs_b, rhs_c, scratch_b, scratch_c);
            take_b_clean(&mut track);
            None
        };
        self.add_tick(2 * self.base.m as u64);
        let (na, nb, nc, a_hyper) = self.triple_r_u_permute_tracked(scratch_a, scratch_b, scratch_c, out_a, out_b, out_c, a_tilde_out, Some(&mut *gp), gp_b, track);
        clear_after_hyper(scratch_a, a_hyper, gp);
        (na, nb, nc)
    }

    /// 追加策 R: `R` eta (`x[p_k] -= r_k · x`、作成順) を、非ゼロになりうる位置が `seeds` に限られる
    /// 疎なベクトル `x` に当てる。内積が 0 でありえない eta (非ゼロの位置に要素を持つもの) だけを
    /// 列方向索引 ([`Self::r_head`]) で選び、作成順 (番号の小さい順) に最小ヒープで処理する。
    /// eta `k` が `x[p_k]` を変えたら、`p_k` に要素を持つ `k` より新しい eta を加える。選ばれない
    /// eta の内積は `±0` の和で `x` を変えないので、全 eta を当てるのと値はビット一致。
    /// 内積が 0 でなかった eta の行を `r_seeds` に積む。tick は呼び出し側が全 eta 分を加える。
    /// 選んだ eta が全体の半分を超えたら、残りは全 eta を順に当てる。
    fn apply_r_sparse(&self, x: &mut [f64], seeds: &[usize], r_seeds: &mut Vec<usize>, work: &mut RSparseWork) {
        #[cfg(test)]
        tests::R_SPARSE_CALLS.with(|c| c.set(c.get() + 1));
        let n_r = self.r_etas.n_headers();
        r_seeds.clear();
        if n_r == 0 {
            return;
        }
        if work.stamp.len() < n_r {
            work.stamp.resize(n_r, 0);
        }
        work.epoch = work.epoch.wrapping_add(1);
        if work.epoch == 0 {
            work.stamp.iter_mut().for_each(|v| *v = 0);
            work.epoch = 1;
        }
        let epoch = work.epoch;
        let heap = &mut work.heap;
        heap.clear();
        let stamp = &mut work.stamp;
        for &i in seeds {
            let mut e = self.r_head[i];
            while e != u32::MAX {
                let k = self.r_owner[e as usize];
                if stamp[k as usize] != epoch {
                    stamp[k as usize] = epoch;
                    heap.push(Reverse(k));
                }
                e = self.r_next[e as usize];
            }
        }
        for &k in &self.r_dense {
            if stamp[k as usize] != epoch {
                stamp[k as usize] = epoch;
                heap.push(Reverse(k));
            }
        }
        // 処理した eta の数
        let mut done = 0usize;
        while let Some(Reverse(k)) = heap.pop() {
            let k = k as usize;
            let slot = self.r_etas.key[k] as usize;
            let dot = self.r_etas.dot(k, x);
            x[slot] -= dot;
            done += 1;
            if dot != 0.0 {
                r_seeds.push(slot);
                if 2 * done > n_r {
                    // 残り (`k` より新しい eta) は全部順に当てる。
                    heap.clear();
                    for k2 in k + 1..n_r {
                        let slot = self.r_etas.key[k2] as usize;
                        let dot = self.r_etas.dot(k2, x);
                        x[slot] -= dot;
                        if dot != 0.0 {
                            r_seeds.push(slot);
                        }
                    }
                    return;
                }
                let mut e = self.r_head[slot];
                while e != u32::MAX {
                    let k2 = self.r_owner[e as usize];
                    if k2 as usize > k && stamp[k2 as usize] != epoch {
                        stamp[k2 as usize] = epoch;
                        heap.push(Reverse(k2));
                    }
                    e = self.r_next[e as usize];
                }
            }
        }
    }

    /// `R` eta 適用後の `scratch_a` を `a_tilde_out` に記録する (FT 更新用)。
    /// `gp_a` (`a` の `L` 段が Gilbert-Peierls で、到達集合 `reach` が有効) と `track` が
    /// あれば、非ゼロになりうる位置 (`reach` ∪ 値が非ゼロの `R` eta の行) だけを書いて
    /// `track.a_tilde` に一覧を残す (策6: `O(m)` の `copy_from_slice` を省く)。
    /// `gp_a.visited` に `R` eta の行の印を加える (重複除去用)。
    fn capture_a_tilde(&self, scratch_a: &[f64], a_tilde_out: &mut [f64], gp_a: Option<&mut GpScratch>, track: Option<&mut FtranTrack>) {
        match (gp_a, track) {
            (Some(gp), Some(t)) => {
                let at = &mut t.a_tilde;
                at.reset(a_tilde_out);
                for &s in &gp.reach {
                    a_tilde_out[s] = scratch_a[s];
                    at.idx.push(s);
                }
                // 到達集合外で値が変わりうるのは `R` eta で値が変わった行 (`r_seeds`) だけ。
                for i in 0..gp.r_seeds.len() {
                    let s = gp.r_seeds[i];
                    if scratch_a[s] != 0.0 && !gp.visited.is_marked(s) {
                        gp.visited.mark(s);
                        a_tilde_out[s] = scratch_a[s];
                        at.idx.push(s);
                    }
                }
            }
            (_, t) => {
                a_tilde_out.copy_from_slice(scratch_a);
                if let Some(t) = t {
                    t.a_tilde.set_full();
                }
            }
        }
    }

    /// `gp` があり [`Self::u_hyper_ok`] なら `x` の `U` 段を超疎に試みる ([`UHyper`])。
    #[inline]
    fn try_u_hyper(&self, x: &mut [f64], gp: Option<&mut GpScratch>) -> UHyper {
        match gp {
            Some(gp) if self.u_hyper_ok(gp) => self.u_solve_hyper_heap(x, gp),
            _ => UHyper::NotTried,
        }
    }

    /// `R` eta の適用で `x[slot]` から引いた値 `dot` が 0 でなければ、`slot` を超疎 `U` 段の
    /// 起点候補 (`gp.r_seeds`) に加える。
    #[inline(always)]
    fn note_r_seed(gp: &mut Option<&mut GpScratch>, slot: usize, dot: f64) {
        if dot != 0.0 {
            if let Some(g) = gp.as_deref_mut() {
                g.r_seeds.push(slot);
            }
        }
    }

    /// 融合 FTRAN の出力置換 1 本分: 超疎 `U` 段が成功した (`list` が `Some`) なら
    /// 一覧の位置だけを書き (`track` があれば前回の位置を消してから書き、位置を記録)、
    /// そうでなければ全体を書く (`track` は無効化)。
    #[inline]
    fn permute_fused_out(&self, scratch: &[f64], out: &mut [f64], list: Option<&[usize]>, track: Option<&mut NzTrack>) -> usize {
        match (list, track) {
            (Some(list), Some(t)) => self.permute_list_tracked(scratch, out, list, t),
            (Some(list), None) => self.permute_list(scratch, out, list),
            (None, t) => {
                if let Some(t) = t {
                    t.set_full();
                }
                self.permute_out(scratch, out)
            }
        }
    }

    /// 3 本目のベクトル `c` (記録なし) を加えた [`Self::pair_r_u_permute_tracked`]。
    #[allow(clippy::too_many_arguments)]
    fn triple_r_u_permute_tracked(
        &self,
        scratch_a: &mut [f64],
        scratch_b: &mut [f64],
        scratch_c: &mut [f64],
        out_a: &mut [f64],
        out_b: &mut [f64],
        out_c: &mut [f64],
        a_tilde_out: &mut [f64],
        mut gp_a: Option<&mut GpScratch>,
        mut gp_b: Option<&mut GpScratch>,
        mut track: Option<&mut FtranTrack>,
    ) -> (usize, usize, usize, bool) {
        let m = self.base.m as u64;
        clear_r_seeds(&mut gp_a, &mut gp_b);
        for reta in self.r_etas.iter() {
            let dot_a = self.r_etas.dot(reta.k, scratch_a);
            let dot_b = self.r_etas.dot(reta.k, scratch_b);
            let dot_c = self.r_etas.dot(reta.k, scratch_c);
            self.add_tick(3 * self.r_etas.nnz(reta.k) as u64);
            scratch_a[reta.slot] -= dot_a;
            scratch_b[reta.slot] -= dot_b;
            scratch_c[reta.slot] -= dot_c;
            Self::note_r_seed(&mut gp_a, reta.slot, dot_a);
            Self::note_r_seed(&mut gp_b, reta.slot, dot_b);
        }
        self.capture_a_tilde(scratch_a, a_tilde_out, gp_a.as_deref_mut(), track.as_deref_mut());
        self.add_tick(3 * m);
        // C5: ベクトル `a`/`b` の `U` 段を超疎に行う (ベクトルは独立なので、
        // 融合走査から外しても他のベクトルの演算は変わらない)。
        let a_u = self.try_u_hyper(scratch_a, gp_a.as_deref_mut());
        let b_u = self.try_u_hyper(scratch_b, gp_b.as_deref_mut());
        // 融合走査に含めるか (超疎段を試さなかったベクトルだけ)
        let (a_scan, b_scan) = (a_u == UHyper::NotTried, b_u == UHyper::NotTried);
        for eta in self.u_seq.iter().rev() {
            let p = eta.slot;
            if a_scan && scratch_a[p] != 0.0 {
                scratch_a[p] /= self.u_seq.pivot[eta.k];
                let xp = scratch_a[p];
                self.add_tick(self.u_seq.nnz(eta.k) as u64);
                self.u_seq.axpy(eta.k, -xp, scratch_a);
            }
            if b_scan && scratch_b[p] != 0.0 {
                scratch_b[p] /= self.u_seq.pivot[eta.k];
                let xp = scratch_b[p];
                self.add_tick(self.u_seq.nnz(eta.k) as u64);
                self.u_seq.axpy(eta.k, -xp, scratch_b);
            }
            if scratch_c[p] != 0.0 {
                scratch_c[p] /= self.u_seq.pivot[eta.k];
                let xp = scratch_c[p];
                self.add_tick(self.u_seq.nnz(eta.k) as u64);
                self.u_seq.axpy(eta.k, -xp, scratch_c);
            }
        }
        // シングルトンの除算は `permute_out` が行う (`single_piv` 参照)。
        let a_list = if a_u == UHyper::Hyper { gp_a.as_deref().map(|g| g.u_list.as_slice()) } else { None };
        let b_list = if b_u == UHyper::Hyper { gp_b.as_deref().map(|g| g.u_list.as_slice()) } else { None };
        let na = self.permute_fused_out(scratch_a, out_a, a_list, track.as_deref_mut().map(|t| &mut t.alpha));
        let nb = self.permute_fused_out(scratch_b, out_b, b_list, track.as_deref_mut().map(|t| &mut t.tau));
        let nc = self.permute_out(scratch_c, out_c);
        finish_b_scratch(scratch_b, b_list, track);
        (na, nb, nc, a_u == UHyper::Hyper)
    }

    /// 上の 2 つの pair 求解が共有する `L` 以降の処理: `R` eta、`a_tilde` 記録
    /// (ベクトル `a` のみ)、`U` (`u_zero_skip` 形式、`gp_*` があり [`Self::u_hyper_ok`] なら
    /// 超疎形式)、出力置換。各ベクトルに単独求解と同じ演算を適用する。
    ///
    /// - `gp_a`/`gp_b`: そのベクトルの `L` 段を Gilbert-Peierls で行った場合のスクラッチ
    ///   (到達集合 `reach` が有効)。
    /// - 戻り値の 3 つ目は `a` の `U` 段を超疎に行ったか (呼び出し側が `scratch_a` を
    ///   `gp_a.u_list` の位置だけ 0 に戻せる)。
    /// - 両ベクトルとも超疎 `U` 段なら `u_seq` の全 eta 走査自体を省く。
    #[allow(clippy::too_many_arguments)]
    fn pair_r_u_permute_tracked(
        &self,
        scratch_a: &mut [f64],
        scratch_b: &mut [f64],
        out_a: &mut [f64],
        out_b: &mut [f64],
        a_tilde_out: &mut [f64],
        mut gp_a: Option<&mut GpScratch>,
        mut gp_b: Option<&mut GpScratch>,
        mut track: Option<&mut FtranTrack>,
    ) -> (usize, usize, bool) {
        let m = self.base.m as u64;
        if let (true, Some(ga), Some(gb)) = (self.r_sparse_ready(), gp_a.as_deref_mut(), gp_b.as_deref_mut()) {
            // 両ベクトルとも `L` 段が Gilbert-Peierls (非ゼロ位置が分かっている): 追加策 R の疎な `R` 段。
            self.add_tick(2 * self.r_nnz_total as u64);
            let GpScratch { reach, r_seeds, r_work, .. } = ga;
            self.apply_r_sparse(scratch_a, reach, r_seeds, r_work);
            let GpScratch { reach, r_seeds, r_work, .. } = gb;
            self.apply_r_sparse(scratch_b, reach, r_seeds, r_work);
        } else {
            clear_r_seeds(&mut gp_a, &mut gp_b);
            for reta in self.r_etas.iter() {
                let dot_a = self.r_etas.dot(reta.k, scratch_a);
                let dot_b = self.r_etas.dot(reta.k, scratch_b);
                self.add_tick(2 * self.r_etas.nnz(reta.k) as u64);
                scratch_a[reta.slot] -= dot_a;
                scratch_b[reta.slot] -= dot_b;
                Self::note_r_seed(&mut gp_a, reta.slot, dot_a);
                Self::note_r_seed(&mut gp_b, reta.slot, dot_b);
            }
        }
        self.capture_a_tilde(scratch_a, a_tilde_out, gp_a.as_deref_mut(), track.as_deref_mut());
        // `U` 段: ベクトルごとに `u_solve_into` の `u_zero_skip` ループ
        // (C5 なら超疎形式、`triple_r_u_permute` 参照)。
        self.add_tick(2 * m);
        let a_u = self.try_u_hyper(scratch_a, gp_a.as_deref_mut());
        // 策12: 要求があり `a` が超疎に解けたら、`tau` は `a` の非ゼロのスロットでだけ求める。
        let b_partial = match (a_u, gp_a.as_deref(), gp_b.as_deref_mut(), track.as_deref_mut()) {
            (UHyper::Hyper, Some(ga), Some(gb), Some(t)) if t.partial_tau && self.u_hyper_ok(gb) => self.u_solve_partial(scratch_b, scratch_a, &ga.u_list, gb, &mut t.owners_tmp),
            _ => false,
        };
        let b_u = if b_partial { UHyper::Hyper } else { self.try_u_hyper(scratch_b, gp_b.as_deref_mut()) };
        // 融合走査に含めるか (超疎段を試さなかったベクトルだけ)
        let (a_scan, b_scan) = (a_u == UHyper::NotTried, b_u == UHyper::NotTried);
        if a_scan || b_scan {
            for eta in self.u_seq.iter().rev() {
                let p = eta.slot;
                if a_scan && scratch_a[p] != 0.0 {
                    scratch_a[p] /= self.u_seq.pivot[eta.k];
                    let xp = scratch_a[p];
                    self.add_tick(self.u_seq.nnz(eta.k) as u64);
                    self.u_seq.axpy(eta.k, -xp, scratch_a);
                }
                if b_scan && scratch_b[p] != 0.0 {
                    scratch_b[p] /= self.u_seq.pivot[eta.k];
                    let xp = scratch_b[p];
                    self.add_tick(self.u_seq.nnz(eta.k) as u64);
                    self.u_seq.axpy(eta.k, -xp, scratch_b);
                }
            }
        }
        // シングルトンの除算は `permute_out` が行う (`single_piv` 参照)。
        let a_list = if a_u == UHyper::Hyper { gp_a.as_deref().map(|g| g.u_list.as_slice()) } else { None };
        let na = self.permute_fused_out(scratch_a, out_a, a_list, track.as_deref_mut().map(|t| &mut t.alpha));
        if b_partial {
            // 部分 `tau`: `a` の非ゼロのスロットだけを書き、`scratch_b` の触れた位置を 0 に戻す。
            let (ga, gb, t) = (gp_a.as_deref().unwrap(), gp_b.as_deref().unwrap(), track.unwrap());
            t.tau.reset(out_b);
            let mut nb = 0usize;
            for &s in &ga.u_list {
                if scratch_a[s] == 0.0 {
                    continue;
                }
                let mut v = scratch_b[s];
                let d = self.single_piv[s];
                if d != 0.0 && v != 0.0 {
                    v /= d;
                }
                let i = self.base.col_perm[s];
                out_b[i] = v;
                t.tau.idx.push(i);
                nb += (v != 0.0) as usize;
            }
            for &s in gb.reach.iter().chain(gb.r_seeds.iter()).chain(gb.u_list.iter()) {
                scratch_b[s] = 0.0;
            }
            t.b_scratch_clean = true;
            t.tau_partial = true;
            return (na, nb, true);
        }
        let b_list = if b_u == UHyper::Hyper { gp_b.as_deref().map(|g| g.u_list.as_slice()) } else { None };
        let nb = self.permute_fused_out(scratch_b, out_b, b_list, track.as_deref_mut().map(|t| &mut t.tau));
        finish_b_scratch(scratch_b, b_list, track);
        (na, nb, a_u == UHyper::Hyper)
    }

    /// 全 FTRAN 経路共通の最終段: 完成したステップ空間ベクトルを元の順
    /// (`out[col_perm[s]] = scratch[s]`) に戻し、非ゼロ数を返す
    /// (カウントは分岐なしの加算)。`U` 段のシングルトン除算
    /// ([`Self::single_piv`]) もここで行う (`ENOMOTO_FTRAN_U_ZERO_SKIP=0` 時を除く)。
    /// [`tiny_drop`] 未満は 0 にする。
    #[inline]
    fn permute_out(&self, scratch: &[f64], out: &mut [f64]) -> usize {
        let mut nnz = 0usize;
        let tiny = tiny_drop();
        let m = self.base.m;
        let col_perm = &self.base.col_perm[..m];
        let scratch = &scratch[..m];
        if !self.u_zero_skip {
            for s in 0..m {
                let v = scratch[s];
                let v = if tiny > 0.0 && v.abs() < tiny { 0.0 } else { v };
                out[col_perm[s]] = v;
                nnz += (v != 0.0) as usize;
            }
            return nnz;
        }
        let piv = &self.single_piv[..m];
        if tiny > 0.0 {
            for s in 0..m {
                let mut v = scratch[s];
                let d = piv[s];
                if d != 0.0 && v != 0.0 {
                    v /= d;
                }
                let v = if v.abs() < tiny { 0.0 } else { v };
                out[col_perm[s]] = v;
                nnz += (v != 0.0) as usize;
            }
            return nnz;
        }
        for s in 0..m {
            let mut v = scratch[s];
            let d = piv[s];
            if d != 0.0 && v != 0.0 {
                v /= d;
            }
            out[col_perm[s]] = v;
            nnz += (v != 0.0) as usize;
        }
        nnz
    }

    /// 疎な右辺版の [`Self::solve_into`]: `rhs` の非ゼロ `(元の行, 値)` を直接受け取り、
    /// `L` 段を [`LuFactors::l_solve_sparse_into`] で行う。非ゼロ数を返す。
    ///
    /// **`scratch`/`gp` はこの呼び出し箇所専用にすること** ([`Self::solve_into`] の
    /// バッファと共有しない)。`l_solve_sparse_into` は入口で `scratch` が全 0 である
    /// ことを要求し、この関数は返却直前に `scratch` を全 0 に戻してその前提を
    /// 次回のために保つ。
    pub fn solve_sparse_into(&self, rhs_sparse: &[(usize, f64)], scratch: &mut [f64], gp: &mut GpScratch, out: &mut [f64]) -> usize {
        self.base.l_solve_sparse_into(rhs_sparse, scratch, gp);
        // CLOCK トリガ用: 疎 `L` 段のコストは到達集合のサイズ。
        self.add_tick(gp.reach.len() as u64);
        for reta in self.r_etas.iter() {
            let dot = self.r_etas.dot(reta.k, scratch);
            self.add_tick(self.r_etas.nnz(reta.k) as u64);
            scratch[reta.slot] -= dot;
        }
        // `U` は通常の `u_solve_into` 走査のまま。
        self.u_solve_into(scratch);
        let nnz = self.permute_out(scratch, out);
        scratch.fill(0.0);
        nnz
    }

    /// [`Self::solve_sparse_into`] に、`gp.u_hyper` の要求で超疎 `U` 段・疎な `R` 段 (追加策 R) を加えたもの
    /// (大きな問題の BFRT 合成フリップ列用、値・tick はビット一致)。以下は元の説明:
    /// 疎な右辺版の [`Self::solve_into`]: `rhs` の非ゼロ `(元の行, 値)` を直接受け取り、
    /// `L` 段を [`LuFactors::l_solve_sparse_into`] で行う。非ゼロ数を返す。
    ///
    /// **`scratch`/`gp` はこの呼び出し箇所専用にすること** ([`Self::solve_into`] の
    /// バッファと共有しない)。`l_solve_sparse_into` は入口で `scratch` が全 0 である
    /// ことを要求し、この関数は返却直前に `scratch` を全 0 に戻してその前提を
    /// 次回のために保つ。
    #[inline(never)]
    pub fn solve_sparse_into_hyper(&self, rhs_sparse: &[(usize, f64)], scratch: &mut [f64], gp: &mut GpScratch, out: &mut [f64]) -> usize {
        self.base.l_solve_sparse_into(rhs_sparse, scratch, gp);
        // CLOCK トリガ用: 疎 `L` 段のコストは到達集合のサイズ。
        self.add_tick(gp.reach.len() as u64);
        if self.u_hyper_ok(gp) && self.r_sparse_ready() {
            // 疎な `R` 段 (追加策 R、tick は全 eta 分を一括で)。
            self.add_tick(self.r_nnz_total as u64);
            let GpScratch { reach, r_seeds, r_work, .. } = &mut *gp;
            self.apply_r_sparse(scratch, reach, r_seeds, r_work);
        } else {
            gp.r_seeds.clear();
            for reta in self.r_etas.iter() {
                let dot = self.r_etas.dot(reta.k, scratch);
                self.add_tick(self.r_etas.nnz(reta.k) as u64);
                scratch[reta.slot] -= dot;
                if dot != 0.0 {
                    gp.r_seeds.push(reta.slot);
                }
            }
        }
        if self.u_hyper_ok(gp) {
            // `gp.u_hyper` の要求があれば `U` 段を超疎に試みる (BFRT の合成フリップ列など。値・tick はビット一致)。
            self.add_tick(self.base.m as u64);
            match self.u_solve_hyper_heap(scratch, gp) {
                UHyper::Hyper => {
                    let nnz = self.permute_list(scratch, out, &gp.u_list);
                    clear_after_hyper(scratch, true, gp);
                    return nnz;
                }
                UHyper::Full => {
                    let nnz = self.permute_out(scratch, out);
                    scratch.fill(0.0);
                    return nnz;
                }
                UHyper::NotTried => self.tick.set(self.tick.get() - self.base.m as u64),
            }
        }
        self.u_solve_into(scratch);
        let nnz = self.permute_out(scratch, out);
        scratch.fill(0.0);
        nnz
    }

    /// 右辺が恒等的に 0 のときに [`Self::solve_into`] (`sparse == false`) または
    /// [`Self::solve_sparse_into`] (`sparse == true`) が加えるのと同じ tick だけを、
    /// 求解せずに加える。結果が 0 と分かっている FTRAN を省いても CLOCK トリガ
    /// (ひいてはピボット経路) をビット一致に保つため。
    pub fn add_zero_rhs_solve_ticks(&self, sparse: bool) {
        let m = self.base.m as u64;
        self.add_tick(if sparse { 0 } else { m });
        for reta in self.r_etas.iter() {
            self.add_tick(self.r_etas.nnz(reta.k) as u64);
        }
        self.add_tick(m);
    }

    /// [`Self::add_zero_rhs_solve_ticks`] の BTRAN 版: 右辺 0 のとき
    /// [`Self::solve_transpose_into`] が加える tick (`U^T` 掃引の `m` と `L^T` 段の `m`)
    /// だけを加える。
    pub fn add_zero_rhs_btran_ticks(&self) {
        let m = self.base.m as u64;
        self.add_tick(m);
        self.add_tick(m);
    }

    /// [`Self::solve_sparse_into`] と同じだが、`L`/`R` 適用後・`U` 適用前の中間値を
    /// `a_tilde_out` (長さ `m`) にも書き出す (疎経路版の [`Self::solve_into_capture`])。
    /// `gp.u_hyper` が有効なら `U` 段を超疎に試みる。
    pub fn solve_sparse_into_capture(
        &self,
        rhs_sparse: &[(usize, f64)],
        scratch: &mut [f64],
        gp: &mut GpScratch,
        out: &mut [f64],
        a_tilde_out: &mut [f64],
    ) -> usize {
        self.base.l_solve_sparse_into(rhs_sparse, scratch, gp);
        self.add_tick(gp.reach.len() as u64);
        for reta in self.r_etas.iter() {
            let dot = self.r_etas.dot(reta.k, scratch);
            self.add_tick(self.r_etas.nnz(reta.k) as u64);
            scratch[reta.slot] -= dot;
        }
        a_tilde_out.copy_from_slice(scratch);
        if self.u_hyper_ok(gp) {
            self.add_tick(self.base.m as u64);
            if self.u_solve_hyper(scratch, gp) {
                let nnz = self.permute_list(scratch, out, &gp.u_list);
                scratch.fill(0.0);
                return nnz;
            }
            // `scratch` に触れる前に中断した: 既に加えた一律 tick を戻して全走査する。
            self.tick.set(self.tick.get() - self.base.m as u64);
        }
        self.u_solve_into(scratch);
        let nnz = self.permute_out(scratch, out);
        scratch.fill(0.0);
        nnz
    }

    /// [`Self::solve_into`] の確保付き簡易版 (テストや再利用バッファを持たない呼び出し用)。
    pub fn solve(&self, rhs: &[f64]) -> Vec<f64> {
        let m = self.base.m;
        let mut scratch = vec![0.0; m];
        let mut out = vec![0.0; m];
        self.solve_into(rhs, &mut scratch, &mut out);
        out
    }

    /// `B^-T rhs` を `out` (長さ `m`) に書く (BTRAN)。`scratch` (長さ `m`) を作業領域に
    /// 使い、確保は行わない。
    pub fn solve_transpose_into(&self, rhs: &[f64], scratch: &mut [f64], out: &mut [f64]) {
        self.permute_transpose_rhs(rhs, scratch);
        self.u_transpose_solve_into(scratch);
        self.btran_tail(scratch, out);
    }

    /// `P_col^{-1} rhs` を `scratch` に書く (全 BTRAN の最初の段、`O(m)` のギャザー)。
    #[inline]
    fn permute_transpose_rhs(&self, rhs: &[f64], scratch: &mut [f64]) {
        for s in 0..self.base.m {
            scratch[s] = rhs[self.base.col_perm[s]];
        }
    }

    /// `rhs = e_i` 用の [`Self::permute_transpose_rhs`]。`scratch` を 0 で埋めて
    /// ステップ `col_perm_inv[i]` に 1 を置き、そのステップを返す。
    #[inline]
    fn seed_unit_rhs(&self, i: usize, scratch: &mut [f64]) -> usize {
        scratch.fill(0.0);
        let s0 = self.base.col_perm_inv[i];
        scratch[s0] = 1.0;
        s0
    }

    /// BTRAN の `U^{-T}` 以降すべて: `R` eta を逆順に適用し、`L^{-T}` で元の行順に戻す。
    #[inline]
    fn btran_tail(&self, scratch: &mut [f64], out: &mut [f64]) {
        self.btran_tail_cap(scratch, out, None)
    }

    /// 結果の非ゼロステップ記録 ([`StepCapture`]) を任意で付けた [`Self::btran_tail`]。
    #[inline]
    fn btran_tail_cap(&self, scratch: &mut [f64], out: &mut [f64], cap: Option<&mut StepCapture>) {
        // 超疎: `yp` が各 `R` eta の要素に掛かる唯一の値なので、0 なら飛ばす。
        for reta in self.r_etas.iter().rev() {
            let yp = scratch[reta.slot];
            if yp == 0.0 {
                continue;
            }
            self.add_tick(self.r_etas.nnz(reta.k) as u64);
            self.r_etas.axpy(reta.k, -yp, scratch);
        }
        // CLOCK トリガ用: `L^{-T}` 段は一律 `m` として数える。
        self.add_tick(self.base.m as u64);
        self.l_transpose_solve_into_cap(scratch, out, cap);
    }

    /// `rhs = e_i` 専用の [`Self::solve_transpose_into`] (ピボット行 BTRAN の形)。
    /// FT 更新後でも有効で、呼び出し側は `e_i` バッファを持つ必要がない。
    /// `scratch` は入口で上書きされるので前提条件なし。
    pub fn solve_transpose_unit(&self, i: usize, scratch: &mut [f64], out: &mut [f64]) {
        let s0 = self.seed_unit_rhs(i, scratch);
        self.u_transpose_solve_seeded(scratch, s0);
        self.btran_tail(scratch, out);
    }

    /// [`Self::solve_transpose_unit`] に `e_tilde` の記録
    /// ([`Self::solve_transpose_into_capture`] と同じ) を加えたもの。
    pub fn solve_transpose_unit_capture(&self, i: usize, scratch: &mut [f64], out: &mut [f64], e_tilde_out: &mut [f64]) {
        self.solve_transpose_unit_capture_steps(i, scratch, out, e_tilde_out, None)
    }

    /// 呼び出し側スクラッチの代わりに [`UnitBtranWork`] を使う
    /// [`Self::solve_transpose_unit_capture_steps`]。`out`・`e_tilde_out`・記録・tick は
    /// ビット一致。
    pub fn solve_transpose_unit_work(&self, i: usize, out: &mut [f64], e_tilde_out: &mut [f64], work: &mut UnitBtranWork, cap: Option<&mut StepCapture>) {
        let m = self.base.m;
        if work.w.len() != m {
            work.w = vec![0.0; m];
            work.e_full = true;
        }
        let w = &mut work.w;
        debug_assert!(w.iter().all(|&v| v == 0.0));
        let s0 = self.base.col_perm_inv[i];
        w[s0] = 1.0;
        work.touch.clear();
        work.touch.push(s0);
        // 非ゼロ位置の記録が溢れずに済んだか
        let tracked = self.u_transpose_sweep_track(w, &mut work.touch);
        // `e_tilde` の記録: 前回の位置を消してから今回の値を書く。
        if work.e_full {
            e_tilde_out.fill(0.0);
        } else {
            for &q in &work.e_touch {
                e_tilde_out[q] = 0.0;
            }
        }
        // `L^T` 段入力の非ゼロ数の上界
        let mut bound;
        if tracked {
            for &q in &work.touch {
                e_tilde_out[q] = w[q];
            }
            bound = work.touch.len();
            std::mem::swap(&mut work.touch, &mut work.e_touch);
            work.e_full = false;
        } else {
            e_tilde_out.copy_from_slice(w);
            bound = usize::MAX;
            work.e_full = true;
        }
        // `btran_tail` と同じ処理を、非ゼロ数の上界を `R` eta 分増やしながら行う。
        for reta in self.r_etas.iter().rev() {
            let yp = w[reta.slot];
            if yp == 0.0 {
                continue;
            }
            let nnz = self.r_etas.nnz(reta.k);
            self.add_tick(nnz as u64);
            bound = bound.saturating_add(nnz);
            self.r_etas.axpy(reta.k, -yp, w);
        }
        self.add_tick(m as u64);
        self.l_transpose_solve_into_ext::<true>(w, out, cap, bound);
    }

    /// [`Self::solve_transpose_unit_work`] の超疎版 (策2、大きな問題の主ループ用。結果はビット一致):
    /// `U^T`/`L^T` 段を非ゼロ位置のヒープで駆動し、出力も一覧の位置だけ書く ([`UnitBtranWork`] 参照)。
    #[inline(never)]
    pub fn solve_transpose_unit_work_sparse(&self, i: usize, out: &mut [f64], e_tilde_out: &mut [f64], work: &mut UnitBtranWork, mut cap: Option<&mut StepCapture>) {
        let m = self.base.m;
        if work.w.len() != m {
            work.w = vec![0.0; m];
            work.e_full = true;
        }
        if work.marks.len() != m {
            work.marks = EpochMarks::new(m);
        }
        // 策2 の超疎経路を試すか (微小値切捨てがあるときは全走査の判定に任せる)。
        let hyper = work.sparse && work.density < 0.5 * tunable!("ENOMOTO_T_BTRAN_HYPER_FRACTION", BTRAN_HYPER_FRACTION, f64) && tiny_drop() <= 0.0;
        let UnitBtranWork { w, touch, e_touch, e_full, marks, uheap, lheap, y, rows, rows_valid, density, .. } = work;
        // 結果の非ゼロ率を移動平均に畳み込む。
        let mut record = |nnz_frac: f64| {
            let a = tunable!("ENOMOTO_T_DENSITY_AVERAGE_MULTIPLIER", DENSITY_AVERAGE_MULTIPLIER, f64);
            *density = (1.0 - a) * *density + a * nnz_frac;
        };
        *rows_valid = false;
        debug_assert!(w.iter().all(|&v| v == 0.0));
        let s0 = self.base.col_perm_inv[i];
        w[s0] = 1.0;
        touch.clear();
        touch.push(s0);
        // 非ゼロ位置の記録が溢れずに済んだか
        let tracked = if hyper { self.u_transpose_sweep_heap(w, touch, marks, uheap) } else { self.u_transpose_sweep_track(w, touch) };
        // `e_tilde` の記録: 前回の位置を消してから今回の値を書く。
        if *e_full {
            e_tilde_out.fill(0.0);
        } else {
            for &q in e_touch.iter() {
                e_tilde_out[q] = 0.0;
            }
        }
        // `L^T` 段入力の非ゼロ数の上界
        let mut bound;
        if tracked {
            for &q in touch.iter() {
                e_tilde_out[q] = w[q];
            }
            bound = touch.len();
            if hyper {
                // 超疎経路は `touch` を続く段でも使うので写す。
                e_touch.clear();
                e_touch.extend_from_slice(touch);
            } else {
                std::mem::swap(touch, e_touch);
            }
            *e_full = false;
        } else {
            e_tilde_out.copy_from_slice(w);
            bound = usize::MAX;
            *e_full = true;
        }
        // `touch` が `L^T` 段入力の非ゼロ位置をすべて含むか (超疎経路のみ)
        let mut list_ok = hyper && tracked;
        // `btran_tail` と同じ処理を、非ゼロ数の上界を `R` eta 分増やしながら行う。
        for reta in self.r_etas.iter().rev() {
            let yp = w[reta.slot];
            if yp == 0.0 {
                continue;
            }
            let nnz = self.r_etas.nnz(reta.k);
            self.add_tick(nnz as u64);
            bound = bound.saturating_add(nnz);
            self.r_etas.axpy(reta.k, -yp, w);
            if list_ok {
                if self.r_etas.is_dense(reta.k) {
                    list_ok = false;
                } else {
                    // 超疎経路: `R` eta が書いた位置を `touch` に加える (印は `U^T` 段のもの)。
                    for &q in self.r_etas.seg(reta.k).0 {
                        let q = q as usize;
                        if !marks.is_marked(q) {
                            marks.mark(q);
                            touch.push(q);
                        }
                    }
                }
            }
        }
        self.add_tick(m as u64);
        if list_ok {
            if self.l_transpose_hyper_out(w, out, cap.as_deref_mut(), touch, marks, lheap, y, rows) {
                *rows_valid = true;
                record(rows.len() as f64 / m.max(1) as f64);
                return;
            }
            // `L^T` 段で非ゼロが増えすぎた: 全走査で出力済み。
            y.set_full();
            record(tunable!("ENOMOTO_T_BTRAN_HYPER_FRACTION", BTRAN_HYPER_FRACTION, f64));
            return;
        }
        y.set_full();
        // 全走査の結果が疎だったら次回は超疎経路をまた試す (記録が有効 = 非ゼロが上限以下)。
        self.l_transpose_solve_into_ext::<true>(w, out, cap.as_deref_mut(), bound);
        // 全走査の結果の非ゼロ率: 記録が有効なら正確、溢れたら上限 (`limit_frac`) 以上。
        match cap.as_ref() {
            Some(c) if c.valid => record(c.steps.len() as f64 / m.max(1) as f64),
            Some(c) => record(c.limit_frac),
            None => {}
        }
    }

    /// 策2: [`Self::u_transpose_sweep_track`] を非ゼロ位置の最小ヒープで駆動したもの。
    /// `touch` (入口で非ゼロの位置) から始め、`U^T` 順のキー ([`Self::u_transpose_key`]) の
    /// 小さい順に取り出して処理し、新たに書いた位置をヒープと `touch` に加える
    /// (`marks` で重複除去)。スロット `p` に書く eta はすべて `p` よりキーが小さいので、
    /// 取り出した時点で `z[p]` は確定しており、全走査と同じスロットを同じ順に処理する
    /// (値・tick ともビット一致)。`touch` が [`BTRAN_HYPER_FRACTION`] `* m` を超えたら
    /// 次のキーから全走査に切り替えて `false` を返す (`touch` は不完全になる)。
    fn u_transpose_sweep_heap(&self, z: &mut [f64], touch: &mut Vec<usize>, marks: &mut EpochMarks, heap: &mut BinaryHeap<Reverse<u32>>) -> bool {
        let m = self.base.m;
        self.add_tick(m as u64);
        let limit = (tunable!("ENOMOTO_T_BTRAN_HYPER_FRACTION", BTRAN_HYPER_FRACTION, f64) * m as f64) as usize;
        marks.begin();
        heap.clear();
        for &p in touch.iter() {
            marks.mark(p);
            heap.push(Reverse(self.u_transpose_key(p) as u32));
        }
        while let Some(Reverse(k)) = heap.pop() {
            let k = k as usize;
            let (p, pivot) = self.u_transpose_at(k);
            let zp = z[p];
            if zp == 0.0 {
                continue;
            }
            let zp = zp / pivot;
            z[p] = zp;
            let owners = &self.row_owners[p];
            self.add_tick(owners.len() as u64);
            for &(q, v) in owners {
                z[q] -= v * zp;
                if !marks.is_marked(q) {
                    marks.mark(q);
                    touch.push(q);
                    heap.push(Reverse(self.u_transpose_key(q) as u32));
                }
            }
            if touch.len() > limit {
                // 残り (キー `k` より後) は全走査で処理する。
                heap.clear();
                let ns = self.singles.len();
                let total = ns + self.u_seq.n_headers();
                for kk in k + 1..total {
                    if kk >= ns && self.slot_pos[self.u_seq.key[kk - ns] as usize] != kk - ns {
                        // 死んだヘッダ (`EtaFile::kill`)
                        continue;
                    }
                    let (p, pivot) = self.u_transpose_at(kk);
                    let zp = z[p];
                    if zp == 0.0 {
                        continue;
                    }
                    let zp = zp / pivot;
                    z[p] = zp;
                    let owners = &self.row_owners[p];
                    self.add_tick(owners.len() as u64);
                    for &(q, v) in owners {
                        z[q] -= v * zp;
                    }
                }
                return false;
            }
        }
        true
    }

    /// スロット `p` の `U^T` 順 ([`Self::u_transpose_order`]) での位置。
    #[inline(always)]
    fn u_transpose_key(&self, p: usize) -> usize {
        let sp = self.singles_pos[p];
        if sp != usize::MAX {
            sp
        } else {
            self.singles.len() + self.slot_pos[p]
        }
    }

    /// `U^T` 順の位置 `k` の `(スロット, ピボット)`。
    #[inline(always)]
    fn u_transpose_at(&self, k: usize) -> (usize, f64) {
        let ns = self.singles.len();
        if k < ns {
            let e = self.singles[k];
            (e.slot, e.pivot)
        } else {
            (self.u_seq.key[k - ns] as usize, self.u_seq.pivot[k - ns])
        }
    }

    /// 策2: 超疎 BTRAN の `L^{-T}` 段と出力置換。`touch` は `w` の非ゼロ位置をすべて含む
    /// (重複なし、`marks` は使い終わり)。スキャッタ/ギャザーの選択は
    /// [`Self::l_transpose_solve_into_ext`] と同じ。スキャッタなら非ゼロステップの最大ヒープで
    /// 降順に処理し (全走査の部分列なのでビット一致)、出力は `touch` の位置だけを書き、
    /// `w` をそこだけ 0 に戻す。`out` の前回の位置は `y` で消し、非ゼロ行を `rows` (昇順) に、
    /// 非ゼロステップを `cap` (昇順) に書く。成功で `true`。ギャザーを選んだ、または
    /// 非ゼロが増えすぎた場合は全走査で出力して `false` を返す (`y` は呼び出し側が無効化する)。
    #[allow(clippy::too_many_arguments)]
    fn l_transpose_hyper_out(
        &self,
        w: &mut [f64],
        out: &mut [f64],
        cap: Option<&mut StepCapture>,
        touch: &mut Vec<usize>,
        marks: &mut EpochMarks,
        heap: &mut BinaryHeap<u32>,
        y: &mut NzTrack,
        rows: &mut Vec<usize>,
    ) -> bool {
        let m = self.base.m;
        let base = &self.base;
        // スキャッタ形式を使う非ゼロ数の上限 (`l_transpose_solve_into_ext` と同じ判定)
        let limit = (self.btran_l_scatter * m as f64) as usize;
        let nnz = touch.iter().filter(|&&s| w[s] != 0.0).count();
        if nnz > limit {
            PROF_BTRAN_L_GATHER.fetch_add(1, Ordering::Relaxed);
            base.l_transpose_gather_core(w);
            btran_full_out(&base.row_perm, w, out, cap);
            return false;
        }
        PROF_BTRAN_L_SCATTER.fetch_add(1, Ordering::Relaxed);
        let abort = (tunable!("ENOMOTO_T_BTRAN_HYPER_FRACTION", BTRAN_HYPER_FRACTION, f64) * m as f64) as usize;
        marks.begin();
        heap.clear();
        for &s in touch.iter() {
            marks.mark(s);
            heap.push(s as u32);
        }
        while let Some(s) = heap.pop() {
            let s = s as usize;
            let ws = w[s];
            if ws == 0.0 {
                continue;
            }
            for &(k, mult) in base.l_row.row(s) {
                w[k] -= mult * ws;
                if !marks.is_marked(k) {
                    marks.mark(k);
                    touch.push(k);
                    heap.push(k as u32);
                }
            }
            if touch.len() > abort {
                // 残り (ステップ `s` 未満) は全走査のスキャッタで処理する。
                heap.clear();
                for s2 in (0..s).rev() {
                    let ws = w[s2];
                    if ws == 0.0 {
                        continue;
                    }
                    for &(k, mult) in base.l_row.row(s2) {
                        w[k] -= mult * ws;
                    }
                }
                btran_full_out(&base.row_perm, w, out, cap);
                return false;
            }
        }
        // 出力: `touch` の位置だけを書き、`w` を 0 に戻す。
        y.reset(out);
        rows.clear();
        let row_perm = &base.row_perm;
        match cap {
            Some(c) => {
                // 記録できる非ゼロ数の上限 (`permute_btran_out_capture` と同じ)
                let climit = (c.limit_frac * m as f64) as usize;
                c.steps.clear();
                let mut over = false;
                for &s in touch.iter() {
                    let v = w[s];
                    w[s] = 0.0;
                    let r = row_perm[s];
                    out[r] = v;
                    y.idx.push(r);
                    if v != 0.0 {
                        rows.push(r);
                        if !over {
                            if c.steps.len() < climit {
                                c.steps.push(s);
                            } else {
                                over = true;
                            }
                        }
                    }
                }
                c.valid = !over;
                c.steps.sort_unstable();
            }
            None => {
                for &s in touch.iter() {
                    let v = w[s];
                    w[s] = 0.0;
                    let r = row_perm[s];
                    out[r] = v;
                    y.idx.push(r);
                    if v != 0.0 {
                        rows.push(r);
                    }
                }
            }
        }
        rows.sort_unstable();
        true
    }

    /// [`Self::solve_transpose_unit_capture`] に加え、続く融合 `tau` FTRAN のために
    /// `out` の非ゼロステップを `cap` に記録する ([`StepCapture`])。`out` はビット一致。
    pub fn solve_transpose_unit_capture_steps(&self, i: usize, scratch: &mut [f64], out: &mut [f64], e_tilde_out: &mut [f64], cap: Option<&mut StepCapture>) {
        let s0 = self.seed_unit_rhs(i, scratch);
        self.u_transpose_solve_seeded(scratch, s0);
        e_tilde_out.copy_from_slice(scratch);
        self.btran_tail_cap(scratch, out, cap);
    }

    /// [`Self::solve_transpose_into`] と同じだが、`U^-T` 適用後・`R` 逆適用前の中間値を
    /// `e_tilde_out` (長さ `m`) にも書き出す。`rhs` がこの反復の離脱行の単位ベクトル
    /// なら、これがそのまま [`Self::try_update_precomputed`] に渡す `e_tilde` になる。
    ///
    /// 本番の呼び出し箇所はもう無い (単位ベクトル版を使う) が、特殊化版と一致する
    /// ことを確認するテストの参照実装として残している。
    #[allow(dead_code)]
    pub fn solve_transpose_into_capture(&self, rhs: &[f64], scratch: &mut [f64], out: &mut [f64], e_tilde_out: &mut [f64]) {
        self.permute_transpose_rhs(rhs, scratch);
        self.u_transpose_solve_into(scratch);
        e_tilde_out.copy_from_slice(scratch);
        self.btran_tail(scratch, out);
    }

    /// [`Self::solve_transpose_into`] の確保付き簡易版 (テスト用)。
    #[cfg(test)]
    pub fn solve_transpose(&self, rhs: &[f64]) -> Vec<f64> {
        let m = self.base.m;
        let mut scratch = vec![0.0; m];
        let mut out = vec![0.0; m];
        self.solve_transpose_into(rhs, &mut scratch, &mut out);
        out
    }

    /// `rhs = e_i` (元の行番号 `i` の単位ベクトル) 専用の BTRAN。
    /// [`super::DseState::from_basis`] が再分解後に行う `m` 回の単位ベクトル求解用。
    ///
    /// **`self.update_count() == 0` (FT 更新なしの分解直後) が必要** (呼び出し側が
    /// 確認し、ここでは assert のみ)。分解直後の `u_seq` では位置 = スロットで、
    /// 非対角要素は常により小さいステップを参照するため、`e_i` を置換したステップ
    /// `s0` より前のスロットは掃引後も 0 のままと証明できる。そこで `s0` から掃引を
    /// 始める ([`Self::u_transpose_solve_from`])。`L^{-T}` 段は通常どおり。
    ///
    /// **前提/事後条件**: `scratch` は入口で全 0、返却前に全 0 に戻す。
    pub fn solve_transpose_unit_into(&self, i: usize, scratch: &mut [f64], out: &mut [f64]) {
        debug_assert_eq!(self.r_etas.n_headers(), 0, "solve_transpose_unit_into requires a fresh (update-free) factorization");
        let s0 = self.base.col_perm_inv[i];
        scratch[s0] = 1.0;
        // 分解直後の `u_seq` は非シングルトンのスロットを昇順に持つので、`[0, s0)` は
        // `u_seq` の先頭部分に当たる。`s0` 以降のシングルトンは先に
        // `(z - 0) / pivot` を適用する (分割前の掃引と同じ演算)。
        for eta in &self.singles {
            let p = eta.slot;
            if p >= s0 {
                // シングルトンの非対角部は空: 以前と同じ空の `f64` 和 (= 0)。
                let y: f64 = std::iter::empty::<f64>().sum();
                scratch[p] = (scratch[p] - y) / eta.pivot;
            }
        }
        // `u_seq` 内で最初にスロット `>= s0` となる位置
        let start = self.u_seq.key.partition_point(|&k| (k as usize) < s0);
        self.u_transpose_solve_from(scratch, start);
        self.l_transpose_solve_into(scratch, out);
        scratch.fill(0.0);
    }

    /// 基底スロット `basis_slot` (基底配列の添字 = `B` の *列* 番号) の列を
    /// `a_q_original` (入る列。密、長さ `m`、元の行番号。`alpha = B^{-1}a_q` では
    /// なく `a_q` そのもの) で置き換える Forrest-Tomlin 更新を記録する。
    /// 新しいピボットの絶対値が `min_pivot` 未満なら何も記録せず `false` を返す
    /// (再分解トリガ (2): 呼び出し側は新しい基底を最初から分解すること)。
    pub fn try_update(&mut self, basis_slot: usize, a_q_original: &[f64], min_pivot: f64) -> bool {
        let m = self.base.m;
        // 置換される列のステップ
        let p = self.base.col_perm_inv[basis_slot];

        // スクラッチを `self` から取り出す (下の `&self` の FTRAN/BTRAN と借用が
        // 衝突しないように)。`commit_update` の後で `self` に戻す。長さが合わなければ
        // (現状の経路では起きない) 新たに確保する。
        let mut a_tilde = std::mem::take(&mut self.scratch_a_tilde);
        if a_tilde.len() != m {
            a_tilde = vec![0.0; m];
        }
        // `l_solve_into` が最初に全要素を上書きするので事前のゼロ化は不要。
        self.ftran_through_l_and_r_into(a_q_original, &mut a_tilde);

        // `u_transpose_solve_into` は `z` を入力兼出力として使うので、入力 `e_p` に
        // するため全要素を 0 に戻す必要がある。
        let mut e_tilde = std::mem::take(&mut self.scratch_e_tilde);
        if e_tilde.len() != m {
            e_tilde = vec![0.0; m];
        } else {
            e_tilde.iter_mut().for_each(|v| *v = 0.0);
        }
        e_tilde[p] = 1.0;
        self.u_transpose_solve_into(&mut e_tilde);

        let result = self.commit_update(basis_slot, &a_tilde, &e_tilde, min_pivot);
        self.scratch_a_tilde = a_tilde;
        self.scratch_e_tilde = e_tilde;
        result
    }

    /// [`Self::try_update`] と同じ更新を、同じ反復の FTRAN/BTRAN の途中で記録済みの
    /// `a_tilde`/`e_tilde` を受け取って行う (再計算を省く)。
    ///
    /// - `a_tilde`: 入る列の FTRAN の `L`/`R` 適用後の中間値
    ///   ([`Self::solve_into_capture`] / [`Self::solve_sparse_into_capture`] 等で記録)。
    /// - `e_tilde`: 離脱行の単位ベクトル BTRAN の `U^-T` 適用後の中間値
    ///   ([`Self::solve_transpose_unit_capture`] 等で記録)。
    ///
    /// **呼び出し側の責任**: 両者は *この同じ反復* に、この `basis_slot` と入る列に
    /// ついて記録したものであること (古い記録や別スロットのものを渡すと、検出
    /// できないまま更新が壊れる)。
    pub fn try_update_precomputed(&mut self, basis_slot: usize, a_tilde: &[f64], e_tilde: &[f64], min_pivot: f64) -> bool {
        self.commit_update(basis_slot, a_tilde, e_tilde, min_pivot)
    }

    /// [`Self::try_update_precomputed`] の疎入力版 (策3): `a_tilde` を書いた融合 FTRAN の記録
    /// (`ftrack.a_tilde`) と `e_tilde` を書いた BTRAN の記録 (`bwork` の `e_touch`) が有効なら、
    /// eta をその位置だけから作る (長さ `m` の走査を省く)。結果はビット一致。
    pub fn try_update_tracked(&mut self, basis_slot: usize, a_tilde: &[f64], ftrack: &mut FtranTrack, e_tilde: &[f64], bwork: &mut UnitBtranWork, min_pivot: f64) -> bool {
        let a_list = if ftrack.a_tilde.full {
            None
        } else {
            let v = &mut ftrack.a_tilde.idx;
            v.sort_unstable();
            v.dedup();
            Some(v.as_slice())
        };
        let e_list = if bwork.e_full {
            None
        } else {
            let v = &mut bwork.e_touch;
            v.sort_unstable();
            v.dedup();
            Some(v.as_slice())
        };
        self.commit_update_tracked(basis_slot, a_tilde, a_list, e_tilde, e_list, min_pivot)
    }

    /// `u_seq` の位置 `k` の eta を除く。小さな問題では並列配列から削除し、削除で後ろがずれる範囲
    /// だけ `slot_pos` を直す。`lazy_remove` (大きな問題、策4) なら死んだヘッダにするだけ。
    #[inline]
    fn remove_u_eta(&mut self, k: usize) {
        if self.lazy_remove {
            self.u_seq.kill(k);
            self.n_dead += 1;
        } else {
            self.u_seq.remove(k);
            for pos in k..self.u_seq.n_headers() {
                self.slot_pos[self.u_seq.key[pos] as usize] = pos;
            }
        }
    }

    /// [`Self::try_update`] と [`Self::try_update_precomputed`] 共通の本体:
    /// `R` eta を作ってピボットを判定し、合格なら `U` のスロット `p` の eta を
    /// 除いて (行 `p` を他の eta からも消し) 新しい列 eta を末尾に追加する。
    /// `row_owners`/`slot_pos`/`u_seq`/`fill` の整合を 1 か所で保つ。
    fn commit_update(&mut self, basis_slot: usize, a_tilde: &[f64], e_tilde: &[f64], min_pivot: f64) -> bool {
        let m = self.base.m;
        debug_assert_eq!(a_tilde.len(), m, "a_tilde must be the full dense column");
        debug_assert_eq!(e_tilde.len(), m, "e_tilde must be the full dense row");
        let p = self.base.col_perm_inv[basis_slot];

        // スロット `p` の eta が `singles` にあればその位置 (無ければ `usize::MAX`)
        let single_idx = self.singles_pos[p];
        // 置換前の対角ピボット `u_pp`
        let old_pivot = if single_idx != usize::MAX { self.singles[single_idx].pivot } else { self.u_seq.pivot[self.slot_pos[p]] };

        // `R` eta (`r = -u_pp · ẽ_p`、第 `p` 成分除く) を `e_tilde` から直接作って
        // `r_etas` に追加する。ピボット判定に要る `dot` はこの eta と `a_tilde` の
        // 内積なので判定より先に作り、棄却なら pop する。
        let rk = self.r_etas.push_scaled_dense(p, 0.0, e_tilde, p, -old_pivot, tunable!("ENOMOTO_T_DENSE_ETA_FRACTION", DENSE_ETA_FRACTION, f64));
        let dot = self.r_etas.dot(rk, a_tilde);
        // 更新後の対角ピボット `ã_pq - r·ã_q`
        let new_pivot = a_tilde[p] - dot;
        if new_pivot.abs() < min_pivot {
            self.r_etas.pop();
            return false;
        }

        // 旧 eta を除く。`u_seq` 側では削除で後ろがずれる範囲だけ `slot_pos` を直す。
        if single_idx != usize::MAX {
            // シングルトンは `singles` から除く (順序は自由、要素なし)。
            self.singles.swap_remove(single_idx);
            if let Some(moved) = self.singles.get(single_idx) {
                self.singles_pos[moved.slot] = single_idx;
            }
            self.singles_pos[p] = usize::MAX;
            self.single_piv[p] = 0.0;
        } else {
            let k = self.slot_pos[p];
            self.fill -= self.u_seq.nnz(k);
            // 上書き前に、スロット `p` の旧非対角要素を `row_owners` から登録解除する
            // (残すと他の行の所有者リストに無効な `p` が残る)。
            let row_owners = &mut self.row_owners;
            self.u_seq.for_each_entry(k, |row_step, _| {
                if let Some(idx) = row_owners[row_step].iter().position(|&(s, _)| s == p) {
                    row_owners[row_step].swap_remove(idx);
                }
            });
            self.remove_u_eta(k);
        }

        // 行 `p` をまだ参照している eta からそれを消す (Tomlin 1974, eq. 12)。
        // 対象は `row_owners[p]` に載っている eta だけで、`slot_pos` で O(1) に引く。
        for (slot, _) in std::mem::take(&mut self.row_owners[p]) {
            let pos = self.slot_pos[slot];
            if self.u_seq.remove_index(pos, p) {
                self.fill -= 1;
            }
        }

        // 置換後の列 eta を `a_tilde` から直接作って末尾に追加する (scale `1.0`)。
        let k = self.u_seq.push_scaled_dense(p, new_pivot, a_tilde, p, 1.0, tunable!("ENOMOTO_T_DENSE_ETA_FRACTION", DENSE_ETA_FRACTION, f64));
        let row_owners = &mut self.row_owners;
        self.u_seq.for_each_entry(k, |row_step, v| row_owners[row_step].push((p, v)));
        self.fill += self.u_seq.nnz(k);
        self.slot_pos[p] = k;

        self.fill += self.r_etas.nnz(rk);
        self.r_nnz_total += self.r_etas.nnz(rk);

        true
    }

    /// 記録付きの [`Self::commit_update`] (大きな問題の主ループ用、[`Self::try_update_tracked`]):
    /// [`Self::try_update`] と [`Self::try_update_precomputed`] 共通の本体:
    /// `R` eta を作ってピボットを判定し、合格なら `U` のスロット `p` の eta を
    /// 除いて (行 `p` を他の eta からも消し) 新しい列 eta を末尾に追加する。
    /// `row_owners`/`slot_pos`/`u_seq`/`fill` の整合を 1 か所で保つ。
    ///
    /// `a_list`/`e_list`: `a_tilde`/`e_tilde` の非ゼロになりうる位置 (昇順・重複なし)。
    /// あれば eta をそこだけから作る ([`EtaFile::push_scaled_list`])。
    #[inline(never)]
    fn commit_update_tracked(&mut self, basis_slot: usize, a_tilde: &[f64], a_list: Option<&[usize]>, e_tilde: &[f64], e_list: Option<&[usize]>, min_pivot: f64) -> bool {
        let m = self.base.m;
        debug_assert_eq!(a_tilde.len(), m, "a_tilde must be the full dense column");
        debug_assert_eq!(e_tilde.len(), m, "e_tilde must be the full dense row");
        let p = self.base.col_perm_inv[basis_slot];

        // スロット `p` の eta が `singles` にあればその位置 (無ければ `usize::MAX`)
        let single_idx = self.singles_pos[p];
        // 置換前の対角ピボット `u_pp`
        let old_pivot = if single_idx != usize::MAX { self.singles[single_idx].pivot } else { self.u_seq.pivot[self.slot_pos[p]] };

        // `R` eta (`r = -u_pp · ẽ_p`、第 `p` 成分除く) を `e_tilde` から直接作って
        // `r_etas` に追加する。ピボット判定に要る `dot` はこの eta と `a_tilde` の
        // 内積なので判定より先に作り、棄却なら pop する。
        let dense_fraction = tunable!("ENOMOTO_T_DENSE_ETA_FRACTION", DENSE_ETA_FRACTION, f64);
        let rk = match e_list {
            Some(list) => self.r_etas.push_scaled_list(p, 0.0, e_tilde, list, p, -old_pivot, dense_fraction),
            None => self.r_etas.push_scaled_dense(p, 0.0, e_tilde, p, -old_pivot, dense_fraction),
        };
        let dot = self.r_etas.dot(rk, a_tilde);
        // 更新後の対角ピボット `ã_pq - r·ã_q`
        let new_pivot = a_tilde[p] - dot;
        if new_pivot.abs() < min_pivot {
            self.r_etas.pop();
            return false;
        }

        // 旧 eta を除く (`u_seq` 側は `remove_u_eta`)。
        if single_idx != usize::MAX {
            // シングルトンは `singles` から除く (順序は自由、要素なし)。
            self.singles.swap_remove(single_idx);
            if let Some(moved) = self.singles.get(single_idx) {
                self.singles_pos[moved.slot] = single_idx;
            }
            self.singles_pos[p] = usize::MAX;
            self.single_piv[p] = 0.0;
        } else {
            let k = self.slot_pos[p];
            self.fill -= self.u_seq.nnz(k);
            // 上書き前に、スロット `p` の旧非対角要素を `row_owners` から登録解除する
            // (残すと他の行の所有者リストに無効な `p` が残る)。
            let row_owners = &mut self.row_owners;
            self.u_seq.for_each_entry(k, |row_step, _| {
                if let Some(idx) = row_owners[row_step].iter().position(|&(s, _)| s == p) {
                    row_owners[row_step].swap_remove(idx);
                }
            });
            self.remove_u_eta(k);
        }

        // 行 `p` をまだ参照している eta からそれを消す (Tomlin 1974, eq. 12)。
        // 対象は `row_owners[p]` に載っている eta だけで、`slot_pos` で O(1) に引く。
        for (slot, _) in std::mem::take(&mut self.row_owners[p]) {
            let pos = self.slot_pos[slot];
            if self.u_seq.remove_index(pos, p) {
                self.fill -= 1;
            }
        }

        // 置換後の列 eta を `a_tilde` から直接作って末尾に追加する (scale `1.0`)。
        let k = match a_list {
            Some(list) => self.u_seq.push_scaled_list(p, new_pivot, a_tilde, list, p, 1.0, dense_fraction),
            None => self.u_seq.push_scaled_dense(p, new_pivot, a_tilde, p, 1.0, dense_fraction),
        };
        let row_owners = &mut self.row_owners;
        self.u_seq.for_each_entry(k, |row_step, v| row_owners[row_step].push((p, v)));
        self.fill += self.u_seq.nnz(k);
        self.slot_pos[p] = k;

        self.fill += self.r_etas.nnz(rk);
        self.r_nnz_total += self.r_etas.nnz(rk);
        // 採用した `R` eta を列方向索引に載せる (追加策 R)。索引は全 `R` eta がこの関数を通った
        // ときだけ有効 (`r_indexed == r_etas.n_headers()`)。
        if self.r_indexed == rk {
            self.r_indexed += 1;
        }
        if self.r_etas.is_dense(rk) {
            self.r_dense.push(rk as u32);
        } else {
            let (start, len) = self.r_etas.span[rk];
            let (start, len) = (start as usize, len as usize);
            if self.r_next.len() < start + len {
                self.r_next.resize(start + len, u32::MAX);
                self.r_owner.resize(start + len, u32::MAX);
            }
            for e in start..start + len {
                let i = self.r_etas.idx[e] as usize;
                self.r_next[e] = self.r_head[i];
                self.r_head[i] = e as u32;
                self.r_owner[e] = rk as u32;
            }
        }

        true
    }

    /// 最後の分解以降に成功した FT 更新の数 (= `R` eta の数)。
    pub fn update_count(&self) -> usize {
        self.r_etas.n_headers()
    }

    /// `U` の eta 列と `R` eta が保持する非対角 fill の合計 (真の非ゼロ数。密形式でも
    /// 格納長ではなく非ゼロ数)。再分解トリガ (3) の「バンプサイズ」指標。
    /// [`Self::commit_update`] が差分更新するので `O(1)`。
    pub fn fill_count(&self) -> usize {
        debug_assert_eq!(
            self.fill,
            self.u_seq.iter().map(|e| self.u_seq.nnz(e.k)).sum::<usize>() + self.r_etas.iter().map(|e| self.r_etas.nnz(e.k)).sum::<usize>(),
            "incrementally maintained fill drifted from the true eta-file fill"
        );
        self.fill
    }

}

#[cfg(test)]
mod tests {
    thread_local! {
        /// このスレッドで [`super::FtLu::apply_r_sparse`] が呼ばれた回数 (テストが経路を通ったかの確認用)。
        pub(super) static R_SPARSE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    /// 解析専用の再生ベンチマーク (`#[ignore]`):
    /// `ENOMOTO_LU_BENCH_FILE=<lu_dump.bin> cargo test --release lu_kernel_bench -- --ignored --nocapture`。
    /// ダンプされた全入力を `ENOMOTO_LU_BENCH_CONFIGS` (`;` 区切り、各々 `,` 区切りの
    /// `KEY=VAL`) の各設定で分解し、`ENOMOTO_LU_BENCH_REPS` 回交互に回して設定ごとの
    /// 最小合計時間を出し、因子が最初の設定とビット一致することを確認する。
    #[test]
    #[ignore]
    fn lu_kernel_bench() {
        let Some(path) = std::env::var_os("ENOMOTO_LU_BENCH_FILE") else { return };
        let data = std::fs::read(path).expect("read dump");
        let words: Vec<u64> = data.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect();
        let mut it = words.into_iter().peekable();
        let mut rd = || it.next().unwrap();
        let mut mats: Vec<(usize, Vec<Vec<(usize, f64)>>)> = Vec::new();
        let total_words = data.len() / 8;
        let mut consumed = 0usize;
        loop {
            if consumed >= total_words {
                break;
            }
            let m = rd() as usize;
            consumed += 1;
            let mut rows = Vec::with_capacity(m);
            for _ in 0..m {
                let len = rd() as usize;
                consumed += 1 + 2 * len;
                let mut r = Vec::with_capacity(len);
                for _ in 0..len {
                    let j = rd() as usize;
                    let v = f64::from_bits(rd());
                    r.push((j, v));
                }
                rows.push(r);
            }
            mats.push((m, rows));
        }
        let configs: Vec<String> = std::env::var("ENOMOTO_LU_BENCH_CONFIGS")
            .unwrap_or_else(|_| "ENOMOTO_LU_NONE=0".into())
            .split(';')
            .map(|s| s.to_string())
            .collect();
        let reps: usize = std::env::var("ENOMOTO_LU_BENCH_REPS").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
        let apply = |cfg: &str| {
            for kv in cfg.split(',') {
                if let Some((k, v)) = kv.split_once('=') {
                    std::env::set_var(k.trim(), v.trim());
                }
            }
        };
        // 因子 (置換・`U`・求解結果) の FNV-1a 風ハッシュ
        let fingerprint = |lu: &LuFactors| -> u64 {
            let mut h: u64 = 1469598103934665603;
            let mut mix = |x: u64| {
                h ^= x;
                h = h.wrapping_mul(1099511628211);
            };
            for &r in &lu.row_perm {
                mix(r as u64);
            }
            for &c in &lu.col_perm {
                mix(c as u64);
            }
            for row in &lu.u_row {
                for &(c, v) in row {
                    mix(c as u64);
                    mix(v.to_bits());
                }
            }
            let rhs: Vec<f64> = (0..lu.m).map(|i| 1.0 + (i % 7) as f64).collect();
            for v in lu.solve(&rhs) {
                mix(v.to_bits());
            }
            h
        };
        let mut best = vec![f64::INFINITY; configs.len()];
        let mut prints: Vec<Vec<u64>> = vec![Vec::new(); configs.len()];
        let mut lu_nnz = vec![0usize; configs.len()];
        for rep in 0..reps {
            for (ci, cfg) in configs.iter().enumerate() {
                apply(cfg);
                let t = std::time::Instant::now();
                let mut out = Vec::with_capacity(mats.len());
                for (m, rows) in &mats {
                    out.push(factorize_flat_markowitz(*m, rows));
                }
                let el = t.elapsed().as_secs_f64() * 1e3;
                best[ci] = best[ci].min(el);
                if rep == 0 {
                    prints[ci] = out.iter().map(|o| o.as_ref().map(|lu| fingerprint(lu)).unwrap_or(0)).collect();
                    lu_nnz[ci] = out.iter().flatten().map(|lu| lu.u_row.iter().map(|r| r.len()).sum::<usize>() + (0..lu.m).map(|s| lu.l_col.col(s).len()).sum::<usize>()).sum();
                }
            }
        }
        for (ci, cfg) in configs.iter().enumerate() {
            let same = prints[ci] == prints[0];
            let agg = prints[ci].iter().fold(0u64, |h, &x| h.wrapping_mul(31).wrapping_add(x));
            println!("LUBENCH mats={} cfg=[{}] min_ms={:.3} identical_to_first={} factors_fp={:016x} lu_nnz={}", mats.len(), cfg, best[ci], same, agg, lu_nnz[ci]);
            if std::env::var_os("ENOMOTO_LU_BENCH_ALLOW_DIFF").is_none() {
                assert!(same, "factors differ from the first configuration");
            }
        }
    }
    use super::*;

    /// 2 つのベクトルが要素ごとに `1e-8` 以内で一致するか。
    fn approx_vec(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-8)
    }

    /// 3x3 三重対角行列で `factorize` + `solve` / `solve_transpose` が既知解を再現する。
    #[test]
    fn factorize_and_solve_matches_expected() {
        // B = [[2,1,0],[1,3,1],[0,1,4]] (三重対角、疎)。
        let rows = vec![vec![(0, 2.0), (1, 1.0)], vec![(0, 1.0), (1, 3.0), (2, 1.0)], vec![(1, 1.0), (2, 4.0)]];
        let lu = factorize(3, &rows).expect("nonsingular");
        let x_true = [1.0, 2.0, 3.0];
        // rhs = B * x_true
        let rhs = [2.0 * 1.0 + 1.0 * 2.0, 1.0 * 1.0 + 3.0 * 2.0 + 1.0 * 3.0, 1.0 * 2.0 + 4.0 * 3.0];
        let x = lu.solve(&rhs);
        assert!(approx_vec(&x, &x_true), "x={x:?}");

        // B^T y = rhs2
        let y_true = [0.5, -1.0, 2.0];
        let rhs2 = [
            2.0 * y_true[0] + 1.0 * y_true[1],
            1.0 * y_true[0] + 3.0 * y_true[1] + 1.0 * y_true[2],
            1.0 * y_true[1] + 4.0 * y_true[2],
        ];
        let y = lu.solve_transpose(&rhs2);
        assert!(approx_vec(&y, &y_true), "y={y:?}");
    }

    /// 診断用 (`#[ignore]`): 符号付き単位行列で `factorize_diagonal` と
    /// 一般の `factorize` の時間を比べる。
    #[test]
    #[ignore]
    fn factorize_diagonal_vs_markowitz_sweep() {
        for &m in &[50, 200, 500, 1000, 2000, 4000] {
            let rows: Vec<Vec<(usize, f64)>> =
                (0..m).map(|i| vec![(i, if i % 7 == 0 { -1.0 } else { 1.0 })]).collect();

            let n_runs = 2000;
            let t0 = std::time::Instant::now();
            for _ in 0..n_runs {
                std::hint::black_box(factorize_diagonal(m, &rows).expect("diagonal"));
            }
            let diag_ns = t0.elapsed().as_nanos() as f64 / n_runs as f64;

            let t0 = std::time::Instant::now();
            for _ in 0..n_runs {
                std::hint::black_box(factorize(m, &rows).expect("diagonal, still nonsingular"));
            }
            let markowitz_ns = t0.elapsed().as_nanos() as f64 / n_runs as f64;

            println!(
                "m={m:5} factorize_diagonal={diag_ns:8.0}ns factorize(markowitz)={markowitz_ns:8.0}ns speedup={:.1}x",
                markowitz_ns / diag_ns
            );
        }
    }

    /// `factorize_diagonal` が `L = I`・恒等置換を作り、既知解を再現する。
    #[test]
    fn factorize_diagonal_matches_expected_solve() {
        let rows = vec![vec![(0, 1.0)], vec![(1, -1.0)], vec![(2, 1.0)]];
        let lu = factorize_diagonal(3, &rows).expect("diagonal input");
        assert_eq!(lu.l_col.nnz(), 0, "L must be identity: {:?}", lu.l_col.to_cols());
        assert_eq!(lu.row_perm, vec![0, 1, 2]);
        assert_eq!(lu.col_perm, vec![0, 1, 2]);

        let x_true = [3.0, -2.0, 5.0];
        let rhs = [1.0 * x_true[0], -1.0 * x_true[1], 1.0 * x_true[2]];
        let x = lu.solve(&rhs);
        assert!(approx_vec(&x, &x_true), "x={x:?}");
    }

    /// 非対角要素・列ずれ・0 対角のいずれでも `factorize_diagonal` が `None` を返す。
    #[test]
    fn factorize_diagonal_rejects_off_diagonal_entries() {
        let rows = vec![vec![(0, 1.0), (1, 2.0)], vec![(1, 1.0)]];
        assert!(factorize_diagonal(2, &rows).is_none());

        let rows_wrong_col = vec![vec![(1, 1.0)], vec![(0, 1.0)]];
        assert!(factorize_diagonal(2, &rows_wrong_col).is_none());

        let rows_zero = vec![vec![(0, 0.0)], vec![(1, 1.0)]];
        assert!(factorize_diagonal(2, &rows_zero).is_none());
    }

    /// 特異行列で `factorize` が `None` を返す。
    #[test]
    fn factorize_detects_singular() {
        // 3x3 で列 {0,1} しか使わず行 2 = 2 * 行 0 -> 列 2 が空 -> 特異。
        let rows = vec![vec![(0, 1.0), (1, 2.0)], vec![(0, 3.0), (1, 1.0)], vec![(0, 2.0), (1, 4.0)]];
        assert!(factorize(3, &rows).is_none());
    }

    /// 稠密な対角優位行列を `factorize_dense_faer` に直接渡し、既知解
    /// (`solve` と `solve_transpose` の両方) を再現することを確認する。
    #[test]
    fn factorize_dense_faer_matches_hand_verified_solve() {
        let m = 8;
        let entry = |i: usize, j: usize| -> f64 { if i == j { 50.0 } else { 1.0 + ((i * 3 + j * 7) % 11) as f64 * 0.4 } };
        let rows: Vec<Vec<(usize, f64)>> = (0..m).map(|i| (0..m).map(|j| (j, entry(i, j))).collect()).collect();

        let lu = factorize_dense_faer(m, &rows).expect("diagonally dominant must be nonsingular");
        let x_true: Vec<f64> = (0..m).map(|i| 1.0 + i as f64 * 0.5).collect();
        let rhs: Vec<f64> = (0..m).map(|i| (0..m).map(|j| entry(i, j) * x_true[j]).sum()).collect();
        let x = lu.solve(&rhs);
        assert!(approx_vec(&x, &x_true), "x={x:?} x_true={x_true:?}");

        // B^T y = rhs2 (転置求解も同様に確認)。
        let y_true: Vec<f64> = (0..m).map(|i| 0.3 - i as f64 * 0.2).collect();
        let rhs2: Vec<f64> = (0..m).map(|j| (0..m).map(|i| entry(i, j) * y_true[i]).sum()).collect();
        let y = lu.solve_transpose(&rhs2);
        assert!(approx_vec(&y, &y_true), "y={y:?} y_true={y_true:?}");
    }

    /// 完全に稠密な 20x20 入力で `is_dense_input` が発火し、`factorize` が
    /// 稠密経路で正しく解けることを確認する。
    #[test]
    fn factorize_dispatches_dense_input_to_faer() {
        let m = 20;
        let rows: Vec<Vec<(usize, f64)>> =
            (0..m).map(|i| (0..m).map(|j| (j, if i == j { 30.0 } else { 1.0 })).collect()).collect();
        assert!(is_dense_input(m, &rows), "fully dense {m}x{m} input must be flagged dense");
        let lu = factorize(m, &rows).expect("nonsingular");
        let x_true = vec![1.0; m];
        let rhs: Vec<f64> = (0..m).map(|_| 30.0 + (m - 1) as f64).collect();
        let x = lu.solve(&rhs);
        assert!(approx_vec(&x, &x_true), "x={x:?}");
    }

    /// 稠密だが階数落ち (同一行 2 本) の行列で `factorize_dense_faer` が `None` を返す。
    #[test]
    fn factorize_dense_faer_detects_singular() {
        let m = 6;
        let mut rows: Vec<Vec<(usize, f64)>> =
            (0..m).map(|i| (0..m).map(|j| (j, 1.0 + ((i + j) % 4) as f64)).collect()).collect();
        rows[3] = rows[1].clone(); // 行 3 が行 1 と同一 -> 階数落ち
        assert!(factorize_dense_faer(m, &rows).is_none());
    }

    /// `fit1p` 型 (局所行 + 全境界列、境界行が可逆な `k x k` コア) の 10x10 行列で、
    /// [`factorize_bordered`] と `factorize_flat_markowitz` の両方が既知解を再現する。
    #[test]
    fn factorize_bordered_matches_flat_markowitz() {
        let m = 10;
        let border = [7usize, 8, 9];
        let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
        for i in 0..7 {
            rows.push(vec![(i, 3.0 + i as f64), (7, 1.0), (8, 1.0 + 0.1 * i as f64), (9, 2.0 - 0.1 * i as f64)]);
        }
        rows.push(vec![(7, 4.0), (8, 1.0), (9, 0.0)]);
        rows.push(vec![(7, 1.0), (8, 3.0), (9, 1.0)]);
        rows.push(vec![(7, 0.0), (8, 1.0), (9, 5.0)]);
        assert_eq!(rows.len(), m);

        let lu_bordered = factorize_bordered(m, &rows, &border).expect("bordered factorization should succeed");
        let lu_flat = factorize_flat_markowitz(m, &rows).expect("plain Markowitz should also succeed");

        let x_true: Vec<f64> = (0..m).map(|i| 1.0 + i as f64 * 0.3).collect();
        let entry = |row: &[(usize, f64)], j: usize| row.iter().find(|&&(c, _)| c == j).map(|&(_, v)| v).unwrap_or(0.0);
        let rhs: Vec<f64> = rows.iter().map(|row| (0..m).map(|j| entry(row, j) * x_true[j]).sum()).collect();

        let x_bordered = lu_bordered.solve(&rhs);
        let x_flat = lu_flat.solve(&rhs);
        assert!(approx_vec(&x_bordered, &x_true), "bordered x={x_bordered:?}");
        assert!(approx_vec(&x_flat, &x_true), "flat x={x_flat:?}");
    }

    /// 同じ形の大きめの版 (30 行、境界列 6 本)。`n_sparse`/境界ステップの
    /// オフセット計算の off-by-one などを検出する。
    #[test]
    fn factorize_bordered_matches_flat_markowitz_larger() {
        let m = 30;
        let k = 6;
        let border: Vec<usize> = (m - k..m).collect();
        let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
        for i in 0..(m - k) {
            let mut row = vec![(i, 5.0 + i as f64)];
            for (bi, &b) in border.iter().enumerate() {
                row.push((b, 1.0 + 0.1 * ((i + bi) % 5) as f64));
            }
            rows.push(row);
        }
        // 対角優位な k x k 境界コア -> 正則。
        for bi in 0..k {
            let mut row = Vec::new();
            for (bj, &b2) in border.iter().enumerate() {
                let v = if bi == bj { 10.0 } else { 1.0 + ((bi + bj) % 3) as f64 * 0.2 };
                row.push((b2, v));
            }
            rows.push(row);
        }
        assert_eq!(rows.len(), m);

        let lu_bordered = factorize_bordered(m, &rows, &border).expect("bordered factorization should succeed");
        let lu_flat = factorize_flat_markowitz(m, &rows).expect("plain Markowitz should also succeed");

        let x_true: Vec<f64> = (0..m).map(|i| 1.0 + i as f64 * 0.13).collect();
        let entry = |row: &[(usize, f64)], j: usize| row.iter().find(|&&(c, _)| c == j).map(|&(_, v)| v).unwrap_or(0.0);
        let rhs: Vec<f64> = rows.iter().map(|row| (0..m).map(|j| entry(row, j) * x_true[j]).sum()).collect();

        let x_bordered = lu_bordered.solve(&rhs);
        let x_flat = lu_flat.solve(&rhs);
        assert!(approx_vec(&x_bordered, &x_true), "bordered x={x_bordered:?}");
        assert!(approx_vec(&x_flat, &x_true), "flat x={x_flat:?}");
    }

    /// 疎フェーズが終わる前に境界列がピボットとして必要になる (疎部分が比例する)
    /// 場合、`factorize_bordered` は `None` を返し、行列全体も特異として報告される
    /// (`rank(A) <= rank(A_sparse) + k`)。
    #[test]
    fn factorize_bordered_falls_back_to_none_on_stuck_sparse_phase() {
        let m = 3;
        let border = [2usize];
        let rows = vec![
            vec![(0, 1.0), (1, 2.0)],
            vec![(0, 2.0), (1, 4.0), (2, 1.0)], // 疎部分が行 0 に比例
            vec![(2, 5.0)],
        ];
        assert!(factorize_bordered(m, &rows, &border).is_none());
        assert!(factorize_flat_markowitz(m, &rows).is_none(), "matrix is genuinely singular");
        assert!(factorize(m, &rows).is_none());
    }

    /// 次数 `m`・境界列 `k` 本の `fit1p` 型矢じり行列 (境界列は全行で非ゼロ、
    /// 境界コアは対角優位) と境界列一覧を作る (クロスオーバー掃引用)。
    fn arrowhead(m: usize, k: usize) -> (Vec<Vec<(usize, f64)>>, Vec<usize>) {
        let border: Vec<usize> = (m - k..m).collect();
        let mut rows: Vec<Vec<(usize, f64)>> = Vec::with_capacity(m);
        for i in 0..(m - k) {
            let mut row = vec![(i, 5.0 + i as f64)];
            for (bi, &b) in border.iter().enumerate() {
                row.push((b, 1.0 + 0.1 * ((i + bi) % 5) as f64));
            }
            rows.push(row);
        }
        for bi in 0..k {
            let mut row = Vec::with_capacity(k);
            for (bj, &b2) in border.iter().enumerate() {
                let v = if bi == bj { 10.0 * k as f64 } else { 1.0 + ((bi + bj) % 3) as f64 * 0.2 };
                row.push((b2, v));
            }
            rows.push(row);
        }
        (rows, border)
    }

    /// **診断用** (`#[ignore]`、`cargo test --release -- --ignored --nocapture border_crossover`):
    /// `m = 800` で境界率 `k/m` を掃引し、`factorize_bordered` と
    /// `factorize_flat_markowitz` の時間を比較する ([`BORDER_MAX_FRACTION`] の調整用)。
    #[test]
    #[ignore]
    fn border_crossover_sweep() {
        let m = 800;
        for &frac in &[0.02, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5] {
            let k = ((m as f64) * frac).round() as usize;
            if k == 0 || k >= m {
                continue;
            }
            let (rows, border) = arrowhead(m, k);
            let nnz: usize = rows.iter().map(|r| r.len()).sum();
            let dense = is_dense_input(m, &rows);

            let n_runs = 20;
            let t0 = std::time::Instant::now();
            for _ in 0..n_runs {
                std::hint::black_box(factorize_bordered(m, &rows, &border).expect("nonsingular"));
            }
            let bordered_us = t0.elapsed().as_micros() as f64 / n_runs as f64;

            let t0 = std::time::Instant::now();
            for _ in 0..n_runs {
                std::hint::black_box(factorize_flat_markowitz(m, &rows).expect("nonsingular"));
            }
            let flat_us = t0.elapsed().as_micros() as f64 / n_runs as f64;

            println!(
                "m={m} k={k} k/m={frac:.2} nnz_frac={:.3} is_dense_input={dense} bordered={bordered_us:.1}us flat={flat_us:.1}us speedup={:.2}x",
                nnz as f64 / (m * m) as f64,
                flat_us / bordered_us
            );
        }
    }

    /// 境界列を部分的にだけ埋めた (`border_density`) 矢じり行列を作る。全体密度を
    /// `is_dense_input` のゲートよりずっと低く保ち、`k/m` 自体のクロスオーバーを
    /// 密度の影響と切り離して測るため。
    fn arrowhead_partial(m: usize, k: usize, border_density: f64) -> (Vec<Vec<(usize, f64)>>, Vec<usize>) {
        let border: Vec<usize> = (m - k..m).collect();
        let mut rows: Vec<Vec<(usize, f64)>> = Vec::with_capacity(m);
        // 境界列を何行に 1 回埋めるか
        let step = (1.0 / border_density).round().max(1.0) as usize;
        for i in 0..(m - k) {
            let mut row = vec![(i, 5.0 + i as f64)];
            for (bi, &b) in border.iter().enumerate() {
                if (i + bi) % step == 0 {
                    row.push((b, 1.0 + 0.1 * ((i + bi) % 5) as f64));
                }
            }
            rows.push(row);
        }
        for bi in 0..k {
            let mut row = Vec::with_capacity(k);
            for (bj, &b2) in border.iter().enumerate() {
                let v = if bi == bj { 10.0 * k as f64 } else { 1.0 + ((bi + bj) % 3) as f64 * 0.2 };
                row.push((b2, v));
            }
            rows.push(row);
        }
        (rows, border)
    }

    /// 部分密度掃引の 1 点 `(m, k/m)` を `n_runs` 回ずつ計測し、境界付きと
    /// `factorize_flat_markowitz` (密度が閾値を超えれば稠密 LU に振り分けられる) の
    /// 時間を出力する。
    fn run_border_crossover_point(m: usize, frac: f64, n_runs: usize) {
        let k = ((m as f64) * frac).round() as usize;
        if k == 0 || k >= m {
            return;
        }
        let (rows, border) = arrowhead_partial(m, k, 0.3);
        let nnz: usize = rows.iter().map(|r| r.len()).sum();
        let dense = is_dense_input(m, &rows);

        let t0 = std::time::Instant::now();
        for _ in 0..n_runs {
            std::hint::black_box(factorize_bordered(m, &rows, &border).expect("nonsingular"));
        }
        let bordered_us = t0.elapsed().as_micros() as f64 / n_runs as f64;

        let t0 = std::time::Instant::now();
        for _ in 0..n_runs {
            std::hint::black_box(factorize_flat_markowitz(m, &rows).expect("nonsingular"));
        }
        let flat_us = t0.elapsed().as_micros() as f64 / n_runs as f64;

        println!(
            "m={m} k={k} k/m={frac:.2} nnz_frac={:.3} is_dense_input={dense} bordered={bordered_us:.1}us flat={flat_us:.1}us speedup={:.2}x",
            nnz as f64 / (m * m) as f64,
            flat_us / bordered_us
        );
    }

    /// 診断用 (`#[ignore]`): `m = 800` の部分密度掃引。
    #[test]
    #[ignore]
    fn border_crossover_sweep_partial_density() {
        for &frac in &[0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.6, 0.7] {
            run_border_crossover_point(800, frac, 20);
        }
    }

    /// 診断用 (`#[ignore]`): `m = 800` のクロスオーバー付近 (`k/m` 0.50-0.60) の細かい掃引。
    #[test]
    #[ignore]
    fn border_crossover_sweep_fine() {
        for &frac in &[0.50, 0.52, 0.54, 0.56, 0.58, 0.60] {
            run_border_crossover_point(800, frac, 20);
        }
    }

    /// 診断用 (`#[ignore]`): `m = 2000` で同じ `k/m` を測り、クロスオーバーが
    /// 割合 (`k/m`) の効果か絶対数 `k` の効果かを見分ける。
    #[test]
    #[ignore]
    fn border_crossover_sweep_scaling() {
        for &frac in &[0.2, 0.3, 0.4, 0.5, 0.6] {
            run_border_crossover_point(2000, frac, 5);
        }
    }

    /// 3x3 単位行列の列 1 を置換する FT 更新 1 回が、新しい基底の完全再分解と
    /// FTRAN/BTRAN で一致する。
    #[test]
    fn ft_update_matches_full_refactor() {
        // B0 = I (3x3)。列 1 を [1,5,2] で置換する (basis_slot=1)。
        let rows0: Vec<Vec<(usize, f64)>> = (0..3).map(|i| vec![(i, 1.0)]).collect();
        let base = factorize(3, &rows0).unwrap();
        let mut state = FtLu::new(base);
        let a_q = [1.0, 5.0, 2.0];
        assert!(state.try_update(1, &a_q, 1e-9));

        // 比較用の新基底の完全再分解 (I の列 1 を [1,5,2] に置換。行疎なので
        // 行 0 に (列1,1.0)、行 1 に (列1,5.0)、行 2 に (列1,2.0) と元の (列2,1.0))。
        let rows1 = vec![vec![(0, 1.0), (1, 1.0)], vec![(1, 5.0)], vec![(1, 2.0), (2, 1.0)]];
        let full = factorize(3, &rows1).unwrap();

        let rhs = [3.0, -2.0, 7.0];
        let x_ft = state.solve(&rhs);
        let x_full = full.solve(&rhs);
        assert!(approx_vec(&x_ft, &x_full), "ft={x_ft:?} full={x_full:?}");

        let y_ft = state.solve_transpose(&rhs);
        let y_full = full.solve_transpose(&rhs);
        assert!(approx_vec(&y_ft, &y_full), "ft={y_ft:?} full={y_full:?}");
    }

    /// 非自明な基底に 2 回連続で FT 更新した結果が完全再分解と一致する。
    #[test]
    fn ft_update_chain_of_two_matches_full_refactor() {
        // B0 を単位行列でなくし、2 回目の更新の部分 BTRAN が更新済みの U を通るようにする。
        let rows0 =
            vec![vec![(0, 2.0), (1, 1.0)], vec![(0, 1.0), (1, 3.0), (2, 1.0)], vec![(1, 1.0), (2, 4.0)]];
        let base = factorize(3, &rows0).unwrap();
        let mut state = FtLu::new(base);

        let a_q1 = [1.0, 5.0, 2.0];
        assert!(state.try_update(1, &a_q1, 1e-9));

        let a_q2 = [4.0, 1.0, 3.0];
        assert!(state.try_update(0, &a_q2, 1e-9));

        // 新しい基底の列: col0 = a_q2, col1 = a_q1, col2 は B0 のまま。
        let rows_full = vec![
            vec![(0, 4.0), (1, 1.0)],
            vec![(0, 1.0), (1, 5.0), (2, 1.0)],
            vec![(0, 3.0), (1, 2.0), (2, 4.0)],
        ];
        let full = factorize(3, &rows_full).unwrap();

        let rhs = [2.0, -3.0, 1.0];
        let x_ft = state.solve(&rhs);
        let x_full = full.solve(&rhs);
        assert!(approx_vec(&x_ft, &x_full), "ft={x_ft:?} full={x_full:?}");

        let y_ft = state.solve_transpose(&rhs);
        let y_full = full.solve_transpose(&rhs);
        assert!(approx_vec(&y_ft, &y_full), "ft={y_ft:?} full={y_full:?}");
    }

    /// 4x4 で 3 回連続の FT 更新 (新しい `a_tilde` の計算で既存 `R` eta の
    /// ループを通る) が完全再分解と一致する。
    #[test]
    fn ft_update_chain_of_three_matches_full_refactor() {
        let rows0 = vec![
            vec![(0, 4.0), (1, 1.0)],
            vec![(0, 1.0), (1, 3.0), (2, 1.0)],
            vec![(1, 1.0), (2, 5.0), (3, 2.0)],
            vec![(2, 1.0), (3, 6.0)],
        ];
        let base = factorize(4, &rows0).unwrap();
        let mut state = FtLu::new(base);

        let a_q1 = [2.0, 7.0, 1.0, 3.0];
        assert!(state.try_update(2, &a_q1, 1e-9));
        let a_q2 = [5.0, 1.0, 4.0, 2.0];
        assert!(state.try_update(0, &a_q2, 1e-9));
        let a_q3 = [1.0, 6.0, 2.0, 3.0];
        assert!(state.try_update(3, &a_q3, 1e-9));

        // 最終基底: col0=a_q2=[5,1,4,2], col1 は元のまま=[1,3,1,0],
        // col2=a_q1=[2,7,1,3], col3=a_q3=[1,6,2,3]。
        let rows_full = vec![
            vec![(0, 5.0), (1, 1.0), (2, 2.0), (3, 1.0)],
            vec![(0, 1.0), (1, 3.0), (2, 7.0), (3, 6.0)],
            vec![(0, 4.0), (1, 1.0), (2, 1.0), (3, 2.0)],
            vec![(0, 2.0), (2, 3.0), (3, 3.0)],
        ];
        let full = factorize(4, &rows_full).unwrap();

        let rhs = [1.0, 2.0, -1.0, 3.0];
        let x_ft = state.solve(&rhs);
        let x_full = full.solve(&rhs);
        assert!(approx_vec(&x_ft, &x_full), "ft={x_ft:?} full={x_full:?}");

        let y_ft = state.solve_transpose(&rhs);
        let y_full = full.solve_transpose(&rhs);
        assert!(approx_vec(&y_ft, &y_full), "ft={y_ft:?} full={y_full:?}");
    }

    /// 新しいピボットが 0 になる更新は棄却され、何も記録されない。
    #[test]
    fn ft_update_rejects_tiny_pivot() {
        let rows0: Vec<Vec<(usize, f64)>> = (0..2).map(|i| vec![(i, 1.0)]).collect();
        let base = factorize(2, &rows0).unwrap();
        let mut state = FtLu::new(base);
        // 列 0 を、(r 補正後の) L^-1 変換値がスロット 0 で 0 になるもので置換 -> ピボット 0。
        let a_q = [0.0, 1.0];
        assert!(!state.try_update(0, &a_q, 1e-9));
        assert_eq!(state.update_count(), 0);
    }

    /// `try_update` (自前で `a_tilde`/`e_tilde` を再計算) と、FTRAN/BTRAN で記録した
    /// 中間値を渡す `try_update_precomputed` が、密・疎どちらの記録経路でも
    /// ビット一致の結果になる (既に 2 回更新済みの状態で確認)。
    #[test]
    fn try_update_precomputed_matches_try_update() {
        let rows0 = vec![vec![(0, 2.0), (1, 1.0)], vec![(0, 1.0), (1, 3.0), (2, 1.0)], vec![(1, 1.0), (2, 4.0)]];
        let base = factorize(3, &rows0).unwrap();
        let mut state = FtLu::new(base);
        assert!(state.try_update(1, &[1.0, 5.0, 2.0], 1e-9));
        assert!(state.try_update(0, &[4.0, 1.0, 3.0], 1e-9));

        let m = 3;
        let basis_slot = 2;
        let a_q = [2.0, 1.0, 6.0];

        // 参照: 素の `try_update` (`a_tilde`/`e_tilde` を自分で再計算)。
        let mut state_ref = state.clone();
        assert!(state_ref.try_update(basis_slot, &a_q, 1e-9));

        // 密記録経路: `solve_into_capture` が `a_tilde`、
        // `solve_transpose_into_capture` が `e_tilde` を供給する。
        let mut state_dense = state.clone();
        let (mut scratch, mut out, mut a_tilde) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
        state_dense.solve_into_capture(&a_q, &mut scratch, &mut out, &mut a_tilde);
        let mut e_p = vec![0.0; m];
        e_p[basis_slot] = 1.0;
        let (mut scratch2, mut out2, mut e_tilde) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
        state_dense.solve_transpose_into_capture(&e_p, &mut scratch2, &mut out2, &mut e_tilde);
        assert!(state_dense.try_update_precomputed(basis_slot, &a_tilde, &e_tilde, 1e-9));

        // 疎記録経路: `solve_sparse_into_capture` が `a_tilde` を供給する
        // (Gilbert-Peierls 経由でも全く同じ中間値になること)。
        let mut state_sparse = state.clone();
        let (mut sscratch, mut sout, mut sa_tilde) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
        let mut gp = GpScratch::new(m);
        state_sparse.solve_sparse_into_capture(&to_sparse(&a_q), &mut sscratch, &mut gp, &mut sout, &mut sa_tilde);
        let (mut sscratch2, mut sout2, mut se_tilde) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
        state_sparse.solve_transpose_into_capture(&e_p, &mut sscratch2, &mut sout2, &mut se_tilde);
        assert!(state_sparse.try_update_precomputed(basis_slot, &sa_tilde, &se_tilde, 1e-9));

        for rhs in [[3.0, -2.0, 7.0], [1.0, 0.0, -1.0], [0.5, 0.5, 0.5]] {
            let x_ref = state_ref.solve(&rhs);
            let x_dense = state_dense.solve(&rhs);
            let x_sparse = state_sparse.solve(&rhs);
            assert_eq!(x_ref, x_dense, "dense-capture diverged from try_update on solve: rhs={rhs:?}");
            assert_eq!(x_ref, x_sparse, "sparse-capture diverged from try_update on solve: rhs={rhs:?}");

            let y_ref = state_ref.solve_transpose(&rhs);
            let y_dense = state_dense.solve_transpose(&rhs);
            let y_sparse = state_sparse.solve_transpose(&rhs);
            assert_eq!(y_ref, y_dense, "dense-capture diverged from try_update on solve_transpose: rhs={rhs:?}");
            assert_eq!(y_ref, y_sparse, "sparse-capture diverged from try_update on solve_transpose: rhs={rhs:?}");
        }
    }

    /// 決定的な線形合同法の擬似乱数 (`[-1, 1)` 程度)。外部 `rand` 依存なしで使う。
    fn next_rand(state: &mut u64) -> f64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((*state >> 33) as f64 / (1u64 << 31) as f64) - 1.0
    }

    /// 非対角要素を持つ対角優位な疎行列 (`m x m`、各行に非対角最大 3 個) を作る。
    /// Markowitz が非恒等な置換と実際の fill を生む程度の大きさで使う。
    fn random_sparse_diag_dominant(m: usize, seed: u64) -> Vec<Vec<(usize, f64)>> {
        let mut state = seed;
        let mut rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
        for i in 0..m {
            let mut off_sum = 0.0f64;
            let n_off = 3.min(m - 1);
            let mut cols: Vec<usize> = Vec::with_capacity(n_off);
            for _ in 0..n_off {
                let j = ((next_rand(&mut state).abs() * m as f64) as usize).min(m - 1);
                if j != i && !cols.contains(&j) {
                    cols.push(j);
                }
            }
            for &j in &cols {
                let v = next_rand(&mut state) * 2.0;
                off_sum += v.abs();
                rows[i].push((j, v));
            }
            rows[i].push((i, off_sum + 5.0 + next_rand(&mut state).abs()));
            rows[i].sort_unstable_by_key(|&(c, _)| c);
        }
        rows
    }

    /// `l_row` が `l_col` のちょうど転置である (全要素が 1 回ずつ、同じ値で対応する)。
    #[test]
    fn l_row_is_the_exact_transpose_of_l_col() {
        for m in [3usize, 12, 40] {
            for seed in [1u64, 7, 99] {
                let rows = random_sparse_diag_dominant(m, seed);
                let lu = factorize(m, &rows).expect("nonsingular");
                let mut from_col: Vec<(usize, usize, u64)> = Vec::new();
                for s in 0..m {
                    for &(row_step, mult) in lu.l_col.col(s) {
                        assert!(row_step > s, "L must be strictly lower triangular in step space");
                        from_col.push((s, row_step, mult.to_bits()));
                    }
                }
                let mut from_row: Vec<(usize, usize, u64)> = Vec::new();
                for r in 0..m {
                    for &(s, mult) in lu.l_row.row(r) {
                        assert!(s < r, "l_row.row(r) must only hold entries at s < r");
                        from_row.push((s, r, mult.to_bits()));
                    }
                }
                from_col.sort_unstable();
                from_row.sort_unstable();
                assert_eq!(from_col, from_row, "l_row is not l_col's transpose (m={m}, seed={seed})");
            }
        }
    }

    /// `L^{-T}` のスキャッタ形式とギャザー形式が、単一非ゼロから密までの右辺で
    /// 相対 `1e-12` 以内で一致する (加算順が逆なのでビット一致ではない)。
    #[test]
    fn l_transpose_scatter_matches_gather() {
        for m in [3usize, 12, 40] {
            for seed in [1u64, 7, 99] {
                let rows = random_sparse_diag_dominant(m, seed);
                let lu = factorize(m, &rows).expect("nonsingular");
                let mut rng = seed ^ 0xabcd;
                let mut rhss: Vec<Vec<f64>> = vec![(0..m).map(|_| next_rand(&mut rng)).collect()];
                for unit in [0usize, m / 2, m - 1] {
                    let mut e = vec![0.0; m];
                    e[unit] = 1.0;
                    rhss.push(e);
                }
                for rhs in rhss {
                    let (mut wg, mut yg) = (rhs.clone(), vec![0.0; m]);
                    lu.l_transpose_solve_gather_into(&mut wg, &mut yg);
                    let (mut ws, mut ys) = (rhs.clone(), vec![0.0; m]);
                    lu.l_transpose_solve_scatter_into(&mut ws, &mut ys);
                    for i in 0..m {
                        let scale = yg[i].abs().max(1.0);
                        assert!(
                            (yg[i] - ys[i]).abs() <= 1e-12 * scale,
                            "scatter/gather mismatch at {i}: {} vs {} (m={m}, seed={seed})",
                            yg[i],
                            ys[i]
                        );
                    }
                }
            }
        }
    }

    /// FT 更新で `R` eta が入った後でも、BTRAN 全体 (`B^-T rhs`) がスキャッタ/
    /// ギャザー両経路で一致する。
    #[test]
    fn btran_agrees_between_scatter_and_gather_after_ft_updates() {
        let m = 40;
        for seed in [3u64, 11] {
            let rows = random_sparse_diag_dominant(m, seed);
            let mut scatter = FtLu::new(factorize(m, &rows).expect("nonsingular"));
            scatter.btran_l_scatter = 1.0;
            let mut gather = FtLu::new(factorize(m, &rows).expect("nonsingular"));
            gather.btran_l_scatter = 0.0;
            let mut rng = seed ^ 0x5eed;
            for slot in [2usize, 9, 25] {
                let a_q: Vec<f64> = (0..m).map(|i| if i % 3 == 0 { next_rand(&mut rng) } else { 0.0 } + if i == slot { 4.0 } else { 0.0 }).collect();
                assert!(scatter.try_update(slot, &a_q, 1e-9));
                assert!(gather.try_update(slot, &a_q, 1e-9));
            }
            for probe in [0usize, 7, 39] {
                let mut rhs = vec![0.0; m];
                rhs[probe] = 1.0;
                let ys = scatter.solve_transpose(&rhs);
                let yg = gather.solve_transpose(&rhs);
                for i in 0..m {
                    let scale = yg[i].abs().max(1.0);
                    assert!(
                        (yg[i] - ys[i]).abs() <= 1e-9 * scale,
                        "BTRAN mismatch at {i}: {} vs {} (seed={seed}, probe={probe})",
                        yg[i],
                        ys[i]
                    );
                }
            }
        }
    }

    /// 疎行 `A` に対する残差 `‖A x - b‖_inf`。
    fn residual_inf(rows: &[Vec<(usize, f64)>], x: &[f64], b: &[f64]) -> f64 {
        rows.iter()
            .enumerate()
            .map(|(i, row)| (row.iter().map(|&(j, v)| v * x[j]).sum::<f64>() - b[i]).abs())
            .fold(0.0, f64::max)
    }

    /// `rows` の列を `n` 本、置き換える列とは *別の行* に非ゼロを持つ新しい内容で
    /// 置換する (FT 更新が基底に対して行うことの模擬。記録した行順を再生できなく
    /// なるケース)。
    fn replace_columns(rows: &mut [Vec<(usize, f64)>], n: usize, seed: u64) {
        let m = rows.len();
        let mut state = seed;
        for k in 0..n {
            let j = (k * 7 + 3) % m;
            for row in rows.iter_mut() {
                row.retain(|&(c, _)| c != j);
            }
            let i0 = (k * 11 + 5) % m;
            rows[i0].push((j, 6.0 + next_rand(&mut state).abs()));
            let i1 = (k * 13 + 1) % m;
            if i1 != i0 {
                rows[i1].push((j, next_rand(&mut state)));
            }
        }
    }

    /// 列を置換した基底でも、記録した列順を再利用した分解が `Bx = b` と
    /// `B^T y = b` を解き、`l_row` が `l_col` の転置のままである。
    #[test]
    fn reused_order_solves_a_basis_whose_columns_were_replaced() {
        for seed in [1u64, 7, 99] {
            let m = 60;
            let rows = random_sparse_diag_dominant(m, seed);
            let first = factorize(m, &rows).expect("well-conditioned matrix factorizes");

            let mut rows2 = rows.clone();
            replace_columns(&mut rows2, 5, seed);

            let reused = factorize_reusing_order(m, &rows2, &first.col_perm, &first.row_perm, usize::MAX)
                .expect("row re-picking keeps the recorded column order usable");

            let b: Vec<f64> = (0..m).map(|i| 1.0 + (i % 5) as f64).collect();
            let x = reused.solve(&b);
            assert!(residual_inf(&rows2, &x, &b) < 1e-9, "seed {seed}: reused order solves the updated matrix");

            // 転置求解も確認 (転置経路は `l_col`/`u_row` を逆向きに読むので、
            // 置換の誤りは片方だけで現れうる)。
            let y = reused.solve_transpose(&b);
            let mut rows_t: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
            for (i, row) in rows2.iter().enumerate() {
                for &(j, v) in row {
                    rows_t[j].push((i, v));
                }
            }
            assert!(residual_inf(&rows_t, &y, &b) < 1e-9, "seed {seed}: reused order solves the transpose");

            // この経路でも `l_row` は `l_col` の転置であること (BTRAN のスキャッタ形式が読む)。
            let mut from_col: Vec<(usize, usize, u64)> = Vec::new();
            for s in 0..m {
                for &(row_step, mult) in reused.l_col.col(s) {
                    assert!(row_step > s, "L must be strictly lower triangular in step space");
                    from_col.push((s, row_step, mult.to_bits()));
                }
            }
            let mut from_row: Vec<(usize, usize, u64)> = Vec::new();
            for r in 0..m {
                for &(s, mult) in reused.l_row.row(r) {
                    from_row.push((s, r, mult.to_bits()));
                }
            }
            from_col.sort_unstable();
            from_row.sort_unstable();
            assert_eq!(from_col, from_row, "seed {seed}: l_row is not l_col's transpose");
        }
    }

    /// 同じ行列を自分自身の順序で再利用分解すると、因子そのものが一致する。
    #[test]
    fn reused_order_reproduces_a_fresh_factorization_of_the_same_matrix() {
        let m = 50;
        let rows = random_sparse_diag_dominant(m, 2024);
        let first = factorize(m, &rows).expect("factorizes");
        let again = factorize_reusing_order(m, &rows, &first.col_perm, &first.row_perm, usize::MAX)
            .expect("its own order is trivially acceptable for the same matrix");

        // 同じ行列・同じ順序なので、解が同じだけでなく因子自体が一致すること。
        assert_eq!(again.row_perm, first.row_perm);
        assert_eq!(again.col_perm, first.col_perm);
        for s in 0..m {
            let mut a: Vec<(usize, f64)> = first.l_col.col(s).to_vec();
            let mut b: Vec<(usize, f64)> = again.l_col.col(s).iter().copied().filter(|&(_, v)| v != 0.0).collect();
            a.sort_unstable_by_key(|&(r, _)| r);
            b.sort_unstable_by_key(|&(r, _)| r);
            assert_eq!(a.len(), b.len(), "L column {s} nonzero count");
            for (&(ra, va), &(rb, vb)) in a.iter().zip(b.iter()) {
                assert_eq!(ra, rb);
                assert!((va - vb).abs() < 1e-12, "L[{ra}][{s}]: {va} vs {vb}");
            }
        }
    }

    /// 特異行列では再利用分解が `None` を返す。
    #[test]
    fn reused_order_rejects_a_singular_matrix() {
        let identity_order: Vec<usize> = vec![0, 1];
        let singular = vec![vec![(0usize, 1.0f64), (1usize, 1.0f64)], vec![(0usize, 1.0f64), (1usize, 1.0f64)]];
        assert!(
            factorize_reusing_order(2, &singular, &identity_order, &identity_order, usize::MAX).is_none(),
            "a column with nothing left in the remaining submatrix must reject the order"
        );
    }

    /// 記録された行が数値的に弱いとき、順序全体を棄却せずに行だけ選び直す。
    #[test]
    fn reused_order_repicks_the_row_when_the_recorded_one_is_numerically_weak() {
        // 記録ではステップ 0 は列 0 の行 0 でピボットするが、この行列ではその要素が
        // 同じ列の他の要素に比べて無視できるほど小さい: 行 1 に選び直されるべき。
        let order: Vec<usize> = vec![0, 1];
        let weak = vec![vec![(0usize, 1e-14f64), (1usize, 1.0f64)], vec![(0usize, 1.0f64), (1usize, 1.0f64)]];
        let lu = factorize_reusing_order(2, &weak, &order, &order, usize::MAX)
            .expect("a weak recorded row is re-picked, not a rejection");
        assert_eq!(lu.row_perm[0], 1, "step 0 must pivot on the numerically sound row");
        let b = vec![1.0, 2.0];
        let x = lu.solve(&b);
        assert!(residual_inf(&weak, &x, &b) < 1e-9, "re-picked pivot still solves: {x:?}");
    }

    /// `max_nnz` を超える因子になる再利用分解は `None` を返す。
    #[test]
    fn reused_order_respects_the_fill_limit() {
        let m = 40;
        let rows = random_sparse_diag_dominant(m, 31337);
        let first = factorize(m, &rows).expect("factorizes");
        // `max_nnz` が対角だけより小さい: どんな分解も収まらないのでガードが働くこと。
        assert!(factorize_reusing_order(m, &rows, &first.col_perm, &first.row_perm, 1).is_none());
    }

    /// 列置換を繰り返しながら `factorize_reusing` で再分解した結果が、同じ行列の
    /// 一からの分解と FTRAN/BTRAN で一致する (エンドツーエンド)。
    #[test]
    fn factorize_reusing_matches_a_from_scratch_factorization_through_column_replacements() {
        let m = 45;
        let mut rows = random_sparse_diag_dominant(m, 555);
        let mut lu = FtLu::new(factorize(m, &rows).expect("factorizes"));
        let b: Vec<f64> = (0..m).map(|i| 0.5 + (i % 7) as f64).collect();

        for round in 0..4 {
            replace_columns(&mut rows, 3, 900 + round);
            lu = factorize_reusing(m, &rows, Some(&lu)).expect("refactorizes");

            let mut scratch = vec![0.0; m];
            let mut out = vec![0.0; m];
            lu.solve_into(&b, &mut scratch, &mut out);
            assert!(residual_inf(&rows, &out, &b) < 1e-9, "round {round}: FTRAN residual");

            let fresh = FtLu::new(factorize(m, &rows).expect("factorizes"));
            let mut fresh_out = vec![0.0; m];
            fresh.solve_into(&b, &mut scratch, &mut fresh_out);
            for i in 0..m {
                assert!((out[i] - fresh_out[i]).abs() < 1e-9, "round {round}, row {i}: reuse vs fresh");
            }

            // BTRAN も (`l_transpose_solve_into` の両経路を通して) 確認する。
            let ours = lu.solve_transpose(&b);
            let theirs = fresh.solve_transpose(&b);
            for i in 0..m {
                assert!((ours[i] - theirs[i]).abs() < 1e-9, "round {round}, row {i}: BTRAN reuse vs fresh");
            }
        }
    }

    /// 分解直後の因子で `solve_transpose_unit_into` が密な `solve_transpose` と
    /// ビット一致し、`scratch` を全 0 に戻す。
    #[test]
    fn solve_transpose_unit_into_matches_dense_on_fresh_factorization() {
        let m = 40;
        for seed in [1u64, 2, 3, 4, 5] {
            let rows = random_sparse_diag_dominant(m, seed);
            let base = factorize(m, &rows).expect("diagonally dominant matrix must factorize");
            let state = FtLu::new(base);
            assert_eq!(state.update_count(), 0, "fresh factorization must have no updates");

            let mut scratch = vec![0.0; m];
            let mut out = vec![0.0; m];
            for i in 0..m {
                let mut e_i = vec![0.0; m];
                e_i[i] = 1.0;
                let expected = state.solve_transpose(&e_i);

                state.solve_transpose_unit_into(i, &mut scratch, &mut out);
                assert_eq!(out, expected, "seed={seed} i={i}: solve_transpose_unit_into diverged from dense solve_transpose");
                assert!(scratch.iter().all(|&v| v == 0.0), "seed={seed} i={i}: scratch not restored to all-zero");
            }
        }
    }

    /// 単位ベクトル BTRAN (`solve_transpose_unit_capture` / `solve_transpose_unit`) が、
    /// 分解直後でも FT 更新後でも、一般右辺版 (`solve_transpose_into_capture`) と
    /// 結果・`e_tilde`・tick までビット一致する。
    #[test]
    fn solve_transpose_unit_is_bit_identical_to_the_dense_unit_rhs_path() {
        let m = 40;
        for seed in [1u64, 2, 3, 4, 5] {
            let rows = random_sparse_diag_dominant(m, seed);
            let base = factorize(m, &rows).expect("diagonally dominant matrix must factorize");
            let mut state = FtLu::new(base);

            // 分解直後と、FT 更新で `u_seq` が並べ替わった後 (`solve_transpose_unit_into`
            // の先頭スキップが使えないケース) の両方で確認する。
            for round in 0..3 {
                let (mut scratch, mut out, mut e_tilde) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                let (mut rscratch, mut rout, mut re_tilde) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                for i in 0..m {
                    let mut e_i = vec![0.0; m];
                    e_i[i] = 1.0;
                    state.solve_transpose_into_capture(&e_i, &mut rscratch, &mut rout, &mut re_tilde);

                    state.solve_transpose_unit_capture(i, &mut scratch, &mut out, &mut e_tilde);
                    assert_eq!(out, rout, "seed={seed} round={round} i={i}: unit BTRAN diverged from the dense-rhs one");
                    assert_eq!(e_tilde, re_tilde, "seed={seed} round={round} i={i}: captured e_tilde diverged");

                    // 記録なし版も両者と一致すること。
                    let mut out2 = vec![0.0; m];
                    state.solve_transpose_unit(i, &mut scratch, &mut out2);
                    assert_eq!(out2, rout, "seed={seed} round={round} i={i}: solve_transpose_unit diverged");
                }
                // tick は決定的な CLOCK 再分解トリガを駆動するので、両経路で同じだけ
                // 加算されなければならない (でないと求解の軌跡が変わる)。
                let before = state.synth_tick();
                let mut s1 = vec![0.0; m];
                let mut o1 = vec![0.0; m];
                let mut e1 = vec![0.0; m];
                state.solve_transpose_unit_capture(0, &mut s1, &mut o1, &mut e1);
                let unit_cost = state.synth_tick() - before;
                let before = state.synth_tick();
                let mut e_0 = vec![0.0; m];
                e_0[0] = 1.0;
                state.solve_transpose_into_capture(&e_0, &mut s1, &mut o1, &mut e1);
                assert_eq!(state.synth_tick() - before, unit_cost, "seed={seed} round={round}: unit and dense BTRAN must charge the same tick");

                let a_q: Vec<f64> = (0..m).map(|k| if k % 7 == round { 1.0 + k as f64 } else { 0.0 }).collect();
                if !state.try_update(round, &a_q, 1e-9) {
                    break;
                }
            }
        }
    }

    /// `solve_transpose_unit_work` (0 に保つ専用スクラッチ、追跡付き `e_tilde` コピー、
    /// 上界付き `L^T` ゲート) と、`StepCapture` による融合 `tau` FTRAN の GP `L` 段が、
    /// 通常経路と tick 込みでビット一致する (分解直後・FT 更新後とも)。
    #[test]
    fn unit_btran_work_and_tau_gp_are_bit_identical() {
        for m in [40usize, 200] {
            for seed in [1u64, 2, 3, 4, 5] {
                let rows = random_sparse_diag_dominant(m, seed);
                let base = factorize(m, &rows).expect("diagonally dominant matrix must factorize");
                let mut state = FtLu::new(base);
                let mut work = UnitBtranWork::new(m);
                let mut cap = StepCapture::new(m);
                let mut e_work = vec![0.0; m];
                for round in 0..6 {
                    for i in 0..m {
                        let (mut scratch, mut out, mut e_tilde) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                        let t0 = state.synth_tick();
                        state.solve_transpose_unit_capture(i, &mut scratch, &mut out, &mut e_tilde);
                        let ref_cost = state.synth_tick() - t0;
                        let mut out_w = vec![0.0; m];
                        let t0 = state.synth_tick();
                        state.solve_transpose_unit_work(i, &mut out_w, &mut e_work, &mut work, Some(&mut cap));
                        assert_eq!(state.synth_tick() - t0, ref_cost, "m={m} seed={seed} round={round} i={i}: tick");
                        assert_eq!(out_w, out, "m={m} seed={seed} round={round} i={i}: rho");
                        assert_eq!(e_work, e_tilde, "m={m} seed={seed} round={round} i={i}: e_tilde");
                        assert!(work.w.iter().all(|&v| v == 0.0));
                        // 記録ありとなしで融合 tau FTRAN を比較する。
                        let a: Vec<f64> = (0..m).map(|k| if (k + i) % 11 == round { 0.5 + k as f64 } else { 0.0 }).collect();
                        let (mut sa, mut sb, mut oa, mut ob, mut ta) = (vec![0.0; m], vec![0.0; m], vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                        let t0 = state.synth_tick();
                        let r1 = state.solve_into_pair_capture(&a, &out, &mut sa, &mut sb, &mut oa, &mut ob, &mut ta, None);
                        let c1 = state.synth_tick() - t0;
                        let (mut sa2, mut sb2, mut oa2, mut ob2, mut ta2) = (vec![0.0; m], vec![7.0; m], vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                        let t0 = state.synth_tick();
                        let r2 = state.solve_into_pair_capture(&a, &out, &mut sa2, &mut sb2, &mut oa2, &mut ob2, &mut ta2, Some(&mut cap));
                        assert_eq!(state.synth_tick() - t0, c1);
                        assert_eq!(r1, r2);
                        assert_eq!(oa, oa2);
                        assert_eq!(ta, ta2);
                        for k in 0..m {
                            assert!(ob[k] == ob2[k], "m={m} seed={seed} round={round} i={i}: tau[{k}]");
                        }
                    }
                    let a_q: Vec<f64> = (0..m).map(|k| if k % 7 == round { 1.0 + k as f64 } else { 0.0 }).collect();
                    if !state.try_update(round, &a_q, 1e-9) {
                        break;
                    }
                }
            }
        }
    }

    /// 主ループと同じ使い方 (出力・スクラッチを反復間で使い回し、`FtranTrack`/`UnitBtranWork`
    /// の記録で疎に書き出し、`try_update_tracked` で更新) が、毎回新しいバッファで求めた
    /// 参照 (記録なし・`try_update_precomputed`) と結果・tick までビット一致する (策1〜6)。
    #[test]
    fn tracked_iteration_matches_untracked_reference() {
        let mut n_partial = 0usize;
        for (m, partial) in [(40usize, false), (300, false), (300, true)] {
            for seed in [1u64, 2, 3, 7] {
                // 奇数 seed はランダムな疎行列 (結果が密になりやすく全走査への切り替えを通る)、
                // 偶数 seed は 6 行ずつのブロック対角 (超疎経路を通る)。
                let rows: Vec<Vec<(usize, f64)>> = if seed % 2 == 1 {
                    random_sparse_diag_dominant(m, seed)
                } else {
                    let mut rs = seed;
                    (0..m)
                        .map(|i| {
                            let b0 = i / 6 * 6;
                            let mut row: Vec<(usize, f64)> = Vec::new();
                            for j in b0..(b0 + 6).min(m) {
                                if j != i && next_rand(&mut rs) > 0.0 {
                                    row.push((j, next_rand(&mut rs)));
                                }
                            }
                            row.push((i, 8.0 + next_rand(&mut rs).abs()));
                            row.sort_unstable_by_key(|&(c, _)| c);
                            row
                        })
                        .collect()
                };
                let mut st = FtLu::new(factorize(m, &rows).unwrap());
                // 疎な `R` 段 (追加策 R) を少ない `R` eta でも通し、死んだ `U` eta ヘッダ (策4) を使う
                // (参照側 `sr` は常に全 eta を順に当て、ヘッダを削除する)。
                st.r_sparse_min = 0;
                st.lazy_remove = true;
                let mut sr = st.clone();
                sr.r_sparse_min = usize::MAX;
                sr.lazy_remove = false;
                let mut work = UnitBtranWork::new(m);
                let mut cap = StepCapture::new(m);
                let mut gp = GpScratch::new(m);
                let mut track = FtranTrack::new();
                let (mut rho, mut e_t, mut alpha, mut tau, mut a_t) = (vec![0.0; m], vec![0.0; m], vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                let (mut sa, mut sb) = (vec![0.0; m], vec![0.0; m]);
                let mut rng = seed * 7919 + 11;
                // 疎な書き出し・超疎 BTRAN が実際に使われた回数
                let (mut n_sparse_out, mut n_sparse_btran) = (0usize, 0usize);
                for it in 0..3 * m {
                    let r = ((next_rand(&mut rng).abs() * m as f64) as usize).min(m - 1);
                    // 入る列: 1〜3 個の非ゼロ
                    let a_sp: Vec<(usize, f64)> = {
                        let mut v: Vec<(usize, f64)> = Vec::new();
                        for _ in 0..1 + it % 3 {
                            let k = ((next_rand(&mut rng).abs() * m as f64) as usize).min(m - 1);
                            if !v.iter().any(|&(i, _)| i == k) {
                                v.push((k, 1.0 + next_rand(&mut rng)));
                            }
                        }
                        v.sort_unstable_by_key(|&(i, _)| i);
                        v
                    };
                    // 参照
                    let (mut rr, mut re, mut rs) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                    let t0 = sr.synth_tick();
                    sr.solve_transpose_unit_capture(r, &mut rs, &mut rr, &mut re);
                    let (mut za, mut zb, mut ra, mut rt, mut rat) = (vec![0.0; m], vec![0.0; m], vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                    let mut gpr = GpScratch::new(m);
                    let (rna, rnb) = sr.solve_sparse_into_pair_capture(&a_sp, &rr, &mut za, &mut gpr, &mut zb, &mut ra, &mut rt, &mut rat, None);
                    let ref_cost = sr.synth_tick() - t0;
                    // 記録付き
                    let t0 = st.synth_tick();
                    st.solve_transpose_unit_work_sparse(r, &mut rho, &mut e_t, &mut work, Some(&mut cap));
                    assert_eq!(rho, rr, "m={m} seed={seed} it={it}: rho");
                    assert_eq!(e_t, re, "m={m} seed={seed} it={it}: e_tilde");
                    if let Some(nz) = work.nonzero_rows() {
                        let want: Vec<usize> = (0..m).filter(|&i| rr[i] != 0.0).collect();
                        assert_eq!(nz, want.as_slice(), "m={m} seed={seed} it={it}: rho rows");
                    }
                    let hyper = it % 4 != 0;
                    gp.u_hyper = hyper;
                    cap.set_u_hyper(hyper);
                    // `partial` なら DSE `tau` を入る列の結果の非ゼロ行でだけ求める (策12)。
                    track.partial_tau = partial;
                    let (na, nb) = st.solve_sparse_into_pair_capture_tracked(&a_sp, &rho, &mut sa, &mut gp, &mut sb, &mut alpha, &mut tau, &mut a_t, Some(&mut cap), Some(&mut track));
                    assert_eq!(na, rna);
                    assert_eq!(alpha, ra, "m={m} seed={seed} it={it}: alpha");
                    for k in 0..m {
                        assert!(a_t[k] == rat[k], "m={m} seed={seed} it={it}: a_tilde[{k}]");
                    }
                    if track.tau_is_partial() {
                        n_partial += 1;
                        assert!(sb.iter().all(|&v| v == 0.0), "m={m} seed={seed} it={it}: tau scratch left dirty");
                        for k in 0..m {
                            if alpha[k] != 0.0 {
                                assert!(tau[k].to_bits() == rt[k].to_bits(), "m={m} seed={seed} it={it}: partial tau[{k}] {} vs {}", tau[k], rt[k]);
                            }
                        }
                        // CLOCK tick の数え方は違うので、以後の比較のために参照側に合わせる。
                        let d = (st.synth_tick() - t0) as i64 - ref_cost as i64;
                        sr.tick.set((sr.tick.get() as i64 + d) as u64);
                    } else {
                        assert_eq!(st.synth_tick() - t0, ref_cost, "m={m} seed={seed} it={it}: tick");
                        assert_eq!(nb, rnb);
                        for k in 0..m {
                            assert!(tau[k] == rt[k], "m={m} seed={seed} it={it}: tau[{k}]");
                        }
                    }
                    assert!(sa.iter().all(|&v| v == 0.0));
                    if let Some(idx) = track.alpha.indices() {
                        assert!((0..m).all(|i| alpha[i] == 0.0 || idx.contains(&i)));
                        n_sparse_out += 1;
                    }
                    n_sparse_btran += work.nonzero_rows().is_some() as usize;
                    // 単独の疎 FTRAN (BFRT の合成フリップ列の経路): 超疎 `U` 段・疎な `R` 段ありとなしで
                    // 値・非ゼロ数・tick が一致する。
                    {
                        let (mut z1, mut z2, mut o1, mut o2) = (vec![0.0; m], vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                        let (mut g1, mut g2) = (GpScratch::new(m), GpScratch::new(m));
                        g2.u_hyper = true;
                        let t1 = sr.synth_tick();
                        let n1 = sr.solve_sparse_into(&a_sp, &mut z1, &mut g1, &mut o1);
                        let c1 = sr.synth_tick() - t1;
                        let t2 = st.synth_tick();
                        let n2 = st.solve_sparse_into_hyper(&a_sp, &mut z2, &mut g2, &mut o2);
                        assert_eq!(st.synth_tick() - t2, c1, "m={m} seed={seed} it={it}: solve_sparse_into tick");
                        assert_eq!(n1, n2);
                        assert!(o1.iter().zip(&o2).all(|(x, y)| x.to_bits() == y.to_bits()), "m={m} seed={seed} it={it}: solve_sparse_into");
                        assert!(z2.iter().all(|&v| v == 0.0));
                    }
                    let slot = st.base.col_perm[st.base.row_perm_inv[r].min(m - 1)];
                    let ok_r = sr.try_update_precomputed(slot, &rat, &re, 1e-9);
                    let ok_t = st.try_update_tracked(slot, &a_t, &mut track, &e_t, &mut work, 1e-9);
                    assert_eq!(ok_r, ok_t);
                    if !ok_r || st.update_count() > 3 * m / 2 {
                        let rows2 = rows.clone();
                        st = FtLu::new(factorize(m, &rows2).unwrap());
                        st.r_sparse_min = 0;
                        st.lazy_remove = true;
                        sr = st.clone();
                        sr.lazy_remove = false;
                        sr.r_sparse_min = usize::MAX;
                    }
                    assert_eq!(st.fill_count(), sr.fill_count());
                }
                assert!(m < 100 || seed % 2 == 1 || (n_sparse_out > 0 && n_sparse_btran > 0), "m={m} seed={seed}: sparse paths never exercised ({n_sparse_out}, {n_sparse_btran})");
            }
        }
        assert!(n_partial > 0, "partial tau never exercised");
        assert!(R_SPARSE_CALLS.with(|c| c.get()) > 0, "sparse R stage never exercised");
    }

    /// `solve_into_pair_capture` / `solve_sparse_into_pair_capture` が、融合元の個別求解と
    /// 結果・`a_tilde`・非ゼロ数・tick までビット一致する (分解直後・FT 更新後とも)。
    #[test]
    fn pair_ftran_is_bit_identical_to_two_separate_solves() {
        let m = 40;
        for seed in [1u64, 2, 3, 4, 5] {
            let rows = random_sparse_diag_dominant(m, seed);
            let base = factorize(m, &rows).expect("diagonally dominant matrix must factorize");
            let mut state = FtLu::new(base);
            for round in 0..4 {
                for variant in 0..3usize {
                    // `a`: 列のような疎な右辺、`b`: やや密な右辺 (DSE の `rho_p` 形)。variant ごとに変える。
                    let a: Vec<f64> = (0..m).map(|k| if (k + variant) % 9 == round { 0.5 + k as f64 } else { 0.0 }).collect();
                    let b: Vec<f64> = (0..m).map(|k| if (k * 3 + variant) % (2 + variant) == 0 { 1.0 / (1.0 + k as f64) } else { 0.0 }).collect();

                    let (mut sa, mut sb) = (vec![0.0; m], vec![0.0; m]);
                    let (mut oa, mut ob, mut ta) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                    let t0 = state.synth_tick();
                    let na = state.solve_into_capture(&a, &mut sa, &mut oa, &mut ta);
                    let nb = state.solve_into(&b, &mut sb, &mut ob);
                    let sep_cost = state.synth_tick() - t0;

                    let (mut pa, mut pb) = (vec![0.0; m], vec![0.0; m]);
                    let (mut qa, mut qb, mut qt) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                    let t0 = state.synth_tick();
                    let (pna, pnb) = state.solve_into_pair_capture(&a, &b, &mut pa, &mut pb, &mut qa, &mut qb, &mut qt, None);
                    assert_eq!(state.synth_tick() - t0, sep_cost, "seed={seed} round={round} v={variant}: dense pair tick");
                    assert_eq!((pna, pnb), (na, nb));
                    assert_eq!(qa, oa, "seed={seed} round={round} v={variant}: dense pair a");
                    assert_eq!(qb, ob, "seed={seed} round={round} v={variant}: dense pair b");
                    assert_eq!(qt, ta, "seed={seed} round={round} v={variant}: dense pair a_tilde");

                    // 疎な `a` 版を `solve_sparse_into_capture` + `solve_into` と比較する。
                    let a_sp = to_sparse(&a);
                    let mut gp = GpScratch::new(m);
                    let mut zs = vec![0.0; m];
                    let (mut ra, mut rt) = (vec![0.0; m], vec![0.0; m]);
                    let t0 = state.synth_tick();
                    let rna = state.solve_sparse_into_capture(&a_sp, &mut zs, &mut gp, &mut ra, &mut rt);
                    let rnb = state.solve_into(&b, &mut sb, &mut ob);
                    let sep_cost = state.synth_tick() - t0;
                    let (mut xa, mut xb, mut xt) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                    let t0 = state.synth_tick();
                    let (xna, xnb) = state.solve_sparse_into_pair_capture(&a_sp, &b, &mut zs, &mut gp, &mut pb, &mut xa, &mut xb, &mut xt, None);
                    assert_eq!(state.synth_tick() - t0, sep_cost, "seed={seed} round={round} v={variant}: sparse pair tick");
                    assert_eq!((xna, xnb), (rna, rnb));
                    assert_eq!(xa, ra, "seed={seed} round={round} v={variant}: sparse pair a");
                    assert_eq!(xb, ob, "seed={seed} round={round} v={variant}: sparse pair b");
                    assert_eq!(xt, rt, "seed={seed} round={round} v={variant}: sparse pair a_tilde");
                    assert!(zs.iter().all(|&v| v == 0.0), "sparse pair must leave its scratch all-zero");
                }
                let a_q: Vec<f64> = (0..m).map(|k| if k % 5 == round { 1.0 + k as f64 } else { 0.0 }).collect();
                if !state.try_update(round, &a_q, 1e-9) {
                    break;
                }
            }
        }
    }

    /// 超疎 `U` 段 (C5) の有無で、疎 FTRAN 系のすべての入口 (単独・pair・triple、
    /// `StepCapture` 経由含む) の結果・非ゼロ数・tick がビット一致し、超疎段が
    /// 実際に 1 回以上走る。
    #[test]
    fn hyper_u_ftran_is_bit_identical() {
        let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<u64>>();
        let m = 200;
        // 超疎 `U` 段が中断せずに走った回数
        let mut hyper_taken = 0usize;
        for seed in [11u64, 12, 13, 14] {
            // `random_sparse_diag_dominant` より疎にする: 各行の非対角を 1 個だけ残し、
            // 短い右辺からの `U` の到達が中断率を超えないようにする。
            let rows: Vec<Vec<(usize, f64)>> = random_sparse_diag_dominant(m, seed)
                .into_iter()
                .enumerate()
                .map(|(i, r)| {
                    let mut kept = false;
                    r.into_iter()
                        .filter(|&(j, _)| {
                            if j == i {
                                return true;
                            }
                            let keep = !kept;
                            kept = true;
                            keep
                        })
                        .collect()
                })
                .collect();
            let base = factorize(m, &rows).expect("diagonally dominant matrix must factorize");
            let mut state = FtLu::new(base);
            for round in 0..6 {
                for variant in 0..4usize {
                    let a: Vec<f64> = (0..m).map(|k| if (k * 7 + variant) % 97 == round { 0.5 + k as f64 } else { 0.0 }).collect();
                    // variant 3 では `b` を非ゼロ 1 個にして、その超疎 `U` 段が中断せず走るようにする。
                    let b: Vec<f64> = (0..m)
                        .map(|k| {
                            let on = if variant == 3 { k == 17 + round } else { (k * 3 + variant) % (2 + variant) == 0 };
                            if on {
                                1.0 / (1.0 + k as f64)
                            } else {
                                0.0
                            }
                        })
                        .collect();
                    let c: Vec<f64> = (0..m).map(|k| if (k + 2 * variant) % 5 == 0 { -1.0 - k as f64 } else { 0.0 }).collect();
                    let a_sp = to_sparse(&a);
                    let mut res: Vec<(u64, Vec<Vec<u64>>, (usize, usize, usize, usize))> = Vec::new();
                    for hyper in [false, true] {
                        let mut gp = GpScratch::new(m);
                        gp.u_hyper = hyper;
                        let mut zs = vec![0.0; m];
                        let (mut o1, mut t1) = (vec![0.0; m], vec![0.0; m]);
                        let t0 = state.synth_tick();
                        let n1 = state.solve_sparse_into_capture(&a_sp, &mut zs, &mut gp, &mut o1, &mut t1);
                        let (mut sb, mut sc) = (vec![0.0; m], vec![0.0; m]);
                        let (mut o2a, mut o2b, mut t2) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                        let (n2, _) = state.solve_sparse_into_pair_capture(&a_sp, &b, &mut zs, &mut gp, &mut sb, &mut o2a, &mut o2b, &mut t2, None);
                        let (mut o3a, mut o3b, mut o3c, mut t3) = (vec![0.0; m], vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                        let (n3, _, n3c) = state.solve_sparse_into_triple_capture(&a_sp, &b, &c, &mut zs, &mut gp, &mut sb, &mut sc, &mut o3a, &mut o3b, &mut o3c, &mut t3, None);
                        // `tau` チャネルをステップ記録経由で (`b` の GP `L` 段)、要求時は超疎 `U` で。
                        let steps: Vec<usize> = (0..m).filter(|&s| b[state.base.row_perm[s]] != 0.0).collect();
                        let mk_cap = |steps: &Vec<usize>| {
                            let mut cap = StepCapture::new(m);
                            cap.steps = steps.clone();
                            cap.valid = true;
                            cap.set_u_hyper(hyper);
                            cap
                        };
                        let mut cap = mk_cap(&steps);
                        let (mut o4a, mut o4b, mut t4) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                        let (n4a, n4b) = state.solve_sparse_into_pair_capture(&a_sp, &b, &mut zs, &mut gp, &mut sb, &mut o4a, &mut o4b, &mut t4, Some(&mut cap));
                        let mut cap = mk_cap(&steps);
                        let (mut sa, mut o5a, mut o5b, mut t5) = (vec![0.0; m], vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                        let (n5a, n5b) = state.solve_into_pair_capture(&a, &b, &mut sa, &mut sb, &mut o5a, &mut o5b, &mut t5, Some(&mut cap));
                        let mut cap = mk_cap(&steps);
                        let (mut o6a, mut o6b, mut o6c, mut t6) = (vec![0.0; m], vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                        let (n6a, n6b, n6c) = state.solve_sparse_into_triple_capture(&a_sp, &b, &c, &mut zs, &mut gp, &mut sb, &mut sc, &mut o6a, &mut o6b, &mut o6c, &mut t6, Some(&mut cap));
                        let mut cap = mk_cap(&steps);
                        let (mut o7a, mut o7b, mut o7c, mut t7) = (vec![0.0; m], vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                        let (n7a, n7b, n7c) = state.solve_into_triple_capture(&a, &b, &c, &mut sa, &mut sb, &mut sc, &mut o7a, &mut o7b, &mut o7c, &mut t7, Some(&mut cap));
                        assert!(zs.iter().all(|&v| v == 0.0), "scratch must be left all-zero");
                        let tick = state.synth_tick() - t0;
                        res.push((
                            tick,
                            vec![
                                bits(&o1), bits(&t1), bits(&o2a), bits(&o2b), bits(&t2), bits(&o3a), bits(&o3b), bits(&o3c), bits(&t3),
                                bits(&o4a), bits(&o4b), bits(&t4), bits(&o5a), bits(&o5b), bits(&t5),
                                bits(&o6a), bits(&o6b), bits(&o6c), bits(&t6), bits(&o7a), bits(&o7b), bits(&o7c), bits(&t7),
                                vec![n4a as u64, n4b as u64, n5a as u64, n5b as u64, n6a as u64, n6b as u64, n6c as u64, n7a as u64, n7b as u64, n7c as u64],
                            ],
                            (n1, n2, n3, n3c),
                        ));
                        if hyper {
                            // 超疎段が (中断せずに) 実際に走ったことを直接確認する。
                            let mut x = t1.clone();
                            let mut gp2 = GpScratch::new(m);
                            gp2.reach = (0..m).filter(|&s| x[s] != 0.0).collect();
                            if state.u_solve_hyper(&mut x, &mut gp2) {
                                hyper_taken += 1;
                            }
                        }
                    }
                    assert_eq!(res[0], res[1], "seed={seed} round={round} v={variant}: hyper U stage must be bit-identical");
                }
                let a_q: Vec<f64> = (0..m).map(|k| if k % 11 == round { 1.0 + k as f64 } else { 0.0 }).collect();
                if !state.try_update(round * 3, &a_q, 1e-9) {
                    break;
                }
            }
        }
        assert!(hyper_taken > 0, "the hyper-sparse U stage never ran");
    }

    /// 密ベクトルを非ゼロの `(添字, 値)` 列に変換する。
    fn to_sparse(dense: &[f64]) -> Vec<(usize, f64)> {
        dense.iter().enumerate().filter(|&(_, &v)| v != 0.0).map(|(i, &v)| (i, v)).collect()
    }

    /// `rhs` を疎形式で `solve_sparse_into` に通し、密な参照 `state.solve(rhs)` と
    /// **ビット一致** することを確認する (同じ到達集合上で同じ演算列になるはずなので、
    /// 少しでも違えば到達集合か呼び出し間のゼロ管理が誤っている)。
    fn assert_sparse_matches_dense(state: &FtLu, m: usize, rhs: &[f64], scratch: &mut [f64], gp: &mut GpScratch, out: &mut [f64]) {
        let expected = state.solve(rhs);
        state.solve_sparse_into(&to_sparse(rhs), scratch, gp, out);
        assert_eq!(&out[..m], &expected[..], "rhs={rhs:?}");
    }

    /// 3x3 三重対角行列で疎 FTRAN が密 FTRAN とビット一致する。
    #[test]
    fn sparse_solve_matches_dense_on_simple_case() {
        // `factorize_and_solve_matches_expected` と同じ 3x3 三重対角行列。
        let rows = vec![vec![(0, 2.0), (1, 1.0)], vec![(0, 1.0), (1, 3.0), (2, 1.0)], vec![(1, 1.0), (2, 4.0)]];
        let base = factorize(3, &rows).unwrap();
        let state = FtLu::new(base);
        let m = 3;
        let mut scratch = vec![0.0; m];
        let mut gp = GpScratch::new(m);
        let mut out = vec![0.0; m];

        for rhs in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [3.0, -2.0, 7.0],
            [0.0, 0.0, 0.0],
        ] {
            assert_sparse_matches_dense(&state, m, &rhs, &mut scratch, &mut gp, &mut out);
        }
    }

    /// FT 更新後 (`r_etas` が非空) でも疎 FTRAN が密 FTRAN とビット一致する。
    #[test]
    fn sparse_solve_matches_dense_with_ft_updates() {
        // `ft_update_chain_of_two_matches_full_refactor` と同じ行列・更新列。
        let rows0 = vec![vec![(0, 2.0), (1, 1.0)], vec![(0, 1.0), (1, 3.0), (2, 1.0)], vec![(1, 1.0), (2, 4.0)]];
        let base = factorize(3, &rows0).unwrap();
        let mut state = FtLu::new(base);
        assert!(state.try_update(1, &[1.0, 5.0, 2.0], 1e-9));
        assert!(state.try_update(0, &[4.0, 1.0, 3.0], 1e-9));

        let m = 3;
        let mut scratch = vec![0.0; m];
        let mut gp = GpScratch::new(m);
        let mut out = vec![0.0; m];

        for rhs in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [2.0, -3.0, 1.0]] {
            assert_sparse_matches_dense(&state, m, &rhs, &mut scratch, &mut gp, &mut out);
        }
    }

    /// 同じ `scratch`/`gp` を使って多様な右辺 (単一非ゼロ・散在・密・全 0) を連続で
    /// 疎 FTRAN しても毎回密版と一致し、最後に `scratch` が全 0 に戻っている
    /// (前回の残りが次回を壊さないことの確認)。
    #[test]
    fn sparse_solve_repeated_calls_reuse_scratch_correctly() {
        let rows0 = vec![
            vec![(0, 4.0), (2, 1.0)],
            vec![(1, 3.0), (3, 1.0)],
            vec![(0, 1.0), (2, 5.0), (4, 1.0)],
            vec![(1, 1.0), (3, 6.0), (5, 2.0)],
            vec![(2, 1.0), (4, 4.0)],
            vec![(3, 1.0), (5, 3.0)],
        ];
        let base = factorize(6, &rows0).unwrap();
        let state = FtLu::new(base);
        let m = 6;
        let mut scratch = vec![0.0; m];
        let mut gp = GpScratch::new(m);
        let mut out = vec![0.0; m];

        let rhs_sequence: Vec<[f64; 6]> = vec![
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 2.0, 0.0, 0.0, 3.0],
            [0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        ];
        for rhs in &rhs_sequence {
            assert_sparse_matches_dense(&state, m, rhs, &mut scratch, &mut gp, &mut out);
        }
        // 最後の呼び出し後もバッファが厳密に 0 であること (次の呼び出し側が依存する不変条件)。
        assert!(scratch.iter().all(|&v| v == 0.0), "scratch not fully cleared: {scratch:?}");
    }

    /// 6x6 基底で位置をばらした 5 回の FT 更新 (`u_seq` の並べ替え) の *毎回* の後に、
    /// 疎 FTRAN が密 FTRAN とビット一致する。
    #[test]
    fn sparse_solve_matches_dense_after_reordering_u_seq() {
        let rows0 = vec![
            vec![(0, 3.0), (2, 1.0)],
            vec![(1, 4.0), (3, 1.0)],
            vec![(0, 1.0), (2, 5.0), (4, 1.0)],
            vec![(1, 1.0), (3, 6.0), (5, 1.0)],
            vec![(2, 1.0), (4, 4.0)],
            vec![(3, 1.0), (5, 3.0)],
        ];
        let base = factorize(6, &rows0).unwrap();
        let mut state = FtLu::new(base);
        let m = 6;
        let mut scratch = vec![0.0; m];
        let mut gp = GpScratch::new(m);
        let mut out = vec![0.0; m];

        let rhs_probes: [[f64; 6]; 5] = [
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 2.0, 0.0, 3.0],
            [1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        ];
        // 更新するスロット: 4 (末尾近く), 0 (先頭), 5 (新しい末尾), 2 (中央), 1。
        // 単調でない順にして `u_seq` の位置とスロットの関係をかき混ぜる。
        let updates: [(usize, [f64; 6]); 5] = [
            (4, [1.0, 0.0, 2.0, 0.0, 3.0, 0.0]),
            (0, [4.0, 1.0, 0.0, 0.0, 1.0, 2.0]),
            (5, [0.0, 2.0, 1.0, 3.0, 0.0, 5.0]),
            (2, [2.0, 0.0, 3.0, 1.0, 0.0, 1.0]),
            (1, [1.0, 3.0, 2.0, 0.0, 1.0, 0.0]),
        ];
        for &(slot, a_q) in &updates {
            assert!(state.try_update(slot, &a_q, 1e-9), "update on slot {slot} rejected");
            for rhs in &rhs_probes {
                assert_sparse_matches_dense(&state, m, rhs, &mut scratch, &mut gp, &mut out);
            }
        }
        assert!(scratch.iter().all(|&v| v == 0.0), "scratch not fully cleared: {scratch:?}");
    }

    /// `should_use_dense_solve` が密な右辺 (5/10) を密と判定し、疎な右辺 (2/10) は判定しない。
    #[test]
    fn should_use_dense_solve_flags_dense_rhs_and_not_sparse() {
        let m = 10;
        let rows: Vec<Vec<(usize, f64)>> = (0..m).map(|i| vec![(i, 4.0)]).collect();
        let lu = FtLu::new(factorize(m, &rows).expect("nonsingular"));
        // 10 個中 5 個が非ゼロの右辺は DENSE_RHS_FRACTION (0.4) を超える。
        assert!(lu.should_use_dense_solve(5), "5/10 nonzero rhs should be flagged dense");
        assert!(!lu.should_use_dense_solve(2), "2/10 nonzero rhs should not be flagged dense");
    }

    /// 結果密度ゲート: 結果が密であり続けるチャネルは、右辺が疎でも密経路に切り替わり、
    /// 結果が疎に戻れば疎経路に戻る (両経路で記録するので張り付かない)。
    #[test]
    fn density_gate_flips_a_sparse_rhs_channel_dense_and_back() {
        let m = 10;
        let rows: Vec<Vec<(usize, f64)>> = (0..m).map(|i| vec![(i, 4.0)]).collect();
        let lu = FtLu::new(factorize(m, &rows).expect("nonsingular"));
        let mut density = FtranDensity::new();
        // 履歴なし: 判定は `should_use_dense_solve` と同じ。
        assert!(!lu.should_use_dense_solve_tracked(2, &density), "a fresh channel must not be gated dense");

        // 毎回完全に密な結果: 移動平均が EXPECTED_DENSE_FRACTION (0.35) を超え、
        // 同じ疎な右辺が密経路になる。
        for _ in 0..30 {
            density.record(m, m);
        }
        assert!(density.expected() > EXPECTED_DENSE_FRACTION, "expected={}", density.expected());
        assert!(lu.should_use_dense_solve_tracked(2, &density), "a persistently dense channel must be gated dense");

        // 戻る: 密経路も自分の結果密度を記録するので張り付かない。
        for _ in 0..60 {
            density.record(0, m);
        }
        assert!(!lu.should_use_dense_solve_tracked(2, &density), "expected={}", density.expected());
    }

    /// FTRAN の各入口が書いた結果の非ゼロ数を返し、密・疎経路で一致する。
    #[test]
    fn solve_paths_report_the_result_nonzero_count() {
        let m = 6;
        // 下二重対角の `B`: `B^-1 e_0` は全行に fill するので、非ゼロ 1 個の右辺から
        // 完全に密な結果になる (入力側の判定だけでは予見できないケース)。
        let rows: Vec<Vec<(usize, f64)>> =
            (0..m).map(|i| if i == 0 { vec![(0, 2.0)] } else { vec![(i - 1, -2.0), (i, 2.0)] }).collect();
        let lu = FtLu::new(factorize(m, &rows).expect("nonsingular"));

        let mut dense_rhs = vec![0.0; m];
        dense_rhs[0] = 1.0;
        let mut scratch = vec![0.0; m];
        let mut out_dense = vec![0.0; m];
        let dense_nnz = lu.solve_into(&dense_rhs, &mut scratch, &mut out_dense);

        let mut sparse_scratch = vec![0.0; m];
        let mut gp = GpScratch::new(m);
        let mut out_sparse = vec![0.0; m];
        let sparse_nnz = lu.solve_sparse_into(&[(0, 1.0)], &mut sparse_scratch, &mut gp, &mut out_sparse);

        assert_eq!(out_dense, out_sparse, "the two FTRAN paths must agree on the result itself");
        assert_eq!(dense_nnz, sparse_nnz, "...and on its nonzero count");
        assert_eq!(dense_nnz, out_dense.iter().filter(|&&v| v != 0.0).count());
        assert_eq!(dense_nnz, m, "this fixture's whole point is a dense result from a one-nonzero rhs");
    }

    /// 係数が稠密な基底 (FT 更新後に `DENSE_ETA_FRACTION` を超える) に稠密な入る列で
    /// 複数回 FT 更新し、最終基底の独立な完全再分解と FTRAN/BTRAN/疎 FTRAN で一致する
    /// (eta の密形式を通すため)。
    #[test]
    fn ft_update_matches_full_refactor_on_dense_basis() {
        let m = 10;
        let entry = |i: usize, j: usize| -> f64 { if i == j { 50.0 } else { 1.0 + ((i + 2 * j) % 5) as f64 * 0.3 } };
        let rows0: Vec<Vec<(usize, f64)>> = (0..m).map(|i| (0..m).map(|j| (j, entry(i, j))).collect()).collect();
        let base = factorize(m, &rows0).expect("nonsingular");
        let mut state = FtLu::new(base);

        // 稠密な入る列 (全要素非ゼロ) で、いくつかの異なるスロットを置換する。
        let entering = |k: usize, slot: usize| -> Vec<f64> {
            (0..m).map(|i| if i == slot { 40.0 + k as f64 } else { 2.0 + ((i * 3 + k) % 7) as f64 }).collect()
        };
        let updates = [(3usize, 0usize), (7, 1), (0, 2)];
        let mut cur_rows = rows0.clone();
        for &(k, slot) in &updates {
            let a_q = entering(k, slot);
            assert!(state.try_update(slot, &a_q, 1e-9), "update on slot {slot} rejected");
            for i in 0..m {
                cur_rows[i].retain(|&(c, _)| c != slot);
                if a_q[i] != 0.0 {
                    cur_rows[i].push((slot, a_q[i]));
                }
            }
        }

        let full = factorize(m, &cur_rows).expect("updated dense basis must still be nonsingular");
        let rhs: Vec<f64> = (0..m).map(|i| 1.0 + i as f64 * 0.5).collect();

        let x_ft = state.solve(&rhs);
        let x_full = full.solve(&rhs);
        assert!(approx_vec(&x_ft, &x_full), "ft={x_ft:?} full={x_full:?}");

        let y_ft = state.solve_transpose(&rhs);
        let y_full = full.solve_transpose(&rhs);
        assert!(approx_vec(&y_ft, &y_full), "ft={y_ft:?} full={y_full:?}");

        // 呼び出し側では密な右辺は `solve_sparse_into` を通らないが、公開 API として
        // 常に正しくなければならないので確認する。
        let mut scratch = vec![0.0; m];
        let mut gp = GpScratch::new(m);
        let mut out = vec![0.0; m];
        state.solve_sparse_into(&to_sparse(&rhs), &mut scratch, &mut gp, &mut out);
        assert!(approx_vec(&out, &x_full), "sparse-rhs path ft={out:?} full={x_full:?}");
    }
}
