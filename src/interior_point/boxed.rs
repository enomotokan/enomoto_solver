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

use super::kkt::{csr_mat_t_vec_into, csr_mat_vec_into, FaerCsr, IpmKkt};
use crate::params::interior_point::{
    CERT_SCALE_MIN, CERT_TOL, DELTA0, DELTA_MIN, EPS_ABS, EPS_REL, GAP_DIV_GUARD, INIT_DIV_GUARD, INIT_POSITIVE_FLOOR,
    INIT_SHIFT_MULTIPLIER, REG_FLOOR_SLACK, RES_DECREASE_RATIO, RHO0, RHO_MIN, SLOW_DECREASE_DIVISOR, STALL_ITERS,
    STALL_PROGRESS_RATIO, TAU,
};
use crate::types::Status;

/// 並列ループの 1 タスクあたりの最小の長さ (小さな問題で rayon の手間が計算を上回らないように)。
const PAR_MIN_LEN: usize = 4096;

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

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.par_iter().zip(b.par_iter()).with_min_len(PAR_MIN_LEN).map(|(x, y)| x * y).sum()
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
    let mut r = solve_box_lp_scaled(a, &bs, &cs, &ls, &us, max_iters);
    for v in r.x.iter_mut() {
        *v *= beta;
    }
    for v in r.y.iter_mut().chain(r.rc.iter_mut()) {
        *v *= gamma;
    }
    r
}

/// [`solve_box_lp`] の本体 (大きさをそろえた後の問題を解く)。
fn solve_box_lp_scaled(a: &FaerCsr, b: &[f64], c: &[f64], l: &[f64], u: &[f64], max_iters: usize) -> BoxIpmResult {
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
    let mut kkt = IpmKkt::new(a);
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

    let mut rho = RHO0;
    let mut delta = DELTA0;
    let rho_min = tunable!("ENOMOTO_T_IPM_RHO_MIN", RHO_MIN, f64);
    let delta_min = tunable!("ENOMOTO_T_IPM_DELTA_MIN", DELTA_MIN, f64);

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
    kkt.solve_refined(a, &top, delta, &mut rhs, REFINE_STEPS, &mut refine_work);
    let mut xi = rhs[..n].to_vec();
    let mut y = rhs[n..].to_vec();
    let mut x = xi.clone();

    if n_bnd == 0 {
        let mut aty = vec![0.0; n];
        csr_mat_t_vec_into(a, &y, &mut aty);
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
                csr_mat_t_vec_into(a, &y, &mut aty);
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
        let bnd_p = EPS_ABS + EPS_REL * norm_inf(&ax).max(norm_b).max(gx).max(norm_h).max(s_inf);
        let bnd_d = EPS_ABS + EPS_REL * norm_inf(&aty).max(z_inf).max(norm_c);
        let bnd_g = EPS_ABS + EPS_REL * cx.abs().max(by.abs()).max(hz.abs());
        rel = (res.primal / bnd_p, res.dual / bnd_d, gap / bnd_g);
        if debug {
            eprintln!(
                "IPM t={:.2}s it={it:3} pobj={cx:.10e} pres={:.2e} dres={:.2e} gap={:.2e} rho={rho:.1e} delta={delta:.1e}",
                t_kkt.elapsed().as_secs_f64(), res.primal, res.dual, gap
            );
        }
        if res.primal <= bnd_p && res.dual <= bnd_d && gap <= bnd_g {
            status = Status::Optimal;
            break;
        }
        let worst = rel.0.max(rel.1).max(rel.2);
        if !worst.is_finite() || !cx.is_finite() {
            // 数値的に破綻した (分解の失敗など): 最良の反復点に戻して打ち切る。
            break;
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

        let at_floor = rho <= rho_min * REG_FLOOR_SLACK && delta <= delta_min * REG_FLOOR_SLACK;
        if res.primal >= prev_p * STALL_PROGRESS_RATIO && res.dual >= prev_d * STALL_PROGRESS_RATIO && at_floor {
            stall += 1;
        } else {
            stall = 0;
        }
        // Farkas 証明は正則化が下限に達し、しかも残差が改善しなくなってから試す (親モジュールは
        // 下限に達しただけで試すが、収束途中の大きな乗数で誤って成立することがある: pilot4)。
        if at_floor && stall >= 2 {
            if primal_infeasibility_certificate(a, b, &y, &lo, &up, &mut aty_new) {
                status = Status::Infeasible;
                break;
            }
            if dual_infeasibility_certificate(a, c, &x, &lo, &up, &mut ax_new) {
                status = Status::Unbounded;
                break;
            }
        }
        prev_p = res.primal;
        prev_d = res.dual;
        if stall >= STALL_ITERS {
            break;
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
        prof.0 += t_f.elapsed().as_secs_f64();
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
        newton(&mut kkt, a, &top, delta, &r_x, &r_y, &mut lo, &mut up, &pos, &mut sol_aff, n, &mut refine_work);
        prof.1 += t_s.elapsed().as_secs_f64();
        if !sol_aff.iter().all(|v| v.is_finite()) {
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
            sz_aff += (0..side.len())
                .into_par_iter()
                .with_min_len(PAR_MIN_LEN)
                .map(|k| (side.s[k] + alpha_p_aff * side.ds_aff[k]) * (side.z[k] + alpha_d_aff * side.dz_aff[k]))
                .sum::<f64>();
        }
        let mu_aff = sz_aff / n_bnd as f64;
        let sigma = (mu_aff / mu.max(GAP_DIV_GUARD)).clamp(0.0, 1.0).powi(3);

        // 2) 修正子 + 中心化
        lo.set_rs(true, sigma * mu);
        up.set_rs(true, sigma * mu);
        let t_s = std::time::Instant::now();
        newton(&mut kkt, a, &top, delta, &r_x, &r_y, &mut lo, &mut up, &pos, &mut rhs, n, &mut refine_work);
        prof.1 += t_s.elapsed().as_secs_f64();
        let alpha_p = fraction_to_boundary(&lo.s, &lo.ds).min(fraction_to_boundary(&up.s, &up.ds));
        let alpha_d = fraction_to_boundary(&lo.z, &lo.dz).min(fraction_to_boundary(&up.z, &up.dz));
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
        csr_mat_t_vec_into(a, &y_new, &mut aty_new);
        let res_new = residuals(&ax_new, b, &x_new, &lo, &up, &lo.s_new, &up.s_new, c, &aty_new, &lo.z_new, &up.z_new, &pos, &mut dual_new);
        if res_new.primal <= RES_DECREASE_RATIO * res.primal {
            lambda.copy_from_slice(&y_new);
            lo.nu.copy_from_slice(&lo.z_new);
            up.nu.copy_from_slice(&up.z_new);
            delta *= 1.0 - r;
        } else {
            delta *= 1.0 - r / SLOW_DECREASE_DIVISOR;
        }
        delta = delta.max(delta_min);
        if res_new.dual <= RES_DECREASE_RATIO * res.dual {
            xi.copy_from_slice(&x_new);
            rho *= 1.0 - r;
        } else {
            rho *= 1.0 - r / SLOW_DECREASE_DIVISOR;
        }
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
        eprintln!("IPM end status={status:?} iters={iters} rel_res={rel:?}");
        eprintln!("IPM profile total={:.3}s factor={:.3}s solve={:.3}s other={:.3}s", t_kkt.elapsed().as_secs_f64(), prof.0, prof.1, t_kkt.elapsed().as_secs_f64() - prof.0 - prof.1);
    }
    BoxIpmResult { status, x, y: y.iter().map(|v| -v).collect(), rc, iters, rel_res: rel }
}

/// 予測子/修正子の Newton 方向: 各 `Side` の `r_z`・`r_s`・`w` から縮約系の右辺を作って解き、
/// `sol[..n] = dx`、`sol[n..] = dy`、各 `Side` の `dz`・`ds` を書く。行列は分解済み。
#[allow(clippy::too_many_arguments)]
fn newton(kkt: &mut IpmKkt, a: &FaerCsr, top: &[f64], delta: f64, r_x: &[f64], r_y: &[f64], lo: &mut Side, up: &mut Side, pos: &Pos, sol: &mut [f64], n: usize, work: &mut Vec<f64>) {
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
    kkt.solve_refined(a, top, delta, sol, REFINE_STEPS, work);
    lo.set_dz_ds(&sol[..n]);
    up.set_dz_ds(&sol[..n]);
}

/// 主実行不能の Farkas 証明: `A^T y + G^T z ≈ 0` かつ `b·y + h·z > 0` (正規化後)。
fn primal_infeasibility_certificate(a: &FaerCsr, b: &[f64], y: &[f64], lo: &Side, up: &Side, buf: &mut [f64]) -> bool {
    let scale = norm_inf(y).max(norm_inf(&lo.z)).max(norm_inf(&up.z));
    if scale < CERT_SCALE_MIN {
        return false;
    }
    csr_mat_t_vec_into(a, y, buf);
    let mut obj = dot(b, y);
    for side in [lo, up] {
        for k in 0..side.len() {
            buf[side.idx[k]] += side.sgn * side.z[k];
            obj += side.sgn * side.bnd[k] * side.z[k];
        }
    }
    norm_inf(buf) / scale < CERT_TOL && obj / scale > CERT_TOL
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
