//! From-scratch sparse LU factorization of a (square) basis matrix, using
//! **Markowitz pivoting**: among numerically-acceptable pivot candidates
//! (`|a_ij| >= stability * max(|a_i'j|)` over the still-active rows i' of
//! column j — the usual "threshold pivoting" stability floor), the one
//! minimizing the Markowitz count `(row_nnz - 1) * (col_nnz - 1)` is
//! chosen, i.e. sparsity (fill-in) is prioritized over picking the
//! numerically largest entry, subject to that stability floor.
//!
//! This produces `P_row B P_col = L U` (`L` unit lower triangular, `U`
//! upper triangular, both stored in *elimination-step* order — step `s`'s
//! pivot row/column are `row_perm[s]`/`col_perm[s]` in the original basis
//! matrix's indexing).
//!
//! The active submatrix during elimination is kept as one `HashMap` per
//! row (rebuilding row/column nonzero counts by scanning on every pivot
//! step); this is simpler and easier to get right than the doubly-linked
//! row/column lists a production implementation would use, at the cost of
//! being `O(m * nnz)` per factorization rather than near-linear — an
//! acceptable MVP trade-off given this module also implements incremental
//! Forrest-Tomlin updates (`ft_update`) specifically so that a full
//! Markowitz refactorization is *not* needed on every basis change.
//!
//! Within that `O(m * nnz)` shape, three constant-factor costs turned out
//! to matter in practice (profiling on this crate's target problem sizes
//! showed even a *trivial* (identity-matrix) factorization costing over a
//! millisecond): column and row nonzero counts were each recomputed by a
//! *separate* full scan of the active submatrix every step (row counts a
//! second time, via a `.filter().count()` inside the pivot search that
//! re-did exactly what the column-stats scan had just done); the
//! column-stats buffers were freshly heap-allocated (`vec![0; m]`) every
//! step instead of being cleared and reused; and the elimination step
//! walked every row index `0..m` unconditionally (skipping used ones)
//! rather than only the rows still actually active. None of these change
//! *which* pivot gets chosen (same Markowitz-count-then-magnitude rule,
//! same stability floor) — they only remove redundant scanning,
//! reallocation, and dead iterations, so the resulting factors (and every
//! downstream `solve`/`solve_transpose`/FT-update result) are unchanged.

use std::collections::HashMap;

const STABILITY: f64 = 0.1;

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
    let mut rows: Vec<HashMap<usize, f64>> = rows_in
        .iter()
        .map(|r| {
            let mut h = HashMap::new();
            for &(j, v) in r {
                if v != 0.0 {
                    *h.entry(j).or_insert(0.0) += v;
                }
            }
            h
        })
        .collect();

    let mut row_used = vec![false; m];
    let mut col_used = vec![false; m];
    let mut row_perm = vec![0usize; m];
    let mut col_perm = vec![0usize; m];

    let mut l_entries: Vec<(usize, usize, f64)> = Vec::new(); // (orig_row, pivot_step, multiplier)
    let mut u_entries: Vec<(usize, usize, f64)> = Vec::new(); // (pivot_step, orig_col, value)

    // Reused across every step instead of freshly heap-allocated each
    // time — `.fill()` below is a plain memset-like sweep, no allocator
    // call.
    let mut col_count = vec![0usize; m];
    let mut col_max_abs = vec![0.0f64; m];
    let mut row_count = vec![0usize; m];
    // Still-active row indices, swap-removed as rows are consumed, so
    // every loop below only ever visits rows that are actually still in
    // play instead of walking `0..m` and skipping used ones every step.
    // `row_pos[i]` tracks row `i`'s current slot in `active_rows` so that
    // removal is O(1) (swap with the last slot, then fix up whichever row
    // got moved into `i`'s old spot) rather than an O(active rows) linear
    // search for `i` every step.
    let mut active_rows: Vec<usize> = (0..m).collect();
    let mut row_pos: Vec<usize> = (0..m).collect();

    for step in 0..m {
        col_count.fill(0);
        col_max_abs.fill(0.0);

        // One pass computes column *and* row statistics together — the
        // original computed column stats here, then recomputed row counts
        // a second time (via `.filter().count()`) while searching for the
        // best pivot below. Same counts, half the scanning.
        for &i in &active_rows {
            let mut count = 0usize;
            for (&j, &v) in &rows[i] {
                if col_used[j] || v == 0.0 {
                    continue;
                }
                count += 1;
                col_count[j] += 1;
                let av = v.abs();
                if av > col_max_abs[j] {
                    col_max_abs[j] = av;
                }
            }
            row_count[i] = count;
        }

        let mut best: Option<(usize, usize)> = None;
        let mut best_score = usize::MAX;
        let mut best_pivot_abs = 0.0f64;

        for &i in &active_rows {
            if row_count[i] == 0 {
                continue;
            }
            for (&j, &v) in &rows[i] {
                if col_used[j] || v == 0.0 {
                    continue;
                }
                if v.abs() < STABILITY * col_max_abs[j] {
                    continue; // fails the numerical stability floor
                }
                let score = (row_count[i] - 1) * (col_count[j] - 1);
                if score < best_score || (score == best_score && v.abs() > best_pivot_abs) {
                    best_score = score;
                    best = Some((i, j));
                    best_pivot_abs = v.abs();
                }
            }
        }

        let (pi, pj) = best?;
        row_used[pi] = true;
        col_used[pj] = true;
        row_perm[step] = pi;
        col_perm[step] = pj;
        let pi_pos = row_pos[pi];
        let last = active_rows.len() - 1;
        active_rows.swap(pi_pos, last);
        row_pos[active_rows[pi_pos]] = pi_pos;
        active_rows.pop();

        let pivot_val = *rows[pi].get(&pj).unwrap();
        let pivot_row_snapshot: Vec<(usize, f64)> = rows[pi]
            .iter()
            .filter(|&(&j, &v)| v != 0.0 && (j == pj || !col_used[j]))
            .map(|(&j, &v)| (j, v))
            .collect();
        for &(j, v) in &pivot_row_snapshot {
            u_entries.push((step, j, v));
        }

        for &i in &active_rows {
            let Some(&aij) = rows[i].get(&pj) else { continue };
            if aij == 0.0 {
                continue;
            }
            let mult = aij / pivot_val;
            l_entries.push((i, step, mult));
            for &(j, v) in &pivot_row_snapshot {
                if j == pj {
                    continue; // eliminated exactly; drop rather than leave a numerical residue
                }
                let entry = rows[i].entry(j).or_insert(0.0);
                *entry -= mult * v;
            }
            rows[i].remove(&pj);
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
    fn u_diag(&self, step: usize) -> f64 {
        self.u_row[step]
            .iter()
            .find(|&&(c, _)| c == step)
            .map(|&(_, v)| v)
            .unwrap_or(0.0)
    }

    /// Partial FTRAN through `L` only (step-space): solves `L z = P_row rhs`.
    fn l_solve(&self, rhs: &[f64]) -> Vec<f64> {
        let m = self.m;
        let mut z: Vec<f64> = (0..m).map(|s| rhs[self.row_perm[s]]).collect();
        for s in 0..m {
            for &(row_step, mult) in &self.l_col[s] {
                z[row_step] -= mult * z[s];
            }
        }
        z
    }

    /// Finishes a BTRAN given a step-space vector already transformed by
    /// `U^{-T}`: applies `L^{-T}` and maps back to original row indices.
    fn l_transpose_solve(&self, z: Vec<f64>) -> Vec<f64> {
        let m = self.m;
        let mut w = z;
        for s in (0..m).rev() {
            for &(row_step, mult) in &self.l_col[s] {
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
    fn u_solve(&self, rhs: &[f64]) -> Vec<f64> {
        let mut x = rhs.to_vec();
        for eta in self.u_seq.iter().rev() {
            let p = eta.slot;
            x[p] /= eta.pivot;
            let xp = x[p];
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
        for reta in self.r_etas.iter().rev() {
            let yp = z[reta.p];
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
