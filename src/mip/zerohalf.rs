//! {0, 1/2}-Chvátal-Gomory カット (zerohalf、SCIP の `sepa_zerohalf.c` を簡略化したもの)。
//!
//! 整数変数だけ・整数係数・整数右辺の行 `a_i x <= b_i` を、変数を LP 値に近い方の境界で置き換えた
//! `x' >= 0` の空間で mod 2 で扱う。行の部分集合 R に乗数 1/2 を掛けて足し丸めると
//! `sum_j floor(c_j / 2) x'_j <= floor(beta / 2)` (`c = sum_R a_i`、`beta = sum_R b'_i`) が成り立ち、
//! beta が奇数なら LP 点での違反量は `(1 - sum_R s_i - sum_{c_j 奇数} x'_j) / 2` (`s_i` は行の余裕)。
//! R は SCIP と同じく、余裕 0 の行を軸にした mod 2 のガウス消去で探す (LP 値の大きい列から消す)。
//! 消去は違反量の見積りにだけ使い、カットは選んだ R から元の係数で厳密に作り直す。

use super::cuts::RawCut;

const MIN_VIOL: f64 = 0.1;

/// mod 2 の行: 列の奇偶 (ビット集合)、右辺の奇偶、余裕 (見積り)、元の行の組み合わせ (ビット集合)。
#[derive(Clone)]
struct Mod2Row {
    cols: Vec<u64>,
    rhs_odd: bool,
    slack: f64,
    combo: Vec<u64>,
}

fn bit(v: &[u64], i: usize) -> bool {
    (v[i / 64] >> (i % 64)) & 1 == 1
}

fn flip(v: &mut [u64], i: usize) {
    v[i / 64] ^= 1u64 << (i % 64);
}

fn xor_into(a: &mut [u64], b: &[u64]) {
    for (x, y) in a.iter_mut().zip(b) {
        *x ^= *y;
    }
}

/// zerohalf カットを作る。`rows`/`row_lo`/`row_up` は元の行、`lo`/`up` は大域的な境界、`x` は LP 解。
/// 戻り値のカットは構造変数だけの式。
pub fn zerohalf_cuts(rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64], is_int: &[bool], lo: &[f64], up: &[f64], x: &[f64], max_cuts: usize) -> Vec<RawCut> {
    let n = x.len();
    let is_integral = |v: f64| (v - v.round()).abs() <= 1e-9;
    // 各列の置き換え: 下限側なら (true, l)、上限側なら (false, u)。x'_j の LP 値。
    let mut use_lower = vec![true; n];
    let mut xbar = vec![0.0; n];
    let mut col_ok = vec![false; n];
    for j in 0..n {
        if !is_int[j] {
            continue;
        }
        let (l, u) = (lo[j], up[j]);
        let lf = l.is_finite() && is_integral(l);
        let uf = u.is_finite() && is_integral(u);
        if !lf && !uf {
            continue;
        }
        col_ok[j] = true;
        use_lower[j] = lf && (!uf || x[j] - l <= u - x[j]);
        xbar[j] = if use_lower[j] { (x[j] - l).max(0.0) } else { (u - x[j]).max(0.0) };
    }
    // 対象の行の向き (元の行番号、符号 +1 は <= 側、-1 は >= 側を反転したもの)
    let mut sides: Vec<(usize, f64)> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        if r.is_empty() || !r.iter().all(|&(j, a)| col_ok[j] && is_integral(a)) {
            continue;
        }
        let act: f64 = r.iter().map(|&(j, a)| a * x[j]).sum();
        for (sg, b) in [(1.0, row_up[i]), (-1.0, row_lo[i])] {
            if !b.is_finite() || !is_integral(b) {
                continue;
            }
            let slack = sg * (b - act);
            if slack < 1.0 - 2.0 * MIN_VIOL {
                sides.push((i, sg));
            }
        }
    }
    if sides.is_empty() {
        return Vec::new();
    }
    // 列: LP 値が境界から離れている整数列だけ (境界上の列は違反量に効かない)
    let mut col_index = vec![usize::MAX; n];
    let mut cols: Vec<usize> = Vec::new();
    for j in 0..n {
        if col_ok[j] && xbar[j] > 1e-9 {
            col_index[j] = cols.len();
            cols.push(j);
        }
    }
    let nc = cols.len();
    let ns = sides.len();
    let (wc, ws) = (nc.div_ceil(64).max(1), ns.div_ceil(64).max(1));
    if (ns as f64) * (wc + ws) as f64 > 5e7 {
        return Vec::new();
    }
    // mod 2 の行を作る
    let mut m2: Vec<Mod2Row> = Vec::with_capacity(ns);
    for (s, &(i, sg)) in sides.iter().enumerate() {
        let mut cv = vec![0u64; wc];
        let mut beta = if sg > 0.0 { row_up[i] } else { -row_lo[i] };
        let mut act = 0.0;
        for &(j, a) in &rows[i] {
            let a = sg * a;
            act += a * x[j];
            // 置き換えた右辺: beta -= a * (境界)
            beta -= a * if use_lower[j] { lo[j] } else { up[j] };
            if (a.round() as i64).rem_euclid(2) == 1 && col_index[j] != usize::MAX {
                flip(&mut cv, col_index[j]);
            }
        }
        let slack = (if sg > 0.0 { row_up[i] } else { -row_lo[i] }) - act;
        let mut combo = vec![0u64; ws];
        flip(&mut combo, s);
        m2.push(Mod2Row { cols: cv, rhs_odd: (beta.round() as i64).rem_euclid(2) == 1, slack: slack.max(0.0), combo });
    }
    // 消去: LP 値の大きい列から、その列を含む余裕 0 の行を軸に他の行から消す
    let mut order: Vec<usize> = (0..nc).collect();
    order.sort_by(|&a, &b| xbar[cols[b]].total_cmp(&xbar[cols[a]]));
    let mut is_pivot = vec![false; ns];
    for &c in &order {
        let Some(p) = (0..ns).find(|&r| !is_pivot[r] && m2[r].slack <= 1e-9 && bit(&m2[r].cols, c)) else { continue };
        is_pivot[p] = true;
        let piv = m2[p].clone();
        for r in 0..ns {
            if r != p && bit(&m2[r].cols, c) {
                xor_into(&mut m2[r].cols, &piv.cols);
                m2[r].rhs_odd ^= piv.rhs_odd;
                m2[r].slack += piv.slack;
                xor_into(&mut m2[r].combo, &piv.combo);
            }
        }
        // 軸の行ではこの列は残るので、その LP 値を余裕に足す (以後この列は数えない)
        m2[p].slack += xbar[cols[c]];
        flip(&mut m2[p].cols, c);
    }
    // 見積りで違反しそうな組み合わせからカットを作る
    let mut cands: Vec<(f64, Vec<u64>)> = Vec::new();
    for r in &m2 {
        if !r.rhs_odd {
            continue;
        }
        let mut val = r.slack;
        for (c, &j) in cols.iter().enumerate() {
            if bit(&r.cols, c) {
                val += xbar[j];
                if val >= 1.0 - 2.0 * MIN_VIOL {
                    break;
                }
            }
        }
        if val < 1.0 - 2.0 * MIN_VIOL {
            cands.push((val, r.combo.clone()));
        }
    }
    cands.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<RawCut> = Vec::new();
    let mut seen: Vec<Vec<u64>> = Vec::new();
    let mut c = vec![0.0f64; n];
    for (_, combo) in cands {
        if out.len() >= max_cuts {
            break;
        }
        if seen.contains(&combo) {
            continue;
        }
        seen.push(combo.clone());
        // 元の係数で c = sum_R a_i、beta = sum_R b_i
        let mut touched: Vec<usize> = Vec::new();
        let mut beta = 0.0;
        for (s, &(i, sg)) in sides.iter().enumerate() {
            if !bit(&combo, s) {
                continue;
            }
            beta += if sg > 0.0 { row_up[i] } else { -row_lo[i] };
            for &(j, a) in &rows[i] {
                if c[j] == 0.0 {
                    touched.push(j);
                }
                c[j] += sg * a;
                if c[j] == 0.0 {
                    c[j] = 1e-300;
                }
            }
        }
        // x' の空間で floor(c'/2) x' <= floor(beta'/2)、元の変数に戻す
        let mut coefs: Vec<(usize, f64)> = Vec::new();
        let mut bp = beta;
        let mut cprime: Vec<(usize, f64)> = Vec::with_capacity(touched.len());
        for &j in &touched {
            let cj = c[j].round();
            c[j] = 0.0;
            if cj == 0.0 {
                continue;
            }
            if use_lower[j] {
                bp -= cj * lo[j];
                cprime.push((j, cj));
            } else {
                bp -= cj * up[j];
                cprime.push((j, -cj));
            }
        }
        let rhs_p = (bp.round() / 2.0).floor();
        let mut rhs = rhs_p;
        for (j, cp) in cprime {
            let g = (cp / 2.0).floor();
            if g == 0.0 {
                continue;
            }
            if use_lower[j] {
                // g (x - l)
                coefs.push((j, g));
                rhs += g * lo[j];
            } else {
                // g (u - x)
                coefs.push((j, -g));
                rhs -= g * up[j];
            }
        }
        if coefs.is_empty() {
            continue;
        }
        let act: f64 = coefs.iter().map(|&(j, a)| a * x[j]).sum();
        if act > rhs + 1e-6 {
            out.push(RawCut { coefs, rhs });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_cycle_in_a_triangle() {
        // 頂点被覆の三角形: x0 + x1 <= 1, x1 + x2 <= 1, x0 + x2 <= 1、LP 点 (0.5, 0.5, 0.5)
        // → x0 + x1 + x2 <= 1
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(1, 1.0), (2, 1.0)], vec![(0, 1.0), (2, 1.0)]];
        let lo = [0.0; 3];
        let up = [1.0; 3];
        let cuts = zerohalf_cuts(&rows, &[f64::NEG_INFINITY; 3], &[1.0; 3], &[true; 3], &lo, &up, &[0.5, 0.5, 0.5], 10);
        assert!(!cuts.is_empty(), "no zerohalf cut");
        let c = &cuts[0];
        let act: f64 = c.coefs.iter().map(|&(j, a)| a * 0.5).sum();
        assert!(act > c.rhs + 0.4, "{:?} <= {}", c.coefs, c.rhs);
        for pt in [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]] {
            let a: f64 = c.coefs.iter().map(|&(j, v)| v * pt[j]).sum();
            assert!(a <= c.rhs + 1e-9);
        }
    }

    /// 乱数の小さな整数行で、zerohalf カットが全ての整数実行可能点で成り立つことを総当たりで確かめる。
    #[test]
    fn random_validity() {
        let mut seed = 777u64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut found = 0;
        for _ in 0..3000 {
            let n = 2 + (rnd() * 4.0) as usize;
            let m = 2 + (rnd() * 5.0) as usize;
            let lo: Vec<f64> = (0..n).map(|_| -(rnd() * 2.0).floor()).collect();
            let up: Vec<f64> = (0..n).map(|j| lo[j] + 1.0 + (rnd() * 2.0).floor()).collect();
            let mut rows = Vec::new();
            let (mut rl, mut ru) = (Vec::new(), Vec::new());
            for _ in 0..m {
                let mut r: Vec<(usize, f64)> = Vec::new();
                for j in 0..n {
                    if rnd() < 0.6 {
                        let a = ((rnd() - 0.5) * 6.0).round();
                        if a != 0.0 {
                            r.push((j, a));
                        }
                    }
                }
                if r.is_empty() {
                    continue;
                }
                rows.push(r);
                let b = ((rnd() - 0.3) * 6.0).round();
                if rnd() < 0.3 {
                    rl.push(b);
                    ru.push(b);
                } else {
                    rl.push(f64::NEG_INFINITY);
                    ru.push(b);
                }
            }
            let x: Vec<f64> = (0..n).map(|j| lo[j] + rnd() * (up[j] - lo[j])).collect();
            let cuts = zerohalf_cuts(&rows, &rl, &ru, &vec![true; n], &lo, &up, &x, 10);
            found += cuts.len();
            // 全ての整数点を列挙
            let mut pt: Vec<f64> = lo.clone();
            loop {
                let feas = rows.iter().enumerate().all(|(i, r)| {
                    let a: f64 = r.iter().map(|&(j, v)| v * pt[j]).sum();
                    a <= ru[i] + 1e-9 && a >= rl[i] - 1e-9
                });
                if feas {
                    for c in &cuts {
                        let a: f64 = c.coefs.iter().map(|&(j, v)| v * pt[j]).sum();
                        assert!(a <= c.rhs + 1e-9, "invalid zerohalf cut {:?} <= {} at {pt:?}; rows {rows:?} {rl:?} {ru:?}", c.coefs, c.rhs);
                    }
                }
                let mut k = 0;
                while k < n {
                    pt[k] += 1.0;
                    if pt[k] <= up[k] {
                        break;
                    }
                    pt[k] = lo[k];
                    k += 1;
                }
                if k == n {
                    break;
                }
            }
        }
        assert!(found > 50, "only {found} cuts");
    }
}
