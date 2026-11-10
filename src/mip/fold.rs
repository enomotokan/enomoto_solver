//! `ENOMOTO_MIP_FOLD` (2026-11-10 から既定で使う、既定の値 `riq`。`0` で止める。analysis/drcr_20261110.md): MIP を色の細分化で畳んで解き、解を元に戻す
//! (van der Hulst 2026「Folding Mixed-Integer Linear Programs and Reflection Symmetries」の第 4 節)。畳み方の本体は
//! [`crate::simplex::lp_fold`]。
//!
//! 値の文字: `r` 反転も使う、`i` 整数列もまとめる (段階 3)、`q` ログを出さない、`c` 畳んだ数を出すだけで解かない (数える用)、
//! `p` 行が 1 本もまとまらない (平行な列だけの) 畳みと、列が 1 本もまとまらない (平行な行だけの) 畳みも使う (既定では使わない。
//! 前処理が同じ列・同じ行をまとめるので)。`1` は連続列だけ・反転なし。
//!
//! - 連続列だけ (段階 2): 整数列はどれも 1 列だけの組にする (論文の Lemma 28)。和の等しい行は整数列の係数が同じになるので、
//!   畳んだ MIP と元の MIP は解が行き来でき (x_w = δ_w + λ_w z_Q、整数列は δ が整数)、最適値も同じ。
//! - 整数列も (段階 3、論文の Algorithm 1 を簡単にしたもの): 整数の組 Q は和の変数 y_Q = Σ λ_w (x_w − δ_w) (整数) にまとめる。
//!   畳んだ MIP は元の緩和 (元の整数解は畳んだ問題の整数解に写り目的値が同じ、Theorem 26 (i)) なので下界はそのまま使える。
//!   元に戻すには、整数の 1 列の組の値と整数の組の和を固定した元の問題 (繊維) の LP を解き、頂点が整数なら元の整数解
//!   (Theorem 32)。繊維が整数の多面体になるよう、組ごとに「組の列の係数が行の中で λ に比例する行は打ち消す (アフィンな TU の
//!   分解 A = S + U T の U)、残り S と組の和の行 T を並べた行列が TU」を確かめた組だけまとめる。TU の判定はネットワーク行列の
//!   一般の判定ではなく、剥がし (非零 1 つ以下の行・列を除く) と Heller-Tompkins の 2 色塗り (列の非零が 2 つ、またはその転置) の
//!   十分条件だけ。連続列を含む行と、係数・左辺・右辺が整数でない行 (V_R) では組の列が全部打ち消されることを求める
//!   (連続列の部分の TU は使わない。論文より狭い)。
//! - 安全網: 戻した解は元の問題で実行可能か (整数性も) を全部の回で確かめ、段階 3 では目的値が畳んだ問題の値以下 (最適の証明)
//!   かも確かめる。外れたら畳まずに残りの時間で元の問題を解く。

use super::problem::MipProblem;
use super::solver::{MipResult, MipStatus};
use crate::simplex::lp_fold::{Fold, FoldIn, FoldOpts};

/// `ENOMOTO_MIP_FOLD` の値。2026-11-10 から既定で使う (既定の値 `riq` = 反転・整数の組・ログなし。`0` か `off` で止める。
/// analysis/drcr_20261110.md の既定化の節)。
pub fn flags() -> Option<&'static str> {
    env_def!("ENOMOTO_MIP_FOLD", "riq")
}

/// 畳み方と畳んだ問題。
pub struct MipFold {
    pub fold: Fold,
    pub q: MipProblem,
    /// 整数の組 (2 列以上) があるか (戻すのに繊維の LP が要る)。
    pub int_groups: bool,
    pub secs: f64,
}

fn integral(v: f64) -> bool {
    (v - v.round()).abs() <= 1e-9 * (1.0 + v.abs())
}

/// 行列 (列 → (行, 係数 ±1)) が十分条件で TU か: 剥がしと、芯の連結成分ごとに列の非零が全部 2 つで行の 2 色塗りができるか、
/// 行の非零が全部 2 つで列の 2 色塗りができるか。
pub fn simple_tu(nrows: usize, cols: &[Vec<(usize, f64)>]) -> bool {
    let nc = cols.len();
    let mut rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); nrows];
    for (j, c) in cols.iter().enumerate() {
        for &(i, a) in c {
            if a != 1.0 && a != -1.0 {
                return false;
            }
            rows[i].push((j, a));
        }
    }
    let mut rdeg: Vec<usize> = rows.iter().map(|r| r.len()).collect();
    let mut cdeg: Vec<usize> = cols.iter().map(|c| c.len()).collect();
    let mut ralive = vec![true; nrows];
    let mut calive = vec![true; nc];
    let mut stack: Vec<(bool, usize)> = Vec::new();
    for i in 0..nrows {
        if rdeg[i] <= 1 {
            stack.push((true, i));
        }
    }
    for j in 0..nc {
        if cdeg[j] <= 1 {
            stack.push((false, j));
        }
    }
    while let Some((is_row, k)) = stack.pop() {
        if is_row {
            if !ralive[k] {
                continue;
            }
            ralive[k] = false;
            for &(j, _) in &rows[k] {
                if calive[j] {
                    cdeg[j] -= 1;
                    if cdeg[j] <= 1 {
                        stack.push((false, j));
                    }
                }
            }
        } else {
            if !calive[k] {
                continue;
            }
            calive[k] = false;
            for &(i, _) in &cols[k] {
                if ralive[i] {
                    rdeg[i] -= 1;
                    if rdeg[i] <= 1 {
                        stack.push((true, i));
                    }
                }
            }
        }
    }
    // 芯の連結成分 (行と列の二部グラフ)
    let mut comp_r = vec![usize::MAX; nrows];
    let mut comp_c = vec![usize::MAX; nc];
    let mut ncomp = 0;
    for s in 0..nrows {
        if !ralive[s] || comp_r[s] != usize::MAX {
            continue;
        }
        let mut rs = vec![s];
        comp_r[s] = ncomp;
        let (mut crows, mut ccols) = (Vec::new(), Vec::new());
        while let Some(i) = rs.pop() {
            crows.push(i);
            for &(j, _) in &rows[i] {
                if calive[j] && comp_c[j] == usize::MAX {
                    comp_c[j] = ncomp;
                    ccols.push(j);
                    for &(i2, _) in &cols[j] {
                        if ralive[i2] && comp_r[i2] == usize::MAX {
                            comp_r[i2] = ncomp;
                            rs.push(i2);
                        }
                    }
                }
            }
        }
        ncomp += 1;
        // 列の非零が 2 つ: 行の 2 色塗り (同じ符号 → 別の色、逆の符号 → 同じ色)
        let col_ok = ccols.iter().all(|&j| cols[j].iter().filter(|&&(i, _)| ralive[i]).count() == 2);
        let row_ok = crows.iter().all(|&i| rows[i].iter().filter(|&&(j, _)| calive[j]).count() == 2);
        let color = |nodes: &[usize], edges: &dyn Fn(usize) -> Vec<(usize, usize, bool)>, n: usize| -> bool {
            // edges(e) = (a, b, same): same なら同じ色、そうでなければ別の色
            let mut col = vec![-1i8; n];
            let mut adj: std::collections::HashMap<usize, Vec<(usize, bool)>> = std::collections::HashMap::new();
            for &e in nodes {
                for (a, b, same) in edges(e) {
                    adj.entry(a).or_default().push((b, same));
                    adj.entry(b).or_default().push((a, same));
                }
            }
            for (&s, _) in adj.iter() {
                if col[s] >= 0 {
                    continue;
                }
                col[s] = 0;
                let mut st = vec![s];
                while let Some(a) = st.pop() {
                    for &(b, same) in &adj[&a] {
                        let want = if same { col[a] } else { 1 - col[a] };
                        if col[b] < 0 {
                            col[b] = want;
                            st.push(b);
                        } else if col[b] != want {
                            return false;
                        }
                    }
                }
            }
            true
        };
        let ok = (col_ok
            && color(
                &ccols,
                &|j| {
                    let e: Vec<(usize, f64)> = cols[j].iter().filter(|&&(i, _)| ralive[i]).copied().collect();
                    vec![(e[0].0, e[1].0, e[0].1 != e[1].1)]
                },
                nrows,
            ))
            || (row_ok
                && color(
                    &crows,
                    &|i| {
                        let e: Vec<(usize, f64)> = rows[i].iter().filter(|&&(j, _)| calive[j]).copied().collect();
                        vec![(e[0].0, e[1].0, e[0].1 != e[1].1)]
                    },
                    nc,
                ));
        if !ok {
            return false;
        }
    }
    true
}

fn fold_in<'a>(p: &'a MipProblem, int_key: Option<&'a [u64]>, single: Option<&'a [bool]>) -> FoldIn<'a> {
    FoldIn { col_lo: &p.col_lo, col_up: &p.col_up, cost: &p.cost, is_int: &p.is_int, rows: &p.rows, row_lo: &p.row_lo, row_up: &p.row_up, int_key, single }
}

/// 段階 3 の整数列の初めの鍵: V_R の行の係数 (符号を揃えて) と、ほかの行の ±1 でない係数の位置と値。
fn int_keys(p: &MipProblem, vr: &[bool]) -> Vec<u64> {
    use std::hash::{Hash, Hasher};
    (0..p.n)
        .map(|j| {
            if !p.is_int[j] {
                return 0;
            }
            let mut ent: Vec<(usize, i64, bool)> = Vec::new();
            for &(i, a) in &p.cols[j] {
                if vr[i] {
                    ent.push((i, (a * 1e9).round() as i64, true));
                } else if a.abs() != 1.0 {
                    ent.push((i, (a * 1e9).round() as i64, false));
                }
            }
            ent.sort_unstable();
            if ent.first().is_some_and(|e| e.1 < 0) {
                for e in ent.iter_mut() {
                    e.1 = -e.1;
                }
            }
            let mut h = std::collections::hash_map::DefaultHasher::new();
            ent.hash(&mut h);
            h.finish()
        })
        .collect()
}

/// 畳む。畳まないなら `None`。
pub fn try_fold(p: &MipProblem, verbose: bool) -> Option<MipFold> {
    let fl = flags()?;
    let log = verbose || !fl.contains('q');
    let t0 = std::time::Instant::now();
    let opts = crate::simplex::lp_fold::opts_from_env(fl.contains('r'));
    let ints = fl.contains('i');
    let f = if !ints {
        Fold::build(&fold_in(p, None, None), opts)
    } else {
        // V_R: 連続列を含む行、係数か有限の左辺・右辺が整数でない行
        let vr: Vec<bool> = (0..p.m)
            .map(|i| {
                p.rows[i].iter().any(|&(j, a)| !p.is_int[j] || !integral(a))
                    || (p.row_lo[i].is_finite() && !integral(p.row_lo[i]))
                    || (p.row_up[i].is_finite() && !integral(p.row_up[i]))
            })
            .collect();
        let keys = int_keys(p, &vr);
        let f1 = Fold::build(&fold_in(p, Some(&keys), None), FoldOpts { max_ratio: 1.0, ..opts });
        match f1 {
            None => None,
            Some(f1) => {
                // 整数の組ごとに打ち消しと TU を確かめる
                let mut single = vec![false; p.n];
                let mut acc_cols: Vec<Vec<(usize, f64)>> = Vec::new(); // TU を確かめる行列 (列 → (行, 係数))
                let mut nrows_m = p.m; // 行 0..m は元の行、m.. は組の和の行
                let mut groups: Vec<(usize, Vec<u32>)> = f1.red_members.iter().enumerate().filter(|(q, mem)| f1.red.is_int[*q] && mem.len() > 1).map(|(q, m)| (q, m.clone())).collect();
                groups.sort_by_key(|g| std::cmp::Reverse(g.1.len()));
                let (mut ok_g, mut bad_g) = (0usize, 0usize);
                for (_q, mem) in &groups {
                    // 行ごとの組の列の係数 (λ をかけて)
                    let mut by_row: std::collections::HashMap<usize, Vec<f64>> = std::collections::HashMap::new();
                    for (k, &w) in mem.iter().enumerate() {
                        let lam = f1.col_sign[w as usize];
                        for &(i, a) in &p.cols[w as usize] {
                            let e = by_row.entry(i).or_insert_with(|| vec![0.0; mem.len()]);
                            e[k] = a * lam;
                        }
                    }
                    let mut cancel = false;
                    let mut good = true;
                    let mut s_cols: Vec<Vec<(usize, f64)>> = vec![Vec::new(); mem.len()];
                    let mut rows_sorted: Vec<_> = by_row.into_iter().collect();
                    rows_sorted.sort_by_key(|r| r.0);
                    for (i, vals) in rows_sorted {
                        let a0 = vals[0];
                        if a0 != 0.0 && vals.iter().all(|&v| (v - a0).abs() <= 1e-12 * a0.abs()) {
                            cancel = true;
                            continue;
                        }
                        if vr[i] {
                            good = false;
                            break;
                        }
                        for (k, &v) in vals.iter().enumerate() {
                            if v != 0.0 {
                                // S の係数は元の係数 (λ をかける前)
                                let w = mem[k] as usize;
                                s_cols[k].push((i, v * f1.col_sign[w]));
                            }
                        }
                    }
                    if good {
                        if cancel {
                            for (k, &w) in mem.iter().enumerate() {
                                s_cols[k].push((nrows_m, f1.col_sign[w as usize]));
                            }
                        }
                        let mut trial = acc_cols.clone();
                        trial.extend(s_cols.iter().cloned());
                        if simple_tu(nrows_m + usize::from(cancel), &trial) {
                            acc_cols = trial;
                            if cancel {
                                nrows_m += 1;
                            }
                            ok_g += 1;
                            continue;
                        }
                    }
                    bad_g += 1;
                    for &w in mem {
                        single[w as usize] = true;
                    }
                }
                if log {
                    eprintln!("MIP_FOLD: integer groups: {} accepted, {} rejected (V_R rows {})", ok_g, bad_g, vr.iter().filter(|&&b| b).count());
                }
                Fold::build(&fold_in(p, Some(&keys), Some(&single)), opts)
            }
        }
    };
    let Some(f) = f else {
        if log {
            eprintln!(
                "MIP_FOLD: no fold (n {} m {}) in {:.3}s (reason line {}, ratio {:.3})",
                p.n,
                p.m,
                t0.elapsed().as_secs_f64(),
                crate::simplex::lp_fold::LAST_FAIL.with(|c| c.get()),
                crate::simplex::lp_fold::LAST_RATIO.with(|c| c.get())
            );
        }
        return None;
    };
    // 行の組がどれも 1 行だけ (行が 1 本もまとまらない) なら、列の組は係数の同じ列 (平行な列) の集まりで、MIP の前処理が同じ列を
    // まとめる (`parallel columns merged`)。畳んでも前処理と同じ仕事を別の変数の形 (平均・和) でするだけで、neos-2987310-joes
    // (連続列 2943 本、既定 11 秒 → 53 秒) と nw04 (整数の組、12.9 → 15.6 秒) で遅くなったので畳まない
    if f.stats.row_parts == p.m && f.stats.bip_rows == 0 && !fl.contains('p') {
        if log {
            eprintln!("MIP_FOLD: no fold: only parallel columns (n {} -> {}, rows not merged); the MIP presolve merges them", p.n, f.stats.n_red);
        }
        return None;
    }
    // 列が 1 本もまとまらない (構造の列の数が同じで、双極の列もない) なら、行の組は係数の同じ行 (平行な行) の集まりで、MIP の
    // 前処理が同じ行を除く。roll3000 (行 2295 → 2091) と neos-860300 (行 850 → 595) が 58 問の種 1〜3 の全部の回で遅くなったので畳まない
    if f.stats.n_red == p.n && f.stats.bip_cols == 0 && !fl.contains('p') {
        if log {
            eprintln!("MIP_FOLD: no fold: only parallel rows (m {} -> {}, columns not merged); the MIP presolve removes them", p.m, f.stats.m_red);
        }
        return None;
    }
    let r = &f.red;
    let q = MipProblem::from_rows(r.col_lo.clone(), r.col_up.clone(), r.cost.clone(), p.offset + r.offset, p.sense_sign, r.is_int.clone(), r.rows.clone(), r.row_lo.clone(), r.row_up.clone());
    let int_groups = f.stats.int_groups > 0;
    let st = &f.stats;
    let secs = t0.elapsed().as_secs_f64();
    if log {
        eprintln!(
            "MIP_FOLD: n {} -> {}, m {} -> {} (ints {} -> {}, int groups {} with {} cols, bip cols {}, bip rows {}, neg cols {}, absorbed {}) in {:.3}s",
            st.n,
            st.n_red,
            st.m,
            st.m_red,
            p.is_int.iter().filter(|&&b| b).count(),
            q.is_int.iter().filter(|&&b| b).count(),
            st.int_groups,
            st.int_grouped_cols,
            st.bip_cols,
            st.bip_rows,
            st.neg_cols,
            st.absorbed,
            secs
        );
    }
    Some(MipFold { fold: f, q, int_groups, secs })
}

impl MipFold {
    /// 畳んだ問題の解 y を元の問題の解に戻す (整数の組があれば繊維の LP を解く)。戻せなければ `None`。
    pub fn lift(&self, p: &MipProblem, y: &[f64]) -> Option<Vec<f64>> {
        let f = &self.fold;
        let mut x = f.unfold(y);
        if self.int_groups {
            // 繊維: 1 列だけの整数列は値に固定、整数の組は和 Σ λ_w x_w = y_q + Σ λ_w δ_w を固定、連続列は自由
            let mut lo = p.col_lo.clone();
            let mut up = p.col_up.clone();
            for j in 0..p.n {
                let qq = f.col_red[j];
                let grouped = qq != u32::MAX && f.red_scale[qq as usize] > 1.0;
                if p.is_int[j] && !grouped {
                    let v = x[j].round();
                    lo[j] = v;
                    up[j] = v;
                }
            }
            let mut rows = p.rows.clone();
            let mut rlo = p.row_lo.clone();
            let mut rup = p.row_up.clone();
            for (qq, mem) in f.red_members.iter().enumerate() {
                if f.red_scale[qq] <= 1.0 {
                    continue;
                }
                let mut row: Vec<(usize, f64)> = mem.iter().map(|&w| (w as usize, f.col_sign[w as usize])).collect();
                row.sort_by_key(|e| e.0);
                let s = y[qq].round() + mem.iter().map(|&w| f.col_sign[w as usize] * f.col_off[w as usize]).sum::<f64>();
                rows.push(row);
                rlo.push(s);
                rup.push(s);
            }
            let mut lp = crate::simplex::mip_lp::TwoStageLp::new(&lo, &up, &p.cost, &rows, &rlo, &rup);
            let st = lp.solve(&super::lp::SolveLimits::default());
            if st != super::lp::LpStatus::Optimal {
                eprintln!("MIP_FOLD: the fiber LP ended with {st:?}");
                return None;
            }
            x = lp.col_values();
        }
        for j in 0..p.n {
            if p.is_int[j] {
                let v = x[j].round();
                if (x[j] - v).abs() > 1e-6 {
                    eprintln!("MIP_FOLD: lifted value of integer column {j} is not integral ({})", x[j]);
                    return None;
                }
                x[j] = v;
            }
        }
        p.is_feasible(&x, 1e-6).then_some(x)
    }
}

/// `ENOMOTO_MIP_FOLD`: 畳んで `solve` で解き、元に戻す。畳まないなら `None`。安全網に外れたら残りの時間で元を `solve` で解く。
pub fn solve_folded(p: &MipProblem, time_limit: f64, verbose: bool, solve: &dyn Fn(&MipProblem, f64) -> MipResult) -> Option<MipResult> {
    if flags().is_some_and(|f| f.contains('c')) {
        // 数えるだけ (畳んだ数を出して解かずに終える)
        let _ = try_fold(p, true);
        return Some(MipResult { status: MipStatus::NotSolved, x: None, objective: None, best_bound: f64::NEG_INFINITY, nodes: 0, lp_iterations: 0 });
    }
    let mf = try_fold(p, verbose)?;
    let t0 = std::time::Instant::now();
    let mut r = solve(&mf.q, (time_limit - mf.secs).max(0.0));
    let left = || (time_limit - mf.secs - t0.elapsed().as_secs_f64()).max(0.0);
    let fallback = |why: &str| {
        eprintln!("MIP_FOLD: safety net: {why}; solving the original ({:.2}s left)", left());
        solve(p, left())
    };
    match r.status {
        MipStatus::Infeasible => return Some(r),
        MipStatus::Unbounded | MipStatus::InfeasibleOrUnbounded if !mf.int_groups => return Some(r),
        MipStatus::Unbounded | MipStatus::InfeasibleOrUnbounded | MipStatus::NotSolved if r.x.is_none() => return Some(fallback("the folded problem gave no solution")),
        _ => {}
    }
    if let Some(y) = r.x.take() {
        let zr = r.objective.unwrap_or(mf.q.objective(&y));
        match mf.lift(p, &y) {
            Some(x) => {
                let z = p.objective(&x);
                if verbose || !flags().is_some_and(|f| f.contains('q')) {
                    eprintln!("MIP_FOLD: lifted solution objective {} (folded {}), status {:?}", p.sense_sign * z, p.sense_sign * zr, r.status);
                }
                if z > zr + 1e-6 * (1.0 + zr.abs()) {
                    // 整数の組で戻した解が畳んだ問題の値より悪い: 最適の証明にならない
                    if r.status == MipStatus::Optimal {
                        return Some(fallback("the lifted solution is worse than the folded optimum"));
                    }
                }
                r.objective = Some(z);
                r.x = Some(x);
            }
            None => return Some(fallback("the lifted solution is not feasible")),
        }
    }
    Some(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_tu_incidence_and_odd_cycle() {
        // 有向グラフの接続行列 (列 = 辺) は TU
        let cols = vec![vec![(0, 1.0), (1, -1.0)], vec![(1, 1.0), (2, -1.0)], vec![(0, -1.0), (2, 1.0)]];
        assert!(simple_tu(3, &cols));
        // 奇閉路の無向の接続行列 (同じ符号) は TU でない
        let cols = vec![vec![(0, 1.0), (1, 1.0)], vec![(1, 1.0), (2, 1.0)], vec![(0, 1.0), (2, 1.0)]];
        assert!(!simple_tu(3, &cols));
    }
}
