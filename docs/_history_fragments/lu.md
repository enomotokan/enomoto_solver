# 改良履歴メモ: 疎 LU 分解 (src/simplex/lu.rs) と params::lu

コード中にあった開発経緯・測定値・撤回した試みなどのコメントを移したもの。本文は元の英語のまま (重複のみ軽く整理)。

## src/simplex/lu.rs

### モジュール冒頭 (//!)

- Degree-list implementation history: row/column degrees live in bucket arrays with O(1) bucket moves via a parallel position index (swap-to-last-then-pop on removal — the same pattern `factorize`'s earlier `active_rows` bookkeeping used). The column-major mirror is what makes the whole scheme actually sub-`O(m)` per step rather than just relocating the same cost: "which rows does eliminating column `pj` affect" is answered by that column's own live-row list directly (cost = that column's own current degree) instead of scanning every active row to test whether it still holds `pj`, and "how many rows still touch column `j`" is that list's length (O(1)) instead of a fresh full-matrix scan. Both the submatrix and its mirror are flat arrays ([`KernelMatrix`], HiGHS's own `HFactor` `mc_*`/`mr_*` layout), not `BTreeMap`/`BTreeSet` containers.
- **Bug history**: An earlier version of this file computed row/column degrees this way but then performed the actual elimination arithmetic *directly* in `factorize`'s main loop, ahead of a separate `eliminate_column` method that was supposed to update the bucket state — since that method detected "which rows changed" via `contains_key(&pj)`, and the earlier direct arithmetic had already removed `pj` from every affected row first, `eliminate_column` always found nothing to do. Bucket degrees then stayed frozen at their *initial* values for the rest of the factorization while the underlying matrix kept changing underneath them, which didn't corrupt the arithmetic (pivot values are always read fresh from `rows`) but could starve `find_best_pivot` of a candidate it should have found, surfacing as a spurious "singular" `None` on matrices that are not actually singular (confirmed: this crate's own HiGHS cross-check benchmark, which the prior, non-bucketed `factorize` solved without issue, started panicking at `n=2000` with exactly that message). The fix: elimination is a single method (`eliminate`) that does the arithmetic *and* the degree/bucket bookkeeping together, and a row being retired as a pivot removes it from every other column's live-row list too (not just its own pivot column's), so no column's degree can drift stale by continuing to count an inactive row.
- Per-pivot cost is `O(pivot column's degree)` for the elimination itself plus `O(fill touched)` for the resulting degree/`col_max_abs` refresh — though `find_best_pivot`'s bucket scan can still fall back to examining more candidates on a poorly-conditioned or unusually dense step; not a hard worst-case guarantee, just a much smaller constant than rescanning the whole active submatrix every step.
- Only FTRAN's entering-column solve gets the full Gilbert-Peierls sparse treatment — see `LuFactors::l_solve_sparse_into`'s history for why.

### thread_local PIVOT_THRESHOLD

Thread-local rather than a field threaded through `factorize`'s half-dozen entry points (and their callers in `simplex.rs`, `extended_dual.rs`, `mip.rs`) because it is a *solve*-scoped setting in exactly the way HiGHS's own `info_.factor_pivot_threshold` is: one value, read once per factorization, written only by the simplex loop that owns the solve. Thread-local (not a `static`) keeps concurrent solves — `mip.rs` runs LP relaxations on rayon workers — from escalating each other's thresholds, which a shared global would do while also making both solves' pivot sequences depend on the interleaving. Every solve entry point calls `reset_pivot_threshold` before its first factorization, so a thread that ran a troublesome solve does not hand the escalated value to the next solve scheduled onto it.

### 関数 pivot_threshold_base

`ENOMOTO_PIVOT_THRESHOLD` is how the A/B behind `STABILITY`'s own value is produced without a rebuild. Read from the environment once per process, not once per solve: a solve that re-read it would pay a `std::env::var` lookup inside the very loop this section is trying to speed up.

### 関数 pivot_threshold

Read once per factorization, never per elimination step — a factorization that read it per step could see it change underneath itself only if the simplex loop ran concurrently with its own factorization, but reading it once also keeps the whole factorization's pivot sequence a function of one scalar, which is what makes a given solve reproducible.

### 関数 escalate_pivot_threshold

This is `docs/lu_comparison_enomoto_vs_highs.md` §2.4's "loosen/tighten the stability floor when the problem is ill-conditioned", in HiGHS's own direction: a solve that keeps *failing* numerically (Forrest-Tomlin updates rejected, `x_B(M)`/`d` drifting away from the true basis, pivots grossly inconsistent with the factorization) is one whose factorizations are too permissive, so the floor goes **up**, buying stability with fill-in. Lowering it on trouble would be the wrong sign: it is exactly the marginal pivots a lower floor admits that produce the eta chains these triggers are catching.

Monotone within a solve, like HiGHS's `info_.factor_pivot_threshold`: nothing lowers it again short of `reset_pivot_threshold`. A ratchet that also relaxed would make "how many troublesome iterations ago" part of the pivot sequence, and the extra state buys nothing measurable — the ladder is one step wide.

**Nothing calls this by default.** Wiring it to the numerical-failure triggers cost +7.4% over NETLIB93; see `PIVOT_ESCALATION_STEP` and `analysis/pivot_threshold_colfixmax_20260922_154500.md` for the measurement, and `ENOMOTO_PIVOT_ESCALATION_STEP` to re-enable it.

### 関数 pivot_search_limit

`0` restores the unbounded scan, which is how the A/B behind the constant's own value is produced. Read once per `MarkowitzState::new` (i.e. once per factorization), never per elimination step: the read is `find_best_pivot`'s own caller-side cost otherwise, paid `m` times per factorization, and an `std::env::var` lookup there would show up in the very measurement this gate exists to make.

### 静的変数 PROF_TOTAL_STEPS / PROF_TRIVIAL_STEPS

Measurement counters for the "should `factorize` triangularize `A_B` into a trivial part plus a smaller Markowitz bump before factoring, the way production codes like HiGHS do" question, read back by `simplex.rs`'s `ENOMOTO_PROF_TRIANGULAR`-gated diagnostic. Answered by measurement rather than by adding the pre-pass speculatively: on every Netlib instance checked (`ganges`, `ship12s`, `stocfor2`, `fit1p`), `find_best_pivot`'s bucket-based early exit already resolves 90-100% of pivots as score-0 "trivial" ones, and cumulative time inside `find_best_pivot` across the *entire* solve was under 0.2% of total solve time in every case — the pivot *search* was never the bottleneck a dedicated triangularization pre-pass would speed up, so one was not added. Kept as a live diagnostic (not deleted) in case a future problem shape changes that picture — and it did: the "under 0.2%" figure above holds only for the four small instances it was measured on. Re-measured across the whole set for `PIVOT_SEARCH_LIMIT`, `dfl001` spent 2.96s of its 22.0s solve (13%) inside `find_best_pivot`, at an average scan width of 261 candidate columns per elimination step — the search *was* a real cost there, just not on problems small enough for the original sample. A triangularization pre-pass still isn't what that calls for (the bound in `PIVOT_SEARCH_LIMIT` addresses it directly, taking the same problem's scan to 0.22s), but the 0.2% claim should not be quoted as if it covered the large instances.

### 静的変数 PROF_BUCKET_SCAN_NS

The step/candidate counters around it are always kept, accumulated per factorization and flushed once from `MarkowitzState`'s `Drop`.

### 静的変数 PROF_DENSE_FALLBACK_STEPS

A direct measurement of how often the dense-column-avoidance heuristic in `factorize` actually gets exercised (as opposed to every dense column simply never coming up as a candidate at all, in which case this stays at `0` and the heuristic is a no-op for that problem).

### 静的変数 PROF_SEARCH_LIMIT_STEPS / PROF_SEARCH_CANDIDATES

`PROF_SEARCH_LIMIT_STEPS` counts early returns because of `PIVOT_SEARCH_LIMIT` (as opposed to the score-0 exit, the per-degree-level `merit_limit` exit, or a full scan) — the direct measurement of how often the bound is exercised at all, without which a flat benchmark result can't be told apart from a no-op. `PROF_SEARCH_CANDIDATES` is the quantity `PIVOT_SEARCH_LIMIT` bounds per call; read against `PROF_TOTAL_STEPS` it gives the average scan width per step, which is what the bound is supposed to move.

### 静的変数 PROF_PIVOT_ESCALATIONS

Without this, a flat benchmark on the §2.4 escalation can't be told apart from one where the ladder never fired at all.

### 静的変数 PROF_COLMAX_RESCAN_ENTRIES

Entries `ensure_col_max_abs` walked to un-stale the columns `find_best_pivot` actually read — the total work an incremental `colFixMax` (`docs/lu_comparison_enomoto_vs_highs.md` §2.4) could have removed, and the reason removing it lost: since §2.5's `PIVOT_SEARCH_LIMIT` bounds a single search to 8 candidate columns, this is already a small fraction of the per-entry bookkeeping such a scheme costs in `eliminate` (measured in `analysis/pivot_threshold_colfixmax_20260922_154500.md` §2). Counted per rescan, not per touched column, so it stays off the elimination loop's own path.

Since `find_best_pivot` recomputes a stale max only when some entry of the column has already passed the Markowitz-score filter (see the lazy-`col_max_abs` note there; `ENOMOTO_LU_LAZY_COLMAX=0` restores the eager rescan), this counts only the rescans that were actually needed: `pilot87` 14.2M -> 9.2M entries, `d2q06c` 258K -> 223K, same pivots.

### 構造体 KernelMatrix

Flat, HiGHS-`HFactor`-style storage for the active submatrix that Markowitz elimination works on — the replacement for the `Vec<BTreeMap<usize, f64>>` (rows) + `Vec<BTreeSet<usize>>` (column mirror) pair `MarkowitzState` used to hold directly, and the item `docs/lu_comparison_enomoto_vs_highs.md` §3.1 flagged as the largest remaining structural gap against HiGHS's own kernel (`mc_*`/`mr_*` flat arrays with in-place insert/delete, against tree nodes scattered across the heap and an `O(log d)` traversal per element touched).

Layout mirrors HiGHS's `mc_start`/`mc_space`/`mc_count` + `mr_start`/`mr_space`/`mr_count` pattern with the two axes swapped — this crate's elimination is row-oriented (it scatters the pivot *row* into every affected row), where HiGHS's is column-oriented, so the *values* live row-major here and the index-only mirror is the column one. Column mirror indices are `u32`: two rows per cache line's worth of what `usize` would cost, and the ascending-degree bucket scan in `find_best_pivot` reads these runs end to end. The mirror only answers "which rows are live in column `j`", exactly as `col_rows` did.

Both runs are kept *sorted*, rather than taking HiGHS's cheaper swap-with-last unordered sets. That is a deliberate extra cost — a `copy_within` over a contiguous, usually single-digit-length run, still far cheaper than the tree traversal it replaces — and it buys exact behavioural equivalence with the ordered containers it replaces: `find_best_pivot` resolves Markowitz-score *and* pivot-magnitude ties by first-encountered, `eliminate` emits its `L` multipliers and refreshes touched columns in iteration order, and LP basis matrices are full of exactly-tied `±1` coefficients. An unordered mirror would therefore silently select different pivots on real Netlib instances, changing the factorization, the iteration counts, and hence what a before/after benchmark of *this* change is actually measuring.

- Field `row_idx`/`row_val`: Row runs, structure-of-arrays. Splitting the `(usize, f64)` pairs this held before means a column lookup (`row_get`, run ~12M times per `pilot87` solve by `find_best_pivot`'s lazy `col_max_abs` rescans) and the index side of `eliminate`'s scatter loop stream 4 bytes per entry instead of 16.

### 関数 KernelMatrix::new

- Capacity, not length: the reserve (`2 * total + 64`, now `KERNEL_RESERVE_MULT`/`KERNEL_RESERVE_EXTRA`) is here so the relocations (`ensure_row_cap`'s, later) stay `resize` inside one allocation instead of repeatedly reallocating and copying the whole buffer. Nothing is *initialized* beyond what is actually written.
- Stable sort by column, then accumulate each duplicate run in input order and drop exact zeros — bit-for-bit what the `*entry(j).or_insert(0.0) += v` + `retain(|_, v| *v != 0.0)` construction this replaces produced, summation order of repeated coordinates included.
- No up-front slack (`cap == len`): a row only ever needs to grow when the merge in `eliminate` leaves it *net* longer, and the pivot column's own entry always leaves at the same time, so one fill-in still fits in place and only two or more relocate. Pre-padding every row instead cost a memset proportional to the padding on *every* refactorization, including for the many rows that never take fill at all — worst of all on a near-slack basis, where `nnz ~= m` makes a flat few-entries-per-row pad several times the size of the real data. `ensure_row_cap` doubles from here, so a row that keeps taking fill still relocates `O(log)` times, not once per insertion.

### 関数 KernelMatrix::row_get / col_insert / col_remove

- `row_get` is the direct stand-in for `rows[i].get(&j)`.
- `col_insert`: `eliminate` walks its affected rows in ascending order, so the insertion point is typically at or near the run's tail and the shift is short.
- `col_remove` is a no-op when absent, matching `BTreeSet::remove`'s own tolerance.

### 構造体 ElimScratch

Instead of allocating a fresh `Vec`/`BTreeSet` per step.

### 構造体 MarkowitzState

- Field `col_max_abs_dirty` history: Most columns `eliminate` dirties get dirtied again by a later elimination step before `find_best_pivot` ever visits them (a column's bucket position, which *is* updated eagerly by `update_col_degree`, is what determines when that happens), so eagerly recomputing every dirtied column's max was mostly wasted work — up to 75% of it, measured on Netlib `greenbea`.
  Marking, rather than maintaining, is also what `docs/lu_comparison_enomoto_vs_highs.md` §2.4's incremental `colFixMax` was measured against and beat. That variant kept `col_max_abs[j]` exact through `eliminate` — raise it on a fill-in or a growing value, mark stale only when the entry that *was* the max shrank or left — which dropped the stale fraction to 0.1-6% of touched columns and left every pivot choice bit-identical. It still lost, by 2.8% over NETLIB93: since §2.5's `PIVOT_SEARCH_LIMIT` bounds one search to 8 candidate columns, the rescans it removed were already small (`pilot87`: 1.9M entries) against the per-entry bookkeeping it added in the merge loop (61.5M updates, each a scattered read-modify-write into an `m`-sized array). See `analysis/pivot_threshold_colfixmax_20260922_154500.md` §2.
- Field `initially_dense`: fixed at construction time and never updated, deliberately: a truly dense column's *current* degree keeps shrinking as unrelated rows get eliminated as pivots for *other*, sparser columns (each such row leaving the basis removes it from every column's live list, including this one's) — dropping into a low bucket only because its rows happened to get cannibalized elsewhere, not because it stopped being structurally dense. Thresholding on the live, shrinking degree would let `find_best_pivot`'s ordinary ascending-bucket scan pick such a column early anyway, right when it looks artificially sparse — exactly the case this field exists to still catch. A pivot on a column with `d` remaining active rows scatters the entire pivot row's pattern into all `d` of them in one step (`eliminate`'s `affected` list), so pivoting on a column that is dense *by original structure* — even at a reduced current degree — is still the single most expensive kind of step Markowitz pivoting can take.
- Field `search_limit`: `0` means "unbounded", the pre-§2.5 behaviour.
- Field `threshold`: this factorization's whole pivot sequence is deliberately a function of one scalar captured at its start, rather than of a value the simplex loop could raise part-way through.
- Field `prof_colmax_rescan_entries`: an `AtomicUsize::fetch_add` per rescan would be a locked read-modify-write on the factorization's own path, and would make two threads factorizing at once contend on one cache line.
- Field `row_singleton_rel` (`ENOMOTO_LU_ROW_SINGLETON`): Measured motive: numerically rejected row singletons (they fail the `0.25` threshold) persist across many steps and every search keeps paying for the columns in front of them. `rel = 0` is HiGHS's own rule (no threshold on singletons). A row singleton's pivot row has no other active entry, so it causes no fill and no update of the active submatrix; its only numerical cost is the size of the `L` multipliers `a_kj / v`. Never applied in `factorize_bordered`'s sparse phase (reset right after `MarkowitzState::new` there): there the row's border entries *are* updated, through the Schur complement, by exactly those multipliers — measured as a wrong `fit2p` objective (`-8.9e117`) when it was.
- Field `row_search` (`ENOMOTO_PIVOT_ROW_SEARCH`, B2(b)): Short rows find small Markowitz scores early, so the per-level exit `best_score <= c^2` fires sooner on matrices whose columns are short but whose rows are long (`dfl001`). A long row costs a `col_max_abs` rescan for most of its columns, which on a matrix with a dense tail (`pilot87`) outweighs what the search saves.

### thread_local BUCKET_POOL

So a refactorization does not re-allocate `2(m+1)` bucket headers and the buckets' own buffers every time.

### 関数 MarkowitzState::refresh_column / ensure_col_max_abs

Marks `col_max_abs[j]` stale rather than recomputing it here; see `col_max_abs_dirty`'s history for why, including why the incremental alternative (`docs/lu_comparison_enomoto_vs_highs.md` §2.4's `colFixMax`) was measured and rejected. `ensure_col_max_abs` is called from `find_best_pivot` right before it reads `col_max_abs[j]`, the one place that value's currency actually matters.

### 関数 MarkowitzState::find_best_pivot

- Only that column's actual active rows (via the column mirror, not every row at that row-degree) are examined — this is the other half (alongside `eliminate`'s use of the same mirror) of what keeps the search from degrading into a full active-submatrix scan. The per-degree-level early exit is a standard practical relaxation (as in production Markowitz implementations): it does not guarantee the globally minimal Markowitz count, only that no further search will find something clearly better — finding the exact minimum every step is itself more expensive than the fill-in it would save.
- `skip_dense`: `factorize`'s caller tries this first and only falls back to a second, unrestricted call if it finds nothing, so a truly-required dense pivot (or a genuinely singular matrix) is still handled correctly, just not preferred.
- `PIVOT_SEARCH_LIMIT` — unlike the per-degree-level exit — can fire part-way *through* a bucket, and so is what actually bounds a single call's cost when one degree level holds hundreds of columns.
- Timing only under profiling gates: two `Instant::now()` calls per elimination step are otherwise pure overhead on the hot path.
- Row singleton path: opt-in, path-changing experiment, HiGHS `buildKernel` step 1.2-style.
- Buckets never hold a used column: `factorize` removes the pivot column from its bucket in the same step it marks it used, and `update_col_degree` refuses to re-insert one. Bucket membership only ever changes via `update_col_degree`/`remove_from_bucket_col`, never from inside `find_best_pivot`.
- Lazy `col_max_abs`: `col_max_abs[j]` is only ever read to form `min_pivot`, and `min_pivot` is only ever read for an entry that has already passed the Markowitz-score filter — so a stale max is recomputed lazily, at the first such entry, rather than up front for every column the scan visits. Most visited columns never produce one (their rows are all too long to beat `best_score`), and for those the rescan — a `row_get` per column entry — was pure waste. `min_pivot` is the same product of the same two numbers either way (HiGHS's `mc_min_pivot[j] = max_value * pivot_threshold`, §2.4), so every comparison — and hence the chosen pivot — is unchanged.
- Score filter skip: this is the vast majority on a matrix with heavy fill-in.
- The search limit is checked after this column's own scan (never before it), so the limit bounds how many columns are examined rather than cutting one short mid-way: the `best` a truncated column produced would otherwise depend on `col_rows`' iteration order in a way the unbounded scan's doesn't.
- Profiling: accumulated in plain fields and flushed to the shared atomics once per factorization (`Drop`) rather than with three or four locked read-modify-writes per elimination step.

### 関数 MarkowitzState::eliminate

- Splitting arithmetic and bookkeeping into separate steps (as an earlier version of this file did) is unsound: bookkeeping keyed off "did this row still contain `pj`" only works if it runs *before* `pj` is actually removed.
- Each affected row is rewritten by a **single sorted merge** of its own run against `pivot_row_snapshot` (both ascending by column), rather than by one keyed lookup per pivot-row entry: with the `BTreeMap` rows this replaced, this inner loop — the hottest in the whole factorization, run once per `(affected row, pivot-row entry)` pair, every elimination step — cost `O(d_p log d_i)` tree descents into scattered heap nodes; the merge costs `O(d_i + d_p)` over two contiguous, sequentially-read runs and one sequentially-written one. That is `docs/lu_comparison_enomoto_vs_highs.md` §3.1's point (HiGHS's `mc_*`/`mr_*` flat arrays against this crate's tree nodes) applied to the one loop where it matters most.
- The common path works HiGHS-style off a dense scatter of the pivot row (`ElimScratch::wval`) instead of a two-pointer merge: with no data-dependent branch per entry — the merge mispredicted on nearly every interleaving of the two patterns. Values, row order and every degree/bucket update are exactly the merge's, so the factorization is bit-identical.
- No per-entry column "touched" bookkeeping: every column whose degree or values this step can change is a column of the pivot row (an update or a fill-in lands only there), and the pivot row is retired from all of those columns anyway, so refreshing the pivot row's columns in ascending order is exactly the sorted, deduplicated touched set the per-entry stamping used to build.

### 構造体 GpScratch

One instance lives for as long as its caller's own dedicated sparse-solve buffer does (`solve_lp_dual_on` creates one, alongside a `z` buffer used *only* for this path — never shared with a plain `FtLu::solve_into` call's own `scratch`, per `FtLu::solve_sparse_into`'s own docs on why that separation matters — before the pivot loop starts, and reuses both every FTRAN).

`visited` history: It was a hand-rolled `Vec<u32>` plus a bare `epoch += 1` until that trick was consolidated into `crate::sparse`; the bare increment had no wraparound guard, so after 2^32 calls a stale stamp would have read as a live mark and silently truncated a reach set — i.e. produced a wrong FTRAN. `EpochMarks::begin` handles it. `stack`: iterative, not recursive — this crate's basis matrices can have `m` in the low thousands, deep enough that a recursive DFS risks a real stack overflow on a long dependency chain.

### 構造体 LuFactors

- Field `l_col`: Stored as a `crate::sparse::CscMat` — one flat `(index, value)` buffer plus offsets, the same layout `simplex.rs`'s `StdForm` uses for the frozen coefficient matrix, on the same reasoning: `L` never changes once a refactorization builds it, and it is then read on every FTRAN/BTRAN's `L`-stage for the rest of that basis's life.
  **This is the second attempt, and the first one that measured as a win.** The first flattened a finished `Vec<Vec<(usize, f64)>>` into the compressed form as a post-pass, and a controlled A/B showed a consistent small regression on every instance that moved at all (`scsd8` +2.1%, `25fv47` +1.8%, `stocfor2` +4.0%, `fit1p` +3.2%, `degen3`/`pilotnov` flat, nothing faster). Its own post-mortem identified the reason and named the fix: the post-pass *keeps* building the `m` small per-column `Vec`s it was meant to remove and then adds an `O(nnz)` copy on top, so it paid the compressed form's cost — two offset reads per column access, against `Vec<Vec>`'s single pointer hop to an already-known `(ptr, len)` — while buying none of its benefit.
  `crate::sparse::CscBuilder` is that fix. Every one of this file's factorizations already emits `L`'s columns in ascending step order (left-looking elimination produces column `s` complete at step `s`), so the flat buffer can be appended to directly, with the column boundary recorded wherever the buffer has reached: no counting pass, no per-column `Vec`, and no copy. What is left is a strict improvement at build time (two allocations for the whole of `L` instead of `m + 1`) plus contiguous entries for `l_solve_into`'s own sequential `for s in 0..m` sweep to prefetch through.
- Field `u_row`: flattening it would cost the same construction work for no repeated-read benefit.
- Field `l_row`: This is HiGHS's own `lr_start/lr_index/lr_value` (`HFactor.h`, built by `buildFinish()` right beside the column-major `l_start/l_index/l_value`), and it exists for exactly the reason HiGHS builds it: `L^{-T}` (BTRAN's tail) is a *gather* when read through the column-major `l_col` — step `s` reads one `w[row_step]` per `l_col[s]` entry, so no single value's zero-ness makes the step skippable (Hall & McKinnon 2000 §4.4's own observation, which `l_transpose_solve_into`'s pre-`l_row` form was stuck with) — but the very same triangular solve becomes a *scatter* when read through this mirror: step `s` multiplies the single value `w[s]` into every `l_row.row(s)` entry, so `w[s] == 0.0` makes the whole step a provable no-op, exactly the skip `l_solve_into`/`u_solve_into` already have in the forward direction. Flat (`CsrMat`: two allocations, offsets + entries), built directly by `CscMat::to_csr`'s counting sort (one pass to size each row's slice, one to fill it), with no intermediate `Vec<Vec<...>>` on either side.

### 関数 factorize_dense_faer

See `DENSE_INPUT_FRACTION`'s history for why this exists instead of running Markowitz on an already-dense matrix.

### 関数 debug_print_block_sizes

Throwaway diagnostic, not wired into any production path: measures what block-size distribution a Dulmage-Mendelsohn SCC decomposition of *this* refactorization's basis matrix would actually have, to check a specific hypothesis about the previously-reverted block-triangularized `factorize` (see `factorize`'s history) — namely, whether the blocks it would find are mostly tiny (say <=10 or <=50 rows), which would matter for a proposal to special-case small blocks with a dense/product-form solve instead of the general sparse Forrest-Tomlin machinery.

### 関数 detect_border_columns

The same "near-fully-dense trend/regression column" shape `MarkowitzState::initially_dense` already detects internally, exposed here as a free function so `factorize`'s routing decision can check it before paying for a `MarkowitzState` at all — this is the only extra cost non-bordered instances pay: one `O(nnz)` degree pass, measured (see `BORDER_MAX_FRACTION`'s history) to be cheap enough to run unconditionally rather than gated behind a flag.

### 関数 factorize_diagonal

`simplex.rs`'s initial-basis call sites use this instead of paying for pivot selection, fill-in bookkeeping, and border/dense-input detection on a matrix that has nothing for any of that to do. Shape check first, with no allocation: every mid-solve refactorization tries this and almost always fails, and building `u_row` row by row until the first non-diagonal row paid one small allocation per leading diagonal row for nothing.

### 静的変数 PROF_REBUILD_*

Read back by the `ENOMOTO_PROF_PHASES_EXT` diagnostic: a rejected attempt is pure overhead paid on top of the full Markowitz factorization that follows, so the accepted/attempted ratio is what decides whether the reuse pays for itself. `PROF_REBUILD_ROW_REPICKS` is bounded below by the number of basis columns the Forrest-Tomlin updates replaced since the order was recorded, and is the reason the row order cannot simply be replayed the way the column order can. `PROF_REBUILD_ACCEPTED_NNZ`/`PROF_FULL_NNZ`/`PROF_FULL_COUNT`: the diagnostic behind "is a reused order producing factors every later FTRAN/BTRAN then pays for".

### 関数 reuse_fill_limit / reuse_pivot_order_enabled

`ENOMOTO_REUSE_FILL_LIMIT` lets the one number be re-tuned against the Netlib set without a rebuild — and, more to the point, flipped *within one process* for an A/B. `ENOMOTO_REUSE_PIVOT_ORDER=0` exists so an A/B of this feature can flip it *within one process* — per `analysis/ftran_density_gate_20260922_062832.md` §4.1, this box's per-problem run-to-run spread across separate processes reaches 4x, far wider than the effect being measured. Read once per refactorization (a handful of times per solve), never per iteration.

### 関数 factorize_reusing

This is this crate's counterpart to HiGHS's `HFactor::build()` trying `rebuild()` (`util/HFactorRefactor.cpp`) before `buildSimple()` + `buildKernel()`, named as gap §2.2 in `docs/lu_comparison_enomoto_vs_highs.md`: a mid-solve refactorization factorizes a basis that differs from the last factorized one only by the columns the Forrest-Tomlin updates since then replaced, so the order Markowitz chose last time is usually still a good order — and *applying a known order* costs only the elimination's own arithmetic, with none of the search, degree bookkeeping, or active-submatrix maintenance (`MarkowitzState`'s `BTreeMap`/`BTreeSet` churn) that choosing one costs.

- Border columns/dense test computed once rather than recomputed by `wants_bordered` and again by `factorize` (each an `O(nnz)` pass).
- A dense input goes to `factorize_dense_faer` regardless of any pivot order, and the bordered path wants its own ordering — reuse targets the ordinary sparse Markowitz case, which is every mid-solve refactorization on a real Netlib basis.
- Backoff: A rejected attempt is wasted work on top of the full factorization that follows it, and rejections cluster: the basis that produced one (too much fill under the recorded column order, or a numerically spent order) is usually still producing them a few refactorizations later. So back off exponentially — 2, 4, 8, ... refactorizations left alone, capped at `REUSE_MAX_BACKOFF` — and reset to zero on the first acceptance, which is what keeps a problem where reuse *does* work paying nothing for this.

### 関数 wants_bordered

Reuse must not take a `fit1p`/`fit2p`-shaped basis: the bordered path's whole point is to keep ~20-25 near-dense "trend" columns *out* of the sparse elimination entirely, and a plain left-looking pass over the order it produced scatters exactly those columns back through every step — measured as `fit2p` +6.4% at a 1.1 fill limit and +16.3% at 1.25, against roughly flat everywhere else, which is what sent this gate in.

### 関数 factorize_reusing_order

- Left-looking: Nothing here searches for *sparsity*, and nothing maintains an active submatrix; per-step cost is the elimination arithmetic plus the reach-set heap, both bounded by the factors' own nonzero count.
- **Why only the column order is replayed, not the row order.** HiGHS's own `rebuild()` replays both (`refactor_info_.pivot_row` / `pivot_var`) and gives up — rank deficiency, full rebuild — the moment a recorded pivot row's entry is too small. That is affordable *there* because HiGHS only ever sets `refactor_info_.use` for a hot start (`HEkk::setNlaRefactorInfo`), i.e. when re-factorizing the very basis the order was recorded from, where the recorded rows trivially still work. Replaying both orders across a *changed* basis was implemented here first and measured: it is rejected essentially always (Netlib `25fv47` 0/26 attempts, `degen3` 0/7, `pilot` 0/30, `fit2p` 0/32, `greenbea` 1/28), and the rejections are overwhelmingly "the recorded pivot row is numerically *empty*" (`fail zero`, not `fail stability`) — which is exactly what a replaced basis column looks like: the entering column has no reason whatsoever to be nonzero at the row that was pivotal for the column that left. The column order is what Markowitz's fill-minimization actually encodes; the row assignment is a numerical choice, and re-making it per step (partial pivoting: take the largest remaining entry) costs one pass over the column that has already been computed.
- Threshold read once: this path reuses the previous factorization's *column* order but still picks each pivot row under the same threshold test, so an escalated threshold has to reach it too (a rebuild that kept the old, looser floor would quietly undo the escalation for as long as the pivot order keeps being reusable).
- Column-major copy: the same shape `CscMat::from_rows` builds, kept local because this one is indexed by *basis slot* and thrown away when the factorization is done.
- `row_remaining`: Maintained in `O(nnz)` total by decrementing a column's rows as that column is consumed, and used only to break the tie among numerically acceptable rows when the recorded one is unusable: with the column order fixed, the row choice is all that is left to keep fill down, and taking the absolutely largest entry (plain partial pivoting) ignores sparsity entirely.
- Forward solve: ascending step order is a valid topological order — the same property `l_solve_sparse_into` relies on, reached here with a heap rather than a DFS because the graph is still being built.
- Row re-pick: The recorded row is gone (a basis column the Forrest-Tomlin updates replaced leaves its old pivot row numerically empty here). The fewest-entries rule is the surviving half of a Markowitz count once the column is fixed.

### 関数 factorize (旧 doc コメント中の履歴: Dulmage-Mendelsohn 分解の試行 2 件)

(These two paragraphs were originally attached as doc comments to `params::lu::DENSE_INPUT_FRACTION` / `BORDER_MAX_FRACTION` after the constant move, but describe `factorize`.)

**A Dulmage-Mendelsohn block-triangularized variant of this function was implemented, thoroughly validated, and measured — then reverted**: rows were partitioned into strongly-connected blocks (via bipartite matching + Tarjan SCC, `crate::graph::dulmage_mendelsohn_blocks_topological`, which remains implemented and tested for a possible future, more targeted revisit) in topological order, each factorized independently, then reassembled via the block-LU identity `U_ij = L_i^{-1} A_ij` for "spillover" entries outside a block's own matched columns (`L` itself stays exactly block-diagonal). Implementation correctness was confirmed via unit tests (including one that caught a real bug: an initial version copied spillover entries unchanged, which is only valid when the emitting block's own `L` is trivial/identity — true for singleton blocks, which is why singleton-only spillover tests passed by coincidence before the fix) and zero objective mismatches across the full 73-problem Netlib benchmark.

**But it measured as a net ~4% aggregate regression** in a controlled back-to-back A/B (same machine, same run, only the feature toggled): dramatic wins on a few instances with genuine block-angular structure (`fit1p` -44%, `wood1p` -17%, `scsd8` -10%, `sierra`/`sctap3`/`scrs8` a few percent) were outweighed by a broad ~10-25% tax on most other medium/large instances (`grow15` +25%, `bnl1` +18%, `modszk1` +17%, `perold` +16%, `25fv47` +16%, `stocfor2` +14%, `pilotnov` +13%, `ganges` +11%) — paying bipartite-matching-plus-SCC cost on *every* refactorization, whether or not it finds anything worth exploiting. Two cheap pre-gating heuristics were tried to avoid paying that cost on instances unlikely to benefit, and both failed: (1) whether `presolve::redundancy`'s own equality-row block decomposition found structure — `ganges` decomposes beautifully there (1053 blocks, a 1% bump) yet was still a net loss here, since the *basis* matrix (all rows, reshuffled by every pivot) doesn't share the *equality system*'s (static, presolve-time-only) structure; (2) the *basis* matrix's own bump size at the first real refactorization — `stocfor2` and `ganges` again showed excellent bump ratios (0.2-1.3%, as good as or better than the actual winners) yet remained net losses, showing the fixed decomposition cost itself, not just a poor decomposition outcome, was the problem. This mirrors HiGHS's own architecture: `HFactor::buildSimple()` peels off trivial (degree-1/logical) pivots via a cheap `O(nnz)` sweep with no bipartite matching at all, leaving full Markowitz elimination (`buildKernel()`) for only the remaining kernel — this file's own bucket-based `find_best_pivot` already gets that same cheap benefit for free (confirmed earlier via `PROF_TOTAL_STEPS`/`PROF_TRIVIAL_STEPS` showing 90-100% of pivots already resolve trivially), so the *additional*, much more expensive structure genuine Dulmage-Mendelsohn decomposition can find beyond that cheap peeling isn't reliably worth its own cost. Fully reverted; see the project history around this doc comment's own commit for the full numbers if revisiting.

**A second, "peel trivial pivots then Dulmage-Mendelsohn-decompose only the remaining kernel" variant of block triangularization was also implemented, tested, and measured — then reverted.** This directly followed up the first attempt, on the hypothesis that peeling first (mirroring HiGHS's own `buildSimple()`/`buildKernel()` split) would fix that attempt's "pays matching+SCC cost on every refactorization regardless of payoff" problem by shrinking the kernel matching+SCC actually runs on. It did not: full 73-problem Netlib A/B showed a **net ~37% aggregate regression** — far worse than the first attempt's ~4%, and a regression on `fit1p` specifically (+80%), the exact instance this was meant to speed up. Root cause, confirmed by direct instrumentation: `fit1p`'s kernel (post-peel) is a single irreducible ~20-row SCC block every time, so the decomposition gate *always* rejects it and falls back to a from-scratch `factorize_flat_markowitz` call — meaning the (redundant) peel work is paid twice, for zero benefit, every refactorization. Worse, the underlying premise turned out wrong: `fit1p`'s real cost was never a large interleaved non-trivial block in the first place. `eliminate`'s cost is `O(col_rows[pj].len())` (the pivot *column*'s remaining active rows) times the pivot row's own snapshot size — a pivot with Markowitz score exactly `0` (row degree `1`, the "trivial" case `PROF_TRIVIAL_STEPS` counts) is only free when its *column*'s degree is also small; a degree-1 *row* whose sole entry sits in an otherwise-still-dense "hub" column is scored as trivial yet costs `O(hub column's current degree)` to eliminate (every other row sharing that column must be updated). `fit1p`'s basis apparently has exactly this shape — many row-degree-1 pivots landing on a handful of not-yet-thinned dense columns — which no SCC/block decomposition addresses, since those rows don't form a separable block with the hub column at all. Fully reverted (including the two dedicated unit tests that validated its spillover-reassembly correctness, which was never in question — the numerics were right, just not worth what they cost). See the project history around this comment's own commit for the full A/B numbers and the `ENOMOTO_DEBUG_BLOCK_TRIANGULAR` trace output that pinned down the root cause, if revisiting; `debug_print_block_sizes` (`ENOMOTO_DEBUG_BLOCK_SIZES`) and the `ENOMOTO_DEBUG_ELIMINATE_COST` timer remain as live diagnostics either attempt's numbers came from.

### 関数 factorize_routed

Tried *before* checking `is_dense_input`, deliberately: the measured crossover (see `BORDER_MAX_FRACTION`'s history) sits around `k/m ~= 0.5`, well past `is_dense_input`'s own 25%-of-`m^2` overall-density gate — a border-heavy input can easily cross that overall gate on the border columns' own density alone while `k/m` is still comfortably under `BORDER_MAX_FRACTION`, and in exactly that range `factorize_bordered` beats `factorize_dense_faer` too (not just plain Markowitz), so gating this attempt on `!is_dense_input` would give up a real win.

### 関数 factorize_flat_markowitz_routed

Prefer a non-dense pivot column whenever one exists: avoiding a dense pivot column matters far more than the score it happens to carry at the moment it's chosen (see `MarkowitzState::initially_dense`). `pivot_row_snapshot` is one buffer for the whole factorization, not a fresh `Vec` per elimination step. `l_entries` is already grouped by `pivot_step` in ascending order — the elimination loop emits step `s`'s whole `L` column before moving to step `s + 1` — so `L` can be appended straight into its final compressed buffer, with no counting pass and no intermediate per-column `Vec`s.

### 関数 factorize_bordered

**Why this exists**: `fit1p`-shaped Netlib instances have ~20-25 columns nonzero in essentially every row (see `DENSE_COL_FRACTION`'s history). The ordinary Markowitz path already defers pivoting *on* these columns as long as possible (`MarkowitzState::initially_dense`), but every ordinary elimination step whose pivot row still carries one of these columns' entries scatters them into every row `eliminate` touches anyway — measured (`ENOMOTO_DEBUG_ELIMINATE_COST`) as the actual cost driver behind `fit1p`'s refactorizations (average row fill climbing from `1.0` at the initial all-slack basis to `~10` a few refactorizations later, each one costing several milliseconds despite `m` only being in the hundreds). Excluding these columns from the sparse phase's own bookkeeping entirely (rather than merely deprioritizing them as pivot targets) removes that scatter cost outright; the algebra it defers is applied once, in bulk, via the classic bordered-block-diagonal LU identity.

`L_DS` (the sparse phase's own elimination multipliers for *every* affected row, border rows included — free, already computed as a side effect of the ordinary elimination). `U_SD` forward solve is structurally identical to `LuFactors::l_solve_into`, just against the in-progress `L_SS` rather than a finished `LuFactors`. The Schur complement reuses the exact same dense path already used for a globally-dense input, just at the `k`-sized scale this bordering was meant to shrink the problem down to. Falling back is exactly as the dense-column-avoidance fallback in `factorize_flat_markowitz` already does for its own `skip_dense` retry; the stuck case is a border column genuinely required as a pivot before all `m - k` sparse columns are resolved.

### 関数 LuFactors::l_solve_into (旧 doc、l_solve_into_pair の上に誤って付いていた段落)

Partial FTRAN through `L` only (step-space): solves `L z = P_row rhs`. Hyper-sparse (Hall & McKinnon, *"Hyper-sparsity in the revised simplex method and how to exploit it"*, 2000, §4.2 "Hyper-sparse FTRAN", Figure 3): `l_col[s]`'s entries only ever modify `z` by adding a multiple of `z[s]` itself — if `z[s]` is exactly zero, the whole inner loop is a provable no-op (every update is `x -= mult * 0`), so it is skipped entirely rather than paying for a test-against-zero (or worse, a real floating point op) per entry. Writes the result into caller-provided `z` (length `m`) instead of allocating — `FtLu`'s hot-path `solve_into` calls this once per FTRAN, so a fresh `Vec` here would mean a fresh heap allocation on every single pivot's FTRAN/BTRAN, several times over.

`active` (for pair/triple/single): for a basis whose `L` is mostly trivial — slack-heavy — this turns an `O(m)` zero-test scan into `O(#non-trivial columns)`.

### 関数 LuFactors::l_solve_sparse_into

A second, dense-scanning implementation (`l_solve_into`) exists rather than making this the only one because a dense `rhs` (this function's own worst case: `|reach| == m`) pays for the DFS bookkeeping (stack pushes, epoch checks) on top of the same elimination work `l_solve_into` would have done anyway with a tight double loop — this function is a net win specifically when `rhs` (and hence typically `reach`) is small relative to `m`, which is the common case for the one caller that has a genuinely sparse `rhs` on hand already (`solve_lp_dual_on`'s entering-column FTRAN: a real LP's constraint columns are themselves sparse).

Why ascending numeric order is already a valid topological order (unlike the general Gilbert & Peierls 1988 presentation for an arbitrary DAG, which needs a DFS-postorder-then-reverse): every `l_col[s]` entry's `row_step` is `> s`, by construction of the elimination itself (`factorize` only ever records a multiplier for a row not yet chosen as a pivot, which by definition gets assigned some *later* step).

Precondition rationale: `z` entirely zero on entry is *not* this function's own job to (re-)establish cheaply — its own reach set only covers what the `L`-stage itself touches, but the R-eta and `U` stages downstream (in `FtLu::solve_sparse_into`) can scatter fill well beyond that set (a long-enough eta chain can, in the worst case, touch entries across the whole vector), so knowing "the previous call's `L`-stage reach" here would not be enough to correctly re-zero what a *subsequent* stage left behind. Instead `FtLu::solve_sparse_into` unconditionally clears its own dedicated `z` buffer once, in full, right before returning — a single `O(m)` `fill(0.0)` per call, far cheaper than the branchy permute-and-scan `l_solve_into` otherwise pays, and the only `O(m)` work left in the whole sparse path.

### 関数 LuFactors::l_transpose_solve_gather_into

Unlike `l_solve`'s forward pass, a single step `s` here can read from *several* `w[row_step]` entries (one per `l_col[s]` entry), so there is no single value whose zero-ness makes the whole step a no-op — matching Hall & McKinnon §4.4's observation that BTRAN's inner-product-shaped work has "no simple way of determining [a trivial] intersection... without a computational overhead comparable to evaluating the inner product itself". Skipping per-*entry* when that specific `w[row_step]` is zero is still safe and free, just a smaller win than `l_solve`'s whole-step skip.

(A first attempt at a fuller DFS-based hyper-sparse implementation, covering all four solve directions and mirroring `L`/`U` both column- and row-major the way HiGHS does, was tried and measured *slower* end to end: the DFS setup's own per-call cost — allocating a fresh `visited` array plus an upfront `O(m)` density scan on every single call, even ones that ended up taking the dense-style branch — outweighed the fill-skipping it bought, on the order of 15-18% slower overall. That attempt was reverted in full. A second, narrower attempt — `LuFactors::l_solve_sparse_into`, covering only this module's one genuinely straightforward GP setting (`L`'s own forward direction, already stored column-major, fed a real LP's own sparse constraint column) with a *persistent*, epoch-stamped scratch (see `GpScratch`) rather than a fresh per-call allocation — measured as a small but real net win on the full Netlib benchmark set (73 problems, aggregate wall time ~1% lower, roughly even split of individually-faster/slower instances, zero objective mismatches) once the specific cost the first attempt's own revert blamed — the allocation, not the algorithm — was actually removed. This `L^{-T}` direction (BTRAN's tail) was deliberately *not* attempted a second time: Hall & McKinnon's observation above still applies unchanged (no static column-major structure of `L` to run the same DFS over without adding a row-major mirror), and the first attempt's win was concentrated in the one direction with a genuinely sparse, already-available seed — this direction's own `w` typically isn't.) — Later superseded by the `l_row` scatter form below.

### 関数 LuFactors::l_transpose_solve_scatter_into

The two loops compute the same `L^T w' = w` back substitution over the same nonzeros, only associating the updates differently: the gather form accumulates *into* `w[s]` one `l_col[s]` entry at a time (so `w[s]`'s own value is only known once every one of them has been read, and no prefix of them can be skipped as a group), while this form propagates *out of* `w[s]` into every `l_row.row(s)` entry at once. Because `w[s]` is the single multiplicand of that whole inner loop, `w[s] == 0.0` makes the entire step a provable no-op — the same whole-step skip `l_solve_into` and `FtLu::u_solve_into` already exploit in the forward direction, and the one Hall & McKinnon (2000) §4.4 explains the gather form *cannot* have. HiGHS reaches the same skip the same way, via its own row-major `lr_*` copy of `L` in `btranL`.

`docs/lu_comparison_enomoto_vs_highs.md` §2.6 names this as the one of HiGHS's four hyper-sparse solve directions this crate had never attempted (the *reason* being precisely that no row-major `L` existed to attempt it with — this method adds it).

Not bit-identical to the gather form: the same set of products is summed into each `w[s]` in the opposite order (descending source step here, `l_col[s]`'s own stored order there), so results can differ in the last ulp and, through the dual ratio test's tie-breaks, shift iteration counts either way on degeneracy-heavy instances. That is measured, not assumed — see this change's own analysis note for the per-problem numbers.

### Forrest-Tomlin 更新 (セクション冒頭コメントの原文)

Following Forrest, J.J.H. and Tomlin, J.A., "Updated triangular factors of the basis to maintain sparsity in the product form simplex method", Mathematical Programming 2 (1972), 263-278, as summarized precisely with full derivations in Huangfu, Q. and Hall, J.A.J., "Novel update techniques for the revised simplex method", Technical Report ERGO-13-001, University of Edinburgh (2013) §2.1 (equations 1, 4-13).

Column replacement `B̄ = B + (a_q - B e_p) e_p^T` is rearranged via the fixed factorization `B = LU` as `L^{-1} B̄ = U + (L^{-1}a_q - U e_p) e_p^T = U + (ã_q - u_p) e_p^T = U'` replacing column `p` of `U` with the partial FTRAN result `ã_q = L^{-1} a_q`. This "spikes" column `p` of `U` (rows > p can now be nonzero, breaking triangularity). Triangularity is restored by one row transformation `R^{-1} = I - e_p r^T` that zeros row `p` across every column: `Ū = R^{-1}U'`, where `r^T = ū_p^T U^{-1}` (`ū_p` = row `p` of `U` without its diagonal) can be obtained at negligible cost from `ẽ_p^T = e_p^T U^{-1}` (a partial BTRAN already computed to derive `r`) as `r = -u_pp · ẽ_p` with the `p`-th entry forced to zero (Tomlin 1974, eq. 12 in the 2013 paper). `R^{-1}` applied to column `p` of `U'` only changes its `p`-th entry: `ã_pq := ã_pq - r·ã_q`.

`L` never changes across updates. `U` is kept as a *sequence* of per-slot column etas (pivot + off-diagonal vector), because after a replacement the slot's eta is removed from wherever it sits and *appended* to the end — this ordering, not raw slot order, is what FTRAN/BTRAN through `U` must respect once updates have happened (verified by hand against a direct dense re-solve while implementing this). Each update additionally produces one `R` row-eta, kept in its own creation-ordered list and applied between `L` and `U` per `B_k^{-1} = U_k^{-1} R_k^{-1} ... R_1^{-1} L^{-1}` (eq. 13).

An eta's off-diagonal entries were a `HybridVec`: a sparse `(row_step, value)` list while the eta is genuinely sparse, a dense length-`m` array once its fill exceeds `DENSE_ETA_FRACTION` of `m` (typical of a dense-coefficient LP, where `U`'s eta chain is already close to fully dense from the very first update). That type's docs covered the trade-off, the skipped-slot convention that lets the dense form's loops run over the whole array unconditionally, and why its two consuming operations (`dot_dense`, `axpy_into_dense`) lived there rather than being re-written as a two-armed `match` at each of this file's eight FTRAN/BTRAN call sites. (Now superseded by `EtaFile`, which reproduces `HybridVec`'s semantics exactly.)

### 構造体 EtaFile

Instead of a `Vec` of structs each owning its own heap `Vec` of `(usize, f64)` pairs. An FTRAN `U` stage then reads 4 B per skipped eta (its `key`) and 12 B per entry, against 48 B per eta header and 16 B per entry (plus a pointer chase per eta) before. The dense form is kept exactly as `HybridVec`'s dense arm, so every loop computes precisely what the per-eta `HybridVec` computed — same entries, same order, same sparse/dense choice — and the results are bit-identical. (`dot` = `HybridVec::dot_dense` exactly; `axpy` = `HybridVec::axpy_into_dense` exactly; `remove_index` = `HybridVec::remove_index` exactly; `push_scaled_dense` = what `HybridVec::pack_scaled_dense` would build, with its own two-pass shape minus its per-eta allocation.)

Replacing a `U` eta removes its header from the parallel arrays (a `memmove` of 20 B per later header, against 48 B per `UEta` before — tombstoning instead was measured to cost more in the per-header dead test of every FTRAN/BTRAN than it saved here). `n_headers` doc said "dead ones included" (a leftover from the tombstoning variant). `span`: one load per eta.

### 関数 expected_dense_gate

Any value `>= 1.0` disables the result-density gate outright, since no result can be denser than `m`, restoring the input-nnz-only dispatch this crate had before `FtranDensity` existed — which is exactly how the A/B runs behind the constant's own value were produced. Read once per `FtranDensity::new` — a handful of times per solve, never on the per-iteration path.

### 関数 build_l_row

The empty case exists so that arm of the A/B is *genuinely* this crate's pre-§2.6 behaviour, construction cost included. Building `l_row` and then never reading it would leave the transpose's own `O(nnz(L))` build — paid at **every** refactorization, `dfl001` alone refactorizes ~100 times — inside both arms, hiding exactly the cost that has to be weighed against the scatter form's own win. Measuring a change against a baseline that already pays for it is how a feature gets adopted on a number that was never real. An all-empty `CscMat` transposes into a `CsrMat` with `m + 1` zero offsets and no entries, so `row(i)` stays valid (and empty) for every `i` rather than needing a separate `Option` on the hot path.

### 関数 btran_l_scatter_gate

(Its doc comment had drifted above `tiny_drop`.) `0` disables the scatter form outright (restoring the pre-`l_row` gather-only BTRAN, which is how the A/B behind the constant's own value is produced), `1` forces it unconditionally. Read once per refactorization, never per solve, same as `expected_dense_gate`.

### 関数 u_zero_skip_enabled

`ENOMOTO_FTRAN_U_ZERO_SKIP=0` restores the unconditional divide, which is how the A/B behind the default is produced.

### 構造体 FtranDensity

`DENSE_RHS_FRACTION` alone judges a solve by its *input*: the reach set the Gilbert-Peierls path walks is bounded below by the rhs's own nonzeros, so a dense rhs does prove the sparse path cannot win. The converse is not true — a one-nonzero rhs can still fill in to a fully dense `B^-1 a` once `L`'s own reach fans out, and then the sparse path has paid its DFS/epoch bookkeeping on top of doing the same elimination work the flat dense scan would have done anyway. Nothing about the *input* distinguishes those two cases, and the gap widens exactly as `m` grows: the bigger the basis, the further a single column's reach can fan out relative to the fixed sparsity of the column itself.

What does distinguish them is the channel's own recent history, which is what this tracks: HiGHS solves the same problem the same way, maintaining a per-operation `expected_density` running average (`HEkk::updateOperationResultDensity`) and handing it to `ftranL`/`ftranU` so each call can decide *before* running which mode it should be in (`HFactor::ftranL`'s own `expected_density > kHyperFtranL` test). This crate's `docs/lu_comparison_enomoto_vs_highs.md` §2.7 names that as the gap this type closes.

One instance per *call site*, never one shared instance: those channels' densities genuinely differ — a BFRT combined rhs sums whole flipped columns and is routinely much denser than a single entering column — and averaging them together would smear each one's own signal. Instances live in the solve loops (`solve_lp_dual_on` and `extended_dual`'s two loops), not in `FtLu` itself, deliberately: `FtLu` is rebuilt from scratch at every refactorization, which would throw the history away precisely when the basis is at its densest, whereas HiGHS's own densities likewise live in `HEkk` and survive across INVERTs.

The measurement itself is free: every solve path already ends in an `O(m)` permutation loop over the finished result, so counting that result's nonzeros costs one branchless add per entry inside a loop that was already running — and it is the *exact* result density, not an estimate. Crucially it is also taken on **both** branches, so the gate can never latch: a channel that starts producing sparse results again is observed doing so while it is on the dense path, and returns to the sparse path on its own. Field `expected` starts at `0.0` so a fresh channel dispatches exactly as it did before this type existed until it has actually observed something.

### 構造体 FtLu

- `u_seq`: physically stored in **creation order** — exactly as before — so `u_transpose_solve_into`/`u_solve_into` (the hot, once-*every*-iteration BTRAN/FTRAN paths, not just `try_update`) keep a plain sequential scan with no pointer-chasing indirection.
- `singles`: 42-99% of all `m` etas right after a refactorization on the heavy Netlib problems. A singleton's own division reads and writes only its own slot, and by `u_seq`'s triangular order every eta that writes into that slot (FTRAN) or reads it (BTRAN) sits *later* in the sequence, so its division can run after the whole `u_seq` pass in the `U` solves and before it in the `U^T` sweep without changing a single result bit — while the sequential loops skip visiting them at all. An eta that loses its last off-diagonal entry later just stays in `u_seq`, exact either way.
- `single_piv`: singletons are independent of each other and last in `U`'s order, so dividing each value as it is read out is bit-identical.
- `slot_pos`: kept in sync by `try_update` over exactly the range its own `Vec::remove`/`push` already touches, so this costs nothing beyond what the reordering itself already pays. Turns "find slot p's eta" from the O(m) linear scan `find_seq_pos` used to do into an O(1) index.
- `row_owners`: before this index existed, zeroing row `p` meant visiting all `m` etas in `U` and asking each "do you have an entry at `p`" (almost all answering no, but each still paying a full scan of its own off-diagonal list to say so). This answers "who has an entry at `p`" directly (combined with `slot_pos` for O(1) access to each one), so only the (typically small — a few percent of `m`, per `ENOMOTO_DEBUG_ETA_DENSITY` measurements) handful that actually do ever get touched.
- `r_etas`: flat like `u_seq`, never tombstoned.
- `scratch_a_tilde`/`scratch_e_tilde`: avoids the two per-pivot heap allocations (`ftran_through_l_and_r`'s owned result, and building `e_p` in place) that `try_update` used to pay on *every* pivot commit, the same "fresh `Vec` every iteration" cost this crate's hot dual-simplex loops elsewhere already eliminated via caller-owned buffers — `try_update` itself was the one hot-path call in this file still allocating, found via `ENOMOTO_PROF_PHASES_EXT` naming `ft_update` as 13-20% of wall time on several Netlib instances with no single other phase anywhere near as consistently large.
- `tick`: the `CLOCK` refactorization trigger (`ENOMOTO_SYNTH_CLOCK_FACTOR`'s docs at its call sites in `extended_dual.rs`). Unlike the wall-clock prototype this replaces (`analysis/ft_refactor_trigger_20260922_040850.md` §5/§6), every increment is driven only by the (already-deterministic) eta chain and right-hand-side content, never by `Instant::now()`, keeping the whole solve bit-reproducible. Stages that add to it: `ftran_through_l_and_r_into`, `solve_sparse_into[_capture]`, `u_solve_into`, `u_transpose_solve_into`, `solve_transpose_into[_capture]`. There is no explicit reset method because a fresh `FtLu` *is* the reset.
- `fill`: The refactorization trigger reads it **once per simplex iteration** (`simplex.rs`'s own trigger (3)), and re-summing meant walking all `m` entries of `u_seq` — touching every `UEta` header in the process — for a number that changes only at the handful of places an update already touches. Its own doc comment called that sum "`O(1)`-ish"; it was `O(m)`, one more full sweep of the eta file per iteration on top of the ones `u_solve_into`/`u_transpose_solve_into` genuinely need. This is that number actually being `O(1)`, with a `debug_assert` in `fill_count` that it still agrees with the sum it replaced.
- `fill_baseline`: Set in `new` to this factorization's own count, then overwritten back to the predecessor's by `factorize_reusing` whenever the factorization it just built came from a reuse, so a chain of reuses is always measured against the last order actually chosen by Markowitz.
- `build_tick`: once the *solving* work done against this factorization is estimated to cost as much as `FACTOR` fresh refactorizations of it would have — see `TICK_BUILD_M_COEF`/`TICK_BUILD_LU_COEF`'s history for where the two coefficients come from.
- `btran_l_scatter`: lives on `FtLu` rather than `LuFactors` so the factor struct stays pure data.

### 関数 FtLu::new

- `U`'s etas straight into the flat file: exactly what the former per-slot `Vec` + `HybridVec::pack` produced, same sparse/dense choice.
- S16 flop term: Unlike `nnz(L+U)` it grows quadratically with the dense tail's size, which is where this crate's Markowitz refactor time concentrates (`dfl001`).

### 関数 FtLu::l_transpose_solve_into

See `BTRAN_L_SCATTER_FRACTION`'s history for why both forms have to stay.

### 関数 FtLu::should_use_dense_solve_tracked

Deliberately only ever moves calls *towards* the dense path, never away from it: a rhs with more than `DENSE_RHS_FRACTION` of `m` nonzeros bounds the Gilbert-Peierls reach set below by that same count, so no amount of "but this channel's results are usually sparse" history could make the sparse path win on such a call. (`docs/lu_comparison_enomoto_vs_highs.md` §2.7.)

### 関数 FtLu::u_transpose_solve_into / u_transpose_solve_seeded / u_transpose_sweep

- Hyper-sparse via `row_owners`, unlike `u_solve_into` (where a GP-style DFS reach set was tried and reverted). The earlier gather form reached the same sparsity through "needed" marks (`analysis/greenbea_20260921_090812.md` §3-4), but still paid each needed eta's whole column dot product.
- `u_transpose_solve_seeded`: Seeding the "needed" set from `seed` directly was *exact*: the seeding loop marked precisely the steps where `z` is nonzero, and for a permuted unit vector that set is exactly `{seed}`. What it saves is that `O(m)` scan — and, at the call site, the `O(m)` permutation *gather* (`z[s] = rhs[col_perm[s]]`, a random-access read per step) that materialized the unit vector in the first place, which `seed_unit_rhs` replaces with a flat `fill` and one store.
- Sweep: the work is the nonzero `p`s' row lengths, not, as the gather form this replaced did, every needed eta's whole column dot product regardless of how few of that column's inputs are nonzero (26x more entries on `fit2p`, whose few dense border columns every BTRAN re-read). Changes the summation order into each `z[q]`, so the last bits — not the math — differ from the gather form.

### 関数 FtLu::u_solve_into

(A GP-sparsified counterpart to this function — restricting the scan to a DFS-computed reach set over `u_seq`'s own dependency graph, exactly mirroring `LuFactors::l_solve_sparse_into`'s own approach for `L` — was fully implemented, proven correct (an inductive argument that every `off_diag` target always sits at a strictly *lower* `u_seq` position than its referrer, mirroring `L`'s own low-to-high property, so descending position order needs no separate topological-sort step either) and tested (multiple sequential `try_update` calls reordering `u_seq` non-trivially, checked against the dense reference after every single one). It was still reverted after measuring it on the full Netlib benchmark set: aggregate wall time **+10.4%** versus `L`-only sparsification, 52 of 73 problems slower and only 6 faster. Unlike `L` (a *static* matrix, fixed once per full refactorization, whose seed — a real LP's own sparse constraint column — is reliably sparse), `U`'s own eta chain accumulates fill from every `try_update` since the last refactorization, so its reach set is typically far less sparse in practice — the DFS/reach-tracking overhead this function's outer loop is cheap enough to not need in the first place stopped paying for itself.)

(C5, third form, opt-in: `u_solve_hyper` — gated per channel on the caller's result-density average (`ENOMOTO_FTRAN_U_HYPER`, `ENOMOTO_FTRAN_U_HYPER_TAU`), aborting to this scan past `ENOMOTO_T_U_HYPER_ABORT` of `m`, and replacing the `O(m)` output permutation with a list scatter as well — the O(m) passes, not the eta loop alone, are what a sparse FTRAN is bound by.)

- `ENOMOTO_FTRAN_U_ZERO_SKIP=0` arm is the pre-§2.6 loop exactly, so that arm of the A/B is this crate's own previous behaviour and not "previous behaviour plus one unrelated change".
- Zero test before dividing: `0.0 / pivot` is `±0.0`, so an already-zero slot's division is a no-op that still costs a division and — worse on a hyper-sparse right-hand side — a store back into a random position of `x`, dirtying a cache line per zero slot for nothing. The skipped store can leave `+0.0` where the unconditional one would have written `-0.0` (when `pivot < 0`), which is exactly the difference `u_transpose_solve_into`'s own hyper-sparse skip already accepts, on the same grounds: every consumer of this result branches on zero-ness (`permute_out`'s own `!= 0.0` count, `commit_update`'s filter, the PRICE/DSE consumers), never on the sign of a zero.

### 関数 FtLu::u_solve_hyper

Applying the reached etas in the scan's own order keeps each entry's accumulation order (a DFS topological order alone would not). Sorting is `O(r log r)` in the reach size `r`, against the full scan's `O(m)`. A dense-arm eta's `axpy` spans all of `x`, hence the bail-out.

### 関数 FtLu::ftran_through_l_and_r_into

The existing `R`s are already part of the "L-like" fixed factor that update `k` treats as known, since `B_{k-1} = L R_1 ... R_{k-1} U_{k-1}` (eq. 13) rather than `B_{k-1} = L U_{k-1}` once `k > 1`. The unconditional `R`-eta tick term is the very "gather-type, no zero-skip" cost the CLOCK trigger's own analysis (§2.1/§2.2) identified as the eta-chain bottleneck, so this term alone is what makes `tick` grow with chain length the way FTRAN's own measured wall time does.

### 関数 FtLu::solve_into

`solve_lp_dual_on` calls this 2-4 times *every pivot* (BTRAN-DSE's `tau`, the entering column's `alpha`, and, when BFRT flips are pending, one more for `combined`), so the 2-3 `Vec` allocations each fresh `solve()` call used to cost here (one each in `l_solve`, `u_solve`, and the final permutation) were real, repeated per-iteration heap traffic — eliminated by having the caller own `scratch`/`out` once, outside the iteration loop, and reuse them every pivot. The returned nonzero count comes from the permutation loop that already visits every entry: one branchless add per entry, exact density rather than an estimate.

### 関数 FtLu::solve_into_capture

See `try_update_precomputed`'s history for why this capture (a plain `copy_from_slice`) lets the caller skip `try_update`'s own redundant re-derivation of the exact same value entirely.

### 関数 FtLu::solve_into_pair_capture (および sparse/triple 版)

The entering column's FTRAN and the DSE `tau = B^-1 rho_p` FTRAN of the same iteration, which HiGHS runs as two separate (optionally concurrent) solves (`HEkkDual::updateFtranDSE`). The synthetic tick is charged exactly as the two separate calls would charge it too, so the `CLOCK` refactorization trigger fires on exactly the same iterations. What is saved is the second pass over `L`/`R`/`U`'s own storage (memory traffic and loop overhead), which on the larger Netlib instances no longer fits in cache between the two solves.

### 関数 FtLu::permute_out

`+= (v != 0.0) as usize` rather than a branch: the compare is a single instruction and the add is unconditional, so the count adds no branch misprediction to a loop whose scatter already dominates it.

### 関数 FtLu::solve_sparse_into

`l_solve_into` (the dense path) starts by unconditionally overwriting every entry of `scratch` (`z[s] = rhs[row_perm[s]]` for every `s`), so it tolerates arbitrary leftover content — but `l_solve_sparse_into` requires `scratch` to *already* be all-zero on entry. The guarantee only holds if nothing else writes through the same buffer in between.

`U` stays on the plain `u_solve_into` scan — see that function's history for the *two* separate attempts at a reach-restricted counterpart (one ungated, one gated exactly the way HiGHS gates its own `ftranU`) that were both implemented, proven correct, measured over the full Netlib set, and reverted as regressions.

### 関数 FtLu::add_zero_rhs_solve_ticks / add_zero_rhs_btran_ticks

Lets a caller skip a provably-zero FTRAN (e.g. the extended dual's BFRT slope channel when no flipped column's width carries an `M` term). For a zero rhs: the dense `L` stage costs a flat `m`, the sparse one's reach set is empty (`0`); every `R` eta is visited unconditionally; the `U` stage pays its flat `m` and no eta survives the zero skip. BTRAN: no `R` eta (each is skipped on its zero `yp`); used by the extended dual's all-slack-cost start (S14).

### 関数 FtLu::solve_sparse_into_capture

The capture happens after the `R`-eta loop (this stage's own last write to `scratch` before `u_solve_into` takes over), so `a_tilde_out` ends up identical regardless of which of the two FTRAN paths (`should_use_dense_solve`'s dense/sparse dispatch) a given call took.

### 関数 FtLu::solve_transpose_into / seed_unit_rhs / btran_tail_cap

- `solve_transpose_into` is `solve_lp_dual_on`'s once-per-pivot BTRAN for `rho_p`; no allocation for the same reason as `solve_into`.
- `seed_unit_rhs`: `P_col^{-1} e_i` is the unit vector at step `col_perm_inv[i]`, so the gather collapses to a flat `fill` plus a single store — no random-access read per step, and the caller never has to own (or keep re-zeroing) a length-`m` `e_i` buffer of its own.
- `btran_tail_cap` tick comment: `l_transpose_solve_into` was a dense `O(m)` reverse scan regardless of fill (see that method's history for why sparsifying it wasn't worth trying — later superseded by the `l_row` scatter form, but the tick accounting still charges a flat `m`).

### 関数 FtLu::solve_transpose_unit

Unlike `solve_transpose_unit_into` this makes no assumption about `u_seq`'s ordering, so it is valid with Forrest-Tomlin updates applied.

### 関数 FtLu::solve_transpose_into_capture

**No production call site left**: every `e_tilde`-capturing BTRAN in this crate has a unit-vector right-hand side and goes through `solve_transpose_unit_capture` instead. Kept, rather than deleted, because it is the general-`rhs` reference that specialization is *checked against* — `solve_transpose_unit_is_bit_identical_to_the_dense_unit_rhs_path` asserts the two agree entry for entry, and on the synthetic tick, both on a fresh factorization and after Forrest-Tomlin updates have reordered `u_seq`. Deleting it would delete the proof.

### 関数 FtLu::solve_transpose_unit_into

Built for `DseState::from_basis`'s own `m` back-to-back unit-vector solves after every refactorization — measured as 27% of `dfl001`'s total wall time before this method existed (`dfl001-bottleneck-max-iters-cap` memory), because that call site pays `solve_transpose_into`'s full `O(m)`-per-call cost `m` times over, every refactorization.

`update_count()` is already the cheapest possible signal, so this method itself only asserts it rather than re-deriving it. The optimization exploits a structural invariant of a *freshly factorized* `u_seq` (`FtLu::new`'s own construction: `u_seq[slot]`'s `off_diag` entries only ever reference `row_step < slot`, `U`'s own upper-triangular structure) that `try_update` is free to break (its own Forrest-Tomlin bump-and-replace algorithm reorders `u_seq` and can introduce entries referencing a *later* row-step than before).

**The optimization**: permuting `e_i` (`col_perm_inv[i]`) yields a single nonzero at step `s0`; the forward recurrence can only ever produce a nonzero at slot `p` if some earlier slot `< p` it depends on is already nonzero — with nothing nonzero below `s0`, every slot `< s0` is therefore provably still `0` after the sweep, without computing a single one of their dot products. Starting the sweep at `s0` instead of `0` is exact, not approximate, and needs no DFS/epoch bookkeeping the way a full Gilbert-Peierls reach-set restriction would (see `[[dfl001-bottleneck-max-iters-cap]]`/this crate's history for why a *fuller* sparsification of the shared `u_transpose_solve_into` — applied to every per-iteration `rho_p` BTRAN, not just `from_basis`'s refactor-time calls — was tried and reverted as a net aggregate regression across the full Netlib set): that measurement's DFS/epoch overhead was paid on tens of thousands of per-iteration calls across many small problems where the skip bought little; this plain prefix skip carries no such per-call bookkeeping cost, and `from_basis`'s own access pattern (`m` calls, but only at refactor time) concentrates exactly on the large/refactor-heavy instances (`dfl001`, `pilot87`) a fuller sparsification would have helped too, without the small-problem dilution that sank the earlier attempt.

`L^{-T}` is left exactly as dense as it always was — a *second* attempt to sparsify it specifically was never worth trying (its own input is typically no longer sparse by that point, fill having already spread across `[s0, m)` during the `U^{-T}` sweep). Postcondition rationale: `L^{-T}`'s own reverse sweep can scatter fill back into positions below `s0`, so (unlike `l_solve_sparse_into`'s narrower reach-set cleanup) nothing cheaper than a full `O(m)` reset is safe here. The singleton loop's empty sum reproduces what the former `HybridVec::dot_dense` returned.

### 関数 FtLu::try_update

FT needs only the partial FTRAN result `L^{-1}a_q` (eq. 1), unlike a product-form update which would need the full solve.

**Schork & Gondzio (2017), "Permuting Spiked Matrices to Triangular Form and its Application to the Forrest-Tomlin Update"**: tried and reverted. The idea: when the spike's own diagonal `a_tilde[p]` is nonzero and its off-diagonal support is disjoint from the structural `Reach(p)` (every slot whose value transitively depends on `p` — a single forward walk over `u_seq`, mirroring `u_transpose_solve_into`'s own traversal but following every *stored* `off_diag` edge unconditionally rather than only the ones whose *propagated* value under one unit-impulse seed happens to still be nonzero — the two differ on real, coefficient-heavy LP data via exact numerical cancellation, confirmed against real Netlib instances via a dedicated invariant cross-check during development), the spiked matrix is *already* permutable to triangular form with no elimination and no `REta` at all (their Theorem 3.1 / Lemma 3.2) — repositioning `p` and every member of `Reach(p)` to the end of `u_seq`, preserving their relative order, instead.

Implemented fully correctly (including the structural-vs-numerical reach distinction above, found and fixed via a randomized stress test plus real-Netlib debug cross-checks) and, separately, a real unrelated bug it exposed (`simplex.rs`'s `FT_MAX_UPDATES` hard refactorization cap read `update_count()`, i.e. `r_etas.len()` — which a permutation-only update never grows, so on instances where many updates resolve that way the cap could go uncrossed far longer than intended, letting numerical drift compound until a later refactorization hit a matrix too corrupted to factor; fixed by counting *every* successful update, not just row-eta ones, for that specific trigger). Even after replacing an initial `HashSet`-based reach implementation with an epoch-stamped array (the same bump-instead-of-clear trick `sparse_lu::GpScratch` already uses), full-Netlib measurement still showed a net regression — not from this function's own added cost (which the epoch-array version brought back down close to baseline), but because the permutation path's slightly different rounding characteristics than the standard row-eta path perturbed dual-simplex tie-breaks on degeneracy-heavy instances (`pilotnov` needed 3218 iterations instead of 1286 for the *same* correct answer) — a downstream effect no amount of tuning this function itself can address. See the project history around this doc comment's own commit for the full numbers if revisiting.

- Scratch reuse: the `e_tilde` zeroing is still one `O(m)` pass, exactly as `vec![0.0; m]` used to pay for its own zero-initialization — what this reuse actually saves is the allocator round-trip itself, not this fill.

### 関数 FtLu::try_update_precomputed

**Why this exists**: a typical dual-simplex iteration already runs exactly the two solves `try_update` used to redo from scratch, for its own unrelated purposes — `rho_p = B^-T e_p` (`solve_transpose_into`, needed for PRICE) computes `U^-T e_p` as an internal step before applying the `R`-etas and `L^-T`, and the entering column's own FTRAN (`solve_into`/`solve_sparse_into`, needed for the primal update and DSE) computes `(L R_1...R_{k-1})^-1 a_q` as an internal step before applying `U^-1` — both are simply overwritten in place by the next stage rather than kept. Since `p`/`a_q_original` are identical between that earlier call and this update (same leaving row, same entering column, same iteration, `self` unchanged in between), the values are not merely *equivalent* to what `try_update` would recompute — they are bit-for-bit identical, `ftran_through_l_and_r_into`/`u_transpose_solve_into` being pure functions of `(self, input)`. Capturing them (a plain `copy_from_slice`) is far cheaper than either of the two full solves this replaces — a dense `O(m)` pass through `L` plus every accumulated `R`-eta for `a_tilde`, and an `O(nnz(U))` scan of the whole eta chain for `e_tilde`, both of which grow as updates accumulate since the last refactorization.

### 関数 FtLu::commit_update

Pulled out into its own `&mut self` method (rather than duplicated in both callers) specifically so the intricate `row_owners`/`slot_pos`/`u_seq` bookkeeping — the part a copy-paste split would risk drifting out of sync between two copies — exists in exactly one place.

- The `R` eta is built straight out of `e_tilde` — same entries, same order, same sparse/dense choice as the `collect()`-then-`HybridVec::pack` this replaces, minus that intermediate `Vec`. The previous code likewise materialized the whole thing before testing, so a rejected update is no more expensive than it already was.
- `Vec::remove` shifts every later element down by one position — update `slot_pos` for exactly that range (elements the memmove itself already touches, so this is no extra asymptotic cost) rather than the old `find_seq_pos`'s full O(m) re-scan.
- Replacement column: built directly from `a_tilde` rather than through a throwaway pair list.

### 関数 FtLu::fill_count

As updates accumulate, the eta file grows (each `R` and each replaced `U` slot can carry up to `m-1` entries), which is exactly the cost trigger (3) exists to bound. Counts true nonzeros (formerly via `HybridVec::nnz`), not storage length, so switching an eta to the dense representation doesn't spuriously inflate this and trip the trigger early.

### テスト (mod tests) — 旧 doc コメント中の経緯・理由

- `factorize_diagonal_vs_markowitz_sweep`: across the `m` range this crate's Netlib benchmark actually exercises. `#[ignore]`d for the same reason as `border_crossover_sweep`: a live diagnostic, not a pass/fail correctness check.
- `factorize_dense_faer_matches_hand_verified_solve`: fed straight to `factorize_dense_faer` (not through `factorize`'s dispatch, to test this path in isolation regardless of where the threshold currently sits) and checked against a hand-verified solve, the same style `factorize_and_solve_matches_expected` uses for the Markowitz path — this is the ground truth that actually matters (not "does it match Markowitz's own answer", which would only prove the two agree with *each other*, not with reality).
- `factorize_dispatches_dense_input_to_faer`: confirms `is_dense_input` actually fires for a fully dense matrix at a size realistic for this crate's target problems, not just in the tiny fixtures the Markowitz-path tests use (which could accidentally clear a generous threshold too).
- `factorize_bordered_matches_flat_markowitz`: both must solve `Bx = rhs` correctly, not just agree with each other, so `rhs` is built from a known `x_true`.
- `border_crossover_sweep`: to find where `BORDER_MAX_FRACTION`'s (then) `0.3` cap should actually sit — that constant's docs at the time candidly noted it was never tuned against real data (Netlib's own `fit1p`/`fit2p` family only ever exercises `k/m` in the few-percent range). Kept as a live diagnostic (like `debug_print_block_sizes`) rather than deleted, since a future problem shape or a revisit of the threshold can just rerun it.
- `arrowhead_partial`: the arrowhead shape makes border columns *fully* dense (every local row touches every border column), which means `nnz` grows with `k` fast enough to trip `is_dense_input`'s own 25%-of-`m^2` gate on its own once `k/m` crosses roughly that same 25% (a border column population of `k` fully-dense columns alone already contributes `k/m` density) — so the first sweep never actually exercises `factorize_bordered` against a *genuinely sparse-overall* `flat_markowitz` at large `k/m`; `factorize`'s own `is_dense_input` check would already have routed those cases to `factorize_dense_faer`. This variant holds overall density far below that gate to see whether `k/m` still has a *genuine* independent crossover once that confound is removed.
- `border_crossover_sweep_fine`: around the `m=800` crossover found (bordered wins at `k/m=0.50`, loses at `0.60`).
- `border_crossover_sweep_scaling`: if the crossover were an *absolute-`k`* effect (the `k x k` Schur complement's own `O(k^3)` dense factorization cost), a larger `m` would cross over at a *smaller* `k/m` (same absolute `k`).

- `try_update_precomputed_matches_try_update`: Direct regression test for the whole premise behind `try_update_precomputed`/`solve_into_capture`/`solve_sparse_into_capture`/`solve_transpose_into_capture`: must produce bit-identical results — not merely close ones — since both are meant to compute exactly the same values. Runs on a state that already has two prior updates applied (non-trivial `u_seq`/`r_etas`), the realistic case, not just a freshly-refactored one. The dense path mirrors `simplex.rs`'s dense-rhs bypass branch and `rho_p` BTRAN usage; the sparse path mirrors `simplex.rs`'s sparse-rhs branch.
- `next_rand`: "Deterministic xorshift-ish LCG".
- `random_sparse_diag_dominant`: diagonal dominance guarantees `factorize` never needs a singularity fallback; unlike the crate's other, smaller hand-written fixtures.
- `l_row_is_the_exact_transpose_of_l_col`: Checked on a factorization with genuine fill (a plain diagonal basis would pass vacuously with both structures empty).
- `btran_agrees_between_scatter_and_gather_after_ft_updates`: the state every per-iteration BTRAN actually runs in, and the one where a wrong transpose would show up as a wrong `rho_p` rather than a merely differently-rounded one.
- `residual_inf`: what a factorization is actually *for*, and therefore a stronger check on a reused order than comparing its `L`/`U` against a Markowitz run's (the two legitimately differ: same matrix, two valid orders).


## src/params.rs (mod lu) — 定数ドキュメントの原文 (測定経緯)

(Phase 1 で lu.rs から移された英語 doc コメントの全文。`DENSE_INPUT_FRACTION`/`BORDER_MAX_FRACTION` の doc 先頭に紛れ込んでいた `factorize` の Dulmage-Mendelsohn 試行の段落は上の「関数 factorize」節に移した。)

### 定数 STABILITY

Threshold-pivoting stability floor (see this module's own top docs): a
pivot candidate must be at least this fraction of its column's live max
magnitude to be eligible, regardless of Markowitz count. Raised from the
textbook-default `0.1` after measuring that `0.1` lets `factorize()` pick
pivots numerically weak enough to make the *resulting* `L`/`U` drift
faster under `extended_dual`'s `XB_DRIFT_TOL` check (see that constant's
own docs) — i.e. a chain of numerically-marginal Markowitz choices, not
any single one bad enough to fail `FT_MIN_PIVOT` outright, was forcing
extra mid-solve refactorizations well before `FT_BUMP_LIMIT_FACTOR`'s own
eta-fill trigger would have. Netlib's `pilot` (the clearest case)
dropped from 190 drift-triggered refactorizations to 88 at `0.25`
(measured twice, deterministic — refactor counts don't vary run to run,
only wall-clock does), for a ~51% wall-time cut on that instance alone;
`greenbeb`/`fit2p` improved or held flat; `d2q06c` was unchanged within
run-to-run noise (~5%, from system load, confirmed by re-running the
unchanged `0.1` baseline twice). The standard 73-problem Netlib set
(`enomoto_solver.benchmark_highs`, which skips these largest instances on
`n_vars`) is flat within the same noise band either way — this constant
only matters for problems that already refactorize dozens-to-hundreds of
times. `0.5` was tried first and rejected: fill-in from the stricter
floor made every iteration measurably more expensive (`d2q06c`,
`greenbeb`, `fit2p` all ~4% slower net, more than offsetting their own
small refactor-count drops), so `0.5` is *not* simply "more of the same
good direction" — `0.25` is a measured sweet spot, not a floor to keep
pushing from without re-benchmarking.
Since `docs/lu_comparison_enomoto_vs_highs.md` §2.4 this is the
*starting* value of a per-solve threshold that a simplex loop may
escalate ([`pivot_threshold`]) — but that escalation is off by default
(`extended_dual::PIVOT_ESCALATION_STEP`, which records what enabling it
measured), so this remains the floor every solve actually runs at, and
everything measured above still describes the default build.

### 定数 PIVOT_THRESHOLD_MAX

Ceiling on the escalated pivot threshold ([`escalate_pivot_threshold`])
— HiGHS's own `kMaxPivotThreshold`. [`STABILITY`]'s docs record that a
*static* `0.5` costs ~4% on `d2q06c`/`greenbeb`/`fit2p` through extra
fill-in, which is exactly why this value is reachable only after the
escalation ladder below has evidence that *this* solve is paying more
for instability than it would for fill.

### 定数 PIVOT_THRESHOLD_MIN

Floor for an operator-supplied `ENOMOTO_PIVOT_THRESHOLD` — HiGHS's own
`kMinPivotThreshold`. Nothing escalates *downwards*, so this only ever
clamps the env override.

### 定数 PIVOT_THRESHOLD_FACTOR

Multiplier applied per [`escalate_pivot_threshold`] step. HiGHS uses
`kPivotThresholdChangeFactor = 5.0` from a `0.1` default; from this
crate's `0.25` a factor of `2.0` lands exactly on
[`PIVOT_THRESHOLD_MAX`] in one step, so the ladder here is
`0.25 -> 0.5`, and a second escalation is a no-op.

### 定数 DENSE_COL_FRACTION

A column whose *initial* (pre-elimination) degree exceeds this fraction
of `m` is treated as "dense" by `find_best_pivot`'s dense-avoidance
pass — see `MarkowitzState::initially_dense`'s own docs for why a
column's *current* (post-elimination) degree is the wrong thing to
threshold on here. `0.5` catches the handful of near-fully-dense
"trend"/regression columns Netlib `fit1p`/`fit1d`-shaped problems are
built around (confirmed: `fit1p`'s basis has columns with degree
610-627 out of `m=627`, against a median column degree of `1`) without
also catching moderately-populated columns that pose no real fill-in
risk.

### 定数 PIVOT_SEARCH_LIMIT

How many candidate columns a single `find_best_pivot` call may examine
before it settles for the best pivot it has already found — HiGHS's
`searchLimit = min(nwork, 8)` in `HFactor::buildKernel`
(`docs/lu_comparison_enomoto_vs_highs.md` §2.5), adapted to this file's
bucket scan.

The existing per-degree-level early exit (`best_score <= deg_col *
deg_col`, the analogue of HiGHS's `merit_limit`) only ever fires at a
*level* boundary, so a single heavily-populated bucket is scanned to
its end no matter how good the pivot found in its first few columns
was. That is the search-explosion case this bound closes: on an
ill-conditioned or fill-heavy step, the low-degree buckets hold
hundreds of columns whose rows all get walked (and whose
`ensure_col_max_abs` recomputes all get paid) to improve on a pivot
that was already acceptable.

Like HiGHS's, the bound is only honoured once a pivot *has* been found
— `find_best_pivot` never returns `None` because of it, so `factorize`'s
`skip_dense` fallback and its genuine-singularity detection are
unchanged. What it does change is *which* acceptable pivot is returned:
the Markowitz count can be worse than the unbounded scan's, so this
trades (bounded) extra fill-in for a bounded search.

**`256`, not HiGHS's `8` — measured, see
`analysis/pivot_search_limit_20260922_143000.md`.** `8` was tried first
and rejected: it is not "more of the same good direction", it is a
different intervention. At `8` the bound fires on ordinary steps and
changes the chosen pivot on **64 of the 93** Netlib problems; each such
change perturbs the factorization's last digits, which moves the dual
ratio test's tie-breaks, which moves the iteration count by an amount
whose *sign is effectively arbitrary per problem* (`greenbeb` +18%
iterations, `25fv47` −11%). Reproduced over two independent 93-problem
runs, `8` left three problems past +10% (`greenbeb` +21/+22%, `pilot`
+15/+18%, `grow22` +13/+13%) even though it cut the search everywhere,
and a sweep showed no smaller constant escapes the lottery: `16` made
`pilot87` **2.9x slower**, `64` still perturbed 26 problems.

`256` is chosen so the bound is a worst-case guard and nothing else. It
fires on 7 of 93 problems, and only one of those (`dfl001`, the single
instance where the unbounded scan is genuinely expensive: 2.96s of a
22.0s solve, averaging 261 candidate columns per elimination step)
changes materially — its scan drops to 1.54s. The other 86 problems are
bit-identical to the unbounded scan, iteration count and
refactorization count included, so the change cannot regress them at
all. Two independent 93-problem runs: −2.1% and −0.8% in total, no
problem past ±10% in either. `512` was also measured (−3.2%/−, perturbs
only 2 problems) but put `wood1p` at +10.6%, so it fails the same rule
`8` does.

### 定数 KERNEL_LINEAR_SCAN_MAX

Rows shorter than this are searched for a column linearly rather than
by binary search ([`KernelMatrix::row_get`]). Markowitz elimination is
specifically choosing pivots to keep the active rows short, so the
linear branch is the common one: a run this size fits in one or two
cache lines and scans branch-predictably, where `binary_search` pays a
mispredict per level for the same work.

### 定数 U_HYPER_ABORT_FRACTION

C5 hyper-sparse `U` stage: give up (and take the plain full scan) once
the DFS has reached more than this fraction of the `m` slots — past it,
sorting the reach and scattering the result by list stop paying for
themselves (HiGHS's own `kHyperFtranU` is `0.10`).

### 定数 DENSE_INPUT_FRACTION

Input whose nonzero density exceeds this fraction of `m^2` skips
Markowitz elimination entirely in favor of [`factorize_dense_faer`]'s
dense partial-pivoting LU (via the `faer` crate). Markowitz's whole
point is to *minimize fill-in*; a matrix already this dense has none
left to save, so its bucket/degree bookkeeping ([`KernelMatrix`]'s own
row/column runs plus `col_buckets`/`row_buckets`) is pure overhead at
that point — confirmed on a synthetic dense LP
(Netlib has none dense enough to exercise this at all): `factorize`
dominated wall time (95-98%, repeated every few dozen `try_update`
calls since a dense basis's eta fill crosses `FT_BUMP_LIMIT_FACTOR *
m` almost immediately) while this file's own FTRAN-side dense
optimizations (`HybridVec`'s dense arm, `FtLu::should_use_dense_solve`)
together accounted for under 1% of the same wall time — i.e. the eta
chain was never the bottleneck for a dense basis, the cold
factorization was. `0.25` is a first-pass threshold, not yet tuned
against a real dense-problem benchmark (Netlib has none).

### 定数 BORDER_MAX_FRACTION

`factorize`'s own gate for attempting [`factorize_bordered`] before
falling back to plain [`factorize_flat_markowitz`] — see
`factorize_bordered`'s own docs for the technique and why it exists.

This gate's own detection cost (`detect_border_columns`, one `O(nnz)`
pass) is cheap enough to run unconditionally: a controlled full-73-
problem Netlib A/B (this gate enabled vs. plain
`factorize_flat_markowitz` always) showed no measurable regression on
any instance once run-to-run subprocess scheduling noise was
controlled for (repeated head-to-head timing, not two independently-
scheduled full-batch runs — several apparent double-digit-percent
"regressions" in the first batch-vs-batch comparison, e.g.
`fffff800`/`scfxm1`/`ganges`, vanished under direct repeated
comparison), while several instances beyond `fit1p` itself improved
substantially (`scrs8` -58%, `ship04s` -57%, `shell` -52%, `maros`
-41%, `fit1p` -26%, plus a handful more in the 20-45% range) — this is
the same `k`-nonzero-columns detection [`DENSE_COL_FRACTION`] already
made cheap for `MarkowitzState::initially_dense`'s own purposes,
evidently common enough across Netlib-shaped LPs (not just the
`fit1p`/`fit2p` "trend column" family) to be worth attempting by
default rather than gating behind an opt-in flag.

**`BORDER_MAX_FRACTION` (`k / m`) is the real, measured constraint —
not an absolute `k` count.** A synthetic-`fit1p`-shaped sweep
(`border_crossover_sweep*` in this module's own tests, `#[ignore]`d,
rerun via `cargo test --release -- --ignored --nocapture border_`) at
both `m=800` and `m=2000` found `factorize_bordered` beating
whatever `factorize_flat_markowitz` would otherwise pick (plain
Markowitz below `is_dense_input`'s own 25% gate, `factorize_dense_faer`
above it — `factorize_bordered` beats *that* too, up to a point) by
**30x-600x** for `k/m` up to `0.40`, crossing over to a wash somewhere
around `k/m ~= 0.5` and a clear loss by `k/m = 0.6` — at *both* `m`
values, i.e. this is a genuine fraction effect (the `k x k` Schur
complement's own `O(k^3)` dense-factor cost, relative to the `(m-k)`-
sized sparse part it's carved out of), not an absolute-`k` one: `m=800,
k=400` and `m=2000, k=1000` (both `k/m=0.5`) landed at the same
break-even point despite `k` itself differing by 2.5x. `0.4` sits with
real margin below the measured crossover.

The *previous* version of this gate paired that fraction with a
`BORDER_MAX_COUNT` of `200` on the mistaken assumption that unbounded
`k` needed an absolute backstop the way the reverted Dulmage-Mendelsohn
attempt did — the sweep above disproves that directly (`m=2000, k=800`,
five times over `200`, still won by 603x). `BORDER_MAX_COUNT` here is
now a purely defensive sanity bound, sized so its own `O(k^3)` dense
factor stays well under this crate's stated basis-size envelope ("`m`
in the low thousands", per `GpScratch`'s own docs) rather than
something expected to actually bind — `BORDER_MAX_FRACTION` is doing
the real work.

### 定数 BORDER_MAX_COUNT

(doc なし; 直前の定数と説明を共有)

### 定数 REBUILD_FILL_LIMIT

A reuse is abandoned (falling back to a full Markowitz `factorize`)
once the factors it is producing exceed this multiple of the nonzero
count of the last *full* factorization's own `L`+`U`.

**`1.25` is measured, not guessed.** Fill a reuse produces is not a
one-off cost — it is paid again by every FTRAN/BTRAN for the whole life
of the resulting factorization — and a generous limit is a net *loss*
even though it accepts more reuses: over a 25-problem in-process A/B
(`analysis/` note for this change, §3) the aggregate against the
feature disabled ran `2.0` +2.7%, `1.1` +0.4%, `1.25` -1.5%, with the
`2.0` arm's worst case `greenbeb` +30%. Too *tight* loses the other
way: `1.0`/`1.05` reject nearly every attempt (the basis genuinely
densifies between refactorizations), so the backoff below stops even
trying and the feature turns into pure overhead.

The reused order was chosen by Markowitz against a *previous* basis;
the current one differs from it by however many Forrest-Tomlin updates
happened since, so the same order can be numerically fine yet produce
far more fill than a fresh Markowitz run would. Fill produced here is
not a one-off cost: it is paid again by every FTRAN/BTRAN for the whole
life of the resulting factorization, which is exactly the trade this
guard exists to cap. The baseline deliberately tracks the last *full*
factorization rather than the immediately-preceding one (see
[`FtLu::fill_baseline`]), so a long chain of reuses cannot ratchet the
limit upward one small increment at a time; a basis whose fill
genuinely grew simply fails this guard once, gets a fresh Markowitz
factorization, and the new baseline is that one's own.

### 定数 REBUILD_MIN_PIVOT

Absolute floor on a pivot's magnitude: below this the column has
nothing usable left in the remaining submatrix at all, and the whole
attempt is abandoned rather than dividing by (almost) zero. Far below
`simplex.rs`'s own `FT_MIN_PIVOT` deliberately — this is a "there is no
pivot here" test, not a quality test, which the [`STABILITY`] check
next to it already is.

### 定数 REUSE_BACKOFF_SHIFT_CAP

Caps on [`factorize_reusing`]'s own exponential backoff after a
rejected reuse: the streak's shift is capped first (so the shift itself
can never overflow), then the resulting skip count.

### 定数 REUSE_MAX_BACKOFF

(doc なし; 直前の定数と説明を共有)

### 定数 DENSE_ETA_FRACTION

A column/row whose off-diagonal fill exceeds this fraction of `m` is
stored densely (see [`HybridVec`]). Unlike [`DENSE_COL_FRACTION`] (tuned
against real Netlib data, all of it sparse), this threshold has no
dense-problem benchmark to tune against yet in this crate's own test
set — `0.4` is a first-pass value, not a measured one; re-tune once a
genuinely dense-coefficient LP is available to benchmark against.

### 定数 DENSE_RHS_FRACTION

A caller-provided FTRAN right-hand side whose own nonzero count exceeds
this fraction of `m` is dense enough that the Gilbert-Peierls sparse
path's DFS/epoch bookkeeping (see `LuFactors::l_solve_sparse_into`'s own
docs) no longer pays for itself — its reach set is bounded below by the
rhs's own nonzero count, so a dense rhs alone already guarantees a large
reach regardless of how sparse `L` itself is. Exposed as
[`FtLu::should_use_dense_solve`] rather than a flag fixed at
construction time: an earlier version of this gate measured density
once per refactorization from the *basis*'s own `L`/`U` fill and cached
it — which reads as permanently sparse for the entire solve whenever
the crash-start basis (the slack identity, always maximally sparse)
never gets refactorized a second time, silently never firing even on a
genuinely dense-coefficient LP whose real (post-pivoting) basis is
dense throughout. Checking the actual rhs at each call site instead has
no such staleness problem and costs nothing extra (the caller already
has the sparse rhs's length on hand). Like `DENSE_ETA_FRACTION`, `0.4`
is a first-pass threshold, not one tuned against a real dense-problem
benchmark yet.

### 定数 DENSITY_AVERAGE_MULTIPLIER

Weight given to the newest observation when folding it into an
[`FtranDensity`] running average. This is HiGHS's own
`kRunningAverageMultiplier` (`HEkk::updateOperationResultDensity`,
used there for exactly the same purpose — see that class's
`col_aq_density`/`row_ep_density` fields), kept at the same value for
the same reason: small enough that one atypical iteration cannot flip
the dense/sparse dispatch on its own, large enough that a genuine
phase change (a basis that has filled in over the last dozen pivots)
is picked up within ~20 iterations rather than being averaged away
over the whole solve.

### 定数 EXPECTED_DENSE_FRACTION

An FTRAN call site whose recent *results* have averaged denser than
this fraction of `m` takes the dense solve regardless of how sparse
the right-hand side it is handed happens to be — see [`FtranDensity`]'s
own docs for why the input's own nonzero count
([`DENSE_RHS_FRACTION`]) is not a sufficient predictor on its own.
Overridable at run time via `ENOMOTO_EXPECTED_DENSITY_GATE` (see
[`expected_dense_gate`]) so this one number can be re-tuned against the
Netlib set without a rebuild.

`0.35` is measured, not guessed (`analysis/ftran_density_gate_20260922_062832.md`
§4.2, an A/B over the full Netlib set run *inside one process* with the
setting flipped between solves, since this box's per-problem run-to-run
spread otherwise reaches 4x): against the gate disabled, `0.35` is -3.9%
over the 14 mid-heavy instances and -1% over all 93, while `0.2` is
*worse* than no gate at all (+0.8%). The reason `0.2` loses is specific
and worth keeping: it drags the BFRT combined-flip channel onto the dense
path too (its results average 0.24-0.49 dense, against the entering
column's 0.65-0.99), and on `greenbeb` that turns a -22% win into -1%.
A threshold between the two channels' own measured densities is what the
gate wants, not the lowest one that still fires.

### 定数 BTRAN_L_SCATTER_FRACTION

Density ceiling for BTRAN's row-major scatter form
([`LuFactors::l_transpose_solve_scatter_into`]): the `L^{-T}` stage
takes it only when under this fraction of the incoming `w` is nonzero,
and falls back to the column-major gather form
([`LuFactors::l_transpose_solve_gather_into`]) otherwise.

**A gate is needed here, not just a faster kernel.** The two forms
touch exactly the same `L` entries; what differs is the access shape.
Gather reads `w[row_step]` at random and accumulates into one place
(`w[s]`, which the compiler keeps in a register across the whole inner
loop); scatter reads one place (`w[s]`) and does a random
read-modify-write per entry. On a sparse `w` the scatter's whole-step
skip wins outright — most steps do no work at all — but on a dense `w`
nothing is skipped and the scatter is left paying random *stores*
where the gather paid random *loads*, which is strictly worse. Measured
exactly that way on the first ungated A/B of this change: `dfl001`
(whose BTRAN `w` is dense by the time `U^{-T}` and the `R` etas are
done with it) +6.5%, against wins on the sparse-`w` instances. HiGHS
gates all four of its own solve directions for the same reason
(`HFactor::btranL`'s own `sparse_solve` test, `kHyperBtranL`).

The test is an exact nonzero count of `w`, not a running-average
prediction: unlike an FTRAN's input (whose density is only knowable
from history — see [`FtranDensity`]'s own docs), `w` is right there in
a buffer that every path over it already scans at least once more
(the permutation into `y`), so one early-exiting `O(m)` sequential
pass answers the question exactly, for a fraction of the `nnz(L)`
random accesses the stage itself is about to do either way.

### 定数 TICK_BUILD_M_COEF

Per-row-of-`U`-and-`L` coefficient for [`FtLu::build_tick`]'s `m`-only
term — HiGHS's own `buildSynthticTick` (`HFactor.cpp`) uses `80` for the
analogous term (`num_row * 80`); kept unchanged here rather than
re-derived, since this crate's `refactorize` pays the same *kind* of
fixed per-row bookkeeping (permutation arrays, `u_seq`/`row_owners`
construction in [`FtLu::new`]) HiGHS's own `buildFinish` does, just at a
different (higher, per `docs/lu_comparison_enomoto_vs_highs.md` §3.1 and
this trigger's own analysis §4) constant of proportionality that the
*other* coefficient ([`TICK_BUILD_LU_COEF`]) already carries — see
[`SYNTH_CLOCK_FACTOR`]'s own docs for why the *ratio* between the two
build-tick terms and the *solve*-side tick units is what calibration
actually tunes, not this constant in isolation.

### 定数 TICK_BUILD_LU_COEF

Per-nonzero-of-`(L+U)` coefficient for [`FtLu::build_tick`] — HiGHS's own
`buildSynthticTick` uses `60` for `(l_nnz + u_off) * 60`. Kept at HiGHS's
own value for the same reason as [`TICK_BUILD_M_COEF`]: this crate's
Markowitz `factorize` (§4/§5 of this trigger's own analysis, measured
when the kernel still used `BTreeMap`/`BTreeSet` storage rather than
today's [`KernelMatrix`]) is 3-25x more expensive *per nonzero* than
HiGHS's `HFactor::buildKernel` — the flattening narrowed that gap but
did not close it, and it is the gap's *existence*, not its exact size,
that makes a
*higher* [`SYNTH_CLOCK_FACTOR`] (not a higher `TICK_BUILD_*_COEF`) the
right lever: raising these two coefficients would inflate `build_tick`
but leave the *solve*-side tick (driven by [`TICK_SOLVE_NNZ_COEF`]) at
the same scale, which double-counts the same "our factorization is
slower" fact the factor calibration already absorbs once.

### 定数 TICK_BUILD_FLOP_COEF

Per-multiply-add coefficient of the elimination flop term in
[`FtLu::build_tick`] (S16, `ENOMOTO_T_TICK_BUILD_FLOP_COEF`). `0` (the
default) leaves `build_tick` exactly the HiGHS-shaped `m`/`nnz(L+U)` sum.

### 定数 TICK_SOLVE_NNZ_COEF

Per-nonzero coefficient applied to every solve-stage tick increment
(`R`-eta nonzeros touched, `U`/`U^T`-eta nonzeros touched, `L`-stage
reach-set size) — kept at `1` (i.e. `tick` is a plain nonzero count,
unscaled) so [`SYNTH_CLOCK_FACTOR`] alone carries the crate-specific
per-nonzero cost ratio between this crate's own solves and HiGHS's; splitting that ratio across two constants
(this one and the factor) would make calibration harder to reason about
with no accuracy benefit, since both only ever appear multiplied
together in the trigger's own comparison.
