//! 試作 `ENOMOTO_MIP_REDSPLIT` (既定では使わない): 単体表の行からの GMI の作り直し (`analysis/redsplit_20261107.md`)。
//!
//! ENOMOTO の「GMI」は、基底にある小数の整数列の単体表の行 (基底逆行列の行で LP の行を論理変数つきの等式として集約したもの)
//! を CMIR に通したもの ([`super::cuts::generate_cuts`])。単体表の行で LP 値が境界の内側にある整数の項は基底の列 (係数 1) だけ
//! なので、δ の候補は 1, 1/2, 1/4, 1/8 で、これは「行を k = 1, 2, 4, 8 倍してから GMI を作り、効き目の最良を採る」に当たる。
//!
//! - `a` (k-cuts、Cornuéjols・Li・Vandenbussche 2003): δ = 1/k (k = 1, …, K、`ENOMOTO_T_MIP_REDSPLIT_K`、既定 10) で同じ行から
//!   CMIR を作り ([`super::cuts::cmir_with_deltas`])、既存の候補と一緒に効き目・質の最良の 1 本を残す。
//! - `b` (reduce-and-split、Andersen・Cornuéjols・Li 2005): 単体表の行 (根で分離に使った行、`ENOMOTO_T_MIP_REDSPLIT_ROWS`
//!   (既定 100) 本まで) の、**連続の非基底列 (論理変数を含む) の係数ベクトル** c_i を、他の行の整数係数の組み合わせで短くする。
//!   基底の整数列の係数は組み合わせの係数そのもの (整数) なので、組み合わせた行からも GMI (δ = 1/k の CMIR) が作れる。
//!   手順 ([`reduce`]): (1) Andersen らと同じく、行 i ごとに min ||c_i + Σ_{k≠i} λ_k c_k|| の実数解 λ を求めて丸め、短くなれば
//!   採る。実数解は Gram 行列 G = (c_i·c_k) の逆行列 H から λ_k = H_ki / H_ii (正則化 1e-9 x 対角の最大)。
//!   (2) 続けて対ごとの縮約 (Gauss の縮約: c_i -= round(c_i·c_k / c_k·c_k) c_k、短くなる間。行 i 自身の係数が 1 のままに
//!   なるよう、組み合わせが行 i を含む行 k は使わない) を最大 `ENOMOTO_T_MIP_REDSPLIT_PASSES`
//!   (既定 3) 回。組み合わせの係数は i64 で持ち、溢れたら (checked_*) その行は諦める。係数の絶対値が
//!   `ENOMOTO_T_MIP_REDSPLIT_LMAX` (既定 1000) を超える組み合わせも使わない。
//! - 数値の検査 (既存の単体表の行の分離と同じもの + 組み合わせの打ち消し): 組み合わせた行の論理変数の係数 (= 基底逆行列の行の
//!   組み合わせ) の最大/最小の比が 1e6 を超える行、打ち消し (Σ |λ_k| max|行 k| / max|組み合わせた行|) が 1e6 を超える行は使わない。
//!   1e-11 以下の係数は捨てる (既存と同じ)。カットは既存と同じ `finish_cut` (小さい係数の吸収・係数の締め付け・動的な範囲) を通す。
//!
//! 組み合わせの正しさ: 単体表の行はどれも LP の等式 (右辺 0) の線形結合なので、どんな実数の組み合わせも妥当な等式で、CMIR はそれを
//! 一般の混合整数の 1 行として扱う (組み合わせの係数が整数であることはカットの強さにだけ効き、妥当さには効かない)。

/// 試作 `ENOMOTO_MIP_REDSPLIT` の動き方 (k-cuts, reduce-and-split)。既定では両方 false
pub(crate) fn redsplit_mode() -> (bool, bool) {
    static M: std::sync::OnceLock<(bool, bool)> = std::sync::OnceLock::new();
    *M.get_or_init(|| match env_str!("ENOMOTO_MIP_REDSPLIT") {
        None | Some("0") => (false, false),
        Some(s) => (s.contains('a'), s.contains('b')),
    })
}

/// 改良 (11-08): 値に `c` があれば、縮約の行のカットは元の行のカットより効き目が大きいときだけ使う
pub(crate) fn compare_mode() -> bool {
    static M: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *M.get_or_init(|| env_str!("ENOMOTO_MIP_REDSPLIT").is_some_and(|s| s.contains('c')))
}

/// CMIR に渡す δ の候補: k-cuts なら 1/k (k = 1..K)、そうでなければ既存と同じ 1, 1/2, 1/4, 1/8
pub(crate) fn deltas(kcuts: bool) -> Vec<f64> {
    if kcuts {
        let kmax = tunable!("ENOMOTO_T_MIP_REDSPLIT_K", 10usize, usize).max(1);
        (1..=kmax).map(|k| 1.0 / k as f64).collect()
    } else {
        vec![1.0, 0.5, 0.25, 0.125]
    }
}

thread_local! {
    /// 改良 (11-08) `ENOMOTO_T_MIP_REDSPLIT_TFRAC`: このスレッドの求解 (開始の時刻で見分ける) での縮約とカットの生成の時間の累計
    static RS_TIME: std::cell::Cell<(Option<std::time::Instant>, f64)> = const { std::cell::Cell::new((None, 0.0)) };
    /// 診断 (`ENOMOTO_MIP_REDSPLIT_LOG`): 縮約の行から作った候補の指紋 (カットの選択で採られた数を数える)
    static RS_FP: std::cell::RefCell<std::collections::HashSet<u64>> = std::cell::RefCell::new(std::collections::HashSet::new());
}

/// 求解 `start` での縮約の時間の累計 (秒)
pub(crate) fn time_used(start: std::time::Instant) -> f64 {
    RS_TIME.with(|c| {
        let (s, t) = c.get();
        if s == Some(start) { t } else { 0.0 }
    })
}

/// 求解 `start` での縮約の時間に `dt` 秒を足す
pub(crate) fn add_time(start: std::time::Instant, dt: f64) {
    RS_TIME.with(|c| {
        let (s, t) = c.get();
        c.set((Some(start), if s == Some(start) { t + dt } else { dt }));
    })
}

/// カットの指紋 (右辺・非零の数・係数)
pub(crate) fn fingerprint(coefs: &[(usize, f64)], rhs: f64) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    rhs.to_bits().hash(&mut h);
    coefs.len().hash(&mut h);
    for &(j, a) in coefs.iter().take(8) {
        j.hash(&mut h);
        a.to_bits().hash(&mut h);
    }
    h.finish()
}

/// 縮約の行から作った候補を覚える (診断)
pub(crate) fn remember(fp: u64) {
    RS_FP.with(|s| {
        s.borrow_mut().insert(fp);
    });
}

/// 覚えた候補のうち `chosen` にあるものの数を返し、覚えたものを消す (診断)
pub(crate) fn count_chosen(chosen: impl Iterator<Item = u64>) -> (usize, usize) {
    RS_FP.with(|s| {
        let mut s = s.borrow_mut();
        let total = s.len();
        let k = chosen.filter(|fp| s.contains(fp)).count();
        s.clear();
        (k, total)
    })
}

/// [`reduce`] の記録
#[derive(Default, Debug, Clone)]
pub(crate) struct RsStats {
    /// 連続の非基底の係数がある行の数 (縮約の対象)
    pub rows: usize,
    /// 短くなった行の数
    pub reduced: usize,
    /// 短くなった行のノルム (2 乗) の比の平均
    pub ratio_sum: f64,
    /// 溢れ・係数の上限・数値の検査で諦めた数
    pub rejected: usize,
}

/// 行 `bases` (単体表の行、列 0..n は構造変数、n.. は論理変数、右辺 0) の連続の列 (`is_int[j]` が false) の部分を縮約し、
/// 短くなった行の組み合わせ (元の行の番号, 連続の部分のノルムの 2 乗の比 (縮約の後 / 前), 組み合わせた行) を返す
pub(crate) fn reduce(bases: &[Vec<(usize, f64)>], is_int: &[bool], n: usize, deadline: Option<std::time::Instant>) -> (Vec<(usize, f64, Vec<(usize, f64)>)>, RsStats) {
    let mut st = RsStats::default();
    let rmax = tunable!("ENOMOTO_T_MIP_REDSPLIT_ROWS", 100usize, usize);
    let passes = tunable!("ENOMOTO_T_MIP_REDSPLIT_PASSES", 3usize, usize);
    let lmax = tunable!("ENOMOTO_T_MIP_REDSPLIT_LMAX", 1000i64, i64);
    let eps = 1e-3;
    // 連続の部分がある行だけを使う
    let rows: Vec<usize> = (0..bases.len()).filter(|&r| bases[r].iter().any(|&(j, a)| !is_int[j] && a != 0.0)).take(rmax).collect();
    let m = rows.len();
    st.rows = m;
    let mut out = Vec::new();
    if m < 2 {
        return (out, st);
    }
    let timed_out = || deadline.is_some_and(|d| std::time::Instant::now() >= d);
    // Gram 行列: 列ごとに (行, 値) を集めて足す
    let mut col: std::collections::HashMap<usize, Vec<(usize, f64)>> = std::collections::HashMap::new();
    for (r, &bi) in rows.iter().enumerate() {
        for &(j, a) in &bases[bi] {
            if !is_int[j] && a != 0.0 {
                col.entry(j).or_default().push((r, a));
            }
        }
    }
    let mut g = vec![vec![0.0f64; m]; m];
    for list in col.values() {
        for (p, &(r1, a1)) in list.iter().enumerate() {
            for &(r2, a2) in &list[p..] {
                g[r1][r2] += a1 * a2;
            }
        }
    }
    for r1 in 0..m {
        for r2 in 0..r1 {
            g[r1][r2] = g[r2][r1];
        }
    }
    if timed_out() {
        return (out, st);
    }
    let norm0: Vec<f64> = (0..m).map(|r| g[r][r]).collect();
    // 組み合わせの係数 (行 r = Σ_k l[r][k] 元の行 k)
    let mut l: Vec<Vec<i64>> = (0..m).map(|r| (0..m).map(|k| (k == r) as i64).collect()).collect();
    // (1) Andersen らの手順: H = (G + τI)^{-1}、λ_k = round(H_ki / H_ii)
    if let Some(h) = inverse_spd(&g) {
        for i in 0..m {
            if h[i][i] <= 0.0 {
                continue;
            }
            let lam: Vec<i64> = (0..m)
                .map(|k| {
                    if k == i {
                        1
                    } else {
                        let v = (h[k][i] / h[i][i]).round();
                        if v.abs() <= lmax as f64 { v as i64 } else { 0 }
                    }
                })
                .collect();
            if lam.iter().filter(|&&v| v != 0).count() <= 1 {
                continue;
            }
            let nn = quad(&g, &lam);
            if nn < (1.0 - eps) * norm0[i] {
                l[i] = lam;
            }
        }
    }
    // (2) 対ごとの縮約。M = L G を持ち、<v_i, v_k> = M_i · L_k
    let mut mm: Vec<Vec<f64>> = l.iter().map(|li| mat_vec(&g, li)).collect();
    let dotv = |mi: &[f64], lk: &[i64]| -> f64 { mi.iter().zip(lk).map(|(a, &b)| a * b as f64).sum() };
    let mut nrm: Vec<f64> = (0..m).map(|i| dotv(&mm[i], &l[i])).collect();
    'outer: for _ in 0..passes {
        let mut changed = false;
        for i in 0..m {
            if timed_out() {
                break 'outer;
            }
            for k in 0..m {
                // 行 i 自身の係数を 1 のままにする (Andersen らと同じく、基底の列 i の係数 1 の行の組み合わせにする):
                // 行 k の組み合わせが行 i を含めば使わない
                if k == i || l[k][i] != 0 || nrm[k] <= 1e-12 * norm0[k].max(1e-300) {
                    continue;
                }
                let d = dotv(&mm[i], &l[k]);
                let r = (d / nrm[k]).round();
                if r == 0.0 || r.abs() > lmax as f64 {
                    continue;
                }
                let new = nrm[i] - 2.0 * r * d + r * r * nrm[k];
                if new >= (1.0 - eps) * nrm[i] {
                    continue;
                }
                let ri = r as i64;
                let li: Option<Vec<i64>> = l[i].iter().zip(&l[k]).map(|(&a, &b)| b.checked_mul(ri).and_then(|t| a.checked_sub(t)).filter(|v| v.abs() <= lmax)).collect();
                let Some(li) = li else {
                    st.rejected += 1;
                    continue;
                };
                l[i] = li;
                let mk = mm[k].clone();
                for (a, b) in mm[i].iter_mut().zip(&mk) {
                    *a -= r * b;
                }
                nrm[i] = dotv(&mm[i], &l[i]);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // 組み合わせた行を作る (短くなったものだけ)
    let mut dense: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
    for i in 0..m {
        let is_id = l[i].iter().enumerate().all(|(k, &v)| v == (k == i) as i64);
        if is_id {
            continue;
        }
        dense.clear();
        let mut scale = 0.0f64;
        for (k, &lk) in l[i].iter().enumerate() {
            if lk == 0 {
                continue;
            }
            let b = &bases[rows[k]];
            let bmax = b.iter().fold(0.0f64, |acc, &(_, a)| acc.max(a.abs()));
            scale += (lk as f64).abs() * bmax;
            for &(j, a) in b {
                *dense.entry(j).or_insert(0.0) += lk as f64 * a;
            }
        }
        let mut row: Vec<(usize, f64)> = dense.iter().filter(|(_, v)| v.abs() > 1e-11).map(|(&j, &v)| (j, v)).collect();
        row.sort_unstable_by_key(|e| e.0);
        let cmax = row.iter().fold(0.0f64, |acc, &(_, a)| acc.max(a.abs()));
        // 論理変数の係数 (基底逆行列の行の組み合わせ) の比 (既存の単体表の行の検査と同じ 1e6)
        let (wmax, wmin) = row.iter().filter(|e| e.0 >= n).fold((0.0f64, f64::INFINITY), |(mx, mn), &(_, a)| (mx.max(a.abs()), mn.min(a.abs())));
        if cmax <= 0.0 || scale / cmax > 1e6 || (wmax > 0.0 && wmax / wmin > 1e6) {
            st.rejected += 1;
            continue;
        }
        // 実際のノルム (連続の部分) で確かめる
        let nn: f64 = row.iter().filter(|e| !is_int[e.0]).map(|e| e.1 * e.1).sum();
        if nn >= (1.0 - eps) * norm0[i] {
            continue;
        }
        st.reduced += 1;
        st.ratio_sum += nn / norm0[i];
        out.push((rows[i], nn / norm0[i], row));
    }
    (out, st)
}

fn quad(g: &[Vec<f64>], lam: &[i64]) -> f64 {
    let v = mat_vec(g, lam);
    v.iter().zip(lam).map(|(a, &b)| a * b as f64).sum()
}

fn mat_vec(g: &[Vec<f64>], lam: &[i64]) -> Vec<f64> {
    let m = g.len();
    let mut out = vec![0.0; m];
    for (k, &lk) in lam.iter().enumerate() {
        if lk != 0 {
            for (o, gv) in out.iter_mut().zip(&g[k]) {
                *o += lk as f64 * gv;
            }
        }
    }
    out
}

/// 対称半正定値の行列の (G + τI) の逆行列 (Cholesky)。τ = 1e-9 x 対角の最大。分解できなければ `None`
fn inverse_spd(g: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let m = g.len();
    let dmax = (0..m).map(|i| g[i][i]).fold(0.0f64, f64::max);
    if dmax <= 0.0 {
        return None;
    }
    let tau = 1e-9 * dmax;
    let mut c = vec![vec![0.0f64; m]; m];
    for i in 0..m {
        for j in 0..=i {
            let mut s = g[i][j] + if i == j { tau } else { 0.0 };
            for k in 0..j {
                s -= c[i][k] * c[j][k];
            }
            if i == j {
                if s <= 0.0 {
                    return None;
                }
                c[i][i] = s.sqrt();
            } else {
                c[i][j] = s / c[j][j];
            }
        }
    }
    // 逆行列: 列ごとに C C^T x = e
    let mut h = vec![vec![0.0f64; m]; m];
    for col in 0..m {
        let mut y = vec![0.0f64; m];
        for i in 0..m {
            let mut s = if i == col { 1.0 } else { 0.0 };
            for k in 0..i {
                s -= c[i][k] * y[k];
            }
            y[i] = s / c[i][i];
        }
        for i in (0..m).rev() {
            let mut s = y[i];
            for k in i + 1..m {
                s -= c[k][i] * h[k][col];
            }
            h[i][col] = s / c[i][i];
        }
    }
    if h.iter().flatten().any(|v| !v.is_finite()) {
        return None;
    }
    Some(h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduce_shortens_pair() {
        // 行 0 の連続の部分 (10, 1)、行 1 (10, 0) -> 行 0 - 行 1 = (0, 1) で短くなる。列 0, 1 は整数 (基底)、2, 3 は連続
        let is_int = vec![true, true, false, false];
        let bases = vec![vec![(0, 1.0), (2, 10.0), (3, 1.0)], vec![(1, 1.0), (2, 10.0)]];
        let (out, st) = reduce(&bases, &is_int, 4, None);
        assert_eq!(st.rows, 2);
        // 行 1 も行 0 を引いて (0, -1) に短くなる
        assert_eq!(out.len(), 2);
        let (_, ratio, row) = out.iter().find(|e| e.0 == 0).unwrap();
        assert!(*ratio < 0.01);
        let get = |j: usize| row.iter().find(|e| e.0 == j).map(|e| e.1).unwrap_or(0.0);
        assert_eq!(get(0), 1.0);
        assert_eq!(get(1), -1.0);
        assert_eq!(get(2), 0.0);
        assert_eq!(get(3), 1.0);
    }

    #[test]
    fn inverse_spd_ok() {
        let g = vec![vec![4.0, 2.0], vec![2.0, 3.0]];
        let h = inverse_spd(&g).unwrap();
        // G H = I (正則化 τ は 4e-9 なので誤差はそれ程度)
        for i in 0..2 {
            for j in 0..2 {
                let v: f64 = (0..2).map(|k| g[i][k] * h[k][j]).sum();
                assert!((v - (i == j) as i64 as f64).abs() < 1e-6);
            }
        }
    }
}
