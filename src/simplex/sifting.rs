//! 篩い分け法 (sifting、CLP の "Sprint"、`ClpSolve.cpp` の `usePrimalorSprint`)。
//!
//! 列数が行数よりはるかに多い問題 (Mittelmann rail4284: 4,176 行 × 109 万列、scpm1: 5,000 行 × 50 万列) では、
//! 双対単体法の 1 反復の PRICE・比率テストが全列にかかって遅い。そこで列の一部 `W` だけの部分問題を解き、
//! その双対 `y` で残りの列の被約費用 `d_j = c_j - a_j^T y` を求めて、改善しうる列 (`d_j < 0` で下限にある列など)
//! を `W` に加えて解き直すことを、改善しうる列が無くなるまで繰り返す。`W` の外の列は既定の境界値
//! (有限な下限、なければ有限な上限) に固定し、その寄与を右辺へ移す。
//!
//! - `W` には毎回、前回の部分問題の解で既定値から離れた列と基底列 (前回の最適解が新しい部分問題でも実行可能で、
//!   目的関数値が単調に下がる)、自由列、被約費用の小さい順の列を、合計 `max(3m, 3000)` 列程度まで入れる (CLP と同じ)。
//! - 既定値に置いたときに満たせない行には、大きな費用 `P` の人工列 (係数 `±1`、下限 0) を足して、部分問題が
//!   常に実行可能になるようにする (CLP の "costed slacks")。最後まで人工列が正の値で残れば `P` を上げ、
//!   それでも残るなら元の問題が実行不能と考えられるので、篩い分けをやめて全体を普通に解く (状態の判定は通常の
//!   求解に任せる)。
//! - 部分問題の解法は通常の傾き・切片双対二段解法 ([`super::slope_intercept_dual`]) で、毎回最初から解く。
//!   双対は最適な基底を分解し直して求める (`request_duals`)。
//! - 打ち切り (回数の上限、部分問題が最適で終わらない、双対が得られない) のときも全体を普通に解く。
//!
//! 結果の `x` は全体の問題の最適解 (`W` の外の列は既定値) なので、呼び出し側は普通の求解の結果と同じに扱える。

use super::{slope_intercept_dual, SimplexResult, Status, StdForm};
use crate::params::simplex::{SIFTING_MIN_COLS, SIFTING_MIN_RATIO};

/// 部分問題の大きさの係数 (`max(SIFT_SIZE_FACTOR * m, SIFT_SIZE_MIN)`、`ENOMOTO_T_SIFT_SIZE_FACTOR`)。CLP は 3 だが、
/// rail4284 では 2 / 3 / 5 で 150 / 176 / 262 s (2 は反復が軽い)。最小 3,000 は CLP と同じ。
const SIFT_SIZE_FACTOR: f64 = 2.0;
const SIFT_SIZE_MIN: usize = 3000;
/// 部分問題を解く回数の上限。
const SIFT_MAX_PASSES: usize = 100;
/// 被約費用の判定の許容誤差 (`|c_j|` に対する相対を足す)。
const SIFT_DUAL_TOL: f64 = 1e-7;
/// 温めた主単体法の結果に許す上下限違反 (`|境界|` に対する相対を足す)。
const SIFT_PRIMAL_TOL: f64 = 1e-7;
/// 人工列の値を 0 とみなす許容誤差。
const SIFT_ART_TOL: f64 = 1e-9;

/// 篩い分け法を使うべき大きさか: 構造列が `SIFTING_MIN_COLS` 以上で、行数の `SIFTING_MIN_RATIO` 倍以上
/// (`ENOMOTO_T_SIFTING_MIN_COLS`・`ENOMOTO_T_SIFTING_MIN_RATIO`、どちらかが 0 なら使わない)。
/// 自由列が多い (行数の 1 割を超える) 問題は CLP と同じく対象外 (自由列は常に部分問題に入る)。
fn applicable(std: &StdForm) -> bool {
    let min_cols = tunable!("ENOMOTO_T_SIFTING_MIN_COLS", SIFTING_MIN_COLS, usize);
    let ratio = tunable!("ENOMOTO_T_SIFTING_MIN_RATIO", SIFTING_MIN_RATIO, f64);
    let m = std.n_rows;
    let n = std.n_total - m;
    if min_cols == 0 || ratio <= 0.0 || n < min_cols || (n as f64) < ratio * m as f64 {
        return false;
    }
    // 行が長い (1 行の平均非零数が `SIFTING_MIN_ROW_NNZ` 以上) 問題だけ: 双対単体法の 1 反復の PRICE が `rho` の
    // 非零行の長さに比例して重い問題 (rail4284 は平均 2,700)。行の短い問題 (osa-60 は 136) は全体を双対単体法で
    // 解くほうが速い (osa-60: 2.8 s、篩い分けでは 7.7 s)。
    let min_row_nnz = tunable!("ENOMOTO_T_SIFTING_MIN_ROW_NNZ", crate::params::simplex::SIFTING_MIN_ROW_NNZ, f64);
    let nnz_struct: usize = (0..n).map(|j| std.cols.col(j).len()).sum();
    if (nnz_struct as f64) < min_row_nnz * m as f64 {
        return false;
    }
    let n_free = (0..n).filter(|&j| std.lb[j] == f64::NEG_INFINITY && std.ub[j] == f64::INFINITY).count();
    if n_free * 10 > m {
        return false;
    }
    if env_str!("ENOMOTO_DEBUG_SIFTING").is_some() {
        eprintln!("SIFTING: m={m} n={n} nnz={nnz_struct} n_free={n_free}");
    }
    true
}

/// 列 `j` を部分問題の外に置くときの値 (有限な下限、なければ有限な上限)。自由列は `None`。
fn default_value(std: &StdForm, j: usize) -> Option<f64> {
    if std.lb[j].is_finite() {
        Some(std.lb[j])
    } else if std.ub[j].is_finite() {
        Some(std.ub[j])
    } else {
        None
    }
}

/// 篩い分け法で解く。対象外、または途中で諦めたときは `None` (呼び出し側が普通に解く)。
pub(super) fn solve(std: &StdForm, opts: &crate::types::LpOptions) -> Option<SimplexResult> {
    if !applicable(std) {
        return None;
    }
    let debug = env_str!("ENOMOTO_DEBUG_SIFTING").is_some();
    let m = std.n_rows;
    let n = std.n_total - m;
    let target = ((tunable!("ENOMOTO_T_SIFT_SIZE_FACTOR", SIFT_SIZE_FACTOR, f64) * m as f64) as usize).max(SIFT_SIZE_MIN).min(n);
    let t0 = std::time::Instant::now();

    // 既定値に置いた列の行活動度と、行 `i` のスラックの範囲 (`a_i x + σ s_i = b_i`)。
    let mut xdef = vec![0.0; n];
    let mut always = vec![false; n];
    for j in 0..n {
        match default_value(std, j) {
            Some(v) => xdef[j] = v,
            None => always[j] = true,
        }
    }
    let mut act = vec![0.0; m];
    for j in 0..n {
        if xdef[j] != 0.0 {
            for &(i, v) in std.cols.col(j) {
                act[i] += v * xdef[j];
            }
        }
    }
    // 人工列: 行 `i` が既定値で満たせない向きに 1 本ずつ (`(行, 係数)`)。
    let mut arts: Vec<(usize, f64)> = Vec::new();
    for i in 0..m {
        // 行 `i` のスラック列とその係数。
        let Some(&(sj, sv)) = std.rows.row(i).iter().find(|&&(j, _)| j >= n) else {
            continue;
        };
        // スラックに要る値 `s = (b_i - act_i) / sv`。
        let s = (std.b[i] - act[i]) / sv;
        let tol = 1e-9 * (1.0 + std.b[i].abs());
        // 人工列 `σ art` (art >= 0) を足すと `s = (b_i - act_i - σ art) / sv`。スラックを増やしたいなら
        // `σ = -sign(sv)`、減らしたいなら `σ = sign(sv)`。
        if s < std.lb[sj] - tol {
            arts.push((i, -sv.signum()));
        } else if s > std.ub[sj] + tol {
            arts.push((i, sv.signum()));
        }
    }
    let cmax = (0..n).fold(0.0f64, |a, j| a.max(std.c[j].abs()));
    let mut penalty = (1e5f64).max(1e3 * cmax.max(1.0));

    // 部分問題に入れる列 (構造列の番号、昇順)。最初は自由列だけ (CLP と同じく人工列から始める)。
    let mut in_w = always.clone();
    let mut x_full = xdef.clone();
    let mut d = vec![0.0; n];
    let mut last_art_sum = f64::INFINITY;
    // 前回の部分問題の最適基底 (構造列・人工列・スラックごとの印)。次の部分問題を主単体法で温めて解くのに使う。
    let mut prev_basic: Option<(Vec<bool>, Vec<bool>, Vec<bool>)> = None;
    let warm_on = tunable!("ENOMOTO_T_SIFT_WARM", 1u8, u8) != 0;
    for pass in 0..SIFT_MAX_PASSES {
        let w: Vec<usize> = (0..n).filter(|&j| in_w[j]).collect();
        let sub = build_sub(std, &w, &arts, &xdef, penalty);
        let n_sub = w.len() + arts.len();
        // 前回の基底を新しい部分問題の列番号で表す (構造列は `w` での位置、人工列・スラックは並びが同じ)。
        let warm_basis: Option<Vec<usize>> = prev_basic.as_ref().and_then(|(bs, ba, bl)| {
            let mut basis = Vec::with_capacity(m);
            for (k, &j) in w.iter().enumerate() {
                if bs[j] {
                    basis.push(k);
                }
            }
            for (k, &b) in ba.iter().enumerate() {
                if b {
                    basis.push(w.len() + k);
                }
            }
            for (i, &b) in bl.iter().enumerate() {
                if b {
                    basis.push(n_sub + i);
                }
            }
            (basis.len() == m).then_some(basis)
        });
        let mut solved = None;
        let it0 = super::prof_phases::RUN_PHASE_ITERS.load(std::sync::atomic::Ordering::Relaxed);
        let tp = std::time::Instant::now();
        let warm_tried = warm_basis.is_some() && warm_on;
        if let (Some(basis), true) = (warm_basis, warm_on) {
            // 非基底列の値: 構造列は前回の値、人工列は 0、スラックは前回の解から求めた値 (どちらの境界かの判定用)。
            let mut x0 = vec![0.0; sub.n_total];
            for (k, &j) in w.iter().enumerate() {
                x0[k] = x_full[j];
            }
            let mut act = vec![0.0; m];
            for j in 0..n {
                if x_full[j] != 0.0 {
                    for &(i, v) in std.cols.col(j) {
                        act[i] += v * x_full[j];
                    }
                }
            }
            for i in 0..m {
                if let Some(&(sj, sv)) = std.rows.row(i).iter().find(|&&(j, _)| j >= n) {
                    x0[n_sub + (sj - n)] = (std.b[i] - act[i]) / sv;
                }
            }
            solved = solve_warm(&sub, basis, &x0, opts);
            if solved.is_none() && debug {
                eprintln!("SIFTING: pass {pass}: warm primal failed -> cold dual");
            }
        }
        if solved.is_none() {
            slope_intercept_dual::request_duals(true);
            let res = slope_intercept_dual::solve_slope_intercept_dual(&sub, opts);
            let duals = slope_intercept_dual::take_duals();
            slope_intercept_dual::request_duals(false);
            if let (Some(res), Some((y, bpos))) = (res, duals) {
                if res.status == Status::Optimal {
                    solved = res.x.map(|x| (x, y, bpos));
                } else if debug {
                    eprintln!("SIFTING: pass {pass}: sub-problem {:?} -> full solve", res.status);
                }
            }
        }
        if debug {
            let it = super::prof_phases::RUN_PHASE_ITERS.load(std::sync::atomic::Ordering::Relaxed) - it0;
            eprintln!("SIFTING: pass {pass}: sub-solve {} {:.2}s primal_iters={it}", if warm_tried && solved.is_some() { "warm" } else { "cold" }, tp.elapsed().as_secs_f64());
        }
        let Some((xs, y, bpos)) = solved else {
            if debug {
                eprintln!("SIFTING: pass {pass}: sub-problem gave no optimal basis -> full solve");
            }
            return None;
        };
        // 基底の印 (元の構造列・人工列・スラック)。
        let mut bs = vec![false; n];
        let mut ba = vec![false; arts.len()];
        let mut bl = vec![false; m];
        if bpos.len() == sub.n_total {
            for (c, p) in bpos.iter().enumerate() {
                if p.is_some() {
                    if c < w.len() {
                        bs[w[c]] = true;
                    } else if c < n_sub {
                        ba[c - w.len()] = true;
                    } else {
                        bl[c - n_sub] = true;
                    }
                }
            }
            prev_basic = Some((bs, ba, bl));
        } else {
            prev_basic = None;
        }
        x_full.copy_from_slice(&xdef);
        for (k, &j) in w.iter().enumerate() {
            x_full[j] = xs[k];
        }
        let art_sum: f64 = (0..arts.len()).map(|k| xs[w.len() + k]).sum();
        if !arts.is_empty() && art_sum <= SIFT_ART_TOL * (1.0 + m as f64) {
            // 人工列がすべて 0 になった: 以後は人工列なしで解く (値 0 で基底に残った人工列の行の双対が罰金の
            // 大きさになり、その行の列が被約費用で選ばれ続けるのを防ぐ。osa-60 で収束しなかった)。基底にある
            // 人工列は同じ行のスラック (向きだけ違う同じ単位列) で置き換える。同じ列集合で解き直す。
            if let Some((_, ba, bl)) = prev_basic.as_mut() {
                for (k, &(i, _)) in arts.iter().enumerate() {
                    if ba[k] {
                        bl[i] = true;
                    }
                }
                ba.clear();
            }
            arts.clear();
            if debug {
                eprintln!("SIFTING: pass {pass}: artificials reached zero -> dropped");
            }
            continue;
        }
        // 全列の被約費用。
        let mut n_neg = 0usize;
        let mut sum_neg = 0.0f64;
        for j in 0..n {
            let mut dj = std.c[j];
            for &(i, v) in std.cols.col(j) {
                dj -= v * y[i];
            }
            d[j] = dj;
            if !in_w[j] {
                let tol = SIFT_DUAL_TOL * (1.0 + std.c[j].abs());
                // 既定値が下限なら増やせば改善 (`d < 0`)、上限なら減らせば改善 (`d > 0`)。
                let bad = if xdef[j] == std.lb[j] { dj < -tol && std.ub[j] > std.lb[j] } else { dj > tol };
                if bad {
                    n_neg += 1;
                    sum_neg += dj.abs();
                }
            }
        }
        if debug {
            let obj: f64 = (0..n).map(|j| std.c[j] * x_full[j]).sum();
            eprintln!(
                "SIFTING: pass {pass}: t={:.2}s |W|={} arts={} penalty={penalty:.3e} obj={obj:.10e} art_sum={art_sum:.3e} priced_out={n_neg} sum={sum_neg:.3e}",
                t0.elapsed().as_secs_f64(),
                w.len(),
                arts.len()
            );
        }
        if n_neg == 0 {
            if art_sum <= SIFT_ART_TOL * (1.0 + m as f64) {
                return Some(SimplexResult { status: Status::Optimal, x: Some(x_full) });
            }
            // 人工列が残る: 罰金を上げて続ける。十分大きくしても残れば実行不能とみて普通に解く。
            if penalty >= 1e12 * cmax.max(1.0) {
                if debug {
                    eprintln!("SIFTING: artificials stay positive -> full solve");
                }
                return None;
            }
            penalty *= 100.0;
            continue;
        }
        if art_sum > SIFT_ART_TOL && pass >= 5 && art_sum >= last_art_sum {
            // 人工列が減らない: CLP と同じく罰金を 1.5 倍にする。
            penalty *= 1.5;
        }
        last_art_sum = art_sum;
        // 次の部分問題の列: 既定値から離れた列・自由列 + 被約費用の順 (改善しうる列が先)。
        let mut next = always.clone();
        let mut n_keep = 0usize;
        for j in 0..n {
            if !next[j] && (x_full[j] != xdef[j] || prev_basic.as_ref().is_some_and(|b| b.0[j])) {
                next[j] = true;
            }
            n_keep += next[j] as usize;
        }
        let mut cand: Vec<(f64, usize)> = Vec::with_capacity(n / 4);
        for j in 0..n {
            if next[j] {
                continue;
            }
            let key = if xdef[j] == std.lb[j] { d[j] } else { -d[j] };
            if std.ub[j] == std.lb[j] {
                continue;
            }
            cand.push((key, j));
        }
        let room = target.saturating_sub(n_keep).max(n_neg.min(target));
        if cand.len() > room {
            cand.select_nth_unstable_by(room, |a, b| a.0.total_cmp(&b.0));
            cand.truncate(room);
        }
        for &(_, j) in &cand {
            next[j] = true;
        }
        in_w = next;
    }
    if debug {
        eprintln!("SIFTING: pass limit -> full solve");
    }
    None
}

/// 部分問題 `sub` を、主実行可能な基底 `basis` (列番号、`m` 本) から有界変数主単体法 (第 2 段階、
/// [`super::run_phase2_incremental`]) で解く。非基底列は前回の値 `x0` に近い側の有限な境界 (自由なら 0) に置く。
/// 最適な `(x, y, basis_pos)` を返し、失敗 (特異、最適で終わらない、境界違反が残る) なら `None`。
fn solve_warm(sub: &StdForm, basis: Vec<usize>, x0: &[f64], _opts: &crate::types::LpOptions) -> Option<(Vec<f64>, Vec<f64>, Vec<Option<usize>>)> {
    use super::{NbStatus, Tableau};
    let m = sub.n_rows;
    let nt = sub.n_total;
    let mut basis_pos = vec![None; nt];
    for (p, &c) in basis.iter().enumerate() {
        basis_pos[c] = Some(p);
    }
    let mut nb_status = vec![None; nt];
    let mut x = vec![0.0; nt];
    for c in 0..nt {
        if basis_pos[c].is_some() {
            continue;
        }
        // 前回の値 `x0` に近い側の有限な境界に置く。
        let near_ub = sub.ub[c].is_finite() && (!sub.lb[c].is_finite() || (x0[c] - sub.ub[c]).abs() < (x0[c] - sub.lb[c]).abs());
        let (st, v) = if near_ub {
            (NbStatus::Upper, sub.ub[c])
        } else if sub.lb[c].is_finite() {
            (NbStatus::Lower, sub.lb[c])
        } else if sub.ub[c].is_finite() {
            (NbStatus::Upper, sub.ub[c])
        } else {
            (NbStatus::Zero, 0.0)
        };
        nb_status[c] = Some(st);
        x[c] = v;
    }
    let mut t = Tableau { std: sub, basis, basis_pos, nb_status, x };
    let mut lu = super::try_refactorize(sub, &t, None)?;
    t.recompute_basics(&lu);
    let mut stall = super::PrimalStallState::new();
    let debug = env_str!("ENOMOTO_DEBUG_SIFTING").is_some();
    let Some(st) = super::run_phase2_incremental(sub, &mut t, &mut lu, &mut stall) else {
        if debug {
            eprintln!("SIFTING: warm primal: singular basis");
        }
        return None;
    };
    if st != Status::Optimal {
        if debug {
            eprintln!("SIFTING: warm primal: status {st:?}");
        }
        return None;
    }
    // 非基底を境界値に戻して `x_B` を作り直し (EXPAND の行き過ぎを残さない)、境界違反が無いことを確かめる。
    t.expand_reset_nonbasics();
    let lu = super::try_refactorize(sub, &t, None)?;
    t.recompute_basics(&lu);
    for c in 0..nt {
        let v = t.x[c];
        if v < sub.lb[c] - SIFT_PRIMAL_TOL * (1.0 + sub.lb[c].abs()) || v > sub.ub[c] + SIFT_PRIMAL_TOL * (1.0 + sub.ub[c].abs()) {
            if debug {
                eprintln!("SIFTING: warm primal: bound violation col {c} x={v:.3e} lb={:.3e} ub={:.3e}", sub.lb[c], sub.ub[c]);
            }
            return None;
        }
    }
    let mut c_b = vec![0.0; m];
    for (p, &c) in t.basis.iter().enumerate() {
        c_b[p] = sub.c[c];
    }
    let mut y = vec![0.0; m];
    let mut scratch = vec![0.0; m];
    lu.solve_transpose_into(&c_b, &mut scratch, &mut y);
    Some((t.x, y, t.basis_pos))
}

/// 構造列 `w` と人工列 `arts` だけの部分問題を作る。`w` の外の構造列は `xdef` に固定し、寄与を右辺へ移す。
/// 列の並びは `w` (昇順)、人工列、スラック (元と同じ並び)。
fn build_sub(std: &StdForm, w: &[usize], arts: &[(usize, f64)], xdef: &[f64], penalty: f64) -> StdForm {
    let m = std.n_rows;
    let n = std.n_total - m;
    let n_struct = w.len() + arts.len();
    let n_total = n_struct + m;
    let mut new_of = vec![usize::MAX; n];
    for (k, &j) in w.iter().enumerate() {
        new_of[j] = k;
    }
    let mut b = std.b.clone();
    let mut rows: Vec<Vec<(usize, f64)>> = vec![Vec::new(); m];
    for j in 0..n {
        let nj = new_of[j];
        if nj == usize::MAX {
            if xdef[j] != 0.0 {
                for &(i, v) in std.cols.col(j) {
                    b[i] -= v * xdef[j];
                }
            }
        }
    }
    for (k, &j) in w.iter().enumerate() {
        for &(i, v) in std.cols.col(j) {
            rows[i].push((k, v));
        }
    }
    for (k, &(i, s)) in arts.iter().enumerate() {
        rows[i].push((w.len() + k, s));
    }
    let mut c = Vec::with_capacity(n_total);
    let mut lb = Vec::with_capacity(n_total);
    let mut ub = Vec::with_capacity(n_total);
    for &j in w {
        c.push(std.c[j]);
        lb.push(std.lb[j]);
        ub.push(std.ub[j]);
    }
    for _ in arts {
        c.push(penalty);
        lb.push(0.0);
        ub.push(f64::INFINITY);
    }
    for i in 0..m {
        let sj = n + i;
        c.push(std.c[sj]);
        lb.push(std.lb[sj]);
        ub.push(std.ub[sj]);
        for &(j, v) in std.rows.row(i) {
            if j >= n {
                rows[i].push((n_struct + (j - n), v));
            }
        }
    }
    let (rows, cols) = super::freeze_std_matrices(&rows, n_total);
    StdForm { n_total, n_rows: m, c, rows, cols, b, lb, ub }
}
