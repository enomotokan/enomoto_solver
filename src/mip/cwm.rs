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
//! - 学ぶ量はベクトル θ = (θ_1, …) で、次元ごとに事前の幅 (`CWM_PRIOR` は衝突の重みの次元の幅) を持つ ([`DIMS`])。対では全部の次元を同時に
//!   ±δ_k ずらし (向きは次元ごとに決定的な乱数、SPSA と同じ同時摂動)、観測 y から次元 k の傾き y / (2 δ_k s_k) (s_k = ±1) を得る。
//!   中心は次元ごとに 1 次元のガウス過程 (bayes) か SPSA で決める。今は衝突の重みの 1 次元だけ (分枝のスコアのほかの係数を後から足す)。
//! - 決め方は観測の数 (仕事量) だけで、時刻は使わない。部分 MIP では使わない。状態はプロセスで 1 つ (主の求解の根で初めに戻す)。
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mode {
    Fix,
    /// `=ab`: 係数 1 つを兄弟に 2 値 a・b で入れ切りして比べるだけ (`ENOMOTO_MIP_CWM_AB=係数:a:b`、学ばない)
    Ab,
    Bayes,
    Spsa,
}

/// `ENOMOTO_MIP_CWM` (`fix` / `bayes` / `spsa`。`1` は `bayes`)。
pub(crate) fn mode() -> Option<Mode> {
    static F: std::sync::OnceLock<Option<Mode>> = std::sync::OnceLock::new();
    *F.get_or_init(|| {
        env_str!("ENOMOTO_MIP_CWM").filter(|v| *v != "0").map(|v| match v {
            "fix" => Mode::Fix,
            "ab" => Mode::Ab,
            "spsa" => Mode::Spsa,
            _ => Mode::Bayes,
        })
    })
}

/// 学ぶ次元: (名前, 事前の幅の既定, ずらし幅 δ の既定)。重み = 0.1 × σ((r̄ − (r_mid + Δ)) / (s e^κ)) × e^θ。
/// - 0 `cw` = θ (外の倍率の対数): 事前の幅 1 (倍率 e^±1 ≈ 0.37〜2.7 倍が 1σ)、δ 0.35 (±1.4 倍)。
/// - 1 `rmid` = Δ (シグモイドの真ん中のずれ、r̄ と同じ「衝突 / ノード」の単位): 事前の幅 0.15 (数え上げ (5.1 節) で害の側の最大 0.62 と得の側の
///   最小 0.76 の間の幅 0.14 ほど)、δ 0.05 (s = 0.1 の半分)。
/// - 2 `lns` = κ (シグモイドの幅 s の対数のずれ): 事前の幅 0.7 (幅 ½〜2 倍が 1σ)、δ 0.3。
/// どの次元を学ぶかは `ENOMOTO_MIP_CWM_DIMS` (`t` = θ、`d` = Δ、`k` = κ を並べる。既定 `t`)。学ばない次元は 0 のまま (ずらさない)。
pub(crate) const DIMS: &[(&str, f64, f64)] = &[("cw", 1.0, 0.35), ("rmid", 0.15, 0.05), ("lns", 0.7, 0.3)];

/// 次元 `k` を学ぶか。
pub(crate) fn active(k: usize) -> bool {
    static F: std::sync::OnceLock<[bool; 3]> = std::sync::OnceLock::new();
    F.get_or_init(|| {
        let v = env_str!("ENOMOTO_MIP_CWM_DIMS").unwrap_or("t");
        [v.contains('t'), v.contains('d'), v.contains('k')]
    })[k]
}

/// `=ab` の係数の名前 (番号はこの並び): 推論の項の重み・打ち切りの項の重み・dynw の倍率・擬似費用の正規化の下限・信頼性分枝の閾値
pub(crate) const AB_COEFS: &[&str] = &["inf", "cut", "dynw", "psfloor", "minrel"];

/// `ENOMOTO_MIP_CWM_AB=係数:a:b` を (係数の番号, a, b) に。
pub(crate) fn ab() -> Option<(usize, f64, f64)> {
    static F: std::sync::OnceLock<Option<(usize, f64, f64)>> = std::sync::OnceLock::new();
    *F.get_or_init(|| {
        let v = env_str!("ENOMOTO_MIP_CWM_AB")?;
        let mut it = v.split(':');
        let k = AB_COEFS.iter().position(|c| Some(*c) == it.next())?;
        let a: f64 = it.next()?.parse().ok()?;
        let b: f64 = it.next()?.parse().ok()?;
        Some((k, a, b))
    })
}

fn delta(k: usize) -> f64 {
    if mode() == Some(Mode::Ab) {
        // 値そのものを θ_0 に入れる: 中心 (a + b)/2 ± (b − a)/2
        return if k == 0 { ab().map_or(0.0, |(_, a, b)| (b - a) / 2.0) } else { 0.0 };
    }
    if !active(k) {
        return 0.0;
    }
    if k == 0 { tunable!("ENOMOTO_T_MIP_CWM_DELTA", DIMS[0].2, f64) } else { DIMS[k].2 }
}

/// 中心を探す範囲: 事前の幅の 2 倍 (θ は `CWM_RANGE`、既定 2)。
fn range(k: usize) -> f64 {
    if k == 0 { tunable!("ENOMOTO_T_MIP_CWM_RANGE", 2.0, f64) } else { 2.0 * DIMS[k].1 }
}

fn prior(k: usize) -> f64 {
    if k == 0 { tunable!("ENOMOTO_T_MIP_CWM_PRIOR", DIMS[0].1, f64) } else { DIMS[k].1 }
}

struct Tuner {
    mode: Mode,
    center: Vec<f64>,
    /// 次元ごとの (対を始めたときの中心, 傾きの観測 y / (2 δ s) に 2δ を掛けた値 = 向きをそろえた y)
    obs: Vec<Vec<(f64, f64)>>,
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
        let k = DIMS.len();
        let mut center = vec![0.0; k];
        if m == Mode::Ab {
            center[0] = ab().map_or(0.0, |(_, a, b)| (a + b) / 2.0);
        }
        *TUNER.lock().unwrap() = Some(Tuner { mode: m, center, obs: vec![Vec::new(); k], since: 0, started: 0, trace: String::new(), verbose });
    }
}

fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// 次元 `dim` の観測 (中心, 向きをそろえた y) にガウス過程で傾きを当て、F(θ) + θ²/(2 PRIOR²) が最小の θ。
fn bayes_center(obs: &[(f64, f64)], dim: usize) -> f64 {
    let d = delta(dim);
    let prior = prior(dim);
    // カーネルは次元の事前の幅で尺度をそろえる (θ の幅 1 のとき 分散 0.25・長さ 0.5)
    let gvar = tunable!("ENOMOTO_T_MIP_CWM_GVAR", 0.25, f64) / (prior * prior);
    let len = tunable!("ENOMOTO_T_MIP_CWM_LEN", 0.5, f64) * prior;
    let range = range(dim);
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
    let h = range / 100.0;
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
    /// 対の観測: 対を始めたときの中心 `c`、次元ごとの向き `sg` (+1 / −1、側 0 の向き)、y = ln(側 0 の大きさ / 側 1 の大きさ)
    fn push(&mut self, c: &[f64], sg: &[f64], y: f64) {
        let y = y.clamp(-3.0, 3.0);
        for k in (0..self.obs.len()).filter(|&k| active(k) || (self.mode == Mode::Ab && k == 0)) {
            // 側 0 が +δ_k なら y はそのまま θ_k を上げた側 − 下げた側
            self.obs[k].push((c[k], y * sg[k]));
        }
        self.since += 1;
        let batch = tunable!("ENOMOTO_T_MIP_CWM_BATCH", 32usize, usize).max(1);
        if self.since < batch {
            return;
        }
        let since = self.since;
        self.since = 0;
        let old = self.center.clone();
        for k in (0..self.center.len()).filter(|&k| active(k)) {
            let o = &self.obs[k];
            self.center[k] = match self.mode {
                Mode::Fix => 0.0,
                Mode::Ab => self.center[k],
                Mode::Spsa => {
                    let recent = &o[o.len() - since..];
                    let ybar = recent.iter().map(|x| x.1).sum::<f64>() / recent.len() as f64;
                    let a = tunable!("ENOMOTO_T_MIP_CWM_SPSA_A", 0.1, f64);
                    let r = range(k);
                    (self.center[k] - a * ybar / (2.0 * delta(k))).clamp(-r, r)
                }
                Mode::Bayes => bayes_center(o, k),
            };
        }
        let n = self.obs[0].len();
        if self.trace.len() < 3000 {
            let c: Vec<String> = self.center.iter().map(|v| format!("{v:+.2}")).collect();
            self.trace.push_str(&format!(" {}:{}", n, c.join("/")));
        }
        if self.verbose && old.iter().zip(&self.center).any(|(a, b)| (a - b).abs() > 1e-9) {
            eprintln!("MIP: CWM: theta center {:?} -> {:?} after {} observations", old, self.center, n);
        }
    }
    fn summary(&self) -> String {
        let o = &self.obs[(0..DIMS.len()).find(|&k| active(k)).unwrap_or(0)];
        let n = o.len();
        let ybar = if n > 0 { o.iter().map(|x| x.1).sum::<f64>() / n as f64 } else { 0.0 };
        let se = if n > 1 { (o.iter().map(|x| (x.1 - ybar).powi(2)).sum::<f64>() / ((n - 1) as f64 * n as f64)).sqrt() } else { f64::NAN };
        let c: Vec<String> = DIMS.iter().zip(&self.center).map(|(d, v)| format!("{}={v:+.3}", d.0)).collect();
        format!("mode {:?}, started {}, observations {}, mean y = ln(size+/size-) {:+.3} (se {:.3}), theta center {}; trace{}", self.mode, self.started, n, ybar, se, c.join(" "), self.trace)
    }
}

/// 対 (両側の部分木の大きさ)。
struct Pair {
    center: Vec<f64>,
    /// 次元ごとの側 0 の向き (+1 / −1)
    sg: Vec<f64>,
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
            if let Some(t) = TUNER.lock().unwrap().as_mut() {
                t.push(&p.center, &p.sg, (a0 / a1).ln());
            }
        }
    }
}

/// ノードの札: このノードの θ と、入れ子の対の側の鎖 (外側から)、一番内側の対を始めた深さ。
pub(crate) struct CwmTag {
    theta: Vec<f64>,
    chain: Vec<Arc<Side>>,
    start_depth: usize,
}

/// ノードを処理するとき: 鎖の全部の側の大きさを数え、このノードの θ (次元ごと) を返す。札がなければ中心 θ₀。
pub(crate) fn on_node(tag: Option<&Arc<CwmTag>>) -> Vec<f64> {
    match tag {
        Some(t) => {
            for sd in &t.chain {
                sd.pair.sizes[sd.side].fetch_add(1, Relaxed);
            }
            t.theta.clone()
        }
        None => TUNER.lock().unwrap().as_ref().map_or_else(|| vec![0.0; DIMS.len()], |t| t.center.clone()),
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
        t.center.clone()
    };
    let sg: Vec<f64> = (0..DIMS.len()).map(|k| if splitmix(seed ^ 0x5851_F42D_4C95_7F2D ^ (k as u64).wrapping_mul(0xA24B_AED4_963E_E407)) & 1 == 0 { 1.0 } else { -1.0 }).collect();
    let pair = Arc::new(Pair { center: center.clone(), sg: sg.clone(), sizes: [AtomicU64::new(0), AtomicU64::new(0)], done: [AtomicBool::new(false), AtomicBool::new(false)], recorded: AtomicBool::new(false) });
    let mk = |side: usize| {
        let mut chain: Vec<Arc<Side>> = tag.map(|t| t.chain.clone()).unwrap_or_default();
        chain.push(Arc::new(Side { pair: pair.clone(), side }));
        let sgn = if side == 0 { 1.0 } else { -1.0 };
        let theta = (0..DIMS.len()).map(|k| center[k] + sgn * sg[k] * delta(k)).collect();
        Some(Arc::new(CwmTag { theta, chain, start_depth: depth + 1 }))
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
        assert_eq!(bayes_center(&[], 0), 0.0);
        let obs: Vec<(f64, f64)> = (0..64).map(|i| (0.0, 0.3 + 0.05 * ((i % 5) as f64 - 2.0))).collect();
        assert!(bayes_center(&obs, 0) < -0.05);
        let obs: Vec<(f64, f64)> = (0..64).map(|i| (0.0, -0.3 + 0.05 * ((i % 5) as f64 - 2.0))).collect();
        assert!(bayes_center(&obs, 0) > 0.05);
        // 平均 0 の雑音だけなら 0 の近く
        let obs: Vec<(f64, f64)> = (0..64).map(|i| (0.0, if i % 2 == 0 { 1.0 } else { -1.0 })).collect();
        assert!(bayes_center(&obs, 0).abs() < 0.1);
    }
}
