//! FoldFixed (固定列の畳み込み): 既に固定された列 (`lb[j] == ub[j]`) の項を各行から取り除き、
//! `coeff * value` を右辺へ移す。
//!
//! `dualfix`・[`super::dualpropagate`] の列固定・`rowsingleton` は境界を固定するだけで
//! `A`/`G` の行を書き換えないため、固定済み変数の項が行に残り、`rowsingleton`・`doubleton`・
//! `aggregator` から見た「生きている項の数」が実際より多くなる。本モジュールはその後始末を行う。
//! (`doubleton`/`colsingleton`/`aggregator` は自分で行を書き換えるのでこの残骸を残さない。)
//!
//! 外側ラウンドごとに 1 回、固定処理の直後に呼ばれる。同ラウンド内で新たに固定された列は
//! 次ラウンドで処理される。

use crate::types::RowSense;
use crate::params::presolve::TOL;

/// [`fold_fixed_columns`] の結果。
pub struct FoldFixedResult {
    /// 固定列の項を除いた後の行 (生きている項が 0 になった行は削除済み)
    pub rows: Vec<Vec<(usize, f64)>>,
    /// `rows` に対応する、定数を畳み込んだ後の右辺
    pub rhs: Vec<f64>,
    /// 項がすべて消えた行が矛盾し、実行不能と判明したか (true のとき他フィールドは空)
    pub infeasible: bool,
}

/// `rows` の各行から固定列 (`lb[j] == ub[j]`) の項を取り除き、`coeff * lb[j]` を右辺から引く。
///
/// `A` の等式行にも `G` の実不等式行にも同じように使える。`sense` は項がすべて消えた行の
/// 判定にのみ使う:
/// - [`RowSense::Eq`]: 畳み込み後の右辺が `TOL` 以内で 0 であること
/// - [`RowSense::Le`]: 畳み込み後の右辺が `-TOL` 以上 (余裕が非負) であること
/// - [`RowSense::Ge`]: 実際には渡されない (`G` は常に `<=` 正規化済み) が、対称に扱う
///
/// 判定を通った空行は削除し、通らなければ実行不能を返す。明示的な 0 係数は右辺に触れず捨てる。
pub fn fold_fixed_columns(rows: &[Vec<(usize, f64)>], rhs: &[f64], lb: &[f64], ub: &[f64], sense: RowSense) -> FoldFixedResult {
    let mut new_rows = Vec::with_capacity(rows.len());
    let mut new_rhs = Vec::with_capacity(rhs.len());
    for (row, &r) in rows.iter().zip(rhs.iter()) {
        // 固定されていない (生きている) 項
        let mut live = Vec::with_capacity(row.len());
        // 固定列の寄与を差し引いた右辺
        let mut folded = r;
        for &(j, v) in row {
            if v == 0.0 {
                continue;
            }
            if lb[j] == ub[j] {
                folded -= v * lb[j];
            } else {
                live.push((j, v));
            }
        }
        if live.is_empty() {
            let ok = match sense {
                RowSense::Eq => folded.abs() <= TOL,
                RowSense::Le => folded >= -TOL,
                RowSense::Ge => folded <= TOL,
            };
            if !ok {
                return FoldFixedResult { rows: Vec::new(), rhs: Vec::new(), infeasible: true };
            }
            continue;
        }
        new_rows.push(live);
        new_rhs.push(folded);
    }
    FoldFixedResult { rows: new_rows, rhs: new_rhs, infeasible: false }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_fixed_terms_and_folds_their_value_into_rhs() {
        // x0 + 2*x1 + x2 = 10, x1 fixed at 3 => x0 + x2 = 4.
        let rows = vec![vec![(0, 1.0), (1, 2.0), (2, 1.0)]];
        let rhs = vec![10.0];
        let lb = vec![0.0, 3.0, 0.0];
        let ub = vec![10.0, 3.0, 10.0];
        let result = fold_fixed_columns(&rows, &rhs, &lb, &ub, RowSense::Eq);
        assert!(!result.infeasible);
        assert_eq!(result.rows, vec![vec![(0, 1.0), (2, 1.0)]]);
        assert_eq!(result.rhs, vec![4.0]);
    }

    #[test]
    fn leaves_a_row_with_no_fixed_columns_untouched() {
        let rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let rhs = vec![5.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 10.0];
        let result = fold_fixed_columns(&rows, &rhs, &lb, &ub, RowSense::Eq);
        assert!(!result.infeasible);
        assert_eq!(result.rows, rows);
        assert_eq!(result.rhs, rhs);
    }

    #[test]
    fn a_row_reduced_to_zero_live_terms_vanishes_when_consistent() {
        // x0 + x1 = 5, both fixed at 2 and 3 => 0 = 0, row drops entirely.
        let rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let rhs = vec![5.0];
        let lb = vec![2.0, 3.0];
        let ub = vec![2.0, 3.0];
        let result = fold_fixed_columns(&rows, &rhs, &lb, &ub, RowSense::Eq);
        assert!(!result.infeasible);
        assert!(result.rows.is_empty());
    }

    #[test]
    fn a_row_reduced_to_zero_live_terms_is_infeasible_when_inconsistent() {
        let rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let rhs = vec![5.0];
        let lb = vec![2.0, 2.0];
        let ub = vec![2.0, 2.0];
        let result = fold_fixed_columns(&rows, &rhs, &lb, &ub, RowSense::Eq);
        assert!(result.infeasible);
    }

    #[test]
    fn an_inequality_row_reduced_to_zero_live_terms_only_needs_nonnegative_slack() {
        // x0 <= 5, x0 fixed at 3 => 0 <= 2, satisfied, row drops.
        let rows = vec![vec![(0, 1.0)]];
        let rhs = vec![5.0];
        let lb = vec![3.0];
        let ub = vec![3.0];
        let result = fold_fixed_columns(&rows, &rhs, &lb, &ub, RowSense::Le);
        assert!(!result.infeasible);
        assert!(result.rows.is_empty());

        // x0 <= 5, x0 fixed at 7 => 0 <= -2, violated.
        let lb2 = vec![7.0];
        let ub2 = vec![7.0];
        let result2 = fold_fixed_columns(&rows, &rhs, &lb2, &ub2, RowSense::Le);
        assert!(result2.infeasible);
    }

    #[test]
    fn explicit_zero_coefficients_are_dropped_without_touching_rhs() {
        let rows = vec![vec![(0, 0.0), (1, 1.0)]];
        let rhs = vec![5.0];
        let lb = vec![100.0, 0.0]; // x0's own bound is irrelevant: its coefficient here is 0
        let ub = vec![100.0, 10.0];
        let result = fold_fixed_columns(&rows, &rhs, &lb, &ub, RowSense::Eq);
        assert!(!result.infeasible);
        assert_eq!(result.rows, vec![vec![(1, 1.0)]]);
        assert_eq!(result.rhs, vec![5.0]);
    }
}
