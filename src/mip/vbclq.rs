//! 試作 `ENOMOTO_MIP_VBCLQ` (既定では使わない。analysis/scipgate_20261120): 目的の上限とクリーク表の含意による 0-1 列の固定
//! (SCIP の pseudoobj の propuseimplics の形)。Solver の根の LP の前 ([`super::solver::Solver::vbclq_pseudoobj`]) と、
//! 前処理の LP による固定の前 (値 `e`、[`super::presolve_vbclq_e`]) から呼ぶ。

use super::clique::CliqueGraph;
use super::domain::Domain;
use super::problem::MipProblem;

/// [`pseudoobj`] の結果
#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct PseudoObjOut {
    /// 固定した列の数
    pub fixed: usize,
    /// 回した回数
    pub passes: usize,
    /// 上限より良い解はない (余裕が負、または固定の後の伝播が矛盾)
    pub infeas: bool,
    /// 目的を配り直した列の数
    pub nsub: usize,
    /// 最初の余裕 (上限 − 目的の最小値)
    pub slack0: f64,
}

/// 目的の上限 `u` (`cost · x <= u`、定数項を除く) とクリーク表 `g` で 0-1 列を大域的に固定する。伝播はせず、手間は辺の数に比例する。
/// - 目的の最小値 (各列を今の境界の目的に良い側に置いた値) と `u` の差を余裕とする。
/// - 自由な 0-1 列の文字 l (x_j = 1 または 0) を真にしたときの目的の損 = 自分の損 + 衝突グラフで l と衝突する文字 m が偽になる
///   列の損 (列ごとに 1 回)。偽にできない (固定された列の値と食い違う、または同じ列の両方の文字と衝突) なら無限大。
/// - 損が余裕を超える文字は偽にできるので、その列を反対の値に大域的に固定する (目的に悪い側への固定なら余裕を減らす)。
/// - 固定が出なくなるまで回す (間に行の伝播を挟む)。回数は `ENOMOTO_T_MIP_VBCLQ_PASSES` (既定 10) まで。
///
/// 損が余裕をちょうど超えるときだけ固定するので、目的値が `u` 以下の解は全て残る (`u` を暫定解の値にすれば暫定解も残る)。
/// 目的の配り直し (前処理は変えず、ここでの数え方だけ): 目的を持つ列 j が 1 本の等式 `a x_j + sum a_k x_k = b` にしか
/// 出なければ、目的 c_j x_j = c_j (b - sum a_k x_k) / a なので、目的を他の列に配る (j 自身の目的は 0、定数 c_j b / a)。
/// 列 j の境界は使わないだけ (弱くなるだけで正しい)。同じ行を 2 回使わず、配り先に配り直す列を含む行は使わない
pub(crate) fn pseudoobj(p: &MipProblem, dom: &mut Domain, g: &CliqueGraph, u: f64) -> PseudoObjOut {
    let mut out = PseudoObjOut { slack0: f64::NAN, ..Default::default() };
    if !u.is_finite() || dom.infeasible {
        return out;
    }
    let tol = 1e-9 * u.abs().max(1.0);
    let bin = |d: &Domain, j: usize| p.is_int[j] && d.lo[j] == 0.0 && d.up[j] == 1.0;
    let max_passes = tunable!("ENOMOTO_T_MIP_VBCLQ_PASSES", 10usize, usize);
    let mut mark: Vec<u32> = vec![u32::MAX; p.n];
    let mut cnt = vec![0u32; p.n];
    let mut where_: Vec<usize> = vec![usize::MAX; p.n];
    for (i, r) in p.rows.iter().enumerate() {
        for &(k, _) in r {
            cnt[k] += 1;
            where_[k] = i;
        }
    }
    let mut cost = p.cost.clone();
    let mut cconst = 0.0;
    let mut row_used = vec![false; p.m];
    for j in 0..p.n {
        if p.cost[j] == 0.0 || cnt[j] != 1 {
            continue;
        }
        let i = where_[j];
        if row_used[i] || p.row_lo[i] != p.row_up[i] || !p.row_lo[i].is_finite() {
            continue;
        }
        let r = &p.rows[i];
        if r.iter().any(|&(k, _)| k != j && p.cost[k] != 0.0 && cnt[k] == 1) {
            continue;
        }
        let a = r.iter().find(|&&(k, _)| k == j).map_or(0.0, |e| e.1);
        if a == 0.0 {
            continue;
        }
        let f = p.cost[j] / a;
        for &(k, ak) in r {
            if k != j {
                cost[k] -= f * ak;
            }
        }
        cost[j] = 0.0;
        cconst += f * p.row_lo[i];
        row_used[i] = true;
        out.nsub += 1;
    }
    while out.passes < max_passes && !out.infeas {
        out.passes += 1;
        // 目的の最小値 (無限なら何もしない)
        let mut minobj = cconst;
        for j in 0..p.n {
            let c = cost[j];
            if c != 0.0 {
                minobj += if c > 0.0 { c * dom.lo[j] } else { c * dom.up[j] };
            }
        }
        if !minobj.is_finite() {
            break;
        }
        let mut slack = u - minobj;
        if out.passes == 1 {
            out.slack0 = slack;
        }
        if slack < -tol {
            out.infeas = true;
            break;
        }
        let fixed0 = out.fixed;
        for j in 0..p.n {
            if !bin(dom, j) {
                continue;
            }
            for (side, l) in [(1.0f64, 2 * j as u32), (0.0, 2 * j as u32 + 1)] {
                // 自分の損: x_j = side
                let c = cost[j];
                let mut loss = if side == 1.0 { c.max(0.0) } else { (-c).max(0.0) };
                let mut inf = false;
                for &m in g.neighbors(l) {
                    let k = (m / 2) as usize;
                    if k == j {
                        continue;
                    }
                    // m が偽: x_k = 0 (m = 2k) / x_k = 1 (m = 2k + 1)
                    let v = if m % 2 == 0 { 0.0 } else { 1.0 };
                    if v < dom.lo[k] || v > dom.up[k] || mark[k] == l {
                        // 固定された値と食い違う、または同じ列の両方の文字と衝突: l は真になれない
                        inf = true;
                        break;
                    }
                    mark[k] = l;
                    if dom.lo[k] < dom.up[k] {
                        let ck = cost[k];
                        let best = if ck > 0.0 { dom.lo[k] } else { dom.up[k] };
                        loss += ck * (v - best);
                    }
                }
                if inf || loss > slack + tol {
                    // 文字 l は偽: x_j = 1 - side
                    let w = 1.0 - side;
                    dom.tighten_global(p, j, w == 0.0, w);
                    if w == 0.0 {
                        dom.tighten_upper(p, j, 0.0);
                    } else {
                        dom.tighten_lower(p, j, 1.0);
                    }
                    out.fixed += 1;
                    // 目的に悪い側に固定したら余裕が減る
                    let best = if c > 0.0 { 0.0 } else { 1.0 };
                    slack -= c * (w - best);
                    if slack < -tol {
                        out.infeas = true;
                    }
                    break;
                }
            }
            if out.infeas {
                break;
            }
        }
        if out.infeas || out.fixed == fixed0 {
            break;
        }
        if !dom.propagate(p) {
            out.infeas = true;
            break;
        }
        mark.iter_mut().for_each(|s| *s = u32::MAX);
    }
    out
}

/// 診断 `ENOMOTO_MIP_VBCLQ_COUNT_FILE=<ファイル>`: 呼ばれるたびに「固定した列の数 打ち切りの証明 (0/1)」を 1 行足す (fuzz で働いた回数を数える)
pub(crate) fn count_file(o: &PseudoObjOut) {
    if let Some(f) = env_str!("ENOMOTO_MIP_VBCLQ_COUNT_FILE") {
        use std::io::Write;
        if let Ok(mut fh) = std::fs::OpenOptions::new().create(true).append(true).open(f) {
            let _ = writeln!(fh, "{} {}", o.fixed, o.infeas as u8);
        }
    }
}
