//! A from-scratch **bounded-variable revised simplex**, dual method
//! ([`solve_lp_dual`]) — `solver::solve_lp`'s default engine for every LP
//! `Model.solve()` needs to solve (including once per branch-and-bound
//! node for MIPs; see `mip.rs`), reachable explicitly via
//! `Model.solve(root_solver="simplex")` alongside the interior-point
//! alternative (`types::RootSolver`). A classical primal two-phase method
//! also lives here (`solve_lp_on`, no longer exposed under a standalone
//! `solve_lp` name), kept only as [`solve_lp_dual`]'s own trivial
//! `n_rows == 0` shortcut — every other case, including the last-resort
//! `BIG_M`-substituted fallback when [`extended_dual::solve_lp_dual_extended`]
//! gives up, goes through the classical *dual* method instead (see
//! `build_std_form_presolved`'s own `clamp_unbounded` docs).
//!
//! ## The bounded-variable invariant
//!
//! Every **structural** (user-facing) variable has two *finite* bounds —
//! `model.rs::add_variable` rejects infinite `lb`/`ub` at the PyO3
//! boundary, so this module never has to represent a free or one-sided
//! variable and treats that as a precondition throughout (`NbStatus::Zero`,
//! the one "free" case, exists only for `extended_dual`'s own use). This is also what makes
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
//! ## Dual method: Devex→DSE pricing, BFRT, incremental reduced costs
//!
//! [`solve_lp_dual`] needs no phase 1 (a dual-feasible start is always
//! constructible — see [`Tableau::crash_dual_feasible`]) and prices the
//! leaving row ([`EdgeWeights`]) starting with cheap Devex weights
//! ([`DevexState`]), escalating one-way to exact dual steepest-edge weights
//! ([`DseState`], per Huangfu & Hall, "Parallelizing the dual revised
//! simplex method", arXiv:1503.01889, §2.2) only if progress over a rolling
//! window of pivots stagnates relative to the cost data's scale — see
//! [`DEVEX_STAGNATION_WINDOW`]'s own docs. The entering-column ratio test is enhanced
//! with the bound-flipping ratio test (BFRT, same paper §2.2.2/2.2.3):
//! candidates are sorted by ascending ratio and every finitely-bounded one
//! ahead of the real entering variable is fully flipped to its opposite
//! bound in the same iteration, rather than needing its own pivot — see
//! [`solve_lp_dual_on`]'s body for the derivation. Reduced costs (`d`) are
//! maintained incrementally across pivots (update-dual, same section) —
//! `d[j] -= theta_d * a_p[j]` for every column, straight off the pivotal
//! row PRICE already computes — rather than recomputed from a fresh BTRAN
//! every iteration. Not (yet) implemented: a genuine two-pass Harris
//! refinement on top of BFRT (a flat ratio-space window is live — see
//! [`HARRIS_RATIO_TOL`]'s own docs, including why *two* more faithful
//! versions — a per-row pivot-scaled Harris window, and a direct port of
//! HiGHS's own `chooseFinalLargeAlpha` — were each tried and reverted
//! after breaking `cycle`'s reported optimum), and dual-specific
//! EXPAND-style anti-cycling (the dual method still falls back to plain
//! Bland's rule under stalling — see `bland_mode`).
//!
//! ## Parallelism: presolve yes, the per-iteration loop no
//!
//! `crate::presolve` (scaling, redundancy, propagation, DualFix,
//! ColSingleton) runs once per solve and is parallelized with rayon
//! throughout. Every loop *inside* [`solve_lp_dual_on`]'s and
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
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Markowitz-pivoted sparse LU + Forrest-Tomlin incremental updates —
/// see `lu`'s own module docs. Referred to below as `sparse_lu` (not the
/// bare module name `lu`) purely to avoid clashing with the many local
/// variables/parameters through this file that are themselves named `lu`
/// (an `FtLu` instance, the live basis factorization).
mod lu;
use self::lu as sparse_lu;

mod extended_dual;

const TOL: f64 = 1e-9;

/// Floor for [`max_iters_for`]'s size-scaled cap — the crate's own
/// historical fixed value, kept as a lower bound so every small/medium
/// instance that already solved within it (the whole Netlib 73-problem
/// `--max-vars 3000` set, per `netlib_benchmark_workflow`) sees no
/// behavior change at all.
const MAX_ITERS_FLOOR: usize = 20_000;

/// Absolute ceiling on [`max_iters_for`]'s output — a defensive backstop,
/// not a value any known Netlib instance approaches (the largest, `dfl001`
/// at `m=4554`/`n_total=9773` post-presolve, needs `MAX_ITERS_SCALE *
/// (m+n_total) = 286,540`, far below this), so a pathological future
/// instance can't turn an unbounded-looking cycle into a multi-hour hang.
const MAX_ITERS_CEILING: usize = 2_000_000;

/// Multiplier applied to `m + n_total` by [`max_iters_for`].
const MAX_ITERS_SCALE: usize = 20;

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
/// the caller falls back to the classical `BIG_M`-substituted path, which
/// is *also* numerically fragile (see `BIG_M`'s own docs /
/// `[[bigm-fallback-invalid-reference]]`) and returned a **wrong**
/// objective on `dfl001` (`11264657.2` vs HiGHS's `11266396.0`, ~1.5e-4
/// relative error) rather than the correct answer the uncapped loop
/// reaches directly. See the `dfl001-bottleneck-max-iters-cap` memory for
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

/// Primal feasibility tolerance for chuzr's basic-variable bound check —
/// deliberately separate from (and looser than) `TOL`, which stays tight
/// for "is this coefficient/ratio exactly zero"-type tests. Matches
/// HiGHS's own default `primal_feasibility_tolerance` (1e-7, `HighsOptions`)
/// rather than reusing `TOL`'s 1e-9: confirmed on real data (Netlib's
/// `agg`, whose bounds/data reach the millions) that `TOL` alone is too
/// tight for a basic variable's accumulated floating-point noise once the
/// problem's own natural scale is large — a variable sitting `-6.25e-9`
/// off its own `lb = 0` (utterly negligible against that scale) still
/// cleared the old `> TOL` (1e-9) threshold, was treated as a genuine
/// primal infeasibility for chuzr to "fix", and — since it wasn't a real
/// one — sometimes left chuzc1/BFRT with no genuine way to fix it,
/// surfacing as a false `Infeasible` report on an actually-optimal LP.
const PRIMAL_FEAS_TOL: f64 = 1e-7;

/// Harris (1973) two-pass ratio test tolerance for the dual method's BFRT
/// (Huangfu & Hall §2.2.2's plain single-boundary BFRT, used until now,
/// picks strictly the smallest-ratio candidate that stops the walk — with
/// no regard for how small its own pivot `a_pj` is. A pivot near zero can
/// make the resulting basis numerically singular even though the LP
/// itself is perfectly well-posed: confirmed on real data (the Netlib LP
/// set's `wood1p`), where the basis `factorize()` eventually rejected as
/// singular had rank 242/243 with a smallest singular value at machine
/// epsilon — not a `factorize()` bug, but a pivot this ratio test should
/// never have accepted). This tolerance widens the acceptance window
/// *backward* (see the BFRT loop below for why backward, not forward) so
/// a slightly smaller-ratio candidate with a much better-conditioned
/// pivot can be chosen instead, at the cost of a small, bounded amount of
/// ratio sub-optimality (more dual-simplex iterations to fully restore
/// optimality, not a feasibility violation). Inspired by HiGHS's
/// `HEkkDualRow::chooseFinal`/`chooseFinalLargeAlpha` (`HEkkDualRow.cpp`),
/// which searches a similar tolerance window but can safely extend
/// forward too, since its bookkeeping (a budget over total remaining
/// infeasibility) isn't tied to a step-by-step walk the way this crate's
/// BFRT loop is.
///
/// Widening this constant was tried directly against the Netlib LP set
/// (1e-7, 1e-5, 1e-4) expecting a monotonic improvement in how many
/// instances crash or mis-report infeasible — it wasn't monotonic. 1e-5
/// fixed some instances 1e-7 missed (`pilot.ja`, `pilotnov`, `qap8`) but
/// broke one 1e-7 got right (`bnl1`) and added a new false-infeasible
/// (`pilot4`); 1e-4 was worse on both counts than either. This is a
/// whack-a-mole pattern, not a tuning curve with a clear optimum — the
/// affected instances (`agg`, `cycle`, `degen2`, `degen3`, `maros`,
/// `perold`, `pilotnov`, `stair`, `vtp.base`, ...) are also independently
/// known in the LP literature as highly degenerate stress tests for
/// anti-cycling machinery specifically, which points at the *actual* gap:
/// this dual method has no anti-degeneracy mechanism of its own (the
/// primal method's EXPAND, per this crate's own history, was never
/// ported to the dual side). A pivot-tolerance constant can only ever
/// paper over that, not fix it — kept at 1e-7 (tied for best of the three
/// tried, and the smallest deviation from ratio-optimality of the tied
/// pair) rather than tuned further.
///
/// **A genuine per-row-scaled version (Harris's actual `r_i + tol/|alpha_i|
/// >= r_stop`, i.e. using `HARRIS_RATIO_TOL / a_pj.abs()` as each
/// candidate's own admission threshold instead of subtracting this flat
/// constant from `stop_idx`'s ratio) was implemented and benchmarked
/// against the same Netlib set, then reverted** — see the BFRT block's
/// pass-2 comment for the full mechanism. Summary: +9.7% aggregate wall
/// time, concentrated entirely in the degenerate instances already named
/// above (`cycle`, `grow22`, `degen3`, `perold`, `fit1p`, `bnl1`,
/// `pilot4`), zero instances newly fixed, and on `cycle` a silently wrong
/// reported optimum. The per-row scaling is real Harris (1973), not a
/// simplification of it, but it widens admission using the *substitute*
/// candidate's own pivot — exactly backwards from what this function's
/// safety argument needs, which is a bound on the *skipped* candidates'
/// worst-case tolerance.
///
/// **A second, independently-sourced attempt was also tried and reverted**:
/// a direct port of HiGHS's real `HEkkDualRow::chooseFinalLargeAlpha`
/// (`highs/simplex/HEkkDualRow.cpp`, read from source, not memory) — an
/// *absolute* pivot-magnitude floor over the candidate pool rather than any
/// ratio-space window, substitution attempted only when `stop_idx`'s own
/// pivot fails it, nearest-clearing-candidate-wins rather than best-in-reach.
/// Faithful to the real source, and it *still* broke `cycle` (a different
/// wrong objective than the first attempt). A follow-up A/B isolated the
/// fault to substitution *at all*, not to either rule's shape: with pass 2
/// short-circuited to a no-op, `cycle` solves correctly. See the BFRT
/// block's pass-2 comment for the full diagnosis — this crate's flip/theta/
/// dual-update accounting doesn't actually defend dual feasibility for the
/// candidates skipped between a substitute and `stop_idx`, only primal
/// non-overshoot of the flip itself; the flat window below stays because it
/// is empirically narrow enough, on the full Netlib set including `cycle`
/// itself, that this gap never becomes large enough to surface.
const HARRIS_RATIO_TOL: f64 = 1e-7;

/// The primal method's own analogue of [`HARRIS_RATIO_TOL`] above, for
/// exactly the same reason: `run_phase`'s two-pass ratio test already
/// widens its acceptance window by the *EXPAND* working tolerance
/// (`expand.delta`, §4.2) to prevent cycling, but that tolerance is
/// designed to be minuscule (order `1e-6`, per [`EXPAND_DELTA_F`]) — its
/// job is proving a strictly positive step exists, not steering the
/// ratio test toward a numerically better-conditioned pivot the way this
/// constant does. When only one candidate row falls inside that
/// razor-thin window, Pass 2's own "largest pivot magnitude" tie-break
/// has nothing to choose between and must accept whatever pivot that one
/// candidate has, however close to zero — confirmed on Netlib's
/// `forplan` (unrelated to EXPAND/cycling: a perfectly ordinary
/// non-degenerate phase-2 iteration, iteration 118, picked a pivot of
/// `~2.4e-9`, well under [`FT_MIN_PIVOT`], which then made the
/// mid-solve refactorization triggered by rejecting it find the *basis
/// itself* singular — not a `factorize()` bug, the same class of finding
/// `HARRIS_RATIO_TOL`'s own docs describe for `wood1p`). Widening the
/// Pass 2 admission window by this much lets a slightly-worse-ratio
/// candidate with a far better pivot be picked instead, at the cost of a
/// small bounded overshoot past the exact leaving bound — well within
/// the same "temporary infeasibility, cleaned up later by
/// `expand_reset_nonbasics`" tolerance this function's EXPAND step
/// already accepts by design, so this doesn't weaken any invariant the
/// algorithm didn't already rely on.
const PRIMAL_HARRIS_TOL: f64 = 1e-7;

/// Threshold below which a pivot's actual contribution to the objective
/// (`theta_q * dj_q`) counts as "no real progress" for `bland_mode`'s
/// stall counter — see that flag's own docs.
const STALL_PROGRESS_EPS: f64 = 1e-9;

/// Rolling-window size (in pivots) for the dual method's Devex-to-DSE
/// pricing escalation — see [`EdgeWeights`]'s own docs. Deliberately much
/// wider than a single `STALL_PROGRESS_EPS` check: that one flags an
/// individual near-zero pivot (Bland's-rule territory), while this one
/// asks a coarser question — "has the *average* pivot over a real stretch
/// of iterations stopped contributing much, relative to the cost data's
/// own scale" — which needs enough pivots averaged together that a few
/// genuinely-degenerate-but-isolated steps don't trigger it by themselves.
const DEVEX_STAGNATION_WINDOW: usize = 20;

/// Relative threshold (as a fraction of the cost vector's own scale,
/// `obj_scale` in [`solve_lp_dual_on`]) below which the
/// [`DEVEX_STAGNATION_WINDOW`]-pivot rolling average objective
/// contribution counts as "stagnating" for the Devex→DSE escalation.
/// Chosen small enough that a healthy solve made of many small-but-real
/// pivots (common once a solve is close to optimal) doesn't false-trigger
/// — the escalation only exists to rescue a solve whose *cheap, approximate*
/// Devex weights are steering `chuzr` toward pivots that are barely moving
/// the objective at all, not to fire on ordinary end-of-solve slowdown.
const DEVEX_STAGNATION_REL_TOL: f64 = 1e-9;

/// Absolute pivot-magnitude threshold below which a single dual pivot
/// counts as "ill-conditioned" and triggers `solve_lp_dual_on`'s
/// in-place Devex->DSE switch (see that function's own docs) —
/// independent of, and (confirmed on Netlib's `cycle`/`forplan`) firing
/// far sooner than, [`DEVEX_STAGNATION_WINDOW`]'s objective-progress
/// check: a pivot with `|alpha[p]|` already this small is direct evidence
/// that Devex's cheap, approximate weights just steered `chuzr` toward a
/// poorly-conditioned row — on both instances, the objective kept
/// improving at an entirely ordinary rate right up until a single pivot
/// this tiny forced a refactorization that then found the basis itself
/// numerically singular, so the rolling-average trigger alone never saw
/// anything worth escalating over. Chosen two orders of magnitude above
/// [`FT_MIN_PIVOT`] itself (the point `try_update` would reject the pivot
/// outright and force that refactorization) so the switch this triggers
/// fires with real margin to spare, not only once a pivot is already
/// unusable.
const DEVEX_ILLCOND_PIVOT_TOL: f64 = 1e-5;

/// Trigger (2): an FT update whose resulting pivot is smaller than this
/// is rejected by `FtLu::try_update`, forcing an immediate refactorization.
const FT_MIN_PIVOT: f64 = 1e-7;
/// Cadence (in iterations) for triggers (1) and (3).
const FT_CHECK_INTERVAL: usize = 5;
/// Trigger (1)'s own check (`compute_rhs` + `basis_residual_norm`, an
/// `O(nnz(A))` scan plus a full basis-matrix multiply) runs only once
/// every this many [`FT_CHECK_INTERVAL`]-cadence checks — i.e. every
/// `FT_CHECK_INTERVAL * RESIDUAL_CHECK_MULTIPLIER` iterations — rather
/// than every single one, per [`FT_RESIDUAL_TOL`]'s own docs: measured
/// residuals on this crate's target problem sizes stay around
/// `1e-11..1e-12`, seven to eight orders of magnitude below the `1e-4`
/// trigger, so this check has enormous slack before genuine drift could
/// ever approach it — checking it this much less often still catches real
/// drift long before it matters, while no longer paying `compute_rhs`'s
/// full-matrix cost on every one of trigger (3)'s own (cheap,
/// `fill_count()`-only) checks. Trigger (3) itself is unaffected — its own
/// `fill_count()` check (an `O(1)`-ish length sum, no matrix scan) still
/// runs every `FT_CHECK_INTERVAL` iterations, and still supplies `rhs` for
/// the resync whenever it actually fires, since `compute_rhs` is only
/// skipped when *neither* trigger has a reason to run this round.
const RESIDUAL_CHECK_MULTIPLIER: usize = 20;
/// Trigger (1): refactor if the true-basis residual exceeds this. In
/// practice this essentially never fires (measured residuals on this
/// crate's target problem sizes stayed around 1e-11..1e-12, several
/// orders of magnitude below even the old 1e-6) — trigger (3) below is
/// what actually governs refactorization frequency — but it costs nothing
/// to leave a wide safety margin here too.
const FT_RESIDUAL_TOL: f64 = 1e-4;
/// Trigger (3): refactor if accumulated eta-file fill exceeds this factor
/// times the basis dimension. This is the trigger that actually fires
/// repeatedly in practice (confirmed by instrumenting a 1000-variable
/// benchmark: every refactorization past the first was this one) — and per
/// that same instrumentation, `fill_count()` grows *compounding*, not
/// linearly, in the update count (each FTRAN/BTRAN walks every existing
/// `U`-eta/`R`-eta, so a longer eta chain smears more fill into every
/// subsequent update, roughly doubling the per-update fill rate every ~50
/// updates in that benchmark). This constant trades more accumulated
/// Forrest-Tomlin eta fill (slower FTRAN/BTRAN per iteration, since each
/// solve walks every eta since the last refactorization) for fewer, less
/// frequent Markowitz refactorizations, and the empirical trade curve is
/// *not* "smaller is safer": a direct A/B sweep on the same benchmark
/// (total wall time across 10 solves) found lowering this factor makes
/// solves slower, not faster — `4` and `2` cost ~10% and ~65% *more* total
/// time than `8` did, because refactorization has its own fixed cost (a
/// full Markowitz factorize plus a full fresh-reduced-cost recompute) that
/// more frequent triggering pays more often than the FTRAN/BTRAN savings
/// recoup. Raising it instead helped monotonically up to a point:
/// `16`/`32`/`64` measured ~5%/~12%/~15% *faster* than `8` on that same
/// benchmark, `128` gave no further improvement (the gain plateaus once
/// refactorizations become rare enough that trigger (4)'s `FT_MAX_UPDATES`
/// cap — or genuine numerical drift via trigger (1) — would dominate
/// instead). `64` is kept rather than pushing further, since it already
/// captures the measured gain with a comfortable margin before the point
/// where a large eta file's numerical safety margin would need
/// re-examining. Re-benchmark if this crate's typical problem shape
/// changes significantly.
const FT_BUMP_LIMIT_FACTOR: usize = 64;
/// Trigger (4): refactor unconditionally once the update count passes
/// this. Measured to be the very first refactorization in a solve (fired
/// once, right around 100, before trigger (3) ever got a chance to) —
/// raising it lets a solve run further into trigger (3)'s own eta-fill
/// budget before this unconditional cap would cut in first.
const FT_MAX_UPDATES: usize = 300;

/// Trigger (5), HiGHS `HEkkDualRow::updateVerify` equivalent: refactor if
/// the pivot element this iteration is about to commit disagrees, by more
/// than this relative amount, between its two independent sources — PRICE's
/// row-direction value (`a_p[q]`, from `rho_p^T A`) and FTRAN's
/// column-direction value (`alpha_buf[p]`, from `B^-1 A_q`). Both are exact
/// in infinite precision; a real gap between them means the Forrest-Tomlin
/// eta chain (`lu`) has already drifted enough to misrepresent `B^-1` by
/// this iteration, *before* that error is baked into `x_B`/`d` by the
/// primal/dual updates that would otherwise follow immediately.
///
/// This is deliberately a *different, earlier* signal than the existing
/// triggers above: trigger (1) (`FT_RESIDUAL_TOL`) only re-checks
/// `‖A_B x_B - rhs‖` every `FT_CHECK_INTERVAL * RESIDUAL_CHECK_MULTIPLIER`
/// iterations, so a bad pivot can still be committed (and the resulting
/// drift compounded by further updates) for up to that many iterations
/// before it's caught; trigger (2) (`FT_MIN_PIVOT`) only rejects a pivot
/// whose magnitude is small in an absolute sense, which says nothing about
/// whether the *value itself* is still accurate. `updateVerify` instead
/// checks every single pivot, immediately, using values both already
/// computed as ordinary byproducts of this iteration's own BTRAN/PRICE and
/// FTRAN (see `solve_lp_dual_on`'s main loop) — no extra BTRAN/FTRAN call
/// is added to pay for it.
///
/// Chosen at `1e-7`, two orders of magnitude above the `~1e-9` (`TOL`)
/// rounding-noise floor a healthy sparse dot-product/triangular-solve pair
/// of this size actually exhibits on this crate's target problem sizes, and
/// matching [`FT_MIN_PIVOT`]'s own order of magnitude (the point below
/// which a pivot is rejected outright regardless of what triggered the
/// check) rather than [`FT_RESIDUAL_TOL`]'s much looser `1e-4` (that
/// constant bounds a *whole-basis* aggregate residual after many updates,
/// not one freshly computed pivot pair — reusing it here would let real
/// per-pivot drift accumulate for a long time before firing). Set too
/// tight, this fires on ordinary floating-point noise and forces far more
/// refactorizations than the drift it exists to catch would ever justify
/// (each refactorization is a full Markowitz factorize plus a full fresh
/// reduced-cost recompute — not cheap, see [`FT_BUMP_LIMIT_FACTOR`]'s own
/// docs on that trade-off); set too loose, it never fires before trigger
/// (1) would have caught the same drift anyway, making it dead code. `1e-7`
/// is the recommended starting point from the port's own spec, not yet
/// independently re-tuned against this crate's Netlib benchmark set beyond
/// the sweep recorded in this feature's own commit message — re-measure
/// with `ENOMOTO_PROF_UPDATE_VERIFY` (below) before moving it.
pub(super) const UPDATE_VERIFY_TOL: f64 = 1e-7;

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
    let scale = alpha_row.abs().max(alpha_col.abs()).max(FT_MIN_PIVOT);
    let rel = (alpha_row - alpha_col).abs() / scale;
    rel <= UPDATE_VERIFY_TOL
}

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

/// Bland's-rule (1977) last-resort anti-cycling fallback for the primal
/// method — the same role `solve_lp_dual_on`'s own `stall_count`/
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

/// Weights never allowed to fall below this — guards against a tiny or
/// negative value (from accumulated rounding) making a column look
/// spuriously "steep".
const STEEPEST_EDGE_FLOOR: f64 = 1e-10;

/// Above this many rows/elements, prefer `rayon`'s parallel iterator over
/// a plain sequential loop for `chuzr`'s row scan and `DseState`'s weight
/// update (both O(one problem dimension) per pivot); `scaling::compute`'s
/// own column-norm fold uses the same constant against its own combined
/// `A`/`G` row count.
///
/// Replaces an earlier "run both ways once, time them, keep the faster"
/// self-calibration at all three call sites: this crate's own
/// `#[ignore]`d microbenchmarks (`rayon_threshold_microbench`,
/// `dse_update_rayon_threshold_microbench`,
/// `col_norm_fold_rayon_threshold_microbench`) never found `rayon` beating
/// a plain sequential scan at *any* size tried on this crate's own
/// development machine — not at `n`/`m` = 200,000, not even at 4,000,000
/// rows for the scaling fold (rayon's own fixed per-call dispatch/thread-
/// wake cost dominates every one of these workloads' actual per-element
/// work) — so the live race was pure overhead on every solve for zero
/// benefit at this crate's realistic problem sizes (Netlib-scale LPs, at
/// most a few thousand rows), and one more source of run-to-run
/// nondeterminism to reason about (see the `chuzr` tie-breaking fix
/// elsewhere in this file, prompted by exactly that). `100_000` sits an
/// order of magnitude above the largest size any of those microbenchmarks
/// found `rayon` still losing at, as a nominal safety valve for a
/// hypothetical future problem far past anything this crate currently
/// targets — not a proven crossover point (none was found).
const RAYON_SIZE_THRESHOLD: usize = 100_000;

/// Below this many structural+slack columns, pricing every nonbasic
/// column's reduced cost every iteration is cheap enough that partial
/// pricing would only add overhead for no benefit; at or above it, both
/// `run_phase`'s primal entering-variable scan and `solve_lp_dual_on`'s
/// `chuzc1` switch to [`partial_pricing_sampled`]'s random-group scheme.
const PARTIAL_PRICING_THRESHOLD: usize = 300;

/// Group count for partial pricing: roughly `1/PARTIAL_PRICING_GROUPS` of
/// eligible candidates are priced first; the rest are only priced if that
/// first group has nothing improving (see [`partial_pricing_sampled`]).
const PARTIAL_PRICING_GROUPS: u64 = 10;

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
    fn update_after_pivot(&mut self, t: &Tableau, std: &StdForm, rho: &[f64], w: &[f64], gamma_t_old: f64, pivot: f64) {
        // Every column's updated weight only depends on its own (previous)
        // old weight plus `rho`/`w`/`gamma_t_old`/`pivot`, all read-only
        // here, so these writes to `self.gamma` are disjoint per column —
        // safe to parallelize. Left sequential anyway: for the column
        // counts this crate actually sees, rayon's per-call dispatch
        // overhead measured *larger* than the loop body itself (see
        // `solve_lp_dual_on`'s module docs for the profiling that found
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
            *gamma_j = (*gamma_j + beta_j * beta_j * (1.0 + gamma_t_old) - 2.0 * beta_j * tau_j).max(STEEPEST_EDGE_FLOOR);
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

/// Ruiz-scaling iterations for the shared presolve pass below — the same
/// value `interior_point.rs` used before `crate::presolve` was extracted
/// out to be shared with this module.
const RUIZ_ITERS: usize = 10;
/// Constraint-propagation passes per presolve round (`propagate::propagate`'s
/// own internal bound-tightening loop — see its module docs for the §3.2
/// activity-bound derivation each pass repeats).
const PROPAGATION_PASSES: usize = 2;
/// Upper bound on how many times `presolve::run_extended` cycles through
/// propagate → dualfix → row-singleton → doubleton → colsingleton — see
/// that function's own docs for why later rounds can unlock reductions an
/// earlier round's static structure couldn't yet see, and for the
/// fixpoint check that stops it short of this cap once a round finds
/// nothing left to do (so raising this constant costs nothing on a
/// problem that stops converging early — only genuinely deep elimination
/// chains ever run all the way to the cap).
const PRESOLVE_ROUNDS: usize = 20;
/// Upper bound on how many times each outer `PRESOLVE_ROUNDS` pass itself
/// cycles through row-singleton <-> colsingleton before `propagate`/
/// `dualfix` run again — see `presolve::run_extended`'s own docs for why
/// this inner pair can have more to find after its own first pass (e.g.
/// colsingleton eliminating a variable turning a row rowsingleton had no
/// reason to touch into a fresh row singleton), for why `doubleton` isn't
/// part of this inner repetition (measured net regression when it was),
/// and for the fixpoint check that stops this loop short of its own cap
/// once a pass finds nothing left to do.
const ROWSINGLETON_COLSINGLETON_INNER_ROUNDS: usize = 1;
/// Finite stand-in for a genuine `+/-inf` bound that survives all the way
/// through `presolve::run_extended` without being eliminated outright —
/// see [`build_std_form_presolved`]'s own docs for why this substitution
/// happens *here*, after presolve, rather than at `model.rs::add_variable`
/// (the old approach, which prevented `colsingleton`/`doubleton` from ever
/// recognizing a free variable's elimination as free in the first place).
/// Same magnitude the Python benchmark harness used to substitute before
/// this crate accepted real infinities at all (`BIG_M` in
/// `python/enomoto_solver/benchmark_highs.py`) — large enough that a
/// genuinely bounded Netlib optimum should sit nowhere near it, small
/// enough to stay well inside `f64` arithmetic's comfortable range.
const BIG_M: f64 = 1e7;

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
    /// `true` iff some structural column still had a genuine infinite
    /// bound after presolve on at least one side (free variables reachable
    /// through an equality row are always eliminated by
    /// `presolve::freevar` before this point, but that module's own docs
    /// name one residual case it cannot soundly resolve itself — a free
    /// variable appearing only in an inequality row — so a genuinely
    /// *both*-sided-infinite column can still reach here, kept as a single
    /// unsplit column: `extended_dual::hat_lower`/`hat_upper` track both
    /// its sides directly, see that module's own docs).
    /// Whether `std.lb`/`std.ub` actually still carry a surviving one-sided
    /// infinity, or had it `BIG_M`-substituted, depends on the
    /// `clamp_unbounded` flag `build_std_form_presolved` was called with —
    /// see that parameter's own docs. `solve_lp_dual` uses this flag to
    /// dispatch to `extended_dual::solve_lp_dual_extended` instead of the
    /// classical bounded-variable path.
    had_unbounded_structural: bool,
}

/// `clamp_unbounded`: whether a surviving one-sided infinite structural
/// bound should be substituted with the numeric [`BIG_M`] sentinel (`true`
/// — the only remaining caller is [`solve_lp_dual`]'s own fallback path,
/// taken when [`extended_dual::solve_lp_dual_extended`] gives up: it
/// re-presolves with this set and falls back to the *classical* bounded-
/// variable dual method, `BIG_M`-substituted, rather than the M-free
/// extended one. A standalone primal entry point used to pass this
/// unconditionally too — removed once every production call path settled
/// on `solve_lp_dual`) or left as a genuine infinity (`false`, what
/// [`solve_lp_dual`] always
/// passes: `had_unbounded_structural` on the result tells it whether to
/// route to [`extended_dual::solve_lp_dual_extended`] instead of the
/// classical path — see that field's own docs). Harmless either way when
/// no such bound survives (the substitution loop below is a no-op then
/// regardless), which is the case for every ordinary (fully bounded)
/// problem.
fn build_std_form_presolved(
    variables: &[VariableData],
    objective: &Objective,
    constraints: &[ConstraintRow],
    clamp_unbounded: bool,
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

    let pre = presolve::run_extended(n, &a, &b, &g, &h, &c0, RUIZ_ITERS, PROPAGATION_PASSES, PRESOLVE_ROUNDS, ROWSINGLETON_COLSINGLETON_INNER_ROUNDS, allow_unbounded_verdict);
    if std::env::var("ENOMOTO_DEBUG_PRESOLVE_INFEAS").is_ok() {
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
    // to anchor to and the later [`BIG_M`] substitution already places it
    // symmetrically around `0`. This is purely a change of coordinate
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

    // Whether any *structural* column still carries a genuine `+/-inf`
    // bound at this point. Usually one-sided only (`lb[j]==-inf xor
    // ub[j]==+inf` — free variables reachable through an equality row are
    // gone by now, see `presolve::freevar`'s own docs), but *can* still be
    // genuinely both-sided: that module's own docs name one residual case
    // it cannot soundly resolve itself — a free variable appearing only in
    // an inequality row — so `lb[j]==-inf && ub[j]==+inf` both is possible
    // here too (confirmed reachable: a hand-built LP with two free
    // variables tied only through opposing inequality-row pairs, never an
    // equality row — `extended_dual`'s own `finish` docs name this exact
    // shape as the one case it still can't resolve internally, falling
    // back to the classical `BIG_M` path below instead). Detected *before*
    // the `BIG_M` substitution below so it reflects the true problem
    // shape, not the sentinel this function still falls back to
    // substituting for now.
    let had_unbounded_structural = (0..n).any(|j| lb[j] == f64::NEG_INFINITY || ub[j] == f64::INFINITY);
    if std::env::var("ENOMOTO_DEBUG_UNBOUNDED_VARS").is_ok() && had_unbounded_structural {
        eprintln!(
            "PRESOLVE: {} structural column(s) still have a genuine infinite bound ({})",
            (0..n).filter(|&j| lb[j] == f64::NEG_INFINITY || ub[j] == f64::INFINITY).count(),
            if clamp_unbounded { "BIG_M-substituted for this call" } else { "left genuinely infinite for the extended dual simplex" }
        );
    }

    // Any variable presolve never fully eliminated may still carry a
    // *genuine* `+/-inf` bound now (`model.rs::add_variable` allows one —
    // see its own docs) — the same finite-bounds need just described
    // applies to it too, so it gets [`BIG_M`] here instead of `0`/`0`.
    // Doing this *after* `run_extended` returns, rather than clamping
    // every bound to `BIG_M` before presolve ever sees the problem (the
    // old design, and what the Python benchmark harness's own `_finite`
    // substitution used to paper over from outside this crate), is the
    // entire point: a variable presolve *did* eliminate above never
    // reaches this loop at all (already finite `0`/`0`), so every
    // genuinely free column a `colsingleton`/`doubleton` chain could
    // cascade through gets the chance to be eliminated outright, at zero
    // replacement-row cost, before anything here ever fixes it to a
    // sentinel value instead.
    //
    // `lb`/`ub` here are already in *scaled* coordinates (see this
    // function's own module docs on why every substitution/bound
    // `run_extended` returns is), so `BIG_M` itself — an original-units
    // constant — needs the same `/ d[j]` conversion every genuinely finite
    // bound already got for free by flowing through `build_a_g` ->
    // `scaling::apply` before ever reaching here (a box row's scaled
    // coefficient is `e_g[i] * d[j]` against a scaled `h` of
    // `original_bound * e_g[i]`, i.e. `x'_j <= original_bound / d[j]`).
    // Substituting a flat `BIG_M` directly into scaled space instead —
    // tried first — silently ignored `d[j]`, so on any column whose own
    // scale factor was far from `1` this landed nowhere near the intended
    // original-space magnitude (confirmed on Netlib `shell`: reachable
    // only through a genuinely infinite bound, not the old
    // clamped-before-presolve path, and false `Infeasible` where HiGHS
    // reaches `Optimal`).
    if clamp_unbounded {
        for j in 0..n {
            let dj = pre.scaling.d[j];
            if lb[j] == f64::NEG_INFINITY {
                lb[j] = -BIG_M / dj;
            }
            if ub[j] == f64::INFINITY {
                ub[j] = BIG_M / dj;
            }
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
    // ub[j]==+inf` — `clamp_unbounded` already finitized every such bound
    // above when true, so this only ever fires for the `solve_lp_dual`
    // extended path: `presolve::freevar`'s own documented residual case, a
    // free variable that appears only in an inequality row and so cannot
    // be soundly eliminated by that module alone) gets exactly the same
    // single compacted slot as any other surviving column — no more
    // `x_j = x_j^+ - x_j^-` split: `extended_dual::hat_lower`/`hat_upper`
    // track a genuinely free column's *both* sides symbolically (`M` on
    // each), so this module no longer needs to represent it with two
    // one-sided-bounded halves (see that module's own docs for why, and
    // its `finish`'s own docs for the one residual case — two free columns
    // coupled *only* to each other, with no third anchoring either — it
    // still can't resolve internally and falls back to the classical
    // `BIG_M` path for, same as every other "should be unreachable" guard
    // in that module).
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

    let mut rows = Vec::with_capacity(n_rows);
    let mut b_out = Vec::with_capacity(n_rows);

    for i in 0..n_eq {
        let slack = n_free + i;
        let mut rhs_i = pre.b[i];
        let mut r: Vec<(usize, f64)> = Vec::new();
        for (j, v) in csr_row_iter(&pre.a, i) {
            match new_index[j] {
                Some(nj) => {
                    r.push((nj, v * sign[nj]));
                    rhs_i -= v * sign[nj] * shift[j];
                }
                None => rhs_i -= v * lb[j],
            }
        }
        new_lb[slack] = 0.0;
        new_ub[slack] = 0.0;
        r.push((slack, 1.0));
        rows.push(r);
        b_out.push(rhs_i);
    }
    for (k, row) in g_rows.into_iter().enumerate() {
        let slack = n_free + n_eq + k;
        let mut rhs_k = g_rhs[k];
        let mut r: Vec<(usize, f64)> = Vec::new();
        for (j, v) in row {
            match new_index[j] {
                Some(nj) => {
                    r.push((nj, v * sign[nj]));
                    rhs_k -= v * sign[nj] * shift[j];
                }
                None => rhs_k -= v * lb[j],
            }
        }
        new_lb[slack] = 0.0;
        new_ub[slack] = f64::INFINITY;
        r.push((slack, 1.0));
        rows.push(r);
        b_out.push(rhs_k);
    }

    let (rows, cols) = freeze_std_matrices(&rows, n_total);
    let shift_of_free: Vec<f64> = orig_of_free.iter().map(|&j| shift[j]).collect();
    Ok(PresolvedForm {
        std: StdForm { n_total, n_rows, c, rows, cols, b: b_out, lb: new_lb, ub: new_ub },
        scaling: pre.scaling,
        postsolve_log: pre.postsolve_log,
        orig_of_free,
        sign,
        fixed_values,
        shift: shift_of_free,
        had_unbounded_structural,
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
    /// (`run_phase`, `solve_lp_dual_on`) now go through the
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
    /// (length `n_rows`) instead of allocating — `solve_lp_dual_on` calls
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
    /// own loop (`solve_lp_dual_on`) can get this ground-truth `rhs` for its
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
        use std::sync::atomic::Ordering::Relaxed;
        let mut rhs = self.std.b.clone();
        for j in 0..self.std.n_total {
            if self.nb_status[j].is_none() {
                continue;
            }
            prof_phases::COMPUTE_RHS_COLS_TOTAL.fetch_add(1, Relaxed);
            let xj = self.x[j];
            if xj == 0.0 {
                prof_phases::COMPUTE_RHS_COLS_SKIPPED.fetch_add(1, Relaxed);
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
    /// `cost` is a parameter (rather than reading `self.std.c` directly)
    /// so the caller can crash against a *perturbed* cost vector — see
    /// `perturb_costs` — while `self.std.c` stays available separately for
    /// the true objective.
    fn crash_dual_feasible(&mut self, cost: &[f64]) {
        for j in 0..self.n_orig() {
            let c = cost[j];
            let lo = self.std.lb[j];
            let hi = self.std.ub[j];
            let status = if c >= -TOL { NbStatus::Lower } else { NbStatus::Upper };
            self.nb_status[j] = Some(status);
            self.x[j] = match status {
                NbStatus::Lower => lo,
                NbStatus::Upper => hi,
                NbStatus::Zero => 0.0,
            };
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

/// `refactorize` for the *initial* all-slack basis specifically: `B` is a
/// signed identity there (`Tableau::new` seats row `i`'s own slack at basis
/// position `i`, coefficient `+/-1`), so its LU factorization is
/// definitionally `L = I`, `U = B`, no permutation — no pivot search
/// required. Falls back to the general `try_refactorize` if the basis
/// somehow isn't exactly diagonal (it always is, right after
/// `Tableau::new`, before `crash_dual_feasible` or any pivot has touched
/// `basis`/`basis_pos`), so this can never mis-factorize even if that
/// invariant is ever violated.
fn try_initial_refactorize(std: &StdForm, t: &Tableau) -> Option<sparse_lu::FtLu> {
    // Start-of-solve point for this module's own solves (the basis here is
    // still the untouched all-slack one), and so where the LU pivot
    // threshold's per-solve escalation ladder is rewound — see
    // `sparse_lu::pivot_threshold`'s own docs for why a thread must not
    // inherit the previous solve's escalated floor.
    sparse_lu::reset_pivot_threshold();
    let rows = t.basis_rows_sparse();
    sparse_lu::factorize_diagonal(std.n_rows, &rows)
        .or_else(|| sparse_lu::factorize(std.n_rows, &rows))
        .map(sparse_lu::FtLu::new)
}

fn initial_refactorize(std: &StdForm, t: &Tableau) -> sparse_lu::FtLu {
    try_initial_refactorize(std, t).expect("initial all-slack basis must be nonsingular")
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
/// context: [`solve_lp_on`]'s phase 1/2 calls have nowhere further to
/// fall back to, so they still fold `None` into the same best-effort
/// `Status::Optimal` the iteration cap already accepts; [`solve_lp_dual_on`]'s
/// cost-perturbation cleanup call, which is the one that can hand this
/// function an arbitrary (not fresh all-slack) basis, instead restarts
/// from scratch via [`solve_lp_on`] — the same safe fallback this
/// function's own sibling call sites inside `solve_lp_dual_on` already use
/// for exactly this "current trajectory is numerically spent" situation.
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
    // inside the loop — mirrors `solve_lp_dual_on`'s own pre-loop buffer
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

    let ratio_pivot_tol = if std::env::var("ENOMOTO_PRIMAL_RATIO_PIVOT_TOL_OLD").is_ok() { TOL } else { FT_MIN_PIVOT };
    let max_iters = max_iters_for(m, std.n_total);
    for iter_idx in 0..max_iters {
        let rhs = t.recompute_basics(lu);

        // Triggers (1) and (3): periodic residual / eta-file-fill checks.
        *since_check += 1;
        if *since_check >= FT_CHECK_INTERVAL {
            *since_check = 0;
            let bump_too_big = lu.fill_count() > FT_BUMP_LIMIT_FACTOR * m.max(1);
            let residual_too_big = !bump_too_big && t.basis_residual_norm(&rhs) > FT_RESIDUAL_TOL;
            if bump_too_big || residual_too_big {
                // `None` here, not a panic — see this function's own docs
                // for what that signals to the caller.
                let Some(l) = try_refactorize(std, t, Some(&*lu)) else {
                    if std::env::var("ENOMOTO_DEBUG_PHASES").is_ok() {
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
                if std::env::var("ENOMOTO_DEBUG_PHASES").is_ok() {
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
        // sequentially, not via rayon — see `solve_lp_dual_on`'s module
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
                let score = dj * dj / se.gamma[j].max(STEEPEST_EDGE_FLOOR);
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
        } else if std.n_total >= PARTIAL_PRICING_THRESHOLD {
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
        let admitted = candidates.iter().filter(|c| c.exact <= alpha1 + PRIMAL_HARRIS_TOL);
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
                if !lu.try_update(r, a_enter, FT_MIN_PIVOT) {
                    let Some(l) = try_refactorize(std, t, Some(&*lu)) else {
                        if std::env::var("ENOMOTO_DEBUG_PHASES").is_ok() {
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

/// Above this many variables, a connected component found by
/// [`connected_components_of_std_form`] is solved via a separate `rayon`
/// task rather than in the main sequential loop, once *any* component in
/// the batch clears this size — solving an entire LP (its own presolved
/// simplex loop, potentially thousands of pivots) is substantial work,
/// unlike this file's other, deliberately sequential per-*iteration*
/// loops (`chuzr`, `chuzc1`, DSE weight updates — see
/// `solve_lp_dual_on`'s own module docs for the profiling that found
/// `rayon`'s per-call dispatch overhead exceeding *those* loop bodies at
/// this crate's realistic problem sizes); at this much coarser
/// "solve a whole sub-problem" granularity, that same dispatch cost is
/// comfortably negligible in comparison. `200` is the size the user
/// requesting this feature asked for directly, not independently tuned —
/// see [`solve_std_form_decomposed`]'s own docs for why no Netlib
/// instance in this crate's own benchmark set actually exercises the
/// parallel path at all (every genuine split found there lands well
/// under this threshold).
const PARALLEL_COMPONENT_MIN_VARS: usize = 200;

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

/// Solves an already-presolved `StdForm`, transparently splitting into
/// independent connected components first when
/// [`connected_components_of_std_form`] finds more than one — each solved
/// via the primal method ([`solve_lp_on`]) if `use_dual` is false, or the
/// dual method ([`solve_lp_dual_on`]) if true and the component has any
/// rows at all (mirrors [`solve_lp_dual`]'s own top-level `n_rows == 0`
/// fallback to the primal method, now applied per component instead of
/// only to `std` as a whole) — then recombined into one [`SimplexResult`]
/// indexed by the *original* variable numbering. Falls straight through
/// to a single, undecomposed solve when no split is found, so this adds
/// only [`connected_components_of_std_form`]'s own `O(nnz)` union-find
/// scan to the cost of a solve that turns out not to be separable.
///
/// **What this crate's own 73-problem Netlib benchmark set actually looks
/// like, post-presolve, once this was measured rather than assumed**: an
/// *earlier* feasibility check, run directly against the 88 cached Netlib
/// MPS files' *original*, pre-presolve constraint graphs, found
/// essentially no exploitable structure (19 files with more than one
/// component, but every one just a single dominant component plus
/// trivial size-1 fragments — the largest real second component anywhere
/// was `standgub`'s 108). That check did *not* look at the graph *this*
/// function actually sees — the fully presolved `std` — and that turned
/// out to look completely different: post-presolve, most instances split
/// into **hundreds** of components (`fit1p` 628, `sctap3` 624, `ganges`
/// 530, `modszk1` 422, …), but the shape is the same as before at a finer
/// grain — one large remaining component (`fit1p`'s is 1050 of its own
/// 1677 variables) plus a great many singletons (variables presolve left
/// with no remaining *real* row to couple them to anything else). No
/// instance in this set ever has two or more components clearing the
/// `real_components` bar below, let alone [`PARALLEL_COMPONENT_MIN_VARS`]
/// — so on this crate's own benchmark set, this whole mechanism is
/// permanently dormant by design (see below), active only for whichever
/// future problem actually has genuine block-diagonal structure.
///
/// Two real regressions were measured and fixed while developing this,
/// both instructive about what "dormant by design" has to mean in
/// practice given how often real Netlib instances *do* find some kind of
/// split (just never a useful one): (1) an initial version called
/// [`split_std_form`]'s predecessor once per component, each call
/// rescanning every row of `std` — quadratic-ish against instances with
/// hundreds of components, and a real, measured aggregate slowdown;
/// fixed by making it a single linear pass. (2) even after that fix, an
/// initial version *always* built every component's own `StdForm` before
/// checking whether any of it was worth using, so a "hundreds of
/// singletons plus one remainder" instance (the common case here) still
/// paid hundreds of small allocations for a split that was about to be
/// thrown away — fixed by the cheap `has_row`-based `real_components`
/// check below running *before* [`split_std_form`] is ever called.
/// Neither fix changed *whether* a split happens, only its cost when it
/// doesn't help — the real remaining risk, confirmed directly on Netlib
/// `25fv47`, is that splitting off even a single genuinely-free variable
/// still *compacts* every surviving variable's own global index (see
/// [`split_std_form`]'s own docs), and on a highly degenerate instance
/// that alone was enough to send chuzc/DSE tie-breaking down a completely
/// different — still correct, but far longer — pivot sequence (3,092 to
/// 11,468 iterations, a 3-4x wall-clock regression for the identical
/// answer). That is why splitting requires *two or more* real components
/// before it is attempted at all: it is the only condition under which
/// this mechanism has anything to gain, and gating on it happens to also
/// be exactly what keeps this crate's own benchmark set from ever paying
/// that tie-break risk for zero benefit.
fn solve_std_form_decomposed(std: &StdForm, use_dual: bool) -> SimplexResult {
    if std.n_rows == 0 {
        return solve_lp_on(std);
    }
    let Some((components, has_row)) = connected_components_of_std_form(std) else {
        return if use_dual { solve_lp_dual_on(std, false) } else { solve_lp_on(std) };
    };
    let n_orig = std.n_total - std.n_rows;

    // Only actually dispatch the split when at least two components have
    // a real (row-bearing) sub-problem to solve — checked here via the
    // *free* `has_row` flags (a component is "real" unless it is a
    // size-1 variable with no row at all, typically one `dualfix` already
    // fixed), deliberately *before* ever calling [`split_std_form`], not
    // by building every component's `StdForm` and discarding the ones
    // that turn out not to matter. Splitting off nothing but trivial
    // singletons around one large remainder buys zero benefit — those
    // variables were already effectively free, static, never-revisited
    // nonbasic values in the *undecomposed* solve too — while
    // `split_std_form` itself is not free: real Netlib instances
    // routinely produce hundreds of components post-presolve (per this
    // function's own docs), and building a throwaway `StdForm` (its own
    // small `Vec`/`CsrMat`/`CscMat` allocations) for every one of them, only
    // discard almost all of them, is a real, measured cost on top of the
    // *other* real cost splitting risks: on a highly degenerate instance,
    // changing *which* global index a variable ends up with (every
    // surviving component's variables are compacted to a fresh
    // `0..local_n` range) can change which candidate wins an exact tie in
    // chuzc's Dantzig-style pricing or dual steepest-edge, sending the
    // solve down a completely different — still correct, but potentially
    // far longer — pivot sequence than the original numbering would have
    // taken. Confirmed directly on Netlib `25fv47`: with only one real
    // component and the rest trivial singletons, splitting anyway (an
    // earlier version of this function did) took iteration count from
    // 3,092 to 11,468 (`ENOMOTO_PROF_PHASES`) for the identical correct
    // answer, a 3-4x wall-clock regression on a single, very ordinary
    // Netlib instance — the same class of floating-point-path sensitivity
    // this session's own `chuzr` tie-break fix, the Schork-Gondzio
    // Forrest-Tomlin variant, and the fixed-width `chuzc1` exclusion all
    // independently ran into on this same family of degenerate instances.
    // Requiring *two* real components before paying either cost at all
    // means both are only ever paid when there is an actual decomposition
    // to gain from, never merely to shave off already-free variables.
    let real_components = components.iter().filter(|c| c.len() > 1 || has_row[c[0]]).count();
    if real_components <= 1 {
        return if use_dual { solve_lp_dual_on(std, false) } else { solve_lp_on(std) };
    }

    let mut x = vec![0.0; n_orig];
    let sub_std_forms = split_std_form(std, &components);

    let solve_component = |(sub, component): (&StdForm, &Vec<usize>)| -> SimplexResult {
        if sub.n_rows == 0 {
            debug_assert_eq!(component.len(), 1);
            let j = 0;
            let val = if sub.c[j] > TOL { sub.lb[j] } else if sub.c[j] < -TOL { sub.ub[j] } else { sub.lb[j] };
            return SimplexResult { status: Status::Optimal, x: Some(vec![val]) };
        }
        if use_dual { solve_lp_dual_on(sub, false) } else { solve_lp_on(sub) }
    };

    let use_parallel = components.iter().any(|c| c.len() >= PARALLEL_COMPONENT_MIN_VARS);
    let pairs: Vec<(&StdForm, &Vec<usize>)> = sub_std_forms.iter().zip(components.iter()).collect();
    let results: Vec<SimplexResult> = if use_parallel {
        use rayon::prelude::*;
        pairs.par_iter().map(|&p| solve_component(p)).collect()
    } else {
        pairs.iter().map(|&p| solve_component(p)).collect()
    };

    for (result, component) in results.iter().zip(components.iter()) {
        match result.status {
            Status::Infeasible => return SimplexResult { status: Status::Infeasible, x: None },
            Status::Unbounded => return SimplexResult { status: Status::Unbounded, x: None },
            Status::InfeasibleOrUnbounded => return SimplexResult { status: Status::InfeasibleOrUnbounded, x: None },
            Status::Optimal => {
                let sub_x = result.x.as_ref().expect("Optimal result must carry x");
                for (local_j, &orig_j) in component.iter().enumerate() {
                    x[orig_j] = sub_x[local_j];
                }
            }
        }
    }
    SimplexResult { status: Status::Optimal, x: Some(x) }
}

/// The primal two-phase method's actual work, operating on an
/// already-presolved `StdForm`. No longer reachable through a standalone
/// primal entry point (`solve_lp`, the classical `BIG_M`-substituted
/// primal method, was removed once every production call path settled on
/// `solve_lp_dual` — see that function's own docs) — kept only because
/// [`solve_lp_dual`]'s own `n_rows == 0` trivial case still reuses it
/// directly, without running presolve twice.
fn solve_lp_on(std: &StdForm) -> SimplexResult {
    if std.n_rows == 0 {
        // No constraints at all: every variable's bound is finite, so the
        // optimum is trivially at whichever bound the sign of c favors
        // (either bound when c is ~0) — no unbounded case is possible.
        let mut x = vec![0.0; std.n_total];
        for j in 0..std.n_total {
            x[j] = if std.c[j] > TOL { std.lb[j] } else if std.c[j] < -TOL { std.ub[j] } else { std.lb[j] };
        }
        return SimplexResult { status: Status::Optimal, x: Some(x) };
    }

    let mut t = Tableau::new(std);
    // Initial basis is all slacks (B = a signed identity): built directly
    // via `factorize_diagonal` rather than run through Markowitz pivoting.
    let mut lu = initial_refactorize(std, &t);
    let mut since_check = 0usize;
    let mut expand = ExpandState::new();
    let mut se = SteepestEdgeState::new(std);
    let mut stall = PrimalStallState::new();

    // `unwrap_or(Status::Optimal)`: this is the top-level primal entry
    // point, with nowhere further to fall back to on `run_phase`'s `None`
    // (unrecoverable mid-solve singular basis) — the same best-effort
    // tolerance its own iteration-cap fallback already accepts (see
    // `run_phase`'s own docs).
    let phase1_status = run_phase(std, &mut t, true, &mut lu, &mut since_check, &mut expand, &mut se, &mut stall).unwrap_or(Status::Optimal);
    if phase1_status == Status::Infeasible {
        return SimplexResult { status: Status::Infeasible, x: None };
    }

    let phase2_status = run_phase(std, &mut t, false, &mut lu, &mut since_check, &mut expand, &mut se, &mut stall).unwrap_or(Status::Optimal);
    match phase2_status {
        Status::Unbounded => SimplexResult { status: Status::Unbounded, x: None },
        Status::Infeasible => SimplexResult { status: Status::Infeasible, x: None }, // shouldn't happen after phase 1
        Status::InfeasibleOrUnbounded => unreachable!("the primal tableau method always classifies"),
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

    /// Exact DSE weights for an **arbitrary** (already-factored) basis,
    /// rather than [`Self::new`]'s all-slack-only `w[i] = 1`. Row `i` of
    /// `B^-1` is `e_i^T B^-1`, obtained by one BTRAN (`B^T z = e_i`, i.e.
    /// `lu.solve_transpose_into(&e_i, ..)`); its squared 2-norm is exactly
    /// `w[i] = ||e_i^T B^-1||^2`. One BTRAN per row (`O(m)` BTRANs) - the
    /// same *kind* of `O(m * nnz)` work [`fresh_d`] already does in one
    /// call after every refactorization, and far cheaper than the full
    /// cold restart it replaces on `forplan` (221 DSE + 245 Devex pivots).
    /// Must be given the `lu` matching the basis the weights are wanted
    /// for; the callers pass the factorization live at the point of the
    /// Devex->DSE switch (see the two call sites below), so the weights
    /// are exact for exactly that basis - no drift from an FT update the
    /// switch has not yet applied.
    fn from_basis(m: usize, lu: &sparse_lu::FtLu) -> Self {
        let mut w = vec![1.0; m];
        let mut scratch = vec![0.0; m];
        let mut z = vec![0.0; m];
        // `FtLu::solve_transpose_unit_into` is only exact on a
        // freshly-factorized `u_seq` (`update_count() == 0` — see its own
        // docs) — most `from_basis` callers are exactly that (every
        // refactor-time DSE refresh), but the Devex->DSE in-place switch
        // (`simplex.rs`'s own two call sites) can land here mid-solve with
        // updates already applied, so this falls back to the unmodified
        // dense `solve_transpose_into` sweep in that case rather than
        // ever risking silent wrong weights.
        if lu.update_count() == 0 {
            for i in 0..m {
                lu.solve_transpose_unit_into(i, &mut scratch, &mut z);
                let norm_sq: f64 = z.iter().map(|&v| v * v).sum();
                w[i] = norm_sq.max(STEEPEST_EDGE_FLOOR);
            }
        } else {
            for i in 0..m {
                lu.solve_transpose_unit(i, &mut scratch, &mut z);
                let norm_sq: f64 = z.iter().map(|&v| v * v).sum();
                w[i] = norm_sq.max(STEEPEST_EDGE_FLOOR);
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
        let wp_old = rho_p.iter().map(|v| v * v).sum::<f64>().max(STEEPEST_EDGE_FLOOR);
        let update_one = |i: usize, w_i: &mut f64| {
            if i == p {
                return;
            }
            let ratio = alpha[i] / pivot;
            *w_i = (*w_i - 2.0 * ratio * tau[i] + ratio * ratio * wp_old).max(STEEPEST_EDGE_FLOOR);
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
                *w_i = (*w_i - 2.0 * ratio * t_i + ratio * ratio * wp_old).max(STEEPEST_EDGE_FLOOR);
            }
        }
        self.w[p] = (wp_old / (pivot * pivot)).max(STEEPEST_EDGE_FLOOR);
    }
}

/// Dual Devex weights: a cheap, approximate substitute for [`DseState`]'s
/// exact row weights (Harris's 1973 Devex idea, adapted to the dual
/// method's row-indexed weights the same way HiGHS's
/// `updateDualDevexWeights` does — see `docs/highs_dual_simplex.tex`
/// §"Devex と DSE の切り替え"). The exact DSE update needs `tau = B^-1
/// (B^-T e_p)`, an entire extra FTRAN solve every pivot purely to maintain
/// weight accuracy (see [`DseState::update_after_pivot`]'s own docs); Devex
/// drops that solve entirely and instead maintains a monotone *overestimate*
/// from `alpha` alone (already computed for the pivot regardless of pricing
/// rule), at the cost of the weights only loosely tracking the true edge
/// norms. [`solve_lp_dual_on`] starts every solve in Devex mode for exactly
/// that saved FTRAN, and escalates one-way to exact [`DseState`] if
/// [`DEVEX_STAGNATION_WINDOW`]'s rolling check ever suggests the
/// approximation itself is steering `chuzr` badly.
struct DevexState {
    w: Vec<f64>,
}

impl DevexState {
    /// Reference framework starts at the all-slack basis with every row's
    /// weight at `1.0` — same starting point as `DseState::new`, for the
    /// same reason (`B0` a signed identity).
    ///
    /// **Always seed a fresh `DevexState` at `1.0`, never from another
    /// scheme's already-known weights (e.g. an existing `DseState`'s exact
    /// values), even though those look like a strictly more accurate
    /// starting point.** Confirmed the hard way on `extended_dual.rs`'s own
    /// delta=0 Dse->Devex downgrade attempt: seeding from the live
    /// `DseState.w` at that point produced a real, reproducible false
    /// `Infeasible` on Netlib `pilot4` (switching back to a fresh `1.0`
    /// start fixed it outright, isolated by testing both side by side).
    /// Root cause: [`Self::update_after_pivot`]'s pivot-row line,
    /// `self.w[p] = (wp_old / (pivot*pivot)).max(1.0)`, hard-floors at
    /// `1.0` — correct *only* under this constructor's own convention that
    /// every weight starts at exactly that scale. Exact DSE weights carry
    /// no such property (`DseState`'s own floor is
    /// [`STEEPEST_EDGE_FLOOR`], orders of magnitude below `1.0`, and true
    /// values can be smaller still before `try_update`... before any
    /// flooring); handing raw DSE values to this recurrence's `1.0` floor
    /// artificially inflates whichever row pivots first, and Devex's own
    /// weights only ever grow from there (never shrink or self-correct the
    /// way DSE's exact recurrence does), so that one distortion compounds
    /// pivot after pivot into a scoring order bad enough to reach a
    /// genuine "no eligible column" conclusion that was never actually
    /// true.
    fn new(m: usize) -> Self {
        DevexState { w: vec![1.0; m] }
    }

    /// `p` = the pivot row, `alpha` = `B^-1 a_q` (the entering column's
    /// FTRAN) — the same two quantities [`DseState::update_after_pivot`]
    /// takes, minus `tau`. Textbook Devex update (see this struct's own
    /// docs for the reference): every other row's weight only ever grows,
    /// to `(alpha_i / alpha_p)^2` times the *old* pivot-row weight if that
    /// exceeds what it already had — an upper bound on the true steepest-edge
    /// update's exact (and possibly weight-shrinking) cross term, not an
    /// attempt to track it precisely. The pivot row itself restarts the
    /// reference framework at its own transformed weight, floored at `1.0`
    /// (a fresh framework member is never allowed to start below the
    /// reference weight every row began this framework at).
    fn update_after_pivot(&mut self, p: usize, alpha: &[f64]) {
        let pivot = alpha[p];
        let wp_old = self.w[p];
        for (i, w_i) in self.w.iter_mut().enumerate() {
            if i == p {
                continue;
            }
            let ratio = alpha[i] / pivot;
            let candidate = ratio * ratio * wp_old;
            if candidate > *w_i {
                *w_i = candidate;
            }
        }
        self.w[p] = (wp_old / (pivot * pivot)).max(1.0);
    }
}

/// Which row-weight scheme is currently pricing the dual method's `chuzr`
/// in [`solve_lp_dual_on`] — see [`DevexState`]'s own docs for why every
/// solve starts in `Devex` mode and what triggers the one-way escalation
/// to `Dse`. Kept as a plain enum (matched at each of the handful of call
/// sites) rather than a trait object: both variants live on the hot pivot
/// loop's path, so a `dyn` vtable call per pivot is worth avoiding for the
/// same reason this file's other per-iteration loops stay monomorphic.
enum EdgeWeights {
    Devex(DevexState),
    Dse(DseState),
}

impl EdgeWeights {
    /// The current weight for basic row `i`, whichever scheme is active —
    /// `chuzr_scan`'s only need from this type.
    #[inline]
    fn weight(&self, i: usize) -> f64 {
        match self {
            EdgeWeights::Devex(s) => s.w[i],
            EdgeWeights::Dse(s) => s.w[i],
        }
    }
}

/// Whether basic row `i` is primal-infeasible under `chuzr`'s own rule —
/// `true` only if `chuzr_scan` (in [`solve_lp_dual_on`]) would return
/// `Some` for this row. Factored out as a free function (rather than a
/// closure, which would otherwise need to re-borrow `t`/`noise_feasible`
/// at every one of [`InfeasibleRows`]'s several call sites scattered
/// through the pivot loop) so [`InfeasibleRows`]'s incremental maintenance
/// and its rebuild-from-scratch path share exactly one definition of
/// "infeasible" — any future change to the tolerance/`noise_feasible`
/// logic only needs to happen here to stay consistent everywhere.
///
/// Simplified from `chuzr_scan`'s own three-way `delta` computation: since
/// both the branch condition and the final `delta <= PRIMAL_FEAS_TOL`
/// check use the same `PRIMAL_FEAS_TOL`, taking the `val < lb - TOL`
/// branch already guarantees `delta = lb - val > TOL` (and symmetrically
/// for the upper-bound branch), so the extra check is redundant once
/// `noise_feasible` has been accounted for.
#[inline]
fn row_infeasible(std: &StdForm, t: &Tableau, noise_feasible: &[bool], i: usize) -> bool {
    let var = t.basis[i];
    if noise_feasible[var] {
        return false;
    }
    let val = t.x[var];
    val < std.lb[var] - PRIMAL_FEAS_TOL || val > std.ub[var] + PRIMAL_FEAS_TOL
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
/// The ratio test (`chuzc`) enhances the classical/textbook one (eligible
/// candidates by the leaving row's sign pattern, smallest `|d_j /
/// alpha_pj|` wins) with the bound-flipping ratio test (BFRT, Huangfu &
/// Hall §2.2.2): candidates are sorted by ascending ratio and every
/// finitely-bounded one ahead of the eventual entering variable is fully
/// flipped to its opposite bound in the same iteration, rather than each
/// needing its own full pivot — see the BFRT walk in the loop body below
/// for the derivation of why this stays dual feasible. Pass 1's own
/// boundary (`stop_idx`) is then refined by a flat-window Harris-style
/// pass 2 — see [`HARRIS_RATIO_TOL`]'s own docs, including why both a
/// per-row pivot-scaled Harris window and a direct port of HiGHS's own
/// `chooseFinalLargeAlpha` were tried in its place and reverted.
/// Per-phase wall-clock counters for the `ENOMOTO_PROF_PHASES` diagnostic
/// (see [`solve_lp_dual`]) — answers "where does time in the dual simplex
/// loop actually go" now that `ENOMOTO_PROF_TRIANGULAR` has already ruled
/// out LU pivot *search* (`sparse_lu::PROF_BUCKET_SCAN_NS`) as the answer.
/// Each covers one named phase of a single iteration of
/// [`solve_lp_dual_on`]'s loop; `phases_profile_enabled` gates the
/// `Instant::now()` calls themselves (not just the final printout) behind
/// one `env::var` check hoisted above the loop, so a normal (non-profiling)
/// solve pays a single `bool` branch per phase per iteration, not an actual
/// timer read.
mod prof_phases {
    use std::sync::atomic::AtomicUsize;
    pub(super) static BTRAN: AtomicUsize = AtomicUsize::new(0);
    pub(super) static PRICE: AtomicUsize = AtomicUsize::new(0);
    pub(super) static CHUZR: AtomicUsize = AtomicUsize::new(0);
    pub(super) static CHUZC1: AtomicUsize = AtomicUsize::new(0);
    pub(super) static BFRT: AtomicUsize = AtomicUsize::new(0);
    pub(super) static FTRAN: AtomicUsize = AtomicUsize::new(0);
    pub(super) static DSE_UPDATE: AtomicUsize = AtomicUsize::new(0);
    pub(super) static DUAL_UPDATE: AtomicUsize = AtomicUsize::new(0);
    pub(super) static FT_UPDATE: AtomicUsize = AtomicUsize::new(0);
    pub(super) static REFACTOR: AtomicUsize = AtomicUsize::new(0);
    /// Number of times a full `factorize()` refactorization actually ran
    /// (not just `try_update`'s cheap per-pivot eta append) — separate
    /// from `REFACTOR`'s cumulative *time* so the diagnostic can show both
    /// how often and how expensive each refactorization was.
    pub(super) static REFACTOR_COUNT: AtomicUsize = AtomicUsize::new(0);
    /// How many of this solve's entering-column/BFRT-combined-flip FTRANs
    /// took the dense bypass (`FtLu::should_use_dense_solve`) instead of
    /// the Gilbert-Peierls sparse path — i.e. how often the
    /// dense-coefficient bypass is actually live, as opposed to a no-op on
    /// every real Netlib instance (measured: zero, on all 73 in-scope
    /// problems).
    pub(super) static DENSE_RHS_BYPASSES: AtomicUsize = AtomicUsize::new(0);
    /// `compute_rhs`'s own nonbasic-column visits: `TOTAL` counts every
    /// nonbasic column it considers, `SKIPPED` how many of those it never
    /// touched a single nonzero of because `x[j] == 0.0` — see that
    /// function's own docs for why a shifted/naturally-zero bound makes
    /// this common rather than a rare edge case.
    pub(super) static COMPUTE_RHS_COLS_TOTAL: AtomicUsize = AtomicUsize::new(0);
    pub(super) static COMPUTE_RHS_COLS_SKIPPED: AtomicUsize = AtomicUsize::new(0);
    pub(super) static ITERS: AtomicUsize = AtomicUsize::new(0);
    /// Per-iteration *shape* of the chuzr/BFRT work, reported alongside
    /// the phase timings above when `ENOMOTO_DEBUG_CHUZR` is also set:
    /// how many basic rows are primal-infeasible (the only ones chuzr can
    /// ever pick — everything else scores exactly zero regardless of its
    /// DSE weight), how many entries of the entering column's FTRAN are
    /// nonzero (the only rows whose `x_B` — hence infeasibility — can
    /// change in that pivot), and how many BFRT candidates get sorted
    /// versus how many the walk actually consumes. Measured on Netlib to
    /// size the hyper-sparse chuzr / heap-based BFRT candidates — see
    /// the project history around this comment's own commit.
    pub(super) static INFEAS_ROWS: AtomicUsize = AtomicUsize::new(0);
    pub(super) static ALPHA_NNZ: AtomicUsize = AtomicUsize::new(0);
    pub(super) static BFRT_CANDS: AtomicUsize = AtomicUsize::new(0);
    pub(super) static BFRT_WALK: AtomicUsize = AtomicUsize::new(0);
    /// How many pivots had pass 2 of the Harris two-pass ratio test
    /// actually swap away from `stop_idx` (pass 1's own boundary) to some
    /// better-conditioned earlier candidate — i.e. how often the
    /// per-candidate, pivot-scaled Harris window (`HARRIS_RATIO_TOL /
    /// |a_pj|`) actually does anything, as opposed to `best_idx == stop_idx`
    /// every time. See the BFRT block's own comment for the eligibility
    /// condition this counts.
    pub(super) static HARRIS_SWAPS: AtomicUsize = AtomicUsize::new(0);

    /// `updateVerify` diagnostics (`ENOMOTO_PROF_UPDATE_VERIFY` — see
    /// [`UPDATE_VERIFY_TOL`]'s own docs for what this check does):
    /// `UPDATE_VERIFY_CHECKS` counts every pivot the check actually ran
    /// against (i.e. every completed FTRAN, whether or not it fired);
    /// `UPDATE_VERIFY_TRIGGERS` counts only the ones that disagreed enough
    /// to force an extra refactorization; `UPDATE_VERIFY_MAX_REL_PPM` and
    /// `UPDATE_VERIFY_SUM_REL_PPM` accumulate the relative error
    /// (`* 1_000_000`, parts-per-million, the same fixed-point trick
    /// `ETA_DENSITY_SUM_PPM` below uses to avoid an atomic `f64`) only for
    /// the pivots that actually triggered, so the mean-over-triggers is
    /// `SUM_REL_PPM / TRIGGERS` and the worst-case is `MAX_REL_PPM` alone.
    pub(super) static UPDATE_VERIFY_CHECKS: AtomicUsize = AtomicUsize::new(0);
    pub(super) static UPDATE_VERIFY_TRIGGERS: AtomicUsize = AtomicUsize::new(0);
    pub(super) static UPDATE_VERIFY_MAX_REL_PPM: AtomicUsize = AtomicUsize::new(0);
    pub(super) static UPDATE_VERIFY_SUM_REL_PPM: AtomicUsize = AtomicUsize::new(0);

    /// Histogram of `FtLu::last_update_off_diag_len()` / `(m-1)` (the
    /// fraction of possible off-diagonal slots a single `try_update`-created
    /// eta actually fills), bucketed into 20 equal-width 5% bins, plus a
    /// running sum/count for the mean — gated by `ENOMOTO_DEBUG_ETA_DENSITY`.
    /// See that flag's use in `solve_lp_dual_on` and `FtLu::last_update_off_diag_len`'s
    /// own docs for why this exists.
    pub(super) static ETA_DENSITY_BINS: std::sync::Mutex<[u64; 20]> = std::sync::Mutex::new([0; 20]);
    pub(super) static ETA_DENSITY_SAMPLES: AtomicUsize = AtomicUsize::new(0);
    pub(super) static ETA_DENSITY_SUM_PPM: AtomicUsize = AtomicUsize::new(0); // sum of fractions in parts-per-million, to avoid an atomic f64

    pub(super) fn reset() {
        use std::sync::atomic::Ordering::Relaxed;
        for c in [&BTRAN, &PRICE, &CHUZR, &CHUZC1, &BFRT, &FTRAN, &DSE_UPDATE, &DUAL_UPDATE, &FT_UPDATE, &REFACTOR, &REFACTOR_COUNT, &DENSE_RHS_BYPASSES, &COMPUTE_RHS_COLS_TOTAL, &COMPUTE_RHS_COLS_SKIPPED, &ITERS, &INFEAS_ROWS, &ALPHA_NNZ, &BFRT_CANDS, &BFRT_WALK, &HARRIS_SWAPS, &UPDATE_VERIFY_CHECKS, &UPDATE_VERIFY_TRIGGERS, &UPDATE_VERIFY_MAX_REL_PPM, &UPDATE_VERIFY_SUM_REL_PPM, &ETA_DENSITY_SAMPLES, &ETA_DENSITY_SUM_PPM] {
            c.store(0, Relaxed);
        }
        *ETA_DENSITY_BINS.lock().unwrap() = [0; 20];
    }
}

/// Times `$body` and adds the elapsed nanoseconds to `$counter` — but only
/// when `$enabled` (a `bool` read once per iteration, not an `env::var`
/// call) is true; otherwise `$body` runs with no `Instant::now()` at all.
macro_rules! timed {
    ($enabled:expr, $counter:expr, $body:expr) => {{
        if $enabled {
            let __t0 = std::time::Instant::now();
            let __r = $body;
            $counter.fetch_add(__t0.elapsed().as_nanos() as usize, std::sync::atomic::Ordering::Relaxed);
            __r
        } else {
            $body
        }
    }};
}

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
/// shortcut, the `BIG_M` fallback) is reported the same way, so the set of
/// possible answers does not depend on which path solved the problem.
pub fn solve_lp_dual_with(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow], opts: crate::types::LpOptions) -> SimplexResult {
    let result = solve_lp_dual_classified(variables, objective, constraints, opts);
    if result.status == Status::Unbounded && !opts.distinguish_infeasible_unbounded {
        return SimplexResult { status: Status::InfeasibleOrUnbounded, x: None };
    }
    result
}

fn solve_lp_dual_classified(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow], opts: crate::types::LpOptions) -> SimplexResult {
    // `clamp_unbounded: false` — this function always fully handles a
    // one-sided infinite structural bound itself, either via the
    // classical path below (when none survived, the common case) or via
    // `extended_dual::solve_lp_dual_extended` (when `had_unbounded_structural`
    // is set) — see that parameter's own docs.
    let PresolvedForm { std, scaling: sc, postsolve_log, orig_of_free, sign, fixed_values, shift, had_unbounded_structural } = match build_std_form_presolved(variables, objective, constraints, false, !opts.distinguish_infeasible_unbounded) {
        Ok(pf) => pf,
        Err(status) => return SimplexResult { status, x: None },
    };
    if std::env::var("ENOMOTO_DEBUG_PRESOLVE_SIZE").is_ok() {
        // Moved here (from inside the `!had_unbounded_structural` branch
        // below, where it used to live) so it fires for *every* solve, not
        // just the common finite-bounds case: this `std` is the one
        // `extended_dual::solve_lp_dual_extended` below actually solves too
        // (same `clamp_unbounded: false` presolve pass, no second one) when
        // `had_unbounded_structural` is true — only the rare
        // extended-solver-returned-`None` fallback a few lines down
        // re-presolves with `clamp_unbounded: true` into its own fresh
        // `std`, which this print never sees (that path is itself the
        // "should be unreachable" case its own comment describes, not the
        // one worth instrumenting here).
        //
        // `std.n_total - std.n_rows` is now the count of structural columns
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
    // Always route through `extended_dual::solve_lp_dual_extended`, not
    // just when `had_unbounded_structural` is set: that solver's "M-side"
    // bookkeeping only exists to track *actually* unbounded columns, so
    // with none present it should degenerate to (and perform like) the
    // classical path below — confirmed directly on `d6cube` after the
    // equality-row propagation added above started giving every one of
    // its previously-unbounded columns a finite bound: the classical path
    // (`solve_std_form_decomposed`, previously this branch's `false` case)
    // turned out to just be the *slower* implementation for this problem's
    // structure once that happened (0.14s base -> 0.25-0.31s through
    // classical vs 0.15-0.17s through extended with nothing left to track;
    // see analysis/greenbea_20260921_230908.md's follow-up). Re-verified
    // on the full 93-problem Netlib set: 93/93 still solved to optimality,
    // aggregate `ours` time flat-to-better (not worse) than routing
    // classical-when-possible. `ENOMOTO_DISABLE_ALWAYS_EXTENDED` reverts to
    // the old had_unbounded_structural-gated routing, for comparison —
    // same pattern as this module's own `ENOMOTO_DISABLE_AGGREGATOR`/
    // `ENOMOTO_DISABLE_PARALLELCOLS` presolve toggles.
    if had_unbounded_structural || std::env::var("ENOMOTO_DISABLE_ALWAYS_EXTENDED").is_err() {
        if std::env::var("ENOMOTO_DEBUG_EXT_COMPONENTS").is_ok() {
            match connected_components_of_std_form(&std) {
                Some((components, _has_row)) => {
                    let mut sizes: Vec<usize> = components.iter().map(|c| c.len()).collect();
                    sizes.sort_unstable_by(|a, b| b.cmp(a));
                    eprintln!(
                        "DEBUG_EXT_COMPONENTS: n_components={} sizes={:?}",
                        components.len(),
                        sizes
                    );
                }
                None => eprintln!("DEBUG_EXT_COMPONENTS: single component (no split found)"),
            }
        }
        let ext_result = extended_dual::solve_lp_dual_extended(&std, &opts);
        if std::env::var("ENOMOTO_DEBUG_EXT_ITERS").is_ok() {
            match &ext_result {
                Some(r) => eprintln!("DEBUG_EXT: solve_lp_dual_extended returned Some({:?})", r.status),
                None => eprintln!("DEBUG_EXT: solve_lp_dual_extended returned None (falling back)"),
            }
        }
        if let Some(result) = ext_result {
            return unscale_result(result, &sc, &postsolve_log, &orig_of_free, &sign, &fixed_values, &shift, variables.len());
        }
        // `None`: the extended solver hit one of its own documented
        // "should be unreachable" cases (a pivot selection with no
        // numerical-stability safeguard walked the basis singular, or
        // exhausted its iteration budget — see `extended_dual`'s own
        // module docs' simplification list) rather than reaching a
        // genuine mathematical answer. Falling back to the classical
        // `BIG_M`-substituted path (this crate's pre-existing behavior for
        // every such problem, still exactly as numerically fragile as its
        // own docs describe, but strictly more tested than reporting a
        // wrong answer here) re-presolves from scratch with
        // `clamp_unbounded: true` — cheap relative to how rarely this
        // path is ever taken. Uses the classical *dual* method
        // (`use_dual: true`), not the primal one: tried once as a
        // "structurally different second opinion" (reasoning: `extended_dual`
        // already failed on this `std`'s own dual pivot sequence, so a
        // different algorithm might avoid the same failure mode), but
        // reverted after it silently returned a *wrong* finite objective on
        // a real Netlib instance (`fit1p`: 10236.95 vs the true 9146.38) —
        // `solve_lp_on`'s own primal Tableau path is not independently
        // verified against a `BIG_M`-truncated, genuinely-large-bound
        // problem shape the way this dual path is (every deleted
        // `solve_lp`-vs-`solve_lp_dual` cross-check test exercised the
        // *dual* side of this exact scenario, never the primal one, once
        // `solve_lp` itself was removed as dead code).
        let PresolvedForm { std, scaling: sc, postsolve_log, orig_of_free, sign, fixed_values, shift, .. } = match build_std_form_presolved(variables, objective, constraints, true, !opts.distinguish_infeasible_unbounded) {
            Ok(pf) => pf,
            Err(status) => return SimplexResult { status, x: None },
        };
        return unscale_result(solve_std_form_decomposed(&std, true), &sc, &postsolve_log, &orig_of_free, &sign, &fixed_values, &shift, variables.len());
    }
    let profile_phases = std::env::var("ENOMOTO_PROF_PHASES").is_ok();
    let debug_eta_density = std::env::var("ENOMOTO_DEBUG_ETA_DENSITY").is_ok();
    let debug_update_verify = std::env::var("ENOMOTO_PROF_UPDATE_VERIFY").is_ok();
    if profile_phases || debug_eta_density || debug_update_verify {
        prof_phases::reset();
    }
    let wall_t0 = std::time::Instant::now();
    let result = solve_std_form_decomposed(&std, true);
    let wall_ns = wall_t0.elapsed().as_nanos() as usize;
    if std::env::var("ENOMOTO_PROF_TRIANGULAR").is_ok() {
        use std::sync::atomic::Ordering::Relaxed;
        let total = sparse_lu::PROF_TOTAL_STEPS.load(Relaxed);
        let trivial = sparse_lu::PROF_TRIVIAL_STEPS.load(Relaxed);
        let ns = sparse_lu::PROF_BUCKET_SCAN_NS.load(Relaxed);
        let dense_fallback = sparse_lu::PROF_DENSE_FALLBACK_STEPS.load(Relaxed);
        let search_limit_steps = sparse_lu::PROF_SEARCH_LIMIT_STEPS.load(Relaxed);
        let candidates = sparse_lu::PROF_SEARCH_CANDIDATES.load(Relaxed);
        eprintln!(
            "PROF_TRIANGULAR total_steps={total} trivial_steps={trivial} ({:.1}%) total_scan_time={:.3}ms dense_fallback_steps={dense_fallback} \
             search_limit_steps={search_limit_steps} ({:.1}%) candidates={candidates} (avg {:.2}/step)",
            100.0 * trivial as f64 / total.max(1) as f64,
            ns as f64 / 1e6,
            100.0 * search_limit_steps as f64 / total.max(1) as f64,
            candidates as f64 / total.max(1) as f64
        );
    }
    if profile_phases {
        use std::sync::atomic::Ordering::Relaxed;
        let iters = prof_phases::ITERS.load(Relaxed).max(1);
        let phases: [(&str, usize); 10] = [
            ("btran(rho_p)", prof_phases::BTRAN.load(Relaxed)),
            ("price", prof_phases::PRICE.load(Relaxed)),
            ("chuzr", prof_phases::CHUZR.load(Relaxed)),
            ("chuzc1", prof_phases::CHUZC1.load(Relaxed)),
            ("bfrt", prof_phases::BFRT.load(Relaxed)),
            ("ftran", prof_phases::FTRAN.load(Relaxed)),
            ("dse_update", prof_phases::DSE_UPDATE.load(Relaxed)),
            ("dual_update", prof_phases::DUAL_UPDATE.load(Relaxed)),
            ("ft_update", prof_phases::FT_UPDATE.load(Relaxed)),
            ("refactor", prof_phases::REFACTOR.load(Relaxed)),
        ];
        let accounted: usize = phases.iter().map(|&(_, ns)| ns).sum();
        eprintln!(
            "PROF_PHASES wall={:.3}ms iters={iters} ({:.1}us/iter) accounted={:.1}% of wall",
            wall_ns as f64 / 1e6,
            wall_ns as f64 / 1e3 / iters as f64,
            100.0 * accounted as f64 / wall_ns.max(1) as f64
        );
        for (name, ns) in phases {
            eprintln!(
                "  {name:20} {:8.3}ms  {:5.1}% of wall  {:.3}us/iter",
                ns as f64 / 1e6,
                100.0 * ns as f64 / wall_ns.max(1) as f64,
                ns as f64 / 1e3 / iters as f64
            );
        }
        let refactor_count = prof_phases::REFACTOR_COUNT.load(Relaxed);
        eprintln!(
            "  refactor_count={refactor_count} avg_refactor={:.3}ms dense_rhs_bypasses={}",
            prof_phases::REFACTOR.load(Relaxed) as f64 / 1e6 / refactor_count.max(1) as f64,
            prof_phases::DENSE_RHS_BYPASSES.load(Relaxed)
        );
        let compute_rhs_total = prof_phases::COMPUTE_RHS_COLS_TOTAL.load(Relaxed);
        let compute_rhs_skipped = prof_phases::COMPUTE_RHS_COLS_SKIPPED.load(Relaxed);
        eprintln!(
            "  compute_rhs_cols total={compute_rhs_total} skipped={compute_rhs_skipped} ({:.1}%)",
            100.0 * compute_rhs_skipped as f64 / compute_rhs_total.max(1) as f64
        );
        if std::env::var("ENOMOTO_DEBUG_CHUZR").is_ok() {
            let m = std.n_rows.max(1);
            eprintln!(
                "  DEBUG_CHUZR m={m} avg_infeasible_rows/iter={:.1} ({:.1}% of m) avg_alpha_nnz/iter={:.1} ({:.1}% of m) avg_bfrt_cands/iter={:.1} avg_bfrt_walk/iter={:.1}",
                prof_phases::INFEAS_ROWS.load(Relaxed) as f64 / iters as f64,
                100.0 * prof_phases::INFEAS_ROWS.load(Relaxed) as f64 / iters as f64 / m as f64,
                prof_phases::ALPHA_NNZ.load(Relaxed) as f64 / iters as f64,
                100.0 * prof_phases::ALPHA_NNZ.load(Relaxed) as f64 / iters as f64 / m as f64,
                prof_phases::BFRT_CANDS.load(Relaxed) as f64 / iters as f64,
                prof_phases::BFRT_WALK.load(Relaxed) as f64 / iters as f64,
            );
            eprintln!(
                "  DEBUG_CHUZR harris_swaps={} ({:.3}% of iters)",
                prof_phases::HARRIS_SWAPS.load(Relaxed),
                100.0 * prof_phases::HARRIS_SWAPS.load(Relaxed) as f64 / iters as f64,
            );
        }
    }
    if debug_update_verify {
        use std::sync::atomic::Ordering::Relaxed;
        let checks = prof_phases::UPDATE_VERIFY_CHECKS.load(Relaxed);
        let triggers = prof_phases::UPDATE_VERIFY_TRIGGERS.load(Relaxed);
        let max_rel = prof_phases::UPDATE_VERIFY_MAX_REL_PPM.load(Relaxed) as f64 / 1e6;
        let mean_rel_on_trigger = prof_phases::UPDATE_VERIFY_SUM_REL_PPM.load(Relaxed) as f64 / 1e6 / triggers.max(1) as f64;
        eprintln!(
            "PROF_UPDATE_VERIFY checks={checks} triggers={triggers} ({:.4}% of checks) max_rel_err={max_rel:.3e} mean_rel_err_on_trigger={mean_rel_on_trigger:.3e}",
            100.0 * triggers as f64 / checks.max(1) as f64
        );
    }
    if debug_eta_density {
        use std::sync::atomic::Ordering::Relaxed;
        let samples = prof_phases::ETA_DENSITY_SAMPLES.load(Relaxed).max(1);
        let mean_pct = 100.0 * prof_phases::ETA_DENSITY_SUM_PPM.load(Relaxed) as f64 / 1_000_000.0 / samples as f64;
        let bins = *prof_phases::ETA_DENSITY_BINS.lock().unwrap();
        eprintln!(
            "DEBUG_ETA_DENSITY m={} updates={samples} mean_off_diag_fill={:.2}%",
            std.n_rows, mean_pct
        );
        for (i, &count) in bins.iter().enumerate() {
            if count == 0 {
                continue;
            }
            eprintln!(
                "  [{:3}-{:3}%) {:8} ({:5.1}%)",
                i * 5,
                (i + 1) * 5,
                count,
                100.0 * count as f64 / samples as f64
            );
        }
    }
    unscale_result(result, &sc, &postsolve_log, &orig_of_free, &sign, &fixed_values, &shift, variables.len())
}

/// EXPERIMENTAL (measurement only, never exercised by production code):
/// drives the tie-triggered branch-and-merge measurement — "when chuzr's
/// top score is tied (within a tolerance) among several rows, branch into
/// each tied candidate, run a few more iterations down each path, and keep
/// whichever branch is winning" — against [`solve_lp_dual_on`]'s real,
/// already-tuned loop, without touching that loop's own control flow or
/// its ~10 existing return sites.
///
/// A genuine live fork (clone every piece of the loop's mutable state —
/// `Tableau`, `FtLu`, `EdgeWeights`, `InfeasibleRows`, every scratch
/// buffer — at each tie, run each clone forward, keep the winner) would
/// need `Clone` wired through all of that state purely to support a
/// one-off measurement. This does the same experiment for free by
/// re-running the deterministic solve from scratch instead: given a fixed
/// map of already-decided tie choices (`In::forced`, iteration index to
/// rank within that iteration's tied group), the solve is fully
/// deterministic, so "fork at iteration `t`, follow candidate `c` for a
/// few more iterations" is just "re-solve from scratch with `forced[t] =
/// c` added, capped at iteration `t + window`" — cheap enough for the
/// small/medium Netlib instances this is run against, and it reuses
/// [`solve_lp_dual_on`]'s exact real logic rather than a separately
/// maintained approximation of it.
///
/// `IN`/`OUT` are thread-locals rather than extra parameters on
/// `solve_lp_dual_on` because every one of that function's ~10 return
/// sites (initial/residual/ft-update singular-basis fallbacks, the
/// optimal/infeasible returns, the DUAL->PRIMAL cleanup handoff, the
/// MAX_ITERS-exhausted trailing value) would otherwise need touching for a
/// mechanism only ever driven by the measurement harness in
/// `solve_lp_dual_with_tie_experiment` below, never by `solve_lp_dual`
/// itself — every real call path leaves `IN` at its default `None` and
/// pays only one thread-local read plus an empty `HashMap` per solve.
pub mod tie_experiment {
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Clone, Default)]
    pub struct In {
        /// Already-decided tie choices: iteration index -> rank (0 =
        /// current default tie-break) within that iteration's tied group.
        pub forced: HashMap<usize, usize>,
        /// Stop the solve early (reporting `Out::Capped`) once `_iter`
        /// reaches this — the "run a few more iterations down this branch"
        /// half of the experiment.
        pub iter_cap: Option<usize>,
        /// When chuzr hits a tied group at an iteration `forced` has no
        /// entry for, stop immediately and report it (`Out::NewTie`)
        /// instead of falling back to the default tie-break — the
        /// "discover the next branch point" half of the experiment.
        pub stop_at_new_tie: bool,
        /// Relative-score tolerance defining "tied" (candidates scoring
        /// within this fraction of chuzr's own best are considered part of
        /// the same tied group) — the same threshold this measurement's
        /// own earlier tie-frequency tally called "loose".
        pub tie_tol: f64,
        /// EXPERIMENTAL (measurement only): a *static* alternative
        /// secondary tie-break, applied at every tie for the whole solve
        /// (unlike `forced`, which is a one-off per-iteration override) —
        /// probing whether some simple, always-on replacement for this
        /// crate's current default ("highest row index among the tied top
        /// score") does better in aggregate, at zero runtime branching
        /// cost. `0` = prefer the *largest* raw (pre-weight) infeasibility
        /// `delta` among the tied group, `1` = smallest `delta`, `2` =
        /// smallest row index. `None` leaves the real default untouched.
        pub static_rule: Option<u8>,
    }

    #[derive(Clone, Debug)]
    pub enum Out {
        Capped { iters: usize, obj: f64 },
        NewTie { iter: usize, tied_rows: Vec<usize> },
    }

    thread_local! {
        pub(super) static IN: RefCell<Option<In>> = const { RefCell::new(None) };
        pub(super) static OUT: RefCell<Option<Out>> = const { RefCell::new(None) };
        /// `_iter` as of the start of the most recently begun iteration —
        /// stashed unconditionally (while `exp_active`) rather than only at
        /// the handful of normal return sites, so a plain completed solve
        /// (`Out` left `None`, i.e. neither `Capped` nor `NewTie` fired)
        /// still reports how many iterations it actually took, without
        /// needing every one of `solve_lp_dual_on`'s ~10 return sites to
        /// know about this side channel individually.
        pub(super) static LAST_ITER: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

    pub fn set_input(input: In) {
        IN.with(|i| *i.borrow_mut() = Some(input));
        OUT.with(|o| *o.borrow_mut() = None);
        LAST_ITER.with(|c| c.set(0));
    }

    pub fn take_output() -> Option<Out> {
        OUT.with(|o| o.borrow_mut().take())
    }

    pub fn clear() {
        IN.with(|i| *i.borrow_mut() = None);
    }
}

/// EXPERIMENTAL (measurement only): every way `solve_lp_dual_with_tie_experiment`
/// can come back — a real completed solve, or either of the two ways the
/// experiment can cut a solve short (see `tie_experiment::In`'s own docs).
pub enum TieExperimentRun {
    Done { iters: usize, result: SimplexResult },
    Capped { iters: usize, obj: f64 },
    NewTie { iter: usize, tied_rows: Vec<usize> },
}

/// EXPERIMENTAL (measurement only): runs [`solve_lp_dual`] with the
/// `tie_experiment` side channel armed — see that module's own docs.
/// Always clears the channel again before returning (including if
/// `solve_lp_dual` itself panics further down the call stack — this
/// crate's own Netlib sweeps have hit real panics on a handful of
/// instances, and leaving `IN` armed for whatever call reuses this thread
/// next would silently corrupt an unrelated, non-experimental solve).
pub fn solve_lp_dual_with_tie_experiment(
    variables: &[VariableData],
    objective: &Objective,
    constraints: &[ConstraintRow],
    input: tie_experiment::In,
) -> TieExperimentRun {
    tie_experiment::set_input(input);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| solve_lp_dual(variables, objective, constraints)));
    let out = tie_experiment::take_output();
    let last_iter = tie_experiment::LAST_ITER.with(|c| c.get());
    tie_experiment::clear();
    match result {
        Ok(r) => match out {
            Some(tie_experiment::Out::Capped { iters, obj }) => TieExperimentRun::Capped { iters, obj },
            Some(tie_experiment::Out::NewTie { iter, tied_rows }) => TieExperimentRun::NewTie { iter, tied_rows },
            None => TieExperimentRun::Done { iters: last_iter, result: r },
        },
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

/// The dual method's actual work, operating on an already-presolved
/// `StdForm` — see [`solve_lp_on`]'s analogous split for the primal
/// method, and [`build_std_form_presolved`]'s docs for what "presolved"
/// means here.
/// `force_dse`: `true` skips the Devex warm-up entirely and prices every
/// iteration with exact DSE from the start, at DSE's usual per-pivot cost
/// (the extra `tau` FTRAN — see [`EdgeWeights`]'s own docs). No call site
/// passes `true` any more: the ill-conditioning trigger below now switches
/// pricing *in place* instead of re-entering this function. The parameter
/// survives only as the `ENOMOTO_FORCE_DSE` diagnostic override (see the
/// `force_dse` binding below), which forces the DSE-from-the-start path
/// for A/B measurement.
fn solve_lp_dual_on(std: &StdForm, force_dse: bool) -> SimplexResult {
    // Checked once here (not per-iteration) — see `timed!`'s own docs for
    // why this keeps a normal, non-profiling solve from paying for any
    // `Instant::now()` calls at all.
    let profile_phases = std::env::var("ENOMOTO_PROF_PHASES").is_ok();
    // Hoisted out of the loop like `profile_phases` itself — an
    // `env::var` lookup per iteration would otherwise inflate the very
    // wall-clock this diagnostic is meant to explain.
    let debug_chuzr = profile_phases && std::env::var("ENOMOTO_DEBUG_CHUZR").is_ok();
    let debug_eta_density = std::env::var("ENOMOTO_DEBUG_ETA_DENSITY").is_ok();
    // Gates the `UPDATE_VERIFY_*` atomic counters below (see
    // [`update_verify`]'s own docs) — hoisted for the same per-iteration
    // `env::var`-cost reason as `debug_chuzr` above, since `update_verify`
    // itself runs every single pivot, not just every `FT_CHECK_INTERVAL`.
    let debug_update_verify = std::env::var("ENOMOTO_PROF_UPDATE_VERIFY").is_ok();
    // Escape hatch for A/B measurement against the always-on default (see
    // [`update_verify`]'s own docs and this feature's own commit message):
    // the check normally runs unconditionally, like `FT_MIN_PIVOT`'s own
    // `try_update` rejection — this does not change pivot *selection*, only
    // how soon a numerically drifted pivot triggers a refactorization, so
    // disabling it can only ever make a solve *more* exposed to stale
    // eta-chain drift between trigger (1)'s own coarser checks, never
    // change which pivot a healthy solve picks.
    let update_verify_disabled = std::env::var("ENOMOTO_DISABLE_UPDATE_VERIFY").is_ok();
    // Reports the one-way Devex→DSE escalation (see [`DEVEX_STAGNATION_WINDOW`]'s
    // own docs) if/when it happens — at most once per solve, so unlike
    // `profile_phases`/`debug_chuzr` above this isn't hoisted for a
    // per-iteration cost reason, just for this file's own convention of
    // reading every `ENOMOTO_DEBUG_*` flag once, up front.
    let debug_devex = std::env::var("ENOMOTO_DEBUG_DEVEX").is_ok();
    // EXPERIMENTAL (measurement only, inert unless a caller has set
    // `tie_experiment::IN` via `solve_lp_dual_with_tie_experiment` — every
    // normal call path, including plain `solve_lp_dual`, leaves this `None`
    // and pays only this one thread-local read plus a `HashMap::new()` per
    // solve). Drives the tie-triggered branch-and-merge measurement: see
    // the `tie_experiment` module's own docs for the mechanism and why it's
    // a thread-local side channel rather than extra parameters threaded
    // through this function's ~10 existing return sites.
    let exp_in = tie_experiment::IN.with(|i| i.borrow().clone());
    let exp_active = exp_in.is_some();
    let exp_forced = exp_in.as_ref().map(|e| e.forced.clone()).unwrap_or_default();
    let exp_iter_cap = exp_in.as_ref().and_then(|e| e.iter_cap);
    let exp_stop_at_new_tie = exp_in.as_ref().map(|e| e.stop_at_new_tie).unwrap_or(false);
    let exp_tie_tol = exp_in.as_ref().map(|e| e.tie_tol).unwrap_or(1e-2);
    let exp_static_rule = exp_in.as_ref().and_then(|e| e.static_rule);
    let mut t = Tableau::new(std);
    let active_cost = perturb_costs(std);
    t.crash_dual_feasible(&active_cost);

    // Every refactorization in this function falls back to the primal
    // method on a numerically singular basis — see `try_refactorize`.
    // `crash_dual_feasible` above only changes nonbasic statuses/values,
    // not `basis`/`basis_pos`, so the basis here is still the initial
    // all-slack signed identity — same fast path as `solve_lp_on`'s.
    let Some(mut lu) = timed!(profile_phases, prof_phases::REFACTOR, {
        if profile_phases {
            prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        try_initial_refactorize(std, &t)
    }) else {
        if debug_devex {
            eprintln!("FALLBACK@initial");
        }
        return solve_lp_on(std);
    };
    // `Tableau::new` leaves every basic (slack) variable's `x` at its
    // default `0.0` — only nonbasic structural variables get a real value
    // there — so `x_B` needs one real solve before the loop's own
    // incremental maintenance (see below) has anything correct to build
    // on. Mirrors `d`'s own initialization from `active_cost.clone()`
    // just below, which is exact for the all-slack basis without needing
    // a solve only because `B = I` there.
    let init_rhs = t.compute_rhs();
    t.resync_basics(&lu, &init_rhs);
    let mut since_check = 0usize;
    // Separate, coarser cadence for trigger (1)'s own `basis_residual_norm`
    // check — see [`RESIDUAL_CHECK_MULTIPLIER`]'s own docs for why this is
    // sound: that check has seven to eight orders of magnitude of slack
    // before real drift could approach [`FT_RESIDUAL_TOL`], so it doesn't
    // need `compute_rhs`'s full `O(nnz(A))` cost paid at the same cadence
    // trigger (3)'s much cheaper `fill_count()` check runs at.
    let mut since_residual_check = 0usize;
    let m = std.n_rows;
    // Pricing starts cheap (Devex) and escalates one-way to exact DSE — see
    // [`EdgeWeights`]/[`DevexState`]'s own docs — unless this is the
    // one-time `force_dse` restart (see this function's own docs), which
    // skips straight to exact DSE and never looks back.
    // `ENOMOTO_FORCE_DSE`: diagnostic override for A/B measurement against
    // the Devex-start default — starts every solve in exact DSE the same
    // way the `force_dse` restart does, not just the ones that actually
    // trip a Devex-mode trigger. Confirmed (73-problem Netlib sweep) that
    // whether this helps or hurts a given problem is explained almost
    // entirely by whether DSE's own tie-breaking happens to need fewer or
    // more pivots than Devex's for that specific instance (iteration-count
    // ratio vs. wall-time ratio: Pearson r=0.87) — DSE wins by a wide
    // margin on genuinely degenerate stress tests (`cycle`, `forplan`,
    // `pilotnov`, `perold`, `stair`, ...) but *loses*, sometimes badly, on
    // several perfectly ordinary ones (`scsd8`, `nesm`, `bnl1`, the `grow*`
    // family) whose own degenerate tie-breaks DSE's different weights
    // happen to resolve less favorably — the same tie-break sensitivity
    // this crate's presolve cards document repeatedly, just surfacing here
    // via pricing instead of row/column order. No static feature tried
    // (size, density, row/column aspect ratio) predicted this in advance;
    // `scsd1`/`scsd6` favor DSE while `scsd8`, the same family scaled up,
    // opposes it. This is exactly why the default stays reactive
    // (Devex-start, escalate only on an actual trigger) rather than
    // switched to DSE-always based on any upfront guess.
    let force_dse = force_dse || std::env::var("ENOMOTO_FORCE_DSE").is_ok();
    let mut weights = if force_dse { EdgeWeights::Dse(DseState::new(m)) } else { EdgeWeights::Devex(DevexState::new(m)) };
    // Rolling window + running sum for the Devex→DSE stagnation check
    // below (see [`DEVEX_STAGNATION_WINDOW`]'s own docs) — a `VecDeque`
    // rather than re-summing every iteration, since this runs every pivot
    // for as long as pricing stays in Devex mode. `obj_scale` is a one-time
    // O(n) pass over the (already-perturbed) cost vector, used to turn the
    // window average into a relative, scale-independent check rather than
    // comparing raw objective contributions against an absolute constant.
    let mut devex_window: std::collections::VecDeque<f64> = std::collections::VecDeque::with_capacity(DEVEX_STAGNATION_WINDOW);
    let mut devex_window_sum = 0.0f64;
    let obj_scale = active_cost.iter().fold(0.0f64, |acc, &c| acc.max(c.abs())).max(1.0);

    // Bland's-rule fallback state (see the BFRT block's own docs for why
    // and when this triggers): `stall_count` counts consecutive pivots
    // contributing essentially nothing to the objective; once it exceeds
    // `stall_limit` (scaled to problem size — a fixed constant would be
    // either too eager on large problems or too slow to rescue small
    // ones), `bland_mode` latches on for the rest of this solve (Bland's
    // rule's finite-termination proof doesn't need it to switch back off).
    let mut stall_count: usize = 0;
    let stall_limit = (5 * m).max(500);
    let mut bland_mode = false;

    // Reduced costs `d[j] = c[j] - y . A_j` for *every* column, maintained
    // directly and incrementally (see the update-dual derivation below)
    // rather than recomputed from a `y` vector every iteration — one
    // fewer per-candidate dot product than even the `y`-based incremental
    // update this replaces (HiGHS's `HEkkDualRow::updateDual` maintains
    // its `workDual` array the same way, straight off the pivotal row,
    // rather than forming a price vector at all). Exactly `c[j]` at the
    // all-slack starting basis (`y = 0` there — every slack's cost is
    // `0` — so `d[j] = c[j] - 0`) — no BTRAN or dot product needed even
    // for this initial value.
    let mut d = active_cost.clone();
    // PRICE's output buffer, allocated once and reused across iterations
    // (reset by `touched_cols` bookkeeping below) rather than
    // `vec![0.0f64; std.n_total]` fresh every iteration — see the PRICE
    // comment further down.
    let mut a_p = vec![0.0f64; std.n_total];
    let mut touched_cols: Vec<usize> = Vec::new();
    // `touched[j]` is the membership test for `touched_cols` — see the
    // PRICE loop for why `a_p[j] == 0.0` can't serve as one.
    let mut touched = vec![false; std.n_total];
    // Basic variables whose (tiny, row-scale-relative) infeasibility
    // chuzc1 has already shown no column can move — see that branch.
    let mut noise_feasible = vec![false; std.n_total];
    // Hyper-sparse `chuzr`'s incrementally maintained infeasible-row set —
    // see [`InfeasibleRows`]'s own docs. Built fresh here (an `O(m)` scan,
    // paid once) since `t.x` was just made exact by the `resync_basics`
    // call above; every resync inside the loop below rebuilds it the same
    // way for the same reason, and every pivot in between updates it
    // incrementally instead.
    let mut infeasible_rows = InfeasibleRows::new(m);
    infeasible_rows.rebuild(m, |i| row_infeasible(std, &t, &noise_feasible, i));
    // Sequential-vs-rayon choice for `chuzr` below, decided fresh every
    // iteration from `infeasible_rows.rows.len()` (not `m`, now that
    // `chuzr` itself scans only that set) against `RAYON_SIZE_THRESHOLD` —
    // see that constant's own docs for why this crate settled on a size
    // threshold rather than a live run-both-and-time race, and never found
    // `rayon` winning even at sizes this small set essentially never
    // reaches in practice.
    // Debug-only verification helper (see the `#[cfg(debug_assertions)]`
    // block below): the expensive, from-first-principles way to get `d`
    // for the *current* basis — one BTRAN plus one dot product per
    // column, exactly what every iteration used to cost before this
    // optimization.
    let fresh_d = |lu: &sparse_lu::FtLu, t: &Tableau, cost: &[f64]| -> Vec<f64> {
        let cost_b: Vec<f64> = t.basis.iter().map(|&v| cost[v]).collect();
        let y = lu.solve_transpose(&cost_b);
        (0..std.n_total)
            .map(|j| cost[j] - sparse_dot_dense(t.column_sparse(j), &y))
            .collect()
    };

    // Scratch/output buffers for every iteration's FTRAN/BTRAN calls,
    // declared once here rather than fresh inside the loop — see
    // `FtLu::solve_into`/`solve_transpose_into`'s own docs for why a bare
    // `lu.solve(...)` per call used to cost 2-3 heap allocations, several
    // times every single pivot (BTRAN-DSE for `rho_p`, the entering
    // column's `alpha`, the DSE cross-term `tau`, and — when BFRT flips
    // are pending — one more for `combined`).
    let mut lu_scratch = vec![0.0; m];
    let mut rho_p_buf = vec![0.0; m];
    let mut a_enter_buf = vec![0.0; m];
    let mut alpha_buf = vec![0.0; m];
    let mut tau_buf = vec![0.0; m];
    // `combined_buf`'s own nonzero indices are tracked the same
    // touched-index way `a_p`/`touched_cols`/`touched` are above: BFRT's
    // combined flip vector is a sum of a handful of flipped columns' own
    // (genuinely sparse, real-LP) entries, so it is itself typically
    // sparse — but a naive `combined_buf[i] += v * delta_x` can touch the
    // same row `i` more than once across different flipped columns, so
    // membership needs an explicit flag (not "is it exactly zero", for the
    // same cancellation reason `touched` exists for `a_p`) to avoid
    // recording one row twice, which `l_solve_sparse_into` would then
    // treat as an overwrite rather than a sum.
    let mut combined_buf = vec![0.0; m];
    let mut combined_touched: Vec<usize> = Vec::new();
    let mut combined_touched_flag = vec![false; m];
    let mut combined_alpha_buf = vec![0.0; m];
    // Dedicated to the entering column's FTRAN alone — never shared with
    // `lu_scratch` — since `sparse_lu::FtLu::solve_sparse_into` requires
    // its own `scratch` buffer to already be all-zero on entry (its own
    // docs explain why), an invariant a plain `solve_into` call through
    // `lu_scratch` would silently violate. The BFRT combined-flip solve
    // (below) reuses these same two buffers rather than needing its own
    // dedicated pair: within one iteration it always runs strictly before
    // the entering column's own `solve_sparse_into` call, and every call
    // leaves both buffers back at all-zero before returning (`solve_sparse_into`'s
    // own documented postcondition), so the entering column's call always
    // still finds them zeroed exactly as it requires.
    let mut sparse_lu_scratch = vec![0.0; m];
    let mut gp_scratch = sparse_lu::GpScratch::new(m);

    // Per-call-site FTRAN **result**-density running averages, feeding the
    // dense/sparse dispatch below alongside each solve's own input
    // nonzero count (`sparse_lu::FtranDensity`'s own docs, and
    // `docs/lu_comparison_enomoto_vs_highs.md` §2.7 — the gap this closes:
    // an rhs that is sparse on input says nothing about how far `L`'s own
    // reach fans out, and that gap widens with `m`). The entering column's
    // FTRAN and the BFRT combined-flip FTRAN keep separate histories
    // because their right-hand sides (one constraint column vs. a sum over
    // every column flipped this iteration) fill in to genuinely different
    // densities. Declared out here with the buffers they parallel, so the
    // history survives every refactorization this loop does: it is a
    // property of the solve, not of any one `FtLu` (HiGHS keeps the same
    // averages in `HEkk`, likewise across INVERTs).
    let mut density_col_aq = sparse_lu::FtranDensity::new();
    let mut density_bfrt = sparse_lu::FtranDensity::new();
    // Dedicated capture buffers for `FtLu::try_update_precomputed` (see its
    // own docs): `e_tilde_buf` is filled as a side effect of this
    // iteration's `rho_p` BTRAN (below) and `a_tilde_buf` as a side effect
    // of this same iteration's entering-column FTRAN (further down) —
    // both intermediates `try_update` used to recompute from scratch.
    // Deliberately separate from every other buffer above: nothing else
    // may write through these between the two capture points and the
    // `try_update_precomputed` call near the end of the loop, or a stale
    // value would silently corrupt that update (see
    // `try_update_precomputed`'s own docs on this exact hazard).
    let mut a_tilde_buf = vec![0.0; m];
    let mut e_tilde_buf = vec![0.0; m];

    let max_iters = max_iters_for(std.n_rows, std.n_total);
    for _iter in 0..max_iters {
        if profile_phases {
            prof_phases::ITERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        // EXPERIMENTAL (measurement only, inert unless `exp_active` — see
        // `tie_experiment`'s own docs): the "run a few more iterations down
        // this branch, then compare" half of the tie-triggered
        // branch-and-merge measurement.
        if exp_active {
            tie_experiment::LAST_ITER.with(|c| c.set(_iter));
            if let Some(cap) = exp_iter_cap {
                if _iter >= cap {
                    let obj: f64 = active_cost.iter().zip(t.x.iter()).map(|(c, x)| c * x).sum();
                    tie_experiment::OUT.with(|o| *o.borrow_mut() = Some(tie_experiment::Out::Capped { iters: _iter, obj }));
                    return SimplexResult { status: Status::Infeasible, x: None }; // dummy sentinel: real result comes back via `tie_experiment::OUT`
                }
            }
        }
        // `x_B` is otherwise maintained incrementally, every iteration,
        // by the primal-step/bound-flip updates near the bottom of this
        // loop (the same values a from-scratch recompute would produce,
        // up to floating-point drift) — so, unlike an earlier version of
        // this function, there is no unconditional full recompute here.
        // Trigger (3)'s `fill_count()` check is cheap (an `O(1)`-ish length
        // sum) and still runs every `FT_CHECK_INTERVAL` iterations; trigger
        // (1)'s own check additionally needs `compute_rhs` (`O(nnz(A))`)
        // and `basis_residual_norm` (a full basis-matrix multiply), so it
        // runs at the coarser `RESIDUAL_CHECK_MULTIPLIER`-scaled cadence
        // instead (see that constant's own docs for why this is safe) —
        // `rhs` itself is computed only when at least one of the two checks
        // this round actually needs it (either because trigger (3) already
        // fired, in which case it's needed for the resync below regardless,
        // or because this round is also due for trigger (1)'s own check).
        // Deliberately computed *before* any resync below when it is
        // computed at all: `basis_residual_norm` must see the still-
        // incremental `x_B` to actually detect drift, not a value that
        // was just snapped back to agree with `rhs` by this same call.
        since_check += 1;
        if since_check >= FT_CHECK_INTERVAL {
            since_check = 0;
            let bump_too_big = lu.fill_count() > FT_BUMP_LIMIT_FACTOR * m.max(1);
            since_residual_check += 1;
            let due_for_residual_check = since_residual_check >= RESIDUAL_CHECK_MULTIPLIER;
            if bump_too_big || due_for_residual_check {
                let rhs = t.compute_rhs();
                let residual_too_big = if due_for_residual_check {
                    since_residual_check = 0;
                    !bump_too_big && t.basis_residual_norm(&rhs) > FT_RESIDUAL_TOL
                } else {
                    false
                };
                if bump_too_big || residual_too_big {
                    timed!(profile_phases, prof_phases::REFACTOR, {
                        if profile_phases {
                            prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                        let Some(l) = try_refactorize(std, &t, Some(&lu)) else {
                            if debug_devex {
                                eprintln!("FALLBACK@residual iter={_iter}");
                            }
                            return solve_lp_on(std);
                        };
                        lu = l;
                        t.resync_basics(&lu, &rhs);
                        d = fresh_d(&lu, &t, &active_cost);
                        // `resync_basics` just rewrote `x_B` for every row
                        // at once, outside `InfeasibleRows`'s own
                        // incremental `set` calls — only a full rebuild
                        // can catch up (see its own docs).
                        infeasible_rows.rebuild(m, |i| row_infeasible(std, &t, &noise_feasible, i));
                    });
                }
            }
        }
        if lu.update_count() > FT_MAX_UPDATES {
            timed!(profile_phases, prof_phases::REFACTOR, {
                if profile_phases {
                    prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                let Some(l) = try_refactorize(std, &t, Some(&lu)) else {
                    if debug_devex {
                        eprintln!("FALLBACK@ft_max_updates iter={_iter}");
                    }
                    return solve_lp_on(std);
                };
                lu = l;
                let rhs = t.compute_rhs();
                t.resync_basics(&lu, &rhs);
                d = fresh_d(&lu, &t, &active_cost);
                infeasible_rows.rebuild(m, |i| row_infeasible(std, &t, &noise_feasible, i));
            });
        }

        // chuzr: most DSE-attractive primal-infeasible basic row, scanned
        // over `infeasible_rows.rows` alone (hyper-sparse chuzr — see
        // [`InfeasibleRows`]'s own docs for why this is exact, not an
        // approximation) rather than the full `0..m`. Each row's
        // infeasibility/score is independent of every other.
        // `chuzr_parallel` below picks sequential for every problem size
        // this crate realistically sees — see `RAYON_SIZE_THRESHOLD`'s own
        // docs. The same reasoning applies to chuzc1 below, which stays
        // unconditionally sequential.
        let chuzr_scan = |i: usize| -> Option<(usize, f64, bool)> {
            let var = t.basis[i];
            let val = t.x[var];
            // A row chuzc1 already proved it cannot move, whose
            // infeasibility was within that row's own rounding noise (see
            // the `noise_feasible` marking in the chuzc1-empty branch
            // below), is treated as feasible from then on rather than
            // re-selected every iteration.
            if noise_feasible[var] {
                return None;
            }
            let delta = if val < std.lb[var] - PRIMAL_FEAS_TOL {
                std.lb[var] - val
            } else if val > std.ub[var] + PRIMAL_FEAS_TOL {
                val - std.ub[var]
            } else {
                0.0
            };
            if delta <= PRIMAL_FEAS_TOL {
                return None;
            }
            let score = delta * delta / weights.weight(i).max(STEEPEST_EDGE_FLOOR);
            Some((i, score, val < std.lb[var]))
        };
        // In `bland_mode`, chuzr also switches to Bland's rule: smallest
        // *variable* index among eligible (primal-infeasible) rows, not
        // the DSE-attractive one — consistent tie-breaking on both the
        // leaving and entering side is what Bland's finite-termination
        // proof actually requires.
        let chuzr_parallel = infeasible_rows.rows.len() > RAYON_SIZE_THRESHOLD;
        let chuzr = timed!(profile_phases, prof_phases::CHUZR, {
            if bland_mode {
                infeasible_rows.rows.iter().copied().filter_map(chuzr_scan).min_by_key(|&(i, _, _)| t.basis[i])
            } else {
                // Tie-broken by row index (`a.0`/`b.0`), not just `score`
                // (`a.1`): `max_by` alone only guarantees returning *a*
                // maximum, and on a genuine tie between two candidates,
                // which one that is can depend on traversal/reduction
                // order — identical for a plain sequential scan every run,
                // but not necessarily identical between `rayon`'s
                // parallel-reduction order and the sequential one, or even
                // between two separate parallel runs if thread scheduling
                // varies. On a highly degenerate problem (many exactly-
                // tied DSE scores) that showed up as this solve's own
                // total pivot count — and wall-clock time — varying by
                // several times across otherwise-identical runs of the
                // same binary (confirmed on Netlib `degen3`), since which
                // row got selected as "the" most-infeasible one at a tied
                // score could differ and cascade into a completely
                // different pivot sequence. Adding `a.0.cmp(&b.0)` as a
                // secondary key makes the comparator a strict total order
                // over `(score, row_index)` pairs (row indices are unique,
                // so no residual tie is possible) — the maximum of a
                // totally ordered set is unique and traversal-order-
                // independent by construction, so this doesn't just make
                // ties "less likely to matter", it removes the
                // sequential-vs-parallel (and parallel-vs-parallel)
                // discrepancy entirely.
                let cmp = |a: &(usize, f64, bool), b: &(usize, f64, bool)| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0));
                // EXPERIMENTAL (measurement only, inert unless `exp_active` —
                // see `tie_experiment`'s own docs): the "branch into each
                // tied candidate" half of the tie-triggered branch-and-merge
                // measurement. Falls straight through to this crate's real
                // default tie-break whenever there's no tie at all (`tied.len()
                // <= 1`), so a non-tied pivot is completely unaffected even
                // with the experiment armed.
                if exp_active {
                    let scored: Vec<(usize, f64, bool)> = if chuzr_parallel {
                        use rayon::prelude::*;
                        infeasible_rows.rows.par_iter().copied().filter_map(chuzr_scan).collect()
                    } else {
                        infeasible_rows.rows.iter().copied().filter_map(chuzr_scan).collect()
                    };
                    match scored.iter().cloned().max_by(cmp) {
                        None => None,
                        Some(best) => {
                            let mut tied: Vec<(usize, f64, bool)> = scored.iter().cloned().filter(|c| c.1 >= best.1 * (1.0 - exp_tie_tol)).collect();
                            if tied.len() > 1 {
                                tied.sort_unstable_by(|a, b| cmp(b, a));
                                // EXPERIMENTAL (measurement only): static alternative
                                // tie-break — see `tie_experiment::In::static_rule`'s
                                // own docs. Recomputes each tied row's raw (pre-weight)
                                // infeasibility straight from `t`/`std` rather than
                                // having `chuzr_scan` return it, since it's needed for
                                // only the (rare) tied rows, not every candidate.
                                if let Some(rule) = exp_static_rule {
                                    let raw_delta = |i: usize| -> f64 {
                                        let var = t.basis[i];
                                        let val = t.x[var];
                                        if val < std.lb[var] { std.lb[var] - val } else if val > std.ub[var] { val - std.ub[var] } else { 0.0 }
                                    };
                                    let pick = match rule {
                                        0 => tied.iter().cloned().max_by(|a, b| raw_delta(a.0).total_cmp(&raw_delta(b.0))),
                                        1 => tied.iter().cloned().min_by(|a, b| raw_delta(a.0).total_cmp(&raw_delta(b.0))),
                                        _ => tied.iter().cloned().min_by_key(|c| c.0),
                                    };
                                    pick
                                } else if let Some(&choice) = exp_forced.get(&_iter) {
                                    Some(tied.get(choice).or_else(|| tied.first()).copied().unwrap())
                                } else if exp_stop_at_new_tie {
                                    let tied_rows: Vec<usize> = tied.iter().map(|c| c.0).collect();
                                    tie_experiment::OUT.with(|o| *o.borrow_mut() = Some(tie_experiment::Out::NewTie { iter: _iter, tied_rows }));
                                    return SimplexResult { status: Status::Infeasible, x: None }; // dummy sentinel: real result comes back via `tie_experiment::OUT`
                                } else {
                                    Some(best)
                                }
                            } else {
                                Some(best)
                            }
                        }
                    }
                } else if chuzr_parallel {
                    use rayon::prelude::*;
                    infeasible_rows.rows.par_iter().copied().filter_map(chuzr_scan).max_by(cmp)
                } else {
                    infeasible_rows.rows.iter().copied().filter_map(chuzr_scan).max_by(cmp)
                }
            }
        });
        #[cfg(debug_assertions)]
        {
            // Verifies `InfeasibleRows`'s incremental maintenance against
            // the expensive from-first-principles scan it replaced —
            // exact equality of the *sets*, not just their sizes, since a
            // bug that swaps one infeasible row for a different one at the
            // same count would otherwise go unnoticed.
            let mut fresh: Vec<usize> = (0..m).filter(|&i| row_infeasible(std, &t, &noise_feasible, i)).collect();
            let mut maintained = infeasible_rows.rows.clone();
            fresh.sort_unstable();
            maintained.sort_unstable();
            debug_assert_eq!(fresh, maintained, "InfeasibleRows drifted from a fresh full scan");
        }
        if debug_chuzr {
            prof_phases::INFEAS_ROWS.fetch_add(infeasible_rows.rows.len(), std::sync::atomic::Ordering::Relaxed);
        }
        let Some((p, _best_score, leaving_infeasible_low)) = chuzr else {
            // Primal feasible against the *perturbed* costs — dual
            // feasibility held throughout by this loop's own invariant,
            // so this point is optimal for the perturbed problem. Whether
            // it is *also* optimal for the true problem depends on
            // whether perturbation happened to mask a genuine dual
            // infeasibility (rare — that's the point of perturbing by
            // something this small — but not impossible).
            let true_d = fresh_d(&lu, &t, &std.c);
            let true_dual_feasible = (0..std.n_total).all(|j| match t.nb_status[j] {
                None => true,
                Some(NbStatus::Lower) => true_d[j] >= -TOL,
                Some(NbStatus::Upper) => true_d[j] <= TOL,
                Some(NbStatus::Zero) => true_d[j].abs() <= TOL,
            });
            if true_dual_feasible {
                return SimplexResult { status: Status::Optimal, x: Some(t.x[0..t.n_orig()].to_vec()) };
            }
            // Perturbation masked a genuine dual infeasibility: this basis
            // is primal feasible but not (quite) dual feasible for the
            // true costs — the complementary situation to what the dual
            // method's own invariant assumes, so finishing the solve here
            // needs the primal method's invariant instead (primal
            // feasibility preserved, working toward dual feasibility),
            // which is exactly `run_phase`'s phase 2 — already primal
            // feasible, so no phase 1 needed. `expand`/`se` start fresh
            // rather than mid-sequence, since primal EXPAND and dual
            // steepest-edge track unrelated quantities; correctness is
            // unaffected either way, only pricing quality for however
            // many iterations this cleanup takes (expected to be brief,
            // since perturbation is tiny).
            let mut expand = ExpandState::new();
            let mut se = SteepestEdgeState::new(std);
            let mut stall = PrimalStallState::new();
            if debug_devex {
                eprintln!("DUAL->PRIMAL cleanup handoff at dual_iter={_iter}");
            }
            let Some(status) = run_phase(std, &mut t, false, &mut lu, &mut since_check, &mut expand, &mut se, &mut stall) else {
                // `run_phase` hit an unrecoverable singular basis partway
                // through this cleanup (see its own docs) — `t` from this
                // point on can no longer be trusted (confirmed on Netlib's
                // `cycle`: trusting it here once produced a wildly wrong
                // "optimal" objective instead of an honest failure), so
                // restart the whole solve from scratch via the primal
                // method's own from-a-fresh-basis entry point, exactly like
                // every other numerically-spent-trajectory fallback in this
                // function already does.
                return solve_lp_on(std);
            };
            return SimplexResult {
                status: status.clone(),
                x: if status == Status::Optimal { Some(t.x[0..t.n_orig()].to_vec()) } else { None },
            };
        };

        // Pivotal row: rho_p = B^-T e_p (btran). Captures `e_tilde_buf`
        // (the post-U^-T, pre-R-reverse intermediate) as a side effect —
        // this row `p` is exactly the `basis_slot` this iteration's own
        // `try_update_precomputed` call (near the end of the loop) will
        // use, provided the pivot actually commits (a `continue` on
        // `verify_failed`/infeasibility below never reaches that call at
        // all, so a stale capture is never fed to it) — see
        // `try_update_precomputed`'s own docs for why this is bit-for-bit
        // the value it would otherwise recompute from scratch.
        timed!(
            profile_phases,
            prof_phases::BTRAN,
            lu.solve_transpose_unit_capture(p, &mut lu_scratch, &mut rho_p_buf, &mut e_tilde_buf)
        );
        let rho_p = &rho_p_buf;

        // PRICE (Huangfu & Hall §2.2.2's "spmv"), row-major: a_p = rho_p^T
        // A, computed by walking only rho_p's *nonzero* rows and scanning
        // each one's own sparse row of `A` — not, as an earlier version of
        // this function did, by visiting every nonbasic column's own
        // nonzeros regardless of whether rho_p even touches the rows that
        // column lives in. A genuinely sparse rho_p (common on many
        // problems; it is a single BTRAN result, not an arbitrary dense
        // vector) turns this into real savings: columns whose only
        // nonzero rows are exactly where rho_p is zero cost nothing here.
        // Deliberately sequential, not parallelized: multiple rows scatter
        // into the same `a_p[j]`, so a naive per-row parallel write would
        // race, and a rayon fold/reduce (a private length-`n_total` buffer
        // per thread, merged at the end) was tried and measured slower
        // than this plain loop for this crate's typical problem sizes —
        // the reduction's own O(n_total) merge cost outweighs the savings
        // once `m` and the number of nonzero rows are this modest.
        //
        // `a_p` is a persistent, reused buffer (declared before the loop),
        // not a fresh `vec![0.0f64; std.n_total]` every iteration — an
        // earlier version reallocated and zero-filled the whole thing here
        // every single iteration, which is `O(n_total)` regardless of how
        // sparse `rho_p`/the resulting `a_p` actually are, undoing exactly
        // the sparsity this PRICE step is written to exploit. `touched_cols`
        // records which entries this iteration actually set (a column can
        // only be pushed once: the check is "was it exactly zero before
        // this contribution"), so chuzc1 below and the end-of-iteration
        // reset can both walk just those instead of `0..n_total` too.
        // Membership is tracked by an explicit `touched` marker, *not* by
        // `a_p[j] == 0.0`: the running sum for a column can cancel back to
        // exactly `0.0` partway through (two rows contributing `+r*v` and
        // `-r*v`, common once presolve's row rewriting leaves many rows
        // sharing identical coefficient blocks), and a zero-test would then
        // push the same column a *second* time. A duplicate here is not
        // harmless: chuzc1 would emit two candidates for one column, the
        // BFRT walk would count its flip twice (moving `x_B` by twice the
        // bound width in `combined` while `x[j]` itself flips once — seen
        // on real data as a basic slack landing exactly one full bound
        // width away from its true value, feeding a false `Infeasible`),
        // and the update-dual loop below would subtract `theta_d * a_p[j]`
        // from `d[j]` twice.
        timed!(profile_phases, prof_phases::PRICE, {
            for i in 0..m {
                let r = rho_p[i];
                if r.abs() <= TOL {
                    continue;
                }
                for &(j, v) in std.rows.row(i) {
                    // Fixed columns (`lb[j] == ub[j]` — every equality
                    // row's own slack, plus every colsingleton/doubleton-
                    // eliminated original variable, see
                    // `build_std_form_presolved`'s own docs) can never
                    // usefully enter: chuzc1 below would never pick one
                    // (flipping a zero-width column moves nothing), so
                    // there is no reason to pay for it in `touched_cols`,
                    // chuzc1's per-column eligibility check, or the
                    // dual-update/reset loops that walk `touched_cols`
                    // later this same iteration. Skipping them here — at
                    // the one place a column enters `touched_cols` at all
                    // — is cheaper than filtering them out of every one of
                    // those loops individually. `d[j]` for such a column is
                    // therefore never incrementally updated after its
                    // initial `active_cost[j]` value — harmless, since
                    // nothing in this function ever reads `d[j]` for a
                    // fixed column again (the final dual-feasibility
                    // recheck uses `fresh_d`, an independent recompute, not
                    // this incremental array) — see the matching skip in
                    // the `#[cfg(debug_assertions)]` cross-check below.
                    if std.lb[j] == std.ub[j] {
                        continue;
                    }
                    if !touched[j] {
                        touched[j] = true;
                        touched_cols.push(j);
                    }
                    a_p[j] += r * v;
                }
            }
        });

        // chuzc1 (Huangfu & Hall §2.2.2): every eligible nonbasic j (sign
        // of a_pj compatible with restoring feasibility at row p, given
        // its bound status), with its reduced cost `dj` — a plain array
        // lookup into the incrementally-maintained `d` above, not a dot
        // product — and ratio |d_j / a_pj|. Each column's eligibility/
        // ratio is independent of every other; only `touched_cols` (the
        // columns PRICE actually gave a nonzero `a_p` entry — everything
        // else is ineligible anyway, since `chuzc1` requires `a_pj != 0`)
        // needs scanning, not the full `0..n_total`, and this is
        // sequential rather than rayon regardless (see the chuzr comment
        // above for the measured-overhead reason); the candidates are then
        // sorted by ascending ratio for chuzc2 (BFRT) below.
        #[derive(Clone, Copy)]
        struct ChuzcCand {
            j: usize,
            a_pj: f64,
            dj: f64,
            ratio: f64,
        }
        // Total order on `(ratio, j)` — `j` as the tiebreak (rather than
        // leaving ties in whatever order a sort/heap happens to produce)
        // is what makes `BinaryHeap<Reverse<ChuzcCand>>` below a well-defined
        // substitute for `sort_unstable_by(|a,b| a.ratio.total_cmp(&b.ratio))`:
        // a heap has no notion of "stable" input order to fall back on for
        // ties the way a sort does, so without an explicit tiebreak the pop
        // order on tied ratios would depend on push/sift order (touched_cols
        // scan order), not on `j`.
        impl PartialEq for ChuzcCand {
            fn eq(&self, other: &Self) -> bool {
                self.ratio == other.ratio && self.j == other.j
            }
        }
        impl Eq for ChuzcCand {}
        impl PartialOrd for ChuzcCand {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for ChuzcCand {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.ratio.total_cmp(&other.ratio).then_with(|| self.j.cmp(&other.j))
            }
        }
        let candidates: Vec<ChuzcCand> = timed!(
            profile_phases,
            prof_phases::CHUZC1,
            touched_cols
                .iter()
                .copied()
                .filter_map(|j| {
                    let st = t.nb_status[j]?;
                    let a_pj = a_p[j];
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
                        // Paper \S2.2 (ii)-(iii): a free column at `0` can
                        // move either way, so it is eligible whenever
                        // `|a_pj| > TOL` (already checked above).
                        NbStatus::Zero => true,
                    };
                    if !eligible {
                        return None;
                    }
                    let dj = d[j];
                    let ratio = (dj / a_pj).abs();
                    Some(ChuzcCand { j, a_pj, dj, ratio })
                })
                .collect()
        );
        if candidates.is_empty() {
            // No column can move this row in the direction it needs. That
            // is the dual method's primal-infeasibility certificate — but
            // only if the infeasibility is real. A basic value is computed
            // from `rhs - N x_N`, so its rounding noise scales with the
            // row's own magnitude; on a row with `rhs ~ 3.4e6` (Netlib
            // `agg`, even after Ruiz scaling) an infeasibility of `1.4e-7`
            // is noise, not a violated constraint, and the "no column can
            // fix it" finding is simply correct for something that was
            // never broken. Such a row is marked feasible-within-noise
            // (`chuzr` skips it from now on) and the iteration is retried
            // on the next-most-infeasible row instead of terminating. The
            // absolute `PRIMAL_FEAS_TOL` used by `chuzr` itself is left
            // strict on purpose: relaxing it globally by row magnitude was
            // tried and measurably degraded final objective accuracy on
            // otherwise well-behaved instances.
            let lv = t.basis[p];
            let infeas = (std.lb[lv] - t.x[lv]).max(t.x[lv] - std.ub[lv]).max(0.0);
            if infeas <= PRIMAL_FEAS_TOL * std.b[p].abs().max(1.0) {
                noise_feasible[lv] = true;
                infeasible_rows.set(p, false);
                for &j in &touched_cols {
                    a_p[j] = 0.0;
                    touched[j] = false;
                }
                touched_cols.clear();
                continue;
            }
            return SimplexResult { status: Status::Infeasible, x: None };
        }

        let leaving_var = t.basis[p];
        let target_bound = if leaving_infeasible_low { std.lb[leaving_var] } else { std.ub[leaving_var] };
        let need_sign = (t.x[leaving_var] - target_bound).signum();

        // Bland's rule (Bland 1977) as a last-resort anti-cycling fallback,
        // triggered by `stall_count` below once cost perturbation and the
        // Harris-style pass have failed to prevent genuine stalling: pick
        // the *smallest-index* eligible candidate directly as `q` — no
        // ratio test, no BFRT flipping. It is the only pivoting rule with a
        // textbook finite-termination proof that doesn't depend on
        // problem-specific perturbation working out, at the cost of being
        // far slower per unit of progress than DSE/Harris — confirmed on
        // real data (Netlib's `qap8`, a highly symmetric/degenerate QAP
        // relaxation) to be worth that cost only rarely: tracing its
        // objective value directly found it climbing steadily for tens of
        // thousands of iterations (perturbation working as intended)
        // before flattening *exactly* once within about 1% of the true
        // optimum — genuine degenerate stalling, not merely slow
        // convergence, and not something raising `MAX_ITERS` alone fixes.
        let (q, dj_q, flips): (usize, f64, Vec<(usize, NbStatus)>) = timed!(profile_phases, prof_phases::BFRT, if bland_mode {
            // Pure textbook Bland's rule (smallest index, full stop)
            // ignores pivot magnitude entirely, which is exactly what the
            // Harris pass above exists to avoid — confirmed by measurement
            // that naively applying it this way reintroduces the same
            // "numerically singular basis" crash on other Netlib instances
            // (`degen3`) even while it correctly rescues the stalled one
            // (`qap8`) it was added for. Prefer the smallest-index
            // candidate whose pivot is at least usably large, and only
            // fall back to the unrestricted smallest index (accepting the
            // stability risk) if literally every eligible candidate's
            // pivot is that small.
            let best = candidates
                .iter()
                .filter(|c| c.a_pj.abs() > FT_MIN_PIVOT)
                .min_by_key(|c| c.j)
                .unwrap_or_else(|| candidates.iter().min_by_key(|c| c.j).unwrap());
            (best.j, best.dj, Vec::new())
        } else {
            // chuzc2, bound-flipping ratio test (BFRT) with a Harris-style
            // stability pass (see `HARRIS_RATIO_TOL`'s own docs). Pass 1
            // walks the candidates in ascending ratio order — a bounded
            // candidate (finite `ub - lb`) is provisionally flipped to its
            // opposite bound as long as doing so doesn't yet bring the
            // leaving variable to `target_bound`; the first candidate that
            // would reach or overshoot it (or the first unbounded
            // candidate, which can never be flipped) marks `stop_idx`, the
            // boundary the plain BFRT algorithm would have used directly.
            // This part is Gauss-Seidel over the sorted list — each step's
            // decision depends on the leaving variable's value *after*
            // every previous step's flip — so, unlike chuzc1 above, it
            // runs sequentially.
            //
            // Measured across a wide range of Netlib instances, `stop_idx`
            // is almost always tiny relative to the candidate pool (e.g.
            // `wood1p` sorts ~1530 candidates per iteration but only ever
            // walks ~1.7 of them; `scsd8` 581 vs 1.4; `25fv47` 386 vs 1.5)
            // — a full `sort_unstable_by` over the whole pool did `O(k log
            // k)` work to answer a question pass 1 only needed the first
            // `w << k` of. A min-heap (`BinaryHeap<Reverse<ChuzcCand>>`,
            // `O(k)` to build) popped one element at a time reproduces
            // exactly the same ascending-`(ratio, j)` order (see
            // `ChuzcCand`'s `Ord` impl above) but does `O(w log k)` work
            // instead: pass 1 stops popping the moment it finds `stop_idx`,
            // and pass 2's backward Harris window search only ever needs
            // indices `<= stop_idx` (see that pass's own comment for why),
            // i.e. only candidates already popped into `sorted_prefix` —
            // so both passes are unchanged in behavior, just fed a lazily
            // materialized prefix instead of the fully sorted vec.
            let n_candidates = candidates.len();
            let mut heap: BinaryHeap<Reverse<ChuzcCand>> = candidates.into_iter().map(Reverse).collect();
            let mut sorted_prefix: Vec<ChuzcCand> = Vec::with_capacity(4);

            let mut x_leaving_now = t.x[leaving_var];
            // "Reached the target" is judged relative to how far the
            // leaving variable had to travel this iteration: a flip of
            // width `w` lands within `~eps * w` of where exact arithmetic
            // says it should (Netlib `maros`: a `16270.15`-wide flip
            // ending `1.04e-7` short of an exact `0`), and a fixed absolute
            // tolerance below that rounding floor turns such an exactly-
            // reaching flip into a false "still short" — exhausting the
            // list and reporting `Infeasible`. Small moves keep the plain
            // `PRIMAL_FEAS_TOL` (the `max(1.0)` floor).
            let reach_tol = PRIMAL_FEAS_TOL * (x_leaving_now - target_bound).abs().max(1.0);
            let mut stop_idx: Option<usize> = None;
            while let Some(Reverse(cand)) = heap.pop() {
                let idx = sorted_prefix.len();
                sorted_prefix.push(cand);
                let width = std.ub[cand.j] - std.lb[cand.j];
                if !width.is_finite() {
                    stop_idx = Some(idx);
                    break;
                }
                let old_status = t.nb_status[cand.j].unwrap();
                let delta_x = match old_status {
                    NbStatus::Lower => width,
                    NbStatus::Upper => -width,
                    NbStatus::Zero => unreachable!("a free (`Zero`) column's width is infinite, so it stops the walk above"),
                };
                let flipped_x_leaving = x_leaving_now - cand.a_pj * delta_x;
                // A flip that lands within `reach_tol` of `target_bound`
                // counts as "reached", not "still short by a rounding
                // error" — a strict `> 0.0`/fixed-`TOL` comparison here
                // (confirmed on real data: Netlib `vtp.base` landed at
                // `-2.8e-14` off an exact `0`, `maros` at `-1.04e-7` off a
                // target `16270.15` away) fed the walk straight past the
                // last real candidate into the false-infeasible branch
                // below on an otherwise perfectly feasible step.
                if (flipped_x_leaving - target_bound) * need_sign > reach_tol {
                    x_leaving_now = flipped_x_leaving;
                } else {
                    stop_idx = Some(idx);
                    break;
                }
            }
            if profile_phases {
                prof_phases::BFRT_CANDS.fetch_add(n_candidates, std::sync::atomic::Ordering::Relaxed);
                prof_phases::BFRT_WALK.fetch_add(stop_idx.map(|s| s + 1).unwrap_or(sorted_prefix.len()), std::sync::atomic::Ordering::Relaxed);
            }
            let Some(stop_idx) = stop_idx else {
                // Every eligible candidate was fully flipped and the
                // leaving variable is still short of its target: no
                // column, real or flipped, can restore feasibility here
                // while keeping dual feasibility — the standard
                // dual-simplex infeasibility case.
                return SimplexResult { status: Status::Infeasible, x: None };
            };

            // Pass 2: search *backward* from `stop_idx` (never forward —
            // see below) within `HARRIS_RATIO_TOL` of its ratio for the
            // candidate with the largest pivot magnitude `|a_pj|`, and use
            // that one as the real entering variable `q` instead, trading
            // a small, bounded amount of ratio-optimality for a pivot that
            // won't make the resulting basis numerically singular. Every
            // candidate strictly before `q` in sorted order still gets
            // flipped — since pass 1 already confirmed that flipping
            // `candidates[0..stop_idx]` one at a time never overshoots
            // `target_bound`, flipping any *smaller* prefix
            // `candidates[0..best_idx]` (`best_idx <= stop_idx`) cannot
            // overshoot it either, so this is always safe: the remaining
            // gap to `target_bound` is simply covered by a slightly larger
            // `theta_q` at the real pivot below, exactly as intended.
            //
            // Searching *forward* past `stop_idx` instead (as first tried,
            // and as HiGHS's own `chooseFinalLargeAlpha` effectively does)
            // breaks this walk's invariant: those candidates were never
            // checked against `target_bound`, so flipping them could
            // overshoot it, and the resulting `theta_q` would no longer
            // correctly restore `leaving_var` to its bound — confirmed by
            // measurement, not just this argument: an earlier
            // forward-searching version of this fix eliminated the
            // crashes it targeted but produced silently wrong objective
            // values on dozens of unrelated Netlib instances that
            // previously solved correctly. HiGHS's version avoids this
            // because its `workDelta`/`totalChange` accounting is a budget
            // over the *total* remaining infeasibility rather than a
            // step-by-step "still short" walk, so overshoot is accounted
            // for correctly there; backward-only search sidesteps needing
            // that machinery.
            //
            // **A per-candidate, pivot-scaled window (`HARRIS_RATIO_TOL /
            // |a_pj|`, matching Harris's (1973) actual `r_i + tol/|alpha_i|
            // >= r_stop` formula instead of a flat ratio-space band) was
            // implemented and benchmarked, then reverted.** The idea reads
            // right in isolation — a candidate with a small pivot (the one
            // pass 2 exists to route around) earns a wide berth, a candidate
            // whose pivot is already large only qualifies near-tied — but it
            // silently breaks the safety argument two paragraphs up. That
            // argument only covers the *flip prefix* `[0, best_idx)`; every
            // candidate strictly *between* `best_idx` and `stop_idx` is left
            // neither flipped nor pivoted, relying on the ratio gap between
            // them and `q` being tiny so the resulting dual-infeasibility
            // (via the flat, unconditional `d[j] -= theta_d * a_p[j]` update
            // below, applied to every column including these) stays
            // negligible. Scaling the window by the *found* candidate's own
            // `a_pj` removes exactly that bound in the one case pass 2
            // exists for: when `stop_idx`'s own pivot is tiny, `best_abs`
            // starts tiny too, so *any* modestly-better-but-still-small
            // pivot clears `HARRIS_RATIO_TOL / abs_a` at a gap far wider
            // than a flat `1e-7` band ever allowed — measured directly on
            // the Netlib set: aggregate wall time +9.7% (73 problems),
            // entirely concentrated in the same degenerate instances this
            // file's own history already names as anti-cycling stress tests
            // (`cycle`, `grow22`, `degen3`, `perold`, `fit1p`, `bnl1`,
            // `pilot4`), zero instances newly fixed, and on `cycle`
            // specifically a silently *wrong* reported optimum (`-3.85` vs.
            // HiGHS's true `-5.226`, both reported `Status::Optimal`) — the
            // exact failure mode the forward-search attempt above was
            // reverted for, reappearing through a different door.
            //
            // **A second, independently-sourced attempt was tried and also
            // reverted: a direct port of HiGHS's real
            // `HEkkDualRow::chooseFinalLargeAlpha` (`highs/simplex/HEkkDualRow.cpp`,
            // fetched and read from source, not reconstructed from memory).**
            // Its acceptance bar isn't a ratio-distance window at all — an
            // *absolute* floor over the candidate pool, `finalCompare =
            // min(0.1 * max(|a_pj|), 1.0)`, substitution attempted only when
            // `stop_idx`'s own pivot fails it, and the *nearest* candidate
            // clearing the floor wins (not the best one in reach — the
            // mistake diagnosed in the paragraph above). This is a faithful
            // port, not a guess, and it *still* broke `cycle`: `-5.566`
            // reported `Optimal` against HiGHS's true `-5.226` (a different
            // wrong number than the previous attempt's `-3.85`, same
            // instance). A direct A/B — this exact code, with the backward
            // search short-circuited to a no-op (`best_idx = stop_idx`
            // unconditionally, i.e. no pass 2 at all) — then solved `cycle`
            // to `-5.226393024948073`, matching HiGHS to 9 significant
            // figures. That isolates the fault to *any* nontrivial
            // substitution, independent of which selection rule chooses it:
            // this file's own safety argument two paragraphs up ("flipping
            // a smaller prefix... cannot overshoot") only defends primal
            // feasibility of the *flip itself*; it says nothing about
            // whether the candidates left in the gap between the substitute
            // and `stop_idx` — neither flipped nor pivoted, yet still fed
            // through the unconditional `d[j] -= theta_d * a_p[j]` update —
            // stay dual feasible, and evidently on `cycle`'s own degeneracy
            // (109 substitutions in 1907 iterations, 5.7%, measured with
            // `ENOMOTO_DEBUG_CHUZR=1`) they sometimes don't. Real HiGHS's
            // own code has the identical gap in its default (`quad_sort`)
            // path — the one branch that *does* re-scan the gap for newly-
            // introduced dual infeasibilities lives only in its disused
            // `heap_sort` alternative — so whatever keeps HiGHS itself safe
            // here isn't in `chooseFinalLargeAlpha` alone; it's elsewhere in
            // machinery this file doesn't have (bound-shifting phase 1,
            // ratio-test tie-breaking by `workNumTotPermutation`, or its own
            // perturbation/anti-degeneracy stack). Finding and porting that
            // is a separate, deeper investigation than "implement two-pass
            // Harris".
            //
            // Confirmed via the same A/B that a *fully disabled* pass 2
            // (`best_idx = stop_idx` unconditionally, no substitution ever)
            // also solves `cycle` correctly — but shipping that outright
            // would be a real regression, not a safe default: this flat
            // `HARRIS_RATIO_TOL` window is the crate's pre-existing,
            // already-proven-on-Netlib mechanism (see this constant's own
            // docs — it exists specifically because `wood1p` reached a
            // pivot at machine-epsilon and its basis came back singular
            // without it). What both reverted attempts above changed was
            // *how far back* a substitution is allowed to reach; this flat,
            // narrow band is what has actually been measured safe across
            // the full Netlib set including `cycle` itself — narrow enough
            // that a substitution rarely fires at all, and when it does,
            // the resulting gap stays too small for the dual-infeasibility
            // mechanism above to surface. It is an empirical safety margin,
            // not a proof; widening it — by any rule — needs the same
            // Netlib A/B this comment is built from, not just a cleaner
            // formula.
            let min_ratio = sorted_prefix[stop_idx].ratio - HARRIS_RATIO_TOL;
            let mut window_start = stop_idx;
            while window_start > 0 && sorted_prefix[window_start - 1].ratio >= min_ratio {
                window_start -= 1;
            }
            let mut best_idx = stop_idx;
            let mut best_abs = sorted_prefix[stop_idx].a_pj.abs();
            for (idx, cand) in sorted_prefix.iter().enumerate().take(stop_idx + 1).skip(window_start) {
                let abs_a = cand.a_pj.abs();
                if abs_a > best_abs {
                    best_abs = abs_a;
                    best_idx = idx;
                }
            }
            if profile_phases && best_idx != stop_idx {
                prof_phases::HARRIS_SWAPS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }

            let q = sorted_prefix[best_idx].j;
            let dj_q = sorted_prefix[best_idx].dj;
            let flips: Vec<(usize, NbStatus)> =
                sorted_prefix[0..best_idx].iter().map(|cand| (cand.j, t.nb_status[cand.j].unwrap())).collect();
            (q, dj_q, flips)
        });

        // ftran-bfrt (Huangfu & Hall §2.2.3): apply every flip's combined
        // effect on the basic variables in a single extra FTRAN — `a_F`,
        // "a linear combination of the constraint columns for the
        // variables in F" — rather than one FTRAN per flipped column.
        //
        // Set by `update_verify` below, once `alpha_buf` (this block's own
        // output) is available — see that check's own docs. Declared here,
        // ahead of the `timed!` block, purely so the FTRAN-DSE cross-term
        // solve a few lines below it can skip itself on failure without
        // restructuring this block into an early-return; both reads happen
        // inside the same plain block `timed!` inlines, not a closure, so a
        // `let mut` from this outer scope is visible either way.
        let mut verify_failed = false;
        timed!(profile_phases, prof_phases::FTRAN, {
            if !flips.is_empty() {
                for &(j, old_status) in &flips {
                    let delta_x = match old_status {
                        NbStatus::Lower => std.ub[j] - std.lb[j],
                        NbStatus::Upper => -(std.ub[j] - std.lb[j]),
                        NbStatus::Zero => unreachable!("a free (`Zero`) column is never flipped"),
                    };
                    for &(i, v) in t.column_sparse(j) {
                        if !combined_touched_flag[i] {
                            combined_touched_flag[i] = true;
                            combined_touched.push(i);
                        }
                        combined_buf[i] += v * delta_x;
                    }
                }
                // A dense-enough combined-flip vector skips the
                // Gilbert-Peierls sparse path (its DFS/epoch bookkeeping
                // only pays for itself when the reach set is small
                // relative to `m` — see `should_use_dense_solve`'s own
                // docs) and goes straight through the plain dense solve
                // against `combined_buf`, which is already built above.
                if lu.should_use_dense_solve_tracked(combined_touched.len(), &density_bfrt) {
                    if profile_phases {
                        prof_phases::DENSE_RHS_BYPASSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    let result_nnz = lu.solve_into(&combined_buf, &mut sparse_lu_scratch, &mut combined_alpha_buf);
                    // Recorded on *both* branches (here and below), never
                    // only on the one the gate happens to have picked: that
                    // is what keeps the gate from latching dense forever
                    // once it has fired once.
                    density_bfrt.record(result_nnz, m);
                    // `solve_into`'s own scratch (unlike `solve_sparse_into`'s)
                    // isn't left zeroed — but `sparse_lu_scratch` is shared
                    // with the entering column's solve below, which *may*
                    // still take the sparse branch this same iteration (the
                    // dense/sparse choice is now per-call, not per-`lu`, so
                    // the two calls can disagree) and requires it zeroed on
                    // entry. Restore that invariant unconditionally here.
                    sparse_lu_scratch.fill(0.0);
                } else {
                    let combined_sparse: Vec<(usize, f64)> =
                        combined_touched.iter().map(|&i| (i, combined_buf[i])).collect();
                    let result_nnz =
                        lu.solve_sparse_into(&combined_sparse, &mut sparse_lu_scratch, &mut gp_scratch, &mut combined_alpha_buf);
                    density_bfrt.record(result_nnz, m);
                }
                #[cfg(debug_assertions)]
                {
                    let dense_combined_alpha = lu.solve(&combined_buf);
                    debug_assert_eq!(
                        combined_alpha_buf, dense_combined_alpha,
                        "sparse FTRAN (BFRT combined flip) diverged from the dense reference"
                    );
                }
                for i in 0..m {
                    let c = combined_alpha_buf[i];
                    // Gated on `c != 0.0`, not unconditional: a zero entry
                    // leaves `x_B` at row `i` untouched, so its
                    // feasibility provably cannot have changed — checking
                    // it anyway (`std.lb`/`std.ub` lookups, a comparison,
                    // a possible `InfeasibleRows` mutation) would cost
                    // real time on *every* row every iteration for a
                    // saving that only ever applies to the ones actually
                    // touched. Measured on Netlib `ganges`: doing this
                    // unconditionally made `chuzr`'s own phase faster but
                    // *increased* total wall time (the O(m) bookkeeping
                    // cost here outweighed it) — gating on nonzero, so the
                    // extra work scales with the FTRAN's own sparsity
                    // instead of `m`, is what actually pays off.
                    if c != 0.0 {
                        let var = t.basis[i];
                        t.x[var] -= c;
                        infeasible_rows.set(i, row_infeasible(std, &t, &noise_feasible, i));
                    }
                }
                for &i in &combined_touched {
                    combined_buf[i] = 0.0;
                    combined_touched_flag[i] = false;
                }
                combined_touched.clear();
                for &(j, old_status) in &flips {
                    let new_status = match old_status {
                        NbStatus::Lower => NbStatus::Upper,
                        NbStatus::Upper => NbStatus::Lower,
                        NbStatus::Zero => unreachable!("a free (`Zero`) column is never flipped"),
                    };
                    t.nb_status[j] = Some(new_status);
                    t.x[j] = match new_status {
                        NbStatus::Lower => std.lb[j],
                        NbStatus::Upper => std.ub[j],
                        NbStatus::Zero => 0.0,
                    };
                }
            }

            // Full ftran of the entering column (needed for both the primal
            // update and the DSE weight update) and ftran-dse for the weight
            // update's cross term. `theta_q` is recomputed from `t.x[leaving_var]`
            // (now reflecting every flip above) and this fresh `alpha[p]`,
            // exactly as the pre-BFRT code did — not from `x_leaving_now`/the
            // candidate's own `a_pj` — so the entering step and the state it's
            // applied to are always derived the same, numerically consistent
            // way.
            // `a_enter_buf` (dense) is still built and kept: the dense-rhs
            // bypass branch below needs it, and so does the
            // `debug_assertions` cross-check against `lu.solve`. The FTRAN
            // itself prefers the Gilbert-Peierls sparse path instead of
            // densifying-then-`solve_into` when the entering column's own
            // nonzero count is small, since a real LP's own constraint
            // columns are themselves sparse — see
            // `sparse_lu::GpScratch`/`FtLu::solve_sparse_into`'s own docs.
            // (`try_update` used to need this original column too, to
            // re-derive `a_tilde` from scratch — no longer: this same
            // FTRAN's own `a_tilde_buf` capture below now supplies it
            // directly, see `try_update_precomputed`'s own docs.)
            t.column_into(q, &mut a_enter_buf);
            // Same bypass as the BFRT combined-flip solve above, keyed off
            // this entering column's own nonzero count — `a_enter_buf`
            // (dense) is already built for `try_update`'s own use below, so
            // reusing it here costs nothing extra. Both branches also
            // capture `a_tilde_buf` (the post-L/R, pre-U intermediate) for
            // this iteration's `try_update_precomputed` call, near the end
            // of the loop — see that method's own docs.
            if lu.should_use_dense_solve_tracked(t.column_sparse(q).len(), &density_col_aq) {
                if profile_phases {
                    prof_phases::DENSE_RHS_BYPASSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                let result_nnz = lu.solve_into_capture(&a_enter_buf, &mut sparse_lu_scratch, &mut alpha_buf, &mut a_tilde_buf);
                density_col_aq.record(result_nnz, m);
                sparse_lu_scratch.fill(0.0); // restore solve_sparse_into's zero-on-entry precondition
            } else {
                let result_nnz = lu.solve_sparse_into_capture(t.column_sparse(q), &mut sparse_lu_scratch, &mut gp_scratch, &mut alpha_buf, &mut a_tilde_buf);
                density_col_aq.record(result_nnz, m);
            }
            if profile_phases {
                prof_phases::ALPHA_NNZ.fetch_add(alpha_buf.iter().filter(|&&v| v != 0.0).count(), std::sync::atomic::Ordering::Relaxed);
            }
            #[cfg(debug_assertions)]
            {
                let dense_alpha = lu.solve(&a_enter_buf);
                debug_assert_eq!(
                    alpha_buf, dense_alpha,
                    "sparse FTRAN (entering column {q}) diverged from the dense reference"
                );
            }

            // updateVerify (HiGHS `HEkkDualRow::updateVerify` equivalent —
            // see [`UPDATE_VERIFY_TOL`]/[`update_verify`]'s own docs):
            // cross-checks this pivot's element between PRICE's
            // row-direction value (`a_p[q]`, already sitting in `a_p` from
            // the PRICE step above — chuzc2 selected `q` from exactly this
            // array, so no recompute is needed) and FTRAN's own
            // column-direction value (`alpha_buf[p]`, just computed above).
            // Placed here — immediately after `alpha_buf` is final, before
            // the FTRAN-DSE cross term below and before any primal/dual/
            // pivot update reads `alpha_buf` — so a numerically drifted
            // pivot is caught at the earliest possible point, before it
            // corrupts anything downstream. Does not touch `p`/`q`/`theta_q`
            // selection at all: by this point in the iteration they are
            // already fully decided (chuzr picked `p`, chuzc2/BFRT picked
            // `q`) — this only ever decides whether this iteration commits
            // that pivot as-is, or discards it for a fresh refactorization.
            if !update_verify_disabled {
                let alpha_row = a_p[q];
                let alpha_col = alpha_buf[p];
                if debug_update_verify {
                    prof_phases::UPDATE_VERIFY_CHECKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                // HiGHS `reinvertOnNumericalTrouble`: only a factorization that has
                // been *updated* can be blamed for the disagreement — right after a
                // fresh refactorization the pivot is committed regardless (otherwise
                // a genuinely tiny pivot loops verify-fail -> refactor -> same pivot).
                if lu.update_count() > 0 && !update_verify(alpha_row, alpha_col) {
                    verify_failed = true;
                    if debug_update_verify {
                        use std::sync::atomic::Ordering::Relaxed;
                        let scale = alpha_row.abs().max(alpha_col.abs()).max(FT_MIN_PIVOT);
                        let rel_ppm = ((alpha_row - alpha_col).abs() / scale * 1_000_000.0) as usize;
                        prof_phases::UPDATE_VERIFY_TRIGGERS.fetch_add(1, Relaxed);
                        prof_phases::UPDATE_VERIFY_SUM_REL_PPM.fetch_add(rel_ppm, Relaxed);
                        prof_phases::UPDATE_VERIFY_MAX_REL_PPM.fetch_max(rel_ppm, Relaxed);
                    }
                }
            }

            // The ftran-dse cross term (`tau`) is only ever read by
            // `DseState::update_after_pivot`, further down — skip the
            // solve entirely while pricing is still in cheap Devex mode
            // (see [`EdgeWeights`]'s own docs for why this is the whole
            // point of starting there), and also when `updateVerify` just
            // failed above: this pivot is about to be discarded and
            // refactorized around, so its DSE cross term would only be
            // thrown away unread. `tau_buf` is left however a prior
            // iteration's solve happened to leave it in either skip case,
            // but that's fine: nothing reads it unless `weights` is `Dse`
            // *and* this iteration's pivot actually commits, which only
            // happens on an iteration whose FTRAN block (this one) both
            // populated it fresh and passed `updateVerify`.
            if !verify_failed && matches!(weights, EdgeWeights::Dse(_)) {
                lu.solve_into(rho_p, &mut lu_scratch, &mut tau_buf);
            }
        });

        // `updateVerify` failed above: this pivot's own basis
        // factorization can no longer be trusted (see [`update_verify`]'s
        // own docs), so discard it — without ever touching `t.basis`,
        // `t.nb_status`, `x_B`, or `d` (none of the primal/dual/pivot
        // updates below have run yet for this pivot; only the BFRT bound
        // flips earlier this same iteration, if any, are committed, and
        // those are independent, already-valid degenerate steps — see this
        // block's own comment above `verify_failed`'s declaration) —
        // refactorize in place and let the `for _iter` loop's next pass
        // re-run `chuzr`/`chuzc2` fresh against the rebuilt factorization,
        // exactly like every other mid-loop refactorization trigger in this
        // function.
        if verify_failed {
            timed!(profile_phases, prof_phases::REFACTOR, {
                if profile_phases {
                    prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                let Some(l) = try_refactorize(std, &t, Some(&lu)) else {
                    if debug_devex {
                        eprintln!("FALLBACK@update_verify iter={_iter}");
                    }
                    return solve_lp_on(std);
                };
                lu = l;
                let rhs = t.compute_rhs();
                t.resync_basics(&lu, &rhs);
                d = fresh_d(&lu, &t, &active_cost);
                infeasible_rows.rebuild(m, |i| row_infeasible(std, &t, &noise_feasible, i));
            });
            // Same touched-entries-only reset the normal end-of-iteration
            // path uses below — `a_p`/`touched`/`touched_cols` must go back
            // to all-zero/empty before the next iteration's PRICE step,
            // which only ever adds into them (see that loop's own docs),
            // regardless of whether this iteration's own pivot committed.
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            continue;
        }

        let alpha = &alpha_buf;

        let theta_q = (t.x[leaving_var] - target_bound) / alpha[p];

        // Stall detection for the Bland's-rule fallback (see `bland_mode`'s
        // own docs above): `theta_q * dj_q` is this pivot's actual
        // contribution to the (dual) objective, so a run of consecutive
        // pivots that all contribute essentially nothing is exactly
        // "degenerate pivots with no real progress" — textbook cycling —
        // regardless of how large `theta_q` itself is (a big step that
        // barely improves the objective is just as stalled as a tiny one).
        if (theta_q * dj_q).abs() < STALL_PROGRESS_EPS {
            stall_count += 1;
            if stall_count > stall_limit {
                bland_mode = true;
            }
        } else {
            stall_count = 0;
        }

        // Ill-conditioning trigger (see [`DEVEX_ILLCOND_PIVOT_TOL`]'s own
        // docs): a single pivot this small, while still in Devex mode, is
        // itself sufficient evidence that this whole problem - not just
        // this one pivot - is a poor fit for Devex's cheap approximation
        // (confirmed on Netlib `cycle`/`forplan`/`25fv47`: each hits this
        // at most once, always fairly early).
        //
        // This used to `return solve_lp_dual_on(std, true)`, a full cold
        // restart that threw away every local this function had built
        // (`t`, `lu`, `d`, `infeasible_rows`, ...) and re-solved from the
        // all-slack basis. That is now an **in-place switch**: the current
        // basis and all incrementally-maintained state are kept, and only
        // `weights` changes - to exact DSE weights for the *pre-pivot*
        // basis ([`DseState::from_basis`]), so this very iteration's own
        // weight-update step below (`dse.update_after_pivot`, which
        // expects pre-pivot weights) correctly advances them to the
        // post-pivot basis. The accumulated Devex work is therefore not
        // discarded, and the restart's own re-solve cost (`forplan`: 221
        // DSE + 245 Devex pivots thrown away) is not paid. Only the one
        // pivot already fully computed this iteration (BFRT flips already
        // applied to `t.x`, `q`/`alpha`/`theta_q` already formed) is
        // committed under Devex - switching any earlier would have to
        // undo committed flips, and `FT_MIN_PIVOT`-level pivots are
        // rejected by `try_update` regardless, so accepting this one
        // usable pivot and pricing every later one with DSE is the safe
        // reading of "resume from here".
        if matches!(weights, EdgeWeights::Devex(_)) && alpha[p].abs() < DEVEX_ILLCOND_PIVOT_TOL {
            if debug_devex {
                eprintln!("DEVEX->DSE in-place switch at iter {_iter} (|alpha_p|={:.3e})", alpha[p].abs());
            }
            let dse = DseState::from_basis(m, &lu);
            // `tau` (ftran-dse) was skipped in the FTRAN block above
            // because pricing was still Devex then, so solve it now for
            // the DSE weight update below. `rho_p` and `lu` are both still
            // for the pre-pivot basis - exactly what this update needs.
            lu.solve_into(rho_p, &mut lu_scratch, &mut tau_buf);
            weights = EdgeWeights::Dse(dse);
        }

        // Devex→DSE escalation (see [`DEVEX_STAGNATION_WINDOW`]'s own
        // docs): the rolling-average stagnation trigger alone now (the
        // single-pivot ill-conditioning case above already restarts this
        // solve outright) — watches a real stretch of iterations against
        // the cost data's own scale rather than any single pivot. This one
        // still escalates *in place* (no restart): a solve reaching this
        // point has already made a long run of ordinary, non-catastrophic
        // progress under Devex, so there is real accumulated work worth
        // keeping, not a doomed trajectory worth discarding. Only tracked
        // while still in Devex mode: once escalated, `weights` never
        // returns to `Devex` this solve (see [`EdgeWeights`]'s docs), so
        // there is nothing left to watch for.
        let mut escalate_to_dse = false;
        if matches!(weights, EdgeWeights::Devex(_)) {
            let contribution = (theta_q * dj_q).abs();
            devex_window.push_back(contribution);
            devex_window_sum += contribution;
            if devex_window.len() > DEVEX_STAGNATION_WINDOW {
                devex_window_sum -= devex_window.pop_front().unwrap();
            }
            if devex_window.len() == DEVEX_STAGNATION_WINDOW
                && devex_window_sum / (DEVEX_STAGNATION_WINDOW as f64) < DEVEX_STAGNATION_REL_TOL * obj_scale
            {
                escalate_to_dse = true;
            }
        }

        for i in 0..m {
            let a = alpha[i];
            // Gated on `a != 0.0` — see the identical reasoning on the
            // BFRT combined-flip loop above (a zero entry means `x_B` at
            // row `i` doesn't move this pivot, so its feasibility can't
            // have changed either). `alpha[p]` is the pivot element itself
            // and so is always nonzero, so row `p` — `var` here is still
            // `leaving_var`, pre-swap — always takes this branch; it lands
            // exactly on `target_bound` by construction (feasible), and
            // its entry gets overwritten again below once row `p`'s basic
            // variable actually becomes `q`.
            if a != 0.0 {
                let var = t.basis[i];
                t.x[var] -= a * theta_q;
                infeasible_rows.set(i, row_infeasible(std, &t, &noise_feasible, i));
            }
        }
        t.x[q] += theta_q;

        timed!(profile_phases, prof_phases::DSE_UPDATE, match &mut weights {
            // Whichever scheme was active *this* iteration is the one that
            // must update now — `tau` was only actually solved for above
            // when `weights` was already `Dse` at that point, so updating
            // by the (possibly just-decided) `escalate_to_dse` flag instead
            // would read a stale `tau_buf` from a stale iteration.
            EdgeWeights::Devex(dv) => dv.update_after_pivot(p, alpha),
            EdgeWeights::Dse(dse) => dse.update_after_pivot(p, alpha, &tau_buf, rho_p),
        });
        // Applied only now, after this pivot's own weight update above has
        // run under the scheme that was actually live for it - takes
        // effect starting next iteration's FTRAN block (which decides
        // whether to pay for the `tau` solve from `weights`'s variant at
        // that point). The actual `DseState` is built at the very end of
        // this iteration, *after* the basis swap and `lu.try_update`
        // below, so its weights are exact for the post-pivot basis (see
        // [`DseState::from_basis`]); this branch only defers that.
        if escalate_to_dse && debug_devex {
            eprintln!("DEVEX->DSE escalation at iter {_iter}");
        }

        t.nb_status[leaving_var] = Some(if leaving_infeasible_low { NbStatus::Lower } else { NbStatus::Upper });
        t.basis_pos[leaving_var] = None;
        t.basis[p] = q;
        t.basis_pos[q] = Some(p);
        t.nb_status[q] = None;
        // Row `p`'s basic variable just changed identity (`leaving_var` ->
        // `q`), so it needs one more membership check now that `t.basis[p]`
        // reflects that.
        infeasible_rows.set(p, row_infeasible(std, &t, &noise_feasible, p));

        // Update-dual (Huangfu & Hall §2.2.3 — same formula HiGHS's
        // `HEkkDualRow::updateDual` applies straight to its `workDual`
        // array): `d'[j] = d[j] - theta_d * a_p[j]` for *every* j, where
        // `theta_d = dj_q / alpha[p]`. Derived from scratch (in terms of
        // `y` first, then simplified) via Sherman-Morrison on
        // `B' = B(I + (alpha - e_p) e_p^T)` — the same rank-1 update
        // `FtLu`'s own eta updates use; the cross terms from `c_B`'s own
        // change at position `p` (from `std.c[leaving_var]` to
        // `std.c[q]`) cancel out entirely, leaving only this one term.
        // Applying it to *every* column (not just this iteration's
        // candidates) is what keeps `d` correct for columns chuzc1 will
        // look at on a *future* iteration; it's a cheap, allocation-free
        // pass since `a_p` is already fully materialized above (and, like
        // every other loop this size in this function, sequential rather
        // than parallel — see the chuzr comment). Bound flips never touch
        // this: `d` depends only on the
        // basis matrix and `c_B`, neither of which a flip changes, so it
        // needs updating only once per iteration, right here, using the
        // *real* pivot's own `dj_q`/`alpha[p]` — never the candidates
        // that only got flipped.
        let theta_d = dj_q / alpha[p];
        timed!(profile_phases, prof_phases::DUAL_UPDATE, {
            for &j in &touched_cols {
                d[j] -= theta_d * a_p[j];
            }
        });

        // Reset `a_p` back to all-zero for the next iteration's PRICE —
        // only at the entries this iteration actually touched, not a full
        // `O(n_total)` sweep (see `a_p`'s own declaration comment above).
        for &j in &touched_cols {
            a_p[j] = 0.0;
            touched[j] = false;
        }
        touched_cols.clear();
        #[cfg(debug_assertions)]
        {
            // `lu` here is still the *old* basis's factorization (any
            // update/refactorization happens below, after this check), so
            // verifying against it directly would compare `d_new` against
            // reduced costs for `B_old` — a meaningless mismatch, not a
            // check of the update formula. A genuinely fresh factorization
            // of the (already basis-swapped) `t` is needed instead.
            let fresh_lu = refactorize(std, &t, None);
            let d_fresh = fresh_d(&fresh_lu, &t, &active_cost);
            for j in 0..std.n_total {
                // Fixed columns are deliberately excluded from PRICE's
                // `touched_cols` (see that loop's own comment) and so never
                // receive an incremental update here — comparing their
                // stale `d[j]` against a fresh recompute would be checking
                // an invariant this function no longer maintains, not a
                // real divergence.
                if std.lb[j] == std.ub[j] {
                    continue;
                }
                assert!(
                    // Loosened from 1e-6 once cost perturbation (`perturb_costs`)
                    // started shifting every cost by a small nonzero amount:
                    // marginally different rounding/cancellation patterns in the
                    // incremental update pushed this crate's own larger dual
                    // cross-check test slightly past the old threshold (~1.4x
                    // over) with no other symptom of a real formula error (which
                    // would show gross, not marginal, divergence) — this is a
                    // debug-only sanity check, not a correctness guarantee the
                    // release build depends on, so a bit more slack here is a
                    // reasonable trade against false alarms.
                    (d[j] - d_fresh[j]).abs() < 1e-5 * d_fresh[j].abs().max(1.0),
                    "incremental reduced-cost update diverged from a fresh recompute: j={j} incremental={} fresh={}",
                    d[j],
                    d_fresh[j]
                );
            }
        }

        let update_ok = timed!(
            profile_phases,
            prof_phases::FT_UPDATE,
            lu.try_update_precomputed(p, &a_tilde_buf, &e_tilde_buf, FT_MIN_PIVOT)
        );
        if update_ok && debug_eta_density {
            let denom = std.n_rows.saturating_sub(1).max(1);
            let frac = lu.last_update_off_diag_len() as f64 / denom as f64;
            let bin = ((frac * 20.0) as usize).min(19);
            prof_phases::ETA_DENSITY_BINS.lock().unwrap()[bin] += 1;
            prof_phases::ETA_DENSITY_SAMPLES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            prof_phases::ETA_DENSITY_SUM_PPM.fetch_add((frac * 1_000_000.0) as usize, std::sync::atomic::Ordering::Relaxed);
        }
        if !update_ok {
            timed!(profile_phases, prof_phases::REFACTOR, {
                if profile_phases {
                    prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                let Some(l) = try_refactorize(std, &t, Some(&lu)) else {
                    if debug_devex {
                        eprintln!("FALLBACK@ft_update iter={_iter}");
                    }
                    return solve_lp_on(std);
                };
                lu = l;
                let rhs = t.compute_rhs();
                t.resync_basics(&lu, &rhs);
                d = fresh_d(&lu, &t, &active_cost);
                infeasible_rows.rebuild(m, |i| row_infeasible(std, &t, &noise_feasible, i));
            });
        }

        // Devex->DSE stagnation escalation (see the `escalate_to_dse` flag
        // above): `lu` now matches the post-pivot basis, so build the DSE
        // weights exactly for *that* basis rather than the all-slack
        // `DseState::new`'s `w[i] = 1` the old code used here - the cheap
        // initialization silently reset every weight to its starting value
        // even though the solve had already accumulated real progress.
        if escalate_to_dse {
            weights = EdgeWeights::Dse(DseState::from_basis(m, &lu));
        }
    }

    SimplexResult { status: Status::Optimal, x: Some(t.x[0..t.n_orig()].to_vec()) } // iteration cap; best-effort
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
                *w_i = (*w_i - 2.0 * ratio * tau[i] + ratio * ratio * wp_old).max(STEEPEST_EDGE_FLOOR);
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

        let pf = build_std_form_presolved(&vars, &obj, &cons, false, true).unwrap();
        assert!(!pf.had_unbounded_structural, "x0 should be fully eliminated by presolve, never reaching a structural column at all");

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

    #[test]
    fn had_unbounded_structural_flag_reflects_surviving_one_sided_bounds() {
        // Fully bounded problem (no structural column ever infinite): the
        // flag stays false — this is the common case, and the one every
        // existing Netlib-style problem takes.
        let vars = vec![var(0.0, 10.0), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, 1.0), (1, 1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(0, 1.0), (1, 1.0)], RowSense::Le, 10.0)];
        let pf = build_std_form_presolved(&vars, &obj, &cons, true, true).unwrap();
        assert!(!pf.had_unbounded_structural);

        // A one-sided-unbounded structural variable whose favored
        // direction (its cost sign) points at its own infinite bound, with
        // no row of its own for `dualfix`/`propagate` to tighten that
        // bound from: it survives presolve with a genuine infinite `ub`,
        // so the flag is set regardless of `clamp_unbounded` — `x0` isn't
        // referenced by `cons` at all here, only `x1` is. With
        // `clamp_unbounded: false`, `std.ub[0]` itself stays genuinely
        // infinite (what `solve_lp_dual` passes, for
        // `extended_dual::solve_lp_dual_extended` to consume).
        let vars = vec![var(0.0, f64::INFINITY), var(0.0, 10.0)];
        let obj = Objective { expr: expr(&[(0, -1.0)]), sense: Sense::Minimize };
        let cons = vec![row(&[(1, 1.0)], RowSense::Le, 5.0)];
        let pf = build_std_form_presolved(&vars, &obj, &cons, true, true).unwrap();
        assert!(pf.had_unbounded_structural);
        assert!(pf.std.ub[0].is_finite(), "clamp_unbounded:true should still BIG_M-substitute, ub={}", pf.std.ub[0]);

        let pf = build_std_form_presolved(&vars, &obj, &cons, false, true).unwrap();
        assert!(pf.had_unbounded_structural);
        assert_eq!(pf.std.ub[0], f64::INFINITY, "clamp_unbounded:false must leave the true infinity in place");
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
