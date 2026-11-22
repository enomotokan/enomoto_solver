//! 試作 `ENOMOTO_MIP_LPBOX` (既定では使わない、analysis/lpbox_20261122、利用者の案): 根のカットのループの後の LP 解 x̄ のまわりに
//! 整数列を狭め (一般整数列は [⌊x̄⌋−k+1, ⌈x̄⌉+k−1]、0-1 列は LP で整数値のものを固定、連続列は元のまま)、定義域の伝播で締めた
//! 問題で FJ と Local-MIP を回す主ヒューリスティクス。「最適解は LP 解から整数列の距離 k 以内」という仮定は保証されないので、
//! 箱は解探しにだけ使い、下界・枝刈りには使わない (見つかった解は元の問題で実行可能なので暫定解に試すだけ)。
//!
//! 文字 (値 `1` は既定の形 = 文字なし):
//! - `n`: 0-1 列を固定しない (一般整数列の箱だけ)
//! - `s`: 並列の FJ のスレッド (根の FJ・Local-MIP、[`super::heuristics::fj_lmip_mix`]) の時間の上限を半分にする (全体の問題と箱の問題で
//!   予算を分ける形。FJ のスレッドはふつう根のカットのループの終わりで止められるので、上限が効くのは長い根だけ)
//! - `l`: FJ を回さず Local-MIP だけ
//! - `t`: 予算を手間で数える (決定的): 解探しの間だけ主のスレッドの時計を手間 ([`crate::ticks::set_vclock`]、`ENOMOTO_MIP_TICKS` の `j` と
//!   同じ物差し、手間 × 1e-9 を秒とみなす) にし、根までの秒の頭打ちの代わりに、ここまでの主のスレッドの手間で頭打ちにする。残り時間の頭打ちはしない
//!
//! - `d`: 箱の定義域 (固定と狭めた境界) を LP に置いて LP を解き直し (x̄ が箱の中なら少ない反復で x̄ に戻る。丸めで x̄ が切られた
//!   ときだけ動く)、その LP 解から根の丸め (simple・randomized) と根のダイブ (分数・係数・ベクトル長、暫定解があれば誘導) を箱の中で回す。
//!   各ダイブの LP 反復の予算は根のダイブと同じ形 (`ENOMOTO_T_MIP_LPBOX_DIVE_Q` (0.05) × ここまでの LP 反復 + 1000)。箱の LP が実行不能
//!   なら記録だけ (k を広げる合図)。その後、FJ・Local-MIP も今までどおり回す。暫定解の出どころはラベル `LPBOX <名前>` で数える
//!
//! 調整: `ENOMOTO_T_MIP_LPBOX_K` (3) 箱の幅、`ENOMOTO_T_MIP_LPBOX_TRIES` (3) 伝播で実行不能のときのやり直しの回数 (k を倍、0-1 の固定を
//! 半分 → なし)、`ENOMOTO_T_MIP_LPBOX_SHARE` (0.5) 予算 (FJ のスレッドの Local-MIP の上限 = `ENOMOTO_T_MIP_LOCALMIP_TIME_FRAC` × 時間上限、
//! 10 秒まで) に掛ける割合、`ENOMOTO_T_MIP_LPBOX_FJ_FRAC` (0.3) そのうち FJ に回す割合、`ENOMOTO_T_MIP_LPBOX_STALL` (0.5) Local-MIP を
//! 予算のこの割合の間改善がなければ打ち切る。時計は今の FJ・Local-MIP と同じ (壁時計。`ENOMOTO_MIP_TICKS` の `j` なら手間)。

use std::time::{Duration, Instant};

use super::domain::FEASTOL;
use super::heuristics::DiveKind;
use super::lp::{LpStatus, SolveLimits};
use super::lp_api::MipLp;
use super::solver::Solver;

/// `ENOMOTO_MIP_LPBOX` が付いているか
pub(crate) fn enabled() -> bool {
    env_str!("ENOMOTO_MIP_LPBOX").is_some_and(|v| v != "0")
}

/// `ENOMOTO_MIP_LPBOX` の文字 `c` が入っているか
pub(crate) fn on(c: char) -> bool {
    env_str!("ENOMOTO_MIP_LPBOX").is_some_and(|v| v != "0" && v.contains(c))
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 根のカットのループの後に 1 回呼ぶ。暫定解が良くなったら真
    pub(super) fn lpbox_heur(&mut self) -> bool {
        let p = self.p;
        let t_all = Instant::now();
        let xl = self.lp.col_values();
        if xl.len() < p.n {
            return false;
        }
        let xbar: Vec<f64> = xl[..p.n].to_vec();
        let k0 = tunable!("ENOMOTO_T_MIP_LPBOX_K", 3.0, f64).max(1.0).round();
        let tries = tunable!("ENOMOTO_T_MIP_LPBOX_TRIES", 3usize, usize).max(1);
        let fix_bin = !on('n');
        let binv: Vec<bool> = (0..p.n).map(|j| p.is_int[j] && self.dom.global_lo[j] >= 0.0 && self.dom.global_up[j] <= 1.0).collect();
        let is_bin = |j: usize| binv[j];
        let nint = (0..p.n).filter(|&j| p.is_int[j] && self.dom.lo[j] < self.dom.up[j] && !is_bin(j)).count();
        let nbin_int = (0..p.n).filter(|&j| is_bin(j) && self.dom.lo[j] < self.dom.up[j] && (xbar[j] - xbar[j].round()).abs() <= FEASTOL).count();
        // 箱を作って伝播する。実行不能なら k を倍、0-1 の固定を半分 → なしにしてやり直す
        let mut k = k0;
        let mut keep = if fix_bin { 1.0f64 } else { 0.0 };
        let mut boxed: Option<(Vec<f64>, Vec<f64>, usize, usize, usize)> = None;
        let mut used_tries = 0usize;
        // `d`: 箱を定義域に置いたまま (この位置まで戻す)
        let mut keep_pos: Option<usize> = None;
        for t in 0..tries {
            used_tries = t + 1;
            let pos = self.dom.stack_len();
            let (mut nfix, mut nbox) = (0usize, 0usize);
            let mut nb = 0usize;
            for j in 0..p.n {
                if !p.is_int[j] || self.dom.lo[j] >= self.dom.up[j] || self.dom.infeasible {
                    continue;
                }
                let v = xbar[j];
                if is_bin(j) {
                    if (v - v.round()).abs() > FEASTOL {
                        continue;
                    }
                    // 半分のときは LP で整数値の 0-1 列を 1 つおきに固定する
                    let take = if keep >= 1.0 { true } else if keep > 0.0 { nb % 2 == 0 } else { false };
                    nb += 1;
                    if take {
                        let r = v.round();
                        let a = self.dom.tighten_lower(p, j, r);
                        let b = self.dom.tighten_upper(p, j, r);
                        if a || b {
                            nfix += 1;
                        }
                    }
                } else {
                    let a = self.dom.tighten_lower(p, j, (v + FEASTOL).floor() - k + 1.0);
                    let b = self.dom.tighten_upper(p, j, (v - FEASTOL).ceil() + k - 1.0);
                    if a || b {
                        nbox += 1;
                    }
                }
            }
            let before: Vec<(f64, f64)> = (0..p.n).map(|j| (self.dom.lo[j], self.dom.up[j])).collect();
            let ok = !self.dom.infeasible && self.dom.propagate(p);
            if ok {
                let nprop = (0..p.n).filter(|&j| self.dom.lo[j] > before[j].0 || self.dom.up[j] < before[j].1).count();
                boxed = Some((self.dom.lo.clone(), self.dom.up.clone(), nfix, nbox, nprop));
            }
            if ok && on('d') {
                keep_pos = Some(pos);
                break;
            }
            self.dom.backtrack_to(p, pos);
            if ok {
                break;
            }
            if self.params.verbose {
                eprintln!("MIP: LPBOX: try {} infeasible after the propagation (k={k}, fixed {nfix} binaries, boxed {nbox} integers)", t + 1);
            }
            k *= 2.0;
            keep = if keep >= 1.0 { 0.5 } else { 0.0 };
        }
        let Some((lo, up, nfix, nbox, nprop)) = boxed else {
            if self.params.verbose {
                eprintln!("MIP: LPBOX: k={k}, fixed -, propagated -, found none, obj - (infeasible box after {used_tries} tries, {:.2}s)", self.start.elapsed().as_secs_f64());
            }
            return false;
        };
        // `d`: 箱の LP から根の丸めとダイブ
        let mut dive_note = String::new();
        if let Some(pos) = keep_pos {
            dive_note = self.lpbox_dives(&xbar, pos);
        }
        // 予算: FJ のスレッドの Local-MIP の上限 × 割合、根までの秒と残り時間で頭打ち
        let t_now = self.start.elapsed().as_secs_f64();
        let ftl = self.params.frac_tl();
        let full = if ftl.is_finite() { (tunable!("ENOMOTO_T_MIP_LOCALMIP_TIME_FRAC", 0.1, f64) * ftl).min(10.0) } else { 10.0 };
        let mut cap = tunable!("ENOMOTO_T_MIP_LPBOX_SHARE", 0.5, f64) * full;
        let tk = on('t');
        if tk {
            cap = cap.min((crate::ticks::total() * 1e-9).max(0.1));
        } else {
            cap = cap.min(t_now.max(0.1));
            if let Some(d) = self.deadline {
                cap = cap.min(d.saturating_duration_since(Instant::now()).as_secs_f64());
            }
        }
        if cap <= 0.0 {
            return false;
        }
        let seed = self.rng ^ 0xB5AD_4ECE_DA1C_E2A9;
        let stop = std::sync::atomic::AtomicBool::new(false);
        let mut sols: Vec<(Vec<f64>, &'static str)> = Vec::new();
        let t0 = Instant::now();
        // `t`: 時計を手間に (終わったら戻す)
        let vprev = crate::ticks::vclock();
        crate::ticks::set_vclock(tk || vprev);
        let c0 = crate::ticks::clock_secs(t0);
        let el = || crate::ticks::clock_secs(t0) - c0;
        // FJ: LP 解を丸めた点から
        let mut t_fj = 0.0;
        let mut fj_found = false;
        if !on('l') {
            let c = tunable!("ENOMOTO_T_MIP_LPBOX_FJ_FRAC", 0.3, f64).clamp(0.0, 1.0) * cap;
            if c > 0.0 {
                if let Some(x) = super::fj::fj2_search_from(p, &lo, &up, Some(&xbar), c, seed, Some(&stop)) {
                    fj_found = true;
                    sols.push((x, "LPBOX FJ"));
                }
            }
            t_fj = el();
        }
        // Local-MIP: LP 解を丸めた点から (FJ の解があれば受け取って磨く)
        let mut lm_imp = 0usize;
        let mut viol0 = usize::MAX;
        if let Some(mut lm) = super::localmip::LocalMip::new(p, &lo, &up, seed ^ 0x6A09_E667_F3BC_C909, true) {
            viol0 = lm.set_start(&xbar);
            if let Some((x, _)) = sols.last() {
                lm.inject(x);
            }
            let stall = tunable!("ENOMOTO_T_MIP_LPBOX_STALL", 0.5, f64) * cap;
            let mut t_imp = el();
            while !lm.done {
                let now = el();
                if now >= cap || now - t_imp >= stall.max(0.2) {
                    break;
                }
                let before = lm.best_obj();
                lm.run(Instant::now() + Duration::from_secs_f64((cap - now).min(0.2)), Some(&stop), &|_| {});
                if lm.best_obj() < before {
                    if let Some(x) = lm.res.best.clone() {
                        sols.push((x, "LPBOX Local-MIP"));
                        lm_imp += 1;
                        t_imp = el();
                    }
                }
            }
        }
        let secs = el();
        crate::ticks::set_vclock(vprev);
        let inc0 = self.incumbent.as_ref().map(|(z, _)| *z);
        let best = sols.last().map(|(x, _)| p.objective(x));
        let n = sols.len();
        let mut acc = false;
        // 良い順 (後ろほど良い) に暫定解に試す
        for (x, lab) in sols.into_iter().rev() {
            self.heur_label = lab;
            let ok = self.try_incumbent(x);
            self.heur_label = "";
            if ok {
                acc = true;
                break;
            }
        }
        if self.params.verbose {
            eprintln!(
                "MIP: LPBOX: {dive_note}k={k}, fixed {nfix} of {nbin_int} LP-integral binaries, boxed {nbox} of {nint} general integers, propagated {nprop}, found {n} (FJ {fj_found} {t_fj:.2}s, Local-MIP {lm_imp} improvements, start violated {viol0}), obj {}, accepted {acc}, incumbent {inc0:?} -> {:?} (cap {cap:.2}s, used {secs:.2}s, total {:.2}s, at {:.2}s)",
                best.map_or("none".to_string(), |z| format!("{z:.10e}")),
                self.incumbent.as_ref().map(|(z, _)| *z),
                t_all.elapsed().as_secs_f64(),
                self.start.elapsed().as_secs_f64()
            );
        }
        acc
    }

    /// `d`: 定義域に箱が置かれた状態 (位置 `pos` から上が箱) で呼ぶ。LP を箱で解き直し、丸めとダイブを回してから、定義域と LP を
    /// 箱の前に戻す。ログに足す文字列を返す
    fn lpbox_dives(&mut self, xbar: &[f64], pos: usize) -> String {
        let p = self.p;
        let saved = self.lp.save_state();
        self.sync_lp();
        let inside = (0..p.n).all(|j| xbar[j] >= self.dom.lo[j] - FEASTOL && xbar[j] <= self.dom.up[j] + FEASTOL);
        let it0 = self.lp.total_iterations();
        let q = tunable!("ENOMOTO_T_MIP_LPBOX_DIVE_Q", 0.05, f64);
        let budget = (q * it0 as f64) as u64 + 1000;
        let st = self.lp.solve(&SolveLimits { iteration_limit: budget, cutoff: f64::INFINITY, deadline: self.deadline });
        let lp_iters = self.lp.total_iterations() - it0;
        let mut out: Vec<String> = Vec::new();
        let mut found_any = 0usize;
        if st == LpStatus::Optimal {
            let x = self.lp.col_values();
            if self.fractional(&x).is_empty() {
                self.heur_label = "LPBOX LP";
                if self.try_lp_solution() {
                    found_any += 1;
                    out.push("LP integral".to_string());
                }
                self.heur_label = "";
            } else {
                let has_inc = self.incumbent.is_some();
                let names: [(&'static str, u8); 6] =
                    [("LPBOX simple rounding", 0), ("LPBOX randomized rounding", 1), ("LPBOX fractional diving", 2), ("LPBOX coefficient diving", 3), ("LPBOX vector length diving", 4), ("LPBOX guided diving", 5)];
                for (name, kind) in names {
                    if self.time_up() || (kind == 5 && !has_inc && self.incumbent.is_none()) {
                        continue;
                    }
                    let z0 = self.incumbent.as_ref().map(|(z, _)| *z);
                    let ti = self.lp.total_iterations();
                    self.heur_label = name;
                    let _ = match kind {
                        0 => self.simple_rounding(&x),
                        1 => self.randomized_rounding(&x, 3),
                        2 => self.fractional_dive(budget),
                        3 => self.dive(DiveKind::Coefficient, budget, f64::INFINITY),
                        4 => self.dive(DiveKind::VectorLength, budget, f64::INFINITY),
                        _ => self.dive(DiveKind::Guided, budget, f64::INFINITY),
                    };
                    self.heur_label = "";
                    let z1 = self.incumbent.as_ref().map(|(z, _)| *z);
                    let imp = match (z0, z1) {
                        (None, Some(_)) => true,
                        (Some(a), Some(b)) => b < a,
                        _ => false,
                    };
                    if imp {
                        found_any += 1;
                    }
                    out.push(format!("{} {}{}", &name[6..], self.lp.total_iterations() - ti, if imp { "*" } else { "" }));
                }
            }
        }
        self.dom.backtrack_to(p, pos);
        self.sync_lp();
        self.lp.restore_state(&saved);
        format!(
            "box LP {st:?} (xbar inside {inside}, {lp_iters} iters, dive budget {budget}), {} improved [{}]; ",
            found_any,
            out.join(", ")
        )
    }
}
