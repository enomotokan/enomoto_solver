//! DominatedColumns (支配列の固定, Andersen & Andersen 1995 §3.1)。**現在は未使用**
//! (パイプラインから呼ばれていない。経緯は履歴メモ参照)。
//!
//! [`dualfix`](super::dualfix) のロック数 0 の場合 (暗黙の全零列との比較) を、任意の他の列との比較に一般化したもの。
//!
//! 等式行に現れない 2 変数 `j`, `k` について、`c[j] <= c[k]` かつすべての実不等式行 `i` で
//! `G[i,j] <= G[i,k]` (`TOL` 込み) なら `j` は `k` を支配する。このとき `k` から `j` へ量を移しても
//! 行の活動値も目的値も増えないので、移動が境界で止まる端点のどちらかに最適解がある:
//!
//! - `j` の上限が無限なら、`k` をその (有限の) 下限に固定する
//! - `k` の下限が無限なら、`j` をその (有限の) 上限に固定する
//!
//! 両方の逃げ道が開いていれば本当の非有界性なので何もしない。どちらも開いていなければ部分的な境界強化しかできず、
//! それはこのパスでは扱わない。逃げ道には文字通りの無限大が必要 (有限の広い範囲では不健全)。
//!
//! **候補探索**: 全列対 `O(n^2)` を避け、各列 `j` について最も短い行 (anchor) を共有する列とだけ比較する。
//!
//! **1 パス・非連鎖**: 1 回の固定に使った 2 列 (固定される側と根拠となる側) は、この呼び出しでは再利用しない。

use crate::sparse::{Csr, csr_row_iter};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use crate::params::presolve::TOL;

/// 固定できる変数の `(j, 値)` の一覧を返す
/// ([`dualfix::fix_dominated_variables`](super::dualfix::fix_dominated_variables) と同じ形式)。
///
/// - `n`: 列数
/// - `a`: 等式行列 (ここに現れる列は対象外)
/// - `real_g_rows`: `G` の実制約 (複数変数) 行
/// - `c`: 目的関数係数
/// - `lb`, `ub`: 変数の境界
pub fn fix_dominated_columns(n: usize, a: &Csr, real_g_rows: &[Vec<(usize, f64)>], c: &[f64], lb: &[f64], ub: &[f64]) -> Vec<(usize, f64)> {
    // in_equality[j]: 列 j が等式行に現れるか
    let mut in_equality = vec![false; n];
    let ar = a.as_ref();
    for i in 0..ar.nrows() {
        for (j, v) in csr_row_iter(a, i) {
            if v != 0.0 {
                in_equality[j] = true;
            }
        }
    }

    // col_rows[j]: 等式行に現れない列 j の、実制約行での (行番号 -> 係数)。箱境界行は含めない。
    let mut col_rows: Vec<BTreeMap<usize, f64>> = vec![BTreeMap::new(); n];
    for (i, row) in real_g_rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 && !in_equality[j] {
                col_rows[j].insert(i, v);
            }
        }
    }

    // 比較する列対 (小さい番号, 大きい番号)。どの固定が `used` 判定を通るかが順序に依存するので、決定的な BTreeSet を使う。
    // 各列 j について、最も短い行 anchor を共有する列とだけ組にする。
    let mut candidates: BTreeSet<(usize, usize)> = BTreeSet::new();
    for j in 0..n {
        if in_equality[j] || col_rows[j].is_empty() {
            continue;
        }
        let anchor = *col_rows[j].keys().min_by_key(|&&i| real_g_rows[i].len()).unwrap();
        for &(k, _) in &real_g_rows[anchor] {
            if k != j && !in_equality[k] {
                candidates.insert((j.min(k), j.max(k)));
            }
        }
    }

    // used: この呼び出しで既に固定に使われた列
    let mut used: HashSet<usize> = HashSet::new();
    let mut fixed: Vec<(usize, f64)> = Vec::new();
    for (p, q) in candidates {
        if used.contains(&p) || used.contains(&q) {
            continue;
        }
        let outcome = try_fix_pair(p, q, &col_rows, c, lb, ub).or_else(|| try_fix_pair(q, p, &col_rows, c, lb, ub));
        if let Some((var, value)) = outcome {
            used.insert(p);
            used.insert(q);
            fixed.push((var, value));
        }
    }
    fixed
}

/// `j` が `k` を支配するかを調べ、支配し逃げ道が開いていれば固定 `(変数, 値)` を返す。
/// 点ごとの比較が成り立たないか、逃げ道がどちらも開いていなければ `None`。
fn try_fix_pair(j: usize, k: usize, col_rows: &[BTreeMap<usize, f64>], c: &[f64], lb: &[f64], ub: &[f64]) -> Option<(usize, f64)> {
    if c[j] > c[k] + TOL {
        return None;
    }
    for (&i, &vk) in &col_rows[k] {
        let vj = col_rows[j].get(&i).copied().unwrap_or(0.0);
        if vj > vk + TOL {
            return None;
        }
    }
    for (&i, &vj) in &col_rows[j] {
        if col_rows[k].contains_key(&i) {
            continue; // 上のループで判定済み
        }
        // この行での k の係数は暗黙に 0。
        if vj > TOL {
            return None;
        }
    }
    if ub[j] == f64::INFINITY && lb[k].is_finite() {
        Some((k, lb[k]))
    } else if lb[k] == f64::NEG_INFINITY && ub[j].is_finite() {
        Some((j, ub[j]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    #[test]
    fn fixes_the_dominated_column_to_its_lower_bound() {
        // Row: x0 + 2*x1 <= 10, so G[.,0]=1 <= G[.,1]=2 pointwise and
        // c0 <= c1 -- x0 dominates x1. x0 has no finite upper bound, x1
        // has a finite lower bound, so x1 should be fixed there.
        let a = csr_from_rows(&[], 2);
        let real_g_rows = vec![vec![(0, 1.0), (1, 2.0)]];
        let c = vec![1.0, 1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY];

        let fixed = fix_dominated_columns(2, &a, &real_g_rows, &c, &lb, &ub);
        assert_eq!(fixed, vec![(1, 0.0)]);
    }

    #[test]
    fn fixes_the_dominating_column_to_its_upper_bound_when_the_dominated_side_is_unbounded_below() {
        // Same pointwise relation (x0 dominates x1), but now x1 has no
        // finite lower bound while x0 has a finite upper bound -- the
        // shift runs the other way, pinning x0 at its own ceiling instead.
        let a = csr_from_rows(&[], 2);
        let real_g_rows = vec![vec![(0, 1.0), (1, 2.0)]];
        let c = vec![1.0, 1.0];
        let lb = vec![0.0, f64::NEG_INFINITY];
        let ub = vec![5.0, f64::INFINITY];

        let fixed = fix_dominated_columns(2, &a, &real_g_rows, &c, &lb, &ub);
        assert_eq!(fixed, vec![(0, 5.0)]);
    }

    #[test]
    fn does_nothing_when_neither_escape_route_is_open() {
        // Same pointwise relation, but both variables are finitely bounded
        // on the side that would need to be infinite -- no fix available,
        // only a partial tightening this pass intentionally leaves alone.
        let a = csr_from_rows(&[], 2);
        let real_g_rows = vec![vec![(0, 1.0), (1, 2.0)]];
        let c = vec![1.0, 1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![5.0, 5.0];

        let fixed = fix_dominated_columns(2, &a, &real_g_rows, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    #[test]
    fn leaves_a_non_dominated_pair_untouched() {
        // The two rows disagree about which column has the smaller
        // coefficient (row0: 1 vs 2; row1: 2 vs 1), so neither pointwise
        // comparison holds in either direction across the whole column.
        let a = csr_from_rows(&[], 2);
        let real_g_rows = vec![vec![(0, 1.0), (1, 2.0)], vec![(0, 2.0), (1, 1.0)]];
        let c = vec![1.0, 1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY];

        let fixed = fix_dominated_columns(2, &a, &real_g_rows, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    #[test]
    fn excludes_variables_appearing_in_any_equality_row() {
        // Same dominance relation as the first test, but x1 also appears
        // in an equality row -- disqualified, same as `dualfix`.
        let a = csr_from_rows(&[vec![(1, 1.0)]], 2);
        let real_g_rows = vec![vec![(0, 1.0), (1, 2.0)]];
        let c = vec![1.0, 1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY];

        let fixed = fix_dominated_columns(2, &a, &real_g_rows, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    #[test]
    fn never_reuses_a_committed_column_as_either_side_of_a_second_fix() {
        // x0 dominates both x1 and x2 on one shared row (coefficient 1 vs
        // 2 for each, equal cost). Only one of the two resulting fixes
        // should survive this single pass -- the other is left for the
        // next round, since committing both would pin x0 twice over.
        let a = csr_from_rows(&[], 3);
        let real_g_rows = vec![vec![(0, 1.0), (1, 2.0), (2, 2.0)]];
        let c = vec![1.0, 1.0, 1.0];
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY, f64::INFINITY];

        let fixed = fix_dominated_columns(3, &a, &real_g_rows, &c, &lb, &ub);
        assert_eq!(fixed.len(), 1, "exactly one of the two competing fixes should be committed this pass");
        assert!(fixed[0].0 == 1 || fixed[0].0 == 2);
        assert_eq!(fixed[0].1, 0.0);
    }
}
