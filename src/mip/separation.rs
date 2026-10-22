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

thread_local! {
    /// `ENOMOTO_MIP_INJECT_ROUND=prog` で読んだ各行の HiGHS のラウンド (診断用)
    static INJ_ROUND: std::cell::RefCell<Vec<i64>> = const { std::cell::RefCell::new(Vec::new()) };
    /// `ENOMOTO_MIP_CLIQUE_CUTS=auto` の、最初の求解 (再スタート前) の判定。再スタートの後の求解が引き継ぐ
    static CLIQUE_AUTO_FIRST: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
    /// `ENOMOTO_MIP_CLIQUE_CUTS=fallback` で clique カットを加えて回す 2 回目のループの中か (時間の上限をループの始めから数える)
    static CLIQUE_FB_SECOND: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// `ENOMOTO_MIP_CUT_EXTEND` で根のカットの時間の上限を延ばした後か (延ばした後は面の頂点での分離を止める)
    static CUT_EXTENDED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
use super::lp_api::MipLp;

/// LP に加える前のカット (構造変数の係数、右辺、効き目)。
/// lifted flow cover を使うか (`ENOMOTO_MIP_NO_FLOWCOVER` で無効)。
/// HiGHS の lifted cover を CMIR と並べて試すか (`ENOMOTO_MIP_LIFTED_COVER` で有効)。qnet1 では根の下界が
/// 15627 -> 15924 (HiGHS 16022) と上がったが、40 問では 19 -> 17 問 (sgeomean 27.91 -> 29.06) と悪化した
/// (misc07・neos5・binkar10_1・neos-860300 が遅くなった) ので既定では使わない。
/// clique カットの使い方 (`ENOMOTO_MIP_CLIQUE_CUTS`)。既定は `fallback` (根のループを clique なしで回し、ほかのカットで伸びず
/// clique カットで伸びる問題でだけ根の続きで使う、[`Solver::clique_fallback_trial`])。`off` (か `0`) で使わない。
/// 40 問 + 16 問 (4 スレッド・60 秒、`ENOMOTO_T_MIP_CLIQUE_FB_FRAC=0.9` と組んで 2 組) で drayage-25-23 が 2 組とも解け、
/// 40 問は解けた数・不一致とも変わらない。手元 16 スレッドでも確かめて既定にした
pub(super) fn clique_cuts_mode() -> Option<&'static str> {
    match env_str!("ENOMOTO_MIP_CLIQUE_CUTS") {
        Some("off") | Some("0") => None,
        Some(m) => Some(m),
        None => Some("fallback"),
    }
}

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

/// 診断 (`ENOMOTO_MIP_INJECT_CUTS=ファイル`): HiGHS の計装版 (`HIGHS_LPCUT_DUMP`) が書き出した、根のカットのループの
/// ラウンドごとの LP のカットの行を読む。`ENOMOTO_MIP_INJECT_ROUND=k` のブロック (`R k ...`、既定はファイルの最後の
/// ブロック = ループの後の LP) の行を `係数 . x <= 右辺` の形で返す。列の番号は元の問題のもので、前処理なし
/// (`ENOMOTO_MIP_NO_PRESOLVE`) の求解でだけ使う (列の数が合わなければ空)。
fn load_injected_cuts(n: usize, var: &str) -> Vec<(Vec<(usize, f64)>, f64)> {
    let Some(path) = std::env::var(var).ok() else { return Vec::new() };
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("INJECT: cannot read {path}");
        return Vec::new();
    };
    let want: Option<i64> = if var == "ENOMOTO_MIP_INJECT_CAND" { None } else { env_str!("ENOMOTO_MIP_INJECT_ROUND").and_then(|v| v.parse().ok()) };
    let mut blocks: Vec<(i64, Vec<(Vec<(usize, f64)>, f64)>)> = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("R") => {
                let k: i64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(-1);
                blocks.push((k, Vec::new()));
            }
            Some("C") => {
                let Some(b) = blocks.last_mut() else { continue };
                let lo: f64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(f64::NEG_INFINITY);
                let up: f64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(f64::INFINITY);
                let len: usize = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                let mut coefs = Vec::with_capacity(len);
                let mut bad = false;
                for _ in 0..len {
                    let j: usize = it.next().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
                    let a: f64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    if j >= n {
                        bad = true;
                    }
                    coefs.push((j, a));
                }
                if bad {
                    continue;
                }
                if up < 1e29 {
                    b.1.push((coefs.clone(), up));
                }
                if lo > -1e29 {
                    b.1.push((coefs.iter().map(|&(j, a)| (j, -a)).collect(), -lo));
                }
            }
            _ => {}
        }
    }
    // `ENOMOTO_MIP_INJECT_ROUND=prog`: 最初のループのブロック (1..999) の行を順に、ブロックの番号を付けて返す ([`Candidate`] に
    // する側が、こちらのラウンド r ではブロック r+1 までのものだけを使う)
    if env_str!("ENOMOTO_MIP_INJECT_ROUND") == Some("prog") && var == "ENOMOTO_MIP_INJECT_CUTS" {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for (k, b) in blocks.into_iter().filter(|b| b.0 >= 1 && b.0 < 1000) {
            for (c, r) in b {
                let key = (c.iter().map(|&(j, a)| (j, a.to_bits())).collect::<Vec<_>>(), r.to_bits());
                if seen.insert(key) {
                    INJ_ROUND.with(|v| v.borrow_mut().push(k));
                    out.push((c, r));
                }
            }
        }
        eprintln!("INJECT: {} cuts from {path} (progressive)", out.len());
        return out;
    }
    let pick = match want {
        Some(k) => blocks.into_iter().find(|b| b.0 == k),
        None => blocks.pop(),
    };
    let cuts = pick.map(|b| b.1).unwrap_or_default();
    eprintln!("INJECT: {} cuts from {path} (round {want:?})", cuts.len());
    cuts
}

struct Candidate {
    coefs: Vec<(usize, f64)>,
    rhs: f64,
    efficacy: f64,
    /// 根の選択を通さずに LP に入れる (implied bound カット)。
    forced: bool,
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

    /// clique カット用の衝突グラフ (根で一度だけ作って使い回す)。
    fn clique_graph_get(&mut self) -> std::rc::Rc<super::clique::CliqueGraph> {
        let p = self.p;
        if self.clique_graph.is_none() {
            let binary: Vec<bool> = (0..p.n).map(|j| p.is_int[j] && self.dom.global_lo[j] >= 0.0 && self.dom.global_up[j] <= 1.0).collect();
            let g = super::clique::CliqueGraph::build_with(p.n, &p.rows, &p.row_lo, &p.row_up, &binary, &self.dom.global_lo, &self.dom.global_up, &p.conflict_edges);
            if self.params.verbose && !self.params.submip {
                let g0 = super::clique::CliqueGraph::build(p.n, &p.rows, &p.row_lo, &p.row_up, &binary, &self.dom.global_lo, &self.dom.global_up);
                eprintln!("MIP: clique graph: {} edges ({} from rows, probing implications {})", g.num_edges(), g0.num_edges(), p.conflict_edges.len());
            }
            self.clique_graph = Some(std::rc::Rc::new(g));
        }
        self.clique_graph.clone().unwrap()
    }

    /// `ENOMOTO_MIP_CLIQUE_CUTS=auto` の判定 (試作): 根の LP 解で破れる clique カットだけを LP に入れて解き直し、目的値の伸びが
    /// `ENOMOTO_T_MIP_CLIQUE_AUTO_GAIN` (既定 1e-3) x max(1, |目的値|) を超えれば使う。試した行は外し、LP の状態を戻す。
    /// drayage-25-23 では 99718.79 -> 100121.99 (4e-3、HiGHS の根の下界と同じ)。clique カットを常に使うと misc07 の根は
    /// 1535.54 -> 1536.21 しか上がらないのに木が変わって 7 秒 -> 10 秒に遅くなる
    fn clique_trial(&mut self, root_iters: u64) -> bool {
        let t0 = std::time::Instant::now();
        let Some((obj0, obj1, nfrac0, nfrac1, ncuts)) = self.clique_trial_lp(root_iters) else {
            if self.params.verbose {
                eprintln!("MIP: clique trial: no violated clique cuts; not used ({:.2}s)", t0.elapsed().as_secs_f64());
            }
            return false;
        };
        let thr = tunable!("ENOMOTO_T_MIP_CLIQUE_AUTO_GAIN", 1e-3, f64) * obj0.abs().max(1.0);
        // `ENOMOTO_T_MIP_CLIQUE_AUTO_FRAC` = r (> 0): 分数の列の数が r 倍以下に減るときも使う (目的値が既に最適値で動かない
        // drayage-100-23 でも、clique カットで LP 解が整数に近づき根の RENS が最適解を見つける)
        let frac_r = tunable!("ENOMOTO_T_MIP_CLIQUE_AUTO_FRAC", 0.0, f64);
        let on = obj1 - obj0 > thr || (frac_r > 0.0 && (nfrac1 as f64) <= frac_r * nfrac0 as f64);
        if self.params.verbose {
            eprintln!("MIP: clique trial: {ncuts} cuts, LP {obj0:.10e} -> {obj1:.10e}, fractional {nfrac0} -> {nfrac1}; {} ({:.2}s)", if on { "used" } else { "not used" }, t0.elapsed().as_secs_f64());
        }
        on
    }

    /// 今の LP 解で破れる clique カットだけを LP に入れて解き直し、(目的値, 入れた後の目的値, 分数の列の数, 入れた後の数, カットの数)
    /// を返す (破れるカットがなければ `None`)。試した行は外し、LP の状態を戻す
    fn clique_trial_lp(&mut self, root_iters: u64) -> Option<(f64, f64, usize, usize, usize)> {
        let g = self.clique_graph_get();
        let x = self.lp.col_values();
        let cuts = g.separate(&x, tunable!("ENOMOTO_T_MIP_CLIQUE_MAX", 100usize, usize));
        if cuts.is_empty() {
            return None;
        }
        let obj0 = self.lp.objective();
        let nfrac0 = self.fractional(&x).len();
        let saved = self.lp.save_state();
        let m0 = self.lp.num_rows();
        let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = cuts.iter().map(|c| (c.coefs.clone(), f64::NEG_INFINITY, c.rhs)).collect();
        self.lp.add_rows(&rows);
        let st = self.lp.solve(&SolveLimits { iteration_limit: 10 * root_iters.max(100) + 10_000, cutoff: f64::INFINITY, deadline: self.deadline });
        let obj1 = if st == LpStatus::Optimal { self.lp.objective() } else { obj0 };
        let nfrac1 = if st == LpStatus::Optimal { self.fractional(&self.lp.col_values()).len() } else { nfrac0 };
        let mut remove = vec![false; self.lp.num_rows()];
        for r in remove.iter_mut().skip(m0) {
            *r = true;
        }
        self.lp.delete_rows(&remove);
        self.lp.restore_state(&saved);
        Some((obj0, obj1, nfrac0, nfrac1, cuts.len()))
    }

    /// `ENOMOTO_MIP_CLIQUE_CUTS=fallback` の判定 (試作): 根の切除平面ループを clique カットなしで回し終えた LP で、破れる clique
    /// カットだけを入れた LP の伸び g_c を測り、ループ全体の伸び g_d (clique なしのカットでの伸び) と比べる。
    /// g_c > `ENOMOTO_T_MIP_CLIQUE_FB_GAIN` (既定 1e-4) x max(1, |目的値|) かつ g_c > `ENOMOTO_T_MIP_CLIQUE_FB_RATIO` (既定 0.5) x g_d
    /// なら使う (ほかのカットで伸びない問題でだけ使う)。drayage-25-23 は g_d = 0、30n20b8 は g_d = 15 に対し g_c は 1 程度
    fn clique_fallback_trial(&mut self, root_iters: u64, first_obj: f64) -> bool {
        let t0 = std::time::Instant::now();
        let Some((obj0, obj1, nfrac0, nfrac1, ncuts)) = self.clique_trial_lp(root_iters) else {
            if self.params.verbose {
                eprintln!("MIP: clique fallback: no violated clique cuts; not used ({:.2}s)", t0.elapsed().as_secs_f64());
            }
            return false;
        };
        let (gc, gd) = (obj1 - obj0, obj0 - first_obj);
        let scale = obj0.abs().max(1.0);
        let on = gc > tunable!("ENOMOTO_T_MIP_CLIQUE_FB_GAIN", 1e-4, f64) * scale && gc > tunable!("ENOMOTO_T_MIP_CLIQUE_FB_RATIO", 0.5, f64) * gd;
        // `ENOMOTO_T_MIP_CLIQUE_FB_FRAC` = r (> 0、既定 0.9。0 で使わない): ほかのカットで目的値がまったく動かず (g_d <= 1e-9)、clique カットで分数の列の数が
        // r 倍以下に減るときも使う (drayage-100-23 は根の LP 値が最適値で g_c = 0。clique カットで LP 解が整数に近づき根の RENS が効く)
        let frac_r = tunable!("ENOMOTO_T_MIP_CLIQUE_FB_FRAC", 0.9, f64);
        let on = on || (frac_r > 0.0 && gd <= 1e-9 * scale && (nfrac1 as f64) <= frac_r * nfrac0 as f64);
        if self.params.verbose {
            eprintln!(
                "MIP: clique fallback: {ncuts} cuts, LP {obj0:.10e} -> {obj1:.10e} (g_c {:.3e}, g_d {:.3e} relative), fractional {nfrac0} -> {nfrac1}; {} ({:.2}s)",
                gc / scale,
                gd / scale,
                if on { "used" } else { "not used" },
                t0.elapsed().as_secs_f64()
            );
        }
        on
    }

    /// 根の切除平面ループ。LP は根の最適解の状態で呼ぶこと。終わったときも LP は最適 (でなければ偽)。
    /// `ENOMOTO_MIP_EQUIV2_CUTS` (試作、既定では使わない): 前処理が `ENOMOTO_MIP_PROBE_EQUIV2` で同値な 0-1 列を多数代入した問題
    /// ([`super::EQUIV2_USED`]) だけ、根のカットのループの予算を広げる: 面の頂点での分離を止め、根のカットの時間の上限を全体の
    /// `ENOMOTO_T_MIP_EQUIV2_CUT_TIME_FRAC` (既定 0.3)、1 回の LP の上限を `ENOMOTO_T_MIP_EQUIV2_CUT_LP_TIME_FRAC` (既定 0.05) にし、
    /// LP が上限で止まってもカットを外さずに解き続ける (`ENOMOTO_MIP_CUT_LP_CONTINUE` と同じ)。physiciansched6-2 は代入の後に
    /// tableau のカットで根の下界が伸び続けるが (HiGHS も Probing の代入の後のカットで根を閉じる)、既定の予算では 1〜2 ラウンドで止まる
    pub(super) fn equiv2_cuts(&self) -> bool {
        !self.params.submip && env_str!("ENOMOTO_MIP_EQUIV2_CUTS").is_some() && super::EQUIV2_USED.with(|f| f.get())
    }

    /// 根のカットのループの 1 回の LP の時間の上限 (全体に対する割合)。
    fn cut_lp_time_frac(&self) -> f64 {
        if self.equiv2_cuts() {
            tunable!("ENOMOTO_T_MIP_EQUIV2_CUT_LP_TIME_FRAC", 0.05, f64)
        } else {
            tunable!("ENOMOTO_T_MIP_CUT_LP_TIME_FRAC", 0.02, f64)
        }
    }

    pub(super) fn root_cut_loop(&mut self, root_iters: u64) -> bool {
        let p = self.p;
        let n = p.n;
        let mut stall = 0;
        let mut prev_obj = self.lp.objective();
        let mut prev_nfrac = self.fractional(&self.lp.col_values()).len();
        let first_obj = prev_obj;
        if clique_cuts_mode() == Some("auto") && self.clique_auto.is_none() && !self.params.submip {
            // 再スタートの後は最初の求解の判定を引き継ぐ (再スタートの根は前の根のカットを持つので、試しても伸びない)
            let on = match CLIQUE_AUTO_FIRST.with(|c| c.get()) {
                Some(v) if self.params.restarts > 0 => v,
                _ => self.clique_trial(root_iters),
            };
            if self.params.restarts == 0 {
                CLIQUE_AUTO_FIRST.with(|c| c.set(Some(on)));
            }
            self.clique_auto = Some(on);
        }
        // `fallback`: ループの後で判定する ([`Self::clique_fallback_trial`])。再スタートの後は最初の求解の判定を引き継ぐ
        let mut clique_fb = clique_cuts_mode() == Some("fallback") && self.clique_auto.is_none() && !self.params.submip;
        if clique_fb && self.params.restarts > 0 {
            if let Some(v) = CLIQUE_AUTO_FIRST.with(|c| c.get()) {
                self.clique_auto = Some(v);
                clique_fb = false;
            }
        }
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
        // `ENOMOTO_T_MIP_SUBMIP_CUT_ROUNDS` / `..._STALL` (試作、既定 5 / 3): サブ MIP の根のカットのラウンドの上限と停滞の回数。
        // beasleyC3 の根の RENS の近傍 (最適値 763・770) は、通常の求解 (25 ラウンド) なら根の下界が 760〜762 に上がって根で
        // 解けるが、5 ラウンドのサブ MIP は 500 ノードで止まり 773〜779 だった。HiGHS のサブ MIP は通常のカットのループを回し、
        // 同じ形の近傍を 1 ノードで解く (analysis/beasleyC3_r3_20261020.md)
        let fc_sub = self.params.submip && self.params.fc_submip;
        let mut max_rounds = if fc_sub { tunable!("ENOMOTO_T_MIP_FC_SUBMIP_CUT_ROUNDS", 25usize, usize) } else if self.params.submip { tunable!("ENOMOTO_T_MIP_SUBMIP_CUT_ROUNDS", 5usize, usize) } else { tunable!("ENOMOTO_T_MIP_CUT_ROUNDS", 25usize, usize) };
        let eq2c = self.equiv2_cuts();
        let cut_frac = if eq2c { tunable!("ENOMOTO_T_MIP_EQUIV2_CUT_TIME_FRAC", 0.3, f64) } else { tunable!("ENOMOTO_T_MIP_CUT_TIME_FRAC", 0.1, f64) };
        let mut time_cap = if self.params.time_limit.is_finite() { cut_frac * self.params.time_limit } else { f64::INFINITY };
        // 根のヒューリスティクスの間にカットを強める求解: 止めの合図まで続ける (ラウンド数・時間の上限なし)
        let job = super::solver::CUT_JOB.with(|j| j.borrow().clone()).filter(|_| self.snap.is_some());
        if job.is_some() {
            max_rounds = tunable!("ENOMOTO_T_MIP_BG_CUTS_MAX_ROUNDS", 1000usize, usize);
            time_cap = f64::INFINITY;
        }
        let heavy = job.as_ref().is_some_and(|j| j.heavy);
        let stopped = |j: &Option<std::sync::Arc<super::solver::CutJob>>| j.as_ref().is_some_and(|j| j.stop.load(std::sync::atomic::Ordering::Relaxed));
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
        let phase2_on = (heavy || env_str!("ENOMOTO_MIP_CUT_PHASE2").is_some() || (self.highs_cutmgmt() && env_str!("ENOMOTO_MIP_HCM_NO_PHASE2").is_none())) && !self.params.submip;
        let mut phase2 = false;
        // 第 2 段階の時間の上限 (第 1 段階に使った時間の `ENOMOTO_T_MIP_CUT_PHASE2_TIME_MULT` 倍。qnet1 では再スタート後の
        // 第 2 段階が 88 ラウンド回って根が 6 秒伸び、9.2 秒 -> 17.2 秒になった)
        // 上限はこのスレッドの CPU 時間で測る ([`super::cap_clock`]。求解の開始からの分は `cpu0` から)
        let t_loop0 = super::cap_clock();
        // fallback の 2 回目のループ: 1 回目で根のカットの時間の上限 (`ENOMOTO_T_MIP_CUT_TIME_FRAC`) を使い切っていても回るよう、
        // 上限をこのループの始めから数える (drayage-25-23 では 1 回目に 6.9 秒かかり、2 回目が 1 ラウンドも回らなかった)
        let cap_base = if CLIQUE_FB_SECOND.with(|c| c.get()) && env_str!("ENOMOTO_MIP_CLIQUE_FB_SHARED_CAP").is_none() { t_loop0 } else { self.cpu0 };
        let mut phase2_deadline = f64::INFINITY;
        let mut round_ctr = 0usize;
        // 診断 (`ENOMOTO_MIP_DEBUG_FACETIME`): このループの分離・面の頂点の分離・LP の時間の合計
        let face0 = (self.face_secs, self.face_used);
        let (mut sep_tot, mut lp_tot, mut nrounds) = (0.0f64, 0.0f64, 0usize);
        let t_loop_wall = std::time::Instant::now();
        // 固定費用ネットワーク型 (連続変数のほぼ全てに VUB `x <= u y` の 2 非零の行があり、その行が大半) では、根のカット選択を
        // 緩める (質の閾値なし、平行度の上限 0.9)。flow cover 系のカットは同じ y の係数が支配的で互いに平行と判定されやすく、
        // 既定の選択では 1 ラウンドに数本しか入らない (sp150x300d: 候補 456 本 -> 31 本、根の下界 56.9 (HiGHS 68.7))。
        // 固定費用型 18 問 (beasleyC1-3, mc7/8/11, h50x2450, h80x6320d, sp150x300d, p200x1188c, aflow30a/40b, rout など、60 秒) で
        // SGM 29.16 -> 27.00、29.09 -> 27.49 (2 回)、解けた問題 9 -> 10。`ENOMOTO_MIP_NO_CUTSEL_FC` で無効
        let fc_net = !self.params.submip && fixed_charge_network(p);
        self.fc_network = fc_net;
        let fc_relax = env_str!("ENOMOTO_MIP_NO_CUTSEL_FC").is_none() && fc_net;
        if fc_relax && self.params.verbose {
            eprintln!("MIP: fixed-charge network structure: relaxed root cut selection");
        }
        // `ENOMOTO_MIP_CUT_ADAPT_MAX` (実験的): 採ったカットが 1 ラウンドの上限に達したら、次のラウンドの上限を 2 倍にする
        // (`ENOMOTO_T_MIP_CUT_ADAPT_MAX_CAP` 本まで)。同じ形のブロックが多数ある問題 (neos-3381206-awhea: 475 個の箱) では
        // 毎ラウンド候補が数千本あり、50 本ずつでは LP が切られていない別のブロックに移るだけで下界が動かない
        let adapt_max = env_str!("ENOMOTO_MIP_CUT_ADAPT_MAX").is_some() && !self.params.submip;
        let mut adapt_max_cuts = 0usize;
        // 上限まで採っても目的値が動かないラウンドが来たら、1 ラウンドの上限を行の数 (`ENOMOTO_T_MIP_CUT_WIDE_MAX` 本まで) に
        // 広げ、ラウンドの上限を `ENOMOTO_T_MIP_CUT_WIDE_ROUNDS` 以上にし、それ以降はまったく進まないラウンドも停滞 1 回と数える
        // (`ENOMOTO_MIP_NO_CUT_WIDEN` で無効)。同じ形のブロックが多数ある問題 (neos-3381206-awhea: 容量 100 の箱 475 個) では
        // 毎ラウンド候補が数千本あり、50 本ずつ切っても LP が切られていない別の箱に移るだけで下界が 415.24 から動かず、
        // 停滞判定で 5 ラウンドで止まっていた。下界が上がっている問題では起きないので、上限を一律に広げる
        // (40 問で悪化した) のと違い他の問題は変わらない
        let widen_on = env_str!("ENOMOTO_MIP_NO_CUT_WIDEN").is_none() && !self.params.submip;
        let mut widened = false;
        // `ENOMOTO_MIP_CUT_ADAPT_ROUNDS` (実験的): ラウンドの上限に達したとき、直近 5 ラウンドの目的値の伸びがそれまでの伸びの
        // `ENOMOTO_T_MIP_CUT_ADAPT_ROUNDS_FRAC` 以上なら上限を 25 ラウンド延ばす (`ENOMOTO_T_MIP_CUT_ADAPT_ROUNDS_CAP` まで)
        let adapt_rounds = env_str!("ENOMOTO_MIP_CUT_ADAPT_ROUNDS").is_some() && !self.params.submip;
        let mut obj_hist: Vec<f64> = vec![prev_obj];
        // カットを加える前の根の LP の値 (`ENOMOTO_MIP_CUT_STRONG_AUTO` の判定用)
        let cut_lp0 = prev_obj;
        // `ENOMOTO_T_MIP_CUT_PROG_WIN` = W (> 0): 直近 W ラウンドの目的値の伸びが、ループ全体の伸びの
        // `ENOMOTO_T_MIP_CUT_PROG_EPS` 倍未満なら止める (進み具合による停滞判定。伸びが続く間はラウンド数の上限まで回す)
        let prog_win = if self.params.submip { 0 } else { tunable!("ENOMOTO_T_MIP_CUT_PROG_WIN", 0usize, usize) };
        let prog_eps = tunable!("ENOMOTO_T_MIP_CUT_PROG_EPS", 0.01, f64);
        let mut obj_hist: Vec<f64> = Vec::new();
        let extend_on = env_str!("ENOMOTO_MIP_CUT_EXTEND").is_some() && !self.params.submip && job.is_none() && self.params.time_limit.is_finite();
        let extend_max = tunable!("ENOMOTO_T_MIP_CUT_EXTEND_MAX", 0.3, f64) * self.params.time_limit;
        CUT_EXTENDED.with(|c| c.set(false));
        // ラウンドごとの目的値 (`ENOMOTO_MIP_CUT_EXTEND` の判定用。最初は根の LP の値)
        let mut ext_hist: Vec<f64> = vec![first_obj];
        // `ENOMOTO_MIP_CUTSEL_FEEDBACK` (試作): 既定の選択の質の閾値 (最良の `ENOMOTO_T_CUTSEL_MIN_REL` 倍) を、前のラウンドの
        // 目的値の伸びで変える。伸びが |目的値| の `ENOMOTO_T_CUTSEL_FB_TAU` 倍 (1e-3) 未満なら閾値を半分に (`ENOMOTO_T_CUTSEL_FB_FLOOR`
        // (0.05) まで)、それ以上なら元に戻す。退化した LP では効き目の大きい疎なカット (roll3000 の x_i - x_j <= 0) が目的値を
        // 動かさず、それが質の基準になって、効き目は 0.1 程度でも目的値を大きく動かす密なカットが閾値で落ちる
        // (analysis/cutloop_hl_20261022.md: HiGHS のラウンド 3 の LP で、HiGHS が入れた 22 本のうち閾値で落ちる 13 本に、単独で +193・+100・+30 の
        // カットがある。閾値なしなら HiGHS と同じ 11648、既定の閾値では 11349)
        let fb_on = !self.params.submip && env_str!("ENOMOTO_MIP_CUTSEL_FEEDBACK").is_some();
        let fb_base = tunable!("ENOMOTO_T_CUTSEL_MIN_REL", 0.5, f64);
        let mut fb_rel = fb_base;
        // 診断 (`ENOMOTO_MIP_INJECT_CUTS`、[`load_injected_cuts`]): HiGHS の根のカットを、最初の求解の根のループで使う。
        // `ENOMOTO_MIP_INJECT_MODE=lp` (既定): ループの前に全部 LP に入れて解く (その後は既定のループ)。`cand`: 毎ラウンド、
        // 今の LP 解を切るものを候補に混ぜて既定の選択に通す (`ENOMOTO_MIP_INJECT_ONLY` ならこちらの分離の候補は捨てる)
        // `ENOMOTO_MIP_INJECT_CAND=ファイル`: 候補に混ぜるカットを別のファイル (最後のブロック) から読む (`ENOMOTO_MIP_INJECT_CUTS` は LP に入れる方)
        let inject_ok = !self.params.submip && job.is_none() && self.params.restarts == 0;
        let inject_on = inject_ok && env_str!("ENOMOTO_MIP_INJECT_CUTS").is_some();
        let cand_file = inject_ok && env_str!("ENOMOTO_MIP_INJECT_CAND").is_some();
        let inject_cand = cand_file || (inject_on && env_str!("ENOMOTO_MIP_INJECT_MODE") == Some("cand"));
        let inject_only = inject_cand && env_str!("ENOMOTO_MIP_INJECT_ONLY").is_some();
        let lp_cuts = if inject_on && !(inject_cand && !cand_file) { load_injected_cuts(n, "ENOMOTO_MIP_INJECT_CUTS") } else { Vec::new() };
        let inj_cuts = if cand_file { load_injected_cuts(n, "ENOMOTO_MIP_INJECT_CAND") } else if inject_cand { load_injected_cuts(n, "ENOMOTO_MIP_INJECT_CUTS") } else { Vec::new() };
        let inj_keys: std::collections::HashSet<(usize, u64, usize)> = inj_cuts.iter().map(|(c, r)| (c.len(), r.to_bits(), c.first().map_or(0, |e| e.0))).collect();
        if !lp_cuts.is_empty() {
            let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = lp_cuts.iter().map(|(c, r)| (c.clone(), f64::NEG_INFINITY, *r)).collect();
            let it0 = self.lp.total_iterations();
            let t0 = std::time::Instant::now();
            self.add_cut_rows(&rows);
            let st = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
            eprintln!("INJECT: added {} cuts to the root LP: {st:?}, obj {:.10e} ({} iters, {:.2}s)", rows.len(), self.lp.objective() + p.offset, self.lp.total_iterations() - it0, t0.elapsed().as_secs_f64());
            if st != LpStatus::Optimal {
                return false;
            }
            prev_obj = self.lp.objective();
            prev_nfrac = self.fractional(&self.lp.col_values()).len();
        }
        loop {
            self.poll_fj_thread();
            if self.root_gap_closed() {
                break;
            }
            if round_ctr >= max_rounds {
                let k = obj_hist.len();
                let extend = adapt_rounds
                    && max_rounds < tunable!("ENOMOTO_T_MIP_CUT_ADAPT_ROUNDS_CAP", 100usize, usize)
                    && k > 5
                    && obj_hist[k - 1] - obj_hist[k - 6] >= tunable!("ENOMOTO_T_MIP_CUT_ADAPT_ROUNDS_FRAC", 0.05, f64) * (obj_hist[k - 1] - obj_hist[0]).max(1e-9);
                // `ENOMOTO_MIP_CUT_PHASE2_AT_CAP`: 第 1 段階がラウンドの上限に当たって止まるときは (停滞していなくても)、
                // 第 2 段階 (HiGHS のスコア) へ進む (`ENOMOTO_MIP_CUT_PHASE2` がなくても。停滞で止まるときは既定のまま止める)。
                // timtab1 は既定の選択で 25 ラウンド目まで毎回 1-3% 伸び、停滞の判定に届かないので、`ENOMOTO_MIP_CUT_PHASE2`・
                // `ENOMOTO_MIP_HIGHS_CUTMGMT` でも第 2 段階に入らない (根の下界 495790、第 2 段階に入ると 598k-602k、HiGHS 569k-610k)
                let to_phase2 = !extend && !phase2 && !self.params.submip && env_str!("ENOMOTO_MIP_CUT_PHASE2_AT_CAP").is_some();
                // `ENOMOTO_MIP_CUT_STRONG_AUTO` (既定で使う。`ENOMOTO_MIP_NO_CUT_STRONG_AUTO` で止める): 第 1 段階がラウンドの上限に当たったとき、カットが根の LP の値を大きく
                // (`ENOMOTO_T_MIP_CUT_STRONG_FRAC` (既定 0.3) × max(|LP|, |今の値|) 以上) 上げていて、直近 5 ラウンドもまだ全体の伸びの
                // `ENOMOTO_T_MIP_CUT_STRONG_RECENT` (既定 0.01) 以上伸びているなら、第 2 段階へ進み (`CUT_PHASE2_AT_CAP` と同じ)、
                // 以後 HiGHS 流のカット管理 (`ENOMOTO_MIP_HIGHS_CUTMGMT`) を有効にする (値が `cap` なら第 2 段階だけ)。
                // timtab1 は 163277 -> 495790 (25 ラウンド目まで毎回伸びる)。40 問で上限に当たるほかの問題の伸びは 1 割未満
                let strong_env = env_str!("ENOMOTO_MIP_CUT_STRONG_AUTO");
                let strong_on = strong_env.is_some() || env_str!("ENOMOTO_MIP_NO_CUT_STRONG_AUTO").is_none();
                let strong = !extend && !phase2 && !self.params.submip && k > 5 && strong_on && {
                    // 大きさは元の問題の目的値 (前処理の定数項を足した値) で測る
                    let (z0, z1) = (cut_lp0 + p.offset, obj_hist[k - 1] + p.offset);
                    let gain = z1 - z0;
                    z0.is_finite()
                        && gain >= tunable!("ENOMOTO_T_MIP_CUT_STRONG_FRAC", 0.3, f64) * z0.abs().max(z1.abs()).max(1.0)
                        && obj_hist[k - 1] - obj_hist[k - 6] >= tunable!("ENOMOTO_T_MIP_CUT_STRONG_RECENT", 0.01, f64) * gain
                };
                if strong {
                    if strong_env.is_none_or(|v| v != "cap") {
                        self.hcm_auto = true;
                    }
                    if self.params.verbose {
                        eprintln!("MIP: strong cuts at the round limit: LP {:.6e} -> {:.6e}, HiGHS cut management {}", cut_lp0 + p.offset, obj_hist[k - 1] + p.offset, self.hcm_auto);
                    }
                }
                let to_phase2 = to_phase2 || strong;
                if to_phase2 {
                    phase2 = true;
                    stall = 0;
                    max_rounds = round_ctr + tunable!("ENOMOTO_T_MIP_CUT_PHASE2_ROUNDS", 60usize, usize);
                    let t1 = super::cap_clock() - t_loop0;
                    phase2_deadline = t1 + (tunable!("ENOMOTO_T_MIP_CUT_PHASE2_TIME_MULT", 3.0, f64) * t1).max(0.2);
                    if self.params.verbose {
                        eprintln!("MIP: cut loop phase 2 (HiGHS cut score) at the round limit {round_ctr} ({:.2}s)", self.start.elapsed().as_secs_f64());
                    }
                } else if !extend {
                    break;
                } else {
                    max_rounds += 25;
                    if self.params.verbose {
                        eprintln!("MIP: cut loop extended to {max_rounds} rounds");
                    }
                }
            }
            let round = round_ctr;
            round_ctr += 1;
            let now = super::cap_clock();
            let now = if env_str!("ENOMOTO_MIP_SEP_COUNT_CPU").is_some() { now + self.sep_par_cpu } else { now };
            // `ENOMOTO_MIP_CUT_EXTEND` (試作、既定では使わない): 根のカットの時間の上限に来ても、直近 3 ラウンドの目的値の伸びが
            // (3 ラウンドに満たなければ回った分) |目的値| の `ENOMOTO_T_MIP_CUT_EXTEND_MIN_GAIN` 倍 (既定 1e-3) 以上なら、上限を全体の `ENOMOTO_T_MIP_CUT_EXTEND_STEP`
            // (0.05) ずつ、合わせて `ENOMOTO_T_MIP_CUT_EXTEND_MAX` (0.3) まで延ばす。延ばした後は面の頂点での分離 (LP を 2 回解く)
            // を止める。また最初から、1 回の LP の時間の上限で止まった LP は外さずに解き続ける (`ENOMOTO_MIP_CUT_LP_CONTINUE` と同じ)。
            // physiciansched6-2 (同値の代入の後) では根のカットが 1 ラウンド 1〜2 秒で 48587 -> 49124 (10 ラウンド) と伸び続けるが、
            // 既定の上限 (全体の 10%) で 1〜2 ラウンドで止まっていた
            if extend_on && now - cap_base > time_cap && !self.time_up() && !phase2 && !stopped(&job) && time_cap + 1e-9 < extend_max {
                let k = ext_hist.len();
                let obj = self.lp.objective();
                let w = (k.saturating_sub(1)).min(3);
                if w >= 1 && ext_hist[k - 1] - ext_hist[k - 1 - w] >= tunable!("ENOMOTO_T_MIP_CUT_EXTEND_MIN_GAIN", 1e-3, f64) * obj.abs().max(1.0) {
                    time_cap = (time_cap + tunable!("ENOMOTO_T_MIP_CUT_EXTEND_STEP", 0.05, f64) * self.params.time_limit).min(extend_max);
                    CUT_EXTENDED.with(|c| c.set(true));
                    if self.params.verbose {
                        eprintln!("MIP: cut round {round}: still improving ({:.6e} in {w} rounds); root cut time cap extended to {time_cap:.1}s", ext_hist[k - 1] - ext_hist[k - 1 - w]);
                    }
                }
            }
            if self.time_up() || now - cap_base > time_cap || (phase2 && now - t_loop0 > phase2_deadline && !heavy) || stopped(&job) {
                break;
            }
            self.bg_cut_rounds = round_ctr;
            // `ENOMOTO_T_MIP_CLIQUE_FB_EARLY` = k (> 0): fallback の判定を、k ラウンド回っても目的値がまったく動かないときは
            // k ラウンド目で行い、使うならこのループの続きで clique カットを加える (drayage-25-23 では clique なしの 6 ラウンドに
            // 6.5 秒かかる)。ここで使わないと決まらなければループの後でもう一度判定する
            let fb_early = tunable!("ENOMOTO_T_MIP_CLIQUE_FB_EARLY", 0usize, usize);
            if clique_fb && fb_early > 0 && round == fb_early && job.is_none() && self.lp.objective() - first_obj <= 1e-9 * self.lp.objective().abs().max(1.0) {
                if self.clique_fallback_trial(root_iters, first_obj) {
                    if self.params.restarts == 0 {
                        CLIQUE_AUTO_FIRST.with(|c| c.set(Some(true)));
                    }
                    self.clique_auto = Some(true);
                    clique_fb = false;
                    stall = 0;
                }
            }
            let x = self.lp.col_values();
            if self.fractional(&x).is_empty() {
                break;
            }
            let t_sep = std::time::Instant::now();
            // `ENOMOTO_T_MIP_FACE_MAX_CUTS` = k (> 0): 面の頂点からのカットは別枠で最大 k 本選ぶ (今の頂点のカットとは
            // 競わせない)。0 なら同じ枠で選ぶ。40 問: 0 (既定) 19 問 27.70、10 は 17 問 28.14、25 は 17 問 27.83 (rout・neos5 が
            // 時間切れ、mik-250 47.9 -> 33-40 秒)。既定では使わない
            let face_quota = tunable!("ENOMOTO_T_MIP_FACE_MAX_CUTS", 0usize, usize);
            let (mut cands, mut fcands) = self.separate_face_split(&x, false);
            // `ENOMOTO_MIP_CUTSEL_FACE_ANCHOR` (実験的): 面の頂点からのカットの効き目はその頂点で測ったもので、今の LP 解は
            // 切らないことがある。それが最良のスコアになると、質の閾値 (最良の 50%) で今の頂点を切るカットが全部落ち、
            // 選んだ面のカットを足しても LP が動かず (効いていないので外され、次のラウンドでまた選ばれる) 停滞判定で止まる
            // (roll3000: 今の解での活動量 0.068 の x818 + x831 <= 1 が効き目 0.38 で毎ラウンド 1 本だけ選ばれる)。
            // 1: 質の閾値の基準 (最良のスコア) を今の頂点の候補だけで決める。2: 加えて面の候補の効き目を今の解で測り直し、
            // 今の解を切らないものは捨てる
            let face_anchor = if self.params.submip || face_quota > 0 { 0 } else { tunable!("ENOMOTO_MIP_CUTSEL_FACE_ANCHOR", 0usize, usize) };
            if face_anchor >= 2 {
                fcands.retain_mut(|c| {
                    let act: f64 = c.coefs.iter().map(|&(j, v)| v * x[j]).sum();
                    let norm = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
                    c.efficacy = (act - c.rhs) / norm.max(1e-300);
                    act - c.rhs > 1e-6 * (1.0 + c.rhs.abs()) && c.efficacy >= 1e-5
                });
            }
            let anchor = if face_anchor >= 1 {
                let cnorm = p.cost.iter().map(|c| c * c).sum::<f64>().sqrt();
                cands.iter().filter(|c| !c.forced).map(|c| cut_score(c, p, cnorm)).fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v))))
            } else {
                None
            };
            let mut face_cands: Vec<Candidate> = Vec::new();
            if face_quota > 0 {
                face_cands = fcands;
            } else {
                cands.extend(fcands);
            }
            // 再スタート前のカットをプールで引き継いだとき (`ENOMOTO_MIP_RESTART_CUTS_TO_POOL`): プールで違反しているものも候補にする
            if ((self.params.restarts > 0 && env_str!("ENOMOTO_MIP_RESTART_CUTS_TO_POOL").is_some()) || env_str!("ENOMOTO_MIP_CUTSEL_HIGHS_POOL").is_some()) && round < tunable!("ENOMOTO_T_MIP_ROOT_POOL_ROUNDS", 5usize, usize) && env_str!("ENOMOTO_MIP_NO_ROOT_POOL").is_none() {
                for (eff, k) in self.pool_violated(&x) {
                    let (c, r, _) = &self.cut_pool[k];
                    cands.push(Candidate { coefs: c.clone(), rhs: *r, efficacy: eff, forced: false });
                }
            }
            let mut inj_in = 0usize;
            if inject_cand {
                if inject_only {
                    cands.clear();
                    face_cands.clear();
                }
                let inj_rounds = INJ_ROUND.with(|v| v.borrow().clone());
                for (ci, (c, r)) in inj_cuts.iter().enumerate() {
                    if inj_rounds.get(ci).is_some_and(|&k| k > round as i64 + 1) {
                        continue;
                    }
                    let act: f64 = c.iter().map(|&(j, a)| a * x[j]).sum();
                    let norm = c.iter().map(|&(_, a)| a * a).sum::<f64>().sqrt();
                    if act - r > 1e-6 * (1.0 + r.abs()) && norm > 0.0 {
                        cands.push(Candidate { coefs: c.clone(), rhs: *r, efficacy: (act - r) / norm, forced: false });
                        inj_in += 1;
                    }
                }
            }
            // 診断 (`ENOMOTO_MIP_DEBUG_CAND_DUMP=ファイル`): 各ラウンドの候補 (選択の前) を `R ラウンド` / `C -inf 右辺 長さ 列 係数 ... ` で書き出す
            if let Some(f) = env_str!("ENOMOTO_MIP_DEBUG_CAND_DUMP").filter(|_| !self.params.submip && job.is_none()) {
                use std::io::Write;
                if let Ok(mut o) = std::fs::OpenOptions::new().create(true).append(true).open(&f) {
                    let _ = writeln!(o, "R {round} {} {:.10}", cands.len(), self.lp.objective() + p.offset);
                    for c in &cands {
                        let t: Vec<String> = c.coefs.iter().map(|&(j, a)| format!("{j} {a:e}")).collect();
                        let _ = writeln!(o, "C -inf {:e} {} {} E {:.6} F {}", c.rhs, c.coefs.len(), t.join(" "), c.efficacy, c.forced as u8);
                    }
                }
            }
            let ncands = cands.len() + face_cands.len();
            if ncands == 0 {
                break;
            }
            let dbg_sep = env_str!("ENOMOTO_MIP_DEBUG_SEP").is_some() && !self.params.submip;
            let t_sel0 = std::time::Instant::now();
            if !self.params.submip {
                self.age_pool();
            }
            // 候補はすべてカットプールに入れる (選択で落ちたものも、ノードで違反すれば使う)
            if !self.params.submip && env_str!("ENOMOTO_MIP_POOL_SELECTED_ONLY").is_none() {
                for c in cands.iter().chain(face_cands.iter()) {
                    self.add_to_pool(&c.coefs, c.rhs);
                }
            }
            let pool_secs = t_sel0.elapsed().as_secs_f64();
            let cand_nnz: usize = cands.iter().map(|c| c.coefs.len()).sum();
            // 効き目の大きい順に、平行なものを除いて選ぶ
            let max_cuts = tunable!("ENOMOTO_T_MIP_ROOT_MAX_CUTS", 50usize, usize).min(p.m.max(50));
            let max_cuts = if adapt_max_cuts > 0 { adapt_max_cuts } else { max_cuts };
            let (forced, cands): (Vec<Candidate>, Vec<Candidate>) = cands.into_iter().partition(|c| c.forced);
            let chosen = if phase2 {
                let maxpar = tunable!("ENOMOTO_T_MIP_CUT_PHASE2_MAXPAR", 0.3, f64);
                self.select_cuts_highs(cands, max_cuts, &x, maxpar)
            } else if env_str!("ENOMOTO_MIP_CUTSEL_HIGHS").is_some() {
                let maxpar = tunable!("ENOMOTO_T_CUTSEL_HIGHS_MAXPAR", 0.1, f64);
                self.select_cuts_highs(cands, max_cuts, &x, maxpar)
            } else if fc_relax {
                select_cuts_hybrid_with(cands, max_cuts, p, tunable!("ENOMOTO_T_CUTSEL_FC_MIN_REL", 0.0, f64), tunable!("ENOMOTO_T_CUTSEL_FC_MAXPAR", 0.9, f64))
            } else if fb_on {
                select_cuts_hybrid_anchor(cands, max_cuts, p, fb_rel, tunable!("ENOMOTO_T_CUTSEL_MAXPAR", 0.3, f64), anchor)
            } else if face_anchor >= 1 && env_str!("ENOMOTO_MIP_CUTSEL_OLD").is_none() {
                select_cuts_hybrid_anchor(cands, max_cuts, p, tunable!("ENOMOTO_T_CUTSEL_MIN_REL", 0.5, f64), tunable!("ENOMOTO_T_CUTSEL_MAXPAR", 0.3, f64), anchor)
            } else {
                select_cuts(cands, max_cuts, p)
            };
            let chosen = if face_cands.is_empty() {
                chosen
            } else {
                // 面の頂点のカットは別枠で選び、今の頂点で選んだカットとほぼ平行なものは除く
                let mut chosen = chosen;
                let extra = select_cuts(std::mem::take(&mut face_cands), face_quota, p);
                for c in extra {
                    if !chosen.iter().any(|d| cut_parallelism(&c.coefs, &d.coefs) > 0.99) {
                        chosen.push(c);
                    }
                }
                chosen
            };
            let mut chosen = chosen;
            // 診断 (`ENOMOTO_MIP_DEBUG_TOPCUTS` = k): 選んだカットの最初の k 本を、列の LP 値・境界・整数かと並べて表示する
            let topk = tunable!("ENOMOTO_MIP_DEBUG_TOPCUTS", 0usize, usize);
            if topk > 0 && !self.params.submip {
                for c in chosen.iter().take(topk) {
                    let terms: Vec<String> = c.coefs.iter().take(8).map(|&(j, a)| format!("{a:+.3}*x{j}[{:.3} {}..{} {}]", x[j], self.dom.global_lo[j], self.dom.global_up[j], if p.is_int[j] { "I" } else { "C" })).collect();
                    eprintln!("TOPCUT round {round} eff {:.4} nnz {} <= {:.3}: {}", c.efficacy, c.coefs.len(), c.rhs, terms.join(" "));
                }
            }
            // 選択の上限まで採ったか (選択を通さないカットは数えない)
            let quota_full = chosen.len() >= max_cuts && ncands >= 2 * max_cuts;
            chosen.extend(forced);
            let sep_secs = t_sep.elapsed().as_secs_f64();
            sep_tot += sep_secs;
            nrounds += 1;
            let sel_secs = t_sel0.elapsed().as_secs_f64() - pool_secs;
            if chosen.is_empty() {
                break;
            }
            // 上限まで採ったラウンドは停滞として数えない (候補が上限を超えて残っている間は LP を締め続ける)
            let saturated = adapt_max && chosen.len() >= max_cuts;
            if saturated {
                adapt_max_cuts = (2 * max_cuts).min(tunable!("ENOMOTO_T_MIP_CUT_ADAPT_MAX_CAP", 500usize, usize)).max(max_cuts);
            }
            if inject_cand {
                let k = chosen.iter().filter(|c| inj_keys.contains(&(c.coefs.len(), c.rhs.to_bits(), c.coefs.first().map_or(0, |e| e.0)))).count();
                eprintln!("INJECT: round {round}: {inj_in} HiGHS cuts violated (of {ncands} candidates), chosen {k} of {}", chosen.len());
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
            let dse_check = env_str!("ENOMOTO_MIP_DEBUG_DSE_CHECK").is_some() && !self.params.submip;
            if dse_check {
                self.lp.debug_dse_check(&format!("round {round} before add_rows"));
            }
            self.add_cut_rows(&rows);
            if dse_check {
                self.lp.debug_dse_check(&format!("round {round} after add_rows ({} rows)", rows.len()));
            }
            let add_secs = t_add0.elapsed().as_secs_f64();
            let t_lp0 = std::time::Instant::now();
            let it0 = self.lp.total_iterations();
            let lim = 10 * root_iters.max(100) + 10_000;
            // 1 回の LP の時間の上限 (全体の 2%、最低 1 秒): 数値的に悪条件の LP が残り時間を使い切るのを防ぐ
            let cap = if self.params.time_limit.is_finite() { (self.cut_lp_time_frac() * self.params.time_limit).max(1.0) } else { f64::INFINITY };
            let lp_deadline = if cap.is_finite() {
                let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(cap);
                Some(self.deadline.map_or(d, |g| g.min(d)))
            } else {
                self.deadline
            };
            // 診断 (`ENOMOTO_MIP_DEBUG_DUMP_CUT_LP=ディレクトリ`): カットを足した LP と出発の基底を、求解の前の状態で
            // `<ディレクトリ>/cutlp_r<ラウンド>_<結果>.mps/.bas` に書き出す (analysis/lp_hang_20261021.md)
            let dump_dir = env_str!("ENOMOTO_MIP_DEBUG_DUMP_CUT_LP").filter(|_| !self.params.submip && job.is_none());
            let dump_basis = dump_dir.as_ref().map(|_| (self.lp.basis_cols(), self.lp.dse_weights()));
            // `ENOMOTO_MIP_CUT_DSE_EXACT` (試作、既定では使わない): カットを足した LP を、引き継いだ DSE 重みではなく出発の基底の
            // 厳密な重み (行ごとに BTRAN 1 回) から解く。引き継いだ重みは求解をまたいで更新を重ねるうちに厳密な値から大きく
            // 外れ (physiciansched6-2 では 1e8〜1e9 の重みが数百行)、カットを足した LP の反復が数倍〜十数倍に増えることがある
            // (analysis/lp_hang_20261021.md)
            let cut_dse_exact = env_str!("ENOMOTO_MIP_CUT_DSE_EXACT").is_some() && !self.params.submip;
            if cut_dse_exact {
                crate::simplex::slope_intercept_dual::DSE_EXACT_ONCE.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            let mut st = self.lp.solve(&SolveLimits { iteration_limit: lim, cutoff: f64::INFINITY, deadline: lp_deadline });
            crate::simplex::slope_intercept_dual::DSE_EXACT_ONCE.store(false, std::sync::atomic::Ordering::Relaxed);
            if let Some(dir) = &dump_dir {
                let prefix = format!("{dir}/cutlp_r{round}_{st:?}");
                let (b, w) = dump_basis.as_ref().unwrap();
                let r = self.lp.dump_lp(&prefix, b.as_deref(), w.as_deref());
                eprintln!("MIP: cut round {round}: LP {st:?} after {} iters ({:.3}s), written to {prefix}.mps/.bas ({r:?})", self.lp.total_iterations() - it0, t_lp0.elapsed().as_secs_f64());
                // 診断 (`ENOMOTO_MIP_DEBUG_CUT_LP_RESOLVE`): 上限で止まった LP を、同じスレッドで新しい LP として解き直す
                if st == LpStatus::TimeLimit && env_str!("ENOMOTO_MIP_DEBUG_CUT_LP_RESOLVE").is_some() {
                    self.lp.resolve_fresh_debug(b.as_deref(), w.as_deref());
                }
            }
            // `ENOMOTO_MIP_CUT_LP_CONTINUE` (試作、既定では使わない): 1 回の LP の時間の上限で止まったとき、カットを外さずに
            // 同じ上限でもう `ENOMOTO_T_MIP_CUT_LP_CONTINUE_MAX` 回 (既定 2) まで解き続ける (根のカットの時間の上限を過ぎていても)。
            // physiciansched6-2 (同値の代入の後) では 1 ラウンドに数百本のカットを入れた LP が上限 (3 秒) を少し超え、外したカットの
            // 分の反復が無駄になり、2 回で根のカットのループが止まっていた
            // `ENOMOTO_MIP_EQUIV2_CUTS` では `ENOMOTO_T_MIP_EQUIV2_CUT_LP_CONTINUE` (既定 1) のときだけ解き続ける
            let eq2_cont = eq2c && tunable!("ENOMOTO_T_MIP_EQUIV2_CUT_LP_CONTINUE", 1usize, usize) > 0;
            if st == LpStatus::TimeLimit && (env_str!("ENOMOTO_MIP_CUT_LP_CONTINUE").is_some() || extend_on || eq2_cont) && !self.params.submip {
                for _ in 0..tunable!("ENOMOTO_T_MIP_CUT_LP_CONTINUE_MAX", 2usize, usize) {
                    if self.time_up() || stopped(&job) {
                        break;
                    }
                    let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(cap);
                    let d = Some(self.deadline.map_or(d, |g| g.min(d)));
                    st = self.lp.solve(&SolveLimits { iteration_limit: lim, cutoff: f64::INFINITY, deadline: d });
                    if self.params.verbose {
                        eprintln!("MIP: cut round {round}: LP continued after the time limit: {st:?}");
                    }
                    if st != LpStatus::TimeLimit {
                        break;
                    }
                }
            }
            // `ENOMOTO_T_MIP_CUT_LP_RETRY` = k (> 0、試作): 1 回の LP の時間の上限で止まったら、カットを外す前に同じ基底から
            // もう k 倍の時間だけ解き続ける。mzzv42z (10460 行) に implied bound カット 300 本余りを一度に入れると LP が
            // 1700 反復ほどかかり、上限 1.2 秒 (60 秒の 2%) に当たる回と当たらない回があって、当たると全部捨てて根の下界が
            // -21534 のまま終わる (当たらなければ -21203)
            // `ENOMOTO_MIP_GR_AUTO` (試作): 一般整数の列がある問題だけ 4 倍 (G のカットを入れた LP を捨てないため)
            let mut retry = tunable!("ENOMOTO_T_MIP_CUT_LP_RETRY", 0.0, f64);
            if retry == 0.0 && st == LpStatus::TimeLimit && super::gr_auto() && super::implbd_genint_for(&p.is_int, &p.col_lo, &p.col_up) {
                retry = 4.0;
            }
            if st == LpStatus::TimeLimit && retry > 0.0 && cap.is_finite() && !self.time_up() {
                let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(retry * cap);
                let d = Some(self.deadline.map_or(d, |g| g.min(d)));
                st = self.lp.solve(&SolveLimits { iteration_limit: lim, cutoff: f64::INFINITY, deadline: d });
                if self.params.verbose {
                    eprintln!("MIP: cut round {round}: LP hit the time cap; continued for up to {:.2}s more: {st:?}", retry * cap);
                }
            }
            self.sb_iters += 0;
            lp_tot += t_lp0.elapsed().as_secs_f64();
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
                // `ENOMOTO_MIP_EQUIV2_CUTS` では `ENOMOTO_T_MIP_EQUIV2_CUT_LP_FAILS` 回目 (既定 2) でやめる
                let max_fail = if eq2c { tunable!("ENOMOTO_T_MIP_EQUIV2_CUT_LP_FAILS", 2usize, usize) } else { 2 };
                if st2 != LpStatus::Optimal || lp_failures >= max_fail || env_str!("ENOMOTO_MIP_CUT_LP_FAIL_STOP").is_some() {
                    return st2 == LpStatus::Optimal;
                }
                continue;
            }
            let obj = self.lp.objective();
            // 最適に解けたカットの LP の値は問題全体の下界 (根を閉じる判定 [`Solver::root_gap_closed`] に使う)
            if job.is_none() && self.snap.is_none() {
                self.root_cut_lb = self.root_cut_lb.max(obj + p.offset);
            }
            let widen_now = widen_on && !widened && quota_full && (obj - prev_obj) / obj.abs().max(prev_obj.abs()).max(1.0) <= 1e-4;
            if widen_now {
                widened = true;
                adapt_max_cuts = p.m.min(tunable!("ENOMOTO_T_MIP_CUT_WIDE_MAX", 1000usize, usize)).max(max_cuts);
                max_rounds = max_rounds.max(round_ctr + tunable!("ENOMOTO_T_MIP_CUT_WIDE_ROUNDS", 100usize, usize));
                if self.params.verbose {
                    eprintln!("MIP: cut round {round}: {max_cuts} cuts did not move the objective; up to {adapt_max_cuts} cuts per round from now");
                }
            }
            if let Some(j) = &job {
                *j.latest_x.lock().unwrap() = Some(self.lp.col_values());
            }
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
            obj_hist.push(obj);
            if prog_win > 0 && obj_hist.len() > prog_win {
                let total = obj - first_obj;
                let recent = obj - obj_hist[obj_hist.len() - 1 - prog_win];
                if total > 1e-9 * obj.abs().max(1.0) && recent < prog_eps * total {
                    if self.params.verbose {
                        eprintln!("MIP: cut loop: progress of the last {prog_win} rounds {recent:.3e} < {prog_eps} x total {total:.3e}; stop");
                    }
                    break;
                }
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
                    if reldiff <= 1e-4 && nfrac as f64 >= (0.9 - 0.1 * stall as f64) * prev_nfrac as f64 && !saturated && !widen_now {
                        stall += if nfrac >= prev_nfrac && env_str!("ENOMOTO_MIP_CUT_STALL_ONE").is_none() && !widened { 2 } else { 1 };
                        if stall >= if fc_sub { tunable!("ENOMOTO_T_MIP_FC_SUBMIP_CUT_STALL", 10usize, usize) } else if self.params.submip { tunable!("ENOMOTO_T_MIP_SUBMIP_CUT_STALL", 3usize, usize) } else if heavy { tunable!("ENOMOTO_T_MIP_BG_CUTS_STALL", 20usize, usize) } else { tunable!("ENOMOTO_T_MIP_CUT_STALL", 10usize, usize) } {
                            if !phase2_on {
                                break;
                            }
                            // 第 2 段階へ (ラウンド数の上限を延ばす)
                            phase2 = true;
                            stall = 0;
                            max_rounds = round_ctr + tunable!("ENOMOTO_T_MIP_CUT_PHASE2_ROUNDS", 60usize, usize);
                            let t1 = super::cap_clock() - t_loop0;
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
            if fb_on {
                let g = (obj - prev_obj) / obj.abs().max(1.0);
                fb_rel = if g < tunable!("ENOMOTO_T_CUTSEL_FB_TAU", 1e-3, f64) { (0.5 * fb_rel).max(tunable!("ENOMOTO_T_CUTSEL_FB_FLOOR", 0.05, f64)) } else { fb_base };
                if self.params.verbose {
                    eprintln!("MIP: cut round {round}: selection threshold {fb_rel:.3} for the next round (gain {g:.2e})");
                }
            }
            prev_obj = obj;
            obj_hist.push(obj);
            ext_hist.push(obj);
            // 効いていないカットを外す (論理変数が基底にあり、行が緩んでいるもの)。`ENOMOTO_T_MIP_CUT_REMOVE_EVERY` ラウンドごと
            // `ENOMOTO_T_MIP_ROOT_CUT_AGE` > 0: HiGHS と同じく、効いていないラウンドが続いたカットだけを外す
            // (HiGHS の mip_lp_age_limit = 10)
            let root_age = tunable!("ENOMOTO_T_MIP_ROOT_CUT_AGE", 0u32, u32);
            let t_rm0 = std::time::Instant::now();
            let rows_before = self.lp.num_rows();
            if root_age > 0 {
                self.age_cuts();
                let removed = self.remove_cuts_older_than(root_age);
                if env_str!("ENOMOTO_MIP_DEBUG_DSE_CHECK").is_some() && !self.params.submip && removed > 0 {
                    self.lp.debug_dse_check(&format!("round {round} after removing {removed} old cuts"));
                }
            } else if (round + 1) % tunable!("ENOMOTO_T_MIP_CUT_REMOVE_EVERY", 1usize, usize).max(1) == 0 {
                self.remove_inactive_cuts();
            }
            if dbg_sep {
                eprintln!("SEP round {round}: removed {} inactive cuts in {:.3}s", rows_before - self.lp.num_rows(), t_rm0.elapsed().as_secs_f64());
            }
        }
        self.remove_inactive_cuts();
        if env_str!("ENOMOTO_MIP_DEBUG_FACETIME").is_some() && !self.params.submip {
            eprintln!(
                "FACETIME {}restarts {} rounds {nrounds}: sep {sep_tot:.2}s (face {:.2}s, {} vertices), LP {lp_tot:.2}s, loop {:.2}s, obj {:.10e}",
                if job.is_some() { "bg " } else { "root " },
                self.params.restarts,
                self.face_secs - face0.0,
                self.face_used - face0.1,
                t_loop_wall.elapsed().as_secs_f64(),
                self.lp.objective() + p.offset
            );
        }
        let st = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
        if clique_fb && st == LpStatus::Optimal && job.is_none() && !self.time_up() {
            let on = self.clique_fallback_trial(root_iters, first_obj);
            if self.params.restarts == 0 {
                CLIQUE_AUTO_FIRST.with(|c| c.set(Some(on)));
            }
            self.clique_auto = Some(on);
            if on {
                // clique カットを加えてループを続ける (2 回目のループは判定しない)
                CLIQUE_FB_SECOND.with(|c| c.set(true));
                let ok = self.root_cut_loop(root_iters);
                CLIQUE_FB_SECOND.with(|c| c.set(false));
                return ok;
            }
        }
        if !self.params.submip && st == LpStatus::Optimal && job.is_none() {
            super::record_root_bound(self.lp.objective() + p.offset);
        }
        user_log!(
            "Root cuts: objective {:.10e} -> {:.10e}, {} cuts in the LP ({:.2}s)",
            p.sense_sign * (first_obj + p.offset),
            p.sense_sign * (self.lp.objective() + p.offset),
            self.lp.num_rows() - p.m,
            self.start.elapsed().as_secs_f64()
        );
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
        let nnz_cap = (tunable!("ENOMOTO_T_MIP_CUT_POOL_NNZ_MULT", 20usize, usize) * self.p.nnz).max(200_000);
        // 最大係数 1 に揃えて 1e-9 の格子で丸めたもののハッシュ
        use std::hash::{Hash, Hasher};
        let cmax = coefs.iter().fold(0.0f64, |m, &(_, v)| m.max(v.abs())).max(1e-300);
        let mut hs = std::collections::hash_map::DefaultHasher::new();
        for &(j, v) in coefs {
            j.hash(&mut hs);
            ((v / cmax) * 1e9).round().to_bits().hash(&mut hs);
        }
        ((rhs / cmax) * 1e9).round().to_bits().hash(&mut hs);
        let key = hs.finish();
        if let Some(&k) = self.cut_pool_keys.get(&key) {
            // 既にある: 使われたので年齢を戻す
            self.pool_age[k] = 0;
            return;
        }
        if self.cut_pool_nnz + coefs.len() > nnz_cap {
            if env_str!("ENOMOTO_MIP_POOL_EVICT").is_none() {
                return;
            }
            self.evict_pool(nnz_cap);
            if self.cut_pool_nnz + coefs.len() > nnz_cap {
                return;
            }
        }
        self.cut_pool_keys.insert(key, self.cut_pool.len());
        self.pool_hash.push(key);
        self.pool_age.push(0);
        let norm = coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
        self.cut_pool_nnz += coefs.len();
        self.cut_pool.push((coefs.to_vec(), rhs, norm));
        for &(j, v) in coefs {
            self.pool_idx.push(j as u32);
            self.pool_val.push(v);
        }
        self.pool_start.push(self.pool_idx.len() as u32);
        // 木を分け合う: 新しいカットをほかのスレッドにも渡す (取り込み中のものは渡し返さない)
        if !self.importing && self.snap.is_some() && super::solver::tree_share_cuts() {
            if let Some(pool) = &self.pool {
                super::solver::learned_push(pool, self.pool_tid, super::solver::LearnedItem::Cut(coefs.to_vec(), rhs));
            }
        }
    }

    /// プールの年齢を 1 進める。
    pub(super) fn age_pool(&mut self) {
        for a in self.pool_age.iter_mut() {
            *a = a.saturating_add(1);
        }
    }

    /// プールが上限 `nnz_cap` に達したとき、年齢の大きい (長く使われていない) カットから非零数が上限の
    /// `ENOMOTO_T_MIP_POOL_EVICT_TO` 倍 (既定 0.8) になるまで捨てる (HiGHS の `HighsCutPool` の agelim に相当)。
    /// 年齢 0 (今のラウンドで加えた・使った) のカットは残す。
    fn evict_pool(&mut self, nnz_cap: usize) {
        let target = (tunable!("ENOMOTO_T_MIP_POOL_EVICT_TO", 0.8, f64) * nnz_cap as f64) as usize;
        let mut ord: Vec<usize> = (0..self.cut_pool.len()).filter(|&k| self.pool_age[k] > 0).collect();
        // 年齢の大きい順、同じなら古い (番号の小さい) 順
        ord.sort_by(|&a, &b| self.pool_age[b].cmp(&self.pool_age[a]).then(a.cmp(&b)));
        let mut drop = vec![false; self.cut_pool.len()];
        let mut nnz = self.cut_pool_nnz;
        for k in ord {
            if nnz <= target {
                break;
            }
            drop[k] = true;
            nnz -= self.cut_pool[k].0.len();
        }
        if !drop.iter().any(|&d| d) {
            return;
        }
        let old = std::mem::take(&mut self.cut_pool);
        let (hash, age) = (std::mem::take(&mut self.pool_hash), std::mem::take(&mut self.pool_age));
        self.cut_pool_keys.clear();
        self.pool_idx.clear();
        self.pool_val.clear();
        self.pool_start.clear();
        self.pool_start.push(0);
        self.cut_pool_nnz = 0;
        for (k, c) in old.into_iter().enumerate() {
            if drop[k] {
                continue;
            }
            self.cut_pool_keys.insert(hash[k], self.cut_pool.len());
            self.pool_hash.push(hash[k]);
            self.pool_age.push(age[k]);
            self.cut_pool_nnz += c.0.len();
            for &(j, v) in &c.0 {
                self.pool_idx.push(j as u32);
                self.pool_val.push(v);
            }
            self.pool_start.push(self.pool_idx.len() as u32);
            self.cut_pool.push(c);
        }
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
        // 効き目の大きい順、同じなら番号の小さい順 (全順序なので、上位 `max_cuts` 本を選んでから並べても全体を
        // 安定に並べたのと同じ結果になる)
        let ord = |a: &(f64, usize), b: &(f64, usize)| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1));
        if max_cuts > 0 && viol.len() > max_cuts {
            viol.select_nth_unstable_by(max_cuts - 1, ord);
        }
        viol.truncate(max_cuts);
        viol.sort_unstable_by(ord);
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
        self.age_pool();
        for &(_, k) in &viol {
            self.pool_age[k] = 0;
        }
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
        // `ENOMOTO_MIP_NODE_FACE_SEP`: ノードでも最適面の頂点で分離する ([`Self::separate_face`])
        // 40 問: 使わない 18 問 27.20、使う 18 問 27.36 (neos5 時間切れ -> 59.7 秒、neos-860300 54.2 秒 -> 時間切れ。木での分離の
        // 時間はもともと小さく、差はほぼない)。既定では使わない
        let cands = if env_str!("ENOMOTO_MIP_NODE_FACE_SEP").is_some() { self.separate_face(x, true) } else { self.separate(x, true) };
        if cands.is_empty() {
            return false;
        }
        let room = max_rows - self.lp.num_rows();
        let chosen = select_cuts(cands, room.min(tunable!("ENOMOTO_T_MIP_NODE_MAX_CUTS", 20usize, usize)), p);
        if chosen.is_empty() {
            return false;
        }
        let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = chosen.into_iter().map(|c| (c.coefs, f64::NEG_INFINITY, c.rhs)).collect();
        // 加えるカットの非零の数の平均を効き目で続けるノードの分離の密度の判定 ([`Solver::nca_dense`]) に入れる
        let d = rows.iter().map(|r| r.0.len()).sum::<usize>() as f64 / rows.len() as f64;
        self.nca_cut_nnz = Some(self.nca_cut_nnz.map_or(d, |e| 0.9 * e + 0.1 * d));
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
            cands.push(Candidate { coefs: c.clone(), rhs: *r, efficacy: eff, forced: false });
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
        // 診断用 (デバッグ解): どのスレッドでも、LP に加えるカットの行がデバッグ解を切っていれば表示する (打ち切り値より良いときだけ)
        if self.dbg_on {
            super::solver::DEBUG_SOL.with(|d| {
                if let Some(x) = d.borrow().as_ref() {
                    if self.p.objective(x) < self.prune_limit() - 1e-6 {
                        for (k, (c, lo, up)) in rows.iter().enumerate() {
                            let act: f64 = c.iter().map(|&(j, a)| a * x[j]).sum();
                            if act < lo - 1e-6 * (1.0 + lo.abs()) || act > up + 1e-6 * (1.0 + up.abs()) {
                                let terms: Vec<String> = c.iter().take(8).map(|&(j, a)| format!("{a:+.4}*x{j}[{:.4}]", x[j])).collect();
                                eprintln!("MIP_DEBUG_SOL: node {} thread {}: cut row {k} of {} added to the LP (as row {}) is violated by the debug solution: {lo} <= {act:.10e} <= {up}, len {}: {}{}", self.nodes, self.pool_tid, rows.len(), self.lp.num_rows() + k, c.len(), terms.join(" "), if c.len() > 8 { " ..." } else { "" });
                            }
                        }
                    }
                }
            });
        }
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
    /// 外さない。取り出したときに保存した基底が必ず合う)。既定では使わない (`ENOMOTO_MIP_PROTECT_QUEUE_CUTS` で使う)。
    /// 以前は既定で使っていた (40 問: 19 問 27.04 -> 19 問 26.89、保護して年齢の上限を一律 30 にすると 17 問 27.55) が、待ち行列の
    /// ノードが数千あると、それぞれが入れた時点で効いていたカットの和でほぼ全てのカットが外せなくなり LP が膨らむ (pk1 は元 45 行が
    /// 約 545 行になり、ノードの LP が 2 倍遅い)。その計測は役の復帰で `idle` が下に回り待ち行列がほぼ空になる不具合のある状態で
    /// 取ったもの (analysis/pk1_variance_20261017.md)。HiGHS も保護せず、基底が行と合わなければ今の基底から解く
    pub(super) fn basis_protected_cuts(&mut self) -> Option<std::rc::Rc<Vec<u64>>> {
        // HiGHS 流のカット管理では保護しない (HiGHS と同じく、取り出したノードの基底が LP の行と合わなければ今の基底から解く。
        // 保護すると待ち行列のノードが数千あるときほぼ全てのカットが外せず、neos-911970 では LP が行数の上限 607 に張り付いた)
        if env_str!("ENOMOTO_MIP_PROTECT_QUEUE_CUTS").is_none() || env_str!("ENOMOTO_MIP_NO_PROTECT_QUEUE_CUTS").is_some() || self.params.submip || (self.highs_cutmgmt() && env_str!("ENOMOTO_MIP_HCM_PROTECT").is_none()) {
            return None;
        }
        self.sync_cut_age();
        let m0 = self.p.m;
        let mr = self.lp.num_rows();
        if mr <= m0 {
            return None;
        }
        let b = self.lp.basis_ref();
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
        let b = self.lp.basis_ref();
        let act = self.lp.row_activities_ref();
        // 相対的な基準 (既定では使わない):
        // `ENOMOTO_T_MIP_CUT_SLACK_REL` = s (> 0): 余裕のあるカットのうち、正規化した余裕 (余裕 / 係数の 2 ノルム) が
        //   カットの中で最大のものの s 倍に満たないもの (ほぼ効いている) は年齢を増やさない
        // `ENOMOTO_T_MIP_CUT_DUAL_REL` = d (> 0): 効いている (論理変数が非基底の) カットのうち、|双対値| がカットの中で
        //   最大のものの d 倍に満たないもの (退化して効いているだけ) は年齢を増やす
        // 40 問 (19 問は同じ): 既定 25.28 に対し s=0.1 は 24.96 だったが、もう一度並べると 25.37 (s=0.05 25.31、
        // s=0.3 25.28)。d=0.01 は 25.35、プールの r=0.1 は 25.26。差はばらつきの範囲なので、どれも既定では使わない
        let srel = tunable!("ENOMOTO_T_MIP_CUT_SLACK_REL", 0.0, f64);
        let drel = tunable!("ENOMOTO_T_MIP_CUT_DUAL_REL", 0.0, f64);
        let ncut = self.cut_age.len();
        // 正規化した余裕 (`srel > 0` のときだけ作る)
        let mut nslack = Vec::new();
        let mut max_slack = 0.0f64;
        if srel > 0.0 {
            nslack = vec![0.0f64; ncut];
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
        // `ENOMOTO_MIP_CUT_AGE_HIGHS`: HiGHS の `HighsLpRelaxation::performAging` と同じく、論理変数が基底のカットは余裕が 0
        // (退化して効いている形) でも年齢を増やし、非基底でも双対値が 0 なら増やす。0 に戻すのは双対値が 0 でないものだけ。
        // 既定の数え方では退化した基底の行の年齢が 0 に戻り続け、上限を下げても LP のカットの行が減らない (mc11 で上限 50 でも 3770 行)
        if env_str!("ENOMOTO_MIP_CUT_AGE_HIGHS").is_some() {
            let y = self.lp.row_duals();
            for (k, age) in self.cut_age.iter_mut().enumerate() {
                let i = m0 + k;
                if b.row[i] == VarStatus::Basic || y[i].abs() <= 1e-9 {
                    *age = age.saturating_add(1);
                } else {
                    *age = 0;
                }
            }
        } else {
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
        if !self.merged_cuts.is_empty() {
            for k in 0..ncut {
                if self.merged_cuts.contains(&self.cut_ids[k]) {
                    self.cut_age[k] = 0;
                }
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
        // `ENOMOTO_T_MIP_TREE_SEP_CUT_AGE` = a (> 0): `ENOMOTO_MIP_TREE_SEP` のラウンドで加えたカットは年齢 a で外す
        // (根のカットは上の上限のまま)。ノードで加えたカットの行が残ってノードの LP が重くなるのを抑える
        let ts_age = tunable!("ENOMOTO_T_MIP_TREE_SEP_CUT_AGE", 0u32, u32);
        if ts_age > 0 && !self.ts_cut_ids.is_empty() {
            self.sync_cut_age();
            let m0 = self.p.m;
            let mut remove = vec![false; self.lp.num_rows()];
            let mut cnt = 0;
            for (k, &age) in self.cut_age.iter().enumerate() {
                if age > ts_age && self.ts_cut_ids.contains(&self.cut_ids[k]) && !self.queue.is_protected(self.cut_ids[k]) {
                    remove[m0 + k] = true;
                    cnt += 1;
                }
            }
            if cnt > 0 {
                self.delete_lp_rows(&remove);
                removed += cnt;
            }
            if self.ts_cut_ids.len() > 4 * self.cut_ids.len() + 1000 {
                let live: std::collections::HashSet<u64> = self.cut_ids.iter().copied().collect();
                self.ts_cut_ids.retain(|id| live.contains(id));
            }
        }
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

    /// `ENOMOTO_MIP_TREE_SEP` のラウンドで LP の行 `r0..` に加えたカットを覚える (`ENOMOTO_T_MIP_TREE_SEP_CUT_AGE` > 0 のときだけ)
    pub(super) fn mark_tree_sep_cuts(&mut self, r0: usize) {
        if tunable!("ENOMOTO_T_MIP_TREE_SEP_CUT_AGE", 0u32, u32) == 0 {
            return;
        }
        self.sync_cut_age();
        let m0 = self.p.m;
        for k in r0.max(m0) - m0..self.cut_ids.len() {
            self.ts_cut_ids.insert(self.cut_ids[k]);
        }
    }

    /// 年齢が `limit` を超えたカットを LP から外す。外した数を返す。
    pub(super) fn remove_cuts_older_than(&mut self, limit: u32) -> usize {
        self.sync_cut_age();
        let m0 = self.p.m;
        let old = |k: usize, age: u32| age > limit && !self.queue.is_protected(self.cut_ids[k]);
        // 外すカットがなければ印の配列を作らない
        if !self.cut_age.iter().enumerate().any(|(k, &age)| old(k, age)) {
            return 0;
        }
        let mut remove = vec![false; self.lp.num_rows()];
        let mut cnt = 0;
        for (k, &age) in self.cut_age.iter().enumerate() {
            if old(k, age) {
                remove[m0 + k] = true;
                cnt += 1;
            }
        }
        self.delete_lp_rows(&remove);
        cnt
    }

    /// 今の LP で効いている (双対値が 0 でない) カットを、|双対値| を重みに足して 1 本の行にまとめて加える
    /// (各カットは `a_i x <= b_i` なので、非負の重みの和 `sum w_i a_i x <= sum w_i b_i` も妥当)。今の LP の双対解は
    /// まとめた行に重み 1 を置いても実行可能なので、元のカットが年齢で外れても今の LP の目的値は保たれる
    /// (子のノードでは個々のカットより弱い)。まとめた行は年齢で外さない。加えたら真。
    pub(super) fn merge_active_cuts(&mut self) -> bool {
        let m0 = self.p.m;
        let n = self.p.n;
        let mr = self.lp.num_rows();
        if mr <= m0 {
            return false;
        }
        let y = self.lp.row_duals();
        let mut wmax = 0.0f64;
        for i in m0..mr {
            let (l, u) = self.lp.row_bounds(i);
            if l == f64::NEG_INFINITY && u.is_finite() {
                wmax = wmax.max(y[i].abs());
            }
        }
        if wmax <= 1e-12 {
            return false;
        }
        let mut dense = vec![0.0f64; n];
        let mut rhs = 0.0f64;
        let mut used = 0usize;
        for i in m0..mr {
            let (l, u) = self.lp.row_bounds(i);
            if l != f64::NEG_INFINITY || !u.is_finite() {
                continue;
            }
            let w = y[i].abs() / wmax;
            if w <= 1e-9 {
                continue;
            }
            used += 1;
            for (j, a) in self.lp.row(i) {
                dense[j] += w * a;
            }
            rhs += w * u;
        }
        if used < 2 {
            return false;
        }
        let cmax = dense.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let mut coefs: Vec<(usize, f64)> = Vec::new();
        for (j, &c) in dense.iter().enumerate() {
            if c == 0.0 {
                continue;
            }
            if c.abs() <= 1e-9 * cmax {
                // 小さい係数は大域的な境界で右辺に移す (c x_j >= min(c lo, c up))
                let m = if c > 0.0 { c * self.dom.global_lo[j] } else { c * self.dom.global_up[j] };
                if m.is_finite() {
                    rhs -= m;
                    continue;
                }
            }
            coefs.push((j, c));
        }
        let max_nnz = tunable!("ENOMOTO_T_MIP_MERGE_MAX_NNZ_FRAC", 1.0, f64);
        if coefs.is_empty() || coefs.len() as f64 > max_nnz * n as f64 {
            return false;
        }
        let nnz = coefs.len();
        self.add_cut_rows(&[(coefs, f64::NEG_INFINITY, rhs)]);
        if let Some(&id) = self.cut_ids.last() {
            self.merged_cuts.insert(id);
        }
        let st = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
        if self.params.verbose {
            eprintln!("MIP: merged {used} active cuts into one row ({nnz} nonzeros), LP {st:?} obj {:.10e}", self.lp.objective() + self.p.offset);
        }
        true
    }

    /// LP の行のうち、元の行より後ろ (カット) で論理変数が基底にあるものを外す。
    pub(super) fn remove_inactive_cuts(&mut self) {
        let m0 = self.p.m;
        let mr = self.lp.num_rows();
        if mr <= m0 {
            return;
        }
        self.sync_cut_age();
        let b = self.lp.basis_ref();
        let act = self.lp.row_activities_ref();
        let mut remove = vec![false; mr];
        let mut any = false;
        for i in m0..mr {
            if b.row[i] == VarStatus::Basic {
                let (_, up) = self.lp.row_bounds(i);
                if act[i] < up - 1e-6 * (1.0 + up.abs()) && !self.merged_cuts.contains(&self.cut_ids[i - m0]) {
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
                    // 並列の分離: 集約した行を集めるだけにする (カットは呼び出し側でまとめて作る)
                    if let Some(col) = self.path_collect.as_mut() {
                        col.push(base);
                    } else {
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

    /// 根の分離 (退化に強い形、`ENOMOTO_MIP_FACE_SEP`)。LP が双対退化している (非基底で被約費用が 0 の列がある) と
    /// 最適解の集合 (最適面) に頂点が多く、どの頂点に着くかは DSE 重みなど解き方の履歴で変わる。カットは着いた頂点から
    /// 作るので、頂点が変わるとカットも根の下界も変わり、また 1 つの頂点だけを切っても LP は同じ目的値の別の頂点へ移るだけで
    /// 下界が動かない。そこで最適面の上で決まった頂点を選んで分離する:
    /// 被約費用が 0 でない非基底列を今の値に固定した LP (の複製) を、費用 c + ε c2 で解き直す (ε c2 は小さいので
    /// 主の目的値は変わらず、最適面の中で c2 が最小の頂点に移る)。モード:
    /// - `canon`: c2 は列ごとに決まった擬似乱数。その頂点だけで分離する (着いた頂点によらず同じ点から分離する)
    /// - `both`: c2 は同じ擬似乱数。今の頂点と面の頂点の両方で分離して候補を合わせる
    /// - `int`: c2 は分数の整数列を近い整数の側へ押す向き。両方で分離する
    /// 面の頂点の真の目的値が今の値から離れたら (ε が大きすぎた) 今の頂点だけで分離する。
    /// 既定は `int` (`off` で使わない)。40 問 (DSE 重みを全ての LP で引き継ぐ既定のもとで): 使わない 18 問 28.07、
    /// `int` 19 問 27.04 (misc07 時間切れ -> 40.9 秒、neos5 時間切れ -> 59.5 秒、mik-250 42.4 -> 16.9 秒、nw04 20.5 -> 8.7 秒、
    /// rout 57.8 秒 -> 時間切れ)、`canon` 17 問 27.66。既定は `int2` (同時に測った `int` 18 問 27.94 に対し 20 問 27.70:
    /// rout 時間切れ -> 31.2 秒、neos5 時間切れ -> 53.2 秒、misc07 39.6 -> 54.1 秒)
    fn separate_face(&mut self, x: &[f64], light: bool) -> Vec<Candidate> {
        let (mut a, mut b) = self.separate_face_split(x, light);
        a.append(&mut b);
        a
    }

    /// [`Self::separate_face`] の候補を、今の頂点からのもの と 面の頂点からのもの に分けて返す。
    fn separate_face_split(&mut self, x: &[f64], light: bool) -> (Vec<Candidate>, Vec<Candidate>) {
        if crate::simplex::slope_intercept_dual::mip_profile() == 1 {
            return (self.separate(x, light), Vec::new());
        }
        if (CUT_EXTENDED.with(|c| c.get()) || self.equiv2_cuts()) && !self.params.submip {
            return (self.separate(x, light), Vec::new());
        }
        // 試作 (inv/gfrsplit-20261021): `ENOMOTO_MIP_FACE_SEP_BG=off` なら背景のカットの求解 (`CUT_JOB`) だけ面の頂点の分離を止める
        // (根の手前のカット・ヒューリスティクスの経路は既定のまま)。`ENOMOTO_MIP_GR_AUTO=2` なら一般整数の列がある問題だけ止める
        // (G が働く問題。mzzv42z は G と面の分離を止めることの両方で根の RENS が崩れなくなる)
        let in_bg = self.snap.is_some() && super::solver::CUT_JOB.with(|j| j.borrow().is_some());
        if in_bg && env_str!("ENOMOTO_MIP_FACE_SEP_BG") == Some("off") {
            return (self.separate(x, light), Vec::new());
        }
        if super::gr_auto_face_off() && super::implbd_genint_for(&self.p.is_int, &self.p.col_lo, &self.p.col_up) {
            return (self.separate(x, light), Vec::new());
        }
        // `ENOMOTO_MIP_FACE_SEP_FC=off` (2026-10-21 から既定。`=on` で止めない): 固定費用ネットワーク型 (根のカット選択を緩める問題) では面の頂点の分離を止める。
        // mc11: 止めると根の手前のカットが 2.6 -> 1.1〜1.5 秒で 11069 (止めなければ 11108) に着き、背景のカットが 23 -> 50〜56
        // ラウンド回って 11455 -> 11672〜11674 (最適 11689) になり 60 秒で解ける (背景のカットだけ止めても 11549〜11604 で解けない)
        // `ENOMOTO_T_MIP_FACE_SEP_FC_MAXCOLS` (既定 5000): 列 (前処理の後) がこの数を超える問題では止めない。
        // h80x6320d (12640 列) では F で根の頂点が変わると根の RINS が 9 回中 5 回 6644.8 しか出さず (既定は 6 回とも
        // 6388〜6418)、最初の再スタートで固まる列が 4473〜5126 -> 996 本になって 2 回目の再スタートが要り 54〜60 秒かかる
        // (analysis/qf_narrow_20261021.md)。mc11 は 3034 列
        if self.fc_network && env_str!("ENOMOTO_MIP_FACE_SEP_FC") != Some("on") && self.p.n <= tunable!("ENOMOTO_T_MIP_FACE_SEP_FC_MAXCOLS", 5000usize, usize) {
            return (self.separate(x, light), Vec::new());
        }
        let mode = match env_str!("ENOMOTO_MIP_FACE_SEP") {
            Some(m) if m == "off" => return (self.separate(x, light), Vec::new()),
            Some(m) if !self.params.submip => m,
            None if !self.params.submip => "int2",
            _ => return (self.separate(x, light), Vec::new()),
        };
        let p = self.p;
        let n = p.n;
        let d = self.lp.reduced_costs();
        let b = self.lp.basis();
        let cmax = p.cost.iter().fold(0.0f64, |m, c| m.max(c.abs())).max(1.0);
        let mut face = self.lp.clone();
        let mut ndeg = 0usize;
        for j in 0..n {
            if b.col[j] == VarStatus::Basic {
                continue;
            }
            let (l, u) = face.col_bounds(j);
            if l == u {
                continue;
            }
            if d[j].abs() > 1e-7 * (1.0 + p.cost[j].abs()) {
                face.set_col_bounds(j, x[j], x[j]);
            } else {
                ndeg += 1;
            }
        }
        // `ENOMOTO_MIP_FACE_BASIS`: 双対値が 0 で非基底の行 (スラック) による退化も数え、面の LP で点が動かなくても
        // 基底が変わっていればその基底で分離する (主退化で最適基底が複数ある場合。tableau 行が変わるので別のカットが出る)
        // 40 問 (`int2` のもとで): 使わない 20 問 27.89、使う 18 問 28.07 (misc07 57.3 -> 45.6 秒だが、neos5・neos-860300 が
        // 時間切れ、nw04 10.5 -> 16.4 秒)。既定では使わない
        let face_basis = env_str!("ENOMOTO_MIP_FACE_BASIS").is_some();
        if face_basis {
            let y = self.lp.row_duals();
            for i in 0..b.row.len() {
                if b.row[i] != VarStatus::Basic && y[i].abs() <= 1e-9 {
                    ndeg += 1;
                }
            }
        }
        if ndeg == 0 {
            return (self.separate(x, light), Vec::new());
        }
        let eps = tunable!("ENOMOTO_T_MIP_FACE_EPS", 1e-5, f64) * cmax;
        let hash = |j: usize| -> f64 {
            let mut h = (j as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
            h ^= h >> 31;
            h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            h ^= h >> 29;
            (h >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
        };
        // 副目的の向き (`int2`: 近い整数の側へ押す頂点と、遠い側へ押す頂点の 2 つ)
        let near: Vec<f64> = (0..n)
            .map(|j| {
                let f = x[j] - x[j].floor();
                if self.cut_int[j] && f > 1e-6 && f < 1.0 - 1e-6 {
                    if f < 0.5 { 1.0 } else { -1.0 }
                } else {
                    0.0
                }
            })
            .collect();
        let dirs: Vec<Vec<f64>> = match mode {
            "int" => vec![near],
            "int2" => {
                let far: Vec<f64> = near.iter().map(|v| -v).collect();
                vec![near, far]
            }
            _ => vec![(0..n).map(hash).collect()],
        };
        let z0: f64 = (0..n).map(|j| p.cost[j] * x[j]).sum();
        self.tab_memo.clear();
        self.tab_memo_mode = 1;
        let cands = if mode == "canon" { Vec::new() } else { self.separate(x, light) };
        self.tab_memo_mode = 2;
        let t_face0 = std::time::Instant::now();
        let mut fcands: Vec<Candidate> = Vec::new();
        let mut used = 0usize;
        // 面の LP 1 回の時間の上限 (カットを加えた LP と同じ、全体の 2%、最低 1 秒)。neos-3656078-kumeu・neos-933966 では
        // 根の最初のラウンドの面の LP が数値的に破綻して解き直しを繰り返し、残り時間 (50 秒以上) を使い切っていた
        let cap = if self.params.time_limit.is_finite() { (self.cut_lp_time_frac() * self.params.time_limit).max(1.0) } else { f64::INFINITY };
        for c2 in &dirs {
            let mut f2 = face.clone();
            let cost2: Vec<f64> = (0..n).map(|j| p.cost[j] + eps * c2[j]).collect();
            f2.set_costs(&cost2);
            let deadline = if cap.is_finite() {
                let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(cap);
                Some(self.deadline.map_or(d, |g| g.min(d)))
            } else {
                self.deadline
            };
            let st = f2.solve(&SolveLimits { iteration_limit: if light { (p.m + n) as u64 + 100 } else { 10 * (p.m + n) as u64 + 1000 }, cutoff: f64::INFINITY, deadline });
            if st != LpStatus::Optimal {
                continue;
            }
            let xv = f2.col_values();
            let zv: f64 = (0..n).map(|j| p.cost[j] * xv[j]).sum();
            let moved = (0..n).filter(|&j| (xv[j] - x[j]).abs() > 1e-6).count();
            if env_str!("ENOMOTO_MIP_DEBUG_FACE").is_some() {
                eprintln!("FACE: degenerate nonbasic {ndeg}, moved {moved} cols, obj {z0:.9} -> {zv:.9}, frac {} -> {}", self.fractional(x).len(), self.fractional(&xv).len());
            }
            if zv > z0 + 1e-7 * (1.0 + z0.abs()) {
                continue;
            }
            if moved == 0 {
                // 点は同じ。基底が変わっていればその基底で分離する (`ENOMOTO_MIP_FACE_BASIS`)
                let fb = f2.basis();
                let same = fb.col.iter().zip(&b.col).all(|(u, v)| (*u == VarStatus::Basic) == (*v == VarStatus::Basic))
                    && fb.row.iter().zip(&b.row).all(|(u, v)| (*u == VarStatus::Basic) == (*v == VarStatus::Basic));
                if !face_basis || same {
                    continue;
                }
            }
            // `ENOMOTO_MIP_FACE_LIGHT`: 面の頂点では軽い分離 (経路集約の始点・tableau 行を減らす) にする
            let lt = light || env_str!("ENOMOTO_MIP_FACE_LIGHT").is_some();
            std::mem::swap(&mut self.lp, &mut f2);
            let mut cv = self.separate(&xv, lt);
            std::mem::swap(&mut self.lp, &mut f2);
            fcands.append(&mut cv);
            used += 1;
        }
        self.tab_memo_mode = 0;
        self.face_secs += t_face0.elapsed().as_secs_f64();
        self.face_used += used;
        if env_str!("ENOMOTO_MIP_DEBUG_FACE").is_some() {
            eprintln!("FACE tab memo: checked {} rows, same as at the current vertex {}", self.tab_memo_stats.0, self.tab_memo_stats.1);
        }
        if mode == "canon" && used == 0 {
            return (self.separate(x, light), Vec::new());
        }
        (cands, fcands)
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
        // 一時的に加えた行 (目的関数の下界の行など、`Solver::extra_sep_rows`)
        if !self.extra_sep_rows.is_empty() {
            let extra = self.extra_sep_rows.clone();
            for (b, r) in &extra {
                let n0 = cands.len();
                for c in generate_cuts(&vars, b, *r, use_flow_cover(), true) {
                    push(Some(c), &mut cands, self);
                }
                keep_best(&mut cands, n0, self.p);
            }
        }
        // 元の行 1 本ずつ (診断用 `ENOMOTO_MIP_DEBUG_NO_ROW_CUTS`: 使わない)
        let row_cuts_m = if env_str!("ENOMOTO_MIP_DEBUG_NO_ROW_CUTS").is_some() { 0 } else { p.m };
        for i in 0..row_cuts_m {
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
        // (13 問 / 幾何平均 30.57 に悪化)。ENOMOTO_MIP_CLIQUE_CUTS=1 で使う。プロービングの含意を辺に加えても (misc07 の辺は
        // 2198 -> 4760、他の問題はほぼ増えない) 40 問で 20 問 28.11 -> 17 問 30.13 (10teams・rout・misc07 が時間切れ)
        // `ENOMOTO_MIP_CLIQUE_CUTS=root` (試作): 根の分離だけで使う (ノードの分離 `light` では使わない)。drayage-25-23 の根の
        // 下界は 1 ラウンド目で 99718.79 -> 100121.99 (HiGHS と同じ。HiGHS も clique と implied bound の分離を止めると根で動かない)
        // `ENOMOTO_MIP_CLIQUE_CUTS=auto` (試作): 根で一度だけ clique カットだけを入れた LP を解き、下界が上がる問題でだけ使う
        // ([`Self::clique_trial`])
        let clique_mode = clique_cuts_mode();
        // `ENOMOTO_MIP_CLIQUE_CUTS=fallback` (既定、[`clique_cuts_mode`]): 根のループを clique なしで回し、ほかのカットで伸びず clique カットで伸びる問題でだけ
        // 根の続きのラウンドで使う ([`Self::clique_fallback_trial`])。ノードでは使わない (`ENOMOTO_MIP_CLIQUE_FB_NODES` で使う)
        let fb_nodes = env_str!("ENOMOTO_MIP_CLIQUE_FB_NODES").is_some();
        if clique_mode.is_some_and(|m| !(light && (m == "root" || (m == "fallback" && !fb_nodes))) && ((m != "auto" && m != "fallback") || self.clique_auto == Some(true))) {
            let g = self.clique_graph_get();
            for raw in g.separate(x, tunable!("ENOMOTO_T_MIP_CLIQUE_MAX", 100usize, usize)) {
                push(Some(raw), &mut cands, self);
            }
        }
        if dbg_sep {
            eprintln!("SEP clique {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        // implied bound カット (HiGHS の `HighsImplications::separateImpliedBounds`): プロービングの含意
        // `y = v ⇒ x_k <= b` から `x_k <= u_k - (u_k - b) y` (v = 1) / `x_k <= b + (u_k - b) y` (v = 0) (下限も同様)。
        // u_k は今の大域的な上限。LP 解で違反するものだけ
        // 既定では含意の相手が連続変数のものだけを使う (`ENOMOTO_MIP_IMPLBD_CONT`、施設配置の分解した不等式 `x_ij <= d_i y_j`:
        // pg は容量の行 `sum_i x_ij <= C y_j` だけでは緩和が弱く、これで 60 秒で解けない -> 25 秒)。`ENOMOTO_MIP_IMPLBD_CUTS` なら
        // 0-1 変数どうしの含意も使う (全問で使うと 40 問で eil33-2・mik-250 が遅くなり SGM +2.7%)。`ENOMOTO_MIP_NO_IMPLBD_CONT` で無効
        let ib_all = env_str!("ENOMOTO_MIP_IMPLBD_CUTS").is_some();
        // 既定 (連続変数のみ) は根だけ (`light` はノードの分離)。ノードでも作ると木の経過が変わり rout などが揺れる
        let ib_cont = !ib_all && !light && env_str!("ENOMOTO_MIP_NO_IMPLBD_CONT").is_none();
        // `ENOMOTO_MIP_IMPLBD_GENINT` (試作): 含意の相手が一般整数 (0-1 でない整数) の列も使う (HiGHS の
        // `separateImpliedBounds` と同じく 0-1 でない列全部)。mzzv42z (全部整数、一般整数 235 列) は連続変数がないので
        // 既定では 1 本も出ないが、HiGHS の根の分離で LP に入るカットの 8 割がこれで、止めると根の下界が -20829 -> -21209 に
        // 落ちる。`=nodes` ならノードの分離 (`light`) でも使う
        let ib_gen_mode = env_str!("ENOMOTO_MIP_IMPLBD_GENINT");
        // `ENOMOTO_MIP_GR_AUTO` (試作): 一般整数の列があるときだけ根で使う ([`super::implbd_genint_for`])
        let ib_gen_auto = ib_gen_mode.is_none() && !light && super::gr_auto() && super::implbd_genint_for(&p.is_int, &p.col_lo, &p.col_up);
        let ib_gen = !ib_all && ((ib_gen_mode.is_some() && (!light || ib_gen_mode == Some("nodes"))) || ib_gen_auto);
        let ib_cont = ib_cont || ib_gen;
        if (ib_all || ib_cont) && !p.implied_bounds.is_empty() {
            let (glo, gup) = (&self.dom.global_lo, &self.dom.global_up);
            let max_ib = tunable!("ENOMOTO_T_MIP_IMPLBD_MAX", 5000usize, usize);
            let mut viol: Vec<(f64, RawCut)> = Vec::new();
            for &(y, v, k, upper, b) in &p.implied_bounds {
                let (y, k) = (y as usize, k as usize);
                if ib_cont && p.is_int[k] && !(ib_gen && !(glo[k] >= 0.0 && gup[k] <= 1.0)) {
                    continue;
                }
                if glo[y] == gup[y] || glo[k] == gup[k] || glo[y] != 0.0 || gup[y] != 1.0 {
                    continue;
                }
                // 列を上限側にそろえる: upper なら x_k、下限なら -x_k の上限 (-l_k、含意の値は -b)
                let (sk, bound, imp) = if upper { (1.0, gup[k], b) } else { (-1.0, -glo[k], -b) };
                if !bound.is_finite() || imp >= bound - 1e-9 * (1.0 + bound.abs()) {
                    continue;
                }
                let d = bound - imp;
                // sk x_k + cy y <= rhs
                let (cy, rhs) = if v { (d, bound) } else { (-d, imp) };
                let act = sk * x[k] + cy * x[y];
                let nrm = (1.0 + cy * cy).sqrt();
                if act - rhs > 1e-6 * nrm * (1.0 + rhs.abs()) {
                    let coefs = if y < k { vec![(y, cy), (k, sk)] } else { vec![(k, sk), (y, cy)] };
                    viol.push(((act - rhs) / nrm, RawCut { coefs, rhs }));
                }
            }
            viol.sort_by(|a, b| b.0.total_cmp(&a.0));
            let n0 = cands.len();
            for (_, raw) in viol.into_iter().take(max_ib) {
                push(Some(raw), &mut cands, self);
            }
            // 2 非零で 1 本ずつ別の (y, x) の組を締めるので、根では選択 (質の閾値・平行度) を通さずに全部入れる (HiGHS も
            // implied bound カットはプールに直接入れる)。pg: 選択を通すと根 -9027、通さなければ -8680 (HiGHS -8680) で 60 秒で解けない -> 7.9 秒
            if env_str!("ENOMOTO_MIP_IMPLBD_SELECT").is_none() {
                for c in &mut cands[n0..] {
                    c.forced = true;
                }
            }
            if dbg_sep {
                eprintln!("SEP implied bounds {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
            }
        }
        // 経路集約 (path aggregation、HiGHS の `HighsPathSeparator`)
        if env_str!("ENOMOTO_MIP_NO_PATH_AGG").is_none() {
            // 固定費用ネットワーク型ではノードでも経路集約の始点を使う (`ENOMOTO_T_MIP_NODE_FC_PATH_STARTS`、[`Solver::node_cuts_due`])
            let max_starts = if !light {
                tunable!("ENOMOTO_T_MIP_PATH_STARTS", 1000usize, usize)
            } else if self.fc_node_sep || self.tree_sep_fc {
                tunable!("ENOMOTO_T_MIP_NODE_FC_PATH_STARTS", 1000usize, usize)
            } else {
                tunable!("ENOMOTO_T_MIP_NODE_PATH_STARTS", 0usize, usize)
            };
            // 既定は HiGHS の経路集約の移植 (`ENOMOTO_MIP_PATH_OLD` で従来の経路集約)
            // 経路集約: 従来のもの (`ENOMOTO_MIP_PATH_OLD`)、HiGHS の移植 (`ENOMOTO_MIP_PATH_HIGHS_ONLY`)、既定は両方
            // (h80x6320d・rout は移植で根の下界が上がり、mik-250 は従来のものの方が強い)
            // `ENOMOTO_MIP_FC_NODE_PATH_HIGHS`: 固定費用ネットワーク型の毎ノードの分離では、ノードでも HiGHS の移植を使う
            let node_highs = light && self.fc_node_sep && env_str!("ENOMOTO_MIP_FC_NODE_PATH_HIGHS").is_some();
            let old_only = env_str!("ENOMOTO_MIP_PATH_OLD").is_some() || (light && !node_highs);
            let highs_only = env_str!("ENOMOTO_MIP_PATH_HIGHS_ONLY").is_some();
            if !old_only {
                self.path_aggregation_highs(&vars, &lp_rows, &mut cands, &mut push);
            }
            if !highs_only || old_only {
                // 根で並列の分離なら、集約した行を順に集めてからカットの生成と仕上げを並列に行う (集約は逐次と同じ順で、
                // カットの有無で経路を変えないので結果は逐次と同じ。`ENOMOTO_MIP_PATH_STOP_ON_CUT` では逐次)
                let par_path = self.params.threads > 1
                    && !light
                    && !self.params.submip
                    && self.pool.is_none()
                    && !gen_old()
                    && env_str!("ENOMOTO_MIP_PATH_STOP_ON_CUT").is_none()
                    && env_str!("ENOMOTO_MIP_SEP_SEQ").is_none()
                    && env_str!("ENOMOTO_MIP_PATH_SEQ").is_none();
                if par_path {
                    self.path_collect = Some(Vec::new());
                    self.path_aggregation(&vars, &lp_rows, &mut cands, &mut push, max_starts);
                    let bases = self.path_collect.take().unwrap_or_default();
                    let inc = self.incumbent.as_ref().map(|(_, v)| v.as_slice());
                    let (by_row, cpu) = gen_cuts_parallel(&bases, self.params.threads, &vars, use_flow_cover(), n, &lp_rows, &self.cut_int, &self.dom.global_lo, &self.dom.global_up, x, inc, self.p);
                    self.sep_par_cpu += cpu;
                    for cs in by_row {
                        cands.extend(cs);
                    }
                } else {
                    self.path_aggregation(&vars, &lp_rows, &mut cands, &mut push, max_starts);
                }
            }
        }
        if dbg_sep {
            eprintln!("SEP path {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        // tableau 行 (診断用 `ENOMOTO_MIP_DEBUG_NO_TAB_CUTS`: 使わない)
        let mut basics: Vec<(usize, f64)> = Vec::new();
        let tab_mr = if env_str!("ENOMOTO_MIP_DEBUG_NO_TAB_CUTS").is_some() { 0 } else { mr };
        for s in 0..tab_mr {
            let k = self.lp.basic_var(s);
            if k < n && self.cut_int[k] {
                let v = x[k];
                let f = v - v.floor();
                if f > 1e-3 && f < 1.0 - 1e-3 {
                    basics.push((s, f * (1.0 - f) / self.lp.dse_weight(s).max(1e-8)));
                }
            }
        }
        let nint = p.nint;
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
        // 根の分離 (木を分け合う前、サブ MIP でない) で既定の行の選び方なら、tableau 行のカットの生成と仕上げ (分離の時間の
        // 大半: 30n20b8・blp-ar98・graph20-20-1rand で 8 割) を複数のスレッドで行う (`ENOMOTO_MIP_SEP_SEQ` で逐次)。
        // 行の選択と集約は逐次で今までと同じ順に行い、行ごとの候補 (keep_best 後) を同じ順につなぐので結果は逐次と同じ
        let par_threads = self.params.threads.max(1);
        let par = par_threads > 1
            && !light
            && !self.params.submip
            && self.pool.is_none()
            && !gen_old()
            && !tab_rules
            && !nnz_budget.is_finite()
            && env_str!("ENOMOTO_MIP_SEP_SEQ").is_none();
        if par {
            let mut bases: Vec<Vec<(usize, f64)>> = Vec::new();
            for &(s, _) in basics.iter() {
                if self.time_up() || n_rows_used >= limit {
                    break;
                }
                let w = self.lp.basis_inverse_row(s);
                let wmax = w.iter().fold(0.0f64, |m, v| m.max(v.abs()));
                if wmax == 0.0 {
                    continue;
                }
                let wmin_keep = 1e-9 * wmax;
                let mut touched: Vec<usize> = Vec::new();
                let mut base: Vec<(usize, f64)> = Vec::new();
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
                let ok = wmax / wnz_min <= 1e6;
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
                // 最適面の頂点での分離の照合 (逐次の経路と同じ)
                if self.tab_memo_mode != 0 {
                    use std::hash::{Hash, Hasher};
                    let q = |v: f64| -> i64 { (v * 1e9).round() as i64 };
                    let mut hs = std::collections::hash_map::DefaultHasher::new();
                    for &(j, a) in &base {
                        j.hash(&mut hs);
                        q(a).hash(&mut hs);
                        q(vars.x[j]).hash(&mut hs);
                    }
                    let h = hs.finish();
                    let k = self.lp.basic_var(s);
                    if self.tab_memo_mode == 1 {
                        self.tab_memo.insert(k, h);
                    } else {
                        self.tab_memo_stats.0 += 1;
                        if self.tab_memo.get(&k) == Some(&h) {
                            self.tab_memo_stats.1 += 1;
                            if env_str!("ENOMOTO_MIP_FACE_TAB_SKIP").is_some() {
                                continue;
                            }
                        }
                    }
                }
                n_rows_used += 1;
                base_nnz_total += base.len();
                bases.push(base);
            }
            let t0 = std::time::Instant::now();
            let (cut_int, glo, gup) = (&self.cut_int[..], &self.dom.global_lo[..], &self.dom.global_up[..]);
            let inc = self.incumbent.as_ref().map(|(_, v)| v.as_slice());
            let (pp, lp_rows_ref, vars_ref, bases_ref) = (self.p, &lp_rows, &vars, &bases);
            let nb = bases.len();
            let nth = par_threads.min(nb.max(1));
            // 行をスレッドに交互に割り当てる (行はスコア順で、重さが偏らないように)
            let parts: Vec<(Vec<(usize, Vec<Candidate>)>, f64)> = std::thread::scope(|sc| {
                let hs: Vec<_> = (0..nth)
                    .map(|t| {
                        sc.spawn(move || {
                            let c0 = super::cap_clock();
                            let mut scratch = vec![0.0f64; n];
                            let mut out = Vec::new();
                            let mut k = t;
                            while k < nb {
                                let base = &bases_ref[k];
                                let neg: Vec<(usize, f64)> = base.iter().map(|&(j, a)| (j, -a)).collect();
                                let mut cs: Vec<Candidate> = Vec::new();
                                for b in [base, &neg] {
                                    let n0 = cs.len();
                                    for raw in generate_cuts(vars_ref, b, 0.0, tab_fc, false) {
                                        if let Some(c) = finish_cut(raw, n, lp_rows_ref, cut_int, glo, gup, x, inc, &mut scratch) {
                                            cs.push(c);
                                        }
                                    }
                                    keep_best(&mut cs, n0, pp);
                                }
                                out.push((k, cs));
                                k += nth;
                            }
                            (out, super::cap_clock() - c0)
                        })
                    })
                    .collect();
                hs.into_iter().map(|h| h.join().unwrap()).collect()
            });
            let mut by_row: Vec<Vec<Candidate>> = (0..nb).map(|_| Vec::new()).collect();
            for (part, cpu) in parts {
                self.sep_par_cpu += cpu;
                for (k, cs) in part {
                    by_row[k] = cs;
                }
            }
            for cs in by_row {
                cands.extend(cs);
            }
            if dbg_sep {
                t_gen += t0.elapsed().as_secs_f64();
            }
        } else {
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
                // 最適面の頂点での分離: 今の頂点で同じ基底変数の行が、集約した行もその列の LP 値も同じなら、作るカットも
                // 同じなので飛ばす (`ENOMOTO_MIP_FACE_TAB_SKIP`。照合と数えるのは常に)。同じ行の割合は misc07 で 25% (1323 行中 325)、
                // 10teams ではほぼ 0 (2088 行中 2: tableau が密で、1 回のピボットでほぼ全ての行が変わる)。分離の時間はほとんど
                // 変わらないので既定では飛ばさない
                if self.tab_memo_mode != 0 {
                    use std::hash::{Hash, Hasher};
                    let q = |v: f64| -> i64 { (v * 1e9).round() as i64 };
                    let mut hs = std::collections::hash_map::DefaultHasher::new();
                    for &(j, a) in &base {
                        j.hash(&mut hs);
                        q(a).hash(&mut hs);
                        q(vars.x[j]).hash(&mut hs);
                    }
                    let h = hs.finish();
                    let k = self.lp.basic_var(s);
                    if self.tab_memo_mode == 1 {
                        self.tab_memo.insert(k, h);
                    } else {
                        self.tab_memo_stats.0 += 1;
                        if self.tab_memo.get(&k) == Some(&h) {
                            self.tab_memo_stats.1 += 1;
                            if env_str!("ENOMOTO_MIP_FACE_TAB_SKIP").is_some() {
                                continue;
                            }
                        }
                    }
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

/// 集約した行 `bases` ごとに (その行と符号を反転した行から) カットを作って仕上げ、行ごとに keep_best で絞った候補を
/// 行の順に返す (`nth` 本のスレッドに交互に割り当てる。結果は逐次と同じ)。2 つ目の戻り値はスレッドの CPU 時間の合計。
#[allow(clippy::too_many_arguments)]
fn gen_cuts_parallel(
    bases: &[Vec<(usize, f64)>],
    nth: usize,
    vars: &CutVars,
    flow_cover: bool,
    n: usize,
    lp_rows: &[Vec<(usize, f64)>],
    cut_int: &[bool],
    glo: &[f64],
    gup: &[f64],
    x: &[f64],
    inc: Option<&[f64]>,
    p: &MipProblem,
) -> (Vec<Vec<Candidate>>, f64) {
    let nb = bases.len();
    let nth = nth.min(nb.max(1));
    let parts: Vec<(Vec<(usize, Vec<Candidate>)>, f64)> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..nth)
            .map(|t| {
                sc.spawn(move || {
                    let c0 = super::cap_clock();
                    let mut scratch = vec![0.0f64; n];
                    let mut out = Vec::new();
                    let mut k = t;
                    while k < nb {
                        let base = &bases[k];
                        let neg: Vec<(usize, f64)> = base.iter().map(|&(j, a)| (j, -a)).collect();
                        let mut cs: Vec<Candidate> = Vec::new();
                        for b in [base, &neg] {
                            let n0 = cs.len();
                            for raw in generate_cuts(vars, b, 0.0, flow_cover, false) {
                                if let Some(c) = finish_cut(raw, n, lp_rows, cut_int, glo, gup, x, inc, &mut scratch) {
                                    cs.push(c);
                                }
                            }
                            keep_best(&mut cs, n0, p);
                        }
                        out.push((k, cs));
                        k += nth;
                    }
                    (out, super::cap_clock() - c0)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut by_row: Vec<Vec<Candidate>> = (0..nb).map(|_| Vec::new()).collect();
    let mut cpu = 0.0;
    for (part, c) in parts {
        cpu += c;
        for (k, cs) in part {
            by_row[k] = cs;
        }
    }
    (by_row, cpu)
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
    // 小さい係数は境界で吸収して消す (HiGHS の `postprocessCut` は 100·feastol·max(cmax, 1e-3) ≈ 1e-4·cmax 以下を消す)。
    // カットは Σ c x <= rhs の向きなので、c_j x_j >= min(c_j lo_j, c_j up_j) (c_j > 0 なら下限、c_j < 0 なら上限) から
    // Σ_{k≠j} c_k x_k <= rhs - c_j x_j <= rhs - min(...) となり、右辺を緩める向きにだけ動く。
    // 閾値は `ENOMOTO_T_MIP_CUT_SMALL_REL` (既定 1e-4、1e-9 で以前と同じ)。その境界が無限のときは、以前の閾値 1e-9 以下なら
    // カットを捨て、それより大きければ係数を残す (以前より多くは捨てない)
    let small_rel = tunable!("ENOMOTO_T_MIP_CUT_SMALL_REL", 1e-4, f64);
    let mut coefs: Vec<(usize, f64)> = Vec::with_capacity(dense.len());
    for (j, c) in dense {
        if c.abs() <= small_rel * cmax {
            let m = if c > 0.0 { c * lo[j] } else { c * up[j] };
            if m.is_finite() {
                rhs -= m;
                continue;
            }
            if c.abs() <= 1e-9 * cmax {
                return None;
            }
        }
        coefs.push((j, c));
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
    // 係数の幅 (dynamism) が大きすぎるカットは数値的に危ないので捨てる (小さい係数を吸収した後で判定する)
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
    Some(Candidate { coefs, rhs, efficacy, forced: false })
}

/// 効き目の大きい順に、既に選んだものとほぼ平行 (|cos| > 0.99) なものを除いて選ぶ。
fn select_cuts(cands: Vec<Candidate>, max_cuts: usize, p: &MipProblem) -> Vec<Candidate> {
    if env_str!("ENOMOTO_MIP_CUTSEL_OLD").is_some() {
        return select_cuts_old(cands, max_cuts);
    }
    select_cuts_hybrid(cands, max_cuts, p)
}

/// 固定費用ネットワーク型か: 連続変数 1 つと整数変数 1 つからなる行 (VUB/VLB) が行の
/// `ENOMOTO_T_FC_ROW_FRAC` (0.7) 以上で、連続変数の `ENOMOTO_T_FC_COL_FRAC` (0.9) 以上がそのような行を持つ。
fn fixed_charge_network(p: &MipProblem) -> bool {
    fixed_charge_network_with(p, tunable!("ENOMOTO_T_FC_ROW_FRAC", 0.7, f64))
}

/// 木のノードの分離を強める固定費用ネットワーク型の判定 ([`Solver::node_cuts_due`]): [`fixed_charge_network`] と同じだが、
/// 行の割合のしきい値は `ENOMOTO_T_FC_NODE_ROW_FRAC` (0.5)。流量保存の行も数えるので、弧に対して節点の多いネットワークでは
/// VUB の行の割合が下がる (beasleyC3: 1473 行中 852 行 = 58%、連続変数は 852 本すべてが VUB を持つ)
pub(super) fn fixed_charge_network_for_nodes(p: &MipProblem) -> bool {
    fixed_charge_network_with(p, tunable!("ENOMOTO_T_FC_NODE_ROW_FRAC", 0.5, f64))
}

fn fixed_charge_network_with(p: &MipProblem, row_frac: f64) -> bool {
    // 前処理で消えた列 (行に現れない、固定された) は数えない
    let ncont = (0..p.n).filter(|&j| !p.is_int[j] && !p.cols[j].is_empty() && p.col_lo[j] < p.col_up[j]).count();
    if ncont == 0 || p.m == 0 {
        return false;
    }
    let mut has_vb = vec![false; p.n];
    let mut nrows = 0usize;
    for r in &p.rows {
        if r.len() != 2 {
            continue;
        }
        let (a, b) = (r[0].0, r[1].0);
        let c = match (p.is_int[a], p.is_int[b]) {
            (false, true) => a,
            (true, false) => b,
            _ => continue,
        };
        nrows += 1;
        has_vb[c] = true;
    }
    let ncol = has_vb.iter().filter(|&&v| v).count();
    if env_str!("ENOMOTO_MIP_LOG").is_some() {
        eprintln!("MIP: fixed-charge check: vb rows {nrows} of {}, continuous with vb {ncol} of {ncont}", p.m);
    }
    nrows as f64 >= row_frac * p.m as f64 && ncol as f64 >= tunable!("ENOMOTO_T_FC_COL_FRAC", 0.9, f64) * ncont as f64
}

/// 疎なベクトル (列番号の昇順) の内積。
/// 2 本のカットの係数の向きの余弦 (絶対値)。
fn cut_parallelism(a: &[(usize, f64)], b: &[(usize, f64)]) -> f64 {
    let na = a.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
    let nb = b.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    sparse_dot(a, b).abs() / (na * nb)
}

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
    select_cuts_hybrid_with(cands, max_cuts, p, tunable!("ENOMOTO_T_CUTSEL_MIN_REL", 0.5, f64), tunable!("ENOMOTO_T_CUTSEL_MAXPAR", 0.3, f64))
}

/// [`select_cuts_hybrid`] の質の閾値 (最良の何倍未満を捨てるか) と平行度の上限を指定する版。
fn select_cuts_hybrid_with(cands: Vec<Candidate>, max_cuts: usize, p: &MipProblem, min_rel: f64, maxpar: f64) -> Vec<Candidate> {
    select_cuts_hybrid_anchor(cands, max_cuts, p, min_rel, maxpar, None)
}

/// [`select_cuts_hybrid`] のスコア。
fn cut_score(c: &Candidate, p: &MipProblem, cnorm: f64) -> f64 {
    // 密なカットを避ける (`ENOMOTO_T_CUTSEL_DENSITY_EXP` = α: スコアを 非零の数^α で割る。HiGHS は効いている
    // 非零の数で割る (α = 1 相当)。密なカットは LP の反復が増え、カットの行から作る次のカットも密になる)
    cut_quality(c, p, cnorm) / (c.coefs.len().max(1) as f64).powf(tunable!("ENOMOTO_T_CUTSEL_DENSITY_EXP", 0.0, f64))
}

/// [`select_cuts_hybrid_with`] の、質の閾値の基準のスコアを `anchor` で与えられる版 (`None` なら候補の最良のスコア)。
fn select_cuts_hybrid_anchor(cands: Vec<Candidate>, max_cuts: usize, p: &MipProblem, min_rel: f64, maxpar: f64, anchor: Option<f64>) -> Vec<Candidate> {
    let cnorm = p.cost.iter().map(|c| c * c).sum::<f64>().sqrt();
    let mut scored: Vec<(f64, f64, Candidate)> = cands
        .into_iter()
        .map(|c| {
            let nc = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
            (cut_score(&c, p, cnorm), nc, c)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let Some(best_score) = scored.first().map(|s| s.0) else { return Vec::new() };
    let best_score = anchor.map_or(best_score, |a| a.min(best_score));
    if env_str!("ENOMOTO_MIP_DEBUG_CUTSEL").is_some() {
        let above = scored.iter().filter(|s| s.0 >= min_rel * best_score).count();
        let top: Vec<String> = scored
            .iter()
            .take(6)
            .map(|(s, nc, c)| {
                let objpar = if cnorm > 0.0 && *nc > 0.0 { c.coefs.iter().map(|&(j, v)| v * p.cost[j]).sum::<f64>().abs() / (cnorm * nc) } else { 0.0 };
                format!("{s:.4}(eff {:.4} op {objpar:.2} nnz {})", c.efficacy, c.coefs.len())
            })
            .collect();
        let max_eff = scored.iter().map(|s| s.2.efficacy).fold(0.0, f64::max);
        eprintln!("CUTSEL: {} cands, {above} above {min_rel} x best, max eff {max_eff:.4}; top {}", scored.len(), top.join(" "));
    }
    // 質が最良の 0.9 倍以上の「良い」カットの平行度の上限 (`maxpar` より小さくてもそのまま使う。固定費用型の緩めた選択で
    // maxpar に合わせて 0.9 にすると sp150x300d の根の下界が 65.3 -> 58.0 に下がった)
    let (goodmaxpar, good) = (tunable!("ENOMOTO_T_CUTSEL_GOODMAXPAR", 0.7, f64), 0.9 * best_score);
    // `ENOMOTO_T_CUTSEL_MIN_REL` (既定 0.5): 質が最良の何倍未満のカットは採らない (Wesselmann・Suhl の MOPS と同じ
    // 最良の 50%)。弱いカットを LP に入れず、1 ラウンドの LP を軽くする (根の LP の反復: qnet1 1936 -> 1149、
    // h80x6320d 7202 -> 3378)。40 問 2 回: sgeomean 27.84 -> 26.72、28.10 -> 26.81 (misc07 36 -> 11 秒、mik-250 33 -> 25 秒)。
    // 0.3 は 27.13。0 なら以前と同じ (上限 50 本まで全部)
    let min_score = min_rel * best_score;
    // `ENOMOTO_T_CUTSEL_FILL_FRAC` = f (> 0): 相対閾値で採ったカットが max_cuts·f 本未満なら、閾値を下げて
    // (候補の上位半分まで) max_cuts·f 本になるまで足す (HiGHS の `HighsCutPool::separate` の「少なすぎれば上位半分」)。
    // p0201 は候補 130-480 本から 1-4 本しか採らず、25 ラウンドかけても根の下界が伸び切らなかった
    let fill = (tunable!("ENOMOTO_T_CUTSEL_FILL_FRAC", 0.0, f64) * max_cuts as f64).round() as usize;
    let half = scored.len().div_ceil(2);
    let dbg = env_str!("ENOMOTO_MIP_DEBUG_CUTSEL").is_some();
    let mut n_par = 0usize;
    let mut chosen: Vec<(f64, Candidate)> = Vec::new();
    let mut rest: Vec<(f64, f64, Candidate)> = Vec::new();
    for (k, (score, nc, c)) in scored.into_iter().enumerate() {
        if chosen.len() >= max_cuts {
            break;
        }
        if score < min_score {
            if fill > 0 && k < half {
                rest.push((score, nc, c));
                continue;
            }
            break;
        }
        let limit = if score >= good { goodmaxpar } else { maxpar };
        let parallel = chosen.iter().any(|(nd, d)| sparse_dot(&c.coefs, &d.coefs).abs() > limit * nc * nd);
        if !parallel {
            chosen.push((nc, c));
        } else {
            n_par += 1;
        }
    }
    if dbg {
        eprintln!("CUTSEL: chosen {} (parallel rejected {n_par})", chosen.len());
    }
    if chosen.len() < fill {
        for (_, nc, c) in rest {
            if chosen.len() >= fill {
                break;
            }
            let parallel = chosen.iter().any(|(nd, d)| sparse_dot(&c.coefs, &d.coefs).abs() > maxpar * nc * nd);
            if !parallel {
                chosen.push((nc, c));
            }
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
