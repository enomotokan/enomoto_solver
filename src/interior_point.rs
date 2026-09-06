//! Interior-Point Proximal Method of Multipliers (IP-PMM) with Mehrotra
//! predictor-corrector updates — a from-scratch reimplementation of the
//! algorithm behind PIQP (Schwan, Jiang, Kuhn, Jones, "PIQP: A Proximal
//! Interior-Point Quadratic Programming Solver", CDC 2023), specialized to
//! P = 0 (linear programs). The per-iteration KKT (Newton) system is
//! symmetric quasi-definite after eliminating the slack step (their
//! Remark 1); it is assembled and factored entirely through faer's sparse
//! machinery (see `spkkt.rs`) — `A`/`G` stay as `SparseRowMat` (CSR) end to
//! end, no dense conversion.
//!
//! Preprocessing: the problem is put through the same shared, *extended*
//! presolve pipeline (`presolve::run_extended` — Ruiz equilibration,
//! redundant-equality removal, then rounds of propagate/dualfix/row-
//! singleton/doubleton/colsingleton) `simplex.rs` uses, rather than the
//! plainer elimination-free `presolve::run` this module used before —
//! every technique ported into that shared pipeline (including ones that
//! eliminate a variable's slot entirely) now benefits this engine too, not
//! just the simplex one; see `unscale_with_substitutions`'s own docs for
//! how an eliminated variable's true value is recovered afterward. This
//! runs once up front, before the interior-point loop starts; the KKT
//! matrix's AMD ordering + symbolic factorization are computed once and
//! reused across every iteration's KKT solve (`kkt.rs`) — only the (much
//! cheaper) numeric factorization is redone each iteration.
//!
//! **Allocation**: every buffer the Newton loop touches (residuals,
//! directions, trial iterates, the KKT right-hand-side/solution) lives in
//! `Workspace`, sized once in `Workspace::new` before the loop starts, and
//! is overwritten in place on every iteration — the loop body performs no
//! `Vec` allocation. Only the (rare) Farkas-certificate checks and the
//! final `Vec<f64>` handed back to the caller still allocate.
//!
//! **Reachable, not default**: `solver::solve_lp` dispatches here only when
//! `Model.solve(root_solver="interior")` is requested explicitly — the
//! default (`root_solver=None`/`"simplex"`) goes straight to
//! `simplex::solve_lp_dual` instead (see `solver.rs`'s own docs). Kept
//! fully reachable rather than deleted, both as a fallback and so the two
//! independent implementations can be run against the same input and
//! cross-checked directly (`simplex.rs`'s own
//! `*_matches_independent_ipm_solver` tests do exactly this).

pub mod qp;
pub mod kkt;

use rayon::prelude::*;

use self::kkt::{mat_t_vec, mat_t_vec_into, mat_vec, mat_vec_into, Csr, SparseKkt};
use self::qp::QpStd;
use crate::presolve::{self, scaling};
use crate::types::{ConstraintRow, Objective, Sense, Status, VariableData};

const TAU: f64 = 0.995;
const RHO_MIN: f64 = 1e-10;
const DELTA_MIN: f64 = 1e-10;
const RHO0: f64 = 1e-1;
const DELTA0: f64 = 1e-1;
const EPS_ABS: f64 = 1e-8;
const EPS_REL: f64 = 1e-8;
const MAX_ITERS: usize = 100;
const STALL_ITERS: usize = 8;
/// Number of times to redo constraint propagation (bound strengthening +
/// redundant/infeasible row detection) over the inequality rows after
/// removing redundant equality rows (see `propagate.rs`). The underlying
/// technique is itself iterative until a fixpoint; two rounds here mirror
/// how Gurobi's presolve caps propagation passes per presolve round rather
/// than iterating to convergence.
const PROPAGATION_PASSES: usize = 2;
/// Upper bound on how many times `presolve::run_extended` cycles through
/// propagate → dualfix → row-singleton → doubleton → colsingleton —
/// mirrors `simplex.rs`'s own `PRESOLVE_ROUNDS` (see that constant's own
/// docs for why this is a cap, not a fixed count: `run_extended` itself
/// stops early once a round converges).
const PRESOLVE_ROUNDS: usize = 10;
/// Upper bound on how many times each outer `PRESOLVE_ROUNDS` pass itself
/// cycles through row-singleton <-> colsingleton before `propagate`/
/// `dualfix` run again — mirrors `simplex.rs`'s own
/// `ROWSINGLETON_COLSINGLETON_INNER_ROUNDS` (see that constant's own docs
/// for why this inner pair can have more to find after its own first
/// pass, why `doubleton` isn't part of this inner repetition, and for the
/// fixpoint check that stops it short of this cap).
const ROWSINGLETON_COLSINGLETON_INNER_ROUNDS: usize = 1;
/// Upper bound on how many *outer* `PRESOLVE_ROUNDS` passes run
/// `doubleton` at all — mirrors `simplex.rs`'s own `DOUBLETON_ROUNDS` (see
/// that constant's own docs for the measurement that settled on `2`).
const DOUBLETON_ROUNDS: usize = 2;

pub struct IpmResult {
    pub status: Status,
    pub x: Option<Vec<f64>>,
}

// Every elementwise/reduction helper below is embarrassingly parallel
// (disjoint per-index reads/writes, or an associative reduction) and
// touches no heap allocation of its own, so parallelizing them via rayon
// doesn't conflict with the Newton loop's own no-allocation discipline
// (module docs). For the problem sizes this crate targets these vectors
// are short enough that rayon's dispatch overhead can outweigh the win —
// real payoff shows up as `n`/`p`/`m` grow — but the operations are
// correct and safe to parallelize at any size, unlike e.g. a triangular
// solve's inherent step-to-step dependency.

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.par_iter().zip(b.par_iter()).map(|(x, y)| x * y).sum()
}

fn norm_inf(v: &[f64]) -> f64 {
    v.par_iter().map(|x| x.abs()).reduce(|| 0.0_f64, f64::max)
}

fn axpy(out: &mut [f64], a: f64, x: &[f64]) {
    out.par_iter_mut().zip(x.par_iter()).for_each(|(o, &xi)| *o += a * xi);
}

/// `out[i] = a[i] - b[i]`.
fn write_sub(out: &mut [f64], a: &[f64], b: &[f64]) {
    out.par_iter_mut().zip(a.par_iter()).zip(b.par_iter()).for_each(|((o, &ai), &bi)| *o = ai - bi);
}

/// `out[i] = a[i] + alpha * d[i]`.
fn write_add_scaled(out: &mut [f64], a: &[f64], alpha: f64, d: &[f64]) {
    out.par_iter_mut().zip(a.par_iter()).zip(d.par_iter()).for_each(|((o, &ai), &di)| *o = ai + alpha * di);
}

/// Farkas certificate of primal infeasibility: (y, z) with
/// A^T y + G^T z ~= 0 and b.y + h.z > 0 proves { Ax=b, Gx<=h } is empty.
/// Only evaluated when regularization has hit its floor (rare relative to
/// the main loop), so it's allowed to allocate.
fn primal_infeasibility_certificate(a: &Csr, g: &Csr, n: usize, b: &[f64], h: &[f64], y: &[f64], z: &[f64]) -> bool {
    let scale = norm_inf(y).max(norm_inf(z));
    if scale < 1e-8 {
        return false;
    }
    let yhat: Vec<f64> = y.iter().map(|v| v / scale).collect();
    let zhat: Vec<f64> = z.iter().map(|v| v / scale).collect();
    let mut stat = mat_t_vec(a, n, &yhat);
    axpy(&mut stat, 1.0, &mat_t_vec(g, n, &zhat));
    norm_inf(&stat) < 1e-5 && dot(b, &yhat) + dot(h, &zhat) > 1e-5
}

/// Farkas certificate of dual infeasibility (primal unboundedness): x with
/// Ax ~= 0, Gx <= 0 and c.x < 0 is an unbounded, cost-decreasing ray.
fn dual_infeasibility_certificate(a: &Csr, g: &Csr, c: &[f64], x: &[f64]) -> bool {
    let scale = norm_inf(x);
    if scale < 1e-8 {
        return false;
    }
    let xhat: Vec<f64> = x.iter().map(|v| v / scale).collect();
    let eq_ok = norm_inf(&mat_vec(a, &xhat)) < 1e-5;
    let ineq_ok = mat_vec(g, &xhat).iter().all(|&v| v <= 1e-5);
    eq_ok && ineq_ok && dot(c, &xhat) < -1e-5
}

fn fraction_to_boundary(v: &[f64], dv: &[f64]) -> f64 {
    let mut alpha = 1.0_f64;
    for (&vi, &dvi) in v.iter().zip(dv) {
        if dvi < 0.0 {
            alpha = alpha.min(TAU * vi / (-dvi));
        }
    }
    alpha.clamp(0.0, 1.0)
}

/// Solves the eliminated KKT system for (dx, dy, dz) into `sol_out`
/// (length n+p+m), using `bottom_diag`/`rhs` as scratch. No allocation.
#[allow(clippy::too_many_arguments)]
fn newton_solve(
    kkt: &mut SparseKkt,
    a: &Csr,
    g: &Csr,
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

    for i in 0..m {
        bottom_diag[i] = -(s[i] / z[i] + delta);
    }

    rhs[0..n].copy_from_slice(r_x);
    rhs[n..n + p].copy_from_slice(r_y);
    for i in 0..m {
        rhs[n + p + i] = r_z[i] - r_s[i] / z[i]; // r_bar_z
    }

    kkt.solve_into(a, g, rho, -delta, bottom_diag, rhs, sol_out);
}

/// Every buffer the Newton loop needs, sized once from (n, p, m) and
/// reused in place for every iteration.
struct Workspace {
    // length n
    aty: Vec<f64>,
    gtz: Vec<f64>,
    dual_res: Vec<f64>,
    r_x: Vec<f64>,
    x_new: Vec<f64>,
    aty_new: Vec<f64>,
    gtz_new: Vec<f64>,
    dual_res_new: Vec<f64>,
    tmp_n: Vec<f64>,
    // length p
    ax: Vec<f64>,
    r_y: Vec<f64>,
    y_new: Vec<f64>,
    ax_new: Vec<f64>,
    tmp_p: Vec<f64>,
    // length m
    gx: Vec<f64>,
    r_z: Vec<f64>,
    r_s: Vec<f64>,
    dsa: Vec<f64>,
    ds: Vec<f64>,
    s_aff: Vec<f64>,
    z_aff: Vec<f64>,
    s_new: Vec<f64>,
    z_new: Vec<f64>,
    gx_new: Vec<f64>,
    bottom_diag: Vec<f64>,
    tmp_m: Vec<f64>,
    // length p + m
    primal_res: Vec<f64>,
    primal_res_new: Vec<f64>,
    // length n + p + m
    kkt_rhs: Vec<f64>,
    kkt_sol_aff: Vec<f64>,
    kkt_sol: Vec<f64>,
}

impl Workspace {
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
            dsa: vec![0.0; m],
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

/// Fills in every `doubleton`/`colsingleton`-eliminated variable's true
/// value (in **reverse** discovery order — see
/// `presolve::ExtendedPresolveResult`'s own docs for why) before the final
/// `scaling::unscale_x` — the interior-point counterpart of `simplex.rs`'s
/// `unscale_result`, needed now that `solve` below runs the same
/// `presolve::run_extended` pipeline simplex.rs does instead of the
/// elimination-free `presolve::run`. `x` is in **scaled** space at every
/// call site (this module's Newton loop, like `simplex.rs`'s, never
/// unscales mid-solve), which is exactly the space each `Substitution`'s
/// own `terms`/`rhs`/`coeff` were recorded in.
fn unscale_with_substitutions(x: &[f64], sc: &scaling::Scaling, substitutions: &[presolve::colsingleton::Substitution]) -> Vec<f64> {
    let mut x = x.to_vec();
    for sub in substitutions.iter().rev() {
        x[sub.var] = sub.value(&x);
    }
    scaling::unscale_x(sc, &x)
}

pub fn solve(qp: &QpStd) -> IpmResult {
    let n = qp.n;

    if n == 0 {
        return IpmResult { status: Status::Optimal, x: Some(vec![]) };
    }

    // One-time shared presolve pass (`crate::presolve` — Ruiz scaling,
    // redundant-equality removal, then the extended propagate/dualfix/
    // row-singleton/doubleton/colsingleton pipeline `simplex.rs` already
    // uses — see `unscale_with_substitutions`'s own docs for why this
    // module now shares it too instead of the plainer `presolve::run`).
    // Everything below operates on the scaled/reduced problem; `x` is
    // mapped back to original-variable space at every return site via
    // `unscale_with_substitutions`.
    let pre = presolve::run_extended(n, &qp.a, &qp.b, &qp.g, &qp.h, &qp.c, 10, PROPAGATION_PASSES, PRESOLVE_ROUNDS, ROWSINGLETON_COLSINGLETON_INNER_ROUNDS, DOUBLETON_ROUNDS);
    if pre.infeasible {
        return IpmResult { status: Status::Infeasible, x: None };
    }
    let sc = pre.scaling;
    let a = pre.a;
    let b = pre.b;
    let g = pre.g;
    let h = pre.h;
    let c = pre.c;
    let substitutions = pre.substitutions;
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

    // ---- Initialization (PIQP paper, Section IV.A) ----
    let mut rho = RHO0;
    let mut delta = DELTA0;

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
        // No inequalities/bounds at all: xi already solves the
        // rho-regularized stationarity system; classify via dual residual.
        let aty = mat_t_vec(a, n, &y0_init);
        let mut dual_res = c.clone();
        axpy(&mut dual_res, 1.0, &aty);
        if norm_inf(&dual_res) > 1e-4 {
            return IpmResult { status: Status::Unbounded, x: None };
        }
        return IpmResult { status: Status::Optimal, x: Some(unscale_with_substitutions(&xi, &sc, &substitutions)) };
    }

    let s_tilde0: Vec<f64> = nu_tilde0.iter().map(|v| -v).collect();
    let ds_tilde0 = (0.5f64 * -s_tilde0.iter().cloned().fold(f64::INFINITY, f64::min) * 3.0).max(0.0);
    let dnu_tilde0 = (0.5f64 * -nu_tilde0.iter().cloned().fold(f64::INFINITY, f64::min) * 3.0).max(0.0);
    let s_shift: Vec<f64> = s_tilde0.iter().map(|v| v + ds_tilde0).collect();
    let nu_shift: Vec<f64> = nu_tilde0.iter().map(|v| v + dnu_tilde0).collect();
    let cross = dot(&s_shift, &nu_shift);
    let sum_nu: f64 = nu_shift.iter().sum();
    let sum_s: f64 = s_shift.iter().sum();
    let d_s0 = ds_tilde0 + 0.5 * cross / sum_nu.max(1e-12);
    let d_nu0 = dnu_tilde0 + 0.5 * cross / sum_s.max(1e-12);

    let mut s: Vec<f64> = s_tilde0.iter().map(|v| (v + d_s0).max(1e-6)).collect();
    let mut z: Vec<f64> = nu_tilde0.iter().map(|v| (v + d_nu0).max(1e-6)).collect();
    let mut x: Vec<f64> = xi.clone();
    let mut y: Vec<f64> = y0_init;

    let mut lambda = y.clone();
    let mut nu = z.clone();

    let mut p_res = f64::INFINITY;
    let mut d_res = f64::INFINITY;
    let mut stall = 0usize;

    for _iter in 0..MAX_ITERS {
        mat_vec_into(a, &x, &mut ws.ax);
        mat_vec_into(g, &x, &mut ws.gx);

        // Termination check (PIQP paper, eq. 13a-13c), P = 0.
        write_sub(&mut ws.primal_res[0..p], &ws.ax, b);
        for i in 0..m {
            ws.primal_res[p + i] = ws.gx[i] - h[i] + s[i];
        }
        let pk = norm_inf(&ws.primal_res);

        mat_t_vec_into(a, &y, &mut ws.aty);
        mat_t_vec_into(g, &z, &mut ws.gtz);
        ws.dual_res.copy_from_slice(c);
        axpy(&mut ws.dual_res, 1.0, &ws.aty);
        axpy(&mut ws.dual_res, 1.0, &ws.gtz);
        let dk = norm_inf(&ws.dual_res);

        let cx = dot(c, &x);
        let by = dot(b, &y);
        let hz = dot(h, &z);
        let gap = (cx + by + hz).abs();

        let bnd_p = EPS_ABS
            + EPS_REL * norm_inf(&ws.ax).max(norm_inf(b)).max(norm_inf(&ws.gx)).max(norm_inf(h)).max(norm_inf(&s));
        let bnd_d = EPS_ABS + EPS_REL * norm_inf(&ws.aty).max(norm_inf(&ws.gtz)).max(norm_inf(c));
        let bnd_g = EPS_ABS + EPS_REL * cx.abs().max(by.abs()).max(hz.abs());

        if pk <= bnd_p && dk <= bnd_d && gap <= bnd_g {
            return IpmResult { status: Status::Optimal, x: Some(unscale_with_substitutions(&x, &sc, &substitutions)) };
        }

        let at_floor = rho <= RHO_MIN * 1.001 && delta <= DELTA_MIN * 1.001;
        if at_floor {
            if primal_infeasibility_certificate(a, g, n, b, h, &y, &z) {
                return IpmResult { status: Status::Infeasible, x: None };
            }
            if dual_infeasibility_certificate(a, g, c, &x) {
                return IpmResult { status: Status::Unbounded, x: None };
            }
        }

        if pk >= p_res * 0.999999 && dk >= d_res * 0.999999 && at_floor {
            stall += 1;
        } else {
            stall = 0;
        }
        p_res = pk;
        d_res = dk;
        if stall >= STALL_ITERS {
            // Regularization is at its floor, residuals have stopped
            // improving, and neither Farkas certificate above triggered
            // cleanly: fall back to the residual that is relatively
            // smaller as a best-effort classification.
            if dk <= pk {
                return IpmResult { status: Status::Infeasible, x: None };
            }
            return IpmResult { status: Status::Unbounded, x: None };
        }

        // ---- one IP-PMM Newton iteration (Mehrotra predictor-corrector) ----
        write_sub(&mut ws.tmp_n, &x, &xi);
        ws.r_x.copy_from_slice(c);
        axpy(&mut ws.r_x, rho, &ws.tmp_n);
        axpy(&mut ws.r_x, 1.0, &ws.aty);
        axpy(&mut ws.r_x, 1.0, &ws.gtz);
        for v in ws.r_x.iter_mut() {
            *v = -*v;
        }

        write_sub(&mut ws.tmp_p, &lambda, &y);
        ws.r_y.copy_from_slice(&ws.ax);
        axpy(&mut ws.r_y, delta, &ws.tmp_p);
        axpy(&mut ws.r_y, -1.0, b);
        for v in ws.r_y.iter_mut() {
            *v = -*v;
        }

        write_sub(&mut ws.tmp_m, &nu, &z);
        ws.r_z.copy_from_slice(&ws.gx);
        axpy(&mut ws.r_z, delta, &ws.tmp_m);
        axpy(&mut ws.r_z, -1.0, h);
        axpy(&mut ws.r_z, 1.0, &s);
        for v in ws.r_z.iter_mut() {
            *v = -*v;
        }

        // 1) Prediction (affine): r_s = -S z
        for i in 0..m {
            ws.r_s[i] = -s[i] * z[i];
        }
        newton_solve(
            &mut kkt, a, g, rho, delta, &s, &z, &ws.r_x, &ws.r_y, &ws.r_z, &ws.r_s,
            &mut ws.bottom_diag, &mut ws.kkt_rhs, &mut ws.kkt_sol_aff,
        );
        for i in 0..m {
            let dz_i = ws.kkt_sol_aff[n + p + i];
            ws.dsa[i] = (ws.r_s[i] - s[i] * dz_i) / z[i];
        }

        let dza = &ws.kkt_sol_aff[n + p..n + p + m];
        let alpha_p_aff = fraction_to_boundary(&s, &ws.dsa);
        let alpha_d_aff = fraction_to_boundary(&z, dza);

        let mu = dot(&s, &z) / m as f64;
        write_add_scaled(&mut ws.s_aff, &s, alpha_p_aff, &ws.dsa);
        write_add_scaled(&mut ws.z_aff, &z, alpha_d_aff, dza);
        let mu_aff = dot(&ws.s_aff, &ws.z_aff) / m as f64;
        let sigma = (mu_aff / mu.max(1e-16)).clamp(0.0, 1.0).powi(3);

        // 3) Combined correction + centering
        for i in 0..m {
            ws.r_s[i] = -s[i] * z[i] - ws.dsa[i] * ws.kkt_sol_aff[n + p + i] + sigma * mu;
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

        let alpha_p = fraction_to_boundary(&s, &ws.ds);
        let alpha_d = fraction_to_boundary(&z, dz);

        write_add_scaled(&mut ws.x_new, &x, alpha_p, dx);
        write_add_scaled(&mut ws.s_new, &s, alpha_p, &ws.ds);
        write_add_scaled(&mut ws.y_new, &y, alpha_d, dy);
        write_add_scaled(&mut ws.z_new, &z, alpha_d, dz);

        // ---- Algorithm 2: penalty parameter / proximal estimate updates ----
        let gap_before = dot(&s, &z).max(1e-16);
        let gap_after = dot(&ws.s_new, &ws.z_new);
        let r = ((gap_before - gap_after) / gap_before).abs();

        mat_vec_into(a, &ws.x_new, &mut ws.ax_new);
        mat_vec_into(g, &ws.x_new, &mut ws.gx_new);
        write_sub(&mut ws.primal_res_new[0..p], &ws.ax_new, b);
        for i in 0..m {
            ws.primal_res_new[p + i] = ws.gx_new[i] - h[i] + ws.s_new[i];
        }
        let pk_new = norm_inf(&ws.primal_res_new);

        if pk_new <= 0.95 * pk {
            lambda.copy_from_slice(&ws.y_new);
            nu.copy_from_slice(&ws.z_new);
            delta = (1.0 - r) * delta;
        } else {
            delta = (1.0 - r / 3.0) * delta;
        }
        delta = delta.max(DELTA_MIN);

        mat_t_vec_into(a, &ws.y_new, &mut ws.aty_new);
        mat_t_vec_into(g, &ws.z_new, &mut ws.gtz_new);
        ws.dual_res_new.copy_from_slice(c);
        axpy(&mut ws.dual_res_new, 1.0, &ws.aty_new);
        axpy(&mut ws.dual_res_new, 1.0, &ws.gtz_new);
        let dk_new = norm_inf(&ws.dual_res_new);

        if dk_new <= 0.95 * dk {
            xi.copy_from_slice(&ws.x_new);
            rho = (1.0 - r) * rho;
        } else {
            rho = (1.0 - r / 3.0) * rho;
        }
        rho = rho.max(RHO_MIN);

        std::mem::swap(&mut x, &mut ws.x_new);
        std::mem::swap(&mut s, &mut ws.s_new);
        std::mem::swap(&mut y, &mut ws.y_new);
        std::mem::swap(&mut z, &mut ws.z_new);
    }

    // Ran out of iterations without meeting the optimality criteria above
    // (which already handles the converged case) — never report Optimal
    // here. Try the Farkas certificates once more regardless of whether
    // rho/delta reached their floor yet, then fall back to the same
    // pk-vs-dk heuristic used when a stall is detected mid-loop.
    if primal_infeasibility_certificate(a, g, n, b, h, &y, &z) {
        return IpmResult { status: Status::Infeasible, x: None };
    }
    if dual_infeasibility_certificate(a, g, c, &x) {
        return IpmResult { status: Status::Unbounded, x: None };
    }
    let ax = mat_vec(a, &x);
    let gx = mat_vec(g, &x);
    let mut primal_res_vec = ax.iter().zip(b).map(|(x, y)| x - y).collect::<Vec<_>>();
    let mut ineq_res: Vec<f64> = (0..m).map(|i| gx[i] - h[i] + s[i]).collect();
    primal_res_vec.append(&mut ineq_res);
    let pk = norm_inf(&primal_res_vec);
    let aty = mat_t_vec(a, n, &y);
    let gtz = mat_t_vec(g, n, &z);
    let mut dual_res_vec = c.clone();
    axpy(&mut dual_res_vec, 1.0, &aty);
    axpy(&mut dual_res_vec, 1.0, &gtz);
    let dk = norm_inf(&dual_res_vec);
    if dk <= pk {
        IpmResult { status: Status::Infeasible, x: None }
    } else {
        IpmResult { status: Status::Unbounded, x: None }
    }
}

/// Builds the QP standard form (`qp::build`) directly from the model's
/// variables/objective/constraints and solves it — the interior-point
/// counterpart of `simplex::solve_lp`/`solve_lp_dual`, letting
/// `solver::solve_lp` dispatch to either engine on the exact same inputs
/// (`types::RootSolver`). Kept alongside `solve` rather than only reachable
/// through `simplex.rs`'s test helper now that `Model.solve`'s
/// `root_solver` argument can select this path directly.
pub fn solve_lp(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow]) -> IpmResult {
    let obj_coeffs_for_min: Vec<(usize, f64)> = match objective.sense {
        Sense::Minimize => objective.expr.coeffs.iter().map(|(&j, &c)| (j, c)).collect(),
        Sense::Maximize => objective.expr.coeffs.iter().map(|(&j, &c)| (j, -c)).collect(),
    };
    let built = qp::build(variables, &obj_coeffs_for_min, constraints);
    solve(&built)
}
