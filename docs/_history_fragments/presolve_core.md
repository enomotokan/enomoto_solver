# 改良履歴メモ: 前処理コア (presolve.rs / aggregator / redundancy / scaling / propagate / colsingleton / doubleton / freevar)

コード中にあった開発経緯・計測値・却下案などのコメントを移したもの (原文の英語のまま)。

## src/presolve.rs

### モジュール冒頭 (//!) — 旧モジュール文書全文 (パイプライン構成と経緯)

Shared presolve pipeline: **the same** scaling, redundant-equality
removal, bound-propagation, and (row/column-singleton, doubleton,
dual-fixing) elimination passes feed both `simplex.rs` and
`interior_point.rs` — there is exactly one implementation of each pass,
run identically by both engines through the single [`run_extended`]
entry point, not two parallel copies (or two different pipelines) that
could drift apart.

**Shape**

Both engines want the same `A x = b`, `G x <= h` shape (variable bounds
folded into `G` as single-variable rows — see [`build_a_g`]), so the
pipeline itself is expressed purely in terms of that shape and knows
nothing about either engine's own internal representation
(`interior_point`'s KKT blocks, `simplex`'s `StdForm`):

  1. [`scaling::compute`] + [`scaling::apply`]: modified Ruiz
     equilibration, run once on the original (unscaled) `A`/`G`/`c`.
  2. [`redundancy::reduce_equalities`] + [`redundancy::reduce_inequalities`]:
     drops duplicate/linearly-dependent rows from the scaled `(A, b)`,
     and duplicate/dominated rows from the scaled `(G, h)`. Both run once
     here, before the round loop below — but [`redundancy::reduce_inequalities`]
     (a cheap O(nnz) hash pass, unlike [`redundancy::reduce_equalities`]'s
     expensive QR/Gaussian elimination) also runs again at the *end* of
     every outer round, since `doubleton`/`colsingleton` rewrite `g`'s real
     inequality rows during substitution and can turn two originally-
     distinct rows into duplicates this pre-loop call could never have
     seen (see that call site's own docs).
  3. [`run_extended`]'s own round loop: [`propagate::propagate`]
     (activity-bound constraint propagation, tightening variable bounds
     and dropping/detecting redundant/infeasible rows) →
     [`dualfix::fix_dominated_variables`] (fixes any variable whose
     objective cost prefers a direction no real row resists, straight
     off the coefficient matrix) → [`rowsingleton`], [`doubleton`], then
     an *inner* fixpoint of [`rowsingleton`] <-> [`colsingleton`] alone
     (up to `inner_rounds` times, without repaying `propagate`/
     `dualfix`'s own cost — colsingleton eliminating a variable can turn
     a row rowsingleton had no reason to touch into a fresh row
     singleton, and vice versa, so this pair alone can have more to
     find after its own first pass; `doubleton` itself isn't part of
     this inner repetition — measured to be a net *regression* when it
     was), the whole thing repeated for up to `rounds` outer passes
     since a bound `propagate` tightens can unlock a `dualfix`/
     singleton/doubleton reduction the previous outer pass couldn't yet
     see. Every stage here retries on *every* outer pass — a genuine
     fixpoint loop over the whole reduction set (the same "keep
     retrying each individual reduction until none of them find
     anything" shape HiGHS's own presolve driver uses), not a loop with
     one technique singled out for an early, hardcoded cutoff — except
     `doubleton`, which is retried every pass only *until* one of its
     own calls finds nothing, then latches off for the rest of this
     run (see [`run_extended`]'s own docs, and `doubleton_active`'s,
     for why a fixed round cap and an unconditional per-round retry
     were each measured worse than this latch on the full Netlib set).
     All of these stop early, before their own cap, once a pass changes
     neither row count nor (for the outer loop) any bound — a fixpoint:
     every stage here is a deterministic function of exactly that
     state, so a pass that changes nothing leaves nothing for a further
     pass to find either.

[`run_extended`] packages exactly this sequence into the one call site
both `simplex.rs` and `interior_point.rs` use, and returns every
eliminated variable's [`colsingleton::Substitution`] for the caller to
recover after solving (`simplex.rs`'s `unscale_result`,
`interior_point.rs`'s `unscale_with_substitutions` — same reverse-order
recovery, one implementation of the *technique*, two small call-site
adapters). An earlier version of this pipeline kept a second,
elimination-free entry point (`run`) that `interior_point.rs` used
instead, back when it had no mechanism to recover an eliminated
variable's value — removed once that mechanism was added, so every
technique ported into this pipeline (present or future) now benefits
both engines without a per-technique decision about which pipeline it
belongs in.

`simplex.rs` builds `StdForm`'s explicit `lb`/`ub` (rather than leaving
bounds as constraint rows with their own slack — a variable's bound is
represented as a bound, in both engines, not as an extra artificial/
slack variable) straight from [`ExtendedPresolveResult`]'s own `lb`/
`ub`/`real_rows`/`real_rhs` fields — the same split [`propagate::propagate`]
already computed internally, exposed here instead of `simplex.rs`
re-deriving it with its own [`propagate::extract_bounds`] call on the
just-rebuilt `g`/`h`. `interior_point.rs`, by contrast, wants bounds
folded into `G` (its central-path Newton iteration has no separate
bound-handling machinery), so it reads [`ExtendedPresolveResult`]'s
`g`/`h` fields instead — both are the same [`propagate::propagate`]
final state, just exposed in whichever shape each caller wants. The
genuinely remaining multi-variable rows (`real_rows`/`real_rhs`) become
ordinary `Eq`/`Le` rows, handled by whatever feasibility mechanism each
engine already has (interior-point's central-path Newton iteration; the
simplex method's own phase 1 / dual-feasible crash) — this pipeline
introduces no new variable of its own to either engine's standard form.

### 構造体 ExtendedPresolveResult — 旧ドキュメント (逆順復元の理由、g/h と分離形式)

The result of running the full shared presolve pipeline once: the
scaled-and-reduced problem data, the [`Scaling`] needed to map a
solution back to the original variables (via [`scaling::unscale_x`]),
an `infeasible` flag `propagate` can raise directly (an activity bound
proving a row can never be satisfied) without either engine having to
run its own solve loop first, and every variable
[`doubleton::eliminate_doubleton_equalities`]/
[`colsingleton::eliminate_singleton_equalities`] substituted out along
the way — the caller must recover each one's true value via
[`colsingleton::Substitution::value`] *before* unscaling (these
coefficients are in [`run_extended`]'s scaled space, unlike
`colsingleton`'s old standalone, pre-scaling call site), and in
**reverse** discovery order: a later round's substitution can
reference a variable an *earlier* round already substituted out (that
variable dropped to zero remaining appearances then, so a later round
is the only one that could newly treat it as eliminable in turn),
never the other way around, so resolving latest-first is what
guarantees every `value()` call only ever reads already-known inputs.

`g`/`h` fold bounds back in as single-variable rows (what
`interior_point.rs` wants — including, for an eliminated variable, its
pinned-to-a-point `[0,0]` box-bound row, since `run_extended` already
applies that fix before deriving `g`/`h`); `lb`/`ub`/`real_rows`/
`real_rhs` are the same information already split apart (what
`simplex.rs` wants) — both are `propagate::propagate`'s own final
state, carried through here so callers needing either form never have
to re-derive it with a second `propagate::extract_bounds` call.

### フィールド ExtendedPresolveResult::unbounded — 旧ドキュメント

`true` iff [`freevar::eliminate_free_variables`] found a free
variable that is genuinely unbounded — either no remaining
appearance anywhere (`A`'s rows or the real inequality rows) with a
nonzero objective coefficient, or exactly one inequality-row
appearance whose sign combination with that coefficient leaves it
unbounded on the objective-favored side (see that function's own
docs for both) — and `a`/`b`/`c`/`lb`/`ub`/etc. below must not be
trusted. Mutually exclusive with `infeasible` — presolve reports at
most one of the two.

### フィールド ExtendedPresolveResult::postsolve_log — 1 本の時系列ログである理由 (greenbea のバグ)

Every eliminating step from every technique in this pipeline
(`doubleton`/`colsingleton`/`aggregator`/`freevar`'s own ordinary
[`colsingleton::Substitution`]s, and every
[`parallelcols::merge_parallel_columns`] fold), in one single
chronological log — a caller recovers every original variable's true
value by walking this log in **reverse** (see each variant's own
`value`/`apply`) — the exact reverse of the order these steps ran in
presolve.

**Must stay one interleaved log, not two separate per-kind lists
undone in two separate passes** (an earlier version of this struct
had exactly that: `substitutions: Vec<colsingleton::Substitution>`
plus a fully separate `parallel_col_substitutions`, undone as two
back-to-back loops). That earlier design's own reasoning — "a column
`parallelcols` eliminates has zero remaining row appearances the
instant it's eliminated, so no *later* round's row-based
substitution can ever reference it as a term" — is true but answers
the wrong direction: it rules out a later ordinary substitution
referencing an already-*eliminated* column, not an *earlier*
substitution referencing a column `parallelcols` merges away
*afterward*. `parallelcols` only ever removes one of a merged pair
(`eliminated`) — the other (`kept`) survives at the *same* index,
now holding a composite value, and any ordinary substitution
recorded *before* that merge whose `terms` reference `kept` is still
sitting in the (undone-first, in the old two-pass design) ordinary
list, so it read `kept`'s post-merge composite value instead of the
real pre-merge one. Confirmed as the exact mechanism behind a false
wrong-objective result on Netlib `greenbea` (see
`parallelcols-postsolve-order-bug` project memory): resolving one
chronological log in one reverse pass (`PostsolveStep` below) is the
fix — see [`PostsolveStep`]'s own docs.

### 列挙型 PostsolveStep — 旧ドキュメント

One step of [`ExtendedPresolveResult::postsolve_log`] — either kind of
elimination this pipeline performs, kept in one shared chronological
order specifically so postsolve can undo them in a single reverse pass
(see that field's own docs for why two separate per-kind passes is
unsound). The two variants' own recovery shapes stay genuinely
different — `Sub`'s [`colsingleton::Substitution::value`] is a one-way
linear formula from already-known inputs, `ParallelCol`'s
[`parallelcols::Substitution::apply`] both reads and rewrites `kept`'s
own slot — this enum only unifies the *order* they're resolved in, not
how each one resolves.

### 関数 run_extended — 旧ドキュメント (doubleton ラッチの経緯、colsingleton のスケール後への移動)

The shared pipeline both `simplex.rs` and `interior_point.rs` call
directly: Ruiz scaling + redundant-row removal, then up to `rounds`
outer repetitions of [`propagate::propagate`] (bound tightening) →
[`dualfix`] → row-singleton fixing ([`rowsingleton`]) →
doubleton-equality substitution ([`doubleton`], once per outer round,
until it latches off — see `doubleton_active`) → up to `inner_rounds`
*inner* repetitions of [`rowsingleton`] <-> column-singleton
substitution ([`colsingleton`]) alone, each stage able to unlock more
of the next: a bound propagate tightens can turn an infinite bound
finite (letting `dualfix` fix a previously-ineligible variable), and
either substitution pass can drop a row or zero out a variable's last
remaining appearance (letting `dualfix`'s structural lock-counts, a
later outer round, or — the inner loop's own reason to exist — the
very next rowsingleton/colsingleton pass in the *same* round find
something the one before it never would). The outer loop is a genuine
fixpoint (see `prev_signature` below): it keeps re-running this whole
stage sequence until one full round changes neither a row/column count
nor any bound — the "retry every individual reduction until none of
them find anything left" loop shape HiGHS's own `HPresolve::run` uses
(`docs/` — see the HiGHS presolve summary) — except for `doubleton`
itself, which additionally *latches off* the moment one of its own
calls finds nothing (rather than being retried every remaining round
regardless, the way `rowsingleton`/`colsingleton` are): see
`doubleton_active`'s own docs for why.

This *reworks* an earlier, narrower version of this function that
capped `doubleton` to a fixed first two outer rounds
(`doubleton_rounds = 2`) after measuring that running it every round —
this function's own original design — cost more than it returned on
several Netlib instances relative to that fixed cap (~9.6% aggregate
faster capped, 51 of 73 problems, at the time of that measurement). A
full re-measurement of the *uncapped* (every round, unconditionally)
version reproduced that regression on this codebase (+6.9% aggregate
wall-clock over 73 Netlib problems, 38 slower / 32 faster, iteration
counts essentially unchanged at -0.1% — see this crate's own benchmark
CSVs), with the worst regressions landing on problems that presolve
converges on quickly (`recipe` +332%, `israel` +315%, `cycle` +133%):
exactly the case where a fixed number of *always*-paid full-matrix
scans costs more than the reduction opportunities left to find. The
latch here is the fix for that: it keeps `doubleton`'s own scan a true
per-technique fixpoint (never capped at a fixed round count picked in
advance, unlike the reverted design) while still stopping the moment
it stops paying for itself (unlike the plain uncapped version), rather
than either. Repeating `doubleton` *inside* the inner
rowsingleton<->colsingleton loop (every inner pass, not just once per
outer round) remains a separate, still-net-negative idea (69 of 73
problems slower, +20.8% aggregate when tried) and is not what this
does — `doubleton` still runs at most once per outer round, on that
round's first inner pass.

`colsingleton` used to be the one piece of this run *before* scaling
(in original, unscaled units) as `simplex.rs`'s own separate pre-step;
folding it into this scaled pipeline instead means one consistent
coordinate system for every reduction here, and lets its own
`TOL`-based decisions benefit from scaling's numerical conditioning the
same way `dualfix`/`propagate` already do (rather than running on the
original problem's raw, possibly very large or very small, coefficient
magnitudes).

### 関数 run_extended — ENOMOTO_PROF_PRESOLVE の導入理由

One-off, env-var-gated wall-clock breakdown of this function's own
major steps — `ENOMOTO_PROF_PHASES`'s `solve_lp_dual` timer starts
*after* this whole function returns, so it was blind to presolve's
own cost entirely; on several Netlib instances (`ganges`, `stocfor2`,
`sierra`) presolve turned out to be 75-92% of *total* solve time,
not the simplex loop `ENOMOTO_PROF_PHASES` already covers. A plain
local `Instant`/`eprintln!` here (not the atomics-based `timed!`
machinery `simplex.rs` uses) is enough since this function runs
once per solve, not once per pivot.

### 関数 run_extended — reduce_equalities と ENOMOTO_REDEQ_MODE

Raw bounds straight off the just-scaled `g`/`h` (no propagation yet —
that only starts inside the round loop below), handed to
`reduce_equalities` purely for its own block-decomposition pre-pass's
small-coefficient edge filter (see that pre-pass's own docs on why an
unpropagated, looser bound here is safe, just more conservative, than
the fully-tightened `orig_lb`/`orig_ub` extracted again below).

`ENOMOTO_REDEQ_MODE` picks where the rank-revealing half of
`reduce_equalities` runs (analysis/presolve_pipeline_20260924_031005.md
§2.1/C1: on 89 of the 93 Netlib problems it finds nothing beyond the
exact duplicates `dedupe_rows` already drops, yet costs ~10% of the
light problems' solve time):
  0 = here, on the full scaled `A` (the historical behaviour);
  1 = (default) duplicates only, no rank detection at all;
  2 = duplicates here, rank detection once on the much smaller `A`
      left after the round loop (see below).

### 関数 run_extended — 未接続の手法 (parallelrows/rowdominance/…) の測定

`parallelrows::merge_parallel_rows` and `rowdominance::find_dominated_rows`
(Andersen & Andersen 1995) were implemented, unit-tested, and wired in
right here for a full-Netlib A/B measurement — then removed again,
mirroring `dominatedcol`/`sparsify`'s own precedent (see either
module's own docs): both fired on **zero** of the 73 in-scope Netlib
instances (instrumented directly, not inferred from timing alone),
so wiring them in was pure candidate-search tax for no reduction
anywhere — aggregate `ours` time 3.85s unwired vs. 3.91s wired
(+1.5%), 73/73 objective values unchanged either way. Left in the
module tree, tested, for the reason each docstring gives (future
problem shapes / a relaxed finite-bounds invariant), not deleted.

Re-measured 2026-09-20 against the current pipeline (post single
dual-path unification, freevar elimination, DSE refresh-on-refactor —
all postdate the original measurement above): all six previously-
shelved reductions (`parallelrows`, `rowdominance`, `dominatedcol`,
`stuffing` here plus `sparsify` once per outer round right before
`colsingleton` and `smallcoeff` once at the very end) wired in
together, full 73-problem set, 73/73 still solved to optimality with
matching objectives either way (the `smallcoeff`-wiring `perold`
crash this module's docs warned of did *not* reproduce under the
current pipeline) — but aggregate `ours` time still regressed, 4.248s
unwired vs. 5.002s wired (+17.8%), concentrated on the same kind of
degenerate/shape-sensitive instances this crate's history keeps
finding for structural presolve changes: `cycle` +289%, `perold`
+88%, `wood1p` +55%. 10 of 73 instances improved (best: `scfxm2`
-23%), nowhere near enough to offset the losses. Conclusion
unchanged: left unwired.

### 関数 run_extended — orig_lb/orig_ub の凍結

Frozen once, before any round's `propagate` call ever runs — see
`dualpropagate::run`'s own docs for why it needs the model's
*original* bounds specifically, not whatever `lb`/`ub` a later
round's own activity-based tightening has since narrowed them to.
(One `extract_bounds` of the pre-loop `G` serves both this and the
first round's `propagate`, see `carry` below.)

### 関数 run_extended — doubleton_active ラッチ

Latches off permanently the first time a round's `doubleton` call
finds nothing: unlike `rowsingleton`/`colsingleton` (cheap enough to
keep re-trying every round regardless), `doubleton`'s own full-matrix
scan is exactly the cost the measurement in this function's own docs
found *not* worth paying once it stops finding anything — this stops
calling it the moment that happens, rather than either an arbitrary
fixed round cap (the earlier, narrower `doubleton_rounds` design) or
paying for the scan on every one of up to `rounds` rounds regardless
of whether a later round's `propagate`/`dualfix`/`rowsingleton`/
`colsingleton` reductions could in principle re-expose a new
doubleton-equality row (measured to be rare enough in practice that
this one-way latch is worth its own presolve-time savings).

### 関数 run_extended — dualpropagate_active ラッチ

Same one-way latch, same reason: `dualpropagate`'s own transpose-and-
propagate call is a full-matrix pass, worth skipping once a round's
call finds neither a new implied-equality row to promote nor a new
column to fix (its two reductions — see `dualpropagate::run`'s docs).

### 関数 run_extended — parallelcols_active ラッチ (2 ストライクの理由: ganges)

Same one-way latch, same reason again: `parallelcols`'s own
signature-grouping scan is a full-matrix pass (see its own module
docs' "Candidate search" section), worth skipping once it stops
finding anything. *Not* a simple one-strike latch like
`doubleton_active` above, though — a 2026-09-20 survey of every one
of the 73 in-scope Netlib instances found `ganges`'s own first
nonzero-elimination round is round *2*, its round 1 finding nothing
at all (only `czprob`/`greenbea` find something on round 1 itself,
then again later): a one-strike version of this latch turned off
after `ganges`'s own empty round 1, before round 2 — the one that
actually matters for it — ever ran, silently losing that instance's
entire reduction (caught by re-running this exact survey after
first writing this latch as one-strike). Requires *two consecutive*
empty rounds before disengaging instead — `ganges` alone would still
cost one avoidable extra call across its own 9 rounds, judged not
worth a third latch state to also chase down.

### 関数 run_extended — 不動点判定 prev_signature

Fixpoint detection: a round that leaves `a`/`g`'s row counts and
every bound unchanged found nothing a further round could act on
either (every stage here is a deterministic, pure function of
exactly this state), so it's safe to stop before `rounds` even on a
round that runs the full stage sequence but accomplishes nothing —
this is what turns `rounds` from "run exactly this many times" into
"run at most this many times, fewer if convergence comes first".

### 関数 run_extended — G の分離保持 (split_enabled)

Bounds kept apart from `G` (analysis/presolve_pipeline_20260924 C13):
whenever `G` would just be `rebuild_g_ref(cur_real_rows, cur_real_rhs,
lb, ub)` of a canonical split (`propagate::split_is_canonical`), the
CSR is not built — `g_split` is set, `g`/`h` are stale, and every
reader of `G` below takes the split form instead (`GView::Split`,
`reduce_inequality_rows`, `propagate_split`), which by construction
decides exactly what it would on the materialized matrix. Between
rounds the split travels in `carry`. `ENOMOTO_T_PRESOLVE_SPLIT_G=0`
always materializes (the previous behaviour, for A/B).

### 関数 run_extended — cur_real_rows を内側ループで更新する理由

The inner loop below rebuilds `g` every pass (to fold in
`rowsingleton`'s freshest fixes) from *this*, kept up to date
after each pass via `extract_bounds` on the just-updated `g` —
not left as this round's own initial `propagate` snapshot, which
would silently discard every row rewrite doubleton/colsingleton
made in an earlier inner pass (see the inner loop's own docs).

### 関数 run_extended — 等式行伝播 (eqprop) の位置と回数

Equality-row counterpart of the `propagate` call above (see
`propagate::propagate_equalities`'s own docs). Run for the first
two outer rounds — the round count that already captured every
forcing row and bound tightening in the 93-problem measurement
(analysis/greenbea_20260921_230908.md §7.1); later rounds found
nothing further but still paid for the full-matrix scan.

Tried running this pass *after* `dualpropagate`/`aggregator`/
`parallelcols` instead, so it would only bound whatever column
none of those three could already eliminate outright (they each
need a column still unbounded on their own side — see every
module's own docs). That ordering did preserve more of
`aggregator`'s reach on `stocfor2` (its own motivating instance),
but cost most of `greenbea`'s win (M-tracked columns 3,569 -> ~2k
instead of ~300, since several outer rounds' worth of `aggregator`
substitutions consume equality rows this pass would otherwise
have used) and, measured on the full 93-problem set, a *worse*
aggregate `ours` time than running here (52.1s vs 50.9s) despite
"fixing" `stocfor2` partway — `pilot87`/`d2q06c` regressed instead
under that ordering. Running here, before `dualfix`, remains the
best aggregate result found; `stocfor2`'s own regression (~+90%,
absolute ~0.13s) is this trade-off's known remaining cost — see
this function's own module docs and the loop's analysis file for
the full comparison table.

`ENOMOTO_T_EQPROP_SKIP_IDLE=1` (default 0): once a round's call
reports no forcing row, fixed column or tightened bound, skip the
remaining eqprop rounds (C20; not guaranteed identical — a later
round starts from tighter bounds and could still find something).

### 関数 run_extended — stuffing を接続しない理由

`stuffing::fix_singleton_columns` was implemented, unit-tested,
and wired in right here for a full-Netlib A/B measurement — then
removed again, joining `dominatedcol`/`sparsify`/`parallelrows`/
`rowdominance` (see the block above and each module's own docs):
it fired on **zero** of the 73 in-scope Netlib instances
(instrumented directly), and its own timing was within this
machine's measured ~20% run-to-run noise band either way (three
repeats each, total `ours` time across 72 of the 73 problems:
3.40-4.25s unwired vs. 3.68-3.97s wired — `cycle` excluded from
this comparison since it reports a wrong objective in *both*
configurations, a pre-existing bug unrelated to this pass; see
this crate's own benchmark notes). Exactly the shape the paper
itself predicts (Gamrath et al. 2015, §6): on a generic MIPLIB-
style test set stuffing fires on well under a quarter of
instances and fixes under 1% of variables even then — the
technique is aimed at supply-chain-shaped models with many
flexible-slack singleton columns, a structure no Netlib LP here
happens to have. Left in the module tree, tested, for the same
reason as its neighbors: a future problem shape (or this crate
someday handling true MIP columns, where the paper's own
reported gains were largest) could still exercise it.

### 関数 run_extended — dualpropagate の 2 つの縮小

Two reductions off one dual-feasibility propagation (see
`dualpropagate`'s own docs for both): promote every inequality
row it proves tight in every optimal solution into the equality
system outright — `doubleton`/`colsingleton`/`rowsingleton`
already know what to do with a true equality, so this needs no
new substitution logic of its own, just a relabeling of which
system a row lives in before those passes run below — and fix
every column whose reduced cost the same propagated dual box
proves one-signed everywhere (HiGHS's own "dominated column";
see `dualpropagate`'s own "Column fixing" docs section), applied
the same way `dualfix`'s own fixes are just above.

### 関数 run_extended — foldfixed を毎ラウンド実行する理由

Fold every column fixed so far (by `dualfix`/`dualpropagate` just
above, by an earlier round's `rowsingleton`, or from the model's
own input bounds) straight out of `A`'s equality rows and `G`'s
real inequality rows — see `foldfixed`'s own docs for why this
needs its own pass: fixing a bound alone leaves a row's *literal*
term count unchanged, which would otherwise hide a row that just
became a genuine `rowsingleton`/`doubleton` candidate (or short
enough for `aggregator`'s implied-free gate) behind stale dead
weight until some *later* round's `extract_bounds` call happened
to notice. Run unconditionally every round rather than gated on
"did anything get fixed this round" — same reasoning as
`reduce_inequalities(round)`'s own unconditional placement
further down: an O(nnz) scan cheap enough that the bookkeeping
to skip it (correctly, across every source of a fix — including
`rowsingleton`'s own, decided later in this same round's inner
loop and easy to under-count here) isn't worth it.

Skipped (bit-identical) when it would hand its input back: every
row non-empty, no stored zero and no fixed column — then each
row survives verbatim with an untouched rhs, and `A` (canonical)
would be rebuilt into itself.

### 関数 run_extended — 内側ループと doubleton の扱い

Inner fixpoint: rowsingleton -> colsingleton, up to `inner_rounds`
times within this same outer round (before `propagate`/`dualfix`
run again) — colsingleton eliminating a variable can turn a row
rowsingleton had no reason to touch into a fresh row singleton,
and vice versa, the same "later step unlocks an earlier one"
logic the outer round loop already relies on, just at a finer
grain and without repaying `propagate`'s own cost each time.
Stops early on the same row-count fixpoint signature the outer
loop uses (cheaper here: `lb`/`ub` aren't touched by doubleton/
colsingleton, only by `rowsingleton`'s own `fixes`, already
folded in before the signature is taken).

`doubleton` runs on every outer round's first inner pass until it
latches off (`doubleton_active`, see its own docs above) — no
longer gated by a fixed round count (see this function's own
docs for the earlier, narrower `doubleton_rounds`-capped version
this reworks). Repeating it on every one of this inner loop's
own passes (alongside rowsingleton/colsingleton) remains the
separately-measured net regression described there and is still
not done — only the *outer*-round gate was replaced with the
latch.

### 関数 run_extended — doubleton 消去変数の即時固定

Pin every newly eliminated variable's bounds to `[0, 0]`
*now*, not deferred to this function's own end-of-run
fix-up (see that fix-up's own docs for the general reason):
this inner loop's own next iteration calls `rebuild_g`
again with these same `lb`/`ub`, and `rebuild_g` emits a
box row for *every* variable with a finite bound with no
notion of "already eliminated" — leaving a substituted
variable's original bounds live would reintroduce it as a
free column for that next `rowsingleton`/`colsingleton`
pass to see and (incorrectly) act on again.

### 関数 run_extended — colsingleton 消去変数の箱制約行を落とす理由

Drop each eliminated variable's own (now-stale) box-bound
rows from `g` before folding in `cs.extra_g_rows` — unlike
`doubleton` (which never adds these rows back for an
eliminated variable in the first place), `colsingleton`
doesn't touch `g`'s existing rows at all, so its eliminated
variable's original bound rows would otherwise survive
untouched: a "phantom" column with zero cost and no `A`
appearances, but *still* carrying its real finite bounds, is
free to sit anywhere in that (possibly huge, post-Ruiz-
scaling) range without affecting feasibility or the
objective — harmless on its own, but exactly the kind of
leftover structure that let a *later* round's `dualfix` (see
its own module docs on this) or a subsequent chain step
reason about this variable as if it still had independent
degrees of freedom, instead of the single value its own
substitution now fully determines.

### 関数 run_extended — 新しい G の組み立て

New `G`: every row of the current one except eliminated
variables' single-entry rows, then `cs.extra_g_rows` —
i.e. `csr_from_rows` of those rows, assembled straight
into a `CsrRowBuilder` (from the split form when `G` is
held split: its real rows, then the bound rows of the
not-eliminated columns, from the bounds *before* the
pinning below). A row the builder rejects (a repeated
column) falls back to building that row list explicitly.

### 関数 run_extended — 内側パス後の再分解 (extract_bounds(inner))

Re-split the just-updated `g` back into its real (multi-
variable) rows for the *next* inner pass's own `rebuild_g`
call — cheap (a single scan of `g`, not a re-run of
`propagate`'s own activity-bound derivation), and necessary:
without it the next pass would silently discard every row
rewrite `doubleton`/`colsingleton` just made, reintroducing
this pass's already-eliminated variables into a stale copy
of the original row. A row that substitution collapsed from
multi-variable down to a single variable (e.g. `-a-2b<=-6`
becoming `-3a<=-6` once `b=a` is substituted in) comes back
from `extract_bounds` as a *bound*, not a row — folded into
`lb`/`ub` here (tighter of the two) rather than discarded,
since dropping it would silently lose a real constraint.

Skipped when it would be an exact round trip: `g` is still the
`rebuild_g_ref` output of this pass (no doubleton rewrite, no
colsingleton substitution since), and every real row is
already in the form `rebuild_g_ref` stores it (not a single
entry, strictly ascending columns, no stored zero) — then
`extract_bounds` would hand back `lb`/`ub` and the real rows
and rhs bit for bit.

### 関数 run_extended — Aggregator の配置・測定・行横断版の経緯

Aggregator (HiGHS's own name; `HPresolve::aggregator`): eliminates
every column a single one of `A`'s own equality rows already
proves implied-free — its box bound already forced redundant by
that row's own activity — reachable through >= 2 such rows, with
no `colsingleton`-style bound-preservation row ever needed (see
`aggregator`'s own module docs for the full history: a first
version with no implied-free gate at all was a severe regression;
a second, cross-row-aggregate version fixed that but produced a
false `Unbounded` on `shell`, caught by this crate's own
objective-mismatch check; this row-local version is the one
that's actually correct and wired in). Placed right here,
mirroring HiGHS's own placement in `HPresolve::presolve`
(`HPresolve.cpp:5901-5917`: right after its fast singleton/
doubleton loop converges, each outer main-loop iteration) — this
crate's own outer round loop already re-enters
`rowsingleton`/`doubleton`/`colsingleton`'s inner fixpoint on the
next round whenever this call changes anything (via the
`signature` fixpoint check below), giving the same "called
repeatedly, cascading with the fast loop" behavior HiGHS's own
`problemSizeReduction() > 0.05 -> continue` re-entry achieves,
without needing a separate re-entry trigger of its own.

Measured on the full 73-problem Netlib set (`ENOMOTO_DISABLE_AGGREGATOR`
A/B, two runs each way): aggregate `ours` time is statistically
indistinguishable from unwired (4.326-4.335s wired vs.
4.215-4.343s unwired — well inside this machine's own ~3% run-to-
run spread), 73/73 objectives matching either way (`cycle`'s own
pre-existing ~3e-4 mismatch unaffected). Individual instances
move more than that noise band in both directions: `stocfor2`
(this module's own motivating instance) -34% (0.191s -> 0.126s,
iterations 1665 -> ~1550), `shell` -30%, `sc205`/`scrs8`/
`scorpion` -21% to -29%; `maros` +59% (951 -> 889 iterations, but
each one costlier — the same fill-in-vs-iteration-count tradeoff
this crate's history keeps finding on specific instances),
`recipe`/`finnis`/`capri`/`agg3`/`scagr25`/`scfxm3` +18-49% (all
small in absolute time). Net: worth keeping wired in, unlike this
module's own two earlier reverted attempts — genuine, reproducible
wins on several instances against a wash everywhere else, not a
one-sided regression.
`aggregator`'s cross-row generalization
(`eliminate_implied_free_columns_xrow`, see that function's own
docs) fixes the correctness bug that sank an *earlier* cross-row
attempt (a false `Unbounded` on `shell`, root-caused: a
candidate's justification is now recomputed from live rows with
current content immediately before elimination, never trusted
from the snapshot candidate-generation pass — see that
function's own docs for the exact mechanism and the numeric
trace on `shell`), and is available here via `ENOMOTO_XROW_AGGREGATOR`.

**Stays opt-in, row-local stays default** — a first measurement
(2026-09-22) was accidentally taken on a stale feature branch 44
commits behind `main` (missing this file's own `eqprop`
wiring above and other propagate/dualpropagate strengthening),
where `greenbea` alone was pathologically slow (3,569 structural
columns left genuinely unbounded post-presolve, ~9-11s) and
dominated the 93-problem aggregate enough to show cross-row as a
reproducible ~12% win. Re-measured on actual `main` (3 reps each,
`greenbea` correctly down to 367 unbounded columns / ~0.6s there):
row-local 54.69s/53.49s/52.02s (mean ~53.4s) vs. cross-row
56.65s/50.25s/55.99s (mean ~54.3s) — the two configs' ranges
overlap and the means are within a percent of each other, i.e. a
wash, not the clean win the stale-branch measurement showed. 0/93
objective mismatches in every rep either way. Kept opt-in rather
than flipping the default a second time on inconclusive numbers —
see `presolve-fxhash-and-rebuildg-measured` and the follow-up
memory on this specific mismeasurement for the full writeup.

### 関数 run_extended — Aggregator v2 を既定にした経緯

Default since 2026-09-23 (analysis/stocfor2_presolve_20260923.md):
stocfor2 1652x1766 -> 950x1072 after presolve, -65% solve time.

### 関数 run_extended — ParallelColumns の配置・測定・既定オン化・greenbea 偽 Infeasible 修正

ParallelColumns (see `parallelcols`'s own module docs): placed
right after `aggregator`, mirroring HiGHS's own log grouping of
"Aggregator" and "Parallel rows and columns" as adjacent passes
within the same outer main-loop iteration. Latched off after two
*consecutive* empty rounds (see `parallelcols_active`'s own docs
for why one strike isn't enough here, unlike
`doubleton_active`/`dualpropagate_active` above) — a 2026-09-20
survey of every one of the 73 in-scope Netlib instances found
only three (`ganges`, `czprob`, `greenbea`) with any nonzero-
elimination round after the first, so this latch skips the
(otherwise pure-overhead) repeat scan on most rounds of the
other 70.

**Measured on the full 73-problem Netlib set (2026-09-20,
`ENOMOTO_ENABLE_PARALLELCOLS` A/B, before this latch existed):
net regression, +8.6% aggregate `ours` time (4.372s -> 4.746s),
concentrated on the same kind of degenerate/shape-sensitive
instances this crate's history keeps finding for *every*
structural presolve extension tried so far** (`aggregator`'s own
two earlier reverted attempts, the 6-technique bundle,
`parallelrows`/`dominatedcol`/`rowdominance`/`sparsify`/
`stuffing` — see each module's own docs): `wood1p` +33% (0
eliminations there, every round — pure candidate-search tax,
exactly what this latch now heads off), `scfxm3` +28%, `perold`
+22%, `maros` +23%, `pilotnov` +11%, `25fv47`/`degen3`/`stocfor2`
+4-10%. A follow-up 2026-09-21 measurement (after this latch, and
after the false-`Infeasible` fix below) found the same
instances' iteration counts don't uniformly increase — on
several (`nesm`/`scfxm3`/`ganges`) they actually *decrease* while
wall time still rises, because `XB_DRIFT_REL_TOL`-triggered
refactorizations increase 2-4x (see
`parallelcols-regression-mechanism` memory) — a real but so far
unaddressed cost, not a correctness concern.

**Turned on by default anyway (2026-09-21)**, at the user's
explicit direction, to make forward progress on the actual
structural win (`standgub`'s 908 -> 830 columns, matching HiGHS's
easier exact-cost-ratio subset of its own 396-column "Parallel
rows and columns" reduction there) while leaving the refactor-
frequency regression above as deliberately deferred future work
— mirrors `aggregator`'s own `ENOMOTO_DISABLE_AGGREGATOR` opt-out
precedent instead of staying opt-in.

Before flipping the default, a separate correctness bug was
found and fixed (see `parallelcols-greenbea-false-infeasible`
memory, and the merge loop's own comment in `parallelcols.rs`):
a merge with an opposite-signed leading coefficient could turn a
`kept` column genuinely free (`lb=-inf` *and* `ub=+inf`), which
`simplex.rs`'s own `x_j = x_j^+ - x_j^-` split handles
correctly on its own, but whose two split halves are forced onto
*exactly* the same rows with opposite coefficients — degenerate
enough on a real Netlib instance (`greenbea`, 5405 columns) to
exhaust `extended_dual`'s own `MAX_ITERS` budget, falling back to
the classical `BIG_M` path (already known unreliable — see
`bigm-fallback-invalid-reference` memory), which then reported a
false `Infeasible`. Fixed by rejecting that specific fold
outright rather than by touching anything downstream.

### 関数 run_extended — 不等式行重複削除を毎ラウンド行う理由と測定

Re-run the cheap hash-based duplicate-row pass on `(g, h)` every
outer round, not just once before this loop starts (the original
design, mirroring `reduce_equalities`'s own one-shot placement) —
unlike that rank-revealing QR/Gaussian-elimination pass (expensive,
and gated to run once for that reason, matching HiGHS's own
`removeDependentEquations`), this one is a single O(nnz) hash scan,
and `doubleton`/`colsingleton` above rewrite `g`'s real inequality
rows during substitution (see either module's own `&g`/`&h` in its
signature) — so two originally-distinct inequality rows can end up
with the same coefficient pattern only *after* a shared variable is
eliminated from both, a duplicate this function's pre-loop call (on
the original, not-yet-substituted `g`) could never have seen. Run
unconditionally rather than latched (contrast `doubleton_active`):
cheap enough every round that gating it on a prior round finding
nothing isn't worth the extra bookkeeping.

Measured directly (an `ENOMOTO_DEBUG_DEDUP_ROUND`-style row-count
counter, since removed, straight before/after this exact call) on
the full 73-problem in-scope Netlib set, A/B against this same
call sitting out here vs. only once before the round loop starts
(its pre-existing placement): this mid-loop call *does* fire —
on 44/73 instances, some substantially (`ganges` drops well over
300 rows across its own several firings within one solve,
`sierra` over 150, `stocfor2`/`cycle` over 200 each) — unlike
`parallelrows`/`rowdominance`/`dominatedcol`/`sparsify`/`stuffing`
above and below, each of which fired on *zero* of these same 73.
Most of that mid-loop churn nets out to the same *final*
`g.nrows()` this function would have reached anyway (the same
duplicate row would otherwise have been resolved some other way
by a later `propagate`/`rowsingleton`/`colsingleton` pass instead)
— pure wasted work in every one of those later passes' own
per-round scans this call now heads off instead — except on two
instances where it also survives to reduce the truly *final*
post-presolve row count: `ganges` (948 -> 936, -1.3%) and
`sierra` (1097 -> 1087, -0.9%). Aggregate wall time across all 73
was a wash either way (10081.7ms without this call vs. 10091.2ms
with it, well within run-to-run noise) — kept for the two
instances' real size reduction and the wasted-work-avoided
argument above, not for any aggregate speedup this measurement
actually showed.

### 関数 run_extended — ENOMOTO_T_ROUND_STRUCT_STOP

`ENOMOTO_T_ROUND_STRUCT_STOP=1` (default 0 = off): stop as soon as
a whole round left the structure unchanged — `A`'s row count,
`g`'s multi-entry (real) row count, the number of fixed columns
and the postsolve log length all equal to the previous round's.
Unlike the signature check below this ignores bound values, so a
round that only keeps shaving bounds (geometric convergence) ends
the loop after one such idle round instead of running to the cap.

### 関数 run_extended — 不動点判定の相対許容 (scagr25)

Bound changes below a relative 1e-3 do not count as progress
(`ENOMOTO_FIXPOINT_EXACT` restores the exact comparison):
propagation over a cyclic row structure — which the aggregator's
folds can create (`scagr25`) — keeps shaving ever-smaller amounts
off a bound (geometric convergence), which an exact comparison
never calls a fixpoint, burning every remaining round. The
tightened bounds themselves are still kept.

### 関数 run_extended — 遅延階数判定 (C1 option ii)

    if redeq_mode == 2 && a.nrows() > 0 {
Deferred rank detection (C1 option ii): the round loop has
removed most rows and columns by now, so the dense-QR / sparse-
elimination cost is paid on the reduced system only. Current
bounds feed only the block decomposition's negligible-edge filter
(see `reduce_equalities`' docs).

### 関数 run_extended — 消去変数の [0,0] 固定

Every eliminated variable's true value is recovered later purely via
`Substitution::value` (see this struct's own docs) — pinned to a
single arbitrary finite point *here*, before `g`/`h` are derived,
rather than left to each caller to notice and fix up on its own
(`simplex.rs` used to do this itself, straight on `pre.lb`/`pre.ub`,
after calling this function — still does, harmlessly redundantly,
now that it's already done here). Without this, a caller that reads
`g`/`h` directly (rather than `lb`/`ub`/`real_rows`/`real_rhs`, the
split form `simplex.rs` prefers) would see an eliminated variable as
a genuinely free, zero-cost, zero-appearance column — which is
exactly the "phantom column with independent degrees of freedom"
shape `colsingleton`'s own module docs warn a *later* presolve round
could be confused by, and which would make an interior-point
method's KKT system singular in that column outright.

### 関数 run_extended — 自由変数消去の位置

General free-variable elimination (paper §4.1): `rowsingleton`/
`doubleton`/`colsingleton` above only ever caught a free variable
appearing in exactly one or two `A` rows; this handles any remaining
one (any number of appearances, including none at all) — see
`freevar`'s own docs. Must run *after* the pinning loop just above,
not before: an already-eliminated column's own box row is long gone
by this point, so without pinning it to `[0,0]` first it would
misread here as a genuinely fresh free variable.

### 関数 run_extended — freevar の unbounded 判定を拒否する理由

`freevar`'s "unbounded" only proves `z^1 < 0` (an improving ray
exists); it is a true unboundedness verdict only if the rest of
the problem is feasible, which presolve never checks. A caller
that must tell infeasible from unbounded refuses the verdict:
the step is skipped and the solver decides.

## src/params.rs (presolve)

### 定数 SUBSTITUTION_PIVOT_RATIO

- 【元: aggregator.rs】 Mirrors `colsingleton`/`freevar`'s own pivot guard exactly (same value, same purpose).
- 【元: colsingleton.rs】 Minimum `|coeff| / max|row|` for a column singleton to be substituted out — see the guard in `eliminate_singleton_equalities`.
- 【元: freevar.rs】 Minimum `|coeff| / max|row|` for either pass to actually eliminate a free variable through a given row — mirrors `colsingleton::SUBSTITUTION_PIVOT_RATIO` exactly (same value, same purpose: a pivot that is tiny only *relative to its own row* still amplifies whatever floating-point error the row already carries when every other entry gets divided by it). Confirmed load-bearing, not merely defensive: two real Netlib instances (`perold`, `pilot4`, both already flagged elsewhere in this crate as numerically difficult) were pushed to a false `Infeasible` — reproducing identically through `extended_dual` *and* the classical `BIG_M` path, and only when this module's own elimination ran at all — by a handful of sub-1%-of-row pivots this module used to accept unconditionally, before this guard existed.

### 定数 MAX_FILLIN

Mirrors HiGHS's own `presolve_substitution_maxfillin` default (registered range `[0, 10]`, default `10`, `HighsOptions.h`): total new nonzeros a single column's elimination may introduce across every row it folds into, above which the column is left for a later call instead of risking a dense-equality-system blowup.

### 定数 MAX_CONSECUTIVE_FILLIN_FAILURES

Mirrors HiGHS's own `nfail == 3` cutoff in `HPresolve::aggregator`: after this many *consecutive* fill-in rejections, stop trying the rest of this call's candidate list outright rather than keep paying for the fill-in check on an already-too-dense region.

### 定数 DENSE_DENSITY_THRESHOLD

Density, not `p` or a dense-QR flop-count estimate, is what actually separates the two regimes — calibrated directly against measured Netlib instances, not derived analytically. The first cut at this dispatch rule used a pure cost estimate (`(n+1) * p^2`, dense QR's own flop order, thresholded so `wood1p`'s small `p` routed to dense): it fixed `wood1p` but *also* routed `standmps` (`p=268`, density 0.96%) and `fffff800` (`p=350`, density 1.6%) to dense even though the sparse method was already faster for both there (their low density means elimination stays close to its own nonzero count, with none of the fill-in blowup a size-based estimate implicitly worries about) — measured regressions of roughly 2-4x on both after that first cut. `wood1p` itself is the outlier that actually needs dense: 11.1% row density, roughly 7-30x denser than every other measured instance (`fffff800` 1.6%, `standmps` 0.96%, `sierra` 0.37%, `ganges` 0.31%, `modszk1` 0.28%, `stocfor2` 0.21%) — dense QR there stayed a bounded ~30ms while the sparse method's fill-in blew up to 962ms. `3%` sits with comfortable margin above every instance that must stay sparse and below `wood1p`'s own density.

### 定数 PIVOT_STABILITY

Numerical-stability floor a candidate pivot must clear, relative to the current **global** maximum active entry anywhere in the matrix (not just its own column's) — see `drop_linearly_dependent_sparse`'s own docs for why "global" here, not "local to the column", is what makes this a correct rank-revealing criterion instead of merely a safe-enough pivot for solving. A different, narrower purpose than `DEP_TOL`: this only gates which *candidates* the fill-minimizing search is allowed to accept, the same role `simplex::lu`'s own `STABILITY` constant plays for the (unrelated) basis factorization.

### 定数 DEP_TOL

`DEP_TOL * row_orig_norm` plays the same role here that `1e-9 * col_norm(orig)` plays against `|R[k,k]|` in `drop_linearly_dependent` (see that function's own docs for why a per-row-relative, not global, threshold matters — the same reasoning applies here). The row norm is the row's own *original* norm (computed once, before any elimination).

### 定数 PARALLEL_DECOMPOSE_ROW_THRESHOLD

Unlike every *other* `rayon` call site in this crate (all gated by `RAYON_SIZE_THRESHOLD`-style raw *problem* size, per `simplex.rs`'s own docs on measured per-element dispatch overhead), the right threshold here is total row count *within the blocks actually being split*, since a component's own elimination is real, non-trivial work per row (unlike a cheap per-element scan) — a modest absolute row count here still comfortably pays for `rayon`'s task dispatch.

### 定数 MIN_ROWS_FOR_BLOCK_DECOMPOSE

Skipping avoids always paying for the bipartite-matching-plus-SCC pass (and, if it does find multiple blocks, the per-block `HashMap`-based column remapping and fresh `BTreeMap`/bucket/heap scaffolding for each one). Measured directly (with the earlier, coarser connected-components version of this same decomposition, before it was replaced by the finer Dulmage-Mendelsohn one — the size/regression picture below is unaffected by that swap, since both pay similar decomposition overhead on tiny inputs): every real Netlib win from decomposition (`ship12s` `p=1045`, `ship08s` `p=698`, `ship04l`/`ship04s` `p=354`, `sierra` `p=528`) has `p` well above this; every case that regressed when decomposition ran unconditionally (`sc105` `p=45`, `scorpion` `p=280`, `sc205` `p=91`, `capri` `p=142`, `standgub`/`standata` `p=160`, `recipe` `p=67`, `bore3d` `p=214`) sits below it — all by a comfortable margin, so `300` is not a tight cutoff. Every one of those regressions was itself only a fraction of a millisecond in absolute terms (these are already sub-10ms problems), but with nothing to gain there either — the decomposition's benefit scales with how much per-row elimination work it *avoids* doing across blocks, which is negligible when the whole problem is this small to begin with.

### 定数 RAYON_SIZE_THRESHOLD

Same constant and rationale as `simplex.rs`'s `RAYON_SIZE_THRESHOLD` (this crate's own `#[ignore]`d `col_norm_fold_rayon_threshold_microbench` never found `rayon` winning, not even at 4,000,000 rows), duplicated locally rather than shared cross-module since each of this crate's rayon-threshold constants is already tuned/re-derived independently per call site (see e.g. `interior_point.rs`'s own separate `PROPAGATION_PASSES` copy for the same "each engine keeps its own tuning constant" convention).

### 定数 SMALLCOEFF_EPS

Analogue of this solver's own primal feasibility tolerance (`simplex.rs`'s `PRIMAL_FEAS_TOL`) — the "eps" the cumulative budget is measured against, kept as this module's own copy rather than importing `simplex`'s (this pipeline is shared with `interior_point`, which has no reason to depend on `simplex`'s own module) since both represent the same underlying concept: how much primal infeasibility this solver is willing to call negligible.

### 定数 CUMULATIVE_FRACTION

Achterberg et al.'s own `1e-1 * eps` (see the smallcoeff module docs for why this single, looser budget suffices on its own).

### 定数 NOISE_THRESHOLD

Achterberg et al.'s own `1e-10`, floating-point noise on any realistically scaled problem.

## src/presolve/aggregator.rs

### モジュール冒頭 (//!)

#### Why the first version of this module was reverted, and what changed

A first version eliminated *every* non-free bounded column (any bound,
not just an implied one) appearing in >= 2 equality rows, unconditionally
emitting `colsingleton`-style bound-preservation rows. Measured on the
full 73-problem Netlib set: +162% aggregate time, and a follow-up that
added a *pivot-row-only* skip (a `row_implied_bound` helper, since
removed — its logic now lives in `row_implies_own_bound`) for those extra
rows still measured +175% (worse). Both were wrongly blamed (in an
earlier version of this doc comment, and of `presolve.rs`'s own wiring
comment) on fill-in from the elimination fold itself slowing down the
simplex loop — that was never profiled and was wrong. `ENOMOTO_PROF_PRESOLVE`
showed the real cost was this *pass's own* running time: the eligibility
test ("any non-free bounded column") let through thousands of columns
with no realistic elimination (`wood1p`: 2592 "eligible" columns, 0
actually eliminated, 3260ms spent finding that out; HiGHS's own
`isImpliedFree` gate lets through only 2 columns on that same model),
each costing a fresh full-matrix rescan.

This version fixes the *performance* defect (the eligibility test is
now `row_implies_own_bound`, gating on a real implied-bound check
rather than "any non-free bounded column" — see that function's own
docs, and `RowActivity`'s for how it stays cheap), and keeps the
*safety* condition row-local: `eliminate_implied_free_columns` only
eliminates a column via a specific row `R` when `R`'s *own* activity
(using every other column's current bound, nothing aggregated in from
any other row) already proves `x_j`'s box bound redundant. Re-validated
at *elimination* time against the pivot row's *current* content (not
just at candidate-generation time against the initial snapshot): a fold
performed earlier in this same call can rewrite a row that is also some
other column's pivot candidate, and the row-local check must hold for
whatever content that row actually has when used, not merely what it
had when candidates were first collected.

#### Cross-row aggregation (the default since 2026-09-22)

A first cross-row aggregate version (intersecting every row's own
implication for a column before checking against `[lb_j, ub_j]`,
catching a column no *single* row alone justifies but several together
do) was tried and reverted early on: it eliminated `stocfor2` correctly,
matching an ablation of HiGHS's own Aggregator to within a few percent,
but produced a false `Unbounded` on `shell` — a real correctness bug
this crate's benchmark objective-check caught before it shipped, root-
caused only much later (its exact mechanism was unknown at revert time,
and `eliminate_implied_free_columns` shipped instead as the
version already known safe).

`eliminate_implied_free_columns_xrow` is the fixed reattempt: a
justifying row for a column can be *deleted* — consumed as a *different*
column's own pivot earlier in the same call — and a justification
computed once at candidate-generation time can go stale exactly that
way (confirmed as `shell`'s own actual mechanism: two columns there
mutually justify each other through one shared row; eliminating one
consumes that row, then a stale check wrongly still treats it as
justifying the other). The fix is to always recompute a candidate's
justification from *live* rows with *current* content immediately
before that specific elimination is committed, never trusting the
snapshot.

**Stays opt-in** (`ENOMOTO_XROW_AGGREGATOR` in `presolve.rs`), row-local
stays this module's default: a first 93-problem-Netlib measurement
(2026-09-22) was accidentally taken on a stale feature branch 44 commits
behind `main`, where `greenbea` was pathologically slow for unrelated
reasons (missing this crate's own `propagate_equalities` wiring, not
anything cross-row-specific) and dominated the aggregate enough to show
a spurious ~12% win. Re-measured on actual `main` (3 reps each way,
`greenbea` corrected): row-local and cross-row land within about a
percent of each other, overlapping ranges — a wash, not a reproducible
win, on this benchmark set. `eliminate_implied_free_columns` stays the
default; `eliminate_implied_free_columns_xrow` stays available (its
own correctness fix is real and independently regression-tested) and
still shares this module's helpers (`compute_row_activity`,
`residual_range`, `implied_range`, `fillin_cost`,
`crate::sparse::axpy_row`).

#### Default since 2026-09-23: `eliminate_implied_free_columns_v2`

The paragraph above is superseded for the default path: on `stocfor2`
the row-local gate left 704 of 1652 surviving columns in exactly the
shape "one equality row + one or more inequality rows", which neither
this gate (it needs >= 2 equality rows) nor `colsingleton` (it counts
the inequality rows too) can remove — HiGHS with only its Aggregator
switched off reproduces our old 1766x1652 presolved size almost exactly.
`eliminate_implied_free_columns_v2` keeps the cross-row version's
live re-validation (the `shell` fix), additionally uses one-sided
implied bounds from real inequality rows, admits single-equality-row
columns, and uses HiGHS's net fill-in with the size-2 exemption.
`ENOMOTO_ROWLOCAL_AGGREGATOR` / `ENOMOTO_XROW_AGGREGATOR` select the older
versions. Measurements: `analysis/stocfor2_presolve_20260923.md`.

#### Candidate order and fill-in

Mirrors `HPresolve::aggregator`'s own design (`HPresolve.cpp:6688`): all
`(row, col)` candidate pairs are collected once, sorted cheapest-first
(row-length-2-or-column-length-2 pairs before anything else, then by
`rowlen * collen` ascending — the same fill-in proxy HiGHS's own
`pdqsort` comparator uses), then processed in that order with a
`SUBSTITUTION_PIVOT_RATIO` numerical guard and a `MAX_FILLIN` cap (HiGHS's
own registered default, `presolve_substitution_maxfillin = 10`) — a
candidate whose fold would exceed it is left for a later call rather than
forced through. Three consecutive fill-in failures abort the rest of this
call's candidate list outright (mirrors HiGHS's own `nfail == 3` cutoff:
"indicates the rows/columns are becoming too dense for substitutions").

#### One non-cascading call; repetition is the caller's job

Like `colsingleton`'s own single pass, this computes implied bounds and
candidates once from an input snapshot; it does not loop internally to a
fixpoint. `presolve.rs`'s own round loop is what should call this
repeatedly (HiGHS itself calls its `aggregator` once per outer main-loop
iteration, right after its fast singleton/doubleton inner loop converges,
re-entering that inner loop whenever `aggregator` shrinks the problem —
`HPresolve.cpp:5901-5917`) so that a column exposed as implied-free only
*after* an earlier fold, or after `rowsingleton`/`colsingleton` tighten a
bound, gets caught on the next call rather than never.

#### Which rows justify / get folded (row-local & xrow versions)

Only `A`'s equality rows are used both as elimination pivots *and* as
the implied-bound justification (an inequality row can't be solved for
one variable in terms of the others the same way, and `G`'s real rows
are never used to justify an elimination even indirectly — the same
staleness hazard the cross-row version's own fix above addresses for
`A`'s rows would need its own analogous argument to extend safely to
`G`, not yet made); `G`'s real rows still get folded like any other row
referencing an eliminated column, exactly as `colsingleton`/`freevar`
already do — they just never contribute to deciding *whether* to
eliminate. Like every other pass in this crate, only appearances in
`A`'s own rows (plus `G`'s real, multi-variable rows, for fold purposes
only) count — a variable's own box-bound rows in `G` never count (same
convention `dualfix`/`colsingleton`/`freevar` use). Free variables
(`lb == -inf && ub == inf`) are left to `crate::presolve::freevar`, which
needs no implied-bound justification at all since a free variable's
bound is already vacuous. (Note: v2 later relaxed the "only `A` rows
justify" rule — see `eliminate_implied_free_columns_v2` below.)

### 構造体 RowActivity

An earlier version of this module instead recomputed each column's
residual from scratch (an `O(row_len)` activity scan over the row's
*other* terms, once per nonzero), making the caller `O(row_len^2)` per
row; on a real Netlib instance (`wood1p`, a max row length of 2592 out of
2594 columns) that cost ~225ms per call for zero eliminations, this
crate's `ENOMOTO_PROF_PRESOLVE` showed — exactly the kind of
self-inflicted cost the module docs warn a naive candidate scan can hide.
Rather than a per-column subtraction of a possibly-infinite running sum
(which risks the `inf - inf` case outright), this tracks how many terms
are unbounded on each side and, when there is exactly one, which —
mirroring HiGHS's own `getNumInfSumUpperOrig`/`getResidualSumLowerOrig`
pattern in `HPresolve.cpp` for the same reason.

### 関数 row_implies_own_bound

See the module docs for why this stays row-local rather than aggregating
across every row `j` appears in (a cross-row aggregate version was tried
and reverted after producing a false `Unbounded` on a real Netlib
instance — see `eliminate_implied_free_columns_xrow` for the root-caused,
fixed reattempt).

### 関数 eliminate_implied_free_columns_if_any

(本体コメント) Candidate pairs sorting mirrors HiGHS's own `pdqsort`
comparator in `HPresolve::aggregator` exactly (`HPresolve.cpp:6703`).
One `compute_row_activity` call per row (not per nonzero) keeps this
`O(nnz)` overall rather than `O(row_len)` per nonzero — see
`RowActivity`'s own docs on why that distinction matters.

(本体コメント, 再検証) The candidate re-check is done against the row's
*current* content: an earlier elimination this same call can have folded
into this row (if it was some other column's "other_a"), changing what it
implies about `j` since candidate generation ran on the initial snapshot
— this re-check, not just the generation-time one, is what soundness
actually depends on.

(本体コメント, other_a/other_g スキャン) Scanned fresh from the current
state — cheap now that `row_implies_own_bound` has already cut the
candidate set down to what HiGHS's own gate would (the reverted first
version's regression was exactly this scan running on thousands of
never-eliminable candidates; see the module-level history).

(本体コメント, 列→行インデックス) Lists are a *superset* (entries can
cancel to zero or rows get deleted); each query re-verifies membership
exactly as the plain scan did and sorts/dedups, so the resulting row
lists — and every decision downstream — are identical to the full-scan
version.

### 関数 eliminate_implied_free_columns_xrow

Cross-row generalization of `eliminate_implied_free_columns` (this
module's `stocfor2` motivation: an earlier attempt at exactly this
matched an ablation of HiGHS's own Aggregator there to within a few
percent). Not wired into `crate::presolve::run_extended`'s default
pipeline — reachable only via `presolve.rs`'s own
`ENOMOTO_XROW_AGGREGATOR` opt-in gate, pending a full-Netlib reach/cost
measurement — because reproducing this generalization faithfully
surfaced a real, previously un-root-caused correctness bug (see below)
rather than the "aggregate the intersection once and go" shape the
module docs' own history describes; this function is the fixed
reattempt, not a resurrection of the original.

**The correctness-critical difference from the row-local version, and
from every earlier cross-row attempt**: a candidate's justification is
*recomputed from live rows only, with their current content*,
immediately before that specific elimination is committed — never
trusted from the snapshot candidate-generation pass, and never merely
re-validated against the one row chosen as pivot (contrast the row-local
version's own single `pivot_row` re-check at its own call site, sound
there only because the row-local version's pivot row *is* its sole
justification, so `row_deleted[row_idx]` alone is enough to catch a stale
candidate). A justifying row for column `j` can itself be *deleted* —
consumed as the pivot row of an *earlier* elimination within this same
call — while a snapshot-only computation still "sees" it as live and
unchanged. Confirmed as the actual mechanism behind the historical false
`Unbounded` on Netlib `shell` (root-caused directly, not inferred):
columns 32 and 52 there mutually justify each other through one shared
length-2 row (`0.236*x32 - 0.983*x52 = 0`); eliminating column 32 first
consumes that row as its own pivot, and a snapshot-only justification for
column 52 then wrongly treats its own bound as still implied by a row
that is already gone, dropping *both* variables' boxes with nothing left
to enforce either — `x52`'s own nonzero objective coefficient then drives
it to `-inf` under plain simplex, an entirely soundness bug in this
presolve reduction itself, not anything downstream. Re-scanning every
currently-live row for `j` fresh (rather than trusting the snapshot's
`col_rows[j]` list, which this function still uses for cheap candidate
*generation* and ordering only) is what closes this gap: a stale or
deleted justifying row simply no longer contributes to the recomputed
intersection, so a candidate whose justification depended on it is
correctly rejected instead of silently eliminated.

A closely related question — can a similarly-shaped bug hide even when
only a *single* row justifies a column, if that row (not just a
multi-row intersection) is the one that gets consumed by an earlier
elimination? — turns out to already be answered by the row-local
version's own design: there, the pivot row *is* the sole justifying row,
so `row_deleted[row_idx]` (checked before any re-validation even runs)
already catches exactly that case. The bug specific to a cross-row
version is a justifying row surviving deletion of some *other* row that
happened to be a *different* column's pivot while still being treated,
by a stale computation, as if it still backed `j`'s own elimination — a
distinction that only exists once more than one row can jointly justify
a single column.

(本体コメント, row_activity キャッシュ) Essential, not just an
optimization: without this, a row with many nonzeros gets its
`O(row_len)` activity recomputed once per column that references it,
both at candidate generation and again at elimination time —
`O(row_len)` per column times up to `row_len` columns sharing one row is
exactly the `O(row_len^2)` blowup `RowActivity`'s own docs describe an
*earlier* version of the row-local pass paying on Netlib `wood1p` (a
single row with 2592 of 2594 columns nonzero) — confirmed to reproduce
here too (0.13s -> 0.46s on the full Netlib set) before this cache was
added.

(本体コメント, live_rows 再計算) "The fix": recompute from *every
currently-live* row referencing `j` (not just `col_rows[j]`'s snapshot
list — a different column's own fold can have introduced a fresh `j` term
into a row that had none at snapshot time; omitting such a row here only
widens the intersection, i.e. makes this check *more* conservative, never
unsound) and with *current* row content, not the snapshot's.

### 構造体 AggOptions

Each generalisation is an independent countermeasure (see
`analysis/stocfor2_presolve_20260923.md`) and is kept separately
switchable (the `ENOMOTO_AGG_*` opt-outs in `AggOptions::from_env`) so
each can be A/B-measured on its own.

### 関数 eliminate_implied_free_columns_v2 / eliminate_implied_free_columns_v2_if_any

(元の doc コメントは v2 本体向けの説明と `_if_any` 向けの説明が一つの
doc ブロックに混在していた — v2 本体に doc が無く、`_if_any` の上に両方
書かれていた。)

(本体コメント, 行のソート) Rows are kept sorted by column so a
coefficient lookup is a binary search, not an O(row length) scan: dense
rows (`fit2d`: ~10^4 per row) otherwise make every
lookup-per-(column, row) pass quadratic.

(本体コメント, 列カウント先行) Column counts first, so every per-column
list is allocated once at its final size (building them by `push` alone
reallocated each one log2(count) times — ~6% of `stocfor1`'s solve).

### テスト neither_row_alone_proving_it_leaves_the_column_un_eliminated

This module's check is deliberately row-local (see the module docs on
why a cross-row aggregate was tried and reverted), so even though a human
could intersect both rows' own true implications to a tighter [-95,-3]
(still not enough here, but illustrating the gap this design accepts),
this pass only ever asks a single row at a time and finds neither
sufficient -- x0 remains un-eliminated by this pass (though `A`'s
equality rows are equal to `x0`'s own true value regardless, so nothing
is *lost*, just not caught by this particular column-elimination
technique).

### テスト non_implied_free_column_is_left_alone

(unlike the reverted first version, which would have eliminated it anyway
with an explicit bound row).

### テスト xrow_rejects_a_justification_whose_shared_row_was_already_consumed

Regression test for the root-caused Netlib `shell` false-`Unbounded` bug
(see `eliminate_implied_free_columns_xrow`'s history above).

## src/presolve/redundancy.rs

### モジュール冒頭 (//!)

Original module docs (design rationale / history portions):

- `reduce_inequalities`'s own duplicate-row pass over `(G, h)` is cheap enough (a single hash scan, no linear algebra) that `run_extended`'s own round loop calls it again at the end of every outer round, not just once here — in short: `doubleton`/`colsingleton` substitution can turn two originally-distinct inequality rows into duplicates only *after* this pre-loop call already ran.
- Rank-revealing elimination: both implementations (dense column-pivoted QR, sparse Gaussian elimination picking at each step the column carrying the most numerical weight and, within it, the largest-magnitude row as pivot, mirroring dense QR's own strategy) answer the identical question — a row whose residual after eliminating every previously-kept row's pivot is negligible relative to its own original norm is a linear combination of the others — just via different arithmetic paths, each cheap in the regime the other is expensive in.
- The Dulmage-Mendelsohn pre-pass: some real Netlib instances decompose into dozens of near-identical-size blocks this way (one per vessel/route/period in a multi-period scheduling LP), and instances that share no exploitable structure by plain column-disjointness alone still often decompose into hundreds of much smaller blocks once the matching's dependency structure is taken into account. Unlike using this same decomposition for LU factorization or solving, redundancy detection only ever asks a *local* per-row question ("is this row exactly some combination of these specific other rows?"), which holds unconditionally once verified — so every block is safe to check independently and in any order, including concurrently.
- **Parallelization**: extracting each row's coefficients out of the CSR `A`/`G` is independent per row, but runs sequentially rather than via rayon — profiling on this crate's target problem sizes found rayon's per-call dispatch overhead exceeding the cost of this simple scan (the same finding as `scaling.rs`'s and `simplex.rs`'s own per-iteration loops; see `simplex.rs`'s `solve_lp_dual_on` module docs). Step 1 (`dedupe_rows`) is a single sequential scan over a shared `HashSet` by design regardless — which duplicate of an equal pair survives depends on scan order, so parallelizing it would make that choice (immaterial to correctness) nondeterministic between runs.

### 静的変数 PROF_TOTAL_STEPS / PROF_TRIVIAL_STEPS

Measurement counters answering "would a dedicated block-triangularization pre-pass (Dulmage-Mendelsohn / BTF, exposing structurally-forced 1x1 pivots before elimination starts, the way `simplex::lu`'s own `PROF_TOTAL_STEPS`/`PROF_TRIVIAL_STEPS` counters investigated for the basis LU) help `drop_linearly_dependent_sparse` the same way it was found *not* to help there" — see the `ENOMOTO_PROF_REDUNDANCY` diagnostic (`presolve.rs`'s `reduce_equalities` call site) for the answer. A step is "trivial" under the identical definition `simplex::lu::MarkowitzState::find_best_pivot` uses: the winning `(row_degree - 1) * (col_degree - 1)` Markowitz score is `0`, i.e. a structurally forced pivot a BTF pre-pass would also have found for free.

### 関数 reduce_equalities

- Dispatch rule: see `DENSE_DENSITY_THRESHOLD`'s own docs for the rule and the real Netlib instances that motivated it.
- `lb`/`ub`: dropping an edge is safe regardless of how accurate `lb`/`ub` are (only costs recall, never soundness), so passing the model's raw, not-yet-propagated bounds — this runs before presolve's own bound-tightening rounds start — is fine: staler/wider bounds just make the negligibility test fire less often, i.e. a more conservative (coarser, never incorrect) split than the fully-tightened bounds would give.

### 関数 dedupe_rows

Implementation note: same decisions as keying a `HashSet` on the normalized signature `[(j, bits(v/pivot))..., (usize::MAX, bits(rhs/pivot))]` (kept as `dedupe_rows_reference` for the equivalence test), but without materializing each signature as its own `Vec` and SipHash-ing it: the signature is hashed on the fly (the same multiplicative mix `reduce_inequalities` uses) and a hash hit is confirmed by recomputing the kept row's signature — normalization is deterministic — and comparing it entry by entry.

### 関数 drop_linearly_dependent (dense QR)

- Why explicitly sequential QR: `ColPivQr::new` would read faer's *global* parallelism (`Rayon(0)` with the `rayon` feature on), which for the sizes seen here (at most a few thousand x a few hundred) buys nothing: on a fresh process it is what first spins up rayon's global pool (4 thread spawns + per-thread arenas, ~0.5 ms), every later call pays the hand-off to the pool, and the parallel reduction order made `wood1p`'s dropped-row set vary from run to run.
- Per-row relative rank test history: an earlier version used one global threshold, `1e-10 * max(n+1, p) * max_k |R[k,k]|` — on a problem with ~1600 columns and a large appended-rhs coordinate that came to ~1e-2 in absolute terms, and it dropped equality rows whose genuine independent component was of that size (confirmed on Netlib `modszk1`/`ganges`: the solves then ended at points *violating* the dropped rows by ~1e-2, with objectives "better" than the true optimum). A tiny absolute floor (`1e-300`) still catches exact zeros.

### 関数 drop_linearly_dependent_sparse

- Why it exists alongside dense QR: dense QR's own docs used to assume "`p` is expected to be small relative to `n`" and had no fallback when several real Netlib instances violate that badly (`ganges`: `n=1681`, `p=1284` equality rows — almost every constraint is an equality), which paid for it directly — dense QR there measured at >99% of `ganges`'s *entire* presolve time, dwarfing every other technique in this pipeline combined. The sparse method's cost scales with actual nonzero fill rather than `n * p` — but that same fill-dependence is a liability on a matrix that isn't actually sparse (`wood1p`: only `p=243` but 11% row density, an order of magnitude denser than every other measured instance — fill-in during elimination blew up to 962ms there, while dense QR's cost bound doesn't care about density at all).
- Pivot selection history: candidates found via an ascending-Markowitz-degree bucket scan like `find_best_pivot`, but a candidate is only acceptable if its magnitude is within `PIVOT_STABILITY` of the current **global** maximum active entry — not, as an earlier version tried, relative only to its *own column's* current maximum. That earlier purely-local-threshold version is what `simplex::lu`'s own Markowitz factorization uses (appropriate for *solving*), but it does not work for *rank revelation*: a column's own max is trivially satisfied by its own max entry, so on real data (`ganges`) a low-degree column holding only small, easily-corrupted-by-cancellation entries got chosen as a pivot purely because it had few nonzeros, ahead of a column that was numerically dominant matrix-wide — three genuinely independent rows were misclassified as dependent (confirmed by direct comparison against the dense reference; raising the local threshold had *no* effect, proving the bug was about which column got selected, not how strong the pivot was). A purely global-norm-maximizing version (no degree preference, matching dense QR's strategy exactly) fixed that but gave up fill control entirely, causing severe fill-in blowups on several other real instances (`modszk1`, `standmps`, `wood1p`, `fffff800` all measured 3-10x slower). Gating the same degree-ascending search with a global acceptance threshold gets both: fill-minimizing order among numerically safe candidates, and a locally-large-but-globally-negligible candidate (the `ganges` failure mode) is skipped — worst case the column realizing the global max itself, which always trivially passes.
- Lazy heap maintenance: bit-identical to the eager version, which rescanned every touched column (the rhs column `n` and other long columns included) after every single step.
- `col_bits[j] < threshold` O(1) skip: same pivot choice, bit for bit; this is what keeps long runs of low-degree, tiny-valued columns (`dfl001`) from being rescanned on every single step.
- Retiring pivot row: `heap`/`col_bits` refresh is not done at the point the pivot row is removed from other columns, since the kept branch's own elimination pass touches the same columns' values again right after, and refreshing twice per step is a real, measured cost on matrices with high average column degree (`wood1p`: doing it unconditionally there as well as after elimination roughly doubled `reduce_equalities`' time).
- Dependent branch (historical, eager-heap era note): this branch never reached the post-elimination refresh pass, so `pi_cols` had to be refreshed there — otherwise a stale, too-high entry could survive in `heap` for a column whose recorded max came only from `pi`, wrongly gating out a genuinely valid pivot elsewhere via an inflated `gmax` on a later step. With the lazy heap, `pi_cols`' possible max decrease is already recorded via `note_change` beforehand.
- After elimination (historical): with the lazy heap every value change is recorded via `note_change`; no per-column rescan is needed there any more.

### 関数 dulmage_mendelsohn_blocks

- Strictly finer than a plain connected-components partition — real Netlib instances that show as a single connected component by raw column-sharing alone (`shell`, `scsd8`, `fit1p`, `ganges`) decompose into hundreds of much smaller SCCs this way (`shell`: 531 blocks from 534 rows; `ganges`: 998 from 1284), most of them singletons.
- Soundness vs LU: LU needs blocks processed in dependency order because it *propagates computed values* forward (well, needs it for *solving* — see `crate::graph::dulmage_mendelsohn_blocks`'s own docs on why even LU *factorization* itself turns out not to need that ordering, since off-diagonal spillover entries are carried through unchanged). Redundancy detection asks a purely local question per row; once verified using the rows' full, untruncated content, it holds unconditionally. The only cost of independence is recall: a redundancy whose witness spans multiple blocks (possible here since an earlier block's row can reach into a later block's columns) goes undetected and that row is conservatively kept — never the reverse.
- Small-coefficient edge filter history: `smallcoeff`'s own module docs record two prior attempts at wiring its reduction into the live pipeline, each reverted after it numerically destabilized a real instance (`perold` newly crashing, `beale_cycling_example_terminates_correctly` newly failing its IPM cross-check) — both traced to the reduction *mutating* a row/rhs a later stage then solved against. Using the same negligibility test only to decide which edges feed a graph algorithm carries none of that risk.
- **Measured** (instrumented directly) against real Netlib instances: the filter is far from a no-op on some — `shell` drops 500 of 3550 structural edges (14%), `25fv47` 72 of 3609, `sierra` 40 of 3973 — but the block *count* barely moves (`shell` 529 -> 524, `sierra` 438 -> 438, `scfxm3` 326 -> 329, `25fv47` 247 -> 241): most negligible coefficients sit inside a block the matching would have kept together anyway. `25fv47` landing on *fewer* blocks is not a soundness concern — a maximum bipartite matching is generally non-unique, so removing an edge can steer the matcher to a different one with its own SCC condensation; not monotonic in count. A full-Netlib wall-clock A/B (73 in-scope instances, 3 repeats each side) showed no aggregate difference distinguishable from run-to-run noise (both sides in the same ~4.1-5.1s band).

### 関数 drop_linearly_dependent_sparse_blocked

- Real Netlib multi-vessel/multi-period scheduling LPs (`ship12s`, `ship08s`, `ship04l`, `ship04s`, `sierra`) decompose into dozens of blocks of nearly identical size (`ship12s`: 12 blocks of exactly 78 rows each, plus 109 size-1 singletons) — one per vessel/route/period. A first version used plain connected components (disjoint column support only) and stopped there; it found those but left `shell`, `scsd8`, `fit1p`, `ganges` as a single fully-coupled component. Replacing it with the full Dulmage-Mendelsohn matching-plus-SCC decomposition finds much finer structure there too (`shell`: 531 blocks from 534 rows; `ganges`: 998 from 1284; `fit1p`: 605 from 627) — the matching exploits a *directional* dependency structure pure column-disjointness can never see. `wood1p` stays on the dense path entirely and is unaffected.
- Column remapping to `0..local_n`: without it every block's call would still pay for `aug_n`-sized scratch arrays (`col_rows`, `col_buckets`, `heap`/`col_bits`) proportional to the whole problem's `n`, defeating the point of splitting.
- rhs-augmentation edge case: `dulmage_mendelsohn_blocks` deliberately does not treat the rhs column as a graph edge, so two originally all-zero-coefficient rows with different nonzero rhs (`0 = 5`, `0 = 3`) land in separate singleton blocks, whereas the un-decomposed algorithm's shared rhs column would link them and drop one. Both outcomes are correct (each is its own infeasibility witness); this function is just more conservative. A row reducing to a pure rhs residual during elimination (general Farkas case) is still caught entirely locally within one block.
- LPT ordering: the biggest components are hardest to load-balance, so dispatching them first gives rayon's work-stealing scheduler the best chance of not stranding two large blocks on the same thread.

### 関数 reduce_inequalities

- Implementation note: same decisions as the straightforward version (`reduce_inequalities_reference`), but without materializing every row and normalized signature as its own `Vec` and SipHash-ing it: rows read straight from CSR slices, signature hashed on the fly with a cheap multiplicative mix, hash hit confirmed by recomputing the class representative's signature (bit-for-bit the reference's key, since normalization is deterministic).
- No inequality analogue of step 2 rank-revealing QR: a positive combination of several `<=` rows can imply another one, but detecting that in general is Fourier-Motzkin elimination, well beyond a cheap presolve pass.

### テスト dense_density_threshold_routes_known_instances_correctly

`standmps` and `fffff800` are the cases that specifically ruled out a pure size-based cost estimate in an earlier version of the dispatch rule — both have modest `p` but low density and must stay on the sparse path. Shapes: wood1p p=243 n=2594 nnz=70214 (11.1%); standmps p=268 n=1075 nnz=2776 (0.96%); fffff800 p=350 n=854 nnz=4775 (1.6%); ganges p=1284 n=1681 nnz=6612 (0.31%).

### テスト dulmage_mendelsohn_blocks_splits_a_pure_chain_into_singletons

Mirrors the structure real instances like `fit1p`/`ganges` showed (hundreds of singleton SCCs) despite looking like one fully-coupled component by column-sharing alone.

## src/presolve/propagate.rs

### モジュール冒頭 (//!)

Constraint propagation over inequality rows (Achterberg, Bixby, Gu,
Rothberg, Weninger, *"Presolve Reductions in Mixed Integer
Programming"*, ZIB Report 16-44, §3.1-3.2): tightens variable bounds
using row activity bounds, and detects rows that are always satisfied
(redundant, dropped) or always violated (the problem is infeasible).

For a row `sum_j a_ij x_j <= b_i`, the paper defines the minimal/maximal
*activity*

    inf{A_i. x} = sum_{a_ij>0} a_ij * lb_j + sum_{a_ij<0} a_ij * ub_j
    sup{A_i. x} = sum_{a_ij>0} a_ij * ub_j + sum_{a_ij<0} a_ij * lb_j

(§2, eq. 2.2/2.3). A row is *redundant* (always satisfied) if
`sup <= b + eps`, and the problem is *infeasible* if `inf > b + eps`
(§3.1). Otherwise, for each variable `x_k` in the row's support, the
activity bound `l_iS` of the row excluding `x_k` gives a tighter bound
on `x_k` (§3.2, eq. 3.4/3.5).

Variable bounds are not a separate vector in this codebase's `G`/`A`
representation — they are folded into `G`/`h` as single-variable rows
by `presolve::build_a_g`. So this module first pulls those out into an
explicit `lb`/`ub` pair via `extract_bounds` (which doubles as this
pass's working representation of "the bounds"), iterates §3.1/§3.2 over
the remaining multi-variable rows for `passes` rounds (each round
re-derives activities from whatever bounds were tightened so far — the
paper itself only does one round per presolve pass to keep the process
finite, see §3.2's `x1 - a*x2 = 0` example), and rebuilds `G`/`h` from
the surviving rows plus fresh bound rows for `interior_point`'s sake
(which wants bounds folded into `G` throughout). `PropagateResult`
*also* carries the already-split `lb`/`ub`/`real_rows`/`real_rhs`
directly, so `simplex.rs` (which wants bounds as `StdForm`'s own
explicit `lb`/`ub`, not folded into a row with its own slack) can use
those as-is instead of calling `extract_bounds` a second time on the
just-rebuilt `g`/`h` to undo the very folding this function just did.

**Parallelization**: `extract_bounds` is a scatter (any row can
tighten any variable's bound), so a naive per-row-parallel write would
race; a rayon fold/reduce would avoid the race (like
`scaling::compute`'s column-norm accumulation once did) but, per
profiling on this crate's target problem sizes, costs more in
dispatch overhead than this scan itself — so it runs as a single
sequential pass instead (see `simplex.rs`'s `solve_lp_dual_on` module
docs for the same finding elsewhere). The main §3.1/§3.2 pass loop in
`propagate` is sequential for an unrelated, non-negotiable reason
regardless of problem size: it is Gauss-Seidel by design (each row
reads whatever `lb`/`ub` the *previous* rows in the *same* pass already
tightened), so parallelizing across rows would change which bounds are
visible to which row and alter the pass's convergence behavior, not
just its speed.

### 構造体 PropagateResult

- `#[allow(dead_code)] // the pipeline uses PropagateSplit; kept for tests / G-form callers`
- `lb`/`ub`: The same bounds already folded into `g`/`h` as single-variable
  rows, pulled back out — see the module docs for why this saves
  callers like `simplex.rs` a redundant `extract_bounds` call.
- `real_rows`: The final surviving multi-variable rows, *not* re-folded with the
  bound rows the way `g`/`h` are — i.e. `g`/`h` minus its
  single-variable rows, equivalently `extract_bounds(n, &g, &h)`'s
  3rd/4th return values, computed once here instead of twice.

### 関数 bounds_inconsistent

A variable's own two folded-in bound rows can contradict each other —
e.g. a single-variable `x >= 10` constraint folding in against an
`x <= 5` box bound (both single-variable rows are indistinguishable to
`extract_bounds`, which just keeps the tighter of the two on each
side). That is a real infeasibility, not a representational quirk, and
must be caught explicitly before `lb`/`ub` are trusted for anything
else — every caller (`simplex.rs`'s `Tableau`, `interior_point`'s `G`
rows, `dualfix`'s own fixing rule) assumes every variable's bounds are
at least self-consistent, and `dualfix` in particular would otherwise
"fix" an already-inconsistent variable to one of its two contradictory
bounds, silently discarding the other and erasing the infeasibility.

### 構造体 PropagateSplit

`propagate`'s outcome without the re-folded `g`/`h` — what every
caller inside the presolve pipeline actually reads (`run_extended`
works on the split `lb`/`ub`/real-row form, `dualpropagate` only reads
the propagated box), so building the CSR there was pure overhead.

### 関数 propagate_split

- `ENOMOTO_T_PROP_RELTOL` (default 0 = off, the historical absolute-PROPAGATE_EPS
  rule only): HiGHS-style, a finite bound is only tightened when the
  improvement also exceeds `reltol * (1 + |bound|)` — stops the
  geometric shaving of bounds over cyclic row structures that keeps
  the outer presolve rounds from reaching a fixpoint.
  (定数 `PROP_RELTOL` に既定値を移した)
- Row activity counters: "Only 'how many' and 'which one, if exactly one' are ever
  read, so no per-row allocation is needed." (以前は `Vec` に集めていた)
- Forcing row (§3.1's sibling case to the redundant/infeasible
  checks just above, HiGHS's `HPresolve::rowPresolve` calls the
  same thing): if the row's own minimum achievable value
  already equals `b` (`true_inf` finite, `== b`), then
  `sum <= b` combined with `sum >= true_inf == b` leaves sum
  exactly one point, `b` — achievable only when every term
  sits at whichever bound produced that minimum (a positive
  coefficient at its lower bound, a negative one at its upper
  bound). That fixes every variable in the row outright, which
  in turn makes the row itself trivially satisfied — drop it,
  the same as the redundant case above, rather than running
  the (now moot) per-variable bound strengthening below on a
  row with no remaining freedom at all. `inf_unbounded.is_empty()`
  guarantees every bound this loop is about to read is finite
  (that emptiness is exactly what made `true_inf` a real
  number rather than `NEG_INFINITY` above).
  (注: `inf_unbounded` は現在 `inf_unbounded_count` カウンタ。`true_inf`/`true_sup` は
  リファクタで `min_activity`/`max_activity` に改名)
- Bound strengthening (§3.2): for each x_k in the row, l_iS is
  the row's minimal activity excluding x_k's own contribution.
  Only computable (finite) when no *other* variable is the
  source of an unbounded contribution.
- Final check: Bound strengthening above tightens `lb[k]`/`ub[k]` independently
  from each row's own activity check, so a pass can drive the two
  past each other even when no single row was individually flagged
  infeasible (e.g. two different rows each push towards the other
  side) — check once more after every pass has run.

### 列挙型 GView

`G x <= h` as a pass reads it: either a materialized CSR, or the split
form `(rows, rhs, lb, ub)` standing for exactly
`rebuild_g_ref(rows, rhs, lb, ub)` — only used when
`split_is_canonical` holds, so that `extract_bounds` of that matrix
would hand back `lb`/`ub`/`rows`/`rhs` themselves, bit for bit, and a
pass can read the split form directly instead of building the CSR (the
"bounds as rows" round trip, analysis/presolve_pipeline_20260924 C13).

### 関数 rebuild_g_ref / rebuild_g

(元コメントは 2 つの doc が連結されていた)

Rebuilds `G x <= h` from a set of "real" (multi-variable) rows plus a
fresh pair of single-variable bound rows per variable with a finite
bound — the inverse of `extract_bounds`. Shared by `propagate`'s
own ending and by `dualfix`, which also needs to fold freshly-fixed
bounds back into `G` the same way.

`rebuild_g` without taking ownership of (and so without the caller
having to clone) `rows`/`rhs`, and without allocating one `Vec` per
bound row: builds the CSR directly. Produces a bit-identical `(G, h)`
(see `sparse::CsrRowBuilder`); falls back to `rebuild_g` itself in
the rare case a real row holds a duplicate column index.

### 構造体 EqPropagateResult / 関数 propagate_equalities

Activity-based propagation over the *equality* system `A x = b`, which
`propagate` above never sees (it is handed only the inequality system
`G x <= h`). Mirrors this module's own §3.1/§3.2 reductions, applied to
an equality row's two implied inequalities `A_i x <= b_i` and
`A_i x >= b_i` instead of one: forcing-row detection on *both* sides
(activity can only reach `b_i` with every term pinned at one bound) and
bound strengthening from whichever side is finite. Only `lb`/`ub` are
mutated; the rows themselves are left for `foldfixed` (fixed terms) and
the round loop's own rowsingleton/doubleton/colsingleton passes to
shrink.

Without this, a column that appears only in equality rows and has no
finite bound anywhere else reaches `extended_dual`'s dual simplex
unbounded on that side, which forces its far more expensive "M-side"
bookkeeping for every such column. On `greenbea` (92% equality rows)
that was 3,569 columns and an 11x blow-up in iteration count; running
this pass drops it to a few hundred columns and brings the iteration
count within 2-3x of HiGHS's own presolved problem. See
`analysis/greenbea_20260921_230908.md` for the full measurement.

- In-body: A finite bound is only replaced when the change exceeds `PROPAGATE_EPS`; an
  infinite bound is always replaced by a finite one.
  `ENOMOTO_T_EQPROP_RELTOL` (default 0 = off): additionally require a
  finite bound to move by more than `reltol * (1 + |old|)`. (既定値を定数 `EQPROP_RELTOL` へ移動)

### テスト

- `forcing_row_fixes_every_variable_to_its_minimum_bound`: x0 in [1,3], x1 in [2,4] (folded in as their own box-bound
  rows), plus the real row x0 + x1 <= 3. That row's own minimum
  achievable activity (1*lb[x0] + 1*lb[x1] = 1 + 2 = 3) already
  equals its RHS, so it's a forcing row: satisfying `<= 3` at all
  requires x0=1, x1=2 exactly (either one any larger would push
  the sum past 3 with no room for the other to compensate, since
  both coefficients are positive). The forcing row itself, now trivially satisfied, is dropped.
- `forcing_row_with_mixed_signs_uses_the_matching_bound_per_term`: x0 in [0,10], x1 in [0,10], row x0 - x1 <= -4. Minimum activity:
  coefficient of x0 is positive -> use lb[x0]=0; coefficient of
  x1 is negative -> use ub[x1]=10. inf = 0 - 10 = -10 -- not equal
  to -4, so *this* row isn't forcing; instead pick bounds so the
  minimum lands exactly on the RHS: x0 in [2,10], x1 in [0,6],
  inf = 1*2 + (-1)*6 = -4 = b. Forces x0=2 (lb, positive coeff),
  x1=6 (ub, negative coeff).
- `eqprop_tests::forcing_row_fixes_every_term_at_its_matching_bound`: x0 in [0,10], x1 in [0,10], row x0 - x1 = -4. Minimum activity
  with x0 at lb=0, x1 at ub=10 is -10 (not -4, not forcing on the
  lower side); maximum activity with x0 at ub=10, x1 at lb=0 is 10
  (not -4 either). Pick bounds so the minimum lands exactly on the
  RHS instead: x0 in [2,10], x1 in [0,6], inf = 1*2 + (-1)*6 = -4 = b.

---

## src/presolve/freevar.rs

### モジュール冒頭 (//!)

FreeVar (general free-variable elimination via equality rows): a
variable with both bounds infinite (`l=-inf`, `u=+inf` — a genuine free
variable, not a finite sentinel later clamped to a large-but-finite
substitute) is eliminated from `A x = b` by plain Gaussian elimination
through any equality row it still appears in, generalizing
`crate::presolve::colsingleton`'s "appears in exactly one row"
restriction to "appears in any number of rows": this module's pivot
search picks, for each free variable in turn, whichever of its
remaining rows has the largest-magnitude coefficient (for numerical
stability, the same reason `colsingleton` guards its own single-row
pivot with `SUBSTITUTION_PIVOT_RATIO`), solves that row for the
variable, and folds it into every *other* row (and the objective) that
still references it — exactly the fold `colsingleton` already performs
against its own single owning row, just repeated across as many rows as
the variable actually appears in, and iterated (since folding one free
variable's pivot row into the rest can drop another free variable's
last remaining appearance, exposing it as its own eliminable case) until
no free variable has any remaining `A`-row appearance left.

Unlike a finite-bound elimination (`colsingleton`, `doubleton`), a truly
free variable never needs a replacement box row on the remaining variables
to preserve its own bounds elsewhere — those bounds are already vacuous
(`-inf <= r <= +inf` constrains nothing) — so eliminating it is
unconditionally free of the "must preserve the eliminated variable's own
bounds somewhere" bookkeeping those modules carry (see `colsingleton`'s own
docs for why that bookkeeping exists at all, and why it is the reason this
module never has to emit anything analogous to its `extra_g_rows`/`extra_h`).

Like `colsingleton`'s own elimination, no constant term folded out of
the objective (`c_j * rhs_i / coeff`, the part of `c_j * x_j` that does
*not* depend on any other variable) needs to be tracked here: nothing
downstream of presolve ever trusts a reduced sub-problem's own internal
objective value directly — the true objective is always recomputed by
evaluating the *original* cost vector against the fully reconstructed
`x` (every substitution's `value()` applied, in reverse discovery
order — see `simplex.rs::unscale_result`), so a constant shift that
would apply identically to every candidate solution can simply be
dropped without changing which `x` is optimal.

#### Leftover free variables

Once every free variable with a remaining `A`-row appearance has been
eliminated, two further cases are resolved directly against
`real_rows`/`real_rhs` (the real, multi-variable inequality rows —
`G`'s own single-variable box rows are excluded, same as everywhere
else in this crate; both start out read-only inputs but this module now
*does* remove a row from them when it eliminates the one free variable
that row exists to bound, so `FreeVarResult` hands back the
post-removal versions for the caller to use from here on):

- **No remaining appearance anywhere** (never had one, or lost its last
  one to a fold above): resolved purely by objective coefficient,
  following the paper's own §4.1 case split — a nonzero one means
  moving it in the direction opposite that coefficient's sign strictly
  improves the objective without bound (the whole problem is unbounded
  — `FreeVarResult::unbounded`), a zero one means fixing it to any
  finite value costs nothing (`0`, arbitrarily).
- **Exactly one inequality-row appearance, no other row anywhere**: the
  generalization of the case above — that single row, say (after `G`'s
  own `<=` normalization) `a_j x_j + rest <= h`, is the *only* thing
  constraining `x_j`, so it can be isolated exactly the way an equality
  row already is (`x_j = (h - rest) / a_j`) *provided* moving `x_j`
  toward that boundary is what the objective wants — i.e. provided
  `a_j` and `c_j` don't share a sign (a shared sign means the row only
  bounds `x_j` on the side the objective has no interest in reaching,
  so it can be pushed the *other*, genuinely unbounded way instead,
  exactly the "no appearance at all" case above but discovered through
  one still-live row rather than zero). When eligible, the row is
  genuinely redundant once `x_j` is gone — nothing else references
  `x_j`, by this case's own "exactly one appearance" precondition, so
  unlike an `A`-row elimination there is nothing left to fold this row
  into — and is dropped from `real_rows`/`real_rhs` outright, cascading
  exactly like the `A`-row loop above (dropping one variable's own row
  can reduce a *different* free variable sharing that row down to zero
  or one appearance in turn).

A free variable that still has two or more inequality-row appearances
(and none in `A`) is left exactly as-is (still free, not resolved) — a
documented residual case for `crate::simplex`'s `x_j = x_j^+ - x_j^-`
split to handle instead (see `simplex.rs::build_std_form_presolved`'s
own docs), rather than something this module can soundly eliminate: no
single one of several rows can be isolated for `x_j` the way exactly
one can, and folding would require picking one row to solve while
leaving `x_j` live in the rest, which is exactly the "must preserve
meaning in every other row" bookkeeping this module's own docs above
note a truly free variable is supposed to be exempt from.

### 構造体 FreeVarResult

- `fixed`: Leftover free variables resolved by objective coefficient alone —
  `(var, 0.0)` pairs, threaded through by the caller exactly like
  `dualfix`'s own `(usize, f64)` fixes (`lb[j] = ub[j] = value`).
- `unbounded`: `true` iff some leftover free variable (no remaining `A`-row
  appearance, and either no inequality-row appearance or exactly one
  whose sign combination doesn't let it be isolated — see the module
  docs' "Leftover free variables" section) has an objective
  coefficient that makes it genuinely unbounded — the problem is
  unbounded and the caller must stop immediately without trusting
  this result's other fields (`fixed`/`real_rows`/`real_rhs` are left
  at whatever partial state they reached and `a`/`b`/`c` reflect only
  the eliminations found before the unbounded variable was
  discovered, none of which the caller needs once it is reporting
  `Status::Unbounded`).
- `real_rows`/`real_rhs`: with every row this module eliminated (the
  "exactly one inequality-row appearance" case above) removed — the
  caller must use these, not its own original copies, for anything
  downstream (`propagate::rebuild_g`, and the final
  `ExtendedPresolveResult` fields `simplex.rs`/`interior_point.rs`
  both read `g_rows`/`g_rhs` from).

### 関数 eliminate_free_variables

- accum: One sparse accumulator for every row fold this pass performs —
  see `crate::sparse::SparseAccum`'s own docs for why the merge is
  not a per-row `BTreeMap`.
- `real_rows` の書き換え (バグ経緯): Mutated by *both* passes below, not just the inequality one: a
  variable the `A`-row loop eliminates can easily also appear in a
  `real_rows` entry (nothing before this module ever removes a
  structural variable from `G`'s own multi-variable rows just because
  some *other* module eliminated it from `A`), and that appearance
  must be folded away too — left alone, it would still reference a
  column this function is about to report as eliminated, which every
  downstream reader treats as *fixed to `0`* (the pinning convention
  `presolve.rs` applies to every `Substitution::var`), silently
  replacing whatever that row's true dependence on `x_j` was with
  "`x_j` is exactly `0`" — confirmed to actually corrupt real Netlib
  instances (`perold`, `pilot4`) into a false `Infeasible` (or, at a
  looser `SUBSTITUTION_PIVOT_RATIO`, a wrong finite objective) before
  this fold existed.
- `skip_a` (リファクタで `weak_pivot_in_a` に改名): Set for a free variable whose best available `A`-row pivot still
  fails `SUBSTITUTION_PIVOT_RATIO` (see that constant's own docs) —
  excluded from the rest of *this* pass (nothing changed about its own
  candidate rows, so re-selecting it would just fail the same check
  forever) and, since it still has a live, un-eliminated `A`-row
  appearance by construction, from the inequality pass below too (that
  pass's own "zero `A`-row appearance" precondition would otherwise be
  silently violated for it) and from the final leftover resolution
  (which must not treat a variable that still owes an equation as
  "genuinely unconstrained").
- 出現行の再計算: Every remaining free variable's appearance rows, recomputed
  fresh each pass since folding one free variable's pivot row into
  the rest can drop a *different* free variable's last remaining
  appearance (never add one — a fold only ever cancels the pivot
  column, it cannot introduce a free variable where there was
  none), exposing it as its own eliminable case next pass.
- real_rows への畳み込み: Same fold, into every `real_rows` entry `j` still appears in
  (see this function's own `real_rows`/`real_rhs` docs above for
  why this is required, not optional) — `real_rows` rows are never
  removed here (only the inequality pass below ever drops a row
  outright, when eliminating the one variable that row exists to
  bound), just rewritten to no longer reference `j`.
- 不等式パス (`skip_g` → `weak_pivot_in_g` に改名): Every free variable reaching this point either has zero remaining
  `A`-row appearances, or is `skip_a`-marked (a live `A`-row appearance
  the loop above deliberately left untouched — excluded below too, see
  `skip_a`'s own docs) — so, for everything actually eligible here,
  only `real_rows` appearances matter. A variable with exactly one is
  eliminable in place, exactly like a single-row `A` equality above,
  *provided* isolating it there is what the objective actually wants
  (see the module docs) *and* the pivot clears the same
  `SUBSTITUTION_PIVOT_RATIO` guard the `A`-row loop above does,
  recorded in `skip_g` for the same reason `skip_a` exists. Cascaded
  the same way, since dropping one variable's own row can reduce a
  *different* free variable sharing that row down to zero or one
  appearance in turn.
- 符号判定: `coeff` and `cj` sharing a (strict, non-`TOL`-noise) sign means
  this row only bounds `x_j` on the side the objective has no
  interest in reaching — pushed the other way, `x_j` is genuinely
  unbounded (see the module docs' derivation). `cj` within `TOL`
  of zero never triggers this: the objective doesn't care which
  feasible value `x_j` takes, so isolating it at this row's own
  boundary below is always valid then.
- コスト畳み込み: Same cost fold as the `A`-row loop above, and for the same
  reason (see its own comment): `x_j`'s objective contribution
  `cj * x_j` becomes, after substituting `x_j = (rhs_i -
  sum(terms)) / coeff`, a constant (dropped, never tracked — see
  the module docs) plus `-factor * a_ik` folded onto each `terms`
  column's own cost. Skipped only when `cj == 0` exactly, where
  there is nothing to fold (`terms` costs are already correct).

### テスト

- `a_row_elimination_folds_into_a_shared_real_row_too`: x0 free, x1,x2 in [0,10]. x0 == x1 (an `A` equality row -- x0's
  *only* `A`-row appearance, so it's eliminated via it), and
  separately x0 + x2 <= 5 (a `real_rows` inequality -- x0 also
  appears here). Regression test for a real bug: the `A`-row loop
  used to fold a variable's elimination into every *other* `A` row
  it appeared in, but never into a `real_rows` entry it happened
  to share -- silently leaving that row referencing a column this
  function reports as eliminated, which `presolve.rs` then pins to
  `lb[var]=ub[var]=0.0`, so the row got read downstream as if
  `x0` were fixed to `0` instead of `x1`. Confirmed to actually
  corrupt real Netlib instances (`perold`, `pilot4`) into a false
  `Infeasible` before this fold was added.
- `free_variable_in_two_inequality_rows_is_left_alone`: x0 free in two separate inequality rows -- neither alone
  determines it, and no single one can be isolated the way
  exactly one can, so it stays exactly as-is (the residual case
  `simplex.rs`'s x_j = x_j^+ - x_j^- split now handles).
- `free_variable_in_one_inequality_row_with_favorable_sign_is_substituted`: x0 free, x1 in [0,10]; single inequality row x0 + x1 <= 5, cost
  -x0 + x1 (minimizing wants x0 *large*, i.e. maximized -- the row
  caps x0 from above, exactly the direction the objective wants
  capped, so it's eligible: coeff(+1) and c[0](-1) have opposite
  signs). Substituted at the row's own boundary: x0 = 5 - x1, and
  the row itself is dropped as redundant.
- `cascading_elimination_through_shared_inequality_row`: x0, x1 free; x2 in [0,10]. Row A: x0 + x1 <= 5 (x0's *only*
  appearance; x1's first). Row B: x1 + x2 <= 3 (x1's second
  appearance; x2's only, but x2 isn't free so it never drives
  elimination itself). x1 starts at appearance count 2, so pass 1
  can only reach x0 (count 1 already): substituted (coeff(+1) vs
  c[0]=-1 opposite signs), folding c[0]'s cost onto x1's own
  (factor -1/1=-1: c[1] -= -1*1 => -2.0+1.0 = -1.0) and dropping
  Row A. That drop is what cascades: x1's appearance count falls
  to 1 (Row B only), so pass 2 reaches it in turn, using its own
  *post-fold* cost (-1.0, not the original -2.0 -- exercising that
  the fold from pass 1 is what pass 2's own sign check must see)
  against Row B's coeff(+1): opposite signs again, substitute,
  folding onto x2's cost (factor -1/1=-1: c[2] -= -1*1 =>
  0.5+1.0 = 1.5) and dropping Row B too.
- `cascading_elimination_of_two_free_variables`: x0, x1 both free. Row0: x0 + x1 = 3. Row1: x0 = 2 (i.e. x0 appears
  alone in row1 too). Eliminating x0 via row1 (larger |coeff|=1,
  tie -> first found) then folds row0 down to a singleton in x1,
  which itself has no remaining row appearance issue since x1 is
  not free. (注: 実際の係数は row1 が 2.0 なので |coeff| が大きい方として row1 が選ばれる)

### 関連 (params.rs 側に既に記載)

`SUBSTITUTION_PIVOT_RATIO` の経緯 (perold/pilot4 の false Infeasible) は params.rs の doc に残っている。

---

## src/presolve/doubleton.rs

### モジュール冒頭 (//!)

DoubletonEquation (Achterberg et al., "Presolve Reductions in Mixed
Integer Programming", §4.5): an equality row with exactly two nonzero
variables, `a_i*x_i + a_k*x_k = rhs`, lets one of them be substituted
out in terms of the other — `x_i = (rhs - a_k*x_k) / a_i` — the same
elimination `colsingleton` performs, generalized to a variable that may
still appear in *other* rows (a true column singleton, by definition,
never does). Which variable is eliminated is a numerical-stability
choice, not a free one: solving for `x_i` divides every substituted
coefficient by `a_i`, so `a_i` should be the row's *larger*-magnitude
entry (the standard choice, e.g. as used by PaPILO's own
`DoubletonEquation` presolver) — eliminating the smaller one would
divide by the smaller number, amplifying rather than damping whatever
floating-point noise is already in `rhs`/`a_k`.

Every row elsewhere that still references the eliminated variable
(`A`'s other rows and `G`'s, including `G`'s own single-variable box-
bound rows) is rewritten in place via the same substitution formula —
unlike `colsingleton`, which never needs this step because its
eliminated variable has no other appearances to rewrite. The box-bound
preservation this needs (deriving `lb_i <= x_i <= ub_i`'s equivalent
constraint on the surviving variable before `x_i`'s own bound rows are
folded away) is exactly `colsingleton`'s own derivation, reused
verbatim.

### 構造体 DoubletonResult

- `unchanged`: Set by the no-candidate fast path: `a`/`b`/`g`/`h`/`c` are exact
  copies of the inputs (see [`unchanged_if_no_candidate`] — 注: 存在しないリンクだった。
  実体は `no_candidate`、リファクタで `inputs_pass_through_unchanged` に改名)。
  `#[allow(dead_code)] // read by tests; eliminate_doubleton_equalities_view returns None instead`

### 関数 no_candidate (→ inputs_pass_through_unchanged に改名)

The no-substitution fast path of `eliminate_doubleton_equalities`:
when no pruned `A` row is a doubleton candidate, and the full pass
would only rebuild its inputs verbatim, returns copies of the inputs
instead of rewriting every row and rebuilding both matrices.

With no substitution the full pass outputs `A`'s pruned rows with
entries `|v| <= TOL` dropped, and `G` as its non-singleton rows (same
filter) followed by one `(j, 1.0) <= ub_j` / `(j, -1.0) <= -lb_j` row per
finite bound of `extract_bounds(G)`, in column order — exactly the shape
`propagate::rebuild_g_ref` produces. So the copy is taken only when `A`
and `G` are canonical CSR with every entry above `TOL`, and `G`'s
singleton rows are exactly that trailing bound block (bit for bit,
including `h`); anything else goes through the full pass.

Returns `true` when that copy would be exact (the caller keeps its
inputs as they are). A split `G` (`GView::Split`) is canonical with
exactly that bound block by construction, so only its real rows' `TOL`
test remains.

- In-body: Same candidate test as the main loop's, on the pruned row (the
  first doubleton row always claims its variables, since nothing
  is claimed yet).

### 関数 rewrite_row

Rewrites `row`/`rhs` in place for every eliminated variable `row`
references (looked up via `by_var`, a `var -> index into subs` map),
merging duplicate column indices (a row can gain a term for a variable
it already had) via a scratch map. **Transitive**: substituting `j`
out can introduce a term for a variable that is *itself* eliminated by
a different row this same pass (a chain, e.g. `x0` kept in terms of
`x1`, `x1` in turn eliminated in terms of `x2`) — a single flat pass
over `row` would leave that freshly-introduced term unresolved, so any
newly-produced term is queued back through the same substitution check
rather than written straight to `merged`; `claimed`'s own guard against
a row using an already-claimed variable as *either* of its two terms
(see the pass below) rules out a cycle, so this always terminates.

- Fast path, bit-identical to the general one below: a row with
  strictly increasing columns (so no duplicate to merge) that touches
  no substituted variable comes out of the accumulator as itself,
  minus entries at or below `TOL` — each `accum.add` is the first
  write to its slot, so the stored value is `v` exactly.
- The surviving terms land in the caller's shared sparse accumulator
  rather than a `BTreeMap` built per rewritten row — see
  `crate::sparse::SparseAccum`'s own docs. `take_sorted` emits in
  ascending column order, so the rewritten row's own ordering (which
  feeds later tie-breaks) is unchanged.

### 関数 eliminate_doubleton_equalities

One non-cascading pass: candidate doubleton rows and which variable
each eliminates are all decided from `a`'s *input* shape — a variable
only claimed as "already eliminated" within this same pass (so two
doubleton rows never both try to eliminate it), not re-checked after
rewriting (mirrors `colsingleton`'s own single-pass scope).
`#[allow(dead_code)] // run_extended calls the GView form directly`

### 関数 eliminate_doubleton_equalities_full

- `subs` の順序 / `claimed`: `subs` in true discovery (row-iteration) order — required for
  correct recovery later (a *different* round's substitution can
  depend on this round's, and must be resolved after it; sorting by
  variable index instead, as a `BTreeMap`-keyed collection would,
  scrambles that relationship). `claimed` guards *both* of a
  candidate row's variables, not just the one it would eliminate:
  a row whose "keep" side already belongs to an earlier-claimed
  variable (in this same pass) can't be treated as an independent
  doubleton either — that would silently drop the earlier
  elimination's own effect on this row (which needs a real
  `rewrite_row` substitution, not to be treated as if the claimed
  variable were still a live decision variable). Such a row is simply
  deferred: left as a surviving row below, rewritten in terms of the
  earlier substitution, and available again as a fresh candidate on
  the *next* round.
- 境界保存行: Preserve each eliminated variable's own box bounds as a `G` row on
  its surviving partner — identical derivation to `colsingleton`'s
  (see that module's docs for why skipping this silently unconstrains
  the partner). Passed through `rewrite_row` just like every other
  surviving row below: `var_keep` here can itself be a *different*
  row's `var_elim` within this same pass (chained doubletons, e.g.
  `x0` kept in terms of `x1`, `x1` in turn eliminated in terms of
  `x2`) — skipping this rewrite would leave the bound-preserving row
  pointing at `x1` after `x1` itself has zero real appearances left
  anywhere else, silently dropping `x0`'s bound constraint from the
  reduced problem instead of correctly chaining it onto `x2`.
  A genuinely infinite `lb`/`ub` (a real free or one-sided-unbounded
  source variable, not a finite sentinel) makes the corresponding
  side's derived row vacuous (`r <= +inf`) — omitted rather than
  emitted with an infinite `h`, same reasoning and arithmetic as
  `colsingleton`'s own identical derivation (see that module's docs).
- Skip a side the kept partner's own box already implies — only when
  that partner is not itself eliminated in this same pass (its box
  must stay enforced for the implication to hold).
- G 再構築: Surviving (non-eliminated) variables' own box bounds, re-folded as
  single-variable rows exactly as `build_a_g`/`propagate` do — `lb`/
  `ub` themselves are untouched by this pass (only `subs`' own
  variables lose their explicit bound rows, replaced by the
  `extra_g_rows` derived above). `G` is
  `csr_from_rows([new_g_rows, bound rows, extra_g_rows])`, assembled in
  a `CsrRowBuilder` without a `Vec` per bound row (same matrix; the
  rare row the builder rejects falls back to exactly that call).
  (ローカル `direct` はリファクタで `built_directly` に改名)

### テスト

- `no_candidate_fast_path_matches_full_pass`: The no-candidate fast path must return exactly what the full pass
  would (bit for bit), whenever it fires.

## src/presolve/colsingleton.rs

### モジュール冒頭 (//!)

- Bound-preservation rows were not part of the first version: "Skipping this (the first version of this module did) silently drops the eliminated variable's bounds: e.g. `x0 + x1 = 5` with only `x0` eliminated left `x1` completely unconstrained above, since removing the row was the *only* place `x1`'s upper reach had been limited."
- Omitting the vacuous side for genuinely infinite `lb_j`/`ub_j`: the classic "free column singleton" case then costs *zero* replacement rows, "letting a chain of these cascade through a network-shaped equality system the same way HiGHS's own presolve does."

### 関数 skip_implied_bound_rows

- Default on since 2026-09-23 (`analysis/stocfor2_presolve_20260923.md`); `ENOMOTO_KEEP_IMPLIED_BOUND_ROWS` turns it off.

### 関数 eliminate_singleton_equalities_view

- G reading: originally used `extract_bounds` (which copies every real row into its own `Vec`); now only `lb`/`ub` and the per-column appearance counts of G's real rows are needed, so G is read in place. The G-row exclusion is the same one `dualfix` applies.
- Pivot guard (`SUBSTITUTION_PIVOT_RATIO`): PaPILO applies the same kind of relative threshold before any substitution. Without it, a `coeff` tiny *relative to its own row* amplifies the reduced problem's residual (`~1e-7`) by `max|row| / |coeff|` when `x_j` is recovered — measured on Netlib `modszk1` as ~1e-6 violations of the eliminated rows and an objective slightly *below* the true optimum.
- Two singleton columns can share one row (e.g. `x5 + x7 = 3` with neither appearing anywhere else); only one is solved for, the other stays a real variable referenced as a term.
- Implied-side skip: `propagate` would drop such a row next round anyway.

## src/presolve/scaling.rs

### モジュール冒頭 (//!) — 並列化の経緯

**Parallelization**: every per-row/per-column loop in this module (`compute`'s column-norm accumulation and row/column normalization, `apply`'s rescaling, `unscale_x`) is embarrassingly parallel in principle, but `apply`/`unscale_x` run sequentially unconditionally — profiling on this crate's target problem sizes (~1000 columns, a couple thousand rows across `A`/`G`) found rayon's per-call dispatch overhead exceeding the arithmetic itself there, the same finding as `simplex.rs`'s per-pivot loops (see `solve_lp_dual_on`'s module docs). `compute`'s own column-norm fold — by far the largest single cost in this crate's presolve pipeline at that same target size (measured at ~38% of total presolve time before this file's sequential rewrite) — picks sequential vs. `rayon` once per call from `RAYON_SIZE_THRESHOLD`, replacing an earlier run-both-and-time self-calibration: this crate's own microbenchmark never found `rayon` beating a plain sequential fold at any size tried, up to 4,000,000 rows, so the live race was pure overhead for a decision with a fixed, always-the-same answer at every problem size this crate has ever actually measured.

### 関数 col_norm_fold_parallel

- Doc used to say: "Only ever invoked by `compute`'s own self-calibration, never on its own, so it always starts from an all-zero `acc` in practice; written to merge into whatever `acc` already holds anyway; matching `col_norm_fold`'s own contract exactly." (The self-calibration has since been replaced by the `RAYON_SIZE_THRESHOLD` switch.)
- `with_min_len`: an unbounded split (rayon's default for a plain range with no length hint keeps splitting under work-stealing, not just once per thread) makes the O(n) merge cost dominate at just a few thousand rows — measured directly making this ~500x slower than sequential at 200,000 rows before `with_min_len` was added.

### 関数 compute_impl — ENOMOTO_T_SCALE_NOBOUNDS 経路

- `ENOMOTO_T_SCALE_NOBOUNDS=1` (default 0 = off, the historical behaviour): leave `g`'s single-entry rows (the box-bound rows `build_a_g` adds per finite bound — on e.g. `fit2d` 99% of `g`) out of the Ruiz iteration entirely, so they neither pull on `d` nor get scanned every iteration, and give each one the closed-form row scale `1/|v * d_j|` afterwards (a bound row's own Ruiz fixed point: its scaled coefficient becomes +-1, i.e. the plain bound on `x'_j`).

### 関数 compute_impl — 単一要素行の高速経路

- Bound-row pairs (`(j, 1.0)`, `(j, -1.0)`: a variable's `ub` row followed by its `lb` row) share one scale since both start at 1 and every update reads only `|v * d_j * e|`, so their factors are equal bit for bit at every iteration.

