//! 双対で陰に自由な不等式行の等式化 (試作、`ENOMOTO_AGG_DUALFREE_INEQ` のときだけ使う。既定では使わない)。
//!
//! HiGHS の `HPresolve::aggregator` は、代入の候補の行として等式の行のほかに「双対で陰に自由な不等式の行」
//! (`isDualImpliedFree`) を使い、その行を右辺の等式として列を代入する (`dualImpliedFreeGetRhsAndRowType`)。
//! ここではその規則を読み、自前の書き方で作った (HiGHS は MIT ライセンス。規則を読んで自前のコードで書き直し、
//! コードは写していない)。当方では代入そのものは既存の集約 ([`crate::presolve::aggregator`]) に任せ、
//! ここでは行を等式に上げるだけにする (後処理は不要: 等式に上げた問題の解は元の問題でも実行可能)。
//!
//! 規則 (最小化、`G` の行は `Σ a_k x_k ≤ h` に正規化済み、≦ 行の双対は `y ≤ 0`、等式の双対は自由):
//! 連続列 `j` が
//! 1. 等式の行に現れず、
//! 2. 上下限が行から導ける (陰に自由。導ける値 = 境界も可で、許容は [`TOL`]。無限の境界は常に満たす) とき、
//!
//! `j` の境界を外しても問題は変わらないので、双対では被約費用が 0: `c_j = Σ_i a_ij y_i`。
//! `j` が現れる行 `r` について、`a_rj > 0` かつ `c_j ≤ 0` かつ他の行の係数がすべて負なら
//! `a_rj y_r = c_j − Σ_{i≠r} a_ij y_i ≤ 0` (各 `y_i ≤ 0`)、つまり `y_r ≤ 0` が他から決まる
//! (`a_rj < 0` なら符号を逆にして `c_j ≥ 0`・他の係数がすべて正)。このとき行 `r` を等式に変えた問題の双対は
//! 元の問題の双対と同じ (行 `r` の双対の符号の制約が冗長) なので、最適値は変わらず、等式に変えた問題の最適解は
//! 元の問題の最適解になる。
//!
//! 整数最適化では、整数列を任意の値に固定した各連続緩和に同じ議論を当てる (陰の境界は整数列の箱で計算するので、
//! 固定するとさらに狭くなり、陰に自由のまま)。そのため `j` は連続列に限る (行には整数列が混ざってよい)。
//!
//! 同時に複数の行を上げるときの注意: 行 `r` の根拠は `j` の他の行の双対が `y ≤ 0` であることなので、
//! その行を (別の列の根拠で) 等式に上げると根拠が崩れる。そこで貪欲に 1 本ずつ選び、選んだ行の根拠に使った行は
//! 以後上げない・すでに上げた行を根拠に使う列は選ばない、とする (1 本ずつ上げる操作の列として正しい)。
//!
//! 既知の限界: 連続緩和が非有界になる整数の固定があると、等式に変えた問題が実行不能になりうる
//! (HiGHS と同じ規則なので同じ)。

use crate::params::presolve::TOL;

/// 等式に上げる `real_rows` の行番号を返す (昇順ではなく選んだ順)。
///
/// - `n`: 列数
/// - `a_rows`: 等式の行 (この行に現れる列は対象外)
/// - `real_rows`, `real_rhs`: `G` の多変数の行 (`real_rows[i] · x ≤ real_rhs[i]`)
/// - `c`: 目的の係数 (最小化)
/// - `lb`, `ub`: 列の境界
/// - `is_int`: 整数列の印 (`None` なら連続の問題)
pub fn find_dual_free_inequalities(n: usize, a_rows: &[Vec<(usize, f64)>], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], c: &[f64], lb: &[f64], ub: &[f64], is_int: Option<&[bool]>) -> Vec<usize> {
    let m = real_rows.len();
    // 等式の行に現れる列の印
    let mut in_eq = vec![false; n];
    for row in a_rows {
        for &(k, _) in row {
            in_eq[k] = true;
        }
    }
    // 列ごとの (不等式の行, 係数)
    let mut col_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for (i, row) in real_rows.iter().enumerate() {
        for &(k, v) in row {
            if v != 0.0 {
                col_rows[k].push((i, v));
            }
        }
    }
    // 行の最小の活動 (有限の和と、-∞ の項の個数)
    let mut act_lo = vec![0.0f64; m];
    let mut act_inf = vec![0usize; m];
    for (i, row) in real_rows.iter().enumerate() {
        for &(k, v) in row {
            let b = if v > 0.0 { lb[k] } else { ub[k] };
            if b.is_finite() {
                act_lo[i] += v * b;
            } else {
                act_inf[i] += 1;
            }
        }
    }
    // 行 i の、列 j (係数 v) を除いた最小の活動
    let resid_lo = |i: usize, j: usize, v: f64| -> f64 {
        let b = if v > 0.0 { lb[j] } else { ub[j] };
        if b.is_finite() {
            if act_inf[i] == 0 { act_lo[i] - v * b } else { f64::NEG_INFINITY }
        } else if act_inf[i] == 1 {
            act_lo[i]
        } else {
            f64::NEG_INFINITY
        }
    };
    // 根拠に使った行 (以後上げない) と、上げた行
    let mut used_as_witness = vec![false; m];
    let mut promoted = vec![false; m];
    let mut out = Vec::new();
    for j in 0..n {
        if in_eq[j] || col_rows[j].is_empty() || is_int.is_some_and(|f| f[j]) || lb[j] >= ub[j] {
            continue;
        }
        let rows = &col_rows[j];
        // 陰に自由か: 各 ≦ 行 `v x_j ≤ h − (残りの最小)` から導ける境界
        let (mut lo, mut hi) = (f64::NEG_INFINITY, f64::INFINITY);
        for &(i, v) in rows {
            let r = resid_lo(i, j, v);
            if !r.is_finite() {
                continue;
            }
            let bnd = (real_rhs[i] - r) / v;
            if v > 0.0 {
                hi = hi.min(bnd);
            } else {
                lo = lo.max(bnd);
            }
        }
        let free = (lb[j] == f64::NEG_INFINITY || lo >= lb[j] - TOL) && (ub[j] == f64::INFINITY || hi <= ub[j] + TOL);
        if !free {
            continue;
        }
        // すでに上げた行を根拠に使う列は選ばない
        if rows.iter().any(|&(i, _)| promoted[i]) {
            continue;
        }
        let npos = rows.iter().filter(|&&(_, v)| v > 0.0).count();
        let nneg = rows.len() - npos;
        // 係数が正の行がちょうど 1 本で他がすべて負、費用 ≤ 0 → その正の行。逆も同様。
        let pick = if npos == 1 && c[j] <= 0.0 {
            rows.iter().find(|&&(_, v)| v > 0.0).map(|&(i, _)| i)
        } else if nneg == 1 && c[j] >= 0.0 {
            rows.iter().find(|&&(_, v)| v < 0.0).map(|&(i, _)| i)
        } else {
            None
        };
        let Some(r) = pick else { continue };
        if used_as_witness[r] {
            continue;
        }
        // 診断 `ENOMOTO_DEBUG_DFI_MAX=k`: プロセス全体で上げる行を k 本までにする (誤りの切り分け用)
        if let Some(k) = env_str!("ENOMOTO_DEBUG_DFI_MAX").and_then(|v| v.parse::<usize>().ok()) {
            static DFI_CNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            if DFI_CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) >= k {
                continue;
            }
        }
        if env_str!("ENOMOTO_DEBUG_DUALFREE_INEQ").is_some() {
            eprintln!("DFI: col {j} [{}, {}] implied [{lo}, {hi}] c {} -> row {r} {:?} <= {} ; rows {:?}", lb[j], ub[j], c[j], real_rows[r], real_rhs[r], rows);
        }
        promoted[r] = true;
        out.push(r);
        for &(i, _) in rows {
            if i != r {
                used_as_witness[i] = true;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 時刻の列 t (費用 0) が、大きい M の行 (係数 +1) と差の不等式 2 本 (係数 −1) に現れる形。
    /// 境界は行から導ける値ちょうど (非厳密)。
    #[test]
    fn big_m_row_is_promoted() {
        // 列: 0 = t, 1 = s (連続 [0, 10]), 2 = z (0-1)
        // 行 0: t − s + 5 z ≤ 5        → t ≤ 5 + s − 5z ≤ 15 (上限 15 は導ける)
        // 行 1: −t + s ≤ 0             → t ≥ s ≥ 0 (下限 0 は導ける)
        // 行 2: −t ≤ 3
        let rows = vec![vec![(0, 1.0), (1, -1.0), (2, 5.0)], vec![(0, -1.0), (1, 1.0)], vec![(0, -1.0)]];
        let rhs = vec![5.0, 0.0, 3.0];
        let c = vec![0.0, 1.0, -1.0];
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![15.0, 10.0, 1.0];
        let is_int = vec![false, false, true];
        let r = find_dual_free_inequalities(3, &[], &rows, &rhs, &c, &lb, &ub, Some(&is_int));
        assert_eq!(r, vec![0]);
    }

    /// 境界が導けない (行から導ける上限 15 より厳しい 14) なら上げない。
    #[test]
    fn strict_bound_blocks() {
        let rows = vec![vec![(0, 1.0), (1, -1.0), (2, 5.0)], vec![(0, -1.0), (1, 1.0)]];
        let rhs = vec![5.0, 0.0];
        let c = vec![0.0, 1.0, -1.0];
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![14.0, 10.0, 1.0];
        let r = find_dual_free_inequalities(3, &[], &rows, &rhs, &c, &lb, &ub, Some(&[false, false, true]));
        assert!(r.is_empty());
    }

    /// 費用の符号が合わない (正の行で c > 0) なら上げない。整数列も対象外。等式に現れる列も対象外。
    #[test]
    fn sign_and_kind_block() {
        let rows = vec![vec![(0, 1.0), (1, -1.0)], vec![(0, -1.0), (1, 1.0)]];
        let rhs = vec![0.0, 0.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 10.0];
        // 列 0 は費用 +1: 正の行 (行 0) は候補外、負の行 (行 1) は他が正なので上げる (行 0 は根拠)。
        // 列 1 はすでに上げた行 1 に現れるので選ばない。
        let r = find_dual_free_inequalities(2, &[], &rows, &rhs, &[1.0, 1.0], &lb, &ub, None);
        assert_eq!(r, vec![1]);
        let r = find_dual_free_inequalities(2, &[], &rows, &rhs, &[1.0, 1.0], &lb, &ub, Some(&[true, true]));
        assert!(r.is_empty());
        let a = vec![vec![(0, 1.0), (1, 1.0)]];
        let r = find_dual_free_inequalities(2, &a, &rows, &rhs, &[1.0, 1.0], &lb, &ub, None);
        assert!(r.is_empty());
    }
}
