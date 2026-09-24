//! Modified Ruiz equilibration: a diagonal preconditioner that rescales
//! variables and constraint rows so that `[A; G]`'s rows and columns have
//! roughly unit infinity-norm, improving the conditioning of both the KKT
//! systems `interior_point.rs` solves and the LU systems `simplex.rs`
//! factorizes. This is a one-time step per `presolve::run_extended()` call
//! (not per iteration), computed on the original problem data before
//! either engine's main loop starts.
//!
//! The scaled problem is `x = D x'`, `A' = diag(e_a) A diag(d)`,
//! `b' = diag(e_a) b`, `G' = diag(e_g) G diag(d)`, `h' = diag(e_g) h`,
//! `c' = diag(d) c`. Solving the scaled problem and recovering
//! `x = diag(d) x'` gives the same solution as solving the original
//! problem directly (a diagonal reparametrization changes neither
//! feasibility nor boundedness).

//! **Parallelization**: every per-row/per-column loop in this module
//! (`compute`'s column-norm accumulation and row/column normalization,
//! `apply`'s rescaling, `unscale_x`) is embarrassingly parallel in
//! principle, but `apply`/`unscale_x` run sequentially unconditionally —
//! profiling on this crate's target problem sizes (~1000 columns, a
//! couple thousand rows across `A`/`G`) found rayon's per-call dispatch
//! overhead exceeding the arithmetic itself there, the same finding as
//! `simplex.rs`'s per-pivot loops (see `solve_lp_dual_on`'s module docs).
//! `compute`'s own column-norm fold — by far the largest single cost in
//! this crate's presolve pipeline at that same target size (measured at
//! ~38% of total presolve time before this file's sequential rewrite) —
//! picks sequential vs. `rayon` once per call from `RAYON_SIZE_THRESHOLD`
//! (own docs), replacing an earlier run-both-and-time self-calibration:
//! this crate's own microbenchmark never found `rayon` beating a plain
//! sequential fold at any size tried, up to 4,000,000 rows, so the live
//! race was pure overhead for a decision with a fixed, always-the-same
//! answer at every problem size this crate has ever actually measured.

use crate::sparse::{Csr, CsrRowBuilder, csr_from_rows, csr_row_iter};
use crate::params::presolve::RAYON_SIZE_THRESHOLD;

pub struct Scaling {
    pub d: Vec<f64>,
    pub e_a: Vec<f64>,
    pub e_g: Vec<f64>,
}

/// The per-row-subset half of the column-norm fold: folds `rows` of `mat`
/// (scaled by `d` and that row's own `e`) into a length-`n` buffer via
/// elementwise max. Shared by the `A` and `G` accumulations in `compute`.
/// Sequential, not rayon — see `compute`'s own docs.
fn col_norm_fold(mat: faer::sparse::SparseRowMatRef<usize, f64>, d: &[f64], e: &[f64], rows: std::ops::Range<usize>, acc: &mut [f64]) {
    for i in rows {
        let ei = e[i];
        for (&j, &v) in mat.col_indices_of_row_raw(i).iter().zip(mat.values_of_row(i)) {
            let cand = (v * d[j] * ei).abs();
            if cand > acc[j] {
                acc[j] = cand;
            }
        }
    }
}

/// `e[i] <- e[i] / sqrt(max_j |v_ij * d_j * e[i]|)` for every row of `mat`
/// (skipped when that max is at most the zero tolerance) — the row-norm
/// half of one Ruiz iteration, over the raw CSR slices.
fn row_norm_update(mat: faer::sparse::SparseRowMatRef<usize, f64>, d: &[f64], e: &mut [f64]) {
    let zero_tol = tunable!("ENOMOTO_T_SCALING_ZERO_TOL", 1e-12, f64);
    for (i, e) in e.iter_mut().enumerate() {
        let old_e = *e;
        let mut row_norm = 0.0f64;
        for (&j, &v) in mat.col_indices_of_row_raw(i).iter().zip(mat.values_of_row(i)) {
            row_norm = row_norm.max((v * d[j] * old_e).abs());
        }
        if row_norm > zero_tol {
            *e = old_e / row_norm.sqrt();
        }
    }
}

/// Same fold as `col_norm_fold`, via rayon: a per-thread local buffer
/// (fold) merged by elementwise max (reduce) — merging needed because
/// different rows can update the same column `j`, so a naive
/// `par_iter_mut` over `acc` would race. Only ever invoked by `compute`'s
/// own self-calibration, never on its own, so it always starts from an
/// all-zero `acc` in practice; written to merge into whatever `acc`
/// already holds anyway; matching `col_norm_fold`'s own contract exactly.
fn col_norm_fold_parallel(mat: faer::sparse::SparseRowMatRef<usize, f64>, d: &[f64], e: &[f64], rows: std::ops::Range<usize>, acc: &mut [f64]) {
    use rayon::prelude::*;
    let n = acc.len();
    // Each fold/reduce leaf allocates and later merges an O(n) buffer, so
    // an unbounded split (rayon's default for a plain range with no
    // length hint keeps splitting under work-stealing, not just once per
    // thread) makes the O(n) merge cost dominate at just a few thousand
    // rows — measured directly making this ~500x slower than sequential
    // at 200,000 rows before `with_min_len` was added. Bounding the chunk
    // size to roughly rows/threads caps the number of leaves at the
    // thread count, so the merge overhead stays O(threads * n) instead of
    // O(rows * n).
    let n_rows = rows.end - rows.start;
    let min_len = (n_rows / rayon::current_num_threads().max(1)).max(1);
    let folded = rows.into_par_iter().with_min_len(min_len).fold(
        || vec![0.0f64; n],
        |mut local, i| {
            for (j, &v) in mat.col_indices_of_row(i).zip(mat.values_of_row(i)) {
                let cand = (v * d[j] * e[i]).abs();
                if cand > local[j] {
                    local[j] = cand;
                }
            }
            local
        },
    );
    let merged = folded.reduce(
        || vec![0.0f64; n],
        |mut a, b| {
            for j in 0..n {
                if b[j] > a[j] {
                    a[j] = b[j];
                }
            }
            a
        },
    );
    for j in 0..n {
        if merged[j] > acc[j] {
            acc[j] = merged[j];
        }
    }
}

pub fn compute(n: usize, a: &Csr, g: &Csr, c: &[f64], iters: usize) -> Scaling {
    compute_impl(n, a, g, c, iters, tunable!("ENOMOTO_T_SCALE_UNIT_FAST", 1, usize) != 0)
}

fn compute_impl(n: usize, a: &Csr, g: &Csr, c: &[f64], iters: usize, unit_fast: bool) -> Scaling {
    let p = a.nrows();
    let m = g.nrows();
    let mut d = vec![1.0; n];
    let mut e_a = vec![1.0; p];
    let mut e_g = vec![1.0; m];

    let ar = a.as_ref();
    let gr = g.as_ref();

    let mut col_norm = vec![0.0f64; n];

    // Decided once from the combined row count against
    // `RAYON_SIZE_THRESHOLD` — see that constant's own docs for why this
    // replaced an earlier run-both-and-time self-calibration.
    let use_parallel_fold = (p + m) > tunable!("ENOMOTO_T_SCALING_RAYON_SIZE_THRESHOLD", RAYON_SIZE_THRESHOLD, usize);

    // `ENOMOTO_T_SCALE_NOBOUNDS=1` (default 0 = off, the historical
    // behaviour): leave `g`'s single-entry rows (the box-bound rows
    // `build_a_g` adds per finite bound — on e.g. `fit2d` 99% of `g`) out
    // of the Ruiz iteration entirely, so they neither pull on `d` nor get
    // scanned every iteration, and give each one the closed-form row scale
    // `1/|v * d_j|` afterwards (a bound row's own Ruiz fixed point: its
    // scaled coefficient becomes +-1, i.e. the plain bound on `x'_j`).
    // Changes the scale factors, hence the numerical path.
    if tunable!("ENOMOTO_T_SCALE_NOBOUNDS", 0, usize) != 0 {
        let multi: Vec<usize> = (0..m).filter(|&i| gr.col_indices_of_row_raw(i).len() > 1).collect();
        for _ in 0..iters {
            col_norm.fill(0.0);
            col_norm_fold(ar, &d, &e_a, 0..p, &mut col_norm);
            for &i in &multi {
                col_norm_fold(gr, &d, &e_g, i..i + 1, &mut col_norm);
            }
            for j in 0..n {
                let cj = (c[j] * d[j]).abs();
                if cj > col_norm[j] {
                    col_norm[j] = cj;
                }
            }
            for j in 0..n {
                if col_norm[j] > tunable!("ENOMOTO_T_SCALING_ZERO_TOL", 1e-12, f64) {
                    d[j] /= col_norm[j].sqrt();
                }
            }
            row_norm_update(ar, &d, &mut e_a);
            let zero_tol = tunable!("ENOMOTO_T_SCALING_ZERO_TOL", 1e-12, f64);
            for &i in &multi {
                let old_e = e_g[i];
                let mut row_norm = 0.0f64;
                for (&j, &v) in gr.col_indices_of_row_raw(i).iter().zip(gr.values_of_row(i)) {
                    row_norm = row_norm.max((v * d[j] * old_e).abs());
                }
                if row_norm > zero_tol {
                    e_g[i] = old_e / row_norm.sqrt();
                }
            }
        }
        for i in 0..m {
            let cols = gr.col_indices_of_row_raw(i);
            if cols.len() == 1 {
                let s = (gr.values_of_row(i)[0] * d[cols[0]]).abs();
                if s > tunable!("ENOMOTO_T_SCALING_ZERO_TOL", 1e-12, f64) && s.is_finite() {
                    e_g[i] = 1.0 / s;
                }
            }
        }
        return Scaling { d, e_a, e_g };
    }

    // Sequential path: `G`'s single-entry rows (the bound rows `build_a_g`
    // adds, often most of `G`) go through a flat `(column, coefficient)`
    // list instead of the CSR walk, and a row whose column and `|v|` equal
    // the previous single-entry row's (a variable's `ub` row followed by its
    // `lb` row: `(j, 1.0)`, `(j, -1.0)`) shares that row's scale — both
    // start at 1 and every update reads only `|v * d_j * e|`, so their
    // factors are equal bit for bit at every iteration. The column-norm
    // fold is a max (order-free), and row updates are per row, so this
    // computes exactly the same `d`/`e_a`/`e_g` as the plain loop below.
    if !use_parallel_fold && unit_fast {
        let zero_tol = tunable!("ENOMOTO_T_SCALING_ZERO_TOL", 1e-12, f64);
        let mut multi: Vec<usize> = Vec::new();
        // Leaders: (row, column, coefficient); `follow[k] = leader index`.
        let mut unit: Vec<(usize, usize, f64)> = Vec::new();
        let mut follow: Vec<(usize, usize)> = Vec::new();
        for i in 0..m {
            let cols = gr.col_indices_of_row_raw(i);
            if cols.len() == 1 {
                let (j, v) = (cols[0], gr.values_of_row(i)[0]);
                match unit.last() {
                    Some(&(_, lj, lv)) if lj == j && lv.abs().to_bits() == v.abs().to_bits() => follow.push((i, unit.len() - 1)),
                    _ => unit.push((i, j, v)),
                }
            } else {
                multi.push(i);
            }
        }
        let mut ue = vec![1.0f64; unit.len()];
        for _ in 0..iters {
            col_norm.fill(0.0);
            col_norm_fold(ar, &d, &e_a, 0..p, &mut col_norm);
            for &i in &multi {
                let ei = e_g[i];
                for (&j, &v) in gr.col_indices_of_row_raw(i).iter().zip(gr.values_of_row(i)) {
                    let cand = (v * d[j] * ei).abs();
                    if cand > col_norm[j] {
                        col_norm[j] = cand;
                    }
                }
            }
            for (&(_, j, v), &ei) in unit.iter().zip(&ue) {
                let cand = (v * d[j] * ei).abs();
                if cand > col_norm[j] {
                    col_norm[j] = cand;
                }
            }
            for j in 0..n {
                let cj = (c[j] * d[j]).abs();
                if cj > col_norm[j] {
                    col_norm[j] = cj;
                }
            }
            for j in 0..n {
                if col_norm[j] > zero_tol {
                    d[j] /= col_norm[j].sqrt();
                }
            }
            row_norm_update(ar, &d, &mut e_a);
            for &i in &multi {
                let old_e = e_g[i];
                let mut row_norm = 0.0f64;
                for (&j, &v) in gr.col_indices_of_row_raw(i).iter().zip(gr.values_of_row(i)) {
                    row_norm = row_norm.max((v * d[j] * old_e).abs());
                }
                if row_norm > zero_tol {
                    e_g[i] = old_e / row_norm.sqrt();
                }
            }
            for (&(_, j, v), e) in unit.iter().zip(ue.iter_mut()) {
                let old_e = *e;
                let row_norm = 0.0f64.max((v * d[j] * old_e).abs());
                if row_norm > zero_tol {
                    *e = old_e / row_norm.sqrt();
                }
            }
        }
        for (&(i, _, _), &e) in unit.iter().zip(&ue) {
            e_g[i] = e;
        }
        for &(i, k) in &follow {
            e_g[i] = ue[k];
        }
        return Scaling { d, e_a, e_g };
    }

    for _ in 0..iters {
        col_norm.fill(0.0);
        if use_parallel_fold {
            col_norm_fold_parallel(ar, &d, &e_a, 0..p, &mut col_norm);
            col_norm_fold_parallel(gr, &d, &e_g, 0..m, &mut col_norm);
        } else {
            col_norm_fold(ar, &d, &e_a, 0..p, &mut col_norm);
            col_norm_fold(gr, &d, &e_g, 0..m, &mut col_norm);
        }
        for j in 0..n {
            let cj = (c[j] * d[j]).abs();
            if cj > col_norm[j] {
                col_norm[j] = cj;
            }
        }

        for j in 0..n {
            if col_norm[j] > tunable!("ENOMOTO_T_SCALING_ZERO_TOL", 1e-12, f64) {
                d[j] /= col_norm[j].sqrt();
            }
        }

        // Row-norm updates: row `i`'s own coefficients only, writing only
        // `e_a[i]`/`e_g[i]`.
        row_norm_update(ar, &d, &mut e_a);
        row_norm_update(gr, &d, &mut e_g);
    }

    Scaling { d, e_a, e_g }
}

pub fn apply(scaling: &Scaling, a: &Csr, g: &Csr, b: &[f64], h: &[f64], c: &[f64]) -> (Csr, Csr, Vec<f64>, Vec<f64>, Vec<f64>) {
    let n = scaling.d.len();
    let p = a.nrows();
    let m = g.nrows();

    // Each row's rescaled entries are independent of every other row —
    // sequential nonetheless, per this module's own parallelization note.
    let a_scaled = scale_rows(a, &scaling.e_a, &scaling.d, n);
    let g_scaled = scale_rows(g, &scaling.e_g, &scaling.d, n);
    debug_assert_eq!(a_scaled.nrows(), p);
    debug_assert_eq!(g_scaled.nrows(), m);
    let b_scaled: Vec<f64> = b.iter().zip(&scaling.e_a).map(|(v, e)| v * e).collect();
    let h_scaled: Vec<f64> = h.iter().zip(&scaling.e_g).map(|(v, e)| v * e).collect();
    let c_scaled: Vec<f64> = c.iter().zip(&scaling.d).map(|(v, d)| v * d).collect();

    (a_scaled, g_scaled, b_scaled, h_scaled, c_scaled)
}

/// `csr_from_rows` of the rows `(j, v * e[i] * d[j])`, written straight
/// into a [`CsrRowBuilder`] through one reused row buffer instead of a
/// `Vec` per row (the builder applies `csr_from_rows`'s own zero-dropping
/// and sorting, so the result is the same matrix bit for bit; a row the
/// builder rejects falls back to the original construction).
fn scale_rows(mat: &Csr, e: &[f64], d: &[f64], n: usize) -> Csr {
    let r = mat.as_ref();
    let rows = r.nrows();
    let nnz: usize = (0..rows).map(|i| r.col_indices_of_row_raw(i).len()).sum();
    let mut builder = CsrRowBuilder::with_capacity(n, rows, nnz);
    let mut buf: Vec<(usize, f64)> = Vec::new();
    for i in 0..rows {
        let ei = e[i];
        buf.clear();
        buf.extend(r.col_indices_of_row_raw(i).iter().zip(r.values_of_row(i)).map(|(&j, &v)| (j, v * ei * d[j])));
        if !builder.push_row(&buf) {
            let all: Vec<Vec<(usize, f64)>> = (0..rows).map(|i| csr_row_iter(mat, i).map(|(j, v)| (j, v * e[i] * d[j])).collect()).collect();
            return csr_from_rows(&all, n);
        }
    }
    builder.finish()
}

/// Recovers the original-problem solution `x = diag(d) x'` from a solve
/// performed on the scaled problem's `x'` — the inverse of what `apply`
/// did to the variables, applied once at the very end after either
/// engine's main loop converges (not needed at any point during the loop
/// itself, which works entirely in scaled space).
pub fn unscale_x(scaling: &Scaling, x: &[f64]) -> Vec<f64> {
    x.iter().zip(&scaling.d).map(|(v, d)| v * d).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The single-entry-row fast path must give exactly the plain loop's
    /// factors (bit for bit), with bound-row pairs, lone bound rows,
    /// scaled singleton constraints, empty rows and multi-entry rows.
    #[test]
    fn unit_fast_path_matches_plain_loop_bit_for_bit() {
        let mut state: u64 = 0x5eed_1234_abcd_ef01;
        let mut rnd = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for trial in 0..100 {
            let n = 3 + (trial % 6);
            let val = |r: u64| [1.0, -2.5, 0.125, 7.0, -0.3, 1e3][(r % 6) as usize];
            let mut rows_of = |count: usize, rnd: &mut dyn FnMut() -> u64| -> Vec<Vec<(usize, f64)>> {
                (0..count)
                    .map(|_| {
                        let mut row: Vec<(usize, f64)> = Vec::new();
                        for _ in 0..(rnd() % 4) {
                            let j = (rnd() % n as u64) as usize;
                            if row.iter().all(|&(k, _)| k != j) {
                                row.push((j, val(rnd())));
                            }
                        }
                        row
                    })
                    .collect()
            };
            let a_rows = rows_of(1 + (rnd() % 3) as usize, &mut rnd);
            let mut g_rows = rows_of((rnd() % 4) as usize, &mut rnd);
            for j in 0..n {
                match rnd() % 4 {
                    0 => {
                        g_rows.push(vec![(j, 1.0)]);
                        g_rows.push(vec![(j, -1.0)]);
                    }
                    1 => g_rows.push(vec![(j, -1.0)]),
                    2 => g_rows.push(vec![(j, val(rnd()))]),
                    _ => {}
                }
            }
            let a = csr_from_rows(&a_rows, n);
            let g = csr_from_rows(&g_rows, n);
            let c: Vec<f64> = (0..n).map(|_| val(rnd())).collect();
            let fast = compute_impl(n, &a, &g, &c, 10, true);
            let plain = compute_impl(n, &a, &g, &c, 10, false);
            let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(&fast.d), bits(&plain.d), "trial {trial}");
            assert_eq!(bits(&fast.e_a), bits(&plain.e_a), "trial {trial}");
            assert_eq!(bits(&fast.e_g), bits(&plain.e_g), "trial {trial}");
        }
    }

    /// Same purpose as `simplex.rs`'s `rayon_threshold_microbench`, timing
    /// `col_norm_fold` vs `col_norm_fold_parallel` on synthetic sparse
    /// matrices of increasing row count (~5 nonzeros/row, similar density
    /// to this crate's real problem data). `#[ignore]`d by default; run
    /// with `cargo test --release col_norm_fold_rayon_threshold_microbench
    /// -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn col_norm_fold_rayon_threshold_microbench() {
        use std::time::Instant;

        for &rows in &[300usize, 1_000, 5_000, 10_000, 50_000, 200_000, 1_000_000, 4_000_000] {
            let n = (rows / 2).max(10);
            let row_data: Vec<Vec<(usize, f64)>> = (0..rows)
                .map(|i| {
                    (0..5)
                        .map(|k| {
                            let j = (i * 7 + k * 131) % n;
                            let v = ((i * 2654435761u64 as usize + k) % 1000) as f64 / 1000.0 + 0.1;
                            (j, v)
                        })
                        .collect()
                })
                .collect();
            let mat = csr_from_rows(&row_data, n);
            let mr = mat.as_ref();
            let d = vec![1.0f64; n];
            let e = vec![1.0f64; rows];
            const REPS: usize = 50;

            let t0 = Instant::now();
            for _ in 0..REPS {
                let mut acc = vec![0.0f64; n];
                col_norm_fold(mr, &d, &e, 0..rows, &mut acc);
                std::hint::black_box(&acc);
            }
            let seq = t0.elapsed() / REPS as u32;

            let t1 = Instant::now();
            for _ in 0..REPS {
                let mut acc = vec![0.0f64; n];
                col_norm_fold_parallel(mr, &d, &e, 0..rows, &mut acc);
                std::hint::black_box(&acc);
            }
            let par = t1.elapsed() / REPS as u32;

            println!("rows={rows:>7} sequential={seq:>10?} rayon={par:>10?} rayon/sequential={:.2}x", par.as_secs_f64() / seq.as_secs_f64());
        }
    }
}
