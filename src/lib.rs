//! Crate layout, grouped by function:
//!
//!   - `types`, `model`, `solver` — shared types, the PyO3 API entry
//!     point, and top-level LP dispatch (thin orchestration, kept flat).
//!   - `sparse` — the shared `Csr` alias and its mat-vec helpers, used by
//!     both `interior_point::kkt` and `presolve`.
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

mod graph;
mod interior_point;
mod mip;
mod model;
mod presolve;
mod simplex;
mod solver;
mod sparse;
mod types;

use pyo3::prelude::*;

use model::PyModel;

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyModel>()?;
    Ok(())
}
