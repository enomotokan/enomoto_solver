//! 列の並びに依らない「列の鍵」(試作 `ENOMOTO_MIP_ORDINV`、既定では使わない。analysis/robust2_20261114.md)。
//!
//! 行・列を並べ替えた同値な問題で、前処理の結果 (どの列を消すか・どの列を併合先にするか・クリーク行にどの列を足すか) が
//! 列番号の順の同点崩しや走査の順で変わり、根の LP の強さまで変わる (analysis/robust_20261114.md の型 D)。
//! ここでは列ごとに、係数・費用・境界・整数性から作るハッシュ (Weisfeiler-Lehman の色の精密化を 2 回) を作り、
//! 同点を列番号でなくこの鍵で崩せるようにする。鍵は行・列の番号を使わないので、並べ替えても同じ列には同じ鍵が付く。
//! 構造が見分けられない列 (対称な列など) は同じ鍵になり、その間の同点は従来どおり列番号で崩す (結果は同値な問題の
//! 並べ替え違いになるだけで、強さは変わらないと見込む)。
//!
//! 係数は並べ替えた MPS の書き出しで最後の桁が揺れうるので、f32 に丸めてからハッシュする。

use crate::sparse::{csr_row_iter, FaerCsr};

thread_local! {
    /// 前処理の呼び出しの間だけ設定する列の鍵 (列番号 → 鍵)。`None` なら試作は切
    static ORD_KEY: std::cell::RefCell<Option<std::rc::Rc<Vec<u64>>>> = const { std::cell::RefCell::new(None) };
}

/// 試作 `ENOMOTO_MIP_ORDINV` の文字 `c` を含むか (既定では偽)。文字:
/// - `k`: 前処理 (整数最適化) の入口で列の鍵を作る (ほかの文字の前提。ほかの文字があれば自動で作る)
/// - `e`: 等式行の伝播を不動点まで回す (行の順に回す Gauss-Seidel をパス数で打ち切ると、順で結果が変わる)
/// - `d`: 二項等式の消去で、どちらの列を消すかの同点と行を見る順を鍵で決める
/// - `p`: 平行列の併合で、正規化を最大の係数で行い、比べを相対 1e-12 の許容誤差にする (先頭の行の係数で割った値の
///   ビット比べは、行の順で丸めが変わって見落とす)。併合先の同点も鍵で崩す
/// - `q`: クリーク行の拡張で、候補の文字を鍵の順に見る
pub(crate) fn ordinv(c: char) -> bool {
    env_str!("ENOMOTO_MIP_ORDINV").is_some_and(|v| v.contains(c))
}

/// 試作が何か入っているか
pub(crate) fn ordinv_any() -> bool {
    env_str!("ENOMOTO_MIP_ORDINV").is_some_and(|v| !v.is_empty() && v != "0")
}

#[inline]
fn mix(mut h: u64) -> u64 {
    // splitmix64 の仕上げ
    h ^= h >> 30;
    h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94d0_49bb_1331_11eb);
    h ^ (h >> 31)
}

#[inline]
fn qf(v: f64) -> u64 {
    if v == 0.0 {
        return 0;
    }
    if !v.is_finite() {
        return if v > 0.0 { 0x7ff0_0000_0000_0001 } else { 0xfff0_0000_0000_0001 };
    }
    (v as f32).to_bits() as u64
}

/// 列の鍵を作る。`rows` は (行, 下限, 上限) の並び (等式・不等式とも)、`c` は費用、`lb`・`ub` は列の境界、`is_int` は整数性。
/// 行の色・列の色を交互に 2 回精密化する。和で集めるので行の中・列の中の並びに依らない
pub(crate) fn column_keys(n: usize, rows: &[(Vec<(usize, f64)>, f64, f64)], c: &[f64], lb: &[f64], ub: &[f64], is_int: &[bool]) -> Vec<u64> {
    let mut col: Vec<u64> = (0..n)
        .map(|j| mix(qf(c[j]) ^ mix(qf(lb[j]).wrapping_add(0x9e37)) ^ mix(qf(ub[j]).wrapping_add(0x7f4a)) ^ (is_int.get(j).copied().unwrap_or(false) as u64) << 1))
        .collect();
    let mut row: Vec<u64> = rows.iter().map(|(r, l, u)| mix(qf(*l) ^ mix(qf(*u) ^ 0x1234) ^ (r.len() as u64) << 7)).collect();
    for _ in 0..2 {
        // 行の色 = 前の行の色 + Σ mix(係数, 列の色)
        for (i, (r, _, _)) in rows.iter().enumerate() {
            let mut s = 0u64;
            for &(j, v) in r {
                s = s.wrapping_add(mix(qf(v) ^ col[j].rotate_left(17)));
            }
            row[i] = mix(row[i] ^ s.rotate_left(5));
        }
        // 列の色 = 前の列の色 + Σ mix(係数, 行の色)
        let mut acc = vec![0u64; n];
        for (i, (r, _, _)) in rows.iter().enumerate() {
            for &(j, v) in r {
                acc[j] = acc[j].wrapping_add(mix(qf(v) ^ row[i].rotate_left(29)));
            }
        }
        for j in 0..n {
            col[j] = mix(col[j] ^ acc[j].rotate_left(11));
        }
    }
    col
}

/// 前処理の行列の形 (`A x = b`、`G x <= h`) から列の鍵を作って設定する ([`clear`] で外す)。`G` の 1 項の行 (境界) も行として数える
pub(crate) fn set_from_ag(n: usize, a: &FaerCsr, b: &[f64], g: &FaerCsr, h: &[f64], c: &[f64], is_int: &[bool]) {
    let mut rows: Vec<(Vec<(usize, f64)>, f64, f64)> = Vec::with_capacity(a.nrows() + g.nrows());
    for i in 0..a.nrows() {
        rows.push((csr_row_iter(a, i).collect(), b[i], b[i]));
    }
    for i in 0..g.nrows() {
        rows.push((csr_row_iter(g, i).collect(), f64::NEG_INFINITY, h[i]));
    }
    let inf = vec![f64::INFINITY; n];
    let ninf = vec![f64::NEG_INFINITY; n];
    let k = column_keys(n, &rows, c, &ninf, &inf, is_int);
    ORD_KEY.with(|m| *m.borrow_mut() = Some(std::rc::Rc::new(k)));
}

/// 設定した鍵を外す
pub(crate) fn clear() {
    ORD_KEY.with(|m| *m.borrow_mut() = None);
}

/// 今の鍵 (試作が切か、前処理の外なら `None`)
pub(crate) fn keys() -> Option<std::rc::Rc<Vec<u64>>> {
    ORD_KEY.with(|m| m.borrow().clone())
}

/// 列 `j` の並べ順の鍵 (鍵, 列番号)。鍵がなければ (0, j) で、従来の列番号の順と同じ
#[inline]
pub(crate) fn sort_key(keys: &Option<std::rc::Rc<Vec<u64>>>, j: usize) -> (u64, usize) {
    match keys {
        Some(k) => (k.get(j).copied().unwrap_or(0), j),
        None => (0, j),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_follow_the_columns_under_permutation() {
        // 3 列 2 行の問題と、列を (2, 0, 1)、行を逆に並べ替えたもの
        let rows = vec![(vec![(0, 1.0), (1, 2.0)], 0.0, 4.0), (vec![(1, 1.0), (2, -3.0)], f64::NEG_INFINITY, 5.0)];
        let c = [1.0, 0.0, 2.0];
        let (lb, ub) = ([0.0; 3], [1.0, 10.0, 1.0]);
        let k = column_keys(3, &rows, &c, &lb, &ub, &[true, false, true]);
        // 新しい列 q は元の列 perm[q]
        let perm = [2usize, 0, 1];
        let mut inv = [0usize; 3];
        for (q, &j) in perm.iter().enumerate() {
            inv[j] = q;
        }
        let rows2 = vec![
            (vec![(inv[2], -3.0), (inv[1], 1.0)], f64::NEG_INFINITY, 5.0),
            (vec![(inv[1], 2.0), (inv[0], 1.0)], 0.0, 4.0),
        ];
        let c2: Vec<f64> = perm.iter().map(|&j| c[j]).collect();
        let lb2: Vec<f64> = perm.iter().map(|&j| lb[j]).collect();
        let ub2: Vec<f64> = perm.iter().map(|&j| ub[j]).collect();
        let int2: Vec<bool> = perm.iter().map(|&j| [true, false, true][j]).collect();
        let k2 = column_keys(3, &rows2, &c2, &lb2, &ub2, &int2);
        for q in 0..3 {
            assert_eq!(k2[q], k[perm[q]]);
        }
        // 構造の違う列は違う鍵
        assert_ne!(k[0], k[2]);
    }
}
