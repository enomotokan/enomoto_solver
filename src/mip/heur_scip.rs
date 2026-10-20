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
        // `ENOMOTO_MIP_LPFACE_TIGHT` (試作): サブ MIP の打ち切り値を根の LP の値 (目的関数の刻みで切り上げたもの) にし、
        // 固定率の下限を `ENOMOTO_T_MIP_LPFACE_TIGHT_RATE` (既定 0.1) に下げ、サブ MIP の時間を残りの
        // `ENOMOTO_T_MIP_LPFACE_TIME_FRAC` (既定 0.07、通常と同じ) にする。最適面の上の整数解だけを探すので、暫定解 (FJ の
        // 質の悪い解) の打ち切り値で LP 値の上の方まで探すより速く終わる。10teams では、被約費用が 0 でない整数列が
        // 回によって 440〜535 本 (固定率 0.28〜0.33) と揺れ、既定の下限 0.3 で LP face が動くかどうかが分かれていた
        // (analysis/10teams_20261020.md)
        let tight = env_str!("ENOMOTO_MIP_LPFACE_TIGHT").is_some();
        let min_rate = if tight { tunable!("ENOMOTO_T_MIP_LPFACE_TIGHT_RATE", 0.1, f64) } else { tunable!("ENOMOTO_T_MIP_LPFACE_RATE", 0.3, f64) };
        if nint == 0 || (nfix as f64) < min_rate * nint as f64 {
            if self.params.verbose && self.nodes <= 1 {
                eprintln!("MIP:   LP face: only {nfix} of {nint} integer columns fixed");
            }
            return false;
        }
        let r = if tight {
            let lb = self.lp.objective() + p.offset;
            let target = match self.obj_step {
                Some(step) => step * ((lb - p.offset) / step - 1e-6).ceil() + p.offset + (1e-6 * lb.abs().max(1.0)).min(0.5 * step),
                None => lb + 1e-6 * lb.abs().max(1.0),
            };
            let cutoff = target.min(self.prune_limit());
            let mut sub = p.clone();
            sub.col_lo = lo;
            sub.col_up = up;
            let keep = self.submip_time_frac;
            self.submip_time_frac = tunable!("ENOMOTO_T_MIP_LPFACE_TIME_FRAC", 0.07, f64);
            if self.params.verbose && self.nodes <= 1 {
                eprintln!("MIP:   LP face (tight): sub-MIP cutoff {cutoff:.10e} (root LP {lb:.10e})");
            }
            let r = self.solve_submip_problem(sub, 500, Some(cutoff)).is_some_and(|r| r.1);
            self.submip_time_frac = keep;
            r
        } else {
            self.solve_submip(lo, up, 500)
        };
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
}
