//! 診断 `ENOMOTO_MIP_REFLSYM_COUNT` (analysis/reflsym_20261112.md、解く側は変えない): 列の反転 (x → l + u − x、0-1 なら補完) を
//! 含む符号つき置換の対称性を数え、置換だけの対称性と比べる。
//!
//! 作り方: 有限の境界の列 j は中心 c_j = (l+u)/2 に移した x'_j = x_j − c_j (範囲 [−h_j, h_j]) で考え、反転は x'_j → −x'_j。
//! 「文字」の列 v+_j (= x'_j) と v−_j (= −x'_j) を持つ写しの問題 A を作る (無限の境界の列は v+ だけで、元の境界のまま)。
//! - 列: v+_j は費用 c、v−_j は費用 −c、どちらも境界 [−h, h]、整数性は元と同じ。
//! - 行 i ごとに 2 本: r+ = Σ a_j v+_j − Σ a_j v−_j (範囲 [L', U'])、r− = その −1 倍 (範囲 [−U', −L'])。L'・U' は中心に移した分を引いたもの。
//! - 対の行: 反転できる列ごとに v+_j + v−_j (目印の範囲) で、文字の対を保つ置換だけが A の対称性になる。
//! A の列の置換 σ (対の行・r± の多重集合を保つ) は、元の問題の符号つき置換 (v+_j → v+_k なら x_j → x_k、v+_j → v−_k なら x_j → 反転の x_k)
//! を与える (費用・境界・整数性・行の集合を保つ)。今の置換の検出 ([`super::symmetry::detect`]) にそのまま掛けて数える。

use super::problem::MipProblem;
use super::symmetry;

/// 目印の範囲 (対の行)。ほかの行と同じ範囲にならない値
const PAIR_MARK: f64 = 7.7712345e7;

/// 写しの問題 A と、反転できる列の v− の番号 (できなければ `usize::MAX`)
fn literal_problem(p: &MipProblem) -> (MipProblem, Vec<usize>) {
    let n = p.n;
    let refl: Vec<bool> = (0..n).map(|j| p.col_lo[j].is_finite() && p.col_up[j].is_finite() && p.col_lo[j] < p.col_up[j]).collect();
    let mut neg = vec![usize::MAX; n];
    let mut k = n;
    for j in 0..n {
        if refl[j] {
            neg[j] = k;
            k += 1;
        }
    }
    let nn = k;
    let c: Vec<f64> = (0..n).map(|j| if refl[j] { 0.5 * (p.col_lo[j] + p.col_up[j]) } else { 0.0 }).collect();
    let mut lo = vec![0.0; nn];
    let mut up = vec![0.0; nn];
    let mut cost = vec![0.0; nn];
    let mut is_int = vec![false; nn];
    for j in 0..n {
        if refl[j] {
            let h = 0.5 * (p.col_up[j] - p.col_lo[j]);
            lo[j] = -h;
            up[j] = h;
            lo[neg[j]] = -h;
            up[neg[j]] = h;
            cost[neg[j]] = -p.cost[j];
            is_int[neg[j]] = p.is_int[j];
        } else {
            lo[j] = p.col_lo[j];
            up[j] = p.col_up[j];
        }
        cost[j] = p.cost[j];
        is_int[j] = p.is_int[j];
    }
    let mut rows = Vec::with_capacity(2 * p.m + n);
    let (mut rlo, mut rup) = (Vec::new(), Vec::new());
    for i in 0..p.m {
        let shift: f64 = p.rows[i].iter().map(|&(j, a)| a * c[j]).sum();
        let (l, u) = (p.row_lo[i] - shift, p.row_up[i] - shift);
        let mut r: Vec<(usize, f64)> = Vec::new();
        for &(j, a) in &p.rows[i] {
            r.push((j, a));
            if refl[j] {
                r.push((neg[j], -a));
            }
        }
        r.sort_by_key(|e| e.0);
        let rn: Vec<(usize, f64)> = r.iter().map(|&(j, a)| (j, -a)).collect();
        rows.push(r);
        rlo.push(l);
        rup.push(u);
        rows.push(rn);
        rlo.push(-u + 0.0);
        rup.push(-l + 0.0);
    }
    for j in 0..n {
        if refl[j] {
            rows.push(vec![(j, 1.0), (neg[j], 1.0)]);
            rlo.push(PAIR_MARK);
            rup.push(PAIR_MARK);
        }
    }
    (MipProblem::from_rows(lo, up, cost, 0.0, p.sense_sign, is_int, rows, rlo, rup), neg)
}

/// 数え: (生成元の数、2 列以上の軌道の数、動く列の数、反転を含む生成元の数、自分の反転に写る列の数)
fn summarize(n: usize, gens: &[Vec<usize>], neg: Option<&[usize]>) -> (usize, usize, usize, usize, usize) {
    // 文字の番号 → (変数, 反転か)
    let var_of = |k: usize| -> (usize, bool) {
        match neg {
            Some(ng) if k >= n => (ng.iter().position(|&v| v == k).unwrap_or(0), true),
            _ => (k, false),
        }
    };
    let mut inv_neg: Vec<usize> = Vec::new();
    if let Some(ng) = neg {
        let mx = ng.iter().copied().filter(|&v| v != usize::MAX).max().unwrap_or(n);
        inv_neg = vec![usize::MAX; mx + 1];
        for (j, &v) in ng.iter().enumerate() {
            if v != usize::MAX {
                inv_neg[v] = j;
            }
        }
    }
    let var_of_fast = |k: usize| -> (usize, bool) {
        if k >= n && !inv_neg.is_empty() {
            (inv_neg[k], true)
        } else {
            (k, false)
        }
    };
    let _ = var_of;
    let mut uf: Vec<usize> = (0..n).collect();
    let mut moved = vec![false; n];
    let mut self_refl = vec![false; n];
    let mut with_refl = 0;
    for g in gens {
        let mut has_refl = false;
        for j in 0..n {
            let (k, r) = var_of_fast(g[j]);
            if r {
                has_refl = true;
            }
            if k != j || r {
                moved[j] = true;
            }
            if k == j && r {
                self_refl[j] = true;
            }
            let (a, b) = (symmetry::find(&mut uf, j), symmetry::find(&mut uf, k));
            if a != b {
                uf[a] = b;
            }
        }
        if has_refl {
            with_refl += 1;
        }
    }
    let mut size = std::collections::HashMap::new();
    for j in 0..n {
        *size.entry(symmetry::find(&mut uf, j)).or_insert(0usize) += 1;
    }
    let orbits = size.values().filter(|&&s| s >= 2).count();
    (gens.len(), orbits, moved.iter().filter(|&&b| b).count(), with_refl, self_refl.iter().filter(|&&b| b).count())
}

/// 前処理の後の問題 `p` で、置換だけの対称性と符号つき置換の対称性を数えて 1 行出す。`tag` は場所 (`presolved`・`restart 1` など)。
pub fn debug_count(p: &MipProblem, tag: &str) {
    let t0 = std::time::Instant::now();
    let tcap: f64 = tunable!("ENOMOTO_T_MIP_REFLSYM_TIME", 20.0, f64);
    // 置換だけ: 今の既定と同じく行の向きをそろえ、同点の行を二重にした写しで探す
    let (q, _) = symmetry::detection_copy(p, false, true);
    let (s0, _) = symmetry::detect(&q, tcap, usize::MAX, u64::MAX);
    let g0: Vec<Vec<usize>> = s0.map(|s| s.generators.into_iter().map(|g| g[..p.n].to_vec()).collect()).unwrap_or_default();
    let a = summarize(p.n, &g0, None);
    let t1 = t0.elapsed().as_secs_f64();
    let (lp, neg) = literal_problem(p);
    let (s1, _) = symmetry::detect(&lp, tcap, usize::MAX, u64::MAX);
    let g1: Vec<Vec<usize>> = s1.map(|s| s.generators).unwrap_or_default();
    // 文字の対を保たない置換は数えない (対の行で防いでいるはずだが念のため)
    let g1: Vec<Vec<usize>> = g1
        .into_iter()
        .filter(|g| {
            (0..p.n).all(|j| {
                let nj = neg[j];
                if nj == usize::MAX {
                    return g[j] < p.n || neg.iter().all(|&v| v != g[j]);
                }
                let (pj, pn) = (g[j], g[nj]);
                // pj と pn が同じ変数の 2 つの文字
                let var = |k: usize| if k < p.n { (k, false) } else { (neg.iter().position(|&v| v == k).unwrap_or(usize::MAX), true) };
                let (a1, r1) = var(pj);
                let (a2, r2) = var(pn);
                a1 == a2 && r1 != r2
            })
        })
        .collect();
    let b = summarize(p.n, &g1, Some(&neg));
    let nb = (0..p.n).filter(|&j| p.is_int[j] && p.col_lo[j] == 0.0 && p.col_up[j] == 1.0).count();
    eprintln!(
        "REFLSYMCOUNT {tag}: n {} m {} bins {} reflectable {} | perm: gens {} orbits {} moved {} | signed: gens {} orbits {} moved {} gens_with_reflection {} self_reflected {} ({:.2}s, {:.2}s)",
        p.n,
        p.m,
        nb,
        neg.iter().filter(|&&v| v != usize::MAX).count(),
        a.0,
        a.1,
        a.2,
        b.0,
        b.1,
        b.2,
        b.3,
        b.4,
        t1,
        t0.elapsed().as_secs_f64() - t1
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// x0 + x1 = 1 (0-1)、費用 0: 置換 (x0 x1) と、反転 x0 → 1 − x0 と x1 → 1 − x1 を同時にする符号つき置換 (どちらも同じ写像になる)
    /// 反転だけでしか写らない例: x0 − x1 = 0 (0-1)、費用 (1, −1)。置換 (x0 x1) は費用を変えるので対称でないが、x0 → 1 − x1、x1 → 1 − x0 は保つ。
    #[test]
    fn reflection_found() {
        let p = MipProblem::from_rows(vec![0.0; 2], vec![1.0; 2], vec![1.0, -1.0], 0.0, 1.0, vec![true; 2], vec![vec![(0, 1.0), (1, -1.0)]], vec![0.0], vec![0.0]);
        let (q, _) = symmetry::detection_copy(&p, false, true);
        let (s0, _) = symmetry::detect(&q, 5.0, usize::MAX, u64::MAX);
        assert!(s0.is_none() || s0.unwrap().generators.is_empty());
        let (lp, neg) = literal_problem(&p);
        let (s1, _) = symmetry::detect(&lp, 5.0, usize::MAX, u64::MAX);
        let g = s1.expect("signed symmetry").generators;
        let (_, orbits, _, with_refl, _) = summarize(2, &g, Some(&neg));
        assert!(with_refl >= 1 && orbits == 1, "{g:?}");
    }
}
