//! DualPropagate (双対側の境界伝播): [`propagate`](super::propagate) の行活動値による境界強化を
//! 制約系の「転置」に対してもう一度実行し、各実制約行の双対変数 (シャドウプライス) の含意境界を得る。
//! そこから次の 2 種類の縮約を読み出す (HiGHS `HPresolve.cpp` の `isDualImpliedFree` /
//! `updateRowDualImpliedBounds` / `isDominatedCol` と同じ考え方)。
//!
//! ## 仕組み
//!
//! `min c^T x`, `A x = b` (双対 `lambda`, 符号自由), `G x <= h` (双対 `mu >= 0`) に対し、
//! 列 `j` の被約費用は `r_j := c_j + sum_i d_i * M_ij` (`d_i` は行 `i` の双対, `M_ij` は格納済み係数)。
//! 相補性より、変数が存在しない境界に張り付くことはないので:
//!
//! - `ub_j = +inf` なら `r_j >= 0` が強制され、`sum_i d_i * M_ij >= -c_j`
//! - `lb_j = -inf` なら `r_j <= 0` が強制され、`sum_i d_i * M_ij <= -c_j`
//! - 両側有限なら何も得られないので飛ばす / 両側無限 (自由変数) なら両方が成り立ち等式になる
//!
//! これらを双対変数 `d_i` を「変数」とする新しい `<=` 系 (t 行) として組み、`G` 行の双対には
//! 符号制約 `-mu_i <= 0` を箱行として加え、[`propagate::propagate`] 系の関数で伝播する。
//!
//! ## 縮約 1: 含意等式
//!
//! 伝播後の `mu_i` の下限が正なら、どの双対実行可能点でも `mu_i > 0` なので、相補性により
//! その `G` 行はすべての最適解で等号成立する。この行を `A` 側へ移せば、既存の
//! `doubleton`/`colsingleton`/`rowsingleton` がそのまま利用できる。
//!
//! 「無限」は文字通り `±inf` のみを意味する。境界強化で元の境界より内側になっただけの列は
//! t 行を作らない (健全でないため。経緯は履歴メモ参照)。固定済み列 (`lb[j] == ub[j]`) は定数なので除外する。
//!
//! ## 縮約 2: 列固定 (HiGHS の「dominated column」)
//!
//! 同じ双対の箱 `[dlo_i, dhi_i]` から、区間演算で各列の被約費用の範囲 `[rlo_j, rhi_j]` を求める。
//! `rlo_j > 0` なら `x_j` は下限に、`rhi_j < 0` なら上限に固定できる (追加の伝播は不要)。
//! Andersen & Andersen の列同士の比較 ([`super::dominatedcol`]) とは別の手法で、名前が似ているのは偶然。
//!
//! どちらも入力のスナップショットから 1 パスで決め、連鎖はしない (次ラウンドで拾われる)。

use crate::presolve::propagate;
use crate::sparse::{FaerCsr, CscMat, csr_from_rows, csr_row_iter};
use crate::params::presolve::TOL;

/// [`run`] が 1 回の双対伝播から読み出す 2 種類の縮約。
#[derive(Default)]
pub struct DualReductions {
    /// すべての最適解で等号成立が証明された `real_g_rows` の行番号
    pub implied_equalities: Vec<usize>,
    /// 固定する `(列番号, 境界値)`。値は被約費用の符号に応じた `lb[j]` か `ub[j]` のどちらか
    pub fixed_columns: Vec<(usize, f64)>,
}

/// `c`・`a`・`real_g_rows` から双対実行可能性の系を組んで伝播し、得られた双対の箱から
/// 2 種類の縮約 ([`DualReductions`]) を読み出す。
///
/// - `n`: 列数
/// - `a`: 等式行 (双対は符号自由として伝播に加わるが、行昇格の候補にはならない)
/// - `real_g_rows`: `G` の実制約行 (`<=` 正規化済み。行昇格の候補)
/// - `c`: 目的関数係数
/// - `lb`, `ub`: 現ラウンドの境界 (固定済み列の判定と、文字通りの `±inf` 判定に使う)
/// - `_orig_lb`, `_orig_ub`: 未使用 (呼び出し側を変えないために残している。経緯は履歴メモ参照)
/// - `passes`: 転置系に対する境界伝播のパス数
pub fn propagate_dual_bounds(n: usize, a: &FaerCsr, real_g_rows: &[Vec<(usize, f64)>], c: &[f64], lb: &[f64], ub: &[f64], _orig_lb: &[f64], _orig_ub: &[f64], passes: usize) -> DualReductions {
    let ar = a.as_ref();
    // 双対変数の番号付け: A 行が 0..num_a、G 実制約行が num_a..num_a+num_g
    let num_a = ar.nrows();
    let num_g = real_g_rows.len();
    let num_duals = num_a + num_g;
    if num_duals == 0 {
        return DualReductions::default();
    }

    // 転置: `col_terms.col(j)` は列 j に係数を持つ (双対変数番号, 係数) の組をすべて保持する。
    // 列ごとの Vec を作らず、圧縮列形式へ直接流し込む (`CscMat::from_entry_stream` 参照)。
    let col_terms = CscMat::from_entry_stream(num_duals, n, |emit| {
        for i in 0..num_a {
            for (j, v) in csr_row_iter(a, i) {
                if v != 0.0 {
                    emit(i, j, v);
                }
            }
        }
        for (gi, row) in real_g_rows.iter().enumerate() {
            for &(j, v) in row {
                if v != 0.0 {
                    emit(num_a + gi, j, v);
                }
            }
        }
    });

    // 転置系の `<=` 行 (双対変数についての制約) とその右辺
    let mut t_rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut t_h: Vec<f64> = Vec::new();

    for j in 0..n {
        if col_terms.col(j).is_empty() || lb[j] == ub[j] {
            continue;
        }
        // `ub_j = +inf` なら `r_j >= 0` が強制される: `-sum d_i*M_ij <= c_j`。
        // (境界強化で `ub` が元より内側になっただけの場合は t 行を作らない。理由は履歴メモ参照。)
        if ub[j] == f64::INFINITY {
            t_rows.push(col_terms.col(j).iter().map(|&(i, v)| (i, -v)).collect());
            t_h.push(c[j]);
        }
        // 下側の対称な場合: `lb_j = -inf` なら `sum d_i*M_ij <= -c_j`。
        if lb[j] == f64::NEG_INFINITY {
            t_rows.push(col_terms.col(j).to_vec());
            t_h.push(-c[j]);
        }
    }

    // G 行の双対には符号制約 `mu_i >= 0` (`-mu_i <= 0`) を箱行として加える。A 行の双対は符号自由。
    //
    // 列由来の t 行が一つもなければ、系は符号制約行だけなので、伝播を経由せずに双対の箱を直接作る
    // (`propagate` が返すのと同じ値 `lb = 0.0 / -1.0`、それ以外は非有界)。
    let result = if t_rows.is_empty() {
        let mut lb = vec![f64::NEG_INFINITY; num_duals];
        for gi in 0..num_g {
            lb[num_a + gi] = 0.0 / -1.0;
        }
        propagate::PropagateSplit { lb, ub: vec![f64::INFINITY; num_duals], real_rows: Vec::new(), real_rhs: Vec::new(), infeasible: false }
    } else {
        for gi in 0..num_g {
            t_rows.push(vec![(num_a + gi, -1.0)]);
            t_h.push(0.0);
        }
        let t_g = csr_from_rows(&t_rows, num_duals);
        propagate::propagate_without_g_rebuild(num_duals, &t_g, &t_h, passes)
    };
    if result.infeasible {
        // 双対系の一部だけから主問題の非有界・実行不能を結論するのは強すぎるので、
        // ここでは「何も見つからなかった」として返し、判定は他の処理 (主側の propagate やソルバ本体) に任せる。
        return DualReductions::default();
    }

    let implied_equalities = (0..num_g).filter(|&gi| result.lb[num_a + gi] > TOL).collect();

    // 列固定: 実制約行に現れ、未固定の各列について、伝播済みの双対の箱上で区間演算により
    // 被約費用 r_j の範囲 [rlo, rhi] を求める。負係数では区間の端が入れ替わる
    // (`col_terms` に 0 係数はないので場合分けは 2 通り)。
    let mut fixed_columns = Vec::new();
    for j in 0..n {
        let terms = col_terms.col(j);
        if terms.is_empty() || lb[j] == ub[j] {
            continue;
        }
        let mut rlo = c[j];
        let mut rhi = c[j];
        for &(i, v) in terms {
            let (dlo, dhi) = (result.lb[i], result.ub[i]);
            let (tlo, thi) = if v > 0.0 { (v * dlo, v * dhi) } else { (v * dhi, v * dlo) };
            rlo += tlo;
            rhi += thi;
        }
        if rlo.is_nan() || rhi.is_nan() {
            // `inf - inf` などで NaN になった場合は何も証明できなかったとみなす。
            continue;
        }
        // 箱全体で r_j > 0 なら下限へ、r_j < 0 なら上限へ固定する (固定先の境界が有限の場合のみ)。
        if rlo > TOL && lb[j] > f64::NEG_INFINITY {
            fixed_columns.push((j, lb[j]));
        } else if rhi < -TOL && ub[j] < f64::INFINITY {
            fixed_columns.push((j, ub[j]));
        }
    }

    DualReductions { implied_equalities, fixed_columns }
}

/// [`run`] の含意等式 (行昇格) の結果だけを返す薄いラッパー。現在はこのモジュールのテストからのみ使用。
pub fn find_implied_equalities(n: usize, a: &FaerCsr, real_g_rows: &[Vec<(usize, f64)>], c: &[f64], lb: &[f64], ub: &[f64], orig_lb: &[f64], orig_ub: &[f64], passes: usize) -> Vec<usize> {
    propagate_dual_bounds(n, a, real_g_rows, c, lb, ub, orig_lb, orig_ub, passes).implied_equalities
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    #[test]
    fn free_hub_column_forces_its_only_inequality_tight() {
        // minimize -x1 (x1 free: [0,+inf)) s.t. x0 + x1 <= 10, x0 in
        // [0,10]. x1's own reduced cost must be 0 (cost -1, and x1 has no
        // upper bound so its dual-row is genuinely an equality: -1 + mu*1
        // = 0 => mu = 1 > 0), so the row is proven tight in every optimal
        // solution even though nothing about its own primal activity
        // range (x0+x1 can range from 0 to +inf, no §3.1 forcing-row
        // shape at all) would show that.
        let n = 2;
        let a = csr_from_rows(&[], n);
        let real_g_rows = vec![vec![(0usize, 1.0), (1usize, 1.0)]];
        let c = vec![0.0, -1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, f64::INFINITY];
        let implied = find_implied_equalities(n, &a, &real_g_rows, &c, &lb, &ub, &lb, &ub, 2);
        assert_eq!(implied, vec![0]);
    }

    #[test]
    fn bounded_hub_column_proves_nothing() {
        // Same shape, but x1 now has a finite upper bound too — neither
        // of its own bounds is infinite, so it contributes no dual
        // constraint at all, and nothing here can prove the row tight.
        let n = 2;
        let a = csr_from_rows(&[], n);
        let real_g_rows = vec![vec![(0usize, 1.0), (1usize, 1.0)]];
        let c = vec![0.0, -1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 10.0];
        let implied = find_implied_equalities(n, &a, &real_g_rows, &c, &lb, &ub, &lb, &ub, 2);
        assert!(implied.is_empty());
    }

    #[test]
    fn unfavorable_cost_sign_alone_proves_nothing() {
        // x1 is free ([0,+inf)) but its cost now favors making the row
        // *slack* (c1 = +1, minimizing wants x1 small, i.e. 0), so mu is
        // forced to 0, not away from it — dualfix's own favorable-sign
        // case would already fix x1 = 0 upstream of this pass; here,
        // taken in isolation, this pass must correctly find nothing.
        let n = 2;
        let a = csr_from_rows(&[], n);
        let real_g_rows = vec![vec![(0usize, 1.0), (1usize, 1.0)]];
        let c = vec![0.0, 1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, f64::INFINITY];
        let implied = find_implied_equalities(n, &a, &real_g_rows, &c, &lb, &ub, &lb, &ub, 2);
        assert!(implied.is_empty());
    }

    #[test]
    fn two_free_columns_sharing_a_row_still_forces_it() {
        // Neither column alone has a *strictly* one-sided-favorable cost
        // (both are free, i.e. both bounds infinite on the unconstrained
        // side, each pinning the row's dual to its own exact cost value)
        // — the row is forced tight regardless, and both free columns'
        // own equalities must be mutually consistent with the same mu.
        let n = 2;
        let a = csr_from_rows(&[], n);
        let real_g_rows = vec![vec![(0usize, 1.0), (1usize, 1.0)]];
        let c = vec![-2.0, -2.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY];
        let implied = find_implied_equalities(n, &a, &real_g_rows, &c, &lb, &ub, &lb, &ub, 2);
        assert_eq!(implied, vec![0]);
    }

    #[test]
    fn no_real_rows_is_a_no_op() {
        let n = 1;
        let a = csr_from_rows(&[], n);
        let real_g_rows: Vec<Vec<(usize, f64)>> = Vec::new();
        let c = vec![0.0];
        let lb = vec![0.0];
        let ub = vec![1.0];
        assert!(find_implied_equalities(n, &a, &real_g_rows, &c, &lb, &ub, &lb, &ub, 2).is_empty());
    }
}
