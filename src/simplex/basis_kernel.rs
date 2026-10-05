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
//! - [`BasisKernel::update`]: 記録した `a_tilde`/`e_tilde` で FT 更新する
//!   ([`sparse_lu::FtLu::try_update`] のように同じ求解をやり直さない)。
//! - [`BasisKernel::refactor_due`]: 再分解トリガ (更新回数の上限・合成クロック・eta の fill)。
//!
//! 双対単体法の主ループの融合 FTRAN (入る列・DSE の `tau`・BFRT の合成フリップ列を
//! 1 回の因子走査で解く pair/triple capture) と、大きな問題向けの疎経路 (`BIG`) は、
//! DSE・BFRT と結びついているので `slope_intercept_dual` 側に置く。

use super::sparse_lu;
use crate::params::simplex::{FT_BUMP_LIMIT_FACTOR, FT_CHECK_INTERVAL, FT_MIN_PIVOT};
use crate::params::slope_intercept_dual::{FT_MAX_UPDATES_FACTOR, FT_MAX_UPDATES_FLOOR, SYNTH_CLOCK_FACTOR, SYNTH_CLOCK_LARGE_M, SYNTH_CLOCK_LARGE_REF_M, SYNTH_CLOCK_MID_REF_MULT, SYNTH_CLOCK_MIN_UPDATES};
use std::sync::OnceLock;

/// [`BasisKernel::refactor_due`] の判定結果。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum RefactorDue {
    /// 再分解は不要。
    No,
    /// 再分解が必要 (更新回数の上限・合成クロック・eta の fill のいずれか)。
    Yes,
    /// 周期検査 ([`FT_CHECK_INTERVAL`] 反復ごと) の時点で、fill では再分解不要だった。
    /// 呼び出し側は必要なら自分の残差検査をここで行う。
    Periodic,
}

/// 1 回の求解ループが持つ、基底の求解の作業領域と再分解の方針 (モジュールの説明参照)。
///
/// 使い方 (1 反復): `ftran_col(lu, 入る列)` → `btran_row(lu, r)` → (基底の入れ替え) →
/// `update(lu, r)` が `false` か `refactor_due(lu) == Yes` なら再分解。`ftran_col` と
/// `btran_row` は同じ因子 (間に FT 更新や再分解を挟まない) で、`update` に渡す `r` は
/// `btran_row` と同じ行であること。順序は問わない。
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
    /// eta の fill の上限 (`FT_BUMP_LIMIT_FACTOR * m`)。
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
            bump_limit: tunable!("ENOMOTO_T_FT_BUMP_LIMIT_FACTOR", FT_BUMP_LIMIT_FACTOR, usize) * m.max(1),
            min_pivot: tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64),
            max_updates,
        }
    }

    /// 入る列 `col` (`(行, 値)` の疎な列) の FTRAN `out = B^-1 col` (`out` は長さ `m`、全体を書く)。
    /// 右辺が密か最近の結果が密なら密経路、それ以外は疎経路。FT 更新用に `a_tilde` を記録する。
    /// 結果の非ゼロ数を返す。
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
    pub(super) fn btran_row(&mut self, lu: &sparse_lu::FtLu, r: usize, out: &mut [f64]) {
        lu.solve_transpose_unit_work(r, out, &mut self.e_tilde, &mut self.btran_work, None);
        self.e_tilde_row = Some(r);
    }

    /// 基底位置 `r` の列を直前の [`Self::ftran_col`] の入る列で置き換える FT 更新を、記録した
    /// `a_tilde`/`e_tilde` で行う。新しいピボットが小さすぎる (または記録が揃っていない) なら
    /// 何も記録せず `false` を返す (呼び出し側は再分解すること)。記録はこの呼び出しで使い切る。
    pub(super) fn update(&mut self, lu: &mut sparse_lu::FtLu, r: usize) -> bool {
        let captured = self.a_tilde_ready && self.e_tilde_row == Some(r);
        debug_assert!(captured, "BasisKernel::update without ftran_col/btran_row for row {r}");
        self.a_tilde_ready = false;
        self.e_tilde_row = None;
        captured && lu.try_update_precomputed(r, &self.a_tilde, &self.e_tilde, self.min_pivot)
    }

    /// FT 更新の後に呼ぶ再分解トリガ: (4) 更新回数が `max_updates` ([`Self::new`]) を超えた、(5) 合成クロック
    /// ([`synth_clock_should_refactor`])、(3) [`FT_CHECK_INTERVAL`] 反復ごとの eta の fill が
    /// `FT_BUMP_LIMIT_FACTOR * m` を超えた。
    pub(super) fn refactor_due(&mut self, lu: &sparse_lu::FtLu) -> RefactorDue {
        self.since_check += 1;
        if lu.update_count() > self.max_updates || synth_clock_should_refactor(lu) {
            return RefactorDue::Yes;
        }
        if self.since_check >= FT_CHECK_INTERVAL {
            self.since_check = 0;
            return if lu.fill_count() > self.bump_limit { RefactorDue::Yes } else { RefactorDue::Periodic };
        }
        RefactorDue::No
    }

    /// eta の fill が上限を超えているか (呼び出し側の任意の時点の検査用)。
    pub(super) fn fill_too_big(&self, lu: &sparse_lu::FtLu) -> bool {
        lu.fill_count() > self.bump_limit
    }
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

/// トリガ (5): 合成クロックによる再分解判定。FT 更新回数が [`SYNTH_CLOCK_MIN_UPDATES`] 以上で、
/// 前回の分解以降に蓄積した求解側の tick が `synth_clock_factor() * build_tick` 以上なら真
/// (現在の分解に対する FTRAN/BTRAN の手間が再分解の手間に達したとみなす)。主ループと
/// 仕上げ (`polish_with_true_bounds`)、主単体法 ([`BasisKernel::refactor_due`]) で共有する(毎ピボット呼べるほど安価)。
#[inline]
pub(super) fn synth_clock_should_refactor(lu: &sparse_lu::FtLu) -> bool {
    synth_clock_should_refactor_density(lu, SynthDensity::Sparse)
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

/// [`synth_clock_should_refactor`] に求解結果の密度の区分を加えたもの。策10 の `sqrt(m)` 倍は「求解の `O(m)`
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
