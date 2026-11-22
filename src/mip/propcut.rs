//! 診断 `ENOMOTO_MIP_DIAG_PROPCUT` (analysis/propcut_20261122、解き方は変えない): LP 解を切るかでなく、伝播で固定を生むかで
//! 不等式を見る。外側の主の根の、カットのループの後 (早めの被約費用の記録の直後・固定の前) で、打ち切り値が有限なら 1 行を出す。
//!
//! 基準 `ab` は「元の行 + LP のカットの行 + 目的関数の行 `c x ≤ U` + 根の被約費用の締め」の伝播で固定される整数列 (objagg と同じ)。
//! それに次を足した伝播の固定の数を数える (どれも大域的な境界から):
//! - `a`: 根のカットの各ラウンドの LP の被約費用の締め (`propcut_rc`。この診断の間だけ集め、締めには使わない)
//! - `b`: 被約費用の不等式 `Σ d_j x_j ≤ (U − z) + Σ d_j 境界_j` (非基底の整数列) に MIR の丸め (δ は係数から最大 4 個) をかけた行
//!   (根と各ラウンドの被約費用の表から)。目的関数の行 + 双対 × 行 の丸め
//! - `c`: LP の行 (元の行 + カットの行、500 列以下) と目的関数の行の両側に、同じ MIR の丸めをかけた行 (LP 解を切るかは見ない)
//! - `all`: a・b・c の全部
//!
//! MIR の作り方: 連続列は係数の向きの有限の境界で置き換えて落とし (緩和)、整数列は係数が正なら下限、負なら上限で補う (y ≥ 0)。
//! δ で割って `Σ (⌊α_j⌋ + max(0, f_j − f0) / (1 − f0)) y_j ≤ ⌊β⌋`。境界が無限の列があればその行は作らない。
//! あわせて、`ab` と `all` の伝播の境界で根の LP (元の行 + カットの行 + 目的関数の行) を解き直した値を出す。
use super::domain::Domain;
use super::lp::{LpStatus, SolveLimits};
use super::lp_api::MipLp;
use super::problem::MipProblem;
use super::solver::Solver;
use std::time::Instant;

pub(super) fn on() -> bool {
    env_str!("ENOMOTO_MIP_DIAG_PROPCUT").is_some_and(|v| v != "0")
}

type Row = (Vec<(usize, f64)>, f64);

/// `rows` (`≤ rup`、`≥ rlo`) と締め `tight` で伝播し、固定された整数列の数と境界を返す (矛盾なら None)
#[allow(clippy::too_many_arguments)]
fn prop(p: &MipProblem, lo: &[f64], up: &[f64], rows: &[Vec<(usize, f64)>], rlo: &[f64], rup: &[f64], tight: &[(usize, bool, f64)]) -> Option<(usize, Vec<f64>, Vec<f64>)> {
    let q = MipProblem::from_rows(lo.to_vec(), up.to_vec(), p.cost.clone(), p.offset, p.sense_sign, p.is_int.clone(), rows.to_vec(), rlo.to_vec(), rup.to_vec());
    let mut d = Domain::new(&q);
    for &(j, upper, v) in tight {
        if upper {
            d.tighten_upper(&q, j, v);
        } else {
            d.tighten_lower(&q, j, v);
        }
    }
    if d.infeasible {
        return None;
    }
    for _ in 0..50 {
        if !d.propagate(&q) {
            return None;
        }
        if !d.has_pending() {
            break;
        }
    }
    let k = (0..q.n).filter(|&j| q.is_int[j] && d.lo[j] == d.up[j]).count();
    Some((k, d.lo.clone(), d.up.clone()))
}

/// 1 行 `Σ a_j x_j ≤ b` の MIR の行 (δ ごと) を `out` に足す
fn mir_rows(row: &[(usize, f64)], b: f64, is_int: &[bool], lo: &[f64], up: &[f64], out: &mut Vec<Row>) {
    let mut beta = b;
    // (列, 係数, 補った向き: 偽 = 下限 y = x − l、真 = 上限 y = u − x)
    let mut ints: Vec<(usize, f64, bool)> = Vec::new();
    for &(j, a) in row {
        if a.abs() < 1e-12 || lo[j] == up[j] {
            beta -= a * lo[j];
            continue;
        }
        if !is_int[j] {
            let bd = if a > 0.0 { lo[j] } else { up[j] };
            if !bd.is_finite() {
                return;
            }
            beta -= a * bd;
            continue;
        }
        if a > 0.0 {
            if !lo[j].is_finite() {
                return;
            }
            beta -= a * lo[j];
            ints.push((j, a, false));
        } else {
            if !up[j].is_finite() {
                return;
            }
            beta -= a * up[j];
            ints.push((j, -a, true));
        }
    }
    if ints.is_empty() || !beta.is_finite() || beta < 0.0 {
        return;
    }
    let mut deltas: Vec<f64> = ints.iter().map(|&(_, a, _)| a).collect();
    deltas.sort_by(|x, y| y.total_cmp(x));
    deltas.dedup_by(|x, y| (*x - *y).abs() <= 1e-9 * x.abs().max(1.0));
    deltas.truncate(4);
    for d in deltas {
        let bb = beta / d;
        let f0 = bb - bb.floor();
        if f0 < 1e-6 || f0 > 1.0 - 1e-6 {
            continue;
        }
        let mut coefs = Vec::with_capacity(ints.len());
        let mut rhs = bb.floor();
        for &(j, a, comp) in &ints {
            let al = a / d;
            let fj = al - al.floor();
            let g = al.floor() + ((fj - f0).max(0.0)) / (1.0 - f0);
            if g.abs() < 1e-12 {
                continue;
            }
            if comp {
                // y = u − x: g (u − x) → −g x、右辺から g u を引く
                coefs.push((j, -g));
                rhs -= g * up[j];
            } else {
                coefs.push((j, g));
                rhs += g * lo[j];
            }
        }
        if !coefs.is_empty() && rhs.is_finite() {
            out.push((coefs, rhs));
        }
    }
}

impl<'a, L: MipLp> Solver<'a, L> {
    pub(super) fn diag_propcut(&mut self) {
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
        // 被約費用の表 (根と各ラウンド) と、その締め
        let tighten = |z: f64, list: &[(usize, bool, f64, f64)], out: &mut Vec<(usize, bool, f64)>| {
            let gap = lim - z;
            if gap.is_finite() && gap >= 0.0 {
                for &(j, at_lower, d, bound) in list {
                    if at_lower {
                        out.push((j, true, bound + gap / d));
                    } else {
                        out.push((j, false, bound - gap / (-d)));
                    }
                }
            }
        };
        let mut rc0 = Vec::new();
        if let Some((z, list)) = &self.root_redcost {
            tighten(*z, list, &mut rc0);
        }
        let mut rca = rc0.clone();
        for (z, list) in &self.propcut_rc {
            tighten(*z, list, &mut rca);
        }
        // (b) 被約費用の不等式の MIR
        let mut lists: Vec<(f64, &Vec<(usize, bool, f64, f64)>)> = self.root_redcost.iter().map(|(z, l)| (*z, l)).collect();
        lists.extend(self.propcut_rc.iter().map(|(z, l)| (*z, l)));
        let mut cb: Vec<Row> = Vec::new();
        for (z, list) in &lists {
            let gap = lim - z;
            if !(gap.is_finite() && gap >= 0.0) {
                continue;
            }
            let mut r = Vec::new();
            let mut rhs = gap;
            for &(j, _, d, bound) in list.iter() {
                if p.is_int[j] && glo[j] < gup[j] {
                    r.push((j, d));
                    rhs += d * bound;
                }
            }
            mir_rows(&r, rhs, &p.is_int, &glo, &gup, &mut cb);
        }
        // (c) LP の行と目的関数の行の両側の MIR (LP 解を切るかは見ない)
        let mut cc: Vec<Row> = Vec::new();
        mir_rows(&obj, u, &p.is_int, &glo, &gup, &mut cc);
        for i in 0..nr {
            if rows[i].len() > 500 {
                continue;
            }
            if rup[i].is_finite() {
                mir_rows(&rows[i], rup[i], &p.is_int, &glo, &gup, &mut cc);
            }
            if rlo[i].is_finite() {
                let neg: Vec<(usize, f64)> = rows[i].iter().map(|&(j, a)| (j, -a)).collect();
                mir_rows(&neg, -rlo[i], &p.is_int, &glo, &gup, &mut cc);
            }
        }
        let build = |extra: &[&Vec<Row>]| -> (Vec<Vec<(usize, f64)>>, Vec<f64>, Vec<f64>) {
            let mut r = rows.clone();
            let mut l = rlo.clone();
            let mut h = rup.clone();
            r.push(obj.clone());
            l.push(f64::NEG_INFINITY);
            h.push(u);
            for e in extra {
                for (c, rhs) in e.iter() {
                    r.push(c.clone());
                    l.push(f64::NEG_INFINITY);
                    h.push(*rhs);
                }
            }
            (r, l, h)
        };
        let (r0, l0, h0) = build(&[]);
        let ab = prop(p, &glo, &gup, &r0, &l0, &h0, &rc0);
        let fa = prop(p, &glo, &gup, &r0, &l0, &h0, &rca);
        let (rb, lb, hb) = build(&[&cb]);
        let fb = prop(p, &glo, &gup, &rb, &lb, &hb, &rc0);
        let (rc_, lc, hc) = build(&[&cc]);
        let fc = prop(p, &glo, &gup, &rc_, &lc, &hc, &rc0);
        let (rall, lall, hall) = build(&[&cb, &cc]);
        let fall = prop(p, &glo, &gup, &rall, &lall, &hall, &rca);
        // 伝播の境界で根の LP (元の行 + カットの行 + 目的関数の行) を解き直す
        let lpv = |d: &Option<(usize, Vec<f64>, Vec<f64>)>| -> String {
            let Some((_, lo, up)) = d else { return "infeasible".into() };
            let mut lp = crate::simplex::mip_lp::TwoStageLp::new(lo, up, &p.cost, &r0, &l0, &h0);
            match lp.solve(&SolveLimits { deadline: Some(Instant::now() + std::time::Duration::from_secs(60)), ..Default::default() }) {
                LpStatus::Optimal => format!("{:.10e}", lp.objective() + p.offset),
                LpStatus::Infeasible => "infeasible".into(),
                s => format!("{s:?}"),
            }
        };
        let fmt = |v: &Option<(usize, Vec<f64>, Vec<f64>)>| v.as_ref().map_or("infeasible".to_string(), |(k, _, _)| k.to_string());
        let x = self.lp.col_values();
        let viol = |cs: &[Row]| cs.iter().filter(|(c, rhs)| c.iter().map(|&(j, a)| a * x[j]).sum::<f64>() > rhs + 1e-6 * (1.0 + rhs.abs())).count();
        eprintln!(
            "PROPCUT restarts {} cutoff {:.10e} free int {free0} rc lists {} | fixed: ab {} +a(rounds) {} +b(rc MIR) {} +c(row MIR) {} all {} | rows b {} ({} violated) c {} ({} violated) | LP root {:.10e} ab {} all {} ({:.2}s)",
            self.params.restarts,
            lim,
            lists.len(),
            fmt(&ab),
            fmt(&fa),
            fmt(&fb),
            fmt(&fc),
            fmt(&fall),
            cb.len(),
            viol(&cb),
            cc.len(),
            viol(&cc),
            self.lp.objective() + p.offset,
            lpv(&ab),
            lpv(&fall),
            t0.elapsed().as_secs_f64()
        );
    }
}
