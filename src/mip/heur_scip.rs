//! SCIP 由来の主ヒューリスティクス (Crossover・DINS・locks・clique・vbounds) と解のプール。

use super::cuts::VarBounds;
use super::domain::FEASTOL;
use super::lp_api::MipLp;
use super::solver::Solver;

/// 点 `x` で違反している行 (相対 1e-6) の数と違反量の合計。
pub(super) fn viol_stats(p: &super::problem::MipProblem, x: &[f64]) -> (usize, f64) {
    let tol = |b: f64| 1e-6 * (1.0 + b.abs());
    let (mut k, mut v) = (0usize, 0.0f64);
    for i in 0..p.m {
        let a: f64 = p.rows[i].iter().map(|&(j, c)| c * x[j]).sum();
        let d = if a < p.row_lo[i] - tol(p.row_lo[i]) { p.row_lo[i] - a } else if a > p.row_up[i] + tol(p.row_up[i]) { a - p.row_up[i] } else { 0.0 };
        if d > 0.0 {
            k += 1;
            v += d;
        }
    }
    (k, v)
}

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

    /// `ENOMOTO_MIP_CROSSOVER_FOCUS`: 解のプールのうち「目的値 - 今の全体の下界」が「最も悪い解の目的値 - 根の下界」の
    /// `ENOMOTO_T_MIP_CROSSOVER_FOCUS_FRAC` (既定 0.5) 倍以下の解が `ENOMOTO_T_MIP_CROSSOVER_FOCUS_MIN` (既定 3) 個以上あれば、
    /// その解の番号 (良い順) を返す。最も悪い解は最初の暫定解とプールの最悪の解の悪い方、根の下界はカットの後の根の LP の値。
    /// 大近傍探索 ([`Self::alns`]) はこのとき `ENOMOTO_T_MIP_CROSSOVER_FOCUS_PROB` (既定 0.75) の確率で、この解の中から選ぶ Crossover を使う
    pub(super) fn crossover_focus_set(&self) -> Option<Vec<usize>> {
        if env_str!("ENOMOTO_MIP_CROSSOVER_FOCUS").is_none() || self.params.submip || self.sol_pool.len() < 3 {
            return None;
        }
        let worst = self.sol_pool.iter().map(|(z, _)| *z).fold(self.proof_first_z, |a, b| if a.is_finite() { a.max(b) } else { b });
        let lb = self.global_lower_bound();
        if !(worst.is_finite() && lb.is_finite() && self.hcm_root_obj.is_finite()) {
            return None;
        }
        let thr = tunable!("ENOMOTO_T_MIP_CROSSOVER_FOCUS_FRAC", 0.5, f64) * (worst - self.hcm_root_obj).max(0.0);
        let good: Vec<usize> = (0..self.sol_pool.len()).filter(|&k| self.sol_pool[k].0 - lb <= thr).collect();
        (good.len() >= tunable!("ENOMOTO_T_MIP_CROSSOVER_FOCUS_MIN", 3usize, usize).max(2)).then_some(good)
    }

    /// Crossover (SCIP の heur_crossover): プールの最良の解と他の解 (最大 2 つ、乱数で選ぶ) で値が一致する整数列を
    /// 固定したサブ MIP を解く。固定率が 5 割に届かなければ使わない。
    pub(super) fn crossover(&mut self) -> bool {
        self.crossover_from(None)
    }

    /// [`Self::crossover`] を、`cands` (解のプールの番号、良い順) があればその中から選んだ解で行う。
    pub(super) fn crossover_from(&mut self, cands: Option<&[usize]>) -> bool {
        let p = self.p;
        if self.params.submip || self.sol_pool.len() < 2 || cands.is_some_and(|c| c.len() < 2) {
            return false;
        }
        let all: Vec<usize> = (0..self.sol_pool.len()).collect();
        let cands = cands.unwrap_or(&all);
        let mut picks = vec![cands[0]];
        let others: Vec<usize> = cands[1..].to_vec();
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

    /// minimum relaxation (違反量を最小にする補助 MIP、Gurobi の feasRelax と同じ考え): 各行に違反量の非負の列
    /// (下限側・上限側) を足し、行の係数の大きさで割った違反量の合計を最小にするサブ MIP を解く。違反量 0 の点が
    /// 見つかれば実行可能解。0 にならなければ、その点の整数列の値を目標に固定・伝播し、連続部分を LP で解いて試す。
    pub(super) fn min_relaxation(&mut self) -> bool {
        let p = self.p;
        if self.params.submip || p.m == 0 {
            return false;
        }
        let n = p.n;
        let mut rows = p.rows.clone();
        let mut col_lo = self.dom.global_lo.clone();
        let mut col_up = self.dom.global_up.clone();
        let mut cost = vec![0.0; n];
        let mut is_int = p.is_int.clone();
        for (i, r) in rows.iter_mut().enumerate() {
            let scale = r.iter().fold(0.0f64, |m, &(_, a)| m.max(a.abs())).max(1e-9);
            // 下限側の違反 s_lo (a x + s_lo >= lo)、上限側の違反 s_up (a x - s_up <= up)
            if p.row_lo[i].is_finite() {
                r.push((col_lo.len(), scale));
                col_lo.push(0.0);
                col_up.push(f64::INFINITY);
                cost.push(1.0);
                is_int.push(false);
            }
            if p.row_up[i].is_finite() {
                r.push((col_lo.len(), -scale));
                col_lo.push(0.0);
                col_up.push(f64::INFINITY);
                cost.push(1.0);
                is_int.push(false);
            }
        }
        let sub = super::problem::MipProblem::from_rows(col_lo, col_up, cost, 0.0, 1.0, is_int, rows, p.row_lo.clone(), p.row_up.clone());
        let remaining = match self.deadline {
            Some(d) => d.saturating_duration_since(std::time::Instant::now()).as_secs_f64(),
            None => f64::INFINITY,
        };
        let params = super::solver::MipParams {
            time_limit: (tunable!("ENOMOTO_T_MIP_MINREL_TIME_FRAC", 0.1, f64) * remaining).min(10.0),
            node_limit: 1000,
            rel_gap: 0.0,
            abs_gap: 1e-9,
            verbose: false,
            submip: true,
            cutoff: f64::INFINITY,
            restarts: 0,
            skip_heurs: 0,
            threads: 1,
            distinguish: false,
            fc_submip: false,
        };
        let r = super::solve_problem(&sub, params, env_str!("ENOMOTO_MIP_SUBMIP_NO_PRESOLVE").is_none());
        self.heur_iters += r.lp_iterations;
        let Some(xs) = r.x else {
            if self.params.verbose {
                eprintln!("MIP:   min relaxation: no point ({:?})", r.status);
            }
            return false;
        };
        let viol: f64 = xs[n..].iter().sum();
        let x: Vec<f64> = xs[..n].to_vec();
        if self.params.verbose {
            eprintln!("MIP:   min relaxation: status {:?}, nodes {}, total violation {viol:.3e}", r.status, r.nodes);
        }
        if self.try_incumbent(x.clone()) {
            return true;
        }
        // 違反が残った: 整数列の値を目標に固定・伝播し、連続部分を LP で解いて直す
        let ints: Vec<usize> = (0..n).filter(|&j| p.is_int[j]).collect();
        self.fix_and_propagate(&x, &ints)
    }

    /// LP face (SCIP の heur_lpface): 根の LP で被約費用が 0 でない非基底の列 (整数・連続とも) をその境界に固定し、
    /// LP の最適面に限ったサブ MIP を解く (そこで見つかる整数解は根の LP 値と同じ目的値なので最適)。
    /// LP は根の最適解の状態で呼ぶ。
    pub(super) fn lp_face(&mut self) -> bool {
        let p = self.p;
        if self.params.submip {
            return false;
        }
        let b = self.lp.basis();
        let d = self.lp.reduced_costs();
        let dmax = d.iter().fold(0.0f64, |m, v| m.max(v.abs())).max(1.0);
        let mut lo = self.dom.lo.clone();
        let mut up = self.dom.up.clone();
        let (mut nint, mut nfix) = (0usize, 0usize);
        for j in 0..p.n {
            if p.is_int[j] && lo[j] < up[j] {
                nint += 1;
            }
            let fixv = match b.col[j] {
                super::lp::VarStatus::Lower if d[j] > 1e-7 * dmax => lo[j],
                super::lp::VarStatus::Upper if d[j] < -1e-7 * dmax => up[j],
                _ => continue,
            };
            if !fixv.is_finite() {
                continue;
            }
            if p.is_int[j] && lo[j] < up[j] {
                nfix += 1;
            }
            lo[j] = fixv;
            up[j] = fixv;
        }
        if nint == 0 || (nfix as f64) < tunable!("ENOMOTO_T_MIP_LPFACE_RATE", 0.3, f64) * nint as f64 {
            if self.params.verbose && self.nodes <= 1 {
                eprintln!("MIP:   LP face: only {nfix} of {nint} integer columns fixed");
            }
            return false;
        }
        let r = self.solve_submip(lo, up, 500);
        if self.params.verbose && self.nodes <= 1 {
            eprintln!("MIP:   LP face: {nfix} of {nint} integer columns fixed, found {r}");
        }
        r
    }

    /// 局所探索 ([`super::heuristics::local_search`]) を根で回す。見つからなければ、違反の最小だった点を
    /// `self.repair_start` に残す (Repair の出発点)。
    /// 局所探索を別スレッドで始める (`ENOMOTO_MIP_PAR_LOCAL_SEARCH`)。引数は [`Self::local_search_heur`] と同じ。
    pub(super) fn spawn_local_search(&mut self) {
        let p = self.p;
        let cap = if self.params.time_limit.is_finite() { (tunable!("ENOMOTO_T_MIP_LS_TIME_FRAC", 0.05, f64) * self.params.time_limit).min(5.0) } else { 5.0 };
        let nnz: u64 = p.rows.iter().map(|r| r.len() as u64).sum();
        let seed = self.rng ^ 0xD1B5_4A32_D192_ED03;
        self.rand();
        let (q, lo, up) = match self.orig.clone() {
            Some(o) => ((*o).clone(), o.col_lo.clone(), o.col_up.clone()),
            None => (p.clone(), self.dom.lo.clone(), self.dom.up.clone()),
        };
        let steps = (200 * nnz).clamp(200_000, 100_000_000);
        self.ls_thread = Some(std::thread::spawn(move || super::heuristics::local_search(&q, &lo, &up, cap, steps, seed)));
    }

    pub(super) fn local_search_heur(&mut self) -> bool {
        let p = self.p;
        if self.params.submip {
            return false;
        }
        if let Some(h) = self.ls_thread.take() {
            let Ok((feas, inf)) = h.join() else { return false };
            if let Some((x, v)) = inf {
                if self.params.verbose && self.nodes <= 1 {
                    eprintln!("MIP:   local search (parallel): least violation {v:.3e}");
                }
                self.repair_start = Some(x);
            }
            return match feas {
                Some(x) => self.try_incumbent(x),
                None => false,
            };
        }
        let cap = if self.params.time_limit.is_finite() { (tunable!("ENOMOTO_T_MIP_LS_TIME_FRAC", 0.05, f64) * self.params.time_limit).min(5.0) } else { 5.0 };
        let nnz: u64 = p.rows.iter().map(|r| r.len() as u64).sum();
        let seed = self.rng ^ 0xD1B5_4A32_D192_ED03;
        self.rand();
        let (feas, inf) = match self.orig.clone() {
            Some(o) => super::heuristics::local_search(&o, &o.col_lo, &o.col_up, cap, (200 * nnz).clamp(200_000, 100_000_000), seed),
            None => super::heuristics::local_search(p, &self.dom.lo, &self.dom.up, cap, (200 * nnz).clamp(200_000, 100_000_000), seed),
        };
        if let Some((x, v)) = inf {
            if self.params.verbose && self.nodes <= 1 {
                eprintln!("MIP:   local search: least violation {v:.3e}");
            }
            self.repair_start = Some(x);
        }
        match feas {
            Some(x) => self.try_incumbent(x),
            None => false,
        }
    }

    /// Repair (SCIP の heur_repair): 実行不能な点 `x0` (局所探索の違反最小の点、なければ丸めた LP 解) から、違反している
    /// 行にだけ違反量の列を足し、その行に現れない整数列を `x0` の値に固定した補助 MIP で違反量の合計を最小にする。
    /// 違反 0 なら実行可能解、残れば整数列の値を目標に固定・伝播して連続部分を LP で直す。
    pub(super) fn repair(&mut self, xlp: &[f64]) -> bool {
        let p = self.p;
        if self.params.submip || p.m == 0 {
            return false;
        }
        let x0: Vec<f64> = match self.repair_start.take() {
            Some(x) => x,
            None => (0..p.n).map(|j| if p.is_int[j] { xlp[j].round().clamp(self.dom.lo[j], self.dom.up[j]) } else { xlp[j] }).collect(),
        };
        let tol = |b: f64| 1e-6 * (1.0 + b.abs());
        let act: Vec<f64> = p.rows.iter().map(|r| r.iter().map(|&(j, a)| a * x0[j]).sum()).collect();
        let violated: Vec<bool> = (0..p.m).map(|i| act[i] < p.row_lo[i] - tol(p.row_lo[i]) || act[i] > p.row_up[i] + tol(p.row_up[i])).collect();
        let nviol = violated.iter().filter(|&&v| v).count();
        if nviol == 0 {
            return self.try_incumbent(x0);
        }
        // 違反行に現れる列は自由にし、それ以外の整数列は x0 に固定する
        let mut free = vec![false; p.n];
        for i in 0..p.m {
            if violated[i] {
                for &(j, _) in &p.rows[i] {
                    free[j] = true;
                }
            }
        }
        let n = p.n;
        let mut col_lo = self.dom.global_lo.clone();
        let mut col_up = self.dom.global_up.clone();
        for j in 0..n {
            if p.is_int[j] && !free[j] && x0[j] >= col_lo[j] && x0[j] <= col_up[j] {
                col_lo[j] = x0[j];
                col_up[j] = x0[j];
            }
        }
        let mut rows = p.rows.clone();
        let mut cost = vec![0.0; n];
        let mut is_int = p.is_int.clone();
        for (i, r) in rows.iter_mut().enumerate() {
            if !violated[i] {
                continue;
            }
            let scale = r.iter().fold(0.0f64, |m, &(_, a)| m.max(a.abs())).max(1e-9);
            for (bound, sign) in [(p.row_lo[i], 1.0), (p.row_up[i], -1.0)] {
                if bound.is_finite() {
                    r.push((col_lo.len(), sign * scale));
                    col_lo.push(0.0);
                    col_up.push(f64::INFINITY);
                    cost.push(1.0);
                    is_int.push(false);
                }
            }
        }
        let sub = super::problem::MipProblem::from_rows(col_lo, col_up, cost, 0.0, 1.0, is_int, rows, p.row_lo.clone(), p.row_up.clone());
        let remaining = match self.deadline {
            Some(d) => d.saturating_duration_since(std::time::Instant::now()).as_secs_f64(),
            None => f64::INFINITY,
        };
        let params = super::solver::MipParams {
            time_limit: (0.07 * remaining).min(6.0),
            node_limit: 500,
            rel_gap: 0.0,
            abs_gap: 1e-9,
            verbose: false,
            submip: true,
            cutoff: f64::INFINITY,
            restarts: 0,
            skip_heurs: 0,
            threads: 1,
            distinguish: false,
            fc_submip: false,
        };
        let r = super::solve_problem(&sub, params, env_str!("ENOMOTO_MIP_SUBMIP_NO_PRESOLVE").is_none());
        self.heur_iters += r.lp_iterations;
        let Some(xs) = r.x else {
            if self.params.verbose && self.nodes <= 1 {
                eprintln!("MIP:   repair: {nviol} violated rows, no point ({:?})", r.status);
            }
            return false;
        };
        let v: f64 = xs[n..].iter().sum();
        if self.params.verbose && self.nodes <= 1 {
            eprintln!("MIP:   repair: {nviol} violated rows, {} free columns, remaining violation {v:.3e}", free.iter().filter(|&&f| f).count());
        }
        let x: Vec<f64> = xs[..n].to_vec();
        if self.try_incumbent(x.clone()) {
            return true;
        }
        let ints: Vec<usize> = (0..n).filter(|&j| p.is_int[j]).collect();
        self.fix_and_propagate(&x, &ints)
    }

    /// FJ の問題 (別スレッドの FJ が探した問題: 対称性の行を加える前の問題があればそれ) とその列の境界。
    pub(super) fn fj_problem(&self) -> (super::problem::MipProblem, Vec<f64>, Vec<f64>) {
        match &self.orig {
            Some(o) => ((**o).clone(), o.col_lo.clone(), o.col_up.clone()),
            None => (self.p.clone(), self.dom.lo.clone(), self.dom.up.clone()),
        }
    }

    /// FJ 修復 (`ENOMOTO_MIP_FJ_REPAIR`、試作・既定では使わない): FJ の最良の点 `x` (違反の行の数が最も少ない点) で違反している行が
    /// `ENOMOTO_T_MIP_FJ_REPAIR_MAXVIOL` 本以下なら、違反している行に現れる整数列と、それらの列と同じ行に現れる整数列 (1 段、
    /// 短い行から `ENOMOTO_T_MIP_FJ_REPAIR_MAXFREE` 本まで) を自由にし、ほかの整数列を `x` の値に固定したサブ MIP を、元の目的関数・
    /// 元の制約で別スレッドで解く ([`Self::poll_fj_repair`] で受け取る)。実行不能なら同じ固定で違反量の合計を最小にする
    /// ([`Self::repair`] と同じ形)。違反が残れば、その点から段を 1 つ増やし自由にする列の上限を 2 倍にしてやり直す
    /// (`ENOMOTO_T_MIP_FJ_REPAIR_ROUNDS` 回まで、既定 4)。根のカットは通常の求解と同じラウンド数 (`fc_submip` の設定、
    /// `ENOMOTO_MIP_FJ_REPAIR_NOCUTS` でサブ MIP の既定)。暫定解があれば使わない (`ENOMOTO_MIP_FJ_REPAIR=2` なら暫定解を打ち切り値にして使う)。
    pub(super) fn spawn_fj_repair(&mut self, x: Vec<f64>) {
        let Some(mode) = env_str!("ENOMOTO_MIP_FJ_REPAIR") else { return };
        if self.params.submip || self.fjr_thread.is_some() || (self.incumbent.is_some() && mode != "2") {
            return;
        }
        let (pc, lo, up) = self.fj_problem();
        let nviol = viol_stats(&pc, &x).0;
        let maxviol = tunable!("ENOMOTO_T_MIP_FJ_REPAIR_MAXVIOL", 20usize, usize);
        let verbose = self.params.verbose || env_str!("ENOMOTO_MIP_FJ_STATS").is_some();
        if nviol > maxviol {
            if verbose {
                eprintln!("MIP: FJ repair: {nviol} violated rows > {maxviol}, not used");
            }
            return;
        }
        let remaining = self.deadline.map_or(f64::INFINITY, |d| d.saturating_duration_since(std::time::Instant::now()).as_secs_f64());
        let tl = (tunable!("ENOMOTO_T_MIP_FJ_REPAIR_TIME_FRAC", 0.25, f64) * remaining).min(tunable!("ENOMOTO_T_MIP_FJ_REPAIR_TIME_MAX", 15.0, f64));
        let cutoff = if self.incumbent.is_some() { self.prune_limit() } else { f64::INFINITY };
        let params = super::solver::MipParams {
            time_limit: tl,
            node_limit: tunable!("ENOMOTO_T_MIP_FJ_REPAIR_NODES", 2000u64, u64),
            rel_gap: self.params.rel_gap.max(1e-4),
            abs_gap: self.params.abs_gap.max(1e-6),
            verbose: self.params.verbose && env_str!("ENOMOTO_MIP_SUBMIP_VERBOSE").is_some(),
            submip: true,
            cutoff,
            restarts: 0,
            skip_heurs: 0,
            threads: 1,
            distinguish: false,
            fc_submip: env_str!("ENOMOTO_MIP_FJ_REPAIR_NOCUTS").is_none(),
        };
        if verbose {
            eprintln!("MIP: FJ repair started: {nviol} violated rows, time limit {tl:.1}s ({:.2}s)", self.start.elapsed().as_secs_f64());
        }
        let presolve = env_str!("ENOMOTO_MIP_SUBMIP_NO_PRESOLVE").is_none();
        let rounds = tunable!("ENOMOTO_T_MIP_FJ_REPAIR_ROUNDS", 4usize, usize).max(1);
        let maxfree0 = tunable!("ENOMOTO_T_MIP_FJ_REPAIR_MAXFREE", 2000usize, usize);
        let h = std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || {
                super::heuristics::SUBMIP_DEPTH.with(|d| d.set(1));
                let t0 = std::time::Instant::now();
                let mut x = x;
                let mut log = String::new();
                for r in 0..rounds {
                    let left = params.time_limit - t0.elapsed().as_secs_f64();
                    if left <= 0.2 {
                        break;
                    }
                    let Some((sub, vsub, info)) = fj_repair_problems(&pc, &lo, &up, &x, r + 1, maxfree0 << r) else {
                        // 違反なし (丸めた点が実行可能)
                        return (Some(x), log + "rounded point feasible");
                    };
                    log += &format!("[round {r}: {info}] ");
                    let res = super::solve_problem(&sub, super::solver::MipParams { time_limit: left, ..params }, presolve);
                    log += &format!("objective sub-MIP {:?} {:?} nodes {} ({:.2}s); ", res.status, res.objective, res.nodes, t0.elapsed().as_secs_f64());
                    if res.x.is_some() || !cutoff.is_infinite() || res.status != super::solver::MipStatus::Infeasible {
                        return (res.x, log);
                    }
                    let left = (params.time_limit - t0.elapsed().as_secs_f64()).max(0.2);
                    let p2 = super::solver::MipParams { time_limit: left, rel_gap: 0.0, abs_gap: 1e-9, fc_submip: false, ..params };
                    let r2 = super::solve_problem(&vsub, p2, presolve);
                    let n = pc.n;
                    let v = r2.x.as_ref().map(|xs| xs[n..].iter().sum::<f64>());
                    log += &format!("violation sub-MIP {:?} remaining {v:?} ({:.2}s); ", r2.status, t0.elapsed().as_secs_f64());
                    match r2.x {
                        Some(xs) if v.is_some_and(|v| v <= 1e-9) => return (Some(xs[..n].to_vec()), log),
                        Some(xs) => x = xs[..n].to_vec(),
                        None => break,
                    }
                }
                (None, log)
            })
            .expect("failed to spawn the FJ repair thread");
        self.fjr_thread = Some(h);
    }
}

/// FJ 修復の 1 回分のサブ MIP ([`Solver::spawn_fj_repair`]): 点 `x` で違反している行の整数列と、そこから `hops` 段までの行の整数列
/// (段ごとに短い行から、合わせて `maxfree` 本まで) を自由にし、ほかの整数列を `x` に固定した (元の目的関数の問題, 違反量を最小にする問題, ログ)。
/// 違反している行がなければ `None`。
fn fj_repair_problems(pc: &super::problem::MipProblem, lo: &[f64], up: &[f64], x: &[f64], hops: usize, maxfree: usize) -> Option<(super::problem::MipProblem, super::problem::MipProblem, String)> {
    let (n, m) = (pc.n, pc.m);
    let tol = |b: f64| 1e-6 * (1.0 + b.abs());
    let xr: Vec<f64> = (0..n).map(|j| if pc.is_int[j] { x[j].round().clamp(lo[j], up[j]) } else { x[j].clamp(lo[j], up[j]) }).collect();
    let act: Vec<f64> = pc.rows.iter().map(|r| r.iter().map(|&(j, a)| a * xr[j]).sum()).collect();
    let violated: Vec<bool> = (0..m).map(|i| act[i] < pc.row_lo[i] - tol(pc.row_lo[i]) || act[i] > pc.row_up[i] + tol(pc.row_up[i])).collect();
    let nviol = violated.iter().filter(|&&v| v).count();
    if nviol == 0 {
        return None;
    }
    let nint = pc.is_int.iter().filter(|&&b| b).count();
    let mut free = vec![false; n];
    let mut used = violated.clone();
    let mut nfree = 0usize;
    for i in 0..m {
        if violated[i] {
            for &(j, _) in &pc.rows[i] {
                if pc.is_int[j] && !free[j] {
                    free[j] = true;
                    nfree += 1;
                }
            }
        }
    }
    let nfree0 = nfree;
    'hops: for _ in 0..hops {
        let core = free.clone();
        let mut near: Vec<usize> = (0..m).filter(|&i| !used[i] && pc.rows[i].iter().any(|&(j, _)| core[j])).collect();
        near.sort_by_key(|&i| pc.rows[i].len());
        for i in near {
            used[i] = true;
            for &(j, _) in &pc.rows[i] {
                if nfree >= nfree0.max(maxfree) {
                    break 'hops;
                }
                if pc.is_int[j] && !free[j] {
                    free[j] = true;
                    nfree += 1;
                }
            }
        }
    }
    let mut col_lo = lo.to_vec();
    let mut col_up = up.to_vec();
    for j in 0..n {
        if pc.is_int[j] && !free[j] {
            col_lo[j] = xr[j];
            col_up[j] = xr[j];
        }
    }
    // 違反量の最小化の問題 (違反している行にだけ違反量の列を足す。この固定のままで `x` が実行可能)
    let mut rows = pc.rows.clone();
    let (mut vlo, mut vup) = (col_lo.clone(), col_up.clone());
    let mut cost = vec![0.0; n];
    let mut is_int = pc.is_int.clone();
    for (i, r) in rows.iter_mut().enumerate() {
        if !violated[i] {
            continue;
        }
        let scale = r.iter().fold(0.0f64, |mx, &(_, a)| mx.max(a.abs())).max(1e-9);
        for (bound, sign) in [(pc.row_lo[i], 1.0), (pc.row_up[i], -1.0)] {
            if bound.is_finite() {
                r.push((vlo.len(), sign * scale));
                vlo.push(0.0);
                vup.push(f64::INFINITY);
                cost.push(1.0);
                is_int.push(false);
            }
        }
    }
    let vsub = super::problem::MipProblem::from_rows(vlo, vup, cost, 0.0, 1.0, is_int, rows, pc.row_lo.clone(), pc.row_up.clone());
    let mut sub = pc.clone();
    sub.col_lo = col_lo;
    sub.col_up = col_up;
    Some((sub, vsub, format!("{nviol} violated rows, free integer columns {nfree} of {nint} ({nfree0} in the violated rows)")))
}
