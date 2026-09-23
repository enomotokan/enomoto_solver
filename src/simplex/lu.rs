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
//! counts) live in `col_degree`/`row_degree`, and columns additionally in
//! bucket arrays (`col_buckets[d]`, each a `VecDeque` of the columns
//! currently at degree `d`), with O(1) bucket moves via a parallel position
//! index (`col_bucket_pos`,
//! swap-to-last-then-pop on removal — the same pattern `factorize`'s
//! earlier `active_rows` bookkeeping used). Crucially, a **column-major
//! mirror** (the exact set of currently-active rows with a nonzero at
//! column `j`) is maintained alongside the row-major active submatrix,
//! kept in sync on every insert/remove during elimination. This is what
//! makes the whole scheme actually sub-`O(m)` per step rather than just
//! relocating the same cost: "which rows does eliminating column `pj`
//! affect" is answered by that column's own live-row list directly (cost =
//! that column's own current degree) instead of scanning every active row
//! to test whether it still holds `pj`, and "how many rows still touch
//! column `j`" is that list's length (O(1)) instead of a fresh
//! full-matrix scan. Both the submatrix and its mirror are **flat arrays**
//! ([`KernelMatrix`], HiGHS's own `HFactor` `mc_*`/`mr_*` layout), not
//! `BTreeMap`/`BTreeSet` containers — see that struct's own docs, and
//! [`MarkowitzState::eliminate`]'s, for why the inner elimination loop is
//! a sorted merge over contiguous runs rather than one keyed tree descent
//! per element touched. An
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
//! live-row list too (not just its own pivot column's), so no column's
//! degree can drift stale by continuing to count an inactive row.
//!
//! Within a factorization, per-pivot cost is `O(pivot column's degree)` for
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

use crate::sparse::{CscBuilder, CscMat, CsrMat, EpochMarks, HybridVec};
use std::cell::Cell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

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
const STABILITY: f64 = 0.25;

/// Ceiling on the escalated pivot threshold ([`escalate_pivot_threshold`])
/// — HiGHS's own `kMaxPivotThreshold`. [`STABILITY`]'s docs record that a
/// *static* `0.5` costs ~4% on `d2q06c`/`greenbeb`/`fit2p` through extra
/// fill-in, which is exactly why this value is reachable only after the
/// escalation ladder below has evidence that *this* solve is paying more
/// for instability than it would for fill.
const PIVOT_THRESHOLD_MAX: f64 = 0.5;

/// Floor for an operator-supplied `ENOMOTO_PIVOT_THRESHOLD` — HiGHS's own
/// `kMinPivotThreshold`. Nothing escalates *downwards*, so this only ever
/// clamps the env override.
const PIVOT_THRESHOLD_MIN: f64 = 8e-4;

/// Multiplier applied per [`escalate_pivot_threshold`] step. HiGHS uses
/// `kPivotThresholdChangeFactor = 5.0` from a `0.1` default; from this
/// crate's `0.25` a factor of `2.0` lands exactly on
/// [`PIVOT_THRESHOLD_MAX`] in one step, so the ladder here is
/// `0.25 -> 0.5`, and a second escalation is a no-op.
const PIVOT_THRESHOLD_FACTOR: f64 = 2.0;

thread_local! {
    /// The pivot threshold in force for *this thread's* current solve —
    /// `None` until first read, then [`pivot_threshold_base`].
    ///
    /// Thread-local rather than a field threaded through `factorize`'s
    /// half-dozen entry points (and their callers in `simplex.rs`,
    /// `extended_dual.rs`, `mip.rs`) because it is a *solve*-scoped
    /// setting in exactly the way HiGHS's own
    /// `info_.factor_pivot_threshold` is: one value, read once per
    /// factorization, written only by the simplex loop that owns the
    /// solve. Thread-local (not a `static`) keeps concurrent solves —
    /// `mip.rs` runs LP relaxations on rayon workers — from escalating
    /// each other's thresholds, which a shared global would do while also
    /// making both solves' pivot sequences depend on the interleaving.
    /// Every solve entry point calls [`reset_pivot_threshold`] before its
    /// first factorization, so a thread that ran a troublesome solve does
    /// not hand the escalated value to the next solve scheduled onto it.
    static PIVOT_THRESHOLD: Cell<Option<f64>> = const { Cell::new(None) };
}

/// The value [`reset_pivot_threshold`] restores: [`STABILITY`], or
/// `ENOMOTO_PIVOT_THRESHOLD` when set (clamped to
/// `[PIVOT_THRESHOLD_MIN, PIVOT_THRESHOLD_MAX]`), which is how the A/B
/// behind the constant's own value is produced without a rebuild. Read
/// from the environment once per process, not once per solve: a solve that
/// re-read it would pay a `std::env::var` lookup inside the very loop this
/// section is trying to speed up.
fn pivot_threshold_base() -> f64 {
    static BASE: OnceLock<f64> = OnceLock::new();
    *BASE.get_or_init(|| {
        std::env::var("ENOMOTO_PIVOT_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .map(|v| v.clamp(PIVOT_THRESHOLD_MIN, PIVOT_THRESHOLD_MAX))
            .unwrap_or(STABILITY)
    })
}

/// The threshold-pivoting floor in force right now: a candidate pivot must
/// be at least this fraction of the largest magnitude left in its column
/// of the active submatrix. Read **once per factorization**
/// (`MarkowitzState::new`, `factorize_reusing_order`), never per
/// elimination step — a factorization that read it per step could see it
/// change underneath itself only if the simplex loop ran concurrently with
/// its own factorization, but reading it once also keeps the whole
/// factorization's pivot sequence a function of one scalar, which is what
/// makes a given solve reproducible.
pub fn pivot_threshold() -> f64 {
    PIVOT_THRESHOLD.with(|c| match c.get() {
        Some(v) => v,
        None => {
            let base = pivot_threshold_base();
            c.set(Some(base));
            base
        }
    })
}

/// Restores [`pivot_threshold_base`] — called by every solve entry point
/// before its first factorization, since the escalation below is
/// deliberately monotone *within* a solve and must not leak across solves
/// (see [`PIVOT_THRESHOLD`]'s own docs).
pub fn reset_pivot_threshold() {
    PIVOT_THRESHOLD.with(|c| c.set(Some(pivot_threshold_base())));
}

/// Raises the threshold one [`PIVOT_THRESHOLD_FACTOR`] step, capped at
/// [`PIVOT_THRESHOLD_MAX`]; returns whether it actually moved.
///
/// This is `docs/lu_comparison_enomoto_vs_highs.md` §2.4's "loosen/tighten
/// the stability floor when the problem is ill-conditioned", in HiGHS's
/// own direction: a solve that keeps *failing* numerically (Forrest-Tomlin
/// updates rejected, `x_B(M)`/`d` drifting away from the true basis,
/// pivots grossly inconsistent with the factorization) is one whose
/// factorizations are too permissive, so the floor goes **up**, buying
/// stability with fill-in. Lowering it on trouble would be the wrong sign:
/// it is exactly the marginal pivots a lower floor admits that produce the
/// eta chains these triggers are catching.
///
/// Monotone within a solve, like HiGHS's `info_.factor_pivot_threshold`:
/// nothing lowers it again short of [`reset_pivot_threshold`]. A ratchet
/// that also relaxed would make "how many troublesome iterations ago" part
/// of the pivot sequence, and the extra state buys nothing measurable —
/// the ladder is one step wide.
///
/// **Nothing calls this by default.** Wiring it to the numerical-failure
/// triggers cost +7.4% over NETLIB93; see
/// `extended_dual::PIVOT_ESCALATION_STEP` and
/// `analysis/pivot_threshold_colfixmax_20260922_154500.md` for the
/// measurement, and `ENOMOTO_PIVOT_ESCALATION_STEP` to re-enable it.
pub fn escalate_pivot_threshold() -> bool {
    let cur = pivot_threshold();
    let next = (cur * PIVOT_THRESHOLD_FACTOR).min(PIVOT_THRESHOLD_MAX);
    if next > cur {
        PIVOT_THRESHOLD.with(|c| c.set(Some(next)));
        PROF_PIVOT_ESCALATIONS.fetch_add(1, Ordering::Relaxed);
        true
    } else {
        false
    }
}

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
/// stale column-max rescans all get paid) to improve on a pivot
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
const PIVOT_SEARCH_LIMIT: usize = 256;

/// [`PIVOT_SEARCH_LIMIT`], overridable via `ENOMOTO_PIVOT_SEARCH_LIMIT` —
/// `0` restores the unbounded scan, which is how the A/B behind the
/// constant's own value is produced. Read once per `MarkowitzState::new`
/// (i.e. once per factorization), never per elimination step: the read is
/// `find_best_pivot`'s own caller-side cost otherwise, paid `m` times per
/// factorization, and an `std::env::var` lookup there would show up in the
/// very measurement this gate exists to make.
fn pivot_search_limit() -> usize {
    std::env::var("ENOMOTO_PIVOT_SEARCH_LIMIT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(PIVOT_SEARCH_LIMIT)
}

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
// shape changes that picture — and it did: the "under 0.2%" figure above
// holds only for the four small instances it was measured on. Re-measured
// across the whole set for [`PIVOT_SEARCH_LIMIT`], `dfl001` spent 2.96s of
// its 22.0s solve (13%) inside `find_best_pivot`, at an average scan width
// of 261 candidate columns per elimination step — the search *was* a real
// cost there, just not on problems small enough for the original sample.
// A triangularization pre-pass still isn't what that calls for (the bound
// in `PIVOT_SEARCH_LIMIT` addresses it directly, taking the same problem's
// scan to 0.22s), but the 0.2% claim should not be quoted as if it covered
// the large instances.
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
/// How many `find_best_pivot` calls actually returned early because of
/// [`PIVOT_SEARCH_LIMIT`] (as opposed to the score-0 exit, the
/// per-degree-level `merit_limit` exit, or a full scan) — the direct
/// measurement of how often the bound is exercised at all, without which
/// a flat benchmark result can't be told apart from a no-op.
pub(crate) static PROF_SEARCH_LIMIT_STEPS: AtomicUsize = AtomicUsize::new(0);
/// Total candidate columns examined across all `find_best_pivot` calls —
/// the quantity [`PIVOT_SEARCH_LIMIT`] bounds per call. Read against
/// [`PROF_TOTAL_STEPS`] it gives the average scan width per step, which is
/// what the bound is supposed to move.
pub(crate) static PROF_SEARCH_CANDIDATES: AtomicUsize = AtomicUsize::new(0);

/// How many times [`escalate_pivot_threshold`] actually moved the
/// threshold — without this, a flat benchmark on the §2.4 escalation
/// can't be told apart from one where the ladder never fired at all.
pub(crate) static PROF_PIVOT_ESCALATIONS: AtomicUsize = AtomicUsize::new(0);
/// Entries `find_best_pivot`'s lazy rescan walked to un-stale the columns
/// `find_best_pivot` actually read — the total work an incremental
/// `colFixMax` (`docs/lu_comparison_enomoto_vs_highs.md` §2.4) could have
/// removed, and the reason removing it lost: since §2.5's
/// [`PIVOT_SEARCH_LIMIT`] bounds a single search to 8 candidate columns,
/// this is already a small fraction of the per-entry bookkeeping such a
/// scheme costs in `eliminate` (measured in
/// `analysis/pivot_threshold_colfixmax_20260922_154500.md` §2). Counted
/// per rescan, not per touched column, so it stays off the elimination
/// loop's own path.
pub(crate) static PROF_COLMAX_RESCAN_ENTRIES: AtomicUsize = AtomicUsize::new(0);

/// BTRANs whose `L^{-T}` stage took the row-major scatter form, against
/// those that fell back to the column-major gather form — see
/// [`BTRAN_L_SCATTER_FRACTION`].
pub(crate) static PROF_BTRAN_L_SCATTER: AtomicUsize = AtomicUsize::new(0);
pub(crate) static PROF_BTRAN_L_GATHER: AtomicUsize = AtomicUsize::new(0);

/// Rows shorter than this are searched for a column linearly rather than
/// by binary search ([`KernelMatrix::row_get`]). Markowitz elimination is
/// specifically choosing pivots to keep the active rows short, so the
/// linear branch is the common one: a run this size fits in one or two
/// cache lines and scans branch-predictably, where `binary_search` pays a
/// mispredict per level for the same work.
const KERNEL_LINEAR_SCAN_MAX: usize = 16;

/// Flat, HiGHS-`HFactor`-style storage for the active submatrix that
/// Markowitz elimination works on — the replacement for the
/// `Vec<BTreeMap<usize, f64>>` (rows) + `Vec<BTreeSet<usize>>` (column
/// mirror) pair [`MarkowitzState`] used to hold directly, and the item
/// `docs/lu_comparison_enomoto_vs_highs.md` §3.1 flagged as the largest
/// remaining structural gap against HiGHS's own kernel (`mc_*`/`mr_*`
/// flat arrays with in-place insert/delete, against tree nodes scattered
/// across the heap and an `O(log d)` traversal per element touched).
///
/// Layout, mirroring HiGHS's `mc_start`/`mc_space`/`mc_count` +
/// `mr_start`/`mr_space`/`mr_count` pattern with the two axes swapped —
/// this crate's elimination is row-oriented (it scatters the pivot *row*
/// into every affected row), where HiGHS's is column-oriented, so the
/// *values* live row-major here and the index-only mirror is the column
/// one:
///
/// - `row_ent[row_start[i] .. row_start[i] + row_len[i]]` is row `i`'s
///   live `(column, value)` run, **sorted ascending by column**.
///   `row_cap[i]` is how much room that run has in place before it must
///   be relocated to the end of `row_ent`.
/// - `col_ent[col_start[j] .. col_start[j] + col_len[j]]` is column `j`'s
///   live row list — indices only (`u32`: two rows per cache line's worth
///   of what `usize` would cost, and the ascending-degree bucket scan in
///   [`MarkowitzState::find_best_pivot`] reads these runs end to end),
///   **sorted ascending by row**. The row side is the single source of
///   truth for values; this mirror only answers "which rows are live in
///   column `j`", exactly as `col_rows` did.
///
/// Both runs are kept *sorted*, rather than taking HiGHS's cheaper
/// swap-with-last unordered sets. That is a deliberate extra cost — a
/// `copy_within` over a contiguous, usually single-digit-length run,
/// still far cheaper than the tree traversal it replaces — and it buys
/// exact behavioural equivalence with the ordered containers it replaces:
/// [`MarkowitzState::find_best_pivot`] resolves Markowitz-score *and*
/// pivot-magnitude ties by first-encountered, `eliminate` emits its `L`
/// multipliers and refreshes touched columns in iteration order, and LP
/// basis matrices are full of exactly-tied `±1` coefficients. An
/// unordered mirror would therefore silently select different pivots on
/// real Netlib instances, changing the factorization, the iteration
/// counts, and hence what a before/after benchmark of *this* change is
/// actually measuring.
struct KernelMatrix {
    row_ent: Vec<(usize, f64)>,
    row_start: Vec<usize>,
    row_len: Vec<usize>,
    row_cap: Vec<usize>,
    col_ent: Vec<u32>,
    col_start: Vec<usize>,
    col_len: Vec<usize>,
    col_cap: Vec<usize>,
}

impl KernelMatrix {
    fn new(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Self {
        assert!(m <= u32::MAX as usize, "kernel row index must fit in u32");
        assert_eq!(rows_in.len(), m, "kernel input must be square");
        let total: usize = rows_in.iter().map(|r| r.len()).sum();
        // Capacity, not length: the reserve is here so the relocations
        // below (and `ensure_row_cap`'s own, later) stay `push`/`resize`
        // inside one allocation instead of repeatedly reallocating and
        // copying the whole buffer. Nothing is *initialized* beyond what
        // is actually written — see the zero-slack note at `row_cap`
        // below.
        let mut row_ent: Vec<(usize, f64)> = Vec::with_capacity(2 * total + 64);
        let mut row_start = Vec::with_capacity(m);
        let mut row_len = Vec::with_capacity(m);
        let mut row_cap = Vec::with_capacity(m);
        let mut buf: Vec<(usize, f64)> = Vec::new();

        for row in rows_in.iter() {
            buf.clear();
            buf.extend_from_slice(row);
            // Stable sort by column, then accumulate each duplicate run in
            // input order and drop exact zeros — bit-for-bit what the
            // `*entry(j).or_insert(0.0) += v` + `retain(|_, v| *v != 0.0)`
            // construction this replaces produced, summation order of
            // repeated coordinates included.
            buf.sort_by_key(|&(j, _)| j);
            let start = row_ent.len();
            let mut k = 0;
            while k < buf.len() {
                let j = buf[k].0;
                let mut acc = 0.0f64;
                while k < buf.len() && buf[k].0 == j {
                    acc += buf[k].1;
                    k += 1;
                }
                if acc != 0.0 {
                    row_ent.push((j, acc));
                }
            }
            // No up-front slack (`cap == len`): a row only ever needs to
            // grow when the merge in `eliminate` leaves it *net* longer,
            // and the pivot column's own entry always leaves at the same
            // time, so one fill-in still fits in place and only two or
            // more relocate. Pre-padding every row instead cost a
            // memset proportional to the padding on *every*
            // refactorization, including for the many rows that never
            // take fill at all — worst of all on a near-slack basis,
            // where `nnz ~= m` makes a flat few-entries-per-row pad
            // several times the size of the real data. `ensure_row_cap`
            // doubles from here, so a row that keeps taking fill still
            // relocates `O(log)` times, not once per insertion.
            let len = row_ent.len() - start;
            row_start.push(start);
            row_len.push(len);
            row_cap.push(len);
        }

        // Column mirror by counting sort. Filling it with `i` ascending is
        // what makes every column's run sorted without a sort.
        let mut col_len = vec![0usize; m];
        for i in 0..m {
            let (s, l) = (row_start[i], row_len[i]);
            for k in s..s + l {
                col_len[row_ent[k].0] += 1;
            }
        }
        let mut col_start = vec![0usize; m];
        let mut pos = 0usize;
        for j in 0..m {
            col_start[j] = pos;
            pos += col_len[j];
        }
        // Zero-slack and reserved, for the same reasons as the row side.
        let col_cap = col_len.clone();
        let mut col_ent: Vec<u32> = Vec::with_capacity(2 * total + 64);
        col_ent.resize(pos, 0);
        let mut fill = vec![0usize; m];
        for i in 0..m {
            let (s, l) = (row_start[i], row_len[i]);
            for k in s..s + l {
                let j = row_ent[k].0;
                col_ent[col_start[j] + fill[j]] = i as u32;
                fill[j] += 1;
            }
        }

        KernelMatrix { row_ent, row_start, row_len, row_cap, col_ent, col_start, col_len, col_cap }
    }

    #[inline]
    fn row(&self, i: usize) -> &[(usize, f64)] {
        let s = self.row_start[i];
        &self.row_ent[s..s + self.row_len[i]]
    }

    #[inline]
    fn col(&self, j: usize) -> &[u32] {
        let s = self.col_start[j];
        &self.col_ent[s..s + self.col_len[j]]
    }

    /// The value at `(i, j)`, or `None` if that coordinate is not live —
    /// the direct stand-in for `rows[i].get(&j)`.
    #[inline]
    fn row_get(&self, i: usize, j: usize) -> Option<f64> {
        let row = self.row(i);
        if row.len() <= KERNEL_LINEAR_SCAN_MAX {
            for &(c, v) in row {
                if c == j {
                    return Some(v);
                }
                if c > j {
                    return None;
                }
            }
            None
        } else {
            match row.binary_search_by(|e| e.0.cmp(&j)) {
                Ok(p) => Some(row[p].1),
                Err(_) => None,
            }
        }
    }

    /// Relocates row `i`'s run to the end of `row_ent` if it cannot hold
    /// `need` entries in place, doubling its capacity (so a row that
    /// keeps taking fill-in relocates `O(log)` times, not once per
    /// insertion). The vacated run is left as dead space rather than
    /// compacted: total dead space is bounded by the live total, and a
    /// factorization is a short-lived, single-pass affair.
    fn ensure_row_cap(&mut self, i: usize, need: usize) {
        if need <= self.row_cap[i] {
            return;
        }
        let new_cap = need.max(self.row_cap[i] * 2).max(4);
        let (old_start, len) = (self.row_start[i], self.row_len[i]);
        let start = self.row_ent.len();
        self.row_ent.resize(start + new_cap, (0, 0.0));
        self.row_ent.copy_within(old_start..old_start + len, start);
        self.row_start[i] = start;
        self.row_cap[i] = new_cap;
    }

    /// Overwrites row `i` with `ents` (which must already be sorted
    /// ascending by column) — `eliminate` rebuilds a whole affected row
    /// in one merge pass rather than poking at it entry by entry, so this
    /// bulk form is the only row mutation the kernel needs.
    fn set_row(&mut self, i: usize, ents: &[(usize, f64)]) {
        self.ensure_row_cap(i, ents.len());
        let s = self.row_start[i];
        self.row_ent[s..s + ents.len()].copy_from_slice(ents);
        self.row_len[i] = ents.len();
    }

    fn ensure_col_cap(&mut self, j: usize, need: usize) {
        if need <= self.col_cap[j] {
            return;
        }
        let new_cap = need.max(self.col_cap[j] * 2).max(4);
        let (old_start, len) = (self.col_start[j], self.col_len[j]);
        let start = self.col_ent.len();
        self.col_ent.resize(start + new_cap, 0);
        self.col_ent.copy_within(old_start..old_start + len, start);
        self.col_start[j] = start;
        self.col_cap[j] = new_cap;
    }

    /// Adds row `i` to column `j`'s live list, keeping it sorted.
    /// `eliminate` walks its affected rows in ascending order, so the
    /// insertion point is typically at or near the run's tail and the
    /// shift is short.
    fn col_insert(&mut self, j: usize, i: usize) {
        let len = self.col_len[j];
        let pos = {
            let s = self.col_start[j];
            self.col_ent[s..s + len].partition_point(|&r| (r as usize) < i)
        };
        self.ensure_col_cap(j, len + 1);
        let s = self.col_start[j];
        self.col_ent.copy_within(s + pos..s + len, s + pos + 1);
        self.col_ent[s + pos] = i as u32;
        self.col_len[j] = len + 1;
    }

    /// Removes row `i` from column `j`'s live list; a no-op when it isn't
    /// there, matching `BTreeSet::remove`'s own tolerance (callers rely on
    /// it: a column already retired by `col_clear` still gets removal
    /// calls for the pivot row).
    fn col_remove(&mut self, j: usize, i: usize) {
        let (s, len) = (self.col_start[j], self.col_len[j]);
        let run = &self.col_ent[s..s + len];
        let pos = run.partition_point(|&r| (r as usize) < i);
        if pos >= len || self.col_ent[s + pos] as usize != i {
            return;
        }
        self.col_ent.copy_within(s + pos + 1..s + len, s + pos);
        self.col_len[j] = len - 1;
    }

    #[inline]
    fn col_clear(&mut self, j: usize) {
        self.col_len[j] = 0;
    }
}

/// Per-elimination-step scratch, owned by [`MarkowitzState`] and lent out
/// via `mem::take` for the duration of [`MarkowitzState::eliminate`], so
/// that a factorization's `m` elimination steps reuse one set of buffers
/// instead of allocating a fresh `Vec`/`BTreeSet` per step.
#[derive(Default)]
struct ElimScratch {
    /// Rows of the pivot column that still need eliminating (a copy: the
    /// column's own live list is mutated while they are processed).
    affected: Vec<usize>,
    /// The merge output for the affected row currently being rewritten.
    merged: Vec<(usize, f64)>,
    /// Columns gaining / losing the current affected row, applied to the
    /// column mirror once the merge has released its borrow on the row.
    col_add: Vec<usize>,
    col_del: Vec<usize>,
    /// `L`'s multipliers for this step, in affected-row order.
    l_out: Vec<(usize, f64)>,
    /// Columns of the retiring pivot row, to drop it from.
    pi_cols: Vec<usize>,
    /// Columns whose degree or values this step changed, deduplicated via
    /// `mark`/`epoch` stamping (an `O(1)` membership test against the
    /// `BTreeSet<usize>` this replaces) and sorted before use — see
    /// `eliminate`'s own note on why the *order* matters.
    touched: Vec<usize>,
    mark: Vec<u32>,
    epoch: u32,
}

impl ElimScratch {
    fn new(m: usize) -> Self {
        ElimScratch { mark: vec![0; m], epoch: 0, ..Default::default() }
    }

    /// Starts a step: advances the stamp epoch, clearing the stamps
    /// outright on the one call in ~4 billion that wraps back to `0`
    /// (where a never-stamped entry's own `0` would read as "already
    /// touched"). Same technique, and same wrap-around caveat, as
    /// [`GpScratch::bump_epoch`].
    fn begin(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.mark.iter_mut().for_each(|e| *e = 0);
            self.epoch = 1;
        }
        self.touched.clear();
        self.l_out.clear();
    }

    #[inline]
    fn touch(&mut self, j: usize) {
        if self.mark[j] != self.epoch {
            self.mark[j] = self.epoch;
            self.touched.push(j);
        }
    }
}

/// Manages the active submatrix plus row/column degrees (via bucket
/// arrays) during Markowitz elimination — see [`KernelMatrix`] for the
/// flat row-major-values + column-major-indices storage itself, and the
/// module docs for why a column-major mirror alongside the row-major
/// matrix is what actually keeps this sub-`O(m)` per step, and why the
/// elimination and degree bookkeeping must happen in one place rather
/// than two.
struct MarkowitzState {
    #[allow(dead_code)]
    m: usize,

    /// Active submatrix: row-major values plus the column-major row-index
    /// mirror, both flat (see [`KernelMatrix`]).
    mat: KernelMatrix,

    // Current degrees (active nonzero counts), mirrored by bucket
    // placement below.
    col_degree: Vec<usize>,
    row_degree: Vec<usize>,

    // Bucket arrays: bucket[d] = indices currently at degree exactly d.
    // Sized m + 1 (a degree can be at most the number of active rows/cols).
    // Columns only: `find_best_pivot` walks columns by degree and reads a
    // row's degree straight off `row_degree`, so a row-side bucket
    // structure (kept here earlier, but never read) is not maintained.
    col_buckets: Vec<VecDeque<usize>>,

    // col_bucket_pos[j] = j's index within col_buckets[col_degree[j]], or
    // None if j has been used already (removed from every bucket).
    col_bucket_pos: Vec<Option<usize>>,

    col_used: Vec<bool>,
    row_used: Vec<bool>,

    // Column max absolute values, over that column's own active rows only
    // (via the column mirror) — the threshold-pivoting stability
    // reference.
    col_max_abs: Vec<f64>,

    // `col_max_abs_dirty[j]`: `col_max_abs[j]` is stale — some entry of
    // column `j` shrank or left, so the stored value is an upper bound
    // rather than the true max. Recomputing is deferred to
    // `find_best_pivot`, and only once it is actually about to read it. Most columns `eliminate` dirties this way get
    // dirtied again by a later elimination step before `find_best_pivot`
    // ever visits them (a column's bucket position, which *is* updated
    // eagerly by `update_col_degree`, is what determines when that
    // happens), so eagerly recomputing every dirtied column's max was
    // mostly wasted work — up to 75% of it, measured on Netlib `greenbea`.
    //
    // Marking, rather than maintaining, is also what
    // `docs/lu_comparison_enomoto_vs_highs.md` §2.4's incremental
    // `colFixMax` was measured against and beat. That variant kept
    // `col_max_abs[j]` exact through `eliminate` — raise it on a fill-in
    // or a growing value, mark stale only when the entry that *was* the
    // max shrank or left — which dropped the stale fraction to 0.1-6% of
    // touched columns and left every pivot choice bit-identical. It still
    // lost, by 2.8% over NETLIB93: since §2.5's [`PIVOT_SEARCH_LIMIT`]
    // bounds one search to 8 candidate columns, the rescans it removed
    // were already small (`pilot87`: 1.9M entries) against the per-entry
    // bookkeeping it added in the merge loop below (61.5M updates, each a
    // scattered read-modify-write into an `m`-sized array). See
    // `analysis/pivot_threshold_colfixmax_20260922_154500.md` §2.
    col_max_abs_dirty: Vec<bool>,

    /// `initially_dense[j]` iff column `j`'s degree *before any
    /// elimination* exceeded `DENSE_COL_FRACTION * m` — fixed at
    /// construction time and never updated, deliberately: a truly dense
    /// column's *current* degree keeps shrinking as unrelated rows get
    /// eliminated as pivots for *other*, sparser columns (each such row
    /// leaving the basis removes it from every column's live list,
    /// including this one's) — dropping into a low bucket only because
    /// its rows happened to get cannibalized elsewhere, not because it
    /// stopped being structurally dense. Thresholding on the live,
    /// shrinking degree would let `find_best_pivot`'s ordinary ascending-
    /// bucket scan pick such a column early anyway, right when it looks
    /// artificially sparse — exactly the case this field exists to still
    /// catch. See `find_best_pivot`'s own docs for what this avoids: a
    /// pivot on a column with `d` remaining active rows scatters the
    /// entire pivot row's pattern into all `d` of them in one step
    /// (`eliminate`'s `affected` list), so pivoting on a column that is
    /// dense *by original structure* — even at a reduced current degree —
    /// is still the single most expensive kind of step Markowitz pivoting
    /// can take.
    initially_dense: Vec<bool>,

    /// Reused per-step buffers; see [`ElimScratch`].
    scratch: ElimScratch,

    /// [`pivot_search_limit`]'s value, resolved once here rather than per
    /// `find_best_pivot` call — `0` means "unbounded", the pre-§2.5
    /// behaviour.
    search_limit: usize,

    /// [`pivot_threshold`]'s value, resolved once per factorization here —
    /// see that function's own docs for why this factorization's whole
    /// pivot sequence is deliberately a function of one scalar captured at
    /// its start, rather than of a value the simplex loop could raise
    /// part-way through.
    threshold: f64,

    /// Plain (non-atomic) accumulator for [`PROF_COLMAX_RESCAN_ENTRIES`],
    /// flushed once in [`Drop`] — an `AtomicUsize::fetch_add` per rescan
    /// would be a locked read-modify-write on the factorization's own
    /// path, and would make two threads factorizing at once contend on
    /// one cache line.
    prof_colmax_rescan_entries: usize,
}

impl Drop for MarkowitzState {
    fn drop(&mut self) {
        PROF_COLMAX_RESCAN_ENTRIES.fetch_add(self.prof_colmax_rescan_entries, Ordering::Relaxed);
    }
}

impl MarkowitzState {
    fn new(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Self {
        let mat = KernelMatrix::new(m, rows_in);

        let mut col_max_abs = vec![0.0f64; m];
        let mut row_degree = vec![0usize; m];
        for i in 0..m {
            let row = mat.row(i);
            row_degree[i] = row.len();
            for &(j, v) in row {
                col_max_abs[j] = col_max_abs[j].max(v.abs());
            }
        }
        let col_degree: Vec<usize> = (0..m).map(|j| mat.col(j).len()).collect();

        let mut col_buckets = vec![VecDeque::new(); m + 1];
        let mut col_bucket_pos = vec![None; m];

        for j in 0..m {
            let deg = col_degree[j];
            col_bucket_pos[j] = Some(col_buckets[deg].len());
            col_buckets[deg].push_back(j);
        }

        let dense_threshold = DENSE_COL_FRACTION * m as f64;
        let initially_dense: Vec<bool> = col_degree.iter().map(|&d| d as f64 > dense_threshold).collect();

        MarkowitzState {
            m,
            mat,
            col_degree,
            row_degree,
            col_buckets,
            col_bucket_pos,
            col_used: vec![false; m],
            row_used: vec![false; m],
            col_max_abs,
            col_max_abs_dirty: vec![false; m],
            initially_dense,
            scratch: ElimScratch::new(m),
            search_limit: pivot_search_limit(),
            threshold: pivot_threshold(),
            prof_colmax_rescan_entries: 0,
        }
    }

    /// Row `i`'s live `(column, value)` entries, sorted ascending by
    /// column — the stand-in for iterating `rows[i]`, with the same order.
    #[inline]
    fn row(&self, i: usize) -> &[(usize, f64)] {
        self.mat.row(i)
    }

    /// The value at `(i, j)`, or `None` — the stand-in for
    /// `rows[i].get(&j)`.
    #[inline]
    fn value_at(&self, i: usize, j: usize) -> Option<f64> {
        self.mat.row_get(i, j)
    }

    /// The `(row, multiplier)` pairs the last [`Self::eliminate`] call
    /// produced for `L`. Held in scratch rather than returned by value so
    /// that a factorization's `m` steps share one buffer.
    #[inline]
    fn l_out(&self) -> &[(usize, f64)] {
        &self.scratch.l_out
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
        if !self.row_used[i] {
            self.row_degree[i] = new_deg;
        }
    }

    /// Updates `j`'s degree/bucket placement from its current
    /// column-mirror membership — O(that column's own active degree),
    /// never O(m).
    ///
    /// Marks `col_max_abs[j]` stale rather than recomputing it here; see
    /// `find_best_pivot`'s lazy rescan and `col_max_abs_dirty`'s own docs for
    /// why, including why the incremental alternative
    /// (`docs/lu_comparison_enomoto_vs_highs.md` §2.4's `colFixMax`) was
    /// measured and rejected.
    fn refresh_column(&mut self, j: usize) {
        if self.col_used[j] {
            return;
        }
        let new_deg = self.mat.col(j).len();
        self.update_col_degree(j, new_deg);
        self.col_max_abs_dirty[j] = true;
    }

    /// `max |a_ij|` over column `j`'s live entries, read straight off the
    /// column mirror — `find_best_pivot`'s lazy un-staling of a dirty
    /// `col_max_abs[j]`, taken only once that column has a candidate entry
    /// that actually needs the stability threshold. Through `&self` so it
    /// can run while the search holds a borrow of column `j`.
    fn col_max_abs_scan(&self, j: usize) -> f64 {
        let mut mx = 0.0f64;
        for &r in self.mat.col(j) {
            if let Some(v) = self.mat.row_get(r as usize, j) {
                mx = f64::max(mx, v.abs());
            }
        }
        mx
    }

    /// Find best pivot: among still-active columns in ascending-degree
    /// order, only that column's actual active rows (via the column
    /// mirror, not every row at that row-degree) are examined — this is
    /// the other half (alongside `eliminate`'s use of the same mirror) of
    /// what keeps the search from degrading into a full active-submatrix
    /// scan. The per-degree-level early exit is a standard practical
    /// relaxation (as in production Markowitz implementations): it does
    /// not guarantee the globally minimal Markowitz count, only that no
    /// further search will find something clearly better — finding the
    /// exact minimum every step is itself more expensive than the fill-in
    /// it would save.
    ///
    /// `skip_dense`: when true, every `initially_dense` column is skipped
    /// outright, regardless of its current (possibly much lower, per that
    /// field's own docs) degree or Markowitz score — `factorize`'s caller
    /// tries this first and only falls back to a second, unrestricted call
    /// if it finds nothing, so a truly-required dense pivot (or a genuinely
    /// singular matrix) is still handled correctly, just not preferred.
    ///
    /// The scan is additionally bounded by [`PIVOT_SEARCH_LIMIT`] candidate
    /// columns, which — unlike the per-degree-level exit above — can fire
    /// part-way *through* a bucket, and so is what actually bounds a single
    /// call's cost when one degree level holds hundreds of columns. It is
    /// honoured only once a pivot has been found, so it never turns a
    /// `Some` into a `None`.
    fn find_best_pivot(&mut self, skip_dense: bool) -> Option<(usize, usize)> {
        let __prof_t0 = std::time::Instant::now();
        PROF_TOTAL_STEPS.fetch_add(1, Ordering::Relaxed);
        let mut best: Option<(usize, usize)> = None;
        let mut best_score = usize::MAX;
        let mut best_pivot_abs = 0.0f64;
        // Candidate columns examined so far by *this* call — the quantity
        // `search_limit` bounds (HiGHS's `searchCount`).
        let mut searched = 0usize;

        for deg_col in 1..self.col_buckets.len() {
            // Indexed rather than iterated by reference: nothing in this
            // loop body mutates `col_buckets[deg_col]` itself (bucket
            // membership only ever changes via `update_col_degree`/
            // `remove_from_bucket_col`, called elsewhere, never from
            // inside `find_best_pivot`), so its length and contents are
            // fixed for this `deg_col`'s scan.
            for idx in 0..self.col_buckets[deg_col].len() {
                let j = self.col_buckets[deg_col][idx];
                if self.col_used[j] || (skip_dense && self.initially_dense[j]) {
                    continue;
                }
                // HiGHS's `mc_min_pivot[j] = max_value * pivot_threshold`
                // (§2.4), a per-*column* quantity — computed lazily, on the
                // first entry that survives the Markowitz-score filter
                // below, rather than before the column's scan: a stale
                // `col_max_abs[j]` costs a full column rescan, and on a
                // column none of whose entries can beat `best_score` it is
                // never read at all (the common case once a good pivot is
                // in hand — `pilot87` rescanned 14.2M entries per solve
                // up front). The rescan reads only the (unchanging, during
                // this search) matrix, so the value — and every
                // comparison against it — is exactly the eager one.
                let col_dirty = self.col_max_abs_dirty[j];
                let mut min_pivot: Option<f64> = None;
                let mut rescanned: Option<f64> = None;
                let col_deg = self.col_degree[j];
                searched += 1;
                for &r in self.mat.col(j) {
                    let i = r as usize;
                    if self.row_used[i] {
                        continue;
                    }
                    // Markowitz score only needs row/col degree, both already
                    // known without touching the row's own run — skip the
                    // value lookup below for candidates that can't possibly
                    // beat `best_score` (this is the vast majority on a
                    // matrix with heavy fill-in after many FT updates).
                    let score = (self.row_degree[i] - 1) * (col_deg - 1);
                    if score > best_score {
                        continue;
                    }
                    let Some(v) = self.mat.row_get(i, j) else { continue };
                    if v == 0.0 {
                        continue;
                    }
                    let mp = *min_pivot.get_or_insert_with(|| {
                        let mx = if col_dirty {
                            *rescanned.insert(self.col_max_abs_scan(j))
                        } else {
                            self.col_max_abs[j]
                        };
                        self.threshold * mx
                    });
                    if v.abs() < mp {
                        continue;
                    }
                    if score < best_score || (score == best_score && v.abs() > best_pivot_abs) {
                        best_score = score;
                        best = Some((i, j));
                        best_pivot_abs = v.abs();
                    }
                }
                if let Some(mx) = rescanned {
                    self.prof_colmax_rescan_entries += self.mat.col(j).len();
                    self.col_max_abs[j] = mx;
                    self.col_max_abs_dirty[j] = false;
                }
                if best_score == 0 {
                    PROF_TRIVIAL_STEPS.fetch_add(1, Ordering::Relaxed);
                    PROF_SEARCH_CANDIDATES.fetch_add(searched, Ordering::Relaxed);
                    PROF_BUCKET_SCAN_NS.fetch_add(__prof_t0.elapsed().as_nanos() as usize, Ordering::Relaxed);
                    return best;
                }
                // Checked after this column's own scan (never before it),
                // so the limit bounds how many columns are examined rather
                // than cutting one short mid-way: the `best` a truncated
                // column produced would otherwise depend on `col_rows`'
                // iteration order in a way the unbounded scan's doesn't.
                if self.search_limit != 0 && searched >= self.search_limit && best.is_some() {
                    PROF_SEARCH_LIMIT_STEPS.fetch_add(1, Ordering::Relaxed);
                    PROF_SEARCH_CANDIDATES.fetch_add(searched, Ordering::Relaxed);
                    PROF_BUCKET_SCAN_NS.fetch_add(__prof_t0.elapsed().as_nanos() as usize, Ordering::Relaxed);
                    return best;
                }
            }
            if best.is_some() && best_score <= deg_col * deg_col {
                PROF_SEARCH_CANDIDATES.fetch_add(searched, Ordering::Relaxed);
                PROF_BUCKET_SCAN_NS.fetch_add(__prof_t0.elapsed().as_nanos() as usize, Ordering::Relaxed);
                return best;
            }
        }

        PROF_SEARCH_CANDIDATES.fetch_add(searched, Ordering::Relaxed);
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
    /// retires row `pi` from every other column's live list (not just
    /// column `pj`'s), so no column's degree can drift by continuing to
    /// count a row that is no longer active. The resulting `(row,
    /// multiplier)` pairs for `factorize`'s own `L` bookkeeping are left
    /// in [`Self::l_out`].
    ///
    /// Each affected row is rewritten by a **single sorted merge** of its
    /// own run against `pivot_row_snapshot` (both ascending by column),
    /// rather than by one keyed lookup per pivot-row entry: with the
    /// `BTreeMap` rows this replaced, this inner loop — the hottest in
    /// the whole factorization, run once per `(affected row, pivot-row
    /// entry)` pair, every elimination step — cost `O(d_p log d_i)` tree
    /// descents into scattered heap nodes; the merge costs `O(d_i + d_p)`
    /// over two contiguous, sequentially-read runs and one sequentially-
    /// written one. That is `docs/lu_comparison_enomoto_vs_highs.md`
    /// §3.1's point (HiGHS's `mc_*`/`mr_*` flat arrays against this
    /// crate's tree nodes) applied to the one loop where it matters most.
    fn eliminate(&mut self, pi: usize, pj: usize, pivot_val: f64, pivot_row_snapshot: &[(usize, f64)]) {
        let mut sc = std::mem::take(&mut self.scratch);
        sc.begin();

        sc.affected.clear();
        sc.affected.extend(self.mat.col(pj).iter().map(|&r| r as usize).filter(|&i| i != pi));

        for ai in 0..sc.affected.len() {
            let i = sc.affected[ai];
            let Some(aij) = self.mat.row_get(i, pj) else { continue };
            if aij == 0.0 {
                continue;
            }
            let mult = aij / pivot_val;
            sc.l_out.push((i, mult));

            sc.merged.clear();
            sc.col_add.clear();
            sc.col_del.clear();
            {
                let row = self.mat.row(i);
                let (mut a, mut b) = (0usize, 0usize);
                while a < row.len() && b < pivot_row_snapshot.len() {
                    let (ja, va) = row[a];
                    let (jb, vb) = pivot_row_snapshot[b];
                    if ja < jb {
                        // Only in this row — including every column
                        // already used as a pivot, which the snapshot
                        // filters out and which must survive untouched.
                        sc.merged.push((ja, va));
                        a += 1;
                    } else if jb < ja {
                        // Fill-in.
                        if jb != pj {
                            let new_val = -mult * vb;
                            if new_val != 0.0 {
                                sc.merged.push((jb, new_val));
                                sc.col_add.push(jb);
                                sc.touch(jb);
                            }
                        }
                        b += 1;
                    } else {
                        if ja == pj {
                            // The pivot column's own entry leaves this
                            // row; its mirror is retired wholesale by the
                            // `col_clear(pj)` below, so no `col_del` and
                            // no `touch` here — exactly what the
                            // `rows[i].remove(&pj)` this replaces did.
                        } else {
                            let new_val = va - mult * vb;
                            if new_val == 0.0 {
                                sc.col_del.push(ja);
                            } else {
                                sc.merged.push((ja, new_val));
                            }
                            sc.touch(ja);
                        }
                        a += 1;
                        b += 1;
                    }
                }
                while a < row.len() {
                    sc.merged.push(row[a]);
                    a += 1;
                }
                while b < pivot_row_snapshot.len() {
                    let (jb, vb) = pivot_row_snapshot[b];
                    if jb != pj {
                        let new_val = -mult * vb;
                        if new_val != 0.0 {
                            sc.merged.push((jb, new_val));
                            sc.col_add.push(jb);
                            sc.touch(jb);
                        }
                    }
                    b += 1;
                }
            }

            self.mat.set_row(i, &sc.merged);
            for k in 0..sc.col_del.len() {
                self.mat.col_remove(sc.col_del[k], i);
            }
            for k in 0..sc.col_add.len() {
                self.mat.col_insert(sc.col_add[k], i);
            }
            let new_deg_i = sc.merged.len();
            self.update_row_degree(i, new_deg_i);
        }
        self.mat.col_clear(pj);

        // Row pi is retiring as the new pivot row; drop it from every
        // other column it still touches so those columns' degrees don't
        // keep counting an inactive row.
        sc.pi_cols.clear();
        sc.pi_cols.extend(self.mat.row(pi).iter().map(|&(j, _)| j).filter(|&j| j != pj));
        for k in 0..sc.pi_cols.len() {
            let j = sc.pi_cols[k];
            self.mat.col_remove(j, pi);
            sc.touch(j);
        }

        // Sorted, not merely deduplicated: `refresh_column` appends to a
        // degree bucket, and `find_best_pivot` scans those buckets in
        // stored order and breaks exact ties by first-encountered, so the
        // refresh order is observable in which pivot gets chosen. The
        // `BTreeSet` this replaced delivered ascending order; sorting the
        // stamped `Vec` reproduces it for strictly less work.
        sc.touched.sort_unstable();
        for k in 0..sc.touched.len() {
            self.refresh_column(sc.touched[k]);
        }

        self.scratch = sc;
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
/// `visited` marks the steps already known to be in the *current* call's
/// reach set, via [`EpochMarks`] — bumping an epoch each call instead of
/// clearing an array is what makes marking/checking `O(1)` without an
/// `O(m)` reset per call. (It was a hand-rolled `Vec<u32>` plus a bare
/// `epoch += 1` until that trick was consolidated into `crate::sparse`;
/// the bare increment had no wraparound guard, so after 2^32 calls a stale
/// stamp would have read as a live mark and silently truncated a reach set
/// — i.e. produced a wrong FTRAN. [`EpochMarks::begin`] handles it.)
/// `stack` is the DFS's own
/// (iterative, not recursive — this crate's basis matrices can have `m`
/// in the low thousands, deep enough that a recursive DFS risks a real
/// stack overflow on a long dependency chain) working stack. `reach` is
/// this call's own collected, then sorted, reach set.
pub struct GpScratch {
    visited: EpochMarks,
    stack: Vec<usize>,
    seeds: Vec<usize>,
    reach: Vec<usize>,
}

impl GpScratch {
    pub fn new(m: usize) -> Self {
        GpScratch { visited: EpochMarks::new(m), stack: Vec::new(), seeds: Vec::new(), reach: Vec::new() }
    }
}

#[derive(Clone)]
pub struct LuFactors {
    pub m: usize,
    /// `l_col[s]`: `(row_step, multiplier)` pairs — the sub-diagonal
    /// entries of `L`'s column `s`.
    ///
    /// Stored as a [`crate::sparse::CscMat`] — one flat `(index, value)`
    /// buffer plus offsets, the same layout `simplex.rs`'s `StdForm` uses
    /// for the frozen coefficient matrix, on the same reasoning: `L` never
    /// changes once a refactorization builds it, and it is then read on
    /// every FTRAN/BTRAN's `L`-stage for the rest of that basis's life.
    ///
    /// **This is the second attempt, and the first one that measured as a
    /// win.** The first flattened a finished `Vec<Vec<(usize, f64)>>` into
    /// the compressed form as a post-pass, and a controlled A/B showed a
    /// consistent small regression on every instance that moved at all
    /// (`scsd8` +2.1%, `25fv47` +1.8%, `stocfor2` +4.0%, `fit1p` +3.2%,
    /// `degen3`/`pilotnov` flat, nothing faster). Its own post-mortem
    /// identified the reason and named the fix: the post-pass *keeps*
    /// building the `m` small per-column `Vec`s it was meant to remove and
    /// then adds an `O(nnz)` copy on top, so it paid the compressed form's
    /// cost — two offset reads per column access, against `Vec<Vec>`'s
    /// single pointer hop to an already-known `(ptr, len)` — while buying
    /// none of its benefit.
    ///
    /// [`crate::sparse::CscBuilder`] is that fix. Every one of this file's
    /// factorizations already emits `L`'s columns in ascending step order
    /// (left-looking elimination produces column `s` complete at step `s`),
    /// so the flat buffer can be appended to directly, with the column
    /// boundary recorded wherever the buffer has reached: no counting pass,
    /// no per-column `Vec`, and no copy. What is left is a strict
    /// improvement at build time (two allocations for the whole of `L`
    /// instead of `m + 1`) plus contiguous entries for `l_solve_into`'s own
    /// sequential `for s in 0..m` sweep to prefetch through.
    pub l_col: crate::sparse::CscMat,
    /// `u_row[s]`: `(col_step, value)` pairs, `col_step >= s` (including
    /// the diagonal at `col_step == s`) — the entries of `U`'s row `s`.
    /// Unlike `l_col`, this is read exactly once per refactorization (by
    /// `FtLu::new`, to seed `u_seq`) and never again, so it stays a plain
    /// `Vec<Vec<...>>` — flattening it would cost the same construction
    /// work for no repeated-read benefit.
    pub u_row: Vec<Vec<(usize, f64)>>,
    /// Row-major mirror of [`Self::l_col`] in the same step space:
    /// `l_row.row(r)` lists `(s, multiplier)` for every entry `(r,
    /// multiplier)` of `l_col[s]` — i.e. the nonzeros of `L`'s *row* `r`,
    /// all of which sit at `s < r`. This is HiGHS's own `lr_start/
    /// lr_index/lr_value` (`HFactor.h`, built by `buildFinish()` right
    /// beside the column-major `l_start/l_index/l_value`), and it exists
    /// for exactly the reason HiGHS builds it: `L^{-T}` (BTRAN's tail) is
    /// a *gather* when read through the column-major `l_col` — step `s`
    /// reads one `w[row_step]` per `l_col[s]` entry, so no single value's
    /// zero-ness makes the step skippable (Hall & McKinnon 2000 §4.4's own
    /// observation, which `l_transpose_solve_into`'s pre-`l_row` form was
    /// stuck with) — but the very same triangular solve becomes a
    /// *scatter* when read through this mirror: step `s` multiplies the
    /// single value `w[s]` into every `l_row.row(s)` entry, so `w[s] ==
    /// 0.0` makes the whole step a provable no-op, exactly the skip
    /// `l_solve_into`/`u_solve_into` already have in the forward
    /// direction. See [`LuFactors::l_transpose_solve_into`]'s own docs for
    /// the measurement.
    ///
    /// Flat ([`CsrMat`]: two allocations, offsets + entries), like
    /// `l_col` itself — built directly from it by [`CscMat::to_csr`]'s
    /// counting sort (one pass to size each row's slice, one to fill it),
    /// with no intermediate `Vec<Vec<...>>` on either side.
    pub l_row: CsrMat,
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

    let mut l_build = CscBuilder::new(m);
    for step in 0..m {
        for row_step in (step + 1)..m {
            let v = l[(row_step, step)];
            if v != 0.0 {
                l_build.push(row_step, v);
            }
        }
        l_build.end_column();
    }
    let l_col = l_build.build();
    let mut u_row: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for step in 0..m {
        for col_step in step..m {
            let v = u[(step, col_step)];
            if v != 0.0 {
                u_row[step].push((col_step, v));
            }
        }
    }

    let l_row = build_l_row(&l_col, m);
    Some(LuFactors { m, l_col, l_row, u_row, row_perm, col_perm, col_perm_inv, row_perm_inv })
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
    // `L` is the identity: `m` columns, every one empty.
    let l_col = CscMat::empty(m, m);
    let l_row = build_l_row(&l_col, m);
    Some(LuFactors {
        m,
        l_col,
        l_row,
        u_row,
        row_perm: identity.clone(),
        col_perm: identity.clone(),
        col_perm_inv: identity.clone(),
        row_perm_inv: identity,
    })
}

// ---------------------------------------------------------------------
// Pivot-order reuse ("rebuild", HiGHS `HFactorRefactor.cpp`)
// ---------------------------------------------------------------------

/// How many times [`factorize_reusing_order`] was attempted, how many of
/// those attempts produced a usable factorization, and how much wall time
/// the attempts (accepted *and* rejected) cost. Read back by the
/// `ENOMOTO_PROF_PHASES_EXT` diagnostic: a rejected attempt is pure
/// overhead paid on top of the full Markowitz factorization that follows,
/// so the accepted/attempted ratio is what decides whether the reuse pays
/// for itself.
pub(crate) static PROF_REBUILD_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);
pub(crate) static PROF_REBUILD_ACCEPTED: AtomicUsize = AtomicUsize::new(0);
pub(crate) static PROF_REBUILD_NS: AtomicUsize = AtomicUsize::new(0);
/// Why rejected attempts were rejected: the remaining submatrix had no
/// numerically usable entry left in the column being processed (a
/// genuinely singular or numerically spent basis), or the factors grew
/// past [`REBUILD_FILL_LIMIT`].
pub(crate) static PROF_REBUILD_FAIL_SINGULAR: AtomicUsize = AtomicUsize::new(0);
pub(crate) static PROF_REBUILD_FAIL_FILL: AtomicUsize = AtomicUsize::new(0);
/// Steps whose recorded pivot *row* was unusable and had to be re-chosen
/// (see [`factorize_reusing_order`]'s own docs) — bounded below by the
/// number of basis columns the Forrest-Tomlin updates replaced since the
/// order was recorded, and the reason the row order cannot simply be
/// replayed the way the column order can.
pub(crate) static PROF_REBUILD_ROW_REPICKS: AtomicUsize = AtomicUsize::new(0);
/// Refactorizations the backoff in [`factorize_reusing`] decided not to
/// even attempt a reuse for, after a rejection.
pub(crate) static PROF_REBUILD_BACKOFF_SKIPS: AtomicUsize = AtomicUsize::new(0);

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
const REBUILD_FILL_LIMIT: f64 = 1.25;

/// [`REBUILD_FILL_LIMIT`], overridable at run time via
/// `ENOMOTO_REUSE_FILL_LIMIT` (a bare float) so the one number can be
/// re-tuned against the Netlib set without a rebuild — and, more to the
/// point, flipped *within one process* for an A/B (see
/// [`reuse_pivot_order_enabled`]'s own docs on why cross-process timing
/// comparisons are not usable here). Read once per refactorization.
fn reuse_fill_limit() -> f64 {
    std::env::var("ENOMOTO_REUSE_FILL_LIMIT").ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(REBUILD_FILL_LIMIT)
}

/// Accepted reuses' own fill against the fresh Markowitz factorizations'
/// (`L`+`U` nonzeros, summed, with the count of each) — the diagnostic
/// behind "is a reused order producing factors every later FTRAN/BTRAN
/// then pays for".
pub(crate) static PROF_REBUILD_ACCEPTED_NNZ: AtomicUsize = AtomicUsize::new(0);
pub(crate) static PROF_FULL_NNZ: AtomicUsize = AtomicUsize::new(0);
pub(crate) static PROF_FULL_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Absolute floor on a pivot's magnitude: below this the column has
/// nothing usable left in the remaining submatrix at all, and the whole
/// attempt is abandoned rather than dividing by (almost) zero. Far below
/// `simplex.rs`'s own `FT_MIN_PIVOT` deliberately — this is a "there is no
/// pivot here" test, not a quality test, which the [`STABILITY`] check
/// next to it already is.
const REBUILD_MIN_PIVOT: f64 = 1e-12;

/// Whether pivot-order reuse is enabled (default: yes), toggled by
/// `ENOMOTO_REUSE_PIVOT_ORDER=0`. Exists so an A/B of this feature can
/// flip it *within one process* — per
/// `analysis/ftran_density_gate_20260922_062832.md` §4.1, this box's
/// per-problem run-to-run spread across separate processes reaches 4x,
/// far wider than the effect being measured. Read once per
/// refactorization (a handful of times per solve), never per iteration.
fn reuse_pivot_order_enabled() -> bool {
    !matches!(std::env::var("ENOMOTO_REUSE_PIVOT_ORDER").as_deref(), Ok("0") | Ok("false"))
}

/// Refactorizes `rows_in` **reusing `prev`'s pivot order** when possible,
/// falling back to a full Markowitz [`factorize`] otherwise.
///
/// This is this crate's counterpart to HiGHS's `HFactor::build()` trying
/// `rebuild()` (`util/HFactorRefactor.cpp`) before `buildSimple()` +
/// `buildKernel()`, named as gap §2.2 in
/// `docs/lu_comparison_enomoto_vs_highs.md`: a mid-solve refactorization
/// factorizes a basis that differs from the last factorized one only by
/// the columns the Forrest-Tomlin updates since then replaced, so the
/// order Markowitz chose last time is usually still a good order — and
/// *applying a known order* costs only the elimination's own arithmetic,
/// with none of the search, degree bookkeeping, or active-submatrix
/// maintenance (`MarkowitzState`'s `BTreeMap`/`BTreeSet` churn) that
/// choosing one costs.
pub fn factorize_reusing(m: usize, rows_in: &[Vec<(usize, f64)>], prev: Option<&FtLu>) -> Option<FtLu> {
    let Some(prev) = prev else {
        return factorize(m, rows_in).map(FtLu::new);
    };
    // A dense input goes to `factorize_dense_faer` (via `factorize`)
    // regardless of any pivot order, and the bordered path wants its own
    // ordering — reuse targets the ordinary sparse Markowitz case, which
    // is every mid-solve refactorization on a real Netlib basis.
    let eligible = reuse_pivot_order_enabled() && m > 0 && !is_dense_input(m, rows_in) && !wants_bordered(m, rows_in);
    if eligible && prev.reuse_skips_left == 0 {
        let t0 = std::time::Instant::now();
        PROF_REBUILD_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        let max_nnz = (reuse_fill_limit() * prev.fill_baseline as f64) as usize + m;
        let rebuilt = factorize_reusing_order(m, rows_in, &prev.base.col_perm, &prev.base.row_perm, max_nnz);
        PROF_REBUILD_NS.fetch_add(t0.elapsed().as_nanos() as usize, Ordering::Relaxed);
        if let Some(lu) = rebuilt {
            PROF_REBUILD_ACCEPTED.fetch_add(1, Ordering::Relaxed);
            let mut ft = FtLu::new(lu);
            PROF_REBUILD_ACCEPTED_NNZ.fetch_add(ft.fill_baseline, Ordering::Relaxed);
            // Keep the *full* factorization's baseline (see
            // `REBUILD_FILL_LIMIT`'s own docs): a chain of reuses must not
            // ratchet the fill limit up step by step.
            ft.fill_baseline = prev.fill_baseline;
            return Some(ft);
        }
    }
    // Either the reuse was rejected, or it is being skipped under the
    // backoff below. Both land on the full Markowitz factorization, whose
    // own fill becomes the new baseline (`FtLu::new` sets it).
    let mut ft = FtLu::new(factorize(m, rows_in)?);
    PROF_FULL_NNZ.fetch_add(ft.fill_baseline, Ordering::Relaxed);
    PROF_FULL_COUNT.fetch_add(1, Ordering::Relaxed);
    if eligible {
        if prev.reuse_skips_left > 0 {
            ft.reuse_fail_streak = prev.reuse_fail_streak;
            ft.reuse_skips_left = prev.reuse_skips_left - 1;
        } else {
            // A rejected attempt is wasted work on top of the full
            // factorization that follows it, and rejections cluster: the
            // basis that produced one (too much fill under the recorded
            // column order, or a numerically spent order) is usually still
            // producing them a few refactorizations later. So back off
            // exponentially — 2, 4, 8, ... refactorizations left alone,
            // capped at `REUSE_MAX_BACKOFF` — and reset to zero on the first
            // acceptance, which is what keeps a problem where reuse *does*
            // work paying nothing for this.
            ft.reuse_fail_streak = prev.reuse_fail_streak.saturating_add(1);
            ft.reuse_skips_left = (1u32 << ft.reuse_fail_streak.min(REUSE_BACKOFF_SHIFT_CAP)).min(REUSE_MAX_BACKOFF);
            PROF_REBUILD_BACKOFF_SKIPS.fetch_add(ft.reuse_skips_left as usize, Ordering::Relaxed);
        }
    }
    Some(ft)
}

/// Whether [`factorize`] would route this input through
/// [`factorize_bordered`] — exactly that function's own gate, factored out
/// so [`factorize_reusing`] can decline to touch such a basis.
///
/// Reuse must not take a `fit1p`/`fit2p`-shaped basis: the bordered path's
/// whole point is to keep ~20-25 near-dense "trend" columns *out* of the
/// sparse elimination entirely (see [`factorize_bordered`]'s own docs), and
/// a plain left-looking pass over the order it produced scatters exactly
/// those columns back through every step — measured as `fit2p` +6.4% at a
/// 1.1 fill limit and +16.3% at 1.25, against roughly flat everywhere else,
/// which is what sent this gate in.
fn wants_bordered(m: usize, rows_in: &[Vec<(usize, f64)>]) -> bool {
    let k = detect_border_columns(m, rows_in).len();
    k > 0 && k <= BORDER_MAX_COUNT && (k as f64) <= BORDER_MAX_FRACTION * m as f64
}

/// Caps on [`factorize_reusing`]'s own exponential backoff after a
/// rejected reuse: the streak's shift is capped first (so the shift itself
/// can never overflow), then the resulting skip count.
const REUSE_BACKOFF_SHIFT_CAP: u32 = 5;
const REUSE_MAX_BACKOFF: u32 = 16;

/// Factorizes `rows_in` with the **column order given** rather than
/// searched for: step `s` eliminates column `pivot_col[s]`, pivoting on
/// row `pivot_row_hint[s]` when that row is still numerically acceptable
/// and on the largest remaining entry in the column otherwise.
///
/// Left-looking (Gilbert-Peierls shape, the same one HiGHS's `rebuild()`
/// has): column `pivot_col[s]` is loaded, pushed through the part of `L`
/// built so far, and then split by the pivot assignment itself — entries
/// in rows already pivotal become `U`'s column `s`, the entry at the pivot
/// row becomes the diagonal, and entries in rows not yet pivotal become
/// `L`'s column `s`. Nothing here searches for *sparsity*, and nothing
/// maintains an active submatrix; per-step cost is the elimination
/// arithmetic plus the reach-set heap, both bounded by the factors' own
/// nonzero count.
///
/// **Why only the column order is replayed, not the row order.** HiGHS's
/// own `rebuild()` replays both (`refactor_info_.pivot_row` /
/// `pivot_var`) and gives up — rank deficiency, full rebuild — the moment
/// a recorded pivot row's entry is too small. That is affordable *there*
/// because HiGHS only ever sets `refactor_info_.use` for a hot start
/// (`HEkk::setNlaRefactorInfo`), i.e. when re-factorizing the very basis
/// the order was recorded from, where the recorded rows trivially still
/// work. Replaying both orders across a *changed* basis was implemented
/// here first and measured: it is rejected essentially always (Netlib
/// `25fv47` 0/26 attempts, `degen3` 0/7, `pilot` 0/30, `fit2p` 0/32,
/// `greenbea` 1/28), and the rejections are overwhelmingly "the recorded
/// pivot row is numerically *empty*" (`fail zero`, not `fail stability`) —
/// which is exactly what a replaced basis column looks like: the entering
/// column has no reason whatsoever to be nonzero at the row that was
/// pivotal for the column that left. The column order is what Markowitz's
/// fill-minimization actually encodes; the row assignment is a numerical
/// choice, and re-making it per step (partial pivoting: take the largest
/// remaining entry) costs one pass over the column that has already been
/// computed.
///
/// Returns `None` — caller falls back to [`factorize`] — if the order is
/// not a valid permutation, if some step's column has nothing left above
/// [`REBUILD_MIN_PIVOT`] in the remaining submatrix (a singular or
/// numerically spent basis), or if the factors exceed `max_nnz` nonzeros.
fn factorize_reusing_order(
    m: usize,
    rows_in: &[Vec<(usize, f64)>],
    pivot_col: &[usize],
    pivot_row_hint: &[usize],
    max_nnz: usize,
) -> Option<LuFactors> {
    if m == 0 || pivot_col.len() != m || pivot_row_hint.len() != m || rows_in.len() != m {
        return None;
    }
    {
        let mut seen = vec![false; m];
        for &j in pivot_col {
            if j >= m || seen[j] {
                return None;
            }
            seen[j] = true;
        }
    }
    // Read once for the whole factorization, exactly as
    // `MarkowitzState::new` does — this path reuses the previous
    // factorization's *column* order but still picks each pivot row under
    // the same threshold test, so an escalated threshold has to reach it
    // too (a rebuild that kept the old, looser floor would quietly undo
    // the escalation for as long as the pivot order keeps being reusable).
    let threshold = pivot_threshold();

    // Column-major copy of the (row-major) input: one `O(nnz)` counting
    // sort into flat arrays, the same shape `CscMat::from_rows` builds,
    // kept local because this one is indexed by *basis slot* and thrown
    // away when the factorization is done.
    let mut col_start = vec![0usize; m + 1];
    for row in rows_in.iter() {
        for &(j, _) in row {
            if j >= m {
                return None;
            }
            col_start[j + 1] += 1;
        }
    }
    for j in 0..m {
        col_start[j + 1] += col_start[j];
    }
    let mut col_row = vec![0usize; col_start[m]];
    let mut col_val = vec![0.0f64; col_start[m]];
    {
        let mut cursor = col_start.clone();
        for (i, row) in rows_in.iter().enumerate() {
            for &(j, v) in row {
                col_row[cursor[j]] = i;
                col_val[cursor[j]] = v;
                cursor[j] += 1;
            }
        }
    }

    let mut work = vec![0.0f64; m];
    let mut touched: Vec<usize> = Vec::new();
    let mut in_touched = vec![false; m];
    let mut heap: BinaryHeap<Reverse<usize>> = BinaryHeap::new();
    let mut queued = vec![false; m];

    // `L`'s columns as one flat buffer in *original row* indexing while
    // the rebuild runs (a row's own step is only known once it is chosen
    // as a pivot, which for `L`'s entries is always later than the step
    // writing them), converted to step indexing in one pass at the end.
    let mut l_entries: Vec<(usize, f64)> = Vec::new();
    let mut l_offsets: Vec<usize> = Vec::with_capacity(m + 1);
    l_offsets.push(0);
    let mut u_row: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    let mut lu_nnz = 0usize;

    // `row_step[r]`: the step row `r` was chosen as a pivot at, or
    // `usize::MAX` while it is still in the remaining submatrix. Becomes
    // `LuFactors::row_perm_inv` once every row has one.
    let mut row_step = vec![usize::MAX; m];
    let mut row_perm = vec![0usize; m];

    // `row_remaining[r]`: how many of row `r`'s own entries still sit in
    // columns this order has not reached yet — the row-degree half of a
    // Markowitz count, over the *input* matrix rather than the (never
    // materialized here) eliminated one. Maintained in `O(nnz)` total by
    // decrementing a column's rows as that column is consumed, and used
    // only to break the tie among numerically acceptable rows when the
    // recorded one is unusable: with the column order fixed, the row
    // choice is all that is left to keep fill down, and taking the
    // absolutely largest entry (plain partial pivoting) ignores sparsity
    // entirely.
    let mut row_remaining: Vec<u32> = rows_in.iter().map(|row| row.len() as u32).collect();

    for s in 0..m {
        let pj = pivot_col[s];
        for p in col_start[pj]..col_start[pj + 1] {
            row_remaining[col_row[p]] = row_remaining[col_row[p]].saturating_sub(1);
        }

        for p in col_start[pj]..col_start[pj + 1] {
            let r = col_row[p];
            work[r] += col_val[p];
            if !in_touched[r] {
                in_touched[r] = true;
                touched.push(r);
            }
            let t = row_step[r];
            if t != usize::MAX && !queued[t] {
                queued[t] = true;
                heap.push(Reverse(t));
            }
        }

        // Forward solve `L y = A[:, pj]` against the `s` columns of `L`
        // built so far. Steps come out of the heap in ascending order, and
        // column `t` of `L` only ever writes rows that were *not* pivotal
        // at step `t` (so their own step, if any, is `> t`), which makes
        // ascending step order a valid topological order — the same
        // property `l_solve_sparse_into` relies on, reached here with a
        // heap rather than a DFS because the graph is still being built.
        while let Some(Reverse(t)) = heap.pop() {
            queued[t] = false;
            let y = work[row_perm[t]];
            if y == 0.0 {
                continue;
            }
            for idx in l_offsets[t]..l_offsets[t + 1] {
                let (r, mult) = l_entries[idx];
                if !in_touched[r] {
                    in_touched[r] = true;
                    touched.push(r);
                }
                work[r] -= mult * y;
                let t2 = row_step[r];
                if t2 != usize::MAX && !queued[t2] {
                    queued[t2] = true;
                    heap.push(Reverse(t2));
                }
            }
        }

        // Pivot choice: the recorded row if it is still free and passes
        // the same threshold test `find_best_pivot` applies
        // ([`pivot_threshold`]
        // times the largest magnitude left in this column of the
        // *remaining* submatrix — which is what `work` now holds over the
        // rows that have no step yet); otherwise that largest entry
        // itself, i.e. plain partial pivoting.
        let mut best_r = usize::MAX;
        let mut best_abs = 0.0f64;
        for &r in &touched {
            if row_step[r] == usize::MAX {
                let a = work[r].abs();
                if a > best_abs {
                    best_abs = a;
                    best_r = r;
                }
            }
        }
        if best_abs < REBUILD_MIN_PIVOT {
            PROF_REBUILD_FAIL_SINGULAR.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        let hint = pivot_row_hint[s];
        let pi = if hint < m && row_step[hint] == usize::MAX && work[hint].abs() >= threshold * best_abs {
            hint
        } else {
            // The recorded row is gone (a basis column the Forrest-Tomlin
            // updates replaced leaves its old pivot row numerically empty
            // here — see this function's own docs): re-pick among the rows
            // that clear the same [`pivot_threshold`] floor `find_best_pivot`
            // applies, taking the one with the fewest entries left in
            // columns this order has yet to reach, ties going to the
            // larger pivot. That is the surviving half of a Markowitz
            // count once the column is fixed.
            PROF_REBUILD_ROW_REPICKS.fetch_add(1, Ordering::Relaxed);
            let floor = threshold * best_abs;
            let mut pick = best_r;
            let mut pick_deg = u32::MAX;
            let mut pick_abs = 0.0f64;
            for &r in &touched {
                if row_step[r] != usize::MAX {
                    continue;
                }
                let a = work[r].abs();
                if a < floor {
                    continue;
                }
                let deg = row_remaining[r];
                if deg < pick_deg || (deg == pick_deg && a > pick_abs) {
                    pick_deg = deg;
                    pick_abs = a;
                    pick = r;
                }
            }
            pick
        };
        let pivot = work[pi];
        row_step[pi] = s;
        row_perm[s] = pi;

        for &r in &touched {
            let v = work[r];
            work[r] = 0.0;
            in_touched[r] = false;
            if v == 0.0 {
                continue;
            }
            let t = row_step[r];
            if t == usize::MAX {
                l_entries.push((r, v / pivot));
            } else {
                // `t <= s` here: `r` is either pivotal from an earlier
                // step or the pivot just chosen for this one.
                u_row[t].push((s, v));
            }
            lu_nnz += 1;
        }
        touched.clear();
        l_offsets.push(l_entries.len());
        if lu_nnz > max_nnz {
            PROF_REBUILD_FAIL_FILL.fetch_add(1, Ordering::Relaxed);
            return None;
        }
    }

    let mut col_perm_inv = vec![0usize; m];
    for (s, &j) in pivot_col.iter().enumerate() {
        col_perm_inv[j] = s;
    }
    let mut l_build = CscBuilder::with_capacity(m, m, l_entries.len());
    for s in 0..m {
        for idx in l_offsets[s]..l_offsets[s + 1] {
            let (r, mult) = l_entries[idx];
            l_build.push(row_step[r], mult);
        }
        l_build.end_column();
    }
    let l_col = l_build.build();
    let l_row = l_col.to_csr();

    Some(LuFactors {
        m,
        l_col,
        l_row,
        u_row,
        row_perm,
        col_perm: pivot_col.to_vec(),
        col_perm_inv,
        row_perm_inv: row_step,
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

        state.remove_from_bucket_col(pj);

        let pivot_val = state.value_at(pi, pj).unwrap();
        let __t_snap0 = if debug_eliminate_cost { Some(std::time::Instant::now()) } else { None };
        let pivot_row_snapshot: Vec<(usize, f64)> = state
            .row(pi)
            .iter()
            .copied()
            .filter(|&(j, v)| v != 0.0 && (j == pj || !state.col_used[j]))
            .collect();
        if let Some(t0) = __t_snap0 {
            snapshot_ns += t0.elapsed().as_nanos();
        }

        for &(j, v) in &pivot_row_snapshot {
            u_entries.push((step, j, v));
        }

        let __t_elim0 = if debug_eliminate_cost { Some(std::time::Instant::now()) } else { None };
        state.eliminate(pi, pj, pivot_val, &pivot_row_snapshot);
        for &(i, mult) in state.l_out() {
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

    // `l_entries` is already grouped by `pivot_step` in ascending order —
    // the elimination loop above emits step `s`'s whole `L` column before
    // moving to step `s + 1` — so `L` can be appended straight into its
    // final compressed buffer, with no counting pass and no intermediate
    // per-column `Vec`s (see `LuFactors::l_col`'s own docs).
    debug_assert!(l_entries.windows(2).all(|w| w[0].1 <= w[1].1), "L entries must be grouped by ascending pivot step");
    let mut l_build = CscBuilder::with_capacity(m, m, l_entries.len());
    let mut next = 0usize;
    for step in 0..m {
        while next < l_entries.len() && l_entries[next].1 == step {
            let (orig_row, _, mult) = l_entries[next];
            l_build.push(row_perm_inv[orig_row], mult);
            next += 1;
        }
        l_build.end_column();
    }
    let l_col = l_build.build();
    let mut u_row: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for (pivot_step, orig_col, val) in u_entries {
        u_row[pivot_step].push((col_perm_inv[orig_col], val));
    }

    let l_row = build_l_row(&l_col, m);
    Some(LuFactors { m, l_col, l_row, u_row, row_perm, col_perm, col_perm_inv, row_perm_inv })
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
        state.remove_from_bucket_col(pj);

        let pivot_val = state.value_at(pi, pj).unwrap();
        let pivot_row_snapshot: Vec<(usize, f64)> = state
            .row(pi)
            .iter()
            .copied()
            .filter(|&(j, v)| v != 0.0 && (j == pj || !state.col_used[j]))
            .collect();
        for &(j, v) in &pivot_row_snapshot {
            u_entries.push((step, j, v));
        }
        state.eliminate(pi, pj, pivot_val, &pivot_row_snapshot);
        for &(i, mult) in state.l_out() {
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

    // As in `factorize_flat_markowitz`: `l_entries` covers steps
    // `0..n_sparse` in ascending order, and the border block's own columns
    // follow at `n_sparse..m`, also ascending — so the whole of `L` is
    // still emitted in column order and goes straight into the compressed
    // buffer (see `LuFactors::l_col`'s own docs).
    debug_assert!(l_entries.windows(2).all(|w| w[0].1 <= w[1].1), "L entries must be grouped by ascending pivot step");
    let mut l_build = CscBuilder::with_capacity(m, m, l_entries.len() + border_lu.l_col.nnz());
    let mut next = 0usize;
    for step in 0..n_sparse {
        while next < l_entries.len() && l_entries[next].1 == step {
            let (orig_row, _, mult) = l_entries[next];
            l_build.push(row_perm_inv_full[orig_row], mult);
            next += 1;
        }
        l_build.end_column();
    }
    for s in 0..k {
        for &(row_step, mult) in border_lu.l_col.col(s) {
            l_build.push(n_sparse + row_step, mult);
        }
        l_build.end_column();
    }
    let l_col = l_build.build();
    let mut u_row: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for (step, orig_col, val) in u_entries {
        u_row[step].push((col_perm_inv_full[orig_col], val));
    }
    for s in 0..k {
        for &(col_step, val) in &border_lu.u_row[s] {
            u_row[n_sparse + s].push((n_sparse + col_step, val));
        }
    }

    let l_row = build_l_row(&l_col, m);
    Some(LuFactors { m, l_col, l_row, u_row, row_perm, col_perm, col_perm_inv: col_perm_inv_full, row_perm_inv: row_perm_inv_full })
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
            for &(row_step, mult) in self.l_col.col(s) {
                z[row_step] -= mult * z[s];
            }
        }
    }

    /// [`Self::l_solve_into`] on two right-hand sides in one pass over `L`:
    /// each step's column is read once for both, while each vector sees
    /// exactly the operations — in exactly the order — its own call would.
    fn l_solve2_into(&self, rhs1: &[f64], z1: &mut [f64], rhs2: &[f64], z2: &mut [f64]) {
        let m = self.m;
        for s in 0..m {
            let src = self.row_perm[s];
            z1[s] = rhs1[src];
            z2[s] = rhs2[src];
        }
        for s in 0..m {
            let (a, b) = (z1[s], z2[s]);
            if a == 0.0 && b == 0.0 {
                continue;
            }
            let col = self.l_col.col(s);
            if a != 0.0 {
                for &(row_step, mult) in col {
                    z1[row_step] -= mult * a;
                }
            }
            if b != 0.0 {
                for &(row_step, mult) in col {
                    z2[row_step] -= mult * b;
                }
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

        scratch.visited.begin();
        scratch.reach.clear();
        for i in 0..scratch.seeds.len() {
            let seed = scratch.seeds[i];
            if scratch.visited.is_marked(seed) {
                continue;
            }
            scratch.visited.mark(seed);
            scratch.stack.push(seed);
            while let Some(node) = scratch.stack.pop() {
                scratch.reach.push(node);
                for &(next, _) in self.l_col.col(node) {
                    if !scratch.visited.is_marked(next) {
                        scratch.visited.mark(next);
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
            for &(row_step, mult) in self.l_col.col(s) {
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
    ///
    /// **This is the pre-[`LuFactors::l_row`] gather form, kept only as the
    /// `ENOMOTO_BTRAN_L_SCATTER=0` A/B arm** (and as the reference the
    /// scatter form's own unit tests check against) — see
    /// [`Self::l_transpose_solve_scatter_into`], which is what every
    /// production BTRAN actually calls.
    fn l_transpose_solve_gather_into(&self, w: &mut [f64], y: &mut [f64]) {
        let m = self.m;
        for s in (0..m).rev() {
            for &(row_step, mult) in self.l_col.col(s) {
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

    /// [`Self::l_transpose_solve_gather_into`]'s own triangular solve, read
    /// through the row-major mirror [`LuFactors::l_row`] instead of the
    /// column-major `l_col` — turning BTRAN's `L^{-T}` stage from a gather
    /// into a scatter, which is what makes it hyper-sparse.
    ///
    /// The two loops compute the same `L^T w' = w` back substitution over
    /// the same nonzeros, only associating the updates differently: the
    /// gather form accumulates *into* `w[s]` one `l_col[s]` entry at a
    /// time (so `w[s]`'s own value is only known once every one of them
    /// has been read, and no prefix of them can be skipped as a group),
    /// while this form propagates *out of* `w[s]` into every `l_row.row(s)`
    /// entry at once. Because `w[s]` is the single multiplicand of that
    /// whole inner loop, `w[s] == 0.0` makes the entire step a provable
    /// no-op — the same whole-step skip [`Self::l_solve_into`] and
    /// [`FtLu::u_solve_into`] already exploit in the forward direction,
    /// and the one Hall & McKinnon (2000) §4.4 explains the gather form
    /// *cannot* have ("no simple way of determining [a trivial]
    /// intersection... without a computational overhead comparable to
    /// evaluating the inner product itself"). HiGHS reaches the same skip
    /// the same way, via its own row-major `lr_*` copy of `L` in `btranL`.
    ///
    /// `docs/lu_comparison_enomoto_vs_highs.md` §2.6 names this as the one
    /// of HiGHS's four hyper-sparse solve directions this crate had never
    /// attempted (the *reason* being precisely that no row-major `L`
    /// existed to attempt it with — this method adds it).
    ///
    /// Not bit-identical to the gather form: the same set of products is
    /// summed into each `w[s]` in the opposite order (descending source
    /// step here, `l_col[s]`'s own stored order there), so results can
    /// differ in the last ulp and, through the dual ratio test's
    /// tie-breaks, shift iteration counts either way on degeneracy-heavy
    /// instances. That is measured, not assumed — see this change's own
    /// analysis note for the per-problem numbers.
    fn l_transpose_solve_scatter_into(&self, w: &mut [f64], y: &mut [f64]) {
        let m = self.m;
        for s in (0..m).rev() {
            let ws = w[s];
            if ws == 0.0 {
                continue;
            }
            for &(k, mult) in self.l_row.row(s) {
                w[k] -= mult * ws;
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
            for &(row_step, mult) in self.l_col.col(s) {
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
            for &(row_step, mult) in self.l_col.col(s) {
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

// An eta's off-diagonal entries are a [`HybridVec`]: a sparse `(row_step,
// value)` list while the eta is genuinely sparse, a dense length-`m` array
// once its fill exceeds [`DENSE_ETA_FRACTION`] of `m` (typical of a
// dense-coefficient LP, where `U`'s eta chain is already close to fully
// dense from the very first update). See that type's own docs for the
// trade-off, for the skipped-slot convention that lets the dense form's
// loops run over the whole array unconditionally, and for why its two
// consuming operations (`dot_dense`, `axpy_into_dense`) live there rather
// than being re-written as a two-armed `match` at each of this file's
// eight FTRAN/BTRAN call sites.

/// A column/row whose off-diagonal fill exceeds this fraction of `m` is
/// stored densely (see [`HybridVec`]). Unlike [`DENSE_COL_FRACTION`] (tuned
/// against real Netlib data, all of it sparse), this threshold has no
/// dense-problem benchmark to tune against yet in this crate's own test
/// set — `0.4` is a first-pass value, not a measured one; re-tune once a
/// genuinely dense-coefficient LP is available to benchmark against.
const DENSE_ETA_FRACTION: f64 = 0.4;

#[derive(Clone)]
struct UEta {
    slot: usize,
    pivot: f64,
    off_diag: HybridVec, // (row_step, value) pairs, row_step != slot
}

#[derive(Clone)]
struct REta {
    p: usize,
    r: HybridVec, // (row_step, value) pairs, row_step != p
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
const DENSITY_AVERAGE_MULTIPLIER: f64 = 0.05;

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
const EXPECTED_DENSE_FRACTION: f64 = 0.35;

/// [`EXPECTED_DENSE_FRACTION`], overridable at run time via the
/// `ENOMOTO_EXPECTED_DENSITY_GATE` environment variable (a bare float;
/// any value `>= 1.0` disables the result-density gate outright, since no
/// result can be denser than `m`, restoring the input-nnz-only dispatch
/// this crate had before [`FtranDensity`] existed — which is exactly how
/// the A/B runs behind the constant's own value were produced). Read once
/// per [`FtranDensity::new`] — a handful of times per solve, never on the
/// per-iteration path.
fn expected_dense_gate() -> f64 {
    std::env::var("ENOMOTO_EXPECTED_DENSITY_GATE")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(EXPECTED_DENSE_FRACTION)
}

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
const BTRAN_L_SCATTER_FRACTION: f64 = 0.10;

/// Builds [`LuFactors::l_row`] — or, when the scatter form is disabled
/// outright (`ENOMOTO_BTRAN_L_SCATTER=0`), an empty stand-in with the same
/// `m` outer slots and no entries.
///
/// The empty case exists so that arm of the A/B is *genuinely* this crate's
/// pre-§2.6 behaviour, construction cost included. Building `l_row` and
/// then never reading it would leave the transpose's own `O(nnz(L))` build
/// — paid at **every** refactorization, `dfl001` alone refactorizes ~100
/// times — inside both arms, hiding exactly the cost that has to be
/// weighed against the scatter form's own win. Measuring a change against
/// a baseline that already pays for it is how a feature gets adopted on a
/// number that was never real.
///
/// An all-empty `CscMat` transposes into a `CsrMat` with `m + 1` zero
/// offsets and no entries, so `row(i)` stays valid (and empty) for every
/// `i` rather than needing a separate `Option` on the hot path.
fn build_l_row(l_col: &CscMat, m: usize) -> CsrMat {
    if btran_l_scatter_gate() <= 0.0 {
        return CscMat::empty(m, m).to_csr();
    }
    l_col.to_csr()
}

/// [`BTRAN_L_SCATTER_FRACTION`], overridable via `ENOMOTO_BTRAN_L_SCATTER`
/// — `0` disables the scatter form outright (restoring the pre-`l_row`
/// gather-only BTRAN, which is how the A/B behind the constant's own value
/// is produced), `1` forces it unconditionally. Read once per
/// refactorization, never per solve, same as [`expected_dense_gate`].
fn btran_l_scatter_gate() -> f64 {
    std::env::var("ENOMOTO_BTRAN_L_SCATTER").ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(BTRAN_L_SCATTER_FRACTION)
}

/// Whether [`FtLu::u_solve_into`] tests a slot for zero *before* dividing
/// it by its eta's pivot rather than after — see that method's own docs.
/// `ENOMOTO_FTRAN_U_ZERO_SKIP=0` restores the unconditional divide, which
/// is how the A/B behind the default is produced. Read once per
/// [`FtLu::new`], never per solve.
fn u_zero_skip_enabled() -> bool {
    std::env::var("ENOMOTO_FTRAN_U_ZERO_SKIP").map(|v| v != "0").unwrap_or(true)
}

/// Running average of one FTRAN *call site*'s own **result** density,
/// feeding [`FtLu::should_use_dense_solve_tracked`]'s dense/sparse
/// dispatch alongside the right-hand side's own nonzero count.
///
/// [`DENSE_RHS_FRACTION`] alone judges a solve by its *input*: the reach
/// set the Gilbert-Peierls path walks is bounded below by the rhs's own
/// nonzeros, so a dense rhs does prove the sparse path cannot win. The
/// converse is not true — a one-nonzero rhs can still fill in to a fully
/// dense `B^-1 a` once `L`'s own reach fans out, and then the sparse
/// path has paid its DFS/epoch bookkeeping (`LuFactors::l_solve_sparse_into`'s
/// own docs) on top of doing the same elimination work the flat dense
/// scan would have done anyway. Nothing about the *input* distinguishes
/// those two cases, and the gap widens exactly as `m` grows: the bigger
/// the basis, the further a single column's reach can fan out relative to
/// the fixed sparsity of the column itself.
///
/// What does distinguish them is the channel's own recent history, which
/// is what this tracks: HiGHS solves the same problem the same way,
/// maintaining a per-operation `expected_density` running average
/// (`HEkk::updateOperationResultDensity`) and handing it to `ftranL`/`ftranU`
/// so each call can decide *before* running which mode it should be in
/// (`HFactor::ftranL`'s own `expected_density > kHyperFtranL` test).
/// This crate's `docs/lu_comparison_enomoto_vs_highs.md` §2.7 names that
/// as the gap this type closes.
///
/// One instance per *call site* (the entering column's FTRAN, the BFRT
/// combined-flip FTRAN, ...), never one shared instance: those channels'
/// densities genuinely differ — a BFRT combined rhs sums whole flipped
/// columns and is routinely much denser than a single entering column —
/// and averaging them together would smear each one's own signal.
/// Instances live in the solve loops (`solve_lp_dual_on` and
/// `extended_dual`'s two loops), not in [`FtLu`] itself, deliberately:
/// `FtLu` is rebuilt from scratch at every refactorization, which would
/// throw the history away precisely when the basis is at its densest,
/// whereas HiGHS's own densities likewise live in `HEkk` and survive
/// across INVERTs.
///
/// The measurement itself is free: every solve path already ends in an
/// `O(m)` permutation loop over the finished result, so counting that
/// result's nonzeros costs one branchless add per entry inside a loop
/// that was already running — and it is the *exact* result density, not
/// an estimate. Crucially it is also taken on **both** branches, so the
/// gate can never latch: a channel that starts producing sparse results
/// again is observed doing so while it is on the dense path, and returns
/// to the sparse path on its own.
#[derive(Clone, Copy, Debug)]
pub struct FtranDensity {
    /// Running average of `result_nnz / m`, in `[0, 1]`. Starts at `0.0`
    /// (maximally sparse) so a fresh channel dispatches exactly as it did
    /// before this type existed until it has actually observed something.
    expected: f64,
    /// [`expected_dense_gate`]'s value, captured once at construction
    /// rather than re-read per call.
    gate: f64,
}

impl FtranDensity {
    pub fn new() -> Self {
        Self { expected: 0.0, gate: expected_dense_gate() }
    }

    /// Folds one finished solve's own result density into the average —
    /// call with whatever nonzero count the solve returned (`solve_into`
    /// and friends all return it), on *either* branch.
    #[inline]
    pub fn record(&mut self, result_nnz: usize, m: usize) {
        if m == 0 {
            return;
        }
        let local = result_nnz as f64 / m as f64;
        self.expected = (1.0 - DENSITY_AVERAGE_MULTIPLIER) * self.expected + DENSITY_AVERAGE_MULTIPLIER * local;
    }

    /// The running average itself, in `[0, 1]` — exposed for diagnostics
    /// (`ENOMOTO_PROF_PHASES_EXT`) rather than for dispatch, which goes
    /// through [`FtLu::should_use_dense_solve_tracked`].
    #[inline]
    pub fn expected(&self) -> f64 {
        self.expected
    }

    /// Whether this channel's own history alone already calls for the
    /// dense path.
    #[inline]
    pub fn predicts_dense(&self) -> bool {
        self.expected > self.gate
    }
}

impl Default for FtranDensity {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct FtLu {
    base: LuFactors,
    /// `U`'s etas, physically stored in **creation order** — exactly as
    /// before — so `u_transpose_solve_into`/`u_solve_into` (the hot,
    /// once-*every*-iteration BTRAN/FTRAN paths, not just `try_update`)
    /// keep a plain sequential scan with no pointer-chasing indirection.
    u_seq: Vec<UEta>,
    /// The **singleton** `U` etas (empty `off_diag`: a bare diagonal pivot)
    /// of the factorization as built, held apart from `u_seq` — 42-99% of
    /// all `m` etas right after a refactorization on the heavy Netlib
    /// problems. A singleton's own division reads and writes only its own
    /// slot, and by `u_seq`'s triangular order every eta that writes into
    /// that slot (FTRAN) or reads it (BTRAN) sits *later* in the sequence,
    /// so its division can run after the whole `u_seq` pass in
    /// [`Self::u_solve_into`] and before it in the `U^T` sweeps without
    /// changing a single result bit — while the sequential loops skip
    /// visiting them at all. Order within `singles` is irrelevant (the
    /// divisions are independent); `commit_update` pulls a replaced one
    /// out with `swap_remove`. `u_seq` never feeds an eta back in here:
    /// an eta that loses its last off-diagonal entry later just stays in
    /// `u_seq`, exact either way.
    singles: Vec<UEta>,
    /// `singles_pos[slot]` is `slot`'s index into `singles`, `usize::MAX`
    /// when `slot` lives in `u_seq` instead (and vice versa for
    /// `slot_pos`).
    singles_pos: Vec<usize>,
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
    row_owners: Vec<Vec<(usize, f64)>>,
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
    /// Deterministic operation-count accumulator for the `CLOCK`
    /// refactorization trigger (`ENOMOTO_SYNTH_CLOCK_FACTOR`'s own docs at
    /// its call sites in `extended_dual.rs`) — this crate's counterpart to
    /// HiGHS's `total_synthetic_tick_` (`HFactor.cpp`/`HEkk.cpp`). Unlike
    /// the wall-clock prototype this replaces
    /// (`analysis/ft_refactor_trigger_20260922_040850.md` §5/§6), every
    /// increment here is a plain nonzero-count add driven only by the
    /// (already-deterministic) eta chain and right-hand-side content, never
    /// by `Instant::now()` — so two solves of the same problem always
    /// accumulate the exact same tick sequence and therefore refactorize at
    /// the exact same iterations, keeping the whole solve bit-reproducible.
    /// A `Cell` (not a plain field) because every solve stage that adds to
    /// it (`ftran_through_l_and_r_into`, `solve_sparse_into[_capture]`,
    /// `u_solve_into`, `u_transpose_solve_into`, `solve_transpose_into[_capture]`)
    /// takes `&self`. Reset implicitly to `0`
    /// every time a new `FtLu` is built (`Self::new`, i.e. every
    /// refactorization) — there is no explicit reset method because a fresh
    /// `FtLu` *is* the reset.
    tick: Cell<u64>,
    /// Running total of the off-diagonal fill currently held across
    /// `u_seq` and `r_etas` — [`Self::fill_count`]'s answer, maintained
    /// incrementally by [`Self::commit_update`] rather than re-summed on
    /// demand.
    ///
    /// The refactorization trigger reads it **once per simplex iteration**
    /// (`simplex.rs`'s own trigger (3)), and re-summing meant walking all
    /// `m` entries of `u_seq` — touching every `UEta` header in the
    /// process — for a number that changes only at the handful of places
    /// an update already touches. Its own doc comment called that sum
    /// "`O(1)`-ish"; it was `O(m)`, one more full sweep of the eta file
    /// per iteration on top of the ones `u_solve_into`/
    /// `u_transpose_solve_into` genuinely need. This is that number
    /// actually being `O(1)`, with a `debug_assert` in `fill_count` that
    /// it still agrees with the sum it replaced.
    fill: usize,
    /// Nonzero count of the last *full* (Markowitz) factorization's
    /// `L`+`U` — the baseline [`factorize_reusing`] caps a reused order's
    /// own fill against (see [`REBUILD_FILL_LIMIT`]). Set in [`Self::new`]
    /// to this factorization's own count, then overwritten back to the
    /// predecessor's by `factorize_reusing` whenever the factorization it
    /// just built came from a reuse rather than a fresh Markowitz run, so
    /// a chain of reuses is always measured against the last order
    /// actually chosen by Markowitz.
    fill_baseline: usize,
    /// Consecutive rejected pivot-order reuses leading up to this
    /// factorization, and how many refactorizations are still to be left
    /// alone before the next attempt — see [`factorize_reusing`]'s own
    /// backoff docs. Carried across refactorizations the same way
    /// [`Self::fill_baseline`] is.
    reuse_fail_streak: u32,
    reuse_skips_left: u32,
    /// This factorization's own one-time build cost, in the same tick
    /// units as [`Self::tick`] — computed once in [`Self::new`] from the
    /// freshly-built `L`/`U` (`m` rows plus their combined off-diagonal
    /// nonzero count), never recomputed afterward. The `CLOCK` trigger
    /// refactorizes once `tick` reaches `FACTOR * build_tick`, i.e. once
    /// the *solving* work done against this factorization is estimated to
    /// cost as much as `FACTOR` fresh refactorizations of it would have —
    /// see [`TICK_BUILD_M_COEF`]/[`TICK_BUILD_LU_COEF`]'s own docs for
    /// where the two coefficients come from.
    build_tick: u64,
    /// [`btran_l_scatter_gate`]'s value, captured once per refactorization
    /// rather than re-read per call — see [`BTRAN_L_SCATTER_FRACTION`].
    /// Lives on [`FtLu`] rather than [`LuFactors`] so the factor struct
    /// stays pure data.
    btran_l_scatter: f64,
    /// [`u_zero_skip_enabled`]'s value, captured once per refactorization —
    /// see [`Self::u_solve_into`].
    u_zero_skip: bool,
}

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
const TICK_BUILD_M_COEF: u64 = 80;
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
const TICK_BUILD_LU_COEF: u64 = 60;
/// Per-nonzero coefficient applied to every solve-stage tick increment
/// (`R`-eta nonzeros touched, `U`/`U^T`-eta nonzeros touched, `L`-stage
/// reach-set size) — kept at `1` (i.e. `tick` is a plain nonzero count,
/// unscaled) so [`SYNTH_CLOCK_FACTOR`] alone carries the crate-specific
/// per-nonzero cost ratio between this crate's own solves and HiGHS's; splitting that ratio across two constants
/// (this one and the factor) would make calibration harder to reason about
/// with no accuracy benefit, since both only ever appear multiplied
/// together in the trigger's own comparison.
const TICK_SOLVE_NNZ_COEF: u64 = 1;

impl FtLu {
    pub fn new(base: LuFactors) -> Self {
        let m = base.m;
        // Exact per-slot/per-row counts first so every inner `Vec` below is
        // allocated once at its final size (this runs on every
        // refactorization; growing ~2m small `Vec`s push by push was a
        // visible share of the allocator's time).
        let mut off_count = vec![0usize; m];
        let mut owner_count = vec![0usize; m];
        for row_step in 0..m {
            for &(col_step, _) in &base.u_row[row_step] {
                if col_step != row_step {
                    off_count[col_step] += 1;
                    owner_count[row_step] += 1;
                }
            }
        }
        let mut off_diags: Vec<Vec<(usize, f64)>> = off_count.iter().map(|&c| Vec::with_capacity(c)).collect();
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
        let mut row_owners: Vec<Vec<(usize, f64)>> = owner_count.iter().map(|&c| Vec::with_capacity(c)).collect();
        for (slot, pairs) in off_diags.iter().enumerate() {
            for &(row_step, v) in pairs {
                row_owners[row_step].push((slot, v));
            }
        }
        let l_nnz: u64 = base.l_col.nnz() as u64;
        // `u_row[s]` includes its own diagonal entry (`col_step == s`,
        // filtered out just above into `pivots`), so its off-diagonal count
        // is one less than its length — mirrors HiGHS's own `u_countX`
        // (`HFactor.cpp`'s `buildFinish`), which likewise counts only
        // off-diagonal `U` nonzeros.
        let u_off: u64 = base.u_row.iter().map(|v| v.len().saturating_sub(1) as u64).sum();
        let build_tick = TICK_BUILD_M_COEF * m as u64 + TICK_BUILD_LU_COEF * (l_nnz + u_off);
        // `u_off` above excludes `U`'s diagonals; the fill baseline counts
        // every stored entry, matching what `factorize_reusing_order`
        // counts as it goes.
        let fill_baseline = (l_nnz + u_off) as usize + m;
        let mut u_seq: Vec<UEta> = Vec::with_capacity(m);
        let mut singles: Vec<UEta> = Vec::new();
        let mut slot_pos = vec![usize::MAX; m];
        let mut singles_pos = vec![usize::MAX; m];
        for slot in 0..m {
            let eta = UEta {
                slot,
                pivot: pivots[slot],
                off_diag: HybridVec::pack(m, std::mem::take(&mut off_diags[slot]), DENSE_ETA_FRACTION),
            };
            if eta.off_diag.nnz() == 0 {
                singles_pos[slot] = singles.len();
                singles.push(eta);
            } else {
                slot_pos[slot] = u_seq.len();
                u_seq.push(eta);
            }
        }
        let fill = u_seq.iter().map(|e: &UEta| e.off_diag.nnz()).sum();
        FtLu {
            base,
            u_seq,
            singles,
            singles_pos,
            slot_pos,
            row_owners,
            r_etas: Vec::new(),
            scratch_a_tilde: vec![0.0; m],
            scratch_e_tilde: vec![0.0; m],
            fill,
            tick: Cell::new(0),
            build_tick,
            fill_baseline,
            reuse_fail_streak: 0,
            reuse_skips_left: 0,
            btran_l_scatter: btran_l_scatter_gate(),
            u_zero_skip: u_zero_skip_enabled(),
        }
    }

    /// BTRAN's `L^{-T}` stage: counts `w`'s nonzeros (bailing out of the
    /// count as soon as it is clearly over the line) and runs the
    /// row-major scatter form on a sparse `w`, the column-major gather
    /// form otherwise — see [`BTRAN_L_SCATTER_FRACTION`] for why both
    /// forms have to stay.
    fn l_transpose_solve_into(&self, w: &mut [f64], y: &mut [f64]) {
        let limit = (self.btran_l_scatter * self.base.m as f64) as usize;
        let mut nnz = 0usize;
        let mut sparse = true;
        for &v in w.iter() {
            nnz += (v != 0.0) as usize;
            if nnz > limit {
                sparse = false;
                break;
            }
        }
        if sparse {
            PROF_BTRAN_L_SCATTER.fetch_add(1, Ordering::Relaxed);
            self.base.l_transpose_solve_scatter_into(w, y);
        } else {
            PROF_BTRAN_L_GATHER.fetch_add(1, Ordering::Relaxed);
            self.base.l_transpose_solve_gather_into(w, y);
        }
    }

    /// Current value of the deterministic operation-count accumulator (see
    /// [`Self::tick`]'s own docs) — read by the `CLOCK` refactorization
    /// trigger in `extended_dual.rs`, never consulted by anything in this
    /// file itself.
    pub fn synth_tick(&self) -> u64 {
        self.tick.get()
    }

    /// This factorization's own build cost, in the same units as
    /// [`Self::synth_tick`] — see [`Self::build_tick`]'s own docs.
    pub fn build_tick(&self) -> u64 {
        self.build_tick
    }

    #[inline]
    fn add_tick(&self, n: u64) {
        self.tick.set(self.tick.get() + TICK_SOLVE_NNZ_COEF * n);
    }

    /// Whether an FTRAN right-hand side with `rhs_nnz` nonzero entries
    /// (out of this basis's `m`) is dense enough that callers should skip
    /// [`Self::solve_sparse_into`] in favor of the plain dense
    /// [`Self::solve_into`] — see [`DENSE_RHS_FRACTION`]'s own docs.
    pub fn should_use_dense_solve(&self, rhs_nnz: usize) -> bool {
        let m = self.base.m;
        m > 0 && rhs_nnz as f64 > DENSE_RHS_FRACTION * m as f64
    }

    /// [`Self::should_use_dense_solve`] widened by the calling channel's
    /// own observed result density: the dense path is taken when *either*
    /// the right-hand side handed in is already dense (that method's own
    /// input-side test, unchanged) *or* this channel's recent results have
    /// been dense enough that the sparse path's own bookkeeping is not
    /// expected to pay for itself ([`FtranDensity`]'s own docs, and
    /// `docs/lu_comparison_enomoto_vs_highs.md` §2.7).
    ///
    /// Deliberately only ever moves calls *towards* the dense path, never
    /// away from it: a rhs with more than [`DENSE_RHS_FRACTION`] of `m`
    /// nonzeros bounds the Gilbert-Peierls reach set below by that same
    /// count, so no amount of "but this channel's results are usually
    /// sparse" history could make the sparse path win on such a call.
    pub fn should_use_dense_solve_tracked(&self, rhs_nnz: usize, density: &FtranDensity) -> bool {
        self.should_use_dense_solve(rhs_nnz) || density.predicts_dense()
    }

    /// `U_k^{-T}` applied in place to a step-space vector: processes the
    /// eta sequence in **forward** (creation) order, each step solving for
    /// that eta's pivotal component via eq. (8). Mutates `z` directly
    /// (rather than allocating a fresh result) — see `LuFactors::l_solve_into`'s
    /// own docs for why this matters on `FtLu`'s hot path.
    ///
    /// Hyper-sparse via [`Self::row_owners`], unlike [`Self::u_solve_into`]
    /// (see that method's own docs for why a GP-style DFS reach set was
    /// tried there and reverted): the sweep ([`Self::u_transpose_sweep`])
    /// is in scatter form over `row_owners`' row-wise copy of `U`, so a
    /// slot whose value is zero costs one test and nothing else. (The
    /// earlier gather form reached the same sparsity through "needed"
    /// marks, `analysis/greenbea_20260921_090812.md` §3-4, but still paid
    /// each needed eta's whole column dot product.)
    fn u_transpose_solve_into(&self, z: &mut [f64]) {
        self.u_transpose_sweep(z);
    }

    /// [`Self::u_transpose_solve_into`] for the case the caller already
    /// knows `z`'s entire support: a single nonzero at step `seed`.
    ///
    /// This is the BTRAN whose right-hand side is a unit vector `e_i` —
    /// the pivotal-row `rho_p`, the steepest-edge weight update's own
    /// `rho`, and `DseState::from_basis`'s `m` reference solves are all
    /// that shape. Seeding the "needed" set from `seed` directly is
    /// *exact*, not approximate: [`Self::u_transpose_solve_into`]'s own
    /// seeding loop marks precisely the steps where `z` is nonzero, and
    /// for a permuted unit vector that set is exactly `{seed}`. What it
    /// saves is that `O(m)` scan — and, at the call site, the `O(m)`
    /// permutation *gather* (`z[s] = rhs[col_perm[s]]`, a random-access
    /// read per step) that materialized the unit vector in the first
    /// place, which [`Self::seed_unit_rhs`] replaces with a flat `fill` and
    /// one store.
    fn u_transpose_solve_seeded(&self, z: &mut [f64], seed: usize) {
        debug_assert!(z.iter().enumerate().all(|(s, &v)| s == seed || v == 0.0));
        self.u_transpose_sweep(z);
    }

    /// The `U^{-T}` sweep itself, shared by both seedings above.
    fn u_transpose_sweep(&self, z: &mut [f64]) {
        // Scatter form (HiGHS `HFactor::btranU` over its row-wise `ur_*`
        // copy): once slot `p`'s value is final it is divided by its pivot
        // and pushed into every eta that reads it (`row_owners[p]`, which
        // carries the `U` entry alongside the slot) — so the work is the
        // nonzero `p`s' row lengths, not, as the gather form this replaced
        // did, every needed eta's whole column dot product regardless of
        // how few of that column's inputs are nonzero (26x more entries on
        // `fit2p`, whose few dense border columns every BTRAN re-read).
        // Changes the summation order into each `z[q]`, so the last bits —
        // not the math — differ from the gather form.
        //
        // CLOCK-trigger accounting (`Self::tick`'s own docs): the flat `m`
        // for the `O(m)` walk over every slot, plus each scattered row's
        // length.
        self.add_tick(self.base.m as u64);
        // Singletons first (see `singles`' own docs): nothing writes into a
        // singleton's slot, so its value is final before the sweep starts.
        for eta in self.singles.iter().chain(self.u_seq.iter()) {
            let p = eta.slot;
            let zp = z[p];
            if zp == 0.0 {
                continue;
            }
            let zp = zp / eta.pivot;
            z[p] = zp;
            let owners = &self.row_owners[p];
            self.add_tick(owners.len() as u64);
            for &(q, v) in owners {
                z[q] -= v * zp;
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
            let y = eta.off_diag.dot_dense(z);
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
        // CLOCK-trigger accounting (`Self::tick`'s own docs): every eta
        // pays the `O(1)` division unconditionally (the `for` loop itself
        // always visits all of `u_seq`, one per basis row), so that part is
        // a flat `m`; the off-diagonal update below is the hyper-sparse
        // part this function's own docs describe, so its cost is added
        // only for etas whose `xp` actually survives the skip.
        self.add_tick(self.base.m as u64);
        if !self.u_zero_skip {
            // `ENOMOTO_FTRAN_U_ZERO_SKIP=0`: the pre-§2.6 loop exactly, so
            // that arm of the A/B is this crate's own previous behaviour
            // and not "previous behaviour plus one unrelated change".
            for eta in self.u_seq.iter().rev() {
                let p = eta.slot;
                x[p] /= eta.pivot;
                let xp = x[p];
                if xp == 0.0 {
                    continue;
                }
                self.add_tick(eta.off_diag.nnz() as u64);
                eta.off_diag.axpy_into_dense(-xp, x);
            }
            for eta in &self.singles {
                x[eta.slot] /= eta.pivot;
            }
            return;
        }
        for eta in self.u_seq.iter().rev() {
            let p = eta.slot;
            // Test *before* dividing, not after: `0.0 / pivot` is `±0.0`,
            // so an already-zero slot's division is a no-op that still
            // costs a division and — worse on a hyper-sparse right-hand
            // side — a store back into a random position of `x`, dirtying
            // a cache line per zero slot for nothing. The skipped store
            // can leave `+0.0` where the unconditional one would have
            // written `-0.0` (when `pivot < 0`), which is exactly the
            // difference `u_transpose_solve_into`'s own hyper-sparse skip
            // already accepts, on the same grounds: every consumer of this
            // result branches on zero-ness (`permute_out`'s own `!= 0.0`
            // count, `commit_update`'s filter, the PRICE/DSE consumers),
            // never on the sign of a zero.
            if x[p] == 0.0 {
                continue;
            }
            x[p] /= eta.pivot;
            let xp = x[p];
            self.add_tick(eta.off_diag.nnz() as u64);
            // `data[p] == 0.0` always (`HybridVec`'s skipped-index
            // convention), so the dense arm leaves `x[p]` — just divided
            // above — untouched, same as the sparse one.
            eta.off_diag.axpy_into_dense(-xp, x);
        }
        // Singletons last (see `singles`' own docs): every write into their
        // slots has happened by now. Same zero-skip as the loop above.
        for eta in &self.singles {
            let p = eta.slot;
            if x[p] != 0.0 {
                x[p] /= eta.pivot;
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
        // CLOCK-trigger accounting (`Self::tick`'s own docs): `l_solve_into`
        // is a dense `O(m)` scan of `z` regardless of fill (this is the
        // dense FTRAN path — the sparse `L`-stage reach set is accounted
        // separately in `solve_sparse_into`/`_capture`).
        self.add_tick(self.base.m as u64);
        for reta in &self.r_etas {
            let dot = reta.r.dot_dense(z);
            // Unconditional (every `r_eta` is visited regardless of `z`'s
            // sparsity — the very "gather-type, no zero-skip" cost this
            // trigger's own analysis (§2.1/§2.2) identified as the eta-chain
            // bottleneck), so this term alone is what makes `tick` grow
            // with chain length the way FTRAN's own measured wall time does.
            self.add_tick(reta.r.nnz() as u64);
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
    ///
    /// Returns the finished result's own nonzero count, for
    /// [`FtranDensity::record`]: the permutation loop below already visits
    /// every entry of the result, so counting them there is one branchless
    /// add per entry on a loop that was running anyway, and yields the
    /// exact density rather than an estimate. Callers with no density
    /// tracker simply ignore it.
    pub fn solve_into(&self, rhs: &[f64], scratch: &mut [f64], out: &mut [f64]) -> usize {
        self.ftran_through_l_and_r_into(rhs, scratch);
        self.u_solve_into(scratch);
        self.permute_out(scratch, out)
    }

    /// Same as [`Self::solve_into`], but additionally captures the
    /// post-`L`/`R`, pre-`U` intermediate (`(L R_1...R_{k-1})^-1 rhs`) into
    /// `a_tilde_out` (length `m`) — exactly the `a_tilde` value
    /// [`Self::try_update_precomputed`] needs when `rhs` is the entering
    /// column being FTRAN'd this same iteration. See that method's own
    /// docs for why this capture (a plain `copy_from_slice`) lets the
    /// caller skip `try_update`'s own redundant re-derivation of the exact
    /// same value entirely.
    pub fn solve_into_capture(&self, rhs: &[f64], scratch: &mut [f64], out: &mut [f64], a_tilde_out: &mut [f64]) -> usize {
        self.ftran_through_l_and_r_into(rhs, scratch);
        a_tilde_out.copy_from_slice(scratch);
        self.u_solve_into(scratch);
        self.permute_out(scratch, out)
    }

    /// [`Self::solve_into_capture`] on `rhs1` fused with a plain
    /// [`Self::solve_into`] on `rhs2`: one traversal of `L`, the `R` etas and
    /// `U` serves both, so each factor entry is fetched once instead of
    /// twice, while each vector's own arithmetic — every operation, in
    /// order — is exactly what its separate call does, so both results are
    /// bit-for-bit the separate calls' results. Charges the same CLOCK ticks
    /// as the two separate calls together. Returns both outputs' nonzero
    /// counts.
    /// Used for the entering column's FTRAN and the DSE `tau` FTRAN, which
    /// run against the same pre-pivot factorization every iteration.
    #[allow(clippy::too_many_arguments)]
    pub fn solve2_into_capture(
        &self,
        rhs1: &[f64],
        scratch1: &mut [f64],
        out1: &mut [f64],
        a_tilde_out: &mut [f64],
        rhs2: &[f64],
        scratch2: &mut [f64],
        out2: &mut [f64],
    ) -> (usize, usize) {
        let m = self.base.m as u64;
        self.base.l_solve2_into(rhs1, scratch1, rhs2, scratch2);
        self.add_tick(m);
        self.add_tick(m);
        for reta in &self.r_etas {
            let dot1 = reta.r.dot_dense(scratch1);
            scratch1[reta.p] -= dot1;
            let dot2 = reta.r.dot_dense(scratch2);
            scratch2[reta.p] -= dot2;
            self.add_tick(2 * reta.r.nnz() as u64);
        }
        a_tilde_out.copy_from_slice(scratch1);
        self.u_solve2_into(scratch1, scratch2);
        (self.permute_out(scratch1, out1), self.permute_out(scratch2, out2))
    }

    /// [`Self::u_solve_into`] on two vectors in one pass over `U` — see
    /// [`Self::solve2_into_capture`].
    fn u_solve2_into(&self, x1: &mut [f64], x2: &mut [f64]) {
        self.add_tick(self.base.m as u64);
        self.add_tick(self.base.m as u64);
        if !self.u_zero_skip {
            for eta in self.u_seq.iter().rev() {
                let p = eta.slot;
                for x in [&mut *x1, &mut *x2] {
                    x[p] /= eta.pivot;
                    let xp = x[p];
                    if xp == 0.0 {
                        continue;
                    }
                    self.add_tick(eta.off_diag.nnz() as u64);
                    eta.off_diag.axpy_into_dense(-xp, x);
                }
            }
            for eta in &self.singles {
                x1[eta.slot] /= eta.pivot;
                x2[eta.slot] /= eta.pivot;
            }
            return;
        }
        for eta in self.u_seq.iter().rev() {
            let p = eta.slot;
            for x in [&mut *x1, &mut *x2] {
                if x[p] == 0.0 {
                    continue;
                }
                x[p] /= eta.pivot;
                let xp = x[p];
                self.add_tick(eta.off_diag.nnz() as u64);
                eta.off_diag.axpy_into_dense(-xp, x);
            }
        }
        for eta in &self.singles {
            let p = eta.slot;
            if x1[p] != 0.0 {
                x1[p] /= eta.pivot;
            }
            if x2[p] != 0.0 {
                x2[p] /= eta.pivot;
            }
        }
    }

    /// The last stage every FTRAN path shares: map the finished
    /// step-space vector back to original row indexing, returning its own
    /// nonzero count (see [`Self::solve_into`]'s own docs for why the
    /// count rides along on this loop rather than a pass of its own).
    /// `+= (v != 0.0) as usize` rather than a branch: the compare is a
    /// single instruction and the add is unconditional, so the count adds
    /// no branch misprediction to a loop whose scatter already dominates it.
    #[inline]
    fn permute_out(&self, scratch: &[f64], out: &mut [f64]) -> usize {
        let mut nnz = 0usize;
        for s in 0..self.base.m {
            let v = scratch[s];
            out[self.base.col_perm[s]] = v;
            nnz += (v != 0.0) as usize;
        }
        nnz
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
    pub fn solve_sparse_into(&self, rhs_sparse: &[(usize, f64)], scratch: &mut [f64], gp: &mut GpScratch, out: &mut [f64]) -> usize {
        self.base.l_solve_sparse_into(rhs_sparse, scratch, gp);
        // CLOCK-trigger accounting (`Self::tick`'s own docs): unlike the
        // dense `L`-stage in `ftran_through_l_and_r_into` (a flat `m`),
        // this GP-sparse path's own real cost is its reach-set size.
        self.add_tick(gp.reach.len() as u64);
        for reta in &self.r_etas {
            let dot = reta.r.dot_dense(scratch);
            self.add_tick(reta.r.nnz() as u64);
            scratch[reta.p] -= dot;
        }
        // `U` stays on the plain `u_solve_into` scan — see that function's
        // own docs for the *two* separate attempts at a reach-restricted
        // counterpart (one ungated, one gated exactly the way HiGHS gates
        // its own `ftranU`) that were both implemented, proven correct,
        // measured over the full Netlib set, and reverted as regressions.
        self.u_solve_into(scratch);
        let nnz = self.permute_out(scratch, out);
        scratch.fill(0.0);
        nnz
    }

    /// Same as [`Self::solve_sparse_into`], but additionally captures the
    /// post-`L`/`R`, pre-`U` intermediate into `a_tilde_out` (length `m`) —
    /// see [`Self::solve_into_capture`]'s own docs, which this mirrors for
    /// the sparse-`rhs` FTRAN path. The capture happens after the `R`-eta
    /// loop (this stage's own last write to `scratch` before `u_solve_into`
    /// takes over), so `a_tilde_out` ends up identical regardless of which
    /// of the two FTRAN paths (`should_use_dense_solve`'s dense/sparse
    /// dispatch) a given call took.
    /// [`Self::solve_sparse_into`], but charging the synthetic clock exactly
    /// what [`Self::solve_into`] on the same right-hand side would have
    /// charged (a flat `m` for the `L` stage instead of the reach-set size).
    ///
    /// For call sites that used to go through the dense path
    /// unconditionally (the DSE `tau = B^-1 rho_p` solve): the result is
    /// bit-for-bit what the dense path returns — both visit the same
    /// nonzero steps of `L` in the same ascending order, the dense one just
    /// also skips over the zero ones — so the only thing that could change
    /// the pivot sequence is the CLOCK refactorization trigger reading a
    /// smaller tick. Keeping the charge identical keeps every
    /// refactorization at the same iteration, i.e. the change is pure cost.
    pub fn solve_sparse_into_dense_tick(&self, rhs_sparse: &[(usize, f64)], scratch: &mut [f64], gp: &mut GpScratch, out: &mut [f64]) -> usize {
        let nnz = self.solve_sparse_into(rhs_sparse, scratch, gp, out);
        self.add_tick((self.base.m as u64).saturating_sub(gp.reach.len() as u64));
        nnz
    }

    /// Charges the synthetic clock what [`Self::solve_into`]
    /// (`dense_l == true`) or [`Self::solve_sparse_into`] (`false`) would
    /// charge on an **all-zero** right-hand side, without doing the solve
    /// (whose result is known to be zero). Lets a caller skip a provably
    /// zero FTRAN while keeping the CLOCK trigger's schedule unchanged.
    pub fn charge_zero_rhs_solve(&self, dense_l: bool) {
        let m = self.base.m as u64;
        if dense_l {
            self.add_tick(m);
        }
        for reta in &self.r_etas {
            self.add_tick(reta.r.nnz() as u64);
        }
        self.add_tick(m);
    }

    pub fn solve_sparse_into_capture(
        &self,
        rhs_sparse: &[(usize, f64)],
        scratch: &mut [f64],
        gp: &mut GpScratch,
        out: &mut [f64],
        a_tilde_out: &mut [f64],
    ) -> usize {
        self.base.l_solve_sparse_into(rhs_sparse, scratch, gp);
        self.add_tick(gp.reach.len() as u64);
        for reta in &self.r_etas {
            let dot = reta.r.dot_dense(scratch);
            self.add_tick(reta.r.nnz() as u64);
            scratch[reta.p] -= dot;
        }
        a_tilde_out.copy_from_slice(scratch);
        self.u_solve_into(scratch);
        let nnz = self.permute_out(scratch, out);
        scratch.fill(0.0);
        nnz
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
        self.permute_transpose_rhs(rhs, scratch);
        self.u_transpose_solve_into(scratch);
        self.btran_tail(scratch, out);
    }

    /// `P_col^{-1} rhs` into `scratch` — an `O(m)` gather through the
    /// column permutation, every BTRAN's first step.
    #[inline]
    fn permute_transpose_rhs(&self, rhs: &[f64], scratch: &mut [f64]) {
        for s in 0..self.base.m {
            scratch[s] = rhs[self.base.col_perm[s]];
        }
    }

    /// [`Self::permute_transpose_rhs`] for `rhs = e_i`, returning the one
    /// step it lands on. `P_col^{-1} e_i` is the unit vector at step
    /// `col_perm_inv[i]`, so the gather collapses to a flat `fill` plus a
    /// single store — no random-access read per step, and the caller never
    /// has to own (or keep re-zeroing) a length-`m` `e_i` buffer of its own.
    #[inline]
    fn seed_unit_rhs(&self, i: usize, scratch: &mut [f64]) -> usize {
        scratch.fill(0.0);
        let s0 = self.base.col_perm_inv[i];
        scratch[s0] = 1.0;
        s0
    }

    /// Everything a BTRAN does after `U^{-T}`: the `R` etas in reverse,
    /// then `L^{-T}` back into original row indexing.
    #[inline]
    fn btran_tail(&self, scratch: &mut [f64], out: &mut [f64]) {
        // Hyper-sparse: same skip as `u_solve_into`/`l_solve_into` — `yp`
        // is the only value each `r_eta`'s entries get multiplied by here.
        for reta in self.r_etas.iter().rev() {
            let yp = scratch[reta.p];
            if yp == 0.0 {
                continue;
            }
            self.add_tick(reta.r.nnz() as u64);
            reta.r.axpy_into_dense(-yp, scratch);
        }
        // CLOCK-trigger accounting (`Self::tick`'s own docs): `l_transpose_solve_into`
        // is a dense `O(m)` reverse scan regardless of fill (see that
        // method's own docs for why sparsifying it wasn't worth trying).
        self.add_tick(self.base.m as u64);
        self.l_transpose_solve_into(scratch, out);
    }

    /// [`Self::solve_transpose_into`] for `rhs = e_i`, the shape every
    /// pivotal-row BTRAN in this crate actually has — see
    /// [`Self::u_transpose_solve_seeded`] for what the specialization
    /// saves and why it is exact. Unlike
    /// [`Self::solve_transpose_unit_into`] this makes no assumption about
    /// `u_seq`'s ordering, so it is valid with Forrest-Tomlin updates
    /// applied; unlike [`Self::solve_transpose_into`] it needs no `e_i`
    /// buffer from the caller. `scratch` is fully overwritten on entry, so
    /// it carries no precondition (same as `solve_transpose_into`).
    pub fn solve_transpose_unit(&self, i: usize, scratch: &mut [f64], out: &mut [f64]) {
        let s0 = self.seed_unit_rhs(i, scratch);
        self.u_transpose_solve_seeded(scratch, s0);
        self.btran_tail(scratch, out);
    }

    /// [`Self::solve_transpose_unit`] plus [`Self::solve_transpose_into_capture`]'s
    /// own `e_tilde` capture.
    pub fn solve_transpose_unit_capture(&self, i: usize, scratch: &mut [f64], out: &mut [f64], e_tilde_out: &mut [f64]) {
        let s0 = self.seed_unit_rhs(i, scratch);
        self.u_transpose_solve_seeded(scratch, s0);
        e_tilde_out.copy_from_slice(scratch);
        self.btran_tail(scratch, out);
    }

    /// Same as [`Self::solve_transpose_into`], but additionally captures
    /// the post-`U^-T`, pre-`R`-reverse intermediate into `e_tilde_out`
    /// (length `m`) — exactly the `e_tilde` value
    /// [`Self::try_update_precomputed`] needs when `rhs` is the unit
    /// vector at the leaving row (original indexing) being BTRAN'd this
    /// same iteration for `rho_p`. See that method's own docs for why this
    /// capture lets the caller skip `try_update`'s own redundant
    /// re-derivation of the exact same value.
    ///
    /// **No production call site left**: every `e_tilde`-capturing BTRAN in
    /// this crate has a unit-vector right-hand side and goes through
    /// [`Self::solve_transpose_unit_capture`] instead. Kept, rather than
    /// deleted, because it is the general-`rhs` reference that
    /// specialization is *checked against* — `solve_transpose_unit_is_bit_identical_to_the_dense_unit_rhs_path`
    /// asserts the two agree entry for entry, and on the synthetic tick,
    /// both on a fresh factorization and after Forrest-Tomlin updates have
    /// reordered `u_seq`. Deleting it would delete the proof.
    #[allow(dead_code)]
    pub fn solve_transpose_into_capture(&self, rhs: &[f64], scratch: &mut [f64], out: &mut [f64], e_tilde_out: &mut [f64]) {
        self.permute_transpose_rhs(rhs, scratch);
        self.u_transpose_solve_into(scratch);
        e_tilde_out.copy_from_slice(scratch);
        self.btran_tail(scratch, out);
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
        // On a fresh factorization `u_seq` holds the non-singleton slots in
        // ascending slot order, so the `[0, s0)` prefix is `u_seq`'s
        // prefix of slots below `s0`; the singletons at or after `s0` get
        // the same `(z - 0) / pivot` the unsplit sweep gave them, first.
        for eta in &self.singles {
            let p = eta.slot;
            if p >= s0 {
                let y = eta.off_diag.dot_dense(scratch);
                scratch[p] = (scratch[p] - y) / eta.pivot;
            }
        }
        let start = self.u_seq.partition_point(|e| e.slot < s0);
        self.u_transpose_solve_from(scratch, start);
        self.l_transpose_solve_into(scratch, out);
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
        debug_assert_eq!(a_tilde.len(), m, "a_tilde must be the full dense column");
        debug_assert_eq!(e_tilde.len(), m, "e_tilde must be the full dense row");
        let p = self.base.col_perm_inv[basis_slot];

        let single_idx = self.singles_pos[p];
        let old_pivot = if single_idx != usize::MAX { self.singles[single_idx].pivot } else { self.u_seq[self.slot_pos[p]].pivot };

        // The `R` eta is built straight out of `e_tilde` — same entries,
        // same order, same sparse/dense choice as the `collect()`-then-
        // `HybridVec::pack` this replaces, minus that intermediate `Vec`
        // (see [`HybridVec::pack_scaled_dense`]'s own docs). It is built
        // *before* the pivot test because `dot` is exactly this eta
        // against `a_tilde`, so the test can read it off the eta rather
        // than needing a separate pass of its own; the previous code
        // likewise materialized the whole thing before testing, so a
        // rejected update is no more expensive than it already was.
        let r_eta = HybridVec::pack_scaled_dense(e_tilde, p, -old_pivot, DENSE_ETA_FRACTION);
        let dot = r_eta.dot_dense(a_tilde);
        let new_pivot = a_tilde[p] - dot;
        if new_pivot.abs() < min_pivot {
            return false;
        }

        // `Vec::remove` shifts every later element down by one position —
        // update `slot_pos` for exactly that range (elements the memmove
        // itself already touches, so this is no extra asymptotic cost)
        // rather than the old `find_seq_pos`'s full O(m) re-scan.
        let removed = if single_idx != usize::MAX {
            // A singleton leaves `singles` instead (order there is free);
            // `u_seq` is untouched until the push below.
            let removed = self.singles.swap_remove(single_idx);
            if let Some(moved) = self.singles.get(single_idx) {
                self.singles_pos[moved.slot] = single_idx;
            }
            self.singles_pos[p] = usize::MAX;
            removed
        } else {
            let seq_pos = self.slot_pos[p];
            let removed = self.u_seq.remove(seq_pos);
            for pos in seq_pos..self.u_seq.len() {
                self.slot_pos[self.u_seq[pos].slot] = pos;
            }
            removed
        };
        self.fill -= removed.off_diag.nnz();

        // Unregister slot `p`'s *old* off-diagonal entries from
        // `row_owners` before overwriting them below — otherwise a stale
        // `p` would linger in some other row's owner list, pointing at
        // content that no longer exists there.
        removed.off_diag.for_each_index(|row_step| {
            if let Some(idx) = self.row_owners[row_step].iter().position(|&(s, _)| s == p) {
                self.row_owners[row_step].swap_remove(idx);
            }
        });

        // Zero row `p` out of every eta that still references it (Tomlin
        // 1974, eq. 12) — only the etas `row_owners[p]` actually lists,
        // not every eta in `U` (see `row_owners`'s own docs), each found
        // in O(1) via `slot_pos`.
        for (slot, _) in std::mem::take(&mut self.row_owners[p]) {
            let pos = self.slot_pos[slot];
            if self.u_seq[pos].off_diag.remove_index(p) {
                self.fill -= 1;
            }
        }

        // Same replacement column as before, built directly from
        // `a_tilde` (scale `1.0`, so the dense arm is a plain copy) rather
        // than through a throwaway pair list.
        let off_diag = HybridVec::pack_scaled_dense(a_tilde, p, 1.0, DENSE_ETA_FRACTION);
        off_diag.for_each_entry(|row_step, v| self.row_owners[row_step].push((p, v)));
        self.fill += off_diag.nnz();
        self.u_seq.push(UEta { slot: p, pivot: new_pivot, off_diag });
        self.slot_pos[p] = self.u_seq.len() - 1;

        self.fill += r_eta.nnz();
        self.r_etas.push(REta { p, r: r_eta });

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
    /// Counts true nonzeros via [`HybridVec::nnz`], not storage length, so
    /// switching an eta to the dense representation doesn't spuriously
    /// inflate this and trip the trigger early.
    ///
    /// `O(1)`: maintained by [`Self::commit_update`] as it goes — see
    /// [`Self::fill`]'s own docs for why re-summing was worth removing.
    pub fn fill_count(&self) -> usize {
        debug_assert_eq!(
            self.fill,
            self.u_seq.iter().map(|e| e.off_diag.nnz()).sum::<usize>() + self.r_etas.iter().map(|e| e.r.nnz()).sum::<usize>(),
            "incrementally maintained fill drifted from the true eta-file fill"
        );
        self.fill
    }

    /// Debug/instrumentation only: off-diagonal nonzero count of the `U`
    /// eta most recently appended by `try_update` (0 if no update has
    /// happened yet) — the fill-in from a single update, as opposed to
    /// `fill_count`'s running total. Used by `simplex.rs`'s
    /// `ENOMOTO_DEBUG_ETA_DENSITY` diagnostic to measure how eta density
    /// is distributed across a real solve, which is what motivated
    /// `HybridVec`'s sparse/dense hybrid representation.
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
        assert_eq!(lu.l_col.nnz(), 0, "L must be identity: {:?}", lu.l_col.to_cols());
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

    /// `l_row` really is `l_col`'s transpose: every stored entry of one
    /// appears exactly once, with the same value, at the mirrored index of
    /// the other. Checked on a factorization with genuine fill (a plain
    /// diagonal basis would pass vacuously with both structures empty).
    #[test]
    fn l_row_is_the_exact_transpose_of_l_col() {
        for m in [3usize, 12, 40] {
            for seed in [1u64, 7, 99] {
                let rows = random_sparse_diag_dominant(m, seed);
                let lu = factorize(m, &rows).expect("nonsingular");
                let mut from_col: Vec<(usize, usize, u64)> = Vec::new();
                for s in 0..m {
                    for &(row_step, mult) in lu.l_col.col(s) {
                        assert!(row_step > s, "L must be strictly lower triangular in step space");
                        from_col.push((s, row_step, mult.to_bits()));
                    }
                }
                let mut from_row: Vec<(usize, usize, u64)> = Vec::new();
                for r in 0..m {
                    for &(s, mult) in lu.l_row.row(r) {
                        assert!(s < r, "l_row.row(r) must only hold entries at s < r");
                        from_row.push((s, r, mult.to_bits()));
                    }
                }
                from_col.sort_unstable();
                from_row.sort_unstable();
                assert_eq!(from_col, from_row, "l_row is not l_col's transpose (m={m}, seed={seed})");
            }
        }
    }

    /// The scatter form of `L^{-T}` (via `l_row`) and the original gather
    /// form (via `l_col`) solve the same system. Not bit-identical by
    /// construction (the same products are summed in the opposite order —
    /// see `l_transpose_solve_scatter_into`'s own docs), so this checks
    /// agreement to a tight relative tolerance rather than exact equality,
    /// on right-hand sides ranging from a single nonzero (the hyper-sparse
    /// case the scatter form exists for) to fully dense.
    #[test]
    fn l_transpose_scatter_matches_gather() {
        for m in [3usize, 12, 40] {
            for seed in [1u64, 7, 99] {
                let rows = random_sparse_diag_dominant(m, seed);
                let lu = factorize(m, &rows).expect("nonsingular");
                let mut rng = seed ^ 0xabcd;
                let mut rhss: Vec<Vec<f64>> = vec![(0..m).map(|_| next_rand(&mut rng)).collect()];
                for unit in [0usize, m / 2, m - 1] {
                    let mut e = vec![0.0; m];
                    e[unit] = 1.0;
                    rhss.push(e);
                }
                for rhs in rhss {
                    let (mut wg, mut yg) = (rhs.clone(), vec![0.0; m]);
                    lu.l_transpose_solve_gather_into(&mut wg, &mut yg);
                    let (mut ws, mut ys) = (rhs.clone(), vec![0.0; m]);
                    lu.l_transpose_solve_scatter_into(&mut ws, &mut ys);
                    for i in 0..m {
                        let scale = yg[i].abs().max(1.0);
                        assert!(
                            (yg[i] - ys[i]).abs() <= 1e-12 * scale,
                            "scatter/gather mismatch at {i}: {} vs {} (m={m}, seed={seed})",
                            yg[i],
                            ys[i]
                        );
                    }
                }
            }
        }
    }

    /// End-to-end: a full `B^-T rhs` BTRAN agrees between the two arms of
    /// the `ENOMOTO_BTRAN_L_SCATTER` dispatch, *after* Forrest-Tomlin
    /// updates have put `R`-etas in front of the `L^{-T}` stage (the state
    /// every per-iteration BTRAN actually runs in, and the one where a
    /// wrong transpose would show up as a wrong `rho_p` rather than a
    /// merely differently-rounded one).
    #[test]
    fn btran_agrees_between_scatter_and_gather_after_ft_updates() {
        let m = 40;
        for seed in [3u64, 11] {
            let rows = random_sparse_diag_dominant(m, seed);
            let mut scatter = FtLu::new(factorize(m, &rows).expect("nonsingular"));
            scatter.btran_l_scatter = 1.0;
            let mut gather = FtLu::new(factorize(m, &rows).expect("nonsingular"));
            gather.btran_l_scatter = 0.0;
            let mut rng = seed ^ 0x5eed;
            for slot in [2usize, 9, 25] {
                let a_q: Vec<f64> = (0..m).map(|i| if i % 3 == 0 { next_rand(&mut rng) } else { 0.0 } + if i == slot { 4.0 } else { 0.0 }).collect();
                assert!(scatter.try_update(slot, &a_q, 1e-9));
                assert!(gather.try_update(slot, &a_q, 1e-9));
            }
            for probe in [0usize, 7, 39] {
                let mut rhs = vec![0.0; m];
                rhs[probe] = 1.0;
                let ys = scatter.solve_transpose(&rhs);
                let yg = gather.solve_transpose(&rhs);
                for i in 0..m {
                    let scale = yg[i].abs().max(1.0);
                    assert!(
                        (yg[i] - ys[i]).abs() <= 1e-9 * scale,
                        "BTRAN mismatch at {i}: {} vs {} (seed={seed}, probe={probe})",
                        yg[i],
                        ys[i]
                    );
                }
            }
        }
    }

    /// Residual `‖A x - b‖_inf` against the sparse rows `A` — what a
    /// factorization is actually *for*, and therefore a stronger check on
    /// a reused order than comparing its `L`/`U` against a Markowitz
    /// run's (the two legitimately differ: same matrix, two valid orders).
    fn residual_inf(rows: &[Vec<(usize, f64)>], x: &[f64], b: &[f64]) -> f64 {
        rows.iter()
            .enumerate()
            .map(|(i, row)| (row.iter().map(|&(j, v)| v * x[j]).sum::<f64>() - b[i]).abs())
            .fold(0.0, f64::max)
    }

    /// Replaces `n` columns of `rows` with fresh content whose nonzeros
    /// sit in *different rows* than the column they replace — what a
    /// Forrest-Tomlin update does to a basis, and specifically the case
    /// that makes replaying the recorded *row* order impossible (see
    /// `factorize_reusing_order`'s own docs).
    fn replace_columns(rows: &mut [Vec<(usize, f64)>], n: usize, seed: u64) {
        let m = rows.len();
        let mut state = seed;
        for k in 0..n {
            let j = (k * 7 + 3) % m;
            for row in rows.iter_mut() {
                row.retain(|&(c, _)| c != j);
            }
            let i0 = (k * 11 + 5) % m;
            rows[i0].push((j, 6.0 + next_rand(&mut state).abs()));
            let i1 = (k * 13 + 1) % m;
            if i1 != i0 {
                rows[i1].push((j, next_rand(&mut state)));
            }
        }
    }

    #[test]
    fn reused_order_solves_a_basis_whose_columns_were_replaced() {
        for seed in [1u64, 7, 99] {
            let m = 60;
            let rows = random_sparse_diag_dominant(m, seed);
            let first = factorize(m, &rows).expect("well-conditioned matrix factorizes");

            let mut rows2 = rows.clone();
            replace_columns(&mut rows2, 5, seed);

            let reused = factorize_reusing_order(m, &rows2, &first.col_perm, &first.row_perm, usize::MAX)
                .expect("row re-picking keeps the recorded column order usable");

            let b: Vec<f64> = (0..m).map(|i| 1.0 + (i % 5) as f64).collect();
            let x = reused.solve(&b);
            assert!(residual_inf(&rows2, &x, &b) < 1e-9, "seed {seed}: reused order solves the updated matrix");

            // And `B^T y = b` through the same factors, since the transpose
            // path reads `l_col`/`u_row` in the other direction (a wrong
            // permutation would pass one and fail the other).
            let y = reused.solve_transpose(&b);
            let mut rows_t: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
            for (i, row) in rows2.iter().enumerate() {
                for &(j, v) in row {
                    rows_t[j].push((i, v));
                }
            }
            assert!(residual_inf(&rows_t, &y, &b) < 1e-9, "seed {seed}: reused order solves the transpose");

            // `l_row` must stay `l_col`'s exact transpose on this path too
            // (BTRAN's scatter form reads it).
            let mut from_col: Vec<(usize, usize, u64)> = Vec::new();
            for s in 0..m {
                for &(row_step, mult) in reused.l_col.col(s) {
                    assert!(row_step > s, "L must be strictly lower triangular in step space");
                    from_col.push((s, row_step, mult.to_bits()));
                }
            }
            let mut from_row: Vec<(usize, usize, u64)> = Vec::new();
            for r in 0..m {
                for &(s, mult) in reused.l_row.row(r) {
                    from_row.push((s, r, mult.to_bits()));
                }
            }
            from_col.sort_unstable();
            from_row.sort_unstable();
            assert_eq!(from_col, from_row, "seed {seed}: l_row is not l_col's transpose");
        }
    }

    #[test]
    fn reused_order_reproduces_a_fresh_factorization_of_the_same_matrix() {
        let m = 50;
        let rows = random_sparse_diag_dominant(m, 2024);
        let first = factorize(m, &rows).expect("factorizes");
        let again = factorize_reusing_order(m, &rows, &first.col_perm, &first.row_perm, usize::MAX)
            .expect("its own order is trivially acceptable for the same matrix");

        // Same matrix, same order, so the factors themselves must match —
        // not merely solve alike.
        assert_eq!(again.row_perm, first.row_perm);
        assert_eq!(again.col_perm, first.col_perm);
        for s in 0..m {
            let mut a: Vec<(usize, f64)> = first.l_col.col(s).to_vec();
            let mut b: Vec<(usize, f64)> = again.l_col.col(s).iter().copied().filter(|&(_, v)| v != 0.0).collect();
            a.sort_unstable_by_key(|&(r, _)| r);
            b.sort_unstable_by_key(|&(r, _)| r);
            assert_eq!(a.len(), b.len(), "L column {s} nonzero count");
            for (&(ra, va), &(rb, vb)) in a.iter().zip(b.iter()) {
                assert_eq!(ra, rb);
                assert!((va - vb).abs() < 1e-12, "L[{ra}][{s}]: {va} vs {vb}");
            }
        }
    }

    #[test]
    fn reused_order_rejects_a_singular_matrix() {
        let identity_order: Vec<usize> = vec![0, 1];
        let singular = vec![vec![(0usize, 1.0f64), (1usize, 1.0f64)], vec![(0usize, 1.0f64), (1usize, 1.0f64)]];
        assert!(
            factorize_reusing_order(2, &singular, &identity_order, &identity_order, usize::MAX).is_none(),
            "a column with nothing left in the remaining submatrix must reject the order"
        );
    }

    #[test]
    fn reused_order_repicks_the_row_when_the_recorded_one_is_numerically_weak() {
        // Recorded order says step 0 pivots on row 0 of column 0, but in
        // *this* matrix that entry is numerically nothing against the same
        // column's other entry: the row must be re-picked (to row 1),
        // rather than the whole order rejected.
        let order: Vec<usize> = vec![0, 1];
        let weak = vec![vec![(0usize, 1e-14f64), (1usize, 1.0f64)], vec![(0usize, 1.0f64), (1usize, 1.0f64)]];
        let lu = factorize_reusing_order(2, &weak, &order, &order, usize::MAX)
            .expect("a weak recorded row is re-picked, not a rejection");
        assert_eq!(lu.row_perm[0], 1, "step 0 must pivot on the numerically sound row");
        let b = vec![1.0, 2.0];
        let x = lu.solve(&b);
        assert!(residual_inf(&weak, &x, &b) < 1e-9, "re-picked pivot still solves: {x:?}");
    }

    #[test]
    fn reused_order_respects_the_fill_limit() {
        let m = 40;
        let rows = random_sparse_diag_dominant(m, 31337);
        let first = factorize(m, &rows).expect("factorizes");
        // `max_nnz` below even the diagonal alone: no factorization of
        // anything can fit, so the guard must fire rather than return
        // factors that exceed it.
        assert!(factorize_reusing_order(m, &rows, &first.col_perm, &first.row_perm, 1).is_none());
    }

    #[test]
    fn factorize_reusing_matches_a_from_scratch_factorization_through_column_replacements() {
        // End-to-end: build a factorization, replace basis columns the way
        // the simplex loop does, refactorize with reuse, and check the
        // result against a from-scratch factorization of the same matrix.
        let m = 45;
        let mut rows = random_sparse_diag_dominant(m, 555);
        let mut lu = FtLu::new(factorize(m, &rows).expect("factorizes"));
        let b: Vec<f64> = (0..m).map(|i| 0.5 + (i % 7) as f64).collect();

        for round in 0..4 {
            replace_columns(&mut rows, 3, 900 + round);
            lu = factorize_reusing(m, &rows, Some(&lu)).expect("refactorizes");

            let mut scratch = vec![0.0; m];
            let mut out = vec![0.0; m];
            lu.solve_into(&b, &mut scratch, &mut out);
            assert!(residual_inf(&rows, &out, &b) < 1e-9, "round {round}: FTRAN residual");

            let fresh = FtLu::new(factorize(m, &rows).expect("factorizes"));
            let mut fresh_out = vec![0.0; m];
            fresh.solve_into(&b, &mut scratch, &mut fresh_out);
            for i in 0..m {
                assert!((out[i] - fresh_out[i]).abs() < 1e-9, "round {round}, row {i}: reuse vs fresh");
            }

            // BTRAN too, through both of `l_transpose_solve_into`'s arms.
            let ours = lu.solve_transpose(&b);
            let theirs = fresh.solve_transpose(&b);
            for i in 0..m {
                assert!((ours[i] - theirs[i]).abs() < 1e-9, "round {round}, row {i}: BTRAN reuse vs fresh");
            }
        }
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

    #[test]
    fn solve_transpose_unit_is_bit_identical_to_the_dense_unit_rhs_path() {
        let m = 40;
        for seed in [1u64, 2, 3, 4, 5] {
            let rows = random_sparse_diag_dominant(m, seed);
            let base = factorize(m, &rows).expect("diagonally dominant matrix must factorize");
            let mut state = FtLu::new(base);

            // Once fresh, and again after Forrest-Tomlin updates have
            // reordered `u_seq` — the case `solve_transpose_unit_into`'s
            // own prefix-skip is *not* valid for, and the whole reason
            // this seeded variant exists alongside it.
            for round in 0..3 {
                let (mut scratch, mut out, mut e_tilde) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                let (mut rscratch, mut rout, mut re_tilde) = (vec![0.0; m], vec![0.0; m], vec![0.0; m]);
                for i in 0..m {
                    let mut e_i = vec![0.0; m];
                    e_i[i] = 1.0;
                    state.solve_transpose_into_capture(&e_i, &mut rscratch, &mut rout, &mut re_tilde);

                    state.solve_transpose_unit_capture(i, &mut scratch, &mut out, &mut e_tilde);
                    assert_eq!(out, rout, "seed={seed} round={round} i={i}: unit BTRAN diverged from the dense-rhs one");
                    assert_eq!(e_tilde, re_tilde, "seed={seed} round={round} i={i}: captured e_tilde diverged");

                    // The non-capturing form must agree with both.
                    let mut out2 = vec![0.0; m];
                    state.solve_transpose_unit(i, &mut scratch, &mut out2);
                    assert_eq!(out2, rout, "seed={seed} round={round} i={i}: solve_transpose_unit diverged");
                }
                // The tick is what drives the deterministic CLOCK
                // refactorization trigger, so the two paths must charge
                // identically or the solve would take a different
                // trajectory (see `u_transpose_sweep`'s own note).
                let before = state.synth_tick();
                let mut s1 = vec![0.0; m];
                let mut o1 = vec![0.0; m];
                let mut e1 = vec![0.0; m];
                state.solve_transpose_unit_capture(0, &mut s1, &mut o1, &mut e1);
                let unit_cost = state.synth_tick() - before;
                let before = state.synth_tick();
                let mut e_0 = vec![0.0; m];
                e_0[0] = 1.0;
                state.solve_transpose_into_capture(&e_0, &mut s1, &mut o1, &mut e1);
                assert_eq!(state.synth_tick() - before, unit_cost, "seed={seed} round={round}: unit and dense BTRAN must charge the same tick");

                let a_q: Vec<f64> = (0..m).map(|k| if k % 7 == round { 1.0 + k as f64 } else { 0.0 }).collect();
                if !state.try_update(round, &a_q, 1e-9) {
                    break;
                }
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

    /// The result-density gate (`docs/lu_comparison_enomoto_vs_highs.md`
    /// §2.7): a channel whose *results* keep coming back dense must end up
    /// on the dense path even when every right-hand side it is handed is
    /// sparse enough for `should_use_dense_solve` alone to say otherwise —
    /// and must find its way back to the sparse path once the results turn
    /// sparse again, since the average is recorded on both branches.
    #[test]
    fn density_gate_flips_a_sparse_rhs_channel_dense_and_back() {
        let m = 10;
        let rows: Vec<Vec<(usize, f64)>> = (0..m).map(|i| vec![(i, 4.0)]).collect();
        let lu = FtLu::new(factorize(m, &rows).expect("nonsingular"));
        let mut density = FtranDensity::new();
        // Untouched history: dispatch is exactly `should_use_dense_solve`'s.
        assert!(!lu.should_use_dense_solve_tracked(2, &density), "a fresh channel must not be gated dense");

        // Fully dense results, iteration after iteration: the running
        // average climbs past EXPECTED_DENSE_FRACTION (0.35) and the same
        // sparse rhs now dispatches dense.
        for _ in 0..30 {
            density.record(m, m);
        }
        assert!(density.expected() > EXPECTED_DENSE_FRACTION, "expected={}", density.expected());
        assert!(lu.should_use_dense_solve_tracked(2, &density), "a persistently dense channel must be gated dense");

        // ...and back: nothing latches, because the dense branch records
        // its own result density too.
        for _ in 0..60 {
            density.record(0, m);
        }
        assert!(!lu.should_use_dense_solve_tracked(2, &density), "expected={}", density.expected());
    }

    /// Every FTRAN entry point reports the nonzero count of the result it
    /// just wrote — the measurement [`FtranDensity::record`] is fed — and
    /// the dense and sparse paths agree on it, since they compute the
    /// identical vector.
    #[test]
    fn solve_paths_report_the_result_nonzero_count() {
        let m = 6;
        // Lower-bidiagonal `B`: `B^-1 e_0` fills in over *every* row, so a
        // one-nonzero rhs has a fully dense result — exactly the case the
        // input-side test alone cannot see coming.
        let rows: Vec<Vec<(usize, f64)>> =
            (0..m).map(|i| if i == 0 { vec![(0, 2.0)] } else { vec![(i - 1, -2.0), (i, 2.0)] }).collect();
        let lu = FtLu::new(factorize(m, &rows).expect("nonsingular"));

        let mut dense_rhs = vec![0.0; m];
        dense_rhs[0] = 1.0;
        let mut scratch = vec![0.0; m];
        let mut out_dense = vec![0.0; m];
        let dense_nnz = lu.solve_into(&dense_rhs, &mut scratch, &mut out_dense);

        let mut sparse_scratch = vec![0.0; m];
        let mut gp = GpScratch::new(m);
        let mut out_sparse = vec![0.0; m];
        let sparse_nnz = lu.solve_sparse_into(&[(0, 1.0)], &mut sparse_scratch, &mut gp, &mut out_sparse);

        assert_eq!(out_dense, out_sparse, "the two FTRAN paths must agree on the result itself");
        assert_eq!(dense_nnz, sparse_nnz, "...and on its nonzero count");
        assert_eq!(dense_nnz, out_dense.iter().filter(|&&v| v != 0.0).count());
        assert_eq!(dense_nnz, m, "this fixture's whole point is a dense result from a one-nonzero rhs");
    }

    /// A dense-coefficient basis (every column of `B` has all `m` entries,
    /// well past `DENSE_ETA_FRACTION` after a couple of FT updates) run
    /// through several `try_update` calls with equally dense entering
    /// columns, then cross-checked against a completely independent full
    /// refactorization of the final basis — the same style of ground truth
    /// the sparse-fixture tests above use, just sized and shaped to
    /// actually exercise `HybridVec`'s dense arm instead of its sparse one.
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
