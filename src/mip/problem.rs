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
    /// プロービングで見つけた 0-1 列の含意を、同時には真にならない文字の組として持つ (文字 `2j` は `x_j = 1`、
    /// `2j + 1` は `x_j = 0`)。衝突グラフ ([`super::clique::CliqueGraph`]) の辺に加える。
    pub conflict_edges: Vec<(u32, u32)>,
    /// プロービングで見つけた含意 `x_j = v ⇒ x_k <= b` (上限) / `x_k >= b` (下限): (j, v, k, 上限か, b)。
    /// implied bound カットの分離に使う。
    pub implied_bounds: Vec<(u32, bool, u32, bool, f64)>,
    /// 行の非零の数と整数列の数 (`from_rows` で数える。作った後に `rows`・`is_int` を変えるなら数え直すこと)。
    pub nnz: usize,
    pub nint: usize,
}

impl MipProblem {
    /// 診断用: 自由 MPS 形式で書き出す (最小化形の費用と定数項、行の範囲は RANGES、整数列は MARKER)。
    pub fn write_mps(&self, path: &str) -> std::io::Result<()> {
        use std::fmt::Write as _;
        let mut o = String::new();
        let _ = writeln!(o, "NAME sub\nROWS\n N obj");
        let mut kind = Vec::with_capacity(self.m);
        for i in 0..self.m {
            let (l, u) = (self.row_lo[i], self.row_up[i]);
            let k = if l == u { 'E' } else if l.is_finite() && !u.is_finite() { 'G' } else { 'L' };
            kind.push(k);
            let _ = writeln!(o, " {k} r{i}");
        }
        let _ = writeln!(o, "COLUMNS");
        let mut cols: Vec<Vec<(usize, f64)>> = vec![Vec::new(); self.n];
        for (i, r) in self.rows.iter().enumerate() {
            for &(j, a) in r {
                cols[j].push((i, a));
            }
        }
        let mut in_int = false;
        for j in 0..self.n {
            if self.is_int[j] != in_int {
                let _ = writeln!(o, " M{j} 'MARKER' '{}'", if self.is_int[j] { "INTORG" } else { "INTEND" });
                in_int = self.is_int[j];
            }
            if self.cost[j] != 0.0 {
                let _ = writeln!(o, " c{j} obj {:.17e}", self.cost[j]);
            }
            for &(i, a) in &cols[j] {
                let _ = writeln!(o, " c{j} r{i} {a:.17e}");
            }
            if self.cost[j] == 0.0 && cols[j].is_empty() {
                let _ = writeln!(o, " c{j} obj 0");
            }
        }
        if in_int {
            let _ = writeln!(o, " Mend 'MARKER' 'INTEND'");
        }
        let _ = writeln!(o, "RHS");
        if self.offset != 0.0 {
            let _ = writeln!(o, " rhs obj {:.17e}", -self.offset);
        }
        for i in 0..self.m {
            let v = if kind[i] == 'G' { self.row_lo[i] } else if self.row_up[i].is_finite() { self.row_up[i] } else { 0.0 };
            if v != 0.0 {
                let _ = writeln!(o, " rhs r{i} {v:.17e}");
            }
        }
        let _ = writeln!(o, "RANGES");
        for i in 0..self.m {
            if kind[i] == 'L' && self.row_lo[i].is_finite() && self.row_up[i].is_finite() {
                let _ = writeln!(o, " rng r{i} {:.17e}", self.row_up[i] - self.row_lo[i]);
            }
            if kind[i] == 'L' && !self.row_up[i].is_finite() {
                // 両側とも無限の行 (自由な行) は L 0 + 範囲なしにはできないので、大きな右辺にする
            }
        }
        let _ = writeln!(o, "BOUNDS");
        for j in 0..self.n {
            let (l, u) = (self.col_lo[j], self.col_up[j]);
            if l == u {
                let _ = writeln!(o, " FX bnd c{j} {l:.17e}");
                continue;
            }
            if l.is_finite() {
                let _ = writeln!(o, " LO bnd c{j} {l:.17e}");
            } else {
                let _ = writeln!(o, " MI bnd c{j}");
            }
            if u.is_finite() {
                let _ = writeln!(o, " UP bnd c{j} {u:.17e}");
            } else if self.is_int[j] {
                let _ = writeln!(o, " PL bnd c{j}");
            }
        }
        let _ = writeln!(o, "ENDATA");
        std::fs::write(path, o)
    }
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
        // 整数列の境界は整数に丸める (伝播・カットは整数列の境界が整数であることを前提にする。前処理の伝播は
        // 整数でない境界を返しうる)。丸めて下限 > 上限になれば実行不能で、定義域が検出する。
        let (mut col_lo, mut col_up) = (col_lo, col_up);
        for j in 0..n {
            if is_int[j] {
                if col_lo[j].is_finite() {
                    col_lo[j] = (col_lo[j] - 1e-6).ceil();
                }
                if col_up[j].is_finite() {
                    col_up[j] = (col_up[j] + 1e-6).floor();
                }
            }
        }
        let mut cols: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        for (i, r) in rows.iter().enumerate() {
            for &(j, v) in r {
                cols[j].push((i, v));
            }
        }
        let nnz = rows.iter().map(|r| r.len()).sum();
        let nint = is_int.iter().filter(|&&b| b).count();
        MipProblem { n, m, col_lo, col_up, cost, offset, sense_sign, is_int, rows, row_lo, row_up, cols, conflict_edges: Vec::new(), implied_bounds: Vec::new(), nnz, nint }
    }

    /// 整数列と暗黙の整数列 (連続列だが、整数解では必ず整数値になるもの) の印。カット生成で整数として扱う
    /// (分枝はしない)。等式行 `a_j x_j + sum_k a_k x_k = b` で、他の列がすべて (暗黙の) 整数、
    /// `a_k / a_j` と `b / a_j` がすべて整数なら `x_j` は暗黙の整数 (HiGHS の implied integer の主側の判定)。
    /// 双対側の判定 (HiGHS の dual implied integer): 費用が非零で 1 本の不等式行にしか現れない列は、最適解では
    /// 境界か、費用を下げる向きに効く行の側を等号にする値を取るので、境界 (有限のもの) が整数で、その行の側と
    /// 他の列が同じ条件を満たせば暗黙の整数 (どの最適解でも整数値。`ENOMOTO_MIP_NO_DUAL_IMPLINT` で使わない)。
    /// 新しく印のついた列から更に判定が進むので、変化がなくなるまで繰り返す。
    pub fn implied_integers(&self) -> Vec<bool> {
        let integral = |v: f64| (v - v.round()).abs() <= 1e-9 * v.abs().max(1.0);
        let dual = env_str!("ENOMOTO_MIP_NO_DUAL_IMPLINT").is_none();
        let mut mark = self.is_int.clone();
        loop {
            let mut changed = false;
            for j in 0..self.n {
                if mark[j] {
                    continue;
                }
                let ok = self.cols[j].iter().any(|&(i, a)| {
                    self.row_lo[i] == self.row_up[i]
                        && integral(self.row_lo[i] / a)
                        && self.rows[i].iter().all(|&(k, b)| k == j || (mark[k] && integral(b / a)))
                }) || (dual && self.cols[j].len() == 1 && self.cost[j] != 0.0 && {
                    let (i, a) = self.cols[j][0];
                    // 費用を下げる向きに x_j を動かすのを止める行の側: c > 0 なら x_j を減らすので、a > 0 なら行の値が
                    // 減って下側、a < 0 なら上側で止まる (c < 0 なら逆)
                    let side = if (self.cost[j] > 0.0) == (a > 0.0) { self.row_lo[i] } else { self.row_up[i] };
                    [self.col_lo[j], self.col_up[j]].iter().all(|&v| !v.is_finite() || integral(v))
                        && (!side.is_finite() || integral(side / a))
                        && self.rows[i].iter().all(|&(k, b)| k == j || (mark[k] && integral(b / a)))
                });
                if ok {
                    mark[j] = true;
                    changed = true;
                }
            }
            if !changed {
                return mark;
            }
        }
    }

    /// `ENOMOTO_MIP_IMPLINT_TU` (試作、既定では使わない): `base` (整数列と [`Self::implied_integers`] の印) に加えて、連続列の部分が
    /// 完全単模 (TU) になる連続列の集まりをまとめて暗に整数とする。整数列を整数値に固定すると、残りの連続列の制約は
    /// 「TU の行列 x_C ∈ 整数の区間」になり、その多面体の頂点は整数なので、整数の最適解がある (カット生成で整数として扱える)。
    /// 候補は境界が整数か無限の連続列。行ごとに、最初の候補の係数の絶対値で割ると、候補の係数が ±1、(暗に) 整数の列の係数と
    /// 有限の左辺・右辺が整数 (固定された連続列はその寄与が整数)、候補でない連続列を含まない、でなければその行の候補を外す。
    /// 行への出現が 3 つ以上の候補も外し、残りで Heller-Tompkins の条件 (同じ符号の 2 つの行は別の組、逆の符号なら同じ組) の
    /// 2 色塗りに矛盾する列を外す。変化がなくなるまで繰り返す。physiciansched6-2 の前処理後の問題では、罰の列 s_k (自分の行と
    /// 長い行 Σ s_k − z ≤ 0 の 2 本) と z の 434 本がこれに当たる (1 列ずつの判定では互いを前提にし合って 0 本)。HiGHS は前処理で
    /// z を消して s_k を 1 行の暗に整数の列にする (424 本) (analysis/highstree_phys_20261024.md)
    pub fn implied_integers_tu(&self, base: &[bool]) -> Vec<bool> {
        let integral = |v: f64| (v - v.round()).abs() <= 1e-9 * v.abs().max(1.0);
        let n = self.n;
        let mut cand: Vec<bool> = (0..n)
            .map(|j| !base[j] && self.col_lo[j] < self.col_up[j] && [self.col_lo[j], self.col_up[j]].iter().all(|&v| !v.is_finite() || integral(v)))
            .collect();
        let fixed_cont = |j: usize| !base[j] && self.col_lo[j] == self.col_up[j];
        loop {
            let mut changed = false;
            for i in 0..self.m {
                let r = &self.rows[i];
                let Some(&(_, a0)) = r.iter().find(|&&(j, _)| cand[j]) else { continue };
                let s = 1.0 / a0.abs();
                let ok = r.iter().all(|&(j, a)| {
                    if cand[j] {
                        (a.abs() * s - 1.0).abs() <= 1e-9
                    } else if base[j] {
                        integral(a * s)
                    } else {
                        fixed_cont(j) && integral(self.col_lo[j] * a * s)
                    }
                }) && [self.row_lo[i], self.row_up[i]].iter().all(|&v| !v.is_finite() || integral(v * s));
                if !ok {
                    for &(j, _) in r {
                        if cand[j] {
                            cand[j] = false;
                            changed = true;
                        }
                    }
                }
            }
            for j in 0..n {
                if cand[j] && self.cols[j].len() > 2 {
                    cand[j] = false;
                    changed = true;
                }
            }
            // Heller-Tompkins の 2 色塗り (行が頂点、出現 2 つの候補の列が辺)
            let mut adj: Vec<Vec<(usize, bool, usize)>> = vec![Vec::new(); self.m];
            for j in 0..n {
                if cand[j] && self.cols[j].len() == 2 {
                    let ((i1, a1), (i2, a2)) = (self.cols[j][0], self.cols[j][1]);
                    let same = (a1 > 0.0) == (a2 > 0.0);
                    adj[i1].push((i2, same, j));
                    adj[i2].push((i1, same, j));
                }
            }
            let mut color: Vec<i8> = vec![-1; self.m];
            let mut bad: Vec<usize> = Vec::new();
            for r0 in 0..self.m {
                if color[r0] >= 0 || adj[r0].is_empty() {
                    continue;
                }
                color[r0] = 0;
                let mut q = std::collections::VecDeque::from([r0]);
                while let Some(r) = q.pop_front() {
                    for &(r2, same, j) in &adj[r] {
                        let want = color[r] ^ (same as i8);
                        if color[r2] < 0 {
                            color[r2] = want;
                            q.push_back(r2);
                        } else if color[r2] != want {
                            bad.push(j);
                        }
                    }
                }
            }
            for j in bad {
                if cand[j] {
                    cand[j] = false;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        (0..n).map(|j| base[j] || cand[j]).collect()
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
        self.objective_step_bounds(None)
    }

    /// [`Self::objective_step`] の本体。境界 `(lo, up)` を渡せば、そこで整数値に固定された連続列を整数列と同じに扱う。
    pub fn objective_step_bounds(&self, bounds: Option<(&[f64], &[f64])>) -> Option<f64> {
        let mut coefs: Vec<f64> = Vec::new();
        for j in 0..self.n {
            if self.cost[j] != 0.0 {
                // 整数値に固定された列は整数列と同じに扱う (寄与 c_j v_j は刻みの整数倍)
                let fixed_int = bounds.is_some_and(|(lo, up)| lo[j] == up[j] && lo[j] == lo[j].round());
                if !self.is_int[j] && !fixed_int {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implied_integers_chain_through_equalities() {
        // x0, x1 整数。s2: 2 x0 + 3 x1 + s2 = 7 → s2 は暗黙の整数。
        // s3: 2 s3 - s2 = 4 は係数比 1/2 なので整数にならない。s4: s4 + x0 - s2 = 1 → 暗黙の整数 (連鎖)。
        let inf = f64::INFINITY;
        let p = MipProblem::from_rows(
            vec![0.0; 5],
            vec![10.0, 10.0, inf, inf, inf],
            vec![0.0; 5],
            0.0,
            1.0,
            vec![true, true, false, false, false],
            vec![vec![(0, 2.0), (1, 3.0), (2, 1.0)], vec![(2, -1.0), (3, 2.0)], vec![(0, 1.0), (2, -1.0), (4, 1.0)]],
            vec![7.0, 4.0, 1.0],
            vec![7.0, 4.0, 1.0],
        );
        assert_eq!(p.implied_integers(), vec![true, true, true, false, true]);
    }

    #[test]
    fn implied_integers_tu_penalty_network() {
        // b0, b1 は 2 値。s0 >= 100 b0 - 200 (行 0)、s1 >= 300 b1 - 100 (行 1)、z >= s0 + s1 (行 2)、z の費用 1。
        // 1 列ずつの判定では s と z が互いを前提にし合って決まらないが、連続列の部分 (s0, s1, z) は TU なので 3 本とも暗に整数。
        // 行 3 (b0 + w <= 2.5) の w は右辺が整数でないので外れる
        let inf = f64::INFINITY;
        let p = MipProblem::from_rows(
            vec![0.0; 6],
            vec![1.0, 1.0, inf, inf, inf, inf],
            vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            0.0,
            1.0,
            vec![true, true, false, false, false, false],
            vec![vec![(0, 100.0), (2, -1.0)], vec![(1, 300.0), (3, -1.0)], vec![(2, 1.0), (3, 1.0), (4, -1.0)], vec![(0, 1.0), (5, 1.0)]],
            vec![-inf, -inf, -inf, -inf],
            vec![200.0, 100.0, 0.0, 2.5],
        );
        let b = p.implied_integers();
        assert_eq!(b, vec![true, true, false, false, false, false]);
        assert_eq!(p.implied_integers_tu(&b), vec![true, true, true, true, true, false]);
    }
}
