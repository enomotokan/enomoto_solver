# 前処理 小規模モジュール群 改良履歴メモ

`src/presolve/` 配下の小規模モジュール (dualfix, dualpropagate, foldfixed, ineqsingleton,
parallelcols, parallelrows, rowdominance, rowsingleton, smallcoeff, sparsify, stuffing,
dominatedcol) のソースコード中にあった、開発経緯・計測結果・試行錯誤・無効化理由などの
長いコメントをここに集約したもの。コード側には簡潔な日本語ドキュメントのみを残している。
原文は英語のまま (翻訳していない)。

## src/presolve/dualfix.rs

### モジュール全体 (原文ドキュメント冒頭)

DualFix (Achterberg, Bixby, Gu, Rothberg, Weninger, "Presolve Reductions in Mixed Integer
Programming", §4.4): a variable whose objective cost prefers one direction can be fixed to the
corresponding bound outright — no simplex iteration needed — provided *no* real constraint would
resist moving it that way. "Real" excludes the variable's own box-bound rows (folded into `G` by
`build_a_g`): those aren't independent constraints, they're the bounds themselves, so counting
them would make every variable look locked in both directions by its own bounds and this
reduction would never fire.

(以降の原文はアルゴリズム説明のみで、コード側の日本語ドキュメントに要約済み。履歴的記述なし。)

## src/presolve/rowsingleton.rs

履歴的記述なし (アルゴリズム説明のみで、コード側の日本語ドキュメントに要約済み)。

## src/presolve/foldfixed.rs

### モジュール全体 (動機の原文)

Left undone, a row that starts with (say) 16 terms and has 14 of them fixed away elsewhere still
*looks* like a 16-variable row to `rowsingleton` (wants exactly 1 live term), `doubleton` (wants
exactly 2), and `aggregator`'s implied-free gate — none of which can fire on it until something
drops those 14 dead terms and shrinks it down to the 2 genuinely live ones.

Run once per outer round, right after whichever passes did the fixing for that round (mirrors
this crate's own "single pass, caller repeats via the round loop" idiom used throughout this
pipeline — a column fixed by *this* round's own `rowsingleton` is picked up by *next* round's
call, not re-scanned within this same one, since `ROWSINGLETON_COLSINGLETON_INNER_ROUNDS`
defaults to a single inner pass anyway).

## src/presolve/ineqsingleton.rs

### モジュール全体「Ranged rows are one row here」の計測根拠

`G` is all `<=` rows, so a ranged row `L <= r.x <= U` lives there as the pair `r.x <= U`,
`-r.x <= -L` — and every bound-preservation row pair `colsingleton`/`doubleton` emit has exactly
that shape. Counted naively a column appearing only in such a row has *two* appearances and looks
like no singleton at all (Netlib `seba`: 86 of its 121 surviving columns were exactly this).

### 等式化 (Tight row) の正当性の原文

Tight row when side `S` alone already implies `x_j`'s bound `t` (finite implied value, `t` on the
far side of it): then every optimum has `S` holding with equality — if `S` were slack, `x_j` could
move toward `t` (it is strictly short of `t`, or at `t` which forces `S` tight by the implication)
and strictly improve the objective. The row becomes the equality `a x_j + r = S`, dropping its
other side (implied by the equality), and `colsingleton` then substitutes `x_j` out, skipping the
now-redundant bound-preservation row for `t`.

## src/presolve/dualpropagate.rs

### モジュール全体「Why this exists」(原文)

`colsingleton`'s own docs note that a column singleton's *inequality* case "needs a sign-based
case analysis of whether the row is guaranteed to bind" and is deferred — and a naive per-column
version of that case analysis (checking only that one column's own objective sign against its one
row) turns out to require a convex epigraph reformulation that *grows* the problem rather than
shrinking it (a variable+row swapped for a variable+two rows), because a single column's own cost
sign says nothing about whether the *row* itself must bind. HiGHS's own `HPresolve.cpp`
(`isDualImpliedFree`, `updateRowDualImpliedBounds`) resolves this the sound way: instead of asking
one column in isolation, it propagates the *entire* dual feasibility system (every column's own
reduced-cost identity, across every row that column touches) and asks whether *that* proves the
row's dual variable can never be zero. This module is that same idea, implemented by reusing this
crate's existing primal propagation code on a transposed problem instead of writing a second,
parallel bound-tightening algorithm from scratch.

### 「Infinite means literally +/-inf」— 厳密内側判定を撤回した経緯 (モジュール docs と `run` 内コメントの原文を統合)

**"Infinite" means literally `+/-inf`, not merely tighter than the model's own bound.** An earlier
version of this module also fired a column's t-row whenever `propagate`'s own activity tightening
had proven a bound strictly inside the model's original one (mirroring HiGHS's
`isLowerStrictlyImplied`/`isUpperStrictlyImplied`), on the theory that `x_j` could then never
reach that original bound either (i.e. also firing on `orig_ub[j] > ub[j] + TOL`). That theory
silently assumed the row which justified the tightening was still part of the row set
(`real_g_rows`/`a`) this function is handed — but `propagate` itself deletes a row as redundant
right after using it to tighten a bound (`propagate.rs`'s own redundant-row elimination), so the
assumption frequently doesn't hold, making the t-row's implicit "no other constraint keeps `x_j`
off `orig_ub[j]`" premise false in the *current* row set and unsound in general (found on Netlib
`80bau3b`: it fixed 106 columns off a dual box with no actual optimal point / containing no actual
optimal dual solution, moving the objective by +1531). `orig_lb`/`orig_ub` (the model's own,
never-tightened bounds, captured once before `run_extended`'s round loop starts) are kept as
parameters for whichever future fix re-derives this reduction with the row/bound bookkeeping it
actually needs, but this module no longer reads them.

### 「Column fixing」節の動機 (原文)

This is exactly what closes the gap this crate's own presolve leaves on network-shaped models dense
with equality-row column singletons (e.g. Netlib's `seba`: HiGHS reduces it to 2 rows / 8 columns,
largely via this reduction's own long fix/substitute cascade — see this crate's project memory for
the full investigation): such a column's cost-0, one-sided-bound "slack" is exactly the case
[`find_implied_equalities`]'s own infinite-bound test already builds a dual-sign constraint from,
but the *box* variable sharing that same equality row (finite on both sides, so invisible to that
test's own column selection) is precisely the kind of column only this read-out can fix —
`dualfix`'s simple lock-counting can't reach it either, since it explicitly disqualifies any column
touching an equality row. Once fixed, the row shrinks by one variable, which is exactly the "row
now short enough to be implied-free" condition `aggregator` needs to fire, cascading into
everything downstream that already knows what to do with a fixed column or a shorter equality row.

The same box-constrained-Lagrangian argument (minimizing `c^T x + d^T(row terms)` over a box
`[lb,ub]` independently per coordinate: `r_j > 0` forces `x_j` down to `lb_j`, `r_j < 0` up to
`ub_j`) applies unconditionally, since it only assumes `d` is *some* dual-optimal value — which the
propagated box always contains. This is HiGHS's `HPresolve.cpp` "dominated column" reduction
(`impliedDualRowBounds` feeding `isDominatedCol`), *not* Andersen & Andersen's column-vs-column
comparison ([`super::dominatedcol`]) — this crate's own name collision is coincidental.

### `run` 内の補足 (原文)

- 双対系が実行不能の場合: A genuinely infeasible dual system here would mean the primal is
  unbounded or infeasible outright — a real finding, but too strong a conclusion to act on from
  this single, partial (box-bound-driven) slice of the full dual system alone.
- 列固定の下限条件: needs `lb_j` finite: an infinite one can never be "fixed" to, and by the same
  argument this module's row-promotion half already relies on, a genuinely infinite `lb_j` would
  instead have contributed its *own* `r_j <= 0` constraint above, making `rlo > TOL` here
  self-contradictory in practice.
- `find_implied_equalities` は `DualReductions` が存在する前に書かれたテストのために残されたラッパー。

## src/presolve/parallelcols.rs

### モジュール全体「Scope」の根拠 (原文)

Every column's own *lower* bound must be finite, checked before `s` is ever computed — that finite
`lb` is the fixed anchor [`Substitution::apply`]'s recovery formula clamps around, so a
lower-unbounded column (surplus-variable-shaped, `lb=-inf`) is left untouched entirely — no
mirrored upper-anchor formula is implemented, since no Netlib instance in this crate's own
benchmark set has ever needed one. The *upper* bound, by contrast, may be finite or genuinely
infinite: a real Netlib instance (`standgub`) has a 108-column GUB block that is exactly this shape
(`lb=0`, `ub=+inf`, `cost=0` on every member) and is otherwise untouched by every other presolve
pass in this pipeline. A column with *both* sides infinite (genuinely free) is excluded outright —
left for [`super::freevar`] instead. This reduction otherwise never interacts with the
free-variable / extended-dual / `BIG_M` machinery `solve_lp_dual`'s own unbounded-structural
routing exists for: a `kept` column that already had `ub=+inf` before any merge keeps exactly that
same one-sided shape after, just with a wider finite `lb` contribution folded in. A column already
fixed (`lb == ub`, from an earlier reduction this same round, possibly not yet folded out of
`real_rows`/`a` by `foldfixed`) is skipped the same way — merging into or out of a phantom fixed
slot serves no purpose.

### 既定有効化と計測の経緯 (原文)

**Implemented, unit-tested, measured against the full Netlib set — regressed the aggregate
73-problem `ours` time 8.6% when first measured, concentrated on the same
degenerate/shape-sensitive instances every other structural presolve extension in this crate's
history has hit, but turned on by default anyway (2026-09-21, `presolve.rs`'s own
`ENOMOTO_DISABLE_PARALLELCOLS` opt-out) to make forward progress on the actual structural win**
(`standgub`'s own 108-column GUB block shrinks as expected) while leaving the regression itself as
deliberately deferred future work — see `presolve.rs`'s own call site for the full measurement
history, and the `parallelcols-regression-mechanism` memory for what the regression actually turned
out to be (mostly extra `XB_DRIFT_REL_TOL`-triggered refactorizations, not more simplex
iterations). A separate correctness bug (a merge that could produce a genuinely free column,
degenerate enough after `simplex.rs`'s own free-variable split to blow `extended_dual`'s iteration
budget and fall back to a false `Infeasible` via the unreliable classical `BIG_M` path) was found
and fixed first — see the merge loop's own comment below and the
`parallelcols-greenbea-false-infeasible` memory.

### 候補探索: 1 呼び出しで群全体を吸収する理由 (原文)

Unlike `parallelrows` (one merge per call, full stop — repeated calls across this pipeline's own
outer-round fixpoint loop pick up whatever a single call leaves behind), a single call here can
absorb an entire group at once. That is deliberate, not merely an optimization: a real Netlib
instance in this crate's own benchmark set (`standgub`) has one 108-column parallel group (a GUB
block of genuinely interchangeable decision variables), and capping this module at one merge per
call would need as many outer rounds as a group has members to fully collapse it — far more than
`run_extended`'s own round cap ever runs.

### `Substitution::apply` の正当性 (原文)

Clamping to whichever end of `[lb, ub]` `raw` overshoots is provably still feasible for `kept`
(worked out in full, both signs of `s`, in this module's own commit/test history —
`merges_and_recovers_*` tests exercise every case).

### `merge_parallel_columns_if_any` 内: 列ストリーム化と署名ハッシュ化の経緯 (原文)

- Streamed straight into the compressed column form rather than `n` growable per-column `Vec`s —
  see `CscMat::from_entry_stream`'s own docs; the two row blocks never have to be concatenated
  first. Both blocks are emitted in row order and `a`'s ids all precede `real_rows`'s, so each
  column comes out already ascending by row id — which is what the signature scan below needs, and
  what the per-column `sort_unstable_by_key` it used to run was (redundantly) producing.
- Groups columns by their normalized signature exactly like keying a `HashMap` on the
  `Vec<(row, bits)>` signature would, but without allocating and SipHash-ing one signature per
  column: a cheap multiplicative hash picks the candidate groups and a hit is confirmed by
  recomputing the group representative's signature (deterministic, so bit-identical to the stored
  key) entry by entry. Members are still appended in increasing `j`, and groups are still ordered
  by their first member, so the result is unchanged.

### 併合ループ: greenbea の偽 Infeasible と `lo == -inf` の併合拒否 (原文)

`var`'s own candidacy only required *its* `lb` to be finite — nothing there stops a *negative* `s`
(opposite-signed leading entry) paired with `var`'s `ub = +inf` from making `lo` itself `-inf`,
which would push `kept`'s own merged lower bound to `-inf`, turning it into a genuinely free
(`lb=-inf` *and* `ub=+inf`) structural column. `simplex.rs::build_std_form_presolved` *does* still
handle that correctly on its own — it splits any surviving doubly-infinite column into
`x_j = x_j^+ - x_j^-` (two `[0, inf)` slots) before `extended_dual` ever sees it, exactly the
documented fallback for `presolve::freevar`'s own residual case — so this is not unsound, just a
shape nothing upstream was ever exercised against. Measured directly on a real Netlib instance
(`greenbea`, eleven `s=-1`/`ub=+inf` pairs): the split's own two halves are forced to occupy
*exactly* the same rows with exactly opposite coefficients (`x_j^+`/`x_j^-` both M-flagged,
perfectly anti-parallel by construction) — apparently degenerate enough, stacked onto an
already-large M-flagged column count, to run `extended_dual`'s main loop out of its own
`MAX_ITERS` budget (confirmed via `ENOMOTO_DEBUG_EXT_ITERS`: `DEBUG_EXT_BAILOUT: MAX_ITERS
exhausted`), which then falls back to the classical `BIG_M` path (`solve_lp_dual`'s own documented
"should be unreachable" fallback) — and *that* path is the one that actually reports the false
`Infeasible` (see [[bigm-fallback-invalid-reference]] memory: already known unreliable independent
of this module). Skip this particular fold outright rather than manufacture that worst-case shape;
`var` is simply left unmerged (eligible for a future call once its own shape changes) — cheaper
than teaching either solver path to cope with an adversarially anti-parallel column pair.

(注: 古典的双対単体法 / BIG_M 経路は 2026-09 に削除済み。)

### 改名

- `Substitution::lb`/`ub` → `var_lb`/`var_ub` (消去列 `var` の境界であることを明示)
- ローカル `group_keys` → `groups_by_first`

## src/presolve/parallelrows.rs

### モジュール全体: 対象とする形状と正規化方法の詳細 (原文)

`-h_j/s == h_i` (within `TOL`): the two rows pin `a.x` to exactly `h_i` — **merged into one** new
equality row appended to `(A, b)`, with both original inequality rows dropped from `(G, h)`. This
is the literal row-count reduction the technique is named for — the shape it targets (an MPS
`RANGES`-section range constraint, split by this crate's own `>=`-as-negated-`<=` convention in
[`super::build_a_g`] into exactly this opposite-sign pair) whenever the range width collapses to
(near) zero.

Candidates are found the same way [`redundancy::dedupe_rows`] finds its own (equality-row)
duplicates: normalize every row (skipping length-1 rows — those are box-bound rows folding a
variable's own `lb`/`ub` into `G`, and merging a variable's own two box rows into an equality would
just re-derive a bound [`propagate`] already maintains directly, not a real reduction) by dividing
by its own *signed* first nonzero coefficient — not [`redundancy::reduce_inequalities`]'s own
`|first coeff|` normalization, which deliberately keeps each entry's sign relative to the row's
overall sign and so never lets two negated rows collide; dividing by the signed value instead
always normalizes the first entry to `+1`, so two rows that are negatives of each other land on the
identical signature regardless of which one happens to be written with a positive leading
coefficient. Run *after* [`redundancy::reduce_inequalities`] in the pipeline, so by construction no
two rows sharing a signature here can still be a *positive* multiple of one another.

Single-pass, non-cascading: a signature group with more than 2 rows only ever produces one merge
per row per call, the rest picked up by the pipeline's own outer fixpoint loop calling this again.

### 未統合 (無効) にした経緯 (原文)

**Implemented, unit-tested, measured against the full Netlib set — then left unintegrated (kept
here, tested, but never called from [`crate::presolve::run_extended`]), mirroring
[`dominatedcol`]/[`sparsify`]'s own precedent.** Wired in once, right after
[`redundancy::reduce_inequalities`], and instrumented directly (not inferred from timing alone):
across all 73 in-scope Netlib instances, zero opposite-sign proportional row pairs were ever found
— `benchmark_highs.py`'s own MPS-reading always keeps a range constraint's two sides with real
slack between them on every instance in this set, never the tight-both-ways degenerate case this
module exists to collapse. Wiring it in therefore cost pure candidate-search overhead (grouping
every multi-variable `G` row by signature) for zero reductions anywhere: aggregate `ours` time went
from 3.85s to 3.91s (+1.5%), 73/73 objective values unchanged either way. Kept here for its
correct, tested core logic — e.g. a future problem source that actually produces tight-range
constraints, or a model-building layer that emits them directly — rather than deleted.

(関連: 2026-09-20 に parallelrows/rowdominance/dominatedcol/sparsify/stuffing/smallcoeff の
6 手法を再配線して再計測し、合計 +17.8% の悪化で再び外した記録がプロジェクトメモリにある。)

### 関数内コメント (原文)

- 決定的な走査順: Deterministic order: sort groups' own keys isn't needed for correctness (every
  group is independent), but iterating a `HashMap` directly would make which-pair-merges-first
  nondeterministic across runs when a group has more than 2 members — harmless for correctness
  (every valid pairing here is equally valid) but still worth pinning down for reproducible
  benchmarking.
- 同符号の重複: already handled upstream by `redundancy::reduce_inequalities` (and if it somehow
  wasn't — e.g. this function called standalone in a test — merging it here too would need the
  same keep-tighter logic that function already implements; skip rather than duplicate that logic).

## src/presolve/rowdominance.rs

### モジュール全体: 位置付けの原文

The row-dual of [`dominatedcol`]'s column domination. There, two *columns* sharing every row were
compared coefficient-by-coefficient to decide whether one variable could always be substituted for
another without hurting feasibility or the objective. Here, two *rows* sharing every column are
compared the same way to decide whether satisfying one row's constraint always forces the other's
to hold too — making the forced one redundant, droppable outright, no bound-shifting or fixing
needed (unlike the column case, a dropped row changes nothing about which `x` remain feasible, so
there is no `infeasible`/fix-value bookkeeping here at all). The equality-row exclusion is the same
one [`dualfix`]/[`dominatedcol`] use — a column elsewhere pinned exactly by an equality has no
freedom left for this per-column argument to reason about.

This is strictly weaker than (and cannot rediscover) the *proportional* case
[`parallelrows`]/[`redundancy::reduce_inequalities`] already catch — it exists for the
complementary case those can't reach at all: two rows whose coefficients are **not** proportional,
only pointwise-ordered, need a column's nonnegativity to license the comparison rather than a
single shared scalar. The `lb[k] >= 0` requirement is what makes this pass non-vacuous on real
Netlib data in a way [`dominatedcol`]'s own escape-route conditions measurably are not there (see
that module's own docs on why every structural variable in this crate always has two *finite*
bounds): a variable merely needs a `0` lower bound, which is the common case for ordinary
structural/slack variables, not a *literal infinity* on either side.

Candidate search: the same "let the cheapest available index bound the search" idea
[`dominatedcol`]'s own candidate search and [`sparsify`]'s `anchor` both use, just picked per-row
here instead of per-column/per-equality-row there.

(注: 原文の「A row is only ever actually dropped if *no* edge anywhere in this same call points
*at* it」は実装と逆の書き方になっていた。実装は「他の行を支配した (edge の支配側に現れた) 行は
削除しない」であり、コード側の日本語ドキュメントは実装に合わせた。)

### 未統合 (無効) にした経緯 (原文)

**Implemented, unit-tested, measured against the full Netlib set — then left unintegrated (kept
here, tested, but never called from [`crate::presolve::run_extended`]), mirroring
[`dominatedcol`]/[`sparsify`]'s own precedent.** Wired in once, right after [`parallelrows`], and
instrumented directly (not inferred from timing alone): zero domination edges were found on any of
the 73 in-scope Netlib instances — the joint condition (every shared column pointwise ordered *and*
nonnegative-lower-bounded on the side that needs it, *and* the right-hand sides ordered to match)
never held for any real row pair measured, the same zero-hit-rate outcome [`dominatedcol`]'s own
docs report for the analogous column case (there for a structural reason specific to this crate's
finite-bounds invariant; here, simply because no row pair in this particular problem set happens to
satisfy a genuinely strict joint condition). Wiring it in cost pure candidate-search overhead (the
anchor-column scan, over every multi-variable row) for zero reductions anywhere — contributing,
together with [`parallelrows`], +1.5% aggregate `ours` time (3.85s unwired vs. 3.91s wired), 73/73
objective values unchanged either way. Kept here for its correct, tested core logic rather than
deleted.

## src/presolve/smallcoeff.rs

### モジュール全体: 論文の 2 段階判定を単一の累積予算にまとめた根拠 (原文)

Distinct from every other reduction in this pipeline: it never removes a row or a variable, only
individual matrix entries — a variable whose *every* remaining appearance happens to get dropped
this way simply becomes a free column for `dualfix`/`propagate` to pick up on a later round.

**The paper's own two-part scheme is implemented here as a single, strictly more general, per-row
cumulative budget**, rather than transcribed literally. Achterberg et al. first test each entry
individually (`|a_ik| < 1e-3` *and* `|a_ik| * (ub_k - lb_k) * |supp(A_i·)| < 1e-2 * eps`), then
*separately* re-scan the row with a looser, purely cumulative budget (drop entries, in column
order, as long as the running sum of `|a_ik| * (ub_k - lb_k)` stays below `1e-1 * eps`) — but the
second pass already subsumes the first: if every one of a row's (at most `|supp(A_i·)|`) entries
satisfied the first test, their contributions would sum to at most `1e-2 * eps`, comfortably
inside the second pass's own `1e-1 * eps` ceiling regardless of any entry's raw magnitude. Running
only the single, looser cumulative-budget pass therefore finds everything the paper's own two-pass
scheme does (and more, since it never additionally requires `|a_ik| < 1e-3`), while staying just as
sound: the *total* perturbation to row `i`'s own activity from every dropped entry combined never
exceeds the same `1e-1 * eps` ceiling the paper itself already accepts as safe.

A separate, unconditional pass — matching the paper's own "finally, we set coefficients with
`|a_ik| < 1e-10` to zero" — drops any coefficient this small regardless of the cumulative budget or
the variable's own bound width, since a coefficient at that scale is floating-point noise on any
problem this solver's own Ruiz scaling has already normalized.

### 未統合 (行列を書き換える版) にした経緯 (原文)

**Implemented, unit-tested, and measured against the full Netlib set — then left unintegrated
(kept here, tested, but never called from [`crate::presolve::run_extended`]).** Wired in two ways,
each measured in turn: once per outer round (right after `propagate` derives fresh bounds, so a
coefficient a bound this round just tightened could newly qualify) and, after that measured a
reproducible ~20% aggregate slowdown for a real but small yield (`ENOMOTO_PROF_SMALLCOEFF`: well
under 1% of scanned entries dropped on every instance checked — e.g. `pilotnov` 889/119286,
`ganges` 80/33500, `bnl1` 144/23989, several instances finding nothing at all), once only, at the
very end of the pipeline against its final, tightest bounds. The once-only version recovered the
lost performance (back in line with the pre-change baseline) — but on the exact same full-Netlib
run, `perold` newly crashed ("simplex basis matrix must be nonsingular"), and a synthetic
regression test already in this crate's own suite
(`simplex::tests::beale_cycling_example_terminates_correctly`) newly failed its interior-point
cross-check (misreporting `Unbounded` on a provably bounded LP). Root cause (established, not just
suspected): dropping a coefficient whose *variable* happens to already be exactly fixed
(`lb[k] == ub[k]`, `contribution == 0` unconditionally, so this reduction accepts it regardless of
the coefficient's own magnitude) is an *exact*, zero-error transformation in real arithmetic, but
it still changes the row's shape and shifts its right-hand side by a tiny floating-point amount —
enough to send the affected instance down a different numerical path, the same "a locally-sound
change can still expose latent fragility on an already-marginal, highly degenerate instance"
pattern this session hit repeatedly elsewhere (the `chuzr` rayon-vs-sequential tie-break fix, the
Schork-Gondzio Forrest-Tomlin variant, the fixed-width `chuzc1` candidacy exclusion — see
`simplex.rs`'s own history for those). `perold` and `beale_cycling`'s own IPM cross-check are both
independently already documented elsewhere in this codebase (`HARRIS_RATIO_TOL`'s own docs; this
test's own comment) as sensitive to exactly this class of small numerical perturbation. Kept
registered and tested (not deleted) in case a future, more targeted version — e.g. skipping any
entry whose variable is already exactly fixed, since that specific case is what triggered both
failures above and contributes nothing `dualfix`'s own fixing hasn't already captured — is worth
trying later.

### 3 つ目の (非破壊的な) 配線 — 現在有効 (原文)

**A third wiring, non-destructive this time, is live**: [`clean_row`] (not
[`remove_small_coefficients`] — the model's `A`/`b` are never touched) feeds
`redundancy::dulmage_mendelsohn_blocks`'s own block-decomposition pre-pass, deciding which
structural edges a negligible coefficient should be left out of when building that pre-pass's
graph. Both prior failures above trace to *mutating* a row/rhs a later stage then solved against;
using the identical negligibility test only to drop a graph edge carries none of that risk — see
`dulmage_mendelsohn_blocks`'s own docs for why dropping an edge there only costs decomposition
recall, never soundness.

## src/presolve/sparsify.rs

### モジュール全体: 設計判断の原文

This is the conservative half of the general technique (PaPILO/HiGHS also allow a bounded amount of
fill-in when the net nonzero count still improves) — restricting to the subset case trades away
those opportunities for a substitution that can never make anything *less* sparse, no fill-in
accounting or budget needed to prove it.

Unlike `doubleton`/`colsingleton`, this never eliminates a variable or a row outright — it only
makes existing rows sparser, which can turn a row into a fresh row singleton, or shrink a
variable's own column degree enough to make it eligible for `colsingleton`/`dualfix` on a later
pass.

The variable actually eliminated (`scale`'s denominator) is `S_eq`'s own largest-magnitude entry,
not necessarily `anchor` — the same "eliminate the row's largest term" stability rule `doubleton`
uses (dividing by the smallest available coefficient amplifies rounding noise), decoupled here from
which variable happened to be cheapest to search candidates on.

### 「書き換え済みの行をピボットにしない」— scorpion での系破損 (原文)

**A row already rewritten as a target this call is never itself used as a later pivot** — this is
required for correctness, not just a simplifying scope choice. An earlier version allowed it
(reading every pivot from an immutable start-of-call snapshot, on the reasoning that a row's
*original* content is an equally valid fact about the system regardless of what its own stored form
has since become): confirmed on Netlib `scorpion` to silently corrupt the system when two rows
reference each other this way in the same pass (row 0 used to sparsify row 1, then row 1's
*original* content — while row 1 itself had just been overwritten — used to sparsify row 0 right
back). Each individual substitution is a locally valid row operation, but replacing a *pair* (or
longer chain) of rows with independently-computed linear combinations of their originals is only
guaranteed equivalent to the original pair if the combined transformation is invertible — true for
an isolated pair often enough to not show up immediately, but not guaranteed once many such
substitutions chain together across a whole pass, where it measurably was not. Forbidding a
just-modified row from being read as a pivot keeps every pivot's own stored form and the fact used
to derive other rows identical, side-stepping the question entirely. `b[eq_idx]` is read fresh (not
from a start-of-call snapshot) for the same reason.

### 未統合 (無効) にした経緯 (原文)

**Implemented, unit-tested (including a regression test for the mutual-reference bug above), and
measured against the full Netlib set — then left unintegrated (kept here, tested, but never called
from [`crate::presolve::run_extended`]).** Wired in once per outer round, right before
`colsingleton`: with the correctness bug fixed, zero objective mismatches across all 73 Netlib
instances — but total `ours` time went from 3.68s to 7.30s (a ~2x aggregate regression),
concentrated almost entirely on a handful of instances already known in this crate's own history to
be unusually sensitive to *any* change in matrix shape: `degen3` (+390%), `25fv47` (+315%),
`wood1p` (+127%), `cycle` (+95%) — `iters` on `degen3` alone went from 2,422 to 11,676 with an
*identical, still-correct* final objective, the same "reshaping the matrix changes chuzc/DSE
tie-breaking, which cascades into a completely different (and on a degenerate instance, potentially
far longer) pivot sequence" pattern this session already hit repeatedly for other structural
presolve changes (the Dulmage-Mendelsohn block-triangularization attempt documented in
`simplex::lu`, connected-component splitting's own "at least 2 real components" gate in
`simplex.rs`). A handful of small instances did improve (`bnl1` -21%, `brandy` -29%), but nowhere
near enough to offset the losses. No cheap pre-gate (route only instances likely to benefit) was
tried before reverting — see this doc comment's own commit for the full numbers if revisiting.

### 関数内コメント (原文)

- `SparseAccum`: One sparse accumulator for every target-row rewrite — see
  `crate::sparse::SparseAccum`'s own docs for why the merge is not a per-target `BTreeMap`.
- 係数の打ち消し: Only `elim_var` is *proven* to cancel exactly (`scale` was chosen specifically
  to zero it) — any other entry's subtraction result, however small, is the mathematically correct
  new coefficient, not noise, and must be kept as-is rather than dropped by some absolute tolerance:
  a target row's other shared coefficient can land close to (but not at) zero by sheer coincidence
  without being a true cancellation, and silently discarding it would corrupt the row.

### 改名

- 非公開 enum `Src` → `RowSource`

## src/presolve/stuffing.rs

(未統合にした経緯・計測値は `src/presolve.rs` の `stuffing::fix_singleton_columns` 呼び出し予定箇所の
コメントにある (そのファイルは本メモの担当外)。要旨: 73 Netlib 問題で発火 0 件、計測差はノイズ範囲内。)

### モジュール全体「Relationship to `dualfix`」(原文)

A singleton column's one nonzero entry gives it either `up_lock=0` or `down_lock=0` (never both
locked, since a lone entry can only resist movement in *one* direction). `dualfix` already fixes
the case where the objective sign *agrees* with the unlocked direction (`down_lock==0 && c>=0`, or
symmetrically `up_lock==0 && c<=0`) — no row reasoning needed there, moving that far is free for
every row simultaneously. This module picks up exactly dualfix's residual: `a_rj>0 && c_j<0` (wants
to move *up*, the *locked* direction for a `<=` row with a positive entry) and its mirror
`a_rj<0 && c_j>0`. Moving that way is no longer unconditionally safe — it uses up row `r`'s own
slack — but with only *one* row to reason about (the column's single appearance), and every other
flexible column in that row treated as a knapsack of competing claims on the same slack, it can
still often be decided outright which of these columns end up at a bound in some optimum, without
solving the LP.

`colsingleton`'s own docs note that a column singleton's *inequality* case "needs a sign-based case
analysis of whether the row is guaranteed to bind" and is deferred; `dualpropagate` resolved the
case where the row provably always binds (a global, propagated argument). This module resolves the
opposite residual — the row does *not* provably bind, so instead of asking "is there room," it asks
"how much room, shared between how many competing columns."

### Algorithm 1 の各分岐の説明 (原文)

- `alpha <= b_r - Ũr + beta` — even in the worst case for every column not yet decided, pushing
  this one to its upper bound still leaves `r` satisfiable, and doing so can only help the
  objective — fix `x_j = ub_j`.
- otherwise, `b_r <= L̃r` — the row's minimum possible activity (everything not yet decided at its
  *most* row-tightening extreme) already saturates `b_r`, so this column (and, once triggered,
  every column processed after it — `L̃r` only ever grows from here) cannot move up at all — fix
  `x_j = lb_j`.
- otherwise: genuinely undetermined by this row alone (the true LP optimum may sit at a fractional
  point, the classic continuous-knapsack shape the paper motivates this with) — left alone.

`Ũr`/`L̃r` are then advanced by `alpha - beta` unconditionally (matching the paper's own Algorithm
1 line-for-line, not just on the branch that fires): once a column's slot has been *considered* in
this ratio order, every column considered after it must reason about the worst case *including*
the possibility that this one ends up granted its push, whether or not it *was* — a column left
undetermined might still take any value up to `ub_j` in the eventual LP solution.

### 鏡像の場合を変数変換で帰着させた理由 (原文)

Rather than re-deriving a second, easily-miscrossed set of `Ũ`/`L̃` formulas and branch conditions
from scratch, this module reduces the mirror case to the one above by the substitution
`y_j = ub_j - x_j` applied to every such column in the row at once. Running the identical
[`stuffing_core`] on this transformed row and translating its results back is a pure change of
variables — no separate derivation to get subtly wrong, and no second implementation to keep in
sync with the first if either is ever revisited.

### 対象外とした範囲の理由 (原文)

`lb`/`ub` for every *other* column in the row may still be infinite — only ever contributing a
consistent `+inf` to `Ũr` or `-inf` to `L̃r`, never both in the same running sum, so no
`inf - inf` ever arises there. A candidate with an infinite bound has no finite `alpha`/`beta`
baseline to reason about, and — since it is a genuinely unbounded column, dualfix's own
unconditional fix would already have caught the favorable-sign case — the unfavorable-sign case
with an infinite bound is simply left to whatever bound-tightening or the LP solve itself resolves
it into. The paper's own algorithm is a fixing procedure, not a bound-strengthening one.

Both cases run independently per row using each column's real, current `lb`/`ub` for every column
*not* in the case currently being decided — including a same-row column that belongs to the
*other* case. This costs a little potential extra reduction on the rare row with columns of both
signs, in exchange for each case's own result never depending on which order the two are
evaluated in.

## src/presolve/dominatedcol.rs

### モジュール全体: 動機と設計判断の原文

Generalizes [`dualfix`]'s own zero-lock special case — there, a variable is fixed only by comparing
it implicitly against the all-zero "column" (no real row resists moving it at all, i.e. its lock
count on one side is `0`); here, any *other* real column can play that role, catching a variable
`dualfix` alone never would because some row genuinely does resist moving it — just not as much as
it resists moving some other column.

An equality-locked variable can't be pushed either direction without a compensating move
elsewhere, which this pass doesn't attempt to find. When *both* escape routes are open the shift is
unbounded in the direction that only helps the objective — a real unboundedness the LP has
regardless, not a reduction this pass should paper over by guessing a side; when *neither* is open,
only a partial bound tightening is available, which — like `dualfix` and `doubleton` — this pass
leaves alone rather than folding a second reduction kind into one.

Candidate search: the same "let the cheapest available row bound the search" idea [`sparsify`]'s
own `anchor` uses, just picked per-column here instead of per-equality-row there. A domination this
misses is left for a later round's structural change to expose, not chased down exhaustively.

Single-pass rule: a column committed as *either* side of one fix this call is never reused as
either side of a second fix in the same call, even though reusing it purely as an *anchor* again
would often still be sound (its own freedom to absorb a shift isn't used up by lending it to one
fix). The conservative blanket rule is simpler to prove correct outright: once a column might
itself end up fixed this same pass, letting it also serve as the premise for fixing something else
risks the same kind of stale-assumption bug `doubleton`'s own `claimed` guard exists to rule out —
any opportunity this costs is picked up by the next round instead.

### 未統合 (無効) にした経緯 (原文)

**Implemented, unit-tested, measured against the full Netlib set — then left unintegrated (kept
here, tested, but never called from [`crate::presolve::run_extended`]), mirroring [`sparsify`]'s own
precedent.** The two escape-route conditions genuinely need a *literal* infinity, not merely "a
wide finite range" — a finite range, however large, can always be defeated by an adversarial choice
of the *other* variable's own starting point within *its* range: e.g. with `j` ranging over `[0,3]`
and `k` over `[0,10]` (`k`'s range wider), an optimal solution sitting at `x_j=0, x_k=0` has *zero*
room to shift either way, even though `range_k >= range_j` — a tempting but unsound generalization
this module's own history once tried and caught before shipping. Only an unbounded side sidesteps
this (infinity beats any finite worst case unconditionally), which is also exactly why `dualfix`'s
own zero-lock case needs no escape route at all: comparing against the *implicit* all-zero column
never needs the zero column itself to move, so there is nothing for an adversarial starting point to
defeat.

That requirement collides with an invariant the rest of this crate enforces outright: every
variable must have two *finite* bounds (`model.rs::add_variable` rejects `+/-inf` bounds at the API
boundary; see `presolve.rs`'s own "bounded-variable invariant" docs on [`super::build_a_g`]) — so
neither `ub[j] == f64::INFINITY` nor `lb[k] == f64::NEG_INFINITY` can ever be true for any model
this crate can actually build, and this pass's two fix branches are unreachable in practice, not
merely rare. Confirmed empirically, not just argued: instrumented and run across all 73 solvable
Netlib/HiGHS-comparison instances (`python/enomoto_solver/benchmark_highs.py`), this pass fixed
zero variables on every single one (consistent with every instance's `+/-inf` bounds already having
been substituted with a large finite `BIG_M` before reaching this crate — itself downstream of the
same finite-bounds invariant), while still costing roughly 5-8% of this crate's own total solve
time in pure candidate-search overhead if wired into [`crate::presolve::run_extended`] (aggregate
Netlib total: 3.67s unwired vs. 3.88s wired, `ours`-side only; the `ours`-vs-HiGHS ratio itself
stayed within run-to-run noise, ~2.02-2.03x either way). Kept here for its correct, tested core
logic — e.g. if this crate's finite-bounds invariant is ever relaxed — rather than deleted outright.

(注: 上記の「全変数が有限境界を持つ」不変条件は執筆当時の記述であり、現在のコードベースでも
成り立つかは本メモ作成時に再確認していない。)

## パラメータについて

上記 12 ファイルには、`params::presolve` に既にある定数 (`TOL`, `SMALLCOEFF_EPS`,
`CUMULATIVE_FRACTION`, `NOISE_THRESHOLD`) 以外の調整用数値リテラルはなかった
(残っているのは 0.0 / 1.0 / 符号 / ハッシュ用乗数などの構造的な定数のみ)。そのため
`src/params.rs` への新規定数の追加はない。
