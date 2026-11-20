//! 試作 `ENOMOTO_MIP_CWM` (p2inf_20261119 6 節、既定では使わない): 衝突の重み (cwrh0 の 0.1 × σ((r̄ − r_mid)/s)) に e^θ を掛け、
//! シグモイドからのずれ θ を、兄弟の部分木の大きさを対で比べた観測から解きながら合わせる。
//!
//! - 対: 分枝のノードで、2 つの子に θ₀ + δ と θ₀ − δ を割り振る (θ₀ は今の中心、δ = `ENOMOTO_T_MIP_CWM_DELTA` (既定 0.35)、
//!   どちらの子に + かは決定的な乱数)。子の部分木は親の θ を受け継ぎ、対の始めから `ENOMOTO_T_MIP_CWM_SPAN` (既定 5) 段下りたノードで
//!   新しい対を始める (入れ子)。対の外のノードは θ₀。
//! - 観測: 対の両側の部分木を解き終えたら y = ln(大きさ₊) − ln(大きさ₋) (大きさ = 処理したノードの数、裾が重いので対数、[−3, 3] に切る)。
//!   y / (2δ) は「ln(部分木の大きさ)」の θ₀ での傾きの雑音つきの観測。両側を解き終えない対 (打ち切り・時間切れ・片側が終わらない) は
//!   使わない (小さめの部分木に偏るが、向きの推定には足りるとみる)。部分木の終わりは、ノードの持つ札の参照 (Arc) が全部なくなったことで知る。
//! - 中心の決め方 (観測 `ENOMOTO_T_MIP_CWM_BATCH` (既定 32) 個ごと):
//!   - `=bayes` (`=1` も): 傾き g(θ) にガウス過程 (平均 0、RBF、分散 `CWM_GVAR`、長さ `CWM_LEN`) を当て、雑音の分散は観測の傾きの標本分散。
//!     F(θ) = ∫₀^θ ĝ (事後平均の傾きの積分 = θ = 0 に比べた ln(大きさ) の見込みの変化) に、事前 θ ~ N(0, `CWM_PRIOR`²) の罰 θ²/(2 PRIOR²)
//!     を足した値が最小の θ を [−`CWM_RANGE`, RANGE] の格子 (0.02 刻み) から選ぶ。観測が少ないうちは ĝ ≈ 0 なので θ₀ = 0 (= cwrh0) に留まる。
//!   - `=spsa`: θ₀ ← θ₀ − a × (束の y の平均) / (2δ) (a = `CWM_SPSA_A`、既定 0.1)。比べ用。
//!   - `=fix`: 動かさない (θ₀ = 0 で ±δ を比べるだけ。成績の指標が向きを当てるかの確かめ)。
//! - 決め方は観測の数 (仕事量) だけで、時刻は使わない。部分 MIP では使わない。状態はプロセスで 1 つ (主の求解の根で初めに戻す)。
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mode {
    Fix,
    Bayes,
    Spsa,
}

/// `ENOMOTO_MIP_CWM` (`fix` / `bayes` / `spsa`。`1` は `bayes`)。
pub(crate) fn mode() -> Option<Mode> {
    static F: std::sync::OnceLock<Option<Mode>> = std::sync::OnceLock::new();
    *F.get_or_init(|| {
        env_str!("ENOMOTO_MIP_CWM").filter(|v| *v != "0").map(|v| match v {
            "fix" => Mode::Fix,
            "spsa" => Mode::Spsa,
            _ => Mode::Bayes,
        })
    })
}

fn delta() -> f64 {
    tunable!("ENOMOTO_T_MIP_CWM_DELTA", 0.35, f64)
}

struct Tuner {
    mode: Mode,
    center: f64,
    /// (対を始めたときの中心, y)
    obs: Vec<(f64, f64)>,
    since: usize,
    started: u64,
    /// 中心の推移 (観測の数:中心)
    trace: String,
    verbose: bool,
}

static TUNER: Mutex<Option<Tuner>> = Mutex::new(None);

/// 主の求解の根で呼ぶ: 状態を戻す。
pub(crate) fn reset(verbose: bool) {
    if let Some(m) = mode() {
        *TUNER.lock().unwrap() = Some(Tuner { mode: m, center: 0.0, obs: Vec::new(), since: 0, started: 0, trace: String::new(), verbose });
    }
}

fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// ガウス過程で傾きを当て、F(θ) + θ²/(2 PRIOR²) が最小の θ。
fn bayes_center(obs: &[(f64, f64)]) -> f64 {
    let d = delta();
    let gvar = tunable!("ENOMOTO_T_MIP_CWM_GVAR", 0.25, f64);
    let len = tunable!("ENOMOTO_T_MIP_CWM_LEN", 0.5, f64);
    let prior = tunable!("ENOMOTO_T_MIP_CWM_PRIOR", 1.0, f64);
    let range = tunable!("ENOMOTO_T_MIP_CWM_RANGE", 2.0, f64);
    // 直近の観測だけ (古い観測ほど今の木と合わない。数で区切る)
    let keep = tunable!("ENOMOTO_T_MIP_CWM_KEEP", 400usize, usize);
    let obs = &obs[obs.len().saturating_sub(keep)..];
    let n = obs.len();
    if n < 2 {
        return 0.0;
    }
    let gs: Vec<f64> = obs.iter().map(|&(_, y)| y / (2.0 * d)).collect();
    let mean = gs.iter().sum::<f64>() / n as f64;
    let noise = (gs.iter().map(|g| (g - mean) * (g - mean)).sum::<f64>() / (n - 1) as f64).max(1e-6);
    let k = |a: f64, b: f64| gvar * (-(a - b) * (a - b) / (2.0 * len * len)).exp();
    // (K + noise I) α = g をコレスキーで解く
    let mut a = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            a[i * n + j] = k(obs[i].0, obs[j].0) + if i == j { noise } else { 0.0 };
        }
    }
    for j in 0..n {
        let mut s = a[j * n + j];
        for t in 0..j {
            s -= a[j * n + t] * a[j * n + t];
        }
        let dj = s.max(1e-12).sqrt();
        a[j * n + j] = dj;
        for i in j + 1..n {
            let mut s = a[i * n + j];
            for t in 0..j {
                s -= a[i * n + t] * a[j * n + t];
            }
            a[i * n + j] = s / dj;
        }
    }
    let mut z = gs.clone();
    for i in 0..n {
        let mut s = z[i];
        for t in 0..i {
            s -= a[i * n + t] * z[t];
        }
        z[i] = s / a[i * n + i];
    }
    for i in (0..n).rev() {
        let mut s = z[i];
        for t in i + 1..n {
            s -= a[t * n + i] * z[t];
        }
        z[i] = s / a[i * n + i];
    }
    let ghat = |th: f64| (0..n).map(|i| k(th, obs[i].0) * z[i]).sum::<f64>();
    // 格子の上で F を 0 から両側へ台形で積む
    let h = 0.02;
    let steps = (range / h).round() as i64;
    let mut best = (0.0f64, 0.0f64);
    for dir in [-1.0f64, 1.0] {
        let mut f = 0.0;
        let mut prev = ghat(0.0);
        for s in 1..=steps {
            let th = dir * s as f64 * h;
            let g = ghat(th);
            f += 0.5 * (prev + g) * dir * h;
            prev = g;
            let obj = f + th * th / (2.0 * prior * prior);
            if obj < best.1 - 1e-12 {
                best = (th, obj);
            }
        }
    }
    best.0
}

impl Tuner {
    fn push(&mut self, c: f64, y: f64) {
        self.obs.push((c, y.clamp(-3.0, 3.0)));
        self.since += 1;
        let batch = tunable!("ENOMOTO_T_MIP_CWM_BATCH", 32usize, usize).max(1);
        if self.since < batch {
            return;
        }
        let recent: Vec<f64> = self.obs[self.obs.len() - self.since..].iter().map(|o| o.1).collect();
        self.since = 0;
        let old = self.center;
        self.center = match self.mode {
            Mode::Fix => 0.0,
            Mode::Spsa => {
                let ybar = recent.iter().sum::<f64>() / recent.len() as f64;
                let a = tunable!("ENOMOTO_T_MIP_CWM_SPSA_A", 0.1, f64);
                let r = tunable!("ENOMOTO_T_MIP_CWM_RANGE", 2.0, f64);
                (self.center - a * ybar / (2.0 * delta())).clamp(-r, r)
            }
            Mode::Bayes => bayes_center(&self.obs),
        };
        if self.trace.len() < 3000 {
            self.trace.push_str(&format!(" {}:{:+.2}", self.obs.len(), self.center));
        }
        if self.verbose && (self.center - old).abs() > 1e-9 {
            eprintln!("MIP: CWM: theta center {:+.3} -> {:+.3} after {} observations", old, self.center, self.obs.len());
        }
    }
    fn summary(&self) -> String {
        let n = self.obs.len();
        let ybar = if n > 0 { self.obs.iter().map(|o| o.1).sum::<f64>() / n as f64 } else { 0.0 };
        let se = if n > 1 { (self.obs.iter().map(|o| (o.1 - ybar).powi(2)).sum::<f64>() / ((n - 1) as f64 * n as f64)).sqrt() } else { f64::NAN };
        format!("mode {:?}, started {}, observations {}, mean y = ln(size+/size-) {:+.3} (se {:.3}), theta center {:+.3}; trace{}", self.mode, self.started, n, ybar, se, self.center, self.trace)
    }
}

/// 対 (両側の部分木の大きさ)。
struct Pair {
    center: f64,
    /// 側 0 が + か
    plus0: bool,
    sizes: [AtomicU64; 2],
    done: [AtomicBool; 2],
    recorded: AtomicBool,
}

/// 対の片側 (ノードの札から参照される)。最後の参照がなくなると、その側の部分木は解き終わり。
pub(crate) struct Side {
    pair: Arc<Pair>,
    side: usize,
}

impl Drop for Side {
    fn drop(&mut self) {
        let p = &self.pair;
        p.done[self.side].store(true, Relaxed);
        if p.done[1 - self.side].load(Relaxed) && !p.recorded.swap(true, Relaxed) {
            let (a0, a1) = (p.sizes[0].load(Relaxed).max(1) as f64, p.sizes[1].load(Relaxed).max(1) as f64);
            let y = if p.plus0 { (a0 / a1).ln() } else { (a1 / a0).ln() };
            if let Some(t) = TUNER.lock().unwrap().as_mut() {
                t.push(p.center, y);
            }
        }
    }
}

/// ノードの札: このノードの θ と、入れ子の対の側の鎖 (外側から)、一番内側の対を始めた深さ。
pub(crate) struct CwmTag {
    theta: f64,
    chain: Vec<Arc<Side>>,
    start_depth: usize,
}

/// ノードを処理するとき: 鎖の全部の側の大きさを数え、このノードの倍率 e^θ を返す。札がなければ e^θ₀。
pub(crate) fn on_node(tag: Option<&Arc<CwmTag>>) -> f64 {
    match tag {
        Some(t) => {
            for sd in &t.chain {
                sd.pair.sizes[sd.side].fetch_add(1, Relaxed);
            }
            t.theta.exp()
        }
        None => TUNER.lock().unwrap().as_ref().map_or(1.0, |t| t.center.exp()),
    }
}

/// 分枝のノード (深さ `depth`、札 `tag`) の 2 つの子の札。対を始めるなら θ₀ ± δ を割り振る。`seed` は決定的な乱数の元。
pub(crate) fn children(tag: Option<&Arc<CwmTag>>, depth: usize, seed: u64) -> [Option<Arc<CwmTag>>; 2] {
    let span = tunable!("ENOMOTO_T_MIP_CWM_SPAN", 5usize, usize).max(1);
    if let Some(t) = tag {
        if depth + 1 < t.start_depth + span {
            return [Some(t.clone()), Some(t.clone())];
        }
    }
    let center = {
        let mut g = TUNER.lock().unwrap();
        let Some(t) = g.as_mut() else { return [tag.cloned(), tag.cloned()] };
        t.started += 1;
        t.center
    };
    let plus0 = splitmix(seed ^ 0x5851_F42D_4C95_7F2D) & 1 == 0;
    let pair = Arc::new(Pair { center, plus0, sizes: [AtomicU64::new(0), AtomicU64::new(0)], done: [AtomicBool::new(false), AtomicBool::new(false)], recorded: AtomicBool::new(false) });
    let d = delta();
    let mk = |side: usize| {
        let mut chain: Vec<Arc<Side>> = tag.map(|t| t.chain.clone()).unwrap_or_default();
        chain.push(Arc::new(Side { pair: pair.clone(), side }));
        let plus = (side == 0) == plus0;
        Some(Arc::new(CwmTag { theta: center + if plus { d } else { -d }, chain, start_depth: depth + 1 }))
    };
    [mk(0), mk(1)]
}

/// 終わりの記録。
pub(crate) fn final_line() -> Option<String> {
    TUNER.lock().unwrap().as_ref().map(|t| t.summary())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bayes_center_follows_gradient_sign() {
        // 傾きが正 (θ を上げると部分木が大きい) なら中心は負へ、観測がなければ 0
        assert_eq!(bayes_center(&[]), 0.0);
        let obs: Vec<(f64, f64)> = (0..64).map(|i| (0.0, 0.3 + 0.05 * ((i % 5) as f64 - 2.0))).collect();
        assert!(bayes_center(&obs) < -0.05);
        let obs: Vec<(f64, f64)> = (0..64).map(|i| (0.0, -0.3 + 0.05 * ((i % 5) as f64 - 2.0))).collect();
        assert!(bayes_center(&obs) > 0.05);
        // 平均 0 の雑音だけなら 0 の近く
        let obs: Vec<(f64, f64)> = (0..64).map(|i| (0.0, if i % 2 == 0 { 1.0 } else { -1.0 })).collect();
        assert!(bayes_center(&obs).abs() < 0.1);
    }
}
