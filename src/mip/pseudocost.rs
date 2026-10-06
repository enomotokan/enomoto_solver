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
            min_reliable: 8,
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

    /// 推定値への寄与 (両側のうち小さい方の増加)。
    pub fn estimate_gain(&self, j: usize, frac: f64) -> f64 {
        (self.cost_up(j) * (1.0 - frac)).min(self.cost_down(j) * frac)
    }
}
