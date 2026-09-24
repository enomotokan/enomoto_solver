//! ソルバー全体の閾値・許容誤差・反復上限などのパラメータを一か所にまとめたもの。
//!
//! 各モジュールは `use crate::params::<領域>::{...}` で必要な定数を取り込む。
//! 一部の値は `tunable!("ENOMOTO_T_...", 定数, 型)` 経由で環境変数から上書きでき、
//! ここの値はその既定値になる (再ビルドなしで A/B 比較するため)。
//! 値の決め方・試して却下した値などの経緯は `docs/improvement_history.md` を参照。

/// 単体法共通 (src/simplex.rs)
pub(crate) mod simplex {
    pub(crate) const TOL: f64 = 1e-9;

    /// Floor for [`max_iters_for`]'s size-scaled cap — the crate's own
    /// historical fixed value, kept as a lower bound so every small/medium
    /// instance that already solved within it (the whole Netlib 73-problem
    /// `--max-vars 3000` set, per `netlib_benchmark_workflow`) sees no
    /// behavior change at all.
    pub(crate) const MAX_ITERS_FLOOR: usize = 20_000;

    /// Absolute ceiling on [`max_iters_for`]'s output — a defensive backstop,
    /// not a value any known Netlib instance approaches (the largest, `dfl001`
    /// at `m=4554`/`n_total=9773` post-presolve, needs `MAX_ITERS_SCALE *
    /// (m+n_total) = 286,540`, far below this), so a pathological future
    /// instance can't turn an unbounded-looking cycle into a multi-hour hang.
    pub(crate) const MAX_ITERS_CEILING: usize = 2_000_000;

    /// Multiplier applied to `m + n_total` by [`max_iters_for`].
    pub(crate) const MAX_ITERS_SCALE: usize = 20;

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
    pub(crate) const PRIMAL_FEAS_TOL: f64 = 1e-7;

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
    pub(crate) const HARRIS_RATIO_TOL: f64 = 1e-7;

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
    pub(crate) const PRIMAL_HARRIS_TOL: f64 = 1e-7;

    /// Threshold below which a pivot's actual contribution to the objective
    /// (`theta_q * dj_q`) counts as "no real progress" for `bland_mode`'s
    /// stall counter — see that flag's own docs.
    pub(crate) const STALL_PROGRESS_EPS: f64 = 1e-9;

    /// Trigger (2): an FT update whose resulting pivot is smaller than this
    /// is rejected by `FtLu::try_update`, forcing an immediate refactorization.
    pub(crate) const FT_MIN_PIVOT: f64 = 1e-7;

    /// Cadence (in iterations) for triggers (1) and (3).
    pub(crate) const FT_CHECK_INTERVAL: usize = 5;

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
    pub(crate) const RESIDUAL_CHECK_MULTIPLIER: usize = 20;

    /// Trigger (1): refactor if the true-basis residual exceeds this. In
    /// practice this essentially never fires (measured residuals on this
    /// crate's target problem sizes stayed around 1e-11..1e-12, several
    /// orders of magnitude below even the old 1e-6) — trigger (3) below is
    /// what actually governs refactorization frequency — but it costs nothing
    /// to leave a wide safety margin here too.
    pub(crate) const FT_RESIDUAL_TOL: f64 = 1e-4;

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
    pub(crate) const FT_BUMP_LIMIT_FACTOR: usize = 64;

    /// Trigger (4): refactor unconditionally once the update count passes
    /// this. Measured to be the very first refactorization in a solve (fired
    /// once, right around 100, before trigger (3) ever got a chance to) —
    /// raising it lets a solve run further into trigger (3)'s own eta-fill
    /// budget before this unconditional cap would cut in first.
    pub(crate) const FT_MAX_UPDATES: usize = 300;

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
    /// FTRAN (see `extended_dual`'s main loop) — no extra BTRAN/FTRAN call
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
    pub(crate) const UPDATE_VERIFY_TOL: f64 = 1e-7;

    /// EXPAND anti-cycling (Gill, Murray, Saunders & Wright, "A practical
    /// anti-cycling procedure for linearly constrained optimization",
    /// Mathematical Programming 45 (1989) 437-474). "Master" feasibility
    /// tolerance the working tolerance `delta` is kept strictly below during
    /// an expanding sequence; also the snap-to-bound threshold used when
    /// resetting nonbasic variables (§4.2-4.3, eq. (4.2)/(4.3)).
    pub(crate) const EXPAND_DELTA_F: f64 = 1e-6;

    /// Iterations per expanding sequence before a reset (§4.2); the paper's
    /// own worked example uses 10000 for large industrial LPs, but this
    /// project's test-scale LPs warrant a much shorter cycle so resets are
    /// actually exercised.
    pub(crate) const EXPAND_K: usize = 50;

    /// Feasibility tolerance an expanding sequence starts from (§4.2: `delta_0 = 0.5 delta_f`).
    pub(crate) const EXPAND_DELTA_0: f64 = 0.5 * EXPAND_DELTA_F;

    /// Ceiling `delta_k` approaches but never reaches within `EXPAND_K` steps
    /// (§4.2: `delta_K = 0.99 delta_f`).
    pub(crate) const EXPAND_DELTA_K: f64 = 0.99 * EXPAND_DELTA_F;

    /// Per-iteration growth of the working tolerance (§4.2: `tau = (delta_K - delta_0) / K`).
    pub(crate) const EXPAND_TAU: f64 = (EXPAND_DELTA_K - EXPAND_DELTA_0) / (EXPAND_K as f64);

    /// Weights never allowed to fall below this — guards against a tiny or
    /// negative value (from accumulated rounding) making a column look
    /// spuriously "steep".
    pub(crate) const STEEPEST_EDGE_FLOOR: f64 = 1e-10;

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
    pub(crate) const RAYON_SIZE_THRESHOLD: usize = 100_000;

    /// Below this many structural+slack columns, pricing every nonbasic
    /// column's reduced cost every iteration is cheap enough that partial
    /// pricing would only add overhead for no benefit; at or above it, both
    /// `run_phase`'s primal entering-variable scan and the dual method's
    /// `chuzc1` switch to [`partial_pricing_sampled`]'s random-group scheme.
    pub(crate) const PARTIAL_PRICING_THRESHOLD: usize = 300;

    /// Group count for partial pricing: roughly `1/PARTIAL_PRICING_GROUPS` of
    /// eligible candidates are priced first; the rest are only priced if that
    /// first group has nothing improving (see [`partial_pricing_sampled`]).
    pub(crate) const PARTIAL_PRICING_GROUPS: u64 = 10;

    /// Ruiz-scaling iterations for the shared presolve pass below — the same
    /// value `interior_point.rs` used before `crate::presolve` was extracted
    /// out to be shared with this module.
    pub(crate) const RUIZ_ITERS: usize = 10;

    /// Constraint-propagation passes per presolve round (`propagate::propagate`'s
    /// own internal bound-tightening loop — see its module docs for the §3.2
    /// activity-bound derivation each pass repeats).
    pub(crate) const PROPAGATION_PASSES: usize = 2;

    /// Upper bound on how many times `presolve::run_extended` cycles through
    /// propagate → dualfix → row-singleton → doubleton → colsingleton — see
    /// that function's own docs for why later rounds can unlock reductions an
    /// earlier round's static structure couldn't yet see, and for the
    /// fixpoint check that stops it short of this cap once a round finds
    /// nothing left to do (so raising this constant costs nothing on a
    /// problem that stops converging early — only genuinely deep elimination
    /// chains ever run all the way to the cap).
    pub(crate) const PRESOLVE_ROUNDS: usize = 20;

    /// Upper bound on how many times each outer `PRESOLVE_ROUNDS` pass itself
    /// cycles through row-singleton <-> colsingleton before `propagate`/
    /// `dualfix` run again — see `presolve::run_extended`'s own docs for why
    /// this inner pair can have more to find after its own first pass (e.g.
    /// colsingleton eliminating a variable turning a row rowsingleton had no
    /// reason to touch into a fresh row singleton), for why `doubleton` isn't
    /// part of this inner repetition (measured net regression when it was),
    /// and for the fixpoint check that stops this loop short of its own cap
    /// once a pass finds nothing left to do.
    pub(crate) const ROWSINGLETON_COLSINGLETON_INNER_ROUNDS: usize = 1;

    /// Above this many variables, a connected component found by
    /// [`connected_components_of_std_form`] is solved via a separate `rayon`
    /// task rather than in the main sequential loop, once *any* component in
    /// the batch clears this size — solving an entire LP (its own presolved
    /// simplex loop, potentially thousands of pivots) is substantial work,
    /// unlike this file's other, deliberately sequential per-*iteration*
    /// loops (`chuzr`, `chuzc1`, DSE weight updates — see
    /// this module's own docs for the profiling that found
    /// `rayon`'s per-call dispatch overhead exceeding *those* loop bodies at
    /// this crate's realistic problem sizes); at this much coarser
    /// "solve a whole sub-problem" granularity, that same dispatch cost is
    /// comfortably negligible in comparison. `200` is the size the user
    /// requesting this feature asked for directly, not independently tuned —
    /// see [`solve_std_form_decomposed`]'s own docs for why no Netlib
    /// instance in this crate's own benchmark set actually exercises the
    /// parallel path at all (every genuine split found there lands well
    /// under this threshold).
    pub(crate) const PARALLEL_COMPONENT_MIN_VARS: usize = 200;
}

/// 拡張双対単体法 (src/simplex/extended_dual.rs)
pub(crate) mod extended_dual {
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
    pub(crate) const XB_CHECK_INTERVAL: usize = super::simplex::FT_CHECK_INTERVAL;

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
    pub(crate) const XB_DRIFT_TOL: f64 = 1e-8;

    /// Every this many drift-triggered refactorizations *within the same
    /// solve*, [`XB_DRIFT_TOL`]'s own effective bound multiplies by
    /// [`XB_DRIFT_ESCALATION_FACTOR`] (capped at [`XB_DRIFT_TOL_MAX`]) — see
    /// that constant's own docs for why this is a per-solve escalation rather
    /// than a static per-problem scale. `10` keeps every instance measured at
    /// single-digit drift-refactor counts (the ones prior attempts broke)
    /// entirely below the first step, while still letting a genuinely
    /// pathological solve (hundreds of triggers at the flat bound) climb
    /// through several steps before this cap's own `1e-4` ceiling.
    pub(crate) const XB_DRIFT_ESCALATION_STEP: usize = 10;

    /// Multiplier applied per [`XB_DRIFT_ESCALATION_STEP`] drift triggers.
    /// `10` mirrors the *single* absolute-loosening step attempt 2 (this
    /// constant's own docs) already measured in isolation (`1e-8` -> `1e-7`
    /// fixed `maros`, broke `fit1p`) — the escalation ladder repeats that same,
    /// already-characterized step size rather than inventing a new one, but
    /// only after `XB_DRIFT_ESCALATION_STEP` proves the *current* solve is
    /// actually the kind that benefits from it.
    pub(crate) const XB_DRIFT_ESCALATION_FACTOR: f64 = 10.0;

    /// Ceiling on the escalated [`XB_DRIFT_TOL`] — reuses `super::FT_RESIDUAL_TOL`'s
    /// already-proven-safe order of magnitude (the classical method's own
    /// absolute drift bound, `1e-4`) rather than letting escalation grow
    /// unbounded into territory no measurement has ever validated.
    pub(crate) const XB_DRIFT_TOL_MAX: f64 = super::simplex::FT_RESIDUAL_TOL;

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
    pub(crate) const PIVOT_ESCALATION_STEP: usize = 0;

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
    pub(crate) const FT_MAX_UPDATES_FACTOR: f64 = 3.0;

    /// Floor for [`ft_max_updates`] — never weaker than the classical method's
    /// own already-proven-safe flat cap, regardless of how small `m` is.
    pub(crate) const FT_MAX_UPDATES_FLOOR: usize = super::simplex::FT_MAX_UPDATES;

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
    pub(crate) const SYNTH_CLOCK_FACTOR: f64 = 16.0;

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
    pub(crate) const SYNTH_CLOCK_MIN_UPDATES: usize = 50;

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
    pub(crate) const D_DRIFT_TOL: f64 = 1.0;

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
    pub(crate) const D_GROSS_MISMATCH_REL_TOL: f64 = 0.5;

    /// `x_B(M)`'s `M`-coefficients are exactly zero or of order one in exact
    /// arithmetic; anything this small is accumulated LU/update noise. Left in,
    /// `Affine1::cmp_lex`'s slope-first order lets it override a comfortably
    /// feasible `base` and drives two-variable cycles (Netlib `greenbea`).
    pub(crate) const X_B_SLOPE_NOISE: f64 = 1e-7;

    /// Absolute tolerance on an `M`-coefficient: stage A treats a slope
    /// deviation as positive only above it, and the stage A -> B handoff counts
    /// a basic `x^1_j` as sitting *on* its slope bound `l^1_j`/`u^1_j` (so that
    /// bound survives into `l^B`/`u^B`) within it. Matches
    /// [`Affine1::gt_zero`]'s own slope threshold, so the stage split draws the
    /// line exactly where the lexicographic comparison it replaces did.
    pub(crate) const SLOPE_TOL: f64 = 1e-9;

    /// How negative `z^1` (the slope of the optimal value `z(M) = z^0 + z^1 M`)
    /// must be to count as `z^1 < 0` (`prop:trichotomy`). A small absolute
    /// tolerance rather than `TOL`: `z^1` is a sum of (possibly many)
    /// reduced-cost terms, so its floor scales with the problem's own cost
    /// magnitudes, not with `TOL`'s coefficient-level tightness — matches this
    /// crate's own precedent of using a looser, separate tolerance for
    /// accumulated-magnitude checks (see `simplex.rs::PRIMAL_FEAS_TOL`'s own
    /// docs for the same reasoning). Shared by stage A's early exit and
    /// [`finish`]'s own unboundedness test so the two can never disagree.
    pub(crate) const Z_SLOPE_TOL: f64 = 1e-7;
}

/// 疎 LU 分解・Forrest-Tomlin 更新 (src/simplex/lu.rs)
///
/// 各定数の測定経緯は `docs/_history_fragments/lu.md` を参照。
pub(crate) mod lu {
    // ---- ピボット選択 (Markowitz) ----

    /// 閾値ピボットの安定性下限: ピボット候補は、その列の活性部分の最大絶対値の
    /// この割合以上でなければならない。求解ごとのピボット閾値
    /// (`sparse_lu::pivot_threshold`) の初期値
    /// (引き上げは既定 off なので実質この値で固定)。
    pub(crate) const STABILITY: f64 = 0.25;

    /// ピボット閾値を引き上げる際の上限 (HiGHS の `kMaxPivotThreshold`)。
    pub(crate) const PIVOT_THRESHOLD_MAX: f64 = 0.5;

    /// 環境変数 `ENOMOTO_PIVOT_THRESHOLD` で指定されたピボット閾値の下限
    /// (HiGHS の `kMinPivotThreshold`)。上書き値のクランプにだけ使う。
    pub(crate) const PIVOT_THRESHOLD_MIN: f64 = 8e-4;

    /// ピボット閾値の 1 段の引き上げ倍率 (`0.25 -> 0.5` で上限に達する)。
    pub(crate) const PIVOT_THRESHOLD_FACTOR: f64 = 2.0;

    /// 消去開始前の次数が `m` のこの割合を超える列を「初期稠密」とみなす
    /// (`find_best_pivot` の稠密列回避、および境界付き分解の境界列検出)。
    pub(crate) const DENSE_COL_FRACTION: f64 = 0.5;

    /// 1 回の `find_best_pivot` が調べる候補列数の上限 (ピボットが見つかっている
    /// 場合のみ適用。`ENOMOTO_PIVOT_SEARCH_LIMIT` で上書き、`0` で無制限)。
    /// 最悪ケースの保険として大きめの値 (HiGHS は 8)。
    pub(crate) const PIVOT_SEARCH_LIMIT: usize = 256;

    /// `find_best_pivot` の行探索 (`ENOMOTO_PIVOT_ROW_SEARCH`) で走査する最大の行次数の
    /// 既定値。`0` = 行探索なし (経路が変わるため既定 off)。
    pub(crate) const PIVOT_ROW_SEARCH_MAX_DEGREE: usize = 0;

    // ---- 活性部分行列の格納 (KernelMatrix) ----

    /// この長さ以下の行ランは二分探索でなく線形探索で列を探す (`KernelMatrix::row_get`)。
    pub(crate) const KERNEL_LINEAR_SCAN_MAX: usize = 16;

    /// `KernelMatrix::new` がバッファ容量を予約する際の入力非ゼロ数に対する倍率
    /// (容量 = `KERNEL_RESERVE_MULT * nnz + KERNEL_RESERVE_EXTRA`)。
    pub(crate) const KERNEL_RESERVE_MULT: usize = 2;

    /// `KernelMatrix::new` の容量予約に加える固定の余裕。
    pub(crate) const KERNEL_RESERVE_EXTRA: usize = 64;

    /// 行・列ランを再配置するときの最小容量。
    pub(crate) const KERNEL_MIN_RUN_CAP: usize = 4;

    /// スレッドごとに保持しておく再利用バケット配列の最大個数 (`BUCKET_POOL`)。
    pub(crate) const BUCKET_POOL_MAX: usize = 4;

    // ---- 分解経路の振り分け ----

    /// 入力の非ゼロ数が `m^2` のこの割合を超えたら Markowitz をやめ、`faer` の
    /// 稠密部分ピボット LU (`factorize_dense_faer`) で分解する。
    pub(crate) const DENSE_INPUT_FRACTION: f64 = 0.25;

    /// 境界付き分解 (`factorize_bordered`) を試す境界列数 `k` の上限 (`k / m`)。
    pub(crate) const BORDER_MAX_FRACTION: f64 = 0.4;

    /// 境界付き分解を試す境界列数 `k` の絶対上限 (`k x k` 稠密分解のコストを
    /// 抑える防御的な上限。通常は `BORDER_MAX_FRACTION` が効く)。
    pub(crate) const BORDER_MAX_COUNT: usize = 3000;

    /// B3 (`ENOMOTO_LU_DENSE_SWITCH`): 活性部分行列の密度が残り `k^2` のこの割合に
    /// 達したら残りを稠密分解に切り替える。既定 `0.0` = 無効 (経路が変わるため)。
    pub(crate) const DENSE_SWITCH_FRACTION: f64 = 0.0;

    /// B3: 稠密切替を検討する残り行数の下限 (`ENOMOTO_LU_DENSE_SWITCH_MIN`)。
    pub(crate) const DENSE_SWITCH_MIN_ROWS: usize = 64;

    /// B3: 稠密切替の密度判定を行う消去ステップの間隔。
    pub(crate) const DENSE_SWITCH_CHECK_INTERVAL: usize = 16;

    // ---- ピボット順の再利用 (factorize_reusing) ----

    /// 再利用分解が作る因子の非ゼロ数が、最後の通常分解の `L`+`U` 非ゼロ数の
    /// この倍数を超えたら再利用を諦める (`ENOMOTO_REUSE_FILL_LIMIT` で上書き可)。
    pub(crate) const REBUILD_FILL_LIMIT: f64 = 1.25;

    /// 再利用分解で、列の残り部分行列の最大絶対値がこれ未満なら特異とみなして
    /// 諦める (質の判定ではなく「ピボットが無い」判定)。
    pub(crate) const REBUILD_MIN_PIVOT: f64 = 1e-12;

    /// 再利用棄却後の指数バックオフのシフト量の上限 (シフト自体のオーバーフロー防止)。
    pub(crate) const REUSE_BACKOFF_SHIFT_CAP: u32 = 5;

    /// 再利用棄却後に見送る再分解回数の上限。
    pub(crate) const REUSE_MAX_BACKOFF: u32 = 16;

    // ---- eta と FTRAN/BTRAN の疎/密切替 ----

    /// eta (`U` の列 eta・`R` の行 eta) の非対角 fill が `m` のこの割合を超えたら
    /// 密形式で格納する (実データでの調整はまだ)。
    pub(crate) const DENSE_ETA_FRACTION: f64 = 0.4;

    /// FTRAN の右辺の非ゼロ数が `m` のこの割合を超えたら、Gilbert-Peierls 疎経路を
    /// やめて密経路を使う (`FtLu::should_use_dense_solve`)。
    pub(crate) const DENSE_RHS_FRACTION: f64 = 0.4;

    /// `FtranDensity` の移動平均で最新の観測に与える重み (HiGHS の
    /// `kRunningAverageMultiplier`)。
    pub(crate) const DENSITY_AVERAGE_MULTIPLIER: f64 = 0.05;

    /// FTRAN チャネルの結果密度の移動平均がこの割合を超えたら、右辺が疎でも
    /// 密経路を使う (`ENOMOTO_EXPECTED_DENSITY_GATE` で上書き、`>= 1.0` で無効)。
    pub(crate) const EXPECTED_DENSE_FRACTION: f64 = 0.35;

    /// BTRAN の `L^{-T}` 段で、入力 `w` の非ゼロが `m` のこの割合以下なら行優先
    /// スキャッタ形式、超えたら列優先ギャザー形式を使う
    /// (`ENOMOTO_BTRAN_L_SCATTER` で上書き、`0` でスキャッタ無効)。
    pub(crate) const BTRAN_L_SCATTER_FRACTION: f64 = 0.10;

    /// C5 超疎 `U` 段: DFS の到達スロット数が `m` のこの割合を超えたら諦めて通常の
    /// 全走査にする (HiGHS の `kHyperFtranU` は 0.10)。
    pub(crate) const U_HYPER_ABORT_FRACTION: f64 = 0.25;

    /// BTRAN 結果の非ゼロステップ記録 (`StepCapture`) を諦める非ゼロ率 (`m` に対する割合、
    /// `ENOMOTO_T_TAU_GP_FRACTION`)。これ以下なら続く `tau` FTRAN の `L` 段を GP で行う。
    pub(crate) const TAU_GP_FRACTION: f64 = 0.1;

    /// FTRAN/BTRAN の結果や新しい eta 要素を厳密な 0 とみなす絶対値の閾値
    /// (`ENOMOTO_TINY`)。既定 `0.0` = 切り捨てなし。
    pub(crate) const TINY_DROP: f64 = 0.0;

    // ---- CLOCK 再分解トリガ用の決定的 tick ----

    /// `FtLu::build_tick` の `m` 比例項の係数 (HiGHS の `buildSynthticTick` と同じ 80)。
    pub(crate) const TICK_BUILD_M_COEF: u64 = 80;

    /// `FtLu::build_tick` の `nnz(L+U)` 比例項の係数 (HiGHS と同じ 60)。
    pub(crate) const TICK_BUILD_LU_COEF: u64 = 60;

    /// `FtLu::build_tick` の消去積和回数項の係数 (S16、`0` = 項なし)。
    pub(crate) const TICK_BUILD_FLOP_COEF: u64 = 0;

    /// 求解段の tick 増分 (触れた非ゼロ数) に掛ける係数 (`1` = 素の非ゼロ数)。
    pub(crate) const TICK_SOLVE_NNZ_COEF: u64 = 1;
}

/// 前処理 (src/presolve.rs, src/presolve/*.rs)
pub(crate) mod presolve {
    /// 【元: src/presolve/aggregator.rs】
    /// 【元: src/presolve/colsingleton.rs】
    /// 【元: src/presolve/dominatedcol.rs】
    /// 【元: src/presolve/doubleton.rs】
    /// 【元: src/presolve/dualfix.rs】
    /// 【元: src/presolve/dualpropagate.rs】
    /// 【元: src/presolve/foldfixed.rs】
    /// 【元: src/presolve/freevar.rs】
    /// 【元: src/presolve/ineqsingleton.rs】
    /// 【元: src/presolve/parallelcols.rs】
    /// 【元: src/presolve/parallelrows.rs】
    /// 【元: src/presolve/rowdominance.rs】
    /// 【元: src/presolve/rowsingleton.rs】
    /// 【元: src/presolve/sparsify.rs】
    /// 【元: src/presolve/stuffing.rs】
    pub(crate) const TOL: f64 = 1e-9;

    /// 【元: src/presolve/aggregator.rs】
    /// Mirrors `colsingleton`/`freevar`'s own pivot guard exactly (same value,
    /// same purpose — see either module's own docs on `SUBSTITUTION_PIVOT_RATIO`).
    /// 【元: src/presolve/colsingleton.rs】
    /// Minimum `|coeff| / max|row|` for a column singleton to be substituted
    /// out — see the guard in `eliminate_singleton_equalities`.
    /// 【元: src/presolve/freevar.rs】
    /// Minimum `|coeff| / max|row|` for either pass below to actually
    /// eliminate a free variable through a given row — mirrors
    /// `colsingleton::SUBSTITUTION_PIVOT_RATIO` exactly (same value, same
    /// purpose: a pivot that is tiny only *relative to its own row* still
    /// amplifies whatever floating-point error the row already carries when
    /// every other entry gets divided by it). Confirmed load-bearing, not
    /// merely defensive: two real Netlib instances (`perold`, `pilot4`, both
    /// already flagged elsewhere in this crate as numerically difficult) were
    /// pushed to a false `Infeasible` — reproducing identically through
    /// `extended_dual` *and* the classical `BIG_M` path, and only when this
    /// module's own elimination ran at all — by a handful of sub-1%-of-row
    /// pivots this module used to accept unconditionally, before this guard
    /// existed.
    pub(crate) const SUBSTITUTION_PIVOT_RATIO: f64 = 1e-2;

    /// Mirrors HiGHS's own `presolve_substitution_maxfillin` default (registered
    /// range `[0, 10]`, default `10`, `HighsOptions.h`): total new nonzeros a
    /// single column's elimination may introduce across every row it folds
    /// into, above which the column is left for a later call instead of
    /// risking a dense-equality-system blowup.
    pub(crate) const MAX_FILLIN: usize = 10;

    /// Mirrors HiGHS's own `nfail == 3` cutoff in `HPresolve::aggregator`: after
    /// this many *consecutive* fill-in rejections, stop trying the rest of this
    /// call's candidate list outright rather than keep paying for the fill-in
    /// check on an already-too-dense region.
    pub(crate) const MAX_CONSECUTIVE_FILLIN_FAILURES: usize = 3;

    /// Relative tolerance for treating a bound-preservation row as already
    /// implied by its terms' own boxes.
    pub(crate) const IMPLIED_TOL: f64 = 1e-9;

    pub(crate) const PROPAGATE_EPS: f64 = 1e-9;

    /// Above this fraction of nonzero coefficients (`nnz / (p * n)`, over the
    /// deduplicated equality rows), [`reduce_equalities`] uses
    /// [`drop_linearly_dependent`] (dense QR) instead of
    /// [`drop_linearly_dependent_sparse`].
    ///
    /// Density, not `p` or a dense-QR flop-count estimate, is what actually
    /// separates the two regimes — calibrated directly against measured
    /// Netlib instances, not derived analytically. The first cut at this
    /// dispatch rule used a pure cost estimate (`(n+1) * p^2`, dense QR's own
    /// flop order, thresholded so `wood1p`'s small `p` routed to dense): it
    /// fixed `wood1p` but *also* routed `standmps` (`p=268`, density 0.96%)
    /// and `fffff800` (`p=350`, density 1.6%) to dense even though the sparse
    /// method was already faster for both there (their low density means
    /// elimination stays close to its own nonzero count, with none of the
    /// fill-in blowup a size-based estimate implicitly worries about) —
    /// measured regressions of roughly 2-4x on both after that first cut.
    /// `wood1p` itself is the outlier that actually needs dense: 11.1% row
    /// density, roughly 7-30x denser than every other measured instance
    /// (`fffff800` 1.6%, `standmps` 0.96%, `sierra` 0.37%, `ganges` 0.31%,
    /// `modszk1` 0.28%, `stocfor2` 0.21%) — dense QR there stayed a bounded
    /// ~30ms while the sparse method's fill-in blew up to 962ms. `3%` sits
    /// with comfortable margin above every instance that must stay sparse and
    /// below `wood1p`'s own density.
    pub(crate) const DENSE_DENSITY_THRESHOLD: f64 = 0.03;

    /// Numerical-stability floor a candidate pivot must clear, relative to the
    /// current **global** maximum active entry anywhere in the matrix (not
    /// just its own column's) — see [`drop_linearly_dependent_sparse`]'s own
    /// docs for why "global" here, not "local to the column", is what makes
    /// this a correct rank-revealing criterion instead of merely a safe-enough
    /// pivot for solving. A different, narrower purpose than `DEP_TOL` below:
    /// this only gates which *candidates* the fill-minimizing search is
    /// allowed to accept, the same role `simplex::lu`'s own `STABILITY`
    /// constant plays for the (unrelated) basis factorization.
    pub(crate) const PIVOT_STABILITY: f64 = 0.1;

    /// Dependency floor, relative to a row's own *original* norm (computed
    /// once, before any elimination) — this is the actual redundancy
    /// criterion. `DEP_TOL * row_orig_norm` plays the same role here that
    /// `1e-9 * col_norm(orig)` plays against `|R[k,k]|` in
    /// [`drop_linearly_dependent`] (see that function's own docs for why a
    /// per-row-relative, not global, threshold matters — the same reasoning
    /// applies here).
    pub(crate) const DEP_TOL: f64 = 1e-9;

    /// Below this total row count across a decomposition's non-trivial
    /// (size > 1) components, [`drop_linearly_dependent_sparse_blocked`] runs
    /// them sequentially rather than via `rayon` — see that function's own
    /// docs for why, unlike every *other* `rayon` call site in this crate
    /// (all gated by `RAYON_SIZE_THRESHOLD`-style raw *problem* size, per
    /// `simplex.rs`'s own docs on measured per-element dispatch overhead),
    /// the right threshold here is total row count *within the blocks
    /// actually being split*, since a component's own elimination is real,
    /// non-trivial work per row (unlike a cheap per-element scan) — a modest
    /// absolute row count here still comfortably pays for `rayon`'s task
    /// dispatch.
    pub(crate) const PARALLEL_DECOMPOSE_ROW_THRESHOLD: usize = 64;

    /// Below this many equality rows, [`drop_linearly_dependent_sparse_blocked`]
    /// skips [`dulmage_mendelsohn_blocks`] entirely and calls
    /// [`drop_linearly_dependent_sparse`] directly, rather than always paying
    /// for the bipartite-matching-plus-SCC pass (and, if it does find multiple
    /// blocks, the per-block `HashMap`-based column remapping and fresh
    /// `BTreeMap`/bucket/heap scaffolding for each one). Measured directly (with
    /// the earlier, coarser connected-components version of this same
    /// decomposition, before it was replaced by the finer Dulmage-Mendelsohn
    /// one — the size/regression picture below is unaffected by that swap,
    /// since both pay similar decomposition overhead on tiny inputs): every
    /// real Netlib win from decomposition (`ship12s` `p=1045`, `ship08s`
    /// `p=698`, `ship04l`/`ship04s` `p=354`, `sierra` `p=528`) has `p` well
    /// above this; every case that regressed when decomposition ran
    /// unconditionally (`sc105` `p=45`, `scorpion` `p=280`, `sc205` `p=91`,
    /// `capri` `p=142`, `standgub`/`standata` `p=160`, `recipe` `p=67`,
    /// `bore3d` `p=214`) sits below it — all by a comfortable margin, so `300`
    /// is not a tight cutoff. Every one of those regressions was itself only
    /// a fraction of a millisecond in absolute terms (these are already
    /// sub-10ms problems), but with nothing to gain there either — the
    /// decomposition's benefit scales with how much per-row elimination work
    /// it *avoids* doing across blocks, which is negligible when the whole
    /// problem is this small to begin with.
    pub(crate) const MIN_ROWS_FOR_BLOCK_DECOMPOSE: usize = 300;

    /// Above this many combined `A`/`G` rows, prefer `rayon`'s parallel
    /// reduce for `compute`'s column-norm fold over a plain sequential scan —
    /// same constant and rationale as `simplex.rs`'s `RAYON_SIZE_THRESHOLD`
    /// (this crate's own `#[ignore]`d `col_norm_fold_rayon_threshold_microbench`
    /// never found `rayon` winning, not even at 4,000,000 rows), duplicated
    /// locally rather than shared cross-module since each of this crate's
    /// rayon-threshold constants is already tuned/re-derived independently per
    /// call site (see e.g. `interior_point.rs`'s own separate
    /// `PROPAGATION_PASSES` copy for the same "each engine keeps its own
    /// tuning constant" convention).
    pub(crate) const RAYON_SIZE_THRESHOLD: usize = 100_000;

    /// Analogue of this solver's own primal feasibility tolerance
    /// (`simplex.rs`'s `PRIMAL_FEAS_TOL`) — the "eps" the cumulative budget
    /// below is measured against, kept as this module's own copy rather than
    /// importing `simplex`'s (this pipeline is shared with `interior_point`,
    /// which has no reason to depend on `simplex`'s own module) since both
    /// represent the same underlying concept: how much primal infeasibility
    /// this solver is willing to call negligible.
    pub(crate) const SMALLCOEFF_EPS: f64 = 1e-7;

    /// Per-row ceiling (as a fraction of [`EPS`]) on the *total*, summed
    /// worst-case activity perturbation this reduction may introduce into any
    /// one row — Achterberg et al.'s own `1e-1 * eps` (see the module docs
    /// for why this single, looser budget suffices on its own).
    pub(crate) const CUMULATIVE_FRACTION: f64 = 0.1;

    /// Coefficients at or below this magnitude are dropped unconditionally,
    /// regardless of the cumulative budget above — Achterberg et al.'s own
    /// `1e-10`, floating-point noise on any realistically scaled problem.
    pub(crate) const NOISE_THRESHOLD: f64 = 1e-10;

    // ==== 以下: presolve/ の小規模モジュール (dualfix, dualpropagate, foldfixed, ineqsingleton,
    // parallelcols, parallelrows, rowdominance, rowsingleton, smallcoeff, sparsify, stuffing,
    // dominatedcol) のコード中から新たに切り出した定数 ====
}

/// 内点法 (src/interior_point.rs, 現在は未使用のエンジン)
pub(crate) mod interior_point {
    pub(crate) const TAU: f64 = 0.995;

    pub(crate) const RHO_MIN: f64 = 1e-10;

    pub(crate) const DELTA_MIN: f64 = 1e-10;

    pub(crate) const RHO0: f64 = 1e-1;

    pub(crate) const DELTA0: f64 = 1e-1;

    pub(crate) const EPS_ABS: f64 = 1e-8;

    pub(crate) const EPS_REL: f64 = 1e-8;

    pub(crate) const MAX_ITERS: usize = 100;

    pub(crate) const STALL_ITERS: usize = 8;

    /// Number of times to redo constraint propagation (bound strengthening +
    /// redundant/infeasible row detection) over the inequality rows after
    /// removing redundant equality rows (see `propagate.rs`). The underlying
    /// technique is itself iterative until a fixpoint; two rounds here mirror
    /// how Gurobi's presolve caps propagation passes per presolve round rather
    /// than iterating to convergence.
    pub(crate) const PROPAGATION_PASSES: usize = 2;

    /// Upper bound on how many times `presolve::run_extended` cycles through
    /// propagate → dualfix → row-singleton → doubleton → colsingleton —
    /// mirrors `simplex.rs`'s own `PRESOLVE_ROUNDS` (see that constant's own
    /// docs for why this is a cap, not a fixed count: `run_extended` itself
    /// stops early once a round converges).
    pub(crate) const PRESOLVE_ROUNDS: usize = 20;

    /// Upper bound on how many times each outer `PRESOLVE_ROUNDS` pass itself
    /// cycles through row-singleton <-> colsingleton before `propagate`/
    /// `dualfix` run again — mirrors `simplex.rs`'s own
    /// `ROWSINGLETON_COLSINGLETON_INNER_ROUNDS` (see that constant's own docs
    /// for why this inner pair can have more to find after its own first
    /// pass, why `doubleton` isn't part of this inner repetition, and for the
    /// fixpoint check that stops it short of this cap).
    pub(crate) const ROWSINGLETON_COLSINGLETON_INNER_ROUNDS: usize = 1;
}

/// 分枝限定法 (src/mip.rs)
pub(crate) mod mip {
    /// How close to an integer a discrete variable's LP-relaxation value must
    /// be to count as "already integer" (`most_fractional` below).
    pub(crate) const INT_TOL: f64 = 1e-6;

    /// How much better a candidate objective must be than the current
    /// incumbent to replace it — guards against replacing the incumbent over
    /// and over for a difference that's really just floating-point noise.
    pub(crate) const OBJ_EPS: f64 = 1e-7;

    /// Safety cap on the number of branch-and-bound nodes explored; if hit,
    /// `solve_mip` returns the best incumbent found so far with
    /// `node_limit_hit: true` rather than the (unproven) true optimum.
    pub(crate) const MAX_NODES: usize = 20_000;
}

/// メモリアロケータ (src/lib.rs)
pub(crate) mod alloc {
    pub(crate) const LARGE: usize = 4 * 1024;
}
