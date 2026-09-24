//! SmallCoeff (微小係数の除去, Achterberg et al. "Presolve Reductions in Mixed Integer
//! Programming" 2020 §3.1)。
//!
//! 行 `i` の係数 `a_ik` の最悪寄与 `|a_ik| * (ub[k] - lb[k])` が実行可能性許容誤差に比べて十分小さければ、
//! その項を行から取り除き、`lb[k]` での値を右辺へ移す。行や変数は削除せず、個々の係数だけを消す。
//!
//! - 行ごとの累積予算: 列番号順に、寄与の累計が `CUMULATIVE_FRACTION * SMALLCOEFF_EPS` 以下に収まる限り除去する
//!   (論文の 2 段階の判定を包含する単一の判定)。
//! - 無条件除去: `|a_ik| <= NOISE_THRESHOLD` の係数は予算と無関係に除去する。
//!
//! 使われ方: 行列を書き換える [`remove_small_coefficients`] は **現在は未使用** (有効化すると不安定化したため。
//! 経緯は履歴メモ参照)。[`clean_row`] だけが `redundancy` のブロック分解の前処理で、
//! グラフの辺を省く判定として非破壊的に使われている。

use std::collections::BTreeMap;
use crate::params::presolve::{CUMULATIVE_FRACTION, NOISE_THRESHOLD, SMALLCOEFF_EPS};

/// 列の重複を解消済みの 1 行から微小係数を除去し、`(除去後の行, 調整後の右辺)` を返す。
///
/// 除去した項は `lb[k]` での値を右辺から引くので、`x_k = lb[k]` では厳密に同値、
/// それ以外でもずれはその項の最悪寄与以下 (行全体で予算以下) に収まる。
/// 項は格納順 (列番号昇順) に調べる。
///
/// - `row`: 行の `(列, 係数)`
/// - `rhs`: 行の右辺
/// - `lb`, `ub`: 変数の境界 (寄与の幅に使用)
pub(crate) fn clean_row(row: &[(usize, f64)], rhs: f64, lb: &[f64], ub: &[f64]) -> (Vec<(usize, f64)>, f64) {
    // 行全体で許される最悪寄与の合計
    let budget = CUMULATIVE_FRACTION * SMALLCOEFF_EPS;
    let mut new_row = Vec::with_capacity(row.len());
    let mut new_rhs = rhs;
    // これまでに除去した項の最悪寄与の累計
    let mut used_budget = 0.0;
    for &(k, v) in row {
        if v == 0.0 {
            continue;
        }
        if v.abs() <= NOISE_THRESHOLD {
            new_rhs -= v * lb[k];
            continue;
        }
        let contribution = v.abs() * (ub[k] - lb[k]);
        if used_budget + contribution <= budget {
            used_budget += contribution;
            new_rhs -= v * lb[k];
            continue;
        }
        new_row.push((k, v));
    }
    (new_row, new_rhs)
}

/// 疎な制約系の全行に [`clean_row`] を適用する。先に同じ列の重複要素を合算し、
/// 列ごとの真の係数で判定する。**現在は未使用** (パイプラインから呼ばれていない)。
///
/// 戻り値は `(除去後の各行, 調整後の各右辺)`。
#[allow(dead_code)]
pub fn remove_small_coefficients(rows: &[Vec<(usize, f64)>], rhs: &[f64], lb: &[f64], ub: &[f64]) -> (Vec<Vec<(usize, f64)>>, Vec<f64>) {
    let mut new_rows = Vec::with_capacity(rows.len());
    let mut new_rhs = Vec::with_capacity(rhs.len());
    for (row, &b) in rows.iter().zip(rhs) {
        // 列ごとに係数を合算した行
        let mut merged: BTreeMap<usize, f64> = BTreeMap::new();
        for &(k, v) in row {
            *merged.entry(k).or_insert(0.0) += v;
        }
        let merged_row: Vec<(usize, f64)> = merged.into_iter().collect();
        let (cleaned, new_b) = clean_row(&merged_row, b, lb, ub);
        new_rows.push(cleaned);
        new_rhs.push(new_b);
    }
    (new_rows, new_rhs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_a_coefficient_whose_contribution_is_negligible_via_a_tiny_bound_width() {
        // Row: 5*x0 + 1e-4*x1 = 10. x1's own coefficient (1e-4) is well
        // above NOISE_THRESHOLD on its own, but its bound width is tiny
        // (2.0 to 2.0+1e-6), so its worst-case contribution
        // (1e-4 * 1e-6 = 1e-10) is negligible against the cumulative
        // budget (CUMULATIVE_FRACTION * SMALLCOEFF_EPS = 1e-8) -- exercising the
        // contribution-based branch specifically, not the raw-noise one.
        let row = vec![(0, 5.0), (1, 1e-4)];
        let lb = vec![0.0, 2.0];
        let ub = vec![10.0, 2.0 + 1e-6];
        let (rows, rhs) = remove_small_coefficients(&[row], &[10.0], &lb, &ub);
        assert_eq!(rows[0], vec![(0, 5.0)]);
        assert!((rhs[0] - (10.0 - 1e-4 * 2.0)).abs() < 1e-12, "rhs={}", rhs[0]);
    }

    #[test]
    fn keeps_a_genuinely_significant_coefficient() {
        let row = vec![(0, 5.0), (1, 3.0)];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 10.0]; // contribution 3.0*10.0 = 30, far over any reasonable budget
        let (rows, rhs) = remove_small_coefficients(&[row], &[10.0], &lb, &ub);
        assert_eq!(rows[0], vec![(0, 5.0), (1, 3.0)]);
        assert_eq!(rhs[0], 10.0);
    }

    #[test]
    fn drops_a_raw_noise_coefficient_regardless_of_bound_width() {
        // Coefficient itself is below NOISE_THRESHOLD even though the
        // variable's own bound width is huge (so the contribution-based
        // budget test alone would *not* have caught this one).
        let row = vec![(0, 5.0), (1, 1e-12)];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 1e9];
        let (rows, rhs) = remove_small_coefficients(&[row], &[10.0], &lb, &ub);
        assert_eq!(rows[0], vec![(0, 5.0)]);
        assert_eq!(rhs[0], 10.0); // lb[1] = 0, so no rhs adjustment needed
    }

    #[test]
    fn respects_the_cumulative_budget_across_several_small_entries_in_one_row() {
        // Three equal-sized small contributions where two fit the row's
        // total budget but a third would exceed it -- only the two
        // encountered first (ascending column order) should be dropped.
        let budget = CUMULATIVE_FRACTION * SMALLCOEFF_EPS;
        let per_term = budget / 2.5; // 2 terms fit (0.8*budget), 3 don't (1.2*budget)
        let row = vec![(0, 100.0), (1, per_term), (2, per_term), (3, per_term)];
        let lb = vec![0.0, 0.0, 0.0, 0.0];
        let ub = vec![1.0, 1.0, 1.0, 1.0]; // width 1 => contribution == coefficient
        let (rows, _rhs) = remove_small_coefficients(&[row], &[0.0], &lb, &ub);
        let kept: Vec<usize> = rows[0].iter().map(|&(k, _)| k).collect();
        // Column 0's own contribution (100) alone busts the budget, so it
        // is always kept; columns 1 and 2 fit the running budget and are
        // dropped; column 3 arrives after the budget is already spent.
        assert_eq!(kept, vec![0, 3], "kept={kept:?}");
    }

    #[test]
    fn merges_duplicate_column_entries_before_testing_them() {
        // Two raw entries for column 1 that individually look negligible
        // but sum to something significant must not be dropped.
        let row = vec![(0, 5.0), (1, 4.9), (1, 4.9)]; // merges to (1, 9.8)
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 10.0];
        let (rows, _rhs) = remove_small_coefficients(&[row], &[10.0], &lb, &ub);
        assert_eq!(rows[0], vec![(0, 5.0), (1, 9.8)]);
    }
}
