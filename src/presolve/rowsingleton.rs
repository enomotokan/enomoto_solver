//! RowSingleton (行シングルトン): 非零係数がちょうど 1 個の等式行 `coeff * x_j = rhs` から
//! `x_j = rhs / coeff` を直接固定し、その行を削除する。
//!
//! - 行内に他の変数がないので、`colsingleton`/`doubleton` のような代入の記録は不要。
//! - `dualfix` (コスト符号による固定) や `propagate::extract_bounds` (`G` の単変数行のみを扱う)
//!   とは別物で、こちらは `A` の等式行だけを見る。
//! - 固定値が変数の `[lb, ub]` の外にあれば、境界を黙って上書きせず実行不能として報告する。

use crate::sparse::{FaerCsr, csr_from_rows, csr_is_canonical, csr_row_iter};
use crate::params::presolve::TOL;

/// [`fix_singleton_equalities`] の結果。
pub struct RowSingletonResult {
    /// シングルトン行を除いた後の等式行列 `A`
    pub a: FaerCsr,
    /// `a` に対応する右辺
    pub b: Vec<f64>,
    /// このパスで固定した変数の `(j, 値)`
    pub fixes: Vec<(usize, f64)>,
    /// 固定値が境界外になり実行不能と判明したか (true のとき他フィールドは空)
    pub infeasible: bool,
}

/// `A` の行を 1 回だけ走査し、シングルトン等式行を固定・削除する (連鎖はしない)。
///
/// 固定後に新たにシングルトンになった行は、呼び出し側の次のラウンドで拾われる
/// (`colsingleton` と同じ「1 パスのみ」方針)。
///
/// - `n`: 列数
/// - `a`, `b`: 等式制約 `A x = b`
/// - `lb`, `ub`: 変数の現在の境界 (実行不能判定に使用)
pub fn fix_singleton_equalities(n: usize, a: &FaerCsr, b: &[f64], lb: &[f64], ub: &[f64]) -> RowSingletonResult {
    let ar = a.as_ref();
    // シングルトン行が一つもない場合 (初回ラウンド以降はこれが普通) は、正準形なら `a` をそのまま返す。
    if csr_is_canonical(a) && (0..ar.nrows()).all(|i| ar.col_indices_of_row_raw(i).len() != 1) {
        return RowSingletonResult { a: a.clone(), b: b[..ar.nrows()].to_vec(), fixes: Vec::new(), infeasible: false };
    }
    let mut new_a_rows = Vec::with_capacity(ar.nrows());
    let mut new_b = Vec::with_capacity(b.len());
    let mut fixes = Vec::new();

    for i in 0..ar.nrows() {
        // 明示的な 0 を除いた行 i の非零要素
        let row: Vec<(usize, f64)> =
            csr_row_iter(a, i).filter(|&(_, v)| v != 0.0).collect();
        if row.len() == 1 {
            let (j, coeff) = row[0];
            let value = b[i] / coeff;
            if value < lb[j] - TOL || value > ub[j] + TOL {
                return RowSingletonResult { a: csr_from_rows(&[], n), b: Vec::new(), fixes: Vec::new(), infeasible: true };
            }
            fixes.push((j, value));
            continue;
        }
        new_a_rows.push(row);
        new_b.push(b[i]);
    }

    RowSingletonResult { a: csr_from_rows(&new_a_rows, n), b: new_b, fixes, infeasible: false }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixes_a_singleton_row_and_drops_it() {
        // 2*x0 = 6 (row 0, singleton) ; x0 + x1 = 5 (row 1, not singleton)
        let a = csr_from_rows(&[vec![(0, 2.0)], vec![(0, 1.0), (1, 1.0)]], 2);
        let b = vec![6.0, 5.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 10.0];
        let result = fix_singleton_equalities(2, &a, &b, &lb, &ub);
        assert!(!result.infeasible);
        assert_eq!(result.fixes, vec![(0, 3.0)]);
        assert_eq!(result.a.nrows(), 1);
        assert_eq!(result.b, vec![5.0]);
    }

    #[test]
    fn reports_infeasible_when_forced_value_outside_bounds() {
        let a = csr_from_rows(&[vec![(0, 2.0)]], 1);
        let b = vec![100.0]; // x0 = 50, but ub is 10
        let lb = vec![0.0];
        let ub = vec![10.0];
        let result = fix_singleton_equalities(1, &a, &b, &lb, &ub);
        assert!(result.infeasible);
    }
}
