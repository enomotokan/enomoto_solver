//! **The one home for sparse storage in this crate.** Every sparse
//! representation the solver uses — the row-major [`CsrMat`], its
//! column-major companion [`CscMat`], the single sparse vector
//! [`SparseVec`], the sparse accumulator [`SparseAccum`] that merges them
//! — lives here, together with the conversions between them and the
//! sparse x dense arithmetic built on top. Nothing outside this file
//! should be re-deriving a transpose, re-writing a `(index, value)` merge,
//! or hand-rolling a mat-vec: the computational modules (`simplex`,
//! `simplex::extended_dual`, `presolve/*`, `interior_point::kkt`) call in
//! here instead.
//!
//! # Layout
//!
//! All four of this module's owning types share one physical layout — the
//! classical "offsets + flat entries" compressed form:
//!
//! ```text
//!   offsets: [0, o1, o2, ..., nnz]     (outer_len + 1 entries)
//!   entries: [(inner, value), ...]     (nnz entries)
//! ```
//!
//! with outer index `k`'s nonzeros living in `entries[offsets[k]..offsets[k+1]]`.
//! Whether "outer" means *row* (so `inner` is a column index) or *column*
//! (so `inner` is a row index) is the only difference between CSR and CSC,
//! which is exactly why both are one [`Compressed`] struct wearing two
//! type-level hats rather than two near-identical copies. Keeping the pair
//! `(inner, value)` interleaved — rather than the textbook's parallel
//! `indices`/`values` arrays — is deliberate: every consumer in this crate
//! reads index and value together, so one contiguous stream beats two
//! that must be advanced in lockstep, and it lets `row()`/`col()` hand out
//! a plain `&[(usize, f64)]` that composes directly with
//! [`SparseVec::entries`] and with `simplex::lu`'s own sparse-rhs solves.
//!
//! Both are *immutable once built*: there is no `insert`, no `push`, no
//! growable per-row `Vec`. A pass that is still rewriting its matrix
//! (everything under `presolve/`) works in `Vec<Vec<(usize, f64)>>` and
//! freezes into a [`CsrMat`] at the end; a pass that only ever reads it
//! (`simplex`'s `StdForm`, which is frozen the moment presolve hands it
//! off) holds the compressed form directly and pays exactly two
//! allocations for the whole matrix.
//!
//! # `Csr` (faer) vs. [`CsrMat`] (ours)
//!
//! Two row-major sparse types coexist here on purpose:
//!
//!   - [`Csr`] is an alias for faer's `SparseRowMat`. It is the currency
//!     of `presolve`'s public interfaces and of `interior_point::kkt`,
//!     which needs faer's own `SparseColMat`/Cholesky machinery downstream
//!     — so that data has to be in faer's types anyway.
//!   - [`CsrMat`]/[`CscMat`] are this crate's own, used where faer buys
//!     nothing and costs something: `simplex`'s hot loops want a
//!     `&[(usize, f64)]` slice per row/column with no iterator
//!     zip-and-map in the way, and want the CSR *and* CSC views of the
//!     same matrix side by side (faer would need two separate matrices
//!     and a transpose through its own builders).
//!
//! [`csr_rows`], [`CsrMat::from_faer`] and [`CsrMat::to_faer`] bridge the
//! two, so a call site never has to open-code the
//! `col_indices_of_row(i).zip(values_of_row(i))` dance that used to be
//! copy-pasted across ~20 presolve files.
//!
//! # Parallelization
//!
//! `mat * x` writes one output entry per row, independently, so it
//! parallelizes via a plain `par_iter_mut` over the already-allocated
//! `out` slice — no allocation inside the call, which matters since
//! `interior_point.rs`'s Newton loop calls it every iteration and its own
//! docs promise that loop never allocates. `mat^T * y` from a *row-major*
//! matrix, by contrast, is a *scatter* over `out` (every row can touch any
//! column of the transpose), which would need either atomics or a
//! fold/reduce with a fresh per-thread buffer — the latter being exactly
//! the allocation this function is called from a no-allocation loop to
//! avoid — so it stays sequential. (A [`CscMat`] has the axes the other
//! way round, so *its* transpose product is the parallel one and its
//! forward product the scatter; see [`CscMat::mat_t_vec_into`].)

// This module is the crate's sparse-storage toolbox, so it deliberately
// carries the *complete* set of operations for each representation — both
// orientations of every product, the conversions in both directions, the
// whole sparse-vector algebra — rather than only the subset today's call
// sites happen to reach. `dead_code` is allowed here, and only here: an
// unused item in this file is API surface waiting for its caller, not the
// rot the lint normally catches, and every one of them is exercised by the
// unit tests at the bottom of the file.
#![allow(dead_code)]

use rayon::prelude::*;

// ===========================================================================
// Sparse vectors
// ===========================================================================

/// A sparse vector over `0..len`: its nonzeros as `(index, value)` pairs,
/// in whatever order they were produced.
///
/// This is the crate's one sparse-vector type, and it is deliberately thin
/// — `entries` is a plain `Vec<(usize, f64)>`, and [`SparseVec::entries`]
/// hands out the bare slice — because the two things that consume sparse
/// vectors here already speak exactly that: `CsrMat::row`/`CscMat::col`
/// return `&[(usize, f64)]`, and `simplex::lu`'s sparse-rhs FTRAN
/// (`FtLu::solve_sparse_into`) takes `&[(usize, f64)]`. A newtype that
/// hid the representation would force a conversion at every one of those
/// boundaries; what this type adds instead is the *operations* that were
/// previously re-implemented per call site — scatter/gather against a
/// dense buffer, a dot product against a dense vector, pruning, and the
/// density check that decides sparse-vs-dense dispatch.
///
/// **Ordering and duplicates are the producer's business.** Nothing here
/// sorts or deduplicates on your behalf (that would silently cost
/// `O(nnz log nnz)` in loops that don't need it); call [`Self::sort`] or
/// [`Self::canonicalize`] when you need a canonical form, which the
/// presolve passes do because their output ordering feeds later
/// tie-breaks.
#[derive(Clone, Debug, PartialEq)]
pub struct SparseVec {
    len: usize,
    entries: Vec<(usize, f64)>,
}

impl SparseVec {
    /// An all-zero sparse vector of logical length `len`.
    pub fn zeros(len: usize) -> Self {
        SparseVec { len, entries: Vec::new() }
    }

    /// An all-zero sparse vector of logical length `len` with room for
    /// `cap` nonzeros already reserved.
    pub fn with_capacity(len: usize, cap: usize) -> Self {
        SparseVec { len, entries: Vec::with_capacity(cap) }
    }

    /// Takes ownership of an existing `(index, value)` list. Indices are
    /// trusted to be `< len`; debug builds assert it.
    pub fn from_entries(len: usize, entries: Vec<(usize, f64)>) -> Self {
        debug_assert!(entries.iter().all(|&(i, _)| i < len), "sparse index out of range");
        SparseVec { len, entries }
    }

    /// Extracts `dense`'s nonzeros, in ascending index order. Exact zeros
    /// only — see [`Self::from_dense_tol`] for a tolerance-gated variant.
    pub fn from_dense(dense: &[f64]) -> Self {
        let entries = dense.iter().enumerate().filter(|&(_, &v)| v != 0.0).map(|(i, &v)| (i, v)).collect();
        SparseVec { len: dense.len(), entries }
    }

    /// Like [`Self::from_dense`], but treating anything with `|v| <= tol`
    /// as structurally zero. This is the "sparsify a dense working
    /// vector" entry point — a dense buffer that a computation filled but
    /// whose result is mostly zeros becomes a sparse vector that the next
    /// stage can iterate in `O(nnz)` instead of `O(len)`.
    pub fn from_dense_tol(dense: &[f64], tol: f64) -> Self {
        let entries = dense.iter().enumerate().filter(|&(_, &v)| v.abs() > tol).map(|(i, &v)| (i, v)).collect();
        SparseVec { len: dense.len(), entries }
    }

    /// The vector's logical (dense) length.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Number of stored entries. Note this counts *stored* entries, which
    /// after an arithmetic cancellation may include explicit zeros until
    /// [`Self::prune`] runs.
    #[inline]
    pub fn nnz(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// `nnz / len` — the ratio the solver's sparse-vs-dense dispatch
    /// decisions are phrased in (`FtLu::should_use_dense_solve` and
    /// friends). `0.0` for a zero-length vector rather than a NaN.
    #[inline]
    pub fn density(&self) -> f64 {
        if self.len == 0 {
            0.0
        } else {
            self.entries.len() as f64 / self.len as f64
        }
    }

    /// The stored `(index, value)` pairs — the form `CsrMat::row`,
    /// `CscMat::col` and `simplex::lu`'s sparse solves all speak.
    #[inline]
    pub fn entries(&self) -> &[(usize, f64)] {
        &self.entries
    }

    #[inline]
    pub fn entries_mut(&mut self) -> &mut [(usize, f64)] {
        &mut self.entries
    }

    /// Consumes the vector, yielding its entry list.
    #[inline]
    pub fn into_entries(self) -> Vec<(usize, f64)> {
        self.entries
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (usize, f64)> + '_ {
        self.entries.iter().copied()
    }

    /// Appends one nonzero. No duplicate check — see the type's own docs.
    #[inline]
    pub fn push(&mut self, index: usize, value: f64) {
        debug_assert!(index < self.len, "sparse index out of range");
        self.entries.push((index, value));
    }

    /// Drops every stored entry, keeping the allocation and the logical
    /// length — so a buffer can be reused across iterations of a loop
    /// without re-allocating.
    #[inline]
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Sorts the entries by index. Does not merge duplicates.
    pub fn sort(&mut self) {
        self.entries.sort_unstable_by_key(|&(i, _)| i);
    }

    /// Drops every entry with `|v| <= tol`.
    pub fn prune(&mut self, tol: f64) {
        self.entries.retain(|&(_, v)| v.abs() > tol);
    }

    /// Sorts by index, sums duplicate indices, and drops anything left
    /// within `tol` of zero — the canonical form presolve wants before it
    /// writes a row back, since its output ordering can feed a later
    /// tie-break (see `types.rs::LinearExpr`'s own docs on determinism).
    pub fn canonicalize(&mut self, tol: f64) {
        self.sort();
        let mut write = 0usize;
        for read in 0..self.entries.len() {
            if write > 0 && self.entries[write - 1].0 == self.entries[read].0 {
                self.entries[write - 1].1 += self.entries[read].1;
            } else {
                self.entries[write] = self.entries[read];
                write += 1;
            }
        }
        self.entries.truncate(write);
        self.prune(tol);
    }

    /// Materializes the dense form as a fresh `Vec`.
    pub fn to_dense(&self) -> Vec<f64> {
        let mut out = vec![0.0; self.len];
        self.scatter_into(&mut out);
        out
    }

    /// Writes this vector's nonzeros into `out` (length `len`) *without*
    /// zeroing it first — the caller owns `out`'s prior contents, which is
    /// the whole point of the non-allocating form: a caller that knows
    /// `out` is already zero (say, because it cleared exactly the entries
    /// it scattered last time via [`Self::unscatter_from`]) skips an
    /// `O(len)` fill per call.
    #[inline]
    pub fn scatter_into(&self, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.len);
        for &(i, v) in &self.entries {
            out[i] = v;
        }
    }

    /// `out += alpha * self`, entrywise over this vector's support.
    #[inline]
    pub fn scatter_add_into(&self, alpha: f64, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.len);
        for &(i, v) in &self.entries {
            out[i] += alpha * v;
        }
    }

    /// Resets exactly this vector's support in `out` back to zero — the
    /// inverse of [`Self::scatter_into`], in `O(nnz)` rather than the
    /// `O(len)` a blanket `fill(0.0)` would cost.
    #[inline]
    pub fn unscatter_from(&self, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.len);
        for &(i, _) in &self.entries {
            out[i] = 0.0;
        }
    }

    /// Replaces this vector's contents with `dense`'s nonzeros (`|v| >
    /// tol`), reusing the existing allocation.
    pub fn gather_from(&mut self, dense: &[f64], tol: f64) {
        self.entries.clear();
        self.len = dense.len();
        for (i, &v) in dense.iter().enumerate() {
            if v.abs() > tol {
                self.entries.push((i, v));
            }
        }
    }

    /// `self . dense` — `O(nnz)`, touching only this vector's support.
    #[inline]
    pub fn dot_dense(&self, dense: &[f64]) -> f64 {
        debug_assert_eq!(dense.len(), self.len);
        self.entries.iter().map(|&(i, v)| v * dense[i]).sum()
    }

    /// Scales every stored value by `k`.
    #[inline]
    pub fn scale(&mut self, k: f64) {
        for e in self.entries.iter_mut() {
            e.1 *= k;
        }
    }

    /// Euclidean norm, over the stored support.
    pub fn norm2(&self) -> f64 {
        self.entries.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt()
    }
}

/// `sparse . dense` for a bare entry slice — the same product
/// [`SparseVec::dot_dense`] computes, for the many call sites that hold a
/// `&[(usize, f64)]` straight out of [`CsrMat::row`]/[`CscMat::col`] and
/// have no reason to wrap it in a [`SparseVec`] first.
#[inline]
pub fn sparse_dot_dense(sparse: &[(usize, f64)], dense: &[f64]) -> f64 {
    sparse.iter().map(|&(i, v)| v * dense[i]).sum()
}

/// `dense += alpha * sparse`, over `sparse`'s support only.
#[inline]
pub fn sparse_axpy_dense(alpha: f64, sparse: &[(usize, f64)], dense: &mut [f64]) {
    for &(i, v) in sparse {
        dense[i] += alpha * v;
    }
}

/// Densifies `sparse` into `out` (length `out.len()`), zeroing `out`
/// first. `O(len + nnz)` — the "I need this column as a dense vector"
/// primitive, kept here so the several call sites that used to open-code
/// a zero-fill plus a scatter share one implementation.
#[inline]
pub fn scatter_dense(sparse: &[(usize, f64)], out: &mut [f64]) {
    out.iter_mut().for_each(|v| *v = 0.0);
    for &(i, v) in sparse {
        out[i] = v;
    }
}

// ===========================================================================
// Hybrid sparse/dense vectors
// ===========================================================================

/// A vector held **either** as an `(index, value)` list **or** as a full
/// dense array, the representation picked once at construction from its own
/// fill — [`HybridVec::pack`].
///
/// # Why a second vector type
///
/// [`SparseVec`] is the right shape for data that is sparse and stays
/// sparse. The vectors `simplex::lu`'s Forrest-Tomlin eta file is made of
/// are not that: an eta starts sparse and fills in as updates accumulate,
/// and on a dense-coefficient LP `U`'s chain is close to fully dense from
/// the very first update. Past some fill the `(index, value)` form loses:
/// the same total FLOP count costs a per-entry tuple load and an indexed
/// indirection, where a straight-line scan over a dense array vectorizes.
/// So the eta file wants *both* forms behind one interface — which is what
/// this is.
///
/// Each consumer of an eta is one of exactly two loops, a dot product
/// against a dense vector ([`Self::dot_dense`]) or an axpy into one
/// ([`Self::axpy_into_dense`]), and before this type existed each of those
/// was written out per call site as a two-armed `match` on the
/// representation — eight copies across FTRAN, BTRAN and both capture
/// variants, every one of which had to be edited in lockstep to change
/// anything. They are these two methods now.
///
/// # The skipped index
///
/// An eta's own pivot slot is excluded from its off-diagonal vector. The
/// sparse form does that by simply not storing it; the **dense form stores
/// a `0.0` there**, so that both loops can run over the whole array with no
/// per-entry test for "is this the slot itself". [`Self::pack`] upholds
/// that automatically (the dense buffer starts all-zero and only the given
/// pairs are written), and [`Self::remove_index`] preserves it.
///
/// `nnz` is tracked explicitly rather than re-derived from the dense
/// array's length — which is always the full length and says nothing about
/// fill — so `simplex::lu`'s refactorization triggers keep measuring true
/// fill regardless of which representation an eta happens to be in.
#[derive(Clone, Debug)]
pub enum HybridVec {
    Sparse(Vec<(usize, f64)>),
    Dense { data: Box<[f64]>, nnz: usize },
}

impl HybridVec {
    /// Wraps `pairs` (indices `< len`, none of them repeated), switching to
    /// the dense form once `pairs.len()` exceeds `dense_fraction * len`.
    pub fn pack(len: usize, pairs: Vec<(usize, f64)>, dense_fraction: f64) -> Self {
        let nnz = pairs.len();
        if nnz as f64 > dense_fraction * len as f64 {
            let mut data = vec![0.0; len];
            for (i, v) in pairs {
                data[i] = v;
            }
            HybridVec::Dense { data: data.into_boxed_slice(), nnz }
        } else {
            HybridVec::Sparse(pairs)
        }
    }

    /// Builds exactly the vector [`Self::pack`] would from the pair list
    /// `{ (i, scale * src[i]) : i != skip, scale * src[i] != 0.0 }`,
    /// **without ever materializing that list**.
    ///
    /// Both of the vectors `simplex::lu`'s Forrest-Tomlin update creates on
    /// every pivot commit are of precisely this shape, each read straight
    /// out of a dense buffer the caller already has: the replacement
    /// column's off-diagonal part (`src = a_tilde`, `scale = 1.0`) and the
    /// new `R` eta (`src = e_tilde`, `scale = -old_pivot`). Both used to be
    /// `collect()`ed into a temporary `Vec<(usize, f64)>` only to be handed
    /// straight to [`Self::pack`], which then either kept that `Vec` (sparse
    /// arm) or scattered it into a freshly allocated dense array and dropped
    /// it (dense arm) — two throwaway heap allocations per update on a path
    /// `ENOMOTO_PROF_PHASES_EXT` measures at 13-20% of wall time, the last
    /// ones left there after `try_update`'s own two scratch buffers
    /// (see [`crate::simplex::lu::FtLu`]'s `scratch_a_tilde`) removed the rest.
    ///
    /// Going through the dense source directly removes both: the sparse arm
    /// allocates once at the exact final length (where `collect()` pays for
    /// geometric regrowth, a filtered iterator being able to report only an
    /// upper bound), and the dense arm allocates only the array it returns,
    /// filling it by copying `src` through rather than by scattering pairs
    /// into it.
    ///
    /// `nnz` is counted in a separate first pass because the
    /// sparse-or-dense decision needs it *before* either arm can start;
    /// that pass is a straight-line scan of a contiguous `f64` array with
    /// `skip` corrected for afterwards instead of tested for inside the
    /// loop, which is why reading `src` twice still costs less than the one
    /// predicated `collect()` it replaces.
    ///
    /// Panics if `skip >= src.len()`, exactly as [`Self::pack`] does on an
    /// out-of-range index.
    pub fn pack_scaled_dense(src: &[f64], skip: usize, scale: f64, dense_fraction: f64) -> Self {
        let len = src.len();
        let mut nnz = 0usize;
        for &x in src {
            nnz += usize::from(scale * x != 0.0);
        }
        // `skip` is excluded from the vector, so undo its contribution
        // rather than branching on it `len` times above. It was counted
        // if and only if this same test holds, so this cannot underflow.
        nnz -= usize::from(scale * src[skip] != 0.0);

        if nnz as f64 > dense_fraction * len as f64 {
            let mut data = src.to_vec();
            if scale != 1.0 {
                for d in data.iter_mut() {
                    *d *= scale;
                }
            }
            // Upholds "the skipped index" convention documented above:
            // the dense form stores a literal `0.0` at its own pivot slot.
            data[skip] = 0.0;
            HybridVec::Dense { data: data.into_boxed_slice(), nnz }
        } else {
            let mut pairs = Vec::with_capacity(nnz);
            for (i, &x) in src.iter().enumerate() {
                let v = scale * x;
                if i != skip && v != 0.0 {
                    pairs.push((i, v));
                }
            }
            HybridVec::Sparse(pairs)
        }
    }

    /// True stored-nonzero count, in either representation.
    #[inline]
    pub fn nnz(&self) -> usize {
        match self {
            HybridVec::Sparse(v) => v.len(),
            HybridVec::Dense { nnz, .. } => *nnz,
        }
    }

    /// Drops any entry at `index`, keeping `nnz` honest in the dense form
    /// rather than leaving it stale. Returns whether an entry was actually
    /// there — which lets a caller maintaining its own running total of
    /// fill across many of these adjust it without re-summing.
    pub fn remove_index(&mut self, index: usize) -> bool {
        match self {
            HybridVec::Sparse(v) => {
                let before = v.len();
                v.retain(|&(i, _)| i != index);
                v.len() != before
            }
            HybridVec::Dense { data, nnz } => {
                if data[index] != 0.0 {
                    data[index] = 0.0;
                    *nnz -= 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Calls `f` once with each of this vector's occupied indices, without
    /// materializing the values.
    ///
    /// Takes a closure rather than returning an iterator because its one
    /// caller — `simplex::lu`'s Forrest-Tomlin update, once per pivot
    /// commit — has to walk both representations and there is no single
    /// concrete iterator type spanning them. The `Box<dyn Iterator>` this
    /// replaces bought that with a heap allocation per update plus a
    /// virtual call per index, on a path that runs every simplex
    /// iteration; a generic closure monomorphizes into each arm instead,
    /// so both loops inline and nothing is allocated.
    ///
    /// The dense arm still scans the whole array (it has no index list to
    /// walk), exactly as before.
    #[inline]
    pub fn for_each_index(&self, mut f: impl FnMut(usize)) {
        match self {
            HybridVec::Sparse(v) => {
                for &(i, _) in v {
                    f(i);
                }
            }
            HybridVec::Dense { data, .. } => {
                for (i, &x) in data.iter().enumerate() {
                    if x != 0.0 {
                        f(i);
                    }
                }
            }
        }
    }

    /// Materializes the stored entries as owned `(index, value)` pairs.
    /// The dense arm is `O(len)`, so this is for cold paths only — not the
    /// per-iteration loops, which are the two methods below.
    pub fn to_pairs(&self) -> Vec<(usize, f64)> {
        match self {
            HybridVec::Sparse(v) => v.clone(),
            HybridVec::Dense { data, .. } => data.iter().enumerate().filter(|&(_, &v)| v != 0.0).map(|(i, &v)| (i, v)).collect(),
        }
    }

    /// `self . dense`. The dense arm zips the whole array, which is
    /// correct precisely because of the skipped-index convention above:
    /// the stored `0.0` contributes nothing.
    #[inline]
    pub fn dot_dense(&self, dense: &[f64]) -> f64 {
        match self {
            HybridVec::Sparse(v) => v.iter().map(|&(i, v)| v * dense[i]).sum(),
            HybridVec::Dense { data, .. } => data.iter().zip(dense.iter()).map(|(&v, &d)| v * d).sum(),
        }
    }

    /// `dense += alpha * self`, over this vector's support (the dense arm
    /// over the whole array — again a no-op at the skipped index).
    #[inline]
    pub fn axpy_into_dense(&self, alpha: f64, dense: &mut [f64]) {
        match self {
            HybridVec::Sparse(v) => {
                for &(i, v) in v {
                    dense[i] += alpha * v;
                }
            }
            HybridVec::Dense { data, .. } => {
                for (d, &v) in dense.iter_mut().zip(data.iter()) {
                    *d += alpha * v;
                }
            }
        }
    }
}

// ===========================================================================
// Epoch-stamped marks
// ===========================================================================

/// A reusable "was this index touched during the current pass" set, cleared
/// in `O(1)` by bumping a counter instead of rewriting the array.
///
/// The standard epoch-stamp (time-stamp) trick, and the reason it is worth
/// a named type here is that this crate had grown **three** independent
/// copies of it — [`SparseAccum`]'s own occupancy map,
/// `simplex::lu::GpScratch`'s Gilbert-Peierls visited set, and
/// `simplex::lu::FtLu`'s `U^T`-solve "needed" set — differing in exactly
/// the way three hand-rolled copies of one idea differ: only one of the
/// three handled counter wraparound, and the other two were silently
/// wrong (a stale stamp from 2^32 passes ago reading as a live mark) if a
/// solve ever ran long enough to wrap. [`Self::begin`] handles it once,
/// for all of them.
///
/// `u32` stamps rather than `u64`: these arrays are length `m` and are
/// indexed in the innermost loop of every FTRAN/BTRAN, so halving their
/// cache footprint matters more than never needing the wraparound branch
/// (which costs one predictable compare per *pass*, not per index).
#[derive(Clone)]
pub struct EpochMarks {
    stamps: Vec<u32>,
    epoch: u32,
}

impl EpochMarks {
    /// Marks over index space `0..n`, all clear.
    ///
    /// `epoch` starts at 1, not 0: `stamps` is zero-initialized, so at
    /// epoch 0 every index would read as already marked before the first
    /// [`Self::begin`] ever ran.
    pub fn new(n: usize) -> Self {
        EpochMarks { stamps: vec![0; n], epoch: 1 }
    }

    /// The index space size.
    #[inline]
    pub fn len(&self) -> usize {
        self.stamps.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.stamps.is_empty()
    }

    /// Clears every mark and starts a new pass. `O(1)` except on the one
    /// pass in 2^32 that wraps the counter, which pays a single `O(n)`
    /// reset — without it, stamps left by the pass 2^32 ago would read as
    /// marks belonging to this one.
    #[inline]
    pub fn begin(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.stamps.iter_mut().for_each(|s| *s = 0);
            self.epoch = 1;
        }
    }

    #[inline]
    pub fn mark(&mut self, i: usize) {
        self.stamps[i] = self.epoch;
    }

    #[inline]
    pub fn is_marked(&self, i: usize) -> bool {
        self.stamps[i] == self.epoch
    }

    /// Test-only: drives the epoch counter to an arbitrary value, so the
    /// wraparound branch in [`Self::begin`] can be exercised without
    /// actually running 2^32 passes.
    #[cfg(test)]
    pub fn force_epoch_for_test(&mut self, epoch: u32) {
        self.epoch = epoch;
    }

    /// Clears index `i`'s mark for the rest of this pass. Used by
    /// [`SparseAccum::remove`], the one consumer that needs to take a mark
    /// back rather than only ever setting it.
    #[inline]
    pub fn unmark(&mut self, i: usize) {
        // Any value that is not the live epoch reads as unmarked; the
        // previous epoch is the one such value guaranteed not to collide
        // with a future one before the next `begin`.
        self.stamps[i] = self.epoch.wrapping_sub(1);
    }
}

// ===========================================================================
// Sparse accumulator (SPA)
// ===========================================================================

/// A **sparse accumulator**: a reusable dense scratch buffer plus an
/// epoch-stamped occupancy map, for building one sparse vector out of a
/// handful of others in `O(total nnz)`.
///
/// # Why this exists
///
/// Every "combine two sparse rows" site in `presolve` used to be written
/// as a `BTreeMap<usize, f64>` built per output row — `freevar::axpy_row`,
/// `aggregator::axpy_row`, `doubleton::rewrite_row`, `sparsify`'s own
/// target merge. That is `O(nnz log nnz)` with a pointer-chasing node per
/// entry, for an operation whose natural cost is `O(nnz)` flat array
/// writes; on a wide Netlib row (`wood1p`'s 2592-nonzero rows, say) the
/// map's allocation churn dominates the arithmetic outright.
///
/// The classical fix — Gilbert/Moler/Schreiber's sparse accumulator, the
/// same structure `simplex::lu::GpScratch` already uses for its reach-set
/// bookkeeping — keeps one `O(n)` value array alive across *all* the merges
/// and uses a monotone epoch counter to tell "written this merge" from
/// "left over from a previous one", so nothing has to be cleared between
/// merges either: the cost of a merge is proportional to what it actually
/// touches, not to `n`. The one `O(n)` price is paid once, at
/// construction, per pass.
///
/// # Determinism
///
/// [`Self::take_sorted`] sorts the accumulated pattern before emitting, so
/// the result is *byte-identical* to what the `BTreeMap` versions produced
/// — which matters, since this crate deliberately prefers ordered maps
/// wherever iteration order can feed back into a later tie-break (see
/// `types.rs::LinearExpr`). The accumulation order itself is the caller's
/// call order, so floating-point results are identical too, not merely
/// equivalent.
pub struct SparseAccum {
    values: Vec<f64>,
    marks: EpochMarks,
    pattern: Vec<usize>,
}

impl SparseAccum {
    /// An accumulator over index space `0..n`. One `O(n)` allocation,
    /// meant to be hoisted out of whatever loop does the merging.
    pub fn new(n: usize) -> Self {
        SparseAccum { values: vec![0.0; n], marks: EpochMarks::new(n), pattern: Vec::new() }
    }

    /// The index-space size this accumulator was built for.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.values.len()
    }

    /// Begins a fresh accumulation. `O(1)`: the epoch bump invalidates
    /// every stale entry at once, so no buffer is cleared here.
    #[inline]
    pub fn reset(&mut self) {
        self.marks.begin();
        self.pattern.clear();
    }

    #[inline]
    fn live(&self, i: usize) -> bool {
        self.marks.is_marked(i)
    }

    /// `self[i] += v`, registering `i` in the pattern on first touch.
    #[inline]
    pub fn add(&mut self, i: usize, v: f64) {
        debug_assert!(i < self.values.len(), "sparse index out of range");
        if self.live(i) {
            self.values[i] += v;
        } else {
            self.marks.mark(i);
            self.values[i] = v;
            self.pattern.push(i);
        }
    }

    /// `self[i] = v`, last write winning — the semantics a
    /// `BTreeMap::from_iter` over a possibly-duplicated entry list has,
    /// which is what [`Self::load`] needs to stay faithful to the merge
    /// code this replaced.
    #[inline]
    pub fn set(&mut self, i: usize, v: f64) {
        debug_assert!(i < self.values.len(), "sparse index out of range");
        if !self.live(i) {
            self.marks.mark(i);
            self.pattern.push(i);
        }
        self.values[i] = v;
    }

    /// The current value at `i` (`0.0` if untouched this accumulation).
    #[inline]
    pub fn get(&self, i: usize) -> f64 {
        if self.live(i) {
            self.values[i]
        } else {
            0.0
        }
    }

    /// Whether `i` is in the accumulated pattern — a membership test that
    /// replaces the `BTreeMap::contains_key`/`BTreeSet` lookups the
    /// subset checks in `sparsify`/`aggregator` used to do.
    #[inline]
    pub fn contains(&self, i: usize) -> bool {
        self.live(i)
    }

    /// Forces `i` out of the pattern (a `BTreeMap::remove`): the value is
    /// zeroed and the index stays in `pattern` but is filtered out on
    /// emit. Used for the "drop the eliminated column outright rather than
    /// trusting floating-point cancellation to zero it" step every
    /// elimination pass performs.
    #[inline]
    pub fn remove(&mut self, i: usize) {
        if self.live(i) {
            self.marks.unmark(i);
            self.values[i] = 0.0;
        }
    }

    /// Starts an accumulation from `entries` (last value wins per index,
    /// matching `BTreeMap::from_iter`).
    pub fn load(&mut self, entries: &[(usize, f64)]) {
        self.reset();
        for &(i, v) in entries {
            self.set(i, v);
        }
    }

    /// `self += alpha * entries`.
    pub fn axpy(&mut self, alpha: f64, entries: &[(usize, f64)]) {
        for &(i, v) in entries {
            self.add(i, alpha * v);
        }
    }

    /// Emits the accumulation as an index-sorted entry list, dropping
    /// everything within `tol` of zero (pass `0.0` to drop exact zeros
    /// only, or `f64::NEG_INFINITY` to keep every stored entry). The
    /// accumulator is left ready for the next [`Self::load`]/[`Self::reset`].
    pub fn take_sorted(&mut self, tol: f64) -> Vec<(usize, f64)> {
        self.pattern.sort_unstable();
        let mut out: Vec<(usize, f64)> = Vec::with_capacity(self.pattern.len());
        for &i in &self.pattern {
            if !self.marks.is_marked(i) {
                continue; // dropped via `remove`
            }
            if out.last().map(|&(k, _)| k) == Some(i) {
                continue; // `remove` then re-`add` can enqueue `i` twice
            }
            let v = self.values[i];
            if v.abs() > tol {
                out.push((i, v));
            }
        }
        out
    }

    /// [`Self::take_sorted`] wrapped in a [`SparseVec`] of logical length
    /// [`Self::capacity`].
    pub fn take_sorted_vec(&mut self, tol: f64) -> SparseVec {
        let n = self.capacity();
        SparseVec::from_entries(n, self.take_sorted(tol))
    }
}

/// `row - factor * pivot`, with `drop_col` removed outright and every
/// other entry within `tol` of zero dropped; the result is index-sorted.
///
/// This is the row-elimination kernel `presolve`'s substitution passes
/// (`freevar`, `aggregator`) each used to carry their own `BTreeMap` copy
/// of. Dropping `drop_col` explicitly rather than trusting the
/// subtraction to cancel it to exactly zero is deliberate and load-bearing
/// — `factor` is chosen to annihilate that column, but in floating point
/// "annihilate" means "to within a rounding error", and leaving a 1e-18
/// coefficient behind would keep the variable structurally present.
///
/// `accum` is the caller's hoisted [`SparseAccum`] (index space must cover
/// every column either row references); it is reset on entry, so its prior
/// contents are irrelevant.
pub fn axpy_row(accum: &mut SparseAccum, row: &[(usize, f64)], pivot: &[(usize, f64)], factor: f64, drop_col: usize, tol: f64) -> Vec<(usize, f64)> {
    accum.load(row);
    accum.axpy(-factor, pivot);
    accum.remove(drop_col);
    accum.take_sorted(tol)
}

// ===========================================================================
// CSR / CSC
// ===========================================================================

/// The shared physical form behind [`CsrMat`] and [`CscMat`]: a fixed,
/// single-allocation-pair ragged array of `(index, value)` pairs.
///
/// Kept generic over which axis it indexes rather than duplicated per
/// orientation — CSR and CSC differ only in what "outer"/"inner" name, and
/// every structural operation here (build, transpose, slice) is written
/// once against that vocabulary. The two public wrappers add the axis
/// names (`row`/`col`), the matrix dimensions, and the arithmetic, which
/// *is* orientation-specific.
///
/// Unlike `Vec<Vec<(usize, f64)>>` (one heap allocation per outer index,
/// each grown by repeated `push`), this is exactly two allocations total —
/// `offsets` and `entries` — built once and never mutated afterward.
#[derive(Clone, Debug, PartialEq)]
pub struct Compressed {
    offsets: Vec<usize>,
    entries: Vec<(usize, f64)>,
}

impl Compressed {
    /// Flattens `outers` (already grouped by outer index) into one
    /// `entries` buffer, `offsets[k]..offsets[k+1]` marking outer index
    /// `k`'s slice. Entries are copied verbatim — no sorting, no merging,
    /// no zero-dropping.
    fn from_groups(outers: &[Vec<(usize, f64)>]) -> Self {
        let mut offsets = Vec::with_capacity(outers.len() + 1);
        offsets.push(0);
        let mut entries = Vec::with_capacity(outers.iter().map(|r| r.len()).sum());
        for outer in outers {
            entries.extend_from_slice(outer);
            offsets.push(entries.len());
        }
        Compressed { offsets, entries }
    }

    /// Builds the transpose of `outers` (`n_inner` = the inner-axis size,
    /// i.e. this call's own outer count) directly into the flat layout via
    /// a counting sort — one pass to size each new outer index's slice,
    /// one pass to fill it — rather than transposing into `n_inner`
    /// separate growable `Vec`s first and flattening those second.
    ///
    /// Within each output slice the entries come out in ascending
    /// *source* outer order, since the fill pass walks `outers` in order;
    /// that is what makes a CSR whose rows are column-sorted transpose
    /// into a CSC whose columns are row-sorted, and vice versa.
    fn from_groups_transposed(outers: &[Vec<(usize, f64)>], n_inner: usize) -> Self {
        let mut offsets = vec![0usize; n_inner + 1];
        for outer in outers {
            for &(j, _) in outer {
                offsets[j + 1] += 1;
            }
        }
        for k in 0..n_inner {
            offsets[k + 1] += offsets[k];
        }
        let mut entries = vec![(0usize, 0.0f64); offsets[n_inner]];
        let mut cursor = offsets.clone();
        for (i, outer) in outers.iter().enumerate() {
            for &(j, v) in outer {
                entries[cursor[j]] = (i, v);
                cursor[j] += 1;
            }
        }
        Compressed { offsets, entries }
    }

    /// Transposes an already-compressed form in place of a rebuild — the
    /// same counting sort as [`Self::from_groups_transposed`], reading
    /// from the flat layout instead of from ragged `Vec`s. This is what
    /// makes CSR <-> CSC conversion `O(nnz + n_inner)` with no
    /// intermediate `Vec<Vec<_>>`.
    fn transposed(&self, n_inner: usize) -> Self {
        let mut offsets = vec![0usize; n_inner + 1];
        for &(j, _) in &self.entries {
            offsets[j + 1] += 1;
        }
        for k in 0..n_inner {
            offsets[k + 1] += offsets[k];
        }
        let mut entries = vec![(0usize, 0.0f64); offsets[n_inner]];
        let mut cursor = offsets.clone();
        for i in 0..self.outer_len() {
            for &(j, v) in self.group(i) {
                entries[cursor[j]] = (i, v);
                cursor[j] += 1;
            }
        }
        Compressed { offsets, entries }
    }

    #[inline]
    fn outer_len(&self) -> usize {
        self.offsets.len() - 1
    }

    #[inline]
    fn group(&self, k: usize) -> &[(usize, f64)] {
        &self.entries[self.offsets[k]..self.offsets[k + 1]]
    }

    #[inline]
    fn nnz(&self) -> usize {
        self.entries.len()
    }

    fn to_groups(&self) -> Vec<Vec<(usize, f64)>> {
        (0..self.outer_len()).map(|k| self.group(k).to_vec()).collect()
    }
}

/// A matrix in **compressed sparse row** form: row `i`'s nonzeros are
/// `(column, value)` pairs, contiguous in one shared buffer.
///
/// The natural orientation for everything that walks a constraint matrix a
/// row at a time — activity bounds, row-singleton and doubleton detection,
/// `y^T A` accumulation, and `simplex`'s own basis-row extraction.
#[derive(Clone, Debug, PartialEq)]
pub struct CsrMat {
    n_rows: usize,
    n_cols: usize,
    inner: Compressed,
}

/// A matrix in **compressed sparse column** form: column `j`'s nonzeros
/// are `(row, value)` pairs, contiguous in one shared buffer.
///
/// The natural orientation for everything that walks one *column* at a
/// time — the simplex pricing loop's `a_j . y` dot products, the entering
/// column's FTRAN right-hand side, column-singleton and dominated-column
/// detection, and dual bound propagation over a column's own terms. The
/// simplex keeps this alongside the [`CsrMat`] of the same matrix
/// precisely so those loops never have to scan every row looking for
/// column `j`.
#[derive(Clone, Debug, PartialEq)]
pub struct CscMat {
    n_rows: usize,
    n_cols: usize,
    inner: Compressed,
}

impl CsrMat {
    /// Builds from a dense list of sparse rows, each `(column, value)`
    /// pairs. Entries are taken verbatim: this does not sort, merge
    /// duplicates, or drop zeros — see [`Self::from_rows_canonical`] when
    /// the input might contain any of those.
    pub fn from_rows(rows: &[Vec<(usize, f64)>], n_cols: usize) -> Self {
        CsrMat { n_rows: rows.len(), n_cols, inner: Compressed::from_groups(rows) }
    }

    /// Like [`Self::from_rows`], but canonicalizing each row first: sorted
    /// by column, duplicate columns summed, and anything within `tol` of
    /// zero dropped.
    pub fn from_rows_canonical(rows: &[Vec<(usize, f64)>], n_cols: usize, tol: f64) -> Self {
        let canon: Vec<Vec<(usize, f64)>> = rows
            .iter()
            .map(|r| {
                let mut v = SparseVec::from_entries(n_cols, r.clone());
                v.canonicalize(tol);
                v.into_entries()
            })
            .collect();
        CsrMat::from_rows(&canon, n_cols)
    }

    /// Builds from `(row, col, value)` triplets. Entries land in each
    /// row in triplet order; duplicates are kept as-is.
    pub fn from_triplets(n_rows: usize, n_cols: usize, triplets: &[(usize, usize, f64)]) -> Self {
        let mut rows = vec![Vec::new(); n_rows];
        for &(i, j, v) in triplets {
            rows[i].push((j, v));
        }
        CsrMat::from_rows(&rows, n_cols)
    }

    /// Reads a faer [`Csr`] into this crate's own row-major form.
    pub fn from_faer(mat: &Csr) -> Self {
        let r = mat.as_ref();
        CsrMat::from_rows(&csr_rows(mat), r.ncols())
    }

    /// Hands the matrix back to faer (for `interior_point::kkt`, whose
    /// downstream Cholesky is faer's). Exact zeros are dropped, since
    /// faer's triplet builder has no use for them.
    pub fn to_faer(&self) -> Csr {
        csr_from_rows(&self.to_rows(), self.n_cols)
    }

    #[inline]
    pub fn n_rows(&self) -> usize {
        self.n_rows
    }

    #[inline]
    pub fn n_cols(&self) -> usize {
        self.n_cols
    }

    #[inline]
    pub fn nnz(&self) -> usize {
        self.inner.nnz()
    }

    /// Row `i`'s `(column, value)` pairs — a plain slice into the shared
    /// flat buffer, no per-call allocation.
    #[inline]
    pub fn row(&self, i: usize) -> &[(usize, f64)] {
        self.inner.group(i)
    }

    /// Row `i` as an owned [`SparseVec`] over the column space.
    pub fn row_vec(&self, i: usize) -> SparseVec {
        SparseVec::from_entries(self.n_cols, self.row(i).to_vec())
    }

    /// Every row as a ragged `Vec<Vec<_>>` — the mutable form the
    /// presolve passes rewrite in.
    pub fn to_rows(&self) -> Vec<Vec<(usize, f64)>> {
        self.inner.to_groups()
    }

    /// The same matrix in column-major form: `O(nnz + n_cols)` counting
    /// sort, no intermediate ragged `Vec`. Each output column's entries
    /// come out in ascending row order.
    pub fn to_csc(&self) -> CscMat {
        CscMat { n_rows: self.n_rows, n_cols: self.n_cols, inner: self.inner.transposed(self.n_cols) }
    }

    /// `A^T` as a [`CsrMat`] — structurally the same reordering
    /// [`Self::to_csc`] performs, relabelled: the CSC of `A` and the CSR
    /// of `A^T` hold identical bytes.
    pub fn transpose(&self) -> CsrMat {
        CsrMat { n_rows: self.n_cols, n_cols: self.n_rows, inner: self.inner.transposed(self.n_cols) }
    }

    /// Per-column nonzero counts, in one `O(nnz)` pass — what the
    /// column-singleton and aggregator passes need before they can pick
    /// candidates, without building the whole transpose.
    pub fn col_counts(&self) -> Vec<usize> {
        let mut counts = vec![0usize; self.n_cols];
        for i in 0..self.n_rows {
            for &(j, _) in self.row(i) {
                counts[j] += 1;
            }
        }
        counts
    }

    /// Writes `A * x` into `out` (length `n_rows`). No allocation; one
    /// independent output entry per row, so it parallelizes (see the
    /// module docs).
    pub fn mat_vec_into(&self, x: &[f64], out: &mut [f64]) {
        debug_assert_eq!(x.len(), self.n_cols);
        debug_assert_eq!(out.len(), self.n_rows);
        out.par_iter_mut().enumerate().for_each(|(i, o)| {
            *o = sparse_dot_dense(self.row(i), x);
        });
    }

    /// `A * x` as a fresh `Vec`.
    pub fn mat_vec(&self, x: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.n_rows];
        self.mat_vec_into(x, &mut out);
        out
    }

    /// Writes `A^T * y` into `out` (length `n_cols`). A scatter over
    /// `out`, hence sequential — see the module docs.
    pub fn mat_t_vec_into(&self, y: &[f64], out: &mut [f64]) {
        debug_assert_eq!(y.len(), self.n_rows);
        debug_assert_eq!(out.len(), self.n_cols);
        out.iter_mut().for_each(|v| *v = 0.0);
        for i in 0..self.n_rows {
            let yi = y[i];
            if yi == 0.0 {
                continue;
            }
            sparse_axpy_dense(yi, self.row(i), out);
        }
    }

    /// `A^T * y` as a fresh `Vec`.
    pub fn mat_t_vec(&self, y: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.n_cols];
        self.mat_t_vec_into(y, &mut out);
        out
    }

    /// `A^T * y` for a **sparse** `y`, accumulated straight into `accum`
    /// — `O(sum of nnz over y's support rows)`, never touching a row `y`
    /// is zero on. This is the row-major matrix's genuinely sparse
    /// product: only the rows `y` selects contribute, and only the columns
    /// those rows occupy appear in the result.
    pub fn mat_t_vec_sparse(&self, y: &[(usize, f64)], accum: &mut SparseAccum, tol: f64) -> SparseVec {
        accum.reset();
        for &(i, yi) in y {
            if yi == 0.0 {
                continue;
            }
            accum.axpy(yi, self.row(i));
        }
        SparseVec::from_entries(self.n_cols, accum.take_sorted(tol))
    }
}

impl CscMat {
    /// Builds from a dense list of sparse columns, each `(row, value)`
    /// pairs, verbatim.
    pub fn from_cols(cols: &[Vec<(usize, f64)>], n_rows: usize) -> Self {
        CscMat { n_rows, n_cols: cols.len(), inner: Compressed::from_groups(cols) }
    }

    /// Builds the column-major form *directly* from row-major input, via
    /// one counting-sort transpose — the path that avoids ever
    /// materializing `n_cols` growable `Vec`s just to flatten them again.
    /// This is how `simplex`'s `StdForm` gets its column view, and how the
    /// presolve passes that need per-column access build theirs.
    pub fn from_rows(rows: &[Vec<(usize, f64)>], n_cols: usize) -> Self {
        CscMat { n_rows: rows.len(), n_cols, inner: Compressed::from_groups_transposed(rows, n_cols) }
    }

    /// Builds the column-major form from an arbitrary **entry stream**:
    /// `emit` is called twice — once to size each column's slice, once to
    /// fill it — handing the caller a `push(row, col, value)` sink both
    /// times.
    ///
    /// This exists for the passes whose matrix is not one contiguous
    /// `Vec<Vec<_>>` to begin with: `presolve`'s dual reductions and
    /// parallel-column detection both need the column view of `A`'s rows
    /// *concatenated with* a separate list of inequality rows, under one
    /// combined row numbering. Written directly, that is `vec![Vec::new();
    /// n_cols]` plus a `push` per entry — one heap allocation per column,
    /// each then grown by reallocation — for a structure that is read-only
    /// the moment it is finished. Streamed through here it is the usual two
    /// allocations and one counting sort, with no intermediate
    /// concatenation of the blocks either.
    ///
    /// `emit` must produce **exactly the same entries in the same order**
    /// on both calls (it is a pure enumeration of the matrix, so this is
    /// the natural way to write it); a caller that filters entries must
    /// apply the same filter both times. Each column's entries come out in
    /// emission order, so emitting row by row yields row-ascending columns.
    pub fn from_entry_stream<F>(n_rows: usize, n_cols: usize, mut emit: F) -> Self
    where
        F: FnMut(&mut dyn FnMut(usize, usize, f64)),
    {
        let mut offsets = vec![0usize; n_cols + 1];
        emit(&mut |_i, j, _v| offsets[j + 1] += 1);
        for k in 0..n_cols {
            offsets[k + 1] += offsets[k];
        }
        let mut entries = vec![(0usize, 0.0f64); offsets[n_cols]];
        let mut cursor = offsets.clone();
        emit(&mut |i, j, v| {
            entries[cursor[j]] = (i, v);
            cursor[j] += 1;
        });
        CscMat { n_rows, n_cols, inner: Compressed { offsets, entries } }
    }

    /// An all-zero `n_rows x n_cols` matrix — every column present and
    /// empty.
    pub fn empty(n_rows: usize, n_cols: usize) -> Self {
        CscMat { n_rows, n_cols, inner: Compressed { offsets: vec![0; n_cols + 1], entries: Vec::new() } }
    }

    /// Reads a faer [`Csr`] straight into column-major form.
    pub fn from_faer(mat: &Csr) -> Self {
        let r = mat.as_ref();
        CscMat::from_rows(&csr_rows(mat), r.ncols())
    }

    #[inline]
    pub fn n_rows(&self) -> usize {
        self.n_rows
    }

    #[inline]
    pub fn n_cols(&self) -> usize {
        self.n_cols
    }

    #[inline]
    pub fn nnz(&self) -> usize {
        self.inner.nnz()
    }

    /// Column `j`'s `(row, value)` pairs — a plain slice into the shared
    /// flat buffer, no per-call allocation. Every per-candidate-column
    /// loop that only ever computes a dot product against column `j`
    /// (entering-variable selection, the dual method's `chuzc`,
    /// steepest-edge weight updates) uses this rather than densifying:
    /// those loops run once per nonbasic column *every pivot*, so avoiding
    /// both the `O(m)` fill and ever touching another column's data is
    /// what turns an `O(n_total * nnz)` pivot into an `O(nnz)` one.
    #[inline]
    pub fn col(&self, j: usize) -> &[(usize, f64)] {
        self.inner.group(j)
    }

    /// Column `j` as an owned [`SparseVec`] over the row space.
    pub fn col_vec(&self, j: usize) -> SparseVec {
        SparseVec::from_entries(self.n_rows, self.col(j).to_vec())
    }

    /// Column `j` densified into `out` (length `n_rows`), zeroing it
    /// first — `O(nnz_j + n_rows)`, not a scan of every row looking for
    /// column `j`.
    #[inline]
    pub fn col_into_dense(&self, j: usize, out: &mut [f64]) {
        scatter_dense(self.col(j), out);
    }

    /// Column `j` as a fresh dense `Vec`.
    pub fn col_dense(&self, j: usize) -> Vec<f64> {
        let mut out = vec![0.0; self.n_rows];
        self.col_into_dense(j, &mut out);
        out
    }

    /// Every column as a ragged `Vec<Vec<_>>`.
    pub fn to_cols(&self) -> Vec<Vec<(usize, f64)>> {
        self.inner.to_groups()
    }

    /// Back to row-major: `O(nnz + n_rows)` counting sort, the exact
    /// inverse of [`CsrMat::to_csc`].
    pub fn to_csr(&self) -> CsrMat {
        CsrMat { n_rows: self.n_rows, n_cols: self.n_cols, inner: self.inner.transposed(self.n_rows) }
    }

    /// Per-row nonzero counts, in one `O(nnz)` pass.
    pub fn row_counts(&self) -> Vec<usize> {
        let mut counts = vec![0usize; self.n_rows];
        for j in 0..self.n_cols {
            for &(i, _) in self.col(j) {
                counts[i] += 1;
            }
        }
        counts
    }

    /// Writes `A * x` into `out` (length `n_rows`). A scatter over `out`
    /// for this orientation, hence sequential and zero-skipping on `x`.
    pub fn mat_vec_into(&self, x: &[f64], out: &mut [f64]) {
        debug_assert_eq!(x.len(), self.n_cols);
        debug_assert_eq!(out.len(), self.n_rows);
        out.iter_mut().for_each(|v| *v = 0.0);
        for j in 0..self.n_cols {
            let xj = x[j];
            if xj == 0.0 {
                continue;
            }
            sparse_axpy_dense(xj, self.col(j), out);
        }
    }

    /// `A * x` as a fresh `Vec`.
    pub fn mat_vec(&self, x: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.n_rows];
        self.mat_vec_into(x, &mut out);
        out
    }

    /// `A * x` for a **sparse** `x`: only the columns `x` selects
    /// contribute. This is the column-major matrix's genuinely sparse
    /// product — the `b - N x_N` shape the simplex's right-hand-side
    /// assembly computes.
    pub fn mat_vec_sparse(&self, x: &[(usize, f64)], accum: &mut SparseAccum, tol: f64) -> SparseVec {
        accum.reset();
        for &(j, xj) in x {
            if xj == 0.0 {
                continue;
            }
            accum.axpy(xj, self.col(j));
        }
        SparseVec::from_entries(self.n_rows, accum.take_sorted(tol))
    }

    /// Writes `A^T * y` into `out` (length `n_cols`). One independent
    /// output entry per column, so it parallelizes.
    pub fn mat_t_vec_into(&self, y: &[f64], out: &mut [f64]) {
        debug_assert_eq!(y.len(), self.n_rows);
        debug_assert_eq!(out.len(), self.n_cols);
        out.par_iter_mut().enumerate().for_each(|(j, o)| {
            *o = sparse_dot_dense(self.col(j), y);
        });
    }

    /// `A^T * y` as a fresh `Vec`.
    pub fn mat_t_vec(&self, y: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.n_cols];
        self.mat_t_vec_into(y, &mut out);
        out
    }
}

/// Builds a [`CscMat`] by emitting its columns **in ascending order**, one
/// at a time, appending straight into the final flat buffer.
///
/// [`CscMat::from_rows`] and [`CscMat::from_entry_stream`] both need two
/// passes because they are handed the matrix in the *wrong* orientation
/// (row-major) and have to count each column's size before they can place
/// anything. A producer that already emits column `0`'s entries, then
/// column `1`'s, and so on — which is what a left-looking LU factorization
/// does, one elimination step at a time — needs neither pass: the entry it
/// is holding belongs at the end of the buffer, and the column boundary is
/// wherever the buffer happens to have reached. That is this builder.
///
/// ```text
///     let mut b = CscBuilder::new(n_rows);
///     for j in 0..n_cols {
///         for (i, v) in column_j_entries() { b.push(i, v); }
///         b.end_column();
///     }
///     let mat = b.build();
/// ```
pub struct CscBuilder {
    n_rows: usize,
    offsets: Vec<usize>,
    entries: Vec<(usize, f64)>,
}

impl CscBuilder {
    /// A builder for a matrix with `n_rows` rows and no columns yet.
    pub fn new(n_rows: usize) -> Self {
        CscBuilder { n_rows, offsets: vec![0], entries: Vec::new() }
    }

    /// As [`Self::new`], with room for `n_cols` columns and `nnz` entries
    /// reserved up front.
    pub fn with_capacity(n_rows: usize, n_cols: usize, nnz: usize) -> Self {
        let mut offsets = Vec::with_capacity(n_cols + 1);
        offsets.push(0);
        CscBuilder { n_rows, offsets, entries: Vec::with_capacity(nnz) }
    }

    /// Appends one entry to the column currently being built.
    #[inline]
    pub fn push(&mut self, row: usize, value: f64) {
        debug_assert!(row < self.n_rows, "row index out of range");
        self.entries.push((row, value));
    }

    /// Closes the current column and opens the next one. Must be called
    /// once per column, including for columns with no entries at all.
    #[inline]
    pub fn end_column(&mut self) {
        self.offsets.push(self.entries.len());
    }

    /// How many columns have been closed so far.
    #[inline]
    pub fn columns_built(&self) -> usize {
        self.offsets.len() - 1
    }

    /// Finishes the matrix. Its column count is however many columns were
    /// closed.
    pub fn build(self) -> CscMat {
        CscMat { n_rows: self.n_rows, n_cols: self.offsets.len() - 1, inner: Compressed { offsets: self.offsets, entries: self.entries } }
    }
}

// ===========================================================================
// faer interop
// ===========================================================================

/// faer's row-major sparse matrix — the type `presolve`'s public
/// interfaces and `interior_point::kkt` pass around, since the latter
/// hands it straight to faer's own symbolic/Cholesky machinery. See the
/// module docs for how it divides responsibility with [`CsrMat`].
pub type Csr = faer::sparse::SparseRowMat<usize, f64>;

/// Builds a [`Csr`] from a dense list of sparse rows (each a `(col,
/// value)` list), dropping exact-zero entries. `n_cols` is the matrix's
/// column count; the row count is `rows.len()`.
pub fn csr_from_rows(rows: &[Vec<(usize, f64)>], n_cols: usize) -> Csr {
    // Fast path: rows already column-ascending without duplicates — what
    // every presolve pass hands in almost always — are exactly what the
    // triplet constructor below produces after its sort, so they can be
    // laid out directly in `O(nnz)` instead of paying that sort (and the
    // triplet buffer) on every rebuild. Anything else (unsorted or
    // duplicated columns, which the triplet path sorts and sums) takes the
    // general path unchanged.
    let sorted = rows.iter().all(|row| {
        let mut prev: Option<usize> = None;
        row.iter().filter(|&&(_, v)| v != 0.0).all(|&(j, _)| {
            let ok = j < n_cols && prev.map_or(true, |p| j > p);
            prev = Some(j);
            ok
        })
    });
    if sorted {
        let nnz: usize = rows.iter().map(|row| row.iter().filter(|&&(_, v)| v != 0.0).count()).sum();
        let mut row_ptr = Vec::with_capacity(rows.len() + 1);
        let mut col_ind = Vec::with_capacity(nnz);
        let mut values = Vec::with_capacity(nnz);
        row_ptr.push(0usize);
        for row in rows {
            for &(j, v) in row {
                if v != 0.0 {
                    col_ind.push(j);
                    values.push(v);
                }
            }
            row_ptr.push(col_ind.len());
        }
        let symbolic = faer::sparse::SymbolicSparseRowMat::new_checked(rows.len(), n_cols, row_ptr, None, col_ind);
        return Csr::new(symbolic, values);
    }
    let mut triplets = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 {
                triplets.push((i, j, v));
            }
        }
    }
    Csr::try_new_from_triplets(rows.len(), n_cols, &triplets).expect("valid CSR triplets")
}

/// Row `i` of a faer [`Csr`] as an iterator of `(column, value)` pairs.
///
/// The one place the `col_indices_of_row(i).zip(values_of_row(i))` dance
/// is written. It used to be open-coded at ~40 call sites across
/// `presolve/*`, which is exactly the kind of duplication that lets two of
/// them quietly disagree about whether to filter zeros.
#[inline]
pub fn csr_row_iter(mat: &Csr, i: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
    let r = mat.as_ref();
    r.col_indices_of_row(i).zip(r.values_of_row(i)).map(|(j, &v)| (j, v))
}

/// Row `i` of a faer [`Csr`] as an owned `(column, value)` list.
pub fn csr_row_vec(mat: &Csr, i: usize) -> Vec<(usize, f64)> {
    csr_row_iter(mat, i).collect()
}

/// Every row of a faer [`Csr`] as a ragged `Vec<Vec<_>>` — the mutable
/// working form the presolve passes rewrite in before freezing back
/// through [`csr_from_rows`]. Entries are taken verbatim, explicit zeros
/// included; use [`csr_rows_pruned`] to drop those.
pub fn csr_rows(mat: &Csr) -> Vec<Vec<(usize, f64)>> {
    (0..mat.as_ref().nrows()).map(|i| csr_row_vec(mat, i)).collect()
}

/// [`csr_rows`], with exact-zero entries dropped — what a pass that keys
/// decisions off a row's *support* (row length, singleton/doubleton
/// detection) needs, since a stored zero is not a real nonzero and
/// counting it would misclassify the row.
pub fn csr_rows_pruned(mat: &Csr) -> Vec<Vec<(usize, f64)>> {
    (0..mat.as_ref().nrows()).map(|i| csr_row_iter(mat, i).filter(|&(_, v)| v != 0.0).collect()).collect()
}

/// Per-column nonzero counts of a faer [`Csr`], in one `O(nnz)` pass —
/// without building the transpose.
pub fn csr_col_counts(mat: &Csr) -> Vec<usize> {
    let r = mat.as_ref();
    let mut counts = vec![0usize; r.ncols()];
    for i in 0..r.nrows() {
        for j in r.col_indices_of_row(i) {
            counts[j] += 1;
        }
    }
    counts
}

/// A faer [`Csr`]'s column-major view, built in one counting-sort pass —
/// the replacement for the "walk every row, `push` onto `columns[j]`"
/// loops the presolve passes that need per-column access used to each
/// write for themselves.
pub fn csr_to_csc(mat: &Csr) -> CscMat {
    CscMat::from_faer(mat)
}

/// Writes `mat * x` into `out` (length `mat.nrows()`). No allocation.
pub fn mat_vec_into(mat: &Csr, x: &[f64], out: &mut [f64]) {
    let r = mat.as_ref();
    out.par_iter_mut().enumerate().for_each(|(i, o)| {
        *o = r.col_indices_of_row(i).zip(r.values_of_row(i)).map(|(j, &v)| v * x[j]).sum();
    });
}

/// Writes `mat^T * y` into `out` (length `mat.ncols()`). No allocation.
pub fn mat_t_vec_into(mat: &Csr, y: &[f64], out: &mut [f64]) {
    for v in out.iter_mut() {
        *v = 0.0;
    }
    let r = mat.as_ref();
    for i in 0..r.nrows() {
        let yi = y[i];
        if yi == 0.0 {
            continue;
        }
        for (j, &v) in r.col_indices_of_row(i).zip(r.values_of_row(i)) {
            out[j] += v * yi;
        }
    }
}

/// Allocating wrapper around `mat_vec_into` — `mat * x` as a fresh `Vec`.
pub fn mat_vec(mat: &Csr, x: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; mat.nrows()];
    mat_vec_into(mat, x, &mut out);
    out
}

/// Allocating wrapper around `mat_t_vec_into` — `mat^T * y` as a fresh `Vec`.
pub fn mat_t_vec(mat: &Csr, n_cols: usize, y: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; n_cols];
    mat_t_vec_into(mat, y, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    fn sample_rows() -> Vec<Vec<(usize, f64)>> {
        //  [ 1  0  2  0 ]
        //  [ 0  3  0  0 ]
        //  [ 4  0  0  5 ]
        vec![vec![(0, 1.0), (2, 2.0)], vec![(1, 3.0)], vec![(0, 4.0), (3, 5.0)]]
    }

    #[test]
    fn csr_round_trips_through_csc_unchanged() {
        let csr = CsrMat::from_rows(&sample_rows(), 4);
        let back = csr.to_csc().to_csr();
        assert_eq!(csr, back);
        assert_eq!(back.to_rows(), sample_rows());
    }

    #[test]
    fn csc_columns_match_the_hand_transposed_matrix() {
        let csc = CsrMat::from_rows(&sample_rows(), 4).to_csc();
        assert_eq!(csc.n_rows(), 3);
        assert_eq!(csc.n_cols(), 4);
        assert_eq!(csc.col(0), &[(0, 1.0), (2, 4.0)]);
        assert_eq!(csc.col(1), &[(1, 3.0)]);
        assert_eq!(csc.col(2), &[(0, 2.0)]);
        assert_eq!(csc.col(3), &[(2, 5.0)]);
    }

    #[test]
    fn csc_built_from_rows_matches_csc_built_by_conversion() {
        let rows = sample_rows();
        assert_eq!(CscMat::from_rows(&rows, 4), CsrMat::from_rows(&rows, 4).to_csc());
    }

    #[test]
    fn transpose_of_transpose_is_the_original() {
        let csr = CsrMat::from_rows(&sample_rows(), 4);
        assert_eq!(csr.transpose().transpose(), csr);
    }

    #[test]
    fn both_orientations_agree_on_mat_vec_and_its_transpose() {
        let csr = CsrMat::from_rows(&sample_rows(), 4);
        let csc = csr.to_csc();
        let x = [1.0, 2.0, 3.0, 4.0];
        let y = [1.0, -1.0, 2.0];

        // A x = [1*1 + 2*3, 3*2, 4*1 + 5*4] = [7, 6, 24]
        assert_eq!(csr.mat_vec(&x), vec![7.0, 6.0, 24.0]);
        assert_eq!(csc.mat_vec(&x), vec![7.0, 6.0, 24.0]);

        // A^T y = [1*1 + 4*2, 3*(-1), 2*1, 5*2] = [9, -3, 2, 10]
        assert_eq!(csr.mat_t_vec(&y), vec![9.0, -3.0, 2.0, 10.0]);
        assert_eq!(csc.mat_t_vec(&y), vec![9.0, -3.0, 2.0, 10.0]);
    }

    #[test]
    fn sparse_products_match_their_dense_counterparts() {
        let csr = CsrMat::from_rows(&sample_rows(), 4);
        let csc = csr.to_csc();
        let mut accum_cols = SparseAccum::new(4);
        let mut accum_rows = SparseAccum::new(3);

        let y_sparse = [(0usize, 1.0f64), (2usize, 2.0f64)];
        let mut y_dense = vec![0.0; 3];
        for &(i, v) in &y_sparse {
            y_dense[i] = v;
        }
        assert_eq!(csr.mat_t_vec_sparse(&y_sparse, &mut accum_cols, 0.0).to_dense(), csr.mat_t_vec(&y_dense));

        let x_sparse = [(2usize, 3.0f64), (3usize, 4.0f64)];
        let mut x_dense = vec![0.0; 4];
        for &(j, v) in &x_sparse {
            x_dense[j] = v;
        }
        assert_eq!(csc.mat_vec_sparse(&x_sparse, &mut accum_rows, 0.0).to_dense(), csc.mat_vec(&x_dense));
    }

    #[test]
    fn col_counts_and_row_counts_agree_across_orientations() {
        let csr = CsrMat::from_rows(&sample_rows(), 4);
        assert_eq!(csr.col_counts(), vec![2, 1, 1, 1]);
        assert_eq!(csr.to_csc().row_counts(), vec![2, 1, 2]);
    }

    #[test]
    fn faer_interop_round_trips() {
        let rows = sample_rows();
        let faer = csr_from_rows(&rows, 4);
        assert_eq!(csr_rows(&faer), rows);
        assert_eq!(csr_row_vec(&faer, 2), vec![(0, 4.0), (3, 5.0)]);
        assert_eq!(csr_col_counts(&faer), vec![2, 1, 1, 1]);
        assert_eq!(CsrMat::from_faer(&faer), CsrMat::from_rows(&rows, 4));
        assert_eq!(csr_rows(&CsrMat::from_rows(&rows, 4).to_faer()), rows);
        assert_eq!(csr_to_csc(&faer), CscMat::from_rows(&rows, 4));
    }

    #[test]
    fn csr_rows_pruned_drops_explicitly_stored_zeros() {
        // `csr_from_rows` drops exact zeros on the way in, so the stored
        // zero has to go through faer's own triplet builder directly.
        let faer = Csr::try_new_from_triplets(1, 3, &[(0, 0, 1.0), (0, 1, 0.0), (0, 2, 3.0)]).unwrap();
        assert_eq!(csr_row_vec(&faer, 0), vec![(0, 1.0), (1, 0.0), (2, 3.0)]);
        assert_eq!(csr_rows(&faer), vec![vec![(0, 1.0), (1, 0.0), (2, 3.0)]]);
        assert_eq!(csr_rows_pruned(&faer), vec![vec![(0, 1.0), (2, 3.0)]]);
    }

    #[test]
    fn sparse_vec_scatter_gather_round_trips_and_dot_matches_dense() {
        let v = SparseVec::from_entries(5, vec![(1, 2.0), (4, -3.0)]);
        assert_eq!(v.nnz(), 2);
        assert!(approx(v.density(), 0.4));
        assert_eq!(v.to_dense(), vec![0.0, 2.0, 0.0, 0.0, -3.0]);

        let mut dense = vec![7.0; 5];
        dense.iter_mut().for_each(|x| *x = 0.0);
        v.scatter_into(&mut dense);
        let mut gathered = SparseVec::zeros(5);
        gathered.gather_from(&dense, 0.0);
        assert_eq!(gathered, v);

        v.unscatter_from(&mut dense);
        assert_eq!(dense, vec![0.0; 5]);

        let w = [1.0, 10.0, 100.0, 1000.0, 10000.0];
        assert!(approx(v.dot_dense(&w), 2.0 * 10.0 - 3.0 * 10000.0));
        assert!(approx(sparse_dot_dense(v.entries(), &w), v.dot_dense(&w)));
    }

    #[test]
    fn sparse_vec_canonicalize_sorts_merges_and_prunes() {
        let mut v = SparseVec::from_entries(6, vec![(4, 1.0), (1, 2.0), (4, -1.0), (0, 1e-14), (3, 5.0)]);
        v.canonicalize(1e-12);
        // (4, 1.0) + (4, -1.0) cancels; (0, 1e-14) falls under `tol`.
        assert_eq!(v.entries(), &[(1, 2.0), (3, 5.0)]);
    }

    #[test]
    fn from_dense_tol_sparsifies_a_noisy_dense_vector() {
        let dense = [1.0, 1e-15, -2.0, 0.0, 1e-13];
        assert_eq!(SparseVec::from_dense(&dense).nnz(), 4);
        assert_eq!(SparseVec::from_dense_tol(&dense, 1e-12).entries(), &[(0, 1.0), (2, -2.0)]);
    }

    #[test]
    fn accumulator_matches_a_btreemap_merge_entry_for_entry() {
        use std::collections::BTreeMap;
        let row = [(3usize, 1.0f64), (0usize, 2.0f64), (5usize, -4.0f64)];
        let pivot = [(5usize, 2.0f64), (1usize, 0.5f64), (0usize, 1.0f64)];
        let factor = 2.0;

        let mut map: BTreeMap<usize, f64> = row.iter().copied().collect();
        for &(k, v) in &pivot {
            *map.entry(k).or_insert(0.0) -= factor * v;
        }
        map.remove(&5);
        map.retain(|_, v| v.abs() > 1e-9);
        let expected: Vec<(usize, f64)> = map.into_iter().collect();

        let mut accum = SparseAccum::new(8);
        assert_eq!(axpy_row(&mut accum, &row, &pivot, factor, 5, 1e-9), expected);
    }

    #[test]
    fn accumulator_reset_isolates_successive_merges() {
        let mut accum = SparseAccum::new(4);
        accum.load(&[(0, 1.0), (2, 2.0)]);
        assert!(accum.contains(2));
        assert!(approx(accum.get(2), 2.0));
        assert_eq!(accum.take_sorted(0.0), vec![(0, 1.0), (2, 2.0)]);

        accum.load(&[(1, 5.0)]);
        assert!(!accum.contains(0), "previous merge's pattern must not leak in");
        assert!(!accum.contains(2));
        assert!(approx(accum.get(2), 0.0));
        assert_eq!(accum.take_sorted(0.0), vec![(1, 5.0)]);
    }

    #[test]
    fn accumulator_set_is_last_write_wins_and_add_accumulates() {
        let mut accum = SparseAccum::new(4);
        accum.reset();
        accum.set(1, 3.0);
        accum.set(1, 7.0);
        accum.add(1, 1.0);
        accum.add(2, 4.0);
        assert_eq!(accum.take_sorted(0.0), vec![(1, 8.0), (2, 4.0)]);
    }

    #[test]
    fn accumulator_remove_drops_an_index_even_when_its_value_is_nonzero() {
        let mut accum = SparseAccum::new(4);
        accum.load(&[(0, 1.0), (1, 2.0), (2, 3.0)]);
        accum.remove(1);
        assert!(!accum.contains(1));
        assert_eq!(accum.take_sorted(0.0), vec![(0, 1.0), (2, 3.0)]);
        // A removed index is reusable by the *next* merge.
        accum.load(&[(1, 9.0)]);
        assert_eq!(accum.take_sorted(0.0), vec![(1, 9.0)]);
    }

    #[test]
    fn from_rows_canonical_fixes_unsorted_duplicated_input() {
        let csr = CsrMat::from_rows_canonical(&[vec![(2, 1.0), (0, 2.0), (2, 3.0)]], 3, 0.0);
        assert_eq!(csr.row(0), &[(0, 2.0), (2, 4.0)]);
    }

    #[test]
    fn from_triplets_builds_the_same_matrix_as_from_rows() {
        let triplets = [(0usize, 0usize, 1.0f64), (0, 2, 2.0), (1, 1, 3.0), (2, 0, 4.0), (2, 3, 5.0)];
        assert_eq!(CsrMat::from_triplets(3, 4, &triplets), CsrMat::from_rows(&sample_rows(), 4));
    }

    #[test]
    fn from_entry_stream_matches_a_concatenated_two_block_build() {
        let block_a = sample_rows();
        let block_g = vec![vec![(1usize, 7.0f64), (3usize, 0.0f64)], vec![(0usize, 8.0f64)]];
        // Entries with an exact zero are filtered out by the stream, so
        // the reference build filters them too.
        let mut concat: Vec<Vec<(usize, f64)>> = block_a.clone();
        concat.extend(block_g.iter().map(|r| r.iter().copied().filter(|&(_, v)| v != 0.0).collect()));
        let expected = CscMat::from_rows(&concat, 4);

        let got = CscMat::from_entry_stream(concat.len(), 4, |emit| {
            for (i, row) in block_a.iter().enumerate() {
                for &(j, v) in row {
                    emit(i, j, v);
                }
            }
            for (gi, row) in block_g.iter().enumerate() {
                for &(j, v) in row {
                    if v != 0.0 {
                        emit(block_a.len() + gi, j, v);
                    }
                }
            }
        });
        assert_eq!(got, expected);
        assert_eq!(got.col(1), &[(1, 3.0), (3, 7.0)]);
    }

    #[test]
    fn csc_col_dense_matches_the_dense_column() {
        let csc = CsrMat::from_rows(&sample_rows(), 4).to_csc();
        assert_eq!(csc.col_dense(0), vec![1.0, 0.0, 4.0]);
        let mut buf = vec![9.9; 3];
        csc.col_into_dense(3, &mut buf);
        assert_eq!(buf, vec![0.0, 0.0, 5.0]);
    }

    #[test]
    fn sparse_vec_mutators_cover_the_incremental_build_path() {
        let mut v = SparseVec::with_capacity(6, 3);
        assert!(v.is_empty());
        assert_eq!(v.len(), 6);
        v.push(4, -2.0);
        v.push(1, 3.0);
        v.push(5, 1e-14);
        assert_eq!(v.nnz(), 3);

        v.prune(1e-12);
        v.sort();
        assert_eq!(v.entries(), &[(1, 3.0), (4, -2.0)]);
        assert!(approx(v.norm2(), 13.0f64.sqrt()));
        assert_eq!(v.iter().collect::<Vec<_>>(), vec![(1, 3.0), (4, -2.0)]);

        v.scale(2.0);
        let mut acc = vec![1.0; 6];
        v.scatter_add_into(0.5, &mut acc);
        assert_eq!(acc, vec![1.0, 1.0 + 3.0, 1.0, 1.0, 1.0 - 2.0, 1.0]);

        v.entries_mut()[0].1 = 0.0;
        v.prune(0.0);
        assert_eq!(v.into_entries(), vec![(4, -4.0)]);

        let mut z = SparseVec::zeros(3);
        z.push(0, 1.0);
        z.clear();
        assert!(z.is_empty());
    }

    #[test]
    fn owned_row_and_column_views_carry_the_right_logical_length() {
        let csr = CsrMat::from_rows(&sample_rows(), 4);
        let csc = csr.to_csc();
        let row = csr.row_vec(2);
        assert_eq!(row.len(), 4);
        assert_eq!(row.entries(), &[(0, 4.0), (3, 5.0)]);
        let col = csc.col_vec(0);
        assert_eq!(col.len(), 3);
        assert_eq!(col.entries(), &[(0, 1.0), (2, 4.0)]);
        assert_eq!(csc.to_cols(), vec![vec![(0, 1.0), (2, 4.0)], vec![(1, 3.0)], vec![(0, 2.0)], vec![(2, 5.0)]]);
        assert_eq!(csr.nnz(), 5);
        assert_eq!(csc.nnz(), 5);
    }

    #[test]
    fn accumulator_take_sorted_vec_reports_the_index_space_as_its_length() {
        let mut accum = SparseAccum::new(7);
        assert_eq!(accum.capacity(), 7);
        accum.load(&[(6, 1.0), (2, 2.0)]);
        let v = accum.take_sorted_vec(0.0);
        assert_eq!(v.len(), 7);
        assert_eq!(v.entries(), &[(2, 2.0), (6, 1.0)]);
    }

    #[test]
    fn scatter_dense_and_axpy_helpers_match_their_sparse_vec_forms() {
        let entries = [(1usize, 2.0f64), (3usize, -4.0f64)];
        let mut buf = vec![9.9; 5];
        scatter_dense(&entries, &mut buf);
        assert_eq!(buf, vec![0.0, 2.0, 0.0, -4.0, 0.0]);
        sparse_axpy_dense(0.5, &entries, &mut buf);
        assert_eq!(buf, vec![0.0, 3.0, 0.0, -6.0, 0.0]);
    }

    #[test]
    fn hybrid_vec_picks_its_representation_from_fill() {
        let sparse = HybridVec::pack(10, vec![(1, 2.0), (7, -1.0)], 0.4);
        assert!(matches!(sparse, HybridVec::Sparse(_)));
        let dense = HybridVec::pack(4, vec![(0, 1.0), (1, 2.0), (3, 4.0)], 0.4);
        assert!(matches!(dense, HybridVec::Dense { .. }));
        // The skipped index (2 here) is a stored `0.0` in the dense arm.
        match &dense {
            HybridVec::Dense { data, nnz } => {
                assert_eq!(&data[..], &[1.0, 2.0, 0.0, 4.0]);
                assert_eq!(*nnz, 3);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn hybrid_vec_both_representations_agree_on_dot_and_axpy() {
        let pairs = vec![(0usize, 1.0f64), (1, 2.0), (3, 4.0)];
        // Same content, forced into each representation by the threshold.
        let as_sparse = HybridVec::pack(4, pairs.clone(), 1.0);
        let as_dense = HybridVec::pack(4, pairs, 0.0);
        assert!(matches!(as_sparse, HybridVec::Sparse(_)));
        assert!(matches!(as_dense, HybridVec::Dense { .. }));

        let x = [10.0, 100.0, 1000.0, 10000.0];
        assert!(approx(as_sparse.dot_dense(&x), 1.0 * 10.0 + 2.0 * 100.0 + 4.0 * 10000.0));
        assert!(approx(as_dense.dot_dense(&x), as_sparse.dot_dense(&x)));

        let mut ds = vec![1.0; 4];
        let mut dd = vec![1.0; 4];
        as_sparse.axpy_into_dense(-2.0, &mut ds);
        as_dense.axpy_into_dense(-2.0, &mut dd);
        assert_eq!(ds, vec![-1.0, -3.0, 1.0, -7.0]);
        assert_eq!(ds, dd, "the skipped index must be untouched in both arms");

        assert_eq!(as_sparse.nnz(), 3);
        assert_eq!(as_dense.nnz(), 3);
        assert_eq!(as_sparse.to_pairs(), as_dense.to_pairs());
    }

    /// `pack_scaled_dense` exists only as an allocation-free shortcut for
    /// what `simplex::lu`'s Forrest-Tomlin update used to spell out as a
    /// filtered `collect()` handed to `pack` — so what it must guarantee is
    /// not some property of its own but *equality with that original
    /// spelling*, in both representations and including the two filters
    /// (the skipped pivot slot, and products that come out exactly zero).
    #[test]
    fn pack_scaled_dense_matches_collect_then_pack() {
        // `src[2]` is the skipped slot; `src[5]` is a value that is nonzero
        // itself but whose scaled product underflows to zero, which the
        // original's post-multiply `v != 0.0` filter dropped and this must
        // drop too (otherwise `nnz`, and with it every refactorization
        // trigger reading `fill_count`, silently drifts).
        let src = [1.5, 0.0, 7.0, -2.0, 0.0, 1e-320, 4.0, 0.0];
        for &scale in &[1.0f64, -3.0, 1e-8] {
            for &frac in &[1.0f64, 0.0, 0.4] {
                let pairs: Vec<(usize, f64)> = (0..src.len())
                    .filter(|&i| i != 2)
                    .map(|i| (i, scale * src[i]))
                    .filter(|&(_, v)| v != 0.0)
                    .collect();
                let expected = HybridVec::pack(src.len(), pairs, frac);
                let got = HybridVec::pack_scaled_dense(&src, 2, scale, frac);
                assert_eq!(
                    std::mem::discriminant(&expected),
                    std::mem::discriminant(&got),
                    "scale={scale} frac={frac}: same representation chosen"
                );
                assert_eq!(got.nnz(), expected.nnz(), "scale={scale} frac={frac}");
                assert_eq!(got.to_pairs(), expected.to_pairs(), "scale={scale} frac={frac}");
                // The dense arm must keep storing a literal zero at the
                // skipped slot, which is what lets `dot_dense`/
                // `axpy_into_dense` run over the whole array untested.
                if let HybridVec::Dense { data, .. } = &got {
                    assert_eq!(data[2], 0.0, "scale={scale} frac={frac}");
                }
            }
        }
    }

    #[test]
    fn hybrid_vec_remove_index_keeps_nnz_honest_in_both_arms() {
        for frac in [1.0f64, 0.0] {
            let mut v = HybridVec::pack(4, vec![(0, 1.0), (1, 2.0), (3, 4.0)], frac);
            assert!(v.remove_index(1), "frac={frac}: removing a present index reports true");
            assert_eq!(v.nnz(), 2, "frac={frac}");
            assert_eq!(v.to_pairs(), vec![(0, 1.0), (3, 4.0)], "frac={frac}");
            let mut seen = Vec::new();
            v.for_each_index(|i| seen.push(i));
            assert_eq!(seen, vec![0, 3], "frac={frac}");
            // Removing an index that holds nothing must not decrement nnz.
            assert!(!v.remove_index(2), "frac={frac}: removing an absent index reports false");
            assert_eq!(v.nnz(), 2, "frac={frac}");
        }
    }

    #[test]
    fn epoch_marks_begin_clears_every_previous_mark() {
        let mut m = EpochMarks::new(4);
        assert_eq!(m.len(), 4);
        assert!(!m.is_marked(0), "nothing may read as marked before the first begin");

        m.begin();
        m.mark(1);
        m.mark(3);
        assert!(m.is_marked(1) && m.is_marked(3));
        assert!(!m.is_marked(0) && !m.is_marked(2));

        m.begin();
        assert!(!m.is_marked(1) && !m.is_marked(3), "a new pass starts clear");
    }

    #[test]
    fn epoch_marks_unmark_takes_one_mark_back_for_this_pass_only() {
        let mut m = EpochMarks::new(3);
        m.begin();
        m.mark(0);
        m.mark(2);
        m.unmark(0);
        assert!(!m.is_marked(0));
        assert!(m.is_marked(2));
        m.mark(0);
        assert!(m.is_marked(0), "an unmarked index is markable again");
        m.begin();
        assert!(!m.is_marked(0) && !m.is_marked(2));
    }

    #[test]
    fn epoch_marks_survive_counter_wraparound() {
        let mut m = EpochMarks::new(2);
        // Drive the counter to the value whose next `begin` wraps to 0.
        m.force_epoch_for_test(u32::MAX);
        m.mark(0);
        assert!(m.is_marked(0));
        m.begin(); // wraps: must reset the stamps rather than keep 0 live
        assert!(!m.is_marked(0), "a stamp from the pre-wrap pass must not read as live");
        m.mark(1);
        assert!(m.is_marked(1) && !m.is_marked(0));
    }

    #[test]
    fn csc_builder_matches_a_two_pass_build_of_the_same_matrix() {
        let cols: Vec<Vec<(usize, f64)>> = vec![vec![(0, 1.0), (2, 4.0)], vec![(1, 3.0)], vec![(0, 2.0)], vec![(2, 5.0)]];
        let mut b = CscBuilder::with_capacity(3, 4, 5);
        for col in &cols {
            for &(i, v) in col {
                b.push(i, v);
            }
            b.end_column();
        }
        assert_eq!(b.columns_built(), 4);
        let built = b.build();
        assert_eq!(built, CsrMat::from_rows(&sample_rows(), 4).to_csc());
        assert_eq!(built.n_rows(), 3);
        assert_eq!(built.n_cols(), 4);
    }

    #[test]
    fn csc_empty_has_every_column_present_and_empty() {
        let m = CscMat::empty(3, 4);
        assert_eq!((m.n_rows(), m.n_cols(), m.nnz()), (3, 4, 0));
        assert!((0..4).all(|j| m.col(j).is_empty()));
        assert_eq!(m.mat_t_vec(&[1.0, 2.0, 3.0]), vec![0.0; 4]);
    }

    #[test]
    fn csc_builder_keeps_empty_columns() {
        let mut b = CscBuilder::new(2);
        b.end_column(); // column 0: empty
        b.push(1, 7.0);
        b.end_column(); // column 1
        b.end_column(); // column 2: empty
        let m = b.build();
        assert_eq!(m.n_cols(), 3);
        assert!(m.col(0).is_empty());
        assert_eq!(m.col(1), &[(1, 7.0)]);
        assert!(m.col(2).is_empty());
        assert_eq!(m.nnz(), 1);
    }

    #[test]
    fn empty_matrix_dimensions_and_products_stay_well_defined() {
        let csr = CsrMat::from_rows(&[], 3);
        assert_eq!(csr.n_rows(), 0);
        assert_eq!(csr.n_cols(), 3);
        assert_eq!(csr.nnz(), 0);
        assert_eq!(csr.mat_t_vec(&[]), vec![0.0, 0.0, 0.0]);
        assert_eq!(csr.to_csc().n_cols(), 3);
        assert_eq!(csr.col_counts(), vec![0, 0, 0]);
    }
}
