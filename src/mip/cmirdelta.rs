//! 試作 `ENOMOTO_MIP_CMIRDELTA` (既定では使わない、analysis/cmirdelta_20261107.md): CMIR の割る数 δ の候補を同時ディオファントス近似で作る。
//!
//! CMIR の強さは、δ で割った整数列の係数 `a_j / δ` の端数が 0 (か 1) の近くに集まるかで大きく変わる。基準の係数 `r` (LP 値が境界の内側の
//! 整数列の係数) をとり、`α_j = |a_j| / r` に対して `q α_j` が多くの j で同時にほぼ整数になる正の整数 `q` を探し、`δ = r / q` を候補にする
//! (`r / δ = q` は整数、`a_j / δ = q α_j` はほぼ整数)。`q` の作り方は 3 つ:
//!
//! - `g` (gcd): すべての α_j を 1e-9 の幅で整数にする最小の `q` (連続分数で分母を出して最小公倍数、上限 [`QMAX_GCD`])。
//!   HiGHS の `cmirCutGenerationHeuristic` の `integralScale` (連続の項がないときだけ) に当たる。
//! - `c` (連続分数): 各 α_j の連続分数の近似分数の分母 `q <= QMAX` (2 列だけを同時に整数にする)。
//! - `l` (LLL): 格子 `{(q ε, q α_1 - p_1, ..., q α_n - p_n)}` の基底を浮動小数点の LLL (Lenstra–Lenstra–Lovász 1982) で縮約し、
//!   短いベクトルの `q` を使う (Lagarias 1985 の同時近似)。変換行列は i64 で溢れを検査して持ち、`q` と残差は元の係数から計算し直して検算する。
//!
//! 値は `1` (= `gcl`) か、`g`・`c`・`l` の組み合わせ。候補の数 (`ENOMOTO_T_CMIRDELTA_K`、既定 8)・使う列の数 (`ENOMOTO_T_CMIRDELTA_N`、既定 8)・
//! `q` の上限 (`ENOMOTO_T_CMIRDELTA_QMAX`、既定 64)・LLL の入れ替えの回数 (列の数の 20 倍) と、全体の時間 (`ENOMOTO_T_CMIRDELTA_SECS`、
//! 既定 20 秒、全スレッドの合計) に上限がある。
//!
//! 診断: 新しい δ が既存の候補より効き目が大きかった回数、その δ で作ったカットが候補になった数・根で選ばれた数を数え、
//! 根のカットのループの終わりに `SEP cmirdelta ...` の 1 行を出す (`ENOMOTO_MIP_LOG` のとき)。

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

const QMAX_GCD: i64 = 10_000;

/// 試作の値 (`g`・`c`・`l` のどれを使うか、ビット 1・2・4)。使わなければ 0
pub(crate) fn mode() -> u8 {
    static M: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
    // 2026-11-09 から既定 `n` (cand1108zd、`=0` で使わない)
    *M.get_or_init(|| match env_def!("ENOMOTO_MIP_CMIRDELTA", "n") {
        Some(v) if !v.is_empty() => {
            if v == "1" {
                7
            } else if v.contains('n') {
                // 絞った形 (`n`、[`narrow_deltas`]): 連続分数を主に、LLL は足りないときだけ
                8 | 2 | 4
            } else {
                (v.contains('g') as u8) | ((v.contains('c') as u8) << 1) | ((v.contains('l') as u8) << 2)
            }
        }
        _ => 0,
    })
}

pub(crate) fn on() -> bool {
    mode() != 0
}

// ---------------------------------------------------------------- 集計

static CALLS: AtomicU64 = AtomicU64::new(0);
static CALLS_NEW: AtomicU64 = AtomicU64::new(0);
static NEW_DELTAS: AtomicU64 = AtomicU64::new(0);
static WINS: AtomicU64 = AtomicU64::new(0);
/// 勝った回の log(新しい効き目 / 既存の最良の効き目) の和 (既存が無効なら数えない) を 1e6 倍した整数
static GAIN_LOG_MICRO: AtomicU64 = AtomicU64::new(0);
static WINS_NOOLD: AtomicU64 = AtomicU64::new(0);
static WIN_KIND: [AtomicU64; 3] = [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];
static TIME_NS: AtomicU64 = AtomicU64::new(0);
static CAND_NEW: AtomicU64 = AtomicU64::new(0);
static CHOSEN_ALL: AtomicU64 = AtomicU64::new(0);
static CHOSEN_NEW: AtomicU64 = AtomicU64::new(0);
static EFF_ALL_MICRO: AtomicU64 = AtomicU64::new(0);
static EFF_NEW_MICRO: AtomicU64 = AtomicU64::new(0);
static RAW_NEW: Mutex<Option<HashSet<u64>>> = Mutex::new(None);
static CAND_SET: Mutex<Option<HashSet<u64>>> = Mutex::new(None);

fn fp(coefs: &[(usize, f64)], rhs: f64) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for &(k, c) in coefs {
        k.hash(&mut h);
        c.to_bits().hash(&mut h);
    }
    rhs.to_bits().hash(&mut h);
    h.finish()
}

fn set_insert(s: &Mutex<Option<HashSet<u64>>>, v: u64) {
    let mut g = s.lock().unwrap();
    let set = g.get_or_insert_with(HashSet::new);
    if set.len() > 2_000_000 {
        set.clear();
    }
    set.insert(v);
}

fn set_has(s: &Mutex<Option<HashSet<u64>>>, v: u64) -> bool {
    s.lock().unwrap().as_ref().is_some_and(|set| set.contains(&v))
}

/// 新しい δ で作った CMIR (置き換えを戻した後の生のカット) を覚える
pub(crate) fn note_new_raw(coefs: &[(usize, f64)], rhs: f64) {
    set_insert(&RAW_NEW, fp(coefs, rhs));
}

/// 生のカットが新しい δ で作ったものか (`finish_cut` の前に)
pub(crate) fn is_new_raw(coefs: &[(usize, f64)], rhs: f64) -> bool {
    set_has(&RAW_NEW, fp(coefs, rhs))
}

/// 新しい δ から来た候補 (`finish_cut` の後) を覚える
pub(crate) fn note_new_cand(coefs: &[(usize, f64)], rhs: f64) {
    CAND_NEW.fetch_add(1, Ordering::Relaxed);
    set_insert(&CAND_SET, fp(coefs, rhs));
}

/// 根で選ばれたカットを数える (新しい δ から来たものを別に)
pub(crate) fn note_chosen(coefs: &[(usize, f64)], rhs: f64, eff: f64) {
    CHOSEN_ALL.fetch_add(1, Ordering::Relaxed);
    EFF_ALL_MICRO.fetch_add((eff.clamp(0.0, 1e6) * 1e6) as u64, Ordering::Relaxed);
    if set_has(&CAND_SET, fp(coefs, rhs)) {
        CHOSEN_NEW.fetch_add(1, Ordering::Relaxed);
        EFF_NEW_MICRO.fetch_add((eff.clamp(0.0, 1e6) * 1e6) as u64, Ordering::Relaxed);
    }
}

/// δ を選んだ結果を数える。`kind` は勝った新しい δ の作り方 (0 = g, 1 = c, 2 = l)。`old_eff` は既存の候補の最良の効き目
pub(crate) fn note_win(kind: usize, new_eff: f64, old_eff: Option<f64>) {
    WINS.fetch_add(1, Ordering::Relaxed);
    WIN_KIND[kind.min(2)].fetch_add(1, Ordering::Relaxed);
    match old_eff {
        Some(o) if o > 0.0 => {
            GAIN_LOG_MICRO.fetch_add(((new_eff / o).ln().max(0.0) * 1e6) as u64, Ordering::Relaxed);
        }
        _ => {
            WINS_NOOLD.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// 集計の 1 行 (累計、全スレッド・サブ MIP を含む)
pub(crate) fn summary() -> String {
    let g = |a: &AtomicU64| a.load(Ordering::Relaxed);
    let wins = g(&WINS);
    let wo = wins - g(&WINS_NOOLD).min(wins);
    let (ca, cn) = (g(&CHOSEN_ALL), g(&CHOSEN_NEW));
    format!(
        "SEP cmirdelta: calls {} with_new {} new_deltas {} wins {} (g {} c {} l {}, no_old {}) mean_gain {:.3} cands_new {} chosen {}/{} eff_new {:.4} eff_all {:.4} time {:.3}s",
        g(&CALLS),
        g(&CALLS_NEW),
        g(&NEW_DELTAS),
        wins,
        g(&WIN_KIND[0]),
        g(&WIN_KIND[1]),
        g(&WIN_KIND[2]),
        g(&WINS_NOOLD),
        if wo > 0 { (g(&GAIN_LOG_MICRO) as f64 / 1e6 / wo as f64).exp() } else { 1.0 },
        g(&CAND_NEW),
        cn,
        ca,
        if cn > 0 { g(&EFF_NEW_MICRO) as f64 / 1e6 / cn as f64 } else { 0.0 },
        if ca > 0 { g(&EFF_ALL_MICRO) as f64 / 1e6 / ca as f64 } else { 0.0 },
        g(&TIME_NS) as f64 / 1e9
    )
}

// ---------------------------------------------------------------- δ の候補

/// 整数の項 `(係数, LP 値, 上界)` (置き換え後の y の空間) と既存の候補 `existing` から、新しい δ の候補を `(δ, 作り方)` で返す
/// (`existing` と重なるものは除く)。作り方は 0 = g, 1 = c, 2 = l
/// `best_old` は既存の候補の中で最良の δ (効き目のあるカットが作れなければ `None`)。絞った形 (`n`) だけが使う
pub(crate) fn extra_deltas(ints: &[(f64, f64, f64)], existing: &[f64], best_old: Option<f64>) -> Vec<(f64, usize)> {
    let m = mode();
    let mut out: Vec<(f64, usize)> = Vec::new();
    if m == 0 {
        return out;
    }
    if m & 8 != 0 {
        return narrow_deltas(ints, existing, best_old);
    }
    let budget = tunable!("ENOMOTO_T_CMIRDELTA_SECS", 20.0, f64);
    if TIME_NS.load(Ordering::Relaxed) as f64 > budget * 1e9 {
        return out;
    }
    let t0 = std::time::Instant::now();
    CALLS.fetch_add(1, Ordering::Relaxed);
    let kmax = tunable!("ENOMOTO_T_CMIRDELTA_K", 8usize, usize);
    let nmax = tunable!("ENOMOTO_T_CMIRDELTA_N", 8usize, usize).max(1);
    let qmax = tunable!("ENOMOTO_T_CMIRDELTA_QMAX", 64i64, i64).max(2);
    // 列: LP 値が境界の内側のものを先に、次に係数の大きい順。|a| の重複は除く
    let mut cols: Vec<(f64, bool)> = ints
        .iter()
        .filter(|&&(a, _, _)| a.abs() > 1e-6 && a.is_finite())
        .map(|&(a, yv, yu)| (a.abs(), yv > 1e-6 && yv < yu - 1e-6))
        .collect();
    cols.sort_by(|x, y| y.1.cmp(&x.1).then(y.0.total_cmp(&x.0)));
    // 長い行で 2 乗の手間にならないよう、使う数 (基準 2 つ + nmax) が集まったら止める
    let mut vals: Vec<(f64, bool)> = Vec::new();
    for c in cols {
        if vals.len() >= nmax + 2 {
            break;
        }
        if !vals.iter().any(|&(v, _)| (v - c.0).abs() <= 1e-9 * v) {
            vals.push(c);
        }
    }
    if vals.len() < 2 {
        TIME_NS.fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
        return out;
    }
    // 基準: 内側の列の係数 (最大 2 つ)。なければ最大の係数
    let refs: Vec<f64> = {
        let r: Vec<f64> = vals.iter().filter(|v| v.1).map(|v| v.0).take(2).collect();
        if r.is_empty() {
            vec![vals[0].0]
        } else {
            r
        }
    };
    let push = |out: &mut Vec<(f64, usize)>, d: f64, kind: usize| {
        if !(d.is_finite() && d > 1e-9) || out.len() >= kmax {
            return;
        }
        if existing.iter().chain(out.iter().map(|(e, _)| e)).any(|&e| (e - d).abs() <= 1e-9 * d) {
            return;
        }
        out.push((d, kind));
    };
    for &r in &refs {
        let alphas: Vec<f64> = vals.iter().filter(|v| (v.0 - r).abs() > 1e-9 * r).take(nmax).map(|v| v.0 / r).collect();
        if alphas.is_empty() {
            continue;
        }
        // g: すべてを整数にする最小の q
        if m & 1 != 0 {
            if let Some(q) = common_denominator(&alphas, QMAX_GCD) {
                if q >= 2 {
                    push(&mut out, r / q as f64, 0);
                }
            }
        }
        // l: LLL
        if m & 4 != 0 && alphas.len() >= 2 {
            if r == refs[0] {
                for q in lll_multipliers(&alphas, qmax, qmax) {
                    push(&mut out, r / q as f64, 2);
                }
            }
        }
        // c: 連続分数 (列ごと、最初の 4 列)
        if m & 2 != 0 {
            for &a in alphas.iter().take(4) {
                for q in convergent_denominators(a, qmax) {
                    if q >= 2 {
                        push(&mut out, r / q as f64, 1);
                    }
                }
            }
        }
    }
    if !out.is_empty() {
        CALLS_NEW.fetch_add(1, Ordering::Relaxed);
        NEW_DELTAS.fetch_add(out.len() as u64, Ordering::Relaxed);
    }
    let el = t0.elapsed().as_nanos() as u64;
    TIME_NS.fetch_add(el, Ordering::Relaxed);
    out
}

/// 絞った形 (`ENOMOTO_MIP_CMIRDELTA=n`)。次のときだけ候補を作る:
/// (1) 既存の候補で効き目のあるカットが作れる (`best_old` がある)、(2) その δ で内側の整数列の端数 `frac(|a_j| / δ)` が 0.02〜0.98 の列がある
/// (そろっていない)。候補は内側の列の係数を基準 r に、ほかの列の α_j = |a_j| / r の連続分数の分母 q から δ = r / q を作り、
/// 「q α_j の端数が 0.05 以内になる内側の列の数」(同点は q の小さい順) で並べて上位 `ENOMOTO_T_CMIRDELTA_NK` (既定 3) 個。
/// 内側の列が 4 つ以上あり、連続分数の最良でも半分未満の列しかそろわないときだけ LLL の q も比べる
fn narrow_deltas(ints: &[(f64, f64, f64)], existing: &[f64], best_old: Option<f64>) -> Vec<(f64, usize)> {
    let mut out: Vec<(f64, usize)> = Vec::new();
    let Some(d0) = best_old else {
        return out;
    };
    let budget = tunable!("ENOMOTO_T_CMIRDELTA_SECS", 20.0, f64);
    if TIME_NS.load(Ordering::Relaxed) as f64 > budget * 1e9 {
        return out;
    }
    let t0 = std::time::Instant::now();
    CALLS.fetch_add(1, Ordering::Relaxed);
    let kmax = tunable!("ENOMOTO_T_CMIRDELTA_NK", 3usize, usize);
    let nmax = tunable!("ENOMOTO_T_CMIRDELTA_N", 8usize, usize).max(1);
    let qmax = tunable!("ENOMOTO_T_CMIRDELTA_QMAX", 64i64, i64).max(2);
    // 内側の整数列の |a| (重複を除き、大きい順に nmax + 1 個まで)
    let mut inner: Vec<f64> = ints.iter().filter(|&&(a, yv, yu)| a.abs() > 1e-6 && a.is_finite() && yv > 1e-6 && yv < yu - 1e-6).map(|t| t.0.abs()).collect();
    let misaligned = inner.iter().any(|&a| {
        let v = a / d0;
        let f = v - v.floor();
        (0.02..=0.98).contains(&f)
    });
    let done = |out: Vec<(f64, usize)>| {
        if !out.is_empty() {
            CALLS_NEW.fetch_add(1, Ordering::Relaxed);
            NEW_DELTAS.fetch_add(out.len() as u64, Ordering::Relaxed);
        }
        TIME_NS.fetch_add(t0.elapsed().as_nanos() as u64, Ordering::Relaxed);
        out
    };
    if !misaligned {
        return done(out);
    }
    inner.sort_by(|x, y| y.total_cmp(x));
    let mut vals: Vec<f64> = Vec::new();
    for a in inner {
        if vals.len() > nmax {
            break;
        }
        if !vals.iter().any(|&v| (v - a).abs() <= 1e-9 * v) {
            vals.push(a);
        }
    }
    if vals.len() < 2 {
        return done(out);
    }
    let score = |r: f64, q: i64| -> usize {
        vals.iter()
            .filter(|&&a| {
                let v = q as f64 * a / r;
                (v - v.round()).abs() <= 0.05
            })
            .count()
    };
    // (そろう列の数, -q, δ, 作り方)
    let mut cands: Vec<(usize, i64, f64, usize)> = Vec::new();
    for &r in vals.iter().take(2) {
        for &a in vals.iter().filter(|&&a| (a - r).abs() > 1e-9 * r).take(4) {
            for q in convergent_denominators(a / r, qmax) {
                if q >= 2 {
                    cands.push((score(r, q), -q, r / q as f64, 1));
                }
            }
        }
    }
    let best_cf = cands.iter().map(|c| c.0).max().unwrap_or(0);
    if vals.len() >= 4 && 2 * best_cf < vals.len() {
        let r = vals[0];
        let alphas: Vec<f64> = vals[1..].iter().map(|&a| a / r).collect();
        for q in lll_multipliers(&alphas, qmax, qmax) {
            cands.push((score(r, q), -q, r / q as f64, 2));
        }
    }
    cands.sort_by(|x, y| y.0.cmp(&x.0).then(y.1.cmp(&x.1)));
    for (sc, _, d, kind) in cands {
        if out.len() >= kmax {
            break;
        }
        // 基準の列だけしかそろわない (sc <= 1) 候補は使わない
        if sc < 2 || !(d.is_finite() && d > 1e-9) {
            continue;
        }
        if existing.iter().chain(out.iter().map(|(e, _)| e)).any(|&e| (e - d).abs() <= 1e-9 * d) {
            continue;
        }
        out.push((d, kind));
    }
    done(out)
}

/// `x` の連続分数の近似分数の分母 (1 を除かない、`qmax` 以下)。
pub(crate) fn convergent_denominators(x: f64, qmax: i64) -> Vec<i64> {
    let mut out = Vec::new();
    let mut f = x - x.floor();
    // 分母の漸化式 k_n = a_n k_{n-1} + k_{n-2}
    let (mut k1, mut k2): (i64, i64) = (1, 0);
    out.push(1);
    for _ in 0..40 {
        if f < 1e-9 {
            break;
        }
        let y = 1.0 / f;
        let an = y.floor();
        f = y - an;
        if an > qmax as f64 {
            break;
        }
        let Some(k) = (an as i64).checked_mul(k1).and_then(|v| v.checked_add(k2)) else {
            break;
        };
        if k > qmax {
            break;
        }
        out.push(k);
        k2 = k1;
        k1 = k;
    }
    out
}

/// すべての `alphas` を 1e-9 の幅で整数にする最小の正の整数 (`qmax` を超えれば `None`)。
pub(crate) fn common_denominator(alphas: &[f64], qmax: i64) -> Option<i64> {
    let mut q: i64 = 1;
    for &a in alphas {
        let v = q as f64 * a;
        if (v - v.round()).abs() <= 1e-9 * v.abs().max(1.0) {
            continue;
        }
        // 連続分数で、v の端数を整数にする分母
        let d = convergent_denominators(v, qmax).into_iter().find(|&d| {
            let w = d as f64 * v;
            (w - w.round()).abs() <= 1e-9 * w.abs().max(1.0)
        })?;
        q = q.checked_mul(d)?;
        if q > qmax {
            return None;
        }
    }
    // 検算
    for &a in alphas {
        let v = q as f64 * a;
        if (v - v.round()).abs() > 1e-9 * v.abs().max(1.0) {
            return None;
        }
    }
    Some(q)
}

/// 浮動小数点の LLL で、`q α_j` を同時にほぼ整数にする `q` (1 ≤ q ≤ `qmax`) を返す。`q ε` の重み ε は `0.25 / eps_q`。
/// 返す `q` は元の α から残差を計算し直し、端数が 0.05 以内の列が 2 つ以上あるものだけ (残差の小さい順)
pub(crate) fn lll_multipliers(alphas: &[f64], eps_q: i64, qmax: i64) -> Vec<i64> {
    let n = alphas.len();
    let d = n + 1;
    let eps = 0.25 / eps_q as f64;
    // 行 0: (ε, frac(α_1), ..., frac(α_n))、行 j: e_j
    let mut b: Vec<Vec<f64>> = vec![vec![0.0; d]; d];
    b[0][0] = eps;
    for j in 0..n {
        b[0][j + 1] = alphas[j] - alphas[j].floor();
        b[j + 1][j + 1] = 1.0;
    }
    let mut u: Vec<Vec<i64>> = (0..d).map(|i| (0..d).map(|j| (i == j) as i64).collect()).collect();
    if !lll(&mut b, &mut u, 20 * d) {
        return Vec::new();
    }
    let mut cand: Vec<(f64, i64)> = Vec::new();
    for row in &u {
        let q = row[0].abs();
        if q < 1 || q > qmax {
            continue;
        }
        // 検算: q α_j の端数を元の値から
        let mut near = 0;
        let mut err2 = 0.0;
        for &a in alphas {
            let v = q as f64 * a;
            let e = (v - v.round()).abs();
            err2 += e * e;
            if e <= 0.05 {
                near += 1;
            }
        }
        if near >= 2.min(n) {
            cand.push((err2, q));
        }
    }
    cand.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut out: Vec<i64> = Vec::new();
    for (_, q) in cand {
        if !out.contains(&q) {
            out.push(q);
        }
    }
    out
}

fn gram_schmidt(b: &[Vec<f64>]) -> (Vec<Vec<f64>>, Vec<f64>) {
    let d = b.len();
    let mut bs: Vec<Vec<f64>> = b.to_vec();
    let mut mu = vec![vec![0.0; d]; d];
    let mut bn = vec![0.0; d];
    for i in 0..d {
        for j in 0..i {
            let m = if bn[j] > 0.0 { dot(&b[i], &bs[j]) / bn[j] } else { 0.0 };
            mu[i][j] = m;
            for t in 0..b[i].len() {
                bs[i][t] -= m * bs[j][t];
            }
        }
        bn[i] = dot(&bs[i], &bs[i]);
    }
    (mu, bn)
}

fn dot(x: &[f64], y: &[f64]) -> f64 {
    x.iter().zip(y).map(|(a, b)| a * b).sum()
}

/// LLL (δ = 0.75)。`u` は `b` の行の整数の変換 (b = u b_初め)。入れ替えが `max_swaps` を超えるか、u の成分が溢れそうなら `false`
fn lll(b: &mut [Vec<f64>], u: &mut [Vec<i64>], max_swaps: usize) -> bool {
    let d = b.len();
    let (mut mu, mut bn) = gram_schmidt(b);
    if bn.iter().any(|&v| v <= 0.0) {
        return false;
    }
    let mut k = 1;
    let mut swaps = 0;
    const LIM: i64 = 1 << 40;
    while k < d {
        for j in (0..k).rev() {
            let q = mu[k][j].round();
            if q == 0.0 {
                continue;
            }
            if q.abs() > LIM as f64 {
                return false;
            }
            let qi = q as i64;
            for t in 0..d {
                let Some(v) = qi.checked_mul(u[j][t]).and_then(|w| u[k][t].checked_sub(w)) else {
                    return false;
                };
                if v.abs() > LIM {
                    return false;
                }
                u[k][t] = v;
            }
            let (bk, bj) = if k > j {
                let (lo, hi) = b.split_at_mut(k);
                (&mut hi[0], &lo[j])
            } else {
                unreachable!()
            };
            for t in 0..bk.len() {
                bk[t] -= q * bj[t];
            }
            for i in 0..j {
                mu[k][i] -= q * mu[j][i];
            }
            mu[k][j] -= q;
        }
        if bn[k] >= (0.75 - mu[k][k - 1] * mu[k][k - 1]) * bn[k - 1] {
            k += 1;
        } else {
            b.swap(k, k - 1);
            u.swap(k, k - 1);
            swaps += 1;
            if swaps > max_swaps {
                return false;
            }
            // Gram–Schmidt の更新 (Cohen 1993 の Algorithm 2.6.3 の SWAP)
            let m = mu[k][k - 1];
            let big = bn[k] + m * m * bn[k - 1];
            if big <= 0.0 {
                return false;
            }
            mu[k][k - 1] = m * bn[k - 1] / big;
            bn[k] = bn[k - 1] * bn[k] / big;
            bn[k - 1] = big;
            for j in 0..k - 1 {
                let t = mu[k - 1][j];
                mu[k - 1][j] = mu[k][j];
                mu[k][j] = t;
            }
            for i in k + 1..d {
                let t = mu[i][k];
                mu[i][k] = mu[i][k - 1] - m * t;
                mu[i][k - 1] = t + mu[k][k - 1] * mu[i][k];
            }
            k = (k - 1).max(1);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convergents() {
        // 0.375 = 3/8: 分母 1, 2, 3, 8
        let q = convergent_denominators(0.375, 64);
        assert_eq!(*q.last().unwrap(), 8);
        assert!(convergent_denominators(std::f64::consts::PI, 64).contains(&7));
    }

    #[test]
    fn gcd_scale() {
        // 1.5, 2.25 → q = 4
        assert_eq!(common_denominator(&[1.5, 2.25], 10_000), Some(4));
        assert_eq!(common_denominator(&[std::f64::consts::PI], 10_000), None);
    }

    #[test]
    fn lll_finds_common_multiplier() {
        // α = (1/3 + 1e-4, 2/3 - 1e-4, 5/3) は q = 3 でほぼ整数
        let q = lll_multipliers(&[1.0 / 3.0 + 1e-4, 2.0 / 3.0 - 1e-4, 5.0 / 3.0], 64, 64);
        assert!(q.contains(&3), "{q:?}");
        // 7 で割り切れる組
        let q = lll_multipliers(&[1.0 / 7.0, 3.0 / 7.0, 9.0 / 7.0 + 2e-4, 0.5714286], 64, 64);
        assert!(q.contains(&7), "{q:?}");
    }

    #[test]
    fn narrow_only_when_misaligned() {
        let ints = [(3.0, 0.5, 10.0), (-4.5, 0.5, 10.0), (7.5, 0.3, 10.0), (2.0, 0.0, 10.0)];
        // δ = 3 では 4.5 / 3 の端数が 0.5 (そろっていない) → δ = 1.5 (3・4.5・7.5 がそろう) を足す
        let out = narrow_deltas(&ints, &[3.0, 1.5 / 2.0], Some(3.0));
        assert!(out.iter().any(|&(d, _)| (d - 1.5).abs() < 1e-12), "{out:?}");
        // δ = 1.5 ならそろっているので足さない。既存の候補でカットが作れないときも足さない
        assert!(narrow_deltas(&ints, &[1.5], Some(1.5)).is_empty());
        assert!(narrow_deltas(&ints, &[3.0], None).is_empty());
    }

    #[test]
    fn extra_deltas_mode_off_is_empty() {
        if mode() == 0 {
            assert!(extra_deltas(&[(3.0, 0.5, 1.0), (5.0, 0.2, 1.0)], &[3.0], Some(3.0)).is_empty());
        }
    }
}

#[cfg(test)]
mod timing {
    #[test]
    #[ignore]
    fn time_pieces() {
        let mut s: u64 = 12345;
        let mut rnd = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (s >> 11) as f64 / (1u64 << 53) as f64
        };
        let sets: Vec<Vec<f64>> = (0..1000).map(|_| (0..8).map(|_| 0.1 + 5.0 * rnd()).collect()).collect();
        let t = std::time::Instant::now();
        let mut c = 0;
        for a in &sets {
            c += super::lll_multipliers(a, 64, 64).len();
        }
        eprintln!("lll {:.1}us/call ({c})", t.elapsed().as_secs_f64() * 1e6 / 1000.0);
        let t = std::time::Instant::now();
        for a in &sets {
            for &x in a.iter().take(4) {
                c += super::convergent_denominators(x, 64).len();
            }
            c += super::common_denominator(a, 10_000).is_some() as usize;
        }
        eprintln!("cf+gcd {:.1}us/call ({c})", t.elapsed().as_secs_f64() * 1e6 / 1000.0);
    }
}
