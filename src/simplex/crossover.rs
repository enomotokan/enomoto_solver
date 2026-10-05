//! 内点法 + クロスオーバーによる LP 求解 (`RootSolver::IpmCrossover`)。
//!
//! 前処理後の標準形 [`StdForm`] (`A z = b, lb <= z <= ub`、各行にスラック列) を
//! 箱型制約付きの PIQP 型内点法 ([`crate::interior_point::boxed`]) で解き、得られた
//! (頂点とは限らない) 最適解から最適基底を作る。クロスオーバーは
//!
//!   T. Liu and H. Lu, "A New Crossover Algorithm for LP Inspired by the Spiral Dynamic
//!   of PDHG", arXiv:2409.14715 (INFORMS J. Comput. 2025)
//!
//! の方式に従う (著者の参照実装 github.com/MIT-Lu-Lab/crossover の構成と同じ段):
//!
//! 1. **主の押し出し (primal push)**: 境界に近い変数 (`x_j - l_j <= max(γ s_j, tol)` など、
//!    `s = c - A^T y`) を非基底として境界に固定し、残り (基底候補 `B`) について、乱数で摂動した
//!    費用 `c̃` の `null(A_B)` への射影 `v = -(c̃_B - A_B^T ŷ)` (PDHG のらせんの「前進」成分) の
//!    向きに比率テストで進め、境界に当たった変数を非基底に移す。`|B| <= m` で `|B|` が
//!    減らなくなるまで繰り返す。
//! 2. **双対の押し出し (dual push)**: 双対活性集合 `D` (基底 ∪ 被約費用 ≈ 0 の列) について、
//!    乱数で摂動した右辺 `b̃` の `A_D` の値域の直交補空間への射影 `v_y = b̃ - A_D x̂` の向きに
//!    `y` を動かし (`A_D^T v_y = 0` なので `D` の被約費用は動かない)、被約費用が 0 に達した列を
//!    `D` に加える。`|D|` が増えなくなるまで繰り返す (`A_D` が行フルランクになれば `v_y = 0`)。
//! 3. **一次独立性の検査**: `B` を優先し、次に `D \ B`、最後にスラック列の順で、一次独立な
//!    `m` 列を選んで基底にする (左から順の疎 LU、Gilbert–Peierls)。
//! 4. **仕上げ**: その基底から `polish_with_true_bounds` (有界双対単体法 + 主単体法への
//!    引き継ぎ) を走らせる。基底が既に最適ならそのまま返る。
//!
//! ## 計算の効率化
//!
//! 参照実装は射影 (最小二乗) を毎回 QR/反復法で一から解くが、ここでは次のようにする:
//!
//! - 射影はどちらも拡大系 `[diag(t) A^T; A diag(m)]` を解いて得る (主: `t_j = 1` (基底候補)
//!   / `H` (非基底)、`m = -ε`。双対: `t_j = ε` (`D`) / `H` (`D` 外)、`m = -1`)。この非零
//!   パターンは全列の `A` で決まり、AMD 順序・記号分解は 1 回だけ (`AugKkt`)。
//! - 1 回の押し出しで `B`/`D` は 1 列ずつしか変わらないので、毎回数値分解し直さず、
//!   分解後の変化を縁取り (bordering) で正確に扱う: 主では非基底に移った列に制約 `v_j = 0`
//!   (縁 `e_j`)、双対では `D` に加わった列を新しい変数 (縁 `[0; a_j]`) として加え、
//!   `k x k` の Schur 補行列だけを密に解く。1 歩の手間は KKT の三角求解 2 回 + `O(k·dim)` で、
//!   `k` が上限 (メモリと手間から決める) に達したら対角を更新して数値分解し直す。
//! - 比率テストで同時に境界 (双対では 0) に達した列は許容誤差でまとめて移す。
//! - 基底の選択は優先順の左から順の疎 LU で、スラック列 (単位列) は消去なしで受理できる。

use super::slope_intercept_dual::{polish_with_true_bounds, refactorize};
use super::{perturb_random, NbStatus, SimplexResult, StdForm};
use crate::interior_point::boxed::solve_box_lp;
use crate::interior_point::kkt::{AugKkt, FaerCsr};
use crate::sparse::{csr_from_rows, sparse_dot_dense};
use crate::types::Status;
use std::time::Instant;

/// 列の主の状態 (押し出し中)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PStat {
    Basic,
    Lower,
    Upper,
}

/// クロスオーバーのパラメータ (参照実装の既定値に合わせる)。
mod prm {
    /// 境界・0 の判定の絶対許容誤差 (`tol_bound`)。
    pub const TOL_BOUND: f64 = 1e-8;
    /// 非基底の判定で、主変数の境界までの距離と被約費用を比べる比 (`gamma`)。
    pub const GAMMA: f64 = 1.0;
    /// 0 とみなす値 (`epsilon_zero`)。射影の向きの成分・比率テストの分母に使う。
    pub const EPS_ZERO: f64 = 1e-10;
    /// 双対の比率テストで、`D` の列での射影誤差の何倍以下の `|a_j^T v_y|` を 0 とみなすか。
    pub const NOISE_FACTOR: f64 = 10.0;
    /// 1 歩で `A x = b`・`A_D^T y = c_D` からずれてよい量。超えたらその場で補正する。
    pub const DRIFT_TOL: f64 = 1e-9;
    /// 押し出しの向きが零空間に入っているとみなす相対残差 (`‖A v‖ ≤ NULL_REL ‖v‖`)。
    pub const NULL_REL: f64 = 1e-6;
    /// 双対の押し出しの向きの相対残差の上限 (`‖A_D^T v_y‖ ≤ NULL_REL_DUAL |v_y|`)。
    pub const NULL_REL_DUAL: f64 = 1e-3;
    /// 主の射影の向き `v` の成分を 0 とみなす閾値 (摂動費用は大きさ O(1))。
    pub const V_REL_ZERO: f64 = 1e-7;
    /// 主の射影系の下段対角の大きさ (`-ε`)。
    pub const EPS_PRIMAL: f64 = 1e-12;
    /// 双対の射影系で `D` の列に置く上段対角 (リッジ正則化)。
    pub const EPS_DUAL: f64 = 1e-12;
    /// 射影から外す列に置く上段対角 (実質的に列を消す)。
    pub const BIG: f64 = 1e12;
    /// 縁取りの列数の上限 (これを超えたら数値分解し直す)。
    pub const MAX_BORDER: usize = 64;
    /// 縁取りの `K^{-1} U` に使ってよいメモリ (バイト)。
    pub const BORDER_MEM: usize = 512 << 20;
    /// 内点法が収束しなかったとき、最良の反復点の相対残差 (許容値 1e-8 の倍率) がこれ以下なら
    /// クロスオーバーに進む (1e4 倍 = 相対 1e-4 程度)。
    pub const IPM_ACCEPT_REL: f64 = 1e4;
    /// 基底の選択で列を受理する残差の相対閾値。
    pub const LI_TOL: f64 = 1e-9;
}

/// 拡大系 `K = [diag(t) A^T; A diag(m)]` の分解と、分解後に足した縁取り
/// `[K U; U^T C]` の Schur 補行列による正確な求解。
///
/// 押し出しの 1 回の数値分解の間 (エポック) は右辺 (乱数で摂動した費用・右辺) を固定し、
/// `K^{-1} r` を [`Self::set_base`] で一度だけ計算する。各歩の向きは
/// `w = K^{-1} r - (K^{-1} U) S^{-1} (-(U^T K^{-1} r))` で、新しい縁 1 本の求解と `O(k·dim)` だけで済む。
/// 縁の手間の累計が数値分解 1 回の時間を超えたら ([`Self::should_refactor`]) 分解し直す。
struct BorderedKkt {
    kkt: AugKkt,
    dim: usize,
    /// 縁の列 (拡大系の添字での疎ベクトル)。
    u: Vec<Vec<(usize, f64)>>,
    /// `K^{-1} u_i` (密)。
    kinv_u: Vec<Vec<f64>>,
    /// Schur 補行列 `S = C - U^T K^{-1} U` (行優先、`k x k`、縁を足すたびに広げる)。
    schur: Vec<Vec<f64>>,
    /// `K^{-1} r` (このエポックの右辺)。
    base: Vec<f64>,
    /// 縁の上限 (メモリ)。
    max_border: usize,
    /// 直近の数値分解にかかった時間と、それ以降の縁の処理の累計時間 (秒)。
    factor_secs: f64,
    border_secs: f64,
    /// 数値分解の回数と三角求解の回数 (統計)。
    n_factor: usize,
    n_solve: usize,
}

impl BorderedKkt {
    fn new(a: &FaerCsr) -> Self {
        let kkt = AugKkt::new(a);
        let dim = kkt.dim();
        let max_border = (prm::BORDER_MEM / (8 * dim.max(1))).clamp(4, prm::MAX_BORDER);
        BorderedKkt {
            kkt,
            dim,
            u: Vec::new(),
            kinv_u: Vec::new(),
            schur: Vec::new(),
            base: vec![0.0; dim],
            max_border,
            factor_secs: 0.0,
            border_secs: 0.0,
            n_factor: 0,
            n_solve: 0,
        }
    }

    /// 対角を与えて数値分解し、右辺 `r` で新しいエポックを始める。
    fn factor(&mut self, top: &[f64], mid: &[f64], r: &[f64]) {
        let t = Instant::now();
        self.kkt.factor(top, mid);
        self.u.clear();
        self.kinv_u.clear();
        self.schur.clear();
        self.base.copy_from_slice(r);
        self.kkt.solve_in_place(&mut self.base);
        self.n_factor += 1;
        self.n_solve += 1;
        self.factor_secs = t.elapsed().as_secs_f64();
        self.border_secs = 0.0;
    }

    /// 縁を `extra` 本足す前に分解し直すべきか (メモリの上限か、縁の手間が分解を上回った)。
    fn should_refactor(&self, extra: usize) -> bool {
        self.u.len() + extra > self.max_border || self.border_secs > self.factor_secs
    }

    /// 縁 `u` (右下 `c`) を加える。
    fn add_border(&mut self, u: Vec<(usize, f64)>, c: f64) {
        let t = Instant::now();
        let mut w = vec![0.0; self.dim];
        for &(i, v) in &u {
            w[i] = v;
        }
        self.kkt.solve_in_place(&mut w);
        self.n_solve += 1;
        let k = self.u.len();
        let mut row = Vec::with_capacity(k + 1);
        for i in 0..k {
            let sij = -sparse_dot_dense(&self.u[i], &w);
            self.schur[i].push(sij);
            row.push(sij);
        }
        row.push(c - sparse_dot_dense(&u, &w));
        self.schur.push(row);
        self.u.push(u);
        self.kinv_u.push(w);
        self.border_secs += t.elapsed().as_secs_f64();
    }

    /// 縁取り系 `[K U; U^T C] [w; λ] = [r; rb]` の `w` を `r` に上書きする (任意の右辺)。
    fn solve_general(&mut self, r: &mut [f64], rb: &[f64]) {
        let t = Instant::now();
        self.kkt.solve_in_place(r);
        self.n_solve += 1;
        let k = self.u.len();
        if k > 0 {
            // S λ = rb - U^T K^{-1} r、w = K^{-1} r - (K^{-1} U) λ
            let g: Vec<f64> = (0..k).map(|i| rb[i] - sparse_dot_dense(&self.u[i], r)).collect();
            let mut sflat: Vec<f64> = self.schur.iter().flatten().copied().collect();
            let lam = dense_solve(k, &mut sflat, g);
            for j in 0..k {
                let l = lam[j];
                if l != 0.0 {
                    for (o, wi) in r.iter_mut().zip(&self.kinv_u[j]) {
                        *o -= l * wi;
                    }
                }
            }
        }
        self.border_secs += t.elapsed().as_secs_f64();
    }

    /// 現在の縁取り系での `[K U; U^T C]^{-1} [r; 0]` の上段 (`r` はこのエポックの右辺) を `out` に書く。
    fn solve_base(&mut self, out: &mut [f64]) {
        let t = Instant::now();
        out.copy_from_slice(&self.base);
        let k = self.u.len();
        if k > 0 {
            // S λ = -U^T K^{-1} r、w = K^{-1} r - (K^{-1} U) λ
            let g: Vec<f64> = (0..k).map(|i| -sparse_dot_dense(&self.u[i], &self.base)).collect();
            let mut sflat: Vec<f64> = self.schur.iter().flatten().copied().collect();
            let lam = dense_solve(k, &mut sflat, g);
            for j in 0..k {
                let l = lam[j];
                if l != 0.0 {
                    for (o, wi) in out.iter_mut().zip(&self.kinv_u[j]) {
                        *o -= l * wi;
                    }
                }
            }
        }
        self.border_secs += t.elapsed().as_secs_f64();
    }
}

/// 部分ピボット付きガウス消去で `S x = rhs` を解く (`k` は小さい)。特異な列は 0 にする。
fn dense_solve(k: usize, s: &mut [f64], mut rhs: Vec<f64>) -> Vec<f64> {
    let mut perm: Vec<usize> = (0..k).collect();
    for col in 0..k {
        let (mut best, mut bv) = (col, s[perm[col] * k + col].abs());
        for r in col + 1..k {
            let v = s[perm[r] * k + col].abs();
            if v > bv {
                best = r;
                bv = v;
            }
        }
        perm.swap(col, best);
        let pr = perm[col];
        let piv = s[pr * k + col];
        if piv.abs() < 1e-300 {
            continue;
        }
        for r in col + 1..k {
            let rr = perm[r];
            let f = s[rr * k + col] / piv;
            if f != 0.0 {
                for c in col..k {
                    s[rr * k + c] -= f * s[pr * k + c];
                }
                rhs[rr] -= f * rhs[pr];
            }
        }
    }
    let mut x = vec![0.0; k];
    for col in (0..k).rev() {
        let pr = perm[col];
        let piv = s[pr * k + col];
        if piv.abs() < 1e-300 {
            continue;
        }
        let mut v = rhs[pr];
        for c in col + 1..k {
            v -= s[pr * k + c] * x[c];
        }
        x[col] = v / piv;
    }
    x
}

/// クロスオーバーの統計 (`ENOMOTO_DEBUG_CROSSOVER`)。
#[derive(Default, Debug)]
struct Stats {
    ipm_iters: usize,
    basic_after_detect: usize,
    primal_steps: usize,
    basic_after_push: usize,
    dual_active_start: usize,
    dual_steps: usize,
    primal_corrections: usize,
    dual_corrections: usize,
    dual_active_end: usize,
    li_from_b: usize,
    li_from_d: usize,
    li_slack: usize,
    factors: usize,
    solves: usize,
}

/// `‖A x - b‖∞` (診断用)。
fn primal_resid(std: &StdForm, x: &[f64]) -> f64 {
    let mut r = std.b.clone();
    for j in 0..std.n_total {
        for &(i, a) in std.cols.col(j) {
            r[i] -= a * x[j];
        }
    }
    r.iter().fold(0.0f64, |m, v| m.max(v.abs()))
}

/// 列 `j` の `A` の列 (`std.cols`)。
#[inline]
fn col(std: &StdForm, j: usize) -> &[(usize, f64)] {
    std.cols.col(j)
}

/// `out = A^T y` (全列)。
fn at_y(std: &StdForm, y: &[f64], out: &mut [f64]) {
    use rayon::prelude::*;
    out.par_iter_mut().enumerate().for_each(|(j, o)| *o = sparse_dot_dense(std.cols.col(j), y));
}

/// 乱数 (列番号と回数のハッシュ、[0, 1))。
#[inline]
fn rnd(j: usize, round: usize) -> f64 {
    perturb_random(j.wrapping_mul(0x9E37_79B9).wrapping_add(round.wrapping_mul(0x85EB_CA6B)))
}

/// 内点法 + クロスオーバーで `std` を解く。内点法が最適に収束しなければ `None`
/// (呼び出し側が傾き・切片二段解法で解き直す)。
pub(super) fn solve_ipm_crossover(std: &StdForm) -> Option<SimplexResult> {
    let debug = env_str!("ENOMOTO_DEBUG_CROSSOVER").is_some();
    let t0 = Instant::now();
    let n = std.n_total;
    let m = std.n_rows;
    let mut st = Stats::default();

    // ---- 1. 内点法 (固定列 lb == ub を除いた問題) ----
    let free_cols: Vec<usize> = (0..n).filter(|&j| std.lb[j] < std.ub[j]).collect();
    let mut new_idx = vec![usize::MAX; n];
    for (k, &j) in free_cols.iter().enumerate() {
        new_idx[j] = k;
    }
    let mut rows_j: Vec<Vec<(usize, f64)>> = Vec::with_capacity(m);
    let mut b_j = std.b.clone();
    for i in 0..m {
        let mut r = Vec::with_capacity(std.rows.row(i).len());
        for &(j, v) in std.rows.row(i) {
            if new_idx[j] != usize::MAX {
                r.push((new_idx[j], v));
            } else {
                b_j[i] -= v * std.lb[j];
            }
        }
        rows_j.push(r);
    }
    let a_j = csr_from_rows(&rows_j, free_cols.len());
    drop(rows_j);
    let c_j: Vec<f64> = free_cols.iter().map(|&j| std.c[j]).collect();
    let l_j: Vec<f64> = free_cols.iter().map(|&j| std.lb[j]).collect();
    let u_j: Vec<f64> = free_cols.iter().map(|&j| std.ub[j]).collect();
    let max_iters = tunable!("ENOMOTO_T_IPM_MAX_ITERS", 200usize, usize);
    let ipm = solve_box_lp(&a_j, &b_j, &c_j, &l_j, &u_j, max_iters);
    drop(a_j);
    st.ipm_iters = ipm.iters;
    crate::phase_timing::mark("ipm_end");
    if debug {
        eprintln!("CROSSOVER ipm status={:?} iters={} rel_res={:?} t={:.3}s", ipm.status, ipm.iters, ipm.rel_res, t0.elapsed().as_secs_f64());
    }
    // 収束しなかった場合も、最良の反復点が相対残差 `IPM_ACCEPT_REL` (各許容値の倍率) 以内なら
    // クロスオーバーに進む (最終段の単体法が残りの誤差を直す)。それ以外は呼び出し側で解き直す。
    let worst = ipm.rel_res.0.max(ipm.rel_res.1).max(ipm.rel_res.2);
    if ipm.status != Status::Optimal && !(matches!(ipm.status, Status::NotSolved) && worst <= prm::IPM_ACCEPT_REL) {
        return None;
    }
    let mut x = std.lb.clone(); // 固定列は lb
    for (k, &j) in free_cols.iter().enumerate() {
        x[j] = ipm.x[k].clamp(std.lb[j], std.ub[j]);
    }
    let mut y = ipm.y;
    drop(ipm.x);
    drop(ipm.rc);
    let mut s = vec![0.0; n];
    {
        let mut aty = vec![0.0; n];
        at_y(std, &y, &mut aty);
        for j in 0..n {
            s[j] = std.c[j] - aty[j];
        }
    }

    // ---- 拡大系 (全列の A) ----
    let a_full: FaerCsr = std.rows.to_faer();
    let mut bk = BorderedKkt::new(&a_full);
    let dim = n + m;
    let mut rhs = vec![0.0; dim];

    // ---- 2. 主の押し出し ----
    let mut ps = vec![PStat::Basic; n];
    let tol = prm::TOL_BOUND;
    // 非基底の検出。新たに非基底になった列を返す。
    let detect = |ps: &mut [PStat], x: &mut [f64], s: &[f64], newly: &mut Vec<usize>| {
        for j in 0..n {
            if ps[j] != PStat::Basic {
                continue;
            }
            let (l, u) = (std.lb[j], std.ub[j]);
            let stat = if l == u {
                Some(if s[j] >= 0.0 { PStat::Lower } else { PStat::Upper })
            } else {
                let at_l = l.is_finite() && x[j] - l <= (prm::GAMMA * s[j]).max(tol);
                let at_u = u.is_finite() && u - x[j] <= (-prm::GAMMA * s[j]).max(tol);
                match (at_l, at_u) {
                    (true, true) => Some(if x[j] - l <= u - x[j] { PStat::Lower } else { PStat::Upper }),
                    (true, false) => Some(PStat::Lower),
                    (false, true) => Some(PStat::Upper),
                    _ => None,
                }
            };
            if let Some(stv) = stat {
                ps[j] = stv;
                x[j] = if stv == PStat::Lower { l } else { u };
                newly.push(j);
            }
        }
    };
    if debug {
        eprintln!("CROSSOVER resid after ipm={:.3e}", primal_resid(std, &x));
    }
    let mut newly = Vec::new();
    detect(&mut ps, &mut x, &s, &mut newly);
    if debug {
        eprintln!("CROSSOVER resid after detect={:.3e}", primal_resid(std, &x));
    }
    let mut n_basic = ps.iter().filter(|&&p| p == PStat::Basic).count();
    st.basic_after_detect = n_basic;
    if debug {
        eprintln!("CROSSOVER m={m} n={n} basic_after_detect={n_basic} t={:.3}s", t0.elapsed().as_secs_f64());
    }
    let cnorm = std.c.iter().fold(0.0f64, |a, v| a.max(v.abs()));
    let mut top = vec![0.0; n];
    let mid_p = vec![-tunable!("ENOMOTO_T_XO_EPS_PRIMAL", prm::EPS_PRIMAL, f64); m];
    let mut need_factor = true;
    let mut n_basic_last = usize::MAX;
    let mut round = 0usize;
    let mut epoch = 0usize;
    let mut v = vec![0.0; n];
    let max_primal = 10 * n + 10;
    let correct = tunable!("ENOMOTO_T_XO_CORRECT", 1u8, u8) != 0;
    // 主の残差の補正: 基底候補の列だけで `A_B δ = b - A x` の最小ノルム解を足す (射影の正則化による
    // `A x = b` からのずれが歩を重ねて溜まり、最終基底で B^{-1} に増幅されるのを防ぐ)。
    // 縁 (非基底になった列の `v_j = 0`) を含む今の分解で解く。
    let primal_correct = |bk: &mut BorderedKkt, x: &mut [f64], ps: &[PStat], buf: &mut [f64]| {
        buf[..n].fill(0.0);
        buf[n..].copy_from_slice(&std.b);
        for j in 0..n {
            if x[j] != 0.0 {
                for &(i, a) in col(std, j) {
                    buf[n + i] -= a * x[j];
                }
            }
        }
        // [D A^T; A -εI][v; y'] = [0; r] で A v ≈ r (D = 1 の列のみ動く)
        let zeros = vec![0.0; bk.u.len()];
        bk.solve_general(buf, &zeros);
        for j in 0..n {
            if ps[j] == PStat::Basic && buf[j] != 0.0 {
                x[j] = (x[j] + buf[j]).clamp(std.lb[j], std.ub[j]);
            }
        }
    };
    let mut corr_buf = vec![0.0; dim];
    loop {
        if n_basic == 0 || (round >= 2 && n_basic <= m && n_basic >= n_basic_last) || round > max_primal {
            break;
        }
        n_basic_last = n_basic;
        round += 1;
        // 係数行列: 新たに非基底になった列を縁 (v_j = 0) に足すか、溜まっていれば分解し直す
        // (分解のたびに摂動費用 c̃ を引き直す。エポックの中では同じ c̃ を射影し続ける)。
        if need_factor || bk.should_refactor(newly.len()) {
            epoch += 1;
            for j in 0..n {
                top[j] = if ps[j] == PStat::Basic { 1.0 } else { prm::BIG };
                rhs[j] = if ps[j] == PStat::Basic {
                    let mut r = rnd(j, epoch);
                    if !std.lb[j].is_finite() && std.ub[j].is_finite() {
                        r = -r;
                    }
                    -(r + std.c[j] / (cnorm + 1.0))
                } else {
                    0.0
                };
            }
            rhs[n..].fill(0.0);
            bk.factor(&top, &mid_p, &rhs);
            need_factor = false;
            if correct {
                primal_correct(&mut bk, &mut x, &ps, &mut corr_buf);
            }
        } else {
            for &j in &newly {
                bk.add_border(vec![(j, 1.0)], 0.0);
            }
        }
        newly.clear();
        bk.solve_base(&mut rhs);
        // 射影の正則化 (ε) による雑音を向きとみなさないよう、摂動費用 (大きさ O(1)) に対する
        // 相対閾値 `V_REL_ZERO` 以下の成分は 0 にする (A_B が列フルランクなら v ≈ 0 で止まる)。
        let mut vnorm = 0.0f64;
        for j in 0..n {
            v[j] = if ps[j] == PStat::Basic && rhs[j].abs() > prm::V_REL_ZERO { rhs[j] } else { 0.0 };
            vnorm = vnorm.max(v[j].abs());
        }
        if vnorm <= prm::V_REL_ZERO {
            continue;
        }
        let mut avn = 0.0f64;
        // 向きが本当に null(A_B) に入っているか: `‖A v‖ ≤ NULL_REL ‖v‖`。A_B が列フルランクになった後の
        // 射影は丸め誤差だけで (blend で |v| ≈ 4e-7、‖A v‖ ≈ |v|)、それを向きとみなすと比率テストが
        // θ ≈ 1e7 で進んで `A x = b` を壊す。満たさなければ向きは無い (これ以上固定できない)。
        {
            let mut av = vec![0.0f64; m];
            for j in 0..n {
                if v[j] != 0.0 {
                    for &(i, a) in col(std, j) {
                        av[i] += a * v[j];
                    }
                }
            }
            avn = av.iter().fold(0.0f64, |mm, t| mm.max(t.abs()));
            if avn > prm::NULL_REL * vnorm {
                if debug {
                    eprintln!("CROSSOVER primal push: |Av|={avn:.2e} > {:.0e}|v| (|v|={vnorm:.2e}); no null-space direction", prm::NULL_REL);
                }
                n_basic_last = n_basic;
                if n_basic <= m {
                    break;
                }
                need_factor = true;
                continue;
            }
        }
        let ratio = |v: &[f64], x: &[f64], sign: f64| -> (f64, usize) {
            let mut best = (f64::INFINITY, usize::MAX);
            for j in 0..n {
                let vj = sign * v[j];
                if vj < -prm::EPS_ZERO && std.lb[j].is_finite() {
                    let t = (std.lb[j] - x[j]) / vj;
                    if t < best.0 {
                        best = (t, j);
                    }
                } else if vj > prm::EPS_ZERO && std.ub[j].is_finite() {
                    let t = (std.ub[j] - x[j]) / vj;
                    if t < best.0 {
                        best = (t, j);
                    }
                }
            }
            best
        };
        let mut sign = 1.0;
        let (mut theta, mut jb) = ratio(&v, &x, 1.0);
        if !theta.is_finite() {
            sign = -1.0;
            (theta, jb) = ratio(&v, &x, -1.0);
        }
        if !theta.is_finite() {
            break; // 両向きとも有界でない (自由列だけ): これ以上固定できない
        }
        let theta = theta.max(0.0);
        for j in 0..n {
            if v[j] != 0.0 {
                x[j] += theta * sign * v[j];
            }
        }
        // ブロックした列は境界に置く
        let vj = sign * v[jb];
        if vj < 0.0 {
            x[jb] = std.lb[jb];
            ps[jb] = PStat::Lower;
        } else {
            x[jb] = std.ub[jb];
            ps[jb] = PStat::Upper;
        }
        newly.push(jb);
        if correct && theta * avn > tunable!("ENOMOTO_T_XO_DRIFT_TOL", prm::DRIFT_TOL, f64) {
            // この歩の `A x = b` からのずれを補正する (縁を足してから)。
            for &j in &newly {
                bk.add_border(vec![(j, 1.0)], 0.0);
            }
            newly.clear();
            primal_correct(&mut bk, &mut x, &ps, &mut corr_buf);
            st.primal_corrections += 1;
        }
        let r_move = if debug { primal_resid(std, &x) } else { 0.0 };
        let nbefore = newly.len();
        detect(&mut ps, &mut x, &s, &mut newly);
        if debug {
            let av = {
                let mut r = vec![0.0; m];
                for j in 0..n {
                    for &(i, a) in col(std, j) {
                        r[i] += a * v[j];
                    }
                }
                r.iter().fold(0.0f64, |mm, t| mm.max(t.abs()))
            };
            eprintln!("CROSSOVER pstep theta={theta:.3e} |v|={vnorm:.3e} |Av|={av:.3e} resid_move={r_move:.3e} resid_detect={:.3e} snapped={}", primal_resid(std, &x), newly.len() - nbefore);
        }
        n_basic = ps.iter().filter(|&&p| p == PStat::Basic).count();
        st.primal_steps += 1;
    }
    if correct && !need_factor {
        for &j in &newly {
            bk.add_border(vec![(j, 1.0)], 0.0);
        }
        newly.clear();
        primal_correct(&mut bk, &mut x, &ps, &mut corr_buf);
    }
    st.basic_after_push = n_basic;
    if debug {
        eprintln!("CROSSOVER resid after primal push={:.3e}", primal_resid(std, &x));
    }
    crate::phase_timing::mark("primal_push_end");
    if debug {
        eprintln!("CROSSOVER primal_push steps={} basic={} t={:.3}s", st.primal_steps, n_basic, t0.elapsed().as_secs_f64());
    }

    // ---- 3. 双対の押し出し ----
    // 固定列の非基底の向きは被約費用の符号で決める (値は変わらない)。
    let mut active = vec![false; n];
    let update_active = |active: &mut [bool], ps: &mut [PStat], s: &[f64], added: &mut Vec<usize>| {
        for j in 0..n {
            if ps[j] != PStat::Basic && std.lb[j] == std.ub[j] {
                ps[j] = if s[j] >= 0.0 { PStat::Lower } else { PStat::Upper };
            }
            if active[j] {
                continue;
            }
            let (hl, hu) = (std.lb[j].is_finite(), std.ub[j].is_finite());
            // 参照実装は下限だけの列を `s_j <= tol` で活性にする (負の s も入る) が、負に外れた列を D に
            // 入れると `A_D^T y = c_D` が両立しなくなり押し出しの不変条件が崩れる (degen3) ので、
            // 向きによらず `|s_j| <= tol` の列だけを加える (外れた列は仕上げの単体法が直す)。
            let _ = (hl, hu);
            let act = ps[j] == PStat::Basic || s[j].abs() <= tol;
            if act {
                active[j] = true;
                added.push(j);
            }
        }
    };
    let mut added = Vec::new();
    update_active(&mut active, &mut ps, &s, &mut added);
    let mut n_active = active.iter().filter(|&&a| a).count();
    st.dual_active_start = n_active;
    let bnorm = std.b.iter().fold(0.0f64, |a, v| a.max(v.abs()));
    let mid_d = vec![-1.0; m];
    let eps_dual = tunable!("ENOMOTO_T_XO_EPS_DUAL", prm::EPS_DUAL, f64);
    let mut need_factor = true;
    let mut n_active_last = 0usize;
    let mut round_d = 0usize;
    let mut w = vec![0.0; n];
    let mut vy = vec![0.0; m];
    let max_dual = 10 * m + 10;
    // 双対の補正: `A_D^T δ = s_D` の最小ノルム解 (`δ ∈ range(A_D)`) を `y` に足して `D` の被約費用を
    // 0 に戻す (射影の誤差で `y` が `A_D^T y = c_D` からずれていくのを防ぐ)。縁の列 (後から `D` に
    // 加えた列) の右辺も `s_j`。最後に `s = c - A^T y` を計算し直す。
    let dual_correct = |bk: &mut BorderedKkt, y: &mut [f64], s: &mut [f64], active: &[bool], buf: &mut [f64], border_cols: &[usize]| {
        // [diag(h) A^T; A -I][x; q] = [f; 0] → A_D^T q ≈ f、q = A x ∈ range(A_D)
        for j in 0..n {
            buf[j] = if active[j] && !border_cols.contains(&j) { s[j] } else { 0.0 };
        }
        buf[n..].fill(0.0);
        let rb: Vec<f64> = border_cols.iter().map(|&j| s[j]).collect();
        bk.solve_general(buf, &rb);
        for i in 0..m {
            y[i] += buf[n + i];
        }
        let mut aty = vec![0.0; n];
        at_y(std, y, &mut aty);
        for j in 0..n {
            s[j] = std.c[j] - aty[j];
        }
    };
    let mut border_cols: Vec<usize> = Vec::new();
    loop {
        if (round_d >= 2 && n_active <= n_active_last) || round_d > max_dual {
            break;
        }
        n_active_last = n_active;
        round_d += 1;
        if need_factor || bk.should_refactor(added.len()) {
            epoch += 1;
            for j in 0..n {
                top[j] = if active[j] { eps_dual } else { prm::BIG };
            }
            // [diag(h) A^T; A -I][x; q] = [0; b̃]、v_y = -q
            rhs[..n].fill(0.0);
            for i in 0..m {
                rhs[n + i] = rnd(i, epoch + 7919) + std.b[i] / (bnorm + 1.0);
            }
            bk.factor(&top, &mid_d, &rhs);
            border_cols.clear();
            need_factor = false;
            if correct {
                dual_correct(&mut bk, &mut y, &mut s, &active, &mut corr_buf, &border_cols);
            }
        } else {
            for &j in &added {
                bk.add_border(col(std, j).iter().map(|&(i, v)| (n + i, v)).collect(), eps_dual);
                border_cols.push(j);
            }
        }
        added.clear();
        bk.solve_base(&mut rhs);
        let mut vn = 0.0f64;
        for i in 0..m {
            vy[i] = -rhs[n + i];
            vn = vn.max(vy[i].abs());
        }
        if vn <= 1e-9 {
            break; // A_D が行フルランク
        }
        at_y(std, &vy, &mut w);
        // 射影の誤差の目安: `D` の列での `|a_j^T v_y|` (厳密には 0)。これと同程度の `|w_j|` は
        // `range(A_D)` の列と区別できない (加えても階数が増えない) ので比率テストで無視する。
        let noise = (0..n).filter(|&j| active[j]).fold(0.0f64, |a, j| a.max(w[j].abs()));
        let wtol = (tunable!("ENOMOTO_T_XO_NOISE_FACTOR", prm::NOISE_FACTOR, f64) * noise).max(prm::EPS_ZERO);
        // 双対の向きも同様: `‖A_D^T v_y‖` が `|v_y|` に比べて小さくなければ (A_D が行フルランクで
        // 射影が丸め誤差だけ) 終わる。
        if noise > prm::NULL_REL_DUAL * vn {
            if debug {
                eprintln!("CROSSOVER dual push: |A_D^T v_y|={noise:.2e} vs |v_y|={vn:.2e}; no direction");
            }
            break;
        }
        if debug && (round_d % 50 == 1 || n_active + 5 >= m) {
            let sd = (0..n).filter(|&j| active[j]).fold(0.0f64, |a, j| a.max(s[j].abs()));
            eprintln!("CROSSOVER dual round={round_d} |v_y|={vn:.3e} max|A_D^T v_y|={noise:.3e} max|s_D|={sd:.2e} active={n_active} borders={}", bk.u.len());
        }
        let ratio = |w: &[f64], s: &[f64], ps: &[PStat], active: &[bool], sign: f64| -> (f64, usize) {
            let mut best = (f64::INFINITY, usize::MAX);
            for j in 0..n {
                if active[j] || ps[j] == PStat::Basic {
                    continue;
                }
                let wj = sign * w[j];
                // s_j(θ) = s_j - θ w_j が 0 を横切る列 (非基底の向きに対して双対実行可能な側から)
                let t = match ps[j] {
                    PStat::Lower if wj > wtol && s[j] >= 0.0 => s[j] / wj,
                    PStat::Upper if wj < -wtol && s[j] <= 0.0 => s[j] / wj,
                    _ => f64::INFINITY,
                };
                if t < best.0 {
                    best = (t, j);
                }
            }
            best
        };
        let mut sign = 1.0;
        let (mut theta, mut jb) = ratio(&w, &s, &ps, &active, 1.0);
        if !theta.is_finite() {
            sign = -1.0;
            (theta, jb) = ratio(&w, &s, &ps, &active, -1.0);
        }
        if !theta.is_finite() {
            break;
        }
        let theta = theta.max(0.0) * sign;
        for i in 0..m {
            y[i] += theta * vy[i];
        }
        for j in 0..n {
            s[j] -= theta * w[j];
        }
        // この歩で D の被約費用がずれた量 |θ|·max|a_j^T v_y| (j ∈ D) が許容を超えたら、その場で補正する
        // (ずれは range(A_D^T) に入るので正確に戻せる)。比率テストの分母が雑音に近い列でブロックすると θ が
        // 1e7 にもなり、放っておくと A_D^T y = c_D が両立しなくなる。
        let drift = theta.abs() * noise;
        active[jb] = true;
        added.push(jb);
        if correct && drift > tunable!("ENOMOTO_T_XO_DRIFT_TOL", prm::DRIFT_TOL, f64) {
            for &j in &added {
                bk.add_border(col(std, j).iter().map(|&(i, v)| (n + i, v)).collect(), eps_dual);
                border_cols.push(j);
            }
            added.clear();
            dual_correct(&mut bk, &mut y, &mut s, &active, &mut corr_buf, &border_cols);
            st.dual_corrections += 1;
        }
        s[jb] = 0.0;
        update_active(&mut active, &mut ps, &s, &mut added);
        n_active = active.iter().filter(|&&a| a).count();
        st.dual_steps += 1;
    }
    if correct && !need_factor {
        for &j in &added {
            bk.add_border(col(std, j).iter().map(|&(i, v)| (n + i, v)).collect(), eps_dual);
            border_cols.push(j);
        }
        added.clear();
        dual_correct(&mut bk, &mut y, &mut s, &active, &mut corr_buf, &border_cols);
    }
    st.dual_active_end = n_active;
    if debug {
        let (mut mx, mut neg, mut big) = (0.0f64, 0usize, 0usize);
        for j in 0..n {
            if active[j] {
                mx = mx.max(s[j].abs());
                if s[j].abs() > 1e-6 {
                    big += 1;
                }
            } else if (ps[j] == PStat::Lower && s[j] < -1e-9) || (ps[j] == PStat::Upper && s[j] > 1e-9) {
                if std.lb[j] != std.ub[j] {
                    neg += 1;
                }
            }
        }
        eprintln!("CROSSOVER after dual push: max|s_D|={mx:.2e} (#>1e-6: {big}) dual-infeasible non-D={neg}");
    }
    st.factors = bk.n_factor;
    st.solves = bk.n_solve;
    drop(bk);
    crate::phase_timing::mark("dual_push_end");
    if debug {
        eprintln!("CROSSOVER dual_push steps={} active {}->{} t={:.3}s", st.dual_steps, st.dual_active_start, n_active, t0.elapsed().as_secs_f64());
    }

    // ---- 4. 一次独立な m 列の選択 ----
    let n_orig = n - m;
    let mut cand_b: Vec<usize> = (0..n).filter(|&j| ps[j] == PStat::Basic).collect();
    let mut cand_d: Vec<usize> = (0..n).filter(|&j| ps[j] != PStat::Basic && active[j]).collect();
    cand_b.sort_by_key(|&j| col(std, j).len());
    cand_d.sort_by_key(|&j| col(std, j).len());
    let mut sel = BasisSelector::new(m, n);
    for &j in &cand_b {
        if sel.full() {
            break;
        }
        if sel.try_add(j, col(std, j)) {
            st.li_from_b += 1;
        }
    }
    for &j in &cand_d {
        if sel.full() {
            break;
        }
        if sel.try_add(j, col(std, j)) {
            st.li_from_d += 1;
        }
    }
    for i in 0..m {
        if sel.full() {
            break;
        }
        if !sel.row_pivoted(i) {
            let j = n_orig + i;
            if !sel.is_chosen[j] && sel.try_add(j, col(std, j)) {
                st.li_slack += 1;
            }
        }
    }
    crate::phase_timing::mark("basis_end");
    if debug {
        eprintln!(
            "CROSSOVER basis from_B={} from_D={} slack={} (m={m}) t={:.3}s",
            st.li_from_b, st.li_from_d, st.li_slack, t0.elapsed().as_secs_f64()
        );
    }
    if !sel.full() {
        return None;
    }

    // ---- 5. 基底状態を作って仕上げ ----
    let mut basis = sel.chosen.clone();
    let mut basis_pos: Vec<Option<usize>> = vec![None; n];
    for (k, &j) in basis.iter().enumerate() {
        basis_pos[j] = Some(k);
    }
    let mut nb_status: Vec<Option<NbStatus>> = vec![None; n];
    for j in 0..n {
        if basis_pos[j].is_some() {
            continue;
        }
        let (l, u) = (std.lb[j], std.ub[j]);
        nb_status[j] = Some(match ps[j] {
            PStat::Lower => NbStatus::Lower,
            PStat::Upper => NbStatus::Upper,
            // 基底に選ばれなかった基底候補: 近い方の有限の境界 (自由列は 0)
            PStat::Basic => {
                if l.is_finite() && (!u.is_finite() || x[j] - l <= u - x[j]) {
                    NbStatus::Lower
                } else if u.is_finite() {
                    NbStatus::Upper
                } else {
                    NbStatus::Zero
                }
            }
        });
    }
    let lu = refactorize(std, &basis_pos, None)?;
    if debug {
        eprintln!("CROSSOVER stats {st:?}");
        // 仕上げ前の基底の主・双対実行不能の数と最大値。
        let mut xb = vec![0.0; m];
        let mut rhs_b = std.b.clone();
        for j in 0..n {
            if let Some(stv) = nb_status[j] {
                let v = match stv {
                    NbStatus::Lower => std.lb[j],
                    NbStatus::Upper => std.ub[j],
                    NbStatus::Zero => 0.0,
                };
                for &(i, a) in col(std, j) {
                    rhs_b[i] -= a * v;
                }
            }
        }
        let mut scratch = vec![0.0; m];
        lu.solve_into(&rhs_b, &mut scratch, &mut xb);
        let (mut np, mut mp) = (0usize, 0.0f64);
        for (k, &j) in basis.iter().enumerate() {
            let viol = (std.lb[j] - xb[k]).max(xb[k] - std.ub[j]);
            if viol > 1e-9 {
                np += 1;
                mp = mp.max(viol);
            }
        }
        let cb: Vec<f64> = basis.iter().map(|&j| std.c[j]).collect();
        let mut yb = vec![0.0; m];
        lu.solve_transpose_into(&cb, &mut scratch, &mut yb);
        let (mut nd, mut md) = (0usize, 0.0f64);
        for j in 0..n {
            if std.lb[j] == std.ub[j] {
                continue;
            }
            let d = std.c[j] - sparse_dot_dense(col(std, j), &yb);
            let viol = match nb_status[j] {
                Some(NbStatus::Lower) => -d,
                Some(NbStatus::Upper) => d,
                Some(NbStatus::Zero) => d.abs(),
                None => 0.0,
            };
            if viol > 1e-9 {
                nd += 1;
                md = md.max(viol);
            }
        }
        eprintln!("CROSSOVER basis quality: primal_infeas={np} (max {mp:.2e}) dual_infeas={nd} (max {md:.2e})");
    }
    let res = polish_with_true_bounds(std, &mut basis, &mut basis_pos, &mut nb_status, lu);
    crate::phase_timing::mark("cleanup_end");
    if debug {
        eprintln!("CROSSOVER cleanup status={:?} total t={:.3}s", res.as_ref().map(|r| r.status.clone()), t0.elapsed().as_secs_f64());
    }
    res
}

/// 優先順に列を受け取り、一次独立なら基底に加える左から順の疎 LU (Gilbert–Peierls)。
/// `L` の列 `k` はピボット行 `piv[k]` と、受理時点の未ピボット行での乗数を持つ。
struct BasisSelector {
    m: usize,
    chosen: Vec<usize>,
    is_chosen: Vec<bool>,
    piv: Vec<usize>,
    lcols: Vec<Vec<(usize, f64)>>,
    pivot_of_row: Vec<usize>,
    // 作業領域
    xw: Vec<f64>,
    mark_row: Vec<bool>,
    nz_rows: Vec<usize>,
    visited: Vec<u32>,
    epoch: u32,
    topo: Vec<usize>,
    stack: Vec<(usize, usize)>,
}

impl BasisSelector {
    fn new(m: usize, n: usize) -> Self {
        BasisSelector {
            m,
            chosen: Vec::with_capacity(m),
            is_chosen: vec![false; n],
            piv: Vec::with_capacity(m),
            lcols: Vec::with_capacity(m),
            pivot_of_row: vec![usize::MAX; m],
            xw: vec![0.0; m],
            mark_row: vec![false; m],
            nz_rows: Vec::new(),
            visited: Vec::new(),
            epoch: 0,
            topo: Vec::new(),
            stack: Vec::new(),
        }
    }

    fn full(&self) -> bool {
        self.chosen.len() >= self.m
    }

    fn row_pivoted(&self, i: usize) -> bool {
        self.pivot_of_row[i] != usize::MAX
    }

    fn touch(&mut self, i: usize) {
        if !self.mark_row[i] {
            self.mark_row[i] = true;
            self.nz_rows.push(i);
        }
    }

    /// 列 `a` (変数 `j`) を試し、一次独立なら受理して `true`。
    fn try_add(&mut self, j: usize, a: &[(usize, f64)]) -> bool {
        // 単位列などで、触れる行がどれもピボット済みでなければ消去は不要。
        self.epoch += 1;
        if self.visited.len() < self.lcols.len() {
            self.visited.resize(self.lcols.len(), 0);
        }
        // DFS で到達する L の列を求め、逆後順 (位相順) に並べる。
        self.topo.clear();
        for &(i, _) in a {
            let k0 = self.pivot_of_row[i];
            if k0 == usize::MAX || self.visited[k0] == self.epoch {
                continue;
            }
            self.visited[k0] = self.epoch;
            self.stack.push((k0, 0));
            while let Some(&mut (k, ref mut pos)) = self.stack.last_mut() {
                let lc = &self.lcols[k];
                let mut pushed = false;
                while *pos < lc.len() {
                    let r = lc[*pos].0;
                    *pos += 1;
                    let k2 = self.pivot_of_row[r];
                    if k2 != usize::MAX && self.visited[k2] != self.epoch {
                        self.visited[k2] = self.epoch;
                        self.stack.push((k2, 0));
                        pushed = true;
                        break;
                    }
                }
                if !pushed {
                    self.topo.push(k);
                    self.stack.pop();
                }
            }
        }
        let mut amax = 0.0f64;
        for &(i, v) in a {
            self.xw[i] += v;
            self.touch(i);
            amax = amax.max(v.abs());
        }
        // 位相順 (逆後順) に消去
        for t in (0..self.topo.len()).rev() {
            let k = self.topo[t];
            let xp = self.xw[self.piv[k]];
            if xp == 0.0 {
                continue;
            }
            for idx in 0..self.lcols[k].len() {
                let (r, l) = self.lcols[k][idx];
                self.xw[r] -= l * xp;
                self.touch(r);
            }
        }
        // 未ピボット行での最大成分
        let mut best = (0.0f64, usize::MAX);
        for &i in &self.nz_rows {
            if self.pivot_of_row[i] == usize::MAX && self.xw[i].abs() > best.0 {
                best = (self.xw[i].abs(), i);
            }
        }
        let ok = best.1 != usize::MAX && best.0 > tunable!("ENOMOTO_T_XO_LI_TOL", prm::LI_TOL, f64) * amax.max(1.0);
        if ok {
            let p = best.1;
            let pv = self.xw[p];
            let mut lc = Vec::new();
            for &i in &self.nz_rows {
                if i != p && self.pivot_of_row[i] == usize::MAX {
                    let v = self.xw[i] / pv;
                    if v.abs() > 1e-14 {
                        lc.push((i, v));
                    }
                }
            }
            let k = self.lcols.len();
            self.lcols.push(lc);
            self.piv.push(p);
            self.pivot_of_row[p] = k;
            self.chosen.push(j);
            self.is_chosen[j] = true;
        }
        for &i in &self.nz_rows {
            self.xw[i] = 0.0;
            self.mark_row[i] = false;
        }
        self.nz_rows.clear();
        ok
    }
}
