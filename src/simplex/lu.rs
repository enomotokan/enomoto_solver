//! From-scratch sparse LU factorization of a (square) basis matrix, using
//! **Markowitz pivoting with bucket-based degree management**: among
//! numerically-acceptable pivot candidates (`|a_ij| >= stability *
//! max(|a_i'j|)` over the still-active rows i' of column j — the usual
//! "threshold pivoting" stability floor), the one minimizing the Markowitz
//! count `(row_nnz - 1) * (col_nnz - 1)` is chosen, i.e. sparsity
//! (fill-in) is prioritized over picking the numerically largest entry,
//! subject to that stability floor.
//!
//! This produces `P_row B P_col = L U` (`L` unit lower triangular, `U`
//! upper triangular, both stored in *elimination-step* order — step `s`'s
//! pivot row/column are `row_perm[s]`/`col_perm[s]` in the original basis
//! matrix's indexing).
//!
//! **Degree-list implementation**: row/column degrees (active nonzero
//! counts) live in bucket arrays (`col_buckets[d]`/`row_buckets[d]`, each a
//! `VecDeque` of indices currently at degree `d`), with O(1) bucket moves
//! via a parallel position index (`col_bucket_pos`/`row_bucket_pos`,
//! swap-to-last-then-pop on removal — the same pattern `factorize`'s
//! earlier `active_rows` bookkeeping used). Crucially, a **column-major
//! mirror** (`col_rows[j]`: the exact set of currently-active rows with a
//! nonzero at column `j`) is maintained alongside the row-major `rows`
//! matrix, kept in sync on every insert/remove during elimination. This is
//! what makes the whole scheme actually sub-`O(m)` per step rather than
//! just relocating the same cost: "which rows does eliminating column `pj`
//! affect" is answered by `col_rows[pj]` directly (cost = that column's own
//! current degree) instead of scanning every active row to test
//! `contains_key(&pj)`, and "how many rows still touch column `j`" is
//! `col_rows[j].len()` (O(1)) instead of a fresh full-matrix scan. An
//! earlier version of this file computed row/column degrees this way but
//! then performed the actual elimination arithmetic *directly* in
//! `factorize`'s main loop, ahead of a separate `eliminate_column` method
//! that was supposed to update the bucket state — since that method
//! detected "which rows changed" via `contains_key(&pj)`, and the earlier
//! direct arithmetic had already removed `pj` from every affected row
//! first, `eliminate_column` always found nothing to do. Bucket degrees
//! then stayed frozen at their *initial* values for the rest of the
//! factorization while the underlying matrix kept changing underneath
//! them, which didn't corrupt the arithmetic (pivot values are always read
//! fresh from `rows`) but could starve `find_best_pivot` of a candidate it
//! should have found, surfacing as a spurious "singular" `None` on
//! matrices that are not actually singular (confirmed: this crate's own
//! HiGHS cross-check benchmark, which the prior, non-bucketed `factorize`
//! solved without issue, started panicking at `n=2000` with exactly that
//! message). The elimination here is a single method (`eliminate`) that
//! does the arithmetic *and* the degree/bucket bookkeeping together, and a
//! row being retired as a pivot removes it from every other column's
//! `col_rows` set too (not just its own pivot column's), so no column's
//! degree can drift stale by continuing to count an inactive row.
//!
//! Within a factorization, per-pivot cost is `O(col_rows[pj].len())` for
//! the elimination itself plus `O(fill touched)` for the resulting
//! degree/`col_max_abs` refresh — bounded by actual sparsity rather than
//! `m` — though `find_best_pivot`'s bucket scan can still fall back to
//! examining more candidates than that on a poorly-conditioned or unusually
//! dense step; it is not a hard worst-case guarantee, just a much smaller
//! constant than rescanning the whole active submatrix every step.
//!
//! Incremental Forrest-Tomlin updates (`ft_update`, in this same file)
//! exist specifically so a full run of this factorization is only needed
//! occasionally, not on every basis change — see `simplex.rs`'s
//! refactorization-trigger docs.
//!
//! Solving against the resulting factors is mostly a `for s in 0..m`
//! dense scan with a per-step zero-skip (`l_solve_into` and friends) —
//! except FTRAN's own entering-column solve, which instead uses a real
//! Gilbert & Peierls (1988)-style sparse forward substitution
//! (`LuFactors::l_solve_sparse_into`, via `GpScratch`'s persistent,
//! epoch-stamped DFS scratch) — see that function's own docs for why only
//! this one direction gets the fuller treatment.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};

const STABILITY: f64 = 0.1;
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
const DENSE_COL_FRACTION: f64 = 0.5;

// Measurement counters for the "should `factorize` triangularize `A_B`
// into a trivial part plus a smaller Markowitz bump before factoring, the
// way production codes like HiGHS do" question, read back by
// `simplex.rs`'s `ENOMOTO_PROF_TRIANGULAR`-gated diagnostic. Answered by
// measurement rather than by adding the pre-pass speculatively: on every
// Netlib instance checked (`ganges`, `ship12s`, `stocfor2`, `fit1p`),
// `find_best_pivot`'s bucket-based early exit already resolves 90-100% of
// pivots as score-0 "trivial" ones, and cumulative time inside
// `find_best_pivot` across the *entire* solve was under 0.2% of total
// solve time in every case — the pivot *search* was never the bottleneck
// a dedicated triangularization pre-pass would speed up, so one was not
// added. Kept as a live diagnostic (not deleted) in case a future problem
// shape changes that picture.
pub(crate) static PROF_TOTAL_STEPS: AtomicUsize = AtomicUsize::new(0);
pub(crate) static PROF_TRIVIAL_STEPS: AtomicUsize = AtomicUsize::new(0);
pub(crate) static PROF_BUCKET_SCAN_NS: AtomicUsize = AtomicUsize::new(0);
/// How many elimination steps had to fall back to `find_best_pivot(false)`
/// because every remaining candidate was `initially_dense` — a direct
/// measurement of how often the dense-column-avoidance heuristic in
/// `factorize` actually gets exercised (as opposed to every dense column
/// simply never coming up as a candidate at all, in which case this stays
/// at `0` and the heuristic is a no-op for that problem).
pub(crate) static PROF_DENSE_FALLBACK_STEPS: AtomicUsize = AtomicUsize::new(0);

/// Manages the active submatrix plus row/column degrees (via bucket
/// arrays) during Markowitz elimination — see the module docs for why a
/// column-major `col_rows` mirror alongside the row-major `rows` matrix is
/// what actually keeps this sub-`O(m)` per step, and why the elimination
/// and degree bookkeeping must happen in one place rather than two.
struct MarkowitzState {
    #[allow(dead_code)]
    m: usize,

    // Active row-major submatrix.
    rows: Vec<BTreeMap<usize, f64>>,
    // Column-major mirror: col_rows[j] = the set of currently-active rows
    // with a nonzero at column j. Kept in exact sync with `rows` by every
    // method below — this is what lets column-degree lookups and
    // "who else has a nonzero here" queries stay O(that column's own
    // degree) instead of O(m).
    col_rows: Vec<BTreeSet<usize>>,

    // Current degrees (active nonzero counts), mirrored by bucket
    // placement below.
    col_degree: Vec<usize>,
    row_degree: Vec<usize>,

    // Bucket arrays: bucket[d] = indices currently at degree exactly d.
    // Sized m + 1 (a degree can be at most the number of active rows/cols).
    col_buckets: Vec<VecDeque<usize>>,
    row_buckets: Vec<VecDeque<usize>>,

    // col_bucket_pos[j] = j's index within col_buckets[col_degree[j]], or
    // None if j has been used already (removed from every bucket).
    col_bucket_pos: Vec<Option<usize>>,
    row_bucket_pos: Vec<Option<usize>>,

    col_used: Vec<bool>,
    row_used: Vec<bool>,

    // Column max absolute values, over that column's own active rows only
    // (via col_rows) — the threshold-pivoting stability reference.
    col_max_abs: Vec<f64>,

    /// `initially_dense[j]` iff column `j`'s degree *before any
    /// elimination* exceeded `DENSE_COL_FRACTION * m` — fixed at
    /// construction time and never updated, deliberately: a truly dense
    /// column's *current* degree keeps shrinking as unrelated rows get
    /// eliminated as pivots for *other*, sparser columns (each such row
    /// leaving the basis removes it from every column's `col_rows`,
    /// including this one's) — dropping into a low bucket only because
    /// its rows happened to get cannibalized elsewhere, not because it
    /// stopped being structurally dense. Thresholding on the live,
    /// shrinking degree would let `find_best_pivot`'s ordinary ascending-
    /// bucket scan pick such a column early anyway, right when it looks
    /// artificially sparse — exactly the case this field exists to still
    /// catch. See `find_best_pivot`'s own docs for what this avoids: a
    /// pivot on a column with `d` remaining active rows scatters the
    /// entire pivot row's pattern into all `d` of them in one step
    /// (`eliminate`'s `affected_rows`), so pivoting on a column that is
    /// dense *by original structure* — even at a reduced current degree —
    /// is still the single most expensive kind of step Markowitz pivoting
    /// can take.
    initially_dense: Vec<bool>,
}

impl MarkowitzState {
    fn new(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Self {
        let rows: Vec<BTreeMap<usize, f64>> = rows_in
            .iter()
            .map(|r| {
                let mut h = BTreeMap::new();
                for &(j, v) in r {
                    *h.entry(j).or_insert(0.0) += v;
                }
                h.retain(|_, v| *v != 0.0);
                h
            })
            .collect();

        let mut col_rows: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); m];
        let mut col_degree = vec![0usize; m];
        let mut col_max_abs = vec![0.0f64; m];
        let mut row_degree = vec![0usize; m];

        for i in 0..m {
            row_degree[i] = rows[i].len();
            for (&j, &v) in &rows[i] {
                col_rows[j].insert(i);
                col_max_abs[j] = col_max_abs[j].max(v.abs());
            }
        }
        for j in 0..m {
            col_degree[j] = col_rows[j].len();
        }

        let mut col_buckets = vec![VecDeque::new(); m + 1];
        let mut row_buckets = vec![VecDeque::new(); m + 1];
        let mut col_bucket_pos = vec![None; m];
        let mut row_bucket_pos = vec![None; m];

        for j in 0..m {
            let deg = col_degree[j];
            col_bucket_pos[j] = Some(col_buckets[deg].len());
            col_buckets[deg].push_back(j);
        }
        for i in 0..m {
            let deg = row_degree[i];
            row_bucket_pos[i] = Some(row_buckets[deg].len());
            row_buckets[deg].push_back(i);
        }

        let dense_threshold = DENSE_COL_FRACTION * m as f64;
        let initially_dense: Vec<bool> = col_degree.iter().map(|&d| d as f64 > dense_threshold).collect();

        MarkowitzState {
            m,
            rows,
            col_rows,
            col_degree,
            row_degree,
            col_buckets,
            row_buckets,
            col_bucket_pos,
            row_bucket_pos,
            col_used: vec![false; m],
            row_used: vec![false; m],
            col_max_abs,
            initially_dense,
        }
    }

    /// Remove an item from bucket[deg] and update position tracking.
    fn remove_from_bucket_col(&mut self, j: usize) {
        if let Some(pos) = self.col_bucket_pos[j] {
            let deg = self.col_degree[j];
            let bucket = &mut self.col_buckets[deg];
            if pos < bucket.len() {
                let last_j = bucket.pop_back().unwrap();
                if pos < bucket.len() {
                    bucket[pos] = last_j;
                    self.col_bucket_pos[last_j] = Some(pos);
                }
            }
            self.col_bucket_pos[j] = None;
        }
    }

    fn remove_from_bucket_row(&mut self, i: usize) {
        if let Some(pos) = self.row_bucket_pos[i] {
            let deg = self.row_degree[i];
            let bucket = &mut self.row_buckets[deg];
            if pos < bucket.len() {
                let last_i = bucket.pop_back().unwrap();
                if pos < bucket.len() {
                    bucket[pos] = last_i;
                    self.row_bucket_pos[last_i] = Some(pos);
                }
            }
            self.row_bucket_pos[i] = None;
        }
    }

    /// Update degree after modifying; move between buckets if needed.
    fn update_col_degree(&mut self, j: usize, new_deg: usize) {
        if self.col_used[j] || new_deg == self.col_degree[j] {
            return;
        }
        self.remove_from_bucket_col(j);
        self.col_degree[j] = new_deg;
        let pos = self.col_buckets[new_deg].len();
        self.col_bucket_pos[j] = Some(pos);
        self.col_buckets[new_deg].push_back(j);
    }

    fn update_row_degree(&mut self, i: usize, new_deg: usize) {
        if self.row_used[i] || new_deg == self.row_degree[i] {
            return;
        }
        self.remove_from_bucket_row(i);
        self.row_degree[i] = new_deg;
        let pos = self.row_buckets[new_deg].len();
        self.row_bucket_pos[i] = Some(pos);
        self.row_buckets[new_deg].push_back(i);
    }

    /// Recomputes `col_max_abs[j]` and its degree/bucket placement from
    /// its current `col_rows[j]` membership — O(that column's own active
    /// degree), never O(m).
    fn refresh_column(&mut self, j: usize) {
        if self.col_used[j] {
            return;
        }
        let new_deg = self.col_rows[j].len();
        self.update_col_degree(j, new_deg);
        self.col_max_abs[j] =
            self.col_rows[j].iter().filter_map(|&i| self.rows[i].get(&j).map(|v| v.abs())).fold(0.0, f64::max);
    }

    /// Find best pivot: among still-active columns in ascending-degree
    /// order, only that column's actual active rows (via `col_rows`, not
    /// every row at that row-degree) are examined — this is the other half
    /// (alongside `eliminate`'s use of `col_rows`) of what keeps the
    /// search from degrading into a full active-submatrix scan. The
    /// per-degree-level early exit is a standard practical relaxation (as
    /// in production Markowitz implementations): it does not guarantee the
    /// globally minimal Markowitz count, only that no further search will
    /// find something clearly better — finding the exact minimum every
    /// step is itself more expensive than the fill-in it would save.
    ///
    /// `skip_dense`: when true, every `initially_dense` column is skipped
    /// outright, regardless of its current (possibly much lower, per that
    /// field's own docs) degree or Markowitz score — `factorize`'s caller
    /// tries this first and only falls back to a second, unrestricted call
    /// if it finds nothing, so a truly-required dense pivot (or a genuinely
    /// singular matrix) is still handled correctly, just not preferred.
    fn find_best_pivot(&self, skip_dense: bool) -> Option<(usize, usize)> {
        let __prof_t0 = std::time::Instant::now();
        PROF_TOTAL_STEPS.fetch_add(1, Ordering::Relaxed);
        let mut best: Option<(usize, usize)> = None;
        let mut best_score = usize::MAX;
        let mut best_pivot_abs = 0.0f64;

        for deg_col in 1..self.col_buckets.len() {
            for &j in &self.col_buckets[deg_col] {
                if self.col_used[j] || (skip_dense && self.initially_dense[j]) {
                    continue;
                }
                for &i in &self.col_rows[j] {
                    if self.row_used[i] {
                        continue;
                    }
                    let Some(&v) = self.rows[i].get(&j) else { continue };
                    if v == 0.0 || v.abs() < STABILITY * self.col_max_abs[j] {
                        continue;
                    }
                    let score = (self.row_degree[i] - 1) * (self.col_degree[j] - 1);
                    if score < best_score || (score == best_score && v.abs() > best_pivot_abs) {
                        best_score = score;
                        best = Some((i, j));
                        best_pivot_abs = v.abs();
                    }
                }
                if best_score == 0 {
                    PROF_TRIVIAL_STEPS.fetch_add(1, Ordering::Relaxed);
                    PROF_BUCKET_SCAN_NS.fetch_add(__prof_t0.elapsed().as_nanos() as usize, Ordering::Relaxed);
                    return best;
                }
            }
            if best.is_some() && best_score <= deg_col * deg_col {
                PROF_BUCKET_SCAN_NS.fetch_add(__prof_t0.elapsed().as_nanos() as usize, Ordering::Relaxed);
                return best;
            }
        }

        PROF_BUCKET_SCAN_NS.fetch_add(__prof_t0.elapsed().as_nanos() as usize, Ordering::Relaxed);
        best
    }

    /// Eliminates column `pj` (whose pivot is `(pi, pj)`, value
    /// `pivot_val`) from every other active row, in one pass that performs
    /// both the Gaussian-elimination arithmetic and the matching degree/
    /// bucket updates — see the module docs for why splitting these two
    /// into separate steps (as an earlier version of this file did) is
    /// unsound: bookkeeping keyed off "did this row still contain `pj`"
    /// only works if it runs *before* `pj` is actually removed. Also
    /// retires row `pi` from every other column's `col_rows` set (not just
    /// column `pj`'s), so no column's degree can drift by continuing to
    /// count a row that is no longer active. Returns the `(row, multiplier)`
    /// pairs for `factorize`'s own `L` bookkeeping.
    fn eliminate(&mut self, pi: usize, pj: usize, pivot_val: f64, pivot_row_snapshot: &[(usize, f64)]) -> Vec<(usize, f64)> {
        let affected_rows: Vec<usize> = self.col_rows[pj].iter().copied().filter(|&i| i != pi).collect();
        let mut touched_cols: BTreeSet<usize> = BTreeSet::new();
        let mut l_out: Vec<(usize, f64)> = Vec::with_capacity(affected_rows.len());

        for i in affected_rows {
            let Some(&aij) = self.rows[i].get(&pj) else { continue };
            if aij == 0.0 {
                continue;
            }
            let mult = aij / pivot_val;
            l_out.push((i, mult));

            for &(j, v) in pivot_row_snapshot {
                if j == pj {
                    continue;
                }
                // A single `entry()` descent instead of the
                // `contains_key`+`get`+(`insert`|`remove`) sequence this
                // used to be — each of those is its own O(log d) BTreeMap
                // traversal to the *same* node, and this loop body is the
                // single hottest piece of the whole factorization (run
                // once per `(affected row, pivot-row entry)` pair, every
                // elimination step).
                use std::collections::btree_map::Entry;
                match self.rows[i].entry(j) {
                    Entry::Occupied(mut e) => {
                        let new_val = *e.get() - mult * v;
                        if new_val == 0.0 {
                            e.remove();
                            self.col_rows[j].remove(&i);
                        } else {
                            *e.get_mut() = new_val;
                        }
                        touched_cols.insert(j);
                    }
                    Entry::Vacant(e) => {
                        let new_val = -mult * v;
                        if new_val != 0.0 {
                            e.insert(new_val);
                            self.col_rows[j].insert(i);
                            touched_cols.insert(j);
                        }
                    }
                }
            }

            self.rows[i].remove(&pj);
            let new_deg_i = self.rows[i].len();
            self.update_row_degree(i, new_deg_i);
        }
        self.col_rows[pj].clear();

        // Row pi is retiring as the new pivot row; drop it from every
        // other column it still touches so those columns' degrees don't
        // keep counting an inactive row.
        let pi_cols: Vec<usize> = self.rows[pi].keys().copied().filter(|&j| j != pj).collect();
        for j in pi_cols {
            self.col_rows[j].remove(&pi);
            touched_cols.insert(j);
        }

        for j in touched_cols {
            self.refresh_column(j);
        }

        l_out
    }
}

/// Persistent, zero-allocation-in-steady-state scratch for
/// [`LuFactors::l_solve_sparse_into`] — one instance lives for as long as
/// its caller's own dedicated sparse-solve buffer does (`solve_lp_dual_on`
/// creates one, alongside a `z` buffer used *only* for this path — never
/// shared with a plain [`FtLu::solve_into`] call's own `scratch`, per
/// [`FtLu::solve_sparse_into`]'s own docs on why that separation matters
/// — before the pivot loop starts, and reuses both every FTRAN).
///
/// `visited_epoch[i] == epoch` means step `i` is already known to be in
/// the *current* call's reach set — bumping `epoch` each call instead of
/// clearing this array is what makes marking/checking `O(1)` without an
/// `O(m)` reset per call (the standard "epoch stamp" / "time stamp"
/// technique for a reusable visited-set). `stack` is the DFS's own
/// (iterative, not recursive — this crate's basis matrices can have `m`
/// in the low thousands, deep enough that a recursive DFS risks a real
/// stack overflow on a long dependency chain) working stack. `reach` is
/// this call's own collected, then sorted, reach set.
pub struct GpScratch {
    visited_epoch: Vec<u32>,
    epoch: u32,
    stack: Vec<usize>,
    seeds: Vec<usize>,
    reach: Vec<usize>,
}

impl GpScratch {
    pub fn new(m: usize) -> Self {
        GpScratch { visited_epoch: vec![0; m], epoch: 0, stack: Vec::new(), seeds: Vec::new(), reach: Vec::new() }
    }
}

#[derive(Clone)]
pub struct LuFactors {
    pub m: usize,
    /// `l_col[s]`: `(row_step, multiplier)` pairs — the sub-diagonal
    /// entries of `L`'s column `s`.
    pub l_col: Vec<Vec<(usize, f64)>>,
    /// `u_row[s]`: `(col_step, value)` pairs, `col_step >= s` (including
    /// the diagonal at `col_step == s`) — the entries of `U`'s row `s`.
    pub u_row: Vec<Vec<(usize, f64)>>,
    pub row_perm: Vec<usize>,
    pub col_perm: Vec<usize>,
    pub col_perm_inv: Vec<usize>,
    /// Inverse of `row_perm`: `row_perm_inv[orig_row]` is the step whose
    /// pivot row was `orig_row` — needed to seed [`l_solve_sparse_into`]'s
    /// reach-set search directly from a sparse (original-row-indexed)
    /// right-hand side, without an `O(m)` scan of `row_perm` itself.
    pub row_perm_inv: Vec<usize>,
}

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
pub fn factorize(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Option<LuFactors> {
    let mut state = MarkowitzState::new(m, rows_in);

    let mut row_perm = vec![0usize; m];
    let mut col_perm = vec![0usize; m];

    let mut l_entries: Vec<(usize, usize, f64)> = Vec::new();
    let mut u_entries: Vec<(usize, usize, f64)> = Vec::new();

    for step in 0..m {
        // Prefer a non-dense pivot column whenever one exists at all,
        // regardless of Markowitz score, and only fall back to the
        // unrestricted search (which also correctly reports a genuinely
        // singular matrix via `None`) once every remaining column is
        // `initially_dense` — see `find_best_pivot`'s and
        // `MarkowitzState::initially_dense`'s own docs for why avoiding a
        // dense pivot column matters far more than the score it happens
        // to carry at the moment it's chosen.
        let (pi, pj) = match state.find_best_pivot(true) {
            Some(p) => p,
            None => {
                PROF_DENSE_FALLBACK_STEPS.fetch_add(1, Ordering::Relaxed);
                state.find_best_pivot(false)?
            }
        };

        state.row_used[pi] = true;
        state.col_used[pj] = true;
        row_perm[step] = pi;
        col_perm[step] = pj;

        state.remove_from_bucket_row(pi);
        state.remove_from_bucket_col(pj);

        let pivot_val = *state.rows[pi].get(&pj).unwrap();
        let pivot_row_snapshot: Vec<(usize, f64)> = state.rows[pi]
            .iter()
            .filter(|&(&j, &v)| v != 0.0 && (j == pj || !state.col_used[j]))
            .map(|(&j, &v)| (j, v))
            .collect();

        for &(j, v) in &pivot_row_snapshot {
            u_entries.push((step, j, v));
        }

        for (i, mult) in state.eliminate(pi, pj, pivot_val, &pivot_row_snapshot) {
            l_entries.push((i, step, mult));
        }
    }

    let mut row_perm_inv = vec![0usize; m];
    let mut col_perm_inv = vec![0usize; m];
    for step in 0..m {
        row_perm_inv[row_perm[step]] = step;
        col_perm_inv[col_perm[step]] = step;
    }

    let mut l_col: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for (orig_row, pivot_step, mult) in l_entries {
        l_col[pivot_step].push((row_perm_inv[orig_row], mult));
    }
    let mut u_row: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for (pivot_step, orig_col, val) in u_entries {
        u_row[pivot_step].push((col_perm_inv[orig_col], val));
    }

    Some(LuFactors { m, l_col, u_row, row_perm, col_perm, col_perm_inv, row_perm_inv })
}

impl LuFactors {
    #[allow(dead_code)]
    fn u_diag(&self, step: usize) -> f64 {
        self.u_row[step]
            .iter()
            .find(|&&(c, _)| c == step)
            .map(|&(_, v)| v)
            .unwrap_or(0.0)
    }

    /// Partial FTRAN through `L` only (step-space): solves `L z = P_row rhs`.
    ///
    /// Hyper-sparse (Hall & McKinnon, *"Hyper-sparsity in the revised
    /// simplex method and how to exploit it"*, 2000, §4.2 "Hyper-sparse
    /// FTRAN", Figure 3): `l_col[s]`'s entries only ever modify `z` by
    /// adding a multiple of `z[s]` itself — if `z[s]` is exactly zero, the
    /// whole inner loop is a provable no-op (every update is `x -= mult *
    /// 0`), so it is skipped entirely rather than paying for a test-against-
    /// zero (or worse, a real floating point op) per entry.
    /// Writes the result into caller-provided `z` (length `m`) instead of
    /// allocating — `FtLu`'s hot-path `solve_into` calls this once per
    /// FTRAN, so a fresh `Vec` here would mean a fresh heap allocation on
    /// every single pivot's FTRAN/BTRAN, several times over (see
    /// `FtLu::solve_into`'s own docs).
    fn l_solve_into(&self, rhs: &[f64], z: &mut [f64]) {
        let m = self.m;
        for s in 0..m {
            z[s] = rhs[self.row_perm[s]];
        }
        for s in 0..m {
            if z[s] == 0.0 {
                continue;
            }
            for &(row_step, mult) in &self.l_col[s] {
                z[row_step] -= mult * z[s];
            }
        }
    }

    /// Gilbert-Peierls sparse forward substitution through `L`: given
    /// `rhs`'s nonzero `(orig_row, value)` pairs directly (no `O(m)`
    /// densification of the caller's own sparse column needed), computes
    /// the *reach set* — every step whose `z` entry could possibly end up
    /// nonzero — via a DFS over `l_col`'s step-to-step edges, then runs
    /// exactly [`l_solve_into`]'s own elimination but restricted to that
    /// set. A second, dense-scanning implementation of this exists
    /// ([`l_solve_into`]) rather than making this the only one because a
    /// dense `rhs` (this function's own worst case: `|reach| == m`) pays
    /// for the DFS bookkeeping (stack pushes, epoch checks) on top of the
    /// same elimination work `l_solve_into` would have done anyway with a
    /// tight double loop — this function is a net win specifically when
    /// `rhs` (and hence typically `reach`) is small relative to `m`, which
    /// is the common case for the one caller that has a genuinely sparse
    /// `rhs` on hand already (`solve_lp_dual_on`'s entering-column FTRAN:
    /// a real LP's constraint columns are themselves sparse).
    ///
    /// **Why ascending numeric order is already a valid topological
    /// order** (unlike the general Gilbert & Peierls 1988 presentation for
    /// an arbitrary DAG, which needs a DFS-postorder-then-reverse to get
    /// one): every `l_col[s]` entry's `row_step` is `> s`, by construction
    /// of the elimination itself (`factorize` only ever records a
    /// multiplier for a row not yet chosen as a pivot, which by definition
    /// gets assigned some *later* step) — so the edges of this graph only
    /// ever point from a lower step to a higher one, meaning simply
    /// sorting the reach set ascending already respects every dependency,
    /// with no need to track a separate visit order during the DFS itself.
    ///
    /// **Precondition**: `z` is entirely zero on entry. This is *not*
    /// this function's own job to (re-)establish cheaply — its own reach
    /// set only covers what the `L`-stage itself touches, but the R-eta
    /// and `U` stages downstream (in [`FtLu::solve_sparse_into`]) can
    /// scatter fill well beyond that set (a long-enough eta chain can, in
    /// the worst case, touch entries across the whole vector), so knowing
    /// "the previous call's `L`-stage reach" here would not be enough to
    /// correctly re-zero what a *subsequent* stage left behind. Instead
    /// [`FtLu::solve_sparse_into`] unconditionally clears its own
    /// dedicated `z` buffer once, in full, right before returning — a
    /// single `O(m)` `fill(0.0)` per call, far cheaper than the branchy
    /// permute-and-scan `l_solve_into` otherwise pays, and the only
    /// `O(m)` work left in the whole sparse path.
    fn l_solve_sparse_into(&self, rhs_sparse: &[(usize, f64)], z: &mut [f64], scratch: &mut GpScratch) {
        scratch.seeds.clear();
        for &(orig_row, v) in rhs_sparse {
            if v == 0.0 {
                continue;
            }
            let s = self.row_perm_inv[orig_row];
            z[s] = v;
            scratch.seeds.push(s);
        }

        scratch.epoch += 1;
        let epoch = scratch.epoch;
        scratch.reach.clear();
        for i in 0..scratch.seeds.len() {
            let seed = scratch.seeds[i];
            if scratch.visited_epoch[seed] == epoch {
                continue;
            }
            scratch.visited_epoch[seed] = epoch;
            scratch.stack.push(seed);
            while let Some(node) = scratch.stack.pop() {
                scratch.reach.push(node);
                for &(next, _) in &self.l_col[node] {
                    if scratch.visited_epoch[next] != epoch {
                        scratch.visited_epoch[next] = epoch;
                        scratch.stack.push(next);
                    }
                }
            }
        }
        scratch.reach.sort_unstable();

        for &s in &scratch.reach {
            if z[s] == 0.0 {
                continue;
            }
            for &(row_step, mult) in &self.l_col[s] {
                z[row_step] -= mult * z[s];
            }
        }
    }

    /// Finishes a BTRAN given a step-space vector already transformed by
    /// `U^{-T}`: applies `L^{-T}` and maps back to original row indices.
    ///
    /// Unlike `l_solve`'s forward pass, a single step `s` here can read
    /// from *several* `w[row_step]` entries (one per `l_col[s]` entry), so
    /// there is no single value whose zero-ness makes the whole step a
    /// no-op — matching Hall & McKinnon §4.4's observation that BTRAN's
    /// inner-product-shaped work has "no simple way of determining [a
    /// trivial] intersection... without a computational overhead
    /// comparable to evaluating the inner product itself". Skipping
    /// per-*entry* when that specific `w[row_step]` is zero is still safe
    /// and free, just a smaller win than `l_solve`'s whole-step skip.
    ///
    /// (A first attempt at a fuller DFS-based hyper-sparse implementation,
    /// covering all four solve directions and mirroring `L`/`U` both
    /// column- and row-major the way HiGHS does, was tried and measured
    /// *slower* end to end: the DFS setup's own per-call cost —
    /// allocating a fresh `visited` array plus an upfront `O(m)` density
    /// scan on every single call, even ones that ended up taking the
    /// dense-style branch — outweighed the fill-skipping it bought, on
    /// the order of 15-18% slower overall. That attempt was reverted in
    /// full. A second, narrower attempt — [`LuFactors::l_solve_sparse_into`],
    /// covering only this module's one genuinely straightforward GP
    /// setting (`L`'s own forward direction, already stored column-major,
    /// fed a real LP's own sparse constraint column) with a *persistent*,
    /// epoch-stamped scratch (see [`GpScratch`]) rather than a fresh
    /// per-call allocation — measured as a small but real net win on the
    /// full Netlib benchmark set (73 problems, aggregate wall time ~1%
    /// lower, roughly even split of individually-faster/slower instances,
    /// zero objective mismatches) once the specific cost the first
    /// attempt's own revert blamed — the allocation, not the algorithm —
    /// was actually removed. This `L^{-T}` direction (BTRAN's tail) was
    /// deliberately *not* attempted a second time: Hall & McKinnon's
    /// observation above still applies unchanged (no static column-major
    /// structure of `L` to run the same DFS over without adding a
    /// row-major mirror), and the first attempt's win was concentrated in
    /// the one direction with a genuinely sparse, already-available
    /// seed — this direction's own `w` typically isn't.)
    /// `w` (step-space, already past `U^{-T}`/the `R` etas) is mutated in
    /// place; the final result is written into caller-provided `y`
    /// (original row indexing) — see `l_solve_into`'s own docs for why
    /// this avoids allocating on `FtLu`'s hot path.
    fn l_transpose_solve_into(&self, w: &mut [f64], y: &mut [f64]) {
        let m = self.m;
        for s in (0..m).rev() {
            for &(row_step, mult) in &self.l_col[s] {
                if w[row_step] == 0.0 {
                    continue;
                }
                w[s] -= mult * w[row_step];
            }
        }
        for s in 0..m {
            y[self.row_perm[s]] = w[s];
        }
    }

    /// Solves `B x = rhs` using the factors (`P_row B P_col = LU`).
    #[allow(dead_code)]
    pub fn solve(&self, rhs: &[f64]) -> Vec<f64> {
        let m = self.m;
        // rhs' = P_row rhs
        let mut z: Vec<f64> = (0..m).map(|s| rhs[self.row_perm[s]]).collect();
        // Forward: L z = rhs' (unit lower triangular, step order)
        for s in 0..m {
            for &(row_step, mult) in &self.l_col[s] {
                z[row_step] -= mult * z[s];
            }
        }
        // Back: U x' = z
        let mut xp = vec![0.0; m];
        for s in (0..m).rev() {
            let mut acc = z[s];
            for &(col_step, v) in &self.u_row[s] {
                if col_step != s {
                    acc -= v * xp[col_step];
                }
            }
            xp[s] = acc / self.u_diag(s);
        }
        // x[col_perm[s]] = xp[s]
        let mut x = vec![0.0; m];
        for s in 0..m {
            x[self.col_perm[s]] = xp[s];
        }
        x
    }

    /// Solves `B^T y = rhs`.
    #[allow(dead_code)]
    pub fn solve_transpose(&self, rhs: &[f64]) -> Vec<f64> {
        let m = self.m;
        // rhs2 = P_col^-1 rhs, i.e. rhs2[s] = rhs[col_perm[s]]
        let mut z: Vec<f64> = (0..m).map(|s| rhs[self.col_perm[s]]).collect();
        // Forward: U^T z2 = rhs2 (lower triangular in step order). Row-wise
        // storage of U means column-wise access (needed for a textbook
        // forward substitution) isn't available, so instead this scatters:
        // by the time step `s` is processed, `z[s]` already holds
        // `rhs2[s]` minus every contribution from steps `k < s` (each such
        // `k` scattered `-U[k,s] * z[k]` into `z[s]` when `k` was
        // processed), so dividing by the diagonal solves for `z[s]`, which
        // is then scattered forward into `z[col_step]` for `col_step > s`.
        for s in 0..m {
            z[s] /= self.u_diag(s);
            for &(col_step, v) in &self.u_row[s] {
                if col_step != s {
                    z[col_step] -= v * z[s];
                }
            }
        }
        // Back: L^T w = z (unit upper triangular in step order)
        let mut w = z;
        for s in (0..m).rev() {
            for &(row_step, mult) in &self.l_col[s] {
                w[s] -= mult * w[row_step];
            }
        }
        // y[row_perm[s]] = w[s]
        let mut y = vec![0.0; m];
        for s in 0..m {
            y[self.row_perm[s]] = w[s];
        }
        y
    }

}

// ---------------------------------------------------------------------
// Forrest-Tomlin incremental update
// ---------------------------------------------------------------------
//
// Following Forrest, J.J.H. and Tomlin, J.A., "Updated triangular factors
// of the basis to maintain sparsity in the product form simplex method",
// Mathematical Programming 2 (1972), 263-278, as summarized precisely
// with full derivations in Huangfu, Q. and Hall, J.A.J., "Novel update
// techniques for the revised simplex method", Technical Report
// ERGO-13-001, University of Edinburgh (2013) §2.1 (equations 1, 4-13).
//
// Column replacement `B̄ = B + (a_q - B e_p) e_p^T` is rearranged via the
// fixed factorization `B = LU` as
//   `L^{-1} B̄ = U + (L^{-1}a_q - U e_p) e_p^T = U + (ã_q - u_p) e_p^T = U'`
// replacing column `p` of `U` with the partial FTRAN result
// `ã_q = L^{-1} a_q`. This "spikes" column `p` of `U` (rows > p can now
// be nonzero, breaking triangularity). Triangularity is restored by one
// row transformation `R^{-1} = I - e_p r^T` that zeros row `p` across
// every column: `Ū = R^{-1}U'`, where `r^T = ū_p^T U^{-1}` (`ū_p` = row
// `p` of `U` without its diagonal) can be obtained at negligible cost
// from `ẽ_p^T = e_p^T U^{-1}` (a partial BTRAN already computed to derive
// `r`) as `r = -u_pp · ẽ_p` with the `p`-th entry forced to zero
// (Tomlin 1974, eq. 12 in the 2013 paper). `R^{-1}` applied to column `p`
// of `U'` only changes its `p`-th entry: `ã_pq := ã_pq - r·ã_q`.
//
// `L` never changes across updates. `U` is kept as a *sequence* of
// per-slot column etas (pivot + off-diagonal vector), because after a
// replacement the slot's eta is removed from wherever it sits and
// *appended* to the end — this ordering, not raw slot order, is what
// FTRAN/BTRAN through `U` must respect once updates have happened
// (verified by hand against a direct dense re-solve while implementing
// this). Each update additionally produces one `R` row-eta, kept in its
// own creation-ordered list and applied between `L` and `U` per
// `B_k^{-1} = U_k^{-1} R_k^{-1} ... R_1^{-1} L^{-1}` (eq. 13).

#[derive(Clone)]
struct UEta {
    slot: usize,
    pivot: f64,
    off_diag: Vec<(usize, f64)>, // (row_step, value), row_step != slot
}

#[derive(Clone)]
struct REta {
    p: usize,
    r: Vec<(usize, f64)>, // (row_step, value), row_step != p
}

#[derive(Clone)]
pub struct FtLu {
    base: LuFactors,
    u_seq: Vec<UEta>,
    r_etas: Vec<REta>,
}

impl FtLu {
    pub fn new(base: LuFactors) -> Self {
        let m = base.m;
        let mut off_diags: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
        let mut pivots = vec![0.0; m];
        for row_step in 0..m {
            for &(col_step, v) in &base.u_row[row_step] {
                if col_step == row_step {
                    pivots[col_step] = v;
                } else {
                    off_diags[col_step].push((row_step, v));
                }
            }
        }
        let u_seq: Vec<UEta> = (0..m)
            .map(|slot| UEta { slot, pivot: pivots[slot], off_diag: std::mem::take(&mut off_diags[slot]) })
            .collect();
        FtLu {
            base,
            u_seq,
            r_etas: Vec::new(),
        }
    }

    fn find_seq_pos(&self, slot: usize) -> usize {
        self.u_seq.iter().position(|e| e.slot == slot).expect("slot must be present in the U sequence")
    }

    /// `U_k^{-T}` applied in place to a step-space vector: processes the
    /// eta sequence in **forward** (creation) order, each step solving for
    /// that eta's pivotal component via eq. (8). Mutates `z` directly
    /// (rather than allocating a fresh result) — see `LuFactors::l_solve_into`'s
    /// own docs for why this matters on `FtLu`'s hot path.
    fn u_transpose_solve_into(&self, z: &mut [f64]) {
        for eta in &self.u_seq {
            let p = eta.slot;
            let y: f64 = eta.off_diag.iter().map(|&(row_step, v)| v * z[row_step]).sum();
            z[p] = (z[p] - y) / eta.pivot;
        }
    }


    /// `U_k^{-1}` applied in place: processes the eta sequence in
    /// **reverse** order, each step solving via eq. (7). Hyper-sparse: same
    /// skip as `LuFactors::l_solve_into` — `xp` is the only value this
    /// eta's off-diagonal entries get multiplied by, so a zero `xp` makes
    /// the whole inner loop a provable no-op.
    ///
    /// (A GP-sparsified counterpart to this function — restricting the
    /// scan to a DFS-computed reach set over `u_seq`'s own dependency
    /// graph, exactly mirroring [`LuFactors::l_solve_sparse_into`]'s own
    /// approach for `L` — was fully implemented, proven correct (an
    /// inductive argument that every `off_diag` target always sits at a
    /// strictly *lower* `u_seq` position than its referrer, mirroring
    /// `L`'s own low-to-high property, so descending position order needs
    /// no separate topological-sort step either) and tested (multiple
    /// sequential `try_update` calls reordering `u_seq` non-trivially,
    /// checked against the dense reference after every single one). It
    /// was still reverted after measuring it on the full Netlib benchmark
    /// set: aggregate wall time **+10.4%** versus `L`-only sparsification,
    /// 52 of 73 problems slower and only 6 faster. Unlike `L` (a *static*
    /// matrix, fixed once per full refactorization, whose seed — a real
    /// LP's own sparse constraint column — is reliably sparse), `U`'s own
    /// eta chain accumulates fill from every `try_update` since the last
    /// refactorization, so its reach set is typically far less sparse in
    /// practice — the DFS/reach-tracking overhead this function's outer
    /// loop is cheap enough to not need in the first place stopped paying
    /// for itself. See this file's own history if revisiting this.)
    fn u_solve_into(&self, x: &mut [f64]) {
        for eta in self.u_seq.iter().rev() {
            let p = eta.slot;
            x[p] /= eta.pivot;
            let xp = x[p];
            if xp == 0.0 {
                continue;
            }
            for &(row_step, v) in &eta.off_diag {
                x[row_step] -= v * xp;
            }
        }
    }

    /// `R_k^{-1} ... R_1^{-1} L^{-1}` applied to a vector in original row
    /// indexing, written into caller-provided `z` (step-space) — i.e.
    /// everything `solve_into` does except the final `U_k^{-1}`. This is
    /// also exactly what a new update needs to turn `a_q` into `ã_q`: per
    /// eq. (11), `ã_q` must be `(L R_1 ... R_{k-1})^{-1} a_q`, *not* just
    /// `L^{-1} a_q` — the existing `R`s are already part of the "L-like"
    /// fixed factor that update `k` treats as known, since `B_{k-1} = L R_1
    /// ... R_{k-1} U_{k-1}` (eq. 13) rather than `B_{k-1} = L U_{k-1}` once
    /// `k > 1`.
    fn ftran_through_l_and_r_into(&self, rhs: &[f64], z: &mut [f64]) {
        self.base.l_solve_into(rhs, z);
        for reta in &self.r_etas {
            let dot: f64 = reta.r.iter().map(|&(i, v)| v * z[i]).sum();
            z[reta.p] -= dot;
        }
    }

    /// Allocating convenience wrapper around [`Self::ftran_through_l_and_r_into`]
    /// — only `try_update` still needs an owned result; see
    /// [`Self::u_transpose_solve`]'s own docs for why that call site is left
    /// allocating rather than threaded through with a buffer too.
    fn ftran_through_l_and_r(&self, rhs: &[f64]) -> Vec<f64> {
        let mut z = vec![0.0; self.base.m];
        self.ftran_through_l_and_r_into(rhs, &mut z);
        z
    }

    /// Writes `B^-1 rhs` into `out` (length `m`), using `scratch` (also
    /// length `m`) as working space — no allocation. `solve_lp_dual_on`
    /// calls this 2-4 times *every pivot* (BTRAN-DSE's `tau`, the entering
    /// column's `alpha`, and, when BFRT flips are pending, one more for
    /// `combined`), so the 2-3 `Vec` allocations each fresh `solve()` call
    /// used to cost here (one each in `l_solve`, `u_solve`, and the final
    /// permutation) were real, repeated per-iteration heap traffic —
    /// eliminated by having the caller own `scratch`/`out` once, outside
    /// the iteration loop, and reuse them every pivot.
    pub fn solve_into(&self, rhs: &[f64], scratch: &mut [f64], out: &mut [f64]) {
        self.ftran_through_l_and_r_into(rhs, scratch);
        self.u_solve_into(scratch);
        for s in 0..self.base.m {
            out[self.base.col_perm[s]] = scratch[s];
        }
    }

    /// Sparse-`rhs` counterpart to [`Self::solve_into`]: the same
    /// `B^-1 rhs` computation, but taking `rhs`'s nonzero
    /// `(orig_row, value)` pairs directly and running the `L`-stage
    /// through [`LuFactors::l_solve_sparse_into`] instead of densifying
    /// `rhs` into `scratch` first — see that function's own docs for the
    /// reach-set algorithm.
    ///
    /// **`scratch`/`gp` must be dedicated to this call site alone, never
    /// shared with a plain [`Self::solve_into`] call's own buffer**:
    /// `l_solve_into` (the dense path) starts by unconditionally
    /// overwriting every entry of `scratch` (`z[s] = rhs[row_perm[s]]`
    /// for every `s`), so it tolerates arbitrary leftover content — but
    /// `l_solve_sparse_into` requires `scratch` to *already* be all-zero
    /// on entry (see its own docs for why cheaply reconstructing that
    /// precondition is this function's job, not its own). This function
    /// upholds that precondition for its *own* next call by clearing
    /// `scratch` back to all-zero, in full, right before returning — but
    /// that guarantee only holds if nothing else writes through the same
    /// buffer in between.
    pub fn solve_sparse_into(&self, rhs_sparse: &[(usize, f64)], scratch: &mut [f64], gp: &mut GpScratch, out: &mut [f64]) {
        self.base.l_solve_sparse_into(rhs_sparse, scratch, gp);
        for reta in &self.r_etas {
            let dot: f64 = reta.r.iter().map(|&(i, v)| v * scratch[i]).sum();
            scratch[reta.p] -= dot;
        }
        // `U` stays on the dense `u_solve_into`, not a GP-sparsified
        // counterpart — see that function's own docs for why a real
        // attempt at exactly that (persistent-buffer, position-aware DFS
        // over `u_seq`, fully implemented and correct) measured as a net
        // *regression* once benchmarked, and was reverted.
        self.u_solve_into(scratch);
        for s in 0..self.base.m {
            out[self.base.col_perm[s]] = scratch[s];
        }
        scratch.fill(0.0);
    }

    /// Allocating convenience wrapper around [`Self::solve_into`] — kept for
    /// call sites (tests, `try_update`) that don't already have a reusable
    /// buffer on hand.
    pub fn solve(&self, rhs: &[f64]) -> Vec<f64> {
        let m = self.base.m;
        let mut scratch = vec![0.0; m];
        let mut out = vec![0.0; m];
        self.solve_into(rhs, &mut scratch, &mut out);
        out
    }

    /// Writes `B^-T rhs` into `out` (length `m`), using `scratch` (also
    /// length `m`) as working space — no allocation; see [`Self::solve_into`]'s
    /// own docs for why this matters (this is `solve_lp_dual_on`'s
    /// once-per-pivot BTRAN for `rho_p`).
    pub fn solve_transpose_into(&self, rhs: &[f64], scratch: &mut [f64], out: &mut [f64]) {
        let m = self.base.m;
        for s in 0..m {
            scratch[s] = rhs[self.base.col_perm[s]];
        }
        self.u_transpose_solve_into(scratch);
        // Hyper-sparse: same skip as `u_solve_into`/`l_solve_into` — `yp`
        // is the only value each `r_eta`'s entries get multiplied by here.
        for reta in self.r_etas.iter().rev() {
            let yp = scratch[reta.p];
            if yp == 0.0 {
                continue;
            }
            for &(i, v) in &reta.r {
                scratch[i] -= v * yp;
            }
        }
        self.base.l_transpose_solve_into(scratch, out);
    }

    /// Allocating convenience wrapper around [`Self::solve_transpose_into`]
    /// — kept for call sites (tests) that don't already have a reusable
    /// buffer on hand.
    pub fn solve_transpose(&self, rhs: &[f64]) -> Vec<f64> {
        let m = self.base.m;
        let mut scratch = vec![0.0; m];
        let mut out = vec![0.0; m];
        self.solve_transpose_into(rhs, &mut scratch, &mut out);
        out
    }

    /// Records a Forrest-Tomlin update replacing the column at basis slot
    /// `basis_slot` (an index into the simplex basis array, i.e. a
    /// *column* index of `B`) with `a_q_original` (the entering column,
    /// dense, length `m`, in original row indexing — this is `a_q` from
    /// eq. 1, *not* `alpha = B^{-1}a_q`: unlike a product-form update,
    /// FT needs only the partial FTRAN result `L^{-1}a_q`, not the full
    /// solve). Returns `false` (recording nothing) if the resulting pivot
    /// is too small — refactorization trigger (2): the caller must
    /// refactorize the new basis from scratch instead.
    ///
    /// **Schork & Gondzio (2017), "Permuting Spiked Matrices to Triangular
    /// Form and its Application to the Forrest-Tomlin Update"**: tried and
    /// reverted this session. The idea: when the spike's own diagonal
    /// `a_tilde[p]` is nonzero and its off-diagonal support is disjoint
    /// from the structural `Reach(p)` (every slot whose value transitively
    /// depends on `p` — a single forward walk over `u_seq`, mirroring
    /// `u_transpose_solve_into`'s own traversal but following every
    /// *stored* `off_diag` edge unconditionally rather than only the ones
    /// whose *propagated* value under one unit-impulse seed happens to
    /// still be nonzero — the two differ on real, coefficient-heavy LP
    /// data via exact numerical cancellation, confirmed against real
    /// Netlib instances via a dedicated invariant cross-check during
    /// development), the spiked matrix is *already* permutable to
    /// triangular form with no elimination and no [`REta`] at all (their
    /// Theorem 3.1 / Lemma 3.2) — repositioning `p` and every member of
    /// `Reach(p)` to the end of `u_seq`, preserving their relative order,
    /// instead.
    ///
    /// Implemented fully correctly (including the structural-vs-numerical
    /// reach distinction above, found and fixed via a randomized stress
    /// test plus real-Netlib debug cross-checks) and, separately, a real
    /// unrelated bug it exposed (`simplex.rs`'s `FT_MAX_UPDATES` hard
    /// refactorization cap read `update_count()`, i.e. `r_etas.len()` —
    /// which a permutation-only update never grows, so on instances where
    /// many updates resolve that way the cap could go uncrossed far longer
    /// than intended, letting numerical drift compound until a later
    /// refactorization hit a matrix too corrupted to factor; fixed by
    /// counting *every* successful update, not just row-eta ones, for that
    /// specific trigger). Even after replacing an initial `HashSet`-based
    /// reach implementation with an epoch-stamped array (the same
    /// bump-instead-of-clear trick `sparse_lu::GpScratch` already uses),
    /// full-Netlib measurement still showed a net regression — not from
    /// this function's own added cost (which the epoch-array version
    /// brought back down close to baseline), but because the permutation
    /// path's slightly different rounding characteristics than the
    /// standard row-eta path perturbed dual-simplex tie-breaks on
    /// degeneracy-heavy instances (`pilotnov` needed 3218 iterations
    /// instead of 1286 for the *same* correct answer) — a downstream
    /// effect no amount of tuning this function itself can address. See
    /// the project history around this doc comment's own commit for the
    /// full numbers if revisiting.
    pub fn try_update(&mut self, basis_slot: usize, a_q_original: &[f64], min_pivot: f64) -> bool {
        let m = self.base.m;
        let p = self.base.col_perm_inv[basis_slot];

        let a_tilde = self.ftran_through_l_and_r(a_q_original);

        // Builds the unit vector directly into the buffer `u_transpose_solve_into`
        // will mutate in place, rather than allocating `e_p` and handing it
        // to the allocating `u_transpose_solve` wrapper (which would then
        // `.to_vec()`-clone it again internally) — one m-length allocation
        // instead of two for what's otherwise almost entirely zeros.
        let mut e_tilde = vec![0.0; m];
        e_tilde[p] = 1.0;
        self.u_transpose_solve_into(&mut e_tilde);

        let seq_pos = self.find_seq_pos(p);
        let old_pivot = self.u_seq[seq_pos].pivot;

        let r_vec: Vec<(usize, f64)> =
            (0..m).filter(|&i| i != p).map(|i| (i, -old_pivot * e_tilde[i])).filter(|&(_, v)| v != 0.0).collect();

        let dot: f64 = r_vec.iter().map(|&(i, v)| v * a_tilde[i]).sum();
        let new_pivot = a_tilde[p] - dot;
        if new_pivot.abs() < min_pivot {
            return false;
        }

        self.u_seq.remove(seq_pos);
        for eta in &mut self.u_seq {
            eta.off_diag.retain(|&(row_step, _)| row_step != p);
        }

        let off_diag: Vec<(usize, f64)> =
            (0..m).filter(|&i| i != p && a_tilde[i] != 0.0).map(|i| (i, a_tilde[i])).collect();
        self.u_seq.push(UEta { slot: p, pivot: new_pivot, off_diag });
        self.r_etas.push(REta { p, r: r_vec });

        true
    }

    pub fn update_count(&self) -> usize {
        self.r_etas.len()
    }

    /// Total off-diagonal fill currently held across `U`'s eta sequence
    /// and the `R` etas — used as the "bump size" measure for
    /// refactorization trigger (3): as updates accumulate, the eta file
    /// grows (each `R` and each replaced `U` slot can carry up to `m-1`
    /// entries), which is exactly the cost this trigger exists to bound.
    pub fn fill_count(&self) -> usize {
        self.u_seq.iter().map(|e| e.off_diag.len()).sum::<usize>() + self.r_etas.iter().map(|e| e.r.len()).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_vec(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-8)
    }

    #[test]
    fn factorize_and_solve_matches_expected() {
        // B = [[2,1,0],[1,3,1],[0,1,4]] (tridiagonal, sparse).
        let rows = vec![vec![(0, 2.0), (1, 1.0)], vec![(0, 1.0), (1, 3.0), (2, 1.0)], vec![(1, 1.0), (2, 4.0)]];
        let lu = factorize(3, &rows).expect("nonsingular");
        let x_true = [1.0, 2.0, 3.0];
        // rhs = B * x_true
        let rhs = [2.0 * 1.0 + 1.0 * 2.0, 1.0 * 1.0 + 3.0 * 2.0 + 1.0 * 3.0, 1.0 * 2.0 + 4.0 * 3.0];
        let x = lu.solve(&rhs);
        assert!(approx_vec(&x, &x_true), "x={x:?}");

        // B^T y = rhs2
        let y_true = [0.5, -1.0, 2.0];
        let rhs2 = [
            2.0 * y_true[0] + 1.0 * y_true[1],
            1.0 * y_true[0] + 3.0 * y_true[1] + 1.0 * y_true[2],
            1.0 * y_true[1] + 4.0 * y_true[2],
        ];
        let y = lu.solve_transpose(&rhs2);
        assert!(approx_vec(&y, &y_true), "y={y:?}");
    }

    #[test]
    fn factorize_detects_singular() {
        // Row 2 = 2 * row 0 in a 3x3 with cols {0,1} only used -> column 2 empty -> singular.
        let rows = vec![vec![(0, 1.0), (1, 2.0)], vec![(0, 3.0), (1, 1.0)], vec![(0, 2.0), (1, 4.0)]];
        assert!(factorize(3, &rows).is_none());
    }

    #[test]
    fn ft_update_matches_full_refactor() {
        // B0 = I (3x3). Replace column 1 with [1,5,2] (basis_slot=1).
        let rows0: Vec<Vec<(usize, f64)>> = (0..3).map(|i| vec![(i, 1.0)]).collect();
        let base = factorize(3, &rows0).unwrap();
        let mut state = FtLu::new(base);
        let a_q = [1.0, 5.0, 2.0];
        assert!(state.try_update(1, &a_q, 1e-9));

        // Full refactor of the new basis for comparison: column 1 of I
        // replaced by [1,5,2] (row-sparse, so the new column shows up as
        // one entry per row: (col1,1.0) in row0, (col1,5.0) in row1,
        // (col1,2.0) plus the untouched (col2,1.0) in row2).
        let rows1 = vec![vec![(0, 1.0), (1, 1.0)], vec![(1, 5.0)], vec![(1, 2.0), (2, 1.0)]];
        let full = factorize(3, &rows1).unwrap();

        let rhs = [3.0, -2.0, 7.0];
        let x_ft = state.solve(&rhs);
        let x_full = full.solve(&rhs);
        assert!(approx_vec(&x_ft, &x_full), "ft={x_ft:?} full={x_full:?}");

        let y_ft = state.solve_transpose(&rhs);
        let y_full = full.solve_transpose(&rhs);
        assert!(approx_vec(&y_ft, &y_full), "ft={y_ft:?} full={y_full:?}");
    }

    #[test]
    fn ft_update_chain_of_two_matches_full_refactor() {
        // B0 nontrivial (not identity) so the second update's partial
        // BTRAN must go through an already-updated U, not the base case.
        let rows0 =
            vec![vec![(0, 2.0), (1, 1.0)], vec![(0, 1.0), (1, 3.0), (2, 1.0)], vec![(1, 1.0), (2, 4.0)]];
        let base = factorize(3, &rows0).unwrap();
        let mut state = FtLu::new(base);

        let a_q1 = [1.0, 5.0, 2.0];
        assert!(state.try_update(1, &a_q1, 1e-9));

        let a_q2 = [4.0, 1.0, 3.0];
        assert!(state.try_update(0, &a_q2, 1e-9));

        // New basis columns: col0 = a_q2, col1 = a_q1, col2 unchanged from B0.
        let rows_full = vec![
            vec![(0, 4.0), (1, 1.0)],
            vec![(0, 1.0), (1, 5.0), (2, 1.0)],
            vec![(0, 3.0), (1, 2.0), (2, 4.0)],
        ];
        let full = factorize(3, &rows_full).unwrap();

        let rhs = [2.0, -3.0, 1.0];
        let x_ft = state.solve(&rhs);
        let x_full = full.solve(&rhs);
        assert!(approx_vec(&x_ft, &x_full), "ft={x_ft:?} full={x_full:?}");

        let y_ft = state.solve_transpose(&rhs);
        let y_full = full.solve_transpose(&rhs);
        assert!(approx_vec(&y_ft, &y_full), "ft={y_ft:?} full={y_full:?}");
    }

    #[test]
    fn ft_update_chain_of_three_matches_full_refactor() {
        // 4x4, three sequential updates (exercising the r_etas loop with
        // two, then two-more-accumulated, prior updates when computing
        // each new a_tilde).
        let rows0 = vec![
            vec![(0, 4.0), (1, 1.0)],
            vec![(0, 1.0), (1, 3.0), (2, 1.0)],
            vec![(1, 1.0), (2, 5.0), (3, 2.0)],
            vec![(2, 1.0), (3, 6.0)],
        ];
        let base = factorize(4, &rows0).unwrap();
        let mut state = FtLu::new(base);

        let a_q1 = [2.0, 7.0, 1.0, 3.0];
        assert!(state.try_update(2, &a_q1, 1e-9));
        let a_q2 = [5.0, 1.0, 4.0, 2.0];
        assert!(state.try_update(0, &a_q2, 1e-9));
        let a_q3 = [1.0, 6.0, 2.0, 3.0];
        assert!(state.try_update(3, &a_q3, 1e-9));

        // Final basis: col0=a_q2=[5,1,4,2], col1 unchanged=[1,3,1,0],
        // col2=a_q1=[2,7,1,3], col3=a_q3=[1,6,2,3].
        let rows_full = vec![
            vec![(0, 5.0), (1, 1.0), (2, 2.0), (3, 1.0)],
            vec![(0, 1.0), (1, 3.0), (2, 7.0), (3, 6.0)],
            vec![(0, 4.0), (1, 1.0), (2, 1.0), (3, 2.0)],
            vec![(0, 2.0), (2, 3.0), (3, 3.0)],
        ];
        let full = factorize(4, &rows_full).unwrap();

        let rhs = [1.0, 2.0, -1.0, 3.0];
        let x_ft = state.solve(&rhs);
        let x_full = full.solve(&rhs);
        assert!(approx_vec(&x_ft, &x_full), "ft={x_ft:?} full={x_full:?}");

        let y_ft = state.solve_transpose(&rhs);
        let y_full = full.solve_transpose(&rhs);
        assert!(approx_vec(&y_ft, &y_full), "ft={y_ft:?} full={y_full:?}");
    }

    #[test]
    fn ft_update_rejects_tiny_pivot() {
        let rows0: Vec<Vec<(usize, f64)>> = (0..2).map(|i| vec![(i, 1.0)]).collect();
        let base = factorize(2, &rows0).unwrap();
        let mut state = FtLu::new(base);
        // Replacing column 0 with something whose L^-1-transformed value
        // at slot 0 is 0 (after the r-correction) makes the new pivot 0.
        let a_q = [0.0, 1.0];
        assert!(!state.try_update(0, &a_q, 1e-9));
        assert_eq!(state.update_count(), 0);
    }


    fn to_sparse(dense: &[f64]) -> Vec<(usize, f64)> {
        dense.iter().enumerate().filter(|&(_, &v)| v != 0.0).map(|(i, &v)| (i, v)).collect()
    }

    /// Runs `rhs` (converted to sparse form) through `solve_sparse_into`
    /// and asserts it matches `state.solve(rhs)` (the dense reference)
    /// exactly — both should compute the identical sequence of floating
    /// point operations restricted to the same reach set, just reached by
    /// different bookkeeping, so unlike `approx_vec`'s tolerance
    /// elsewhere in this module (guarding against genuinely different
    /// numerical paths, e.g. FT-updated vs freshly-refactored), this
    /// checks bit-for-bit equality — any mismatch at all means the reach
    /// set or the zero-management between calls is wrong.
    fn assert_sparse_matches_dense(state: &FtLu, m: usize, rhs: &[f64], scratch: &mut [f64], gp: &mut GpScratch, out: &mut [f64]) {
        let expected = state.solve(rhs);
        state.solve_sparse_into(&to_sparse(rhs), scratch, gp, out);
        assert_eq!(&out[..m], &expected[..], "rhs={rhs:?}");
    }

    #[test]
    fn sparse_solve_matches_dense_on_simple_case() {
        // Same 3x3 tridiagonal fixture as `factorize_and_solve_matches_expected`.
        let rows = vec![vec![(0, 2.0), (1, 1.0)], vec![(0, 1.0), (1, 3.0), (2, 1.0)], vec![(1, 1.0), (2, 4.0)]];
        let base = factorize(3, &rows).unwrap();
        let state = FtLu::new(base);
        let m = 3;
        let mut scratch = vec![0.0; m];
        let mut gp = GpScratch::new(m);
        let mut out = vec![0.0; m];

        for rhs in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [3.0, -2.0, 7.0],
            [0.0, 0.0, 0.0],
        ] {
            assert_sparse_matches_dense(&state, m, &rhs, &mut scratch, &mut gp, &mut out);
        }
    }

    #[test]
    fn sparse_solve_matches_dense_with_ft_updates() {
        // Same fixture (and update sequence) as
        // `ft_update_chain_of_two_matches_full_refactor` — `state.r_etas`
        // is non-empty here, exercising the sparse path's R-eta stage
        // (unchanged from the dense path, but only actually run if this
        // wiring is correct).
        let rows0 = vec![vec![(0, 2.0), (1, 1.0)], vec![(0, 1.0), (1, 3.0), (2, 1.0)], vec![(1, 1.0), (2, 4.0)]];
        let base = factorize(3, &rows0).unwrap();
        let mut state = FtLu::new(base);
        assert!(state.try_update(1, &[1.0, 5.0, 2.0], 1e-9));
        assert!(state.try_update(0, &[4.0, 1.0, 3.0], 1e-9));

        let m = 3;
        let mut scratch = vec![0.0; m];
        let mut gp = GpScratch::new(m);
        let mut out = vec![0.0; m];

        for rhs in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [2.0, -3.0, 1.0]] {
            assert_sparse_matches_dense(&state, m, &rhs, &mut scratch, &mut gp, &mut out);
        }
    }

    #[test]
    fn sparse_solve_repeated_calls_reuse_scratch_correctly() {
        // A larger (6x6), more sparsely-structured matrix — enough steps
        // and fill-in variety that the reach set genuinely differs across
        // calls — solved for a long, varied sequence of sparse right-hand
        // sides (single nonzero, several scattered nonzeros, fully dense,
        // and all-zero) through the *same* `scratch`/`gp` buffers, back
        // to back. This is the specific scenario `l_solve_sparse_into`'s
        // "z must be all-zero on entry" precondition depends on
        // `solve_sparse_into`'s own end-of-call `fill(0.0)` to uphold —
        // if that cleanup were wrong or incomplete, an *earlier* call's
        // leftover values would corrupt a *later* call's result, so
        // running many varied calls in sequence and checking every one
        // (not just the first) is the point of this test.
        let rows0 = vec![
            vec![(0, 4.0), (2, 1.0)],
            vec![(1, 3.0), (3, 1.0)],
            vec![(0, 1.0), (2, 5.0), (4, 1.0)],
            vec![(1, 1.0), (3, 6.0), (5, 2.0)],
            vec![(2, 1.0), (4, 4.0)],
            vec![(3, 1.0), (5, 3.0)],
        ];
        let base = factorize(6, &rows0).unwrap();
        let state = FtLu::new(base);
        let m = 6;
        let mut scratch = vec![0.0; m];
        let mut gp = GpScratch::new(m);
        let mut out = vec![0.0; m];

        let rhs_sequence: Vec<[f64; 6]> = vec![
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [1.0, 0.0, 2.0, 0.0, 0.0, 3.0],
            [0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        ];
        for rhs in &rhs_sequence {
            assert_sparse_matches_dense(&state, m, rhs, &mut scratch, &mut gp, &mut out);
        }
        // The buffer must be back to exactly zero after the last call too
        // — not just "happened to match the expected output" — since
        // that's the invariant the *next* caller (whoever it is) relies on.
        assert!(scratch.iter().all(|&v| v == 0.0), "scratch not fully cleared: {scratch:?}");
    }

    #[test]
    fn sparse_solve_matches_dense_after_reordering_u_seq() {
        // `solve_sparse_into` (sparse `L` + dense `U`) must keep matching
        // the dense reference through *repeated* `u_seq` reordering
        // (every `try_update` removes one eta from wherever it sits and
        // appends a fresh one at the end, shifting everything after the
        // removal point down by one) — a single update, as the other
        // FT-update tests already exercise, isn't enough to be confident
        // this stays right across several. Five sequential updates on a
        // 6x6 basis, each replacing a different slot (including slots at
        // both ends and the middle of the current `u_seq`, so removals
        // happen at varied positions), checked against the dense
        // reference for a run of varied sparse right-hand sides after
        // *every single* update — not just the final one.
        let rows0 = vec![
            vec![(0, 3.0), (2, 1.0)],
            vec![(1, 4.0), (3, 1.0)],
            vec![(0, 1.0), (2, 5.0), (4, 1.0)],
            vec![(1, 1.0), (3, 6.0), (5, 1.0)],
            vec![(2, 1.0), (4, 4.0)],
            vec![(3, 1.0), (5, 3.0)],
        ];
        let base = factorize(6, &rows0).unwrap();
        let mut state = FtLu::new(base);
        let m = 6;
        let mut scratch = vec![0.0; m];
        let mut gp = GpScratch::new(m);
        let mut out = vec![0.0; m];

        let rhs_probes: [[f64; 6]; 5] = [
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 2.0, 0.0, 3.0],
            [1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        ];
        // Updates hit slot 4 (near the end), then 0 (the start), then 5
        // (the new end), then 2 (the middle), then 1 — deliberately not a
        // monotonic sequence, so `u_seq`'s position-vs-slot relationship
        // is scrambled well beyond a simple "always append" pattern.
        let updates: [(usize, [f64; 6]); 5] = [
            (4, [1.0, 0.0, 2.0, 0.0, 3.0, 0.0]),
            (0, [4.0, 1.0, 0.0, 0.0, 1.0, 2.0]),
            (5, [0.0, 2.0, 1.0, 3.0, 0.0, 5.0]),
            (2, [2.0, 0.0, 3.0, 1.0, 0.0, 1.0]),
            (1, [1.0, 3.0, 2.0, 0.0, 1.0, 0.0]),
        ];
        for &(slot, a_q) in &updates {
            assert!(state.try_update(slot, &a_q, 1e-9), "update on slot {slot} rejected");
            for rhs in &rhs_probes {
                assert_sparse_matches_dense(&state, m, rhs, &mut scratch, &mut gp, &mut out);
            }
        }
        assert!(scratch.iter().all(|&v| v == 0.0), "scratch not fully cleared: {scratch:?}");
    }

}
