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
pub(crate) mod lu {
    /// Threshold-pivoting stability floor (see this module's own top docs): a
    /// pivot candidate must be at least this fraction of its column's live max
    /// magnitude to be eligible, regardless of Markowitz count. Raised from the
    /// textbook-default `0.1` after measuring that `0.1` lets `factorize()` pick
    /// pivots numerically weak enough to make the *resulting* `L`/`U` drift
    /// faster under `extended_dual`'s `XB_DRIFT_TOL` check (see that constant's
    /// own docs) — i.e. a chain of numerically-marginal Markowitz choices, not
    /// any single one bad enough to fail `FT_MIN_PIVOT` outright, was forcing
    /// extra mid-solve refactorizations well before `FT_BUMP_LIMIT_FACTOR`'s own
    /// eta-fill trigger would have. Netlib's `pilot` (the clearest case)
    /// dropped from 190 drift-triggered refactorizations to 88 at `0.25`
    /// (measured twice, deterministic — refactor counts don't vary run to run,
    /// only wall-clock does), for a ~51% wall-time cut on that instance alone;
    /// `greenbeb`/`fit2p` improved or held flat; `d2q06c` was unchanged within
    /// run-to-run noise (~5%, from system load, confirmed by re-running the
    /// unchanged `0.1` baseline twice). The standard 73-problem Netlib set
    /// (`enomoto_solver.benchmark_highs`, which skips these largest instances on
    /// `n_vars`) is flat within the same noise band either way — this constant
    /// only matters for problems that already refactorize dozens-to-hundreds of
    /// times. `0.5` was tried first and rejected: fill-in from the stricter
    /// floor made every iteration measurably more expensive (`d2q06c`,
    /// `greenbeb`, `fit2p` all ~4% slower net, more than offsetting their own
    /// small refactor-count drops), so `0.5` is *not* simply "more of the same
    /// good direction" — `0.25` is a measured sweet spot, not a floor to keep
    /// pushing from without re-benchmarking.
    /// Since `docs/lu_comparison_enomoto_vs_highs.md` §2.4 this is the
    /// *starting* value of a per-solve threshold that a simplex loop may
    /// escalate ([`pivot_threshold`]) — but that escalation is off by default
    /// (`extended_dual::PIVOT_ESCALATION_STEP`, which records what enabling it
    /// measured), so this remains the floor every solve actually runs at, and
    /// everything measured above still describes the default build.
    pub(crate) const STABILITY: f64 = 0.25;

    /// Ceiling on the escalated pivot threshold ([`escalate_pivot_threshold`])
    /// — HiGHS's own `kMaxPivotThreshold`. [`STABILITY`]'s docs record that a
    /// *static* `0.5` costs ~4% on `d2q06c`/`greenbeb`/`fit2p` through extra
    /// fill-in, which is exactly why this value is reachable only after the
    /// escalation ladder below has evidence that *this* solve is paying more
    /// for instability than it would for fill.
    pub(crate) const PIVOT_THRESHOLD_MAX: f64 = 0.5;

    /// Floor for an operator-supplied `ENOMOTO_PIVOT_THRESHOLD` — HiGHS's own
    /// `kMinPivotThreshold`. Nothing escalates *downwards*, so this only ever
    /// clamps the env override.
    pub(crate) const PIVOT_THRESHOLD_MIN: f64 = 8e-4;

    /// Multiplier applied per [`escalate_pivot_threshold`] step. HiGHS uses
    /// `kPivotThresholdChangeFactor = 5.0` from a `0.1` default; from this
    /// crate's `0.25` a factor of `2.0` lands exactly on
    /// [`PIVOT_THRESHOLD_MAX`] in one step, so the ladder here is
    /// `0.25 -> 0.5`, and a second escalation is a no-op.
    pub(crate) const PIVOT_THRESHOLD_FACTOR: f64 = 2.0;

    /// A column whose *initial* (pre-elimination) degree exceeds this fraction
    /// of `m` is treated as "dense" by `find_best_pivot`'s dense-avoidance
    /// pass — see `MarkowitzState::initially_dense`'s own docs for why a
    /// column's *current* (post-elimination) degree is the wrong thing to
    /// threshold on here. `0.5` catches the handful of near-fully-dense
    /// "trend"/regression columns Netlib `fit1p`/`fit1d`-shaped problems are
    /// built around (confirmed: `fit1p`'s basis has columns with degree
    /// 610-627 out of `m=627`, against a median column degree of `1`) without
    /// also catching moderately-populated columns that pose no real fill-in
    /// risk.
    pub(crate) const DENSE_COL_FRACTION: f64 = 0.5;

    /// How many candidate columns a single `find_best_pivot` call may examine
    /// before it settles for the best pivot it has already found — HiGHS's
    /// `searchLimit = min(nwork, 8)` in `HFactor::buildKernel`
    /// (`docs/lu_comparison_enomoto_vs_highs.md` §2.5), adapted to this file's
    /// bucket scan.
    ///
    /// The existing per-degree-level early exit (`best_score <= deg_col *
    /// deg_col`, the analogue of HiGHS's `merit_limit`) only ever fires at a
    /// *level* boundary, so a single heavily-populated bucket is scanned to
    /// its end no matter how good the pivot found in its first few columns
    /// was. That is the search-explosion case this bound closes: on an
    /// ill-conditioned or fill-heavy step, the low-degree buckets hold
    /// hundreds of columns whose rows all get walked (and whose
    /// `ensure_col_max_abs` recomputes all get paid) to improve on a pivot
    /// that was already acceptable.
    ///
    /// Like HiGHS's, the bound is only honoured once a pivot *has* been found
    /// — `find_best_pivot` never returns `None` because of it, so `factorize`'s
    /// `skip_dense` fallback and its genuine-singularity detection are
    /// unchanged. What it does change is *which* acceptable pivot is returned:
    /// the Markowitz count can be worse than the unbounded scan's, so this
    /// trades (bounded) extra fill-in for a bounded search.
    ///
    /// **`256`, not HiGHS's `8` — measured, see
    /// `analysis/pivot_search_limit_20260922_143000.md`.** `8` was tried first
    /// and rejected: it is not "more of the same good direction", it is a
    /// different intervention. At `8` the bound fires on ordinary steps and
    /// changes the chosen pivot on **64 of the 93** Netlib problems; each such
    /// change perturbs the factorization's last digits, which moves the dual
    /// ratio test's tie-breaks, which moves the iteration count by an amount
    /// whose *sign is effectively arbitrary per problem* (`greenbeb` +18%
    /// iterations, `25fv47` −11%). Reproduced over two independent 93-problem
    /// runs, `8` left three problems past +10% (`greenbeb` +21/+22%, `pilot`
    /// +15/+18%, `grow22` +13/+13%) even though it cut the search everywhere,
    /// and a sweep showed no smaller constant escapes the lottery: `16` made
    /// `pilot87` **2.9x slower**, `64` still perturbed 26 problems.
    ///
    /// `256` is chosen so the bound is a worst-case guard and nothing else. It
    /// fires on 7 of 93 problems, and only one of those (`dfl001`, the single
    /// instance where the unbounded scan is genuinely expensive: 2.96s of a
    /// 22.0s solve, averaging 261 candidate columns per elimination step)
    /// changes materially — its scan drops to 1.54s. The other 86 problems are
    /// bit-identical to the unbounded scan, iteration count and
    /// refactorization count included, so the change cannot regress them at
    /// all. Two independent 93-problem runs: −2.1% and −0.8% in total, no
    /// problem past ±10% in either. `512` was also measured (−3.2%/−, perturbs
    /// only 2 problems) but put `wood1p` at +10.6%, so it fails the same rule
    /// `8` does.
    pub(crate) const PIVOT_SEARCH_LIMIT: usize = 256;

    /// Rows shorter than this are searched for a column linearly rather than
    /// by binary search ([`KernelMatrix::row_get`]). Markowitz elimination is
    /// specifically choosing pivots to keep the active rows short, so the
    /// linear branch is the common one: a run this size fits in one or two
    /// cache lines and scans branch-predictably, where `binary_search` pays a
    /// mispredict per level for the same work.
    pub(crate) const KERNEL_LINEAR_SCAN_MAX: usize = 16;

    /// C5 hyper-sparse `U` stage: give up (and take the plain full scan) once
    /// the DFS has reached more than this fraction of the `m` slots — past it,
    /// sorting the reach and scattering the result by list stop paying for
    /// themselves (HiGHS's own `kHyperFtranU` is `0.10`).
    pub(crate) const U_HYPER_ABORT_FRACTION: f64 = 0.25;

    /// Factorizes the `m x m` sparse matrix given as sparse rows
    /// `(col, value)`. Returns `None` if the matrix is (numerically)
    /// singular — no acceptable pivot remains at some step.
    ///
    /// **A Dulmage-Mendelsohn block-triangularized variant of this function
    /// was implemented, thoroughly validated, and measured — then reverted**:
    /// rows were partitioned into strongly-connected blocks (via bipartite
    /// matching + Tarjan SCC, [`crate::graph::dulmage_mendelsohn_blocks_topological`],
    /// which remains implemented and tested for a possible future, more
    /// targeted revisit) in topological order, each factorized independently,
    /// then reassembled via the block-LU identity `U_ij = L_i^{-1} A_ij` for
    /// "spillover" entries outside a block's own matched columns (`L` itself
    /// stays exactly block-diagonal). Implementation correctness was
    /// confirmed via unit tests (including one that caught a real bug: an
    /// initial version copied spillover entries unchanged, which is only
    /// valid when the emitting block's own `L` is trivial/identity — true for
    /// singleton blocks, which is why singleton-only spillover tests passed
    /// by coincidence before the fix) and zero objective mismatches across
    /// the full 73-problem Netlib benchmark.
    ///
    /// **But it measured as a net ~4% aggregate regression** in a controlled
    /// back-to-back A/B (same machine, same run, only the feature toggled):
    /// dramatic wins on a few instances with genuine block-angular structure
    /// (`fit1p` -44%, `wood1p` -17%, `scsd8` -10%, `sierra`/`sctap3`/`scrs8`
    /// a few percent) were outweighed by a broad ~10-25% tax on most other
    /// medium/large instances (`grow15` +25%, `bnl1` +18%, `modszk1` +17%,
    /// `perold` +16%, `25fv47` +16%, `stocfor2` +14%, `pilotnov` +13%,
    /// `ganges` +11%) — paying bipartite-matching-plus-SCC cost on *every*
    /// refactorization, whether or not it finds anything worth exploiting.
    /// Two cheap pre-gating heuristics were tried to avoid paying that cost
    /// on instances unlikely to benefit, and both failed: (1) whether
    /// `presolve::redundancy`'s own equality-row block decomposition found
    /// structure — `ganges` decomposes beautifully there (1053 blocks, a 1%
    /// bump) yet was still a net loss here, since the *basis* matrix (all
    /// rows, reshuffled by every pivot) doesn't share the *equality
    /// system*'s (static, presolve-time-only) structure; (2) the *basis*
    /// matrix's own bump size at the first real refactorization — `stocfor2`
    /// and `ganges` again showed excellent bump ratios (0.2-1.3%, as good as
    /// or better than the actual winners) yet remained net losses, showing
    /// the fixed decomposition cost itself, not just a poor decomposition
    /// outcome, was the problem. This mirrors HiGHS's own architecture:
    /// `HFactor::buildSimple()` peels off trivial (degree-1/logical) pivots
    /// via a cheap `O(nnz)` sweep with no bipartite matching at all, leaving
    /// full Markowitz elimination (`buildKernel()`) for only the remaining
    /// kernel — this file's own bucket-based `find_best_pivot` already gets
    /// that same cheap benefit for free (confirmed earlier via
    /// `PROF_TOTAL_STEPS`/`PROF_TRIVIAL_STEPS` showing 90-100% of pivots
    /// already resolve trivially), so the *additional*, much more expensive
    /// structure genuine Dulmage-Mendelsohn decomposition can find beyond
    /// that cheap peeling isn't reliably worth its own cost. Fully reverted;
    /// see the project history around this doc comment's own commit for the
    /// full numbers if revisiting.
    /// Input whose nonzero density exceeds this fraction of `m^2` skips
    /// Markowitz elimination entirely in favor of [`factorize_dense_faer`]'s
    /// dense partial-pivoting LU (via the `faer` crate). Markowitz's whole
    /// point is to *minimize fill-in*; a matrix already this dense has none
    /// left to save, so its bucket/degree bookkeeping ([`KernelMatrix`]'s own
    /// row/column runs plus `col_buckets`/`row_buckets`) is pure overhead at
    /// that point — confirmed on a synthetic dense LP
    /// (Netlib has none dense enough to exercise this at all): `factorize`
    /// dominated wall time (95-98%, repeated every few dozen `try_update`
    /// calls since a dense basis's eta fill crosses `FT_BUMP_LIMIT_FACTOR *
    /// m` almost immediately) while this file's own FTRAN-side dense
    /// optimizations (`HybridVec`'s dense arm, `FtLu::should_use_dense_solve`)
    /// together accounted for under 1% of the same wall time — i.e. the eta
    /// chain was never the bottleneck for a dense basis, the cold
    /// factorization was. `0.25` is a first-pass threshold, not yet tuned
    /// against a real dense-problem benchmark (Netlib has none).
    pub(crate) const DENSE_INPUT_FRACTION: f64 = 0.25;

    /// **A second, "peel trivial pivots then Dulmage-Mendelsohn-decompose only
    /// the remaining kernel" variant of block triangularization was also
    /// implemented, tested, and measured — then reverted.** This directly
    /// followed up the first attempt documented below, on the hypothesis that
    /// peeling first (mirroring HiGHS's own `buildSimple()`/`buildKernel()`
    /// split) would fix that attempt's "pays matching+SCC cost on every
    /// refactorization regardless of payoff" problem by shrinking the kernel
    /// matching+SCC actually runs on. It did not: full 73-problem Netlib A/B
    /// showed a **net ~37% aggregate regression** — far worse than the first
    /// attempt's ~4%, and a regression on `fit1p` specifically (+80%), the
    /// exact instance this was meant to speed up. Root cause, confirmed by
    /// direct instrumentation: `fit1p`'s kernel (post-peel) is a single
    /// irreducible ~20-row SCC block every time, so the decomposition gate
    /// *always* rejects it and falls back to a from-scratch
    /// `factorize_flat_markowitz` call — meaning the (redundant) peel work is
    /// paid twice, for zero benefit, every refactorization. Worse, the
    /// underlying premise turned out wrong: `fit1p`'s real cost was never a
    /// large interleaved non-trivial block in the first place. `eliminate`'s
    /// cost is `O(col_rows[pj].len())` (the pivot *column*'s remaining active
    /// rows) times the pivot row's own snapshot size — a pivot with Markowitz
    /// score exactly `0` (row degree `1`, the "trivial" case `PROF_TRIVIAL_STEPS`
    /// counts) is only free when its *column*'s degree is also small; a
    /// degree-1 *row* whose sole entry sits in an otherwise-still-dense
    /// "hub" column is scored as trivial yet costs `O(hub column's current
    /// degree)` to eliminate (every other row sharing that column must be
    /// updated). `fit1p`'s basis apparently has exactly this shape — many
    /// row-degree-1 pivots landing on a handful of not-yet-thinned dense
    /// columns — which no SCC/block decomposition addresses, since those rows
    /// don't form a separable block with the hub column at all. Fully
    /// reverted (including the two dedicated unit tests that validated its
    /// spillover-reassembly correctness, which was never in question — the
    /// numerics were right, just not worth what they cost). See the project
    /// history around this comment's own commit for the full A/B numbers and
    /// the `ENOMOTO_DEBUG_BLOCK_TRIANGULAR` trace output that pinned down the
    /// root cause, if revisiting; `debug_print_block_sizes`
    /// (`ENOMOTO_DEBUG_BLOCK_SIZES`) and the `ENOMOTO_DEBUG_ELIMINATE_COST`
    /// timer below remain as live diagnostics either attempt's numbers came
    /// from.
    /// `factorize`'s own gate for attempting [`factorize_bordered`] before
    /// falling back to plain [`factorize_flat_markowitz`] — see
    /// `factorize_bordered`'s own docs for the technique and why it exists.
    ///
    /// This gate's own detection cost (`detect_border_columns`, one `O(nnz)`
    /// pass) is cheap enough to run unconditionally: a controlled full-73-
    /// problem Netlib A/B (this gate enabled vs. plain
    /// `factorize_flat_markowitz` always) showed no measurable regression on
    /// any instance once run-to-run subprocess scheduling noise was
    /// controlled for (repeated head-to-head timing, not two independently-
    /// scheduled full-batch runs — several apparent double-digit-percent
    /// "regressions" in the first batch-vs-batch comparison, e.g.
    /// `fffff800`/`scfxm1`/`ganges`, vanished under direct repeated
    /// comparison), while several instances beyond `fit1p` itself improved
    /// substantially (`scrs8` -58%, `ship04s` -57%, `shell` -52%, `maros`
    /// -41%, `fit1p` -26%, plus a handful more in the 20-45% range) — this is
    /// the same `k`-nonzero-columns detection [`DENSE_COL_FRACTION`] already
    /// made cheap for `MarkowitzState::initially_dense`'s own purposes,
    /// evidently common enough across Netlib-shaped LPs (not just the
    /// `fit1p`/`fit2p` "trend column" family) to be worth attempting by
    /// default rather than gating behind an opt-in flag.
    ///
    /// **`BORDER_MAX_FRACTION` (`k / m`) is the real, measured constraint —
    /// not an absolute `k` count.** A synthetic-`fit1p`-shaped sweep
    /// (`border_crossover_sweep*` in this module's own tests, `#[ignore]`d,
    /// rerun via `cargo test --release -- --ignored --nocapture border_`) at
    /// both `m=800` and `m=2000` found `factorize_bordered` beating
    /// whatever `factorize_flat_markowitz` would otherwise pick (plain
    /// Markowitz below `is_dense_input`'s own 25% gate, `factorize_dense_faer`
    /// above it — `factorize_bordered` beats *that* too, up to a point) by
    /// **30x-600x** for `k/m` up to `0.40`, crossing over to a wash somewhere
    /// around `k/m ~= 0.5` and a clear loss by `k/m = 0.6` — at *both* `m`
    /// values, i.e. this is a genuine fraction effect (the `k x k` Schur
    /// complement's own `O(k^3)` dense-factor cost, relative to the `(m-k)`-
    /// sized sparse part it's carved out of), not an absolute-`k` one: `m=800,
    /// k=400` and `m=2000, k=1000` (both `k/m=0.5`) landed at the same
    /// break-even point despite `k` itself differing by 2.5x. `0.4` sits with
    /// real margin below the measured crossover.
    ///
    /// The *previous* version of this gate paired that fraction with a
    /// `BORDER_MAX_COUNT` of `200` on the mistaken assumption that unbounded
    /// `k` needed an absolute backstop the way the reverted Dulmage-Mendelsohn
    /// attempt did — the sweep above disproves that directly (`m=2000, k=800`,
    /// five times over `200`, still won by 603x). `BORDER_MAX_COUNT` here is
    /// now a purely defensive sanity bound, sized so its own `O(k^3)` dense
    /// factor stays well under this crate's stated basis-size envelope ("`m`
    /// in the low thousands", per `GpScratch`'s own docs) rather than
    /// something expected to actually bind — `BORDER_MAX_FRACTION` is doing
    /// the real work.
    pub(crate) const BORDER_MAX_FRACTION: f64 = 0.4;

    pub(crate) const BORDER_MAX_COUNT: usize = 3000;

    /// A reuse is abandoned (falling back to a full Markowitz `factorize`)
    /// once the factors it is producing exceed this multiple of the nonzero
    /// count of the last *full* factorization's own `L`+`U`.
    ///
    /// **`1.25` is measured, not guessed.** Fill a reuse produces is not a
    /// one-off cost — it is paid again by every FTRAN/BTRAN for the whole life
    /// of the resulting factorization — and a generous limit is a net *loss*
    /// even though it accepts more reuses: over a 25-problem in-process A/B
    /// (`analysis/` note for this change, §3) the aggregate against the
    /// feature disabled ran `2.0` +2.7%, `1.1` +0.4%, `1.25` -1.5%, with the
    /// `2.0` arm's worst case `greenbeb` +30%. Too *tight* loses the other
    /// way: `1.0`/`1.05` reject nearly every attempt (the basis genuinely
    /// densifies between refactorizations), so the backoff below stops even
    /// trying and the feature turns into pure overhead.
    ///
    /// The reused order was chosen by Markowitz against a *previous* basis;
    /// the current one differs from it by however many Forrest-Tomlin updates
    /// happened since, so the same order can be numerically fine yet produce
    /// far more fill than a fresh Markowitz run would. Fill produced here is
    /// not a one-off cost: it is paid again by every FTRAN/BTRAN for the whole
    /// life of the resulting factorization, which is exactly the trade this
    /// guard exists to cap. The baseline deliberately tracks the last *full*
    /// factorization rather than the immediately-preceding one (see
    /// [`FtLu::fill_baseline`]), so a long chain of reuses cannot ratchet the
    /// limit upward one small increment at a time; a basis whose fill
    /// genuinely grew simply fails this guard once, gets a fresh Markowitz
    /// factorization, and the new baseline is that one's own.
    pub(crate) const REBUILD_FILL_LIMIT: f64 = 1.25;

    /// Absolute floor on a pivot's magnitude: below this the column has
    /// nothing usable left in the remaining submatrix at all, and the whole
    /// attempt is abandoned rather than dividing by (almost) zero. Far below
    /// `simplex.rs`'s own `FT_MIN_PIVOT` deliberately — this is a "there is no
    /// pivot here" test, not a quality test, which the [`STABILITY`] check
    /// next to it already is.
    pub(crate) const REBUILD_MIN_PIVOT: f64 = 1e-12;

    /// Caps on [`factorize_reusing`]'s own exponential backoff after a
    /// rejected reuse: the streak's shift is capped first (so the shift itself
    /// can never overflow), then the resulting skip count.
    pub(crate) const REUSE_BACKOFF_SHIFT_CAP: u32 = 5;

    pub(crate) const REUSE_MAX_BACKOFF: u32 = 16;

    /// A column/row whose off-diagonal fill exceeds this fraction of `m` is
    /// stored densely (see [`HybridVec`]). Unlike [`DENSE_COL_FRACTION`] (tuned
    /// against real Netlib data, all of it sparse), this threshold has no
    /// dense-problem benchmark to tune against yet in this crate's own test
    /// set — `0.4` is a first-pass value, not a measured one; re-tune once a
    /// genuinely dense-coefficient LP is available to benchmark against.
    pub(crate) const DENSE_ETA_FRACTION: f64 = 0.4;

    /// A caller-provided FTRAN right-hand side whose own nonzero count exceeds
    /// this fraction of `m` is dense enough that the Gilbert-Peierls sparse
    /// path's DFS/epoch bookkeeping (see `LuFactors::l_solve_sparse_into`'s own
    /// docs) no longer pays for itself — its reach set is bounded below by the
    /// rhs's own nonzero count, so a dense rhs alone already guarantees a large
    /// reach regardless of how sparse `L` itself is. Exposed as
    /// [`FtLu::should_use_dense_solve`] rather than a flag fixed at
    /// construction time: an earlier version of this gate measured density
    /// once per refactorization from the *basis*'s own `L`/`U` fill and cached
    /// it — which reads as permanently sparse for the entire solve whenever
    /// the crash-start basis (the slack identity, always maximally sparse)
    /// never gets refactorized a second time, silently never firing even on a
    /// genuinely dense-coefficient LP whose real (post-pivoting) basis is
    /// dense throughout. Checking the actual rhs at each call site instead has
    /// no such staleness problem and costs nothing extra (the caller already
    /// has the sparse rhs's length on hand). Like `DENSE_ETA_FRACTION`, `0.4`
    /// is a first-pass threshold, not one tuned against a real dense-problem
    /// benchmark yet.
    pub(crate) const DENSE_RHS_FRACTION: f64 = 0.4;

    /// Weight given to the newest observation when folding it into an
    /// [`FtranDensity`] running average. This is HiGHS's own
    /// `kRunningAverageMultiplier` (`HEkk::updateOperationResultDensity`,
    /// used there for exactly the same purpose — see that class's
    /// `col_aq_density`/`row_ep_density` fields), kept at the same value for
    /// the same reason: small enough that one atypical iteration cannot flip
    /// the dense/sparse dispatch on its own, large enough that a genuine
    /// phase change (a basis that has filled in over the last dozen pivots)
    /// is picked up within ~20 iterations rather than being averaged away
    /// over the whole solve.
    pub(crate) const DENSITY_AVERAGE_MULTIPLIER: f64 = 0.05;

    /// An FTRAN call site whose recent *results* have averaged denser than
    /// this fraction of `m` takes the dense solve regardless of how sparse
    /// the right-hand side it is handed happens to be — see [`FtranDensity`]'s
    /// own docs for why the input's own nonzero count
    /// ([`DENSE_RHS_FRACTION`]) is not a sufficient predictor on its own.
    /// Overridable at run time via `ENOMOTO_EXPECTED_DENSITY_GATE` (see
    /// [`expected_dense_gate`]) so this one number can be re-tuned against the
    /// Netlib set without a rebuild.
    ///
    /// `0.35` is measured, not guessed (`analysis/ftran_density_gate_20260922_062832.md`
    /// §4.2, an A/B over the full Netlib set run *inside one process* with the
    /// setting flipped between solves, since this box's per-problem run-to-run
    /// spread otherwise reaches 4x): against the gate disabled, `0.35` is -3.9%
    /// over the 14 mid-heavy instances and -1% over all 93, while `0.2` is
    /// *worse* than no gate at all (+0.8%). The reason `0.2` loses is specific
    /// and worth keeping: it drags the BFRT combined-flip channel onto the dense
    /// path too (its results average 0.24-0.49 dense, against the entering
    /// column's 0.65-0.99), and on `greenbeb` that turns a -22% win into -1%.
    /// A threshold between the two channels' own measured densities is what the
    /// gate wants, not the lowest one that still fires.
    pub(crate) const EXPECTED_DENSE_FRACTION: f64 = 0.35;

    /// Density ceiling for BTRAN's row-major scatter form
    /// ([`LuFactors::l_transpose_solve_scatter_into`]): the `L^{-T}` stage
    /// takes it only when under this fraction of the incoming `w` is nonzero,
    /// and falls back to the column-major gather form
    /// ([`LuFactors::l_transpose_solve_gather_into`]) otherwise.
    ///
    /// **A gate is needed here, not just a faster kernel.** The two forms
    /// touch exactly the same `L` entries; what differs is the access shape.
    /// Gather reads `w[row_step]` at random and accumulates into one place
    /// (`w[s]`, which the compiler keeps in a register across the whole inner
    /// loop); scatter reads one place (`w[s]`) and does a random
    /// read-modify-write per entry. On a sparse `w` the scatter's whole-step
    /// skip wins outright — most steps do no work at all — but on a dense `w`
    /// nothing is skipped and the scatter is left paying random *stores*
    /// where the gather paid random *loads*, which is strictly worse. Measured
    /// exactly that way on the first ungated A/B of this change: `dfl001`
    /// (whose BTRAN `w` is dense by the time `U^{-T}` and the `R` etas are
    /// done with it) +6.5%, against wins on the sparse-`w` instances. HiGHS
    /// gates all four of its own solve directions for the same reason
    /// (`HFactor::btranL`'s own `sparse_solve` test, `kHyperBtranL`).
    ///
    /// The test is an exact nonzero count of `w`, not a running-average
    /// prediction: unlike an FTRAN's input (whose density is only knowable
    /// from history — see [`FtranDensity`]'s own docs), `w` is right there in
    /// a buffer that every path over it already scans at least once more
    /// (the permutation into `y`), so one early-exiting `O(m)` sequential
    /// pass answers the question exactly, for a fraction of the `nnz(L)`
    /// random accesses the stage itself is about to do either way.
    pub(crate) const BTRAN_L_SCATTER_FRACTION: f64 = 0.10;

    /// Per-row-of-`U`-and-`L` coefficient for [`FtLu::build_tick`]'s `m`-only
    /// term — HiGHS's own `buildSynthticTick` (`HFactor.cpp`) uses `80` for the
    /// analogous term (`num_row * 80`); kept unchanged here rather than
    /// re-derived, since this crate's `refactorize` pays the same *kind* of
    /// fixed per-row bookkeeping (permutation arrays, `u_seq`/`row_owners`
    /// construction in [`FtLu::new`]) HiGHS's own `buildFinish` does, just at a
    /// different (higher, per `docs/lu_comparison_enomoto_vs_highs.md` §3.1 and
    /// this trigger's own analysis §4) constant of proportionality that the
    /// *other* coefficient ([`TICK_BUILD_LU_COEF`]) already carries — see
    /// [`SYNTH_CLOCK_FACTOR`]'s own docs for why the *ratio* between the two
    /// build-tick terms and the *solve*-side tick units is what calibration
    /// actually tunes, not this constant in isolation.
    pub(crate) const TICK_BUILD_M_COEF: u64 = 80;

    /// Per-nonzero-of-`(L+U)` coefficient for [`FtLu::build_tick`] — HiGHS's own
    /// `buildSynthticTick` uses `60` for `(l_nnz + u_off) * 60`. Kept at HiGHS's
    /// own value for the same reason as [`TICK_BUILD_M_COEF`]: this crate's
    /// Markowitz `factorize` (§4/§5 of this trigger's own analysis, measured
    /// when the kernel still used `BTreeMap`/`BTreeSet` storage rather than
    /// today's [`KernelMatrix`]) is 3-25x more expensive *per nonzero* than
    /// HiGHS's `HFactor::buildKernel` — the flattening narrowed that gap but
    /// did not close it, and it is the gap's *existence*, not its exact size,
    /// that makes a
    /// *higher* [`SYNTH_CLOCK_FACTOR`] (not a higher `TICK_BUILD_*_COEF`) the
    /// right lever: raising these two coefficients would inflate `build_tick`
    /// but leave the *solve*-side tick (driven by [`TICK_SOLVE_NNZ_COEF`]) at
    /// the same scale, which double-counts the same "our factorization is
    /// slower" fact the factor calibration already absorbs once.
    pub(crate) const TICK_BUILD_LU_COEF: u64 = 60;

    /// Per-multiply-add coefficient of the elimination flop term in
    /// [`FtLu::build_tick`] (S16, `ENOMOTO_T_TICK_BUILD_FLOP_COEF`). `0` (the
    /// default) leaves `build_tick` exactly the HiGHS-shaped `m`/`nnz(L+U)` sum.
    pub(crate) const TICK_BUILD_FLOP_COEF: u64 = 0;

    /// Per-nonzero coefficient applied to every solve-stage tick increment
    /// (`R`-eta nonzeros touched, `U`/`U^T`-eta nonzeros touched, `L`-stage
    /// reach-set size) — kept at `1` (i.e. `tick` is a plain nonzero count,
    /// unscaled) so [`SYNTH_CLOCK_FACTOR`] alone carries the crate-specific
    /// per-nonzero cost ratio between this crate's own solves and HiGHS's; splitting that ratio across two constants
    /// (this one and the factor) would make calibration harder to reason about
    /// with no accuracy benefit, since both only ever appear multiplied
    /// together in the trigger's own comparison.
    pub(crate) const TICK_SOLVE_NNZ_COEF: u64 = 1;
}

/// 前処理 (src/presolve.rs, src/presolve/*.rs)
pub(crate) mod presolve {
    // ---- 共通 ----

    /// 前処理全般で「0 とみなす」係数・差の絶対許容誤差 (ピボットが 0 か、係数が
    /// 実質 0 か、行が空か等の判定)。
    /// 【元: aggregator, colsingleton, dominatedcol, doubleton, dualfix, dualpropagate,
    /// foldfixed, freevar, ineqsingleton, parallelcols, parallelrows, rowdominance,
    /// rowsingleton, sparsify, stuffing の各 `TOL` を統合】
    pub(crate) const TOL: f64 = 1e-9;

    /// 代入消去のピボット判定: 消去に使う係数が `|coeff| >= この値 * max|row|` を
    /// 満たさなければその行では消去しない (小さいピボットで割ると復元時の誤差が増幅される)。
    /// colsingleton / freevar / aggregator で共通 (`ENOMOTO_T_*SUBSTITUTION_PIVOT_RATIO`)。
    pub(crate) const SUBSTITUTION_PIVOT_RATIO: f64 = 1e-2;

    /// 境界保存行が残りの項の箱制約から既に含意されているかを判定する相対許容誤差
    /// (colsingleton / doubleton)。
    pub(crate) const IMPLIED_TOL: f64 = 1e-9;

    // ---- aggregator ----

    /// aggregator: 1 列の消去で増えてよい非零要素数の上限 (HiGHS の
    /// `presolve_substitution_maxfillin` の既定値)。超える列は見送る。
    pub(crate) const MAX_FILLIN: usize = 10;

    /// aggregator: fill-in 超過による却下がこの回数連続したら、その呼び出しの残り候補を
    /// 打ち切る (HiGHS の `nfail == 3`)。
    pub(crate) const MAX_CONSECUTIVE_FILLIN_FAILURES: usize = 3;

    // ---- propagate ----

    /// 上下限伝播の絶対許容誤差 (上下限の更新幅・矛盾判定・行の冗長判定に使う)。
    pub(crate) const PROPAGATE_EPS: f64 = 1e-9;

    /// 不等式伝播 (`propagate_split`) の相対改善閾値 (`ENOMOTO_T_PROP_RELTOL`)。
    /// 有限の境界は改善幅が `reltol * (1 + |bound|)` を超えるときだけ更新する。
    /// 0 で無効 (絶対閾値 `PROPAGATE_EPS` のみ)。
    pub(crate) const PROP_RELTOL: f64 = 0.0;

    /// 等式伝播 (`propagate_equalities`) の相対改善閾値 (`ENOMOTO_T_EQPROP_RELTOL`)。
    /// 有限の境界は変化量が `reltol * (1 + |old|)` を超えるときだけ更新する。0 で無効。
    pub(crate) const EQPROP_RELTOL: f64 = 0.0;

    // ---- redundancy (等式行の一次従属検出) ----

    /// 重複除去後の等式行の非零密度 `nnz / (p * n)` がこれを超えたら密 QR
    /// (`drop_linearly_dependent`)、以下なら疎消去 (`drop_linearly_dependent_sparse`) を使う。
    pub(crate) const DENSE_DENSITY_THRESHOLD: f64 = 0.03;

    /// 密 QR 経路 (`drop_linearly_dependent`) の従属判定の相対許容誤差。`|R[k,k]|` が
    /// その行自身の拡大ノルム `||[係数; 右辺]||_2` のこの倍数以下なら一次従属として落とす
    /// (`ENOMOTO_T_REDEQ_QR_RANK_TOL`)。
    pub(crate) const REDEQ_QR_RANK_TOL: f64 = 1e-9;

    /// 疎消去でピボット候補が満たすべき安定性の下限。行列全体の現在の最大絶対値に
    /// 対する比で判定する (列内最大でなく全体最大に対する比なので階数判定として正しい)。
    pub(crate) const PIVOT_STABILITY: f64 = 0.1;

    /// 疎消去で行を一次従属 (冗長) とみなす閾値。ピボット値が
    /// `DEP_TOL * (その行の元のノルム)` 以下なら従属と判定する。
    pub(crate) const DEP_TOL: f64 = 1e-9;

    /// ブロック分解後の非自明ブロック (サイズ > 1) の総行数がこれ以上なら、
    /// ブロックごとの消去を rayon で並列実行する。
    pub(crate) const PARALLEL_DECOMPOSE_ROW_THRESHOLD: usize = 64;

    /// 等式行数がこれ未満なら Dulmage-Mendelsohn ブロック分解を省き、
    /// 疎消去を直接呼ぶ (小さい問題では分解のオーバーヘッドが得にならない)。
    pub(crate) const MIN_ROWS_FOR_BLOCK_DECOMPOSE: usize = 300;

    // ---- scaling ----

    /// scaling: A と G の合計行数がこれを超えたら列ノルム集計を rayon で並列化する
    /// (`simplex.rs` の `RAYON_SIZE_THRESHOLD` と同値の独立コピー)。
    pub(crate) const RAYON_SIZE_THRESHOLD: usize = 100_000;

    /// scaling: 行・列のノルムがこれ以下なら 0 とみなしてスケールを更新しない
    /// (`ENOMOTO_T_SCALING_ZERO_TOL`)。
    pub(crate) const SCALING_ZERO_TOL: f64 = 1e-12;

    /// scaling: G の単一要素行 (箱制約行) を専用リストで処理する高速経路を使うか
    /// (1 = 使う。結果は通常経路とビット一致。`ENOMOTO_T_SCALE_UNIT_FAST`)。
    pub(crate) const SCALE_UNIT_FAST: usize = 1;

    /// scaling: 箱制約行を Ruiz 反復から除外し閉形式のスケールを与える実験的経路を使うか
    /// (0 = 使わない。スケール係数が変わる。`ENOMOTO_T_SCALE_NOBOUNDS`)。
    pub(crate) const SCALE_NOBOUNDS: usize = 0;

    // ---- smallcoeff ----

    /// smallcoeff: 無視できる主実行不能量の基準 eps (単体法の `PRIMAL_FEAS_TOL` 相当の独立コピー)。
    pub(crate) const SMALLCOEFF_EPS: f64 = 1e-7;

    /// smallcoeff: 1 行あたりに許す係数除去による最悪活動度変化の合計の上限
    /// (`SMALLCOEFF_EPS` に対する比。Achterberg et al. の `1e-1 * eps`)。
    pub(crate) const CUMULATIVE_FRACTION: f64 = 0.1;

    /// smallcoeff: 絶対値がこれ以下の係数は予算と無関係に除去する (Achterberg et al. の `1e-10`)。
    pub(crate) const NOISE_THRESHOLD: f64 = 1e-10;

    // ---- パイプライン (src/presolve.rs の run_extended) ----

    /// 等式の冗長行削除の方式 (`ENOMOTO_REDEQ_MODE`)。
    /// 0 = ラウンド前に完全版 (重複 + 階数判定)、1 = 重複削除のみ、
    /// 2 = ラウンド前に重複削除、ラウンド後の縮小問題で階数判定。
    pub(crate) const REDEQ_MODE: usize = 1;

    /// G を上下限と多変数行に分離したまま保持するか (1 = 保持。0 だと毎回 CSR を構築。
    /// `ENOMOTO_T_PRESOLVE_SPLIT_G`)。
    pub(crate) const PRESOLVE_SPLIT_G: usize = 1;

    /// 等式行による上下限伝播を行う外側ラウンド数 (最初のこの回数だけ。`ENOMOTO_T_EQPROP_ROUNDS`)。
    pub(crate) const EQPROP_ROUNDS: usize = 2;

    /// 等式行伝播が 1 回でも何も見つけなければ残りのラウンドで省略するか
    /// (0 = 省略しない。`ENOMOTO_T_EQPROP_SKIP_IDLE`)。
    pub(crate) const EQPROP_SKIP_IDLE: usize = 0;

    /// dualpropagate がこの回数連続で何も見つけなければ以降のラウンドで停止する
    /// (`ENOMOTO_T_DUALPROPAGATE_STRIKES`)。
    pub(crate) const DUALPROPAGATE_STRIKES: usize = 1;

    /// doubleton がこの回数連続で何も消去しなければ以降のラウンドで停止する
    /// (`ENOMOTO_T_DOUBLETON_STRIKES`)。
    pub(crate) const DOUBLETON_STRIKES: usize = 1;

    /// parallelcols がこの回数連続で何も併合しなければ以降のラウンドで停止する
    /// (`ENOMOTO_T_PARALLELCOLS_STRIKES`)。
    pub(crate) const PARALLELCOLS_STRIKES: usize = 2;

    /// 構造 (行数・固定列数・ログ長) が前ラウンドと同じなら上下限の変化を無視して
    /// ラウンドを打ち切るか (0 = しない。`ENOMOTO_T_ROUND_STRUCT_STOP`)。
    pub(crate) const ROUND_STRUCT_STOP: usize = 0;

    /// 外側ラウンドの不動点判定で、上下限の変化を進展とみなす相対閾値。
    /// `|u - v| <= この値 * (1 + max(|u|, |v|))` の変化は無視する (`ENOMOTO_T_FIXPOINT_RELTOL`)。
    pub(crate) const FIXPOINT_RELTOL: f64 = 1e-3;

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
