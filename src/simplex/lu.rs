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

use std::collections::{HashMap, HashSet, VecDeque};

const STABILITY: f64 = 0.1;

/// Manages the active submatrix plus row/column degrees (via bucket
/// arrays) during Markowitz elimination — see the module docs for why a
/// column-major `col_rows` mirror alongside the row-major `rows` matrix is
/// what actually keeps this sub-`O(m)` per step, and why the elimination
/// and degree bookkeeping must happen in one place rather than two.
struct MarkowitzState {
    #[allow(dead_code)]
    m: usize,

    // Active row-major submatrix.
    rows: Vec<HashMap<usize, f64>>,
    // Column-major mirror: col_rows[j] = the set of currently-active rows
    // with a nonzero at column j. Kept in exact sync with `rows` by every
    // method below — this is what lets column-degree lookups and
    // "who else has a nonzero here" queries stay O(that column's own
    // degree) instead of O(m).
    col_rows: Vec<HashSet<usize>>,

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
}

impl MarkowitzState {
    fn new(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Self {
        let rows: Vec<HashMap<usize, f64>> = rows_in
            .iter()
            .map(|r| {
                let mut h = HashMap::new();
                for &(j, v) in r {
                    *h.entry(j).or_insert(0.0) += v;
                }
                h.retain(|_, v| *v != 0.0);
                h
            })
            .collect();

        let mut col_rows: Vec<HashSet<usize>> = vec![HashSet::new(); m];
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
    fn find_best_pivot(&self) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize)> = None;
        let mut best_score = usize::MAX;
        let mut best_pivot_abs = 0.0f64;

        for deg_col in 1..self.col_buckets.len() {
            for &j in &self.col_buckets[deg_col] {
                if self.col_used[j] {
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
                    return best;
                }
            }
            if best.is_some() && best_score <= deg_col * deg_col {
                return best;
            }
        }

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
        let mut touched_cols: HashSet<usize> = HashSet::new();
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
                let existed = self.rows[i].contains_key(&j);
                let new_val = self.rows[i].get(&j).copied().unwrap_or(0.0) - mult * v;
                if new_val == 0.0 {
                    if existed {
                        self.rows[i].remove(&j);
                        self.col_rows[j].remove(&i);
                        touched_cols.insert(j);
                    }
                } else {
                    self.rows[i].insert(j, new_val);
                    if !existed {
                        self.col_rows[j].insert(i);
                    }
                    touched_cols.insert(j);
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
}

/// Factorizes the `m x m` sparse matrix given as sparse rows
/// `(col, value)`. Returns `None` if the matrix is (numerically)
/// singular — no acceptable pivot remains at some step.
pub fn factorize(m: usize, rows_in: &[Vec<(usize, f64)>]) -> Option<LuFactors> {
    let mut state = MarkowitzState::new(m, rows_in);

    let mut row_perm = vec![0usize; m];
    let mut col_perm = vec![0usize; m];

    let mut l_entries: Vec<(usize, usize, f64)> = Vec::new();
    let mut u_entries: Vec<(usize, usize, f64)> = Vec::new();

    for step in 0..m {
        let (pi, pj) = state.find_best_pivot()?;

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

    Some(LuFactors { m, l_col, u_row, row_perm, col_perm, col_perm_inv })
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
    fn l_solve(&self, rhs: &[f64]) -> Vec<f64> {
        let m = self.m;
        let mut z: Vec<f64> = (0..m).map(|s| rhs[self.row_perm[s]]).collect();
        for s in 0..m {
            if z[s] == 0.0 {
                continue;
            }
            for &(row_step, mult) in &self.l_col[s] {
                z[row_step] -= mult * z[s];
            }
        }
        z
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
    /// and free, just a smaller win than `l_solve`'s whole-step skip. (A
    /// fuller DFS-based hyper-sparse implementation for this direction,
    /// mirroring `L`/`U` both column- and row-major the way HiGHS does,
    /// was tried and measured *slower* end to end on this crate's
    /// benchmark: the DFS setup's own per-call cost — allocating a fresh
    /// `visited` array plus an upfront `O(m)` density scan on every single
    /// `l_solve`/`l_transpose_solve`/`u_solve`/`u_transpose_solve` call,
    /// even ones that end up taking the dense-style branch — outweighed
    /// the fill-skipping it bought, on the order of 15-18% slower overall
    /// despite the hyper-sparse branch firing on a majority of calls.
    /// Reverted; see this file's own history if revisiting this.)
    #[allow(dead_code)]
    fn l_transpose_solve(&self, z: Vec<f64>) -> Vec<f64> {
        let m = self.m;
        let mut w = z;
        for s in (0..m).rev() {
            for &(row_step, mult) in &self.l_col[s] {
                if w[row_step] == 0.0 {
                    continue;
                }
                w[s] -= mult * w[row_step];
            }
        }
        let mut y = vec![0.0; m];
        for s in 0..m {
            y[self.row_perm[s]] = w[s];
        }
        y
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
        let u_seq = (0..m)
            .map(|slot| UEta { slot, pivot: pivots[slot], off_diag: std::mem::take(&mut off_diags[slot]) })
            .collect();
        FtLu { base, u_seq, r_etas: Vec::new() }
    }

    fn find_seq_pos(&self, slot: usize) -> usize {
        self.u_seq.iter().position(|e| e.slot == slot).expect("slot must be present in the U sequence")
    }

    /// `U_k^{-T}` applied to a step-space vector: processes the eta
    /// sequence in **forward** (creation) order, each step solving for
    /// that eta's pivotal component via eq. (8).
    fn u_transpose_solve(&self, rhs: &[f64]) -> Vec<f64> {
        let mut z = rhs.to_vec();
        for eta in &self.u_seq {
            let p = eta.slot;
            let y: f64 = eta.off_diag.iter().map(|&(row_step, v)| v * z[row_step]).sum();
            z[p] = (z[p] - y) / eta.pivot;
        }
        z
    }

    /// `U_k^{-1}` applied to a step-space vector: processes the eta
    /// sequence in **reverse** order, each step solving via eq. (7).
    /// Hyper-sparse: same skip as `LuFactors::l_solve` — `xp` is the only
    /// value this eta's off-diagonal entries get multiplied by, so a zero
    /// `xp` makes the whole inner loop a provable no-op.
    fn u_solve(&self, rhs: &[f64]) -> Vec<f64> {
        let mut x = rhs.to_vec();
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
        x
    }

    /// `R_k^{-1} ... R_1^{-1} L^{-1}` applied to a vector in original row
    /// indexing, giving a step-space result — i.e. everything `solve`
    /// does except the final `U_k^{-1}`. This is also exactly what a new
    /// update needs to turn `a_q` into `ã_q`: per eq. (11), `ã_q` must be
    /// `(L R_1 ... R_{k-1})^{-1} a_q`, *not* just `L^{-1} a_q` — the
    /// existing `R`s are already part of the "L-like" fixed factor that
    /// update `k` treats as known, since `B_{k-1} = L R_1 ... R_{k-1}
    /// U_{k-1}` (eq. 13) rather than `B_{k-1} = L U_{k-1}` once `k > 1`.
    fn ftran_through_l_and_r(&self, rhs: &[f64]) -> Vec<f64> {
        let mut z = self.base.l_solve(rhs);
        for reta in &self.r_etas {
            let dot: f64 = reta.r.iter().map(|&(i, v)| v * z[i]).sum();
            z[reta.p] -= dot;
        }
        z
    }

    pub fn solve(&self, rhs: &[f64]) -> Vec<f64> {
        let z = self.ftran_through_l_and_r(rhs);
        let step_x = self.u_solve(&z);
        let mut x = vec![0.0; self.base.m];
        for s in 0..self.base.m {
            x[self.base.col_perm[s]] = step_x[s];
        }
        x
    }

    pub fn solve_transpose(&self, rhs: &[f64]) -> Vec<f64> {
        let m = self.base.m;
        let z: Vec<f64> = (0..m).map(|s| rhs[self.base.col_perm[s]]).collect();
        let mut z = self.u_transpose_solve(&z);
        // Hyper-sparse: same skip as `u_solve`/`l_solve` — `yp` is the
        // only value each `r_eta`'s entries get multiplied by here.
        for reta in self.r_etas.iter().rev() {
            let yp = z[reta.p];
            if yp == 0.0 {
                continue;
            }
            for &(i, v) in &reta.r {
                z[i] -= v * yp;
            }
        }
        self.base.l_transpose_solve(z)
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
    pub fn try_update(&mut self, basis_slot: usize, a_q_original: &[f64], min_pivot: f64) -> bool {
        let m = self.base.m;
        let p = self.base.col_perm_inv[basis_slot];

        let a_tilde = self.ftran_through_l_and_r(a_q_original);

        let mut e_p = vec![0.0; m];
        e_p[p] = 1.0;
        let e_tilde = self.u_transpose_solve(&e_p);

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

}
