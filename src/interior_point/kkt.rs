//! Sparse KKT assembly and solve for the IP-PMM Newton system.
//!
//! `A` and `G` are kept as faer's native CSR (`SparseRowMat`) end to end —
//! no dense conversion. Every KKT system this solver needs (the
//! initialization system and each Newton iteration) has the same block
//! shape:
//!
//!   [ top_diag*I      A^T          G^T         ]
//!   [   A          mid_diag*I       0          ]
//!   [   G              0       diag(bottom)    ]
//!
//! which is symmetric quasi-definite (Vanderbei), so it is factored with
//! faer's generic sparse Cholesky entry point
//! (`faer::sparse::linalg::cholesky::factorize_symbolic_cholesky` +
//! `SymbolicCholesky::factorize_numeric_ldlt`), which picks between the
//! *simplicial* and *supernodal* factorization kernels itself and applies
//! its AMD fill-reducing ordering internally.
//!
//! **Everything reusable is allocated once, on the first `solve()` call,
//! not on every iteration**: only `top_diag`/`mid_diag`/`bottom_diag`'s
//! *values* change between calls — the KKT matrix's non-zero *positions*
//! (`A`/`G`'s pattern plus the always-present diagonal), the AMD ordering,
//! the simplicial/supernodal symbolic factorization, and every scratch
//! buffer are all fixed for the lifetime of one `SparseKkt`. So `Setup`
//! (built lazily on the first call) keeps:
//!   - `values`: the flat value array in insertion order, with the three
//!     diagonal blocks living in fixed contiguous ranges that later calls
//!     overwrite directly (`copy_from_slice`/`fill`, no allocation);
//!   - `symbolic_base` + `order` (`faer::sparse::ValuesOrder`): a
//!     structure/order pair from `SymbolicSparseColMat::try_new_from_indices`
//!     that turns a later `values` array into a `SparseColMat` by
//!     re-applying the *already-computed* sort/dedup order, instead of
//!     re-sorting the triplets from scratch every call;
//!   - `chol_symbolic`, `l_values`, `numeric_buf`, `solve_buf`, `sol`: the
//!     Cholesky symbolic factorization and every numeric-factorization /
//!     solve scratch buffer, sized once and reused in place.
//! The only unavoidable per-call allocations left are `symbolic_base`'s
//! cheap `Clone` (`new_from_order_and_values` takes it by value) and the
//! `Vec<f64>` returned to the caller.

use std::ops::Range;

use faer::dyn_stack::{GlobalPodBuffer, PodStack};
use faer::mat::from_column_major_slice_mut;
use faer::sparse::linalg::cholesky::{factorize_symbolic_cholesky, LdltRegularization, SymbolicCholesky, SymmetricOrdering};
use faer::sparse::{SparseColMat, SymbolicSparseColMat, ValuesOrder};
use faer::{Conj, Parallelism, Side};

pub use crate::sparse::{mat_t_vec, mat_t_vec_into, mat_vec, mat_vec_into, Csr};

/// `0` hints faer to use `rayon::current_num_threads()` — the numeric
/// Cholesky factorization and triangular solve below are the only
/// genuinely expensive per-iteration steps in the IP-PMM loop, so this is
/// where interior-point's own parallelism budget goes; every other
/// per-iteration vector op (`sparse::mat_vec_into` etc.) is comparatively
/// cheap. Used for both the `_req` scratch-sizing call and the matching
/// real call below — they must agree, since the scratch size faer reports
/// depends on the parallelism strategy.
const PARALLELISM: Parallelism = Parallelism::Rayon(0);

/// Everything computed once per `A`/`G` sparsity pattern and reused
/// across every KKT solve for that pattern: the symbolic Cholesky
/// factorization (AMD ordering + elimination structure, independent of
/// the actual numeric values) and every scratch buffer the numeric
/// factorization/solve steps need, sized once so no iteration allocates.
struct Setup {
    /// Total KKT dimension `n + p + m`.
    dim: usize,
    /// Row/column index ranges, within the `dim x dim` KKT matrix, of
    /// each of its three diagonal blocks (top = primal `x` block, mid =
    /// equality-multiplier `y` block, bottom = inequality-multiplier `z`
    /// block).
    top_range: Range<usize>,
    mid_range: Range<usize>,
    bottom_range: Range<usize>,
    /// The KKT matrix's nonzero values in `order`'s layout, rewritten in
    /// place every solve (structure fixed, only values change).
    values: Vec<f64>,
    /// The fixed sparsity pattern (upper triangle only, as faer's
    /// Cholesky-family solvers require).
    symbolic_base: SymbolicSparseColMat<usize>,
    /// Maps `(row, col, value)` triplets to their position in `values` —
    /// lets a new set of numeric values be dropped in without re-sorting
    /// or re-deduplicating the triplet list from scratch every call.
    order: ValuesOrder<usize>,
    /// The AMD-ordered elimination structure, computed once from
    /// `symbolic_base` and independent of the actual numeric values.
    chol_symbolic: SymbolicCholesky<usize>,
    /// The numeric `L` factor's values — recomputed every solve via
    /// `chol_symbolic.factorize_numeric_ldlt`, but the `Vec` itself is
    /// only ever allocated once here.
    l_values: Vec<f64>,
    numeric_buf: GlobalPodBuffer,
    solve_buf: GlobalPodBuffer,
}

/// The KKT system's sparsity pattern (`n` primal + `p` equality-multiplier
/// + `m` inequality-multiplier variables) plus, once `solve_into` has been
/// called at least once, the cached `Setup` every subsequent call reuses.
/// One `SparseKkt` is built per problem instance and lives for the whole
/// IP-PMM Newton loop (`ipm.rs`'s `Workspace`), so `Setup` is built
/// exactly once regardless of how many Newton iterations run.
pub struct SparseKkt {
    pub n: usize,
    pub p: usize,
    pub m: usize,
    setup: Option<Setup>,
}

impl SparseKkt {
    pub fn new(n: usize, p: usize, m: usize) -> Self {
        SparseKkt { n, p, m, setup: None }
    }

    /// The KKT matrix's total dimension, `n + p + m`.
    pub fn dim(&self) -> usize {
        self.n + self.p + self.m
    }

    fn build_setup(&self, a: &Csr, g: &Csr, top_diag: f64, mid_diag: f64, bottom_diag: &[f64]) -> Setup {
        let (n, p, m) = (self.n, self.p, self.m);
        let dim = self.dim();

        let mut positions: Vec<(usize, usize)> = Vec::new();
        let mut values: Vec<f64> = Vec::new();

        for i in 0..n {
            positions.push((i, i));
            values.push(top_diag);
        }
        let top_range = 0..n;

        let ar = a.as_ref();
        for i in 0..p {
            for (j, &v) in ar.col_indices_of_row(i).zip(ar.values_of_row(i)) {
                if v != 0.0 {
                    // row = j < n <= n+i = col: always upper triangular.
                    positions.push((j, n + i));
                    values.push(v);
                }
            }
        }
        let gr = g.as_ref();
        for i in 0..m {
            for (j, &v) in gr.col_indices_of_row(i).zip(gr.values_of_row(i)) {
                if v != 0.0 {
                    positions.push((j, n + p + i));
                    values.push(v);
                }
            }
        }

        let mid_start = positions.len();
        for i in 0..p {
            positions.push((n + i, n + i));
            values.push(mid_diag);
        }
        let mid_range = mid_start..positions.len();

        let bottom_start = positions.len();
        for i in 0..m {
            positions.push((n + p + i, n + p + i));
            values.push(bottom_diag[i]);
        }
        let bottom_range = bottom_start..positions.len();

        let (symbolic_base, order) = SymbolicSparseColMat::<usize>::try_new_from_indices(dim, dim, &positions)
            .expect("valid KKT sparsity pattern");

        // One-time: AMD ordering + simplicial-vs-supernodal symbolic
        // analysis, both chosen automatically by faer. Only the pattern
        // matters here, so this can run before `values` holds real numbers.
        let chol_symbolic = factorize_symbolic_cholesky::<usize>(
            symbolic_base.as_ref(),
            Side::Upper,
            SymmetricOrdering::Amd,
            Default::default(),
        )
        .expect("symbolic factorization failed");

        let l_values = vec![0.0f64; chol_symbolic.len_values()];
        // The scratch size `_req` reports depends on the parallelism
        // strategy, so it must match whatever `solve_into` actually passes
        // to `factorize_numeric_ldlt` below (`PARALLELISM`) — a mismatch
        // here would under-size `numeric_buf` for the real call.
        let numeric_buf = GlobalPodBuffer::new(
            chol_symbolic.factorize_numeric_ldlt_req::<f64>(false, PARALLELISM).unwrap(),
        );
        let solve_buf = GlobalPodBuffer::new(chol_symbolic.solve_in_place_req::<f64>(1).unwrap());

        Setup {
            dim,
            top_range,
            mid_range,
            bottom_range,
            values,
            symbolic_base,
            order,
            chol_symbolic,
            l_values,
            numeric_buf,
            solve_buf,
        }
    }

    /// Solves `K x = rhs` for the block KKT system described above, writing
    /// the length-`dim()` solution into `out` (no allocation beyond the
    /// unavoidable `symbolic_base` clone described in the module docs).
    pub fn solve_into(&mut self, a: &Csr, g: &Csr, top_diag: f64, mid_diag: f64, bottom_diag: &[f64], rhs: &[f64], out: &mut [f64]) {
        if self.setup.is_none() {
            self.setup = Some(self.build_setup(a, g, top_diag, mid_diag, bottom_diag));
        }
        let setup = self.setup.as_mut().unwrap();

        // Only the diagonal blocks change between calls; the off-diagonal
        // A/G-derived entries were written once in `build_setup` and never
        // touched again.
        for v in &mut setup.values[setup.top_range.clone()] {
            *v = top_diag;
        }
        for v in &mut setup.values[setup.mid_range.clone()] {
            *v = mid_diag;
        }
        setup.values[setup.bottom_range.clone()].copy_from_slice(bottom_diag);

        // Re-applies the sort/dedup order computed once in `build_setup`
        // instead of re-sorting the (row, col) pattern from scratch.
        let a_upper = SparseColMat::<usize, f64>::new_from_order_and_values(
            setup.symbolic_base.clone(),
            &setup.order,
            &setup.values,
        )
        .expect("value reorder failed");

        let ldlt = setup.chol_symbolic.factorize_numeric_ldlt::<f64>(
            &mut setup.l_values,
            a_upper.as_ref(),
            Side::Upper,
            LdltRegularization::default(),
            PARALLELISM,
            PodStack::new(&mut setup.numeric_buf),
        );

        out.copy_from_slice(rhs);
        ldlt.solve_in_place_with_conj(
            Conj::No,
            from_column_major_slice_mut(out, setup.dim, 1),
            PARALLELISM,
            PodStack::new(&mut setup.solve_buf),
        );
    }
}
