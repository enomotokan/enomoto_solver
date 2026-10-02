//! 双対化 (行数 ≫ 列数の問題を、その双対 LP で解く。CLP の `dualize`、SoPlex の行表現と同じ考え方)。
//!
//! 行数 `m` が構造列数 `n` よりはるかに多い問題 (Mittelmann supportcase10: 前処理後 105,209 行 × 8,955 列) では、
//! 基底の次元が `m` なので FTRAN/BTRAN が重い。双対 LP は行数 `n`・列数 `m` 程度になり、基底が小さい。
//!
//! 標準形 `M z = b` (`z = (x, s)`、`s` は各行のスラック `σ_i`)、`l <= z <= u` の構造列を有限な境界へ平行移動
//! (`x = x0 + x'`) してから双対をとる。双対 LP (最小化) は
//!
//! - 列: 行 `i` ごとの双対変数 `y_i` (係数は行 `i` の構造列の係数、費用 `-b'_i` + スラックの境界の寄与。
//!   スラックの境界で符号が決まり、範囲行は `σ y ≤ 0` と `σ y ≥ 0` の 2 列に分ける)、両側有限な構造列 `j` の `μ_j`
//!   (係数 `-1`、費用は幅 `u_j - l_j`)。
//! - 行: 固定でない構造列 `j` ごとに `Σ_i a_ij y_i (- μ_j) + t_j = c_j`。`t_j` は下限側 (`x' >= 0`) なら `[0, ∞)`、
//!   上限側 (`x' <= 0`) なら `(-∞, 0]`、自由列なら `[0, 0]`。
//!
//! 双対 LP の最適基底の行の双対 `π` (`B^-T c_B`) から `x'_j = -π_j` で元の解を戻す。双対 LP が最適で
//! 終わらない、または戻した解が元の問題の制約を許容誤差で満たさないときは `None` (呼び出し側が普通に解く)。

use super::{slope_intercept_dual, SimplexResult, Status, StdForm};

/// 双対化する最小の行数 (`ENOMOTO_T_DUALIZE_MIN_ROWS`、0 = 無効)。CLP と同じ 50,000。
const DUALIZE_MIN_ROWS: usize = 50_000;
/// 双対化する最小の「行数 / 構造列数」(`ENOMOTO_T_DUALIZE_MIN_RATIO`)。CLP と同じ 5。
const DUALIZE_MIN_RATIO: f64 = 5.0;
/// 双対 LP で、初期配置で無限の側に置かれる (費用の符号が無限の境界を好む) 双対列の数の上限 (双対 LP の行数に対する比、
/// `ENOMOTO_T_DUALIZE_MAX_M_SIDE`)。傾き・切片双対二段解法の段階 A はそれらの列を 1 本ずつ基底に入れるので、
/// 多いと段階 A が長い (前処理なしで作った neos の双対は 45 万列が無限の側で、段階 A が 40 万反復を超えた。
/// 前処理・平行移動の後の neos の双対では 60,542 列 = 行数の 1.7 倍で、135 s で解ける)。
const DUALIZE_MAX_M_SIDE: f64 = 4.0;
/// 戻した解の制約違反の許容誤差 (`|右辺|` に対する相対を足す)。
const DUALIZE_FEAS_TOL: f64 = 1e-6;

/// 双対列の種類 (元の行 `i` の双対 `y_i`、または両側有限な構造列の `μ_j`)。
#[derive(Clone, Copy)]
enum DualCol {
    /// 行 `i` の `y_i` (分割した片方を含む)。
    Y(usize),
    /// 構造列 `j` の上限の双対 `μ_j`。
    Mu(usize),
}

/// 双対化するか: 行数 `m >= DUALIZE_MIN_ROWS`、`m >= DUALIZE_MIN_RATIO * n` (構造列数)。
fn applicable(std: &StdForm) -> bool {
    let min_rows = tunable!("ENOMOTO_T_DUALIZE_MIN_ROWS", DUALIZE_MIN_ROWS, usize);
    let ratio = tunable!("ENOMOTO_T_DUALIZE_MIN_RATIO", DUALIZE_MIN_RATIO, f64);
    let m = std.n_rows;
    let n = std.n_total - m;
    min_rows != 0 && m >= min_rows && (m as f64) >= ratio * n as f64
}

/// 双対化して解く。対象外、または途中で諦めたときは `None`。
pub(super) fn solve(std: &StdForm, opts: &crate::types::LpOptions) -> Option<SimplexResult> {
    if !applicable(std) {
        return None;
    }
    let debug = env_str!("ENOMOTO_DEBUG_DUALIZE").is_some();
    let t0 = std::time::Instant::now();
    let m = std.n_rows;
    let n = std.n_total - m;
    let inf = f64::INFINITY;

    // 構造列の平行移動 `x0` と、移動後の幅 (`x'` の範囲)。
    let mut x0 = vec![0.0; n];
    // 列の種類: 0 = 下限側 [0, w] (w は inf も), 1 = 上限側 (-inf, 0], 2 = 自由, 3 = 固定。
    let mut kind = vec![0u8; n];
    let mut width = vec![inf; n];
    for j in 0..n {
        let (l, u) = (std.lb[j], std.ub[j]);
        if l.is_finite() && u.is_finite() && l == u {
            kind[j] = 3;
            x0[j] = l;
        } else if l.is_finite() {
            x0[j] = l;
            width[j] = u - l;
        } else if u.is_finite() {
            kind[j] = 1;
            x0[j] = u;
        } else {
            kind[j] = 2;
        }
    }
    // `b' = b - A x0`。
    let mut bp = std.b.clone();
    for j in 0..n {
        if x0[j] != 0.0 {
            for &(i, v) in std.cols.col(j) {
                bp[i] -= v * x0[j];
            }
        }
    }
    // 行 `i` のスラック (係数 σ、境界) から `y_i` の列 (符号の向き、費用) を作る。
    let mut cols: Vec<DualCol> = Vec::with_capacity(m + n / 4);
    let mut d_cost: Vec<f64> = Vec::with_capacity(m + n / 4);
    let mut d_lb: Vec<f64> = Vec::with_capacity(m + n / 4);
    let mut d_ub: Vec<f64> = Vec::with_capacity(m + n / 4);
    // 行 `i` の `y` 列の範囲 `[y_start[i], y_start[i+1])`。
    let mut y_start = vec![0usize; m + 1];
    // 初期配置で無限の側に置かれる双対列の数 (費用の符号が無限の境界を好む)。
    let mut n_m_side = 0usize;
    for i in 0..m {
        y_start[i] = cols.len();
        let Some(&(sj, sigma)) = std.rows.row(i).iter().find(|&&(j, _)| j >= n) else {
            return None;
        };
        let (ls, us) = (std.lb[sj], std.ub[sj]);
        // `σ y <= 0` (スラックの下限が有限) の列と `σ y >= 0` (上限が有限) の列。費用は `-b'_i + bound * σ`。
        let mut push = |le_zero: bool, bound: f64| {
            // `σ y <= 0` ⇔ σ > 0 なら y <= 0。
            let y_nonpos = (sigma > 0.0) == le_zero;
            let cost = -bp[i] + bound * sigma;
            cols.push(DualCol::Y(i));
            d_cost.push(cost);
            if y_nonpos {
                d_lb.push(-inf);
                d_ub.push(0.0);
                if cost > 0.0 {
                    n_m_side += 1;
                }
            } else {
                d_lb.push(0.0);
                d_ub.push(inf);
                if cost < 0.0 {
                    n_m_side += 1;
                }
            }
        };
        if ls.is_finite() && us.is_finite() && ls == us {
            // 等式行: `y_i` は自由。
            cols.push(DualCol::Y(i));
            d_cost.push(-bp[i] + ls * sigma);
            d_lb.push(-inf);
            d_ub.push(inf);
            n_m_side += (d_cost[d_cost.len() - 1] != 0.0) as usize;
        } else {
            if ls.is_finite() {
                push(true, ls);
            }
            if us.is_finite() {
                push(false, us);
            }
            // 両側無限 (自由な行) なら `y_i = 0` で列なし。
        }
    }
    y_start[m] = cols.len();
    let n_y = cols.len();
    // 双対 LP の行 = 固定でない構造列。
    let rows_of: Vec<usize> = (0..n).filter(|&j| kind[j] != 3).collect();
    let nd = rows_of.len();
    let mut mu_of = vec![usize::MAX; n];
    for &j in &rows_of {
        if kind[j] == 0 && width[j].is_finite() {
            mu_of[j] = cols.len();
            cols.push(DualCol::Mu(j));
            d_cost.push(width[j]);
            d_lb.push(0.0);
            d_ub.push(inf);
        }
    }
    let n_dcols = cols.len();
    let max_m_side = tunable!("ENOMOTO_T_DUALIZE_MAX_M_SIDE", DUALIZE_MAX_M_SIDE, f64);
    if debug {
        eprintln!("DUALIZE: primal m={m} n={n} -> dual rows={nd} cols={n_dcols} (y={n_y}) m_side={n_m_side}");
    }
    if (n_m_side as f64) > max_m_side * nd as f64 {
        if debug {
            eprintln!("DUALIZE: too many dual columns on the infinite side -> primal solve");
        }
        return None;
    }
    // 双対 LP の行列 (行 = 構造列 `j`)。
    let n_total = n_dcols + nd;
    let mut rows: Vec<Vec<(usize, f64)>> = Vec::with_capacity(nd);
    let mut c_rhs = Vec::with_capacity(nd);
    let mut lb = d_lb;
    let mut ub = d_ub;
    let mut cost = d_cost;
    for (r, &j) in rows_of.iter().enumerate() {
        let col = std.cols.col(j);
        let mut row = Vec::with_capacity(col.len() * 2 + 2);
        for &(i, v) in col {
            for k in y_start[i]..y_start[i + 1] {
                row.push((k, v));
            }
        }
        if mu_of[j] != usize::MAX {
            row.push((mu_of[j], -1.0));
        }
        row.push((n_dcols + r, 1.0));
        debug_assert!(row.windows(2).all(|w| w[0].0 < w[1].0));
        rows.push(row);
        c_rhs.push(std.c[j]);
    }
    for &j in &rows_of {
        cost.push(0.0);
        match kind[j] {
            0 => {
                lb.push(0.0);
                ub.push(inf);
            }
            1 => {
                lb.push(-inf);
                ub.push(0.0);
            }
            _ => {
                lb.push(0.0);
                ub.push(0.0);
            }
        }
    }
    let (rmat, cmat) = super::freeze_std_matrices(&rows, n_total);
    drop(rows);
    let dual = StdForm { n_total, n_rows: nd, c: cost, rows: rmat, cols: cmat, b: c_rhs, lb, ub };

    slope_intercept_dual::request_duals(true);
    let res = slope_intercept_dual::solve_slope_intercept_dual(&dual, opts);
    let duals = slope_intercept_dual::take_duals();
    slope_intercept_dual::request_duals(false);
    let (Some(res), Some((pi, _))) = (res, duals) else {
        if debug {
            eprintln!("DUALIZE: dual solve gave no optimal basis -> primal solve");
        }
        return None;
    };
    if res.status != Status::Optimal {
        if debug {
            eprintln!("DUALIZE: dual status {:?} -> primal solve", res.status);
        }
        return None;
    }
    // `x'_j = -π_j` を元の範囲に収めて戻す。
    let mut x = x0;
    for (r, &j) in rows_of.iter().enumerate() {
        let xp = -pi[r];
        let (lo, hi) = match kind[j] {
            0 => (0.0, width[j]),
            1 => (-inf, 0.0),
            _ => (-inf, inf),
        };
        x[j] += xp.clamp(lo, hi);
    }
    // 元の行の活動度がスラックの範囲に収まるか (`a_i x + σ s = b`、`s = (b - a_i x) / σ`)。
    let mut act = vec![0.0; m];
    for j in 0..n {
        if x[j] != 0.0 {
            for &(i, v) in std.cols.col(j) {
                act[i] += v * x[j];
            }
        }
    }
    let mut worst = 0.0f64;
    for i in 0..m {
        let Some(&(sj, sigma)) = std.rows.row(i).iter().find(|&&(j, _)| j >= n) else {
            continue;
        };
        let s = (std.b[i] - act[i]) / sigma;
        let tol = DUALIZE_FEAS_TOL * (1.0 + std.b[i].abs());
        let viol = (std.lb[sj] - s).max(s - std.ub[sj]).max(0.0);
        if viol > tol {
            worst = worst.max(viol / tol);
        }
    }
    if debug {
        eprintln!("DUALIZE: solved dual in {:.2}s, worst row violation / tol = {worst:.3e}", t0.elapsed().as_secs_f64());
    }
    if worst > 1.0 {
        return None;
    }
    Some(SimplexResult { status: Status::Optimal, x: Some(x) })
}
