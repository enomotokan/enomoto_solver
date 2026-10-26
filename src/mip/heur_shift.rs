//! SCIP の丸め・シフト系の主ヒューリスティクス (`heur_rounding`・`heur_shifting`・`heur_intshifting`) と、
//! 暫定解の 2 列を同時に動かす改善 (`heur_twoopt`) の簡略版。
//!
//! - [`Solver::lock_rounding`] (`ENOMOTO_MIP_SHIFT_HEURS`): LP 解の小数の整数列を 1 本ずつ丸める。破れた行があれば
//!   その行の小数の列のうち、その行を直す向きに丸めて新たに破れる行の最も少ないものを丸める。破れた行がなければ
//!   lock の多い列から、新たに破れる行の少ない向き (同じなら目的値の良い向き) に丸める。
//! - [`Solver::shifting`]: 丸めに加えて、破れた行を直すために整数でない列 (整数列・連続列) もずらす。
//! - [`Solver::int_shifting`]: 整数列だけをずらす。行の範囲は連続列が動ける幅だけ広げて見て、整数列を全部
//!   整数にできたら固定し、連続列を LP で解き直す。
//! - [`Solver::two_opt`] (`ENOMOTO_MIP_TWOOPT`): 実行可能解の整数列 `j` を目的値の良くなる向きに動かすと破れる行があるとき、
//!   その行の別の整数列 `k` を逆に効く向きに同じ幅だけ動かし、目的値の合計が良くなる組を探す。

use super::domain::FEASTOL;
use super::lp_api::MipLp;
use super::solver::Solver;

/// 行の破れの判定の許容誤差 (試す点は [`super::problem::MipProblem::is_feasible`] の FEASTOL で確かめるので、それより厳しく)。
fn row_tol(b: f64) -> f64 {
    1e-9 * (1.0 + b.abs())
}

/// 行の破れの量 (範囲の内側なら 0)。
fn viol(act: f64, lo: f64, up: f64) -> f64 {
    if act < lo - row_tol(lo) {
        lo - act
    } else if act > up + row_tol(up) {
        act - up
    } else {
        0.0
    }
}

/// どの列を動かせるか。
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// 小数の整数列を丸めるだけ。
    Round,
    /// 整数列・連続列のどれでもずらす。
    Shift,
    /// 整数列だけをずらす (行の範囲は連続列の動ける幅で広げる)。
    IntShift,
}

/// 丸め・シフトの探索の状態: 今の点、行の活動量、破れた行の集合。
struct State {
    x: Vec<f64>,
    act: Vec<f64>,
    lo: Vec<f64>,
    up: Vec<f64>,
    /// 破れた行の一覧と、行ごとの一覧の中の位置 (なければ usize::MAX)。
    violated: Vec<usize>,
    pos: Vec<usize>,
}

impl State {
    fn set_row(&mut self, i: usize) {
        let v = viol(self.act[i], self.lo[i], self.up[i]) > 0.0;
        if v && self.pos[i] == usize::MAX {
            self.pos[i] = self.violated.len();
            self.violated.push(i);
        } else if !v && self.pos[i] != usize::MAX {
            let k = self.pos[i];
            let last = *self.violated.last().unwrap();
            self.violated.swap_remove(k);
            if last != i {
                self.pos[last] = k;
            }
            self.pos[i] = usize::MAX;
        }
    }
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// SCIP の `heur_rounding` の簡略版 (モジュールの説明を参照)。
    pub(super) fn lock_rounding(&mut self, x: &[f64]) -> bool {
        self.shift_heur(x, Mode::Round)
    }

    /// SCIP の `heur_shifting` の簡略版 (モジュールの説明を参照)。
    pub(super) fn shifting(&mut self, x: &[f64]) -> bool {
        self.shift_heur(x, Mode::Shift)
    }

    /// SCIP の `heur_intshifting` の簡略版 (モジュールの説明を参照)。連続列のない問題では [`Self::shifting`] と同じなので何もしない。
    pub(super) fn int_shifting(&mut self, x: &[f64]) -> bool {
        if self.p.is_int.iter().all(|&b| b) {
            return false;
        }
        self.shift_heur(x, Mode::IntShift)
    }

    fn shift_heur(&mut self, x0: &[f64], mode: Mode) -> bool {
        let p = self.p;
        let (n, m) = (p.n, p.m);
        let is_frac = |v: f64| (v - v.round()).abs() > FEASTOL;
        let frac0 = (0..n).filter(|&j| p.is_int[j] && is_frac(x0[j])).count();
        if frac0 == 0 || !p.is_int.iter().any(|&b| b) {
            return false;
        }
        // 行の範囲と活動量 (IntShift では整数列だけの活動量と、連続列の動ける幅で広げた範囲)
        let (mut lo, mut up) = (p.row_lo.clone(), p.row_up.clone());
        let mut act = vec![0.0f64; m];
        for (i, r) in p.rows.iter().enumerate() {
            let (mut cmin, mut cmax) = (0.0f64, 0.0f64);
            for &(j, a) in r {
                if mode == Mode::IntShift && !p.is_int[j] {
                    let (l, u) = (self.dom.global_lo[j], self.dom.global_up[j]);
                    let (a1, a2) = if a > 0.0 { (a * l, a * u) } else { (a * u, a * l) };
                    cmin += a1;
                    cmax += a2;
                } else {
                    act[i] += a * x0[j];
                }
            }
            if mode == Mode::IntShift {
                lo[i] -= cmax;
                up[i] -= cmin;
                if lo[i].is_nan() {
                    lo[i] = f64::NEG_INFINITY;
                }
                if up[i].is_nan() {
                    up[i] = f64::INFINITY;
                }
            }
        }
        // 今の定義域 (閉包から self を借りないよう写す)
        let (dlo, dup) = (self.dom.lo.clone(), self.dom.up.clone());
        let mut st = State { x: x0.to_vec(), act, lo, up, violated: Vec::new(), pos: vec![usize::MAX; m] };
        for i in 0..m {
            st.set_row(i);
        }
        // 動かせる列か (Round では小数の整数列だけ。連続列は IntShift では動かさない)
        let movable = |j: usize, x: &[f64]| -> bool {
            if dlo[j] >= dup[j] {
                return false;
            }
            match mode {
                Mode::Round => p.is_int[j] && is_frac(x[j]),
                Mode::Shift => true,
                Mode::IntShift => p.is_int[j],
            }
        };
        let col_in = |j: usize| mode != Mode::IntShift || p.is_int[j];
        // 列 j を d だけ動かしたときの (新たに破れる行の数 - 直る行の数, 破れの量の変化の合計)
        let eval = |st: &State, j: usize, d: f64, work: &mut u64| -> (i64, f64) {
            let mut cnt = 0i64;
            let mut amt = 0.0;
            *work += p.cols[j].len() as u64;
            for &(i, a) in &p.cols[j] {
                let (v0, v1) = (viol(st.act[i], st.lo[i], st.up[i]), viol(st.act[i] + a * d, st.lo[i], st.up[i]));
                if v0 == 0.0 && v1 > 0.0 {
                    cnt += 1;
                } else if v0 > 0.0 && v1 == 0.0 {
                    cnt -= 1;
                }
                amt += v1 - v0;
            }
            (cnt, amt)
        };
        let apply = |st: &mut State, j: usize, nv: f64| {
            let d = nv - st.x[j];
            st.x[j] = nv;
            for &(i, a) in &p.cols[j] {
                st.act[i] += a * d;
                st.set_row(i);
            }
        };
        // lock の多い順の小数の列 (破れた行がないときに丸める順)
        let mut order: Vec<usize> = (0..n).filter(|&j| p.is_int[j] && is_frac(x0[j])).collect();
        order.sort_by_key(|&j| std::cmp::Reverse(self.locks[j].0 + self.locks[j].1));
        let mut next = 0usize;
        // 前に動かした向き (+1/-1、0 はまだ)。Shift・IntShift では向きを戻す手を選ばない (行き来を防ぐ)
        let mut last_dir = vec![0i8; n];
        let nnz = p.nnz as u64;
        let work_cap = 10 * nnz + 100_000;
        let mut work = 0u64;
        let max_steps = 10 * (frac0 + st.violated.len()) + 100;
        // 破れた行・小数の列の数が最良から何手よくならないか
        let mut best_key = (usize::MAX, usize::MAX);
        let mut nonimprove = 0usize;
        let mut nfrac = frac0;
        for step in 0..max_steps {
            if step % 64 == 63 && self.time_up() {
                return false;
            }
            if work > work_cap {
                return false;
            }
            let key = (st.violated.len(), nfrac);
            if key < best_key {
                best_key = key;
                nonimprove = 0;
            } else {
                nonimprove += 1;
                if nonimprove > 50 {
                    return false;
                }
            }
            if !st.violated.is_empty() {
                // 破れた行を 1 本 (乱数で) 選び、その行を直す向きに動かす列のうち最良のもの
                let k = (self.rand() * st.violated.len() as f64) as usize % st.violated.len();
                let i = st.violated[k];
                let need_up = st.act[i] < st.lo[i];
                let need = if need_up { st.lo[i] - st.act[i] } else { st.act[i] - st.up[i] };
                let mut best: Option<((i64, f64, f64), usize, f64)> = None;
                work += p.rows[i].len() as u64;
                for &(j, a) in &p.rows[i] {
                    if !col_in(j) || !movable(j, &st.x) {
                        continue;
                    }
                    let dir = if (a > 0.0) == need_up { 1.0 } else { -1.0 };
                    if mode != Mode::Round && last_dir[j] as f64 == -dir {
                        continue;
                    }
                    let v = st.x[j];
                    let nv = if p.is_int[j] && is_frac(v) {
                        if dir > 0.0 { v.ceil() } else { v.floor() }
                    } else if p.is_int[j] {
                        let s = (need / a.abs() - 1e-9).ceil().max(1.0);
                        v + dir * s
                    } else {
                        v + dir * need / a.abs()
                    };
                    let nv = nv.clamp(dlo[j], dup[j]);
                    let d = nv - v;
                    if d.abs() < 1e-12 {
                        continue;
                    }
                    let (cnt, amt) = eval(&st, j, d, &mut work);
                    let sc = (cnt, amt, p.cost[j] * d);
                    if best.as_ref().is_none_or(|(b, _, _)| sc.0 < b.0 || (sc.0 == b.0 && (sc.1 < b.1 - 1e-12 || (sc.1 <= b.1 + 1e-12 && sc.2 < b.2)))) {
                        best = Some((sc, j, nv));
                    }
                }
                let Some((_, j, nv)) = best else { return false };
                if p.is_int[j] && is_frac(st.x[j]) {
                    nfrac -= 1;
                }
                last_dir[j] = if nv > st.x[j] { 1 } else { -1 };
                apply(&mut st, j, nv);
                continue;
            }
            // 破れた行がない: 次の小数の列を、新たに破れる行の少ない向き (同じなら目的値の良い向き) に丸める
            while next < order.len() && !is_frac(st.x[order[next]]) {
                next += 1;
            }
            if next >= order.len() {
                break;
            }
            let j = order[next];
            next += 1;
            let v = st.x[j];
            let (fl, ce) = (v.floor().max(dlo[j]), v.ceil().min(dup[j]));
            let (cf, _) = eval(&st, j, fl - v, &mut work);
            let (cc, _) = eval(&st, j, ce - v, &mut work);
            let nv = if cf < cc || (cf == cc && p.cost[j] * (fl - v) <= p.cost[j] * (ce - v)) { fl } else { ce };
            nfrac -= 1;
            last_dir[j] = if nv > v { 1 } else { -1 };
            apply(&mut st, j, nv);
        }
        if !st.violated.is_empty() || (0..n).any(|j| p.is_int[j] && is_frac(st.x[j])) {
            return false;
        }
        if mode == Mode::IntShift {
            return self.fixed_int_lp(&st.x);
        }
        // 打ち切り値より悪い点は試さない
        if p.objective(&st.x) >= self.prune_limit() {
            return false;
        }
        self.try_incumbent(st.x)
    }

    /// SCIP の `heur_twoopt` の簡略版 (モジュールの説明を参照)。実行可能解 `x` を書き換える。動かしたら真。
    /// 連続列は動かさないので、結果も実行可能。
    pub(super) fn two_opt(&mut self, x: &mut [f64]) -> bool {
        let p = self.p;
        if self.params.submip || !p.is_int.iter().any(|&b| b) {
            return false;
        }
        let (lo, up) = (&self.dom.global_lo, &self.dom.global_up);
        let mut act: Vec<f64> = p.rows.iter().map(|r| r.iter().map(|&(j, a)| a * x[j]).sum()).collect();
        let mut masters: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j] && p.cost[j] != 0.0 && lo[j] < up[j]).collect();
        masters.sort_by(|&a, &b| p.cost[b].abs().total_cmp(&p.cost[a].abs()));
        let nnz = p.nnz as u64;
        let work_cap = 20 * nnz + 1_000_000;
        let mut work = 0u64;
        let mut delta = vec![0.0f64; p.m];
        let mut touched: Vec<usize> = Vec::new();
        let mut seen = vec![false; p.n];
        let max_cands = tunable!("ENOMOTO_T_MIP_TWOOPT_CANDS", 256, usize);
        let mut improved = false;
        'pass: for _pass in 0..3 {
            let mut moved = false;
            for &j in &masters {
                if work > work_cap {
                    break 'pass;
                }
                let dj = if p.cost[j] > 0.0 { -1.0 } else { 1.0 };
                if (dj > 0.0 && x[j] + 1.0 > up[j] + 1e-9) || (dj < 0.0 && x[j] - 1.0 < lo[j] - 1e-9) {
                    continue;
                }
                // j を 1 動かすと破れる行の、逆に効く向きに動かせる整数列 k
                let mut cands: Vec<(usize, f64)> = Vec::new();
                work += p.cols[j].len() as u64;
                for &(i, a) in &p.cols[j] {
                    let da = a * dj;
                    let blocks = if da > 0.0 { act[i] + da > p.row_up[i] + 1e-9 } else { act[i] + da < p.row_lo[i] - 1e-9 };
                    if !blocks {
                        continue;
                    }
                    work += p.rows[i].len() as u64;
                    for &(k, b) in &p.rows[i] {
                        if k == j || seen[k] || !p.is_int[k] || lo[k] >= up[k] {
                            continue;
                        }
                        let dk = if (b > 0.0) == (da > 0.0) { -1.0 } else { 1.0 };
                        // 目的値の 1 単位あたりの改善 (j で |c_j| 良くなり、k で c_k dk 悪くなる)
                        if p.cost[j].abs() - p.cost[k] * dk <= 1e-9 * p.cost[j].abs().max(1.0) {
                            continue;
                        }
                        seen[k] = true;
                        cands.push((k, dk));
                        if cands.len() >= max_cands {
                            break;
                        }
                    }
                    if cands.len() >= max_cands {
                        break;
                    }
                }
                for &(k, _) in &cands {
                    seen[k] = false;
                }
                let mut best: Option<(f64, usize, f64, f64)> = None; // (改善, k, dk, 幅)
                for &(k, dk) in &cands {
                    let mut t = if dj > 0.0 { up[j] - x[j] } else { x[j] - lo[j] };
                    t = t.min(if dk > 0.0 { up[k] - x[k] } else { x[k] - lo[k] });
                    for &(i, a) in &p.cols[j] {
                        if delta[i] == 0.0 {
                            touched.push(i);
                        }
                        delta[i] += a * dj;
                    }
                    for &(i, b) in &p.cols[k] {
                        if delta[i] == 0.0 {
                            touched.push(i);
                        }
                        delta[i] += b * dk;
                    }
                    work += (p.cols[j].len() + p.cols[k].len()) as u64;
                    for &i in &touched {
                        let d = delta[i];
                        if d > 1e-12 {
                            t = t.min((p.row_up[i] - act[i] + 1e-9) / d);
                        } else if d < -1e-12 {
                            t = t.min((act[i] - p.row_lo[i] + 1e-9) / -d);
                        }
                        delta[i] = 0.0;
                    }
                    touched.clear();
                    if !t.is_finite() {
                        continue;
                    }
                    let t = t.floor();
                    if t < 1.0 {
                        continue;
                    }
                    let gain = (p.cost[j].abs() - p.cost[k] * dk) * t;
                    if best.is_none_or(|b| gain > b.0) {
                        best = Some((gain, k, dk, t));
                    }
                }
                if let Some((_, k, dk, t)) = best {
                    x[j] += dj * t;
                    x[k] += dk * t;
                    for &(i, a) in &p.cols[j] {
                        act[i] += a * dj * t;
                    }
                    for &(i, b) in &p.cols[k] {
                        act[i] += b * dk * t;
                    }
                    moved = true;
                }
            }
            if !moved {
                break;
            }
            improved = true;
        }
        improved
    }
}
