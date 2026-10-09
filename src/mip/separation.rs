//! 根での切除平面のループ (HiGHS の `HighsSeparation` / `HighsTableauSeparator` を簡略化したもの)。
//!
//! 各ラウンドで、元の行 1 本ずつと、基底にある小数の整数変数の tableau 行 (基底逆行列の行で LP 行を
//! 集約したもの、Gomory 相当) を CMIR に通して切除平面を作り、効き目の大きい順に、ほぼ平行なものを
//! 除いて LP に加えて解き直す。目的値がほとんど動かなくなったら止める。最後に効いていない (論理変数が
//! 基底にある) カット行を LP から外す。カットは大域的な境界から作るので、木全体で有効。

use super::cuts::{cmir, extended_cover, generate_cuts, lifted_cover, lifted_flow_cover, CutVars, RawCut, VarBounds};
use super::problem::MipProblem;
use super::domain::FEASTOL;
use super::lp::{LpStatus, SolveLimits, VarStatus};
use super::solver::Solver;
use super::lp_api::MipLp;

/// LP に加える前のカット (構造変数の係数、右辺、効き目)。
/// lifted flow cover を使うか (`ENOMOTO_MIP_NO_FLOWCOVER` で無効)。
/// HiGHS の lifted cover を CMIR と並べて試すか (`ENOMOTO_MIP_LIFTED_COVER` で有効)。qnet1 では根の下界が
/// 15627 -> 15924 (HiGHS 16022) と上がったが、40 問では 19 -> 17 問 (sgeomean 27.91 -> 29.06) と悪化した
/// (misc07・neos5・binkar10_1・neos-860300 が遅くなった) ので既定では使わない。
fn use_lifted_cover() -> bool {
    env_str!("ENOMOTO_MIP_LIFTED_COVER").is_some()
}

fn use_flow_cover() -> bool {
    env_str!("ENOMOTO_MIP_NO_FLOWCOVER").is_none()
}

/// 旧来の生成 (CMIR・flow cover・cover を別々に作って全て候補に入れる) に戻す (`ENOMOTO_MIP_GEN_OLD`)。
/// 既定は HiGHS と同じく、集約行 1 本 (の向き 1 つ) ごとに `generate_cuts` の候補から効き目の最も大きい 1 本だけ残す。
fn gen_old() -> bool {
    env_str!("ENOMOTO_MIP_GEN_OLD").is_some()
}

/// カットの質 (Wesselmann・Suhl "Implementing cutting plane management and selection techniques" (2012) で最も良かった
/// 重み付き和): 距離 (効き目) + 0.1 × 目的関数との平行度 + 0.1 × 整数台の割合 (カットの非零のうち整数列の割合)。
/// カット選択 (`select_cuts_hybrid`) と、集約行 1 本から作った候補のうち 1 本を残すとき (`keep_best`) に使う。
fn cut_quality(c: &Candidate, p: &MipProblem, cnorm: f64) -> f64 {
    let nc = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
    let objpar = if cnorm > 0.0 && nc > 0.0 { c.coefs.iter().map(|&(j, v)| v * p.cost[j]).sum::<f64>().abs() / (cnorm * nc) } else { 0.0 };
    let intsup = c.coefs.iter().filter(|&&(j, _)| p.is_int[j]).count() as f64 / c.coefs.len().max(1) as f64;
    c.efficacy + 0.1 * objpar + 0.1 * intsup
}

/// `cands[n0..]` のうち質 ([`cut_quality`]、`ENOMOTO_MIP_KEEP_BEST_EFF` なら効き目だけ) の最も大きい 1 本だけ残す。
fn keep_best(cands: &mut Vec<Candidate>, n0: usize, p: &MipProblem) {
    if cands.len() <= n0 + 1 {
        return;
    }
    let eff_only = env_str!("ENOMOTO_MIP_KEEP_BEST_EFF").is_some();
    let cnorm = p.cost.iter().map(|c| c * c).sum::<f64>().sqrt();
    let q = |c: &Candidate| if eff_only { c.efficacy } else { cut_quality(c, p, cnorm) };
    let mut bi = n0;
    let mut bq = q(&cands[n0]);
    for i in n0 + 1..cands.len() {
        let qi = q(&cands[i]);
        if qi > bq {
            bi = i;
            bq = qi;
        }
    }
    cands.swap(n0, bi);
    cands.truncate(n0 + 1);
}

struct Candidate {
    coefs: Vec<(usize, f64)>,
    rhs: f64,
    efficacy: f64,
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// HiGHS のカット選択 (`HighsCutPool::separate`): スコア = 違反量 / (効いている非零の数 x 効いている列だけのノルム)
    /// (効いている = LP 解が境界から離れている列)。これまでに見た最良のスコアの `min_factor` 倍以上を残し
    /// (残りが少なすぎれば上位半分、全部残れば係数を下げる)、採ったカットとの平行度が 0.1 を超えるものは捨てる。
    fn select_cuts_highs(&mut self, cands: Vec<Candidate>, max_cuts: usize, x: &[f64], maxpar: f64) -> Vec<Candidate> {
        let tol = FEASTOL;
        let mut sc: Vec<(f64, Candidate)> = Vec::new();
        for c in cands {
            let act: f64 = c.coefs.iter().map(|&(j, a)| a * x[j]).sum();
            let viol = act - c.rhs;
            if viol <= tol {
                continue;
            }
            let mut norm = 0.0;
            let mut nact = 0usize;
            for &(j, a) in &c.coefs {
                let active = if a > 0.0 { x[j] > self.dom.global_lo[j] + tol } else { x[j] < self.dom.global_up[j] - tol };
                if active {
                    norm += a * a;
                    nact += 1;
                }
            }
            if nact == 0 {
                continue;
            }
            sc.push((viol / (nact as f64 * norm.sqrt()), c));
        }
        if sc.is_empty() {
            return Vec::new();
        }
        sc.sort_by(|a, b| b.0.total_cmp(&a.0));
        self.cutsel_best = self.cutsel_best.max(sc[0].0);
        let min_score = self.cutsel_factor * self.cutsel_best;
        let mut keep = sc.partition_point(|s| s.0 >= min_score);
        let lower = sc.len() / 20;
        let upper = sc.len() - 1;
        if keep <= lower {
            keep = (sc.len() / 2).max(1);
            self.cutsel_factor = sc[keep - 1].0 / self.cutsel_best;
        } else if keep > upper {
            self.cutsel_factor = sc[upper].0 / self.cutsel_best;
        }
        sc.truncate(keep);
        let mut chosen: Vec<(f64, Candidate)> = Vec::new();
        for (_, c) in sc {
            if chosen.len() >= max_cuts {
                break;
            }
            let nc = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
            if chosen.iter().any(|(nd, d)| sparse_dot(&c.coefs, &d.coefs).abs() > maxpar * nc * nd) {
                continue;
            }
            chosen.push((nc, c));
        }
        chosen.into_iter().map(|(_, c)| c).collect()
    }

    /// 根の切除平面ループ。LP は根の最適解の状態で呼ぶこと。終わったときも LP は最適 (でなければ偽)。
    pub(super) fn root_cut_loop(&mut self, root_iters: u64) -> bool {
        let p = self.p;
        let n = p.n;
        let mut stall = 0;
        let mut prev_obj = self.lp.objective();
        let mut prev_nfrac = self.fractional(&self.lp.col_values()).len();
        let first_obj = prev_obj;
        // HiGHS の停滞判定 (`ENOMOTO_MIP_CUT_STALL_HIGHS`, HighsMipSolverData::evaluateRootNode): 最初の LP 解からの
        // 移動方向の平均と今の移動の内積 (進み具合) を平滑化し、それが 1% 以上伸びず、かつ目的値の伸びが前のラウンドまでの
        // 伸びの 0.1% 以下なら停滞。3 回続けて停滞したら止める (目的値が動かなくても LP 解が動いている間は続ける)。
        // 40 問では 19 問 27.87 -> 17 問 28.49 (最大 100 ラウンドにすると 16 問 29.77): 根の下界は上がる (neos-1456979
        // 154 -> 163) が、こちらの 1 ラウンドは HiGHS の数倍重く、根に時間がかかる問題 (qnet1・10teams・misc07) が遅くなる
        let highs_stall = env_str!("ENOMOTO_MIP_CUT_STALL_HIGHS").is_some() && !self.params.submip;
        let first_x = self.lp.col_values();
        let mut avgdir = vec![0.0f64; n];
        let mut smooth = 0.0f64;
        let mut hstall = 0usize;
        let mut hrounds = 0usize;
        // サブ MIP (RENS/RINS) では分離に時間をかけない
        let mut max_rounds = if self.params.submip { 5 } else { tunable!("ENOMOTO_T_MIP_CUT_ROUNDS", 25usize, usize) };
        let time_cap = if self.params.time_limit.is_finite() { tunable!("ENOMOTO_T_MIP_CUT_TIME_FRAC", 0.1, f64) * self.params.time_limit } else { f64::INFINITY };
        let mut total_added = 0usize;
        let mut lp_failures = 0usize;
        // `ENOMOTO_MIP_CUT_PHASE2`: 既定の選択 (`select_cuts_hybrid`: 質が最良の 50% 未満のカットは採らない) で停滞して
        // 止まるところで、HiGHS のスコア (違反量 / (効いている非零の数 x そのノルム)、疎なカットを好む。ラウンドをまたいで
        // 最良スコアを覚え、その一定割合以上を採る) に切り替えてもう一度続ける第 2 段階。neos-911970 では既定の選択は根の
        // 下界 47.26 で止まり (100 ラウンド回しても動かない)、HiGHS のスコアなら 51.8 まで上がる (HiGHS は 38 ラウンドで
        // 52.1)。最初から HiGHS のスコアを使うと misc07・mik-250・neos5 が 60 秒で解けなくなった (木が数倍になる) ので、
        // 既定の選択が止まった後にだけ使う。第 2 段階は目的値が `ENOMOTO_T_MIP_CUT_PHASE2_STALL` ラウンド続けて
        // 動かなければ止め、ラウンド数は `ENOMOTO_T_MIP_CUT_PHASE2_ROUNDS` まで (根のカットの時間上限はそのまま)。
        // 40 問 (同時に測った既定 19 問 27.81 に対し) 18 問 28.64: nw04 33 -> 21 秒、h80x6320d の上界は良くなるが、
        // neos5 が時間切れ、10teams 4.2 -> 9.1 秒、qnet1 9.4 -> 14.7 秒、misc07 13.7 -> 18.8 秒 (根の LP 解が変わって木が
        // 変わる)。neos-911970 も 60 秒では解けない (根は 52.1 になるが暫定解が悪くなる) ので既定では使わない
        let phase2_on = (env_str!("ENOMOTO_MIP_CUT_PHASE2").is_some() || (self.highs_cutmgmt() && env_str!("ENOMOTO_MIP_HCM_NO_PHASE2").is_none())) && !self.params.submip;
        let mut phase2 = false;
        // 第 2 段階の時間の上限 (第 1 段階に使った時間の `ENOMOTO_T_MIP_CUT_PHASE2_TIME_MULT` 倍。qnet1 では再スタート後の
        // 第 2 段階が 88 ラウンド回って根が 6 秒伸び、9.2 秒 -> 17.2 秒になった)
        let t_loop0 = std::time::Instant::now();
        let mut phase2_deadline = f64::INFINITY;
        let mut round_ctr = 0usize;
        loop {
            if round_ctr >= max_rounds {
                break;
            }
            let round = round_ctr;
            round_ctr += 1;
            if self.time_up() || self.start.elapsed().as_secs_f64() > time_cap || (phase2 && t_loop0.elapsed().as_secs_f64() > phase2_deadline) {
                break;
            }
            let x = self.lp.col_values();
            if self.fractional(&x).is_empty() {
                break;
            }
            let t_sep = std::time::Instant::now();
            let mut cands = self.separate(&x, false);
            // 再スタート前のカットをプールで引き継いだとき (`ENOMOTO_MIP_RESTART_CUTS_TO_POOL`): プールで違反しているものも候補にする
            if ((self.params.restarts > 0 && env_str!("ENOMOTO_MIP_RESTART_CUTS_TO_POOL").is_some()) || env_str!("ENOMOTO_MIP_CUTSEL_HIGHS_POOL").is_some()) && round < tunable!("ENOMOTO_T_MIP_ROOT_POOL_ROUNDS", 5usize, usize) && env_str!("ENOMOTO_MIP_NO_ROOT_POOL").is_none() {
                for (eff, k) in self.pool_violated(&x) {
                    let (c, r, _) = &self.cut_pool[k];
                    cands.push(Candidate { coefs: c.clone(), rhs: *r, efficacy: eff });
                }
            }
            let ncands = cands.len();
            if cands.is_empty() {
                break;
            }
            let dbg_sep = env_str!("ENOMOTO_MIP_DEBUG_SEP").is_some() && !self.params.submip;
            let t_sel0 = std::time::Instant::now();
            // 候補はすべてカットプールに入れる (選択で落ちたものも、ノードで違反すれば使う)
            if !self.params.submip && env_str!("ENOMOTO_MIP_POOL_SELECTED_ONLY").is_none() {
                for c in &cands {
                    self.add_to_pool(&c.coefs, c.rhs);
                }
            }
            let pool_secs = t_sel0.elapsed().as_secs_f64();
            let cand_nnz: usize = cands.iter().map(|c| c.coefs.len()).sum();
            // 効き目の大きい順に、平行なものを除いて選ぶ
            let max_cuts = tunable!("ENOMOTO_T_MIP_ROOT_MAX_CUTS", 50usize, usize).min(p.m.max(50));
            let chosen = if phase2 {
                let maxpar = tunable!("ENOMOTO_T_MIP_CUT_PHASE2_MAXPAR", 0.3, f64);
                self.select_cuts_highs(cands, max_cuts, &x, maxpar)
            } else if env_str!("ENOMOTO_MIP_CUTSEL_HIGHS").is_some() {
                let maxpar = tunable!("ENOMOTO_T_CUTSEL_HIGHS_MAXPAR", 0.1, f64);
                self.select_cuts_highs(cands, max_cuts, &x, maxpar)
            } else {
                select_cuts(cands, max_cuts, p)
            };
            let sep_secs = t_sep.elapsed().as_secs_f64();
            let sel_secs = t_sel0.elapsed().as_secs_f64() - pool_secs;
            if chosen.is_empty() {
                break;
            }
            let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = chosen.into_iter().map(|c| (c.coefs, f64::NEG_INFINITY, c.rhs)).collect();
            total_added += rows.len();
            // カットプールに残す (ノードで違反していれば LP に戻す。候補として入れ済みなら重複は除かれる)
            if !self.params.submip {
                for (c, _, r) in &rows {
                    self.add_to_pool(c, *r);
                }
            }
            let saved_rows = self.lp.num_rows();
            let t_add0 = std::time::Instant::now();
            self.add_cut_rows(&rows);
            let add_secs = t_add0.elapsed().as_secs_f64();
            let t_lp0 = std::time::Instant::now();
            let it0 = self.lp.total_iterations();
            let lim = 10 * root_iters.max(100) + 10_000;
            // 1 回の LP の時間の上限 (全体の 2%、最低 1 秒): 数値的に悪条件の LP が残り時間を使い切るのを防ぐ
            let cap = if self.params.time_limit.is_finite() { (tunable!("ENOMOTO_T_MIP_CUT_LP_TIME_FRAC", 0.02, f64) * self.params.time_limit).max(1.0) } else { f64::INFINITY };
            let lp_deadline = if cap.is_finite() {
                let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(cap);
                Some(self.deadline.map_or(d, |g| g.min(d)))
            } else {
                self.deadline
            };
            let st = self.lp.solve(&SolveLimits { iteration_limit: lim, cutoff: f64::INFINITY, deadline: lp_deadline });
            self.sb_iters += 0;
            let _ = it0;
            if st != LpStatus::Optimal {
                // 数値的に困ったら今回のカットを外して戻す
                let mut remove = vec![false; self.lp.num_rows()];
                for r in remove.iter_mut().skip(saved_rows) {
                    *r = true;
                }
                self.delete_lp_rows(&remove);
                let st2 = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
                if self.params.verbose {
                    eprintln!("MIP: cut round {round}: LP status {st:?}, removed the cuts ({st2:?})");
                }
                // 1 回の失敗ではやめない (そのラウンドのカットだけ捨てて続ける。2 回目でやめる)
                lp_failures += 1;
                if st2 != LpStatus::Optimal || lp_failures >= 2 || env_str!("ENOMOTO_MIP_CUT_LP_FAIL_STOP").is_some() {
                    return st2 == LpStatus::Optimal;
                }
                continue;
            }
            let obj = self.lp.objective();
            if dbg_sep {
                eprintln!(
                    "SEP round {round}: cands {ncands} (nnz {cand_nnz}) pool {pool_secs:.3}s select {sel_secs:.3}s add_rows {add_secs:.3}s LP {:.3}s ({} iters, {:.1} us/iter)",
                    t_lp0.elapsed().as_secs_f64(),
                    self.lp.total_iterations() - it0,
                    1e6 * t_lp0.elapsed().as_secs_f64() / (self.lp.total_iterations() - it0).max(1) as f64
                );
            }
            if env_str!("ENOMOTO_MIP_DEBUG_FC").is_some() && !self.params.submip {
                super::cuts::FC_STATS.with(|s| eprintln!("FC stats (calls, cuts, no SNF): {:?}", s.borrow()));
            }
            if self.params.verbose {
                let cut_nnz: usize = (p.m..self.lp.num_rows()).map(|i| self.lp.row(i).len()).sum();
                let ncut_rows = self.lp.num_rows() - p.m;
                eprintln!(
                    "MIP: cut round {round}: {} cuts (of {ncands}, sep {sep_secs:.2}s, LP {} iters), LP rows {} (cut rows mean nnz {:.0}), obj {:.10e} ({:.2}s)",
                    rows.len(),
                    self.lp.total_iterations() - it0,
                    self.lp.num_rows(),
                    cut_nnz as f64 / ncut_rows.max(1) as f64,
                    obj + p.offset,
                    self.start.elapsed().as_secs_f64()
                );
            }
            if highs_stall {
                hrounds += 1;
                let xs = self.lp.col_values();
                let cur: Vec<f64> = (0..n).map(|j| first_x[j] - xs[j]).collect();
                let nrm = cur.iter().map(|v| v * v).sum::<f64>().sqrt();
                let scale = if nrm > 0.0 { 1.0 / nrm } else { 0.0 };
                let (mut sq, mut dot) = (0.0, 0.0);
                for j in 0..n {
                    avgdir[j] = (scale * cur[j] - avgdir[j]) / hrounds as f64;
                    sq += avgdir[j] * avgdir[j];
                    dot += avgdir[j] * cur[j];
                }
                let progress = if sq > 0.0 { dot / sq.sqrt() } else { 0.0 };
                if hrounds == 1 {
                    smooth = progress;
                } else {
                    let next = (2.0 / 3.0) * smooth + progress / 3.0;
                    if next < smooth * 1.01 && obj - first_obj <= (prev_obj - first_obj) * 1.001 {
                        hstall += 1;
                        if hstall >= 3 {
                            break;
                        }
                    } else {
                        hstall = 0;
                    }
                    smooth = next;
                }
            } else if env_str!("ENOMOTO_MIP_STALL_OLD").is_some() {
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
            } else {
                // SCIP の停滞判定 (solve.c): 目的値の相対変化が 1e-4 以下で、分数の列の数も十分に減っていなければ
                // 停滞。根では 10 回分で止める (サブ MIP では 3 回)。目的値も分数の列の数もまったく減らない
                // ラウンドは 2 回分と数える (何も進まない問題で長く回さない。分数の列が減っている間は続ける)。
                let nfrac = self.fractional(&self.lp.col_values()).len();
                let reldiff = (obj - prev_obj) / obj.abs().max(prev_obj.abs()).max(1.0);
                if phase2 {
                    // 第 2 段階: 目的値だけで停滞を数える
                    if reldiff <= 1e-4 {
                        stall += 1;
                        if stall >= tunable!("ENOMOTO_T_MIP_CUT_PHASE2_STALL", 10usize, usize) {
                            break;
                        }
                    } else {
                        stall = 0;
                    }
                } else {
                    // `ENOMOTO_MIP_CUT_STALL_ONE`: まったく進まないラウンドも 1 回分と数える (neos-1456979 では最初の
                    // 4 ラウンドは下界が動かず、続ければ上がる。HiGHS も止めずに 66 ラウンド回して 154 -> 171)
                    if reldiff <= 1e-4 && nfrac as f64 >= (0.9 - 0.1 * stall as f64) * prev_nfrac as f64 {
                        stall += if nfrac >= prev_nfrac && env_str!("ENOMOTO_MIP_CUT_STALL_ONE").is_none() { 2 } else { 1 };
                        if stall >= if self.params.submip { 3 } else { tunable!("ENOMOTO_T_MIP_CUT_STALL", 10usize, usize) } {
                            if !phase2_on {
                                break;
                            }
                            // 第 2 段階へ (ラウンド数の上限を延ばす)
                            phase2 = true;
                            stall = 0;
                            max_rounds = round_ctr + tunable!("ENOMOTO_T_MIP_CUT_PHASE2_ROUNDS", 60usize, usize);
                            let t1 = t_loop0.elapsed().as_secs_f64();
                            phase2_deadline = t1 + (tunable!("ENOMOTO_T_MIP_CUT_PHASE2_TIME_MULT", 3.0, f64) * t1).max(0.2);
                            if self.params.verbose {
                                eprintln!("MIP: cut loop phase 2 (HiGHS cut score) from round {} ({:.2}s)", round + 1, self.start.elapsed().as_secs_f64());
                            }
                        }
                    } else {
                        stall = 0;
                    }
                }
                prev_nfrac = nfrac;
            }
            prev_obj = obj;
            // 効いていないカットを外す (論理変数が基底にあり、行が緩んでいるもの)。`ENOMOTO_T_MIP_CUT_REMOVE_EVERY` ラウンドごと
            // `ENOMOTO_T_MIP_ROOT_CUT_AGE` > 0: HiGHS と同じく、効いていないラウンドが続いたカットだけを外す
            // (HiGHS の mip_lp_age_limit = 10)
            let root_age = tunable!("ENOMOTO_T_MIP_ROOT_CUT_AGE", 0u32, u32);
            let t_rm0 = std::time::Instant::now();
            let rows_before = self.lp.num_rows();
            if root_age > 0 {
                self.age_cuts();
                self.remove_cuts_older_than(root_age);
            } else if (round + 1) % tunable!("ENOMOTO_T_MIP_CUT_REMOVE_EVERY", 1usize, usize).max(1) == 0 {
                self.remove_inactive_cuts();
            }
            if dbg_sep {
                eprintln!("SEP round {round}: removed {} inactive cuts in {:.3}s", rows_before - self.lp.num_rows(), t_rm0.elapsed().as_secs_f64());
            }
        }
        self.remove_inactive_cuts();
        let st = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
        if self.params.verbose {
            eprintln!(
                "MIP: cut loop done: {} cuts added, {} in LP, pool {} cuts ({} nonzeros), obj {:.10e} -> {:.10e}",
                total_added,
                self.lp.num_rows() - p.m,
                self.cut_pool.len(),
                self.cut_pool_nnz,
                first_obj + p.offset,
                self.lp.objective() + p.offset
            );
        }
        st == LpStatus::Optimal
    }

    /// カットをプールに加える。係数と右辺が (丸めて) 同じものは加えない。プールの非零数は問題の非零数の
    /// 20 倍 (最低 20 万) までにする。
    pub(super) fn add_to_pool(&mut self, coefs: &[(usize, f64)], rhs: f64) {
        let nnz_cap = (tunable!("ENOMOTO_T_MIP_CUT_POOL_NNZ_MULT", 20usize, usize) * self.p.rows.iter().map(|r| r.len()).sum::<usize>()).max(200_000);
        if self.cut_pool_nnz + coefs.len() > nnz_cap {
            return;
        }
        // 最大係数 1 に揃えて 1e-9 の格子で丸めたもののハッシュ
        use std::hash::{Hash, Hasher};
        let cmax = coefs.iter().fold(0.0f64, |m, &(_, v)| m.max(v.abs())).max(1e-300);
        let mut hs = std::collections::hash_map::DefaultHasher::new();
        for &(j, v) in coefs {
            j.hash(&mut hs);
            ((v / cmax) * 1e9).round().to_bits().hash(&mut hs);
        }
        ((rhs / cmax) * 1e9).round().to_bits().hash(&mut hs);
        if !self.cut_pool_keys.insert(hs.finish()) {
            return;
        }
        let norm = coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
        self.cut_pool_nnz += coefs.len();
        self.cut_pool.push((coefs.to_vec(), rhs, norm));
        for &(j, v) in coefs {
            self.pool_idx.push(j as u32);
            self.pool_val.push(v);
        }
        self.pool_start.push(self.pool_idx.len() as u32);
    }

    /// カットプールのうち LP 解 `x` が違反するカットの (効き目, 番号)。平坦な複製 (`pool_idx` など) を走査する。
    pub(super) fn pool_violated(&self, x: &[f64]) -> Vec<(f64, usize)> {
        let mut viol: Vec<(f64, usize)> = Vec::new();
        for k in 0..self.cut_pool.len() {
            let (s, e) = (self.pool_start[k] as usize, self.pool_start[k + 1] as usize);
            let mut act = 0.0;
            for t in s..e {
                act += self.pool_val[t] * x[self.pool_idx[t] as usize];
            }
            let r = self.cut_pool[k].1;
            let eff = (act - r) / self.cut_pool[k].2.max(1e-12);
            if eff > 1e-4 && act - r > 1e-6 * (1.0 + r.abs()) {
                viol.push((eff, k));
            }
        }
        viol
    }

    /// カットプールから、LP 解 `x` が違反するカットを効き目の大きい順に最大 `max_cuts` 本 LP に加える
    /// (違反しているので今の LP にはないカット)。LP の行が増えすぎていれば先に効いていないカットを外す。加えたら真。
    pub(super) fn pool_cut_round(&mut self, x: &[f64], max_cuts: usize) -> bool {
        if self.cut_pool.is_empty() {
            return false;
        }
        let mut viol = self.pool_violated(x);
        if viol.is_empty() {
            return false;
        }
        viol.sort_by(|a, b| b.0.total_cmp(&a.0));
        viol.truncate(max_cuts);
        // `ENOMOTO_T_MIP_POOL_REL` = r (> 0): 効き目が最大のカットの r 倍に満たないカットは戻さない
        let rel = tunable!("ENOMOTO_T_MIP_POOL_REL", 0.0, f64);
        if rel > 0.0 {
            let top = viol[0].0;
            viol.retain(|&(e, _)| e >= rel * top);
        }
        let cap = self.p.m + (2 * self.cut_pool.len()).clamp(100, 2000);
        if self.lp.num_rows() + viol.len() > cap {
            self.remove_inactive_cuts();
        }
        let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = viol.iter().map(|&(_, k)| (self.cut_pool[k].0.clone(), f64::NEG_INFINITY, self.cut_pool[k].1)).collect();
        self.add_cut_rows(&rows);
        true
    }

    /// ノードでの分離 (1 ラウンド)。カットを加えたら LP を解き直し、真を返す。LP の行数の上限を超えたら何もしない。
    pub(super) fn node_cut_round(&mut self, x: &[f64]) -> bool {
        let p = self.p;
        let max_rows = p.m + (2 * p.m).max(500);
        if self.lp.num_rows() >= max_rows {
            return false;
        }
        // ノードでは軽い分離 (経路集約の始点・tableau 行を減らす) で、追加するカットも少なくする
        let cands = self.separate(x, true);
        if cands.is_empty() {
            return false;
        }
        let room = max_rows - self.lp.num_rows();
        let chosen = select_cuts(cands, room.min(tunable!("ENOMOTO_T_MIP_NODE_MAX_CUTS", 20usize, usize)), p);
        if chosen.is_empty() {
            return false;
        }
        let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = chosen.into_iter().map(|c| (c.coefs, f64::NEG_INFINITY, c.rhs)).collect();
        // カットはプールにも入れ (他のノードで違反すれば戻る)、このノードで効いていないカットは外してから加える
        // (HiGHS の aging と同じく LP を小さく保つ。外したカットもプールにあれば戻せる)
        if env_str!("ENOMOTO_MIP_NODE_CUTS_KEEP").is_none() {
            for (c, _, r) in &rows {
                self.add_to_pool(c, *r);
            }
            self.remove_aged_cuts();
        }
        self.add_cut_rows(&rows);
        true
    }

    /// HiGHS 流のカット管理 (`Solver::highs_cutmgmt`) でのノードでの分離 1 ラウンド: プールで違反しているカットと、
    /// 新しく分離したカット (軽い分離) を合わせて選び (最大 `ENOMOTO_T_MIP_HCM_NODE_MAX_CUTS` 本)、年齢の上限を超えたカットを
    /// 外してから加える。加えたら真 (呼び出し側が LP を解き直す)。LP の行数の上限を超えていれば何もしない。
    pub(super) fn hcm_node_sep_round(&mut self, x: &[f64], fresh: bool) -> bool {
        let p = self.p;
        let max_rows = p.m + (2 * p.m).max(500);
        if self.lp.num_rows() >= max_rows {
            return false;
        }
        // `fresh` でなければプールのカットだけ (安価。年齢で外れた根のカットを戻すのが主な役目)
        crate::simplex::slope_intercept_dual::xprof("hcm_pre");
        let mut cands = if fresh { self.separate(x, true) } else { Vec::new() };
        crate::simplex::slope_intercept_dual::xprof("hcm_sep");
        for (eff, k) in self.pool_violated(x) {
            let (c, r, _) = &self.cut_pool[k];
            cands.push(Candidate { coefs: c.clone(), rhs: *r, efficacy: eff });
        }
        crate::simplex::slope_intercept_dual::xprof("hcm_pool");
        if cands.is_empty() {
            return false;
        }
        let room = max_rows - self.lp.num_rows();
        // 1 ラウンドの本数は根と同じ程度まで許す (HiGHS はプールの違反カットを平行度だけで絞る)。年齢で外れたカットが多い
        // ノードでは、20 本ずつでは根で効いていたカットが戻りきらず、ノードの LP が根より弱くなって木の下界が動かなかった
        let chosen = select_cuts(cands, room.min(tunable!("ENOMOTO_T_MIP_HCM_NODE_MAX_CUTS", 100usize, usize)), p);
        if chosen.is_empty() {
            return false;
        }
        let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = chosen.into_iter().map(|c| (c.coefs, f64::NEG_INFINITY, c.rhs)).collect();
        for (c, _, r) in &rows {
            self.add_to_pool(c, *r);
        }
        self.remove_aged_cuts();
        self.add_cut_rows(&rows);
        true
    }

    /// LP にカットの行を加える (年齢 0)。
    pub(super) fn add_cut_rows(&mut self, rows: &[(Vec<(usize, f64)>, f64, f64)]) {
        self.sync_cut_age();
        self.lp.add_rows(rows);
        self.cut_age.extend(std::iter::repeat_n(0, rows.len()));
        for _ in 0..rows.len() {
            self.cut_ids.push(self.next_cut_id);
            self.next_cut_id += 1;
        }
        self.row_log.push(super::solver::RowEdit::Add(rows.len()));
    }

    /// LP の行を消す (カットの年齢も合わせて消す)。
    pub(super) fn delete_lp_rows(&mut self, remove: &[bool]) {
        self.sync_cut_age();
        let m0 = self.p.m;
        let mut k = m0;
        self.cut_age.retain(|_| {
            let keep = !remove[k];
            k += 1;
            keep
        });
        let mut k = m0;
        self.cut_ids.retain(|_| {
            let keep = !remove[k];
            k += 1;
            keep
        });
        self.lp.delete_rows(remove);
        self.row_log.push(super::solver::RowEdit::Delete(remove.to_vec()));
    }

    /// カットの年齢の長さを LP のカットの行の数に合わせる (足りなければ 0 で埋める)。
    fn sync_cut_age(&mut self) {
        let want = self.lp.num_rows().saturating_sub(self.p.m);
        self.cut_age.resize(want, 0);
        if self.cut_ids.len() > want {
            self.cut_ids.truncate(want);
        }
        while self.cut_ids.len() < want {
            self.cut_ids.push(self.next_cut_id);
            self.next_cut_id += 1;
        }
    }

    /// 今の LP の基底で効いている (論理変数が非基底の) カットの番号 (待ち行列に入れるノードに持たせ、その間は年齢で
    /// 外さない。取り出したときに保存した基底が必ず合う)。`ENOMOTO_MIP_NO_PROTECT_QUEUE_CUTS` で無効。
    /// 40 問: 19 問 27.04 -> 19 問 26.89 (保護して年齢の上限を一律 30 にすると 17 問 27.55)
    pub(super) fn basis_protected_cuts(&mut self) -> Option<std::rc::Rc<Vec<u64>>> {
        // HiGHS 流のカット管理では保護しない (HiGHS と同じく、取り出したノードの基底が LP の行と合わなければ今の基底から解く。
        // 保護すると待ち行列のノードが数千あるときほぼ全てのカットが外せず、neos-911970 では LP が行数の上限 607 に張り付いた)
        if env_str!("ENOMOTO_MIP_NO_PROTECT_QUEUE_CUTS").is_some() || self.params.submip || (self.highs_cutmgmt() && env_str!("ENOMOTO_MIP_HCM_PROTECT").is_none()) {
            return None;
        }
        self.sync_cut_age();
        let m0 = self.p.m;
        let mr = self.lp.num_rows();
        if mr <= m0 {
            return None;
        }
        let b = self.lp.basis();
        let v: Vec<u64> = (m0..mr).filter(|&i| b.row[i] != VarStatus::Basic).map(|i| self.cut_ids[i - m0]).collect();
        if v.is_empty() { None } else { Some(std::rc::Rc::new(v)) }
    }

    /// ノードの LP の後に呼ぶ (HiGHS の LP の aging): 効いていない (論理変数が基底で行に余裕がある) カットの年齢を
    /// 1 増やし、効いているカットは 0 に戻す。
    pub(super) fn age_cuts(&mut self) {
        self.sync_cut_age();
        if self.cut_age.is_empty() {
            return;
        }
        let m0 = self.p.m;
        let b = self.lp.basis();
        let act = self.lp.row_activities();
        // 相対的な基準 (既定では使わない):
        // `ENOMOTO_T_MIP_CUT_SLACK_REL` = s (> 0): 余裕のあるカットのうち、正規化した余裕 (余裕 / 係数の 2 ノルム) が
        //   カットの中で最大のものの s 倍に満たないもの (ほぼ効いている) は年齢を増やさない
        // `ENOMOTO_T_MIP_CUT_DUAL_REL` = d (> 0): 効いている (論理変数が非基底の) カットのうち、|双対値| がカットの中で
        //   最大のものの d 倍に満たないもの (退化して効いているだけ) は年齢を増やす
        let srel = tunable!("ENOMOTO_T_MIP_CUT_SLACK_REL", 0.0, f64);
        let drel = tunable!("ENOMOTO_T_MIP_CUT_DUAL_REL", 0.0, f64);
        let ncut = self.cut_age.len();
        let mut nslack = vec![0.0f64; ncut];
        let mut max_slack = 0.0f64;
        if srel > 0.0 {
            for k in 0..ncut {
                let i = m0 + k;
                let (_, up) = self.lp.row_bounds(i);
                if b.row[i] == VarStatus::Basic && act[i] < up - 1e-6 * (1.0 + up.abs()) {
                    let id = self.cut_ids[k];
                    let norm = match self.cut_norm.get(&id) {
                        Some(&v) => v,
                        None => {
                            let v = self.lp.row(i).iter().map(|&(_, a)| a * a).sum::<f64>().sqrt().max(1e-12);
                            self.cut_norm.insert(id, v);
                            v
                        }
                    };
                    nslack[k] = (up - act[i]) / norm;
                    max_slack = max_slack.max(nslack[k]);
                }
            }
            if self.cut_norm.len() > 4 * ncut + 1000 {
                let live: std::collections::HashSet<u64> = self.cut_ids.iter().copied().collect();
                self.cut_norm.retain(|id, _| live.contains(id));
            }
        }
        let mut dual = Vec::new();
        let mut max_dual = 0.0f64;
        if drel > 0.0 {
            dual = self.lp.row_duals();
            for k in 0..ncut {
                if b.row[m0 + k] != VarStatus::Basic {
                    max_dual = max_dual.max(dual[m0 + k].abs());
                }
            }
        }
        for (k, age) in self.cut_age.iter_mut().enumerate() {
            let i = m0 + k;
            let (_, up) = self.lp.row_bounds(i);
            if b.row[i] == VarStatus::Basic && act[i] < up - 1e-6 * (1.0 + up.abs()) {
                if srel > 0.0 && nslack[k] < srel * max_slack {
                    continue;
                }
                *age = age.saturating_add(1);
            } else if drel > 0.0 && b.row[i] != VarStatus::Basic && dual[i].abs() < drel * max_dual {
                *age = age.saturating_add(1);
            } else {
                *age = 0;
            }
        }
    }

    /// 年齢が上限 (`ENOMOTO_T_MIP_CUT_AGE_LIMIT`、既定 300。HiGHS の `mip_lp_age_limit` は 10 だが、10 では binkar10_1 で効くカットまで外れた) を超えたカットを LP から外す
    /// (プールにあるカットは、違反すればまた戻る)。外した数を返す。
    /// 年齢は今のノードで効いていない回数なので、待ち行列のノードで効いているカットまで外れうる。そのノードを取り出すと
    /// 保存した基底が合わず LP がほぼ解き直しになる (binkar10_1: 上限 30 で 1 ノード平均 272 反復、外さなければ 37)。
    /// 40 問: 上限 30 は 17 問 27.21、100 は 19 問 27.05、300 は 19 問 26.64 (外さないと neos5 で LP が重くなる)
    ///
    /// `ENOMOTO_T_MIP_CUT_AGE_PER_ROW` = f (> 0): 上限を f × (元の行の数) にして [`ENOMOTO_T_MIP_CUT_AGE_MIN`, 上の上限] に収める
    /// (行の少ない問題ではカットが LP を何倍にもする: markshare_4_0 は 4 行で、上限 300 だと遅くなった)。
    /// `ENOMOTO_T_MIP_CUT_ROWS_CAP` = r (> 0): カットの行が max(r × 元の行の数, 200) を超えたら、年齢の高い順に外して収める。
    pub(super) fn remove_aged_cuts(&mut self) -> usize {
        let base = tunable!("ENOMOTO_T_MIP_CUT_AGE_LIMIT", 300u32, u32);
        // 既定は f = 0.5 (40 問: 一律 300 の 19 問 26.72 に対し 19 問 26.41。markshare_4_0 (4 行) 25 -> 14 秒、
        // neos5 (63 行) は 42 -> 48 秒。カットの行の総数で抑える形 (`ENOMOTO_T_MIP_CUT_ROWS_CAP=2`) は 26.61)
        let f = tunable!("ENOMOTO_T_MIP_CUT_AGE_PER_ROW", 0.5, f64);
        // HiGHS 流のカット管理では上限を `ENOMOTO_T_MIP_HCM_AGE_LIMIT` (HiGHS の mip_lp_age_limit は 10) にする。
        // 30 では neos-911970 の木の下界が根から動かず (潜りの間に根で効いていたカットが外れ、ノードの LP が根より弱くなる)、
        // 100 で下界 54.70 / 上界 54.76 (最適値)。mik-250 は 30 でも 100 でも 60 秒で解けない (既定は 35 秒)
        let limit = if self.highs_cutmgmt() {
            tunable!("ENOMOTO_T_MIP_HCM_AGE_LIMIT", 100u32, u32)
        } else if f > 0.0 {
            let lo = tunable!("ENOMOTO_T_MIP_CUT_AGE_MIN", 30u32, u32);
            ((f * self.p.m as f64) as u32).clamp(lo.min(base), base)
        } else {
            base
        };
        let mut removed = self.remove_cuts_older_than(limit);
        let r = tunable!("ENOMOTO_T_MIP_CUT_ROWS_CAP", 0.0, f64);
        if r > 0.0 {
            self.sync_cut_age();
            let cap = ((r * self.p.m as f64) as usize).max(200);
            let ncut = self.cut_age.len();
            if ncut > cap {
                // 年齢の高い順に (ncut - cap) 本外す (年齢 0 = 今効いているものは外さない)
                let mut order: Vec<(u32, usize)> = self.cut_age.iter().enumerate().filter(|&(_, &a)| a > 0).map(|(k, &a)| (a, k)).collect();
                order.sort_by(|a, b| b.cmp(a));
                let m0 = self.p.m;
                let mut remove = vec![false; self.lp.num_rows()];
                let mut cnt = 0;
                for &(_, k) in order.iter().filter(|&&(_, k)| !self.queue.is_protected(self.cut_ids[k])).take(ncut - cap) {
                    remove[m0 + k] = true;
                    cnt += 1;
                }
                if cnt > 0 {
                    self.delete_lp_rows(&remove);
                    removed += cnt;
                }
            }
        }
        removed
    }

    /// 年齢が `limit` を超えたカットを LP から外す。外した数を返す。
    pub(super) fn remove_cuts_older_than(&mut self, limit: u32) -> usize {
        self.sync_cut_age();
        let m0 = self.p.m;
        let mut remove = vec![false; self.lp.num_rows()];
        let mut cnt = 0;
        for (k, &age) in self.cut_age.iter().enumerate() {
            if age > limit && !self.queue.is_protected(self.cut_ids[k]) {
                remove[m0 + k] = true;
                cnt += 1;
            }
        }
        if cnt > 0 {
            self.delete_lp_rows(&remove);
        }
        cnt
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
            self.delete_lp_rows(&remove);
        }
    }

    /// HiGHS の `HighsPathSeparator` (path mixing cut を除く) の移植。
    /// - 行の型: 等式 / LP で効いている側 (`<=` か `>=`) / 効いていない (使わない)
    /// - 連続変数 (境界から離れているもの) ごとに、係数の符号で「入る」行と「出る」行に分ける (`<=` の行で負なら入る、
    ///   `>=` の行で正なら入る、等式は両方)。集約に残った連続変数を消すときは、係数が負なら入る行、正なら出る行から選ぶ
    /// - 連続変数がそれ 1 つだけの等式は代入用にとっておき、始点・相手には使わない
    /// - 効いている行それぞれを始点に、最大 6 段。各段で集約行とその符号を反転したものから CMIR / flow cover を作り、
    ///   カットが出たら伸ばさない
    fn path_aggregation_highs<F>(&mut self, vars: &CutVars, lp_rows: &[Vec<(usize, f64)>], cands: &mut Vec<Candidate>, push: &mut F)
    where
        F: FnMut(Option<RawCut>, &mut Vec<Candidate>, &Solver<L>),
    {
        const MAX_PATH_LEN: usize = 6;
        let n = self.p.n;
        let m = lp_rows.len();
        let feastol = 1e-6;
        let bound_dist = |j: usize| -> f64 {
            let xj = vars.x[j];
            let mut blo = vars.lo[j];
            let mut bup = vars.up[j];
            if let Some(vb) = vars.vb {
                for &(y, a, e) in &vb.vub[j] {
                    bup = bup.min(a * vars.x[y] + e);
                }
                for &(y, a, e) in &vb.vlb[j] {
                    blo = blo.max(a * vars.x[y] + e);
                }
            }
            let mut dl = xj - blo;
            let mut du = bup - xj;
            if dl <= feastol {
                dl = 0.0;
            }
            if du <= feastol {
                du = 0.0;
            }
            dl.min(du)
        };
        // 行の型: 0 使わない, 1 <=, -1 >=, 2 等式
        let mut rtype = vec![0i8; m];
        for i in 0..m {
            let (l, u) = (vars.lo[n + i], vars.up[n + i]);
            if l == u {
                rtype[i] = 2;
                continue;
            }
            let r = vars.x[n + i];
            let ls = if l.is_finite() { r - l } else { f64::INFINITY };
            let us = if u.is_finite() { u - r } else { f64::INFINITY };
            rtype[i] = if ls > feastol && us > feastol {
                0
            } else if ls < us {
                -1
            } else {
                1
            };
        }
        let cont: Vec<bool> = (0..n).map(|j| !vars.is_int[j] && bound_dist(j) > 0.0).collect();
        let mut num_cont = vec![0usize; m];
        let mut col_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        for i in 0..m {
            for &(j, a) in &lp_rows[i] {
                if cont[j] {
                    num_cont[i] += 1;
                    col_rows[j].push((i, a));
                }
            }
        }
        // 代入用の等式 (連続変数が 1 つだけ)
        let mut subst: Vec<Option<(usize, f64)>> = vec![None; n];
        for i in 0..m {
            if rtype[i] != 2 || num_cont[i] != 1 {
                continue;
            }
            let Some(&(j, a)) = lp_rows[i].iter().find(|&&(j, _)| cont[j]) else { continue };
            if subst[j].is_some() {
                continue;
            }
            subst[j] = Some((i, a));
            rtype[i] = 0;
        }
        // 連続変数ごとの入る行・出る行
        let mut in_arcs: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        let mut out_arcs: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        for j in 0..n {
            if !cont[j] || subst[j].is_some() {
                continue;
            }
            for &(i, a) in &col_rows[j] {
                match rtype[i] {
                    0 => {}
                    1 => {
                        if a < 0.0 { in_arcs[j].push((i, a)) } else { out_arcs[j].push((i, a)) }
                    }
                    -1 => {
                        if a > 0.0 { in_arcs[j].push((i, a)) } else { out_arcs[j].push((i, a)) }
                    }
                    _ => {
                        in_arcs[j].push((i, a));
                        out_arcs[j].push((i, a));
                    }
                }
            }
        }
        let mut agg = vec![0.0f64; n + m];
        let mut touched: Vec<usize> = Vec::new();
        let mut in_t = vec![false; n + m];
        let add_row = |i: usize, w: f64, agg: &mut Vec<f64>, touched: &mut Vec<usize>, in_t: &mut Vec<bool>| {
            for &(j, a) in &lp_rows[i] {
                if !in_t[j] {
                    in_t[j] = true;
                    touched.push(j);
                }
                agg[j] += w * a;
            }
            let k = n + i;
            if !in_t[k] {
                in_t[k] = true;
                touched.push(k);
            }
            agg[k] -= w;
        };
        for start in 0..m {
            if start % 64 == 0 && self.time_up() {
                return;
            }
            let scales: [f64; 2] = match rtype[start] {
                0 => continue,
                1 | 2 => [1.0, -1.0],
                _ => [-1.0, 1.0],
            };
            for &scale in &scales {
                for &k in &touched {
                    agg[k] = 0.0;
                    in_t[k] = false;
                }
                touched.clear();
                add_row(start, scale, &mut agg, &mut touched, &mut in_t);
                let mut path: Vec<usize> = vec![start];
                let mut try_neg = false;
                let mut success = false;
                let mut guard = 0;
                while path.len() < MAX_PATH_LEN {
                    guard += 1;
                    if guard > 50 {
                        break;
                    }
                    let in_path = |r: usize, path: &Vec<usize>| path.contains(&r);
                    // 代入できる連続変数があれば先に代入して数え直す
                    let mut substituted = false;
                    let snapshot: Vec<(usize, f64)> = touched.iter().filter(|&&k| k < n && agg[k].abs() > 1e-9).map(|&k| (k, agg[k])).collect();
                    for &(j, v) in &snapshot {
                        if !cont[j] {
                            continue;
                        }
                        if let Some((r, a)) = subst[j] {
                            add_row(r, -v / a, &mut agg, &mut touched, &mut in_t);
                            agg[j] = 0.0;
                            substituted = true;
                        }
                    }
                    if substituted {
                        continue;
                    }
                    let plen = path.len();
                    let mut skip_col = |j: usize, arcs: &Vec<Vec<(usize, f64)>>, other: &Vec<Vec<(usize, f64)>>, try_neg: &mut bool, path: &Vec<usize>| -> bool {
                        if plen == 1 && !*try_neg {
                            if arcs[j].len() <= plen {
                                if arcs[j].iter().any(|&(r, _)| r != start) {
                                    *try_neg = true;
                                }
                            } else {
                                *try_neg = true;
                            }
                        }
                        if other[j].is_empty() {
                            return true;
                        }
                        if other[j].len() <= plen {
                            return other[j].iter().all(|&(r, _)| in_path(r, path));
                        }
                        false
                    };
                    let mut best_out: Option<(usize, f64, f64)> = None;
                    let mut best_in: Option<(usize, f64, f64)> = None;
                    for &(j, v) in &snapshot {
                        if !cont[j] {
                            continue;
                        }
                        let d = bound_dist(j);
                        if v < 0.0 {
                            if skip_col(j, &out_arcs, &in_arcs, &mut try_neg, &path) {
                                continue;
                            }
                            if best_out.is_none_or(|(_, _, bd)| d > bd) {
                                best_out = Some((j, v, d));
                            }
                        } else {
                            if skip_col(j, &in_arcs, &out_arcs, &mut try_neg, &path) {
                                continue;
                            }
                            if best_in.is_none_or(|(_, _, bd)| d > bd) {
                                best_in = Some((j, v, d));
                            }
                        }
                    }
                    // カットを作る (集約行と、その符号を反転したもの)
                    let n_before = cands.len();
                    let amax = touched.iter().fold(0.0f64, |mx, &k| mx.max(agg[k].abs()));
                    let base: Vec<(usize, f64)> = touched.iter().filter(|&&k| agg[k].abs() > 1e-9 * amax.max(1.0) || k >= n && agg[k] != 0.0).map(|&k| (k, agg[k])).collect();
                    let neg: Vec<(usize, f64)> = base.iter().map(|&(k, a)| (k, -a)).collect();
                    if gen_old() {
                        push(cmir(vars, &base, 0.0), cands, self);
                        if use_lifted_cover() {
                            push(lifted_cover(vars, &base, 0.0), cands, self);
                        }
                        if use_flow_cover() {
                            push(lifted_flow_cover(vars, &base, 0.0), cands, self);
                        }
                        push(cmir(vars, &neg, 0.0), cands, self);
                        if use_lifted_cover() {
                            push(lifted_cover(vars, &neg, 0.0), cands, self);
                        }
                        if use_flow_cover() {
                            push(lifted_flow_cover(vars, &neg, 0.0), cands, self);
                        }
                    } else {
                        for b in [&base, &neg] {
                            let n0 = cands.len();
                            for r in generate_cuts(vars, b, 0.0, use_flow_cover(), false) {
                                push(Some(r), cands, self);
                            }
                            keep_best(cands, n0, self.p);
                        }
                    }
                    success = cands.len() > n_before;
                    if success || (best_out.is_none() && best_in.is_none()) {
                        break;
                    }
                    // 消す連続変数と相手の行 (乱数の位置から、経路にない行で重みが範囲内のもの)
                    let find_row = |j: usize, v: f64, arcs: &Vec<(usize, f64)>, s: &mut Self, path: &Vec<usize>| -> Option<(usize, f64)> {
                        if arcs.is_empty() {
                            return None;
                        }
                        let k0 = ((s.rand() * arcs.len() as f64) as usize).min(arcs.len() - 1);
                        for t in 0..arcs.len() {
                            let (r, a) = arcs[(k0 + t) % arcs.len()];
                            let w = -v / a;
                            if !in_path(r, path) && w.abs() <= 1.0 / feastol && w.abs() >= feastol {
                                let _ = j;
                                return Some((r, w));
                            }
                        }
                        None
                    };
                    let pick = match (best_out, best_in) {
                        (Some(o), i) if i.is_none_or(|i| o.2 >= i.2 - feastol) => find_row(o.0, o.1, &in_arcs[o.0], self, &path).or_else(|| i.and_then(|i| find_row(i.0, i.1, &out_arcs[i.0], self, &path))),
                        (_, Some(i)) => find_row(i.0, i.1, &out_arcs[i.0], self, &path),
                        _ => None,
                    };
                    let Some((r, w)) = pick else { break };
                    add_row(r, w, &mut agg, &mut touched, &mut in_t);
                    path.push(r);
                }
                let _ = success;
                if !try_neg {
                    break;
                }
            }
        }
    }

    /// 経路集約: 元の行を 1 本選び、集約行に残った連続変数のうち LP 値が境界 (変数上下限を含む) から
    /// 最も離れたものを、それを含む別の元の行で打ち消す、を最大 `PATH_MAX_LEN` 段繰り返し、各段で CMIR を試す
    /// (Marchand & Wolsey の集約ヒューリスティクス)。行 i の活動量を変数 `n + i` として
    /// `sum_i w_i (a_i x - r_i) = 0` の形で集約するので、等式・不等式・範囲行を区別せずに扱える
    /// (`r_i` の置き換えは CMIR が行の上下限で行う)。
    fn path_aggregation<F>(&mut self, vars: &CutVars, lp_rows: &[Vec<(usize, f64)>], cands: &mut Vec<Candidate>, push: &mut F, max_starts: usize)
    where
        F: FnMut(Option<RawCut>, &mut Vec<Candidate>, &Solver<L>),
    {
        const PATH_MAX_LEN: usize = 6;
        const MAX_ROW_LEN: usize = 500;
        let p = self.p;
        let n = p.n;
        // 元の行だけでなく LP に入っているカットの行も集約に使う (HiGHS と同じ)。再スタート後はカットが元の行になり
        // 経路の候補が大きく増えて根の下界が伸びたので、再スタートを待たずに使う
        let m = if env_str!("ENOMOTO_MIP_PATH_ORIG_ONLY").is_some() { p.m } else { lp_rows.len() };
        // 連続変数の LP 値の、最も近い境界 (単純な上下限・変数上下限) までの距離
        let bound_dist = |j: usize| -> f64 {
            let xj = vars.x[j];
            let mut d = (xj - vars.lo[j]).min(vars.up[j] - xj);
            if let Some(vb) = vars.vb {
                for &(y, a, e) in &vb.vub[j] {
                    d = d.min(a * vars.x[y] + e - xj);
                }
                for &(y, a, e) in &vb.vlb[j] {
                    d = d.min(xj - a * vars.x[y] - e);
                }
            }
            d.max(0.0)
        };
        // 列 → 元の行 (短い行だけ)
        let mut col_rows: Vec<Vec<usize>> = vec![Vec::new(); n];
        for i in 0..m {
            if lp_rows[i].len() <= MAX_ROW_LEN {
                for &(j, _) in &lp_rows[i] {
                    if !vars.is_int[j] {
                        col_rows[j].push(i);
                    }
                }
            }
        }
        // 行が LP で効いているか (活動量が上下限に近い)
        let tight = |i: usize| -> bool {
            let r = vars.x[n + i];
            let (l, u) = (vars.lo[n + i], vars.up[n + i]);
            (r - l).abs() <= 1e-6 * (1.0 + l.abs()) || (u - r).abs() <= 1e-6 * (1.0 + u.abs())
        };
        let mut agg = vec![0.0f64; n];
        let mut in_agg = vec![false; n];
        let mut touched: Vec<usize> = Vec::new();
        let mut used_row = vec![false; m];
        let mut starts = 0usize;
        // 始点の行 (既定は元の行だけ。カットの行は相手としてだけ使う)
        let m_start = if env_str!("ENOMOTO_MIP_PATH_START_CUTS").is_some() { m } else { p.m.min(m) };
        // `ENOMOTO_MIP_PATH_TIGHT_STARTS`: 始点は効いている行だけ (HiGHS)。`ENOMOTO_MIP_PATH_ROTATE`: 始点の行の順を
        // 毎回ずらす。`ENOMOTO_MIP_PATH_BOTH_SIGNS`: 始点の行を両方の向きで使う (HiGHS)。h80x6320d ではどれも根の
        // 下界を上げず (両方の向きは候補が増えすぎて LP が詰まり悪化)、既定では使わない
        let tight_only = env_str!("ENOMOTO_MIP_PATH_TIGHT_STARTS").is_some();
        let offset = if m_start > 0 && env_str!("ENOMOTO_MIP_PATH_ROTATE").is_some() { self.path_offset % m_start } else { 0 };
        let both_signs = env_str!("ENOMOTO_MIP_PATH_BOTH_SIGNS").is_some();
        let stop_on_cut = env_str!("ENOMOTO_MIP_PATH_STOP_ON_CUT").is_some();
        let mut last = offset;
        for sidx in 0..m_start * if both_signs { 2 } else { 1 } {
            let start = (offset + sidx / if both_signs { 2 } else { 1 }) % m_start;
            let sign = if both_signs && sidx % 2 == 1 { -1.0 } else { 1.0 };
            if starts >= max_starts || (sidx % 64 == 0 && self.time_up()) {
                break;
            }
            last = start;
            let row = &lp_rows[start];
            if row.len() < 2 || row.len() > MAX_ROW_LEN {
                continue;
            }
            if tight_only && !tight(start) {
                continue;
            }
            // 境界から離れた連続変数がなければ 1 行の CMIR と同じなので飛ばす
            if !row.iter().any(|&(j, _)| !vars.is_int[j] && bound_dist(j) > 1e-6) {
                continue;
            }
            starts += 1;
            for &j in &touched {
                agg[j] = 0.0;
                in_agg[j] = false;
            }
            touched.clear();
            let mut weights: Vec<(usize, f64)> = vec![(start, sign)];
            used_row[start] = true;
            for &(j, a) in row {
                if !in_agg[j] {
                    in_agg[j] = true;
                    touched.push(j);
                }
                agg[j] += sign * a;
            }
            for step in 0..PATH_MAX_LEN {
                if step > 0 {
                    let n_before = cands.len();
                    let mut base: Vec<(usize, f64)> = Vec::with_capacity(touched.len() + weights.len());
                    let amax = touched.iter().fold(0.0f64, |mx, &j| mx.max(agg[j].abs()));
                    for &j in &touched {
                        if agg[j].abs() > 1e-9 * amax.max(1.0) {
                            base.push((j, agg[j]));
                        }
                    }
                    for &(i, w) in &weights {
                        base.push((n + i, -w));
                    }
                    let neg: Vec<(usize, f64)> = base.iter().map(|&(k, a)| (k, -a)).collect();
                    if gen_old() {
                        push(cmir(vars, &base, 0.0), cands, self);
                        if use_lifted_cover() {
                            push(lifted_cover(vars, &base, 0.0), cands, self);
                        }
                        push(cmir(vars, &neg, 0.0), cands, self);
                        if use_lifted_cover() {
                            push(lifted_cover(vars, &neg, 0.0), cands, self);
                        }
                        if use_flow_cover() {
                            push(lifted_flow_cover(vars, &base, 0.0), cands, self);
                            push(lifted_flow_cover(vars, &neg, 0.0), cands, self);
                        }
                    } else {
                        for b in [&base, &neg] {
                            let n0 = cands.len();
                            for r in generate_cuts(vars, b, 0.0, use_flow_cover(), false) {
                                push(Some(r), cands, self);
                            }
                            keep_best(cands, n0, self.p);
                        }
                    }
                    // HiGHS と同じく、カットが出たら経路を伸ばさない (`ENOMOTO_MIP_PATH_STOP_ON_CUT`)
                    if cands.len() > n_before && stop_on_cut {
                        break;
                    }
                }
                // 打ち消す連続変数: 境界から最も離れたもの
                let amax = touched.iter().fold(0.0f64, |mx, &j| mx.max(agg[j].abs()));
                let mut best: Option<(usize, f64)> = None;
                for &j in &touched {
                    if vars.is_int[j] || agg[j].abs() <= 1e-9 * amax.max(1.0) {
                        continue;
                    }
                    let d = bound_dist(j);
                    if d > 1e-6 && best.is_none_or(|(_, bd)| d > bd) {
                        best = Some((j, d));
                    }
                }
                let Some((j, _)) = best else { break };
                // 相手の行: 未使用で j を含むもの。効いている行を優先し、同じ組では乱数で選ぶ
                let mut pick: Option<(usize, f64)> = None;
                for &i in &col_rows[j] {
                    if used_row[i] {
                        continue;
                    }
                    let score = if tight(i) { 2.0 } else { 1.0 } + 0.5 * self.rand();
                    if pick.is_none_or(|(_, sc)| score > sc) {
                        pick = Some((i, score));
                    }
                }
                let Some((r, _)) = pick else { break };
                let arj = lp_rows[r].iter().find(|&&(k, _)| k == j).map_or(0.0, |&(_, a)| a);
                if arj.abs() < 1e-9 {
                    break;
                }
                let w = -agg[j] / arj;
                // 重みの比が大きすぎる集約は数値的に危ないので止める
                let wmax = weights.iter().fold(w.abs(), |mx, &(_, v)| mx.max(v.abs()));
                let wmin = weights.iter().fold(w.abs(), |mn, &(_, v)| mn.min(v.abs()));
                if wmax / wmin > 1e4 {
                    break;
                }
                used_row[r] = true;
                weights.push((r, w));
                for &(k, a) in &lp_rows[r] {
                    if !in_agg[k] {
                        in_agg[k] = true;
                        touched.push(k);
                    }
                    agg[k] += w * a;
                }
                agg[j] = 0.0;
            }
            for &(i, _) in &weights {
                used_row[i] = false;
            }
        }
        self.path_offset = last + 1;
    }

    /// 現在の LP 解 `x` を切る候補を作る。
    /// `light` (ノード用) なら経路集約の始点と tableau 行の数を絞る。
    fn separate(&mut self, x: &[f64], light: bool) -> Vec<Candidate> {
        let p = self.p;
        let n = p.n;
        let mr = self.lp.num_rows();
        // 変数: 構造変数 0..n と、LP 行の活動量 n..n+mr
        let lp_rows: Vec<Vec<(usize, f64)>> = (0..mr).map(|i| self.lp.row(i)).collect();
        let act = self.lp.row_activities();
        // カットは木全体で有効にするため、大域的な境界から作る
        let mut lo = self.dom.global_lo.clone();
        let mut up = self.dom.global_up.clone();
        let mut is_int = self.cut_int.clone();
        // 暗黙の整数列の境界も整数に丸める (整数列は丸め済み)
        for j in 0..n {
            if is_int[j] && !p.is_int[j] {
                lo[j] = (lo[j] - FEASTOL).ceil();
                up[j] = (up[j] + FEASTOL).floor();
            }
        }
        let mut xv = x.to_vec();
        for i in 0..mr {
            let (l, u) = self.lp.row_bounds(i);
            let integral = lp_rows[i].iter().all(|&(j, a)| self.cut_int[j] && (a - a.round()).abs() <= 1e-9);
            let (l, u) = if integral { ((l - FEASTOL).ceil(), (u + FEASTOL).floor()) } else { (l, u) };
            lo.push(l);
            up.push(u);
            is_int.push(integral);
            xv.push(act[i]);
        }
        if self.vbounds.is_none() {
            self.vbounds = Some(std::rc::Rc::new(VarBounds::from_rows(n, &p.is_int, &p.rows, &p.row_lo, &p.row_up)));
        }
        let vb = self.vbounds.clone();
        let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &xv, vb: if env_str!("ENOMOTO_MIP_NO_VB").is_some() { None } else { vb.as_deref() } };
        // 診断用 (`ENOMOTO_MIP_DEBUG_XFILE`): HiGHS の書き出し (`HIGHS_CUT_DUMP`) の LP 解で経路集約だけを試し、カットの数と
        // 効き目を表示する (前処理なし `ENOMOTO_MIP_NO_PRESOLVE` で、列の並びを HiGHS と揃えて使う)
        if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_XFILE") {
            if !self.debug_x_done && !self.params.submip {
                self.debug_x_done = true;
                let xh: Vec<f64> = std::fs::read_to_string(&f).unwrap().lines().filter(|l| l.starts_with("C ")).map(|l| l.split_whitespace().nth(4).unwrap().parse().unwrap()).collect();
                if xh.len() == n {
                    let mut xvh = xh.clone();
                    for row in &lp_rows {
                        xvh.push(row.iter().map(|&(j, a)| a * xh[j]).sum());
                    }
                    let vars_h = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &xvh, vb: vb.as_deref() };
                    let mut ch: Vec<Candidate> = Vec::new();
                    let mut scratch_h = vec![0.0f64; n];
                    let mut push_h = |raw: Option<RawCut>, cands: &mut Vec<Candidate>, s: &Solver<L>| {
                        if let Some(raw) = raw {
                            if let Some(c) = finish_cut(raw, n, &lp_rows, &s.cut_int, &s.dom.global_lo, &s.dom.global_up, &xh, None, &mut scratch_h) {
                                cands.push(c);
                            }
                        }
                    };
                    if env_str!("ENOMOTO_MIP_PATH_OLD").is_none() {
                        self.path_aggregation_highs(&vars_h, &lp_rows, &mut ch, &mut push_h);
                    } else {
                        self.path_aggregation(&vars_h, &lp_rows, &mut ch, &mut push_h, usize::MAX);
                    }
                    let mut e: Vec<f64> = ch.iter().map(|c| c.efficacy).collect();
                    e.sort_by(|a, b| b.total_cmp(a));
                    let mean = e.iter().sum::<f64>() / e.len().max(1) as f64;
                    eprintln!(
                        "DEBUG_XFILE path cuts {} mean eff {:.4} max {:.4} top10 {:?} >0.1: {} >0.01: {}",
                        e.len(),
                        mean,
                        e.first().copied().unwrap_or(0.0),
                        e.iter().take(10).map(|v| (v * 1e4).round() / 1e4).collect::<Vec<_>>(),
                        e.iter().filter(|&&v| v > 0.1).count(),
                        e.iter().filter(|&&v| v > 0.01).count()
                    );
                } else {
                    eprintln!("DEBUG_XFILE: {} columns in the file, {} in the problem", xh.len(), n);
                }
            }
        }
        let mut cands: Vec<Candidate> = Vec::new();
        // 診断用: finish_cut に使った時間の累計 (秒)
        let t_finish = std::cell::Cell::new(0.0f64);
        let mut finish_scratch = vec![0.0f64; n];
        let mut push = |raw: Option<RawCut>, cands: &mut Vec<Candidate>, s: &Solver<L>| {
            if let Some(raw) = raw {
                let t0 = std::time::Instant::now();
                if let Some(c) = finish_cut(raw, n, &lp_rows, &s.cut_int, &s.dom.global_lo, &s.dom.global_up, x, s.incumbent.as_ref().map(|(_, v)| v.as_slice()), &mut finish_scratch) {
                    cands.push(c);
                }
                t_finish.set(t_finish.get() + t0.elapsed().as_secs_f64());
            }
        };
        let sep_t0 = std::time::Instant::now();
        let dbg_sep = env_str!("ENOMOTO_MIP_DEBUG_SEP").is_some();
        // 元の行 1 本ずつ
        for i in 0..p.m {
            if i % 64 == 0 && self.time_up() {
                return cands;
            }
            let row = &p.rows[i];
            if row.len() < 2 {
                continue;
            }
            if !gen_old() {
                let neg: Vec<(usize, f64)> = row.iter().map(|&(j, a)| (j, -a)).collect();
                for (b, r) in [(row.as_slice(), p.row_up[i]), (neg.as_slice(), -p.row_lo[i])] {
                    if r.is_finite() {
                        let n0 = cands.len();
                        for c in generate_cuts(&vars, b, r, use_flow_cover(), true) {
                            push(Some(c), &mut cands, self);
                        }
                        keep_best(&mut cands, n0, self.p);
                    }
                }
                continue;
            }
            if p.row_up[i].is_finite() {
                if use_flow_cover() {
                    push(lifted_flow_cover(&vars, row, p.row_up[i]), &mut cands, self);
                }
                if use_lifted_cover() {
                    push(lifted_cover(&vars, row, p.row_up[i]), &mut cands, self);
                }
                let r = cmir(&vars, row, p.row_up[i]);
                push(r, &mut cands, self);
                let r = extended_cover(&vars, row, p.row_up[i]);
                push(r, &mut cands, self);
            }
            if p.row_lo[i].is_finite() {
                let neg: Vec<(usize, f64)> = row.iter().map(|&(j, a)| (j, -a)).collect();
                if use_flow_cover() {
                    push(lifted_flow_cover(&vars, &neg, -p.row_lo[i]), &mut cands, self);
                }
                if use_lifted_cover() {
                    push(lifted_cover(&vars, &neg, -p.row_lo[i]), &mut cands, self);
                }
                let r = cmir(&vars, &neg, -p.row_lo[i]);
                push(r, &mut cands, self);
                let r = extended_cover(&vars, &neg, -p.row_lo[i]);
                push(r, &mut cands, self);
            }
        }
        if dbg_sep {
            eprintln!("SEP rows {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        // zerohalf ({0, 1/2}-CG) カット (元の行から)
        if env_str!("ENOMOTO_MIP_NO_ZEROHALF").is_none() {
            let zh = super::zerohalf::zerohalf_cuts(&p.rows, &p.row_lo, &p.row_up, &self.cut_int, &self.dom.global_lo, &self.dom.global_up, x, 100);
            for raw in zh {
                push(Some(raw), &mut cands, self);
            }
        }
        if dbg_sep {
            eprintln!("SEP zerohalf {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        // clique カット (2 値列の衝突グラフ。根で一度だけ作って使い回す)。既定では使わない: 40 問で根の下界は
        // ほとんど変わらず、10teams では根の LP 解が変わって根の被約費用ヒューリスティクスが解を見つけられなくなった
        // (13 問 / 幾何平均 30.57 に悪化)。ENOMOTO_MIP_CLIQUE_CUTS=1 で使う
        if env_str!("ENOMOTO_MIP_CLIQUE_CUTS").is_some() {
            if self.clique_graph.is_none() {
                let binary: Vec<bool> = (0..p.n).map(|j| p.is_int[j] && self.dom.global_lo[j] >= 0.0 && self.dom.global_up[j] <= 1.0).collect();
                let g = super::clique::CliqueGraph::build(p.n, &p.rows, &p.row_lo, &p.row_up, &binary, &self.dom.global_lo, &self.dom.global_up);
                if self.params.verbose && !self.params.submip {
                    eprintln!("MIP: clique graph: {} edges", g.num_edges());
                }
                self.clique_graph = Some(std::rc::Rc::new(g));
            }
            let g = self.clique_graph.clone().unwrap();
            for raw in g.separate(x, 100) {
                push(Some(raw), &mut cands, self);
            }
        }
        if dbg_sep {
            eprintln!("SEP clique {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        // 経路集約 (path aggregation、HiGHS の `HighsPathSeparator`)
        if env_str!("ENOMOTO_MIP_NO_PATH_AGG").is_none() {
            let max_starts = if light { tunable!("ENOMOTO_T_MIP_NODE_PATH_STARTS", 0usize, usize) } else { tunable!("ENOMOTO_T_MIP_PATH_STARTS", 1000usize, usize) };
            // 既定は HiGHS の経路集約の移植 (`ENOMOTO_MIP_PATH_OLD` で従来の経路集約)
            // 経路集約: 従来のもの (`ENOMOTO_MIP_PATH_OLD`)、HiGHS の移植 (`ENOMOTO_MIP_PATH_HIGHS_ONLY`)、既定は両方
            // (h80x6320d・rout は移植で根の下界が上がり、mik-250 は従来のものの方が強い)
            let old_only = env_str!("ENOMOTO_MIP_PATH_OLD").is_some() || light;
            let highs_only = env_str!("ENOMOTO_MIP_PATH_HIGHS_ONLY").is_some();
            if !old_only {
                self.path_aggregation_highs(&vars, &lp_rows, &mut cands, &mut push);
            }
            if !highs_only || old_only {
                self.path_aggregation(&vars, &lp_rows, &mut cands, &mut push, max_starts);
            }
        }
        if dbg_sep {
            eprintln!("SEP path {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        // tableau 行
        let mut basics: Vec<(usize, f64)> = Vec::new();
        for s in 0..mr {
            let k = self.lp.basic_var(s);
            if k < n && self.cut_int[k] {
                let v = x[k];
                let f = v - v.floor();
                if f > 1e-3 && f < 1.0 - 1e-3 {
                    basics.push((s, f * (1.0 - f) / self.lp.dse_weight(s).max(1e-8)));
                }
            }
        }
        let nint = p.is_int.iter().filter(|&&b| b).count();
        let limit = if light { tunable!("ENOMOTO_T_MIP_NODE_TAB_ROWS", 50usize, usize) } else { 200 + (0.1 * (mr.min(nint)) as f64) as usize };
        // `ENOMOTO_MIP_TAB_HIGHS_RULES`: HiGHS の HighsTableauSeparator と同じ行の選び方。`dse_weight` は 1 を返すだけなので
        // 既定のスコアは f(1-f) で、密な行 (基底逆行列の行の非零が多い) が不利にならない。HiGHS は
        // (1) 基底逆行列の行 w を先に全部計算し、スコア = f(1-f) / ||w||^2 (w_i は行 i の最大係数で拡大縮小) で並べる、
        // (2) w の最大/最小の比が 1e4 を超える行は使わない、(3) 集約で増える非零 (集約行の長さ - w の非零) の 10 倍が
        // 10000 + 列数 を超える行は使わない (fill の予算)、(4) 最初にカットが出た行のスコアを基準に、その 0.0025 倍
        // (カットが 50 本を超えたら 0.01 倍) を下回ったら止める。30n20b8 では集約行の平均非零が 4400-5000 で、200 行の
        // カット生成 + finish_cut に 1 ラウンド 0.45 秒かかっていた (HiGHS は tableau のカットを 1 本も作らず 0.002-0.012 秒)。
        // 実測 (根のループ全体の時間 / 最後の下界): neos-1456979 0.58 -> 0.38 秒 (154 のまま)、qnet1 0.54 -> 0.33 秒
        // (15654 -> 15745)、neos-911970 0.42 -> 0.28 秒 (47.26 のまま)、h80x6320d 2.00 -> 1.84 秒 (5936 -> 5926) だが、
        // 30n20b8 は 2.41 -> 2.08 秒で下界 151 -> 88.5 (fill の予算なしでも 102: 停止規則が密なカットを切り、こちらは
        // mod-k などの代わりに tableau 行の密なカットで下界を得ている)。既定では使わない
        let tab_rules = env_str!("ENOMOTO_MIP_TAB_HIGHS_RULES").is_some();
        let mut w_cache: Vec<Vec<f64>> = Vec::new();
        if tab_rules {
            // 行の最大係数 (LP 行、スラック列を含まない)
            let row_max: Vec<f64> = lp_rows.iter().map(|r| r.iter().fold(0.0f64, |m, &(_, a)| m.max(a.abs()))).collect();
            let mut scored: Vec<(usize, f64, Vec<f64>)> = Vec::new();
            for &(s, _) in &basics {
                let w = self.lp.basis_inverse_row(s);
                let (mut norm2, mut wmin, mut wmax, mut cnt) = (0.0f64, f64::INFINITY, 0.0f64, 0usize);
                for i in 0..mr {
                    let sw = row_max[i] * w[i].abs();
                    if sw <= 1e-6 {
                        continue;
                    }
                    wmin = wmin.min(sw);
                    wmax = wmax.max(sw);
                    norm2 += sw * sw;
                    cnt += 1;
                }
                if cnt <= 1 || wmax / wmin > 1e4 {
                    continue;
                }
                let k = self.lp.basic_var(s);
                let f = x[k] - x[k].floor();
                scored.push((s, f * (1.0 - f) / norm2, w));
            }
            scored.sort_by(|a, b| b.1.total_cmp(&a.1));
            scored.truncate(limit);
            basics = scored.iter().map(|&(s, sc, _)| (s, sc)).collect();
            w_cache = scored.into_iter().map(|(_, _, w)| w).collect();
        } else {
            basics.sort_by(|a, b| b.1.total_cmp(&a.1));
        }
        // `ENOMOTO_T_MIP_TAB_NNZ_BUDGET` = B (>0 で有効): 1 ラウンドの集約の手間 (w の非零の行の長さの合計) の上限を
        // B * (列数 + LP の非零) にする。BTRAN の直後に手間が分かるので、上限を超える (密な) 行は集約せずに飛ばし、
        // 後ろの疎な行で埋める (候補は limit の 3 倍まで見て、使う行は limit まで)。既定では使わない (0):
        // 30n20b8 は w の非零が平均 5 しかなく、LP の行そのものが密 (集約行 4400-7700 非零) で、根の最初のラウンドの
        // 下界 43 -> 81 は密な行からの 1 本のカットによる。予算をかけると分離は 0.3 -> 0.1 秒/ラウンドになるが、
        // 根の下界が 151 -> 92 (B=2), 109 (B=5), 105 (B=20) に下がる。neos-911970 は B=2,5 で根のラウンドが減り
        // 木の下界 53.1 -> 51 に悪化、qnet1 は 4.5 -> 3.0 秒と速くなり、neos-1456979・h80x6320d はほぼ変わらない
        let lp_nnz: usize = lp_rows.iter().map(|r| r.len()).sum();
        let nnz_budget_mult = tunable!("ENOMOTO_T_MIP_TAB_NNZ_BUDGET", 0.0, f64);
        let nnz_budget = if nnz_budget_mult > 0.0 { nnz_budget_mult * (n + lp_nnz) as f64 } else { f64::INFINITY };
        let mut work_used = 0.0f64;
        let mut n_budget_skip = 0usize;
        if !tab_rules {
            basics.truncate(if nnz_budget.is_finite() { 3 * limit } else { limit });
        }
        let mut agg = vec![0.0; n];
        let tab_fc = use_flow_cover() && env_str!("ENOMOTO_MIP_TAB_FC").is_some();
        // 診断 (`ENOMOTO_MIP_DEBUG_SEP`): tableau 行の内訳の時間 (BTRAN, 集約, カット生成 + finish_cut) と行の非零数
        let (mut t_btran, mut t_agg, mut t_gen) = (0.0f64, 0.0f64, 0.0f64);
        let (mut n_rows_used, mut w_nnz_total, mut base_nnz_total, mut n_fill_skip) = (0usize, 0usize, 0usize, 0usize);
        let cands_before_tab = cands.len();
        let mut best_score = -1.0f64;
        for (bi, &(s, score)) in basics.iter().enumerate() {
            if self.time_up() {
                break;
            }
            if tab_rules && best_score >= 0.0 {
                let made = cands.len() - cands_before_tab;
                let fac = if made >= 50 { 0.01 } else { 0.0025 };
                if score < fac * best_score {
                    break;
                }
            }
            if n_rows_used >= limit {
                break;
            }
            let t0 = std::time::Instant::now();
            let w = if tab_rules { std::mem::take(&mut w_cache[bi]) } else { self.lp.basis_inverse_row(s) };
            if dbg_sep {
                t_btran += t0.elapsed().as_secs_f64();
            }
            let t0 = std::time::Instant::now();
            let wmax = w.iter().fold(0.0f64, |m, v| m.max(v.abs()));
            if wmax == 0.0 {
                continue;
            }
            let wmin_keep = 1e-9 * wmax;
            if nnz_budget.is_finite() {
                let work: usize = (0..mr).filter(|&i| w[i].abs() > wmin_keep).map(|i| lp_rows[i].len() + 1).sum();
                if work_used + work as f64 > nnz_budget {
                    n_budget_skip += 1;
                    continue;
                }
                work_used += work as f64;
            }
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
            let w_nnz = w.iter().filter(|v| v.abs() > wmin_keep).count();
            if dbg_sep {
                t_agg += t0.elapsed().as_secs_f64();
                w_nnz_total += w_nnz;
            }
            if !ok {
                continue;
            }
            // HiGHS の fill の予算: 集約で増えた非零の 10 倍が 10000 + 列数 を超える行は使わない
            // (`ENOMOTO_T_MIP_TAB_FILL_MULT` 倍に緩める。0 なら予算なし。30n20b8 では HiGHS と同じ予算だと全ての行が外れ、
            // 根の下界が 151 -> 88.5 に下がった: HiGHS は mod-k など他の分離で下界を得るが、こちらは tableau 行に頼っている)
            let fill_mult = tunable!("ENOMOTO_T_MIP_TAB_FILL_MULT", 1.0, f64);
            if tab_rules && fill_mult > 0.0 && 10.0 * base.len().saturating_sub(w_nnz) as f64 > fill_mult * (10_000 + n) as f64 {
                n_fill_skip += 1;
                continue;
            }
            n_rows_used += 1;
            base_nnz_total += base.len();
            if !gen_old() {
                let t0 = std::time::Instant::now();
                let n_before = cands.len();
                let neg: Vec<(usize, f64)> = base.iter().map(|&(k, a)| (k, -a)).collect();
                for b in [&base, &neg] {
                    let n0 = cands.len();
                    for c in generate_cuts(&vars, b, 0.0, tab_fc, false) {
                        push(Some(c), &mut cands, self);
                    }
                    keep_best(&mut cands, n0, self.p);
                }
                if dbg_sep {
                    t_gen += t0.elapsed().as_secs_f64();
                }
                if tab_rules && best_score < 0.0 && cands.len() > n_before {
                    best_score = score;
                }
                continue;
            }
            let r = cmir(&vars, &base, 0.0);
            push(r, &mut cands, self);
            let neg: Vec<(usize, f64)> = base.iter().map(|&(k, a)| (k, -a)).collect();
            let r = cmir(&vars, &neg, 0.0);
            push(r, &mut cands, self);
            if use_lifted_cover() {
                push(lifted_cover(&vars, &base, 0.0), &mut cands, self);
                push(lifted_cover(&vars, &neg, 0.0), &mut cands, self);
            }
            if tab_fc {
                push(lifted_flow_cover(&vars, &base, 0.0), &mut cands, self);
                push(lifted_flow_cover(&vars, &neg, 0.0), &mut cands, self);
            }
        }
        if dbg_sep {
            eprintln!(
                "SEP tableau {:.3}s cands {} (rows {} of {} candidates, fill-skipped {}, budget-skipped {} (work {:.0} / {:.0}), btran {:.3}s agg {:.3}s gen+finish {:.3}s (finish total this call {:.3}s), mean w nnz {:.0}, mean agg nnz {:.0}, LP rows {} nnz {})",
                sep_t0.elapsed().as_secs_f64(),
                cands.len(),
                n_rows_used,
                basics.len(),
                n_fill_skip,
                n_budget_skip,
                work_used,
                nnz_budget,
                t_btran,
                t_agg,
                t_gen,
                t_finish.get(),
                w_nnz_total as f64 / basics.len().max(1) as f64,
                base_nnz_total as f64 / n_rows_used.max(1) as f64,
                mr,
                lp_rows.iter().map(|r| r.len()).sum::<usize>()
            );
        }
        cands
    }
}

/// 生のカット (構造変数と行の活動量の変数) を構造変数だけの式にし、整え、検査する。
fn finish_cut(
    raw: RawCut,
    n: usize,
    lp_rows: &[Vec<(usize, f64)>],
    is_int: &[bool],
    lo: &[f64],
    up: &[f64],
    x: &[f64],
    incumbent: Option<&[f64]>,
    scratch: &mut [f64],
) -> Option<Candidate> {
    // 構造変数ごとの係数を密な作業領域 (呼び出し側が長さ n の 0 で用意し、ここで 0 に戻す) に集める。
    // 以前は BTreeMap で、集約行の非零が数千のときは finish_cut が tableau 行のカット生成と同程度の時間を使っていた
    let mut touched: Vec<usize> = Vec::with_capacity(raw.coefs.len());
    for &(k, c) in &raw.coefs {
        if k < n {
            if scratch[k] == 0.0 {
                touched.push(k);
            }
            scratch[k] += c;
            if scratch[k] == 0.0 {
                scratch[k] = 1e-300;
            }
        } else {
            for &(j, a) in &lp_rows[k - n] {
                if scratch[j] == 0.0 {
                    touched.push(j);
                }
                scratch[j] += c * a;
                if scratch[j] == 0.0 {
                    scratch[j] = 1e-300;
                }
            }
        }
    }
    touched.sort_unstable();
    let mut dense: Vec<(usize, f64)> = Vec::with_capacity(touched.len());
    for &j in &touched {
        let v = scratch[j];
        scratch[j] = 0.0;
        dense.push((j, if v == 1e-300 { 0.0 } else { v }));
    }
    let mut rhs = raw.rhs;
    let cmax = dense.iter().fold(0.0f64, |m, &(_, v)| m.max(v.abs()));
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
    // 係数の締め付け (HiGHS の `HighsDomain::tightenCoefficients`): 最大活動量が右辺を delta 超えるなら、
    // 係数の絶対値が delta を超える整数列の係数を delta に下げる (x がその境界から 1 以上離れれば、残りの項の
    // 最大活動量だけで成り立つので、右辺も同じだけ動かしてよい)。`ENOMOTO_MIP_NO_CUT_TIGHTEN` で無効。
    if env_str!("ENOMOTO_MIP_NO_CUT_TIGHTEN").is_none() {
        let mut maxact = 0.0;
        let mut fin = true;
        for &(j, c) in &coefs {
            let m = if c > 0.0 { c * up[j] } else { c * lo[j] };
            if !m.is_finite() {
                fin = false;
                break;
            }
            maxact += m;
        }
        let delta = maxact - rhs;
        if fin && delta > 1e-6 * (1.0 + rhs.abs()) {
            for (j, c) in coefs.iter_mut() {
                if is_int[*j] && c.abs() > delta + 1e-9 {
                    if *c > 0.0 {
                        rhs -= (*c - delta) * up[*j];
                        *c = delta;
                    } else {
                        rhs += (-*c - delta) * lo[*j];
                        *c = -delta;
                    }
                }
            }
        }
    }
    let cmax = coefs.iter().fold(0.0f64, |m, &(_, c)| m.max(c.abs()));
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
fn select_cuts(cands: Vec<Candidate>, max_cuts: usize, p: &MipProblem) -> Vec<Candidate> {
    if env_str!("ENOMOTO_MIP_CUTSEL_OLD").is_some() {
        return select_cuts_old(cands, max_cuts);
    }
    select_cuts_hybrid(cands, max_cuts, p)
}

/// 疎なベクトル (列番号の昇順) の内積。
fn sparse_dot(a: &[(usize, f64)], b: &[(usize, f64)]) -> f64 {
    let (mut i, mut k, mut dot) = (0, 0, 0.0);
    while i < a.len() && k < b.len() {
        let (x, y) = (a[i].0, b[k].0);
        if x == y {
            dot += a[i].1 * b[k].1;
            i += 1;
            k += 1;
        } else if x < y {
            i += 1;
        } else {
            k += 1;
        }
    }
    dot
}

/// SCIP の `cutsel_hybrid` に倣ったカット選択: スコア = 効き目 + 0.1 × 目的関数との平行度 + 0.1 × 整数列の割合。
/// スコアの高い順に採り、採ったカットとの平行度 (|cos|) が 0.3 を超えるものは捨てる。ただしスコアが最良の
/// 0.9 倍以上の「良い」カットは平行度 0.7 まで許す (SCIP の既定は 0.1 / 0.5 だが、こちらはカットのラウンド数が
/// 少ないので緩めにした方が良かった)。
fn select_cuts_hybrid(cands: Vec<Candidate>, max_cuts: usize, p: &MipProblem) -> Vec<Candidate> {
    let cnorm = p.cost.iter().map(|c| c * c).sum::<f64>().sqrt();
    let density_exp = tunable!("ENOMOTO_T_CUTSEL_DENSITY_EXP", 0.0, f64);
    let mut scored: Vec<(f64, f64, Candidate)> = cands
        .into_iter()
        .map(|c| {
            let nc = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
            // 密なカットを避ける (`ENOMOTO_T_CUTSEL_DENSITY_EXP` = α: スコアを 非零の数^α で割る。HiGHS は効いている
            // 非零の数で割る (α = 1 相当)。密なカットは LP の反復が増え、カットの行から作る次のカットも密になる)
            let score = cut_quality(&c, p, cnorm) / (c.coefs.len().max(1) as f64).powf(density_exp);
            (score, nc, c)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let Some(best_score) = scored.first().map(|s| s.0) else { return Vec::new() };
    let (maxpar, goodmaxpar, good) = (tunable!("ENOMOTO_T_CUTSEL_MAXPAR", 0.3, f64), tunable!("ENOMOTO_T_CUTSEL_GOODMAXPAR", 0.7, f64), 0.9 * best_score);
    // `ENOMOTO_T_CUTSEL_MIN_REL` (既定 0.5): 質が最良の何倍未満のカットは採らない (Wesselmann・Suhl の MOPS と同じ
    // 最良の 50%)。弱いカットを LP に入れず、1 ラウンドの LP を軽くする (根の LP の反復: qnet1 1936 -> 1149、
    // h80x6320d 7202 -> 3378)。40 問 2 回: sgeomean 27.84 -> 26.72、28.10 -> 26.81 (misc07 36 -> 11 秒、mik-250 33 -> 25 秒)。
    // 0.3 は 27.13。0 なら以前と同じ (上限 50 本まで全部)
    let min_score = tunable!("ENOMOTO_T_CUTSEL_MIN_REL", 0.5, f64) * best_score;
    let mut chosen: Vec<(f64, Candidate)> = Vec::new();
    for (score, nc, c) in scored {
        if chosen.len() >= max_cuts || score < min_score {
            break;
        }
        let limit = if score >= good { goodmaxpar } else { maxpar };
        let parallel = chosen.iter().any(|(nd, d)| sparse_dot(&c.coefs, &d.coefs).abs() > limit * nc * nd);
        if !parallel {
            chosen.push((nc, c));
        }
    }
    chosen.into_iter().map(|(_, c)| c).collect()
}

fn select_cuts_old(mut cands: Vec<Candidate>, max_cuts: usize) -> Vec<Candidate> {
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
