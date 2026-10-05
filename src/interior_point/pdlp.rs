//! PDLP (Applegate et al. 2021、restarted PDHG) による LP の一次法:
//!
//!   minimize   c^T x
//!   subject to A x = b,   l <= x <= u   (l, u は ±inf でもよい)
//!
//! 主な用途は内点法のウォームスタート (`simplex::crossover`): 低い精度 (既定 1e-4) まで解いた
//! `(x, y)` を近接内点法の近接中心 (と、指定があれば初期点) にする。行列の積しか使わないので、
//! 分解が重い問題・内点法が数値的に行き詰まる問題でも 1 反復は安い。
//!
//! 実装は PDLP の主要部分に従う:
//!
//! - 前処理: Ruiz の平衡化 (10 回) の後に Pock–Chambolle (α = 1)。`Â = D_r A D_c`。
//! - PDHG: `x⁺ = proj_[l,u](x - (η/ω)(c - Âᵀy))`、`y⁺ = y + ηω (b - Â(2x⁺ - x))`。
//! - 適応的な歩幅: `η ≤ ‖Δz‖²_ω / (2 |Δyᵀ Â Δx|)` を満たすまで縮める (PDLP の式)。
//! - 再始動: 64 反復ごとに、今の点と (η で重み付けした) 平均の KKT 誤差の小さい方を候補にし、
//!   十分減った (0.2 倍)・減ったが停滞 (0.8 倍)・長すぎる (総反復の 0.36) のどれかで再始動する
//!   (cuPDLP の KKT 誤差による判定)。
//! - 主の重み `ω`: 再始動のたびに `exp(½ log(Δy/Δx) + ½ log ω)` で更新する。
//!
//! 停止は元の (前処理前の) 問題の相対誤差 `‖Ax - b‖₂ ≤ ε(1 + ‖b‖₂)`、
//! `‖c - Aᵀy - λ‖₂ ≤ ε(1 + ‖c‖₂)`、`|pobj - dobj| ≤ ε(1 + |pobj| + |dobj|)` で判定する
//! (`λ` は被約費用を境界の向きに射影したもの)。

use rayon::prelude::*;
use std::time::Instant;

use super::kkt::FaerCsr;
use crate::types::Status;

/// 並列ループの 1 タスクあたりの最小の長さ。
const PAR_MIN_LEN: usize = 2048;
/// 再始動・停止の判定の間隔 (反復)。
const CHECK_EVERY: usize = 64;
const RUIZ_ITERS: usize = 10;
const BETA_SUFFICIENT: f64 = 0.2;
const BETA_NECESSARY: f64 = 0.8;
const BETA_ARTIFICIAL: f64 = 0.36;
const PRIMAL_WEIGHT_SMOOTHING: f64 = 0.5;

/// PDLP の設定。
pub struct PdlpOptions {
    /// 停止の相対許容誤差。
    pub eps: f64,
    /// 反復の上限。
    pub max_iters: usize,
    /// 時間の上限 (秒)。
    pub time_limit: f64,
}

/// PDLP の結果。
pub struct PdlpResult {
    /// `Optimal` (許容誤差内) か `NotSolved` (上限に達した。最良の点を返す)。
    pub status: Status,
    /// 主解 (`l <= x <= u`)。
    pub x: Vec<f64>,
    /// 等式の双対 (LP の符号: 被約費用は `c - A^T y`)。
    pub y: Vec<f64>,
    /// 反復回数。
    pub iters: usize,
    /// 相対的な主残差・双対残差・ギャップ (`ε` の分母で割ったもの)。
    pub rel: (f64, f64, f64),
}

/// 行圧縮の疎行列 (自前の配列で持ち、行ごとの積を決まった順で足す)。
struct Csr {
    ptr: Vec<usize>,
    idx: Vec<usize>,
    val: Vec<f64>,
    ncols: usize,
}

impl Csr {
    fn nrows(&self) -> usize {
        self.ptr.len() - 1
    }

    /// `out = M v`。
    fn mul(&self, v: &[f64], out: &mut [f64]) {
        out.par_iter_mut().enumerate().with_min_len(PAR_MIN_LEN).for_each(|(i, o)| {
            let mut s = 0.0;
            for k in self.ptr[i]..self.ptr[i + 1] {
                s += self.val[k] * v[self.idx[k]];
            }
            *o = s;
        });
    }

    fn transpose(&self) -> Csr {
        let m = self.nrows();
        let n = self.ncols;
        let mut cnt = vec![0usize; n + 1];
        for &j in &self.idx {
            cnt[j + 1] += 1;
        }
        for j in 0..n {
            cnt[j + 1] += cnt[j];
        }
        let mut next = cnt.clone();
        let nnz = self.idx.len();
        let mut idx = vec![0usize; nnz];
        let mut val = vec![0.0; nnz];
        for i in 0..m {
            for k in self.ptr[i]..self.ptr[i + 1] {
                let j = self.idx[k];
                idx[next[j]] = i;
                val[next[j]] = self.val[k];
                next[j] += 1;
            }
        }
        Csr { ptr: cnt, idx, val, ncols: m }
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn norm2(a: &[f64]) -> f64 {
    dot(a, a).sqrt()
}

/// 前処理後の問題とその縮尺。
struct Scaled {
    a: Csr,
    at: Csr,
    b: Vec<f64>,
    c: Vec<f64>,
    l: Vec<f64>,
    u: Vec<f64>,
    /// `x = dc ∘ x̂`、`y = dr ∘ ŷ`。
    dr: Vec<f64>,
    dc: Vec<f64>,
}

fn scale(a: &FaerCsr, b: &[f64], c: &[f64], l: &[f64], u: &[f64]) -> Scaled {
    let m = a.nrows();
    let n = a.ncols();
    let ar = a.as_ref();
    let mut ptr = vec![0usize; m + 1];
    let mut idx = Vec::with_capacity(ar.compute_nnz());
    let mut val = Vec::with_capacity(ar.compute_nnz());
    for i in 0..m {
        for (j, &v) in ar.col_indices_of_row(i).zip(ar.values_of_row(i)) {
            if v != 0.0 {
                idx.push(j);
                val.push(v);
            }
        }
        ptr[i + 1] = idx.len();
    }
    let mut mat = Csr { ptr, idx, val, ncols: n };
    let mut dr = vec![1.0; m];
    let mut dc = vec![1.0; n];
    let apply = |mat: &mut Csr, rs: &[f64], cs: &[f64]| {
        for i in 0..m {
            for k in mat.ptr[i]..mat.ptr[i + 1] {
                mat.val[k] *= rs[i] * cs[mat.idx[k]];
            }
        }
    };
    // Ruiz: 行・列の最大絶対値を 1 に近づける。
    for _ in 0..RUIZ_ITERS {
        let mut rmax = vec![0.0f64; m];
        let mut cmax = vec![0.0f64; n];
        for i in 0..m {
            for k in mat.ptr[i]..mat.ptr[i + 1] {
                let v = mat.val[k].abs();
                rmax[i] = rmax[i].max(v);
                cmax[mat.idx[k]] = cmax[mat.idx[k]].max(v);
            }
        }
        let rs: Vec<f64> = rmax.iter().map(|&v| if v > 0.0 { 1.0 / v.sqrt() } else { 1.0 }).collect();
        let cs: Vec<f64> = cmax.iter().map(|&v| if v > 0.0 { 1.0 / v.sqrt() } else { 1.0 }).collect();
        apply(&mut mat, &rs, &cs);
        for i in 0..m {
            dr[i] *= rs[i];
        }
        for j in 0..n {
            dc[j] *= cs[j];
        }
    }
    // Pock–Chambolle (α = 1): 行・列の絶対値の和の平方根で割る。
    {
        let mut rsum = vec![0.0f64; m];
        let mut csum = vec![0.0f64; n];
        for i in 0..m {
            for k in mat.ptr[i]..mat.ptr[i + 1] {
                let v = mat.val[k].abs();
                rsum[i] += v;
                csum[mat.idx[k]] += v;
            }
        }
        let rs: Vec<f64> = rsum.iter().map(|&v| if v > 0.0 { 1.0 / v.sqrt() } else { 1.0 }).collect();
        let cs: Vec<f64> = csum.iter().map(|&v| if v > 0.0 { 1.0 / v.sqrt() } else { 1.0 }).collect();
        apply(&mut mat, &rs, &cs);
        for i in 0..m {
            dr[i] *= rs[i];
        }
        for j in 0..n {
            dc[j] *= cs[j];
        }
    }
    let at = mat.transpose();
    Scaled {
        b: (0..m).map(|i| b[i] * dr[i]).collect(),
        c: (0..n).map(|j| c[j] * dc[j]).collect(),
        l: (0..n).map(|j| l[j] / dc[j]).collect(),
        u: (0..n).map(|j| u[j] / dc[j]).collect(),
        a: mat,
        at,
        dr,
        dc,
    }
}

/// 点 `(x, y)` (前処理後) の誤差。`ax = Âx`、`aty = Âᵀy`。
struct Err {
    /// 前処理後の主残差・双対残差の 2 ノルムとギャップ (再始動の KKT 誤差用)。
    p: f64,
    d: f64,
    gap: f64,
    /// 元の問題の相対誤差 (停止判定用)。
    rel: (f64, f64, f64),
}

fn evaluate(s: &Scaled, x: &[f64], y: &[f64], ax: &[f64], aty: &[f64], nb: f64, nc: f64) -> Err {
    let m = s.b.len();
    let n = s.c.len();
    // 主残差 (前処理後と元の問題: r = D_r^{-1} r̂)。
    let (mut p2, mut po2) = (0.0, 0.0);
    for i in 0..m {
        let r = ax[i] - s.b[i];
        p2 += r * r;
        let ro = r / s.dr[i];
        po2 += ro * ro;
    }
    // 双対残差: rc = ĉ - Âᵀy を境界の向きに射影した λ との差。
    let (mut d2, mut do2) = (0.0, 0.0);
    let mut dobj = dot(&s.b, y);
    for j in 0..n {
        let rc = s.c[j] - aty[j];
        let lam = if (rc > 0.0 && s.l[j].is_finite()) || (rc < 0.0 && s.u[j].is_finite()) { rc } else { 0.0 };
        let r = rc - lam;
        d2 += r * r;
        let ro = r / s.dc[j];
        do2 += ro * ro;
        if lam > 0.0 {
            dobj += s.l[j] * lam;
        } else if lam < 0.0 {
            dobj += s.u[j] * lam;
        }
    }
    let pobj = dot(&s.c, x);
    let gap = (pobj - dobj).abs();
    Err {
        p: p2.sqrt(),
        d: d2.sqrt(),
        gap,
        rel: (po2.sqrt() / (1.0 + nb), do2.sqrt() / (1.0 + nc), gap / (1.0 + pobj.abs() + dobj.abs())),
    }
}

fn kkt(e: &Err, w: f64) -> f64 {
    (w * w * e.p * e.p + e.d * e.d / (w * w) + e.gap * e.gap).sqrt()
}

/// `min c^T x, A x = b, l <= x <= u` を PDLP で解く。
pub fn solve_pdlp(a: &FaerCsr, b: &[f64], c: &[f64], l: &[f64], u: &[f64], opts: &PdlpOptions) -> PdlpResult {
    let t0 = Instant::now();
    let debug = env_str!("ENOMOTO_DEBUG_PDLP").is_some();
    let m = a.nrows();
    let n = a.ncols();
    let s = scale(a, b, c, l, u);
    let nb = norm2(b);
    let nc = norm2(c);
    let proj = |j: usize, v: f64| v.max(s.l[j]).min(s.u[j]);

    let mut x: Vec<f64> = (0..n).map(|j| proj(j, 0.0)).collect();
    let mut y = vec![0.0; m];
    let mut ax = vec![0.0; m];
    let mut aty = vec![0.0; n];
    s.a.mul(&x, &mut ax);
    let amax = s.a.val.iter().fold(0.0f64, |mm, v| mm.max(v.abs()));
    let mut eta = if amax > 0.0 { 1.0 / amax } else { 1.0 };
    let (nbs, ncs) = (norm2(&s.b), norm2(&s.c));
    let mut w = if nbs > 1e-10 && ncs > 1e-10 { ncs / nbs } else { 1.0 };

    let mut x_new = vec![0.0; n];
    let mut y_new = vec![0.0; m];
    let mut ax_new = vec![0.0; m];
    let mut aty_new = vec![0.0; n];
    // 平均 (η で重み付け) と再始動の基準点。
    let mut x_sum = vec![0.0; n];
    let mut y_sum = vec![0.0; m];
    let mut w_sum = 0.0;
    let mut x_avg = vec![0.0; n];
    let mut y_avg = vec![0.0; m];
    let mut ax_avg = vec![0.0; m];
    let mut aty_avg = vec![0.0; n];
    let mut x_last = x.clone();
    let mut y_last = y.clone();
    let e0 = evaluate(&s, &x, &y, &ax, &aty, nb, nc);
    let mut err_last = kkt(&e0, w);
    let mut err_prev_cand = f64::INFINITY;
    let mut since_restart = 0usize;
    let mut total = 0usize;
    let mut best: (f64, Vec<f64>, Vec<f64>, (f64, f64, f64)) = (f64::INFINITY, x.clone(), y.clone(), e0.rel);
    let mut status = Status::NotSolved;
    let mut restarts = 0usize;

    'outer: while total < opts.max_iters {
        // ---- 1 反復 (歩幅が受理されるまで縮める) ----
        loop {
            let tau = eta / w;
            let sig = eta * w;
            x_new.par_iter_mut().enumerate().with_min_len(PAR_MIN_LEN).for_each(|(j, v)| {
                *v = proj(j, x[j] - tau * (s.c[j] - aty[j]));
            });
            s.a.mul(&x_new, &mut ax_new);
            y_new.par_iter_mut().enumerate().with_min_len(PAR_MIN_LEN).for_each(|(i, v)| {
                *v = y[i] + sig * (s.b[i] - (2.0 * ax_new[i] - ax[i]));
            });
            s.at.mul(&y_new, &mut aty_new);
            let mut dx2 = 0.0;
            for j in 0..n {
                let d = x_new[j] - x[j];
                dx2 += d * d;
            }
            let mut dy2 = 0.0;
            let mut inter = 0.0;
            for i in 0..m {
                let d = y_new[i] - y[i];
                dy2 += d * d;
                inter += d * (ax_new[i] - ax[i]);
            }
            total += 1;
            let k = total as f64;
            let lim = if inter.abs() > 0.0 { (w * dx2 + dy2 / w) / (2.0 * inter.abs()) } else { f64::INFINITY };
            let eta_next = ((1.0 - (k + 1.0).powf(-0.3)) * lim).min((1.0 + (k + 1.0).powf(-0.6)) * eta);
            if eta <= lim {
                // 受理: 平均に足して進める。
                for j in 0..n {
                    x_sum[j] += eta * x_new[j];
                }
                for i in 0..m {
                    y_sum[i] += eta * y_new[i];
                }
                w_sum += eta;
                std::mem::swap(&mut x, &mut x_new);
                std::mem::swap(&mut y, &mut y_new);
                std::mem::swap(&mut ax, &mut ax_new);
                std::mem::swap(&mut aty, &mut aty_new);
                eta = if eta_next.is_finite() { eta_next } else { eta };
                break;
            }
            eta = eta_next;
            if total >= opts.max_iters || !eta.is_finite() || eta <= 0.0 {
                break 'outer;
            }
        }
        since_restart += 1;
        if since_restart % CHECK_EVERY != 0 {
            continue;
        }
        if crate::cancel::is_cancelled() || t0.elapsed().as_secs_f64() > opts.time_limit {
            break;
        }
        // ---- 再始動・停止の判定 ----
        let ec = evaluate(&s, &x, &y, &ax, &aty, nb, nc);
        for j in 0..n {
            x_avg[j] = x_sum[j] / w_sum;
        }
        for i in 0..m {
            y_avg[i] = y_sum[i] / w_sum;
        }
        s.a.mul(&x_avg, &mut ax_avg);
        s.at.mul(&y_avg, &mut aty_avg);
        let ea = evaluate(&s, &x_avg, &y_avg, &ax_avg, &aty_avg, nb, nc);
        for (e, xx, yy) in [(&ec, &x, &y), (&ea, &x_avg, &y_avg)] {
            let worst = e.rel.0.max(e.rel.1).max(e.rel.2);
            if worst < best.0 {
                best = (worst, xx.clone(), yy.clone(), e.rel);
            }
        }
        if debug && (since_restart % (CHECK_EVERY * 16) == 0 || best.0 <= opts.eps) {
            eprintln!(
                "PDLP it={total} t={:.2}s rel_cur=({:.1e},{:.1e},{:.1e}) rel_avg=({:.1e},{:.1e},{:.1e}) eta={eta:.2e} w={w:.2e} restarts={restarts}",
                t0.elapsed().as_secs_f64(),
                ec.rel.0, ec.rel.1, ec.rel.2, ea.rel.0, ea.rel.1, ea.rel.2
            );
        }
        if best.0 <= opts.eps {
            status = Status::Optimal;
            break;
        }
        let (kc, ka) = (kkt(&ec, w), kkt(&ea, w));
        let use_avg = ka < kc;
        let err_cand = kc.min(ka);
        let restart = err_cand <= BETA_SUFFICIENT * err_last
            || (err_cand <= BETA_NECESSARY * err_last && err_cand > err_prev_cand)
            || (since_restart as f64) >= BETA_ARTIFICIAL * total as f64;
        err_prev_cand = err_cand;
        if restart {
            if use_avg {
                x.copy_from_slice(&x_avg);
                y.copy_from_slice(&y_avg);
                ax.copy_from_slice(&ax_avg);
                aty.copy_from_slice(&aty_avg);
            }
            // 主の重みの更新。
            let mut dx2 = 0.0;
            for j in 0..n {
                let d = x[j] - x_last[j];
                dx2 += d * d;
            }
            let mut dy2 = 0.0;
            for i in 0..m {
                let d = y[i] - y_last[i];
                dy2 += d * d;
            }
            let (dxn, dyn_) = (dx2.sqrt(), dy2.sqrt());
            if dxn > 1e-10 && dyn_ > 1e-10 {
                w = (PRIMAL_WEIGHT_SMOOTHING * (dyn_ / dxn).ln() + (1.0 - PRIMAL_WEIGHT_SMOOTHING) * w.ln()).exp();
            }
            x_last.copy_from_slice(&x);
            y_last.copy_from_slice(&y);
            let er = if use_avg { &ea } else { &ec };
            err_last = kkt(er, w);
            err_prev_cand = f64::INFINITY;
            x_sum.fill(0.0);
            y_sum.fill(0.0);
            w_sum = 0.0;
            since_restart = 0;
            restarts += 1;
        }
    }
    let (_, bx, by, rel) = best;
    if debug {
        eprintln!(
            "PDLP end status={status:?} iters={total} restarts={restarts} rel=({:.1e},{:.1e},{:.1e}) t={:.2}s",
            rel.0,
            rel.1,
            rel.2,
            t0.elapsed().as_secs_f64()
        );
    }
    PdlpResult {
        status,
        x: (0..n).map(|j| (bx[j] * s.dc[j]).max(l[j]).min(u[j])).collect(),
        y: (0..m).map(|i| by[i] * s.dr[i]).collect(),
        iters: total,
        rel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_from_rows;

    #[test]
    fn pdlp_solves_small_lp() {
        // min -x0 - 2 x1, x0 + x1 + s0 = 4, x0 + 3 x1 + s1 = 6, x, s >= 0 → x = (3, 1), obj = -5。
        let rows = vec![vec![(0, 1.0), (1, 1.0), (2, 1.0)], vec![(0, 1.0), (1, 3.0), (3, 1.0)]];
        let a = csr_from_rows(&rows, 4);
        let b = [4.0, 6.0];
        let c = [-1.0, -2.0, 0.0, 0.0];
        let l = [0.0; 4];
        let u = [f64::INFINITY; 4];
        let r = solve_pdlp(&a, &b, &c, &l, &u, &PdlpOptions { eps: 1e-8, max_iters: 100_000, time_limit: 10.0 });
        assert_eq!(r.status, Status::Optimal);
        assert!((r.x[0] - 3.0).abs() < 1e-5 && (r.x[1] - 1.0).abs() < 1e-5, "x = {:?}", r.x);
        // 双対: c - A^T y >= 0、y = (-0.5, -0.5)。
        assert!((r.y[0] + 0.5).abs() < 1e-5 && (r.y[1] + 0.5).abs() < 1e-5, "y = {:?}", r.y);
    }
}
