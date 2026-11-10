//! 確率的な強分岐の打ち切り (PLSB、試作、`ENOMOTO_MIP_PLSB`、既定では使わない)
//!
//! Mexi, Shamsi, Besançon, Le Bodic「Probabilistic Lookahead Strong Branching via a Stochastic Abstract Branching Model」
//! (CPAIOR 2024) と SCIP 10 の `branching/relpscost/dynamiclookahead` (`branch_relpscost.c` の
//! `continueStrongBranchingTreeSizeEstimation`・`expectedTreeSize`・`updateMinMaxMeanGain`) にならう。
//!
//! 強分岐した候補の左右の利得の幾何平均 g = √((g⁻ + 0.01)(g⁺ + 0.01)) − 0.01 を、0 の点の重み (zeroprob) と正の部分の連続分布
//! (既定はパレート分布、形の値は最尤 λ = n / (Σ log g_i − n log g_min)。ほかに対数正規・指数) の混ぜ合わせからの標本とみなす。
//! 今の最良 g* でギャップ G を閉じる木の深さ d = ⌈G / g*⌉、大きさ T(d) = 2^(d+1) − 1 (Le Bodic・Nemhauser の抽象的な分枝の模型)。
//! もう 1 本強分岐したときの期待の木の大きさ E[T'] = Σ_k P(新しい深さ = k) T(k) + 費用 が T(d) − 余裕 以下なら続ける。
//!
//! 環境変数 `ENOMOTO_MIP_PLSB` の文字 (値は `1` でもよい。文字がなければ既定の組み合わせ):
//! - 統計の範囲: 既定はノードごと (SCIP 10 の実装の意図。SCIP は `currndualgains` だけ戻し忘れている、issue #247)。
//!   `g` で木全体 (このスレッドの求解の全ての強分岐)
//! - 分布: 既定はパレート、`l` で対数正規、`e` で指数
//! - 打ち切りの範囲: 既定は SCIP と同じく、従来の lookahead の数の 0.6 倍 (`ENOMOTO_T_MIP_PLSB_QUOT`) に達してから判定し、
//!   打ち切るだけ (従来より長くは続けない)。`x` で従来の lookahead を外し、標本が足りるときは PLSB だけで判定する
//!   (標本が足りないうちは従来の打ち切り。候補の数・反復の予算の上限は残す)
//! - 費用: 既定は SCIP と同じく強分岐 1 本 = 2 ノード、続ける条件は E[T'] <= T(d) − 1。`c` で強分岐 1 本 = (強分岐の LP 1 回の
//!   平均反復 × 2) / (ノードの LP の平均反復) ノード (`ENOMOTO_T_MIP_PLSB_COSTMULT` 倍) にし、続ける条件は E[T'] <= T(d)
//!
//! - `u`: 深さ 50 以上でも判定する (上限 1000。SCIP は 50 以上なら判定せずに続ける)
//!
//! 正の観測が `ENOMOTO_T_MIP_PLSB_MINSAMPLE` (既定 10、SCIP の `minsamplesize`) 未満、暫定解がない、g* が 0、d が 50 以上の
//! ときは判定しない (従来の打ち切り。SCIP と同じ)。

use super::lp_api::MipLp;
use super::solver::Solver;

/// SCIP の `GEOMMEANSHIFT`
const SHIFT: f64 = 0.01;
/// SCIP の `MINGAINTHRESHOLD`: これより小さい利得は 0 とみなし、これより小さい改善の確率は 0 とみなす
const MIN_GAIN: f64 = 1e-5;
/// SCIP の `MAXGAINTHRESHOLD`
const MAX_GAIN: f64 = 1e15;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Dist {
    Pareto,
    LogNormal,
    Exponential,
}

/// 設定 (`ENOMOTO_MIP_PLSB` の文字)
#[derive(Clone, Copy, Debug)]
pub(crate) struct PlsbCfg {
    pub global: bool,
    pub dist: Dist,
    pub replace: bool,
    pub iter_cost: bool,
    pub uncapped: bool,
    pub min_sample: usize,
    pub quot: f64,
    pub cost_mult: f64,
}

/// `ENOMOTO_MIP_PLSB` が付いていれば設定を返す
pub(crate) fn plsb_cfg() -> Option<PlsbCfg> {
    static V: std::sync::OnceLock<Option<PlsbCfg>> = std::sync::OnceLock::new();
    *V.get_or_init(|| {
        let s = std::env::var("ENOMOTO_MIP_PLSB").ok()?;
        if s == "0" || s == "off" {
            return None;
        }
        let dist = if s.contains('l') {
            Dist::LogNormal
        } else if s.contains('e') {
            Dist::Exponential
        } else {
            Dist::Pareto
        };
        Some(PlsbCfg {
            global: s.contains('g'),
            dist,
            replace: s.contains('x'),
            iter_cost: s.contains('c'),
            uncapped: s.contains('u'),
            min_sample: tunable!("ENOMOTO_T_MIP_PLSB_MINSAMPLE", 10usize, usize),
            quot: tunable!("ENOMOTO_T_MIP_PLSB_QUOT", 0.6, f64),
            cost_mult: tunable!("ENOMOTO_T_MIP_PLSB_COSTMULT", 1.0, f64),
        })
    })
}

/// 利得の統計 (SCIP の `updateMinMaxMeanGain` と同じもの。対数正規の分散は和から求める)
#[derive(Clone, Debug, Default)]
pub(crate) struct GainStats {
    /// 正の観測の数
    pub n: usize,
    /// 0 とみなした観測の数
    pub nzero: usize,
    pub sum: f64,
    pub sum_log: f64,
    pub sum_log2: f64,
    pub min: f64,
    pub max: f64,
}

impl GainStats {
    pub fn new() -> Self {
        GainStats { min: f64::INFINITY, max: f64::NEG_INFINITY, ..Default::default() }
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// 左右の利得 (どちらも >= 0) の観測を足す。足した幾何平均を返す (大きすぎて捨てたなら None)
    pub fn add(&mut self, down: f64, up: f64) -> Option<f64> {
        if !(down < MAX_GAIN && up < MAX_GAIN) {
            return None;
        }
        let g = mean_gain(down, up);
        if g < MIN_GAIN {
            self.nzero += 1;
            return Some(0.0);
        }
        self.n += 1;
        self.sum += g;
        let lg = g.ln();
        self.sum_log += lg;
        self.sum_log2 += lg * lg;
        self.min = self.min.min(g);
        self.max = self.max.max(g);
        Some(g)
    }

    pub fn merge(&mut self, o: &GainStats) {
        self.n += o.n;
        self.nzero += o.nzero;
        self.sum += o.sum;
        self.sum_log += o.sum_log;
        self.sum_log2 += o.sum_log2;
        self.min = self.min.min(o.min);
        self.max = self.max.max(o.max);
    }

    fn zero_prob(&self) -> f64 {
        self.nzero as f64 / (self.nzero + self.n).max(1) as f64
    }
}

/// 左右の利得の幾何平均 (SCIP の `GEOMMEANSHIFT` でずらす)
pub(crate) fn mean_gain(down: f64, up: f64) -> f64 {
    ((down.max(0.0) + SHIFT) * (up.max(0.0) + SHIFT)).sqrt() - SHIFT
}

/// 当てはめた分布
#[derive(Clone, Copy, Debug)]
struct Fitted {
    dist: Dist,
    zero: f64,
    /// パレートの形の値・指数の率
    rate: f64,
    /// パレートの下限
    min: f64,
    /// 対数正規の対数の平均・標準偏差
    mu: f64,
    sigma: f64,
}

impl Fitted {
    fn fit(st: &GainStats, dist: Dist) -> Option<Fitted> {
        let n = st.n as f64;
        if st.n == 0 {
            return None;
        }
        let zero = st.zero_prob();
        match dist {
            Dist::Pareto => {
                let s = st.sum_log - n * st.min.ln();
                if s.abs() < 1e-9 {
                    return None;
                }
                Some(Fitted { dist, zero, rate: n / s, min: st.min, mu: 0.0, sigma: 0.0 })
            }
            Dist::Exponential => {
                let mean = st.sum / n;
                if mean <= 0.0 {
                    return None;
                }
                Some(Fitted { dist, zero, rate: 1.0 / mean, min: 0.0, mu: 0.0, sigma: 0.0 })
            }
            Dist::LogNormal => {
                if st.n < 2 {
                    return None;
                }
                let mu = st.sum_log / n;
                let var = ((st.sum_log2 - n * mu * mu) / (n - 1.0)).max(0.0);
                // SCIP は `logstdevgain` に分散を入れて erfc の分母に使っている (名前は標準偏差)。ここでは標準偏差を使う
                let sigma = var.sqrt();
                if sigma < 1e-9 {
                    return None;
                }
                Some(Fitted { dist, zero, rate: 0.0, min: 0.0, mu, sigma })
            }
        }
    }

    /// 混ぜ合わせの累積分布 F(x) = zero + (1 − zero) F_c(x)
    fn cdf(&self, x: f64) -> f64 {
        let fc = match self.dist {
            Dist::Pareto => {
                if x < self.min {
                    // 連続の部分は 0 (SCIP はここで混ぜ合わせ全体に 0 を返し、0 の点の重みも落ちる。ここでは F = zero)
                    0.0
                } else {
                    1.0 - (self.min / x).powf(self.rate)
                }
            }
            Dist::Exponential => 1.0 - (-self.rate * x).exp(),
            Dist::LogNormal => {
                if x <= 0.0 {
                    0.0
                } else {
                    0.5 * erfc(-(x.ln() - self.mu) / (self.sigma * std::f64::consts::SQRT_2))
                }
            }
        };
        self.zero + (1.0 - self.zero) * fc
    }
}

/// erfc (Numerical Recipes の `erfcc`、相対誤差 1.2e-7)
fn erfc(x: f64) -> f64 {
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.5 * z);
    let r = t * (-z * z - 1.26551223
        + t * (1.00002368 + t * (0.37409196 + t * (0.09678418 + t * (-0.18628806 + t * (0.27886807 + t * (-1.13520398 + t * (1.48851587 + t * (-0.82215223 + t * 0.17087277)))))))))
        .exp();
    if x >= 0.0 {
        r
    } else {
        2.0 - r
    }
}

/// T(d) = 2^(d+1) − 1
fn tree_size(d: f64) -> f64 {
    (2.0f64).powf(d + 1.0) - 1.0
}

/// 判定の結果
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// 判定できない: 従来の打ち切り。理由は 0 = 標本が足りない、1 = 暫定解がない・g* が 0・当てはめられない、2 = 深さが 50 以上
    Undecided(u8),
    Continue,
    Stop,
}

/// もう 1 本強分岐するかの判定。`gap` = 打ち切り値 − ノードの LP 値、`best` = このノードで強分岐した候補の最良の幾何平均の利得、
/// `cost` = 強分岐 1 本の費用 (ノード数)、`margin` = 続けるのに要る木の大きさの減り (SCIP は 1)
pub(crate) fn decide(st: &GainStats, cfg: &PlsbCfg, gap: f64, best: f64, cost: f64, margin: f64) -> Verdict {
    if st.n < cfg.min_sample {
        return Verdict::Undecided(0);
    }
    if !gap.is_finite() || gap <= 0.0 || !(best > 0.0) {
        return Verdict::Undecided(1);
    }
    let Some(f) = Fitted::fit(st, cfg.dist) else {
        return Verdict::Undecided(1);
    };
    let d = (gap / best.max(MIN_GAIN)).ceil().max(1.0);
    if d >= if cfg.uncapped { 1000.0 } else { 50.0 } {
        return Verdict::Undecided(2);
    }
    // 深さを 1 つでも減らす確率が `MIN_GAIN` 未満なら止める (SCIP の `expectedTreeSize` は無限大を返す)
    if d >= 2.0 && 1.0 - f.cdf(gap / (d - 1.0)) < MIN_GAIN {
        return Verdict::Stop;
    }
    let cur = tree_size(d);
    let next = expected_next(&f, gap, d) + cost;
    if next <= cur - margin {
        Verdict::Continue
    } else {
        Verdict::Stop
    }
}

/// もう 1 本強分岐した後の木の期待の大きさ (費用を除く)。新しい深さは min(d, ⌈G / g⌉)。P(⌈G/g⌉ <= k) = 1 − F(G/k)
fn expected_next(f: &Fitted, gap: f64, d: f64) -> f64 {
    let d = d as i64;
    if d <= 1 {
        return tree_size(1.0);
    }
    // P(新しい深さ <= k) = 1 − F(G/k) (k < d)、k = d なら 1
    let p_le = |k: i64| -> f64 { if k >= d { 1.0 } else { (1.0 - f.cdf(gap / k as f64)).clamp(0.0, 1.0) } };
    let mut e = 0.0;
    let mut prev = 0.0; // P(深さ <= k−1)
    for k in 1..=d {
        let p = p_le(k);
        let pk = (p - prev).max(0.0);
        if pk > 0.0 {
            e += pk * tree_size(k as f64);
        }
        prev = p.max(prev);
    }
    e
}

/// ノードの中の状態 (`select_branch_inner` の 1 回の呼び出しごと)
pub(crate) struct PlsbNode {
    pub cfg: PlsbCfg,
    pub stats: GainStats,
    /// このノードで強分岐した候補の幾何平均の利得の最大 (g*)
    pub best: f64,
    /// 一度 Stop と判定したら、以後の候補でも止める (標本は増えないので判定は変わらない)
    pub stopped: bool,
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// `ENOMOTO_MIP_PLSB` の設定 (サブ MIP では使わない)
    pub(super) fn plsb_begin(&self) -> Option<PlsbNode> {
        if self.params.submip {
            return None;
        }
        plsb_cfg().map(|cfg| PlsbNode { cfg, stats: GainStats::new(), best: 0.0, stopped: false })
    }

    /// 強分岐した候補の左右の利得を記録する
    pub(super) fn plsb_record(&mut self, pn: &mut PlsbNode, down: f64, up: f64) {
        if let Some(g) = pn.stats.add(down, up) {
            pn.best = pn.best.max(g);
        }
        if pn.cfg.global {
            self.plsb_tree.add(down, up);
        }
    }

    /// 従来の lookahead の打ち切りの代わり。`no_improve` は改善のない強分岐の数、`lookahead` は従来の上限
    pub(super) fn plsb_lookahead_stop(&mut self, pn: &mut PlsbNode, no_improve: f64, lookahead: f64, node_obj: f64) -> bool {
        let old = no_improve >= lookahead;
        if pn.stopped {
            return true;
        }
        if !pn.cfg.replace && (old || no_improve < pn.cfg.quot * lookahead) {
            return old;
        }
        let gap = self.prune_limit() - node_obj;
        let (cost, margin) = if pn.cfg.iter_cost {
            let per_lp = if self.sb_lps == 0 { self.avg_node_iters() as f64 } else { self.sb_iters as f64 / self.sb_lps as f64 };
            (pn.cfg.cost_mult * 2.0 * per_lp / self.avg_node_iters().max(1) as f64, 0.0)
        } else {
            (2.0, 1.0)
        };
        let st = if pn.cfg.global { &self.plsb_tree } else { &pn.stats };
        let v = decide(st, &pn.cfg, gap, pn.best, cost, margin);
        self.plsb_stats[match v {
            Verdict::Undecided(r) => 5 + r as usize,
            Verdict::Continue => 1,
            Verdict::Stop => 2,
        }] += 1;
        match v {
            Verdict::Undecided(_) => {
                self.plsb_stats[0] += 1;
                old
            }
            Verdict::Continue => {
                if old {
                    // 従来なら止めていたところで続ける (`x`)
                    self.plsb_stats[3] += 1;
                }
                false
            }
            Verdict::Stop => {
                if !old {
                    // 従来より早く止める
                    self.plsb_stats[4] += 1;
                }
                pn.stopped = true;
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(dist: Dist) -> PlsbCfg {
        PlsbCfg { global: false, dist, replace: false, iter_cost: false, uncapped: false, min_sample: 3, quot: 0.6, cost_mult: 1.0 }
    }

    #[test]
    fn pareto_fit_and_decide() {
        let mut st = GainStats::new();
        for g in [1.0, 2.0, 4.0, 1.5, 3.0] {
            st.add(g, g);
        }
        st.add(0.0, 0.0);
        assert_eq!(st.n, 5);
        assert_eq!(st.nzero, 1);
        let f = Fitted::fit(&st, Dist::Pareto).unwrap();
        // λ = n / Σ log(g/g_min)
        let s: f64 = [1.0f64, 2.0, 4.0, 1.5, 3.0].iter().map(|g| g.ln()).sum();
        assert!((f.rate - 5.0 / s).abs() < 1e-9);
        assert!((f.cdf(1.0) - 1.0 / 6.0).abs() < 1e-9);
        // 最良 4 でギャップ 40: 深さ 10。より良い候補の確率は小さくない (パレートは裾が重い) ので続ける
        assert_eq!(decide(&st, &cfg(Dist::Pareto), 40.0, 4.0, 2.0, 1.0), Verdict::Continue);
        // ギャップ 4: 深さ 1 なら減らせないので止める
        assert_eq!(decide(&st, &cfg(Dist::Pareto), 4.0, 4.0, 2.0, 1.0), Verdict::Stop);
        // 標本が足りない
        let mut c = cfg(Dist::Pareto);
        c.min_sample = 10;
        assert_eq!(decide(&st, &c, 40.0, 4.0, 2.0, 1.0), Verdict::Undecided(0));
    }

    #[test]
    fn expected_matches_two_level() {
        // d = 2: E = p T(1) + (1 − p) T(2)、p = 1 − F(G)
        let mut st = GainStats::new();
        for g in [1.0, 2.0, 3.0] {
            st.add(g, g);
        }
        let f = Fitted::fit(&st, Dist::Exponential).unwrap();
        let gap = 5.0;
        let p = 1.0 - f.cdf(gap);
        let e = expected_next(&f, gap, 2.0);
        assert!((e - (p * 3.0 + (1.0 - p) * 7.0)).abs() < 1e-9);
    }

    #[test]
    fn erfc_values() {
        assert!((erfc(0.0) - 1.0).abs() < 1e-6);
        assert!((erfc(1.0) - 0.157_299_207).abs() < 1e-6);
        assert!((erfc(-1.0) - 1.842_700_793).abs() < 1e-6);
    }

    #[test]
    fn lognormal_fit() {
        let mut st = GainStats::new();
        for g in [1.0f64, std::f64::consts::E, std::f64::consts::E * std::f64::consts::E] {
            st.add(g, g);
        }
        let f = Fitted::fit(&st, Dist::LogNormal).unwrap();
        assert!((f.mu - 1.0).abs() < 1e-6);
        assert!((f.sigma - 1.0).abs() < 1e-6);
        assert!((f.cdf(std::f64::consts::E) - 0.5).abs() < 1e-6);
    }
}
