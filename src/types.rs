//! クレート内の各層で受け渡す共通データ型。PyO3 境界 (`model.rs`) が Python 側の
//! `Model` / `Variable` / `Function` / `Constraint` からこれらを組み立て、各ソルバー
//! (`simplex`、`interior_point`、`mip`) は Python を意識せずにこれらだけを扱う。

use pyo3::exceptions::PyValueError;
use pyo3::PyResult;
use std::collections::BTreeMap;

/// 決定変数の種類。二値変数専用の種類はなく、二値変数は境界 `[0, 1]` の `Integer` で表す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarType {
    /// 連続変数。
    Continuous,
    /// 整数変数 (分枝限定法の対象)。
    Integer,
}

impl VarType {
    /// FFI 境界から渡される文字列 (`"continuous"` / `"integer"`。Python 側の
    /// `types.py` で正規化済み) を解釈する。それ以外は `ValueError`。
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

/// 目的関数の向き (最小化/最大化)。ソルバー内部はすべて最小化形で扱い、
/// 最大化は各ソルバーの標準形を作る時点で目的係数の符号を反転して処理する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sense {
    /// 最小化。
    Minimize,
    /// 最大化。
    Maximize,
}

impl Sense {
    /// `"minimize"` / `"maximize"` を解釈する。それ以外は `ValueError`。
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

/// 制約行の比較演算子。制約は Rust に届く時点で `expr <sense> rhs` の形に正規化済み。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowSense {
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `==`
    Eq,
}

impl RowSense {
    /// `"<="` / `">="` / `"=="` を解釈する。それ以外は `ValueError`。
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

    /// 両辺に -1 を掛けたときの向き (`<=` と `>=` を入れ替え、`==` はそのまま)。
    /// 不等式の向きを一方にそろえたい標準形の構築用 (現在は未使用)。
    pub fn flip(self) -> Self {
        match self {
            RowSense::Le => RowSense::Ge,
            RowSense::Ge => RowSense::Le,
            RowSense::Eq => RowSense::Eq,
        }
    }
}

/// `solver::solve_lp` (および MIP の各ノードの緩和問題) が使う LP エンジンの選択。
/// Python の `Model.solve(root_solver=...)` に対応し、既定は `"simplex"`。
/// 両エンジンは前処理 (`crate::presolve`) だけを共有する独立実装なので、
/// 同じ問題で結果を突き合わせる検証に使える。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootSolver {
    /// 拡張双対単体法 (既定)。
    Simplex,
    /// IP-PMM 内点法。
    Interior,
}

impl RootSolver {
    /// `"simplex"` / `"interior"` を解釈する。それ以外は `ValueError`。
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

/// 1 つの決定変数の種類と境界。境界は `+/-inf` でもよい (NaN と `lb > ub` は
/// `model.rs::add_variable` で弾かれる)。
#[derive(Debug, Clone)]
pub struct VariableData {
    /// 変数の種類 (連続/整数)。
    pub vtype: VarType,
    /// 下限。
    pub lb: f64,
    /// 上限。
    pub ub: f64,
}

/// 変数の疎な線形結合と定数項: `sum(coeffs[j] * x_j) + constant`。
/// Python の `Function` の `{index: coeff}` 表現を `model.rs::to_linear_expr` で
/// 組み立て直したもの。
///
/// `HashMap` ではなく `BTreeMap` を使うのは、行の項の並び順を実行ごとに固定する
/// ため。退化した問題では項の順序がタイブレークを左右し、ピボット列や実行時間が
/// 実行ごとに変わりうる (経緯は `docs/_history_fragments/misc.md`)。
#[derive(Debug, Clone)]
pub struct LinearExpr {
    /// 変数番号 → 係数 (番号の昇順に並ぶ)。
    pub coeffs: BTreeMap<usize, f64>,
    /// 定数項。
    pub constant: f64,
}

/// `Model.set_objective(...)` で登録された目的関数: 式と最適化の向き。
#[derive(Debug, Clone)]
pub struct Objective {
    /// 目的関数の式。
    pub expr: LinearExpr,
    /// 最小化/最大化。
    pub sense: Sense,
}

/// `Model.add_constraint(...)` で登録された制約 1 本を `expr <sense> rhs` に
/// 正規化したもの (例: Python の `x + y <= 10` → `expr = x + y`, `sense = Le`, `rhs = 10`)。
#[derive(Debug, Clone)]
pub struct ConstraintRow {
    /// 左辺の式 (定数項は `rhs` 側に移してある)。
    pub expr: LinearExpr,
    /// 比較演算子。
    pub sense: RowSense,
    /// 右辺の定数。
    pub rhs: f64,
}

/// 求解の結果状態。どのソルバーもアルゴリズムによらずこのいずれかを返す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// 最適解が見つかった。
    Optimal,
    /// 実行可能解が存在しない。
    Infeasible,
    /// 目的関数が非有界。
    Unbounded,
    /// 有限の最適値を持たないこと (実行不能か非有界のどちらか) は証明したが、
    /// どちらかは区別していない。拡張双対単体法の段階 A (傾き問題) が `z^1 < 0` で
    /// 終わった場合 (論文の `prop:trichotomy`)。
    /// [`LpOptions::distinguish_infeasible_unbounded`] が `false` (既定) のときだけ返る。
    /// `true` なら段階 B まで進めて `Infeasible` か `Unbounded` を返す。
    InfeasibleOrUnbounded,
    /// どの判定にも至らずに諦めた (回復できない特異な基底や反復上限など)。
    /// 問題自体については何も主張しない。
    NotSolved,
}

/// 求解ごとのオプション (モデルの中身ではなく、何を報告するかを変えるもの)。
/// `Model.solve` から LP エンジンまで渡される。
#[derive(Debug, Clone, Copy, Default)]
pub struct LpOptions {
    /// `false` (既定): 結果は `Optimal` / `Infeasible` / [`Status::InfeasibleOrUnbounded`]
    /// のいずれか。段階 A が `z^1 < 0` ならそこで止まり、`z^1 = 0` なら段階 B で
    /// 最適解を求めるか実行不能を証明する。
    /// `true`: `z^1 < 0` の場合も段階 B を実行して `Infeasible` / `Unbounded` を区別する。
    /// このとき前処理の「改善方向レイによる非有界判定」の近道は使わない
    /// (問題の残りの実行可能性を確認できないため)。
    pub distinguish_infeasible_unbounded: bool,
}

impl Status {
    /// Python に返す dict の `"status"` キーに入れる文字列
    /// (Python 側の `Solution.status`)。
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

/// 求解結果。`solver::solve_lp` と `mip::solve_mip` の戻り値。
#[derive(Debug, Clone)]
pub struct SolveResult {
    /// 結果状態。
    pub status: Status,
    /// 目的関数値 (`status == Optimal` のときだけ `Some`)。
    pub objective: Option<f64>,
    /// 解ベクトル (元の変数の順。`status == Optimal` のときだけ `Some`)。
    pub x: Option<Vec<f64>>,
    /// MIP がノード数上限で打ち切られたか (最良暫定解を返しており、最適性は未証明)。
    /// 通常の LP では常に `false`。
    pub node_limit_hit: bool,
}
