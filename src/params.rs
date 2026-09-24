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
    /// `x_B(M)` の増分維持値のドリフト検査(と eta フィル検査)を行う主ループの反復間隔。
    /// `fill_count` にかかわらず毎回検査し、古典法のように `RESIDUAL_CHECK_MULTIPLIER` で
    /// さらに間引くことはしない(比較の多くが傾き項で `LEX_REL_TOL = 1e-9` という厳しさで決まるため)。
    /// 値は `simplex::FT_CHECK_INTERVAL`(5)と同じ。
    pub(crate) const XB_CHECK_INTERVAL: usize = super::simplex::FT_CHECK_INTERVAL;

    /// `x_B(M)` のドリフト検査の許容誤差(残差 `‖A_B x_B - rhs‖` の絶対値、基底・傾きの両チャネル)。
    /// 比較が `1e-9` の相対許容誤差で決まるので、古典法の `FT_RESIDUAL_TOL`(1e-4)より十分厳しくする。
    /// 1 回の求解内で、ドリフト起因の再分解が [`XB_DRIFT_ESCALATION_STEP`] 回起きるごとに
    /// [`XB_DRIFT_ESCALATION_FACTOR`] 倍に緩める(上限 [`XB_DRIFT_TOL_MAX`])。再分解が少ない求解は
    /// この値のまま。`ENOMOTO_XB_DRIFT_TOL` で上書き可。
    pub(crate) const XB_DRIFT_TOL: f64 = 1e-8;

    /// 同じ求解内でドリフト起因の再分解がこの回数起きるごとに、[`XB_DRIFT_TOL`] の実効値を
    /// [`XB_DRIFT_ESCALATION_FACTOR`] 倍にする(上限 [`XB_DRIFT_TOL_MAX`])。
    pub(crate) const XB_DRIFT_ESCALATION_STEP: usize = 10;

    /// [`XB_DRIFT_ESCALATION_STEP`] 回ごとに [`XB_DRIFT_TOL`] の実効値に掛ける倍率。
    pub(crate) const XB_DRIFT_ESCALATION_FACTOR: f64 = 10.0;

    /// 段階的に緩めた [`XB_DRIFT_TOL`] の上限(古典法の `FT_RESIDUAL_TOL`、1e-4)。
    pub(crate) const XB_DRIFT_TOL_MAX: f64 = super::simplex::FT_RESIDUAL_TOL;

    /// 数値的原因による再分解(FT 更新の拒否、`x_B(M)` ドリフト、`d` ドリフト、PRICE と大きく
    /// 食い違うピボット)がこの回数起きるごとに、LU のピボット閾値を 1 段引き上げる
    /// (`sparse_lu::escalate_pivot_threshold`)。コスト起因のトリガ(eta フィル、更新回数上限、
    /// 合成クロック)は数えない。**0 = 無効(既定)**。`ENOMOTO_PIVOT_ESCALATION_STEP` で上書き可。
    pub(crate) const PIVOT_ESCALATION_STEP: usize = 0;

    /// トリガ (4): FT 更新回数の上限を `m` に比例させる係数。上限は
    /// `max(FT_MAX_UPDATES_FACTOR * m, FT_MAX_UPDATES_FLOOR)`(`ft_max_updates`)。eta 連鎖の
    /// 無制限な伸長を防ぐめったに発火しない安全網。`ENOMOTO_T_FT_MAX_UPDATES_FACTOR` で上書き可。
    pub(crate) const FT_MAX_UPDATES_FACTOR: f64 = 3.0;

    /// トリガ (4) の上限の下限(古典法の `simplex::FT_MAX_UPDATES`、300)。`m` が小さくてもこれより弱くしない。
    pub(crate) const FT_MAX_UPDATES_FLOOR: usize = super::simplex::FT_MAX_UPDATES;

    /// トリガ (5): 決定的な「合成クロック」による再分解。FT 更新回数が
    /// [`SYNTH_CLOCK_MIN_UPDATES`] 以上で、前回の分解以降の求解側の演算量推定(`synth_tick`)が
    /// `SYNTH_CLOCK_FACTOR * build_tick`(分解自体の演算量推定)に達したら再分解する
    /// (HiGHS `HEkk::updateFactor` と同じ考え方。壁時計ではなく演算回数なので再現性がある)。
    /// `ENOMOTO_SYNTH_CLOCK_FACTOR` で上書き可。
    pub(crate) const SYNTH_CLOCK_FACTOR: f64 = 16.0;

    /// トリガ (5) が発火するのに必要な最小の FT 更新回数(HiGHS の
    /// `kSyntheticTickReinversionMinUpdateCount` と同じ 50)。更新直後の誤発火を防ぐ。
    pub(crate) const SYNTH_CLOCK_MIN_UPDATES: usize = 50;

    /// 増分維持している被約費用 `d` のドリフト検査の相対許容誤差: `‖d - fresh_d‖`(固定列を除く)が
    /// `D_DRIFT_TOL * max(‖fresh_d‖, 1)` を超えたら再分解する。
    pub(crate) const D_DRIFT_TOL: f64 = 1.0;

    /// PRICE によるピボット要素 `alpha_q` と FTRAN による `alpha_full[r]` の相対差がこれを超えたら
    /// 「桁違いの不一致」(`pivot_grossly_inconsistent`)としてピボットを破棄する。
    /// `update_verify` の厳しい許容誤差(1e-7)よりずっと緩く、`update_count` によらず常に検査する。
    pub(crate) const D_GROSS_MISMATCH_REL_TOL: f64 = 0.5;

    /// `x_B(M)` の `M` 係数は厳密には 0 か 1 のオーダーなので、絶対値がこれ未満の係数は
    /// LU/更新の雑音とみなして 0 に丸める(`snap_slope`)。`ENOMOTO_T_X_B_SLOPE_NOISE` で上書き可。
    pub(crate) const X_B_SLOPE_NOISE: f64 = 1e-7;

    /// `M` 係数の絶対許容誤差: 段階 A → B の移行で、基底の `x^1_j` が傾き境界 `l^1_j`/`u^1_j` 上に
    /// あるとみなす(その境界を `l^B`/`u^B` に残す)範囲。`Affine1::gt_zero` の傾き閾値と同じ値。
    pub(crate) const SLOPE_TOL: f64 = 1e-9;

    /// 最適値の傾き `z^1`(`z(M) = z^0 + z^1 M`)が `z^1 < 0`(実行不能または非有界、
    /// `prop:trichotomy`)とみなされる負の閾値(`-Z_SLOPE_TOL` 未満)。段階 A の早期終了と
    /// `finish` の非有界判定で共有する。
    pub(crate) const Z_SLOPE_TOL: f64 = 1e-7;

    /// `M`-アフィン量(`Affine1`/`Score2`)の辞書式比較で使う相対許容誤差。各成分の差が
    /// `LEX_REL_TOL * max(|a|, |b|, 1)` 以下なら等しいとみなす(別経路で計算された同じ値の
    /// 数 ULP のずれを吸収する)。`Affine1::gt_zero`/`deviation_flat` の「正」判定、`bfrt_reached` の
    /// 傾き比較、`Score2` の許容誤差の基準値(`ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL` の既定値)、
    /// polish の BFRT 到達判定 `reach_tol` の相対部分にも使う。
    pub(crate) const LEX_REL_TOL: f64 = 1e-9;

    /// 停滞ピボット数の上限 `stall_limit = max(STALL_LIMIT_PER_ROW * m, STALL_LIMIT_MIN)` の
    /// 行数あたりの係数。超えたら Bland 規則(`bland_mode`)に切り替える。
    pub(crate) const STALL_LIMIT_PER_ROW: usize = 5;

    /// `stall_limit` の下限(小さな問題でも最低この回数の停滞は許す)。
    pub(crate) const STALL_LIMIT_MIN: usize = 500;

    /// 「最大改善」chuzr エスカレーションで試行 PRICE を行う DSE 上位候補行の数。
    pub(crate) const GREATEST_IMPROVEMENT_TOP_K: usize = 8;

    /// 「最大改善」エスカレーションの発動閾値 `max(stall_limit / GREATEST_IMPROVEMENT_STALL_DIVISOR,
    /// GREATEST_IMPROVEMENT_STALL_MIN)` の除数(`bland_mode` よりずっと早く発動させる)。
    pub(crate) const GREATEST_IMPROVEMENT_STALL_DIVISOR: usize = 4;

    /// 「最大改善」エスカレーションの発動閾値の下限。
    pub(crate) const GREATEST_IMPROVEMENT_STALL_MIN: usize = 30;

    /// 実行不能行数プラトー検出の上限 `min(INFEASIBLE_PLATEAU_STALL_MULT * stall_limit,
    /// MAX_ITERS_FLOOR / INFEASIBLE_PLATEAU_BUDGET_DIVISOR)` の `stall_limit` に対する倍率
    /// (健全だが遅い求解の揺らぎで誤発火しないよう大きめにする)。
    pub(crate) const INFEASIBLE_PLATEAU_STALL_MULT: usize = 4;

    /// 実行不能行数プラトー検出の上限を反復予算 `MAX_ITERS_FLOOR` の何分の一に抑えるか
    /// (予算内で確実に発火できるようにする)。
    pub(crate) const INFEASIBLE_PLATEAU_BUDGET_DIVISOR: usize = 4;

    /// S2: ドリフト残差の事前検査で使う巡回行サンプルの間隔 `k`(`k >= 2` で有効、0 = オフ)。
    /// `ENOMOTO_XB_DRIFT_SAMPLE` で上書き可。
    pub(crate) const XB_DRIFT_SAMPLE_K: usize = 0;

    /// S2: サンプル推定の残差が `XB_DRIFT_SAMPLE_GUARD * 許容誤差` 以下なら全体の検査を省く。
    /// `ENOMOTO_XB_DRIFT_SAMPLE_GUARD` で上書き可。
    pub(crate) const XB_DRIFT_SAMPLE_GUARD: f64 = 0.1;

    /// 新規残差の下限の係数(0 = オフ): 再分解直後の残差 `r` がすでに許容誤差の
    /// [`XB_DRIFT_FRESH_FLOOR_FRAC`] 倍を超えていれば、次の再分解まで `XB_DRIFT_FRESH_FLOOR_FACTOR * r`
    /// を許容誤差の下限にする。`ENOMOTO_XB_DRIFT_FRESH_FLOOR` で上書き可。
    pub(crate) const XB_DRIFT_FRESH_FLOOR_FACTOR: f64 = 0.0;

    /// 新規残差の下限を有効にする、許容誤差に対する新規残差の割合。
    /// `ENOMOTO_XB_DRIFT_FRESH_FLOOR_FRAC` で上書き可。
    pub(crate) const XB_DRIFT_FRESH_FLOOR_FRAC: f64 = 0.5;

    /// 相対下限の係数(0 = オフ): FT 更新が `XB_CHECK_INTERVAL` 回を超えたら、許容誤差を
    /// `XB_DRIFT_REL_K * 再分解直後の残差` 以上にする。`ENOMOTO_XB_DRIFT_REL_K` で上書き可。
    pub(crate) const XB_DRIFT_REL_K: f64 = 0.0;

    /// FT 更新回数がこれ未満の若い eta ファイルでは許容誤差を
    /// [`XB_DRIFT_MIN_UPDATES_MULT`] 倍に緩める(0 = オフ)。`ENOMOTO_XB_DRIFT_MIN_UPDATES` で上書き可。
    pub(crate) const XB_DRIFT_MIN_UPDATES: usize = 0;

    /// [`XB_DRIFT_MIN_UPDATES`] 未満のときの許容誤差の倍率。`ENOMOTO_XB_DRIFT_MIN_UPDATES_MULT` で上書き可。
    pub(crate) const XB_DRIFT_MIN_UPDATES_MULT: f64 = 100.0;

    /// 実験的な `Score2` 適応許容誤差で、M 側の進展が止まったときに許容誤差を基準値へ引き戻す
    /// 速さの半減期(反復数)。`ENOMOTO_SCORE2_STALL_HALFLIFE` で上書き可。
    pub(crate) const SCORE2_STALL_HALFLIFE: f64 = 50.0;

    /// 実験的な stuck_row ブースト: 同じ行でピボット破棄がこの回数以上続いたらブーストする。
    /// `ENOMOTO_STUCK_ROW_BOOST_THRESHOLD` で上書き可。
    pub(crate) const STUCK_ROW_BOOST_THRESHOLD: usize = 3;

    /// 実験的な stuck_row ブースト: 該当行の逸脱の `M` 係数に掛ける倍率(1.0 = 無効)。
    /// `ENOMOTO_STUCK_ROW_BOOST_FACTOR` で上書き可。
    pub(crate) const STUCK_ROW_BOOST_FACTOR: f64 = 1.0;

    /// S11(実験的): chuzr 候補短縮リストの長さ `K`(0 = オフ)。`ENOMOTO_T_CHUZR_SHORTLIST` で上書き可。
    pub(crate) const CHUZR_SHORTLIST_K: usize = 0;

    /// S11: 実行不能行プールが `CHUZR_SHORTLIST_MIN_POOL_FACTOR * K` 行を超えるときだけ短縮リストを使う。
    pub(crate) const CHUZR_SHORTLIST_MIN_POOL_FACTOR: usize = 4;

    /// S11: 短縮リストが `CHUZR_SHORTLIST_MAX_LEN_FACTOR * K + CHUZR_SHORTLIST_MAX_LEN_SLACK` 行を
    /// 超えたら無効にして全走査に戻す(係数部分)。
    pub(crate) const CHUZR_SHORTLIST_MAX_LEN_FACTOR: usize = 4;

    /// S11: 短縮リストの最大長の定数部分。
    pub(crate) const CHUZR_SHORTLIST_MAX_LEN_SLACK: usize = 64;

    /// S9(既定オフ): `rho` の非ゼロ数が `PRICE_COLUMN_DENSITY * m` を超えたら列方向 PRICE に切り替える
    /// (HiGHS の切り替え点)。`ENOMOTO_PRICE_COLUMN_DENSITY` で上書き可。
    pub(crate) const PRICE_COLUMN_DENSITY: f64 = 0.1;

    /// S8: 前反復の PRICE 行数が `PRICE_LIST_DENSITY * m` 以下なら `rho` の非ゼロ行一覧を作って
    /// それだけを走査する。`ENOMOTO_T_PRICE_LIST_DENSITY` で上書き可。
    pub(crate) const PRICE_LIST_DENSITY: f64 = 0.1;

    /// S6: `x_B` 更新の非ゼロ行数の推定が `XB_LIST_DENSITY * m` 以下なら非ゼロ行一覧を作って
    /// それだけを走査する。`ENOMOTO_T_XB_LIST_DENSITY` で上書き可。
    pub(crate) const XB_LIST_DENSITY: f64 = 0.3;

    /// C5: 入る列の疎 FTRAN で、結果密度の移動平均がこれ未満なら `U` 段を超疎で解く(0 = オフ)。
    /// `ENOMOTO_FTRAN_U_HYPER` で上書き可。
    pub(crate) const FTRAN_U_HYPER_DENSITY: f64 = 0.1;

    /// C5: 融合 DSE `tau` FTRAN で、結果密度の移動平均がこれ未満なら `U` 段を超疎で解く(0 = オフ)。
    /// `ENOMOTO_FTRAN_U_HYPER_TAU` で上書き可。
    pub(crate) const FTRAN_U_HYPER_TAU_DENSITY: f64 = 0.1;

    /// `pivot_grossly_inconsistent` の相対差の分母の下限(0/0 を避けるためだけの極小値)。
    pub(crate) const GROSS_MISMATCH_SCALE_FLOOR: f64 = 1e-300;

    /// `compact_rows` がまとめて非ゼロ判定する行ブロックの大きさ。
    pub(crate) const COMPACT_ROWS_BLOCK: usize = 8;
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
