//! 内点近接乗数法 (IP-PMM) + Mehrotra の予測子・修正子法による LP ソルバー。
//! PIQP (Schwan, Jiang, Kuhn, Jones, "PIQP: A Proximal Interior-Point Quadratic
//! Programming Solver", CDC 2023) のアルゴリズムを P = 0 (線形計画) に特化して
//! 独自に実装したもの。各反復の KKT (Newton) 系はスラックのステップを消去すると
//! 対称準定値になり (PIQP 論文の Remark 1)、faer の疎行列機能だけで組み立て・分解する
//! (`kkt.rs`)。`A`/`G` は最後まで CSR のまま扱う。
//!
//! 前処理: 単体法と同じ拡張前処理 `presolve::run_extended` (Ruiz 平衡化、冗長等式の
//! 除去、propagate/dualfix/行シングルトン/ダブルトン/列シングルトンのラウンド) を
//! 最初に一度だけ実行し、縮小・スケーリング後の問題を解く。消去された変数の値は
//! `unscale_with_substitutions` で復元する。KKT 行列の AMD 順序と記号分解も一度だけ
//! 計算し、反復ごとには数値分解だけをやり直す。
//!
//! メモリ確保: Newton ループが使うバッファ (残差、方向、試行点、KKT の右辺・解) は
//! すべて `Workspace::new` でループ前に一度だけ確保し、毎反復上書きする。ループ本体で
//! 確保するのは (稀な) Farkas 証明の判定と、呼び出し元に返す最終の `Vec<f64>` だけ。
//!
//! 位置付け: 既定のエンジンではない。`Model.solve(root_solver="interior")` を
//! 明示したときだけ `solver::solve_lp` から呼ばれる。単体法との突き合わせ検証
//! (`simplex.rs` の `*_matches_independent_ipm_solver` テスト) にも使う。

pub mod qp;
pub mod kkt;
pub mod boxed;

use rayon::prelude::*;

use self::kkt::{csr_mat_t_vec, csr_mat_t_vec_into, csr_mat_vec, csr_mat_vec_into, FaerCsr, SparseKkt};
use self::qp::QpStd;
use crate::presolve::{self, scaling};
use crate::types::{ConstraintRow, Objective, Sense, Status, VariableData};
use crate::params::interior_point::{
    CERT_SCALE_MIN, CERT_TOL, DELTA0, DELTA_MIN, EPS_ABS, EPS_REL, GAP_DIV_GUARD, INIT_DIV_GUARD, INIT_POSITIVE_FLOOR,
    INIT_SHIFT_MULTIPLIER, MAX_ITERS, NO_INEQ_DUAL_RES_TOL, PRESOLVE_ROUNDS, PROPAGATION_PASSES, REG_FLOOR_SLACK,
    RES_DECREASE_RATIO, RHO0, RHO_MIN, ROWSINGLETON_COLSINGLETON_INNER_ROUNDS, RUIZ_ITERS, SLOW_DECREASE_DIVISOR,
    STALL_ITERS, STALL_PROGRESS_RATIO, TAU,
};

/// 内点法の求解結果。
pub struct IpmResult {
    /// 結果状態。
    pub status: Status,
    /// 解ベクトル (元の変数空間。`Optimal` のときだけ `Some`)。
    pub x: Option<Vec<f64>>,
}

// 以下の要素ごと演算・リダクションの補助関数は、添字ごとに独立 (または結合的な
// リダクション) でヒープ確保もしないので、rayon で並列化している。

/// 内積 `a · b`。
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.par_iter().zip(b.par_iter()).map(|(x, y)| x * y).sum()
}

/// 無限大ノルム `max |v_i|` (空なら 0)。
fn norm_inf(v: &[f64]) -> f64 {
    v.par_iter().map(|x| x.abs()).reduce(|| 0.0_f64, f64::max)
}

/// `out += a * x`。
fn axpy(out: &mut [f64], a: f64, x: &[f64]) {
    out.par_iter_mut().zip(x.par_iter()).for_each(|(o, &xi)| *o += a * xi);
}

/// `out[i] = a[i] - b[i]`。
fn write_sub(out: &mut [f64], a: &[f64], b: &[f64]) {
    out.par_iter_mut().zip(a.par_iter()).zip(b.par_iter()).for_each(|((o, &ai), &bi)| *o = ai - bi);
}

/// `out[i] = a[i] + alpha * d[i]`。
fn write_add_scaled(out: &mut [f64], a: &[f64], alpha: f64, d: &[f64]) {
    out.par_iter_mut().zip(a.par_iter()).zip(d.par_iter()).for_each(|((o, &ai), &di)| *o = ai + alpha * di);
}

/// 主問題の実行不能性の Farkas 証明を判定する: `A^T y + G^T z ≈ 0` かつ
/// `b·y + h·z > 0` を満たす `(y, z)` があれば `{Ax=b, Gx<=h}` は空。
/// `(y, z)` は無限大ノルムで正規化してから判定する。正則化が下限に達したとき
/// (まれ) だけ呼ばれるので、メモリ確保してよい。
fn primal_infeasibility_certificate(a: &FaerCsr, g: &FaerCsr, n: usize, b: &[f64], h: &[f64], y: &[f64], z: &[f64]) -> bool {
    let scale = norm_inf(y).max(norm_inf(z));
    if scale < CERT_SCALE_MIN {
        return false;
    }
    let yhat: Vec<f64> = y.iter().map(|v| v / scale).collect();
    let zhat: Vec<f64> = z.iter().map(|v| v / scale).collect();
    // 定常性残差 A^T ŷ + G^T ẑ
    let mut stat = csr_mat_t_vec(a, n, &yhat);
    axpy(&mut stat, 1.0, &csr_mat_t_vec(g, n, &zhat));
    norm_inf(&stat) < CERT_TOL && dot(b, &yhat) + dot(h, &zhat) > CERT_TOL
}

/// 双対の実行不能性 (主問題の非有界性) の Farkas 証明を判定する:
/// `Ax ≈ 0`, `Gx <= 0`, `c·x < 0` を満たす `x` は目的関数を減らし続ける非有界な方向。
fn dual_infeasibility_certificate(a: &FaerCsr, g: &FaerCsr, c: &[f64], x: &[f64]) -> bool {
    let scale = norm_inf(x);
    if scale < CERT_SCALE_MIN {
        return false;
    }
    let xhat: Vec<f64> = x.iter().map(|v| v / scale).collect();
    let eq_ok = norm_inf(&csr_mat_vec(a, &xhat)) < CERT_TOL;
    let ineq_ok = csr_mat_vec(g, &xhat).iter().all(|&v| v <= CERT_TOL);
    eq_ok && ineq_ok && dot(c, &xhat) < -CERT_TOL
}

/// fraction-to-boundary 則: `v + alpha * dv` が正のままでいられる最大ステップの
/// `TAU` 倍 (1 以下に切り詰め) を返す。
fn fraction_to_boundary(v: &[f64], dv: &[f64]) -> f64 {
    let mut alpha = 1.0_f64;
    for (&vi, &dvi) in v.iter().zip(dv) {
        if dvi < 0.0 {
            alpha = alpha.min(TAU * vi / (-dvi));
        }
    }
    alpha.clamp(0.0, 1.0)
}

/// スラックを消去した KKT 系を解き、`(dx, dy, dz)` を `sol_out` (長さ n+p+m) に書く。
/// `r_x, r_y, r_z, r_s` は各ブロックの残差 (右辺)、`bottom_diag`/`rhs` は作業領域。
/// メモリ確保しない。
#[allow(clippy::too_many_arguments)]
fn newton_solve(
    kkt: &mut SparseKkt,
    a: &FaerCsr,
    g: &FaerCsr,
    rho: f64,
    delta: f64,
    s: &[f64],
    z: &[f64],
    r_x: &[f64],
    r_y: &[f64],
    r_z: &[f64],
    r_s: &[f64],
    bottom_diag: &mut [f64],
    rhs: &mut [f64],
    sol_out: &mut [f64],
) {
    let n = r_x.len();
    let p = r_y.len();
    let m = r_z.len();

    // 下段対角: -(S Z^{-1} + δ I)
    for i in 0..m {
        bottom_diag[i] = -(s[i] / z[i] + delta);
    }

    rhs[0..n].copy_from_slice(r_x);
    rhs[n..n + p].copy_from_slice(r_y);
    for i in 0..m {
        rhs[n + p + i] = r_z[i] - r_s[i] / z[i]; // スラック消去後の r_z
    }

    kkt.solve_into(a, g, rho, -delta, bottom_diag, rhs, sol_out);
}

/// Newton ループが使う全バッファ。(n, p, m) から一度だけ確保し、毎反復上書きして使う。
/// 名前の `_new` は試行点 (ステップ後) の値、`_aff` はアフィン (予測子) ステップの値。
struct Workspace {
    // ---- 長さ n ----
    /// `A^T y`
    aty: Vec<f64>,
    /// `G^T z`
    gtz: Vec<f64>,
    /// 双対残差 `c + A^T y + G^T z`
    dual_res: Vec<f64>,
    /// KKT 右辺の x ブロック
    r_x: Vec<f64>,
    /// 試行点の x
    x_new: Vec<f64>,
    /// 試行点での `A^T y`
    aty_new: Vec<f64>,
    /// 試行点での `G^T z`
    gtz_new: Vec<f64>,
    /// 試行点での双対残差
    dual_res_new: Vec<f64>,
    /// 長さ n の作業領域
    tmp_n: Vec<f64>,
    // ---- 長さ p ----
    /// `A x`
    ax: Vec<f64>,
    /// KKT 右辺の y ブロック
    r_y: Vec<f64>,
    /// 試行点の y
    y_new: Vec<f64>,
    /// 試行点での `A x`
    ax_new: Vec<f64>,
    /// 長さ p の作業領域
    tmp_p: Vec<f64>,
    // ---- 長さ m ----
    /// `G x`
    gx: Vec<f64>,
    /// KKT 右辺の z ブロック
    r_z: Vec<f64>,
    /// 相補性の右辺 (予測子では `-S z`、修正子では中心化項込み)
    r_s: Vec<f64>,
    /// アフィンステップのスラック方向 ds
    ds_aff: Vec<f64>,
    /// 修正子 (最終) ステップのスラック方向 ds
    ds: Vec<f64>,
    /// アフィンステップ後の試行スラック
    s_aff: Vec<f64>,
    /// アフィンステップ後の試行双対 z
    z_aff: Vec<f64>,
    /// 試行点のスラック
    s_new: Vec<f64>,
    /// 試行点の z
    z_new: Vec<f64>,
    /// 試行点での `G x`
    gx_new: Vec<f64>,
    /// KKT 下段の対角
    bottom_diag: Vec<f64>,
    /// 長さ m の作業領域
    tmp_m: Vec<f64>,
    // ---- 長さ p + m ----
    /// 主残差 (`Ax - b` と `Gx - h + s` を連結)
    primal_res: Vec<f64>,
    /// 試行点での主残差
    primal_res_new: Vec<f64>,
    // ---- 長さ n + p + m ----
    /// KKT 系の右辺
    kkt_rhs: Vec<f64>,
    /// アフィン (予測子) ステップの KKT 解
    kkt_sol_aff: Vec<f64>,
    /// 修正子ステップの KKT 解
    kkt_sol: Vec<f64>,
}

impl Workspace {
    /// 寸法 (n, p, m) に合わせて全バッファを 0 で確保する。
    fn new(n: usize, p: usize, m: usize) -> Self {
        let dim = n + p + m;
        Workspace {
            aty: vec![0.0; n],
            gtz: vec![0.0; n],
            dual_res: vec![0.0; n],
            r_x: vec![0.0; n],
            x_new: vec![0.0; n],
            aty_new: vec![0.0; n],
            gtz_new: vec![0.0; n],
            dual_res_new: vec![0.0; n],
            tmp_n: vec![0.0; n],
            ax: vec![0.0; p],
            r_y: vec![0.0; p],
            y_new: vec![0.0; p],
            ax_new: vec![0.0; p],
            tmp_p: vec![0.0; p],
            gx: vec![0.0; m],
            r_z: vec![0.0; m],
            r_s: vec![0.0; m],
            ds_aff: vec![0.0; m],
            ds: vec![0.0; m],
            s_aff: vec![0.0; m],
            z_aff: vec![0.0; m],
            s_new: vec![0.0; m],
            z_new: vec![0.0; m],
            gx_new: vec![0.0; m],
            bottom_diag: vec![0.0; m],
            tmp_m: vec![0.0; m],
            primal_res: vec![0.0; p + m],
            primal_res_new: vec![0.0; p + m],
            kkt_rhs: vec![0.0; dim],
            kkt_sol_aff: vec![0.0; dim],
            kkt_sol: vec![0.0; dim],
        }
    }
}

/// 前処理で消去された変数 (`doubleton`/`colsingleton`/平行列など) の値を、後処理ログを
/// **逆順**にたどって復元し、最後にスケーリングを戻して元の変数空間の解を返す。
/// `x` はスケーリング後の空間の値 (各 `Substitution` もその空間で記録されている)。
/// 単体法側の `unscale_result` に相当する。
fn unscale_with_substitutions(x: &[f64], sc: &scaling::Scaling, postsolve_log: &[presolve::PostsolveStep]) -> Vec<f64> {
    let mut x = x.to_vec();
    // 共有の時系列ログを 1 回だけ逆順に走査する (種類ごとに分けて処理してはいけない。
    // 後の置換が前の置換で決まる値を参照しうるため)。
    for step in postsolve_log.iter().rev() {
        match step {
            presolve::PostsolveStep::Sub(sub) => x[sub.var] = sub.value(&x),
            presolve::PostsolveStep::ParallelCol(sub) => sub.apply(&mut x),
        }
    }
    scaling::unscale_x(sc, &x)
}

/// 標準形 `qp` を IP-PMM で解く。前処理 → 初期点の計算 → Newton 反復
/// (停止判定・Farkas 証明・停滞判定を含む) → 解の復元、の順に行う。
pub fn solve(qp: &QpStd) -> IpmResult {
    let n = qp.n;

    if n == 0 {
        return IpmResult { status: Status::Optimal, x: Some(vec![]) };
    }

    // 共通の前処理を一度だけ実行する。以降はスケーリング・縮小後の問題を扱い、
    // 各 return で `unscale_with_substitutions` により元の変数空間に戻す。
    let pre = presolve::run_extended(n, &qp.a, &qp.b, &qp.g, &qp.h, &qp.c, RUIZ_ITERS, PROPAGATION_PASSES, PRESOLVE_ROUNDS, ROWSINGLETON_COLSINGLETON_INNER_ROUNDS, true);
    if pre.infeasible {
        return IpmResult { status: Status::Infeasible, x: None };
    }
    if pre.unbounded {
        // 前処理 (`presolve::freevar`) が、どの行にも現れず費用が非零の自由変数を見つけた:
        // どのエンジンで解いても非有界。
        return IpmResult { status: Status::Unbounded, x: None };
    }
    let (g, h) = pre.g_h();
    let sc = pre.scaling;
    let a = pre.a;
    let b = pre.b;
    let c = pre.c;
    let postsolve_log = pre.postsolve_log;
    let p = a.nrows();
    let m = g.nrows();

    let a = &a;
    let g = &g;
    let b = &b;
    let h = &h;
    let c = &c;
    let mut kkt = SparseKkt::new(n, p, m);
    let mut ws = Workspace::new(n, p, m);
    let dim = n + p + m;

    // ---- 初期化 (PIQP 論文 Section IV.A) ----
    // ρ: 主変数側の近接正則化、δ: 双対側の近接正則化
    let mut rho = RHO0;
    let mut delta = DELTA0;

    // 正則化付き KKT 系を 1 回解いて初期値を得る。
    // xi: 主変数の近接中心 ξ、y0_init: 等式乗数の初期値、nu_tilde0: 不等式乗数の暫定値
    let (mut xi, y0_init, nu_tilde0) = {
        let mut rhs = vec![0.0; dim];
        for i in 0..n {
            rhs[i] = -c[i];
        }
        rhs[n..n + p].copy_from_slice(b);
        rhs[n + p..n + p + m].copy_from_slice(h);
        let bottom_diag = vec![-(1.0 + delta); m];
        let mut sol = vec![0.0; dim];
        kkt.solve_into(a, g, rho, -delta, &bottom_diag, &rhs, &mut sol);
        let xi = sol[0..n].to_vec();
        let y0 = sol[n..n + p].to_vec();
        let nu_tilde0 = sol[n + p..n + p + m].to_vec();
        (xi, y0, nu_tilde0)
    };

    if m == 0 {
        // 不等式・境界が 1 つもない: xi が既に ρ 正則化付きの定常条件を満たすので、
        // 双対残差の大きさだけで分類する。
        let aty = csr_mat_t_vec(a, n, &y0_init);
        let mut dual_res = c.clone();
        axpy(&mut dual_res, 1.0, &aty);
        if norm_inf(&dual_res) > NO_INEQ_DUAL_RES_TOL {
            return IpmResult { status: Status::Unbounded, x: None };
        }
        return IpmResult { status: Status::Optimal, x: Some(unscale_with_substitutions(&xi, &sc, &postsolve_log)) };
    }

    // スラック s と双対 z が厳密に正になるよう、暫定値をシフトして初期点を作る
    // (PIQP の初期化式)。
    let s_tilde0: Vec<f64> = nu_tilde0.iter().map(|v| -v).collect();
    let ds_tilde0 = (0.5f64 * -s_tilde0.iter().cloned().fold(f64::INFINITY, f64::min) * INIT_SHIFT_MULTIPLIER).max(0.0);
    let dnu_tilde0 = (0.5f64 * -nu_tilde0.iter().cloned().fold(f64::INFINITY, f64::min) * INIT_SHIFT_MULTIPLIER).max(0.0);
    let s_shift: Vec<f64> = s_tilde0.iter().map(|v| v + ds_tilde0).collect();
    let nu_shift: Vec<f64> = nu_tilde0.iter().map(|v| v + dnu_tilde0).collect();
    let cross = dot(&s_shift, &nu_shift);
    let sum_nu: f64 = nu_shift.iter().sum();
    let sum_s: f64 = s_shift.iter().sum();
    let d_s0 = ds_tilde0 + 0.5 * cross / sum_nu.max(INIT_DIV_GUARD);
    let d_nu0 = dnu_tilde0 + 0.5 * cross / sum_s.max(INIT_DIV_GUARD);

    let mut s: Vec<f64> = s_tilde0.iter().map(|v| (v + d_s0).max(INIT_POSITIVE_FLOOR)).collect();
    let mut z: Vec<f64> = nu_tilde0.iter().map(|v| (v + d_nu0).max(INIT_POSITIVE_FLOOR)).collect();
    let mut x: Vec<f64> = xi.clone();
    let mut y: Vec<f64> = y0_init;

    // 双対側の近接中心 (λ: 等式乗数用、ν: 不等式乗数用)
    let mut lambda = y.clone();
    let mut nu = z.clone();

    // 前回反復の主・双対残差の無限大ノルム (停滞判定用)
    let mut prev_primal_res = f64::INFINITY;
    let mut prev_dual_res = f64::INFINITY;
    // 改善のない反復が連続した回数
    let mut stall = 0usize;

    for _iter in 0..MAX_ITERS {
        csr_mat_vec_into(a, &x, &mut ws.ax);
        csr_mat_vec_into(g, &x, &mut ws.gx);

        // 停止判定 (PIQP 論文 式 13a-13c、P = 0)
        write_sub(&mut ws.primal_res[0..p], &ws.ax, b);
        for i in 0..m {
            ws.primal_res[p + i] = ws.gx[i] - h[i] + s[i];
        }
        // 主残差の無限大ノルム
        let primal_res_inf = norm_inf(&ws.primal_res);

        csr_mat_t_vec_into(a, &y, &mut ws.aty);
        csr_mat_t_vec_into(g, &z, &mut ws.gtz);
        ws.dual_res.copy_from_slice(c);
        axpy(&mut ws.dual_res, 1.0, &ws.aty);
        axpy(&mut ws.dual_res, 1.0, &ws.gtz);
        // 双対残差の無限大ノルム
        let dual_res_inf = norm_inf(&ws.dual_res);

        let cx = dot(c, &x);
        let by = dot(b, &y);
        let hz = dot(h, &z);
        // 双対ギャップ |c·x + b·y + h·z|
        let gap = (cx + by + hz).abs();

        // 主残差・双対残差・ギャップそれぞれの許容値
        let bnd_p = EPS_ABS
            + EPS_REL * norm_inf(&ws.ax).max(norm_inf(b)).max(norm_inf(&ws.gx)).max(norm_inf(h)).max(norm_inf(&s));
        let bnd_d = EPS_ABS + EPS_REL * norm_inf(&ws.aty).max(norm_inf(&ws.gtz)).max(norm_inf(c));
        let bnd_g = EPS_ABS + EPS_REL * cx.abs().max(by.abs()).max(hz.abs());

        if primal_res_inf <= bnd_p && dual_res_inf <= bnd_d && gap <= bnd_g {
            return IpmResult { status: Status::Optimal, x: Some(unscale_with_substitutions(&x, &sc, &postsolve_log)) };
        }

        // ρ と δ が両方とも下限に達しているか
        let at_floor = rho <= RHO_MIN * REG_FLOOR_SLACK && delta <= DELTA_MIN * REG_FLOOR_SLACK;
        if at_floor {
            if primal_infeasibility_certificate(a, g, n, b, h, &y, &z) {
                return IpmResult { status: Status::Infeasible, x: None };
            }
            if dual_infeasibility_certificate(a, g, c, &x) {
                return IpmResult { status: Status::Unbounded, x: None };
            }
        }

        if primal_res_inf >= prev_primal_res * STALL_PROGRESS_RATIO && dual_res_inf >= prev_dual_res * STALL_PROGRESS_RATIO && at_floor {
            stall += 1;
        } else {
            stall = 0;
        }
        prev_primal_res = primal_res_inf;
        prev_dual_res = dual_res_inf;
        if stall >= STALL_ITERS {
            // 正則化は下限、残差は改善せず、Farkas 証明も成立しなかった:
            // 相対的に小さいほうの残差から最善の推定で分類する。
            if dual_res_inf <= primal_res_inf {
                return IpmResult { status: Status::Infeasible, x: None };
            }
            return IpmResult { status: Status::Unbounded, x: None };
        }

        // ---- IP-PMM の Newton 反復 1 回 (Mehrotra の予測子・修正子) ----
        // r_x = -(c + ρ(x - ξ) + A^T y + G^T z)
        write_sub(&mut ws.tmp_n, &x, &xi);
        ws.r_x.copy_from_slice(c);
        axpy(&mut ws.r_x, rho, &ws.tmp_n);
        axpy(&mut ws.r_x, 1.0, &ws.aty);
        axpy(&mut ws.r_x, 1.0, &ws.gtz);
        for v in ws.r_x.iter_mut() {
            *v = -*v;
        }

        // r_y = -(A x + δ(λ - y) - b)
        write_sub(&mut ws.tmp_p, &lambda, &y);
        ws.r_y.copy_from_slice(&ws.ax);
        axpy(&mut ws.r_y, delta, &ws.tmp_p);
        axpy(&mut ws.r_y, -1.0, b);
        for v in ws.r_y.iter_mut() {
            *v = -*v;
        }

        // r_z = -(G x + δ(ν - z) - h + s)
        write_sub(&mut ws.tmp_m, &nu, &z);
        ws.r_z.copy_from_slice(&ws.gx);
        axpy(&mut ws.r_z, delta, &ws.tmp_m);
        axpy(&mut ws.r_z, -1.0, h);
        axpy(&mut ws.r_z, 1.0, &s);
        for v in ws.r_z.iter_mut() {
            *v = -*v;
        }

        // 1) 予測子 (アフィン) ステップ: r_s = -S z
        for i in 0..m {
            ws.r_s[i] = -s[i] * z[i];
        }
        newton_solve(
            &mut kkt, a, g, rho, delta, &s, &z, &ws.r_x, &ws.r_y, &ws.r_z, &ws.r_s,
            &mut ws.bottom_diag, &mut ws.kkt_rhs, &mut ws.kkt_sol_aff,
        );
        for i in 0..m {
            let dz_i = ws.kkt_sol_aff[n + p + i];
            ws.ds_aff[i] = (ws.r_s[i] - s[i] * dz_i) / z[i];
        }

        // アフィンステップの z 方向
        let dz_aff = &ws.kkt_sol_aff[n + p..n + p + m];
        let alpha_p_aff = fraction_to_boundary(&s, &ws.ds_aff);
        let alpha_d_aff = fraction_to_boundary(&z, dz_aff);

        // 2) 中心化パラメータ σ = (μ_aff / μ)^3
        let mu = dot(&s, &z) / m as f64;
        write_add_scaled(&mut ws.s_aff, &s, alpha_p_aff, &ws.ds_aff);
        write_add_scaled(&mut ws.z_aff, &z, alpha_d_aff, dz_aff);
        let mu_aff = dot(&ws.s_aff, &ws.z_aff) / m as f64;
        let sigma = (mu_aff / mu.max(GAP_DIV_GUARD)).clamp(0.0, 1.0).powi(3);

        // 3) 修正子 + 中心化の合成ステップ
        for i in 0..m {
            ws.r_s[i] = -s[i] * z[i] - ws.ds_aff[i] * ws.kkt_sol_aff[n + p + i] + sigma * mu;
        }
        newton_solve(
            &mut kkt, a, g, rho, delta, &s, &z, &ws.r_x, &ws.r_y, &ws.r_z, &ws.r_s,
            &mut ws.bottom_diag, &mut ws.kkt_rhs, &mut ws.kkt_sol,
        );
        for i in 0..m {
            let dz_i = ws.kkt_sol[n + p + i];
            ws.ds[i] = (ws.r_s[i] - s[i] * dz_i) / z[i];
        }

        let dx = &ws.kkt_sol[0..n];
        let dy = &ws.kkt_sol[n..n + p];
        let dz = &ws.kkt_sol[n + p..n + p + m];

        // 主側・双対側それぞれのステップ幅
        let alpha_p = fraction_to_boundary(&s, &ws.ds);
        let alpha_d = fraction_to_boundary(&z, dz);

        write_add_scaled(&mut ws.x_new, &x, alpha_p, dx);
        write_add_scaled(&mut ws.s_new, &s, alpha_p, &ws.ds);
        write_add_scaled(&mut ws.y_new, &y, alpha_d, dy);
        write_add_scaled(&mut ws.z_new, &z, alpha_d, dz);

        // ---- Algorithm 2: 正則化パラメータと近接中心の更新 ----
        // r: 相補性ギャップ s·z の相対減少率
        let gap_before = dot(&s, &z).max(GAP_DIV_GUARD);
        let gap_after = dot(&ws.s_new, &ws.z_new);
        let r = ((gap_before - gap_after) / gap_before).abs();

        csr_mat_vec_into(a, &ws.x_new, &mut ws.ax_new);
        csr_mat_vec_into(g, &ws.x_new, &mut ws.gx_new);
        write_sub(&mut ws.primal_res_new[0..p], &ws.ax_new, b);
        for i in 0..m {
            ws.primal_res_new[p + i] = ws.gx_new[i] - h[i] + ws.s_new[i];
        }
        let primal_res_inf_new = norm_inf(&ws.primal_res_new);

        // 主残差が十分減れば双対側の近接中心 (λ, ν) を更新して δ を大きく減らす
        if primal_res_inf_new <= RES_DECREASE_RATIO * primal_res_inf {
            lambda.copy_from_slice(&ws.y_new);
            nu.copy_from_slice(&ws.z_new);
            delta = (1.0 - r) * delta;
        } else {
            delta = (1.0 - r / SLOW_DECREASE_DIVISOR) * delta;
        }
        delta = delta.max(DELTA_MIN);

        csr_mat_t_vec_into(a, &ws.y_new, &mut ws.aty_new);
        csr_mat_t_vec_into(g, &ws.z_new, &mut ws.gtz_new);
        ws.dual_res_new.copy_from_slice(c);
        axpy(&mut ws.dual_res_new, 1.0, &ws.aty_new);
        axpy(&mut ws.dual_res_new, 1.0, &ws.gtz_new);
        let dual_res_inf_new = norm_inf(&ws.dual_res_new);

        // 双対残差が十分減れば主側の近接中心 ξ を更新して ρ を大きく減らす
        if dual_res_inf_new <= RES_DECREASE_RATIO * dual_res_inf {
            xi.copy_from_slice(&ws.x_new);
            rho = (1.0 - r) * rho;
        } else {
            rho = (1.0 - r / SLOW_DECREASE_DIVISOR) * rho;
        }
        rho = rho.max(RHO_MIN);

        std::mem::swap(&mut x, &mut ws.x_new);
        std::mem::swap(&mut s, &mut ws.s_new);
        std::mem::swap(&mut y, &mut ws.y_new);
        std::mem::swap(&mut z, &mut ws.z_new);
    }

    // 反復上限に達した (収束していれば上で return 済みなので Optimal は返さない)。
    // 正則化が下限に達したかどうかに関係なく Farkas 証明をもう一度試し、だめなら
    // 停滞時と同じ「残差の大小」で推定する。
    if primal_infeasibility_certificate(a, g, n, b, h, &y, &z) {
        return IpmResult { status: Status::Infeasible, x: None };
    }
    if dual_infeasibility_certificate(a, g, c, &x) {
        return IpmResult { status: Status::Unbounded, x: None };
    }
    let ax = csr_mat_vec(a, &x);
    let gx = csr_mat_vec(g, &x);
    let mut primal_res_vec = ax.iter().zip(b).map(|(x, y)| x - y).collect::<Vec<_>>();
    let mut ineq_res: Vec<f64> = (0..m).map(|i| gx[i] - h[i] + s[i]).collect();
    primal_res_vec.append(&mut ineq_res);
    let primal_res_inf = norm_inf(&primal_res_vec);
    let aty = csr_mat_t_vec(a, n, &y);
    let gtz = csr_mat_t_vec(g, n, &z);
    let mut dual_res_vec = c.clone();
    axpy(&mut dual_res_vec, 1.0, &aty);
    axpy(&mut dual_res_vec, 1.0, &gtz);
    let dual_res_inf = norm_inf(&dual_res_vec);
    if dual_res_inf <= primal_res_inf {
        IpmResult { status: Status::Infeasible, x: None }
    } else {
        IpmResult { status: Status::Unbounded, x: None }
    }
}

/// モデルの変数・目的関数・制約から標準形 (`qp::build`) を作って `solve` で解く。
/// `simplex::solve_lp_dual_with` の内点法版で、`solver::solve_lp` から
/// `RootSolver::Interior` のときに呼ばれる。最大化は目的係数の符号反転で最小化に直す。
pub fn solve_lp(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow]) -> IpmResult {
    // 最小化形の目的係数 (変数番号, 係数)
    let obj_coeffs_for_min: Vec<(usize, f64)> = match objective.sense {
        Sense::Minimize => objective.expr.coeffs.iter().map(|(&j, &c)| (j, c)).collect(),
        Sense::Maximize => objective.expr.coeffs.iter().map(|(&j, &c)| (j, -c)).collect(),
    };
    let built = qp::build(variables, &obj_coeffs_for_min, constraints);
    solve(&built)
}
