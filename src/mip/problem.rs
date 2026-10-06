//! 分枝切除法が扱う問題データ (最小化形、行は `row_lo <= a_i x <= row_up` の範囲制約)。

use crate::types::{ConstraintRow, Objective, RowSense, Sense, VarType, VariableData};

/// 整数変数の境界を丸めるときの許容誤差。
pub const INT_ROUND_TOL: f64 = 1e-6;

/// MIP の問題データ。目的は常に最小化 (`cost` は最大化なら符号反転済み)。
#[derive(Debug, Clone)]
pub struct MipProblem {
    /// 列 (変数) の数。
    pub n: usize,
    /// 行の数。
    pub m: usize,
    /// 列の下限・上限 (整数列は丸め済み)。
    pub col_lo: Vec<f64>,
    pub col_up: Vec<f64>,
    /// 最小化形の費用。
    pub cost: Vec<f64>,
    /// 目的の定数項 (最小化形)。
    pub offset: f64,
    /// 元の向きに戻すときの符号 (最小化 +1、最大化 -1)。
    pub sense_sign: f64,
    /// 整数列か。
    pub is_int: Vec<bool>,
    /// 行 (列番号の昇順の (列, 係数))。
    pub rows: Vec<Vec<(usize, f64)>>,
    /// 行の下限・上限。
    pub row_lo: Vec<f64>,
    pub row_up: Vec<f64>,
    /// 列方向の表現 (列 → (行, 係数))。
    pub cols: Vec<Vec<(usize, f64)>>,
}

impl MipProblem {
    /// モデルの変数・目的・制約から作る。整数列の境界は丸める。
    pub fn from_model(variables: &[VariableData], objective: &Objective, constraints: &[ConstraintRow]) -> Self {
        let n = variables.len();
        let sense_sign = match objective.sense {
            Sense::Minimize => 1.0,
            Sense::Maximize => -1.0,
        };
        let mut cost = vec![0.0; n];
        for (&j, &v) in &objective.expr.coeffs {
            cost[j] += sense_sign * v;
        }
        let offset = sense_sign * objective.expr.constant;
        let is_int: Vec<bool> = variables.iter().map(|v| v.vtype == VarType::Integer).collect();
        let mut col_lo: Vec<f64> = variables.iter().map(|v| v.lb).collect();
        let mut col_up: Vec<f64> = variables.iter().map(|v| v.ub).collect();
        for j in 0..n {
            if is_int[j] {
                if col_lo[j].is_finite() {
                    col_lo[j] = (col_lo[j] - INT_ROUND_TOL).ceil();
                }
                if col_up[j].is_finite() {
                    col_up[j] = (col_up[j] + INT_ROUND_TOL).floor();
                }
            }
        }
        let mut rows = Vec::with_capacity(constraints.len());
        let mut row_lo = Vec::with_capacity(constraints.len());
        let mut row_up = Vec::with_capacity(constraints.len());
        for c in constraints {
            let row: Vec<(usize, f64)> = c.expr.coeffs.iter().filter(|&(_, &v)| v != 0.0).map(|(&j, &v)| (j, v)).collect();
            let rhs = c.rhs - c.expr.constant;
            let (lo, up) = match c.sense {
                RowSense::Le => (f64::NEG_INFINITY, rhs),
                RowSense::Ge => (rhs, f64::INFINITY),
                RowSense::Eq => (rhs, rhs),
            };
            rows.push(row);
            row_lo.push(lo);
            row_up.push(up);
        }
        Self::from_rows(col_lo, col_up, cost, offset, sense_sign, is_int, rows, row_lo, row_up)
    }

    /// 行データから作る (列方向の表現をここで作る)。
    #[allow(clippy::too_many_arguments)]
    pub fn from_rows(
        col_lo: Vec<f64>,
        col_up: Vec<f64>,
        cost: Vec<f64>,
        offset: f64,
        sense_sign: f64,
        is_int: Vec<bool>,
        rows: Vec<Vec<(usize, f64)>>,
        row_lo: Vec<f64>,
        row_up: Vec<f64>,
    ) -> Self {
        let n = col_lo.len();
        let m = rows.len();
        let mut cols: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        for (i, r) in rows.iter().enumerate() {
            for &(j, v) in r {
                cols[j].push((i, v));
            }
        }
        MipProblem { n, m, col_lo, col_up, cost, offset, sense_sign, is_int, rows, row_lo, row_up, cols }
    }

    /// 点 `x` の目的値 (最小化形、定数項込み)。
    pub fn objective(&self, x: &[f64]) -> f64 {
        self.offset + (0..self.n).map(|j| self.cost[j] * x[j]).sum::<f64>()
    }

    /// 点 `x` が許容誤差 `tol` で実行可能か (整数性も確かめる)。行・境界の許容誤差は
    /// 絶対値 `tol` に、右辺の大きさに比例する丸め誤差分 (`1e-9 |rhs|`) を足したもの。
    pub fn is_feasible(&self, x: &[f64], tol: f64) -> bool {
        for j in 0..self.n {
            let v = x[j];
            if !v.is_finite() {
                return false;
            }
            if v < self.col_lo[j] - feas_tol(tol, self.col_lo[j]) || v > self.col_up[j] + feas_tol(tol, self.col_up[j]) {
                return false;
            }
            if self.is_int[j] && (v - v.round()).abs() > tol {
                return false;
            }
        }
        for i in 0..self.m {
            let act: f64 = self.rows[i].iter().map(|&(j, a)| a * x[j]).sum();
            if act < self.row_lo[i] - feas_tol(tol, self.row_lo[i]) || act > self.row_up[i] + feas_tol(tol, self.row_up[i]) {
                return false;
            }
        }
        true
    }

    /// 目的値が整数刻みしか取らないなら、その刻み (最小の正の値) を返す。
    /// 費用が非零の列がすべて整数で、係数が共通の刻みの整数倍のとき。
    pub fn objective_step(&self) -> Option<f64> {
        let mut coefs: Vec<f64> = Vec::new();
        for j in 0..self.n {
            if self.cost[j] != 0.0 {
                if !self.is_int[j] {
                    return None;
                }
                coefs.push(self.cost[j].abs());
            }
        }
        if coefs.is_empty() {
            return None;
        }
        // 係数を最小値で割ったものが整数に近ければ、最小値を刻みとし、その gcd を取る。
        let minc = coefs.iter().cloned().fold(f64::INFINITY, f64::min);
        // 有理数の刻みを小さな分母で探す (1, 2, ..., 1000 倍して整数になるか)。
        for mult in [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 100.0, 1000.0] {
            let scaled: Vec<f64> = coefs.iter().map(|&c| c * mult).collect();
            if scaled.iter().all(|&s| (s - s.round()).abs() <= 1e-9 * s.max(1.0)) {
                let mut g = 0i64;
                for s in &scaled {
                    let v = s.round() as i64;
                    g = gcd(g, v);
                }
                if g > 0 {
                    return Some(g as f64 / mult);
                }
            }
        }
        let _ = minc;
        None
    }
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// 境界 `b` に対する実行可能性の許容誤差 (絶対 `tol` + 相対 1e-9)。
#[inline]
fn feas_tol(tol: f64, b: f64) -> f64 {
    tol + 1e-9 * b.abs()
}
