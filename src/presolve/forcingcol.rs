//! 強制列 (forcing column、HiGHS `HPresolve` の "Forcing col") の消去。
//!
//! 費用 0 の列 `j` が、等式行に現れず、現れるすべての不等式行 (`G` は `<=` 正規化済み) で
//! 「増やすと行が緩む」(係数が負) かつ上限が `+inf` なら、他の変数の値が何であっても `x_j` を十分
//! 大きくすればそれらの行はすべて満たされ、目的関数値は変わらない。したがって列 `j` と、`j` を含む
//! 行をすべて取り除いてよい (減らす向きも同様: 係数がすべて正で下限が `-inf`)。後処理では、残りの
//! 変数の値から各行を満たす `x_j` の最小値 (減らす向きなら最大値) を求め、元の上下限と合わせて置く。
//!
//! 例 (Mittelmann ns1688926): 費用 0・下限 0 の列 1 本が `x_k - t >= 0` 型の 8,192 行すべてに
//! 正係数で現れる。HiGHS・CLP はこれで 8,192 行を消すが、`dualfix` は固定先の上限が無限なので
//! 固定できなかった。
//!
//! 取り除いた行が別の列の唯一の「ロック」なら、その列が新たに候補になる (待ち行列で連鎖させる)。

use crate::sparse::{FaerCsr, csr_row_iter};

/// 強制列 1 本の後処理の記録。値は `rows` の各行を満たす範囲の端 (と元の上下限) から求める。
#[derive(Clone, Debug)]
pub struct ForcingCol {
    /// 消去した列。
    pub var: usize,
    /// 消去時の下限・上限 (増やす向きなら `ub = +inf`、減らす向きなら `lb = -inf`)。
    pub lb: f64,
    pub ub: f64,
    /// 取り除いた行: `(他の項, 右辺, x_j の係数)`、行は `Σ 他の項 + coeff * x_j <= rhs`。
    pub rows: Vec<(Vec<(usize, f64)>, f64, f64)>,
}

impl ForcingCol {
    /// 他の変数の値 `x` から `x[self.var]` を求めて書く。増やす向き (係数が負) なら各行の
    /// 下限 `(rhs - Σ) / coeff` と元の下限の最大、減らす向きなら上限と元の上限の最小。
    pub fn apply(&self, x: &mut [f64]) {
        let increase = self.rows.first().is_some_and(|r| r.2 < 0.0);
        let mut v = if increase { self.lb } else { self.ub };
        for (terms, rhs, coeff) in &self.rows {
            let mut s = *rhs;
            for &(k, a) in terms {
                s -= a * x[k];
            }
            let t = s / coeff;
            if increase {
                if !(t <= v) {
                    v = t;
                }
            } else if !(t >= v) {
                v = t;
            }
        }
        if !v.is_finite() {
            // 行がすべて緩く (どの行も制約にならない)、かつ元の上下限も無限: 0 が許される。
            v = if increase { self.lb.max(0.0) } else { self.ub.min(0.0) };
            if !v.is_finite() {
                v = 0.0;
            }
        }
        x[self.var] = v;
    }
}

/// 強制列を探して消去する。`real_rows`/`real_rhs` (`G` の多変数行) から取り除いた行を除き、
/// 消去した列の記録を返す。消去した列の上下限は呼び出し側で `[0, 0]` に固定すること
/// (どの行にも現れなくなるので値は後処理で決まる)。
pub fn eliminate_forcing_columns(
    n: usize,
    a: &FaerCsr,
    real_rows: &mut Vec<Vec<(usize, f64)>>,
    real_rhs: &mut Vec<f64>,
    c: &[f64],
    lb: &[f64],
    ub: &[f64],
) -> Vec<ForcingCol> {
    let mut in_equality = vec![false; n];
    let ar = a.as_ref();
    for i in 0..ar.nrows() {
        for (j, v) in csr_row_iter(a, i) {
            if v != 0.0 {
                in_equality[j] = true;
            }
        }
    }
    // 候補になりうる列: 費用 0、等式行に現れない、固定されていない、増やす (減らす) 側が無限。
    let maybe = |j: usize| !in_equality[j] && c[j] == 0.0 && lb[j] < ub[j] && (ub[j] == f64::INFINITY || lb[j] == f64::NEG_INFINITY);
    if !(0..n).any(maybe) {
        return Vec::new();
    }
    let mut up_lock = vec![0usize; n];
    let mut down_lock = vec![0usize; n];
    let mut col_rows: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (i, row) in real_rows.iter().enumerate() {
        for &(j, v) in row {
            if v > 0.0 {
                up_lock[j] += 1;
            } else if v < 0.0 {
                down_lock[j] += 1;
            } else {
                continue;
            }
            if maybe(j) {
                col_rows[j].push(i as u32);
            }
        }
    }
    let candidate = |j: usize, up: &[usize], down: &[usize]| {
        maybe(j) && up[j] + down[j] > 0 && ((up[j] == 0 && ub[j] == f64::INFINITY) || (down[j] == 0 && lb[j] == f64::NEG_INFINITY))
    };
    let mut removed = vec![false; real_rows.len()];
    let mut done = vec![false; n];
    let mut queue: Vec<usize> = (0..n).filter(|&j| candidate(j, &up_lock, &down_lock)).collect();
    let mut out = Vec::new();
    while let Some(j) = queue.pop() {
        if done[j] || !candidate(j, &up_lock, &down_lock) {
            continue;
        }
        done[j] = true;
        let mut rows = Vec::new();
        for &i in &col_rows[j] {
            let i = i as usize;
            if removed[i] {
                continue;
            }
            removed[i] = true;
            let mut coeff = 0.0;
            let mut terms = Vec::with_capacity(real_rows[i].len().saturating_sub(1));
            for &(k, v) in &real_rows[i] {
                if k == j {
                    coeff += v;
                    continue;
                }
                terms.push((k, v));
                if v > 0.0 {
                    up_lock[k] -= 1;
                } else if v < 0.0 {
                    down_lock[k] -= 1;
                }
                if !done[k] && candidate(k, &up_lock, &down_lock) {
                    queue.push(k);
                }
            }
            if coeff > 0.0 {
                up_lock[j] -= 1;
            } else if coeff < 0.0 {
                down_lock[j] -= 1;
            }
            rows.push((terms, real_rhs[i], coeff));
        }
        if rows.is_empty() {
            continue;
        }
        out.push(ForcingCol { var: j, lb: lb[j], ub: ub[j], rows });
    }
    if !out.is_empty() {
        let mut k = 0usize;
        let mut kept_rows = Vec::with_capacity(real_rows.len());
        let mut kept_rhs = Vec::with_capacity(real_rows.len());
        for (row, rhs) in real_rows.drain(..).zip(real_rhs.drain(..)) {
            if !removed[k] {
                kept_rows.push(row);
                kept_rhs.push(rhs);
            }
            k += 1;
        }
        *real_rows = kept_rows;
        *real_rhs = kept_rhs;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    #[test]
    fn zero_cost_column_relaxing_all_its_rows_is_removed_and_restored() {
        // 行 0: x0 - x2 <= 0 (x2 >= x0)、行 1: -x1 - x2 <= -3 (x2 >= 3 - x1)、行 2: x0 + x1 <= 4。
        // x2 は費用 0・下限 0・上限 +inf で、行 0・1 で係数が負 (増やすと緩む)。
        let a = csr_from_rows(&[], 3);
        let mut rows = vec![vec![(0, 1.0), (2, -1.0)], vec![(1, -1.0), (2, -1.0)], vec![(0, 1.0), (1, 1.0)]];
        let mut rhs = vec![0.0, -3.0, 4.0];
        let c = vec![1.0, 1.0, 0.0];
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![f64::INFINITY; 3];
        let steps = eliminate_forcing_columns(3, &a, &mut rows, &mut rhs, &c, &lb, &ub);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].var, 2);
        assert_eq!(rows, vec![vec![(0, 1.0), (1, 1.0)]]);
        assert_eq!(rhs, vec![4.0]);
        let mut x = vec![2.0, 0.5, 0.0];
        steps[0].apply(&mut x);
        assert_eq!(x[2], 2.5);
        let mut x = vec![0.0, 4.0, 0.0];
        steps[0].apply(&mut x);
        assert_eq!(x[2], 0.0);
    }

    #[test]
    fn column_with_cost_or_in_equality_or_locked_is_kept() {
        let a = csr_from_rows(&[vec![(1, 1.0)]], 3);
        let mut rows = vec![vec![(0, 1.0), (1, -1.0), (2, -1.0)], vec![(0, -1.0), (2, 1.0)]];
        let mut rhs = vec![0.0, 0.0];
        // x0: 上下に locks、x1: 等式行に現れる、x2: 両方向に locks。
        let c = vec![0.0, 0.0, 0.0];
        let lb = vec![0.0; 3];
        let ub = vec![f64::INFINITY; 3];
        assert!(eliminate_forcing_columns(3, &a, &mut rows, &mut rhs, &c, &lb, &ub).is_empty());
        assert_eq!(rows.len(), 2);
    }
}
