//! 箱型制約を直接扱う IP-PMM (PIQP 型) の LP ソルバー:
//!
//!   minimize   c^T x
//!   subject to A x = b,   l <= x <= u   (l, u は ±inf でもよい。l < u を仮定)
//!
//! アルゴリズムは親モジュール ([`super::solve`]) と同じ PIQP (P = 0) の近接内点法
//! (Mehrotra の予測子・修正子、Algorithm 2 の正則化と近接中心の更新) で、変数境界を
//! `G = [-I; I]` の不等式行として持つ代わりに、その行のスラック・乗数を Newton 系から
//! 消去して上段対角に畳み込む (PIQP の箱型制約の扱いと同じ):
//!
//!   [ ρI + Σ   A^T ] [dx]   [ r_x + (境界の項) ]
//!   [   A     -δI  ] [dy] = [ r_y               ],   Σ_jj = 1/W_l + 1/W_u,  W = S/Z + δ.
//!
//! 数学的には `G` 行で境界を表した親モジュールの反復と同じ (同じ式を消去しただけ) だが、
//! KKT 系の次元が `n + p + (境界の数)` から `n + p` に減り、さらに 1 反復の予測子・修正子が
//! 同じ行列を使うので数値分解を 1 回にしている ([`AugKkt`])。
//!
//! 主な用途はクロスオーバー (`simplex::crossover`) の初期解: 単体法と同じ前処理後の標準形
//! (`StdForm`) をそのまま解き、主解 `x`・等式の双対 `y`・被約費用 `c - A^T y` を返す。

use rayon::prelude::*;

use super::kkt::{at_mul, csr_mat_t_vec_into, csr_mat_vec_into, csr_transpose, FaerCsr, IpmKkt};
use crate::params::interior_point::{
    BOX_BLOWUP, BOX_BLOWUP_NEAR, BOX_GONDZIO, BOX_REG0, BOX_REG_MIN, CERT_SCALE_MIN, CERT_TOL, EPS_ABS, EPS_REL, GAP_DIV_GUARD, INIT_DIV_GUARD, INIT_POSITIVE_FLOOR,
    INIT_SHIFT_MULTIPLIER, REG_FLOOR_SLACK, RES_DECREASE_RATIO, SLOW_DECREASE_DIVISOR, STALL_ITERS,
    STALL_PROGRESS_RATIO, TAU,
};
use crate::types::Status;

/// 並列ループの 1 タスクあたりの最小の長さ (小さな問題で rayon の手間が計算を上回らないように)。
const PAR_MIN_LEN: usize = 4096;

/// 決定的な総和の塊の長さ ([`dot`])。
const SUM_CHUNK: usize = 4096;

/// Newton 系の求解ごとの反復改良の回数 (上限)。
const REFINE_STEPS: usize = 3;

/// 箱型 IP-PMM の結果。
pub struct BoxIpmResult {
    /// `Optimal` (許容誤差内で収束)、`Infeasible`/`Unbounded` (Farkas 証明または推定)、
    /// `NotSolved` (反復上限で収束せず)。
    pub status: Status,
    /// 主解 (常に最後の反復点。`NotSolved` でも入っている)。
    pub x: Vec<f64>,
    /// 等式の双対 (LP の符号: `c - A^T y = z_l - z_u`)。
    pub y: Vec<f64>,
    /// 被約費用 `z_l - z_u` (下限側の乗数 − 上限側の乗数)。
    pub rc: Vec<f64>,
    /// Newton 反復の回数。
    pub iters: usize,
    /// 最後の反復点の相対的な主残差・双対残差・ギャップ (各許容値で割った値。1 以下で収束)。
    pub rel_res: (f64, f64, f64),
}

/// 内積 `a · b`。固定長 [`SUM_CHUNK`] の塊ごとの部分和を順に足すので、並列でも加算順が実行ごとに
/// 変わらない (rayon の `sum()` は分割が実行ごとに変わり、最終桁の違いが 50 反復を経てクロスオーバーの
/// 経路と仕上げの時間を大きく揺らしていた: pilot87 で 0.9〜11 秒)。
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.par_chunks(SUM_CHUNK)
        .zip(b.par_chunks(SUM_CHUNK))
        .map(|(x, y)| x.iter().zip(y).map(|(p, q)| p * q).sum::<f64>())
        .collect::<Vec<f64>>()
        .iter()
        .sum()
}

/// `f(k)` (`k < len`) の和を [`dot`] と同じく決定的な順で取る。
fn det_sum(len: usize, f: impl Fn(usize) -> f64 + Sync) -> f64 {
    (0..len.div_ceil(SUM_CHUNK))
        .into_par_iter()
        .map(|c| (c * SUM_CHUNK..((c + 1) * SUM_CHUNK).min(len)).map(&f).sum::<f64>())
        .collect::<Vec<f64>>()
        .iter()
        .sum()
}

fn norm_inf(v: &[f64]) -> f64 {
    v.par_iter().with_min_len(PAR_MIN_LEN).map(|x| x.abs()).reduce(|| 0.0_f64, f64::max)
}

/// `v + alpha dv > 0` を保つ最大ステップの `TAU` 倍 (1 で頭打ち)。
fn fraction_to_boundary(v: &[f64], dv: &[f64]) -> f64 {
    let a = v
        .par_iter()
        .zip(dv.par_iter())
        .with_min_len(PAR_MIN_LEN)
        .map(|(&vi, &dvi)| if dvi < 0.0 { TAU * vi / (-dvi) } else { f64::INFINITY })
        .reduce(|| f64::INFINITY, f64::min);
    a.clamp(0.0, 1.0)
}

/// 境界 1 種類 (下限側または上限側) の内点法の変数。`idx[k]` が対象の列、`bnd[k]` がその境界値。
/// 下限側は `s = x - l`、上限側は `s = u - x` (`G` 行の符号 `sgn` = -1 / +1)。
struct Side {
    /// `G` 行の符号 (下限 -1、上限 +1)。
    sgn: f64,
    idx: Vec<usize>,
    bnd: Vec<f64>,
    s: Vec<f64>,
    z: Vec<f64>,
    nu: Vec<f64>,
    // 作業領域
    r_z: Vec<f64>,
    r_s: Vec<f64>,
    w: Vec<f64>,
    rzp: Vec<f64>,
    ds_aff: Vec<f64>,
    dz_aff: Vec<f64>,
    ds: Vec<f64>,
    dz: Vec<f64>,
    s_new: Vec<f64>,
    z_new: Vec<f64>,
}

impl Side {
    fn new(sgn: f64, idx: Vec<usize>, bnd: Vec<f64>) -> Self {
        let k = idx.len();
        let z = || vec![0.0; k];
        Side {
            sgn,
            idx,
            bnd,
            s: z(),
            z: z(),
            nu: z(),
            r_z: z(),
            r_s: z(),
            w: z(),
            rzp: z(),
            ds_aff: z(),
            dz_aff: z(),
            ds: z(),
            dz: z(),
            s_new: z(),
            z_new: z(),
        }
    }

    fn len(&self) -> usize {
        self.idx.len()
    }

    /// `r_z = -(sgn (x - bnd) + δ(ν - z) + s)`、`W = s/z + δ` (並列)。
    fn update_rz_w(&mut self, x: &[f64], delta: f64) {
        let (sgn, idx, bnd, s, z, nu) = (self.sgn, &self.idx, &self.bnd, &self.s, &self.z, &self.nu);
        self.r_z.par_iter_mut().zip(self.w.par_iter_mut()).enumerate().with_min_len(PAR_MIN_LEN).for_each(|(k, (rz, w))| {
            *rz = -(sgn * (x[idx[k]] - bnd[k]) + delta * (nu[k] - z[k]) + s[k]);
            *w = s[k] / z[k] + delta;
        });
    }

    /// 相補性の右辺 `r_s = -s z - ds_aff dz_aff + σμ` (`corr = false` なら予測子の `-s z`)。
    fn set_rs(&mut self, corr: bool, sigma_mu: f64) {
        let (s, z, dsa, dza) = (&self.s, &self.z, &self.ds_aff, &self.dz_aff);
        self.r_s.par_iter_mut().enumerate().with_min_len(PAR_MIN_LEN).for_each(|(k, r)| {
            *r = if corr { -s[k] * z[k] - dsa[k] * dza[k] + sigma_mu } else { -s[k] * z[k] };
        });
    }

    /// `rz' = r_z - r_s / z` (並列)。
    fn set_rzp(&mut self) {
        let (rz, rs, z) = (&self.r_z, &self.r_s, &self.z);
        self.rzp.par_iter_mut().enumerate().with_min_len(PAR_MIN_LEN).for_each(|(k, o)| *o = rz[k] - rs[k] / z[k]);
    }

    /// 解いた `dx` から `dz = (sgn dx - rz') / W`、`ds = (r_s - s dz) / z` (並列)。
    fn set_dz_ds(&mut self, dx: &[f64]) {
        let (sgn, idx, rzp, w, rs, s, z) = (self.sgn, &self.idx, &self.rzp, &self.w, &self.r_s, &self.s, &self.z);
        self.dz.par_iter_mut().zip(self.ds.par_iter_mut()).enumerate().with_min_len(PAR_MIN_LEN).for_each(|(k, (dzk, dsk))| {
            let d = (sgn * dx[idx[k]] - rzp[k]) / w[k];
            *dzk = d;
            *dsk = (rs[k] - s[k] * d) / z[k];
        });
    }

    /// 試行点 `s_new = s + αp ds`、`z_new = z + αd dz` (並列)。
    fn set_new(&mut self, ap: f64, ad: f64) {
        let (s, ds, z, dz) = (&self.s, &self.ds, &self.z, &self.dz);
        self.s_new.par_iter_mut().zip(self.z_new.par_iter_mut()).enumerate().with_min_len(PAR_MIN_LEN).for_each(|(k, (sn, zn))| {
            *sn = s[k] + ap * ds[k];
            *zn = z[k] + ad * dz[k];
        });
    }

    /// 主残差 `G x - h + s` の成分 (`sgn x_j - sgn bnd + s`)。
    #[inline]
    fn primal_res(&self, k: usize, x: &[f64], s: &[f64]) -> f64 {
        self.sgn * (x[self.idx[k]] - self.bnd[k]) + s[k]
    }
}

/// [`solve_box_lp`] の停止判定・近接中心更新に使う残差の無限大ノルム群。
#[derive(Clone, Copy)]
struct Res {
    primal: f64,
    dual: f64,
}

/// 現在の点の主残差・双対残差の無限大ノルム (`ax`, `aty` は計算済み)。`pos_lo[j]`/`pos_up[j]` は列 `j` の
/// 下限側・上限側の番号 (無ければ `usize::MAX`)。双対残差 `c + A^T y - z_l + z_u` を `dual_buf` に書く。
#[allow(clippy::too_many_arguments)]
fn residuals(ax: &[f64], b: &[f64], x: &[f64], lo: &Side, up: &Side, s_lo: &[f64], s_up: &[f64], c: &[f64], aty: &[f64], z_lo: &[f64], z_up: &[f64], pos: &Pos, dual_buf: &mut [f64]) -> Res {
    let mut p = ax.par_iter().zip(b.par_iter()).map(|(a, b)| (a - b).abs()).reduce(|| 0.0, f64::max);
    for (side, sv) in [(lo, s_lo), (up, s_up)] {
        let m = (0..side.len()).into_par_iter().with_min_len(PAR_MIN_LEN).map(|k| side.primal_res(k, x, sv).abs()).reduce(|| 0.0, f64::max);
        p = p.max(m);
    }
    dual_buf.par_iter_mut().enumerate().with_min_len(PAR_MIN_LEN).for_each(|(j, d)| {
        let mut v = c[j] + aty[j];
        let kl = pos.lo[j];
        if kl != usize::MAX {
            v -= z_lo[kl];
        }
        let ku = pos.up[j];
        if ku != usize::MAX {
            v += z_up[ku];
        }
        *d = v;
    });
    Res { primal: p, dual: norm_inf(dual_buf) }
}

/// 列 `j` → 下限側・上限側の番号 (無ければ `usize::MAX`)。列ごとの処理を並列に書くために使う。
struct Pos {
    lo: Vec<usize>,
    up: Vec<usize>,
}

/// `min c^T x, A x = b, l <= x <= u` を IP-PMM で解く (`l[j] < u[j]` を仮定)。
/// `max_iters` は Newton 反復の上限。
pub fn solve_box_lp(a: &FaerCsr, b: &[f64], c: &[f64], l: &[f64], u: &[f64], max_iters: usize) -> BoxIpmResult {
    solve_box_lp_warm(a, b, c, l, u, max_iters, None)
}

/// 内点法のウォームスタートに使う近似解 (PDLP などの一次法の解、二段解法の段階 A の双対など。
/// LP の符号: 被約費用は `c - A^T y`) と、既知の情報。
#[derive(Default)]
pub struct WarmStart<'a> {
    /// 主の近似解 (近接中心 `ξ` にする)。
    pub x: Option<&'a [f64]>,
    /// 双対の近似解 (近接中心 `λ`・`ν` にする)。
    pub y: Option<&'a [f64]>,
    /// 真なら初期点もこの近似解から作る (境界からの距離・乗数を `theta` 以上に持ち上げる。`x`・`y` の
    /// 両方があるときだけ)。偽なら近接中心だけを近似解にして、初期点は通常どおり作る。
    pub point: bool,
    /// 初期点のスラック・乗数の下限 (大きさをそろえた後の単位)。
    pub theta: f64,
    /// 初期の正則化 `ρ`・`δ` (`None` なら通常の `BOX_REG0`)。
    pub reg0: Option<f64>,
    /// 真なら問題が双対実行可能だと分かっている (二段解法の段階 A で `z^1 = 0`、または費用 0 の
    /// 実行可能性問題)。主の非有界は起こらないので双対実行不能の判定は行わず、結論は「最適」か
    /// 「双対の発散 (主実行不能)」だけになる。乗数の大きさが初期の [`DIVERGE_FACTOR`] 倍を超えたら、
    /// 正則化・停滞の条件を待たずに Farkas の証明を確かめる。
    pub dual_feasible_known: bool,
    /// 元の問題 (この関数に渡した `c`・`x` の単位) の最適値の下界を返す関数 (同時実行の二段解法が共有する。
    /// 未設定なら `-inf`)。主目的値との差の相対値が `bound_gap` 以下で、主残差が許容値の `bound_pres` 倍以下
    /// なら、収束を待たずに `Optimal` として返す (クロスオーバーに渡す)。
    pub lower_bound: Option<&'a (dyn Fn() -> f64 + Sync)>,
    pub bound_gap: f64,
    pub bound_pres: f64,
}

/// 双対実行可能と分かっているとき、乗数の大きさがこの倍数を超えたら発散とみなして証明を確かめる。
const DIVERGE_FACTOR: f64 = 1e6;

/// [`solve_box_lp`] に近似解 `warm` からのウォームスタートを加えたもの。
pub fn solve_box_lp_warm(a: &FaerCsr, b: &[f64], c: &[f64], l: &[f64], u: &[f64], max_iters: usize, warm: Option<&WarmStart>) -> BoxIpmResult {
    // 主変数と費用の大きさをそろえる: `x = β x'`、`c = γ c'` (β = max(1, ‖b‖∞)、γ = ‖c‖∞)。
    // 単体法向けの前処理 (Ruiz) は行列の要素をそろえるが、右辺・境界 (伝播で付いた暗黙の
    // 境界を含む) と費用の大きさの差は残り (pilot4 で ‖b‖ ≈ 3e4、‖c‖ ≈ 0.05)、PIQP の初期点の
    // ギャップが 1e11 を超えて収束が遅れる。
    let beta = b.iter().fold(1.0f64, |m, v| m.max(v.abs()));
    let cmax = c.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let gamma = if cmax > 0.0 { cmax } else { 1.0 };
    let bs: Vec<f64> = b.iter().map(|v| v / beta).collect();
    let cs: Vec<f64> = c.iter().map(|v| v / gamma).collect();
    let ls: Vec<f64> = l.iter().map(|v| v / beta).collect();
    let us: Vec<f64> = u.iter().map(|v| v / beta).collect();
    // 近似解も同じ大きさにそろえる (`x' = x/β`、`y' = y/γ`)。
    let warm_s = warm.map(|w| WarmScaled {
        x: w.x.map(|x| x.iter().map(|v| v / beta).collect()),
        y: w.y.map(|y| y.iter().map(|v| v / gamma).collect()),
        point: w.point,
        theta: w.theta,
        reg0: w.reg0,
        dual_feasible_known: w.dual_feasible_known,
        lower_bound: w.lower_bound,
        bound_gap: w.bound_gap,
        bound_pres: w.bound_pres,
        obj_scale: beta * gamma,
    });
    let mut r = solve_box_lp_scaled(a, &bs, &cs, &ls, &us, max_iters, warm_s.as_ref());
    for v in r.x.iter_mut() {
        *v *= beta;
    }
    for v in r.y.iter_mut().chain(r.rc.iter_mut()) {
        *v *= gamma;
    }
    r
}

/// [`solve_box_lp`] の本体 (大きさをそろえた後の問題を解く)。
/// [`WarmStart`] を大きさをそろえた後の単位にしたもの。
struct WarmScaled<'a> {
    x: Option<Vec<f64>>,
    y: Option<Vec<f64>>,
    point: bool,
    theta: f64,
    reg0: Option<f64>,
    dual_feasible_known: bool,
    lower_bound: Option<&'a (dyn Fn() -> f64 + Sync)>,
    bound_gap: f64,
    bound_pres: f64,
    /// 目的値の縮尺 `β γ` (内部の目的値 = 元の目的値 / (β γ))。
    obj_scale: f64,
}

fn solve_box_lp_scaled(a: &FaerCsr, b: &[f64], c: &[f64], l: &[f64], u: &[f64], max_iters: usize, warm: Option<&WarmScaled>) -> BoxIpmResult {
    let n = a.ncols();
    let p = a.nrows();
    let debug = env_str!("ENOMOTO_DEBUG_IPM").is_some();
    let lo_idx: Vec<usize> = (0..n).filter(|&j| l[j].is_finite()).collect();
    let up_idx: Vec<usize> = (0..n).filter(|&j| u[j].is_finite()).collect();
    let mut lo = Side::new(-1.0, lo_idx.clone(), lo_idx.iter().map(|&j| l[j]).collect());
    let mut up = Side::new(1.0, up_idx.clone(), up_idx.iter().map(|&j| u[j]).collect());
    let n_bnd = lo.len() + up.len();
    let mut pos = Pos { lo: vec![usize::MAX; n], up: vec![usize::MAX; n] };
    for (k, &j) in lo.idx.iter().enumerate() {
        pos.lo[j] = k;
    }
    for (k, &j) in up.idx.iter().enumerate() {
        pos.up[j] = k;
    }
    let pos = pos;

    let t_kkt = std::time::Instant::now();
    // 診断用 (ENOMOTO_DEBUG_IPM): 数値分解・Newton 系の求解 (反復改良を含む) の累計時間。
    let mut prof = (0.0f64, 0.0f64);
    // `Aᵀ` を行圧縮で持ち、`Aᵀ y` を行ごとに並列に計算する (`A` の行圧縮のまま足し込むと逐次: osa-60 は列が 24 万本で
    // `Aᵀ y` が反復の時間の多くを占めた)。
    let at = csr_transpose(a);
    let Some(mut kkt) = IpmKkt::new(a) else {
        // 因子が大きすぎる (`MAX_FACTOR_NNZ`): 内点法を諦める。
        if debug {
            eprintln!("IPM factor too large; giving up (setup {:.2}s)", t_kkt.elapsed().as_secs_f64());
        }
        crate::phase_timing::mark("ipm_factor_too_large");
        return BoxIpmResult {
            status: Status::NotSolved,
            x: vec![0.0; n],
            y: vec![0.0; p],
            rc: vec![0.0; n],
            iters: 0,
            rel_res: (f64::INFINITY, f64::INFINITY, f64::INFINITY),
        };
    };
    if debug {
        eprintln!(
            "IPM kkt={} nnz(A)={} nnz(L)={} setup={:.2}s",
            if kkt.is_normal() { "normal" } else { "augmented" },
            a.compute_nnz(),
            kkt.factor_nnz(),
            t_kkt.elapsed().as_secs_f64()
        );
    }
    let dim = n + p;
    let mut top = vec![0.0; n];
    let mut mid = vec![0.0; p];
    let mut rhs = vec![0.0; dim];
    let mut sol_aff = vec![0.0; dim];

    let reg0 = warm.and_then(|w| w.reg0);
    let dual_feasible_known = warm.is_some_and(|w| w.dual_feasible_known);
    let reg_init = tunable!("ENOMOTO_T_IPM_REG0", BOX_REG0, f64);
    let mut rho = reg0.unwrap_or(reg_init);
    let mut delta = reg0.unwrap_or(reg_init);
    let mut rho_min = tunable!("ENOMOTO_T_IPM_RHO_MIN", BOX_REG_MIN, f64);
    // 停止の許容誤差 (絶対・相対とも。既定は PIQP の 1e-8)。
    let eps_abs = tunable!("ENOMOTO_T_IPM_EPS", EPS_ABS, f64);
    let eps_rel = tunable!("ENOMOTO_T_IPM_EPS", EPS_REL, f64);
    // Gondzio の多重中心性補正子の最大回数 (0 で Mehrotra の予測子・修正子だけ)。
    let gondzio_max = tunable!("ENOMOTO_T_IPM_GONDZIO", BOX_GONDZIO, usize);
    let gondzio_small_step = tunable!("ENOMOTO_T_IPM_GONDZIO_SMALL_STEP", 1.0f64, f64);
    let gondzio_auto = tunable!("ENOMOTO_T_IPM_GONDZIO_AUTO", 0.0f64, f64);
    let gondzio_auto_max = tunable!("ENOMOTO_T_IPM_GONDZIO_AUTO_MAX", 2usize, usize);
    let mut nan_recover_left = tunable!("ENOMOTO_T_IPM_NAN_RECOVER", 0usize, usize);
    let switch_aug_k = tunable!("ENOMOTO_T_IPM_SWITCH_AUG", 0usize, usize);
    let mut sw_best = f64::INFINITY;
    let mut sw_count = 0usize;
    // 真なら近接中心 (ξ, λ, ν) を残差の減り方によらず毎反復更新する。
    let prox_always = tunable!("ENOMOTO_T_IPM_PROX_ALWAYS", 0u8, u8) != 0;
    let mut delta_min = tunable!("ENOMOTO_T_IPM_DELTA_MIN", BOX_REG_MIN, f64);
    // 正則化 ρ・δ の下げ方 (反復の終わりの説明参照)。0: PIQP、1: IP-PMM の著者の実装 (既定、第 8 回の比較)、
    // 2: ρ = δ = κ μ、3: PIQP の規則を κ μ で頭打ち。
    let reg_mode = tunable!("ENOMOTO_T_IPM_REG_MODE", 1u8, u8);
    let reg_kappa = tunable!("ENOMOTO_T_IPM_REG_KAPPA", 1.0f64, f64);
    let reg_gap_floor = tunable!("ENOMOTO_T_IPM_REG_GAP_FLOOR", 0.0f64, f64);

    // ---- 初期化: W = 1 + δ の正則化 KKT 系を 1 回解く ----
    let w0 = 1.0 + delta;
    top.fill(rho);
    for &j in lo.idx.iter().chain(up.idx.iter()) {
        top[j] += 1.0 / w0;
    }
    kkt.factor(&top, delta, &mut mid);
    for j in 0..n {
        rhs[j] = -c[j];
    }
    // G^T W^{-1} h: 下限 (-1)(-l)/W、上限 (+1)(u)/W
    for k in 0..lo.len() {
        rhs[lo.idx[k]] += lo.bnd[k] / w0;
    }
    for k in 0..up.len() {
        rhs[up.idx[k]] += up.bnd[k] / w0;
    }
    rhs[n..].copy_from_slice(b);
    let mut refine_work: Vec<f64> = Vec::new();
    kkt.solve_refined(a, &at, &top, delta, &mut rhs, REFINE_STEPS, &mut refine_work);
    let mut xi = rhs[..n].to_vec();
    let mut y = rhs[n..].to_vec();
    let mut x = xi.clone();

    if n_bnd == 0 {
        let mut aty = vec![0.0; n];
        at_mul(a, &at, &y, &mut aty);
        let rc: Vec<f64> = (0..n).map(|j| c[j] + aty[j]).collect();
        let st = if norm_inf(&rc) > 1e-4 { Status::Unbounded } else { Status::Optimal };
        return BoxIpmResult { status: st, x, y: y.iter().map(|v| -v).collect(), rc, iters: 0, rel_res: (0.0, 0.0, 0.0) };
    }

    // ν̃ = (G x - h)/W、s̃ = -ν̃。
    let mut nu_t: Vec<f64> = Vec::with_capacity(n_bnd);
    for side in [&lo, &up] {
        for k in 0..side.len() {
            nu_t.push(side.sgn * (x[side.idx[k]] - side.bnd[k]) / w0);
        }
    }
    let s_t: Vec<f64> = nu_t.iter().map(|v| -v).collect();
    let ds_t = (0.5 * -s_t.iter().cloned().fold(f64::INFINITY, f64::min) * INIT_SHIFT_MULTIPLIER).max(0.0);
    let dnu_t = (0.5 * -nu_t.iter().cloned().fold(f64::INFINITY, f64::min) * INIT_SHIFT_MULTIPLIER).max(0.0);
    let s_shift: Vec<f64> = s_t.iter().map(|v| v + ds_t).collect();
    let nu_shift: Vec<f64> = nu_t.iter().map(|v| v + dnu_t).collect();
    let cross = dot(&s_shift, &nu_shift);
    let sum_nu: f64 = nu_shift.iter().sum();
    let sum_s: f64 = s_shift.iter().sum();
    let d_s0 = ds_t + 0.5 * cross / sum_nu.max(INIT_DIV_GUARD);
    let d_nu0 = dnu_t + 0.5 * cross / sum_s.max(INIT_DIV_GUARD);
    {
        let nl = lo.len();
        for k in 0..nl {
            lo.s[k] = (s_t[k] + d_s0).max(INIT_POSITIVE_FLOOR);
            lo.z[k] = (nu_t[k] + d_nu0).max(INIT_POSITIVE_FLOOR);
        }
        for k in 0..up.len() {
            up.s[k] = (s_t[nl + k] + d_s0).max(INIT_POSITIVE_FLOOR);
            up.z[k] = (nu_t[nl + k] + d_nu0).max(INIT_POSITIVE_FLOOR);
        }
        lo.nu.copy_from_slice(&lo.z);
        up.nu.copy_from_slice(&up.z);
    }
    let mut lambda = y.clone();
    // ---- ウォームスタート: 近似解を近接中心に (指定があれば初期点にも) する ----
    // 内部の双対は LP の符号と逆 (`c + A^T y_int = z_l - z_u`)。
    if let Some(w) = warm {
        if let Some(wx) = &w.x {
            xi.copy_from_slice(wx);
        }
        if let Some(wy) = &w.y {
            let y_int: Vec<f64> = wy.iter().map(|v| -v).collect();
            let mut aty_w = vec![0.0; n];
            at_mul(a, &at, &y_int, &mut aty_w);
            // 被約費用 rc = c + A^T y_int を下限側 (正) と上限側 (負) に分ける。
            let rc: Vec<f64> = (0..n).map(|j| c[j] + aty_w[j]).collect();
            lambda.copy_from_slice(&y_int);
            for side in [&mut lo, &mut up] {
                for k in 0..side.len() {
                    let j = side.idx[k];
                    // 下限側 (sgn = -1) の乗数は rc⁺、上限側 (sgn = +1) は rc⁻。
                    side.nu[k] = (-side.sgn * rc[j]).max(0.0);
                }
            }
            if let (true, Some(wx)) = (w.point, &w.x) {
                x.copy_from_slice(wx);
                y.copy_from_slice(&y_int);
                for side in [&mut lo, &mut up] {
                    for k in 0..side.len() {
                        let j = side.idx[k];
                        side.s[k] = (-side.sgn * (x[j] - side.bnd[k])).max(w.theta);
                        side.z[k] = side.nu[k].max(w.theta);
                    }
                    side.nu.copy_from_slice(&side.z);
                }
            }
        }
    }
    // 乗数の大きさの基準 (発散の判定用)。
    let mult_scale0 = norm_inf(&y).max(norm_inf(&lo.z)).max(norm_inf(&up.z)).max(1.0);

    // 作業領域
    let mut ax = vec![0.0; p];
    let mut aty = vec![0.0; n];
    let mut dual_res = vec![0.0; n];
    let mut r_x = vec![0.0; n];
    let mut r_y = vec![0.0; p];
    let mut x_new = vec![0.0; n];
    let mut y_new = vec![0.0; p];
    let mut ax_new = vec![0.0; p];
    let mut aty_new = vec![0.0; n];
    let mut dual_new = vec![0.0; n];
    // これまでで最良の反復点 (相対残差の最大が最小のもの): 数値的に破綻したときに返す。
    let mut best: Option<(f64, Vec<f64>, Vec<f64>, Vec<f64>, (f64, f64, f64))> = None;

    let mut prev_p = f64::INFINITY;
    let mut prev_d = f64::INFINITY;
    let mut stall = 0usize;
    let norm_b = norm_inf(b);
    let norm_c = norm_inf(c);
    let norm_h = lo.bnd.iter().chain(up.bnd.iter()).fold(0.0f64, |m, v| m.max(v.abs()));
    if debug {
        let big = lo.bnd.iter().chain(up.bnd.iter()).filter(|v| v.abs() > 1e6).count();
        eprintln!("IPM n={n} p={p} n_lo={} n_up={} |h|max={norm_h:.3e} (#|bnd|>1e6: {big}) |b|={norm_b:.3e} |c|={norm_c:.3e}", lo.len(), up.len());
    }
    let mut rel = (f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut status = Status::NotSolved;
    let mut iters = 0usize;
    let mut cached: Option<Res> = None;
    let noimprove_k = tunable!("ENOMOTO_T_IPM_NOIMPROVE", 0usize, usize);
    let blowup = tunable!("ENOMOTO_T_IPM_BLOWUP", BOX_BLOWUP, f64);
    let mut noimprove = 0usize;
    // 停止基準の後の高精度化の目標 (停止基準の倍率、既定 1e-3 は第 43〜46 回の比較で決めた。0 で行わない)。
    let push_f = tunable!("ENOMOTO_T_IPM_PUSH", 1e-3f64, f64);
    let solve_acc = tunable!("ENOMOTO_T_IPM_SOLVE_ACC", 0.0f64, f64);
    let solve_acc_bump = tunable!("ENOMOTO_T_IPM_SOLVE_ACC_BUMP", 100.0f64, f64);
    let solve_acc_min = tunable!("ENOMOTO_T_IPM_SOLVE_ACC_MIN", 1e-11f64, f64);
    let mut acc_retries = 0usize;
    let aug_on_inacc = tunable!("ENOMOTO_T_IPM_AUG_ON_INACCURATE", 1e-6f64, f64);
    let aug_on_inacc_any = tunable!("ENOMOTO_T_IPM_AUG_ON_INACCURATE_ANY", 0u8, u8) != 0;
    let solve_acc2 = tunable!("ENOMOTO_T_IPM_SOLVE_ACC2", 1e-6f64, f64);
    let solve_acc2_reg = tunable!("ENOMOTO_T_IPM_SOLVE_ACC2_REG", 1e-10f64, f64);
    let mut prev_solve_rel = 0.0f64;
    let mut prev_worst_acc = f64::INFINITY;
    let mut temp_reg = 0.0f64;
    let solve_acc2_k = tunable!("ENOMOTO_T_IPM_SOLVE_ACC2_K", 3usize, usize);
    let mut stuck_count = 0usize;
    let stall_bump = tunable!("ENOMOTO_T_IPM_STALL_BUMP", 0.0f64, f64);
    let stall_bump_k = tunable!("ENOMOTO_T_IPM_STALL_BUMP_K", 2usize, usize);
    let stall_floor = tunable!("ENOMOTO_T_IPM_STALL_FLOOR", 0u8, u8) != 0;
    let stall_jump = tunable!("ENOMOTO_T_IPM_STALL_JUMP", 0.0f64, f64);
    let mut min_primal = f64::INFINITY;
    let mut jumped = false;
    let stall_jump_k = tunable!("ENOMOTO_T_IPM_STALL_JUMP_K", 1usize, usize);
    let mut jump_count = 0usize;
    let (rho_min0, delta_min0) = (rho_min, delta_min);
    let mut bump_best = f64::INFINITY;
    let mut bump_count = 0usize;
    let push_max = tunable!("ENOMOTO_T_IPM_PUSH_ITERS", 15usize, usize);
    // 残差の最悪値が最良の半分を下回らない反復がこの回数続いたら高精度化をやめる。
    let push_stall_max = tunable!("ENOMOTO_T_IPM_PUSH_STALL", 1usize, usize);
    let mut pushing = false;
    let mut push_it = 0usize;
    let mut push_stall = 0usize;
    let mut push_prev = f64::NAN;

    for it in 0..max_iters {
        iters = it;
        if crate::cancel::is_cancelled() {
            break; // 同時実行の相手が先に結論を出した (status は NotSolved のまま)
        }
        // 前の反復の終わりに同じ点で計算した残差 (`ax`・`aty`・`dual_res` も) があれば使い回す。
        let res = match cached.take() {
            Some(r) => r,
            None => {
                csr_mat_vec_into(a, &x, &mut ax);
                at_mul(a, &at, &y, &mut aty);
                residuals(&ax, b, &x, &lo, &up, &lo.s, &up.s, c, &aty, &lo.z, &up.z, &pos, &mut dual_res)
            }
        };
        let cx = dot(c, &x);
        let by = dot(b, &y);
        let hz: f64 = -dot(&lo.bnd, &lo.z) + dot(&up.bnd, &up.z);
        let gap = (cx + by + hz).abs();
        let gx = x.par_iter().enumerate().with_min_len(PAR_MIN_LEN).filter(|&(j, _)| pos.lo[j] != usize::MAX || pos.up[j] != usize::MAX).map(|(_, v)| v.abs()).reduce(|| 0.0, f64::max);
        let s_inf = norm_inf(&lo.s).max(norm_inf(&up.s));
        let z_inf = norm_inf(&lo.z).max(norm_inf(&up.z));
        let bnd_p = eps_abs + eps_rel * norm_inf(&ax).max(norm_b).max(gx).max(norm_h).max(s_inf);
        let bnd_d = eps_abs + eps_rel * norm_inf(&aty).max(z_inf).max(norm_c);
        let bnd_g = eps_abs + eps_rel * cx.abs().max(by.abs()).max(hz.abs());
        rel = (res.primal / bnd_p, res.dual / bnd_d, gap / bnd_g);
        if debug {
            eprintln!(
                "IPM t={:.2}s it={it:3} pobj={cx:.10e} pres={:.2e} dres={:.2e} gap={:.2e} rho={rho:.1e} delta={delta:.1e}",
                t_kkt.elapsed().as_secs_f64(), res.primal, res.dual, gap
            );
        }
        if res.primal <= bnd_p && res.dual <= bnd_d && gap <= bnd_g {
            // 高精度化 (`ENOMOTO_T_IPM_PUSH=f`): 停止基準を満たした後も、相対残差の最悪値が f (停止基準の倍率) に
            // 下がるか、伸びなくなる・数値が破綻する・`ENOMOTO_T_IPM_PUSH_ITERS` 反復に達するまで続け、最良の点を返す。
            let worst_now = rel.0.max(rel.1).max(rel.2);
            if push_f <= 0.0 || worst_now <= push_f {
                status = Status::Optimal;
                break;
            }
            if !pushing {
                pushing = true;
                // 停滞で引き上げた正則化の下限は、高精度化の段では元に戻す (下限が高いままでは精度が出ない)。
                if stall_floor {
                    rho_min = rho_min0;
                    delta_min = delta_min0;
                }
                if debug {
                    eprintln!("IPM it={it} reached the tolerance; pushing for higher accuracy (target {push_f:.1e})");
                }
            }
        }
        if pushing {
            let worst_now = rel.0.max(rel.1).max(rel.2);
            push_it += 1;
            let bw = best.as_ref().map_or(f64::INFINITY, |b| b.0);
            // 停止基準に達した反復そのもの (push_it == 1) と、正則化を強めて同じ点で解き直した反復
            // (点が動かず残差がまったく同じ) は数えない。
            if push_it == 1 || worst_now == push_prev {
            } else if worst_now.is_finite() && worst_now < 0.5 * bw {
                push_stall = 0;
            } else {
                push_stall += 1;
            }
            push_prev = worst_now;
            if push_it > push_max || push_stall >= push_stall_max || !worst_now.is_finite() {
                if debug {
                    eprintln!("IPM it={it} stop pushing (iters {push_it}, stall {push_stall}, worst {worst_now:.2e}, best {bw:.2e})");
                }
                if worst_now.is_finite() && worst_now < bw {
                    status = Status::Optimal;
                }
                break;
            }
        }
        // 同時実行の二段解法の下界との差 (真の双対ギャップの上界) で、クロスオーバーに渡す。
        if let Some(w) = warm {
            if let Some(lbf) = w.lower_bound {
                let lb = lbf() / w.obj_scale;
                if lb.is_finite() {
                    let ext = (cx - lb) / cx.abs().max(lb.abs()).max(1e-300);
                    if debug {
                        eprintln!("IPM it={it} external lower bound={lb:.10e} rel_gap={ext:.2e}");
                    }
                    if ext <= w.bound_gap && rel.0 <= w.bound_pres {
                        if debug {
                            eprintln!("IPM it={it} stopping: gap to the shared lower bound {ext:.2e} <= {:.1e}", w.bound_gap);
                        }
                        status = Status::Optimal;
                        break;
                    }
                }
            }
        }
        let worst = rel.0.max(rel.1).max(rel.2);
        if !worst.is_finite() || !cx.is_finite() {
            // 数値的に破綻した (分解の失敗など): 最良の反復点に戻して打ち切る。
            break;
        }
        // 発散の打ち切り (`BOX_BLOWUP`): 正則化が下限に達した後、ほぼ収束した最良点からいまの点が大きく離れたら
        // 最良点に戻して打ち切る (呼び出し側は最良点の残差で受理するか決める)。
        if blowup > 0.0 && rho <= rho_min * REG_FLOOR_SLACK && delta <= delta_min * REG_FLOOR_SLACK {
            if let Some(b) = best.as_ref() {
                if b.0 <= BOX_BLOWUP_NEAR && worst > blowup * b.0 {
                    if debug {
                        eprintln!("IPM it={it} residual blew up ({worst:.3e} vs best {:.3e}); returning the best iterate", b.0);
                    }
                    crate::phase_timing::mark("ipm_blowup");
                    break;
                }
            }
        }
        // 停滞の打ち切り (試験用 `ENOMOTO_T_IPM_NOIMPROVE=K`): 正則化が下限に達した後、最良点の `worst` が
        // K 反復続けて 10% 以上改善しなければ打ち切る (greenbea は 45 反復目から同じ点を往復する)。
        if noimprove_k > 0 && rho <= rho_min * REG_FLOOR_SLACK && delta <= delta_min * REG_FLOOR_SLACK {
            if best.as_ref().map_or(true, |b| worst < 0.9 * b.0) {
                noimprove = 0;
            } else {
                noimprove += 1;
                if noimprove >= noimprove_k {
                    if debug {
                        eprintln!("IPM it={it} no improvement for {noimprove_k} iterations; stopping");
                    }
                    break;
                }
            }
        }
        if best.as_ref().map_or(true, |b| worst < b.0) {
            let mut rc = vec![0.0; n];
            for k in 0..lo.len() {
                rc[lo.idx[k]] += lo.z[k];
            }
            for k in 0..up.len() {
                rc[up.idx[k]] -= up.z[k];
            }
            best = Some((worst, x.clone(), y.clone(), rc, rel));
        }

        // 試験用 (`ENOMOTO_T_IPM_SWITCH_AUG=K`): 正規方程式で、相対残差の最悪値が K 反復続けて最良値の 0.9 倍を
        // 下回らなければ拡大系に切り替える (稠密な列を分離した正規方程式は、残りの疎な部分がほぼ特異だと
        // 桁落ちで方向の精度が出ず停滞する: ns1688926)。
        if switch_aug_k > 0 && kkt.is_normal() {
            if worst < 0.9 * sw_best {
                sw_best = worst;
                sw_count = 0;
            } else {
                sw_count += 1;
                if sw_count >= switch_aug_k {
                    if debug {
                        eprintln!("IPM it={it} normal equations stalled for {switch_aug_k} iterations; switching to the augmented system");
                    }
                    kkt = IpmKkt::augmented(a);
                    crate::phase_timing::mark("ipm_switch_augmented");
                }
            }
        }
        let at_floor = rho <= rho_min * REG_FLOOR_SLACK && delta <= delta_min * REG_FLOOR_SLACK;
        if res.primal >= prev_p * STALL_PROGRESS_RATIO && res.dual >= prev_d * STALL_PROGRESS_RATIO && at_floor {
            stall += 1;
        } else {
            stall = 0;
        }
        // Farkas 証明は正則化が下限に達し、しかも残差が改善しなくなってから試す (親モジュールは
        // 下限に達しただけで試すが、収束途中の大きな乗数で誤って成立することがある: pilot4)。
        // 双対実行可能と分かっていれば、乗数が初期の DIVERGE_FACTOR 倍を超えた (発散した) ときも主実行不能の
        // 証明を確かめる。双対実行不能 (非有界) の判定はしない (起こらない)。
        let diverged = dual_feasible_known
            && norm_inf(&y).max(norm_inf(&lo.z)).max(norm_inf(&up.z)) > DIVERGE_FACTOR * mult_scale0;
        if (at_floor && stall >= 2) || diverged {
            if primal_infeasibility_certificate(a, b, &y, &lo, &up, &mut aty_new) {
                status = Status::Infeasible;
                break;
            }
            if !dual_feasible_known && dual_infeasibility_certificate(a, c, &x, &lo, &up, &mut ax_new) {
                status = Status::Unbounded;
                break;
            }
        }
        prev_p = res.primal;
        prev_d = res.dual;
        if stall >= STALL_ITERS {
            break;
        }
        // 試験用 (`ENOMOTO_T_IPM_STALL_BUMP=v`): 正則化が下限にある間に、相対残差の最悪値が最良の 0.5 倍を下回らない反復が
        // `ENOMOTO_T_IPM_STALL_BUMP_K` 回続いたら (終盤に Newton 系の精度が足りず空回りしている: ken-18 は 28 反復目から
        // 主残差が 3e-6 で 35 反復止まり、方向が非有限になって正則化が上がるまで続いた)、正則化を v に上げて解き直す。
        // `ENOMOTO_T_IPM_STALL_JUMP=f` (f > 0): 停滞の代わりに、下限にある間に主残差がそれまでの最小の f 倍以上に跳ね上がった
        // 反復で一度だけ上げる (ken-18: 1e-10 → 3e-6。序盤のゆっくりした収束 (dfl001) では上げない)。
        if stall_jump > 0.0 {
            // 跳ね上がりが `ENOMOTO_T_IPM_STALL_JUMP_K` 反復続いたときだけ (一時的な跳ね上がりは次の反復で戻る: pilotnov)。
            if at_floor && res.primal > stall_jump * min_primal {
                jump_count += 1;
            } else {
                jump_count = 0;
            }
            if stall_bump > 0.0 && jump_count >= stall_jump_k && !jumped {
                rho = rho.max(stall_bump);
                delta = delta.max(stall_bump);
                jumped = true;
                crate::phase_timing::mark("ipm_stall_bump");
                if debug {
                    eprintln!("IPM it={it} primal residual jumped ({:.2e} vs min {min_primal:.2e}); raising the regularization to {stall_bump:.1e}", res.primal);
                }
            }
            min_primal = min_primal.min(res.primal);
        } else if stall_bump > 0.0 && at_floor {
            if worst < 0.5 * bump_best {
                bump_best = worst;
                bump_count = 0;
            } else {
                bump_count += 1;
                if bump_count >= stall_bump_k {
                    // `ENOMOTO_T_IPM_STALL_FLOOR=1`: 一度だけ上げる代わりに、下限そのものを 100 倍 (v まで) に上げる。
                    if stall_floor {
                        rho_min = (rho_min * 100.0).min(stall_bump);
                        delta_min = (delta_min * 100.0).min(stall_bump);
                        rho = rho.max(rho_min);
                        delta = delta.max(delta_min);
                    } else {
                        rho = rho.max(stall_bump);
                        delta = delta.max(stall_bump);
                    }
                    bump_count = 0;
                    bump_best = worst;
                    crate::phase_timing::mark("ipm_stall_bump");
                    if debug {
                        eprintln!("IPM it={it} stalled at the regularization floor; raising it to {stall_bump:.1e}");
                    }
                }
            }
        } else if worst < bump_best {
            bump_best = worst;
        }

        // 既定 t = 1e-6、K = 3 (第 55 回、0 で使わない) (`ENOMOTO_T_IPM_SOLVE_ACC2=t`、続く回数 `ENOMOTO_T_IPM_SOLVE_ACC2_K`): 前の反復の予測子の Newton 系の相対残差が t を超え (分解の精度が足りない)、
        // しかも相対残差の最悪値が 0.95 倍未満に減らなかった (実際に進まなかった) ときだけ、この反復は ρ・δ を一時的に
        // `temp_reg` (初回 `ENOMOTO_T_IPM_SOLVE_ACC2_REG`、続けて失敗したら 100 倍ずつ) に強めて解き、反復の終わりに
        // 元の値に戻す。不正確でも進んでいる反復 (pilotnov) は乱さない。
        let mut temp_saved: Option<(f64, f64)> = None;
        if solve_acc2 > 0.0 {
            let stuck_now = prev_solve_rel > solve_acc2 && worst >= 0.95 * prev_worst_acc;
            stuck_count = if stuck_now { stuck_count + 1 } else { 0 };
            if stuck_count >= solve_acc2_k {
                temp_reg = if temp_reg > 0.0 { (temp_reg * 100.0).min(1e-4) } else { solve_acc2_reg };
                temp_saved = Some((rho, delta));
                rho = rho.max(temp_reg);
                delta = delta.max(temp_reg);
                crate::phase_timing::mark("ipm_solve_inaccurate");
                if debug {
                    eprintln!("IPM it={it} inaccurate solve ({prev_solve_rel:.2e}) without progress; regularization {temp_reg:.1e} for this iteration");
                }
            } else if worst < 0.95 * prev_worst_acc {
                temp_reg = 0.0;
            }
            prev_worst_acc = worst;
        }

        // ---- 右辺 ----
        // r_x = -(c + ρ(x - ξ) + A^T y - z_l + z_u) = -(dual_res + ρ(x - ξ))
        r_x.par_iter_mut().enumerate().with_min_len(PAR_MIN_LEN).for_each(|(j, r)| *r = -(dual_res[j] + rho * (x[j] - xi[j])));
        // r_y = -(A x + δ(λ - y) - b)
        r_y.par_iter_mut().enumerate().with_min_len(PAR_MIN_LEN).for_each(|(i, r)| *r = -(ax[i] + delta * (lambda[i] - y[i]) - b[i]));
        lo.update_rz_w(&x, delta);
        up.update_rz_w(&x, delta);
        // ---- 行列 (予測子・修正子で共通) ----
        {
            let (wl, wu) = (&lo.w, &up.w);
            top.par_iter_mut().enumerate().with_min_len(PAR_MIN_LEN).for_each(|(j, t)| {
                let mut v = rho;
                if pos.lo[j] != usize::MAX {
                    v += 1.0 / wl[pos.lo[j]];
                }
                if pos.up[j] != usize::MAX {
                    v += 1.0 / wu[pos.up[j]];
                }
                *t = v;
            });
        }
        let t_f = std::time::Instant::now();
        let fac_ok = kkt.factor(&top, delta, &mut mid);
        let last_tf = t_f.elapsed().as_secs_f64();
        prof.0 += last_tf;
        if !fac_ok {
            rho = (rho * 100.0).max(1e-8);
            delta = (delta * 100.0).max(1e-8);
            if debug {
                eprintln!("IPM it={it} factorization failed; raising regularization");
            }
            cached = Some(res);
            continue;
        }

        // 1) 予測子: r_s = -S z
        lo.set_rs(false, 0.0);
        up.set_rs(false, 0.0);
        let t_s = std::time::Instant::now();
        let solve_rel = newton(&mut kkt, a, &at, &top, delta, &r_x, &r_y, &mut lo, &mut up, &pos, &mut sol_aff, n, &mut refine_work);
        prof.1 += t_s.elapsed().as_secs_f64();
        prev_solve_rel = solve_rel;
        // 既定 t = 1e-6 (第 55 回、0 で使わない) (`ENOMOTO_T_IPM_AUG_ON_INACCURATE=t`): 正規方程式の予測子の Newton 系の相対残差が t を超えたら拡大系に切り替えて
        // 同じ点で解き直す (`ENOMOTO_T_IPM_AUG_ON_INACCURATE_ANY=1` でなければ、稠密な列を Woodbury で扱っているときだけ。
        // ns1688926: 外した後の疎な部分がほぼ特異になり、44 反復目から相対残差 1e3〜1e12 で 200 反復空回りした)。
        if aug_on_inacc > 0.0
            && solve_rel > aug_on_inacc
            && kkt.is_normal()
            && (aug_on_inacc_any || kkt.has_dense_cols())
            && sol_aff.iter().all(|v| v.is_finite())
        {
            kkt = IpmKkt::augmented(a);
            crate::phase_timing::mark("ipm_switch_augmented");
            if debug {
                eprintln!("IPM it={it} inaccurate normal-equations solve ({solve_rel:.2e}); switching to the augmented system");
            }
            cached = Some(res);
            continue;
        }
        // 試験用 (`ENOMOTO_T_IPM_SOLVE_ACC=t`): 予測子の Newton 系の (反復改良後の) 相対残差が t を超えたら (分解の精度が
        // 足りない: ken-18 は 30 反復目から 1e-5〜1e2 になり、方向が意味をなさず 35 反復空回りした)、この反復を捨てて
        // ρ・δ を `ENOMOTO_T_IPM_SOLVE_ACC_BUMP` 倍 (下限 `ENOMOTO_T_IPM_SOLVE_ACC_MIN`) に強めて同じ点で解き直す。
        if solve_acc > 0.0 && solve_rel > solve_acc && acc_retries < 5 && sol_aff.iter().all(|v| v.is_finite()) {
            acc_retries += 1;
            rho = (rho * solve_acc_bump).max(solve_acc_min);
            delta = (delta * solve_acc_bump).max(solve_acc_min);
            crate::phase_timing::mark("ipm_solve_inaccurate");
            if debug {
                eprintln!("IPM it={it} inaccurate Newton solve (rel residual {solve_rel:.2e}); raising regularization to rho={rho:.1e} delta={delta:.1e}");
            }
            cached = Some(res);
            continue;
        }
        acc_retries = 0;
        if !sol_aff.iter().all(|v| v.is_finite()) {
            // 正規方程式の破綻は、切り替えが有効なら拡大系に切り替える。
            if switch_aug_k > 0 && kkt.is_normal() {
                kkt = IpmKkt::augmented(a);
                crate::phase_timing::mark("ipm_switch_augmented");
            }
            // 分解が破綻した: 正則化を強めて次の反復で分解し直す。
            rho = (rho * 100.0).max(1e-8);
            delta = (delta * 100.0).max(1e-8);
            if debug {
                eprintln!("IPM it={it} non-finite Newton direction; raising regularization to rho={rho:.1e} delta={delta:.1e}");
            }
            cached = Some(res);
            continue;
        }
        for side in [&mut lo, &mut up] {
            side.dz_aff.copy_from_slice(&side.dz);
            side.ds_aff.copy_from_slice(&side.ds);
        }
        let alpha_p_aff = fraction_to_boundary(&lo.s, &lo.ds_aff).min(fraction_to_boundary(&up.s, &up.ds_aff));
        let alpha_d_aff = fraction_to_boundary(&lo.z, &lo.dz_aff).min(fraction_to_boundary(&up.z, &up.dz_aff));
        let sz: f64 = dot(&lo.s, &lo.z) + dot(&up.s, &up.z);
        let mu = sz / n_bnd as f64;
        let mut sz_aff = 0.0;
        for side in [&lo, &up] {
            sz_aff += det_sum(side.len(), |k| (side.s[k] + alpha_p_aff * side.ds_aff[k]) * (side.z[k] + alpha_d_aff * side.dz_aff[k]));
        }
        let mu_aff = sz_aff / n_bnd as f64;
        let sigma = (mu_aff / mu.max(GAP_DIV_GUARD)).clamp(0.0, 1.0).powi(3);

        // 2) 修正子 + 中心化
        lo.set_rs(true, sigma * mu);
        up.set_rs(true, sigma * mu);
        let t_s = std::time::Instant::now();
        newton(&mut kkt, a, &at, &top, delta, &r_x, &r_y, &mut lo, &mut up, &pos, &mut rhs, n, &mut refine_work);
        let last_ts = t_s.elapsed().as_secs_f64();
        prof.1 += last_ts;
        // 試験用 (`ENOMOTO_T_IPM_GONDZIO_AUTO=C`): 補正子の回数を、この反復の分解の時間が Newton 系 1 回の求解の
        // 時間の何倍かで決める (`floor(t_分解 / (C t_求解))`、上限 `ENOMOTO_T_IPM_GONDZIO_AUTO_MAX`)。分解が重く求解が
        // 軽い問題 (dfl001・pds-20・ken-18) だけ補正子で反復を減らし、求解が重い問題 (osa-60) では使わない。
        let gondzio_k = if gondzio_auto > 0.0 && last_ts > 0.0 {
            gondzio_max.max(((last_tf / (gondzio_auto * last_ts)) as usize).min(gondzio_auto_max))
        } else {
            gondzio_max
        };
        let mut alpha_p = fraction_to_boundary(&lo.s, &lo.ds).min(fraction_to_boundary(&up.s, &up.ds));
        let mut alpha_d = fraction_to_boundary(&lo.z, &lo.dz).min(fraction_to_boundary(&up.z, &up.dz));
        // 3) Gondzio の多重中心性補正子 (Gondzio 1996、Colombo & Gondzio 2008): ステップ幅を伸ばした試行点の
        //    相補積 v を [β_min σμ, β_max σμ] に寄せる補正を相補性の右辺に足し、同じ分解で解き直す。
        //    ステップ幅が十分伸びたときだけ採る。
        for _ in 0..gondzio_k {
            const DELTA_ALPHA: f64 = 0.1;
            const BETA_MIN: f64 = 0.1;
            const BETA_MAX: f64 = 10.0;
            const ACCEPT: f64 = 0.01;
            let (ap0, ad0) = (alpha_p, alpha_d);
            // 試験用 `ENOMOTO_T_IPM_GONDZIO_SMALL_STEP=t`: 歩幅が t 未満の反復 (中心性を失った反復) だけ補正する。
            if ap0.min(ad0) >= 0.999 || ap0.min(ad0) >= gondzio_small_step {
                break;
            }
            let (tp, td) = ((ap0 + DELTA_ALPHA).min(1.0), (ad0 + DELTA_ALPHA).min(1.0));
            let target = sigma * mu;
            let saved_rhs = rhs.clone();
            let saved: Vec<(Vec<f64>, Vec<f64>, Vec<f64>)> =
                [&lo, &up].iter().map(|sd| (sd.r_s.clone(), sd.dz.clone(), sd.ds.clone())).collect();
            for side in [&mut lo, &mut up] {
                let (sv, zv, dsv, dzv) = (&side.s, &side.z, &side.ds, &side.dz);
                side.r_s.par_iter_mut().enumerate().with_min_len(PAR_MIN_LEN).for_each(|(k, r)| {
                    let v = (sv[k] + tp * dsv[k]) * (zv[k] + td * dzv[k]);
                    let t = v.clamp(BETA_MIN * target, BETA_MAX * target);
                    // 大きすぎる積を下げる補正は -β_max σμ までに抑える。
                    *r += (t - v).max(-BETA_MAX * target);
                });
            }
            let t_s = std::time::Instant::now();
            newton(&mut kkt, a, &at, &top, delta, &r_x, &r_y, &mut lo, &mut up, &pos, &mut rhs, n, &mut refine_work);
            prof.1 += t_s.elapsed().as_secs_f64();
            let ap1 = fraction_to_boundary(&lo.s, &lo.ds).min(fraction_to_boundary(&up.s, &up.ds));
            let ad1 = fraction_to_boundary(&lo.z, &lo.dz).min(fraction_to_boundary(&up.z, &up.dz));
            if rhs.iter().all(|v| v.is_finite()) && ap1 + ad1 >= ap0 + ad0 + ACCEPT * DELTA_ALPHA {
                alpha_p = ap1;
                alpha_d = ad1;
            } else {
                rhs.copy_from_slice(&saved_rhs);
                for (side, (rs, dz, ds)) in [&mut lo, &mut up].into_iter().zip(saved) {
                    side.r_s = rs;
                    side.dz = dz;
                    side.ds = ds;
                }
                break;
            }
        }
        if debug {
            // 歩幅と中心性 (相補積の最小・最大と平均の比)。
            let (mut pmin, mut pmax) = (f64::INFINITY, 0.0f64);
            for side in [&lo, &up] {
                for k in 0..side.len() {
                    let v = side.s[k] * side.z[k];
                    pmin = pmin.min(v);
                    pmax = pmax.max(v);
                }
            }
            eprintln!(
                "IPM it={it:3} alpha_p={alpha_p:.2e} alpha_d={alpha_d:.2e} mu={mu:.2e} sigma={sigma:.2e} min(sz)/mu={:.1e} max(sz)/mu={:.1e}",
                pmin / mu.max(GAP_DIV_GUARD),
                pmax / mu.max(GAP_DIV_GUARD)
            );
        }
        {
            let (dx, dy) = rhs.split_at(n);
            x_new.par_iter_mut().enumerate().with_min_len(PAR_MIN_LEN).for_each(|(j, v)| *v = x[j] + alpha_p * dx[j]);
            y_new.par_iter_mut().enumerate().with_min_len(PAR_MIN_LEN).for_each(|(i, v)| *v = y[i] + alpha_d * dy[i]);
        }
        lo.set_new(alpha_p, alpha_d);
        up.set_new(alpha_p, alpha_d);

        // ---- Algorithm 2: 正則化と近接中心の更新 ----
        let gap_before = sz.max(GAP_DIV_GUARD);
        let gap_after = dot(&lo.s_new, &lo.z_new) + dot(&up.s_new, &up.z_new);
        // 相補性の相対減少率。親モジュールは絶対値を取るが、ギャップが増えた反復で r > 1 になると
        // ρ, δ が一気に下限へ落ちて (adlittle で 3 反復目) 早すぎる Farkas 判定を招くので、[0, 0.999] に収める。
        let r = ((gap_before - gap_after) / gap_before).clamp(0.0, 0.999);
        csr_mat_vec_into(a, &x_new, &mut ax_new);
        at_mul(a, &at, &y_new, &mut aty_new);
        let res_new = residuals(&ax_new, b, &x_new, &lo, &up, &lo.s_new, &up.s_new, c, &aty_new, &lo.z_new, &up.z_new, &pos, &mut dual_new);
        // 試験用 (`ENOMOTO_T_IPM_NAN_RECOVER=K`): 新しい点の残差が非有限 (分解の精度の破綻: dfl001) なら、その歩を
        // 捨てて正則化を強め、今の点から解き直す (K 回まで)。既定は打ち切って最良の反復点を返す。
        if nan_recover_left > 0 && !(res_new.primal.is_finite() && res_new.dual.is_finite()) {
            nan_recover_left -= 1;
            rho = (rho * 100.0).max(1e-8);
            delta = (delta * 100.0).max(1e-8);
            if debug {
                eprintln!("IPM it={it} non-finite new point; raising regularization to rho={rho:.1e} delta={delta:.1e}");
            }
            cached = Some(res);
            continue;
        }
        // 残差が十分減らなかった反復の正則化の減らし方 `(1 - r/slow_div)`。既定は PIQP の 3、
        // 試験用 `ENOMOTO_T_IPM_REG_MODE=1` で IP-PMM の著者の実装の `(1 - 0.666 r)` (slow_div = 1/0.666)。
        if let Some((r0, d0)) = temp_saved {
            rho = r0;
            delta = d0;
        }
        let slow_div = if reg_mode == 1 { 1.0 / 0.666 } else { SLOW_DECREASE_DIVISOR };
        if prox_always || res_new.primal <= RES_DECREASE_RATIO * res.primal {
            lambda.copy_from_slice(&y_new);
            lo.nu.copy_from_slice(&lo.z_new);
            up.nu.copy_from_slice(&up.z_new);
            delta *= 1.0 - r;
        } else {
            delta *= 1.0 - r / slow_div;
        }
        if prox_always || res_new.dual <= RES_DECREASE_RATIO * res.dual {
            xi.copy_from_slice(&x_new);
            rho *= 1.0 - r;
        } else {
            rho *= 1.0 - r / slow_div;
        }
        // 正則化を μ に連動させる (Pougkakiotis–Gondzio の理論の版は ρ_k = δ_k = μ_k):
        //   2: ρ = δ = κ μ (近接中心の更新の条件は上のまま)、3: 上の規則の値を κ μ で頭打ちにする。
        if reg_mode == 2 || reg_mode == 3 {
            let mu_new = gap_after / n_bnd as f64;
            let target = reg_kappa * mu_new;
            if reg_mode == 2 {
                rho = target;
                delta = target;
            } else {
                rho = rho.min(target);
                delta = delta.min(target);
            }
        }
        // 試験用 (`ENOMOTO_T_IPM_REG_GAP_FLOOR=κ`): 正則化を相対双対ギャップに比例する値より下げない
        // (ρ, δ >= κ · min(1, gap / (1 + |c·x| + |b·y + h·z|)))。ギャップが大きいまま正則化が下限に落ちて停滞する
        // 問題 (dfl001, fome13) 用。相対ギャップは 1 で頭打ちにする (序盤の巨大なギャップで正則化が跳ね上がらないように)。
        if reg_gap_floor > 0.0 {
            let floor = reg_gap_floor * (gap / (1.0 + cx.abs() + (by + hz).abs())).min(1.0);
            if floor.is_finite() {
                rho = rho.max(floor);
                delta = delta.max(floor);
            }
        }
        delta = delta.max(delta_min);
        rho = rho.max(rho_min);

        std::mem::swap(&mut x, &mut x_new);
        std::mem::swap(&mut y, &mut y_new);
        // 新しい点の残差は計算済みなので次の反復の先頭で使い回す。
        std::mem::swap(&mut ax, &mut ax_new);
        std::mem::swap(&mut aty, &mut aty_new);
        std::mem::swap(&mut dual_res, &mut dual_new);
        cached = Some(res_new);
        for side in [&mut lo, &mut up] {
            std::mem::swap(&mut side.s, &mut side.s_new);
            std::mem::swap(&mut side.z, &mut side.z_new);
        }
        iters = it + 1;
    }

    if pushing && status != Status::Optimal {
        // 高精度化の途中で止めた: 停止基準を満たした最良の点を最適として返す。
        status = Status::Optimal;
        if let Some((_, bx, by, brc, brel)) = best {
            crate::phase_timing::record("ipm_push_best", brel.0.max(brel.1).max(brel.2));
            return BoxIpmResult { status, x: bx, y: by.iter().map(|v| -v).collect(), rc: brc, iters, rel_res: brel };
        }
    }
    if status != Status::Optimal {
        if let Some((_, bx, by, brc, brel)) = best {
            if debug {
                eprintln!("IPM end status={status:?} iters={iters}; returning best iterate rel_res={brel:?}");
        eprintln!("IPM profile total={:.3}s factor={:.3}s solve={:.3}s other={:.3}s", t_kkt.elapsed().as_secs_f64(), prof.0, prof.1, t_kkt.elapsed().as_secs_f64() - prof.0 - prof.1);
            }
            return BoxIpmResult { status, x: bx, y: by.iter().map(|v| -v).collect(), rc: brc, iters, rel_res: brel };
        }
    }
    let mut rc = vec![0.0; n];
    for k in 0..lo.len() {
        rc[lo.idx[k]] += lo.z[k];
    }
    for k in 0..up.len() {
        rc[up.idx[k]] -= up.z[k];
    }
    if debug {
        let (asm, num) = crate::interior_point::kkt::take_factor_prof();
        eprintln!("IPM factor breakdown: assemble={asm:.3}s numeric={num:.3}s");
        eprintln!("IPM end status={status:?} iters={iters} rel_res={rel:?}");
        eprintln!("IPM profile total={:.3}s factor={:.3}s solve={:.3}s other={:.3}s", t_kkt.elapsed().as_secs_f64(), prof.0, prof.1, t_kkt.elapsed().as_secs_f64() - prof.0 - prof.1);
    }
    BoxIpmResult { status, x, y: y.iter().map(|v| -v).collect(), rc, iters, rel_res: rel }
}

/// 予測子/修正子の Newton 方向: 各 `Side` の `r_z`・`r_s`・`w` から縮約系の右辺を作って解き、
/// `sol[..n] = dx`、`sol[n..] = dy`、各 `Side` の `dz`・`ds` を書く。行列は分解済み。
#[allow(clippy::too_many_arguments)]
fn newton(kkt: &mut IpmKkt, a: &FaerCsr, at: &FaerCsr, top: &[f64], delta: f64, r_x: &[f64], r_y: &[f64], lo: &mut Side, up: &mut Side, pos: &Pos, sol: &mut [f64], n: usize, work: &mut Vec<f64>) -> f64 {
    lo.set_rzp();
    up.set_rzp();
    {
        // sol_x = r_x + G^T W^{-1} rz' (列ごとに並列)
        let (sl, su) = (&*lo, &*up);
        sol[..n].par_iter_mut().enumerate().with_min_len(PAR_MIN_LEN).for_each(|(j, o)| {
            let mut v = r_x[j];
            let kl = pos.lo[j];
            if kl != usize::MAX {
                v += sl.sgn * sl.rzp[kl] / sl.w[kl];
            }
            let ku = pos.up[j];
            if ku != usize::MAX {
                v += su.sgn * su.rzp[ku] / su.w[ku];
            }
            *o = v;
        });
    }
    sol[n..].copy_from_slice(r_y);
    // 正則化が小さくなると分解の精度が落ちるので反復改良する (動的正則化で置き換えた
    // ピボットの誤差もここで取り戻す)。残差が十分小さければ追加の求解はしない。
    let rel = kkt.solve_refined(a, at, top, delta, sol, REFINE_STEPS, work);
    lo.set_dz_ds(&sol[..n]);
    up.set_dz_ds(&sol[..n]);
    rel
}

/// 主実行不能の Farkas 証明。内部の双対 (`c + A^T y + G^T z = 0`、`z >= 0`、`G x <= h`) で、実行可能な
/// `x` があれば `r = A^T y + G^T z` について `x·r = b·y + z·(G x) <= b·y + h·z` なので、
/// `min_{l <= x <= u} x·r > b·y + h·z` なら実行可能な `x` は無い。`r ≈ 0` の近似ではなく、残差 `r` を
/// 箱の上での最小値として勘定に入れて判定する (境界が大きいと残差が増幅されるため)。無限の境界の向きに
/// 有意な `r_j` があれば最小値は `-inf` なので証明にならない。余裕は `CERT_TOL * scale`。
///
/// (以前は `‖r‖/scale` が小さく `(b·y + h·z)/scale > CERT_TOL` なら証明としていたが、符号が逆で、
/// 双対実行可能で乗数が大きく双対目的値が悪い収束途中の点を実行不能と誤判定した: dfl001。)
fn primal_infeasibility_certificate(a: &FaerCsr, b: &[f64], y: &[f64], lo: &Side, up: &Side, buf: &mut [f64]) -> bool {
    let scale = norm_inf(y).max(norm_inf(&lo.z)).max(norm_inf(&up.z));
    if scale < CERT_SCALE_MIN {
        return false;
    }
    csr_mat_t_vec_into(a, y, buf);
    let mut obj = dot(b, y);
    let n = buf.len();
    let mut lower = vec![f64::NEG_INFINITY; n];
    let mut upper = vec![f64::INFINITY; n];
    for side in [lo, up] {
        for k in 0..side.len() {
            let j = side.idx[k];
            buf[j] += side.sgn * side.z[k];
            obj += side.sgn * side.bnd[k] * side.z[k];
            if side.sgn < 0.0 {
                lower[j] = side.bnd[k];
            } else {
                upper[j] = side.bnd[k];
            }
        }
    }
    // 無限の境界の向きの成分で、これ以下は 0 とみなす (丸め誤差)。
    let tiny = 1e-12 * scale;
    let mut box_min = 0.0;
    for j in 0..n {
        let r = buf[j];
        if r > 0.0 {
            if lower[j].is_finite() {
                box_min += lower[j] * r;
            } else if r > tiny {
                box_min = f64::NEG_INFINITY;
                break;
            }
        } else if r < 0.0 {
            if upper[j].is_finite() {
                box_min += upper[j] * r;
            } else if -r > tiny {
                box_min = f64::NEG_INFINITY;
                break;
            }
        }
    }
    let ok = box_min - obj > CERT_TOL * scale;
    if env_str!("ENOMOTO_DEBUG_IPM").is_some() {
        eprintln!("IPM farkas(primal) min_box(x·r)={box_min:.3e} b·y+h·z={obj:.3e} scale={scale:.2e} -> {ok}");
    }
    ok
}

/// 双対実行不能 (非有界) の証明: `A x ≈ 0`、`G x <= 0`、`c·x < 0` (正規化後)。
fn dual_infeasibility_certificate(a: &FaerCsr, c: &[f64], x: &[f64], lo: &Side, up: &Side, buf: &mut [f64]) -> bool {
    let scale = norm_inf(x);
    if scale < CERT_SCALE_MIN {
        return false;
    }
    csr_mat_vec_into(a, x, buf);
    let eq_ok = norm_inf(buf) / scale < CERT_TOL;
    let ineq_ok = [lo, up].iter().all(|side| side.idx.iter().all(|&j| side.sgn * x[j] / scale <= CERT_TOL));
    eq_ok && ineq_ok && dot(c, x) / scale < -CERT_TOL
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    #[test]
    fn box_ipm_detects_infeasible_and_solves_feasible() {
        // x0 + x1 = 3、0 <= x <= 1: 実行不能。
        let a = csr_from_rows(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let r = solve_box_lp(&a, &[3.0], &[1.0, 1.0], &[0.0, 0.0], &[1.0, 1.0], 200);
        assert_eq!(r.status, Status::Infeasible);
        // x0 + x1 = 1.5、0 <= x <= 1、min -x0 - 2 x1: 最適 (0.5, 1)、目的値 -2.5 (負の目的値で誤判定しない)。
        let r = solve_box_lp(&a, &[1.5], &[-1.0, -2.0], &[0.0, 0.0], &[1.0, 1.0], 200);
        assert_eq!(r.status, Status::Optimal);
        assert!((r.x[0] - 0.5).abs() < 1e-6 && (r.x[1] - 1.0).abs() < 1e-6, "x = {:?}", r.x);
    }
}
