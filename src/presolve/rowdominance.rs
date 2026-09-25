//! DominatedRows (支配行の削除, Andersen & Andersen 1995 §3.1)。[`dominatedcol`](super::dominatedcol) の行版。
//! **現在は未使用** (パイプラインから呼ばれていない。理由は履歴メモ参照)。
//!
//! 2 本の `<=` 行 `p` (被支配, 削除候補) と `q` (支配, 残す) について、すべての列 `k` で
//!
//! - `G[q,k] == G[p,k]` (`TOL` 以内)、または
//! - `G[q,k] >= G[p,k]` かつ `lb[k] >= -TOL` (`x_k` が非負)
//!
//! が成り立ち、さらに `h[q] <= h[p]` なら、`G[p,:].x <= G[q,:].x <= h[q] <= h[p]` となり、
//! 行 `q` が成り立てば行 `p` も必ず成り立つので `p` を削除できる (実行可能領域は変わらない)。
//! 等式行に現れる列を含む行、および長さ 1 の行 (箱境界行) は対象外。
//!
//! 比例関係にある行 ([`parallelrows`](super::parallelrows)・`redundancy::reduce_inequalities` の担当) ではなく、
//! 係数が点ごとに順序付けられているだけの行の組を扱う。
//!
//! **候補探索**: 全行対 `O(m^2)` を避け、各行について「最も出現行数の少ない列 (anchor)」を共有する行とだけ比較する。
//!
//! **1 パス・非連鎖**: 支配関係は入力のまま一度に計算する。他の行を支配する行はこの呼び出しでは削除しない
//! (削除の根拠となる行が必ず残るようにするため)。連鎖 `p -> q -> r` は次の呼び出しで解消される。

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashSet;

use crate::sparse::{FaerCsr, csr_row_iter};
use crate::params::presolve::TOL;

/// この呼び出しで安全に削除できる被支配行の番号 (`g` の行番号, 昇順) を返す。
/// 呼び出し側はこれらの行を `(G, h)` から除くだけでよい (境界の固定や実行不能判定はない)。
///
/// - `n`: 列数
/// - `a`: 等式行列 (ここに現れる列を含む行は対象外)
/// - `g`, `h`: 不等式制約 `G x <= h`
/// - `lb`: 変数の下限 (非負性の判定に使用)
pub fn find_dominated_rows(n: usize, a: &FaerCsr, g: &FaerCsr, h: &[f64], lb: &[f64]) -> Vec<usize> {
    let ar = a.as_ref();
    // in_equality[j]: 列 j が等式行に現れるか
    let mut in_equality = vec![false; n];
    for i in 0..ar.nrows() {
        for (j, v) in csr_row_iter(a, i) {
            if v != 0.0 {
                in_equality[j] = true;
            }
        }
    }

    let gr = g.as_ref();
    let m = gr.nrows();
    // 候補は複数変数の行のみ (長さ 1 の行は変数の箱境界行で、他所で意味を持つので削除しない)。
    // rows[i]: 行 i の (列 -> 係数)。長さ 1 の行は空、等式行の列を含む行は番兵 usize::MAX のみを持つ。
    let rows: Vec<BTreeMap<usize, f64>> = (0..m)
        .map(|i| {
            let raw: Vec<(usize, f64)> = csr_row_iter(g, i).filter(|&(_, v)| v != 0.0).collect();
            let mut row = BTreeMap::new();
            if raw.len() >= 2 {
                for (j, v) in raw {
                    if in_equality[j] {
                        // 等式行の列を含む行は対象外。番兵を入れて後で除外する。
                        row.clear();
                        row.insert(usize::MAX, 0.0);
                        break;
                    }
                    row.insert(j, v);
                }
            }
            row
        })
        .collect();

    // 比較対象となる候補行
    let candidates: Vec<usize> = (0..m).filter(|&i| !rows[i].is_empty() && !rows[i].contains_key(&usize::MAX)).collect();
    if candidates.len() < 2 {
        return Vec::new();
    }

    // col_candidates[k]: 列 k を含む候補行の集合
    let mut col_candidates: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    for &i in &candidates {
        for &k in rows[i].keys() {
            col_candidates[k].insert(i);
        }
    }

    // `q` が `p` を支配するなら true (モジュール docs の列ごとの判定 + 右辺の比較)。
    let dominates = |q: usize, p: usize| -> bool {
        for (&k, &gp) in &rows[p] {
            let gq = rows[q].get(&k).copied().unwrap_or(0.0);
            if (gq - gp).abs() <= TOL {
                continue;
            }
            if gq + TOL >= gp && lb[k] >= -TOL {
                continue;
            }
            return false;
        }
        for (&k, &gq) in &rows[q] {
            if rows[p].contains_key(&k) {
                continue; // 上のループで判定済み
            }
            let gp = 0.0;
            if (gq - gp).abs() <= TOL {
                continue;
            }
            if gq + TOL >= gp && lb[k] >= -TOL {
                continue;
            }
            return false;
        }
        h[q] <= h[p] + TOL
    };

    // 候補対の列挙: 各行について最も出現の少ない列 (anchor) を共有する行とだけ組にする。
    // これで見逃す支配関係は後のラウンドに任せる。pairs は (小さい番号, 大きい番号) の決定的な集合。
    let mut pairs: BTreeSet<(usize, usize)> = BTreeSet::new();
    for &i in &candidates {
        let anchor = *rows[i].keys().min_by_key(|&&k| col_candidates[k].len()).unwrap();
        for &j in &col_candidates[anchor] {
            if j != i {
                pairs.insert((i.min(j), i.max(j)));
            }
        }
    }

    // dominated_by[p]: p を支配する行 q の一覧 / ever_dominates: 何らかの行を支配した行の集合
    let mut dominated_by: Vec<Vec<usize>> = vec![Vec::new(); m];
    let mut ever_dominates: HashSet<usize> = HashSet::new();
    for (x, y) in pairs {
        if dominates(x, y) {
            dominated_by[y].push(x);
            ever_dominates.insert(x);
        }
        if dominates(y, x) {
            dominated_by[x].push(y);
            ever_dominates.insert(y);
        }
    }

    // 他の行を支配した行はこの呼び出しでは削除しない。したがって dominated_by[p] の q は必ず残り、
    // p を削除する根拠として常に安全である。
    let mut drop = Vec::new();
    for &p in &candidates {
        if ever_dominates.contains(&p) {
            continue; // 他の行の削除根拠なので、この呼び出しでは削除しない
        }
        if !dominated_by[p].is_empty() {
            drop.push(p);
        }
    }
    drop.sort_unstable();
    drop
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    #[test]
    fn drops_the_row_dominated_by_a_tighter_pointwise_larger_row() {
        // row0: x0 + x1 <= 5 (dominated)
        // row1: 2*x0 + 2*x1 <= 3 (dominating: coeffs pointwise >=, bound tighter)
        // x0,x1 >= 0, so row1 holding forces 2*(x0+x1) <= 3 -> x0+x1 <= 1.5 <= 5.
        let a = csr_from_rows(&[], 2);
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0)]], 2);
        let h = vec![5.0, 3.0];
        let lb = vec![0.0, 0.0];

        let dropped = find_dominated_rows(2, &a, &g, &h, &lb);
        assert_eq!(dropped, vec![0]);
    }

    #[test]
    fn keeps_both_when_a_column_can_go_negative() {
        // Same coefficients as above, but x1 is free (lb < 0) -- the
        // pointwise argument for column 1 no longer holds, so neither row
        // may be dropped via this rule.
        let a = csr_from_rows(&[], 2);
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0)]], 2);
        let h = vec![5.0, 3.0];
        let lb = vec![0.0, -1.0];

        let dropped = find_dominated_rows(2, &a, &g, &h, &lb);
        assert!(dropped.is_empty());
    }

    #[test]
    fn keeps_both_when_bounds_disagree_with_coefficient_order() {
        // row1's coefficients pointwise dominate row0's, but its own bound
        // is looser (12 > 5), so row1 holding does NOT force row0 -- no
        // domination either way.
        let a = csr_from_rows(&[], 2);
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0)]], 2);
        let h = vec![5.0, 12.0];
        let lb = vec![0.0, 0.0];

        let dropped = find_dominated_rows(2, &a, &g, &h, &lb);
        assert!(dropped.is_empty());
    }

    #[test]
    fn excludes_rows_touching_an_equality_locked_column() {
        let a = csr_from_rows(&[vec![(1, 1.0)]], 2);
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0)]], 2);
        let h = vec![5.0, 3.0];
        let lb = vec![0.0, 0.0];

        let dropped = find_dominated_rows(2, &a, &g, &h, &lb);
        assert!(dropped.is_empty());
    }

    #[test]
    fn leaves_box_bound_rows_untouched() {
        let a = csr_from_rows(&[], 1);
        let g = csr_from_rows(&[vec![(0, 1.0)], vec![(0, 2.0)]], 1);
        let h = vec![5.0, 3.0];
        let lb = vec![0.0];

        let dropped = find_dominated_rows(1, &a, &g, &h, &lb);
        assert!(dropped.is_empty());
    }

    #[test]
    fn a_row_used_to_justify_a_drop_is_never_itself_dropped_in_the_same_call() {
        // row0: x0+x1 <= 10 (dominated by row1)
        // row1: 2*x0+2*x1 <= 6 (dominates row0; also dominated by row2)
        // row2: 3*x0+3*x1 <= 3 (dominates row1)
        // row1 is cited to justify dropping row0, so row1 itself must
        // survive this call even though row2 also dominates it -- that
        // second link (row1 dominated by row2) is left for the pipeline's
        // next call to resolve, once row1's own removal can no longer
        // invalidate anything else's justification from this pass.
        let a = csr_from_rows(&[], 2);
        let g = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0), (1, 2.0)], vec![(0, 3.0), (1, 3.0)]], 2);
        let h = vec![10.0, 6.0, 3.0];
        let lb = vec![0.0, 0.0];

        let dropped = find_dominated_rows(2, &a, &g, &h, &lb);
        assert_eq!(dropped, vec![0], "only row0 is safe to drop this call -- its dominator row1 survives since row1 is never dropped while justifying row0");
    }
}
