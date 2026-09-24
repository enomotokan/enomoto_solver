//! DualFix (双対固定, Achterberg et al. "Presolve Reductions in Mixed Integer
//! Programming" §4.4)。
//!
//! 目的関数が一方向を好み、かつその方向への移動を妨げる「実制約」が一つもない変数を、
//! 対応する境界値へ直ちに固定する。
//!
//! - 各実制約行 (`G` は `<=` 正規化済み) について、正係数は「上ロック」、負係数は「下ロック」を数える。
//! - 等式行に現れる変数は両方向ロックとみなし、対象外とする。
//! - 変数自身の箱境界行は実制約に含めない (含めると全変数が自分の境界でロックされてしまう)。
//! - 下ロック数 0 かつ コスト `>= 0` なら下限へ、上ロック数 0 かつ コスト `<= 0` なら上限へ固定。

use crate::sparse::{Csr, csr_row_iter};
use crate::params::presolve::TOL;

/// 直ちに固定できる変数の `(列番号 j, 固定値)` の一覧を返す。
///
/// - `n`: 変数 (列) 数
/// - `a`: 等式制約行列 `A` (ここに現れる変数は固定対象外)
/// - `real_g_rows`: `G` のうち複数変数を含む「実制約」行のみ
///   (`propagate::extract_bounds` が単変数の境界行と分離済みのものを再利用)
/// - `c`: 目的関数係数 (最小化)
/// - `lb`, `ub`: 各変数の下限・上限 (固定先の境界が有限でなければ固定しない)
pub fn fix_dominated_variables(
    n: usize,
    a: &Csr,
    real_g_rows: &[Vec<(usize, f64)>],
    c: &[f64],
    lb: &[f64],
    ub: &[f64],
) -> Vec<(usize, f64)> {
    // 各変数の上ロック数・下ロック数 (G の実制約行での正係数・負係数の個数)
    let mut up_lock = vec![0usize; n];
    let mut down_lock = vec![0usize; n];
    // 変数がいずれかの等式行に非零係数で現れるか
    let mut in_equality = vec![false; n];

    let ar = a.as_ref();
    for i in 0..ar.nrows() {
        for (j, v) in csr_row_iter(a, i) {
            if v != 0.0 {
                in_equality[j] = true;
            }
        }
    }

    for row in real_g_rows {
        for &(j, v) in row {
            if v > 0.0 {
                up_lock[j] += 1;
            } else if v < 0.0 {
                down_lock[j] += 1;
            }
        }
    }

    // 固定する (列番号, 値) の一覧
    let mut fixed = Vec::new();
    for j in 0..n {
        if in_equality[j] {
            continue;
        }
        if down_lock[j] == 0 && c[j] >= -TOL && lb[j].is_finite() {
            fixed.push((j, lb[j]));
        } else if up_lock[j] == 0 && c[j] <= TOL && ub[j].is_finite() {
            fixed.push((j, ub[j]));
        }
    }
    fixed
}
