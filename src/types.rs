//! Shared data types passed between every layer of the crate: the PyO3
//! boundary (`model.rs`) builds these from the Python-side `Model` /
//! `Variable` / `Function` / `Constraint` objects, and every solver
//! (`simplex`, the inactive `interior_point`, `mip`) consumes them
//! without needing to know anything about Python at all.

use pyo3::exceptions::PyValueError;
use pyo3::PyResult;
use std::collections::HashMap;

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
/// `HashMap`-backed form in `model.rs::to_linear_expr`.
#[derive(Debug, Clone)]
pub struct LinearExpr {
    pub coeffs: HashMap<usize, f64>,
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
/// three regardless of its internal algorithm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Optimal,
    Infeasible,
    Unbounded,
}

impl Status {
    /// The string `model.rs::solve` puts in the `"status"` key of the
    /// dict handed back to Python (`Solution.status` on the Python side).
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::Optimal => "optimal",
            Status::Infeasible => "infeasible",
            Status::Unbounded => "unbounded",
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
