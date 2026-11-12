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
    /// 行の確かめで候補を外した行の数を、理由ごとに (候補でない連続列・候補の係数が ±1 にならない・整数列の係数が整数でない・
    /// 左辺か右辺が整数でない・固定された連続列の寄与・係数が大きすぎる)。行ごとに最初に見つけた理由だけ数える。
    pub why: [usize; 6],
    /// 端数の境界が冗長と分かって候補に入れた列の数。
    pub red: usize,
    /// `e` で TU と分かった全体の成分の数と、分からなかった数。
    pub er_ok: usize,
    pub er_bad: usize,
}

fn tu_flag_e() -> bool {
    tu2_flag('e')
}

// TU の判定 (`tu_shape`・`is_tu_er`) は DRCR (src/mip/fold.rs) と共通の部品 [`super::tu`] に移した (2026-11-11、中身は同じ)
pub use super::tu::{is_tu_er, tu_shape};

/// 値の文字で部品を選ぶ (`1` は `b` と同じ): `b` 行から導ける端数の境界を無視する ([`redundant_frac_bounds`])、
/// `v` 変数上下限の行に出る列はカットの整数の印から外す ([`cut_mask`])、`a` TU2 で足した列を SCIP と同じく、集約・変数上下限の
/// 置き換え・flow cover では連続、c-MIR で単純な境界で置き換えるときだけ整数として扱う (`CutVars::imp`、solver.rs の `cut_imp`)、
/// `F` (診断、誤答になりうる) TU と判定できない芯も TU とみなす、`e` 判定できない成分を重複の除去と行の分割で同値な形に直して判定し直す ([`is_tu_er`])、`s` 目的値の刻みにも使う (solver.rs、`ENOMOTO_MIP_OBJ_STEP_IMPLINT`
/// を付けなくても TU2 の印で刻みを取り、刈り込みは格子の点で取る)、`n` カットの印には使わない (刻みだけ)。
pub fn tu2_flag(c: char) -> bool {
    env_str!("ENOMOTO_MIP_IMPLINT_TU2").is_some_and(|v| v.contains(c) || (v == "1" && "b".contains(c)))
}

/// 連続列 (base でなく、固定されていない) の端数の有限の境界が、行と他の列の (使ってよい) 境界から導けるかを調べ、両側とも
/// (端数の側が全部) 導ける列に印を付ける。導けた境界は、その境界がなくても実行可能集合は変わらないので、TU の判定では無限と
/// みなしてよい。循環を避けるため、まだ導けていない端数の境界は使わず (無限とみなす)、前の回までに導けた境界だけを次の回で
/// 使う (導けた順に並べると、どの境界もそれより前の境界と行だけから導ける)。前処理が締めた在庫の上限 (tr12-30) がこれに当たる。
pub fn redundant_frac_bounds(p: &MipProblem, base: &[bool]) -> Vec<bool> {
    let n = p.n;
    let frac = |v: f64| v.is_finite() && !integral(v);
    let f: Vec<bool> = (0..n).map(|j| !base[j] && p.col_lo[j] < p.col_up[j] && (frac(p.col_lo[j]) || frac(p.col_up[j]))).collect();
    if !f.iter().any(|&b| b) {
        return vec![false; n];
    }
    let mut lo_use = p.col_lo.clone();
    let mut up_use = p.col_up.clone();
    let mut lo_pend = vec![false; n];
    let mut up_pend = vec![false; n];
    for j in 0..n {
        if f[j] {
            if frac(p.col_lo[j]) {
                lo_use[j] = f64::NEG_INFINITY;
                lo_pend[j] = true;
            }
            if frac(p.col_up[j]) {
                up_use[j] = f64::INFINITY;
                up_pend[j] = true;
            }
        }
    }
    // 行の活動の下限・上限 (有限の部分と無限の項の数)
    let act = |lo_use: &[f64], up_use: &[f64]| -> Vec<(f64, usize, f64, usize)> {
        p.rows
            .iter()
            .map(|r| {
                let (mut mn, mut nmn, mut mx, mut nmx) = (0.0, 0usize, 0.0, 0usize);
                for &(k, a) in r {
                    let (l, u) = if a > 0.0 { (lo_use[k], up_use[k]) } else { (up_use[k], lo_use[k]) };
                    if l.is_finite() { mn += a * l } else { nmn += 1 }
                    if u.is_finite() { mx += a * u } else { nmx += 1 }
                }
                (mn, nmn, mx, nmx)
            })
            .collect()
    };
    for _round in 0..60 {
        if !(0..n).any(|j| lo_pend[j] || up_pend[j]) {
            break;
        }
        let ac = act(&lo_use, &up_use);
        // 導いた境界 (列, 上限か, 値)。この回の終わりに使える境界に入れる (Jacobi の形)
        let mut done: Vec<(usize, bool, f64)> = Vec::new();
        let mut any = false;
        for j in 0..n {
            // 連続列 (base でない) だけ境界を導く。導いた境界は (冗長なので) 次の回から使ってよい
            if base[j] || p.col_lo[j] >= p.col_up[j] {
                continue;
            }
            let (mut best_u, mut best_l) = (up_use[j], lo_use[j]);
            for &(i, a) in &p.cols[j] {
                let (mn, nmn, mx, nmx) = ac[i];
                // 自分の寄与を除いた他の列の活動の下限・上限
                let (l, u) = if a > 0.0 { (lo_use[j], up_use[j]) } else { (up_use[j], lo_use[j]) };
                let omin = if l.is_finite() { if nmn == 0 { Some(mn - a * l) } else { None } } else if nmn == 1 { Some(mn) } else { None };
                let omax = if u.is_finite() { if nmx == 0 { Some(mx - a * u) } else { None } } else if nmx == 1 { Some(mx) } else { None };
                // a x_j ∈ [row_lo - omax, row_up - omin]
                let (rl, ru) = (p.row_lo[i], p.row_up[i]);
                let ax_up = omin.filter(|_| ru.is_finite()).map(|v| ru - v);
                let ax_lo = omax.filter(|_| rl.is_finite()).map(|v| rl - v);
                let (xu, xl) = if a > 0.0 { (ax_up.map(|v| v / a), ax_lo.map(|v| v / a)) } else { (ax_lo.map(|v| v / a), ax_up.map(|v| v / a)) };
                if let Some(v) = xu {
                    best_u = best_u.min(v);
                }
                if let Some(v) = xl {
                    best_l = best_l.max(v);
                }
            }
            let tol = |v: f64| 1e-9 * v.abs().max(1.0);
            if up_pend[j] && best_u <= p.col_up[j] + tol(p.col_up[j]) {
                up_pend[j] = false;
                any = true;
            }
            if lo_pend[j] && best_l >= p.col_lo[j] - tol(p.col_lo[j]) {
                lo_pend[j] = false;
                any = true;
            }
            // 使える境界を締める (列の境界より緩い値は使わない。少しの締めは数えない)
            let cu = best_u.max(p.col_lo[j]);
            if cu < up_use[j] - 1e-6 * cu.abs().max(1.0) {
                done.push((j, true, cu));
            }
            let cl = best_l.min(p.col_up[j]);
            if cl > lo_use[j] + 1e-6 * cl.abs().max(1.0) {
                done.push((j, false, cl));
            }
        }
        if done.is_empty() && !any {
            break;
        }
        for (j, up, v) in done {
            if up {
                up_use[j] = v;
            } else {
                lo_use[j] = v;
            }
        }
    }
    (0..n).map(|j| f[j] && !lo_pend[j] && !up_pend[j]).collect()
}

/// 整数かを、数値の大きさに応じた許容誤差で確かめる (大きすぎる値は整数とみなさない)。
fn integral(v: f64) -> bool {
    v.abs() <= 1e9 && (v - v.round()).abs() <= (1e-9 * v.abs().max(1.0)).min(1e-6)
}

/// 係数の大きさの限度 (SCIP の `numericslimit` と同じ 1e8)。行の倍率をかけた後でこれを超える係数のある行は使わない。
const NUM_LIMIT: f64 = 1e8;

/// `base` (整数列と暗黙の整数列の印) に、TU の部分による暗黙の整数列を加えた印を返す (モジュールの説明を見よ)。
pub fn implied_integers_tu2(p: &MipProblem, base: &[bool]) -> (Vec<bool>, Tu2Stats) {
    implied_integers_tu2_opt(p, base, tu2_flag('b'))
}

/// [`implied_integers_tu2`] の本体。`use_red` なら行から導ける端数の境界を無視する。
pub fn implied_integers_tu2_opt(p: &MipProblem, base: &[bool], use_red: bool) -> (Vec<bool>, Tu2Stats) {
    let n = p.n;
    let m = p.m;
    let mut st = Tu2Stats::default();
    // 端数の境界が行から導ける (冗長な) 列は、その境界を無いものとして候補にする ([`redundant_frac_bounds`])
    let red = if use_red { redundant_frac_bounds(p, base) } else { vec![false; n] };
    let mut cand: Vec<bool> = (0..n)
        .map(|j| {
            !base[j]
                && p.col_lo[j] < p.col_up[j]
                && !p.cols[j].is_empty()
                && (red[j] || [p.col_lo[j], p.col_up[j]].iter().all(|&v| !v.is_finite() || integral(v)))
        })
        .collect();
    st.red = red.iter().zip(&cand).filter(|(r, c)| **r && **c).count();
    st.cand0 = cand.iter().filter(|&&c| c).count();
    if st.cand0 == 0 {
        return (base.to_vec(), st);
    }
    let fixed_cont = |j: usize| !base[j] && p.col_lo[j] == p.col_up[j];
    // 2. 行の確かめ (候補を外すと、その列の行を確かめ直す)
    // 行が良いか (悪ければ理由の番号、[`Tu2Stats::why`])
    let row_bad = |i: usize, cand: &[bool]| -> Option<usize> {
        let r = &p.rows[i];
        let &(_, a0) = r.iter().find(|&&(j, _)| cand[j])?;
        let s = 1.0 / a0.abs();
        for &(j, a) in r {
            let v = a * s;
            if v.abs() > NUM_LIMIT {
                return Some(5);
            } else if cand[j] {
                if (v.abs() - 1.0).abs() > 1e-9 {
                    return Some(1);
                }
            } else if base[j] {
                if !integral(v) {
                    return Some(2);
                }
            } else if !fixed_cont(j) {
                return Some(0);
            } else if !integral(p.col_lo[j] * v) {
                return Some(4);
            }
        }
        if [p.row_lo[i], p.row_up[i]].iter().any(|&v| v.is_finite() && !integral(v * s)) {
            return Some(3);
        }
        None
    };
    let mut queued = vec![true; m];
    let mut q: std::collections::VecDeque<usize> = (0..m).collect();
    while let Some(i) = q.pop_front() {
        queued[i] = false;
        let Some(why) = row_bad(i, &cand) else { continue };
        st.why[why] += 1;
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
            // 診断 `F` (誤答になりうる、計測の見積もりだけに使う): TU と判定できない芯も TU とみなす。ネットワーク行列の一般の判定を
            // 入れたら効くかを、TU らしい問題 (mc11・sp150x300d の前処理の集約で形の崩れた流れ) で前もって見るため
            if !tu2_flag('F') {
                comp_bad[root] = true;
            }
        }
    }
    // `e`: TU と判定できなかった全体の成分を、重複の除去と行の分割で同値な形に直して判定し直す ([`is_tu_er`])
    if tu_flag_e() {
        let bad_roots: Vec<usize> = (0..n).filter(|&j| cand[j] && find(&mut uf, j) == j && comp_bad[j]).collect();
        if !bad_roots.is_empty() {
            let mut rows_of: std::collections::HashMap<usize, Vec<usize>> = bad_roots.iter().map(|&r| (r, Vec::new())).collect();
            for i in 0..m {
                if let Some(&(j0, _)) = crow[i].first() {
                    let r = find(&mut uf, j0);
                    if let Some(v) = rows_of.get_mut(&r) {
                        v.push(i);
                    }
                }
            }
            for root in bad_roots {
                let rs = &rows_of[&root];
                let nnz: usize = rs.iter().map(|&i| crow[i].len()).sum();
                if nnz > 200_000 {
                    continue;
                }
                // 列を詰めた番号に直す
                let mut lid: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
                let rows: Vec<Vec<(usize, i8)>> = rs
                    .iter()
                    .map(|&i| {
                        let mut r: Vec<(usize, i8)> = crow[i]
                            .iter()
                            .map(|&(j, pos)| {
                                let k = lid.len();
                                (*lid.entry(j).or_insert(k), if pos { 1 } else { -1 })
                            })
                            .collect();
                        r.sort_unstable();
                        r
                    })
                    .collect();
                let eq: Vec<bool> = rs.iter().map(|&i| p.row_lo[i] == p.row_up[i]).collect();
                if is_tu_er(rows, &eq, lid.len()) {
                    comp_bad[root] = false;
                    st.er_ok += 1;
                } else {
                    st.er_bad += 1;
                }
            }
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
    if tu2_flag('n') {
        return b;
    }
    let (mut t, st) = implied_integers_tu2(p, &b);
    // `v`: 変数上下限の行 (この列と整数列の 2 つだけの行、x <= u y の形) に出る列は、カットでは連続のままにする。整数として扱うと
    // c-MIR の変数上限の置き換えと flow cover の対象から外れ、固定費用の流れの問題 (exp-1-500-5-5・h80x6320d) で根の下界が大きく
    // 下がった。SCIP (cuts.c、2025 年の master) も連続の暗黙の整数には変数上下限の置き換えを許し、集約では連続として消す
    let mut nvb = 0;
    if tu2_flag('v') {
        for j in 0..p.n {
            if t[j] && !b[j] && p.cols[j].iter().any(|&(i, _)| p.rows[i].len() == 2 && p.rows[i].iter().any(|&(k, _)| k != j && p.is_int[k])) {
                t[j] = false;
                nvb += 1;
            }
        }
    }
    if verbose {
        eprintln!(
            "MIP: implied integers: {} by the row rules, +{} with the TU2 rule (-{} with variable bounds kept continuous) ({:?}, {:.3}s)",
            b.iter().zip(&p.is_int).filter(|(a, i)| **a && !**i).count(),
            st.marked,
            nvb,
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
    let (t2n, _) = implied_integers_tu2_opt(p, &b, false);
    let (t2, st) = implied_integers_tu2_opt(p, &b, true);
    let nt2n = t2n.iter().zip(&b).filter(|(a, i)| **a && !**i).count();
    // TU2 が既存の TU の印を全部含むか (含まない列の数)
    let miss = t1.iter().zip(&t2).filter(|(a, c)| **a && !**c).count();
    let costed = (0..p.n).filter(|&j| p.cost[j] != 0.0 && !p.is_int[j]).count();
    let costed_t2 = (0..p.n).filter(|&j| p.cost[j] != 0.0 && !p.is_int[j] && !t2[j]).count();
    // OBJINT (analysis/objint_20261112.md): HiGHS の双対側の規則 (`implied_integers_ho2`) での刻みと、行の規則だけの刻み
    let h = p.implied_integers_ho2();
    let nh = h.iter().zip(&b).filter(|(a, i)| **a && !**i).count();
    eprintln!(
        "IMPLINTCOUNT step_rowrule {:?} step_ho2 {:?} step_hf {:?} ho2_extra {} costed_cont_left_ho2 {}",
        p.objective_step_mask(&b, None),
        p.objective_step_mask(&h, None),
        p.objective_step_mask(&h, Some((&p.col_lo, &p.col_up))),
        nh,
        (0..p.n).filter(|&j| p.cost[j] != 0.0 && !h[j]).count()
    );
    eprintln!(
        "IMPLINTCOUNT n {} m {} cont {} rowrule {} tu {} tu2nob {} tu2 {} tu_not_in_tu2 {} costed_cont {} costed_cont_left_tu2 {} step0 {:?} step_tu2 {:?} stats {:?} ({:.3}s)",
        p.n,
        p.m,
        ncont,
        nb,
        nt1,
        nt2n,
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

    #[test]
    fn tu2_redundant_fractional_bound() {
        // 在庫の流れ: x0 - s0 = 0、s0 + x1 - s1 = 0 (保存)、x_t - 10 y_t <= 0 (容量、y は 2 値)。s0 <= 10.5 は x0 <= 10 y0 <= 10
        // から導ける (2 段)。境界を無視すれば TU。s1 <= 7.5 は導けない (x1 + s0 は 20 まで) ので成分ごと外れる
        let rows = vec![vec![(0, 1.0), (2, -1.0)], vec![(2, 1.0), (1, 1.0), (3, -1.0)], vec![(0, 1.0), (4, -10.0)], vec![(1, 1.0), (5, -10.0)]];
        let mk = |s1up: f64| {
            MipProblem::from_rows(
                vec![0.0; 6],
                vec![INF, INF, 10.5, s1up, 1.0, 1.0],
                vec![0.0; 6],
                0.0,
                1.0,
                vec![false, false, false, false, true, true],
                rows.clone(),
                vec![0.0, 0.0, -INF, -INF],
                vec![0.0, 0.0, 0.0, 0.0],
            )
        };
        let p = mk(INF);
        let b = p.implied_integers();
        assert_eq!(redundant_frac_bounds(&p, &b), vec![false, false, true, false, false, false]);
        assert_eq!(implied_integers_tu2_opt(&p, &b, false).0[..4], [false; 4]);
        assert_eq!(implied_integers_tu2_opt(&p, &b, true).0[..4], [true; 4]);
        let p = mk(7.5);
        let b = p.implied_integers();
        assert_eq!(implied_integers_tu2_opt(&p, &b, true).0[..4], [false; 4]);
    }

    #[test]
    fn tu2_split_recovers_contracted_network() {
        // 節点 0..4 の流れ。枝: a0 0→1、a1 1→3、a2 2→3、a3 2→4、a4 0→2、a5 3→4。消した枝 k: 1→2 (前処理が節点 1 の行で消した)。
        // 前処理の後の行: 節点 0、節点 1∪2 (和、等式)、節点 3、節点 4、k の容量の行 (節点 1 の行の連続列の部分 + 整数列、不等式) と
        // その符号を反転した重複。芯の列の非零が 3 つになり 2 色塗りでは判定できないが、`e` の分割で接続行列に戻る
        let out = |v: usize, arcs: &[(usize, usize)]| -> Vec<(usize, f64)> {
            let mut r: Vec<(usize, f64)> = Vec::new();
            for (a, &(f, t)) in arcs.iter().enumerate() {
                if f == v {
                    r.push((a, 1.0));
                } else if t == v {
                    r.push((a, -1.0));
                }
            }
            r
        };
        let arcs = [(0, 1), (1, 3), (2, 3), (2, 4), (0, 2), (3, 4)];
        let y = 6; // 整数列 (k の開閉)
        let mut merged = out(1, &arcs);
        for (a, v) in out(2, &arcs) {
            merged.push((a, v));
        }
        merged.sort_by_key(|&(a, _)| a);
        let mut cap = out(1, &arcs); // x_k = 1 - (節点 1 の他の枝) <= 5 y  →  -(節点 1 の他の枝) - 5 y <= -1
        for t in cap.iter_mut() {
            t.1 = -t.1;
        }
        cap.push((y, -5.0));
        let mut cap2: Vec<(usize, f64)> = cap.iter().map(|&(a, v)| if a == y { (a, 7.0) } else { (a, -v) }).collect();
        cap2.sort_by_key(|&(a, _)| a);
        let rows = vec![out(0, &arcs), merged, out(3, &arcs), out(4, &arcs), cap, cap2];
        let mut up = vec![INF; 7];
        up[y] = 1.0;
        let mut is_int = vec![false; 7];
        is_int[y] = true;
        let p = prob(7, is_int, up, rows, vec![-2.0, 3.0, 0.0, -1.0, -INF, -INF], vec![-2.0, 3.0, 0.0, -1.0, -1.0, 9.0]);
        let b = p.implied_integers();
        let (t, st) = implied_integers_tu2_opt(&p, &b, true);
        assert_eq!(t[..6], [false; 6], "{st:?}");
        // 分割の判定を直接呼ぶ (環境変数によらず)
        let crow: Vec<Vec<(usize, i8)>> = p.rows.iter().map(|r| r.iter().filter(|&&(j, _)| j < 6).map(|&(j, a)| (j, if a > 0.0 { 1 } else { -1 })).collect()).collect();
        assert!(!tu_shape(&crow, 6));
        assert!(is_tu_er(crow, &[true, true, true, true, false, false], 6));
    }

    #[test]
    fn tu2_split_random_is_tu() {
        // でたらめな小さな 0/±1 行列で、is_tu_er が真なら本当に TU (全部の正方の部分行列の行列式) か
        let mut seed: u64 = 777;
        let mut rnd = |k: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % k
        };
        let (mut yes, mut yes_shape) = (0, 0);
        for _ in 0..20000 {
            let m = 2 + rnd(5) as usize;
            let n = 2 + rnd(5) as usize;
            let mut rows: Vec<Vec<(usize, i8)>> = Vec::new();
            for _ in 0..m {
                let mut r = Vec::new();
                for j in 0..n {
                    match rnd(5) {
                        0 => r.push((j, 1i8)),
                        1 => r.push((j, -1i8)),
                        _ => {}
                    }
                }
                rows.push(r);
            }
            let a: Vec<Vec<i64>> = rows.iter().map(|r| (0..n).map(|j| r.iter().find(|&&(k, _)| k == j).map_or(0, |&(_, s)| s as i64)).collect()).collect();
            if tu_shape(&rows, n) {
                yes_shape += 1;
                assert!(is_tu(&a), "tu_shape: {a:?}");
            }
            if is_tu_er(rows, &vec![false; m], n) {
                yes += 1;
                assert!(is_tu(&a), "is_tu_er: {a:?}");
            }
        }
        assert!(yes > yes_shape && yes_shape > 1000, "{yes} {yes_shape}");
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
