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

pub(crate) fn run(p: &MipProblem, safety: f64, seed: u64) -> TpreOut {
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
    let (Some(l), Some(u)) = (out.l, out.u) else { return out };
    if !(u > l + 1e-9 * u.abs().max(1.0)) {
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
