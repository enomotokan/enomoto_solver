//! 試作 `ENOMOTO_MIP_DUAL_TIGHT_INEQ` (既定では使わない): 双対の議論 (支配) で不等式の行を等式に上げる (整数列でも使える形)。
//!
//! SCIP の linear 制約の双対の前処理と同じ考え (規則を読んで自前で書いた)。`G` の行は `Σ a_k x_k ≤ h` に正規化済み、最小化。
//! 行 `r` と、その行の列 `j` (係数 `a = a_rj`) について、`a > 0` のとき
//! 1. `j` は等式の行に現れず、`r` 以外の `G` の行の係数はすべて `≤ 0` (`x_j` を増やしても他の行は緩むだけ)、
//! 2. `c_j ≤ 0` (`x_j` を増やしても目的は悪くならない)、
//! 3. `j` が整数列なら、行 `r` の列はすべて整数列で、係数と右辺は `a` の整数倍 (余裕 `(h - 左辺) / a` が整数)、
//! 4. `x_j` が上限 `u_j` に着いたら行がちょうど等号になる: `u_j` が無限か、`r` の他の項の最小の左辺 + `a u_j ≥ h`、
//!
//! ならば、どの実行可能解も `x_j` を `min(u_j - x_j, 余裕 / a)` だけ増やして (実行可能・目的は悪くならない) 行 `r` を等号にできる。
//! なので行 `r` を等式にした問題は、元の問題と最適値・実行可能性が同じで、その解は元の問題の解でもある (後処理は不要)。
//! `a < 0` は向きを逆にする (他の行の係数 `≥ 0`、`c_j ≥ 0`、下限 `l_j` で `最小の左辺 + a l_j ≥ h`)。
//!
//! neos859080: `b_a + b_b ≤ 1` (0-1 列、費用 0、他は `y ≤ 10 b` だけ) が 40 本あり、等式にすると集約が `b_b = 1 - b_a` で 40 列を消す
//! (SCIP・HiGHS の前処理と同じ 120 列)。
//!
//! 1 本ずつ上げる操作の列として正しいように、上げた行は等式として扱い、等式に現れる列は以後根拠に使わない (今の問題で毎回確かめる)。

use crate::params::presolve::TOL;

/// 等式に上げる `real_rows` の行番号 (選んだ順)。引数は [`super::dualfreeineq::find_dual_free_inequalities`] と同じ。
pub fn find_dual_tight_inequalities(n: usize, a_rows: &[Vec<(usize, f64)>], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], c: &[f64], lb: &[f64], ub: &[f64], is_int: Option<&[bool]>) -> Vec<usize> {
    let int = |j: usize| is_int.is_some_and(|m| m[j]);
    // 列が等式の行に現れるか
    let mut in_eq = vec![false; n];
    for r in a_rows {
        for &(j, v) in r {
            if v != 0.0 {
                in_eq[j] = true;
            }
        }
    }
    // 列ごとの G の行での係数の符号の数 (正・負)
    let mut npos = vec![0u32; n];
    let mut nneg = vec![0u32; n];
    for r in real_rows {
        for &(j, v) in r {
            if v > 0.0 {
                npos[j] += 1;
            } else if v < 0.0 {
                nneg[j] += 1;
            }
        }
    }
    let is_integral = |x: f64| (x - x.round()).abs() <= TOL * (1.0 + x.abs());
    let mut out = Vec::new();
    for (ri, row) in real_rows.iter().enumerate() {
        let h = real_rhs[ri];
        if !h.is_finite() || row.len() < 2 {
            continue;
        }
        // 他の項の最小の左辺を出すための、行全体の最小の左辺 (無限の項の数も数える)
        let (mut minact, mut ninf) = (0.0f64, 0usize);
        for &(k, v) in row {
            let b = if v > 0.0 { lb[k] } else { ub[k] };
            if b.is_finite() {
                minact += v * b;
            } else {
                ninf += 1;
            }
        }
        let all_int = row.iter().all(|&(k, _)| int(k));
        let mut chosen = None;
        for &(j, a) in row {
            if a == 0.0 || in_eq[j] {
                continue;
            }
            // 1・2: 他の行は j を動かす向きで緩むだけ、目的も悪くならない
            let (ok_rows, ok_cost, bound) = if a > 0.0 { (npos[j] == 1, c[j] <= 0.0, ub[j]) } else { (nneg[j] == 1, c[j] >= 0.0, lb[j]) };
            if !ok_rows || !ok_cost {
                continue;
            }
            // 3: 整数列なら余裕 / a が整数
            if int(j) && !(all_int && is_integral(h / a) && row.iter().all(|&(_, v)| is_integral(v / a))) {
                continue;
            }
            // 4: 境界に着いたら等号 (境界が無限なら必ず等号まで動ける)
            if bound.is_finite() {
                let own = if a > 0.0 { lb[j] } else { ub[j] };
                // 行の最小の左辺から j の分を除く
                let (others, inf_others) = if own.is_finite() { (minact - a * own, ninf) } else { (minact, ninf - 1) };
                if inf_others > 0 || others + a * bound < h - TOL * (1.0 + h.abs()) {
                    continue;
                }
            }
            chosen = Some(j);
            break;
        }
        if chosen.is_some() {
            out.push(ri);
            // 上げた行は等式: その列は以後根拠に使わない
            for &(k, _) in row {
                in_eq[k] = true;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_row_of_binaries_is_promoted() {
        // b0 + b1 ≤ 1, y0 - 10 b0 ≤ 0, y1 - 10 b1 ≤ 0 (列: b0, b1, y0, y1)
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(0, -10.0), (2, 1.0)], vec![(1, -10.0), (3, 1.0)]];
        let rhs = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 0.0, 1.0, 1.0];
        let lb = vec![0.0; 4];
        let ub = vec![1.0, 1.0, 10.0, 10.0];
        let isint = vec![true; 4];
        assert_eq!(find_dual_tight_inequalities(4, &[], &rows, &rhs, &c, &lb, &ub, Some(&isint)), vec![0]);
    }

    #[test]
    fn not_promoted_when_bound_leaves_slack() {
        // b0 + b1 ≤ 2 (0-1 列): b0 = 1 で行が等号になるとは限らない
        let rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let isint = vec![true; 2];
        assert!(find_dual_tight_inequalities(2, &[], &rows, &[2.0], &[0.0, 0.0], &[0.0, 0.0], &[1.0, 1.0], Some(&isint)).is_empty());
        // 費用が負でない向きにしか動かせない列 (費用 1 の増加) は使わない
        assert!(find_dual_tight_inequalities(2, &[], &rows[..], &[1.0], &[1.0, 1.0], &[0.0, 0.0], &[1.0, 1.0], Some(&isint)).is_empty());
    }

    #[test]
    fn fractional_step_blocks_integer_column() {
        // b0 + 0.5 x ≤ 1 (x 整数): b0 は余裕が 0.5 の倍数なので整数のまま等号にできないことがあり使えない。
        // x は余裕 / 0.5 が整数で使えるが、上限 1 では x = 1 で等号にならない (0.5 < 1) ので上げない
        let rows = vec![vec![(0, 1.0), (1, 0.5)]];
        let isint = vec![true; 2];
        assert!(find_dual_tight_inequalities(2, &[], &rows, &[1.0], &[0.0, 0.0], &[0.0, 0.0], &[1.0, 1.0], Some(&isint)).is_empty());
        // x の上限が 3 なら x = 2 で等号まで動ける (b0 = 0) か、b0 = 1 で既に等号
        assert_eq!(find_dual_tight_inequalities(2, &[], &rows, &[1.0], &[0.0, 0.0], &[0.0, 0.0], &[1.0, 3.0], Some(&isint)), vec![0]);
    }
}
