//! Extended dual simplex with BFRT, for `StdForm`s where some *structural*
//! column still carries a genuine one-sided infinite bound (Stage 3 of the
//! paper's algorithm — see the plan this implements). Dispatched to by
//! `solve_lp_dual` exactly when `PresolvedForm::had_unbounded_structural`
//! is set; every other call path (the overwhelming majority of problems)
//! never reaches this module at all and keeps using the classical
//! bounded-variable [`super::solve_lp_dual_on`] unchanged.
//!
//! ## The M→∞ symbolic trick, and why no number ever stands for `M`
//!
//! The paper this implements replaces a fixed numeric truncation
//! (`+/-M` substituted for `+/-inf`, this crate's own former `BIG_M`
//! sentinel — see `simplex.rs::BIG_M`'s own docs for the numerical
//! fragility that motivated dropping it here) with a *symbolic* analysis:
//! every quantity that would depend on the truncation value is tracked as
//! an affine function `base + slope * M` ([`Affine1`]) instead of a plain
//! number, and every comparison between two such quantities is decided by
//! comparing `(slope, base)` lexicographically (`Affine1::cmp_lex`) rather
//! than by ever plugging in a concrete `M`. Since the *sign* of a
//! sufficiently-large-`M` comparison is exactly decided by the
//! leading (slope) term — falling back to the base term only when slopes
//! tie exactly — this reproduces precisely what running the classical
//! algorithm with an arbitrarily large numeric `M` would decide, without
//! ever risking the numerical fragility (or the "how large is large
//! enough?" judgment call) a concrete `M` would introduce.
//!
//! `M`'s only two jobs, per the paper: (1) let every nonbasic *structural*
//! column be placed at *some* bound (needed so the all-slack initial basis
//! is dual-feasible purely from the sign of `c`, with no phase 1 —
//! [`crash`]) even when one side is a genuine infinity, and (2) let the
//! bound-flipping ratio test flip through such a column instead of
//! stopping dead at it. Both become well-defined once every affected
//! quantity is [`Affine1`] instead of `f64`.
//!
//! The paper's revised §4.2 restricts `M`-tracking to the minimal set `S`
//! (a one-sided-unbounded column is `M`-tracked only if its cost sign
//! *forces* `crash` to place it on the genuinely infinite side; every
//! other one-sided-unbounded column's infinite side would instead be
//! represented as a real, unreachable infinity, `None`, never `M`-tracked
//! at all — see `[[extended-dual-s-restriction]]` memory for the design).
//! **This module currently does *not* do that restriction** — [`delta_of`]
//! flags every one-sided-unbounded column, matching this module's original
//! (pre-`S`-restriction) behavior. The `S`-restricted version was
//! implemented and is mathematically correct (matched HiGHS on the full
//! Netlib 73-problem set), but was reverted after measuring it as a net
//! *regression* (7.9s -> 9.6s total): excluding a column from `S` also
//! excludes it from `width_affine`'s BFRT-flip-eligibility, so several
//! degenerate instances (`pilot4`, `fit1p`, `degen2`, `degen3`) needed
//! substantially more real pivots in place of the cheap flips the
//! unrestricted flagging used to allow. See [`delta_of`]'s own docs for how
//! to restore the `S`-restricted form.
//!
//! ## Preconditions this module relies on (established by earlier stages)
//!
//! - A structural column *may* now be genuinely *free* (`lb == -inf` **and**
//!   `ub == +inf`) — `presolve::freevar::eliminate_free_variables` still
//!   removes every one reachable through `A`'s own rows before this module
//!   ever runs, but that module's own docs name a residual case it cannot
//!   soundly resolve itself (a free variable that only appears in an
//!   inequality row); `simplex.rs::build_std_form_presolved` no longer
//!   splits that residual case into `x_j = x_j^+ - x_j^-` before building
//!   the `StdForm` this module receives — [`delta_of`]/[`hat_lower`]/
//!   [`hat_upper`] track *both* sides of such a column independently (see
//!   their own docs) instead. The one place this module still cannot
//!   soundly handle a free column is the cleanup phase evicting *another*
//!   genuinely free basic variable to make room for one (`finish`'s own
//!   docs) — that specific, doubly-rare case bails out to `None` (the
//!   classical `BIG_M` fallback) rather than risk an unproven termination
//!   argument, exactly like this module's other "should be unreachable"
//!   guards.
//! - Every one-sided-unbounded structural column has already been shifted
//!   (`simplex.rs::build_std_form_presolved`'s own shift step, unchanged
//!   for this path) so its *finite* side sits at exactly `0` — this module
//!   leans on that fact directly: [`hat_lower`]/[`hat_upper`] never need a
//!   variable's own `w_j` term (the paper's general
//!   `hat_u_j(M)-hat_l_j(M) = w_j + s_j*M` collapses to exactly `s_j*M`
//!   here, `w_j == 0` always).
//! - No slack column (a `<=`/`>=` row's own, `[0, inf)`) is ever
//!   M-flagged: it is one-sided by construction, not a truncated free/
//!   one-sided *structural* variable, and (per `simplex.rs`'s own module
//!   docs) the classical method already handles a slack's genuine
//!   infinity correctly by simply never placing it there — this module
//!   preserves that behavior unchanged (see [`hat_upper`]'s own docs).
//!
//! ## Deliberate simplifications versus the classical method
//!
//! This started as a first, from-scratch implementation of the paper's
//! algorithm, not a symbolic-`M` retrofit of [`super::solve_lp_dual_on`]'s
//! far more elaborate machinery — but two of the three simplifications
//! originally listed here have since been ported over (see each item
//! below); only the third remains as originally written. Kept as a record
//! of what was deliberately deferred versus what has since caught up, not
//! as a current-state summary — read [`Score2`]'s and [`try_update`]'s own
//! call sites for what actually runs today.
//!
//! 1. ~~Dantzig's rule only for the leaving row~~ — **superseded**: the main
//!    loop now uses the same `super::EdgeWeights::Dse`/`DseState` this
//!    module shares with [`super::solve_lp_dual_on`], generalized to
//!    [`Score2`]'s lexicographic degree-2-in-`M` comparison exactly as the
//!    paper's §4.5 describes (`weights.weight(i)` feeding every [`Score2`]
//!    built in the main loop below). Ported once `x_B(M)`'s incremental
//!    maintenance (item 2 below) made the leaving row's true deviation
//!    available every iteration instead of only after a full recompute.
//! 2. ~~No incremental basis update~~ — **superseded**: the main loop calls
//!    `FtLu::try_update` (Forrest-Tomlin eta update) every pivot, exactly
//!    like [`super::solve_lp_dual_on`], and only falls back to a full
//!    [`refactorize`] on `try_update`'s own rejection, the eta-bump-count
//!    cap, or this module's own tightened drift check ([`XB_CHECK_INTERVAL`]/
//!    [`XB_DRIFT_TOL`]) — typically single digits per solve, not once per
//!    iteration (see `ENOMOTO_PROF_PHASES_EXT`'s own `refactor_count`
//!    breakdown).
//! 3. **Anti-cycling by a raw iteration-count stall counter** (still as
//!    originally implemented), not the paper's own full `(M, epsilon)`-
//!    extended lexicographic rule (§4.7): once `stall_limit` iterations
//!    pass, tie-breaking permanently switches to smallest-index-first
//!    (`bland_mode`), matching this crate's existing convention for the
//!    classical dual method's own Bland fallback (see
//!    `simplex.rs::solve_lp_dual_on`'s own `bland_mode`).

use super::{sparse_lu, InfeasibleRows, NbStatus, SimplexResult, StdForm, Status, TOL};
use crate::sparse::sparse_axpy_dense;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::OnceLock;

/// Per-phase wall-clock counters for the `ENOMOTO_PROF_PHASES_EXT`
/// diagnostic — this module's own counterpart to `simplex::prof_phases`
/// (see that module's own docs), answering the same "where does time in
/// the loop actually go" question for [`solve_lp_dual_extended`]'s own
/// main loop instead of the classical `solve_lp_dual_on`'s. Kept as a
/// separate module (and a separate env var) rather than reusing
/// `super::prof_phases` directly: that module's statics are private to
/// `simplex.rs`, and the two loops' phase boundaries don't line up
/// one-to-one anyway (this loop has no separate primal-update pass — the
/// combined-flip/entering-column steps fold that into `XB_UPDATE` below,
/// and there is no analogue of the classical method's own `COMPUTE_RHS_COLS_*`
/// skip-counting since `compute_rhs_affine` only ever runs at a resync,
/// not once per iteration).
mod prof_phases {
    use std::sync::atomic::AtomicUsize;
    pub(super) static BTRAN: AtomicUsize = AtomicUsize::new(0);
    /// FTRANs this solve dispatched to the dense path *because of the
    /// result-density gate alone* — i.e. ones whose own right-hand side
    /// was sparse enough that `FtLu::should_use_dense_solve`'s input test
    /// (the only test this crate had before `sparse_lu::FtranDensity`)
    /// would have sent them down the Gilbert-Peierls path. This is the
    /// whole population the §2.7 change moves, so it is the number to
    /// look at first when a problem's wall time shifts:
    /// `density_gate_ftrans=0` means the gate never fired there and any
    /// timing difference is noise or the nonzero-counting overhead alone.
    pub(super) static DENSITY_GATE_FTRANS: AtomicUsize = AtomicUsize::new(0);
    /// Each tracked channel's own running-average result density at the
    /// last iteration that updated it, in per-mille of `m` (an
    /// `AtomicUsize` because this module's counters are all atomics; the
    /// value is a density, not a count).
    pub(super) static DENSITY_COL_AQ_PPT: AtomicUsize = AtomicUsize::new(0);
    pub(super) static DENSITY_BFRT_PPT: AtomicUsize = AtomicUsize::new(0);
    pub(super) static PRICE: AtomicUsize = AtomicUsize::new(0);
    pub(super) static CHUZR: AtomicUsize = AtomicUsize::new(0);
    pub(super) static CHUZC1: AtomicUsize = AtomicUsize::new(0);
    pub(super) static BFRT: AtomicUsize = AtomicUsize::new(0);
    pub(super) static FTRAN: AtomicUsize = AtomicUsize::new(0);
    pub(super) static XB_UPDATE: AtomicUsize = AtomicUsize::new(0);
    pub(super) static DSE_UPDATE: AtomicUsize = AtomicUsize::new(0);
    pub(super) static DUAL_UPDATE: AtomicUsize = AtomicUsize::new(0);
    pub(super) static FT_UPDATE: AtomicUsize = AtomicUsize::new(0);
    pub(super) static REFACTOR: AtomicUsize = AtomicUsize::new(0);
    pub(super) static REFACTOR_COUNT: AtomicUsize = AtomicUsize::new(0);
    /// Breaks `REFACTOR_COUNT` down by *why* each refactorization fired —
    /// added to answer "why does `25fv47` refactorize 97 times when most
    /// problems refactorize single digits" (see `report`'s own printout):
    /// `VERIFY` is `super::update_verify`'s own cross-check rejecting a
    /// pivot outright (the earliest, cheapest-to-avoid trigger — this
    /// pivot's numbers disagreed enough between PRICE and FTRAN that it
    /// was never committed at all); `TRY_UPDATE` is `FtLu::try_update`
    /// itself refusing a *committed* pivot for too small a resulting
    /// diagonal (`FT_MIN_PIVOT`); `BUMP` is the eta-file-size cap
    /// (`FT_BUMP_LIMIT_FACTOR`); `DRIFT` is this module's own tightened
    /// `XB_DRIFT_TOL`/`XB_CHECK_INTERVAL` residual check (see that
    /// constant's own docs — introduced to fix the `maros` regression).
    pub(super) static REFACTOR_CAUSE_VERIFY: AtomicUsize = AtomicUsize::new(0);
    pub(super) static REFACTOR_CAUSE_TRY_UPDATE: AtomicUsize = AtomicUsize::new(0);
    pub(super) static REFACTOR_CAUSE_BUMP: AtomicUsize = AtomicUsize::new(0);
    pub(super) static REFACTOR_CAUSE_DRIFT: AtomicUsize = AtomicUsize::new(0);
    /// `d`'s own independent drift check ([`super::D_DRIFT_TOL`]'s own
    /// docs) firing — distinct from `DRIFT` above, which only ever checks
    /// `x_B(M)`.
    pub(super) static REFACTOR_CAUSE_D_DRIFT: AtomicUsize = AtomicUsize::new(0);
    /// `pivot_grossly_inconsistent` (its own call site's docs) firing —
    /// distinct from `VERIFY` above (the ordinary, tight-tolerance
    /// `update_verify` check, gated by `lu.update_count() > 0`): this one
    /// runs regardless of `update_count` but only ever fires on a
    /// qualitatively worse PRICE/FTRAN disagreement than `VERIFY` screens
    /// for.
    pub(super) static REFACTOR_CAUSE_ILLCOND: AtomicUsize = AtomicUsize::new(0);
    /// Trigger (4) (`super::extended_dual::ft_max_updates`) firing — see
    /// that function's own docs. Expected to stay at `0` on every real
    /// instance (it is sized as a rarely-firing safety net, not a routine
    /// lever); nonzero here is itself the signal to re-tune
    /// `FT_MAX_UPDATES_FACTOR`.
    pub(super) static REFACTOR_CAUSE_MAX_UPDATES: AtomicUsize = AtomicUsize::new(0);
    /// A would-be `Infeasible` conclusion refusing to be drawn from an
    /// updated factorization (both `Status::Infeasible` sites' own docs):
    /// the iteration is redone from a fresh one instead. Nonzero here
    /// means this guard actually saved (or at least delayed) an
    /// infeasibility report — on `greenbea` it fires exactly once.
    pub(super) static REFACTOR_CAUSE_INFEAS_CHECK: AtomicUsize = AtomicUsize::new(0);
    /// Trigger (5) ([`super::extended_dual::SYNTH_CLOCK_FACTOR`]'s own
    /// docs) firing — the deterministic operation-count ("synthetic tick")
    /// cost-based trigger, this module's counterpart to HiGHS's own
    /// `kRebuildReasonSyntheticClockSaysInvert`
    /// (`analysis/ft_refactor_trigger_20260922_040850.md` §5/§6). Distinct
    /// from `MAX_UPDATES` above (a fixed `update_count` cap regardless of
    /// how cheap or expensive each individual update's own solves were):
    /// this one refactorizes once the *measured* solve-side work done
    /// against the current factorization is estimated to already cost as
    /// much as re-factorizing it outright would, the actual gap this
    /// trigger's own analysis found between this crate's eta-chain length
    /// (300-2,700 updates before refactorizing) and HiGHS's own
    /// cost-triggered interval (50-140 updates).
    pub(super) static REFACTOR_CAUSE_CLOCK: AtomicUsize = AtomicUsize::new(0);
    /// Peak `lu.update_count()` observed *at any point* during the solve
    /// (via `fetch_max`, so this is the true peak across every
    /// refactorization interval, not just the value at solve end) — a
    /// calibration gauge for [`super::extended_dual::FT_MAX_UPDATES_FACTOR`]
    /// itself, printed by `ENOMOTO_PROF_PHASES_EXT` but never consulted by
    /// any control-flow decision.
    pub(super) static MAX_UPDATE_STREAK: AtomicUsize = AtomicUsize::new(0);
    pub(super) static ITERS: AtomicUsize = AtomicUsize::new(0);
    /// Diagnostic only (`ENOMOTO_PROF_PHASES_EXT`'s own report): sum of
    /// `best_idx` (candidates flipped before the real pivot) across every
    /// iteration, and how many rows were in `infeasible_rows` at the start
    /// of chuzr each iteration — together give "average BFRT batch size"
    /// and "average infeasible-row pool size" without needing a separate
    /// `#[cfg(test)]`-gated counter.
    pub(super) static BFRT_FLIPS: AtomicUsize = AtomicUsize::new(0);
    pub(super) static INFEASIBLE_POOL: AtomicUsize = AtomicUsize::new(0);
    /// M-exit-acceleration investigation (`degen3` bottleneck follow-up):
    /// how often the entering column `q` was itself sitting on its `M`
    /// side pre-pivot (the *only* way an `M`-flagged column ever leaves
    /// it), how often a Harris pass-2 window contained an `M`-side
    /// candidate at some index other than the one actually chosen (a
    /// missed opportunity for a same-cost tie-break to prefer draining
    /// `M`), and how often a BFRT flip moved a column *onto* its `M`
    /// side (the mechanism that can make the `M` population grow instead
    /// of only ever shrink).
    pub(super) static M_EXIT: AtomicUsize = AtomicUsize::new(0);
    pub(super) static HARRIS_WINDOW_M_MISS: AtomicUsize = AtomicUsize::new(0);
    pub(super) static M_ENTER_VIA_FLIP: AtomicUsize = AtomicUsize::new(0);
    /// Generic-degeneracy follow-up (once `M`-exit itself measured as a
    /// minor slice of `degen3`'s iteration count): `DEGENERATE_PIVOTS`
    /// counts iterations whose entering column `q` had (already, before
    /// this pivot) an essentially-zero reduced cost — a pivot that changes
    /// basis composition without moving the objective at all, the
    /// standard definition of a degenerate step. `HARRIS_WINDOW_SIZE_SUM`
    /// (divided by `ITERS` for the average) measures real tie contention
    /// in the Harris pass-2 window regardless of `M`, i.e. how often
    /// *any* two candidates are close enough in ratio to matter for the
    /// tie-break, not just `M`-side ones (`HARRIS_WINDOW_M_MISS` above).
    pub(super) static DEGENERATE_PIVOTS: AtomicUsize = AtomicUsize::new(0);
    pub(super) static HARRIS_WINDOW_SIZE_SUM: AtomicUsize = AtomicUsize::new(0);
    /// "Ripple" follow-up: does fixing one infeasible row typically leave
    /// the *pool* of infeasible rows smaller, the same size, or larger
    /// than it was at the start of the previous iteration? Compared
    /// iteration-to-iteration against `INFEASIBLE_POOL`'s own per-iteration
    /// snapshot, not within a single iteration (a single dual pivot always
    /// drives its *own* chosen row `r` to exact feasibility — what this
    /// counts is whether the resulting `x_B` shift pushed *other* rows
    /// into infeasibility faster than rows get fixed, one iteration to the
    /// next).
    pub(super) static POOL_GREW: AtomicUsize = AtomicUsize::new(0);
    pub(super) static POOL_SHRANK: AtomicUsize = AtomicUsize::new(0);
    pub(super) static POOL_SAME: AtomicUsize = AtomicUsize::new(0);
    /// DSE weight quality follow-up: `rho` (this iteration's own BTRAN,
    /// `B^-1 e_r`) is *exactly* `super::DseState`'s own target quantity
    /// (`gamma_r = ||B^-1 e_r||^2`) for the chosen row `r` — so comparing
    /// `dot(rho, rho)` against the incrementally-maintained
    /// `weights.weight(r)` (read just before it, still pre-update) costs
    /// nothing extra (no additional linear solve) and answers whether
    /// `chuzr`'s notion of "how disruptive is pivoting on this row" has
    /// drifted from the true value by the time it actually gets picked —
    /// bucketed by relative error since summing an unbounded ratio in a
    /// plain `AtomicUsize` isn't meaningful.
    pub(super) static DSE_CHECKED: AtomicUsize = AtomicUsize::new(0);
    pub(super) static DSE_ERR_LT_1PCT: AtomicUsize = AtomicUsize::new(0);
    pub(super) static DSE_ERR_LT_10PCT: AtomicUsize = AtomicUsize::new(0);
    pub(super) static DSE_ERR_LT_100PCT: AtomicUsize = AtomicUsize::new(0);
    pub(super) static DSE_ERR_GE_100PCT: AtomicUsize = AtomicUsize::new(0);
    /// Per-iteration work-volume counters for the price/chuzc1/DSE
    /// phases (summed over iterations; the report divides by `ITERS`),
    /// only gathered under `ENOMOTO_PROF_PHASES_EXT_WORK=1`:
    /// `rho_p` nonzeros, PRICE's inner-loop entries visited, PRICE's
    /// touched-column count (and how many of those were nonbasic),
    /// chuzc1 candidates, and the nonzero counts of `alpha_q` and of the
    /// DSE `tau` FTRAN, plus the wall time of that `tau` FTRAN alone
    /// (a sub-part of `DSE_UPDATE`).
    pub(super) static STAT_RHO_NNZ: AtomicUsize = AtomicUsize::new(0);
    pub(super) static STAT_PRICE_ENTRIES: AtomicUsize = AtomicUsize::new(0);
    pub(super) static STAT_TOUCHED: AtomicUsize = AtomicUsize::new(0);
    pub(super) static STAT_TOUCHED_NONBASIC: AtomicUsize = AtomicUsize::new(0);
    pub(super) static STAT_CANDS: AtomicUsize = AtomicUsize::new(0);
    pub(super) static STAT_ALPHA_NNZ: AtomicUsize = AtomicUsize::new(0);
    pub(super) static STAT_TAU_NNZ: AtomicUsize = AtomicUsize::new(0);
    pub(super) static DSE_FTRAN: AtomicUsize = AtomicUsize::new(0);
    pub(super) static CHUZC1_HEAP: AtomicUsize = AtomicUsize::new(0);
    pub(super) static STAT_M: AtomicUsize = AtomicUsize::new(0);
    pub(super) static STAT_PRICE_ENTRIES_NB: AtomicUsize = AtomicUsize::new(0);

    pub(super) fn reset() {
        use std::sync::atomic::Ordering::Relaxed;
        for c in [
            &BTRAN,
            &PRICE,
            &CHUZR,
            &CHUZC1,
            &BFRT,
            &FTRAN,
            &XB_UPDATE,
            &DSE_UPDATE,
            &DUAL_UPDATE,
            &FT_UPDATE,
            &REFACTOR,
            &REFACTOR_COUNT,
            &REFACTOR_CAUSE_VERIFY,
            &REFACTOR_CAUSE_TRY_UPDATE,
            &REFACTOR_CAUSE_BUMP,
            &REFACTOR_CAUSE_DRIFT,
            &REFACTOR_CAUSE_D_DRIFT,
            &REFACTOR_CAUSE_ILLCOND,
            &REFACTOR_CAUSE_MAX_UPDATES,
            &REFACTOR_CAUSE_INFEAS_CHECK,
            &REFACTOR_CAUSE_CLOCK,
            &MAX_UPDATE_STREAK,
            &ITERS,
            &BFRT_FLIPS,
            &INFEASIBLE_POOL,
            &M_EXIT,
            &HARRIS_WINDOW_M_MISS,
            &M_ENTER_VIA_FLIP,
            &DEGENERATE_PIVOTS,
            &HARRIS_WINDOW_SIZE_SUM,
            &POOL_GREW,
            &POOL_SHRANK,
            &POOL_SAME,
            &DSE_CHECKED,
            &DSE_ERR_LT_1PCT,
            &DSE_ERR_LT_10PCT,
            &DSE_ERR_LT_100PCT,
            &DSE_ERR_GE_100PCT,
            &STAT_RHO_NNZ,
            &STAT_PRICE_ENTRIES,
            &STAT_TOUCHED,
            &STAT_TOUCHED_NONBASIC,
            &STAT_CANDS,
            &STAT_ALPHA_NNZ,
            &STAT_TAU_NNZ,
            &DSE_FTRAN,
            &CHUZC1_HEAP,
            &STAT_M,
            &STAT_PRICE_ENTRIES_NB,
            &DENSITY_GATE_FTRANS,
            &DENSITY_COL_AQ_PPT,
            &DENSITY_BFRT_PPT,
        ] {
            c.store(0, Relaxed);
        }
    }

    /// Prints the same `PROF_PHASES`-shaped report as `simplex::solve_lp_dual`'s
    /// own printout, so the two are directly comparable line-for-line —
    /// called from [`super::solve_lp_dual_extended`] itself (not from
    /// `simplex.rs`, since only this function knows its own `wall_ns`).
    pub(super) fn report(wall_ns: usize) {
        use std::sync::atomic::Ordering::Relaxed;
        let iters = ITERS.load(Relaxed).max(1);
        let phases: [(&str, usize); 10] = [
            ("btran(rho_p)", BTRAN.load(Relaxed)),
            ("price", PRICE.load(Relaxed)),
            ("chuzr", CHUZR.load(Relaxed)),
            ("chuzc1", CHUZC1.load(Relaxed)),
            ("bfrt", BFRT.load(Relaxed)),
            ("ftran", FTRAN.load(Relaxed)),
            ("xb_update", XB_UPDATE.load(Relaxed)),
            ("dse_update", DSE_UPDATE.load(Relaxed)),
            ("dual_update", DUAL_UPDATE.load(Relaxed)),
            ("ft_update", FT_UPDATE.load(Relaxed)),
        ];
        let accounted: usize = phases.iter().map(|&(_, ns)| ns).sum::<usize>() + REFACTOR.load(Relaxed);
        eprintln!(
            "PROF_PHASES_EXT wall={:.3}ms iters={iters} ({:.1}us/iter) accounted={:.1}% of wall refactor_count={} (verify={} try_update={} bump={} drift={} d_drift={} illcond={} max_updates={} infeas_check={} clock={}) max_update_streak={}",
            wall_ns as f64 / 1e6,
            wall_ns as f64 / 1e3 / iters as f64,
            100.0 * accounted as f64 / wall_ns.max(1) as f64,
            REFACTOR_COUNT.load(Relaxed),
            REFACTOR_CAUSE_VERIFY.load(Relaxed),
            REFACTOR_CAUSE_TRY_UPDATE.load(Relaxed),
            REFACTOR_CAUSE_BUMP.load(Relaxed),
            REFACTOR_CAUSE_DRIFT.load(Relaxed),
            REFACTOR_CAUSE_D_DRIFT.load(Relaxed),
            REFACTOR_CAUSE_ILLCOND.load(Relaxed),
            REFACTOR_CAUSE_MAX_UPDATES.load(Relaxed),
            REFACTOR_CAUSE_INFEAS_CHECK.load(Relaxed),
            REFACTOR_CAUSE_CLOCK.load(Relaxed),
            MAX_UPDATE_STREAK.load(Relaxed)
        );
        eprintln!(
            "  dse_ftran(unfused)={:.1}us/iter chuzc1_heap={:.1}us/iter",
            DSE_FTRAN.load(Relaxed) as f64 / 1e3 / iters as f64,
            CHUZC1_HEAP.load(Relaxed) as f64 / 1e3 / iters as f64
        );
        if STAT_M.load(Relaxed) != 0 && STAT_RHO_NNZ.load(Relaxed) != 0 {
            eprintln!(
                "  work/iter (m={}): rho_nnz={:.1} price_entries={:.1} (nonbasic {:.1}) touched={:.1} touched_nonbasic={:.1} cands={:.1} alpha_nnz={:.1} tau_nnz={:.1}",
                STAT_M.load(Relaxed),
                STAT_RHO_NNZ.load(Relaxed) as f64 / iters as f64,
                STAT_PRICE_ENTRIES.load(Relaxed) as f64 / iters as f64,
                STAT_PRICE_ENTRIES_NB.load(Relaxed) as f64 / iters as f64,
                STAT_TOUCHED.load(Relaxed) as f64 / iters as f64,
                STAT_TOUCHED_NONBASIC.load(Relaxed) as f64 / iters as f64,
                STAT_CANDS.load(Relaxed) as f64 / iters as f64,
                STAT_ALPHA_NNZ.load(Relaxed) as f64 / iters as f64,
                STAT_TAU_NNZ.load(Relaxed) as f64 / iters as f64
            );
        }
        eprintln!(
            "  avg_bfrt_flips/iter={:.3} avg_infeasible_pool/iter={:.1}",
            BFRT_FLIPS.load(Relaxed) as f64 / iters as f64,
            INFEASIBLE_POOL.load(Relaxed) as f64 / iters as f64
        );
        {
            // Pivot-order reuse (`sparse_lu::factorize_reusing`, HiGHS
            // `HFactor::rebuild()`'s counterpart): how often the previous
            // order was tried, how often it was usable, how much wall time
            // the attempts (accepted *and* rejected) cost, and how many
            // steps had to re-pick their pivot row because the recorded
            // one was numerically empty (a basis column the Forrest-Tomlin
            // updates replaced since).
            use crate::simplex::sparse_lu;
            let attempts = sparse_lu::PROF_REBUILD_ATTEMPTS.load(Relaxed);
            let accepted = sparse_lu::PROF_REBUILD_ACCEPTED.load(Relaxed);
            eprintln!(
                "  pivot_order_reuse: attempts={attempts} accepted={accepted} ({:.1}%) attempt_time={:.3}ms row_repicks={} fail(singular={} fill={}) backoff_skips={} nnz_reuse_avg={:.0} nnz_full_avg={:.0}",
                100.0 * accepted as f64 / attempts.max(1) as f64,
                sparse_lu::PROF_REBUILD_NS.load(Relaxed) as f64 / 1e6,
                sparse_lu::PROF_REBUILD_ROW_REPICKS.load(Relaxed),
                sparse_lu::PROF_REBUILD_FAIL_SINGULAR.load(Relaxed),
                sparse_lu::PROF_REBUILD_FAIL_FILL.load(Relaxed),
                sparse_lu::PROF_REBUILD_BACKOFF_SKIPS.load(Relaxed),
                sparse_lu::PROF_REBUILD_ACCEPTED_NNZ.load(Relaxed) as f64 / accepted.max(1) as f64,
                sparse_lu::PROF_FULL_NNZ.load(Relaxed) as f64
                    / sparse_lu::PROF_FULL_COUNT.load(Relaxed).max(1) as f64,
            );
        }
        eprintln!(
            "  density_gate_ftrans={} final_expected_density col_aq={:.3} bfrt={:.3}",
            DENSITY_GATE_FTRANS.load(Relaxed),
            DENSITY_COL_AQ_PPT.load(Relaxed) as f64 / 1000.0,
            DENSITY_BFRT_PPT.load(Relaxed) as f64 / 1000.0
        );
        {
            use super::super::sparse_lu as lu;
            eprintln!(
                "  btran_l scatter={} gather={}",
                lu::PROF_BTRAN_L_SCATTER.load(Relaxed),
                lu::PROF_BTRAN_L_GATHER.load(Relaxed),
            );
            // Markowitz pivot search, from this module's side: `simplex.rs`'s
            // own `ENOMOTO_PROF_TRIANGULAR` block only ever runs on the
            // classical fallback path, which this solver returns before
            // reaching — so without these lines the pivot-search counters
            // are unreadable for every problem the extended solver actually
            // handles (i.e. all of them).
            let steps = lu::PROF_TOTAL_STEPS.load(Relaxed);
            eprintln!(
                "  pivot_search steps={steps} search_limit_hits={} ({:.1}%) candidates={} (avg {:.2}/step) scan={:.3}ms",
                lu::PROF_SEARCH_LIMIT_STEPS.load(Relaxed),
                100.0 * lu::PROF_SEARCH_LIMIT_STEPS.load(Relaxed) as f64 / steps.max(1) as f64,
                lu::PROF_SEARCH_CANDIDATES.load(Relaxed),
                lu::PROF_SEARCH_CANDIDATES.load(Relaxed) as f64 / steps.max(1) as f64,
                lu::PROF_BUCKET_SCAN_NS.load(Relaxed) as f64 / 1e6,
            );
            // §2.4: the work an incremental `colFixMax` could have removed
            // (see `PROF_COLMAX_RESCAN_ENTRIES`'s own docs), plus whether
            // this solve's pivot threshold left its starting value — a
            // flat benchmark on the escalation is uninterpretable without
            // the latter.
            eprintln!(
                "  col_max_abs rescan_entries={} pivot_threshold={} (escalations={})",
                lu::PROF_COLMAX_RESCAN_ENTRIES.load(Relaxed),
                lu::pivot_threshold(),
                lu::PROF_PIVOT_ESCALATIONS.load(Relaxed),
            );
        }
        eprintln!(
            "  m_exit(q_was_m)={} harris_window_m_miss={} m_enter_via_flip={}",
            M_EXIT.load(Relaxed),
            HARRIS_WINDOW_M_MISS.load(Relaxed),
            M_ENTER_VIA_FLIP.load(Relaxed)
        );
        eprintln!(
            "  degenerate_pivots={} ({:.1}% of iters) avg_harris_window_size={:.3}",
            DEGENERATE_PIVOTS.load(Relaxed),
            100.0 * DEGENERATE_PIVOTS.load(Relaxed) as f64 / iters as f64,
            HARRIS_WINDOW_SIZE_SUM.load(Relaxed) as f64 / iters as f64
        );
        eprintln!(
            "  pool_grew={} ({:.1}%) pool_shrank={} ({:.1}%) pool_same={} ({:.1}%)",
            POOL_GREW.load(Relaxed),
            100.0 * POOL_GREW.load(Relaxed) as f64 / iters as f64,
            POOL_SHRANK.load(Relaxed),
            100.0 * POOL_SHRANK.load(Relaxed) as f64 / iters as f64,
            POOL_SAME.load(Relaxed),
            100.0 * POOL_SAME.load(Relaxed) as f64 / iters as f64
        );
        let dse_checked = DSE_CHECKED.load(Relaxed).max(1);
        eprintln!(
            "  dse_rel_err: checked={} <1%={:.1}% <10%={:.1}% <100%={:.1}% >=100%={:.1}%",
            DSE_CHECKED.load(Relaxed),
            100.0 * DSE_ERR_LT_1PCT.load(Relaxed) as f64 / dse_checked as f64,
            100.0 * DSE_ERR_LT_10PCT.load(Relaxed) as f64 / dse_checked as f64,
            100.0 * DSE_ERR_LT_100PCT.load(Relaxed) as f64 / dse_checked as f64,
            100.0 * DSE_ERR_GE_100PCT.load(Relaxed) as f64 / dse_checked as f64
        );
        for (name, ns) in phases {
            eprintln!(
                "  {name:20} {:8.3}ms  {:5.1}% of wall  {:.3}us/iter",
                ns as f64 / 1e6,
                100.0 * ns as f64 / wall_ns.max(1) as f64,
                ns as f64 / 1e3 / iters as f64
            );
        }
        eprintln!(
            "  {:20} {:8.3}ms  {:5.1}% of wall  {:.3}us/iter",
            "refactor",
            REFACTOR.load(Relaxed) as f64 / 1e6,
            100.0 * REFACTOR.load(Relaxed) as f64 / wall_ns.max(1) as f64,
            REFACTOR.load(Relaxed) as f64 / 1e3 / iters as f64
        );
    }
}

/// Times `$body` and adds the elapsed nanoseconds to `$counter`, gated by
/// `$enabled` (a `bool` read once per iteration, matching `simplex.rs`'s
/// own `timed!` macro — duplicated here rather than shared across the
/// module boundary, see [`prof_phases`]'s own docs for why).
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

/// How often (in main-loop iterations) the incremental `x_B(M)` drift
/// check runs, checked every time regardless of `fill_count` — *not*
/// gated by a further `RESIDUAL_CHECK_MULTIPLIER`-style coarser cadence
/// the way `super::solve_lp_dual_on`'s own plain-`f64` `x_B` drift check
/// is. That module's comparisons are plain Dantzig ratios; this one's
/// (`Affine1::cmp_lex`/`Score2::cmp_lex`) decide almost every comparison
/// on the *slope* term first, at a much tighter `REL_TOL` (`1e-9`) than
/// the classical method's own drift tolerance (`FT_RESIDUAL_TOL`,
/// `1e-4`) ever has to survive. Confirmed load-bearing, not merely
/// tighter-for-safety's-sake: Netlib `maros`, at the classical method's
/// own 100-iteration cadence, silently drifted its incrementally-
/// maintained `x_B(M)` enough (each individual step well inside a loose
/// tolerance, but accumulating over ~900 pivots) to flip a `cmp_lex`
/// decision and reach the "no eligible entering column" case on a
/// problem HiGHS solves — a false `Infeasible`, not a numerical no-op —
/// before this tighter cadence existed.
///
/// **Loosening just this interval (keeping `XB_DRIFT_TOL` itself tight)
/// was tried and reverted** — a bottleneck-analysis follow-up
/// (`ENOMOTO_DEBUG_XB_DRIFT_EXT`) found that every drift-triggered
/// refactor sampled across several slow Netlib instances, `maros`
/// included, came from the `base` channel alone (`slope` never exceeded
/// ~1e-12, four-plus orders of magnitude below even this tight
/// tolerance), which made "check less often, same tolerance" look like a
/// safe lever: `maros` itself stayed optimal all the way out to a
/// (temporary, env-var-overridden) 100-iteration interval. But a full
/// 73-problem benchmark run at interval `50` told a different story: 5
/// problems broke — `cycle` and `degen3` regressed to a **false
/// `Infeasible`** (exactly the failure mode this whole mechanism exists
/// to prevent, just on different instances than `maros`), and
/// `perold`/`pilotnov`/`wood1p` blew past a 30s timeout (a degenerate
/// instance's pivot sequence is sensitive to the exact floating-point
/// state a resync leaves behind — `25fv47`'s own iteration count swung
/// non-monotonically between roughly 5,000 and 20,000 across intervals
/// 5–100 during this same sweep, confirming the effect isn't isolated to
/// the two infeasible cases). `maros` alone passing was not
/// representative of the other 72 problems' own margins — reverted back
/// to `FT_CHECK_INTERVAL` (`5`).
const XB_CHECK_INTERVAL: usize = super::FT_CHECK_INTERVAL;

/// The drift tolerance [`XB_CHECK_INTERVAL`]'s own check compares against
/// — tighter than `super::FT_RESIDUAL_TOL` for the same reason that
/// constant's own docs give: this module's decisions are sensitive at
/// `Affine1::cmp_lex`'s `1e-9` `REL_TOL`, so the drift check needs
/// headroom below that, not `FT_RESIDUAL_TOL`'s much looser `1e-4`.
///
/// **History — four single-knob loosening attempts tried and reverted
/// before landing on the per-solve escalation below:**
///
/// 1. Loosening [`XB_CHECK_INTERVAL`] (keeping the tolerance itself tight)
///    broke `cycle`/`degen3` into false `Infeasible`.
/// 2. Loosening this *absolute* bound itself (`1e-8` -> `1e-7`) fixed
///    `maros` but regressed `fit1p` 5.8x on a full 73-problem sweep.
/// 3. A *relative* bound scaled by `‖fresh_base‖`/`‖fresh_slope‖` alone
///    (mirroring [`D_DRIFT_TOL`]'s own `scale_d` pattern for `d`), tried at
///    two values three orders of magnitude apart, regressed the
///    73-problem set either way (~+8-9%) while giving big wins on
///    `d2q06c`/`greenbeb`/`pilot` — the `‖x_B(M)‖`-only floor (`1.0`) never
///    actually engages on real Netlib instances (every instance measured
///    sits far above it), so a single multiplier just applies uniformly to
///    everyone.
/// 4. A backward-error-style scale (`‖A_B‖_max * ‖x_B(M)‖ + ‖fresh_rhs‖`,
///    the standard LAPACK relative-residual formula for `Ax=b`) measured
///    real per-instance scales spanning `~43` (`wood1p`) to `~3.6e6`
///    (`maros`) — but `fit1p` (scale `~859`, fragile to *any* loosening per
///    attempt 2) sits *above* `wood1p` (scale `~43`, which needs loosening
///    just to avoid becoming *stricter* than the old absolute bound). No
///    single scale-derived multiplier can loosen `wood1p` without loosening
///    `fit1p` by more than its own known-fragile margin — fragility and
///    problem-scale don't correlate under any norm tried.
///
/// **This version** breaks that correlation requirement entirely: instead
/// of predicting up front which problems need a looser bound from some
/// static property, it escalates *within a single solve*, based only on
/// how many times *this solve's own* drift check has already fired
/// ([`XB_DRIFT_ESCALATION_STEP`]/[`XB_DRIFT_ESCALATION_FACTOR`]/
/// [`XB_DRIFT_TOL_MAX`]'s own docs). A problem that drift-refactors 0-9
/// times in its whole solve (measured: `fit1p` 5, `wood1p` 1, `cycle` 0-2,
/// `degen3` 0, `pilotnov` 6 — every instance any prior attempt broke or
/// nearly broke) never reaches the first escalation step, so it sees
/// *zero* behavior change from the unmodified `1e-8` this constant always
/// was. Only a solve that has already proven itself drift-heavy (`d2q06c`
/// 546, `greenbeb` 236, `pilot` 88-190, `fit2p` 25, all measured at the
/// flat `1e-8` baseline) earns a progressively looser bound — and since
/// loosening it also slows the *rate* new drift triggers accumulate, this
/// is a self-damping control loop, not an open-loop guess: a solve
/// escalates only as fast as its own residual growth actually demands.
const XB_DRIFT_TOL: f64 = 1e-8;
/// Every this many drift-triggered refactorizations *within the same
/// solve*, [`XB_DRIFT_TOL`]'s own effective bound multiplies by
/// [`XB_DRIFT_ESCALATION_FACTOR`] (capped at [`XB_DRIFT_TOL_MAX`]) — see
/// that constant's own docs for why this is a per-solve escalation rather
/// than a static per-problem scale. `10` keeps every instance measured at
/// single-digit drift-refactor counts (the ones prior attempts broke)
/// entirely below the first step, while still letting a genuinely
/// pathological solve (hundreds of triggers at the flat bound) climb
/// through several steps before this cap's own `1e-4` ceiling.
const XB_DRIFT_ESCALATION_STEP: usize = 10;
/// Multiplier applied per [`XB_DRIFT_ESCALATION_STEP`] drift triggers.
/// `10` mirrors the *single* absolute-loosening step attempt 2 (this
/// constant's own docs) already measured in isolation (`1e-8` -> `1e-7`
/// fixed `maros`, broke `fit1p`) — the escalation ladder repeats that same,
/// already-characterized step size rather than inventing a new one, but
/// only after `XB_DRIFT_ESCALATION_STEP` proves the *current* solve is
/// actually the kind that benefits from it.
const XB_DRIFT_ESCALATION_FACTOR: f64 = 10.0;
/// Ceiling on the escalated [`XB_DRIFT_TOL`] — reuses `super::FT_RESIDUAL_TOL`'s
/// already-proven-safe order of magnitude (the classical method's own
/// absolute drift bound, `1e-4`) rather than letting escalation grow
/// unbounded into territory no measurement has ever validated.
const XB_DRIFT_TOL_MAX: f64 = super::FT_RESIDUAL_TOL;

/// How many *numerically-caused* refactorizations this solve has to take
/// before its LU pivot threshold is escalated one step
/// (`sparse_lu::escalate_pivot_threshold`,
/// `docs/lu_comparison_enomoto_vs_highs.md` §2.4) — **`0`, i.e. the
/// escalation is off by default**, because it was measured and lost.
///
/// HiGHS raises `info_.factor_pivot_threshold` on a numerical failure and
/// this crate can too, but on NETLIB93 tightening the floor costs far more
/// in fill-in than it saves in refactorizations: at `10` (the value
/// [`XB_DRIFT_ESCALATION_STEP`]'s own ladder uses, and the one measured)
/// the 93-problem total went **+7.4%**, with `pilot87` +29.2% (6.50s ->
/// 8.40s, reproducible across all three runs), `brandy` +67% and
/// `gfrd-pnc` +12.3% — three problems past the 10%-regression bar on their
/// own. The escalation fired on 7 of the 10 heaviest problems, because the
/// `x_B(M)` drift trigger alone reaches 10 on most of them (`pilot` 21,
/// `dfl001` 24), so it is the *ordinary* heavy solve that gets the `0.5`
/// floor `sparse_lu::STABILITY`'s own docs already measured as ~4% worse.
/// Raising this constant until only pathological solves qualify makes it
/// fire nowhere on NETLIB93 at all, which is not a measurable improvement
/// either — hence off, rather than retuned.
///
/// The mechanism is kept (and reachable via
/// `ENOMOTO_PIVOT_ESCALATION_STEP`, alongside `ENOMOTO_PIVOT_THRESHOLD`
/// for the floor itself) so that a future attempt — a smaller step than
/// `sparse_lu::PIVOT_THRESHOLD_FACTOR`'s doubling, or a trouble signal
/// narrower than the four below — can be A/B'd without re-plumbing it.
///
/// "Numerically caused" means the four triggers that fire because the
/// factorization stopped agreeing with the basis it stands for — a
/// rejected Forrest-Tomlin update, `x_B(M)` drift, `d` drift, and a pivot
/// grossly inconsistent with PRICE. It deliberately excludes the
/// *cost*-based triggers (eta-bump fill, `ft_max_updates`, the
/// deterministic CLOCK): those fire on schedule even on a perfectly
/// conditioned problem, so counting them would escalate `dfl001`'s
/// hundreds of routine refactorizations into fill-in it has no numerical
/// reason to pay for.
const PIVOT_ESCALATION_STEP: usize = 0;

/// Trigger (4) for this module — `super::FT_MAX_UPDATES`'s own equivalent
/// (an unconditional backstop against unbounded Forrest-Tomlin eta-chain
/// growth, independent of the fill-based trigger (3) above and the
/// `XB_CHECK_INTERVAL`/`XB_DRIFT_TOL` drift check), which this module never
/// had at all until now — `super::FT_MAX_UPDATES` itself is only ever read
/// from `super::solve_lp_dual_on`/`run_phase` (`grep` confirms no reference
/// here), so a pathological pivot sequence whose eta fill happens to stay
/// under trigger (3)'s own `FT_BUMP_LIMIT_FACTOR * m` budget indefinitely
/// (a very sparse basis, or one where each update's own fill stays small)
/// could accumulate Forrest-Tomlin updates without any hard ceiling.
///
/// **Sized adaptively to `m`, not copied as `super::FT_MAX_UPDATES`'s flat
/// `300`** — a flat value tuned against the classical method's own problem
/// mix would be wrong here by construction: this module already runs
/// noticeably longer streaks between refactorizations, on some instances
/// well past `super::FT_MAX_UPDATES` itself, before this trigger existed at
/// all. Confirmed by adding [`prof_phases::MAX_UPDATE_STREAK`] (a
/// `fetch_max` gauge of `lu.update_count()`, paying nothing beyond one
/// atomic op per pivot) and sweeping essentially every Netlib `.mps` file
/// available locally — not just the 73-problem, <=3000-variable subset the
/// rest of this crate's own benchmark methodology otherwise targets, since
/// calibrating a hard safety ceiling specifically wants the *widest*
/// available range of `m` and pivot-sequence shapes, including the larger
/// instances that benchmark excludes. The worst observed ratio
/// (`peak_streak / m`) was **not** the largest basis in the sweep — it was
/// `nesm` (`m=662`, peak streak `1090`, ratio `1.647`) — ahead of `scsd6`
/// (`1.395`), `scsd1` (`1.338`), `adlittle` (`1.732`, but `m=56` is small
/// enough [`FT_MAX_UPDATES_FLOOR`]'s own floor absorbs it), and `stocfor2`
/// (`m=2157`, peak streak `1665`, ratio `0.772` — the instance this
/// constant's very first draft was calibrated against, before the fuller
/// sweep found worse ratios elsewhere; kept in this history as a reminder
/// that a handful of hand-picked instances is not a substitute for sweeping
/// everything available). [`ft_max_updates`] scales with `m`
/// ([`FT_MAX_UPDATES_FACTOR`] `* m`, floored at [`FT_MAX_UPDATES_FLOOR`] so
/// a tiny basis still gets at least the classical method's own
/// already-proven `300`) — at `nesm`'s own `m=662` this gives `1986`, a
/// `1.82x` margin over its own observed peak, comfortably wider than the
/// `1.21x` a smaller factor (`2.0`) left there. Every other instance in the
/// sweep has a lower ratio than `nesm`'s, so this margin is the binding one
/// crate-wide, not merely for one instance. Generous by design — this is
/// meant to sit as a rarely-firing safety net (mirroring
/// `super::FT_MAX_UPDATES`'s own documented role once tuned high enough —
/// see that constant's own docs), not a routine performance lever the way
/// trigger (3) is; `max_updates_fired=0` across every instance in the same
/// sweep (see [`prof_phases::REFACTOR_CAUSE_MAX_UPDATES`]) confirms this
/// trigger changes nothing about the current benchmark's own behavior —
/// its only job is bounding the *next* pathological instance that shows up.
///
/// `3.0` carries real margin above the worst case actually measured, not an
/// exhaustively swept optimum the way [`super::FT_BUMP_LIMIT_FACTOR`] was —
/// re-tune (sweeping *at least* as wide a problem set as the survey above,
/// not just the 73-problem subset) if
/// [`prof_phases::REFACTOR_CAUSE_MAX_UPDATES`] is ever observed firing
/// nonzero on a real instance, which would mean either the factor needs
/// raising further or a genuinely pathological low-fill/long-streak
/// instance has been found.
const FT_MAX_UPDATES_FACTOR: f64 = 3.0;
/// Floor for [`ft_max_updates`] — never weaker than the classical method's
/// own already-proven-safe flat cap, regardless of how small `m` is.
const FT_MAX_UPDATES_FLOOR: usize = super::FT_MAX_UPDATES;

/// `m`-scaled trigger (4) threshold — see [`FT_MAX_UPDATES_FACTOR`]'s own
/// docs for the reasoning and the measurement that ruled out reusing
/// `super::FT_MAX_UPDATES`'s flat value directly.
#[inline]
fn ft_max_updates(m: usize) -> usize {
    ((FT_MAX_UPDATES_FACTOR * m as f64) as usize).max(FT_MAX_UPDATES_FLOOR)
}

/// Trigger (5): the deterministic, cost-based refactorization trigger
/// (`analysis/ft_refactor_trigger_20260922_040850.md` §5/§6) — this
/// module's replacement for that analysis's wall-clock
/// `ENOMOTO_SYNTH_CLOCK` prototype, using [`sparse_lu::FtLu::synth_tick`]'s
/// deterministic operation-count accumulator instead of `Instant::now()` so
/// the same solve always refactorizes at the same iterations (the
/// analysis's own §6 explicitly calls out replacing the wall-clock stand-in
/// with exactly this kind of counter before shipping it, precisely to avoid
/// making refactorization timing — and hence the whole pivot sequence —
/// depend on machine load/scheduling noise).
///
/// Mirrors HiGHS's own `HEkk::updateFactor` (`HEkk.cpp:3075-3090`,
/// `total_synthetic_tick_ >= build_synthetic_tick_ && update_count >= 50`):
/// once `update_count` reaches [`SYNTH_CLOCK_MIN_UPDATES`] *and* the
/// accumulated solve-side tick since the last refactorization reaches
/// `SYNTH_CLOCK_FACTOR * lu.build_tick()`, this basis is deemed to have
/// already "paid for" a fresh factorization in the FTRAN/BTRAN work spent
/// solving against the current (eta-chain-lengthened) one — see this
/// trigger's own analysis file, §2.3 in particular, for why that FTRAN/BTRAN
/// unit cost keeps climbing with chain length (the `R`-eta stage's own
/// gather structure can't skip zeros, so a longer chain means literally
/// more nonzero-multiply-adds every single solve).
///
/// **`SYNTH_CLOCK_FACTOR` calibration**: the wall-clock prototype
/// (`ENOMOTO_SYNTH_CLOCK`) found 2-4x optimal in *wall-clock* units, but a
/// tick built from [`sparse_lu::TICK_BUILD_M_COEF`]/[`sparse_lu::TICK_BUILD_LU_COEF`]
/// left at HiGHS's own values is **not** the same unit as wall-clock
/// seconds, so that factor doesn't carry over — re-swept from scratch here,
/// in tick units, over the same 24-problem set the analysis itself used
/// (`analysis/ft_refactor_trigger_20260922_040850.md` §5's own table).
///
/// A coarse sweep (`ENOMOTO_SYNTH_CLOCK_FACTOR` in `{1, 1.5, 2, 3, 4, 6, 8,
/// 12, 16, 20, 24, 32, 48, 64, 100}`, one full 24-problem pass per value)
/// found the aggregate 24-problem total *non-monotonic* — individual
/// instances (`pilot87` worst, occasionally `dfl001`) are sensitive to
/// exactly where a refactorization lands (a changed pivot sequence can
/// resync onto a longer or shorter path than before; §5's own note that
/// "反復数の変化は...丸めが変わるため" already flags this), so a single
/// aggregate-total-minimizing value can hide a large regression on one
/// instance a small improvement on many others outweighs in the sum. `12`,
/// for instance, is a genuine cliff (`pilot87` alone balloons from ~10s to
/// ~70s at that exact value, not measurement noise — confirmed
/// reproducible bit-for-bit given [`Self::tick`]'s own determinism) that a
/// coarser or finer grid could easily have stepped over in either
/// direction. `16` was chosen instead by the same per-problem regression
/// budget this trigger's own verification uses (no instance may regress
/// >10%): every one of the 24 problems is flat-to-improved at `16` except
/// `pilot87` (+7-9%, confirmed stable — not a cliff — at `10`/`14`/`16`/
/// `18`/`20` alike) and `dfl001` (-1.6%, i.e. not a regression at all) —
/// the two instances with by far the largest absolute runtime, so keeping
/// *both* comfortably inside the regression budget outweighed chasing a
/// marginally lower 24-problem aggregate at `20` (which flips that
/// trade-off: `dfl001` +5.1%, `pilot87` ~flat) or higher. The target
/// instances this trigger exists for (`stocfor2` -39%, `bnl2` -27%,
/// `80bau3b` -31%, `greenbea` -23%, `degen3` -24%, `d2q06c` -17%) are all
/// comfortably at or beyond the wall-clock prototype's own §5 numbers at
/// this value. See this crate's commit history around this trigger's
/// introduction for the full sweep's raw numbers if re-calibrating.
const SYNTH_CLOCK_FACTOR: f64 = 16.0;

/// [`SYNTH_CLOCK_FACTOR`], overridable via `ENOMOTO_SYNTH_CLOCK_FACTOR` —
/// a calibration knob only (parsed once, cached), matching this file's
/// existing convention of env-var-gated tunables for values that need
/// re-sweeping against a benchmark set (e.g. the analysis's own
/// `ENOMOTO_SYNTH_CLOCK` wall-clock prototype this trigger replaces). The
/// *shipped* behavior is [`SYNTH_CLOCK_FACTOR`]'s own hardcoded, calibrated
/// value; this override exists so a future re-calibration sweep (a basis
/// composition shift large enough to move the crate's own per-nonzero cost
/// ratios — see [`sparse_lu::TICK_BUILD_LU_COEF`]'s own docs) doesn't need a
/// rebuild per candidate value.
fn synth_clock_factor() -> f64 {
    static FACTOR: OnceLock<f64> = OnceLock::new();
    *FACTOR.get_or_init(|| {
        std::env::var("ENOMOTO_SYNTH_CLOCK_FACTOR")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|f| f.is_finite() && *f > 0.0)
            .unwrap_or(SYNTH_CLOCK_FACTOR)
    })
}
/// Same role as HiGHS's own `kSyntheticTickReinversionMinUpdateCount`
/// (`50`, `HEkk.h`) — a floor below which this trigger never fires
/// regardless of `synth_tick`, so a basis that has barely been updated at
/// all (where a stray large tick from a single unusually dense solve could
/// otherwise fire this trigger prematurely) always gets at least this many
/// Forrest-Tomlin updates first. Kept at HiGHS's own value: nothing in this
/// crate's own cost structure (unlike [`SYNTH_CLOCK_FACTOR`], which *does*
/// need re-deriving — see that constant's own docs) gives a reason to move
/// off HiGHS's number here, since this floor's only job is ruling out a
/// noisy false-positive on the *first few* updates, independent of either
/// side's own per-update cost.
const SYNTH_CLOCK_MIN_UPDATES: usize = 50;

/// Trigger (5) itself — see [`SYNTH_CLOCK_FACTOR`]'s own docs. Checked
/// unconditionally (like trigger (4)'s own `ft_max_updates` check, a single
/// integer comparison plus two `Cell` reads, cheap enough to run every
/// pivot) by both `solve_lp_dual_extended`'s main loop and
/// `polish_with_true_bounds`, so it is shared here rather than duplicated —
/// unlike the drift/bump checks (trigger (2)/(3)), which read module-local
/// state (`x_b_base`/`x_b_slope`, only the main loop's own `Affine1`
/// variables exist) that has no polish-phase equivalent to share against.
#[inline]
fn synth_clock_should_refactor(lu: &sparse_lu::FtLu) -> bool {
    lu.update_count() >= SYNTH_CLOCK_MIN_UPDATES && (lu.synth_tick() as f64) >= synth_clock_factor() * (lu.build_tick().max(1) as f64)
}

/// Independent drift check for the incrementally-maintained `d` (reduced
/// costs), checked on the same [`XB_CHECK_INTERVAL`] cadence as
/// [`XB_DRIFT_TOL`] but against its own residual (`‖d - fresh_d‖` scaled by
/// `‖fresh_d‖`, [`fresh_d_into`]'s own recomputation), not `x_B(M)`'s.
///
/// Added after tracking down a real false `Infeasible` on Netlib `pilot4`:
/// `d` drifted from its true (BTRAN-recomputed) value by an amount that
/// reached the *millions* — not noise — for over 200 nonbasic columns by
/// the time chuzc1 found no eligible entering column and this loop
/// concluded (wrongly) that the row it had just selected was a genuine
/// Proposition 4.6(ii) infeasibility. Before this check, `d`'s only
/// correction was as a *side effect* of `x_B(M)`'s own drift/refactor
/// triggers (`fresh_d_into` is called there anyway once a refactor already
/// fires) — nothing ever measured `d`'s own drift directly, so a pivot
/// sequence that happened to keep `x_B(M)`'s residual under [`XB_DRIFT_TOL`]
/// and `try_update` succeeding could let `d`'s independent error compound
/// unchecked for as long as that held. Confirmed present but *harmless* on
/// the unmodified baseline too (a genuine, moderate-magnitude violation at
/// one `pilot4` iteration that never compounded before the next refactor
/// happened to clear it) — this check does not depend on, and was not
/// caused by, any experimental pivot-selection change; it closes a latent
/// gap in this loop's own numerical safety net that any sufficiently
/// unlucky pivot sequence could have hit.
const D_DRIFT_TOL: f64 = 1.0;

/// How far apart `alpha_q` (PRICE) and `alpha_full[r]` (FTRAN) may be,
/// relatively, before the entering pivot counts as "grossly inconsistent"
/// (its own call site's docs) rather than merely the ordinary floating-
/// point disagreement `super::UPDATE_VERIFY_TOL` (`1e-7`) already screens
/// for when `lu.update_count() > 0`. Sits far above `UPDATE_VERIFY_TOL`
/// deliberately: `0.5` still comfortably separates Netlib `pilot4`'s
/// `alpha_q ~ 3.4e-9` vs `alpha_full[r] ~ 1e-21` (a relative gap of
/// essentially `1.0`) from `bnl1`'s persistent, perfectly legitimate
/// `~2.4e-7` disagreement on a normal-magnitude (`~3.3e-3`) pivot — the
/// two Netlib instances that pinned this threshold's lower and upper
/// bounds respectively (confirmed via a full 73-problem sweep both ways:
/// tighter reproduced `perold`'s old refactor-storm pathology on `bnl1`
/// instead; this value reproduces neither).
const D_GROSS_MISMATCH_REL_TOL: f64 = 0.5;

/// Test-only instrumentation: counts cleanup-lemma pivots actually
/// performed (the "found r2" branch in [`finish`]) so a direct unit test
/// can confirm that code path was exercised, rather than needing to
/// hand-derive in advance which hand-built example reaches it.
///
/// Per *thread* (tests run in parallel, and a process-global counter let
/// any concurrently running test's cleanup pivots leak into another
/// test's before/after comparison — a timing-dependent flake), behind an
/// `AtomicUsize`-shaped `load`/`fetch_add` interface.
#[cfg(test)]
pub(crate) struct ThreadLocalCounter;
#[cfg(test)]
thread_local! {
    static CLEANUP_PIVOTS_TL: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
impl ThreadLocalCounter {
    pub(crate) fn load(&self, _: std::sync::atomic::Ordering) -> usize {
        CLEANUP_PIVOTS_TL.with(|c| c.get())
    }
    pub(crate) fn fetch_add(&self, v: usize, _: std::sync::atomic::Ordering) -> usize {
        CLEANUP_PIVOTS_TL.with(|c| {
            let old = c.get();
            c.set(old + v);
            old
        })
    }
}
#[cfg(test)]
pub(crate) static CLEANUP_PIVOTS: ThreadLocalCounter = ThreadLocalCounter;

/// Test-only instrumentation: counts how many candidates the BFRT walk
/// actually flipped (summed across every iteration of every solve) before
/// reaching the real entering column — i.e. how many times the combined-
/// flip incremental `x_B(M)` update actually ran with nonempty work,
/// rather than being a no-op every iteration. A direct unit test can
/// confirm a hand-built example exercises that path at all, rather than
/// needing to hand-derive in advance exactly which iteration does.
#[cfg(test)]
pub(crate) static COMBINED_FLIP_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// `x_B(M)`'s `M`-coefficients are exactly zero or of order one in exact
/// arithmetic; anything this small is accumulated LU/update noise. Left in,
/// `Affine1::cmp_lex`'s slope-first order lets it override a comfortably
/// feasible `base` and drives two-variable cycles (Netlib `greenbea`).
const X_B_SLOPE_NOISE: f64 = 1e-7;

#[inline]
fn snap_slope(v: f64) -> f64 {
    if v.abs() < X_B_SLOPE_NOISE {
        0.0
    } else {
        v
    }
}

#[inline]
fn snap_slopes(v: &mut [f64]) {
    for x in v.iter_mut() {
        *x = snap_slope(*x);
    }
}

/// `base + slope * M`, for a conceptual, never-numerically-substituted
/// `M -> +infinity`. See this module's own docs for why comparisons never
/// plug in a concrete `M`.
#[derive(Clone, Copy, Debug)]
struct Affine1 {
    base: f64,
    slope: f64,
}

impl Affine1 {
    const ZERO: Affine1 = Affine1 { base: 0.0, slope: 0.0 };

    #[inline]
    fn new(base: f64, slope: f64) -> Self {
        Affine1 { base, slope }
    }

    /// The paper's `succ` (\S4.5): compares `(slope, base)`
    /// lexicographically — for any `M` at least as large as some
    /// (unneeded-to-compute) threshold, `self.value(M) > other.value(M)`
    /// iff this returns `Greater`.
    ///
    /// Both components use a *relative* tolerance, not exact equality: two
    /// `Affine1` values that are mathematically equal (e.g. a row's own
    /// deviation and the BFRT walk's accumulated flip capacity, at exactly
    /// the point capacity should just cover it) are typically computed via
    /// entirely different paths — one through an LU solve chain
    /// (`solve_x_b`), the other by summing per-candidate capacities
    /// (`width_affine`/`scale`/`add`) — so they land a few ULPs apart, not
    /// bit-for-bit identical. An exact `total_cmp` (tried first) treated
    /// that noise as a genuine, decisive difference: confirmed directly on
    /// Netlib `degen2` (named for exactly this kind of degeneracy) — a
    /// slope difference of order `1e-13` on a magnitude-`~2.8` slope made
    /// an exactly-sufficient BFRT capacity register as `Less` than the
    /// deviation it was supposed to exactly cover, so the walk ran off the
    /// end of the candidate list and reported a false `Infeasible`.
    #[inline]
    fn cmp_lex(&self, other: &Affine1) -> std::cmp::Ordering {
        const REL_TOL: f64 = 1e-9;
        let slope_scale = self.slope.abs().max(other.slope.abs()).max(1.0);
        let slope_diff = self.slope - other.slope;
        if slope_diff.abs() > REL_TOL * slope_scale {
            return if slope_diff > 0.0 { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less };
        }
        let base_scale = self.base.abs().max(other.base.abs()).max(1.0);
        let base_diff = self.base - other.base;
        if base_diff.abs() > REL_TOL * base_scale {
            if base_diff > 0.0 { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less }
        } else {
            std::cmp::Ordering::Equal
        }
    }

    /// Exactly `self.cmp_lex(&Affine1::ZERO) == Ordering::Greater`, with
    /// the tolerance arithmetic folded for the `other == 0` case (hot:
    /// every primal-feasibility re-check calls this twice). Against zero
    /// `cmp_lex`'s slope test is `|s| > 1e-9 * max(|s|, 1)`, which is
    /// `|s| > 1e-9` for `|s| < 1` (the scale is exactly `1.0`), always true
    /// for finite `|s| >= 1` (`1e-9 * |s| < |s|` in floating point), and
    /// false for `|s| = inf` (`inf > inf`) or NaN — i.e. `1e-9 < |s| < inf`;
    /// the base test folds the same way, and `Greater` additionally needs
    /// the deciding component to be positive.
    #[inline]
    fn gt_zero(&self) -> bool {
        const REL_TOL: f64 = 1e-9;
        let sa = self.slope.abs();
        if sa > REL_TOL && sa < f64::INFINITY {
            return self.slope > 0.0;
        }
        self.base > REL_TOL && self.base < f64::INFINITY
    }

    #[inline]
    fn add(self, other: Affine1) -> Affine1 {
        Affine1::new(self.base + other.base, self.slope + other.slope)
    }

    #[inline]
    fn sub(self, other: Affine1) -> Affine1 {
        Affine1::new(self.base - other.base, self.slope - other.slope)
    }

    #[inline]
    fn scale(self, k: f64) -> Affine1 {
        Affine1::new(self.base * k, self.slope * k)
    }
}

/// Whether the BFRT walk's accumulated flip capacity `cum` has caught up
/// to (covers) the row deviation `w_r` — i.e. whether the walk should stop
/// *here* rather than keep consuming candidates. Same lexicographic
/// (slope, then base) comparison as [`Affine1::cmp_lex`], but the base
/// channel's tolerance is scaled by this row's own basic-value magnitude
/// (`x_b_base_r`), not `cmp_lex`'s bare `1e-9`-absolute floor (its
/// `base_scale` floors at `1.0` regardless of the magnitudes the two
/// `Affine1`s actually carry). That floor is fine for most rows, but this
/// module's own Forrest-Tomlin drift check already tolerates `x_B` error
/// up to `XB_DRIFT_TOL_MAX` (`1e-4`) between refactorizations — on a row
/// whose basic value reaches `1e5..1e8` (large-magnitude Netlib instances,
/// confirmed on `greenbea`: `analysis/greenbea_20260921_030127.md`), that
/// ordinary refactorization noise (measured there at `3.0e-7`, five orders
/// of magnitude above `1e-9`) reads as a genuine, unrecoverable shortfall
/// and the walk runs off the end reporting a false `Infeasible` — even
/// though a fresh factorization shows the same row's deviation is exactly
/// covered. Mirrors `polish_with_true_bounds`'s own `reach_tol` (its own
/// docs) and `super::PRIMAL_FEAS_TOL`'s stated purpose (a large-magnitude
/// row's rounding floor scales with it; a fixed absolute bar is
/// simultaneously too strict on large rows and too loose on tiny ones).
#[inline]
fn bfrt_reached(w_r: Affine1, cum: Affine1, x_b_base_r: f64) -> bool {
    const REL_TOL: f64 = 1e-9;
    let slope_scale = w_r.slope.abs().max(cum.slope.abs()).max(1.0);
    let slope_diff = w_r.slope - cum.slope;
    if slope_diff.abs() > REL_TOL * slope_scale {
        return slope_diff <= 0.0;
    }
    let base_diff = w_r.base - cum.base;
    base_diff <= super::PRIMAL_FEAS_TOL * w_r.base.abs().max(x_b_base_r.abs()).max(1.0)
}

/// `Δ_i(M)^2 / w_i` (paper \S4.5's steepest-edge/Devex generalization),
/// as the coefficients of the resulting degree-2 polynomial in `M` —
/// `w_i` itself never depends on `M` (`super::DseState`/`DevexState`'s own
/// weights are pure tableau-row quantities, `super::solve_lp_dual_on`'s
/// module docs), so squaring `Δ_i = slope*M + base` and dividing by `w_i`
/// is the only place a degree-2 (rather than degree-1) comparison enters
/// this module at all.
#[derive(Clone, Copy, Debug)]
struct Score2 {
    c2: f64,
    c1: f64,
    c0: f64,
}

impl Score2 {
    #[inline]
    fn new(dev: Affine1, w: f64) -> Self {
        let w = w.max(super::STEEPEST_EDGE_FLOOR);
        if dev.slope == 0.0 {
            // `M`-free deviation (every row once delta=0 is reached): skip
            // two of the three divisions. `c2` is `+0.0` either way
            // (`0*0/w`); `c1` would be `±0.0` depending on `base`'s sign,
            // but [`Score2::cmp_lex`] only ever looks at `|a - b|` and the
            // sign of a nonzero `a - b`, neither of which a zero operand's
            // sign can change — so every comparison is bit-for-bit the one
            // the full formula would produce.
            return Score2 { c2: 0.0, c1: 0.0, c0: dev.base * dev.base / w };
        }
        Score2 { c2: dev.slope * dev.slope / w, c1: 2.0 * dev.slope * dev.base / w, c0: dev.base * dev.base / w }
    }

    /// Degree-2 lexicographic order (paper \S4.5's "多項式の辞書式順序"),
    /// with the same relative tolerance at every degree as
    /// [`Affine1::cmp_lex`] and for the identical reason: two candidate
    /// rows' scores that are mathematically tied (most commonly, both
    /// exactly `0` before either has any `M`-dependence, or two BFRT-
    /// unrelated rows with identical raw deviations) are computed from
    /// unrelated FTRAN/BTRAN chains and so are not expected to agree past
    /// a few ULPs.
    /// `c2_tol` overrides the leading (`c2`, slope-driven) term's relative
    /// tolerance for this one comparison — `c1`/`c0` always use the fixed
    /// `1e-9`. EXPERIMENTAL (`ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL`, its own call
    /// site's docs): the main loop below derives `c2_tol` per iteration
    /// from how many M-flagged columns are still unresolved, so passing
    /// `1e-9` here (the default when that env var is unset) reproduces the
    /// original fixed-tolerance comparison exactly.
    #[inline]
    fn cmp_lex(&self, other: &Score2, c2_tol: f64) -> std::cmp::Ordering {
        const REL_TOL: f64 = 1e-9;
        for (idx, (a, b)) in [(self.c2, other.c2), (self.c1, other.c1), (self.c0, other.c0)].into_iter().enumerate() {
            let tol = if idx == 0 { c2_tol } else { REL_TOL };
            let scale = a.abs().max(b.abs()).max(1.0);
            let diff = a - b;
            if diff.abs() > tol * scale {
                return if diff > 0.0 { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less };
            }
        }
        std::cmp::Ordering::Equal
    }
}

/// Which side(s) of structural column `j` are `M`-tracked (paper's Lemma
/// 4.1, generalized to a genuinely free column tracking *both*): `Lower`
/// if only `lb[j] == -inf` (placing it at `Lower` means `x_j = -M`),
/// `Upper` if only `ub[j] == +inf` (`x_j = +M` there), `Both` if genuinely
/// free (`lb[j] == -inf` **and** `ub[j] == +inf` — presolve's own
/// documented residual case, a free variable reachable only through an
/// inequality row; see this module's own top-of-file docs), `None`
/// otherwise. Always `None` for a slack column (see this module's own docs
/// on why a slack is never M-flagged) — [`delta_of`] itself doesn't special-
/// case that; every call site building the full per-column `delta` array
/// forces slack entries to [`MSide::None`] directly instead (`j >= n_orig`
/// never reaches this function at all).
///
/// REVERTED (2026-09-20) from the paper's revised §4.2 `S`-restricted form
/// — every one-sided-unbounded column is flagged again, regardless of cost
/// sign, matching this function's pre-`S`-restriction behavior bit-for-bit.
/// Reason: restricting to `S` is provably minimal and matches the paper,
/// but empirically made the 73-problem Netlib total *worse* (7.9s -> 9.6s)
/// — `width_affine` stops treating a non-`S` column as BFRT-flip-eligible
/// once it's excluded, forcing a full pivot instead of a cheap flip for
/// every such column, and several degenerate instances (`pilot4`, `fit1p`,
/// `degen2`, `degen3`) needed measurably more of those real pivots as a
/// result — see `[[extended-dual-s-restriction]]` memory for the full
/// writeup. Restoring this exact `S`-restricted version later needs a
/// `cost` parameter back (gating which of `Lower`/`Upper`/`Both` a column's
/// own cost sign actually forces `crash` onto) — removed here since it was
/// already fully unused in the un-restricted form this reverted to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MSide {
    None,
    Lower,
    Upper,
    Both,
}

impl MSide {
    #[inline]
    fn has_lower(self) -> bool {
        matches!(self, MSide::Lower | MSide::Both)
    }

    #[inline]
    fn has_upper(self) -> bool {
        matches!(self, MSide::Upper | MSide::Both)
    }

    #[inline]
    fn is_flagged(self) -> bool {
        !matches!(self, MSide::None)
    }
}

fn delta_of(std: &StdForm, j: usize) -> MSide {
    match (std.lb[j] == f64::NEG_INFINITY, std.ub[j] == f64::INFINITY) {
        (true, true) => MSide::Both,
        (true, false) => MSide::Lower,
        (false, true) => MSide::Upper,
        (false, false) => MSide::None,
    }
}

/// The value nonbasic column `j` takes at `Lower` — total over every
/// structural column (`j < n_orig`): `Some(Affine1::new(0.0, -1.0))`
/// (`-M`) when `lb[j] == -inf` (regardless of whether `ub[j]` is *also*
/// infinite — a genuinely free column tracks this side exactly like a
/// one-sided one, [`hat_upper`] independently tracking its own upper side
/// the same way), otherwise the plain finite bound. `None` only for a
/// slack (`j >= n_orig`) whose own `lb` is somehow infinite — never
/// actually reachable (every slack's `lb` is `0`, `simplex.rs`'s own slack
/// construction), kept only so this function has the same total-except-
/// slack-infinities shape as [`hat_upper`] rather than asserting a
/// precondition it doesn't need to.
fn hat_lower(std: &StdForm, n_orig: usize, j: usize) -> Option<Affine1> {
    if std.lb[j] == f64::NEG_INFINITY {
        if j < n_orig {
            Some(Affine1::new(0.0, -1.0))
        } else {
            None
        }
    } else {
        Some(Affine1::new(std.lb[j], 0.0))
    }
}

/// The value nonbasic column `j` takes at `Upper` — mirrors [`hat_lower`]
/// exactly, tracking `M` on this side whenever `ub[j] == +inf` **and** `j`
/// is structural. `None` iff this is a *genuine* (non-`M`) infinity, only
/// possible for a `<=`/`>=` row's own slack (`j >= n_orig`): this module
/// treats that exactly like the classical bounded dual simplex already
/// does, never placing a nonbasic column there and never flipping through
/// it (see the BFRT walk in [`solve_lp_dual_extended`]).
fn hat_upper(std: &StdForm, n_orig: usize, j: usize) -> Option<Affine1> {
    if std.ub[j] == f64::INFINITY {
        if j < n_orig {
            Some(Affine1::new(0.0, 1.0))
        } else {
            None
        }
    } else {
        Some(Affine1::new(std.ub[j], 0.0))
    }
}

/// `None` iff `j` is nonbasic at `status` with a genuinely infinite bound
/// there — only possible for a slack (a `<=`/`>=` row's own, per
/// [`hat_lower`]/[`hat_upper`]'s own docs: every structural column tracks
/// both sides it needs, one-sided or genuinely free alike). Should never
/// happen given this module's own invariants ([`crash`] and every later
/// pivot only ever place a column at a side [`hat_lower`]/[`hat_upper`]
/// resolves to `Some`), but a
/// violation is a signal to give up cleanly (propagated via `?` up to
/// [`solve_lp_dual_extended`]'s `None` return, which `solve_lp_dual` reads
/// as "fall back to the classical path") rather than the outright process
/// abort a `.expect()`/`.unwrap()` here would cause across the PyO3
/// boundary — confirmed to actually occur on two real Netlib instances
/// (`perold`, `pilot4`) before this was a graceful `None` instead of a
/// panic, so this is not merely defensive.
#[inline]
fn nb_value_affine(cache: &ColCache, status: NbStatus, j: usize) -> Option<Affine1> {
    let r = match status {
        NbStatus::Lower => cache.lower[j],
        NbStatus::Upper => cache.upper[j],
    };
    if r.is_none() && std::env::var("ENOMOTO_DEBUG_EXT_ITERS").is_ok() {
        eprintln!("DEBUG_EXT_BAILOUT: nb_value_affine None at j={j} status={status:?}");
    }
    r
}

/// `B^{-1}` freshly factorized from the current `basis_pos` — no
/// incremental (Forrest-Tomlin) update, by design; see this module's own
/// docs, simplification (2).
///
/// `prev` is the factorization being replaced, when there is one: its
/// pivot order is reused (`sparse_lu::factorize_reusing`, this crate's
/// counterpart to HiGHS's `HFactor::rebuild()`) rather than searched for
/// again, since the basis it factorized differs from the current one only
/// by the columns the Forrest-Tomlin updates since then replaced. The
/// reuse re-checks threshold pivoting and fill at every step and falls
/// back to the full Markowitz search by itself when either fails, so
/// passing `prev` never changes *whether* a usable factorization results,
/// only how much work finding it costs.
fn refactorize(
    std: &StdForm,
    basis_pos: &[Option<usize>],
    prev: Option<&sparse_lu::FtLu>,
) -> Option<sparse_lu::FtLu> {
    let m = std.n_rows;
    let mut rows = vec![Vec::new(); m];
    // Column-driven, via `std.cols` — `nnz(A_B)` work instead of the
    // `nnz(A)` scan this replaced, which read every nonbasic entry only to
    // drop it. Visiting `j` in ascending order reproduces each row's entry
    // order exactly (see `super::freeze_std_matrices`), so the
    // factorization below — Markowitz tie-breaks included — is unchanged.
    for j in 0..std.n_total {
        if let Some(col) = basis_pos[j] {
            for &(i, v) in std.cols.col(j) {
                rows[i].push((col, v));
            }
        }
    }
    let r = sparse_lu::factorize_diagonal(m, &rows)
        .map(sparse_lu::FtLu::new)
        .or_else(|| sparse_lu::factorize_reusing(m, &rows, prev));
    if r.is_none() && std::env::var("ENOMOTO_DEBUG_EXT_ITERS").is_ok() {
        eprintln!("DEBUG_EXT_BAILOUT: refactorize returned None (singular basis)");
    }
    r
}

/// `‖A_B x_B - rhs‖`, using the *true* basis matrix (`std.rows`, filtered
/// to currently-basic columns via `basis_pos`) against an already-solved
/// `x_b` — `super::Tableau::basis_residual_norm`'s own check, adapted to
/// this module's `x_b` (indexed by basis *position*, not by variable)
/// instead of a full per-variable `x` array. Detects Forrest-Tomlin eta-
/// chain drift that a rejected-update/periodic-count trigger alone can
/// miss: `try_update` can keep *accepting* pivots whose accumulated
/// numerical error nonetheless makes `lu`'s own solves quietly diverge
/// from the true basis matrix — confirmed as a real (not hypothetical)
/// cause of non-termination on Netlib `agg`/`25fv47` once this module
/// stopped refactorizing every iteration.
fn residual_norm(std: &StdForm, basis_pos: &[Option<usize>], x_b: &[f64], rhs: &[f64]) -> f64 {
    // `A_B x_B` accumulated one *basic column* at a time, the same
    // `nnz(A_B)`-sized walk [`residual_norm_affine`] below already used —
    // this plain-`f64` form was the one still paying for a full `nnz(A)`
    // scan. One `O(m)` buffer; ascending `j` keeps each row's summation
    // order (and so its rounding) identical to the row-driven form.
    let mut val = vec![0.0; std.n_rows];
    for j in 0..std.n_total {
        if let Some(pos) = basis_pos[j] {
            sparse_axpy_dense(x_b[pos], std.cols.col(j), &mut val);
        }
    }
    let mut resid_sq = 0.0f64;
    for i in 0..std.n_rows {
        let r = val[i] - rhs[i];
        resid_sq += r * r;
    }
    resid_sq.sqrt()
}

/// Combined two-channel version of [`residual_norm`], for the `x_B(M)`
/// drift check's `Affine1` pair (`x_b_base`/`x_b_slope` against
/// `rhs_base`/`rhs_slope`): iterates once over the *basic* columns
/// (`basis[pos]`'s own `std.cols.col(j)`, `nnz(A_B)`-sized) instead of
/// `residual_norm`'s own twice-over-every-row-of-`A`, `2*nnz(A)`-sized,
/// scan. `nnz(A_B)` is a small fraction of `nnz(A)` on problems like
/// Netlib `greenbea` (most of `A`'s columns are nonbasic at any one time).
/// `scratch_base`/`scratch_slope` are `m`-length buffers owned by the
/// caller (this module's own "sized once, before the loop, reused every
/// iteration" convention — see the buffers declared alongside `x_b_base`
/// in [`solve_lp_dual_extended`]) and are fully overwritten here before
/// use, so their entering contents don't matter.
fn residual_norm_affine(
    std: &StdForm,
    basis: &[usize],
    x_b_base: &[f64],
    x_b_slope: &[f64],
    rhs_base: &[f64],
    rhs_slope: &[f64],
    scratch_base: &mut [f64],
    scratch_slope: &mut [f64],
) -> (f64, f64) {
    scratch_base.iter_mut().for_each(|v| *v = 0.0);
    scratch_slope.iter_mut().for_each(|v| *v = 0.0);
    for (pos, &j) in basis.iter().enumerate() {
        let (b, s) = (x_b_base[pos], x_b_slope[pos]);
        if s == 0.0 {
            // Same exact-no-op argument as `compute_rhs_affine`'s own
            // slope skip: `+0.0`-seeded sums never become `-0.0`, so adding
            // `v * 0.0 = ±0.0` changes nothing.
            for &(i, v) in std.cols.col(j) {
                scratch_base[i] += v * b;
            }
            continue;
        }
        for &(i, v) in std.cols.col(j) {
            scratch_base[i] += v * b;
            scratch_slope[i] += v * s;
        }
    }
    let mut resid_base_sq = 0.0f64;
    let mut resid_slope_sq = 0.0f64;
    for i in 0..std.n_rows {
        let rb = scratch_base[i] - rhs_base[i];
        let rs = scratch_slope[i] - rhs_slope[i];
        resid_base_sq += rb * rb;
        resid_slope_sq += rs * rs;
    }
    (resid_base_sq.sqrt(), resid_slope_sq.sqrt())
}

/// `B x_B(M) = base + slope*M`'s own right-hand side (Lemma 4.1's `b - N
/// x_N`, split into its `M`-independent and `M`-coefficient parts) — an
/// `O(nnz(A))` pass over every nonbasic column, the same cost
/// `super::Tableau::compute_rhs` pays for the classical method's own
/// (plain-`f64`) `x_B`. Deliberately factored out from the two solves that
/// used to always follow it immediately (`solve_x_b`, below): the main
/// loop in [`solve_lp_dual_extended`] now maintains `x_B(M)` incrementally
/// pivot-to-pivot (see that function's own docs) and calls this — the full
/// recompute — only at its own periodic drift-check/resync cadence, not
/// every iteration; `finish` still wants the one-shot `solve_x_b` below
/// exactly as before.
fn compute_rhs_affine(std: &StdForm, cache: &ColCache, nb_status: &[Option<NbStatus>]) -> Option<(Vec<f64>, Vec<f64>)> {
    let m = std.n_rows;
    let mut rhs_base = std.b.clone();
    let mut rhs_slope = vec![0.0; m];
    for j in 0..std.n_total {
        let Some(status) = nb_status[j] else { continue };
        let val = nb_value_affine(cache, status, j)?;
        if val.base == 0.0 && val.slope == 0.0 {
            continue;
        }
        if val.slope == 0.0 {
            // Finite-side column (the vast majority): its slope-channel
            // contribution is `v * 0.0 = ±0.0`, and subtracting a signed
            // zero from an `rhs_slope` entry is an exact no-op — every entry
            // starts at `+0.0` and round-to-nearest subtraction never
            // produces `-0.0` from a `+0.0`-seeded running difference — so
            // skipping it is bit-for-bit identical.
            for &(i, v) in std.cols.col(j) {
                rhs_base[i] -= v * val.base;
            }
            continue;
        }
        for &(i, v) in std.cols.col(j) {
            rhs_base[i] -= v * val.base;
            rhs_slope[i] -= v * val.slope;
        }
    }
    Some((rhs_base, rhs_slope))
}

/// `x_B(M) = base + slope*M` (Lemma 4.1), as two independent FTRAN solves
/// against the *same* factorization — the "M-dependent and M-independent
/// parts are the same linear operation, run twice" structure this whole
/// module is built around (nothing here is symbolic linear algebra; it is
/// two ordinary numeric solves). One-shot: allocates and solves dense,
/// unlike the main loop's own reused-buffer `solve_into` pair — fine for
/// `finish`'s own single call, not reused mid-loop.
fn solve_x_b(std: &StdForm, lu: &sparse_lu::FtLu, nb_status: &[Option<NbStatus>], cache: &ColCache) -> Option<(Vec<f64>, Vec<f64>)> {
    let (rhs_base, rhs_slope) = compute_rhs_affine(std, cache, nb_status)?;
    Some((lu.solve(&rhs_base), lu.solve(&rhs_slope)))
}

/// Basic row `i`'s own deviation outside its bounds, as `(d_dir, dev)` —
/// `d_dir == 1` for a deviation *below* the lower bound (the row wants to
/// increase), `-1` for *above* the upper bound (wants to decrease), or
/// `None` iff row `i` is feasible. Factored out of the chuzr scan so
/// [`InfeasibleRows`]'s own feasibility predicate ([`row_infeasible_affine`])
/// and the scan's own per-row scoring loop in [`solve_lp_dual_extended`]
/// share one implementation rather than two copies that could drift apart
/// on which side "infeasible" means.
#[inline]
fn row_deviation(cache: &ColCache, basis: &[usize], x_b_base: &[f64], x_b_slope: &[f64], noise_feasible: &[bool], i: usize) -> Option<(i32, Affine1)> {
    let bv = basis[i];
    // A row already proven "infeasible only within noise" at the
    // Eligible=empty juncture (see [`solve_lp_dual_extended`]'s own
    // `noise_feasible` marking, `super::solve_lp_dual_on`'s own
    // identically-named mechanism) is treated as feasible from then on,
    // the same as that classical method's own `chuzr_scan` does — rather
    // than re-selected (and re-failing chuzc1) every subsequent
    // iteration.
    deviation_core(noise_feasible[bv], cache.lower[bv], cache.upper[bv], x_b_base[i], x_b_slope[i])
}

/// [`row_deviation`]'s own arithmetic, given the basic variable's
/// `noise_feasible` flag and cached bounds directly — shared with
/// [`RowBounds::deviation`] (which reads them from row-indexed arrays
/// instead of through `basis[i]`) so both are one implementation.
#[inline(always)]
fn deviation_core(noise: bool, lower: Option<Affine1>, upper: Option<Affine1>, x_base: f64, x_slope: f64) -> Option<(i32, Affine1)> {
    if noise {
        return None;
    }
    let x_bi = Affine1::new(x_base, x_slope);
    // `lower` is `None` iff the basic variable's lower bound is a genuine
    // (non-`M`) infinity (a structural `(J_L∪J_U)\S` column at its true
    // `-inf` side — [`hat_lower`]'s own docs) — never violated by any
    // finite `x_bi`, mirroring `upper`'s own `and_then` just below
    // exactly.
    let dev_minus = lower.and_then(|hat_l| {
        let v_minus = hat_l.sub(x_bi);
        v_minus.gt_zero().then_some(v_minus)
    });
    let dev_plus = upper.and_then(|hat_u| {
        let v_plus = x_bi.sub(hat_u);
        v_plus.gt_zero().then_some(v_plus)
    });
    match (dev_minus, dev_plus) {
        (Some(vm), Some(vp)) => Some(if vm.cmp_lex(&vp) == std::cmp::Ordering::Greater { (1i32, vm) } else { (-1i32, vp) }),
        (Some(vm), None) => Some((1i32, vm)),
        (None, Some(vp)) => Some((-1i32, vp)),
        (None, None) => None,
    }
}

/// [`row_deviation`], collapsed to the plain boolean [`InfeasibleRows`]
/// wants — every call site already has every argument `row_deviation`
/// needs on hand (the same reason it's a free function, not a method on
/// some larger state struct: none of the arguments' owners are the same
/// object across every call site — see [`solve_lp_dual_extended`]'s own
/// `InfeasibleRows::set`/`rebuild` call sites).
#[inline]
#[cfg_attr(not(debug_assertions), allow(dead_code))]
fn row_infeasible_affine(cache: &ColCache, basis: &[usize], x_b_base: &[f64], x_b_slope: &[f64], noise_feasible: &[bool], i: usize) -> bool {
    row_deviation(cache, basis, x_b_base, x_b_slope, noise_feasible, i).is_some()
}

/// [`row_deviation`]'s own result for every row currently in
/// [`InfeasibleRows`], cached at the moment that row's membership was last
/// (re)decided — the extended counterpart of HiGHS's `work_infeasibility`
/// array (`HEkkDualRHS::updatePrimal`/`updateInfeasList`), which CHUZR
/// reads instead of re-deriving each row's primal infeasibility from
/// `baseValue`/`baseLower`/`baseUpper` on every scan. Every input
/// `row_deviation` reads (`x_b_base[i]`, `x_b_slope[i]`, `basis[i]`,
/// `noise_feasible[basis[i]]`) only ever changes at a site that already
/// re-decides row `i`'s membership (this loop's own `InfeasibleRows::set`/
/// `rebuild` call sites, all of which now go through [`refresh_row`]/
/// [`rebuild_rows`]), so for every row in the pool the cached value is
/// bit-for-bit what a fresh `row_deviation` call would return. Entries for
/// rows *not* in the pool are stale and never read.
struct RowDevCache {
    dir: Vec<i32>,
    dev: Vec<Affine1>,
}

impl RowDevCache {
    fn new(m: usize) -> Self {
        RowDevCache { dir: vec![0; m], dev: vec![Affine1::ZERO; m] }
    }
}

/// The current basic variable's cached bounds ([`ColCache`]) and
/// `noise_feasible` flag, indexed by basis *row* rather than by variable —
/// HiGHS's own `baseLower`/`baseUpper` arrays (`HEkk::info_`), kept for the
/// same reason: every primal-feasibility re-check after an `x_B` update
/// ([`refresh_row`]) walks rows in ascending order, and reading bounds
/// through `basis[i]` turned each of those into three random accesses into
/// `n_total`-sized arrays (`cache.lower`/`cache.upper`/`noise_feasible`)
/// instead of sequential ones. Must be updated at every `basis[i]` write
/// ([`RowBounds::assign`]) and every `noise_feasible` write for a basic
/// variable ([`RowBounds::mark_noise`]); [`RowBounds::deviation`] is then
/// bit-for-bit [`row_deviation`] (checked under `debug_assertions`).
struct RowBounds {
    lower: Vec<Option<Affine1>>,
    upper: Vec<Option<Affine1>>,
    noise: Vec<bool>,
}

impl RowBounds {
    fn new(cache: &ColCache, basis: &[usize], noise_feasible: &[bool]) -> Self {
        RowBounds {
            lower: basis.iter().map(|&j| cache.lower[j]).collect(),
            upper: basis.iter().map(|&j| cache.upper[j]).collect(),
            noise: basis.iter().map(|&j| noise_feasible[j]).collect(),
        }
    }

    /// Row `i` is now occupied by variable `j`.
    #[inline]
    fn assign(&mut self, i: usize, j: usize, cache: &ColCache, noise_feasible: &[bool]) {
        self.lower[i] = cache.lower[j];
        self.upper[i] = cache.upper[j];
        self.noise[i] = noise_feasible[j];
    }

    /// Row `i`'s current basic variable was just marked `noise_feasible`.
    #[inline]
    fn mark_noise(&mut self, i: usize) {
        self.noise[i] = true;
    }

    #[inline]
    fn deviation(&self, x_b_base: &[f64], x_b_slope: &[f64], i: usize) -> Option<(i32, Affine1)> {
        deviation_core(self.noise[i], self.lower[i], self.upper[i], x_b_base[i], x_b_slope[i])
    }
}

/// Re-decides row `i`'s [`InfeasibleRows`] membership exactly like
/// `infeasible_rows.set(i, row_infeasible_affine(..))` did, additionally
/// caching the deviation itself in `dc` ([`RowDevCache`]'s own docs).
#[inline]
#[allow(clippy::too_many_arguments)]
fn refresh_row(rows: &mut InfeasibleRows, dc: &mut RowDevCache, rb: &RowBounds, x_b_base: &[f64], x_b_slope: &[f64], i: usize) {
    match rb.deviation(x_b_base, x_b_slope, i) {
        Some((dir, dev)) => {
            dc.dir[i] = dir;
            dc.dev[i] = dev;
            rows.set(i, true);
        }
        None => rows.set(i, false),
    }
}

/// [`refresh_row`]'s full-rebuild counterpart (`InfeasibleRows::rebuild`).
#[allow(clippy::too_many_arguments)]
fn rebuild_rows(rows: &mut InfeasibleRows, dc: &mut RowDevCache, m: usize, rb: &RowBounds, x_b_base: &[f64], x_b_slope: &[f64]) {
    rows.rebuild(m, |i| match rb.deviation(x_b_base, x_b_slope, i) {
        Some((dir, dev)) => {
            dc.dir[i] = dir;
            dc.dev[i] = dev;
            true
        }
        None => false,
    });
}

/// Plain-`f64` counterpart of [`row_deviation`]/[`compute_rhs_affine`]/
/// [`width_affine`], for [`polish_with_true_bounds`] — once cleanup has run,
/// no nonbasic column is ever `M`-flagged anymore, so this phase operates
/// directly on `std.lb`/`std.ub` with a single channel throughout, exactly
/// like `super::solve_lp_dual_on`'s own `row_infeasible`/`chuzr_scan`/
/// `compute_rhs`. Kept as free functions taking `basis`/`x_b` directly
/// (matching this module's own convention above) rather than routed through
/// `super::Tableau`, since `polish_with_true_bounds` — like the rest of this
/// module — never constructs one.
///
/// Returns `(d_dir, magnitude)`: `d_dir == 1` for a deviation *below* `lb`
/// (row wants to increase), `-1` for *above* `ub`. The tolerance is scaled
/// by the row's own computed value (`super::PRIMAL_FEAS_TOL * xi.abs().max(1.0)`),
/// not a bare `PRIMAL_FEAS_TOL`, matching this function's own long-standing
/// convention (a flat tolerance is simultaneously too strict on large-
/// magnitude rows and too loose on tiny ones — `PRIMAL_FEAS_TOL`'s own docs).
fn row_deviation_plain(std: &StdForm, basis: &[usize], x_b: &[f64], noise_feasible: &[bool], i: usize) -> Option<(i32, f64)> {
    let bv = basis[i];
    if noise_feasible[bv] {
        return None;
    }
    let xi = x_b[i];
    let feas_tol = super::PRIMAL_FEAS_TOL * xi.abs().max(1.0);
    if xi < std.lb[bv] - feas_tol {
        Some((1, std.lb[bv] - xi))
    } else if xi > std.ub[bv] + feas_tol {
        Some((-1, xi - std.ub[bv]))
    } else {
        None
    }
}

/// [`row_deviation_plain`], collapsed to the plain boolean [`InfeasibleRows`]
/// wants — same reason [`row_infeasible_affine`] is a free function, not a
/// method (see its own docs).
fn row_infeasible_plain(std: &StdForm, basis: &[usize], x_b: &[f64], noise_feasible: &[bool], i: usize) -> bool {
    row_deviation_plain(std, basis, x_b, noise_feasible, i).is_some()
}

/// `b - N x_N`, plain-`f64` — [`compute_rhs_affine`]'s single-channel
/// counterpart, called once as `polish_with_true_bounds`'s own seed and
/// again only at that phase's periodic resync (never once per iteration —
/// see that function's own docs on why the previous version's unconditional
/// per-iteration recompute here was the dominant cost on larger Netlib
/// instances). Every nonbasic column here carries a genuinely finite value
/// at its own current bound: cleanup has already removed every `M`-flagged
/// one, and (per this module's own invariants) no nonbasic column is ever
/// placed at a genuine one-sided infinity (a `<=`/`>=` row's own slack stays
/// at `Lower == 0`, never `Upper`).
fn compute_rhs_plain(std: &StdForm, nb_status: &[Option<NbStatus>]) -> Vec<f64> {
    let mut rhs = std.b.clone();
    for j in 0..std.n_total {
        let Some(status) = nb_status[j] else { continue };
        let val = match status {
            NbStatus::Lower => std.lb[j],
            NbStatus::Upper => std.ub[j],
        };
        if val == 0.0 {
            continue;
        }
        for &(i, v) in std.cols.col(j) {
            rhs[i] -= v * val;
        }
    }
    rhs
}

/// `ub[j] - lb[j]`, plain-`f64` — [`width_affine`]'s counterpart for
/// [`polish_with_true_bounds`], where no column is ever `M`-flagged anymore
/// so the general case (a slack's own genuine one-sided infinity aside) is
/// simply the true bound gap. `f64::INFINITY` when not finite, matching
/// `width_affine`'s own `None`-via-`is_finite()` convention at call sites.
fn width_plain(std: &StdForm, j: usize) -> f64 {
    std.ub[j] - std.lb[j]
}

/// `d = c - A^T B^{-T} c_B` recomputed from scratch against the current
/// factorization — the extended counterpart of `super::fresh_d` (basic
/// columns get `0`). Called once at the start and after every
/// refactorization, exactly like the classical loop: the incremental
/// `d[j] -= theta_d * a_p[j]` update propagates any error linearly through
/// every later pivot (`e' = e - (e_q / alpha_q) * a_p`), and measured on
/// Netlib `25fv47` the maintained `d` had drifted by up to `2.4e2` from the
/// true reduced costs before this resync existed — with up to 543 columns
/// dual-infeasible under the true values while the maintained `d` reported
/// none — so chuzc was ranking entering columns on stale numbers.
fn fresh_d_into(std: &StdForm, lu: &sparse_lu::FtLu, basis: &[usize], basis_pos: &[Option<usize>], active_cost: &[f64], cb_buf: &mut [f64], scratch: &mut [f64], y_buf: &mut [f64], d: &mut [f64]) {
    for i in 0..std.n_rows {
        cb_buf[i] = active_cost[basis[i]];
    }
    lu.solve_transpose_into(cb_buf, scratch, y_buf);
    for j in 0..std.n_total {
        if basis_pos[j].is_some() {
            d[j] = 0.0;
            continue;
        }
        let mut dj = active_cost[j];
        for &(i, v) in std.cols.col(j) {
            dj -= v * y_buf[i];
        }
        d[j] = dj;
    }
}

/// Sign-of-cost dual-feasible crash (Proposition 4.3): unlike
/// [`super::Tableau::crash_dual_feasible`], this never needs both of a
/// column's bounds to be finite — [`hat_lower`]/[`hat_upper`] are total
/// (module-docs) precisely because every structural column has at least
/// one finite side and this module represents the other symbolically.
///
/// Takes `cost` as a parameter (the *perturbed* cost, `super::perturb_costs`'
/// own output) rather than reading `std.c` directly — matching
/// `super::Tableau::crash_dual_feasible`'s own convention exactly:
/// perturbation is built to never flip which side of dual feasibility a
/// sign-based placement lands on, so this is the same placement either
/// way, but using the same cost vector the incremental reduced-cost
/// maintenance (`d`, initialized to this same `cost.clone()`) is built on
/// keeps the two consistent by construction rather than by coincidence.
fn crash(std: &StdForm, cost: &[f64], n_orig: usize) -> Vec<Option<NbStatus>> {
    let mut nb_status = vec![None; std.n_total];
    for j in 0..n_orig {
        nb_status[j] = Some(if cost[j] >= -TOL { NbStatus::Lower } else { NbStatus::Upper });
    }
    nb_status
}

/// EXPERIMENTAL crash refinement: a nonbasic column with *exactly* zero
/// true cost and both bounds genuinely finite (`boxed`) has reduced cost
/// exactly `0` at the all-slack basis (`y = 0` there, so `d[j] = c[j]`)
/// regardless of which bound [`crash`] parks it at — dual feasibility
/// never prefers `Lower` over `Upper` for such a column, so [`crash`]'s
/// own tie-break (`cost[j] >= -TOL`, which `perturb_costs` always nudges
/// positive for a zero-cost boxed column, `perturb_costs`'s own docs)
/// picks `Lower` unconditionally with no primal-feasibility rationale at
/// all — pure coincidence of the tie-break's direction, not a considered
/// choice. This instead sweeps those columns once, in column-index
/// order, and places each one at whichever side leaves less violation
/// (summed over the rows it touches) on the *plain* row residual
/// (`b` minus every nonbasic column's contribution so far) — cheap
/// (`O(nnz)` over just the flexible columns' own entries) since the
/// all-slack basis makes `x_B[i] = residual[i]` directly, no LU solve
/// needed yet. A column touching zero rows costs nothing either way and
/// is left at `Lower` (its `viol_lo`/`viol_hi` both `0`, so the
/// `>=`-based tie-break in the call site keeps the original placement,
/// matching `crash`'s own convention).
///
/// A flipped column's own entry in `active_cost` (the `perturb_costs`
/// output `crash` itself was built from) is negated in lockstep so the
/// incrementally-maintained reduced cost `d` this loop seeds from stays
/// consistent with the new placement (`Upper` needs `d[j] <= 0`; the
/// perturbation for a zero-cost boxed column is always the small
/// positive `xpert` `crash`'s own doc comment names, so negating it is
/// exactly the mirror-image nudge `perturb_costs` would have produced
/// had its own `pc[j] >= 0.0` branch gone the other way) — otherwise
/// every downstream reduced-cost-sign assumption (chuzc1's `hat_alpha`
/// sign convention, the stall/anti-cycling check) would see a column
/// sitting at `Upper` with a positive `d[j]`, a genuine dual-feasibility
/// violation this function must never introduce.
///
/// **Confirmed not to help (73-problem Netlib sweep, `netlib-benchmark-workflow`'s
/// own methodology)** — gated behind `ENOMOTO_CRASH_ZERO_COST_PLACEMENT`
/// (default: **off**) rather than removed outright, as a documented
/// negative result. Reducing the *count* of primal-infeasible rows
/// immediately after crash is not the same objective as reducing the
/// dual simplex's own total iteration count: which *specific* rows are
/// infeasible, and by how much, drives the whole subsequent pivot
/// sequence (DSE/Devex weighting, BFRT batching, tie-breaking), and a
/// locally-greedy placement can steer that sequence somewhere worse even
/// while genuinely lowering the row-violation count it was greedy over.
/// Measured directly: `greenbea` 7564->8705 iterations, `greenbeb`
/// 10528->13038, `maros` 1435->1660, `nesm` 1383->1447 (all *worse*, not
/// better), and `perold` collapsed into a pathological pivot sequence —
/// 0.32s at baseline to over 54s (still not done) with this enabled, the
/// same "discard-and-retry" failure shape `ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL`'s
/// own docs name on `pilot4`. Left wired up only so a future, smarter
/// version of this idea (e.g. weighting the greedy choice by how it
/// affects the *specific* rows DSE/BFRT will actually pivot on next,
/// rather than a flat violation-count sum) has a tested starting point
/// to build from, not because this version is a candidate default.
fn refine_zero_cost_placement(std: &StdForm, active_cost: &mut [f64], nb_status: &mut [Option<NbStatus>], n_orig: usize) {
    let n_rows = std.n_rows;
    let mut residual = std.b.clone();
    let mut flexible: Vec<usize> = Vec::new();
    for j in 0..n_orig {
        let lo = std.lb[j];
        let hi = std.ub[j];
        if lo == hi {
            continue; // fixed column: contributes the same constant regardless, skip entirely
        }
        if std.c[j] == 0.0 && lo.is_finite() && hi.is_finite() {
            flexible.push(j);
            continue; // decided by the sweep below, not subtracted yet
        }
        let x = match nb_status[j] {
            Some(NbStatus::Lower) => lo,
            Some(NbStatus::Upper) => hi,
            None => continue,
        };
        if x != 0.0 {
            for &(i, a) in std.cols.col(j) {
                residual[i] -= a * x;
            }
        }
    }
    if flexible.is_empty() {
        return;
    }
    let debug = std::env::var("ENOMOTO_DEBUG_EXT_CRASH").is_ok();
    let violation = |v: f64, lo: f64, hi: f64| -> f64 {
        if v < lo { lo - v } else if v > hi { v - hi } else { 0.0 }
    };
    let infeasible_count = |residual: &[f64]| -> usize {
        (0..n_rows)
            .filter(|&i| violation(residual[i], std.lb[n_orig + i], std.ub[n_orig + i]) > TOL)
            .count()
    };
    // Fair baseline for the debug printout: what `crash`'s own unconditional
    // `Lower` placement would have left every flexible column at, not the
    // (misleading) count with them contributing nothing at all.
    let before = if debug {
        let mut baseline = residual.clone();
        for &j in &flexible {
            let lo = std.lb[j];
            if lo != 0.0 {
                for &(i, a) in std.cols.col(j) {
                    baseline[i] -= a * lo;
                }
            }
        }
        Some(infeasible_count(&baseline))
    } else {
        None
    };
    let n_flexible = flexible.len();
    let mut flipped = 0usize;
    for j in flexible {
        let lo = std.lb[j];
        let hi = std.ub[j];
        let col = std.cols.col(j);
        let mut viol_lo = 0.0f64;
        let mut viol_hi = 0.0f64;
        for &(i, a) in col {
            let bi_lo = std.lb[n_orig + i];
            let bi_hi = std.ub[n_orig + i];
            viol_lo += violation(residual[i] - a * lo, bi_lo, bi_hi);
            viol_hi += violation(residual[i] - a * hi, bi_lo, bi_hi);
        }
        let (status, x) = if viol_hi < viol_lo { (NbStatus::Upper, hi) } else { (NbStatus::Lower, lo) };
        if matches!(status, NbStatus::Upper) {
            flipped += 1;
            active_cost[j] = -active_cost[j];
        }
        nb_status[j] = Some(status);
        if x != 0.0 {
            for &(i, a) in col {
                residual[i] -= a * x;
            }
        }
    }
    if debug {
        eprintln!(
            "DEBUG_EXT_CRASH: flexible_cols={n_flexible} flipped_to_upper={flipped} infeasible_rows_before={} infeasible_rows_after={}",
            before.unwrap(),
            infeasible_count(&residual)
        );
    }
}

/// One candidate in the entering-column ratio test / BFRT walk.
#[derive(Clone, Copy)]
struct Cand {
    j: usize,
    hat_alpha: f64,
    ratio: f64,
}

// Ascending `(ratio, j)` order — the same order `candidates.sort_by(|a, b|
// a.ratio.total_cmp(&b.ratio).then_with(|| a.j.cmp(&b.j)))` used to produce
// directly, now backing a `BinaryHeap<Reverse<Cand>>` so the main loop's
// chuzc1/BFRT walk (see its own comment) can pop this same order lazily
// instead of sorting the whole candidate pool upfront. `j` breaks ties so
// the pop order on a tied `ratio` doesn't depend on push/heapify order.
impl PartialEq for Cand {
    fn eq(&self, other: &Self) -> bool {
        self.ratio == other.ratio && self.j == other.j
    }
}
impl Eq for Cand {}
impl PartialOrd for Cand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Cand {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ratio.total_cmp(&other.ratio).then_with(|| self.j.cmp(&other.j))
    }
}

/// Trial BTRAN + PRICE + (BFRT-free) ratio test for a candidate leaving row
/// `row`, without committing any basis/nonbasic-status change — the
/// "greatest improvement" chuzr escalation's own building block (see that
/// mechanism's own docs, next to `stall_count`'s declaration). Returns the
/// smallest `hat_c_j / |hat_alpha_j|` ratio over row `row`'s own Eligible
/// set (`None` iff Eligible is empty there, mirroring the main loop's own
/// `Eligible = empty` case — such a row can never actually be pivoted on).
/// Deliberately skips the full BFRT capacity walk the real pivot commit
/// does: this ratio alone already lets the caller estimate
/// `dev_i.scale(ratio)` as this row's real (single-pivot) objective gain,
/// which is what picks *which* row to commit to — the chosen row's own
/// commit, further down in the main loop, redoes this same PRICE step
/// (unavoidably — this function's own scratch buffers are cleared before
/// returning) and always uses the *full* BFRT walk, so no result computed
/// here is ever reused for the actual pivot.
///
/// `lu_scratch`/`rho`/`a_p`/`touched`/`touched_cols` are the caller's own
/// reusable scratch buffers (the same ones the main loop's real PRICE step
/// uses) — guaranteed all-zero/empty on entry and restored to that state
/// before this function returns, so interleaving trial calls for several
/// candidate rows with each other, and with the main loop's own later use
/// of the same buffers for the row it actually commits to, is safe.
#[allow(clippy::too_many_arguments)]
fn trial_row_ratio(
    std: &StdForm,
    lu: &sparse_lu::FtLu,
    nb_status: &[Option<NbStatus>],
    d: &[f64],
    d_dir: i32,
    lu_scratch: &mut [f64],
    rho: &mut [f64],
    a_p: &mut [f64],
    touched: &mut [bool],
    touched_cols: &mut Vec<usize>,
    row: usize,
) -> Option<f64> {
    lu.solve_transpose_unit(row, lu_scratch, rho);

    for i in 0..std.n_rows {
        let rv = rho[i];
        if rv.abs() <= TOL {
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
            a_p[j] += rv * v;
        }
    }

    let mut best_ratio = f64::INFINITY;
    for &j in touched_cols.iter() {
        let Some(status) = nb_status[j] else { continue };
        let alpha_j = a_p[j];
        if alpha_j.abs() <= TOL {
            continue;
        }
        let sigma = match status {
            NbStatus::Lower => 1.0,
            NbStatus::Upper => -1.0,
        };
        let hat_alpha = sigma * alpha_j;
        if (d_dir as f64) * hat_alpha >= 0.0 {
            continue;
        }
        let hat_c = (sigma * d[j]).max(0.0);
        let ratio = hat_c / hat_alpha.abs();
        if ratio < best_ratio {
            best_ratio = ratio;
        }
    }

    for &j in touched_cols.iter() {
        a_p[j] = 0.0;
        touched[j] = false;
    }
    touched_cols.clear();

    best_ratio.is_finite().then_some(best_ratio)
}

/// `hat_u_j(M) - hat_l_j(M)`, as an affine function of `M` — `None` iff
/// column `j`'s width is a *genuine* infinity (a slack whose own `<=`/`>=`
/// row leaves it one-sided), matching the classical method's own
/// `!width.is_finite()` BFRT guard (`simplex.rs`'s own docs) exactly:
/// such a column can never be flipped, only ever pivoted on directly.
/// Computed directly from the already-cached [`hat_lower`]/[`hat_upper`]
/// rather than re-deriving `lb`/`ub` itself — this is what makes a
/// genuinely free column (both sides `M`-tracked) fall out correctly with
/// no special-casing at all: `hat_upper - hat_lower` there is
/// `(0 + 1*M) - (0 - 1*M) = 2*M` (`Affine1::new(0.0, 2.0)`), exactly the
/// paper's general `w_j + s_j*M` with `s_j = 2` for a column `M`-tracked on
/// *both* sides — versus a one-sided column's `s_j = 1` (one `M` term
/// minus one exactly-`0` finite term, post-shift).
fn width_affine(lower: Option<Affine1>, upper: Option<Affine1>) -> Option<Affine1> {
    match (lower, upper) {
        (Some(lo), Some(hi)) => Some(hi.sub(lo)),
        _ => None,
    }
}

/// Per-column cache of [`hat_lower`]/[`hat_upper`]/[`width_affine`], built
/// once before [`solve_lp_dual_extended`]'s main loop starts and read for
/// the rest of that call (`finish`/`polish_with_true_bounds`'s own tail
/// included). All three depend only on `std.lb[j]`/`std.ub[j]` and `n_orig`,
/// none of which ever changes once this module's `std` is built —
/// critically, `nb_status[j]` (which *does* change every pivot) plays no
/// part in any of the three, so nothing here ever needs invalidating
/// mid-solve. Exists because [`row_deviation`] and the BFRT candidate walk
/// both re-derive these same `Affine1`s (and re-index `std.lb`/`std.ub`) on
/// every row/candidate they visit — real cost in the chuzr/chuzc hot paths
/// at scale, for a value that was already fully determined before the loop
/// even started.
struct ColCache {
    lower: Vec<Option<Affine1>>,
    upper: Vec<Option<Affine1>>,
    width: Vec<Option<Affine1>>,
}

impl ColCache {
    fn build(std: &StdForm, n_orig: usize) -> Self {
        let n_total = std.n_total;
        let lower: Vec<Option<Affine1>> = (0..n_total).map(|j| hat_lower(std, n_orig, j)).collect();
        let upper: Vec<Option<Affine1>> = (0..n_total).map(|j| hat_upper(std, n_orig, j)).collect();
        let width: Vec<Option<Affine1>> = (0..n_total).map(|j| width_affine(lower[j], upper[j])).collect();
        ColCache { lower, upper, width }
    }
}

/// The extended dual simplex's own main phase (paper \S4.2-\S4.7): reaches
/// a state that is primal feasible for the M-truncated problem while
/// never fixing a numeric `M`, then hands off to [`finish`] for the
/// termination classification, cleanup (\S4.5 end), and handoff to the
/// classical method.
pub fn solve_lp_dual_extended(std: &StdForm) -> Option<SimplexResult> {
    let n_total = std.n_total;
    let m = std.n_rows;
    let n_orig = n_total - m;

    // Cost perturbation (`super::perturb_costs`), reused directly rather
    // than reimplemented — the same anti-degeneracy mechanism
    // `polish_with_true_bounds` already relies on, now applied to this
    // phase's own reduced-cost maintenance too (`d`, below) and its own
    // crash (`crash` takes this same vector, not `std.c`, for exactly the
    // reason its own docs give).
    let mut active_cost = super::perturb_costs(std);
    // Slack columns keep their exact (zero) cost: `perturb_costs` nudges
    // every one-sided column, slacks included, which makes `y = B^-T c_B`
    // nonzero even at the all-slack basis — so the true reduced cost of a
    // structural column then differs from `active_cost[j]`, and [`crash`]
    // (which places by the sign of `active_cost`) starts dual-*infeasible*
    // (70 columns on Netlib `perold`). HiGHS perturbs logicals only at a
    // `1e-12` scale for the same reason. With slacks unperturbed, `y = 0`
    // at the start and the crash is dual feasible by construction.
    for j in n_orig..n_total {
        active_cost[j] = std.c[j];
    }

    // `delta[j]` (see [`delta_of`]'s own docs — currently the reverted,
    // un-`S`-restricted form: every one-sided-unbounded or genuinely free
    // structural column is flagged, regardless of `active_cost`'s sign).
    let delta: Vec<MSide> = (0..n_total).map(|j| if j < n_orig { delta_of(std, j) } else { MSide::None }).collect();
    let cache = ColCache::build(std, n_orig);
    // `cache.width[j].is_none()`, one byte per column — chuzc1's stopper
    // test reads it for every candidate.
    let width_inf: Vec<bool> = cache.width.iter().map(|w| w.is_none()).collect();

    if m == 0 {
        // No constraints at all (mirrors `solve_lp_on`'s own `n_rows == 0`
        // shortcut): every column sits wherever its (perturbed) cost sign
        // favors, unconditionally — unbounded iff that favored side is a
        // genuine `M` side with nonzero cost. Uses `active_cost`, not raw
        // `std.c`, so this decision agrees with `delta`/`cache` above on
        // every zero-cost tie-break (see this function's own docs just
        // above).
        let mut x = vec![0.0; n_total];
        for j in 0..n_orig {
            let status = if active_cost[j] >= -TOL { NbStatus::Lower } else { NbStatus::Upper };
            let val = nb_value_affine(&cache, status, j)?;
            if val.slope != 0.0 && active_cost[j].abs() > TOL {
                return Some(SimplexResult { status: Status::Unbounded, x: None });
            }
            x[j] = val.base;
        }
        return Some(SimplexResult { status: Status::Optimal, x: Some(x) });
    }

    let mut basis: Vec<usize> = (n_orig..n_total).collect();
    let mut basis_pos: Vec<Option<usize>> = vec![None; n_total];
    for (col, &v) in basis.iter().enumerate() {
        basis_pos[v] = Some(col);
    }
    let mut nb_status = crash(std, &active_cost, n_orig);
    // EXPERIMENTAL, confirmed not to help by default — see
    // `refine_zero_cost_placement`'s own docs for the measured regressions.
    if std::env::var("ENOMOTO_CRASH_ZERO_COST_PLACEMENT").is_ok_and(|v| v != "0") {
        refine_zero_cost_placement(std, &mut active_cost, &mut nb_status, n_orig);
    }

    // This solve's own pivot-threshold ladder starts from the default
    // (`sparse_lu::STABILITY`, or `ENOMOTO_PIVOT_THRESHOLD`): the
    // escalation below is per-solve state living in a thread-local, so a
    // thread that has just finished a numerically nasty solve must not
    // hand its raised floor — and the extra fill-in that buys — to this
    // one. See `sparse_lu::pivot_threshold`'s own docs.
    sparse_lu::reset_pivot_threshold();
    let mut lu = refactorize(std, &basis_pos, None)?;
    let mut since_check = 0usize;

    // Reduced costs (`d`), maintained incrementally (Huangfu & Hall
    // §2.2.3's update-dual, `super::solve_lp_dual_on`'s own `d`) instead
    // of a fresh BTRAN(`c_B`) every iteration — sound here because a
    // column's *cost* never depends on `M` (only its *bounds* do), so `d`
    // is a plain `f64` array exactly like the classical method's, `M`
    // never entering into it at all. Correct at the all-slack start for
    // the same reason the classical crash's `d` is: `y = 0` there (every
    // slack's cost is `0`), so `d[j] = active_cost[j] - 0`.
    let mut d = active_cost.clone();
    let mut fd_cb = vec![0.0f64; m];
    let mut fd_y = vec![0.0f64; m];
    // Scratch for the `d`-drift check ([`D_DRIFT_TOL`]'s own docs) — a
    // fresh `d` recomputed here is thrown away once compared against the
    // incrementally-maintained one; the real, kept recomputation on an
    // actual trigger still goes through `fresh_d_into(..., &mut d)`
    // directly, unchanged.
    let mut fresh_d_buf = vec![0.0f64; n_total];

    // Buffer reuse: every one of these is sized once, before the loop,
    // and reused every iteration (`super::solve_lp_dual_on`'s own
    // convention — see that function's own docs on why a fresh `Vec` of
    // `O(m)`/`O(n_total))` size every iteration costs real time at scale).
    let mut a_p = vec![0.0f64; n_total];
    let mut touched = vec![false; n_total];
    let mut touched_cols: Vec<usize> = Vec::new();
    let mut x_b_base = vec![0.0f64; m];
    let mut x_b_slope = vec![0.0f64; m];
    let mut lu_scratch = vec![0.0f64; m];
    // Scratch for [`residual_norm_affine`]'s own combined drift-check pass.
    let mut resid_scratch_base = vec![0.0f64; m];
    let mut resid_scratch_slope = vec![0.0f64; m];
    let mut rho = vec![0.0f64; m];
    let mut dense_q = vec![0.0f64; m];
    let mut alpha_full = vec![0.0f64; m];
    let mut tau = vec![0.0f64; m];
    // Scratch for the DSE `tau` half of the fused entering-column/`tau`
    // FTRAN ([`sparse_lu::FtLu::solve_into_pair_capture`]); `lu_scratch`
    // serves the entering-column half, as it does for the plain dense solve.
    let mut tau_scratch = vec![0.0f64; m];
    // `ENOMOTO_FUSED_DSE_FTRAN=0` restores the two separate solves (A/B
    // only — the fused form is bit-identical, see its own docs).
    let fused_dse_ftran = std::env::var("ENOMOTO_FUSED_DSE_FTRAN").map_or(true, |v| v != "0");
    // BFRT combined-flip FTRAN (dense branch) folded into the same fused
    // traversal as a third vector ([`sparse_lu::FtLu::solve_into_triple_capture`]);
    // `ENOMOTO_FUSED_BFRT_FTRAN=0` restores the separate solve (A/B only —
    // bit-identical). `combined_scratch` is that third vector's scratch.
    let fused_bfrt_ftran = std::env::var("ENOMOTO_FUSED_BFRT_FTRAN").map_or(true, |v| v != "0");
    let mut combined_scratch = vec![0.0f64; m];
    // Apply the BFRT combined-flip result to `x_B(M)` inside the entering
    // column's own `x_B` update loop (one pass, one `refresh_row` per row)
    // instead of a separate `0..m` pass. `ENOMOTO_MERGE_FLIP_XB=0` restores
    // the separate pass (A/B only). Per-row arithmetic is unchanged; only the
    // order of `InfeasibleRows` membership changes can differ.
    let merge_flip_xb = std::env::var("ENOMOTO_MERGE_FLIP_XB").map_or(true, |v| v != "0");
    // Dedicated `try_update_precomputed` capture buffers — see
    // `super::solve_lp_dual_on`'s own identical pair (`a_tilde_buf`/
    // `e_tilde_buf`) for the full reasoning: `e_tilde_buf` is filled as a
    // side effect of this iteration's `rho` BTRAN just below, `a_tilde_buf`
    // as a side effect of this same iteration's entering-column FTRAN
    // further down, both replacing what `try_update` used to recompute
    // from scratch. Kept separate from every other buffer here for the
    // same reason: nothing else may write through them between capture and
    // the `try_update_precomputed` call.
    let mut a_tilde_buf = vec![0.0f64; m];
    let mut e_tilde_buf = vec![0.0f64; m];
    let mut candidates: Vec<Cand> = Vec::new();
    // chuzc1's heap storage and the non-`bland` walk's popped prefix,
    // kept across iterations (this loop's preallocate-once convention)
    // instead of a fresh `BinaryHeap`/`Vec` allocation every pivot.
    let mut heap_buf: Vec<Reverse<Cand>> = Vec::new();
    // chuzc1's branch-free write-then-keep buffer (see that step's docs).
    let mut cand_scratch: Vec<Cand> = Vec::new();
    let mut sorted_prefix: Vec<Cand> = Vec::new();

    // PRICE's own row-major copy of `A`, built once: `std.rows` minus
    // every fixed column (`lb == ub`, which PRICE skips anyway), in
    // struct-of-arrays form with `u32` column indices. Each row keeps
    // `std.rows.row(i)`'s own column order, so the accumulation into
    // `a_p` (and hence every `a_p[j]` bit) is unchanged — what it saves is
    // the two random `std.lb[j]`/`std.ub[j]` loads and the branch per
    // visited entry, plus 4 bytes of index per entry (12 vs 16 bytes).
    let (price_start, mut price_col, mut price_val) = {
        let mut start: Vec<usize> = Vec::with_capacity(m + 1);
        let mut col: Vec<u32> = Vec::with_capacity(std.rows.nnz());
        let mut val: Vec<f64> = Vec::with_capacity(std.rows.nnz());
        start.push(0);
        for i in 0..m {
            for &(j, v) in std.rows.row(i) {
                if std.lb[j] == std.ub[j] {
                    continue;
                }
                col.push(u32::try_from(j).ok()?);
                val.push(v);
            }
            start.push(col.len());
        }
        (start, col, val)
    };
    // **Path-changing, default on** (`ENOMOTO_PRICE_NONBASIC_ONLY=0`
    // restores the old every-non-fixed-column PRICE; NETLIB93 A/B against
    // that: -5.3% total, no problem >10% slower): HiGHS's own row-wise
    // PRICE matrix is *partitioned* (`HighsSparseMatrix::createRowwisePartitioned`/`update`, `p_end_`):
    // each row's nonbasic entries sit in `[start, p_end)`, its basic ones
    // after, swapped across the boundary on every basis change, so PRICE
    // never visits a basic column at all. This loop's PRICE used to
    // include basic columns deliberately (see PRICE's own comment below:
    // the leaving column needs its `d` update) — measured at 35-58% of all
    // PRICE entries on the heavy Netlib instances (`dfl001` 41%, `pilot87`
    // 39%, `maros-r7` 58%). With the partition, the leaving column's `d` is
    // instead set directly the way HiGHS's `HEkkDual::updateDual` does
    // (`workDual[variable_in] = 0; workDual[variable_out] = -theta_dual`),
    // exact in infinite precision but no longer bit-identical: basic
    // columns stop accumulating the rounding noise their `a_p ~ 0` entries
    // used to feed into `d`, so the pivot path can drift.
    // `price_nb_end[i]` is row `i`'s partition boundary; with the flag off
    // it is simply the row end (every non-fixed column priced, as before).
    let price_nonbasic_only = std::env::var("ENOMOTO_PRICE_NONBASIC_ONLY").map_or(true, |v| v != "0");
    let mut price_nb_end: Vec<usize> = price_start[1..].to_vec();
    if price_nonbasic_only {
        let mut tmp: Vec<(u32, f64)> = Vec::new();
        for i in 0..m {
            let (lo, hi) = (price_start[i], price_start[i + 1]);
            tmp.clear();
            tmp.extend(price_col[lo..hi].iter().zip(&price_val[lo..hi]).filter(|(&j, _)| nb_status[j as usize].is_some()).map(|(&j, &v)| (j, v)));
            let n_nb = tmp.len();
            tmp.extend(price_col[lo..hi].iter().zip(&price_val[lo..hi]).filter(|(&j, _)| nb_status[j as usize].is_none()).map(|(&j, &v)| (j, v)));
            for (k, &(j, v)) in tmp.iter().enumerate() {
                price_col[lo + k] = j;
                price_val[lo + k] = v;
            }
            price_nb_end[i] = lo + n_nb;
        }
    }

    // Dedicated to `solve_sparse_into` alone, per that method's own
    // documented precondition (`FtLu::solve_sparse_into`'s own docs) —
    // never shared with a `solve_into`/`solve_transpose_into` call's own
    // `lu_scratch` above, which tolerates arbitrary leftover content in a
    // way the sparse path's `l_solve_sparse_into` does not.
    let mut sparse_scratch = vec![0.0f64; m];
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

    // BFRT combined-flip accumulators (two channels — `Affine1` has no
    // single-`f64` representation to solve for at once): summed sparse
    // contribution of every candidate flipped this iteration, in `base`/
    // `slope` in lockstep with `touched`/`touched_cols`'s own shared-index
    // convention above.
    let mut combined_base = vec![0.0f64; m];
    let mut combined_slope = vec![0.0f64; m];
    let mut combined_touched_flag = vec![false; m];
    let mut combined_touched: Vec<usize> = Vec::new();
    // The `should_use_dense_solve` sparse branch's own input buffers,
    // reused every iteration (this loop's own established convention —
    // see the buffers above) instead of two fresh `Vec`s `.collect()`-ed
    // from `combined_touched` every time this branch runs.
    let mut sparse_base_buf: Vec<(usize, f64)> = Vec::with_capacity(m);
    let mut sparse_slope_buf: Vec<(usize, f64)> = Vec::with_capacity(m);
    let mut combined_alpha_base = vec![0.0f64; m];
    let mut combined_alpha_slope = vec![0.0f64; m];

    // `x_B(M)`'s own one-time seed (Lemma 4.1's `b - N x_N`, solved once)
    // — every iteration from here on maintains `x_b_base`/`x_b_slope`
    // incrementally instead of recomputing this same `O(nnz(A))` sum from
    // scratch (see the loop body's own docs on why, and on the periodic
    // resync that re-anchors it against exactly this same computation).
    let (seed_base, seed_slope) = compute_rhs_affine(std, &cache, &nb_status)?;
    lu.solve_into(&seed_base, &mut lu_scratch, &mut x_b_base);
    lu.solve_into(&seed_slope, &mut lu_scratch, &mut x_b_slope);
    snap_slopes(&mut x_b_slope);

    // `super::solve_lp_dual_on`'s own `noise_feasible`, ported: a row
    // whose own infeasibility, at the Eligible=empty juncture below, is
    // proven to be within this problem's own rounding noise (scaled by
    // that row's own RHS magnitude, not a bare absolute constant — see
    // that site's own docs, and `PRIMAL_FEAS_TOL`'s own doc comment's
    // `agg` example: an infeasibility of `1.4e-7` on a row whose own
    // `rhs ~ 3.4e6` is noise, not a violated constraint) is marked here
    // and skipped by every later `chuzr` scan instead of being reselected
    // (and re-failing chuzc1 identically) every subsequent iteration.
    // Indexed by *variable* id (`basis[i]`), matching that classical
    // mechanism's own convention, since the row index `i` a variable
    // currently occupies can change across pivots but this fact about
    // the variable itself does not.
    let mut noise_feasible = vec![false; n_total];

    // Hyper-sparse `chuzr` (`super::InfeasibleRows`'s own docs): the set
    // of basic rows currently primal-infeasible, maintained incrementally
    // pivot-to-pivot by every `x_B`-writing loop below instead of
    // rescanned in full every iteration — exactly the classical dual
    // method's own optimization, reused verbatim (see that struct's own
    // docs on why it has no `f64`-vs-`Affine1` dependency to generalize).
    let mut infeasible_rows = InfeasibleRows::new(m);
    let mut row_dev = RowDevCache::new(m);
    let mut row_bounds = RowBounds::new(&cache, &basis, &noise_feasible);
    rebuild_rows(&mut infeasible_rows, &mut row_dev, m, &row_bounds, &x_b_base, &x_b_slope);
    fresh_d_into(std, &lu, &basis, &basis_pos, &active_cost, &mut fd_cb, &mut lu_scratch, &mut fd_y, &mut d);

    // Leaving-row weighting (paper \S4.5): starts in cheap `Devex` mode and
    // escalates one-way to exact `Dse`, reusing `super::DevexState`/
    // `super::DseState`/`super::EdgeWeights` directly rather than
    // reimplementing them — their own weights are pure, `M`-independent
    // tableau-row quantities (see [`Score2`]'s own docs), so nothing about
    // them needs to change to serve this module; only the *score* they
    // feed into (`Score2`, generalizing the classical method's plain
    // `delta^2/w` to a degree-2 polynomial in `M`) is new.
    // Exact DSE from the very first pivot, not the classical loop's
    // Devex-then-escalate scheme. The all-slack `B0` is a signed identity,
    // so `DseState::new`'s unit weights are exact here. The earlier
    // "DSE from the start is a gamble" finding (`bnl1`/`perold` ~190x
    // slower) was not a pricing effect at all: both blow-ups were the
    // `update_verify` refactor-and-retry loop below firing on a freshly
    // factorized basis (see that check's own comment); with that fixed,
    // a 73-problem Netlib sweep put DSE-from-start at 8.7s total against
    // 10.7s for Devex-start (fit1p 1.95s -> 0.49s, wood1p 0.31s -> 0.16s,
    // cycle 0.42s -> 0.13s), with no problem regressing by more than the
    // run-to-run noise except `maros` (0.15s -> 0.24s).
    let mut weights = super::EdgeWeights::Dse(super::DseState::new(m));

    let stall_limit = (5 * m).max(500);
    let mut stall_count = 0usize;
    let mut bland_mode = false;
    // EXPERIMENTAL "greatest improvement" chuzr escalation: once `stall_count`
    // (already-existing objective-progress stall signal, see its own
    // increment site below) crosses this threshold — well before `bland_mode`
    // would latch on at `stall_limit` — row selection stops trusting DSE's
    // cheap geometric proxy (`Score2`, \S4.5) and instead directly estimates
    // the *actual* dual objective gain of pivoting on each of the top
    // `GREATEST_IMPROVEMENT_TOP_K` DSE-ranked candidate rows (a real trial
    // BTRAN+PRICE+ratio-test per candidate, [`trial_row_ratio`]'s own docs),
    // picking whichever row's estimated gain is largest instead of whichever
    // has the largest DSE score. The motivation is exactly DSE's own known
    // gap: `Delta_i^2/gamma_i` approximates the objective gain achievable by
    // *some* pivot on row `i`, without the true reduced costs `d[j]` (only
    // the tableau geometry) ever entering the estimate — usually a good
    // enough proxy, but not identical, and the gap is largest on exactly the
    // kind of degenerate instance that stalls DSE in the first place.
    // Deliberately gated behind a stall signal rather than replacing DSE
    // outright: the per-candidate trial pricing costs roughly as much as
    // this iteration's *own* real PRICE step, times `GREATEST_IMPROVEMENT_TOP_K`,
    // so paying it every iteration on a healthy (non-stalling) solve would
    // only add cost for no benefit DSE wasn't already providing.
    const GREATEST_IMPROVEMENT_TOP_K: usize = 8;
    let greatest_improvement_stall_threshold = (stall_limit / 4).max(30);
    let greatest_improvement_enabled = std::env::var("ENOMOTO_DISABLE_GREATEST_IMPROVEMENT").is_err();
    let debug_greatest_improvement = std::env::var("ENOMOTO_DEBUG_EXT_GREATEST_IMPROVEMENT").is_ok();
    let mut gi_candidates: Vec<(usize, i32, Affine1, Score2)> = Vec::with_capacity(GREATEST_IMPROVEMENT_TOP_K * 4);
    // See the infeasible-row-count plateau check's own docs (this loop's
    // body, next to `stall_count`'s own increment) — a much larger
    // threshold than `stall_limit` deliberately, to stay clear of a
    // healthy-but-slow solve's own normal infeasible-count fluctuation.
    // ... but capped so the trigger can actually fire inside this loop's
    // own `MAX_ITERS` budget. `4 * stall_limit` alone is `20 * m`, which
    // for any `m > 1000` exceeds `MAX_ITERS` outright — on those problems
    // the plateau detector could never fire at all, however static the
    // infeasible set got, and the solve just spent its whole budget
    // before falling back (Netlib `greenbea`, `m = 2056`: limit 41,120
    // against a 20,000-iteration budget, with the infeasible count sitting
    // at a constant 302 for the last ~12,000 of them). A safety net sized
    // above the budget it is meant to protect is not a safety net.
    let infeasible_plateau_limit = (4 * stall_limit).min(super::MAX_ITERS_FLOOR / 4);
    let mut infeasible_plateau_count = 0usize;
    // Smallest infeasible-row count seen so far, *not* the previous
    // iteration's — see the plateau check's own docs in the loop body.
    let mut best_infeasible_len = infeasible_rows.rows.len();
    // Override for A/B testing [`XB_DRIFT_TOL`] itself (the escalation
    // ladder's own starting point) — see that constant's own docs for the
    // four prior single-knob attempts this per-solve escalation replaced.
    let xb_drift_tol: f64 = std::env::var("ENOMOTO_XB_DRIFT_TOL").ok().and_then(|s| s.parse::<f64>().ok()).unwrap_or(XB_DRIFT_TOL);
    // Escalation state for [`XB_DRIFT_TOL`]'s own per-solve ladder — counts
    // drift-triggered refactorizations *in this solve only* (reset to `0`
    // for every call, unlike a module-level constant); see that constant's
    // own docs for why counting this directly, rather than deriving a bound
    // from any static per-problem property, is what finally separates
    // "genuinely drift-heavy solve" from "fragile to any loosening at all".
    let mut drift_trigger_count: usize = 0;
    // §2.4's own per-solve ladder, counted separately from
    // `drift_trigger_count` because it answers a different question: that
    // one counts only the `x_B(M)` residual trigger (whose *tolerance* it
    // loosens), this one counts every numerically-caused refactorization
    // (see [`PIVOT_ESCALATION_STEP`]) and tightens the *factorization*
    // instead. `0` disables the escalation entirely, which is how the A/B
    // behind it is produced without a rebuild.
    let pivot_escalation_step: usize = std::env::var("ENOMOTO_PIVOT_ESCALATION_STEP")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(PIVOT_ESCALATION_STEP);
    let mut numeric_trouble_count: usize = 0;
    /// Records one numerically-caused refactorization and escalates the
    /// LU pivot threshold every [`PIVOT_ESCALATION_STEP`] of them. A macro
    /// rather than a closure: the four call sites are spread through a
    /// loop body that already holds `&mut` borrows of half this function's
    /// locals, and a closure capturing `numeric_trouble_count` mutably
    /// would conflict with them.
    macro_rules! note_numeric_trouble {
        () => {{
            numeric_trouble_count += 1;
            if pivot_escalation_step != 0 && numeric_trouble_count % pivot_escalation_step == 0 {
                sparse_lu::escalate_pivot_threshold();
            }
        }};
    }
    // Separate, coarser cadence for the `d`-drift check below, mirroring
    // `super::RESIDUAL_CHECK_MULTIPLIER`'s own rationale for the classical
    // path's `since_residual_check`: this check's residual is dominated by
    // fixed columns (`lb == ub`) that `d` is never updated for in the first
    // place (PRICE skips them), so once those are excluded from the
    // residual (see the loop below) genuine drift stays many orders of
    // magnitude below `D_DRIFT_TOL` — measured on the full Netlib suite, it
    // fires on only one problem, and even there it's the same fixed-column
    // artifact, not real drift — so paying its `O(n_total)` BTRAN-based
    // `fresh_d_into` every `XB_CHECK_INTERVAL` iterations buys nothing.
    let mut since_d_drift_check: usize = 0;
    // `super::update_verify`'s own env-var escape hatch, hoisted outside
    // the loop for the same reason `super::solve_lp_dual_on` hoists its
    // own copy: a single `bool` branch per pivot, not an `env::var` call.
    let update_verify_disabled = std::env::var("ENOMOTO_DISABLE_UPDATE_VERIFY").is_ok();
    // Hoisted for the same reason: `ENOMOTO_DEBUG_D_DRIFT_EXT` used to be
    // looked up with `std::env::var` (environment lock + linear scan +
    // `String` allocation) on *every* pivot just to guard a debug print.
    let debug_d_drift_ext = std::env::var("ENOMOTO_DEBUG_D_DRIFT_EXT").is_ok();
    // `ENOMOTO_PROF_PHASES_EXT` — this module's own counterpart to
    // `simplex::solve_lp_dual`'s `ENOMOTO_PROF_PHASES` (see [`prof_phases`]'s
    // own docs). Hoisted here for the same reason: a single `bool` branch
    // per phase per iteration, not an `env::var` call. Only covers this
    // function's own main loop, not `polish_with_true_bounds`'s separate
    // (and typically far shorter) cleanup loop.
    let profile_phases = std::env::var("ENOMOTO_PROF_PHASES_EXT").is_ok();
    if profile_phases {
        prof_phases::reset();
        prof_phases::STAT_M.store(m, std::sync::atomic::Ordering::Relaxed);
    }
    // `ENOMOTO_PROF_PHASES_EXT_WORK=1` (on top of `ENOMOTO_PROF_PHASES_EXT`)
    // adds the `work/iter` volume counters (`rho_p`/`alpha`/`tau` nonzeros,
    // PRICE entries, touched columns, chuzc1 candidates). Separate because
    // gathering them costs `O(m + PRICE entries)` per iteration outside
    // every phase timer — enough to distort the report's own wall total.
    let profile_work = profile_phases && std::env::var("ENOMOTO_PROF_PHASES_EXT_WORK").is_ok_and(|v| v != "0");
    // Diagnostic only (`ENOMOTO_DEBUG_EXT_DELTA0`): the paper's own
    // remark (\S4.5's absorbing-boundary result, `prop:no-return`) says
    // that once every M-flagged structural column is off its `M` side —
    // basic, or nonbasic at its genuinely finite side — it never returns,
    // and from that iteration on this loop's own decisions coincide
    // exactly with the classical bounded method's. `m_flagged_cols` is
    // fixed for the whole solve (`delta` never changes); a column counts
    // as "still on the M side" iff it's nonbasic (`nb_status[j].is_some()`)
    // at the side whose `ColCache` value has nonzero slope — see
    // `hat_lower`/`hat_upper`'s own docs for why the finite side is always
    // slope `0`. Answers "how many of this loop's own iterations run
    // *after* delta=0, where the paper says nothing more distinguishes
    // this phase from classical dual simplex" — a question the loop
    // itself never otherwise answers, since reaching delta=0 mid-loop
    // isn't a branch this code currently acts on (see `finish`'s own docs
    // on `cleanup_pivots`/`polish_with_true_bounds` firing only when
    // delta=0 is reached by loop *exit*, not mid-loop).
    //
    // **Tried and reverted: switching `weights` from exact `Dse` to cheap
    // `Devex` right at this point** — at delta=0 the current basis is
    // dual-feasible for the *true* problem with every nonbasic column at a
    // genuinely finite bound, precisely the state `solve_lp_dual_on` itself
    // starts every classical solve from in cheap Devex mode (see
    // `DevexState`'s own docs), so it looked like a legitimate way to stop
    // paying DSE's extra per-pivot FTRAN (`tau`) for whatever remains of
    // the solve. Implemented in full, including porting `solve_lp_dual_on`'s
    // own two escalation-back-to-Dse triggers (ill-conditioned single pivot,
    // rolling stagnation window) as a safety net. First attempt (seeding
    // Devex's weights from the live, exact `DseState.w` at the switch point
    // — reasoned to be strictly *more* accurate than Devex's own trivial
    // `1.0` start) produced a real, reproducible false `Infeasible` on
    // Netlib `pilot4` — root-caused to `DevexState::update_after_pivot`'s
    // pivot-row line hard-flooring at `1.0`, a floor calibrated for
    // `DevexState::new`'s own "every row starts at exactly `1.0`"
    // convention and not for raw DSE-scale values (see that constructor's
    // own docs for the full mechanism). Fixed by seeding fresh at `1.0`
    // instead (correctness restored, confirmed via the full 73-problem
    // sweep) — but even fixed, this was a clear net *regression*: Netlib
    // `fit1p`'s own main-loop iteration count alone rose from `4412`
    // (exact Dse throughout) to `17270` after switching to fresh Devex at
    // its own delta=0 point (reached at iteration `1176`), and the full
    // 73-problem total rose from ~7.6s to ~10.4s. The reason `solve_lp_dual_on`
    // benefits from starting Devex cheap is specifically that it starts at
    // the *trivial* all-slack basis, where a fresh `1.0` approximation
    // costs nothing to be reasonably accurate; restarting Devex from
    // scratch at a basis already hundreds or thousands of pivots deep
    // (typical for a solve that takes this long to reach delta=0 at all)
    // discards far more exact geometric information (that `Dse` was
    // already maintaining for free via incremental updates) than the saved
    // FTRAN was ever going to be worth. Reverted outright rather than left
    // behind an env-var flag — unlike this module's other reverted
    // experiments, no configuration of this idea (fresh-seeded or
    // DSE-seeded) showed any redeeming case worth preserving a toggle for.
    // Always on since **validated** (`degen3` DSE-drift follow-up, full
    // 73-problem Netlib sweep, two repeats) — `ENOMOTO_DSE_REFRESH_ON_REFACTOR=0`
    // to disable for A/B comparison. `DseState`'s incremental
    // `update_after_pivot` is exact only in infinite precision — measured
    // directly on `degen3` (`ENOMOTO_PROF_PHASES_EXT`'s own `dse_rel_err`
    // line, still wired up) drifting to >=100% relative error from the
    // true `||B^-T e_r||^2` on over a quarter of iterations, with only 11%
    // staying within 1%, and this module's own refactor cadence
    // (`REFACTOR_COUNT`, typically single digits per solve) never
    // refreshed the weights the way it already refreshes `x_B`/`d` — a
    // `refactorize()` already pays for a fresh `lu`, so recomputing exact
    // weights from it (`DseState::from_basis`, the same `O(m)`-BTRAN cost
    // as the `fresh_d` resync already done at the same point) is close to
    // free relative to the refactor itself.
    //
    // **Measured**: `degen3` alone 7,915 -> 2,626 main-loop iterations
    // (2.79s -> 0.88s), refactor count 13 -> 4 (better-conditioned pivots
    // from more accurate weights need fewer drift-triggered refactors too
    // — not just fewer iterations from better row choices). Full
    // 73-problem sweep: 8.91s -> 5.68s and 5.34s (two repeats, both well
    // clear of baseline noise), ratio-to-HiGHS 4.10x -> ~2.8x. No status
    // changes on any problem; the only objective mismatch either run
    // produces is the pre-existing `cycle` ~3e-4 relative gap
    // (`netlib-benchmark-workflow`'s own note, unrelated to this change).
    // Worst single-problem regression: `wood1p` +0.016s. Unlike the
    // `polish_with_true_bounds` "Exact DSE was tried here" note (a
    // *different* placement — adding DSE weighting to a phase that had
    // none — found a net regression elsewhere): this only refreshes
    // weights an *already-DSE-weighted* loop maintains, at points where a
    // refactor is happening anyway, so it carries none of that placement's
    // extra per-pivot FTRAN cost.
    //
    // **Re-evaluated and defaulted off**: the drift this refresh was
    // compensating for came from `DseState::update_after_pivot`'s old
    // `wp_old` self-amplification bug, fixed separately since the
    // measurement above was taken (see that function's own `wp_old =
    // ||rho_p||^2` docs). With that fixed, `degen3`'s `dse_rel_err`
    // diagnostic now reports <1% relative error on 100% of iterations
    // with no refresh at all — the >=100%-drifting case this refresh
    // exists for no longer occurs, so on the current codebase it is pure
    // cost: profiling `degen3` (`ENOMOTO_PROF_PHASES_EXT`) attributes 12%
    // of wall time to `DseState::from_basis` at each refactor (1,412
    // BTRANs/event on this problem), on an identical 2,163-iteration
    // pivot path and objective with or without it. Full 77-problem Netlib
    // sweep with the refresh off: -10.8% total wall time, no status or
    // objective changes. `ENOMOTO_DSE_REFRESH_ON_REFACTOR=1` re-enables it
    // for A/B comparison if a future drift regression reappears.
    let dse_refresh_on_refactor = std::env::var("ENOMOTO_DSE_REFRESH_ON_REFACTOR").is_ok_and(|v| v != "0");
    let debug_delta0 = std::env::var("ENOMOTO_DEBUG_EXT_DELTA0").is_ok();
    let debug_ext_iters_verbose = std::env::var("ENOMOTO_DEBUG_EXT_TRACE").is_ok();
    let m_flagged_cols: Vec<usize> = (0..n_orig).filter(|&j| delta[j].is_flagged()).collect();
    let mut delta0_iter: Option<usize> = None;
    if debug_ext_iters_verbose {
        let one_sided_total = (0..n_orig).filter(|&j| std.lb[j] == f64::NEG_INFINITY || std.ub[j] == f64::INFINITY).count();
        eprintln!("DEBUG_EXT_TRACE: one_sided_unbounded_total={one_sided_total} n_m_flagged(|S|)={}", m_flagged_cols.len());
    }

    // EXPERIMENTAL (`ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL`, unset = `1e-9` =
    // identical to the fixed-tolerance baseline): a column flagged
    // M (`delta[j] != 0`) only ever leaves its `M` side by being chosen as
    // the *entering* column `q` — never by a BFRT flip (candidates with
    // `s_j = 1` are provably excluded from the flip set, `prop:no-return`'s
    // own proof, part (ii)) — so `resolved_m`/`remaining_m_side` need only
    // watch `q`, not rescan every nonbasic column every iteration the way
    // `ENOMOTO_DEBUG_EXT_DELTA0`'s own diagnostic does. `remaining_m_side`
    // reaching `0` is exactly this iteration's delta=0 (see that
    // diagnostic's own docs for what that means). The *ratio* still
    // remaining drives `Score2::cmp_lex`'s tie tolerance for the leading
    // (`c2`, slope-driven) term: loose while many M-flagged columns are
    // still unresolved (so a row's *true* infeasibility can influence
    // which row gets fixed next, not just how many M-flagged columns feed
    // it), tightening back to the exact `1e-9` as `remaining_m_side` hits
    // `0` — i.e. by the time delta=0 actually arrives, this is bit-for-bit
    // the original fixed-tolerance comparison, so nothing about the
    // provably-classical post-delta=0 phase changes.
    //
    // **Measured, not just theorized (73-problem sweep, `netlib_benchmark_workflow`'s
    // own methodology)**: unlike a *flat* loosened `c2` tolerance (which
    // regressed the full-set total outright), this adaptive version can
    // genuinely cut iterations on more than one pathological instance at
    // once — at `1e-2`, `fit1p` 4412->3997, `degen3` 7914->7638, `25fv47`
    // 7957->6500 all improved simultaneously (a flat tolerance never
    // achieved that — always traded one of these off against another).
    //
    // `1e-2` also used to make `pilot4` return a **wrong** `Infeasible`
    // (HiGHS: optimal, obj=-2581.14) — since root-caused and fixed at the
    // pivot-*commit* level (`pivot_grossly_inconsistent`'s own docs, an
    // `updateVerify` gap letting an effectively-zero-pivot commit right
    // after a refactor), `pilot4` no longer returns a wrong answer at any
    // tolerance. It does, however, still cost tens of seconds at `1e-2`:
    // once that fix correctly discards the pathological (`q=9`, `r=144`)
    // candidate, nothing stops chuzr/chuzc1 from reselecting the exact
    // same pair next iteration (its PRICE-computed `a_p[9]` is
    // recomputed fresh from the same basis and rounds to the same
    // spurious ~3e-9 every time) — a discard-and-retry loop that only
    // terminates via `MAX_ITERS`, not via any actual escape.
    //
    // `stall_shrink` below exists to break exactly that loop: it tracks
    // how many iterations have passed since an M-flagged column last left
    // its `M` side (`iters_since_m_progress`) and multiplies the fraction-
    // based tolerance above by a factor that decays from `1.0` toward `0`
    // the longer that stretch runs — i.e. a stall in M-side progress pulls
    // `score2_c2_tol` back toward the exact, proven-safe `1e-9` regardless
    // of how many M-flagged columns nominally remain, since a
    // pathologically-repeating candidate is exactly the kind of "no real
    // progress" this loop can otherwise never detect on its own (unlike
    // `stall_count`/`bland_mode` below, which watches *objective*
    // progress, not M-side progress specifically — the two can diverge,
    // as `pilot4` demonstrates: plenty of ordinary, non-M pivots keep
    // firing while the *same* M-flagged row goes nowhere).
    //
    // **Validated (73-problem sweep, three repeats each)**: at
    // `ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL=1e-2` with the default
    // `ENOMOTO_SCORE2_STALL_HALFLIFE=50`, `pilot4` collapsed from the
    // ~34s `MAX_ITERS` exhaustion described above back down to 803
    // iterations / 0.09s — *and* every one of the 73 problems stayed
    // correct (only the pre-existing `cycle` objective mismatch, same as
    // the fixed-tolerance baseline). The full-set total landed at
    // 7.76-7.91s across three runs versus a same-binary baseline (Score2
    // disabled) of 7.52-7.72s — a small, roughly 3% aggregate regression,
    // inside this machine's own noise band but consistently on the wrong
    // side of it, so still not a demonstrated net win.
    //
    // The mechanism is also more parameter-sensitive than it looks:
    // `ENOMOTO_SCORE2_STALL_HALFLIFE=20` (a *shorter*, seemingly-safer
    // half-life — shrinking toward `1e-9` faster) reintroduced `pilot4`'s
    // ~34s `MAX_ITERS` exhaustion instead of fixing it, and `10` made it
    // worse still (73-set total 41-54s). `iters_since_m_progress` resets
    // on *any* M-flagged column resolving, not specifically the one stuck
    // in a repeating candidate — with a short half-life, some *other*
    // M-flagged column resolving elsewhere in the problem can hand the
    // stuck row a fresh grace period before its own tolerance ever
    // shrinks enough to change chuzr's row selection away from it, so a
    // shorter half-life can paradoxically make the escape *less* reliable,
    // not more. `50` is the only value swept that reliably escaped it.
    //
    // Left wired up (default: exactly the original fixed `1e-9`, zero
    // behavior change) as a documented, correctness-validated research
    // direction, not a tuned default: it needs either a smarter stall
    // signal (specific to the *stuck row/candidate*, not global M
    // progress) or a broader sweep across more pathological instances
    // before its own `50`-iteration half-life could be trusted as
    // anything more than "the one value that happened to work here".
    let n_m_flagged = m_flagged_cols.len().max(1);
    let mut resolved_m = vec![false; n_total];
    let mut remaining_m_side = m_flagged_cols.len();
    // Companion best-so-far for the infeasible-row-count plateau check
    // below: `remaining_m_side` only ever decreases (its one mutation
    // site is a plain `-= 1`, never incremented), so on a healthy solve
    // it is a strictly more reliable progress signal than
    // `infeasible_rows.rows.len()` — which measures how large the
    // (constantly churning) infeasible-row *set* is right now, not
    // whether the M-side resolution actually driving the solve forward
    // is stuck. Netlib `dfl001` (`analysis/dfl001_20260921_035239.md`)
    // is a healthy solve the plateau check otherwise mistook for
    // `pilot4`'s genuine stall: its infeasible-row count's own minimum
    // goes unbeaten for 5,000+ consecutive iterations (the set keeps
    // churning — 40% grow / 50% shrink — without ever posting a new
    // low) while `remaining_m_side` falls steadily throughout (11045 ->
    // 4762 over the same span), which is what this tracks.
    let mut best_remaining_m_side = remaining_m_side;
    let mut iters_since_m_progress: usize = 0;
    let score2_stall_halflife: f64 = std::env::var("ENOMOTO_SCORE2_STALL_HALFLIFE")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(50.0);
    let score2_max_tol = std::env::var("ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1e-9);
    let wall_t0 = std::time::Instant::now();

    let debug_dual_check = std::env::var("ENOMOTO_DEBUG_EXT_DUAL_CHECK").is_ok();
    let mut dual_violation_reported = false;
    let mut prev_q: Option<usize> = None;
    let mut prev_r: Option<usize> = None;
    let mut prev_alpha_q: f64 = 0.0;
    let mut prev_dj_q: f64 = 0.0;
    let mut prev_dbg: Option<(Vec<f64>, Vec<usize>, usize)> = None;

    // EXPERIMENTAL, **confirmed not to work** (`ENOMOTO_STUCK_ROW_BOOST_FACTOR`,
    // unset/`1.0` = no-op, zero behavior change either way): the idea —
    // rather than loosening the tie tolerance globally based on overall
    // M-side progress (`score2_max_tol`/`stall_shrink` above, which a
    // stall specific to one row can miss) — was to track the *specific*
    // row that `pivot_grossly_inconsistent`/`updateVerify` has just
    // discarded a pivot for, consecutively, and once that streak crosses
    // `ENOMOTO_STUCK_ROW_BOOST_THRESHOLD` (default `3`), score *that row
    // only* in chuzr as if its own deviation's `M`-dependence were scaled
    // by `ENOMOTO_STUCK_ROW_BOOST_FACTOR` (`2.0` tried, i.e. literally
    // substituting `2M` for `M` in `dev`'s own `base + slope*M` — only the
    // `slope` term changes), nudging chuzr's ranking of this one row
    // relative to every other currently-infeasible row.
    //
    // **Tested directly on the exact `pilot4` stall this was built for
    // (`ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL=1e-2` with `stall_shrink`
    // effectively disabled via a huge `ENOMOTO_SCORE2_STALL_HALFLIFE`, to
    // isolate this mechanism alone): factor `2.0` (boost) and `0.1`
    // (dampen) both still hit `MAX_ITERS` (~34s) — neither direction
    // escaped the loop.** The reason, on reflection, is structural: chuzr
    // (which this mechanism biases) only decides *which infeasible row*
    // gets worked on next; the actual failure lives one level down, in
    // chuzc1/BFRT's choice of *entering column* for whichever row wins —
    // `pilot4`'s own stuck candidate (`q=9`) is picked by ratio-test
    // ordering and Harris pass 2 (largest pivot magnitude in a flat ratio
    // window), neither of which reads `delta`, width, or anything this
    // mechanism touches. Re-scoring the row differently in chuzr does not
    // change what chuzc1 does once that row is selected, so — assuming
    // row 144 keeps winning chuzr regardless (plausible if it is the only,
    // or persistently the most attractive, infeasible row available) —
    // the exact same doomed candidate gets re-picked every time either
    // way. `stall_shrink` (above) worked instead precisely because it
    // changes the *cross-row* comparison outcome directly (which row wins
    // chuzr), not because it does anything smarter within a row.
    //
    // Left wired up, still fully inert by default, as a documented dead
    // end: a real per-candidate fix would need to act at the chuzc1/BFRT
    // level (e.g. excluding a specifically-discarded `(q, r)` pair from
    // re-selection) rather than reweighting chuzr's own row scores.
    let mut stuck_row: Option<usize> = None;
    let mut stuck_row_streak: usize = 0;
    let stuck_row_boost_threshold: usize = std::env::var("ENOMOTO_STUCK_ROW_BOOST_THRESHOLD")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(3);
    let stuck_row_boost_factor: f64 = std::env::var("ENOMOTO_STUCK_ROW_BOOST_FACTOR")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1.0);

    // Always on (formerly `ENOMOTO_BAN_DISCARDED_CANDIDATES`-gated and
    // off by default): unlike `stuck_row`/`stuck_row_streak` above (which
    // reweight chuzr's *row* ranking and, per that mechanism's own docs,
    // were confirmed *not* to change chuzc1's candidate choice within a
    // row), this acts directly on chuzc1 itself. The moment
    // `pivot_grossly_inconsistent`/`updateVerify` discards a pivot, `q` is
    // remembered as banned *for row `discard_row` specifically* — chuzc1's
    // own candidate-building loop (below) then skips it next time that
    // same row is selected, forcing the ratio test/Harris pass 2 to pick
    // among the *remaining* candidates instead of reproducing the
    // identical (and already-proven-untrustworthy) choice. Scoped to one
    // row at a time (cleared the moment a *different* row's pivot is
    // discarded, or `discard_row`'s own pivot finally commits, both
    // below): a column discarded for row `r` is not assumed bad for any
    // other row, and a ban is not assumed permanent once `r` itself moves
    // on. No streak/threshold here (unlike `stuck_row_boost`) — a single
    // discard is itself direct evidence this exact `(q, r)` pair is
    // unusable from the current basis, not merely inconvenient.
    //
    // **Validated as the fix that actually works, twice over.** First
    // (while still experimental) isolated from `stall_shrink` via a huge
    // `ENOMOTO_SCORE2_STALL_HALFLIFE`, on the `pilot4` stall both
    // `stall_shrink` and `stuck_row_boost` were built for: it settled in
    // 707 main-loop iterations / 0.07s — matching the unmodified
    // baseline's own 705 almost exactly, versus the ~34s `MAX_ITERS`
    // exhaustion every other mechanism left behind — confirming the
    // failure was never about *which row* chuzr picks, but chuzc1
    // mechanically re-deriving the identical doomed candidate from
    // unchanged inputs every retry. At the time, a full 73-problem sweep
    // found it statistically inert on its own (7.48-7.80s across several
    // configurations, all inside one noise band) *because* nothing in the
    // then-current default configuration ever loosened chuzr's tie-break
    // enough to land on a genuinely untrustworthy `(q, r)` pair in the
    // first place — so it stayed opt-in.
    //
    // Promoted to always-on once restricting `M`-bounding to `S` (§4.2,
    // this module's own top-of-file docs) changed that: with only the
    // columns whose cost sign actually forces infinite-side placement now
    // `M`-tracked, `pilot4` reaches exactly this landmine *by default* —
    // `infeasible_rows.rows.len()` frozen at `31` for 16,000+ consecutive
    // iterations post-`delta=0` (`ENOMOTO_DEBUG_EXT_TRACE`'s own trace),
    // chuzc1 re-selecting the same discarded `(q, r)` every single time —
    // until `MAX_ITERS` exhausts and the whole solve falls back to the
    // documented-fragile classical `BIG_M` path (55s+, on a problem this
    // module otherwise solves in under 0.1s). Enabling this unconditionally
    // fixes it outright (back to ~700-800 iterations) and, per the full
    // 73-problem sweep re-run under the `S`-restricted default (comparing
    // this flag on vs. off with `S` unchanged), reproduces the original
    // "statistically inert elsewhere" finding: every other problem's timing
    // stayed within normal run-to-run noise.
    let ban_discarded_candidates = true;
    let mut discard_row: Option<usize> = None;
    let mut discard_banned_cols: Vec<usize> = Vec::new();
    let mut prev_pool_len: Option<usize> = None;
    let max_iters = super::max_iters_for(m, n_total);
    for _iter in 0..max_iters {
        iters_since_m_progress += 1;
        if debug_ext_iters_verbose && _iter % 2000 == 0 {
            eprintln!(
                "DEBUG_EXT_TRACE: iter={_iter} infeasible_rows={} stall_count={stall_count} bland_mode={bland_mode} remaining_m_side={remaining_m_side}",
                infeasible_rows.rows.len()
            );
        }
        if profile_phases {
            prof_phases::ITERS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let pool_len = infeasible_rows.rows.len();
            prof_phases::INFEASIBLE_POOL.fetch_add(pool_len, std::sync::atomic::Ordering::Relaxed);
            if let Some(prev) = prev_pool_len {
                use std::cmp::Ordering as CmpOrdering;
                match pool_len.cmp(&prev) {
                    CmpOrdering::Greater => prof_phases::POOL_GREW.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    CmpOrdering::Less => prof_phases::POOL_SHRANK.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    CmpOrdering::Equal => prof_phases::POOL_SAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                };
            }
            prev_pool_len = Some(pool_len);
        }
        if debug_dual_check && !dual_violation_reported {
            let mut worst: Option<(usize, f64)> = None;
            for j in 0..std.n_total {
                let Some(status) = nb_status[j] else { continue };
                if std.lb[j] == std.ub[j] {
                    continue;
                }
                let dj = d[j];
                let viol = match status {
                    NbStatus::Lower => -dj,
                    NbStatus::Upper => dj,
                };
                if viol > 1.0 && worst.map_or(true, |(_, w)| viol > w) {
                    worst = Some((j, viol));
                }
            }
            if let Some((j, viol)) = worst {
                eprintln!(
                    "DEBUG_EXT_DUAL_CHECK: first dual-feasibility violation (>1.0) at iter={_iter} j={j} magnitude={viol} -- caused by PREVIOUS iter's pivot: prev_q={prev_q:?} prev_r={prev_r:?} prev_alpha_q={prev_alpha_q} prev_dj_q={prev_dj_q}"
                );
                if let Some((ap, flips, ncand)) = &prev_dbg {
                    let flipped = flips.contains(&j);
                    eprintln!(
                        "  DBG j={j}: prev a_p[j]={} flipped_prev={flipped} n_flips={} n_cand={ncand} status={:?} d[j]={} delta={:?} lower={:?} upper={:?} is_slack={}",
                        ap[j], flips.len(), nb_status[j], d[j], if j < n_orig { delta[j] } else { MSide::None }, cache.lower[j], cache.upper[j], j >= n_orig
                    );
                    let mut sumabs = 0.0f64;
                    for (i, &v) in ap.iter().enumerate() { if v.abs() > 1e3 { sumabs += 1.0; let _ = i; } }
                    eprintln!("  DBG prev row has {sumabs} entries with |a_p|>1e3");
                }
                dual_violation_reported = true;
            }
        }
        // (a')(b'): primal feasibility check + leaving-row selection —
        // Devex/DSE-weighted (paper \S4.5's generalization, [`Score2`]'s
        // own docs), not plain Dantzig: `super::solve_lp_dual_on`'s own
        // history names Netlib `cycle` directly as an instance where pure
        // largest-deviation selection (even with cost perturbation active)
        // converges to a slightly-off vertex instead of the true optimum,
        // which this weighting exists to fix. Scanned over
        // `infeasible_rows.rows` only, not `0..m` — `x_B(M)` itself is now
        // maintained incrementally (every write site below calls
        // `infeasible_rows.set`), not recomputed from scratch here the way
        // it used to be, so this scan visits only the rows that can
        // possibly win regardless.
        // Linear interpolation from `score2_max_tol` (all M-flagged columns
        // still unresolved, `remaining_m_side == n_m_flagged`) down to the
        // exact `1e-9` (`remaining_m_side == 0`, i.e. delta=0 for this
        // iteration onward) — see `score2_max_tol`'s own docs — then pulled
        // further toward `1e-9` by `stall_shrink` (`iters_since_m_progress`'s
        // own docs) whenever M-side progress has stalled, regardless of how
        // much of `remaining_m_side` is nominally left. `.max(1e-9)` guards
        // a `score2_max_tol` set below `1e-9` from ever tightening the
        // comparison beyond the original baseline.
        let stall_shrink = score2_stall_halflife / (score2_stall_halflife + iters_since_m_progress as f64);
        let score2_c2_tol = (1e-9 + (score2_max_tol - 1e-9) * (remaining_m_side as f64 / n_m_flagged as f64) * stall_shrink).max(1e-9);
        let mut best: Option<(usize, i32, Affine1, Score2)> = None;
        timed!(profile_phases, prof_phases::CHUZR, {
            for &i in &infeasible_rows.rows {
                // Cached at this row's last membership decision
                // ([`RowDevCache`]'s own docs) — bit-for-bit the value a
                // fresh `row_deviation` call would return here, without
                // re-reading `basis`/`noise_feasible`/`cache` at `basis[i]`.
                let (d_dir, dev) = (row_dev.dir[i], row_dev.dev[i]);
                // `dev` itself (returned in `best` below, used for this
                // row's actual target/theta arithmetic if chosen) stays
                // the true, unscaled value — only the `Score2` fed to
                // chuzr's own ranking sees the boosted one, and only for
                // the specific row `stuck_row` has flagged (its own docs).
                let scored_dev = if stuck_row_streak >= stuck_row_boost_threshold && stuck_row == Some(i) {
                    Affine1::new(dev.base, dev.slope * stuck_row_boost_factor)
                } else {
                    dev
                };
                let score = Score2::new(scored_dev, weights.weight(i));
                let better = match best {
                    None => true,
                    Some((br, _, _, bscore)) => {
                        if bland_mode {
                            i < br
                        } else {
                            match score.cmp_lex(&bscore, score2_c2_tol) {
                                std::cmp::Ordering::Greater => true,
                                std::cmp::Ordering::Less => false,
                                std::cmp::Ordering::Equal => i < br,
                            }
                        }
                    }
                };
                if better {
                    best = Some((i, d_dir, dev, score));
                }
            }
        });
        #[cfg(debug_assertions)]
        {
            // `InfeasibleRows`'s own exactness claim (its docs): every row
            // it lists infeasible, and no others, matches a fresh full
            // scan — mirrors `super::solve_lp_dual_on`'s own cross-check.
            let mut fresh: Vec<usize> = (0..m).filter(|&i| row_infeasible_affine(&cache, &basis, &x_b_base, &x_b_slope, &noise_feasible, i)).collect();
            let mut maintained: Vec<usize> = infeasible_rows.rows.clone();
            fresh.sort_unstable();
            maintained.sort_unstable();
            debug_assert_eq!(fresh, maintained, "InfeasibleRows drifted from a fresh scan at iter {_iter}");
            for &i in &infeasible_rows.rows {
                let (fd, fv) = row_deviation(&cache, &basis, &x_b_base, &x_b_slope, &noise_feasible, i).expect("pool row must be infeasible");
                let cv = row_dev.dev[i];
                debug_assert!(
                    fd == row_dev.dir[i] && fv.base.to_bits() == cv.base.to_bits() && fv.slope.to_bits() == cv.slope.to_bits(),
                    "RowDevCache drifted from a fresh row_deviation at iter {_iter}, row {i}"
                );
            }
        }
        let mut best = best.map(|(i, d_dir, dev, _)| (i, d_dir, dev));

        // "Greatest improvement" chuzr escalation (see `stall_count`'s own
        // declaration for the full rationale): only once DSE-based
        // selection has stalled for a while, re-rank the top DSE candidates
        // by a real trial-priced objective-gain estimate instead.
        if greatest_improvement_enabled && !bland_mode && stall_count > greatest_improvement_stall_threshold && infeasible_rows.rows.len() > 1 {
            timed!(profile_phases, prof_phases::CHUZR, {
                gi_candidates.clear();
                for &i in &infeasible_rows.rows {
                    let (d_dir_i, dev_i) = (row_dev.dir[i], row_dev.dev[i]);
                    let score = Score2::new(dev_i, weights.weight(i));
                    gi_candidates.push((i, d_dir_i, dev_i, score));
                }
                gi_candidates.sort_by(|a, b| b.3.cmp_lex(&a.3, score2_c2_tol));
                gi_candidates.truncate(GREATEST_IMPROVEMENT_TOP_K);

                let mut best_gain: Option<Affine1> = None;
                let mut best_gi: Option<(usize, i32, Affine1)> = None;
                for &(i, d_dir_i, dev_i, _) in &gi_candidates {
                    let Some(ratio) = trial_row_ratio(std, &lu, &nb_status, &d, d_dir_i, &mut lu_scratch, &mut rho, &mut a_p, &mut touched, &mut touched_cols, i) else {
                        continue;
                    };
                    let gain = dev_i.scale(ratio);
                    let better = match best_gain {
                        None => true,
                        Some(bg) => gain.cmp_lex(&bg) == std::cmp::Ordering::Greater,
                    };
                    if better {
                        best_gain = Some(gain);
                        best_gi = Some((i, d_dir_i, dev_i));
                    }
                }
                if let Some(chosen) = best_gi {
                    if debug_greatest_improvement {
                        eprintln!(
                            "DEBUG_EXT_GREATEST_IMPROVEMENT: iter={_iter} stall_count={stall_count} dse_pick={:?} gi_pick={} gain=({},{})",
                            best.map(|(i, ..)| i),
                            chosen.0,
                            best_gain.unwrap().base,
                            best_gain.unwrap().slope
                        );
                    }
                    best = Some(chosen);
                }
            });
        }

        let Some((r, d_dir, w_r)) = best else {
            // Primal feasible for the M-truncated problem (\S4.5's
            // `V_infty = empty`): proceed to Step III.
            if std::env::var("ENOMOTO_DEBUG_EXT_ITERS").is_ok() {
                eprintln!("DEBUG_EXT: main_loop_iters={_iter} bland_mode={bland_mode}");
            }
            if debug_delta0 {
                match delta0_iter {
                    Some(k) => eprintln!(
                        "DEBUG_EXT_DELTA0: delta=0 first reached at iter={k}, main_loop_iters={_iter}, iters_after_delta0={}",
                        _iter - k
                    ),
                    None => eprintln!("DEBUG_EXT_DELTA0: delta never reached 0 within the main loop (main_loop_iters={_iter})"),
                }
            }
            if profile_phases {
                prof_phases::report(wall_t0.elapsed().as_nanos() as usize);
            }
            return finish(std, &mut basis, &mut basis_pos, &mut nb_status, &delta, &cache, n_orig, lu);
        };

        // (c): pivot row, BTRAN against `e_r` — `M`-independent (paper
        // \S4.5's opening observation), so `rho`/`a_p`/`d` below are all
        // plain `f64`, exactly like the classical method's. Captures
        // `e_tilde_buf` (the post-U^-T, pre-R-reverse intermediate) as a
        // side effect, for this iteration's own `try_update_precomputed`
        // call further down — see `FtLu::try_update_precomputed`'s own
        // docs for why this is bit-for-bit the value that call would
        // otherwise recompute from scratch.
        timed!(profile_phases, prof_phases::BTRAN, lu.solve_transpose_unit_capture(r, &mut lu_scratch, &mut rho, &mut e_tilde_buf));
        if profile_phases {
            let exact_w = dot(&rho, &rho);
            let maintained_w = weights.weight(r);
            let rel_err = (maintained_w - exact_w).abs() / exact_w.max(1e-9);
            prof_phases::DSE_CHECKED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let bucket = if rel_err < 0.01 {
                &prof_phases::DSE_ERR_LT_1PCT
            } else if rel_err < 0.10 {
                &prof_phases::DSE_ERR_LT_10PCT
            } else if rel_err < 1.00 {
                &prof_phases::DSE_ERR_LT_100PCT
            } else {
                &prof_phases::DSE_ERR_GE_100PCT
            };
            bucket.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        // Row-major sparse PRICE (Huangfu & Hall §2.2.2's "spmv",
        // `super::solve_lp_dual_on`'s own PRICE step): `a_p = rho^T A`,
        // computed by walking only `rho`'s *nonzero* rows and scanning
        // each one's own sparse row of `A` — not, as this function's own
        // first version did, a per-column dot product over *every*
        // nonbasic column regardless of whether `rho` even touches the
        // rows that column lives in. This is also what makes the
        // incremental `d` update below possible at all: `a_p` is exactly
        // the row this iteration's entering column's own reduced-cost
        // drop is measured against.
        // Only fixed columns are excluded here, matching the classical
        // method's own PRICE filter — a *basic* column is deliberately
        // NOT skipped: the column currently basic at the leaving row
        // needs its own `a_p` entry too (its incremental `d` update below
        // is what gives it a correct reduced cost the moment it becomes
        // nonbasic this same iteration — omitting it here left `d[j]` for
        // a former basic column stale, corrupting a *later* iteration's
        // candidate ratio the first time that column became eligible
        // again; confirmed as a real, not merely theoretical, bug: it
        // broke the objective on the large majority of real Netlib
        // instances, ordinary bounded ones included, once this module's
        // own M-bounding logic even fires for a single unbounded-above
        // column — routine for a plain `x_j >= 0` MPS column with no
        // explicit upper bound).
        timed!(profile_phases, prof_phases::PRICE, {
            for i in 0..m {
                let rv = rho[i];
                if rv.abs() <= TOL {
                    continue;
                }
                let (lo, hi) = (price_start[i], price_nb_end[i]);
                for (&j, &v) in price_col[lo..hi].iter().zip(&price_val[lo..hi]) {
                    let j = j as usize;
                    if !touched[j] {
                        touched[j] = true;
                        touched_cols.push(j);
                    }
                    a_p[j] += rv * v;
                }
            }
        });
        if profile_work {
            use std::sync::atomic::Ordering::Relaxed;
            let mut rho_nnz = 0usize;
            let mut entries = 0usize;
            let mut entries_nb = 0usize;
            for i in 0..m {
                if rho[i].abs() > TOL {
                    rho_nnz += 1;
                    entries += price_start[i + 1] - price_start[i];
                    entries_nb += price_col[price_start[i]..price_start[i + 1]].iter().filter(|&&j| nb_status[j as usize].is_some()).count();
                }
            }
            prof_phases::STAT_RHO_NNZ.fetch_add(rho_nnz, Relaxed);
            prof_phases::STAT_PRICE_ENTRIES.fetch_add(entries, Relaxed);
            prof_phases::STAT_PRICE_ENTRIES_NB.fetch_add(entries_nb, Relaxed);
            prof_phases::STAT_TOUCHED.fetch_add(touched_cols.len(), Relaxed);
            prof_phases::STAT_TOUCHED_NONBASIC.fetch_add(touched_cols.iter().filter(|&&j| nb_status[j].is_some()).count(), Relaxed);
        }

        timed!(profile_phases, prof_phases::CHUZC1, {
            candidates.clear();
            let ban_active = ban_discarded_candidates && discard_row == Some(r);
            // Branch-free candidate filter: every touched column is
            // written into the next slot and the slot is kept (`k +=
            // keep`) only when it passes the same three tests the old
            // `continue`-based loop applied (nonbasic, `|alpha_j| > TOL`
            // in its original negated-`<=` form so a NaN is treated
            // identically, and the ratio-test sign condition). Those tests
            // are close to coin flips per column (a third of touched
            // columns are basic, about half fail the sign test), so the
            // branchy form paid a misprediction on most columns; the
            // arithmetic done for a rejected column is discarded. Kept
            // candidates carry exactly the `hat_alpha`/`ratio` the old loop
            // computed, in the same `touched_cols` order.
            // `cand_scratch` only ever grows (never cleared), so this is a
            // no-op after the first few iterations instead of an `O(k)`
            // fill every pivot.
            if cand_scratch.len() < touched_cols.len() {
                cand_scratch.resize(touched_cols.len(), Cand { j: 0, hat_alpha: 0.0, ratio: 0.0 });
            }
            let d_dir_f = d_dir as f64;
            let mut k = 0usize;
            for &j in &touched_cols {
                let (is_nb, sigma) = match nb_status[j] {
                    Some(NbStatus::Lower) => (true, 1.0),
                    Some(NbStatus::Upper) => (true, -1.0),
                    None => (false, 0.0),
                };
                let alpha_j = a_p[j];
                let hat_alpha = sigma * alpha_j;
                // `d[j]` here is the *incrementally maintained* reduced cost
                // (see this function's own docs) — no per-candidate BTRAN
                // dot-product needed, unlike this function's first version.
                let hat_c = (sigma * d[j]).max(0.0);
                cand_scratch[k] = Cand { j, hat_alpha, ratio: hat_c / hat_alpha.abs() };
                let keep = is_nb & !(alpha_j.abs() <= TOL) & !(d_dir_f * hat_alpha >= 0.0);
                k += keep as usize;
            }
            let kept = &cand_scratch[..k];
            // Smallest (in `Cand`'s own `(ratio, j)` order) candidate whose
            // width is a genuine infinity (`cache.width[j] == None`, a
            // one-sided row's slack): the BFRT walk below stops
            // unconditionally on reaching such a candidate (both the heap
            // and the `bland_mode` walk visit in this same order), and pass
            // 2 / the flip loop only ever index `[0, k_star]`, so no
            // candidate ordered after it can influence anything. Dropping
            // those leaves the chosen `q`, the flip set and every
            // floating-point value exactly as before, while shrinking the
            // pool the heap has to be built from — typically from ~1000
            // candidates to a few dozen on the heavy Netlib instances.
            // (HiGHS's `HEkkDualRow::choosePossible` prunes its own
            // candidate pack by a ratio bound for the same reason.)
            let mut stopper: Option<Cand> = None;
            for c in kept {
                if width_inf[c.j] && stopper.map_or(true, |s| *c < s) && !(ban_active && discard_banned_cols.contains(&c.j)) {
                    stopper = Some(*c);
                }
            }
            candidates.extend(kept.iter().filter(|c| stopper.map_or(true, |s| **c <= s) && !(ban_active && discard_banned_cols.contains(&c.j))));
        });
        if profile_work {
            prof_phases::STAT_CANDS.fetch_add(candidates.len(), std::sync::atomic::Ordering::Relaxed);
        }
        if candidates.is_empty() {
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            // `super::solve_lp_dual_on`'s own `noise_feasible` second
            // chance, ported (see that mechanism's own docs, and
            // `noise_feasible`'s own declaration above): before concluding
            // Proposition 4.6's genuine "Eligible = empty" case, check
            // whether `r`'s own deviation is actually within this row's
            // rounding noise rather than a real violation. Only ever
            // applies when `w_r.slope == 0` (within `Affine1::cmp_lex`'s
            // own `1e-9` floor) — a nonzero slope means the deviation is
            // genuinely `M`-scaled (unboundedly large in the limit this
            // module reasons about), which can never be noise regardless
            // of its `base` term, so only the plain-`f64`-equivalent case
            // gets this classical-style reprieve. Scaled by this row's own
            // RHS magnitude (`std.b[r]`), not a bare `PRIMAL_FEAS_TOL`,
            // for the identical reason `PRIMAL_FEAS_TOL`'s own doc comment
            // gives: a large-magnitude row's rounding floor scales with
            // it, so a fixed absolute bar is simultaneously too strict on
            // large rows and too loose on tiny ones.
            if bfrt_reached(w_r, Affine1::ZERO, x_b_base[r]) {
                noise_feasible[basis[r]] = true;
                row_bounds.mark_noise(r);
                infeasible_rows.set(r, false);
                continue;
            }
            // Never conclude infeasibility from an *updated* basis
            // factorization. `Affine1::cmp_lex`'s own `base` comparison
            // is absolute at `1e-9` (its `base_scale` floor of `1.0`),
            // while this loop knowingly tolerates `x_B` drift up to
            // `XB_DRIFT_TOL_MAX` (`1e-4`) between refactorizations — five
            // orders of magnitude of slack in which accumulated
            // Forrest-Tomlin error alone can make a perfectly feasible
            // row look like a violated one. Measured on Netlib's
            // `greenbea` (the instance this guard was written for): at
            // `update_count = 47` the deviation read `base = 3.0e-7`
            // against a flip capacity of exactly `0`, so the walk below
            // ran out of candidates and reported `Infeasible`; refactorized
            // at that same basis it reads exactly `-0.0` — i.e. the
            // capacity covers it precisely and the iteration has a pivot.
            // So: redo the iteration from a fresh factorization, and only
            // report `Infeasible` when it still holds there (the same
            // refactor-and-resync the `update_verify` discard path below
            // performs). Terminating: `refactorize` leaves
            // `update_count() == 0`, so a second visit with no committed
            // pivot in between falls straight through to the report.
            if lu.update_count() > 0 {
                if profile_phases {
                    prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    prof_phases::REFACTOR_CAUSE_INFEAS_CHECK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                timed!(profile_phases, prof_phases::REFACTOR, {
                    lu = refactorize(std, &basis_pos, Some(&lu))?;
                    let (fresh_base, fresh_slope) = compute_rhs_affine(std, &cache, &nb_status)?;
                    lu.solve_into(&fresh_base, &mut lu_scratch, &mut x_b_base);
                    lu.solve_into(&fresh_slope, &mut lu_scratch, &mut x_b_slope);
                    snap_slopes(&mut x_b_slope);
                    rebuild_rows(&mut infeasible_rows, &mut row_dev, m, &row_bounds, &x_b_base, &x_b_slope);
                    fresh_d_into(std, &lu, &basis, &basis_pos, &active_cost, &mut fd_cb, &mut lu_scratch, &mut fd_y, &mut d);
                    if dse_refresh_on_refactor {
                        if let super::EdgeWeights::Dse(dse) = &mut weights {
                            *dse = super::DseState::from_basis(m, &lu);
                        }
                    }
                });
                continue;
            }
            // Genuine mathematical conclusion (Proposition 4.6, the
            // classical `Eligible = empty` case), not a numerical
            // artifact — reported directly, no fallback.
            if std::env::var("ENOMOTO_DEBUG_EXT_INFEASIBLE").is_ok() {
                eprintln!(
                    "DEBUG_EXT_INFEASIBLE: site=eligible_empty iter={_iter} r={r} basis_r={} d_dir={d_dir} w_r=({},{}) noise_feasible_check_failed=true remaining_m_side={remaining_m_side}",
                    basis[r], w_r.base, w_r.slope
                );
                let mut dual_violations = 0usize;
                for j in 0..std.n_total {
                    let Some(status) = nb_status[j] else { continue };
                    let dj = d[j];
                    let bad = match status {
                        NbStatus::Lower => dj < -1e-6,
                        NbStatus::Upper => dj > 1e-6,
                    };
                    if bad {
                        dual_violations += 1;
                        if dual_violations <= 5 {
                            eprintln!("  DUAL_FEAS_VIOLATION: j={j} status={status:?} d[j]={dj} delta[j]={:?}", if j < n_orig { delta[j] } else { MSide::None });
                        }
                    }
                }
                eprintln!("  total dual_feasibility_violations={dual_violations} (out of nonbasic columns)");
            }
            if profile_phases {
                prof_phases::report(wall_t0.elapsed().as_nanos() as usize);
            }
            return Some(SimplexResult { status: Status::Infeasible, x: None });
        }
        // chuzc1 + BFRT pass 1, combined: `bland_mode` still needs the
        // *whole* pool sorted by `j` (its walk has no early-exit shortcut
        // to lean on), but the common case's walk almost always stops
        // after just the first one or two candidates in ascending-`(ratio,
        // j)` order (measured on Netlib `greenbea`: ~521 candidates/
        // iteration built, but the walk below consumes ~0.5 on average) —
        // a full `sort_by` pays `O(k log k)` to answer a question the walk
        // only needed the first `w << k` of. A min-heap
        // (`BinaryHeap<Reverse<Cand>>`, `O(k)` to build) popped one at a
        // time reproduces the exact same ascending order (`Cand`'s `Ord`
        // impl, same formula the old `sort_by` used) but costs `O(w log
        // k)` instead — `crate::simplex`'s own `ChuzcCand`/`BinaryHeap`
        // already does this for the classical path; this mirrors it here.
        // `candidates.drain(..)` (not `.into_iter()`) so `candidates`
        // itself keeps its allocation for the next iteration's PRICE scan
        // to reuse, matching this function's own preallocate-once
        // convention for its other per-iteration buffers.
        let n_candidates = candidates.len();
        sorted_prefix.clear();
        let mut k_star: Option<usize> = None;
        // BFRT (\S4.6), generalized: cumulative flip capacity is an
        // `Affine1` running sum, compared lexicographically against
        // `w_r`; a candidate with a genuinely infinite width (`None`)
        // always stops the walk immediately, exactly like the classical
        // method's own guard (see [`width_affine`]'s own docs). Declared
        // here, outside the `bland_mode` branch below, so its final value
        // is still readable in the `k_star == None` debug print past the
        // branch.
        let mut cum = Affine1::ZERO;
        if bland_mode {
            timed!(profile_phases, prof_phases::CHUZC1, {
                // Same ascending `(ratio, j)` order the non-`bland` heap
                // path below produces (`Cand`'s own `Ord` impl) -- *not* a
                // plain `j` sort. `j` still breaks ties deterministically
                // (Bland's rule's actual anti-cycling guarantee), but the
                // BFRT walk just below fundamentally requires ratio-
                // ascending order to be a ratio test at all: sorting by `j`
                // alone let it walk straight past cheap, well-conditioned
                // candidates into whichever tiny/ill-conditioned pivot
                // happened to sit at a small column index, confirmed as
                // a contributing mechanism behind Netlib `greenbea`'s
                // singular-basis failure once `bland_mode` latched.
                candidates.sort();
            });
            timed!(profile_phases, prof_phases::BFRT, {
                for (idx, cand) in candidates.iter().enumerate() {
                    let Some(width) = cache.width[cand.j] else {
                        k_star = Some(idx);
                        break;
                    };
                    let new_cum = cum.add(width.scale(cand.hat_alpha.abs()));
                    if bfrt_reached(w_r, new_cum, x_b_base[r]) {
                        k_star = Some(idx);
                        break;
                    }
                    cum = new_cum;
                }
            });
        } else {
            let heap_t0 = profile_phases.then(std::time::Instant::now);
            let mut heap: BinaryHeap<Reverse<Cand>> = timed!(profile_phases, prof_phases::CHUZC1, {
                heap_buf.clear();
                heap_buf.extend(candidates.drain(..).map(Reverse));
                BinaryHeap::from(std::mem::take(&mut heap_buf))
            });
            if let Some(t0) = heap_t0 {
                prof_phases::CHUZC1_HEAP.fetch_add(t0.elapsed().as_nanos() as usize, std::sync::atomic::Ordering::Relaxed);
            }
            timed!(profile_phases, prof_phases::BFRT, {
                while let Some(Reverse(cand)) = heap.pop() {
                    let idx = sorted_prefix.len();
                    sorted_prefix.push(cand);
                    let Some(width) = cache.width[cand.j] else {
                        k_star = Some(idx);
                        break;
                    };
                    let new_cum = cum.add(width.scale(cand.hat_alpha.abs()));
                    if bfrt_reached(w_r, new_cum, x_b_base[r]) {
                        k_star = Some(idx);
                        break;
                    }
                    cum = new_cum;
                }
            });
            heap_buf = heap.into_vec();
        }
        // Unified view over "the sorted candidates, in the order chuzc1
        // above produced them" for pass 2 and the flip application below —
        // `bland_mode`'s own fully-sorted `candidates`, or the non-`bland`
        // path's lazily-built `sorted_prefix` (only ever holds `[0,
        // k_star]`, which is all pass 2 / the flip loop below ever index
        // into).
        let sorted: &[Cand] = if bland_mode { &candidates[..] } else { &sorted_prefix[..] };
        let Some(k_star) = k_star else {
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            // Same guard as the `Eligible = empty` site above, for the
            // same reason (see its own docs): an `Infeasible` conclusion
            // drawn at `1e-9` from a factorization this loop lets drift to
            // `1e-4` is not a conclusion. This is the site `greenbea`
            // actually reached (`site=bfrt_exhausted`, iteration 4715).
            if lu.update_count() > 0 {
                if profile_phases {
                    prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    prof_phases::REFACTOR_CAUSE_INFEAS_CHECK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                timed!(profile_phases, prof_phases::REFACTOR, {
                    lu = refactorize(std, &basis_pos, Some(&lu))?;
                    let (fresh_base, fresh_slope) = compute_rhs_affine(std, &cache, &nb_status)?;
                    lu.solve_into(&fresh_base, &mut lu_scratch, &mut x_b_base);
                    lu.solve_into(&fresh_slope, &mut lu_scratch, &mut x_b_slope);
                    snap_slopes(&mut x_b_slope);
                    rebuild_rows(&mut infeasible_rows, &mut row_dev, m, &row_bounds, &x_b_base, &x_b_slope);
                    fresh_d_into(std, &lu, &basis, &basis_pos, &active_cost, &mut fd_cb, &mut lu_scratch, &mut fd_y, &mut d);
                    if dse_refresh_on_refactor {
                        if let super::EdgeWeights::Dse(dse) = &mut weights {
                            *dse = super::DseState::from_basis(m, &lu);
                        }
                    }
                });
                continue;
            }
            // Every eligible column fully flipped and still short:
            // Proposition 4.6(ii) — genuine, reported directly.
            if std::env::var("ENOMOTO_DEBUG_EXT_INFEASIBLE").is_ok() {
                eprintln!(
                    "DEBUG_EXT_INFEASIBLE: site=bfrt_exhausted iter={_iter} r={r} basis_r={} d_dir={d_dir} w_r=({},{}) n_candidates={} cum=({},{}) remaining_m_side={remaining_m_side}",
                    basis[r], w_r.base, w_r.slope, n_candidates, cum.base, cum.slope
                );
            }
            if profile_phases {
                prof_phases::report(wall_t0.elapsed().as_nanos() as usize);
            }
            return Some(SimplexResult { status: Status::Infeasible, x: None });
        };

        // Harris-style pass 2 (`super::HARRIS_RATIO_TOL`'s own docs,
        // `super::solve_lp_dual_on`'s own BFRT): search *backward* from
        // `k_star` (never forward -- see that constant's own docs for why,
        // confirmed the hard way there) within a flat ratio-space window
        // for the candidate with the largest pivot magnitude, and pivot on
        // that one instead -- every candidate strictly before it still gets
        // flipped (pass 1 already proved that flipping any prefix up to
        // and including `k_star` never overshoots `w_r`, so a *smaller*
        // prefix cannot either). Confirmed empirically necessary, not
        // merely by analogy: without it, this phase converges to a
        // measurably wrong (not just imprecise) objective on Netlib
        // `cycle` even with Devex/DSE weighting and cost perturbation both
        // already active -- this crate's own `chuzc1`/BFRT history names
        // `cycle` directly as the reason this exact refinement exists at
        // all, and as the instance that broke two independently-tried,
        // more "faithful" alternatives (a per-row pivot-scaled window, and
        // a port of HiGHS's own `chooseFinalLargeAlpha`) -- this module
        // deliberately reuses the same flat, narrow window that one, not
        // either reverted alternative.
        let mut best_idx = k_star;
        timed!(profile_phases, prof_phases::BFRT, {
            let min_ratio = sorted[k_star].ratio - super::HARRIS_RATIO_TOL;
            let mut window_start = k_star;
            while window_start > 0 && sorted[window_start - 1].ratio >= min_ratio {
                window_start -= 1;
            }
            let mut best_abs = sorted[k_star].hat_alpha.abs();
            for (idx, cand) in sorted.iter().enumerate().take(k_star + 1).skip(window_start) {
                let abs_a = cand.hat_alpha.abs();
                if abs_a > best_abs {
                    best_abs = abs_a;
                    best_idx = idx;
                }
            }
            if profile_phases {
                prof_phases::HARRIS_WINDOW_SIZE_SUM.fetch_add(k_star - window_start + 1, std::sync::atomic::Ordering::Relaxed);
                let window_has_m_elsewhere = sorted[window_start..=k_star].iter().enumerate().any(|(off, cand)| {
                    let idx = window_start + off;
                    if idx == best_idx || !delta[cand.j].is_flagged() {
                        return false;
                    }
                    match nb_status[cand.j] {
                        Some(NbStatus::Lower) => delta[cand.j].has_lower(),
                        Some(NbStatus::Upper) => delta[cand.j].has_upper(),
                        None => false,
                    }
                });
                if window_has_m_elsewhere {
                    prof_phases::HARRIS_WINDOW_M_MISS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
        });

        // BFRT combined-flip (`super::solve_lp_dual_on`'s own "apply every
        // flip's combined effect on the basic variables in a single extra
        // FTRAN" — its own docs): every candidate about to be flipped
        // contributes its own bound jump (`width_affine`, signed by which
        // way it's flipping) to one summed sparse right-hand side *per
        // `Affine1` channel*, solved once each (dense-or-sparse dispatched
        // by `lu.should_use_dense_solve`, the same choice `finish`'s own
        // cleanup-lemma FTRAN uses) rather than one FTRAN per flipped
        // column. Reads `nb_status[cand.j]` while it's still pre-flip, so
        // this must run *before* the flip-commit loop just below. Its
        // effect on `x_b_base`/`x_b_slope` is applied after the entering
        // column's FTRAN (`combined_pending`): by default inside the
        // entering step's own `x_B` update loop (`merge_flip_xb`), whose
        // `theta` reads row `r`'s post-flip value, so every flip this same
        // iteration made is reflected exactly as if applied first.
        if profile_phases {
            prof_phases::BFRT_FLIPS.fetch_add(best_idx, std::sync::atomic::Ordering::Relaxed);
        }
        // `combined_pending`: a combined-flip FTRAN result (or, when
        // `combined_deferred`, its still-to-be-solved rhs) is waiting to be
        // applied to `x_B(M)`. The application runs right after the
        // entering column's FTRAN below (which reads neither `x_B(M)` nor
        // `InfeasibleRows`), so deferring it is invisible to the pivot path.
        let mut combined_pending = false;
        let mut combined_deferred = false;
        let mut combined_slope_nonzero = false;
        let mut combined_slope_nnz = 0usize;
        timed!(profile_phases, prof_phases::BFRT, {
            // Whether any flipped column's width carries an `M` term
            // (`width.slope != 0`, i.e. an `M`-flagged column flipping onto
            // or off its `M` side — `ENOMOTO_PROF_PHASES_EXT`'s own
            // `m_enter_via_flip` counts how rare that is). When none does,
            // every `combined_slope` entry is an exact (signed) zero, so the
            // slope channel's FTRAN is provably all-zero and is skipped
            // below — only its synthetic-clock ticks and its (zero) density
            // sample are replayed, so the CLOCK refactorization trigger and
            // the dense/sparse FTRAN dispatch (and with them the whole pivot
            // path) stay bit-for-bit unchanged.
            let mut slope_nonzero = false;
            for cand in &sorted[..best_idx] {
                let old = nb_status[cand.j].unwrap();
                let width = cache.width[cand.j].unwrap();
                let sigma = if old == NbStatus::Lower { 1.0 } else { -1.0 };
                let delta_x = width.scale(sigma);
                if delta_x.slope != 0.0 {
                    slope_nonzero = true;
                    for &(i, v) in std.cols.col(cand.j) {
                        if !combined_touched_flag[i] {
                            combined_touched_flag[i] = true;
                            combined_touched.push(i);
                        }
                        combined_base[i] += v * delta_x.base;
                        combined_slope[i] += v * delta_x.slope;
                    }
                } else {
                    for &(i, v) in std.cols.col(cand.j) {
                        if !combined_touched_flag[i] {
                            combined_touched_flag[i] = true;
                            combined_touched.push(i);
                        }
                        combined_base[i] += v * delta_x.base;
                    }
                }
            }
            if !combined_touched.is_empty() {
                #[cfg(test)]
                COMBINED_FLIP_COUNT.fetch_add(best_idx, std::sync::atomic::Ordering::Relaxed);
                // Both `Affine1` channels share one density history: they
                // are the same rhs pattern solved against the same basis,
                // differing only in the values scattered into it, so their
                // results fill in alike. Recorded on both branches (see
                // `super::solve_lp_dual_on`'s own identical dispatch).
                if profile_phases && density_bfrt.predicts_dense() && !lu.should_use_dense_solve(combined_touched.len()) {
                    prof_phases::DENSITY_GATE_FTRANS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                if lu.should_use_dense_solve_tracked(combined_touched.len(), &density_bfrt) {
                    // The slope channel (rare) is solved here on its own;
                    // the base channel either rides along the entering
                    // column's fused FTRAN below (`combined_deferred`) or is
                    // solved here as before. Density samples are recorded in
                    // the same (base, slope) order either way.
                    let slope_nnz = if slope_nonzero {
                        lu.solve_into(&combined_slope, &mut lu_scratch, &mut combined_alpha_slope)
                    } else {
                        lu.add_zero_rhs_solve_ticks(false);
                        0
                    };
                    if fused_bfrt_ftran && fused_dse_ftran && matches!(weights, super::EdgeWeights::Dse(_)) {
                        combined_deferred = true;
                        combined_slope_nnz = slope_nnz;
                    } else {
                        let base_nnz = lu.solve_into(&combined_base, &mut lu_scratch, &mut combined_alpha_base);
                        density_bfrt.record(base_nnz, m);
                        density_bfrt.record(slope_nnz, m);
                        if profile_phases {
                            prof_phases::DENSITY_BFRT_PPT.store((density_bfrt.expected() * 1000.0) as usize, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                } else {
                    sparse_base_buf.clear();
                    sparse_slope_buf.clear();
                    sparse_base_buf.extend(combined_touched.iter().map(|&i| (i, combined_base[i])));
                    let base_nnz = lu.solve_sparse_into(&sparse_base_buf, &mut sparse_scratch, &mut gp_scratch, &mut combined_alpha_base);
                    let slope_nnz = if slope_nonzero {
                        sparse_slope_buf.extend(combined_touched.iter().map(|&i| (i, combined_slope[i])));
                        lu.solve_sparse_into(&sparse_slope_buf, &mut sparse_scratch, &mut gp_scratch, &mut combined_alpha_slope)
                    } else {
                        lu.add_zero_rhs_solve_ticks(true);
                        0
                    };
                    density_bfrt.record(base_nnz, m);
                    density_bfrt.record(slope_nnz, m);
                    if profile_phases {
                        prof_phases::DENSITY_BFRT_PPT.store((density_bfrt.expected() * 1000.0) as usize, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                combined_pending = true;
                combined_slope_nonzero = slope_nonzero;
            }

            for cand in &sorted[..best_idx] {
                let old = nb_status[cand.j].unwrap();
                let new = match old {
                    NbStatus::Lower => NbStatus::Upper,
                    NbStatus::Upper => NbStatus::Lower,
                };
                if profile_phases && delta[cand.j].is_flagged() {
                    let lands_on_m = match new {
                        NbStatus::Lower => delta[cand.j].has_lower(),
                        NbStatus::Upper => delta[cand.j].has_upper(),
                    };
                    if lands_on_m {
                        prof_phases::M_ENTER_VIA_FLIP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                nb_status[cand.j] = Some(new);
            }
        });
        let q = sorted[best_idx].j;
        let dj_q = d[q];
        let alpha_q = a_p[q];
        if profile_phases && dj_q.abs() <= 1e-9 {
            prof_phases::DEGENERATE_PIVOTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if debug_dual_check {
            prev_q = Some(q);
            prev_r = Some(r);
            prev_alpha_q = alpha_q;
            prev_dj_q = dj_q;
            prev_dbg = Some((a_p.clone(), sorted[..best_idx].iter().map(|c| c.j).collect(), sorted.len()));
        }

        // `alpha_full = B^-1 A_q` (FTRAN of the entering column, against
        // the still-pre-pivot `lu`) — needed by the weight update below,
        // this phase's own incremental `x_B(M)` step, and `updateVerify`
        // just below, so it is computed here rather than alongside
        // `dense_q` further down. `alpha_q` above and `alpha_full[r]` are
        // the *same* pivot element via two independent routes (BTRAN+dot
        // vs FTRAN — `e_r^T B^-1 A_q = (B^-T e_r)^T A_q`) — expected to
        // agree exactly in infinite precision, which is exactly what
        // `updateVerify` below checks rather than merely assumes.
        // `dense_q` (the raw column) is built on the dense branch only: it is
        // this FTRAN's own rhs, exactly like `super::solve_lp_dual_on`'s
        // own `a_enter_buf` (`try_update_precomputed` further down no
        // longer needs the raw column itself — see its own docs — only the
        // `a_tilde_buf` this same FTRAN captures below). The FTRAN itself
        // takes the same dense-or-sparse fork that function's own entering-column solve
        // does (`FtLu::should_use_dense_solve`, keyed off the column's own
        // nonzero count via `std.cols.col(q)`), instead of always
        // densifying through `solve_into`: a real LP's constraint columns
        // are themselves sparse, and this loop already makes the identical
        // choice for the BFRT combined-flip solve just above — leaving the
        // entering column as the one FTRAN in this loop still forced dense
        // was a straight port gap from the classical method, not a
        // deliberate simplification.
        // Both branches also capture `a_tilde_buf` (the post-L/R, pre-U
        // intermediate) for this iteration's `try_update_precomputed` call
        // further down — see that method's own docs.
        let mut tau_ready = false;
        let mut combined_base_nnz = 0usize;
        timed!(profile_phases, prof_phases::FTRAN, {
            if profile_phases && density_col_aq.predicts_dense() && !lu.should_use_dense_solve(std.cols.col(q).len()) {
                prof_phases::DENSITY_GATE_FTRANS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            if lu.should_use_dense_solve_tracked(std.cols.col(q).len(), &density_col_aq) {
                // `dense_q` is kept all-zero between iterations and only
                // densified on this branch (the sparse branch never reads
                // it): scatter in, solve, scatter the same pattern back to
                // zero — `O(nnz(a_q))` instead of an `O(m)` `fill` every
                // pivot regardless of which branch runs.
                for &(i, v) in std.cols.col(q) {
                    dense_q[i] = v;
                }
                let result_nnz = if combined_deferred {
                    let (a_nnz, _, c_nnz) = lu.solve_into_triple_capture(
                        &dense_q,
                        &rho,
                        &combined_base,
                        &mut lu_scratch,
                        &mut tau_scratch,
                        &mut combined_scratch,
                        &mut alpha_full,
                        &mut tau,
                        &mut combined_alpha_base,
                        &mut a_tilde_buf,
                    );
                    combined_base_nnz = c_nnz;
                    tau_ready = true;
                    a_nnz
                } else if fused_dse_ftran && matches!(weights, super::EdgeWeights::Dse(_)) {
                    // DSE's own `tau = B^-1 rho_p` FTRAN (formerly run
                    // separately inside the weight update below, against
                    // this same pre-pivot `lu` and the same `rho`) fused
                    // into this one — see `solve_into_pair_capture`'s docs.
                    let (a_nnz, _) = lu.solve_into_pair_capture(&dense_q, &rho, &mut lu_scratch, &mut tau_scratch, &mut alpha_full, &mut tau, &mut a_tilde_buf);
                    tau_ready = true;
                    a_nnz
                } else {
                    lu.solve_into_capture(&dense_q, &mut lu_scratch, &mut alpha_full, &mut a_tilde_buf)
                };
                for &(i, _) in std.cols.col(q) {
                    dense_q[i] = 0.0;
                }
                density_col_aq.record(result_nnz, m);
            } else {
                let result_nnz = if combined_deferred {
                    let (a_nnz, _, c_nnz) = lu.solve_sparse_into_triple_capture(
                        std.cols.col(q),
                        &rho,
                        &combined_base,
                        &mut sparse_scratch,
                        &mut gp_scratch,
                        &mut tau_scratch,
                        &mut combined_scratch,
                        &mut alpha_full,
                        &mut tau,
                        &mut combined_alpha_base,
                        &mut a_tilde_buf,
                    );
                    combined_base_nnz = c_nnz;
                    tau_ready = true;
                    a_nnz
                } else if fused_dse_ftran && matches!(weights, super::EdgeWeights::Dse(_)) {
                    let (a_nnz, _) = lu.solve_sparse_into_pair_capture(
                        std.cols.col(q),
                        &rho,
                        &mut sparse_scratch,
                        &mut gp_scratch,
                        &mut tau_scratch,
                        &mut alpha_full,
                        &mut tau,
                        &mut a_tilde_buf,
                    );
                    tau_ready = true;
                    a_nnz
                } else {
                    lu.solve_sparse_into_capture(std.cols.col(q), &mut sparse_scratch, &mut gp_scratch, &mut alpha_full, &mut a_tilde_buf)
                };
                density_col_aq.record(result_nnz, m);
            }
            if profile_phases {
                prof_phases::DENSITY_COL_AQ_PPT.store((density_col_aq.expected() * 1000.0) as usize, std::sync::atomic::Ordering::Relaxed);
            }
        });
        if combined_deferred {
            density_bfrt.record(combined_base_nnz, m);
            density_bfrt.record(combined_slope_nnz, m);
            if profile_phases {
                prof_phases::DENSITY_BFRT_PPT.store((density_bfrt.expected() * 1000.0) as usize, std::sync::atomic::Ordering::Relaxed);
            }
        }
        if combined_pending && !merge_flip_xb {
            combined_pending = false;
            timed!(profile_phases, prof_phases::BFRT, {
                // Scanned over `0..m`, not `combined_touched`: FTRAN fill-in
                // can produce nonzeros outside the input's own sparsity
                // pattern (`super::solve_lp_dual_on`'s own combined-flip
                // update does the same, for the same reason).
                if combined_slope_nonzero {
                    for i in 0..m {
                        if combined_alpha_base[i] != 0.0 || combined_alpha_slope[i] != 0.0 {
                            x_b_base[i] -= combined_alpha_base[i];
                            x_b_slope[i] = snap_slope(x_b_slope[i] - combined_alpha_slope[i]);
                            refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, i);
                        }
                    }
                    for &i in &combined_touched {
                        combined_base[i] = 0.0;
                        combined_slope[i] = 0.0;
                        combined_touched_flag[i] = false;
                    }
                } else {
                    // Slope channel skipped (see `slope_nonzero`): its
                    // would-be result is all `±0.0`, which neither the
                    // `!= 0.0` test nor `snap_slope(x - ±0.0) == x` (every
                    // `x_b_slope` entry is already snapped) can observe.
                    for i in 0..m {
                        if combined_alpha_base[i] != 0.0 {
                            x_b_base[i] -= combined_alpha_base[i];
                            refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, i);
                        }
                    }
                    for &i in &combined_touched {
                        combined_base[i] = 0.0;
                        combined_touched_flag[i] = false;
                    }
                }
                combined_touched.clear();
            });
        }

        // updateVerify (`super::update_verify`'s own docs, HiGHS
        // `HEkkDualRow::updateVerify` equivalent): cross-checks this
        // pivot's element between PRICE's row-direction value (`alpha_q`,
        // already sitting in `a_p[q]`) and FTRAN's own column-direction
        // value (`alpha_full[r]`, just computed above) — placed here,
        // before anything derived from `alpha_full` is committed (the
        // incremental `x_B(M)` step, the pivot commit, the dual update),
        // so a numerically drifted pivot is caught at the earliest
        // possible point, exactly like the classical method's own
        // placement. On failure this pivot is discarded outright:
        // refactorize, fully resync `x_B(M)`/`InfeasibleRows` against the
        // rebuilt factorization, and let the next pass re-run chuzr/chuzc
        // fresh. The BFRT flips already committed to `nb_status` this same
        // iteration (if any) are *not* rolled back — `super::solve_lp_dual_on`'s
        // own reasoning applies unchanged: those are independent,
        // already-valid degenerate steps, and the resync below recomputes
        // `x_B(M)` fresh against the (already-flipped) `nb_status` anyway,
        // so their effect is captured correctly regardless.
        //
        // The `lu.update_count() > 0` gate below (unchanged from the
        // classical method's own copy) exists because right after a
        // refactorization, refusing a pivot `update_verify`'s *tight*
        // (`UPDATE_VERIFY_TOL`, `1e-7`) tolerance rejects just reproduces
        // the identical chuzr/chuzc choice next iteration — measured on
        // Netlib `perold` as 19,860 verify-fail refactorizations in 20,000
        // iterations before that gate existed. Removing the gate outright
        // was tried and reverted: Netlib `bnl1` has its own persistent
        // pivot whose two values agree to `~2.4e-7` relative — comfortably
        // real and well-conditioned (magnitude `~3.3e-3`, nowhere near
        // `FT_MIN_PIVOT`), just barely over the *tight* tolerance — and
        // unconditionally re-checking it every iteration reproduced
        // `perold`'s exact old pathology on `bnl1` instead (22s vs `bnl1`'s
        // normal ~0.03s, confirmed via a full 73-problem sweep).
        //
        // `pivot_grossly_inconsistent` below is a *second*, much looser
        // check that fires regardless of `update_count`, catching only the
        // qualitatively different failure this module's own history
        // actually needs guarding against: Netlib `pilot4`
        // (`ENOMOTO_DEBUG_EXT_DUAL_CHECK`, under an experimental chuzr
        // change) hit a candidate right after a refactor where PRICE's
        // row-sum rounded to `alpha_q ~ 3.4e-9` while FTRAN's own value
        // came out `alpha_full[r] ~ 1e-21` — not "these two barely
        // disagree" (`bnl1`) or "these two agree well within tolerance"
        // (`perold`), but *aren't even the same order of magnitude*: one
        // is effectively exact zero, the other is noise mistaken for a
        // real value. `D_GROSS_MISMATCH_REL_TOL` sits far above
        // `UPDATE_VERIFY_TOL` specifically so it never fires on `bnl1`'s
        // ordinary `~2.4e-7` disagreement, confirmed via the same
        // full-suite sweep (identical result to the original gated
        // behavior on all 73 problems). Committing `basis[r] = q` on a
        // grossly-mismatched pivot leaves the basis matrix itself
        // singular, after which every subsequent `x_B(M)`/`d`
        // recomputation (fresh *or* incremental) is computed from a
        // singular `B` and is garbage regardless — this is why catching it
        // here, before commit, is the only point that can actually help; a
        // periodic drift check ([`D_DRIFT_TOL`]) or a per-column dual-side
        // re-verification, both tried first, cannot: both would recompute
        // from the same already-singular basis and simply agree with the
        // garbage more precisely.
        // Deliberately *not* `.max(super::FT_MIN_PIVOT)` the way
        // `super::update_verify`'s own `scale` is: flooring at
        // `FT_MIN_PIVOT` is exactly right for a tight, `~1e-7`-scale
        // relative tolerance (it keeps two values that both happen to be
        // smaller than `FT_MIN_PIVOT` from registering a huge *relative*
        // difference over a *tiny* *absolute* one), but it defeats this
        // check's whole purpose at `D_GROSS_MISMATCH_REL_TOL`'s much
        // looser scale: `pilot4`'s own `alpha_q ~ 3.4e-9` vs
        // `alpha_full[r] ~ 1e-21` divided by `FT_MIN_PIVOT` (`1e-7`) comes
        // out as only `~0.03` — comfortably under `0.5` — even though the
        // two values don't share an order of magnitude with *each other*.
        // A tiny fixed floor here only guards the literal `0/0` case.
        let pivot_grossly_inconsistent = {
            let scale = alpha_q.abs().max(alpha_full[r].abs()).max(1e-300);
            (alpha_q - alpha_full[r]).abs() / scale > D_GROSS_MISMATCH_REL_TOL
        };
        if pivot_grossly_inconsistent || (!update_verify_disabled && lu.update_count() > 0 && !super::update_verify(alpha_q, alpha_full[r])) {
            if std::env::var("ENOMOTO_DEBUG_D_DRIFT_EXT").is_ok() {
                eprintln!("DEBUG_D_DRIFT: VERIFY_FAIL at iter={_iter} q={q} r={r} alpha_q={alpha_q} alpha_full_r={}", alpha_full[r]);
            }
            if stuck_row == Some(r) {
                stuck_row_streak += 1;
            } else {
                stuck_row = Some(r);
                stuck_row_streak = 1;
            }
            if ban_discarded_candidates {
                if discard_row == Some(r) {
                    if !discard_banned_cols.contains(&q) {
                        discard_banned_cols.push(q);
                    }
                } else {
                    discard_row = Some(r);
                    discard_banned_cols.clear();
                    discard_banned_cols.push(q);
                }
            }
            // The PRICE-vs-FTRAN disagreement this branch caught is a
            // numerical failure of the factorization in exactly §2.4's
            // sense, whether it was gross (`ILLCOND`) or merely past
            // `update_verify`'s tolerance (`VERIFY`).
            note_numeric_trouble!();
            if profile_phases {
                prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if pivot_grossly_inconsistent {
                    prof_phases::REFACTOR_CAUSE_ILLCOND.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                } else {
                    prof_phases::REFACTOR_CAUSE_VERIFY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
            timed!(profile_phases, prof_phases::REFACTOR, {
                lu = refactorize(std, &basis_pos, Some(&lu))?;
                let (fresh_base, fresh_slope) = compute_rhs_affine(std, &cache, &nb_status)?;
                lu.solve_into(&fresh_base, &mut lu_scratch, &mut x_b_base);
                lu.solve_into(&fresh_slope, &mut lu_scratch, &mut x_b_slope);
                snap_slopes(&mut x_b_slope);
                rebuild_rows(&mut infeasible_rows, &mut row_dev, m, &row_bounds, &x_b_base, &x_b_slope);
                fresh_d_into(std, &lu, &basis, &basis_pos, &active_cost, &mut fd_cb, &mut lu_scratch, &mut fd_y, &mut d);
                if dse_refresh_on_refactor {
                    if let super::EdgeWeights::Dse(dse) = &mut weights {
                        *dse = super::DseState::from_basis(m, &lu);
                    }
                }
            });
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            if combined_pending {
                // Unapplied flip result: the resync above already rebuilt
                // `x_B(M)` from the (flipped) `nb_status`; just reset the
                // combined-rhs buffers.
                for &i in &combined_touched {
                    combined_base[i] = 0.0;
                    combined_slope[i] = 0.0;
                    combined_touched_flag[i] = false;
                }
                combined_touched.clear();
            }
            continue;
        }

        // `q` is the only way an M-flagged column ever leaves its `M` side
        // (`score2_c2_tol`'s own docs) — mark it resolved exactly once,
        // here (only once this pivot is confirmed to actually commit, not
        // at `q`'s own selection above — a pivot the guard just above
        // discarded never happened, and `iters_since_m_progress` below
        // specifically needs to *not* reset on a discarded, no-progress
        // iteration to do its job). A plain index into `delta` rather than
        // a lookup in `m_flagged_cols` since `q < n_orig` is guaranteed
        // whenever `delta[q].is_flagged()` (slack columns are never
        // M-flagged, `delta_of`'s own docs). `q` becoming basic resolves a
        // genuinely free (`MSide::Both`) column exactly like a one-sided
        // one — it stops being nonbasic-at-either-M-side at all, regardless
        // of which side it left from.
        if profile_phases && q < n_orig && delta[q].is_flagged() {
            let q_was_m = match nb_status[q] {
                Some(NbStatus::Lower) => delta[q].has_lower(),
                Some(NbStatus::Upper) => delta[q].has_upper(),
                None => false,
            };
            if q_was_m {
                prof_phases::M_EXIT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        if q < n_orig && delta[q].is_flagged() && !resolved_m[q] {
            resolved_m[q] = true;
            remaining_m_side -= 1;
            iters_since_m_progress = 0;
        }
        // Reaching here means row `r`'s pivot just committed successfully
        // (the discard branch above always `continue`s) — whatever streak
        // `stuck_row`/`stuck_row_streak` were tracking for this row is
        // over, boosted or not (`stuck_row_boost_factor`'s own docs).
        if stuck_row == Some(r) {
            stuck_row = None;
            stuck_row_streak = 0;
        }
        if discard_row == Some(r) {
            discard_row = None;
            discard_banned_cols.clear();
        }

        // Entering column's own incremental `x_B(M)` step (the other half
        // of `super::solve_lp_dual_on`'s combined-flip/`alpha`-scaled
        // primal-step pair, generalized): `theta_q` is how far `q` moves
        // from its own pre-pivot nonbasic value to reach the leaving row's
        // target bound, computed from `x_b_base[r]`/`x_b_slope[r]` *after*
        // the BFRT combined-flip step above already applied this same
        // iteration's own flips to them. Reads `nb_status[q]` here, before
        // it's cleared at the pivot commit below. Row `r`'s own new value
        // is assigned directly (`nb_val_q + theta_q`) rather than trusted
        // from the `alpha`-loop below, for the same reason
        // `super::solve_lp_dual_on` does: `alpha_full[r]` lands exactly on
        // `target` by construction, so the direct assignment is both
        // simpler and exact.
        let old_status_q = nb_status[q].unwrap();
        let nb_val_q = nb_value_affine(&cache, old_status_q, q)?;
        // `basis[r]`'s own genuinely-infinite side can never be the one
        // `d_dir` just picked (`row_deviation`'s own docs: `dev_minus`/
        // `dev_plus` are only ever `Some` when `cache.lower`/`cache.upper`
        // already is), so this mirrors [`nb_value_affine`]'s own "should
        // never happen" `?` rather than needing its own bailout print.
        let target = if d_dir > 0 { cache.lower[basis[r]]? } else { cache.upper[basis[r]]? };
        // With a still-unapplied flip result (`combined_pending`), row `r`'s
        // post-flip value is exactly what the separate flip pass would have
        // left there.
        let x_r = if combined_pending {
            let ca = combined_alpha_base[r];
            let cs = if combined_slope_nonzero { combined_alpha_slope[r] } else { 0.0 };
            if ca != 0.0 || cs != 0.0 {
                let xs = if combined_slope_nonzero { snap_slope(x_b_slope[r] - cs) } else { x_b_slope[r] };
                Affine1::new(x_b_base[r] - ca, xs)
            } else {
                Affine1::new(x_b_base[r], x_b_slope[r])
            }
        } else {
            Affine1::new(x_b_base[r], x_b_slope[r])
        };
        let theta_base = (x_r.base - target.base) / alpha_q;
        let theta_slope = (x_r.slope - target.slope) / alpha_q;
        timed!(profile_phases, prof_phases::XB_UPDATE, {
            if combined_pending {
                // Flip result and entering step in one pass: per row, the
                // exact operations of the separate flip pass followed by
                // those of the entering step, then one `refresh_row`.
                for i in 0..m {
                    let ca = combined_alpha_base[i];
                    let cs = if combined_slope_nonzero { combined_alpha_slope[i] } else { 0.0 };
                    let flip = ca != 0.0 || cs != 0.0;
                    let a = alpha_full[i];
                    if !flip && a == 0.0 {
                        continue;
                    }
                    if flip {
                        x_b_base[i] -= ca;
                        if combined_slope_nonzero {
                            x_b_slope[i] = snap_slope(x_b_slope[i] - cs);
                        }
                    }
                    if a != 0.0 {
                        x_b_base[i] -= a * theta_base;
                        if theta_slope != 0.0 {
                            x_b_slope[i] = snap_slope(x_b_slope[i] - a * theta_slope);
                        }
                    }
                    refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, i);
                }
                for &i in &combined_touched {
                    combined_base[i] = 0.0;
                    combined_slope[i] = 0.0;
                    combined_touched_flag[i] = false;
                }
                combined_touched.clear();
            } else if theta_slope == 0.0 {
                // `M`-free step (every iteration once delta=0 is reached,
                // and most before it): the slope channel's update is an
                // exact no-op — every `x_b_slope` entry is already
                // `snap_slope`-normalized (never `-0.0`, never below
                // `X_B_SLOPE_NOISE` in magnitude), so `x - a*(±0.0)` is `x`
                // bit-for-bit and `snap_slope(x) == x` — so it is skipped.
                for i in 0..m {
                    let a = alpha_full[i];
                    if a != 0.0 {
                        x_b_base[i] -= a * theta_base;
                        refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, i);
                    }
                }
            } else {
                for i in 0..m {
                    let a = alpha_full[i];
                    if a != 0.0 {
                        x_b_base[i] -= a * theta_base;
                        x_b_slope[i] = snap_slope(x_b_slope[i] - a * theta_slope);
                        refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, i);
                    }
                }
            }
            x_b_base[r] = nb_val_q.base + theta_base;
            x_b_slope[r] = snap_slope(nb_val_q.slope + theta_slope);
        });

        // This pivot's actual objective contribution — `theta_q(M) * dj_q`
        // generalized to this module's own `Affine1` step (`dj_q` is
        // always `M`-independent, see this module's own `d` docs) — is
        // `super::solve_lp_dual_on`'s own exact `theta_q * dj_q`, feeding
        // the stall check further down.
        let contribution_base = theta_base * dj_q;
        let contribution_slope = theta_slope * dj_q;

        timed!(profile_phases, prof_phases::DSE_UPDATE, match &mut weights {
            super::EdgeWeights::Devex(dv) => dv.update_after_pivot(r, &alpha_full),
            super::EdgeWeights::Dse(dse) => {
                if !tau_ready {
                    timed!(profile_phases, prof_phases::DSE_FTRAN, lu.solve_into(&rho, &mut lu_scratch, &mut tau));
                }
                dse.update_after_pivot(r, &alpha_full, &tau, &rho);
            }
        });
        if profile_work {
            use std::sync::atomic::Ordering::Relaxed;
            prof_phases::STAT_TAU_NNZ.fetch_add(tau.iter().filter(|v| **v != 0.0).count(), Relaxed);
            prof_phases::STAT_ALPHA_NNZ.fetch_add(alpha_full.iter().filter(|v| **v != 0.0).count(), Relaxed);
        }

        // Stall detection (`super::solve_lp_dual_on`'s own convention, its
        // own docs): only a pivot whose objective contribution is itself
        // essentially zero counts as "no real progress" — *not* every
        // iteration unconditionally (an earlier version of this loop did
        // exactly that, which latched `bland_mode` on for any sufficiently
        // long-running but otherwise perfectly healthy solve, e.g. Netlib
        // `degen2`'s ~2900 main-phase iterations, no genuine cycling
        // involved at all). Now the classical method's own exact check,
        // `(theta_q * dj_q).abs() < STALL_PROGRESS_EPS`, via
        // `contribution_base`/`contribution_slope` above — an earlier
        // version of this check used `dj_q` alone as a proxy, before this
        // module's own incremental `x_B(M)` step existed to compute a real
        // `theta_q`; see that computation's own docs for why a nonzero
        // slope is treated as unambiguous progress rather than folded into
        // the base-term comparison.
        if contribution_slope == 0.0 && contribution_base.abs() < super::STALL_PROGRESS_EPS {
            stall_count += 1;
            if stall_count > stall_limit {
                bland_mode = true;
            }
        } else {
            stall_count = 0;
        }

        // Secondary anti-cycling signal: a long run of pivots each reporting
        // *genuine* (non-stalling) objective progress by the check just
        // above, yet never changing which rows are infeasible at all,
        // still isn't converging in any way that matters — a long streak
        // of tiny-but-nonzero contributions that keeps `stall_count` reset
        // every iteration, so it alone never latches `bland_mode`.
        // Confirmed necessary on Netlib `pilot4` after restricting `M`-
        // bounding to `S` (§4.2): `infeasible_rows.rows.len()` sat at
        // exactly `31` for 16,000+ consecutive iterations post-`delta=0`
        // (`ENOMOTO_DEBUG_EXT_TRACE`'s own trace), `stall_count` never
        // exceeding `0`, until this loop's own `MAX_ITERS` budget was
        // exhausted and the whole solve fell back to the classical
        // `BIG_M`-substituted path (55s+, versus ~0.03s once this trigger
        // fires and `bland_mode` breaks the pattern). Deliberately a much
        // larger threshold than `stall_limit` alone, and reset on *any*
        // change in the infeasible set's size (not just a decrease) rather
        // than every iteration unconditionally: a healthy, merely slow
        // solve's infeasible-row count fluctuates (grows and shrinks) far
        // more often than this plateau tolerates, which is exactly what
        // distinguishes it from `pilot4`'s own multi-thousand-iteration
        // plateau — Netlib `degen2` (this module's own `stall_count` docs
        // name it directly as a false-positive risk for an eager,
        // every-iteration stall trigger) never comes close to a static
        // infeasible count for anywhere near this many consecutive
        // iterations across its own ~2900-iteration main phase.
        // Measured against the best (smallest) infeasible count seen so
        // far, not against the previous iteration's: an exact-equality
        // test is defeated by a stall that merely *oscillates*. Netlib
        // `greenbea` does exactly that — past iteration ~8000 its count
        // alternates between 248 and 249 (two variables trading places on
        // a single row, `analysis/greenbea_20260921_021218.md` §4), which
        // resets an equality-based counter every second iteration and let
        // the solve burn its remaining ~12,000 iterations with
        // `bland_mode` never latching. "No new best in
        // `infeasible_plateau_limit` iterations" catches both that and the
        // literally-static `pilot4` plateau this check was written for,
        // and is still reset by any genuine progress.
        let infeasible_len = infeasible_rows.rows.len();
        let made_infeasible_progress = infeasible_len < best_infeasible_len;
        if made_infeasible_progress {
            best_infeasible_len = infeasible_len;
        }
        // `remaining_m_side` progress also counts (see its own
        // `best_remaining_m_side` docs above) — a solve can keep steadily
        // resolving M-side columns while the infeasible-row *count*'s
        // minimum sits unbeaten simply because that set is churning
        // (Netlib `dfl001`), and treating that as a stall latches
        // `bland_mode` on a solve that was never stuck.
        let made_m_side_progress = remaining_m_side < best_remaining_m_side;
        if made_m_side_progress {
            best_remaining_m_side = remaining_m_side;
        }
        if made_infeasible_progress || made_m_side_progress {
            infeasible_plateau_count = 0;
        } else {
            infeasible_plateau_count += 1;
            if infeasible_plateau_count > infeasible_plateau_limit {
                bland_mode = true;
            }
        }

        let leaving_var = basis[r];
        nb_status[leaving_var] = Some(if d_dir > 0 { NbStatus::Lower } else { NbStatus::Upper });
        basis_pos[leaving_var] = None;
        basis[r] = q;
        basis_pos[q] = Some(r);
        nb_status[q] = None;
        row_bounds.assign(r, q, &cache, &noise_feasible);
        if price_nonbasic_only {
            // `HighsSparseMatrix::update`'s own swap scheme: `q` (always
            // non-fixed — PRICE only ever offers those) moves from each of
            // its rows' nonbasic part to the basic part, the leaving
            // column (if non-fixed; a fixed one was never in this matrix)
            // the other way. Within-row order is irrelevant to `a_p`'s
            // values: each `a_p[j]` still accumulates over rows `i` in
            // ascending order, one entry per row.
            for &(i, _) in std.cols.col(q) {
                let lo = price_start[i];
                let last = price_nb_end[i] - 1;
                let pos = lo + price_col[lo..=last].iter().position(|&c| c as usize == q).expect("entering column missing from its row's nonbasic PRICE partition");
                price_col.swap(pos, last);
                price_val.swap(pos, last);
                price_nb_end[i] = last;
            }
            if std.lb[leaving_var] != std.ub[leaving_var] {
                for &(i, _) in std.cols.col(leaving_var) {
                    let first = price_nb_end[i];
                    let hi = price_start[i + 1];
                    let pos = first + price_col[first..hi].iter().position(|&c| c as usize == leaving_var).expect("leaving column missing from its row's basic PRICE partition");
                    price_col.swap(pos, first);
                    price_val.swap(pos, first);
                    price_nb_end[i] = first + 1;
                }
            }
        }
        if debug_delta0 && delta0_iter.is_none() {
            let all_off_m_side = m_flagged_cols.iter().all(|&j| match nb_status[j] {
                None => true,
                Some(NbStatus::Lower) => cache.lower[j].map_or(true, |a| a.slope == 0.0),
                Some(NbStatus::Upper) => cache.upper[j].map_or(true, |a| a.slope == 0.0),
            });
            if all_off_m_side {
                delta0_iter = Some(_iter);
                eprintln!(
                    "DEBUG_EXT_DELTA0: at delta=0 (iter={_iter}): infeasible_rows={} q_was_m_flagged={} stall_count={stall_count} n_m_flagged={} m_resolved_by_entering={}",
                    infeasible_rows.rows.len(),
                    m_flagged_cols.contains(&q),
                    m_flagged_cols.len(),
                    m_flagged_cols.len() - remaining_m_side
                );
            }
        }
        if debug_delta0 && delta0_iter.is_some() && (_iter % 500 == 0) {
            eprintln!(
                "DEBUG_EXT_DELTA0: iter={_iter} infeasible_rows={} stall_count={stall_count}",
                infeasible_rows.rows.len()
            );
        }
        // Row `r`'s occupant identity just changed (`leaving_var` -> `q`)
        // — `row_deviation` reads bounds off `basis[i]`, so this must be
        // re-checked against the *new* occupant now that `basis[r]` holds
        // it, superseding whatever the `alpha`-loop above computed for
        // index `r` against the stale identity (`super::solve_lp_dual_on`'s
        // own post-swap `infeasible_rows.set` call, same reason).
        refresh_row(&mut infeasible_rows, &mut row_dev, &row_bounds, &x_b_base, &x_b_slope, r);

        // Incremental dual update (Huangfu & Hall §2.2.3,
        // `super::solve_lp_dual_on`'s own derivation via Sherman-Morrison
        // on `B' = B(I + (alpha - e_p)e_p^T)`): `d[j] -= theta_d*a_p[j]`
        // for every column PRICE touched this iteration, using the *real*
        // pivot's own `dj_q`/`alpha_q` — a BFRT flip never touches `d`
        // (depends only on `B`/`c_B`, neither of which a flip changes),
        // so this runs once per iteration regardless of how many columns
        // just got flipped.
        let theta_d = dj_q / alpha_q;
        if debug_d_drift_ext && theta_d.abs() > 1e3 {
            eprintln!(
                "DEBUG_D_DRIFT: LARGE theta_d at iter={_iter}: q={q} dj_q={dj_q} alpha_q={alpha_q} theta_d={theta_d} touched_cols={} r={r}",
                touched_cols.len()
            );
        }
        timed!(profile_phases, prof_phases::DUAL_UPDATE, {
            // One pass (update + clear) instead of two over `touched_cols`.
            for &j in &touched_cols {
                d[j] -= theta_d * a_p[j];
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            if price_nonbasic_only {
                // HiGHS `HEkkDual::updateDual`'s own closing assignments —
                // see `price_nonbasic_only`'s own docs.
                d[q] = 0.0;
                d[leaving_var] = -theta_d;
            }
        });

        // Forrest-Tomlin incremental update instead of a fresh
        // refactorization every iteration — `super::solve_lp_dual_on`'s
        // own trigger scheme: an outright-rejected pivot always
        // refactorizes; otherwise, every `FT_CHECK_INTERVAL` iterations,
        // a cheap eta-fill check (`fill_count`) runs, and at the coarser
        // `FT_CHECK_INTERVAL * RESIDUAL_CHECK_MULTIPLIER` cadence a
        // genuine `‖A_B x_B - rhs‖` drift check ([`residual_norm`]'s own
        // docs) against a *freshly recomputed* right-hand side
        // (`compute_rhs_affine`, paid only when this coarse cadence
        // actually fires — `x_B(M)` is otherwise maintained incrementally
        // pivot-to-pivot now, see this function's own docs on why the old
        // "recomputed fresh every iteration" design was replaced) —
        // refactorizing only if one of these two actually finds a
        // problem, *not* unconditionally every `FT_CHECK_INTERVAL` (an
        // earlier version of this loop did exactly that, needlessly
        // discarding a healthy Forrest-Tomlin chain most of the time).
        // Both `Affine1` channels are checked, not just `base`: almost
        // every comparison in this module (`Affine1::cmp_lex`,
        // `Score2::cmp_lex`) decides on the *slope* term first, so a
        // drifted `x_b_slope` is at least as dangerous to correctness as a
        // drifted `x_b_base` — checking only one channel would leave the
        // module's single most decision-relevant quantity unguarded.
        // `try_update_precomputed` wants `a_tilde_buf`/`e_tilde_buf` —
        // already captured above as a side effect of this same iteration's
        // own BTRAN (`rho`, for `e_tilde_buf`) and FTRAN (`alpha_full`, for
        // `a_tilde_buf`); nothing between either capture point and here
        // writes through them, so no re-derivation is needed — see
        // `FtLu::try_update_precomputed`'s own docs (this used to be a
        // plain `try_update(r, &dense_q, ...)`, which recomputed both from
        // scratch every single pivot).
        since_check += 1;
        let mut need_refactor = timed!(
            profile_phases,
            prof_phases::FT_UPDATE,
            !lu.try_update_precomputed(r, &a_tilde_buf, &e_tilde_buf, super::FT_MIN_PIVOT)
        );
        if need_refactor {
            // Trigger (2), the Forrest-Tomlin update rejecting its own
            // pivot — the most direct evidence there is that the current
            // factorization is too loose (§2.4).
            note_numeric_trouble!();
            if profile_phases {
                prof_phases::REFACTOR_CAUSE_TRY_UPDATE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        // Trigger (4) ([`ft_max_updates`]'s own docs) — an unconditional,
        // every-iteration check (like `super::FT_MAX_UPDATES`'s own site),
        // not gated by `XB_CHECK_INTERVAL`: it is a single `usize`
        // comparison, cheap enough to run every pivot regardless.
        if profile_phases {
            prof_phases::MAX_UPDATE_STREAK.fetch_max(lu.update_count(), std::sync::atomic::Ordering::Relaxed);
        }
        if !need_refactor && lu.update_count() > ft_max_updates(m) {
            need_refactor = true;
            if profile_phases {
                prof_phases::REFACTOR_CAUSE_MAX_UPDATES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        // Trigger (5) ([`synth_clock_should_refactor`]'s own docs) — same
        // unconditional, every-iteration placement as trigger (4) just
        // above (two `Cell` reads plus a comparison, not worth gating
        // behind `XB_CHECK_INTERVAL`).
        if !need_refactor && synth_clock_should_refactor(&lu) {
            need_refactor = true;
            if profile_phases {
                prof_phases::REFACTOR_CAUSE_CLOCK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        if !need_refactor && since_check >= XB_CHECK_INTERVAL {
            since_check = 0;
            let bump_too_big = lu.fill_count() > super::FT_BUMP_LIMIT_FACTOR * m.max(1);
            if bump_too_big {
                need_refactor = true;
                if profile_phases {
                    prof_phases::REFACTOR_CAUSE_BUMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            } else {
                // Checked every `XB_CHECK_INTERVAL` iterations here, not
                // gated by a further `RESIDUAL_CHECK_MULTIPLIER`-style
                // coarser cadence the way `super::solve_lp_dual_on`'s own
                // plain-`f64` drift check is — see `XB_CHECK_INTERVAL`'s
                // own docs for why this module's `Affine1` slope channel
                // needs a tighter leash.
                let (fresh_base, fresh_slope) = compute_rhs_affine(std, &cache, &nb_status)?;
                let (resid_base, resid_slope) = residual_norm_affine(
                    std,
                    &basis,
                    &x_b_base,
                    &x_b_slope,
                    &fresh_base,
                    &fresh_slope,
                    &mut resid_scratch_base,
                    &mut resid_scratch_slope,
                );
                // Per-solve escalation ladder — see [`XB_DRIFT_TOL`]'s own
                // docs. `drift_trigger_count` only ever grows within this
                // one call to `solve_lp_dual_extended`, so a solve that
                // hasn't yet proven itself drift-heavy always compares
                // against the unmodified `xb_drift_tol` (step `0`).
                let escalation_steps = (drift_trigger_count / XB_DRIFT_ESCALATION_STEP) as i32;
                let effective_drift_tol = (xb_drift_tol * XB_DRIFT_ESCALATION_FACTOR.powi(escalation_steps)).min(XB_DRIFT_TOL_MAX);
                if std::env::var("ENOMOTO_DEBUG_XB_DRIFT_EXT").is_ok() {
                    eprintln!(
                        "DEBUG_XB_DRIFT: iter={_iter} resid_base={resid_base:.3e} resid_slope={resid_slope:.3e} drift_trigger_count={drift_trigger_count} effective_tol={effective_drift_tol:.3e}"
                    );
                }
                need_refactor = resid_base > effective_drift_tol || resid_slope > effective_drift_tol;
                if need_refactor {
                    drift_trigger_count += 1;
                    note_numeric_trouble!();
                    if profile_phases {
                        prof_phases::REFACTOR_CAUSE_DRIFT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                // `d`'s own independent drift check ([`D_DRIFT_TOL`]'s own
                // docs — added after a real false `Infeasible` on Netlib
                // `pilot4` traced to exactly this gap): only run when the
                // `x_B(M)` check above didn't already decide to
                // refactorize, same as that check's own short-circuit
                // intent — no point paying for a second O(nnz) BTRAN-based
                // recomputation when a refactor (which calls
                // `fresh_d_into` on `d` directly, unconditionally) is
                // about to happen anyway. Gated by [`since_d_drift_check`]'s
                // own coarser cadence (see its declaration) — the residual
                // below also excludes fixed columns (`lb == ub`), which
                // PRICE never updates `d` for, so they'd otherwise swamp the
                // genuine-drift signal with a constant, non-drift offset.
                if !need_refactor {
                    since_d_drift_check += 1;
                }
                if !need_refactor && since_d_drift_check >= super::RESIDUAL_CHECK_MULTIPLIER {
                    since_d_drift_check = 0;
                    fresh_d_into(std, &lu, &basis, &basis_pos, &active_cost, &mut fd_cb, &mut lu_scratch, &mut fd_y, &mut fresh_d_buf);
                    let mut resid_sq = 0.0f64;
                    let mut scale_sq = 0.0f64;
                    for j in 0..std.n_total {
                        if std.lb[j] == std.ub[j] {
                            continue;
                        }
                        let diff = d[j] - fresh_d_buf[j];
                        resid_sq += diff * diff;
                        scale_sq += fresh_d_buf[j] * fresh_d_buf[j];
                    }
                    let resid_d = resid_sq.sqrt();
                    let scale_d = scale_sq.sqrt().max(1.0);
                    if std::env::var("ENOMOTO_DEBUG_D_DRIFT_EXT").is_ok() {
                        eprintln!("DEBUG_D_DRIFT: iter={_iter} resid_d={resid_d:.3e} scale_d={scale_d:.3e} rel={:.3e}", resid_d / scale_d);
                    }
                    if resid_d > D_DRIFT_TOL * scale_d {
                        need_refactor = true;
                        note_numeric_trouble!();
                        if profile_phases {
                            prof_phases::REFACTOR_CAUSE_D_DRIFT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }
            }
        }
        if need_refactor {
            if profile_phases {
                prof_phases::REFACTOR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            timed!(profile_phases, prof_phases::REFACTOR, {
                lu = refactorize(std, &basis_pos, Some(&lu))?;
                // Full resync, not just the basis refactorization: whatever
                // drift this trigger just caught (or, on the `try_update`-
                // rejected path, without even needing to have measured any)
                // is cleared by re-solving `x_B(M)` from a fresh right-hand
                // side — `super::solve_lp_dual_on`'s own `resync_basics` after
                // an equivalent trigger, generalized to both channels — and
                // `InfeasibleRows` is rebuilt from scratch to match, since a
                // resync can change many rows' feasibility at once, outside
                // the reach of its own incremental `set` calls.
                let (fresh_base, fresh_slope) = compute_rhs_affine(std, &cache, &nb_status)?;
                lu.solve_into(&fresh_base, &mut lu_scratch, &mut x_b_base);
                lu.solve_into(&fresh_slope, &mut lu_scratch, &mut x_b_slope);
                snap_slopes(&mut x_b_slope);
                rebuild_rows(&mut infeasible_rows, &mut row_dev, m, &row_bounds, &x_b_base, &x_b_slope);
                fresh_d_into(std, &lu, &basis, &basis_pos, &active_cost, &mut fd_cb, &mut lu_scratch, &mut fd_y, &mut d);
                if dse_refresh_on_refactor {
                    if let super::EdgeWeights::Dse(dse) = &mut weights {
                        *dse = super::DseState::from_basis(m, &lu);
                    }
                }
            });
        }

    }

    // `max_iters` exceeded without reaching Step III — `None` (fall back
    // to the classical `BIG_M` path) rather than a false `Infeasible`.
    if profile_phases {
        prof_phases::report(wall_t0.elapsed().as_nanos() as usize);
    }
    if std::env::var("ENOMOTO_DEBUG_EXT_ITERS").is_ok() {
        eprintln!(
            "DEBUG_EXT_BAILOUT: max_iters={max_iters} exhausted bland_mode={bland_mode} stall_count={stall_count} remaining_m_side={remaining_m_side} n_m_flagged={}",
            m_flagged_cols.len()
        );
    }
    None
}

/// Step III: termination classification (Proposition 4.10 — `z1 < 0` iff
/// unbounded), the cleanup lemma (removing every nonbasic column still
/// sitting at its artificial `M` side), and final extraction.
///
/// Handoff to the classical method (the paper's own "restart the classical
/// dual simplex from the cleaned-up basis") is not yet implemented — see
/// this module's own docs' simplification list — so this function itself
/// performs the final extraction directly once cleanup leaves no
/// M-flagged nonbasic behind. Since the basis was already primal feasible
/// under the (now-irrelevant) `M` truncation and cleanup is a sequence of
/// zero-objective-change pivots (Lemma 4.9), the result is primal feasible
/// under the *true* bounds too.
fn finish(std: &StdForm, basis: &mut [usize], basis_pos: &mut [Option<usize>], nb_status: &mut [Option<NbStatus>], delta: &[MSide], cache: &ColCache, n_orig: usize, lu: sparse_lu::FtLu) -> Option<SimplexResult> {
    // `lu` is the main loop's own last-iteration factorization (already
    // exact for the current basis — the main loop's own termination check
    // just used it), passed in rather than rebuilt here: this function
    // runs once per solve, so the saving is small in isolation, but there
    // is no reason to pay for a `refactorize` this basis already has.
    let (x_b_base, x_b_slope) = solve_x_b(std, &lu, nb_status, cache)?;
    let c_b: Vec<f64> = basis.iter().map(|&bv| std.c[bv]).collect();
    let z_b = Affine1::new(dot(&c_b, &x_b_base), dot(&c_b, &x_b_slope));
    let mut z_n = Affine1::ZERO;
    for j in 0..std.n_total {
        if let Some(status) = nb_status[j] {
            z_n = z_n.add(nb_value_affine(cache, status, j)?.scale(std.c[j]));
        }
    }
    let z = z_b.add(z_n);
    // Small absolute tolerance rather than `TOL`: `z.slope` is a sum of
    // (possibly many) reduced-cost terms, so its floor scales with the
    // problem's own cost magnitudes, not with `TOL`'s coefficient-level
    // tightness — matches this crate's own precedent of using a looser,
    // separate tolerance for accumulated-magnitude checks (see
    // `simplex.rs::PRIMAL_FEAS_TOL`'s own docs for the same reasoning).
    const Z_SLOPE_TOL: f64 = 1e-7;
    if std::env::var("ENOMOTO_DEBUG_EXT_ITERS").is_ok() {
        eprintln!("DEBUG_EXT: z=({},{}) z0_base={}", z.base, z.slope, z_b.base);
    }
    if z.slope < -Z_SLOPE_TOL {
        return Some(SimplexResult { status: Status::Unbounded, x: None });
    }

    // Cleanup lemma (\S4.5 end): repeatedly pivot a nonbasic column still
    // sitting at its own artificial `M` side into the basis, at zero
    // objective cost, until none remain. `lu` (shadowing the outer one,
    // now genuinely mutable) carries across iterations via Forrest-Tomlin
    // `try_update` instead of a fresh `refactorize` every pivot — the same
    // two-trigger scheme (rejected update, or a periodic safety net) the
    // main phase uses.
    let mut lu = lu;
    let mut since_check = 0usize;
    let mut lu_scratch = vec![0.0f64; std.n_rows];
    let mut gp_scratch = sparse_lu::GpScratch::new(std.n_rows);
    let mut alpha_col = vec![0.0f64; std.n_rows];
    // The entering column, densified for `try_update`'s own dense-`rhs`
    // FTRAN. Hoisted out of the loop and refilled by
    // `CscMat::col_into_dense` each pivot, rather than a fresh `Vec` per
    // cleanup pivot.
    let mut dense_j = vec![0.0f64; std.n_rows];
    let debug_ext = std::env::var("ENOMOTO_DEBUG_EXT_ITERS").is_ok();
    let mut cleanup_count = 0usize;
    loop {
        let Some(j) = (0..n_orig).find(|&j| match nb_status[j] {
            Some(NbStatus::Lower) => delta[j].has_lower(),
            Some(NbStatus::Upper) => delta[j].has_upper(),
            None => false,
        }) else {
            break;
        };
        // Hyper-sparse FTRAN (`solve_sparse_into`/`GpScratch`, Gilbert-
        // Peierls): `j`'s own column is genuinely sparse, unlike `x_B`'s
        // own `rhs_base`/`rhs_slope` (summed contributions from every
        // nonbasic column, generally *not* sparse) — this is the one
        // place in this module a hyper-sparse solve has a natural
        // application without also committing to full incremental `x_B`
        // maintenance (see this module's own docs).
        lu.solve_sparse_into(std.cols.col(j), &mut lu_scratch, &mut gp_scratch, &mut alpha_col);
        let Some(r2) = (0..std.n_rows).find(|&i| alpha_col[i].abs() > TOL) else {
            // `B^{-1}A_j` is identically zero: `j` never needs to enter
            // the basis at all (its own module docs, and the paper's own
            // remark after Lemma 4.9) — its value genuinely doesn't
            // matter, so it is parked directly at its true finite side.
            // A genuinely free `j` reaching here (no finite side to park
            // at all) is not something `presolve::freevar` should ever
            // leave standing alone: every residual free variable it can't
            // eliminate outright is documented to appear in at least one
            // row (see this module's own top-of-file docs), so an
            // orphaned one touching zero rows is "should be unreachable"
            // — bail gracefully rather than park it at a side that's
            // still symbolically `M`.
            let Some(side) = (if std.lb[j].is_finite() {
                Some(NbStatus::Lower)
            } else if std.ub[j].is_finite() {
                Some(NbStatus::Upper)
            } else {
                None
            }) else {
                return None;
            };
            nb_status[j] = Some(side);
            continue;
        };
        #[cfg(test)]
        CLEANUP_PIVOTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        cleanup_count += 1;
        let beta_r2 = basis[r2];
        // The variable cleanup evicts to make room for `j` needs a real
        // (non-`M`) nonbasic placement of its own — guaranteed to exist
        // for a one-sided-unbounded column (its own finite side), but not
        // for a genuinely free one (`MSide::Both`, both sides still
        // symbolically `M`): this module's termination argument for the
        // cleanup loop (each pivot strictly shrinks the M-flagged-nonbasic
        // count) is established for the one-sided case, not for evicting
        // *another* free variable — rather than risk an unproven
        // termination argument or an unsound placement, bail to `None`
        // (the classical `BIG_M` fallback) exactly like this module's
        // other "should be unreachable" guards.
        let Some(true_status) = (if std.lb[beta_r2].is_finite() {
            Some(NbStatus::Lower)
        } else if std.ub[beta_r2].is_finite() {
            Some(NbStatus::Upper)
        } else {
            None
        }) else {
            return None;
        };
        nb_status[beta_r2] = Some(true_status);
        basis_pos[beta_r2] = None;
        basis[r2] = j;
        basis_pos[j] = Some(r2);
        nb_status[j] = None;

        std.cols.col_into_dense(j, &mut dense_j);
        since_check += 1;
        let rejected = !lu.try_update(r2, &dense_j, super::FT_MIN_PIVOT);
        if rejected || since_check >= super::FT_CHECK_INTERVAL {
            since_check = 0;
            lu = refactorize(std, basis_pos, Some(&lu))?;
        }
    }

    // The paper's own Step III does not stop at cleanup: "得られた基底...
    // から古典的双対単体法...を再開し、主実行可能な解...に到達するまで反復
    // する" (\S4.8) — restart the classical dual simplex, against the
    // *true* bounds, until primal feasible. This is not optional polish:
    // Lemma 4.9 guarantees a cleanup pivot leaves the objective and every
    // *other* reduced cost unchanged, but says nothing about primal
    // feasibility of rows it didn't directly target — a cleanup pivot can
    // (and empirically does, on real Netlib instances with a genuinely
    // interior-valued `beta(r)`) leave *other* rows outside their true
    // bounds, which only this phase's own deviation-driven iteration
    // discovers and fixes.
    if debug_ext {
        eprintln!("DEBUG_EXT: cleanup_pivots={cleanup_count}");
    }
    polish_with_true_bounds(std, basis, basis_pos, nb_status, lu)
}

/// Step III's own tail: a plain bounded dual simplex operating directly on
/// `std.lb`/`std.ub`, no `M`/`Affine1` involved anymore — but now using the
/// same incremental machinery [`solve_lp_dual_extended`]'s own main phase
/// (and, transitively, `super::solve_lp_dual_on`) already relies on:
/// hyper-sparse chuzr via [`InfeasibleRows`], incremental `x_B` maintenance
/// (seeded once, then updated pivot-to-pivot instead of recomputed from
/// scratch), a Harris-style BFRT pass 2, and `updateVerify` before every
/// pivot commit. This function used to recompute the full right-hand side
/// and rescan every row from scratch every single iteration — the one piece
/// of this module that never received the main phase's own incremental
/// upgrade, and (per this module's own commit history) the dominant cost on
/// larger Netlib instances as a result.
///
/// Deliberately **not** Devex/DSE-weighted, unlike the main phase or the
/// classical method: `finish`'s own `z.slope` check already proves the
/// objective at the basis cleanup hands off is exactly the true final
/// optimum, and cleanup's own Lemma 4.9 leaves it unchanged — so, staying
/// dual feasible throughout, every pivot this phase ever takes is
/// necessarily a zero-objective-contribution (degenerate) one. Devex/DSE
/// exist to steer toward whichever pivot advances the objective the most
/// per step; with no such "more attractive" pivot to steer toward here
/// (every pivot is equally degenerate), the weighting would only add cost
/// (weight maintenance, an extra FTRAN for DSE's own `tau`) for no
/// convergence benefit. Anti-cycling is `perturb_costs` plus the Bland
/// fallback below alone — already confirmed sufficient here, not merely
/// assumed: this function's own history names `cycle`/`degen2`/`degen3`
/// directly as instances that reported a false `Infeasible` without it, and
/// that fix long predates this incremental rewrite (weighting was never
/// part of it).
///
/// A column that still carries a genuine one-sided infinity here (any
/// nonbasic that never needed cleanup, e.g. a `<=` row's own slack) is
/// handled exactly like the classical method already does: its width is
/// simply not finite, so it is never a flip candidate, only ever a direct
/// pivot target.
fn polish_with_true_bounds(std: &StdForm, basis: &mut [usize], basis_pos: &mut [Option<usize>], nb_status: &mut [Option<NbStatus>], lu: sparse_lu::FtLu) -> Option<SimplexResult> {
    let n_total = std.n_total;
    let m = std.n_rows;
    let stall_limit = (5 * m).max(500);
    let mut stall_count = 0usize;
    let mut bland_mode = false;
    let active_cost = super::perturb_costs(std);

    let mut lu = lu;
    let mut since_check = 0usize;
    let mut since_residual_check = 0usize;
    let mut lu_scratch = vec![0.0f64; m];
    // `PROF_PHASES_EXT` diagnostic (this function's own counterpart to the
    // main loop's `prof_phases` module, see [`synth_clock_should_refactor`]'s
    // own docs for why this trigger is shared code): unlike the main loop,
    // this phase has no other profiling instrumentation, so this trigger's
    // own fire count is tracked in a plain local rather than threading a
    // new module-level atomic through a function that otherwise has none.
    let profile_phases_polish = std::env::var("ENOMOTO_PROF_PHASES_EXT").is_ok();
    let mut polish_clock_refactors = 0usize;
    let mut polish_refactors = 0usize;

    // Unlike the main phase (which always starts from the trivial
    // all-slack basis, `y = 0` for free), this phase inherits whatever
    // basis cleanup left — so `d` needs one genuine BTRAN against that
    // basis's own `c_B` to become valid *before* the incremental
    // update-dual loop below can take over maintaining it.
    let mut d = vec![0.0f64; n_total];
    {
        let c_b: Vec<f64> = basis.iter().map(|&bv| active_cost[bv]).collect();
        let mut y = vec![0.0f64; m];
        lu.solve_transpose_into(&c_b, &mut lu_scratch, &mut y);
        for j in 0..n_total {
            let mut dj = active_cost[j];
            for &(i, v) in std.cols.col(j) {
                dj -= v * y[i];
            }
            d[j] = dj;
        }
    }

    // Buffer reuse, row-major PRICE, incremental `d` — same techniques as
    // the main phase (see its own docs), now over plain `f64` throughout
    // (no `M`/`Affine1` involved in this phase at all).
    let mut a_p = vec![0.0f64; n_total];
    let mut touched = vec![false; n_total];
    let mut touched_cols: Vec<usize> = Vec::new();
    let mut x_b = vec![0.0f64; m];
    let mut rho = vec![0.0f64; m];
    let mut dense_q = vec![0.0f64; m];
    let mut alpha_full = vec![0.0f64; m];
    let mut candidates: Vec<Cand> = Vec::new();
    // `try_update_precomputed` capture buffers — see the main phase's own
    // identically-purposed pair's docs.
    let mut a_tilde_buf = vec![0.0f64; m];
    let mut e_tilde_buf = vec![0.0f64; m];

    // Dedicated to `solve_sparse_into` alone (see the main phase's own
    // identically-purposed buffers' docs — never shared with `lu_scratch`
    // above).
    let mut sparse_scratch = vec![0.0f64; m];
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

    let mut combined = vec![0.0f64; m];
    let mut combined_touched_flag = vec![false; m];
    let mut combined_touched: Vec<usize> = Vec::new();
    let mut combined_alpha = vec![0.0f64; m];
    let mut sparse_buf: Vec<(usize, f64)> = Vec::with_capacity(m);

    let mut noise_feasible = vec![false; n_total];
    let update_verify_disabled = std::env::var("ENOMOTO_DISABLE_UPDATE_VERIFY").is_ok();

    // `x_B`'s own one-time seed ([`compute_rhs_plain`]'s own docs) — every
    // iteration from here on maintains it incrementally instead of paying
    // this same `O(nnz(A))` sum from scratch every pivot (this function's
    // own docs above).
    lu.solve_into(&compute_rhs_plain(std, nb_status), &mut lu_scratch, &mut x_b);
    let mut infeasible_rows = InfeasibleRows::new(m);
    infeasible_rows.rebuild(m, |i| row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));

    let max_iters = super::max_iters_for(m, n_total);
    for _iter in 0..max_iters {
        // chuzr: plain largest-deviation Dantzig rule, exactly as this
        // function always used — *not* Devex/DSE-weighted. **Exact DSE
        // was tried here** (`DseState::from_basis` at entry, `mag^2/w[i]`
        // scoring, `update_after_pivot` after every pivot, mirroring the
        // main phase's own scheme) on the theory that DSE's real
        // justification — steering `chuzr` toward the pivot that reduces
        // *remaining infeasibility* fastest — is a different claim than
        // "every pivot here has zero objective contribution" and so isn't
        // actually addressed by this function's own docs above. **Measured
        // as a net regression on the full 73-problem Netlib set** (8.53s
        // baseline -> 9.00s, +5.5%; `stocfor2` alone 0.25s -> 0.36s, +46%)
        // and reverted — the extra `from_basis` O(m) BTRAN plus one more
        // FTRAN (`tau`) every pivot costs more than the weighting ever
        // saves in pivot count on this phase's already-small, already-
        // degenerate iteration counts. Scanned over `infeasible_rows.rows`
        // only (hyper-sparse — [`InfeasibleRows`]'s own docs), not `0..m`.
        let mut best: Option<(usize, i32, f64)> = None;
        for &i in &infeasible_rows.rows {
            let Some((d_dir, mag)) = row_deviation_plain(std, basis, &x_b, &noise_feasible, i) else { continue };
            let better = match best {
                None => true,
                Some((br, _, bmag)) => {
                    if bland_mode {
                        i < br
                    } else if mag > bmag {
                        true
                    } else if mag < bmag {
                        false
                    } else {
                        i < br
                    }
                }
            };
            if better {
                best = Some((i, d_dir, mag));
            }
        }
        #[cfg(debug_assertions)]
        {
            // [`InfeasibleRows`]'s own exactness claim (its docs): every
            // row it lists infeasible, and no others, matches a fresh full
            // scan — mirrors the main phase's own identical cross-check.
            let mut fresh: Vec<usize> = (0..m).filter(|&i| row_infeasible_plain(std, basis, &x_b, &noise_feasible, i)).collect();
            let mut maintained: Vec<usize> = infeasible_rows.rows.clone();
            fresh.sort_unstable();
            maintained.sort_unstable();
            debug_assert_eq!(fresh, maintained, "InfeasibleRows drifted from a fresh scan at polish iter {_iter}");
        }

        let Some((r, d_dir, needed)) = best else {
            let mut x = vec![0.0; n_total];
            for j in 0..n_total {
                x[j] = match basis_pos[j] {
                    Some(pos) => x_b[pos],
                    None => match nb_status[j].unwrap() {
                        NbStatus::Lower => std.lb[j],
                        NbStatus::Upper => std.ub[j],
                    },
                };
            }
            if std::env::var("ENOMOTO_DEBUG_EXT_ITERS").is_ok() {
                let obj: f64 = (0..n_total).map(|j| std.c[j] * x[j]).sum();
                eprintln!("DEBUG_EXT: polish_iters={_iter} bland_mode={bland_mode} obj={obj}");
            }
            // Primal feasible against this phase's own *perturbed* costs
            // (`active_cost`, set at this function's entry) -- dual
            // feasibility held throughout by the loop invariant above, so
            // this point is optimal for the perturbed problem. Whether it
            // is *also* optimal for the true problem depends on whether
            // perturbation happened to mask a genuine dual infeasibility:
            // ported from `super::solve_lp_dual_on`'s own identical check
            // (that function's own docs) -- this phase never had it, and
            // the gap is not hypothetical: measured directly on Netlib
            // `greenbea`, the unperturbed check below fails and this
            // phase's own perturbed-optimal point is off by 92808 in
            // objective (0.13%) from the true optimum
            // (`analysis/greenbea_20260921_030127.md`, mechanism (B)).
            let mut true_d = vec![0.0f64; n_total];
            {
                let c_b: Vec<f64> = basis.iter().map(|&bv| std.c[bv]).collect();
                let mut y = vec![0.0f64; m];
                lu.solve_transpose_into(&c_b, &mut lu_scratch, &mut y);
                for j in 0..n_total {
                    let mut dj = std.c[j];
                    for &(i, v) in std.cols.col(j) {
                        dj -= v * y[i];
                    }
                    true_d[j] = dj;
                }
            }
            let true_dual_feasible = (0..n_total).all(|j| match nb_status[j] {
                None => true,
                Some(NbStatus::Lower) => true_d[j] >= -TOL,
                Some(NbStatus::Upper) => true_d[j] <= TOL,
            });
            if true_dual_feasible {
                return Some(SimplexResult { status: Status::Optimal, x: Some(x) });
            }
            // Perturbation masked a genuine dual infeasibility: this basis
            // is primal feasible (feasibility never depended on costs) but
            // not dual feasible for the true costs. Finishing from here
            // needs the primal method's own invariant (primal feasibility
            // preserved, working toward dual feasibility) instead of this
            // phase's dual one -- exactly `super::run_phase`'s phase 2,
            // reused directly rather than reimplemented: every nonbasic
            // column here already sits at a *finite* side (the cleanup
            // lemma's own guarantee, `finish`'s docs above), the same
            // situation every `<=`-row slack is already in in the
            // classical path this was written for, so it needs no special
            // handling for this module's own genuinely-infinite bounds.
            // `expand`/`se` start fresh rather than mid-sequence
            // (unrelated quantities to this phase's own dual state;
            // `solve_lp_dual_on`'s own identical handoff confirms this only
            // costs pricing quality, not correctness).
            let mut t = super::Tableau { std, basis: basis.to_vec(), basis_pos: basis_pos.to_vec(), nb_status: nb_status.to_vec(), x };
            let mut expand = super::ExpandState::new();
            let mut se = super::SteepestEdgeState::new(std);
            let mut stall = super::PrimalStallState::new();
            if std::env::var("ENOMOTO_DEBUG_EXT_ITERS").is_ok() {
                eprintln!("DEBUG_EXT: polish DUAL->PRIMAL cleanup handoff at polish_iter={_iter}");
            }
            // Unlike `solve_lp_dual_on`'s identical handoff, a singular
            // basis here does *not* fall back to a from-scratch classical
            // solve: `solve_lp_on`/`Tableau::new` assume every structural
            // column has a finite bound (their own docs), which this
            // module exists specifically to handle when false -- so `None`
            // is propagated to this function's own caller instead, which
            // already knows how to fall back (the existing `BIG_M`-clamped
            // classical path in `solve_lp_dual`) without that assumption.
            let handoff_t0 = profile_phases_polish.then(std::time::Instant::now);
            let status = super::run_phase(std, &mut t, false, &mut lu, &mut since_check, &mut expand, &mut se, &mut stall);
            if let Some(t0) = handoff_t0 {
                eprintln!("PROF_HANDOFF run_phase={:.3}ms ok={}", t0.elapsed().as_secs_f64() * 1e3, status.is_some());
            }
            let status = status?;
            return Some(SimplexResult {
                status: status.clone(),
                x: if status == Status::Optimal { Some(t.x[0..t.n_orig()].to_vec()) } else { None },
            });
        };

        // Captures `e_tilde_buf` for this iteration's own
        // `try_update_precomputed` call further down — see the main
        // phase's own identical BTRAN capture docs.
        lu.solve_transpose_unit_capture(r, &mut lu_scratch, &mut rho, &mut e_tilde_buf);

        // Row-major sparse PRICE (see the main phase's own docs for the
        // full derivation, including why *basic* columns are deliberately
        // not filtered out here) — `a_p = rho^T A`, walking only `rho`'s
        // nonzero rows.
        for i in 0..m {
            let rv = rho[i];
            if rv.abs() <= TOL {
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
                a_p[j] += rv * v;
            }
        }

        candidates.clear();
        for &j in &touched_cols {
            let Some(status) = nb_status[j] else { continue };
            let alpha_j = a_p[j];
            if alpha_j.abs() <= TOL {
                continue;
            }
            let sigma = match status {
                NbStatus::Lower => 1.0,
                NbStatus::Upper => -1.0,
            };
            let hat_alpha = sigma * alpha_j;
            if (d_dir as f64) * hat_alpha >= 0.0 {
                continue;
            }
            let hat_c = (sigma * d[j]).max(0.0);
            candidates.push(Cand { j, hat_alpha, ratio: hat_c / hat_alpha.abs() });
        }
        if candidates.is_empty() {
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            // `super::solve_lp_dual_on`'s own `noise_feasible` second
            // chance, ported here now that this function no longer just
            // re-derives a fresh (and possibly slightly different) `x_B`
            // next iteration regardless: before concluding genuine
            // infeasibility, check whether `r`'s own deviation is actually
            // within this row's rounding noise (scaled by its own RHS
            // magnitude — `PRIMAL_FEAS_TOL`'s own docs).
            if needed <= super::PRIMAL_FEAS_TOL * std.b[r].abs().max(1.0) {
                noise_feasible[basis[r]] = true;
                infeasible_rows.set(r, false);
                continue;
            }
            return Some(SimplexResult { status: Status::Infeasible, x: None });
        }
        // Same order regardless of `bland_mode`: ascending `(ratio, j)`.
        // `bland_mode` sorting by `j` alone (an earlier version of this
        // branch) broke the ratio test the BFRT walk just below relies on
        // -- see the main M-tracked loop's own identical fix above for the
        // Netlib `greenbea` failure this caused there. `j` already breaks
        // ties deterministically in the one order, which is all Bland's
        // rule actually needs.
        candidates.sort_by(|a, b| a.ratio.total_cmp(&b.ratio).then_with(|| a.j.cmp(&b.j)));

        // Relative tolerance, not `TOL` flat: `needed` (from an LU solve)
        // and `cum` (a sum of per-candidate capacities) reach the same
        // mathematical value via unrelated rounding paths — the exact
        // same class of noise `Affine1::cmp_lex`'s own docs describe for
        // the main phase, just for plain `f64` here.
        let reach_tol = TOL.max(1e-9 * needed.abs());
        let mut cum = 0.0f64;
        let mut k_star: Option<usize> = None;
        for (idx, cand) in candidates.iter().enumerate() {
            let width = width_plain(std, cand.j);
            if !width.is_finite() {
                k_star = Some(idx);
                break;
            }
            let new_cum = cum + width * cand.hat_alpha.abs();
            if new_cum >= needed - reach_tol {
                k_star = Some(idx);
                break;
            }
            cum = new_cum;
        }
        let Some(k_star) = k_star else {
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            return Some(SimplexResult { status: Status::Infeasible, x: None });
        };

        // Harris-style pass 2 (ported from the main phase — see that
        // block's own docs for why this refinement matters on Netlib
        // `cycle`): search backward from `k_star` within a flat
        // ratio-space window for the candidate with the largest pivot
        // magnitude, and pivot on that one instead.
        let mut best_idx = k_star;
        {
            let min_ratio = candidates[k_star].ratio - super::HARRIS_RATIO_TOL;
            let mut window_start = k_star;
            while window_start > 0 && candidates[window_start - 1].ratio >= min_ratio {
                window_start -= 1;
            }
            let mut best_abs = candidates[k_star].hat_alpha.abs();
            for (idx, cand) in candidates.iter().enumerate().take(k_star + 1).skip(window_start) {
                let abs_a = cand.hat_alpha.abs();
                if abs_a > best_abs {
                    best_abs = abs_a;
                    best_idx = idx;
                }
            }
        }

        // BFRT combined-flip (main phase's own docs): every candidate about
        // to be flipped contributes its own bound jump to one summed sparse
        // right-hand side, solved once, rather than one FTRAN per flipped
        // column — necessary now that `x_B` is maintained incrementally
        // instead of recomputed fresh every iteration. Reads
        // `nb_status[cand.j]` while it's still pre-flip, so this must run
        // before the flip-commit loop just below.
        for cand in &candidates[..best_idx] {
            let old = nb_status[cand.j].unwrap();
            let width = width_plain(std, cand.j);
            let sigma = if old == NbStatus::Lower { 1.0 } else { -1.0 };
            let delta_x = width * sigma;
            for &(i, v) in std.cols.col(cand.j) {
                if !combined_touched_flag[i] {
                    combined_touched_flag[i] = true;
                    combined_touched.push(i);
                }
                combined[i] += v * delta_x;
            }
        }
        if !combined_touched.is_empty() {
            if lu.should_use_dense_solve_tracked(combined_touched.len(), &density_bfrt) {
                let result_nnz = lu.solve_into(&combined, &mut lu_scratch, &mut combined_alpha);
                density_bfrt.record(result_nnz, m);
            } else {
                sparse_buf.clear();
                sparse_buf.extend(combined_touched.iter().map(|&i| (i, combined[i])));
                let result_nnz = lu.solve_sparse_into(&sparse_buf, &mut sparse_scratch, &mut gp_scratch, &mut combined_alpha);
                density_bfrt.record(result_nnz, m);
            }
            // Scanned over `0..m`, not `combined_touched`: FTRAN fill-in
            // can produce nonzeros outside the input's own sparsity
            // pattern (main phase's own docs).
            for i in 0..m {
                if combined_alpha[i] != 0.0 {
                    x_b[i] -= combined_alpha[i];
                    infeasible_rows.set(i, row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));
                }
            }
            for &i in &combined_touched {
                combined[i] = 0.0;
                combined_touched_flag[i] = false;
            }
            combined_touched.clear();
        }
        for cand in &candidates[..best_idx] {
            let old = nb_status[cand.j].unwrap();
            nb_status[cand.j] = Some(match old {
                NbStatus::Lower => NbStatus::Upper,
                NbStatus::Upper => NbStatus::Lower,
            });
        }

        let q = candidates[best_idx].j;
        let dj_q = d[q];
        let alpha_q = a_p[q];

        // `alpha_full = B^{-1}A_q` — needed by this phase's own incremental
        // `x_B` step below — computed once here, matching the main phase's
        // own convention. Same dense-or-sparse fork as the main phase's
        // own entering-column FTRAN (its own docs); both branches also
        // capture `a_tilde_buf` for this iteration's own
        // `try_update_precomputed` call further down.
        dense_q.fill(0.0);
        for &(i, v) in std.cols.col(q) {
            dense_q[i] = v;
        }
        if lu.should_use_dense_solve_tracked(std.cols.col(q).len(), &density_col_aq) {
            let result_nnz = lu.solve_into_capture(&dense_q, &mut lu_scratch, &mut alpha_full, &mut a_tilde_buf);
            density_col_aq.record(result_nnz, m);
        } else {
            let result_nnz = lu.solve_sparse_into_capture(std.cols.col(q), &mut sparse_scratch, &mut gp_scratch, &mut alpha_full, &mut a_tilde_buf);
            density_col_aq.record(result_nnz, m);
        }

        // `updateVerify` (`super::update_verify`'s own docs): cross-checks
        // this pivot's element between PRICE's row-direction value
        // (`alpha_q`) and FTRAN's own column-direction value
        // (`alpha_full[r]`) before anything derived from `alpha_full` is
        // committed — same placement as the main phase's own.
        if !update_verify_disabled && lu.update_count() > 0 && !super::update_verify(alpha_q, alpha_full[r]) {
            lu = refactorize(std, basis_pos, Some(&lu))?;
            lu.solve_into(&compute_rhs_plain(std, nb_status), &mut lu_scratch, &mut x_b);
            infeasible_rows.rebuild(m, |i| row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));
            for &j in &touched_cols {
                a_p[j] = 0.0;
                touched[j] = false;
            }
            touched_cols.clear();
            continue;
        }

        // Entering column's own incremental `x_B` step — the other half of
        // the main phase's own combined-flip/`alpha`-scaled primal-step
        // pair (see its own docs), now over a single `f64` channel.
        let old_status_q = nb_status[q].unwrap();
        let nb_val_q = match old_status_q {
            NbStatus::Lower => std.lb[q],
            NbStatus::Upper => std.ub[q],
        };
        let target = if d_dir > 0 { std.lb[basis[r]] } else { std.ub[basis[r]] };
        let theta = (x_b[r] - target) / alpha_q;
        for i in 0..m {
            let a = alpha_full[i];
            if a != 0.0 {
                x_b[i] -= a * theta;
                infeasible_rows.set(i, row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));
            }
        }
        x_b[r] = nb_val_q + theta;

        // Stall detection — now the classical method's own *exact* check
        // (`(theta_q * dj_q).abs() < STALL_PROGRESS_EPS`), not the
        // `dj_q`-alone proxy the previous, non-incremental version of this
        // function needed for lack of a real `theta_q`: the incremental
        // `x_B` step just above now computes a real one.
        let contribution = theta * dj_q;
        if contribution.abs() < super::STALL_PROGRESS_EPS {
            stall_count += 1;
            if stall_count > stall_limit {
                bland_mode = true;
            }
        } else {
            stall_count = 0;
        }

        let leaving_var = basis[r];
        nb_status[leaving_var] = Some(if d_dir > 0 { NbStatus::Lower } else { NbStatus::Upper });
        basis_pos[leaving_var] = None;
        basis[r] = q;
        basis_pos[q] = Some(r);
        nb_status[q] = None;
        // Row `r`'s occupant identity just changed — re-check it against
        // the new occupant now that `basis[r]` holds it (main phase's own
        // identical post-swap call).
        infeasible_rows.set(r, row_infeasible_plain(std, basis, &x_b, &noise_feasible, r));

        // Incremental dual update — same formula/reasoning as the main
        // phase's own (see its own docs); `d` here is a plain `f64` array
        // throughout this phase, never `M`-dependent.
        let theta_d = dj_q / alpha_q;
        for &j in &touched_cols {
            d[j] -= theta_d * a_p[j];
        }
        for &j in &touched_cols {
            a_p[j] = 0.0;
            touched[j] = false;
        }
        touched_cols.clear();

        // Forrest-Tomlin incremental update — same trigger scheme as the
        // main phase's own (see its own docs), now via the same
        // `try_update_precomputed` capture reuse (`a_tilde_buf`/
        // `e_tilde_buf`, filled above by this iteration's own FTRAN/BTRAN).
        since_check += 1;
        let mut need_refactor = !lu.try_update_precomputed(r, &a_tilde_buf, &e_tilde_buf, super::FT_MIN_PIVOT);
        // Trigger (4) ([`ft_max_updates`]'s own docs) — unconditional every
        // iteration, same as the main phase's own identical check.
        if !need_refactor && lu.update_count() > ft_max_updates(m) {
            need_refactor = true;
        }
        // Trigger (5) — same shared check as the main phase's own (see
        // [`synth_clock_should_refactor`]'s own docs).
        if !need_refactor && synth_clock_should_refactor(&lu) {
            need_refactor = true;
            if profile_phases_polish {
                polish_clock_refactors += 1;
            }
        }
        if !need_refactor && since_check >= super::FT_CHECK_INTERVAL {
            since_check = 0;
            let bump_too_big = lu.fill_count() > super::FT_BUMP_LIMIT_FACTOR * m.max(1);
            since_residual_check += 1;
            if bump_too_big {
                need_refactor = true;
            } else if since_residual_check >= super::RESIDUAL_CHECK_MULTIPLIER {
                since_residual_check = 0;
                let fresh_rhs = compute_rhs_plain(std, nb_status);
                need_refactor = residual_norm(std, basis_pos, &x_b, &fresh_rhs) > super::FT_RESIDUAL_TOL;
            }
        }
        if need_refactor {
            if profile_phases_polish {
                polish_refactors += 1;
            }
            lu = refactorize(std, basis_pos, Some(&lu))?;
            lu.solve_into(&compute_rhs_plain(std, nb_status), &mut lu_scratch, &mut x_b);
            infeasible_rows.rebuild(m, |i| row_infeasible_plain(std, basis, &x_b, &noise_feasible, i));
        }
    }
    if profile_phases_polish {
        eprintln!("PROF_PHASES_EXT_POLISH refactor_count={polish_refactors} (clock={polish_clock_refactors})");
    }

    None
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::{CscMat, CsrMat};
    use std::sync::atomic::Ordering::Relaxed;

    /// Builds a `StdForm` directly from a dense list of sparse rows (each
    /// row *already* including its own slack term), bypassing
    /// presolve/`build_std_form_presolved` entirely — these tests exercise
    /// [`solve_lp_dual_extended`] in isolation, independent of whatever
    /// presolve does or doesn't eliminate for a given problem shape (see
    /// `simplex.rs`'s own end-to-end `freevar_*`/`had_unbounded_structural_*`
    /// tests for the presolve-integrated path).
    fn std_form(rows: &[Vec<(usize, f64)>], b: Vec<f64>, c: Vec<f64>, lb: Vec<f64>, ub: Vec<f64>) -> StdForm {
        let n_total = lb.len();
        let n_rows = rows.len();
        assert_eq!(c.len(), n_total);
        assert_eq!(ub.len(), n_total);
        let cols = CscMat::from_rows(rows, n_total);
        StdForm { n_total, n_rows, c, rows: CsrMat::from_rows(rows, n_total), cols, b, lb, ub }
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn genuinely_free_column_reaches_a_finite_optimum_via_one_equality_row() {
        // x0 free (both bounds infinite — `simplex.rs` no longer splits
        // this into x0+ - x0- before handing a `StdForm` to this module),
        // x1 in [0,10]; x0 + x1 + s = 5, s in [0,0]. min -x0 (== max x0):
        // x0 = 5 - x1 in [-5, 5], optimal x0 = 5 (x1 = 0). Exercises
        // `hat_lower`/`hat_upper` both being `Some` (M-tracked) for the
        // same column at once.
        let std = std_form(&[vec![(0, 1.0), (1, 1.0), (2, 1.0)]], vec![5.0], vec![-1.0, 0.0, 0.0], vec![f64::NEG_INFINITY, 0.0, 0.0], vec![f64::INFINITY, 10.0, 0.0]);
        let res = solve_lp_dual_extended(&std).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 5.0), "x={x:?}");
        assert!(approx(x[1], 0.0), "x={x:?}");
    }

    #[test]
    fn two_free_columns_tied_only_through_opposing_inequality_rows_bails_out_gracefully() {
        // The exact shape this module's own top-of-file docs name as
        // `presolve::freevar`'s undecidable residual case: two free
        // columns (x0, x1) that appear only in inequality rows, never an
        // equality one, so nothing upstream can eliminate or bound either
        // in isolation. x0 - x1 + s0 = 3 (s0 >= 0, i.e. x0 - x1 <= 3) and
        // -x0 + x1 + s1 = 3 (s1 >= 0, i.e. x0 - x1 >= -3): min x0 - x1
        // drives the difference to its lower bound, -3, but x0 and x1
        // individually stay genuinely unbounded (only their *difference*
        // is pinned) — the true optimal face is a whole line, not a single
        // point, so *some* variable must end up nonbasic with no real bound
        // to rest at. Traced by hand (and confirmed via temporary tracing
        // during development): the main phase resolves one of the two
        // directly, leaving the other still nonbasic at its own `M` side;
        // cleanup's own `B^{-1}A_j` for that survivor is nonzero in exactly
        // one row, and the variable *currently basic there* is the other
        // free column — evicting it would need to pin it at an arbitrary
        // real anchor this module has no representation for (`finish`'s own
        // docs on why this specific case bails rather than guesses). This
        // is not a rare corner this test is inventing: it is the *forced*
        // outcome whenever two structural columns are free and coupled only
        // to each other, with no third column anchoring either — so this
        // asserts the graceful-`None` contract itself (falls back to the
        // classical `BIG_M` path one level up, `simplex.rs`'s own
        // `mutually_coupled_free_variables_falls_back_to_a_correct_answer`
        // end-to-end test), not a wrong answer or a panic.
        let rows = vec![vec![(0, 1.0), (1, -1.0), (2, 1.0)], vec![(0, -1.0), (1, 1.0), (3, 1.0)]];
        let std = std_form(&rows, vec![3.0, 3.0], vec![1.0, -1.0, 0.0, 0.0], vec![f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0, 0.0], vec![f64::INFINITY, f64::INFINITY, f64::INFINITY, f64::INFINITY]);
        assert!(solve_lp_dual_extended(&std).is_none());
    }

    #[test]
    fn direct_pivot_resolves_unbounded_column_into_the_basis() {
        // x0 in [0, +inf), x1 in [0,10]; x0 + x1 + s = 5, s in [0,0].
        // min -x0 (== max x0): x0 <= 5 - x1 <= 5, optimal x0=5, x1=0.
        // Traced by hand: x0 is chosen as `q` directly in the first
        // iteration (its own row's deviation has no other candidate that
        // can absorb an M-order gap), landing with slope exactly 0 —
        // cleanup never fires here (`CLEANUP_PIVOTS` stays untouched).
        let std = std_form(&[vec![(0, 1.0), (1, 1.0), (2, 1.0)]], vec![5.0], vec![-1.0, 0.0, 0.0], vec![0.0, 0.0, 0.0], vec![f64::INFINITY, 10.0, 0.0]);
        let before = CLEANUP_PIVOTS.load(Relaxed);
        let res = solve_lp_dual_extended(&std).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 5.0), "x={x:?}");
        assert!(approx(x[1], 0.0), "x={x:?}");
        assert_eq!(CLEANUP_PIVOTS.load(Relaxed), before, "this example shouldn't need any cleanup pivot");
    }

    #[test]
    fn m_zero_bounded_direction_is_optimal_at_its_finite_bound() {
        // No rows at all; x0 in [0, +inf), cost favors the finite (lower)
        // side directly — the `m == 0` shortcut's own "bounded" branch.
        let std = std_form(&[], vec![], vec![1.0], vec![0.0], vec![f64::INFINITY]);
        let res = solve_lp_dual_extended(&std).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Optimal);
        assert!(approx(res.x.unwrap()[0], 0.0));
    }

    #[test]
    fn m_zero_unbounded_direction_reports_unbounded() {
        // No rows at all; x0 in [0, +inf), cost favors the infinite side —
        // the `m == 0` shortcut's own "unbounded" branch.
        let std = std_form(&[], vec![], vec![-1.0], vec![0.0], vec![f64::INFINITY]);
        let res = solve_lp_dual_extended(&std).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Unbounded);
    }

    #[test]
    fn infeasible_row_is_reported_regardless_of_an_unbounded_column() {
        // x0 in [0,+inf), x1 in [0,5]; x0 + x1 + s = -1, s in [0,0] —
        // infeasible regardless of x0's own bound shape, since x0,x1 >= 0
        // can never sum to a negative right-hand side.
        let std = std_form(&[vec![(0, 1.0), (1, 1.0), (2, 1.0)]], vec![-1.0], vec![0.0, 0.0, 0.0], vec![0.0, 0.0, 0.0], vec![f64::INFINITY, 5.0, 0.0]);
        let res = solve_lp_dual_extended(&std).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Infeasible);
    }

    #[test]
    fn orphaned_unbounded_column_with_zero_cost_is_cleaned_up_without_a_pivot() {
        // x0 in (-inf, 0] (shifted so its finite side, ub, sits at 0),
        // appearing in *no* row at all, zero cost: crash places it at
        // Lower (its own artificial `-M` side, since `c[0] >= -TOL`
        // unconditionally when `c[0] == 0.0`) — main loop never touches
        // it (no row ever mentions it), so it survives to cleanup's own
        // "B^{-1}A_j is identically zero" branch (this module's own
        // docs): parked directly at its true finite side (`ub = 0`)
        // without ever needing an actual pivot swap.
        // x1 in [0,10] with its own trivial forcing row keeps `m >= 1` so
        // this exercises the main-loop-then-cleanup path, not the
        // `m == 0` shortcut.
        let std = std_form(&[vec![(1, 1.0), (2, 1.0)]], vec![3.0], vec![0.0, 0.0, 0.0], vec![f64::NEG_INFINITY, 0.0, 0.0], vec![0.0, 10.0, 0.0]);
        let before = CLEANUP_PIVOTS.load(Relaxed);
        let res = solve_lp_dual_extended(&std).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 0.0), "x={x:?}");
        assert!(approx(x[1], 3.0), "x={x:?}");
        assert_eq!(CLEANUP_PIVOTS.load(Relaxed), before, "the identically-zero branch never performs an actual pivot swap");
    }

    #[test]
    fn bfrt_flips_multiple_bounded_candidates_before_the_real_unbounded_pivot() {
        // x1 in [0,1] cost 1, x2 in [0,1] cost 2, x0 in [0,+inf) cost 100
        // (the real eventual entering column, deliberately the most
        // expensive so it sorts *last*) — x1 + x2 + x0 + s = 10, s in
        // [0,0]. All three start at their own lower bound (every cost is
        // non-negative, so `crash` places them all there — a uniform
        // starting side unlike `direct_pivot_resolves_unbounded_column_
        // into_the_basis`'s own single-candidate example, where `x0`'s
        // negative cost placed it at `Upper` instead and so never shared
        // an eligible candidate list with any bounded column at all).
        // Regression test for the combined-flip incremental `x_B(M)`
        // update (`COMBINED_FLIP_COUNT`, this module's own docs on why
        // none of the other hand-built tests in this file are large
        // enough to exercise it at all): x1 and x2's own finite widths (1
        // each) are far short of the row's own deviation (10), so both
        // get fully flipped to their own upper bound (the BFRT walk's
        // ascending-ratio order: cheapest cost first) before it ever
        // reaches `x0` as the real pivot. To *minimize* a positive-cost
        // equality-constrained sum, the cheapest variables should indeed
        // be maxed out first: x1=1, x2=1, x0 absorbs the rest (10-1-1=8).
        let std = std_form(
            &[vec![(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0)]],
            vec![10.0],
            vec![100.0, 1.0, 2.0, 0.0],
            vec![0.0, 0.0, 0.0, 0.0],
            vec![f64::INFINITY, 1.0, 1.0, 0.0],
        );
        let flips_before = COMBINED_FLIP_COUNT.load(Relaxed);
        let res = solve_lp_dual_extended(&std).expect("should reach a mathematical answer, not the None fallback sentinel");
        assert_eq!(res.status, Status::Optimal);
        let x = res.x.unwrap();
        assert!(approx(x[0], 8.0), "x={x:?}");
        assert!(approx(x[1], 1.0), "x={x:?}");
        assert!(approx(x[2], 1.0), "x={x:?}");
        assert!(
            COMBINED_FLIP_COUNT.load(Relaxed) - flips_before >= 2,
            "expected the BFRT walk to flip both x1 and x2 before reaching x0"
        );
    }
}
