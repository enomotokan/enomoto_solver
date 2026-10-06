//! ParallelColumns (平行列の併合, HiGHS の "Parallel rows and columns" の列側 /
//! Andersen & Andersen 1995 の行重複除去 [`parallelrows`](super::parallelrows) の転置版)。
//!
//! 全実制約行にわたって `A[:,j] = s * A[:,k]` (非零スカラー `s`) となる 2 列は、各行の活動値を
//! 常に同じ比率で動かすので、併合変数 `z = x_k + s*x_j` 1 本に置き換えられる。`z` は列 `k` の位置に置き、
//! `k` の境界を `z` の到達範囲に広げる。列 `j` は全行と目的関数から取り除く。
//!
//! ## 健全性の条件
//!
//! `x_k = z - s*x_j` を代入すると目的関数は `c_k*z + (c_j - s*c_k)*x_j` になる。
//! `c_j == s*c_k` (相対 `TOL` で判定) のときに限り併合する。このとき `z` の分け方によらず目的値が
//! 同じなので、どの分割も最適であり、[`Substitution::apply`] で代数的に分割を復元できる。
//! 目的関数が一方を厳密に好む場合は [`dominatedcol`](super::dominatedcol) の担当 (現在は無効)。
//!
//! ## 対象範囲
//!
//! - 列の下限は有限でなければならない (復元式の基準点になるため)。上限は有限でも `+inf` でもよい。
//! - 自由列 (両側無限) は [`super::freevar`] の担当なので対象外。
//! - 既に固定された列 (`lb == ub`) は飛ばす。
//! - 併合で `kept` の下限が `-inf` になる (自由列ができる) 組は併合しない。
//!
//! 既定で有効 (`ENOMOTO_DISABLE_PARALLELCOLS` で無効化可)。計測結果・不具合修正の経緯は履歴メモ参照。
//!
//! ## 候補探索
//!
//! 各列を、その列の最初の (符号付き) 要素で割って正規化した「署名」で分類する (`parallelrows` と同じ方法)。
//! 同じ署名の群では番号最小の列を併合先 `kept` とし、他の条件を満たす列を番号順にすべて畳み込む。
//! 各 [`Substitution`] はその時点での `kept` の下限 (`kept_lb`) を記録するので、ポストソルブは
//! 逆順に 1 層ずつ正しく分解できる。1 呼び出しで群全体を吸収する (大きな群を外側ラウンド数に依存せず潰すため)。

use crate::sparse::{FaerCsr, CscMat, csr_rows};
use crate::params::presolve::TOL;

/// 1 回の列併合のポストソルブ記録。併合変数 `z = x[kept]` の値から `x[var]` と `x[kept]` の
/// 本来の値を復元する。
///
/// 書き込み先が 2 か所で、しかも `kept` は入力でもあるため、
/// [`super::colsingleton::Substitution`] の単一の線形式では表せない。
pub struct Substitution {
    /// 消去された列
    pub var: usize,
    /// 併合先の列 (`z` を保持する)
    pub kept: usize,
    /// 比率 `s` (`A[:,var] = s * A[:,kept]`、`z = x_kept + s * x_var`)
    pub s: f64,
    /// 併合時点での `var` の下限
    pub var_lb: f64,
    /// 併合時点での `var` の上限
    pub var_ub: f64,
    /// この併合を行った時点での `kept` の下限 (同じ `kept` に複数併合した場合、元問題の下限とは限らない)
    pub kept_lb: f64,
}

impl Substitution {
    /// `x[kept]` に入っている `z = x_kept + s * x[var]` から本来の分割を復元して `x` に書き戻す。
    ///
    /// `raw = (z - kept_lb) / s` は `kept` が `kept_lb` にあるときの `var` の値。これを
    /// `[var_lb, var_ub]` にクランプすれば、`x[kept] = z - s*x[var]` は `kept` の範囲内に収まる
    /// (`s` の符号によらず成り立つ)。
    pub fn apply(&self, x: &mut [f64]) {
        let z = x[self.kept];
        let raw = (z - self.kept_lb) / self.s;
        let xj = raw.clamp(self.var_lb, self.var_ub);
        x[self.var] = xj;
        x[self.kept] = z - self.s * xj;
    }
}

/// 平行列併合の結果。消去された列は係数を行から除き、`c`・`lb`・`ub` を 0 にしてある (列番号は保持)。
pub struct ParallelColsResult {
    /// 消去列を除いた等式行列
    pub a: FaerCsr,
    /// 目的関数係数 (消去列は 0)
    pub c: Vec<f64>,
    /// 下限 (併合先は `z` の範囲に拡大、消去列は 0)
    pub lb: Vec<f64>,
    /// 上限 (併合先は `z` の範囲に拡大、消去列は 0)
    pub ub: Vec<f64>,
    /// 消去列を除いた `G` の実制約行
    pub real_rows: Vec<Vec<(usize, f64)>>,
    /// ポストソルブ用の併合記録 (発見順)
    pub substitutions: Vec<Substitution>,
}

/// 平行列を併合する (入力スナップショットから候補を決める 1 パス。連鎖はしない)。
/// 何も併合しなかった場合は入力の複製を返す。現在はテストからのみ使用
/// (本番のパイプラインは [`merge_parallel_columns_if_any`] を呼ぶ)。
#[cfg_attr(not(test), allow(dead_code))]
pub fn merge_parallel_columns(n: usize, a: &FaerCsr, real_rows: &[Vec<(usize, f64)>], c: &[f64], lb: &[f64], ub: &[f64]) -> ParallelColsResult {
    merge_parallel_columns_if_any(n, a, real_rows, c, lb, ub).unwrap_or_else(|| ParallelColsResult {
        a: a.clone(),
        c: c.to_vec(),
        lb: lb.to_vec(),
        ub: ub.to_vec(),
        real_rows: real_rows.to_vec(),
        substitutions: Vec::new(),
    })
}

/// [`merge_parallel_columns`] と同じだが、何も併合しなかった場合 (よくある場合) は入力を複製せず `None` を返す。
///
/// - `n`: 列数
/// - `a`: 等式行列
/// - `real_rows`: `G` の実制約行
/// - `c`, `lb`, `ub`: 目的関数係数と境界
pub fn merge_parallel_columns_if_any(n: usize, a: &FaerCsr, real_rows: &[Vec<(usize, f64)>], c: &[f64], lb: &[f64], ub: &[f64]) -> Option<ParallelColsResult> {
    let ar = a.as_ref();
    let n_a_rows = ar.nrows();

    // この呼び出し内だけの通し行番号: `a` の行が 0..n_a_rows、`real_rows` がその後。
    // 圧縮列形式へ直接流し込む。行順に出力するので各列は行番号昇順になる (署名計算の前提)。
    let columns = CscMat::from_entry_stream(n_a_rows + real_rows.len(), n, |emit| {
        for i in 0..n_a_rows {
            for (&j, &v) in ar.col_indices_of_row_raw(i).iter().zip(ar.values_of_row(i)) {
                if v != 0.0 {
                    emit(i, j, v);
                }
            }
        }
        for (gi, row) in real_rows.iter().enumerate() {
            for &(j, v) in row {
                if v != 0.0 {
                    emit(n_a_rows + gi, j, v);
                }
            }
        }
    });

    // 正規化した署名 (行番号, 係数/先頭係数 のビット列) で列を群に分ける。
    // 署名を実体化せず、安価な乗算ハッシュで候補群を引き、群の代表列の署名を再計算して要素ごとに厳密比較する。
    // sig_hash(col, inv): 列 col を inv (= 1/先頭係数) 倍した署名のハッシュ
    let sig_hash = |col: &[(usize, f64)], inv: f64| -> u64 {
        let mut hash = col.len() as u64;
        for &(row_id, v) in col {
            hash = (hash.rotate_left(5) ^ row_id as u64).wrapping_mul(0x517c_c1b7_2722_0a95);
            hash = (hash.rotate_left(5) ^ (v * inv).to_bits()).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
        hash
    };
    // ハッシュ連鎖: `heads[hash]` はそのハッシュの最新の群、`group_next[gid]` は次に古い群。
    // 署名が一致する群は高々 1 つなので、走査順は結果に影響しない。
    // group_members[gid]: 群 gid に属する列 (番号昇順に追加される)
    let mut group_members: Vec<Vec<usize>> = Vec::new();
    let mut group_next: Vec<usize> = Vec::new();
    let mut heads: std::collections::HashMap<u64, usize, std::hash::BuildHasherDefault<crate::presolve::redundancy::IdentityU64Hasher>> = std::collections::HashMap::default();
    for j in 0..n {
        let col = columns.col(j);
        // 候補条件: 実制約行に現れる、下限が有限 (上限は `+inf` でもよい)、固定されていない。
        if col.is_empty() || !lb[j].is_finite() || (ub[j] - lb[j]).abs() < TOL || crate::presolve::is_int_col(j) {
            continue;
        }
        // 先頭係数の逆数 (正規化用)
        let inv = 1.0 / col[0].1;
        let hash = sig_hash(col, inv);
        let head = heads.get(&hash).copied().unwrap_or(usize::MAX);
        let mut found = None;
        let mut gid = head;
        while gid != usize::MAX {
            let rep = columns.col(group_members[gid][0]);
            let rep_inv = 1.0 / rep[0].1;
            if rep.len() == col.len() && rep.iter().zip(col).all(|(&(ri, rv), &(ci, cv))| ri == ci && (rv * rep_inv).to_bits() == (cv * inv).to_bits()) {
                found = Some(gid);
                break;
            }
            gid = group_next[gid];
        }
        match found {
            Some(gid) => group_members[gid].push(j),
            None => {
                heads.insert(hash, group_members.len());
                group_next.push(head);
                group_members.push(vec![j]);
            }
        }
    }

    // 群を先頭の列番号順に並べたもの
    let mut groups_by_first: Vec<&Vec<usize>> = group_members.iter().collect();
    groups_by_first.sort_by_key(|v| v[0]);

    // used[j]: 列 j がこの呼び出しで既に併合 (併合先または消去) に使われたか
    let mut used = vec![false; n];
    let mut new_lb = lb.to_vec();
    let mut new_ub = ub.to_vec();
    let mut new_c = c.to_vec();
    let mut substitutions = Vec::new();
    // 消去された列の集合
    let mut eliminated: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();

    for members in groups_by_first {
        if members.len() < 2 {
            continue;
        }
        let mut members = members.clone();
        members.sort_unstable();
        let kept = members[0];
        if used[kept] {
            continue;
        }
        // kept_lead: 併合先の先頭係数 / cur_lb, cur_ub: 併合を重ねた時点での z の範囲
        let kept_lead = columns.col(kept)[0].1;
        let mut cur_lb = new_lb[kept];
        let mut cur_ub = new_ub[kept];
        let mut any_merged = false;
        for &var in &members[1..] {
            if used[var] {
                continue;
            }
            let var_lead = columns.col(var)[0].1;
            let s = var_lead / kept_lead;
            // 比例コスト条件 c_var == s * c_kept を相対許容誤差で判定
            let predicted_c_var = s * new_c[kept];
            let tol = TOL * (1.0 + new_c[var].abs().max(predicted_c_var.abs()));
            if (new_c[var] - predicted_c_var).abs() > tol {
                continue;
            }
            // s * x_var の取りうる範囲 (z の範囲の増分)。符号で向きを決める (min/max で並べ替えると
            // 矛盾した境界 lb > ub が正常な区間に化けて実行不能性が失われる)。
            let (lo, hi) = if s >= 0.0 { (s * new_lb[var], s * new_ub[var]) } else { (s * new_ub[var], s * new_lb[var]) };
            // 負の s と `var` の上限 `+inf` の組では lo = -inf となり、併合先が自由列になってしまう。
            // その形は下流で非常に退化しやすいため、この併合は行わず `var` を未併合のまま残す (経緯は履歴メモ参照)。
            if lo == f64::NEG_INFINITY {
                continue;
            }
            substitutions.push(Substitution { var, kept, s, var_lb: new_lb[var], var_ub: new_ub[var], kept_lb: cur_lb });
            cur_lb += lo;
            cur_ub += hi;
            used[var] = true;
            any_merged = true;
            eliminated.insert(var);
        }
        if any_merged {
            new_lb[kept] = cur_lb;
            new_ub[kept] = cur_ub;
            used[kept] = true;
        }
    }

    if eliminated.is_empty() {
        return None;
    }

    // 消去列は係数・境界を 0 にし (列番号は残す)、全行からその項を取り除く。
    for &j in &eliminated {
        new_c[j] = 0.0;
        new_lb[j] = 0.0;
        new_ub[j] = 0.0;
    }

    let new_a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a).into_iter().map(|row| row.into_iter().filter(|&(j, _)| !eliminated.contains(&j)).collect()).collect();
    let new_real_rows: Vec<Vec<(usize, f64)>> = real_rows.iter().map(|row| row.iter().copied().filter(|&(j, _)| !eliminated.contains(&j)).collect()).collect();

    Some(ParallelColsResult { a: crate::sparse::csr_from_rows(&new_a_rows, n), c: new_c, lb: new_lb, ub: new_ub, real_rows: new_real_rows, substitutions })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;
    use crate::sparse::csr_row_vec;

    #[test]
    fn merges_two_identical_columns_with_equal_cost() {
        // x0 + x1 + x2 = 10, cost x0 and x1 identical (s=1) -- must merge
        // into one, x2 left alone (different pattern: only x2 appears in
        // the second row).
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0), (2, 1.0)], vec![(2, 1.0)]], 3);
        let c = vec![5.0, 5.0, 2.0];
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![4.0, 6.0, 3.0];
        let r = merge_parallel_columns(3, &a, &[], &c, &lb, &ub);
        assert_eq!(r.substitutions.len(), 1);
        let sub = &r.substitutions[0];
        assert_eq!(sub.kept, 0);
        assert_eq!(sub.var, 1);
        assert!((sub.s - 1.0).abs() < 1e-9);
        assert!((r.lb[0] - 0.0).abs() < 1e-9);
        assert!((r.ub[0] - 10.0).abs() < 1e-9); // 4 + 6
        assert_eq!(r.lb[1], 0.0);
        assert_eq!(r.ub[1], 0.0);
        assert_eq!(r.c[1], 0.0);
        // Column 1 dropped from the row.
        let row0 = csr_row_vec(&r.a, 0);
        assert!(!row0.iter().any(|&(j, _)| j == 1));
    }

    #[test]
    fn recovers_a_feasible_split_at_every_extreme_and_interior_point() {
        let a = csr_from_rows(&[vec![(0, 2.0), (1, 3.0)]], 2);
        let c = vec![4.0, 6.0]; // s = 2/3 col-wise; c0 = s*c1 => 4 = (2/3)*6 OK
        let lb = vec![1.0, 0.5];
        let ub = vec![3.0, 4.0];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert_eq!(r.substitutions.len(), 1);
        let sub = &r.substitutions[0];
        assert_eq!(sub.kept, 0);
        assert_eq!(sub.var, 1);

        // z range: lb0 + min(s*lb1,s*ub1) .. ub0 + max(...)
        let z_lo = r.lb[0];
        let z_hi = r.ub[0];
        for frac in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let z = z_lo + frac * (z_hi - z_lo);
            let mut x = vec![0.0; 2];
            x[0] = z;
            sub.apply(&mut x);
            let x1 = x[sub.var];
            let x0 = x[sub.kept];
            assert!(x1 >= lb[1] - 1e-9 && x1 <= ub[1] + 1e-9, "x1={x1} out of [{}, {}]", lb[1], ub[1]);
            assert!(x0 >= lb[0] - 1e-9 && x0 <= ub[0] + 1e-9, "x0={x0} out of [{}, {}]", lb[0], ub[0]);
            assert!((x0 + sub.s * x1 - z).abs() < 1e-9);
        }
    }

    #[test]
    fn handles_negative_scale() {
        let a = csr_from_rows(&[vec![(0, 1.0), (1, -2.0)]], 2);
        let lb = vec![0.0, 0.0];
        let ub = vec![5.0, 5.0];
        // col0 leading=1.0, col1 leading=-2.0 => s = -2.0/1.0 = -2.0 for
        // var=1 relative to kept=0. Cost check needs c[1] == s*c[0]: pick
        // c0=-3.0 => predicted c1 = -2.0*-3.0 = 6.0.
        let c = vec![-3.0, 6.0];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert_eq!(r.substitutions.len(), 1);
        let sub = &r.substitutions[0];
        assert!((sub.s - (-2.0)).abs() < 1e-9);
        let z_lo = r.lb[0];
        let z_hi = r.ub[0];
        for frac in [0.0, 0.33, 0.5, 0.9, 1.0] {
            let z = z_lo + frac * (z_hi - z_lo);
            let mut x = vec![0.0; 2];
            x[0] = z;
            sub.apply(&mut x);
            let x1 = x[sub.var];
            let x0 = x[sub.kept];
            assert!(x1 >= lb[1] - 1e-9 && x1 <= ub[1] + 1e-9, "x1={x1}");
            assert!(x0 >= lb[0] - 1e-9 && x0 <= ub[0] + 1e-9, "x0={x0}");
            assert!((x0 + sub.s * x1 - z).abs() < 1e-9);
        }
    }

    #[test]
    fn leaves_unequal_cost_ratio_untouched() {
        // Same pattern, but cost isn't proportional -- dominatedcol's job,
        // not this module's.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let c = vec![1.0, 2.0]; // s=1, but c1 != s*c0
        let lb = vec![0.0, 0.0];
        let ub = vec![5.0, 5.0];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert!(r.substitutions.is_empty());
    }

    #[test]
    fn merges_an_upper_unbounded_column_the_standgub_shape() {
        // lb=0, ub=+inf, cost=0 on both -- the exact shape of `standgub`'s
        // own 108-column GUB block this module was written for. Only the
        // *lower* bound needs to be finite; the merged `z`'s own upper
        // bound must come out `+inf` too, and postsolve must still recover
        // a feasible split at an arbitrarily large (but finite, as any
        // real solved value is) `z`.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let c = vec![0.0, 0.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![5.0, f64::INFINITY];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert_eq!(r.substitutions.len(), 1);
        assert!(r.ub[0].is_infinite() && r.ub[0] > 0.0);
        let sub = &r.substitutions[0];
        for &z in &[0.0, 3.0, 1_000_000.0] {
            let mut x = vec![0.0; 2];
            x[0] = z;
            sub.apply(&mut x);
            let x1 = x[sub.var];
            let x0 = x[sub.kept];
            assert!(x1 >= lb[1] - 1e-9 && x1 <= ub[1] + 1e-9);
            assert!(x0 >= lb[0] - 1e-9 && x0 <= ub[0] + 1e-9);
            assert!((x0 + sub.s * x1 - z).abs() < 1e-6);
        }
    }

    #[test]
    fn leaves_a_fully_free_column_untouched() {
        // Both sides infinite -- `freevar`'s job, not this module's (see
        // the module docs' "Scope" section).
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let c = vec![3.0, 3.0];
        let lb = vec![0.0, f64::NEG_INFINITY];
        let ub = vec![5.0, f64::INFINITY];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert!(r.substitutions.is_empty());
    }

    #[test]
    fn refuses_a_merge_that_would_make_the_result_lower_unbounded() {
        // `standgub`-shaped columns (`lb=0`, `ub=+inf`, `cost=0`) but with
        // *opposite-signed* leading matrix entries (`s=-1`) -- the exact
        // shape a real Netlib instance (`greenbea`) hit: both columns are
        // individually candidacy-eligible (finite `lb`), but folding `var`
        // into `kept` at `s=-1` would send `kept`'s own merged lower bound
        // to `-inf` (`lo = min(s*lb[var], s*ub[var]) = min(0, -inf) =
        // -inf`), producing a genuinely free (`lb=-inf` *and* `ub=+inf`)
        // structural column downstream -- exactly what
        // `presolve::freevar`/`simplex::slope_intercept_dual`'s own preconditions
        // require never exists past this point (see this loop's own
        // comment). Must be refused outright, not merged.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, -1.0)]], 2);
        let c = vec![0.0, 0.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert!(r.substitutions.is_empty());
        assert!(r.lb[0].is_finite());
    }

    #[test]
    fn merges_a_whole_group_in_one_call() {
        // Five identical columns, one call absorbs all four duplicates
        // into column 0.
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0), (4, 1.0)]], 5);
        let c = vec![7.0; 5];
        let lb = vec![0.0; 5];
        let ub = vec![2.0; 5];
        let r = merge_parallel_columns(5, &a, &[], &c, &lb, &ub);
        assert_eq!(r.substitutions.len(), 4);
        assert!((r.ub[0] - 10.0).abs() < 1e-9); // 5 * 2.0
        for j in 1..5 {
            assert_eq!(r.lb[j], 0.0);
            assert_eq!(r.ub[j], 0.0);
        }
    }

    #[test]
    fn ignores_columns_with_no_real_row_appearance() {
        let a = csr_from_rows(&[], 2);
        let c = vec![1.0, 1.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![5.0, 5.0];
        let r = merge_parallel_columns(2, &a, &[], &c, &lb, &ub);
        assert!(r.substitutions.is_empty());
    }
}
