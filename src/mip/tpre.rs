//! 試作 `ENOMOTO_MIP_TPRE=文字` (analysis/objtarget_20261119/ の 5・6、既定では使わない): 並列の「仮の暫定値つき前処理」。
//!
//! 最初の求解 (サブ MIP・再スタートでない、2 スレッド以上) の前処理の始めに、1 本のスレッドで元の問題 `p` の写しについて
//!
//! 1. 下界 L: 元の問題の LP を 1 スレッドの PDLP で `ENOMOTO_T_MIP_TPRE_PDLP_ITERS` 反復まで解き、確認点ごとの双対 y の
//!    ラグランジュ下界 L(y) = bᵀy + Σ_j min_{境界の中} (c − Aᵀy)_j x_j の最良 (どの y でも正しい下界。src/simplex/mip_lp.rs の `lagrange_std`)。
//! 2. 上界 U: FJ (`fj2_search`、手間は非零の数で決まる) の解の目的値。
//! 3. 目標 T = U − α (U − L) (α = `ENOMOTO_T_MIP_TPRE_ALPHA`、既定 0.5)。U・L のどちらかがなければ何もしない。
//! 4. 問題 P_T = P ∩ {c x ≤ T} の定義域の伝播 (目的の行が強く効く)。矛盾すれば「T 以下の解はない」(最適値 > T の証明)。
//! 5. P_T の締まった境界の中で FJ を `ENOMOTO_T_MIP_TPRE_FJ_RUNS` 回 (種を変えて) 回す。解があれば T 以下の解。
//!
//! 主の側 (src/mip/mod.rs `solve_problem_inner`) は前処理の後、開始解を前処理後の空間に写す所で、このスレッドの終わりを待って
//! (待つ所が決まっているので種ごとに再現する) 結果を取り込む: 一番良い解 (T 以下の解か U の解) を開始解にする。`b` なら、T 以下の
//! 解があったときだけ、P_T の伝播で締まった境界を前処理後の問題に持ち込む。
//!
//! 持ち込みの正しさ (`b`): T 以下の解 x̂ があるので最適値 z* ≤ c x̂ ≤ T、どの最適解も {c x ≤ T} の中にある。P_T の伝播で導いた
//! 境界は「P_T の全ての解」が満たす (主の簡約: 行と境界だけから導く) ので、P の全ての最適解も満たす。主の前処理の簡約には
//! 「最適解の 1 つを残す」双対の簡約 (支配された列の固定・双対の固定など) も含まれるが、それが残す最適解 S* も最適解なので
//! P_T の境界を満たし、両方を合わせても S* は残る。持ち込んでよいのは主の簡約 (伝播の固定・境界の締め) だけで、P_T の上で
//! 導いた双対の簡約 (最適解の 1 つを残すもの) は、主の前処理の双対の簡約と別の最適解を残しうるので、合わせると最適解を
//! 全て消しうる (持ち込まない。この試作は P_T では伝播だけをする)。開始解 x̂ は P_T の解なので持ち込んだ境界を満たす。
//! 前処理後の問題は列の番号を保ち x = d x' と尺度を変える (`scaling.d`) ので、境界は d で割って写す。前処理で固定・代入された列
//! (前処理後に境界が 1 点の列) には持ち込まない。

use super::problem::MipProblem;

pub(crate) fn mode() -> Option<&'static str> {
    env_str!("ENOMOTO_MIP_TPRE")
}

/// 結果
pub(crate) struct TpreOut {
    pub l: Option<f64>,
    pub l_iters: usize,
    pub u: Option<f64>,
    pub t: Option<f64>,
    /// 一番良い解 (元の問題の空間) と目的値、それが T 以下か
    pub x: Option<Vec<f64>>,
    pub x_z: f64,
    pub below_t: bool,
    /// P_T の伝播で締まった境界 (T 以下の解があったときだけ持ち込める)
    pub lo: Vec<f64>,
    pub up: Vec<f64>,
    pub n_tight: usize,
    pub n_fix: usize,
    /// P_T の伝播が矛盾した (T 以下の解はない)
    pub infeasible_t: bool,
    /// (PDLP, FJ (U), 伝播, FJ (P_T)) の秒
    pub secs: [f64; 4],
}

/// 診断 `ENOMOTO_MIP_TPRE` の `u` (analysis/objtarget_20261119/ の 7): 元の問題の LP 緩和で、最小化の下界 L と最大化の上界 U_max
/// (目的の符号を反転した LP のラグランジュ下界の符号を戻したもの。どの双対でも正しい上界) を、PDLP の反復ごとに出す。
/// 最大化が非有界なら、どの双対でも U_max = +inf (ラグランジュの形で無限の境界の向きに被約費用が残る)
pub(crate) fn umax_diag(p: &MipProblem) {
    use std::sync::{Arc, Mutex};
    let iters = tunable!("ENOMOTO_T_MIP_TPRE_PDLP_ITERS", 20000usize, usize);
    let caps = [500usize, 2000, 5000, 20000];
    let at = |h: &[(usize, f64)], c: usize| h.iter().filter(|&&(k, _)| k <= c).map(|&(_, v)| v).fold(f64::NEG_INFINITY, f64::max);
    let t0 = std::time::Instant::now();
    let lp = crate::simplex::mip_lp::TwoStageLp::new(&p.col_lo, &p.col_up, &p.cost, &p.rows, &p.row_lo, &p.row_up);
    let h1 = Arc::new(Mutex::new(Vec::new()));
    let _ = lp.pdlrc_run_iters_hist(iters, Some(h1.clone()), None);
    let t1 = t0.elapsed().as_secs_f64();
    let neg: Vec<f64> = p.cost.iter().map(|c| -c).collect();
    let lp2 = crate::simplex::mip_lp::TwoStageLp::new(&p.col_lo, &p.col_up, &neg, &p.rows, &p.row_lo, &p.row_up);
    let h2 = Arc::new(Mutex::new(Vec::new()));
    let _ = lp2.pdlrc_run_iters_hist(iters, Some(h2.clone()), None);
    let t2 = t0.elapsed().as_secs_f64() - t1;
    let h1 = h1.lock().map(|h| h.clone()).unwrap_or_default();
    let h2 = h2.lock().map(|h| h.clone()).unwrap_or_default();
    let mut s = format!("MIP: TPRE_UMAX n {} m {} nnz {} offset {:.10e}", p.n, p.m, p.rows.iter().map(|r| r.len()).sum::<usize>(), p.offset);
    for &c in &caps {
        let l = at(&h1, c) + p.offset;
        let u = -at(&h2, c) + p.offset;
        s.push_str(&format!(" | it{c} L {l:.10e} Umax {u:.10e}"));
    }
    s.push_str(&format!(" | iters_min {} iters_max {} secs_min {t1:.2} secs_max {t2:.2}", h1.last().map_or(0, |x| x.0), h2.last().map_or(0, |x| x.0)));
    eprintln!("{s}");
}

/// 診断 `ENOMOTO_MIP_TPRE` の `s` (analysis/objtarget_20261119/ の 8): FJ と Local-MIP を出発点を変えて回し、出発点で破られている行の数・
/// 最初の解までの手の数・時間・最良の目的値を出す。出発点: 既定 (境界の端) / 単体法の LP の点 / PDLP の点 (500・2000・10000 反復) /
/// 目的を 0 にした PDLP の点 (2000 反復)。Local-MIP は `ENOMOTO_T_MIP_TPRE_LMIP_SECS` 秒 (既定 5、時刻で止める)
pub(crate) fn start_diag(p: &MipProblem, safety: f64, seed: u64) {
    use super::lp::{LpStatus, SolveLimits};
    use super::lp_api::MipLp;
    use std::sync::{Arc, Mutex};
    let mut starts: Vec<(String, Option<Vec<f64>>, f64)> = vec![("default".into(), None, 0.0)];
    let t = std::time::Instant::now();
    let mut lp = crate::simplex::mip_lp::TwoStageLp::new(&p.col_lo, &p.col_up, &p.cost, &p.rows, &p.row_lo, &p.row_up);
    let st = lp.solve(&SolveLimits { iteration_limit: tunable!("ENOMOTO_T_MIP_TPRE_LP_ITERS", 200_000u64, u64), ..Default::default() });
    if st == LpStatus::Optimal {
        starts.push(("lp".into(), Some(lp.col_values()), t.elapsed().as_secs_f64()));
    }
    let zero = vec![0.0; p.n];
    for (name, cost, it) in [("pdlp500", &p.cost, 500usize), ("pdlp2000", &p.cost, 2000), ("pdlp10000", &p.cost, 10000), ("pdlp0_2000", &zero, 2000)] {
        let t = std::time::Instant::now();
        let lp = crate::simplex::mip_lp::TwoStageLp::new(&p.col_lo, &p.col_up, cost, &p.rows, &p.row_lo, &p.row_up);
        let xo = Arc::new(Mutex::new(Vec::new()));
        let _ = lp.pdlrc_run_iters_hist(it, None, Some(xo.clone()));
        let x = xo.lock().map(|x| x.clone()).unwrap_or_default();
        if x.len() == p.n {
            starts.push((name.into(), Some(x), t.elapsed().as_secs_f64()));
        }
    }
    let lmip_secs = tunable!("ENOMOTO_T_MIP_TPRE_LMIP_SECS", 5.0, f64);
    for (name, x0, secs0) in &starts {
        let t1 = std::time::Instant::now();
        let fx = super::fj::fj2_search_from(p, &p.col_lo, &p.col_up, x0.as_deref(), safety, seed, None);
        let fsecs = t1.elapsed().as_secs_f64();
        let (sv, fs, fe, steps) = super::fj::FJ_LAST.with(|c| c.get());
        let fz = fx.as_ref().filter(|x| p.is_feasible(x, 1e-6)).map(|x| p.objective(x));
        let t2 = std::time::Instant::now();
        let (lv, lfirst, lz) = match super::localmip::LocalMip::new(p, &p.col_lo, &p.col_up, seed ^ 0x6A09_E667_F3BC_C909, true) {
            Some(mut lm) => {
                let v = match x0 { Some(x) => lm.set_start(x), None => usize::MAX };
                lm.run(std::time::Instant::now() + std::time::Duration::from_secs_f64(lmip_secs), None, &|_| {});
                let z = lm.res.best.as_ref().map(|x| p.objective(x));
                (v, lm.res.first_time, z)
            }
            None => (usize::MAX, None, None),
        };
        let _ = t2;
        eprintln!(
            "MIP: TPRE_START {name}: prep {secs0:.2}s | FJ start_viol {sv} first_steps {} first_effort {} steps {steps} found {:?} ({fsecs:.2}s) | LMIP start_viol {} first {:?} best {:?}",
            if fs == u64::MAX { -1 } else { fs as i64 },
            if fe == u64::MAX { -1 } else { fe as i64 },
            fz,
            if lv == usize::MAX { -1 } else { lv as i64 },
            lfirst,
            lz
        );
    }
}

pub(crate) fn run(p: &MipProblem, safety: f64, seed: u64) -> TpreOut {
    if mode().is_some_and(|m| m.contains('u')) {
        umax_diag(p);
    }
    if mode().is_some_and(|m| m.contains('s')) {
        start_diag(p, safety, seed);
    }
    // `p` (analysis/objtarget_20261119/ の 8 の試作): PDLP の点を出発点にした FJ だけを回す。元の問題の LP を 1 スレッドの PDLP で
    // `ENOMOTO_T_MIP_TPRE_PDLP_ITERS` 反復 (既定 2000) 解いた主の点 (整数列は四捨五入) から FJ を `ENOMOTO_T_MIP_TPRE_FJ_RUNS` 回 (種を変えて)。
    // 手間は反復の数と FJ の手間 (非零の数で決まる) で決まり、主の側は決まった所で待つので種ごとに再現する
    if mode().is_some_and(|m| m.contains('p')) {
        return pdlp_start(p, safety, seed);
    }
    let mut out = TpreOut { l: None, l_iters: 0, u: None, t: None, x: None, x_z: f64::INFINITY, below_t: false, lo: Vec::new(), up: Vec::new(), n_tight: 0, n_fix: 0, infeasible_t: false, secs: [0.0; 4] };
    if !p.cost.iter().any(|&c| c != 0.0) {
        return out;
    }
    // 1. PDLP のラグランジュ下界
    let t0 = std::time::Instant::now();
    let iters = tunable!("ENOMOTO_T_MIP_TPRE_PDLP_ITERS", 2000usize, usize);
    if iters > 0 {
        let lp = crate::simplex::mip_lp::TwoStageLp::new(&p.col_lo, &p.col_up, &p.cost, &p.rows, &p.row_lo, &p.row_up);
        if let Some(b) = lp.pdlrc_run_iters(iters) {
            out.l = Some(b.lb + p.offset);
            out.l_iters = b.iters;
        }
    }
    out.secs[0] = t0.elapsed().as_secs_f64();
    // 2. FJ の上界
    let t1 = std::time::Instant::now();
    if let Some(x) = super::fj::fj2_search(p, &p.col_lo, &p.col_up, safety, seed, None) {
        if p.is_feasible(&x, 1e-6) {
            let z = p.objective(&x);
            out.u = Some(z);
            out.x_z = z;
            out.x = Some(x);
        }
    }
    out.secs[1] = t1.elapsed().as_secs_f64();
    // 9 (`d`) で FJ の解がなければ、U に LP 緩和の最大化の上界 U_max (7、正しい上界。非有界なら使えない) を使う
    let u_for_t = match out.u {
        Some(u) => Some(u),
        None if mode().is_some_and(|m| m.contains('d')) => {
            let neg: Vec<f64> = p.cost.iter().map(|c| -c).collect();
            let lp2 = crate::simplex::mip_lp::TwoStageLp::new(&p.col_lo, &p.col_up, &neg, &p.rows, &p.row_lo, &p.row_up);
            lp2.pdlrc_run_iters(iters).map(|b| -b.lb + p.offset).filter(|u| u.is_finite())
        }
        None => None,
    };
    let (Some(l), Some(u)) = (out.l, u_for_t) else { return out };
    if !(u > l + 1e-9 * u.abs().max(1.0)) {
        return out;
    }
    // 9 (`d`): T を下界の近くから順に上げる (θ = 1/16, 1/8, 1/4, 1/2 で T = L + θ (U − L))。各段で P_T を伝播し、矛盾なら
    // 「最適値 > T」(下界を T に上げて次へ)、そうでなければ P_T の中の FJ で T 以下の解を探し、出たら止める
    if mode().is_some_and(|m| m.contains('d')) {
        ladder(p, l, u, safety, seed, &mut out);
        return out;
    }
    // 3. 目標 T
    let alpha = tunable!("ENOMOTO_T_MIP_TPRE_ALPHA", 0.5, f64);
    let t = u - alpha * (u - l);
    out.t = Some(t);
    // 4. P_T の伝播
    let t2 = std::time::Instant::now();
    let mut rows = p.rows.clone();
    let mut rlo = p.row_lo.clone();
    let mut rup = p.row_up.clone();
    rows.push((0..p.n).filter(|&j| p.cost[j] != 0.0).map(|j| (j, p.cost[j])).collect());
    rlo.push(f64::NEG_INFINITY);
    rup.push(t - p.offset);
    let q = MipProblem::from_rows(p.col_lo.clone(), p.col_up.clone(), p.cost.clone(), p.offset, p.sense_sign, p.is_int.clone(), rows, rlo, rup);
    let mut d = super::domain::Domain::new(&q);
    if d.infeasible || !d.propagate(&q) {
        out.infeasible_t = true;
        out.secs[2] = t2.elapsed().as_secs_f64();
        return out;
    }
    out.lo = d.lo.clone();
    out.up = d.up.clone();
    for j in 0..p.n {
        if out.lo[j] > p.col_lo[j] + 1e-9 || out.up[j] < p.col_up[j] - 1e-9 {
            out.n_tight += 1;
            if out.lo[j] == out.up[j] && p.col_lo[j] < p.col_up[j] {
                out.n_fix += 1;
            }
        }
    }
    out.secs[2] = t2.elapsed().as_secs_f64();
    // 5. P_T の中の FJ
    let t3 = std::time::Instant::now();
    let runs = tunable!("ENOMOTO_T_MIP_TPRE_FJ_RUNS", 2usize, usize);
    for k in 0..runs {
        if let Some(x) = super::fj::fj2_search(&q, &out.lo, &out.up, safety, seed.wrapping_add((k as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)), None) {
            if p.is_feasible(&x, 1e-6) {
                let z = p.objective(&x);
                if z <= t + 1e-6 * t.abs().max(1.0) && z < out.x_z {
                    out.x_z = z;
                    out.x = Some(x);
                    out.below_t = true;
                }
            }
        }
    }
    out.secs[3] = t3.elapsed().as_secs_f64();
    out
}

/// P = p ∩ {c x ≤ t} を作る
fn with_obj_row(p: &MipProblem, t: f64) -> MipProblem {
    let mut rows = p.rows.clone();
    let mut rlo = p.row_lo.clone();
    let mut rup = p.row_up.clone();
    rows.push((0..p.n).filter(|&j| p.cost[j] != 0.0).map(|j| (j, p.cost[j])).collect());
    rlo.push(f64::NEG_INFINITY);
    rup.push(t - p.offset);
    MipProblem::from_rows(p.col_lo.clone(), p.col_up.clone(), p.cost.clone(), p.offset, p.sense_sign, p.is_int.clone(), rows, rlo, rup)
}

/// 9 の段 (`d`)。`out.l` は伝播の矛盾で上がった下界 (証明つき) に置き換える。`out.t` は解の出た段の T
fn ladder(p: &MipProblem, l0: f64, u: f64, safety: f64, seed: u64, out: &mut TpreOut) {
    let mut l = l0;
    let t2 = std::time::Instant::now();
    let mut stages = String::new();
    for (k, theta) in [1.0 / 16.0, 1.0 / 8.0, 0.25, 0.5].into_iter().enumerate() {
        let t = l0 + theta * (u - l0);
        if t <= l {
            continue;
        }
        let q = with_obj_row(p, t);
        let mut d = super::domain::Domain::new(&q);
        if d.infeasible || !d.propagate(&q) {
            // P_T に解はない: 最適値 > T
            l = t;
            stages.push_str(&format!(" θ{theta}:inf"));
            continue;
        }
        let ntight = (0..p.n).filter(|&j| d.lo[j] > p.col_lo[j] + 1e-9 || d.up[j] < p.col_up[j] - 1e-9).count();
        let mut found = false;
        for r in 0..tunable!("ENOMOTO_T_MIP_TPRE_FJ_RUNS", 2usize, usize) {
            let sd = seed.wrapping_add(((k * 8 + r) as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
            if let Some(x) = super::fj::fj2_search(&q, &d.lo, &d.up, safety, sd, None) {
                if p.is_feasible(&x, 1e-6) {
                    let z = p.objective(&x);
                    if z <= t + 1e-6 * t.abs().max(1.0) && z < out.x_z {
                        out.x_z = z;
                        out.x = Some(x);
                        out.below_t = true;
                        found = true;
                    }
                }
            }
        }
        stages.push_str(&format!(" θ{theta}:{}(tight {ntight})", if found { "found" } else { "none" }));
        if found {
            out.t = Some(t);
            out.lo = d.lo.clone();
            out.up = d.up.clone();
            out.n_tight = ntight;
            out.n_fix = (0..p.n).filter(|&j| d.lo[j] == d.up[j] && p.col_lo[j] < p.col_up[j]).count();
            break;
        }
    }
    out.secs[2] = t2.elapsed().as_secs_f64();
    if l > l0 {
        out.infeasible_t = true;
    }
    out.l = Some(l);
    eprintln!("MIP: TPRE ladder: L0 {l0:.10e} U {u:.10e} -> proven L {l:.10e};{stages}");
}

/// [`run`] の `p`: PDLP の点からの FJ
fn pdlp_start(p: &MipProblem, safety: f64, seed: u64) -> TpreOut {
    use std::sync::{Arc, Mutex};
    let mut out = TpreOut { l: None, l_iters: 0, u: None, t: None, x: None, x_z: f64::INFINITY, below_t: false, lo: Vec::new(), up: Vec::new(), n_tight: 0, n_fix: 0, infeasible_t: false, secs: [0.0; 4] };
    let t0 = std::time::Instant::now();
    let iters = tunable!("ENOMOTO_T_MIP_TPRE_PDLP_ITERS", 2000usize, usize);
    let lp = crate::simplex::mip_lp::TwoStageLp::new(&p.col_lo, &p.col_up, &p.cost, &p.rows, &p.row_lo, &p.row_up);
    let xo = Arc::new(Mutex::new(Vec::new()));
    if let Some(b) = lp.pdlrc_run_iters_hist(iters, None, Some(xo.clone())) {
        out.l = Some(b.lb + p.offset);
        out.l_iters = b.iters;
    }
    drop(lp);
    let x0 = xo.lock().map(|x| x.clone()).unwrap_or_default();
    out.secs[0] = t0.elapsed().as_secs_f64();
    if x0.len() != p.n {
        return out;
    }
    let t1 = std::time::Instant::now();
    let mut viol = 0usize;
    for k in 0..tunable!("ENOMOTO_T_MIP_TPRE_FJ_RUNS", 2usize, usize) {
        let sd = seed.wrapping_add((k as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        if let Some(x) = super::fj::fj2_search_from(p, &p.col_lo, &p.col_up, Some(&x0), safety, sd, None) {
            if p.is_feasible(&x, 1e-6) {
                let z = p.objective(&x);
                if z < out.x_z {
                    out.x_z = z;
                    out.u = Some(z);
                    out.x = Some(x);
                }
            }
        }
        if k == 0 {
            viol = super::fj::FJ_LAST.with(|c| c.get().0);
        }
    }
    out.n_tight = viol;
    out.secs[1] = t1.elapsed().as_secs_f64();
    // `pl`: PDLP の点から Local-MIP を `ENOMOTO_T_MIP_TPRE_LMIP_STEPS` 手 (既定 100000、loue・kakapo で 4〜6 秒) 回す (手の数で止める)
    if mode().is_some_and(|m| m.contains("pl")) {
        let t2 = std::time::Instant::now();
        if let Some(mut lm) = super::localmip::LocalMip::new(p, &p.col_lo, &p.col_up, seed ^ 0x6A09_E667_F3BC_C909, true) {
            // 出発点は PDLP の点 (8 の単独の比べで、FJ の解から始めるより良かった)
            lm.set_start(&x0);
            lm.step_cap = tunable!("ENOMOTO_T_MIP_TPRE_LMIP_STEPS", 100_000u64, u64);
            lm.run(std::time::Instant::now() + std::time::Duration::from_secs_f64(safety), None, &|_| {});
            if let Some(x) = lm.res.best.clone() {
                if p.is_feasible(&x, 1e-6) {
                    let z = p.objective(&x);
                    if z < out.x_z {
                        out.x_z = z;
                        out.u = Some(z);
                        out.x = Some(x);
                    }
                }
            }
        }
        out.secs[2] = t2.elapsed().as_secs_f64();
        if env_str!("ENOMOTO_MIP_TPRE_DEBUG").is_some() {
            eprintln!("MIP: TPRE pl: Local-MIP {:.2}s, best {:.10e}", out.secs[2], out.x_z);
        }
    }
    out
}

/// 試作 `ENOMOTO_MIP_TPRE` の `t` (`pt`、analysis/objtarget_20261119/): 前処理の間、空いたスレッド (スレッド数 − 1 本) で FJ・Local-MIP を
/// 出発点を変えて回し続ける (スレッド k の出発点: k mod 4 = 0 PDLP 500 反復の点、1 PDLP 2000 反復、2 目的 0 の PDLP 2000 反復、3 境界の端 (FJ の既定))。
/// 再現性: 各スレッドに手間の門 ([`crate::ticks::Gate`]) を付け、主は前処理の段の終わり ([`super::pretime::Guard`] を捨てるとき) ごとに
/// 「主が前処理で使った手間 × `ENOMOTO_T_MIP_TPRE_PT_RATE` (既定 0.5。1 では手伝いのスレッドが主に追いつかず、satellites2-60-fs で主が 7〜11 秒待った)」まで許す。前処理の終わりで同じ式の手間まで許して門で止まるのを待ち、
/// その時点の最良を読むので、読む状態は時刻によらず主の手間で決まる (スレッドが遅れていれば主が待つ。PDLP は反復の数で止まり門を見ない)。
pub(crate) struct PtJob {
    gates: Vec<std::sync::Arc<crate::ticks::Gate>>,
    best: std::sync::Arc<std::sync::Mutex<Vec<Option<(f64, Vec<f64>)>>>>,
    base: f64,
    granted: std::cell::Cell<f64>,
    rate: f64,
    pdlp_stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for PtJob {
    fn drop(&mut self) {
        for g in &self.gates {
            g.stop();
        }
        self.pdlp_stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

thread_local! {
    static PT_JOB: std::cell::RefCell<Option<std::rc::Weak<PtJob>>> = const { std::cell::RefCell::new(None) };
}

/// PDLP の点の受け渡し (`pt`)。0 = 境界の端 (いつも用意)、1 = PDLP 500 反復、2 = 2000 反復、3 = 目的 0 の 2000 反復、4 = 10000 反復
/// (非零が `ENOMOTO_T_MIP_TPRE_PT_BIG_NNZ` (1e5) を超える問題では作らない)
#[derive(Clone)]
enum Slot {
    Pending,
    Ready(std::sync::Arc<Vec<f64>>),
    Failed,
}

struct Points {
    slots: std::sync::Mutex<Vec<Slot>>,
    cv: std::sync::Condvar,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Points {
    /// 点 `i` を待って返す (失敗・止めなら境界の端 = `None`)。待つ間は計算しないので、使う点は時刻によらず決まる
    fn get(&self, i: usize) -> Option<std::sync::Arc<Vec<f64>>> {
        let mut g = self.slots.lock().unwrap();
        loop {
            match &g[i] {
                Slot::Ready(x) => return Some(x.clone()),
                Slot::Failed => return None,
                Slot::Pending => {}
            }
            g = self.cv.wait(g).unwrap();
        }
    }
    fn set(&self, i: usize, v: Slot) {
        let mut g = self.slots.lock().unwrap();
        g[i] = v;
        self.cv.notify_all();
    }
}

pub(crate) fn pt_spawn(p: &MipProblem, threads: usize, seed: u64) -> std::rc::Rc<PtJob> {
    use std::sync::{Arc, Mutex};
    // スレッド数 − 1 本のうち 1 本は PDLP、残り (最低 1 本) は FJ・Local-MIP
    let k = threads.saturating_sub(2).max(1);
    let best = Arc::new(Mutex::new(vec![None; k]));
    let nnz: usize = p.rows.iter().map(|r| r.len()).sum();
    let big = nnz as f64 > tunable!("ENOMOTO_T_MIP_TPRE_PT_BIG_NNZ", 1e5, f64);
    let pdlp_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let pts = Arc::new(Points { slots: Mutex::new(vec![Slot::Pending; 5]), cv: std::sync::Condvar::new(), stop: pdlp_stop.clone() });
    pts.set(0, Slot::Failed);
    if big {
        pts.set(4, Slot::Failed);
    }
    {
        let q = p.clone();
        let pts = pts.clone();
        std::thread::spawn(move || {
            lower_priority();
            for (i, iters, zero) in [(1usize, 500usize, false), (2, 2000, false), (3, 2000, true), (4, 10000, false)] {
                if i == 4 && big {
                    continue;
                }
                if pts.stop.load(std::sync::atomic::Ordering::Relaxed) {
                    pts.set(i, Slot::Failed);
                    continue;
                }
                let cost: Vec<f64> = if zero { vec![0.0; q.n] } else { q.cost.clone() };
                let lp = crate::simplex::mip_lp::TwoStageLp::new(&q.col_lo, &q.col_up, &cost, &q.rows, &q.row_lo, &q.row_up);
                let xo = Arc::new(Mutex::new(Vec::new()));
                crate::cancel::with_token(Some(pts.stop.clone()), || {
                    let _ = lp.pdlrc_run_iters_hist(iters, None, Some(xo.clone()));
                });
                let x = xo.lock().map(|x| x.clone()).unwrap_or_default();
                // 止めで途中で終わった点は使わない (反復の数が決まらないので)
                if x.len() == q.n && !pts.stop.load(std::sync::atomic::Ordering::Relaxed) {
                    pts.set(i, Slot::Ready(Arc::new(x)));
                } else {
                    pts.set(i, Slot::Failed);
                }
            }
        });
    }
    // `T` (比べ用、再現しない): 門で止めず、主は前処理の終わりの時点の最良を待たずに読む
    let nowait = mode().is_some_and(|m| m.contains('T'));
    let mut gates = Vec::new();
    for t in 0..k {
        let g = crate::ticks::Gate::new(if nowait { f64::INFINITY } else { 0.0 });
        gates.push(g.clone());
        let q = p.clone();
        let best = best.clone();
        let pts = pts.clone();
        let sd = seed.wrapping_add((t as u64 + 1).wrapping_mul(0xA24B_AED4_963E_E407));
        std::thread::spawn(move || {
            lower_priority();
            let _guard = crate::ticks::install_gate(g);
            pt_worker(&q, t, sd, &best, &pts);
        });
    }
    let job = std::rc::Rc::new(PtJob { gates, best, base: crate::ticks::total(), granted: std::cell::Cell::new(0.0), rate: tunable!("ENOMOTO_T_MIP_TPRE_PT_RATE", 0.5, f64), pdlp_stop });
    PT_JOB.with(|j| *j.borrow_mut() = Some(std::rc::Rc::downgrade(&job)));
    job
}

/// 主の前処理の手間まで許す ([`super::pretime::Guard`] を捨てるときに呼ばれる。このスレッドに仕事がなければ何もしない)
pub(crate) fn pt_grant() {
    let Some(job) = PT_JOB.with(|j| j.borrow().as_ref().and_then(|w| w.upgrade())) else { return };
    job.grant_to_main();
}

impl PtJob {
    fn grant_to_main(&self) -> f64 {
        let w = (crate::ticks::total() - self.base).max(0.0) * self.rate;
        let add = w - self.granted.get();
        if add > 0.0 {
            for g in &self.gates {
                g.grant(add);
            }
            self.granted.set(w);
        }
        w
    }

    /// 前処理の終わり: 主の手間まで許し、全部のスレッドが門で止まる (か終わる) のを待って最良を返し、止める。(手間, 最良の目的値, 解, スレッド)
    pub(crate) fn finish(&self, p: &MipProblem) -> (f64, Option<(f64, Vec<f64>, usize)>) {
        let w = self.grant_to_main();
        if !mode().is_some_and(|m| m.contains('T')) {
            for g in &self.gates {
                let _ = g.wait_paused();
            }
        }
        let b = self.best.lock().map(|b| b.clone()).unwrap_or_default();
        for g in &self.gates {
            g.stop();
        }
        self.pdlp_stop.store(true, std::sync::atomic::Ordering::Relaxed);
        PT_JOB.with(|j| *j.borrow_mut() = None);
        let mut out: Option<(f64, Vec<f64>, usize)> = None;
        for (t, e) in b.into_iter().enumerate() {
            if let Some((z, x)) = e {
                if p.is_feasible(&x, 1e-6) && out.as_ref().is_none_or(|o| z < o.0) {
                    out = Some((z, x, t));
                }
            }
        }
        (w, out)
    }
}

/// 手伝いのスレッド `t` の周回 r: 点 ((r + t) mod 5) (0 = 境界の端、1〜4 = PDLP の点。用意されるまで待つ) から FJ を 1 回
/// (種を変えて)、続けて Local-MIP を `ENOMOTO_T_MIP_TPRE_LMIP_STEPS` 手。門 (主の手間) で止められるまで続ける
fn pt_worker(p: &MipProblem, t: usize, seed: u64, best: &std::sync::Mutex<Vec<Option<(f64, Vec<f64>)>>>, pts: &Points) {
    let put = |x: &[f64]| {
        if !p.is_feasible(x, 1e-6) {
            return;
        }
        let z = p.objective(x);
        if let Ok(mut b) = best.lock() {
            if b[t].as_ref().is_none_or(|(bz, _)| z < *bz) {
                b[t] = Some((z, x.to_vec()));
            }
        }
    };
    let steps = tunable!("ENOMOTO_T_MIP_TPRE_LMIP_STEPS", 100_000u64, u64);
    for r in 0..100_000u64 {
        let start = pts.get(((r as usize) + t) % 5);
        let sd = seed.wrapping_add(r.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        if let Some(x) = super::fj::fj2_search_from(p, &p.col_lo, &p.col_up, start.as_deref().map(|v| v.as_slice()), 1e9, sd, None) {
            put(&x);
        }
        if crate::ticks::over() {
            return;
        }
        if let Some(mut lm) = super::localmip::LocalMip::new(p, &p.col_lo, &p.col_up, sd ^ 0x6A09_E667_F3BC_C909, true) {
            if let Some(x0) = start.as_ref() {
                lm.set_start(x0);
            }
            lm.step_cap = steps;
            lm.run(std::time::Instant::now() + std::time::Duration::from_secs(1_000_000), None, &|x| put(x));
        }
        if crate::ticks::over() {
            return;
        }
    }
}

/// このスレッドの OS の優先度を下げる (nice 19、`ENOMOTO_T_MIP_TPRE_PT_NICE`)。主の前処理と並列のプロービングにコアを譲る
/// (取り合うと主の前処理が遅くなった: satellites2-60-fs 10 秒 → 16〜18 秒)。計算の中身は変わらない (門で手間を数える)
fn lower_priority() {
    #[cfg(target_os = "linux")]
    {
        extern "C" {
            fn setpriority(which: i32, who: u32, prio: i32) -> i32;
            fn gettid() -> i32;
        }
        let nice = tunable!("ENOMOTO_T_MIP_TPRE_PT_NICE", 19i32, i32);
        // SAFETY: 自分のスレッドの nice を変えるだけ (PRIO_PROCESS = 0 に tid を渡すと Linux ではそのスレッドだけ)
        unsafe {
            let tid = gettid();
            let _ = setpriority(0, tid as u32, nice);
        }
    }
}
