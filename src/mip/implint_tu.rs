//! 試作 `ENOMOTO_MIP_IMPLINT_TU2` (既定では使わない。analysis/implint_tu_20261110.md): 完全単模 (TU) の部分による暗黙の整数
//! (van der Hulst・Walter 2025、SCIP 10 の `presol_implint.c`) の判定。
//!
//! 整数列 (と暗黙の整数列) を整数値に固定したとき、残る連続列の集まり C の行列 A_C が TU で、C の行の整数列の係数・左辺・右辺と
//! C の列の境界が整数なら、{x_C : lo ≤ A_C x_C ≤ up, 境界} の空でない面はどれも整数の点を持つので、どの目的でも C が整数の最適解が
//! ある (「弱い」暗黙の整数: どの最適解でも整数とは限らない)。カット生成で整数として扱える。
//!
//! 判定の流れ ([`implied_integers_tu2`]):
//! 1. 候補: 整数でも暗黙の整数でもなく、境界が整数か無限で、固定されていない列。
//! 2. 行の確かめ (連鎖): 候補を含む行を、最初の候補の係数の絶対値で割って、候補の係数が ±1、(暗に) 整数の列の係数と有限の
//!    左辺・右辺が整数、整数値に固定された連続列の寄与が整数、ほかの連続列 (候補でない) を含まない、でなければ、その行の候補を
//!    全部外す。外した列の行を確かめ直す (単調なので結果は順序によらない)。
//! 3. 候補の列を行でつないだ連結成分 (全体の成分) を作る。
//! 4. 剥がし: 候補の部分の行列で、非零が 1 つ以下の行・列を繰り返し除く (単位ベクトルの行・列を足しても TU は保たれるので、
//!    剥がした残り (芯) が TU なら全体も TU)。流れの保存の行と容量の行 `x_e ≤ u y_e` の組では、容量の行が剥がれる。
//! 5. 芯の連結成分ごとに、列の非零が全部 2 つで Heller-Tompkins の 2 色塗り (同じ符号の 2 行は別の組、逆の符号なら同じ組) が
//!    できる (有向グラフの接続行列、ネットワーク行列の特別な形) か、行の非零が全部 2 つで列の 2 色塗りができる (その転置。
//!    差の制約 x_i − x_j ≤ c の形) なら TU。どちらでもなければ、その芯を含む全体の成分の候補を全部外す。
//!
//! 成分ごとに丸ごと採るか捨てるので、結果は列の番号の付け方によらない (問題の対称性で移り合う列は同じ扱いになる。SCIP の
//! 注意: 対称性の扱いは軌道の列がどれも整数であることを前提にすることがある)。既存の試作 [`MipProblem::implied_integers_tu`]
//! は、行への出現が 3 つ以上の候補 (容量の行も数える) を外し、2 色塗りの矛盾は BFS の順で外す (番号の付け方による)。

use super::problem::MipProblem;

/// 判定の数 (診断用)。
#[derive(Debug, Default, Clone)]
pub struct Tu2Stats {
    /// 初めの候補の数。
    pub cand0: usize,
    /// 行の確かめの後の候補の数。
    pub cand1: usize,
    /// 全体の成分の数。
    pub comps: usize,
    /// 芯の連結成分の数と、そのうち列の 2 色塗りで TU・行の 2 色塗りで TU・どちらでもない数。
    pub cores: usize,
    pub core_col: usize,
    pub core_row: usize,
    pub core_bad: usize,
    /// TU と判定できなかった芯のうち最大のもの (行の数・列の数・列の最大の非零・行の最大の非零)。
    pub worst_bad: (usize, usize, usize, usize),
    /// 最後に暗黙の整数とした列の数 (base に加えた分)。
    pub marked: usize,
}

/// 整数かを、数値の大きさに応じた許容誤差で確かめる (大きすぎる値は整数とみなさない)。
fn integral(v: f64) -> bool {
    v.abs() <= 1e9 && (v - v.round()).abs() <= (1e-9 * v.abs().max(1.0)).min(1e-6)
}

/// 係数の大きさの限度 (SCIP の `numericslimit` と同じ 1e8)。行の倍率をかけた後でこれを超える係数のある行は使わない。
const NUM_LIMIT: f64 = 1e8;

/// `base` (整数列と暗黙の整数列の印) に、TU の部分による暗黙の整数列を加えた印を返す (モジュールの説明を見よ)。
pub fn implied_integers_tu2(p: &MipProblem, base: &[bool]) -> (Vec<bool>, Tu2Stats) {
    let n = p.n;
    let m = p.m;
    let mut st = Tu2Stats::default();
    let mut cand: Vec<bool> = (0..n)
        .map(|j| {
            !base[j]
                && p.col_lo[j] < p.col_up[j]
                && !p.cols[j].is_empty()
                && [p.col_lo[j], p.col_up[j]].iter().all(|&v| !v.is_finite() || integral(v))
        })
        .collect();
    st.cand0 = cand.iter().filter(|&&c| c).count();
    if st.cand0 == 0 {
        return (base.to_vec(), st);
    }
    let fixed_cont = |j: usize| !base[j] && p.col_lo[j] == p.col_up[j];
    // 2. 行の確かめ (候補を外すと、その列の行を確かめ直す)
    let row_ok = |i: usize, cand: &[bool]| -> bool {
        let r = &p.rows[i];
        let Some(&(_, a0)) = r.iter().find(|&&(j, _)| cand[j]) else { return true };
        let s = 1.0 / a0.abs();
        r.iter().all(|&(j, a)| {
            let v = a * s;
            if v.abs() > NUM_LIMIT {
                false
            } else if cand[j] {
                (v.abs() - 1.0).abs() <= 1e-9
            } else if base[j] {
                integral(v)
            } else {
                fixed_cont(j) && integral(p.col_lo[j] * v)
            }
        }) && [p.row_lo[i], p.row_up[i]].iter().all(|&v| !v.is_finite() || integral(v * s))
    };
    let mut queued = vec![true; m];
    let mut q: std::collections::VecDeque<usize> = (0..m).collect();
    while let Some(i) = q.pop_front() {
        queued[i] = false;
        if row_ok(i, &cand) {
            continue;
        }
        for &(j, _) in &p.rows[i] {
            if cand[j] {
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
    st.cand1 = cand.iter().filter(|&&c| c).count();
    if st.cand1 == 0 {
        return (base.to_vec(), st);
    }
    // 行ごとの候補の符号 (倍率は正なので符号は元の係数の符号)。候補だけの行列 (行 → (列, 正か))
    let crow: Vec<Vec<(usize, bool)>> = (0..m).map(|i| p.rows[i].iter().filter(|&&(j, _)| cand[j]).map(|&(j, a)| (j, a > 0.0)).collect()).collect();
    let ccol: Vec<Vec<(usize, bool)>> = (0..n).map(|j| if cand[j] { p.cols[j].iter().map(|&(i, a)| (i, a > 0.0)).collect() } else { Vec::new() }).collect();
    // 3. 全体の成分 (列の番号の union-find、同じ行の候補をつなぐ)
    let mut uf: Vec<usize> = (0..n).collect();
    fn find(uf: &mut [usize], mut x: usize) -> usize {
        while uf[x] != x {
            uf[x] = uf[uf[x]];
            x = uf[x];
        }
        x
    }
    for r in &crow {
        if let Some(&(j0, _)) = r.first() {
            let a = find(&mut uf, j0);
            for &(j, _) in &r[1..] {
                let b = find(&mut uf, j);
                if a != b {
                    uf[b] = a;
                }
            }
        }
    }
    let mut comp_bad = vec![false; n]; // 根の列の番号で引く
    st.comps = (0..n).filter(|&j| cand[j] && find(&mut uf, j) == j).count();
    // 4. 剥がし
    let mut rdeg: Vec<usize> = crow.iter().map(|r| r.len()).collect();
    let mut cdeg: Vec<usize> = ccol.iter().map(|c| c.len()).collect();
    let mut ralive: Vec<bool> = rdeg.iter().map(|&d| d > 0).collect();
    let mut calive: Vec<bool> = cand.clone();
    // 積み (正: 行 i、負: 列 !j)
    let mut stack: Vec<(bool, usize)> = Vec::new();
    for i in 0..m {
        if ralive[i] && rdeg[i] <= 1 {
            stack.push((true, i));
        }
    }
    for j in 0..n {
        if calive[j] && cdeg[j] <= 1 {
            stack.push((false, j));
        }
    }
    while let Some((is_row, k)) = stack.pop() {
        if is_row {
            if !ralive[k] {
                continue;
            }
            ralive[k] = false;
            for &(j, _) in &crow[k] {
                if calive[j] {
                    cdeg[j] -= 1;
                    if cdeg[j] == 1 {
                        stack.push((false, j));
                    }
                }
            }
        } else {
            if !calive[k] {
                continue;
            }
            calive[k] = false;
            for &(i, _) in &ccol[k] {
                if ralive[i] {
                    rdeg[i] -= 1;
                    if rdeg[i] == 1 {
                        stack.push((true, i));
                    }
                }
            }
        }
    }
    // 5. 芯の連結成分 (行と列の BFS) ごとに TU を確かめる
    let mut rseen = vec![false; m];
    let mut cseen = vec![false; n];
    let mut rcol: Vec<i8> = vec![-1; m];
    let mut ccolr: Vec<i8> = vec![-1; n];
    for r0 in 0..m {
        if !ralive[r0] || rseen[r0] {
            continue;
        }
        // 芯の成分を集める
        let (mut rs, mut cs) = (vec![r0], Vec::new());
        rseen[r0] = true;
        let mut h = 0;
        while h < rs.len() {
            let r = rs[h];
            h += 1;
            for &(j, _) in &crow[r] {
                if calive[j] && !cseen[j] {
                    cseen[j] = true;
                    cs.push(j);
                    for &(i, _) in &ccol[j] {
                        if ralive[i] && !rseen[i] {
                            rseen[i] = true;
                            rs.push(i);
                        }
                    }
                }
            }
        }
        st.cores += 1;
        let live_c = |j: usize| -> Vec<(usize, bool)> { ccol[j].iter().copied().filter(|&(i, _)| ralive[i]).collect() };
        let live_r = |i: usize| -> Vec<(usize, bool)> { crow[i].iter().copied().filter(|&(j, _)| calive[j]).collect() };
        let maxc = cs.iter().map(|&j| live_c(j).len()).max().unwrap_or(0);
        let maxr = rs.iter().map(|&i| live_r(i).len()).max().unwrap_or(0);
        // 列の 2 色塗り (行が頂点、列が辺)
        let col_ok = maxc == 2 && {
            let mut ok = true;
            'bfs: for &s in &rs {
                if rcol[s] >= 0 {
                    continue;
                }
                rcol[s] = 0;
                let mut qq = vec![s];
                while let Some(r) = qq.pop() {
                    for &(j, sr) in &live_r(r) {
                        let lc = live_c(j);
                        let &(r2, s2) = lc.iter().find(|&&(i, _)| i != r).unwrap();
                        let want = rcol[r] ^ ((sr == s2) as i8);
                        if rcol[r2] < 0 {
                            rcol[r2] = want;
                            qq.push(r2);
                        } else if rcol[r2] != want {
                            ok = false;
                            break 'bfs;
                        }
                    }
                }
            }
            ok
        };
        // 行の 2 色塗り (列が頂点、行が辺)
        let row_ok2 = !col_ok && maxr == 2 && {
            let mut ok = true;
            'bfs2: for &s in &cs {
                if ccolr[s] >= 0 {
                    continue;
                }
                ccolr[s] = 0;
                let mut qq = vec![s];
                while let Some(j) = qq.pop() {
                    for &(i, sj) in &live_c(j) {
                        let lr = live_r(i);
                        let &(j2, s2) = lr.iter().find(|&&(k, _)| k != j).unwrap();
                        let want = ccolr[j] ^ ((sj == s2) as i8);
                        if ccolr[j2] < 0 {
                            ccolr[j2] = want;
                            qq.push(j2);
                        } else if ccolr[j2] != want {
                            ok = false;
                            break 'bfs2;
                        }
                    }
                }
            }
            ok
        };
        if col_ok {
            st.core_col += 1;
        } else if row_ok2 {
            st.core_row += 1;
        } else {
            st.core_bad += 1;
            if (rs.len(), cs.len()) > (st.worst_bad.0, st.worst_bad.1) {
                st.worst_bad = (rs.len(), cs.len(), maxc, maxr);
            }
            let root = find(&mut uf, cs[0]);
            comp_bad[root] = true;
        }
    }
    let out: Vec<bool> = (0..n).map(|j| base[j] || (cand[j] && !comp_bad[find(&mut uf, j)])).collect();
    st.marked = (0..n).filter(|&j| out[j] && !base[j]).count();
    (out, st)
}

/// `ENOMOTO_MIP_IMPLINT_TU2` を付けているか (`0`・`off` は付けていないのと同じ)。
pub fn tu2_on() -> bool {
    env_str!("ENOMOTO_MIP_IMPLINT_TU2").is_some_and(|v| v != "0" && v != "off")
}

/// カット生成で整数として扱う列の印 ([`MipProblem::implied_integers`] に TU2 の判定を加えたもの)。
pub fn cut_mask(p: &MipProblem, verbose: bool) -> Vec<bool> {
    let t0 = std::time::Instant::now();
    let b = p.implied_integers();
    let (t, st) = implied_integers_tu2(p, &b);
    if verbose {
        eprintln!(
            "MIP: implied integers: {} by the row rules, +{} with the TU2 rule ({:?}, {:.3}s)",
            b.iter().zip(&p.is_int).filter(|(a, i)| **a && !**i).count(),
            st.marked,
            st,
            t0.elapsed().as_secs_f64()
        );
    }
    t
}

/// 診断 `ENOMOTO_MIP_DEBUG_IMPLINTCOUNT`: 前処理の後の問題で、行の規則・既存の試作 TU・TU2 の暗黙の整数の数を 1 行で出す。
pub fn debug_count(p: &MipProblem, stop: bool) {
    let t0 = std::time::Instant::now();
    let ncont = p.is_int.iter().filter(|&&b| !b).count();
    let b = p.implied_integers();
    let nb = b.iter().zip(&p.is_int).filter(|(a, i)| **a && !**i).count();
    let t1 = p.implied_integers_tu(&b);
    let nt1 = t1.iter().zip(&b).filter(|(a, i)| **a && !**i).count();
    let (t2, st) = implied_integers_tu2(p, &b);
    // TU2 が既存の TU の印を全部含むか (含まない列の数)
    let miss = t1.iter().zip(&t2).filter(|(a, c)| **a && !**c).count();
    let costed = (0..p.n).filter(|&j| p.cost[j] != 0.0 && !p.is_int[j]).count();
    let costed_t2 = (0..p.n).filter(|&j| p.cost[j] != 0.0 && !p.is_int[j] && !t2[j]).count();
    eprintln!(
        "IMPLINTCOUNT n {} m {} cont {} rowrule {} tu {} tu2 {} tu_not_in_tu2 {} costed_cont {} costed_cont_left_tu2 {} step0 {:?} step_tu2 {:?} stats {:?} ({:.3}s)",
        p.n,
        p.m,
        ncont,
        nb,
        nt1,
        st.marked,
        miss,
        costed,
        costed_t2,
        p.objective_step(),
        p.objective_step_mask(&t2, None),
        st,
        t0.elapsed().as_secs_f64()
    );
    if stop {
        std::process::exit(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INF: f64 = f64::INFINITY;

    fn prob(n: usize, is_int: Vec<bool>, up: Vec<f64>, rows: Vec<Vec<(usize, f64)>>, lo: Vec<f64>, upr: Vec<f64>) -> MipProblem {
        MipProblem::from_rows(vec![0.0; n], up, vec![0.0; n], 0.0, 1.0, is_int, rows, lo, upr)
    }

    #[test]
    fn tu2_includes_penalty_network() {
        // problem.rs の implied_integers_tu_penalty_network と同じ問題: s0, s1, z は暗に整数、w は外れる
        let p = MipProblem::from_rows(
            vec![0.0; 6],
            vec![1.0, 1.0, INF, INF, INF, INF],
            vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            0.0,
            1.0,
            vec![true, true, false, false, false, false],
            vec![vec![(0, 100.0), (2, -1.0)], vec![(1, 300.0), (3, -1.0)], vec![(2, 1.0), (3, 1.0), (4, -1.0)], vec![(0, 1.0), (5, 1.0)]],
            vec![-INF, -INF, -INF, -INF],
            vec![200.0, 100.0, 0.0, 2.5],
        );
        let b = p.implied_integers();
        assert_eq!(implied_integers_tu2(&p, &b).0, vec![true, true, true, true, true, false]);
    }

    #[test]
    fn tu2_fixed_charge_network() {
        // 節点 3 つの流れ: x01 (列 0)、x12 (列 1)、x02 (列 2) は連続、y0..y2 (列 3..5) は 2 値。
        // 保存: -x01 - x02 = -3、x01 - x12 = 0、x12 + x02 = 3。容量: x_e - 5 y_e <= 0 (列の出現は 3 行)。
        // 既存の試作 (出現 2 行まで) は外すが、容量の行は剥がれるので TU2 では 3 本とも暗に整数
        let rows = vec![
            vec![(0, -1.0), (2, -1.0)],
            vec![(0, 1.0), (1, -1.0)],
            vec![(1, 1.0), (2, 1.0)],
            vec![(0, 1.0), (3, -5.0)],
            vec![(1, 1.0), (4, -5.0)],
            vec![(2, 1.0), (5, -5.0)],
        ];
        let p = prob(6, vec![false, false, false, true, true, true], vec![INF, INF, INF, 1.0, 1.0, 1.0], rows, vec![-3.0, 0.0, 3.0, -INF, -INF, -INF], vec![-3.0, 0.0, 3.0, 0.0, 0.0, 0.0]);
        let b = p.implied_integers();
        assert_eq!(p.implied_integers_tu(&b)[..3], [false, false, false]);
        assert_eq!(implied_integers_tu2(&p, &b).0, vec![true; 6]);
    }

    #[test]
    fn tu2_rejects_odd_cycle_and_fractional_rhs() {
        // x0 + x1 = 1, x1 + x2 = 1, x0 + x2 = 1 (LP の解は 0.5): TU でない
        let p = prob(3, vec![false; 3], vec![INF; 3], vec![vec![(0, 1.0), (1, 1.0)], vec![(1, 1.0), (2, 1.0)], vec![(0, 1.0), (2, 1.0)]], vec![1.0; 3], vec![1.0; 3]);
        let b = p.implied_integers();
        assert_eq!(implied_integers_tu2(&p, &b).0, vec![false; 3]);
        // 2 x0 - 2 x1 <= 1 (割ると右辺 0.5)
        let p = prob(2, vec![false; 2], vec![INF; 2], vec![vec![(0, 2.0), (1, -2.0)]], vec![-INF], vec![1.0]);
        assert_eq!(implied_integers_tu2(&p, &p.implied_integers()).0, vec![false; 2]);
        // 候補でない連続列 (境界が 0.5) が同じ行にあると、つながった成分ごと外れる
        let p = MipProblem::from_rows(
            vec![0.0; 3],
            vec![INF, INF, 0.5],
            vec![0.0; 3],
            0.0,
            1.0,
            vec![false; 3],
            vec![vec![(0, 1.0), (1, -1.0)], vec![(1, 1.0), (2, 1.0)]],
            vec![-INF, -INF],
            vec![0.0, 3.0],
        );
        assert_eq!(implied_integers_tu2(&p, &p.implied_integers()).0, vec![false; 3]);
    }

    #[test]
    fn tu2_difference_constraints_and_multicommodity() {
        // 開始時刻 S0..S2 (連続) と順序の 2 値 y (列 3): S1 - S0 >= 3、S2 - S1 >= 2、S0 - S2 + 10 y >= 1、C - S2 >= 4 (C は列 4)。
        // 行ごとの連続列は 2 つ (差の形) なので転置の 2 色塗りで TU
        let rows = vec![vec![(0, -1.0), (1, 1.0)], vec![(1, -1.0), (2, 1.0)], vec![(0, 1.0), (2, -1.0), (3, 10.0)], vec![(2, -1.0), (4, 1.0)], vec![(0, 1.0), (1, 1.0), (4, -1.0)]];
        let p = prob(5, vec![false, false, false, true, false], vec![INF, INF, INF, 1.0, INF], rows[..4].to_vec(), vec![3.0, 2.0, 1.0, 4.0], vec![INF; 4]);
        assert_eq!(implied_integers_tu2(&p, &p.implied_integers()).0, vec![true; 5]);
        // 行 4 (S0 + S1 - C <= 0、連続列 3 つ) を足すと、行の非零が 3 の行と、列の非零が 3 の列が芯に残るので外れる
        let p = prob(5, vec![false, false, false, true, false], vec![INF, INF, INF, 1.0, INF], rows, vec![3.0, 2.0, 1.0, 4.0, -INF], vec![INF, INF, INF, INF, 0.0]);
        let t = implied_integers_tu2(&p, &p.implied_integers()).0;
        assert_eq!(t[..3], [false, false, false]);
        // 2 品種の流れ (枝 2 本ずつ) と共同の容量の行: x^k_e は保存の行 2 本と容量の行に出て、芯の列の非零が 3 (列の 2 色塗りはできない)
        // 品種 k の保存: 節点 a: -x^k_1 - x^k_2 = -1、節点 b: x^k_1 + x^k_2 = 1。容量: x^0_e + x^1_e - y_e <= 0
        let rows = vec![
            vec![(0, -1.0), (1, -1.0)],
            vec![(0, 1.0), (1, 1.0)],
            vec![(2, -1.0), (3, -1.0)],
            vec![(2, 1.0), (3, 1.0)],
            vec![(0, 1.0), (2, 1.0), (4, -1.0)],
            vec![(1, 1.0), (3, 1.0), (5, -1.0)],
        ];
        let p = prob(6, vec![false, false, false, false, true, true], vec![INF, INF, INF, INF, 1.0, 1.0], rows, vec![-1.0, 1.0, -1.0, 1.0, -INF, -INF], vec![-1.0, 1.0, -1.0, 1.0, 0.0, 0.0]);
        let (t, st) = implied_integers_tu2(&p, &p.implied_integers());
        // この小さな例は、どの行も連続列が 2 つなので転置の 2 色塗り (x^0_1・x^1_2 と x^0_2・x^1_1 の組) で TU と分かる
        assert_eq!(t, vec![true; 6], "{st:?}");
        assert_eq!(st.core_row, 1);
    }

    /// 0/±1 の小さな行列が TU かを全部の正方の部分行列の行列式で確かめる。
    fn is_tu(a: &[Vec<i64>]) -> bool {
        let (m, n) = (a.len(), a.first().map_or(0, |r| r.len()));
        fn det(mt: &mut Vec<Vec<i64>>) -> i64 {
            let k = mt.len();
            if k == 1 {
                return mt[0][0];
            }
            let mut d = 0;
            for c in 0..k {
                if mt[0][c] == 0 {
                    continue;
                }
                let mut sub: Vec<Vec<i64>> = mt[1..].iter().map(|r| r.iter().enumerate().filter(|&(cc, _)| cc != c).map(|(_, &v)| v).collect()).collect();
                let s = if c % 2 == 0 { 1 } else { -1 };
                d += s * mt[0][c] * det(&mut sub);
            }
            d
        }
        for rm in 1u32..(1 << m) {
            let rs: Vec<usize> = (0..m).filter(|&i| rm >> i & 1 == 1).collect();
            for cm in 1u32..(1 << n) {
                let cs: Vec<usize> = (0..n).filter(|&j| cm >> j & 1 == 1).collect();
                if cs.len() != rs.len() {
                    continue;
                }
                let mut mt: Vec<Vec<i64>> = rs.iter().map(|&i| cs.iter().map(|&j| a[i][j]).collect()).collect();
                if det(&mut mt).abs() > 1 {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn tu2_random_marked_parts_are_tu() {
        // でたらめな小さな問題で、TU2 が印を付けた連続列の部分 (行を倍率で割ったもの) が本当に TU かを確かめる
        let mut seed: u64 = 12345;
        let mut rnd = |k: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % k
        };
        let mut nmarked = 0;
        for _ in 0..3000 {
            let n = 3 + rnd(5) as usize;
            let m = 2 + rnd(5) as usize;
            let is_int: Vec<bool> = (0..n).map(|_| rnd(3) == 0).collect();
            let mut rows = Vec::new();
            for _ in 0..m {
                let mut r = Vec::new();
                for j in 0..n {
                    if rnd(5) < 2 {
                        let a = [1.0, -1.0, 2.0, -2.0, 1.0, -1.0][rnd(6) as usize];
                        r.push((j, a));
                    }
                }
                if r.is_empty() {
                    r.push((rnd(n as u64) as usize, 1.0));
                }
                rows.push(r);
            }
            let up: Vec<f64> = (0..n).map(|j| if is_int[j] { 3.0 } else { INF }).collect();
            let p = prob(n, is_int.clone(), up, rows.clone(), vec![-INF; m], vec![4.0; m]);
            let base = p.implied_integers();
            let (t, _) = implied_integers_tu2(&p, &base);
            let c: Vec<usize> = (0..n).filter(|&j| t[j] && !base[j]).collect();
            if c.is_empty() {
                continue;
            }
            nmarked += 1;
            let mut a: Vec<Vec<i64>> = Vec::new();
            for r in &rows {
                let Some(&(_, a0)) = r.iter().find(|&&(j, _)| c.contains(&j)) else { continue };
                let row: Vec<i64> = c.iter().map(|&j| r.iter().find(|&&(k, _)| k == j).map_or(0, |&(_, v)| (v / a0.abs()).round() as i64)).collect();
                a.push(row);
            }
            assert!(a.len() <= 7 && c.len() <= 7 || true);
            if a.len() <= 7 && c.len() <= 7 {
                assert!(is_tu(&a), "not TU: {a:?} rows {rows:?} int {is_int:?}");
            }
        }
        assert!(nmarked > 100, "{nmarked}");
    }
}
