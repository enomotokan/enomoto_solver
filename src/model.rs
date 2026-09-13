//! The crate's sole PyO3 entry point: `PyModel` is what Python's
//! `enomoto_solver._core.PyModel` binds to, and every method here is
//! called directly by the Python-side `Model` class in
//! `python/enomoto_solver/model.py`. This is also the crate's *only*
//! input-validation boundary — every other module trusts the data it's
//! given (e.g. `add_variable` below is the only place `VariableData` gets
//! constructed from outside input, so it alone is responsible for
//! rejecting NaN and `lb > ub`).
//!
//! A variable's bounds *may* be genuine `+/-inf` — `simplex.rs`'s presolve
//! pipeline (`colsingleton`/`doubleton` in particular) can eliminate a
//! truly free variable's row entirely, at zero replacement-row cost,
//! exactly the way HiGHS's own free-column-singleton substitution does;
//! substituting a finite `BIG_M` sentinel here instead — the old
//! invariant this module used to enforce — would hide that from presolve
//! and force it to re-materialize the variable's (fake) box bound as real
//! rows on every such elimination, capping how far a chain of them can
//! cascade. Only `simplex.rs`'s own `Tableau` (the dual-feasible crash, in
//! particular) still needs every *surviving* variable to have two finite
//! bounds — `build_std_form_presolved` substitutes `BIG_M` for any
//! genuine infinity presolve didn't eliminate, but only *after* presolve
//! has had its chance, not before.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::BTreeMap;

use crate::mip::solve_mip;
use crate::types::{ConstraintRow, LinearExpr, Objective, RootSolver, RowSense, Sense, Status, VarType, VariableData};

/// A model's accumulated state: every variable, the (single) objective
/// once set, and every constraint added so far. Mutated in place by
/// `add_variable`/`set_objective`/`add_constraint`; read (never mutated)
/// by `solve`.
#[pyclass(name = "PyModel", module = "enomoto_solver._core")]
pub struct PyModel {
    variables: Vec<VariableData>,
    objective: Option<Objective>,
    constraints: Vec<ConstraintRow>,
}

/// Converts the `Vec<(variable_index, coefficient)>` pairs PyO3 hands
/// across the FFI boundary (Python's `Function.coeffs.items()`) into a
/// `LinearExpr`'s `BTreeMap` form, summing duplicate indices (so e.g.
/// `x + x` and `2*x` produce the same coefficient) and rejecting any
/// index outside the model's registered variable range.
fn to_linear_expr(coeffs: Vec<(usize, f64)>, constant: f64, n_vars: usize) -> PyResult<LinearExpr> {
    let mut map: BTreeMap<usize, f64> = BTreeMap::new();
    for (j, v) in coeffs {
        if j >= n_vars {
            return Err(PyValueError::new_err(format!(
                "variable index {j} out of range (model has {n_vars} variables)"
            )));
        }
        *map.entry(j).or_insert(0.0) += v;
    }
    Ok(LinearExpr { coeffs: map, constant })
}

#[pymethods]
impl PyModel {
    #[new]
    fn new() -> Self {
        PyModel {
            variables: Vec::new(),
            objective: None,
            constraints: Vec::new(),
        }
    }

    /// Registers a new variable in the model's internal storage and
    /// returns its index. A binary variable is just an integer variable
    /// bounded to [0, 1] by the caller (there is no dedicated binary
    /// vtype — Continuous and Integer are the only two kinds).
    ///
    /// `lb`/`ub` may be `+/-inf` (a genuinely free or one-sided-unbounded
    /// variable, e.g. straight from an MPS `FR`/`MI`/`PL` bound) — see
    /// this module's own docs for why that is no longer rejected here.
    /// `NaN` and `lb > ub` are the only bound values actually invalid at
    /// this boundary.
    fn add_variable(&mut self, vtype: &str, lb: f64, ub: f64) -> PyResult<usize> {
        let vt = VarType::parse(vtype)?;
        if lb.is_nan() || ub.is_nan() {
            return Err(PyValueError::new_err(format!(
                "invalid bounds: lower bound {lb} and upper bound {ub} must not be NaN"
            )));
        }
        if lb > ub {
            return Err(PyValueError::new_err(format!(
                "invalid bounds: lower bound {lb} is greater than upper bound {ub}"
            )));
        }
        self.variables.push(VariableData { vtype: vt, lb, ub });
        Ok(self.variables.len() - 1)
    }

    fn n_variables(&self) -> usize {
        self.variables.len()
    }

    /// Replaces the model's objective. Python's `Model.set_objective`
    /// calls this once per `Function`/`sense` pair — a second call simply
    /// overwrites the first, there is no "add to objective" operation.
    fn set_objective(&mut self, coeffs: Vec<(usize, f64)>, constant: f64, sense: &str) -> PyResult<()> {
        let sense = Sense::parse(sense)?;
        let expr = to_linear_expr(coeffs, constant, self.variables.len())?;
        self.objective = Some(Objective { expr, sense });
        Ok(())
    }

    /// Appends one constraint row. Python's `Model.add_constraint` calls
    /// this once per `Constraint` object (each `Constraint` already
    /// normalized to a single `expr <sense> rhs` row on the Python side
    /// by the time it reaches here).
    fn add_constraint(&mut self, coeffs: Vec<(usize, f64)>, sense: &str, rhs: f64) -> PyResult<()> {
        let sense = RowSense::parse(sense)?;
        let expr = to_linear_expr(coeffs, 0.0, self.variables.len())?;
        self.constraints.push(ConstraintRow { expr, sense, rhs });
        Ok(())
    }

    fn n_constraints(&self) -> usize {
        self.constraints.len()
    }

    /// Solves the model (via branch-and-bound if any variable is
    /// `Integer`, a plain LP solve otherwise — `mip::solve_mip` decides
    /// which) and returns a Python dict with the same four keys
    /// regardless of outcome: `"status"` (always present),
    /// `"objective"`/`"x"` (populated only when `status == "optimal"`,
    /// `None` otherwise), and `"node_limit_hit"` (only ever `True` for a
    /// MIP that hit `mip::MAX_NODES` before proving optimality). The
    /// Python-side `Model.solve` wraps this dict into a `Solution`
    /// namedtuple and raises `InfeasibleError`/`UnboundedError` for the
    /// corresponding statuses.
    ///
    /// `root_solver` selects which LP engine every relaxation is solved
    /// with (`types::RootSolver::parse`: `"simplex"`, the default, or
    /// `"interior"`) — both are full independent implementations sharing
    /// only the presolve pipeline, kept reachable side by side so results
    /// can be cross-checked rather than one being deleted outright.
    #[pyo3(signature = (root_solver=None))]
    fn solve<'py>(&self, py: Python<'py>, root_solver: Option<&str>) -> PyResult<Bound<'py, PyDict>> {
        let objective = self.objective.clone().ok_or_else(|| {
            PyValueError::new_err("no objective set: call Model.set_objective(...) before solve()")
        })?;
        let root_solver = match root_solver {
            Some(s) => RootSolver::parse(s)?,
            None => RootSolver::Simplex,
        };

        let result = solve_mip(&self.variables, &objective, &self.constraints, root_solver);

        let dict = PyDict::new_bound(py);
        dict.set_item("status", result.status.as_str())?;
        match result.status {
            Status::Optimal => {
                dict.set_item("objective", result.objective)?;
                dict.set_item("x", result.x)?;
            }
            Status::Infeasible | Status::Unbounded => {
                dict.set_item("objective", py.None())?;
                dict.set_item("x", py.None())?;
            }
        }
        dict.set_item("node_limit_hit", result.node_limit_hit)?;
        Ok(dict)
    }
}
