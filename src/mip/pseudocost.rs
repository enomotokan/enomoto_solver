//! pseudocost (分枝による目的値の増加の単位あたりの平均) と分枝スコア
//! (HiGHS の `HighsPseudocost` を簡略化したもの)。

/// 列ごとの pseudocost と観測回数。
pub struct Pseudocost {
    sum_up: Vec<f64>,
    n_up: Vec<u32>,
    sum_down: Vec<f64>,
    n_down: Vec<u32>,
    /// 全列の平均 (未観測の列の代わりに使う)。
    total_up: f64,
    total_n_up: u64,
    total_down: f64,
    total_n_down: u64,
    /// 分枝後に片側が実行不能/打ち切りになった回数 (cutoff スコア)。
    cutoff_up: Vec<u32>,
    cutoff_down: Vec<u32>,
    /// これ以上観測があれば信頼する。
    pub min_reliable: u32,
    /// 分枝後の伝播で締まった境界の数 (推論) の和と回数 (列・向きごと)。
    inf_sum: [Vec<f64>; 2],
    inf_n: [Vec<u32>; 2],
    inf_total: f64,
    inf_total_n: u64,
    /// 実行不能で終わった葉と、目的値で打ち切った葉の数 (スコアの動的な重み)。
    pub infeasible_leaves: u64,
    pub objlim_leaves: u64,
    /// 衝突スコア (VSIDS と同じ): 衝突に現れた列・向きに `conflict_inc` を足し、`conflict_inc` を衝突ごとに 2% 増やす
    /// (古い衝突の重みが相対的に下がる)。`[0]` は下向き (`x <= v` のリテラル)、`[1]` は上向き (`x >= v`)。
    conflict: [Vec<f64>; 2],
    conflict_inc: f64,
    conflict_total: f64,
}

impl Pseudocost {
    pub fn new(n: usize) -> Self {
        Pseudocost {
            sum_up: vec![0.0; n],
            n_up: vec![0; n],
            sum_down: vec![0.0; n],
            n_down: vec![0; n],
            total_up: 0.0,
            total_n_up: 0,
            total_down: 0.0,
            total_n_down: 0,
            cutoff_up: vec![0; n],
            cutoff_down: vec![0; n],
            min_reliable: tunable!("ENOMOTO_T_MIP_MINREL", 8u32, u32),
            inf_sum: [vec![0.0; n], vec![0.0; n]],
            inf_n: [vec![0; n], vec![0; n]],
            inf_total: 0.0,
            inf_total_n: 0,
            infeasible_leaves: 0,
            objlim_leaves: 0,
            conflict: [vec![0.0; n], vec![0.0; n]],
            conflict_inc: 1.0,
            conflict_total: 0.0,
        }
    }

    /// 衝突 (リテラル (列, 上限か) の組) を衝突スコアに加える。
    pub fn add_conflict(&mut self, lits: &[(usize, bool, f64)]) {
        for &(j, upper, _) in lits {
            let d = (!upper) as usize;
            self.conflict[d][j] += self.conflict_inc;
            self.conflict_total += self.conflict_inc;
        }
        self.conflict_inc *= 1.02;
        // 桁あふれを防ぐため、大きくなったら全体を縮める (比だけが意味を持つ)
        if self.conflict_inc > 1e100 {
            let f = 1e-100;
            for v in self.conflict.iter_mut().flat_map(|c| c.iter_mut()) {
                *v *= f;
            }
            self.conflict_inc *= f;
            self.conflict_total *= f;
        }
    }

    /// 分枝の観測を加える: 列 `j` を `delta` (>0) だけ動かしたら目的値が `gain` (>=0) 増えた。
    pub fn add_observation(&mut self, j: usize, up: bool, delta: f64, gain: f64) {
        if delta <= 1e-9 || !gain.is_finite() {
            return;
        }
        let unit = gain.max(0.0) / delta;
        if up {
            self.sum_up[j] += unit;
            self.n_up[j] += 1;
            self.total_up += unit;
            self.total_n_up += 1;
        } else {
            self.sum_down[j] += unit;
            self.n_down[j] += 1;
            self.total_down += unit;
            self.total_n_down += 1;
        }
    }

    /// 片側が実行不能/打ち切りになった記録。
    pub fn add_cutoff(&mut self, j: usize, up: bool) {
        if up {
            self.cutoff_up[j] += 1;
        } else {
            self.cutoff_down[j] += 1;
        }
    }

    fn avg_up(&self) -> f64 {
        if self.total_n_up > 0 { self.total_up / self.total_n_up as f64 } else { 1.0 }
    }

    fn avg_down(&self) -> f64 {
        if self.total_n_down > 0 { self.total_down / self.total_n_down as f64 } else { 1.0 }
    }

    /// 上向きの単位あたりコスト (観測が少なければ全体平均と混ぜる)。
    pub fn cost_up(&self, j: usize) -> f64 {
        let n = self.n_up[j];
        let avg = self.avg_up();
        if n == 0 {
            return avg;
        }
        let own = self.sum_up[j] / n as f64;
        let w = (0.9 + 0.1 * n as f64 / self.min_reliable as f64).min(1.0);
        w * own + (1.0 - w) * avg
    }

    pub fn cost_down(&self, j: usize) -> f64 {
        let n = self.n_down[j];
        let avg = self.avg_down();
        if n == 0 {
            return avg;
        }
        let own = self.sum_down[j] / n as f64;
        let w = (0.9 + 0.1 * n as f64 / self.min_reliable as f64).min(1.0);
        w * own + (1.0 - w) * avg
    }

    /// 両側とも十分観測されているか。
    pub fn is_reliable(&self, j: usize) -> bool {
        self.n_up[j] >= self.min_reliable && self.n_down[j] >= self.min_reliable
    }

    /// 両側の観測回数の少ない方。
    pub fn min_observations(&self, j: usize) -> u32 {
        self.n_up[j].min(self.n_down[j])
    }

    /// 分枝スコア (product score。小数部 `frac` = x - floor(x))。
    pub fn score(&self, j: usize, frac: f64) -> f64 {
        let up = self.cost_up(j) * (1.0 - frac);
        let down = self.cost_down(j) * frac;
        let eps = 1e-6;
        let cost = up.max(eps) * down.max(eps);
        // 片側が打ち切りになりやすい列を少し優先する。
        let cut = (self.cutoff_up[j] + self.cutoff_down[j]) as f64;
        cost * (1.0 + 1e-2 * cut.min(100.0))
    }

    /// 分枝 (列 `j`、向き `up`) の後の伝播で締まった境界の数を記録する。
    pub fn add_inference(&mut self, j: usize, up: bool, count: f64) {
        let d = up as usize;
        self.inf_sum[d][j] += count;
        self.inf_n[d][j] += 1;
        self.inf_total += count;
        self.inf_total_n += 1;
    }

    fn inference(&self, j: usize, up: bool) -> f64 {
        let d = up as usize;
        let avg = if self.inf_total_n > 0 { self.inf_total / self.inf_total_n as f64 } else { 0.0 };
        if self.inf_n[d][j] > 0 { self.inf_sum[d][j] / self.inf_n[d][j] as f64 } else { avg }
    }

    /// 打ち切り率 (分枝した子が実行不能/打ち切りになった割合)。
    fn cutoff_rate(&self, j: usize, up: bool) -> f64 {
        let (c, n) = if up { (self.cutoff_up[j], self.n_up[j]) } else { (self.cutoff_down[j], self.n_down[j]) };
        c as f64 / (c + n + 1) as f64
    }

    /// SCIP の relpscost のスコア (branch_relpscost.c `calcScore`):
    /// `dyn * (1e-4 s(推論) + 1e-4 s(打ち切り)) + s(pseudocost) / dyn`。`s(v, avg) = 1 - 1/(1 + v/max(avg, 0.1))`、
    /// `dyn = (実行不能の葉 + 1) / (打ち切りの葉 + 1)` (暫定解がない間は推論・打ち切りの比重が大きくなる)。
    /// `down`/`up` は両側の目的値の増加の見積り (pseudocost × 小数部、または強分岐の実測値)。
    fn hybrid(&self, j: usize, down: f64, up: f64) -> f64 {
        let s = |v: f64, avg: f64| 1.0 - 1.0 / (1.0 + v / avg.max(0.1));
        let eps = 1e-6;
        let ps = down.max(eps) * up.max(eps);
        let ps_avg = self.avg_down().max(eps) * self.avg_up().max(eps) * 0.25;
        let inf = self.inference(j, false).max(eps) * self.inference(j, true).max(eps);
        let inf_avg = if self.inf_total_n > 0 { (self.inf_total / self.inf_total_n as f64).max(eps).powi(2) } else { 1.0 };
        let cut = self.cutoff_rate(j, false).max(eps) * self.cutoff_rate(j, true).max(eps);
        let dynw = (self.infeasible_leaves + 1) as f64 / (self.objlim_leaves + 1) as f64;
        // 衝突スコア (SCIP の conflictweight は 0.01。ここでは既定 0: misc07・rout で下界の伸びが悪くなった): 両側の積を、全列の平均の 2 乗で正規化する
        let n = self.conflict[0].len().max(1) as f64;
        let conf_avg = (self.conflict_total / (2.0 * n)).max(eps);
        let conf = self.conflict[0][j].max(eps * conf_avg) * self.conflict[1][j].max(eps * conf_avg);
        let wc = tunable!("ENOMOTO_T_MIP_CONFLICT_WEIGHT", 0.0, f64);
        let conf_term = if self.conflict_total > 0.0 { wc * s(conf, conf_avg * conf_avg) } else { 0.0 };
        dynw * (1e-4 * s(inf, inf_avg) + 1e-4 * s(cut, 0.01) + conf_term) + s(ps, ps_avg) / dynw
    }

    /// pseudocost による [`Self::hybrid`] スコア (小数部 `frac`)。
    pub fn hybrid_score(&self, j: usize, frac: f64) -> f64 {
        self.hybrid(j, self.cost_down(j) * frac, self.cost_up(j) * (1.0 - frac))
    }

    /// 強分岐の実測値による [`Self::hybrid`] スコア。
    pub fn hybrid_score_with_gains(&self, j: usize, down: f64, up: f64) -> f64 {
        self.hybrid(j, down, up)
    }

    /// 推定値への寄与 (両側のうち小さい方の増加)。
    pub fn estimate_gain(&self, j: usize, frac: f64) -> f64 {
        (self.cost_up(j) * (1.0 - frac)).min(self.cost_down(j) * frac)
    }
}
