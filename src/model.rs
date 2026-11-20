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

use crate::types::{ConstraintRow, LinearExpr, LpOptions, Objective, RootSolver, RowSense, Sense, VarType, VariableData, MipSettings};

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

    /// 制約行の非零の総数 (ログの見出し用)。
    fn n_nonzeros(&self) -> usize {
        self.constraints.iter().map(|r| r.expr.coeffs.len()).sum()
    }

    /// 変数の `(下限の一覧, 上限の一覧, 整数かの一覧)` (`Model.readModel` が Python 側の
    /// `Variable` を作るのに使う)。
    fn variable_bounds(&self) -> (Vec<f64>, Vec<f64>, Vec<bool>) {
        let lb = self.variables.iter().map(|v| v.lb).collect();
        let ub = self.variables.iter().map(|v| v.ub).collect();
        let is_int = self.variables.iter().map(|v| v.vtype == VarType::Integer).collect();
        (lb, ub, is_int)
    }

    /// 目的関数の `(非零の (変数番号, 係数) の一覧, 定数, 向き)`。目的がなければ空・0・`"minimize"`。
    fn objective_terms(&self) -> (Vec<(usize, f64)>, f64, &'static str) {
        match &self.objective {
            Some(o) => (
                o.expr.coeffs.iter().map(|(&j, &c)| (j, c)).collect(),
                o.expr.constant,
                if o.sense == Sense::Maximize { "maximize" } else { "minimize" },
            ),
            None => (Vec::new(), 0.0, "minimize"),
        }
    }

    /// MPS の本文 `text` を読み、空のモデルに変数・目的・制約を入れる
    /// (Python の `Model.readModel` から呼ばれる。読み方は `crate::mps`)。
    /// 範囲の行 (両側有限で幅のあるもの) は `<=` と `>=` の 2 行に、両側とも無限の行は捨てる
    /// (HiGHS で読んで組み立てる `benchmark_highs._build_our_model` と同じ形)。
    /// 戻り値は `(問題の名前, 列の名前の一覧, 読み飛ばした箇所の注意の一覧)`。
    fn read_mps(&mut self, text: &str) -> PyResult<(String, Vec<String>, Vec<String>)> {
        if !self.variables.is_empty() || !self.constraints.is_empty() || self.objective.is_some() {
            return Err(PyValueError::new_err("read_mps needs an empty model"));
        }
        let d = crate::mps::parse_mps(text).map_err(|e| PyValueError::new_err(format!("MPS: {e}")))?;
        let n = d.col_names.len();
        let mut vars = Vec::with_capacity(n);
        for j in 0..n {
            let (lb, ub) = (d.col_lb[j], d.col_ub[j]);
            if lb > ub {
                return Err(PyValueError::new_err(format!(
                    "MPS: the column {:?} has lower bound {lb} greater than upper bound {ub}",
                    d.col_names[j]
                )));
            }
            let vtype = if d.is_int[j] { VarType::Integer } else { VarType::Continuous };
            vars.push(VariableData { vtype, lb, ub });
        }
        let mut cons = Vec::with_capacity(d.rows.len());
        for (i, terms) in d.rows.iter().enumerate() {
            let (lo, hi) = (d.row_lo[i], d.row_hi[i]);
            if lo == f64::NEG_INFINITY && hi == f64::INFINITY {
                continue;
            }
            let expr = LinearExpr { coeffs: terms.iter().copied().collect(), constant: 0.0 };
            if lo.is_finite() && hi.is_finite() && (hi - lo).abs() < 1e-12 {
                cons.push(ConstraintRow { expr, sense: RowSense::Eq, rhs: hi });
                continue;
            }
            if hi.is_finite() {
                cons.push(ConstraintRow { expr: expr.clone(), sense: RowSense::Le, rhs: hi });
            }
            if lo.is_finite() {
                cons.push(ConstraintRow { expr, sense: RowSense::Ge, rhs: lo });
            }
        }
        let coeffs: BTreeMap<usize, f64> = d.cost.iter().enumerate().filter(|(_, &c)| c != 0.0).map(|(j, &c)| (j, c)).collect();
        let sense = if d.maximize { Sense::Maximize } else { Sense::Minimize };
        self.variables = vars;
        self.constraints = cons;
        self.objective = Some(Objective { expr: LinearExpr { coeffs, constant: d.offset }, sense });
        Ok((d.name, d.col_names, d.warnings))
    }

    /// モデルを解き、結果を dict で返す。整数変数があれば分枝限定法、なければ LP を
    /// 1 回解く (判断は `mip::solve_mip`)。dict のキー:
    /// - `"status"`: 状態文字列 (`"optimal"` / `"infeasible"` / `"unbounded"` /
    ///   `"infeasible_or_unbounded"` / `"not_solved"` / `"time_limit"`、整数最適化ではさらに `"node_limit"`)。
    /// - `"objective"` と `"x"`: 解があるとき (`"optimal"`、または上限で打ち切られたが暫定解があるとき) の値。
    ///   それ以外は `None`。
    /// - `"node_limit_hit"`: 整数最適化がノード数上限で打ち切られたか。
    /// - `"best_bound"` / `"mip_gap"` / `"nodes"` / `"lp_iterations"`: 整数最適化の探索の要約 (LP では `None`)。
    /// - `"root_bound"` / `"first_solution_time"` / `"first_solution_objective"`: 根の切除平面ループ後の下界 (再スタートがあれば最後の根の
    ///   もので、暫定解の目的値で上から切り詰める)、最初の暫定解の時刻 [秒] と目的値 (整数最適化の評価用。なければ `None`)。
    ///
    /// `lp_solver`: 各 LP の解法。`"auto"` (既定: 前処理後の行数が 5000 以上なら傾き・切片双対二段解法と
    /// 内点法 + クロスオーバーを同時に解き、先に結論を出した側を採る。未満なら二段解法)、`"simplex"`
    /// (二段解法だけ)、`"ipm_crossover"` (内点法 + クロスオーバーだけ)、`"interior"` (独立の IP-PMM 内点法)。
    /// 整数最適化では根と各ノードの LP は二段解法を warm start で解き直す (この指定は LP のときだけ効く)。
    ///
    /// `distinguish_infeasible_unbounded` (既定 `false`): `false` なら、有限の最適値が
    /// ないと分かった時点で `"infeasible_or_unbounded"` を返す。`true` なら
    /// `"infeasible"` と `"unbounded"` を区別する (`types::LpOptions` 参照)。
    ///
    /// `time_limit` [秒]: 制限時間。LP では期限までに結論が出なければ `"time_limit"` を返す (期限は
    /// [`crate::cancel::with_deadline`]。前処理の途中では止まらない)。
    /// `time_limit` / `mip_rel_gap` (既定 0) / `node_limit`: 整数最適化の打ち切り条件。
    /// `threads`: 使うスレッドの数 (既定は `default_threads()`: 使える CPU の数 + 1 の半分。`max_threads()` を超える値は
    /// そこで頭打ち)。求解全体をこの数のスレッドの
    /// rayon プールで動かす。`"auto"` で 2 以上なら、LP (整数最適化では基底のない緩和問題) を二段解法 (1 スレッド) と
    /// 内点法 + クロスオーバー (残り) で同時に解く。整数最適化では木の探索もこの数のスレッドで分け合う。
    ///
    /// `log` (既定 `false`): 求解の要点 (前処理の結果、根の LP、探索の経過など) を `sys.stdout` に出す ([`crate::log`])。
    /// Python の `Model.solve` は既定で `true` を渡す (案内とまとめの行は Python 側で出す)。
    ///
    /// 目的関数が未設定なら `ValueError`。
    #[pyo3(signature = (lp_solver=None, distinguish_infeasible_unbounded=false, time_limit=None, mip_rel_gap=None, node_limit=None, threads=None, log=false))]
    fn solve<'py>(
        &self,
        py: Python<'py>,
        lp_solver: Option<&str>,
        distinguish_infeasible_unbounded: bool,
        time_limit: Option<f64>,
        mip_rel_gap: Option<f64>,
        node_limit: Option<u64>,
        threads: Option<usize>,
        log: bool,
    ) -> PyResult<Bound<'py, PyDict>> {
        let objective = self.objective.as_ref().ok_or_else(|| {
            PyValueError::new_err("no objective set: call Model.set_objective(...) before solve()")
        })?;
        let root_solver = match lp_solver {
            Some(s) => RootSolver::parse(s)?,
            None => RootSolver::Auto,
        };

        // 整数最適化の設定には指定のまま渡す (未指定なら環境変数 `ENOMOTO_MIP_TREE_THREADS` が効く)
        let settings = MipSettings { time_limit, rel_gap: mip_rel_gap, node_limit, threads };
        let threads = threads.unwrap_or_else(crate::mip::default_threads).clamp(1, crate::mip::max_threads());
        let opts = LpOptions { distinguish_infeasible_unbounded, threads, ..Default::default() };
        let sink = if log { Some(py.import_bound("sys")?.getattr("stdout")?.unbind()) } else { None };
        let result = py.allow_threads(move || {
            let run = || crate::log::with_sink(sink, || solve_mip(&self.variables, objective, &self.constraints, root_solver, opts, settings));
            // 前処理・成分の並列求解などの rayon の並列区間も `threads` 本に収める
            match rayon::ThreadPoolBuilder::new().num_threads(threads).thread_name(|i| format!("enomoto-{i}")).build() {
                Ok(pool) => pool.install(run),
                Err(_) => run(),
            }
        });

        // Python に返す結果 dict
        let dict = PyDict::new_bound(py);
        dict.set_item(pyo3::intern!(py, "status"), result.status.as_str())?;
        if result.x.is_some() {
            dict.set_item(pyo3::intern!(py, "objective"), result.objective)?;
            dict.set_item(pyo3::intern!(py, "x"), result.x)?;
        } else {
            dict.set_item(pyo3::intern!(py, "objective"), py.None())?;
            dict.set_item(pyo3::intern!(py, "x"), py.None())?;
        }
        match result.mip {
            Some(m) => {
                dict.set_item(pyo3::intern!(py, "best_bound"), m.best_bound)?;
                dict.set_item(pyo3::intern!(py, "mip_gap"), m.gap)?;
                dict.set_item(pyo3::intern!(py, "nodes"), m.nodes)?;
                dict.set_item(pyo3::intern!(py, "lp_iterations"), m.lp_iterations)?;
                let opt = |v: f64| if v.is_nan() { None } else { Some(v) };
                dict.set_item(pyo3::intern!(py, "root_bound"), opt(m.root_bound))?;
                dict.set_item(pyo3::intern!(py, "first_solution_time"), opt(m.first_solution_time))?;
                dict.set_item(pyo3::intern!(py, "first_solution_objective"), opt(m.first_solution_objective))?;
            }
            None => {
                for k in ["best_bound", "mip_gap", "nodes", "lp_iterations", "root_bound", "first_solution_time", "first_solution_objective"] {
                    dict.set_item(k, py.None())?;
                }
            }
        }
        dict.set_item(pyo3::intern!(py, "node_limit_hit"), result.node_limit_hit)?;
        Ok(dict)
    }
}
