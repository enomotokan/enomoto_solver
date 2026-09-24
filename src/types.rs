//! Shared data types passed between every layer of the crate: the PyO3
//! boundary (`model.rs`) builds these from the Python-side `Model` /
//! `Variable` / `Function` / `Constraint` objects, and every solver
//! (`simplex`, the inactive `interior_point`, `mip`) consumes them
//! without needing to know anything about Python at all.

use pyo3::exceptions::PyValueError;
use pyo3::PyResult;
use std::collections::BTreeMap;

/// A decision variable's kind. There is no dedicated "binary" variant —
/// `model.rs`'s Python-facing API represents a binary variable as an
/// `Integer` variable whose bounds happen to be `[0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarType {
    Continuous,
    Integer,
}

impl VarType {
    /// Parses the string form PyO3 hands across the FFI boundary
    /// (`"continuous"` / `"integer"`, already normalized by
    /// `python/enomoto_solver/types.py` before it reaches Rust).
    pub fn parse(s: &str) -> PyResult<Self> {
        match s {
            "continuous" => Ok(VarType::Continuous),
            "integer" => Ok(VarType::Integer),
            other => Err(PyValueError::new_err(format!(
                "unknown variable type '{other}' (expected 'continuous' or 'integer')"
            ))),
        }
    }
}

/// Whether an objective is minimized or maximized. Every solver internally
/// works in minimize form; `Sense::Maximize` is handled by negating
/// objective coefficients at the point they're built into a solver's own
/// standard form (see e.g. `simplex::build_std_form`), not by threading
/// `Sense` itself through the solve loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sense {
    Minimize,
    Maximize,
}

impl Sense {
    pub fn parse(s: &str) -> PyResult<Self> {
        match s {
            "minimize" => Ok(Sense::Minimize),
            "maximize" => Ok(Sense::Maximize),
            other => Err(PyValueError::new_err(format!(
                "unknown objective sense '{other}' (expected 'minimize' or 'maximize')"
            ))),
        }
    }
}

/// A constraint row's comparison operator. Every `Constraint` the Python
/// side builds carries exactly one of these — `Function <= Function`,
/// `>=`, or `==` — normalized to `expr <sense> rhs` form (`ConstraintRow`
/// below) by the time it reaches Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowSense {
    Le,
    Ge,
    Eq,
}

impl RowSense {
    pub fn parse(s: &str) -> PyResult<Self> {
        match s {
            "<=" => Ok(RowSense::Le),
            ">=" => Ok(RowSense::Ge),
            "==" => Ok(RowSense::Eq),
            other => Err(PyValueError::new_err(format!(
                "unknown constraint sense '{other}' (expected '<=', '>=' or '==')"
            ))),
        }
    }

    /// The sense of `-expr <sense'> -rhs` given `expr <sense> rhs` — i.e.
    /// negating both sides of a row flips `<=`/`>=` and leaves `==`
    /// unchanged. Used when a solver's standard form wants every
    /// inequality in one canonical direction (e.g. the inactive
    /// `interior_point::qp` normalizes `>=` rows to `<=` this way).
    pub fn flip(self) -> Self {
        match self {
            RowSense::Le => RowSense::Ge,
            RowSense::Ge => RowSense::Le,
            RowSense::Eq => RowSense::Eq,
        }
    }
}

/// Which LP engine `solver::solve_lp` (and, through it, every
/// `mip::solve_mip` node relaxation) dispatches to — `Model.solve`'s
/// Python-facing `root_solver` argument, `"simplex"` by default. Both are
/// full, independent implementations of the same LP semantics (see
/// `simplex.rs`'s and `interior_point.rs`'s module docs), sharing only the
/// presolve pipeline (`crate::presolve`); keeping both reachable, rather
/// than deleting the interior-point path once `simplex` became the
/// default, is what makes an apples-to-apples comparison between them
/// possible on the exact same problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootSolver {
    Simplex,
    Interior,
}

impl RootSolver {
    pub fn parse(s: &str) -> PyResult<Self> {
        match s {
            "simplex" => Ok(RootSolver::Simplex),
            "interior" => Ok(RootSolver::Interior),
            other => Err(PyValueError::new_err(format!(
                "unknown root_solver '{other}' (expected 'simplex' or 'interior')"
            ))),
        }
    }
}

/// One decision variable's type and bounds. `lb`/`ub` are required to be
/// finite (validated at `model.rs::add_variable`, the only place this
/// struct is constructed from user input) — every solver in this crate
/// relies on that invariant rather than re-checking it; see `simplex.rs`'s
/// module docs for what that specifically enables.
#[derive(Debug, Clone)]
pub struct VariableData {
    pub vtype: VarType,
    pub lb: f64,
    pub ub: f64,
}

/// A sparse linear combination of variables plus a constant:
/// `sum(coeffs[j] * x_j) + constant`. Mirrors the Python `Function`
/// class's own `{index: coeff}` representation — built up by `Function`'s
/// operator overloads on the Python side, then handed across the FFI
/// boundary as a plain `Vec<(usize, f64)>` and reassembled into this
/// `BTreeMap`-backed form in `model.rs::to_linear_expr`.
///
/// `BTreeMap`, not `HashMap`: `presolve::build_a_g`/`simplex.rs`'s
/// `build_std_form` both materialize a row's final term order by iterating
/// `coeffs` directly (`.coeffs.iter().collect()`) — with a `HashMap`,
/// whose default `RandomState` reseeds every process, that order (though
/// always the same *set* of terms) could differ between separate runs of
/// the identical binary on the identical model. On most problems that
/// never surfaces (nothing downstream cares which order a row's terms
/// arrived in), but on a highly degenerate one — many exactly-tied
/// Markowitz/ratio-test/normalization decisions, which is what
/// "degenerate" means — a row's incoming term order can be the one thing
/// deciding which of several equally-valid choices an algorithm makes,
/// cascading into a completely different (though equally correct) pivot
/// sequence and wall-clock time. Prime suspect for exactly this kind of
/// symptom on Netlib `degen3` (bimodal wall-clock time, ~0.8s vs ~3.0-3.5s,
/// across repeated runs of one binary) surviving even after every `rayon`
/// parallel/sequential choice elsewhere in the crate was made a fixed,
/// size-based decision (see `simplex.rs`'s `RAYON_SIZE_THRESHOLD`) ruled
/// out thread-scheduling nondeterminism as the cause. `BTreeMap` iterates
/// in a fixed (ascending-key) order regardless of process/seed, removing
/// the discrepancy at its source rather than downstream at each consumer.
#[derive(Debug, Clone)]
pub struct LinearExpr {
    pub coeffs: BTreeMap<usize, f64>,
    pub constant: f64,
}

/// The objective function `Model.set_objective(...)` registers: an
/// expression plus which direction to optimize it in.
#[derive(Debug, Clone)]
pub struct Objective {
    pub expr: LinearExpr,
    pub sense: Sense,
}

/// One constraint `Model.add_constraint(...)` registers, normalized to
/// `expr <sense> rhs` form (e.g. Python's `x + y <= 10` becomes
/// `expr = x + y`, `sense = Le`, `rhs = 10`).
#[derive(Debug, Clone)]
pub struct ConstraintRow {
    pub expr: LinearExpr,
    pub sense: RowSense,
    pub rhs: f64,
}

/// The outcome of a solve attempt, independent of *why* — every solver
/// (`simplex`, `mip`, the inactive `interior_point`) reports one of these
/// regardless of its internal algorithm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Optimal,
    Infeasible,
    Unbounded,
    /// Proven *not* to have a finite optimum, without having spent the extra
    /// work to tell which of `Infeasible`/`Unbounded` holds: the extended
    /// dual simplex's stage A (the slope problem) ended with `z^1 < 0`,
    /// which by the paper's `prop:trichotomy` means the problem is
    /// infeasible or unbounded (whereas `z^1 = 0` means a finite optimum or
    /// infeasible, never unbounded). Only returned when
    /// [`LpOptions::distinguish_infeasible_unbounded`] is `false` (the
    /// default); with it `true` the solve continues through stage B and
    /// reports `Infeasible` or `Unbounded` instead.
    InfeasibleOrUnbounded,
    /// The solver gave up without reaching any of the verdicts above: the
    /// extended dual simplex hit one of its "should be unreachable"
    /// bail-outs (a singular basis it could not recover from, or its
    /// iteration budget running out). Nothing is claimed about the
    /// problem itself.
    NotSolved,
}

/// Per-solve options that change *what* is reported, not how the model is
/// built — threaded from `Model.solve` down to the LP engines.
#[derive(Debug, Clone, Copy, Default)]
pub struct LpOptions {
    /// `false` (the default): report `Optimal`, `Infeasible` or
    /// [`Status::InfeasibleOrUnbounded`] — stage A of the extended dual
    /// simplex stops at `z^1 < 0` (no finite optimum), and `z^1 = 0` goes on
    /// to stage B, which finds an optimum or proves infeasibility.
    /// `true`: also split `z^1 < 0` into `Infeasible` / `Unbounded` by
    /// running stage B there too; presolve's improving-ray shortcut is then
    /// refused, since it cannot check the rest of the problem's feasibility.
    pub distinguish_infeasible_unbounded: bool,
}

impl Status {
    /// The string `model.rs::solve` puts in the `"status"` key of the
    /// dict handed back to Python (`Solution.status` on the Python side).
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Optimal => "optimal",
            Status::Infeasible => "infeasible",
            Status::Unbounded => "unbounded",
            Status::InfeasibleOrUnbounded => "infeasible_or_unbounded",
            Status::NotSolved => "not_solved",
        }
    }
}

/// A fully-resolved solve result: `solver::solve_lp` and `mip::solve_mip`
/// both return this. `objective`/`x` are only `Some` when
/// `status == Optimal`; `node_limit_hit` is meaningful only for MIP solves
/// (`mip::solve_mip` stopping at its node cap with the best incumbent
/// found so far rather than a proven optimum) and is always `false` for a
/// plain LP solve.
#[derive(Debug, Clone)]
pub struct SolveResult {
    pub status: Status,
    pub objective: Option<f64>,
    pub x: Option<Vec<f64>>,
    pub node_limit_hit: bool,
}
