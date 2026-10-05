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

use super::basis_kernel::{factorize_basis, ft_max_updates, BasisKernel};
use super::slope_intercept_dual::polish_with_true_bounds;
use super::{perturb_random, sparse_lu, NbStatus, SimplexResult, StdForm};
use crate::interior_point::boxed::{solve_box_lp, solve_box_lp_warm, BoxIpmResult, WarmStart};
use crate::interior_point::pdlp::{solve_pdlp, PdlpOptions};
use crate::interior_point::kkt::AugKkt;
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
    /// 分解 1 回の手間の見積もり `REFACTOR_WORK_FACTOR * (nnz(L) + dim)` (縁の処理の手間がこれを超えたら
    /// 分解し直す。記号分解・AMD を含む分解は三角求解 1 回 (`2 nnz(L)`) のおよそ 10 倍強)。
    pub const REFACTOR_WORK_FACTOR: f64 = 24.0;
    /// 縁取りの列数の上限 (これを超えたら数値分解し直す)。
    pub const MAX_BORDER: usize = 64;
    /// 縁取りの `K^{-1} U` に使ってよいメモリ (バイト)。
    pub const BORDER_MEM: usize = 512 << 20;
    /// 内点法が収束しなかったとき、最良の反復点の相対残差 (許容値 1e-8 の倍率) がこれ以下なら
    /// クロスオーバーに進む (1e4 倍 = 相対 1e-4 程度)。
    pub const IPM_ACCEPT_REL: f64 = 1e4;
    /// 基底の選択で列を受理する残差の相対閾値。
    pub const LI_TOL: f64 = 1e-9;
    /// 双対の押し出しの開始時に `|D| < DUAL_SKIP_FRAC · m` なら押し出しを飛ばす (欠けた階数が多すぎ、
    /// 押し出しの歩数 ≈ `m - rank(A_D)` が仕上げの単体法より高くつく: cre-*, ken-*)。
    pub const DUAL_SKIP_FRAC: f64 = 0.7;
    /// 双対の押し出しの時間が内点法の時間のこの倍数を超えたら打ち切る。
    pub const DUAL_TIME_FACTOR: f64 = 2.0;
}

/// 拡大系 `K = [diag(t) A^T; A diag(m)]` の分解と、分解後に足した縁取り
/// `[K U; U^T C]` の Schur 補行列による正確な求解。
///
/// 押し出しの 1 回の数値分解の間 (エポック) は右辺 (乱数で摂動した費用・右辺) を固定し、
/// `K^{-1} r` を [`Self::set_base`] で一度だけ計算する。各歩の向きは
/// `w = K^{-1} r - (K^{-1} U) S^{-1} (-(U^T K^{-1} r))` で、新しい縁 1 本の求解と `O(k·dim)` だけで済む。
/// 縁の手間の累計が数値分解 1 回の時間を超えたら ([`Self::should_refactor`]) 分解し直す。
struct BorderedKkt {
    /// 直近の分解の拡大系 (列の部分集合 `cols` だけ)。
    kkt: Option<AugKkt>,
    /// 全体の列数 `n` と行数 `m` (呼び出し側の添字は全体の `n + m`)。
    n: usize,
    m: usize,
    /// 分解に含めた列 (`top_j < BIG`) とその局所添字 (`local[j]`、含まない列は `usize::MAX`)。
    cols: Vec<usize>,
    local: Vec<usize>,
    /// 局所系の次元 `|cols| + m`。
    dim: usize,
    /// 縁の列 (局所系の添字での疎ベクトル)。
    u: Vec<Vec<(usize, f64)>>,
    /// `K^{-1} u_i` (密、局所系)。
    kinv_u: Vec<Vec<f64>>,
    /// Schur 補行列 `S = C - U^T K^{-1} U` (行優先、`k x k`、縁を足すたびに広げる)。
    schur: Vec<Vec<f64>>,
    /// `K^{-1} r` (このエポックの右辺、局所系)。
    base: Vec<f64>,
    /// 局所系の作業領域。
    work: Vec<f64>,
    /// 直近の数値分解にかかった時間と、それ以降の縁の処理の累計時間 (秒)。
    factor_secs: f64,
    border_secs: f64,
    /// 数値分解の回数と三角求解の回数 (統計)。
    n_factor: usize,
    n_solve: usize,
    /// 縁 1 本の求解の時間の累計と回数 (見込みの計算用)。
    border_solve_secs: f64,
    n_border_solves: usize,
    /// 数値分解 (記号分解を含む) の時間の累計 (統計)。
    total_factor_secs: f64,
    /// 分解し直しの判断に使う手間の見積もり (決定的): 分解 1 回の手間 (`REFACTOR_WORK_FACTOR * nnz(L)`) と、
    /// 分解し直した後の縁の処理の手間の累計 (三角求解 1 回 = `2 nnz(L)`、縁の補正 = `k * dim`)。
    /// 以前は実測時間で判断していたため、計時の揺れで分解し直す時点 (と引き直す摂動) が実行ごとに変わり、
    /// クロスオーバーの経路と仕上げの時間が大きくばらついた (pilot87 で 5〜800 秒)。
    factor_work: f64,
    border_work: f64,
    nnz_l: f64,
}

impl BorderedKkt {
    fn new(n: usize, m: usize) -> Self {
        BorderedKkt {
            kkt: None,
            n,
            m,
            cols: Vec::new(),
            local: vec![usize::MAX; n],
            dim: 0,
            u: Vec::new(),
            kinv_u: Vec::new(),
            schur: Vec::new(),
            base: Vec::new(),
            work: Vec::new(),
            factor_secs: 0.0,
            border_secs: 0.0,
            n_factor: 0,
            n_solve: 0,
            border_solve_secs: 0.0,
            n_border_solves: 0,
            total_factor_secs: 0.0,
            factor_work: 0.0,
            border_work: 0.0,
            nnz_l: 0.0,
        }
    }

    /// 縁の数の上限 (`K^{-1} U` のメモリ)。
    fn max_border(&self) -> usize {
        (prm::BORDER_MEM / (8 * self.dim.max(1))).clamp(4, prm::MAX_BORDER)
    }

    /// 全体の添字 → 局所系の添字 (x 側は `local[j]`、行側は `|cols| + i`)。
    #[inline]
    fn to_local(&self, g: usize) -> usize {
        if g < self.n {
            self.local[g]
        } else {
            self.cols.len() + (g - self.n)
        }
    }

    /// 対角 (全体の添字、`top_j >= BIG` の列は系から除く) で拡大系を作り直して数値分解し、
    /// 右辺 `r` (全体の添字) で新しいエポックを始める。非零パターンが変わるので記号分解もやり直す
    /// (除いた列の `A_j A_j^T` 由来の fill-in を持ち込まないため。全列で 1 回だけ記号分解して大きな
    /// 対角で列を消す方法より、cre-b で分解・求解とも軽い)。
    fn factor(&mut self, std: &StdForm, top: &[f64], mid: &[f64], r: &[f64]) {
        let t = Instant::now();
        let (n, m) = (self.n, self.m);
        for &j in &self.cols {
            self.local[j] = usize::MAX;
        }
        self.cols.clear();
        for j in 0..n {
            if top[j] < prm::BIG {
                self.local[j] = self.cols.len();
                self.cols.push(j);
            }
        }
        let ns = self.cols.len();
        let rows: Vec<Vec<(usize, f64)>> = (0..m)
            .map(|i| std.rows.row(i).iter().filter(|&&(j, _)| self.local[j] != usize::MAX).map(|&(j, v)| (self.local[j], v)).collect())
            .collect();
        let a_sub = csr_from_rows(&rows, ns);
        drop(rows);
        let mut kkt = AugKkt::new(&a_sub);
        let top_l: Vec<f64> = self.cols.iter().map(|&j| top[j]).collect();
        kkt.factor(&top_l, mid);
        self.dim = ns + m;
        self.base = vec![0.0; self.dim];
        for (k, &j) in self.cols.iter().enumerate() {
            self.base[k] = r[j];
        }
        self.base[ns..].copy_from_slice(&r[n..]);
        kkt.solve_in_place(&mut self.base);
        self.kkt = Some(kkt);
        self.u.clear();
        self.kinv_u.clear();
        self.schur.clear();
        self.n_factor += 1;
        self.n_solve += 1;
        self.factor_secs = t.elapsed().as_secs_f64();
        self.total_factor_secs += self.factor_secs;
        self.border_secs = 0.0;
        self.nnz_l = self.kkt.as_ref().map_or(0, |k| k.factor_nnz()) as f64;
        self.factor_work = tunable!("ENOMOTO_T_XO_REFACTOR_WORK", prm::REFACTOR_WORK_FACTOR, f64) * (self.nnz_l + self.dim as f64);
        self.border_work = 0.0;
    }

    /// 縁を `extra` 本足す前に分解し直すべきか: メモリの上限か、縁の手間 (これまでの累計 + これから足す
    /// `extra` 本の求解の見込み) が数値分解 1 回を上回るなら分解し直す (許容誤差の範囲で一度に多くの列が
    /// D に入る退化した問題 (cre-b) で、1 列ずつ縁を足すより速い)。
    fn should_refactor(&self, extra: usize) -> bool {
        let solve_work = 2.0 * self.nnz_l + (self.u.len() + extra) as f64 * self.dim as f64;
        self.u.len() + extra > self.max_border() || self.border_work + extra as f64 * solve_work > self.factor_work
    }

    /// 縁 `u` (全体の添字) と右下 `c` を加える。x 側の添字は分解に含まれる列でなければならない。
    fn add_border(&mut self, u: Vec<(usize, f64)>, c: f64) {
        let t = Instant::now();
        let u: Vec<(usize, f64)> = u.into_iter().map(|(g, v)| (self.to_local(g), v)).collect();
        debug_assert!(u.iter().all(|&(i, _)| i < self.dim));
        let mut w = vec![0.0; self.dim];
        for &(i, v) in &u {
            w[i] = v;
        }
        self.kkt.as_mut().expect("factor first").solve_in_place(&mut w);
        self.n_solve += 1;
        self.border_solve_secs += t.elapsed().as_secs_f64();
        self.n_border_solves += 1;
        self.border_work += 2.0 * self.nnz_l + (self.u.len() + 1) as f64 * self.dim as f64;
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

    /// 局所系の解 `w` に縁の補正 `w -= (K^{-1} U) S^{-1} (U^T w - rb)` を施す。
    fn border_correct(&self, w: &mut [f64], rb: &[f64]) {
        let k = self.u.len();
        if k == 0 {
            return;
        }
        // S λ = rb - U^T K^{-1} r、w = K^{-1} r - (K^{-1} U) λ
        let g: Vec<f64> = (0..k).map(|i| rb[i] - sparse_dot_dense(&self.u[i], w)).collect();
        let mut sflat: Vec<f64> = self.schur.iter().flatten().copied().collect();
        let lam = dense_solve(k, &mut sflat, g);
        for j in 0..k {
            let l = lam[j];
            if l != 0.0 {
                for (o, wi) in w.iter_mut().zip(&self.kinv_u[j]) {
                    *o -= l * wi;
                }
            }
        }
    }

    /// 局所系の解を全体の添字 (長さ `n + m`、系に含まない列は 0) に書く。
    fn scatter(&self, w: &[f64], out: &mut [f64]) {
        let (n, ns) = (self.n, self.cols.len());
        out[..n].fill(0.0);
        for (k, &j) in self.cols.iter().enumerate() {
            out[j] = w[k];
        }
        out[n..].copy_from_slice(&w[ns..]);
    }

    /// 縁取り系 `[K U; U^T C] [w; λ] = [r; rb]` の `w` を `r` (全体の添字) に上書きする (任意の右辺)。
    /// 系に含まない列の右辺は無視する。
    fn solve_general(&mut self, r: &mut [f64], rb: &[f64]) {
        let t = Instant::now();
        let (n, ns) = (self.n, self.cols.len());
        let mut w = std::mem::take(&mut self.work);
        w.clear();
        w.resize(self.dim, 0.0);
        for (k, &j) in self.cols.iter().enumerate() {
            w[k] = r[j];
        }
        w[ns..].copy_from_slice(&r[n..]);
        self.kkt.as_mut().expect("factor first").solve_in_place(&mut w);
        self.n_solve += 1;
        self.border_work += 2.0 * self.nnz_l + self.u.len() as f64 * self.dim as f64;
        self.border_correct(&mut w, rb);
        self.scatter(&w, r);
        self.work = w;
        self.border_secs += t.elapsed().as_secs_f64();
    }

    /// 現在の縁取り系での `[K U; U^T C]^{-1} [r; 0]` の上段 (`r` はこのエポックの右辺) を `out`
    /// (全体の添字) に書く。
    fn solve_base(&mut self, out: &mut [f64]) {
        let t = Instant::now();
        let mut w = std::mem::take(&mut self.work);
        w.clear();
        w.extend_from_slice(&self.base);
        let zeros = vec![0.0; self.u.len()];
        self.border_work += self.u.len() as f64 * self.dim as f64;
        self.border_correct(&mut w, &zeros);
        self.scatter(&w, out);
        self.work = w;
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
    factor_secs: f64,
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
/// [`solve_ipm_crossover_with`] の設定。
#[derive(Default)]
pub(super) struct XoOptions<'a> {
    /// 内点法の双対の近接中心 (LP の符号。二段解法の段階 A の双対など)。
    pub dual_center: Option<&'a [f64]>,
    /// 真なら問題が双対実行可能だと分かっている (段階 A で `z^1 = 0`)。内点法は「最適」か「双対の発散
    /// (主実行不能)」だけを結論し、主実行不能なら `Infeasible` を返す。
    pub dual_feasible_known: bool,
}

/// 固定列 (`lb == ub`) を除いた問題 (内点法に渡す形)。
struct Reduced {
    free_cols: Vec<usize>,
    a: crate::sparse::FaerCsr,
    b: Vec<f64>,
    l: Vec<f64>,
    u: Vec<f64>,
}

fn reduce_fixed(std: &StdForm) -> Reduced {
    let n = std.n_total;
    let m = std.n_rows;
    let free_cols: Vec<usize> = (0..n).filter(|&j| std.lb[j] < std.ub[j]).collect();
    let mut new_idx = vec![usize::MAX; n];
    for (k, &j) in free_cols.iter().enumerate() {
        new_idx[j] = k;
    }
    let mut rows_j: Vec<Vec<(usize, f64)>> = Vec::with_capacity(m);
    let mut b = std.b.clone();
    for i in 0..m {
        let mut r = Vec::with_capacity(std.rows.row(i).len());
        for &(j, v) in std.rows.row(i) {
            if new_idx[j] != usize::MAX {
                r.push((new_idx[j], v));
            } else {
                b[i] -= v * std.lb[j];
            }
        }
        rows_j.push(r);
    }
    let a = csr_from_rows(&rows_j, free_cols.len());
    let l = free_cols.iter().map(|&j| std.lb[j]).collect();
    let u = free_cols.iter().map(|&j| std.ub[j]).collect();
    Reduced { free_cols, a, b, l, u }
}

/// 費用 0 の実行可能性問題を内点法で解く (`y = 0` が双対実行可能なので、結論は「実行可能」か「双対の発散
/// = 実行不能」)。`Some(true)` は実行可能、`Some(false)` は実行不能 (Farkas の証明つき)、`None` は不明。
pub(super) fn ipm_feasibility(std: &StdForm) -> Option<bool> {
    let r = reduce_fixed(std);
    let c0 = vec![0.0; r.free_cols.len()];
    let warm = WarmStart { dual_feasible_known: true, ..Default::default() };
    let max_iters = tunable!("ENOMOTO_T_IPM_MAX_ITERS", 200usize, usize);
    let ipm = solve_box_lp_warm(&r.a, &r.b, &c0, &r.l, &r.u, max_iters, Some(&warm));
    match ipm.status {
        Status::Optimal => Some(true),
        Status::Infeasible => Some(false),
        _ => None,
    }
}

pub(super) fn solve_ipm_crossover(std: &StdForm) -> Option<SimplexResult> {
    solve_ipm_crossover_with(std, &XoOptions::default())
}

/// 内点法 + クロスオーバー ([`XoOptions`] 付き)。
pub(super) fn solve_ipm_crossover_with(std: &StdForm, xo: &XoOptions) -> Option<SimplexResult> {
    let debug = env_str!("ENOMOTO_DEBUG_CROSSOVER").is_some();
    let t0 = Instant::now();
    let n = std.n_total;
    let m = std.n_rows;
    let mut st = Stats::default();

    // ---- 1. 内点法 (固定列 lb == ub を除いた問題) ----
    let Reduced { free_cols, a: a_j, b: b_j, l: l_j, u: u_j } = reduce_fixed(std);
    let c_j: Vec<f64> = free_cols.iter().map(|&j| std.c[j]).collect();
    let max_iters = tunable!("ENOMOTO_T_IPM_MAX_ITERS", 200usize, usize);
    // 試験用 (`ENOMOTO_T_XO_PDLP`): 先に PDLP で近似解を求め、
    //   1: 内点法の近接中心にする、2: 近接中心と初期点にする、3: 内点法を飛ばしてそのままクロスオーバーに渡す。
    let pdlp_mode = tunable!("ENOMOTO_T_XO_PDLP", 0u8, u8);
    let ipm = if pdlp_mode == 0 {
        if xo.dual_center.is_some() || xo.dual_feasible_known {
            let warm = WarmStart { y: xo.dual_center, dual_feasible_known: xo.dual_feasible_known, ..Default::default() };
            solve_box_lp_warm(&a_j, &b_j, &c_j, &l_j, &u_j, max_iters, Some(&warm))
        } else {
            solve_box_lp(&a_j, &b_j, &c_j, &l_j, &u_j, max_iters)
        }
    } else {
        let opts = PdlpOptions {
            eps: tunable!("ENOMOTO_T_PDLP_EPS", if pdlp_mode == 3 { 1e-8 } else { 1e-4 }, f64),
            max_iters: tunable!("ENOMOTO_T_PDLP_MAX_ITERS", if pdlp_mode == 3 { 1_000_000 } else { 20_000 }, usize),
            time_limit: tunable!("ENOMOTO_T_PDLP_TIME", 1e9, f64),
        };
        let pd = solve_pdlp(&a_j, &b_j, &c_j, &l_j, &u_j, &opts);
        crate::phase_timing::mark("pdlp_end");
        crate::phase_timing::record("xo_pdlp_iters", pd.iters as f64);
        crate::phase_timing::record("xo_pdlp_rel", pd.rel.0.max(pd.rel.1).max(pd.rel.2));
        if debug {
            eprintln!("CROSSOVER pdlp status={:?} iters={} rel={:?} t={:.3}s", pd.status, pd.iters, pd.rel, t0.elapsed().as_secs_f64());
        }
        if pdlp_mode == 3 {
            // PDLP の解をそのまま使う (収束しなければ呼び出し側で解き直す)。
            let mut aty = vec![0.0; free_cols.len()];
            crate::sparse::csr_mat_t_vec_into(&a_j, &pd.y, &mut aty);
            let rc: Vec<f64> = (0..free_cols.len()).map(|k| c_j[k] - aty[k]).collect();
            BoxIpmResult {
                status: if pd.status == Status::Optimal { Status::Optimal } else { Status::NotSolved },
                x: pd.x,
                y: pd.y,
                rc,
                iters: 0,
                rel_res: if pd.status == Status::Optimal { (0.0, 0.0, 0.0) } else { (f64::INFINITY, 0.0, 0.0) },
            }
        } else {
            let warm = WarmStart {
                x: Some(&pd.x),
                y: Some(xo.dual_center.unwrap_or(&pd.y)),
                point: pdlp_mode == 2,
                theta: tunable!("ENOMOTO_T_PDLP_WARM_THETA", 1e-2, f64),
                reg0: env_str!("ENOMOTO_T_PDLP_WARM_REG0").and_then(|v| v.parse().ok()),
                dual_feasible_known: xo.dual_feasible_known,
            };
            solve_box_lp_warm(&a_j, &b_j, &c_j, &l_j, &u_j, max_iters, Some(&warm))
        }
    };
    drop(a_j);
    st.ipm_iters = ipm.iters;
    if crate::cancel::is_cancelled() {
        return None;
    }
    crate::phase_timing::mark("ipm_end");
    let ipm_secs = t0.elapsed().as_secs_f64();
    if debug {
        eprintln!("CROSSOVER ipm status={:?} iters={} rel_res={:?} t={:.3}s", ipm.status, ipm.iters, ipm.rel_res, t0.elapsed().as_secs_f64());
    }
    // 収束しなかった場合も、最良の反復点が相対残差 `IPM_ACCEPT_REL` (各許容値の倍率) 以内なら
    // クロスオーバーに進む (最終段の単体法が残りの誤差を直す)。それ以外は呼び出し側で解き直す。
    // 双対実行可能と分かっていれば、内点法の実行不能の結論 (双対の発散と Farkas の証明) をそのまま採る。
    if xo.dual_feasible_known && ipm.status == Status::Infeasible {
        return Some(SimplexResult { status: Status::Infeasible, x: None });
    }
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
    let mut bk = BorderedKkt::new(n, m);
    let dim = n + m;
    let mut rhs = vec![0.0; dim];

    // ---- 2. 主の押し出し ----
    let mut ps = vec![PStat::Basic; n];
    let tol = prm::TOL_BOUND;
    // 非基底の検出。新たに非基底になった列を返す。
    // 列 `j` (基底候補) が境界に十分近ければ非基底にする (値を境界に置き、`newly` に積む)。
    // 戻り値は `x_j` の変化量 (残差の増分更新用)。
    let detect_one = |j: usize, ps: &mut [PStat], x: &mut [f64], s: &[f64], newly: &mut Vec<usize>| -> Option<f64> {
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
        stat.map(|stv| {
            ps[j] = stv;
            let old = x[j];
            x[j] = if stv == PStat::Lower { l } else { u };
            newly.push(j);
            x[j] - old
        })
    };
    // 残差 `r = b - A x` (主の押し出しの間は増分で保ち、分解し直すたびに一から計算する)。
    let full_resid = |x: &[f64], r: &mut [f64]| {
        r.copy_from_slice(&std.b);
        for j in 0..n {
            if x[j] != 0.0 {
                for &(i, a) in col(std, j) {
                    r[i] -= a * x[j];
                }
            }
        }
    };
    if debug {
        eprintln!("CROSSOVER resid after ipm={:.3e}", primal_resid(std, &x));
    }
    let mut newly = Vec::new();
    for j in 0..n {
        detect_one(j, &mut ps, &mut x, &s, &mut newly);
    }
    // 基底候補の一覧 (押し出しの間は減る一方。以後の走査はこれだけ)。
    let mut basic_list: Vec<usize> = (0..n).filter(|&j| ps[j] == PStat::Basic).collect();
    if debug {
        eprintln!("CROSSOVER resid after detect={:.3e}", primal_resid(std, &x));
    }
    let mut n_basic = basic_list.len();
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
    let drift_tol = tunable!("ENOMOTO_T_XO_DRIFT_TOL", prm::DRIFT_TOL, f64);
    let mut resid = vec![0.0; m];
    let mut av = vec![0.0f64; m];
    // 主の残差の補正: 基底候補の列だけで `A_B δ = r` (`r = b - A x`) の最小ノルム解を足す (射影の正則化による
    // `A x = b` からのずれが歩を重ねて溜まり、最終基底で B^{-1} に増幅されるのを防ぐ)。
    // 縁 (非基底になった列の `v_j = 0`) を含む今の分解で解く。`r` は補正の分だけ更新する。
    let primal_correct = |bk: &mut BorderedKkt, x: &mut [f64], basic_list: &[usize], buf: &mut [f64], r: &mut [f64]| {
        buf[..n].fill(0.0);
        buf[n..].copy_from_slice(r);
        // [D A^T; A -εI][v; y'] = [0; r] で A v ≈ r (D = 1 の列のみ動く)
        let zeros = vec![0.0; bk.u.len()];
        bk.solve_general(buf, &zeros);
        for &j in basic_list {
            if buf[j] != 0.0 {
                let new = (x[j] + buf[j]).clamp(std.lb[j], std.ub[j]);
                let d = new - x[j];
                x[j] = new;
                if d != 0.0 {
                    for &(i, a) in col(std, j) {
                        r[i] -= a * d;
                    }
                }
            }
        }
    };
    let mut corr_buf = vec![0.0; dim];
    // 向きが見つからなかったとき、摂動を引き直して 1 回だけ分解し直したか。
    let mut retried = false;
    // Megiddo 式の押し出し (既定で有効、`ENOMOTO_T_XO_MEGIDDO=0` で無効)。
    // 試験用: 残りの超基底が少なくなったら (`|B| <= m + megiddo_switch`)、射影の押し出しを打ち切って
    // 基底の選択 + Megiddo 式の押し出しに任せる (`ENOMOTO_T_XO_MEGIDDO_SWITCH`、0 で無効。
    // Megiddo 式の押し出しが有効なときだけ効く)。
    let megiddo = tunable!("ENOMOTO_T_XO_MEGIDDO", 1u8, u8) != 0;
    let megiddo_switch = if megiddo { tunable!("ENOMOTO_T_XO_MEGIDDO_SWITCH", 0usize, usize) } else { 0 };
    loop {
        if n_basic == 0 || (round >= 2 && n_basic <= m && n_basic >= n_basic_last) || round > max_primal {
            break;
        }
        if megiddo_switch > 0 && n_basic <= m + megiddo_switch {
            break;
        }
        n_basic_last = n_basic;
        round += 1;
        if crate::cancel::is_cancelled() {
            return None;
        }
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
            bk.factor(std, &top, &mid_p, &rhs);
            need_factor = false;
            full_resid(&x, &mut resid);
            if correct {
                primal_correct(&mut bk, &mut x, &basic_list, &mut corr_buf, &mut resid);
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
        let mut finite = true;
        for &j in &basic_list {
            let r = rhs[j];
            finite &= r.is_finite();
            v[j] = if r.abs() > prm::V_REL_ZERO { r } else { 0.0 };
            vnorm = vnorm.max(v[j].abs());
        }
        // 向きが本当に null(A_B) に入っているか: `‖A v‖ ≤ NULL_REL ‖v‖`。A_B が列フルランクになった後の
        // 射影は丸め誤差だけで (blend で |v| ≈ 4e-7、‖A v‖ ≈ |v|)、それを向きとみなすと比率テストが
        // θ ≈ 1e7 で進んで `A x = b` を壊す。射影が非有限 (分解の破綻: pds-20) なら向きとみなさない
        // (`inf > 1e-6·inf` は偽なので、有限性を先に確かめる)。
        let mut avn = 0.0f64;
        let has_dir = finite && vnorm > prm::V_REL_ZERO && {
            av.fill(0.0);
            for &j in &basic_list {
                if v[j] != 0.0 {
                    for &(i, a) in col(std, j) {
                        av[i] += a * v[j];
                    }
                }
            }
            avn = av.iter().fold(0.0f64, |mm, t| mm.max(t.abs()));
            avn <= prm::NULL_REL * vnorm
        };
        if !has_dir {
            if debug {
                eprintln!(
                    "CROSSOVER primal push: no null-space direction (finite={finite} |v|={vnorm:.2e} |Av|={avn:.2e}, basic={n_basic}, m={m})"
                );
            }
            // A_B が列フルランクなら正常な終わり。|B| > m なのに向きが無いのは射影の数値的な失敗なので、
            // 摂動を引き直して 1 回だけ分解し直し、それでも駄目なら押し出しを終えて基底の選択に任せる
            // (以前は分解し直しを 10n 回まで繰り返していた: pds-20)。
            if n_basic > m && !retried {
                retried = true;
                need_factor = true;
                n_basic_last = usize::MAX;
                continue;
            }
            break;
        }
        retried = false;
        let ratio = |v: &[f64], x: &[f64], sign: f64| -> (f64, usize) {
            let mut best = (f64::INFINITY, usize::MAX);
            for &j in &basic_list {
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
        for &j in &basic_list {
            if v[j] != 0.0 {
                x[j] += theta * sign * v[j];
            }
        }
        // r -= θ·sign·A v
        let ts = theta * sign;
        for i in 0..m {
            resid[i] -= ts * av[i];
        }
        // ブロックした列は境界に置く
        let vj = sign * v[jb];
        let old = x[jb];
        if vj < 0.0 {
            x[jb] = std.lb[jb];
            ps[jb] = PStat::Lower;
        } else {
            x[jb] = std.ub[jb];
            ps[jb] = PStat::Upper;
        }
        let d = x[jb] - old;
        if d != 0.0 {
            for &(i, a) in col(std, jb) {
                resid[i] -= a * d;
            }
        }
        newly.push(jb);
        basic_list.retain(|&j| ps[j] == PStat::Basic);
        if correct && theta * avn > drift_tol {
            // この歩の `A x = b` からのずれを補正する (縁を足してから)。
            for &j in &newly {
                bk.add_border(vec![(j, 1.0)], 0.0);
            }
            newly.clear();
            primal_correct(&mut bk, &mut x, &basic_list, &mut corr_buf, &mut resid);
            st.primal_corrections += 1;
        }
        let r_move = if debug { primal_resid(std, &x) } else { 0.0 };
        let nbefore = newly.len();
        for k in 0..basic_list.len() {
            let j = basic_list[k];
            if let Some(d) = detect_one(j, &mut ps, &mut x, &s, &mut newly) {
                if d != 0.0 {
                    for &(i, a) in col(std, j) {
                        resid[i] -= a * d;
                    }
                }
            }
        }
        basic_list.retain(|&j| ps[j] == PStat::Basic);
        if debug {
            eprintln!(
                "CROSSOVER pstep theta={theta:.3e} |v|={vnorm:.3e} |Av|={avn:.3e} resid_move={r_move:.3e} resid_detect={:.3e} snapped={}",
                primal_resid(std, &x),
                newly.len() - nbefore
            );
        }
        n_basic = basic_list.len();
        st.primal_steps += 1;
    }
    if correct && !need_factor {
        for &j in &newly {
            bk.add_border(vec![(j, 1.0)], 0.0);
        }
        newly.clear();
        primal_correct(&mut bk, &mut x, &basic_list, &mut corr_buf, &mut resid);
    }
    st.basic_after_push = n_basic;
    crate::phase_timing::record("xo_excess", n_basic as f64 - m as f64);
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
    // 列 `j` (非活性) を必要なら活性にする (固定列の非基底の向きも被約費用の符号で付け直す)。
    let activate_if = |j: usize, active: &mut [bool], ps: &mut [PStat], s: &[f64], added: &mut Vec<usize>| {
        if ps[j] != PStat::Basic && std.lb[j] == std.ub[j] {
            ps[j] = if s[j] >= 0.0 { PStat::Lower } else { PStat::Upper };
        }
        // 参照実装は下限だけの列を `s_j <= tol` で活性にする (負の s も入る) が、負に外れた列を D に
        // 入れると `A_D^T y = c_D` が両立しなくなり押し出しの不変条件が崩れる (degen3) ので、
        // 向きによらず `|s_j| <= tol` の列だけを加える (外れた列は仕上げの単体法が直す)。
        if ps[j] == PStat::Basic || s[j].abs() <= tol {
            active[j] = true;
            added.push(j);
        }
    };
    let mut added = Vec::new();
    for j in 0..n {
        activate_if(j, &mut active, &mut ps, &s, &mut added);
    }
    // 非活性の列 (比率テスト・被約費用の更新・活性化の判定の対象。活性になれば外す)。
    let mut cand: Vec<usize> = (0..n).filter(|&j| !active[j]).collect();
    // 各列が D に入った時点の被約費用 (押し出しは理論上これを保つ: A_D^T v_y = 0)。補正はこの値へ戻す
    // (0 へ戻すと、内点法の誤差で 0 でない基底候補の s があるとき D が m 列を超えた後に両立しない: ken-13)。
    let mut s_tgt = vec![0.0; n];
    for &j in &added {
        s_tgt[j] = s[j];
    }
    let mut n_active = added.len();
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
    let mut is_border = vec![false; n];
    // 双対の補正: `A_D^T δ = s_D - s_tgt` の最小ノルム解 (`δ ∈ range(A_D)`) を `y` に足して `D` の被約費用を
    // 入った時点の値に戻す (射影の誤差で `y` がずれていくのを防ぐ)。縁の列 (後から `D` に加えた列) の
    // 右辺も同じ。`D` の列の `s` は歩ごとには更新しないので、補正の前後で `s = c - A^T y` を計算し直す。
    let dual_correct = |bk: &mut BorderedKkt, y: &mut [f64], s: &mut [f64], s_tgt: &[f64], active: &[bool], buf: &mut [f64], border_cols: &[usize], is_border: &[bool]| {
        let mut aty = vec![0.0; n];
        at_y(std, y, &mut aty);
        for j in 0..n {
            s[j] = std.c[j] - aty[j];
        }
        // [diag(h) A^T; A -I][x; q] = [f; 0] → A_D^T q ≈ f、q = A x ∈ range(A_D)
        for j in 0..n {
            buf[j] = if active[j] && !is_border[j] { s[j] - s_tgt[j] } else { 0.0 };
        }
        buf[n..].fill(0.0);
        let rb: Vec<f64> = border_cols.iter().map(|&j| s[j] - s_tgt[j]).collect();
        bk.solve_general(buf, &rb);
        for i in 0..m {
            y[i] += buf[n + i];
        }
        at_y(std, y, &mut aty);
        for j in 0..n {
            s[j] = std.c[j] - aty[j];
        }
    };
    let mut border_cols: Vec<usize> = Vec::new();
    // 試験用: |v_y| が初回の `vy_rel` 倍を下回ったら終える (`ENOMOTO_T_XO_VY_REL`、0 で無効)。終盤は射影の
    // 雑音の向きで列を D に入れて基底の質を壊すことがある (ken-13)。
    let vy_rel = tunable!("ENOMOTO_T_XO_VY_REL", 0.0f64, f64);
    let mut vn_first = f64::NAN;
    // 開始時の |D|/m が `skip_frac` 未満なら双対の押し出しを飛ばし (`ENOMOTO_T_XO_DUAL_SKIP_FRAC`)、
    // 押し出しの時間が内点法の時間の `time_factor` 倍を超えたら打ち切る (`ENOMOTO_T_XO_DUAL_TIME_FACTOR`)。
    // どちらも 0 で無効。残りの階数は仕上げの単体法が埋める (既定値は Netlib + Kennington の比較で
    // 決めた: docs/crossover.md)。
    let skip_frac = tunable!("ENOMOTO_T_XO_DUAL_SKIP_FRAC", prm::DUAL_SKIP_FRAC, f64);
    let time_factor = tunable!("ENOMOTO_T_XO_DUAL_TIME_FACTOR", prm::DUAL_TIME_FACTOR, f64);
    let t_dual0 = Instant::now();
    let skip_dual = (n_active as f64) < skip_frac * m as f64;
    if skip_dual && debug {
        eprintln!("CROSSOVER dual push skipped (|D|={n_active} < {skip_frac} m)");
    }
    let drift_tol = tunable!("ENOMOTO_T_XO_DRIFT_TOL", prm::DRIFT_TOL, f64);
    let noise_factor = tunable!("ENOMOTO_T_XO_NOISE_FACTOR", prm::NOISE_FACTOR, f64);
    loop {
        if skip_dual || (round_d >= 2 && n_active <= n_active_last) || round_d > max_dual {
            break;
        }
        if time_factor > 0.0 && t_dual0.elapsed().as_secs_f64() > time_factor * ipm_secs {
            if debug {
                eprintln!("CROSSOVER dual push: time cap ({time_factor} x ipm {ipm_secs:.2}s) reached");
            }
            break;
        }
        n_active_last = n_active;
        round_d += 1;
        if crate::cancel::is_cancelled() {
            return None;
        }
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
            bk.factor(std, &top, &mid_d, &rhs);
            for &j in &border_cols {
                is_border[j] = false;
            }
            border_cols.clear();
            need_factor = false;
            if correct {
                dual_correct(&mut bk, &mut y, &mut s, &s_tgt, &active, &mut corr_buf, &border_cols, &is_border);
            }
        } else {
            for &j in &added {
                bk.add_border(col(std, j).iter().map(|&(i, v)| (n + i, v)).collect(), eps_dual);
                border_cols.push(j);
                is_border[j] = true;
            }
        }
        added.clear();
        bk.solve_base(&mut rhs);
        let mut vn = 0.0f64;
        let mut finite = true;
        for i in 0..m {
            vy[i] = -rhs[n + i];
            finite &= vy[i].is_finite();
            vn = vn.max(vy[i].abs());
        }
        if !finite {
            if debug {
                eprintln!("CROSSOVER dual push: non-finite direction; stopping");
            }
            break;
        }
        if vn <= 1e-9 {
            break; // A_D が行フルランク
        }
        if vn_first.is_nan() {
            vn_first = vn;
        } else if vy_rel > 0.0 && vn < vy_rel * vn_first {
            if debug {
                eprintln!("CROSSOVER dual push: |v_y|={vn:.2e} < {vy_rel} x first {vn_first:.2e}; stopping");
            }
            break;
        }
        at_y(std, &vy, &mut w);
        // 射影の誤差の目安: `D` の列での `|a_j^T v_y|` (厳密には 0)。これと同程度の `|w_j|` は
        // `range(A_D)` の列と区別できない (加えても階数が増えない) ので比率テストで無視する。
        let noise = {
            use rayon::prelude::*;
            (0..n).into_par_iter().with_min_len(4096).filter(|&j| active[j]).map(|j| w[j].abs()).reduce(|| 0.0f64, f64::max)
        };
        let wtol = (noise_factor * noise).max(prm::EPS_ZERO);
        // 双対の向きも同様: `‖A_D^T v_y‖` が `|v_y|` に比べて小さくなければ (A_D が行フルランクで
        // 射影が丸め誤差だけ) 終わる。
        if noise > prm::NULL_REL_DUAL * vn {
            if debug {
                eprintln!("CROSSOVER dual push: |A_D^T v_y|={noise:.2e} vs |v_y|={vn:.2e}; no direction");
            }
            break;
        }
        if debug && (round_d % 50 == 1 || n_active + 5 >= m) {
            eprintln!("CROSSOVER dual round={round_d} |v_y|={vn:.3e} max|A_D^T v_y|={noise:.3e} active={n_active} borders={}", bk.u.len());
        }
        let ratio = |w: &[f64], s: &[f64], ps: &[PStat], sign: f64| -> (f64, usize) {
            let mut best = (f64::INFINITY, usize::MAX);
            for &j in &cand {
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
        let (mut theta, mut jb) = ratio(&w, &s, &ps, 1.0);
        if !theta.is_finite() {
            sign = -1.0;
            (theta, jb) = ratio(&w, &s, &ps, -1.0);
        }
        if !theta.is_finite() {
            break;
        }
        let theta = theta.max(0.0) * sign;
        for i in 0..m {
            y[i] += theta * vy[i];
        }
        // 非活性の列の被約費用だけ更新する (D の列の s は補正の前に計算し直す)。
        for &j in &cand {
            s[j] -= theta * w[j];
        }
        // この歩で D の被約費用がずれた量 |θ|·max|a_j^T v_y| (j ∈ D) が許容を超えたら、その場で補正する
        // (ずれは range(A_D^T) に入るので正確に戻せる)。比率テストの分母が雑音に近い列でブロックすると θ が
        // 1e7 にもなり、放っておくと A_D^T y = c_D が両立しなくなる。
        let drift = theta.abs() * noise;
        active[jb] = true;
        s_tgt[jb] = 0.0;
        added.push(jb);
        if correct && drift > drift_tol {
            for &j in &added {
                bk.add_border(col(std, j).iter().map(|&(i, v)| (n + i, v)).collect(), eps_dual);
                border_cols.push(j);
                is_border[j] = true;
            }
            added.clear();
            dual_correct(&mut bk, &mut y, &mut s, &s_tgt, &active, &mut corr_buf, &border_cols, &is_border);
            st.dual_corrections += 1;
        }
        s[jb] = 0.0;
        let before = added.len();
        for &j in &cand {
            if !active[j] {
                activate_if(j, &mut active, &mut ps, &s, &mut added);
            }
        }
        for k in before..added.len() {
            s_tgt[added[k]] = s[added[k]];
        }
        n_active += 1 + (added.len() - before);
        cand.retain(|&j| !active[j]);
        st.dual_steps += 1;
    }
    if correct && !need_factor {
        for &j in &added {
            bk.add_border(col(std, j).iter().map(|&(i, v)| (n + i, v)).collect(), eps_dual);
            border_cols.push(j);
            is_border[j] = true;
        }
        added.clear();
        dual_correct(&mut bk, &mut y, &mut s, &s_tgt, &active, &mut corr_buf, &border_cols, &is_border);
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
    st.factor_secs = bk.total_factor_secs;
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
    let mut lu = factorize_basis(std, &basis_pos, None)?;
    // 基底に選ばれず、境界から離れたまま残った基底候補 (超基底変数)。今は上で近い方の境界へ移した
    // 扱いにしてあり、Megiddo 式の押し出しが有効なら、そこから主実行可能性を保って境界へ押し出す。
    let leftover: Vec<usize> = (0..n)
        .filter(|&j| {
            ps[j] == PStat::Basic
                && basis_pos[j].is_none()
                && match nb_status[j] {
                    Some(NbStatus::Lower) => x[j] - std.lb[j] > tol,
                    Some(NbStatus::Upper) => std.ub[j] - x[j] > tol,
                    _ => x[j].abs() > tol,
                }
        })
        .collect();
    crate::phase_timing::record("xo_leftover", leftover.len() as f64);
    if debug {
        eprintln!("CROSSOVER leftover superbasics={} (|B|-m after primal push={})", leftover.len(), st.basic_after_push as i64 - m as i64);
    }
    if megiddo && !leftover.is_empty() {
        let ms = megiddo_push(std, &x, &leftover, &mut basis, &mut basis_pos, &mut nb_status, lu)?;
        lu = ms.lu;
        crate::phase_timing::record("xo_megiddo_pivots", ms.pivots as f64);
        crate::phase_timing::record("xo_megiddo_bound", ms.to_bound as f64);
        crate::phase_timing::record("xo_megiddo_unresolved", ms.unresolved as f64);
        crate::phase_timing::mark("megiddo_end");
        if debug {
            eprintln!(
                "CROSSOVER megiddo pivots={} to_bound={} unresolved={} t={:.3}s",
                ms.pivots,
                ms.to_bound,
                ms.unresolved,
                t0.elapsed().as_secs_f64()
            );
        }
        if crate::cancel::is_cancelled() {
            return None;
        }
    }
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

/// [`megiddo_push`] の結果。
struct MegiddoResult {
    /// 最後の基底の分解 (押し出しの後で分解し直したもの)。
    lu: sparse_lu::FtLu,
    /// 超基底変数を基底に入れ、塞いだ基底変数を境界へ出した回数。
    pivots: usize,
    /// 超基底変数自身が境界に達した数。
    to_bound: usize,
    /// どちら向きにも塞がれず押し出せなかった数 (呼び出し側が先に決めた境界に置いたまま)。
    unresolved: usize,
}

/// Megiddo (1991) 式の超基底変数の押し出し。
///
/// 正則な基底 `B` (`basis`/`basis_pos`、分解 `lu`) と、基底に選ばれず境界から離れたまま残った
/// 列 `sup` (値は `x`) を受け取り、主実行可能性を保ったまま `sup` を 1 本ずつ消す。`j ∈ sup` を
/// 向き `σ` に動かすと `x_B` は `-σ B^{-1} a_j` の向きに動く (`A x = b` は厳密に保たれる)。
/// 比率テストで
///
/// - `x_j` 自身が先に境界に達したら、`j` はその境界の非基底になる。
/// - 基底変数 `x_r` が先に境界に達したら、`j` を基底位置 `r` に入れ、`x_r` をその境界の非基底に出す。
///
/// どちらでも超基底変数は 1 本減るので、`|sup|` 回で終わる。`σ` は被約費用 `d_j` が目的を
/// 悪化させない向き (`d_j ≈ 0` なら近い方の境界の向き) を選ぶ。比率テストは Harris の 2 段で、
/// 既に僅かに外れている基底変数は外向きには動かさない (歩幅 0 で塞ぐ)。両向きとも塞がれない列
/// (自由列だけの向き) は、`nb_status` に入っている境界 (呼び出し側が決めた近い方) のまま残す。
///
/// 呼び出し側の `nb_status` は、`sup` の列も含めて全非基底列に入っていること。終わりに分解し
/// 直した `lu` を返す。基底が特異になったら `None` (呼び出し側は解き直す)。
fn megiddo_push(
    std: &StdForm,
    x: &[f64],
    sup: &[usize],
    basis: &mut [usize],
    basis_pos: &mut [Option<usize>],
    nb_status: &mut [Option<NbStatus>],
    mut lu: sparse_lu::FtLu,
) -> Option<MegiddoResult> {
    let m = std.n_rows;
    let n = std.n_total;
    let tol_p = tunable!("ENOMOTO_T_XO_MEGIDDO_TOL", 1e-9f64, f64);
    let piv_rel = tunable!("ENOMOTO_T_XO_MEGIDDO_PIV", 1e-7f64, f64);
    // 非基底の値: `sup` は `x` の値、それ以外は `nb_status` の境界 (自由列は 0)。
    let mut is_sup = vec![false; n];
    for &j in sup {
        is_sup[j] = true;
    }
    let mut xs: Vec<f64> = sup.iter().map(|&j| x[j]).collect();
    let nb_value = |j: usize, st: Option<NbStatus>| match st {
        Some(NbStatus::Lower) => std.lb[j],
        Some(NbStatus::Upper) => std.ub[j],
        _ => 0.0,
    };
    // `x_B = B^{-1} (b - Σ_{非基底} a_j x_j)`。`sup` の値は `vals` (添字は `sup` の位置) から取る。
    let recompute_xb = |lu: &sparse_lu::FtLu, basis_pos: &[Option<usize>], nb_status: &[Option<NbStatus>], is_sup: &[bool], sup_val: &dyn Fn(usize) -> f64, xb: &mut [f64]| {
        let mut rhs = std.b.clone();
        for j in 0..n {
            if basis_pos[j].is_some() {
                continue;
            }
            let v = if is_sup[j] { sup_val(j) } else { nb_value(j, nb_status[j]) };
            if v != 0.0 {
                for &(i, a) in col(std, j) {
                    rhs[i] -= a * v;
                }
            }
        }
        let mut scratch = vec![0.0; m];
        lu.solve_into(&rhs, &mut scratch, xb);
    };
    let mut sup_idx = vec![usize::MAX; n];
    for (k, &j) in sup.iter().enumerate() {
        sup_idx[j] = k;
    }
    let mut xb = vec![0.0; m];
    {
        let xs_ref = &xs;
        let sv = |j: usize| xs_ref[sup_idx[j]];
        recompute_xb(&lu, basis_pos, nb_status, &is_sup, &sv, &mut xb);
    }
    let mut cb: Vec<f64> = basis.iter().map(|&j| std.c[j]).collect();
    let mut kernel = BasisKernel::new(m, ft_max_updates(m));
    let mut d = vec![0.0; m];
    let mut rho = vec![0.0; m];
    let (mut pivots, mut to_bound, mut unresolved) = (0usize, 0usize, 0usize);
    // 向き `σ` での比率テスト。(歩幅, 塞ぐ基底位置 (自身なら None), 塞いだ境界は上限か)。
    let ratio = |sigma: f64, j: usize, xj: f64, d: &[f64], xb: &[f64], basis: &[usize]| -> (f64, Option<usize>, bool) {
        let own = if sigma > 0.0 { std.ub[j] - xj } else { xj - std.lb[j] };
        let dmax = d.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let piv_tol = (piv_rel * dmax).max(1e-11);
        // 1 段目: 許容誤差 tol_p だけ緩めた歩幅の上限。
        let mut tmax = own.max(0.0) + tol_p;
        for k in 0..m {
            let dk = -sigma * d[k];
            if dk.abs() <= piv_tol {
                continue;
            }
            let q = basis[k];
            let t = if dk < 0.0 {
                if !std.lb[q].is_finite() {
                    continue;
                }
                (xb[k] - std.lb[q] + tol_p).max(0.0) / -dk
            } else {
                if !std.ub[q].is_finite() {
                    continue;
                }
                (std.ub[q] - xb[k] + tol_p).max(0.0) / dk
            };
            tmax = tmax.min(t);
        }
        if !tmax.is_finite() {
            return (f64::INFINITY, None, false);
        }
        // 2 段目: 上限以内で塞ぐ候補のうち |d_k| が最大の基底変数 (自身の境界は上限以内なら優先しない)。
        let mut best: Option<(usize, f64, bool)> = None;
        let mut best_piv = 0.0;
        for k in 0..m {
            let dk = -sigma * d[k];
            if dk.abs() <= piv_tol {
                continue;
            }
            let q = basis[k];
            let (t, up) = if dk < 0.0 {
                if !std.lb[q].is_finite() {
                    continue;
                }
                ((xb[k] - std.lb[q]).max(0.0) / -dk, false)
            } else {
                if !std.ub[q].is_finite() {
                    continue;
                }
                ((std.ub[q] - xb[k]).max(0.0) / dk, true)
            };
            if t <= tmax && dk.abs() > best_piv {
                best_piv = dk.abs();
                best = Some((k, t, up));
            }
        }
        match best {
            // 自身の境界が基底変数より手前 (または同じ) なら自身を境界へ。
            Some((k, t, up)) if t < own => (t, Some(k), up),
            _ if own.is_finite() => (own.max(0.0), None, sigma > 0.0),
            Some((k, t, up)) => (t, Some(k), up),
            None => (f64::INFINITY, None, false),
        }
    };
    for (si, &j) in sup.iter().enumerate() {
        if si % 64 == 0 && crate::cancel::is_cancelled() {
            break;
        }
        let xj = xs[si];
        kernel.ftran_col(&lu, col(std, j), &mut d);
        let mut dj = std.c[j];
        for k in 0..m {
            if d[k] != 0.0 {
                dj -= cb[k] * d[k];
            }
        }
        let dtol = 1e-9 * (1.0 + std.c[j].abs());
        // 目的を悪化させない向き。d_j ≈ 0 なら近い方の有限の境界の向き。
        let pref = if dj > dtol {
            -1.0
        } else if dj < -dtol {
            1.0
        } else {
            let dl = xj - std.lb[j];
            let du = std.ub[j] - xj;
            if dl <= du { -1.0 } else { 1.0 }
        };
        let mut sigma = pref;
        let (mut t, mut blk, mut up) = ratio(sigma, j, xj, &d, &xb, basis);
        if !t.is_finite() && dj.abs() <= dtol {
            sigma = -pref;
            (t, blk, up) = ratio(sigma, j, xj, &d, &xb, basis);
        }
        if !t.is_finite() {
            // 押し出せない (自由列だけの向き、または目的が下がり続ける向き): 呼び出し側の境界のまま。
            unresolved += 1;
            is_sup[j] = false;
            continue;
        }
        // x_B ← x_B - t σ d
        let ts = t * sigma;
        if ts != 0.0 {
            for k in 0..m {
                if d[k] != 0.0 {
                    xb[k] -= ts * d[k];
                }
            }
        }
        is_sup[j] = false;
        match blk {
            None => {
                nb_status[j] = Some(if up { NbStatus::Upper } else { NbStatus::Lower });
                to_bound += 1;
            }
            Some(r) => {
                let q = basis[r];
                kernel.btran_row(&lu, r, &mut rho);
                basis[r] = j;
                basis_pos[j] = Some(r);
                basis_pos[q] = None;
                nb_status[j] = None;
                nb_status[q] = Some(if up { NbStatus::Upper } else { NbStatus::Lower });
                xb[r] = xj + ts;
                cb[r] = std.c[j];
                pivots += 1;
                if kernel.update_and_check(&mut lu, r).is_due() {
                    lu = factorize_basis(std, basis_pos, Some(&lu))?;
                    kernel = BasisKernel::new(m, ft_max_updates(m));
                    let xs_ref = &xs;
                    let sv = |jj: usize| xs_ref[sup_idx[jj]];
                    recompute_xb(&lu, basis_pos, nb_status, &is_sup, &sv, &mut xb);
                }
            }
        }
        xs[si] = f64::NAN; // 使い終わり (is_sup が偽なので読まれない)
    }
    let lu = factorize_basis(std, basis_pos, Some(&lu))?;
    Some(MegiddoResult { lu, pivots, to_bound, unresolved })
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
