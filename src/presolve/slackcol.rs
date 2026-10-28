//! 費用 0 のスラック列の消去 (`ENOMOTO_PRESOLVE_SLACK_COL`、試作)。
//!
//! 費用 0・上下限が有限の連続列 `j` が、等式行に現れず、`G` (`<=` 正規化済み) の 1 行だけ、または符号を反転しただけの
//! 2 行 (範囲制約 `L <= r + a x_j <= U` を 2 本の `<=` で表したもの) にだけ現れるなら、`x_j` を消して行を `a x_j` の値域
//! `[mn, mx]` だけ緩めてよい (`r <= U - mn`、`r >= L - mx`)。緩めた行を満たす `r` に対しては、元の行を満たす `x_j` が
//! 上下限の中に必ずある (2 本の行は同じ行なので、それぞれの緩め方が同時に成り立つ)。目的値は変わらない。
//!
//! [`super::colsingleton`] は等式行の 1 行だけの列を等式 1 本につき 1 列だけ代入で消し、その等式は範囲制約になる。同じ行に
//! 残った費用 0 のスラック列は範囲制約で両向きにロックされるので [`super::dualfix`] では固定できず、上限が有限なので
//! [`super::forcingcol`] の対象でもない (swath1: 行 0 の費用 0・[0, 1] の連続列 54 本が残っていた)。
//!
//! 後処理は [`SlackCol::apply`]: 取り除く前の各行が `x_j` に許す区間と上下限の共通部分の中の値を置く。

/// 消去した列 1 本の後処理の記録。
#[derive(Clone, Debug)]
pub struct SlackCol {
    /// 消去した列。
    pub var: usize,
    /// 消去時の上下限 (どちらも有限)。
    pub lb: f64,
    pub ub: f64,
    /// 取り除く前の行: `(他の項, 右辺, x_j の係数)`、行は `Σ 他の項 + coeff * x_j <= rhs`。
    pub rows: Vec<(Vec<(usize, f64)>, f64, f64)>,
}

impl SlackCol {
    /// 他の変数の値 `x` から、全ての行を満たす `x[self.var]` を上下限の中に置く (区間が空なら、数値誤差として
    /// 区間の端のうち上下限に近い方に寄せる)。
    pub fn apply(&self, x: &mut [f64]) {
        let (mut lo, mut hi) = (self.lb, self.ub);
        for (terms, rhs, coeff) in &self.rows {
            let mut s = *rhs;
            for &(k, a) in terms {
                s -= a * x[k];
            }
            let t = s / coeff;
            if *coeff > 0.0 {
                hi = hi.min(t);
            } else {
                lo = lo.max(t);
            }
        }
        let v = if lo <= hi { lo } else { 0.5 * (lo + hi) };
        x[self.var] = v.max(self.lb).min(self.ub);
    }
}

/// 2 本の行が符号を反転しただけの同じ行か (項の並びは列の昇順とは限らないので並べ替えて比べる)。
fn opposite(a: &[(usize, f64)], b: &[(usize, f64)]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut x: Vec<(usize, f64)> = a.to_vec();
    let mut y: Vec<(usize, f64)> = b.to_vec();
    x.sort_unstable_by_key(|t| t.0);
    y.sort_unstable_by_key(|t| t.0);
    x.iter().zip(&y).all(|(p, q)| p.0 == q.0 && p.1 == -q.1)
}

/// 費用 0 のスラック列を探して消去する。`real_rows`/`real_rhs` (`G` の多変数行) を書き換え、消去した列の記録を返す。
/// 消去した列の上下限は呼び出し側で `[0, 0]` に固定すること。`is_int` の列 (整数列) は消さない。
pub fn eliminate_slack_columns(
    n: usize,
    in_equality: &[bool],
    real_rows: &mut [Vec<(usize, f64)>],
    real_rhs: &mut [f64],
    c: &[f64],
    lb: &[f64],
    ub: &[f64],
    is_int: &dyn Fn(usize) -> bool,
) -> Vec<SlackCol> {
    let cand = |j: usize| c[j] == 0.0 && lb[j].is_finite() && ub[j].is_finite() && lb[j] < ub[j] && !in_equality[j] && !is_int(j);
    if !(0..n).any(cand) {
        return Vec::new();
    }
    // 列ごとに現れる行 (候補の列だけ、3 行目以降は数だけ分かればよい)
    let mut where_: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, row) in real_rows.iter().enumerate() {
        for &(j, _) in row {
            if cand(j) && where_[j].len() < 3 {
                where_[j].push(i);
            }
        }
    }
    // 2 行の組が反転した同じ行かの控え (同じ組に多数のスラック列があるので 1 回だけ比べる)
    let mut pair_ok: std::collections::HashMap<(usize, usize), bool> = std::collections::HashMap::new();
    let mut out = Vec::new();
    for j in 0..n {
        let rs = &where_[j];
        if rs.is_empty() || rs.len() > 2 || !cand(j) {
            continue;
        }
        if rs.len() == 2 {
            let key = (rs[0], rs[1]);
            let ok = match pair_ok.get(&key) {
                Some(&v) => v,
                None => {
                    let v = opposite(&real_rows[rs[0]], &real_rows[rs[1]]);
                    pair_ok.insert(key, v);
                    v
                }
            };
            if !ok {
                continue;
            }
        }
        let mut rec = SlackCol { var: j, lb: lb[j], ub: ub[j], rows: Vec::with_capacity(rs.len()) };
        for &i in rs.iter() {
            let coeff = real_rows[i].iter().find(|t| t.0 == j).map(|t| t.1).unwrap_or(0.0);
            let terms: Vec<(usize, f64)> = real_rows[i].iter().copied().filter(|t| t.0 != j).collect();
            rec.rows.push((terms.clone(), real_rhs[i], coeff));
            // 行を x_j の項の最小値だけ緩める
            let mn = if coeff > 0.0 { coeff * lb[j] } else { coeff * ub[j] };
            real_rhs[i] -= mn;
            real_rows[i] = terms;
        }
        // 反転した組では両方の行から消したので、組の控えは同じまま使える (両方の行から同じ列が消えた)
        out.push(rec);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_zero_cost_slacks_from_a_ranged_row() {
        // 0 <= y - 2 s1 - 2 s2 <= 1 (y は費用 1、s1, s2 は費用 0・[0, 1])。G では y - 2 s1 - 2 s2 <= 1 と -y + 2 s1 + 2 s2 <= 0
        let mut rows = vec![vec![(0, 1.0), (1, -2.0), (2, -2.0)], vec![(0, -1.0), (1, 2.0), (2, 2.0)]];
        let mut rhs = vec![1.0, 0.0];
        let c = vec![1.0, 0.0, 0.0];
        let (lb, ub) = (vec![0.0; 3], vec![10.0, 1.0, 1.0]);
        let recs = eliminate_slack_columns(3, &[false; 3], &mut rows, &mut rhs, &c, &lb, &ub, &|_| false);
        assert_eq!(recs.len(), 2);
        // y <= 1 + 4 と -y <= 0
        assert_eq!(rows, vec![vec![(0, 1.0)], vec![(0, -1.0)]]);
        assert_eq!(rhs, vec![5.0, 0.0]);
        // y = 3 なら 2 s1 + 2 s2 が [2, 3] に入るよう s を置ける
        for y in [0.0, 1.0, 3.0, 5.0] {
            let mut x = vec![y, 0.0, 0.0];
            for r in recs.iter().rev() {
                r.apply(&mut x);
            }
            let r = x[0] - 2.0 * x[1] - 2.0 * x[2];
            assert!((-1e-9..=1.0 + 1e-9).contains(&r), "y {y}: x {x:?}");
            assert!(x[1..].iter().all(|v| (0.0..=1.0).contains(v)));
        }
    }

    #[test]
    fn keeps_columns_in_two_different_rows_or_with_cost() {
        let mut rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (1, -1.0)], vec![(2, 1.0), (0, 1.0)]];
        let mut rhs = vec![1.0, 1.0, 1.0];
        let c = vec![0.0, 0.0, 1.0];
        let (lb, ub) = (vec![0.0; 3], vec![1.0; 3]);
        // 列 0 は 3 行、列 1 は反転していない 2 行、列 2 は費用 1
        let recs = eliminate_slack_columns(3, &[false; 3], &mut rows, &mut rhs, &c, &lb, &ub, &|_| false);
        assert!(recs.is_empty());
    }
}
