# 改良履歴メモ: その他のモジュール (sparse / graph / interior_point / mip / model / solver / types / lib / Python パッケージ)

コード中のコメントから切り出した開発経緯・計測値・試行錯誤の記録。原文 (英語) を
ほぼそのまま残し、重複のみ軽く整理した。

---

## src/lib.rs

### クレート構成 (旧モジュールドキュメントの全文)

> Crate layout, grouped by function:
>
>   - `types`, `model`, `solver` — shared types, the PyO3 API entry point, and
>     top-level LP dispatch (thin orchestration, kept flat).
>   - `sparse` — the crate's one home for sparse storage: the `CsrMat`/`CscMat`
>     compressed pair and the conversions between them, the `SparseVec` sparse
>     vector and the `SparseAccum` accumulator its row merges run on, the sparse x
>     dense arithmetic built on all of them, and the `Csr` alias (faer's own
>     row-major type) plus the helpers that bridge to it. Used by `simplex`,
>     `presolve` and `interior_point::kkt` alike — no other module re-derives a
>     transpose, a `(index, value)` merge, or a mat-vec for itself.
>   - `presolve` (+ `presolve::{scaling,redundancy,propagate}`) — the **shared**
>     presolve pipeline (Ruiz scaling, redundant-equality removal, inequality
>     propagation) run identically by both `simplex` and `interior_point` — one
>     implementation of each pass, not two.
>   - `mip` — branch-and-bound for integer/binary variables, sitting on top of
>     `solver`.
>   - `simplex` (+ `simplex::lu`) — the **active** engine: a from-scratch
>     bounded-variable primal/dual revised simplex (Markowitz/Forrest-Tomlin
>     sparse LU, EXPAND anti-cycling, (dual) steepest-edge pricing), presolved via
>     `presolve`. `solver::solve_lp` calls straight into this.
>   - `interior_point` (+ its `qp`/`kkt` submodules) — the **inactive** IP-PMM
>     interior-point solver this project used before `simplex` was implemented
>     and verified. Kept in the module tree, unused, in case that path is wanted
>     again; also presolved via `presolve`.
>
> `legacy/` (`csr.rs`, `preprocess.rs`, `simplex.rs`) holds the very first,
> since-superseded implementation (a two-phase simplex with its own
> dense-oriented CSR type). It predates both `simplex` and `interior_point`
> above and is unrelated to either; left on disk, out of the module tree
> entirely (not even `mod`-declared here), purely for historical reference.

(注: その後古典的双対単体法は削除され、`simplex` は拡張双対単体法エンジンのみになった。)

### `env_str!` マクロを作った理由

> `std::env::var` costs an environment lock + a linear `environ` scan + a
> `String` allocation, and the solve path consults ~100 `ENOMOTO_*` flags per LP
> (callgrind: ~10% of `afiro`'s instructions).

### `SplitAlloc` (mimalloc / システムアロケータの振り分け) の計測経緯

> Presolve works in owned `Vec<Vec<_>>` row lists and rebuilds its CSR matrices
> several times per round, so on the small Netlib problems glibc `malloc`/`free`
> (incl. `malloc_consolidate`) measured at roughly a quarter of all instructions
> of a `solve()` call (callgrind); mimalloc's size-class free lists make those
> short-lived small allocations much cheaper. Routing *every* allocation to
> mimalloc, however, made several mid-size problems 20-80% slower in the simplex
> main loop (`fit2d`, `degen3`, `greenbea`, ... — no extra syscalls, so a
> placement effect on the large dense work vectors), so blocks of `LARGE` bytes
> or more stay with glibc exactly as before.

(注: 定数 `LARGE` は `params::alloc::MIMALLOC_SIZE_LIMIT` に改名。)

---

## src/sparse.rs

### モジュール全体: 一箇所に集約した経緯

> [`csr_rows`], [`CsrMat::from_faer`] and [`CsrMat::to_faer`] bridge the two, so
> a call site never has to open-code the
> `col_indices_of_row(i).zip(values_of_row(i))` dance that used to be
> copy-pasted across ~20 presolve files.

> This module is the crate's sparse-storage toolbox, so it deliberately carries
> the *complete* set of operations for each representation — both orientations
> of every product, the conversions in both directions, the whole sparse-vector
> algebra — rather than only the subset today's call sites happen to reach.
> `dead_code` is allowed here, and only here: an unused item in this file is
> API surface waiting for its caller, not the rot the lint normally catches, and
> every one of them is exercised by the unit tests at the bottom of the file.

### `SparseVec` を薄い型にした理由

> A newtype that hid the representation would force a conversion at every one of
> those boundaries; what this type adds instead is the *operations* that were
> previously re-implemented per call site — scatter/gather against a dense
> buffer, a dot product against a dense vector, pruning, and the density check
> that decides sparse-vs-dense dispatch.

### `scatter_dense`

> kept here so the several call sites that used to open-code a zero-fill plus a
> scatter share one implementation.

### `HybridVec` (Forrest-Tomlin eta 用の疎/密ハイブリッドベクトル) の導入経緯

> Each consumer of an eta is one of exactly two loops, a dot product against a
> dense vector ([`Self::dot_dense`]) or an axpy into one
> ([`Self::axpy_into_dense`]), and before this type existed each of those was
> written out per call site as a two-armed `match` on the representation — eight
> copies across FTRAN, BTRAN and both capture variants, every one of which had
> to be edited in lockstep to change anything. They are these two methods now.

### `HybridVec::pack_scaled_dense` の経緯

> Both of the vectors `simplex::lu`'s Forrest-Tomlin update creates on every
> pivot commit are of precisely this shape, each read straight out of a dense
> buffer the caller already has: the replacement column's off-diagonal part
> (`src = a_tilde`, `scale = 1.0`) and the new `R` eta (`src = e_tilde`,
> `scale = -old_pivot`). Both used to be `collect()`ed into a temporary
> `Vec<(usize, f64)>` only to be handed straight to [`Self::pack`], which then
> either kept that `Vec` (sparse arm) or scattered it into a freshly allocated
> dense array and dropped it (dense arm) — two throwaway heap allocations per
> update on a path `ENOMOTO_PROF_PHASES_EXT` measures at 13-20% of wall time, the
> last ones left there after `try_update`'s own two scratch buffers (see
> [`crate::simplex::lu::FtLu`]'s `scratch_a_tilde`) removed the rest.
>
> Going through the dense source directly removes both: the sparse arm allocates
> once at the exact final length (where `collect()` pays for geometric regrowth,
> a filtered iterator being able to report only an upper bound), and the dense
> arm allocates only the array it returns, filling it by copying `src` through
> rather than by scattering pairs into it.
>
> `nnz` is counted in a separate first pass because the sparse-or-dense decision
> needs it *before* either arm can start; that pass is a straight-line scan of a
> contiguous `f64` array with `skip` corrected for afterwards instead of tested
> for inside the loop, which is why reading `src` twice still costs less than
> the one predicated `collect()` it replaces.

その後の一パス化 (関数本体内コメント):

> One pass: every `(i, scale * src[i])` is written unconditionally into a
> reusable length-`len` scratch and the write cursor advances only for a kept
> entry (branch-free compaction), so the pair list and its count come out of a
> single read of `src`. The sparse arm then copies exactly `nnz` pairs out (same
> content and capacity as the former two-pass form); the dense arm is unchanged.

テスト `pack_scaled_dense_matches_collect_then_pack` の由来:

> `pack_scaled_dense` exists only as an allocation-free shortcut for what
> `simplex::lu`'s Forrest-Tomlin update used to spell out as a filtered
> `collect()` handed to `pack` — so what it must guarantee is not some property
> of its own but *equality with that original spelling*, in both
> representations and including the two filters (the skipped pivot slot, and
> products that come out exactly zero). ... `src[5]` is a value that is nonzero
> itself but whose scaled product underflows to zero, which the original's
> post-multiply `v != 0.0` filter dropped and this must drop too (otherwise
> `nnz`, and with it every refactorization trigger reading `fill_count`,
> silently drifts).

### `HybridVec::for_each_index` がクロージャを取る理由

> The `Box<dyn Iterator>` this replaces bought that with a heap allocation per
> update plus a virtual call per index, on a path that runs every simplex
> iteration; a generic closure monomorphizes into each arm instead, so both
> loops inline and nothing is allocated. The dense arm still scans the whole
> array (it has no index list to walk), exactly as before.

### `EpochMarks` の導入経緯 (3 つの独立実装の統合)

> The standard epoch-stamp (time-stamp) trick, and the reason it is worth a
> named type here is that this crate had grown **three** independent copies of
> it — [`SparseAccum`]'s own occupancy map, `simplex::lu::GpScratch`'s
> Gilbert-Peierls visited set, and `simplex::lu::FtLu`'s `U^T`-solve "needed"
> set — differing in exactly the way three hand-rolled copies of one idea
> differ: only one of the three handled counter wraparound, and the other two
> were silently wrong (a stale stamp from 2^32 passes ago reading as a live
> mark) if a solve ever ran long enough to wrap. [`Self::begin`] handles it
> once, for all of them.
>
> `u32` stamps rather than `u64`: these arrays are length `m` and are indexed in
> the innermost loop of every FTRAN/BTRAN, so halving their cache footprint
> matters more than never needing the wraparound branch (which costs one
> predictable compare per *pass*, not per index).

### `SparseAccum` (疎アキュムレータ) の導入経緯

> Every "combine two sparse rows" site in `presolve` used to be written as a
> `BTreeMap<usize, f64>` built per output row — `freevar::axpy_row`,
> `aggregator::axpy_row`, `doubleton::rewrite_row`, `sparsify`'s own target
> merge. That is `O(nnz log nnz)` with a pointer-chasing node per entry, for an
> operation whose natural cost is `O(nnz)` flat array writes; on a wide Netlib
> row (`wood1p`'s 2592-nonzero rows, say) the map's allocation churn dominates
> the arithmetic outright.
>
> The classical fix — Gilbert/Moler/Schreiber's sparse accumulator, the same
> structure `simplex::lu::GpScratch` already uses for its reach-set bookkeeping
> — keeps one `O(n)` value array alive across *all* the merges and uses a
> monotone epoch counter ...
>
> Determinism: [`Self::take_sorted`] sorts the accumulated pattern before
> emitting, so the result is *byte-identical* to what the `BTreeMap` versions
> produced — which matters, since this crate deliberately prefers ordered maps
> wherever iteration order can feed back into a later tie-break (see
> `types.rs::LinearExpr`). The accumulation order itself is the caller's call
> order, so floating-point results are identical too, not merely equivalent.

`SparseAccum::set` / `contains` / `remove`:

> `set`: last write winning — the semantics a `BTreeMap::from_iter` over a
> possibly-duplicated entry list has, which is what `load` needs to stay
> faithful to the merge code this replaced.
> `contains`: a membership test that replaces the `BTreeMap::contains_key`/
> `BTreeSet` lookups the subset checks in `sparsify`/`aggregator` used to do.
> `remove`: (a `BTreeMap::remove`) ...

### `axpy_row`

> This is the row-elimination kernel `presolve`'s substitution passes
> (`freevar`, `aggregator`) each used to carry their own `BTreeMap` copy of.

### `csr_to_csc`

> the replacement for the "walk every row, `push` onto `columns[j]`" loops the
> presolve passes that need per-column access used to each write for
> themselves.

### `csr_row_iter`

> The one place the `col_indices_of_row(i).zip(values_of_row(i))` dance is
> written. It used to be open-coded at ~40 call sites across `presolve/*`, which
> is exactly the kind of duplication that lets two of them quietly disagree
> about whether to filter zeros.

### `CsrRowBuilder::finish` で faer の検証を省いた理由

> faer's `new_checked` re-validation (a second pass over every column index; ~4%
> of a small LP's `solve()` under callgrind) would only re-prove this.

### `CscMat::from_entry_stream`

> Written directly, that is `vec![Vec::new(); n_cols]` plus a `push` per entry —
> one heap allocation per column, each then grown by reallocation — for a
> structure that is read-only the moment it is finished. Streamed through here
> it is the usual two allocations and one counting sort, with no intermediate
> concatenation of the blocks either.

---

## src/graph.rs

### `max_bipartite_matching` を反復版 DFS にした経緯 (degen3 のスタックオーバーフロー)

> Iterative augmenting-path DFS — an explicit stack instead of real recursion,
> for exactly the reason [`tarjan_scc`]'s own docs give for doing the same thing
> to Tarjan's algorithm just below: an augmenting path's length is bounded only
> by `min(p, n_cols)`, which can run into the thousands on this crate's own
> problem sizes, and a genuinely long one previously *did* overflow the call
> stack on a production Netlib instance (`degen3`) once a presolve change
> shifted which rows got fed to this matcher in what order — this crate's own
> project history has the full incident.

深い増加路テスト (`max_bipartite_matching_handles_a_deep_augmenting_chain_without_overflowing_the_stack`) の経緯:

> Built in two phases so the whole test stays `O(N)` rather than `O(N^2)` (an
> earlier version of this test listed each row's *previous*-column preference
> first, which forces recursion during every single row's own initial placement
> below — still correct, but O(N) work apiece made the whole test O(N^2) and far
> too slow to run routinely; the swapped preference order here keeps every one
> of the first `N` rows' own placements `O(1)`, so the only genuinely deep call
> is the one probe row added afterward).
> ...
> That single call is the exact shape (long, ultimately-failing recursive
> descent) that overflowed the call stack with a genuinely recursive DFS on a
> large production instance (`degen3`) — `N = 1_000_000` here comfortably
> exceeds anything a 1-2MB call stack could survive recursively ...
>
> `N = 100_000` rather than something larger: `max_bipartite_matching`'s own
> per-row `vec![false; n_cols]` (one fresh visited buffer per top-level call,
> `n_cols` long) makes the *whole test* — not just the probe — `O(N^2)`
> regardless of how cheap each individual row's own search is, so this is chosen
> as the smallest value that still leaves no realistic doubt (100,000 call
> frames is far beyond what any plausible thread stack could hold) without
> costing more than a fraction of a second here.

(注: 現在の実装は visited をエポックスタンプ (`u32`) で持つため、行ごとの
`vec![false; n_cols]` は既に無い。)

`max_bipartite_matching_recursive_reference` は「反復版に書き換える前の再帰版」を
テスト用の参照実装として残したもの。

### `dulmage_mendelsohn_blocks_topological` が未使用の理由

> **Currently unused in production** (kept, with tests, for a possible future
> revisit): `simplex::lu::factorize` was extended to use this for a
> block-triangularized basis-matrix factorization, validated for correctness,
> then reverted after measuring a net ~4% aggregate regression on the Netlib
> benchmark — see that function's own doc comment for the full write-up (wins
> on a few block-angular instances outweighed by a broad tax elsewhere from
> paying bipartite-matching cost on every refactorization, mirroring why HiGHS
> itself only does cheap degree-1 peeling here, not full matching-based
> decomposition).

---

## src/interior_point.rs / interior_point/{qp,kkt}.rs

### 内点法エンジンの位置付けの変遷

> Preprocessing: the problem is put through the same shared, *extended* presolve
> pipeline (`presolve::run_extended` ...) `simplex.rs` uses, rather than the
> plainer elimination-free `presolve::run` this module used before — every
> technique ported into that shared pipeline (including ones that eliminate a
> variable's slot entirely) now benefits this engine too, not just the simplex
> one.

> **Reachable, not default**: `solver::solve_lp` dispatches here only when
> `Model.solve(root_solver="interior")` is requested explicitly — the default
> (`root_solver=None`/`"simplex"`) goes straight to `simplex::solve_lp_dual`
> instead. Kept fully reachable rather than deleted, both as a fallback and so
> the two independent implementations can be run against the same input and
> cross-checked directly (`simplex.rs`'s own `*_matches_independent_ipm_solver`
> tests do exactly this).

(注: 旧ドキュメントは KKT 組立を `spkkt.rs`、Workspace を `ipm.rs` と呼んでいたが、
現在のファイル名はそれぞれ `interior_point/kkt.rs`、`interior_point.rs`。)

`unscale_with_substitutions`:

> the interior-point counterpart of `simplex.rs`'s `unscale_result`, needed now
> that `solve` below runs the same `presolve::run_extended` pipeline simplex.rs
> does instead of the elimination-free `presolve::run`.

`solve_lp`:

> Kept alongside `solve` rather than only reachable through `simplex.rs`'s test
> helper now that `Model.solve`'s `root_solver` argument can select this path
> directly.

要素ごとのベクトル演算を rayon 並列化していることについて:

> For the problem sizes this crate targets these vectors are short enough that
> rayon's dispatch overhead can outweigh the win — real payoff shows up as
> `n`/`p`/`m` grow — but the operations are correct and safe to parallelize at
> any size, unlike e.g. a triangular solve's inherent step-to-step dependency.

### `qp.rs`: 変数のシフト/分割をしない理由

> no shift/split preprocessing (that was only ever needed by the simplex
> method's ">= 0" requirement; the interior point method below handles
> arbitrary bounds natively).

### `kkt.rs`: `PARALLELISM` (faer の rayon 並列) を残した経緯

> A run-to-run nondeterminism investigation on a highly degenerate Netlib
> instance briefly disabled `faer`'s `rayon` feature crate-wide — tracing the
> residual nondeterminism there to `faer`'s own internal rayon usage in
> `presolve::redundancy`'s `ColPivQr`, *not* this module — but reverted it:
> doing so also removed real, substantial parallelism `faer`'s dense linear
> algebra gets from `rayon` on plenty of *other* Netlib instances, measured as a
> ~20% aggregate slowdown across the benchmark set with individual problems up
> to 2x slower. The nondeterminism is diagnosed but deliberately left as-is: not
> worth that trade for determinism on one pathological instance.

(注: 定数は `params::interior_point::KKT_PARALLELISM` に移動。)

### `params::interior_point` の presolve 系定数の由来

> `PROPAGATION_PASSES`: The underlying technique is itself iterative until a
> fixpoint; two rounds here mirror how Gurobi's presolve caps propagation passes
> per presolve round rather than iterating to convergence.
>
> `PRESOLVE_ROUNDS`: mirrors `simplex.rs`'s own `PRESOLVE_ROUNDS` (see that
> constant's own docs for why this is a cap, not a fixed count: `run_extended`
> itself stops early once a round converges).
>
> `ROWSINGLETON_COLSINGLETON_INNER_ROUNDS`: mirrors `simplex.rs`'s own
> `ROWSINGLETON_COLSINGLETON_INNER_ROUNDS` (see that constant's own docs for why
> this inner pair can have more to find after its own first pass, why
> `doubleton` isn't part of this inner repetition, and for the fixpoint check
> that stops it short of this cap).

---

## src/mip.rs

> Simple depth-first branch-and-bound ... Each node re-solves the LP relaxation
> with tightened variable bounds (no warm start) — simple and correct, adequate
> for the problem sizes this MVP targets.

---

## src/model.rs

### 無限大の変数境界を受け付けるようにした経緯 (旧 BIG_M 不変条件)

> A variable's bounds *may* be genuine `+/-inf` — `simplex.rs`'s presolve
> pipeline (`colsingleton`/`doubleton` in particular) can eliminate a truly free
> variable's row entirely, at zero replacement-row cost, exactly the way HiGHS's
> own free-column-singleton substitution does; substituting a finite `BIG_M`
> sentinel here instead — the old invariant this module used to enforce — would
> hide that from presolve and force it to re-materialize the variable's (fake)
> box bound as real rows on every such elimination, capping how far a chain of
> them can cascade. Only `simplex.rs`'s own `Tableau` (the dual-feasible crash,
> in particular) still needs every *surviving* variable to have two finite
> bounds — `build_std_form_presolved` substitutes `BIG_M` for any genuine
> infinity presolve didn't eliminate, but only *after* presolve has had its
> chance, not before.

(`add_variable` の旧ドキュメント: "see this module's own docs for why that is no
longer rejected here.")

---

## src/solver.rs

> `Simplex` (`simplex::solve_lp_dual`, the bounded-variable dual revised
> simplex) is the default, having replaced the original IP-PMM (PIQP-style
> interior point) path as the primary engine; `Interior`
> (`interior_point::solve_lp`) is kept fully reachable rather than deleted, both
> as a fallback and so the two independent implementations can be run against
> the same input and compared directly.

---

## src/types.rs

### `LinearExpr` が `BTreeMap` を使う理由 (degen3 の実行時間二峰性)

> `BTreeMap`, not `HashMap`: `presolve::build_a_g`/`simplex.rs`'s
> `build_std_form` both materialize a row's final term order by iterating
> `coeffs` directly (`.coeffs.iter().collect()`) — with a `HashMap`, whose
> default `RandomState` reseeds every process, that order (though always the
> same *set* of terms) could differ between separate runs of the identical
> binary on the identical model. On most problems that never surfaces (nothing
> downstream cares which order a row's terms arrived in), but on a highly
> degenerate one — many exactly-tied Markowitz/ratio-test/normalization
> decisions, which is what "degenerate" means — a row's incoming term order can
> be the one thing deciding which of several equally-valid choices an algorithm
> makes, cascading into a completely different (though equally correct) pivot
> sequence and wall-clock time. Prime suspect for exactly this kind of symptom
> on Netlib `degen3` (bimodal wall-clock time, ~0.8s vs ~3.0-3.5s, across
> repeated runs of one binary) surviving even after every `rayon`
> parallel/sequential choice elsewhere in the crate was made a fixed,
> size-based decision (see `simplex.rs`'s `RAYON_SIZE_THRESHOLD`) ruled out
> thread-scheduling nondeterminism as the cause. `BTreeMap` iterates in a fixed
> (ascending-key) order regardless of process/seed, removing the discrepancy at
> its source rather than downstream at each consumer.

### `RootSolver` で両エンジンを残している理由

> keeping both reachable, rather than deleting the interior-point path once
> `simplex` became the default, is what makes an apples-to-apples comparison
> between them possible on the exact same problem.

---

## python/enomoto_solver/benchmark_highs.py

### 無限大境界をそのまま渡すようになった経緯

> `model.rs::add_variable` now accepts genuine `+/-inf` bounds directly (see its
> own docs) — this crate's presolve pipeline substitutes a finite `BIG_M`
> internally only for whatever survives presolve without being eliminated
> outright (`simplex.rs::build_std_form_presolved`), not before presolve ever
> runs. So every Netlib problem here is handed to this crate exactly as HiGHS
> itself reads it from the `.mps` file, MPS `+inf` bounds included — no bound
> substitution happens in this script at all anymore.

### 問題ごとにサブプロセスで解く理由

> this crate's simplex engine `panic!`s (rather than returning an error) on a
> handful of known-hard Netlib instances (e.g. `cycle`, named for exactly the
> degenerate-pivoting behavior it stresses) when a refactorization hits a
> numerically singular basis — a Rust panic crossing the PyO3 boundary aborts
> the whole interpreter, which would otherwise take the entire batch down with
> one bad problem. A subprocess also gives `--timeout` real teeth (a hung/slow
> solve is simply killed), which an in-process call has no way to do.

### `_build_our_model` の二乗オーダー解消 (fbcec94)

> highspy copies the whole vector on every attribute access, so read each one
> exactly once — per-element `lp.col_lower_[j]` is O(n^2).
