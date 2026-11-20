//! 全体では速いが一部で大きく悪化した試作を、問題の形の条件付きで入れ直す試作 (CONDREV、analysis/condrev_20261118、既定では使わない)。
//!
//! 外側の求解の始め ([`decide`]、前処理の前の問題) に 1 回だけ決め、外側の求解の全スレッド (サブ MIP を含む) で共通に使う。
//! 条件の環境変数を付けなければ何もしない (試作の環境変数の効きはそのまま)。
//! - `ENOMOTO_MIP_SBT_GENINT_MIN=k`: 一般整数の列 (整数で上下限の幅が 1 を越える列) が k 本未満の問題では、sbt の束
//!   (`ENOMOTO_MIP_SB_TIGHTEN`・`_INHERIT`・`_ANCESTOR`・`ENOMOTO_MIP_SB_POLY_RC`) を付けていても切る。
//!   根拠: 一般整数 16 問では 3 回とも −4〜−6% (analysis/cbcimpl_20261029.md 6 段目)、害の neos-911970 は一般整数の列が少ない。
//! - `ENOMOTO_MIP_SUBFIX_OFF_NINT=k`: 整数列が k 本を越える問題では、`ENOMOTO_MIP_SUBFIX` を `0` とみなす (既定の RENS・RINS に戻す)。
//!   根拠: neos-873061 (整数列 8 万本超) で SUBFIX の根の RENS のサブ MIP が 1 回 421 秒 (analysis/prune_20261112.md §3)。
//!   ただし害の neos-960392 (整数列 5.9 万本) も大きいので、整数列の数では分けられない (analysis/condrev_20261118/README.md)。
//! - `ENOMOTO_T_MIP_SUBFIX_CAP=f`: 外側の根の SUBFIX のサブ MIP (RENS・RINS のダイブの後) の時間の上限を、時間の上限の f 倍にする
//!   (SUBFIX は止めず、長いサブ MIP だけを打ち切る。neos-960392 の根の RENS は 0.05 秒で最適解を出すので残る)。

use super::problem::MipProblem;
use std::sync::atomic::{AtomicBool, Ordering};

/// sbt の束を切るか (外側の求解ごとに [`decide`] で決める)
static SBT_OFF: AtomicBool = AtomicBool::new(false);
/// SUBFIX を切るか (同上)
static SUBFIX_OFF: AtomicBool = AtomicBool::new(false);

/// sbt の束をこの問題で切るか
#[inline]
pub(crate) fn sbt_off() -> bool {
    SBT_OFF.load(Ordering::Relaxed)
}

/// SUBFIX をこの問題で切るか
#[inline]
pub(crate) fn subfix_off() -> bool {
    SUBFIX_OFF.load(Ordering::Relaxed)
}

/// 外側の求解の始めに、前処理の前の問題から条件を決める
pub(crate) fn decide(p: &MipProblem, verbose: bool) {
    let n_int = p.is_int.iter().filter(|&&b| b).count();
    let n_gen = (0..p.n).filter(|&j| p.is_int[j] && p.col_up[j] - p.col_lo[j] > 1.5).count();
    let sbt = env_str!("ENOMOTO_MIP_SBT_GENINT_MIN").and_then(|v| v.parse::<usize>().ok()).is_some_and(|k| n_gen < k);
    let subfix = env_str!("ENOMOTO_MIP_SUBFIX_OFF_NINT").and_then(|v| v.parse::<usize>().ok()).is_some_and(|k| n_int > k);
    SBT_OFF.store(sbt, Ordering::Relaxed);
    SUBFIX_OFF.store(subfix, Ordering::Relaxed);
    if verbose && (env_str!("ENOMOTO_MIP_SBT_GENINT_MIN").is_some() || env_str!("ENOMOTO_MIP_SUBFIX_OFF_NINT").is_some()) {
        eprintln!("MIP: CONDREV: integer columns {n_int}, general integer columns {n_gen}; sbt {}, SUBFIX {}",
                  if sbt { "off" } else { "as set" }, if subfix { "off" } else { "as set" });
    }
}

/// `ENOMOTO_T_MIP_SUBFIX_CAP` (付けていなければ `None`): 外側の根の SUBFIX のサブ MIP の時間の上限 (秒)。`tl` は時間の上限
pub(crate) fn subfix_cap(tl: f64) -> Option<f64> {
    static F: std::sync::OnceLock<Option<f64>> = std::sync::OnceLock::new();
    let f = (*F.get_or_init(|| env_str!("ENOMOTO_T_MIP_SUBFIX_CAP").and_then(|v| v.parse::<f64>().ok())))?;
    tl.is_finite().then(|| (f * tl).max(tunable!("ENOMOTO_T_MIP_SUBFIX_CAP_MIN", 0.5, f64)))
}
