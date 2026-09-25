//! IneqSingleton (不等式行の列シングルトン): 不等式行 (範囲制約行を含む) にだけ一度現れる列を、
//! コストの符号に基づいて処理する (Andersen & Andersen 1995 §3.2, HiGHS `colSingleton` と同じ論法)。
//! 等式行に一度だけ現れる列は [`colsingleton`](super::colsingleton) の担当。
//!
//! ## 範囲制約行は 1 論理行として扱う
//!
//! `G` は全て `<=` 行なので、範囲制約 `L <= r.x <= U` は `r.x <= U` と `-r.x <= -L` の
//! 2 行の組として格納される。まず各 `G` 行を厳密に符号反転した相方と組にし、1 論理行とみなす。
//!
//! ## 縮約
//!
//! 列 `x_j` (コスト `c_j != 0`、等式行に現れない) が 1 論理行 `L <= a x_j + r <= U` にだけ現れるとする。
//! 目的関数は `x_j` を方向 `d` (`c_j < 0` なら `+1`) の箱境界 `t` へ押し、それにより行の活動値は
//! 片側 `S` (`a*d > 0` なら `U`、そうでなければ `L`) へ向かう。`[r_lo, r_hi]` を他列の箱上での `r` の範囲とする。
//!
//! - **固定**: `t` が有限で、他列がどうであれ `x_j = t` が `S` を破らないなら `x_j = t` に固定する。
//! - **等式化**: 側 `S` だけで `x_j` の境界 `t` が含意されるなら、最適解では `S` が必ず等号で成り立つ。
//!   行を等式 `a x_j + r = S` に置き換え (反対側は冗長なので捨てる)、その後 `colsingleton` が `x_j` を消去する。
//!
//! 1 呼び出しにつき 1 論理行あたり 1 回だけ判定する (等式化した行はその呼び出し中は他列の候補にしない)。
//! 同じ行の複数列を固定するのは、各判定が他列の箱全体を使うので問題ない。

use std::collections::HashMap;

use crate::sparse::FaerCsr;
use crate::params::presolve::TOL;

/// [`run`] の結果。
pub struct IneqSingletonResult {
    /// 固定する `(列, 値)`
    pub fixes: Vec<(usize, f64)>,
    /// `(g_row, partner)`: `real_rows[g_row]` (右辺込みの `<=` 行) を等式に変える。
    /// この行と、組になった反対側の行 `partner` (あれば) は `G` から取り除かれる。
    pub implied_equalities: Vec<(usize, Option<usize>)>,
}

/// 列 `skip` を除いた各項の箱境界上で `sum(v * x_k)` が取りうる範囲 `(下限, 上限)` を返す。
/// 無限大同士の打ち消しで NaN になった端は、それぞれ -∞ / +∞ とみなす。
fn others_range(row: &[(usize, f64)], skip: usize, lb: &[f64], ub: &[f64]) -> (f64, f64) {
    let (mut lo, mut hi) = (0.0f64, 0.0f64);
    for &(k, v) in row {
        if k == skip {
            continue;
        }
        let (a, b) = if v > 0.0 { (v * lb[k], v * ub[k]) } else { (v * ub[k], v * lb[k]) };
        lo += a;
        hi += b;
    }
    (if lo.is_nan() { f64::NEG_INFINITY } else { lo }, if hi.is_nan() { f64::INFINITY } else { hi })
}

/// 不等式行の列シングルトンを探し、固定と等式化の候補を返す (行列自体は書き換えない)。
///
/// - `n`: 列数
/// - `a`: 等式制約行列 (ここに現れる列は対象外)
/// - `real_rows`, `real_rhs`: `G` の実制約行 (`<=` 正規化済み) とその右辺
/// - `c`: 目的関数係数
/// - `lb`, `ub`: 変数の境界
pub fn resolve_inequality_singletons(n: usize, a: &FaerCsr, real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], c: &[f64], lb: &[f64], ub: &[f64]) -> IneqSingletonResult {
    let mut out = IneqSingletonResult { fixes: Vec::new(), implied_equalities: Vec::new() };

    // 等式行に現れる列は colsingleton / aggregator の担当なので除外する。
    let mut in_a = vec![false; n];
    {
        let ar = a.as_ref();
        for i in 0..ar.nrows() {
            for &k in ar.col_indices_of_row_raw(i) {
                in_a[k] = true;
            }
        }
    }

    // 安価な事前判定: 候補列は等式行に現れず、`G` 行に高々 2 回 (1 論理行) だけ現れる。
    // 該当列がなければ、下の組み合わせ構築をせずに終了する。
    // g_count[k]: 列 k が現れる `G` 行の数
    let mut g_count = vec![0u32; n];
    for row in real_rows {
        for &(k, v) in row {
            if v != 0.0 {
                g_count[k] += 1;
            }
        }
    }
    if !(0..n).any(|j| !in_a[j] && (1..=2).contains(&g_count[j]) && c[j] != 0.0 && lb[j] != ub[j]) {
        return out;
    }

    // 各行を厳密な符号反転行と組にする (列番号と係数ビット列の FNV-1a 系ハッシュで候補を引き、厳密比較で確定)。
    // row_hash(row, negate): negate=true なら係数を符号反転したものとしてハッシュする。
    let row_hash = |row: &[(usize, f64)], negate: bool| -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for &(k, v) in row {
            let bits = (if negate { -v } else { v }).to_bits();
            h = (h ^ k as u64).wrapping_mul(0x0100_0000_01b3);
            h = (h ^ bits).wrapping_mul(0x0100_0000_01b3);
        }
        h
    };
    // ハッシュ値 -> その値を持つ行番号の一覧
    let mut index: HashMap<u64, Vec<usize>> = HashMap::with_capacity(real_rows.len());
    for (i, row) in real_rows.iter().enumerate() {
        index.entry(row_hash(row, false)).or_default().push(i);
    }
    // x と y が同じ台で係数が互いに符号反転か
    let is_negation = |x: &[(usize, f64)], y: &[(usize, f64)]| x.len() == y.len() && x.iter().zip(y).all(|(&(k1, v1), &(k2, v2))| k1 == k2 && v1 == -v2);
    // partner[i]: 行 i と組になる符号反転行 (なければ None)
    let mut partner: Vec<Option<usize>> = vec![None; real_rows.len()];
    for (i, row) in real_rows.iter().enumerate() {
        if partner[i].is_some() {
            continue;
        }
        if let Some(cands) = index.get(&row_hash(row, true)) {
            if let Some(&p) = cands.iter().find(|&&p| p != i && partner[p].is_none() && is_negation(row, &real_rows[p])) {
                partner[i] = Some(p);
                partner[p] = Some(i);
            }
        }
    }

    // 論理行: 各組は番号の小さい方の行で代表させる。
    // logical_count[k]: 列 k が現れる論理行の数 / logical_row_of[k]: 最後に現れた論理行の代表行
    let mut logical_count = vec![0usize; n];
    let mut logical_row_of = vec![usize::MAX; n];
    for (i, row) in real_rows.iter().enumerate() {
        if matches!(partner[i], Some(p) if p < i) {
            continue;
        }
        for &(k, v) in row {
            if v != 0.0 {
                logical_count[k] += 1;
                logical_row_of[k] = i;
            }
        }
    }

    // row_done[i]: 論理行 i がこの呼び出しで既に等式化されたか
    let mut row_done = vec![false; real_rows.len()];
    for j in 0..n {
        if in_a[j] || logical_count[j] != 1 || c[j] == 0.0 || lb[j] == ub[j] {
            continue;
        }
        let i = logical_row_of[j];
        if row_done[i] {
            continue;
        }
        let row = &real_rows[i];
        let Some(&(_, aj)) = row.iter().find(|&&(k, _)| k == j) else { continue };
        // aj: 列 j の係数 / upper, lower: 論理行の上側・下側の右辺 (相方がなければ下側は -∞)
        let upper = real_rhs[i];
        let lower = partner[i].map_or(f64::NEG_INFINITY, |p| -real_rhs[p]);
        let (r_lo, r_hi) = others_range(row, j, lb, ub);
        // d: 目的関数が x_j を押す方向 (+1: 増加), target: その方向の箱境界 t
        // toward_upper: その移動で行の活動値が上側 U へ向かうか
        let d = if c[j] < 0.0 { 1.0 } else { -1.0 };
        let target = if d > 0.0 { ub[j] } else { lb[j] };
        let toward_upper = aj * d > 0.0;
        // 値 x の大きさに応じた相対許容誤差
        let tol = |x: f64| TOL * (1.0 + x.abs());

        // 固定: target まで動かしても側 S を決して破らない。
        if target.is_finite() {
            let never_violates = if toward_upper {
                aj * target + r_hi <= upper + tol(upper)
            } else {
                lower == f64::NEG_INFINITY || aj * target + r_lo >= lower - tol(lower)
            };
            if never_violates {
                out.fixes.push((j, target));
                continue;
            }
        }

        // 等式化: 側 S だけで方向 d の x_j の境界が含意される。
        // side/side_rhs: 側 S の右辺, residual: S に最も有利な他列の寄与, g_row: 側 S を表す G 行
        let (side, side_rhs, residual, g_row) = if toward_upper {
            (upper, upper, r_lo, Some(i))
        } else {
            (lower, lower, r_hi, partner[i])
        };
        let Some(g_row) = g_row else { continue };
        if !side.is_finite() || !residual.is_finite() {
            continue;
        }
        // 上側なら a x_j <= S - r_lo、下側なら a x_j >= S - r_hi から得られる x_j の含意値。
        let implied = (side_rhs - residual) / aj;
        let implies_target = if d > 0.0 { implied <= target + tol(target) } else { implied >= target - tol(target) };
        if implies_target {
            // 等式化した行の反対側 (これも G から除く)
            let other = if g_row == i { partner[i] } else { Some(i) };
            out.implied_equalities.push((g_row, other));
            row_done[i] = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    fn empty_a(n: usize) -> FaerCsr {
        csr_from_rows(&[], n)
    }

    #[test]
    fn fixes_when_the_row_can_never_bind() {
        // min -x0, 0 <= x0 <= 1, x0 + x1 <= 5, x1 in [0,1]: x0 = 1 never violates.
        let r = resolve_inequality_singletons(2, &empty_a(2), &[vec![(0, 1.0), (1, 1.0)]], &[5.0], &[-1.0, 0.0], &[0.0, 0.0], &[1.0, 1.0]);
        assert_eq!(r.fixes, vec![(0, 1.0)]);
        assert!(r.implied_equalities.is_empty());
    }

    #[test]
    fn ranged_pair_is_one_row_and_its_upper_side_becomes_tight() {
        // min -x0, x0 in [0,10], 1 <= x0 + x1 <= 3 (as a pair), x1 in [0,1]:
        // the upper side implies x0 <= 3 <= 10, so it is tight at every optimum.
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(0, -1.0), (1, -1.0)]];
        let r = resolve_inequality_singletons(2, &empty_a(2), &rows, &[3.0, -1.0], &[-1.0, 0.0], &[0.0, 0.0], &[10.0, 1.0]);
        assert!(r.fixes.is_empty());
        assert_eq!(r.implied_equalities, vec![(0, Some(1))]);
    }

    #[test]
    fn lower_side_becomes_tight_when_cost_pushes_down() {
        // min x0 (pushes down), x0 in [0,10], 1 <= x0 + x1 <= 3, x1 in [0,1]:
        // the lower side implies x0 >= 0 = lb only when 1 - r_hi = 0 >= 0.
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(0, -1.0), (1, -1.0)]];
        let r = resolve_inequality_singletons(2, &empty_a(2), &rows, &[3.0, -1.0], &[1.0, 0.0], &[0.0, 0.0], &[10.0, 1.0]);
        assert!(r.fixes.is_empty());
        assert_eq!(r.implied_equalities, vec![(1, Some(0))]);
    }

    #[test]
    fn neither_when_the_box_and_the_row_both_can_bind() {
        // min -x0, x0 in [0,2], x0 + x1 <= 2.5, x1 in [0,1]: x0 = 2 violates when
        // x1 = 1, and the row only implies x0 <= 2.5 > 2 — no reduction.
        let r = resolve_inequality_singletons(2, &empty_a(2), &[vec![(0, 1.0), (1, 1.0)]], &[2.5], &[-1.0, 0.0], &[0.0, 0.0], &[2.0, 1.0]);
        assert!(r.fixes.is_empty() && r.implied_equalities.is_empty());
    }

    #[test]
    fn equality_row_columns_are_left_to_colsingleton() {
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let r = resolve_inequality_singletons(2, &a, &[vec![(0, 1.0), (1, 1.0)]], &[5.0], &[-1.0, 0.0], &[0.0, 0.0], &[1.0, 1.0]);
        assert!(r.fixes.is_empty() && r.implied_equalities.is_empty());
    }
}
