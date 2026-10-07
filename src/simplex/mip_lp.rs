//! 分枝限定法から傾き・切片二段解法 ([`super::slope_intercept_dual`]) を使うためのラッパー。
//!
//! 前処理を通さずに、MIP の LP (`row_lo <= a_i x <= row_up`, `lo <= x <= up`) を標準形
//! `σ_i a_i x + s_i = b_i` (スラック `s_i >= 0`、等式行は `s_i = 0`、`>=` だけの行は `σ_i = -1`) に
//! 直して [`StdForm`] として持つ。構造列は「有限の境界のうち下側 (なければ上側) が 0」になるよう
//! 平行移動する (`x = x' + shift`、二段解法の前提)。境界の変更は平行移動量と右辺 `b` の差分更新で
//! 反映するので、行列は作り直さない。行の追加・削除と費用の変更のときだけ作り直す。
//!
//! 毎回の求解は、前回の最適基底 (基底列の集合) を渡して二段解法の warm start
//! (`solve_slope_intercept_dual_from_basis`) で解く。打ち切り条件 (反復上限・目的値・時刻) は
//! [`super::slope_intercept_dual::set_ext_control`] で渡す。

use super::slope_intercept_dual::{self as sid, ExtControl, ExtStop};
use super::{freeze_std_matrices, StdForm};
use crate::mip::lp::{Basis, LpStatus, SolveLimits, VarStatus};
use crate::types::{LpOptions, Status};

/// 二段解法を使う MIP 用の LP。
#[derive(Clone)]
pub struct TwoStageLp {
    n: usize,
    /// 元の行 (列番号の昇順) と行の境界。
    rows: Vec<Vec<(usize, f64)>>,
    row_lo: Vec<f64>,
    row_up: Vec<f64>,
    /// 元の費用と列の境界。
    cost: Vec<f64>,
    lo: Vec<f64>,
    up: Vec<f64>,
    /// 行の符号 σ_i (`>=` だけの行は -1)。
    sigma: Vec<f64>,
    /// 列の平行移動量。
    shift: Vec<f64>,
    std: StdForm,
    /// 基底 (基底列の番号、長さ m。構造列 j < n、スラック n + i)。`None` なら全スラック基底から。
    basis: Option<Vec<usize>>,
    /// 直近の解 (元の空間の構造列の値)、双対 (標準形の行)、状態。
    x: Vec<f64>,
    y: Vec<f64>,
    status: Option<LpStatus>,
    /// 直近に分解した基底の LU (tableau 行用、基底が変わったら捨てる)。
    lu: Option<super::lu::FtLu>,
    /// 直近の最適基底とその LU (次の warm start で分解を省く)。
    lu_cache: Option<(Vec<usize>, super::lu::FtLu, Option<Vec<f64>>)>,
    iters: u64,
    /// 制約行列の識別子 (作り直すたびに新しい値。PRICE 用の行列の使い回しに使う)。
    price_key: u64,
    /// 直前の求解が実行不能だったときの双対射線 (元の行の向き)。
    ray: Option<Vec<f64>>,
}

/// [`TwoStageLp::price_key`] の発行元。
static NEXT_PRICE_KEY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn new_price_key() -> u64 {
    NEXT_PRICE_KEY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// [`TwoStageLp::interior_point`] の結果 (元の空間の構造列の値)。
pub struct InteriorPoint {
    /// 内点法の解。
    pub x: Vec<f64>,
    /// クロスオーバーの押し出しで作った (基底 (標準形の列番号), 頂点 (主実行可能でなければ `None`))。
    /// 基底は双対実行可能とは限らない。
    pub vertex: Option<(Vec<usize>, Option<Vec<f64>>)>,
}

/// 状態の保存 (強分岐用)。
#[derive(Clone)]
pub struct TwoStageState {
    lo: Vec<f64>,
    up: Vec<f64>,
    basis: Option<Vec<usize>>,
    x: Vec<f64>,
    y: Vec<f64>,
    status: Option<LpStatus>,
    lu_cache: Option<(Vec<usize>, super::lu::FtLu, Option<Vec<f64>>)>,
}

fn shift_of(lo: f64, up: f64) -> f64 {
    if lo.is_finite() {
        lo
    } else if up.is_finite() {
        up
    } else {
        0.0
    }
}

impl TwoStageLp {
    pub fn new(col_lo: &[f64], col_up: &[f64], cost: &[f64], rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64]) -> Self {
        let n = col_lo.len();
        let mut sorted: Vec<Vec<(usize, f64)>> = rows.iter().map(|r| {
            let mut r: Vec<(usize, f64)> = r.iter().filter(|&&(_, v)| v != 0.0).cloned().collect();
            r.sort_unstable_by_key(|&(j, _)| j);
            r
        }).collect();
        // 同じ列の重複を足し合わせる
        for r in sorted.iter_mut() {
            let mut out: Vec<(usize, f64)> = Vec::with_capacity(r.len());
            for &(j, v) in r.iter() {
                match out.last_mut() {
                    Some(last) if last.0 == j => last.1 += v,
                    _ => out.push((j, v)),
                }
            }
            *r = out;
        }
        let m = sorted.len();
        let shift: Vec<f64> = (0..n).map(|j| shift_of(col_lo[j], col_up[j])).collect();
        let dummy = Self::build_std(n, &sorted, row_lo, row_up, cost, col_lo, col_up, &shift);
        TwoStageLp {
            n,
            rows: sorted,
            row_lo: row_lo.to_vec(),
            row_up: row_up.to_vec(),
            cost: cost.to_vec(),
            lo: col_lo.to_vec(),
            up: col_up.to_vec(),
            sigma: dummy.1,
            shift,
            std: dummy.0,
            basis: None,
            x: vec![0.0; n],
            y: vec![0.0; m],
            status: None,
            lu: None,
            lu_cache: None,
            iters: 0,
            price_key: new_price_key(),
            ray: None,
        }
    }

    /// 標準形を作る。戻り値は (標準形, 行の符号)。
    #[allow(clippy::too_many_arguments)]
    fn build_std(n: usize, rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64], cost: &[f64], lo: &[f64], up: &[f64], shift: &[f64]) -> (StdForm, Vec<f64>) {
        let m = rows.len();
        let n_total = n + m;
        let mut std_rows: Vec<Vec<(usize, f64)>> = Vec::with_capacity(m);
        let mut b = Vec::with_capacity(m);
        let mut slb = vec![0.0; n_total];
        let mut sub = vec![0.0; n_total];
        let mut sigma = Vec::with_capacity(m);
        for j in 0..n {
            slb[j] = lo[j] - shift[j];
            sub[j] = up[j] - shift[j];
        }
        for i in 0..m {
            let (l, u) = (row_lo[i], row_up[i]);
            // (σ, 右辺, スラックの上限)
            let (sg, rhs, s_up, empty) = if l.is_finite() && u.is_finite() {
                (1.0, u, u - l, false)
            } else if u.is_finite() {
                (1.0, u, f64::INFINITY, false)
            } else if l.is_finite() {
                (-1.0, -l, f64::INFINITY, false)
            } else {
                // 制約のない行: 空の行 (0 = 0) にする
                (1.0, 0.0, 0.0, true)
            };
            let mut r: Vec<(usize, f64)> = Vec::with_capacity(rows[i].len() + 1);
            let mut rhs = rhs;
            if !empty {
                for &(j, v) in &rows[i] {
                    r.push((j, sg * v));
                    rhs -= sg * v * shift[j];
                }
            }
            r.push((n + i, 1.0));
            std_rows.push(r);
            b.push(rhs);
            slb[n + i] = 0.0;
            sub[n + i] = s_up.max(0.0);
            sigma.push(sg);
        }
        let mut c = vec![0.0; n_total];
        c[..n].copy_from_slice(cost);
        let (rows_m, cols_m) = freeze_std_matrices(&std_rows, n_total);
        (StdForm { n_total, n_rows: m, c, rows: rows_m, cols: cols_m, b, lb: slb, ub: sub }, sigma)
    }

    fn rebuild(&mut self) {
        sid::xcount("n_rebuild");
        self.lu_cache = None;
        let (std, sigma) = Self::build_std(self.n, &self.rows, &self.row_lo, &self.row_up, &self.cost, &self.lo, &self.up, &self.shift);
        self.std = std;
        self.sigma = sigma;
        self.price_key = new_price_key();
        self.lu = None;
    }

    pub fn num_cols(&self) -> usize {
        self.n
    }

    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    pub fn total_iterations(&self) -> u64 {
        self.iters
    }

    pub fn col_bounds(&self, j: usize) -> (f64, f64) {
        (self.lo[j], self.up[j])
    }

    pub fn row_bounds(&self, i: usize) -> (f64, f64) {
        (self.row_lo[i], self.row_up[i])
    }

    pub fn row(&self, i: usize) -> Vec<(usize, f64)> {
        self.rows[i].clone()
    }

    pub fn costs(&self) -> Vec<f64> {
        self.cost.clone()
    }

    pub fn set_col_bounds(&mut self, j: usize, lo: f64, up: f64) {
        if self.lo[j] == lo && self.up[j] == up {
            return;
        }
        self.lo[j] = lo;
        self.up[j] = up;
        let ns = shift_of(lo, up);
        let d = ns - self.shift[j];
        if d != 0.0 {
            for &(i, v) in self.std.cols.col(j) {
                self.std.b[i] -= v * d;
            }
            self.shift[j] = ns;
        }
        self.std.lb[j] = lo - ns;
        self.std.ub[j] = up - ns;
        self.status = None;
    }

    pub fn set_row_bounds(&mut self, i: usize, lo: f64, up: f64) {
        self.row_lo[i] = lo;
        self.row_up[i] = up;
        self.rebuild();
        self.status = None;
    }

    pub fn set_costs(&mut self, c: &[f64]) {
        self.cost.copy_from_slice(c);
        self.std.c[..self.n].copy_from_slice(c);
        self.status = None;
    }

    pub fn add_rows(&mut self, new_rows: &[(Vec<(usize, f64)>, f64, f64)]) {
        let m0 = self.rows.len();
        for (r, l, u) in new_rows {
            let mut r: Vec<(usize, f64)> = r.iter().filter(|&&(_, v)| v != 0.0).cloned().collect();
            r.sort_unstable_by_key(|&(j, _)| j);
            self.rows.push(r);
            self.row_lo.push(*l);
            self.row_up.push(*u);
        }
        self.rebuild();
        // 新しい行のスラックを基底に入れる (スラックの番号は n + i なので、既存の基底は番号が変わらない)
        if let Some(b) = self.basis.as_mut() {
            for i in m0..self.rows.len() {
                b.push(self.n + i);
            }
        }
        self.y.resize(self.rows.len(), 0.0);
        self.status = None;
    }

    pub fn delete_rows(&mut self, remove: &[bool]) {
        let m = self.rows.len();
        let mut new_index = vec![usize::MAX; m];
        let mut k = 0;
        for i in 0..m {
            if !remove[i] {
                new_index[i] = k;
                k += 1;
            }
        }
        let keep = |v: &mut Vec<f64>| {
            let mut i = 0;
            v.retain(|_| {
                i += 1;
                !remove[i - 1]
            });
        };
        keep(&mut self.row_lo);
        keep(&mut self.row_up);
        keep(&mut self.y);
        let mut i = 0;
        self.rows.retain(|_| {
            i += 1;
            !remove[i - 1]
        });
        let n = self.n;
        if let Some(b) = self.basis.take() {
            let mut nb: Vec<usize> = Vec::with_capacity(k);
            for &v in &b {
                if v < n {
                    nb.push(v);
                } else if !remove[v - n] {
                    nb.push(n + new_index[v - n]);
                }
            }
            self.basis = Some(nb);
        }
        self.rebuild();
        self.repair_basis();
        self.status = None;
    }

    /// 基底列の数を行数に合わせる (多ければ構造列を外し、足りなければ非基底の行のスラックを入れる)。
    fn repair_basis(&mut self) {
        let m = self.rows.len();
        let n = self.n;
        let Some(b) = self.basis.as_mut() else { return };
        let mut used = vec![false; n + m];
        b.retain(|&v| v < n + m && !std::mem::replace(&mut used[v], true));
        while b.len() > m {
            if let Some(pos) = b.iter().rposition(|&v| v < n) {
                let v = b.remove(pos);
                used[v] = false;
            } else {
                b.pop();
            }
        }
        let mut i = 0;
        while b.len() < m && i < m {
            if !used[n + i] {
                used[n + i] = true;
                b.push(n + i);
            }
            i += 1;
        }
    }

    pub fn basis(&self) -> Basis {
        let n = self.n;
        let m = self.rows.len();
        let mut col = vec![VarStatus::Lower; n];
        let mut row = vec![VarStatus::Lower; m];
        let mut is_basic = vec![false; n + m];
        if let Some(b) = &self.basis {
            for &v in b {
                is_basic[v] = true;
            }
        } else {
            for i in 0..m {
                is_basic[n + i] = true;
            }
        }
        for j in 0..n {
            col[j] = if is_basic[j] {
                VarStatus::Basic
            } else if self.lo[j].is_finite() && (self.x[j] - self.lo[j]).abs() <= 1e-9 * (1.0 + self.lo[j].abs()) {
                VarStatus::Lower
            } else if self.up[j].is_finite() && (self.x[j] - self.up[j]).abs() <= 1e-9 * (1.0 + self.up[j].abs()) {
                VarStatus::Upper
            } else if !self.lo[j].is_finite() && !self.up[j].is_finite() {
                VarStatus::Zero
            } else if self.lo[j].is_finite() {
                VarStatus::Lower
            } else {
                VarStatus::Upper
            };
        }
        for i in 0..m {
            // スラック 0 は σ = +1 なら行が上限、σ = -1 なら下限にある
            row[i] = if is_basic[n + i] {
                VarStatus::Basic
            } else if self.sigma[i] > 0.0 {
                VarStatus::Upper
            } else {
                VarStatus::Lower
            };
        }
        Basis { col, row }
    }

    pub fn set_basis(&mut self, b: &Basis) {
        let n = self.n;
        let mut list: Vec<usize> = (0..n).filter(|&j| b.col[j] == VarStatus::Basic).collect();
        list.extend((0..b.row.len().min(self.rows.len())).filter(|&i| b.row[i] == VarStatus::Basic).map(|i| n + i));
        self.basis = Some(list);
        self.repair_basis();
        self.lu = None;
        self.status = None;
    }

    /// 今の LP (行・列の境界・費用) を内点法で解いた点 (元の空間の構造列の値) を返す。`zero_cost` なら費用 0
    /// (実行可能領域の中心付近の点)。`want_vertex` なら、その点からクロスオーバーの押し出しだけで作った頂点と
    /// 基底 (標準形の列番号: 構造列 j < n、スラック n + i) も返す。この基底は**双対実行可能とは限らない**ので、
    /// warm start ([`Self::set_basis_cols`]) にだけ使い、最適基底 (下界・被約費用) として使ってはいけない。
    /// `deadline` を過ぎたら打ち切って `None`。
    pub fn interior_point(&self, zero_cost: bool, want_vertex: bool, deadline: std::time::Instant) -> Option<InteriorPoint> {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        let token = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicBool::new(false));
        // 時間切れでトークンを立てる見張り (内点法・クロスオーバーはトークンを見て止まる)
        let watch = {
            let (token, done) = (token.clone(), done.clone());
            std::thread::spawn(move || {
                while !done.load(Ordering::Relaxed) {
                    if std::time::Instant::now() >= deadline {
                        token.store(true, Ordering::Relaxed);
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            })
        };
        let r = crate::cancel::with_token(Some(token.clone()), || super::crossover::interior_point(&self.std, zero_cost, want_vertex));
        done.store(true, Ordering::Relaxed);
        let _ = watch.join();
        if token.load(Ordering::Relaxed) {
            return None;
        }
        let r = r?;
        let n = self.n;
        let to_orig = |z: &[f64]| -> Vec<f64> { (0..n).map(|j| z[j] + self.shift[j]).collect() };
        let x = to_orig(&r.x);
        let vertex = r.vertex.map(|(b, xv)| (b, xv.map(|v| to_orig(&v))));
        Some(InteriorPoint { x, vertex })
    }

    /// 基底を標準形の列番号 (長さ m) で直接与える (特異なら修復する)。`x` を渡すと、次の求解の非基底列の
    /// 置き場所の希望 ([`Self::nb_hint`]) をその点から作る。基底は双対実行可能でなくてよい (二段解法の warm start は
    /// 非基底列を被約費用の符号の側 (無限なら記号的な M) に置くか費用をずらして始め、最後に真の費用で仕上げる)。
    pub fn set_basis_cols(&mut self, b: &[usize], x: Option<&[f64]>) {
        if b.len() != self.rows.len() {
            return;
        }
        self.basis = Some(b.to_vec());
        self.repair_basis();
        if let Some(x) = x {
            self.x.copy_from_slice(&x[..self.n]);
        }
        self.lu = None;
        self.status = None;
    }

    pub fn basic_var(&self, s: usize) -> usize {
        match &self.basis {
            Some(b) => b[s],
            None => self.n + s,
        }
    }

    pub fn dse_weight(&self, _s: usize) -> f64 {
        1.0
    }

    pub fn last_status(&self) -> Option<LpStatus> {
        self.status
    }

    pub fn save_state(&self) -> TwoStageState {
        TwoStageState { lo: self.lo.clone(), up: self.up.clone(), basis: self.basis.clone(), x: self.x.clone(), y: self.y.clone(), status: self.status, lu_cache: self.lu_cache.clone() }
    }

    pub fn restore_state(&mut self, s: &TwoStageState) {
        for j in 0..self.n {
            if self.lo[j] != s.lo[j] || self.up[j] != s.up[j] {
                self.set_col_bounds(j, s.lo[j], s.up[j]);
            }
        }
        if self.basis != s.basis {
            self.lu = None;
        }
        self.basis = s.basis.clone();
        self.lu_cache = s.lu_cache.clone();
        self.x.clone_from(&s.x);
        self.y.clone_from(&s.y);
        self.status = s.status;
    }

    pub fn solve(&mut self, lim: &SolveLimits) -> LpStatus {
        if self.iters == 0 && env_str!("ENOMOTO_MIP_XPROF").is_some() {
            sid::xprof_enable(true);
        }
        let t0 = std::time::Instant::now();
        let it0 = self.iters;
        let pol0 = sid::ext_polish_iterations();
        let st = self.solve_inner(lim);
        if env_str!("ENOMOTO_MIP_DEBUG_LP").is_some() {
            eprintln!("TSLP solve st={st:?} iters={} polish={} us={} warm={}", self.iters - it0, sid::ext_polish_iterations() - pol0, t0.elapsed().as_micros(), self.basis.is_some());
        }
        self.status = Some(st);
        st
    }

    pub fn solve_primal(&mut self, lim: &SolveLimits) -> LpStatus {
        self.solve(lim)
    }

    fn solve_inner(&mut self, lim: &SolveLimits) -> LpStatus {
        let n = self.n;
        // 目的値の定数 (平行移動の分): c·x = c·x' + c·shift
        let const_obj: f64 = (0..n).map(|j| self.cost[j] * self.shift[j]).sum();
        let ctrl = ExtControl {
            iteration_limit: usize::try_from(lim.iteration_limit).unwrap_or(usize::MAX),
            cutoff: lim.cutoff - const_obj,
            deadline: lim.deadline,
        };
        let opts = LpOptions { distinguish_infeasible_unbounded: true, ..Default::default() };
        let it0 = sid::ext_iterations();
        sid::xprof("between");
        sid::set_ext_control(Some(ctrl));
        sid::set_fast_reopt(env_str!("ENOMOTO_MIP_NO_FAST_REOPT").is_none());
        sid::request_duals(true);
        sid::request_lu(true);
        sid::set_price_key(Some(self.price_key));
        let r = match self.basis.clone() {
            Some(b) if b.len() == self.rows.len() => {
                // 同じ基底の LU が手元にあれば渡す (分解を省く)
                match self.lu_cache.take() {
                    // Forrest-Tomlin の更新が積み重なった LU は FTRAN/BTRAN が遅いので、分解し直させる
                    // (主ループの上限 3m に任せると、再分解は減るがノードの処理数はかえって減った)
                    Some((cb, lu, d)) if cb == b && lu.update_count() < tunable!("ENOMOTO_T_MIP_LU_MAX_UPD", 64usize, usize) => {
                        sid::set_warm_lu(Some(lu));
                        // 前回の求解の最終の被約費用は使わない: 主ループが増分で保つ値は求解をまたぐと誤差が積もり
                        // (相対 1e-4 程度)、価格付けしない固定列の値は更新されないので、双対実行可能性の判定を誤る
                        if env_str!("ENOMOTO_MIP_WARM_D").is_some() {
                            sid::set_warm_d(d);
                        }
                    }
                    Some((cb, _, _)) if cb == b => {
                        sid::xcount("n_upd64");
                        sid::set_warm_lu(None)
                    }
                    Some(_) => {
                        sid::xcount("n_diffb");
                        sid::set_warm_lu(None)
                    }
                    None => {
                        sid::xcount("n_nocache");
                        sid::set_warm_lu(None)
                    }
                }
                if env_str!("ENOMOTO_MIP_NO_NB_HINT").is_none() {
                    sid::set_warm_nb(Some(self.nb_hint()));
                }
                sid::solve_slope_intercept_dual_from_basis(&self.std, &opts, b)
            }
            _ => sid::solve_slope_intercept_dual(&self.std, &opts),
        };
        sid::xprof("wrapper");
        sid::set_warm_lu(None);
        sid::set_warm_d(None);
        sid::set_warm_nb(None);
        sid::set_price_key(None);
        let stop = sid::ext_stop();
        sid::set_ext_control(None);
        sid::set_fast_reopt(false);
        let duals = sid::take_duals();
        sid::request_duals(false);
        let last_lu = sid::take_last_lu();
        let last_d = sid::take_last_d();
        let ray = sid::take_last_ray();
        self.ray = None;
        sid::request_lu(false);
        self.lu_cache = None;
        self.iters += sid::ext_iterations() - it0;
        self.lu = None;
        sid::xprof("after");
        match r {
            Some(res) => match res.status {
                Status::Optimal => {
                    let xs = res.x.unwrap_or_default();
                    for j in 0..n {
                        self.x[j] = xs.get(j).copied().unwrap_or(0.0) + self.shift[j];
                    }
                    if let Some((y, bpos)) = duals {
                        self.y = y;
                        let mut b = vec![usize::MAX; self.rows.len()];
                        for (v, p) in bpos.iter().enumerate() {
                            if let Some(p) = *p {
                                if p < b.len() {
                                    b[p] = v;
                                }
                            }
                        }
                        if b.iter().all(|&v| v != usize::MAX) {
                            if let Some(lu) = last_lu {
                                self.lu_cache = Some((b.clone(), lu, last_d));
                            }
                            self.basis = Some(b);
                        }
                    }
                    LpStatus::Optimal
                }
                Status::Infeasible => {
                    // 標準形の行は元の行に符号 σ を掛けたもの
                    self.ray = ray.filter(|r| r.len() >= self.rows.len()).map(|r| (0..self.rows.len()).map(|i| r[i] * self.sigma[i]).collect());
                    LpStatus::Infeasible
                }
                Status::Unbounded | Status::InfeasibleOrUnbounded => LpStatus::Unbounded,
                _ => LpStatus::Error,
            },
            None => match stop {
                ExtStop::IterationLimit => LpStatus::IterationLimit,
                ExtStop::ObjectiveBound => {
                    // 打ち切ったときの双対 (双対証明に使う)
                    let yb = sid::take_bound_duals();
                    if yb.len() == self.rows.len() {
                        self.y = yb;
                    }
                    LpStatus::ObjectiveBound
                }
                ExtStop::TimeLimit => LpStatus::TimeLimit,
                ExtStop::None => LpStatus::Error,
            },
        }
    }

    /// 前回の解での非基底列の位置 (標準形の列ごと: -1 下限、+1 上限、0 不明)。
    fn nb_hint(&self) -> Vec<i8> {
        let n = self.n;
        let m = self.rows.len();
        let mut h = vec![0i8; n + m];
        let at = |v: f64, b: f64| b.is_finite() && (v - b).abs() <= 1e-9 * (1.0 + b.abs());
        for j in 0..n {
            let (v, lo, up) = (self.x[j], self.lo[j], self.up[j]);
            h[j] = if at(v, lo) { -1 } else if at(v, up) { 1 } else { 0 };
        }
        // スラック s_i = b_i - σ_i a_i x (元の空間で): 0 なら下限、上限 (範囲行の幅) なら上限
        for i in 0..m {
            let act: f64 = self.rows[i].iter().map(|&(j, a)| a * self.x[j]).sum();
            let (l, u) = (self.row_lo[i], self.row_up[i]);
            let sl = if self.sigma[i] > 0.0 { u - act } else { act - l };
            let su = self.std.ub[n + i];
            h[n + i] = if sl.abs() <= 1e-9 * (1.0 + act.abs()) { -1 } else if su.is_finite() && (sl - su).abs() <= 1e-9 * (1.0 + su.abs()) { 1 } else { 0 };
        }
        h
    }

    pub fn objective(&self) -> f64 {
        (0..self.n).map(|j| self.cost[j] * self.x[j]).sum()
    }

    pub fn col_values(&self) -> Vec<f64> {
        self.x.clone()
    }

    pub fn col_value(&self, j: usize) -> f64 {
        self.x[j]
    }

    pub fn row_activities(&self) -> Vec<f64> {
        self.rows.iter().map(|r| r.iter().map(|&(j, a)| a * self.x[j]).sum()).collect()
    }

    /// 被約費用 `c - A^T y` (元の空間。標準形の行の符号を考慮)。
    pub fn reduced_costs(&self) -> Vec<f64> {
        let mut d = self.cost.clone();
        for (i, r) in self.rows.iter().enumerate() {
            let yi = self.y.get(i).copied().unwrap_or(0.0) * self.sigma[i];
            if yi != 0.0 {
                for &(j, a) in r {
                    d[j] -= yi * a;
                }
            }
        }
        d
    }

    /// 行の双対値 (元の行の向き)。
    pub fn farkas_ray(&self) -> Option<Vec<f64>> {
        self.ray.clone()
    }

    pub fn row_duals(&self) -> Vec<f64> {
        (0..self.rows.len()).map(|i| self.y.get(i).copied().unwrap_or(0.0) * self.sigma[i]).collect()
    }

    /// 基底位置 `s` の `e_s^T B^{-1}` を、元の行に掛ける重みとして返す
    /// (`sum_i w_i (a_i x - r_i) = 0` が基底変数 `basic_var(s)` についての tableau 行)。
    pub fn basis_inverse_row(&mut self, s: usize) -> Vec<f64> {
        let m = self.rows.len();
        if self.lu.is_none() {
            if let (Some((cb, lu, _)), Some(b)) = (&self.lu_cache, &self.basis) {
                if cb == b {
                    self.lu = Some(lu.clone());
                }
            }
        }
        if self.lu.is_none() {
            let n_total = self.n + m;
            let mut pos: Vec<Option<usize>> = vec![None; n_total];
            match &self.basis {
                Some(b) => {
                    for (k, &v) in b.iter().enumerate() {
                        pos[v] = Some(k);
                    }
                }
                None => {
                    for i in 0..m {
                        pos[self.n + i] = Some(i);
                    }
                }
            }
            self.lu = super::basis_kernel::factorize_basis(&self.std, &pos, None);
        }
        let Some(lu) = &self.lu else { return vec![0.0; m] };
        let mut out = vec![0.0; m];
        let mut scratch = vec![0.0; m];
        lu.solve_transpose_unit(s, &mut scratch, &mut out);
        // σ_i a_i x + s_i = b_i で s_i = b_i - σ_i r_i なので、重みは ρ_i σ_i
        // (スラックが基底の行では、基底変数 s_i の係数は 1 で、r_i については -σ_i)
        let k = self.basic_var(s);
        let sc = if k >= self.n { -self.sigma[k - self.n] } else { 1.0 };
        (0..m).map(|i| out[i] * self.sigma[i] / sc).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_lp_and_bound_change() {
        // min -x - y  s.t. x + 2y <= 4, 3x + y <= 6, x, y in [0, 10]
        let mut lp = TwoStageLp::new(&[0.0, 0.0], &[10.0, 10.0], &[-1.0, -1.0], &[vec![(0, 1.0), (1, 2.0)], vec![(0, 3.0), (1, 1.0)]], &[f64::NEG_INFINITY; 2], &[4.0, 6.0]);
        assert_eq!(lp.solve(&SolveLimits::default()), LpStatus::Optimal);
        assert!((lp.objective() - (-2.8)).abs() < 1e-9, "{}", lp.objective());
        lp.set_col_bounds(0, 0.0, 1.0);
        assert_eq!(lp.solve(&SolveLimits::default()), LpStatus::Optimal);
        assert!((lp.objective() - (-2.5)).abs() < 1e-9, "{}", lp.objective());
        lp.set_col_bounds(0, 3.0, 10.0);
        assert_eq!(lp.solve(&SolveLimits::default()), LpStatus::Infeasible);
    }

    #[test]
    fn interior_point_and_crossover_vertex() {
        // min -x - y  s.t. x + y <= 4, x, y in [0, 3]: 最適面は (1, 3)-(3, 1) の線分。内点法の点はその内側、
        // 押し出しの頂点は実行可能な頂点
        let lp = TwoStageLp::new(&[0.0, 0.0], &[3.0, 3.0], &[-1.0, -1.0], &[vec![(0, 1.0), (1, 1.0)]], &[f64::NEG_INFINITY], &[4.0]);
        let far = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let ip = lp.interior_point(false, true, far).expect("interior point");
        assert!((ip.x[0] + ip.x[1] - 4.0).abs() < 1e-5, "{:?}", ip.x);
        assert!(ip.x[0] > 1.0 + 1e-3 && ip.x[0] < 3.0 - 1e-3, "not interior to the optimal face: {:?}", ip.x);
        let (b, xv) = ip.vertex.expect("vertex");
        assert_eq!(b.len(), 1);
        let xv = xv.expect("primal feasible vertex");
        assert!(xv[0] + xv[1] <= 4.0 + 1e-9 && xv.iter().all(|&v| (-1e-9..=3.0 + 1e-9).contains(&v)), "{xv:?}");
        // 費用 0: 実行可能領域の中心付近
        let ac = lp.interior_point(true, false, far).expect("analytic center");
        assert!(ac.x[0] + ac.x[1] < 4.0 - 1e-3 && ac.x.iter().all(|&v| v > 1e-3 && v < 3.0 - 1e-3), "{:?}", ac.x);
    }

    #[test]
    fn warm_start_from_dual_infeasible_basis() {
        // min -x - 2y  s.t. x + y <= 4, x, y in [0, 3]。基底 {x} (y、スラックは非基底) は被約費用
        // d_y = -2 - (-1) = -1 < 0 で双対実行可能でない。そこから始めても最適値 -7 (x = 1, y = 3) に達する
        let mut lp = TwoStageLp::new(&[0.0, 0.0], &[3.0, 3.0], &[-1.0, -2.0], &[vec![(0, 1.0), (1, 1.0)]], &[f64::NEG_INFINITY], &[4.0]);
        lp.set_basis_cols(&[0], Some(&[3.0, 0.0]));
        assert_eq!(lp.solve(&SolveLimits::default()), LpStatus::Optimal);
        assert!((lp.objective() - (-7.0)).abs() < 1e-9, "{}", lp.objective());
    }
}
