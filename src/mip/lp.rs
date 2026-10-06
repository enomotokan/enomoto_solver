//! 分枝切除法のための、状態を保持する LP エンジン (`LpEngine`)。
//!
//! 既存の LP 経路 (`solver::solve_lp`) は前処理 → 求解 → 後処理を 1 回で行い、
//! 基底や LU を捨ててしまう。MIP では同じ LP を境界・行・費用を少しずつ変えながら
//! 何千回も解き直すので、ここでは次を保持し続ける:
//!
//! - スケール済みの行列 (列方向 CSC と行方向 CSR の両方)
//! - 基底 (各変数の状態) と LU 分解 (`simplex::lu::FtLu`、Forrest-Tomlin 更新)
//! - 主の値 `x`、被約費用 `d`、双対最急辺 (DSE) の重み
//!
//! # 定式化
//!
//! 構造変数 `x_j` (`j < n`) と、各行 `i` の活動量を表す論理変数 `r_i = a_i x`
//! (変数番号 `n + i`) を持つ:
//!
//! ```text
//!   min  c^T x   s.t.  A x - r = 0,   lo_k <= (x, r)_k <= up_k
//! ```
//!
//! 行の上下限は論理変数の境界として持つので、`<=`・`>=`・`=`・範囲制約をすべて同じ形で扱える。
//! 行の追加 (カット) は論理変数を基底に入れて行うので、基底は常に正方に保たれる。
//!
//! # 解法
//!
//! - 双対単体法 (主): DSE による行選択、境界反転付き (BFRT) の Harris 比率判定。
//!   双対実行可能でない出発点では、境界の反転と費用のずらし (cost shifting) で双対実行可能にしてから解き、
//!   最後にずらしを外して残った双対の不実行可能を主単体法で片付ける (HiGHS と同じ方式)。
//! - 主単体法 (従): 後片付けと、費用だけを変えた解き直し (Feasibility Pump) に使う。
//!
//! # スケーリング
//!
//! 内部では行・列を 2 のべき乗でスケールした問題を解く (丸め誤差を生まない)。公開 API の入出力は
//! すべて元のスケールで行う。

use crate::simplex::lu as sparse_lu;
use std::time::Instant;

/// 主の実行可能性の許容誤差 (スケール後の空間)。
const PRIMAL_TOL: f64 = 1e-7;
/// 双対の実行可能性の許容誤差 (スケール後の空間)。
const DUAL_TOL: f64 = 1e-7;
/// 比率判定で 0 とみなす係数の絶対値。
const ALPHA_TOL: f64 = 1e-9;
/// ピボットとして許す最小の絶対値。
const PIVOT_TOL: f64 = 1e-7;
/// Forrest-Tomlin 更新でこの回数を超えたら分解し直す。
const MAX_UPDATES: usize = 100;
/// DSE 重みの下限。
const DSE_FLOOR: f64 = 1e-8;
/// 費用の摂動の基準の大きさ (HiGHS の dual_simplex_cost_perturbation_multiplier と同程度)。
const PERTURB_BASE: f64 = 5e-7;

/// 変数 (構造変数または論理変数) の基底状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarStatus {
    /// 基底変数。
    Basic,
    /// 非基底で下限にある。
    Lower,
    /// 非基底で上限にある。
    Upper,
    /// 非基底の自由変数 (値 0)。
    Zero,
}

/// 基底 (構造変数と行ごとの状態)。[`LpEngine::basis`] で取得し、[`LpEngine::set_basis`] で戻す。
#[derive(Debug, Clone, PartialEq)]
pub struct Basis {
    /// 構造変数の状態 (長さ `n`)。
    pub col: Vec<VarStatus>,
    /// 各行の論理変数の状態 (長さ `m`)。
    pub row: Vec<VarStatus>,
}

/// 1 回の [`LpEngine::solve`] の結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LpStatus {
    /// 最適解に達した。
    Optimal,
    /// 主実行不能 (双対光線を [`LpEngine::dual_ray`] で取得できる)。
    Infeasible,
    /// 主の非有界 (双対実行不能)。
    Unbounded,
    /// 目的値の下界が打ち切り値 ([`SolveLimits::cutoff`]) を超えた。
    ObjectiveBound,
    /// 反復上限に達した。
    IterationLimit,
    /// 時間上限に達した。
    TimeLimit,
    /// 数値的な理由で解けなかった。
    Error,
}

/// [`LpEngine::solve`] の打ち切り条件。
#[derive(Debug, Clone, Copy)]
pub struct SolveLimits {
    /// 反復回数の上限。
    pub iteration_limit: u64,
    /// 目的値 (元のスケール、定数項なし) の下界がこれを超えたら [`LpStatus::ObjectiveBound`] で止める。
    pub cutoff: f64,
    /// 時刻の上限。
    pub deadline: Option<Instant>,
}

impl Default for SolveLimits {
    fn default() -> Self {
        SolveLimits { iteration_limit: u64::MAX, cutoff: f64::INFINITY, deadline: None }
    }
}

/// 強分岐で使う、LP の状態の保存 ([`LpEngine::save_state`] / [`LpEngine::restore_state`])。
#[derive(Clone)]
pub struct LpState {
    lo: Vec<f64>,
    up: Vec<f64>,
    status: Vec<VarStatus>,
    basic_var: Vec<usize>,
    x: Vec<f64>,
    d: Vec<f64>,
    dse: Vec<f64>,
    lu: Option<sparse_lu::FtLu>,
    updates: usize,
    fresh: Fresh,
    last_status: Option<LpStatus>,
}

/// どの量が現在の基底・境界と整合しているか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fresh {
    /// LU が現在の基底を表している。
    lu: bool,
    /// 基底変数の値 `x_B` が非基底の値と整合している。
    primal: bool,
    /// 被約費用 `d` が現在の費用と整合している。
    dual: bool,
}

/// 状態を保持する LP エンジン。モジュールの説明を参照。
#[derive(Clone)]
pub struct LpEngine {
    /// 構造変数の数。
    n: usize,
    /// 行の数。
    m: usize,
    /// 列スケール (`x = cs * x'`)。
    cs: Vec<f64>,
    /// 行スケール (`r' = rs * r`)。
    rs: Vec<f64>,
    /// スケール後の行列の行方向表現 (行ごとの (列, 値))。
    rows: Vec<Vec<(usize, f64)>>,
    /// スケール後の行列の列方向表現。`rows` から作り直す。
    col_start: Vec<usize>,
    col_idx: Vec<usize>,
    col_val: Vec<f64>,
    /// スケール後の費用 (長さ `n + m`、論理変数は 0)。
    cost: Vec<f64>,
    /// 費用のずらし + 摂動 (長さ `n + m`)。作業用の費用は `cost + shift`。
    shift: Vec<f64>,
    /// スケール後の下限・上限 (長さ `n + m`)。
    lo: Vec<f64>,
    up: Vec<f64>,
    /// 変数ごとの基底状態 (長さ `n + m`)。
    status: Vec<VarStatus>,
    /// 基底位置 → 変数番号 (長さ `m`)。
    basic_var: Vec<usize>,
    /// 全変数の値 (スケール後)。
    x: Vec<f64>,
    /// 全変数の被約費用 (作業用の費用に対するもの)。基底変数は 0。
    d: Vec<f64>,
    /// 基底位置ごとの DSE 重み。
    dse: Vec<f64>,
    lu: Option<sparse_lu::FtLu>,
    /// 最後の分解からの更新回数。
    updates: usize,
    fresh: Fresh,
    /// 直近の求解の結果。
    last_status: Option<LpStatus>,
    /// 直近の求解で主実行不能を示した行 (BTRAN の結果、スケール後の行空間)。
    ray: Option<Vec<f64>>,
    /// 累計の単体法反復数。
    total_iters: u64,
    /// 擬似乱数の状態 (摂動用)。
    rng: u64,
    // 作業領域
    work_m: Vec<f64>,
    work_m2: Vec<f64>,
    work_m3: Vec<f64>,
    scratch: Vec<f64>,
    alpha_row: Vec<f64>,
}

/// 2 のべき乗に丸める (丸め誤差を生まないスケール係数にするため)。
fn pow2(v: f64) -> f64 {
    if !v.is_finite() || v <= 0.0 {
        return 1.0;
    }
    2f64.powi(v.log2().round() as i32)
}

impl LpEngine {
    /// LP を作る。`rows[i]` は行 `i` の (列, 係数)、行の境界は `row_lo[i] <= a_i x <= row_up[i]`。
    /// 初期基底は全論理変数基底 (構造変数は費用の符号に合う側の境界)。
    pub fn new(
        col_lo: &[f64],
        col_up: &[f64],
        cost: &[f64],
        rows: &[Vec<(usize, f64)>],
        row_lo: &[f64],
        row_up: &[f64],
    ) -> Self {
        let n = col_lo.len();
        let m = rows.len();
        assert_eq!(col_up.len(), n);
        assert_eq!(cost.len(), n);
        assert_eq!(row_lo.len(), m);
        assert_eq!(row_up.len(), m);
        let (cs, rs) = Self::compute_scaling(n, rows);
        let mut srows: Vec<Vec<(usize, f64)>> = Vec::with_capacity(m);
        for (i, r) in rows.iter().enumerate() {
            let mut row: Vec<(usize, f64)> = r.iter().filter(|&&(_, v)| v != 0.0).map(|&(j, v)| (j, v * rs[i] * cs[j])).collect();
            row.sort_unstable_by_key(|&(j, _)| j);
            srows.push(row);
        }
        let nt = n + m;
        let mut lo = vec![0.0; nt];
        let mut up = vec![0.0; nt];
        let mut c = vec![0.0; nt];
        for j in 0..n {
            lo[j] = col_lo[j] / cs[j];
            up[j] = col_up[j] / cs[j];
            c[j] = cost[j] * cs[j];
        }
        for i in 0..m {
            lo[n + i] = row_lo[i] * rs[i];
            up[n + i] = row_up[i] * rs[i];
        }
        let mut e = LpEngine {
            n,
            m,
            cs,
            rs,
            rows: srows,
            col_start: Vec::new(),
            col_idx: Vec::new(),
            col_val: Vec::new(),
            cost: c,
            shift: vec![0.0; nt],
            lo,
            up,
            status: vec![VarStatus::Lower; nt],
            basic_var: (n..nt).collect(),
            x: vec![0.0; nt],
            d: vec![0.0; nt],
            dse: vec![1.0; m],
            lu: None,
            updates: 0,
            fresh: Fresh { lu: false, primal: false, dual: false },
            last_status: None,
            ray: None,
            total_iters: 0,
            rng: 0x9E37_79B9_7F4A_7C15,
            work_m: Vec::new(),
            work_m2: Vec::new(),
            work_m3: Vec::new(),
            scratch: Vec::new(),
            alpha_row: Vec::new(),
        };
        e.rebuild_cols();
        for i in 0..m {
            e.status[n + i] = VarStatus::Basic;
        }
        for j in 0..n {
            e.status[j] = e.default_nonbasic(j, e.cost[j]);
            e.x[j] = e.nonbasic_value(j);
        }
        e.resize_work();
        e
    }

    /// 幾何平均スケーリング (行・列を交互に数回)。係数は 2 のべき乗に丸める。
    fn compute_scaling(n: usize, rows: &[Vec<(usize, f64)>]) -> (Vec<f64>, Vec<f64>) {
        let m = rows.len();
        let mut cs = vec![1.0; n];
        let mut rs = vec![1.0; m];
        // 係数の最大比が小さければスケールしない。
        let mut amin = f64::INFINITY;
        let mut amax = 0.0f64;
        for r in rows {
            for &(_, v) in r {
                let a = v.abs();
                if a > 0.0 {
                    amin = amin.min(a);
                    amax = amax.max(a);
                }
            }
        }
        if amax == 0.0 || amax / amin <= 16.0 {
            return (cs, rs);
        }
        for _ in 0..6 {
            // 行
            for (i, r) in rows.iter().enumerate() {
                let (mut lo, mut hi) = (f64::INFINITY, 0.0f64);
                for &(j, v) in r {
                    let a = v.abs() * cs[j];
                    if a > 0.0 {
                        lo = lo.min(a);
                        hi = hi.max(a);
                    }
                }
                if hi > 0.0 {
                    rs[i] = 1.0 / (lo * hi).sqrt();
                }
            }
            // 列
            let mut clo = vec![f64::INFINITY; n];
            let mut chi = vec![0.0f64; n];
            for (i, r) in rows.iter().enumerate() {
                for &(j, v) in r {
                    let a = v.abs() * rs[i];
                    if a > 0.0 {
                        clo[j] = clo[j].min(a);
                        chi[j] = chi[j].max(a);
                    }
                }
            }
            for j in 0..n {
                if chi[j] > 0.0 {
                    cs[j] = 1.0 / (clo[j] * chi[j]).sqrt();
                }
            }
        }
        for v in cs.iter_mut() {
            *v = pow2(*v);
        }
        for v in rs.iter_mut() {
            *v = pow2(*v);
        }
        (cs, rs)
    }

    /// `rows` から列方向表現を作り直す。
    fn rebuild_cols(&mut self) {
        let n = self.n;
        let mut cnt = vec![0usize; n + 1];
        for r in &self.rows {
            for &(j, _) in r {
                cnt[j + 1] += 1;
            }
        }
        for j in 0..n {
            cnt[j + 1] += cnt[j];
        }
        let nnz = cnt[n];
        let mut idx = vec![0usize; nnz];
        let mut val = vec![0.0; nnz];
        let mut pos = cnt.clone();
        for (i, r) in self.rows.iter().enumerate() {
            for &(j, v) in r {
                idx[pos[j]] = i;
                val[pos[j]] = v;
                pos[j] += 1;
            }
        }
        self.col_start = cnt;
        self.col_idx = idx;
        self.col_val = val;
    }

    fn resize_work(&mut self) {
        let m = self.m;
        let nt = self.n + self.m;
        self.work_m.resize(m, 0.0);
        self.work_m2.resize(m, 0.0);
        self.work_m3.resize(m, 0.0);
        self.scratch.resize(m.max(1), 0.0);
        self.alpha_row.resize(nt, 0.0);
    }

    // ------------------------------------------------------------------
    // 公開 API: 問題の大きさ・境界・費用
    // ------------------------------------------------------------------

    /// 構造変数の数。
    pub fn num_cols(&self) -> usize {
        self.n
    }

    /// 行の数。
    pub fn num_rows(&self) -> usize {
        self.m
    }

    /// 累計の単体法反復数。
    pub fn total_iterations(&self) -> u64 {
        self.total_iters
    }

    /// 列 `j` の境界 (元のスケール)。
    pub fn col_bounds(&self, j: usize) -> (f64, f64) {
        (self.lo[j] * self.cs[j], self.up[j] * self.cs[j])
    }

    /// 行 `i` の境界 (元のスケール)。
    pub fn row_bounds(&self, i: usize) -> (f64, f64) {
        let k = self.n + i;
        (self.lo[k] / self.rs[i], self.up[k] / self.rs[i])
    }

    /// 行 `i` の係数 (元のスケール、列番号順)。
    pub fn row(&self, i: usize) -> Vec<(usize, f64)> {
        let rs = self.rs[i];
        self.rows[i].iter().map(|&(j, v)| (j, v / (rs * self.cs[j]))).collect()
    }

    /// 費用 (元のスケール)。
    pub fn costs(&self) -> Vec<f64> {
        (0..self.n).map(|j| self.cost[j] / self.cs[j]).collect()
    }

    /// 列 `j` の境界を変える (元のスケール)。
    pub fn set_col_bounds(&mut self, j: usize, lo: f64, up: f64) {
        self.set_var_bounds(j, lo / self.cs[j], up / self.cs[j]);
    }

    /// 行 `i` の境界を変える (元のスケール)。
    pub fn set_row_bounds(&mut self, i: usize, lo: f64, up: f64) {
        let rs = self.rs[i];
        self.set_var_bounds(self.n + i, lo * rs, up * rs);
    }

    fn set_var_bounds(&mut self, k: usize, lo: f64, up: f64) {
        if self.lo[k] == lo && self.up[k] == up {
            return;
        }
        self.lo[k] = lo;
        self.up[k] = up;
        if self.status[k] != VarStatus::Basic {
            let old = self.x[k];
            let st = self.status[k];
            // 状態が新しい境界で表せなければ、表せる側へ移す。
            let new_st = match st {
                VarStatus::Lower if lo.is_finite() => VarStatus::Lower,
                VarStatus::Upper if up.is_finite() => VarStatus::Upper,
                VarStatus::Zero if !lo.is_finite() && !up.is_finite() => VarStatus::Zero,
                _ => self.default_nonbasic(k, self.d[k]),
            };
            self.status[k] = new_st;
            let v = self.nonbasic_value(k);
            self.x[k] = v;
            if v != old {
                self.fresh.primal = false;
            }
            if new_st != st {
                // 状態が変わると双対実行可能性が崩れうる (求解の冒頭で直す)。
                self.fresh.dual = self.fresh.dual && self.dual_ok_at(k);
            }
        }
        self.last_status = None;
    }

    /// 費用を差し替える (元のスケール、長さ `n`)。
    pub fn set_costs(&mut self, c: &[f64]) {
        assert_eq!(c.len(), self.n);
        for j in 0..self.n {
            self.cost[j] = c[j] * self.cs[j];
        }
        self.fresh.dual = false;
        self.last_status = None;
    }

    // ------------------------------------------------------------------
    // 公開 API: 行の追加・削除
    // ------------------------------------------------------------------

    /// 行を追加する (元のスケール)。新しい行の論理変数は基底に入る。
    pub fn add_rows(&mut self, new_rows: &[(Vec<(usize, f64)>, f64, f64)]) {
        if new_rows.is_empty() {
            return;
        }
        let n = self.n;
        // 論理変数の番号が n + i なので、全変数配列の末尾に追加すればよい。
        for (r, rlo, rup) in new_rows {
            let mut amax = 0.0f64;
            for &(j, v) in r {
                amax = amax.max((v * self.cs[j]).abs());
            }
            let rsi = if amax > 0.0 { pow2(1.0 / amax) } else { 1.0 };
            let mut row: Vec<(usize, f64)> = r.iter().filter(|&&(_, v)| v != 0.0).map(|&(j, v)| (j, v * rsi * self.cs[j])).collect();
            row.sort_unstable_by_key(|&(j, _)| j);
            // 現在の値での活動量
            let act: f64 = row.iter().map(|&(j, v)| v * self.x[j]).sum();
            self.rows.push(row);
            self.rs.push(rsi);
            self.cost.push(0.0);
            self.shift.push(0.0);
            self.lo.push(rlo * rsi);
            self.up.push(rup * rsi);
            self.status.push(VarStatus::Basic);
            self.x.push(act);
            self.d.push(0.0);
            self.basic_var.push(n + self.m);
            self.dse.push(1.0);
            self.m += 1;
        }
        self.rebuild_cols();
        self.resize_work();
        self.fresh.lu = false;
        self.lu = None;
        self.last_status = None;
    }

    /// `remove[i]` が真の行を削除する。残る行の番号は詰められる。
    /// 削除する行の論理変数が非基底なら、基底を作り直す (基底の大きさを保つため)。
    pub fn delete_rows(&mut self, remove: &[bool]) {
        assert_eq!(remove.len(), self.m);
        if !remove.iter().any(|&r| r) {
            return;
        }
        let n = self.n;
        let old_basis = self.basis();
        let mut new_index = vec![usize::MAX; self.m];
        let mut cnt = 0;
        for i in 0..self.m {
            if !remove[i] {
                new_index[i] = cnt;
                cnt += 1;
            }
        }
        // 削除する行の論理変数が非基底のとき、その分だけ基底の構造変数を外す必要がある。
        let mut need_repair = false;
        for i in 0..self.m {
            if remove[i] && old_basis.row[i] != VarStatus::Basic {
                need_repair = true;
            }
        }
        // DSE 重みを基底位置ごとに保持し直す (論理変数基底の行は削除される)。
        let mut dse_of_var: Vec<(usize, f64)> = Vec::with_capacity(self.m);
        for s in 0..self.m {
            dse_of_var.push((self.basic_var[s], self.dse[s]));
        }
        let old_m = self.m;
        let keep = |k: usize| k < n || !remove[k - n];
        let remap = |k: usize| if k < n { k } else { n + new_index[k - n] };
        let mut rows = Vec::with_capacity(cnt);
        let mut rs = Vec::with_capacity(cnt);
        for i in 0..self.m {
            if !remove[i] {
                rows.push(std::mem::take(&mut self.rows[i]));
                rs.push(self.rs[i]);
            }
        }
        let filter = |v: &Vec<f64>| -> Vec<f64> { (0..n + old_m).filter(|&k| keep(k)).map(|k| v[k]).collect() };
        self.cost = filter(&self.cost);
        self.shift = filter(&self.shift);
        self.lo = filter(&self.lo);
        self.up = filter(&self.up);
        self.x = filter(&self.x);
        self.d = filter(&self.d);
        self.status = (0..n + old_m).filter(|&k| keep(k)).map(|k| self.status[k]).collect();
        self.rows = rows;
        self.rs = rs;
        self.m = cnt;
        let mut basic_var = Vec::with_capacity(cnt);
        let mut dse = Vec::with_capacity(cnt);
        for &(k, w) in &dse_of_var {
            if keep(k) {
                basic_var.push(remap(k));
                dse.push(w);
            }
        }
        self.basic_var = basic_var;
        self.dse = dse;
        self.rebuild_cols();
        self.resize_work();
        self.lu = None;
        self.fresh = Fresh { lu: false, primal: false, dual: false };
        self.last_status = None;
        if need_repair || self.basic_var.len() != self.m {
            let b = Basis { col: self.status[..n].to_vec(), row: self.status[n..].to_vec() };
            self.set_basis(&b);
        }
    }

    // ------------------------------------------------------------------
    // 公開 API: 基底
    // ------------------------------------------------------------------

    /// 現在の基底。
    pub fn basis(&self) -> Basis {
        Basis { col: self.status[..self.n].to_vec(), row: self.status[self.n..].to_vec() }
    }

    /// 基底を設定する。基底変数の数が行数と合わなければ修復する。
    pub fn set_basis(&mut self, b: &Basis) {
        assert_eq!(b.col.len(), self.n);
        assert_eq!(b.row.len(), self.m);
        let n = self.n;
        let nt = n + self.m;
        for k in 0..nt {
            self.status[k] = if k < n { b.col[k] } else { b.row[k - n] };
        }
        let mut basic: Vec<usize> = (0..nt).filter(|&k| self.status[k] == VarStatus::Basic).collect();
        if basic.len() > self.m {
            // 多すぎる: 構造変数から外す (後ろから)。
            let mut excess = basic.len() - self.m;
            for k in (0..n).rev() {
                if excess == 0 {
                    break;
                }
                if self.status[k] == VarStatus::Basic {
                    self.status[k] = self.default_nonbasic(k, self.cost[k]);
                    excess -= 1;
                }
            }
            basic = (0..nt).filter(|&k| self.status[k] == VarStatus::Basic).collect();
        }
        if basic.len() < self.m {
            // 足りない: 論理変数を基底に入れる。
            let mut lack = self.m - basic.len();
            for i in 0..self.m {
                if lack == 0 {
                    break;
                }
                if self.status[n + i] != VarStatus::Basic {
                    self.status[n + i] = VarStatus::Basic;
                    lack -= 1;
                }
            }
            basic = (0..nt).filter(|&k| self.status[k] == VarStatus::Basic).collect();
        }
        for k in 0..nt {
            if self.status[k] != VarStatus::Basic {
                let st = self.status[k];
                let ok = match st {
                    VarStatus::Lower => self.lo[k].is_finite(),
                    VarStatus::Upper => self.up[k].is_finite(),
                    VarStatus::Zero => !self.lo[k].is_finite() && !self.up[k].is_finite(),
                    VarStatus::Basic => true,
                };
                if !ok {
                    self.status[k] = self.default_nonbasic(k, self.cost[k]);
                }
                self.x[k] = self.nonbasic_value(k);
            }
        }
        self.basic_var = basic;
        self.dse = vec![1.0; self.m];
        self.lu = None;
        self.fresh = Fresh { lu: false, primal: false, dual: false };
        self.last_status = None;
    }

    /// 基底位置 `s` にある変数の番号 (`< n` なら構造変数、それ以外は行 `k - n` の論理変数)。
    pub fn basic_var(&self, s: usize) -> usize {
        self.basic_var[s]
    }

    /// 基底位置 `s` の DSE 重み。
    pub fn dse_weight(&self, s: usize) -> f64 {
        self.dse[s]
    }

    // ------------------------------------------------------------------
    // 公開 API: 解
    // ------------------------------------------------------------------

    /// 直近の求解の結果。
    pub fn last_status(&self) -> Option<LpStatus> {
        self.last_status
    }

    /// 構造変数の値 (元のスケール)。
    pub fn col_values(&self) -> Vec<f64> {
        (0..self.n).map(|j| self.x[j] * self.cs[j]).collect()
    }

    /// 列 `j` の値 (元のスケール)。
    pub fn col_value(&self, j: usize) -> f64 {
        self.x[j] * self.cs[j]
    }

    /// 行の活動量 (元のスケール)。
    pub fn row_activities(&self) -> Vec<f64> {
        (0..self.m).map(|i| self.x[self.n + i] / self.rs[i]).collect()
    }

    /// 目的値 (元のスケール、定数項なし、真の費用で計算)。
    pub fn objective(&self) -> f64 {
        (0..self.n).map(|j| self.cost[j] * self.x[j]).sum()
    }

    /// 被約費用 (元のスケール、作業用の費用に対するもの。最適で返った後は真の費用と一致する)。
    pub fn reduced_costs(&self) -> Vec<f64> {
        (0..self.n).map(|j| self.d[j] / self.cs[j]).collect()
    }

    /// 行の双対値 `y` (元のスケール。`c - A^T y` が被約費用になる符号)。
    pub fn row_duals(&self) -> Vec<f64> {
        (0..self.m).map(|i| self.d[self.n + i] * self.rs[i]).collect()
    }

    /// 直近の求解が主実行不能で終わったとき、それを示す行の重み (元のスケールの行に掛ける係数)。
    /// `sum_i w_i (a_i x - r_i) = 0` が境界のもとで満たせないことを示す。
    pub fn dual_ray(&self) -> Option<Vec<f64>> {
        let ray = self.ray.as_ref()?;
        Some((0..self.m).map(|i| ray[i] * self.rs[i]).collect())
    }

    /// 基底位置 `s` の基底逆行列の行 `e_s^T B^{-1}` を、元のスケールの行に掛ける重みとして返す。
    /// 基底変数 `k = basic_var(s)` の tableau 行は `sum_i w_i (a_i x - r_i) = 0` を
    /// `k` について解いた形になる (`k` が構造変数なら `x_k` の係数は `1 / cs` 倍の違いを含む)。
    pub fn basis_inverse_row(&mut self, s: usize) -> Vec<f64> {
        self.ensure_factor();
        let mut out = vec![0.0; self.m];
        self.btran_unit(s, &mut out);
        // スケール後の行 i の重み rho_i は、元の行 i に rho_i * rs_i を掛けたものに当たる。
        // さらに基底変数が構造変数 j なら x_j' = x_j / cs_j なので、x_j の係数を 1 にするには cs_j で割る。
        let k = self.basic_var[s];
        let colscale = if k < self.n { 1.0 / self.cs[k] } else { self.rs[k - self.n] };
        (0..self.m).map(|i| out[i] * self.rs[i] / colscale).collect()
    }

    // ------------------------------------------------------------------
    // 公開 API: 状態の保存と復元 (強分岐用)
    // ------------------------------------------------------------------

    /// 境界・基底・LU を保存する。
    pub fn save_state(&self) -> LpState {
        LpState {
            lo: self.lo.clone(),
            up: self.up.clone(),
            status: self.status.clone(),
            basic_var: self.basic_var.clone(),
            x: self.x.clone(),
            d: self.d.clone(),
            dse: self.dse.clone(),
            lu: self.lu.clone(),
            updates: self.updates,
            fresh: self.fresh,
            last_status: self.last_status,
        }
    }

    /// [`Self::save_state`] で保存した状態に戻す (行の追加・削除をまたいではならない)。
    pub fn restore_state(&mut self, st: &LpState) {
        assert_eq!(st.status.len(), self.n + self.m);
        self.lo.clone_from(&st.lo);
        self.up.clone_from(&st.up);
        self.status.clone_from(&st.status);
        self.basic_var.clone_from(&st.basic_var);
        self.x.clone_from(&st.x);
        self.d.clone_from(&st.d);
        self.dse.clone_from(&st.dse);
        self.lu = st.lu.clone();
        self.updates = st.updates;
        self.fresh = st.fresh;
        self.last_status = st.last_status;
    }

    // ------------------------------------------------------------------
    // 内部: 補助
    // ------------------------------------------------------------------

    fn next_rand(&mut self) -> f64 {
        // xorshift64*
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64) / ((1u64 << 53) as f64)
    }

    /// 被約費用 `dk` に合う非基底状態。
    fn default_nonbasic(&self, k: usize, dk: f64) -> VarStatus {
        let (l, u) = (self.lo[k], self.up[k]);
        match (l.is_finite(), u.is_finite()) {
            (true, true) => {
                if dk < 0.0 && l != u {
                    VarStatus::Upper
                } else {
                    VarStatus::Lower
                }
            }
            (true, false) => VarStatus::Lower,
            (false, true) => VarStatus::Upper,
            (false, false) => VarStatus::Zero,
        }
    }

    fn nonbasic_value(&self, k: usize) -> f64 {
        match self.status[k] {
            VarStatus::Lower => self.lo[k],
            VarStatus::Upper => self.up[k],
            VarStatus::Zero => 0.0,
            VarStatus::Basic => self.x[k],
        }
    }

    fn dual_ok_at(&self, k: usize) -> bool {
        let dk = self.d[k];
        match self.status[k] {
            VarStatus::Basic => true,
            _ if self.lo[k] == self.up[k] => true,
            VarStatus::Lower => dk >= -DUAL_TOL,
            VarStatus::Upper => dk <= DUAL_TOL,
            VarStatus::Zero => dk.abs() <= DUAL_TOL,
        }
    }

    /// 変数 `k` の列 (スケール後) を密ベクトル `out` (長さ m、事前に 0) に書く。
    fn scatter_col(&self, k: usize, scale: f64, out: &mut [f64]) {
        if k < self.n {
            for p in self.col_start[k]..self.col_start[k + 1] {
                out[self.col_idx[p]] += scale * self.col_val[p];
            }
        } else {
            out[k - self.n] -= scale;
        }
    }

    /// 変数 `k` の列と密ベクトル `v` の内積。
    fn col_dot(&self, k: usize, v: &[f64]) -> f64 {
        if k < self.n {
            let mut s = 0.0;
            for p in self.col_start[k]..self.col_start[k + 1] {
                s += self.col_val[p] * v[self.col_idx[p]];
            }
            s
        } else {
            -v[k - self.n]
        }
    }

    fn ftran(&mut self, rhs: &[f64], out: &mut [f64]) {
        if self.m == 0 {
            return;
        }
        let lu = self.lu.as_ref().expect("LU");
        lu.solve_into(rhs, &mut self.scratch, out);
    }

    fn btran(&mut self, rhs: &[f64], out: &mut [f64]) {
        if self.m == 0 {
            return;
        }
        let lu = self.lu.as_ref().expect("LU");
        lu.solve_transpose_into(rhs, &mut self.scratch, out);
    }

    fn btran_unit(&mut self, s: usize, out: &mut [f64]) {
        let lu = self.lu.as_ref().expect("LU");
        lu.solve_transpose_unit(s, &mut self.scratch, out);
    }

    /// 基底を分解する。特異なら、構造変数を論理変数で置き換えながら分解できるまで修復する。
    fn factor(&mut self) -> bool {
        let m = self.m;
        if m == 0 {
            self.lu = None;
            self.updates = 0;
            self.fresh.lu = true;
            return true;
        }
        let rows = self.basis_rows();
        let prev = self.lu.take();
        if let Some(lu) = sparse_lu::factorize_reusing(m, &rows, prev.as_ref()) {
            self.lu = Some(lu);
            self.updates = 0;
            self.fresh.lu = true;
            return true;
        }
        // 特異: 従属な基底列を見つけて論理変数に置き換える。
        self.repair_singular_basis();
        let rows = self.basis_rows();
        match sparse_lu::factorize(m, &rows).map(sparse_lu::FtLu::new) {
            Some(lu) => {
                self.lu = Some(lu);
                self.updates = 0;
                self.fresh = Fresh { lu: true, primal: false, dual: false };
                true
            }
            None => {
                // それでも駄目なら全論理変数基底にする。
                self.slack_basis();
                let rows = self.basis_rows();
                self.lu = sparse_lu::factorize(m, &rows).map(sparse_lu::FtLu::new);
                self.updates = 0;
                self.fresh = Fresh { lu: self.lu.is_some(), primal: false, dual: false };
                self.lu.is_some()
            }
        }
    }

    /// 基底行列の行リスト (行 i → (基底位置, 値))。
    fn basis_rows(&self) -> Vec<Vec<(usize, f64)>> {
        let mut rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); self.m];
        for (s, &k) in self.basic_var.iter().enumerate() {
            if k < self.n {
                for p in self.col_start[k]..self.col_start[k + 1] {
                    rows[self.col_idx[p]].push((s, self.col_val[p]));
                }
            } else {
                rows[k - self.n].push((s, -1.0));
            }
        }
        rows
    }

    /// 全論理変数基底にする。
    fn slack_basis(&mut self) {
        let n = self.n;
        for k in 0..n {
            if self.status[k] == VarStatus::Basic {
                self.status[k] = self.default_nonbasic(k, self.cost[k]);
                self.x[k] = self.nonbasic_value(k);
            }
        }
        for i in 0..self.m {
            self.status[n + i] = VarStatus::Basic;
        }
        self.basic_var = (n..n + self.m).collect();
        self.dse = vec![1.0; self.m];
    }

    /// 基底列の従属性を、部分ピボット付きの消去で調べ、従属な構造変数の列を
    /// 未ピボットの行の論理変数に置き換える。
    fn repair_singular_basis(&mut self) {
        let m = self.m;
        let n = self.n;
        // 列ごとの疎ベクトル (行 → 値)。
        let mut cols: Vec<Vec<(usize, f64)>> = Vec::with_capacity(m);
        for &k in &self.basic_var {
            let mut c = Vec::new();
            if k < n {
                for p in self.col_start[k]..self.col_start[k + 1] {
                    c.push((self.col_idx[p], self.col_val[p]));
                }
            } else {
                c.push((k - n, -1.0));
            }
            cols.push(c);
        }
        // 左から順に、既にピボットした列で消去して残りから最大要素をピボットに選ぶ (密な作業ベクトル)。
        let mut pivot_row_of_col: Vec<Option<usize>> = vec![None; m];
        let mut row_pivoted = vec![false; m];
        // ピボット済み列の消去後ベクトル (行 → 値) とピボット行。
        let mut done: Vec<(usize, Vec<(usize, f64)>)> = Vec::new();
        let mut work = vec![0.0; m];
        // 論理変数の列を先に処理する (論理変数は必ず独立に残したい)。
        let mut order: Vec<usize> = (0..m).collect();
        order.sort_by_key(|&s| (self.basic_var[s] < n) as u8);
        for &s in &order {
            for &(i, v) in &cols[s] {
                work[i] += v;
            }
            for (pr, vec) in &done {
                let f = work[*pr];
                if f != 0.0 {
                    for &(i, v) in vec {
                        work[i] -= f * v;
                    }
                    work[*pr] = 0.0;
                }
            }
            let mut best = (usize::MAX, 0.0f64);
            let mut cmax = 0.0f64;
            for &v in work.iter() {
                cmax = cmax.max(v.abs());
            }
            for i in 0..m {
                if !row_pivoted[i] && work[i].abs() > best.1 {
                    best = (i, work[i].abs());
                }
            }
            if best.0 != usize::MAX && best.1 > 1e-9 * cmax.max(1.0) {
                let pr = best.0;
                let pv = work[pr];
                let mut vec = Vec::new();
                for i in 0..m {
                    if work[i] != 0.0 && i != pr {
                        vec.push((i, work[i] / pv));
                    }
                }
                row_pivoted[pr] = true;
                pivot_row_of_col[s] = Some(pr);
                done.push((pr, vec));
            }
            for v in work.iter_mut() {
                *v = 0.0;
            }
        }
        // 従属な列を、ピボットされなかった行の論理変数で置き換える。
        let mut free_rows: Vec<usize> = (0..m).filter(|&i| !row_pivoted[i]).collect();
        for s in 0..m {
            if pivot_row_of_col[s].is_none() {
                let k = self.basic_var[s];
                let Some(i) = free_rows.pop() else { break };
                if k != n + i {
                    self.status[k] = self.default_nonbasic(k, self.cost[k]);
                    self.x[k] = self.nonbasic_value(k);
                    self.status[n + i] = VarStatus::Basic;
                    self.basic_var[s] = n + i;
                    self.dse[s] = 1.0;
                }
            }
        }
    }

    fn ensure_factor(&mut self) -> bool {
        if !self.fresh.lu || (self.lu.is_none() && self.m > 0) {
            if !self.factor() {
                return false;
            }
        }
        true
    }

    /// 非基底の値から基底変数の値を計算する: `B x_B = -N x_N`。
    fn compute_primal(&mut self) {
        let m = self.m;
        let mut rhs = std::mem::take(&mut self.work_m);
        rhs.iter_mut().for_each(|v| *v = 0.0);
        for k in 0..self.n + m {
            if self.status[k] != VarStatus::Basic {
                let v = self.x[k];
                if v != 0.0 {
                    self.scatter_col(k, -v, &mut rhs);
                }
            }
        }
        let mut sol = std::mem::take(&mut self.work_m2);
        if m > 0 {
            self.ftran(&rhs, &mut sol);
        }
        for s in 0..m {
            let k = self.basic_var[s];
            self.x[k] = sol[s];
        }
        self.work_m = rhs;
        self.work_m2 = sol;
        self.fresh.primal = true;
    }

    /// 作業用の費用から双対値と被約費用を計算する。
    fn compute_dual(&mut self) {
        let m = self.m;
        let mut cb = std::mem::take(&mut self.work_m);
        for s in 0..m {
            let k = self.basic_var[s];
            cb[s] = self.cost[k] + self.shift[k];
        }
        let mut y = std::mem::take(&mut self.work_m2);
        if m > 0 {
            self.btran(&cb, &mut y);
        }
        for k in 0..self.n + m {
            if self.status[k] == VarStatus::Basic {
                self.d[k] = 0.0;
            } else {
                self.d[k] = self.cost[k] + self.shift[k] - self.col_dot(k, &y);
            }
        }
        self.work_m = cb;
        self.work_m2 = y;
        self.fresh.dual = true;
    }

    /// 任意の `y` (作業用でなく真の費用から計算) で得られるラグランジュ下界 (スケール後の目的値)。
    fn true_dual_bound(&mut self) -> f64 {
        let m = self.m;
        let mut cb = vec![0.0; m];
        for s in 0..m {
            cb[s] = self.cost[self.basic_var[s]];
        }
        let mut y = vec![0.0; m];
        if m > 0 {
            self.btran(&cb, &mut y);
        }
        let mut bound = 0.0;
        for k in 0..self.n + m {
            let dk = self.cost[k] - self.col_dot(k, &y);
            if dk.abs() <= 1e-12 {
                continue;
            }
            let v = if dk > 0.0 { self.lo[k] } else { self.up[k] };
            if !v.is_finite() {
                return f64::NEG_INFINITY;
            }
            bound += dk * v;
        }
        bound
    }

    fn primal_infeasibility(&self, k: usize) -> f64 {
        let v = self.x[k];
        if v < self.lo[k] - PRIMAL_TOL {
            self.lo[k] - v
        } else if v > self.up[k] + PRIMAL_TOL {
            v - self.up[k]
        } else {
            0.0
        }
    }

    fn count_primal_infeasible(&self) -> usize {
        self.basic_var.iter().filter(|&&k| self.primal_infeasibility(k) > 0.0).count()
    }

    fn count_dual_infeasible(&self) -> usize {
        (0..self.n + self.m).filter(|&k| !self.dual_ok_at(k)).count()
    }

    /// 双対の不実行可能を、境界の反転 (両側有限) か費用のずらしで解消する。
    /// 反転したら主の値が変わるので `fresh.primal` を落とす。
    fn make_dual_feasible(&mut self) {
        let mut flipped = false;
        for k in 0..self.n + self.m {
            if self.status[k] == VarStatus::Basic || self.lo[k] == self.up[k] {
                continue;
            }
            let dk = self.d[k];
            let st = self.status[k];
            let bad = match st {
                VarStatus::Lower => dk < -DUAL_TOL,
                VarStatus::Upper => dk > DUAL_TOL,
                VarStatus::Zero => dk.abs() > DUAL_TOL,
                VarStatus::Basic => false,
            };
            if !bad {
                continue;
            }
            let boxed = self.lo[k].is_finite() && self.up[k].is_finite();
            if boxed {
                self.status[k] = if dk < 0.0 { VarStatus::Upper } else { VarStatus::Lower };
                self.x[k] = self.nonbasic_value(k);
                flipped = true;
            } else {
                // 費用をずらして、少し余裕のある側に寄せる。
                let target = match st {
                    VarStatus::Lower => DUAL_TOL * (1.0 + self.next_rand()),
                    VarStatus::Upper => -DUAL_TOL * (1.0 + self.next_rand()),
                    _ => 0.0,
                };
                let delta = target - dk;
                self.shift[k] += delta;
                self.d[k] = target;
            }
        }
        if flipped {
            self.fresh.primal = false;
        }
    }

    /// 費用に小さな摂動を加える (双対単体法の退化対策)。
    fn perturb_costs(&mut self) {
        for k in 0..self.n {
            if self.status[k] == VarStatus::Basic || self.lo[k] == self.up[k] {
                continue;
            }
            let c = self.cost[k];
            let mag = PERTURB_BASE * (1.0 + c.abs()) * (1.0 + self.next_rand());
            let dir = match self.status[k] {
                VarStatus::Lower => 1.0,
                VarStatus::Upper => -1.0,
                _ => 0.0,
            };
            self.shift[k] += dir * mag;
        }
        self.fresh.dual = false;
    }

    fn has_shift(&self) -> bool {
        self.shift.iter().any(|&v| v != 0.0)
    }

    fn time_up(lim: &SolveLimits) -> bool {
        match lim.deadline {
            Some(t) => Instant::now() >= t,
            None => false,
        }
    }

    // ------------------------------------------------------------------
    // 公開 API: 求解
    // ------------------------------------------------------------------

    /// 現在の基底から解く (双対単体法、必要なら最後に主単体法)。
    pub fn solve(&mut self, lim: &SolveLimits) -> LpStatus {
        let st = self.solve_inner(lim, false);
        self.last_status = Some(st);
        st
    }

    /// 主実行可能な基底から主単体法で解く (費用だけを変えた後の解き直し用)。
    /// 主実行可能でなければ [`Self::solve`] と同じ。
    pub fn solve_primal(&mut self, lim: &SolveLimits) -> LpStatus {
        let st = self.solve_inner(lim, true);
        self.last_status = Some(st);
        st
    }

    fn solve_inner(&mut self, lim: &SolveLimits, prefer_primal: bool) -> LpStatus {
        self.ray = None;
        let start_iters = self.total_iters;
        let mut attempts = 0;
        let mut perturbed = false;
        loop {
            attempts += 1;
            if !self.ensure_factor() {
                return LpStatus::Error;
            }
            if !self.fresh.primal {
                self.compute_primal();
            }
            if !self.fresh.dual {
                self.compute_dual();
            }
            let pinf = self.count_primal_infeasible();
            let dinf = self.count_dual_infeasible();
            if pinf == 0 && dinf == 0 && !self.has_shift() {
                return LpStatus::Optimal;
            }
            let remaining = |e: &Self| lim.iteration_limit.saturating_sub(e.total_iters - start_iters);
            let st = if dinf > 0 && (pinf == 0 || prefer_primal && pinf == 0) {
                self.primal_simplex(lim, remaining(self))
            } else {
                if dinf > 0 {
                    self.make_dual_feasible();
                    if !self.fresh.primal {
                        self.compute_primal();
                    }
                }
                // 初めから解く (全論理変数基底の) 場合は退化に備えて摂動する。
                if !perturbed && self.basic_var.iter().all(|&k| k >= self.n) && self.n > 0 && !self.has_shift() {
                    perturbed = true;
                    self.perturb_costs();
                    self.compute_dual();
                    self.make_dual_feasible();
                    if !self.fresh.primal {
                        self.compute_primal();
                    }
                }
                self.dual_simplex(lim, remaining(self))
            };
            match st {
                LpStatus::Optimal => {
                    if self.has_shift() {
                        // ずらしを外して確認し直す。
                        self.shift.iter_mut().for_each(|v| *v = 0.0);
                        self.compute_dual();
                        if attempts > 20 {
                            return LpStatus::Error;
                        }
                        continue;
                    }
                    // 最新の値で最終確認 (ループ先頭で判定する)
                    self.fresh.primal = false;
                    self.fresh.dual = false;
                    if !self.factor() {
                        return LpStatus::Error;
                    }
                    if attempts > 20 {
                        return LpStatus::Error;
                    }
                    continue;
                }
                LpStatus::Infeasible => {
                    self.shift.iter_mut().for_each(|v| *v = 0.0);
                    self.fresh.dual = false;
                    return LpStatus::Infeasible;
                }
                LpStatus::ObjectiveBound => {
                    self.shift.iter_mut().for_each(|v| *v = 0.0);
                    self.fresh.dual = false;
                    return LpStatus::ObjectiveBound;
                }
                LpStatus::Unbounded => {
                    if self.has_shift() {
                        // 摂動のせいかもしれないので外して続ける。
                        self.shift.iter_mut().for_each(|v| *v = 0.0);
                        self.fresh.dual = false;
                        if attempts > 20 {
                            return LpStatus::Error;
                        }
                        continue;
                    }
                    return LpStatus::Unbounded;
                }
                LpStatus::Error => {
                    if attempts > 3 {
                        self.shift.iter_mut().for_each(|v| *v = 0.0);
                        self.fresh.dual = false;
                        return LpStatus::Error;
                    }
                    // 分解し直してやり直す。
                    self.fresh = Fresh { lu: false, primal: false, dual: false };
                    continue;
                }
                other => {
                    self.shift.iter_mut().for_each(|v| *v = 0.0);
                    self.fresh.dual = false;
                    return other;
                }
            }
        }
    }

    /// 非基底変数 `k` の、比率判定に使う「動ける向き」の係数の符号を考慮した値を返す。
    /// (双対単体法用) 戻り値は (候補か, 双対の比率の分子)。
    #[inline]
    fn dual_ratio_candidate(&self, k: usize, a: f64) -> Option<f64> {
        // a = sigma * alpha_rk。d_k - t a が双対実行可能に留まる t の上限を考える。
        match self.status[k] {
            VarStatus::Basic => None,
            _ if self.lo[k] == self.up[k] => None,
            VarStatus::Lower => {
                if a > ALPHA_TOL {
                    Some(self.d[k].max(0.0))
                } else {
                    None
                }
            }
            VarStatus::Upper => {
                if a < -ALPHA_TOL {
                    Some(self.d[k].min(0.0))
                } else {
                    None
                }
            }
            VarStatus::Zero => {
                if a.abs() > ALPHA_TOL {
                    Some(self.d[k])
                } else {
                    None
                }
            }
        }
    }

    /// 双対単体法 (第 2 段階)。双対実行可能な基底から始める。
    fn dual_simplex(&mut self, lim: &SolveLimits, iter_budget: u64) -> LpStatus {
        let n = self.n;
        let m = self.m;
        let nt = n + m;
        let cutoff_scaled = lim.cutoff; // 目的のスケールは不変 (費用 * cs と x / cs)
        let mut iters: u64 = 0;
        let mut rho = vec![0.0; m];
        let mut col = vec![0.0; m];
        let mut aq = vec![0.0; m];
        let mut tau = vec![0.0; m];
        let mut flip_col = vec![0.0; m];
        let mut dflip = vec![0.0; m];
        let mut cands: Vec<(usize, f64, f64)> = Vec::new(); // (k, ratio, |a|)
        let mut flips: Vec<usize> = Vec::new();
        let mut nz_rows: Vec<usize> = Vec::new();
        loop {
            if iters >= iter_budget {
                return LpStatus::IterationLimit;
            }
            if iters % 32 == 0 && Self::time_up(lim) {
                return LpStatus::TimeLimit;
            }
            if self.updates >= MAX_UPDATES {
                if !self.factor() {
                    return LpStatus::Error;
                }
                self.compute_primal();
                self.compute_dual();
                if self.count_dual_infeasible() > 0 {
                    self.make_dual_feasible();
                    if !self.fresh.primal {
                        self.compute_primal();
                    }
                }
            }
            // 目的値による打ち切り: 作業用の費用での双対目的値が打ち切り値を超えたら、真の費用で確かめる。
            if cutoff_scaled.is_finite() && iters % 8 == 0 {
                let dobj: f64 = (0..nt).map(|k| (self.cost[k] + self.shift[k]) * self.x[k]).sum();
                if dobj > cutoff_scaled {
                    let b = self.true_dual_bound();
                    if b > cutoff_scaled {
                        return LpStatus::ObjectiveBound;
                    }
                }
            }
            // CHUZR: 最大の (不実行可能量)^2 / DSE 重み
            let mut r = usize::MAX;
            let mut best = 0.0;
            for s in 0..m {
                let k = self.basic_var[s];
                let inf = self.primal_infeasibility(k);
                if inf > 0.0 {
                    let score = inf * inf / self.dse[s];
                    if score > best {
                        best = score;
                        r = s;
                    }
                }
            }
            if r == usize::MAX {
                return LpStatus::Optimal;
            }
            let p = self.basic_var[r];
            let xp = self.x[p];
            // sigma = +1: 上限を超えている (上限へ出る)、-1: 下限を下回る (下限へ出る)
            let (sigma, bound) = if xp > self.up[p] { (1.0, self.up[p]) } else { (-1.0, self.lo[p]) };
            let delta = xp - bound; // sigma と同符号
            // BTRAN
            self.btran_unit(r, &mut rho);
            // PRICE: alpha_row[k] = rho^T a_k (非基底のみ)
            nz_rows.clear();
            for i in 0..m {
                if rho[i] != 0.0 {
                    nz_rows.push(i);
                }
            }
            let mut alpha_row = std::mem::take(&mut self.alpha_row);
            if nz_rows.len() * 10 < m {
                for &k in self.basic_var.iter() {
                    alpha_row[k] = 0.0;
                }
                for v in alpha_row[..n].iter_mut() {
                    *v = 0.0;
                }
                for &i in &nz_rows {
                    let ri = rho[i];
                    for &(j, v) in &self.rows[i] {
                        alpha_row[j] += ri * v;
                    }
                }
                for i in 0..m {
                    alpha_row[n + i] = -rho[i];
                }
            } else {
                for k in 0..nt {
                    alpha_row[k] = if self.status[k] == VarStatus::Basic { 0.0 } else { self.col_dot(k, &rho) };
                }
            }
            // CHUZC: 境界反転付き Harris 比率判定
            cands.clear();
            for k in 0..nt {
                if self.status[k] == VarStatus::Basic {
                    continue;
                }
                let a = sigma * alpha_row[k];
                if let Some(dk) = self.dual_ratio_candidate(k, a) {
                    let _ = dk;
                    cands.push((k, 0.0, a.abs()));
                }
            }
            let mut slope = delta.abs();
            flips.clear();
            let mut q = usize::MAX;
            // 候補の集合から、Harris の 2 パスで入る変数を決めることを、反転できる限り繰り返す。
            let mut remaining: Vec<(usize, f64, f64)> = cands.clone();
            loop {
                if remaining.is_empty() {
                    break;
                }
                // パス 1: 許容誤差つきの最小比率
                let mut tmax = f64::INFINITY;
                for &(k, _, aa) in &remaining {
                    let a = sigma * alpha_row[k];
                    let dk = self.d[k];
                    let t = match self.status[k] {
                        VarStatus::Lower => (dk + DUAL_TOL) / a,
                        VarStatus::Upper => (dk - DUAL_TOL) / a,
                        _ => (dk.abs() + DUAL_TOL) / aa,
                    };
                    if t < tmax {
                        tmax = t;
                    }
                }
                // パス 2: 比率が tmax 以下の候補のうち |a| 最大
                let mut group: Vec<usize> = Vec::new();
                let mut best_k = usize::MAX;
                let mut best_a = 0.0;
                for (idx, &(k, _, aa)) in remaining.iter().enumerate() {
                    let a = sigma * alpha_row[k];
                    let dk = self.d[k];
                    let t = match self.status[k] {
                        VarStatus::Zero => dk.abs() / aa,
                        _ => dk / a,
                    };
                    if t <= tmax {
                        group.push(idx);
                        if aa > best_a {
                            best_a = aa;
                            best_k = k;
                        }
                    }
                }
                if best_k == usize::MAX {
                    break;
                }
                // この群をすべて反転しても傾きが残るなら反転して続ける。
                let mut can_flip = true;
                let mut slope_drop = 0.0;
                for &idx in &group {
                    let k = remaining[idx].0;
                    let range = self.up[k] - self.lo[k];
                    if !range.is_finite() || self.status[k] == VarStatus::Zero {
                        can_flip = false;
                        break;
                    }
                    slope_drop += remaining[idx].2 * range;
                }
                if can_flip && slope - slope_drop > PRIMAL_TOL && group.len() < remaining.len() {
                    slope -= slope_drop;
                    let mut keep = Vec::with_capacity(remaining.len() - group.len());
                    let mut gi = 0;
                    for (idx, c) in remaining.iter().enumerate() {
                        if gi < group.len() && group[gi] == idx {
                            flips.push(c.0);
                            gi += 1;
                        } else {
                            keep.push(*c);
                        }
                    }
                    remaining = keep;
                    continue;
                }
                if can_flip && slope - slope_drop > PRIMAL_TOL {
                    // すべて反転しても傾きが残る = 双対非有界 (主実行不能)
                    for &idx in &group {
                        flips.push(remaining[idx].0);
                    }
                    remaining.clear();
                    break;
                }
                q = best_k;
                break;
            }
            if q == usize::MAX {
                // 主実行不能の候補: 行 rho で確かめる。
                self.alpha_row = alpha_row;
                if self.verify_infeasible_row(r, &rho) {
                    self.ray = Some(rho.clone());
                    return LpStatus::Infeasible;
                }
                // 確かめられなければ数値誤差とみなして分解し直す。
                if env_str!("ENOMOTO_DEBUG_MIPLP").is_some() {
                    eprintln!("MIPLP: infeasible row not verified r={r} flips={} sigma={sigma} delta={delta} slope={slope}", flips.len());
                    for k in 0..nt {
                        let g = self.col_dot(k, &rho);
                        if g.abs() > 1e-12 {
                            eprintln!("   k={k} st={:?} g={g} arow={} x={} lo={} up={} d={}", self.status[k], self.alpha_row[k], self.x[k], self.lo[k], self.up[k], self.d[k]);
                        }
                    }
                }
                return LpStatus::Error;
            }
            let alpha_rq_row = alpha_row[q];
            // FTRAN 入る列
            col.iter_mut().for_each(|v| *v = 0.0);
            self.scatter_col(q, 1.0, &mut col);
            self.ftran(&col, &mut aq);
            let alpha_rq = aq[r];
            if alpha_rq.abs() < PIVOT_TOL || (alpha_rq - alpha_rq_row).abs() > 1e-6 * (1.0 + alpha_rq.abs()) {
                self.alpha_row = alpha_row;
                if env_str!("ENOMOTO_DEBUG_MIPLP").is_some() {
                    eprintln!("MIPLP: pivot mismatch {alpha_rq} vs {alpha_rq_row} updates={}", self.updates);
                }
                if self.updates == 0 {
                    // 分解直後でも不一致なら諦める。
                    return LpStatus::Error;
                }
                if !self.factor() {
                    return LpStatus::Error;
                }
                self.compute_primal();
                self.compute_dual();
                if self.count_dual_infeasible() > 0 {
                    self.make_dual_feasible();
                    if !self.fresh.primal {
                        self.compute_primal();
                    }
                }
                continue;
            }
            // 反転の反映
            if !flips.is_empty() {
                flip_col.iter_mut().for_each(|v| *v = 0.0);
                for &k in &flips {
                    let (from, to) = match self.status[k] {
                        VarStatus::Lower => (self.lo[k], self.up[k]),
                        _ => (self.up[k], self.lo[k]),
                    };
                    self.status[k] = if self.status[k] == VarStatus::Lower { VarStatus::Upper } else { VarStatus::Lower };
                    self.x[k] = to;
                    self.scatter_col(k, to - from, &mut flip_col);
                }
                self.ftran(&flip_col, &mut dflip);
                for s in 0..m {
                    let kb = self.basic_var[s];
                    self.x[kb] -= dflip[s];
                }
            }
            // DSE 用の tau = B^{-1} rho
            self.ftran(&rho, &mut tau);
            // 主の更新 (反転後の x_p で)
            let xp = self.x[p];
            let theta_p = (xp - bound) / alpha_rq;
            for s in 0..m {
                let a = aq[s];
                if a != 0.0 {
                    let kb = self.basic_var[s];
                    self.x[kb] -= theta_p * a;
                }
            }
            self.x[q] += theta_p;
            self.x[p] = bound;
            // 双対の更新
            let theta_d = self.d[q] / alpha_rq;
            for k in 0..nt {
                if self.status[k] != VarStatus::Basic {
                    let a = alpha_row[k];
                    if a != 0.0 {
                        self.d[k] -= theta_d * a;
                    }
                }
            }
            self.d[q] = 0.0;
            self.d[p] = -theta_d;
            // DSE 重みの更新
            let wr = self.dse[r];
            for s in 0..m {
                if s == r {
                    continue;
                }
                let a = aq[s];
                if a != 0.0 {
                    let ratio = a / alpha_rq;
                    let w = self.dse[s] - 2.0 * ratio * tau[s] + ratio * ratio * wr;
                    self.dse[s] = w.max(DSE_FLOOR);
                }
            }
            self.dse[r] = (wr / (alpha_rq * alpha_rq)).max(DSE_FLOOR);
            // 基底の交換
            self.status[q] = VarStatus::Basic;
            self.status[p] = if sigma > 0.0 { VarStatus::Upper } else { VarStatus::Lower };
            if self.lo[p] == self.up[p] {
                self.status[p] = VarStatus::Lower;
            }
            self.basic_var[r] = q;
            self.alpha_row = alpha_row;
            // LU 更新
            let ok = {
                let lu = self.lu.as_mut().unwrap();
                lu.try_update(r, &col, 1e-9)
            };
            self.updates += 1;
            if !ok {
                if !self.factor() {
                    return LpStatus::Error;
                }
                self.compute_primal();
                self.compute_dual();
                if self.count_dual_infeasible() > 0 {
                    self.make_dual_feasible();
                    if !self.fresh.primal {
                        self.compute_primal();
                    }
                }
            }
            iters += 1;
            self.total_iters += 1;
        }
    }

    /// 行 `rho` (BTRAN の結果、基底位置 `r` の行) が主実行不能を示すか確かめる。
    /// `sum_k g_k x_k = 0` (g = rho^T [A, -I]) が境界のもとで満たせなければ真。
    fn verify_infeasible_row(&self, _r: usize, rho: &[f64]) -> bool {
        let n = self.n;
        let m = self.m;
        let mut lo_sum = 0.0;
        let mut up_sum = 0.0;
        let mut scale = 0.0f64;
        for k in 0..n + m {
            let g = if k < n {
                let mut s = 0.0;
                for p in self.col_start[k]..self.col_start[k + 1] {
                    s += self.col_val[p] * rho[self.col_idx[p]];
                }
                s
            } else {
                -rho[k - n]
            };
            if g.abs() < 1e-11 {
                continue;
            }
            let (a, b) = if g > 0.0 { (g * self.lo[k], g * self.up[k]) } else { (g * self.up[k], g * self.lo[k]) };
            lo_sum += a;
            up_sum += b;
            if a.is_finite() {
                scale = scale.max(a.abs());
            }
            if b.is_finite() {
                scale = scale.max(b.abs());
            }
        }
        // 丸め誤差 (相対 1e-9) を超えて符号が確かなら証明とみなす。
        let tol = 1e-9 * (1.0 + scale);
        lo_sum > tol || up_sum < -tol
    }

    /// 主単体法 (第 2 段階)。主実行可能な基底から始める。価格付けは Devex。
    fn primal_simplex(&mut self, lim: &SolveLimits, iter_budget: u64) -> LpStatus {
        let n = self.n;
        let m = self.m;
        let nt = n + m;
        let mut iters: u64 = 0;
        let mut col = vec![0.0; m];
        let mut aq = vec![0.0; m];
        let mut rho = vec![0.0; m];
        let mut devex = vec![1.0; nt];
        loop {
            if iters >= iter_budget {
                return LpStatus::IterationLimit;
            }
            if iters % 32 == 0 && Self::time_up(lim) {
                return LpStatus::TimeLimit;
            }
            if self.updates >= MAX_UPDATES {
                if !self.factor() {
                    return LpStatus::Error;
                }
                self.compute_primal();
                self.compute_dual();
            }
            // 価格付け
            let mut q = usize::MAX;
            let mut best = 0.0;
            for k in 0..nt {
                if self.status[k] == VarStatus::Basic || self.lo[k] == self.up[k] {
                    continue;
                }
                let dk = self.d[k];
                let inf = match self.status[k] {
                    VarStatus::Lower => (-dk).max(0.0),
                    VarStatus::Upper => dk.max(0.0),
                    VarStatus::Zero => dk.abs(),
                    VarStatus::Basic => 0.0,
                };
                if inf > DUAL_TOL {
                    let score = inf * inf / devex[k];
                    if score > best {
                        best = score;
                        q = k;
                    }
                }
            }
            if q == usize::MAX {
                // 主実行可能性が崩れていないか確かめる。
                if self.count_primal_infeasible() > 0 {
                    return LpStatus::Optimal; // 呼び出し側が双対で続ける
                }
                return LpStatus::Optimal;
            }
            let dir = if self.d[q] < 0.0 { 1.0 } else { -1.0 }; // x_q を増やす (+1) / 減らす (-1)
            col.iter_mut().for_each(|v| *v = 0.0);
            self.scatter_col(q, 1.0, &mut col);
            self.ftran(&col, &mut aq);
            // 比率判定 (Harris 2 パス)。x_B(t) = x_B - dir t aq
            let mut tmax = f64::INFINITY;
            for s in 0..m {
                let a = dir * aq[s];
                if a.abs() <= ALPHA_TOL {
                    continue;
                }
                let k = self.basic_var[s];
                let t = if a > 0.0 {
                    if self.lo[k].is_finite() { (self.x[k] - self.lo[k] + PRIMAL_TOL) / a } else { f64::INFINITY }
                } else if self.up[k].is_finite() {
                    (self.x[k] - self.up[k] - PRIMAL_TOL) / a
                } else {
                    f64::INFINITY
                };
                if t < tmax {
                    tmax = t;
                }
            }
            let own_range = self.up[q] - self.lo[q];
            let mut r = usize::MAX;
            let mut best_a = 0.0;
            if tmax.is_finite() {
                for s in 0..m {
                    let a = dir * aq[s];
                    if a.abs() <= ALPHA_TOL {
                        continue;
                    }
                    let k = self.basic_var[s];
                    let t = if a > 0.0 {
                        if self.lo[k].is_finite() { (self.x[k] - self.lo[k]) / a } else { f64::INFINITY }
                    } else if self.up[k].is_finite() {
                        (self.x[k] - self.up[k]) / a
                    } else {
                        f64::INFINITY
                    };
                    if t <= tmax && a.abs() > best_a {
                        best_a = a.abs();
                        r = s;
                    }
                }
            }
            let flip_only = own_range.is_finite() && (r == usize::MAX || own_range <= tmax);
            if r == usize::MAX && !flip_only {
                return LpStatus::Unbounded;
            }
            if flip_only {
                // 入る変数が反対側の境界に移るだけ
                let step = own_range;
                for s in 0..m {
                    let a = aq[s];
                    if a != 0.0 {
                        let kb = self.basic_var[s];
                        self.x[kb] -= dir * step * a;
                    }
                }
                self.status[q] = if dir > 0.0 { VarStatus::Upper } else { VarStatus::Lower };
                self.x[q] = self.nonbasic_value(q);
                iters += 1;
                self.total_iters += 1;
                continue;
            }
            let p = self.basic_var[r];
            let a_r = dir * aq[r];
            let target = if a_r > 0.0 { self.lo[p] } else { self.up[p] };
            let t = ((self.x[p] - target) / a_r).max(0.0);
            for s in 0..m {
                let a = aq[s];
                if a != 0.0 {
                    let kb = self.basic_var[s];
                    self.x[kb] -= dir * t * a;
                }
            }
            self.x[q] += dir * t;
            self.x[p] = target;
            // 双対の更新には離脱行の tableau 行が要る。
            self.btran_unit(r, &mut rho);
            let alpha_rq = aq[r];
            let theta_d = self.d[q] / alpha_rq;
            let wq = devex[q];
            for k in 0..nt {
                if self.status[k] == VarStatus::Basic || k == q {
                    continue;
                }
                let a = self.col_dot(k, &rho);
                if a != 0.0 {
                    self.d[k] -= theta_d * a;
                    let ratio = a / alpha_rq;
                    let w = ratio * ratio * wq;
                    if w > devex[k] {
                        devex[k] = w;
                    }
                }
            }
            self.d[q] = 0.0;
            self.d[p] = -theta_d;
            devex[p] = (wq / (alpha_rq * alpha_rq)).max(1.0);
            self.status[q] = VarStatus::Basic;
            self.status[p] = if a_r > 0.0 { VarStatus::Lower } else { VarStatus::Upper };
            if self.lo[p] == self.up[p] {
                self.status[p] = VarStatus::Lower;
            }
            self.basic_var[r] = q;
            // DSE 重みは主単体法では保てないので 1 に戻す
            self.dse[r] = 1.0;
            let ok = {
                let lu = self.lu.as_mut().unwrap();
                lu.try_update(r, &col, 1e-9)
            };
            self.updates += 1;
            if !ok {
                if !self.factor() {
                    return LpStatus::Error;
                }
                self.compute_primal();
                self.compute_dual();
            }
            iters += 1;
            self.total_iters += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ConstraintRow, LinearExpr, LpOptions, Objective, RootSolver, RowSense, Sense, Status, VarType, VariableData};
    use std::collections::BTreeMap;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
        fn int(&mut self, k: usize) -> usize {
            (self.next() * k as f64) as usize % k
        }
    }

    /// 乱数で LP を作る: (列下限, 列上限, 費用, 行, 行下限, 行上限)
    fn random_lp(seed: u64, n: usize, m: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<Vec<(usize, f64)>>, Vec<f64>, Vec<f64>) {
        let mut g = Rng(seed * 7919 + 13);
        let mut lo = vec![0.0; n];
        let mut up = vec![0.0; n];
        let mut c = vec![0.0; n];
        for j in 0..n {
            match g.int(4) {
                0 => {
                    lo[j] = 0.0;
                    up[j] = f64::INFINITY;
                }
                1 => {
                    lo[j] = -(g.int(5) as f64);
                    up[j] = lo[j] + 1.0 + g.int(10) as f64;
                }
                2 => {
                    lo[j] = 0.0;
                    up[j] = 1.0;
                }
                _ => {
                    lo[j] = -5.0;
                    up[j] = f64::INFINITY;
                }
            }
            c[j] = (g.next() - 0.3) * 10.0;
        }
        // 実行可能点を作り、それを満たすように行の境界を決める
        let x0: Vec<f64> = (0..n).map(|j| if up[j].is_finite() { lo[j] + (up[j] - lo[j]) * g.next() } else { lo[j] + 3.0 * g.next() }).collect();
        let mut rows = Vec::new();
        let mut rlo = Vec::new();
        let mut rup = Vec::new();
        for _ in 0..m {
            let mut r = Vec::new();
            let len = 2 + g.int(n.min(6));
            let mut used = std::collections::BTreeSet::new();
            for _ in 0..len {
                let j = g.int(n);
                if used.insert(j) {
                    let v = ((g.next() - 0.5) * 20.0).round();
                    if v != 0.0 {
                        r.push((j, v));
                    }
                }
            }
            let act: f64 = r.iter().map(|&(j, v)| v * x0[j]).sum();
            match g.int(3) {
                0 => {
                    rlo.push(f64::NEG_INFINITY);
                    rup.push(act + g.next() * 3.0);
                }
                1 => {
                    rlo.push(act - g.next() * 3.0);
                    rup.push(f64::INFINITY);
                }
                _ => {
                    rlo.push(act);
                    rup.push(act);
                }
            }
            rows.push(r);
        }
        (lo, up, c, rows, rlo, rup)
    }

    fn reference(lo: &[f64], up: &[f64], c: &[f64], rows: &[Vec<(usize, f64)>], rlo: &[f64], rup: &[f64]) -> (Status, Option<f64>) {
        let vars: Vec<VariableData> = (0..lo.len()).map(|j| VariableData { vtype: VarType::Continuous, lb: lo[j], ub: up[j] }).collect();
        let obj = Objective { expr: LinearExpr { coeffs: c.iter().enumerate().map(|(j, &v)| (j, v)).collect(), constant: 0.0 }, sense: Sense::Minimize };
        let mut cons = Vec::new();
        for (i, r) in rows.iter().enumerate() {
            let expr = LinearExpr { coeffs: r.iter().cloned().collect::<BTreeMap<_, _>>(), constant: 0.0 };
            if rlo[i] == rup[i] {
                cons.push(ConstraintRow { expr, sense: RowSense::Eq, rhs: rlo[i] });
            } else {
                if rlo[i].is_finite() {
                    cons.push(ConstraintRow { expr: expr.clone(), sense: RowSense::Ge, rhs: rlo[i] });
                }
                if rup[i].is_finite() {
                    cons.push(ConstraintRow { expr, sense: RowSense::Le, rhs: rup[i] });
                }
            }
        }
        let opts = LpOptions { distinguish_infeasible_unbounded: true, ..Default::default() };
        let r = crate::solver::solve_lp(&vars, &obj, &cons, RootSolver::Simplex, opts);
        (r.status, r.objective)
    }

    pub(super) fn random_lp_pub(seed: u64, n: usize, m: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<Vec<(usize, f64)>>, Vec<f64>, Vec<f64>) {
        random_lp(seed, n, m)
    }

    pub(super) fn reference_pub(lo: &[f64], up: &[f64], c: &[f64], rows: &[Vec<(usize, f64)>], rlo: &[f64], rup: &[f64]) -> (Status, Option<f64>) {
        reference(lo, up, c, rows, rlo, rup)
    }

    /// highspy で確認するための JSON を書き出す (デバッグ用)。
    #[allow(dead_code)]
    pub(super) fn dump_json(path: &str, lo: &[f64], up: &[f64], c: &[f64], rows: &[Vec<(usize, f64)>], rlo: &[f64], rup: &[f64]) {
        let f = |v: f64| if v.is_finite() { format!("{v:e}") } else if v > 0.0 { "1e300".into() } else { "-1e300".into() };
        let mut s = String::from("{");
        s += &format!("\"lo\":[{}],", lo.iter().map(|&v| f(v)).collect::<Vec<_>>().join(","));
        s += &format!("\"up\":[{}],", up.iter().map(|&v| f(v)).collect::<Vec<_>>().join(","));
        s += &format!("\"c\":[{}],", c.iter().map(|&v| f(v)).collect::<Vec<_>>().join(","));
        s += &format!("\"rlo\":[{}],", rlo.iter().map(|&v| f(v)).collect::<Vec<_>>().join(","));
        s += &format!("\"rup\":[{}],", rup.iter().map(|&v| f(v)).collect::<Vec<_>>().join(","));
        s += &format!("\"rows\":[{}]}}", rows.iter().map(|r| format!("[{}]", r.iter().map(|&(j, v)| format!("[{j},{}]", f(v))).collect::<Vec<_>>().join(","))).collect::<Vec<_>>().join(","));
        std::fs::write(path, s).unwrap();
    }

    /// 結果を証明書で確かめる (参照ソルバーに頼らない)。
    /// - Optimal: 主実行可能 + 双対実行可能 (被約費用と行双対の符号が境界の位置と整合)。
    /// - Infeasible: 双対光線が境界のもとで満たせない行の組合せを示す。
    /// - Unbounded: 主実行可能な点を持つ (非有界の方向そのものは検査しない)。
    fn check_certificate(e: &LpEngine, st: LpStatus, lo: &[f64], up: &[f64], c: &[f64], rows: &[Vec<(usize, f64)>], rlo: &[f64], rup: &[f64], ctx: &str) {
        let n = lo.len();
        match st {
            LpStatus::Optimal => {
                check_feasible(e, lo, up, rows, rlo, rup);
                let x = e.col_values();
                let y = e.row_duals();
                let mut d = c.to_vec();
                for (i, r) in rows.iter().enumerate() {
                    for &(j, v) in r {
                        d[j] -= y[i] * v;
                    }
                }
                let tol = 1e-6;
                for j in 0..n {
                    let s = 1.0 + c[j].abs();
                    let at_lo = (x[j] - lo[j]).abs() <= 1e-6 * (1.0 + lo[j].abs());
                    let at_up = (x[j] - up[j]).abs() <= 1e-6 * (1.0 + up[j].abs());
                    if at_lo && at_up {
                        continue;
                    }
                    if at_lo {
                        assert!(d[j] >= -tol * s, "{ctx}: col {j} at lower with d={}", d[j]);
                    } else if at_up {
                        assert!(d[j] <= tol * s, "{ctx}: col {j} at upper with d={}", d[j]);
                    } else {
                        assert!(d[j].abs() <= tol * s, "{ctx}: col {j} interior with d={}", d[j]);
                    }
                }
                let act = e.row_activities();
                for i in 0..rows.len() {
                    let at_lo = (act[i] - rlo[i]).abs() <= 1e-6 * (1.0 + rlo[i].abs());
                    let at_up = (act[i] - rup[i]).abs() <= 1e-6 * (1.0 + rup[i].abs());
                    if at_lo && at_up {
                        continue;
                    }
                    if at_lo {
                        assert!(y[i] >= -tol, "{ctx}: row {i} at lower with y={}", y[i]);
                    } else if at_up {
                        assert!(y[i] <= tol, "{ctx}: row {i} at upper with y={}", y[i]);
                    } else {
                        assert!(y[i].abs() <= tol, "{ctx}: row {i} interior with y={}", y[i]);
                    }
                }
            }
            LpStatus::Infeasible => {
                let w = e.dual_ray().expect("ray");
                // sum_i w_i (a_i x - r_i) = 0 を満たせないこと: g_j = sum_i w_i a_ij, 行側は -w_i
                let mut g = vec![0.0; n];
                for (i, r) in rows.iter().enumerate() {
                    for &(j, v) in r {
                        g[j] += w[i] * v;
                    }
                }
                let (mut mn, mut mx, mut sc) = (0.0f64, 0.0f64, 0.0f64);
                let mut add = |coef: f64, l: f64, u: f64| {
                    if coef.abs() < 1e-12 {
                        return;
                    }
                    let (a, b) = if coef > 0.0 { (coef * l, coef * u) } else { (coef * u, coef * l) };
                    mn += a;
                    mx += b;
                    if a.is_finite() { sc = sc.max(a.abs()); }
                    if b.is_finite() { sc = sc.max(b.abs()); }
                };
                for j in 0..n {
                    add(g[j], lo[j], up[j]);
                }
                for i in 0..rows.len() {
                    add(-w[i], rlo[i], rup[i]);
                }
                let tol = 1e-7 * (1.0 + sc);
                assert!(mn > tol || mx < -tol, "{ctx}: ray does not prove infeasibility ({mn}, {mx})");
            }
            LpStatus::Unbounded => check_feasible(e, lo, up, rows, rlo, rup),
            other => panic!("{ctx}: unexpected status {other:?}"),
        }
    }

    fn check_feasible(e: &LpEngine, lo: &[f64], up: &[f64], rows: &[Vec<(usize, f64)>], rlo: &[f64], rup: &[f64]) {
        let x = e.col_values();
        for j in 0..lo.len() {
            assert!(x[j] >= lo[j] - 1e-6 && x[j] <= up[j] + 1e-6, "col {j} {} not in [{}, {}]", x[j], lo[j], up[j]);
        }
        for (i, r) in rows.iter().enumerate() {
            let act: f64 = r.iter().map(|&(j, v)| v * x[j]).sum();
            assert!(act >= rlo[i] - 1e-5 * (1.0 + act.abs()) && act <= rup[i] + 1e-5 * (1.0 + act.abs()), "row {i} {act} not in [{}, {}]", rlo[i], rup[i]);
        }
    }

    #[test]
    fn random_lps_certified() {
        for seed in 0..200 {
            let n = 5 + (seed as usize % 20);
            let m = 3 + (seed as usize % 15);
            let (lo, up, c, rows, rlo, rup) = random_lp(seed, n, m);
            let mut e = LpEngine::new(&lo, &up, &c, &rows, &rlo, &rup);
            let st = e.solve(&SolveLimits::default());
            check_certificate(&e, st, &lo, &up, &c, &rows, &rlo, &rup, &format!("seed {seed}"));
        }
    }

    #[test]
    fn warm_start_after_bound_changes_certified() {
        for seed in 0..120 {
            let n = 8 + (seed as usize % 15);
            let m = 5 + (seed as usize % 10);
            let (lo, up, c, rows, rlo, rup) = random_lp(seed + 1000, n, m);
            let mut e = LpEngine::new(&lo, &up, &c, &rows, &rlo, &rup);
            if e.solve(&SolveLimits::default()) != LpStatus::Optimal {
                continue;
            }
            let mut g = Rng(seed + 77);
            let mut lo2 = lo.clone();
            let mut up2 = up.clone();
            for step in 0..4 {
                let x = e.col_values();
                let j = g.int(n);
                let v = x[j];
                if g.int(2) == 0 {
                    up2[j] = (v - 0.5).floor().max(lo2[j]);
                } else {
                    lo2[j] = (v + 0.5).ceil().min(up2[j]);
                }
                e.set_col_bounds(j, lo2[j], up2[j]);
                let st = e.solve(&SolveLimits::default());
                check_certificate(&e, st, &lo2, &up2, &c, &rows, &rlo, &rup, &format!("seed {seed} step {step}"));
                if st != LpStatus::Optimal {
                    break;
                }
            }
        }
    }

    #[test]
    fn add_and_delete_rows_certified() {
        for seed in 0..80 {
            let n = 6 + (seed as usize % 10);
            let m = 4 + (seed as usize % 8);
            let (lo, up, c, mut rows, mut rlo, mut rup) = random_lp(seed + 5000, n, m);
            let mut e = LpEngine::new(&lo, &up, &c, &rows, &rlo, &rup);
            if e.solve(&SolveLimits::default()) != LpStatus::Optimal {
                continue;
            }
            let x = e.col_values();
            let cut: Vec<(usize, f64)> = (0..3.min(n)).map(|j| (j, 1.0 + j as f64)).collect();
            let act: f64 = cut.iter().map(|&(j, v)| v * x[j]).sum();
            e.add_rows(&[(cut.clone(), f64::NEG_INFINITY, act - 0.5)]);
            rows.push(cut);
            rlo.push(f64::NEG_INFINITY);
            rup.push(act - 0.5);
            let st = e.solve(&SolveLimits::default());
            check_certificate(&e, st, &lo, &up, &c, &rows, &rlo, &rup, &format!("seed {seed} after add"));
            if st != LpStatus::Optimal {
                continue;
            }
            // 先頭の行と追加した行を交互に削除
            let mut remove = vec![false; rows.len()];
            remove[if seed % 2 == 0 { 0 } else { rows.len() - 1 }] = true;
            e.delete_rows(&remove);
            let mut k = 0;
            rows.retain(|_| { k += 1; !remove[k - 1] });
            let mut k = 0;
            rlo.retain(|_| { k += 1; !remove[k - 1] });
            let mut k = 0;
            rup.retain(|_| { k += 1; !remove[k - 1] });
            let st = e.solve(&SolveLimits::default());
            check_certificate(&e, st, &lo, &up, &c, &rows, &rlo, &rup, &format!("seed {seed} after delete"));
        }
    }

    #[test]
    fn cutoff_and_state_restore() {
        let mut tested = 0;
        for seed in 0..40 {
            let (lo, up, c, rows, rlo, rup) = random_lp(seed + 300, 15, 10);
            let mut e = LpEngine::new(&lo, &up, &c, &rows, &rlo, &rup);
            if e.solve(&SolveLimits::default()) != LpStatus::Optimal {
                continue;
            }
            let opt = e.objective();
            let saved = e.save_state();
            let x = e.col_values();
            let Some(j) = (0..15).find(|&j| x[j] - lo[j] > 0.5) else { continue };
            e.set_col_bounds(j, lo[j], lo[j]);
            let st = e.solve(&SolveLimits { cutoff: opt + 1e-9, ..Default::default() });
            assert!(matches!(st, LpStatus::ObjectiveBound | LpStatus::Optimal | LpStatus::Infeasible));
            if st == LpStatus::Optimal {
                assert!(e.objective() >= opt - 1e-6);
            }
            e.restore_state(&saved);
            assert_eq!(e.solve(&SolveLimits::default()), LpStatus::Optimal);
            assert!((e.objective() - opt).abs() < 1e-7 * (1.0 + opt.abs()));
            tested += 1;
        }
        assert!(tested > 5);
    }

    #[test]
    fn cost_change_primal_resolve_certified() {
        for seed in 0..60 {
            let (lo, up, c, rows, rlo, rup) = random_lp(seed + 9000, 12, 8);
            let mut e = LpEngine::new(&lo, &up, &c, &rows, &rlo, &rup);
            if e.solve(&SolveLimits::default()) != LpStatus::Optimal {
                continue;
            }
            let c2: Vec<f64> = c.iter().enumerate().map(|(j, &v)| if j % 2 == 0 { -v } else { v + 1.0 }).collect();
            e.set_costs(&c2);
            let st = e.solve_primal(&SolveLimits::default());
            check_certificate(&e, st, &lo, &up, &c2, &rows, &rlo, &rup, &format!("seed {seed}"));
        }
    }
}

#[cfg(test)]
mod debug_tests {
    #[test]
    #[ignore]
    fn debug_seed() {
        let seed: u64 = std::env::var("SEED").unwrap().parse().unwrap();
        let n = 5 + (seed as usize % 20);
        let m = 3 + (seed as usize % 15);
        let (lo, up, c, rows, rlo, rup) = super::tests::random_lp_pub(seed, n, m);
        let mut e = super::LpEngine::new(&lo, &up, &c, &rows, &rlo, &rup);
        let st = e.solve(&super::SolveLimits::default());
        eprintln!("status {:?} obj {}", st, e.objective());
        let x = e.col_values();
        for j in 0..n { eprintln!("x{j} = {} in [{}, {}] c={}", x[j], lo[j], up[j], c[j]); }
        for (i, r) in rows.iter().enumerate() {
            let act: f64 = r.iter().map(|&(j, v)| v * x[j]).sum();
            eprintln!("row {i}: {:?} act {act} in [{}, {}]", r, rlo[i], rup[i]);
        }
        if let Ok(path) = std::env::var("DUMP_TXT") {
            let f = |v: f64| if v.is_finite() { format!("{v:?}") } else if v > 0.0 { "inf".into() } else { "-inf".into() };
            let mut s = format!("{} {}\n", n, rows.len());
            for j in 0..n { s += &format!("{} {} {:?}\n", f(lo[j]), f(up[j]), c[j]); }
            for (i, r) in rows.iter().enumerate() { s += &format!("{} {} {} {}\n", f(rlo[i]), f(rup[i]), r.len(), r.iter().map(|&(j, v)| format!("{j} {v:?}")).collect::<Vec<_>>().join(" ")); }
            std::fs::write(path, s).unwrap();
        }
        let (rs, ro) = super::tests::reference_pub(&lo, &up, &c, &rows, &rlo, &rup);
        eprintln!("reference {:?} {:?}", rs, ro);
    }
}
