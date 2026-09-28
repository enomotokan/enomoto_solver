//! RowSingleton (行シングルトン): 非零係数がちょうど 1 個の等式行 `coeff * x_j = rhs` から
//! `x_j = rhs / coeff` を直接固定し、その行を削除する。
//!
//! - 行内に他の変数がないので、`colsingleton`/`doubleton` のような代入の記録は不要。
//! - `dualfix` (コスト符号による固定) や `propagate::extract_bounds` (`G` の単変数行のみを扱う)
//!   とは別物で、こちらは `A` の等式行だけを見る。
//! - 固定値が変数の `[lb, ub]` の外にあれば、境界を黙って上書きせず実行不能として報告する。
//!   ただし、はみ出しが丸めの範囲 (`TOL * (1 + |値|)`) なら値のまま、実行可能性許容誤差
//!   (`ROWSINGLETON_CLAMP_TOL * max(|値|, 1)`) 以内なら境界へクランプして固定する (作業 #9)。

use crate::sparse::{FaerCsr, csr_from_rows, csr_is_canonical, csr_row_iter};
use crate::params::presolve::{ROWSINGLETON_CLAMP_TOL, TOL};
use super::infeas_tol;

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
            let mut value = b[i] / coeff;
            // 境界からのはみ出し (境界内なら 0 以下)
            let viol = (lb[j] - value).max(value - ub[j]);
            if viol > 0.0 {
                // 丸めの範囲 (`TOL`、既定は値の大きさに対する相対形) なら値そのもの (行を厳密に満たす) で固定する。
                // それを超えても実行可能性許容誤差 (`ROWSINGLETON_CLAMP_TOL * max(|値|, 1)`、単体法の基底変数の判定と同じ形) 以内なら境界へクランプ
                // (HiGHS の行シングルトンと同じ)。さらに超えれば実行不能。
                // ken-18 では伝播で得た下限が桁落ちで真値より 1.4e-8 (相対 4e-14) 大きく、絶対 1e-9 では
                // 実行可能解そのものを境界外と判定していた。
                if viol > infeas_tol(TOL, value) {
                    let clamp_tol = tunable!("ENOMOTO_T_ROWSINGLETON_CLAMP_TOL", ROWSINGLETON_CLAMP_TOL, f64);
                    if viol > clamp_tol * value.abs().max(1.0) {
                        return RowSingletonResult { a: csr_from_rows(&[], n), b: Vec::new(), fixes: Vec::new(), infeasible: true };
                    }
                    value = if value < lb[j] { lb[j] } else { ub[j] };
                }
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

    /// ken-18 型: スケール後の大きな値 (3.5e5) で、伝播由来の下限が丸め・桁落ちで値より相対 4e-14
    /// (絶対 1.4e-8 > 1e-9) だけ大きい。実行可能なので値そのもので固定する。
    #[test]
    fn tiny_relative_bound_violation_at_large_scale_is_not_infeasible() {
        let coeff = -1.888290021904761e-2;
        let b = vec![-6.619373669328833e3];
        let a = csr_from_rows(&[vec![(0, coeff)]], 1);
        let value = b[0] / coeff;
        let lb = vec![3.505485700047277e5];
        assert!(lb[0] - value > 1e-9, "the absolute violation must exceed the old absolute TOL");
        let ub = vec![1.051645710014155e6];
        let result = fix_singleton_equalities(1, &a, &b, &lb, &ub);
        assert!(!result.infeasible);
        assert_eq!(result.fixes, vec![(0, value)]);
    }

    /// はみ出しが丸めより大きいが実行可能性許容誤差 (相対 1e-7) 以内なら境界へクランプする。
    #[test]
    fn feasibility_level_violation_is_clamped_to_the_bound() {
        let a = csr_from_rows(&[vec![(0, 1.0)]], 1);
        let b = vec![1000.0 * (1.0 + 1e-8)]; // ub を相対 1e-8 超える
        let lb = vec![0.0];
        let ub = vec![1000.0];
        let result = fix_singleton_equalities(1, &a, &b, &lb, &ub);
        assert!(!result.infeasible);
        assert_eq!(result.fixes, vec![(0, 1000.0)]);
    }

    /// 相対 1e-7 を超えるはみ出しは (値が大きくても) 実行不能。
    #[test]
    fn violation_beyond_feasibility_tolerance_is_infeasible_at_large_scale() {
        let a = csr_from_rows(&[vec![(0, 1.0)]], 1);
        let b = vec![1e6 * (1.0 + 1e-6)];
        let lb = vec![0.0];
        let ub = vec![1e6];
        let result = fix_singleton_equalities(1, &a, &b, &lb, &ub);
        assert!(result.infeasible);
    }
}
