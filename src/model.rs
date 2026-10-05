//! このクレート唯一の PyO3 の入口。`PyModel` は Python の `enomoto_solver._core.PyModel`
//! に対応し、各メソッドは Python 側 `Model` (`python/enomoto_solver/model.py`) から
//! 直接呼ばれる。ここが唯一の入力検証の境界で、他のモジュールは渡されたデータを
//! そのまま信頼する (例: 変数境界の NaN と `lb > ub` を弾くのは `add_variable` だけ)。
//!
//! 変数境界は本物の `+/-inf` でもよい。自由変数などは前処理で消去されうるので、
//! ここでは有限の大きな値 (`BIG_M`) に置き換えない。前処理で消えずに残った
//! 無限境界だけを、単体法の標準形構築 (`build_std_form_presolved`) で扱う。

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::BTreeMap;

use crate::mip::solve_mip;

use crate::types::{ConstraintRow, LinearExpr, LpOptions, Objective, RootSolver, RowSense, Sense, Status, VarType, VariableData};

/// モデルの蓄積状態: 登録済みの全変数、(1 つだけの) 目的関数、全制約。
/// `add_variable` / `set_objective` / `add_constraint` で追加・更新され、
/// `solve` は読むだけで変更しない。
#[pyclass(name = "PyModel", module = "enomoto_solver._core")]
pub struct PyModel {
    /// 変数の種類と境界 (添字が変数番号)。
    variables: Vec<VariableData>,
    /// 目的関数 (`set_objective` が呼ばれるまで `None`)。
    objective: Option<Objective>,
    /// 追加順の制約行。
    constraints: Vec<ConstraintRow>,
}

/// FFI 境界から渡された `(変数番号, 係数)` の列 (Python の `Function.coeffs.items()`) を
/// `LinearExpr` (`BTreeMap` 形) に変換する。同じ番号の係数は合算し
/// (`x + x` と `2*x` が同じになる)、登録済み変数の範囲外の番号は `ValueError` にする。
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
    /// 空のモデルを作る。
    #[new]
    fn new() -> Self {
        PyModel {
            variables: Vec::new(),
            objective: None,
            constraints: Vec::new(),
        }
    }

    /// 変数を 1 つ登録し、その変数番号を返す。`vtype` は `"continuous"` / `"integer"`。
    /// 二値変数は呼び出し側が境界 `[0, 1]` の整数変数として登録する。
    ///
    /// `lb`/`ub` は `+/-inf` でもよい (自由変数・片側非有界の変数。例: MPS の
    /// `FR`/`MI`/`PL` 境界)。不正なのは NaN と `lb > ub` だけで、その場合は `ValueError`。
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

    /// 登録済みの変数の数。
    fn n_variables(&self) -> usize {
        self.variables.len()
    }

    /// 目的関数を設定する (`sense` は `"minimize"` / `"maximize"`)。
    /// 2 回目以降の呼び出しは前の目的関数を上書きする (加算ではない)。
    fn set_objective(&mut self, coeffs: Vec<(usize, f64)>, constant: f64, sense: &str) -> PyResult<()> {
        let sense = Sense::parse(sense)?;
        let expr = to_linear_expr(coeffs, constant, self.variables.len())?;
        self.objective = Some(Objective { expr, sense });
        Ok(())
    }

    /// 制約を 1 行追加する (`sense` は `"<="` / `">="` / `"=="`)。Python 側で
    /// `expr <sense> rhs` の 1 行に正規化済みの `Constraint` 1 つにつき 1 回呼ばれる。
    fn add_constraint(&mut self, coeffs: Vec<(usize, f64)>, sense: &str, rhs: f64) -> PyResult<()> {
        let sense = RowSense::parse(sense)?;
        let expr = to_linear_expr(coeffs, 0.0, self.variables.len())?;
        self.constraints.push(ConstraintRow { expr, sense, rhs });
        Ok(())
    }

    /// 登録済みの制約の数。
    fn n_constraints(&self) -> usize {
        self.constraints.len()
    }

    /// モデルを解き、結果を dict で返す。整数変数があれば分枝限定法、なければ LP を
    /// 1 回解く (判断は `mip::solve_mip`)。dict のキーは常に次の 4 つ:
    /// `"status"` (状態文字列)、`"objective"` と `"x"` (`"optimal"` のときだけ値、
    /// それ以外は `None`)、`"node_limit_hit"` (MIP がノード数上限で打ち切られたときだけ `True`)。
    ///
    /// `root_solver`: 各 LP の解法。`"simplex"` (既定) か `"interior"`。
    ///
    /// `distinguish_infeasible_unbounded` (既定 `false`): `false` なら、有限の最適値が
    /// ないと分かった時点で `"infeasible_or_unbounded"` を返す。`true` なら
    /// `"infeasible"` と `"unbounded"` を区別する (`types::LpOptions` 参照)。
    ///
    /// 目的関数が未設定なら `ValueError`。
    #[pyo3(signature = (root_solver=None, distinguish_infeasible_unbounded=false))]
    fn solve<'py>(&self, py: Python<'py>, root_solver: Option<&str>, distinguish_infeasible_unbounded: bool) -> PyResult<Bound<'py, PyDict>> {
        let objective = self.objective.as_ref().ok_or_else(|| {
            PyValueError::new_err("no objective set: call Model.set_objective(...) before solve()")
        })?;
        let root_solver = match root_solver {
            Some(s) => RootSolver::parse(s)?,
            None => RootSolver::Simplex,
        };

        let opts = LpOptions { distinguish_infeasible_unbounded, ..Default::default() };
        let result = solve_mip(&self.variables, objective, &self.constraints, root_solver, opts);

        // Python に返す結果 dict
        let dict = PyDict::new_bound(py);
        dict.set_item(pyo3::intern!(py, "status"), result.status.as_str())?;
        match result.status {
            Status::Optimal => {
                dict.set_item(pyo3::intern!(py, "objective"), result.objective)?;
                dict.set_item(pyo3::intern!(py, "x"), result.x)?;
            }
            Status::Infeasible | Status::Unbounded | Status::InfeasibleOrUnbounded | Status::NotSolved => {
                dict.set_item(pyo3::intern!(py, "objective"), py.None())?;
                dict.set_item(pyo3::intern!(py, "x"), py.None())?;
            }
        }
        dict.set_item(pyo3::intern!(py, "node_limit_hit"), result.node_limit_hit)?;
        Ok(dict)
    }
}
