//! 試作 `ENOMOTO_MIP_RCSTRONG` (既定では使わない。analysis/rcstrong_20261111.md): 被約費用による固定を強める。
//!
//! 被約費用による固定はもともと 1 本のナップサック不等式 Σ_j d_j δ_j ≤ G (G = 打ち切り値 − LP の値、δ_j は LP で張り付いた
//! 境界からの距離)。今の作りは列 1 本ずつ δ_j ≤ G / d_j で締め、`is_int` の列だけ切り捨てる。
//!
//! 値の文字で部品を選ぶ:
//! - `i` 暗黙の整数 (主側のもの: 等式行の規則と TU2 の規則、[`primal_implint_mask`]) の連続列も、根の被約費用による大域的な固定と
//!   ノードの局所的な固定で境界を切り捨てる。TU2 の列は「整数の最適解が 1 つは残る」意味なので、切り捨ては最適解を 1 つ残す縮約
//!   (双対的な縮約と同じ扱い)。双対側の暗黙の整数 (最適解でだけ整数) は使わない。
//! - `k` 根の LP の被約費用の不等式 Σ_j d_j (x_j − 境界_j) ≤ 打ち切り値 − z を、目的関数の伝播と同じ 1 本の行として、ノードの伝播の
//!   不動点の繰り返し (`Solver::propagate_objective` を呼ぶ所: ノードの LP の前、不動点の繰り返し、強分岐の子の伝播) で伝播する
//!   (SCIP の被約費用の伝播と同じ考え)。全ての改善解で成り立つので、矛盾したら衝突解析の証明にもする。大域的な境界では
//!   どの項も 0 以上なので、項を落としても成り立つ: 密にならないよう、min(|d_j| × 幅_j, G) が G の
//!   `ENOMOTO_T_MIP_RCSTRONG_KFRAC` (既定 0.05) 倍以上の列に絞り、多いときは大きい順に `ENOMOTO_T_MIP_RCSTRONG_KMAX`
//!   (既定 2000) 本まで。打ち切り値が変わるたびに作り直す。
//! - `r` (`i` と一緒に) 切り捨てを根の大域的な固定だけにし、ノードの局所的な固定では切り捨てない (pk1 でノードの切り捨てが
//!   揃って遅くしたため。ノードでは数えるだけ)。
//! - `c` (診断、解き方は変えない): `i` で切り捨てたら新たに締まる・固定される本数を数えて、終わりに 1 行出す (`i` と一緒なら
//!   実際に締めた数)。

use super::domain::FEASTOL;
use super::lp_api::MipLp;
use super::problem::MipProblem;
use super::solver::Solver;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// 値の文字 `c` を含むか。
pub(crate) fn flag(c: char) -> bool {
    env_str!("ENOMOTO_MIP_RCSTRONG").is_some_and(|v| v.contains(c))
}

/// 暗黙の整数を使う (`i`) か、数える (`c`) か。
pub(crate) fn implint_on() -> bool {
    flag('i') || flag('c')
}

/// 全スレッドのノードでの数: [見た列 (印のない列も), 切り捨てで新たに締まった列 (切り捨てなしでは締めない), 切り捨てで境界に固定された列,
/// 切り捨てなしでも締める列]
static NODE_CNT: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];

/// 根の被約費用の表 (根の LP の値, (列, 下限側か, 被約費用, 境界))。
pub(crate) type RootRc = std::sync::Arc<(f64, Vec<(usize, bool, f64, f64)>)>;

/// 主が従に渡す根の被約費用の表 (`WorkPool::rcs_root`)。
pub(crate) type SharedRoot = std::sync::Mutex<Option<RootRc>>;

/// `k` の全スレッドの数: [呼んだ回数, 矛盾 (刈り込み), 締めた境界 (後の行の伝播の分も含む), 行を作った回数, 行の長さの和]
static K_CNT: [AtomicU64; 5] = [const { AtomicU64::new(0) }; 5];

/// ソルバーごとの状態。
#[derive(Default)]
pub(crate) struct RcsState {
    /// 暗黙の整数の連続列の印 (主側のもの)。`None` は未計算。
    pub mask: Option<Vec<bool>>,
    /// 根の固定で: 被約費用の表に入った暗黙の整数の列、切り捨てなしで締まった列、切り捨てで新たに締まった列、切り捨てで固定された列
    /// (列ごとに一度だけ数える)
    root_seen: Vec<u8>,
    root_cnt: [usize; 4],
    /// 印の数 (等式行の規則・TU2 の規則)。
    nmark: (usize, usize),
    /// 捨てるときに数を出すか (主のスレッドで印を作ったとき)。
    report: bool,
    /// `k` の行 (列, 被約費用) と右辺、作ったときの打ち切り値。
    krow: std::rc::Rc<Vec<(usize, f64)>>,
    krhs: f64,
    klim: f64,
    /// `k` で使う根の被約費用の表 (主は自分の表の写し、従は主から受け取ったもの)。
    kroot: Option<RootRc>,
}

impl Drop for RcsState {
    /// 数を 1 行出す (再スタートで求解の途中で捨てるものも出す。ノードの数は全スレッド・全ての求解の累計)。
    fn drop(&mut self) {
        if !self.report {
            return;
        }
        let c = self.root_cnt;
        let n: Vec<u64> = NODE_CNT.iter().map(|a| a.load(Relaxed)).collect();
        if flag('k') {
            let k: Vec<u64> = K_CNT.iter().map(|a| a.load(Relaxed)).collect();
            eprintln!(
                "MIP: RCSTRONG: knapsack row (all threads, cumulative): calls {}, pruned {}, tightened {}, built {} (mean length {:.1})",
                k[0],
                k[1],
                k[2],
                k[3],
                k[4] as f64 / k[3].max(1) as f64
            );
        }
        if self.mask.is_none() {
            return;
        }
        eprintln!(
            "MIP: RCSTRONG: implint {}+{}; root: in rc list {}, tightened without rounding {}, tighter by rounding {}, fixed by rounding {}; nodes (cumulative): tighter by rounding {}, fixed by rounding {}, tightened without rounding {} (columns looked at {})",
            self.nmark.0, self.nmark.1, c[0], c[1], c[2], c[3], n[1], n[2], n[3], n[0]
        );
    }
}

/// 主側の暗黙の整数の連続列の印。等式行 `a_j x_j + Σ a_k x_k = b` で他の列がすべて (暗に) 整数、`a_k / a_j`・`b / a_j` が整数なら
/// どの整数解でも `x_j` は整数 ([`MipProblem::implied_integers`] の前半。双対側の規則は使わない)。その上に TU2 の規則
/// ([`super::implint_tu::implied_integers_tu2_opt`]、整数の最適解が 1 つは残る) を足す。返すのは連続列で印の付いたものだけ真の列と、
/// (等式行の規則の数, TU2 で足した数)。
pub(crate) fn primal_implint_mask(p: &MipProblem) -> (Vec<bool>, (usize, usize)) {
    let integral = |v: f64| (v - v.round()).abs() <= 1e-9 * v.abs().max(1.0);
    let mut mark = p.is_int.clone();
    loop {
        let mut changed = false;
        for j in 0..p.n {
            if mark[j] {
                continue;
            }
            let ok = p.cols[j].iter().any(|&(i, a)| {
                p.row_lo[i] == p.row_up[i] && integral(p.row_lo[i] / a) && p.rows[i].iter().all(|&(k, b)| k == j || (mark[k] && integral(b / a)))
            });
            if ok {
                mark[j] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let n_eq = (0..p.n).filter(|&j| mark[j] && !p.is_int[j]).count();
    let (t, _) = super::implint_tu::implied_integers_tu2_opt(p, &mark, true);
    let out: Vec<bool> = (0..p.n).map(|j| t[j] && !p.is_int[j]).collect();
    let n_all = out.iter().filter(|&&b| b).count();
    (out, (n_eq, n_all - n_eq))
}

/// ノードの局所的な固定 (`Solver::local_redcost_fixing`) の 1 列分: 暗黙の整数の連続列 `j` の境界の候補 `v` (上限なら `upper`) を
/// 切り捨てた値を返す (`i` を付けていなければ数えるだけで `None`)。`min_cut` は切り捨てなしの締め付けの最小幅。
/// 印のない列・整数列は `None` (呼び出し側の今まで通りの扱い)。
pub(crate) fn node_round(st: &RcsState, j: usize, upper: bool, v: f64, lo: f64, up: f64, min_cut: f64) -> Option<f64> {
    let m = st.mask.as_ref()?;
    NODE_CNT[0].fetch_add(1, Relaxed);
    if !m[j] {
        return None;
    }
    let raw = if upper { up - v >= min_cut } else { v - lo >= min_cut };
    let r = if upper { (v + FEASTOL).floor() } else { (v - FEASTOL).ceil() };
    let tight = if upper { r < up - 1e-9 } else { r > lo + 1e-9 };
    if raw {
        NODE_CNT[3].fetch_add(1, Relaxed);
    } else if tight {
        NODE_CNT[1].fetch_add(1, Relaxed);
        let fixed = if upper { r <= lo + 1e-9 } else { r >= up - 1e-9 };
        if fixed {
            NODE_CNT[2].fetch_add(1, Relaxed);
        }
    }
    if flag('i') && !flag('r') && tight {
        Some(r)
    } else {
        None
    }
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// `k`: 根の被約費用の行を今の定義域で伝播する。矛盾なら偽。
    pub(super) fn rcs_propagate(&mut self) -> bool {
        if self.dom.infeasible {
            return false;
        }
        if !flag('k') || self.params.submip {
            return true;
        }
        let lim = self.prune_limit();
        if !lim.is_finite() {
            return true;
        }
        // 根の表: 主は自分の表 (根の LP の値が変われば取り直す) を写して従にも渡す。従は主から受け取る
        if let Some((z, list)) = &self.root_redcost {
            if self.rcs.kroot.as_ref().is_none_or(|r| r.0 != *z || r.1.len() != list.len()) {
                let r: RootRc = std::sync::Arc::new((*z, list.clone()));
                if let Some(pool) = &self.pool {
                    *pool.rcs_root.lock().unwrap() = Some(r.clone());
                }
                self.rcs.kroot = Some(r);
                self.rcs.klim = f64::NAN;
            }
        } else if self.rcs.kroot.is_none() {
            let Some(pool) = &self.pool else { return true };
            let Some(r) = pool.rcs_root.lock().unwrap().clone() else { return true };
            self.rcs.kroot = Some(r);
            self.rcs.klim = f64::NAN;
        }
        let Some(root) = self.rcs.kroot.clone() else { return true };
        let (z, list) = (&root.0, &root.1);
        if lim != self.rcs.klim {
            let g = lim - z;
            let frac = tunable!("ENOMOTO_T_MIP_RCSTRONG_KFRAC", 0.05, f64);
            let kmax = tunable!("ENOMOTO_T_MIP_RCSTRONG_KMAX", 2000usize, usize);
            let mut v: Vec<(f64, usize, f64, f64)> = Vec::new();
            for &(j, _, d, bound) in list {
                let w = self.dom.global_up[j] - self.dom.global_lo[j];
                if !(w > 0.0) {
                    continue;
                }
                let sc = (d.abs() * w).min(g.max(0.0));
                if sc >= frac * g {
                    v.push((sc, j, d, bound));
                }
            }
            if v.len() > kmax {
                v.sort_by(|a, b| b.0.total_cmp(&a.0));
                v.truncate(kmax);
            }
            v.sort_by_key(|e| e.1);
            // Σ d_j (x_j − b_j) ≤ G  ⇔  Σ d_j x_j ≤ G + Σ d_j b_j
            self.rcs.krhs = g + v.iter().map(|e| e.2 * e.3).sum::<f64>();
            self.rcs.krow = std::rc::Rc::new(v.iter().map(|e| (e.1, e.2)).collect());
            self.rcs.klim = lim;
            self.rcs.report |= self.pool_tid == 0 && self.root_redcost.is_some();
            K_CNT[3].fetch_add(1, Relaxed);
            K_CNT[4].fetch_add(self.rcs.krow.len() as u64, Relaxed);
        }
        if self.rcs.krow.is_empty() {
            return true;
        }
        let row = self.rcs.krow.clone();
        let s0 = self.dom.stack_len();
        K_CNT[0].fetch_add(1, Relaxed);
        let ok = self.propagate_le_row(&row, self.rcs.krhs, true);
        if !ok {
            K_CNT[1].fetch_add(1, Relaxed);
        } else {
            K_CNT[2].fetch_add(self.dom.stack_len().saturating_sub(s0) as u64, Relaxed);
        }
        ok
    }

    /// 印を (まだなら) 作る。根の被約費用を保存する前に呼ぶ。
    pub(super) fn rcs_prepare(&mut self) {
        if !implint_on() || self.params.submip || self.rcs.mask.as_ref().is_some_and(|m| m.len() == self.p.n) {
            return;
        }
        let t0 = std::time::Instant::now();
        let (m, nm) = primal_implint_mask(self.p);
        self.rcs.nmark = nm;
        self.rcs.root_seen = vec![0; self.p.n];
        if self.pool_tid == 0 {
            eprintln!("MIP: RCSTRONG: implied integer columns {} by equality rows, +{} by TU2 ({:.3}s)", nm.0, nm.1, t0.elapsed().as_secs_f64());
        }
        self.rcs.mask = Some(m);
        self.rcs.report = self.pool_tid == 0;
    }

    /// 根の被約費用の表に連続列 `j` を入れるか (暗黙の整数の印がある列)。
    pub(super) fn rcs_store_col(&self, j: usize) -> bool {
        self.rcs.mask.as_ref().is_some_and(|m| m[j])
    }

    /// 根の大域的な固定の 1 列分: 締める値 (整数列・印のない列は `v` のまま)。暗黙の整数の連続列は、`i` なら切り捨てた値、`c` だけなら
    /// 締めない (`None`)。数も取る。
    pub(super) fn rcs_root_round(&mut self, j: usize, upper: bool, v: f64) -> Option<f64> {
        if self.p.is_int[j] || !self.rcs_store_col(j) {
            return Some(v);
        }
        let (glo, gup) = (self.dom.global_lo[j], self.dom.global_up[j]);
        let r = if upper { (v + FEASTOL).floor() } else { (v - FEASTOL).ceil() };
        let eps = |b: f64| 1e-9 * (1.0 + b.abs());
        let (raw, tight, fixed) = if upper {
            (v < gup - eps(gup), r < gup.min(v) - eps(gup), r <= glo + eps(glo))
        } else {
            (v > glo + eps(glo), r > glo.max(v) + eps(glo), r >= gup - eps(gup))
        };
        let s = &mut self.rcs.root_seen[j];
        let mut bit = |b: u8, k: usize, c: &mut [usize; 4]| {
            if *s & b == 0 {
                *s |= b;
                c[k] += 1;
            }
        };
        let c = &mut self.rcs.root_cnt;
        bit(1, 0, c);
        if raw {
            bit(2, 1, c);
        }
        if tight {
            bit(4, 2, c);
            if fixed {
                bit(8, 3, c);
            }
        }
        if flag('i') {
            Some(r)
        } else {
            None
        }
    }
}
