# 改良履歴メモ

ソースコード中にあった「改良の経緯」のコメント (試した案・計測値・Netlib での確認結果・
却下/撤回した変更・バグ調査の記録・HiGHS との比較など) を、コードから切り離して一か所に
まとめたもの。コード側には現在の動作を説明する簡潔な日本語ドキュメントだけを残している。

- 本文は元コメントの英語のまま (翻訳はしていない)。見出しは日本語。
- 各項目の行番号・識別子名は整理前 (コミット 4ab948b) のもの。整理時の主な改名は末尾の「改名一覧」を参照。
- 本メモ中の論文の節・命題番号 (§4.5, Lemma 4.1, `prop:bfrt` など) は当時の版のもの。コード側の参照は
  現行版 (`paper.tex`) の番号に更新済み。
- 閾値・許容誤差などの定数は `src/params.rs` に集約してある。各定数の値の決め方の経緯も本メモにある。
- 削除済みの古典的双対単体法 (`solve_lp_dual_on`)・BIG_M フォールバックに付いていたコメントは
  本メモには含まれない。必要ならコミット 2595475 より前の git 履歴を参照。

## 目次

- 単体法共通 (src/simplex.rs)
- 拡張双対単体法 (src/simplex/extended_dual.rs)
- 疎 LU 分解 (src/simplex/lu.rs)
- 前処理コア (presolve.rs / aggregator / redundancy / scaling / propagate / colsingleton / doubleton / freevar)
- 前処理 小規模モジュール群 (src/presolve/*.rs)
- その他のモジュール (sparse / graph / interior_point / mip / model / solver / types / lib / Python パッケージ)
- stormG2_1000 対応: 反復あたりの `O(m)` パス削減 (2026-09-25)

## 単体法共通 (src/simplex.rs)
### src/simplex.rs

コード中のコメントから移した改良・計測・却下案の経緯。本文は元の英語のまま (重複のみ整理)。

#### モジュール全体

- There is no classical dual method or `BIG_M` fallback any more: when the extended solver gives up, the solve reports `Status::NotSolved`. (The classical dual simplex was deleted; the connected-component split now runs on the extended side, in parallel.)
- Bounds: `Tableau` and the primal method still assume finite structural bounds — they only ever see a basis `extended_dual` has already brought within its true bounds. Slack columns of `<=`/`>=` rows are genuinely one-sided (`[0, inf)`), so infinite-bound handling is still alive in the ratio test/EXPAND machinery — just never for a structural variable.
- Parallelism (presolve yes, the per-iteration loop no): `crate::presolve` (scaling, redundancy, propagation, DualFix, ColSingleton) runs once per solve and is parallelized with rayon throughout. Every loop inside the dual simplex's and `run_phase`'s per-iteration bodies (chuzr, chuzc1's candidate scan, the reduced-cost/DSE/steepest-edge weight updates) is embarrassingly parallel in the same sense, but is deliberately sequential: profiling them on this crate's target problem sizes (~1000 variables, a few hundred rows) found rayon's per-call dispatch overhead alone — paid hundreds of times per solve, once per iteration — costing *more* than the rest of the dual simplex loop combined, and removing it measured roughly a 5x end-to-end speedup. A large-enough problem could tip this back in rayon's favor, but no such threshold is implemented; if this module is ever aimed at dramatically larger LPs, re-profile before reaching for `into_par_iter()` again rather than assuming it helps.

#### 関数 max_iters_for (定数 MAX_ITERS_FLOOR / MAX_ITERS_CEILING / MAX_ITERS_SCALE)

- Size-scaled replacement for a flat iteration cap on every simplex main loop (classical primal/dual and the extended-dual module).
- **Why a flat constant was wrong:** a fixed `20_000`-iteration budget is independent of problem size, so it silently starves large instances instead of scaling with the amount of work a correct solve of that size can legitimately need — confirmed directly on Netlib `dfl001` (`m=6071`/`n=12230`, presolved to `m=4554`/`n_total=9773`): the extended-dual loop needs 22,015 iterations to reach the exact HiGHS objective, ~10% over the old flat cap. Hitting that cap doesn't fail loudly — `extended_dual::solve_lp_dual_extended` returns `None` and the solve reports `Status::NotSolved` (before that status existed, the caller fell back to a classical `BIG_M`-substituted path, which returned a **wrong** objective on `dfl001`: `11264657.2` vs HiGHS's `11266396.0`, ~1.5e-4 relative error). See the `dfl001-bottleneck-max-iters-cap` memory for the full measurement.
- Bland's-rule anti-cycling already gives a textbook finite-termination guarantee independent of this cap — this function exists only to bound the practical wall time, so generous headroom is the right tradeoff over a tight one.
- `MAX_ITERS_FLOOR` (20,000): the crate's own historical fixed value, kept as a lower bound so every small/medium instance that already solved within it (the whole Netlib 73-problem `--max-vars 3000` set, per `netlib_benchmark_workflow`) sees no behavior change at all.
- `MAX_ITERS_CEILING` (2,000,000): a defensive backstop, not a value any known Netlib instance approaches (the largest, `dfl001` at `m=4554`/`n_total=9773` post-presolve, needs `MAX_ITERS_SCALE * (m+n_total) = 286,540`), so a pathological future instance can't turn an unbounded-looking cycle into a multi-hour hang.

#### 関数 update_verify (定数 UPDATE_VERIFY_TOL)

- `pub(super)`: `extended_dual::solve_lp_dual_extended` reuses this exact function and `UPDATE_VERIFY_TOL` for its own pivot element (`a_p[q]` vs `alpha_full[r]`) — the agreement this checks for has no `Affine1`-vs-`f64` dependency at all (both values are always `M`-independent structural quantities), so there is nothing for that module to generalize, only to call.
- Selection rules (`chuzr`/`chuzc`/BFRT/Harris/DSE/Devex) are never touched by this check — by the time it runs, `p`/`q`/`theta_q` are already decided.
- `UPDATE_VERIFY_TOL` (trigger (5), HiGHS `HEkkDualRow::updateVerify` equivalent): both values are exact in infinite precision; a real gap means the Forrest-Tomlin eta chain has already drifted enough to misrepresent `B^-1` by this iteration, *before* that error is baked into `x_B`/`d`. Deliberately a *different, earlier* signal than the other triggers: trigger (1) (`FT_RESIDUAL_TOL`) only re-checks `‖A_B x_B - rhs‖` every `FT_CHECK_INTERVAL * RESIDUAL_CHECK_MULTIPLIER` iterations, so a bad pivot can be committed (and compounded) for up to that many iterations before it's caught; trigger (2) (`FT_MIN_PIVOT`) only rejects a pivot small in an absolute sense, which says nothing about whether the value itself is accurate. `updateVerify` checks every pivot, immediately, using values already computed as byproducts of this iteration's BTRAN/PRICE and FTRAN — no extra BTRAN/FTRAN.
- Chosen at `1e-7`, two orders of magnitude above the `~1e-9` (`TOL`) rounding-noise floor a healthy sparse dot-product/triangular-solve pair of this size exhibits, and matching `FT_MIN_PIVOT`'s order of magnitude rather than `FT_RESIDUAL_TOL`'s much looser `1e-4` (that bounds a whole-basis aggregate residual after many updates, not one freshly computed pivot pair). Too tight: fires on ordinary floating-point noise and forces far more refactorizations than justified; too loose: never fires before trigger (1) would, making it dead code. `1e-7` is the recommended starting point from the port's own spec, not yet independently re-tuned against the Netlib set beyond the sweep recorded in this feature's commit message — re-measure with `ENOMOTO_PROF_UPDATE_VERIFY` before moving it.

#### 構造体 PrimalStallState (定数 STALL_PROGRESS_EPS)

- Bland's-rule (1977) last-resort anti-cycling fallback for the primal method — the same role `extended_dual`'s own `stall_count`/`bland_mode` locals play for the dual method, ported here once a real degenerate Netlib instance (`cycle`) showed that EXPAND alone, plus `PRIMAL_HARRIS_TOL`'s pivot-conditioning widening, still isn't always enough: a long-enough run of essentially-zero-progress pivots can still walk the basis into a state `run_phase`'s own mid-solve refactorization finds numerically singular.
- `stall_limit` is scaled to problem size the same way the dual method's own `stall_limit` is (`(5 * m).max(500)`, now `PRIMAL_STALL_LIMIT_PER_ROW`/`PRIMAL_STALL_LIMIT_MIN`). Persists across the phase-1/phase-2 boundary like `ExpandState` does: a stalling run spanning the boundary shouldn't get its counter reset back to zero for free.

#### 関数 partial_pricing_sampled (定数 PARTIAL_PRICING_THRESHOLD / PARTIAL_PRICING_GROUPS)

- A pure function of `(seed, iter, j)` rather than a `&mut` RNG threaded through the pricing loop, so the exact same call answers both "in the sample" and "in the rest" without recording which columns the first pass visited — and results stay reproducible run-to-run for the same problem, unlike a time-seeded RNG, which matters for this crate's cycling/regression tests. `seed` varies across problem instances (`n_total`-only-distinct LPs would otherwise always sample the same columns); `iter` varies pivot-to-pivot so a column that loses the draw isn't permanently excluded. splitmix64's finalizer (Steele, Lea & Flood 2014) chosen only for being cheap and adequately unbiased, not for any cryptographic property.
- Partial pricing is Dantzig/Forrest-Goldfarb-Reid-style grouping; on a wide problem most columns never come close to winning.

#### 構造体 SteepestEdgeState

- The update formula was re-derived from scratch (Sherman-Morrison applied to `B_new^-1 = E^-1 B^-1`, the same rank-one identity `sparse_lu::FtLu`'s eta updates use) rather than transcribed from a secondary source describing the Forrest-Goldfarb formula: that source rendered the update as `gamma_j + beta_j(1+gamma_t) - 2 beta_j tau_j`, but the derivation gives `beta_j` *squared* in the middle term, and this was confirmed empirically (`steepest_edge_weights_match_brute_force_recompute` compares the incremental update against `||B_new^-1 A_j||^2` computed by a fresh solve for every nonbasic column after a pivot). Unlike a wrong pivot in `sparse_lu`, an incorrect weight here could only degrade pricing quality, not correctness — but it was still worth resolving via direct evidence rather than trusting either source blindly.
- `update_after_pivot`: the per-column writes are disjoint, so safe to parallelize, but left sequential: for the column counts this crate actually sees, rayon's per-call dispatch overhead measured *larger* than the loop body itself. Fixed columns (`lb == ub`) are skipped because they never win `price_one`'s entering scan — recomputing their weight every pivot was pure waste (same reasoning as the dual method's PRICE-loop skip above `chuzc1`).

#### 構造体 StdForm / 関数 freeze_std_matrices

- `cols` exists purely so `Tableau::column`/`column_sparse` never have to scan every row looking for column `j`. Built once (`CscMat::from_rows`, a single O(nnz) counting sort) right after `rows` is finalized.
- The pair are this crate's own `CsrMat`/`CscMat` (one flat `(index, value)` buffer plus offsets each) rather than `Vec<Vec<(usize, f64)>>`: once presolve hands off the final matrix, it is read every pivot and never mutated again, so there is no reason to keep paying for one heap allocation per row/column the way a still-being-rewritten presolve pass does. Row order within each `cols.col(j)` is ascending; nothing downstream depends on it either way.
- `freeze_std_matrices`' `debug_assert` is load-bearing documentation, not paranoia: every walk that reaches the basis through the column view (`Tableau::basis_rows_sparse`, `Tableau::basis_residual_norm`, `extended_dual::refactorize`, `extended_dual::residual_norm`) visits columns in ascending index and therefore reproduces each row's entry order — and so the LU's own pivot-order tie-breaks and each residual's summation order — bit for bit, *provided* the rows were column-ascending to begin with. All three builders produce that (a presolved row's structural terms come out of an ascending faer CSR row through a monotone re-index, and its slack is appended last with the largest index).

#### 関数 build_std_form_presolved / 構造体 PresolvedForm

- Runs `presolve::run_extended` (the same shared pipeline `interior_point.rs` now also uses) rather than `colsingleton` alone: every substitution it returns is now in *scaled* coordinates (colsingleton used to run once, pre-scaling, specifically to avoid this — see `unscale_result` for how recovery order changes to match). The equality-row slack is fixed at `[0, 0]` — feasibility for it is phase 1's job or the dual-feasible crash's; this function constructs no basis. `G`'s single-variable rows are pulled back out as `lb`/`ub` (a variable's bound is represented as a bound, not as an extra slack variable).
- `pre.lb`/`pre.ub`/`pre.real_rows`/`pre.real_rhs` are reused directly instead of re-deriving them with a second `extract_bounds` call on `pre.g`/`pre.h` (which would just be undoing the row-folding `run_extended` did).
- Substituted variables fixed to 0: every variable eliminated by `doubleton`/`colsingleton` deliberately has *no* remaining bound rows of its own. But the rest of this file (the dual-feasible crash, `Tableau::new`'s nonbasic-at-lower start, the BFRT walk) assumes finite bounds — a genuinely infinite pair here is a phantom column the solver was never designed to represent, and was observed causing the dual method to report a false `Infeasible` on otherwise-feasible problems (confirmed by cross-checking against the primal method, which reached `Optimal` on the identical `StdForm`). Fixed to `0` ("finite and never read").
- Bound-shift: simplex-only (this function has no `interior_point.rs` caller — that engine keeps reading `pre.g`/`pre.h` unshifted) and done exactly once, on the fully presolved `lb`/`ub`, never re-run mid-presolve. A one-sided variable's only finite bound is exactly the one the dual-feasible crash / bound-flip logic will park it at nonbasic.
- Reflection of lower-unbounded columns (`x_j = ub[j] - y_j`): turns `(-inf, 0]` (nonbasic-at-upper, `extended_dual::delta_of` returns `delta_j = -1.0`) into the same shape as a naturally upper-unbounded column (`[0, +inf)`, nonbasic-at-lower, `delta_j = +1.0`), so every M-tracked column has the same direction and initial nonbasic value 0 — the same `sign[nj] * (x_free[nj] + shift[nj])` recovery the (former) free-column split relied on, generalized to a single reflected slot. Confirmed as a real, correctness-neutral win by a full 93-problem Netlib sweep (2026-09-21): every problem's status and objective matched the un-reflected baseline exactly, total solve time -3.75% (62.21s -> 59.88s), and the single largest problem (`dfl001`, 36s+) improved -5.3% with no large problem regressing.
- Column compaction: every `lb == ub` column (doubleton/colsingleton sentinel, or `dualfix`/forcing-row fixed value — pricing already can't tell) is dropped from the solve's column space entirely rather than kept as a slot every column-oriented loop has to check-and-skip and every row-oriented loop (the dual PRICE step) has to read-and-discard on every pivot. Its contribution is folded into the row rhs once — the same arithmetic `x_B = B^{-1}(b - N x_N)` already did, performed once instead of on every basis (re)computation. (Previously kept as an always-skipped `lb[j] == ub[j]` slot.)
- Free columns: a column still free on both sides (`presolve::freevar`'s documented residual case — a free variable appearing only in an inequality row) gets a single compacted slot — no more `x_j = x_j^+ - x_j^-` split: `extended_dual::hat_lower`/`hat_upper` track both sides symbolically (`M` on each). The one residual case — two free columns coupled only to each other — was reported as `NotSolved` at one point; now `extended_dual`'s cleanup parks one at `NbStatus::Zero`.
- Structural columns still carrying a genuine infinite bound are usually one-sided (free variables reachable through an equality row are gone by then, see `presolve::freevar`), but a free variable appearing only in inequality rows can still arrive both-sided.
- `ENOMOTO_DEBUG_PRESOLVE_SIZE`: `n_vars_out` is directly comparable to HiGHS's `getPresolvedLp` column count, not `variables.len()`.
- Rows go straight into one flat CSR buffer — the same matrices, entry for entry, that `freeze_std_matrices` builds from the equivalent `Vec<Vec<_>>`.

#### 関数 unscale_result

- Substitutions are evaluated in **reverse** discovery order and *before* `scaling::unscale_x`, not after: since every elimination now runs inside `run_extended`'s already-scaled pipeline, a substitution's `terms`/`rhs`/`coeff` are scaled-space values, so `value()` must be evaluated against the still-scaled output; the single `unscale_x` call at the end then converts the whole vector, substituted entries included.
- `shift` is added back in the same expansion step, before any `value()` call: a substitution's terms were computed in `run_extended`'s (unshifted) scaled space.
- Fixed values must already be in place: a substitution's `terms` can reference a variable that `dualfix`/a forcing row fixed outright.
- One reverse pass over the *shared* chronological log, not two per-kind passes: a `Sub` recorded before a later `ParallelCol` merge can reference the merge's own `kept` column, so that merge's `apply` must already have run (restoring `kept`'s pre-merge value) by the time the `Sub` reads it (see the parallelcols postsolve-order bug: greenbea off by 8e7).
- `n` is passed explicitly (equal to `orig_of_free.len() + fixed_values.len()`) rather than derived.

#### 構造体 Tableau とメソッド

- `basis_pos`: avoids an O(m) scan per lookup every time the sparse basis matrix or a residual is rebuilt.
- `Tableau::new` (tests only): structural variables were validated at the `model.rs` boundary to have finite bounds (historical module assumption), so every one starts nonbasic at its lower bound.
- `basis_rows_sparse`: column-driven via `std.cols`, touching only `nnz(A_B)`, where the row-driven form it replaced scanned all `nnz(A)` and discarded every nonbasic entry (on a real instance most columns are nonbasic, so that discarded work was the bulk of it). Ascending `j` keeps each row in exactly the order the row-driven scan produced, so the LU is bit-for-bit the same.
- `column` / `column_into`: both real per-iteration call sites (`run_phase`, the dual loop) go through the non-allocating `column_into`; the dual loop calls it every pivot, so a fresh `Vec` would be one more per-iteration allocation on top of the FTRAN/BTRAN ones `FtLu::solve_into`/`solve_transpose_into` already eliminate.
- `column_sparse`: those loops run once per nonbasic column every pivot; avoiding the O(m) fill turns an O(n_total * nnz) pivot into an O(nnz) one.
- `compute_rhs`: split out so the dual loop can get a ground-truth `rhs` for its periodic drift *check* without the check silently masking the drift by resyncing `x_B` first. Column-major over nonbasic columns, skipping `x[j] == 0.0`, rather than the row-major scan it replaced (one `nb_status` check per matrix entry, `O(nnz(A))` unconditionally). Nonbasic-at-0 is common (0 is the most common lower bound, and the bound-shift step widens that set further).
- `resync_basics`: the same "snap back to ground truth" refresh `d`'s `fresh_d` gets, at the same cadence (right after a refactorization).

#### 関数 perturb_costs (定数 COST_PERTURB_*)

- The dual analog of the primal method's EXPAND, but a genuinely different technique, not a direct port: confirmed against HiGHS's own source (`HEkk::initialiseCost`, `HEkk.cpp`), which perturbs costs for exactly this reason rather than using anything EXPAND-shaped for its dual simplex. EXPAND relaxes *primal* bounds because primal degeneracy (a tied ratio test) is what risks cycling there; the dual method's analogous risk is *dual* degeneracy — tied ratios in chuzc1/BFRT, already partly addressed by `HARRIS_RATIO_TOL` — but reduced costs have no bounds for an EXPAND-style relaxation to widen. Perturbing once, before any reduced cost is computed, generically avoids exact ties from the first iteration.
- Constants follow HiGHS: damp `max_abs_cost` (fourth root) if > 100, cap at 1 if fewer than 1% of columns are boxed, base `5e-7 * max_abs_cost`. Deterministic per-column hash instead of HiGHS's `numTotRandomValue_` array — this only needs "generically distinct" values, and determinism keeps a solve reproducible.

#### 関数 try_refactorize

- A basis reached by valid pivots is nonsingular in exact arithmetic, so `None` only happens once accumulated floating-point error (a run of near-`FT_MIN_PIVOT` pivots on a degenerate problem) has made it singular to working precision — historically the dual loop treated that as "this trajectory is numerically spent" and handed the problem to the primal method rather than crashing.
- `prev`'s pivot order is reused (`sparse_lu::factorize_reusing`); a reuse failing the threshold-pivoting or fill checks falls back to the full Markowitz search, so `prev` never changes which factorizations are accepted.

#### 関数 run_phase

- Returns `None` rather than treating a singular mid-solve refactorization like the iteration-cap fallback (which only means "still converging"): this can trigger after only a modest number of pivots, on a `t` whose primal feasibility may already have silently drifted — confirmed on Netlib's `cycle`, where trusting `t.x` at exactly this point produced a wildly wrong "optimal" objective instead of an honest failure.
- Work buffers hoisted out of the loop (and `Candidate` hoisted so `candidates_buf` can name it): this primal loop used to allocate a fresh `cost`/`y`/`a_enter`/`alpha`/`e_r`/`rho`/`w` `Vec` (and a fresh `candidates` vec) on *every* iteration — several m-length heap allocations per pivot (mirrors the dual loop's pre-loop buffer block).
- `ratio_pivot_tol` = `FT_MIN_PIVOT`, not `TOL`: a leaving row with `|alpha|` this small makes the new basis (nearly) singular — `try_update` rejects the update and the refactorization that follows fails, aborting this phase (Netlib `dfl001`'s cleanup handoff hit a `1.2e-9` pivot this way once the extended dual's path shifted slightly, and fell back to a from-scratch solve costing more than the whole dual run). Treating such rows as non-blocking is the usual primal ratio-test pivot tolerance (HiGHS `HEkkPrimal`'s `alpha_tol` reaches `1e-7` as well). `ENOMOTO_PRIMAL_RATIO_PIVOT_TOL_OLD` restores `TOL` (A/B only).
- Ratio-test relaxation: a row returning to feasibility gets no outward slack — EXPAND's relaxation targets rows that could newly become infeasible, which the classical (non-Phase-1) presentation of the algorithm is the only case that arises.
- In `bland_mode` the entering choice bypasses partial pricing (random sampling has no finite-termination guarantee), the same way the dual method's `bland_mode` bypasses DSE-based `chuzr`; the leaving tie-break switches to smallest basic-variable index (consistent, deterministic tie-breaking on both sides is what Bland's proof needs).

#### 定数 PRIMAL_HARRIS_TOL

- The primal analogue of `HARRIS_RATIO_TOL`: `run_phase`'s two-pass ratio test already widens its window by the EXPAND working tolerance (`expand.delta`, §4.2), but that tolerance is designed to be minuscule (order `1e-6`, per `EXPAND_DELTA_F`) — its job is proving a strictly positive step exists, not steering toward a better-conditioned pivot. When only one candidate row falls inside that razor-thin window, Pass 2's "largest pivot" tie-break must accept whatever pivot it has — confirmed on Netlib's `forplan` (unrelated to EXPAND/cycling: a perfectly ordinary non-degenerate phase-2 iteration, iteration 118, picked a pivot of `~2.4e-9`, well under `FT_MIN_PIVOT`, which made the mid-solve refactorization triggered by rejecting it find the *basis itself* singular — not a `factorize()` bug). The cost is a small bounded overshoot past the exact leaving bound, within the same "temporary infeasibility, cleaned up by `expand_reset_nonbasics`" tolerance EXPAND already accepts.

#### 定数 HARRIS_RATIO_TOL

- Huangfu & Hall §2.2.2's plain single-boundary BFRT (used until then) picks strictly the smallest-ratio candidate with no regard for its pivot `a_pj`. A near-zero pivot can make the basis numerically singular even though the LP is well-posed: confirmed on Netlib `wood1p`, where the basis `factorize()` rejected as singular had rank 242/243 with a smallest singular value at machine epsilon — not a `factorize()` bug, but a pivot the ratio test should never have accepted. The tolerance widens the window *backward* (see the BFRT loop) so a slightly smaller-ratio candidate with a much better pivot can be chosen, at the cost of bounded ratio sub-optimality (more iterations, not a feasibility violation). Inspired by HiGHS `HEkkDualRow::chooseFinal`/`chooseFinalLargeAlpha`, which can safely extend forward too, since its bookkeeping (a budget over total remaining infeasibility) isn't tied to a step-by-step walk.
- Sweep 1e-7 / 1e-5 / 1e-4 on the Netlib set was not monotonic: 1e-5 fixed `pilot.ja`, `pilotnov`, `qap8` but broke `bnl1` and added a false-infeasible on `pilot4`; 1e-4 was worse on both counts. A whack-a-mole pattern — the affected instances (`agg`, `cycle`, `degen2`, `degen3`, `maros`, `perold`, `pilotnov`, `stair`, `vtp.base`, ...) are known degenerate stress tests; the real gap was that this dual method had no anti-degeneracy mechanism of its own (EXPAND was never ported to the dual side). Kept at 1e-7 (tied for best, smallest deviation from ratio-optimality).
- **Per-row-scaled Harris (`r_i + tol/|alpha_i| >= r_stop`) implemented and reverted**: +9.7% aggregate wall time, concentrated in degenerate instances (`cycle`, `grow22`, `degen3`, `perold`, `fit1p`, `bnl1`, `pilot4`), zero instances newly fixed, and on `cycle` a silently wrong optimum. It widens admission using the *substitute* candidate's own pivot — backwards from what the safety argument needs (a bound on the *skipped* candidates' worst-case tolerance).
- **Direct port of HiGHS `chooseFinalLargeAlpha` (read from source) also tried and reverted**: absolute pivot-magnitude floor, substitution only when `stop_idx`'s pivot fails it, nearest-clearing-candidate wins. It *still* broke `cycle` (a different wrong objective). A/B isolated the fault to substitution *at all*: with pass 2 short-circuited, `cycle` solves correctly — this crate's flip/theta/dual-update accounting doesn't defend dual feasibility for the candidates skipped between a substitute and `stop_idx`, only primal non-overshoot of the flip itself; the flat window stays because it is empirically narrow enough on the full Netlib set, including `cycle`.

#### 定数 PRIMAL_FEAS_TOL

- Matches HiGHS's default `primal_feasibility_tolerance` (1e-7) rather than reusing `TOL`'s 1e-9: confirmed on Netlib `agg` (bounds/data reach the millions) that `TOL` alone is too tight for a basic variable's accumulated floating-point noise once the problem's natural scale is large — a variable sitting `-6.25e-9` off its `lb = 0` cleared the old `> TOL` threshold, was treated as a genuine infeasibility for chuzr to "fix", and sometimes left chuzc1/BFRT with no genuine way to fix it, surfacing as a false `Infeasible` report on an actually-optimal LP.

#### 定数 FT_CHECK_INTERVAL / RESIDUAL_CHECK_MULTIPLIER / FT_RESIDUAL_TOL / FT_BUMP_LIMIT_FACTOR / FT_MAX_UPDATES

- `RESIDUAL_CHECK_MULTIPLIER`: measured residuals stay around `1e-11..1e-12`, seven to eight orders of magnitude below the `1e-4` trigger, so checking much less often still catches real drift long before it matters, while no longer paying `compute_rhs`'s full-matrix cost on every one of trigger (3)'s cheap `fill_count()`-only checks. Trigger (3) still runs every `FT_CHECK_INTERVAL` iterations and still supplies `rhs` for the resync when it fires.
- `FT_RESIDUAL_TOL` (1e-4): essentially never fires (measured residuals 1e-11..1e-12, far below even the old 1e-6) — trigger (3) is what governs refactorization frequency — but it costs nothing to leave a wide safety margin.
- `FT_BUMP_LIMIT_FACTOR` (64): the trigger that actually fires repeatedly (instrumenting a 1000-variable benchmark: every refactorization past the first was this one); `fill_count()` grows *compounding*, roughly doubling the per-update fill rate every ~50 updates. A/B sweep (total wall time across 10 solves): `4` and `2` cost ~10% and ~65% *more* than `8` (refactorization has its own fixed cost — full Markowitz factorize plus fresh reduced-cost recompute); `16`/`32`/`64` measured ~5%/~12%/~15% *faster* than `8`; `128` gave no further improvement. `64` kept with margin before a large eta file's numerical safety would need re-examining. Re-benchmark if the typical problem shape changes significantly.
- `FT_MAX_UPDATES` (300): measured to be the very first refactorization in a solve (fired once, right around 100, before trigger (3) got a chance) — raising it lets a solve run further into trigger (3)'s eta-fill budget.

#### 定数 EXPAND_K

- The paper's own worked example uses 10000 for large industrial LPs, but this project's test-scale LPs warrant a much shorter cycle so resets are actually exercised.

#### 定数 RAYON_SIZE_THRESHOLD / 構造体 DseState.use_parallel

- Replaces an earlier "run both ways once, time them, keep the faster" self-calibration at all three call sites (`chuzr`'s row scan, `DseState`'s weight update, `scaling::compute`'s column-norm fold): the `#[ignore]`d microbenchmarks (`rayon_threshold_microbench`, `dse_update_rayon_threshold_microbench`, `col_norm_fold_rayon_threshold_microbench`) never found `rayon` beating a plain sequential scan at *any* size tried — not at `n`/`m` = 200,000, not even at 4,000,000 rows for the scaling fold — so the live race was pure overhead on every solve, and one more source of run-to-run nondeterminism (see the `chuzr` tie-breaking fix, prompted by exactly that). `100_000` is an order of magnitude above the largest size the microbenchmarks found rayon still losing at — a nominal safety valve, not a proven crossover point (none was found).
- Microbenchmark result recorded in the test docs: rayon was slower than a plain sequential loop for a trivial per-element workload at *every* size tried, including 200,000 elements (~2.7x slower there; ~17x slower at 10,000).

#### 定数 RUIZ_ITERS / PROPAGATION_PASSES / PRESOLVE_ROUNDS / ROWSINGLETON_COLSINGLETON_INNER_ROUNDS

- `RUIZ_ITERS` (10): the same value `interior_point.rs` used before `crate::presolve` was extracted to be shared with this module.
- `PRESOLVE_ROUNDS`: later rounds can unlock reductions an earlier round's static structure couldn't yet see; the fixpoint check stops it early, so raising it costs nothing on problems that converge early.
- `ROWSINGLETON_COLSINGLETON_INNER_ROUNDS`: the inner pair can have more to find after its first pass (e.g. colsingleton turning a row into a fresh row singleton); `doubleton` isn't part of this inner repetition (measured net regression when it was; see memory "doubleton inner-round chaining measured: 0/93").

#### 関数 connected_components_of_std_form / split_std_form / solve_std_form_decomposed (定数 PARALLEL_COMPONENT_MIN_VARS)

- Why checking once against the fully presolved `std` is enough: every stage of `presolve::run_extended` only removes rows, tightens bounds, or substitutes a variable out in terms of others already sharing its row — none can introduce new coupling. So the presolved `std` shows the same or a *more* separated structure; one check catches both "originally separable" and "presolve revealed separability".
- `has_row` lets `solve_std_form_decomposed` decide cheaply *before* calling the allocation-heavy `split_std_form`: building and then discarding hundreds of throwaway single-variable `StdForm`s (this crate's first version of this optimization) was itself a measurable regression.
- Zero-structural rows: `doubleton` can rewrite a surviving row down to only its slack (every structural coefficient cancels) while its rhs stays nonzero, deliberately kept as a Farkas infeasibility witness the *solver* is meant to catch (see `contradictory_equality_rows_detected_infeasible` and `doubleton`'s module docs). Silently omitting it from every component would erase that witness; bailing out of splitting keeps the already-correct undecomposed path.
- `split_std_form` is one combined `O(nnz)` pass — not one call per component each rescanning every row, which is `O(components * n_rows)` and was this feature's first, measured-as-a-real-regression implementation (Netlib instances routinely produce hundreds of components post-presolve, making that cost dominate the solve time it was meant to save).
- What Netlib looks like post-presolve: most instances split into **hundreds** of components (`fit1p` 628, `sctap3` 624, `ganges` 530, `modszk1` 422, …), always as one large remaining component (`fit1p`'s is 1050 of its 1677 variables) plus many singletons. No benchmark instance has two or more components clearing the `real_components` bar, so the mechanism is dormant on that set by design.
- Why *two or more* real components are required: splitting off trivial singletons buys nothing (they were static nonbasic values already) while costing (1) hundreds of throwaway `StdForm` allocations and (2) a changed pivot path: compacting global indices changes which candidate wins an exact pricing tie, and on Netlib `25fv47` that alone took iterations from 3,092 to 11,468 for the identical answer.
- `PARALLEL_COMPONENT_MIN_VARS` (200): solving a whole LP is substantial work, unlike the deliberately sequential per-iteration loops, so rayon's dispatch cost is negligible at this granularity. `200` is the size the user requesting this feature asked for directly, not independently tuned; no Netlib instance exercises the parallel path (every genuine split lands well under it).

#### 構造体 DseState

- `from_basis`: one BTRAN per row — the same kind of `O(m * nnz)` work `fresh_d` does after every refactorization, and far cheaper than the full cold restart it replaces on `forplan` (221 DSE + 245 Devex pivots). `FtLu::solve_transpose_unit_into` is only exact on a freshly-factorized `u_seq` (`update_count() == 0`); every current caller is a refactor-time refresh, but a factorization with updates falls back to the dense sweep rather than risking silent wrong weights.
- `update_after_pivot`: `wp_old` recomputed as `||rho_p||^2` rather than trusted off `self.w[p]`: because `wp_old` feeds *every other* row's update, a drifted `self.w[p]` gets re-injected into the entire vector the next time row `p` pivots, compounding with no self-correction — measured via `ENOMOTO_PROF_PHASES_EXT`'s `dse_rel_err` reaching >=100% relative error (vs. the true `||B^-T e_r||^2`) on the large majority of iterations on Netlib `fit1p`, far worse than the `degen3` case that motivated refreshing weights at `refactorize()` time — refactors alone are too infrequent (single digits per solve). `rho_p` costs nothing extra: every call site already computed it for PRICE. (Memory: fit1p iters 2614 -> 787.)
- Sequential branch: branch-free over every row including `p` so the plain zip compiles to packed SIMD — the same IEEE operations in the same order, bit-identical. Skipping `alpha[i] == 0.0` rows instead — exact too — was measured *slower* at the ~50% `alpha` densities of `dfl001`/`pilot87`: the unpredictable branch costs more than the division it saves.
- Disjoint per-row writes, so it parallelizes trivially — no fold/reduce needed, unlike `scaling::compute`'s max-accumulation.

#### 構造体 InfeasibleRows

- Eliminates the separate full `0..m` scan `chuzr` used to need every iteration; membership checks are folded into already-O(m) loops (BFRT combined-flip update, the main `alpha`-scaled primal step, the post-swap identity change). Mirrors HiGHS's `HEkkDualRHS::workCount`/`workIndex` (incrementally updated off the FTRAN indices touched each pivot, full rebuild after every basis resync/refactorization).
- `pub(super)`: `extended_dual` reuses this exact struct for its own (`Affine1`-valued) `x_B(M)` once its main loop moved to the same incremental design — the membership logic has no `f64`-vs-`Affine1` dependency, so duplicating it would only risk the copies drifting apart.

#### 関数 run_phase2_incremental

- The S4 alternative to `run_phase` for the polish -> primal handoff (`ENOMOTO_HANDOFF_INCREMENTAL=1`, default off; see `analysis/simplex_loop_20260924_113533.md` §3.2/§4 S4). `run_phase` re-derives everything every iteration: `x_B` from scratch (`compute_rhs` `O(nnz(A))` + an FTRAN), `y` by a BTRAN, every nonbasic reduced cost by a dot product, and the steepest-edge update's two BTRANs plus two dot products per nonbasic column. On Netlib `pilot87` that made one handoff pivot cost 2.3x a dual one.

#### 関数 solve_lp_dual_with

- An `Unbounded` reached by any route other than stage A (the `m == 0` shortcut, or combining independent components) is reported the same way as the trichotomy result, so the set of possible answers does not depend on which path solved the problem.

#### テスト

- `lp2_minimize_with_ge_and_eq` / `lp3_wide_bounds_dont_bind`: written when every variable needed two finite bounds (the project then did not support genuinely free/infinite-bound variables).
- `dual_bfrt_flips_multiple_variables_in_one_iteration`: by hand — chuzr always picks the only row; chuzc1 sorts x0..x4 (ratios 1..5) ahead of x5..x9; BFRT fully flips x0..x3 before x4 is left as the real (exactly bound-hitting, degenerate) entering pivot, landing on the greedy optimum in a *single* iteration. Without BFRT the classical ratio test would push x0 to 5, past its upper bound of 1 — the incorrectness BFRT exists to prevent, not merely a performance optimization.
- `freevar_residual_both_sides_infinite_only_in_inequality_rows_end_to_end`: historically this made `solve_lp_dual` report a false `Infeasible` (`extended_dual::delta_of` silently misclassified a doubly-infinite column as one-sided); first fixed by splitting into `x_j = x_j^+ - x_j^-` before this module ran, since superseded by native (unsplit) support.
- `mutually_coupled_free_variables_reach_a_correct_answer`: this shape used to bail to the classical `BIG_M` fallback; now `extended_dual`'s cleanup parks one column at `NbStatus::Zero` (its unit test `two_free_columns_tied_only_through_opposing_inequality_rows_park_one_at_zero`).
- `freevar_in_exactly_one_inequality_row_eliminated_by_presolve_end_to_end`: `presolve::freevar`'s then-new "exactly one inequality-row appearance" case substitutes x0=8-x1 and drops the row, rather than leaving it for `build_std_form_presolved`'s (former) `x_j=x_j^+-x_j^-` split.
- `freevar_eliminated_via_equality_row_folds_into_a_shared_inequality_row_end_to_end`: regression test for a real bug — `presolve::freevar`'s `A`-row elimination folded a substitution into every other `A` row but never into a `real_rows` entry it shared, silently leaving that row referencing a column pinned to `0`, corrupting Netlib `perold`, `pilot4` into a false `Infeasible`. Misreading would give a spuriously better-looking but infeasible objective of -25.
- `doubleton_chain_within_one_pass_recovers_correctly`: exercises `interior_point.rs`'s `unscale_with_substitutions` on a *chained* doubleton substitution, not just the single-substitution colsingleton case `colsingleton_substitutes_singleton_equality_and_folds_cost` checks via `solve_via_ipm`.
- `larger_lp_matches_independent_ipm_solver_and_exercises_ft_triggers`: cross-checked against `solve_via_ipm` (called directly rather than via `solver::solve_lp`, which now routes to this module) since the LP is too big to verify by hand.
- `solve_via_ipm`: `solver::solve_lp` dispatches to either engine on request (`types::RootSolver`), so this is the same dispatch `Model.solve(root_solver="interior")` uses, not a test-only shortcut.
- `steepest_edge_weights_match_brute_force_recompute`: see `SteepestEdgeState` above (secondary source vs. derivation over whether `beta_j` is squared).

## 拡張双対単体法 (src/simplex/extended_dual.rs)
`src/simplex/extended_dual.rs` と `src/params.rs` の `mod extended_dual` から取り除いた、開発経緯・計測結果・
試して不採用にした案・バグ調査の記録などのコメントを、元の英文のまま項目ごとに集めたもの。
コード中のコメントは現在の動作を説明する簡潔な日本語に置き換えてある。行番号は整理前
(コミット 4ab948b)のファイルでの位置。整理時に改名した識別子は、この記録では旧名のまま
(`sl_*` → `shortlist_*`、`_iter` → `iter_idx`、`fd_cb`/`fd_y` → `fresh_d_cb`/`fresh_d_y`、
`cm_off`/`cm_pos`/`price_cm` → `col_entry_start`/`price_pos_of_col_entry`/`col_entry_of_price`、
`drift_r0` → `drift_resid_after_refactor`、`n_zero_nb` → `n_zero_nonbasic`、関数内の `REL_TOL` → `LEX_REL_TOL` など)。

### src/simplex/extended_dual.rs

#### モジュール先頭ドキュメント

(元の位置: L1 付近)

Extended dual simplex with BFRT — the one LP engine behind
`solve_lp_dual`, for every presolved `StdForm` whether or not some
*structural* column still carries a genuine infinite bound (with none
left, the M-side bookkeeping simply stays empty). Called once per
independent connected component by `simplex::solve_std_form_decomposed`.
A `None` return (one of the "should be unreachable" bail-outs below) is
reported to the user as `Status::NotSolved`; there is no other solver to
fall back to.

### The M→∞ symbolic trick, and why no number ever stands for `M`

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

### Preconditions this module relies on (established by earlier stages)

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

### Deliberate simplifications versus the classical method

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

#### モジュール prof_phases(診断カウンタ)

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

#### 関数 Affine1::cmp_lex

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

#### 関数 bfrt_reached

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

#### 列挙型 MSide(S 制限の撤回)

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

#### 関数 nb_value_affine

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

#### 関数 refactorize

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

#### 関数 residual_norm

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

#### 関数 compute_rhs_affine

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

#### 関数 resolve_x_b_into

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

#### 構造体 RowDevCache

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

#### 構造体 RowBounds

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

#### 関数 row_deviation_plain

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

#### 関数 fresh_d_into

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

#### 関数 crash

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

#### 関数 refine_zero_cost_placement(実験的・効果なし)

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

#### 関数 trial_row_ratio

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

#### 構造体 ColCache

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

#### 本体: スラック列のコストを摂動しない理由

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

#### 本体: delta と cache_orig

(元の位置: L2242 付近)

`delta[j]` (see [`delta_of`]'s own docs — currently the reverted,
un-`S`-restricted form: every one-sided-unbounded or genuinely free
structural column is flagged, regardless of `active_cost`'s sign).
`cache_orig` holds the true `M`-affine bounds (`hat_l`, `hat_u`) —
what `finish`'s cleanup and the `m == 0` shortcut need. The main loop
itself runs on `cache`, the bounds of whichever problem the current
stage solves ([`Phase`]'s own docs): the slope problem first, then the
intercept problem, swapped at the stage A -> B handoff.

#### 本体: 段階 A の省略(S15)

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

#### 本体: d の増分維持と fresh_d_buf

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

#### 本体: 作業バッファ群

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

#### 本体: PRICE を非基底列だけにする分割(price_nonbasic_only)

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

#### 本体: PRICE 専用の行優先コピー(S15)

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

#### 本体: FTRAN 結果密度の移動平均(§2.7)

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

#### 本体: noise_feasible

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

#### 本体: 最初のピボットから厳密 DSE を使う理由

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

#### 本体: 「最大改善」chuzr エスカレーション(実験的)

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

#### 本体: 実行不能行数プラトー検出の上限

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

#### 本体: ドリフト許容誤差関連の状態

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

#### 本体: d のドリフト検査の粗い周期

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

#### 本体: 診断フラグの読み込み

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

#### 本体: delta0 診断・Devex 切り替え(不採用)・DSE 再計算(既定オフ)

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

#### 本体: Score2 の適応的許容誤差(実験的)

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

#### 本体: best_remaining_m_side(プラトー検出の補助指標)

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

#### 本体: stuck_row ブースト(実験的・効果なし)

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

#### 本体: 破棄された候補の禁止(常時オン)

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

#### 本体: chuzr 候補短縮リスト S11(実験的)

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

#### 本体ループ: chuzr(DSE 重み付き離基行選択)と Score2 許容誤差の補間

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

#### 本体ループ: 行方向 PRICE(基底列を含める理由の歴史)

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

#### 本体ループ: chuzc1 の分岐なし候補フィルタ

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

#### 本体ループ: 停止候補による刈り込み

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

#### 本体ループ: Eligible 空での noise_feasible 再挑戦

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

#### 本体ループ: 更新済み分解からは Infeasible を結論しない

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

#### 本体ループ: chuzc1 のヒープ化(遅延ソート)

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

#### 本体ループ: bland_mode でも (ratio, j) 順に並べる理由

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

#### 本体ループ: BFRT 使い切り時の Infeasible ガード

(元の位置: L3913 付近)

Same guard as the `Eligible = empty` site above, for the
same reason (see its own docs): an `Infeasible` conclusion
drawn at `1e-9` from a factorization this loop lets drift to
`1e-4` is not a conclusion. This is the site `greenbea`
actually reached (`site=bfrt_exhausted`, iteration 4715).

#### 本体ループ: Harris 型パス 2

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

#### 本体ループ: BFRT 結合フリップ

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

#### 本体ループ: 入る列の FTRAN

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

#### 本体ループ: updateVerify と pivot_grossly_inconsistent

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

#### 本体ループ: M 側解消の記録

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

#### 本体ループ: 入る列による x_B(M) の増分更新

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

#### 本体ループ: 停滞検出

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

#### 本体ループ: 実行不能行数のプラトー検出

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

#### 本体ループ: remaining_m_side の進展もプラトー判定に数える

(元の位置: L4844 付近)

`remaining_m_side` progress also counts (see its own
`best_remaining_m_side` docs above) — a solve can keep steadily
resolving M-side columns while the infeasible-row *count*'s
minimum sits unbeaten simply because that set is churning
(Netlib `dfl001`), and treating that as a stall latches
`bland_mode` on a solve that was never stuck.

#### 本体ループ: PRICE 分割の入れ替え(S12)

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

#### 本体ループ: Forrest-Tomlin 更新と再分解トリガ

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

#### 本体ループ: delta = 0 での傾きチャネル残差の省略

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

#### 本体ループ: d のドリフト検査

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

#### 本体ループ: ENOMOTO_D_DRIFT_REFACTOR_ONLY

(元の位置: L5186 付近)

`ENOMOTO_D_DRIFT_REFACTOR_ONLY=1` (S18, A/B, default off)
drops this periodic check altogether and leaves `d`'s
resync to the `fresh_d_into` every refactorization already
does — measured to fire 0 times on all 93 Netlib problems
(`analysis/simplex_loop_20260924_113533.md` §4 S18), so
this only saves its BTRAN + `O(nnz(A))` every
`RESIDUAL_CHECK_MULTIPLIER` checks.

#### 関数 finish: lu を引き継ぐ理由

(元の位置: L5291 付近)

`lu` is the main loop's own last-iteration factorization (already
exact for the current basis — the main loop's own termination check
just used it), passed in rather than rebuilt here: this function
runs once per solve, so the saving is small in isolation, but there
is no reason to pay for a `refactorize` this basis already has.

#### 関数 finish: cleanup 補題

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

#### 関数 finish: 旧 cleanup の超疎 FTRAN

(元の位置: L5478 付近)

Hyper-sparse FTRAN (`solve_sparse_into`/`GpScratch`, Gilbert-
Peierls): `j`'s own column is genuinely sparse, unlike `x_B`'s
own `rhs_base`/`rhs_slope` (summed contributions from every
nonbasic column, generally *not* sparse) — this is the one
place in this module a hyper-sparse solve has a natural
application without also committing to full incremental `x_B`
maintenance (see this module's own docs).

#### 関数 finish: 旧 cleanup の B^-1 A_j = 0 分岐

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

#### 関数 finish: 旧 cleanup の自由変数追い出し

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

#### 関数 polish_with_true_bounds

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

#### 関数 polish_with_true_bounds: chuzr(DSE を試して不採用)

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

#### 関数 polish_with_true_bounds: 真のコストでの双対実行可能性チェック

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

#### 関数 polish_with_true_bounds: 固定列を双対チェックから除外

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

#### 関数 polish_with_true_bounds: 固定列だけの不整合時の早期返却

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

#### 関数 polish_with_true_bounds: S5 ハンドオフフリップ

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

#### 関数 polish_with_true_bounds: 主単体法への引き継ぎ

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

#### 関数 polish_with_true_bounds: 引き継ぎ時の特異基底

(元の位置: L5907 付近)

A singular basis here propagates `None` (reported as
`NotSolved`): there is no from-scratch solver to restart
with, since `Tableau::new` assumes every structural column
has a finite bound, which this module exists specifically to
handle when false.

#### 関数 polish_with_true_bounds: noise_feasible の移植

(元の位置: L6002 付近)

`super::solve_lp_dual_on`'s own `noise_feasible` second
chance, ported here now that this function no longer just
re-derives a fresh (and possibly slightly different) `x_B`
next iteration regardless: before concluding genuine
infeasibility, check whether `r`'s own deviation is actually
within this row's rounding noise (scaled by its own RHS
magnitude — `PRIMAL_FEAS_TOL`'s own docs).

#### 関数 polish_with_true_bounds: bland_mode の並び順

(元の位置: L6016 付近)

Same order regardless of `bland_mode`: ascending `(ratio, j)`.
`bland_mode` sorting by `j` alone (an earlier version of this
branch) broke the ratio test the BFRT walk just below relies on
-- see the main M-tracked loop's own identical fix above for the
Netlib `greenbea` failure this caused there. `j` already breaks
ties deterministically in the one order, which is all Bland's
rule actually needs.

#### 関数 polish_with_true_bounds: 停滞検出

(元の位置: L6191 付近)

Stall detection — now the classical method's own *exact* check
(`(theta_q * dj_q).abs() < STALL_PROGRESS_EPS`), not the
`dj_q`-alone proxy the previous, non-incremental version of this
function needed for lack of a real `theta_q`: the incremental
`x_B` step just above now computes a real one.

#### テスト補助 std_form

(元の位置: L6319 付近)

Builds a `StdForm` directly from a dense list of sparse rows (each
row *already* including its own slack term), bypassing
presolve/`build_std_form_presolved` entirely — these tests exercise
[`solve_lp_dual_extended`] in isolation, independent of whatever
presolve does or doesn't eliminate for a given problem shape (see
`simplex.rs`'s own end-to-end `freevar_*`/`had_unbounded_structural_*`
tests for the presolve-integrated path).

#### テスト two_free_columns_tied_only_through_opposing_inequality_rows_park_one_at_zero

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

#### テスト bfrt_flips_multiple_bounded_candidates_before_the_real_unbounded_pivot

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

### src/params.rs(`mod extended_dual`)

#### 定数 XB_CHECK_INTERVAL

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

#### 定数 XB_DRIFT_TOL

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

#### 定数 XB_DRIFT_ESCALATION_STEP

Every this many drift-triggered refactorizations *within the same
solve*, [`XB_DRIFT_TOL`]'s own effective bound multiplies by
[`XB_DRIFT_ESCALATION_FACTOR`] (capped at [`XB_DRIFT_TOL_MAX`]) — see
that constant's own docs for why this is a per-solve escalation rather
than a static per-problem scale. `10` keeps every instance measured at
single-digit drift-refactor counts (the ones prior attempts broke)
entirely below the first step, while still letting a genuinely
pathological solve (hundreds of triggers at the flat bound) climb
through several steps before this cap's own `1e-4` ceiling.

#### 定数 XB_DRIFT_ESCALATION_FACTOR

Multiplier applied per [`XB_DRIFT_ESCALATION_STEP`] drift triggers.
`10` mirrors the *single* absolute-loosening step attempt 2 (this
constant's own docs) already measured in isolation (`1e-8` -> `1e-7`
fixed `maros`, broke `fit1p`) — the escalation ladder repeats that same,
already-characterized step size rather than inventing a new one, but
only after `XB_DRIFT_ESCALATION_STEP` proves the *current* solve is
actually the kind that benefits from it.

#### 定数 XB_DRIFT_TOL_MAX

Ceiling on the escalated [`XB_DRIFT_TOL`] — reuses `super::FT_RESIDUAL_TOL`'s
already-proven-safe order of magnitude (the classical method's own
absolute drift bound, `1e-4`) rather than letting escalation grow
unbounded into territory no measurement has ever validated.

#### 定数 PIVOT_ESCALATION_STEP

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

#### 定数 FT_MAX_UPDATES_FACTOR

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

#### 定数 FT_MAX_UPDATES_FLOOR

Floor for [`ft_max_updates`] — never weaker than the classical method's
own already-proven-safe flat cap, regardless of how small `m` is.

#### 定数 SYNTH_CLOCK_FACTOR

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

#### 定数 SYNTH_CLOCK_MIN_UPDATES

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

#### 定数 D_DRIFT_TOL

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

#### 定数 D_GROSS_MISMATCH_REL_TOL

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

#### 定数 X_B_SLOPE_NOISE

`x_B(M)`'s `M`-coefficients are exactly zero or of order one in exact
arithmetic; anything this small is accumulated LU/update noise. Left in,
`Affine1::cmp_lex`'s slope-first order lets it override a comfortably
feasible `base` and drives two-variable cycles (Netlib `greenbea`).

#### 定数 SLOPE_TOL

Absolute tolerance on an `M`-coefficient: stage A treats a slope
deviation as positive only above it, and the stage A -> B handoff counts
a basic `x^1_j` as sitting *on* its slope bound `l^1_j`/`u^1_j` (so that
bound survives into `l^B`/`u^B`) within it. Matches
[`Affine1::gt_zero`]'s own slope threshold, so the stage split draws the
line exactly where the lexicographic comparison it replaces did.

#### 定数 Z_SLOPE_TOL

How negative `z^1` (the slope of the optimal value `z(M) = z^0 + z^1 M`)
must be to count as `z^1 < 0` (`prop:trichotomy`). A small absolute
tolerance rather than `TOL`: `z^1` is a sum of (possibly many)
reduced-cost terms, so its floor scales with the problem's own cost
magnitudes, not with `TOL`'s coefficient-level tightness — matches this
crate's own precedent of using a looser, separate tolerance for
accumulated-magnitude checks (see `simplex.rs::PRIMAL_FEAS_TOL`'s own
docs for the same reasoning). Shared by stage A's early exit and
[`finish`]'s own unboundedness test so the two can never disagree.

## 疎 LU 分解 (src/simplex/lu.rs)
コード中にあった開発経緯・測定値・撤回した試みなどのコメントを移したもの。本文は元の英語のまま (重複のみ軽く整理)。

### src/simplex/lu.rs

#### モジュール冒頭 (//!)

- Degree-list implementation history: row/column degrees live in bucket arrays with O(1) bucket moves via a parallel position index (swap-to-last-then-pop on removal — the same pattern `factorize`'s earlier `active_rows` bookkeeping used). The column-major mirror is what makes the whole scheme actually sub-`O(m)` per step rather than just relocating the same cost: "which rows does eliminating column `pj` affect" is answered by that column's own live-row list directly (cost = that column's own current degree) instead of scanning every active row to test whether it still holds `pj`, and "how many rows still touch column `j`" is that list's length (O(1)) instead of a fresh full-matrix scan. Both the submatrix and its mirror are flat arrays ([`KernelMatrix`], HiGHS's own `HFactor` `mc_*`/`mr_*` layout), not `BTreeMap`/`BTreeSet` containers.
- **Bug history**: An earlier version of this file computed row/column degrees this way but then performed the actual elimination arithmetic *directly* in `factorize`'s main loop, ahead of a separate `eliminate_column` method that was supposed to update the bucket state — since that method detected "which rows changed" via `contains_key(&pj)`, and the earlier direct arithmetic had already removed `pj` from every affected row first, `eliminate_column` always found nothing to do. Bucket degrees then stayed frozen at their *initial* values for the rest of the factorization while the underlying matrix kept changing underneath them, which didn't corrupt the arithmetic (pivot values are always read fresh from `rows`) but could starve `find_best_pivot` of a candidate it should have found, surfacing as a spurious "singular" `None` on matrices that are not actually singular (confirmed: this crate's own HiGHS cross-check benchmark, which the prior, non-bucketed `factorize` solved without issue, started panicking at `n=2000` with exactly that message). The fix: elimination is a single method (`eliminate`) that does the arithmetic *and* the degree/bucket bookkeeping together, and a row being retired as a pivot removes it from every other column's live-row list too (not just its own pivot column's), so no column's degree can drift stale by continuing to count an inactive row.
- Per-pivot cost is `O(pivot column's degree)` for the elimination itself plus `O(fill touched)` for the resulting degree/`col_max_abs` refresh — though `find_best_pivot`'s bucket scan can still fall back to examining more candidates on a poorly-conditioned or unusually dense step; not a hard worst-case guarantee, just a much smaller constant than rescanning the whole active submatrix every step.
- Only FTRAN's entering-column solve gets the full Gilbert-Peierls sparse treatment — see `LuFactors::l_solve_sparse_into`'s history for why.

#### thread_local PIVOT_THRESHOLD

Thread-local rather than a field threaded through `factorize`'s half-dozen entry points (and their callers in `simplex.rs`, `extended_dual.rs`, `mip.rs`) because it is a *solve*-scoped setting in exactly the way HiGHS's own `info_.factor_pivot_threshold` is: one value, read once per factorization, written only by the simplex loop that owns the solve. Thread-local (not a `static`) keeps concurrent solves — `mip.rs` runs LP relaxations on rayon workers — from escalating each other's thresholds, which a shared global would do while also making both solves' pivot sequences depend on the interleaving. Every solve entry point calls `reset_pivot_threshold` before its first factorization, so a thread that ran a troublesome solve does not hand the escalated value to the next solve scheduled onto it.

#### 関数 pivot_threshold_base

`ENOMOTO_PIVOT_THRESHOLD` is how the A/B behind `STABILITY`'s own value is produced without a rebuild. Read from the environment once per process, not once per solve: a solve that re-read it would pay a `std::env::var` lookup inside the very loop this section is trying to speed up.

#### 関数 pivot_threshold

Read once per factorization, never per elimination step — a factorization that read it per step could see it change underneath itself only if the simplex loop ran concurrently with its own factorization, but reading it once also keeps the whole factorization's pivot sequence a function of one scalar, which is what makes a given solve reproducible.

#### 関数 escalate_pivot_threshold

This is `docs/lu_comparison_enomoto_vs_highs.md` §2.4's "loosen/tighten the stability floor when the problem is ill-conditioned", in HiGHS's own direction: a solve that keeps *failing* numerically (Forrest-Tomlin updates rejected, `x_B(M)`/`d` drifting away from the true basis, pivots grossly inconsistent with the factorization) is one whose factorizations are too permissive, so the floor goes **up**, buying stability with fill-in. Lowering it on trouble would be the wrong sign: it is exactly the marginal pivots a lower floor admits that produce the eta chains these triggers are catching.

Monotone within a solve, like HiGHS's `info_.factor_pivot_threshold`: nothing lowers it again short of `reset_pivot_threshold`. A ratchet that also relaxed would make "how many troublesome iterations ago" part of the pivot sequence, and the extra state buys nothing measurable — the ladder is one step wide.

**Nothing calls this by default.** Wiring it to the numerical-failure triggers cost +7.4% over NETLIB93; see `PIVOT_ESCALATION_STEP` and `analysis/pivot_threshold_colfixmax_20260922_154500.md` for the measurement, and `ENOMOTO_PIVOT_ESCALATION_STEP` to re-enable it.

#### 関数 pivot_search_limit

`0` restores the unbounded scan, which is how the A/B behind the constant's own value is produced. Read once per `MarkowitzState::new` (i.e. once per factorization), never per elimination step: the read is `find_best_pivot`'s own caller-side cost otherwise, paid `m` times per factorization, and an `std::env::var` lookup there would show up in the very measurement this gate exists to make.

#### 静的変数 PROF_TOTAL_STEPS / PROF_TRIVIAL_STEPS

Measurement counters for the "should `factorize` triangularize `A_B` into a trivial part plus a smaller Markowitz bump before factoring, the way production codes like HiGHS do" question, read back by `simplex.rs`'s `ENOMOTO_PROF_TRIANGULAR`-gated diagnostic. Answered by measurement rather than by adding the pre-pass speculatively: on every Netlib instance checked (`ganges`, `ship12s`, `stocfor2`, `fit1p`), `find_best_pivot`'s bucket-based early exit already resolves 90-100% of pivots as score-0 "trivial" ones, and cumulative time inside `find_best_pivot` across the *entire* solve was under 0.2% of total solve time in every case — the pivot *search* was never the bottleneck a dedicated triangularization pre-pass would speed up, so one was not added. Kept as a live diagnostic (not deleted) in case a future problem shape changes that picture — and it did: the "under 0.2%" figure above holds only for the four small instances it was measured on. Re-measured across the whole set for `PIVOT_SEARCH_LIMIT`, `dfl001` spent 2.96s of its 22.0s solve (13%) inside `find_best_pivot`, at an average scan width of 261 candidate columns per elimination step — the search *was* a real cost there, just not on problems small enough for the original sample. A triangularization pre-pass still isn't what that calls for (the bound in `PIVOT_SEARCH_LIMIT` addresses it directly, taking the same problem's scan to 0.22s), but the 0.2% claim should not be quoted as if it covered the large instances.

#### 静的変数 PROF_BUCKET_SCAN_NS

The step/candidate counters around it are always kept, accumulated per factorization and flushed once from `MarkowitzState`'s `Drop`.

#### 静的変数 PROF_DENSE_FALLBACK_STEPS

A direct measurement of how often the dense-column-avoidance heuristic in `factorize` actually gets exercised (as opposed to every dense column simply never coming up as a candidate at all, in which case this stays at `0` and the heuristic is a no-op for that problem).

#### 静的変数 PROF_SEARCH_LIMIT_STEPS / PROF_SEARCH_CANDIDATES

`PROF_SEARCH_LIMIT_STEPS` counts early returns because of `PIVOT_SEARCH_LIMIT` (as opposed to the score-0 exit, the per-degree-level `merit_limit` exit, or a full scan) — the direct measurement of how often the bound is exercised at all, without which a flat benchmark result can't be told apart from a no-op. `PROF_SEARCH_CANDIDATES` is the quantity `PIVOT_SEARCH_LIMIT` bounds per call; read against `PROF_TOTAL_STEPS` it gives the average scan width per step, which is what the bound is supposed to move.

#### 静的変数 PROF_PIVOT_ESCALATIONS

Without this, a flat benchmark on the §2.4 escalation can't be told apart from one where the ladder never fired at all.

#### 静的変数 PROF_COLMAX_RESCAN_ENTRIES

Entries `ensure_col_max_abs` walked to un-stale the columns `find_best_pivot` actually read — the total work an incremental `colFixMax` (`docs/lu_comparison_enomoto_vs_highs.md` §2.4) could have removed, and the reason removing it lost: since §2.5's `PIVOT_SEARCH_LIMIT` bounds a single search to 8 candidate columns, this is already a small fraction of the per-entry bookkeeping such a scheme costs in `eliminate` (measured in `analysis/pivot_threshold_colfixmax_20260922_154500.md` §2). Counted per rescan, not per touched column, so it stays off the elimination loop's own path.

Since `find_best_pivot` recomputes a stale max only when some entry of the column has already passed the Markowitz-score filter (see the lazy-`col_max_abs` note there; `ENOMOTO_LU_LAZY_COLMAX=0` restores the eager rescan), this counts only the rescans that were actually needed: `pilot87` 14.2M -> 9.2M entries, `d2q06c` 258K -> 223K, same pivots.

#### 構造体 KernelMatrix

Flat, HiGHS-`HFactor`-style storage for the active submatrix that Markowitz elimination works on — the replacement for the `Vec<BTreeMap<usize, f64>>` (rows) + `Vec<BTreeSet<usize>>` (column mirror) pair `MarkowitzState` used to hold directly, and the item `docs/lu_comparison_enomoto_vs_highs.md` §3.1 flagged as the largest remaining structural gap against HiGHS's own kernel (`mc_*`/`mr_*` flat arrays with in-place insert/delete, against tree nodes scattered across the heap and an `O(log d)` traversal per element touched).

Layout mirrors HiGHS's `mc_start`/`mc_space`/`mc_count` + `mr_start`/`mr_space`/`mr_count` pattern with the two axes swapped — this crate's elimination is row-oriented (it scatters the pivot *row* into every affected row), where HiGHS's is column-oriented, so the *values* live row-major here and the index-only mirror is the column one. Column mirror indices are `u32`: two rows per cache line's worth of what `usize` would cost, and the ascending-degree bucket scan in `find_best_pivot` reads these runs end to end. The mirror only answers "which rows are live in column `j`", exactly as `col_rows` did.

Both runs are kept *sorted*, rather than taking HiGHS's cheaper swap-with-last unordered sets. That is a deliberate extra cost — a `copy_within` over a contiguous, usually single-digit-length run, still far cheaper than the tree traversal it replaces — and it buys exact behavioural equivalence with the ordered containers it replaces: `find_best_pivot` resolves Markowitz-score *and* pivot-magnitude ties by first-encountered, `eliminate` emits its `L` multipliers and refreshes touched columns in iteration order, and LP basis matrices are full of exactly-tied `±1` coefficients. An unordered mirror would therefore silently select different pivots on real Netlib instances, changing the factorization, the iteration counts, and hence what a before/after benchmark of *this* change is actually measuring.

- Field `row_idx`/`row_val`: Row runs, structure-of-arrays. Splitting the `(usize, f64)` pairs this held before means a column lookup (`row_get`, run ~12M times per `pilot87` solve by `find_best_pivot`'s lazy `col_max_abs` rescans) and the index side of `eliminate`'s scatter loop stream 4 bytes per entry instead of 16.

#### 関数 KernelMatrix::new

- Capacity, not length: the reserve (`2 * total + 64`, now `KERNEL_RESERVE_MULT`/`KERNEL_RESERVE_EXTRA`) is here so the relocations (`ensure_row_cap`'s, later) stay `resize` inside one allocation instead of repeatedly reallocating and copying the whole buffer. Nothing is *initialized* beyond what is actually written.
- Stable sort by column, then accumulate each duplicate run in input order and drop exact zeros — bit-for-bit what the `*entry(j).or_insert(0.0) += v` + `retain(|_, v| *v != 0.0)` construction this replaces produced, summation order of repeated coordinates included.
- No up-front slack (`cap == len`): a row only ever needs to grow when the merge in `eliminate` leaves it *net* longer, and the pivot column's own entry always leaves at the same time, so one fill-in still fits in place and only two or more relocate. Pre-padding every row instead cost a memset proportional to the padding on *every* refactorization, including for the many rows that never take fill at all — worst of all on a near-slack basis, where `nnz ~= m` makes a flat few-entries-per-row pad several times the size of the real data. `ensure_row_cap` doubles from here, so a row that keeps taking fill still relocates `O(log)` times, not once per insertion.

#### 関数 KernelMatrix::row_get / col_insert / col_remove

- `row_get` is the direct stand-in for `rows[i].get(&j)`.
- `col_insert`: `eliminate` walks its affected rows in ascending order, so the insertion point is typically at or near the run's tail and the shift is short.
- `col_remove` is a no-op when absent, matching `BTreeSet::remove`'s own tolerance.

#### 構造体 ElimScratch

Instead of allocating a fresh `Vec`/`BTreeSet` per step.

#### 構造体 MarkowitzState

- Field `col_max_abs_dirty` history: Most columns `eliminate` dirties get dirtied again by a later elimination step before `find_best_pivot` ever visits them (a column's bucket position, which *is* updated eagerly by `update_col_degree`, is what determines when that happens), so eagerly recomputing every dirtied column's max was mostly wasted work — up to 75% of it, measured on Netlib `greenbea`.
  Marking, rather than maintaining, is also what `docs/lu_comparison_enomoto_vs_highs.md` §2.4's incremental `colFixMax` was measured against and beat. That variant kept `col_max_abs[j]` exact through `eliminate` — raise it on a fill-in or a growing value, mark stale only when the entry that *was* the max shrank or left — which dropped the stale fraction to 0.1-6% of touched columns and left every pivot choice bit-identical. It still lost, by 2.8% over NETLIB93: since §2.5's `PIVOT_SEARCH_LIMIT` bounds one search to 8 candidate columns, the rescans it removed were already small (`pilot87`: 1.9M entries) against the per-entry bookkeeping it added in the merge loop (61.5M updates, each a scattered read-modify-write into an `m`-sized array). See `analysis/pivot_threshold_colfixmax_20260922_154500.md` §2.
- Field `initially_dense`: fixed at construction time and never updated, deliberately: a truly dense column's *current* degree keeps shrinking as unrelated rows get eliminated as pivots for *other*, sparser columns (each such row leaving the basis removes it from every column's live list, including this one's) — dropping into a low bucket only because its rows happened to get cannibalized elsewhere, not because it stopped being structurally dense. Thresholding on the live, shrinking degree would let `find_best_pivot`'s ordinary ascending-bucket scan pick such a column early anyway, right when it looks artificially sparse — exactly the case this field exists to still catch. A pivot on a column with `d` remaining active rows scatters the entire pivot row's pattern into all `d` of them in one step (`eliminate`'s `affected` list), so pivoting on a column that is dense *by original structure* — even at a reduced current degree — is still the single most expensive kind of step Markowitz pivoting can take.
- Field `search_limit`: `0` means "unbounded", the pre-§2.5 behaviour.
- Field `threshold`: this factorization's whole pivot sequence is deliberately a function of one scalar captured at its start, rather than of a value the simplex loop could raise part-way through.
- Field `prof_colmax_rescan_entries`: an `AtomicUsize::fetch_add` per rescan would be a locked read-modify-write on the factorization's own path, and would make two threads factorizing at once contend on one cache line.
- Field `row_singleton_rel` (`ENOMOTO_LU_ROW_SINGLETON`): Measured motive: numerically rejected row singletons (they fail the `0.25` threshold) persist across many steps and every search keeps paying for the columns in front of them. `rel = 0` is HiGHS's own rule (no threshold on singletons). A row singleton's pivot row has no other active entry, so it causes no fill and no update of the active submatrix; its only numerical cost is the size of the `L` multipliers `a_kj / v`. Never applied in `factorize_bordered`'s sparse phase (reset right after `MarkowitzState::new` there): there the row's border entries *are* updated, through the Schur complement, by exactly those multipliers — measured as a wrong `fit2p` objective (`-8.9e117`) when it was.
- Field `row_search` (`ENOMOTO_PIVOT_ROW_SEARCH`, B2(b)): Short rows find small Markowitz scores early, so the per-level exit `best_score <= c^2` fires sooner on matrices whose columns are short but whose rows are long (`dfl001`). A long row costs a `col_max_abs` rescan for most of its columns, which on a matrix with a dense tail (`pilot87`) outweighs what the search saves.

#### thread_local BUCKET_POOL

So a refactorization does not re-allocate `2(m+1)` bucket headers and the buckets' own buffers every time.

#### 関数 MarkowitzState::refresh_column / ensure_col_max_abs

Marks `col_max_abs[j]` stale rather than recomputing it here; see `col_max_abs_dirty`'s history for why, including why the incremental alternative (`docs/lu_comparison_enomoto_vs_highs.md` §2.4's `colFixMax`) was measured and rejected. `ensure_col_max_abs` is called from `find_best_pivot` right before it reads `col_max_abs[j]`, the one place that value's currency actually matters.

#### 関数 MarkowitzState::find_best_pivot

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

#### 関数 MarkowitzState::eliminate

- Splitting arithmetic and bookkeeping into separate steps (as an earlier version of this file did) is unsound: bookkeeping keyed off "did this row still contain `pj`" only works if it runs *before* `pj` is actually removed.
- Each affected row is rewritten by a **single sorted merge** of its own run against `pivot_row_snapshot` (both ascending by column), rather than by one keyed lookup per pivot-row entry: with the `BTreeMap` rows this replaced, this inner loop — the hottest in the whole factorization, run once per `(affected row, pivot-row entry)` pair, every elimination step — cost `O(d_p log d_i)` tree descents into scattered heap nodes; the merge costs `O(d_i + d_p)` over two contiguous, sequentially-read runs and one sequentially-written one. That is `docs/lu_comparison_enomoto_vs_highs.md` §3.1's point (HiGHS's `mc_*`/`mr_*` flat arrays against this crate's tree nodes) applied to the one loop where it matters most.
- The common path works HiGHS-style off a dense scatter of the pivot row (`ElimScratch::wval`) instead of a two-pointer merge: with no data-dependent branch per entry — the merge mispredicted on nearly every interleaving of the two patterns. Values, row order and every degree/bucket update are exactly the merge's, so the factorization is bit-identical.
- No per-entry column "touched" bookkeeping: every column whose degree or values this step can change is a column of the pivot row (an update or a fill-in lands only there), and the pivot row is retired from all of those columns anyway, so refreshing the pivot row's columns in ascending order is exactly the sorted, deduplicated touched set the per-entry stamping used to build.

#### 構造体 GpScratch

One instance lives for as long as its caller's own dedicated sparse-solve buffer does (`solve_lp_dual_on` creates one, alongside a `z` buffer used *only* for this path — never shared with a plain `FtLu::solve_into` call's own `scratch`, per `FtLu::solve_sparse_into`'s own docs on why that separation matters — before the pivot loop starts, and reuses both every FTRAN).

`visited` history: It was a hand-rolled `Vec<u32>` plus a bare `epoch += 1` until that trick was consolidated into `crate::sparse`; the bare increment had no wraparound guard, so after 2^32 calls a stale stamp would have read as a live mark and silently truncated a reach set — i.e. produced a wrong FTRAN. `EpochMarks::begin` handles it. `stack`: iterative, not recursive — this crate's basis matrices can have `m` in the low thousands, deep enough that a recursive DFS risks a real stack overflow on a long dependency chain.

#### 構造体 LuFactors

- Field `l_col`: Stored as a `crate::sparse::CscMat` — one flat `(index, value)` buffer plus offsets, the same layout `simplex.rs`'s `StdForm` uses for the frozen coefficient matrix, on the same reasoning: `L` never changes once a refactorization builds it, and it is then read on every FTRAN/BTRAN's `L`-stage for the rest of that basis's life.
  **This is the second attempt, and the first one that measured as a win.** The first flattened a finished `Vec<Vec<(usize, f64)>>` into the compressed form as a post-pass, and a controlled A/B showed a consistent small regression on every instance that moved at all (`scsd8` +2.1%, `25fv47` +1.8%, `stocfor2` +4.0%, `fit1p` +3.2%, `degen3`/`pilotnov` flat, nothing faster). Its own post-mortem identified the reason and named the fix: the post-pass *keeps* building the `m` small per-column `Vec`s it was meant to remove and then adds an `O(nnz)` copy on top, so it paid the compressed form's cost — two offset reads per column access, against `Vec<Vec>`'s single pointer hop to an already-known `(ptr, len)` — while buying none of its benefit.
  `crate::sparse::CscBuilder` is that fix. Every one of this file's factorizations already emits `L`'s columns in ascending step order (left-looking elimination produces column `s` complete at step `s`), so the flat buffer can be appended to directly, with the column boundary recorded wherever the buffer has reached: no counting pass, no per-column `Vec`, and no copy. What is left is a strict improvement at build time (two allocations for the whole of `L` instead of `m + 1`) plus contiguous entries for `l_solve_into`'s own sequential `for s in 0..m` sweep to prefetch through.
- Field `u_row`: flattening it would cost the same construction work for no repeated-read benefit.
- Field `l_row`: This is HiGHS's own `lr_start/lr_index/lr_value` (`HFactor.h`, built by `buildFinish()` right beside the column-major `l_start/l_index/l_value`), and it exists for exactly the reason HiGHS builds it: `L^{-T}` (BTRAN's tail) is a *gather* when read through the column-major `l_col` — step `s` reads one `w[row_step]` per `l_col[s]` entry, so no single value's zero-ness makes the step skippable (Hall & McKinnon 2000 §4.4's own observation, which `l_transpose_solve_into`'s pre-`l_row` form was stuck with) — but the very same triangular solve becomes a *scatter* when read through this mirror: step `s` multiplies the single value `w[s]` into every `l_row.row(s)` entry, so `w[s] == 0.0` makes the whole step a provable no-op, exactly the skip `l_solve_into`/`u_solve_into` already have in the forward direction. Flat (`CsrMat`: two allocations, offsets + entries), built directly by `CscMat::to_csr`'s counting sort (one pass to size each row's slice, one to fill it), with no intermediate `Vec<Vec<...>>` on either side.

#### 関数 factorize_dense_faer

See `DENSE_INPUT_FRACTION`'s history for why this exists instead of running Markowitz on an already-dense matrix.

#### 関数 debug_print_block_sizes

Throwaway diagnostic, not wired into any production path: measures what block-size distribution a Dulmage-Mendelsohn SCC decomposition of *this* refactorization's basis matrix would actually have, to check a specific hypothesis about the previously-reverted block-triangularized `factorize` (see `factorize`'s history) — namely, whether the blocks it would find are mostly tiny (say <=10 or <=50 rows), which would matter for a proposal to special-case small blocks with a dense/product-form solve instead of the general sparse Forrest-Tomlin machinery.

#### 関数 detect_border_columns

The same "near-fully-dense trend/regression column" shape `MarkowitzState::initially_dense` already detects internally, exposed here as a free function so `factorize`'s routing decision can check it before paying for a `MarkowitzState` at all — this is the only extra cost non-bordered instances pay: one `O(nnz)` degree pass, measured (see `BORDER_MAX_FRACTION`'s history) to be cheap enough to run unconditionally rather than gated behind a flag.

#### 関数 factorize_diagonal

`simplex.rs`'s initial-basis call sites use this instead of paying for pivot selection, fill-in bookkeeping, and border/dense-input detection on a matrix that has nothing for any of that to do. Shape check first, with no allocation: every mid-solve refactorization tries this and almost always fails, and building `u_row` row by row until the first non-diagonal row paid one small allocation per leading diagonal row for nothing.

#### 静的変数 PROF_REBUILD_*

Read back by the `ENOMOTO_PROF_PHASES_EXT` diagnostic: a rejected attempt is pure overhead paid on top of the full Markowitz factorization that follows, so the accepted/attempted ratio is what decides whether the reuse pays for itself. `PROF_REBUILD_ROW_REPICKS` is bounded below by the number of basis columns the Forrest-Tomlin updates replaced since the order was recorded, and is the reason the row order cannot simply be replayed the way the column order can. `PROF_REBUILD_ACCEPTED_NNZ`/`PROF_FULL_NNZ`/`PROF_FULL_COUNT`: the diagnostic behind "is a reused order producing factors every later FTRAN/BTRAN then pays for".

#### 関数 reuse_fill_limit / reuse_pivot_order_enabled

`ENOMOTO_REUSE_FILL_LIMIT` lets the one number be re-tuned against the Netlib set without a rebuild — and, more to the point, flipped *within one process* for an A/B. `ENOMOTO_REUSE_PIVOT_ORDER=0` exists so an A/B of this feature can flip it *within one process* — per `analysis/ftran_density_gate_20260922_062832.md` §4.1, this box's per-problem run-to-run spread across separate processes reaches 4x, far wider than the effect being measured. Read once per refactorization (a handful of times per solve), never per iteration.

#### 関数 factorize_reusing

This is this crate's counterpart to HiGHS's `HFactor::build()` trying `rebuild()` (`util/HFactorRefactor.cpp`) before `buildSimple()` + `buildKernel()`, named as gap §2.2 in `docs/lu_comparison_enomoto_vs_highs.md`: a mid-solve refactorization factorizes a basis that differs from the last factorized one only by the columns the Forrest-Tomlin updates since then replaced, so the order Markowitz chose last time is usually still a good order — and *applying a known order* costs only the elimination's own arithmetic, with none of the search, degree bookkeeping, or active-submatrix maintenance (`MarkowitzState`'s `BTreeMap`/`BTreeSet` churn) that choosing one costs.

- Border columns/dense test computed once rather than recomputed by `wants_bordered` and again by `factorize` (each an `O(nnz)` pass).
- A dense input goes to `factorize_dense_faer` regardless of any pivot order, and the bordered path wants its own ordering — reuse targets the ordinary sparse Markowitz case, which is every mid-solve refactorization on a real Netlib basis.
- Backoff: A rejected attempt is wasted work on top of the full factorization that follows it, and rejections cluster: the basis that produced one (too much fill under the recorded column order, or a numerically spent order) is usually still producing them a few refactorizations later. So back off exponentially — 2, 4, 8, ... refactorizations left alone, capped at `REUSE_MAX_BACKOFF` — and reset to zero on the first acceptance, which is what keeps a problem where reuse *does* work paying nothing for this.

#### 関数 wants_bordered

Reuse must not take a `fit1p`/`fit2p`-shaped basis: the bordered path's whole point is to keep ~20-25 near-dense "trend" columns *out* of the sparse elimination entirely, and a plain left-looking pass over the order it produced scatters exactly those columns back through every step — measured as `fit2p` +6.4% at a 1.1 fill limit and +16.3% at 1.25, against roughly flat everywhere else, which is what sent this gate in.

#### 関数 factorize_reusing_order

- Left-looking: Nothing here searches for *sparsity*, and nothing maintains an active submatrix; per-step cost is the elimination arithmetic plus the reach-set heap, both bounded by the factors' own nonzero count.
- **Why only the column order is replayed, not the row order.** HiGHS's own `rebuild()` replays both (`refactor_info_.pivot_row` / `pivot_var`) and gives up — rank deficiency, full rebuild — the moment a recorded pivot row's entry is too small. That is affordable *there* because HiGHS only ever sets `refactor_info_.use` for a hot start (`HEkk::setNlaRefactorInfo`), i.e. when re-factorizing the very basis the order was recorded from, where the recorded rows trivially still work. Replaying both orders across a *changed* basis was implemented here first and measured: it is rejected essentially always (Netlib `25fv47` 0/26 attempts, `degen3` 0/7, `pilot` 0/30, `fit2p` 0/32, `greenbea` 1/28), and the rejections are overwhelmingly "the recorded pivot row is numerically *empty*" (`fail zero`, not `fail stability`) — which is exactly what a replaced basis column looks like: the entering column has no reason whatsoever to be nonzero at the row that was pivotal for the column that left. The column order is what Markowitz's fill-minimization actually encodes; the row assignment is a numerical choice, and re-making it per step (partial pivoting: take the largest remaining entry) costs one pass over the column that has already been computed.
- Threshold read once: this path reuses the previous factorization's *column* order but still picks each pivot row under the same threshold test, so an escalated threshold has to reach it too (a rebuild that kept the old, looser floor would quietly undo the escalation for as long as the pivot order keeps being reusable).
- Column-major copy: the same shape `CscMat::from_rows` builds, kept local because this one is indexed by *basis slot* and thrown away when the factorization is done.
- `row_remaining`: Maintained in `O(nnz)` total by decrementing a column's rows as that column is consumed, and used only to break the tie among numerically acceptable rows when the recorded one is unusable: with the column order fixed, the row choice is all that is left to keep fill down, and taking the absolutely largest entry (plain partial pivoting) ignores sparsity entirely.
- Forward solve: ascending step order is a valid topological order — the same property `l_solve_sparse_into` relies on, reached here with a heap rather than a DFS because the graph is still being built.
- Row re-pick: The recorded row is gone (a basis column the Forrest-Tomlin updates replaced leaves its old pivot row numerically empty here). The fewest-entries rule is the surviving half of a Markowitz count once the column is fixed.

#### 関数 factorize (旧 doc コメント中の履歴: Dulmage-Mendelsohn 分解の試行 2 件)

(These two paragraphs were originally attached as doc comments to `params::lu::DENSE_INPUT_FRACTION` / `BORDER_MAX_FRACTION` after the constant move, but describe `factorize`.)

**A Dulmage-Mendelsohn block-triangularized variant of this function was implemented, thoroughly validated, and measured — then reverted**: rows were partitioned into strongly-connected blocks (via bipartite matching + Tarjan SCC, `crate::graph::dulmage_mendelsohn_blocks_topological`, which remains implemented and tested for a possible future, more targeted revisit) in topological order, each factorized independently, then reassembled via the block-LU identity `U_ij = L_i^{-1} A_ij` for "spillover" entries outside a block's own matched columns (`L` itself stays exactly block-diagonal). Implementation correctness was confirmed via unit tests (including one that caught a real bug: an initial version copied spillover entries unchanged, which is only valid when the emitting block's own `L` is trivial/identity — true for singleton blocks, which is why singleton-only spillover tests passed by coincidence before the fix) and zero objective mismatches across the full 73-problem Netlib benchmark.

**But it measured as a net ~4% aggregate regression** in a controlled back-to-back A/B (same machine, same run, only the feature toggled): dramatic wins on a few instances with genuine block-angular structure (`fit1p` -44%, `wood1p` -17%, `scsd8` -10%, `sierra`/`sctap3`/`scrs8` a few percent) were outweighed by a broad ~10-25% tax on most other medium/large instances (`grow15` +25%, `bnl1` +18%, `modszk1` +17%, `perold` +16%, `25fv47` +16%, `stocfor2` +14%, `pilotnov` +13%, `ganges` +11%) — paying bipartite-matching-plus-SCC cost on *every* refactorization, whether or not it finds anything worth exploiting. Two cheap pre-gating heuristics were tried to avoid paying that cost on instances unlikely to benefit, and both failed: (1) whether `presolve::redundancy`'s own equality-row block decomposition found structure — `ganges` decomposes beautifully there (1053 blocks, a 1% bump) yet was still a net loss here, since the *basis* matrix (all rows, reshuffled by every pivot) doesn't share the *equality system*'s (static, presolve-time-only) structure; (2) the *basis* matrix's own bump size at the first real refactorization — `stocfor2` and `ganges` again showed excellent bump ratios (0.2-1.3%, as good as or better than the actual winners) yet remained net losses, showing the fixed decomposition cost itself, not just a poor decomposition outcome, was the problem. This mirrors HiGHS's own architecture: `HFactor::buildSimple()` peels off trivial (degree-1/logical) pivots via a cheap `O(nnz)` sweep with no bipartite matching at all, leaving full Markowitz elimination (`buildKernel()`) for only the remaining kernel — this file's own bucket-based `find_best_pivot` already gets that same cheap benefit for free (confirmed earlier via `PROF_TOTAL_STEPS`/`PROF_TRIVIAL_STEPS` showing 90-100% of pivots already resolve trivially), so the *additional*, much more expensive structure genuine Dulmage-Mendelsohn decomposition can find beyond that cheap peeling isn't reliably worth its own cost. Fully reverted; see the project history around this doc comment's own commit for the full numbers if revisiting.

**A second, "peel trivial pivots then Dulmage-Mendelsohn-decompose only the remaining kernel" variant of block triangularization was also implemented, tested, and measured — then reverted.** This directly followed up the first attempt, on the hypothesis that peeling first (mirroring HiGHS's own `buildSimple()`/`buildKernel()` split) would fix that attempt's "pays matching+SCC cost on every refactorization regardless of payoff" problem by shrinking the kernel matching+SCC actually runs on. It did not: full 73-problem Netlib A/B showed a **net ~37% aggregate regression** — far worse than the first attempt's ~4%, and a regression on `fit1p` specifically (+80%), the exact instance this was meant to speed up. Root cause, confirmed by direct instrumentation: `fit1p`'s kernel (post-peel) is a single irreducible ~20-row SCC block every time, so the decomposition gate *always* rejects it and falls back to a from-scratch `factorize_flat_markowitz` call — meaning the (redundant) peel work is paid twice, for zero benefit, every refactorization. Worse, the underlying premise turned out wrong: `fit1p`'s real cost was never a large interleaved non-trivial block in the first place. `eliminate`'s cost is `O(col_rows[pj].len())` (the pivot *column*'s remaining active rows) times the pivot row's own snapshot size — a pivot with Markowitz score exactly `0` (row degree `1`, the "trivial" case `PROF_TRIVIAL_STEPS` counts) is only free when its *column*'s degree is also small; a degree-1 *row* whose sole entry sits in an otherwise-still-dense "hub" column is scored as trivial yet costs `O(hub column's current degree)` to eliminate (every other row sharing that column must be updated). `fit1p`'s basis apparently has exactly this shape — many row-degree-1 pivots landing on a handful of not-yet-thinned dense columns — which no SCC/block decomposition addresses, since those rows don't form a separable block with the hub column at all. Fully reverted (including the two dedicated unit tests that validated its spillover-reassembly correctness, which was never in question — the numerics were right, just not worth what they cost). See the project history around this comment's own commit for the full A/B numbers and the `ENOMOTO_DEBUG_BLOCK_TRIANGULAR` trace output that pinned down the root cause, if revisiting; `debug_print_block_sizes` (`ENOMOTO_DEBUG_BLOCK_SIZES`) and the `ENOMOTO_DEBUG_ELIMINATE_COST` timer remain as live diagnostics either attempt's numbers came from.

#### 関数 factorize_routed

Tried *before* checking `is_dense_input`, deliberately: the measured crossover (see `BORDER_MAX_FRACTION`'s history) sits around `k/m ~= 0.5`, well past `is_dense_input`'s own 25%-of-`m^2` overall-density gate — a border-heavy input can easily cross that overall gate on the border columns' own density alone while `k/m` is still comfortably under `BORDER_MAX_FRACTION`, and in exactly that range `factorize_bordered` beats `factorize_dense_faer` too (not just plain Markowitz), so gating this attempt on `!is_dense_input` would give up a real win.

#### 関数 factorize_flat_markowitz_routed

Prefer a non-dense pivot column whenever one exists: avoiding a dense pivot column matters far more than the score it happens to carry at the moment it's chosen (see `MarkowitzState::initially_dense`). `pivot_row_snapshot` is one buffer for the whole factorization, not a fresh `Vec` per elimination step. `l_entries` is already grouped by `pivot_step` in ascending order — the elimination loop emits step `s`'s whole `L` column before moving to step `s + 1` — so `L` can be appended straight into its final compressed buffer, with no counting pass and no intermediate per-column `Vec`s.

#### 関数 factorize_bordered

**Why this exists**: `fit1p`-shaped Netlib instances have ~20-25 columns nonzero in essentially every row (see `DENSE_COL_FRACTION`'s history). The ordinary Markowitz path already defers pivoting *on* these columns as long as possible (`MarkowitzState::initially_dense`), but every ordinary elimination step whose pivot row still carries one of these columns' entries scatters them into every row `eliminate` touches anyway — measured (`ENOMOTO_DEBUG_ELIMINATE_COST`) as the actual cost driver behind `fit1p`'s refactorizations (average row fill climbing from `1.0` at the initial all-slack basis to `~10` a few refactorizations later, each one costing several milliseconds despite `m` only being in the hundreds). Excluding these columns from the sparse phase's own bookkeeping entirely (rather than merely deprioritizing them as pivot targets) removes that scatter cost outright; the algebra it defers is applied once, in bulk, via the classic bordered-block-diagonal LU identity.

`L_DS` (the sparse phase's own elimination multipliers for *every* affected row, border rows included — free, already computed as a side effect of the ordinary elimination). `U_SD` forward solve is structurally identical to `LuFactors::l_solve_into`, just against the in-progress `L_SS` rather than a finished `LuFactors`. The Schur complement reuses the exact same dense path already used for a globally-dense input, just at the `k`-sized scale this bordering was meant to shrink the problem down to. Falling back is exactly as the dense-column-avoidance fallback in `factorize_flat_markowitz` already does for its own `skip_dense` retry; the stuck case is a border column genuinely required as a pivot before all `m - k` sparse columns are resolved.

#### 関数 LuFactors::l_solve_into (旧 doc、l_solve_into_pair の上に誤って付いていた段落)

Partial FTRAN through `L` only (step-space): solves `L z = P_row rhs`. Hyper-sparse (Hall & McKinnon, *"Hyper-sparsity in the revised simplex method and how to exploit it"*, 2000, §4.2 "Hyper-sparse FTRAN", Figure 3): `l_col[s]`'s entries only ever modify `z` by adding a multiple of `z[s]` itself — if `z[s]` is exactly zero, the whole inner loop is a provable no-op (every update is `x -= mult * 0`), so it is skipped entirely rather than paying for a test-against-zero (or worse, a real floating point op) per entry. Writes the result into caller-provided `z` (length `m`) instead of allocating — `FtLu`'s hot-path `solve_into` calls this once per FTRAN, so a fresh `Vec` here would mean a fresh heap allocation on every single pivot's FTRAN/BTRAN, several times over.

`active` (for pair/triple/single): for a basis whose `L` is mostly trivial — slack-heavy — this turns an `O(m)` zero-test scan into `O(#non-trivial columns)`.

#### 関数 LuFactors::l_solve_sparse_into

A second, dense-scanning implementation (`l_solve_into`) exists rather than making this the only one because a dense `rhs` (this function's own worst case: `|reach| == m`) pays for the DFS bookkeeping (stack pushes, epoch checks) on top of the same elimination work `l_solve_into` would have done anyway with a tight double loop — this function is a net win specifically when `rhs` (and hence typically `reach`) is small relative to `m`, which is the common case for the one caller that has a genuinely sparse `rhs` on hand already (`solve_lp_dual_on`'s entering-column FTRAN: a real LP's constraint columns are themselves sparse).

Why ascending numeric order is already a valid topological order (unlike the general Gilbert & Peierls 1988 presentation for an arbitrary DAG, which needs a DFS-postorder-then-reverse): every `l_col[s]` entry's `row_step` is `> s`, by construction of the elimination itself (`factorize` only ever records a multiplier for a row not yet chosen as a pivot, which by definition gets assigned some *later* step).

Precondition rationale: `z` entirely zero on entry is *not* this function's own job to (re-)establish cheaply — its own reach set only covers what the `L`-stage itself touches, but the R-eta and `U` stages downstream (in `FtLu::solve_sparse_into`) can scatter fill well beyond that set (a long-enough eta chain can, in the worst case, touch entries across the whole vector), so knowing "the previous call's `L`-stage reach" here would not be enough to correctly re-zero what a *subsequent* stage left behind. Instead `FtLu::solve_sparse_into` unconditionally clears its own dedicated `z` buffer once, in full, right before returning — a single `O(m)` `fill(0.0)` per call, far cheaper than the branchy permute-and-scan `l_solve_into` otherwise pays, and the only `O(m)` work left in the whole sparse path.

#### 関数 LuFactors::l_transpose_solve_gather_into

Unlike `l_solve`'s forward pass, a single step `s` here can read from *several* `w[row_step]` entries (one per `l_col[s]` entry), so there is no single value whose zero-ness makes the whole step a no-op — matching Hall & McKinnon §4.4's observation that BTRAN's inner-product-shaped work has "no simple way of determining [a trivial] intersection... without a computational overhead comparable to evaluating the inner product itself". Skipping per-*entry* when that specific `w[row_step]` is zero is still safe and free, just a smaller win than `l_solve`'s whole-step skip.

(A first attempt at a fuller DFS-based hyper-sparse implementation, covering all four solve directions and mirroring `L`/`U` both column- and row-major the way HiGHS does, was tried and measured *slower* end to end: the DFS setup's own per-call cost — allocating a fresh `visited` array plus an upfront `O(m)` density scan on every single call, even ones that ended up taking the dense-style branch — outweighed the fill-skipping it bought, on the order of 15-18% slower overall. That attempt was reverted in full. A second, narrower attempt — `LuFactors::l_solve_sparse_into`, covering only this module's one genuinely straightforward GP setting (`L`'s own forward direction, already stored column-major, fed a real LP's own sparse constraint column) with a *persistent*, epoch-stamped scratch (see `GpScratch`) rather than a fresh per-call allocation — measured as a small but real net win on the full Netlib benchmark set (73 problems, aggregate wall time ~1% lower, roughly even split of individually-faster/slower instances, zero objective mismatches) once the specific cost the first attempt's own revert blamed — the allocation, not the algorithm — was actually removed. This `L^{-T}` direction (BTRAN's tail) was deliberately *not* attempted a second time: Hall & McKinnon's observation above still applies unchanged (no static column-major structure of `L` to run the same DFS over without adding a row-major mirror), and the first attempt's win was concentrated in the one direction with a genuinely sparse, already-available seed — this direction's own `w` typically isn't.) — Later superseded by the `l_row` scatter form below.

#### 関数 LuFactors::l_transpose_solve_scatter_into

The two loops compute the same `L^T w' = w` back substitution over the same nonzeros, only associating the updates differently: the gather form accumulates *into* `w[s]` one `l_col[s]` entry at a time (so `w[s]`'s own value is only known once every one of them has been read, and no prefix of them can be skipped as a group), while this form propagates *out of* `w[s]` into every `l_row.row(s)` entry at once. Because `w[s]` is the single multiplicand of that whole inner loop, `w[s] == 0.0` makes the entire step a provable no-op — the same whole-step skip `l_solve_into` and `FtLu::u_solve_into` already exploit in the forward direction, and the one Hall & McKinnon (2000) §4.4 explains the gather form *cannot* have. HiGHS reaches the same skip the same way, via its own row-major `lr_*` copy of `L` in `btranL`.

`docs/lu_comparison_enomoto_vs_highs.md` §2.6 names this as the one of HiGHS's four hyper-sparse solve directions this crate had never attempted (the *reason* being precisely that no row-major `L` existed to attempt it with — this method adds it).

Not bit-identical to the gather form: the same set of products is summed into each `w[s]` in the opposite order (descending source step here, `l_col[s]`'s own stored order there), so results can differ in the last ulp and, through the dual ratio test's tie-breaks, shift iteration counts either way on degeneracy-heavy instances. That is measured, not assumed — see this change's own analysis note for the per-problem numbers.

#### Forrest-Tomlin 更新 (セクション冒頭コメントの原文)

Following Forrest, J.J.H. and Tomlin, J.A., "Updated triangular factors of the basis to maintain sparsity in the product form simplex method", Mathematical Programming 2 (1972), 263-278, as summarized precisely with full derivations in Huangfu, Q. and Hall, J.A.J., "Novel update techniques for the revised simplex method", Technical Report ERGO-13-001, University of Edinburgh (2013) §2.1 (equations 1, 4-13).

Column replacement `B̄ = B + (a_q - B e_p) e_p^T` is rearranged via the fixed factorization `B = LU` as `L^{-1} B̄ = U + (L^{-1}a_q - U e_p) e_p^T = U + (ã_q - u_p) e_p^T = U'` replacing column `p` of `U` with the partial FTRAN result `ã_q = L^{-1} a_q`. This "spikes" column `p` of `U` (rows > p can now be nonzero, breaking triangularity). Triangularity is restored by one row transformation `R^{-1} = I - e_p r^T` that zeros row `p` across every column: `Ū = R^{-1}U'`, where `r^T = ū_p^T U^{-1}` (`ū_p` = row `p` of `U` without its diagonal) can be obtained at negligible cost from `ẽ_p^T = e_p^T U^{-1}` (a partial BTRAN already computed to derive `r`) as `r = -u_pp · ẽ_p` with the `p`-th entry forced to zero (Tomlin 1974, eq. 12 in the 2013 paper). `R^{-1}` applied to column `p` of `U'` only changes its `p`-th entry: `ã_pq := ã_pq - r·ã_q`.

`L` never changes across updates. `U` is kept as a *sequence* of per-slot column etas (pivot + off-diagonal vector), because after a replacement the slot's eta is removed from wherever it sits and *appended* to the end — this ordering, not raw slot order, is what FTRAN/BTRAN through `U` must respect once updates have happened (verified by hand against a direct dense re-solve while implementing this). Each update additionally produces one `R` row-eta, kept in its own creation-ordered list and applied between `L` and `U` per `B_k^{-1} = U_k^{-1} R_k^{-1} ... R_1^{-1} L^{-1}` (eq. 13).

An eta's off-diagonal entries were a `HybridVec`: a sparse `(row_step, value)` list while the eta is genuinely sparse, a dense length-`m` array once its fill exceeds `DENSE_ETA_FRACTION` of `m` (typical of a dense-coefficient LP, where `U`'s eta chain is already close to fully dense from the very first update). That type's docs covered the trade-off, the skipped-slot convention that lets the dense form's loops run over the whole array unconditionally, and why its two consuming operations (`dot_dense`, `axpy_into_dense`) lived there rather than being re-written as a two-armed `match` at each of this file's eight FTRAN/BTRAN call sites. (Now superseded by `EtaFile`, which reproduces `HybridVec`'s semantics exactly.)

#### 構造体 EtaFile

Instead of a `Vec` of structs each owning its own heap `Vec` of `(usize, f64)` pairs. An FTRAN `U` stage then reads 4 B per skipped eta (its `key`) and 12 B per entry, against 48 B per eta header and 16 B per entry (plus a pointer chase per eta) before. The dense form is kept exactly as `HybridVec`'s dense arm, so every loop computes precisely what the per-eta `HybridVec` computed — same entries, same order, same sparse/dense choice — and the results are bit-identical. (`dot` = `HybridVec::dot_dense` exactly; `axpy` = `HybridVec::axpy_into_dense` exactly; `remove_index` = `HybridVec::remove_index` exactly; `push_scaled_dense` = what `HybridVec::pack_scaled_dense` would build, with its own two-pass shape minus its per-eta allocation.)

Replacing a `U` eta removes its header from the parallel arrays (a `memmove` of 20 B per later header, against 48 B per `UEta` before — tombstoning instead was measured to cost more in the per-header dead test of every FTRAN/BTRAN than it saved here). `n_headers` doc said "dead ones included" (a leftover from the tombstoning variant). `span`: one load per eta.

#### 関数 expected_dense_gate

Any value `>= 1.0` disables the result-density gate outright, since no result can be denser than `m`, restoring the input-nnz-only dispatch this crate had before `FtranDensity` existed — which is exactly how the A/B runs behind the constant's own value were produced. Read once per `FtranDensity::new` — a handful of times per solve, never on the per-iteration path.

#### 関数 build_l_row

The empty case exists so that arm of the A/B is *genuinely* this crate's pre-§2.6 behaviour, construction cost included. Building `l_row` and then never reading it would leave the transpose's own `O(nnz(L))` build — paid at **every** refactorization, `dfl001` alone refactorizes ~100 times — inside both arms, hiding exactly the cost that has to be weighed against the scatter form's own win. Measuring a change against a baseline that already pays for it is how a feature gets adopted on a number that was never real. An all-empty `CscMat` transposes into a `CsrMat` with `m + 1` zero offsets and no entries, so `row(i)` stays valid (and empty) for every `i` rather than needing a separate `Option` on the hot path.

#### 関数 btran_l_scatter_gate

(Its doc comment had drifted above `tiny_drop`.) `0` disables the scatter form outright (restoring the pre-`l_row` gather-only BTRAN, which is how the A/B behind the constant's own value is produced), `1` forces it unconditionally. Read once per refactorization, never per solve, same as `expected_dense_gate`.

#### 関数 u_zero_skip_enabled

`ENOMOTO_FTRAN_U_ZERO_SKIP=0` restores the unconditional divide, which is how the A/B behind the default is produced.

#### 構造体 FtranDensity

`DENSE_RHS_FRACTION` alone judges a solve by its *input*: the reach set the Gilbert-Peierls path walks is bounded below by the rhs's own nonzeros, so a dense rhs does prove the sparse path cannot win. The converse is not true — a one-nonzero rhs can still fill in to a fully dense `B^-1 a` once `L`'s own reach fans out, and then the sparse path has paid its DFS/epoch bookkeeping on top of doing the same elimination work the flat dense scan would have done anyway. Nothing about the *input* distinguishes those two cases, and the gap widens exactly as `m` grows: the bigger the basis, the further a single column's reach can fan out relative to the fixed sparsity of the column itself.

What does distinguish them is the channel's own recent history, which is what this tracks: HiGHS solves the same problem the same way, maintaining a per-operation `expected_density` running average (`HEkk::updateOperationResultDensity`) and handing it to `ftranL`/`ftranU` so each call can decide *before* running which mode it should be in (`HFactor::ftranL`'s own `expected_density > kHyperFtranL` test). This crate's `docs/lu_comparison_enomoto_vs_highs.md` §2.7 names that as the gap this type closes.

One instance per *call site*, never one shared instance: those channels' densities genuinely differ — a BFRT combined rhs sums whole flipped columns and is routinely much denser than a single entering column — and averaging them together would smear each one's own signal. Instances live in the solve loops (`solve_lp_dual_on` and `extended_dual`'s two loops), not in `FtLu` itself, deliberately: `FtLu` is rebuilt from scratch at every refactorization, which would throw the history away precisely when the basis is at its densest, whereas HiGHS's own densities likewise live in `HEkk` and survive across INVERTs.

The measurement itself is free: every solve path already ends in an `O(m)` permutation loop over the finished result, so counting that result's nonzeros costs one branchless add per entry inside a loop that was already running — and it is the *exact* result density, not an estimate. Crucially it is also taken on **both** branches, so the gate can never latch: a channel that starts producing sparse results again is observed doing so while it is on the dense path, and returns to the sparse path on its own. Field `expected` starts at `0.0` so a fresh channel dispatches exactly as it did before this type existed until it has actually observed something.

#### 構造体 FtLu

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

#### 関数 FtLu::new

- `U`'s etas straight into the flat file: exactly what the former per-slot `Vec` + `HybridVec::pack` produced, same sparse/dense choice.
- S16 flop term: Unlike `nnz(L+U)` it grows quadratically with the dense tail's size, which is where this crate's Markowitz refactor time concentrates (`dfl001`).

#### 関数 FtLu::l_transpose_solve_into

See `BTRAN_L_SCATTER_FRACTION`'s history for why both forms have to stay.

#### 関数 FtLu::should_use_dense_solve_tracked

Deliberately only ever moves calls *towards* the dense path, never away from it: a rhs with more than `DENSE_RHS_FRACTION` of `m` nonzeros bounds the Gilbert-Peierls reach set below by that same count, so no amount of "but this channel's results are usually sparse" history could make the sparse path win on such a call. (`docs/lu_comparison_enomoto_vs_highs.md` §2.7.)

#### 関数 FtLu::u_transpose_solve_into / u_transpose_solve_seeded / u_transpose_sweep

- Hyper-sparse via `row_owners`, unlike `u_solve_into` (where a GP-style DFS reach set was tried and reverted). The earlier gather form reached the same sparsity through "needed" marks (`analysis/greenbea_20260921_090812.md` §3-4), but still paid each needed eta's whole column dot product.
- `u_transpose_solve_seeded`: Seeding the "needed" set from `seed` directly was *exact*: the seeding loop marked precisely the steps where `z` is nonzero, and for a permuted unit vector that set is exactly `{seed}`. What it saves is that `O(m)` scan — and, at the call site, the `O(m)` permutation *gather* (`z[s] = rhs[col_perm[s]]`, a random-access read per step) that materialized the unit vector in the first place, which `seed_unit_rhs` replaces with a flat `fill` and one store.
- Sweep: the work is the nonzero `p`s' row lengths, not, as the gather form this replaced did, every needed eta's whole column dot product regardless of how few of that column's inputs are nonzero (26x more entries on `fit2p`, whose few dense border columns every BTRAN re-read). Changes the summation order into each `z[q]`, so the last bits — not the math — differ from the gather form.

#### 関数 FtLu::u_solve_into

(A GP-sparsified counterpart to this function — restricting the scan to a DFS-computed reach set over `u_seq`'s own dependency graph, exactly mirroring `LuFactors::l_solve_sparse_into`'s own approach for `L` — was fully implemented, proven correct (an inductive argument that every `off_diag` target always sits at a strictly *lower* `u_seq` position than its referrer, mirroring `L`'s own low-to-high property, so descending position order needs no separate topological-sort step either) and tested (multiple sequential `try_update` calls reordering `u_seq` non-trivially, checked against the dense reference after every single one). It was still reverted after measuring it on the full Netlib benchmark set: aggregate wall time **+10.4%** versus `L`-only sparsification, 52 of 73 problems slower and only 6 faster. Unlike `L` (a *static* matrix, fixed once per full refactorization, whose seed — a real LP's own sparse constraint column — is reliably sparse), `U`'s own eta chain accumulates fill from every `try_update` since the last refactorization, so its reach set is typically far less sparse in practice — the DFS/reach-tracking overhead this function's outer loop is cheap enough to not need in the first place stopped paying for itself.)

(C5, third form, opt-in: `u_solve_hyper` — gated per channel on the caller's result-density average (`ENOMOTO_FTRAN_U_HYPER`, `ENOMOTO_FTRAN_U_HYPER_TAU`), aborting to this scan past `ENOMOTO_T_U_HYPER_ABORT` of `m`, and replacing the `O(m)` output permutation with a list scatter as well — the O(m) passes, not the eta loop alone, are what a sparse FTRAN is bound by.)

- `ENOMOTO_FTRAN_U_ZERO_SKIP=0` arm is the pre-§2.6 loop exactly, so that arm of the A/B is this crate's own previous behaviour and not "previous behaviour plus one unrelated change".
- Zero test before dividing: `0.0 / pivot` is `±0.0`, so an already-zero slot's division is a no-op that still costs a division and — worse on a hyper-sparse right-hand side — a store back into a random position of `x`, dirtying a cache line per zero slot for nothing. The skipped store can leave `+0.0` where the unconditional one would have written `-0.0` (when `pivot < 0`), which is exactly the difference `u_transpose_solve_into`'s own hyper-sparse skip already accepts, on the same grounds: every consumer of this result branches on zero-ness (`permute_out`'s own `!= 0.0` count, `commit_update`'s filter, the PRICE/DSE consumers), never on the sign of a zero.

#### 関数 FtLu::u_solve_hyper

Applying the reached etas in the scan's own order keeps each entry's accumulation order (a DFS topological order alone would not). Sorting is `O(r log r)` in the reach size `r`, against the full scan's `O(m)`. A dense-arm eta's `axpy` spans all of `x`, hence the bail-out.

#### 関数 FtLu::ftran_through_l_and_r_into

The existing `R`s are already part of the "L-like" fixed factor that update `k` treats as known, since `B_{k-1} = L R_1 ... R_{k-1} U_{k-1}` (eq. 13) rather than `B_{k-1} = L U_{k-1}` once `k > 1`. The unconditional `R`-eta tick term is the very "gather-type, no zero-skip" cost the CLOCK trigger's own analysis (§2.1/§2.2) identified as the eta-chain bottleneck, so this term alone is what makes `tick` grow with chain length the way FTRAN's own measured wall time does.

#### 関数 FtLu::solve_into

`solve_lp_dual_on` calls this 2-4 times *every pivot* (BTRAN-DSE's `tau`, the entering column's `alpha`, and, when BFRT flips are pending, one more for `combined`), so the 2-3 `Vec` allocations each fresh `solve()` call used to cost here (one each in `l_solve`, `u_solve`, and the final permutation) were real, repeated per-iteration heap traffic — eliminated by having the caller own `scratch`/`out` once, outside the iteration loop, and reuse them every pivot. The returned nonzero count comes from the permutation loop that already visits every entry: one branchless add per entry, exact density rather than an estimate.

#### 関数 FtLu::solve_into_capture

See `try_update_precomputed`'s history for why this capture (a plain `copy_from_slice`) lets the caller skip `try_update`'s own redundant re-derivation of the exact same value entirely.

#### 関数 FtLu::solve_into_pair_capture (および sparse/triple 版)

The entering column's FTRAN and the DSE `tau = B^-1 rho_p` FTRAN of the same iteration, which HiGHS runs as two separate (optionally concurrent) solves (`HEkkDual::updateFtranDSE`). The synthetic tick is charged exactly as the two separate calls would charge it too, so the `CLOCK` refactorization trigger fires on exactly the same iterations. What is saved is the second pass over `L`/`R`/`U`'s own storage (memory traffic and loop overhead), which on the larger Netlib instances no longer fits in cache between the two solves.

#### 関数 FtLu::permute_out

`+= (v != 0.0) as usize` rather than a branch: the compare is a single instruction and the add is unconditional, so the count adds no branch misprediction to a loop whose scatter already dominates it.

#### 関数 FtLu::solve_sparse_into

`l_solve_into` (the dense path) starts by unconditionally overwriting every entry of `scratch` (`z[s] = rhs[row_perm[s]]` for every `s`), so it tolerates arbitrary leftover content — but `l_solve_sparse_into` requires `scratch` to *already* be all-zero on entry. The guarantee only holds if nothing else writes through the same buffer in between.

`U` stays on the plain `u_solve_into` scan — see that function's history for the *two* separate attempts at a reach-restricted counterpart (one ungated, one gated exactly the way HiGHS gates its own `ftranU`) that were both implemented, proven correct, measured over the full Netlib set, and reverted as regressions.

#### 関数 FtLu::add_zero_rhs_solve_ticks / add_zero_rhs_btran_ticks

Lets a caller skip a provably-zero FTRAN (e.g. the extended dual's BFRT slope channel when no flipped column's width carries an `M` term). For a zero rhs: the dense `L` stage costs a flat `m`, the sparse one's reach set is empty (`0`); every `R` eta is visited unconditionally; the `U` stage pays its flat `m` and no eta survives the zero skip. BTRAN: no `R` eta (each is skipped on its zero `yp`); used by the extended dual's all-slack-cost start (S14).

#### 関数 FtLu::solve_sparse_into_capture

The capture happens after the `R`-eta loop (this stage's own last write to `scratch` before `u_solve_into` takes over), so `a_tilde_out` ends up identical regardless of which of the two FTRAN paths (`should_use_dense_solve`'s dense/sparse dispatch) a given call took.

#### 関数 FtLu::solve_transpose_into / seed_unit_rhs / btran_tail_cap

- `solve_transpose_into` is `solve_lp_dual_on`'s once-per-pivot BTRAN for `rho_p`; no allocation for the same reason as `solve_into`.
- `seed_unit_rhs`: `P_col^{-1} e_i` is the unit vector at step `col_perm_inv[i]`, so the gather collapses to a flat `fill` plus a single store — no random-access read per step, and the caller never has to own (or keep re-zeroing) a length-`m` `e_i` buffer of its own.
- `btran_tail_cap` tick comment: `l_transpose_solve_into` was a dense `O(m)` reverse scan regardless of fill (see that method's history for why sparsifying it wasn't worth trying — later superseded by the `l_row` scatter form, but the tick accounting still charges a flat `m`).

#### 関数 FtLu::solve_transpose_unit

Unlike `solve_transpose_unit_into` this makes no assumption about `u_seq`'s ordering, so it is valid with Forrest-Tomlin updates applied.

#### 関数 FtLu::solve_transpose_into_capture

**No production call site left**: every `e_tilde`-capturing BTRAN in this crate has a unit-vector right-hand side and goes through `solve_transpose_unit_capture` instead. Kept, rather than deleted, because it is the general-`rhs` reference that specialization is *checked against* — `solve_transpose_unit_is_bit_identical_to_the_dense_unit_rhs_path` asserts the two agree entry for entry, and on the synthetic tick, both on a fresh factorization and after Forrest-Tomlin updates have reordered `u_seq`. Deleting it would delete the proof.

#### 関数 FtLu::solve_transpose_unit_into

Built for `DseState::from_basis`'s own `m` back-to-back unit-vector solves after every refactorization — measured as 27% of `dfl001`'s total wall time before this method existed (`dfl001-bottleneck-max-iters-cap` memory), because that call site pays `solve_transpose_into`'s full `O(m)`-per-call cost `m` times over, every refactorization.

`update_count()` is already the cheapest possible signal, so this method itself only asserts it rather than re-deriving it. The optimization exploits a structural invariant of a *freshly factorized* `u_seq` (`FtLu::new`'s own construction: `u_seq[slot]`'s `off_diag` entries only ever reference `row_step < slot`, `U`'s own upper-triangular structure) that `try_update` is free to break (its own Forrest-Tomlin bump-and-replace algorithm reorders `u_seq` and can introduce entries referencing a *later* row-step than before).

**The optimization**: permuting `e_i` (`col_perm_inv[i]`) yields a single nonzero at step `s0`; the forward recurrence can only ever produce a nonzero at slot `p` if some earlier slot `< p` it depends on is already nonzero — with nothing nonzero below `s0`, every slot `< s0` is therefore provably still `0` after the sweep, without computing a single one of their dot products. Starting the sweep at `s0` instead of `0` is exact, not approximate, and needs no DFS/epoch bookkeeping the way a full Gilbert-Peierls reach-set restriction would (see `[[dfl001-bottleneck-max-iters-cap]]`/this crate's history for why a *fuller* sparsification of the shared `u_transpose_solve_into` — applied to every per-iteration `rho_p` BTRAN, not just `from_basis`'s refactor-time calls — was tried and reverted as a net aggregate regression across the full Netlib set): that measurement's DFS/epoch overhead was paid on tens of thousands of per-iteration calls across many small problems where the skip bought little; this plain prefix skip carries no such per-call bookkeeping cost, and `from_basis`'s own access pattern (`m` calls, but only at refactor time) concentrates exactly on the large/refactor-heavy instances (`dfl001`, `pilot87`) a fuller sparsification would have helped too, without the small-problem dilution that sank the earlier attempt.

`L^{-T}` is left exactly as dense as it always was — a *second* attempt to sparsify it specifically was never worth trying (its own input is typically no longer sparse by that point, fill having already spread across `[s0, m)` during the `U^{-T}` sweep). Postcondition rationale: `L^{-T}`'s own reverse sweep can scatter fill back into positions below `s0`, so (unlike `l_solve_sparse_into`'s narrower reach-set cleanup) nothing cheaper than a full `O(m)` reset is safe here. The singleton loop's empty sum reproduces what the former `HybridVec::dot_dense` returned.

#### 関数 FtLu::try_update

FT needs only the partial FTRAN result `L^{-1}a_q` (eq. 1), unlike a product-form update which would need the full solve.

**Schork & Gondzio (2017), "Permuting Spiked Matrices to Triangular Form and its Application to the Forrest-Tomlin Update"**: tried and reverted. The idea: when the spike's own diagonal `a_tilde[p]` is nonzero and its off-diagonal support is disjoint from the structural `Reach(p)` (every slot whose value transitively depends on `p` — a single forward walk over `u_seq`, mirroring `u_transpose_solve_into`'s own traversal but following every *stored* `off_diag` edge unconditionally rather than only the ones whose *propagated* value under one unit-impulse seed happens to still be nonzero — the two differ on real, coefficient-heavy LP data via exact numerical cancellation, confirmed against real Netlib instances via a dedicated invariant cross-check during development), the spiked matrix is *already* permutable to triangular form with no elimination and no `REta` at all (their Theorem 3.1 / Lemma 3.2) — repositioning `p` and every member of `Reach(p)` to the end of `u_seq`, preserving their relative order, instead.

Implemented fully correctly (including the structural-vs-numerical reach distinction above, found and fixed via a randomized stress test plus real-Netlib debug cross-checks) and, separately, a real unrelated bug it exposed (`simplex.rs`'s `FT_MAX_UPDATES` hard refactorization cap read `update_count()`, i.e. `r_etas.len()` — which a permutation-only update never grows, so on instances where many updates resolve that way the cap could go uncrossed far longer than intended, letting numerical drift compound until a later refactorization hit a matrix too corrupted to factor; fixed by counting *every* successful update, not just row-eta ones, for that specific trigger). Even after replacing an initial `HashSet`-based reach implementation with an epoch-stamped array (the same bump-instead-of-clear trick `sparse_lu::GpScratch` already uses), full-Netlib measurement still showed a net regression — not from this function's own added cost (which the epoch-array version brought back down close to baseline), but because the permutation path's slightly different rounding characteristics than the standard row-eta path perturbed dual-simplex tie-breaks on degeneracy-heavy instances (`pilotnov` needed 3218 iterations instead of 1286 for the *same* correct answer) — a downstream effect no amount of tuning this function itself can address. See the project history around this doc comment's own commit for the full numbers if revisiting.

- Scratch reuse: the `e_tilde` zeroing is still one `O(m)` pass, exactly as `vec![0.0; m]` used to pay for its own zero-initialization — what this reuse actually saves is the allocator round-trip itself, not this fill.

#### 関数 FtLu::try_update_precomputed

**Why this exists**: a typical dual-simplex iteration already runs exactly the two solves `try_update` used to redo from scratch, for its own unrelated purposes — `rho_p = B^-T e_p` (`solve_transpose_into`, needed for PRICE) computes `U^-T e_p` as an internal step before applying the `R`-etas and `L^-T`, and the entering column's own FTRAN (`solve_into`/`solve_sparse_into`, needed for the primal update and DSE) computes `(L R_1...R_{k-1})^-1 a_q` as an internal step before applying `U^-1` — both are simply overwritten in place by the next stage rather than kept. Since `p`/`a_q_original` are identical between that earlier call and this update (same leaving row, same entering column, same iteration, `self` unchanged in between), the values are not merely *equivalent* to what `try_update` would recompute — they are bit-for-bit identical, `ftran_through_l_and_r_into`/`u_transpose_solve_into` being pure functions of `(self, input)`. Capturing them (a plain `copy_from_slice`) is far cheaper than either of the two full solves this replaces — a dense `O(m)` pass through `L` plus every accumulated `R`-eta for `a_tilde`, and an `O(nnz(U))` scan of the whole eta chain for `e_tilde`, both of which grow as updates accumulate since the last refactorization.

#### 関数 FtLu::commit_update

Pulled out into its own `&mut self` method (rather than duplicated in both callers) specifically so the intricate `row_owners`/`slot_pos`/`u_seq` bookkeeping — the part a copy-paste split would risk drifting out of sync between two copies — exists in exactly one place.

- The `R` eta is built straight out of `e_tilde` — same entries, same order, same sparse/dense choice as the `collect()`-then-`HybridVec::pack` this replaces, minus that intermediate `Vec`. The previous code likewise materialized the whole thing before testing, so a rejected update is no more expensive than it already was.
- `Vec::remove` shifts every later element down by one position — update `slot_pos` for exactly that range (elements the memmove itself already touches, so this is no extra asymptotic cost) rather than the old `find_seq_pos`'s full O(m) re-scan.
- Replacement column: built directly from `a_tilde` rather than through a throwaway pair list.

#### 関数 FtLu::fill_count

As updates accumulate, the eta file grows (each `R` and each replaced `U` slot can carry up to `m-1` entries), which is exactly the cost trigger (3) exists to bound. Counts true nonzeros (formerly via `HybridVec::nnz`), not storage length, so switching an eta to the dense representation doesn't spuriously inflate this and trip the trigger early.

#### テスト (mod tests) — 旧 doc コメント中の経緯・理由

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


### src/params.rs (mod lu) — 定数ドキュメントの原文 (測定経緯)

(Phase 1 で lu.rs から移された英語 doc コメントの全文。`DENSE_INPUT_FRACTION`/`BORDER_MAX_FRACTION` の doc 先頭に紛れ込んでいた `factorize` の Dulmage-Mendelsohn 試行の段落は上の「関数 factorize」節に移した。)

#### 定数 STABILITY

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

#### 定数 PIVOT_THRESHOLD_MAX

Ceiling on the escalated pivot threshold ([`escalate_pivot_threshold`])
— HiGHS's own `kMaxPivotThreshold`. [`STABILITY`]'s docs record that a
*static* `0.5` costs ~4% on `d2q06c`/`greenbeb`/`fit2p` through extra
fill-in, which is exactly why this value is reachable only after the
escalation ladder below has evidence that *this* solve is paying more
for instability than it would for fill.

#### 定数 PIVOT_THRESHOLD_MIN

Floor for an operator-supplied `ENOMOTO_PIVOT_THRESHOLD` — HiGHS's own
`kMinPivotThreshold`. Nothing escalates *downwards*, so this only ever
clamps the env override.

#### 定数 PIVOT_THRESHOLD_FACTOR

Multiplier applied per [`escalate_pivot_threshold`] step. HiGHS uses
`kPivotThresholdChangeFactor = 5.0` from a `0.1` default; from this
crate's `0.25` a factor of `2.0` lands exactly on
[`PIVOT_THRESHOLD_MAX`] in one step, so the ladder here is
`0.25 -> 0.5`, and a second escalation is a no-op.

#### 定数 DENSE_COL_FRACTION

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

#### 定数 PIVOT_SEARCH_LIMIT

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

#### 定数 KERNEL_LINEAR_SCAN_MAX

Rows shorter than this are searched for a column linearly rather than
by binary search ([`KernelMatrix::row_get`]). Markowitz elimination is
specifically choosing pivots to keep the active rows short, so the
linear branch is the common one: a run this size fits in one or two
cache lines and scans branch-predictably, where `binary_search` pays a
mispredict per level for the same work.

#### 定数 U_HYPER_ABORT_FRACTION

C5 hyper-sparse `U` stage: give up (and take the plain full scan) once
the DFS has reached more than this fraction of the `m` slots — past it,
sorting the reach and scattering the result by list stop paying for
themselves (HiGHS's own `kHyperFtranU` is `0.10`).

#### 定数 DENSE_INPUT_FRACTION

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

#### 定数 BORDER_MAX_FRACTION

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

#### 定数 BORDER_MAX_COUNT

(doc なし; 直前の定数と説明を共有)

#### 定数 REBUILD_FILL_LIMIT

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

#### 定数 REBUILD_MIN_PIVOT

Absolute floor on a pivot's magnitude: below this the column has
nothing usable left in the remaining submatrix at all, and the whole
attempt is abandoned rather than dividing by (almost) zero. Far below
`simplex.rs`'s own `FT_MIN_PIVOT` deliberately — this is a "there is no
pivot here" test, not a quality test, which the [`STABILITY`] check
next to it already is.

#### 定数 REUSE_BACKOFF_SHIFT_CAP

Caps on [`factorize_reusing`]'s own exponential backoff after a
rejected reuse: the streak's shift is capped first (so the shift itself
can never overflow), then the resulting skip count.

#### 定数 REUSE_MAX_BACKOFF

(doc なし; 直前の定数と説明を共有)

#### 定数 DENSE_ETA_FRACTION

A column/row whose off-diagonal fill exceeds this fraction of `m` is
stored densely (see [`HybridVec`]). Unlike [`DENSE_COL_FRACTION`] (tuned
against real Netlib data, all of it sparse), this threshold has no
dense-problem benchmark to tune against yet in this crate's own test
set — `0.4` is a first-pass value, not a measured one; re-tune once a
genuinely dense-coefficient LP is available to benchmark against.

#### 定数 DENSE_RHS_FRACTION

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

#### 定数 DENSITY_AVERAGE_MULTIPLIER

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

#### 定数 EXPECTED_DENSE_FRACTION

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

#### 定数 BTRAN_L_SCATTER_FRACTION

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

#### 定数 TICK_BUILD_M_COEF

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

#### 定数 TICK_BUILD_LU_COEF

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

#### 定数 TICK_BUILD_FLOP_COEF

Per-multiply-add coefficient of the elimination flop term in
[`FtLu::build_tick`] (S16, `ENOMOTO_T_TICK_BUILD_FLOP_COEF`). `0` (the
default) leaves `build_tick` exactly the HiGHS-shaped `m`/`nnz(L+U)` sum.

#### 定数 TICK_SOLVE_NNZ_COEF

Per-nonzero coefficient applied to every solve-stage tick increment
(`R`-eta nonzeros touched, `U`/`U^T`-eta nonzeros touched, `L`-stage
reach-set size) — kept at `1` (i.e. `tick` is a plain nonzero count,
unscaled) so [`SYNTH_CLOCK_FACTOR`] alone carries the crate-specific
per-nonzero cost ratio between this crate's own solves and HiGHS's; splitting that ratio across two constants
(this one and the factor) would make calibration harder to reason about
with no accuracy benefit, since both only ever appear multiplied
together in the trigger's own comparison.

#### テスト群 (solve_transpose_unit / sparse_solve / density_gate / dense-basis FT 更新) の元コメント

4ab948b 時点の `src/simplex/lu.rs` 6699 行目以降のテストに付いていたコメントをそのまま転記。

##### solve_transpose_unit_is_bit_identical_to_the_dense_unit_rhs_path

Once fresh, and again after Forrest-Tomlin updates have
reordered `u_seq` — the case `solve_transpose_unit_into`'s
own prefix-skip is *not* valid for, and the whole reason
this seeded variant exists alongside it.
The non-capturing form must agree with both.
The tick is what drives the deterministic CLOCK
refactorization trigger, so the two paths must charge
identically or the solve would take a different
trajectory (see `u_transpose_sweep`'s own note).
`solve_transpose_unit_work` (dedicated zero-kept scratch, tracked
`e_tilde` copy, bounded `L^T` gate) and the `StepCapture`-driven GP
`L` stage of the fused `tau` FTRAN must be bit-identical to the plain
paths, ticks included, fresh and after Forrest-Tomlin updates.

##### unit_btran_work_and_tau_gp_are_bit_identical

Fused tau FTRAN with and without the capture.
`solve_into_pair_capture`/`solve_sparse_into_pair_capture` must be
bit-identical to the separate single-vector solves they fuse — both
results, the `a_tilde` capture, the returned nonzero counts, and the
synthetic tick (which drives the CLOCK refactorization trigger) —
on a fresh factorization and after Forrest-Tomlin updates.

##### pair_ftran_is_bit_identical_to_two_separate_solves

`a`: a sparse column-like rhs; `b`: a dense-ish one
(the DSE `rho_p` shape), varied per variant.
Sparse-`a` form, against `solve_sparse_into_capture` + `solve_into`.

##### hyper_u_ftran_is_bit_identical

Sparser than `random_sparse_diag_dominant`: keep one
off-diagonal entry per row, so `U`'s reach from a short rhs
stays under the abort fraction.
Variant 3 gives `b` a single nonzero, so its hyper
`U` stage runs rather than aborting.
`tau` channel through a step capture (C3's GP `L`
stage for `b`), hyper-sparse `U` when requested.
Direct check that the hyper stage really ran
(rather than aborting) on some of these.

##### to_sparse

Runs `rhs` (converted to sparse form) through `solve_sparse_into`
and asserts it matches `state.solve(rhs)` (the dense reference)
exactly — both should compute the identical sequence of floating
point operations restricted to the same reach set, just reached by
different bookkeeping, so unlike `approx_vec`'s tolerance
elsewhere in this module (guarding against genuinely different
numerical paths, e.g. FT-updated vs freshly-refactored), this
checks bit-for-bit equality — any mismatch at all means the reach
set or the zero-management between calls is wrong.

##### sparse_solve_matches_dense_on_simple_case

Same 3x3 tridiagonal fixture as `factorize_and_solve_matches_expected`.

##### sparse_solve_matches_dense_with_ft_updates

Same fixture (and update sequence) as
`ft_update_chain_of_two_matches_full_refactor` — `state.r_etas`
is non-empty here, exercising the sparse path's R-eta stage
(unchanged from the dense path, but only actually run if this
wiring is correct).

##### sparse_solve_repeated_calls_reuse_scratch_correctly

A larger (6x6), more sparsely-structured matrix — enough steps
and fill-in variety that the reach set genuinely differs across
calls — solved for a long, varied sequence of sparse right-hand
sides (single nonzero, several scattered nonzeros, fully dense,
and all-zero) through the *same* `scratch`/`gp` buffers, back
to back. This is the specific scenario `l_solve_sparse_into`'s
"z must be all-zero on entry" precondition depends on
`solve_sparse_into`'s own end-of-call `fill(0.0)` to uphold —
if that cleanup were wrong or incomplete, an *earlier* call's
leftover values would corrupt a *later* call's result, so
running many varied calls in sequence and checking every one
(not just the first) is the point of this test.
The buffer must be back to exactly zero after the last call too
— not just "happened to match the expected output" — since
that's the invariant the *next* caller (whoever it is) relies on.

##### sparse_solve_matches_dense_after_reordering_u_seq

`solve_sparse_into` (sparse `L` + dense `U`) must keep matching
the dense reference through *repeated* `u_seq` reordering
(every `try_update` removes one eta from wherever it sits and
appends a fresh one at the end, shifting everything after the
removal point down by one) — a single update, as the other
FT-update tests already exercise, isn't enough to be confident
this stays right across several. Five sequential updates on a
6x6 basis, each replacing a different slot (including slots at
both ends and the middle of the current `u_seq`, so removals
happen at varied positions), checked against the dense
reference for a run of varied sparse right-hand sides after
*every single* update — not just the final one.
Updates hit slot 4 (near the end), then 0 (the start), then 5
(the new end), then 2 (the middle), then 1 — deliberately not a
monotonic sequence, so `u_seq`'s position-vs-slot relationship
is scrambled well beyond a simple "always append" pattern.

##### should_use_dense_solve_flags_dense_rhs_and_not_sparse

A rhs with 5 of 10 entries nonzero exceeds DENSE_RHS_FRACTION (0.4).
The result-density gate (`docs/lu_comparison_enomoto_vs_highs.md`
§2.7): a channel whose *results* keep coming back dense must end up
on the dense path even when every right-hand side it is handed is
sparse enough for `should_use_dense_solve` alone to say otherwise —
and must find its way back to the sparse path once the results turn
sparse again, since the average is recorded on both branches.

##### density_gate_flips_a_sparse_rhs_channel_dense_and_back

Untouched history: dispatch is exactly `should_use_dense_solve`'s.
Fully dense results, iteration after iteration: the running
average climbs past EXPECTED_DENSE_FRACTION (0.35) and the same
sparse rhs now dispatches dense.
...and back: nothing latches, because the dense branch records
its own result density too.
Every FTRAN entry point reports the nonzero count of the result it
just wrote — the measurement [`FtranDensity::record`] is fed — and
the dense and sparse paths agree on it, since they compute the
identical vector.

##### solve_paths_report_the_result_nonzero_count

Lower-bidiagonal `B`: `B^-1 e_0` fills in over *every* row, so a
one-nonzero rhs has a fully dense result — exactly the case the
input-side test alone cannot see coming.
A dense-coefficient basis (every column of `B` has all `m` entries,
well past `DENSE_ETA_FRACTION` after a couple of FT updates) run
through several `try_update` calls with equally dense entering
columns, then cross-checked against a completely independent full
refactorization of the final basis — the same style of ground truth
the sparse-fixture tests above use, just sized and shaped to
actually exercise `HybridVec`'s dense arm instead of its sparse one.

##### ft_update_matches_full_refactor_on_dense_basis

Dense entering columns (every entry nonzero), replacing a few
different slots.
`solve_sparse_into` must still agree too, even though a dense
rhs is routed around it at the `simplex.rs` call sites (see
`should_use_dense_solve`'s own docs) — it remains public API and
must stay correct regardless of caller choice.

## 前処理コア (presolve.rs / aggregator / redundancy / scaling / propagate / colsingleton / doubleton / freevar)
コード中にあった開発経緯・計測値・却下案などのコメントを移したもの (原文の英語のまま)。

### src/presolve.rs

#### モジュール冒頭 (//!) — 旧モジュール文書全文 (パイプライン構成と経緯)

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

#### 構造体 ExtendedPresolveResult — 旧ドキュメント (逆順復元の理由、g/h と分離形式)

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

#### フィールド ExtendedPresolveResult::unbounded — 旧ドキュメント

`true` iff [`freevar::eliminate_free_variables`] found a free
variable that is genuinely unbounded — either no remaining
appearance anywhere (`A`'s rows or the real inequality rows) with a
nonzero objective coefficient, or exactly one inequality-row
appearance whose sign combination with that coefficient leaves it
unbounded on the objective-favored side (see that function's own
docs for both) — and `a`/`b`/`c`/`lb`/`ub`/etc. below must not be
trusted. Mutually exclusive with `infeasible` — presolve reports at
most one of the two.

#### フィールド ExtendedPresolveResult::postsolve_log — 1 本の時系列ログである理由 (greenbea のバグ)

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

#### 列挙型 PostsolveStep — 旧ドキュメント

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

#### 関数 run_extended — 旧ドキュメント (doubleton ラッチの経緯、colsingleton のスケール後への移動)

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

#### 関数 run_extended — ENOMOTO_PROF_PRESOLVE の導入理由

One-off, env-var-gated wall-clock breakdown of this function's own
major steps — `ENOMOTO_PROF_PHASES`'s `solve_lp_dual` timer starts
*after* this whole function returns, so it was blind to presolve's
own cost entirely; on several Netlib instances (`ganges`, `stocfor2`,
`sierra`) presolve turned out to be 75-92% of *total* solve time,
not the simplex loop `ENOMOTO_PROF_PHASES` already covers. A plain
local `Instant`/`eprintln!` here (not the atomics-based `timed!`
machinery `simplex.rs` uses) is enough since this function runs
once per solve, not once per pivot.

#### 関数 run_extended — reduce_equalities と ENOMOTO_REDEQ_MODE

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

#### 関数 run_extended — 未接続の手法 (parallelrows/rowdominance/…) の測定

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

#### 関数 run_extended — orig_lb/orig_ub の凍結

Frozen once, before any round's `propagate` call ever runs — see
`dualpropagate::run`'s own docs for why it needs the model's
*original* bounds specifically, not whatever `lb`/`ub` a later
round's own activity-based tightening has since narrowed them to.
(One `extract_bounds` of the pre-loop `G` serves both this and the
first round's `propagate`, see `carry` below.)

#### 関数 run_extended — doubleton_active ラッチ

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

#### 関数 run_extended — dualpropagate_active ラッチ

Same one-way latch, same reason: `dualpropagate`'s own transpose-and-
propagate call is a full-matrix pass, worth skipping once a round's
call finds neither a new implied-equality row to promote nor a new
column to fix (its two reductions — see `dualpropagate::run`'s docs).

#### 関数 run_extended — parallelcols_active ラッチ (2 ストライクの理由: ganges)

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

#### 関数 run_extended — 不動点判定 prev_signature

Fixpoint detection: a round that leaves `a`/`g`'s row counts and
every bound unchanged found nothing a further round could act on
either (every stage here is a deterministic, pure function of
exactly this state), so it's safe to stop before `rounds` even on a
round that runs the full stage sequence but accomplishes nothing —
this is what turns `rounds` from "run exactly this many times" into
"run at most this many times, fewer if convergence comes first".

#### 関数 run_extended — G の分離保持 (split_enabled)

Bounds kept apart from `G` (analysis/presolve_pipeline_20260924 C13):
whenever `G` would just be `rebuild_g_ref(cur_real_rows, cur_real_rhs,
lb, ub)` of a canonical split (`propagate::split_is_canonical`), the
CSR is not built — `g_split` is set, `g`/`h` are stale, and every
reader of `G` below takes the split form instead (`GView::Split`,
`reduce_inequality_rows`, `propagate_split`), which by construction
decides exactly what it would on the materialized matrix. Between
rounds the split travels in `carry`. `ENOMOTO_T_PRESOLVE_SPLIT_G=0`
always materializes (the previous behaviour, for A/B).

#### 関数 run_extended — cur_real_rows を内側ループで更新する理由

The inner loop below rebuilds `g` every pass (to fold in
`rowsingleton`'s freshest fixes) from *this*, kept up to date
after each pass via `extract_bounds` on the just-updated `g` —
not left as this round's own initial `propagate` snapshot, which
would silently discard every row rewrite doubleton/colsingleton
made in an earlier inner pass (see the inner loop's own docs).

#### 関数 run_extended — 等式行伝播 (eqprop) の位置と回数

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

#### 関数 run_extended — stuffing を接続しない理由

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

#### 関数 run_extended — dualpropagate の 2 つの縮小

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

#### 関数 run_extended — foldfixed を毎ラウンド実行する理由

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

#### 関数 run_extended — 内側ループと doubleton の扱い

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

#### 関数 run_extended — doubleton 消去変数の即時固定

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

#### 関数 run_extended — colsingleton 消去変数の箱制約行を落とす理由

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

#### 関数 run_extended — 新しい G の組み立て

New `G`: every row of the current one except eliminated
variables' single-entry rows, then `cs.extra_g_rows` —
i.e. `csr_from_rows` of those rows, assembled straight
into a `CsrRowBuilder` (from the split form when `G` is
held split: its real rows, then the bound rows of the
not-eliminated columns, from the bounds *before* the
pinning below). A row the builder rejects (a repeated
column) falls back to building that row list explicitly.

#### 関数 run_extended — 内側パス後の再分解 (extract_bounds(inner))

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

#### 関数 run_extended — Aggregator の配置・測定・行横断版の経緯

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

#### 関数 run_extended — Aggregator v2 を既定にした経緯

Default since 2026-09-23 (analysis/stocfor2_presolve_20260923.md):
stocfor2 1652x1766 -> 950x1072 after presolve, -65% solve time.

#### 関数 run_extended — ParallelColumns の配置・測定・既定オン化・greenbea 偽 Infeasible 修正

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

#### 関数 run_extended — 不等式行重複削除を毎ラウンド行う理由と測定

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

#### 関数 run_extended — ENOMOTO_T_ROUND_STRUCT_STOP

`ENOMOTO_T_ROUND_STRUCT_STOP=1` (default 0 = off): stop as soon as
a whole round left the structure unchanged — `A`'s row count,
`g`'s multi-entry (real) row count, the number of fixed columns
and the postsolve log length all equal to the previous round's.
Unlike the signature check below this ignores bound values, so a
round that only keeps shaving bounds (geometric convergence) ends
the loop after one such idle round instead of running to the cap.

#### 関数 run_extended — 不動点判定の相対許容 (scagr25)

Bound changes below a relative 1e-3 do not count as progress
(`ENOMOTO_FIXPOINT_EXACT` restores the exact comparison):
propagation over a cyclic row structure — which the aggregator's
folds can create (`scagr25`) — keeps shaving ever-smaller amounts
off a bound (geometric convergence), which an exact comparison
never calls a fixpoint, burning every remaining round. The
tightened bounds themselves are still kept.

#### 関数 run_extended — 遅延階数判定 (C1 option ii)

    if redeq_mode == 2 && a.nrows() > 0 {
Deferred rank detection (C1 option ii): the round loop has
removed most rows and columns by now, so the dense-QR / sparse-
elimination cost is paid on the reduced system only. Current
bounds feed only the block decomposition's negligible-edge filter
(see `reduce_equalities`' docs).

#### 関数 run_extended — 消去変数の [0,0] 固定

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

#### 関数 run_extended — 自由変数消去の位置

General free-variable elimination (paper §4.1): `rowsingleton`/
`doubleton`/`colsingleton` above only ever caught a free variable
appearing in exactly one or two `A` rows; this handles any remaining
one (any number of appearances, including none at all) — see
`freevar`'s own docs. Must run *after* the pinning loop just above,
not before: an already-eliminated column's own box row is long gone
by this point, so without pinning it to `[0,0]` first it would
misread here as a genuinely fresh free variable.

#### 関数 run_extended — freevar の unbounded 判定を拒否する理由

`freevar`'s "unbounded" only proves `z^1 < 0` (an improving ray
exists); it is a true unboundedness verdict only if the rest of
the problem is feasible, which presolve never checks. A caller
that must tell infeasible from unbounded refuses the verdict:
the step is skipped and the solver decides.

### src/params.rs (presolve)

#### 定数 SUBSTITUTION_PIVOT_RATIO

- 【元: aggregator.rs】 Mirrors `colsingleton`/`freevar`'s own pivot guard exactly (same value, same purpose).
- 【元: colsingleton.rs】 Minimum `|coeff| / max|row|` for a column singleton to be substituted out — see the guard in `eliminate_singleton_equalities`.
- 【元: freevar.rs】 Minimum `|coeff| / max|row|` for either pass to actually eliminate a free variable through a given row — mirrors `colsingleton::SUBSTITUTION_PIVOT_RATIO` exactly (same value, same purpose: a pivot that is tiny only *relative to its own row* still amplifies whatever floating-point error the row already carries when every other entry gets divided by it). Confirmed load-bearing, not merely defensive: two real Netlib instances (`perold`, `pilot4`, both already flagged elsewhere in this crate as numerically difficult) were pushed to a false `Infeasible` — reproducing identically through `extended_dual` *and* the classical `BIG_M` path, and only when this module's own elimination ran at all — by a handful of sub-1%-of-row pivots this module used to accept unconditionally, before this guard existed.

#### 定数 MAX_FILLIN

Mirrors HiGHS's own `presolve_substitution_maxfillin` default (registered range `[0, 10]`, default `10`, `HighsOptions.h`): total new nonzeros a single column's elimination may introduce across every row it folds into, above which the column is left for a later call instead of risking a dense-equality-system blowup.

#### 定数 MAX_CONSECUTIVE_FILLIN_FAILURES

Mirrors HiGHS's own `nfail == 3` cutoff in `HPresolve::aggregator`: after this many *consecutive* fill-in rejections, stop trying the rest of this call's candidate list outright rather than keep paying for the fill-in check on an already-too-dense region.

#### 定数 DENSE_DENSITY_THRESHOLD

Density, not `p` or a dense-QR flop-count estimate, is what actually separates the two regimes — calibrated directly against measured Netlib instances, not derived analytically. The first cut at this dispatch rule used a pure cost estimate (`(n+1) * p^2`, dense QR's own flop order, thresholded so `wood1p`'s small `p` routed to dense): it fixed `wood1p` but *also* routed `standmps` (`p=268`, density 0.96%) and `fffff800` (`p=350`, density 1.6%) to dense even though the sparse method was already faster for both there (their low density means elimination stays close to its own nonzero count, with none of the fill-in blowup a size-based estimate implicitly worries about) — measured regressions of roughly 2-4x on both after that first cut. `wood1p` itself is the outlier that actually needs dense: 11.1% row density, roughly 7-30x denser than every other measured instance (`fffff800` 1.6%, `standmps` 0.96%, `sierra` 0.37%, `ganges` 0.31%, `modszk1` 0.28%, `stocfor2` 0.21%) — dense QR there stayed a bounded ~30ms while the sparse method's fill-in blew up to 962ms. `3%` sits with comfortable margin above every instance that must stay sparse and below `wood1p`'s own density.

#### 定数 PIVOT_STABILITY

Numerical-stability floor a candidate pivot must clear, relative to the current **global** maximum active entry anywhere in the matrix (not just its own column's) — see `drop_linearly_dependent_sparse`'s own docs for why "global" here, not "local to the column", is what makes this a correct rank-revealing criterion instead of merely a safe-enough pivot for solving. A different, narrower purpose than `DEP_TOL`: this only gates which *candidates* the fill-minimizing search is allowed to accept, the same role `simplex::lu`'s own `STABILITY` constant plays for the (unrelated) basis factorization.

#### 定数 DEP_TOL

`DEP_TOL * row_orig_norm` plays the same role here that `1e-9 * col_norm(orig)` plays against `|R[k,k]|` in `drop_linearly_dependent` (see that function's own docs for why a per-row-relative, not global, threshold matters — the same reasoning applies here). The row norm is the row's own *original* norm (computed once, before any elimination).

#### 定数 PARALLEL_DECOMPOSE_ROW_THRESHOLD

Unlike every *other* `rayon` call site in this crate (all gated by `RAYON_SIZE_THRESHOLD`-style raw *problem* size, per `simplex.rs`'s own docs on measured per-element dispatch overhead), the right threshold here is total row count *within the blocks actually being split*, since a component's own elimination is real, non-trivial work per row (unlike a cheap per-element scan) — a modest absolute row count here still comfortably pays for `rayon`'s task dispatch.

#### 定数 MIN_ROWS_FOR_BLOCK_DECOMPOSE

Skipping avoids always paying for the bipartite-matching-plus-SCC pass (and, if it does find multiple blocks, the per-block `HashMap`-based column remapping and fresh `BTreeMap`/bucket/heap scaffolding for each one). Measured directly (with the earlier, coarser connected-components version of this same decomposition, before it was replaced by the finer Dulmage-Mendelsohn one — the size/regression picture below is unaffected by that swap, since both pay similar decomposition overhead on tiny inputs): every real Netlib win from decomposition (`ship12s` `p=1045`, `ship08s` `p=698`, `ship04l`/`ship04s` `p=354`, `sierra` `p=528`) has `p` well above this; every case that regressed when decomposition ran unconditionally (`sc105` `p=45`, `scorpion` `p=280`, `sc205` `p=91`, `capri` `p=142`, `standgub`/`standata` `p=160`, `recipe` `p=67`, `bore3d` `p=214`) sits below it — all by a comfortable margin, so `300` is not a tight cutoff. Every one of those regressions was itself only a fraction of a millisecond in absolute terms (these are already sub-10ms problems), but with nothing to gain there either — the decomposition's benefit scales with how much per-row elimination work it *avoids* doing across blocks, which is negligible when the whole problem is this small to begin with.

#### 定数 RAYON_SIZE_THRESHOLD

Same constant and rationale as `simplex.rs`'s `RAYON_SIZE_THRESHOLD` (this crate's own `#[ignore]`d `col_norm_fold_rayon_threshold_microbench` never found `rayon` winning, not even at 4,000,000 rows), duplicated locally rather than shared cross-module since each of this crate's rayon-threshold constants is already tuned/re-derived independently per call site (see e.g. `interior_point.rs`'s own separate `PROPAGATION_PASSES` copy for the same "each engine keeps its own tuning constant" convention).

#### 定数 SMALLCOEFF_EPS

Analogue of this solver's own primal feasibility tolerance (`simplex.rs`'s `PRIMAL_FEAS_TOL`) — the "eps" the cumulative budget is measured against, kept as this module's own copy rather than importing `simplex`'s (this pipeline is shared with `interior_point`, which has no reason to depend on `simplex`'s own module) since both represent the same underlying concept: how much primal infeasibility this solver is willing to call negligible.

#### 定数 CUMULATIVE_FRACTION

Achterberg et al.'s own `1e-1 * eps` (see the smallcoeff module docs for why this single, looser budget suffices on its own).

#### 定数 NOISE_THRESHOLD

Achterberg et al.'s own `1e-10`, floating-point noise on any realistically scaled problem.

### src/presolve/aggregator.rs

#### モジュール冒頭 (//!)

##### Why the first version of this module was reverted, and what changed

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

##### Cross-row aggregation (the default since 2026-09-22)

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

##### Default since 2026-09-23: `eliminate_implied_free_columns_v2`

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

##### Candidate order and fill-in

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

##### One non-cascading call; repetition is the caller's job

Like `colsingleton`'s own single pass, this computes implied bounds and
candidates once from an input snapshot; it does not loop internally to a
fixpoint. `presolve.rs`'s own round loop is what should call this
repeatedly (HiGHS itself calls its `aggregator` once per outer main-loop
iteration, right after its fast singleton/doubleton inner loop converges,
re-entering that inner loop whenever `aggregator` shrinks the problem —
`HPresolve.cpp:5901-5917`) so that a column exposed as implied-free only
*after* an earlier fold, or after `rowsingleton`/`colsingleton` tighten a
bound, gets caught on the next call rather than never.

##### Which rows justify / get folded (row-local & xrow versions)

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

#### 構造体 RowActivity

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

#### 関数 row_implies_own_bound

See the module docs for why this stays row-local rather than aggregating
across every row `j` appears in (a cross-row aggregate version was tried
and reverted after producing a false `Unbounded` on a real Netlib
instance — see `eliminate_implied_free_columns_xrow` for the root-caused,
fixed reattempt).

#### 関数 eliminate_implied_free_columns_if_any

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

#### 関数 eliminate_implied_free_columns_xrow

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

#### 構造体 AggOptions

Each generalisation is an independent countermeasure (see
`analysis/stocfor2_presolve_20260923.md`) and is kept separately
switchable (the `ENOMOTO_AGG_*` opt-outs in `AggOptions::from_env`) so
each can be A/B-measured on its own.

#### 関数 eliminate_implied_free_columns_v2 / eliminate_implied_free_columns_v2_if_any

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

#### テスト neither_row_alone_proving_it_leaves_the_column_un_eliminated

This module's check is deliberately row-local (see the module docs on
why a cross-row aggregate was tried and reverted), so even though a human
could intersect both rows' own true implications to a tighter [-95,-3]
(still not enough here, but illustrating the gap this design accepts),
this pass only ever asks a single row at a time and finds neither
sufficient -- x0 remains un-eliminated by this pass (though `A`'s
equality rows are equal to `x0`'s own true value regardless, so nothing
is *lost*, just not caught by this particular column-elimination
technique).

#### テスト non_implied_free_column_is_left_alone

(unlike the reverted first version, which would have eliminated it anyway
with an explicit bound row).

#### テスト xrow_rejects_a_justification_whose_shared_row_was_already_consumed

Regression test for the root-caused Netlib `shell` false-`Unbounded` bug
(see `eliminate_implied_free_columns_xrow`'s history above).

### src/presolve/redundancy.rs

#### モジュール冒頭 (//!)

Original module docs (design rationale / history portions):

- `reduce_inequalities`'s own duplicate-row pass over `(G, h)` is cheap enough (a single hash scan, no linear algebra) that `run_extended`'s own round loop calls it again at the end of every outer round, not just once here — in short: `doubleton`/`colsingleton` substitution can turn two originally-distinct inequality rows into duplicates only *after* this pre-loop call already ran.
- Rank-revealing elimination: both implementations (dense column-pivoted QR, sparse Gaussian elimination picking at each step the column carrying the most numerical weight and, within it, the largest-magnitude row as pivot, mirroring dense QR's own strategy) answer the identical question — a row whose residual after eliminating every previously-kept row's pivot is negligible relative to its own original norm is a linear combination of the others — just via different arithmetic paths, each cheap in the regime the other is expensive in.
- The Dulmage-Mendelsohn pre-pass: some real Netlib instances decompose into dozens of near-identical-size blocks this way (one per vessel/route/period in a multi-period scheduling LP), and instances that share no exploitable structure by plain column-disjointness alone still often decompose into hundreds of much smaller blocks once the matching's dependency structure is taken into account. Unlike using this same decomposition for LU factorization or solving, redundancy detection only ever asks a *local* per-row question ("is this row exactly some combination of these specific other rows?"), which holds unconditionally once verified — so every block is safe to check independently and in any order, including concurrently.
- **Parallelization**: extracting each row's coefficients out of the CSR `A`/`G` is independent per row, but runs sequentially rather than via rayon — profiling on this crate's target problem sizes found rayon's per-call dispatch overhead exceeding the cost of this simple scan (the same finding as `scaling.rs`'s and `simplex.rs`'s own per-iteration loops; see `simplex.rs`'s `solve_lp_dual_on` module docs). Step 1 (`dedupe_rows`) is a single sequential scan over a shared `HashSet` by design regardless — which duplicate of an equal pair survives depends on scan order, so parallelizing it would make that choice (immaterial to correctness) nondeterministic between runs.

#### 静的変数 PROF_TOTAL_STEPS / PROF_TRIVIAL_STEPS

Measurement counters answering "would a dedicated block-triangularization pre-pass (Dulmage-Mendelsohn / BTF, exposing structurally-forced 1x1 pivots before elimination starts, the way `simplex::lu`'s own `PROF_TOTAL_STEPS`/`PROF_TRIVIAL_STEPS` counters investigated for the basis LU) help `drop_linearly_dependent_sparse` the same way it was found *not* to help there" — see the `ENOMOTO_PROF_REDUNDANCY` diagnostic (`presolve.rs`'s `reduce_equalities` call site) for the answer. A step is "trivial" under the identical definition `simplex::lu::MarkowitzState::find_best_pivot` uses: the winning `(row_degree - 1) * (col_degree - 1)` Markowitz score is `0`, i.e. a structurally forced pivot a BTF pre-pass would also have found for free.

#### 関数 reduce_equalities

- Dispatch rule: see `DENSE_DENSITY_THRESHOLD`'s own docs for the rule and the real Netlib instances that motivated it.
- `lb`/`ub`: dropping an edge is safe regardless of how accurate `lb`/`ub` are (only costs recall, never soundness), so passing the model's raw, not-yet-propagated bounds — this runs before presolve's own bound-tightening rounds start — is fine: staler/wider bounds just make the negligibility test fire less often, i.e. a more conservative (coarser, never incorrect) split than the fully-tightened bounds would give.

#### 関数 dedupe_rows

Implementation note: same decisions as keying a `HashSet` on the normalized signature `[(j, bits(v/pivot))..., (usize::MAX, bits(rhs/pivot))]` (kept as `dedupe_rows_reference` for the equivalence test), but without materializing each signature as its own `Vec` and SipHash-ing it: the signature is hashed on the fly (the same multiplicative mix `reduce_inequalities` uses) and a hash hit is confirmed by recomputing the kept row's signature — normalization is deterministic — and comparing it entry by entry.

#### 関数 drop_linearly_dependent (dense QR)

- Why explicitly sequential QR: `ColPivQr::new` would read faer's *global* parallelism (`Rayon(0)` with the `rayon` feature on), which for the sizes seen here (at most a few thousand x a few hundred) buys nothing: on a fresh process it is what first spins up rayon's global pool (4 thread spawns + per-thread arenas, ~0.5 ms), every later call pays the hand-off to the pool, and the parallel reduction order made `wood1p`'s dropped-row set vary from run to run.
- Per-row relative rank test history: an earlier version used one global threshold, `1e-10 * max(n+1, p) * max_k |R[k,k]|` — on a problem with ~1600 columns and a large appended-rhs coordinate that came to ~1e-2 in absolute terms, and it dropped equality rows whose genuine independent component was of that size (confirmed on Netlib `modszk1`/`ganges`: the solves then ended at points *violating* the dropped rows by ~1e-2, with objectives "better" than the true optimum). A tiny absolute floor (`1e-300`) still catches exact zeros.

#### 関数 drop_linearly_dependent_sparse

- Why it exists alongside dense QR: dense QR's own docs used to assume "`p` is expected to be small relative to `n`" and had no fallback when several real Netlib instances violate that badly (`ganges`: `n=1681`, `p=1284` equality rows — almost every constraint is an equality), which paid for it directly — dense QR there measured at >99% of `ganges`'s *entire* presolve time, dwarfing every other technique in this pipeline combined. The sparse method's cost scales with actual nonzero fill rather than `n * p` — but that same fill-dependence is a liability on a matrix that isn't actually sparse (`wood1p`: only `p=243` but 11% row density, an order of magnitude denser than every other measured instance — fill-in during elimination blew up to 962ms there, while dense QR's cost bound doesn't care about density at all).
- Pivot selection history: candidates found via an ascending-Markowitz-degree bucket scan like `find_best_pivot`, but a candidate is only acceptable if its magnitude is within `PIVOT_STABILITY` of the current **global** maximum active entry — not, as an earlier version tried, relative only to its *own column's* current maximum. That earlier purely-local-threshold version is what `simplex::lu`'s own Markowitz factorization uses (appropriate for *solving*), but it does not work for *rank revelation*: a column's own max is trivially satisfied by its own max entry, so on real data (`ganges`) a low-degree column holding only small, easily-corrupted-by-cancellation entries got chosen as a pivot purely because it had few nonzeros, ahead of a column that was numerically dominant matrix-wide — three genuinely independent rows were misclassified as dependent (confirmed by direct comparison against the dense reference; raising the local threshold had *no* effect, proving the bug was about which column got selected, not how strong the pivot was). A purely global-norm-maximizing version (no degree preference, matching dense QR's strategy exactly) fixed that but gave up fill control entirely, causing severe fill-in blowups on several other real instances (`modszk1`, `standmps`, `wood1p`, `fffff800` all measured 3-10x slower). Gating the same degree-ascending search with a global acceptance threshold gets both: fill-minimizing order among numerically safe candidates, and a locally-large-but-globally-negligible candidate (the `ganges` failure mode) is skipped — worst case the column realizing the global max itself, which always trivially passes.
- Lazy heap maintenance: bit-identical to the eager version, which rescanned every touched column (the rhs column `n` and other long columns included) after every single step.
- `col_bits[j] < threshold` O(1) skip: same pivot choice, bit for bit; this is what keeps long runs of low-degree, tiny-valued columns (`dfl001`) from being rescanned on every single step.
- Retiring pivot row: `heap`/`col_bits` refresh is not done at the point the pivot row is removed from other columns, since the kept branch's own elimination pass touches the same columns' values again right after, and refreshing twice per step is a real, measured cost on matrices with high average column degree (`wood1p`: doing it unconditionally there as well as after elimination roughly doubled `reduce_equalities`' time).
- Dependent branch (historical, eager-heap era note): this branch never reached the post-elimination refresh pass, so `pi_cols` had to be refreshed there — otherwise a stale, too-high entry could survive in `heap` for a column whose recorded max came only from `pi`, wrongly gating out a genuinely valid pivot elsewhere via an inflated `gmax` on a later step. With the lazy heap, `pi_cols`' possible max decrease is already recorded via `note_change` beforehand.
- After elimination (historical): with the lazy heap every value change is recorded via `note_change`; no per-column rescan is needed there any more.

#### 関数 dulmage_mendelsohn_blocks

- Strictly finer than a plain connected-components partition — real Netlib instances that show as a single connected component by raw column-sharing alone (`shell`, `scsd8`, `fit1p`, `ganges`) decompose into hundreds of much smaller SCCs this way (`shell`: 531 blocks from 534 rows; `ganges`: 998 from 1284), most of them singletons.
- Soundness vs LU: LU needs blocks processed in dependency order because it *propagates computed values* forward (well, needs it for *solving* — see `crate::graph::dulmage_mendelsohn_blocks`'s own docs on why even LU *factorization* itself turns out not to need that ordering, since off-diagonal spillover entries are carried through unchanged). Redundancy detection asks a purely local question per row; once verified using the rows' full, untruncated content, it holds unconditionally. The only cost of independence is recall: a redundancy whose witness spans multiple blocks (possible here since an earlier block's row can reach into a later block's columns) goes undetected and that row is conservatively kept — never the reverse.
- Small-coefficient edge filter history: `smallcoeff`'s own module docs record two prior attempts at wiring its reduction into the live pipeline, each reverted after it numerically destabilized a real instance (`perold` newly crashing, `beale_cycling_example_terminates_correctly` newly failing its IPM cross-check) — both traced to the reduction *mutating* a row/rhs a later stage then solved against. Using the same negligibility test only to decide which edges feed a graph algorithm carries none of that risk.
- **Measured** (instrumented directly) against real Netlib instances: the filter is far from a no-op on some — `shell` drops 500 of 3550 structural edges (14%), `25fv47` 72 of 3609, `sierra` 40 of 3973 — but the block *count* barely moves (`shell` 529 -> 524, `sierra` 438 -> 438, `scfxm3` 326 -> 329, `25fv47` 247 -> 241): most negligible coefficients sit inside a block the matching would have kept together anyway. `25fv47` landing on *fewer* blocks is not a soundness concern — a maximum bipartite matching is generally non-unique, so removing an edge can steer the matcher to a different one with its own SCC condensation; not monotonic in count. A full-Netlib wall-clock A/B (73 in-scope instances, 3 repeats each side) showed no aggregate difference distinguishable from run-to-run noise (both sides in the same ~4.1-5.1s band).

#### 関数 drop_linearly_dependent_sparse_blocked

- Real Netlib multi-vessel/multi-period scheduling LPs (`ship12s`, `ship08s`, `ship04l`, `ship04s`, `sierra`) decompose into dozens of blocks of nearly identical size (`ship12s`: 12 blocks of exactly 78 rows each, plus 109 size-1 singletons) — one per vessel/route/period. A first version used plain connected components (disjoint column support only) and stopped there; it found those but left `shell`, `scsd8`, `fit1p`, `ganges` as a single fully-coupled component. Replacing it with the full Dulmage-Mendelsohn matching-plus-SCC decomposition finds much finer structure there too (`shell`: 531 blocks from 534 rows; `ganges`: 998 from 1284; `fit1p`: 605 from 627) — the matching exploits a *directional* dependency structure pure column-disjointness can never see. `wood1p` stays on the dense path entirely and is unaffected.
- Column remapping to `0..local_n`: without it every block's call would still pay for `aug_n`-sized scratch arrays (`col_rows`, `col_buckets`, `heap`/`col_bits`) proportional to the whole problem's `n`, defeating the point of splitting.
- rhs-augmentation edge case: `dulmage_mendelsohn_blocks` deliberately does not treat the rhs column as a graph edge, so two originally all-zero-coefficient rows with different nonzero rhs (`0 = 5`, `0 = 3`) land in separate singleton blocks, whereas the un-decomposed algorithm's shared rhs column would link them and drop one. Both outcomes are correct (each is its own infeasibility witness); this function is just more conservative. A row reducing to a pure rhs residual during elimination (general Farkas case) is still caught entirely locally within one block.
- LPT ordering: the biggest components are hardest to load-balance, so dispatching them first gives rayon's work-stealing scheduler the best chance of not stranding two large blocks on the same thread.

#### 関数 reduce_inequalities

- Implementation note: same decisions as the straightforward version (`reduce_inequalities_reference`), but without materializing every row and normalized signature as its own `Vec` and SipHash-ing it: rows read straight from CSR slices, signature hashed on the fly with a cheap multiplicative mix, hash hit confirmed by recomputing the class representative's signature (bit-for-bit the reference's key, since normalization is deterministic).
- No inequality analogue of step 2 rank-revealing QR: a positive combination of several `<=` rows can imply another one, but detecting that in general is Fourier-Motzkin elimination, well beyond a cheap presolve pass.

#### テスト dense_density_threshold_routes_known_instances_correctly

`standmps` and `fffff800` are the cases that specifically ruled out a pure size-based cost estimate in an earlier version of the dispatch rule — both have modest `p` but low density and must stay on the sparse path. Shapes: wood1p p=243 n=2594 nnz=70214 (11.1%); standmps p=268 n=1075 nnz=2776 (0.96%); fffff800 p=350 n=854 nnz=4775 (1.6%); ganges p=1284 n=1681 nnz=6612 (0.31%).

#### テスト dulmage_mendelsohn_blocks_splits_a_pure_chain_into_singletons

Mirrors the structure real instances like `fit1p`/`ganges` showed (hundreds of singleton SCCs) despite looking like one fully-coupled component by column-sharing alone.

### src/presolve/propagate.rs

#### モジュール冒頭 (//!)

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

#### 構造体 PropagateResult

- `#[allow(dead_code)] // the pipeline uses PropagateSplit; kept for tests / G-form callers`
- `lb`/`ub`: The same bounds already folded into `g`/`h` as single-variable
  rows, pulled back out — see the module docs for why this saves
  callers like `simplex.rs` a redundant `extract_bounds` call.
- `real_rows`: The final surviving multi-variable rows, *not* re-folded with the
  bound rows the way `g`/`h` are — i.e. `g`/`h` minus its
  single-variable rows, equivalently `extract_bounds(n, &g, &h)`'s
  3rd/4th return values, computed once here instead of twice.

#### 関数 bounds_inconsistent

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

#### 構造体 PropagateSplit

`propagate`'s outcome without the re-folded `g`/`h` — what every
caller inside the presolve pipeline actually reads (`run_extended`
works on the split `lb`/`ub`/real-row form, `dualpropagate` only reads
the propagated box), so building the CSR there was pure overhead.

#### 関数 propagate_split

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

#### 列挙型 GView

`G x <= h` as a pass reads it: either a materialized CSR, or the split
form `(rows, rhs, lb, ub)` standing for exactly
`rebuild_g_ref(rows, rhs, lb, ub)` — only used when
`split_is_canonical` holds, so that `extract_bounds` of that matrix
would hand back `lb`/`ub`/`rows`/`rhs` themselves, bit for bit, and a
pass can read the split form directly instead of building the CSR (the
"bounds as rows" round trip, analysis/presolve_pipeline_20260924 C13).

#### 関数 rebuild_g_ref / rebuild_g

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

#### 構造体 EqPropagateResult / 関数 propagate_equalities

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

#### テスト

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

### src/presolve/freevar.rs

#### モジュール冒頭 (//!)

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

##### Leftover free variables

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

#### 構造体 FreeVarResult

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

#### 関数 eliminate_free_variables

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

#### テスト

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

#### 関連 (params.rs 側に既に記載)

`SUBSTITUTION_PIVOT_RATIO` の経緯 (perold/pilot4 の false Infeasible) は params.rs の doc に残っている。

---

### src/presolve/doubleton.rs

#### モジュール冒頭 (//!)

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

#### 構造体 DoubletonResult

- `unchanged`: Set by the no-candidate fast path: `a`/`b`/`g`/`h`/`c` are exact
  copies of the inputs (see [`unchanged_if_no_candidate`] — 注: 存在しないリンクだった。
  実体は `no_candidate`、リファクタで `inputs_pass_through_unchanged` に改名)。
  `#[allow(dead_code)] // read by tests; eliminate_doubleton_equalities_view returns None instead`

#### 関数 no_candidate (→ inputs_pass_through_unchanged に改名)

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

#### 関数 rewrite_row

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

#### 関数 eliminate_doubleton_equalities

One non-cascading pass: candidate doubleton rows and which variable
each eliminates are all decided from `a`'s *input* shape — a variable
only claimed as "already eliminated" within this same pass (so two
doubleton rows never both try to eliminate it), not re-checked after
rewriting (mirrors `colsingleton`'s own single-pass scope).
`#[allow(dead_code)] // run_extended calls the GView form directly`

#### 関数 eliminate_doubleton_equalities_full

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

#### テスト

- `no_candidate_fast_path_matches_full_pass`: The no-candidate fast path must return exactly what the full pass
  would (bit for bit), whenever it fires.

### src/presolve/colsingleton.rs

#### モジュール冒頭 (//!)

- Bound-preservation rows were not part of the first version: "Skipping this (the first version of this module did) silently drops the eliminated variable's bounds: e.g. `x0 + x1 = 5` with only `x0` eliminated left `x1` completely unconstrained above, since removing the row was the *only* place `x1`'s upper reach had been limited."
- Omitting the vacuous side for genuinely infinite `lb_j`/`ub_j`: the classic "free column singleton" case then costs *zero* replacement rows, "letting a chain of these cascade through a network-shaped equality system the same way HiGHS's own presolve does."

#### 関数 skip_implied_bound_rows

- Default on since 2026-09-23 (`analysis/stocfor2_presolve_20260923.md`); `ENOMOTO_KEEP_IMPLIED_BOUND_ROWS` turns it off.

#### 関数 eliminate_singleton_equalities_view

- G reading: originally used `extract_bounds` (which copies every real row into its own `Vec`); now only `lb`/`ub` and the per-column appearance counts of G's real rows are needed, so G is read in place. The G-row exclusion is the same one `dualfix` applies.
- Pivot guard (`SUBSTITUTION_PIVOT_RATIO`): PaPILO applies the same kind of relative threshold before any substitution. Without it, a `coeff` tiny *relative to its own row* amplifies the reduced problem's residual (`~1e-7`) by `max|row| / |coeff|` when `x_j` is recovered — measured on Netlib `modszk1` as ~1e-6 violations of the eliminated rows and an objective slightly *below* the true optimum.
- Two singleton columns can share one row (e.g. `x5 + x7 = 3` with neither appearing anywhere else); only one is solved for, the other stays a real variable referenced as a term.
- Implied-side skip: `propagate` would drop such a row next round anyway.

### src/presolve/scaling.rs

#### モジュール冒頭 (//!) — 並列化の経緯

**Parallelization**: every per-row/per-column loop in this module (`compute`'s column-norm accumulation and row/column normalization, `apply`'s rescaling, `unscale_x`) is embarrassingly parallel in principle, but `apply`/`unscale_x` run sequentially unconditionally — profiling on this crate's target problem sizes (~1000 columns, a couple thousand rows across `A`/`G`) found rayon's per-call dispatch overhead exceeding the arithmetic itself there, the same finding as `simplex.rs`'s per-pivot loops (see `solve_lp_dual_on`'s module docs). `compute`'s own column-norm fold — by far the largest single cost in this crate's presolve pipeline at that same target size (measured at ~38% of total presolve time before this file's sequential rewrite) — picks sequential vs. `rayon` once per call from `RAYON_SIZE_THRESHOLD`, replacing an earlier run-both-and-time self-calibration: this crate's own microbenchmark never found `rayon` beating a plain sequential fold at any size tried, up to 4,000,000 rows, so the live race was pure overhead for a decision with a fixed, always-the-same answer at every problem size this crate has ever actually measured.

#### 関数 col_norm_fold_parallel

- Doc used to say: "Only ever invoked by `compute`'s own self-calibration, never on its own, so it always starts from an all-zero `acc` in practice; written to merge into whatever `acc` already holds anyway; matching `col_norm_fold`'s own contract exactly." (The self-calibration has since been replaced by the `RAYON_SIZE_THRESHOLD` switch.)
- `with_min_len`: an unbounded split (rayon's default for a plain range with no length hint keeps splitting under work-stealing, not just once per thread) makes the O(n) merge cost dominate at just a few thousand rows — measured directly making this ~500x slower than sequential at 200,000 rows before `with_min_len` was added.

#### 関数 compute_impl — ENOMOTO_T_SCALE_NOBOUNDS 経路

- `ENOMOTO_T_SCALE_NOBOUNDS=1` (default 0 = off, the historical behaviour): leave `g`'s single-entry rows (the box-bound rows `build_a_g` adds per finite bound — on e.g. `fit2d` 99% of `g`) out of the Ruiz iteration entirely, so they neither pull on `d` nor get scanned every iteration, and give each one the closed-form row scale `1/|v * d_j|` afterwards (a bound row's own Ruiz fixed point: its scaled coefficient becomes +-1, i.e. the plain bound on `x'_j`).

#### 関数 compute_impl — 単一要素行の高速経路

- Bound-row pairs (`(j, 1.0)`, `(j, -1.0)`: a variable's `ub` row followed by its `lb` row) share one scale since both start at 1 and every update reads only `|v * d_j * e|`, so their factors are equal bit for bit at every iteration.

## 前処理 小規模モジュール群 (src/presolve/*.rs)
`src/presolve/` 配下の小規模モジュール (dualfix, dualpropagate, foldfixed, ineqsingleton,
parallelcols, parallelrows, rowdominance, rowsingleton, smallcoeff, sparsify, stuffing,
dominatedcol) のソースコード中にあった、開発経緯・計測結果・試行錯誤・無効化理由などの
長いコメントをここに集約したもの。コード側には簡潔な日本語ドキュメントのみを残している。
原文は英語のまま (翻訳していない)。

### src/presolve/dualfix.rs

#### モジュール全体 (原文ドキュメント冒頭)

DualFix (Achterberg, Bixby, Gu, Rothberg, Weninger, "Presolve Reductions in Mixed Integer
Programming", §4.4): a variable whose objective cost prefers one direction can be fixed to the
corresponding bound outright — no simplex iteration needed — provided *no* real constraint would
resist moving it that way. "Real" excludes the variable's own box-bound rows (folded into `G` by
`build_a_g`): those aren't independent constraints, they're the bounds themselves, so counting
them would make every variable look locked in both directions by its own bounds and this
reduction would never fire.

(以降の原文はアルゴリズム説明のみで、コード側の日本語ドキュメントに要約済み。履歴的記述なし。)

### src/presolve/rowsingleton.rs

履歴的記述なし (アルゴリズム説明のみで、コード側の日本語ドキュメントに要約済み)。

### src/presolve/foldfixed.rs

#### モジュール全体 (動機の原文)

Left undone, a row that starts with (say) 16 terms and has 14 of them fixed away elsewhere still
*looks* like a 16-variable row to `rowsingleton` (wants exactly 1 live term), `doubleton` (wants
exactly 2), and `aggregator`'s implied-free gate — none of which can fire on it until something
drops those 14 dead terms and shrinks it down to the 2 genuinely live ones.

Run once per outer round, right after whichever passes did the fixing for that round (mirrors
this crate's own "single pass, caller repeats via the round loop" idiom used throughout this
pipeline — a column fixed by *this* round's own `rowsingleton` is picked up by *next* round's
call, not re-scanned within this same one, since `ROWSINGLETON_COLSINGLETON_INNER_ROUNDS`
defaults to a single inner pass anyway).

### src/presolve/ineqsingleton.rs

#### モジュール全体「Ranged rows are one row here」の計測根拠

`G` is all `<=` rows, so a ranged row `L <= r.x <= U` lives there as the pair `r.x <= U`,
`-r.x <= -L` — and every bound-preservation row pair `colsingleton`/`doubleton` emit has exactly
that shape. Counted naively a column appearing only in such a row has *two* appearances and looks
like no singleton at all (Netlib `seba`: 86 of its 121 surviving columns were exactly this).

#### 等式化 (Tight row) の正当性の原文

Tight row when side `S` alone already implies `x_j`'s bound `t` (finite implied value, `t` on the
far side of it): then every optimum has `S` holding with equality — if `S` were slack, `x_j` could
move toward `t` (it is strictly short of `t`, or at `t` which forces `S` tight by the implication)
and strictly improve the objective. The row becomes the equality `a x_j + r = S`, dropping its
other side (implied by the equality), and `colsingleton` then substitutes `x_j` out, skipping the
now-redundant bound-preservation row for `t`.

### src/presolve/dualpropagate.rs

#### モジュール全体「Why this exists」(原文)

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

#### 「Infinite means literally +/-inf」— 厳密内側判定を撤回した経緯 (モジュール docs と `run` 内コメントの原文を統合)

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

#### 「Column fixing」節の動機 (原文)

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

#### `run` 内の補足 (原文)

- 双対系が実行不能の場合: A genuinely infeasible dual system here would mean the primal is
  unbounded or infeasible outright — a real finding, but too strong a conclusion to act on from
  this single, partial (box-bound-driven) slice of the full dual system alone.
- 列固定の下限条件: needs `lb_j` finite: an infinite one can never be "fixed" to, and by the same
  argument this module's row-promotion half already relies on, a genuinely infinite `lb_j` would
  instead have contributed its *own* `r_j <= 0` constraint above, making `rlo > TOL` here
  self-contradictory in practice.
- `find_implied_equalities` は `DualReductions` が存在する前に書かれたテストのために残されたラッパー。

### src/presolve/parallelcols.rs

#### モジュール全体「Scope」の根拠 (原文)

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

#### 既定有効化と計測の経緯 (原文)

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

#### 候補探索: 1 呼び出しで群全体を吸収する理由 (原文)

Unlike `parallelrows` (one merge per call, full stop — repeated calls across this pipeline's own
outer-round fixpoint loop pick up whatever a single call leaves behind), a single call here can
absorb an entire group at once. That is deliberate, not merely an optimization: a real Netlib
instance in this crate's own benchmark set (`standgub`) has one 108-column parallel group (a GUB
block of genuinely interchangeable decision variables), and capping this module at one merge per
call would need as many outer rounds as a group has members to fully collapse it — far more than
`run_extended`'s own round cap ever runs.

#### `Substitution::apply` の正当性 (原文)

Clamping to whichever end of `[lb, ub]` `raw` overshoots is provably still feasible for `kept`
(worked out in full, both signs of `s`, in this module's own commit/test history —
`merges_and_recovers_*` tests exercise every case).

#### `merge_parallel_columns_if_any` 内: 列ストリーム化と署名ハッシュ化の経緯 (原文)

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

#### 併合ループ: greenbea の偽 Infeasible と `lo == -inf` の併合拒否 (原文)

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

#### 改名

- `Substitution::lb`/`ub` → `var_lb`/`var_ub` (消去列 `var` の境界であることを明示)
- ローカル `group_keys` → `groups_by_first`

### src/presolve/parallelrows.rs

#### モジュール全体: 対象とする形状と正規化方法の詳細 (原文)

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

#### 未統合 (無効) にした経緯 (原文)

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

#### 関数内コメント (原文)

- 決定的な走査順: Deterministic order: sort groups' own keys isn't needed for correctness (every
  group is independent), but iterating a `HashMap` directly would make which-pair-merges-first
  nondeterministic across runs when a group has more than 2 members — harmless for correctness
  (every valid pairing here is equally valid) but still worth pinning down for reproducible
  benchmarking.
- 同符号の重複: already handled upstream by `redundancy::reduce_inequalities` (and if it somehow
  wasn't — e.g. this function called standalone in a test — merging it here too would need the
  same keep-tighter logic that function already implements; skip rather than duplicate that logic).

### src/presolve/rowdominance.rs

#### モジュール全体: 位置付けの原文

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

#### 未統合 (無効) にした経緯 (原文)

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

### src/presolve/smallcoeff.rs

#### モジュール全体: 論文の 2 段階判定を単一の累積予算にまとめた根拠 (原文)

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

#### 未統合 (行列を書き換える版) にした経緯 (原文)

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

#### 3 つ目の (非破壊的な) 配線 — 現在有効 (原文)

**A third wiring, non-destructive this time, is live**: [`clean_row`] (not
[`remove_small_coefficients`] — the model's `A`/`b` are never touched) feeds
`redundancy::dulmage_mendelsohn_blocks`'s own block-decomposition pre-pass, deciding which
structural edges a negligible coefficient should be left out of when building that pre-pass's
graph. Both prior failures above trace to *mutating* a row/rhs a later stage then solved against;
using the identical negligibility test only to drop a graph edge carries none of that risk — see
`dulmage_mendelsohn_blocks`'s own docs for why dropping an edge there only costs decomposition
recall, never soundness.

### src/presolve/sparsify.rs

#### モジュール全体: 設計判断の原文

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

#### 「書き換え済みの行をピボットにしない」— scorpion での系破損 (原文)

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

#### 未統合 (無効) にした経緯 (原文)

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

#### 関数内コメント (原文)

- `SparseAccum`: One sparse accumulator for every target-row rewrite — see
  `crate::sparse::SparseAccum`'s own docs for why the merge is not a per-target `BTreeMap`.
- 係数の打ち消し: Only `elim_var` is *proven* to cancel exactly (`scale` was chosen specifically
  to zero it) — any other entry's subtraction result, however small, is the mathematically correct
  new coefficient, not noise, and must be kept as-is rather than dropped by some absolute tolerance:
  a target row's other shared coefficient can land close to (but not at) zero by sheer coincidence
  without being a true cancellation, and silently discarding it would corrupt the row.

#### 改名

- 非公開 enum `Src` → `RowSource`

### src/presolve/stuffing.rs

(未統合にした経緯・計測値は `src/presolve.rs` の `stuffing::fix_singleton_columns` 呼び出し予定箇所の
コメントにある (そのファイルは本メモの担当外)。要旨: 73 Netlib 問題で発火 0 件、計測差はノイズ範囲内。)

#### モジュール全体「Relationship to `dualfix`」(原文)

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

#### Algorithm 1 の各分岐の説明 (原文)

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

#### 鏡像の場合を変数変換で帰着させた理由 (原文)

Rather than re-deriving a second, easily-miscrossed set of `Ũ`/`L̃` formulas and branch conditions
from scratch, this module reduces the mirror case to the one above by the substitution
`y_j = ub_j - x_j` applied to every such column in the row at once. Running the identical
[`stuffing_core`] on this transformed row and translating its results back is a pure change of
variables — no separate derivation to get subtly wrong, and no second implementation to keep in
sync with the first if either is ever revisited.

#### 対象外とした範囲の理由 (原文)

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

### src/presolve/dominatedcol.rs

#### モジュール全体: 動機と設計判断の原文

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

#### 未統合 (無効) にした経緯 (原文)

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

### パラメータについて

上記 12 ファイルには、`params::presolve` に既にある定数 (`TOL`, `SMALLCOEFF_EPS`,
`CUMULATIVE_FRACTION`, `NOISE_THRESHOLD`) 以外の調整用数値リテラルはなかった
(残っているのは 0.0 / 1.0 / 符号 / ハッシュ用乗数などの構造的な定数のみ)。そのため
`src/params.rs` への新規定数の追加はない。

## その他のモジュール (sparse / graph / interior_point / mip / model / solver / types / lib / Python パッケージ)
コード中のコメントから切り出した開発経緯・計測値・試行錯誤の記録。原文 (英語) を
ほぼそのまま残し、重複のみ軽く整理した。

---

### src/lib.rs

#### クレート構成 (旧モジュールドキュメントの全文)

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

#### `env_str!` マクロを作った理由

> `std::env::var` costs an environment lock + a linear `environ` scan + a
> `String` allocation, and the solve path consults ~100 `ENOMOTO_*` flags per LP
> (callgrind: ~10% of `afiro`'s instructions).

#### `SplitAlloc` (mimalloc / システムアロケータの振り分け) の計測経緯

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

### src/sparse.rs

#### モジュール全体: 一箇所に集約した経緯

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

#### `SparseVec` を薄い型にした理由

> A newtype that hid the representation would force a conversion at every one of
> those boundaries; what this type adds instead is the *operations* that were
> previously re-implemented per call site — scatter/gather against a dense
> buffer, a dot product against a dense vector, pruning, and the density check
> that decides sparse-vs-dense dispatch.

#### `scatter_dense`

> kept here so the several call sites that used to open-code a zero-fill plus a
> scatter share one implementation.

#### `HybridVec` (Forrest-Tomlin eta 用の疎/密ハイブリッドベクトル) の導入経緯

> Each consumer of an eta is one of exactly two loops, a dot product against a
> dense vector ([`Self::dot_dense`]) or an axpy into one
> ([`Self::axpy_into_dense`]), and before this type existed each of those was
> written out per call site as a two-armed `match` on the representation — eight
> copies across FTRAN, BTRAN and both capture variants, every one of which had
> to be edited in lockstep to change anything. They are these two methods now.

#### `HybridVec::pack_scaled_dense` の経緯

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

#### `HybridVec::for_each_index` がクロージャを取る理由

> The `Box<dyn Iterator>` this replaces bought that with a heap allocation per
> update plus a virtual call per index, on a path that runs every simplex
> iteration; a generic closure monomorphizes into each arm instead, so both
> loops inline and nothing is allocated. The dense arm still scans the whole
> array (it has no index list to walk), exactly as before.

#### `EpochMarks` の導入経緯 (3 つの独立実装の統合)

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

#### `SparseAccum` (疎アキュムレータ) の導入経緯

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

#### `axpy_row`

> This is the row-elimination kernel `presolve`'s substitution passes
> (`freevar`, `aggregator`) each used to carry their own `BTreeMap` copy of.

#### `csr_to_csc`

> the replacement for the "walk every row, `push` onto `columns[j]`" loops the
> presolve passes that need per-column access used to each write for
> themselves.

#### `csr_row_iter`

> The one place the `col_indices_of_row(i).zip(values_of_row(i))` dance is
> written. It used to be open-coded at ~40 call sites across `presolve/*`, which
> is exactly the kind of duplication that lets two of them quietly disagree
> about whether to filter zeros.

#### `CsrRowBuilder::finish` で faer の検証を省いた理由

> faer's `new_checked` re-validation (a second pass over every column index; ~4%
> of a small LP's `solve()` under callgrind) would only re-prove this.

#### `CscMat::from_entry_stream`

> Written directly, that is `vec![Vec::new(); n_cols]` plus a `push` per entry —
> one heap allocation per column, each then grown by reallocation — for a
> structure that is read-only the moment it is finished. Streamed through here
> it is the usual two allocations and one counting sort, with no intermediate
> concatenation of the blocks either.

---

### src/graph.rs

#### `max_bipartite_matching` を反復版 DFS にした経緯 (degen3 のスタックオーバーフロー)

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

#### `dulmage_mendelsohn_blocks_topological` が未使用の理由

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

### src/interior_point.rs / interior_point/{qp,kkt}.rs

#### 内点法エンジンの位置付けの変遷

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

#### `qp.rs`: 変数のシフト/分割をしない理由

> no shift/split preprocessing (that was only ever needed by the simplex
> method's ">= 0" requirement; the interior point method below handles
> arbitrary bounds natively).

#### `kkt.rs`: `PARALLELISM` (faer の rayon 並列) を残した経緯

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

#### `params::interior_point` の presolve 系定数の由来

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

### src/mip.rs

> Simple depth-first branch-and-bound ... Each node re-solves the LP relaxation
> with tightened variable bounds (no warm start) — simple and correct, adequate
> for the problem sizes this MVP targets.

---

### src/model.rs

#### 無限大の変数境界を受け付けるようにした経緯 (旧 BIG_M 不変条件)

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

### src/solver.rs

> `Simplex` (`simplex::solve_lp_dual`, the bounded-variable dual revised
> simplex) is the default, having replaced the original IP-PMM (PIQP-style
> interior point) path as the primary engine; `Interior`
> (`interior_point::solve_lp`) is kept fully reachable rather than deleted, both
> as a fallback and so the two independent implementations can be run against
> the same input and compared directly.

---

### src/types.rs

#### `LinearExpr` が `BTreeMap` を使う理由 (degen3 の実行時間二峰性)

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

#### `RootSolver` で両エンジンを残している理由

> keeping both reachable, rather than deleting the interior-point path once
> `simplex` became the default, is what makes an apples-to-apples comparison
> between them possible on the exact same problem.

---

### python/enomoto_solver/benchmark_highs.py

#### 無限大境界をそのまま渡すようになった経緯

> `model.rs::add_variable` now accepts genuine `+/-inf` bounds directly (see its
> own docs) — this crate's presolve pipeline substitutes a finite `BIG_M`
> internally only for whatever survives presolve without being eliminated
> outright (`simplex.rs::build_std_form_presolved`), not before presolve ever
> runs. So every Netlib problem here is handed to this crate exactly as HiGHS
> itself reads it from the `.mps` file, MPS `+inf` bounds included — no bound
> substitution happens in this script at all anymore.

#### 問題ごとにサブプロセスで解く理由

> this crate's simplex engine `panic!`s (rather than returning an error) on a
> handful of known-hard Netlib instances (e.g. `cycle`, named for exactly the
> degenerate-pivoting behavior it stresses) when a refactorization hits a
> numerically singular basis — a Rust panic crossing the PyO3 boundary aborts
> the whole interpreter, which would otherwise take the entire batch down with
> one bad problem. A subprocess also gives `--timeout` real teeth (a hung/slow
> solve is simply killed), which an in-process call has no way to do.

#### `_build_our_model` の二乗オーダー解消 (fbcec94)

> highspy copies the whole vector on every attribute access, so read each one
> exactly once — per-element `lp.col_lower_[j]` is O(n^2).

## stormG2_1000 対応: 反復あたりの `O(m)` パス削減 (2026-09-25)

分析 `analysis/stormG2_1000_20260925_055112.md` の §4 の改善策を実装した記録。stormG2_1000 (presolve 後
m ≈ 378K 行) は反復数が HiGHS と同等なのに 1 反復 8.8 ms (HiGHS 0.1 ms) で、原因は 1 反復あたり
15〜16 本の長さ `m` の密ベクトル `fill`/`copy`/全走査だった。策番号は同報告のもの。

### 策1・策5・策6: 融合 FTRAN の出力を非ゼロ位置の記録付きに (src/simplex/lu.rs `NzTrack` / `FtranTrack`)

- 入る列と DSE `tau` の融合 FTRAN (`solve_sparse_into_pair_capture` 等) が超疎 `U` 段 (C5) で解けたとき、
  出力 `alpha_full`/`tau` は `permute_list` が `out.fill(0.0)` してから一覧の位置を書いていた。
  `NzTrack` で前回書いた位置を覚え、そこだけ 0 に戻してから書く。記録の外で全体を書いた場合
  (密経路・非融合経路・DSE の単独 FTRAN) は `set_full` で無効化し、次の疎書き出しが 1 回だけ全体を戻す。
- `a_tilde` (FT 更新用の `L`/`R` 適用後の値) の `copy_from_slice` を、Gilbert-Peierls の到達集合 ∪ 値が非ゼロの
  `R` eta の行だけの書き込みに (策6)。
- 入る列側スクラッチの返却前 `fill(0.0)` と、`tau` 側スクラッチの `l_solve_steps_into` 入口の `fill(0.0)` を、
  超疎 `U` 段の到達スロット (`u_list`) だけの 0 戻しに。`U` 段で全 0 と分かっているかを `FtranTrack` が持つ。
  値 0 の位置はすべて `+0.0` のまま (`+0.0 - x` が `-0.0` になることはない) なので、スクラッチの状態は従来と同一。
- 両ベクトルとも超疎 `U` 段なら、何もしない `u_seq` 全 eta の走査自体を省く。
- 主ループの `x_B` 更新・DSE 更新の行一覧 `xb_rows` を `compact_rows` (`O(m/8)` のブロック走査) ではなく
  記録した位置から作る (値が非ゼロの行を集めて昇順ソート。同じ行・同じ順序) (策5)。
- `ENOMOTO_SPARSE_FTRAN_OUT=0` で従来の全体書き出し (A/B 用)。

### 策2・策5: ピボット行 BTRAN の超疎化 (src/simplex/lu.rs `UnitBtranWork` / `solve_transpose_unit_work`)

- `U^T` 掃引 (`u_transpose_sweep_track`) は `U^T` 順の全スロット (m 個) を `z[p] == 0.0` 判定しながら走査していた。
  非ゼロになった位置をキー (`U^T` 順の位置 = シングルトンの並び → `u_seq` の位置) の最小ヒープに積み、小さい順に
  処理する (`u_transpose_sweep_heap`)。スロット `p` に書く eta はすべて `p` よりキーが小さいので、取り出した時点で
  `z[p]` は確定しており、全走査と同じスロットを同じ順に処理する (値・CLOCK tick ともビット一致)。
- `R` eta を逆適用した位置も一覧に加え、`L^{-T}` 段 (スキャッタ形式) は非ゼロステップの最大ヒープで降順に処理する
  (`l_transpose_hyper_out`)。スキャッタ/ギャザーの選択は従来と同じ判定 (入力の非ゼロ数 ≤ `BTRAN_L_SCATTER_FRACTION * m`)。
- 出力置換 (`permute_btran_out_capture`、`w` 全体を読み `y` 全体に書く) を一覧の位置だけに。`rho` の前回の位置は
  `NzTrack` で消す。非ゼロ行の昇順一覧をそのまま PRICE の行一覧 `rho_rows` に使う (`compact_rows` を省く、策5)。
  `tau` FTRAN 用の非ゼロステップ記録 (`StepCapture`) は昇順ソートして従来と同じ内容にする。
- 一覧が `BTRAN_HYPER_FRACTION` (0.10) `* m` を超えたらその位置から全走査に切り替える (途中から全走査しても同じ
  順序)。結果の非ゼロ率の移動平均 (`FtranDensity` と同じ重み) がその半分以上なら最初から全走査にする
  (当初は「前回が密なら全走査、全走査の結果が疎なら次回また試す」にしていたが、sctap1 のように非ゼロ率が 10% 前後の
  問題で超疎経路と全走査を交互に繰り返し、途中で落ちる分の手間が無駄になっていた)。
- `ENOMOTO_SPARSE_BTRAN=0` で従来の全走査 (A/B 用)。`ENOMOTO_T_BTRAN_HYPER_FRACTION` で閾値を上書き。
- 大域最適化で `rho` を別経路 (`trial_row_ratio`、全体書き) で書く場合は `invalidate_out` で記録を無効化する。

### 策3: FT 更新の eta を疎入力から作る (src/simplex/lu.rs `EtaFile::push_scaled_list` / `FtLu::try_update_tracked`)

- `commit_update` の `push_scaled_dense` は長さ `m` の `e_tilde`/`a_tilde` を 2 回ずつ走査 (数える → 詰める) していた。
  BTRAN の `e_tilde` 記録位置 (`UnitBtranWork::e_touch`) と FTRAN の `a_tilde` 記録位置 (`FtranTrack::a_tilde`) を
  昇順ソート・重複除去して、そこだけから同じ eta を作る。非ゼロ数・疎/密判定・要素順 (添字昇順) が同じなので
  ビット一致。密形式になる場合と記録が無効な場合は従来の密走査。

### 策4: `U` eta の置換で `Vec::remove` をやめる (src/simplex/lu.rs `EtaFile::remove`)

- 置換された `U` eta のヘッダを 3 本の並列配列から `Vec::remove` で除き、後続ヘッダの `slot_pos` を付け直していた
  (後続ヘッダ数 = 非シングルトン `U` 列数に比例する memmove)。ヘッダに死んだ印 (`key = ETA_DEAD_KEY`、要素なし)
  を付けるだけにし、`EtaFile::iter`・`u_transpose_order`・超疎 `U^T` の全走査への切り替えで飛ばす。位置がずれない
  ので `slot_pos` の付け直しも不要。死んだヘッダは次の再分解で捨てられる。

### 策8: `profile_phases` 時の `dot(rho, rho)` 診断を作業量集計側へ

- DSE 重みの相対誤差の診断 (`O(m)`) を `ENOMOTO_PROF_PHASES_EXT_WORK` のときだけにした (フェーズ計測の歪み除去)。

### 策7: 大きな問題で chuzr 候補短縮リスト (S11) を自動で有効に (src/simplex/slope_intercept_dual.rs)

- 主実行不能行プールが数万〜十数万行 (stormG2_1000 で m の 35%) になり、毎反復のプール全走査
  (`Score2::new` + `cmp_lex`、約 8 ns/行) が支配的になっていた。既存の S11 (HiGHS `chooseHyperSparse` 風の
  短縮リスト、既定オフ) を、`m >= CHUZR_SHORTLIST_AUTO_MIN_M` (10,000) の問題では `K` 未指定でも有効にする。
  Netlib (m ≤ 約 6K) には掛からないので経路は変わらない。
- 自動モードの `K` は全走査のたびに `clamp(プール行数 / 64, 64, 512)` で決め直す (storm k=200 で固定 K=32/64/128 は
  chuzr 47.7 / 29.8 / 14.7 µs/反復、プール比例 (除数 32/64/128) では 11.2 / 9.1 / 10.9 µs/反復)。
- 全走査の 2 パス目 (上位 `K+1` 行を挿入ソートで集める。`K` が大きいと `O(K)` の挿入が重い) をやめ、最良行を探す
  1 パス目の中で「最も劣る行を根に持つ二分ヒープ」(`shortlist_heap_offer`) に集める。`cmp_lex` の許容誤差は推移的で
  ないがヒープは panic しない (集合が近似になるだけ)。明示指定 (`ENOMOTO_T_CHUZR_SHORTLIST=K`) の S11 も同じ実装になる。
- 経路はビット同一ではない (近い同点の選び方が変わりうる) が、storm k=50/200 で反復数・目的関数値は不変。
- `ENOMOTO_T_CHUZR_SHORTLIST_AUTO_MIN_M=0` で無効。`_AUTO_K` / `_AUTO_DIV` / `_AUTO_K_MAX` で調整。
- **その後、自動モードは短縮リストをやめて遅延最大ヒープに置き換えた** (`ChuzrEntry`、`ENOMOTO_T_CHUZR_HEAP=0` で上の短縮
  リストに戻る)。本体 (プール平均 8.7 万行) では短縮リストでも chuzr が 54 µs/反復 (リスト 2,000 行超の毎反復走査と
  数十反復ごとの全走査) 残っていた。ヒープはプール全行の `(スコア, 行, 版番号)` を持ち、逸脱か DSE 重みが変わった行
  (`x_B` 更新の一覧と `r`) は版番号を進めて新しい要素を積む。根が古い (版番号違い・プール外) なら捨てる。
  再分解・一覧のない反復・途中で `continue` した反復の後は作り直す (`O(プール)` の heapify)。古い要素がプールの
  2 倍を超えても作り直す。storm k=200 で chuzr 7.2 → 1.8 µs/反復 (反復数・目的関数値は不変)。

### 策9: ドリフト検査の `fill` 省略と大きな問題での間引き (src/simplex/slope_intercept_dual.rs)

- `residual_norm_affine` / `residual_scale_affine` / `residual_norm_slope` の入口の `fill(0.0)` をやめ、残差を集める
  最後の走査で作業領域を 0 に戻す約束にした (値・順序は同じでビット一致)。
- `m >= XB_CHECK_CADENCE_LARGE_M` (10,000) の問題ではドリフト検査の間隔を 20 → `max(100, m / 256)` 反復に
  (`XB_CHECK_CADENCE_LARGE` / `_DIV`)。検査 1 回が `O(m + nnz(A_B))` (m=19K で約 110 万命令) で、storm k=50 では
  20 反復ごとの検査だけで総時間の約 15%、本体 (m=378K) では 100 反復ごとでも未計時分の大半 (1 反復 100 µs 規模) だった。
  間隔を m に比例させて償却コストを m によらず一定にする (k=200 で間隔 100 → 300 が総時間 -11%)。storm の再分解は
  合成クロック起因だけでドリフト起因は 0 回。Netlib には掛からない。`ENOMOTO_T_XB_CHECK_LARGE_M=0` で無効。

### 策10: 大きな問題で合成クロックの再分解間隔を `sqrt(m)` に比例して広げる (src/simplex/slope_intercept_dual.rs)

- 求解の `O(m)` パスを消した後も CLOCK tick は段ごとに一律 `m` を数えるので、再分解は m によらず約 436 反復ごと
  (storm) のまま。再分解 1 回の手間は m に比例し、更新 1 回ごとに増える `R` eta の手間は m によらないので、
  釣り合う間隔は `sqrt(m)` に比例する。`m >= SYNTH_CLOCK_LARGE_M` (10,000) なら係数を
  `SYNTH_CLOCK_FACTOR * sqrt(m / SYNTH_CLOCK_LARGE_REF_M)` にする。基準行数は当初 5,000 (m=19K で約 31、m=76K で
  約 62、m=378K で約 139)、疎な `R` 段 (追加策 R) の後に 2,000 に再調整した (m=378K で約 220。本体 92 → 84 s)。
- 報告の案 (tick を実作業量にする) は Netlib を含む全問題の再分解時期を変えるので、m でゲートできる係数の拡大にした。
- storm k=200: 係数 16 / 32 / 64 で 20.0 / 17.7 / 16.3 s (再分解 219 / 110 / 56 回)。反復数・目的関数値は不変。
  `ENOMOTO_T_SYNTH_CLOCK_LARGE_M=0` で無効。

### 策12: DSE `tau` FTRAN の `U` 段 (src/simplex/lu.rs `u_solve_hyper` / `u_solve_partial`)

策1〜10 の後、storm k=200 (m=75,636) の 1 反復 147 µs のうち FTRAN が 77〜92 µs で、その大半が DSE の
`tau = B^-1 rho` の超疎 `U` 段だった (`tau` の非ゼロは m の約 1.3% で、m に比例して増える。超疎段の中断は 0 回)。

- 超疎 `U` 段 (C5) を「DFS で構造的な到達集合を集める → `u_seq` 位置をソート → 降順に適用」から、「非ゼロになった
  スロットの位置を最大ヒープに積み、降順に取り出して適用しながら書いた行を積む」に変えた。スロット `p` に書く eta は
  すべて `p` の eta より後ろの位置なので、取り出した時点で値は確定しており、適用する eta と順序は以前と同じ
  (値・tick ともビット一致)。DFS の辺走査・スタック・位置のソートが消える。
  一覧が上限を超えたら、その位置から全走査で仕上げる (`UHyper::Full`。以前は `x` に触れる前に中断していた)。
- 超疎 `U` 段の起点を「`L` の到達集合 + 全 `R` eta の行」から「`L` の到達集合 + `R` eta で値が変わった行
  (`GpScratch::r_seeds`)」に。`R` eta が 1,000 個を超えると、全 `R` eta の行を見るだけでランダムアクセスが数千回あった。
  `a_tilde` の記録も同じ一覧を使う。
- **部分 `tau`** (`m >= PARTIAL_TAU_MIN_M` = 10,000 の問題のみ): DSE 重みの更新 (`update_after_pivot_rows`) が読む
  `tau` は入る列の結果 `alpha` の非ゼロ行だけなので、融合 FTRAN で `alpha` が超疎に解けたら、`tau` はその行だけ求める
  (`u_solve_partial`)。必要なスロットの依存閉包を `row_owners` (`U` の行 → その行に要素を持つ eta) でたどり、
  `u_seq` 位置の降順に、各スロットへの寄与を `row_owners` から位置の降順に集めて加える。全体の `U` 段が同じスロットに
  加える演算と同じ値・同じ順序なので、求めた行の値はビット一致 (単体テスト `tracked_iteration_matches_untracked_reference`)。
  ただし CLOCK tick は集めた寄与の数で数えるので再分解の時期が変わりうる (そのため m でゲート)。フリップ結果の併合が
  ある反復や、行一覧での DSE 更新にならない反復では使わない (後者は全体の `tau` を求め直す)。
  `ENOMOTO_T_PARTIAL_TAU_MIN_M=0` で無効。
- storm k=200: FTRAN 92 → 77 µs/反復 (ヒープ化・`r_seeds`) → 17 µs/反復 (部分 `tau`)。総時間 16.8 → 15.1 → 8.5 s。
  k=50: 1.25 → 1.10 → 0.87 s。反復数・目的関数値は不変。

### 追加策 R (報告外): 疎なベクトルへの `R` eta 適用 (src/simplex/lu.rs `FtLu::apply_r_sparse`)

- 策10 で再分解間隔を広げると `R` eta (FT 更新ごとに 1 つ) が数千個たまり、融合 FTRAN は毎回それを全部なめて
  2 本のベクトルとの内積を取っていた (本体で 1 反復数十 µs)。入る列も `tau` の右辺も `L` 段の到達集合は数十個で、
  内積が 0 でない eta はごく一部。
- `R` eta の列方向索引 (位置 → その位置に要素を持つ eta の連結リスト、`r_head`/`r_next`/`r_owner`) を FT 更新の
  採用時に作り、非ゼロになりうる位置 (`L` 段の到達集合) に要素を持つ eta だけを作成順の最小ヒープで処理する。
  eta `k` が `x[p_k]` を変えたら、`p_k` に要素を持つ `k` より新しい eta を加える。選ばれない eta の内積は `±0` の和で
  `x` を変えないので、値はビット一致。CLOCK tick は全 eta の非ゼロ数の和を一括で加える (従来と同じ値)。
  選んだ eta が半数を超えたら残りは全部順に当てる。両ベクトルの `L` 段が Gilbert-Peierls で、`R` eta が
  `R_SPARSE_MIN_ETAS` (256) 個以上あるときだけ使う (少ないうちは全部順に当てる方が速い。常時使うと Netlib sctap1 で +5%)。
- storm k=200: FTRAN 17 → 11 µs/反復、総時間 8.5 → 7.2 s。Netlib 93 問はビット一致。

### BFRT 合成フリップ列の疎 FTRAN にも超疎 `U` 段・疎な `R` 段 (src/simplex/lu.rs `FtLu::solve_sparse_into`)

- BFRT のフリップ列 (`combined_base`/`combined_slope`) の疎 FTRAN は `U` 段を全 eta 走査し、出力を全体置換して
  スクラッチを全体 `fill` していた (1 回 `O(m)` × 3)。本体ではフリップのある反復は 1% 強だが 1 回 1 ms 近く、
  BFRT 段が平均 9.7 µs/反復あった。入る列と同じく、結果密度の移動平均 (`density_bfrt`) が
  `FTRAN_U_HYPER_DENSITY` 未満なら超疎 `U` 段 (`gp.u_hyper`) と疎な `R` 段 (追加策 R) を使う。値・tick はビット一致
  (単体テストで確認)。storm k=200 で BFRT 段 0.7 µs/反復。

### 小さな問題のコード配置を元に戻す (`SPARSE_PATH_MIN_M`)

- 最終判定ベンチで、極小問題 (sc50a/sc50b/kb2/adlittle/blend) が一貫して 5〜7% 遅くなっていた (sc50a +11%、
  sc50b +11.7%)。命令数はほぼ同じ (+0.5〜0.8%) で、callgrind のキャッシュシミュレーションでは I1 ミスが主ループ・
  BTRAN・FT 更新・融合 FTRAN で 1.5〜2 倍 (新しい疎経路のコードがホット関数に入ってコードが肥大・配置が変わった)。
  経路を env で切っても差は縮まらなかった (実行しない分岐でも配置で効く)。
- 新しい疎経路 (超疎 BTRAN、融合 FTRAN の出力記録、疎入力の FT 更新、疎な `R` 段、部分 `tau`、BFRT 列の超疎 FTRAN、
  chuzr の遅延ヒープ、死んだ `U` eta ヘッダ) をすべて元の関数から分離した:
  - `lu.rs`: 元の関数 (`solve_*_pair/triple_capture`、`pair/triple_r_u_permute`、DFS 版 `u_solve_hyper`、
    `solve_sparse_into(_capture)`、`solve_transpose_unit_work`、`commit_update`、`l_solve_steps_into`、`EtaFile::iter/remove`)
    は元のコードに戻し、新経路は `*_tracked` / `*_sparse` / `*_hyper` / `u_solve_hyper_heap` / `commit_update_tracked` の
    別関数 (`#[inline(never)]`) にした。
  - 死んだ `U` eta ヘッダ (策4) はキー (スロット) を残したままピボット 1・要素なしにする形 (`EtaFile::kill`) に変えた。
    `U` 段の走査はそのまま通しても値・tick が変わらないので元のループのままでよく、`U^T` 掃引だけ死んだヘッダがあるとき
    (`n_dead > 0`) に別関数 (`u_transpose_sweep(_track)_live`、`slot_pos[スロット] != 位置` で判定) に回す。
    `lazy_remove` は `m >= SPARSE_PATH_MIN_M` の `FtLu` だけ。
  - 疎な `R` 段は、全 `R` eta が `commit_update_tracked` で列方向索引に載ったときだけ使う (`r_indexed`)。
  - 主ループ `solve_slope_intercept_dual_with` は `solve_slope_intercept_dual_impl::<BIG>` の 2 つの実体に分け、
    `m < SPARSE_PATH_MIN_M` (300) なら新経路のコードを含まない `BIG = false` の実体を使う。
- 閾値 300 は presolve 後 m での計測から (ship04l 313 は新経路で -9%、czprob 463 / ganges 490 は -15%、
  sctap1 269 / fffff800 279 は新経路だと +3〜5%)。
- 結果: 400 回求解の最小時間でベース比 sc50a -1.7%、sc50b +1.7%、kb2 -0.4%、adlittle +0.5%、blend +0.4%、
  afiro +2.2%、sc105 +0.3%。adlittle ×20 の I1 ミスは全体で 4.58M → 4.62M (+1%)、主ループ 206K → 228K
  (分離前は 301K)。Netlib 93 問はビット一致のまま、storm k=200 は 5.5 → 4.9〜5.3 s。

### 効果

- 策1〜6・8 (すべてビット同一) の時点: Netlib 93 問は全問でステータス・目的関数値 (ビット)・反復数がベースと一致。
  storm 縮小版 k=50 (m=18,936): 6.3 s → 2.3 s、k=200 (m=75,636): 132 s → 43 s (反復数 95,510 で不変)。
- 全体 (策1〜10・12・13、BFRT 列の疎 FTRAN):
  - Netlib 93 問: ステータス・目的関数値 (ビット)・反復数とも全問ベースと一致 (大きな問題向けの経路変更は
    すべて `m >= 10,000` でゲート)。`debug_assertions` 付きビルドでも全問通過。`scripts/ab_bench.py --rounds 3`:
    幾何平均 -1.9%、shifted (100 ms) -0.3%、10% 超の退行なし (1 ms 未満の問題の ±数% は計測のばらつき)。
  - storm 縮小版: k=50 6.3 → 0.80 s、k=200 132 → 5.5 s (HiGHS 5.25 s)。反復数・目的関数値は不変
    (k=50 の目的関数値は最終桁が 1 ulp 変わる)。
  - stormG2_1000 本体 (m=378,036): ベース 3,600 s 枠で打ち切り (外挿 ~4,200 s) → 83.4 s (presolve 込み、
    477,103 反復、目的関数値 15802591.120880328、HiGHS 1.15.1 は同じ機械で 61.7 s / 522,245 反復 / 15802591.12088125)。
    1 反復 8.8 ms → 約 165 µs。残りの内訳 (PROF_PHASES_EXT): FTRAN 約 48% (DSE `tau` の `L` 段の
    Gilbert-Peierls、部分 `tau`、`R` 段)、再分解約 22%、BTRAN 約 5%。
- 取り下げ・未実装: 策11 は策2 の `L^T` 段のヒープ化に含めた。策14 (並列化) は策7 のヒープ化で chuzr が
  1 反復数 µs になり不要。策15 (presolve) は本体で 5 s 程度 (総時間の 6%) で未着手。策13 (報告の番号、`d` の
  ドリフト検査) は策9 の間隔拡大で相対的に小さくなったので据え置き。

## pds-100 / s250r10 / square41 / ex10 対応 (2026-09-25)

分析 `analysis/pds100_s250r10_20260925_123500.md` (以下「報告 P」) の §4 と `analysis/square41_ex10_20260925_122411.md`
(以下「報告 S」) の §6 の改善策を実装した記録。策番号はそれぞれの報告のもの。4 問とも HEAD では 600 s 打ち切り
(実行不能行数のプラトー検出の誤発火で Bland 規則に落ちて収束しない)。

### 報告 P 策1 / 報告 S 策1(a): 実行不能行数プラトー検出の上限を実際の反復予算に連動 (src/simplex/slope_intercept_dual.rs)

- 上限 `min(4·stall_limit, MAX_ITERS_FLOOR / 4)` は反復予算が 20,000 固定だった頃の名残で、m ≥ 250 の全問題で
  5,000 反復固定だった。双対単体法 (BFRT 付き) では主実行不能行の個数は単調でなく、大きな問題では最小値を数千反復
  更新しないのが普通なので、pds-100 (反復 ~13,000) と s250r10 (~7,000) で誤発火して Bland 規則に入り、1 反復が
  3〜8 倍遅くなったうえ収束しなくなっていた。
- 上限の頭打ちを実際の予算 `max_iters_for(m, n_total) / 4` にした (m + n ≤ 1,000 なら予算は `MAX_ITERS_FLOOR`
  なので値は従来どおり 5,000)。pilot4/greenbea 型の停滞 (本来の対象) では予算の 1/4 以内で発火する性質は保つ。
- 策2 (指標を双対目的の改善に変える) は見送り: Netlib 93 問は現状 1 問も Bland に入らず、策1 だけで両問題とも
  誤発火しなくなる (pds-100 の上限は 1.75M、s250r10 は 139,600 反復 = `4·stall_limit`)。
- `ENOMOTO_T_PLATEAU_BUDGET_FLOOR=1` で従来の上限 (A/B 用)。
- 結果: Netlib 93 問はステータス・目的関数値 (ビット)・`DEBUG_EXT` の反復数行とも全問一致。pds-100: 打ち切り →
  206 s (174,880 反復、10928229968.0)、s250r10: 打ち切り → 141 s (117,384 反復、-0.17267704190548122)。
  いずれも別の重い計測と同時実行 (負荷 3〜4) での時間。
- square41 (m = 1,754) は上限 `4·stall_limit = 35,080` がそのまま効き、ex10 (m = 63K) は 126 万反復。どちらも
  誤発火しなくなる (square41 は既定経路で 20,906 反復、ex10 は 22,207 反復で最適)。

### 報告 P 策2 / 報告 S 策1(b): プラトー検出で双対目的関数の進展も進展として数える

- 実行不能行数は最適解の手前まで増減しうる (ex10 は開始点の 204 行が最小値のまま、square41 は 300〜380 行で
  2 万反復振動) ので、策1 の上限だけだと反復数の多い問題でいずれ誤発火しうる (square41 は上限 35,080 に対し
  既定経路の反復数が 2〜3 万)。プラトー計数を最後にリセットしてからの `|contribution_base|` の累積が、求解開始から
  の累積 (目的関数の尺度) の `PLATEAU_OBJ_REL` (1e-6) 倍を超えたら (傾き成分の寄与は常に) 進展ありとしてリセットする。
  寄与が微小なまま行集合も動かない本物の停滞 (pilot4/greenbea 型) だけを捕まえる。
- Netlib 93 問は現状 Bland に入らない (計数をリセットする条件が増えるだけ) のでビット一致。
  `ENOMOTO_T_PLATEAU_OBJ_REL=0` で無効。
- 報告 S 策1(c) (Bland から抜ける条件) は見送り (Bland に入らなくなったため)。

### 報告 S 策2: 密な基底の FT 更新の fill 上限を LU の大きさに比例させる (src/simplex/slope_intercept_dual.rs, lu.rs)

- square41 (1,754 × 23,828、列あたり 182 非零) は基底の LU が 16.5 万要素 (≈ 94·m) で、FT 更新 1 回の eta が数千要素
  になり、fill 上限 `FT_BUMP_LIMIT_FACTOR·m = 64·m` (11.2 万) で 20 反復に 1 回再分解していた (1 回 179 ms)。
- 分解の格納要素数 `nnz(LU)` (対角込み、`FtLu::lu_nnz`、再利用分解でもその分解自体の値) が `64·m` 以上なら、
  上限を `FT_BUMP_LU_RATIO · nnz(LU)` (3 倍) にする。当初の案 `max(64m, 3·nnz(LU))` は pilot87 (LU ≈ 31·m) と
  maros-r7 で bump 起因の再分解時期が変わって経路が変わったので、「LU 自体が従来の上限より大きい」ときだけに絞った
  (Netlib の LU は最大でも約 35·m)。
- Netlib 93 問はビット一致。square41: 再分解 1,584 → 49 回、`ENOMOTO_T_FT_BUMP_LU_RATIO=0` で無効。

### 報告 S 策3: PRICE の密結果モード (src/simplex/slope_intercept_dual.rs `price_row_dense`)

- square41 は `rho` が m の 30〜80% 非零で、行方向 PRICE が 1 反復 380 万要素 (PRICE だけで 1 反復の 80%)。各要素で
  `touched[j]` の初到達判定 (分岐) と `touched_cols` への積み込みをしていた。
- 前反復の PRICE 要素数が `PRICE_DENSE_RESULT_RATIO · n_total` (1.0) を超えたら、判定なしで `a_p` に加算だけ行い、
  加算の結果がちょうど 0 になった列には `-0.0` を置いて「触れた」印を残す (`-0.0 + x` と `+0.0 + x` は同じ値)。
  PRICE の後に `a_p` を 1 回順に走査し、ビットが `+0.0` でない列を列番号の昇順で `touched_cols` にする。
  触れた列の集合と `a_p` の値は同じで、一覧の順序は後段 (chuzc1 の候補は `(ratio, j)` の全順序で並べる、`d` 更新は
  列ごとに独立) に影響しない。`-0.0` を読むのは chuzc1 (`|alpha_j| <= TOL` で落ちる) と `d` 更新 (`+ 0.0` で `+0.0` に
  戻してから使う) だけなのでビット一致。`BIG` のみ、`ENOMOTO_T_PRICE_DENSE_RESULT=0` で無効。
- Netlib 93 問はビット一致 (scsd8 など `rho` が密な問題もこのモードに入る)。square41: PRICE 8.76 → 6.65 ms/反復、
  230 → 185 s。残りはメモリ帯域 (1 反復 46 MB の読み出し) と反復数 (HiGHS 10,227 に対し 20,906)。

### 報告 P 策8・策7: chuzc1 の候補全体のヒープ化をやめ、上位 `K` 個を選んでから歩進 (src/simplex/slope_intercept_dual.rs)

- s250r10 は 1 反復の候補が平均 7,500 個で、BFRT の歩進は数個〜十数個で止まるのに全候補を `BinaryHeap::from` で
  ヒープ化していた (chuzc1 の 3 割)。1 パスで `(ratio, j)` 順の小さい方から `CHUZC1_TOPK` (128) 個を上限付きの
  最大ヒープに集めて並べ、先に歩進する。止まらなければ、直前の組の最大より大きい候補から次の組 (個数は 4 倍ずつ) を
  同じように選んで続ける。取り出す順序は全体ヒープと同じなのでビット一致 (策8)。
  当初は止まらなければ残り全部をヒープにしていたが、s250r10 の序盤 (フリップ 11 回/反復) では 32 個で止まらない
  反復が多く、残り (停止候補で刈り込んでいない数千個) の heapify と写しが callgrind で命令数の 17% を占めていた。
  組を 4 倍ずつ広げる方式に変えて 824 → 652 µs/反復 (K = 32)。K = 16 / 32 / 64 / 128 / 256 / 512 は
  661〜694 / 652〜668 / 633 / 605〜633 / 612 / 636 µs/反復で 128 にした。
  停止候補で刈り込んだ後の候補が `CHUZC1_TOPK_MIN_CANDS` (1,024) 未満なら従来の全体ヒープを使う (Netlib czprob で
  K = 128 の選択が全体ヒープより 5% 遅かったため)。
- さらに、この経路では停止候補 (幅が無限の候補の最小) による刈り込みと `candidates` への写しを省き、
  `cand_scratch[..k]` をそのまま候補列にする案 (策7 の一部) も入れたが、Netlib の A/B で scsd8 +17%、gfrd-pnc +19%、
  pilot.we +15%、perold +10%、25fv47 +8% (単独実行の最小時間、ビット一致) と退行した。片側行のスラック (幅無限) が多い
  問題では刈り込みで候補が桁違いに減るのに、刈り込まないと数千候補を毎回選択にかけるため。「幅無限の列が全列の 20%
  未満」で判定する案も pilot.we (+15%) と 25fv47 で外れた。最終的に (1) 列数の多い問題 (`n_total >= PREFETCH_MIN_COLS`、
  `width_inf` がキャッシュに乗らない) に限り、(2) 上位候補の選択を使う反復の `CHUZC1_FAST_PROBE` (64) 回に 1 回は刈り込みを
  行って残った割合を測り、半分より多く残ったら次の測定まで刈り込みを省く、とした (`ENOMOTO_T_CHUZC1_FAST=0` で常に刈り込む)。
  s250r10 の最初の 2 万反復: 刈り込みあり 697 µs/反復、省略 617 µs/反復。
- `BIG` かつ候補数が `K` を超える反復のみ。`ENOMOTO_T_CHUZC1_TOPK=0` で従来の全体ヒープ。
- s250r10 の最初の 2 万反復 (`ENOMOTO_T_PROF_MAX_ITERS` を足した計測用ビルド、単独実行): 1 反復 1,033〜1,143 →
  922〜926 µs (K = 32、残り全部をヒープにする版。chuzc1 441〜488 → 363〜367 µs、うちヒープ 130〜137 → 53 µs)、
  組を広げる版と下のプリフェッチを合わせて 605〜633 µs。

### 報告 P 策9 (touched 配列の廃止): 試して取り下げ

- `a_p[j]` のビットが `+0.0` かで初到達を判定する案 (打ち消しで 0 になった列は `-0.0`) は、`d` 更新の `touched[j] = false`
  が消えて −30 µs だが、PRICE の分岐が `a_p` の読み (キャッシュミス) に依存して解決が遅れ +27 µs で差し引きゼロ。
  1 列 1 ビットのビット集合 (s250r10 で 34 KB) も PRICE +18 µs / `d` 更新 −21 µs で同程度。どちらも取り下げ、
  `touched` のまま (密結果モードの `-0.0` の扱いだけ残した)。

### 列数の多い問題でのソフトウェアプリフェッチ (報告 P 策7 の代わり)

- 報告 P 策7 (列ごとの配列を構造体にまとめる) は `d`・`nb_status` を使う箇所が多く大改造になるので、代わりに
  `n_total >= PREFETCH_MIN_COLS` (10 万) の問題で、PRICE (`price_row_prefetch`)・chuzc1 の候補フィルタ
  (`chuzc1_filter_prefetch`)・`d` 更新 (`dual_update_prefetch`) の列添字ランダムアクセスを `PREFETCH_DIST` (16) 要素先から
  `_mm_prefetch` で先読みする。値は変わらない。小さな問題の主ループのコードを増やさないよう `#[inline(never)]` の
  別関数にした。`ENOMOTO_T_PREFETCH_MIN_COLS=0` で無効、`ENOMOTO_T_PREFETCH_DIST` で距離 (8/16/32 で差なし)。
- s250r10 の最初の 2 万反復: 847〜859 → 823〜825 µs/反復 (PRICE 233 → 209、chuzc1 324〜329 → 314〜317 µs)。

### 報告 P 策10 (`d` ドリフト検査の間引き): 試して取り下げ

- `nnz(A) >= 25 万` で `d` のドリフト検査の周期を `nnz(A)/25 万` 倍にする案は、検査の BTRAN が合成クロックに tick を
  加えるため再分解時期が変わって経路が変わり (s250r10: 117,385 → 113,149 反復、再分解 249 → 241 回)、2 万反復の
  計測でも未計時分が 1 反復 10 µs 減るだけだったので取り下げた。

### 報告 S 策7: `alpha` が密な反復の後は chuzr の遅延ヒープを作り直さず全走査 (src/simplex/slope_intercept_dual.rs)

- ex10 (m = 63K、入る列の FTRAN 結果の 80% が非ゼロ) では `x_B` 更新が毎反復一覧なし (全行走査) になり、遅延ヒープ
  (策7、`m >= 10,000`) が毎反復無効になってプール 2 万行の `BinaryHeap::from` を払っていた (chuzr 0.66 ms/反復)。
  前反復の `x_B` 更新が一覧なしなら、この反復はヒープを作らずプールの全走査 (比較 1 回/行) で選ぶ。次に一覧のある
  反復が来たらヒープを作り直す。`cmp_lex` の許容誤差の非推移性のぶん選ぶ行がヒープと違いうる (経路が変わる) が、
  遅延ヒープ自体が `m >= 10,000` 限定なので Netlib は不変。pds-100 / stormG2_1000 は一覧のない反復がほぼ無く経路不変
  (pds-100 は反復数 174,881 のまま)。`ENOMOTO_T_CHUZR_DENSE_SCAN=0` で無効。

### 報告 S 策11: 求解結果が密な問題では合成クロックの `sqrt(m)` 倍を掛けない

- 策10 (stormG2 対応) の `sqrt(m / 2000)` 倍は「求解の `O(m)` パスを消したので求解の手間は m によらない」ことが前提で、
  ex10 のように入る列の FTRAN 結果が密な問題では求解の手間も m に比例する。入る列の結果の非ゼロ率の移動平均が
  `SYNTH_CLOCK_DENSE_FRACTION` (0.3) 以上なら倍率を掛けない (ex10 約 0.8〜0.9、pds-100 約 0.09)。`m >= 10,000` 限定なので
  Netlib 不変。`ENOMOTO_T_SYNTH_CLOCK_DENSE_FRACTION=0` で無効。
- ex10 (策7 と合わせて): 141.6 s / 23,651 反復 → 116.3 s / 22,207 反復 (再分解 36 → 72 回、chuzr 658 → 398 µs、
  BTRAN 824 → 531 µs、FTRAN 2,018 → 1,704 µs/反復)。

### 報告 P 策5(b): 求解結果が中程度に密な問題では合成クロックの倍率を下げる

- pds-100 (m = 87K) は策10 の倍率 `16·sqrt(m/2000) = 106` で 2,270 反復ごとに再分解していたが、FT 更新が積もるにつれて
  `R` 段 (平均 1,181 eta) と `tau` の密な反復 (FTRAN の全走査フォールバック) が重くなり、間隔が長すぎた。
  最初の 6 万反復: 基準行数 2,000 / 8,000 で 454 / 269 µs/反復 (FTRAN 192 → 92 µs)。完走: 基準行数 2,000 / 8,000 / 32,000
  (係数 106 / 53 / 26) で 179〜206 / 136 / 150 s (反復 174,881 / 169,272 / 178,021)。
- stormG2_1000 は逆に長い間隔が良い (基準行数 5,000 で 92 s、2,000 で 84 s) ので、両者を DSE `tau` の FTRAN 結果の非ゼロ率
  (移動平均) で分ける: stormG2_1000 は最初の 10 万反復で平均 6 非零 / 378K 行 (2e-5)、pds-100 は 1,100 / 87K 行 (1.3%)、
  ex10 は 60〜90%。非ゼロ率が `SYNTH_CLOCK_MID_TAU_FRACTION` (0.2%) 以上なら基準行数を `SYNTH_CLOCK_MID_REF_MULT` (4) 倍に
  する (`SynthDensity::Mid`)。策11 の密 (`Dense`) と合わせて 3 段階。`m >= 10,000` 限定なので Netlib 不変。
  `ENOMOTO_T_SYNTH_CLOCK_MID_TAU_FRACTION=0` で無効。

### 報告 P 策13 の一部: 合成フリップ列の FTRAN 結果の非ゼロ位置を記録 (src/simplex/lu.rs `solve_sparse_into_hyper_tracked`)

- callgrind (pds-100 の最初の 15,000 反復) で命令数の 36% が `memset`、34% が `FtLu::permute_list` の `out.fill(0.0)` だった。
  BFRT の合成フリップ列の FTRAN (pds-100 は 1.7 フリップ/反復でほぼ毎反復) が超疎 `U` 段で解けても、出力
  (`combined_alpha_base`/`_slope`、長さ m = 87K) を毎回全体 0 埋めしていた。さらにフリップのある反復の `x_B` 更新の
  行一覧は `alpha_full`・フリップ結果 2 本を `compact_rows` で全行走査して作っていた (主ループ本体の命令数の大半、
  PROF_PHASES の未計時 17% の正体)。
- 出力に `NzTrack` を付け (`cab_track`/`cas_track`)、超疎に解けたら前回の位置だけ 0 に戻して一覧の位置を書く。記録の外で
  全体を書く経路 (密な FTRAN、入る列の融合 FTRAN への相乗り、段階 A) では無効化する。`x_B` 更新の行一覧は、入る列と
  フリップ結果の記録がすべて有効なら、その和集合の非ゼロ行を昇順に並べて作る (`compact_rows` と同じ行・順序)。
  値・行の順序ともビット一致。`BIG` のみ。
- pds-100 の最初の 6 万反復: 258〜263 → 212〜215 µs/反復 (BFRT 19 → 10 µs、未計時 17% → 6%)。
- 小さな問題では `fill` も `compact_rows` も安く一覧の和集合の並べ替えのほうが高いので、`m >= FLIP_TRACK_MIN_M` (10,000)
  に限る (`ENOMOTO_T_FLIP_TRACK_MIN_M=0` で無効)。callgrind (pds-100 の最初の 15,000 反復) の命令数は 14.3 G → 4.0 G。

### 報告 S 策6 (一部): 実行不能行集合の位置配列を `u32` に (src/simplex.rs `InfeasibleRows`)

- ex10 は `x_B` 更新で 1 反復 3〜5 万行を走査し、行ごとに `InfeasibleRows::pos` (`Option<usize>`、16 バイト) を読む。
  `u32` (集合外は `u32::MAX`) にして 1 行あたりの読み出しを 12 バイト減らした。値・集合の順序は同じ (ビット一致)。
- ex10 の最初の 6,000 反復: `x_B` 更新 800〜813 → 777〜787 µs/反復 (全体 −1.5%)。
- 策6 の残り (傾き 0 専用の行ループ、密な `alpha` での分岐削減) は、`x_b_slope` が全 0 であることを安く知る手段が無いのと、
  `alpha_i = 0` の行も更新すると `x_B` の符号付きゼロが変わりうる (ビット一致でなくなる) ので見送り。

### 報告 S 策5 (FTRAN の出力置換): 試して取り下げ

- `permute_out` を「全スロットの分岐なし置換 + シングルトンのスロットだけ除算」の 2 パスにする案 (ビット一致) は、
  ex10 ではシングルトンが多く 2 パス目がランダムアクセスの全走査に近くなり、最初の 6,000 反復で FTRAN 910〜945 →
  1,429〜1,431 µs/反復と大きく悪化したので取り下げた。除算を逆数の乗算にする案はビット一致でないので未実施。

### 効果 (pds-100 / s250r10 / square41 / ex10 対応の全体)

- Netlib 93 問: ステータス・目的関数値 (ビット)・`DEBUG_EXT` の反復数行とも全問ベースと一致 (大きな問題向けの経路変更は
  `m >= 10,000`・`n_total >= 10 万`・`nnz(LU) >= 64·m` などでゲート)。`cargo test --release --lib` 248 件通過。
  `scripts/ab_bench.py --rounds 3`: 合計 −2.0%、幾何平均 −0.7%、shifted (100 ms) 幾何平均 −0.2%、10% 超の退行なし
  (途中の A/B で出た scsd8・pilot.we・25fv47・czprob の退行は策7 の測定付き切り替えと策8 の候補数ゲートで解消)。
- Mittelmann 5 問 (600 s 打ち切り、2 本並走。同じ機械の負荷で ±15〜50% 揺れる):

| 問題 | HEAD | 本対応後 | HiGHS (依頼時の値) |
|---|---:|---:|---:|
| stormG2_1000 | 84.2 s (同時走行) | 80.8 s (経路不変 477,103 反復) | 64.0 s |
| square41 | 打ち切り | 150.7 s (17,927 反復) | 95.2 s |
| pds-100 | 打ち切り | 137.9 s (168,538 反復) | 75.9 s |
| ex10 | 打ち切り | 129.2 s (24,843 反復) | 90.4 s |
| s250r10 | 打ち切り | 90.8 s (117,384 反復) | 108.3 s |

- 98 問の shifted (100 ms、打ち切りは 600 s) 幾何平均: −6.4%。

## cont1 対応: 主単体法引き継ぎが実行不能解を optimal として返す問題 (2026-09-26)

分析 `analysis/cont1_20260926_001402.md` (以下「報告 C」) の §5 策1〜策3 を実装した記録。HEAD の cont1 は "optimal"
を返すが、元問題の 12 行を最大 6.6e-3 違反する実行不能解で、目的関数値 0.008782471582949961 は HiGHS
(0.00878248600370569) より 1.4e-8 小さかった。原因は仕上げ (`polish_with_true_bounds`) の主単体法への引き継ぎ
(`run_phase2_incremental`) が、比率テストの行き過ぎで上下限を外れた値のまま非基底にした列を、そのまま最終解として
返していたこと (最終基底自体は最適基底)。

### 報告 C 策1・策2: 引き継ぎ後に非基底を境界へ戻して `x_B` を作り直し、双対ループで確かめてから返す (src/simplex/slope_intercept_dual.rs)

- 主単体法が `Optimal` を返したら、`t.x` をそのまま返すのをやめ、基底状態を仕上げのループへ書き戻して、非基底を
  `nb_status` の境界値に置いた右辺 (`compute_rhs_plain`) から `x_B` を解き直し、仕上げの双対ループの先頭へ戻る。
  `x_B` が `PRIMAL_FEAS_TOL` (相対) で実行可能なら、先頭の「実行不能行なし」の分岐が真の費用で双対実行可能性を
  確かめて返す (Netlib の引き継ぎ 13 問と pds-100・s250r10 はすべてこちら)。実行不能なら、基底は真の費用で
  双対実行可能なので、新しい LU で `x_B` と真の被約費用 `d` を作り直して双対単体法で直す (cont1 はこちらで 2〜16 行、
  数〜12 反復)。
- 引き継ぎは `HANDOFF_MAX_ROUNDS` (3) 回まで。それを超えてまだ真の費用で双対実行不能なら、実行不能・非最適な解を
  optimal と報告しないよう `NotSolved` (`None`) にする。`ENOMOTO_HANDOFF_RAW_RETURN=1` で旧動作 (A/B 用)。
- 最初の版は戻るたびに再分解と `d` の作り直しをしていて、引き継ぎ 13 問が +2〜5% (1 回の再分解・BTRAN・`O(nnz)` の分)
  だったので、主単体法の LU で `x_B` を解いて実行可能ならそれだけで戻す形にした。

### 報告 C 策3: 主単体法引き継ぎの比率テスト (src/simplex.rs `run_phase2_incremental`)

- パス 2 の窓 `exact <= alpha1 + PRIMAL_HARRIS_TOL` はステップ長の絶対値なので、窓の中の他の行の行き過ぎが
  `harris·|alpha_i|` になり、`B^-1` が密で `|alpha|` が 100 級の cont1 では 1e-5〜1e-3 の違反を作っていた。EXPAND の
  緩め幅 `delta` がすでに変数空間の許容幅なので、窓なし (`exact <= alpha1`) にした。
- 出る変数が `delta` を超えて上下限を外れたまま非基底になる場合 (すでに外れていた基底変数が出る、最小ステップ
  `EXPAND_TAU/|pivot|` が大きい) は境界に置き直し、次の反復の頭で `x_B` を作り直す。`delta` 以内の外れは従来どおり
  `expand_reset_nonbasics` に任せる。`ENOMOTO_HANDOFF_RATIO_OLD=1` で旧版。
- 報告の案の「すでに外れた基底変数は戻る向きだけでブロックする」(`run_phase` 第 1 段階の規則) も試したが、cont1 の
  引き継ぎが 2,213 → 4,123 反復 (53 → 102 s) に増えたので採らなかった。cont1 の引き継ぎ (同じ双対段の終点から):

| 版 | 引き継ぎ反復 | 引き継ぎ時間 | 戻った後の実行不能行 |
|---|---:|---:|---:|
| 策1・策2 のみ (比率テストは旧版) | 2,213 | 53.0 s | 0 |
| 窓なし + 戻る向きの規則 + 出る変数を境界へ | 4,123 | 101.7 s | 16 |
| 旧窓 + 戻る向きの規則 + 出る変数を境界へ | 2,132 | 50.4 s | 2 |
| **窓なし + 出る変数を境界へ (採用)** | **1,819** | **45.1 s** | 0 |

- 策4 (`ENOMOTO_HANDOFF_FLIP` の既定化)・策5 (摂動量) は経路を変えるので後の A/B 枠で扱う。

### 効果

- cont1: optimal、0.008782486003705103 (HiGHS 0.00878248600370569 と相対 7e-14)、元問題の最大行違反 7.1e-13
  (HEAD は 6.6e-3)。518.7 s (HEAD 546〜553 s。引き継ぎ 55 s → 45 s)。
- Netlib 93 問: ステータス全一致、目的関数値の相対差は最大 9.7e-14 (greenbea)。引き継ぎ経路に入る 13 問はすべて
  「戻った後の実行不能行 0」。`scripts/ab_bench.py --rounds 3`: 合計 −0.1%、幾何平均 +0.1%、10% 超の退行なし
  (最大 fit2p +5.4%。引き継ぎ 13 問は −4.4〜+3.0%)。`cargo test --release --lib` 248 件通過。
- Mittelmann 5 問 (2 本並走): 目的関数値は HEAD と同じ (s250r10 のみ相対 3e-15 の差。pds-100・s250r10 は引き継ぎ経路に
  入り、戻った後の実行不能行 0)。stormG2_1000 73.2 s、square41 142.1 s、pds-100 128.0 s、ex10 114.5 s、s250r10 80.9 s。

## nug08-3rd / irish-electricity 対応 (2026-09-26)

分析 `analysis/nug08_irish_20260926_013500.md` (以下「報告 N」) の §5 の改善策の記録。策番号は報告 N のもの。比較の基準は
cont1 対応 (上の節) の後のコミット。

### 報告 N #3: 大きく密な LU の基底で稠密切替を自動で有効に (src/simplex/lu.rs `dense_switch_for`)

- 既存の B3 (`ENOMOTO_LU_DENSE_SWITCH`、活性部分行列の密度が残り `k^2` の割合に達したら残りを faer の稠密 LU で分解) を、
  [`factorize_reusing`] の通常分解で、行数 `m >= DENSE_SWITCH_AUTO_MIN_M` (1 万) かつ直前の通常分解の
  `nnz(L+U) >= DENSE_SWITCH_AUTO_LU_PER_ROW · m` (32 m) のときだけ閾値 `DENSE_SWITCH_AUTO_FRACTION` (0.3) で有効にする。
  LP 基底の LU は通常 1 行あたり数要素 (stormG2・pds-100 は 2〜4、cont1 は 12) で、nug08-3rd は 2 万反復以降 33〜84/行。
  閾値 0.3 は報告 N §3 の 30K 反復固定の実験 (0.15 は fill が増えすぎ、0.3〜0.5 が同程度) から。
- 旧実装は B3 が有効だと列シングルトンの前処理 (`peel_column_singletons`) ごと止めていたので、切替の有無にかかわらず
  経路が変わっていた。列シングルトンは消去を伴わず密度を上げないので、前処理の後から密度判定をするようにした。
- 密度判定の `O(m)` の数え上げは、行バッファの長さ (活性非零数の上界) が閾値に届くまで省く。
- `ENOMOTO_LU_DENSE_SWITCH` を設定すれば従来どおり全分解でその値 (A/B 用)、`ENOMOTO_T_LU_DENSE_SWITCH_AUTO_MIN_M=0` で自動切替なし。
- Netlib 93 問・Mittelmann の LU が疎な問題は条件を満たさず経路不変 (Netlib は目的関数値のビットが全問一致)。
- nug08-3rd (2 本並走): 642 s (報告 N の HEAD) → 489 s (再分解 13%、稠密切替 51〜80 回)。

### 報告 N #5・#6: 従属等式を Markowitz 消去で落とす (src/presolve/redundancy.rs `drop_dependent_equalities_markowitz`、src/simplex/lu.rs `markowitz_independent_columns`)

- 既定 (`REDEQ_MODE` 1) は重複行の除去だけで階数判定をしないので、nug08-3rd の一次従属な等式 1,458 行 (HiGHS も同数を落とす)
  が残っていた。`REDEQ_MODE` 0/2 の既存の判定は Dulmage-Mendelsohn の細かいブロックごとなので長方形・階数落ちの系では
  ブロックをまたぐ従属を見逃し (462 行)、分割しないと `BTreeMap` の消去が 126 s かかる。
- 等式行を列、変数と右辺を行とする正方行列を、単体法の基底分解と同じ Markowitz 消去 (平坦な格納・バケット探索・閾値
  ピボット・列シングルトンの前処理) で消去し、ピボットが取れた列 = 一次独立な等式の極大集合を残す。右辺も 1 行として
  含める (拡大行列の階数) ので矛盾する行は残る。列 `j` の要素は `DEP_TOL · (等式 j の元の拡大ノルム)` 未満ならピボットに
  しない (`MarkowitzState::col_floor`、通常の分解では空で無効)。全要素がこれ未満になった列は従属と確定するので、探索の
  たびに走査しないようバケットから外す (`dead_cols`)。
- 消去の終盤で活性部分 (残りの列 × 要素のある残りの行) が `REDEQ_DENSE_LIMIT` (1,600 万) 要素以下かつ密度
  `REDEQ_DENSE_FRACTION` (5%) 以上になったら、残りを列ピボット付き QR (既存の `drop_linearly_dependent` と同じ基準) で
  判定する。疎な消去のまま最後まで進めると nug08-3rd で 14 s (最後の 350 ピボットで 13.6 s)、QR の仕上げで 9.2 s。
  QR の代わりに完全ピボットの稠密ガウス消去 (手間は `行数 × 列数 × 残りの階数` で QR の約 1/4) も試したが、1 億要素級の
  行列を毎段 2 回読むメモリ帯域律速で 10.2 s と遅かったので、ブロック化された faer の QR のままにした。
- ラウンド後の縮小した A に対して、問題の行数 (等式行 + 多変数の不等式行) が `REDEQ_MARKOWITZ_MIN_ROWS` (1 万) 以上で、
  等式行の平均要素数が `REDEQ_MARKOWITZ_MAX_ROW_LEN` (64) 以下のときだけ行う。Netlib の A/B では階数判定を切って −9.5%
  だった経緯 (presolve_pipeline_20260924 §6) があるので小さな問題は従来どおり。密な等式行の問題 (square41 は 1 行 2,524
  要素で 8.3 s、従属 0) は割に合わない。`ENOMOTO_REDEQ_MARKOWITZ=0/1` で上書き。
- 手間 (従属 0 の問題は純粋な上乗せ): stormG2_1000 1.4 s、pds-100 3.2 s、s250r10 0.9 s、ex10 0.02 s。
- nug08-3rd (#3 と併用、2 本並走): 10,320 → 8,862 等式行 (18,270 行、HiGHS と同じ)。#3 のみ 491 s / 44,969 反復に対し
  299〜413 s / 35,923〜46,962 反復 (どの 1,458 行を落とすかは消去の細部で変わり、残った行で反復経路が変わる。
  開発中の版ごとの値の幅)。
- Netlib 93 問は行数ゲートで対象外 (目的関数値のビットが全問一致)。`cargo test --release --lib` 249 件通過
  (Markowitz 版の階数・矛盾行の保持のテストを追加)。

### 試して取り下げたもの

- 報告 N #1 (`ineqsingleton` を大きな問題で既定 on)・#13 (主実行不能判定を相対 `PRIMAL_FEAS_TOL` に)・#2 (特異基底からの復旧):
  irish-electricity は #1 で列 41K → 36.6K になり目的関数の進みは HiGHS に近づくが、どの組み合わせでも 600 s 以内に
  完走しなかった (4 本並走で 10〜14 万反復、目的関数値は最適の 98.7〜99%、HiGHS は 86.6K 反復)。
  - #2 は「直前の正常な再分解時点の基底へ戻る」(HiGHS の backtracking basis と同じ) を実装したが、同じピボット列を
    たどって同じ特異基底 (同じ反復番号) に着くだけだった。原因は候補が `|alpha| = 1e-9〜6e-6` の 1 列だけの行を
    chuzr が繰り返し選ぶこと (双対ステップ 1e12〜1e15)。戻った後に相対的に小さなピボットを候補から外す案は双対実行
    可能性が崩れて発散、HiGHS の `Ta` と同じ更新回数に応じた `|alpha|` の下限と「候補が 1e-5 未満の行を一時的に外す」
    案は特異化を先送りするだけ (10.5 万 → 12.6 万反復) で、nug08-3rd の経路も変わって 299 s → 397 s になった。
    特異基底の列をスラックに置き換える基底修復 (HiGHS の rank deficiency 処理) は分解には成功するが、置き換えた列の
    被約費用の符号が崩れて目的関数値が発散する場合があった (HiGHS はここで費用シフトをする)。
  - #13 単独は nug08-3rd で 335 → 313 s (2 本並走、経路の揺れの範囲) と中立で、m ≥ 1 万の全問題の経路を変えるので採らなかった。
  - いずれも irish が解けない以上、経路を変えるだけの変更になるので取り下げた (`ENOMOTO_INEQ_SINGLETON=1` は従来どおり使える)。
- cont1 報告 策4 (`ENOMOTO_HANDOFF_FLIP`): モード 1 は反転できない列 (箱型でない 120 列) があるため発動せず、モード 2
  (反転できる 668 列だけ反転) は主単体法 1,894 反復の代わりに双対 3,035 反復になって 545 s → 550 s。効果なし。
- 報告 N #4 (Markowitz カーネルに列方向の値)・#7 (DSE `tau` の密な FTRAN)・#8〜#11 は未着手 (#3 で nug08 の再分解は 13% まで
  下がり、irish は完走させられなかったため優先度を下げた)。

### 効果

(段階 2 全体の最終計測は次のコミットで記録する)

## 改名一覧 (整理時)

本メモ中は旧名で書かれている。

| 旧名 | 新名 | 場所 |
|---|---|---|
| モジュール `simplex::extended_dual` (`extended_dual.rs`)、`params::extended_dual` | `simplex::slope_intercept_dual` (`slope_intercept_dual.rs`)、`params::slope_intercept_dual` | simplex |
| `solve_lp_dual_extended` | `solve_slope_intercept_dual` | slope_intercept_dual.rs |
| `LARGE` | `MIMALLOC_SIZE_LIMIT` | params::alloc (lib.rs) |
| `EPS` (propagate / smallcoeff) | `PROPAGATE_EPS` / `SMALLCOEFF_EPS` | params::presolve |
| 関数内 `REL_TOL` (4 箇所) | `LEX_REL_TOL` | params::extended_dual |
| `PARALLELISM` (kkt.rs) | `KKT_PARALLELISM` | params::interior_point |
| `orig_of_free` / `n_free` / `x_free` / `shift_of_free` | `orig_of_kept` / `n_kept` / `x_kept` / `shift_of_kept` | simplex.rs |
| `refl_sign` | `reflect_sign` | simplex.rs |
| `solve_lp_dual_classified` | `solve_lp_dual_full_status` | simplex.rs |
| `Tableau::n_orig()` | `Tableau::n_structural()` | simplex.rs / extended_dual.rs |
| `update_verify` | `pivot_values_agree` | simplex.rs / extended_dual.rs |
| `sl_*` / `sl_in` | `shortlist_*` / `in_shortlist` | extended_dual.rs |
| `_iter` | `iter_idx` | extended_dual.rs |
| `gi_candidates` | `greatest_improvement_cands` | extended_dual.rs |
| `fd_cb` / `fd_y` | `fresh_d_cb` / `fresh_d_y` | extended_dual.rs |
| `cm_off` / `cm_pos` / `price_cm` / `pcm` | `col_entry_start` / `price_pos_of_col_entry` / `col_entry_of_price` / `entry_of_price` | extended_dual.rs |
| `drift_r0` | `drift_resid_after_refactor` | extended_dual.rs |
| `n_zero_nb` | `n_zero_nonbasic` | extended_dual.rs |
| `MarkowitzState::colval` / `prof_limit` | `col_value_cache` / `prof_search_limit_hits` | lu.rs |
| `ineqsingleton::run` | `ineqsingleton::resolve_inequality_singletons` | presolve |
| `dualpropagate::run` | `dualpropagate::propagate_dual_bounds` | presolve |
| `dualfix::fix_dominated_variables` | `dualfix::fix_by_lock_count` | presolve |
| `propagate::propagate_nog` | `propagate::propagate_without_g_rebuild` | presolve |
| `true_inf` / `true_sup` | `min_activity` / `max_activity` | propagate.rs |
| `no_candidate` | `inputs_pass_through_unchanged` | doubleton.rs |
| `skip_a` / `skip_g` | `weak_pivot_in_a` / `weak_pivot_in_g` | freevar.rs |
| `parallelcols::Substitution::{lb, ub}` | `var_lb` / `var_ub` | parallelcols.rs |
| `sparsify::Src` | `RowSource` | sparsify.rs |
| `sparse::Csr` (faer の型別名) | `FaerCsr` | crate 全体 |
| 自由関数 `mat_vec` / `mat_vec_into` / `mat_t_vec` / `mat_t_vec_into` (faer Csr 用) | `csr_mat_vec` / `csr_mat_vec_into` / `csr_mat_t_vec` / `csr_mat_t_vec_into` | sparse.rs / interior_point |
| `Compressed::group` | `outer_slice` | sparse.rs |
| interior_point の `dsa` / `dza` / `pk` / `dk` など | `ds_aff` / `dz_aff` / `primal_res_inf` / `dual_res_inf` など | interior_point.rs |
