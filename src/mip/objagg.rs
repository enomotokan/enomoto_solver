//! 診断 `ENOMOTO_MIP_DIAG_OBJAGG` (analysis/rootcutoff_20261121、解き方は変えない): 打ち切り値 U による目的関数の行
//! `c x ≤ U − 定数項` を出発点にしたカットが、伝播 (固定) と根の LP の下界を強くするかを数える。
//!
//! 外側の主の根の、カットのループの後 (早めの被約費用の記録の前) で、打ち切り値が有限なら 1 行を出す。数えるもの (どれも大域的な境界から):
//! - `base`: 元の行 + LP のカットの行の伝播で固定される整数列 (大域的な境界は伝播済みなので、ふつうは今の固定の数と同じ)
//! - `a`: それに目的関数の行を足した伝播 (SCIP の pseudoobj に当たる)
//! - `b`: 根の被約費用 (`root_redcost`) による締めを入れた伝播
//! - `ab`: `a` と `b` の両方
//! - `c1`: `ab` に、目的関数の行 1 本だけから作ったカット (cmir・被覆、`generate_cuts`) を足した伝播
//! - `c2`: `ab` に、目的関数の行から連続列を消しながら LP の行を足していった各段 (分離の経路の集約と同じ選び方、最大 6 段) から作ったカットを足した伝播
//!
//! `=t` は目的関数の行の余裕を 0 とみなした点でカットを作る (既定の `=1` は LP 解で作るので、目的の行が効いていなければカットは出ない)。
//! あわせて、根の LP (元の行 + カットの行) に c1・c2 のカットを足した LP の値を出す。カットは「打ち切り値より良い解」でだけ成り立つ
//! (被約費用による固定と同じ種類の推論)。
use super::cuts::{generate_cuts, CutVars, RawCut};
use super::domain::Domain;
use super::lp::{LpStatus, SolveLimits};
use super::lp_api::MipLp;
use super::problem::MipProblem;
use super::solver::Solver;
use std::time::Instant;

pub(super) fn on() -> bool {
    env_str!("ENOMOTO_MIP_DIAG_OBJAGG").is_some_and(|v| v != "0")
}

/// 伝播を落ち着くまで回す (矛盾なら偽)
fn propagate_all(p: &MipProblem, d: &mut Domain) -> bool {
    for _ in 0..50 {
        if !d.propagate(p) {
            return false;
        }
        if !d.has_pending() {
            break;
        }
    }
    true
}

/// `rows` (と境界 `lo`・`up`) の問題で、`tight` の締めを入れて伝播し、固定された整数列の数を返す (矛盾なら None)
#[allow(clippy::too_many_arguments)]
fn count_fixed(p: &MipProblem, lo: &[f64], up: &[f64], rows: &[Vec<(usize, f64)>], rlo: &[f64], rup: &[f64], tight: &[(usize, bool, f64)]) -> Option<usize> {
    let q = MipProblem::from_rows(lo.to_vec(), up.to_vec(), p.cost.clone(), p.offset, p.sense_sign, p.is_int.clone(), rows.to_vec(), rlo.to_vec(), rup.to_vec());
    let mut d = Domain::new(&q);
    for &(j, upper, v) in tight {
        if upper {
            d.tighten_upper(&q, j, v);
        } else {
            d.tighten_lower(&q, j, v);
        }
    }
    if d.infeasible || !propagate_all(&q, &mut d) {
        return None;
    }
    Some((0..q.n).filter(|&j| q.is_int[j] && d.lo[j] == d.up[j]).count())
}

/// 分離の空間 (列 + 行の余裕の列) のカットを構造変数の式に戻す
fn to_struct(c: &RawCut, n: usize, srows: &[Vec<(usize, f64)>]) -> (Vec<(usize, f64)>, f64) {
    let mut acc = std::collections::HashMap::<usize, f64>::new();
    for &(k, a) in &c.coefs {
        if k < n {
            *acc.entry(k).or_default() += a;
        } else {
            for &(j, b) in &srows[k - n] {
                *acc.entry(j).or_default() += a * b;
            }
        }
    }
    let mut v: Vec<(usize, f64)> = acc.into_iter().filter(|&(_, a)| a.abs() > 1e-12).collect();
    v.sort_by_key(|&(j, _)| j);
    (v, c.rhs)
}

impl<'a, L: MipLp> Solver<'a, L> {
    pub(super) fn diag_objagg(&mut self) {
        let lim = self.prune_limit();
        if !lim.is_finite() {
            return;
        }
        let t0 = Instant::now();
        let p = self.p;
        let n = p.n;
        let u = lim - p.offset;
        let (glo, gup) = (self.dom.global_lo.clone(), self.dom.global_up.clone());
        let nr = self.lp.num_rows();
        let rows: Vec<Vec<(usize, f64)>> = (0..nr).map(|i| self.lp.row(i)).collect();
        let (rlo, rup): (Vec<f64>, Vec<f64>) = (0..nr).map(|i| self.lp.row_bounds(i)).unzip();
        let obj: Vec<(usize, f64)> = (0..n).filter(|&j| p.cost[j] != 0.0).map(|j| (j, p.cost[j])).collect();
        let free0 = (0..n).filter(|&j| p.is_int[j] && glo[j] < gup[j]).count();
        // 被約費用の締め
        let mut rc: Vec<(usize, bool, f64)> = Vec::new();
        if let Some((z, list)) = &self.root_redcost {
            let gap = lim - z;
            if gap.is_finite() && gap >= 0.0 {
                for &(j, at_lower, d, bound) in list {
                    if at_lower {
                        rc.push((j, true, bound + gap / d));
                    } else {
                        rc.push((j, false, bound - gap / (-d)));
                    }
                }
            }
        }
        let with_obj = |extra: &[(Vec<(usize, f64)>, f64)]| -> (Vec<Vec<(usize, f64)>>, Vec<f64>, Vec<f64>) {
            let mut r = rows.clone();
            let mut l = rlo.clone();
            let mut h = rup.clone();
            r.push(obj.clone());
            l.push(f64::NEG_INFINITY);
            h.push(u);
            for (c, rhs) in extra {
                r.push(c.clone());
                l.push(f64::NEG_INFINITY);
                h.push(*rhs);
            }
            (r, l, h)
        };
        let fmt = |v: Option<usize>| v.map_or("infeasible".to_string(), |k| k.to_string());
        let base = count_fixed(p, &glo, &gup, &rows, &rlo, &rup, &[]);
        let (ro, lo_, hi_) = with_obj(&[]);
        let fa = count_fixed(p, &glo, &gup, &ro, &lo_, &hi_, &[]);
        let fb = count_fixed(p, &glo, &gup, &rows, &rlo, &rup, &rc);
        let fab = count_fixed(p, &glo, &gup, &ro, &lo_, &hi_, &rc);
        // 分離の空間: 列 + LP の行の余裕 + 目的関数の行の余裕 (添字 n + nr)
        let x = self.lp.col_values();
        let act = self.lp.row_activities();
        let mut vlo = glo.clone();
        let mut vup = gup.clone();
        let mut is_int = p.is_int.clone();
        let mut xv = x.clone();
        let mut srows = rows.clone();
        for i in 0..nr {
            vlo.push(rlo[i]);
            vup.push(rup[i]);
            is_int.push(false);
            xv.push(act[i]);
        }
        srows.push(obj.clone());
        vlo.push(f64::NEG_INFINITY);
        vup.push(u);
        is_int.push(false);
        xv.push(obj.iter().map(|&(j, c)| c * x[j]).sum());
        let vb = self.vbounds.clone();
        // `ENOMOTO_MIP_DIAG_OBJAGG=t`: 目的関数の行の余裕を 0 とみなした点で作る (LP 解では c x = z_LP < U で、目的の行は効いていないので
        // 違反するカットが出ない。カットの妥当性は点によらない)
        if env_str!("ENOMOTO_MIP_DIAG_OBJAGG") == Some("t") {
            *xv.last_mut().unwrap() = u;
        }
        let vars = CutVars { lo: &vlo, up: &vup, is_int: &is_int, x: &xv, vb: vb.as_deref(), imp: None };
        let gen = |agg: &[f64], weights: &[(usize, f64)], out: &mut Vec<RawCut>| {
            let mut b: Vec<(usize, f64)> = (0..n).filter(|&j| agg[j].abs() > 1e-9).map(|j| (j, agg[j])).collect();
            for &(i, w) in weights {
                b.push((n + i, -w));
            }
            let neg: Vec<(usize, f64)> = b.iter().map(|&(k, a)| (k, -a)).collect();
            out.extend(generate_cuts(&vars, &b, 0.0, true, true));
            out.extend(generate_cuts(&vars, &neg, 0.0, true, true));
        };
        // c1: 目的関数の行 1 本
        let mut agg = vec![0.0f64; n];
        for &(j, c) in &obj {
            agg[j] = c;
        }
        let mut c1: Vec<RawCut> = Vec::new();
        gen(&agg, &[(nr, 1.0)], &mut c1);
        // c2: 連続列を消しながら LP の行を足す (分離の経路の集約と同じ: 境界から最も離れた連続列を、効いている行を優先して消す)
        let mut col_rows: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, r) in rows.iter().enumerate() {
            if r.len() <= 500 {
                for &(j, _) in r {
                    if !p.is_int[j] {
                        col_rows[j].push(i);
                    }
                }
            }
        }
        let tight = |i: usize| (act[i] - rlo[i]).abs() <= 1e-6 * (1.0 + rlo[i].abs()) || (rup[i] - act[i]).abs() <= 1e-6 * (1.0 + rup[i].abs());
        let dist = |j: usize| (x[j] - glo[j]).min(gup[j] - x[j]).max(0.0);
        let mut weights = vec![(nr, 1.0)];
        let mut used = vec![false; nr];
        let mut c2: Vec<RawCut> = Vec::new();
        let mut steps = 0usize;
        for _ in 0..6 {
            let best = (0..n).filter(|&j| !p.is_int[j] && agg[j].abs() > 1e-9 && dist(j) > 1e-6).max_by(|&a, &b| dist(a).total_cmp(&dist(b)));
            let Some(j) = best else { break };
            let cand = col_rows[j].iter().copied().filter(|&i| !used[i]).max_by_key(|&i| tight(i));
            let Some(i) = cand else { break };
            let a = rows[i].iter().find(|&&(k, _)| k == j).map_or(0.0, |&(_, v)| v);
            if a == 0.0 {
                break;
            }
            let w = -agg[j] / a;
            for &(k, v) in &rows[i] {
                agg[k] += w * v;
            }
            agg[j] = 0.0;
            used[i] = true;
            weights.push((i, w));
            steps += 1;
            gen(&agg, &weights, &mut c2);
        }
        let s1: Vec<(Vec<(usize, f64)>, f64)> = c1.iter().map(|c| to_struct(c, n, &srows)).collect();
        let s2: Vec<(Vec<(usize, f64)>, f64)> = c2.iter().map(|c| to_struct(c, n, &srows)).collect();
        let (r1, l1, h1) = with_obj(&s1);
        let fc1 = count_fixed(p, &glo, &gup, &r1, &l1, &h1, &rc);
        let (r2, l2, h2) = with_obj(&s2);
        let fc2 = count_fixed(p, &glo, &gup, &r2, &l2, &h2, &rc);
        // 根の LP にカットを足した値 (目的関数の行も足す。根の LP の値では効かない)
        let lpv = |r: &[Vec<(usize, f64)>], l: &[f64], h: &[f64]| -> String {
            let mut lp = crate::simplex::mip_lp::TwoStageLp::new(&glo, &gup, &p.cost, r, l, h);
            match lp.solve(&SolveLimits { deadline: Some(Instant::now() + std::time::Duration::from_secs(60)), ..Default::default() }) {
                LpStatus::Optimal => format!("{:.10e}", lp.objective() + p.offset),
                s => format!("{s:?}"),
            }
        };
        let viol = |cs: &[(Vec<(usize, f64)>, f64)]| cs.iter().filter(|(c, rhs)| c.iter().map(|&(j, a)| a * x[j]).sum::<f64>() > rhs + 1e-6).count();
        eprintln!(
            "OBJAGG restarts {} cutoff {:.10e} free int {free0} rc bounds {} | fixed: base {} a(obj row) {} b(redcost) {} ab {} c1 {} c2 {} | cuts c1 {} ({} violated) c2 {} ({} violated, {steps} steps) | LP {} c1 {} c2 {} ({:.2}s)",
            self.params.restarts,
            lim,
            rc.len(),
            fmt(base),
            fmt(fa),
            fmt(fb),
            fmt(fab),
            fmt(fc1),
            fmt(fc2),
            s1.len(),
            viol(&s1),
            s2.len(),
            viol(&s2),
            self.lp.objective() + p.offset,
            lpv(&r1, &l1, &h1),
            lpv(&r2, &l2, &h2),
            t0.elapsed().as_secs_f64()
        );
    }
}
