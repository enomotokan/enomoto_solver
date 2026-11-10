//! 試作 `ENOMOTO_MIP_BRCUT` (既定では使わない。analysis/brcut_20261110.md): カット選択の履歴を使う分枝
//! (Turner, Berthold, Besançon, Koch 2023「Branching via Cutting Plane Selection: Improving Hybrid Branching」)。
//!
//! 列 x_j での分枝 x_j ≤ ⌊v⌋ ∨ x_j ≥ ⌈v⌉ は、x_j が基底にある tableau 行から作る GMI カットの split と同じなので、
//! その行から作ったカットの効き目 (efficacy) が大きい列は、分枝しても下界がよく動くと見て点数を上げる。
//! tableau 行の分離 ([`super::separation`] の `SRC_TAB`) の 1 回ごとに、行 (元の基底の列) ごとの候補のカット
//! (採られたかに関わらず) の効き目の最大を、その回の全ての行の最大で割った値 (0〜1) を記録し、列ごとの平均と最後の値を持つ。
//! 分枝の hybrid 点数 ([`super::pseudocost::Pseudocost`] の `hybrid`) に `dyn * (w_avg * 平均 + w_last * 最後の値)` を足す
//! (SCIP 9 の branch_relpscost.c `calcScore` と sepa_gomory.c の形。SCIP では効き目のあるカットを作った行だけを記録し、
//! 正規化は同じ回の行の最大で割る)。
//!
//! 状態はプロセスで 1 つ (根の分離と木の各スレッドの分離・分枝で共有する)。外側の求解の始めに空にし、サブ MIP では使わない。

use std::sync::RwLock;

struct State {
    sum: Vec<f64>,
    n: Vec<u32>,
    last: Vec<f64>,
    /// 記録した回数・行の数 (診断用)
    rounds: u64,
    rows: u64,
}

static ST: RwLock<State> = RwLock::new(State { sum: Vec::new(), n: Vec::new(), last: Vec::new(), rounds: 0, rows: 0 });

/// 試作を使うか (`ENOMOTO_MIP_BRCUT` が空でも `0` でもない)。
pub fn on() -> bool {
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| env_str!("ENOMOTO_MIP_BRCUT").is_some_and(|v| v != "0"))
}

/// 平均の重み `ENOMOTO_T_MIP_BRCUT_WAVG` と最後の値の重み `ENOMOTO_T_MIP_BRCUT_WLAST`。既定は SCIP (branch_relpscost.c の
/// `DEFAULT_GMIAVGEFFWEIGHT` 0、`DEFAULT_GMILASTEFFWEIGHT` 1e-5) と同じ。SCIP と同じく推論・打ち切りの項と一緒に動的な重みを掛ける
fn weights() -> (f64, f64) {
    static V: std::sync::OnceLock<(f64, f64)> = std::sync::OnceLock::new();
    *V.get_or_init(|| (tunable!("ENOMOTO_T_MIP_BRCUT_WAVG", 0.0, f64), tunable!("ENOMOTO_T_MIP_BRCUT_WLAST", 1e-5, f64)))
}

/// `ENOMOTO_MIP_BRCUT` の値に `z` があれば、カットの出なかった行も 0 として記録する (既定は SCIP の sepa_gomory.c と同じく、
/// 効き目のあるカットを作った行だけを記録する)。
fn record_zero() -> bool {
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| env_str!("ENOMOTO_MIP_BRCUT").is_some_and(|v| v.contains('z')))
}

fn log_on() -> bool {
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| env_str!("ENOMOTO_MIP_BRCUT_LOG").is_some())
}

/// 外側の求解の始め: 列の数 `n` で空にする。
pub fn reset(n: usize) {
    if !on() {
        return;
    }
    let mut st = ST.write().unwrap();
    st.sum = vec![0.0; n];
    st.n = vec![0; n];
    st.last = vec![0.0; n];
    st.rounds = 0;
    st.rows = 0;
}

/// tableau 行の分離の 1 回の記録: (元の基底の列, その行の候補のカットの効き目の最大 (カットがなければ 0))。
pub fn record(round: &[(usize, f64)]) {
    if round.is_empty() {
        return;
    }
    let emax = round.iter().fold(0.0f64, |m, &(_, e)| m.max(e));
    let zero = record_zero();
    let mut st = ST.write().unwrap();
    st.rounds += 1;
    for &(j, e) in round {
        if j >= st.n.len() || (e <= 0.0 && !zero) {
            continue;
        }
        let v = if emax > 0.0 { (e / emax).clamp(0.0, 1.0) } else { 0.0 };
        st.sum[j] += v;
        st.n[j] += 1;
        st.last[j] = v;
        st.rows += 1;
    }
    if log_on() && st.rounds.is_power_of_two() {
        let nc = st.n.iter().filter(|&&k| k > 0).count();
        eprintln!("BRCUT rounds {} rows {} cols {} (this round {} rows, max eff {:.4})", st.rounds, st.rows, nc, round.len(), emax);
    }
}

/// 分枝の点数に足す値 (記録のない列は 0。呼び出し側で動的な重みを掛ける)。
pub fn term(j: usize) -> f64 {
    let (wa, wl) = weights();
    let st = ST.read().unwrap();
    match st.n.get(j) {
        Some(&k) if k > 0 => wa * st.sum[j] / k as f64 + wl * st.last[j],
        _ => 0.0,
    }
}

/// 診断: (記録した回数, 行の数, 記録のある列の数)。
pub fn stats() -> (u64, u64, usize) {
    let st = ST.read().unwrap();
    (st.rounds, st.rows, st.n.iter().filter(|&&k| k > 0).count())
}
