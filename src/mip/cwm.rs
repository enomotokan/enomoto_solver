//! 試作 `ENOMOTO_MIP_CWM` (p2inf_20261119 6 節、既定では使わない): 衝突の重み (cwrh0 の 0.1 × σ) に掛ける倍率 m を部分木ごとに割り振り、
//! 兄弟の部分木の大きさを対で比べて、成績の良い m に寄せる (対で比べる多腕バンディット)。
//!
//! - 対: 分枝のノードで、2 つの子に違う腕 (m の候補) を割り振る (どちらの子にどちらの腕かは乱数)。子の部分木は親の腕を受け継ぐ。
//!   部分木の中でも、対の始めから `ENOMOTO_T_MIP_CWM_SPAN` (既定 5) 段下りたノードで新しい対を始める (入れ子)。
//! - 成績: 対の両側の部分木で処理したノードの数 (小さい方が勝ち)。片側が解き終わり (その側のノードが全部なくなる)、もう片側が既にそれより
//!   多くのノードを処理していれば、その時点で勝ち負けが決まる。両側とも解き終えて同じ数なら引き分け (数えない)。
//!   部分木の終わりは、ノードの持つ札 ([`CwmTag`]) の参照 (Arc) が全部なくなったことで知る (処理した・刈った・捨てたのどれでも)。
//! - 選び方: 腕ごとの勝ちの割合の事後分布 (Beta(勝ち + 1, 負け + 1) の正規近似) から引いた値の上位 2 本 (決定的な乱数: スレッドの種と
//!   ノード番号から、時刻は使わない)。`ENOMOTO_MIP_CWM=fix` は学習なしで、腕 0 と `ENOMOTO_T_MIP_CWM_FIX_B` (既定: m = 1 の腕) を
//!   いつも比べる (成績の指標が向きを当てられるかの確かめ)。
//! - 対の外のノード (根の近く) は、今の事後平均が一番の腕。
//! - 部分 MIP では使わない。状態はプロセスで 1 つ (主の求解の根で初めに戻す)。
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

/// 腕 (m の候補)。`ENOMOTO_MIP_CWM_ARMS` (「,」区切り) で変えられる。
pub(crate) fn arms() -> &'static [f64] {
    static A: std::sync::OnceLock<Vec<f64>> = std::sync::OnceLock::new();
    A.get_or_init(|| {
        env_str!("ENOMOTO_MIP_CWM_ARMS")
            .map(|s| s.split(',').filter_map(|t| t.trim().parse().ok()).collect::<Vec<f64>>())
            .filter(|v| v.len() >= 2)
            .unwrap_or_else(|| vec![0.0, 0.5, 1.0, 2.0])
    })
}

/// `ENOMOTO_MIP_CWM` が付いていれば `Some(学習なしか)`。
pub(crate) fn mode() -> Option<bool> {
    static F: std::sync::OnceLock<Option<bool>> = std::sync::OnceLock::new();
    *F.get_or_init(|| env_str!("ENOMOTO_MIP_CWM").filter(|v| *v != "0").map(|v| v == "fix"))
}

struct Bandit {
    wins: Vec<f64>,
    plays: Vec<f64>,
    /// 決まった対の数、引き分け、(腕 a, 腕 b, 勝った腕, a の数, b の数) の先頭の記録
    decided: u64,
    ties: u64,
    /// 対を始めた数と、始めた対の腕の m の和 (推移の記録)
    started: u64,
    trace: String,
    verbose: bool,
    /// 両側とも解き終えた対: 腕ごとの ln(自分の大きさ / 相手の大きさ) の和と数、大きさの和 (成績の指標が向きを当てるかの確かめ)
    rel: Vec<f64>,
    rel_n: Vec<f64>,
    tot: Vec<f64>,
}

static BANDIT: Mutex<Option<Bandit>> = Mutex::new(None);

/// 主の求解の根で呼ぶ: 状態を戻す。
pub(crate) fn reset(verbose: bool) {
    let k = arms().len();
    *BANDIT.lock().unwrap() = Some(Bandit { wins: vec![0.0; k], plays: vec![0.0; k], decided: 0, ties: 0, started: 0, trace: String::new(), verbose, rel: vec![0.0; k], rel_n: vec![0.0; k], tot: vec![0.0; k] });
}

fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn unit(z: u64) -> f64 {
    (z >> 11) as f64 / (1u64 << 53) as f64
}

impl Bandit {
    fn mean(&self, i: usize) -> f64 {
        (self.wins[i] + 1.0) / (self.plays[i] + 2.0)
    }
    fn best(&self) -> usize {
        // 同じなら m = 1 に近い腕
        let a = arms();
        (0..a.len()).max_by(|&i, &j| self.mean(i).total_cmp(&self.mean(j)).then_with(|| (a[j] - 1.0).abs().total_cmp(&(a[i] - 1.0).abs()))).unwrap_or(0)
    }
    fn pick2(&self, seed: u64) -> (usize, usize) {
        if mode() == Some(true) {
            let a = arms();
            let b = tunable!("ENOMOTO_T_MIP_CWM_FIX_B", usize::MAX, usize);
            let b = if b < a.len() { b } else { a.iter().position(|&m| m == 1.0).unwrap_or(a.len() - 1) };
            return (0, b.max(1));
        }
        let mut th: Vec<(f64, usize)> = (0..self.wins.len())
            .map(|i| {
                let m = self.mean(i);
                let sd = (m * (1.0 - m) / (self.plays[i] + 3.0)).sqrt();
                let u1 = unit(splitmix(seed ^ (i as u64).wrapping_mul(0xA24B_AED4_963E_E407))).max(1e-300);
                let u2 = unit(splitmix(seed.rotate_left(17) ^ (i as u64 + 1).wrapping_mul(0x9FB2_1C65_1E98_DF25)));
                let z = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
                (m + sd * z, i)
            })
            .collect();
        th.sort_by(|x, y| y.0.total_cmp(&x.0));
        (th[0].1, th[1].1)
    }
    fn summary(&self) -> String {
        let a = arms();
        let parts: Vec<String> = (0..a.len()).map(|i| format!("m={}: {}/{} (mean {:.2})", a[i], self.wins[i], self.plays[i], self.mean(i))).collect();
        let rel: Vec<String> = (0..a.len()).filter(|&i| self.rel_n[i] > 0.0).map(|i| format!("m={}: {:+.3} ({} pairs, {} nodes)", a[i], self.rel[i] / self.rel_n[i], self.rel_n[i], self.tot[i])).collect();
        format!("decided {}, ties {}, started {}; {}; best m={}; finished pairs mean ln(size/other) {}", self.decided, self.ties, self.started, parts.join(", "), a[self.best()], rel.join(", "))
    }
}

/// 対の片側の部分木の大きさの数え。
struct Pair {
    arms: [usize; 2],
    sizes: [AtomicU64; 2],
    done: [AtomicBool; 2],
    decided: AtomicBool,
    recorded: AtomicBool,
}

impl Pair {
    fn decide(&self, winner: Option<usize>) {
        if self.decided.swap(true, Relaxed) {
            return;
        }
        let mut g = BANDIT.lock().unwrap();
        let Some(b) = g.as_mut() else { return };
        let (sa, sb) = (self.sizes[0].load(Relaxed), self.sizes[1].load(Relaxed));
        match winner {
            Some(w) => {
                // 重み ln(1 + 勝った側の大きさ): 1〜2 ノードで終わる小さな部分木の勝ち負けは m の差をほとんど映さない (mcsched の 1 スレッドで
                // 対の 6 割が小さい方 ≤ 1 ノード) ので軽くする
                let x = (if w == 0 { sa } else { sb }) as f64;
                let wt = (1.0 + x).ln();
                b.wins[self.arms[w]] += wt;
                b.plays[self.arms[0]] += wt;
                b.plays[self.arms[1]] += wt;
                b.decided += 1;
                if b.trace.len() < 4000 {
                    b.trace.push_str(&format!(" {}v{}:{}({}/{})", arms()[self.arms[0]], arms()[self.arms[1]], arms()[self.arms[w]], sa, sb));
                }
                if b.verbose && b.decided % 25 == 0 {
                    eprintln!("MIP: CWM: {}", b.summary());
                }
            }
            None => b.ties += 1,
        }
    }
}

/// 対の片側 (ノードの札から参照される)。最後の参照がなくなると、その側の部分木は解き終わり。
pub(crate) struct Side {
    pair: Arc<Pair>,
    side: usize,
}

impl Drop for Side {
    fn drop(&mut self) {
        let p = &self.pair;
        let s = self.side;
        p.done[s].store(true, Relaxed);
        let (me, other) = (p.sizes[s].load(Relaxed), p.sizes[1 - s].load(Relaxed));
        if p.done[1 - s].load(Relaxed) && p.arms[0] != p.arms[1] && !p.recorded.swap(true, Relaxed) {
            // 両側とも解き終えた: 大きさの比を記録 (勝ち負けが先に決まっていても)
            if let Some(b) = BANDIT.lock().unwrap().as_mut() {
                let (a0, a1) = (p.sizes[0].load(Relaxed).max(1) as f64, p.sizes[1].load(Relaxed).max(1) as f64);
                b.rel[p.arms[0]] += (a0 / a1).ln();
                b.rel[p.arms[1]] += (a1 / a0).ln();
                b.rel_n[p.arms[0]] += 1.0;
                b.rel_n[p.arms[1]] += 1.0;
                b.tot[p.arms[0]] += a0;
                b.tot[p.arms[1]] += a1;
            }
        }
        if p.decided.load(Relaxed) {
            return;
        }
        if p.done[1 - s].load(Relaxed) {
            p.decide(if me < other { Some(s) } else if other < me { Some(1 - s) } else { None });
        } else if other > me {
            p.decide(Some(s));
        }
    }
}

/// ノードの札: 腕の倍率と、入れ子の対の側の鎖 (外側から)、一番内側の対を始めた深さ。
pub(crate) struct CwmTag {
    pub m: f64,
    chain: Vec<Arc<Side>>,
    start_depth: usize,
}

/// ノードを処理するとき: 鎖の全部の側の大きさを数え、このノードの倍率を返す。札がなければ今の一番の腕。
pub(crate) fn on_node(tag: Option<&Arc<CwmTag>>) -> f64 {
    match tag {
        Some(t) => {
            for sd in &t.chain {
                let p = &sd.pair;
                let s = sd.side;
                let me = p.sizes[s].fetch_add(1, Relaxed) + 1;
                if p.done[1 - s].load(Relaxed) && !p.decided.load(Relaxed) && me > p.sizes[1 - s].load(Relaxed) {
                    p.decide(Some(1 - s));
                }
            }
            t.m
        }
        None => {
            let g = BANDIT.lock().unwrap();
            g.as_ref().map_or(1.0, |b| arms()[b.best()])
        }
    }
}

/// 分枝のノード (深さ `depth`、札 `tag`) の 2 つの子の札。対を始めるなら違う腕を割り振る。`seed` は決定的な乱数の元。
pub(crate) fn children(tag: Option<&Arc<CwmTag>>, depth: usize, seed: u64) -> [Option<Arc<CwmTag>>; 2] {
    let span = tunable!("ENOMOTO_T_MIP_CWM_SPAN", 5usize, usize).max(1);
    if let Some(t) = tag {
        if depth + 1 < t.start_depth + span {
            return [Some(t.clone()), Some(t.clone())];
        }
    }
    let (a, b) = {
        let mut g = BANDIT.lock().unwrap();
        let Some(bd) = g.as_mut() else { return [tag.cloned(), tag.cloned()] };
        bd.started += 1;
        bd.pick2(seed)
    };
    // どちらの子にどちらの腕か
    let (a, b) = if splitmix(seed ^ 0x5851_F42D_4C95_7F2D) & 1 == 0 { (a, b) } else { (b, a) };
    let pair = Arc::new(Pair { arms: [a, b], sizes: [AtomicU64::new(0), AtomicU64::new(0)], done: [AtomicBool::new(false), AtomicBool::new(false)], decided: AtomicBool::new(false), recorded: AtomicBool::new(false) });
    let mk = |side: usize, arm: usize| {
        let mut chain: Vec<Arc<Side>> = tag.map(|t| t.chain.clone()).unwrap_or_default();
        chain.push(Arc::new(Side { pair: pair.clone(), side }));
        Some(Arc::new(CwmTag { m: arms()[arm], chain, start_depth: depth + 1 }))
    };
    [mk(0, a), mk(1, b)]
}

/// 終わりの記録 (勝ち負けと、決まった対の並びの先頭)。
pub(crate) fn final_line() -> Option<String> {
    let g = BANDIT.lock().unwrap();
    g.as_ref().map(|b| format!("{}; trace (a v b:winner(size a/size b)){}", b.summary(), b.trace))
}
