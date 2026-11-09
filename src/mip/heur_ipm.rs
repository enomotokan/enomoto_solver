//! 内点法の解から整数解を作るヒューリスティクス (HiGHS の central rounding に、クロスオーバーの頂点の丸めを加えたもの)。
//!
//! 根の LP (カット込み) を内点法で解いた点 `x_c` は、最適面 (費用 0 なら実行可能領域) の中心付近にある。
//! 単体法の頂点 `x_lp` は境界に張り付いていて丸めると実行不能になりやすいが、中心に寄せた点は行に余裕があるので
//! 丸めが通りやすい。次の点を丸め、整数列を (整数に近い順に) 固定・伝播し、連続部分を LP で解いて試す
//! ([`Solver::propagate_rounding_from`]):
//!
//! 1. 直線 `x(α) = x_lp + α (x_c - x_lp)` 上の点 (`α = 1, 0.8, …, 0.2`。丸めた整数列が同じ点は 1 回だけ)。
//! 2. `x_c` からクロスオーバーの押し出し (主の押し出し・基底の選択・Megiddo 式の押し出し) だけで作った頂点。
//!    この頂点の連続部分の LP は、押し出しで得た基底から始める。この基底は主実行可能な頂点を与えるが
//!    **双対実行可能とは限らない** (仕上げの単体法を走らせていない) ので、LP の最適基底としては扱わない
//!    (下界・被約費用に使わない)。warm start にだけ使い、二段解法の warm start が双対実行不能な列を
//!    被約費用の符号の側 (無限なら記号的な M) に置き直すか費用をずらして始め、真の費用で仕上げる。

use super::lp_api::MipLp;
use super::solver::Solver;
use std::time::{Duration, Instant};

impl<'a, L: MipLp> Solver<'a, L> {
    /// 内点法の解からの丸め。`xlp` は根の LP の (単体法の) 解。
    pub(super) fn interior_rounding(&mut self, xlp: &[f64]) -> bool {
        let p = self.p;
        if self.params.submip || !p.is_int.iter().any(|&b| b) {
            return false;
        }
        // 時間の上限: 全体の 5% (時間制限がなければ 10 秒)、残り時間の半分まで
        let frac = tunable!("ENOMOTO_T_MIP_IPM_TIME_FRAC", 0.05, f64);
        let mut budget = if self.params.time_limit.is_finite() { frac * self.params.time_limit } else { 10.0 };
        if let Some(d) = self.deadline {
            budget = budget.min(0.5 * d.saturating_duration_since(Instant::now()).as_secs_f64());
        }
        if budget <= 0.01 {
            return false;
        }
        // `ENOMOTO_MIP_IPM_HEUR_LPGATE` (試作、既定では使わない): 根の LP (単体法) に `ENOMOTO_T_MIP_IPM_HEUR_LPGATE_MULT` (既定 1) × 予算
        // より長くかかった問題では内点法を始めない。内点法の準備 (KKT の AMD・METIS の順序付け、多前線法の記号分解・最初の分解) は
        // 止めの合図を見ないので、大きい問題では予算を大きく超える: ns1116954 (13 万行) の 1 スレッドで予算 0.85 秒に対し 11〜16 秒かかり、
        // 時間の上限を 10〜11 秒超えた (AMD と METIS は `rayon::join` で並ぶので 1 スレッドで最も長い。
        // analysis/mittelmann_ready_20261103/threads.md)
        if env_str!("ENOMOTO_MIP_IPM_HEUR_LPGATE").is_some() && self.root_lp_secs > tunable!("ENOMOTO_T_MIP_IPM_HEUR_LPGATE_MULT", 1.0, f64) * budget {
            if self.params.verbose {
                eprintln!("MIP:   interior rounding: skipped (root LP {:.2}s > budget {budget:.2}s)", self.root_lp_secs);
            }
            return false;
        }
        let deadline = Instant::now() + Duration::from_secs_f64(budget);
        // 0: 費用つき (最適面の中心付近) + クロスオーバーの頂点、1: 費用 0 (実行可能領域の中心付近、頂点なし)、2: 両方
        let mode = tunable!("ENOMOTO_T_MIP_IPM_MODE", 0u8, u8);
        let mut centers: Vec<(&str, Vec<f64>)> = Vec::new();
        let mut vertex: Option<(Vec<usize>, Vec<f64>)> = None;
        let t0 = Instant::now();
        if mode != 1 {
            match self.lp.interior_point(false, env_str!("ENOMOTO_MIP_IPM_NO_VERTEX").is_none(), deadline) {
                Some(ip) => {
                    centers.push(("optimal face", ip.x));
                    if let Some((b, Some(xv))) = ip.vertex {
                        vertex = Some((b, xv));
                    }
                }
                None => {
                    if self.params.verbose {
                        eprintln!("MIP:   interior rounding: IPM (cost) failed ({:.2}s)", t0.elapsed().as_secs_f64());
                    }
                }
            }
        }
        // 試作 `ENOMOTO_MIP_HO4_ACFIX` (analysis/highs_only_20261105/G4.md、既定では使わない): HiGHS の
        // `finishAnalyticCenterComputation` と同じく、費用 0 の根の LP (カット込み) の解析的中心で境界から
        // `ENOMOTO_T_MIP_HO4_ACFIX_TOL` (既定 1e-6) 以内にある整数列を、その境界に大域的に固定する (解析的中心は実行可能領域の
        // 相対的な内部の点なので、そこで境界にある列は LP の実行可能領域全体でその値。カットは整数解を切らないので整数解でも同じ)。
        // satellites2-60-fs では HiGHS が根で 11955 列をこれで固定し (40.3%)、再スタートで 12852 x 31721 -> 6047 x 12408 になる
        let acfix = !self.params.submip && self.snap.is_none() && env_str!("ENOMOTO_MIP_HO4_ACFIX").is_some();
        if (mode != 0 || acfix) && Instant::now() < deadline {
            if let Some(ip) = self.lp.interior_point(true, false, deadline) {
                if acfix {
                    let tol = tunable!("ENOMOTO_T_MIP_HO4_ACFIX_TOL", 1e-6, f64);
                    // 候補: 中心で大域の境界から tol 以内の整数列 (true = 下限側)
                    let mut cands: Vec<(usize, bool)> = Vec::new();
                    for j in 0..p.n {
                        if !p.is_int[j] || self.dom.global_lo[j] >= self.dom.global_up[j] {
                            continue;
                        }
                        let (lo, up) = (self.dom.global_lo[j], self.dom.global_up[j]);
                        if lo.is_finite() && ip.x[j] <= lo + tol {
                            cands.push((j, true));
                        } else if up.is_finite() && ip.x[j] >= up - tol {
                            cands.push((j, false));
                        }
                    }
                    let n_cand = cands.len();
                    // 確かめ: 内点法の点は解析的中心とは限らない (58 問の qnet1・nu25-pr12 では確かめなしの固定が最適解を切り、
                    // 誤った最適値を返した)。候補の境界からのずれの和を今の LP (カット込み) で最大化し、ずれた列を候補から外して
                    // やり直す (最大 `ENOMOTO_T_MIP_HO4_ACFIX_ROUNDS` 回)。和が 0 になった候補だけ固定する (LP の許容誤差の範囲で正しい)
                    let mut verified = false;
                    let mut rounds = 0usize;
                    if !cands.is_empty() {
                        let saved = self.lp.save_state();
                        let c0 = self.lp.costs();
                        let max_rounds = tunable!("ENOMOTO_T_MIP_HO4_ACFIX_ROUNDS", 3usize, usize);
                        // 確かめの LP の時間の上限: 内点法の丸めの上限か、今から `ENOMOTO_T_MIP_HO4_ACFIX_LP_MULT` (既定 2) x 根の LP の秒の
                        // 遅い方 (全体の期限まで)。satellites2-60-fs では 30 秒の丸めの上限に当たって確かめられなかった
                        let vd = deadline.max(Instant::now() + Duration::from_secs_f64((tunable!("ENOMOTO_T_MIP_HO4_ACFIX_LP_MULT", 2.0, f64) * self.root_lp_secs).min(1e6)));
                        let vd = self.deadline.map_or(vd, |g| g.min(vd));
                        while rounds < max_rounds && !cands.is_empty() && Instant::now() < vd {
                            rounds += 1;
                            let mut c = vec![0.0; p.n];
                            for &(j, at_lo) in &cands {
                                c[j] = if at_lo { -1.0 } else { 1.0 };
                            }
                            self.lp.set_costs(&c);
                            let st = self.lp.solve(&super::lp::SolveLimits { iteration_limit: u64::MAX, cutoff: f64::INFINITY, deadline: Some(vd) });
                            if st != super::lp::LpStatus::Optimal {
                                if self.params.verbose {
                                    eprintln!("MIP:   interior rounding: ENOMOTO_MIP_HO4_ACFIX: the check LP stopped ({st:?}, {:.2}s)", t0.elapsed().as_secs_f64());
                                }
                                cands.clear();
                                break;
                            }
                            let x = self.lp.col_values();
                            let before = cands.len();
                            let dev_tol = 1e-6;
                            cands.retain(|&(j, at_lo)| if at_lo { x[j] <= self.dom.global_lo[j] + dev_tol } else { x[j] >= self.dom.global_up[j] - dev_tol });
                            if cands.len() == before {
                                verified = true;
                                break;
                            }
                        }
                        self.lp.set_costs(&c0);
                        self.lp.restore_state(&saved);
                    }
                    let (mut nlo, mut nup) = (0usize, 0usize);
                    if verified {
                        for &(j, at_lo) in &cands {
                            if at_lo {
                                let v = self.dom.global_lo[j];
                                self.dom.tighten_global(p, j, true, v);
                                nlo += 1;
                            } else {
                                let v = self.dom.global_up[j];
                                self.dom.tighten_global(p, j, false, v);
                                nup += 1;
                            }
                        }
                    }
                    if self.params.verbose {
                        eprintln!(
                            "MIP:   interior rounding: ENOMOTO_MIP_HO4_ACFIX: {n_cand} candidates at the analytic center, verified {verified} after {rounds} LPs; fixed {nlo} integer columns at the lower bound and {nup} at the upper bound (newly fixed {}, {:.2}s)",
                            self.newly_fixed_ints(),
                            t0.elapsed().as_secs_f64()
                        );
                    }
                }
                if mode != 0 {
                    centers.push(("analytic center", ip.x));
                }
            }
        }
        if self.params.verbose {
            eprintln!("MIP:   interior rounding: {} interior points, vertex {} ({:.2}s)", centers.len(), vertex.is_some(), t0.elapsed().as_secs_f64());
        }
        // 丸めて試す点 (名前, 点, 連続部分の LP の開始基底)
        let mut seen: std::collections::HashSet<Vec<i64>> = std::collections::HashSet::new();
        let key = |x: &[f64]| -> Vec<i64> { (0..p.n).filter(|&j| p.is_int[j]).map(|j| x[j].round() as i64).collect() };
        let mut tried = 0usize;
        let try_point = |s: &mut Self, x: &[f64], start: Option<(&[usize], &[f64])>, seen: &mut std::collections::HashSet<Vec<i64>>, tried: &mut usize| -> bool {
            if !seen.insert(key(x)) {
                return false;
            }
            *tried += 1;
            // 整数に近い列から固定する
            let mut order: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j]).collect();
            order.sort_by(|&a, &b| (x[a] - x[a].round()).abs().total_cmp(&(x[b] - x[b].round()).abs()));
            s.propagate_rounding_from(x, &order, None, start)
        };
        let mut found = false;
        let alphas = [1.0, 0.8, 0.6, 0.4, 0.2];
        for (_, xc) in &centers {
            for &a in &alphas {
                if found || self.time_up() || Instant::now() >= deadline {
                    break;
                }
                let x: Vec<f64> = (0..p.n).map(|j| xlp[j] + a * (xc[j] - xlp[j])).collect();
                found |= try_point(self, &x, None, &mut seen, &mut tried);
            }
        }
        if let Some((b, xv)) = &vertex {
            if !found && !self.time_up() {
                found |= try_point(self, xv, Some((b.as_slice(), xv.as_slice())), &mut seen, &mut tried);
            }
        }
        if self.params.verbose {
            eprintln!("MIP:   interior rounding: {tried} rounded points tried, found {found}");
        }
        found
    }
}
