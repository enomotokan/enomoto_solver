//! 課題 TUBR (analysis/tubr_20261112.md): TU の部品を逆向きに使い、「分枝しなくてよい整数列」I' を見つける。
//!
//! I' 以外の整数列 (と暗黙の整数列) を整数値に固定したとき、I' と連続列を合わせた部分 S の行列 A_S が TU で、S を含む行の
//! S 以外の (暗に) 整数の列の係数・有限の左辺と右辺、S の列の境界が整数 (行を正の数で割った後) なら、{x_S : lo ≤ A_S x_S ≤ up − A_B x_B,
//! 境界} の頂点はどれも整数。したがって I' 以外が整数のノードの LP は、I' 以外を固定した LP の頂点で I' も整数にできる。
//!
//! 判定 ([`detect`]):
//! 1. 候補: 固定されていない列のうち、整数列と、境界が整数か無限の連続列 (S には連続列も入る。境界が整数でない連続列は
//!    候補にならず、その列を含む行は使えない)。
//! 2. 行の確かめ (候補が変わったら確かめ直す): 行を候補の係数で割る。割る数は連続の候補の係数の絶対値 (違えば行は悪い)、なければ
//!    候補の係数の絶対値の最小。割った後に ±1 でない整数の候補は外して I' 以外 (固定する側) に回す。そのうえで、候補でない列の係数が
//!    整数列なら整数、固定された連続列なら寄与が整数、ほかの連続列を含まない、有限の左辺・右辺が整数、でなければ行の候補を全部外す。
//! 3. 候補の部分の行列の芯 (剥がしの後) の連結成分ごとに TU を確かめる (2 色塗り、だめならネットワーク行列かその転置
//!    [`super::tu::tu_cols_f64_net`])。だめな成分の候補は全部外す。2・3 を変わらなくなるまで繰り返す。
//! 4. 最後に、残った候補を含む全部の行で 2 の条件 (外さずに) と、候補の行列全体の TU を確かめ直す (安全網)。
//!
//! 数えるだけの診断 `ENOMOTO_MIP_TUBR_COUNT` (`=stop` で数えた後に終える): 前処理の後の問題で I' の本数などを 1 行に出す。

use super::problem::MipProblem;

fn integral(v: f64) -> bool {
    (v - v.round()).abs() <= 1e-9 * (1.0 + v.abs())
}

/// 判定の数 (診断用)
#[derive(Debug, Default, Clone)]
pub struct TubrStats {
    pub ints: usize,
    pub bins: usize,
    /// I' の整数列の本数と、そのうち 0-1
    pub iprime: usize,
    pub iprime_bin: usize,
    /// I' のうち剥がしで消えずに芯に残る (TU の判定に掛かる) 整数列の本数
    pub iprime_core: usize,
    /// S の連続列の本数
    pub cont_in_s: usize,
    /// 繰り返しの回数・TU の判定で外した成分の数
    pub rounds: usize,
    pub bad_comps: usize,
    /// 最後の確かめに落ちた (起きてはならない)
    pub final_fail: bool,
}

/// I' (分枝しなくてよい整数列) の印と数を返す。`base_int` は整数として扱う列 (整数列と暗黙の整数列)。
pub fn detect(p: &MipProblem, base_int: &[bool], use_net: bool) -> (Vec<bool>, TubrStats) {
    detect_opt(p, base_int, use_net, false)
}

/// [`detect`] の本体。`greedy` なら TU と分からない成分の候補を全部外さず、成分の中で行への出現が一番多い整数の候補だけを外して
/// 確かめ直す (連続の候補しかない成分は全部外す)。回数は 200 回まで (超えたら残りの悪い成分は全部外す)。
pub fn detect_opt(p: &MipProblem, base_int: &[bool], use_net: bool, greedy: bool) -> (Vec<bool>, TubrStats) {
    let n = p.n;
    let m = p.m;
    let mut st = TubrStats::default();
    st.ints = (0..n).filter(|&j| p.is_int[j]).count();
    st.bins = (0..n).filter(|&j| p.is_int[j] && p.col_lo[j] == 0.0 && p.col_up[j] == 1.0).count();
    let fixed = |j: usize| p.col_lo[j] == p.col_up[j];
    let mut cand: Vec<bool> = (0..n)
        .map(|j| !fixed(j) && !p.cols[j].is_empty() && (base_int[j] || [p.col_lo[j], p.col_up[j]].iter().all(|&v| !v.is_finite() || integral(v))))
        .collect();
    // 行の確かめ。変えた候補の列を返す (行が悪くて全部外したか、±1 でない整数の候補を外した)
    let check_row = |i: usize, cand: &mut [bool], modify: bool| -> (bool, Vec<usize>) {
        let r = &p.rows[i];
        let cands: Vec<(usize, f64)> = r.iter().copied().filter(|&(j, _)| cand[j]).collect();
        if cands.is_empty() {
            return (true, Vec::new());
        }
        let conts: Vec<f64> = cands.iter().filter(|&&(j, _)| !base_int[j]).map(|&(_, a)| a.abs()).collect();
        let scale = if let Some(&c0) = conts.first() {
            if conts.iter().any(|&c| (c - c0).abs() > 1e-12 * c0) {
                None
            } else {
                Some(c0)
            }
        } else {
            Some(cands.iter().map(|&(_, a)| a.abs()).fold(f64::INFINITY, f64::min))
        };
        let mut changed = Vec::new();
        let mut ok = scale.is_some();
        if let Some(sc) = scale {
            let s = 1.0 / sc;
            if modify {
                for &(j, a) in &cands {
                    if base_int[j] && ((a * s).abs() - 1.0).abs() > 1e-9 {
                        cand[j] = false;
                        changed.push(j);
                    }
                }
            }
            for &(j, a) in r {
                let v = a * s;
                if cand[j] {
                    if (v.abs() - 1.0).abs() > 1e-9 {
                        ok = false;
                    }
                } else if base_int[j] {
                    if !integral(v) {
                        ok = false;
                    }
                } else if fixed(j) {
                    if !integral(p.col_lo[j] * v) {
                        ok = false;
                    }
                } else {
                    ok = false;
                }
            }
            if [p.row_lo[i], p.row_up[i]].iter().any(|&v| v.is_finite() && !integral(v * s)) {
                ok = false;
            }
        }
        if !ok && modify {
            for &(j, _) in &cands {
                if cand[j] {
                    cand[j] = false;
                    changed.push(j);
                }
            }
        }
        (ok, changed)
    };
    let mut queued = vec![true; m];
    let mut q: std::collections::VecDeque<usize> = (0..m).collect();
    loop {
        st.rounds += 1;
        while let Some(i) = q.pop_front() {
            queued[i] = false;
            let (_, changed) = check_row(i, &mut cand, true);
            for j in changed {
                for &(i2, _) in &p.cols[j] {
                    if !queued[i2] {
                        queued[i2] = true;
                        q.push_back(i2);
                    }
                }
            }
        }
        // 候補の部分の行列の連結成分ごとの TU
        let bad = bad_components(p, &cand, use_net);
        if bad.is_empty() {
            break;
        }
        st.bad_comps += bad.len();
        for comp in bad {
            let ints: Vec<usize> = comp.iter().copied().filter(|&j| base_int[j]).collect();
            let comp: Vec<usize> = if greedy && st.rounds < 200 && !ints.is_empty() {
                let dmax = ints.iter().map(|&j| p.cols[j].len()).max().unwrap();
                ints.into_iter().filter(|&j| p.cols[j].len() == dmax).collect()
            } else {
                comp
            };
            for j in comp {
                cand[j] = false;
                for &(i2, _) in &p.cols[j] {
                    if !queued[i2] {
                        queued[i2] = true;
                        q.push_back(i2);
                    }
                }
            }
        }
    }
    // 4. 安全網: 外さずに全部の行を確かめ、全体の TU も確かめる
    let mut c2 = cand.clone();
    let rows_ok = (0..m).all(|i| check_row(i, &mut c2, false).0);
    if !rows_ok || !bad_components(p, &cand, use_net).is_empty() {
        st.final_fail = true;
        return (vec![false; n], st);
    }
    let iprime: Vec<bool> = (0..n).map(|j| cand[j] && p.is_int[j]).collect();
    st.iprime = iprime.iter().filter(|&&b| b).count();
    st.iprime_bin = (0..n).filter(|&j| iprime[j] && p.col_lo[j] == 0.0 && p.col_up[j] == 1.0).count();
    st.cont_in_s = (0..n).filter(|&j| cand[j] && !p.is_int[j]).count();
    st.iprime_core = core_count(p, &cand, &p.is_int);
    (iprime, st)
}

/// 候補の部分の行列 (行の倍率は TU に関係しないので符号だけ) の連結成分のうち、TU と分からないものの列の集まり。
fn bad_components(p: &MipProblem, cand: &[bool], use_net: bool) -> Vec<Vec<usize>> {
    let n = p.n;
    // 列の連結成分 (同じ行の候補をつなぐ)
    let mut uf: Vec<usize> = (0..n).collect();
    fn find(uf: &mut [usize], mut x: usize) -> usize {
        while uf[x] != x {
            uf[x] = uf[uf[x]];
            x = uf[x];
        }
        x
    }
    let mut comp_rows: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for r in &p.rows {
        let mut first = None;
        for &(j, _) in r {
            if cand[j] {
                match first {
                    None => first = Some(j),
                    Some(j0) => {
                        let (a, b) = (find(&mut uf, j0), find(&mut uf, j));
                        if a != b {
                            uf[b] = a;
                        }
                    }
                }
            }
        }
    }
    for (i, r) in p.rows.iter().enumerate() {
        if let Some(&(j0, _)) = r.iter().find(|&&(j, _)| cand[j]) {
            let root = find(&mut uf, j0);
            comp_rows.entry(root).or_default().push(i);
        }
    }
    let mut bad = Vec::new();
    for (_root, rows) in comp_rows {
        let mut lid: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        let mut cols: Vec<Vec<(usize, f64)>> = Vec::new();
        for (k, &i) in rows.iter().enumerate() {
            for &(j, a) in &p.rows[i] {
                if cand[j] {
                    let c = *lid.entry(j).or_insert_with(|| {
                        cols.push(Vec::new());
                        cols.len() - 1
                    });
                    cols[c].push((k, if a > 0.0 { 1.0 } else { -1.0 }));
                }
            }
        }
        let ok = if use_net { super::tu::tu_cols_f64_net(rows.len(), &cols, 2_000_000) } else { super::tu::tu_cols_f64(rows.len(), &cols) };
        if !ok {
            bad.push(lid.keys().copied().collect());
        }
    }
    bad
}

/// 剥がしの後の芯に残る `mask` の列の本数
fn core_count(p: &MipProblem, cand: &[bool], mask: &[bool]) -> usize {
    let n = p.n;
    let m = p.m;
    let mut rdeg: Vec<usize> = (0..m).map(|i| p.rows[i].iter().filter(|&&(j, _)| cand[j]).count()).collect();
    let mut cdeg: Vec<usize> = (0..n).map(|j| if cand[j] { p.cols[j].len() } else { 0 }).collect();
    let mut ra: Vec<bool> = rdeg.iter().map(|&d| d > 0).collect();
    let mut ca: Vec<bool> = cand.to_vec();
    let mut stack: Vec<(bool, usize)> = (0..m).filter(|&i| ra[i] && rdeg[i] <= 1).map(|i| (true, i)).chain((0..n).filter(|&j| ca[j] && cdeg[j] <= 1).map(|j| (false, j))).collect();
    while let Some((is_row, k)) = stack.pop() {
        if is_row {
            if !ra[k] {
                continue;
            }
            ra[k] = false;
            for &(j, _) in &p.rows[k] {
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
            for &(i, _) in &p.cols[k] {
                if ra[i] {
                    rdeg[i] -= 1;
                    if rdeg[i] == 1 {
                        stack.push((true, i));
                    }
                }
            }
        }
    }
    (0..n).filter(|&j| ca[j] && mask[j]).count()
}

/// 診断 `ENOMOTO_MIP_TUBR_COUNT`: 前処理の後の問題で I' を数えて 1 行出す (2 色塗りだけと、ネットワークの判定も)。
pub fn debug_count(p: &MipProblem, stop: bool) {
    let t0 = std::time::Instant::now();
    let base: Vec<bool> = p.implied_integers();
    let (_, s0) = detect(p, &base, false);
    let t1 = t0.elapsed().as_secs_f64();
    let (_, s1) = detect(p, &base, true);
    let t2 = t0.elapsed().as_secs_f64();
    let (_, s2) = detect_opt(p, &base, true, true);
    eprintln!(
        "TUBRCOUNT greedy: iprime {} (bin {}, core {}) cont_in_s {} rounds {} final_fail {} ({:.3}s)",
        s2.iprime,
        s2.iprime_bin,
        s2.iprime_core,
        s2.cont_in_s,
        s2.rounds,
        s2.final_fail,
        t0.elapsed().as_secs_f64() - t2
    );
    eprintln!(
        "TUBRCOUNT n {} m {} ints {} bins {} | 2color: iprime {} (bin {}, core {}) cont_in_s {} | net: iprime {} (bin {}, core {}) cont_in_s {} rounds {} bad_comps {} final_fail {} {} ({:.3}s, {:.3}s)",
        p.n,
        p.m,
        s1.ints,
        s1.bins,
        s0.iprime,
        s0.iprime_bin,
        s0.iprime_core,
        s0.cont_in_s,
        s1.iprime,
        s1.iprime_bin,
        s1.iprime_core,
        s1.cont_in_s,
        s1.rounds,
        s1.bad_comps,
        s0.final_fail,
        s1.final_fail,
        t1,
        t2 - t1
    );
    if stop {
        std::process::exit(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 施設の開設 y (0-1) と流量 x (整数): Σ_j x_ij = d_i (需要)、x_ij ≤ u y_j の形 (容量の行は剥がれる)。x は I'、y は外れる。
    #[test]
    fn flow_with_open_binaries() {
        // 列: x00 x01 x10 x11 (整数 0..5)、y0 y1 (0-1)
        let lo = vec![0.0; 6];
        let up = vec![5.0, 5.0, 5.0, 5.0, 1.0, 1.0];
        let cost = vec![1.0, 2.0, 2.0, 1.0, 3.0, 3.0];
        let is_int = vec![true; 6];
        let rows = vec![
            vec![(0, 1.0), (1, 1.0)],
            vec![(2, 1.0), (3, 1.0)],
            vec![(0, 1.0), (2, 1.0), (4, -5.0)],
            vec![(1, 1.0), (3, 1.0), (5, -5.0)],
        ];
        let p = MipProblem::from_rows(lo, up, cost, 0.0, 1.0, is_int, rows, vec![3.0, 2.0, f64::NEG_INFINITY, f64::NEG_INFINITY], vec![3.0, 2.0, 0.0, 0.0]);
        let (ip, st) = detect(&p, &p.is_int, true);
        assert!(!st.final_fail);
        assert_eq!(ip, vec![true, true, true, true, false, false], "{st:?}");
    }

    /// 奇閉路 (x0+x1 ≤ 1, x1+x2 ≤ 1, x0+x2 ≤ 1、0-1) は TU でない: I' は空
    #[test]
    fn odd_cycle_is_not_free() {
        let p = MipProblem::from_rows(
            vec![0.0; 3],
            vec![1.0; 3],
            vec![-1.0; 3],
            0.0,
            1.0,
            vec![true; 3],
            vec![vec![(0, 1.0), (1, 1.0)], vec![(1, 1.0), (2, 1.0)], vec![(0, 1.0), (2, 1.0)]],
            vec![f64::NEG_INFINITY; 3],
            vec![1.0; 3],
        );
        let (ip, st) = detect(&p, &p.is_int, true);
        assert!(ip.iter().all(|&b| !b), "{st:?}");
    }

    /// 右辺が整数でない行の列は I' にならない
    #[test]
    fn fractional_rhs_blocks() {
        let p = MipProblem::from_rows(vec![0.0; 2], vec![3.0; 2], vec![1.0; 2], 0.0, 1.0, vec![true; 2], vec![vec![(0, 1.0), (1, 1.0)]], vec![1.5], vec![f64::INFINITY]);
        let (ip, _) = detect(&p, &p.is_int, true);
        assert_eq!(ip, vec![false, false]);
    }
}
