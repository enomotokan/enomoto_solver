//! Sparsify (疎化, HiGHS/PaPILO の "Sparsify")。**現在は未使用** (パイプラインから呼ばれていない。理由は履歴メモ参照)。
//!
//! 行 `r` (`A` の等式行、または `G` の複数変数の実不等式行) を、台 (非零の列集合) が `r` の台に含まれる
//! 別の等式行 `eq` を使って `r - scale * eq` に書き換える。`eq` の変数はすべて `r` に既にあるので
//! フィルインは生じず、少なくとも 1 つの非零 (消去変数) が減る。フィルインを許す一般版は扱わない保守的な半分。
//!
//! `eq` 自体は変わらず成り立つので、書き換え後も実行可能領域は同じ (可逆な行演算。行の向きも不変)。
//! 変数や行は削除せず行を疎にするだけだが、それにより後のパスで他の縮約が可能になりうる。
//!
//! 候補探索: 各等式行 `eq` (`|S_eq| >= 2`) について、出現行数が最も少ない変数 (`anchor`) を含む行だけを
//! 調べ、`S_eq ⊆ S_r` を直接確認する。消去する変数は `eq` の中で絶対値最大の係数を持つもの (数値安定性のため)。
//!
//! **この呼び出しで書き換えられた行は、後でピボット (`eq`) として使わない** (正しさのために必須。
//! 相互参照で系が壊れた事例は履歴メモ参照)。各対象行は 1 呼び出しにつき高々 1 回だけ書き換える。

use crate::presolve::propagate;
use crate::sparse::{FaerCsr, SparseAccum, csr_from_rows, csr_rows_pruned};
use crate::params::presolve::TOL;

/// [`sparsify`] の結果。
pub struct SparsifyResult {
    /// 書き換え後の等式行列
    pub a: FaerCsr,
    /// `a` に対応する右辺
    pub b: Vec<f64>,
    /// 書き換え後の不等式行列 (箱境界行を含めて再構築したもの)
    pub g: FaerCsr,
    /// `g` に対応する右辺
    pub h: Vec<f64>,
    /// この呼び出しで書き換えた行数 (`A` と `G` の合計)。0 なら何も見つからなかった
    pub n_rows_changed: usize,
}

/// 対象行がどちらの行列の行か。
#[derive(Clone, Copy, PartialEq, Eq)]
enum RowSource {
    /// 等式行列 `A` の行
    A,
    /// `G` の実制約 (複数変数) 行
    G,
}

/// 等式行による部分集合型の疎化を 1 パス行う。
///
/// - `n`: 列数
/// - `a`, `b`: 等式制約 `A x = b`
/// - `g`, `h`: 不等式制約 `G x <= h` (内部で箱境界行と実制約行に分離し、最後に再構築する)
pub fn sparsify(n: usize, a: &FaerCsr, b: &[f64], g: &FaerCsr, h: &[f64]) -> SparsifyResult {
    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows_pruned(a);
    let mut b: Vec<f64> = b.to_vec();

    let (lb, ub, mut real_g_rows, mut real_g_rhs) = propagate::extract_bounds(n, g, h);

    // col_to_rows[j]: 列 j を含む対象候補行 (行列の種類, 行番号) の一覧。書き換え前の状態から一度だけ作る。
    // 箱境界行はピボットにも対象にもしない (既に最小なので)。
    let mut col_to_rows: Vec<Vec<(RowSource, usize)>> = vec![Vec::new(); n];
    for (i, row) in a_rows.iter().enumerate() {
        for &(j, _) in row {
            col_to_rows[j].push((RowSource::A, i));
        }
    }
    for (i, row) in real_g_rows.iter().enumerate() {
        for &(j, _) in row {
            col_to_rows[j].push((RowSource::G, i));
        }
    }

    // changed_a / changed_g: その行がこの呼び出しで既に書き換えられたか
    let mut changed_a = vec![false; a_rows.len()];
    let mut changed_g = vec![false; real_g_rows.len()];
    // 全対象行の書き換えで共用する疎アキュムレータ
    let mut accum = SparseAccum::new(n);
    let mut n_rows_changed = 0usize;

    for eq_idx in 0..a_rows.len() {
        // 既に書き換えられた行はピボットにしない (正しさのため)。したがってここで読む行と
        // `b[eq_idx]` は、この呼び出しで未変更の元の形である。
        if changed_a[eq_idx] {
            continue;
        }
        let eq_row = a_rows[eq_idx].clone();
        if eq_row.len() < 2 {
            continue;
        }
        // anchor: eq の変数のうち出現行数が最小のもの (候補行の絞り込み用)
        let anchor = eq_row.iter().map(|&(j, _)| j).min_by_key(|&j| col_to_rows[j].len()).unwrap();
        // 消去する変数: eq の中で絶対値最大の係数を持つもの。
        let (elim_var, elim_coeff) = *eq_row.iter().max_by(|x, y| x.1.abs().total_cmp(&y.1.abs())).unwrap();
        if elim_coeff.abs() < TOL {
            continue;
        }
        let eq_rhs = b[eq_idx];

        for &(src, idx) in &col_to_rows[anchor] {
            if src == RowSource::A && idx == eq_idx {
                continue;
            }
            let already_changed = match src {
                RowSource::A => changed_a[idx],
                RowSource::G => changed_g[idx],
            };
            if already_changed {
                continue;
            }
            let target_row = match src {
                RowSource::A => &a_rows[idx],
                RowSource::G => &real_g_rows[idx],
            };
            if target_row.len() < eq_row.len() {
                // eq より短い行は台を包含しえないので、部分集合判定の前に除外する。
                continue;
            }
            // 対象行をアキュムレータに読み込み、S_eq ⊆ S_r を O(1) の `contains` で判定する。
            accum.load(target_row);
            if !eq_row.iter().all(|&(k, _)| accum.contains(k)) {
                continue;
            }
            debug_assert!(accum.contains(elim_var), "elim_var is in eq_row's support, which the subset check just verified the target covers");
            // 対象行での消去変数の係数と、それを 0 にする倍率
            let target_elim_coeff = accum.get(elim_var);
            let scale = target_elim_coeff / elim_coeff;
            if scale == 0.0 {
                continue;
            }
            accum.axpy(-scale, &eq_row);
            // 厳密に打ち消されるのは elim_var だけなので、それだけを取り除く。
            // 他の要素は小さくても正しい新係数なので、許容誤差で捨ててはならない。
            accum.remove(elim_var);
            let new_rhs = match src {
                RowSource::A => b[idx] - scale * eq_rhs,
                RowSource::G => real_g_rhs[idx] - scale * eq_rhs,
            };
            let new_row_vec: Vec<(usize, f64)> = accum.take_sorted(0.0);
            match src {
                RowSource::A => {
                    a_rows[idx] = new_row_vec;
                    b[idx] = new_rhs;
                    changed_a[idx] = true;
                }
                RowSource::G => {
                    real_g_rows[idx] = new_row_vec;
                    real_g_rhs[idx] = new_rhs;
                    changed_g[idx] = true;
                }
            }
            n_rows_changed += 1;
        }
    }

    let (g, h) = propagate::rebuild_g(n, real_g_rows, real_g_rhs, &lb, &ub);
    SparsifyResult { a: csr_from_rows(&a_rows, n), b, g, h, n_rows_changed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_row_vec;

    #[test]
    fn sparsifies_a_superset_inequality_row_with_zero_fill_in() {
        // eq: x0 + x1 = 5 (support {0,1}).
        // target (G, real row): x0 + x1 + x2 <= 10 (support {0,1,2}, a
        // strict superset) -> after subtracting 1*eq: x2 <= 5.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 3);
        let b = vec![5.0];
        let g = csr_from_rows(
            &[
                vec![(0, 1.0), (1, 1.0), (2, 1.0)], // target
                vec![(0, 1.0)],
                vec![(0, -1.0)],
                vec![(1, 1.0)],
                vec![(1, -1.0)],
                vec![(2, 1.0)],
                vec![(2, -1.0)],
            ],
            3,
        );
        let h = vec![10.0, 10.0, 0.0, 10.0, 0.0, 10.0, 0.0];

        let result = sparsify(3, &a, &b, &g, &h);
        assert_eq!(result.n_rows_changed, 1);

        // The sparsified row (`x2 <= 5`) has exactly one variable, so
        // `extract_bounds` correctly folds it straight into `ub[2]` as a
        // tighter bound rather than keeping it as a "real" multi-variable
        // row — a free extra reduction, not a bug in this test.
        let (_, ub, real_rows, _) = propagate::extract_bounds(3, &result.g, &result.h);
        assert!(real_rows.is_empty(), "the fully-sparsified row collapsed into a bound, not a real row");
        assert!((ub[2] - 5.0).abs() < 1e-9);
    }

    #[test]
    fn sparsifies_a_superset_equality_row() {
        // eq: x0 + x1 = 5. target (A): 2*x0 + 2*x1 + 3*x2 = 20
        // -> subtract 2*eq: 3*x2 = 10.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0), (2, 3.0)]], 3);
        let b = vec![5.0, 20.0];
        let g = csr_from_rows(
            &[vec![(0, 1.0)], vec![(0, -1.0)], vec![(1, 1.0)], vec![(1, -1.0)], vec![(2, 1.0)], vec![(2, -1.0)]],
            3,
        );
        let h = vec![10.0, 0.0, 10.0, 0.0, 10.0, 0.0];

        let result = sparsify(3, &a, &b, &g, &h);
        assert_eq!(result.n_rows_changed, 1);

        let row1 = csr_row_vec(&result.a, 1);
        assert_eq!(row1, vec![(2, 3.0)]);
        assert!((result.b[1] - 10.0).abs() < 1e-9);
    }

    #[test]
    fn leaves_a_non_superset_row_untouched() {
        // eq: x0 + x1 = 5. target: x0 + x2 <= 10 -- doesn't contain x1,
        // so `S_eq` isn't a subset of the target's support; nothing to do.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 3);
        let b = vec![5.0];
        let g = csr_from_rows(
            &[
                vec![(0, 1.0), (2, 1.0)],
                vec![(0, 1.0)],
                vec![(0, -1.0)],
                vec![(1, 1.0)],
                vec![(1, -1.0)],
                vec![(2, 1.0)],
                vec![(2, -1.0)],
            ],
            3,
        );
        let h = vec![10.0, 10.0, 0.0, 10.0, 0.0, 10.0, 0.0];

        let result = sparsify(3, &a, &b, &g, &h);
        assert_eq!(result.n_rows_changed, 0);
    }

    #[test]
    fn a_row_already_used_as_a_target_is_never_used_as_a_later_pivot() {
        // Three equality rows sharing structure such that, without the
        // "never re-pivot an already-rewritten row" guard, row 0 would
        // sparsify row 1, then row 1's *original* content (while row 1
        // itself had just been overwritten) would sparsify row 0 right
        // back — silently corrupting the pair (confirmed on Netlib
        // `scorpion`; see the module docs). Regression test for that.
        //
        // eq0: x0 + x1 = 3            (support {0,1})
        // eq1: x0 + x1 + x2 + x3 = 10 (support {0,1,2,3}, superset of eq0)
        // After eq0 sparsifies eq1 (subtract 1*eq0): eq1' = x2 + x3 = 7.
        // eq1's *original* support ({0,1,2,3}) is not a subset of eq0's
        // ({0,1}), so eq1 (original) could never legitimately sparsify
        // eq0 anyway -- eq0 must survive this call completely unchanged.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0)]], 4);
        let b = vec![3.0, 10.0];
        let g = csr_from_rows(
            &[
                vec![(0, 1.0)],
                vec![(0, -1.0)],
                vec![(1, 1.0)],
                vec![(1, -1.0)],
                vec![(2, 1.0)],
                vec![(2, -1.0)],
                vec![(3, 1.0)],
                vec![(3, -1.0)],
            ],
            4,
        );
        let h = vec![10.0, 0.0, 10.0, 0.0, 10.0, 0.0, 10.0, 0.0];

        let result = sparsify(4, &a, &b, &g, &h);
        assert_eq!(result.n_rows_changed, 1);

        let row0 = csr_row_vec(&result.a, 0);
        assert_eq!(row0, vec![(0, 1.0), (1, 1.0)], "eq0 must survive this call completely unchanged");
        assert!((result.b[0] - 3.0).abs() < 1e-9);

        let row1 = csr_row_vec(&result.a, 1);
        assert_eq!(row1, vec![(2, 1.0), (3, 1.0)]);
        assert!((result.b[1] - 7.0).abs() < 1e-9);
    }
}
