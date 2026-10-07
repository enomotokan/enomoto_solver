//! SCIP 由来の主ヒューリスティクス (Crossover・DINS・locks・clique・vbounds) と解のプール。

use super::cuts::VarBounds;
use super::domain::FEASTOL;
use super::lp_api::MipLp;
use super::solver::Solver;

/// 解のプールに保つ解の数。
const POOL_SIZE: usize = 10;

impl<'a, L: MipLp> Solver<'a, L> {
    /// 実行可能解をプールに加える (目的値の良い順に最大 `POOL_SIZE` 個、整数列の値が同じものは入れない)。
    pub(super) fn pool_add(&mut self, z: f64, x: &[f64]) {
        let p = self.p;
        let same = |y: &[f64]| (0..p.n).all(|j| !p.is_int[j] || (x[j] - y[j]).abs() <= 0.5);
        if self.sol_pool.iter().any(|(_, y)| same(y)) {
            return;
        }
        let pos = self.sol_pool.partition_point(|(w, _)| *w <= z);
        if pos >= POOL_SIZE {
            return;
        }
        self.sol_pool.insert(pos, (z, x.to_vec()));
        self.sol_pool.truncate(POOL_SIZE);
    }

    /// Crossover (SCIP の heur_crossover): プールの最良の解と他の解 (最大 2 つ、乱数で選ぶ) で値が一致する整数列を
    /// 固定したサブ MIP を解く。固定率が 5 割に届かなければ使わない。
    pub(super) fn crossover(&mut self) -> bool {
        let p = self.p;
        if self.params.submip || self.sol_pool.len() < 2 {
            return false;
        }
        let mut picks = vec![0usize];
        let others: Vec<usize> = (1..self.sol_pool.len()).collect();
        let k = others.len().min(2);
        let mut rest = others;
        for _ in 0..k {
            let r = (self.rand() * rest.len() as f64) as usize;
            picks.push(rest.remove(r.min(rest.len() - 1)));
        }
        let mut lo = self.dom.global_lo.clone();
        let mut up = self.dom.global_up.clone();
        let (mut nint, mut nfix) = (0usize, 0usize);
        for j in 0..p.n {
            if !p.is_int[j] {
                continue;
            }
            nint += 1;
            let v = self.sol_pool[picks[0]].1[j].round();
            if picks.iter().all(|&q| (self.sol_pool[q].1[j] - v).abs() <= FEASTOL) && v >= lo[j] && v <= up[j] {
                lo[j] = v;
                up[j] = v;
                nfix += 1;
            }
        }
        if nint == 0 || (nfix as f64) < tunable!("ENOMOTO_T_MIP_CROSSOVER_RATE", 0.5, f64) * nint as f64 || nfix == nint {
            return false;
        }
        let r = self.solve_submip(lo, up, 500);
        if self.params.verbose {
            eprintln!("MIP:   crossover ({nfix} of {nint} fixed, {} solutions): improved {r}", picks.len());
        }
        r
    }

    /// DINS (Ghosh 2007、SCIP の heur_dins の簡略版): 暫定解と LP 解の差が 0.5 未満の整数列を暫定解の値に固定し、
    /// 他の整数列は暫定解から LP 解との差の範囲に制限したサブ MIP を解く。
    pub(super) fn dins(&mut self, x: &[f64]) -> bool {
        let p = self.p;
        if self.params.submip {
            return false;
        }
        let Some((_, inc)) = self.incumbent.as_ref() else { return false };
        let inc = inc.clone();
        let mut lo = self.dom.lo.clone();
        let mut up = self.dom.up.clone();
        let (mut nint, mut nfix) = (0usize, 0usize);
        for j in 0..p.n {
            if !p.is_int[j] {
                continue;
            }
            nint += 1;
            let d = (inc[j] - x[j]).abs();
            if inc[j] < lo[j] || inc[j] > up[j] {
                continue;
            }
            if d < 0.5 {
                lo[j] = inc[j];
                up[j] = inc[j];
                nfix += 1;
            } else {
                lo[j] = lo[j].max((inc[j] - d).floor());
                up[j] = up[j].min((inc[j] + d).ceil());
            }
        }
        if nint == 0 || (nfix as f64) < 0.3 * nint as f64 || nfix == nint {
            return false;
        }
        let r = self.solve_submip(lo, up, 500);
        if self.params.verbose {
            eprintln!("MIP:   DINS ({nfix} of {nint} fixed): improved {r}");
        }
        r
    }

    /// 固定 (列, 値) を順に積んで伝播し (矛盾したものは飛ばす)、整数列の固定率が `stop_rate` に達したら止める。
    /// 固定率が `min_rate` 以上ならその定義域でサブ MIP を解く。定義域は戻す。
    fn fix_then_submip(&mut self, fixes: &[(usize, f64)], stop_rate: f64, min_rate: f64, label: &str) -> bool {
        let p = self.p;
        let ints: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j] && self.dom.lo[j] < self.dom.up[j]).collect();
        if ints.is_empty() || fixes.is_empty() {
            return false;
        }
        let nint = ints.len() as f64;
        let rate = |s: &Self| ints.iter().filter(|&&j| s.dom.lo[j] == s.dom.up[j]).count() as f64 / nint;
        let pos = self.dom.stack_len();
        let work0 = self.dom.debug_work();
        let work_cap = 20 * p.rows.iter().map(|r| r.len() as u64).sum::<u64>() + 1_000_000;
        for (k, &(j, v)) in fixes.iter().enumerate() {
            if k % 64 == 0 && (self.time_up() || self.dom.debug_work() - work0 > work_cap) {
                break;
            }
            if self.dom.lo[j] == self.dom.up[j] || v < self.dom.lo[j] || v > self.dom.up[j] {
                continue;
            }
            let before = self.dom.stack_len();
            self.dom.tighten_lower(p, j, v);
            self.dom.tighten_upper(p, j, v);
            if !self.dom.propagate(p) {
                self.dom.backtrack_to(p, before);
                continue;
            }
            if k % 16 == 0 && rate(self) >= stop_rate {
                break;
            }
        }
        let fr = rate(self);
        let (lo, up) = (self.dom.lo.clone(), self.dom.up.clone());
        self.dom.backtrack_to(p, pos);
        if self.params.verbose && self.nodes <= 1 {
            eprintln!("MIP:   {label}: fixing rate {fr:.2}");
        }
        if fr < min_rate {
            return false;
        }
        self.solve_submip(lo, up, 500)
    }

    /// locks (SCIP の heur_locks): lock の多い整数列から、lock の少ない側の境界に固定して伝播し、固定率 65% で
    /// サブ MIP を解く。
    pub(super) fn locks_heur(&mut self) -> bool {
        let p = self.p;
        if self.params.submip {
            return false;
        }
        let mut order: Vec<(u32, usize)> = (0..p.n).filter(|&j| p.is_int[j] && self.dom.lo[j] < self.dom.up[j]).map(|j| (self.locks[j].0 + self.locks[j].1, j)).collect();
        order.sort_by(|a, b| b.0.cmp(&a.0));
        let fixes: Vec<(usize, f64)> = order
            .iter()
            .filter_map(|&(_, j)| {
                let (dl, ul) = self.locks[j];
                let (lo, up) = (self.dom.lo[j], self.dom.up[j]);
                let v = if dl <= ul { lo } else { up };
                let v = if v.is_finite() { v } else if lo.is_finite() { lo } else { up };
                v.is_finite().then_some((j, v))
            })
            .collect();
        self.fix_then_submip(&fixes, 0.65, 0.3, "locks")
    }

    /// clique (SCIP の heur_clique の簡略版): 集合パッキング・分割の行 (2 値列だけ、係数が同じ正の値で上限がその値)
    /// を大きい順に、まだ 1 の列がなければ LP 値 (なければ費用) の最も良い列を 1 に固定して伝播する (他の列は伝播で 0)。
    /// 固定率が 3 割以上ならサブ MIP を解く。`x` は根の LP 解。
    pub(super) fn clique_heur(&mut self, x: &[f64]) -> bool {
        let p = self.p;
        if self.params.submip {
            return false;
        }
        let binary = |j: usize| p.is_int[j] && self.dom.global_lo[j] == 0.0 && self.dom.global_up[j] == 1.0;
        let mut cliques: Vec<usize> = (0..p.m)
            .filter(|&i| {
                let r = &p.rows[i];
                if r.len() < 2 {
                    return false;
                }
                let c = r[0].1;
                c > 0.0 && r.iter().all(|&(j, a)| binary(j) && (a - c).abs() <= 1e-12 * c) && (p.row_up[i] - c).abs() <= 1e-9 * c
            })
            .collect();
        if cliques.is_empty() {
            return false;
        }
        cliques.sort_by_key(|&i| std::cmp::Reverse(p.rows[i].len()));
        let mut fixes: Vec<(usize, f64)> = Vec::new();
        let mut chosen = vec![false; p.n];
        for &i in &cliques {
            if p.rows[i].iter().any(|&(j, _)| chosen[j]) {
                continue;
            }
            let best = p.rows[i].iter().map(|&(j, _)| j).max_by(|&a, &b| (x[a] - 1e-6 * p.cost[a]).total_cmp(&(x[b] - 1e-6 * p.cost[b])));
            if let Some(j) = best {
                chosen[j] = true;
                fixes.push((j, 1.0));
            }
        }
        self.fix_then_submip(&fixes, 1.0, 0.3, "clique")
    }

    /// vbounds (SCIP の heur_vbounds の簡略版): 変数上下限 (連続列 `x <= d y + e` / `x >= d y + e`、`y` は整数列) の
    /// 制御側の整数列を、`loose` なら上下限を緩める側 (VUB で d > 0 なら上限)、そうでなければ締める側に固定して伝播し、
    /// 固定率が 3 割以上ならサブ MIP を解く。
    pub(super) fn vbounds_heur(&mut self, loose: bool) -> bool {
        let p = self.p;
        if self.params.submip {
            return false;
        }
        let vb = VarBounds::from_rows(p.n, &p.is_int, &p.rows, &p.row_lo, &p.row_up);
        let mut want: Vec<Option<bool>> = vec![None; p.n]; // 制御側の列を上限に (true) / 下限に
        for k in 0..p.n {
            for &(y, d, _) in &vb.vub[k] {
                // x <= d y + e: d > 0 なら y を大きくすると緩む
                want[y].get_or_insert((d > 0.0) == loose);
            }
            for &(y, d, _) in &vb.vlb[k] {
                // x >= d y + e: d > 0 なら y を小さくすると緩む
                want[y].get_or_insert((d < 0.0) == loose);
            }
        }
        let fixes: Vec<(usize, f64)> = (0..p.n)
            .filter_map(|y| {
                let w = want[y]?;
                let v = if w { self.dom.up[y] } else { self.dom.lo[y] };
                v.is_finite().then_some((y, v))
            })
            .collect();
        if fixes.is_empty() {
            return false;
        }
        self.fix_then_submip(&fixes, 1.0, 0.3, if loose { "vbounds (loose)" } else { "vbounds (tight)" })
    }
}
