//! 変数の定義域 (境界) と、その変更の記録・巻き戻し・制約伝播 (HiGHS の `HighsDomain` に相当)。
//!
//! - 各行について、現在の境界のもとでの活動量の最小値・最大値 (有限部分と無限の項の数) を
//!   保持し、境界が変わるたびに差分で更新する。
//! - 境界の変更はスタックに積み、[`Domain::backtrack_to`] で任意の位置まで戻せる。
//!   分枝のたびに積み、別のノードへ移るときは根まで戻して積み直す。
//! - [`Domain::propagate`] は活動量から各変数の境界を締める (整数変数は丸める)。

use super::problem::MipProblem;

/// 実行可能性の許容誤差。
pub const FEASTOL: f64 = 1e-6;

#[derive(Debug, Clone, Copy)]
struct Change {
    col: usize,
    upper: bool,
    old: f64,
    /// 変更後の値。
    new: f64,
    /// 理由: 伝播した行 (`>= 0`)、または決定 (分枝・ヒューリスティクス・被約費用固定など、`-1`)。
    reason: i32,
}

/// 伝播で見つかった矛盾の出どころ (衝突解析用)。
#[derive(Clone, Copy, Debug)]
pub enum ConflictSrc {
    /// 行 `i` の最小活動量が上限を超えた (`true`) / 最大活動量が下限を下回った (`false`)。
    Row(usize, bool),
    /// 列 `j` の下限が上限を超えた。
    Col(usize),
}

/// [`Domain::analyze_multi`] の始点: 伝播の矛盾、または破れている証明 `sum coefs x <= rhs`。
pub enum ConflictStart<'a> {
    Src(ConflictSrc),
    Proof(&'a [(usize, f64)], f64),
}

/// 変数の定義域。
#[derive(Clone)]
pub struct Domain {
    /// 現在の下限・上限。
    pub lo: Vec<f64>,
    pub up: Vec<f64>,
    /// 行ごとの最小活動量の有限部分と、`-inf` の項の数。
    min_act: Vec<f64>,
    min_inf: Vec<u32>,
    /// 行ごとの最大活動量の有限部分と、`+inf` の項の数。
    max_act: Vec<f64>,
    max_inf: Vec<u32>,
    /// 変更の記録 (巻き戻し用)。
    stack: Vec<Change>,
    /// 前回 [`Self::take_changed_for_proofs`] から境界が変わった列 (双対証明の差分評価用、`changed` とは別に数える)。
    changed_p: Vec<usize>,
    changed_p_mark: Vec<bool>,
    /// 前回 [`Self::take_changed`] から境界が変わった列。
    changed: Vec<usize>,
    changed_mark: Vec<bool>,
    /// 伝播を待つ行。
    queue: Vec<usize>,
    in_queue: Vec<bool>,
    /// 矛盾が見つかったか。
    pub infeasible: bool,
    /// 矛盾の出どころ (衝突解析用、巻き戻すと消える)。
    pub conflict: Option<ConflictSrc>,
    /// 今伝播している行 (境界の変更の理由として記録する。伝播の外では `-1`)。
    cur_reason: i32,
    /// 根に戻ったときに適用する大域的な境界の締め付け (列, 上限か, 値)。
    pending_global: Vec<(usize, bool, f64)>,
    /// 根に戻った回数 (活動量の誤差の蓄積を防ぐための定期的な再計算に使う)。
    resets: u64,
    /// 大域的な境界 (根で確定したものと、その後の大域的な締め付けを反映したもの)。
    pub global_lo: Vec<f64>,
    pub global_up: Vec<f64>,
    /// 行ごとの `max_j |a_ij| (u_j - l_j)` (作ったときの境界で。境界は締まるだけなので以後も上界)。
    /// 行の余裕がこれ以上なら、その行からはどの境界も締まらないので走査を省く (HiGHS の capacity threshold)。
    row_cap: Vec<f64>,
}

thread_local! {
    static DEBUG_WORK: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

impl Domain {
    /// これまでに伝播で走査した行の長さの合計 (スレッドごと。ヒューリスティクスの手間の上限に使う)。
    pub fn debug_work(&self) -> u64 {
        DEBUG_WORK.with(|w| w.get())
    }

    /// 問題の境界から作る (伝播はしない)。
    pub fn new(p: &MipProblem) -> Self {
        let mut d = Domain {
            lo: p.col_lo.clone(),
            up: p.col_up.clone(),
            min_act: vec![0.0; p.m],
            min_inf: vec![0; p.m],
            max_act: vec![0.0; p.m],
            max_inf: vec![0; p.m],
            stack: Vec::new(),
            changed_p: Vec::new(),
            changed_p_mark: vec![false; p.n],
            changed: Vec::new(),
            changed_mark: vec![false; p.n],
            queue: Vec::new(),
            in_queue: vec![false; p.m],
            infeasible: false,
            conflict: None,
            cur_reason: -1,
            pending_global: Vec::new(),
            resets: 0,
            global_lo: p.col_lo.clone(),
            global_up: p.col_up.clone(),
            row_cap: if env_str!("ENOMOTO_MIP_NO_ROWCAP").is_some() { vec![f64::INFINITY; p.m] } else { p.rows.iter().map(|r| r.iter().map(|&(j, a)| a.abs() * (p.col_up[j] - p.col_lo[j])).fold(0.0f64, f64::max)).collect() },
        };
        d.recompute_activities(p);
        for i in 0..p.m {
            d.mark_row(i);
        }
        for j in 0..p.n {
            if d.lo[j] > d.up[j] + bound_tol(d.lo[j]) {
                d.infeasible = true;
            }
        }
        d
    }

    fn recompute_activities(&mut self, p: &MipProblem) {
        for i in 0..p.m {
            let (mut mn, mut mni, mut mx, mut mxi) = (0.0, 0u32, 0.0, 0u32);
            for &(j, a) in &p.rows[i] {
                let (cmin, cmax) = if a > 0.0 { (a * self.lo[j], a * self.up[j]) } else { (a * self.up[j], a * self.lo[j]) };
                if cmin.is_finite() {
                    mn += cmin;
                } else {
                    mni += 1;
                }
                if cmax.is_finite() {
                    mx += cmax;
                } else {
                    mxi += 1;
                }
            }
            self.min_act[i] = mn;
            self.min_inf[i] = mni;
            self.max_act[i] = mx;
            self.max_inf[i] = mxi;
        }
    }

    /// 変更の記録の長さ (巻き戻し位置として使う)。
    /// 記録の位置 `pos` より後の境界の変更 (列, 上限か, 変更後の値)。同じ列・側が何度も変わったら最後のものが後に来る。
    pub fn changes_since(&self, pos: usize) -> Vec<(usize, bool, f64)> {
        self.stack[pos.min(self.stack.len())..].iter().map(|c| (c.col, c.upper, c.new)).collect()
    }

    pub fn stack_len(&self) -> usize {
        self.stack.len()
    }

    /// 根の状態 (記録を全部巻き戻した境界) を返す。
    pub fn root_bounds(&self) -> (Vec<f64>, Vec<f64>) {
        let (mut lo, mut up) = (self.lo.clone(), self.up.clone());
        for c in self.stack.iter().rev() {
            if c.upper {
                up[c.col] = c.old;
            } else {
                lo[c.col] = c.old;
            }
        }
        (lo, up)
    }

    /// 前回呼んでから境界が変わった列を取り出す (LP への反映用)。
    pub fn take_changed(&mut self) -> Vec<usize> {
        for &j in &self.changed {
            self.changed_mark[j] = false;
        }
        std::mem::take(&mut self.changed)
    }

    fn mark_changed(&mut self, j: usize) {
        if !self.changed_mark[j] {
            self.changed_mark[j] = true;
            self.changed.push(j);
        }
        if !self.changed_p_mark[j] {
            self.changed_p_mark[j] = true;
            self.changed_p.push(j);
        }
    }

    /// 前回呼んでから境界が変わった列を取り出す (双対証明の差分評価用)。
    pub fn take_changed_for_proofs(&mut self) -> Vec<usize> {
        for &j in &self.changed_p {
            self.changed_p_mark[j] = false;
        }
        std::mem::take(&mut self.changed_p)
    }

    fn mark_row(&mut self, i: usize) {
        if !self.in_queue[i] {
            self.in_queue[i] = true;
            self.queue.push(i);
        }
    }

    /// 列 `j` の境界を `old` から `new` に変えたときの活動量の更新。
    fn update_activity(&mut self, p: &MipProblem, j: usize, upper: bool, old: f64, new: f64) {
        for &(i, a) in &p.cols[j] {
            // a > 0 なら下限は最小活動量、上限は最大活動量に効く (a < 0 なら逆)。
            let affects_min = (a > 0.0) != upper;
            let (oc, nc) = (a * old, a * new);
            if affects_min {
                if oc.is_finite() {
                    self.min_act[i] -= oc;
                } else {
                    self.min_inf[i] -= 1;
                }
                if nc.is_finite() {
                    self.min_act[i] += nc;
                } else {
                    self.min_inf[i] += 1;
                }
            } else {
                if oc.is_finite() {
                    self.max_act[i] -= oc;
                } else {
                    self.max_inf[i] -= 1;
                }
                if nc.is_finite() {
                    self.max_act[i] += nc;
                } else {
                    self.max_inf[i] += 1;
                }
            }
            self.mark_row(i);
        }
    }

    fn set_bound(&mut self, p: &MipProblem, j: usize, upper: bool, v: f64, record: bool) {
        let old = if upper { self.up[j] } else { self.lo[j] };
        if old == v {
            return;
        }
        if upper {
            self.up[j] = v;
        } else {
            self.lo[j] = v;
        }
        if record {
            self.stack.push(Change { col: j, upper, old, new: v, reason: self.cur_reason });
        }
        self.update_activity(p, j, upper, old, v);
        self.mark_changed(j);
        if self.lo[j] > self.up[j] + bound_tol(self.lo[j]) {
            if !self.infeasible {
                self.conflict = Some(ConflictSrc::Col(j));
            }
            self.infeasible = true;
        }
    }

    /// 下限を `v` 以上に締める (整数列は丸める)。締まったら真。
    pub fn tighten_lower(&mut self, p: &MipProblem, j: usize, v: f64) -> bool {
        let v = if p.is_int[j] { (v - FEASTOL).ceil() } else { v };
        if v > self.lo[j] {
            self.set_bound(p, j, false, v, true);
            true
        } else {
            false
        }
    }

    /// 上限を `v` 以下に締める (整数列は丸める)。締まったら真。
    pub fn tighten_upper(&mut self, p: &MipProblem, j: usize, v: f64) -> bool {
        let v = if p.is_int[j] { (v + FEASTOL).floor() } else { v };
        if v < self.up[j] {
            self.set_bound(p, j, true, v, true);
            true
        } else {
            false
        }
    }

    /// 記録を `pos` まで巻き戻す (境界と活動量を戻し、矛盾の印を消す)。
    pub fn backtrack_to(&mut self, p: &MipProblem, pos: usize) {
        while self.stack.len() > pos {
            let c = self.stack.pop().unwrap();
            let cur = if c.upper { self.up[c.col] } else { self.lo[c.col] };
            if c.upper {
                self.up[c.col] = c.old;
            } else {
                self.lo[c.col] = c.old;
            }
            self.update_activity(p, c.col, c.upper, cur, c.old);
            self.mark_changed(c.col);
        }
        self.infeasible = false;
        self.conflict = None;
        for &i in &self.queue {
            self.in_queue[i] = false;
        }
        self.queue.clear();
    }

    /// 根まで戻し、保留中の大域的な締め付けを適用する。
    pub fn reset_to_root(&mut self, p: &MipProblem) {
        self.backtrack_to(p, 0);
        let pend = std::mem::take(&mut self.pending_global);
        for (j, upper, v) in pend {
            let tighter = if upper { v < self.up[j] } else { v > self.lo[j] };
            if tighter {
                self.set_bound(p, j, upper, v, false);
            }
        }
        self.resets += 1;
        if self.resets % 64 == 0 {
            self.recompute_activities(p);
        }
        self.queue.iter().for_each(|&i| self.in_queue[i] = false);
        self.queue.clear();
    }

    /// 大域的に (すべてのノードで) 有効な締め付け。根にいればすぐ、そうでなければ次に根へ戻ったときに適用する。
    pub fn tighten_global(&mut self, p: &MipProblem, j: usize, upper: bool, v: f64) {
        let v = if p.is_int[j] {
            if upper {
                (v + FEASTOL).floor()
            } else {
                (v - FEASTOL).ceil()
            }
        } else {
            v
        };
        if upper {
            self.global_up[j] = self.global_up[j].min(v);
        } else {
            self.global_lo[j] = self.global_lo[j].max(v);
        }
        if self.stack.is_empty() {
            let tighter = if upper { v < self.up[j] } else { v > self.lo[j] };
            if tighter {
                self.set_bound(p, j, upper, v, false);
            }
        } else {
            self.pending_global.push((j, upper, v));
        }
    }

    /// 根にいる間に積んだ変更を大域的なもの (巻き戻されないもの) にする。
    pub fn commit_root(&mut self) {
        self.stack.clear();
        self.global_lo.clone_from(&self.lo);
        self.global_up.clone_from(&self.up);
    }

    /// 伝播を待っている行をすべて処理する。矛盾がなければ真。
    pub fn propagate(&mut self, p: &MipProblem) -> bool {
        if self.infeasible {
            return false;
        }
        let nnz_limit = 20 * (p.m + p.n) + 100_000;
        let mut work = 0usize;
        while let Some(i) = self.queue.pop() {
            self.in_queue[i] = false;
            // 走査を省いた行は数えない
            let scanned = self.row_can_tighten(p, i);
            if scanned {
                work += p.rows[i].len();
                DEBUG_WORK.with(|w| w.set(w.get() + p.rows[i].len() as u64));
            }
            self.cur_reason = i as i32;
            let ok = self.propagate_row(p, i);
            self.cur_reason = -1;
            if !ok {
                self.infeasible = true;
                break;
            }
            if self.infeasible {
                break;
            }
            if work > nnz_limit {
                // 打ち切り (残りは次回に回す)
                break;
            }
        }
        !self.infeasible
    }

    /// 行 `i` からどれかの境界が締まりうるか (走査が必要か)。
    fn row_can_tighten(&self, p: &MipProblem, i: usize) -> bool {
        let cap = self.row_cap[i];
        let (lo_r, up_r) = (p.row_lo[i], p.row_up[i]);
        let up_side = up_r.is_finite() && self.min_inf[i] <= 1 && !(self.min_inf[i] == 0 && up_r - self.min_act[i] >= cap - 1e-9 * (1.0 + cap));
        let lo_side = lo_r.is_finite() && self.max_inf[i] <= 1 && !(self.max_inf[i] == 0 && self.max_act[i] - lo_r >= cap - 1e-9 * (1.0 + cap));
        up_side || lo_side
    }

    fn propagate_row(&mut self, p: &MipProblem, i: usize) -> bool {
        let lo_r = p.row_lo[i];
        let up_r = p.row_up[i];
        if up_r.is_finite() && self.min_inf[i] == 0 && self.min_act[i] > up_r + row_tol(up_r, self.min_act[i]) {
            if self.conflict.is_none() {
                self.conflict = Some(ConflictSrc::Row(i, true));
            }
            return false;
        }
        if lo_r.is_finite() && self.max_inf[i] == 0 && self.max_act[i] < lo_r - row_tol(lo_r, self.max_act[i]) {
            if self.conflict.is_none() {
                self.conflict = Some(ConflictSrc::Row(i, false));
            }
            return false;
        }
        // 余裕が行の最大の幅以上なら、その側からはどの境界も締まらない
        let cap = self.row_cap[i];
        let use_up = up_r.is_finite() && self.min_inf[i] <= 1 && !(self.min_inf[i] == 0 && up_r - self.min_act[i] >= cap - 1e-9 * (1.0 + cap));
        let use_lo = lo_r.is_finite() && self.max_inf[i] <= 1 && !(self.max_inf[i] == 0 && self.max_act[i] - lo_r >= cap - 1e-9 * (1.0 + cap));
        if !use_up && !use_lo {
            return true;
        }
        let row = &p.rows[i];
        for k in 0..row.len() {
            let (j, a) = row[k];
            if use_up && self.min_inf[i] <= 1 {
                let cmin = if a > 0.0 { a * self.lo[j] } else { a * self.up[j] };
                let rest = if cmin.is_finite() {
                    if self.min_inf[i] == 0 {
                        Some(self.min_act[i] - cmin)
                    } else {
                        None
                    }
                } else if self.min_inf[i] == 1 {
                    Some(self.min_act[i])
                } else {
                    None
                };
                if let Some(rest) = rest {
                    let v = (up_r - rest) / a;
                    if a > 0.0 {
                        self.propose_upper(p, j, v);
                    } else {
                        self.propose_lower(p, j, v);
                    }
                    if self.infeasible {
                        return false;
                    }
                }
            }
            if use_lo && self.max_inf[i] <= 1 {
                let cmax = if a > 0.0 { a * self.up[j] } else { a * self.lo[j] };
                let rest = if cmax.is_finite() {
                    if self.max_inf[i] == 0 {
                        Some(self.max_act[i] - cmax)
                    } else {
                        None
                    }
                } else if self.max_inf[i] == 1 {
                    Some(self.max_act[i])
                } else {
                    None
                };
                if let Some(rest) = rest {
                    let v = (lo_r - rest) / a;
                    if a > 0.0 {
                        self.propose_lower(p, j, v);
                    } else {
                        self.propose_upper(p, j, v);
                    }
                    if self.infeasible {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// 伝播で得た上限の候補を、意味のある締め付けなら適用する。
    fn propose_upper(&mut self, p: &MipProblem, j: usize, v: f64) {
        if !v.is_finite() || v.abs() > 1e15 {
            return;
        }
        let cur = self.up[j];
        let nv = if p.is_int[j] {
            (v + FEASTOL).floor()
        } else {
            // 数値誤差で実行可能点を切らないよう少し緩める。
            let nv = v + 1e-9 * (1.0 + v.abs());
            // 連続変数は、無限から有限になるか、幅の 30% 以上締まるときだけ採る (細かい締め付けの連鎖を防ぐ)。
            if cur.is_finite() {
                let range = cur - self.lo[j];
                let gain = cur - nv;
                if !(gain > 0.3 * range.max(1e-9) || (range.is_infinite() && gain > 1e-3 * (1.0 + cur.abs()))) {
                    return;
                }
            }
            nv
        };
        if nv < cur {
            self.set_bound(p, j, true, nv, true);
        }
    }

    fn propose_lower(&mut self, p: &MipProblem, j: usize, v: f64) {
        if !v.is_finite() || v.abs() > 1e15 {
            return;
        }
        let cur = self.lo[j];
        let nv = if p.is_int[j] {
            (v - FEASTOL).ceil()
        } else {
            let nv = v - 1e-9 * (1.0 + v.abs());
            if cur.is_finite() {
                let range = self.up[j] - cur;
                let gain = nv - cur;
                if !(gain > 0.3 * range.max(1e-9) || (range.is_infinite() && gain > 1e-3 * (1.0 + cur.abs()))) {
                    return;
                }
            }
            nv
        };
        if nv > cur {
            self.set_bound(p, j, false, nv, true);
        }
    }

    /// 衝突解析 (含意グラフ、1-UIP): 直前の伝播の矛盾 (`conflict`) を、その原因になった境界の変更に戻し、
    /// 伝播で生じた変更をその理由 (伝播した行の、その時点の他の列の境界) で置き換える、を最後の決定のレベルで
    /// 変更が 1 つになるまで (または置き換えられない変更に当たるまで) 続ける。戻り値は、同時には成り立たない
    /// 境界の組 (列, 上限か, 値) (下限なら `x_j >= 値`、上限なら `x_j <= 値`)。根の境界 (記録にない) は大域的に
    /// 成り立つので含めない。矛盾がない、決定がない (根で矛盾)、大きすぎる場合は `None`。
    pub fn analyze_conflict(&self, p: &MipProblem, max_len: usize) -> Option<Vec<(usize, bool, f64)>> {
        let src = self.conflict?;
        self.resolve_conflict(p, Err(src), max_len, false)
    }

    /// [`Self::analyze_conflict`] で、最後の決定のレベルだけでなく全てのレベルの伝播による変更を理由で置き換え、
    /// 決定 (分枝など) だけの組にする (1-UIP の組が 0-1 列以外の境界を含んで線形の行にできないときの代わり)。
    pub fn analyze_conflict_decisions(&self, p: &MipProblem, max_len: usize) -> Option<Vec<(usize, bool, f64)>> {
        let src = self.conflict?;
        self.resolve_conflict(p, Err(src), max_len, true)
    }

    /// 今のノードで破れている証明 `sum coefs x <= rhs` (双対証明・Farkas の証明) から衝突を作る (HiGHS の
    /// `conflictAnalysis` と同じ): 証明の最小活動量を上げている境界の変更 (係数が正なら下限、負なら上限) から始めて、
    /// [`Self::analyze_conflict`] と同じく 1-UIP まで理由で置き換える。
    /// `decisions` なら [`Self::analyze_conflict_decisions`] と同じく決定だけの組にする。
    pub fn analyze_proof_conflict(&self, p: &MipProblem, coefs: &[(usize, f64)], max_len: usize, decisions: bool) -> Option<Vec<(usize, bool, f64)>> {
        self.resolve_conflict(p, Ok(coefs), max_len, decisions)
    }

    /// 以後の境界の変更を「外から来た変更」(双対証明・衝突プール・オービトープの固定など、理由で置き換えられないが
    /// 分枝の決定でもないもの、理由 `-2`) として記録する (`false` で分枝の決定 `-1` に戻す)。衝突解析の深さの
    /// 段は分枝の決定だけで区切る。
    pub fn set_external(&mut self, on: bool) {
        self.cur_reason = if on { -2 } else { -1 };
    }

    /// HiGHS 式の衝突解析 (`HighsDomain::ConflictSet::conflictAnalysis`)。矛盾 (`start`) を、
    /// (1) 必要な分だけの境界の変更で説明し (`resolveLinearLeq`: 大域的な境界からの寄与の大きい変更から、矛盾を示すのに
    /// 足りるまで選ぶ)、(2) 分枝の深さの段ごとに、その段の変更が 1 つ (UIP) になるまで理由で置き換えて衝突を作り
    /// (最後の段から、衝突が出る限り、少なくとも 5 段)、(3) その UIP が伝播による変更なら、UIP を同じ段の理由で
    /// 置き換えた組と UIP の否定から「再収束」の衝突も作る。各衝突は同時には成り立たない境界のリテラルの組
    /// (列, 上限か, 値)。長さ `max_len` を超える衝突は作らない。
    pub fn analyze_multi(&self, p: &MipProblem, start: ConflictStart, max_len: usize, reconv: bool) -> Vec<Vec<(usize, bool, f64)>> {
        let end = self.stack.len();
        let dec: Vec<usize> = (0..end).filter(|&k| self.stack[k].reason == -1).collect();
        if dec.is_empty() {
            return Vec::new();
        }
        let (rlo, rup) = self.root_bounds();
        let glo = |j: usize| rlo[j].max(self.global_lo[j]);
        let gup = |j: usize| rup[j].min(self.global_up[j]);
        // (列, 側) ごとの最後の変更と、各変更の 1 つ前の同じ (列, 側) の変更 (前に辿って `before` より前のものを探す)
        const NONE: usize = usize::MAX;
        let mut last_lo = vec![NONE; p.n];
        let mut last_up = vec![NONE; p.n];
        let mut prev = vec![NONE; end];
        for (k, c) in self.stack.iter().enumerate() {
            let l = if c.upper { &mut last_up[c.col] } else { &mut last_lo[c.col] };
            prev[k] = *l;
            *l = k;
        }
        let latest_before = |j: usize, upper: bool, before: usize| -> Option<usize> {
            let mut k = if upper { last_up[j] } else { last_lo[j] };
            while k != NONE && k >= before {
                k = prev[k];
            }
            if k == NONE { None } else { Some(k) }
        };
        // 1 回の解析で調べる行の項の数の上限 (長い行の問題で解析が重くなりすぎないように)
        let budget = std::cell::Cell::new(tunable!("ENOMOTO_T_MIP_CONFLICT_WORK", 200_000usize, usize));
        // `sum_j a_j x_j >= m_req` (x_j は a_j > 0 なら下限、a_j < 0 なら上限) を示す、位置 `before` より前の変更の組。
        // 大域的な境界で足りない分を、寄与の大きい変更から選ぶ。足りなければ関わる変更を全部返す (従来の説明)
        let explain = |terms: &mut dyn Iterator<Item = (usize, f64)>, m_req: f64, before: usize, skip: usize| -> Option<Vec<usize>> {
            let mut base = 0.0f64;
            let mut cands: Vec<(f64, usize)> = Vec::new();
            let (lo_n, hi_n) = terms.size_hint();
            let n_terms = hi_n.unwrap_or(lo_n);
            if budget.get() < n_terms {
                return None;
            }
            budget.set(budget.get() - n_terms);
            let mut chosen: Vec<usize> = Vec::new();
            let mut all: Vec<usize> = Vec::new();
            for (j, a) in terms {
                if j == skip || a == 0.0 {
                    continue;
                }
                let upper = a < 0.0;
                let g = if upper { gup(j) } else { glo(j) };
                match latest_before(j, upper, before) {
                    Some(k) => {
                        let l = self.stack[k].new;
                        all.push(k);
                        if g.is_finite() {
                            base += a * g;
                            let d = a * (l - g);
                            if d > 0.0 {
                                cands.push((d, k));
                            }
                        } else {
                            base += a * l;
                            chosen.push(k);
                        }
                    }
                    None => {
                        if !g.is_finite() {
                            return None;
                        }
                        base += a * g;
                    }
                }
            }
            if !base.is_finite() {
                return Some(all);
            }
            let tol = 1e-9 * (1.0 + m_req.abs());
            if base < m_req - tol {
                cands.sort_by(|x, y| y.0.total_cmp(&x.0));
                for (d, k) in cands {
                    base += d;
                    chosen.push(k);
                    if base >= m_req - tol {
                        break;
                    }
                }
            }
            if base >= m_req - tol { Some(chosen) } else { Some(all) }
        };
        // 伝播による変更 `k` の説明
        let explain_change = |k: usize| -> Option<Vec<usize>> {
            let c = &self.stack[k];
            if c.reason < 0 {
                return None;
            }
            let r = c.reason as usize;
            let a_c = p.rows[r].iter().find(|&&(j, _)| j == c.col).map(|&(_, a)| a)?;
            let v = c.new;
            // 整数列の境界は丸めてある: 上限 v は (rhs - rest)/a < v + 1 - FEASTOL から、下限 v は > v - 1 + FEASTOL から
            let w = if p.is_int[c.col] { if c.upper { v + 1.0 - FEASTOL } else { v - 1.0 + FEASTOL } } else { v };
            let min_side = (a_c > 0.0) == c.upper;
            if min_side {
                let u = p.row_up[r];
                explain(&mut p.rows[r].iter().copied(), u - a_c * w, k, c.col)
            } else {
                let l = p.row_lo[r];
                explain(&mut p.rows[r].iter().map(|&(j, a)| (j, -a)), -l + a_c * w, k, c.col)
            }
        };
        let init: Vec<usize> = match start {
            ConflictStart::Src(ConflictSrc::Row(i, true)) => {
                let u = p.row_up[i];
                match explain(&mut p.rows[i].iter().copied(), u + row_tol(u, u), end, usize::MAX) {
                    Some(v) => v,
                    None => return Vec::new(),
                }
            }
            ConflictStart::Src(ConflictSrc::Row(i, false)) => {
                let l = p.row_lo[i];
                match explain(&mut p.rows[i].iter().map(|&(j, a)| (j, -a)), -l + row_tol(l, l), end, usize::MAX) {
                    Some(v) => v,
                    None => return Vec::new(),
                }
            }
            ConflictStart::Src(ConflictSrc::Col(j)) => latest_before(j, false, end).into_iter().chain(latest_before(j, true, end)).collect(),
            ConflictStart::Proof(coefs, rhs) => {
                // 証明 `sum coefs x <= rhs` が破れている: 最小活動量 > rhs
                match explain(&mut coefs.iter().copied(), rhs + 1e-6 * (1.0 + rhs.abs()), end, usize::MAX) {
                    Some(v) => v,
                    None => return Vec::new(),
                }
            }
        };
        if init.is_empty() || init.len() > max_len {
            return Vec::new();
        }
        let to_lits = |set: &std::collections::BTreeSet<usize>| -> Vec<(usize, bool, f64)> {
            // 同じ列・側は最も新しい (きつい) ものだけ
            let mut m: std::collections::BTreeMap<(usize, bool), usize> = std::collections::BTreeMap::new();
            for &k in set {
                let c = &self.stack[k];
                let e = m.entry((c.col, c.upper)).or_insert(k);
                if k > *e {
                    *e = k;
                }
            }
            m.values().map(|&k| (self.stack[k].col, self.stack[k].upper, self.stack[k].new)).collect()
        };
        let cap = 4 * max_len + 100;
        let mut set: std::collections::BTreeSet<usize> = init.into_iter().collect();
        let mut out: Vec<Vec<(usize, bool, f64)>> = Vec::new();
        let mut lev = dec.len();
        let mut done = 0usize;
        while lev >= 1 {
            let lo = dec[lev - 1];
            let hi = if lev < dec.len() { dec[lev] } else { end };
            // この段の変更が 1 つになるまで、最も新しい伝播による変更を理由で置き換える
            let mut resolved = 0usize;
            let mut cnt = set.range(lo..hi).count();
            while cnt > 1 {
                if set.len() > cap {
                    return out;
                }
                let Some(&k) = set.range(lo..hi).rev().find(|&&k| self.stack[k].reason >= 0) else { break };
                let Some(ex) = explain_change(k) else { return out };
                set.remove(&k);
                cnt -= 1;
                for e in ex {
                    if set.insert(e) && e >= lo && e < hi {
                        cnt += 1;
                    }
                }
                resolved += 1;
            }
            let inlev: Vec<usize> = set.range(lo..hi).copied().collect();
            if inlev.is_empty() {
                lev -= 1;
                continue;
            }
            done += 1;
            let before = out.len();
            // HiGHS と同じく、この段で置き換えが起きたときだけ衝突にする (最初の段で置き換えがなければ今の組を 1 つ作って
            // 止める: 矛盾は最後の分枝そのものから来ている)
            if resolved == 0 && done == 1 {
                let lits = to_lits(&set);
                if !lits.is_empty() && lits.len() <= max_len {
                    out.push(lits);
                }
                break;
            }
            let lits = to_lits(&set);
            if resolved > 0 && !lits.is_empty() && lits.len() <= max_len {
                out.push(lits);
            }
            // 再収束: UIP u (伝播による変更) を同じ段の理由で置き換えた組 R は u を導く。R かつ (u の否定) は矛盾
            if reconv && inlev.len() == 1 && self.stack[inlev[0]].reason >= 0 {
                let u = inlev[0];
                let mut r: std::collections::BTreeSet<usize> = std::iter::once(u).collect();
                let mut ok = true;
                loop {
                    if r.len() > cap {
                        ok = false;
                        break;
                    }
                    let Some(&k) = r.range(lo..hi).rev().find(|&&k| self.stack[k].reason >= 0) else { break };
                    let Some(ex) = explain_change(k) else {
                        ok = false;
                        break;
                    };
                    r.remove(&k);
                    r.extend(ex);
                }
                if ok && !r.contains(&u) && !r.is_empty() {
                    let mut lits = to_lits(&r);
                    let c = &self.stack[u];
                    let neg = if p.is_int[c.col] {
                        if c.upper { c.new + 1.0 } else { c.new - 1.0 }
                    } else if c.upper {
                        c.new + FEASTOL * (1.0 + c.new.abs())
                    } else {
                        c.new - FEASTOL * (1.0 + c.new.abs())
                    };
                    lits.retain(|&(j, up, _)| !(j == c.col && up != c.upper));
                    lits.push((c.col, !c.upper, neg));
                    if lits.len() <= max_len {
                        out.push(lits);
                    }
                }
            }
            let new_here = out.len() - before;
            // HiGHS と同じ打ち切り: 最初の段で衝突が出なければ止める。5 段以上調べて新しい衝突がなければ止める
            if out.is_empty() || (done >= 5 && new_here == 0) {
                break;
            }
            lev -= 1;
        }
        out
    }

    /// 衝突解析の本体。`start` は矛盾の出どころ (`Err`) か、破れている証明の係数 (`Ok`)。
    fn resolve_conflict(&self, p: &MipProblem, start: Result<&[(usize, f64)], ConflictSrc>, max_len: usize, decisions: bool) -> Option<Vec<(usize, bool, f64)>> {
        let Some(last_dec) = self.stack.iter().rposition(|c| c.reason < 0) else {
            CONFLICT_FAIL[0].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return None;
        };
        // (列, 側) ごとの記録の位置 (昇順)
        let mut hist: std::collections::HashMap<(usize, bool), Vec<usize>> = std::collections::HashMap::new();
        for (k, c) in self.stack.iter().enumerate() {
            hist.entry((c.col, c.upper)).or_default().push(k);
        }
        // 位置 `before` より前で (列, 側) を最後に変えた記録
        let latest_before = |j: usize, upper: bool, before: usize| -> Option<usize> {
            let v = hist.get(&(j, upper))?;
            let idx = v.partition_point(|&k| k < before);
            if idx == 0 { None } else { Some(v[idx - 1]) }
        };
        // 行 `i` の最小活動量 (`min_side`) または最大活動量に効く、位置 `before` より前の記録 (列 `skip` を除く)
        let explain_row = |i: usize, min_side: bool, before: usize, skip: usize, out: &mut Vec<usize>| {
            for &(j, a) in &p.rows[i] {
                if j == skip {
                    continue;
                }
                // 最小活動量: a > 0 なら下限、a < 0 なら上限。最大活動量はその逆
                let upper = if min_side { a < 0.0 } else { a > 0.0 };
                if let Some(k) = latest_before(j, upper, before) {
                    out.push(k);
                }
            }
        };
        let mut set: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
        let mut tmp: Vec<usize> = Vec::new();
        let end = self.stack.len();
        match start {
            Err(ConflictSrc::Row(i, min_side)) => explain_row(i, min_side, end, usize::MAX, &mut tmp),
            Err(ConflictSrc::Col(j)) => {
                tmp.extend(latest_before(j, false, end));
                tmp.extend(latest_before(j, true, end));
            }
            Ok(coefs) => {
                for &(j, a) in coefs {
                    tmp.extend(latest_before(j, a < 0.0, end));
                }
            }
        }
        set.extend(tmp.drain(..));
        for _ in 0..10_000 {
            if set.len() > 4 * max_len + 100 {
                CONFLICT_FAIL[1].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return None;
            }
            let k = if decisions {
                // 全てのレベルで、伝播による変更のうち最も新しいもの
                match set.iter().rev().find(|&&k| self.stack[k].reason >= 0) {
                    Some(&k) => k,
                    None => break,
                }
            } else {
                let cur: Vec<usize> = set.range(last_dec..).copied().collect();
                if cur.len() <= 1 {
                    break;
                }
                *cur.last().unwrap()
            };
            let c = &self.stack[k];
            if c.reason < 0 {
                break; // 置き換えられない (決定に相当する) 変更
            }
            let r = c.reason as usize;
            let Some(a) = p.rows[r].iter().find(|&&(j, _)| j == c.col).map(|&(_, a)| a) else {
                CONFLICT_FAIL[2].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return None;
            };
            // 上限を締めたのは a > 0 なら行の上限 (最小活動量) から、a < 0 なら下限 (最大活動量) から
            let min_side = (a > 0.0) == c.upper;
            set.remove(&k);
            explain_row(r, min_side, k, c.col, &mut tmp);
            set.extend(tmp.drain(..));
        }
        if set.is_empty() || set.len() > max_len {
            CONFLICT_FAIL[if set.is_empty() { 3 } else { 4 }].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if set.len() > max_len {
                CONFLICT_FAIL[5].fetch_add(set.len() as u64, std::sync::atomic::Ordering::Relaxed);
            }
            return None;
        }
        Some(set.iter().map(|&k| (self.stack[k].col, self.stack[k].upper, self.stack[k].new)).collect())
    }

    /// 列 `j` が固定されているか。
    pub fn is_fixed(&self, j: usize) -> bool {
        self.lo[j] == self.up[j]
    }
}

/// 診断用: 衝突解析が失敗した理由の回数 [決定なし, 途中で長すぎ, 理由の行に列がない, 空, 最後に長すぎ, その長さの和]
pub static CONFLICT_FAIL: [std::sync::atomic::AtomicU64; 6] = [const { std::sync::atomic::AtomicU64::new(0) }; 6];

#[inline]
fn bound_tol(v: f64) -> f64 {
    FEASTOL * (1.0 + v.abs()).min(1e3)
}

#[inline]
fn row_tol(rhs: f64, act: f64) -> f64 {
    FEASTOL * (1.0 + rhs.abs().max(act.abs())).min(1e6)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn knap() -> MipProblem {
        // 3x0 + 4x1 + 5x2 <= 7, x 0/1 整数; x0 + x1 >= 1
        MipProblem::from_rows(
            vec![0.0; 3],
            vec![1.0; 3],
            vec![-1.0, -1.0, -1.0],
            0.0,
            1.0,
            vec![true; 3],
            vec![vec![(0, 3.0), (1, 4.0), (2, 5.0)], vec![(0, 1.0), (1, 1.0)]],
            vec![f64::NEG_INFINITY, 1.0],
            vec![7.0, f64::INFINITY],
        )
    }

    #[test]
    fn propagation_and_backtrack() {
        let p = knap();
        let mut d = Domain::new(&p);
        assert!(d.propagate(&p));
        assert_eq!(d.up, vec![1.0, 1.0, 1.0]);
        let pos = d.stack_len();
        // x2 = 1 なら 3x0 + 4x1 <= 2 → x0 = x1 = 0 → 2 行目が矛盾
        d.tighten_lower(&p, 2, 1.0);
        assert!(!d.propagate(&p));
        d.backtrack_to(&p, pos);
        assert!(d.propagate(&p));
        // x0 = 0 → x1 >= 1 → x1 = 1 → 5x2 <= 3 → x2 = 0
        d.tighten_upper(&p, 0, 0.0);
        assert!(d.propagate(&p));
        assert_eq!((d.lo[1], d.up[2]), (1.0, 0.0));
        d.backtrack_to(&p, pos);
        assert_eq!(d.lo, vec![0.0; 3]);
        assert_eq!(d.up, vec![1.0; 3]);
    }
}
