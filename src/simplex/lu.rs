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

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};

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
const STABILITY: f64 = 0.25;
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

    // `col_max_abs_dirty[j]`: `refresh_column` marked `col_max_abs[j]`
    // stale (its `col_rows[j]` membership or values changed) but hasn't
    // recomputed it yet — deferred to `ensure_col_max_abs`, called only
    // once `find_best_pivot` is actually about to read it. Most columns
    // `eliminate` dirties this way get dirtied again by a later
    // elimination step before `find_best_pivot` ever visits them (a
    // column's bucket position, which *is* updated eagerly by
    // `update_col_degree`, is what determines when that happens), so
    // eagerly recomputing every dirtied column's max here was mostly
    // wasted work — up to 75% of it, measured on Netlib `greenbea`.
    col_max_abs_dirty: Vec<bool>,

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
            col_max_abs_dirty: vec![false; m],
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

    /// Updates `j`'s degree/bucket placement from its current
    /// `col_rows[j]` membership — O(that column's own active degree),
    /// never O(m) — and marks `col_max_abs[j]` stale rather than
    /// recomputing it here; see [`Self::ensure_col_max_abs`] and
    /// `col_max_abs_dirty`'s own docs for why.
    fn refresh_column(&mut self, j: usize) {
        if self.col_used[j] {
            return;
        }
        let new_deg = self.col_rows[j].len();
        self.update_col_degree(j, new_deg);
        self.col_max_abs_dirty[j] = true;
    }

    /// Recomputes `col_max_abs[j]` from its current `col_rows[j]`
    /// membership if `refresh_column` left it marked stale, otherwise a
    /// no-op — called from `find_best_pivot` right before it reads
    /// `col_max_abs[j]`, the one place that value's currency actually
    /// matters.
    fn ensure_col_max_abs(&mut self, j: usize) {
        if !self.col_max_abs_dirty[j] {
            return;
        }
        self.col_max_abs[j] =
            self.col_rows[j].iter().filter_map(|&i| self.rows[i].get(&j).map(|v| v.abs())).fold(0.0, f64::max);
        self.col_max_abs_dirty[j] = false;
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
    fn find_best_pivot(&mut self, skip_dense: bool) -> Option<(usize, usize)> {
        let __prof_t0 = std::time::Instant::now();
        PROF_TOTAL_STEPS.fetch_add(1, Ordering::Relaxed);
        let mut best: Option<(usize, usize)> = None;
        let mut best_score = usize::MAX;
        let mut best_pivot_abs = 0.0f64;

        for deg_col in 1..self.col_buckets.len() {
            // Indexed rather than iterated by reference: nothing in this
            // loop body mutates `col_buckets[deg_col]` itself (bucket
            // membership only ever changes via `update_col_degree`/
            // `remove_from_bucket_col`, called elsewhere, never from
            // inside `find_best_pivot`), so its length and contents are
            // fixed for this `deg_col`'s scan — indexing just avoids
            // holding an immutable borrow of `self` across the
            // `ensure_col_max_abs(j)` call below, which needs `&mut self`.
            for idx in 0..self.col_buckets[deg_col].len() {
                let j = self.col_buckets[deg_col][idx];
                if self.col_used[j] || (skip_dense && self.initially_dense[j]) {
                    continue;
                }
                self.ensure_col_max_abs(j);
                for &i in &self.col_rows[j] {
                    if self.row_used[i] {
                        continue;
                    }
                    // Markowitz score only needs row/col degree, both already
                    // known without touching `rows[i]` — skip the BTreeMap
                    // lookup below for candidates that can't possibly beat
                    // `best_score` (this is the vast majority on a matrix
                    // with heavy fill-in after many FT updates).
                    let score = (self.row_degree[i] - 1) * (self.col_degree[j] - 1);
                    if score > best_score {
                        continue;
                    }
                    let Some(&v) = self.rows[i].get(&j) else { continue };
                    if v == 0.0 || v.abs() < STABILITY * self.col_max_abs[j] {
                        continue;
                    }
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
    ///
    /// **Flattening this into a [`FixedRows`] (one flat `(index, value)`
    /// buffer plus offsets — the same layout `simplex.rs`'s `StdForm` uses
    /// for the frozen coefficient matrix, on the same reasoning: `L` never
    /// changes once a refactorization builds it) was implemented and
    /// measured, then reverted.** The theory was sound (`L` really is read
    /// every FTRAN/BTRAN's `L`-stage for the rest of that basis's life and
    /// never mutated again, so a per-row heap allocation plus pointer
    /// indirection looked like pure waste), but a controlled A/B (same
    /// build, only this field's representation toggled, 3 runs per problem)
    /// showed a **consistent small regression**, not a win: `scsd8` +2.1%,
    /// `25fv47` +1.8%, `stocfor2` +4.0%, `fit1p` +3.2%, with `degen3`/
    /// `pilotnov` flat within run-to-run noise — zero problems improved.
    /// Two likely reasons, neither of which the `Vec<Vec<...>>` form
    /// suffers from: (1) `l_col[s]` in real Netlib bases is typically very
    /// short (Markowitz elimination is specifically choosing pivots to keep
    /// it that way), so the "many small allocations" cost this was meant to
    /// remove was never that large to begin with, while accessing a
    /// `FixedRows` row still costs *two* offset reads (`offsets[i]`,
    /// `offsets[i+1]`) before the slice is even known, against `Vec<Vec>`'s
    /// single pointer hop to an already-known `(ptr, len)` pair; (2) the
    /// conversion itself doesn't avoid building the `m` small per-column
    /// `Vec`s first (both `factorize` and `factorize_dense_faer` still
    /// populate a `Vec<Vec<...>>` while walking `L`'s entries in whatever
    /// order they're produced) — flattening just added one more `O(nnz)`
    /// copy on top afterward, at construction time, without ever removing
    /// the allocations it was trying to avoid. A version that builds the
    /// flat buffer directly (computing offsets in one pass, filling
    /// `entries` in a second, the way `FixedRows::from_transpose` already
    /// does for a *transposed* build) might still be worth trying — this
    /// attempt just never built that version — but plain
    /// `Vec<Vec<(usize, f64)>>` is what's actually measured fastest so far.
    pub l_col: Vec<Vec<(usize, f64)>>,
    /// `u_row[s]`: `(col_step, value)` pairs, `col_step >= s` (including
    /// the diagonal at `col_step == s`) — the entries of `U`'s row `s`.
    /// Unlike `l_col`, this is read exactly once per refactorization (by
    /// `FtLu::new`, to seed `u_seq`) and never again, so it stays a plain
    /// `Vec<Vec<...>>` — flattening it would cost the same construction
    /// work for no repeated-read benefit.
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
/// Input whose nonzero density exceeds this fraction of `m^2` skips
/// Markowitz elimination entirely in favor of [`factorize_dense_faer`]'s
/// dense partial-pivoting LU (via the `faer` crate). Markowitz's whole
/// point is to *minimize fill-in*; a matrix already this dense has none
/// left to save, so its bucket/degree bookkeeping (`col_rows`,
/// `col_buckets`/`row_buckets`, the `BTreeMap`-per-row active submatrix)
/// is pure overhead at that point — confirmed on a synthetic dense LP
/// (Netlib has none dense enough to exercise this at all): `factorize`
/// dominated wall time (95-98%, repeated every few dozen `try_update`
/// calls since a dense basis's eta fill crosses `FT_BUMP_LIMIT_FACTOR *
/// m` almost immediately) while this file's own FTRAN-side dense
/// optimizations (`OffDiag::Dense`, `FtLu::should_use_dense_solve`)
/// together accounted for under 1% of the same wall time — i.e. the eta
/// chain was never the bottleneck for a dense basis, the cold
/// factorization was. `0.25` is a first-pass threshold, not yet tuned
/// against a real dense-problem benchmark (Netlib has none).
const DENSE_INPUT_FRACTION: f64 = 0.25;

fn is_dense_input(m: usize, rows_in: &[Vec<(usize, f64)>]) -> bool {
    if m == 0 {
        return false;
    }
    let nnz: usize = rows_in.iter().map(|r| r.len()).sum();
    nnz as f64 > DENSE_INPUT_FRACTION * (m as f64) * (m as f64)
}

/// Dense partial-pivoting LU via `faer` (`PartialPivLu`, `PA = LU`, row
/// pivoting only — so `col_perm` here is always the identity). Converts
/// `faer`'s dense `Mat<f64>` factors into this module's existing
/// `LuFactors` representation so every downstream consumer (`FtLu`,
/// `l_solve_into`/`u_solve_into`, the Forrest-Tomlin update machinery) is
/// completely unaware of which path produced its `LuFactors` — see
/// [`DENSE_INPUT_FRACTION`]'s own docs for why this exists instead of
/// running Markowitz on an already-dense matrix.
fn factorize_dense_faer(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Option<LuFactors> {
    let mut a = faer::Mat::<f64>::zeros(m, m);
    for (i, row) in rows_in.iter().enumerate() {
        for &(j, v) in row {
            a[(i, j)] += v;
        }
    }

    let lu = faer::linalg::solvers::PartialPivLu::new(a.as_ref());
    let l = lu.compute_l();
    let u = lu.compute_u();
    let perm = lu.row_permutation();
    let (fwd, _inv) = perm.arrays();
    // `PA = LU`: row `step` of the permuted matrix `PA` is original row
    // `fwd[step]` — exactly this module's own `row_perm[step]` meaning
    // (`l_solve_into` permutes `rhs` the same way: `z[s] = rhs[row_perm[s]]`).
    let row_perm: Vec<usize> = fwd.iter().map(|&idx| usize::from(idx)).collect();
    let col_perm: Vec<usize> = (0..m).collect();

    for step in 0..m {
        if u[(step, step)] == 0.0 {
            return None;
        }
    }

    let mut row_perm_inv = vec![0usize; m];
    let mut col_perm_inv = vec![0usize; m];
    for step in 0..m {
        row_perm_inv[row_perm[step]] = step;
        col_perm_inv[col_perm[step]] = step;
    }

    let mut l_col_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for step in 0..m {
        for row_step in (step + 1)..m {
            let v = l[(row_step, step)];
            if v != 0.0 {
                l_col_rows[step].push((row_step, v));
            }
        }
    }
    let l_col = l_col_rows;
    let mut u_row: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for step in 0..m {
        for col_step in step..m {
            let v = u[(step, col_step)];
            if v != 0.0 {
                u_row[step].push((col_step, v));
            }
        }
    }

    Some(LuFactors { m, l_col, u_row, row_perm, col_perm, col_perm_inv, row_perm_inv })
}

/// Throwaway diagnostic (`ENOMOTO_DEBUG_BLOCK_SIZES`), not wired into any
/// production path: measures what block-size distribution a Dulmage-Mendelsohn
/// SCC decomposition of *this* refactorization's basis matrix would actually
/// have, to check a specific hypothesis about the previously-reverted
/// block-triangularized `factorize` (see this function's own doc comment) —
/// namely, whether the blocks it would find are mostly tiny (say <=10 or
/// <=50 rows), which would matter for a proposal to special-case small
/// blocks with a dense/product-form solve instead of the general sparse
/// Forrest-Tomlin machinery.
fn debug_print_block_sizes(m: usize, rows_in: &[Vec<(usize, f64)>]) {
    let adj: Vec<Vec<usize>> = rows_in.iter().map(|row| row.iter().map(|&(j, _)| j).collect()).collect();
    let __t0 = std::time::Instant::now();
    let decomp = crate::graph::dulmage_mendelsohn_blocks_topological(&adj, m);
    let decomp_us = __t0.elapsed().as_micros();
    match decomp {
        None => eprintln!("BLOCK_SIZES m={m} no-perfect-matching decomp_us={decomp_us}"),
        Some((blocks, _)) => {
            let mut sizes: Vec<usize> = blocks.iter().map(|b| b.len()).collect();
            sizes.sort_unstable();
            let n_blocks = sizes.len();
            let le10 = sizes.iter().filter(|&&s| s <= 10).count();
            let le50 = sizes.iter().filter(|&&s| s <= 50).count();
            let rows_le10: usize = sizes.iter().filter(|&&s| s <= 10).sum();
            let rows_le50: usize = sizes.iter().filter(|&&s| s <= 50).sum();
            let max = sizes.last().copied().unwrap_or(0);
            eprintln!(
                "BLOCK_SIZES m={m} n_blocks={n_blocks} max_block={max} blocks_le10={le10} blocks_le50={le50} rows_in_blocks_le10={rows_le10}({:.1}%) rows_in_blocks_le50={rows_le50}({:.1}%) decomp_us={decomp_us}",
                100.0 * rows_le10 as f64 / m as f64,
                100.0 * rows_le50 as f64 / m as f64,
            );
        }
    }
}

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
const BORDER_MAX_FRACTION: f64 = 0.4;
const BORDER_MAX_COUNT: usize = 3000;

/// Columns whose nonzero count exceeds [`DENSE_COL_FRACTION`] of `m` —
/// the same "near-fully-dense trend/regression column" shape
/// `MarkowitzState::initially_dense` already detects internally, exposed
/// here as a free function so `factorize`'s routing decision (attempt
/// [`factorize_bordered`] or not) can check it before paying for a
/// `MarkowitzState` at all — this is the only extra cost non-bordered
/// instances pay: one `O(nnz)` degree pass, measured (see
/// `BORDER_MAX_FRACTION`'s own docs) to be cheap enough to run
/// unconditionally rather than gated behind a flag.
fn detect_border_columns(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Vec<usize> {
    let mut col_degree = vec![0usize; m];
    for row in rows_in {
        for &(j, v) in row {
            if v != 0.0 {
                col_degree[j] += 1;
            }
        }
    }
    let threshold = DENSE_COL_FRACTION * m as f64;
    (0..m).filter(|&j| col_degree[j] as f64 > threshold).collect()
}

/// Builds the LU factorization of a diagonal basis matrix directly — `L =
/// I`, `U = B`, no row or column permutation — instead of running it
/// through [`factorize`]'s general Markowitz elimination. This is exactly
/// the shape of the initial all-slack basis (`B` a signed identity: each
/// row has exactly one nonzero, `+/-1`, at that row's own column), so
/// `simplex.rs`'s initial-basis call sites use this instead of paying for
/// pivot selection, fill-in bookkeeping, and border/dense-input detection
/// on a matrix that has nothing for any of that to do. Returns `None` if
/// `rows_in` isn't exactly diagonal, so a caller can fall back to
/// [`factorize`] rather than silently mis-factorizing.
pub fn factorize_diagonal(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Option<LuFactors> {
    let mut u_row = Vec::with_capacity(m);
    for (i, row) in rows_in.iter().enumerate() {
        match row.as_slice() {
            [(j, v)] if *j == i && *v != 0.0 => u_row.push(vec![(i, *v)]),
            _ => return None,
        }
    }
    let identity: Vec<usize> = (0..m).collect();
    Some(LuFactors {
        m,
        l_col: vec![Vec::new(); m],
        u_row,
        row_perm: identity.clone(),
        col_perm: identity.clone(),
        col_perm_inv: identity.clone(),
        row_perm_inv: identity,
    })
}

pub fn factorize(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Option<LuFactors> {
    if std::env::var("ENOMOTO_DEBUG_BLOCK_SIZES").is_ok() {
        debug_print_block_sizes(m, rows_in);
    }
    // Tried *before* checking `is_dense_input`, deliberately: the measured
    // crossover (see `BORDER_MAX_FRACTION`'s own docs) sits around
    // `k/m ~= 0.5`, well past `is_dense_input`'s own 25%-of-`m^2` overall-
    // density gate — a border-heavy input can easily cross that overall
    // gate on the border columns' own density alone while `k/m` is still
    // comfortably under `BORDER_MAX_FRACTION`, and in exactly that range
    // `factorize_bordered` beats `factorize_dense_faer` too (not just
    // plain Markowitz), so gating this attempt on `!is_dense_input` would
    // give up a real win. `factorize_bordered` itself falls through to
    // `None` (this function's own fallback to `factorize_flat_markowitz`,
    // which still makes its own `is_dense_input` dispatch) if the sparse
    // phase can't find `m - k` independent pivots.
    let border = detect_border_columns(m, rows_in);
    let k = border.len();
    if k > 0 && k <= BORDER_MAX_COUNT && (k as f64) <= BORDER_MAX_FRACTION * m as f64 {
        if let Some(lu) = factorize_bordered(m, rows_in, &border) {
            return Some(lu);
        }
    }
    factorize_flat_markowitz(m, rows_in)
}

fn factorize_flat_markowitz(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Option<LuFactors> {
    if is_dense_input(m, rows_in) {
        return factorize_dense_faer(m, rows_in);
    }
    let mut state = MarkowitzState::new(m, rows_in);

    let mut row_perm = vec![0usize; m];
    let mut col_perm = vec![0usize; m];

    let mut l_entries: Vec<(usize, usize, f64)> = Vec::new();
    let mut u_entries: Vec<(usize, usize, f64)> = Vec::new();
    let debug_eliminate_cost = std::env::var("ENOMOTO_DEBUG_ELIMINATE_COST").is_ok();
    let mut eliminate_ns: u128 = 0;
    let mut snapshot_ns: u128 = 0;

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
        let __t_snap0 = if debug_eliminate_cost { Some(std::time::Instant::now()) } else { None };
        let pivot_row_snapshot: Vec<(usize, f64)> = state.rows[pi]
            .iter()
            .filter(|&(&j, &v)| v != 0.0 && (j == pj || !state.col_used[j]))
            .map(|(&j, &v)| (j, v))
            .collect();
        if let Some(t0) = __t_snap0 {
            snapshot_ns += t0.elapsed().as_nanos();
        }

        for &(j, v) in &pivot_row_snapshot {
            u_entries.push((step, j, v));
        }

        let __t_elim0 = if debug_eliminate_cost { Some(std::time::Instant::now()) } else { None };
        for (i, mult) in state.eliminate(pi, pj, pivot_val, &pivot_row_snapshot) {
            l_entries.push((i, step, mult));
        }
        if let Some(t0) = __t_elim0 {
            eliminate_ns += t0.elapsed().as_nanos();
        }
    }
    if debug_eliminate_cost {
        eprintln!(
            "ELIMINATE_COST m={m} eliminate_ms={:.3} snapshot_ms={:.3} l_nnz={} u_nnz={} avg_row_fill={:.1}",
            eliminate_ns as f64 / 1e6,
            snapshot_ns as f64 / 1e6,
            l_entries.len(),
            u_entries.len(),
            u_entries.len() as f64 / m as f64,
        );
    }

    let mut row_perm_inv = vec![0usize; m];
    let mut col_perm_inv = vec![0usize; m];
    for step in 0..m {
        row_perm_inv[row_perm[step]] = step;
        col_perm_inv[col_perm[step]] = step;
    }

    let mut l_col_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for (orig_row, pivot_step, mult) in l_entries {
        l_col_rows[pivot_step].push((row_perm_inv[orig_row], mult));
    }
    let l_col = l_col_rows;
    let mut u_row: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for (pivot_step, orig_col, val) in u_entries {
        u_row[pivot_step].push((col_perm_inv[orig_col], val));
    }

    Some(LuFactors { m, l_col, u_row, row_perm, col_perm, col_perm_inv, row_perm_inv })
}

/// Bordered (Schur-complement) factorization: `border` columns — see
/// [`detect_border_columns`] — are excluded from the ordinary sparse
/// Markowitz phase entirely (never appear as pivot candidates, never
/// receive scattered fill from it) and instead resolved via a single
/// small dense `k x k` Schur complement at the end.
///
/// **Why this exists**: `fit1p`-shaped Netlib instances have ~20-25
/// columns nonzero in essentially every row (see `DENSE_COL_FRACTION`'s
/// own docs). The ordinary Markowitz path already defers pivoting *on*
/// these columns as long as possible (`MarkowitzState::initially_dense`),
/// but every ordinary elimination step whose pivot row still carries one
/// of these columns' entries scatters them into every row `eliminate`
/// touches anyway — measured (`ENOMOTO_DEBUG_ELIMINATE_COST`) as the
/// actual cost driver behind `fit1p`'s refactorizations (average row fill
/// climbing from `1.0` at the initial all-slack basis to `~10` a few
/// refactorizations later, each one costing several milliseconds despite
/// `m` only being in the hundreds). Excluding these columns from the
/// sparse phase's own bookkeeping entirely (rather than merely
/// deprioritizing them as pivot targets) removes that scatter cost
/// outright; the algebra it defers is applied once, in bulk, via the
/// classic bordered-block-diagonal LU identity:
///
/// ```text
/// A = [ A_SS  A_SD ]    L = [ L_SS   0  ]    U = [ U_SS  U_SD ]
///     [ A_DS  A_DD ]        [ L_DS  L_DD]        [  0    U_DD ]
/// ```
///
/// where `S` is the non-border columns and whichever `m - k` rows the
/// sparse phase ends up choosing as their pivots, and `D` is the `k`
/// border columns plus the `k` rows the sparse phase never touches.
/// `L_SS`/`U_SS` and `L_DS` (the sparse phase's own elimination
/// multipliers for *every* affected row, border rows included — free,
/// already computed as a side effect of the ordinary elimination) fall
/// out of a single Markowitz run with `border`'s entries simply absent
/// from the input. `U_SD = L_SS^{-1} A_SD` is computed via one sparse
/// forward solve per border column (structurally identical to
/// [`LuFactors::l_solve_into`], just against the in-progress `L_SS`
/// rather than a finished `LuFactors`). The Schur complement `A_DD -
/// L_DS U_SD` (`k x k`, dense) is then factored directly via
/// [`factorize_dense_faer`] — reusing the exact same dense path already
/// used for a globally-dense input, just at the `k`-sized scale this
/// bordering was meant to shrink the problem down to.
///
/// Returns `None` (falling back to [`factorize_flat_markowitz`] is the
/// caller's job, exactly as the dense-column-avoidance fallback in that
/// function already does for its own `skip_dense` retry) if the sparse
/// phase gets stuck before finding pivots for every non-border column —
/// a border column genuinely required as a pivot before all `m - k`
/// sparse columns are resolved — or if the final `k x k` Schur complement
/// itself turns out numerically singular.
fn factorize_bordered(m: usize, rows_in: &[Vec<(usize, f64)>], border: &[usize]) -> Option<LuFactors> {
    let k = border.len();
    if k == 0 || k >= m {
        return None;
    }
    let n_sparse = m - k;

    let mut is_border = vec![false; m];
    for &j in border {
        is_border[j] = true;
    }

    // Strip border columns from the input entirely before building the
    // Markowitz state — this (not any change to `find_best_pivot` or
    // `eliminate`) is what keeps the sparse phase from ever scattering
    // into them: a column with zero remaining entries never appears in
    // any `col_buckets` entry beyond bucket `0`, which `find_best_pivot`'s
    // `for deg_col in 1..` loop never even visits.
    let sparse_rows: Vec<Vec<(usize, f64)>> =
        rows_in.iter().map(|row| row.iter().copied().filter(|&(j, v)| v != 0.0 && !is_border[j]).collect()).collect();

    let mut state = MarkowitzState::new(m, &sparse_rows);

    let mut row_perm = vec![usize::MAX; m];
    let mut col_perm = vec![usize::MAX; m];
    // (orig_row, pivot_step, mult) for every row `eliminate` ever touches
    // during the sparse phase, border rows included — exactly `L_SS` and
    // `L_DS` together, no separate bookkeeping needed for the latter.
    let mut l_entries: Vec<(usize, usize, f64)> = Vec::new();
    // (pivot_step, orig_col, value) — `U_SS`'s own entries; `U_SD` is
    // appended to this same list further down.
    let mut u_entries: Vec<(usize, usize, f64)> = Vec::new();

    for step in 0..n_sparse {
        let (pi, pj) = state.find_best_pivot(true).or_else(|| state.find_best_pivot(false))?;

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

    let border_rows: Vec<usize> = (0..m).filter(|&i| !state.row_used[i]).collect();
    assert_eq!(border_rows.len(), k, "sparse phase must leave exactly `k` rows unpivoted");

    let mut row_perm_inv = vec![usize::MAX; m];
    for step in 0..n_sparse {
        row_perm_inv[row_perm[step]] = step;
    }
    let mut border_row_local = vec![usize::MAX; m];
    for (local, &r) in border_rows.iter().enumerate() {
        border_row_local[r] = local;
    }

    // `l_col_ss[s]`: `L_SS`'s own column `s` (row targets restricted to
    // sparse-phase steps) — an intermediate used only by this function's
    // own forward solves below, never exposed outside it.
    let mut l_col_ss: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n_sparse];
    // `l_ds[local]`: border row `local`'s accumulated multipliers against
    // each sparse step — exactly `L_DS[local, :]`, already complete.
    let mut l_ds: Vec<Vec<(usize, f64)>> = vec![Vec::new(); k];
    for &(orig_row, step, mult) in &l_entries {
        let ri = row_perm_inv[orig_row];
        if ri != usize::MAX {
            l_col_ss[step].push((ri, mult));
        } else {
            l_ds[border_row_local[orig_row]].push((step, mult));
        }
    }

    let mut border_index = vec![usize::MAX; m];
    for (idx, &j) in border.iter().enumerate() {
        border_index[j] = idx;
    }

    // `border_col_rows[idx]`: every original `(row, value)` pair at
    // border column `border[idx]`, gathered in one `O(nnz)` pass — reused
    // below both for `A_SD` (this column's forward solve) and `A_DD`.
    let mut border_col_rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); k];
    for (i, row) in rows_in.iter().enumerate() {
        for &(j, v) in row {
            if v == 0.0 {
                continue;
            }
            let idx = border_index[j];
            if idx != usize::MAX {
                border_col_rows[idx].push((i, v));
            }
        }
    }

    // A_DD (original values; the Schur complement below subtracts the
    // `L_DS * U_SD` correction from this directly, rather than reading
    // any partially-reduced state — this *is* the standard bordered-LU
    // identity, not an approximation of it).
    let mut schur = vec![vec![0.0f64; k]; k];
    for (idx, rows_for_col) in border_col_rows.iter().enumerate() {
        for &(orig_row, v) in rows_for_col {
            let local_e = border_row_local[orig_row];
            if local_e != usize::MAX {
                schur[local_e][idx] += v;
            }
        }
    }

    for (idx, rows_for_col) in border_col_rows.iter().enumerate() {
        let mut y = vec![0.0f64; n_sparse];
        for &(orig_row, v) in rows_for_col {
            let s = row_perm_inv[orig_row];
            if s != usize::MAX {
                y[s] += v;
            }
        }
        // Forward solve `L_SS y = y` in place (unit lower triangular,
        // step order) — structurally identical to `l_solve_into`.
        for s in 0..n_sparse {
            if y[s] == 0.0 {
                continue;
            }
            for &(row_step, mult) in &l_col_ss[s] {
                y[row_step] -= mult * y[s];
            }
        }
        for (s, &val) in y.iter().enumerate() {
            if val != 0.0 {
                u_entries.push((s, border[idx], val));
            }
        }
        for local_e in 0..k {
            if l_ds[local_e].is_empty() {
                continue;
            }
            let mut acc = 0.0;
            for &(step, mult) in &l_ds[local_e] {
                if y[step] != 0.0 {
                    acc += mult * y[step];
                }
            }
            schur[local_e][idx] -= acc;
        }
    }

    let schur_rows: Vec<Vec<(usize, f64)>> = schur
        .iter()
        .map(|row| row.iter().enumerate().filter(|&(_, &v)| v != 0.0).map(|(j, &v)| (j, v)).collect())
        .collect();
    let border_lu = factorize_dense_faer(k, &schur_rows)?;

    for s in 0..k {
        debug_assert_eq!(border_lu.col_perm[s], s, "factorize_dense_faer's own col_perm is always identity");
        row_perm[n_sparse + s] = border_rows[border_lu.row_perm[s]];
        col_perm[n_sparse + s] = border[s];
    }

    let mut row_perm_inv_full = vec![0usize; m];
    let mut col_perm_inv_full = vec![0usize; m];
    for step in 0..m {
        row_perm_inv_full[row_perm[step]] = step;
        col_perm_inv_full[col_perm[step]] = step;
    }

    let mut l_col: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for (orig_row, step, mult) in l_entries {
        l_col[step].push((row_perm_inv_full[orig_row], mult));
    }
    let mut u_row: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for (step, orig_col, val) in u_entries {
        u_row[step].push((col_perm_inv_full[orig_col], val));
    }
    for s in 0..k {
        for &(row_step, mult) in &border_lu.l_col[s] {
            l_col[n_sparse + s].push((n_sparse + row_step, mult));
        }
        for &(col_step, val) in &border_lu.u_row[s] {
            u_row[n_sparse + s].push((n_sparse + col_step, val));
        }
    }

    Some(LuFactors { m, l_col, u_row, row_perm, col_perm, col_perm_inv: col_perm_inv_full, row_perm_inv: row_perm_inv_full })
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

/// An eta's off-diagonal entries, chosen at construction time (see
/// [`pack_off_diag`]) between a sparse `(row_step, value)` list and a dense
/// length-`m` array (with the eta's own slot always left at `0.0`, so a
/// dense loop over the whole array never needs to special-case it). The
/// sparse form pays a per-entry tuple/indirection cost that is worth it
/// only while the eta is genuinely sparse; once an eta's own fill exceeds
/// [`DENSE_ETA_FRACTION`] of `m` (typical of a dense-coefficient LP, where
/// `U`'s eta chain is already close to fully dense from the very first
/// update), the dense form turns each consuming loop into a straight-line
/// scan with no per-entry branch or index indirection, which vectorizes
/// far better for the same total FLOP count. `nnz` is tracked separately
/// (not re-derived from the dense array's length, which is always `m`)
/// so [`FtLu::fill_count`]'s refactorization-trigger accounting keeps
/// measuring true fill regardless of which representation is in use.
#[derive(Clone)]
enum OffDiag {
    Sparse(Vec<(usize, f64)>),
    Dense { data: Box<[f64]>, nnz: usize },
}

/// A column/row whose off-diagonal fill exceeds this fraction of `m` is
/// stored densely (see [`OffDiag`]). Unlike [`DENSE_COL_FRACTION`] (tuned
/// against real Netlib data, all of it sparse), this threshold has no
/// dense-problem benchmark to tune against yet in this crate's own test
/// set — `0.4` is a first-pass value, not a measured one; re-tune once a
/// genuinely dense-coefficient LP is available to benchmark against.
const DENSE_ETA_FRACTION: f64 = 0.4;

fn pack_off_diag(m: usize, pairs: Vec<(usize, f64)>) -> OffDiag {
    let nnz = pairs.len();
    if nnz as f64 > DENSE_ETA_FRACTION * m as f64 {
        let mut data = vec![0.0; m];
        for (i, v) in pairs {
            data[i] = v;
        }
        OffDiag::Dense { data: data.into_boxed_slice(), nnz }
    } else {
        OffDiag::Sparse(pairs)
    }
}

impl OffDiag {
    fn nnz(&self) -> usize {
        match self {
            OffDiag::Sparse(v) => v.len(),
            OffDiag::Dense { nnz, .. } => *nnz,
        }
    }

    /// Removes any entry at `row_step == p` — used when an eta at an
    /// *earlier* `u_seq` position stops depending on a slot that just got
    /// moved to the end (see `try_update`'s own docs). Keeps `nnz` correct
    /// for the dense form too, rather than leaving it stale.
    fn remove_row(&mut self, p: usize) {
        match self {
            OffDiag::Sparse(v) => v.retain(|&(row_step, _)| row_step != p),
            OffDiag::Dense { data, nnz } => {
                if data[p] != 0.0 {
                    data[p] = 0.0;
                    *nnz -= 1;
                }
            }
        }
    }

    /// Materializes this eta's off-diagonal entries as owned `(row_step,
    /// value)` pairs. Only called on a cold, small-`nnz` path (`try_update`
    /// unregistering a slot's *old* content from `FtLu::row_owners` before
    /// overwriting it) — unlike `remove_row`/the dot-product loops above,
    /// this never runs once per row of `U`, so an intermediate `Vec` here
    /// costs nothing that matters.
    fn pairs(&self) -> Vec<(usize, f64)> {
        match self {
            OffDiag::Sparse(v) => v.clone(),
            OffDiag::Dense { data, .. } => data.iter().enumerate().filter(|&(_, &v)| v != 0.0).map(|(i, &v)| (i, v)).collect(),
        }
    }
}

#[derive(Clone)]
struct UEta {
    slot: usize,
    pivot: f64,
    off_diag: OffDiag, // (row_step, value) pairs, row_step != slot
}

#[derive(Clone)]
struct REta {
    p: usize,
    r: OffDiag, // (row_step, value) pairs, row_step != p
}

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
const DENSE_RHS_FRACTION: f64 = 0.4;

#[derive(Clone)]
pub struct FtLu {
    base: LuFactors,
    /// `U`'s etas, physically stored in **creation order** — exactly as
    /// before — so `u_transpose_solve_into`/`u_solve_into` (the hot,
    /// once-*every*-iteration BTRAN/FTRAN paths, not just `try_update`)
    /// keep a plain sequential scan with no pointer-chasing indirection.
    u_seq: Vec<UEta>,
    /// `slot_pos[slot]` is `slot`'s current index into `u_seq` — kept in
    /// sync by `try_update` over exactly the range its own
    /// `Vec::remove`/`push` already touches (see `try_update`'s own docs),
    /// so this costs nothing beyond what the reordering itself already
    /// pays. Turns "find slot p's eta" from the O(m) linear scan
    /// `find_seq_pos` used to do into an O(1) index.
    slot_pos: Vec<usize>,
    /// Reverse index: `row_owners[r]` lists every slot whose `off_diag`
    /// currently holds a nonzero at row-step `r`. `try_update`'s
    /// replace-column step (Tomlin 1974, eq. 12) must zero row `p` out of
    /// every *other* eta that still references it; before this index
    /// existed, that meant visiting all `m` etas in `U` and asking each
    /// "do you have an entry at `p`" (almost all answering no, but each
    /// still paying a full scan of its own off-diagonal list to say so).
    /// This answers "who has an entry at `p`" directly (combined with
    /// `slot_pos` above for O(1) access to each one), so only the
    /// (typically small — a few percent of `m`, per `ENOMOTO_DEBUG_ETA_DENSITY`
    /// measurements) handful that actually do ever get touched.
    row_owners: Vec<Vec<usize>>,
    r_etas: Vec<REta>,
    /// [`Self::try_update`]'s own reusable scratch (length `m`, always
    /// restored to that length before returning — see that method's own
    /// docs on the `mem::take`/restore pattern this exists for): avoids the
    /// two per-pivot heap allocations (`ftran_through_l_and_r`'s owned
    /// result, and building `e_p` in place) that method used to pay on
    /// *every* pivot commit, the same "fresh `Vec` every iteration" cost
    /// this crate's hot dual-simplex loops elsewhere already eliminated via
    /// caller-owned buffers (see e.g. `solve_into`'s own docs) — `try_update`
    /// itself was the one hot-path call in this file still allocating,
    /// found via `ENOMOTO_PROF_PHASES_EXT` naming `ft_update` as 13-20% of
    /// wall time on several Netlib instances with no single other phase
    /// anywhere near as consistently large.
    scratch_a_tilde: Vec<f64>,
    /// See [`Self::scratch_a_tilde`]'s own docs — the other of
    /// [`Self::try_update`]'s two scratch buffers.
    scratch_e_tilde: Vec<f64>,
    /// [`Self::u_transpose_solve_into`]'s own reusable epoch-stamped
    /// "needed" scratch (see that method's own docs): `(stamps, epoch)`
    /// where `stamps[s] == epoch` means step `s` is known to end up
    /// nonzero this call. A `RefCell` rather than a `&mut` parameter
    /// because `u_transpose_solve_into` and its callers
    /// (`solve_transpose_into`/`solve_transpose_into_capture`) are called
    /// through a shared `&FtLu` from many call sites across this crate;
    /// threading a new scratch parameter through all of them for an
    /// internal, call-local bookkeeping array would be a much larger,
    /// more invasive change for the same result. Never borrowed
    /// re-entrantly (this method doesn't call itself), so the `borrow_mut`
    /// can't panic.
    ut_needed: RefCell<(Vec<u32>, u32)>,
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
        let mut row_owners: Vec<Vec<usize>> = vec![Vec::new(); m];
        for (slot, pairs) in off_diags.iter().enumerate() {
            for &(row_step, _) in pairs {
                row_owners[row_step].push(slot);
            }
        }
        let u_seq: Vec<UEta> = (0..m)
            .map(|slot| UEta {
                slot,
                pivot: pivots[slot],
                off_diag: pack_off_diag(m, std::mem::take(&mut off_diags[slot])),
            })
            .collect();
        FtLu {
            base,
            u_seq,
            slot_pos: (0..m).collect(),
            row_owners,
            r_etas: Vec::new(),
            scratch_a_tilde: vec![0.0; m],
            scratch_e_tilde: vec![0.0; m],
            ut_needed: RefCell::new((vec![0; m], 0)),
        }
    }

    /// Whether an FTRAN right-hand side with `rhs_nnz` nonzero entries
    /// (out of this basis's `m`) is dense enough that callers should skip
    /// [`Self::solve_sparse_into`] in favor of the plain dense
    /// [`Self::solve_into`] — see [`DENSE_RHS_FRACTION`]'s own docs.
    pub fn should_use_dense_solve(&self, rhs_nnz: usize) -> bool {
        let m = self.base.m;
        m > 0 && rhs_nnz as f64 > DENSE_RHS_FRACTION * m as f64
    }

    /// `U_k^{-T}` applied in place to a step-space vector: processes the
    /// eta sequence in **forward** (creation) order, each step solving for
    /// that eta's pivotal component via eq. (8). Mutates `z` directly
    /// (rather than allocating a fresh result) — see `LuFactors::l_solve_into`'s
    /// own docs for why this matters on `FtLu`'s hot path.
    ///
    /// Hyper-sparse via [`Self::row_owners`], unlike [`Self::u_solve_into`]
    /// (see that method's own docs for why a GP-style DFS reach set was
    /// tried there and reverted): `z`'s input here is always a single unit
    /// vector's worth of nonzeros (BTRAN's callers only ever seed one
    /// entry before permutation), and this loop already visits `u_seq` in
    /// the one order (forward/creation order) in which every eta's
    /// `off_diag` targets are guaranteed to sit at strictly earlier
    /// positions (the same invariant `u_solve_into`'s reverse pass relies
    /// on, mirrored) — so marking "which later etas can possibly end up
    /// nonzero" is a single forward pass with no separate DFS/reach
    /// pre-pass needed: whenever this loop finds `z[p] != 0.0`, every slot
    /// listed in `row_owners[p]` (the etas whose `off_diag` reads row-step
    /// `p`) is marked needed, and any eta never marked needed is skipped
    /// outright. A skipped eta's `z[p]` is left at whatever `0 - 0 == 0`
    /// (or `-0.0`) it already held — never read by any downstream code as
    /// anything but "zero" (see `commit_update`'s `!= 0.0` filter, the
    /// `r_etas`/PRICE/DSE consumers immediately below and downstream of
    /// this call, all of which branch on zero-ness, not sign of zero), so
    /// every nonzero result is bit-identical to the unconditional scan.
    /// Measured (`analysis/greenbea_20260921_090812.md` §3-4): on
    /// `greenbea`, only ~3% of `u_seq` ends up nonzero per call, cutting
    /// this stage's wall time by more than half with the pivot sequence,
    /// iteration count, refactorization count and objective value all
    /// unchanged (bit-identical) across the full Netlib set.
    fn u_transpose_solve_into(&self, z: &mut [f64]) {
        let mut needed = self.ut_needed.borrow_mut();
        let (stamps, epoch) = &mut *needed;
        *epoch = epoch.wrapping_add(1);
        if *epoch == 0 {
            // Wrapped after ~4 billion calls: every stale stamp is now
            // indistinguishable from a real match at epoch 0, so clear
            // them once and restart from epoch 1.
            stamps.iter_mut().for_each(|s| *s = 0);
            *epoch = 1;
        }
        let epoch = *epoch;
        for (s, &zs) in z.iter().enumerate() {
            if zs != 0.0 {
                stamps[s] = epoch;
            }
        }
        for eta in &self.u_seq {
            let p = eta.slot;
            if stamps[p] != epoch {
                continue;
            }
            let y: f64 = match &eta.off_diag {
                OffDiag::Sparse(v) => v.iter().map(|&(row_step, v)| v * z[row_step]).sum(),
                // `data[p]` is always `0.0` (see `OffDiag`'s own docs), so
                // this dot product already excludes `z[p]` on its own.
                OffDiag::Dense { data, .. } => data.iter().zip(z.iter()).map(|(&v, &zi)| v * zi).sum(),
            };
            z[p] = (z[p] - y) / eta.pivot;
            if z[p] != 0.0 {
                for &q in &self.row_owners[p] {
                    stamps[q] = epoch;
                }
            }
        }
    }

    /// [`Self::u_transpose_solve_into`], restricted to `u_seq[start..]` —
    /// see [`FtLu::solve_transpose_unit_into`]'s own docs for why skipping
    /// the `[0, start)` prefix is *exact*, not approximate, whenever `z`
    /// is already known to be all-zero there on entry (true only for a
    /// freshly-factorized `u_seq` where Vec position equals slot, per
    /// that method's own precondition — never called on its own from
    /// anywhere `try_update` may have reordered `u_seq`).
    fn u_transpose_solve_from(&self, z: &mut [f64], start: usize) {
        for eta in &self.u_seq[start..] {
            let p = eta.slot;
            let y: f64 = match &eta.off_diag {
                OffDiag::Sparse(v) => v.iter().map(|&(row_step, v)| v * z[row_step]).sum(),
                OffDiag::Dense { data, .. } => data.iter().zip(z.iter()).map(|(&v, &zi)| v * zi).sum(),
            };
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
            match &eta.off_diag {
                OffDiag::Sparse(v) => {
                    for &(row_step, v) in v {
                        x[row_step] -= v * xp;
                    }
                }
                // `data[p] == 0.0` always, so this leaves `x[p]` (just
                // divided above) untouched, same as the sparse form.
                OffDiag::Dense { data, .. } => {
                    for (xi, &v) in x.iter_mut().zip(data.iter()) {
                        *xi -= v * xp;
                    }
                }
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
            let dot: f64 = match &reta.r {
                OffDiag::Sparse(v) => v.iter().map(|&(i, v)| v * z[i]).sum(),
                OffDiag::Dense { data, .. } => data.iter().zip(z.iter()).map(|(&v, &zi)| v * zi).sum(),
            };
            z[reta.p] -= dot;
        }
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

    /// Same as [`Self::solve_into`], but additionally captures the
    /// post-`L`/`R`, pre-`U` intermediate (`(L R_1...R_{k-1})^-1 rhs`) into
    /// `a_tilde_out` (length `m`) — exactly the `a_tilde` value
    /// [`Self::try_update_precomputed`] needs when `rhs` is the entering
    /// column being FTRAN'd this same iteration. See that method's own
    /// docs for why this capture (a plain `copy_from_slice`) lets the
    /// caller skip `try_update`'s own redundant re-derivation of the exact
    /// same value entirely.
    pub fn solve_into_capture(&self, rhs: &[f64], scratch: &mut [f64], out: &mut [f64], a_tilde_out: &mut [f64]) {
        self.ftran_through_l_and_r_into(rhs, scratch);
        a_tilde_out.copy_from_slice(scratch);
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
            let dot: f64 = match &reta.r {
                OffDiag::Sparse(v) => v.iter().map(|&(i, v)| v * scratch[i]).sum(),
                OffDiag::Dense { data, .. } => data.iter().zip(scratch.iter()).map(|(&v, &zi)| v * zi).sum(),
            };
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

    /// Same as [`Self::solve_sparse_into`], but additionally captures the
    /// post-`L`/`R`, pre-`U` intermediate into `a_tilde_out` (length `m`) —
    /// see [`Self::solve_into_capture`]'s own docs, which this mirrors for
    /// the sparse-`rhs` FTRAN path. The capture happens after the `R`-eta
    /// loop (this stage's own last write to `scratch` before `u_solve_into`
    /// takes over), so `a_tilde_out` ends up identical regardless of which
    /// of the two FTRAN paths (`should_use_dense_solve`'s dense/sparse
    /// dispatch) a given call took.
    pub fn solve_sparse_into_capture(
        &self,
        rhs_sparse: &[(usize, f64)],
        scratch: &mut [f64],
        gp: &mut GpScratch,
        out: &mut [f64],
        a_tilde_out: &mut [f64],
    ) {
        self.base.l_solve_sparse_into(rhs_sparse, scratch, gp);
        for reta in &self.r_etas {
            let dot: f64 = match &reta.r {
                OffDiag::Sparse(v) => v.iter().map(|&(i, v)| v * scratch[i]).sum(),
                OffDiag::Dense { data, .. } => data.iter().zip(scratch.iter()).map(|(&v, &zi)| v * zi).sum(),
            };
            scratch[reta.p] -= dot;
        }
        a_tilde_out.copy_from_slice(scratch);
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
            match &reta.r {
                OffDiag::Sparse(v) => {
                    for &(i, v) in v {
                        scratch[i] -= v * yp;
                    }
                }
                OffDiag::Dense { data, .. } => {
                    for (si, &v) in scratch.iter_mut().zip(data.iter()) {
                        *si -= v * yp;
                    }
                }
            }
        }
        self.base.l_transpose_solve_into(scratch, out);
    }

    /// Same as [`Self::solve_transpose_into`], but additionally captures
    /// the post-`U^-T`, pre-`R`-reverse intermediate into `e_tilde_out`
    /// (length `m`) — exactly the `e_tilde` value
    /// [`Self::try_update_precomputed`] needs when `rhs` is the unit
    /// vector at the leaving row (original indexing) being BTRAN'd this
    /// same iteration for `rho_p`. See that method's own docs for why this
    /// capture lets the caller skip `try_update`'s own redundant
    /// re-derivation of the exact same value.
    pub fn solve_transpose_into_capture(&self, rhs: &[f64], scratch: &mut [f64], out: &mut [f64], e_tilde_out: &mut [f64]) {
        let m = self.base.m;
        for s in 0..m {
            scratch[s] = rhs[self.base.col_perm[s]];
        }
        self.u_transpose_solve_into(scratch);
        e_tilde_out.copy_from_slice(scratch);
        // Hyper-sparse: same skip as `u_solve_into`/`l_solve_into` — `yp`
        // is the only value each `r_eta`'s entries get multiplied by here.
        for reta in self.r_etas.iter().rev() {
            let yp = scratch[reta.p];
            if yp == 0.0 {
                continue;
            }
            match &reta.r {
                OffDiag::Sparse(v) => {
                    for &(i, v) in v {
                        scratch[i] -= v * yp;
                    }
                }
                OffDiag::Dense { data, .. } => {
                    for (si, &v) in scratch.iter_mut().zip(data.iter()) {
                        *si -= v * yp;
                    }
                }
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

    /// Sparse-seed BTRAN specialized for `rhs = e_i` (a single unit vector
    /// at original row index `i`), built for [`super::DseState::from_basis`]'s
    /// own `m` back-to-back unit-vector solves after every refactorization
    /// — measured as 27% of `dfl001`'s total wall time before this method
    /// existed (`dfl001-bottleneck-max-iters-cap` memory), because that
    /// call site pays `solve_transpose_into`'s full `O(m)`-per-call cost
    /// `m` times over, every refactorization.
    ///
    /// **Requires `self.update_count() == 0`** — checked by the caller
    /// (`update_count()` is already the cheapest possible signal, so this
    /// method itself only asserts it rather than re-deriving it). The
    /// optimization below exploits a structural invariant of a *freshly
    /// factorized* `u_seq` (`FtLu::new`'s own construction: `u_seq[slot]`'s
    /// `off_diag` entries only ever reference `row_step < slot`, `U`'s own
    /// upper-triangular structure — the same property [`Self::u_solve_into`]'s
    /// own docs describe, mirrored for the transpose direction) that
    /// `try_update` is free to break (its own Forrest-Tomlin bump-and-
    /// replace algorithm reorders `u_seq` and can introduce entries
    /// referencing a *later* row-step than before) — so this method is
    /// only exact on a `u_seq` no `try_update` call has touched yet.
    ///
    /// **The optimization**: permuting `e_i` (`col_perm_inv[i]`) yields a
    /// single nonzero at step `s0`; [`Self::u_transpose_solve_into`]'s own
    /// forward recurrence can only ever produce a nonzero at slot `p` if
    /// some earlier slot `< p` it depends on is already nonzero — with
    /// nothing nonzero below `s0`, every slot `< s0` is therefore provably
    /// still `0` after the sweep, without computing a single one of their
    /// dot products. Starting the sweep at `s0` ([`Self::u_transpose_solve_from`])
    /// instead of `0` is exact, not approximate, and needs no DFS/epoch
    /// bookkeeping the way a full Gilbert-Peierls reach-set restriction
    /// would (see [`LuFactors::l_solve_sparse_into`]'s own docs for that
    /// technique, and `[[dfl001-bottleneck-max-iters-cap]]`/this crate's
    /// own history for why a *fuller* sparsification of the shared
    /// `u_transpose_solve_into` — applied to every per-iteration `rho_p`
    /// BTRAN, not just `from_basis`'s refactor-time calls — was tried and
    /// reverted as a net aggregate regression across the full Netlib set):
    /// that measurement's DFS/epoch overhead was paid on tens of thousands
    /// of per-iteration calls across many small problems where the skip
    /// bought little; this plain prefix skip carries no such per-call
    /// bookkeeping cost, and `from_basis`'s own access pattern (`m` calls,
    /// but only at refactor time) concentrates exactly on the large/
    /// refactor-heavy instances (`dfl001`, `pilot87`) a fuller
    /// sparsification would have helped too, without the small-problem
    /// dilution that sank the earlier attempt.
    ///
    /// `L^{-T}` (this BTRAN's tail, applied after the skip above via the
    /// unmodified [`LuFactors::l_transpose_solve_into`]) is left exactly
    /// as dense as it always was — see that method's own docs for why a
    /// *second* attempt to sparsify it specifically was never worth
    /// trying (its own input is typically no longer sparse by that point,
    /// fill having already spread across `[s0, m)` during the `U^{-T}`
    /// sweep above).
    ///
    /// **Precondition/postcondition** (mirrors [`Self::solve_sparse_into`]'s
    /// own convention): `scratch` must be all-zero on entry, and is
    /// restored to all-zero before returning — `L^{-T}`'s own reverse
    /// sweep can scatter fill back into positions below `s0`, so (unlike
    /// [`LuFactors::l_solve_sparse_into`]'s own narrower reach-set
    /// cleanup) nothing cheaper than a full `O(m)` reset is safe here.
    pub fn solve_transpose_unit_into(&self, i: usize, scratch: &mut [f64], out: &mut [f64]) {
        debug_assert_eq!(self.r_etas.len(), 0, "solve_transpose_unit_into requires a fresh (update-free) factorization");
        let s0 = self.base.col_perm_inv[i];
        scratch[s0] = 1.0;
        self.u_transpose_solve_from(scratch, s0);
        self.base.l_transpose_solve_into(scratch, out);
        scratch.fill(0.0);
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

        // `scratch_a_tilde`/`scratch_e_tilde` (see their own docs): taken out
        // of `self` (rather than borrowed) so the `&self` FTRAN/BTRAN calls
        // just below don't conflict with holding a `&mut` into one of
        // `self`'s own fields at the same time — restored to `self` right
        // after `commit_update` (which needs `&mut self`) is done reading
        // them. A length mismatch (only possible if a *previous* call
        // somehow left it empty, which no current code path does) falls
        // back to a fresh allocation rather than indexing out of bounds.
        let mut a_tilde = std::mem::take(&mut self.scratch_a_tilde);
        if a_tilde.len() != m {
            a_tilde = vec![0.0; m];
        }
        // `l_solve_into` (this function's own first step) fully overwrites
        // every entry of `a_tilde` before ever reading one back, so no
        // explicit zeroing is needed here regardless of what this buffer
        // held from its previous use.
        self.ftran_through_l_and_r_into(a_q_original, &mut a_tilde);

        // Unlike `a_tilde` above, `u_transpose_solve_into` mutates `z` as
        // *both* the input right-hand side and the evolving solution in
        // place (eq. 8's forward substitution) — reusing this buffer
        // without resetting every entry to the true input (`e_p`) first
        // would solve against whatever stale values its previous use left
        // behind instead. Zeroing it here is still one `O(m)` pass, exactly
        // as `vec![0.0; m]` used to pay for its own zero-initialization —
        // what this reuse actually saves is the allocator round-trip
        // itself, not this fill.
        let mut e_tilde = std::mem::take(&mut self.scratch_e_tilde);
        if e_tilde.len() != m {
            e_tilde = vec![0.0; m];
        } else {
            e_tilde.iter_mut().for_each(|v| *v = 0.0);
        }
        e_tilde[p] = 1.0;
        self.u_transpose_solve_into(&mut e_tilde);

        let result = self.commit_update(basis_slot, &a_tilde, &e_tilde, min_pivot);
        self.scratch_a_tilde = a_tilde;
        self.scratch_e_tilde = e_tilde;
        result
    }

    /// Same update as [`Self::try_update`], but for a caller that has
    /// *already computed* `a_tilde`/`e_tilde` this same iteration as an
    /// intermediate of its own FTRAN/BTRAN calls, and can hand them over
    /// directly instead of paying for [`Self::try_update`]'s own redundant
    /// re-derivation of both.
    ///
    /// **Why this exists**: a typical dual-simplex iteration already runs
    /// exactly the two solves `try_update` used to redo from scratch, for
    /// its own unrelated purposes — `rho_p = B^-T e_p` (`solve_transpose_into`,
    /// needed for PRICE) computes `U^-T e_p` as an internal step before
    /// applying the `R`-etas and `L^-T`, and the entering column's own FTRAN
    /// (`solve_into`/`solve_sparse_into`, needed for the primal update and
    /// DSE) computes `(L R_1...R_{k-1})^-1 a_q` as an internal step before
    /// applying `U^-1` — both are simply overwritten in place by the next
    /// stage rather than kept. Since `p`/`a_q_original` are identical
    /// between that earlier call and this update (same leaving row, same
    /// entering column, same iteration, `self` unchanged in between), the
    /// values are not merely *equivalent* to what `try_update` would
    /// recompute — they are bit-for-bit identical, `ftran_through_l_and_r_into`/
    /// `u_transpose_solve_into` being pure functions of `(self, input)` and
    /// neither `self` nor the input changing between the two computations.
    /// Capturing them (a plain `copy_from_slice`, via
    /// [`Self::solve_into_capture`]/[`Self::solve_sparse_into_capture`]/
    /// [`Self::solve_transpose_into_capture`]) is far cheaper than either of
    /// the two full solves this replaces — a dense `O(m)` pass through `L`
    /// plus every accumulated `R`-eta for `a_tilde`, and an `O(nnz(U))` scan
    /// of the whole eta chain for `e_tilde`, both of which grow as updates
    /// accumulate since the last refactorization.
    ///
    /// **Caller's responsibility**: `a_tilde`/`e_tilde` must come from a
    /// capture made *this same iteration*, for this exact `basis_slot` and
    /// the same `a_q_original` that is about to become basic — anything
    /// else (a stale capture from a discarded/refactorized iteration, or a
    /// mismatched `basis_slot`) silently corrupts the update with no way
    /// for this function to detect it, since it has no independent way to
    /// check what produced the slices it's handed.
    pub fn try_update_precomputed(&mut self, basis_slot: usize, a_tilde: &[f64], e_tilde: &[f64], min_pivot: f64) -> bool {
        self.commit_update(basis_slot, a_tilde, e_tilde, min_pivot)
    }

    /// Shared success/failure logic between [`Self::try_update`] (which
    /// computes `a_tilde`/`e_tilde` itself) and [`Self::try_update_precomputed`]
    /// (which takes them from the caller) — see the latter's own docs for
    /// why both end up needing exactly this same tail. Pulled out into its
    /// own `&mut self` method (rather than duplicated in both callers)
    /// specifically so the intricate `row_owners`/`slot_pos`/`u_seq`
    /// bookkeeping below — the part a copy-paste split would risk drifting
    /// out of sync between two copies — exists in exactly one place.
    fn commit_update(&mut self, basis_slot: usize, a_tilde: &[f64], e_tilde: &[f64], min_pivot: f64) -> bool {
        let m = self.base.m;
        let p = self.base.col_perm_inv[basis_slot];

        let seq_pos = self.slot_pos[p];
        let old_pivot = self.u_seq[seq_pos].pivot;

        let r_vec: Vec<(usize, f64)> =
            (0..m).filter(|&i| i != p).map(|i| (i, -old_pivot * e_tilde[i])).filter(|&(_, v)| v != 0.0).collect();

        let dot: f64 = r_vec.iter().map(|&(i, v)| v * a_tilde[i]).sum();
        let new_pivot = a_tilde[p] - dot;
        if new_pivot.abs() < min_pivot {
            return false;
        }

        // `Vec::remove` shifts every later element down by one position —
        // update `slot_pos` for exactly that range (elements the memmove
        // itself already touches, so this is no extra asymptotic cost)
        // rather than the old `find_seq_pos`'s full O(m) re-scan.
        let removed = self.u_seq.remove(seq_pos);
        for pos in seq_pos..self.u_seq.len() {
            self.slot_pos[self.u_seq[pos].slot] = pos;
        }

        // Unregister slot `p`'s *old* off-diagonal entries from
        // `row_owners` before overwriting them below — otherwise a stale
        // `p` would linger in some other row's owner list, pointing at
        // content that no longer exists there.
        for (row_step, _) in removed.off_diag.pairs() {
            if let Some(idx) = self.row_owners[row_step].iter().position(|&s| s == p) {
                self.row_owners[row_step].swap_remove(idx);
            }
        }

        // Zero row `p` out of every eta that still references it (Tomlin
        // 1974, eq. 12) — only the etas `row_owners[p]` actually lists,
        // not every eta in `U` (see `row_owners`'s own docs), each found
        // in O(1) via `slot_pos`.
        for slot in std::mem::take(&mut self.row_owners[p]) {
            let pos = self.slot_pos[slot];
            self.u_seq[pos].off_diag.remove_row(p);
        }

        let off_diag: Vec<(usize, f64)> =
            (0..m).filter(|&i| i != p && a_tilde[i] != 0.0).map(|i| (i, a_tilde[i])).collect();
        for &(row_step, _) in &off_diag {
            self.row_owners[row_step].push(p);
        }
        self.u_seq.push(UEta { slot: p, pivot: new_pivot, off_diag: pack_off_diag(m, off_diag) });
        self.slot_pos[p] = self.u_seq.len() - 1;

        self.r_etas.push(REta { p, r: pack_off_diag(m, r_vec) });

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
    /// Reads true nonzero counts via [`OffDiag::nnz`], not storage length,
    /// so switching an eta to the dense representation doesn't spuriously
    /// inflate this and trip the trigger early.
    pub fn fill_count(&self) -> usize {
        self.u_seq.iter().map(|e| e.off_diag.nnz()).sum::<usize>() + self.r_etas.iter().map(|e| e.r.nnz()).sum::<usize>()
    }

    /// Debug/instrumentation only: off-diagonal nonzero count of the `U`
    /// eta most recently appended by `try_update` (0 if no update has
    /// happened yet) — the fill-in from a single update, as opposed to
    /// `fill_count`'s running total. Used by `simplex.rs`'s
    /// `ENOMOTO_DEBUG_ETA_DENSITY` diagnostic to measure how eta density
    /// is distributed across a real solve, which is what motivated
    /// `OffDiag`'s sparse/dense hybrid representation above.
    pub fn last_update_off_diag_len(&self) -> usize {
        self.u_seq.last().map(|e| e.off_diag.nnz()).unwrap_or(0)
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

    /// Times `factorize_diagonal` against the general `factorize` on the
    /// exact shape `factorize_diagonal` exists for (a signed-identity
    /// initial basis), across the `m` range this crate's Netlib benchmark
    /// actually exercises. `#[ignore]`d for the same reason as
    /// `border_crossover_sweep` above: a live diagnostic, not a
    /// pass/fail correctness check.
    #[test]
    #[ignore]
    fn factorize_diagonal_vs_markowitz_sweep() {
        for &m in &[50, 200, 500, 1000, 2000, 4000] {
            let rows: Vec<Vec<(usize, f64)>> =
                (0..m).map(|i| vec![(i, if i % 7 == 0 { -1.0 } else { 1.0 })]).collect();

            let n_runs = 2000;
            let t0 = std::time::Instant::now();
            for _ in 0..n_runs {
                std::hint::black_box(factorize_diagonal(m, &rows).expect("diagonal"));
            }
            let diag_ns = t0.elapsed().as_nanos() as f64 / n_runs as f64;

            let t0 = std::time::Instant::now();
            for _ in 0..n_runs {
                std::hint::black_box(factorize(m, &rows).expect("diagonal, still nonsingular"));
            }
            let markowitz_ns = t0.elapsed().as_nanos() as f64 / n_runs as f64;

            println!(
                "m={m:5} factorize_diagonal={diag_ns:8.0}ns factorize(markowitz)={markowitz_ns:8.0}ns speedup={:.1}x",
                markowitz_ns / diag_ns
            );
        }
    }

    #[test]
    fn factorize_diagonal_matches_expected_solve() {
        let rows = vec![vec![(0, 1.0)], vec![(1, -1.0)], vec![(2, 1.0)]];
        let lu = factorize_diagonal(3, &rows).expect("diagonal input");
        assert!(lu.l_col.iter().all(Vec::is_empty), "L must be identity: {:?}", lu.l_col);
        assert_eq!(lu.row_perm, vec![0, 1, 2]);
        assert_eq!(lu.col_perm, vec![0, 1, 2]);

        let x_true = [3.0, -2.0, 5.0];
        let rhs = [1.0 * x_true[0], -1.0 * x_true[1], 1.0 * x_true[2]];
        let x = lu.solve(&rhs);
        assert!(approx_vec(&x, &x_true), "x={x:?}");
    }

    #[test]
    fn factorize_diagonal_rejects_off_diagonal_entries() {
        let rows = vec![vec![(0, 1.0), (1, 2.0)], vec![(1, 1.0)]];
        assert!(factorize_diagonal(2, &rows).is_none());

        let rows_wrong_col = vec![vec![(1, 1.0)], vec![(0, 1.0)]];
        assert!(factorize_diagonal(2, &rows_wrong_col).is_none());

        let rows_zero = vec![vec![(0, 0.0)], vec![(1, 1.0)]];
        assert!(factorize_diagonal(2, &rows_zero).is_none());
    }

    #[test]
    fn factorize_detects_singular() {
        // Row 2 = 2 * row 0 in a 3x3 with cols {0,1} only used -> column 2 empty -> singular.
        let rows = vec![vec![(0, 1.0), (1, 2.0)], vec![(0, 3.0), (1, 1.0)], vec![(0, 2.0), (1, 4.0)]];
        assert!(factorize(3, &rows).is_none());
    }

    /// A dense diagonally-dominant matrix well past `DENSE_INPUT_FRACTION`
    /// (100% fill), fed straight to `factorize_dense_faer` (not through
    /// `factorize`'s dispatch, to test this path in isolation regardless
    /// of where the threshold currently sits) and checked against a
    /// hand-verified solve, the same style `factorize_and_solve_matches_expected`
    /// uses for the Markowitz path — this is the ground truth that
    /// actually matters (not "does it match Markowitz's own answer",
    /// which would only prove the two agree with *each other*, not with
    /// reality).
    #[test]
    fn factorize_dense_faer_matches_hand_verified_solve() {
        let m = 8;
        let entry = |i: usize, j: usize| -> f64 { if i == j { 50.0 } else { 1.0 + ((i * 3 + j * 7) % 11) as f64 * 0.4 } };
        let rows: Vec<Vec<(usize, f64)>> = (0..m).map(|i| (0..m).map(|j| (j, entry(i, j))).collect()).collect();

        let lu = factorize_dense_faer(m, &rows).expect("diagonally dominant must be nonsingular");
        let x_true: Vec<f64> = (0..m).map(|i| 1.0 + i as f64 * 0.5).collect();
        let rhs: Vec<f64> = (0..m).map(|i| (0..m).map(|j| entry(i, j) * x_true[j]).sum()).collect();
        let x = lu.solve(&rhs);
        assert!(approx_vec(&x, &x_true), "x={x:?} x_true={x_true:?}");

        // B^T y = rhs2, same cross-check for the transpose solve path.
        let y_true: Vec<f64> = (0..m).map(|i| 0.3 - i as f64 * 0.2).collect();
        let rhs2: Vec<f64> = (0..m).map(|j| (0..m).map(|i| entry(i, j) * y_true[i]).sum()).collect();
        let y = lu.solve_transpose(&rhs2);
        assert!(approx_vec(&y, &y_true), "y={y:?} y_true={y_true:?}");
    }

    /// `factorize` itself (the public dispatcher) must route this input to
    /// `factorize_dense_faer` — confirms `is_dense_input` actually fires
    /// for a fully dense matrix at a size realistic for this crate's
    /// target problems, not just in the tiny fixtures the Markowitz-path
    /// tests use (which could accidentally clear a generous threshold too).
    #[test]
    fn factorize_dispatches_dense_input_to_faer() {
        let m = 20;
        let rows: Vec<Vec<(usize, f64)>> =
            (0..m).map(|i| (0..m).map(|j| (j, if i == j { 30.0 } else { 1.0 })).collect()).collect();
        assert!(is_dense_input(m, &rows), "fully dense {m}x{m} input must be flagged dense");
        let lu = factorize(m, &rows).expect("nonsingular");
        let x_true = vec![1.0; m];
        let rhs: Vec<f64> = (0..m).map(|_| 30.0 + (m - 1) as f64).collect();
        let x = lu.solve(&rhs);
        assert!(approx_vec(&x, &x_true), "x={x:?}");
    }

    /// A dense but rank-deficient matrix (two identical rows) must still
    /// be reported as singular through the `faer` path, exactly as the
    /// Markowitz path already does for its own sparse singular fixture
    /// (`factorize_detects_singular`, above).
    #[test]
    fn factorize_dense_faer_detects_singular() {
        let m = 6;
        let mut rows: Vec<Vec<(usize, f64)>> =
            (0..m).map(|i| (0..m).map(|j| (j, 1.0 + ((i + j) % 4) as f64)).collect()).collect();
        rows[3] = rows[1].clone(); // row 3 duplicates row 1 -> rank-deficient
        assert!(factorize_dense_faer(m, &rows).is_none());
    }

    /// `fit1p`-shaped fixture for [`factorize_bordered`]: `m - k` "local"
    /// rows each with one sparse entry of their own plus every border
    /// column, and `k` purely-border rows forming an invertible `k x k`
    /// core — checked against `factorize_flat_markowitz`'s own answer for
    /// the same matrix (both must solve `Bx = rhs` correctly, not just
    /// agree with each other, so `rhs` is built from a known `x_true`
    /// exactly as `factorize_and_solve_matches_expected` does).
    #[test]
    fn factorize_bordered_matches_flat_markowitz() {
        let m = 10;
        let border = [7usize, 8, 9];
        let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
        for i in 0..7 {
            rows.push(vec![(i, 3.0 + i as f64), (7, 1.0), (8, 1.0 + 0.1 * i as f64), (9, 2.0 - 0.1 * i as f64)]);
        }
        rows.push(vec![(7, 4.0), (8, 1.0), (9, 0.0)]);
        rows.push(vec![(7, 1.0), (8, 3.0), (9, 1.0)]);
        rows.push(vec![(7, 0.0), (8, 1.0), (9, 5.0)]);
        assert_eq!(rows.len(), m);

        let lu_bordered = factorize_bordered(m, &rows, &border).expect("bordered factorization should succeed");
        let lu_flat = factorize_flat_markowitz(m, &rows).expect("plain Markowitz should also succeed");

        let x_true: Vec<f64> = (0..m).map(|i| 1.0 + i as f64 * 0.3).collect();
        let entry = |row: &[(usize, f64)], j: usize| row.iter().find(|&&(c, _)| c == j).map(|&(_, v)| v).unwrap_or(0.0);
        let rhs: Vec<f64> = rows.iter().map(|row| (0..m).map(|j| entry(row, j) * x_true[j]).sum()).collect();

        let x_bordered = lu_bordered.solve(&rhs);
        let x_flat = lu_flat.solve(&rhs);
        assert!(approx_vec(&x_bordered, &x_true), "bordered x={x_bordered:?}");
        assert!(approx_vec(&x_flat, &x_true), "flat x={x_flat:?}");
    }

    /// Larger version of the same fixture (30 rows, 6 border columns) to
    /// catch indexing bugs a tiny fixture could miss (e.g. an off-by-one
    /// in the `n_sparse`/border step-offset arithmetic).
    #[test]
    fn factorize_bordered_matches_flat_markowitz_larger() {
        let m = 30;
        let k = 6;
        let border: Vec<usize> = (m - k..m).collect();
        let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
        for i in 0..(m - k) {
            let mut row = vec![(i, 5.0 + i as f64)];
            for (bi, &b) in border.iter().enumerate() {
                row.push((b, 1.0 + 0.1 * ((i + bi) % 5) as f64));
            }
            rows.push(row);
        }
        // Diagonally dominant k x k border core -> nonsingular.
        for bi in 0..k {
            let mut row = Vec::new();
            for (bj, &b2) in border.iter().enumerate() {
                let v = if bi == bj { 10.0 } else { 1.0 + ((bi + bj) % 3) as f64 * 0.2 };
                row.push((b2, v));
            }
            rows.push(row);
        }
        assert_eq!(rows.len(), m);

        let lu_bordered = factorize_bordered(m, &rows, &border).expect("bordered factorization should succeed");
        let lu_flat = factorize_flat_markowitz(m, &rows).expect("plain Markowitz should also succeed");

        let x_true: Vec<f64> = (0..m).map(|i| 1.0 + i as f64 * 0.13).collect();
        let entry = |row: &[(usize, f64)], j: usize| row.iter().find(|&&(c, _)| c == j).map(|&(_, v)| v).unwrap_or(0.0);
        let rhs: Vec<f64> = rows.iter().map(|row| (0..m).map(|j| entry(row, j) * x_true[j]).sum()).collect();

        let x_bordered = lu_bordered.solve(&rhs);
        let x_flat = lu_flat.solve(&rhs);
        assert!(approx_vec(&x_bordered, &x_true), "bordered x={x_bordered:?}");
        assert!(approx_vec(&x_flat, &x_true), "flat x={x_flat:?}");
    }

    /// A border column that is genuinely required as a pivot before the
    /// sparse phase can finish (the two sparse columns share a
    /// proportional pattern in the only rows that touch them at all) must
    /// make `factorize_bordered` bail out with `None` rather than produce
    /// wrong factors — and per the rank argument in this file's own docs
    /// (`rank(A) <= rank(A_sparse) + k`), a sparse phase that cannot find
    /// `m - k` independent pivots means the *full* matrix is genuinely
    /// singular too, which `factorize`'s own dispatch (falling back to
    /// `factorize_flat_markowitz`) must also report as such.
    #[test]
    fn factorize_bordered_falls_back_to_none_on_stuck_sparse_phase() {
        let m = 3;
        let border = [2usize];
        let rows = vec![
            vec![(0, 1.0), (1, 2.0)],
            vec![(0, 2.0), (1, 4.0), (2, 1.0)], // sparse part proportional to row 0
            vec![(2, 5.0)],
        ];
        assert!(factorize_bordered(m, &rows, &border).is_none());
        assert!(factorize_flat_markowitz(m, &rows).is_none(), "matrix is genuinely singular");
        assert!(factorize(m, &rows).is_none());
    }

    /// Builds an `fit1p`-shaped arrowhead matrix at a chosen `m` and
    /// border fraction `k/m`: `m - k` "local" rows each with one local
    /// sparse entry (own diagonal-ish column) plus every border column,
    /// and `k` purely-border rows forming a diagonally dominant (hence
    /// nonsingular) `k x k` core — the same shape
    /// `factorize_bordered_matches_flat_markowitz_larger` uses, just
    /// parameterized for the crossover sweep below.
    fn arrowhead(m: usize, k: usize) -> (Vec<Vec<(usize, f64)>>, Vec<usize>) {
        let border: Vec<usize> = (m - k..m).collect();
        let mut rows: Vec<Vec<(usize, f64)>> = Vec::with_capacity(m);
        for i in 0..(m - k) {
            let mut row = vec![(i, 5.0 + i as f64)];
            for (bi, &b) in border.iter().enumerate() {
                row.push((b, 1.0 + 0.1 * ((i + bi) % 5) as f64));
            }
            rows.push(row);
        }
        for bi in 0..k {
            let mut row = Vec::with_capacity(k);
            for (bj, &b2) in border.iter().enumerate() {
                let v = if bi == bj { 10.0 * k as f64 } else { 1.0 + ((bi + bj) % 3) as f64 * 0.2 };
                row.push((b2, v));
            }
            rows.push(row);
        }
        (rows, border)
    }

    /// **Diagnostic, not a correctness test** (`#[ignore]`d — run
    /// explicitly via `cargo test --release -- --ignored --nocapture
    /// border_crossover`): sweeps the border fraction `k/m` at a fixed
    /// `m` on the synthetic `arrowhead` shape above and times
    /// `factorize_bordered` against `factorize_flat_markowitz`, to find
    /// where [`BORDER_MAX_FRACTION`]'s `0.3` cap should actually sit —
    /// that constant's own docs candidly note it was never tuned against
    /// real data (Netlib's own `fit1p`/`fit2p` family only ever exercises
    /// `k/m` in the few-percent range). Kept as a live diagnostic (like
    /// `debug_print_block_sizes`) rather than deleted, since a future
    /// problem shape or a revisit of the threshold can just rerun it.
    #[test]
    #[ignore]
    fn border_crossover_sweep() {
        let m = 800;
        for &frac in &[0.02, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5] {
            let k = ((m as f64) * frac).round() as usize;
            if k == 0 || k >= m {
                continue;
            }
            let (rows, border) = arrowhead(m, k);
            let nnz: usize = rows.iter().map(|r| r.len()).sum();
            let dense = is_dense_input(m, &rows);

            let n_runs = 20;
            let t0 = std::time::Instant::now();
            for _ in 0..n_runs {
                std::hint::black_box(factorize_bordered(m, &rows, &border).expect("nonsingular"));
            }
            let bordered_us = t0.elapsed().as_micros() as f64 / n_runs as f64;

            let t0 = std::time::Instant::now();
            for _ in 0..n_runs {
                std::hint::black_box(factorize_flat_markowitz(m, &rows).expect("nonsingular"));
            }
            let flat_us = t0.elapsed().as_micros() as f64 / n_runs as f64;

            println!(
                "m={m} k={k} k/m={frac:.2} nnz_frac={:.3} is_dense_input={dense} bordered={bordered_us:.1}us flat={flat_us:.1}us speedup={:.2}x",
                nnz as f64 / (m * m) as f64,
                flat_us / bordered_us
            );
        }
    }

    /// Second half of the sweep: the arrowhead shape above makes border
    /// columns *fully* dense (every local row touches every border
    /// column), which means `nnz` grows with `k` fast enough to trip
    /// [`is_dense_input`]'s own 25%-of-`m^2` gate on its own once `k/m`
    /// crosses roughly that same 25% (a border column population of `k`
    /// fully-dense columns alone already contributes `k/m` density) — so
    /// the sweep above never actually exercises `factorize_bordered`
    /// against a *genuinely sparse-overall* `flat_markowitz` at large
    /// `k/m`; `factorize`'s own `is_dense_input` check would already have
    /// routed those cases to `factorize_dense_faer` before `k/m` ever
    /// became this function's own problem. This variant holds overall
    /// density far below that gate by making border columns only
    /// partially populated (`border_density`), to see whether `k/m` still
    /// has a *genuine* independent crossover once that confound is
    /// removed.
    fn arrowhead_partial(m: usize, k: usize, border_density: f64) -> (Vec<Vec<(usize, f64)>>, Vec<usize>) {
        let border: Vec<usize> = (m - k..m).collect();
        let mut rows: Vec<Vec<(usize, f64)>> = Vec::with_capacity(m);
        let step = (1.0 / border_density).round().max(1.0) as usize;
        for i in 0..(m - k) {
            let mut row = vec![(i, 5.0 + i as f64)];
            for (bi, &b) in border.iter().enumerate() {
                if (i + bi) % step == 0 {
                    row.push((b, 1.0 + 0.1 * ((i + bi) % 5) as f64));
                }
            }
            rows.push(row);
        }
        for bi in 0..k {
            let mut row = Vec::with_capacity(k);
            for (bj, &b2) in border.iter().enumerate() {
                let v = if bi == bj { 10.0 * k as f64 } else { 1.0 + ((bi + bj) % 3) as f64 * 0.2 };
                row.push((b2, v));
            }
            rows.push(row);
        }
        (rows, border)
    }

    /// Runs one `(m, k/m)` point of the partial-density sweep and prints
    /// bordered-vs-`factorize_flat_markowitz` timing (the latter dispatches
    /// to `factorize_dense_faer` itself once `is_dense_input` fires, so
    /// this is really "bordered vs whatever `factorize` would otherwise
    /// pick" once density crosses that gate).
    fn run_border_crossover_point(m: usize, frac: f64, n_runs: usize) {
        let k = ((m as f64) * frac).round() as usize;
        if k == 0 || k >= m {
            return;
        }
        let (rows, border) = arrowhead_partial(m, k, 0.3);
        let nnz: usize = rows.iter().map(|r| r.len()).sum();
        let dense = is_dense_input(m, &rows);

        let t0 = std::time::Instant::now();
        for _ in 0..n_runs {
            std::hint::black_box(factorize_bordered(m, &rows, &border).expect("nonsingular"));
        }
        let bordered_us = t0.elapsed().as_micros() as f64 / n_runs as f64;

        let t0 = std::time::Instant::now();
        for _ in 0..n_runs {
            std::hint::black_box(factorize_flat_markowitz(m, &rows).expect("nonsingular"));
        }
        let flat_us = t0.elapsed().as_micros() as f64 / n_runs as f64;

        println!(
            "m={m} k={k} k/m={frac:.2} nnz_frac={:.3} is_dense_input={dense} bordered={bordered_us:.1}us flat={flat_us:.1}us speedup={:.2}x",
            nnz as f64 / (m * m) as f64,
            flat_us / bordered_us
        );
    }

    #[test]
    #[ignore]
    fn border_crossover_sweep_partial_density() {
        for &frac in &[0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.6, 0.7] {
            run_border_crossover_point(800, frac, 20);
        }
    }

    /// Fine-grained pass around the `m=800` crossover found above
    /// (bordered wins at `k/m=0.50`, loses at `0.60`) to pin it down more
    /// precisely.
    #[test]
    #[ignore]
    fn border_crossover_sweep_fine() {
        for &frac in &[0.50, 0.52, 0.54, 0.56, 0.58, 0.60] {
            run_border_crossover_point(800, frac, 20);
        }
    }

    /// Same `k/m` points at a different `m` (`2000` instead of `800`), to
    /// tell whether the crossover found above is a genuine *fraction*
    /// (`k/m`) effect — in which case this should land at roughly the same
    /// `k/m` — or actually an *absolute-`k`* effect (the `k x k` Schur
    /// complement's own `O(k^3)` dense factorization cost), in which case
    /// a larger `m` should cross over at a *smaller* `k/m` (same absolute
    /// `k`).
    #[test]
    #[ignore]
    fn border_crossover_sweep_scaling() {
        for &frac in &[0.2, 0.3, 0.4, 0.5, 0.6] {
            run_border_crossover_point(2000, frac, 5);
        }
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

    /// Direct regression test for the whole premise behind
    /// `try_update_precomputed`/`solve_into_capture`/`solve_sparse_into_capture`/
    /// `solve_transpose_into_capture` (see their own docs): feeding
    /// `try_update` a manually-recomputed `a_tilde`/`e_tilde` vs. feeding
    /// `try_update_precomputed` the *captured* intermediate from an
    /// otherwise-ordinary FTRAN/BTRAN call for the same `basis_slot`/
    /// `a_q_original` must produce bit-identical results — not merely
    /// close ones — since both are meant to compute exactly the same
    /// values. Runs on a state that already has two prior updates applied
    /// (non-trivial `u_seq`/`r_etas`), the realistic case, not just a
    /// freshly-refactored one, and checks both the dense
    /// (`solve_into_capture`) and sparse (`solve_sparse_into_capture`)
    /// FTRAN capture paths independently.
    #[test]
    fn try_update_precomputed_matches_try_update() {
        let rows0 = vec![vec![(0, 2.0), (1, 1.0)], vec![(0, 1.0), (1, 3.0), (2, 1.0)], vec![(1, 1.0), (2, 4.0)]];
        let base = factorize(3, &rows0).unwrap();
        let mut state = FtLu::new(base);
        assert!(state.try_update(1, &[1.0, 5.0, 2.0], 1e-9));
        assert!(state.try_update(0, &[4.0, 1.0, 3.0], 1e-9));

        let m = 3;
        let basis_slot = 2;
        let a_q = [2.0, 1.0, 6.0];

        // Reference: plain `try_update`, which recomputes `a_tilde`/`e_tilde`
        // itself from scratch.
        let mut state_ref = state.clone();
        assert!(state_ref.try_update(basis_slot, &a_q, 1e-9));

        // Dense-capture path: `solve_into_capture` (as `simplex.rs`'s
        // dense-rhs bypass branch uses it) supplies `a_tilde`, and
        // `solve_transpose_into_capture` (as its `rho_p` BTRAN uses it)
        // supplies `e_tilde`.
        let mut state_dense = state.clone();
        let (mut scratch, mut out, mut a_tilde) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
        state_dense.solve_into_capture(&a_q, &mut scratch, &mut out, &mut a_tilde);
        let mut e_p = vec![0.0; m];
        e_p[basis_slot] = 1.0;
        let (mut scratch2, mut out2, mut e_tilde) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
        state_dense.solve_transpose_into_capture(&e_p, &mut scratch2, &mut out2, &mut e_tilde);
        assert!(state_dense.try_update_precomputed(basis_slot, &a_tilde, &e_tilde, 1e-9));

        // Sparse-capture path: `solve_sparse_into_capture` (as
        // `simplex.rs`'s sparse-rhs branch uses it) supplies `a_tilde`
        // instead — must land on the exact same intermediate despite going
        // through the Gilbert-Peierls reach-set machinery rather than a
        // dense scan.
        let mut state_sparse = state.clone();
        let (mut sscratch, mut sout, mut sa_tilde) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
        let mut gp = GpScratch::new(m);
        state_sparse.solve_sparse_into_capture(&to_sparse(&a_q), &mut sscratch, &mut gp, &mut sout, &mut sa_tilde);
        let (mut sscratch2, mut sout2, mut se_tilde) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
        state_sparse.solve_transpose_into_capture(&e_p, &mut sscratch2, &mut sout2, &mut se_tilde);
        assert!(state_sparse.try_update_precomputed(basis_slot, &sa_tilde, &se_tilde, 1e-9));

        for rhs in [[3.0, -2.0, 7.0], [1.0, 0.0, -1.0], [0.5, 0.5, 0.5]] {
            let x_ref = state_ref.solve(&rhs);
            let x_dense = state_dense.solve(&rhs);
            let x_sparse = state_sparse.solve(&rhs);
            assert_eq!(x_ref, x_dense, "dense-capture diverged from try_update on solve: rhs={rhs:?}");
            assert_eq!(x_ref, x_sparse, "sparse-capture diverged from try_update on solve: rhs={rhs:?}");

            let y_ref = state_ref.solve_transpose(&rhs);
            let y_dense = state_dense.solve_transpose(&rhs);
            let y_sparse = state_sparse.solve_transpose(&rhs);
            assert_eq!(y_ref, y_dense, "dense-capture diverged from try_update on solve_transpose: rhs={rhs:?}");
            assert_eq!(y_ref, y_sparse, "sparse-capture diverged from try_update on solve_transpose: rhs={rhs:?}");
        }
    }


    /// Deterministic xorshift-ish LCG, no external `rand` dependency
    /// needed for a test fixture this small.
    fn next_rand(state: &mut u64) -> f64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((*state >> 33) as f64 / (1u64 << 31) as f64) - 1.0
    }

    /// A moderately-sized, genuinely off-diagonal sparse matrix (diagonal
    /// dominance guarantees `factorize` never needs a singularity
    /// fallback) — big enough that `factorize`'s own Markowitz pivoting
    /// produces a non-identity `col_perm`/`row_perm` and real off-diagonal
    /// `U`/`L` fill, unlike the crate's other, smaller hand-written
    /// fixtures.
    fn random_sparse_diag_dominant(m: usize, seed: u64) -> Vec<Vec<(usize, f64)>> {
        let mut state = seed;
        let mut rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
        for i in 0..m {
            let mut off_sum = 0.0f64;
            let n_off = 3.min(m - 1);
            let mut cols: Vec<usize> = Vec::with_capacity(n_off);
            for _ in 0..n_off {
                let j = ((next_rand(&mut state).abs() * m as f64) as usize).min(m - 1);
                if j != i && !cols.contains(&j) {
                    cols.push(j);
                }
            }
            for &j in &cols {
                let v = next_rand(&mut state) * 2.0;
                off_sum += v.abs();
                rows[i].push((j, v));
            }
            rows[i].push((i, off_sum + 5.0 + next_rand(&mut state).abs()));
            rows[i].sort_unstable_by_key(|&(c, _)| c);
        }
        rows
    }

    #[test]
    fn solve_transpose_unit_into_matches_dense_on_fresh_factorization() {
        let m = 40;
        for seed in [1u64, 2, 3, 4, 5] {
            let rows = random_sparse_diag_dominant(m, seed);
            let base = factorize(m, &rows).expect("diagonally dominant matrix must factorize");
            let state = FtLu::new(base);
            assert_eq!(state.update_count(), 0, "fresh factorization must have no updates");

            let mut scratch = vec![0.0; m];
            let mut out = vec![0.0; m];
            for i in 0..m {
                let mut e_i = vec![0.0; m];
                e_i[i] = 1.0;
                let expected = state.solve_transpose(&e_i);

                state.solve_transpose_unit_into(i, &mut scratch, &mut out);
                assert_eq!(out, expected, "seed={seed} i={i}: solve_transpose_unit_into diverged from dense solve_transpose");
                assert!(scratch.iter().all(|&v| v == 0.0), "seed={seed} i={i}: scratch not restored to all-zero");
            }
        }
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

    #[test]
    fn should_use_dense_solve_flags_dense_rhs_and_not_sparse() {
        let m = 10;
        let rows: Vec<Vec<(usize, f64)>> = (0..m).map(|i| vec![(i, 4.0)]).collect();
        let lu = FtLu::new(factorize(m, &rows).expect("nonsingular"));
        // A rhs with 5 of 10 entries nonzero exceeds DENSE_RHS_FRACTION (0.4).
        assert!(lu.should_use_dense_solve(5), "5/10 nonzero rhs should be flagged dense");
        assert!(!lu.should_use_dense_solve(2), "2/10 nonzero rhs should not be flagged dense");
    }

    /// A dense-coefficient basis (every column of `B` has all `m` entries,
    /// well past `DENSE_ETA_FRACTION` after a couple of FT updates) run
    /// through several `try_update` calls with equally dense entering
    /// columns, then cross-checked against a completely independent full
    /// refactorization of the final basis — the same style of ground truth
    /// the sparse-fixture tests above use, just sized and shaped to
    /// actually exercise `OffDiag::Dense` instead of `OffDiag::Sparse`.
    #[test]
    fn ft_update_matches_full_refactor_on_dense_basis() {
        let m = 10;
        let entry = |i: usize, j: usize| -> f64 { if i == j { 50.0 } else { 1.0 + ((i + 2 * j) % 5) as f64 * 0.3 } };
        let rows0: Vec<Vec<(usize, f64)>> = (0..m).map(|i| (0..m).map(|j| (j, entry(i, j))).collect()).collect();
        let base = factorize(m, &rows0).expect("nonsingular");
        let mut state = FtLu::new(base);

        // Dense entering columns (every entry nonzero), replacing a few
        // different slots.
        let entering = |k: usize, slot: usize| -> Vec<f64> {
            (0..m).map(|i| if i == slot { 40.0 + k as f64 } else { 2.0 + ((i * 3 + k) % 7) as f64 }).collect()
        };
        let updates = [(3usize, 0usize), (7, 1), (0, 2)];
        let mut cur_rows = rows0.clone();
        for &(k, slot) in &updates {
            let a_q = entering(k, slot);
            assert!(state.try_update(slot, &a_q, 1e-9), "update on slot {slot} rejected");
            for i in 0..m {
                cur_rows[i].retain(|&(c, _)| c != slot);
                if a_q[i] != 0.0 {
                    cur_rows[i].push((slot, a_q[i]));
                }
            }
        }

        let full = factorize(m, &cur_rows).expect("updated dense basis must still be nonsingular");
        let rhs: Vec<f64> = (0..m).map(|i| 1.0 + i as f64 * 0.5).collect();

        let x_ft = state.solve(&rhs);
        let x_full = full.solve(&rhs);
        assert!(approx_vec(&x_ft, &x_full), "ft={x_ft:?} full={x_full:?}");

        let y_ft = state.solve_transpose(&rhs);
        let y_full = full.solve_transpose(&rhs);
        assert!(approx_vec(&y_ft, &y_full), "ft={y_ft:?} full={y_full:?}");

        // `solve_sparse_into` must still agree too, even though a dense
        // rhs is routed around it at the `simplex.rs` call sites (see
        // `should_use_dense_solve`'s own docs) — it remains public API and
        // must stay correct regardless of caller choice.
        let mut scratch = vec![0.0; m];
        let mut gp = GpScratch::new(m);
        let mut out = vec![0.0; m];
        state.solve_sparse_into(&to_sparse(&rhs), &mut scratch, &mut gp, &mut out);
        assert!(approx_vec(&out, &x_full), "sparse-rhs path ft={out:?} full={x_full:?}");
    }
}
