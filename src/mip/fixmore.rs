//! 試作 `ENOMOTO_MIP_FIXMORE` (既定では使わない。analysis/fixmore_20261112.md): 根の終わりの固定を増やす。
//!
//! HiGHS (1.15.1) は大域的な範囲を変えるたびに (被約費用による固定 `propagateRootRedcost`・新しい暫定解) 大域的な範囲で
//! 行・クリーク表・含意・目的関数の行 (打ち切り値つき、クリーク分割つき) を伝播し (`HighsDomain::propagate`・`changeBound` の
//! `cliquetable.addImplications`)、その結果の固定を再スタートの判定 (`percentageInactiveIntegers`) に数える。
//! ENOMOTO の根の被約費用による固定 (`Solver::root_redcost_fixing`) は境界を締めるだけで、大域的な範囲では伝播しない
//! (伝播はノードごとにやり直す)。そのため、再スタートの判定で数える固定が少ない。
//!
//! 値の文字で部品を選ぶ:
//! - `d` (診断、解き方は変えない): 再スタートの判定の前に、大域的な範囲から (1) 行だけ (2) 行 + 含意 (衝突グラフ
//!   `MipProblem::conflict_edges`) (3) それに打ち切り値つきの目的関数の行、で伝播した閉包の固定の数を 1 行出す。
//! - `p` 閉包の結果を大域的な範囲に入れる (整数列と目的関数の列だけ)。`i` で含意を、`o` で目的関数の行を伝播に入れる。
//!
//! 正しさ: 行と含意の伝播は全ての実行可能解で成り立つ推論。目的関数の行 `c x <= U` は打ち切り値より良い解の推論で、
//! 被約費用による固定と同じ扱い (暫定解を持ったまま、より良い解だけを探す)。

use super::domain::Domain;
use super::problem::MipProblem;
use super::lp_api::MipLp;
use super::solver::Solver;
use std::time::Instant;

/// 値の文字 `c` を含むか。
pub(crate) fn flag(c: char) -> bool {
    env_str!("ENOMOTO_MIP_FIXMORE").is_some_and(|v| v.contains(c))
}

/// 伝播を待ちがなくなるまで (手間の上限で打ち切られたら続けて、最大 `ENOMOTO_T_MIP_FIXMORE_ROUNDS` (既定 50) 回) 回す。矛盾なら偽。
fn propagate_all(p: &MipProblem, d: &mut Domain) -> bool {
    let rounds = tunable!("ENOMOTO_T_MIP_FIXMORE_ROUNDS", 50usize, usize);
    for _ in 0..rounds {
        if !d.propagate(p) {
            return false;
        }
        if !d.has_pending() {
            break;
        }
    }
    true
}

/// 目的関数の行 `sum c_j x_j <= u` (定数項を除いた打ち切り値) で 1 回締める (`Solver::global_obj_prop` と同じ)。
/// 矛盾なら None、締めたら Some(真)。
fn obj_row_pass(p: &MipProblem, d: &mut Domain, obj: &[(usize, f64)], u: f64) -> Option<bool> {
    let (mut minact, mut ninf) = (0.0f64, 0usize);
    for &(j, c) in obj {
        let b = if c > 0.0 { d.lo[j] } else { d.up[j] };
        if b.is_finite() {
            minact += c * b;
        } else {
            ninf += 1;
        }
    }
    if ninf > 1 {
        return Some(false);
    }
    let s0 = d.stack_len();
    for &(j, c) in obj {
        let b = if c > 0.0 { d.lo[j] } else { d.up[j] };
        let rest = if b.is_finite() {
            if ninf > 0 {
                continue;
            }
            minact - c * b
        } else {
            minact
        };
        let lim = (u - rest) / c;
        // 丸め誤差の分だけ緩める (整数列は `tighten_*` が切り捨てる前に許容誤差を足す)
        let slack = 1e-9 * (1.0 + lim.abs());
        if c > 0.0 {
            if lim + slack < d.up[j] {
                d.tighten_upper(p, j, lim + slack);
            }
        } else if lim - slack > d.lo[j] {
            d.tighten_lower(p, j, lim - slack);
        }
        if d.infeasible {
            return None;
        }
    }
    Some(d.stack_len() > s0)
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 大域的な範囲から伝播した閉包を作る。`impls` で含意を、`obj` で打ち切り値つきの目的関数の行を使う。矛盾なら None。
    fn fixmore_closure(&self, impls: bool, obj: bool) -> Option<Domain> {
        let p = self.p;
        let mut d = Domain::new(p);
        if impls {
            d.add_implications(p.n, &p.conflict_edges);
        }
        for j in 0..p.n {
            if self.dom.global_lo[j] > d.lo[j] {
                d.tighten_lower(p, j, self.dom.global_lo[j]);
            }
            if self.dom.global_up[j] < d.up[j] {
                d.tighten_upper(p, j, self.dom.global_up[j]);
            }
        }
        if d.infeasible {
            return None;
        }
        if impls {
            d.queue_fixed_binaries(p);
        }
        if !propagate_all(p, &mut d) {
            return None;
        }
        if obj {
            let u = self.prune_limit() - p.offset;
            let cols: Vec<(usize, f64)> = (0..p.n).filter(|&j| p.cost[j] != 0.0).map(|j| (j, p.cost[j])).collect();
            if u.is_finite() && !cols.is_empty() {
                for _ in 0..tunable!("ENOMOTO_T_MIP_FIXMORE_OBJ_PASSES", 10usize, usize) {
                    match obj_row_pass(p, &mut d, &cols, u) {
                        None => return None,
                        Some(false) => break,
                        Some(true) => {
                            if !propagate_all(p, &mut d) {
                                return None;
                            }
                        }
                    }
                }
            }
        }
        Some(d)
    }

    /// 再スタートの判定の前に呼ぶ (`at` はログの印)。`d` なら閉包の固定の数を出し、`p` なら閉包を大域的な範囲に入れる。
    pub(super) fn fixmore_root(&mut self, at: &str) {
        if self.params.submip || !(flag('d') || flag('p')) || self.dom.infeasible {
            return;
        }
        let p = self.p;
        let t0 = Instant::now();
        let nfix = |lo: &[f64], up: &[f64]| (0..p.n).filter(|&j| p.is_int[j] && lo[j] == up[j]).count();
        let before = nfix(&self.dom.global_lo, &self.dom.global_up);
        if flag('d') {
            let mut cnt = [0usize; 3];
            let mut infeas = [false; 3];
            for (k, (im, ob)) in [(false, false), (true, false), (true, true)].into_iter().enumerate() {
                match self.fixmore_closure(im, ob) {
                    Some(d) => cnt[k] = nfix(&d.lo, &d.up),
                    None => infeas[k] = true,
                }
            }
            eprintln!(
                "FIXMORE diag at={at} restarts={} nint={} root_fixed0={} fixed={before} rows={} rows_impl={} rows_impl_obj={} infeas={:?} ub={:?} edges={} ({:.3}s)",
                self.params.restarts,
                p.nint,
                self.root_fixed0,
                cnt[0],
                cnt[1],
                cnt[2],
                infeas,
                self.incumbent.as_ref().map(|(z, _)| *z),
                p.conflict_edges.len(),
                t0.elapsed().as_secs_f64()
            );
        }
        if flag('p') {
            let Some(d) = self.fixmore_closure(flag('i'), flag('o')) else {
                // 打ち切り値より良い解がない (または問題が実行不能)。ここでは何もしない (木が刈って閉じる)
                return;
            };
            let mut nt = 0usize;
            for j in 0..p.n {
                if !(p.is_int[j] || p.cost[j] != 0.0) {
                    continue;
                }
                let (glo, gup) = (self.dom.global_lo[j], self.dom.global_up[j]);
                if d.lo[j] > glo + 1e-9 * (1.0 + glo.abs()) {
                    self.dom.tighten_global(p, j, false, d.lo[j]);
                    nt += 1;
                }
                if d.up[j] < gup - 1e-9 * (1.0 + gup.abs()) {
                    self.dom.tighten_global(p, j, true, d.up[j]);
                    nt += 1;
                }
            }
            if self.params.verbose {
                eprintln!(
                    "MIP: FIXMORE at {at}: tightened {nt} bounds, fixed integers {before} -> {} ({:.3}s)",
                    nfix(&self.dom.global_lo, &self.dom.global_up),
                    t0.elapsed().as_secs_f64()
                );
            }
        }
    }
}
