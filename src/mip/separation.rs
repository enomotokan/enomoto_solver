//! 根での切除平面のループ (HiGHS の `HighsSeparation` / `HighsTableauSeparator` を簡略化したもの)。
//!
//! 各ラウンドで、元の行 1 本ずつと、基底にある小数の整数変数の tableau 行 (基底逆行列の行で LP 行を
//! 集約したもの、Gomory 相当) を CMIR に通して切除平面を作り、効き目の大きい順に、ほぼ平行なものを
//! 除いて LP に加えて解き直す。目的値がほとんど動かなくなったら止める。最後に効いていない (論理変数が
//! 基底にある) カット行を LP から外す。カットは大域的な境界から作るので、木全体で有効。

use super::cuts::{cmir, extended_cover, lifted_flow_cover, CutVars, RawCut, VarBounds};
use super::problem::MipProblem;
use super::domain::FEASTOL;
use super::lp::{LpStatus, SolveLimits, VarStatus};
use super::solver::Solver;
use super::lp_api::MipLp;

/// LP に加える前のカット (構造変数の係数、右辺、効き目)。
/// lifted flow cover を使うか (`ENOMOTO_MIP_NO_FLOWCOVER` で無効)。
fn use_flow_cover() -> bool {
    env_str!("ENOMOTO_MIP_NO_FLOWCOVER").is_none()
}

struct Candidate {
    coefs: Vec<(usize, f64)>,
    rhs: f64,
    efficacy: f64,
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 根の切除平面ループ。LP は根の最適解の状態で呼ぶこと。終わったときも LP は最適 (でなければ偽)。
    pub(super) fn root_cut_loop(&mut self, root_iters: u64) -> bool {
        let p = self.p;
        let n = p.n;
        let mut stall = 0;
        let mut prev_obj = self.lp.objective();
        let mut prev_nfrac = self.fractional(&self.lp.col_values()).len();
        let first_obj = prev_obj;
        // サブ MIP (RENS/RINS) では分離に時間をかけない
        let max_rounds = if self.params.submip { 5 } else { tunable!("ENOMOTO_T_MIP_CUT_ROUNDS", 25usize, usize) };
        let time_cap = if self.params.time_limit.is_finite() { tunable!("ENOMOTO_T_MIP_CUT_TIME_FRAC", 0.1, f64) * self.params.time_limit } else { f64::INFINITY };
        let mut total_added = 0usize;
        for round in 0..max_rounds {
            if self.time_up() || self.start.elapsed().as_secs_f64() > time_cap {
                break;
            }
            let x = self.lp.col_values();
            if self.fractional(&x).is_empty() {
                break;
            }
            let t_sep = std::time::Instant::now();
            let cands = self.separate(&x, false);
            let ncands = cands.len();
            if cands.is_empty() {
                break;
            }
            // 候補はすべてカットプールに入れる (選択で落ちたものも、ノードで違反すれば使う)
            if !self.params.submip && env_str!("ENOMOTO_MIP_POOL_SELECTED_ONLY").is_none() {
                for c in &cands {
                    self.add_to_pool(&c.coefs, c.rhs);
                }
            }
            // 効き目の大きい順に、平行なものを除いて選ぶ
            let max_cuts = (p.m.max(50)).min(500);
            let chosen = select_cuts(cands, max_cuts, p);
            let sep_secs = t_sep.elapsed().as_secs_f64();
            if chosen.is_empty() {
                break;
            }
            let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = chosen.into_iter().map(|c| (c.coefs, f64::NEG_INFINITY, c.rhs)).collect();
            total_added += rows.len();
            // カットプールに残す (ノードで違反していれば LP に戻す。候補として入れ済みなら重複は除かれる)
            if !self.params.submip {
                for (c, _, r) in &rows {
                    self.add_to_pool(c, *r);
                }
            }
            let saved_rows = self.lp.num_rows();
            self.add_cut_rows(&rows);
            let it0 = self.lp.total_iterations();
            let lim = 10 * root_iters.max(100) + 10_000;
            // 1 回の LP の時間の上限 (全体の 2%、最低 1 秒): 数値的に悪条件の LP が残り時間を使い切るのを防ぐ
            let cap = if self.params.time_limit.is_finite() { (tunable!("ENOMOTO_T_MIP_CUT_LP_TIME_FRAC", 0.02, f64) * self.params.time_limit).max(1.0) } else { f64::INFINITY };
            let lp_deadline = if cap.is_finite() {
                let d = std::time::Instant::now() + std::time::Duration::from_secs_f64(cap);
                Some(self.deadline.map_or(d, |g| g.min(d)))
            } else {
                self.deadline
            };
            let st = self.lp.solve(&SolveLimits { iteration_limit: lim, cutoff: f64::INFINITY, deadline: lp_deadline });
            self.sb_iters += 0;
            let _ = it0;
            if st != LpStatus::Optimal {
                // 数値的に困ったら今回のカットを外して戻す
                let mut remove = vec![false; self.lp.num_rows()];
                for r in remove.iter_mut().skip(saved_rows) {
                    *r = true;
                }
                self.delete_lp_rows(&remove);
                let st2 = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
                if self.params.verbose {
                    eprintln!("MIP: cut round {round}: LP status {st:?}, removed the cuts ({st2:?})");
                }
                return st2 == LpStatus::Optimal;
            }
            let obj = self.lp.objective();
            if env_str!("ENOMOTO_MIP_DEBUG_FC").is_some() && !self.params.submip {
                super::cuts::FC_STATS.with(|s| eprintln!("FC stats (calls, cuts, no SNF): {:?}", s.borrow()));
            }
            if self.params.verbose {
                eprintln!(
                    "MIP: cut round {round}: {} cuts (of {ncands}, sep {sep_secs:.2}s, LP {} iters), LP rows {}, obj {:.10e} ({:.2}s)",
                    rows.len(),
                    self.lp.total_iterations() - it0,
                    self.lp.num_rows(),
                    obj + p.offset,
                    self.start.elapsed().as_secs_f64()
                );
            }
            if env_str!("ENOMOTO_MIP_STALL_OLD").is_some() {
                // 停滞判定: 改善が初回からの改善量のわずかな割合なら停滞
                let gain = obj - prev_obj;
                let scale = (obj - first_obj).abs().max(1e-6 * obj.abs().max(1.0));
                if gain <= 1e-3 * scale || gain <= 1e-9 * obj.abs().max(1.0) {
                    stall += 1;
                    if stall >= 3 {
                        break;
                    }
                } else {
                    stall = 0;
                }
            } else {
                // SCIP の停滞判定 (solve.c): 目的値の相対変化が 1e-4 以下で、分数の列の数も十分に減っていなければ
                // 停滞。根では 10 回分で止める (サブ MIP では 3 回)。目的値も分数の列の数もまったく減らない
                // ラウンドは 2 回分と数える (何も進まない問題で長く回さない。分数の列が減っている間は続ける)。
                let nfrac = self.fractional(&self.lp.col_values()).len();
                let reldiff = (obj - prev_obj) / obj.abs().max(prev_obj.abs()).max(1.0);
                if reldiff <= 1e-4 && nfrac as f64 >= (0.9 - 0.1 * stall as f64) * prev_nfrac as f64 {
                    stall += if nfrac >= prev_nfrac { 2 } else { 1 };
                    if stall >= if self.params.submip { 3 } else { tunable!("ENOMOTO_T_MIP_CUT_STALL", 10usize, usize) } {
                        break;
                    }
                } else {
                    stall = 0;
                }
                prev_nfrac = nfrac;
            }
            prev_obj = obj;
            // 効いていないカットを外す (論理変数が基底にあり、行が緩んでいるもの)
            self.remove_inactive_cuts();
        }
        self.remove_inactive_cuts();
        let st = self.lp.solve(&SolveLimits { deadline: self.deadline, ..Default::default() });
        if self.params.verbose {
            eprintln!(
                "MIP: cut loop done: {} cuts added, {} in LP, obj {:.10e} -> {:.10e}",
                total_added,
                self.lp.num_rows() - p.m,
                first_obj + p.offset,
                self.lp.objective() + p.offset
            );
        }
        st == LpStatus::Optimal
    }

    /// カットをプールに加える。係数と右辺が (丸めて) 同じものは加えない。プールの非零数は問題の非零数の
    /// 20 倍 (最低 20 万) までにする。
    pub(super) fn add_to_pool(&mut self, coefs: &[(usize, f64)], rhs: f64) {
        let nnz_cap = (20 * self.p.rows.iter().map(|r| r.len()).sum::<usize>()).max(200_000);
        if self.cut_pool_nnz + coefs.len() > nnz_cap {
            return;
        }
        // 最大係数 1 に揃えて 1e-9 の格子で丸めたもののハッシュ
        use std::hash::{Hash, Hasher};
        let cmax = coefs.iter().fold(0.0f64, |m, &(_, v)| m.max(v.abs())).max(1e-300);
        let mut hs = std::collections::hash_map::DefaultHasher::new();
        for &(j, v) in coefs {
            j.hash(&mut hs);
            ((v / cmax) * 1e9).round().to_bits().hash(&mut hs);
        }
        ((rhs / cmax) * 1e9).round().to_bits().hash(&mut hs);
        if !self.cut_pool_keys.insert(hs.finish()) {
            return;
        }
        let norm = coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
        self.cut_pool_nnz += coefs.len();
        self.cut_pool.push((coefs.to_vec(), rhs, norm));
    }

    /// カットプールから、LP 解 `x` が違反するカットを効き目の大きい順に最大 `max_cuts` 本 LP に加える
    /// (違反しているので今の LP にはないカット)。LP の行が増えすぎていれば先に効いていないカットを外す。加えたら真。
    pub(super) fn pool_cut_round(&mut self, x: &[f64], max_cuts: usize) -> bool {
        if self.cut_pool.is_empty() {
            return false;
        }
        let mut viol: Vec<(f64, usize)> = Vec::new();
        for (k, (c, r, norm)) in self.cut_pool.iter().enumerate() {
            let act: f64 = c.iter().map(|&(j, v)| v * x[j]).sum();
            let eff = (act - r) / norm.max(1e-12);
            if eff > 1e-4 && act - r > 1e-6 * (1.0 + r.abs()) {
                viol.push((eff, k));
            }
        }
        if viol.is_empty() {
            return false;
        }
        viol.sort_by(|a, b| b.0.total_cmp(&a.0));
        viol.truncate(max_cuts);
        let cap = self.p.m + (2 * self.cut_pool.len()).clamp(100, 2000);
        if self.lp.num_rows() + viol.len() > cap {
            self.remove_inactive_cuts();
        }
        let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = viol.iter().map(|&(_, k)| (self.cut_pool[k].0.clone(), f64::NEG_INFINITY, self.cut_pool[k].1)).collect();
        self.add_cut_rows(&rows);
        true
    }

    /// ノードでの分離 (1 ラウンド)。カットを加えたら LP を解き直し、真を返す。LP の行数の上限を超えたら何もしない。
    pub(super) fn node_cut_round(&mut self, x: &[f64]) -> bool {
        let p = self.p;
        let max_rows = p.m + (2 * p.m).max(500);
        if self.lp.num_rows() >= max_rows {
            return false;
        }
        // ノードでは軽い分離 (経路集約の始点・tableau 行を減らす) で、追加するカットも少なくする
        let cands = self.separate(x, true);
        if cands.is_empty() {
            return false;
        }
        let room = max_rows - self.lp.num_rows();
        let chosen = select_cuts(cands, room.min(tunable!("ENOMOTO_T_MIP_NODE_MAX_CUTS", 20usize, usize)), p);
        if chosen.is_empty() {
            return false;
        }
        let rows: Vec<(Vec<(usize, f64)>, f64, f64)> = chosen.into_iter().map(|c| (c.coefs, f64::NEG_INFINITY, c.rhs)).collect();
        // カットはプールにも入れ (他のノードで違反すれば戻る)、このノードで効いていないカットは外してから加える
        // (HiGHS の aging と同じく LP を小さく保つ。外したカットもプールにあれば戻せる)
        if env_str!("ENOMOTO_MIP_NODE_CUTS_KEEP").is_none() {
            for (c, _, r) in &rows {
                self.add_to_pool(c, *r);
            }
            self.remove_aged_cuts();
        }
        self.add_cut_rows(&rows);
        true
    }

    /// LP にカットの行を加える (年齢 0)。
    pub(super) fn add_cut_rows(&mut self, rows: &[(Vec<(usize, f64)>, f64, f64)]) {
        self.sync_cut_age();
        self.lp.add_rows(rows);
        self.cut_age.extend(std::iter::repeat_n(0, rows.len()));
        self.row_log.push(super::solver::RowEdit::Add(rows.len()));
    }

    /// LP の行を消す (カットの年齢も合わせて消す)。
    pub(super) fn delete_lp_rows(&mut self, remove: &[bool]) {
        self.sync_cut_age();
        let m0 = self.p.m;
        let mut k = m0;
        self.cut_age.retain(|_| {
            let keep = !remove[k];
            k += 1;
            keep
        });
        self.lp.delete_rows(remove);
        self.row_log.push(super::solver::RowEdit::Delete(remove.to_vec()));
    }

    /// カットの年齢の長さを LP のカットの行の数に合わせる (足りなければ 0 で埋める)。
    fn sync_cut_age(&mut self) {
        let want = self.lp.num_rows().saturating_sub(self.p.m);
        self.cut_age.resize(want, 0);
    }

    /// ノードの LP の後に呼ぶ (HiGHS の LP の aging): 効いていない (論理変数が基底で行に余裕がある) カットの年齢を
    /// 1 増やし、効いているカットは 0 に戻す。
    pub(super) fn age_cuts(&mut self) {
        self.sync_cut_age();
        if self.cut_age.is_empty() {
            return;
        }
        let m0 = self.p.m;
        let b = self.lp.basis();
        let act = self.lp.row_activities();
        for (k, age) in self.cut_age.iter_mut().enumerate() {
            let i = m0 + k;
            let (_, up) = self.lp.row_bounds(i);
            if b.row[i] == VarStatus::Basic && act[i] < up - 1e-6 * (1.0 + up.abs()) {
                *age = age.saturating_add(1);
            } else {
                *age = 0;
            }
        }
    }

    /// 年齢が上限 (`ENOMOTO_T_MIP_CUT_AGE_LIMIT`、既定 30。HiGHS の `mip_lp_age_limit` は 10 だが、10 では binkar10_1 で効くカットまで外れた) を超えたカットを LP から外す
    /// (プールにあるカットは、違反すればまた戻る)。外した数を返す。
    pub(super) fn remove_aged_cuts(&mut self) -> usize {
        self.sync_cut_age();
        let limit = tunable!("ENOMOTO_T_MIP_CUT_AGE_LIMIT", 30u32, u32);
        let m0 = self.p.m;
        let mut remove = vec![false; self.lp.num_rows()];
        let mut cnt = 0;
        for (k, &age) in self.cut_age.iter().enumerate() {
            if age > limit {
                remove[m0 + k] = true;
                cnt += 1;
            }
        }
        if cnt > 0 {
            self.delete_lp_rows(&remove);
        }
        cnt
    }

    /// LP の行のうち、元の行より後ろ (カット) で論理変数が基底にあるものを外す。
    pub(super) fn remove_inactive_cuts(&mut self) {
        let m0 = self.p.m;
        let mr = self.lp.num_rows();
        if mr <= m0 {
            return;
        }
        let b = self.lp.basis();
        let act = self.lp.row_activities();
        let mut remove = vec![false; mr];
        let mut any = false;
        for i in m0..mr {
            if b.row[i] == VarStatus::Basic {
                let (_, up) = self.lp.row_bounds(i);
                if act[i] < up - 1e-6 * (1.0 + up.abs()) {
                    remove[i] = true;
                    any = true;
                }
            }
        }
        if any {
            self.delete_lp_rows(&remove);
        }
    }

    /// 経路集約: 元の行を 1 本選び、集約行に残った連続変数のうち LP 値が境界 (変数上下限を含む) から
    /// 最も離れたものを、それを含む別の元の行で打ち消す、を最大 `PATH_MAX_LEN` 段繰り返し、各段で CMIR を試す
    /// (Marchand & Wolsey の集約ヒューリスティクス)。行 i の活動量を変数 `n + i` として
    /// `sum_i w_i (a_i x - r_i) = 0` の形で集約するので、等式・不等式・範囲行を区別せずに扱える
    /// (`r_i` の置き換えは CMIR が行の上下限で行う)。
    fn path_aggregation<F>(&mut self, vars: &CutVars, lp_rows: &[Vec<(usize, f64)>], cands: &mut Vec<Candidate>, push: &mut F, max_starts: usize)
    where
        F: FnMut(Option<RawCut>, &mut Vec<Candidate>, &Solver<L>),
    {
        const PATH_MAX_LEN: usize = 6;
        const MAX_ROW_LEN: usize = 500;
        let p = self.p;
        let n = p.n;
        // 元の行だけでなく LP に入っているカットの行も集約に使う (HiGHS と同じ)。再スタート後はカットが元の行になり
        // 経路の候補が大きく増えて根の下界が伸びたので、再スタートを待たずに使う
        let m = if env_str!("ENOMOTO_MIP_PATH_ORIG_ONLY").is_some() { p.m } else { lp_rows.len() };
        // 連続変数の LP 値の、最も近い境界 (単純な上下限・変数上下限) までの距離
        let bound_dist = |j: usize| -> f64 {
            let xj = vars.x[j];
            let mut d = (xj - vars.lo[j]).min(vars.up[j] - xj);
            if let Some(vb) = vars.vb {
                for &(y, a, e) in &vb.vub[j] {
                    d = d.min(a * vars.x[y] + e - xj);
                }
                for &(y, a, e) in &vb.vlb[j] {
                    d = d.min(xj - a * vars.x[y] - e);
                }
            }
            d.max(0.0)
        };
        // 列 → 元の行 (短い行だけ)
        let mut col_rows: Vec<Vec<usize>> = vec![Vec::new(); n];
        for i in 0..m {
            if lp_rows[i].len() <= MAX_ROW_LEN {
                for &(j, _) in &lp_rows[i] {
                    if !vars.is_int[j] {
                        col_rows[j].push(i);
                    }
                }
            }
        }
        // 行が LP で効いているか (活動量が上下限に近い)
        let tight = |i: usize| -> bool {
            let r = vars.x[n + i];
            let (l, u) = (vars.lo[n + i], vars.up[n + i]);
            (r - l).abs() <= 1e-6 * (1.0 + l.abs()) || (u - r).abs() <= 1e-6 * (1.0 + u.abs())
        };
        let mut agg = vec![0.0f64; n];
        let mut in_agg = vec![false; n];
        let mut touched: Vec<usize> = Vec::new();
        let mut used_row = vec![false; m];
        let mut starts = 0usize;
        // 始点の行 (既定は元の行だけ。カットの行は相手としてだけ使う)
        let m_start = if env_str!("ENOMOTO_MIP_PATH_START_CUTS").is_some() { m } else { p.m.min(m) };
        for start in 0..m_start {
            if starts >= max_starts || (start % 64 == 0 && self.time_up()) {
                break;
            }
            let row = &lp_rows[start];
            if row.len() < 2 || row.len() > MAX_ROW_LEN {
                continue;
            }
            // 境界から離れた連続変数がなければ 1 行の CMIR と同じなので飛ばす
            if !row.iter().any(|&(j, _)| !vars.is_int[j] && bound_dist(j) > 1e-6) {
                continue;
            }
            starts += 1;
            for &j in &touched {
                agg[j] = 0.0;
                in_agg[j] = false;
            }
            touched.clear();
            let mut weights: Vec<(usize, f64)> = vec![(start, 1.0)];
            used_row[start] = true;
            for &(j, a) in row {
                if !in_agg[j] {
                    in_agg[j] = true;
                    touched.push(j);
                }
                agg[j] += a;
            }
            for step in 0..PATH_MAX_LEN {
                if step > 0 {
                    let mut base: Vec<(usize, f64)> = Vec::with_capacity(touched.len() + weights.len());
                    let amax = touched.iter().fold(0.0f64, |mx, &j| mx.max(agg[j].abs()));
                    for &j in &touched {
                        if agg[j].abs() > 1e-9 * amax.max(1.0) {
                            base.push((j, agg[j]));
                        }
                    }
                    for &(i, w) in &weights {
                        base.push((n + i, -w));
                    }
                    push(cmir(vars, &base, 0.0), cands, self);
                    let neg: Vec<(usize, f64)> = base.iter().map(|&(k, a)| (k, -a)).collect();
                    push(cmir(vars, &neg, 0.0), cands, self);
                    if use_flow_cover() {
                        push(lifted_flow_cover(vars, &base, 0.0), cands, self);
                        push(lifted_flow_cover(vars, &neg, 0.0), cands, self);
                    }
                }
                // 打ち消す連続変数: 境界から最も離れたもの
                let amax = touched.iter().fold(0.0f64, |mx, &j| mx.max(agg[j].abs()));
                let mut best: Option<(usize, f64)> = None;
                for &j in &touched {
                    if vars.is_int[j] || agg[j].abs() <= 1e-9 * amax.max(1.0) {
                        continue;
                    }
                    let d = bound_dist(j);
                    if d > 1e-6 && best.is_none_or(|(_, bd)| d > bd) {
                        best = Some((j, d));
                    }
                }
                let Some((j, _)) = best else { break };
                // 相手の行: 未使用で j を含むもの。効いている行を優先し、同じ組では乱数で選ぶ
                let mut pick: Option<(usize, f64)> = None;
                for &i in &col_rows[j] {
                    if used_row[i] {
                        continue;
                    }
                    let score = if tight(i) { 2.0 } else { 1.0 } + 0.5 * self.rand();
                    if pick.is_none_or(|(_, sc)| score > sc) {
                        pick = Some((i, score));
                    }
                }
                let Some((r, _)) = pick else { break };
                let arj = lp_rows[r].iter().find(|&&(k, _)| k == j).map_or(0.0, |&(_, a)| a);
                if arj.abs() < 1e-9 {
                    break;
                }
                let w = -agg[j] / arj;
                // 重みの比が大きすぎる集約は数値的に危ないので止める
                let wmax = weights.iter().fold(w.abs(), |mx, &(_, v)| mx.max(v.abs()));
                let wmin = weights.iter().fold(w.abs(), |mn, &(_, v)| mn.min(v.abs()));
                if wmax / wmin > 1e4 {
                    break;
                }
                used_row[r] = true;
                weights.push((r, w));
                for &(k, a) in &lp_rows[r] {
                    if !in_agg[k] {
                        in_agg[k] = true;
                        touched.push(k);
                    }
                    agg[k] += w * a;
                }
                agg[j] = 0.0;
            }
            for &(i, _) in &weights {
                used_row[i] = false;
            }
        }
    }

    /// 現在の LP 解 `x` を切る候補を作る。
    /// `light` (ノード用) なら経路集約の始点と tableau 行の数を絞る。
    fn separate(&mut self, x: &[f64], light: bool) -> Vec<Candidate> {
        let p = self.p;
        let n = p.n;
        let mr = self.lp.num_rows();
        // 変数: 構造変数 0..n と、LP 行の活動量 n..n+mr
        let lp_rows: Vec<Vec<(usize, f64)>> = (0..mr).map(|i| self.lp.row(i)).collect();
        let act = self.lp.row_activities();
        // カットは木全体で有効にするため、大域的な境界から作る
        let mut lo = self.dom.global_lo.clone();
        let mut up = self.dom.global_up.clone();
        let mut is_int = self.cut_int.clone();
        // 暗黙の整数列の境界も整数に丸める (整数列は丸め済み)
        for j in 0..n {
            if is_int[j] && !p.is_int[j] {
                lo[j] = (lo[j] - FEASTOL).ceil();
                up[j] = (up[j] + FEASTOL).floor();
            }
        }
        let mut xv = x.to_vec();
        for i in 0..mr {
            let (l, u) = self.lp.row_bounds(i);
            let integral = lp_rows[i].iter().all(|&(j, a)| self.cut_int[j] && (a - a.round()).abs() <= 1e-9);
            let (l, u) = if integral { ((l - FEASTOL).ceil(), (u + FEASTOL).floor()) } else { (l, u) };
            lo.push(l);
            up.push(u);
            is_int.push(integral);
            xv.push(act[i]);
        }
        if self.vbounds.is_none() {
            self.vbounds = Some(std::rc::Rc::new(VarBounds::from_rows(n, &p.is_int, &p.rows, &p.row_lo, &p.row_up)));
        }
        let vb = self.vbounds.clone();
        let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &xv, vb: if env_str!("ENOMOTO_MIP_NO_VB").is_some() { None } else { vb.as_deref() } };
        let mut cands: Vec<Candidate> = Vec::new();
        let mut push = |raw: Option<RawCut>, cands: &mut Vec<Candidate>, s: &Solver<L>| {
            if let Some(raw) = raw {
                if let Some(c) = finish_cut(raw, n, &lp_rows, &s.dom.global_lo, &s.dom.global_up, x, s.incumbent.as_ref().map(|(_, v)| v.as_slice())) {
                    cands.push(c);
                }
            }
        };
        let sep_t0 = std::time::Instant::now();
        let dbg_sep = env_str!("ENOMOTO_MIP_DEBUG_SEP").is_some();
        // 元の行 1 本ずつ
        for i in 0..p.m {
            if i % 64 == 0 && self.time_up() {
                return cands;
            }
            let row = &p.rows[i];
            if row.len() < 2 {
                continue;
            }
            if p.row_up[i].is_finite() {
                if use_flow_cover() {
                    push(lifted_flow_cover(&vars, row, p.row_up[i]), &mut cands, self);
                }
                let r = cmir(&vars, row, p.row_up[i]);
                push(r, &mut cands, self);
                let r = extended_cover(&vars, row, p.row_up[i]);
                push(r, &mut cands, self);
            }
            if p.row_lo[i].is_finite() {
                let neg: Vec<(usize, f64)> = row.iter().map(|&(j, a)| (j, -a)).collect();
                if use_flow_cover() {
                    push(lifted_flow_cover(&vars, &neg, -p.row_lo[i]), &mut cands, self);
                }
                let r = cmir(&vars, &neg, -p.row_lo[i]);
                push(r, &mut cands, self);
                let r = extended_cover(&vars, &neg, -p.row_lo[i]);
                push(r, &mut cands, self);
            }
        }
        if dbg_sep {
            eprintln!("SEP rows {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        // zerohalf ({0, 1/2}-CG) カット (元の行から)
        if env_str!("ENOMOTO_MIP_NO_ZEROHALF").is_none() {
            let zh = super::zerohalf::zerohalf_cuts(&p.rows, &p.row_lo, &p.row_up, &self.cut_int, &self.dom.global_lo, &self.dom.global_up, x, 100);
            for raw in zh {
                push(Some(raw), &mut cands, self);
            }
        }
        if dbg_sep {
            eprintln!("SEP zerohalf {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        // clique カット (2 値列の衝突グラフ。根で一度だけ作って使い回す)。既定では使わない: 40 問で根の下界は
        // ほとんど変わらず、10teams では根の LP 解が変わって根の被約費用ヒューリスティクスが解を見つけられなくなった
        // (13 問 / 幾何平均 30.57 に悪化)。ENOMOTO_MIP_CLIQUE_CUTS=1 で使う
        if env_str!("ENOMOTO_MIP_CLIQUE_CUTS").is_some() {
            if self.clique_graph.is_none() {
                let binary: Vec<bool> = (0..p.n).map(|j| p.is_int[j] && self.dom.global_lo[j] >= 0.0 && self.dom.global_up[j] <= 1.0).collect();
                let g = super::clique::CliqueGraph::build(p.n, &p.rows, &p.row_lo, &p.row_up, &binary, &self.dom.global_lo, &self.dom.global_up);
                if self.params.verbose && !self.params.submip {
                    eprintln!("MIP: clique graph: {} edges", g.num_edges());
                }
                self.clique_graph = Some(std::rc::Rc::new(g));
            }
            let g = self.clique_graph.clone().unwrap();
            for raw in g.separate(x, 100) {
                push(Some(raw), &mut cands, self);
            }
        }
        if dbg_sep {
            eprintln!("SEP clique {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        // 経路集約 (path aggregation、HiGHS の `HighsPathSeparator`)
        if env_str!("ENOMOTO_MIP_NO_PATH_AGG").is_none() {
            let max_starts = if light { tunable!("ENOMOTO_T_MIP_NODE_PATH_STARTS", 0usize, usize) } else { 1000 };
            self.path_aggregation(&vars, &lp_rows, &mut cands, &mut push, max_starts);
        }
        if dbg_sep {
            eprintln!("SEP path {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        // tableau 行
        let mut basics: Vec<(usize, f64)> = Vec::new();
        for s in 0..mr {
            let k = self.lp.basic_var(s);
            if k < n && self.cut_int[k] {
                let v = x[k];
                let f = v - v.floor();
                if f > 1e-3 && f < 1.0 - 1e-3 {
                    basics.push((s, f * (1.0 - f) / self.lp.dse_weight(s).max(1e-8)));
                }
            }
        }
        basics.sort_by(|a, b| b.1.total_cmp(&a.1));
        let nint = p.is_int.iter().filter(|&&b| b).count();
        let limit = if light { tunable!("ENOMOTO_T_MIP_NODE_TAB_ROWS", 50usize, usize) } else { 200 + (0.1 * (mr.min(nint)) as f64) as usize };
        basics.truncate(limit);
        let mut agg = vec![0.0; n];
        let tab_fc = use_flow_cover() && env_str!("ENOMOTO_MIP_TAB_FC").is_some();
        for &(s, _) in &basics {
            if self.time_up() {
                break;
            }
            let w = self.lp.basis_inverse_row(s);
            let wmax = w.iter().fold(0.0f64, |m, v| m.max(v.abs()));
            if wmax == 0.0 {
                continue;
            }
            let wmin_keep = 1e-9 * wmax;
            // 集約: sum_i w_i (a_i x - r_i) = 0
            let mut touched: Vec<usize> = Vec::new();
            let mut base: Vec<(usize, f64)> = Vec::new();
            let mut ok = true;
            let mut wnz_min = f64::INFINITY;
            for i in 0..mr {
                let wi = w[i];
                if wi.abs() <= wmin_keep {
                    continue;
                }
                wnz_min = wnz_min.min(wi.abs());
                for &(j, a) in &lp_rows[i] {
                    if agg[j] == 0.0 {
                        touched.push(j);
                    }
                    agg[j] += wi * a;
                    if agg[j] == 0.0 {
                        agg[j] = 1e-300;
                    }
                }
                base.push((n + i, -wi));
            }
            // 重みの比が大きすぎる集約は数値的に危ないので使わない
            if wmax / wnz_min > 1e6 {
                ok = false;
            }
            for &j in &touched {
                let v = agg[j];
                if v.abs() > 1e-11 && ok {
                    base.push((j, v));
                }
                agg[j] = 0.0;
            }
            if !ok {
                continue;
            }
            let r = cmir(&vars, &base, 0.0);
            push(r, &mut cands, self);
            let neg: Vec<(usize, f64)> = base.iter().map(|&(k, a)| (k, -a)).collect();
            let r = cmir(&vars, &neg, 0.0);
            push(r, &mut cands, self);
            if tab_fc {
                push(lifted_flow_cover(&vars, &base, 0.0), &mut cands, self);
                push(lifted_flow_cover(&vars, &neg, 0.0), &mut cands, self);
            }
        }
        if dbg_sep {
            eprintln!("SEP tableau {:.3}s cands {}", sep_t0.elapsed().as_secs_f64(), cands.len());
        }
        cands
    }
}

/// 生のカット (構造変数と行の活動量の変数) を構造変数だけの式にし、整え、検査する。
fn finish_cut(
    raw: RawCut,
    n: usize,
    lp_rows: &[Vec<(usize, f64)>],
    lo: &[f64],
    up: &[f64],
    x: &[f64],
    incumbent: Option<&[f64]>,
) -> Option<Candidate> {
    let mut dense: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
    for &(k, c) in &raw.coefs {
        if k < n {
            *dense.entry(k).or_insert(0.0) += c;
        } else {
            for &(j, a) in &lp_rows[k - n] {
                *dense.entry(j).or_insert(0.0) += c * a;
            }
        }
    }
    let mut rhs = raw.rhs;
    let cmax = dense.values().fold(0.0f64, |m, v| m.max(v.abs()));
    if cmax <= 0.0 || !rhs.is_finite() {
        return None;
    }
    // 小さい係数は境界で吸収して消す
    let mut coefs: Vec<(usize, f64)> = Vec::with_capacity(dense.len());
    for (j, c) in dense {
        if c.abs() <= 1e-9 * cmax {
            // c x_j >= min(c lo, c up) を使って右辺を緩める
            let m = if c > 0.0 { c * lo[j] } else { c * up[j] };
            if !m.is_finite() {
                return None;
            }
            rhs -= m;
        } else {
            coefs.push((j, c));
        }
    }
    if coefs.is_empty() {
        return None;
    }
    let cmin = coefs.iter().fold(f64::INFINITY, |m, &(_, c)| m.min(c.abs()));
    if cmax / cmin > 1e6 {
        return None;
    }
    // 最大係数 1 にスケール
    let s = 1.0 / cmax;
    for c in coefs.iter_mut() {
        c.1 *= s;
    }
    rhs *= s;
    let act: f64 = coefs.iter().map(|&(j, c)| c * x[j]).sum();
    let norm: f64 = coefs.iter().map(|&(_, c)| c * c).sum::<f64>().sqrt();
    let viol = act - rhs;
    if viol <= 1e-6 * (1.0 + rhs.abs()) {
        return None;
    }
    let efficacy = viol / norm;
    if efficacy < 1e-5 {
        return None;
    }
    // 安全確認: 暫定解を切るカットは (生成の誤りなので) 捨てる
    if let Some(inc) = incumbent {
        let a: f64 = coefs.iter().map(|&(j, c)| c * inc[j]).sum();
        if a > rhs + 1e-6 * (1.0 + rhs.abs()) {
            if env_str!("ENOMOTO_MIP_LOG").is_some() {
                eprintln!("MIP: warning: a generated cut cuts off the incumbent (rejected)");
            }
            return None;
        }
    }
    Some(Candidate { coefs, rhs, efficacy })
}

/// 効き目の大きい順に、既に選んだものとほぼ平行 (|cos| > 0.99) なものを除いて選ぶ。
fn select_cuts(cands: Vec<Candidate>, max_cuts: usize, p: &MipProblem) -> Vec<Candidate> {
    if env_str!("ENOMOTO_MIP_CUTSEL_OLD").is_some() {
        return select_cuts_old(cands, max_cuts);
    }
    select_cuts_hybrid(cands, max_cuts, p)
}

/// 疎なベクトル (列番号の昇順) の内積。
fn sparse_dot(a: &[(usize, f64)], b: &[(usize, f64)]) -> f64 {
    let (mut i, mut k, mut dot) = (0, 0, 0.0);
    while i < a.len() && k < b.len() {
        let (x, y) = (a[i].0, b[k].0);
        if x == y {
            dot += a[i].1 * b[k].1;
            i += 1;
            k += 1;
        } else if x < y {
            i += 1;
        } else {
            k += 1;
        }
    }
    dot
}

/// SCIP の `cutsel_hybrid` に倣ったカット選択: スコア = 効き目 + 0.1 × 目的関数との平行度 + 0.1 × 整数列の割合。
/// スコアの高い順に採り、採ったカットとの平行度 (|cos|) が 0.3 を超えるものは捨てる。ただしスコアが最良の
/// 0.9 倍以上の「良い」カットは平行度 0.7 まで許す (SCIP の既定は 0.1 / 0.5 だが、こちらはカットのラウンド数が
/// 少ないので緩めにした方が良かった)。
fn select_cuts_hybrid(cands: Vec<Candidate>, max_cuts: usize, p: &MipProblem) -> Vec<Candidate> {
    let cnorm = p.cost.iter().map(|c| c * c).sum::<f64>().sqrt();
    let mut scored: Vec<(f64, f64, Candidate)> = cands
        .into_iter()
        .map(|c| {
            let nc = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
            let objpar = if cnorm > 0.0 && nc > 0.0 { c.coefs.iter().map(|&(j, v)| v * p.cost[j]).sum::<f64>().abs() / (cnorm * nc) } else { 0.0 };
            let intsup = c.coefs.iter().filter(|&&(j, _)| p.is_int[j]).count() as f64 / c.coefs.len().max(1) as f64;
            (c.efficacy + 0.1 * objpar + 0.1 * intsup, nc, c)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let Some(best_score) = scored.first().map(|s| s.0) else { return Vec::new() };
    let (maxpar, goodmaxpar, good) = (tunable!("ENOMOTO_T_CUTSEL_MAXPAR", 0.3, f64), tunable!("ENOMOTO_T_CUTSEL_GOODMAXPAR", 0.7, f64), 0.9 * best_score);
    let mut chosen: Vec<(f64, Candidate)> = Vec::new();
    for (score, nc, c) in scored {
        if chosen.len() >= max_cuts {
            break;
        }
        let limit = if score >= good { goodmaxpar } else { maxpar };
        let parallel = chosen.iter().any(|(nd, d)| sparse_dot(&c.coefs, &d.coefs).abs() > limit * nc * nd);
        if !parallel {
            chosen.push((nc, c));
        }
    }
    chosen.into_iter().map(|(_, c)| c).collect()
}

fn select_cuts_old(mut cands: Vec<Candidate>, max_cuts: usize) -> Vec<Candidate> {
    cands.sort_by(|a, b| b.efficacy.total_cmp(&a.efficacy));
    let mut chosen: Vec<Candidate> = Vec::new();
    let mut norms: Vec<f64> = Vec::new();
    for c in cands {
        if chosen.len() >= max_cuts {
            break;
        }
        let nc: f64 = c.coefs.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt();
        let mut parallel = false;
        for (d, &nd) in chosen.iter().zip(&norms) {
            // 疎な内積 (両方とも列番号の昇順)
            let (mut i, mut k, mut dot) = (0, 0, 0.0);
            while i < c.coefs.len() && k < d.coefs.len() {
                let (a, b) = (c.coefs[i].0, d.coefs[k].0);
                if a == b {
                    dot += c.coefs[i].1 * d.coefs[k].1;
                    i += 1;
                    k += 1;
                } else if a < b {
                    i += 1;
                } else {
                    k += 1;
                }
            }
            if dot.abs() > 0.99 * nc * nd {
                parallel = true;
                break;
            }
        }
        if !parallel {
            norms.push(nc);
            chosen.push(c);
        }
    }
    chosen
}
