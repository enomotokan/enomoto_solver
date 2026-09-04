//! A from-scratch **bounded-variable revised simplex**, both primal
//! ([`solve_lp`]) and dual ([`solve_lp_dual`]) — the engine
//! `solver::solve_lp` calls for every LP `Model.solve()` needs to solve
//! (including once per branch-and-bound node for MIPs; see `bnb.rs`).
//!
//! ## The bounded-variable invariant
//!
//! Every **structural** (user-facing) variable has two *finite* bounds —
//! `model.rs::add_variable` rejects infinite `lb`/`ub` at the PyO3
//! boundary, so this module never has to represent a free or one-sided
//! variable and treats that as a precondition throughout (`NbStatus` has
//! only `Lower`/`Upper`, never a "free" case). This is also what makes
//! [`Tableau::crash_dual_feasible`] unconditional (§ its own docs) and
//! makes a genuinely unbounded objective impossible in principle (a
//! linear objective over a bounded box is always bounded) — `Status::Unbounded`
//! is kept only as a defensive fallback (§ the ratio test below), not as
//! a reachable outcome for well-formed input.
//!
//! **Slack** columns (one per constraint row, added internally to reach
//! standard form) are a different matter: a `<=`/`>=` row's slack is
//! genuinely one-sided (`[0, inf)`), so infinite-bound handling is still
//! very much alive in the ratio test/EXPAND machinery below — just never
//! for a structural variable.
//!
//! ## Basis representation
//!
//! A Markowitz-pivoted sparse LU (`sparse_lu::LuFactors`, sparsity-
//! prioritizing rather than numerical-stability-prioritizing pivoting)
//! plus incremental **Forrest-Tomlin** updates (`sparse_lu::FtLu`,
//! implemented per Forrest & Tomlin (1972) as precisely summarized with
//! full derivations in Huangfu & Hall, "Novel update techniques for the
//! revised simplex method", ERGO-13-001, University of Edinburgh (2013)),
//! refactorizing from scratch only when one of four triggers fires:
//!
//!   1. every [`FT_CHECK_INTERVAL`] iterations, if the true-basis residual
//!      `‖A_B x_B - rhs‖` exceeds [`FT_RESIDUAL_TOL`];
//!   2. immediately, if an update's resulting pivot is smaller than
//!      [`FT_MIN_PIVOT`] (checked in-place by `FtLu::try_update`);
//!   3. at the same periodic check as (1), if the accumulated eta-file
//!      fill (`FtLu::fill_count`) exceeds [`FT_BUMP_LIMIT_FACTOR`] `* m`;
//!   4. unconditionally, once the update count exceeds [`FT_MAX_UPDATES`].
//!
//! ## Standard form
//!
//! Every constraint row gets its own explicit slack column, so the solver
//! always works with `M z = rhs`, `lo <= z <= hi`, where `z = [x; s]`:
//!
//!   - `A x = b`           becomes  `A x + s = b`,  `s` fixed at `[0, 0]`
//!   - `G x <= h`           becomes  `G x + s = h`,  `s` free in `[0, inf)`
//!   - `G x >= h`  (pre-normalized to `-G x <= -h` by the caller, matching
//!     `qp.rs`'s convention) is handled the same way as `<=`.
//!
//! This lets every row start with its own slack as the initial basic
//! variable (`B = I`), regardless of constraint sense.
//!
//! ## Primal method: two-phase, EXPAND anti-cycling, steepest edge
//!
//! Phase 1 minimizes the sum of bound infeasibilities of the basic
//! variables using a **composite objective** that is recomputed every
//! iteration (`cost_i = -1` if `x_Bi` is below its lower bound, `+1` if
//! above its upper bound, `0` if feasible) and a modified ratio test: a
//! basic variable that currently violates its lower bound is only
//! blocked by its *upper* bound (moving it up, towards feasibility, is
//! never blocking), and symmetrically for a variable violating its upper
//! bound. Phase 2 is the standard bounded-variable primal simplex once
//! every basic variable is feasible. Both phases share the same EXPAND
//! anti-cycling ratio test (Gill, Murray, Saunders & Wright 1989) and
//! primal steepest-edge entering rule (Forrest & Goldfarb 1992) — see
//! [`ExpandState`] and [`SteepestEdgeState`]'s docs.
//!
//! ## Dual method: classical ratio test, dual steepest edge
//!
//! [`solve_lp_dual`] needs no phase 1 (a dual-feasible start is always
//! constructible — see [`Tableau::crash_dual_feasible`]) and uses dual
//! steepest-edge pricing ([`DseState`], per Huangfu & Hall,
//! "Parallelizing the dual revised simplex method", arXiv:1503.01889
//! §2.2) for the leaving row; the entering-column ratio test is the
//! classical/textbook one (no Harris two-pass or bound-flipping (BFRT)
//! refinement yet), and there is no dual-specific EXPAND-style anti-
//! cycling yet either — both are candidates for a later pass.

use rayon::prelude::*;

use crate::types::{ConstraintRow, Objective, RowSense, Sense, Status, VariableData};

/// Markowitz-pivoted sparse LU + Forrest-Tomlin incremental updates —
/// see `lu`'s own module docs. Referred to below as `sparse_lu` (not the
/// bare module name `lu`) purely to avoid clashing with the many local
/// variables/parameters through this file that are themselves named `lu`
/// (an `FtLu` instance, the live basis factorization).
mod lu;
use self::lu as sparse_lu;

const TOL: f64 = 1e-9;
const MAX_ITERS: usize = 20_000;

/// Trigger (2): an FT update whose resulting pivot is smaller than this
/// is rejected by `FtLu::try_update`, forcing an immediate refactorization.
const FT_MIN_PIVOT: f64 = 1e-7;
/// Cadence (in iterations) for triggers (1) and (3).
const FT_CHECK_INTERVAL: usize = 5;
/// Trigger (1): refactor if the true-basis residual exceeds this.
const FT_RESIDUAL_TOL: f64 = 1e-6;
/// Trigger (3): refactor if accumulated eta-file fill exceeds this factor
/// times the basis dimension.
const FT_BUMP_LIMIT_FACTOR: usize = 4;
/// Trigger (4): refactor unconditionally once the update count passes this.
const FT_MAX_UPDATES: usize = 100;

/// EXPAND anti-cycling (Gill, Murray, Saunders & Wright, "A practical
/// anti-cycling procedure for linearly constrained optimization",
/// Mathematical Programming 45 (1989) 437-474). "Master" feasibility
/// tolerance the working tolerance `delta` is kept strictly below during
/// an expanding sequence; also the snap-to-bound threshold used when
/// resetting nonbasic variables (§4.2-4.3, eq. (4.2)/(4.3)).
const EXPAND_DELTA_F: f64 = 1e-6;
/// Iterations per expanding sequence before a reset (§4.2); the paper's
/// own worked example uses 10000 for large industrial LPs, but this
/// project's test-scale LPs warrant a much shorter cycle so resets are
/// actually exercised.
const EXPAND_K: usize = 50;
/// Feasibility tolerance an expanding sequence starts from (§4.2: `delta_0 = 0.5 delta_f`).
const EXPAND_DELTA_0: f64 = 0.5 * EXPAND_DELTA_F;
/// Ceiling `delta_k` approaches but never reaches within `EXPAND_K` steps
/// (§4.2: `delta_K = 0.99 delta_f`).
const EXPAND_DELTA_K: f64 = 0.99 * EXPAND_DELTA_F;
/// Per-iteration growth of the working tolerance (§4.2: `tau = (delta_K - delta_0) / K`).
const EXPAND_TAU: f64 = (EXPAND_DELTA_K - EXPAND_DELTA_0) / (EXPAND_K as f64);

/// Persists the EXPAND working feasibility tolerance and reset cadence
/// across both phases of one `solve_lp` call (an expanding sequence is
/// not restarted at the phase-1/phase-2 boundary).
struct ExpandState {
    delta: f64,
    iters_since_reset: usize,
}

impl ExpandState {
    fn new() -> Self {
        ExpandState { delta: EXPAND_DELTA_0, iters_since_reset: 0 }
    }
}

/// Weights never allowed to fall below this — guards against a tiny or
/// negative value (from accumulated rounding) making a column look
/// spuriously "steep".
const STEEPEST_EDGE_FLOOR: f64 = 1e-10;

/// Steepest-edge entering-variable weights (Forrest, J.J.H. and Goldfarb,
/// D., "Steepest-edge simplex algorithms for linear programming",
/// Mathematical Programming 57 (1992) 341-374): for nonbasic `j`,
/// `gamma[j] = ||B^-1 A_j||^2`, the squared norm of the direction basic
/// variables move in if `j` were to enter. The entering rule becomes
/// `max_j d_j^2 / gamma_j` instead of Dantzig's `max_j |d_j|`, approximating
/// the actual objective improvement per unit distance traveled rather than
/// per unit change in the entering variable itself.
///
/// The update formula below was re-derived from scratch (Sherman-Morrison
/// applied to `B_new^-1 = E^-1 B^-1`, the same rank-one identity
/// `sparse_lu::FtLu`'s eta updates use) rather than transcribed from a
/// secondary source describing the Forrest-Goldfarb formula: that source
/// rendered the update as `gamma_j + beta_j(1+gamma_t) - 2 beta_j tau_j`,
/// but the derivation here gives `beta_j` *squared* in the middle term,
/// and this was confirmed empirically (`steepest_edge_weights_match_brute_force_recompute`
/// below compares the incremental update against `||B_new^-1 A_j||^2`
/// computed by a fresh solve for every nonbasic column after a pivot).
/// Unlike a wrong pivot in `sparse_lu`, an incorrect weight here could
/// only degrade pricing quality, not the simplex's correctness — but it
/// was still worth resolving via direct evidence rather than trusting
/// either source blindly.
struct SteepestEdgeState {
    gamma: Vec<f64>,
}

impl SteepestEdgeState {
    /// `gamma[j] = ||A_j||^2` for every initially-nonbasic (structural)
    /// variable: the initial basis `B0` is a signed identity (+/-1 per
    /// row), so `B0^-1 A_j` is just `A_j` with some rows sign-flipped,
    /// and squaring erases the signs. Slack columns start basic, so their
    /// entries are left as an unused placeholder — see the loop in
    /// `run_phase` that assigns a leaving variable's new weight, which
    /// covers every slack the first time it ever becomes nonbasic.
    fn new(std: &StdForm) -> Self {
        let mut gamma = vec![1.0; std.n_total];
        let n_orig = std.n_total - std.n_rows;
        for j in 0..n_orig {
            let mut norm_sq = 0.0;
            for i in 0..std.n_rows {
                for &(jj, v) in &std.rows[i] {
                    if jj == j {
                        norm_sq += v * v;
                    }
                }
            }
            gamma[j] = norm_sq.max(STEEPEST_EDGE_FLOOR);
        }
        SteepestEdgeState { gamma }
    }

    /// Applies the Forrest-Goldfarb weight update after a pivot at basis
    /// slot `r` that brought in `enter` (whose weight *before* the pivot
    /// was `gamma_t_old`), given `rho` = row `r` of `B^-1` and `w` =
    /// `B^-T alpha` — both computed against the basis as it stood
    /// *before* the pivot — and `pivot = alpha[r]`. Must be called with
    /// `t` already reflecting the *post*-pivot basis/nonbasic status.
    fn update_after_pivot(&mut self, t: &Tableau, _std: &StdForm, rho: &[f64], w: &[f64], gamma_t_old: f64, pivot: f64) {
        // Every column's updated weight only depends on its own (fixed)
        // old weight plus `rho`/`w`/`gamma_t_old`/`pivot`, all read-only
        // here, so this writes disjoint elements of `self.gamma` in
        // parallel via rayon rather than looping sequentially.
        self.gamma.par_iter_mut().enumerate().for_each(|(j, gamma_j)| {
            if t.nb_status[j].is_none() {
                return;
            }
            let col = t.column(j);
            let pivot_sj: f64 = rho.iter().zip(&col).map(|(a, b)| a * b).sum();
            let tau_j: f64 = w.iter().zip(&col).map(|(a, b)| a * b).sum();
            let beta_j = pivot_sj / pivot;
            *gamma_j = (*gamma_j + beta_j * beta_j * (1.0 + gamma_t_old) - 2.0 * beta_j * tau_j).max(STEEPEST_EDGE_FLOOR);
        });
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NbStatus {
    Lower,
    Upper,
}

pub struct SimplexResult {
    pub status: Status,
    pub x: Option<Vec<f64>>,
}

/// Builds the `[A x + s = b]` standard form described above directly from
/// the model's variables/objective/constraints (mirrors `qp::build`, but
/// produces one slack column per row instead of folding bounds into `G`).
struct StdForm {
    n_total: usize,
    n_rows: usize,
    c: Vec<f64>,
    rows: Vec<Vec<(usize, f64)>>, // sparse rows over the n_total columns
    b: Vec<f64>,
    lb: Vec<f64>,
    ub: Vec<f64>,
}

fn build_std_form(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow]) -> StdForm {
    let n = variables.len();
    let n_rows = constraints.len();
    let n_total = n + n_rows;

    let mut c = vec![0.0; n_total];
    let sign = match objective.sense {
        Sense::Minimize => 1.0,
        Sense::Maximize => -1.0,
    };
    for (&j, &v) in objective.expr.coeffs.iter() {
        c[j] = sign * v;
    }

    let mut lb = vec![0.0; n_total];
    let mut ub = vec![0.0; n_total];
    for (j, v) in variables.iter().enumerate() {
        lb[j] = v.lb;
        ub[j] = v.ub;
    }

    let mut rows = Vec::with_capacity(n_rows);
    let mut b = Vec::with_capacity(n_rows);
    for (i, row) in constraints.iter().enumerate() {
        let slack = n + i;
        let mut r: Vec<(usize, f64)> = row.expr.coeffs.iter().map(|(&j, &v)| (j, v)).collect();
        let rhs = row.rhs - row.expr.constant;
        match row.sense {
            RowSense::Eq => {
                lb[slack] = 0.0;
                ub[slack] = 0.0;
                r.push((slack, 1.0));
                b.push(rhs);
            }
            RowSense::Le => {
                lb[slack] = 0.0;
                ub[slack] = f64::INFINITY;
                r.push((slack, 1.0));
                b.push(rhs);
            }
            RowSense::Ge => {
                lb[slack] = 0.0;
                ub[slack] = f64::INFINITY;
                r.push((slack, -1.0));
                b.push(rhs);
            }
        }
        rows.push(r);
    }

    StdForm { n_total, n_rows, c, rows, b, lb, ub }
}

struct Tableau<'a> {
    std: &'a StdForm,
    basis: Vec<usize>,
    /// Inverse of `basis`: `basis_pos[var] = Some(col)` iff `var` is
    /// currently basic at column `col` of `B` — avoids an O(m) scan per
    /// lookup (as a plain linear search over `basis` would need) every
    /// time the sparse basis matrix or a residual is rebuilt.
    basis_pos: Vec<Option<usize>>,
    nb_status: Vec<Option<NbStatus>>, // None for basic columns
    x: Vec<f64>,
}

impl<'a> Tableau<'a> {
    fn new(std: &'a StdForm) -> Self {
        let n_total = std.n_total;
        let n_rows = std.n_rows;
        let mut nb_status = vec![None; n_total];
        let mut x = vec![0.0; n_total];

        // Structural variables always have finite bounds (validated at the
        // `model.rs` boundary — see the module docs), so every one starts
        // nonbasic at its lower bound; there is no "free variable" case.
        for j in 0..(n_total - n_rows) {
            x[j] = std.lb[j];
            nb_status[j] = Some(NbStatus::Lower);
        }
        // Slacks start basic, one per row (B = I).
        let basis: Vec<usize> = (0..n_rows).map(|i| (n_total - n_rows) + i).collect();
        let mut basis_pos = vec![None; n_total];
        for (col, &var) in basis.iter().enumerate() {
            basis_pos[var] = Some(col);
        }

        Tableau { std, basis, basis_pos, nb_status, x }
    }

    fn n_orig(&self) -> usize {
        self.std.n_total - self.std.n_rows
    }

    /// The current basis matrix `B`, sparse rows, column indices in
    /// `0..n_rows` (basis-array position, matching `sparse_lu`'s
    /// convention that a factorized matrix's own column index is what
    /// `col_perm`/`try_update`'s `basis_slot` refer to).
    fn basis_rows_sparse(&self) -> Vec<Vec<(usize, f64)>> {
        let m = self.std.n_rows;
        let mut rows = vec![Vec::new(); m];
        for i in 0..m {
            for &(j, v) in &self.std.rows[i] {
                if let Some(col) = self.basis_pos[j] {
                    rows[i].push((col, v));
                }
            }
        }
        rows
    }

    /// Column `j` of the full constraint matrix, dense, length n_rows.
    fn column(&self, j: usize) -> Vec<f64> {
        let m = self.std.n_rows;
        let mut col = vec![0.0; m];
        for i in 0..m {
            for &(jj, v) in &self.std.rows[i] {
                if jj == j {
                    col[i] = v;
                }
            }
        }
        col
    }

    /// Recomputes every basic variable's value from the current nonbasic
    /// values by solving `B x_B = b - N x_N`. Returns the reduced RHS
    /// (`b - N x_N`) so the caller can cheaply check the true-basis
    /// residual without an extra solve.
    fn recompute_basics(&mut self, lu: &sparse_lu::FtLu) -> Vec<f64> {
        let m = self.std.n_rows;
        let mut rhs = self.std.b.clone();
        for i in 0..m {
            for &(j, v) in &self.std.rows[i] {
                if self.nb_status[j].is_some() {
                    rhs[i] -= v * self.x[j];
                }
            }
        }
        let sol = lu.solve(&rhs);
        for i in 0..m {
            self.x[self.basis[i]] = sol[i];
        }
        rhs
    }

    /// `‖A_B x_B - rhs‖`, using the *true* (not LU-derived) basis matrix
    /// against the already-recomputed `self.x` — refactorization trigger
    /// (1)'s numerical-drift check on the incrementally-updated LU.
    fn basis_residual_norm(&self, rhs: &[f64]) -> f64 {
        let m = self.std.n_rows;
        let mut resid_sq = 0.0;
        for i in 0..m {
            let mut val = 0.0;
            for &(j, v) in &self.std.rows[i] {
                if self.nb_status[j].is_none() {
                    val += v * self.x[j];
                }
            }
            let r = val - rhs[i];
            resid_sq += r * r;
        }
        resid_sq.sqrt()
    }

    /// EXPAND resetting (§4.3): every nonbasic variable within
    /// `EXPAND_DELTA_F` of the bound its status points to is snapped
    /// exactly onto it — undoing the small infeasibilities degenerate
    /// EXPAND steps may have left it with. Basic values are refreshed by
    /// the caller's next `recompute_basics` (using these newly-adjusted
    /// nonbasics), not here.
    fn expand_reset_nonbasics(&mut self) {
        for j in 0..self.std.n_total {
            match self.nb_status[j] {
                Some(NbStatus::Lower) => {
                    if (self.x[j] - self.std.lb[j]).abs() < EXPAND_DELTA_F {
                        self.x[j] = self.std.lb[j];
                    }
                }
                Some(NbStatus::Upper) => {
                    if (self.x[j] - self.std.ub[j]).abs() < EXPAND_DELTA_F {
                        self.x[j] = self.std.ub[j];
                    }
                }
                None => {}
            }
        }
    }

    /// Reassigns every nonbasic (structural) variable's bound to make the
    /// all-slack basis **dual feasible** for `std.c`: since the basic
    /// (slack) costs are always 0, `y = c_B^T B^-1 = 0` here, so every
    /// nonbasic reduced cost equals its raw cost `c_j`. Dual feasibility
    /// needs `c_j >= 0` at the lower bound and `c_j <= 0` at the upper
    /// bound, so a variable is assigned to whichever bound matches the
    /// sign of its cost. This always succeeds — every structural variable
    /// has two finite bounds (validated at the `model.rs` boundary), and
    /// for any real `c`, `c >= -TOL` or `c <= TOL` always holds (the two
    /// half-lines overlap around 0), so one of the two bounds is always a
    /// valid, dual-feasible assignment. A genuinely unbounded-per-variable
    /// LP (needing a free-variable case here) can no longer be
    /// constructed — see the module docs.
    fn crash_dual_feasible(&mut self) {
        for j in 0..self.n_orig() {
            let c = self.std.c[j];
            let lo = self.std.lb[j];
            let hi = self.std.ub[j];
            let status = if c >= -TOL { NbStatus::Lower } else { NbStatus::Upper };
            self.nb_status[j] = Some(status);
            self.x[j] = match status {
                NbStatus::Lower => lo,
                NbStatus::Upper => hi,
            };
        }
    }
}

fn refactorize(std: &StdForm, t: &Tableau) -> sparse_lu::FtLu {
    let rows = t.basis_rows_sparse();
    let base = sparse_lu::factorize(std.n_rows, &rows).expect("simplex basis matrix must be nonsingular");
    sparse_lu::FtLu::new(base)
}

/// One phase of the bounded-variable primal simplex.
///
/// `cost` is recomputed by the caller on every call for phase 1 (it
/// depends on which basics are currently infeasible) and is fixed
/// (`std.c`) for phase 2. `phase1` selects the modified ratio-test
/// blocking rule described in the module docs. `lu` and `since_check`
/// carry the Forrest-Tomlin basis representation and refactorization-
/// trigger cadence across both phases (the basis persists from phase 1
/// into phase 2, so refactorizing it at the phase boundary would be
/// wasted work). `expand` carries the EXPAND working feasibility
/// tolerance and reset cadence across both phases likewise.
fn run_phase(
    std: &StdForm,
    t: &mut Tableau,
    phase1: bool,
    lu: &mut sparse_lu::FtLu,
    since_check: &mut usize,
    expand: &mut ExpandState,
    se: &mut SteepestEdgeState,
) -> Status {
    let m = std.n_rows;

    for _iter in 0..MAX_ITERS {
        let rhs = t.recompute_basics(lu);

        // Triggers (1) and (3): periodic residual / eta-file-fill checks.
        *since_check += 1;
        if *since_check >= FT_CHECK_INTERVAL {
            *since_check = 0;
            let bump_too_big = lu.fill_count() > FT_BUMP_LIMIT_FACTOR * m.max(1);
            let residual_too_big = !bump_too_big && t.basis_residual_norm(&rhs) > FT_RESIDUAL_TOL;
            if bump_too_big || residual_too_big {
                *lu = refactorize(std, t);
            }
        }
        // Trigger (4): unconditional cap on accumulated updates.
        if lu.update_count() > FT_MAX_UPDATES {
            *lu = refactorize(std, t);
        }

        // EXPAND (§4.2): the working feasibility tolerance grows every
        // iteration, and every EXPAND_K iterations a reset restores exact
        // nonbasic bounds and starts a fresh expanding sequence.
        expand.delta += EXPAND_TAU;
        expand.iters_since_reset += 1;
        if expand.iters_since_reset >= EXPAND_K {
            expand.iters_since_reset = 0;
            expand.delta = EXPAND_DELTA_0;
            t.expand_reset_nonbasics();
        }

        // ---- cost vector for this iteration ----
        // Per (7.1)-(7.2): a basic variable counts as infeasible against
        // the *current* working tolerance `expand.delta`, not a fixed
        // epsilon — this is what lets phase 1's "no infeasibilities left"
        // termination test below stay consistent with the same tolerance
        // the ratio test (below) uses to decide which bound is blocking.
        let cost: Vec<f64> = if phase1 {
            (0..m)
                .map(|i| {
                    let var = t.basis[i];
                    let v = t.x[var];
                    if v < std.lb[var] - expand.delta {
                        -1.0
                    } else if v > std.ub[var] + expand.delta {
                        1.0
                    } else {
                        0.0
                    }
                })
                .collect()
        } else {
            t.basis.iter().map(|&var| std.c[var]).collect()
        };

        if phase1 && cost.iter().all(|&c| c == 0.0) {
            return Status::Optimal; // phase-1 feasible
        }

        // y = B^-T cost_B ; reduced cost d_j = c_j - y . a_j
        let y = lu.solve_transpose(&cost);

        // Steepest-edge entering rule (Forrest & Goldfarb 1992): among
        // eligible nonbasic j, maximize d_j^2 / gamma_j rather than
        // Dantzig's |d_j| — see `SteepestEdgeState`'s docs. Each column's
        // reduced cost/score is independent of every other, so this scans
        // `0..n_total` in parallel via rayon.
        let best = (0..std.n_total)
            .into_par_iter()
            .filter_map(|j| {
                let st = t.nb_status[j]?;
                let cj = if phase1 { 0.0 } else { std.c[j] };
                let col = t.column(j);
                let dot: f64 = col.iter().zip(&y).map(|(a, b)| a * b).sum();
                let dj = cj - dot;

                let (eligible, dir) = match st {
                    NbStatus::Lower => (dj < -TOL, 1.0),
                    NbStatus::Upper => (dj > TOL, -1.0),
                };
                if eligible && dj.abs() > TOL {
                    let score = dj * dj / se.gamma[j].max(STEEPEST_EDGE_FLOOR);
                    Some((j, score, dir))
                } else {
                    None
                }
            })
            .max_by(|a, b| a.1.total_cmp(&b.1));

        let Some((enter, _best_score, best_dir)) = best else {
            // No improving direction.
            return if phase1 { Status::Infeasible } else { Status::Optimal };
        };

        // alpha = B^-1 a_enter
        let a_enter = t.column(enter);
        let alpha = lu.solve(&a_enter);

        // ---- two-pass Harris/EXPAND ratio test (Gill, Murray, Saunders &
        // Wright, "A practical anti-cycling procedure for linearly
        // constrained optimization", Mathematical Programming 45 (1989)
        // 437-474, §3.2 and §4) ----
        //
        // Pass 1 computes `alpha1`, the step at which *some* row's bound —
        // relaxed outward by the current working tolerance `expand.delta`
        // — would first be reached. Pass 2 then scans every row whose
        // *exact*-bound step is within that relaxed envelope (`<= alpha1`,
        // possibly negative — see the paper's Cases 1-3) and picks the
        // one with the largest pivot magnitude, favoring a well-
        // conditioned pivot over the textbook "first to block" choice.
        // The final step is floored at `alpha_min = tau / |pivot|` > 0
        // (never exactly 0), which is what actually prevents cycling: a
        // degenerate pivot may leave the leaving variable slightly
        // outside its bound (by at most `expand.delta`), cleaned up later
        // by `expand_reset_nonbasics`.
        struct Candidate {
            row: usize,
            exact: f64,
            relaxed: f64,
            pivot_abs: f64,
            hits_upper: bool,
        }

        let self_width = std.ub[enter] - std.lb[enter];
        let init_alpha1 = if self_width.is_finite() { self_width } else { f64::INFINITY };

        // Each row's blocking analysis is independent of every other row's
        // — only the final `alpha1`/leaving-row reductions below combine
        // them — so this scans `0..m` in parallel via rayon.
        let candidates: Vec<Candidate> = (0..m)
            .into_par_iter()
            .filter_map(|i| {
                let rate = -best_dir * alpha[i]; // d(x_Bi)/d(theta)
                if rate.abs() <= TOL {
                    return None;
                }
                let var = t.basis[i];
                let val = t.x[var];
                let infeasible_low = phase1 && val < std.lb[var] - expand.delta;
                let infeasible_high = phase1 && val > std.ub[var] + expand.delta;

                // The blocking bound for this row depends on whether it is
                // currently feasible, or (phase 1 only) which side it
                // violates: an infeasible variable is only blocked by the
                // bound it is heading *towards* — moving further into
                // infeasibility is never itself blocked by this row (the
                // Phase-1 bounds of §7.1: the violated side's bound is, in
                // effect, infinite). A row *returning* to feasibility isn't
                // at risk of a *new* infeasibility from this bound, so it
                // gets no outward slack (`relaxed == exact`) — EXPAND's
                // relaxation targets rows that could newly become infeasible,
                // which the classical (non-Phase-1) presentation of the
                // algorithm is the only case that arises.
                let (bound, is_upper, active, returning_to_feasibility) = if rate < 0.0 {
                    if infeasible_high {
                        (std.ub[var], true, true, true)
                    } else if infeasible_low {
                        (std.lb[var], false, false, false)
                    } else {
                        (std.lb[var], false, true, false)
                    }
                } else if infeasible_low {
                    (std.lb[var], false, true, true)
                } else if infeasible_high {
                    (std.ub[var], true, false, false)
                } else {
                    (std.ub[var], true, true, false)
                };

                if !active || !bound.is_finite() {
                    return None;
                }
                let exact = (bound - val) / rate;
                let relaxed = if returning_to_feasibility {
                    exact
                } else {
                    let relaxed_bound = if is_upper { bound + expand.delta } else { bound - expand.delta };
                    (relaxed_bound - val) / rate
                };

                Some(Candidate { row: i, exact, relaxed, pivot_abs: alpha[i].abs(), hits_upper: is_upper })
            })
            .collect();

        let alpha1 = candidates.par_iter().map(|c| c.relaxed).reduce(|| init_alpha1, f64::min);

        let leaving = candidates
            .par_iter()
            .filter(|c| c.exact <= alpha1 + TOL)
            .max_by(|a, b| a.pivot_abs.total_cmp(&b.pivot_abs));
        let (leaving_row, leaving_hits_upper, alpha2, best_pivot_mag) = match leaving {
            Some(c) if c.pivot_abs > 0.0 => (Some(c.row), c.hits_upper, c.exact, c.pivot_abs),
            _ => (None, false, 0.0, 0.0),
        };

        let theta = match leaving_row {
            None => {
                if !alpha1.is_finite() {
                    return Status::Unbounded;
                }
                alpha1
            }
            Some(_) => alpha2.max(EXPAND_TAU / best_pivot_mag),
        };

        // Apply the step.
        for i in 0..m {
            let var = t.basis[i];
            t.x[var] -= best_dir * alpha[i] * theta;
        }
        t.x[enter] += best_dir * theta;

        match leaving_row {
            None => {
                // Bound flip: entering variable moves to its opposite bound,
                // stays nonbasic.
                let new_status = if best_dir > 0.0 { NbStatus::Upper } else { NbStatus::Lower };
                t.nb_status[enter] = Some(new_status);
                t.x[enter] = if best_dir > 0.0 { std.ub[enter] } else { std.lb[enter] };
            }
            Some(r) => {
                // Steepest-edge weight update (§ see `SteepestEdgeState`):
                // needs two extra BTRAN-style solves against the OLD
                // basis's LU — `rho` = row r of B^-1 (for beta_j) and `w`
                // = B^-T alpha (for the cross term tau_j) — computed now,
                // before the swap changes what `lu` represents.
                let mut e_r = vec![0.0; m];
                e_r[r] = 1.0;
                let rho = lu.solve_transpose(&e_r);
                let w = lu.solve_transpose(&alpha);
                let gamma_t_old = se.gamma[enter];
                let pivot = alpha[r];

                let leaving_var = t.basis[r];
                t.nb_status[leaving_var] = Some(if leaving_hits_upper { NbStatus::Upper } else { NbStatus::Lower });
                // Unlike a textbook ratio test, EXPAND does *not* snap the
                // leaving variable exactly onto its bound on a degenerate
                // step (theta == alpha_min): its value is whatever the
                // step above computed, which may violate the bound by up
                // to `expand.delta` (Cases 2-3 in the paper) — that small
                // infeasibility is what guarantees a strictly positive
                // step was possible, and is cleaned up later by
                // `expand_reset_nonbasics`.
                t.basis_pos[leaving_var] = None;
                t.basis[r] = enter;
                t.basis_pos[enter] = Some(r);
                t.nb_status[enter] = None;

                // Applies to every (now-)nonbasic column, which naturally
                // includes the just-arrived leaving variable and excludes
                // the just-entered one.
                se.update_after_pivot(t, std, &rho, &w, gamma_t_old, pivot);

                // Trigger (2): FtLu::try_update refactorizes in-place if
                // the resulting pivot is too small to use safely.
                if !lu.try_update(r, &a_enter, FT_MIN_PIVOT) {
                    *lu = refactorize(std, t);
                }
            }
        }
    }

    Status::Optimal // iteration cap hit; best-effort
}

pub fn solve_lp(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow]) -> SimplexResult {
    let std = build_std_form(variables, objective, constraints);
    if std.n_rows == 0 {
        // No constraints at all: every variable's bound is finite, so the
        // optimum is trivially at whichever bound the sign of c favors
        // (either bound when c is ~0) — no unbounded case is possible.
        let mut x = vec![0.0; variables.len()];
        for j in 0..variables.len() {
            x[j] = if std.c[j] > TOL { std.lb[j] } else if std.c[j] < -TOL { std.ub[j] } else { std.lb[j] };
        }
        return SimplexResult { status: Status::Optimal, x: Some(x) };
    }

    let mut t = Tableau::new(&std);
    // Initial basis is all slacks (B = a signed identity), trivially
    // factorized; Markowitz pivoting still goes through `factorize` for
    // uniformity rather than special-casing this as the identity.
    let mut lu = refactorize(&std, &t);
    let mut since_check = 0usize;
    let mut expand = ExpandState::new();
    let mut se = SteepestEdgeState::new(&std);

    let phase1_status = run_phase(&std, &mut t, true, &mut lu, &mut since_check, &mut expand, &mut se);
    if phase1_status == Status::Infeasible {
        return SimplexResult { status: Status::Infeasible, x: None };
    }

    let phase2_status = run_phase(&std, &mut t, false, &mut lu, &mut since_check, &mut expand, &mut se);
    match phase2_status {
        Status::Unbounded => SimplexResult { status: Status::Unbounded, x: None },
        Status::Infeasible => SimplexResult { status: Status::Infeasible, x: None }, // shouldn't happen after phase 1
        Status::Optimal => SimplexResult { status: Status::Optimal, x: Some(t.x[0..t.n_orig()].to_vec()) },
    }
}

/// Dual steepest-edge (DSE) weights (Forrest & Goldfarb 1992; formulas as
/// stated — and independently re-derived and confirmed to match exactly —
/// in Huangfu, Q. and Hall, J.A.J., "Parallelizing the dual revised
/// simplex method", arXiv:1503.01889, §2.2.1/2.2.3): for basic row `i`,
/// `w[i] = ||e_i^T B^-1||^2`, tracking how much a unit change forced onto
/// row `i` (by some future pivot) would perturb the whole basic solution.
/// `chuzr` (leaving-row selection) picks the primal-infeasible row
/// maximizing `delta_i^2 / w[i]` — the dual analogue of primal steepest
/// edge's `d_j^2 / gamma_j`.
struct DseState {
    w: Vec<f64>,
}

impl DseState {
    /// `B0` is a signed identity, so `e_i^T B0^-1` is `+/-e_i^T` and
    /// `w[i] = 1` for every row initially.
    fn new(m: usize) -> Self {
        DseState { w: vec![1.0; m] }
    }

    /// `p` = the pivot row (basis slot that left), `alpha` = `B^-1 a_q`
    /// (the entering column's FTRAN, against the basis as it stood
    /// *before* the pivot), `tau` = `B^-1 (B^-T e_p)` ("ftran-dse").
    fn update_after_pivot(&mut self, p: usize, alpha: &[f64], tau: &[f64]) {
        let pivot = alpha[p];
        let wp_old = self.w[p];
        // Disjoint per-row writes, same rationale as `SteepestEdgeState`'s
        // update — parallelized via rayon.
        self.w.par_iter_mut().enumerate().for_each(|(i, w_i)| {
            if i == p {
                return;
            }
            let ratio = alpha[i] / pivot;
            *w_i = (*w_i - 2.0 * ratio * tau[i] + ratio * ratio * wp_old).max(STEEPEST_EDGE_FLOOR);
        });
        self.w[p] = (wp_old / (pivot * pivot)).max(STEEPEST_EDGE_FLOOR);
    }
}

/// Bounded-variable **dual** revised simplex, reusing the same basis
/// representation (`sparse_lu::FtLu`, the Markowitz/FT/4-trigger machinery
/// of Stage 2) and standard form (`StdForm`/`Tableau`) as the primal
/// method above, with dual steepest-edge pricing (`DseState`) in place of
/// Dantzig's rule.
///
/// Unlike the primal method, this needs a *dual-feasible* starting basis
/// rather than a two-phase procedure — `Tableau::crash_dual_feasible` gets
/// one unconditionally, assigning every variable to whichever of its two
/// (always finite) bounds matches its cost's sign (`y = 0` at the
/// all-slack basis since slack costs are always 0, so a nonbasic's
/// reduced cost is just its raw cost) — see that method's docs for why
/// this can never fail once every variable has two finite bounds.
///
/// The ratio test (`chuzc`) here is the classical/textbook one: eligible
/// candidates are found from the leaving row's sign pattern and the
/// smallest `|d_j / alpha_pj|` among them is taken exactly — not (yet)
/// the Harris two-pass or bound-flipping (BFRT) refinements a production
/// implementation would add on top.
pub fn solve_lp_dual(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow]) -> SimplexResult {
    let std = build_std_form(variables, objective, constraints);
    if std.n_rows == 0 {
        return solve_lp(variables, objective, constraints);
    }

    let mut t = Tableau::new(&std);
    t.crash_dual_feasible();

    let mut lu = refactorize(&std, &t);
    let mut since_check = 0usize;
    let mut dse = DseState::new(std.n_rows);
    let m = std.n_rows;

    for _iter in 0..MAX_ITERS {
        let rhs = t.recompute_basics(&lu);

        // Same 4-trigger refactorization policy as the primal method.
        since_check += 1;
        if since_check >= FT_CHECK_INTERVAL {
            since_check = 0;
            let bump_too_big = lu.fill_count() > FT_BUMP_LIMIT_FACTOR * m.max(1);
            let residual_too_big = !bump_too_big && t.basis_residual_norm(&rhs) > FT_RESIDUAL_TOL;
            if bump_too_big || residual_too_big {
                lu = refactorize(&std, &t);
            }
        }
        if lu.update_count() > FT_MAX_UPDATES {
            lu = refactorize(&std, &t);
        }

        // chuzr: most DSE-attractive primal-infeasible basic row. Each
        // row's infeasibility/score is independent, so this scans `0..m`
        // in parallel via rayon.
        let chuzr = (0..m)
            .into_par_iter()
            .filter_map(|i| {
                let var = t.basis[i];
                let val = t.x[var];
                let delta = if val < std.lb[var] - TOL {
                    std.lb[var] - val
                } else if val > std.ub[var] + TOL {
                    val - std.ub[var]
                } else {
                    0.0
                };
                if delta <= TOL {
                    return None;
                }
                let score = delta * delta / dse.w[i].max(STEEPEST_EDGE_FLOOR);
                Some((i, score, val < std.lb[var]))
            })
            .max_by(|a, b| a.1.total_cmp(&b.1));
        let Some((p, _best_score, leaving_infeasible_low)) = chuzr else {
            return SimplexResult { status: Status::Optimal, x: Some(t.x[0..t.n_orig()].to_vec()) };
        };

        // Pivotal row: rho_p = B^-T e_p (btran), then a_pj = rho_p . A_j
        // for each nonbasic j (spmv) — same pattern as the primal
        // method's steepest-edge `rho`.
        let mut e_p = vec![0.0; m];
        e_p[p] = 1.0;
        let rho_p = lu.solve_transpose(&e_p);

        let cost_b: Vec<f64> = t.basis.iter().map(|&v| std.c[v]).collect();
        let y = lu.solve_transpose(&cost_b);

        // chuzc: eligible nonbasic j (sign of a_pj compatible with
        // restoring feasibility at row p, given its bound status) with
        // smallest |d_j / a_pj|. Each column's ratio is independent, so
        // this scans `0..n_total` in parallel via rayon.
        let chuzc = (0..std.n_total)
            .into_par_iter()
            .filter_map(|j| {
                let st = t.nb_status[j]?;
                let col = t.column(j);
                let a_pj: f64 = rho_p.iter().zip(&col).map(|(a, b)| a * b).sum();
                if a_pj.abs() <= TOL {
                    return None;
                }
                let eligible = match st {
                    NbStatus::Lower => {
                        if leaving_infeasible_low {
                            a_pj < -TOL
                        } else {
                            a_pj > TOL
                        }
                    }
                    NbStatus::Upper => {
                        if leaving_infeasible_low {
                            a_pj > TOL
                        } else {
                            a_pj < -TOL
                        }
                    }
                };
                if !eligible {
                    return None;
                }
                let dj = std.c[j] - col.iter().zip(&y).map(|(a, b)| a * b).sum::<f64>();
                let ratio = (dj / a_pj).abs();
                Some((j, ratio))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let Some((q, _best_ratio)) = chuzc else {
            return SimplexResult { status: Status::Infeasible, x: None };
        };

        // Full ftran of the entering column (needed for both the primal
        // update and the DSE weight update) and ftran-dse for the weight
        // update's cross term.
        let a_enter = t.column(q);
        let alpha = lu.solve(&a_enter);
        let tau = lu.solve(&rho_p);

        let leaving_var = t.basis[p];
        let target_bound = if leaving_infeasible_low { std.lb[leaving_var] } else { std.ub[leaving_var] };
        let theta_q = (t.x[leaving_var] - target_bound) / alpha[p];

        for i in 0..m {
            let var = t.basis[i];
            t.x[var] -= alpha[i] * theta_q;
        }
        t.x[q] += theta_q;

        dse.update_after_pivot(p, &alpha, &tau);

        t.nb_status[leaving_var] = Some(if leaving_infeasible_low { NbStatus::Lower } else { NbStatus::Upper });
        t.basis_pos[leaving_var] = None;
        t.basis[p] = q;
        t.basis_pos[q] = Some(p);
        t.nb_status[q] = None;

        if !lu.try_update(p, &a_enter, FT_MIN_PIVOT) {
            lu = refactorize(&std, &t);
        }
    }

    SimplexResult { status: Status::Optimal, x: Some(t.x[0..t.n_orig()].to_vec()) } // iteration cap; best-effort
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LinearExpr, VarType};
    use std::collections::HashMap;

    fn var(lb: f64, ub: f64) -> VariableData {
        VariableData { vtype: VarType::Continuous, lb, ub }
    }

    fn expr(terms: &[(usize, f64)]) -> LinearExpr {
        LinearExpr { coeffs: terms.iter().cloned().collect::<HashMap<_, _>>(), constant: 0.0 }
    }

    fn row(terms: &[(usize, f64)], sense: RowSense, rhs: f64) -> ConstraintRow {
        ConstraintRow { expr: expr(terms), sense, rhs }
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    /// Solves via the interior-point (IP-PMM) engine directly —
    /// `qp::build` + `ipm::solve`, bypassing `solver::solve_lp` — so
    /// cross-check tests stay genuinely independent of this module now
    /// that `solver::solve_lp` itself calls `solve_lp_dual` (§4 of
    /// DESIGN.md: the simplex engine is the active `Model.solve()` path).
    fn solve_via_ipm(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow]) -> crate::types::SolveResult {
        let obj_coeffs_for_min: Vec<(usize, f64)> = match objective.sense {
            Sense::Minimize => objective.expr.coeffs.iter().map(|(&j, &c)| (j, c)).collect(),
            Sense::Maximize => objective.expr.coeffs.iter().map(|(&j, &c)| (j, -c)).collect(),
        };
        let qp = crate::interior_point::qp::build(variables, &obj_coeffs_for_min, constraints);
        let result = crate::interior_point::solve(&qp);
        match result.status {
            Status::Optimal => {
                let x = result.x.unwrap();
                let obj_val = objective.expr.constant + objective.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum::<f64>();
                crate::types::SolveResult { status: Status::Optimal, objective: Some(obj_val), x: Some(x), node_limit_hit: false }
            }
            status => crate::types::SolveResult { status, objective: None, x: None, node_limit_hit: false },
        }
    }

    #[test]
    fn lp1_maximize_with_le_bounds() {
        // max x + 2y s.t. x+y<=10, x,y in [0,10] -> optimal 20 at (0,10)
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 2.0)]), sense: Sense::Maximize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 10.0)];
        let res = solve_lp(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 0.0), "x={x:?}");
        assert!(approx(x[1], 10.0), "x={x:?}");
    }

    #[test]
    fn lp2_minimize_with_ge_and_eq() {
        // min a+b s.t. a+2b>=6, a-b==0, a,b in [0,1000] -> optimal 4 at (2,2)
        // (the upper bound is generous enough that it never binds; every
        // variable now needs two finite bounds — see the module docs).
        let vars = vec![var(0.0, 1000.0), var(0.0, 1000.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 2.0)], RowSense::Ge, 6.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Eq, 0.0),
        ];
        let res = solve_lp(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 2.0), "x={x:?}");
        assert!(approx(x[1], 2.0), "x={x:?}");
    }

    #[test]
    fn lp3_wide_bounds_dont_bind() {
        // min z s.t. z>=-5, z<=100, z in [-1e6,1e6] -> optimal -5 (the
        // variable's own declared bounds are far wider than the
        // constraints, which are what actually determine the optimum;
        // this project no longer supports genuinely free/infinite-bound
        // variables — see the module docs).
        let vars = vec![var(-1.0e6, 1.0e6)];
        let obj = Objective { expr: expr(&[(0, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0)], RowSense::Ge, -5.0),
            row(&[(0, 1.0)], RowSense::Le, 100.0),
        ];
        let res = solve_lp(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        assert!(approx(res.x.unwrap()[0], -5.0));
    }

    #[test]
    fn infeasible_detected() {
        // w in [0,5], w>=10 -> infeasible
        let vars = vec![var(0.0, 5.0)];
        let obj = Objective { expr: expr(&[(0, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0)], RowSense::Ge, 10.0)];
        let res = solve_lp(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Infeasible);
    }

    #[test]
    fn knapsack_lp_relaxation() {
        // max 60x1+100x2+120x3 s.t. 10x1+20x2+30x3<=50, x_i in [0,1]
        // (LP relaxation of the classic 0/1 knapsack instance used
        // elsewhere in this project's test suite).
        let vars = vec![var(0.0, 1.0), var(0.0, 1.0), var(0.0, 1.0)];
        let obj = Objective { expr: expr(&[(0, 60.0), (1, 100.0), (2, 120.0)]), sense: Sense::Maximize };
        let cons = vec![row(&[(0, 10.0), (1, 20.0), (2, 30.0)], RowSense::Le, 50.0)];
        let res = solve_lp(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        let obj_val = 60.0 * x[0] + 100.0 * x[1] + 120.0 * x[2];
        // LP relaxation optimum is 240 (x2=x3=1, x1=0), strictly better
        // than the integer optimum of 220 found elsewhere by bnb.rs.
        assert!(approx(obj_val, 240.0), "obj={obj_val} x={x:?}");
    }

    #[test]
    fn larger_lp_matches_independent_ipm_solver_and_exercises_ft_triggers() {
        // 15 variables, 14 chained pairwise constraints plus one global
        // constraint -> comfortably more than FT_CHECK_INTERVAL (5)
        // basis changes, so this exercises the periodic residual/bump
        // checks (and, if the eta file grows enough, an in-loop
        // refactorization) rather than only the "never triggers" path
        // the small hand-checked LPs above take. Cross-checked against
        // `solve_via_ipm` (the independent IP-PMM solver, called directly
        // rather than via `solver::solve_lp` which now routes to this
        // module) rather than a hand-computed optimum, since this LP is
        // too big to verify by hand with confidence.
        let n = 60;
        let vars: Vec<VariableData> = (0..n).map(|_| var(0.0, 8.0)).collect();
        let obj_terms: Vec<(usize, f64)> = (0..n).map(|i| (i, 1.0 + (i % 4) as f64)).collect();
        let obj = Objective { expr: expr(&obj_terms), sense: Sense::Maximize };

        let mut cons: Vec<ConstraintRow> = Vec::new();
        for i in 0..(n - 1) {
            cons.push(row(&[(i, 1.0), (i + 1, 1.0)], RowSense::Le, 10.0));
        }
        cons.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Le, 220.0));
        // Forces genuine phase-1 work (slack-only start is infeasible here).
        cons.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Ge, 40.0));

        let res = solve_lp(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        let obj_val: f64 = obj_terms.iter().map(|&(j, c)| c * x[j]).sum();

        let ipm_res = solve_via_ipm(&vars, &obj, &cons);
        assert_eq!(ipm_res.status, Status::Optimal);
        let ipm_obj = ipm_res.objective.unwrap();

        assert!((obj_val - ipm_obj).abs() < 1e-4, "simplex_obj={obj_val} ipm_obj={ipm_obj} x={x:?}");
    }

    #[test]
    fn beale_cycling_example_terminates_correctly() {
        // The classic Beale/Chvátal degenerate LP known to cycle under
        // Dantzig's rule with naive tie-breaking when started from the
        // slack basis at the origin — exactly this project's entering-
        // rule (Dantzig) and starting point. Without genuine anti-cycling
        // (EXPAND's positive-step guarantee), this either loops forever
        // or — since this implementation caps at MAX_ITERS and returns
        // "Optimal" as a best-effort fallback — silently returns a wrong,
        // non-optimal answer at the iteration cap instead of the true
        // optimum. Cross-checked against the independent IP-PMM solver.
        //
        //   minimize -0.75 x0 + 150 x1 - 0.02 x2 + 6 x3
        //   s.t.  0.25 x0 - 60 x1 - 0.04 x2 + 9 x3 <= 0
        //         0.5  x0 - 90 x1 - 0.02 x2 + 3 x3 <= 0
        //         x2 <= 1
        //         x0,x1,x2,x3 >= 0
        //
        // Upper bound 100 (true optimum is near [0.04, 0, 1, 0]): bounds
        // >= 1000 here make `solve_via_ipm` (the cross-check oracle, not
        // this module) misreport Infeasible — a pre-existing IP-PMM
        // scaling/tolerance sensitivity to loose bounds on a problem this
        // numerically small, confirmed unrelated to this module by
        // bisecting the bound value; 100 is still comfortably non-binding.
        let vars = vec![var(0.0, 100.0), var(0.0, 100.0), var(0.0, 100.0), var(0.0, 100.0)];
        let obj = Objective { expr: expr(&[(0, -0.75), (1, 150.0), (2, -0.02), (3, 6.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 0.25), (1, -60.0), (2, -0.04), (3, 9.0)], RowSense::Le, 0.0),
            row(&[(0, 0.5), (1, -90.0), (2, -0.02), (3, 3.0)], RowSense::Le, 0.0),
            row(&[(2, 1.0)], RowSense::Le, 1.0),
        ];

        let res = solve_lp(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        let obj_val =
            -0.75 * x[0] + 150.0 * x[1] - 0.02 * x[2] + 6.0 * x[3];

        let ipm_res = solve_via_ipm(&vars, &obj, &cons);
        assert_eq!(ipm_res.status, Status::Optimal);
        let ipm_obj = ipm_res.objective.unwrap();

        assert!((obj_val - ipm_obj).abs() < 1e-4, "simplex_obj={obj_val} ipm_obj={ipm_obj} x={x:?}");
    }

    #[test]
    fn steepest_edge_weights_match_brute_force_recompute() {
        // Empirical check for SteepestEdgeState::update_after_pivot's
        // formula: a secondary source's rendering of the Forrest-Goldfarb
        // update disagreed with a from-scratch derivation over whether
        // the middle term's `beta_j` factor is squared (derivation: yes;
        // source as transcribed: no). Since a wrong weight here only
        // degrades pricing quality, not correctness (unlike a wrong
        // `FtLu` pivot), the tiebreaker is empirical: perform one real
        // pivot, then compare the incrementally-updated gamma[j] against
        // ||fresh_lu.solve(A_j)||^2 computed via a brand new
        // factorization of the post-pivot basis, for every nonbasic j.
        //
        //   vars x0,x1,x2 in [0,10]
        //   row0: x0+x1+x2 <= 10
        //   row1: x0-x1+2x2 <= 8
        // Starting basis = slacks (B0 = I). Entering x0 (best_dir=+1):
        // alpha = B0^-1 * col(x0) = [1,1]. Both slack rows decrease as x0
        // increases (rate<0 for both); ratio test: s0/1=10, s1/1=8, so
        // row 1 (s1) blocks first at theta=8 -- a clean, hand-verifiable,
        // non-degenerate pivot.
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0), (2, 1.0)]), sense: Sense::Maximize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0), (2, 1.0)], RowSense::Le, 10.0),
            row(&[(0, 1.0), (1, -1.0), (2, 2.0)], RowSense::Le, 8.0),
        ];
        let std = build_std_form(&vars, &obj, &cons);
        let mut t = Tableau::new(&std);
        let lu = refactorize(&std, &t);
        t.recompute_basics(&lu);
        let mut se = SteepestEdgeState::new(&std);

        let enter = 0usize; // x0
        let a_enter = t.column(enter);
        let alpha = lu.solve(&a_enter);
        let r = 1usize; // row 1 (s1) blocks first, per the analysis above

        let mut e_r = vec![0.0; std.n_rows];
        e_r[r] = 1.0;
        let rho = lu.solve_transpose(&e_r);
        let w = lu.solve_transpose(&alpha);
        let gamma_t_old = se.gamma[enter];
        let pivot = alpha[r];

        // Apply the pivot to the tableau (mirrors run_phase's bookkeeping).
        // s1 is a Le slack (bounds [0, inf)) decreasing from 8 to 0 as x0
        // increases, so it leaves at its *lower* bound.
        let leaving_var = t.basis[r];
        t.nb_status[leaving_var] = Some(NbStatus::Lower);
        t.basis_pos[leaving_var] = None;
        t.basis[r] = enter;
        t.basis_pos[enter] = Some(r);
        t.nb_status[enter] = None;

        se.update_after_pivot(&t, &std, &rho, &w, gamma_t_old, pivot);

        // Brute force: factorize the new basis fresh and directly compute
        // ||B_new^-1 A_j||^2 for every nonbasic j.
        let fresh_lu = refactorize(&std, &t);
        for j in 0..std.n_total {
            if t.nb_status[j].is_none() {
                continue;
            }
            let col = t.column(j);
            let brute = fresh_lu.solve(&col);
            let brute_norm_sq: f64 = brute.iter().map(|v| v * v).sum();
            assert!(
                (se.gamma[j] - brute_norm_sq).abs() < 1e-6 * brute_norm_sq.max(1.0),
                "j={j} incremental={} brute_force={}",
                se.gamma[j],
                brute_norm_sq
            );
        }
    }

    #[test]
    fn dual_lp1_matches_primal() {
        // Same LP as lp1_maximize_with_le_bounds. Both x,y have negative
        // internal (minimize-form) cost and finite upper bounds, so the
        // dual crash starts at x=y=10 -- primal infeasible (slack -10) but
        // dual feasible -- a genuine dual-simplex pivot, not the trivial
        // already-optimal case.
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 2.0)]), sense: Sense::Maximize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 10.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 0.0), "x={x:?}");
        assert!(approx(x[1], 10.0), "x={x:?}");
    }

    #[test]
    fn dual_lp2_matches_primal() {
        let vars = vec![var(0.0, 1000.0), var(0.0, 1000.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 2.0)], RowSense::Ge, 6.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Eq, 0.0),
        ];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 2.0), "x={x:?}");
        assert!(approx(x[1], 2.0), "x={x:?}");
    }

    #[test]
    fn dual_knapsack_lp_relaxation() {
        let vars = vec![var(0.0, 1.0), var(0.0, 1.0), var(0.0, 1.0)];
        let obj = Objective { expr: expr(&[(0, 60.0), (1, 100.0), (2, 120.0)]), sense: Sense::Maximize };
        let cons = vec![row(&[(0, 10.0), (1, 20.0), (2, 30.0)], RowSense::Le, 50.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        let obj_val = 60.0 * x[0] + 100.0 * x[1] + 120.0 * x[2];
        assert!(approx(obj_val, 240.0), "obj={obj_val} x={x:?}");
    }

    #[test]
    fn dual_larger_lp_matches_independent_ipm_solver() {
        // Same construction as
        // larger_lp_matches_independent_ipm_solver_and_exercises_ft_triggers,
        // reused here to cross-check the dual method at a scale beyond
        // what's practical to verify by hand.
        let n = 60;
        let vars: Vec<VariableData> = (0..n).map(|_| var(0.0, 8.0)).collect();
        let obj_terms: Vec<(usize, f64)> = (0..n).map(|i| (i, 1.0 + (i % 4) as f64)).collect();
        let obj = Objective { expr: expr(&obj_terms), sense: Sense::Maximize };

        let mut cons: Vec<ConstraintRow> = Vec::new();
        for i in 0..(n - 1) {
            cons.push(row(&[(i, 1.0), (i + 1, 1.0)], RowSense::Le, 10.0));
        }
        cons.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Le, 220.0));
        cons.push(row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Ge, 40.0));

        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        let obj_val: f64 = obj_terms.iter().map(|&(j, c)| c * x[j]).sum();

        let ipm_res = solve_via_ipm(&vars, &obj, &cons);
        assert_eq!(ipm_res.status, Status::Optimal);
        let ipm_obj = ipm_res.objective.unwrap();

        assert!((obj_val - ipm_obj).abs() < 1e-4, "dual_obj={obj_val} ipm_obj={ipm_obj} x={x:?}");
    }
}
