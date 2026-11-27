//! pseudocost (分枝による目的値の増加の単位あたりの平均) と分枝スコア
//! (HiGHS の `HighsPseudocost` を簡略化したもの)。

/// `ENOMOTO_MIP_GAP1` の値 (0.1.7 から既定 `u`、束 cand1114。`0` で空)。analysis/gap1_tree_20261109.md
pub fn gap1() -> &'static str {
    static V: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    V.get_or_init(|| std::env::var("ENOMOTO_MIP_GAP1").map(|v| if v == "0" || v == "off" { String::new() } else { v }).unwrap_or_else(|_| "u".to_string()))
}

/// 列ごとの pseudocost と観測回数。
/// 診断用 (試作 `ENOMOTO_T_MIP_CONFLICT_WEIGHT_FLAT`): [hybrid のスコアの数, そのうち手がかりなしの判定が真だった数] (全スレッド・サブ MIP 込み)。
static FLAT_CALLS: [std::sync::atomic::AtomicU64; 2] = [const { std::sync::atomic::AtomicU64::new(0) }; 2];

#[derive(Clone)]
pub struct Pseudocost {
    sum_up: Vec<f64>,
    n_up: Vec<u32>,
    sum_down: Vec<f64>,
    n_down: Vec<u32>,
    /// 試作 `ENOMOTO_MIP_PCL` の `c` (analysis/disj_20261119.md の 11 節): 単位あたりの値の 2 乗の和 (列・向きごと、ばらつきの信頼区間に使う)
    sq_up: Vec<f64>,
    sq_down: Vec<f64>,
    /// 試作 `ENOMOTO_MIP_PCL` の `b`: 似た列の組 (列 → 組の番号、組 → 列)。観測のない列の費用を同じ組の観測のある列の平均で埋める
    pub groups: Option<std::sync::Arc<(Vec<u32>, Vec<Vec<u32>>)>>,
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
    /// 試作 `ENOMOTO_T_MIP_CONFLICT_WEIGHT_FLAT` の判定用: 目的の利得が正 (1e-9 より大きい) だった観測の数 (上下合計)。
    n_pos_gain: u64,
    /// 試作 `ENOMOTO_MIP_CONFLICT_WEIGHT_DYN` (p2inf_20261119): 解いている途中で上げた衝突の重み (0 なら上げていない)。木の様子を見て
    /// [`super::solver`] が設定する
    pub cw_dyn: f64,
    /// 試作 `ENOMOTO_MIP_CWM`: 今のノードの衝突の重みの倍率 (`cw_dyn` に掛ける。既定 1)
    pub cw_mult: f64,
    /// 試作 `ENOMOTO_MIP_SB_ADAPT` (analysis/sbadapt_20261124): 衝突の重みの床 (0.01 × f。[`super::solver`] が設定する、既定 0)
    pub cw_floor: f64,
    /// 試作 `ENOMOTO_MIP_CWM=ab` (p2inf_20261119 7 節): 今のノードで上書きする分枝のスコアの係数 (番号は [`super::cwm::AB_COEFS`], 値)
    pub ab: Option<(usize, f64)>,
    /// 実行不能で終わった葉と、目的値で打ち切った葉の数 (スコアの動的な重み)。
    pub infeasible_leaves: u64,
    pub objlim_leaves: u64,
    /// 衝突スコア (VSIDS と同じ): 衝突に現れた列・向きに `conflict_inc` を足し、`conflict_inc` を衝突ごとに 2% 増やす
    /// (古い衝突の重みが相対的に下がる)。`[0]` は下向き (`x <= v` のリテラル)、`[1]` は上向き (`x >= v`)。
    conflict: [Vec<f64>; 2],
    conflict_inc: f64,
    conflict_total: f64,
    /// 試作 `ENOMOTO_MIP_S4_DEGSCORE` (既定では使わない): 今のノードの LP の退化の度合い (HiGHS の `computeLPDegneracy` に倣う。1 = 退化なし)。
    /// [`Self::hybrid`] の擬似費用の項をこれで割る (退化したノードでは推論・打ち切りの点数で選ぶ)
    pub degen_fac: f64,
}

impl Pseudocost {
    pub fn new(n: usize) -> Self {
        Pseudocost {
            sq_up: vec![0.0; n],
            sq_down: vec![0.0; n],
            groups: None,
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
            n_pos_gain: 0,
            cw_dyn: 0.0,
            cw_floor: 0.0,
            cw_mult: 1.0,
            ab: None,
            infeasible_leaves: 0,
            objlim_leaves: 0,
            conflict: [vec![0.0; n], vec![0.0; n]],
            conflict_inc: 1.0,
            conflict_total: 0.0,
            degen_fac: 1.0,
        }
    }

    /// 列の数。
    pub fn len(&self) -> usize {
        self.n_up.len()
    }

    /// 観測の量 (診断用): (上向きの観測のある列, 下向きの観測のある列, 両向きのある列, 上向きの観測の総数, 下向きの観測の総数)。
    pub fn observation_counts(&self) -> (usize, usize, usize, u64, u64) {
        let up = self.n_up.iter().filter(|&&k| k > 0).count();
        let down = self.n_down.iter().filter(|&&k| k > 0).count();
        let both = self.n_up.iter().zip(&self.n_down).filter(|&(&u, &d)| u > 0 && d > 0).count();
        let su: u64 = self.n_up.iter().map(|&k| k as u64).sum();
        let sd: u64 = self.n_down.iter().map(|&k| k as u64).sum();
        (up, down, both, su, sd)
    }

    /// `ENOMOTO_MIP_RESTART_PC` (試作): 再スタートの前処理の後の問題へ写す。前処理は列の番号を保ち、尺度 `d` を掛ける (x = d x')
    /// ので、残った列 (`keep`) は単位あたりの値に d を掛けて観測の数ごと写し、消えた列は観測を捨てる。全体の平均は残った列から数え直す
    pub fn remap_for_restart(&mut self, keep: &[bool], d: &[f64]) {
        for j in 0..self.len().min(keep.len()) {
            if keep[j] {
                let s = d.get(j).copied().unwrap_or(1.0);
                self.sum_up[j] *= s;
                self.sum_down[j] *= s;
                self.sq_up[j] *= s * s;
                self.sq_down[j] *= s * s;
            } else {
                self.sum_up[j] = 0.0;
                self.n_up[j] = 0;
                self.sq_up[j] = 0.0;
                self.sq_down[j] = 0.0;
                self.sum_down[j] = 0.0;
                self.n_down[j] = 0;
                self.cutoff_up[j] = 0;
                self.cutoff_down[j] = 0;
                for k in 0..2 {
                    self.inf_sum[k][j] = 0.0;
                    self.inf_n[k][j] = 0;
                    self.conflict[k][j] = 0.0;
                }
            }
        }
        self.total_up = self.sum_up.iter().sum();
        self.total_n_up = self.n_up.iter().map(|&k| k as u64).sum();
        self.total_down = self.sum_down.iter().sum();
        self.total_n_down = self.n_down.iter().map(|&k| k as u64).sum();
        self.inf_total = self.inf_sum.iter().flatten().sum();
        self.inf_total_n = self.inf_n.iter().flatten().map(|&k| k as u64).sum();
        self.conflict_total = self.conflict.iter().flatten().sum();
    }

    /// スレッドの間での共有: 前回の共有の時点の自分 `base` から増えた観測を共有の `shared` に足し、自分を `shared` の
    /// 観測に揃える (`base` も揃えた後の値にする)。衝突スコア (古いものほど軽くする重み) はスレッドごとのまま。
    pub fn merge_shared(&mut self, base: &mut Pseudocost, shared: &mut Pseudocost) {
        fn mf(a: &mut [f64], b: &mut [f64], s: &mut [f64]) {
            for k in 0..a.len() {
                s[k] += a[k] - b[k];
                a[k] = s[k];
                b[k] = s[k];
            }
        }
        fn mu(a: &mut [u32], b: &mut [u32], s: &mut [u32]) {
            for k in 0..a.len() {
                s[k] = s[k].saturating_add(a[k].saturating_sub(b[k]));
                a[k] = s[k];
                b[k] = s[k];
            }
        }
        fn sf(a: &mut f64, b: &mut f64, s: &mut f64) {
            *s += *a - *b;
            *a = *s;
            *b = *s;
        }
        fn su(a: &mut u64, b: &mut u64, s: &mut u64) {
            *s = s.saturating_add(a.saturating_sub(*b));
            *a = *s;
            *b = *s;
        }
        mf(&mut self.sum_up, &mut base.sum_up, &mut shared.sum_up);
        mf(&mut self.sq_up, &mut base.sq_up, &mut shared.sq_up);
        mf(&mut self.sq_down, &mut base.sq_down, &mut shared.sq_down);
        mu(&mut self.n_up, &mut base.n_up, &mut shared.n_up);
        mf(&mut self.sum_down, &mut base.sum_down, &mut shared.sum_down);
        mu(&mut self.n_down, &mut base.n_down, &mut shared.n_down);
        sf(&mut self.total_up, &mut base.total_up, &mut shared.total_up);
        su(&mut self.total_n_up, &mut base.total_n_up, &mut shared.total_n_up);
        sf(&mut self.total_down, &mut base.total_down, &mut shared.total_down);
        su(&mut self.total_n_down, &mut base.total_n_down, &mut shared.total_n_down);
        mu(&mut self.cutoff_up, &mut base.cutoff_up, &mut shared.cutoff_up);
        mu(&mut self.cutoff_down, &mut base.cutoff_down, &mut shared.cutoff_down);
        for d in 0..2 {
            mf(&mut self.inf_sum[d], &mut base.inf_sum[d], &mut shared.inf_sum[d]);
            mu(&mut self.inf_n[d], &mut base.inf_n[d], &mut shared.inf_n[d]);
        }
        sf(&mut self.inf_total, &mut base.inf_total, &mut shared.inf_total);
        su(&mut self.inf_total_n, &mut base.inf_total_n, &mut shared.inf_total_n);
        su(&mut self.infeasible_leaves, &mut base.infeasible_leaves, &mut shared.infeasible_leaves);
        su(&mut self.objlim_leaves, &mut base.objlim_leaves, &mut shared.objlim_leaves);
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
    /// HiGHS の `increaseConflictWeight` + `increaseConflictScoreUp/Down`: 1 回の衝突解析で説明に使った境界の変更
    /// (列, 上限か) の列の衝突スコアを今の重みだけ上げ、重みを 1.02 倍にする (下限の変更は上への分枝の側)。
    pub fn bump_conflict(&mut self, chg: &[(usize, bool)]) {
        self.conflict_inc *= 1.02;
        for &(j, upper) in chg {
            let d = (!upper) as usize;
            self.conflict[d][j] += self.conflict_inc;
            self.conflict_total += self.conflict_inc;
        }
        if self.conflict_inc > 1e100 {
            let f = 1e-100;
            for v in self.conflict.iter_mut().flat_map(|c| c.iter_mut()) {
                *v *= f;
            }
            self.conflict_inc *= f;
            self.conflict_total *= f;
        }
    }

    pub fn add_observation(&mut self, j: usize, up: bool, delta: f64, gain: f64) {
        if delta <= 1e-9 || !gain.is_finite() {
            return;
        }
        let unit = gain.max(0.0) / delta;
        if gain > 1e-9 {
            self.n_pos_gain += 1;
        }
        if up {
            self.sq_up[j] += unit * unit;
            self.sum_up[j] += unit;
            self.n_up[j] += 1;
            self.total_up += unit;
            self.total_n_up += 1;
        } else {
            self.sq_down[j] += unit * unit;
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
            return self.group_mean(j, true).unwrap_or(avg);
        }
        let own = self.sum_up[j] / n as f64;
        let w = (0.9 + 0.1 * n as f64 / self.min_reliable as f64).min(1.0);
        w * own + (1.0 - w) * avg
    }

    pub fn cost_down(&self, j: usize) -> f64 {
        let n = self.n_down[j];
        let avg = self.avg_down();
        if n == 0 {
            return self.group_mean(j, false).unwrap_or(avg);
        }
        let own = self.sum_down[j] / n as f64;
        let w = (0.9 + 0.1 * n as f64 / self.min_reliable as f64).min(1.0);
        w * own + (1.0 - w) * avg
    }

    /// 列・向きの打ち切りの回数 (診断 `ENOMOTO_MIP_PCLOG` 用)。
    pub fn cutoff_count(&self, j: usize, up: bool) -> u32 {
        if up { self.cutoff_up[j] } else { self.cutoff_down[j] }
    }

    /// 列・向きの観測の数 (診断 `ENOMOTO_MIP_PCLOG` 用)。
    pub fn n_obs(&self, j: usize, up: bool) -> u32 {
        if up { self.n_up[j] } else { self.n_down[j] }
    }

    /// 試作 `ENOMOTO_MIP_PCL` の `b`: 同じ組の観測のある列 (最大 64 本を見る) の単位あたりの値の平均。組がなければ `None`
    fn group_mean(&self, j: usize, up: bool) -> Option<f64> {
        let g = self.groups.as_ref()?;
        let (sum, n) = if up { (&self.sum_up, &self.n_up) } else { (&self.sum_down, &self.n_down) };
        let (mut acc, mut k) = (0.0, 0usize);
        for &m in g.1[g.0[j] as usize].iter().take(64) {
            let m = m as usize;
            if m != j && n[m] > 0 {
                acc += sum[m] / n[m] as f64;
                k += 1;
            }
        }
        (k > 0).then(|| acc / k as f64)
    }

    /// 試作 `ENOMOTO_MIP_PCL` の `c`: 両向きとも観測が 2 回以上あり、単位あたりの値の平均の 95% 信頼区間の半幅 (2 sd / √n) が平均の
    /// `rel` 倍以下なら信頼する (観測の数でなくばらつきで決める)
    pub fn ci_reliable(&self, j: usize, rel: f64) -> bool {
        let ok = |sum: f64, sq: f64, n: u32| {
            if n < 2 {
                return false;
            }
            let nf = n as f64;
            let mean = sum / nf;
            let var = ((sq - sum * sum / nf) / (nf - 1.0)).max(0.0);
            2.0 * (var / nf).sqrt() <= rel * mean.abs().max(1e-12)
        };
        ok(self.sum_up[j], self.sq_up[j], self.n_up[j]) && ok(self.sum_down[j], self.sq_down[j], self.n_down[j])
    }

    /// 両側とも十分観測されているか。
    pub fn is_reliable(&self, j: usize) -> bool {
        self.min_observations(j) >= self.min_reliable
    }

    /// 両側の観測回数の少ない方。
    /// `ENOMOTO_MIP_PC_COUNT_CUTOFF` (試作、既定では使わない): 子が実行不能・打ち切りになった回数も観測に数える (CBC の
    /// CbcSimpleIntegerDynamicPseudoCost.cpp の `INFEAS == 1` は実行不能の子も `numberTimesDown_` に数える)。子がよく切れる列が
    /// 強分岐の対象に残り続けないようにする
    pub fn min_observations(&self, j: usize) -> u32 {
        if env_str!("ENOMOTO_MIP_PC_COUNT_CUTOFF").is_some() {
            return (self.n_up[j] + self.cutoff_up[j]).min(self.n_down[j] + self.cutoff_down[j]);
        }
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

    /// 診断用: 記録した分枝の数と、1 回あたりの推論の数の平均。
    pub fn inference_count(&self) -> u64 {
        self.inf_total_n
    }

    /// 診断用: (FLAT の判定の呼び出し (全スレッド), 真だった数 (全スレッド), 利得が正の観測 (このスレッド), 観測の数)。
    pub fn flat_diag(&self) -> (u64, u64, u64, u64) {
        use std::sync::atomic::Ordering::Relaxed;
        (FLAT_CALLS[0].load(Relaxed), FLAT_CALLS[1].load(Relaxed), self.n_pos_gain, self.total_n_down + self.total_n_up)
    }

    pub fn inference_mean(&self) -> f64 {
        if self.inf_total_n > 0 { self.inf_total / self.inf_total_n as f64 } else { 0.0 }
    }

    pub fn inference(&self, j: usize, up: bool) -> f64 {
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
        let mut wc = tunable!("ENOMOTO_T_MIP_CONFLICT_WEIGHT", 0.0, f64);
        // 試作 `ENOMOTO_T_MIP_CONFLICT_WEIGHT_FLAT` = w (p2inf_20261119、既定 0 で使わない): 擬似費用の平均が上下とも 0 (目的の利得の手がかりが
        // ない: bnatt400 は LP の値が最後まで 0) のときだけ衝突の重みを w にする。一律の 0.1 は bnatt400 を解くが、擬似費用に意味のある
        // mas76・pk1 を 1.3〜1.6 倍遅くした (衝突の項が擬似費用の選び方を乱す)
        wc = wc.max(self.cw_dyn * self.cw_mult).max(self.cw_floor);
        let wflat = tunable!("ENOMOTO_T_MIP_CONFLICT_WEIGHT_FLAT", 0.0, f64);
        // 判定: 観測のうち目的の利得が正だったものが `ENOMOTO_T_MIP_CONFLICT_WEIGHT_FLAT_FRAC` (既定 5%) 以下 (観測がなければ手がかりなしとみなす)。
        // 平均が厳密に 0 という判定では、目的の列の分枝などで少し利得が出るとすぐ外れ、bnatt400 の 4 スレッドが 900 秒で解けなかった
        let nobs = self.total_n_down + self.total_n_up;
        let flat = nobs == 0 || (self.n_pos_gain as f64) <= tunable!("ENOMOTO_T_MIP_CONFLICT_WEIGHT_FLAT_FRAC", 0.05, f64) * nobs as f64;
        if wflat > 0.0 {
            FLAT_CALLS[0].fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            FLAT_CALLS[1].fetch_add(flat as u64, std::sync::atomic::Ordering::Relaxed);
        }
        if wflat > 0.0 && flat {
            wc = wc.max(wflat);
        }
        let conf_term = if self.conflict_total > 0.0 { wc * s(conf, conf_avg * conf_avg) } else { 0.0 };
        // `ENOMOTO_MIP_GAP1` (試作、既定では使わない。analysis/gap1_tree_20261109.md): 文字で部品を選ぶ。
        // `s`: 擬似費用の項の正規化の下限を 0.1 から 1e-6 にする (HiGHS の `getScore` は `max(1e-6, cost_total^2)`)。目的値の刻みが
        // 1e-4 程度の問題 (neos-957323) では、0.1 の下限で擬似費用の項が 1e-9 程度に潰れ、推論の項 (1e-4) だけで列を選んでいた。
        // `w`: SCIP の動的な重み `dynw` を 1 にする (HiGHS にはない)。
        let g1 = gap1();
        let dynw = if g1.contains('w') { 1.0 } else { dynw };
        // 係数 (p2inf_20261119 7 節の入れ切りの確かめ): 推論の項・打ち切りの項の重み (既定 1e-4)、dynw に掛ける倍率 (既定 1)、
        // 擬似費用の項の正規化の下限 (既定 0.1)。`ENOMOTO_T_MIP_HY_*` で全体を、[`Self::ab`] でノードごとに上書きする
        let ov = |c: usize, d: f64| match self.ab {
            Some((k, v)) if k == c => v,
            _ => d,
        };
        let w_inf = ov(0, tunable!("ENOMOTO_T_MIP_HY_INF_W", 1e-4, f64));
        let w_cut = ov(1, tunable!("ENOMOTO_T_MIP_HY_CUT_W", 1e-4, f64));
        let dynw = dynw * ov(2, tunable!("ENOMOTO_T_MIP_HY_DYNW_MULT", 1.0, f64));
        let ps_floor = ov(3, tunable!("ENOMOTO_T_MIP_HY_PS_FLOOR", 0.1, f64));
        let ps_s = if g1.contains('s') { 1.0 - 1.0 / (1.0 + ps / ps_avg.max(1e-6)) } else { 1.0 - 1.0 / (1.0 + ps / ps_avg.max(ps_floor)) };
        dynw * (w_inf * s(inf, inf_avg) + w_cut * s(cut, 0.01) + conf_term) + ps_s / (dynw * self.degen_fac.max(1.0))
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
