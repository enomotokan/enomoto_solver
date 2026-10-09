//! 基底行列 `B` の求解の心臓部 (FTRAN・BTRAN・Forrest-Tomlin 更新・再分解の判定) を、
//! 主単体法と双対単体法で共有するための部品。
//!
//! 因子そのもの (`L`/`U`/`R` eta、各求解の本体) は [`sparse_lu::FtLu`] が持つ。この
//! モジュールが持つのはその一つ上の層、すなわち「どの版の求解を呼び、FT 更新と再分解を
//! いつ行うか」の部分:
//!
//! - [`BasisKernel::ftran_col`]: 入る列の FTRAN。右辺の非ゼロ数と結果密度の移動平均
//!   ([`sparse_lu::FtranDensity`]) で密/疎 (Gilbert-Peierls) を切り替え、FT 更新用の
//!   中間値 `a_tilde` を記録する。
//! - [`BasisKernel::btran_row`]: ピボット行の BTRAN `B^-T e_r` ([`sparse_lu::UnitBtranWork`])。
//!   FT 更新用の中間値 `e_tilde` を記録する。
//! - [`BasisKernel::update_and_check`]: 記録した `a_tilde`/`e_tilde` で FT 更新し
//!   ([`sparse_lu::FtLu::try_update`] のように同じ求解をやり直さない)、続けて再分解トリガ
//!   (更新回数の上限・合成クロック・eta の fill) を判定する。
//!
//! 双対単体法の主ループの融合 FTRAN (入る列・DSE の `tau`・BFRT の合成フリップ列を
//! 1 回の因子走査で解く pair/triple capture) と、大きな問題向けの疎経路 (`BIG`) は、
//! DSE・BFRT と結びついているので `slope_intercept_dual` 側に置く。

use super::{sparse_lu, StdForm};
use crate::params::simplex::{FT_BUMP_LIMIT_FACTOR, FT_CHECK_INTERVAL, FT_MIN_PIVOT};
use crate::params::slope_intercept_dual::{FT_MAX_UPDATES_FACTOR, FT_MAX_UPDATES_FLOOR, SYNTH_CLOCK_FACTOR, SYNTH_CLOCK_LARGE_M, SYNTH_CLOCK_LARGE_REF_M, SYNTH_CLOCK_MID_REF_MULT, SYNTH_CLOCK_MIN_UPDATES};
use std::sync::OnceLock;

/// [`BasisKernel::update_and_check`] の判定結果 (どのトリガで再分解が要るか)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum RefactorDue {
    /// 再分解は不要。
    No,
    /// 周期検査 ([`FT_CHECK_INTERVAL`] 反復ごと) の時点で、fill では再分解不要だった。
    /// 呼び出し側は必要なら自分の残差検査をここで行う。
    Periodic,
    /// トリガ (2): FT 更新が新しいピボットを小さすぎるとして退けた (更新は記録されていない)。
    Rejected,
    /// トリガ (4): FT 更新回数が上限を超えた。
    MaxUpdates,
    /// トリガ (5): 合成クロック。
    Clock,
    /// トリガ (3): 周期検査で eta の fill が上限を超えた (周期検査の時点なので、[`Self::Periodic`] と
    /// 同じく呼び出し側の周期の数え上げを進めること)。
    Fill,
}

impl RefactorDue {
    /// 再分解が必要か。
    #[inline]
    pub(super) fn is_due(self) -> bool {
        !matches!(self, RefactorDue::No | RefactorDue::Periodic)
    }

    /// 周期検査の時点だったか ([`Self::Periodic`] または [`Self::Fill`])。
    #[inline]
    pub(super) fn is_periodic(self) -> bool {
        matches!(self, RefactorDue::Periodic | RefactorDue::Fill)
    }
}

/// 1 回の求解ループが持つ、基底の求解の作業領域と再分解の方針 (モジュールの説明参照)。
///
/// 使い方 (1 反復): `ftran_col(lu, 入る列)` → `btran_row(lu, r)` → (基底の入れ替え) →
/// `update_and_check(lu, r).is_due()` なら再分解。`ftran_col` と `btran_row` は同じ因子
/// (間に FT 更新や再分解を挟まない) で、`update_and_check` に渡す `r` は `btran_row` と
/// 同じ行であること。`ftran_col` と `btran_row` の順序は問わない。
///
/// 双対単体法の主ループは入る列の FTRAN を DSE の `tau` などと融合して自分で解くので、
/// `ftran_col` の代わりにその `a_tilde` を [`Self::update_and_check_fused`] に渡す。BTRAN は
/// 融合 FTRAN 用の非ゼロステップの記録付きの [`Self::btran_row_steps`] を使う。
pub(super) struct BasisKernel {
    /// 行数。
    m: usize,
    /// 密な FTRAN の右辺 (入る列を散布する。呼び出し間は全 0)。
    dense_rhs: Vec<f64>,
    /// 密な FTRAN の作業領域。
    dense_scratch: Vec<f64>,
    /// 疎な FTRAN の作業領域 (呼び出し間は全 0、[`sparse_lu::FtLu::solve_sparse_into_capture`] の約束)。
    sparse_scratch: Vec<f64>,
    gp: sparse_lu::GpScratch,
    /// 入る列の FTRAN の結果密度の移動平均 (再分解をまたいで保持する)。
    density_col: sparse_lu::FtranDensity,
    /// ピボット行 BTRAN の作業領域 (`e_tilde` の書き込み位置の記録を含むので、`e_tilde` と対で持つ)。
    btran_work: sparse_lu::UnitBtranWork,
    /// FT 更新用の中間値: 入る列の FTRAN の `L`/`R` 適用後の値。
    a_tilde: Vec<f64>,
    /// FT 更新用の中間値: ピボット行 BTRAN の `U^-T` 適用後の値。
    e_tilde: Vec<f64>,
    /// `a_tilde` がこの因子の入る列について記録済みか。
    a_tilde_ready: bool,
    /// `e_tilde` を記録した行 (未記録なら `None`)。
    e_tilde_row: Option<usize>,
    /// 周期検査までの反復数。
    since_check: usize,
    /// 周期検査の間隔 (既定 [`FT_CHECK_INTERVAL`]、[`Self::with_periodic_check`])。
    check_interval: usize,
    /// 分解自体が `bump_limit` より大きいとき、fill の上限を `bump_lu_ratio * nnz(LU)` まで広げる
    /// (0 なら広げない。既定 0、[`Self::with_periodic_check`])。
    bump_lu_ratio: f64,
    /// eta の fill の上限の基準 (`FT_BUMP_LIMIT_FACTOR * m`)。
    bump_limit: usize,
    /// FT 更新で受け入れるピボットの絶対値の下限。
    min_pivot: f64,
    /// 再分解トリガ (4) の FT 更新回数の上限。
    max_updates: usize,
}

impl BasisKernel {
    /// 行数 `m` 用の作業領域を作る。`max_updates` は再分解トリガ (4) の FT 更新回数の上限:
    /// 双対単体法は [`ft_max_updates`] (`m` に比例)、主単体法は固定の `FT_MAX_UPDATES` を渡す
    /// (主単体法に `m` 比例の上限を使うと、rail4284 の篩い分けで eta が伸びて 1 反復が約 7% 遅くなった)。
    pub(super) fn new(m: usize, max_updates: usize) -> Self {
        BasisKernel {
            m,
            dense_rhs: vec![0.0; m],
            dense_scratch: vec![0.0; m],
            sparse_scratch: vec![0.0; m],
            gp: sparse_lu::GpScratch::new(m),
            density_col: sparse_lu::FtranDensity::new(),
            btran_work: sparse_lu::UnitBtranWork::new(m),
            a_tilde: vec![0.0; m],
            e_tilde: vec![0.0; m],
            a_tilde_ready: false,
            e_tilde_row: None,
            since_check: 0,
            check_interval: FT_CHECK_INTERVAL,
            bump_lu_ratio: 0.0,
            bump_limit: tunable!("ENOMOTO_T_FT_BUMP_LIMIT_FACTOR", FT_BUMP_LIMIT_FACTOR, usize) * m.max(1),
            min_pivot: tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64),
            max_updates,
        }
    }

    /// 周期検査の間隔 `interval` と、fill の上限を分解の大きさで広げる係数 `bump_lu_ratio`
    /// (0 で広げない) を設定する (双対単体法の主ループ用。`x_B` のずれの検査と同じ周期にする)。
    pub(super) fn with_periodic_check(mut self, interval: usize, bump_lu_ratio: f64) -> Self {
        self.check_interval = interval;
        self.bump_lu_ratio = bump_lu_ratio;
        self
    }

    /// 入る列 `col` (`(行, 値)` の疎な列) の FTRAN `out = B^-1 col` (`out` は長さ `m`、全体を書く)。
    /// 右辺が密か最近の結果が密なら密経路、それ以外は疎経路。FT 更新用に `a_tilde` を記録する。
    /// 結果の非ゼロ数を返す。
    #[inline]
    pub(super) fn ftran_col(&mut self, lu: &sparse_lu::FtLu, col: &[(usize, f64)], out: &mut [f64]) -> usize {
        let nnz = if lu.should_use_dense_solve_tracked(col.len(), &self.density_col) {
            for &(i, v) in col {
                self.dense_rhs[i] = v;
            }
            let nnz = lu.solve_into_capture(&self.dense_rhs, &mut self.dense_scratch, out, &mut self.a_tilde);
            for &(i, _) in col {
                self.dense_rhs[i] = 0.0;
            }
            nnz
        } else {
            lu.solve_sparse_into_capture(col, &mut self.sparse_scratch, &mut self.gp, out, &mut self.a_tilde)
        };
        self.density_col.record(nnz, self.m);
        self.a_tilde_ready = true;
        nnz
    }

    /// ピボット行の BTRAN `out = B^-T e_r` (`out` は長さ `m`、全体を書く)。FT 更新用に `e_tilde` を記録する。
    #[inline]
    pub(super) fn btran_row(&mut self, lu: &sparse_lu::FtLu, r: usize, out: &mut [f64]) {
        lu.solve_transpose_unit_work(r, out, &mut self.e_tilde, &mut self.btran_work, None);
        self.e_tilde_row = Some(r);
    }

    /// [`Self::btran_row`] に、続く融合 FTRAN (DSE の `tau = B^-1 rho`) のための非ゼロステップの
    /// 記録 `steps` を加えたもの。`sparse` なら超疎版
    /// ([`sparse_lu::FtLu::solve_transpose_unit_work_sparse`]、大きな問題用。`out` は前回の非ゼロ位置
    /// だけを消して書くので、`out` を外で書いたら [`Self::btran_invalidate_out`] を呼ぶこと)。
    /// 結果はどちらもビット一致。
    #[inline]
    pub(super) fn btran_row_steps(&mut self, lu: &sparse_lu::FtLu, r: usize, out: &mut [f64], steps: &mut sparse_lu::StepCapture, sparse: bool) {
        if sparse {
            lu.solve_transpose_unit_work_sparse(r, out, &mut self.e_tilde, &mut self.btran_work, Some(steps));
        } else {
            lu.solve_transpose_unit_work(r, out, &mut self.e_tilde, &mut self.btran_work, Some(steps));
        }
        self.e_tilde_row = Some(r);
    }

    /// 直前のピボット行 BTRAN の結果の非ゼロ行 (昇順)。超疎版で求めたときだけ `Some`。
    #[inline]
    pub(super) fn btran_nonzero_rows(&self) -> Option<&[usize]> {
        self.btran_work.nonzero_rows()
    }

    /// ピボット行 BTRAN の出力ベクトルを外で書いた後に呼ぶ ([`Self::btran_row_steps`] 参照)。
    #[inline]
    pub(super) fn btran_invalidate_out(&mut self) {
        self.btran_work.invalidate_out();
    }

    /// 基底位置 `r` の列を直前の [`Self::ftran_col`] の入る列で置き換える FT 更新を記録した
    /// `a_tilde`/`e_tilde` で行い (記録はこの呼び出しで使い切る)、続けて再分解トリガを判定する。
    /// 判定の順序 (どれかが当たったら残りは見ない): (2) 更新が退けられた (または記録が揃って
    /// いない)、(4) 更新回数が `max_updates` ([`Self::new`]) を超えた、(5) 合成クロック
    /// ([`synth_clock_should_refactor_density`]、[`SynthDensity::Sparse`])、(3) 周期検査
    /// (既定 [`FT_CHECK_INTERVAL`] 反復ごと) で eta の fill が上限を超えた ([`Self::fill_too_big`])。
    /// 周期の数え上げは更新の成否によらず毎回進め、周期検査に達したときだけ 0 に戻す。
    #[inline]
    pub(super) fn update_and_check(&mut self, lu: &mut sparse_lu::FtLu, r: usize) -> RefactorDue {
        self.since_check += 1;
        let captured = self.a_tilde_ready && self.e_tilde_row == Some(r);
        debug_assert!(captured, "BasisKernel::update_and_check without ftran_col/btran_row for row {r}");
        self.a_tilde_ready = false;
        self.e_tilde_row = None;
        let updated = captured && lu.try_update_precomputed(r, &self.a_tilde, &self.e_tilde, self.min_pivot);
        self.check_after_update(lu, updated, SynthDensity::Sparse)
    }

    /// [`Self::update_and_check`] の、ピボット行の BTRAN を [`Self::btran_row_steps`] (超疎版) で解いたときの版:
    /// R eta を BTRAN の非ゼロ位置の記録から作る ([`sparse_lu::FtLu::try_update_e_tracked`]、結果はビット一致)。
    /// 行数が多く基底のほとんどがスラックの問題 (クロスオーバーの Megiddo 式の押し出し) で長さ `m` の走査を省く。
    #[inline]
    pub(super) fn update_and_check_e_tracked(&mut self, lu: &mut sparse_lu::FtLu, r: usize) -> RefactorDue {
        self.since_check += 1;
        let captured = self.a_tilde_ready && self.e_tilde_row == Some(r);
        debug_assert!(captured, "BasisKernel::update_and_check_e_tracked without ftran_col/btran_row_steps for row {r}");
        self.a_tilde_ready = false;
        self.e_tilde_row = None;
        let updated = captured && lu.try_update_e_tracked(r, &self.a_tilde, &self.e_tilde, &mut self.btran_work, self.min_pivot);
        self.check_after_update(lu, updated, SynthDensity::Sparse)
    }

    /// [`Self::update_and_check`] の、入る列の FTRAN を呼び出し側が解いた版 (双対単体法の主ループの
    /// 融合 FTRAN)。`a_tilde` はその FTRAN が同じ因子で記録した中間値。`ftrack` があれば、その FTRAN と
    /// 直前の [`Self::btran_row_steps`] の非ゼロ位置の記録から eta を作る
    /// ([`sparse_lu::FtLu::try_update_tracked`]、結果はビット一致)。`density` は合成クロックの係数の区分。
    #[inline]
    pub(super) fn update_and_check_fused(&mut self, lu: &mut sparse_lu::FtLu, r: usize, a_tilde: &[f64], ftrack: Option<&mut sparse_lu::FtranTrack>, density: SynthDensity) -> RefactorDue {
        self.since_check += 1;
        let captured = self.e_tilde_row == Some(r);
        debug_assert!(captured, "BasisKernel::update_and_check_fused without btran_row_steps for row {r}");
        self.a_tilde_ready = false;
        self.e_tilde_row = None;
        let updated = captured
            && match ftrack {
                Some(track) => lu.try_update_tracked(r, a_tilde, track, &self.e_tilde, &mut self.btran_work, self.min_pivot),
                None => lu.try_update_precomputed(r, a_tilde, &self.e_tilde, self.min_pivot),
            };
        self.check_after_update(lu, updated, density)
    }

    /// FT 更新の後の再分解トリガ (2)(4)(5)(3) ([`Self::update_and_check`] の説明の順)。
    #[inline]
    fn check_after_update(&mut self, lu: &sparse_lu::FtLu, updated: bool, density: SynthDensity) -> RefactorDue {
        if !updated {
            return RefactorDue::Rejected;
        }
        if lu.update_count() > self.max_updates {
            return RefactorDue::MaxUpdates;
        }
        if synth_clock_should_refactor_density(lu, density) {
            return RefactorDue::Clock;
        }
        if self.since_check >= self.check_interval {
            self.since_check = 0;
            return if self.fill_too_big(lu) { RefactorDue::Fill } else { RefactorDue::Periodic };
        }
        RefactorDue::No
    }

    /// 周期検査の数え上げを 0 に戻す (呼び出し側が自分の都合で検査をやり直すとき)。
    pub(super) fn reset_periodic(&mut self) {
        self.since_check = 0;
    }

    /// 周期検査の数え上げそのもの (同じ数え上げを使う旧経路 `run_phase` に渡す用)。
    pub(super) fn periodic_counter_mut(&mut self) -> &mut usize {
        &mut self.since_check
    }

    /// eta の fill が上限を超えているか (周期検査と、呼び出し側の任意の時点の検査用)。上限は
    /// `FT_BUMP_LIMIT_FACTOR * m`。ただし `bump_lu_ratio > 0` で分解自体がそれ以上に大きい (基底が密な)
    /// ときは `bump_lu_ratio * nnz(LU)` まで広げる (square41 報告の策2: FT 更新 1 回の eta が数千要素に
    /// なり、`64·m` だと数十反復ごとに再分解していた。Netlib (`nnz(LU)` は最大でも `~35·m`) には掛からない)。
    #[inline]
    pub(super) fn fill_too_big(&self, lu: &sparse_lu::FtLu) -> bool {
        let limit = if self.bump_lu_ratio > 0.0 && lu.lu_nnz() >= self.bump_limit {
            self.bump_limit.max((self.bump_lu_ratio * lu.lu_nnz() as f64) as usize)
        } else {
            self.bump_limit
        };
        lu.fill_count() > limit
    }
}

/// 現在の `basis_pos` から `B` を新たに LU 分解する(Forrest-Tomlin 更新ではない)。
/// 対角行列ならその専用分解、そうでなければ Markowitz 分解を行う。
///
/// `prev` は置き換える前の分解(あれば): そのピボット順を再利用する
/// (`sparse_lu::factorize_reusing`、HiGHS `HFactor::rebuild()` 相当)。再利用は各段で閾値
/// ピボットとフィルを再確認し、だめなら自動で完全な Markowitz 探索に戻るので、`prev` を渡しても
/// 分解が得られるかどうかは変わらず、手間だけが変わる。特異基底なら `None`。
/// 主単体法・双対単体法の再分解はすべてこれを通る (双対単体法は特異のときの印を
/// `slope_intercept_dual::refactorize` で付け足す)。
pub(super) fn factorize_basis(
    std: &StdForm,
    basis_pos: &[Option<usize>],
    prev: Option<&sparse_lu::FtLu>,
) -> Option<sparse_lu::FtLu> {
    let m = std.n_rows;
    if let Some(p) = prev {
        if env_str!("ENOMOTO_DEBUG_FT_FILL").is_some() {
            let (u, r) = p.u_r_nnz();
            eprintln!("FT_FILL m={m} updates={} lu_nnz_at_build={} u_nnz={u} r_nnz={r}", p.update_count(), p.lu_nnz_baseline());
        }
    }
    // 行リストはスレッドローカルに再利用する(再確保せずクリアするので各行の容量が残る)。
    // 中身と順序は新規作成と同一。
    thread_local! {
        static ROWS: std::cell::RefCell<Vec<Vec<(usize, f64)>>> = const { std::cell::RefCell::new(Vec::new()) };
        /// 基底位置 → 変数番号の作業配列(同じくスレッドローカルに再利用)。
        static BASIS_OF: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
    }
    let mut rows = ROWS.with(|r| std::mem::take(&mut *r.borrow_mut()));
    rows.truncate(m);
    for row in rows.iter_mut() {
        row.clear();
    }
    rows.resize_with(m, Vec::new);
    // 基底位置 → 変数番号の逆引き(基底位置を持つ変数はちょうど `m` 個)。
    let mut basis_of = BASIS_OF.with(|b| std::mem::take(&mut *b.borrow_mut()));
    basis_of.clear();
    basis_of.resize(m, usize::MAX);
    for j in 0..std.n_total {
        if let Some(col) = basis_pos[j] {
            basis_of[col] = j;
        }
    }
    // `std.cols` による列駆動の構築(`nnz(A_B)` の手間)。基底位置 `col` の昇順に訪れるので
    // 各行の要素は列番号順に並び、`KernelMatrix::new` の安定ソートが並べ替えなしで済む
    // (整列済み入力なので分解結果は変数番号順に積んだ場合と同一)。
    for (col, &j) in basis_of.iter().enumerate() {
        if j == usize::MAX {
            continue;
        }
        for &(i, v) in std.cols.col(j) {
            rows[i].push((col, v));
        }
    }
    BASIS_OF.with(|b| *b.borrow_mut() = basis_of);
    let r = sparse_lu::factorize_diagonal(m, &rows)
        .map(sparse_lu::FtLu::new)
        .or_else(|| sparse_lu::factorize_reusing(m, &rows, prev));
    ROWS.with(|r| *r.borrow_mut() = rows);
    r
}

/// トリガ (4) の閾値: FT 更新回数の上限 `max(FT_MAX_UPDATES_FACTOR * m, FT_MAX_UPDATES_FLOOR)`
/// (`m` に比例させる。[`FT_MAX_UPDATES_FACTOR`] 参照)。
#[inline]
pub(super) fn ft_max_updates(m: usize) -> usize {
    ((tunable!("ENOMOTO_T_FT_MAX_UPDATES_FACTOR", FT_MAX_UPDATES_FACTOR, f64) * m as f64) as usize).max(FT_MAX_UPDATES_FLOOR)
}

/// [`SYNTH_CLOCK_FACTOR`](`ENOMOTO_SYNTH_CLOCK_FACTOR` で上書き可、一度だけ解析してキャッシュ)。
/// 再較正のための調整つまみで、出荷時の挙動は [`SYNTH_CLOCK_FACTOR`] の値。
fn synth_clock_factor() -> f64 {
    static FACTOR: OnceLock<f64> = OnceLock::new();
    *FACTOR.get_or_init(|| {
        env_str!("ENOMOTO_SYNTH_CLOCK_FACTOR")
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|f| f.is_finite() && *f > 0.0)
            .unwrap_or(SYNTH_CLOCK_FACTOR)
    })
}

/// 合成クロックの係数を選ぶための求解結果の密度の区分 (square41 / ex10 報告の策11 と pds-100 報告の策5(b))。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SynthDensity {
    /// 求解結果が超疎 (stormG2_1000: DSE `tau` の非ゼロ率 1e-5 程度)。策10 の `sqrt(m / REF)` 倍をそのまま掛ける。
    Sparse,
    /// 中程度 (pds-100: `tau` の非ゼロ率 1〜5%)。基準行数を [`SYNTH_CLOCK_MID_REF_MULT`] 倍にして倍率を小さくする。
    Mid,
    /// 密 (ex10: 入る列の結果の 80% が非ゼロ)。倍率を掛けない。
    Dense,
}

/// トリガ (5): 合成クロックによる再分解判定。FT 更新回数が [`SYNTH_CLOCK_MIN_UPDATES`] 以上で、
/// 前回の分解以降に蓄積した求解側の tick が係数 `* build_tick` 以上なら真 (現在の分解に対する
/// FTRAN/BTRAN の手間が再分解の手間に達したとみなす。毎ピボット呼べるほど安価)。係数は求解結果の
/// 密度の区分 `density` で選ぶ ([`SynthDensity::Sparse`] が従来の判定)。策10 の `sqrt(m)` 倍は「求解の `O(m)`
/// パスを消したので実際の求解の手間は `m` によらない」ことが前提で、結果が密になるほど前提が崩れる:
/// 密なら掛けず (策11)、中程度なら基準行数を大きくして倍率を下げる (pds-100 は FT 更新が積もるにつれて
/// `R` 段と `tau` の密な反復が重くなり、2,270 反復ごとの再分解では遅すぎた)。
#[inline]
pub(super) fn synth_clock_should_refactor_density(lu: &sparse_lu::FtLu, density: SynthDensity) -> bool {
    let factor = match density {
        SynthDensity::Sparse => synth_clock_factor_for(lu.dim()),
        SynthDensity::Mid => synth_clock_factor_for_ref(lu.dim(), tunable!("ENOMOTO_T_SYNTH_CLOCK_MID_REF_MULT", SYNTH_CLOCK_MID_REF_MULT, f64)),
        SynthDensity::Dense => synth_clock_factor(),
    };
    lu.update_count() >= tunable!("ENOMOTO_T_SYNTH_CLOCK_MIN_UPDATES", SYNTH_CLOCK_MIN_UPDATES, usize) && (lu.synth_tick() as f64) >= factor * (lu.build_tick().max(1) as f64)
}

/// 行数 `m` の問題に使う合成クロックの係数: [`synth_clock_factor`]、ただし策10 で
/// `m >= SYNTH_CLOCK_LARGE_M` なら `sqrt(m / SYNTH_CLOCK_LARGE_REF_M)` 倍する。
#[inline]
fn synth_clock_factor_for(m: usize) -> f64 {
    synth_clock_factor_for_ref(m, 1.0)
}

/// [`synth_clock_factor_for`] の基準行数 `SYNTH_CLOCK_LARGE_REF_M` を `ref_mult` 倍したもの。
#[inline]
fn synth_clock_factor_for_ref(m: usize, ref_mult: f64) -> f64 {
    let large_m = tunable!("ENOMOTO_T_SYNTH_CLOCK_LARGE_M", SYNTH_CLOCK_LARGE_M, usize);
    if large_m > 0 && m >= large_m {
        let ref_m = tunable!("ENOMOTO_T_SYNTH_CLOCK_LARGE_REF_M", SYNTH_CLOCK_LARGE_REF_M, usize).max(1) as f64 * ref_mult;
        synth_clock_factor() * (m as f64 / ref_m).sqrt().max(1.0)
    } else {
        synth_clock_factor()
    }
}
