//! ParallelRows (平行行の併合, 逆符号側。Andersen & Andersen 1995 / PaPILO "ParallelRows",
//! Achterberg et al. 2019 §4.4)。**現在は未使用** (パイプラインから呼ばれていない。理由は履歴メモ参照)。
//!
//! 正の定数倍の関係にある `<=` 行の組は [`redundancy::reduce_inequalities`](super::redundancy::reduce_inequalities)
//! が既に処理するので、本モジュールは残りの「負の定数倍」の組 `a.x <= h_i` と `(-s*a).x <= h_j` (`s > 0`)
//! を扱う。これは両側制約 `-h_j/s <= a.x <= h_i` と書けるので:
//!
//! - `-h_j/s > h_i` (`TOL` を超える): 実行不能
//! - `-h_j/s == h_i` (`TOL` 以内): `a.x = h_i` の等式行 1 本に併合して `(A, b)` に追加し、元の 2 行を `G` から削除
//! - それ以外: 余裕があるので何もしない (さらなる強化は [`propagate`](super::propagate) の担当)
//!
//! 候補は、各行を「符号付きの」先頭係数で割った署名で分類して探す (互いに符号反転な行が同じ署名になる)。
//! 長さ 1 の行 (変数自身の箱境界行) は対象外。
//!
//! 入力のスナップショットから 1 パスで決める。1 回の併合に使った行はその呼び出し中は再利用しない。

use std::collections::HashMap;

use crate::sparse::{FaerCsr, csr_from_rows, csr_rows};
use crate::params::presolve::TOL;

/// [`merge_parallel_rows`] の結果。`infeasible` が true のときは他のフィールドは入力の複製で意味を持たない
/// (呼び出し側は先に `infeasible` を確認する)。
pub struct ParallelRowsResult {
    /// 新たに見つかった等式行を末尾に追加した等式行列
    pub a: FaerCsr,
    /// `a` に対応する右辺
    pub b: Vec<f64>,
    /// 併合した行の組を両方とも除いた不等式行列
    pub g: FaerCsr,
    /// `g` に対応する右辺
    pub h: Vec<f64>,
    /// 逆符号の組の境界が交差し、実行不能と判明したか
    pub infeasible: bool,
}

/// 逆符号の平行な `<=` 行の組を探し、等式への併合または実行不能の検出を行う。
///
/// - `a`, `b`: 等式制約 `A x = b`
/// - `g`, `h`: 不等式制約 `G x <= h`
/// - `n`: 列数
pub fn merge_parallel_rows(a: &FaerCsr, b: &[f64], g: &FaerCsr, h: &[f64], n: usize) -> ParallelRowsResult {
    let gr = g.as_ref();
    let m = gr.nrows();
    // G の各行 (列番号, 係数)
    let rows: Vec<Vec<(usize, f64)>> = csr_rows(g);

    // 複数変数の行を、符号付き先頭係数で割った署名で分類する (先頭が常に +1 になるので、
    // 符号反転した行同士も同じ署名になる)。同じ群の 2 行は符号を除いて比例しており、
    // 元の先頭係数の符号で同符号か逆符号かを区別する。
    let mut groups: HashMap<Vec<(usize, u64)>, Vec<usize>> = HashMap::new();
    for (i, row) in rows.iter().enumerate() {
        if row.len() < 2 {
            continue;
        }
        let inv = 1.0 / row[0].1;
        let sig: Vec<(usize, u64)> = row.iter().map(|&(j, v)| (j, (v * inv).to_bits())).collect();
        groups.entry(sig).or_default().push(i);
    }

    // used[i]: 行 i がこの呼び出しで既に併合に使われたか / drop_g[i]: 行 i を G から削除するか
    let mut used = vec![false; m];
    let mut drop_g: Vec<bool> = vec![false; m];
    // 新たに追加する等式行とその右辺
    let mut new_eq_rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut new_eq_b: Vec<f64> = Vec::new();
    let mut infeasible = false;

    // 再現性のため、HashMap の走査順に依存せず、群を先頭行番号順に、群内の行を昇順に試す。
    let mut group_indices: Vec<&Vec<usize>> = groups.values().collect();
    group_indices.sort_by_key(|v| v[0]);

    'groups: for idx_list in group_indices {
        let mut members = idx_list.clone();
        members.sort_unstable();
        for a_pos in 0..members.len() {
            let i = members[a_pos];
            if used[i] {
                continue;
            }
            for b_pos in (a_pos + 1)..members.len() {
                let j = members[b_pos];
                if used[j] {
                    continue;
                }
                // 行 i, j の元の先頭係数
                let vi = rows[i][0].1;
                let vj = rows[j][0].1;
                if vi.signum() == vj.signum() {
                    // 同符号の重複は上流の `redundancy::reduce_inequalities` の担当なので飛ばす。
                    continue;
                }
                // rows[j] = -s * rows[i] (s > 0) なので、行 j は `rows[i].x >= -h[j]/s` と書ける。
                // lower, upper: rows[i].x の下限・上限
                let s = -(vj / vi);
                let lower = -h[j] / s;
                let upper = h[i];
                let tol = TOL * (1.0 + upper.abs().max(lower.abs()));
                if lower > upper + tol {
                    infeasible = true;
                    break 'groups;
                } else if (lower - upper).abs() <= tol {
                    used[i] = true;
                    used[j] = true;
                    drop_g[i] = true;
                    drop_g[j] = true;
                    new_eq_rows.push(rows[i].clone());
                    new_eq_b.push(upper);
                    break; // 行 i は使用済みになったので次の i へ
                }
                // 余裕が残る場合は両行ともそのままにし、i の別の相手を探す。
            }
        }
    }

    if infeasible {
        return ParallelRowsResult { a: a.clone(), b: b.to_vec(), g: g.clone(), h: h.to_vec(), infeasible: true };
    }

    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);
    let mut new_b = b.to_vec();
    a_rows.extend(new_eq_rows);
    new_b.extend(new_eq_b);

    let mut g_rows = Vec::with_capacity(m);
    let mut new_h = Vec::with_capacity(m);
    for i in 0..m {
        if drop_g[i] {
            continue;
        }
        g_rows.push(rows[i].clone());
        new_h.push(h[i]);
    }

    ParallelRowsResult { a: csr_from_rows(&a_rows, n), b: new_b, g: csr_from_rows(&g_rows, n), h: new_h, infeasible: false }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    #[test]
    fn merges_a_tight_opposite_pair_into_an_equality() {
        // x0 + 2*x1 <= 10 and -x0 - 2*x1 <= -10 (i.e. x0+2x1 >= 10) pin
        // x0+2x1 exactly to 10 -- must collapse to one equality row.
        let a = csr_from_rows(&[], 2);
        let b: Vec<f64> = vec![];
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 2.0)], vec![(0, -1.0), (1, -2.0)]], 2);
        let h = vec![10.0, -10.0];

        let r = merge_parallel_rows(&a, &b, &g, &h, 2);
        assert!(!r.infeasible);
        assert_eq!(r.g.as_ref().nrows(), 0, "both inequality rows must be dropped");
        assert_eq!(r.a.as_ref().nrows(), 1, "exactly one equality row must be added");
        assert!((r.b[0] - 10.0).abs() < 1e-9);
    }

    #[test]
    fn detects_infeasibility_when_the_bounds_cross() {
        // x0 + x1 <= 5 and x0 + x1 >= 8 (via -x0-x1 <= -8) -- no x can
        // satisfy both.
        let a = csr_from_rows(&[], 2);
        let b: Vec<f64> = vec![];
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, -1.0), (1, -1.0)]], 2);
        let h = vec![5.0, -8.0];

        let r = merge_parallel_rows(&a, &b, &g, &h, 2);
        assert!(r.infeasible);
    }

    #[test]
    fn leaves_a_genuine_range_untouched() {
        // x0 + x1 <= 10 and x0 + x1 >= 2 (via -x0-x1 <= -2) -- real slack,
        // no row-count reduction possible here.
        let a = csr_from_rows(&[], 2);
        let b: Vec<f64> = vec![];
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, -1.0), (1, -1.0)]], 2);
        let h = vec![10.0, -2.0];

        let r = merge_parallel_rows(&a, &b, &g, &h, 2);
        assert!(!r.infeasible);
        assert_eq!(r.g.as_ref().nrows(), 2);
        assert_eq!(r.a.as_ref().nrows(), 0);
    }

    #[test]
    fn leaves_box_bound_rows_untouched() {
        // Two length-1 rows (a variable's own lb/ub, per build_a_g) must
        // never be merged into an equality here, even when they happen to
        // pin the variable to a point -- that's already represented
        // directly as a bound, not a real row pair to collapse.
        let a = csr_from_rows(&[], 1);
        let b: Vec<f64> = vec![];
        let g = csr_from_rows(&[vec![(0, 1.0)], vec![(0, -1.0)]], 1);
        let h = vec![5.0, -5.0];

        let r = merge_parallel_rows(&a, &b, &g, &h, 1);
        assert!(!r.infeasible);
        assert_eq!(r.g.as_ref().nrows(), 2);
        assert_eq!(r.a.as_ref().nrows(), 0);
    }

    #[test]
    fn ignores_same_sign_duplicates_leaving_them_for_reduce_inequalities() {
        // Two positive multiples of the same row -- this module's job ends
        // at opposite-sign pairs, so both survive untouched here.
        let a = csr_from_rows(&[], 2);
        let b: Vec<f64> = vec![];
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0)]], 2);
        let h = vec![5.0, 12.0];

        let r = merge_parallel_rows(&a, &b, &g, &h, 2);
        assert!(!r.infeasible);
        assert_eq!(r.g.as_ref().nrows(), 2);
    }
}
