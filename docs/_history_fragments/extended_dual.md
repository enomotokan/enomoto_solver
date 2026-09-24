# 拡張双対単体法: 改良履歴メモ(extended_dual 担当分)

`src/simplex/extended_dual.rs` と `src/params.rs` の `mod extended_dual` から取り除いた、開発経緯・計測結果・
試して不採用にした案・バグ調査の記録などのコメントを、元の英文のまま項目ごとに集めたもの。
コード中のコメントは現在の動作を説明する簡潔な日本語に置き換えてある。行番号は整理前
(コミット 4ab948b)のファイルでの位置。整理時に改名した識別子は、この記録では旧名のまま
(`sl_*` → `shortlist_*`、`_iter` → `iter_idx`、`fd_cb`/`fd_y` → `fresh_d_cb`/`fresh_d_y`、
`cm_off`/`cm_pos`/`price_cm` → `col_entry_start`/`price_pos_of_col_entry`/`col_entry_of_price`、
`drift_r0` → `drift_resid_after_refactor`、`n_zero_nb` → `n_zero_nonbasic`、関数内の `REL_TOL` → `LEX_REL_TOL` など)。

## src/simplex/extended_dual.rs

### モジュール先頭ドキュメント

(元の位置: L1 付近)

Extended dual simplex with BFRT — the one LP engine behind
`solve_lp_dual`, for every presolved `StdForm` whether or not some
*structural* column still carries a genuine infinite bound (with none
left, the M-side bookkeeping simply stays empty). Called once per
independent connected component by `simplex::solve_std_form_decomposed`.
A `None` return (one of the "should be unreachable" bail-outs below) is
reported to the user as `Status::NotSolved`; there is no other solver to
fall back to.

## The M→∞ symbolic trick, and why no number ever stands for `M`

The paper this implements replaces a fixed numeric truncation
(`+/-M` substituted for `+/-inf`, this crate's own former `BIG_M`
sentinel, since removed for its numerical fragility) with a *symbolic* analysis:
every quantity that would depend on the truncation value is tracked as
an affine function `base + slope * M` ([`Affine1`]) instead of a plain
number, and every comparison between two such quantities is decided by
comparing `(slope, base)` lexicographically (`Affine1::cmp_lex`) rather
than by ever plugging in a concrete `M`. Since the *sign* of a
sufficiently-large-`M` comparison is exactly decided by the
leading (slope) term — falling back to the base term only when slopes
tie exactly — this reproduces precisely what running the classical
algorithm with an arbitrarily large numeric `M` would decide, without
ever risking the numerical fragility (or the "how large is large
enough?" judgment call) a concrete `M` would introduce.

`M`'s only two jobs, per the paper: (1) let every nonbasic *structural*
column be placed at *some* bound (needed so the all-slack initial basis
is dual-feasible purely from the sign of `c`, with no phase 1 —
[`crash`]) even when one side is a genuine infinity, and (2) let the
bound-flipping ratio test flip through such a column instead of
stopping dead at it. Both become well-defined once every affected
quantity is [`Affine1`] instead of `f64`.

The paper's revised §4.2 restricts `M`-tracking to the minimal set `S`
(a one-sided-unbounded column is `M`-tracked only if its cost sign
*forces* `crash` to place it on the genuinely infinite side; every
other one-sided-unbounded column's infinite side would instead be
represented as a real, unreachable infinity, `None`, never `M`-tracked
at all — see `[[extended-dual-s-restriction]]` memory for the design).
**This module currently does *not* do that restriction** — [`delta_of`]
flags every one-sided-unbounded column, matching this module's original
(pre-`S`-restriction) behavior. The `S`-restricted version was
implemented and is mathematically correct (matched HiGHS on the full
Netlib 73-problem set), but was reverted after measuring it as a net
*regression* (7.9s -> 9.6s total): excluding a column from `S` also
excludes it from `width_affine`'s BFRT-flip-eligibility, so several
degenerate instances (`pilot4`, `fit1p`, `degen2`, `degen3`) needed
substantially more real pivots in place of the cheap flips the
unrestricted flagging used to allow. See [`delta_of`]'s own docs for how
to restore the `S`-restricted form.

## Preconditions this module relies on (established by earlier stages)

- A structural column *may* now be genuinely *free* (`lb == -inf` **and**
  `ub == +inf`) — `presolve::freevar::eliminate_free_variables` still
  removes every one reachable through `A`'s own rows before this module
  ever runs, but that module's own docs name a residual case it cannot
  soundly resolve itself (a free variable that only appears in an
  inequality row); `simplex.rs::build_std_form_presolved` no longer
  splits that residual case into `x_j = x_j^+ - x_j^-` before building
  the `StdForm` this module receives — [`delta_of`]/[`hat_lower`]/
  [`hat_upper`] track *both* sides of such a column independently (see
  their own docs) instead. The paper's state `Z` (`NbStatus::Zero`,
  nonbasic at value `0`) is used exactly where the paper uses it
  (`rem:state-F`): [`crash`] places a *zero-cost* free column there, and
  the cleanup lemma's case (A) parks a free column there. A nonzero-cost
  free column starts at `-M`/`+M` by its cost sign like any other
  `M`-flagged column. A `Zero` column is always eligible (ratio `0`) in
  chuzc1 and, when present, is pivoted on directly with no BFRT walk
  (paper \S4.6); it is never flipped and never produced by a leaving
  variable. Only the legacy cleanup (`ENOMOTO_LEGACY_CLEANUP=1`) still
  bails to `None` when it would have to evict another free basic
  variable.
- Every one-sided-unbounded structural column has already been shifted
  (`simplex.rs::build_std_form_presolved`'s own shift step, unchanged
  for this path) so its *finite* side sits at exactly `0` — this module
  leans on that fact directly: [`hat_lower`]/[`hat_upper`] never need a
  variable's own `w_j` term (the paper's general
  `hat_u_j(M)-hat_l_j(M) = w_j + s_j*M` collapses to exactly `s_j*M`
  here, `w_j == 0` always).
- No slack column (a `<=`/`>=` row's own, `[0, inf)`) is ever
  M-flagged: it is one-sided by construction, not a truncated free/
  one-sided *structural* variable, and (per `simplex.rs`'s own module
  docs) the classical method already handles a slack's genuine
  infinity correctly by simply never placing it there — this module
  preserves that behavior unchanged (see [`hat_upper`]'s own docs).

## Deliberate simplifications versus the classical method

This started as a first, from-scratch implementation of the paper's
algorithm, not a symbolic-`M` retrofit of the (since removed) classical
dual method's far more elaborate machinery — but two of the three simplifications
originally listed here have since been ported over (see each item
below); only the third remains as originally written. Kept as a record
of what was deliberately deferred versus what has since caught up, not
as a current-state summary — read [`Score2`]'s and [`try_update`]'s own
call sites for what actually runs today.

1. ~~Dantzig's rule only for the leaving row~~ — **superseded**: the main
   loop now uses `super::DseState`, generalized to
   [`Score2`]'s lexicographic degree-2-in-`M` comparison exactly as the
   paper's §4.5 describes (`dse.weight(i)` feeding every [`Score2`]
   built in the main loop below). Ported once `x_B(M)`'s incremental
   maintenance (item 2 below) made the leaving row's true deviation
   available every iteration instead of only after a full recompute.
2. ~~No incremental basis update~~ — **superseded**: the main loop calls
   `FtLu::try_update` (Forrest-Tomlin eta update) every pivot, exactly
   and only falls back to a full
   [`refactorize`] on `try_update`'s own rejection, the eta-bump-count
   cap, or this module's own tightened drift check ([`XB_CHECK_INTERVAL`]/
   [`XB_DRIFT_TOL`]) — typically single digits per solve, not once per
   iteration (see `ENOMOTO_PROF_PHASES_EXT`'s own `refactor_count`
   breakdown).
3. **Anti-cycling by a raw iteration-count stall counter** (still as
   originally implemented), not the paper's own full `(M, epsilon)`-
   extended lexicographic rule (§4.7): once `stall_limit` iterations
   pass, tie-breaking permanently switches to smallest-index-first
   (`bland_mode`).

### モジュール prof_phases(診断カウンタ)

(元の位置: L129 付近)

Per-phase wall-clock counters for the `ENOMOTO_PROF_PHASES_EXT`
diagnostic — this module's own counterpart to `simplex::prof_phases`
(see that module's own docs), answering the same "where does time in
the loop actually go" question for [`solve_lp_dual_extended`]'s own
main loop instead of the classical `solve_lp_dual_on`'s. Kept as a
separate module (and a separate env var) rather than reusing
`super::prof_phases` directly: that module's statics are private to
`simplex.rs`, and the two loops' phase boundaries don't line up
one-to-one anyway (this loop has no separate primal-update pass — the
combined-flip/entering-column steps fold that into `XB_UPDATE` below,
and there is no analogue of the classical method's own `COMPUTE_RHS_COLS_*`
skip-counting since `compute_rhs_affine` only ever runs at a resync,
not once per iteration).
FTRANs this solve dispatched to the dense path *because of the
result-density gate alone* — i.e. ones whose own right-hand side
was sparse enough that `FtLu::should_use_dense_solve`'s input test
(the only test this crate had before `sparse_lu::FtranDensity`)
would have sent them down the Gilbert-Peierls path. This is the
whole population the §2.7 change moves, so it is the number to
look at first when a problem's wall time shifts:
`density_gate_ftrans=0` means the gate never fired there and any
timing difference is noise or the nonzero-counting overhead alone.
Each tracked channel's own running-average result density at the
last iteration that updated it, in per-mille of `m` (an
`AtomicUsize` because this module's counters are all atomics; the
value is a density, not a count).
Breaks `REFACTOR_COUNT` down by *why* each refactorization fired —
added to answer "why does `25fv47` refactorize 97 times when most
problems refactorize single digits" (see `report`'s own printout):
`VERIFY` is `super::update_verify`'s own cross-check rejecting a
pivot outright (the earliest, cheapest-to-avoid trigger — this
pivot's numbers disagreed enough between PRICE and FTRAN that it
was never committed at all); `TRY_UPDATE` is `FtLu::try_update`
itself refusing a *committed* pivot for too small a resulting
diagonal (`FT_MIN_PIVOT`); `BUMP` is the eta-file-size cap
(`FT_BUMP_LIMIT_FACTOR`); `DRIFT` is this module's own tightened
`XB_DRIFT_TOL`/`XB_CHECK_INTERVAL` residual check (see that
constant's own docs — introduced to fix the `maros` regression).
`d`'s own independent drift check ([`D_DRIFT_TOL`]'s own
docs) firing — distinct from `DRIFT` above, which only ever checks
`x_B(M)`.
`pivot_grossly_inconsistent` (its own call site's docs) firing —
distinct from `VERIFY` above (the ordinary, tight-tolerance
`update_verify` check, gated by `lu.update_count() > 0`): this one
runs regardless of `update_count` but only ever fires on a
qualitatively worse PRICE/FTRAN disagreement than `VERIFY` screens
for.
Trigger (4) (`super::extended_dual::ft_max_updates`) firing — see
that function's own docs. Expected to stay at `0` on every real
instance (it is sized as a rarely-firing safety net, not a routine
lever); nonzero here is itself the signal to re-tune
`FT_MAX_UPDATES_FACTOR`.
A would-be `Infeasible` conclusion refusing to be drawn from an
updated factorization (both `Status::Infeasible` sites' own docs):
the iteration is redone from a fresh one instead. Nonzero here
means this guard actually saved (or at least delayed) an
infeasibility report — on `greenbea` it fires exactly once.
Trigger (5) ([`SYNTH_CLOCK_FACTOR`]'s own
docs) firing — the deterministic operation-count ("synthetic tick")
cost-based trigger, this module's counterpart to HiGHS's own
`kRebuildReasonSyntheticClockSaysInvert`
(`analysis/ft_refactor_trigger_20260922_040850.md` §5/§6). Distinct
from `MAX_UPDATES` above (a fixed `update_count` cap regardless of
how cheap or expensive each individual update's own solves were):
this one refactorizes once the *measured* solve-side work done
against the current factorization is estimated to already cost as
much as re-factorizing it outright would, the actual gap this
trigger's own analysis found between this crate's eta-chain length
(300-2,700 updates before refactorizing) and HiGHS's own
cost-triggered interval (50-140 updates).
Peak `lu.update_count()` observed *at any point* during the solve
(via `fetch_max`, so this is the true peak across every
refactorization interval, not just the value at solve end) — a
calibration gauge for [`FT_MAX_UPDATES_FACTOR`]
itself, printed by `ENOMOTO_PROF_PHASES_EXT` but never consulted by
any control-flow decision.
Diagnostic only (`ENOMOTO_PROF_PHASES_EXT`'s own report): sum of
`best_idx` (candidates flipped before the real pivot) across every
iteration, and how many rows were in `infeasible_rows` at the start
of chuzr each iteration — together give "average BFRT batch size"
and "average infeasible-row pool size" without needing a separate
`#[cfg(test)]`-gated counter.
M-exit-acceleration investigation (`degen3` bottleneck follow-up):
how often the entering column `q` was itself sitting on its `M`
side pre-pivot (the *only* way an `M`-flagged column ever leaves
it), how often a Harris pass-2 window contained an `M`-side
candidate at some index other than the one actually chosen (a
missed opportunity for a same-cost tie-break to prefer draining
`M`), and how often a BFRT flip moved a column *onto* its `M`
side (the mechanism that can make the `M` population grow instead
of only ever shrink).
Generic-degeneracy follow-up (once `M`-exit itself measured as a
minor slice of `degen3`'s iteration count): `DEGENERATE_PIVOTS`
counts iterations whose entering column `q` had (already, before
this pivot) an essentially-zero reduced cost — a pivot that changes
basis composition without moving the objective at all, the
standard definition of a degenerate step. `HARRIS_WINDOW_SIZE_SUM`
(divided by `ITERS` for the average) measures real tie contention
in the Harris pass-2 window regardless of `M`, i.e. how often
*any* two candidates are close enough in ratio to matter for the
tie-break, not just `M`-side ones (`HARRIS_WINDOW_M_MISS` above).
"Ripple" follow-up: does fixing one infeasible row typically leave
the *pool* of infeasible rows smaller, the same size, or larger
than it was at the start of the previous iteration? Compared
iteration-to-iteration against `INFEASIBLE_POOL`'s own per-iteration
snapshot, not within a single iteration (a single dual pivot always
drives its *own* chosen row `r` to exact feasibility — what this
counts is whether the resulting `x_B` shift pushed *other* rows
into infeasibility faster than rows get fixed, one iteration to the
next).
DSE weight quality follow-up: `rho` (this iteration's own BTRAN,
`B^-1 e_r`) is *exactly* `super::DseState`'s own target quantity
(`gamma_r = ||B^-1 e_r||^2`) for the chosen row `r` — so comparing
`dot(rho, rho)` against the incrementally-maintained
`dse.weight(r)` (read just before it, still pre-update) costs
nothing extra (no additional linear solve) and answers whether
`chuzr`'s notion of "how disruptive is pivoting on this row" has
drifted from the true value by the time it actually gets picked —
bucketed by relative error since summing an unbounded ratio in a
plain `AtomicUsize` isn't meaningful.
Per-iteration work-volume counters for the price/chuzc1/DSE
phases (summed over iterations; the report divides by `ITERS`),
only gathered under `ENOMOTO_PROF_PHASES_EXT_WORK=1`:
`rho_p` nonzeros, PRICE's inner-loop entries visited, PRICE's
touched-column count (and how many of those were nonbasic),
chuzc1 candidates, and the nonzero counts of `alpha_q` and of the
DSE `tau` FTRAN, plus the wall time of that `tau` FTRAN alone
(a sub-part of `DSE_UPDATE`).

### 関数 Affine1::cmp_lex

(元の位置: L678 付近)

The paper's `succ` (\S4.5): compares `(slope, base)`
lexicographically — for any `M` at least as large as some
(unneeded-to-compute) threshold, `self.value(M) > other.value(M)`
iff this returns `Greater`.

Both components use a *relative* tolerance, not exact equality: two
`Affine1` values that are mathematically equal (e.g. a row's own
deviation and the BFRT walk's accumulated flip capacity, at exactly
the point capacity should just cover it) are typically computed via
entirely different paths — one through an LU solve chain
(`solve_x_b`), the other by summing per-candidate capacities
(`width_affine`/`scale`/`add`) — so they land a few ULPs apart, not
bit-for-bit identical. An exact `total_cmp` (tried first) treated
that noise as a genuine, decisive difference: confirmed directly on
Netlib `degen2` (named for exactly this kind of degeneracy) — a
slope difference of order `1e-13` on a magnitude-`~2.8` slope made
an exactly-sufficient BFRT capacity register as `Less` than the
deviation it was supposed to exactly cover, so the walk ran off the
end of the candidate list and reported a false `Infeasible`.

### 関数 bfrt_reached

(元の位置: L749 付近)

Whether the BFRT walk's accumulated flip capacity `cum` has caught up
to (covers) the row deviation `w_r` — i.e. whether the walk should stop
*here* rather than keep consuming candidates. Same lexicographic
(slope, then base) comparison as [`Affine1::cmp_lex`], but the base
channel's tolerance is scaled by this row's own basic-value magnitude
(`x_b_base_r`), not `cmp_lex`'s bare `1e-9`-absolute floor (its
`base_scale` floors at `1.0` regardless of the magnitudes the two
`Affine1`s actually carry). That floor is fine for most rows, but this
module's own Forrest-Tomlin drift check already tolerates `x_B` error
up to `XB_DRIFT_TOL_MAX` (`1e-4`) between refactorizations — on a row
whose basic value reaches `1e5..1e8` (large-magnitude Netlib instances,
confirmed on `greenbea`: `analysis/greenbea_20260921_030127.md`), that
ordinary refactorization noise (measured there at `3.0e-7`, five orders
of magnitude above `1e-9`) reads as a genuine, unrecoverable shortfall
and the walk runs off the end reporting a false `Infeasible` — even
though a fresh factorization shows the same row's deviation is exactly
covered. Mirrors `polish_with_true_bounds`'s own `reach_tol` (its own
docs) and `PRIMAL_FEAS_TOL`'s stated purpose (a large-magnitude
row's rounding floor scales with it; a fixed absolute bar is
simultaneously too strict on large rows and too loose on tiny ones).

### 列挙型 MSide(S 制限の撤回)

(元の位置: L861 付近)

Which side(s) of structural column `j` are `M`-tracked (paper's Lemma
4.1, generalized to a genuinely free column tracking *both*): `Lower`
if only `lb[j] == -inf` (placing it at `Lower` means `x_j = -M`),
`Upper` if only `ub[j] == +inf` (`x_j = +M` there), `Both` if genuinely
free (`lb[j] == -inf` **and** `ub[j] == +inf` — presolve's own
documented residual case, a free variable reachable only through an
inequality row; see this module's own top-of-file docs), `None`
otherwise. Always `None` for a slack column (see this module's own docs
on why a slack is never M-flagged) — [`delta_of`] itself doesn't special-
case that; every call site building the full per-column `delta` array
forces slack entries to [`MSide::None`] directly instead (`j >= n_orig`
never reaches this function at all).

REVERTED (2026-09-20) from the paper's revised §4.2 `S`-restricted form
— every one-sided-unbounded column is flagged again, regardless of cost
sign, matching this function's pre-`S`-restriction behavior bit-for-bit.
Reason: restricting to `S` is provably minimal and matches the paper,
but empirically made the 73-problem Netlib total *worse* (7.9s -> 9.6s)
— `width_affine` stops treating a non-`S` column as BFRT-flip-eligible
once it's excluded, forcing a full pivot instead of a cheap flip for
every such column, and several degenerate instances (`pilot4`, `fit1p`,
`degen2`, `degen3`) needed measurably more of those real pivots as a
result — see `[[extended-dual-s-restriction]]` memory for the full
writeup. Restoring this exact `S`-restricted version later needs a
`cost` parameter back (gating which of `Lower`/`Upper`/`Both` a column's
own cost sign actually forces `crash` onto) — removed here since it was
already fully unused in the un-restricted form this reverted to.

### 関数 nb_value_affine

(元の位置: L964 付近)

`None` iff `j` is nonbasic at `status` with a genuinely infinite bound
there — only possible for a slack (a `<=`/`>=` row's own, per
[`hat_lower`]/[`hat_upper`]'s own docs: every structural column tracks
both sides it needs, one-sided or genuinely free alike). Should never
happen given this module's own invariants ([`crash`] and every later
pivot only ever place a column at a side [`hat_lower`]/[`hat_upper`]
resolves to `Some`), but a
violation is a signal to give up cleanly (propagated via `?` up to
[`solve_lp_dual_extended`]'s `None` return, which `solve_lp_dual`
reports as `Status::NotSolved`) rather than the outright process
abort a `.expect()`/`.unwrap()` here would cause across the PyO3
boundary — confirmed to actually occur on two real Netlib instances
(`perold`, `pilot4`) before this was a graceful `None` instead of a
panic, so this is not merely defensive.

### 関数 refactorize

(元の位置: L991 付近)

`B^{-1}` freshly factorized from the current `basis_pos` — no
incremental (Forrest-Tomlin) update, by design; see this module's own
docs, simplification (2).

`prev` is the factorization being replaced, when there is one: its
pivot order is reused (`sparse_lu::factorize_reusing`, this crate's
counterpart to HiGHS's `HFactor::rebuild()`) rather than searched for
again, since the basis it factorized differs from the current one only
by the columns the Forrest-Tomlin updates since then replaced. The
reuse re-checks threshold pivoting and fill at every step and falls
back to the full Markowitz search by itself when either fails, so
passing `prev` never changes *whether* a usable factorization results,
only how much work finding it costs.

### 関数 residual_norm

(元の位置: L1044 付近)

`‖A_B x_B - rhs‖`, using the *true* basis matrix (`std.rows`, filtered
to currently-basic columns via `basis_pos`) against an already-solved
`x_b` — `super::Tableau::basis_residual_norm`'s own check, adapted to
this module's `x_b` (indexed by basis *position*, not by variable)
instead of a full per-variable `x` array. Detects Forrest-Tomlin eta-
chain drift that a rejected-update/periodic-count trigger alone can
miss: `try_update` can keep *accepting* pivots whose accumulated
numerical error nonetheless makes `lu`'s own solves quietly diverge
from the true basis matrix — confirmed as a real (not hypothetical)
cause of non-termination on Netlib `agg`/`25fv47` once this module
stopped refactorizing every iteration.

### 関数 compute_rhs_affine

(元の位置: L1201 付近)

`B x_B(M) = base + slope*M`'s own right-hand side (Lemma 4.1's `b - N
x_N`, split into its `M`-independent and `M`-coefficient parts) — an
`O(nnz(A))` pass over every nonbasic column, the same cost
`super::Tableau::compute_rhs` pays for the classical method's own
(plain-`f64`) `x_B`. Deliberately factored out from the two solves that
used to always follow it immediately (`solve_x_b`, below): the main
loop in [`solve_lp_dual_extended`] now maintains `x_B(M)` incrementally
pivot-to-pivot (see that function's own docs) and calls this — the full
recompute — only at its own periodic drift-check/resync cadence, not
every iteration; `finish` still wants the one-shot `solve_x_b` below
exactly as before.

The returned `rhs_slope` is **empty** iff no nonbasic column sits at an
artificial `M` bound any more (`delta = 0` in Lemma 4.1's own notation)
— the paper's \S4.6 closing state. Empty rather than a length-`m` block
of zeros because that block *is* the whole slope channel's content then:
leaving it unallocated both skips the allocation (this runs on the drift
check's own every-[`XB_CHECK_INTERVAL`]-iterations cadence, not just at
resyncs) and hands every consumer — [`resolve_x_b_into`],
[`residual_norm_affine`], `solve_x_b` — a one-word test for "there is no
`M` coefficient left to compute with", which is exactly the condition
the paper says to stop computing on.

### 関数 resolve_x_b_into

(元の位置: L1256 付近)

The two FTRANs every resync site in [`solve_lp_dual_extended`] runs on
[`compute_rhs_affine`]'s output, with the paper's own \S4.6 closing
remark ("$M$ 係数計算の打ち切り") applied to the slope channel: when
`rhs_slope` is empty (no nonbasic column at an artificial `M` bound —
Proposition `prop:no-return`'s `delta = 0`), the slope right-hand side
is identically `+0.0`, so `B^-1 rhs_slope` is identically `±0.0` and
`snap_slopes` maps every entry of it to `+0.0` — bit-for-bit what
`fill(0.0)` writes. The solve is therefore pure waste and is skipped,
replaying only its synthetic-clock ticks
([`sparse_lu::FtLu::add_zero_rhs_solve_ticks`], the same mechanism the
BFRT combined-flip slope skip already uses) so the CLOCK refactorization
trigger — and with it the entire pivot path — stays unchanged. Verified
empirically, not just argued: with the skip instrumented to run the real
solve alongside it, no resync on any Netlib instance disagreed on either
the result or the tick count.

Note this is an *exact, locally recomputed* condition, not the paper's
absorbing-boundary theorem applied as a latch: it is re-derived from the
live `nb_status` at every resync, so an `M` bound reappearing (which
`prop:no-return` says cannot happen, but which no code here has to
assume) simply turns the full solve back on.

### 構造体 RowDevCache

(元の位置: L1481 付近)

[`row_deviation`]'s own result for every row currently in
[`InfeasibleRows`], cached at the moment that row's membership was last
(re)decided — the extended counterpart of HiGHS's `work_infeasibility`
array (`HEkkDualRHS::updatePrimal`/`updateInfeasList`), which CHUZR
reads instead of re-deriving each row's primal infeasibility from
`baseValue`/`baseLower`/`baseUpper` on every scan. Every input
`row_deviation` reads (`x_b_base[i]`, `x_b_slope[i]`, `basis[i]`,
`noise_feasible[basis[i]]`) only ever changes at a site that already
re-decides row `i`'s membership (this loop's own `InfeasibleRows::set`/
`rebuild` call sites, all of which now go through [`refresh_row`]/
[`rebuild_rows`]), so for every row in the pool the cached value is
bit-for-bit what a fresh `row_deviation` call would return. Entries for
rows *not* in the pool are stale and never read.

### 構造体 RowBounds

(元の位置: L1505 付近)

The current basic variable's cached bounds ([`ColCache`]) and
`noise_feasible` flag, indexed by basis *row* rather than by variable —
HiGHS's own `baseLower`/`baseUpper` arrays (`HEkk::info_`), kept for the
same reason: every primal-feasibility re-check after an `x_B` update
([`refresh_row`]) walks rows in ascending order, and reading bounds
through `basis[i]` turned each of those into three random accesses into
`n_total`-sized arrays (`cache.lower`/`cache.upper`/`noise_feasible`)
instead of sequential ones. Must be updated at every `basis[i]` write
([`RowBounds::assign`]) and every `noise_feasible` write for a basic
variable ([`RowBounds::mark_noise`]); [`RowBounds::deviation`] is then
bit-for-bit [`row_deviation`] (checked under `debug_assertions`).
`(lower, upper)` as plain `f64` for [`deviation_flat`]: the bound's
value when it is a real finite bound (`M`-free, slope `0`), and
`∓inf` when it is absent or an artificial `∓M` side. A `noise` row
is stored as `(-inf, +inf)`: [`deviation_flat`] then returns `None`
for it exactly as its `noise` early-out does, so the flat path reads
one 16-byte pair per row and no `noise` flag (S10).

### 関数 row_deviation_plain

(元の位置: L1657 付近)

Plain-`f64` counterpart of [`row_deviation`]/[`compute_rhs_affine`]/
[`width_affine`], for [`polish_with_true_bounds`] — once cleanup has run,
no nonbasic column is ever `M`-flagged anymore, so this phase operates
directly on `std.lb`/`std.ub` with a single channel throughout, exactly
like `super::solve_lp_dual_on`'s own `row_infeasible`/`chuzr_scan`/
`compute_rhs`. Kept as free functions taking `basis`/`x_b` directly
(matching this module's own convention above) rather than routed through
`super::Tableau`, since `polish_with_true_bounds` — like the rest of this
module — never constructs one.

Returns `(d_dir, magnitude)`: `d_dir == 1` for a deviation *below* `lb`
(row wants to increase), `-1` for *above* `ub`. The tolerance is scaled
by the row's own computed value (`PRIMAL_FEAS_TOL * xi.abs().max(1.0)`),
not a bare `PRIMAL_FEAS_TOL`, matching this function's own long-standing
convention (a flat tolerance is simultaneously too strict on large-
magnitude rows and too loose on tiny ones — `PRIMAL_FEAS_TOL`'s own docs).

### 関数 fresh_d_into

(元の位置: L1755 付近)

`d = c - A^T B^{-T} c_B` recomputed from scratch against the current
factorization — the extended counterpart of `super::fresh_d` (basic
columns get `0`). Called once at the start and after every
refactorization, exactly like the classical loop: the incremental
`d[j] -= theta_d * a_p[j]` update propagates any error linearly through
every later pivot (`e' = e - (e_q / alpha_q) * a_p`), and measured on
Netlib `25fv47` the maintained `d` had drifted by up to `2.4e2` from the
true reduced costs before this resync existed — with up to 543 columns
dual-infeasible under the true values while the maintained `d` reported
none — so chuzc was ranking entering columns on stale numbers.

### 関数 crash

(元の位置: L1823 付近)

Sign-of-cost dual-feasible crash (Proposition 4.3): unlike
[`super::Tableau::crash_dual_feasible`], this never needs both of a
column's bounds to be finite — [`hat_lower`]/[`hat_upper`] are total
(module-docs) precisely because every structural column has at least
one finite side and this module represents the other symbolically.

Takes `cost` as a parameter (the *perturbed* cost, `super::perturb_costs`'
own output) rather than reading `std.c` directly — matching
`super::Tableau::crash_dual_feasible`'s own convention exactly:
perturbation is built to never flip which side of dual feasibility a
sign-based placement lands on, so this is the same placement either
way, but using the same cost vector the incremental reduced-cost
maintenance (`d`, initialized to this same `cost.clone()`) is built on
keeps the two consistent by construction rather than by coincidence.

A zero-cost genuinely free column (`lb == -inf` **and** `ub == +inf`)
goes to `Zero` (value `0`) instead — the paper's `eq:init-status` third
case: `Lower`/`Upper` would both put it at `-M`/`+M`, while `Zero` is
dual feasible (`d_j = c_j = 0` at the all-slack basis) with no `M`
dependence at all. `perturb_costs` leaves free columns unperturbed, so
`cost[j]` here is the true cost. The paper's other zero-cost tie-break
(a column with only `lb == -inf` goes to `Upper`) already falls out of
`perturb_costs`, which nudges such a column's cost negative.

### 関数 refine_zero_cost_placement(実験的・効果なし)

(元の位置: L1861 付近)

EXPERIMENTAL crash refinement: a nonbasic column with *exactly* zero
true cost and both bounds genuinely finite (`boxed`) has reduced cost
exactly `0` at the all-slack basis (`y = 0` there, so `d[j] = c[j]`)
regardless of which bound [`crash`] parks it at — dual feasibility
never prefers `Lower` over `Upper` for such a column, so [`crash`]'s
own tie-break (`cost[j] >= -TOL`, which `perturb_costs` always nudges
positive for a zero-cost boxed column, `perturb_costs`'s own docs)
picks `Lower` unconditionally with no primal-feasibility rationale at
all — pure coincidence of the tie-break's direction, not a considered
choice. This instead sweeps those columns once, in column-index
order, and places each one at whichever side leaves less violation
(summed over the rows it touches) on the *plain* row residual
(`b` minus every nonbasic column's contribution so far) — cheap
(`O(nnz)` over just the flexible columns' own entries) since the
all-slack basis makes `x_B[i] = residual[i]` directly, no LU solve
needed yet. A column touching zero rows costs nothing either way and
is left at `Lower` (its `viol_lo`/`viol_hi` both `0`, so the
`>=`-based tie-break in the call site keeps the original placement,
matching `crash`'s own convention).

A flipped column's own entry in `active_cost` (the `perturb_costs`
output `crash` itself was built from) is negated in lockstep so the
incrementally-maintained reduced cost `d` this loop seeds from stays
consistent with the new placement (`Upper` needs `d[j] <= 0`; the
perturbation for a zero-cost boxed column is always the small
positive `xpert` `crash`'s own doc comment names, so negating it is
exactly the mirror-image nudge `perturb_costs` would have produced
had its own `pc[j] >= 0.0` branch gone the other way) — otherwise
every downstream reduced-cost-sign assumption (chuzc1's `hat_alpha`
sign convention, the stall/anti-cycling check) would see a column
sitting at `Upper` with a positive `d[j]`, a genuine dual-feasibility
violation this function must never introduce.

**Confirmed not to help (73-problem Netlib sweep, `netlib-benchmark-workflow`'s
own methodology)** — gated behind `ENOMOTO_CRASH_ZERO_COST_PLACEMENT`
(default: **off**) rather than removed outright, as a documented
negative result. Reducing the *count* of primal-infeasible rows
immediately after crash is not the same objective as reducing the
dual simplex's own total iteration count: which *specific* rows are
infeasible, and by how much, drives the whole subsequent pivot
sequence (DSE/Devex weighting, BFRT batching, tie-breaking), and a
locally-greedy placement can steer that sequence somewhere worse even
while genuinely lowering the row-violation count it was greedy over.
Measured directly: `greenbea` 7564->8705 iterations, `greenbeb`
10528->13038, `maros` 1435->1660, `nesm` 1383->1447 (all *worse*, not
better), and `perold` collapsed into a pathological pivot sequence —
0.32s at baseline to over 54s (still not done) with this enabled, the
same "discard-and-retry" failure shape `ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL`'s
own docs name on `pilot4`. Left wired up only so a future, smarter
version of this idea (e.g. weighting the greedy choice by how it
affects the *specific* rows DSE/BFRT will actually pivot on next,
rather than a flat violation-count sum) has a tested starting point
to build from, not because this version is a candidate default.

### 関数 trial_row_ratio

(元の位置: L2034 付近)

Trial BTRAN + PRICE + (BFRT-free) ratio test for a candidate leaving row
`row`, without committing any basis/nonbasic-status change — the
"greatest improvement" chuzr escalation's own building block (see that
mechanism's own docs, next to `stall_count`'s declaration). Returns the
smallest `hat_c_j / |hat_alpha_j|` ratio over row `row`'s own Eligible
set (`None` iff Eligible is empty there, mirroring the main loop's own
`Eligible = empty` case — such a row can never actually be pivoted on).
Deliberately skips the full BFRT capacity walk the real pivot commit
does: this ratio alone already lets the caller estimate
`dev_i.scale(ratio)` as this row's real (single-pivot) objective gain,
which is what picks *which* row to commit to — the chosen row's own
commit, further down in the main loop, redoes this same PRICE step
(unavoidably — this function's own scratch buffers are cleared before
returning) and always uses the *full* BFRT walk, so no result computed
here is ever reused for the actual pivot.

`lu_scratch`/`rho`/`a_p`/`touched`/`touched_cols` are the caller's own
reusable scratch buffers (the same ones the main loop's real PRICE step
uses) — guaranteed all-zero/empty on entry and restored to that state
before this function returns, so interleaving trial calls for several
candidate rows with each other, and with the main loop's own later use
of the same buffers for the row it actually commits to, is safe.

### 構造体 ColCache

(元の位置: L2139 付近)

Per-column cache of [`hat_lower`]/[`hat_upper`]/[`width_affine`], built
once before [`solve_lp_dual_extended`]'s main loop starts and read for
the rest of that call (`finish`/`polish_with_true_bounds`'s own tail
included). All three depend only on `std.lb[j]`/`std.ub[j]` and `n_orig`,
none of which ever changes once this module's `std` is built —
critically, `nb_status[j]` (which *does* change every pivot) plays no
part in any of the three, so nothing here ever needs invalidating
mid-solve. Exists because [`row_deviation`] and the BFRT candidate walk
both re-derive these same `Affine1`s (and re-index `std.lb`/`std.ub`) on
every row/candidate they visit — real cost in the chuzr/chuzc hot paths
at scale, for a value that was already fully determined before the loop
even started.

### 本体: スラック列のコストを摂動しない理由

(元の位置: L2223 付近)

Cost perturbation (`super::perturb_costs`), reused directly rather
than reimplemented — the same anti-degeneracy mechanism
`polish_with_true_bounds` already relies on, now applied to this
phase's own reduced-cost maintenance too (`d`, below) and its own
crash (`crash` takes this same vector, not `std.c`, for exactly the
reason its own docs give).
Slack columns keep their exact (zero) cost: `perturb_costs` nudges
every one-sided column, slacks included, which makes `y = B^-T c_B`
nonzero even at the all-slack basis — so the true reduced cost of a
structural column then differs from `active_cost[j]`, and [`crash`]
(which places by the sign of `active_cost`) starts dual-*infeasible*
(70 columns on Netlib `perold`). HiGHS perturbs logicals only at a
`1e-12` scale for the same reason. With slacks unperturbed, `y = 0`
at the start and the crash is dual feasible by construction.

### 本体: delta と cache_orig

(元の位置: L2242 付近)

`delta[j]` (see [`delta_of`]'s own docs — currently the reverted,
un-`S`-restricted form: every one-sided-unbounded or genuinely free
structural column is flagged, regardless of `active_cost`'s sign).
`cache_orig` holds the true `M`-affine bounds (`hat_l`, `hat_u`) —
what `finish`'s cleanup and the `m == 0` shortcut need. The main loop
itself runs on `cache`, the bounds of whichever problem the current
stage solves ([`Phase`]'s own docs): the slope problem first, then the
intercept problem, swapped at the stage A -> B handoff.

### 本体: 段階 A の省略(S15)

(元の位置: L2285 付近)

Stage A is empty when crash parks no nonbasic column on an `M` side
(`S = empty`, eq:init-status): then `x_N^1 = 0`, so the all-slack
basis has `x^1 = 0`, which is already optimal for the slope problem
(its objective `c^T x^1 = 0` and every slope deviation is `0`). Start
directly in stage B on the unmodified bounds instead of entering
stage A only to hand off on its first iteration.
(S15: `cache` is built once, here, for whichever stage the solve
actually starts in — previously the slope problem was always built
first and thrown away when stage A turned out empty.)

### 本体: d の増分維持と fresh_d_buf

(元の位置: L2326 付近)

Reduced costs (`d`), maintained incrementally (Huangfu & Hall
§2.2.3's update-dual, `super::solve_lp_dual_on`'s own `d`) instead
of a fresh BTRAN(`c_B`) every iteration — sound here because a
column's *cost* never depends on `M` (only its *bounds* do), so `d`
is a plain `f64` array exactly like the classical method's, `M`
never entering into it at all. Correct at the all-slack start for
the same reason the classical crash's `d` is: `y = 0` there (every
slack's cost is `0`), so `d[j] = active_cost[j] - 0`.
Scratch for the `d`-drift check ([`D_DRIFT_TOL`]'s own docs) — a
fresh `d` recomputed here is thrown away once compared against the
incrementally-maintained one; the real, kept recomputation on an
actual trigger still goes through `fresh_d_into(..., &mut d)`
directly, unchanged.

### 本体: 作業バッファ群

(元の位置: L2344 付近)

Buffer reuse: every one of these is sized once, before the loop,
and reused every iteration (`super::solve_lp_dual_on`'s own
convention — see that function's own docs on why a fresh `Vec` of
`O(m)`/`O(n_total))` size every iteration costs real time at scale).
Scratch for [`residual_norm_affine`]'s own combined drift-check pass.
Ascending nonzero-row list of `rho` for the sparse PRICE (S8), and the
previous iteration's priced-row count that predicts `rho`'s density.
Ascending nonzero-row list of `alpha_full` (plus the BFRT flip
result) for the sparse `x_B` update (see its use site).
Scratch for the DSE `tau` half of the fused entering-column/`tau`
FTRAN ([`sparse_lu::FtLu::solve_into_pair_capture`]); `lu_scratch`
serves the entering-column half, as it does for the plain dense solve.
Nonzero steps of this iteration's `rho` (recorded by its BTRAN) so the
fused `tau` FTRAN's `L` stage can take the Gilbert-Peierls path — see
[`sparse_lu::StepCapture`]. Bit-identical to the dense `L` stage.
Dedicated zero-kept scratch + touched-position lists for the pivotal-row
BTRAN — see [`sparse_lu::UnitBtranWork`]. `e_tilde_buf` must not be
written by anything else in this loop (it is not).
`ENOMOTO_FUSED_DSE_FTRAN=0` restores the two separate solves (A/B
only — the fused form is bit-identical, see its own docs).
BFRT combined-flip FTRAN (dense branch) folded into the same fused
traversal as a third vector ([`sparse_lu::FtLu::solve_into_triple_capture`]);
`ENOMOTO_FUSED_BFRT_FTRAN=0` restores the separate solve (A/B only —
bit-identical). `combined_scratch` is that third vector's scratch.
Apply the BFRT combined-flip result to `x_B(M)` inside the entering
column's own `x_B` update loop (one pass, one `refresh_row` per row)
instead of a separate `0..m` pass. `ENOMOTO_MERGE_FLIP_XB=0` restores
the separate pass (A/B only). Per-row arithmetic is unchanged; only the
order of `InfeasibleRows` membership changes can differ.
Dedicated `try_update_precomputed` capture buffers — see
`super::solve_lp_dual_on`'s own identical pair (`a_tilde_buf`/
`e_tilde_buf`) for the full reasoning: `e_tilde_buf` is filled as a
side effect of this iteration's `rho` BTRAN just below, `a_tilde_buf`
as a side effect of this same iteration's entering-column FTRAN
further down, both replacing what `try_update` used to recompute
from scratch. Kept separate from every other buffer here for the
same reason: nothing else may write through them between capture and
the `try_update_precomputed` call.
chuzc1's heap storage and the non-`bland` walk's popped prefix,
kept across iterations (this loop's preallocate-once convention)
instead of a fresh `BinaryHeap`/`Vec` allocation every pivot.
chuzc1's branch-free write-then-keep buffer (see that step's docs).

### 本体: PRICE を非基底列だけにする分割(price_nonbasic_only)

(元の位置: L2415 付近)

**Path-changing, default on** (`ENOMOTO_PRICE_NONBASIC_ONLY=0`
restores the old every-non-fixed-column PRICE; NETLIB93 A/B against
that: -5.3% total, no problem >10% slower): HiGHS's own row-wise
PRICE matrix is *partitioned* (`HighsSparseMatrix::createRowwisePartitioned`/`update`, `p_end_`):
each row's nonbasic entries sit in `[start, p_end)`, its basic ones
after, swapped across the boundary on every basis change, so PRICE
never visits a basic column at all. This loop's PRICE used to
include basic columns deliberately (see PRICE's own comment below:
the leaving column needs its `d` update) — measured at 35-58% of all
PRICE entries on the heavy Netlib instances (`dfl001` 41%, `pilot87`
39%, `maros-r7` 58%). With the partition, the leaving column's `d` is
instead set directly the way HiGHS's `HEkkDual::updateDual` does
(`workDual[variable_in] = 0; workDual[variable_out] = -theta_dual`),
exact in infinite precision but no longer bit-identical: basic
columns stop accumulating the rounding noise their `a_p ~ 0` entries
used to feed into `d`, so the pivot path can drift.
`price_nb_end[i]` is row `i`'s partition boundary; with the flag off
it is simply the row end (every non-fixed column priced, as before).
S12: position index for the partition swaps below (HiGHS keeps no such
index and pays `O(row length)` per swapped entry, as this loop used
to). `cm_off[j]` is column `j`'s offset into `std.cols`' entry order,
`cm_pos[cm_off[j] + k]` the PRICE position of `std.cols.col(j)[k]`
(`u32::MAX` for a fixed column, never in the PRICE matrix), and
`price_cm[p]` the inverse (`std.cols` entry index of PRICE entry `p`).
Every swap keeps both in step, so the swapped positions — and hence
`price_col`/`price_val` — are exactly the ones the linear `position`
search found (one entry per (row, column) pair).

### 本体: PRICE 専用の行優先コピー(S15)

(元の位置: L2446 付近)

PRICE's own row-major copy of `A`, built once: `std.rows` minus
every fixed column (`lb == ub`, which PRICE skips anyway), in
struct-of-arrays form with `u32` column indices. Each row keeps
`std.rows.row(i)`'s own column order (within each partition part),
so the accumulation into `a_p` (and hence every `a_p[j]` bit) is
unchanged — what it saves is the two random `std.lb[j]`/`std.ub[j]`
loads and the branch per visited entry, plus 4 bytes of index per
entry (12 vs 16 bytes). With `price_nonbasic_only` each row is
written already partitioned (nonbasic entries, then basic ones, each
in row order — S15: formerly a second pass through a temporary).

### 本体: FTRAN 結果密度の移動平均(§2.7)

(元の位置: L2528 付近)

Per-call-site FTRAN **result**-density running averages, feeding the
dense/sparse dispatch below alongside each solve's own input
nonzero count (`sparse_lu::FtranDensity`'s own docs, and
`docs/lu_comparison_enomoto_vs_highs.md` §2.7 — the gap this closes:
an rhs that is sparse on input says nothing about how far `L`'s own
reach fans out, and that gap widens with `m`). The entering column's
FTRAN and the BFRT combined-flip FTRAN keep separate histories
because their right-hand sides (one constraint column vs. a sum over
every column flipped this iteration) fill in to genuinely different
densities. Declared out here with the buffers they parallel, so the
history survives every refactorization this loop does: it is a
property of the solve, not of any one `FtLu` (HiGHS keeps the same
averages in `HEkk`, likewise across INVERTs).
C5: result density of the fused DSE `tau` FTRAN, gating its
hyper-sparse `U` stage (`ENOMOTO_FTRAN_U_HYPER_TAU`).

### 本体: noise_feasible

(元の位置: L2580 付近)

`super::solve_lp_dual_on`'s own `noise_feasible`, ported: a row
whose own infeasibility, at the Eligible=empty juncture below, is
proven to be within this problem's own rounding noise (scaled by
that row's own RHS magnitude, not a bare absolute constant — see
that site's own docs, and `PRIMAL_FEAS_TOL`'s own doc comment's
`agg` example: an infeasibility of `1.4e-7` on a row whose own
`rhs ~ 3.4e6` is noise, not a violated constraint) is marked here
and skipped by every later `chuzr` scan instead of being reselected
(and re-failing chuzc1 identically) every subsequent iteration.
Indexed by *variable* id (`basis[i]`), matching that classical
mechanism's own convention, since the row index `i` a variable
currently occupies can change across pivots but this fact about
the variable itself does not.

### 本体: 最初のピボットから厳密 DSE を使う理由

(元の位置: L2618 付近)

Leaving-row weighting (paper \S4.5): `super::DseState`, reused
directly — its weights are pure, `M`-independent tableau-row
quantities (see [`Score2`]'s own docs); only the *score* they feed
into (`Score2`, generalizing plain `delta^2/w` to a degree-2
polynomial in `M`) is specific to this module.
Exact DSE from the very first pivot (a Devex-then-escalate scheme was
tried and measured slower). The all-slack `B0` is a signed identity,
so `DseState::new`'s unit weights are exact here. The earlier
"DSE from the start is a gamble" finding (`bnl1`/`perold` ~190x
slower) was not a pricing effect at all: both blow-ups were the
`update_verify` refactor-and-retry loop below firing on a freshly
factorized basis (see that check's own comment); with that fixed,
a 73-problem Netlib sweep put DSE-from-start at 8.7s total against
10.7s for Devex-start (fit1p 1.95s -> 0.49s, wood1p 0.31s -> 0.16s,
cycle 0.42s -> 0.13s), with no problem regressing by more than the
run-to-run noise except `maros` (0.15s -> 0.24s).

### 本体: 「最大改善」chuzr エスカレーション(実験的)

(元の位置: L2636 付近)

EXPERIMENTAL "greatest improvement" chuzr escalation: once `stall_count`
(already-existing objective-progress stall signal, see its own
increment site below) crosses this threshold — well before `bland_mode`
would latch on at `stall_limit` — row selection stops trusting DSE's
cheap geometric proxy (`Score2`, \S4.5) and instead directly estimates
the *actual* dual objective gain of pivoting on each of the top
`GREATEST_IMPROVEMENT_TOP_K` DSE-ranked candidate rows (a real trial
BTRAN+PRICE+ratio-test per candidate, [`trial_row_ratio`]'s own docs),
picking whichever row's estimated gain is largest instead of whichever
has the largest DSE score. The motivation is exactly DSE's own known
gap: `Delta_i^2/gamma_i` approximates the objective gain achievable by
*some* pivot on row `i`, without the true reduced costs `d[j]` (only
the tableau geometry) ever entering the estimate — usually a good
enough proxy, but not identical, and the gap is largest on exactly the
kind of degenerate instance that stalls DSE in the first place.
Deliberately gated behind a stall signal rather than replacing DSE
outright: the per-candidate trial pricing costs roughly as much as
this iteration's *own* real PRICE step, times `GREATEST_IMPROVEMENT_TOP_K`,
so paying it every iteration on a healthy (non-stalling) solve would
only add cost for no benefit DSE wasn't already providing.

### 本体: 実行不能行数プラトー検出の上限

(元の位置: L2664 付近)

See the infeasible-row-count plateau check's own docs (this loop's
body, next to `stall_count`'s own increment) — a much larger
threshold than `stall_limit` deliberately, to stay clear of a
healthy-but-slow solve's own normal infeasible-count fluctuation.
... but capped so the trigger can actually fire inside this loop's
own `MAX_ITERS` budget. `4 * stall_limit` alone is `20 * m`, which
for any `m > 1000` exceeds `MAX_ITERS` outright — on those problems
the plateau detector could never fire at all, however static the
infeasible set got, and the solve just spent its whole budget
before falling back (Netlib `greenbea`, `m = 2056`: limit 41,120
against a 20,000-iteration budget, with the infeasible count sitting
at a constant 302 for the last ~12,000 of them). A safety net sized
above the budget it is meant to protect is not a safety net.
Smallest infeasible-row count seen so far, *not* the previous
iteration's — see the plateau check's own docs in the loop body.

### 本体: ドリフト許容誤差関連の状態

(元の位置: L2682 付近)

Override for A/B testing [`XB_DRIFT_TOL`] itself (the escalation
ladder's own starting point) — see that constant's own docs for the
four prior single-knob attempts this per-solve escalation replaced.
Escalation state for [`XB_DRIFT_TOL`]'s own per-solve ladder — counts
drift-triggered refactorizations *in this solve only* (reset to `0`
for every call, unlike a module-level constant); see that constant's
own docs for why counting this directly, rather than deriving a bound
from any static per-problem property, is what finally separates
"genuinely drift-heavy solve" from "fragile to any loosening at all".
Residual measured at the first drift check after a refactorization
(the factorization's own noise floor) — see the drift check below.
S2 (`ENOMOTO_XB_DRIFT_SAMPLE`, default off): rotating row-subsample
pre-check of the drift residual — see [`sampled_residual_affine`].
Fresh-residual floor (`ENOMOTO_XB_DRIFT_FRESH_FLOOR`, default off):
the residual measured right after each refactorization's resync;
when that fresh value is itself already close to the drift
tolerance (refactorizing cannot bring it lower), `factor * fresh`
becomes a floor on the tolerance until the next refactorization.
§2.4's own per-solve ladder, counted separately from
`drift_trigger_count` because it answers a different question: that
one counts only the `x_B(M)` residual trigger (whose *tolerance* it
loosens), this one counts every numerically-caused refactorization
(see [`PIVOT_ESCALATION_STEP`]) and tightens the *factorization*
instead. `0` disables the escalation entirely, which is how the A/B
behind it is produced without a rebuild.

### 本体: d のドリフト検査の粗い周期

(元の位置: L2734 付近)

Separate, coarser cadence for the `d`-drift check below, mirroring
`RESIDUAL_CHECK_MULTIPLIER`'s own rationale for the classical
path's `since_residual_check`: this check's residual is dominated by
fixed columns (`lb == ub`) that `d` is never updated for in the first
place (PRICE skips them), so once those are excluded from the
residual (see the loop below) genuine drift stays many orders of
magnitude below `D_DRIFT_TOL` — measured on the full Netlib suite, it
fires on only one problem, and even there it's the same fixed-column
artifact, not real drift — so paying its `O(n_total)` BTRAN-based
`fresh_d_into` every `XB_CHECK_INTERVAL` iterations buys nothing.

### 本体: 診断フラグの読み込み

(元の位置: L2745 付近)

`super::update_verify`'s own env-var escape hatch, hoisted outside
the loop for the same reason `super::solve_lp_dual_on` hoists its
own copy: a single `bool` branch per pivot, not an `env::var` call.
Hoisted for the same reason: `ENOMOTO_DEBUG_D_DRIFT_EXT` used to be
looked up with `std::env::var` (environment lock + linear scan +
`String` allocation) on *every* pivot just to guard a debug print.
`ENOMOTO_PROF_PHASES_EXT` — this module's own counterpart to
`simplex::solve_lp_dual`'s `ENOMOTO_PROF_PHASES` (see [`prof_phases`]'s
own docs). Hoisted here for the same reason: a single `bool` branch
per phase per iteration, not an `env::var` call. Only covers this
function's own main loop, not `polish_with_true_bounds`'s separate
(and typically far shorter) cleanup loop.
`ENOMOTO_PROF_PHASES_EXT_WORK=1` (on top of `ENOMOTO_PROF_PHASES_EXT`)
adds the `work/iter` volume counters (`rho_p`/`alpha`/`tau` nonzeros,
PRICE entries, touched columns, chuzc1 candidates). Separate because
gathering them costs `O(m + PRICE entries)` per iteration outside
every phase timer — enough to distort the report's own wall total.

### 本体: delta0 診断・Devex 切り替え(不採用)・DSE 再計算(既定オフ)

(元の位置: L2770 付近)

Diagnostic only (`ENOMOTO_DEBUG_EXT_DELTA0`): the paper's own
remark (\S4.5's absorbing-boundary result, `prop:no-return`) says
that once every M-flagged structural column is off its `M` side —
basic, or nonbasic at its genuinely finite side — it never returns,
and from that iteration on this loop's own decisions coincide
exactly with the classical bounded method's. `m_flagged_cols` is
fixed for the whole solve (`delta` never changes); a column counts
as "still on the M side" iff it's nonbasic (`nb_status[j].is_some()`)
at the side whose `ColCache` value has nonzero slope — see
`hat_lower`/`hat_upper`'s own docs for why the finite side is always
slope `0`. Answers "how many of this loop's own iterations run
*after* delta=0, where the paper says nothing more distinguishes
this phase from classical dual simplex" — a question the loop
itself never otherwise answers, since reaching delta=0 mid-loop
isn't a branch this code currently acts on (see `finish`'s own docs
on `cleanup_pivots`/`polish_with_true_bounds` firing only when
delta=0 is reached by loop *exit*, not mid-loop).

**Tried and reverted: switching `weights` from exact `Dse` to cheap
`Devex` right at this point** — at delta=0 the current basis is
dual-feasible for the *true* problem with every nonbasic column at a
genuinely finite bound, precisely the state `solve_lp_dual_on` itself
starts every classical solve from in cheap Devex mode (see
`DevexState`'s own docs), so it looked like a legitimate way to stop
paying DSE's extra per-pivot FTRAN (`tau`) for whatever remains of
the solve. Implemented in full, including porting `solve_lp_dual_on`'s
own two escalation-back-to-Dse triggers (ill-conditioned single pivot,
rolling stagnation window) as a safety net. First attempt (seeding
Devex's weights from the live, exact `DseState.w` at the switch point
— reasoned to be strictly *more* accurate than Devex's own trivial
`1.0` start) produced a real, reproducible false `Infeasible` on
Netlib `pilot4` — root-caused to `DevexState::update_after_pivot`'s
pivot-row line hard-flooring at `1.0`, a floor calibrated for
`DevexState::new`'s own "every row starts at exactly `1.0`"
convention and not for raw DSE-scale values (see that constructor's
own docs for the full mechanism). Fixed by seeding fresh at `1.0`
instead (correctness restored, confirmed via the full 73-problem
sweep) — but even fixed, this was a clear net *regression*: Netlib
`fit1p`'s own main-loop iteration count alone rose from `4412`
(exact Dse throughout) to `17270` after switching to fresh Devex at
its own delta=0 point (reached at iteration `1176`), and the full
73-problem total rose from ~7.6s to ~10.4s. The reason `solve_lp_dual_on`
benefits from starting Devex cheap is specifically that it starts at
the *trivial* all-slack basis, where a fresh `1.0` approximation
costs nothing to be reasonably accurate; restarting Devex from
scratch at a basis already hundreds or thousands of pivots deep
(typical for a solve that takes this long to reach delta=0 at all)
discards far more exact geometric information (that `Dse` was
already maintaining for free via incremental updates) than the saved
FTRAN was ever going to be worth. Reverted outright rather than left
behind an env-var flag — unlike this module's other reverted
experiments, no configuration of this idea (fresh-seeded or
DSE-seeded) showed any redeeming case worth preserving a toggle for.
Always on since **validated** (`degen3` DSE-drift follow-up, full
73-problem Netlib sweep, two repeats) — `ENOMOTO_DSE_REFRESH_ON_REFACTOR=0`
to disable for A/B comparison. `DseState`'s incremental
`update_after_pivot` is exact only in infinite precision — measured
directly on `degen3` (`ENOMOTO_PROF_PHASES_EXT`'s own `dse_rel_err`
line, still wired up) drifting to >=100% relative error from the
true `||B^-T e_r||^2` on over a quarter of iterations, with only 11%
staying within 1%, and this module's own refactor cadence
(`REFACTOR_COUNT`, typically single digits per solve) never
refreshed the weights the way it already refreshes `x_B`/`d` — a
`refactorize()` already pays for a fresh `lu`, so recomputing exact
weights from it (`DseState::from_basis`, the same `O(m)`-BTRAN cost
as the `fresh_d` resync already done at the same point) is close to
free relative to the refactor itself.

**Measured**: `degen3` alone 7,915 -> 2,626 main-loop iterations
(2.79s -> 0.88s), refactor count 13 -> 4 (better-conditioned pivots
from more accurate weights need fewer drift-triggered refactors too
— not just fewer iterations from better row choices). Full
73-problem sweep: 8.91s -> 5.68s and 5.34s (two repeats, both well
clear of baseline noise), ratio-to-HiGHS 4.10x -> ~2.8x. No status
changes on any problem; the only objective mismatch either run
produces is the pre-existing `cycle` ~3e-4 relative gap
(`netlib-benchmark-workflow`'s own note, unrelated to this change).
Worst single-problem regression: `wood1p` +0.016s. Unlike the
`polish_with_true_bounds` "Exact DSE was tried here" note (a
*different* placement — adding DSE weighting to a phase that had
none — found a net regression elsewhere): this only refreshes
weights an *already-DSE-weighted* loop maintains, at points where a
refactor is happening anyway, so it carries none of that placement's
extra per-pivot FTRAN cost.

**Re-evaluated and defaulted off**: the drift this refresh was
compensating for came from `DseState::update_after_pivot`'s old
`wp_old` self-amplification bug, fixed separately since the
measurement above was taken (see that function's own `wp_old =
||rho_p||^2` docs). With that fixed, `degen3`'s `dse_rel_err`
diagnostic now reports <1% relative error on 100% of iterations
with no refresh at all — the >=100%-drifting case this refresh
exists for no longer occurs, so on the current codebase it is pure
cost: profiling `degen3` (`ENOMOTO_PROF_PHASES_EXT`) attributes 12%
of wall time to `DseState::from_basis` at each refactor (1,412
BTRANs/event on this problem), on an identical 2,163-iteration
pivot path and objective with or without it. Full 77-problem Netlib
sweep with the refresh off: -10.8% total wall time, no status or
objective changes. `ENOMOTO_DSE_REFRESH_ON_REFACTOR=1` re-enables it
for A/B comparison if a future drift regression reappears.

### 本体: Score2 の適応的許容誤差(実験的)

(元の位置: L2889 付近)

EXPERIMENTAL (`ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL`, unset = `1e-9` =
identical to the fixed-tolerance baseline): a column flagged
M (`delta[j] != 0`) only ever leaves its `M` side by being chosen as
the *entering* column `q` — never by a BFRT flip (candidates with
`s_j = 1` are provably excluded from the flip set, `prop:no-return`'s
own proof, part (ii)) — so `resolved_m`/`remaining_m_side` need only
watch `q`, not rescan every nonbasic column every iteration the way
`ENOMOTO_DEBUG_EXT_DELTA0`'s own diagnostic does. `remaining_m_side`
reaching `0` is exactly this iteration's delta=0 (see that
diagnostic's own docs for what that means). The *ratio* still
remaining drives `Score2::cmp_lex`'s tie tolerance for the leading
(`c2`, slope-driven) term: loose while many M-flagged columns are
still unresolved (so a row's *true* infeasibility can influence
which row gets fixed next, not just how many M-flagged columns feed
it), tightening back to the exact `1e-9` as `remaining_m_side` hits
`0` — i.e. by the time delta=0 actually arrives, this is bit-for-bit
the original fixed-tolerance comparison, so nothing about the
provably-classical post-delta=0 phase changes.

**Measured, not just theorized (73-problem sweep, `netlib_benchmark_workflow`'s
own methodology)**: unlike a *flat* loosened `c2` tolerance (which
regressed the full-set total outright), this adaptive version can
genuinely cut iterations on more than one pathological instance at
once — at `1e-2`, `fit1p` 4412->3997, `degen3` 7914->7638, `25fv47`
7957->6500 all improved simultaneously (a flat tolerance never
achieved that — always traded one of these off against another).

`1e-2` also used to make `pilot4` return a **wrong** `Infeasible`
(HiGHS: optimal, obj=-2581.14) — since root-caused and fixed at the
pivot-*commit* level (`pivot_grossly_inconsistent`'s own docs, an
`updateVerify` gap letting an effectively-zero-pivot commit right
after a refactor), `pilot4` no longer returns a wrong answer at any
tolerance. It does, however, still cost tens of seconds at `1e-2`:
once that fix correctly discards the pathological (`q=9`, `r=144`)
candidate, nothing stops chuzr/chuzc1 from reselecting the exact
same pair next iteration (its PRICE-computed `a_p[9]` is
recomputed fresh from the same basis and rounds to the same
spurious ~3e-9 every time) — a discard-and-retry loop that only
terminates via `MAX_ITERS`, not via any actual escape.

`stall_shrink` below exists to break exactly that loop: it tracks
how many iterations have passed since an M-flagged column last left
its `M` side (`iters_since_m_progress`) and multiplies the fraction-
based tolerance above by a factor that decays from `1.0` toward `0`
the longer that stretch runs — i.e. a stall in M-side progress pulls
`score2_c2_tol` back toward the exact, proven-safe `1e-9` regardless
of how many M-flagged columns nominally remain, since a
pathologically-repeating candidate is exactly the kind of "no real
progress" this loop can otherwise never detect on its own (unlike
`stall_count`/`bland_mode` below, which watches *objective*
progress, not M-side progress specifically — the two can diverge,
as `pilot4` demonstrates: plenty of ordinary, non-M pivots keep
firing while the *same* M-flagged row goes nowhere).

**Validated (73-problem sweep, three repeats each)**: at
`ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL=1e-2` with the default
`ENOMOTO_SCORE2_STALL_HALFLIFE=50`, `pilot4` collapsed from the
~34s `MAX_ITERS` exhaustion described above back down to 803
iterations / 0.09s — *and* every one of the 73 problems stayed
correct (only the pre-existing `cycle` objective mismatch, same as
the fixed-tolerance baseline). The full-set total landed at
7.76-7.91s across three runs versus a same-binary baseline (Score2
disabled) of 7.52-7.72s — a small, roughly 3% aggregate regression,
inside this machine's own noise band but consistently on the wrong
side of it, so still not a demonstrated net win.

The mechanism is also more parameter-sensitive than it looks:
`ENOMOTO_SCORE2_STALL_HALFLIFE=20` (a *shorter*, seemingly-safer
half-life — shrinking toward `1e-9` faster) reintroduced `pilot4`'s
~34s `MAX_ITERS` exhaustion instead of fixing it, and `10` made it
worse still (73-set total 41-54s). `iters_since_m_progress` resets
on *any* M-flagged column resolving, not specifically the one stuck
in a repeating candidate — with a short half-life, some *other*
M-flagged column resolving elsewhere in the problem can hand the
stuck row a fresh grace period before its own tolerance ever
shrinks enough to change chuzr's row selection away from it, so a
shorter half-life can paradoxically make the escape *less* reliable,
not more. `50` is the only value swept that reliably escaped it.

Left wired up (default: exactly the original fixed `1e-9`, zero
behavior change) as a documented, correctness-validated research
direction, not a tuned default: it needs either a smarter stall
signal (specific to the *stuck row/candidate*, not global M
progress) or a broader sweep across more pathological instances
before its own `50`-iteration half-life could be trusted as
anything more than "the one value that happened to work here".

### 本体: best_remaining_m_side(プラトー検出の補助指標)

(元の位置: L2983 付近)

Companion best-so-far for the infeasible-row-count plateau check
below: `remaining_m_side` only ever decreases (its one mutation
site is a plain `-= 1`, never incremented), so on a healthy solve
it is a strictly more reliable progress signal than
`infeasible_rows.rows.len()` — which measures how large the
(constantly churning) infeasible-row *set* is right now, not
whether the M-side resolution actually driving the solve forward
is stuck. Netlib `dfl001` (`analysis/dfl001_20260921_035239.md`)
is a healthy solve the plateau check otherwise mistook for
`pilot4`'s genuine stall: its infeasible-row count's own minimum
goes unbeaten for 5,000+ consecutive iterations (the set keeps
churning — 40% grow / 50% shrink — without ever posting a new
low) while `remaining_m_side` falls steadily throughout (11045 ->
4762 over the same span), which is what this tracks.

### 本体: stuck_row ブースト(実験的・効果なし)

(元の位置: L3015 付近)

EXPERIMENTAL, **confirmed not to work** (`ENOMOTO_STUCK_ROW_BOOST_FACTOR`,
unset/`1.0` = no-op, zero behavior change either way): the idea —
rather than loosening the tie tolerance globally based on overall
M-side progress (`score2_max_tol`/`stall_shrink` above, which a
stall specific to one row can miss) — was to track the *specific*
row that `pivot_grossly_inconsistent`/`updateVerify` has just
discarded a pivot for, consecutively, and once that streak crosses
`ENOMOTO_STUCK_ROW_BOOST_THRESHOLD` (default `3`), score *that row
only* in chuzr as if its own deviation's `M`-dependence were scaled
by `ENOMOTO_STUCK_ROW_BOOST_FACTOR` (`2.0` tried, i.e. literally
substituting `2M` for `M` in `dev`'s own `base + slope*M` — only the
`slope` term changes), nudging chuzr's ranking of this one row
relative to every other currently-infeasible row.

**Tested directly on the exact `pilot4` stall this was built for
(`ENOMOTO_SCORE2_ADAPTIVE_MAX_TOL=1e-2` with `stall_shrink`
effectively disabled via a huge `ENOMOTO_SCORE2_STALL_HALFLIFE`, to
isolate this mechanism alone): factor `2.0` (boost) and `0.1`
(dampen) both still hit `MAX_ITERS` (~34s) — neither direction
escaped the loop.** The reason, on reflection, is structural: chuzr
(which this mechanism biases) only decides *which infeasible row*
gets worked on next; the actual failure lives one level down, in
chuzc1/BFRT's choice of *entering column* for whichever row wins —
`pilot4`'s own stuck candidate (`q=9`) is picked by ratio-test
ordering and Harris pass 2 (largest pivot magnitude in a flat ratio
window), neither of which reads `delta`, width, or anything this
mechanism touches. Re-scoring the row differently in chuzr does not
change what chuzc1 does once that row is selected, so — assuming
row 144 keeps winning chuzr regardless (plausible if it is the only,
or persistently the most attractive, infeasible row available) —
the exact same doomed candidate gets re-picked every time either
way. `stall_shrink` (above) worked instead precisely because it
changes the *cross-row* comparison outcome directly (which row wins
chuzr), not because it does anything smarter within a row.

Left wired up, still fully inert by default, as a documented dead
end: a real per-candidate fix would need to act at the chuzc1/BFRT
level (e.g. excluding a specifically-discarded `(q, r)` pair from
re-selection) rather than reweighting chuzr's own row scores.

### 本体: 破棄された候補の禁止(常時オン)

(元の位置: L3063 付近)

Always on (formerly `ENOMOTO_BAN_DISCARDED_CANDIDATES`-gated and
off by default): unlike `stuck_row`/`stuck_row_streak` above (which
reweight chuzr's *row* ranking and, per that mechanism's own docs,
were confirmed *not* to change chuzc1's candidate choice within a
row), this acts directly on chuzc1 itself. The moment
`pivot_grossly_inconsistent`/`updateVerify` discards a pivot, `q` is
remembered as banned *for row `discard_row` specifically* — chuzc1's
own candidate-building loop (below) then skips it next time that
same row is selected, forcing the ratio test/Harris pass 2 to pick
among the *remaining* candidates instead of reproducing the
identical (and already-proven-untrustworthy) choice. Scoped to one
row at a time (cleared the moment a *different* row's pivot is
discarded, or `discard_row`'s own pivot finally commits, both
below): a column discarded for row `r` is not assumed bad for any
other row, and a ban is not assumed permanent once `r` itself moves
on. No streak/threshold here (unlike `stuck_row_boost`) — a single
discard is itself direct evidence this exact `(q, r)` pair is
unusable from the current basis, not merely inconvenient.

**Validated as the fix that actually works, twice over.** First
(while still experimental) isolated from `stall_shrink` via a huge
`ENOMOTO_SCORE2_STALL_HALFLIFE`, on the `pilot4` stall both
`stall_shrink` and `stuck_row_boost` were built for: it settled in
707 main-loop iterations / 0.07s — matching the unmodified
baseline's own 705 almost exactly, versus the ~34s `MAX_ITERS`
exhaustion every other mechanism left behind — confirming the
failure was never about *which row* chuzr picks, but chuzc1
mechanically re-deriving the identical doomed candidate from
unchanged inputs every retry. At the time, a full 73-problem sweep
found it statistically inert on its own (7.48-7.80s across several
configurations, all inside one noise band) *because* nothing in the
then-current default configuration ever loosened chuzr's tie-break
enough to land on a genuinely untrustworthy `(q, r)` pair in the
first place — so it stayed opt-in.

Promoted to always-on once restricting `M`-bounding to `S` (§4.2,
this module's own top-of-file docs) changed that: with only the
columns whose cost sign actually forces infinite-side placement now
`M`-tracked, `pilot4` reaches exactly this landmine *by default* —
`infeasible_rows.rows.len()` frozen at `31` for 16,000+ consecutive
iterations post-`delta=0` (`ENOMOTO_DEBUG_EXT_TRACE`'s own trace),
chuzc1 re-selecting the same discarded `(q, r)` every single time —
until `MAX_ITERS` exhausts and the whole solve falls back to the
documented-fragile classical `BIG_M` path (55s+, on a problem this
module otherwise solves in under 0.1s). Enabling this unconditionally
fixes it outright (back to ~700-800 iterations) and, per the full
73-problem sweep re-run under the `S`-restricted default (comparing
this flag on vs. off with `S` unchanged), reproduces the original
"statistically inert elsewhere" finding: every other problem's timing
stayed within normal run-to-run noise.

### 本体: chuzr 候補短縮リスト S11(実験的)

(元の位置: L3117 付近)

S11 (EXPERIMENTAL, path-changing, default off): hyper-sparse chuzr
short list in the spirit of HiGHS `HEkkDualRHS::chooseHyperSparse`.
A full pool scan also records the `K` best rows (`sl_rows`) and the
`(K+1)`-th best score (`sl_cut`). Every row whose deviation or DSE
weight changes afterwards (the `x_B` update's row list, plus `r`) is
appended to `sl_rows`; every other pool row still scores at most
`sl_cut`. So while no full resync intervenes, the best of `sl_rows`
is the pool's best whenever it beats `sl_cut` (`cmp_lex` `Greater`);
otherwise, or once the list outgrows `4K + 64` rows, a full scan runs.
Not bit-identical: `Score2::cmp_lex`'s `1e-9` tie tolerance is not
transitive, so the scan order can change which of two near-tied rows
wins. `ENOMOTO_T_CHUZR_SHORTLIST=K` (e.g. `8`) enables it; `0` = off.

### 本体ループ: chuzr(DSE 重み付き離基行選択)と Score2 許容誤差の補間

(元の位置: L3198 付近)

(a')(b'): primal feasibility check + leaving-row selection —
Devex/DSE-weighted (paper \S4.5's generalization, [`Score2`]'s
own docs), not plain Dantzig: `super::solve_lp_dual_on`'s own
history names Netlib `cycle` directly as an instance where pure
largest-deviation selection (even with cost perturbation active)
converges to a slightly-off vertex instead of the true optimum,
which this weighting exists to fix. Scanned over
`infeasible_rows.rows` only, not `0..m` — `x_B(M)` itself is now
maintained incrementally (every write site below calls
`infeasible_rows.set`), not recomputed from scratch here the way
it used to be, so this scan visits only the rows that can
possibly win regardless.
Linear interpolation from `score2_max_tol` (all M-flagged columns
still unresolved, `remaining_m_side == n_m_flagged`) down to the
exact `1e-9` (`remaining_m_side == 0`, i.e. delta=0 for this
iteration onward) — see `score2_max_tol`'s own docs — then pulled
further toward `1e-9` by `stall_shrink` (`iters_since_m_progress`'s
own docs) whenever M-side progress has stalled, regardless of how
much of `remaining_m_side` is nominally left. `.max(1e-9)` guards
a `score2_max_tol` set below `1e-9` from ever tightening the
comparison beyond the original baseline.

### 本体ループ: 行方向 PRICE(基底列を含める理由の歴史)

(元の位置: L3501 付近)

Row-major sparse PRICE (Huangfu & Hall §2.2.2's "spmv",
`super::solve_lp_dual_on`'s own PRICE step): `a_p = rho^T A`,
computed by walking only `rho`'s *nonzero* rows and scanning
each one's own sparse row of `A` — not, as this function's own
first version did, a per-column dot product over *every*
nonbasic column regardless of whether `rho` even touches the
rows that column lives in. This is also what makes the
incremental `d` update below possible at all: `a_p` is exactly
the row this iteration's entering column's own reduced-cost
drop is measured against.
Only fixed columns are excluded here, matching the classical
method's own PRICE filter — a *basic* column is deliberately
NOT skipped: the column currently basic at the leaving row
needs its own `a_p` entry too (its incremental `d` update below
is what gives it a correct reduced cost the moment it becomes
nonbasic this same iteration — omitting it here left `d[j]` for
a former basic column stale, corrupting a *later* iteration's
candidate ratio the first time that column became eligible
again; confirmed as a real, not merely theoretical, bug: it
broke the objective on the large majority of real Netlib
instances, ordinary bounded ones included, once this module's
own M-bounding logic even fires for a single unbounded-above
column — routine for a plain `x_j >= 0` MPS column with no
explicit upper bound).
S9 (`price_by_column`, default off): with a dense `rho` the
row-wise scatter visits most of `A` anyway, so gather instead —
one dot product `rho^T A_j` per non-fixed (and, under the
default partition, nonbasic) column over `std.cols`, no
`touched` bookkeeping per entry (HiGHS `priceByColumn`, used
above density 0.1). Summation order changes from row order to
column order, so `a_p` rounding and hence the path can differ.

### 本体ループ: chuzc1 の分岐なし候補フィルタ

(元の位置: L3622 付近)

Branch-free candidate filter: every touched column is
written into the next slot and the slot is kept (`k +=
keep`) only when it passes the same three tests the old
`continue`-based loop applied (nonbasic, `|alpha_j| > TOL`
in its original negated-`<=` form so a NaN is treated
identically, and the ratio-test sign condition). Those tests
are close to coin flips per column (a third of touched
columns are basic, about half fail the sign test), so the
branchy form paid a misprediction on most columns; the
arithmetic done for a rejected column is discarded. Kept
candidates carry exactly the `hat_alpha`/`ratio` the old loop
computed, in the same `touched_cols` order.
`cand_scratch` only ever grows (never cleared), so this is a
no-op after the first few iterations instead of an `O(k)`
fill every pivot.

### 本体ループ: 停止候補による刈り込み

(元の位置: L3677 付近)

Smallest (in `Cand`'s own `(ratio, j)` order) candidate whose
width is a genuine infinity (`cache.width[j] == None`, a
one-sided row's slack): the BFRT walk below stops
unconditionally on reaching such a candidate (both the heap
and the `bland_mode` walk visit in this same order), and pass
2 / the flip loop only ever index `[0, k_star]`, so no
candidate ordered after it can influence anything. Dropping
those leaves the chosen `q`, the flip set and every
floating-point value exactly as before, while shrinking the
pool the heap has to be built from — typically from ~1000
candidates to a few dozen on the heavy Netlib instances.
(HiGHS's `HEkkDualRow::choosePossible` prunes its own
candidate pack by a ratio bound for the same reason.)

### 本体ループ: Eligible 空での noise_feasible 再挑戦

(元の位置: L3707 付近)

`super::solve_lp_dual_on`'s own `noise_feasible` second
chance, ported (see that mechanism's own docs, and
`noise_feasible`'s own declaration above): before concluding
Proposition 4.6's genuine "Eligible = empty" case, check
whether `r`'s own deviation is actually within this row's
rounding noise rather than a real violation. Only ever
applies when `w_r.slope == 0` (within `Affine1::cmp_lex`'s
own `1e-9` floor) — a nonzero slope means the deviation is
genuinely `M`-scaled (unboundedly large in the limit this
module reasons about), which can never be noise regardless
of its `base` term, so only the plain-`f64`-equivalent case
gets this classical-style reprieve. Scaled by this row's own
RHS magnitude (`std.b[r]`), not a bare `PRIMAL_FEAS_TOL`,
for the identical reason `PRIMAL_FEAS_TOL`'s own doc comment
gives: a large-magnitude row's rounding floor scales with
it, so a fixed absolute bar is simultaneously too strict on
large rows and too loose on tiny ones.

### 本体ループ: 更新済み分解からは Infeasible を結論しない

(元の位置: L3730 付近)

Never conclude infeasibility from an *updated* basis
factorization. `Affine1::cmp_lex`'s own `base` comparison
is absolute at `1e-9` (its `base_scale` floor of `1.0`),
while this loop knowingly tolerates `x_B` drift up to
`XB_DRIFT_TOL_MAX` (`1e-4`) between refactorizations — five
orders of magnitude of slack in which accumulated
Forrest-Tomlin error alone can make a perfectly feasible
row look like a violated one. Measured on Netlib's
`greenbea` (the instance this guard was written for): at
`update_count = 47` the deviation read `base = 3.0e-7`
against a flip capacity of exactly `0`, so the walk below
ran out of candidates and reported `Infeasible`; refactorized
at that same basis it reads exactly `-0.0` — i.e. the
capacity covers it precisely and the iteration has a pivot.
So: redo the iteration from a fresh factorization, and only
report `Infeasible` when it still holds there (the same
refactor-and-resync the `update_verify` discard path below
performs). Terminating: `refactorize` leaves
`update_count() == 0`, so a second visit with no committed
pivot in between falls straight through to the report.

### 本体ループ: chuzc1 のヒープ化(遅延ソート)

(元の位置: L3810 付近)

chuzc1 + BFRT pass 1, combined: `bland_mode` still needs the
*whole* pool sorted by `j` (its walk has no early-exit shortcut
to lean on), but the common case's walk almost always stops
after just the first one or two candidates in ascending-`(ratio,
j)` order (measured on Netlib `greenbea`: ~521 candidates/
iteration built, but the walk below consumes ~0.5 on average) —
a full `sort_by` pays `O(k log k)` to answer a question the walk
only needed the first `w << k` of. A min-heap
(`BinaryHeap<Reverse<Cand>>`, `O(k)` to build) popped one at a
time reproduces the exact same ascending order (`Cand`'s `Ord`
impl, same formula the old `sort_by` used) but costs `O(w log
k)` instead — `crate::simplex`'s own `ChuzcCand`/`BinaryHeap`
already does this for the classical path; this mirrors it here.
`candidates.drain(..)` (not `.into_iter()`) so `candidates`
itself keeps its allocation for the next iteration's PRICE scan
to reuse, matching this function's own preallocate-once
convention for its other per-iteration buffers.

### 本体ループ: bland_mode でも (ratio, j) 順に並べる理由

(元の位置: L3845 付近)

Same ascending `(ratio, j)` order the non-`bland` heap
path below produces (`Cand`'s own `Ord` impl) -- *not* a
plain `j` sort. `j` still breaks ties deterministically
(Bland's rule's actual anti-cycling guarantee), but the
BFRT walk just below fundamentally requires ratio-
ascending order to be a ratio test at all: sorting by `j`
alone let it walk straight past cheap, well-conditioned
candidates into whichever tiny/ill-conditioned pivot
happened to sit at a small column index, confirmed as
a contributing mechanism behind Netlib `greenbea`'s
singular-basis failure once `bland_mode` latched.

### 本体ループ: BFRT 使い切り時の Infeasible ガード

(元の位置: L3913 付近)

Same guard as the `Eligible = empty` site above, for the
same reason (see its own docs): an `Infeasible` conclusion
drawn at `1e-9` from a factorization this loop lets drift to
`1e-4` is not a conclusion. This is the site `greenbea`
actually reached (`site=bfrt_exhausted`, iteration 4715).

### 本体ループ: Harris 型パス 2

(元の位置: L3961 付近)

Harris-style pass 2 (`HARRIS_RATIO_TOL`'s own docs,
`super::solve_lp_dual_on`'s own BFRT): search *backward* from
`k_star` (never forward -- see that constant's own docs for why,
confirmed the hard way there) within a flat ratio-space window
for the candidate with the largest pivot magnitude, and pivot on
that one instead -- every candidate strictly before it still gets
flipped (pass 1 already proved that flipping any prefix up to
and including `k_star` never overshoots `w_r`, so a *smaller*
prefix cannot either). Confirmed empirically necessary, not
merely by analogy: without it, this phase converges to a
measurably wrong (not just imprecise) objective on Netlib
`cycle` even with Devex/DSE weighting and cost perturbation both
already active -- this crate's own `chuzc1`/BFRT history names
`cycle` directly as the reason this exact refinement exists at
all, and as the instance that broke two independently-tried,
more "faithful" alternatives (a per-row pivot-scaled window, and
a port of HiGHS's own `chooseFinalLargeAlpha`) -- this module
deliberately reuses the same flat, narrow window that one, not
either reverted alternative.

### 本体ループ: BFRT 結合フリップ

(元の位置: L4014 付近)

BFRT combined-flip (`super::solve_lp_dual_on`'s own "apply every
flip's combined effect on the basic variables in a single extra
FTRAN" — its own docs): every candidate about to be flipped
contributes its own bound jump (`width_affine`, signed by which
way it's flipping) to one summed sparse right-hand side *per
`Affine1` channel*, solved once each (dense-or-sparse dispatched
by `lu.should_use_dense_solve`, the same choice `finish`'s own
cleanup-lemma FTRAN uses) rather than one FTRAN per flipped
column. Reads `nb_status[cand.j]` while it's still pre-flip, so
this must run *before* the flip-commit loop just below. Its
effect on `x_b_base`/`x_b_slope` is applied after the entering
column's FTRAN (`combined_pending`): by default inside the
entering step's own `x_B` update loop (`merge_flip_xb`), whose
`theta` reads row `r`'s post-flip value, so every flip this same
iteration made is reflected exactly as if applied first.

### 本体ループ: 入る列の FTRAN

(元の位置: L4194 付近)

`alpha_full = B^-1 A_q` (FTRAN of the entering column, against
the still-pre-pivot `lu`) — needed by the weight update below,
this phase's own incremental `x_B(M)` step, and `updateVerify`
just below, so it is computed here rather than alongside
`dense_q` further down. `alpha_q` above and `alpha_full[r]` are
the *same* pivot element via two independent routes (BTRAN+dot
vs FTRAN — `e_r^T B^-1 A_q = (B^-T e_r)^T A_q`) — expected to
agree exactly in infinite precision, which is exactly what
`updateVerify` below checks rather than merely assumes.
`dense_q` (the raw column) is built on the dense branch only: it is
this FTRAN's own rhs, exactly like `super::solve_lp_dual_on`'s
own `a_enter_buf` (`try_update_precomputed` further down no
longer needs the raw column itself — see its own docs — only the
`a_tilde_buf` this same FTRAN captures below). The FTRAN itself
takes the same dense-or-sparse fork that function's own entering-column solve
does (`FtLu::should_use_dense_solve`, keyed off the column's own
nonzero count via `std.cols.col(q)`), instead of always
densifying through `solve_into`: a real LP's constraint columns
are themselves sparse, and this loop already makes the identical
choice for the BFRT combined-flip solve just above — leaving the
entering column as the one FTRAN in this loop still forced dense
was a straight port gap from the classical method, not a
deliberate simplification.
Both branches also capture `a_tilde_buf` (the post-L/R, pre-U
intermediate) for this iteration's `try_update_precomputed` call
further down — see that method's own docs.

### 本体ループ: updateVerify と pivot_grossly_inconsistent

(元の位置: L4388 付近)

updateVerify (`super::update_verify`'s own docs, HiGHS
`HEkkDualRow::updateVerify` equivalent): cross-checks this
pivot's element between PRICE's row-direction value (`alpha_q`,
already sitting in `a_p[q]`) and FTRAN's own column-direction
value (`alpha_full[r]`, just computed above) — placed here,
before anything derived from `alpha_full` is committed (the
incremental `x_B(M)` step, the pivot commit, the dual update),
so a numerically drifted pivot is caught at the earliest
possible point, exactly like the classical method's own
placement. On failure this pivot is discarded outright:
refactorize, fully resync `x_B(M)`/`InfeasibleRows` against the
rebuilt factorization, and let the next pass re-run chuzr/chuzc
fresh. The BFRT flips already committed to `nb_status` this same
iteration (if any) are *not* rolled back — `super::solve_lp_dual_on`'s
own reasoning applies unchanged: those are independent,
already-valid degenerate steps, and the resync below recomputes
`x_B(M)` fresh against the (already-flipped) `nb_status` anyway,
so their effect is captured correctly regardless.

The `lu.update_count() > 0` gate below (unchanged from the
classical method's own copy) exists because right after a
refactorization, refusing a pivot `update_verify`'s *tight*
(`UPDATE_VERIFY_TOL`, `1e-7`) tolerance rejects just reproduces
the identical chuzr/chuzc choice next iteration — measured on
Netlib `perold` as 19,860 verify-fail refactorizations in 20,000
iterations before that gate existed. Removing the gate outright
was tried and reverted: Netlib `bnl1` has its own persistent
pivot whose two values agree to `~2.4e-7` relative — comfortably
real and well-conditioned (magnitude `~3.3e-3`, nowhere near
`FT_MIN_PIVOT`), just barely over the *tight* tolerance — and
unconditionally re-checking it every iteration reproduced
`perold`'s exact old pathology on `bnl1` instead (22s vs `bnl1`'s
normal ~0.03s, confirmed via a full 73-problem sweep).

`pivot_grossly_inconsistent` below is a *second*, much looser
check that fires regardless of `update_count`, catching only the
qualitatively different failure this module's own history
actually needs guarding against: Netlib `pilot4`
(`ENOMOTO_DEBUG_EXT_DUAL_CHECK`, under an experimental chuzr
change) hit a candidate right after a refactor where PRICE's
row-sum rounded to `alpha_q ~ 3.4e-9` while FTRAN's own value
came out `alpha_full[r] ~ 1e-21` — not "these two barely
disagree" (`bnl1`) or "these two agree well within tolerance"
(`perold`), but *aren't even the same order of magnitude*: one
is effectively exact zero, the other is noise mistaken for a
real value. `D_GROSS_MISMATCH_REL_TOL` sits far above
`UPDATE_VERIFY_TOL` specifically so it never fires on `bnl1`'s
ordinary `~2.4e-7` disagreement, confirmed via the same
full-suite sweep (identical result to the original gated
behavior on all 73 problems). Committing `basis[r] = q` on a
grossly-mismatched pivot leaves the basis matrix itself
singular, after which every subsequent `x_B(M)`/`d`
recomputation (fresh *or* incremental) is computed from a
singular `B` and is garbage regardless — this is why catching it
here, before commit, is the only point that can actually help; a
periodic drift check ([`D_DRIFT_TOL`]) or a per-column dual-side
re-verification, both tried first, cannot: both would recompute
from the same already-singular basis and simply agree with the
garbage more precisely.
Deliberately *not* `.max(FT_MIN_PIVOT)` the way
`super::update_verify`'s own `scale` is: flooring at
`FT_MIN_PIVOT` is exactly right for a tight, `~1e-7`-scale
relative tolerance (it keeps two values that both happen to be
smaller than `FT_MIN_PIVOT` from registering a huge *relative*
difference over a *tiny* *absolute* one), but it defeats this
check's whole purpose at `D_GROSS_MISMATCH_REL_TOL`'s much
looser scale: `pilot4`'s own `alpha_q ~ 3.4e-9` vs
`alpha_full[r] ~ 1e-21` divided by `FT_MIN_PIVOT` (`1e-7`) comes
out as only `~0.03` — comfortably under `0.5` — even though the
two values don't share an order of magnitude with *each other*.
A tiny fixed floor here only guards the literal `0/0` case.

### 本体ループ: M 側解消の記録

(元の位置: L4527 付近)

`q` is the only way an M-flagged column ever leaves its `M` side
(`score2_c2_tol`'s own docs) — mark it resolved exactly once,
here (only once this pivot is confirmed to actually commit, not
at `q`'s own selection above — a pivot the guard just above
discarded never happened, and `iters_since_m_progress` below
specifically needs to *not* reset on a discarded, no-progress
iteration to do its job). A plain index into `delta` rather than
a lookup in `m_flagged_cols` since `q < n_orig` is guaranteed
whenever `delta[q].is_flagged()` (slack columns are never
M-flagged, `delta_of`'s own docs). `q` becoming basic resolves a
genuinely free (`MSide::Both`) column exactly like a one-sided
one — it stops being nonbasic-at-either-M-side at all, regardless
of which side it left from.

### 本体ループ: 入る列による x_B(M) の増分更新

(元の位置: L4568 付近)

Entering column's own incremental `x_B(M)` step (the other half
of `super::solve_lp_dual_on`'s combined-flip/`alpha`-scaled
primal-step pair, generalized): `theta_q` is how far `q` moves
from its own pre-pivot nonbasic value to reach the leaving row's
target bound, computed from `x_b_base[r]`/`x_b_slope[r]` *after*
the BFRT combined-flip step above already applied this same
iteration's own flips to them. Reads `nb_status[q]` here, before
it's cleared at the pivot commit below. Row `r`'s own new value
is assigned directly (`nb_val_q + theta_q`) rather than trusted
from the `alpha`-loop below, for the same reason
`super::solve_lp_dual_on` does: `alpha_full[r]` lands exactly on
`target` by construction, so the direct assignment is both
simpler and exact.

### 本体ループ: 停滞検出

(元の位置: L4778 付近)

Stall detection (`super::solve_lp_dual_on`'s own convention, its
own docs): only a pivot whose objective contribution is itself
essentially zero counts as "no real progress" — *not* every
iteration unconditionally (an earlier version of this loop did
exactly that, which latched `bland_mode` on for any sufficiently
long-running but otherwise perfectly healthy solve, e.g. Netlib
`degen2`'s ~2900 main-phase iterations, no genuine cycling
involved at all). Now the classical method's own exact check,
`(theta_q * dj_q).abs() < STALL_PROGRESS_EPS`, via
`contribution_base`/`contribution_slope` above — an earlier
version of this check used `dj_q` alone as a proxy, before this
module's own incremental `x_B(M)` step existed to compute a real
`theta_q`; see that computation's own docs for why a nonzero
slope is treated as unambiguous progress rather than folded into
the base-term comparison.

### 本体ループ: 実行不能行数のプラトー検出

(元の位置: L4802 付近)

Secondary anti-cycling signal: a long run of pivots each reporting
*genuine* (non-stalling) objective progress by the check just
above, yet never changing which rows are infeasible at all,
still isn't converging in any way that matters — a long streak
of tiny-but-nonzero contributions that keeps `stall_count` reset
every iteration, so it alone never latches `bland_mode`.
Confirmed necessary on Netlib `pilot4` after restricting `M`-
bounding to `S` (§4.2): `infeasible_rows.rows.len()` sat at
exactly `31` for 16,000+ consecutive iterations post-`delta=0`
(`ENOMOTO_DEBUG_EXT_TRACE`'s own trace), `stall_count` never
exceeding `0`, until this loop's own `MAX_ITERS` budget was
exhausted and the whole solve fell back to the classical
`BIG_M`-substituted path (55s+, versus ~0.03s once this trigger
fires and `bland_mode` breaks the pattern). Deliberately a much
larger threshold than `stall_limit` alone, and reset on *any*
change in the infeasible set's size (not just a decrease) rather
than every iteration unconditionally: a healthy, merely slow
solve's infeasible-row count fluctuates (grows and shrinks) far
more often than this plateau tolerates, which is exactly what
distinguishes it from `pilot4`'s own multi-thousand-iteration
plateau — Netlib `degen2` (this module's own `stall_count` docs
name it directly as a false-positive risk for an eager,
every-iteration stall trigger) never comes close to a static
infeasible count for anywhere near this many consecutive
iterations across its own ~2900-iteration main phase.
Measured against the best (smallest) infeasible count seen so
far, not against the previous iteration's: an exact-equality
test is defeated by a stall that merely *oscillates*. Netlib
`greenbea` does exactly that — past iteration ~8000 its count
alternates between 248 and 249 (two variables trading places on
a single row, `analysis/greenbea_20260921_021218.md` §4), which
resets an equality-based counter every second iteration and let
the solve burn its remaining ~12,000 iterations with
`bland_mode` never latching. "No new best in
`infeasible_plateau_limit` iterations" catches both that and the
literally-static `pilot4` plateau this check was written for,
and is still reset by any genuine progress.

### 本体ループ: remaining_m_side の進展もプラトー判定に数える

(元の位置: L4844 付近)

`remaining_m_side` progress also counts (see its own
`best_remaining_m_side` docs above) — a solve can keep steadily
resolving M-side columns while the infeasible-row *count*'s
minimum sits unbeaten simply because that set is churning
(Netlib `dfl001`), and treating that as a stall latches
`bland_mode` on a solve that was never stuck.

### 本体ループ: PRICE 分割の入れ替え(S12)

(元の位置: L4893 付近)

`HighsSparseMatrix::update`'s own swap scheme: `q` (always
non-fixed — PRICE only ever offers those) moves from each of
its rows' nonbasic part to the basic part, the leaving
column (if non-fixed; a fixed one was never in this matrix)
the other way. Within-row order is irrelevant to `a_p`'s
values: each `a_p[j]` still accumulates over rows `i` in
ascending order, one entry per row.
Positions come from the `cm_pos` index (S12) instead of a
linear search of the row; see its own docs.

### 本体ループ: Forrest-Tomlin 更新と再分解トリガ

(元の位置: L4989 付近)

Forrest-Tomlin incremental update instead of a fresh
refactorization every iteration — `super::solve_lp_dual_on`'s
own trigger scheme: an outright-rejected pivot always
refactorizes; otherwise, every `FT_CHECK_INTERVAL` iterations,
a cheap eta-fill check (`fill_count`) runs, and at the coarser
`FT_CHECK_INTERVAL * RESIDUAL_CHECK_MULTIPLIER` cadence a
genuine `‖A_B x_B - rhs‖` drift check ([`residual_norm`]'s own
docs) against a *freshly recomputed* right-hand side
(`compute_rhs_affine`, paid only when this coarse cadence
actually fires — `x_B(M)` is otherwise maintained incrementally
pivot-to-pivot now, see this function's own docs on why the old
"recomputed fresh every iteration" design was replaced) —
refactorizing only if one of these two actually finds a
problem, *not* unconditionally every `FT_CHECK_INTERVAL` (an
earlier version of this loop did exactly that, needlessly
discarding a healthy Forrest-Tomlin chain most of the time).
Both `Affine1` channels are checked, not just `base`: almost
every comparison in this module (`Affine1::cmp_lex`,
`Score2::cmp_lex`) decides on the *slope* term first, so a
drifted `x_b_slope` is at least as dangerous to correctness as a
drifted `x_b_base` — checking only one channel would leave the
module's single most decision-relevant quantity unguarded.
`try_update_precomputed` wants `a_tilde_buf`/`e_tilde_buf` —
already captured above as a side effect of this same iteration's
own BTRAN (`rho`, for `e_tilde_buf`) and FTRAN (`alpha_full`, for
`a_tilde_buf`); nothing between either capture point and here
writes through them, so no re-derivation is needed — see
`FtLu::try_update_precomputed`'s own docs (this used to be a
plain `try_update(r, &dense_q, ...)`, which recomputed both from
scratch every single pivot).

### 本体ループ: delta = 0 での傾きチャネル残差の省略

(元の位置: L5083 付近)

Paper \S4.6's closing remark, applied to the slope
channel's drift check: an empty `rs` (fresh or incrementally maintained) is
[`compute_rhs_affine`]'s own `delta = 0` signal, and
Lemma 4.1 then makes `x_B(M)`'s slope part exactly
`-B^-1 A_N delta = 0`. The maintained `x_b_slope` holds
that same exact zero — not merely something close to it —
because every later update short-circuits on
`theta_slope == 0.0` and `snap_slope` has already flushed
the resolving pivot's sub-`X_B_SLOPE_NOISE` residue. So
`residual_norm_affine` below is measuring `||A_B*0 - 0||`,
a guaranteed exact `0.0`, over two `O(m)` passes and an
`O(nnz(A_B))` accumulation; it skips the whole channel
and returns that `0.0` directly. `need_refactor` is
therefore bit-for-bit what it was, decided by the base
channel (and `d`'s own check) exactly as before.

### 本体ループ: d のドリフト検査

(元の位置: L5170 付近)

`d`'s own independent drift check ([`D_DRIFT_TOL`]'s own
docs — added after a real false `Infeasible` on Netlib
`pilot4` traced to exactly this gap): only run when the
`x_B(M)` check above didn't already decide to
refactorize, same as that check's own short-circuit
intent — no point paying for a second O(nnz) BTRAN-based
recomputation when a refactor (which calls
`fresh_d_into` on `d` directly, unconditionally) is
about to happen anyway. Gated by [`since_d_drift_check`]'s
own coarser cadence (see its declaration) — the residual
below also excludes fixed columns (`lb == ub`), which
PRICE never updates `d` for, so they'd otherwise swamp the
genuine-drift signal with a constant, non-drift offset.

### 本体ループ: ENOMOTO_D_DRIFT_REFACTOR_ONLY

(元の位置: L5186 付近)

`ENOMOTO_D_DRIFT_REFACTOR_ONLY=1` (S18, A/B, default off)
drops this periodic check altogether and leaves `d`'s
resync to the `fresh_d_into` every refactorization already
does — measured to fire 0 times on all 93 Netlib problems
(`analysis/simplex_loop_20260924_113533.md` §4 S18), so
this only saves its BTRAN + `O(nnz(A))` every
`RESIDUAL_CHECK_MULTIPLIER` checks.

### 関数 finish: lu を引き継ぐ理由

(元の位置: L5291 付近)

`lu` is the main loop's own last-iteration factorization (already
exact for the current basis — the main loop's own termination check
just used it), passed in rather than rebuilt here: this function
runs once per solve, so the saving is small in isolation, but there
is no reason to pay for a `refactorize` this basis already has.

### 関数 finish: cleanup 補題

(元の位置: L5313 付近)

Cleanup lemma (\S4.5 end, `lem:cleanup`): every nonbasic column still
sitting at its own artificial `M` side is moved toward its finite
side by a *primal ratio test* over the true finite bounds of the
basic variables — case (A) no row blocks before the column reaches
its finite side (re-parked there, basis unchanged), case (B) row `r`
blocks first (the column enters, `basis[r]` leaves at the finite
bound it just reached). Unlike the arbitrary-row degenerate pivot
this replaced, every step keeps `x(M)` primal feasible under the
true bounds for all large `M` (lemma (iii)), so exactly `K` steps
leave an `M`-free optimal basic solution and `polish_with_true_bounds`
below finds nothing left to do — it stays only as the numerical
safety net and the true-cost dual check. `ENOMOTO_LEGACY_CLEANUP=1`
restores the old degenerate-pivot cleanup (A/B only).

### 関数 finish: 旧 cleanup の超疎 FTRAN

(元の位置: L5478 付近)

Hyper-sparse FTRAN (`solve_sparse_into`/`GpScratch`, Gilbert-
Peierls): `j`'s own column is genuinely sparse, unlike `x_B`'s
own `rhs_base`/`rhs_slope` (summed contributions from every
nonbasic column, generally *not* sparse) — this is the one
place in this module a hyper-sparse solve has a natural
application without also committing to full incremental `x_B`
maintenance (see this module's own docs).

### 関数 finish: 旧 cleanup の B^-1 A_j = 0 分岐

(元の位置: L5487 付近)

`B^{-1}A_j` is identically zero: `j` never needs to enter
the basis at all (its own module docs, and the paper's own
remark after Lemma 4.9) — its value genuinely doesn't
matter, so it is parked directly at its true finite side.
A genuinely free `j` reaching here (no finite side to park
at all) is not something `presolve::freevar` should ever
leave standing alone: every residual free variable it can't
eliminate outright is documented to appear in at least one
row (see this module's own top-of-file docs), so an
orphaned one touching zero rows is "should be unreachable"
— bail gracefully rather than park it at a side that's
still symbolically `M`.

### 関数 finish: 旧 cleanup の自由変数追い出し

(元の位置: L5515 付近)

The variable cleanup evicts to make room for `j` needs a real
(non-`M`) nonbasic placement of its own — guaranteed to exist
for a one-sided-unbounded column (its own finite side), but not
for a genuinely free one (`MSide::Both`, both sides still
symbolically `M`): this module's termination argument for the
cleanup loop (each pivot strictly shrinks the M-flagged-nonbasic
count) is established for the one-sided case, not for evicting
*another* free variable — rather than risk an unproven
termination argument or an unsound placement, bail to `None`
(reported as `NotSolved`) exactly like this module's
other "should be unreachable" guards.

### 関数 polish_with_true_bounds

(元の位置: L5560 付近)

Step III's own tail: a plain bounded dual simplex operating directly on
`std.lb`/`std.ub`, no `M`/`Affine1` involved anymore — but now using the
same incremental machinery [`solve_lp_dual_extended`]'s own main phase
(and, transitively, `super::solve_lp_dual_on`) already relies on:
hyper-sparse chuzr via [`InfeasibleRows`], incremental `x_B` maintenance
(seeded once, then updated pivot-to-pivot instead of recomputed from
scratch), a Harris-style BFRT pass 2, and `updateVerify` before every
pivot commit. This function used to recompute the full right-hand side
and rescan every row from scratch every single iteration — the one piece
of this module that never received the main phase's own incremental
upgrade, and (per this module's own commit history) the dominant cost on
larger Netlib instances as a result.

Deliberately **not** Devex/DSE-weighted, unlike the main phase or the
classical method: `finish`'s own `z.slope` check already proves the
objective at the basis cleanup hands off is exactly the true final
optimum, and cleanup's own Lemma 4.9 leaves it unchanged — so, staying
dual feasible throughout, every pivot this phase ever takes is
necessarily a zero-objective-contribution (degenerate) one. Devex/DSE
exist to steer toward whichever pivot advances the objective the most
per step; with no such "more attractive" pivot to steer toward here
(every pivot is equally degenerate), the weighting would only add cost
(weight maintenance, an extra FTRAN for DSE's own `tau`) for no
convergence benefit. Anti-cycling is `perturb_costs` plus the Bland
fallback below alone — already confirmed sufficient here, not merely
assumed: this function's own history names `cycle`/`degen2`/`degen3`
directly as instances that reported a false `Infeasible` without it, and
that fix long predates this incremental rewrite (weighting was never
part of it).

A column that still carries a genuine one-sided infinity here (any
nonbasic that never needed cleanup, e.g. a `<=` row's own slack) is
handled exactly like the classical method already does: its width is
simply not finite, so it is never a flip candidate, only ever a direct
pivot target.

### 関数 polish_with_true_bounds: chuzr(DSE を試して不採用)

(元の位置: L5698 付近)

chuzr: plain largest-deviation Dantzig rule, exactly as this
function always used — *not* Devex/DSE-weighted. **Exact DSE
was tried here** (`DseState::from_basis` at entry, `mag^2/w[i]`
scoring, `update_after_pivot` after every pivot, mirroring the
main phase's own scheme) on the theory that DSE's real
justification — steering `chuzr` toward the pivot that reduces
*remaining infeasibility* fastest — is a different claim than
"every pivot here has zero objective contribution" and so isn't
actually addressed by this function's own docs above. **Measured
as a net regression on the full 73-problem Netlib set** (8.53s
baseline -> 9.00s, +5.5%; `stocfor2` alone 0.25s -> 0.36s, +46%)
and reverted — the extra `from_basis` O(m) BTRAN plus one more
FTRAN (`tau`) every pivot costs more than the weighting ever
saves in pivot count on this phase's already-small, already-
degenerate iteration counts. Scanned over `infeasible_rows.rows`
only (hyper-sparse — [`InfeasibleRows`]'s own docs), not `0..m`.

### 関数 polish_with_true_bounds: 真のコストでの双対実行可能性チェック

(元の位置: L5765 付近)

Primal feasible against this phase's own *perturbed* costs
(`active_cost`, set at this function's entry) -- dual
feasibility held throughout by the loop invariant above, so
this point is optimal for the perturbed problem. Whether it
is *also* optimal for the true problem depends on whether
perturbation happened to mask a genuine dual infeasibility:
ported from `super::solve_lp_dual_on`'s own identical check
(that function's own docs) -- this phase never had it, and
the gap is not hypothetical: measured directly on Netlib
`greenbea`, the unperturbed check below fails and this
phase's own perturbed-optimal point is off by 92808 in
objective (0.13%) from the true optimum
(`analysis/greenbea_20260921_030127.md`, mechanism (B)).

### 関数 polish_with_true_bounds: 固定列を双対チェックから除外

(元の位置: L5791 付近)

Fixed columns (`lb == ub`) are left out of this check: no
pivot can ever move them (PRICE above and `run_phase`'s own
`price_one` both skip them, and the main loop's `d`-drift
check excludes them too), so a "wrong-signed" reduced cost
on one is not a dual infeasibility anything could act on —
counting it only sent the whole solve through a primal
handoff that then found nothing to price
(`analysis/simplex_loop_20260924_113533.md` §3.2: 36 of the
48 Netlib handoffs were exactly that). `polish_dual_tol` is
`TOL` unless overridden (`ENOMOTO_POLISH_DUAL_TOL`, A/B).

### 関数 polish_with_true_bounds: 固定列だけの不整合時の早期返却

(元の位置: L5816 付近)

Only a fixed-column (or, with a loosened
`ENOMOTO_POLISH_DUAL_TOL`, sub-tolerance) mismatch
remained, or none at all. In the former case the old
path handed off to `run_phase`, whose first iteration
re-derives `x_B` from scratch (`recompute_basics`) and
then stops with nothing to price — so this returns
exactly that re-derived `x_B`, bit for bit, without the
pricing/BTRAN/steepest-edge setup around it. When the
unfiltered check passes as well, `x` is returned as it
always was.

### 関数 polish_with_true_bounds: S5 ハンドオフフリップ

(元の位置: L5842 付近)

S5 (`ENOMOTO_HANDOFF_FLIP=1`, default off;
`analysis/simplex_loop_20260924_113533.md` §4 S5): when every
true-cost dual infeasibility sits on a column whose opposite
bound is finite, flipping those columns to that bound makes
this basis dual feasible for the *true* costs (`d` itself
does not move — only which sign it needs to have), at the
price of primal infeasibility from the moved nonbasic values.
That is exactly what this dual loop repairs, so it simply
continues on the true reduced costs instead of handing off to
the (per pivot costlier) primal method. Once only: a second
failure — or any infeasibility on a free / one-sided column,
which no flip can fix — still goes to the primal handoff
below.

### 関数 polish_with_true_bounds: 主単体法への引き継ぎ

(元の位置: L5887 付近)

Perturbation masked a genuine dual infeasibility: this basis
is primal feasible (feasibility never depended on costs) but
not dual feasible for the true costs. Finishing from here
needs the primal method's own invariant (primal feasibility
preserved, working toward dual feasibility) instead of this
phase's dual one -- exactly `super::run_phase`'s phase 2,
reused directly rather than reimplemented: every nonbasic
column here already sits at a *finite* side (the cleanup
lemma's own guarantee, `finish`'s docs above), the same
situation every `<=`-row slack is already in in the
classical path this was written for, so it needs no special
handling for this module's own genuinely-infinite bounds.
`expand`/`se` start fresh rather than mid-sequence
(unrelated quantities to this phase's own dual state;
`solve_lp_dual_on`'s own identical handoff confirms this only
costs pricing quality, not correctness).

### 関数 polish_with_true_bounds: 引き継ぎ時の特異基底

(元の位置: L5907 付近)

A singular basis here propagates `None` (reported as
`NotSolved`): there is no from-scratch solver to restart
with, since `Tableau::new` assumes every structural column
has a finite bound, which this module exists specifically to
handle when false.

### 関数 polish_with_true_bounds: noise_feasible の移植

(元の位置: L6002 付近)

`super::solve_lp_dual_on`'s own `noise_feasible` second
chance, ported here now that this function no longer just
re-derives a fresh (and possibly slightly different) `x_B`
next iteration regardless: before concluding genuine
infeasibility, check whether `r`'s own deviation is actually
within this row's rounding noise (scaled by its own RHS
magnitude — `PRIMAL_FEAS_TOL`'s own docs).

### 関数 polish_with_true_bounds: bland_mode の並び順

(元の位置: L6016 付近)

Same order regardless of `bland_mode`: ascending `(ratio, j)`.
`bland_mode` sorting by `j` alone (an earlier version of this
branch) broke the ratio test the BFRT walk just below relies on
-- see the main M-tracked loop's own identical fix above for the
Netlib `greenbea` failure this caused there. `j` already breaks
ties deterministically in the one order, which is all Bland's
rule actually needs.

### 関数 polish_with_true_bounds: 停滞検出

(元の位置: L6191 付近)

Stall detection — now the classical method's own *exact* check
(`(theta_q * dj_q).abs() < STALL_PROGRESS_EPS`), not the
`dj_q`-alone proxy the previous, non-incremental version of this
function needed for lack of a real `theta_q`: the incremental
`x_B` step just above now computes a real one.

### テスト補助 std_form

(元の位置: L6319 付近)

Builds a `StdForm` directly from a dense list of sparse rows (each
row *already* including its own slack term), bypassing
presolve/`build_std_form_presolved` entirely — these tests exercise
[`solve_lp_dual_extended`] in isolation, independent of whatever
presolve does or doesn't eliminate for a given problem shape (see
`simplex.rs`'s own end-to-end `freevar_*`/`had_unbounded_structural_*`
tests for the presolve-integrated path).

### テスト two_free_columns_tied_only_through_opposing_inequality_rows_park_one_at_zero

(元の位置: L6355 付近)

The exact shape this module's own top-of-file docs name as
`presolve::freevar`'s undecidable residual case: two free
columns (x0, x1) that appear only in inequality rows, never an
equality one, so nothing upstream can eliminate or bound either
in isolation. x0 - x1 + s0 = 3 (s0 >= 0, i.e. x0 - x1 <= 3) and
-x0 + x1 + s1 = 3 (s1 >= 0, i.e. x0 - x1 >= -3): min x0 - x1
drives the difference to its lower bound, -3, but x0 and x1
individually stay genuinely unbounded (only their *difference*
is pinned) — the true optimal face is a whole line, so *some*
variable must end up nonbasic with no real bound to rest at. The
main phase resolves one of the two directly, leaving the other
nonbasic at its own `M` side; cleanup's primal ratio test for
that survivor is blocked by nothing (the only row it touches has
the other free column basic there, with no finite bound), so
case (A) parks it at the paper's state `Z` (value `0`,
`rem:state-F`) — this used to bail to `None` (the `BIG_M`
fallback) for lack of that state.

### テスト bfrt_flips_multiple_bounded_candidates_before_the_real_unbounded_pivot

(元の位置: L6502 付近)

x1 in [0,1] cost 1, x2 in [0,1] cost 2, x0 in [0,+inf) cost 100
(the real eventual entering column, deliberately the most
expensive so it sorts *last*) — x1 + x2 + x0 + s = 10, s in
[0,0]. All three start at their own lower bound (every cost is
non-negative, so `crash` places them all there — a uniform
starting side unlike `direct_pivot_resolves_unbounded_column_
into_the_basis`'s own single-candidate example, where `x0`'s
negative cost placed it at `Upper` instead and so never shared
an eligible candidate list with any bounded column at all).
Regression test for the combined-flip incremental `x_B(M)`
update (`COMBINED_FLIP_COUNT`, this module's own docs on why
none of the other hand-built tests in this file are large
enough to exercise it at all): x1 and x2's own finite widths (1
each) are far short of the row's own deviation (10), so both
get fully flipped to their own upper bound (the BFRT walk's
ascending-ratio order: cheapest cost first) before it ever
reaches `x0` as the real pivot. To *minimize* a positive-cost
equality-constrained sum, the cheapest variables should indeed
be maxed out first: x1=1, x2=1, x0 absorbs the rest (10-1-1=8).

## src/params.rs(`mod extended_dual`)

### 定数 XB_CHECK_INTERVAL

How often (in main-loop iterations) the incremental `x_B(M)` drift
check runs, checked every time regardless of `fill_count` — *not*
gated by a further `RESIDUAL_CHECK_MULTIPLIER`-style coarser cadence
the way `super::solve_lp_dual_on`'s own plain-`f64` `x_B` drift check
is. That module's comparisons are plain Dantzig ratios; this one's
(`Affine1::cmp_lex`/`Score2::cmp_lex`) decide almost every comparison
on the *slope* term first, at a much tighter `REL_TOL` (`1e-9`) than
the classical method's own drift tolerance (`FT_RESIDUAL_TOL`,
`1e-4`) ever has to survive. Confirmed load-bearing, not merely
tighter-for-safety's-sake: Netlib `maros`, at the classical method's
own 100-iteration cadence, silently drifted its incrementally-
maintained `x_B(M)` enough (each individual step well inside a loose
tolerance, but accumulating over ~900 pivots) to flip a `cmp_lex`
decision and reach the "no eligible entering column" case on a
problem HiGHS solves — a false `Infeasible`, not a numerical no-op —
before this tighter cadence existed.

**Loosening just this interval (keeping `XB_DRIFT_TOL` itself tight)
was tried and reverted** — a bottleneck-analysis follow-up
(`ENOMOTO_DEBUG_XB_DRIFT_EXT`) found that every drift-triggered
refactor sampled across several slow Netlib instances, `maros`
included, came from the `base` channel alone (`slope` never exceeded
~1e-12, four-plus orders of magnitude below even this tight
tolerance), which made "check less often, same tolerance" look like a
safe lever: `maros` itself stayed optimal all the way out to a
(temporary, env-var-overridden) 100-iteration interval. But a full
73-problem benchmark run at interval `50` told a different story: 5
problems broke — `cycle` and `degen3` regressed to a **false
`Infeasible`** (exactly the failure mode this whole mechanism exists
to prevent, just on different instances than `maros`), and
`perold`/`pilotnov`/`wood1p` blew past a 30s timeout (a degenerate
instance's pivot sequence is sensitive to the exact floating-point
state a resync leaves behind — `25fv47`'s own iteration count swung
non-monotonically between roughly 5,000 and 20,000 across intervals
5–100 during this same sweep, confirming the effect isn't isolated to
the two infeasible cases). `maros` alone passing was not
representative of the other 72 problems' own margins — reverted back
to `FT_CHECK_INTERVAL` (`5`).

### 定数 XB_DRIFT_TOL

The drift tolerance [`XB_CHECK_INTERVAL`]'s own check compares against
— tighter than `super::FT_RESIDUAL_TOL` for the same reason that
constant's own docs give: this module's decisions are sensitive at
`Affine1::cmp_lex`'s `1e-9` `REL_TOL`, so the drift check needs
headroom below that, not `FT_RESIDUAL_TOL`'s much looser `1e-4`.

**History — four single-knob loosening attempts tried and reverted
before landing on the per-solve escalation below:**

1. Loosening [`XB_CHECK_INTERVAL`] (keeping the tolerance itself tight)
   broke `cycle`/`degen3` into false `Infeasible`.
2. Loosening this *absolute* bound itself (`1e-8` -> `1e-7`) fixed
   `maros` but regressed `fit1p` 5.8x on a full 73-problem sweep.
3. A *relative* bound scaled by `‖fresh_base‖`/`‖fresh_slope‖` alone
   (mirroring [`D_DRIFT_TOL`]'s own `scale_d` pattern for `d`), tried at
   two values three orders of magnitude apart, regressed the
   73-problem set either way (~+8-9%) while giving big wins on
   `d2q06c`/`greenbeb`/`pilot` — the `‖x_B(M)‖`-only floor (`1.0`) never
   actually engages on real Netlib instances (every instance measured
   sits far above it), so a single multiplier just applies uniformly to
   everyone.
4. A backward-error-style scale (`‖A_B‖_max * ‖x_B(M)‖ + ‖fresh_rhs‖`,
   the standard LAPACK relative-residual formula for `Ax=b`) measured
   real per-instance scales spanning `~43` (`wood1p`) to `~3.6e6`
   (`maros`) — but `fit1p` (scale `~859`, fragile to *any* loosening per
   attempt 2) sits *above* `wood1p` (scale `~43`, which needs loosening
   just to avoid becoming *stricter* than the old absolute bound). No
   single scale-derived multiplier can loosen `wood1p` without loosening
   `fit1p` by more than its own known-fragile margin — fragility and
   problem-scale don't correlate under any norm tried.

**This version** breaks that correlation requirement entirely: instead
of predicting up front which problems need a looser bound from some
static property, it escalates *within a single solve*, based only on
how many times *this solve's own* drift check has already fired
([`XB_DRIFT_ESCALATION_STEP`]/[`XB_DRIFT_ESCALATION_FACTOR`]/
[`XB_DRIFT_TOL_MAX`]'s own docs). A problem that drift-refactors 0-9
times in its whole solve (measured: `fit1p` 5, `wood1p` 1, `cycle` 0-2,
`degen3` 0, `pilotnov` 6 — every instance any prior attempt broke or
nearly broke) never reaches the first escalation step, so it sees
*zero* behavior change from the unmodified `1e-8` this constant always
was. Only a solve that has already proven itself drift-heavy (`d2q06c`
546, `greenbeb` 236, `pilot` 88-190, `fit2p` 25, all measured at the
flat `1e-8` baseline) earns a progressively looser bound — and since
loosening it also slows the *rate* new drift triggers accumulate, this
is a self-damping control loop, not an open-loop guess: a solve
escalates only as fast as its own residual growth actually demands.

### 定数 XB_DRIFT_ESCALATION_STEP

Every this many drift-triggered refactorizations *within the same
solve*, [`XB_DRIFT_TOL`]'s own effective bound multiplies by
[`XB_DRIFT_ESCALATION_FACTOR`] (capped at [`XB_DRIFT_TOL_MAX`]) — see
that constant's own docs for why this is a per-solve escalation rather
than a static per-problem scale. `10` keeps every instance measured at
single-digit drift-refactor counts (the ones prior attempts broke)
entirely below the first step, while still letting a genuinely
pathological solve (hundreds of triggers at the flat bound) climb
through several steps before this cap's own `1e-4` ceiling.

### 定数 XB_DRIFT_ESCALATION_FACTOR

Multiplier applied per [`XB_DRIFT_ESCALATION_STEP`] drift triggers.
`10` mirrors the *single* absolute-loosening step attempt 2 (this
constant's own docs) already measured in isolation (`1e-8` -> `1e-7`
fixed `maros`, broke `fit1p`) — the escalation ladder repeats that same,
already-characterized step size rather than inventing a new one, but
only after `XB_DRIFT_ESCALATION_STEP` proves the *current* solve is
actually the kind that benefits from it.

### 定数 XB_DRIFT_TOL_MAX

Ceiling on the escalated [`XB_DRIFT_TOL`] — reuses `super::FT_RESIDUAL_TOL`'s
already-proven-safe order of magnitude (the classical method's own
absolute drift bound, `1e-4`) rather than letting escalation grow
unbounded into territory no measurement has ever validated.

### 定数 PIVOT_ESCALATION_STEP

How many *numerically-caused* refactorizations this solve has to take
before its LU pivot threshold is escalated one step
(`sparse_lu::escalate_pivot_threshold`,
`docs/lu_comparison_enomoto_vs_highs.md` §2.4) — **`0`, i.e. the
escalation is off by default**, because it was measured and lost.

HiGHS raises `info_.factor_pivot_threshold` on a numerical failure and
this crate can too, but on NETLIB93 tightening the floor costs far more
in fill-in than it saves in refactorizations: at `10` (the value
[`XB_DRIFT_ESCALATION_STEP`]'s own ladder uses, and the one measured)
the 93-problem total went **+7.4%**, with `pilot87` +29.2% (6.50s ->
8.40s, reproducible across all three runs), `brandy` +67% and
`gfrd-pnc` +12.3% — three problems past the 10%-regression bar on their
own. The escalation fired on 7 of the 10 heaviest problems, because the
`x_B(M)` drift trigger alone reaches 10 on most of them (`pilot` 21,
`dfl001` 24), so it is the *ordinary* heavy solve that gets the `0.5`
floor `sparse_lu::STABILITY`'s own docs already measured as ~4% worse.
Raising this constant until only pathological solves qualify makes it
fire nowhere on NETLIB93 at all, which is not a measurable improvement
either — hence off, rather than retuned.

The mechanism is kept (and reachable via
`ENOMOTO_PIVOT_ESCALATION_STEP`, alongside `ENOMOTO_PIVOT_THRESHOLD`
for the floor itself) so that a future attempt — a smaller step than
`sparse_lu::PIVOT_THRESHOLD_FACTOR`'s doubling, or a trouble signal
narrower than the four below — can be A/B'd without re-plumbing it.

"Numerically caused" means the four triggers that fire because the
factorization stopped agreeing with the basis it stands for — a
rejected Forrest-Tomlin update, `x_B(M)` drift, `d` drift, and a pivot
grossly inconsistent with PRICE. It deliberately excludes the
*cost*-based triggers (eta-bump fill, `ft_max_updates`, the
deterministic CLOCK): those fire on schedule even on a perfectly
conditioned problem, so counting them would escalate `dfl001`'s
hundreds of routine refactorizations into fill-in it has no numerical
reason to pay for.

### 定数 FT_MAX_UPDATES_FACTOR

Trigger (4) for this module — `super::FT_MAX_UPDATES`'s own equivalent
(an unconditional backstop against unbounded Forrest-Tomlin eta-chain
growth, independent of the fill-based trigger (3) above and the
`XB_CHECK_INTERVAL`/`XB_DRIFT_TOL` drift check), which this module never
had at all until now — `super::FT_MAX_UPDATES` itself is only ever read
from `super::solve_lp_dual_on`/`run_phase` (`grep` confirms no reference
here), so a pathological pivot sequence whose eta fill happens to stay
under trigger (3)'s own `FT_BUMP_LIMIT_FACTOR * m` budget indefinitely
(a very sparse basis, or one where each update's own fill stays small)
could accumulate Forrest-Tomlin updates without any hard ceiling.

**Sized adaptively to `m`, not copied as `super::FT_MAX_UPDATES`'s flat
`300`** — a flat value tuned against the classical method's own problem
mix would be wrong here by construction: this module already runs
noticeably longer streaks between refactorizations, on some instances
well past `super::FT_MAX_UPDATES` itself, before this trigger existed at
all. Confirmed by adding [`prof_phases::MAX_UPDATE_STREAK`] (a
`fetch_max` gauge of `lu.update_count()`, paying nothing beyond one
atomic op per pivot) and sweeping essentially every Netlib `.mps` file
available locally — not just the 73-problem, <=3000-variable subset the
rest of this crate's own benchmark methodology otherwise targets, since
calibrating a hard safety ceiling specifically wants the *widest*
available range of `m` and pivot-sequence shapes, including the larger
instances that benchmark excludes. The worst observed ratio
(`peak_streak / m`) was **not** the largest basis in the sweep — it was
`nesm` (`m=662`, peak streak `1090`, ratio `1.647`) — ahead of `scsd6`
(`1.395`), `scsd1` (`1.338`), `adlittle` (`1.732`, but `m=56` is small
enough [`FT_MAX_UPDATES_FLOOR`]'s own floor absorbs it), and `stocfor2`
(`m=2157`, peak streak `1665`, ratio `0.772` — the instance this
constant's very first draft was calibrated against, before the fuller
sweep found worse ratios elsewhere; kept in this history as a reminder
that a handful of hand-picked instances is not a substitute for sweeping
everything available). [`ft_max_updates`] scales with `m`
([`FT_MAX_UPDATES_FACTOR`] `* m`, floored at [`FT_MAX_UPDATES_FLOOR`] so
a tiny basis still gets at least the classical method's own
already-proven `300`) — at `nesm`'s own `m=662` this gives `1986`, a
`1.82x` margin over its own observed peak, comfortably wider than the
`1.21x` a smaller factor (`2.0`) left there. Every other instance in the
sweep has a lower ratio than `nesm`'s, so this margin is the binding one
crate-wide, not merely for one instance. Generous by design — this is
meant to sit as a rarely-firing safety net (mirroring
`super::FT_MAX_UPDATES`'s own documented role once tuned high enough —
see that constant's own docs), not a routine performance lever the way
trigger (3) is; `max_updates_fired=0` across every instance in the same
sweep (see [`prof_phases::REFACTOR_CAUSE_MAX_UPDATES`]) confirms this
trigger changes nothing about the current benchmark's own behavior —
its only job is bounding the *next* pathological instance that shows up.

`3.0` carries real margin above the worst case actually measured, not an
exhaustively swept optimum the way [`super::FT_BUMP_LIMIT_FACTOR`] was —
re-tune (sweeping *at least* as wide a problem set as the survey above,
not just the 73-problem subset) if
[`prof_phases::REFACTOR_CAUSE_MAX_UPDATES`] is ever observed firing
nonzero on a real instance, which would mean either the factor needs
raising further or a genuinely pathological low-fill/long-streak
instance has been found.

### 定数 FT_MAX_UPDATES_FLOOR

Floor for [`ft_max_updates`] — never weaker than the classical method's
own already-proven-safe flat cap, regardless of how small `m` is.

### 定数 SYNTH_CLOCK_FACTOR

Trigger (5): the deterministic, cost-based refactorization trigger
(`analysis/ft_refactor_trigger_20260922_040850.md` §5/§6) — this
module's replacement for that analysis's wall-clock
`ENOMOTO_SYNTH_CLOCK` prototype, using [`sparse_lu::FtLu::synth_tick`]'s
deterministic operation-count accumulator instead of `Instant::now()` so
the same solve always refactorizes at the same iterations (the
analysis's own §6 explicitly calls out replacing the wall-clock stand-in
with exactly this kind of counter before shipping it, precisely to avoid
making refactorization timing — and hence the whole pivot sequence —
depend on machine load/scheduling noise).

Mirrors HiGHS's own `HEkk::updateFactor` (`HEkk.cpp:3075-3090`,
`total_synthetic_tick_ >= build_synthetic_tick_ && update_count >= 50`):
once `update_count` reaches [`SYNTH_CLOCK_MIN_UPDATES`] *and* the
accumulated solve-side tick since the last refactorization reaches
`SYNTH_CLOCK_FACTOR * lu.build_tick()`, this basis is deemed to have
already "paid for" a fresh factorization in the FTRAN/BTRAN work spent
solving against the current (eta-chain-lengthened) one — see this
trigger's own analysis file, §2.3 in particular, for why that FTRAN/BTRAN
unit cost keeps climbing with chain length (the `R`-eta stage's own
gather structure can't skip zeros, so a longer chain means literally
more nonzero-multiply-adds every single solve).

**`SYNTH_CLOCK_FACTOR` calibration**: the wall-clock prototype
(`ENOMOTO_SYNTH_CLOCK`) found 2-4x optimal in *wall-clock* units, but a
tick built from [`sparse_lu::TICK_BUILD_M_COEF`]/[`sparse_lu::TICK_BUILD_LU_COEF`]
left at HiGHS's own values is **not** the same unit as wall-clock
seconds, so that factor doesn't carry over — re-swept from scratch here,
in tick units, over the same 24-problem set the analysis itself used
(`analysis/ft_refactor_trigger_20260922_040850.md` §5's own table).

A coarse sweep (`ENOMOTO_SYNTH_CLOCK_FACTOR` in `{1, 1.5, 2, 3, 4, 6, 8,
12, 16, 20, 24, 32, 48, 64, 100}`, one full 24-problem pass per value)
found the aggregate 24-problem total *non-monotonic* — individual
instances (`pilot87` worst, occasionally `dfl001`) are sensitive to
exactly where a refactorization lands (a changed pivot sequence can
resync onto a longer or shorter path than before; §5's own note that
"反復数の変化は...丸めが変わるため" already flags this), so a single
aggregate-total-minimizing value can hide a large regression on one
instance a small improvement on many others outweighs in the sum. `12`,
for instance, is a genuine cliff (`pilot87` alone balloons from ~10s to
~70s at that exact value, not measurement noise — confirmed
reproducible bit-for-bit given [`Self::tick`]'s own determinism) that a
coarser or finer grid could easily have stepped over in either
direction. `16` was chosen instead by the same per-problem regression
budget this trigger's own verification uses (no instance may regress
>10%): every one of the 24 problems is flat-to-improved at `16` except
`pilot87` (+7-9%, confirmed stable — not a cliff — at `10`/`14`/`16`/
`18`/`20` alike) and `dfl001` (-1.6%, i.e. not a regression at all) —
the two instances with by far the largest absolute runtime, so keeping
*both* comfortably inside the regression budget outweighed chasing a
marginally lower 24-problem aggregate at `20` (which flips that
trade-off: `dfl001` +5.1%, `pilot87` ~flat) or higher. The target
instances this trigger exists for (`stocfor2` -39%, `bnl2` -27%,
`80bau3b` -31%, `greenbea` -23%, `degen3` -24%, `d2q06c` -17%) are all
comfortably at or beyond the wall-clock prototype's own §5 numbers at
this value. See this crate's commit history around this trigger's
introduction for the full sweep's raw numbers if re-calibrating.

### 定数 SYNTH_CLOCK_MIN_UPDATES

Same role as HiGHS's own `kSyntheticTickReinversionMinUpdateCount`
(`50`, `HEkk.h`) — a floor below which this trigger never fires
regardless of `synth_tick`, so a basis that has barely been updated at
all (where a stray large tick from a single unusually dense solve could
otherwise fire this trigger prematurely) always gets at least this many
Forrest-Tomlin updates first. Kept at HiGHS's own value: nothing in this
crate's own cost structure (unlike [`SYNTH_CLOCK_FACTOR`], which *does*
need re-deriving — see that constant's own docs) gives a reason to move
off HiGHS's number here, since this floor's only job is ruling out a
noisy false-positive on the *first few* updates, independent of either
side's own per-update cost.

### 定数 D_DRIFT_TOL

Independent drift check for the incrementally-maintained `d` (reduced
costs), checked on the same [`XB_CHECK_INTERVAL`] cadence as
[`XB_DRIFT_TOL`] but against its own residual (`‖d - fresh_d‖` scaled by
`‖fresh_d‖`, [`fresh_d_into`]'s own recomputation), not `x_B(M)`'s.

Added after tracking down a real false `Infeasible` on Netlib `pilot4`:
`d` drifted from its true (BTRAN-recomputed) value by an amount that
reached the *millions* — not noise — for over 200 nonbasic columns by
the time chuzc1 found no eligible entering column and this loop
concluded (wrongly) that the row it had just selected was a genuine
Proposition 4.6(ii) infeasibility. Before this check, `d`'s only
correction was as a *side effect* of `x_B(M)`'s own drift/refactor
triggers (`fresh_d_into` is called there anyway once a refactor already
fires) — nothing ever measured `d`'s own drift directly, so a pivot
sequence that happened to keep `x_B(M)`'s residual under [`XB_DRIFT_TOL`]
and `try_update` succeeding could let `d`'s independent error compound
unchecked for as long as that held. Confirmed present but *harmless* on
the unmodified baseline too (a genuine, moderate-magnitude violation at
one `pilot4` iteration that never compounded before the next refactor
happened to clear it) — this check does not depend on, and was not
caused by, any experimental pivot-selection change; it closes a latent
gap in this loop's own numerical safety net that any sufficiently
unlucky pivot sequence could have hit.

### 定数 D_GROSS_MISMATCH_REL_TOL

How far apart `alpha_q` (PRICE) and `alpha_full[r]` (FTRAN) may be,
relatively, before the entering pivot counts as "grossly inconsistent"
(its own call site's docs) rather than merely the ordinary floating-
point disagreement `super::UPDATE_VERIFY_TOL` (`1e-7`) already screens
for when `lu.update_count() > 0`. Sits far above `UPDATE_VERIFY_TOL`
deliberately: `0.5` still comfortably separates Netlib `pilot4`'s
`alpha_q ~ 3.4e-9` vs `alpha_full[r] ~ 1e-21` (a relative gap of
essentially `1.0`) from `bnl1`'s persistent, perfectly legitimate
`~2.4e-7` disagreement on a normal-magnitude (`~3.3e-3`) pivot — the
two Netlib instances that pinned this threshold's lower and upper
bounds respectively (confirmed via a full 73-problem sweep both ways:
tighter reproduced `perold`'s old refactor-storm pathology on `bnl1`
instead; this value reproduces neither).

### 定数 X_B_SLOPE_NOISE

`x_B(M)`'s `M`-coefficients are exactly zero or of order one in exact
arithmetic; anything this small is accumulated LU/update noise. Left in,
`Affine1::cmp_lex`'s slope-first order lets it override a comfortably
feasible `base` and drives two-variable cycles (Netlib `greenbea`).

### 定数 SLOPE_TOL

Absolute tolerance on an `M`-coefficient: stage A treats a slope
deviation as positive only above it, and the stage A -> B handoff counts
a basic `x^1_j` as sitting *on* its slope bound `l^1_j`/`u^1_j` (so that
bound survives into `l^B`/`u^B`) within it. Matches
[`Affine1::gt_zero`]'s own slope threshold, so the stage split draws the
line exactly where the lexicographic comparison it replaces did.

### 定数 Z_SLOPE_TOL

How negative `z^1` (the slope of the optimal value `z(M) = z^0 + z^1 M`)
must be to count as `z^1 < 0` (`prop:trichotomy`). A small absolute
tolerance rather than `TOL`: `z^1` is a sum of (possibly many)
reduced-cost terms, so its floor scales with the problem's own cost
magnitudes, not with `TOL`'s coefficient-level tightness — matches this
crate's own precedent of using a looser, separate tolerance for
accumulated-magnitude checks (see `simplex.rs::PRIMAL_FEAS_TOL`'s own
docs for the same reasoning). Shared by stage A's early exit and
[`finish`]'s own unboundedness test so the two can never disagree.
