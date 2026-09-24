//! A from-scratch **bounded-variable revised simplex**, dual method
//! ([`solve_lp_dual`]) — `solver::solve_lp`'s default engine for every LP
//! `Model.solve()` needs to solve (including once per branch-and-bound
//! node for MIPs; see `mip.rs`), reachable explicitly via
//! `Model.solve(root_solver="simplex")` alongside the interior-point
//! alternative (`types::RootSolver`). Every solve goes through the
//! extended dual simplex ([`extended_dual::solve_lp_dual_extended`]), split
//! into independent connected components first when the presolved problem
//! has them (see [`solve_std_form_decomposed`]). This module keeps the
//! shared machinery that solver builds on — standard form, presolve
//! glue, [`Tableau`], the primal [`run_phase`] its polish hands off to,
//! and the DSE pricing state. There is no classical dual method or
//! `BIG_M` fallback any more: when the extended solver gives up, the
//! solve reports [`Status::NotSolved`].
//!
//! ## Bounds
//!
//! A structural variable's bounds may be infinite on either side; presolve
//! eliminates what it can, and whatever survives is handed to
//! `extended_dual` as a genuine infinity (tracked symbolically there, see
//! that module's docs). [`Tableau`] and the primal method below still
//! assume finite structural bounds — they only ever see a basis
//! `extended_dual` has already brought within its true bounds.
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
//! ## Dual method
//!
//! Lives in `extended_dual` (DSE pricing via [`DseState`], BFRT,
//! incremental reduced costs). The primal method above is only its
//! polish-stage handoff target.
//!
//! ## Parallelism: presolve yes, the per-iteration loop no
//!
//! `crate::presolve` (scaling, redundancy, propagation, DualFix,
//! ColSingleton) runs once per solve and is parallelized with rayon
//! throughout. Every loop *inside* the dual simplex's and
//! [`run_phase`]'s per-iteration bodies (chuzr, chuzc1's candidate scan,
//! the reduced-cost/DSE/steepest-edge weight updates) is embarrassingly
//! parallel in the same sense, but is deliberately sequential: profiling
//! them on this crate's target problem sizes (~1000 variables, a few
//! hundred rows) found rayon's per-call dispatch overhead alone — paid
//! hundreds of times per solve, once per iteration — costing *more* than
//! the rest of the dual simplex loop combined, and removing it measured
//! roughly a 5x end-to-end speedup. A large-enough problem could tip this
//! back in rayon's favor, but no such threshold is implemented; if this
//! module is ever aimed at dramatically larger LPs, re-profile before
//! reaching for `into_par_iter()` again rather than assuming it helps.

use crate::presolve::{self, scaling};
use crate::sparse::{CscMat, CsrMat, csr_row_iter, sparse_axpy_dense, sparse_dot_dense};
use crate::types::{ConstraintRow, Objective, RowSense, Sense, Status, VariableData};
use crate::params::simplex::{EXPAND_DELTA_0, EXPAND_DELTA_F, EXPAND_K, EXPAND_TAU, FT_BUMP_LIMIT_FACTOR, FT_CHECK_INTERVAL, FT_MAX_UPDATES, FT_MIN_PIVOT, FT_RESIDUAL_TOL, MAX_ITERS_CEILING, MAX_ITERS_FLOOR, MAX_ITERS_SCALE, PARALLEL_COMPONENT_MIN_VARS, PARTIAL_PRICING_GROUPS, PARTIAL_PRICING_THRESHOLD, PRESOLVE_ROUNDS, PRIMAL_HARRIS_TOL, PROPAGATION_PASSES, RAYON_SIZE_THRESHOLD, ROWSINGLETON_COLSINGLETON_INNER_ROUNDS, RUIZ_ITERS, STALL_PROGRESS_EPS, STEEPEST_EDGE_FLOOR, TOL, UPDATE_VERIFY_TOL};

/// Markowitz-pivoted sparse LU + Forrest-Tomlin incremental updates —
/// see `lu`'s own module docs. Referred to below as `sparse_lu` (not the
/// bare module name `lu`) purely to avoid clashing with the many local
/// variables/parameters through this file that are themselves named `lu`
/// (an `FtLu` instance, the live basis factorization).
mod lu;
pub(crate) use lu::tiny_drop;
use self::lu as sparse_lu;

mod extended_dual;

/// Size-scaled replacement for a flat iteration cap on every simplex main
/// loop (classical primal/dual and the extended-dual module).
///
/// **Why a flat constant was wrong:** a fixed `20_000`-iteration budget is
/// independent of problem size, so it silently starves large instances
/// instead of scaling with the amount of work a correct solve of that size
/// can legitimately need — confirmed directly on Netlib `dfl001`
/// (`m=6071`/`n=12230`, presolved to `m=4554`/`n_total=9773`): the
/// extended-dual loop needs 22,015 iterations to reach the exact HiGHS
/// objective, ~10% over the old flat cap. Hitting that cap doesn't fail
/// loudly — [`extended_dual::solve_lp_dual_extended`] returns `None` and
/// the solve reports `Status::NotSolved` (before that status existed, the
/// caller fell back to a classical `BIG_M`-substituted path, which
/// returned a **wrong** objective on `dfl001`: `11264657.2` vs HiGHS's
/// `11266396.0`, ~1.5e-4 relative error). See the `dfl001-bottleneck-max-iters-cap` memory for
/// the full measurement.
///
/// Bland's-rule anti-cycling (`bland_mode` in every main loop this feeds)
/// already gives a textbook finite-termination guarantee independent of
/// this cap — this function exists only to bound the *practical* wall
/// time of a single solve, not to serve as the actual correctness
/// safeguard, so generous headroom above any realistically-needed
/// iteration count is the right tradeoff over a tight one.
fn max_iters_for(m: usize, n_total: usize) -> usize {
    (MAX_ITERS_SCALE * (m + n_total)).clamp(MAX_ITERS_FLOOR, MAX_ITERS_CEILING)
}

/// HiGHS `HEkkDualRow::updateVerify` equivalent: cross-checks this
/// iteration's pivot element between PRICE's row-direction value
/// (`alpha_row`, `a_p[q]`) and FTRAN's column-direction value (`alpha_col`,
/// `alpha_buf[p]`) — see [`UPDATE_VERIFY_TOL`]'s own docs for why these two
/// independently-computed scalars are expected to agree, and what a
/// growing gap between them means. Both arguments must already be the
/// *same* pivot's two values — the caller is responsible for reading them
/// at the right point in the iteration (right after FTRAN produces
/// `alpha_buf`, before anything derived from it is committed).
///
/// Returns `true` ("healthy — proceed with this pivot") or `false` ("this
/// pivot can no longer be trusted here; refactorize before committing
/// it"). Selection rules (`chuzr`/`chuzc`/BFRT/Harris/DSE/Devex) are never
/// touched by this check — by the time it runs, `p`/`q`/`theta_q` are
/// already fully decided; this only ever changes *whether a refactorization
/// happens sooner*, never which pivot is chosen.
///
/// `pub(super)`: `extended_dual::solve_lp_dual_extended` reuses this exact
/// function and [`UPDATE_VERIFY_TOL`] for its own pivot element (`a_p[q]`
/// vs `alpha_full[r]`) — the agreement this checks for has no
/// `Affine1`-vs-`f64` dependency at all (both values it compares are
/// always `M`-independent structural quantities), so there is nothing for
/// that module to generalize here, only to call.
#[inline]
pub(super) fn update_verify(alpha_row: f64, alpha_col: f64) -> bool {
    let scale = alpha_row.abs().max(alpha_col.abs()).max(tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64));
    let rel = (alpha_row - alpha_col).abs() / scale;
    rel <= UPDATE_VERIFY_TOL
}

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

/// Bland's-rule (1977) last-resort anti-cycling fallback for the primal
/// method — the same role `extended_dual`'s own `stall_count`/
/// `bland_mode` locals play for the dual method (see that state's own
/// docs for the full rationale), ported here once a real degenerate
/// Netlib instance (`cycle`) showed that EXPAND alone, plus
/// [`PRIMAL_HARRIS_TOL`]'s pivot-conditioning widening, still isn't
/// always enough: a long-enough run of essentially-zero-progress pivots
/// can still walk the basis into a state `run_phase`'s own mid-solve
/// refactorization finds numerically singular. `stall_count` counts
/// consecutive pivots whose actual contribution to the objective
/// (`theta * dj` of the entering variable — this method's analogue of the
/// dual method's `theta_q * dj_q`) is below [`STALL_PROGRESS_EPS`]; once
/// it exceeds `stall_limit` (scaled to problem size the same way the dual
/// method's own `stall_limit` is), `bland_mode` latches on for the rest
/// of the solve. Persists across the phase-1/phase-2 boundary like
/// [`ExpandState`] does, for the same reason: a stalling run spanning the
/// boundary shouldn't get its counter reset back to zero for free.
struct PrimalStallState {
    stall_count: usize,
    bland_mode: bool,
}

impl PrimalStallState {
    fn new() -> Self {
        PrimalStallState { stall_count: 0, bland_mode: false }
    }
}

/// Deterministic membership test for partial pricing's first-pass sample:
/// roughly one column in [`PARTIAL_PRICING_GROUPS`] is "in the sample" for
/// a given `(seed, iter, j)`. A pure function of its inputs rather than a
/// `&mut` RNG threaded through the pricing loop, so the exact same call
/// answers both "is this column in the sample" (first pass) and "is this
/// column in the rest" (`!partial_pricing_sampled(..)`, second pass)
/// without needing to record which columns the first pass actually visited
/// — and results stay reproducible run-to-run for the same problem, unlike
/// a time-seeded RNG, which matters for this crate's cycling/regression
/// tests. `seed` varies the pricing's columns across problem instances
/// (`n_total`-only-distinct LPs would otherwise always sample the same
/// columns), while `iter` varies the sampled set pivot-to-pivot so a
/// column that loses the draw one iteration isn't permanently excluded.
/// Bit-mixing is splitmix64's finalizer (Steele, Lea & Flood 2014), chosen
/// only for being cheap and adequately unbiased here, not for any
/// cryptographic property.
#[inline]
fn partial_pricing_sampled(seed: u64, iter: u64, j: usize) -> bool {
    let mut z = seed ^ iter.wrapping_mul(0x9E3779B97F4A7C15) ^ (j as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58476D1CE4E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    z % PARTIAL_PRICING_GROUPS == 0
}

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
            let norm_sq: f64 = std.cols.col(j).iter().map(|&(_, v)| v * v).sum();
            gamma[j] = norm_sq.max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
        }
        SteepestEdgeState { gamma }
    }

    /// Applies the Forrest-Goldfarb weight update after a pivot at basis
    /// slot `r` that brought in `enter` (whose weight *before* the pivot
    /// was `gamma_t_old`), given `rho` = row `r` of `B^-1` and `w` =
    /// `B^-T alpha` — both computed against the basis as it stood
    /// *before* the pivot — and `pivot = alpha[r]`. Must be called with
    /// `t` already reflecting the *post*-pivot basis/nonbasic status.
    fn update_after_pivot(&mut self, t: &Tableau, std: &StdForm, rho: &[f64], w: &[f64], gamma_t_old: f64, pivot: f64) {
        // Every column's updated weight only depends on its own (previous)
        // old weight plus `rho`/`w`/`gamma_t_old`/`pivot`, all read-only
        // here, so these writes to `self.gamma` are disjoint per column —
        // safe to parallelize. Left sequential anyway: for the column
        // counts this crate actually sees, rayon's per-call dispatch
        // overhead measured *larger* than the loop body itself (see
        // this module's own docs for the profiling that found
        // this — the same reasoning applies to every small, high-frequency
        // per-iteration loop in this file, not just that one).
        for (j, gamma_j) in self.gamma.iter_mut().enumerate() {
            // A fixed column (`lb[j] == ub[j]`) never wins `price_one`'s
            // entering-variable scan (see that closure's own skip) no
            // matter what its weight is, so recomputing that weight every
            // single pivot is pure waste — same reasoning as the dual
            // method's PRICE-loop skip just above `chuzc1`.
            if t.nb_status[j].is_none() || std.lb[j] == std.ub[j] {
                continue;
            }
            let col = t.column_sparse(j);
            let pivot_sj: f64 = col.iter().map(|&(i, v)| v * rho[i]).sum();
            let tau_j: f64 = col.iter().map(|&(i, v)| v * w[i]).sum();
            let beta_j = pivot_sj / pivot;
            *gamma_j = (*gamma_j + beta_j * beta_j * (1.0 + gamma_t_old) - 2.0 * beta_j * tau_j).max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NbStatus {
    Lower,
    Upper,
    /// Nonbasic at value `0` — the paper's state `Z` (\S2.2), for a
    /// genuinely free column (`lb == -inf` **and** `ub == +inf`) only, and
    /// only ever produced by [`extended_dual`] (its crash, for a zero-cost
    /// free column, and its cleanup lemma's case (A)). Dual feasible iff
    /// the reduced cost is exactly `0`; eligible to enter in either
    /// direction; never flipped (its width is infinite). This module's own
    /// classical paths never create it, but [`run_phase`] can inherit one
    /// through `extended_dual`'s primal handoff, so every match handles it.
    Zero,
}

pub struct SimplexResult {
    pub status: Status,
    pub x: Option<Vec<f64>>,
}

/// Builds the `[A x + s = b]` standard form described above directly from
/// the model's variables/objective/constraints (mirrors `qp::build`, but
/// produces one slack column per row instead of folding bounds into `G`).
///
/// `cols` is `rows` transposed — `cols.col(j)` is column `j`'s own `(row,
/// value)` pairs — built once ([`CscMat::from_rows`], a single O(nnz)
/// counting sort straight into the flat layout) right after `rows` is
/// finalized and never touched again: `StdForm` itself is never mutated
/// during a solve (only `Tableau`'s basis/nonbasic status and `x` change),
/// so there is no risk of the two views drifting out of sync. It exists
/// purely so `Tableau::column`/`column_sparse` never have to scan every
/// row looking for column `j` — see their own docs for why that mattered.
///
/// The pair are this crate's own [`CsrMat`]/[`CscMat`] — one flat `(index,
/// value)` buffer plus offsets each, per `crate::sparse`'s own docs —
/// rather than `Vec<Vec<(usize, f64)>>`: once presolve hands off the final
/// matrix here, it is read every pivot for the rest of the solve and never
/// mutated again, so there is no reason to keep paying for one separate
/// heap allocation per row/column the way a still-being-rewritten presolve
/// pass does. Row order within each `cols.col(j)` is ascending; nothing
/// downstream (dot products, densifying one column) depends on it either
/// way.
struct StdForm {
    n_total: usize,
    n_rows: usize,
    c: Vec<f64>,
    rows: CsrMat, // sparse rows over the n_total columns
    cols: CscMat, // `rows` transposed: cols.col(j) = column j's (row, value) pairs
    b: Vec<f64>,
    lb: Vec<f64>,
    ub: Vec<f64>,
}

/// Freezes `rows` into the [`CsrMat`]/[`CscMat`] pair a [`StdForm`] holds,
/// in one place for all three construction sites.
///
/// The `debug_assert` is load-bearing documentation, not a paranoia check:
/// every walk that reaches the basis *through the column view* (
/// [`Tableau::basis_rows_sparse`], [`Tableau::basis_residual_norm`],
/// `extended_dual::refactorize`, `extended_dual::residual_norm`) visits
/// columns in ascending index and therefore reproduces each row's entry
/// order — and so the LU's own pivot-order tie-breaks, and each residual's
/// summation order — bit for bit, *provided* the rows were column-ascending
/// to begin with. All three builders do produce that (a presolved row's
/// structural terms come out of an ascending faer CSR row through a
/// monotone re-index, and its slack is appended last with the largest
/// index of all); this asserts it rather than leaving it to be rediscovered.
fn freeze_std_matrices(rows: &[Vec<(usize, f64)>], n_total: usize) -> (CsrMat, CscMat) {
    debug_assert!(
        rows.iter().all(|r| r.windows(2).all(|w| w[0].0 < w[1].0)),
        "StdForm rows must be strictly column-ascending"
    );
    (CsrMat::from_rows(rows, n_total), CscMat::from_rows(rows, n_total))
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

    let (rows, cols) = freeze_std_matrices(&rows, n_total);
    StdForm { n_total, n_rows, c, rows, cols, b, lb, ub }
}

/// Runs the shared presolve pipeline (`crate::presolve`: Ruiz scaling,
/// redundant-equality removal, then inequality propagation) and builds
/// the resulting `StdForm` from *its* output, instead of from the model's
/// variables/constraints directly (contrast [`build_std_form`] above).
///
/// `A`'s rows (after redundant-equality removal) become `Eq` rows and
/// `G`'s remaining multi-variable rows (after propagation) become `Le`
/// rows — both via the exact same slack-per-row convention
/// [`build_std_form`] already uses, so equality constraints get no new or
/// different kind of variable here than they always have (a slack fixed
/// at `[0, 0]`, feasibility for which is still entirely phase 1's job, or
/// the dual-feasible crash's for [`solve_lp_dual`] — this function itself
/// constructs no basis). `G`'s single-variable rows, by contrast, are
/// pulled back out as `StdForm`'s own `lb`/`ub` via
/// `presolve::propagate::extract_bounds` (the same extraction
/// `propagate` already does internally) rather than becoming constraint
/// rows with their own slack: a variable's bound is represented as a
/// bound, not as an extra artificial/slack variable.
///
/// Returns `None` if presolve alone already proves the problem infeasible
/// (a propagated row whose activity bound can never be satisfied) — the
/// caller reports `Status::Infeasible` directly, without ever building a
/// `Tableau`.
///
/// Runs `presolve::run_extended` (the same shared pipeline
/// `interior_point.rs` now also uses — see that function's own docs)
/// rather than `colsingleton` alone: every substitution it returns is now
/// in *scaled* coordinates (colsingleton used to run once, pre-scaling,
/// specifically to avoid this — see `unscale_result`'s own docs for how
/// recovery order changes to match).
/// [`build_std_form_presolved`]'s return value: the presolved, *compacted*
/// [`StdForm`] (see that function's own docs for why every fixed — not just
/// substituted — structural variable is now excluded from it entirely,
/// rather than kept as an always-skipped `lb[j] == ub[j]` slot), plus
/// everything a caller needs to expand a solve's output back into the
/// original `variables.len()`-length `x`: `orig_of_free[nj]` is the
/// original variable index that compacted structural column `nj` stands
/// for, and `fixed_values` is every excluded variable's own `(original
/// index, value)` pair — a plain constant for a `dualfix`/forcing-row
/// fixed variable, and the doubleton/colsingleton sentinel `0.0` for one
/// [`presolve::colsingleton::Substitution::value`] will overwrite right
/// afterward. `sign[nj]` is `orig_of_free[nj]`'s own contribution sign —
/// always `1.0`, except `-1.0` for a column the shift step reflected (see
/// `sign`'s own field docs) — `orig_of_free.len() + fixed_values.len() ==
/// variables.len()` always holds (one compacted slot per surviving
/// column).
///
/// `shift[nj]` is how far column `nj`'s *finite* bound was translated
/// toward `0` (see the bound-shift step in [`build_std_form_presolved`]
/// itself) — `std.lb[nj]`/`std.ub[nj]` are expressed in this shifted
/// coordinate system, so recovering the true (still-scaled) value of
/// column `nj` needs `x_free[nj] + shift[nj]`, done once in
/// [`unscale_result`] before any substitution's own `value()` call reads
/// it back out. Always `0.0` for a genuinely free column (no finite side
/// to shift toward at all — see the shift step's own docs).
struct PresolvedForm {
    std: StdForm,
    scaling: scaling::Scaling,
    /// One shared chronological log of every eliminating presolve step —
    /// see [`presolve::ExtendedPresolveResult::postsolve_log`]'s own docs
    /// for why this must stay a single interleaved log (resolved by
    /// [`unscale_result`] in one reverse pass) rather than two separate
    /// per-kind lists.
    postsolve_log: Vec<presolve::PostsolveStep>,
    orig_of_free: Vec<usize>,
    /// `sign[nj] * (x_free[nj] + shift[nj])` is `nj`'s own contribution to
    /// `orig_of_free[nj]`'s true value — see [`PresolvedForm`]'s own docs.
    /// `-1.0` for a slot the shift step reflected (a column unbounded only
    /// below — see that step's own docs), `1.0` otherwise — the
    /// row-building loops multiply each such column's own coefficients by
    /// `sign[nj]` too, so this recovery formula and the `std` the solver
    /// actually sees stay consistent with each other.
    sign: Vec<f64>,
    fixed_values: Vec<(usize, f64)>,
    shift: Vec<f64>,
}

fn build_std_form_presolved(
    variables: &[VariableData],
    objective: &Objective,
    constraints: &[ConstraintRow],
    allow_unbounded_verdict: bool,
) -> Result<PresolvedForm, Status> {
    let n = variables.len();
    let sign = match objective.sense {
        Sense::Minimize => 1.0,
        Sense::Maximize => -1.0,
    };
    let mut c0 = vec![0.0; n];
    for (&j, &v) in objective.expr.coeffs.iter() {
        c0[j] = sign * v;
    }

    let (a, b, g, h) = presolve::build_a_g(variables, constraints);

    let pre = presolve::run_extended(
        n,
        &a,
        &b,
        &g,
        &h,
        &c0,
        tunable!("ENOMOTO_T_RUIZ_ITERS", RUIZ_ITERS, usize),
        tunable!("ENOMOTO_T_PROPAGATION_PASSES", PROPAGATION_PASSES, usize),
        tunable!("ENOMOTO_T_PRESOLVE_ROUNDS", PRESOLVE_ROUNDS, usize),
        tunable!("ENOMOTO_T_INNER_ROUNDS", ROWSINGLETON_COLSINGLETON_INNER_ROUNDS, usize),
        allow_unbounded_verdict,
    );
    if env_str!("ENOMOTO_DEBUG_PRESOLVE_INFEAS").is_some() {
        eprintln!("DEBUG_PRESOLVE: infeasible={} unbounded={}", pre.infeasible, pre.unbounded);
    }
    if pre.infeasible {
        return Err(Status::Infeasible);
    }
    if pre.unbounded {
        // `presolve::freevar::eliminate_free_variables` found an improving
        // ray (a free variable whose cost pushes it toward an unconstrained
        // side) — that proves `z^1 < 0`, i.e. no finite optimum, but not
        // that the rest of the problem is feasible. Only reachable with
        // `allow_unbounded_verdict` (the default, non-distinguishing mode).
        return Err(Status::InfeasibleOrUnbounded);
    }

    // `pre.lb`/`pre.ub`/`pre.real_rows`/`pre.real_rhs` are the box bounds
    // and genuine multi-variable inequality rows `propagate::propagate`
    // already split apart internally — reused directly instead of
    // re-deriving them with a second `extract_bounds` call on `pre.g`/
    // `pre.h` (which would just be undoing the row-folding `run_extended`
    // did to produce them in the first place).
    let (mut lb, mut ub, g_rows, g_rhs) = (pre.lb, pre.ub, pre.real_rows, pre.real_rhs);
    // Every variable eliminated by `doubleton`/`colsingleton` inside
    // `presolve::run_extended` deliberately has *no* remaining bound rows
    // of its own (see `doubleton`'s "Surviving variables" loop and this
    // module's own removal of `colsingleton`'s stale ones) — its true
    // value is recovered later purely via `Substitution::value`, never
    // read off the solve directly. But every other piece of this file
    // (the dual-feasible crash, `Tableau::new`'s nonbasic-at-lower-bound
    // start, the BFRT walk) assumes every variable has two *finite*
    // bounds — a genuinely infinite pair here is a phantom column the
    // solver was never designed to represent, and was observed causing
    // the dual method to report a false `Infeasible` on otherwise-
    // feasible problems (confirmed by cross-checking against the primal
    // method, which reached `Optimal` on the identical `StdForm`). Fixed
    // to a single arbitrary finite point — `0` needs no justification
    // beyond "finite and never read" — before this variable's slot ever
    // reaches the solver.
    for step in &pre.postsolve_log {
        if let presolve::PostsolveStep::Sub(sub) = step {
            lb[sub.var] = 0.0;
            ub[sub.var] = 0.0;
        }
    }

    // Bound-shift: translate every surviving structural variable (every
    // `j` with `lb[j] != ub[j]` at this point — a fixed/substituted one is
    // never part of the solve at all, see `new_index` below) so its own
    // *genuine* finite bound sits at exactly `0`, simplex-only (this
    // function has no `interior_point.rs` caller — that engine keeps
    // reading `pre.g`/`pre.h` straight off `run_extended`, unshifted) and
    // done exactly once, right here, on the fully presolved/propagated
    // `lb`/`ub` `run_extended` just returned — never re-run mid-presolve.
    // A boxed variable (`lb`/`ub` both finite) shifts toward its lower
    // bound; a one-sided variable shifts toward whichever bound is
    // genuinely finite (its *only* finite bound is exactly the one the
    // dual-feasible crash / bound-flip logic below will park it at
    // nonbasic — it can never rest at the infinite side); a genuinely free
    // variable (both infinite) gets no shift, since it has no finite bound
    // to anchor to. This is purely a change of coordinate
    // origin per column (`x_j = x'_j + shift[j]`) — it changes neither the
    // feasible region's shape nor the objective's linearity, only which
    // point in it reads as `0`; every row referencing a shifted column
    // gets its own rhs adjusted to match (see the two row-building loops
    // below), and [`unscale_result`] adds `shift` back before any
    // substitution reads a shifted column's true scaled value.
    // A one-sided column unbounded *below* only (`lb[j]==-inf`, `ub[j]`
    // finite) is reflected (`x_j = ub[j] - y_j`, `y_j >= 0`) before the
    // ordinary shift below ever runs, turning it into the exact same shape
    // as a naturally upper-unbounded column (`[0, +inf)`, nonbasic-at-
    // lower, `extended_dual::delta_of` returns `delta_j = +1.0`) instead of
    // its own natural `(-inf, 0]` (nonbasic-at-upper, `delta_j = -1.0`).
    // Every M-tracked column ends up with the same direction and the same
    // initial nonbasic value of exactly `0`, at the cost of negating this
    // column's row/objective coefficients (folded into `refl_sign`, reused
    // as this slot's `sign[nj]` below — the exact same
    // `sign[nj] * (x_free[nj] + shift[nj])` recovery the free-column split
    // already relies on, generalized from `{+1, -1}` split-halves to a
    // single reflected slot). Confirmed as a real, correctness-neutral win
    // by a full 93-problem Netlib sweep (2026-09-21): every problem's
    // status and objective matched the un-reflected baseline exactly,
    // total solve time -3.75% (62.21s -> 59.88s), and the single largest
    // problem (`dfl001`, 36s+) improved -5.3% with no large problem
    // regressing. A doubly-infinite column (`ub[j]` also infinite) is left
    // untouched — that's the free-variable split's own case just below.
    let mut refl_sign = vec![1.0; n];
    let mut shift = vec![0.0; n];
    for j in 0..n {
        if lb[j] == ub[j] {
            continue;
        }
        if lb[j] == f64::NEG_INFINITY && ub[j].is_finite() {
            let u = ub[j];
            refl_sign[j] = -1.0;
            lb[j] = -u;
            ub[j] = f64::INFINITY;
        }
        let s = if lb[j].is_finite() {
            lb[j]
        } else if ub[j].is_finite() {
            ub[j]
        } else {
            0.0
        };
        shift[j] = s;
        lb[j] -= s;
        ub[j] -= s;
    }

    // Structural columns that still carry a genuine `+/-inf` bound here
    // are left infinite: `extended_dual::solve_lp_dual_extended` tracks
    // them symbolically (see that module's own docs). Usually one-sided
    // (free variables reachable through an equality row are gone by now,
    // see `presolve::freevar`'s own docs), but a free variable appearing
    // only in inequality rows can still arrive here both-sided.
    if env_str!("ENOMOTO_DEBUG_UNBOUNDED_VARS").is_some() {
        let count = (0..n).filter(|&j| lb[j] == f64::NEG_INFINITY || ub[j] == f64::INFINITY).count();
        if count > 0 {
            eprintln!("PRESOLVE: {count} structural column(s) still have a genuine infinite bound");
        }
    }

    // Every structural variable with `lb[j] < ub[j]` gets a compacted slot
    // `new_index[j] = Some((nj, None))`; every `lb[j] == ub[j]` one
    // (doubleton/colsingleton substitution sentinel, or a `dualfix`/
    // forcing-row real fixed value — pricing already can't tell, and
    // doesn't need to, see `price_one`'s own `lb[j] == ub[j]` skip) is
    // dropped from the solve's column space entirely rather than kept as a
    // slot every column-oriented loop still has to check-and-skip and
    // every row-oriented loop (the dual simplex's own PRICE step,
    // expanding a touched row's full nonzero list) still has to
    // read-and-discard on every pivot for the rest of the solve. Its
    // contribution to any row it appears in is folded into that row's own
    // right-hand side below instead — the same arithmetic
    // `x_B = B^{-1}(b - N x_N)` already did with this column included in
    // `N` at its bound, just performed once here instead of on every basis
    // (re)computation for the life of the solve.
    //
    // A column still genuinely free on *both* sides here (`lb[j]==-inf &&
    // ub[j]==+inf` — `presolve::freevar`'s own documented residual case, a
    // free variable that appears only in an inequality row and so cannot
    // be soundly eliminated by that module alone) gets exactly the same
    // single compacted slot as any other surviving column — no more
    // `x_j = x_j^+ - x_j^-` split: `extended_dual::hat_lower`/`hat_upper`
    // track a genuinely free column's *both* sides symbolically (`M` on
    // each), so this module no longer needs to represent it with two
    // one-sided-bounded halves (see that module's own docs for why, and
    // its `finish`'s own docs for the one residual case — two free columns
    // coupled *only* to each other, with no third anchoring either — it
    // still can't resolve internally and reports `Status::NotSolved` for,
    // same as every other "should be unreachable" guard in that module).
    let mut new_index: Vec<Option<usize>> = vec![None; n];
    let mut orig_of_free: Vec<usize> = Vec::new();
    let mut sign: Vec<f64> = Vec::new();
    let mut slot_lb: Vec<f64> = Vec::new();
    let mut slot_ub: Vec<f64> = Vec::new();
    for j in 0..n {
        if lb[j] == ub[j] {
            continue;
        }
        let nj = orig_of_free.len();
        orig_of_free.push(j);
        sign.push(refl_sign[j]);
        slot_lb.push(lb[j]);
        slot_ub.push(ub[j]);
        new_index[j] = Some(nj);
    }
    let n_free = orig_of_free.len();
    let fixed_values: Vec<(usize, f64)> = (0..n).filter(|&j| new_index[j].is_none()).map(|j| (j, lb[j])).collect();

    let n_eq = pre.a.nrows();
    let n_le = g_rows.len();
    let n_rows = n_eq + n_le;
    let n_total = n_free + n_rows;

    let mut c = vec![0.0; n_total];
    for (nj, &j) in orig_of_free.iter().enumerate() {
        c[nj] = sign[nj] * pre.c[j];
    }

    let mut new_lb = vec![0.0; n_total];
    let mut new_ub = vec![0.0; n_total];
    new_lb[..n_free].copy_from_slice(&slot_lb);
    new_ub[..n_free].copy_from_slice(&slot_ub);

    let mut b_out = Vec::with_capacity(n_rows);
    // The rows go straight into one flat CSR buffer (row `k` =
    // `entries[offsets[k]..offsets[k + 1]]`) rather than one `Vec` per
    // row, and the column form is transposed from it — the same matrices,
    // entry for entry, that `freeze_std_matrices` builds from the
    // equivalent `Vec<Vec<_>>`.
    let nnz_bound = pre.a.as_ref().compute_nnz() + g_rows.iter().map(|r| r.len()).sum::<usize>() + n_rows;
    let mut offsets: Vec<usize> = Vec::with_capacity(n_rows + 1);
    offsets.push(0);
    let mut entries: Vec<(usize, f64)> = Vec::with_capacity(nnz_bound);

    for i in 0..n_eq {
        let slack = n_free + i;
        let mut rhs_i = pre.b[i];
        for (j, v) in csr_row_iter(&pre.a, i) {
            match new_index[j] {
                Some(nj) => {
                    entries.push((nj, v * sign[nj]));
                    rhs_i -= v * sign[nj] * shift[j];
                }
                None => rhs_i -= v * lb[j],
            }
        }
        new_lb[slack] = 0.0;
        new_ub[slack] = 0.0;
        entries.push((slack, 1.0));
        offsets.push(entries.len());
        b_out.push(rhs_i);
    }
    for (k, row) in g_rows.into_iter().enumerate() {
        let slack = n_free + n_eq + k;
        let mut rhs_k = g_rhs[k];
        for (j, v) in row {
            match new_index[j] {
                Some(nj) => {
                    entries.push((nj, v * sign[nj]));
                    rhs_k -= v * sign[nj] * shift[j];
                }
                None => rhs_k -= v * lb[j],
            }
        }
        new_lb[slack] = 0.0;
        new_ub[slack] = f64::INFINITY;
        entries.push((slack, 1.0));
        offsets.push(entries.len());
        b_out.push(rhs_k);
    }

    debug_assert!(
        offsets.windows(2).all(|w| entries[w[0]..w[1]].windows(2).all(|e| e[0].0 < e[1].0)),
        "StdForm rows must be strictly column-ascending"
    );
    let rows = CsrMat::from_flat(n_total, offsets, entries);
    let cols = rows.to_csc();
    let shift_of_free: Vec<f64> = orig_of_free.iter().map(|&j| shift[j]).collect();
    Ok(PresolvedForm {
        std: StdForm { n_total, n_rows, c, rows, cols, b: b_out, lb: new_lb, ub: new_ub },
        scaling: pre.scaling,
        postsolve_log: pre.postsolve_log,
        orig_of_free,
        sign,
        fixed_values,
        shift: shift_of_free,
    })
}

/// Fills in every `doubleton`/`colsingleton`-eliminated variable's true
/// value via [`presolve::colsingleton::Substitution::value`] — in
/// **reverse** discovery order (see [`presolve::ExtendedPresolveResult`]'s
/// own docs for why) — and *before* `scaling::unscale_x`, not after: since
/// [`build_std_form_presolved`] now runs every elimination inside
/// `presolve::run_extended`'s already-scaled pipeline, a substitution's
/// `terms`/`rhs`/`coeff` are themselves scaled-space values, so `value()`
/// must be evaluated against the still-scaled solve output — each
/// eliminated variable's own recovered value is exactly as scaled as
/// every other entry at that point, so the same single `unscale_x` call
/// at the end correctly converts the whole vector, substituted entries
/// included. `Infeasible`/`Unbounded` pass through unchanged (there is no
/// `x` to fix up).
///
/// `shift[nj]` is added back in the very same expansion step, before any
/// substitution's own `value()` call runs: `x_free[nj]` is column `nj`'s
/// value in [`build_std_form_presolved`]'s shifted coordinates, but a
/// substitution's `terms`/`rhs`/`coeff` were computed in `run_extended`'s
/// (unshifted) scaled space, so `sub.value(&x)` needs the true scaled
/// value at every index it reads, shifted columns included.
///
/// `n` is the original `variables.len()` — always equal to
/// `orig_of_free.len() + fixed_values.len()` (every surviving column gets
/// exactly one compacted slot, [`build_std_form_presolved`]'s own docs),
/// but passed explicitly rather than derived from that sum so this
/// function doesn't need to recompute it.
fn unscale_result(
    result: SimplexResult,
    sc: &scaling::Scaling,
    postsolve_log: &[presolve::PostsolveStep],
    orig_of_free: &[usize],
    sign: &[f64],
    fixed_values: &[(usize, f64)],
    shift: &[f64],
    n: usize,
) -> SimplexResult {
    match result.status {
        Status::Optimal => {
            let x_free = result.x.unwrap();
            // Expand the compacted solve's output (one entry per surviving
            // structural column, see `PresolvedForm`'s own docs) back into
            // the original `variables.len()`-length space *before* the
            // postsolve loop below: a substitution's own `terms` can
            // reference a variable that `dualfix`/a forcing row fixed
            // outright (not one this loop itself resolves), so every fixed
            // value must already be in place at its original index by the
            // time `sub.value(&x)` reads it.
            let mut x = vec![0.0; n];
            for (nj, &j) in orig_of_free.iter().enumerate() {
                x[j] = sign[nj] * (x_free[nj] + shift[nj]);
            }
            for &(j, v) in fixed_values {
                x[j] = v;
            }
            // One reverse pass over the *shared* chronological log — see
            // `presolve::ExtendedPresolveResult::postsolve_log`'s own docs
            // for why this must not be two separate per-kind passes: a
            // `Sub` recorded before a later `ParallelCol` merge can
            // reference the merge's own `kept` column, so that merge's own
            // `apply` must already have run (restoring `kept`'s true
            // pre-merge value) by the time this `Sub`'s `value()` reads it.
            for step in postsolve_log.iter().rev() {
                match step {
                    presolve::PostsolveStep::Sub(sub) => x[sub.var] = sub.value(&x),
                    presolve::PostsolveStep::ParallelCol(sub) => sub.apply(&mut x),
                }
            }
            let x = scaling::unscale_x(sc, &x);
            SimplexResult { status: Status::Optimal, x: Some(x) }
        }
        other => SimplexResult { status: other, x: None },
    }
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
    #[cfg(test)]
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
        // Column-driven, via `std.cols`: this touches only `nnz(A_B)` —
        // the basis's own entries — where the row-driven form it replaced
        // scanned all `nnz(A)` and discarded every nonbasic entry it read.
        // On a real instance most columns are nonbasic at any one time, so
        // that discarded work was the bulk of it. Ascending `j` keeps each
        // `rows[i]` in exactly the order the row-driven scan produced (see
        // `freeze_std_matrices`), so the LU factorization this feeds is
        // bit-for-bit the same one.
        for j in 0..self.std.n_total {
            if let Some(col) = self.basis_pos[j] {
                for &(i, v) in self.std.cols.col(j) {
                    rows[i].push((col, v));
                }
            }
        }
        rows
    }

    /// Column `j` of the full constraint matrix, densified from
    /// `std.cols.col(j)` — O(nnz_j + n_rows), not a scan of every row
    /// looking for column `j`. Both real per-iteration call sites
    /// (`run_phase`, the dual loop) now go through the
    /// non-allocating [`Self::column_into`] instead — this allocating
    /// form only remains for tests that want a plain `Vec` without a
    /// buffer to hand.
    #[cfg(test)]
    fn column(&self, j: usize) -> Vec<f64> {
        let m = self.std.n_rows;
        let mut col = vec![0.0; m];
        self.column_into(j, &mut col);
        col
    }

    /// Same as [`Self::column`] but writing into a caller-provided buffer
    /// (length `n_rows`) instead of allocating — the dual loop calls
    /// this once every pivot for the entering column, so a fresh `Vec`
    /// here would be one more per-iteration heap allocation on top of the
    /// FTRAN/BTRAN ones `FtLu::solve_into`/`solve_transpose_into` already
    /// eliminate.
    fn column_into(&self, j: usize, out: &mut [f64]) {
        for v in out.iter_mut() {
            *v = 0.0;
        }
        for &(i, v) in self.std.cols.col(j) {
            out[i] = v;
        }
    }

    /// Column `j`'s sparse `(row, value)` pairs directly, with no O(m)
    /// densification at all. Every per-candidate-column loop that only
    /// ever computes a dot product against column `j` (entering-variable
    /// selection, the dual method's `chuzc`, steepest-edge weight
    /// updates) should use this instead of `column` — those loops run
    /// once per nonbasic column *every pivot*, so avoiding both the O(m)
    /// fill and ever touching another column's data is what turns an
    /// O(n_total * nnz) pivot into an O(nnz) one.
    fn column_sparse(&self, j: usize) -> &[(usize, f64)] {
        self.std.cols.col(j)
    }

    /// Recomputes every basic variable's value from the current nonbasic
    /// values by solving `B x_B = b - N x_N`. Returns the reduced RHS
    /// (`b - N x_N`) so the caller can cheaply check the true-basis
    /// residual without an extra solve.
    fn recompute_basics(&mut self, lu: &sparse_lu::FtLu) -> Vec<f64> {
        let rhs = self.compute_rhs();
        self.resync_basics(lu, &rhs);
        rhs
    }

    /// `b - N x_N` from the current nonbasic assignment, without touching
    /// `x_B` or doing the triangular solve. Split out so the dual method's
    /// own loop can get this ground-truth `rhs` for its
    /// periodic drift *check* without that check itself silently masking
    /// the very drift it's supposed to detect by resyncing `x_B` first —
    /// see that loop's own docs for why `x_B` is otherwise maintained
    /// incrementally, not recomputed here every iteration.
    ///
    /// Column-major over nonbasic columns, skipping any with `x[j] == 0.0`
    /// entirely (never touching that column's own nonzeros at all), rather
    /// than the row-major `for i, for (j, v) in row(i), if nonbasic` scan
    /// this replaces (which paid one `nb_status` check per matrix entry
    /// regardless of `x[j]`, `O(nnz(A))` unconditionally). A nonbasic
    /// column sitting at a bound of exactly `0` is common even without any
    /// special handling (`0` is the default/most common variable lower
    /// bound in LP models generally), and [`build_std_form_presolved`]'s
    /// own bound-shift step (see its docs) widens that set further by
    /// translating every other surviving column's *finite* bound to `0`
    /// too — so this is worth the skip rather than an `O(nnz(A))` floor
    /// this function can never beat regardless of how `x` is distributed.
    fn compute_rhs(&self) -> Vec<f64> {
        let mut rhs = self.std.b.clone();
        for j in 0..self.std.n_total {
            if self.nb_status[j].is_none() {
                continue;
            }
            let xj = self.x[j];
            if xj == 0.0 {
                continue;
            }
            for &(i, v) in self.std.cols.col(j) {
                rhs[i] -= v * xj;
            }
        }
        rhs
    }

    /// The solve half of `recompute_basics`: given an already-computed
    /// `rhs` (from `compute_rhs`), resolves `B x_B = rhs` against `lu` and
    /// overwrites `x_B` with the result — the same "snap back to ground
    /// truth" refresh `d`'s own `fresh_d` gets, at the same cadence (right
    /// after a refactorization actually happens, not every iteration).
    fn resync_basics(&mut self, lu: &sparse_lu::FtLu, rhs: &[f64]) {
        let sol = lu.solve(rhs);
        for i in 0..self.std.n_rows {
            self.x[self.basis[i]] = sol[i];
        }
    }

    /// `‖A_B x_B - rhs‖`, using the *true* (not LU-derived) basis matrix
    /// against the already-recomputed `self.x` — refactorization trigger
    /// (1)'s numerical-drift check on the incrementally-updated LU.
    fn basis_residual_norm(&self, rhs: &[f64]) -> f64 {
        let m = self.std.n_rows;
        // `A_B x_B` accumulated one *basic column* at a time (see
        // `basis_rows_sparse`'s own note): `nnz(A_B)` work plus one `O(m)`
        // buffer, rather than a full `nnz(A)` scan that reads every
        // nonbasic entry only to skip it.
        let mut val = vec![0.0; m];
        for j in 0..self.std.n_total {
            if self.nb_status[j].is_some() {
                continue;
            }
            sparse_axpy_dense(self.x[j], self.std.cols.col(j), &mut val);
        }
        let mut resid_sq = 0.0;
        for i in 0..m {
            let r = val[i] - rhs[i];
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
                Some(NbStatus::Zero) => {
                    if self.x[j].abs() < EXPAND_DELTA_F {
                        self.x[j] = 0.0;
                    }
                }
                None => {}
            }
        }
    }

}

/// Cost perturbation for the dual method's anti-degeneracy — the dual
/// analog of the primal method's EXPAND, but a genuinely different
/// technique, not a direct port: confirmed against HiGHS's own source
/// (`HEkk::initialiseCost`, `HEkk.cpp`), which perturbs costs for exactly
/// this reason rather than using anything EXPAND-shaped for its dual
/// simplex. EXPAND relaxes *primal* bounds because primal degeneracy
/// (a tied ratio test) is what risks cycling there; the dual method's
/// analogous risk is *dual* degeneracy — tied ratios in chuzc1/BFRT,
/// already partly addressed by `HARRIS_RATIO_TOL` — but reduced costs
/// have no bounds of their own for an EXPAND-style relaxation to widen.
/// Perturbing the cost vector once, before any reduced cost is ever
/// computed, generically avoids exact ties from the first iteration
/// instead of trying to detect and route around them later.
///
/// Follows HiGHS's algorithm directly: perturbation magnitude scales with
/// `max_abs_cost` (damped if very large, or capped at 1 if almost nothing
/// is boxed), and each column's own perturbation is proportional to its
/// own cost magnitude (so a zero-cost column still gets a small nudge)
/// times a per-column pseudo-random factor in `[1, 2)`, applied in
/// whichever direction keeps it from ever *flipping* which side of dual
/// feasibility the column sits on: fixed and free columns are left alone
/// (a free column's reduced cost must be exactly 0 regardless; a fixed
/// one's sign never matters), a one-sided column is nudged away from the
/// missing bound, and a genuinely boxed column is nudged further in
/// whichever sign its own cost already has.
///
/// Deterministic per-column pseudo-randomness (a cheap integer hash of
/// the column index, not a seeded RNG) stands in for HiGHS's own
/// `numTotRandomValue_` array — this only needs "generically distinct"
/// values, and determinism keeps a solve reproducible.
fn perturb_costs(std: &StdForm) -> Vec<f64> {
    let n = std.n_total;
    let mut max_abs_cost = std.c.iter().fold(0.0f64, |acc, &c| acc.max(c.abs()));
    if max_abs_cost > 100.0 {
        max_abs_cost = max_abs_cost.sqrt().sqrt();
    }
    let boxed = (0..n).filter(|&j| (std.ub[j] - std.lb[j]).is_finite()).count();
    if (boxed as f64) < 0.01 * (n.max(1) as f64) {
        max_abs_cost = max_abs_cost.min(1.0);
    }
    let base = 5e-7 * max_abs_cost;

    let mut pc = std.c.clone();
    for j in 0..n {
        let lo = std.lb[j];
        let hi = std.ub[j];
        let free = !lo.is_finite() && !hi.is_finite();
        let fixed = lo == hi;
        if free || fixed {
            continue;
        }

        // splitmix64-style hash of `j` into a pseudo-random value in [0, 1).
        let mut h = (j as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
        h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        h ^= h >> 31;
        let r = (h >> 40) as f64 / (1u64 << 24) as f64;

        let xpert = (1.0 + r) * (pc[j].abs() + 1.0) * base;
        if !hi.is_finite() {
            pc[j] += xpert;
        } else if !lo.is_finite() {
            pc[j] -= xpert;
        } else {
            pc[j] += if pc[j] >= 0.0 { xpert } else { -xpert };
        }
    }
    pc
}

#[cfg(test)]
fn refactorize(std: &StdForm, t: &Tableau, prev: Option<&sparse_lu::FtLu>) -> sparse_lu::FtLu {
    try_refactorize(std, t, prev).expect("simplex basis matrix must be nonsingular")
}

/// `refactorize` without the panic: `None` when `factorize` finds the
/// current basis numerically singular. A basis reached by valid pivots
/// is nonsingular in exact arithmetic, so this only happens once
/// accumulated floating-point error (a run of near-`FT_MIN_PIVOT` pivots
/// on a degenerate problem) has made it singular *to working precision*
/// — the dual loop treats that as "this trajectory is numerically spent"
/// and hands the problem to the primal method (whose different pivot
/// sequence sidesteps it), rather than crashing the whole solve.
///
/// `prev` is the factorization being replaced, when the caller has one on
/// hand: its pivot order is reused rather than searched for again — see
/// [`sparse_lu::factorize_reusing`] for what that does, and for the
/// threshold-pivoting and fill checks that make passing it safe (a reuse
/// failing either check falls back to the full Markowitz search by
/// itself, so `prev` never changes which factorizations are accepted).
fn try_refactorize(std: &StdForm, t: &Tableau, prev: Option<&sparse_lu::FtLu>) -> Option<sparse_lu::FtLu> {
    let rows = t.basis_rows_sparse();
    sparse_lu::factorize_reusing(std.n_rows, &rows, prev)
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
///
/// Returns `None`, rather than panicking, if a mid-solve refactorization
/// ever finds the *current* basis numerically singular (see
/// `try_refactorize`'s own docs for why a validly-reached basis can still
/// get here) and can't be corrected — at that point `t`'s incrementally-
/// maintained state can no longer be trusted or repaired (there is no way
/// left to verify or resync it against a fresh factorization), so the
/// caller must not read `t` on `None` and must not treat it as a genuine
/// (if unconfirmed) `Status::Optimal` the way the iteration-cap fallback
/// at the very end of this function still can: unlike that cap (which
/// only means "still converging, just slowly"), this can trigger after
/// only a modest number of pivots, on a `t` whose primal feasibility may
/// already have silently drifted away from what it was last confirmed to
/// be — confirmed on Netlib's own `cycle` instance, where trusting `t.x`
/// at exactly this point produced a wildly wrong "optimal" objective
/// instead of an honest failure. The caller's own recovery differs by
/// context: `extended_dual`'s polish handoff (the only caller) propagates
/// `None`, which the solve reports as `Status::NotSolved`.
fn run_phase(
    std: &StdForm,
    t: &mut Tableau,
    phase1: bool,
    lu: &mut sparse_lu::FtLu,
    since_check: &mut usize,
    expand: &mut ExpandState,
    se: &mut SteepestEdgeState,
    stall: &mut PrimalStallState,
) -> Option<Status> {
    let m = std.n_rows;
    // See [`partial_pricing_sampled`]'s own docs for why this is a fixed
    // seed (reproducibility) varied only by `n_total` (so distinctly-sized
    // LPs don't all sample the same columns) rather than a time-seeded RNG.
    let pricing_seed = 0x2545_F491_4F6C_DD1D_u64 ^ (std.n_total as u64);
    // Same scaling the dual method's own `stall_limit` uses — see
    // [`PrimalStallState`]'s own docs.
    let stall_limit = (5 * m).max(500);

    // Per-iteration blocking-row candidate for the ratio test below —
    // hoisted out of the loop body (it used to be defined inline, next to
    // its only use) purely so `candidates_buf` below can name the type.
    struct Candidate {
        row: usize,
        exact: f64,
        relaxed: f64,
        pivot_abs: f64,
        hits_upper: bool,
    }

    // Per-iteration scratch/output buffers for every iteration's FTRAN/
    // BTRAN calls and candidate list, declared once here rather than fresh
    // inside the loop — mirrors the dual loop's own pre-loop buffer
    // block (see that function's own docs): this primal loop used to
    // allocate a fresh `cost`/`y`/`a_enter`/`alpha`/`e_r`/`rho`/`w` `Vec`
    // (and a fresh `candidates` vec) on *every* iteration, i.e. several
    // m-length heap allocations per pivot.
    let mut cost_buf = vec![0.0; m];
    let mut y_buf = vec![0.0; m];
    let mut a_enter_buf = vec![0.0; m];
    let mut alpha_buf = vec![0.0; m];
    let mut scratch_buf = vec![0.0; m];
    let mut rho_buf = vec![0.0; m];
    let mut w_buf = vec![0.0; m];
    let mut candidates_buf: Vec<Candidate> = Vec::with_capacity(m);

    let ratio_pivot_tol = if env_str!("ENOMOTO_PRIMAL_RATIO_PIVOT_TOL_OLD").is_some() { TOL } else { tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64) };
    let max_iters = max_iters_for(m, std.n_total);
    for iter_idx in 0..max_iters {
        prof_phases::RUN_PHASE_ITERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let rhs = t.recompute_basics(lu);

        // Triggers (1) and (3): periodic residual / eta-file-fill checks.
        *since_check += 1;
        if *since_check >= FT_CHECK_INTERVAL {
            *since_check = 0;
            let bump_too_big = lu.fill_count() > tunable!("ENOMOTO_T_FT_BUMP_LIMIT_FACTOR", FT_BUMP_LIMIT_FACTOR, usize) * m.max(1);
            let residual_too_big = !bump_too_big && t.basis_residual_norm(&rhs) > FT_RESIDUAL_TOL;
            if bump_too_big || residual_too_big {
                // `None` here, not a panic — see this function's own docs
                // for what that signals to the caller.
                let Some(l) = try_refactorize(std, t, Some(&*lu)) else {
                    if env_str!("ENOMOTO_DEBUG_PHASES").is_some() {
                        eprintln!("run_phase None@residual iter={iter_idx} phase1={phase1} bump={bump_too_big} residual={residual_too_big}");
                    }
                    return None;
                };
                *lu = l;
            }
        }
        // Trigger (4): unconditional cap on accumulated updates.
        if lu.update_count() > FT_MAX_UPDATES {
            let Some(l) = try_refactorize(std, t, Some(&*lu)) else {
                if env_str!("ENOMOTO_DEBUG_PHASES").is_some() {
                    eprintln!("run_phase None@ft_max_updates iter={iter_idx} phase1={phase1}");
                }
                return None;
            };
            *lu = l;
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
        if phase1 {
            for i in 0..m {
                let var = t.basis[i];
                let v = t.x[var];
                cost_buf[i] = if v < std.lb[var] - expand.delta {
                    -1.0
                } else if v > std.ub[var] + expand.delta {
                    1.0
                } else {
                    0.0
                };
            }
        } else {
            for i in 0..m {
                cost_buf[i] = std.c[t.basis[i]];
            }
        }
        let cost: &[f64] = &cost_buf;

        if phase1 && cost.iter().all(|&c| c == 0.0) {
            return Some(Status::Optimal); // phase-1 feasible
        }

        // y = B^-T cost_B ; reduced cost d_j = c_j - y . a_j
        lu.solve_transpose_into(cost, &mut scratch_buf, &mut y_buf);
        let y: &[f64] = &y_buf;

        // Steepest-edge entering rule (Forrest & Goldfarb 1992): among
        // eligible nonbasic j, maximize d_j^2 / gamma_j rather than
        // Dantzig's |d_j| — see `SteepestEdgeState`'s docs. Each column's
        // reduced cost/score is independent of every other, but this scans
        // sequentially, not via rayon — see this module's own
        // docs for the profiling that found rayon's per-call dispatch
        // overhead exceeding the loop body's own cost at this crate's
        // typical problem sizes, for every hot per-iteration loop like
        // this one, not just that specific one.
        //
        // Pricing every nonbasic column costs one `column_sparse` dot
        // product each — on a wide problem (`n_total >=
        // PARTIAL_PRICING_THRESHOLD`) most of those never come close to
        // winning, so partial pricing (Dantzig/Forrest-Goldfarb-Reid-style
        // grouping) prices only a random ~`1/PARTIAL_PRICING_GROUPS`
        // sample first; only when that sample has no improving candidate
        // at all does it pay for the rest.
        let price_one = |j: usize| -> Option<(usize, f64, f64, f64)> {
            let st = t.nb_status[j]?;
            // Fixed columns (`lb[j] == ub[j]`) can never be a genuine
            // entering candidate — any step away from their single
            // feasible point violates their own bound immediately (the
            // ratio test below would floor the step at `0`), so pricing
            // them wastes a `column_sparse` dot product only to produce a
            // score that, if it ever won, would buy a degenerate pivot.
            // Skipped before that dot product, not after, since this
            // closure runs once per nonbasic column every iteration.
            if std.lb[j] == std.ub[j] {
                return None;
            }
            let cj = if phase1 { 0.0 } else { std.c[j] };
            let dot = sparse_dot_dense(t.column_sparse(j), y);
            let dj = cj - dot;

            let (eligible, dir) = match st {
                NbStatus::Lower => (dj < -TOL, 1.0),
                NbStatus::Upper => (dj > TOL, -1.0),
                // Free at `0`: improving in whichever direction lowers the
                // objective (its width is infinite, so the ratio test below
                // never turns this into a bound flip).
                NbStatus::Zero => (true, if dj < 0.0 { 1.0 } else { -1.0 }),
            };
            if eligible && dj.abs() > TOL {
                let score = dj * dj / se.gamma[j].max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
                Some((j, score, dir, dj))
            } else {
                None
            }
        };

        // In `bland_mode`, entering-variable selection also switches to
        // Bland's rule: smallest index among eligible candidates, scanned
        // in full — partial pricing's random sampling has no finite-
        // termination guarantee, so it's bypassed here the same way the
        // dual method's own `bland_mode` bypasses DSE-based `chuzr`.
        let best = if stall.bland_mode {
            (0..std.n_total).filter_map(price_one).min_by_key(|&(j, ..)| j)
        } else if std.n_total >= tunable!("ENOMOTO_T_PARTIAL_PRICING_THRESHOLD", PARTIAL_PRICING_THRESHOLD, usize) {
            let iter_u64 = iter_idx as u64;
            let sample_best = (0..std.n_total)
                .filter(|&j| partial_pricing_sampled(pricing_seed, iter_u64, j))
                .filter_map(price_one)
                .max_by(|a, b| a.1.total_cmp(&b.1));
            sample_best.or_else(|| {
                (0..std.n_total)
                    .filter(|&j| !partial_pricing_sampled(pricing_seed, iter_u64, j))
                    .filter_map(price_one)
                    .max_by(|a, b| a.1.total_cmp(&b.1))
            })
        } else {
            (0..std.n_total).filter_map(price_one).max_by(|a, b| a.1.total_cmp(&b.1))
        };

        let Some((enter, _best_score, best_dir, dj_enter)) = best else {
            // No improving direction.
            return Some(if phase1 { Status::Infeasible } else { Status::Optimal });
        };

        // alpha = B^-1 a_enter
        t.column_into(enter, &mut a_enter_buf);
        lu.solve_into(&a_enter_buf, &mut scratch_buf, &mut alpha_buf);
        let a_enter: &[f64] = &a_enter_buf;
        let alpha: &[f64] = &alpha_buf;

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
        let self_width = std.ub[enter] - std.lb[enter];
        let init_alpha1 = if self_width.is_finite() { self_width } else { f64::INFINITY };

        // Each row's blocking analysis is independent of every other row's
        // — only the final `alpha1`/leaving-row reductions below combine
        // them — but, like the entering-variable scan above, this runs
        // sequentially rather than via rayon (same measured overhead).
        candidates_buf.clear();
        for i in 0..m {
            let rate = -best_dir * alpha[i]; // d(x_Bi)/d(theta)
            // `FT_MIN_PIVOT`, not `TOL`: a leaving row with `|alpha|` this
            // small makes the new basis (nearly) singular — `try_update`
            // rejects the update and the refactorization that follows fails,
            // aborting this phase (Netlib `dfl001`'s cleanup handoff hit a
            // `1.2e-9` pivot this way once the extended dual's path shifted
            // slightly, and fell back to a from-scratch solve costing more
            // than the whole dual run). Treating such rows as non-blocking is
            // the usual primal ratio-test pivot tolerance (HiGHS
            // `HEkkPrimal`'s `alpha_tol` reaches `1e-7` as well); the bound
            // violation it permits is at most `theta * 1e-7`.
            // `ENOMOTO_PRIMAL_RATIO_PIVOT_TOL_OLD` restores `TOL` (A/B only).
            if rate.abs() <= ratio_pivot_tol {
                continue;
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
                continue;
            }
            let exact = (bound - val) / rate;
            let relaxed = if returning_to_feasibility {
                exact
            } else {
                let relaxed_bound = if is_upper { bound + expand.delta } else { bound - expand.delta };
                (relaxed_bound - val) / rate
            };

            candidates_buf.push(Candidate { row: i, exact, relaxed, pivot_abs: alpha[i].abs(), hits_upper: is_upper });
        }
        let candidates: &[Candidate] = &candidates_buf;

        let alpha1 = candidates.iter().map(|c| c.relaxed).fold(init_alpha1, f64::min);

        // `PRIMAL_HARRIS_TOL`, not `TOL`: see that constant's own docs for
        // why the EXPAND-only window (`alpha1` itself, already widened by
        // `expand.delta`) isn't enough on its own to steer this "largest
        // pivot magnitude" tie-break away from an arbitrarily small pivot.
        // In `bland_mode`, the tie-break itself also switches — smallest
        // *basic-variable* index among admitted candidates, not the
        // largest pivot — matching Bland's rule's own leaving-variable
        // requirement (consistent, deterministic tie-breaking on both
        // sides of a pivot is what its finite-termination proof needs).
        let admitted = candidates.iter().filter(|c| c.exact <= alpha1 + tunable!("ENOMOTO_T_PRIMAL_HARRIS_TOL", PRIMAL_HARRIS_TOL, f64));
        let leaving = if stall.bland_mode {
            admitted.min_by_key(|c| t.basis[c.row])
        } else {
            admitted.max_by(|a, b| a.pivot_abs.total_cmp(&b.pivot_abs))
        };
        let (leaving_row, leaving_hits_upper, alpha2, best_pivot_mag) = match leaving {
            Some(c) if c.pivot_abs > 0.0 => (Some(c.row), c.hits_upper, c.exact, c.pivot_abs),
            _ => (None, false, 0.0, 0.0),
        };

        let theta = match leaving_row {
            None => {
                if !alpha1.is_finite() {
                    return Some(Status::Unbounded);
                }
                alpha1
            }
            Some(_) => alpha2.max(EXPAND_TAU / best_pivot_mag),
        };

        // Stall detection for the Bland's-rule fallback (see
        // [`PrimalStallState`]'s own docs): `theta * dj_enter` is this
        // pivot's actual contribution to the objective (phase 2) or the
        // composite infeasibility measure (phase 1) — this method's own
        // analogue of the dual method's `theta_q * dj_q` stall signal, so
        // a run of consecutive near-zero-contribution pivots gets the same
        // treatment here.
        if (theta * dj_enter).abs() < STALL_PROGRESS_EPS {
            stall.stall_count += 1;
            if stall.stall_count > stall_limit {
                stall.bland_mode = true;
            }
        } else {
            stall.stall_count = 0;
        }

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
                lu.solve_transpose_unit(r, &mut scratch_buf, &mut rho_buf);
                lu.solve_transpose_into(alpha, &mut scratch_buf, &mut w_buf);
                let rho: &[f64] = &rho_buf;
                let w: &[f64] = &w_buf;
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
                se.update_after_pivot(t, std, rho, w, gamma_t_old, pivot);

                // Trigger (2): FtLu::try_update refactorizes in-place if
                // the resulting pivot is too small to use safely. `None`
                // here too — see the identical fallback earlier in this
                // same loop, and this function's own docs.
                if !lu.try_update(r, a_enter, tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64)) {
                    let Some(l) = try_refactorize(std, t, Some(&*lu)) else {
                        if env_str!("ENOMOTO_DEBUG_PHASES").is_some() {
                            eprintln!("run_phase None@ft_update iter={iter_idx} phase1={phase1} pivot={pivot}");
                        }
                        return None;
                    };
                    *lu = l;
                }
            }
        }
    }

    Some(Status::Optimal) // iteration cap hit; best-effort
}

/// Incremental phase-2 primal simplex — the S4 alternative to
/// [`run_phase`] for `extended_dual`'s polish -> primal handoff only
/// (`ENOMOTO_HANDOFF_INCREMENTAL=1`, default off; see
/// `analysis/simplex_loop_20260924_113533.md` §3.2/§4 S4).
///
/// [`run_phase`] re-derives everything every iteration: `x_B` from scratch
/// (`compute_rhs` `O(nnz(A))` + an FTRAN), `y` by a BTRAN, every nonbasic
/// column's reduced cost by a dot product, and the steepest-edge update's
/// own two BTRANs (`rho`, `w`) plus two dot products per nonbasic column.
/// On Netlib `pilot87` that made one handoff pivot cost 2.3x a dual one.
/// This loop keeps `x_B` and `d` up to date incrementally instead, the way
/// the dual loops do:
///
/// - `x_B` moves by the ratio test's own step (`run_phase` applies the
///   same step, then discards it at its next `recompute_basics`), and is
///   re-derived from scratch only at a refresh point (below);
/// - `d` is updated from the pivot row: with `rho = B^-T e_r` and
///   `alpha_r = rho^T A` (a row-wise PRICE over `rho`'s nonzeros),
///   `d_j -= (d_q / alpha_rq) alpha_rj`, and the leaving column gets
///   `-d_q / alpha_rq`;
/// - the steepest-edge update reads `alpha_r` (and `w^T A`, gathered in the
///   same row-wise pass over `w = B^-T alpha`'s nonzeros) instead of two
///   dot products per nonbasic column; `ENOMOTO_HANDOFF_INC_DEVEX=1`
///   swaps it for primal Devex (reference weights, `rho` only — one BTRAN
///   per pivot instead of two).
///
/// Refresh points (fresh `x_B` via `recompute_basics` + the residual /
/// eta-fill refactorization check, fresh `d` via one BTRAN on `c_B`): the
/// first iteration, every EXPAND reset (`EXPAND_K` iterations, where the
/// nonbasic values move anyway), after every refactorization, and — the
/// correctness guard — before trusting either "no entering candidate"
/// (`Optimal`) or "no blocking row" (`Unbounded`): the maintained `d` only
/// proposes those, a freshly computed one confirms them, which is exactly
/// the test [`run_phase`] applies every iteration.
fn run_phase2_incremental(std: &StdForm, t: &mut Tableau, lu: &mut sparse_lu::FtLu, stall: &mut PrimalStallState) -> Option<Status> {
    use std::sync::atomic::Ordering::Relaxed;
    let m = std.n_rows;
    let n = std.n_total;
    let stall_limit = (5 * m).max(500);
    let floor = tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64);
    let min_pivot = tunable!("ENOMOTO_T_FT_MIN_PIVOT", FT_MIN_PIVOT, f64);
    let harris = tunable!("ENOMOTO_T_PRIMAL_HARRIS_TOL", PRIMAL_HARRIS_TOL, f64);
    let bump_limit = tunable!("ENOMOTO_T_FT_BUMP_LIMIT_FACTOR", FT_BUMP_LIMIT_FACTOR, usize) * m.max(1);
    let use_devex = tunable!("ENOMOTO_HANDOFF_INC_DEVEX", 0u8, u8) != 0;
    let mut expand = ExpandState::new();

    let mut gamma: Vec<f64> = if use_devex { vec![1.0; n] } else { SteepestEdgeState::new(std).gamma };
    let mut cost_b = vec![0.0; m];
    let mut y = vec![0.0; m];
    let mut scratch = vec![0.0; m];
    let mut d = vec![0.0; n];
    let mut a_enter = vec![0.0; m];
    let mut alpha = vec![0.0; m];
    let mut rho = vec![0.0; m];
    let mut w = vec![0.0; m];
    let mut ap = vec![0.0; n];
    let mut tp = vec![0.0; n];
    let mut touched = vec![false; n];
    let mut touched_cols: Vec<usize> = Vec::new();

    struct Candidate {
        row: usize,
        exact: f64,
        relaxed: f64,
        pivot_abs: f64,
        hits_upper: bool,
    }
    let mut candidates: Vec<Candidate> = Vec::with_capacity(m);

    let mut need_fresh = true;
    let mut since_check = 0usize;
    let max_iters = max_iters_for(m, n);
    for _iter in 0..max_iters {
        prof_phases::RUN_PHASE_ITERS.fetch_add(1, Relaxed);
        let fresh_now = need_fresh;
        if need_fresh {
            need_fresh = false;
            let rhs = t.recompute_basics(lu);
            if lu.fill_count() > bump_limit || t.basis_residual_norm(&rhs) > FT_RESIDUAL_TOL {
                *lu = try_refactorize(std, t, Some(&*lu))?;
                t.recompute_basics(lu);
            }
            for i in 0..m {
                cost_b[i] = std.c[t.basis[i]];
            }
            lu.solve_transpose_into(&cost_b, &mut scratch, &mut y);
            for j in 0..n {
                d[j] = if t.nb_status[j].is_some() { std.c[j] - sparse_dot_dense(t.column_sparse(j), &y) } else { 0.0 };
            }
        }

        // Refactorization triggers (`run_phase`'s (3)/(4); its per-iteration
        // residual check (1) is paid only at refresh points).
        since_check += 1;
        if since_check >= FT_CHECK_INTERVAL {
            since_check = 0;
            if lu.fill_count() > bump_limit {
                *lu = try_refactorize(std, t, Some(&*lu))?;
                need_fresh = true;
                continue;
            }
        }
        if lu.update_count() > FT_MAX_UPDATES {
            *lu = try_refactorize(std, t, Some(&*lu))?;
            need_fresh = true;
            continue;
        }

        expand.delta += EXPAND_TAU;
        expand.iters_since_reset += 1;
        if expand.iters_since_reset >= EXPAND_K {
            expand.iters_since_reset = 0;
            expand.delta = EXPAND_DELTA_0;
            t.expand_reset_nonbasics();
            need_fresh = true;
            continue;
        }

        // Pricing over the maintained `d` (`run_phase`'s `price_one`
        // eligibility rule, without its per-column dot product).
        let mut best: Option<(usize, f64, f64, f64)> = None;
        for j in 0..n {
            let Some(st) = t.nb_status[j] else { continue };
            if std.lb[j] == std.ub[j] {
                continue;
            }
            let dj = d[j];
            let (eligible, dir) = match st {
                NbStatus::Lower => (dj < -TOL, 1.0),
                NbStatus::Upper => (dj > TOL, -1.0),
                NbStatus::Zero => (true, if dj < 0.0 { 1.0 } else { -1.0 }),
            };
            if !(eligible && dj.abs() > TOL) {
                continue;
            }
            if stall.bland_mode {
                if best.is_none() {
                    best = Some((j, 0.0, dir, dj));
                }
                continue;
            }
            let score = dj * dj / gamma[j].max(floor);
            if best.map_or(true, |b| score > b.1) {
                best = Some((j, score, dir, dj));
            }
        }
        let Some((enter, _, best_dir, dj_enter)) = best else {
            if fresh_now {
                return Some(Status::Optimal);
            }
            need_fresh = true;
            continue;
        };

        t.column_into(enter, &mut a_enter);
        lu.solve_into(&a_enter, &mut scratch, &mut alpha);

        // Ratio test: `run_phase`'s phase-2 Harris/EXPAND two-pass test.
        let self_width = std.ub[enter] - std.lb[enter];
        let init_alpha1 = if self_width.is_finite() { self_width } else { f64::INFINITY };
        candidates.clear();
        for i in 0..m {
            let rate = -best_dir * alpha[i];
            if rate.abs() <= min_pivot {
                continue;
            }
            let var = t.basis[i];
            let val = t.x[var];
            let (bound, is_upper) = if rate < 0.0 { (std.lb[var], false) } else { (std.ub[var], true) };
            if !bound.is_finite() {
                continue;
            }
            let exact = (bound - val) / rate;
            let relaxed_bound = if is_upper { bound + expand.delta } else { bound - expand.delta };
            let relaxed = (relaxed_bound - val) / rate;
            candidates.push(Candidate { row: i, exact, relaxed, pivot_abs: alpha[i].abs(), hits_upper: is_upper });
        }
        let alpha1 = candidates.iter().map(|c| c.relaxed).fold(init_alpha1, f64::min);
        let admitted = candidates.iter().filter(|c| c.exact <= alpha1 + harris);
        let leaving = if stall.bland_mode { admitted.min_by_key(|c| t.basis[c.row]) } else { admitted.max_by(|a, b| a.pivot_abs.total_cmp(&b.pivot_abs)) };
        let (leaving_row, leaving_hits_upper, alpha2, best_pivot_mag) = match leaving {
            Some(c) if c.pivot_abs > 0.0 => (Some(c.row), c.hits_upper, c.exact, c.pivot_abs),
            _ => (None, false, 0.0, 0.0),
        };
        let theta = match leaving_row {
            None => {
                if !alpha1.is_finite() {
                    if fresh_now {
                        return Some(Status::Unbounded);
                    }
                    need_fresh = true;
                    continue;
                }
                alpha1
            }
            Some(_) => alpha2.max(EXPAND_TAU / best_pivot_mag),
        };

        if (theta * dj_enter).abs() < STALL_PROGRESS_EPS {
            stall.stall_count += 1;
            if stall.stall_count > stall_limit {
                stall.bland_mode = true;
            }
        } else {
            stall.stall_count = 0;
        }

        for i in 0..m {
            let var = t.basis[i];
            t.x[var] -= best_dir * alpha[i] * theta;
        }
        t.x[enter] += best_dir * theta;

        let Some(r) = leaving_row else {
            let new_status = if best_dir > 0.0 { NbStatus::Upper } else { NbStatus::Lower };
            t.nb_status[enter] = Some(new_status);
            t.x[enter] = if best_dir > 0.0 { std.ub[enter] } else { std.lb[enter] };
            continue;
        };

        // Pivot row (and, for steepest edge, `w^T A`) against the OLD basis.
        lu.solve_transpose_unit(r, &mut scratch, &mut rho);
        if !use_devex {
            lu.solve_transpose_into(&alpha, &mut scratch, &mut w);
        }
        for i in 0..m {
            let rv = rho[i];
            let wv = if use_devex { 0.0 } else { w[i] };
            if rv.abs() <= TOL && wv.abs() <= TOL {
                continue;
            }
            for &(j, v) in std.rows.row(i) {
                if std.lb[j] == std.ub[j] {
                    continue;
                }
                if !touched[j] {
                    touched[j] = true;
                    touched_cols.push(j);
                }
                ap[j] += rv * v;
                tp[j] += wv * v;
            }
        }

        let pivot = alpha[r];
        let theta_d = d[enter] / pivot;
        let gamma_q = gamma[enter];

        let leaving_var = t.basis[r];
        t.nb_status[leaving_var] = Some(if leaving_hits_upper { NbStatus::Upper } else { NbStatus::Lower });
        t.basis_pos[leaving_var] = None;
        t.basis[r] = enter;
        t.basis_pos[enter] = Some(r);
        t.nb_status[enter] = None;

        for &j in &touched_cols {
            if t.nb_status[j].is_some() {
                let beta = ap[j] / pivot;
                if j != leaving_var {
                    d[j] -= theta_d * ap[j];
                    if use_devex {
                        gamma[j] = gamma[j].max(beta * beta * gamma_q);
                    }
                }
                if !use_devex {
                    gamma[j] = (gamma[j] + beta * beta * (1.0 + gamma_q) - 2.0 * beta * tp[j]).max(floor);
                }
            }
            ap[j] = 0.0;
            tp[j] = 0.0;
            touched[j] = false;
        }
        touched_cols.clear();
        d[leaving_var] = -theta_d;
        d[enter] = 0.0;
        if use_devex {
            gamma[leaving_var] = (gamma_q / (pivot * pivot)).max(1.0);
        }

        if !lu.try_update(r, &a_enter, min_pivot) {
            *lu = try_refactorize(std, t, Some(&*lu))?;
            need_fresh = true;
        }
    }

    Some(Status::Optimal) // iteration cap hit; best-effort, as in `run_phase`
}

/// Partitions `std`'s structural variables (`0..n_orig`, `n_orig =
/// n_total - n_rows`) into connected components: two variables are
/// connected iff some row's own structural (non-slack) members include
/// both of them. A row's own slack column (added once per row by
/// [`build_std_form_presolved`]) is excluded from this graph — it is
/// unique to that row and never shared with another row, so it can never
/// itself be a bridge between two otherwise-unconnected variables.
///
/// **Why checking this once, against the fully presolved `std`, is
/// enough — no separate pre-presolve check is needed**: every stage of
/// `presolve::run_extended` only ever *removes* rows, tightens bounds, or
/// substitutes a variable out in terms of others already appearing
/// alongside it in the same row — none of that can introduce a new
/// coupling between two variables that never shared a row to begin with.
/// So whatever connectivity structure the *original* model had, the
/// presolved `std` can only ever show the same structure or a *more*
/// separated one (e.g. eliminating the one row that coupled two
/// otherwise-independent halves of the model) — checking the final,
/// most-reduced state this pipeline ever produces catches both "the
/// original model was already separable" and "presolve's own reductions
/// revealed separability the original model's own structure didn't show"
/// in the same single pass.
///
/// Returns `None` when there is only one component — not worth the
/// reassembly overhead of [`split_std_form`]/[`solve_std_form_decomposed`]
/// over just solving `std` directly.
///
/// Alongside the components themselves, also returns `has_row[j]` for
/// every structural variable `j` — whether it appears in at least one
/// row — computed for free from the exact same scan this function's own
/// union-find already makes. [`solve_std_form_decomposed`] uses it to
/// decide, cheaply and *before* ever calling the real (allocation-heavy,
/// per-component) [`split_std_form`], whether splitting is even worth
/// attempting: a size-1 component with `has_row[j] == false` is a
/// variable with no row at all (typically one `dualfix` already fixed),
/// contributing nothing whether split off or left in place — see that
/// function's own docs for why building and then discarding hundreds of
/// such throwaway single-variable `StdForm`s (this crate's own first
/// version of this optimization) was itself a measurable regression, not
/// merely wasted-but-harmless effort.
fn connected_components_of_std_form(std: &StdForm) -> Option<(Vec<Vec<usize>>, Vec<bool>)> {
    let n_orig = std.n_total - std.n_rows;
    let mut parent: Vec<usize> = (0..n_orig).collect();
    let mut has_row = vec![false; n_orig];
    fn find(parent: &mut [usize], x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }
    fn union(parent: &mut [usize], a: usize, b: usize) {
        let (ra, rb) = (find(parent, a), find(parent, b));
        if ra != rb {
            parent[ra] = rb;
        }
    }

    for i in 0..std.n_rows {
        let mut first: Option<usize> = None;
        for &(j, _) in std.rows.row(i) {
            if j >= n_orig {
                continue; // this row's own slack column
            }
            has_row[j] = true;
            match first {
                None => first = Some(j),
                Some(f) => union(&mut parent, f, j),
            }
        }
        // A row with *no* structural members at all (only its own slack)
        // doesn't naturally belong to any variable-based component — but
        // it can still be a genuine, load-bearing constraint: `doubleton`
        // can rewrite a surviving row down to exactly this shape (every
        // structural coefficient cancels to zero) while its right-hand
        // side stays nonzero, deliberately kept rather than dropped as a
        // Farkas infeasibility witness the *solver* is meant to catch
        // (see `contradictory_equality_rows_detected_infeasible`'s own
        // test and `doubleton`'s module docs) — not presolve, and
        // certainly not this purely structural split. Silently omitting
        // such a row from every component's own rebuilt `StdForm` (it
        // can't touch any of them, since it touches no variable at all)
        // would erase that witness entirely. Bailing out of splitting
        // altogether whenever one exists is conservative — it forgoes a
        // split this solve might otherwise have had — but keeps the
        // *existing*, already-correct undecomposed path as the fallback,
        // rather than trying to special-case a row this genuinely
        // degenerate inside the split machinery itself.
        if first.is_none() {
            return None;
        }
    }

    let mut groups: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for j in 0..n_orig {
        let root = find(&mut parent, j);
        groups.entry(root).or_default().push(j);
    }
    if groups.len() <= 1 {
        return None;
    }
    Some((groups.into_values().collect(), has_row))
}

/// Builds a standalone `StdForm` per connected component found by
/// [`connected_components_of_std_form`], in one combined `O(nnz)` pass —
/// **not** one call per component each rescanning every row of `std`,
/// which is `O(components * n_rows)` and was this feature's own first,
/// measured-as-a-real-regression implementation (real Netlib instances
/// routinely produce hundreds of components post-presolve — almost
/// always one large remainder plus a great many singletons, per
/// [`solve_std_form_decomposed`]'s own docs — making that quadratic-ish
/// cost dominate the actual solve time it was meant to save). Each row of
/// `std` is assigned to a component via any one of its own structural
/// members (guaranteed to all share one component, by construction of
/// the components themselves — a row can never straddle two). Variables
/// are re-indexed to `0..component.len()` in each component's own order;
/// each surviving row keeps its original slack's bounds but gets a fresh
/// local slack column.
fn split_std_form(std: &StdForm, components: &[Vec<usize>]) -> Vec<StdForm> {
    let n_orig = std.n_total - std.n_rows;
    let mut comp_id = vec![usize::MAX; n_orig];
    let mut local_idx = vec![usize::MAX; n_orig];
    for (cid, comp) in components.iter().enumerate() {
        for (local_j, &orig_j) in comp.iter().enumerate() {
            comp_id[orig_j] = cid;
            local_idx[orig_j] = local_j;
        }
    }

    let mut rows_acc: Vec<Vec<Vec<(usize, f64)>>> = vec![Vec::new(); components.len()];
    let mut b_acc: Vec<Vec<f64>> = vec![Vec::new(); components.len()];
    let mut lb_acc: Vec<Vec<f64>> = components.iter().map(|c| c.iter().map(|&j| std.lb[j]).collect()).collect();
    let mut ub_acc: Vec<Vec<f64>> = components.iter().map(|c| c.iter().map(|&j| std.ub[j]).collect()).collect();
    let mut c_acc: Vec<Vec<f64>> = components.iter().map(|c| c.iter().map(|&j| std.c[j]).collect()).collect();

    for i in 0..std.n_rows {
        let cid = std
            .rows
            .row(i)
            .iter()
            .find_map(|&(j, _)| if j < n_orig { Some(comp_id[j]) } else { None })
            .expect("row with no structural members must have made connected_components_of_std_form bail out already");
        let local_n = components[cid].len();
        let slack_col = local_n + rows_acc[cid].len();
        let mut row: Vec<(usize, f64)> = Vec::with_capacity(std.rows.row(i).len());
        let (mut slack_lb, mut slack_ub) = (0.0, 0.0);
        for &(j, v) in std.rows.row(i) {
            if j < n_orig {
                debug_assert_eq!(comp_id[j], cid, "row split across two components");
                row.push((local_idx[j], v));
            } else {
                row.push((slack_col, v));
                slack_lb = std.lb[j];
                slack_ub = std.ub[j];
            }
        }
        rows_acc[cid].push(row);
        b_acc[cid].push(std.b[i]);
        lb_acc[cid].push(slack_lb);
        ub_acc[cid].push(slack_ub);
        c_acc[cid].push(0.0);
    }

    let mut result = Vec::with_capacity(components.len());
    for cid in 0..components.len() {
        let local_n = components[cid].len();
        let rows = std::mem::take(&mut rows_acc[cid]);
        let n_rows = rows.len();
        let n_total = local_n + n_rows;
        let (rows, cols) = freeze_std_matrices(&rows, n_total);
        result.push(StdForm {
            n_total,
            n_rows,
            c: std::mem::take(&mut c_acc[cid]),
            rows,
            cols,
            b: std::mem::take(&mut b_acc[cid]),
            lb: std::mem::take(&mut lb_acc[cid]),
            ub: std::mem::take(&mut ub_acc[cid]),
        });
    }
    result
}

/// Solves an already-presolved `StdForm` with
/// [`extended_dual::solve_lp_dual_extended`], transparently splitting into
/// independent connected components first when
/// [`connected_components_of_std_form`] finds at least two real ones —
/// each solved by its own `solve_lp_dual_extended` call (on `rayon` when
/// any component reaches [`PARALLEL_COMPONENT_MIN_VARS`]), then recombined
/// into one [`SimplexResult`] indexed by the *original* variable numbering
/// (see [`combine_component_statuses`] for how per-component verdicts
/// combine). Falls straight through to a single, undecomposed solve when
/// no useful split is found, so this adds only
/// [`connected_components_of_std_form`]'s own `O(nnz)` union-find scan to
/// the cost of a solve that turns out not to be separable. An extended
/// solve returning `None` (one of its documented "should be unreachable"
/// bail-outs) is reported as [`Status::NotSolved`].
///
/// **What Netlib actually looks like post-presolve**: most instances split
/// into **hundreds** of components (`fit1p` 628, `sctap3` 624, `ganges`
/// 530, `modszk1` 422, …), but always as one large remaining component
/// (`fit1p`'s is 1050 of its own 1677 variables) plus a great many
/// singletons — variables presolve left with no remaining *real* row to
/// couple them to anything else. No instance in the benchmark set has two
/// or more components clearing the `real_components` bar below, so on
/// that set this mechanism is dormant by design, active only for problems
/// with genuine block-diagonal structure.
///
/// Why splitting requires *two or more* real components (checked via the
/// free `has_row` flags *before* [`split_std_form`] is ever called):
/// splitting off only trivial singletons around one large remainder buys
/// nothing — those variables were already static nonbasic values in the
/// undecomposed solve — while costing (1) hundreds of throwaway `StdForm`
/// allocations (a measured regression when an earlier version built them
/// first and discarded them after), and (2) a changed pivot path:
/// compacting every surviving variable's global index changes which
/// candidate wins an exact tie in pricing, and on Netlib `25fv47` that
/// alone took iterations from 3,092 to 11,468 for the identical answer.
fn solve_std_form_decomposed(std: &StdForm, opts: &crate::types::LpOptions) -> SimplexResult {
    let solve_one = |s: &StdForm| extended_dual::solve_lp_dual_extended(s, opts).unwrap_or(SimplexResult { status: Status::NotSolved, x: None });

    let Some((components, has_row)) = connected_components_of_std_form(std) else {
        return solve_one(std);
    };
    let real_components = components.iter().filter(|c| c.len() > 1 || has_row[c[0]]).count();
    if real_components <= 1 {
        return solve_one(std);
    }

    let sub_std_forms = split_std_form(std, &components);
    let use_parallel = components.iter().any(|c| c.len() >= PARALLEL_COMPONENT_MIN_VARS);
    let results: Vec<SimplexResult> = if use_parallel {
        use rayon::prelude::*;
        sub_std_forms.par_iter().map(solve_one).collect()
    } else {
        sub_std_forms.iter().map(solve_one).collect()
    };

    let status = combine_component_statuses(results.iter().map(|r| &r.status));
    if status != Status::Optimal {
        return SimplexResult { status, x: None };
    }
    let n_orig = std.n_total - std.n_rows;
    let mut x = vec![0.0; n_orig];
    for (result, component) in results.iter().zip(components.iter()) {
        let sub_x = result.x.as_ref().expect("Optimal result must carry x");
        for (local_j, &orig_j) in component.iter().enumerate() {
            x[orig_j] = sub_x[local_j];
        }
    }
    SimplexResult { status: Status::Optimal, x: Some(x) }
}

/// The whole problem's status from its independent components' statuses.
/// One infeasible component makes the whole problem infeasible, whatever
/// the others say. Otherwise a component with no finite optimum
/// (`InfeasibleOrUnbounded`, or `Unbounded`) means the whole problem has
/// none either — but `Unbounded` for the whole also needs every other
/// component feasible, so an unsolved component (`NotSolved`) next to an
/// unbounded one only supports `InfeasibleOrUnbounded`. Only when every
/// component is `Optimal` is the whole problem `Optimal`.
fn combine_component_statuses<'a>(statuses: impl Iterator<Item = &'a Status>) -> Status {
    let (mut infeasible, mut ioru, mut unbounded, mut not_solved) = (false, false, false, false);
    for s in statuses {
        match s {
            Status::Infeasible => infeasible = true,
            Status::InfeasibleOrUnbounded => ioru = true,
            Status::Unbounded => unbounded = true,
            Status::NotSolved => not_solved = true,
            Status::Optimal => {}
        }
    }
    if infeasible {
        Status::Infeasible
    } else if ioru || (unbounded && not_solved) {
        Status::InfeasibleOrUnbounded
    } else if unbounded {
        Status::Unbounded
    } else if not_solved {
        Status::NotSolved
    } else {
        Status::Optimal
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
    // Decided once from `m` at construction — see `RAYON_SIZE_THRESHOLD`'s
    // own docs for why this replaced an earlier run-both-and-time
    // self-calibration: this crate's own microbenchmarks never found
    // `rayon` winning at any size actually tried, so a live timing race
    // was pure overhead (and one less source of run-to-run nondeterminism
    // to reason about) for every solve at this crate's realistic problem
    // sizes.
    use_parallel: bool,
}

impl DseState {
    /// `B0` is a signed identity, so `e_i^T B0^-1` is `+/-e_i^T` and
    /// `w[i] = 1` for every row initially.
    fn new(m: usize) -> Self {
        DseState { w: vec![1.0; m], use_parallel: m > RAYON_SIZE_THRESHOLD }
    }

    /// The current weight for basic row `i` — `chuzr`'s only need from this type.
    #[inline]
    fn weight(&self, i: usize) -> f64 {
        self.w[i]
    }

    /// Exact DSE weights for an **arbitrary** (already-factored) basis,
    /// rather than [`Self::new`]'s all-slack-only `w[i] = 1`. Row `i` of
    /// `B^-1` is `e_i^T B^-1`, obtained by one BTRAN (`B^T z = e_i`, i.e.
    /// `lu.solve_transpose_into(&e_i, ..)`); its squared 2-norm is exactly
    /// `w[i] = ||e_i^T B^-1||^2`. One BTRAN per row (`O(m)` BTRANs) - the
    /// same *kind* of `O(m * nnz)` work [`fresh_d`] already does in one
    /// call after every refactorization, and far cheaper than the full
    /// cold restart it replaces on `forplan` (221 DSE + 245 Devex pivots).
    /// Must be given the `lu` matching the basis the weights are wanted
    /// for; `extended_dual` calls it right after each refactorization, so
    /// the weights are exact for exactly that basis.
    fn from_basis(m: usize, lu: &sparse_lu::FtLu) -> Self {
        let mut w = vec![1.0; m];
        let mut scratch = vec![0.0; m];
        let mut z = vec![0.0; m];
        // `FtLu::solve_transpose_unit_into` is only exact on a
        // freshly-factorized `u_seq` (`update_count() == 0` — see its own
        // docs) — every current caller is a refactor-time DSE refresh, but
        // a factorization with updates already applied still falls back to
        // the unmodified dense `solve_transpose_into` sweep rather than
        // ever risking silent wrong weights.
        if lu.update_count() == 0 {
            for i in 0..m {
                lu.solve_transpose_unit_into(i, &mut scratch, &mut z);
                let norm_sq: f64 = z.iter().map(|&v| v * v).sum();
                w[i] = norm_sq.max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
            }
        } else {
            for i in 0..m {
                lu.solve_transpose_unit(i, &mut scratch, &mut z);
                let norm_sq: f64 = z.iter().map(|&v| v * v).sum();
                w[i] = norm_sq.max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
            }
        }
        DseState { w, use_parallel: m > RAYON_SIZE_THRESHOLD }
    }

    /// `p` = the pivot row (basis slot that left), `alpha` = `B^-1 a_q`
    /// (the entering column's FTRAN, against the basis as it stood
    /// *before* the pivot), `tau` = `B^-1 (B^-T e_p)` ("ftran-dse"), `rho_p`
    /// = `B^-T e_p` — the BTRAN every caller already computed this same
    /// iteration for PRICE, pre-pivot.
    ///
    /// Disjoint per-row writes (`w[i]` for `i != p` each depend only on
    /// `alpha[i]`/`tau[i]`/the *old* `w[p]`, never on another row's *new*
    /// value), so this parallelizes trivially — no fold/reduce needed,
    /// unlike `scaling::compute`'s max-accumulation.
    ///
    /// `wp_old` is recomputed here as `||rho_p||^2` — the exact `B^-T e_p`
    /// squared norm, i.e. exactly `w[p]`'s own textbook definition for the
    /// pre-pivot basis — rather than trusted off `self.w[p]`'s
    /// incrementally-maintained value. The two are supposed to agree, but
    /// only in infinite precision; because `wp_old` is the one quantity fed
    /// into *every other* row's update below, a `self.w[p]` that has
    /// already drifted gets re-injected into the *entire* weight vector the
    /// next time row `p` itself pivots, compounding pivot after pivot with
    /// no self-correction otherwise in reach — measured directly via
    /// `ENOMOTO_PROF_PHASES_EXT`'s own `dse_rel_err` diagnostic reaching
    /// >=100% relative error (vs. the true `||B^-T e_r||^2`) on the large
    /// majority of iterations on Netlib `fit1p`, far worse than the
    /// already-documented `degen3` case that motivated refreshing weights
    /// at `refactorize()` time (see this crate's own memory notes on that
    /// fix) — refactors alone are too infrequent (single digits per solve)
    /// to bound this. `rho_p` costs nothing extra to pass in: every call
    /// site already had to compute it this same iteration (PRICE needs it
    /// regardless of pricing scheme), so this trades an admittedly-drifting
    /// `O(1)` read for a correct `O(m)` recomputation that was already
    /// sitting there, unread, at every single call site.
    fn update_after_pivot(&mut self, p: usize, alpha: &[f64], tau: &[f64], rho_p: &[f64]) {
        let pivot = alpha[p];
        let wp_old = rho_p.iter().map(|v| v * v).sum::<f64>().max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
        let update_one = |i: usize, w_i: &mut f64| {
            if i == p {
                return;
            }
            let ratio = alpha[i] / pivot;
            *w_i = (*w_i - 2.0 * ratio * tau[i] + ratio * ratio * wp_old).max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
        };
        if self.use_parallel {
            use rayon::prelude::*;
            self.w.par_iter_mut().enumerate().for_each(|(i, w_i)| update_one(i, w_i));
        } else {
            // Branch-free over every row, `p` included: `w[p]` is
            // overwritten unconditionally just below, so computing (and
            // discarding) it here is exact, and dropping the `i == p` test
            // lets this plain zip compile to packed SIMD — the very same
            // per-element IEEE operations in the same order, so every
            // other `w[i]` is bit-identical to the scalar closure above.
            // (Skipping `alpha[i] == 0.0` rows instead — exact too — was
            // measured *slower* at the ~50% `alpha` densities of `dfl001`
            // /`pilot87`: the unpredictable branch costs more than the
            // division it saves.)
            let m = self.w.len();
            for ((w_i, &a_i), &t_i) in self.w.iter_mut().zip(&alpha[..m]).zip(&tau[..m]) {
                let ratio = a_i / pivot;
                *w_i = (*w_i - 2.0 * ratio * t_i + ratio * ratio * wp_old).max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
            }
        }
        self.w[p] = (wp_old / (pivot * pivot)).max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
    }

    /// [`Self::update_after_pivot`] restricted to `rows`, a superset of
    /// `alpha`'s nonzero rows (any order): for `alpha[i] == 0` the update is
    /// `w_i - 2*(±0)*tau_i + (±0)*(±0)*wp_old = w_i` exactly (`w_i` is
    /// already floored), so skipping those rows is bit-identical while the
    /// work drops from `O(m)` to `O(nnz(alpha))`. `wp_old` is passed in
    /// (`Σ rho_p[i]^2` summed in ascending `i`, zeros skipped — also exact,
    /// adding `+0.0` to a nonnegative partial sum changes nothing that the
    /// floor below does not already absorb).
    fn update_after_pivot_rows(&mut self, p: usize, alpha: &[f64], tau: &[f64], wp_old: f64, rows: &[u32]) {
        let floor = tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64);
        let pivot = alpha[p];
        let wp_old = wp_old.max(floor);
        for &i in rows {
            let i = i as usize;
            let ratio = alpha[i] / pivot;
            self.w[i] = (self.w[i] - 2.0 * ratio * tau[i] + ratio * ratio * wp_old).max(floor);
        }
        self.w[p] = (wp_old / (pivot * pivot)).max(floor);
    }
}

/// Hyper-sparse `chuzr`: the set of basic rows currently primal-infeasible,
/// maintained incrementally pivot-to-pivot instead of rescanned in full
/// every iteration.
///
/// The key fact making this exact, not an approximation: a feasible row's
/// `chuzr` score is *strictly* `0` (`delta == 0` in `chuzr_scan`) no matter
/// what its DSE weight is, so it can never win the row selection — only
/// the infeasible rows are ever candidates. And the only pivot-loop writes
/// that can change any row's feasibility are the ones that change its
/// basic variable's value (`x_B`) or identity, both of which are already
/// visited, row by row, by the O(m) loops that apply them (the BFRT
/// combined-flip update, the main `alpha`-scaled primal step, and the
/// post-swap identity change at the pivot row itself) — so folding a
/// membership check into those *already-O(m)* loops costs nothing extra
/// asymptotically, while it eliminates the separate full `0..m` scan
/// `chuzr` used to need every single iteration to find this same set.
/// This mirrors HiGHS's `HEkkDualRHS::workCount`/`workIndex` (incrementally
/// updated off the FTRAN indices touched each pivot, full rebuild after
/// every basis resync/refactorization).
///
/// `pub(super)` rather than private: `extended_dual::solve_lp_dual_extended`
/// reuses this exact struct for its own (`Affine1`-valued) `x_B(M)`, once
/// that module's main loop moved to the same incremental-maintenance
/// design this one already used — the membership-tracking logic itself has
/// no `f64`-vs-`Affine1` dependency at all, so duplicating it there would
/// only risk the two copies drifting apart.
pub(super) struct InfeasibleRows {
    /// Row indices currently infeasible, in no particular order.
    pub(super) rows: Vec<usize>,
    /// `pos[i] == Some(k)` iff `rows[k] == i` — the O(1) membership test
    /// and removal index `rows.push`/`swap_remove` alone can't provide.
    pos: Vec<Option<usize>>,
}

impl InfeasibleRows {
    pub(super) fn new(m: usize) -> Self {
        InfeasibleRows { rows: Vec::new(), pos: vec![None; m] }
    }

    /// Sets row `i`'s membership to `infeasible`, doing nothing if it's
    /// already in that state. `O(1)`: insertion appends; removal
    /// swap-removes and patches the displaced row's `pos` entry.
    pub(super) fn set(&mut self, i: usize, infeasible: bool) {
        match (infeasible, self.pos[i]) {
            (true, None) => {
                self.pos[i] = Some(self.rows.len());
                self.rows.push(i);
            }
            (false, Some(idx)) => {
                let last = self.rows.len() - 1;
                self.rows.swap(idx, last);
                self.rows.pop();
                if idx < self.rows.len() {
                    self.pos[self.rows[idx]] = Some(idx);
                }
                self.pos[i] = None;
            }
            _ => {}
        }
    }

    /// Full `O(m)` rebuild against `pred` — needed only right after a
    /// `resync_basics` (or the initial one, before the pivot loop starts),
    /// since that's the only operation that can change many rows' `x_B`
    /// values at once without this struct's own incremental `set` calls
    /// seeing each change individually.
    /// Whether row `i` is currently in the pool.
    #[inline]
    pub(super) fn contains(&self, i: usize) -> bool {
        self.pos[i].is_some()
    }

    pub(super) fn rebuild(&mut self, m: usize, mut pred: impl FnMut(usize) -> bool) {
        self.rows.clear();
        for i in 0..m {
            if pred(i) {
                self.pos[i] = Some(self.rows.len());
                self.rows.push(i);
            } else {
                self.pos[i] = None;
            }
        }
    }
}

/// Process-wide counters read by `extended_dual`'s diagnostics.
mod prof_phases {
    use std::sync::atomic::AtomicUsize;
    /// Iterations [`super::run_phase`] has run (cumulative, per process) —
    /// read as a before/after difference by `extended_dual`'s primal
    /// handoff diagnostic (`ENOMOTO_DEBUG_EXT_ITERS`).
    pub(crate) static RUN_PHASE_ITERS: AtomicUsize = AtomicUsize::new(0);
}

#[cfg(test)]
pub fn solve_lp_dual(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow]) -> SimplexResult {
    solve_lp_dual_with(variables, objective, constraints, crate::types::LpOptions::default())
}

/// [`solve_lp_dual`] with explicit [`crate::types::LpOptions`] — see
/// [`crate::types::LpOptions::distinguish_infeasible_unbounded`] for the one
/// option this path honors.
///
/// By default the reported status is one of `Optimal`, `Infeasible` or
/// `InfeasibleOrUnbounded` (`prop:trichotomy`: stage A's `z^1 < 0` rules out
/// a finite optimum, `z^1 = 0` rules out unboundedness and stage B decides
/// the rest). An `Unbounded` reached by any other route (the `m == 0`
/// shortcut, or combining independent components) is reported the same
/// way, so the set of possible answers does not depend on which path
/// solved the problem. [`Status::NotSolved`] is reported when the extended
/// solver gives up without an answer.
pub fn solve_lp_dual_with(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow], opts: crate::types::LpOptions) -> SimplexResult {
    let result = solve_lp_dual_classified(variables, objective, constraints, opts);
    if result.status == Status::Unbounded && !opts.distinguish_infeasible_unbounded {
        return SimplexResult { status: Status::InfeasibleOrUnbounded, x: None };
    }
    result
}

fn solve_lp_dual_classified(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow], opts: crate::types::LpOptions) -> SimplexResult {
    let PresolvedForm { std, scaling: sc, postsolve_log, orig_of_free, sign, fixed_values, shift } = match build_std_form_presolved(variables, objective, constraints, !opts.distinguish_infeasible_unbounded) {
        Ok(pf) => pf,
        Err(status) => return SimplexResult { status, x: None },
    };
    if env_str!("ENOMOTO_DEBUG_PRESOLVE_SIZE").is_some() {
        // `std.n_total - std.n_rows` is the count of structural columns
        // actually handed to the solver — every fixed (`lb[j] == ub[j]`)
        // variable, substituted or not, is excluded from `std` entirely
        // (see `PresolvedForm`'s own docs), so this is directly comparable
        // to HiGHS's `getPresolvedLp` column count, not `variables.len()`.
        eprintln!(
            "PRESOLVE_SIZE n_vars_in={} n_rows_in={} n_vars_out={} n_rows_out={}",
            variables.len(),
            constraints.len(),
            std.n_total - std.n_rows,
            std.n_rows
        );
    }
    if env_str!("ENOMOTO_DEBUG_EXT_COMPONENTS").is_some() {
        match connected_components_of_std_form(&std) {
            Some((components, _has_row)) => {
                let mut sizes: Vec<usize> = components.iter().map(|c| c.len()).collect();
                sizes.sort_unstable_by(|a, b| b.cmp(a));
                eprintln!("DEBUG_EXT_COMPONENTS: n_components={} sizes={:?}", components.len(), sizes);
            }
            None => eprintln!("DEBUG_EXT_COMPONENTS: single component (no split found)"),
        }
    }
    let result = solve_std_form_decomposed(&std, &opts);
    if env_str!("ENOMOTO_DEBUG_EXT_ITERS").is_some() {
        eprintln!("DEBUG_EXT: solve_std_form_decomposed returned {:?}", result.status);
    }
    unscale_result(result, &sc, &postsolve_log, &orig_of_free, &sign, &fixed_values, &shift, variables.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LinearExpr, VarType};
    use std::collections::BTreeMap;

    /// Not a correctness test: a standing diagnostic for the parallelism
    /// call this module's docs make ("re-profile before reaching for
    /// `into_par_iter()` again"). `#[ignore]`d so `cargo test` skips it by
    /// default; run explicitly with `cargo test --release <name> --
    /// --ignored --nocapture` (release mode matters — rayon's fixed
    /// per-call dispatch overhead is much larger, relatively, in a debug
    /// build). Measured on this machine at the time this was written:
    /// rayon was slower than a plain sequential loop for a trivial
    /// per-element workload at *every* size tried, including 200,000
    /// elements (~2.7x slower there; ~17x slower at 10,000) — the fixed
    /// per-call dispatch/join cost dominates until the per-element work or
    /// `n` is much larger than anything in this crate's hot loops.
    #[test]
    #[ignore]
    fn rayon_threshold_microbench() {
        use rayon::prelude::*;
        use std::time::Instant;

        fn work(i: usize, data: &[f64]) -> Option<(usize, f64)> {
            let v = data[i];
            let delta = if v < 0.3 { 0.3 - v } else { 0.0 };
            if delta <= 1e-9 {
                return None;
            }
            let score = delta * delta / (data[(i + 1) % data.len()]).max(1e-9);
            Some((i, score))
        }

        for &n in &[300usize, 1_000, 5_000, 10_000, 50_000, 200_000] {
            let data: Vec<f64> = (0..n).map(|i| ((i * 2654435761u64 as usize) % 1000) as f64 / 1000.0).collect();
            const REPS: usize = 200;

            let t0 = Instant::now();
            for _ in 0..REPS {
                let _best = (0..n).into_iter().filter_map(|i| work(i, &data)).max_by(|a, b| a.1.total_cmp(&b.1));
            }
            let seq = t0.elapsed() / REPS as u32;

            let t1 = Instant::now();
            for _ in 0..REPS {
                let _best = (0..n).into_par_iter().filter_map(|i| work(i, &data)).max_by(|a, b| a.1.total_cmp(&b.1));
            }
            let par = t1.elapsed() / REPS as u32;

            println!("n={n:>7} sequential={seq:>10?} rayon={par:>10?} rayon/sequential={:.2}x", par.as_secs_f64() / seq.as_secs_f64());
        }
    }

    /// Same purpose as `rayon_threshold_microbench`, but timing the exact
    /// workload `DseState::update_after_pivot` runs (a single elementwise
    /// pass over `w`, no reduce) instead of a generic filter/max. Run with
    /// `cargo test --release dse_update_rayon_threshold_microbench --
    /// --ignored --nocapture`.
    #[test]
    #[ignore]
    fn dse_update_rayon_threshold_microbench() {
        use rayon::prelude::*;
        use std::time::Instant;

        for &m in &[300usize, 1_000, 5_000, 10_000, 50_000, 200_000] {
            let alpha: Vec<f64> = (0..m).map(|i| ((i * 2654435761u64 as usize) % 1000) as f64 / 1000.0 + 0.1).collect();
            let tau: Vec<f64> = (0..m).map(|i| ((i * 40503u64 as usize) % 1000) as f64 / 1000.0).collect();
            let w: Vec<f64> = vec![1.0; m];
            let p = m / 2;
            let pivot = alpha[p];
            let wp_old = w[p];
            let update_one = |i: usize, w_i: &mut f64| {
                if i == p {
                    return;
                }
                let ratio = alpha[i] / pivot;
                *w_i = (*w_i - 2.0 * ratio * tau[i] + ratio * ratio * wp_old).max(tunable!("ENOMOTO_T_STEEPEST_EDGE_FLOOR", STEEPEST_EDGE_FLOOR, f64));
            };
            const REPS: usize = 200;

            let t0 = Instant::now();
            for _ in 0..REPS {
                let mut seq_w = w.clone();
                for (i, w_i) in seq_w.iter_mut().enumerate() {
                    update_one(i, w_i);
                }
                std::hint::black_box(&seq_w);
            }
            let seq = t0.elapsed() / REPS as u32;

            let t1 = Instant::now();
            for _ in 0..REPS {
                let mut par_w = w.clone();
                par_w.par_iter_mut().enumerate().for_each(|(i, w_i)| update_one(i, w_i));
                std::hint::black_box(&par_w);
            }
            let par = t1.elapsed() / REPS as u32;

            println!("m={m:>7} sequential={seq:>10?} rayon={par:>10?} rayon/sequential={:.2}x", par.as_secs_f64() / seq.as_secs_f64());
        }
    }

    fn var(lb: f64, ub: f64) -> VariableData {
        VariableData { vtype: VarType::Continuous, lb, ub }
    }

    fn expr(terms: &[(usize, f64)]) -> LinearExpr {
        LinearExpr { coeffs: terms.iter().cloned().collect::<BTreeMap<_, _>>(), constant: 0.0 }
    }

    fn row(terms: &[(usize, f64)], sense: RowSense, rhs: f64) -> ConstraintRow {
        ConstraintRow { expr: expr(terms), sense, rhs }
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    /// Solves via the interior-point (IP-PMM) engine directly —
    /// `crate::solver::solve_lp` with `RootSolver::Interior` — so
    /// cross-check tests stay genuinely independent of this module.
    /// `solver::solve_lp` now dispatches to either engine on request (see
    /// `types::RootSolver`), so this is the same dispatch
    /// `Model.solve(root_solver="interior")` uses on the Python side, not
    /// a test-only shortcut.
    fn solve_via_ipm(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow]) -> crate::types::SolveResult {
        crate::solver::solve_lp(variables, objective, constraints, crate::types::RootSolver::Interior, crate::types::LpOptions::default())
    }

    #[test]
    fn lp1_maximize_with_le_bounds() {
        // max x + 2y s.t. x+y<=10, x,y in [0,10] -> optimal 20 at (0,10)
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
        let res = solve_lp_dual(&vars, &obj, &cons);
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
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        assert!(approx(res.x.unwrap()[0], -5.0));
    }

    #[test]
    fn infeasible_detected() {
        // w in [0,5], w>=10 -> infeasible
        let vars = vec![var(0.0, 5.0)];
        let obj = Objective { expr: expr(&[(0, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0)], RowSense::Ge, 10.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Infeasible);
    }

    #[test]
    fn dual_bfrt_flips_multiple_variables_in_one_iteration() {
        // min sum_{i=0..9} (i+1)*x_i s.t. sum(x_i) == 5, every x_i in
        // [0, 1]. This is a single-row problem (m=1), so the dual
        // method's crash lands on a single, hugely infeasible row
        // (initial slack = 5, fixed at [0,0]) with 10 equally-eligible
        // candidates whose ratios are exactly their own cost (1..10) —
        // by hand: chuzr always picks the only row; chuzc1 sorts
        // candidates x0..x4 (ratios 1..5) ahead of x5..x9 (ratios 6..10);
        // BFRT's walk fully flips x0..x3 (cost 1-4) from lower to upper
        // bound before x4 (cost 5) is left as the real (here: exactly
        // bound-hitting, degenerate) entering pivot — landing on the
        // provably optimal greedy solution (smallest costs at their upper
        // bound) in a *single* dual-simplex iteration.
        //
        // Without BFRT this same iteration's classical ratio test would
        // pick x0 (smallest ratio) alone as the entering variable with
        // theta = 5, i.e. push x0 to value 5 — past its own upper bound
        // of 1 — which is exactly the incorrectness BFRT exists to
        // prevent, not merely a performance optimization.
        let n = 10;
        let vars: Vec<VariableData> = (0..n).map(|_| var(0.0, 1.0)).collect();
        let obj = Objective { expr: expr(&(0..n).map(|i| (i, (i + 1) as f64)).collect::<Vec<_>>()), sense: Sense::Minimize };
        let cons = vec![row(&(0..n).map(|i| (i, 1.0)).collect::<Vec<_>>(), RowSense::Eq, 5.0)];

        let expected_x = [1.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let check = |x: Vec<f64>| {
            let objective: f64 = obj.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum();
            assert!(approx(objective, 15.0), "objective={objective}, x={x:?}");
            for j in 0..n {
                assert!(approx(x[j], expected_x[j]), "x[{j}]={}, x={x:?}", x[j]);
            }
        };

        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());

        let ipm = solve_via_ipm(&vars, &obj, &cons);
        assert_eq!(ipm.status, Status::Optimal);
        check(ipm.x.unwrap());
    }

    #[test]
    fn parallel_inequality_rows_keep_the_tighter_one() {
        // max 2x+y s.t. x+y<=10 (tight) and 2x+2y<=30 (a scalar multiple,
        // equivalent to x+y<=15, strictly looser), x,y in [0,20].
        // Exercises `crate::presolve::redundancy::reduce_inequalities`:
        // if it kept the *looser* row instead of the tighter one, x+y
        // could reach 15 instead of 10 and the optimum would come out at
        // x=15 (objective 30) rather than the true x=10,y=0 (objective 20).
        let vars = vec![var(0.0, 20.0), var(0.0, 20.0)];
        let obj = Objective { expr: expr(&[(0, 2.0), (1, 1.0)]), sense: Sense::Maximize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 10.0),
            row(&[(0, 2.0), (1, 2.0)], RowSense::Le, 30.0),
        ];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 10.0), "x={x:?}");
        assert!(approx(x[1], 0.0), "x={x:?}");
    }

    #[test]
    fn dualfix_fixes_a_dominated_variable() {
        // min z + x s.t. z+x<=10, z in [2,8], x in [0,10]. `z` appears in
        // only this one `<=` row with a positive coefficient, so it has
        // zero down-lock; its cost (+1) wants it small, so
        // `crate::presolve::dualfix::fix_dominated_variables` should fix
        // it to its own lower bound (2) directly. The true optimum agrees
        // (z=2, x=0, objective=2) regardless of whether DualFix actually
        // fired — a correct simplex finds the same answer on its own —
        // but a *wrong* fix (e.g. to the upper bound, or firing when a
        // real down-lock exists) would show up as a wrong objective here.
        let vars = vec![var(2.0, 8.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 10.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 2.0), "x={x:?}");
        assert!(approx(x[1], 0.0), "x={x:?}");
    }

    #[test]
    fn colsingleton_substitutes_singleton_equality_and_folds_cost() {
        // min -3*x0 + x1 + 2*x2
        // s.t. x0 - x1 - x2 == 0   (x0 is a singleton column: appears
        //                           nowhere else)
        //      x1 + x2 <= 10
        //      x0 in [0,20], x1,x2 in [0,10]
        //
        // `crate::presolve::colsingleton::eliminate_singleton_equalities`
        // should substitute x0 = x1+x2, drop the equality row, and fold
        // x0's cost (-3) into x1/x2's own costs: -3*(x1+x2)+x1+2*x2 =
        // -2*x1 - x2, minimized (i.e. maximizing 2*x1+x2) subject to
        // x1+x2<=10 -- x1 is more valuable per unit, so the optimum
        // spends the whole budget on it: x1=10, x2=0. Back-substitution
        // then gives x0 = x1+x2 = 10, which must still respect x0's own
        // upper bound of 20 (it does) -- the derived box-bound rows this
        // module adds are what guarantee that in general.
        // True optimum: x0=10, x1=10, x2=0, objective = -30+10+0 = -20.
        let vars = vec![var(0.0, 20.0), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -3.0), (1, 1.0), (2, 2.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, -1.0), (2, -1.0)], RowSense::Eq, 0.0),
            row(&[(1, 1.0), (2, 1.0)], RowSense::Le, 10.0),
        ];
        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 10.0), "x={x:?}");
            assert!(approx(x[1], 10.0), "x={x:?}");
            assert!(approx(x[2], 0.0), "x={x:?}");
            let objective: f64 = obj.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum();
            assert!(approx(objective, -20.0), "objective={objective}, x={x:?}");
        };

        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());

        let ipm = solve_via_ipm(&vars, &obj, &cons);
        assert_eq!(ipm.status, Status::Optimal);
        check(ipm.x.unwrap());
    }

    #[test]
    fn freevar_eliminated_through_multiple_equality_rows_end_to_end() {
        // x0 free, x1,x2 in [0,10]. x0 + x1 == 5, x0 - x2 == 1: unlike
        // colsingleton/doubleton (a column appearing in at most 2 rows),
        // this exercises `presolve::freevar::eliminate_free_variables`
        // through the *full* solve path (`build_std_form_presolved` ->
        // `presolve::run_extended`), not just the module's own unit tests.
        // x1 = 5-x0, x2 = x0-1, feasible for x0 in [1,5] (x1,x2 in [0,10],
        // the upper bounds never bind). min 2*x1+x2 = 9-x0 is minimized by
        // maximizing x0, i.e. x0=5, giving x1=0, x2=4, objective=4.
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(1, 2.0), (2, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Eq, 5.0),
            row(&[(0, 1.0), (2, -1.0)], RowSense::Eq, 1.0),
        ];
        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 5.0), "x={x:?}");
            assert!(approx(x[1], 0.0), "x={x:?}");
            assert!(approx(x[2], 4.0), "x={x:?}");
        };

        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());
    }

    #[test]
    fn freevar_leftover_unconstrained_with_nonzero_cost_is_unbounded() {
        // x0 free, appears in no row at all (equality or inequality); its
        // own objective coefficient is nonzero, so the problem is
        // unbounded regardless of x1's own (bounded, irrelevant) row.
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(1, 1.0)], RowSense::Le, 5.0)];

        assert_eq!(solve_lp_dual(&vars, &obj, &cons).status, Status::InfeasibleOrUnbounded);
        let distinguish = crate::types::LpOptions { distinguish_infeasible_unbounded: true };
        assert_eq!(solve_lp_dual_with(&vars, &obj, &cons, distinguish).status, Status::Unbounded);
    }

    #[test]
    fn improving_ray_over_an_infeasible_rest_is_infeasible_when_distinguishing() {
        // Same leftover free x0 (an improving ray, so no finite optimum),
        // but y1 - y2 >= 1, y2 - y3 >= 1, y3 - y1 >= 1 sum to 0 >= 3: no
        // feasible point at all, and bound propagation cannot see it (it
        // only keeps raising the lower bounds). Presolve's ray alone must
        // not be reported as `Unbounded`.
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, f64::INFINITY), var(0.0, f64::INFINITY), var(0.0, f64::INFINITY)];
        let obj = Objective { expr: expr(&[(0, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(1, 1.0), (2, -1.0)], RowSense::Ge, 1.0),
            row(&[(2, 1.0), (3, -1.0)], RowSense::Ge, 1.0),
            row(&[(3, 1.0), (1, -1.0)], RowSense::Ge, 1.0),
        ];

        assert_eq!(solve_lp_dual(&vars, &obj, &cons).status, Status::InfeasibleOrUnbounded);
        let distinguish = crate::types::LpOptions { distinguish_infeasible_unbounded: true };
        assert_eq!(solve_lp_dual_with(&vars, &obj, &cons, distinguish).status, Status::Infeasible);
    }

    #[test]
    fn freevar_leftover_unconstrained_with_zero_cost_fixes_to_zero() {
        // x0 free, appears in no row at all, zero objective coefficient:
        // fixed to 0 at no cost, leaving x1's own optimum (3, at its own
        // lower bound raised by the >= row) untouched.
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(1, 1.0)], RowSense::Ge, 3.0)];

        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        let x = dual.x.unwrap();
        assert!(approx(x[0], 0.0), "x={x:?}");
        assert!(approx(x[1], 3.0), "x={x:?}");
    }

    #[test]
    fn freevar_residual_both_sides_infinite_only_in_inequality_rows_end_to_end() {
        // x0, x1 both genuinely free, and *never* appear in any equality
        // row — only in two pairs of opposing inequality rows that jointly
        // pin x0+x1=5 and x0-x1=1 (x0=3, x1=2) without ever being spelled
        // as an `Eq` row. Single-row interval bound propagation can't
        // tighten either bound here (each row's *other* free term is
        // unbounded in the direction needed), so both variables reach
        // `presolve::freevar` still doubly-infinite and, per its own docs,
        // are left exactly as-is (the "only in an inequality row" residual
        // case) — handed to `extended_dual::solve_lp_dual_extended` as a
        // single unsplit column, both its sides M-tracked directly (that
        // module's own docs). Historically this made `solve_lp_dual`
        // report a false `Infeasible` (`extended_dual::delta_of` silently
        // misclassified a doubly-infinite column as one-sided); first fixed
        // by splitting into `x_j = x_j^+ - x_j^-` before this module ever
        // ran, since superseded by native (unsplit) support.
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(f64::NEG_INFINITY, f64::INFINITY)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 5.0),
            row(&[(0, 1.0), (1, 1.0)], RowSense::Ge, 5.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Le, 1.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Ge, 1.0),
        ];
        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        let x = dual.x.unwrap();
        assert!(approx(x[0], 3.0), "x={x:?}");
        assert!(approx(x[1], 2.0), "x={x:?}");
    }

    #[test]
    fn freevar_residual_only_in_inequality_rows_with_nonzero_objective_end_to_end() {
        // x0 free, x1 in [0,10]; x0 appears only in two inequality rows
        // (never an equality row). Maximize x0 (minimize -x0) subject to
        // x0+x1<=8, x0-x1<=3: optimum at x1=2.5, x0=5.5. Here presolve
        // finds a one-sided implied bound on x0 (not the doubly-infinite
        // case above), exercising the ordinary one-sided-unbounded column
        // path alongside the genuinely-free one.
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 8.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Le, 3.0),
        ];
        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        let x = dual.x.unwrap();
        assert!(approx(x[0], 5.5), "x={x:?}");
        assert!(approx(x[1], 2.5), "x={x:?}");
    }

    #[test]
    fn mutually_coupled_free_variables_reach_a_correct_answer() {
        // x0, x1 both genuinely free, coupled *only* to each other (never
        // anchored by a third column or an equality row) — the true
        // optimum is a whole line (x0 - x1 = -3), not a single point, so
        // whichever of the two ends up nonbasic in `extended_dual`'s own
        // `M`-phase has no real bound to rest at once cleanup tries to pin
        // it down. `extended_dual`'s cleanup parks it at `NbStatus::Zero`
        // (value `0`, the paper's state `Z`) — its
        // `two_free_columns_tied_only_through_opposing_inequality_rows_park_one_at_zero`
        // unit test; this shape used to bail to the classical `BIG_M`
        // fallback instead. This end-to-end test checks the *public*
        // contract: the true optimal objective, not a wrong answer or a
        // panic.
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(f64::NEG_INFINITY, f64::INFINITY)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, -1.0)], RowSense::Le, 3.0), row(&[(0, -1.0), (1, 1.0)], RowSense::Le, 3.0)];
        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        let x = dual.x.unwrap();
        assert!(x[0].is_finite() && x[1].is_finite(), "x={x:?}");
        assert!(approx(x[0] - x[1], -3.0), "x={x:?}");
    }

    #[test]
    fn freevar_in_exactly_one_inequality_row_eliminated_by_presolve_end_to_end() {
        // x0 free, x1 in [0,10]; x0 appears in *exactly one* inequality row
        // (x0+x1<=8, never an equality row) -- unlike the two-appearance
        // test above, `presolve::freevar` now fully eliminates x0 outright
        // (its own new "exactly one inequality-row appearance" case,
        // substituting x0=8-x1 and dropping the row as redundant) rather
        // than leaving it for `build_std_form_presolved`'s x_j=x_j^+-x_j^-
        // split. Minimize -x0: maximized by minimizing x1 (its post-fold
        // cost becomes +1, see `freevar`'s own unit tests for the fold
        // arithmetic), so x1=0, x0=8, objective=-8.
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 8.0)];

        let pf = build_std_form_presolved(&vars, &obj, &cons, true).unwrap();
        assert!(!has_unbounded_structural(&pf), "x0 should be fully eliminated by presolve, never reaching a structural column at all");

        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 8.0), "x={x:?}");
            assert!(approx(x[1], 0.0), "x={x:?}");
        };
        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());
    }

    #[test]
    fn freevar_eliminated_via_equality_row_folds_into_a_shared_inequality_row_end_to_end() {
        // x0 free, x1,x2 in [0,10]. x0 == x1 (an equality row, x0's only
        // `A`-row appearance -- eliminated via it), and separately
        // x0 + x2 <= 5 (an inequality row x0 *also* appears in). Regression
        // test (full solve path) for a real bug: `presolve::freevar`'s
        // `A`-row elimination used to fold a substitution into every other
        // `A` row a variable appeared in, but never into a `real_rows`
        // entry it happened to share -- silently leaving that row
        // referencing a column pinned to `0` downstream, corrupting real
        // Netlib instances (`perold`, `pilot4`) into a false `Infeasible`.
        // Minimize -2*x1 - x2: with the row correctly read as x1+x2<=5,
        // the optimum pushes x1 (weighted higher) to the binding row's
        // full budget: x1=5, x2=0, x0=5, objective=-10. Wrongly reading
        // the row as x2<=5 (x0 misread as fixed to 0) would instead let
        // x1 run away to its own unrelated bound (10) with x2=5, giving a
        // spuriously better-looking but infeasible objective of -25.
        let vars = vec![var(f64::NEG_INFINITY, f64::INFINITY), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(1, -2.0), (2, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, -1.0)], RowSense::Eq, 0.0), row(&[(0, 1.0), (2, 1.0)], RowSense::Le, 5.0)];
        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 5.0), "x={x:?}");
            assert!(approx(x[1], 5.0), "x={x:?}");
            assert!(approx(x[2], 0.0), "x={x:?}");
        };
        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());
    }

    #[test]
    fn freevar_residual_mixed_with_ordinary_and_bounded_columns_end_to_end() {
        // Stresses the compacted-slot numbering (`new_lb`/`new_ub`/the
        // row-building loops' `nj`/slack indices) when a genuinely free
        // column, an ordinary one-sided-infinite column, and a normal
        // bounded column all coexist, plus a genuine equality row (so the
        // `n_eq` slack loop runs too, not just the `g_rows` one). x0, x1
        // free only via inequality pairs pinning x0+x1=5, x0-x1=1 (x0=3,
        // x1=2, same as the pure residual test above — deliberately never
        // tied to x2/x3 by any row, so `presolve::freevar` can't eliminate
        // them through an equality row and this residual case still
        // fires). x2 in [0,inf), x3 in [0,10] tied by an unrelated equality
        // row x2-x3=4: minimized at x3=0, x2=4.
        let vars = vec![
            var(f64::NEG_INFINITY, f64::INFINITY),
            var(f64::NEG_INFINITY, f64::INFINITY),
            var(0.0, f64::INFINITY),
            var(0.0, 10.0),
        ];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0), (2, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 5.0),
            row(&[(0, 1.0), (1, 1.0)], RowSense::Ge, 5.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Le, 1.0),
            row(&[(0, 1.0), (1, -1.0)], RowSense::Ge, 1.0),
            row(&[(2, 1.0), (3, -1.0)], RowSense::Eq, 4.0),
        ];
        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 3.0), "x={x:?}");
            assert!(approx(x[1], 2.0), "x={x:?}");
            assert!(approx(x[2], 4.0), "x={x:?}");
            assert!(approx(x[3], 0.0), "x={x:?}");
        };
        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());
    }

    #[test]
    fn solve_lp_dual_end_to_end_unbounded_structural_column() {
        // x0 in [0,+inf), never referenced by any constraint; cost favors
        // its own infinite side. Exercises the full dispatch path
        // (presolve -> build_std_form_presolved -> extended_dual, not a
        // hand-built StdForm — see `extended_dual::tests` for those).
        let vars = vec![var(0.0, f64::INFINITY), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(1, 1.0)], RowSense::Le, 5.0)];
        assert_eq!(solve_lp_dual(&vars, &obj, &cons).status, Status::InfeasibleOrUnbounded);
        let distinguish = crate::types::LpOptions { distinguish_infeasible_unbounded: true };
        assert_eq!(solve_lp_dual_with(&vars, &obj, &cons, distinguish).status, Status::Unbounded);
    }

    #[test]
    fn lb_unbounded_below_reaches_finite_optimum() {
        // x0 in (-inf, 10], cost favors driving it up toward its own
        // finite bound; x1 in [0,10] just keeps the row genuinely
        // multi-variable. The shift step reflects x0 first
        // (`x0 = 10 - y0, y0 >= 0`) into a one-sided-unbounded-*above*
        // shape (`delta_j = +1`) rather than leaving it unbounded-*below*
        // (`delta_j = -1`) — see that step's own docs. True optimum is
        // x0 = 10 either way; this exercises the reflected path.
        let vars = vec![var(f64::NEG_INFINITY, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 15.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 10.0), "x={x:?}");
        let objective: f64 = obj.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum();
        assert!(approx(objective, -10.0), "objective={objective}, x={x:?}");
    }

    #[test]
    fn solve_lp_dual_end_to_end_finite_optimum_with_unbounded_structural_column() {
        // x0 in [0,+inf), pulled in two directions by two separate
        // 3-variable equality rows (neither a colsingleton nor a
        // doubleton case, so presolve can't eliminate it outright): max
        // x0 s.t. x0<=8 (row0, x1,x2>=0) and x0<=6 (row1, x3,x4>=0) -> the
        // tighter bound (6) wins.
        let vars = vec![var(0.0, f64::INFINITY), var(0.0, 10.0), var(0.0, 10.0), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0), (2, 1.0)], RowSense::Eq, 8.0), row(&[(0, 1.0), (3, 1.0), (4, 1.0)], RowSense::Eq, 6.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 6.0), "x={x:?}");
        let objective: f64 = obj.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum();
        assert!(approx(objective, -6.0), "objective={objective}, x={x:?}");
    }

    /// Whether some structural column of `pf.std` still has a genuine
    /// infinite bound (the columns `extended_dual` tracks symbolically).
    fn has_unbounded_structural(pf: &PresolvedForm) -> bool {
        let n_orig = pf.std.n_total - pf.std.n_rows;
        (0..n_orig).any(|j| pf.std.lb[j] == f64::NEG_INFINITY || pf.std.ub[j] == f64::INFINITY)
    }

    #[test]
    fn surviving_one_sided_bounds_stay_infinite_in_std_form() {
        // Fully bounded problem (no structural column ever infinite): the
        // common case, and the one every existing Netlib-style problem takes.
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 10.0)];
        let pf = build_std_form_presolved(&vars, &obj, &cons, true).unwrap();
        assert!(!has_unbounded_structural(&pf));

        // A one-sided-unbounded structural variable whose favored
        // direction (its cost sign) points at its own infinite bound, with
        // no row of its own for `dualfix`/`propagate` to tighten that
        // bound from: it survives presolve, and its `ub` must stay the
        // true infinity for `extended_dual::solve_lp_dual_extended` to
        // consume — `x0` isn't referenced by `cons` at all here, only `x1` is.
        let vars = vec![var(0.0, f64::INFINITY), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(1, 1.0)], RowSense::Le, 5.0)];
        let pf = build_std_form_presolved(&vars, &obj, &cons, true).unwrap();
        assert!(has_unbounded_structural(&pf));
        assert_eq!(pf.std.ub[0], f64::INFINITY, "a surviving infinite bound must be left in place");
    }

    #[test]
    fn doubleton_eliminates_larger_coefficient_variable_and_rewrites_other_row() {
        // min -3*x0 + x1 + 2*x2
        // s.t. 4*x0 + 2*x1 == 12   (doubleton: |4|>|2|, so x0 is
        //                           eliminated -- x0 = 3 - 0.5*x1 -- unlike
        //                           colsingleton's case, x0 is NOT a
        //                           column singleton: it also appears in
        //                           the row below, which must get
        //                           rewritten, not just the objective)
        //      x0 + x2 <= 8
        //      x1 + x2 <= 10
        //      x0 in [0,20], x1,x2 in [0,10]
        //
        // The equation pins x0 = 3 - 0.5*x1 <= 3 (x1 >= 0), so x0's own
        // upper bound of 20 never binds -- x0's true maximum (most
        // negative objective, since its cost is -3) is reached at x1=0,
        // giving x0=3. Any x1>0 both shrinks x0 (bad, cost -3) and adds
        // its own positive cost (also bad), so x1=0 is doubly optimal;
        // x2's cost is positive (+2), so x2=0 too. True optimum: x0=3,
        // x1=0, x2=0, objective = -9.
        let vars = vec![var(0.0, 20.0), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -3.0), (1, 1.0), (2, 2.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 4.0), (1, 2.0)], RowSense::Eq, 12.0),
            row(&[(0, 1.0), (2, 1.0)], RowSense::Le, 8.0),
            row(&[(1, 1.0), (2, 1.0)], RowSense::Le, 10.0),
        ];
        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 3.0), "x={x:?}");
            assert!(approx(x[1], 0.0), "x={x:?}");
            assert!(approx(x[2], 0.0), "x={x:?}");
            let objective: f64 = obj.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum();
            assert!(approx(objective, -9.0), "objective={objective}, x={x:?}");
        };

        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());

        let ipm = solve_via_ipm(&vars, &obj, &cons);
        assert_eq!(ipm.status, Status::Optimal);
        check(ipm.x.unwrap());
    }

    #[test]
    fn doubleton_chain_within_one_pass_recovers_correctly() {
        // Two doubleton rows sharing a variable, in the same presolve
        // pass, in file order such that the *first*-scanned row's own
        // eliminated variable is expressed in terms of a variable the
        // *second*-scanned row then eliminates in turn -- exercising the
        // "later substitution defines what an earlier one merely
        // referenced" recovery-order requirement within a single
        // `doubleton::eliminate_doubleton_equalities` call, not just
        // across rounds.
        //
        // Row 0: 4*x0 + 2*x1 == 12  -> eliminates x0 (|4|>|2|): x0 = 3 - 0.5*x1
        // Row 1: 5*x1 + 1*x2 == 10  -> eliminates x1 (|5|>|1|): x1 = 2 - 0.2*x2
        // (x1 is claimed as row 1's own elimination *after* row 0 already
        // used it as a live term -- row 0 is scanned first, so nothing
        // stops row 1 from later claiming x1 too.)
        //
        // Ground truth (independent of which variable either row
        // eliminates): parametrize by x0 via both equations directly:
        // x1 = 6 - 2*x0, x2 = 10 - 5*x1 = 10*x0 - 20. Box bounds [0,20]
        // on all three force x0 in [2,3] (x1>=0 needs x0<=3; x2>=0 needs
        // x0>=2). Minimizing -x0 wants x0 as large as possible: x0=3,
        // x1=0, x2=10, objective=-3.
        let vars = vec![var(0.0, 20.0), var(0.0, 20.0), var(0.0, 20.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 4.0), (1, 2.0)], RowSense::Eq, 12.0),
            row(&[(1, 5.0), (2, 1.0)], RowSense::Eq, 10.0),
        ];
        let check = |x: Vec<f64>| {
            assert!(approx(x[0], 3.0), "x={x:?}");
            assert!(approx(x[1], 0.0), "x={x:?}");
            assert!(approx(x[2], 10.0), "x={x:?}");
            let objective: f64 = obj.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum();
            assert!(approx(objective, -3.0), "objective={objective}, x={x:?}");
        };

        let dual = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(dual.status, Status::Optimal);
        check(dual.x.unwrap());

        // Exercises `interior_point.rs`'s `unscale_with_substitutions` on a
        // *chained* doubleton substitution specifically (x0 recovered in
        // terms of x1, x1 in turn recovered in terms of x2 — the exact
        // "later substitution defines what an earlier one merely
        // referenced" ordering `presolve::ExtendedPresolveResult`'s own
        // docs describe), not just the single-substitution colsingleton
        // case `colsingleton_substitutes_singleton_equality_and_folds_cost`
        // already checks via `solve_via_ipm`.
        let ipm = solve_via_ipm(&vars, &obj, &cons);
        assert_eq!(ipm.status, Status::Optimal);
        check(ipm.x.unwrap());
    }

    #[test]
    fn duplicate_equality_rows_via_shared_presolve() {
        // min x+y s.t. x+y==4 (stated three times, one as a scalar
        // multiple) plus x-y==0, x,y in [0,10] -> optimal 4 at (2,2).
        // Exercises `crate::presolve::redundancy::reduce_equalities`
        // through the active engine: `build_std_form_presolved` runs the
        // shared presolve pipeline before ever building a `Tableau`, so
        // the duplicate/scalar-multiple rows below are dropped well
        // before phase 1 (primal) or the dual-feasible crash ever sees
        // them, not merely tolerated by the slack-per-row representation.
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Eq, 4.0),
            row(&[(0, 2.0), (1, 2.0)], RowSense::Eq, 8.0), // scalar multiple of the row above
            row(&[(0, 1.0), (1, 1.0)], RowSense::Eq, 4.0), // exact duplicate
            row(&[(0, 1.0), (1, -1.0)], RowSense::Eq, 0.0),
        ];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 2.0), "x={x:?}");
        assert!(approx(x[1], 2.0), "x={x:?}");
    }

    #[test]
    fn disconnected_model_solves_via_connected_component_split() {
        // Two genuinely independent blocks sharing no variable and no
        // row: block A (vars 0,1) minimize 2*x0+x1 s.t. x0+x1>=4 -- to
        // minimize, prefer the cheaper-per-unit x1, so x0*=0, x1*=4
        // (unique: raising x0 while lowering x1 by the same amount costs
        // strictly more). Block B (vars 2,3,4) minimize -3x2-2x3-x4 s.t.
        // x2+x3+x4<=15 -- a fractional-knapsack shape, greedily filling
        // the highest-coefficient variable first: x2*=10 (its own upper
        // bound), leaving 5 of the row's budget for x3 (next-highest
        // coefficient) at x3*=5, x4*=0. Combined optimum: x*=(0,4,10,5,0),
        // objective = 4 + (-40) = -36, exercising
        // `connected_components_of_std_form` finding exactly 2 components
        // and `build_component_std_form`/`solve_std_form_decomposed`
        // reassembling their independently-solved results correctly.
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0), var(0.0, 10.0), var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective {
            expr: expr(&[(0, 2.0), (1, 1.0), (2, -3.0), (3, -2.0), (4, -1.0)]),
            sense: Sense::Minimize,
        };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Ge, 4.0),
            row(&[(2, 1.0), (3, 1.0), (4, 1.0)], RowSense::Le, 15.0),
        ];
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 0.0), "x={x:?}");
        assert!(approx(x[1], 4.0), "x={x:?}");
        assert!(approx(x[2], 10.0), "x={x:?}");
        assert!(approx(x[3], 5.0), "x={x:?}");
        assert!(approx(x[4], 0.0), "x={x:?}");
        let obj_val = 2.0 * x[0] + x[1] - 3.0 * x[2] - 2.0 * x[3] - x[4];
        assert!(approx(obj_val, -36.0), "obj={obj_val}");
    }

    #[test]
    fn disconnected_model_reports_infeasible_when_either_component_is() {
        // Block A is trivially infeasible on its own (x0 confined to
        // [0,1] but forced >= 5 by its own row); block B is perfectly
        // feasible and independent. The combined model must still report
        // Infeasible overall -- a feasible, unrelated component must never
        // mask another component's genuine infeasibility.
        let vars = vec![var(0.0, 1.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0)], RowSense::Ge, 5.0), row(&[(1, 1.0)], RowSense::Le, 10.0)];
        assert_eq!(solve_lp_dual(&vars, &obj, &cons).status, Status::Infeasible);
    }

    #[test]
    fn many_independent_singleton_components_exercise_the_parallel_split_path() {
        // 250 fully independent one-variable "blocks" (no shared row or
        // variable between any two), clearing `PARALLEL_COMPONENT_MIN_VARS`
        // (200) so `solve_std_form_decomposed` actually dispatches via
        // `rayon` rather than iterating components sequentially. Each
        // block i: minimize -x_i s.t. x_i <= (i % 7) + 1, x_i in [0, 20] --
        // unique optimum x_i* = (i % 7) + 1.
        const N: usize = 250;
        let vars: Vec<VariableData> = (0..N).map(|_| var(0.0, 20.0)).collect();
        let obj = Objective { expr: expr(&(0..N).map(|i| (i, -1.0)).collect::<Vec<_>>()), sense: Sense::Minimize };
        let cons: Vec<ConstraintRow> = (0..N).map(|i| row(&[(i, 1.0)], RowSense::Le, ((i % 7) + 1) as f64)).collect();
        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        for i in 0..N {
            let expected = ((i % 7) + 1) as f64;
            assert!(approx(x[i], expected), "x[{i}]={} expected={expected}", x[i]);
        }
    }

    /// Two independent blocks of `k` variables each, built straight as a
    /// `StdForm` (no presolve to collapse them): block `b` is
    /// `min -sum_j (j+1) x_j  s.t. sum_j x_j <= cap_b,  x in [0, 1]` — a
    /// unit-weight fractional knapsack, so the optimum takes the
    /// `floor(cap_b)` highest-cost variables at 1 and the next one at the
    /// fractional remainder.
    fn two_block_knapsack_std_form(k: usize, caps: [f64; 2]) -> StdForm {
        let n_orig = 2 * k;
        let n_total = n_orig + 2;
        let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
        for b in 0..2 {
            let mut row: Vec<(usize, f64)> = (0..k).map(|j| (b * k + j, 1.0)).collect();
            row.push((n_orig + b, 1.0));
            rows.push(row);
        }
        let (rows, cols) = freeze_std_matrices(&rows, n_total);
        let mut c: Vec<f64> = (0..n_orig).map(|j| -((j % k) as f64 + 1.0)).collect();
        c.extend([0.0, 0.0]);
        let lb = vec![0.0; n_total];
        let mut ub = vec![1.0; n_orig];
        ub.extend([f64::INFINITY, f64::INFINITY]);
        StdForm { n_total, n_rows: 2, c, rows, cols, b: caps.to_vec(), lb, ub }
    }

    #[test]
    fn large_independent_components_are_solved_by_parallel_extended_calls() {
        // 250 variables per block clears `PARALLEL_COMPONENT_MIN_VARS`, so
        // `solve_std_form_decomposed` splits and runs each block's
        // `solve_lp_dual_extended` on rayon.
        const K: usize = 250;
        assert!(K >= PARALLEL_COMPONENT_MIN_VARS);
        let std = two_block_knapsack_std_form(K, [100.5, 30.25]);
        let (components, has_row) = connected_components_of_std_form(&std).expect("two blocks");
        assert_eq!(components.iter().filter(|c| c.len() > 1 || has_row[c[0]]).count(), 2);

        let res = solve_std_form_decomposed(&std, &crate::types::LpOptions::default());
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert_eq!(x.len(), 2 * K);
        for (b, cap) in [(0usize, 100.5f64), (1, 30.25)] {
            let whole = cap.floor() as usize;
            for j in 0..K {
                // Highest cost is the largest in-block index.
                let rank = K - 1 - j;
                let expected = if rank < whole { 1.0 } else if rank == whole { cap - whole as f64 } else { 0.0 };
                assert!(approx(x[b * K + j], expected), "block {b} x[{j}]={} expected={expected}", x[b * K + j]);
            }
        }
    }

    #[test]
    fn component_statuses_combine_soundly() {
        use Status::*;
        let combine = |v: &[Status]| combine_component_statuses(v.iter());
        assert_eq!(combine(&[Optimal, Optimal]), Optimal);
        assert_eq!(combine(&[Optimal, Infeasible]), Infeasible);
        assert_eq!(combine(&[Unbounded, Infeasible]), Infeasible);
        assert_eq!(combine(&[NotSolved, Infeasible]), Infeasible);
        assert_eq!(combine(&[Optimal, Unbounded]), Unbounded);
        // Unbounded needs every other component feasible, which an unsolved
        // one can't vouch for — but "no finite optimum" still holds.
        assert_eq!(combine(&[Unbounded, NotSolved]), InfeasibleOrUnbounded);
        assert_eq!(combine(&[InfeasibleOrUnbounded, NotSolved]), InfeasibleOrUnbounded);
        assert_eq!(combine(&[InfeasibleOrUnbounded, Unbounded]), InfeasibleOrUnbounded);
        assert_eq!(combine(&[Optimal, NotSolved]), NotSolved);
    }

    #[test]
    fn contradictory_equality_rows_detected_infeasible() {
        // x+y==4 and x+y==5 (same coefficients, different rhs) can never
        // both hold: `redundancy::reduce_equalities` keeps the second row
        // as linearly independent in its RHS-augmented sense (see its own
        // module docs), leaving the resulting standard form with two
        // equality rows whose slacks can never simultaneously sit at
        // their fixed [0,0] bound — caught by phase 1 / the dual method's
        // own infeasibility detection, not by presolve itself.
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![
            row(&[(0, 1.0), (1, 1.0)], RowSense::Eq, 4.0),
            row(&[(0, 1.0), (1, 1.0)], RowSense::Eq, 5.0),
        ];
        assert_eq!(solve_lp_dual(&vars, &obj, &cons).status, Status::Infeasible);
    }

    #[test]
    fn single_variable_ge_row_conflicting_with_own_bound_is_infeasible() {
        // A single-variable inequality (`w >= 10`) folds into `G` as a
        // one-entry row exactly like a variable's own bound rows do (see
        // `presolve::propagate::extract_bounds`), so this specifically
        // exercises the box-consistency check `propagate::propagate` runs
        // after merging them: w's own upper bound is 5, contradicting the
        // constraint's implied lower bound of 10, and this must be caught
        // rather than silently producing a `Tableau` with lb > ub for w.
        let vars = vec![var(0.0, 5.0)];
        let obj = Objective { expr: expr(&[(0, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0)], RowSense::Ge, 10.0)];
        assert_eq!(solve_lp_dual(&vars, &obj, &cons).status, Status::Infeasible);
    }

    #[test]
    fn knapsack_lp_relaxation() {
        // max 60x1+100x2+120x3 s.t. 10x1+20x2+30x3<=50, x_i in [0,1]
        // (LP relaxation of the classic 0/1 knapsack instance used
        // elsewhere in this project's test suite).
        let vars = vec![var(0.0, 1.0), var(0.0, 1.0), var(0.0, 1.0)];
        let obj = Objective { expr: expr(&[(0, 60.0), (1, 100.0), (2, 120.0)]), sense: Sense::Maximize };
        let cons = vec![row(&[(0, 10.0), (1, 20.0), (2, 30.0)], RowSense::Le, 50.0)];
        let res = solve_lp_dual(&vars, &obj, &cons);
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

        let res = solve_lp_dual(&vars, &obj, &cons);
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        let obj_val: f64 = obj_terms.iter().map(|&(j, c)| c * x[j]).sum();

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
        let lu = refactorize(&std, &t, None);
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
        let fresh_lu = refactorize(&std, &t, None);
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
