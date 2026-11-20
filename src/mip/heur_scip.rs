//! SCIP 由来の主ヒューリスティクス (Crossover・DINS・locks・clique・vbounds) と解のプール。

use super::cuts::VarBounds;
use super::domain::Domain;
use super::problem::MipProblem;
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
        // 試作 `ENOMOTO_MIP_GAP4` の `k`: locks の固定の伝播に前処理の含意 (`conflict_edges`) も使う (写しの定義域で固定し、終わったら戻す)。
        // 行の伝播だけでは、`q` の前処理が含意で見つけた矛盾を見落とし、サブ MIP の前処理ですぐ実行不能になっていた (neos-3216931-puriri)
        let saved_dom = if label == "locks" && super::solver::gap4('k') && !p.conflict_edges.is_empty() {
            let mut d = self.dom.clone();
            d.add_implications(p.n, &p.conflict_edges);
            if !d.propagate(p) {
                return false;
            }
            Some(std::mem::replace(&mut self.dom, d))
        } else {
            None
        };
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
        match saved_dom {
            Some(d) => self.dom = d,
            None => self.dom.backtrack_to(p, pos),
        }
        if label == "locks" {
            self.gap4_locks.0 = fr;
        }
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
        self.locks_heur_with(0.65)
    }

    /// 試作 `ENOMOTO_MIP_GAP4` の `k` (既定では使わない。analysis/gap4_rootstuck_mem_20261109.md): 根の並列の locks のサブ MIP が根で実行不能
    /// (ノード 1 以下) だったら、並列のサブ MIP を受け取った後に主のスレッドで固定率の目標を「達した固定率 × 0.5」にしてやり直す
    /// (`SUBFIX` の RENS のやり直しと同じ形)。neos-3216931-puriri では `q` で締まった問題の locks の固定 (固定率 0.73) が
    /// サブ MIP の前処理ですぐ実行不能になり、既定で 81480 を出す locks が何も出さなかった
    pub(super) fn locks_retry(&mut self) -> bool {
        let fr = std::mem::replace(&mut self.gap4_locks, (0.0, false)).0;
        if self.time_up() || 0.5 * fr < 0.3 {
            return false;
        }
        let keep = self.parallel_submips;
        self.parallel_submips = false;
        if self.params.verbose {
            eprintln!("MIP:   GAP4 k: locks sub-MIP was infeasible at the root at fixing rate {fr:.2}; retrying on the main thread with target {:.2}", 0.5 * fr);
        }
        let r = self.locks_heur_with(0.5 * fr);
        self.parallel_submips = keep;
        r
    }

    fn locks_heur_with(&mut self, stop_rate: f64) -> bool {
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
        self.fix_then_submip(&fixes, stop_rate, stop_rate.min(0.3), "locks")
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

    /// 試作 `ENOMOTO_MIP_VB01` (既定では使わない。analysis/vb01_20261120/README.md): 0-1 列どうしの含意を使う vbounds
    /// (SCIP の heur_vbounds がクリーク表を変数上下限のグラフに入れる形をなぞる)。衝突グラフ ([`Self::clique_graph_get`]、行から作る組と
    /// 前処理の含意) の辺 a—b (同時には真にならない文字) を含意 a ⇒ ¬b と読む。境界の節点 (文字 `2j` = `x_j` の下限を 1 に上げる、
    /// `2j+1` = 上限を 0 に下げる) の逆向きのグラフ (a の後続は a を導く文字 = ¬a の隣) を、節点の番号の順に深さ優先でたどった帰りがけ順
    /// (SCIP の topologicalSort と同じ。辺のない節点だけの木は入れない) に、締める型では節点の境界を動かす側 (文字を真)、緩める型では
    /// 動かさない側 (文字を偽) に固定して伝播する。矛盾したら直前の列を反対の値にし、それも矛盾すればその型はやめる (反対にした回数が
    /// 10 を超えたら固定をやめる)。固定率が 0.65 以上ならサブ MIP (LP から) を解く。型の順は SCIP の既定と同じ: 緩める型の目的の良い側だけ・
    /// 悪い側だけ、締める型の目的を見ない・良い側だけ・悪い側だけ (費用 >= 0 なら 0 が良い側。目的で選ばない列は固定せず残す)。
    /// 手間: グラフの辺の数に比例する並べ替えと、型ごとの伝播 (`fix_then_submip` と同じ手間の上限) とサブ MIP (500 ノード)。
    pub(super) fn vbounds01_heur(&mut self) -> bool {
        let p = self.p;
        if self.params.submip {
            return false;
        }
        let g = self.clique_graph_get();
        let fb: Vec<bool> = (0..p.n).map(|j| p.is_int[j] && self.dom.lo[j] == 0.0 && self.dom.up[j] == 1.0).collect();
        let ints: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j] && self.dom.lo[j] < self.dom.up[j]).collect();
        let cliques = if vb01_clq() { vb01_cliques(p, &self.dom.lo, &self.dom.up, &fb) } else { Default::default() };
        let order = vb01_order(&g, &fb, &cliques);
        // SCIP: 節点の数が整数列の数 × 0.65 に届かなければ使わない
        if ints.is_empty() || (order.len() as f64) < 0.65 * ints.len() as f64 {
            if self.params.verbose && self.nodes <= 1 {
                eprintln!("MIP:   VB01: {} bound nodes in the implication graph < 0.65 x {} free integer columns: not used", order.len(), ints.len());
            }
            return false;
        }
        // 前処理の含意 (`conflict_edges`) も伝播に使う (写しの定義域で固定する)
        let mut dom = self.dom.clone();
        if !p.conflict_edges.is_empty() {
            dom.add_implications(p.n, &p.conflict_edges);
            if !dom.propagate(p) {
                return false;
            }
        }
        let mut found = false;
        let (mut skip1, mut skip2) = (false, false);
        for (tight, obj) in VB01_VARIANTS {
            if self.time_up() || (obj == 1 && skip1) || (obj == 2 && skip2) {
                continue;
            }
            let deadline = self.deadline;
            let stop = || deadline.is_some_and(|d| std::time::Instant::now() >= d);
            let (bounds, all1, all2, log) = vb01_fix_dom(p, &mut dom, &order, tight, obj, &ints, &stop);
            if self.params.verbose && self.nodes <= 1 {
                eprintln!("MIP:   {log}");
            }
            if obj == 0 {
                // 目的を見ない型の固定がすべて良い側 (悪い側) だったら、その型は同じなので回さない (SCIP の skipobj)
                skip1 = all1;
                skip2 = all2;
            }
            if let Some((lo, up)) = bounds {
                found |= self.solve_submip(lo, up, 500);
            }
        }
        found
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
            tl_base: f64::NAN,
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
        // 試作 `ENOMOTO_MIP_LPFACE` (既定では使わない、analysis/scip_rens_20261120): SCIP の heur_lpface の形
        // (目的値 = LP の下界の等式を足し、面の上の実行可能解だけを探す)
        if let Some(v) = env_str!("ENOMOTO_MIP_LPFACE").filter(|v| !v.is_empty() && *v != "0") {
            return self.lp_face_obj(lo, up, nfix, nint, v);
        }
        let r = self.solve_submip(lo, up, 500);
        if self.params.verbose && self.nodes <= 1 {
            eprintln!("MIP:   LP face: {nfix} of {nint} integer columns fixed, found {r}");
        }
        r
    }

    /// 試作 `ENOMOTO_MIP_LPFACE=<文字の並び>` の LP face のサブ MIP。`lo`・`up` は面の固定の後の境界。
    /// 目的値の行 c·x ≤ z̄ (z̄ は今の LP の値を目的値の刻みに切り上げたもの) を足し、打ち切り値も z̄ のすぐ上にする
    /// (z̄ は根の下界なので、見つかる解は最適)。
    /// - `c`: この形を使う (目的は元のまま)
    /// - `z`: サブ MIP の目的を 0 にする (SCIP の既定 `subscipobjective = z`。最初の実行可能解で止まる)
    /// - `f`: 根で LP face を根の被約費用のサブ MIP より先に回す (solver.rs の根のヒューリスティクスの並び)
    fn lp_face_obj(&mut self, lo: Vec<f64>, up: Vec<f64>, nfix: usize, nint: usize, v: &str) -> bool {
        let p = self.p;
        let z = self.lp.objective() + p.offset;
        // 目的値の刻みがあれば切り上げ、打ち切り値は刻みの半分の上。なければ相対 1e-6 の上
        let (zbar, cut) = match self.obj_step {
            Some(s) if s > 0.0 => {
                let zb = p.offset + ((z - p.offset) / s - 1e-6).ceil() * s;
                (zb, zb + 0.5 * s)
            }
            _ => (z, z + 1e-6 * z.abs().max(1.0)),
        };
        let mut sub = p.clone();
        sub.col_lo = lo;
        sub.col_up = up;
        let objrow: Vec<(usize, f64)> = (0..p.n).filter(|&j| p.cost[j] != 0.0).map(|j| (j, p.cost[j])).collect();
        let mut rows = std::mem::take(&mut sub.rows);
        let mut row_lo = std::mem::take(&mut sub.row_lo);
        let mut row_up = std::mem::take(&mut sub.row_up);
        if !objrow.is_empty() {
            rows.push(objrow);
            row_lo.push(f64::NEG_INFINITY);
            row_up.push(cut - p.offset);
        }
        let zero = v.contains('z');
        let cost = if zero { vec![0.0; p.n] } else { std::mem::take(&mut sub.cost) };
        let mut s2 = super::problem::MipProblem::from_rows(std::mem::take(&mut sub.col_lo), std::mem::take(&mut sub.col_up), cost, if zero { 0.0 } else { sub.offset }, sub.sense_sign, std::mem::take(&mut sub.is_int), rows, row_lo, row_up);
        s2.conflict_edges = std::mem::take(&mut sub.conflict_edges);
        s2.implied_bounds = std::mem::take(&mut sub.implied_bounds);
        let cutoff = if zero { f64::INFINITY } else { cut.min(self.prune_limit()) };
        let r = self.solve_submip_problem(s2, 500, Some(cutoff));
        let found = r.as_ref().is_some_and(|r| r.1);
        if self.params.verbose && self.nodes <= 1 {
            eprintln!("MIP:   LP face ({v}): {nfix} of {nint} integer columns fixed, objective row <= {zbar:.6e} (LP {z:.6e}), status {:?}, found {found}", r.map(|r| r.0));
        }
        found
    }

    /// 局所探索 ([`super::heuristics::local_search`]) を根で回す。見つからなければ、違反の最小だった点を
    /// `self.repair_start` に残す (Repair の出発点)。
    /// 局所探索を別スレッドで始める (`ENOMOTO_MIP_PAR_LOCAL_SEARCH`)。引数は [`Self::local_search_heur`] と同じ。
    pub(super) fn spawn_local_search(&mut self) {
        let p = self.p;
        let cap = self.ls_time_cap();
        let nnz: u64 = p.rows.iter().map(|r| r.len() as u64).sum();
        let seed = self.rng ^ 0xD1B5_4A32_D192_ED03;
        self.rand();
        let (q, lo, up) = match self.orig.clone() {
            Some(o) => ((*o).clone(), o.col_lo.clone(), o.col_up.clone()),
            None => (p.clone(), self.dom.lo.clone(), self.dom.up.clone()),
        };
        let steps = (200 * nnz).clamp(200_000, 100_000_000);
        let mw = self.ls_max_work();
        self.ls_thread = Some(std::thread::spawn(move || {
            super::heuristics::LS_MAX_WORK.with(|c| c.set(mw));
            super::heuristics::local_search(&q, &lo, &up, cap, steps, seed)
        }));
    }

    /// 局所探索の時間の枠 (上限の `ENOMOTO_T_MIP_LS_TIME_FRAC` (0.05) 倍、最大 5 秒)。`ENOMOTO_MIP_OVERRUN3` の `h` (試作): 期限までの残りで
    /// 頭打ちにする (app1-2 では期限の 1 秒前に始めた局所探索が 3 秒の枠を使い切り、期限を 2 秒過ぎていた)
    /// 試作 `ENOMOTO_MIP_TIMEDEP` の `l`: 局所探索の手間 (触った非零の数) の上限 = `ENOMOTO_T_MIP_TIMEDEP_LS_RATE` (既定 1.5e8、
    /// この記録の機械で 1 秒に触る数。10teams で 3 秒 4.5e8) x 今の時間の枠の秒。使わないなら無制限
    fn ls_max_work(&self) -> u64 {
        if super::timedep('l') {
            (tunable!("ENOMOTO_T_MIP_TIMEDEP_LS_RATE", 1.5e8, f64) * self.ls_base_cap()) as u64
        } else {
            u64::MAX
        }
    }

    fn ls_base_cap(&self) -> f64 {
        if self.params.frac_tl().is_finite() { (tunable!("ENOMOTO_T_MIP_LS_TIME_FRAC", 0.05, f64) * self.params.frac_tl()).min(5.0) } else { 5.0 }
    }

    fn ls_time_cap(&self) -> f64 {
        let cap = self.ls_base_cap();
        // `ENOMOTO_MIP_TIMEDEP` の `l`: 時間は 10 倍の安全網 (止めるのは手間)
        let cap = if super::timedep('l') { 10.0 * cap } else { cap };
        match self.deadline.filter(|_| super::zerohalf::overrun3('h')) {
            Some(d) => cap.min(d.saturating_duration_since(std::time::Instant::now()).as_secs_f64()),
            None => cap,
        }
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
        let cap = self.ls_time_cap();
        let nnz: u64 = p.rows.iter().map(|r| r.len() as u64).sum();
        let seed = self.rng ^ 0xD1B5_4A32_D192_ED03;
        self.rand();
        super::heuristics::LS_MAX_WORK.with(|c| c.set(self.ls_max_work()));
        let (feas, inf) = match self.orig.clone() {
            Some(o) => super::heuristics::local_search(&o, &o.col_lo, &o.col_up, cap, (200 * nnz).clamp(200_000, 100_000_000), seed),
            None => super::heuristics::local_search(p, &self.dom.lo, &self.dom.up, cap, (200 * nnz).clamp(200_000, 100_000_000), seed),
        };
        super::heuristics::LS_MAX_WORK.with(|c| c.set(u64::MAX));
        if self.params.verbose && self.nodes <= 1 {
            let (st, w, bt) = super::heuristics::LS_LAST.with(|c| c.get());
            eprintln!("MIP:   local search: {st} steps, work {w} ({:.1} x nnz {nnz}), stopped by time {bt}, cap {cap:.2}s", w as f64 / nnz.max(1) as f64);
        }
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
            tl_base: f64::NAN,
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

/// 試作 `ENOMOTO_MIP_VB01` の `c`: 位相順に行のクリーク ([`vb01_cliques`]) も使う (衝突グラフで落ちる長い集合分割・詰め込みの行を入れる)
fn vb01_clq() -> bool {
    env_str!("ENOMOTO_MIP_VB01").is_some_and(|v| v.contains('c'))
}

/// [`Solver::vbounds01_heur`] の型 (締めるか, 目的: 0 見ない・1 良い側だけ・2 悪い側だけ)。SCIP の既定 (feasvariant 6・tightenvariant 7) の順。
const VB01_VARIANTS: [(bool, u8); 5] = [(false, 1), (false, 2), (true, 0), (true, 1), (true, 2)];

fn vb01_label(tight: bool, obj: u8) -> &'static str {
    match (tight, obj) {
        (true, 0) => "VB01 (tight)",
        (true, 1) => "VB01 (tight, best)",
        (true, _) => "VB01 (tight, worst)",
        (false, 1) => "VB01 (loose, best)",
        (false, _) => "VB01 (loose, worst)",
    }
}

/// VB01 のクリーク (SCIP のクリーク表に当たる): 行のうち、0-1 列の文字 (係数が正なら `x = 1`、負なら `x = 0`) のどの 2 つも同時には真に
/// なれない行 (重みの小さい 2 つの和が余裕を超える。集合分割・詰め込みなど)。衝突グラフ ([`super::clique::CliqueGraph`]) は組の辺で持ち、
/// 文字 2000 を超える行・辺 2 万を超える行を落とすので、長い行の含意は入らない (nw04)。クリークは 1 つの物として持ち、深さ優先では
/// SCIP と同じく 1 つのクリークを 1 度だけ展開する ([`vb01_order`])。`lo`・`up` は列の境界。(クリークの文字の並び, 文字ごとのクリークの番号)
pub(super) fn vb01_cliques(p: &MipProblem, lo: &[f64], up: &[f64], fb: &[bool]) -> (Vec<Vec<u32>>, Vec<Vec<u32>>) {
    let mut cl: Vec<Vec<u32>> = Vec::new();
    let mut of_lit: Vec<Vec<u32>> = vec![Vec::new(); 2 * p.n];
    for (i, r) in p.rows.iter().enumerate() {
        if r.len() < 3 {
            continue;
        }
        for (sign, b) in [(1.0, p.row_up[i]), (-1.0, -p.row_lo[i])] {
            if !b.is_finite() {
                continue;
            }
            let mut minact = 0.0;
            let mut lits: Vec<(f64, u32)> = Vec::new();
            let mut ok = true;
            for &(j, a0) in r {
                let a = sign * a0;
                let m = if a > 0.0 { a * lo[j] } else { a * up[j] };
                if !m.is_finite() {
                    ok = false;
                    break;
                }
                minact += m;
                if fb[j] && a != 0.0 {
                    lits.push((a.abs(), if a > 0.0 { 2 * j as u32 } else { 2 * j as u32 + 1 }));
                }
            }
            if !ok || lits.len() < 3 {
                continue;
            }
            let slack = b - minact;
            let tol = 1e-9 * (1.0 + b.abs());
            // 重みの小さい 2 つでも余裕を超えれば、どの 2 つも同時には真になれない
            let (mut w1, mut w2) = (f64::INFINITY, f64::INFINITY);
            for &(w, _) in &lits {
                if w < w1 {
                    w2 = w1;
                    w1 = w;
                } else if w < w2 {
                    w2 = w;
                }
            }
            if w1 + w2 <= slack + tol {
                continue;
            }
            let id = cl.len() as u32;
            for &(_, l) in &lits {
                of_lit[l as usize].push(id);
            }
            cl.push(lits.into_iter().map(|(_, l)| l).collect());
        }
    }
    (cl, of_lit)
}

/// VB01 の位相順 (SCIP の topologicalSort): 境界の節点 (文字 `2j` = `x_j` の下限、`2j+1` = 上限) の逆向きのグラフ (a の後続は a を導く文字 =
/// ¬a の衝突グラフの隣と、¬a を含むクリークの他の文字) を、節点の番号の順に深さ優先でたどった帰りがけ順。辺のない節点だけの木は入れない。
/// クリークは SCIP の cliqueexit と同じく、最初に入った節点 u から全員を展開し、2 度目からは u の反対の文字 (u の展開に入らない唯一の
/// 文字) だけを見る (長いクリークでも手間は長さに比例)。`fb[j]`: 自由な 0-1 列か。`cliques`: [`vb01_cliques`] (空でもよい)。
pub(super) fn vb01_order(g: &super::clique::CliqueGraph, fb: &[bool], cliques: &(Vec<Vec<u32>>, Vec<Vec<u32>>)) -> Vec<u32> {
    let n = fb.len();
    let (cl, of_lit) = cliques;
    let empty: Vec<u32> = Vec::new();
    // クリークの出口: 0 = まだ入っていない、e + 1 = 最初に入った節点の反対の文字 e、-1 = 出口も使った
    let mut exit: Vec<i64> = vec![0; cl.len()];
    let mut visited = vec![false; 2 * n];
    let mut order: Vec<u32> = Vec::new();
    // (節点, 組の辺の位置, クリークの位置, クリークの中の位置)
    let mut stack: Vec<(u32, usize, usize, usize)> = Vec::new();
    for s0 in 0..2 * n as u32 {
        if visited[s0 as usize] || !fb[(s0 / 2) as usize] {
            continue;
        }
        stack.push((s0, 0, 0, 0));
        let mut maxstack = 1usize;
        while let Some(top) = stack.last_mut() {
            let a = top.0;
            visited[a as usize] = true;
            let mut next = None;
            let nb = g.neighbors(a ^ 1);
            while top.1 < nb.len() {
                let b = nb[top.1];
                top.1 += 1;
                if fb[(b / 2) as usize] && !visited[b as usize] {
                    next = Some(b);
                    break;
                }
            }
            if next.is_none() {
                let cs = of_lit.get((a ^ 1) as usize).unwrap_or(&empty);
                while top.2 < cs.len() {
                    let c = cs[top.2] as usize;
                    if top.3 == 0 {
                        // このクリークに入るのは初めてか
                        if exit[c] > 0 {
                            let e = (exit[c] - 1) as u32;
                            exit[c] = -1;
                            top.2 += 1;
                            if e != a && !visited[e as usize] && fb[(e / 2) as usize] {
                                next = Some(e);
                                break;
                            }
                            continue;
                        } else if exit[c] == 0 {
                            exit[c] = (a ^ 1) as i64 + 1;
                        } else {
                            top.2 += 1;
                            continue;
                        }
                    }
                    let mem = &cl[c];
                    while top.3 < mem.len() {
                        let b = mem[top.3];
                        top.3 += 1;
                        if b != (a ^ 1) && fb[(b / 2) as usize] && !visited[b as usize] {
                            next = Some(b);
                            break;
                        }
                    }
                    if next.is_some() {
                        break;
                    }
                    top.2 += 1;
                    top.3 = 0;
                }
            }
            if let Some(b) = next {
                stack.push((b, 0, 0, 0));
                maxstack = maxstack.max(stack.len());
                continue;
            }
            stack.pop();
            if maxstack > 1 {
                order.push(a);
            } else {
                visited[a as usize] = false;
            }
        }
    }
    order
}

/// VB01 の 1 つの型の固定・伝播 (SCIP の applyVboundsFixings): `order` の順に、締める型は節点の文字を真、緩める型は偽に固定して伝播する
/// (目的で選ばない列は飛ばす。費用 >= 0 なら 0 が良い側)。矛盾したら直前の列を反対の値にし、それも矛盾すればやめる (反対にした回数が 10 を
/// 超えたら固定をやめる)。整数列 (`ints`) の固定率と全列の固定率 (SCIP の minintfixingrate・minmipfixingrate) がともに 0.65 以上なら、
/// 固定した後の境界を返す。(境界, 固定がすべて目的の良い側だったか, すべて悪い側だったか, ログの 1 行)。`dom` は戻す。
/// 手間の上限は伝播で非零数 × 20 + 100 万 ([`Solver::fix_then_submip`] と同じ)。
pub(super) fn vb01_fix_dom(p: &MipProblem, dom: &mut Domain, order: &[u32], tight: bool, obj: u8, ints: &[usize], stop: &dyn Fn() -> bool) -> (Option<(Vec<f64>, Vec<f64>)>, bool, bool, String) {
    let (mut all1, mut all2) = (true, true);
    let pos = dom.stack_len();
    let work0 = dom.debug_work();
    let work_cap = 20 * p.rows.iter().map(|r| r.len() as u64).sum::<u64>() + 1_000_000;
    let (mut nback, mut dead, mut nfix) = (0usize, false, 0usize);
    for (k, &a) in order.iter().enumerate() {
        if k % 64 == 0 && (stop() || dom.debug_work() - work0 > work_cap) {
            break;
        }
        let j = (a / 2) as usize;
        if dom.lo[j] + 0.5 > dom.up[j] {
            continue;
        }
        let v = if (a % 2 == 0) == tight { 1.0 } else { 0.0 };
        let best = (p.cost[j] >= 0.0) == (v == 0.0);
        if !best {
            if obj == 1 {
                continue;
            }
            all1 = false;
        }
        if best {
            if obj == 2 {
                continue;
            }
            all2 = false;
        }
        let before = dom.stack_len();
        dom.tighten_lower(p, j, v);
        dom.tighten_upper(p, j, v);
        nfix += 1;
        if dom.propagate(p) {
            continue;
        }
        dom.backtrack_to(p, before);
        nback += 1;
        let w = 1.0 - v;
        dom.tighten_lower(p, j, w);
        dom.tighten_upper(p, j, w);
        if !dom.propagate(p) {
            dead = true;
            break;
        }
        if nback > 10 {
            break;
        }
    }
    let nfree = ints.iter().filter(|&&j| dom.lo[j] < dom.up[j]).count();
    let fr = 1.0 - nfree as f64 / ints.len().max(1) as f64;
    // 全列の固定率は定義域で固定された列で数える (SCIP はサブ問題の前処理の後で数える)。連続列の多い問題 (neos-5188808-nattai) で、
    // 整数列だけ固定してもサブ MIP の LP が大きく時間切れになっていた
    let frall = (0..p.n).filter(|&j| dom.lo[j] == dom.up[j]).count() as f64 / p.n.max(1) as f64;
    let bounds = (!dead && fr >= 0.65 && frall >= 0.65).then(|| (dom.lo.clone(), dom.up.clone()));
    dom.backtrack_to(p, pos);
    let log = format!("{}: {nfix} fixings, {nback} backtracks{}, fixing rate {fr:.2} (all columns {frall:.2})", vb01_label(tight, obj), if dead { ", infeasible" } else { "" });
    (bounds, all1, all2, log)
}

/// [`Solver::vbounds01_heur`] を Solver を作らずに問題 `p` (前処理の後) の上で回す (SCIP-GATE の `ENOMOTO_MIP_VBCLQ=e` が前処理の
/// LP probing の前に呼ぶ)。定義域は `p` の境界 + 含意 `conflict_edges`、衝突グラフは行と `conflict_edges` から作り、サブ MIP は
/// [`super::solve_problem`] (500 ノード、1 本 `time_limit` 秒まで、それまでの最良の値を打ち切り値に) で解く。
/// 見つけた最良の実行可能解 (目的値 (最小化形、定数項込み), `p` の列の空間の値) を返す。
pub(crate) fn vbounds01_on_problem(p: &MipProblem, verbose: bool, time_limit: f64) -> Option<(f64, Vec<f64>)> {
    let t0 = std::time::Instant::now();
    let binary: Vec<bool> = (0..p.n).map(|j| p.is_int[j] && p.col_lo[j] >= 0.0 && p.col_up[j] <= 1.0).collect();
    let g = super::clique::CliqueGraph::build_with(p.n, &p.rows, &p.row_lo, &p.row_up, &binary, &p.col_lo, &p.col_up, &p.conflict_edges);
    let fb: Vec<bool> = (0..p.n).map(|j| p.is_int[j] && p.col_lo[j] == 0.0 && p.col_up[j] == 1.0).collect();
    let ints: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j] && p.col_lo[j] < p.col_up[j]).collect();
    let cliques = if vb01_clq() { vb01_cliques(p, &p.col_lo, &p.col_up, &fb) } else { Default::default() };
    let order = vb01_order(&g, &fb, &cliques);
    if ints.is_empty() || (order.len() as f64) < 0.65 * ints.len() as f64 {
        if verbose {
            eprintln!("MIP:   VB01: {} bound nodes in the implication graph < 0.65 x {} free integer columns: not used", order.len(), ints.len());
        }
        return None;
    }
    let mut dom = Domain::new(p);
    dom.add_implications(p.n, &p.conflict_edges);
    if !dom.propagate(p) {
        return None;
    }
    let mut best: Option<(f64, Vec<f64>)> = None;
    let (mut skip1, mut skip2) = (false, false);
    let stop = || false;
    for (tight, obj) in VB01_VARIANTS {
        if (obj == 1 && skip1) || (obj == 2 && skip2) {
            continue;
        }
        let (bounds, all1, all2, log) = vb01_fix_dom(p, &mut dom, &order, tight, obj, &ints, &stop);
        if verbose {
            eprintln!("MIP:   {log}");
        }
        if obj == 0 {
            skip1 = all1;
            skip2 = all2;
        }
        let Some((lo, up)) = bounds else { continue };
        let mut sub = p.clone();
        sub.col_lo = lo;
        sub.col_up = up;
        let params = super::solver::MipParams {
            time_limit,
            node_limit: 500,
            rel_gap: 0.0,
            abs_gap: 1e-9,
            verbose: false,
            submip: true,
            cutoff: best.as_ref().map_or(f64::INFINITY, |(z, _)| *z),
            restarts: 0,
            skip_heurs: 0,
            threads: 1,
            distinguish: false,
            fc_submip: false,
            tl_base: f64::NAN,
        };
        let r = super::solve_problem(&sub, params, true);
        if let Some(x) = r.x {
            let z = p.objective(&x);
            if p.is_feasible(&x, 1e-6) && best.as_ref().is_none_or(|(b, _)| z < *b) {
                if verbose {
                    eprintln!("MIP:   {}: sub-MIP solution {z:.10e}", vb01_label(tight, obj));
                }
                best = Some((z, x));
            }
        }
    }
    if verbose {
        eprintln!("MIP: VB01 on the presolved problem: {} ({:.2}s)", best.as_ref().map_or("none".to_string(), |(z, _)| format!("{z:.10e}")), t0.elapsed().as_secs_f64());
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [`vbounds01_on_problem`]: 0-1 列 4 本、含意 x0 <= x1・クリーク x2 + x3 <= 1・被覆 x1 + x2 >= 1 の問題で、実行可能な解を返す
    #[test]
    fn vb01_on_problem_finds_feasible() {
        let inf = f64::INFINITY;
        let rows = vec![vec![(0, 1.0), (1, -1.0)], vec![(2, 1.0), (3, 1.0)], vec![(1, 1.0), (2, 1.0)]];
        let p = MipProblem::from_rows(vec![0.0; 4], vec![1.0; 4], vec![-1.0, 0.0, 0.0, -1.0], 0.0, 1.0, vec![true; 4], rows, vec![-inf, -inf, 1.0], vec![0.0, 1.0, inf]);
        let (z, x) = vbounds01_on_problem(&p, false, 10.0).expect("VB01 should find a solution");
        assert!(p.is_feasible(&x, 1e-9));
        assert!((p.objective(&x) - z).abs() <= 1e-9);
    }
}
