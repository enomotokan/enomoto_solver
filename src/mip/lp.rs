//! 分枝切除法の LP ([`super::lp_api::MipLp`]、実装は [`crate::simplex::mip_lp::TwoStageLp`]) とやり取りする型。
//!
//! 変数は構造変数 `x_j` (`j < n`) と、各行 `i` の活動量を表す論理変数 `r_i = a_i x` (変数番号 `n + i`)。
//! 行の上下限は論理変数の境界として持つので、`<=`・`>=`・`=`・範囲制約をすべて同じ形で扱える。

use std::time::Instant;

/// 変数 (構造変数または論理変数) の基底状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarStatus {
    /// 基底変数。
    Basic,
    /// 非基底で下限にある。
    Lower,
    /// 非基底で上限にある。
    Upper,
    /// 非基底の自由変数 (値 0)。
    Zero,
}

/// 基底 (構造変数と行ごとの状態)。[`MipLp::basis`](super::lp_api::MipLp::basis) で取得し、[`MipLp::set_basis`](super::lp_api::MipLp::set_basis) で戻す。
#[derive(Debug, Clone, PartialEq)]
pub struct Basis {
    /// 構造変数の状態 (長さ `n`)。
    pub col: Vec<VarStatus>,
    /// 各行の論理変数の状態 (長さ `m`)。
    pub row: Vec<VarStatus>,
}

/// 1 回の [`MipLp::solve`](super::lp_api::MipLp::solve) の結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LpStatus {
    /// 最適解に達した。
    Optimal,
    /// 主実行不能 (双対射線を [`MipLp::farkas_ray`](super::lp_api::MipLp::farkas_ray) で取得できる)。
    Infeasible,
    /// 主の非有界 (双対実行不能)。
    Unbounded,
    /// 目的値の下界が打ち切り値 ([`SolveLimits::cutoff`]) を超えた。
    ObjectiveBound,
    /// 反復上限に達した。
    IterationLimit,
    /// 時間上限に達した。
    TimeLimit,
    /// 数値的な理由で解けなかった。
    Error,
}

/// [`MipLp::solve`](super::lp_api::MipLp::solve) の打ち切り条件。
#[derive(Debug, Clone, Copy)]
pub struct SolveLimits {
    /// 反復回数の上限。
    pub iteration_limit: u64,
    /// 目的値 (元のスケール、定数項なし) の下界がこれを超えたら [`LpStatus::ObjectiveBound`] で止める。
    pub cutoff: f64,
    /// 時刻の上限。
    pub deadline: Option<Instant>,
}

impl Default for SolveLimits {
    fn default() -> Self {
        SolveLimits { iteration_limit: u64::MAX, cutoff: f64::INFINITY, deadline: None }
    }
}
