//! Crate layout, grouped by function:
//!
//!   - `types`, `model`, `solver` — shared types, the PyO3 API entry
//!     point, and top-level LP dispatch (thin orchestration, kept flat).
//!   - `sparse` — the crate's one home for sparse storage: the `CsrMat`/
//!     `CscMat` compressed pair and the conversions between them, the
//!     `SparseVec` sparse vector and the `SparseAccum` accumulator its
//!     row merges run on, the sparse x dense arithmetic built on all of
//!     them, and the `Csr` alias (faer's own row-major type) plus the
//!     helpers that bridge to it. Used by `simplex`, `presolve` and
//!     `interior_point::kkt` alike — no other module re-derives a
//!     transpose, a `(index, value)` merge, or a mat-vec for itself.
//!   - `presolve` (+ `presolve::{scaling,redundancy,propagate}`) — the
//!     **shared** presolve pipeline (Ruiz scaling, redundant-equality
//!     removal, inequality propagation) run identically by both `simplex`
//!     and `interior_point` — one implementation of each pass, not two.
//!   - `mip` — branch-and-bound for integer/binary variables, sitting on
//!     top of `solver`.
//!   - `simplex` (+ `simplex::lu`) — the **active** engine: a from-scratch
//!     bounded-variable primal/dual revised simplex (Markowitz/Forrest-
//!     Tomlin sparse LU, EXPAND anti-cycling, (dual) steepest-edge
//!     pricing), presolved via `presolve`. `solver::solve_lp` calls
//!     straight into this.
//!   - `interior_point` (+ its `qp`/`kkt` submodules) — the **inactive**
//!     IP-PMM interior-point solver this project used before `simplex` was
//!     implemented and verified. Kept in the module tree, unused, in case
//!     that path is wanted again; also presolved via `presolve`.
//!
//! `legacy/` (`csr.rs`, `preprocess.rs`, `simplex.rs`) holds the very
//! first, since-superseded implementation (a two-phase simplex with its
//! own dense-oriented CSR type). It predates both `simplex` and
//! `interior_point` above and is unrelated to either; left on disk, out
//! of the module tree entirely (not even `mod`-declared here), purely for
//! historical reference.

/// Reads a numeric tuning knob from the environment once per process
/// (cached in a `OnceLock`), falling back to `$default`. Used for A/B
/// sweeps of tolerances and thresholds without a rebuild; the defaults are
/// the tuned values.
macro_rules! tunable {
    ($name:literal, $default:expr, $t:ty) => {{
        static V: std::sync::OnceLock<$t> = std::sync::OnceLock::new();
        *V.get_or_init(|| std::env::var($name).ok().and_then(|s| s.parse::<$t>().ok()).unwrap_or($default))
    }};
}

/// Reads an environment variable (a flag or a string/number setting) once
/// per process, caching it in a `OnceLock` — `std::env::var` costs an
/// environment lock + a linear `environ` scan + a `String` allocation, and
/// the solve path consults ~100 `ENOMOTO_*` flags per LP (callgrind: ~10%
/// of `afiro`'s instructions). Yields `Option<&'static str>`; like
/// [`tunable!`], a value changed with `std::env::set_var` after the first
/// read is not observed (set flags before the process starts).
macro_rules! env_str {
    ($name:literal) => {{
        static V: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
        V.get_or_init(|| std::env::var($name).ok()).as_deref()
    }};
}

mod graph;
mod interior_point;
mod mip;
mod model;
mod params;
mod presolve;
mod simplex;
mod solver;
mod sparse;
mod types;

use pyo3::prelude::*;
use crate::params::alloc::LARGE;

/// Rust-side global allocator: small blocks from mimalloc, large ones from
/// the system allocator.
///
/// Presolve works in owned `Vec<Vec<_>>` row lists and rebuilds its CSR
/// matrices several times per round, so on the small Netlib problems glibc
/// `malloc`/`free` (incl. `malloc_consolidate`) measured at roughly a
/// quarter of all instructions of a `solve()` call (callgrind); mimalloc's
/// size-class free lists make those short-lived small allocations much
/// cheaper. Routing *every* allocation to mimalloc, however, made several
/// mid-size problems 20-80% slower in the simplex main loop (`fit2d`,
/// `degen3`, `greenbea`, ... — no extra syscalls, so a placement effect on
/// the large dense work vectors), so blocks of `LARGE` bytes or more stay
/// with glibc exactly as before. The route is a pure function of the
/// layout size, so `dealloc` always reaches the allocator that made the
/// block; `realloc` across the threshold moves the block between the two.
/// Numerics are unaffected (nothing in this crate depends on allocation
/// addresses).
struct SplitAlloc;

unsafe impl std::alloc::GlobalAlloc for SplitAlloc {
    #[inline]
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        if layout.size() < LARGE {
            mimalloc::MiMalloc.alloc(layout)
        } else {
            std::alloc::System.alloc(layout)
        }
    }
    #[inline]
    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        if layout.size() < LARGE {
            mimalloc::MiMalloc.alloc_zeroed(layout)
        } else {
            std::alloc::System.alloc_zeroed(layout)
        }
    }
    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        if layout.size() < LARGE {
            mimalloc::MiMalloc.dealloc(ptr, layout)
        } else {
            std::alloc::System.dealloc(ptr, layout)
        }
    }
    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: std::alloc::Layout, new_size: usize) -> *mut u8 {
        let old_small = layout.size() < LARGE;
        let new_small = new_size < LARGE;
        if old_small && new_small {
            return mimalloc::MiMalloc.realloc(ptr, layout, new_size);
        }
        if !old_small && !new_small {
            return std::alloc::System.realloc(ptr, layout, new_size);
        }
        let new_layout = std::alloc::Layout::from_size_align_unchecked(new_size, layout.align());
        let new_ptr = self.alloc(new_layout);
        if !new_ptr.is_null() {
            std::ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size));
            self.dealloc(ptr, layout);
        }
        new_ptr
    }
}

#[global_allocator]
static GLOBAL: SplitAlloc = SplitAlloc;

use model::PyModel;

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyModel>()?;
    Ok(())
}
