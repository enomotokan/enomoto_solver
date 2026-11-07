//! 試作 `ENOMOTO_MIP_HNFPRE_LAT` (既定では使わない): 整数列を格子の簡約した基底で置き換える書き直し
//! (Aardal–Hurkens–Lenstra 2000 の格子の書き直しを、最適化問題・連続列つきの問題に広げたもの)。
//!
//! 整数列 x (n_I 本) を `x = U^T ν` (U は n_I × n_I の単模な整数行列、ν は整数 (上下限は x の箱から出る範囲)) と置き換える。U の行は
//! 埋め込み `[e_j | N A_eq e_j]` (A_eq は等式の行の整数列の係数、N は大きな重み) を LLL で簡約した基底の前半で、
//! 先頭の n_I − rank 本は A_eq の核 (A_eq u = 0) になる。U は単模なので、x の整数解と ν の整数解は 1 対 1 に対応し
//! (問題は同値)、元の整数列の上下限は行 `lo_j ≤ Σ_k U_kj ν_k ≤ up_j` になる。連続列はそのまま残す。
//! 分枝は ν の上で行われ、核の向き (等式を保つ向き) の短いベクトルで分ける形になる。market split (Cornuéjols–Dawande 1999) の
//! 型で分枝のノードが桁で減る (AHL 2000)。
//!
//! 使う条件 ([`reform`]): 整数列が 2〜`ENOMOTO_T_MIP_HNFPRE_LAT_MAXN` (既定 120) 本、整数列はすべて上下限が有限、整数列の係数が
//! 整数で 3 本以上の整数列を含み、係数の絶対値の最大が 2 以上の等式が 1 本以上ある (集合分割のような ±1 の行だけの問題には使わない)。

use super::problem::MipProblem;

pub(crate) fn on() -> bool {
    env_str!("ENOMOTO_MIP_HNFPRE_LAT").is_some_and(|v| v != "0")
}

/// 書き直した問題と、元へ戻すための情報。
pub(crate) struct LatReform {
    pub q: MipProblem,
    /// 元の整数列 (q の列 0..ni が ν)。
    pub int_cols: Vec<usize>,
    /// 元の連続列 (q の列 ni.. に順に対応)。
    pub cont_cols: Vec<usize>,
    /// U (ni 行 × ni 列、行 k が ν_k の向き)。
    pub u: Vec<Vec<i64>>,
    pub ker: usize,
    pub lll_secs: f64,
}

impl LatReform {
    /// q の解 y を元の問題の解に戻す (ν は整数に丸める)。
    pub fn map_back(&self, n: usize, y: &[f64]) -> Vec<f64> {
        let ni = self.int_cols.len();
        let mut x = vec![0.0; n];
        let nu: Vec<i64> = y[..ni].iter().map(|v| v.round() as i64).collect();
        for (t, &j) in self.int_cols.iter().enumerate() {
            let mut s: i128 = 0;
            for k in 0..ni {
                s += self.u[k][t] as i128 * nu[k] as i128;
            }
            x[j] = s as f64;
        }
        for (t, &j) in self.cont_cols.iter().enumerate() {
            x[j] = y[ni + t];
        }
        x
    }
}

/// 整数の行ベクトルの基底を LLL で簡約する (δ = 0.99、Gram–Schmidt は f64)。行の操作は整数のままなので、簡約の出来が
/// 浮動小数の誤差で悪くなっても、結果は元の格子の基底 (単模な変換) のまま。溢れ・手間の上限で `None`。
fn lll(b: &mut [Vec<i128>], max_steps: usize, deadline: std::time::Instant) -> Option<()> {
    let n = b.len();
    if n <= 1 {
        return Some(());
    }
    let d = b[0].len();
    let dot = |x: &[f64], y: &[f64]| x.iter().zip(y).map(|(a, c)| a * c).sum::<f64>();
    let tof = |v: &[i128]| v.iter().map(|&a| a as f64).collect::<Vec<f64>>();
    let mut bs: Vec<Vec<f64>> = vec![vec![0.0; d]; n]; // b*_i
    let mut bb: Vec<f64> = vec![0.0; n]; // |b*_i|^2
    let mut mu: Vec<Vec<f64>> = vec![vec![0.0; n]; n];
    // 行 i の Gram–Schmidt を (0..i が正しい前提で) 作り直す
    let gs = |i: usize, b: &[Vec<i128>], bs: &mut Vec<Vec<f64>>, bb: &mut Vec<f64>, mu: &mut Vec<Vec<f64>>| {
        let bi = tof(&b[i]);
        let mut v = bi.clone();
        for j in 0..i {
            let m = if bb[j] > 1e-12 { dot(&bi, &bs[j]) / bb[j] } else { 0.0 };
            mu[i][j] = m;
            for t in 0..d {
                v[t] -= m * bs[j][t];
            }
        }
        bb[i] = dot(&v, &v);
        bs[i] = v;
    };
    gs(0, b, &mut bs, &mut bb, &mut mu);
    let mut k = 1;
    let mut done_upto = 0; // 0..=done_upto の Gram–Schmidt は正しい
    let mut steps = 0usize;
    while k < n {
        steps += 1;
        if steps > max_steps || (steps % 64 == 0 && std::time::Instant::now() > deadline) {
            return None;
        }
        if done_upto < k {
            gs(k, b, &mut bs, &mut bb, &mut mu);
            done_upto = k;
        }
        // 大きさの簡約
        for j in (0..k).rev() {
            let q = mu[k][j].round();
            if q != 0.0 {
                if q.abs() > 1e15 {
                    return None;
                }
                let qi = q as i128;
                let (lo, hi) = b.split_at_mut(k);
                for t in 0..d {
                    hi[0][t] = hi[0][t].checked_sub(qi.checked_mul(lo[j][t])?)?;
                    if hi[0][t].unsigned_abs() > (1u128 << 100) {
                        return None;
                    }
                }
                for l in 0..j {
                    mu[k][l] -= q * mu[j][l];
                }
                mu[k][j] -= q;
            }
        }
        if bb[k] < (0.99 - mu[k][k - 1] * mu[k][k - 1]) * bb[k - 1] {
            b.swap(k, k - 1);
            // k−1 と k の Gram–Schmidt を作り直す (k より後は、そこへ進むときに作り直す)
            gs(k - 1, b, &mut bs, &mut bb, &mut mu);
            gs(k, b, &mut bs, &mut bb, &mut mu);
            done_upto = k;
            k = (k - 1).max(1);
        } else {
            k += 1;
        }
    }
    Some(())
}

/// 単模な整数行列の逆行列 (整数)。f64 の Gauss–Jordan で求めて丸め、`u w = I` を i128 で確かめる。
fn int_inverse(u: &[Vec<i64>]) -> Option<Vec<Vec<i64>>> {
    let n = u.len();
    let mut a: Vec<Vec<f64>> = (0..n).map(|i| {
        let mut r: Vec<f64> = u[i].iter().map(|&v| v as f64).collect();
        r.extend((0..n).map(|j| if i == j { 1.0 } else { 0.0 }));
        r
    }).collect();
    for c in 0..n {
        let p = (c..n).max_by(|&x, &y| a[x][c].abs().partial_cmp(&a[y][c].abs()).unwrap())?;
        if a[p][c].abs() < 1e-9 {
            return None;
        }
        a.swap(c, p);
        let pv = a[c][c];
        for v in a[c].iter_mut() {
            *v /= pv;
        }
        for r in 0..n {
            if r != c && a[r][c] != 0.0 {
                let f = a[r][c];
                let (rc, rr) = if r < c { let (x, y) = a.split_at_mut(c); (&y[0], &mut x[r]) } else { let (x, y) = a.split_at_mut(r); (&x[c], &mut y[0]) };
                for t in 0..2 * n {
                    rr[t] -= f * rc[t];
                }
            }
        }
    }
    let w: Vec<Vec<i64>> = (0..n).map(|i| (0..n).map(|j| a[i][n + j].round() as i64).collect()).collect();
    for i in 0..n {
        for j in 0..n {
            let s: i128 = (0..n).map(|t| u[i][t] as i128 * w[t][j] as i128).sum();
            if s != (i == j) as i128 {
                return None;
            }
        }
    }
    Some(w)
}

/// 書き直しを作る (条件に合わなければ `None`)。
pub(crate) fn reform(p: &MipProblem, verbose: bool) -> Option<LatReform> {
    let t0 = std::time::Instant::now();
    let maxn = tunable!("ENOMOTO_T_MIP_HNFPRE_LAT_MAXN", 120.0, f64) as usize;
    let int_cols: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j]).collect();
    let cont_cols: Vec<usize> = (0..p.n).filter(|&j| !p.is_int[j]).collect();
    let ni = int_cols.len();
    if ni < 2 || ni > maxn {
        return None;
    }
    if int_cols.iter().any(|&j| !p.col_lo[j].is_finite() || !p.col_up[j].is_finite()) {
        return None;
    }
    let mut pos = vec![usize::MAX; p.n];
    for (t, &j) in int_cols.iter().enumerate() {
        pos[j] = t;
    }
    // 等式の整数列の部分 (係数が整数のもの)
    let mut aeq: Vec<Vec<i128>> = Vec::new();
    let mut useful = false;
    for i in 0..p.m {
        if p.row_lo[i] != p.row_up[i] {
            continue;
        }
        let mut r = vec![0i128; ni];
        let mut ok = true;
        let (mut nz, mut amax) = (0usize, 0f64);
        for &(j, a) in &p.rows[i] {
            if pos[j] != usize::MAX {
                if (a - a.round()).abs() > 1e-9 || a.abs() > 1e9 {
                    ok = false;
                    break;
                }
                r[pos[j]] = a.round() as i128;
                nz += 1;
                amax = amax.max(a.abs());
            }
        }
        if ok && nz > 0 {
            useful |= nz >= 3 && amax >= 2.0;
            aeq.push(r);
        }
    }
    if !useful {
        return None;
    }
    // 埋め込み [e_j | N A_eq e_j] (N は核の向きが先に来るように十分大きく)
    let nw: i128 = (tunable!("ENOMOTO_T_MIP_HNFPRE_LAT_N", 10000.0, f64) as i128).max(1);
    let mut b: Vec<Vec<i128>> = (0..ni)
        .map(|j| {
            let mut v = vec![0i128; ni + aeq.len()];
            v[j] = 1;
            for (i, r) in aeq.iter().enumerate() {
                v[ni + i] = nw * r[j];
            }
            v
        })
        .collect();
    let deadline = t0 + std::time::Duration::from_secs_f64(tunable!("ENOMOTO_T_MIP_HNFPRE_LAT_SECS", 2.0, f64));
    if lll(&mut b, 5_000_000, deadline).is_none() {
        if verbose {
            eprintln!("HNFPRE_LAT: LLL gave up ({:.3}s)", t0.elapsed().as_secs_f64());
        }
        return None;
    }
    let mut u: Vec<Vec<i64>> = Vec::with_capacity(ni);
    for row in &b {
        let mut r = Vec::with_capacity(ni);
        for &v in &row[..ni] {
            if v.unsigned_abs() > (1u128 << 40) {
                return None;
            }
            r.push(v as i64);
        }
        u.push(r);
    }
    let ker = b.iter().filter(|r| r[ni..].iter().all(|&v| v == 0)).count();
    // ν = U^{-T} x なので、x の箱から ν の上下限が出る。U は単模なので逆行列 W = U^{-1} は整数 (f64 の Gauss–Jordan で求めて
    // 丸め、U W = I を整数で確かめる。合わなければ書き直さない)。上下限のない列を作らない (列だけの基底からの根の LP が自由変数で
    // 誤る件、analysis/hnfpre_20261107.md §9。ヒューリスティクスにも上下限がある方がよい)
    let w = int_inverse(&u)?;
    let mut nu_lo = vec![0.0f64; ni];
    let mut nu_up = vec![0.0f64; ni];
    for k in 0..ni {
        let (mut l, mut h) = (0.0f64, 0.0f64);
        for (t, &j) in int_cols.iter().enumerate() {
            let c = w[t][k] as f64;
            if c > 0.0 {
                l += c * p.col_lo[j];
                h += c * p.col_up[j];
            } else if c < 0.0 {
                l += c * p.col_up[j];
                h += c * p.col_lo[j];
            }
        }
        nu_lo[k] = l.round();
        nu_up[k] = h.round();
    }
    // 新しい問題
    let nq = ni + cont_cols.len();
    let mut col_lo = nu_lo;
    let mut col_up = nu_up;
    let mut cost: Vec<f64> = (0..ni).map(|k| int_cols.iter().enumerate().map(|(t, &j)| p.cost[j] * u[k][t] as f64).sum()).collect();
    let mut is_int = vec![true; ni];
    for &j in &cont_cols {
        col_lo.push(p.col_lo[j]);
        col_up.push(p.col_up[j]);
        cost.push(p.cost[j]);
        is_int.push(false);
    }
    let mut cpos = vec![usize::MAX; p.n];
    for (t, &j) in cont_cols.iter().enumerate() {
        cpos[j] = ni + t;
    }
    let mut rows: Vec<Vec<(usize, f64)>> = Vec::with_capacity(p.m + ni);
    let (mut row_lo, mut row_up) = (Vec::with_capacity(p.m + ni), Vec::with_capacity(p.m + ni));
    let mut acc = vec![0.0f64; nq];
    for i in 0..p.m {
        let mut touched: Vec<usize> = Vec::new();
        for &(j, a) in &p.rows[i] {
            if pos[j] != usize::MAX {
                let t = pos[j];
                for k in 0..ni {
                    if u[k][t] != 0 {
                        if acc[k] == 0.0 {
                            touched.push(k);
                        }
                        acc[k] += a * u[k][t] as f64;
                    }
                }
            } else {
                let c = cpos[j];
                if acc[c] == 0.0 {
                    touched.push(c);
                }
                acc[c] += a;
            }
        }
        touched.sort_unstable();
        touched.dedup();
        let r: Vec<(usize, f64)> = touched.iter().filter_map(|&c| (acc[c] != 0.0).then_some((c, acc[c]))).collect();
        for &c in &touched {
            acc[c] = 0.0;
        }
        rows.push(r);
        row_lo.push(p.row_lo[i]);
        row_up.push(p.row_up[i]);
    }
    for (t, &j) in int_cols.iter().enumerate() {
        rows.push((0..ni).filter(|&k| u[k][t] != 0).map(|k| (k, u[k][t] as f64)).collect());
        row_lo.push(p.col_lo[j]);
        row_up.push(p.col_up[j]);
    }
    let q = MipProblem::from_rows(col_lo, col_up, cost, p.offset, p.sense_sign, is_int, rows, row_lo, row_up);
    let lll_secs = t0.elapsed().as_secs_f64();
    if verbose || env_str!("ENOMOTO_MIP_HNFPRE_QUIET").is_none() {
        let umax = u.iter().flatten().map(|v| v.abs()).max().unwrap_or(0);
        eprintln!("HNFPRE_LAT: {ni} integer columns, {} equality rows, kernel {ker}, max |U| {umax}, N {nw}, {:.3}s", aeq.len(), lll_secs);
    }
    // 診断用: 書き直した問題を書き出す
    if let Some(path) = env_str!("ENOMOTO_MIP_HNFPRE_LAT_DUMP") {
        let _ = q.write_mps(path);
    }
    Some(LatReform { q, int_cols, cont_cols, u, ker, lll_secs })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lll_keeps_lattice() {
        // 単模な変換か (行列式 ±1) を 2x2 で確かめる
        let mut b = vec![vec![1i128, 0, 1000 * 7], vec![0, 1, 1000 * 11]];
        lll(&mut b, 1000, std::time::Instant::now() + std::time::Duration::from_secs(1)).unwrap();
        let det = b[0][0] * b[1][1] - b[0][1] * b[1][0];
        assert_eq!(det.abs(), 1);
        // 核 (7 x + 11 y = 0 の (11, -7)) が先頭に来る
        assert_eq!(b[0][2], 0);
        assert_eq!(b[0][0].abs(), 11);
    }
}
