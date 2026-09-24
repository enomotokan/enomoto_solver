//! Stuffing Singleton Columns (シングルトン列の詰め込み, Gamrath et al. "Progress in Presolving
//! for Mixed Integer Programming" 2015 §3)。**現在は未使用** (パイプラインから呼ばれていない。
//! 経緯は `presolve.rs` の呼び出し箇所のコメントおよび履歴メモ参照)。
//!
//! 実不等式行にちょうど 1 回だけ現れる連続変数 (シングルトン列) のうち、目的関数の符号が
//! 行にとって「不利な」向き (`dualfix` が扱えない残り) のものを、その行の余裕をナップサック的に
//! 分け合う推論で境界値に固定する。
//!
//! ## `a_rj > 0, c_j < 0` の場合 (論文の Algorithm 1)
//!
//! `<=` 行 `r` の候補集合 `J(r)` について、候補を下限に置いたまま他列を最悪/最良にした活動値
//!
//! ```text
//! Ũr = sum_{j in J(r)} a_rj*lb_j + sum_{k not in J(r), a_rk>0} a_rk*ub_k + sum_{k not in J(r), a_rk<0} a_rk*lb_k
//! L̃r = sum_{j in J(r)} a_rj*lb_j + sum_{k not in J(r), a_rk<0} a_rk*ub_k + sum_{k not in J(r), a_rk>0} a_rk*lb_k
//! ```
//!
//! を求め、`c_j/a_rj` の昇順に `alpha = a_rj*ub_j`, `beta = a_rj*lb_j` として:
//!
//! - `alpha <= b_r - Ũr + beta` なら上限に固定
//! - そうでなく `b_r <= L̃r` なら下限に固定
//! - それ以外は未決定
//!
//! 判定の結果によらず毎回 `Ũr`・`L̃r` に `alpha - beta` を加える (後の候補は、この候補が上限まで動く可能性を考慮する必要があるため)。
//!
//! ## 鏡像の場合 `a_rj < 0, c_j > 0`
//!
//! `y_j = ub_j - x_j` と変数変換すると係数 `-a_rj > 0`、コスト `-c_j < 0`、右辺 `b_r - sum a_rj*ub_j` の
//! 上の場合に帰着するので、同じ [`stuffing_core`] で判定し、結果を `x` に戻す (`y` 上限 ⇔ `x` 下限)。
//!
//! ## 対象外
//!
//! 候補列自身の境界は両側有限でなければならない (他の列は無限でもよい)。固定は自分の境界値へのみで、
//! 境界の部分的な強化はしない。同じ行の 2 つの場合は互いを通常の「他列」として独立に扱う。

use crate::sparse::{Csr, csr_row_iter};
use crate::params::presolve::TOL;

/// 1 つの行について判定する連続シングルトン列の候補。`stuffing_core` が前提とする
/// `a > 0, c < 0` の向きに変換済み (鏡像の場合は `y` 空間の値)。
struct Candidate {
    /// 元の列番号
    j: usize,
    /// 行での係数 (> 0)
    a: f64,
    /// 下限 (鏡像の場合は `y` の下限 0)
    l: f64,
    /// 上限 (鏡像の場合は `y` の上限 `ub - lb`)
    u: f64,
    /// コスト (< 0)
    c: f64,
}

/// モジュール docs の Algorithm 1 (`a > 0, c < 0` の向きのみ)。各候補を `u` に固定するか
/// `l` に固定するか未決定のままにするかを決め、`(j, true)` = `u` へ固定、`(j, false)` = `l` へ固定 を返す。
///
/// - `row_other`: 候補以外の行の非零要素 (`Ũr`/`L̃r` の「その他」の和に、実際の `lb`/`ub` で使う)
/// - `b`: 行の右辺 (鏡像の場合はシフト済み)
/// - `candidates`: 判定対象の候補
/// - `lb`, `ub`: 全変数の境界
fn stuffing_core(row_other: &[(usize, f64)], b: f64, mut candidates: Vec<Candidate>, lb: &[f64], ub: &[f64]) -> Vec<(usize, bool)> {
    // tilde_u = Ũr (活動値の上界), tilde_l = L̃r (活動値の下界)
    let mut tilde_u = 0.0f64;
    let mut tilde_l = 0.0f64;
    for cand in &candidates {
        tilde_u += cand.a * cand.l;
        tilde_l += cand.a * cand.l;
    }
    for &(k, ak) in row_other {
        if ak > 0.0 {
            tilde_u += ak * ub[k];
            tilde_l += ak * lb[k];
        } else if ak < 0.0 {
            tilde_u += ak * lb[k];
            tilde_l += ak * ub[k];
        }
    }

    // c/a の昇順 (単位余裕あたりの目的改善が大きい順)。同値なら列番号順。
    candidates.sort_by(|x, y| {
        let rx = x.c / x.a;
        let ry = y.c / y.a;
        rx.partial_cmp(&ry).unwrap_or(std::cmp::Ordering::Equal).then(x.j.cmp(&y.j))
    });

    let mut fixings = Vec::new();
    for cand in &candidates {
        let alpha = cand.a * cand.u;
        let beta = cand.a * cand.l;
        if alpha <= b - tilde_u + beta - TOL {
            fixings.push((cand.j, true));
        } else if b <= tilde_l - TOL {
            fixings.push((cand.j, false));
        }
        // 判定結果によらず無条件に更新する (後の候補はこの候補が上限まで動く可能性を仮定する)。
        tilde_l += alpha - beta;
        tilde_u += alpha - beta;
    }
    fixings
}

/// 固定できる連続シングルトン列の `(j, 値)` の一覧を返す。
///
/// - `n`: 列数
/// - `a`: 等式行列 (ここに現れる列は対象外)
/// - `real_g_rows`, `real_g_rhs`: `G` の複数変数の実制約行とその右辺 (箱境界行は出現回数に数えない)
/// - `c`: 目的関数係数
/// - `lb`, `ub`: 変数の境界
pub fn fix_singleton_columns(n: usize, a: &Csr, real_g_rows: &[Vec<(usize, f64)>], real_g_rhs: &[f64], c: &[f64], lb: &[f64], ub: &[f64]) -> Vec<(usize, f64)> {
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

    // occ[j]: 列 j の実不等式行でのただ 1 回の出現 (行番号, 係数)。2 回目を見たら seen_twice[j] を立てて除外する。
    let mut occ: Vec<Option<(usize, f64)>> = vec![None; n];
    let mut seen_twice = vec![false; n];
    for (i, row) in real_g_rows.iter().enumerate() {
        for &(j, v) in row {
            if v == 0.0 || seen_twice[j] {
                continue;
            }
            if occ[j].is_some() {
                occ[j] = None;
                seen_twice[j] = true;
            } else {
                occ[j] = Some((i, v));
            }
        }
    }

    // 各行の候補を 2 つに分ける: case_a = `a_rj>0, c_j<0` (直接判定)、
    // case_b = `a_rj<0, c_j>0` (`y = ub-x` で case A に帰着)。1 列はどちらか一方にしか入らない。
    let mut case_a: Vec<Vec<(usize, f64)>> = vec![Vec::new(); real_g_rows.len()];
    let mut case_b: Vec<Vec<(usize, f64)>> = vec![Vec::new(); real_g_rows.len()];
    for j in 0..n {
        let Some((i, coeff)) = occ[j] else { continue };
        if in_equality[j] || !lb[j].is_finite() || !ub[j].is_finite() {
            continue;
        }
        if coeff > 0.0 && c[j] < -TOL {
            case_a[i].push((j, coeff));
        } else if coeff < 0.0 && c[j] > TOL {
            case_b[i].push((j, coeff));
        }
    }

    let mut fixed = Vec::new();
    for (i, row) in real_g_rows.iter().enumerate() {
        if !case_a[i].is_empty() {
            // members: この行の候補列の集合 / row_other: 候補以外の要素
            let members: std::collections::HashSet<usize> = case_a[i].iter().map(|&(j, _)| j).collect();
            let row_other: Vec<(usize, f64)> = row.iter().filter(|&&(k, _)| !members.contains(&k)).cloned().collect();
            let candidates: Vec<Candidate> = case_a[i].iter().map(|&(j, coeff)| Candidate { j, a: coeff, l: lb[j], u: ub[j], c: c[j] }).collect();
            for (j, at_upper) in stuffing_core(&row_other, real_g_rhs[i], candidates, lb, ub) {
                fixed.push((j, if at_upper { ub[j] } else { lb[j] }));
            }
        }
        if !case_b[i].is_empty() {
            let members: std::collections::HashSet<usize> = case_b[i].iter().map(|&(j, _)| j).collect();
            let row_other: Vec<(usize, f64)> = row.iter().filter(|&&(k, _)| !members.contains(&k)).cloned().collect();
            // y 変換で生じる定数項を右辺へ移したもの
            let mut b_shifted = real_g_rhs[i];
            let candidates: Vec<Candidate> = case_b[i]
                .iter()
                .map(|&(j, coeff)| {
                    b_shifted -= coeff * ub[j];
                    Candidate { j, a: -coeff, l: 0.0, u: ub[j] - lb[j], c: -c[j] }
                })
                .collect();
            for (j, at_upper_y) in stuffing_core(&row_other, b_shifted, candidates, lb, ub) {
                // y が上限なら x_j = lb_j、y が下限 (0) なら x_j = ub_j (case A と逆の対応)。
                fixed.push((j, if at_upper_y { lb[j] } else { ub[j] }));
            }
        }
    }
    fixed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    /// The paper's own motivating example: a fractional (continuous)
    /// knapsack, `3x0+2x1+x2<=4`, `x in [0,1]^3`, maximizing profits
    /// `9,4,1` (stored as costs `-9,-4,-1` to minimize). Ratios
    /// `c_j/a_j = -3,-2,-1`. Greedily filling by best ratio first: item 0
    /// fits entirely (uses 3 of 4), item 1 only half-fits (fractional in
    /// the true LP optimum — must stay undetermined), item 2 gets none of
    /// the remaining budget. Hand-verified against the classic fractional-
    /// knapsack optimum (`x0=1, x1=0.5, x2=0`), not just against this
    /// module's own arithmetic.
    #[test]
    fn fractional_knapsack_best_and_worst_ratio_items_get_fixed() {
        let n = 3;
        let a = csr_from_rows(&[], n);
        let row = vec![(0, 3.0), (1, 2.0), (2, 1.0)];
        let rhs = vec![4.0];
        let c = vec![-9.0, -4.0, -1.0];
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![1.0, 1.0, 1.0];
        let fixed = fix_singleton_columns(n, &a, &[row], &rhs, &c, &lb, &ub);
        let mut by_j: std::collections::HashMap<usize, f64> = fixed.into_iter().collect();
        assert_eq!(by_j.remove(&0), Some(1.0), "best ratio: fully packed to its upper bound");
        assert_eq!(by_j.remove(&2), Some(0.0), "worst ratio: no budget left, fixed to lower bound");
        assert!(!by_j.contains_key(&1), "middle item is genuinely fractional (0.5) in the true optimum, must stay undetermined");
    }

    /// The exact mirror of the knapsack test above under `y_j = ub_j-x_j`
    /// (`a' = -a`, `c' = -c`, `b' = b - sum(a*ub)` — see the module docs'
    /// derivation): independently hand-verified (not just algebraically)
    /// against the same fractional-knapsack optimum, reframed as "which
    /// items must be forced up, at least cost, to make an already-violated
    /// row feasible again" — cheapest-per-unit-of-slack-gained (item 2,
    /// ratio 1) goes first, most expensive (item 0, ratio 3) last.
    #[test]
    fn mirror_case_reduces_correctly_via_the_y_substitution() {
        let n = 3;
        let a = csr_from_rows(&[], n);
        let row = vec![(0, -3.0), (1, -2.0), (2, -1.0)];
        let rhs = vec![-2.0];
        let c = vec![9.0, 4.0, 1.0];
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![1.0, 1.0, 1.0];
        let fixed = fix_singleton_columns(n, &a, &[row], &rhs, &c, &lb, &ub);
        let mut by_j: std::collections::HashMap<usize, f64> = fixed.into_iter().collect();
        assert_eq!(by_j.remove(&0), Some(0.0), "most expensive to force up: stays at its cost-favorable lower bound");
        assert_eq!(by_j.remove(&2), Some(1.0), "cheapest to force up: fully forced to its upper bound");
        assert!(!by_j.contains_key(&1), "genuinely fractional (0.5) in the true optimum, must stay undetermined");
    }

    /// A column appearing in *two* real rows is not a singleton at all —
    /// moving it can affect a row this module never looks at, so it must
    /// never be fixed here regardless of how favorable its sign looks in
    /// either row alone.
    #[test]
    fn column_in_two_rows_is_not_a_singleton() {
        let n = 2;
        let a = csr_from_rows(&[], n);
        let rows = vec![vec![(0, 3.0), (1, 1.0)], vec![(0, 2.0)]];
        let rhs = vec![4.0, 10.0];
        let c = vec![-9.0, 0.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![1.0, 1.0];
        let fixed = fix_singleton_columns(n, &a, &rows, &rhs, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    /// A column appearing in an equality row (even with a favorable-
    /// looking sign in some unrelated inequality row) is excluded outright
    /// — same exclusion `dualfix` applies, since an equality row locks
    /// both directions at once.
    #[test]
    fn column_in_equality_row_is_excluded() {
        let n = 2;
        let a = csr_from_rows(&[vec![(0, 1.0)]], n);
        let rows = vec![vec![(0, 3.0), (1, 1.0)]];
        let rhs = vec![4.0];
        let c = vec![-9.0, 0.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![1.0, 1.0];
        let fixed = fix_singleton_columns(n, &a, &rows, &rhs, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    /// The objective-*favorable* sign combination (`a_rj>0, c_j>=0`) is
    /// `dualfix`'s own residual, not this module's — left untouched here
    /// regardless of the row's slack.
    #[test]
    fn favorable_sign_is_left_to_dualfix() {
        let n = 1;
        let a = csr_from_rows(&[], n);
        let row = vec![(0, 3.0)];
        let rhs = vec![4.0];
        let c = vec![1.0];
        let lb = vec![0.0];
        let ub = vec![1.0];
        let fixed = fix_singleton_columns(n, &a, &[row], &rhs, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    /// An unbounded "else" column sharing the row makes `Ũr` genuinely
    /// `+inf` (it could always claim however much slack is left), so the
    /// only sound conclusion is "never provably safe to push the candidate
    /// up" — no `alpha <= b - inf + beta` ever fires, and no `inf - inf`
    /// arithmetic ever happens either (see the module docs on why not).
    #[test]
    fn unbounded_other_column_blocks_the_upper_fixing() {
        let n = 2;
        let a = csr_from_rows(&[], n);
        // x1 unbounded above with a positive coefficient: the row's true
        // maximum activity is +inf, so nothing about x0's own slack can
        // ever be guaranteed.
        let row = vec![(0, 3.0), (1, 1.0)];
        let rhs = vec![100.0];
        let c = vec![-9.0, 0.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![1.0, f64::INFINITY];
        let fixed = fix_singleton_columns(n, &a, &[row], &rhs, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    /// A candidate with an infinite bound of its own has no finite
    /// baseline to reason about and must be skipped outright, not treated
    /// as some sentinel-large finite value.
    #[test]
    fn candidate_with_infinite_own_bound_is_skipped() {
        let n = 1;
        let a = csr_from_rows(&[], n);
        let row = vec![(0, 3.0)];
        let rhs = vec![4.0];
        let c = vec![-9.0];
        let lb = vec![0.0];
        let ub = vec![f64::INFINITY];
        let fixed = fix_singleton_columns(n, &a, &[row], &rhs, &c, &lb, &ub);
        assert!(fixed.is_empty());
    }

    #[test]
    fn empty_input_fixes_nothing() {
        let n = 0;
        let a = csr_from_rows(&[], n);
        let fixed = fix_singleton_columns(n, &a, &[], &[], &[], &[], &[]);
        assert!(fixed.is_empty());
    }
}
