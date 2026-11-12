//! 0/±1 の行列が完全単模 (TU) かの判定 (十分条件)。論文 7 の暗黙の整数 ([`super::implint_tu`]、van der Hulst・Walter 2025) と
//! DRCR の整数の組 ([`super::fold`]、van der Hulst 2026) で共通に使う (2026-11-11 に implint_tu.rs から移し、fold.rs の `simple_tu` を
//! これに置き換えた。analysis/drcr2_20261111.md の (5))。
//!
//! - [`tu_shape`]: 剥がし (非零が 1 つ以下の行・列を除く) の後の芯の連結成分ごとに、列の非零が全部 2 で行の 2 色塗りができる
//!   (Heller-Tompkins: 同じ符号の 2 行は別の色、逆の符号なら同じ色。有向グラフの接続行列の形) か、行の非零が全部 2 で列の 2 色塗りが
//!   できる (その転置) なら TU。どちらでもない成分が 1 つでもあれば偽 (TU でないとは限らない)。
//! - [`is_tu_er`]: [`tu_shape`] で分からなければ、重複の行の除去と行の分割で同値な形に直して判定し直す (論文 7 の `e`)。
//! - [`tu_cols_f64`]: 列の形 (列 → (行, 係数 f64)) の入口。係数が ±1 ちょうどでなければ偽。
//!
//! どちらの使い方でも、偽を返すのは安全側 (暗黙の整数にしない・組をまとめない)。真を返したのに TU でないと誤答になりうるので、
//! 変えたら両方の誤答の検査 (論文 7 の fuzz_tu.py、DRCR の fuzz_fold.py・fuzz_tiny・mip_fuzz_vs_highs) をやり直すこと。

/// 0/±1 の行列 (行ごとに列の番号の昇順の (列, 符号)) が、剥がし (非零が 1 つ以下の行・列を除く) の後の芯の連結成分ごとに、
/// 列の 2 色塗り (列の非零が全部 2) か行の 2 色塗り (行の非零が全部 2) で TU と分かるか ([`implied_integers_tu2`] の 4・5 と同じ判定)。
pub fn tu_shape(rows: &[Vec<(usize, i8)>], ncols: usize) -> bool {
    let m = rows.len();
    let mut cols: Vec<Vec<(usize, i8)>> = vec![Vec::new(); ncols];
    for (i, r) in rows.iter().enumerate() {
        for &(j, s) in r {
            cols[j].push((i, s));
        }
    }
    let mut rdeg: Vec<usize> = rows.iter().map(|r| r.len()).collect();
    let mut cdeg: Vec<usize> = cols.iter().map(|c| c.len()).collect();
    let mut ra: Vec<bool> = vec![true; m];
    let mut ca: Vec<bool> = vec![true; ncols];
    let mut stack: Vec<(bool, usize)> = (0..m).filter(|&i| rdeg[i] <= 1).map(|i| (true, i)).chain((0..ncols).filter(|&j| cdeg[j] <= 1).map(|j| (false, j))).collect();
    while let Some((is_row, k)) = stack.pop() {
        if is_row {
            if !ra[k] {
                continue;
            }
            ra[k] = false;
            for &(j, _) in &rows[k] {
                if ca[j] {
                    cdeg[j] -= 1;
                    if cdeg[j] == 1 {
                        stack.push((false, j));
                    }
                }
            }
        } else {
            if !ca[k] {
                continue;
            }
            ca[k] = false;
            for &(i, _) in &cols[k] {
                if ra[i] {
                    rdeg[i] -= 1;
                    if rdeg[i] == 1 {
                        stack.push((true, i));
                    }
                }
            }
        }
    }
    // 芯の連結成分ごと
    let mut rseen = vec![false; m];
    let mut color_r: Vec<i8> = vec![-1; m];
    let mut color_c: Vec<i8> = vec![-1; ncols];
    let mut cseen = vec![false; ncols];
    for r0 in 0..m {
        if !ra[r0] || rseen[r0] {
            continue;
        }
        let (mut rs, mut cs) = (vec![r0], Vec::new());
        rseen[r0] = true;
        let mut h = 0;
        while h < rs.len() {
            let r = rs[h];
            h += 1;
            for &(j, _) in &rows[r] {
                if ca[j] && !cseen[j] {
                    cseen[j] = true;
                    cs.push(j);
                    for &(i, _) in &cols[j] {
                        if ra[i] && !rseen[i] {
                            rseen[i] = true;
                            rs.push(i);
                        }
                    }
                }
            }
        }
        let lr = |i: usize| -> Vec<(usize, i8)> { rows[i].iter().copied().filter(|&(j, _)| ca[j]).collect() };
        let lc = |j: usize| -> Vec<(usize, i8)> { cols[j].iter().copied().filter(|&(i, _)| ra[i]).collect() };
        let col2 = cs.iter().all(|&j| lc(j).len() == 2);
        let row2 = rs.iter().all(|&i| lr(i).len() == 2);
        let two_color = |nodes: &[usize], color: &mut [i8], edges_of: &dyn Fn(usize) -> Vec<(usize, i8)>, other_end: &dyn Fn(usize, usize) -> (usize, i8)| -> bool {
            for &s in nodes {
                if color[s] >= 0 {
                    continue;
                }
                color[s] = 0;
                let mut q = vec![s];
                while let Some(v) = q.pop() {
                    for (e, sv) in edges_of(v) {
                        let (w, sw) = other_end(e, v);
                        let want = color[v] ^ ((sv == sw) as i8);
                        if color[w] < 0 {
                            color[w] = want;
                            q.push(w);
                        } else if color[w] != want {
                            return false;
                        }
                    }
                }
            }
            true
        };
        let ok = (col2 && two_color(&rs, &mut color_r, &|i| lr(i), &|j, i| *lc(j).iter().find(|&&(k, _)| k != i).unwrap()))
            || (row2 && two_color(&cs, &mut color_c, &|j| lc(j), &|i, j| *lr(i).iter().find(|&&(k, _)| k != j).unwrap()));
        if !ok {
            return false;
        }
    }
    true
}

/// [`tu_shape`] で分からなければ、行列を TU か同値な形に直してから判定し直す (`e`)。
/// 1. 符号を除いて同じ行を 1 本にする (同じ行を足しても TU は保たれる)。
/// 2. 分割: 行 R の非零が別の行 E に含まれ、R = σE (R の列の上で) なら、補助の連続列 z (= R の連続列の部分、境界なし) を足して、
///    E を E − σR + σz に、R を R − z (= 0、等式) に、元の R の行を z + (整数列の部分) に置き換えた問題を考える。z の定義が等式なので、
///    x への射影の実行可能集合は元と同じで、整数列の係数・左辺・右辺の整数性も保たれる。置き換えた行列が TU なら、整数列を固定した LP に
///    (x, z) が整数の最適解があり、x はその一部。元の R の行は z だけの行になって剥がれるので、行列の上では E ← E − σR + σz、
///    R ← R − z とすればよい。前処理が流れの保存の行で列 (枝 k: r → q) を消すと、q の行は q と r の行の和に、k の容量の行は r の行
///    (の連続列の部分) になる (mc11・sp150x300d)。分割で z が消えた枝 k の役をして、節点と枝の接続行列に戻る。
/// 分割は R の非零が 2 以上のときだけ、回数は行の数の 2 倍まで。
pub fn is_tu_er(rows: Vec<Vec<(usize, i8)>>, eq: &[bool], ncols: usize) -> bool {
    if tu_shape(&rows, ncols) {
        return true;
    }
    // 1. 重複を除く (最初の非零の符号を + に揃えた形で比べる)
    let norm = |r: &Vec<(usize, i8)>| -> Vec<(usize, i8)> {
        let f = r.first().map_or(1, |&(_, s)| s);
        r.iter().map(|&(j, s)| (j, s * f)).collect()
    };
    let mut seen: std::collections::HashSet<Vec<(usize, i8)>> = std::collections::HashSet::new();
    // 等式の行を先に残す (重複のうち等式でない方を落とす)
    let mut order0: Vec<usize> = (0..rows.len()).collect();
    order0.sort_by_key(|&i| (!eq[i], i));
    let mut keep: Vec<(Vec<(usize, i8)>, bool)> = Vec::new();
    for i in order0 {
        if rows[i].len() >= 2 && seen.insert(norm(&rows[i])) {
            keep.push((rows[i].clone(), eq[i]));
        }
    }
    let mut eq: Vec<bool> = keep.iter().map(|k| k.1).collect();
    let mut rows: Vec<Vec<(usize, i8)>> = keep.into_iter().map(|k| k.0).collect();
    let mut ncols = ncols;
    if tu_shape(&rows, ncols) {
        return true;
    }
    // 2. 分割 (短い R から)
    let limit = 2 * rows.len() + 10;
    let mut done = 0;
    let mut val: Vec<i8> = vec![0; ncols];
    loop {
        let mut cols: Vec<Vec<usize>> = vec![Vec::new(); ncols];
        for (i, r) in rows.iter().enumerate() {
            for &(j, _) in r {
                cols[j].push(i);
            }
        }
        let mut order: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].len() >= 2).collect();
        // R は不等式の行 (前処理が消した枝の容量の行) を先に、短いものから
        order.sort_by_key(|&i| (eq[i], rows[i].len(), i));
        let mut found: Option<(usize, usize, i8)> = None;
        'outer: for &r in &order {
            let (j0, s0) = rows[r][0];
            // E は R の最初の列を含む、R より長い行
            for &e in &cols[j0] {
                if e == r || rows[e].len() <= rows[r].len() {
                    continue;
                }
                for &(j, sj) in &rows[e] {
                    val[j] = sj;
                }
                let se0 = val[j0];
                let sigma = s0 * se0;
                let contained = rows[r].iter().all(|&(j, sj)| val[j] != 0 && sj == sigma * val[j]);
                for &(j, _) in &rows[e] {
                    val[j] = 0;
                }
                if contained {
                    found = Some((r, e, sigma));
                    break 'outer;
                }
            }
        }
        let Some((r, e, sigma)) = found else { break };
        let z = ncols;
        ncols += 1;
        val.push(0);
        let rset: std::collections::HashSet<usize> = rows[r].iter().map(|&(j, _)| j).collect();
        let mut ne: Vec<(usize, i8)> = rows[e].iter().copied().filter(|&(j, _)| !rset.contains(&j)).collect();
        ne.push((z, sigma));
        rows[e] = ne;
        rows[r].push((z, -1));
        // R − z = 0 は等式
        eq[r] = true;
        done += 1;
        if tu_shape(&rows, ncols) {
            return true;
        }
        if done >= limit {
            break;
        }
    }
    false
}

/// 列の形 (列 → (行, 係数)) の行列が [`tu_shape`] で TU と分かるか。係数が ±1 ちょうどでなければ偽 (DRCR の整数の組の判定の入口)。
pub fn tu_cols_f64(nrows: usize, cols: &[Vec<(usize, f64)>]) -> bool {
    let mut rows: Vec<Vec<(usize, i8)>> = vec![Vec::new(); nrows];
    for (j, c) in cols.iter().enumerate() {
        for &(i, a) in c {
            let s = if a == 1.0 {
                1
            } else if a == -1.0 {
                -1
            } else {
                return false;
            };
            rows[i].push((j, s));
        }
    }
    for r in rows.iter_mut() {
        r.sort_unstable_by_key(|e| e.0);
    }
    tu_shape(&rows, cols.len())
}

/// [`tu_cols_f64`] で分からなければ、剥がし (非零が 1 つ以下の行・列を除く。TU は保たれる) の後の芯の連結成分ごとに、ネットワーク
/// 行列かその転置か ([`super::network::is_network_or_transpose`]、MATREC の列の追加と証明書の検査) を確かめる。どの成分も通れば TU
/// (ブロック対角の TU 行列は TU)。`ENOMOTO_MIP_FOLD` の値の文字 `n` (analysis/drcrtu_20261111.md)。成分の非零が `max_nnz` を超えたら偽。
pub fn tu_cols_f64_net(nrows: usize, cols: &[Vec<(usize, f64)>], max_nnz: usize) -> bool {
    if tu_cols_f64(nrows, cols) {
        return true;
    }
    let ncols = cols.len();
    let mut rows: Vec<Vec<usize>> = vec![Vec::new(); nrows];
    let mut colb: Vec<Vec<(usize, bool)>> = Vec::with_capacity(ncols);
    for (j, c) in cols.iter().enumerate() {
        let mut v = Vec::with_capacity(c.len());
        for &(i, a) in c {
            let neg = if a == 1.0 {
                false
            } else if a == -1.0 {
                true
            } else {
                return false;
            };
            rows[i].push(j);
            v.push((i, neg));
        }
        colb.push(v);
    }
    // 剥がし
    let mut rdeg: Vec<usize> = rows.iter().map(|r| r.len()).collect();
    let mut cdeg: Vec<usize> = colb.iter().map(|c| c.len()).collect();
    let mut ra = vec![true; nrows];
    let mut ca = vec![true; ncols];
    let mut stack: Vec<(bool, usize)> = (0..nrows).filter(|&i| rdeg[i] <= 1).map(|i| (true, i)).chain((0..ncols).filter(|&j| cdeg[j] <= 1).map(|j| (false, j))).collect();
    while let Some((is_row, k)) = stack.pop() {
        if is_row {
            if !ra[k] {
                continue;
            }
            ra[k] = false;
            for &j in &rows[k] {
                if ca[j] {
                    cdeg[j] -= 1;
                    if cdeg[j] == 1 {
                        stack.push((false, j));
                    }
                }
            }
        } else {
            if !ca[k] {
                continue;
            }
            ca[k] = false;
            for &(i, _) in &colb[k] {
                if ra[i] {
                    rdeg[i] -= 1;
                    if rdeg[i] == 1 {
                        stack.push((true, i));
                    }
                }
            }
        }
    }
    // 芯の連結成分ごと
    let mut rseen = vec![false; nrows];
    let mut rloc = vec![usize::MAX; nrows];
    for r0 in 0..nrows {
        if !ra[r0] || rseen[r0] {
            continue;
        }
        let (mut rs, mut cs) = (vec![r0], Vec::new());
        rseen[r0] = true;
        let mut cseen_local: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut h = 0;
        while h < rs.len() {
            let r = rs[h];
            h += 1;
            for &j in &rows[r] {
                if ca[j] && cseen_local.insert(j) {
                    cs.push(j);
                    for &(i, _) in &colb[j] {
                        if ra[i] && !rseen[i] {
                            rseen[i] = true;
                            rs.push(i);
                        }
                    }
                }
            }
        }
        for (k, &r) in rs.iter().enumerate() {
            rloc[r] = k;
        }
        let sub: Vec<Vec<(usize, bool)>> = cs.iter().map(|&j| colb[j].iter().filter(|e| ra[e.0]).map(|&(i, s)| (rloc[i], s)).collect()).collect();
        let nnz: usize = sub.iter().map(|c| c.len()).sum();
        if nnz > max_nnz || !super::network::is_network_or_transpose(rs.len(), &sub) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incidence_is_tu_and_odd_cycle_is_not() {
        // 有向グラフの接続行列 (列 = 辺) は TU
        let cols = vec![vec![(0, 1.0), (1, -1.0)], vec![(1, 1.0), (2, -1.0)], vec![(0, -1.0), (2, 1.0)]];
        assert!(tu_cols_f64(3, &cols));
        // 奇閉路の無向の接続行列 (同じ符号) は判定できない
        let cols = vec![vec![(0, 1.0), (1, 1.0)], vec![(1, 1.0), (2, 1.0)], vec![(0, 1.0), (2, 1.0)]];
        assert!(!tu_cols_f64(3, &cols));
        // ±1 でない係数は偽
        assert!(!tu_cols_f64(2, &[vec![(0, 2.0), (1, 1.0)]]));
    }
}
