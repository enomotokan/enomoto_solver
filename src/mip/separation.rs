//! 根での切除平面のループ (HiGHS の `HighsSeparation` / `HighsTableauSeparator` を簡略化したもの)。
//!
//! 各ラウンドで、元の行 1 本ずつと、基底にある小数の整数変数の tableau 行 (基底逆行列の行で LP 行を
//! 集約したもの、Gomory 相当) を CMIR に通して切除平面を作り、効き目の大きい順に、ほぼ平行なものを
//! 除いて LP に加えて解き直す。目的値がほとんど動かなくなったら止める。最後に効いていない (論理変数が
//! 基底にある) カット行を LP から外す。カットは大域的な境界から作るので、木全体で有効。

use super::cuts::{cmir, extended_cover, CutVars, RawCut};
use super::domain::FEASTOL;
use super::lp::{LpStatus, SolveLimits, VarStatus};
use super::solver::Solver;
use super::lp_api::MipLp;

/// LP に加える前のカット (構造変数の係数、右辺、効き目)。
struct Candidate {
    coefs: Vec<(usize, f64)>,
    rhs: f64,
    efficacy: f64,
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 根の切除平面ループ。LP は根の最適解の状態で呼ぶこと。終わったときも LP は最適 (でなければ偽)。
    pub(super) fn root_cut_loop(&mut self, root_iters: u64) -> bool {
        let p = self.p;
        let n = p.n;
        let mut stall = 0;
        let mut prev_obj = self.lp.objective();
        let first_obj = prev_obj;
        let max_rounds = 25;
        let time_cap = if self.params.time_limit.is_finite() { 0.2 * self.params.time_limit } else { f64::INFINITY };
        let mut total_added = 0usize;
        for round in 0..max_rounds {
            if self.time_up() || self.start.elapsed().as_secs_f64() > time_cap {
                break;
            }
            let x = self.lp.col_values();
            if self.fractional(&x).is_empty() {
                break;
            }
            let cands = self.separate(&x);
            if cands.is_empty() {
                break;
            }
            // 効き目の大きい順に、平行なものを除いて選ぶ
            let max_cuts = (p.m.max(50)).min(500);
            let chosen = select_cuts(cands, max_cuts);
            if chosen.is_empty() {
                break;
            }
            let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = chosen.into_iter().map(|c| (c.coefs, f64::NEG_INFINITY, c.rhs)).collect();
            total_added += rows.len();
            let saved_rows = self.lp.num_rows();
            self.lp.add_rows(&rows);
            let it0 = self.lp.total_iterations();
            let lim = 10 * root_iters.max(100) + 10_000;
            let st = self.lp.solve(&SolveLimits { iteration_limit: lim, cutoff: f64::INFINITY, deadline: self.deadline });
            self.sb_iters += 0;
            let _ = it0;
            if st != LpStatus::Optimal {
                // 数値的に困ったら今回のカットを外して戻す
                let mut remove = vec![false; self.lp.num_rows()];
                for r in remove.iter_mut().skip(saved_rows) {
                    *r = true;
                }
                self.lp.delete_rows(&remove);
                let st2 = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
                if self.params.verbose {
                    eprintln!("MIP: cut round {round}: LP status {st:?}, removed the cuts ({st2:?})");
                }
                return st2 == LpStatus::Optimal;
            }
            let obj = self.lp.objective();
            if self.params.verbose {
                eprintln!(
                    "MIP: cut round {round}: {} cuts, LP rows {}, obj {:.10e} ({:.2}s)",
                    rows.len(),
                    self.lp.num_rows(),
                    obj + p.offset,
                    self.start.elapsed().as_secs_f64()
                );
            }
            // 停滞判定: 改善が初回からの改善量のわずかな割合なら停滞
            let gain = obj - prev_obj;
            let scale = (obj - first_obj).abs().max(1e-6 * obj.abs().max(1.0));
            if gain <= 1e-3 * scale || gain <= 1e-9 * obj.abs().max(1.0) {
                stall += 1;
                if stall >= 3 {
                    break;
                }
            } else {
                stall = 0;
            }
            prev_obj = obj;
            // 効いていないカットを外す (論理変数が基底にあり、行が緩んでいるもの)
            self.remove_inactive_cuts();
        }
        self.remove_inactive_cuts();
        let st = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
        if self.params.verbose {
            eprintln!(
                "MIP: cut loop done: {} cuts added, {} in LP, obj {:.10e} -> {:.10e}",
                total_added,
                self.lp.num_rows() - p.m,
                first_obj + p.offset,
                self.lp.objective() + p.offset
            );
        }
        st == LpStatus::Optimal
    }

    /// ノードでの分離 (1 ラウンド)。カットを加えたら LP を解き直し、真を返す。LP の行数の上限を超えたら何もしない。
    pub(super) fn node_cut_round(&mut self, x: &[f64]) -> bool {
        let p = self.p;
        let max_rows = p.m + (2 * p.m).max(500);
        if self.lp.num_rows() >= max_rows {
            return false;
        }
        let cands = self.separate(x);
        if cands.is_empty() {
            return false;
        }
        let room = max_rows - self.lp.num_rows();
        let chosen = select_cuts(cands, room.min(50));
        if chosen.is_empty() {
            return false;
        }
        let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = chosen.into_iter().map(|c| (c.coefs, f64::NEG_INFINITY, c.rhs)).collect();
        self.lp.add_rows(&rows);
        true
    }

    /// LP の行のうち、元の行より後ろ (カット) で論理変数が基底にあるものを外す。
    pub(super) fn remove_inactive_cuts(&mut self) {
        let m0 = self.p.m;
        let mr = self.lp.num_rows();
        if mr <= m0 {
            return;
        }
        let b = self.lp.basis();
        let act = self.lp.row_activities();
        let mut remove = vec![false; mr];
        let mut any = false;
        for i in m0..mr {
            if b.row[i] == VarStatus::Basic {
                let (_, up) = self.lp.row_bounds(i);
                if act[i] < up - 1e-6 * (1.0 + up.abs()) {
                    remove[i] = true;
                    any = true;
                }
            }
        }
        if any {
            self.lp.delete_rows(&remove);
        }
    }

    /// 現在の LP 解 `x` を切る候補を作る。
    fn separate(&mut self, x: &[f64]) -> Vec<Candidate> {
        let p = self.p;
        let n = p.n;
        let mr = self.lp.num_rows();
        // 変数: 構造変数 0..n と、LP 行の活動量 n..n+mr
        let lp_rows: Vec<Vec<(usize, f64)>> = (0..mr).map(|i| self.lp.row(i)).collect();
        let act = self.lp.row_activities();
        // カットは木全体で有効にするため、大域的な境界から作る
        let mut lo = self.dom.global_lo.clone();
        let mut up = self.dom.global_up.clone();
        let mut is_int = p.is_int.clone();
        let mut xv = x.to_vec();
        for i in 0..mr {
            let (l, u) = self.lp.row_bounds(i);
            let integral = lp_rows[i].iter().all(|&(j, a)| p.is_int[j] && (a - a.round()).abs() <= 1e-9);
            let (l, u) = if integral { ((l - FEASTOL).ceil(), (u + FEASTOL).floor()) } else { (l, u) };
            lo.push(l);
            up.push(u);
            is_int.push(integral);
            xv.push(act[i]);
        }
        let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &xv };
        let mut cands: Vec<Candidate> = Vec::new();
        let mut push = |raw: Option<RawCut>, cands: &mut Vec<Candidate>, s: &Solver<L>| {
            if let Some(raw) = raw {
                if let Some(c) = finish_cut(raw, n, &lp_rows, &s.dom.global_lo, &s.dom.global_up, x, s.incumbent.as_ref().map(|(_, v)| v.as_slice())) {
                    cands.push(c);
                }
            }
        };
        // 元の行 1 本ずつ
        for i in 0..p.m {
            let row = &p.rows[i];
            if row.len() < 2 {
                continue;
            }
            if p.row_up[i].is_finite() {
                let r = cmir(&vars, row, p.row_up[i]);
                push(r, &mut cands, self);
                let r = extended_cover(&vars, row, p.row_up[i]);
                push(r, &mut cands, self);
            }
            if p.row_lo[i].is_finite() {
                let neg: Vec<(usize, f64)> = row.iter().map(|&(j, a)| (j, -a)).collect();
                let r = cmir(&vars, &neg, -p.row_lo[i]);
                push(r, &mut cands, self);
                let r = extended_cover(&vars, &neg, -p.row_lo[i]);
                push(r, &mut cands, self);
            }
        }
        // tableau 行
        let mut basics: Vec<(usize, f64)> = Vec::new();
        for s in 0..mr {
            let k = self.lp.basic_var(s);
            if k < n && p.is_int[k] {
                let v = x[k];
                let f = v - v.floor();
                if f > 1e-3 && f < 1.0 - 1e-3 {
                    basics.push((s, f * (1.0 - f) / self.lp.dse_weight(s).max(1e-8)));
                }
            }
        }
        basics.sort_by(|a, b| b.1.total_cmp(&a.1));
        let nint = p.is_int.iter().filter(|&&b| b).count();
        let limit = 200 + (0.1 * (mr.min(nint)) as f64) as usize;
        basics.truncate(limit);
        let mut agg = vec![0.0; n];
        for &(s, _) in &basics {
            let w = self.lp.basis_inverse_row(s);
            let wmax = w.iter().fold(0.0f64, |m, v| m.max(v.abs()));
            if wmax == 0.0 {
                continue;
            }
            let wmin_keep = 1e-9 * wmax;
            // 集約: sum_i w_i (a_i x - r_i) = 0
            let mut touched: Vec<usize> = Vec::new();
            let mut base: Vec<(usize, f64)> = Vec::new();
            let mut ok = true;
            let mut wnz_min = f64::INFINITY;
            for i in 0..mr {
                let wi = w[i];
                if wi.abs() <= wmin_keep {
                    continue;
                }
                wnz_min = wnz_min.min(wi.abs());
                for &(j, a) in &lp_rows[i] {
                    if agg[j] == 0.0 {
                        touched.push(j);
                    }
                    agg[j] += wi * a;
                    if agg[j] == 0.0 {
                        agg[j] = 1e-300;
                    }
                }
                base.push((n + i, -wi));
            }
            // 重みの比が大きすぎる集約は数値的に危ないので使わない
            if wmax / wnz_min > 1e6 {
                ok = false;
            }
            for &j in &touched {
                let v = agg[j];
                if v.abs() > 1e-11 && ok {
                    base.push((j, v));
                }
                agg[j] = 0.0;
            }
            if !ok {
                continue;
            }
            let r = cmir(&vars, &base, 0.0);
            push(r, &mut cands, self);
            let neg: Vec<(usize, f64)> = base.iter().map(|&(k, a)| (k, -a)).collect();
            let r = cmir(&vars, &neg, 0.0);
            push(r, &mut cands, self);
        }
        cands
    }
}

/// 生のカット (構造変数と行の活動量の変数) を構造変数だけの式にし、整え、検査する。
fn finish_cut(
    raw: RawCut,
    n: usize,
    lp_rows: &[Vec<(usize, f64)>],
    lo: &[f64],
    up: &[f64],
    x: &[f64],
    incumbent: Option<&[f64]>,
) -> Option<Candidate> {
    let mut dense: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
    for &(k, c) in &raw.coefs {
        if k < n {
            *dense.entry(k).or_insert(0.0) += c;
        } else {
            for &(j, a) in &lp_rows[k - n] {
                *dense.entry(j).or_insert(0.0) += c * a;
            }
        }
    }
    let mut rhs = raw.rhs;
    let cmax = dense.values().fold(0.0f64, |m, v| m.max(v.abs()));
    if cmax <= 0.0 || !rhs.is_finite() {
        return None;
    }
    // 小さい係数は境界で吸収して消す
    let mut coefs: Vec<(usize, f64)> = Vec::with_capacity(dense.len());
    for (j, c) in dense {
        if c.abs() <= 1e-9 * cmax {
            // c x_j >= min(c lo, c up) を使って右辺を緩める
            let m = if c > 0.0 { c * lo[j] } else { c * up[j] };
            if !m.is_finite() {
                return None;
            }
            rhs -= m;
        } else {
            coefs.push((j, c));
        }
    }
    if coefs.is_empty() {
        return None;
    }
    let cmin = coefs.iter().fold(f64::INFINITY, |m, &(_, c)| m.min(c.abs()));
    if cmax / cmin > 1e6 {
        return None;
    }
    // 最大係数 1 にスケール
    let s = 1.0 / cmax;
    for c in coefs.iter_mut() {
        c.1 *= s;
    }
    rhs *= s;
    let act: f64 = coefs.iter().map(|&(j, c)| c * x[j]).sum();
    let norm: f64 = coefs.iter().map(|&(_, c)| c * c).sum::<f64>().sqrt();
    let viol = act - rhs;
    if viol <= 1e-6 * (1.0 + rhs.abs()) {
        return None;
    }
    let efficacy = viol / norm;
    if efficacy < 1e-5 {
        return None;
    }
    // 安全確認: 暫定解を切るカットは (生成の誤りなので) 捨てる
    if let Some(inc) = incumbent {
        let a: f64 = coefs.iter().map(|&(j, c)| c * inc[j]).sum();
        if a > rhs + 1e-6 * (1.0 + rhs.abs()) {
            if env_str!("ENOMOTO_MIP_LOG").is_some() {
                eprintln!("MIP: warning: a generated cut cuts off the incumbent (rejected)");
            }
            return None;
        }
    }
    Some(Candidate { coefs, rhs, efficacy })
}

/// 効き目の大きい順に、既に選んだものとほぼ平行 (|cos| > 0.99) なものを除いて選ぶ。
fn select_cuts(mut cands: Vec<Candidate>, max_cuts: usize) -> Vec<Candidate> {
    cands.sort_by(|a, b| b.efficacy.total_cmp(&a.efficacy));
    let mut chosen: Vec<Candidate> = Vec::new();
    let mut norms: Vec<f64> = Vec::new();
    for c in cands {
        if chosen.len() >= max_cuts {
            break;
        }
        let nc: f64 = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
        let mut parallel = false;
        for (d, &nd) in chosen.iter().zip(&norms) {
            // 疎な内積 (両方とも列番号の昇順)
            let (mut i, mut k, mut dot) = (0, 0, 0.0);
            while i < c.coefs.len() && k < d.coefs.len() {
                let (a, b) = (c.coefs[i].0, d.coefs[k].0);
                if a == b {
                    dot += c.coefs[i].1 * d.coefs[k].1;
                    i += 1;
                    k += 1;
                } else if a < b {
                    i += 1;
                } else {
                    k += 1;
                }
            }
            if dot.abs() > 0.99 * nc * nd {
                parallel = true;
                break;
            }
        }
        if !parallel {
            norms.push(nc);
            chosen.push(c);
        }
    }
    chosen
}
