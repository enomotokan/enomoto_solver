//! 試作 `ENOMOTO_MIP_CUTCONF` (既定では使わない): cut-based の衝突解析 (Mexi, Serrano, Berthold, Gleixner, Nordström 2025、
//! 「Cut-based Conflict Analysis in Mixed Integer Programming」、INFORMS JoC。SCIP 10 の実装の考え方)。
//!
//! 衝突解析を境界のリテラルのグラフの上ではなく、線形の不等式の線形結合と丸めの列として行う:
//! 1. 今の境界で破れている制約 (伝播で矛盾した行、または双対証明・Farkas の証明) を衝突の制約 `C: sum c_j x_j <= b` とする。
//! 2. `C` の最小活動量を上げている境界の変更 (`c_j > 0` なら下限、`c_j < 0` なら上限) のうち、記録の最も新しいもの `k`
//!    (列 `x_p`) を、その理由の行 `R` (伝播した元の行) と線形結合して消す: `C' = C + λ R` (`λ = -c_p / ρ_p > 0`)。
//!    `C'` は妥当な不等式の非負結合なので常に妥当。`k` の直前の境界で `C'` がまだ破れていれば採る。
//! 3. 破れなくなる (理由の行の、整数の丸めによる余裕が大きい) ときは、`R` を `ρ_p` で割って MIR で丸めた `R'` (理由の
//!    「縮約」。各列は大域的な境界のうち `k` の時点の値に近い方で補にするので、0-1 列なら `k` の時点で余裕が 0 になる) で
//!    結合し直す。それでも破れなければそこで止める。
//! 4. 最後の決定の段で `C` の最小活動量を上げている変更が 1 つになったら (1-UIP)、または理由のない変更 (分枝・外から来た
//!    変更) に当たったら止める。
//!
//! 使う境界は大域的な境界 (根の境界と大域的な締め付け) だけなので、得た制約は (数値誤差を除き) 境界の記録に関わらず妥当。
//! 大域的な締め付けには打ち切り値による固定も入るので、妥当なのは暫定解より良い解について (双対証明の CMIR と同じ)。
//! 論文の division・saturation、各段の CMIR による強め、一般整数の扱いの細部は確かめられていない (未確認) ので、
//! ここの縮約は ENOMOTO の形 (MIR で 1 回丸める) にしてある。

use super::domain::{ConflictSrc, ConflictStart, Domain};
use super::problem::MipProblem;

const NONE: usize = usize::MAX;

/// 統計の番号 ([`stats`] の配列の位置)。
pub mod st {
    pub const CALLS: usize = 0;
    /// 始めの制約が作れない (列の矛盾・無限の境界) か、破れていない
    pub const NO_START: usize = 1;
    /// そのままの結合で進んだ段
    pub const PLAIN: usize = 2;
    /// 理由の行を MIR で縮約して進んだ段
    pub const REDUCED: usize = 3;
    /// 結合しても破れなくなって止めた
    pub const FAIL_STEP: usize = 4;
    /// 理由のない変更 (決定・外から来た変更) に当たって止めた
    pub const STOP_DEC: usize = 5;
    /// 1-UIP に着いて止めた
    pub const STOP_UIP: usize = 6;
    /// 手間・長さの上限で止めた
    pub const STOP_BUDGET: usize = 7;
    /// 衝突の制約を返した数
    pub const OUT: usize = 8;
    /// 返した制約の長さの和
    pub const OUT_LEN: usize = 9;
    /// 最後に今の境界で破れていない・長すぎるので捨てた
    pub const DROP: usize = 10;
    /// 1 段も進まなかった (始めから 1-UIP など)
    pub const NO_STEP: usize = 11;
    /// 返した制約のうち 0-1 列以外を含むもの
    pub const OUT_NONBIN: usize = 12;
    /// 外から来た変更を覚えた行で置き換えた段
    pub const EXT: usize = 13;
    /// 結合しても破れなくなって止めた段の列の種類: 一般整数、連続 (残りは 0-1)
    pub const FAIL_GENINT: usize = 14;
    pub const FAIL_CONT: usize = 15;
    /// 証明のプールの cutconf の行が、ノードを刈った・境界を締めた回数 (このスレッドで作ったもの)
    pub const USE_PRUNE: usize = 16;
    pub const USE_TIGHTEN: usize = 17;
    pub const N: usize = 18;
}

static STATS: [std::sync::atomic::AtomicU64; st::N] = [const { std::sync::atomic::AtomicU64::new(0) }; st::N];
/// 全スレッドで解析に使った時間 (マイクロ秒)。
static MICROS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

thread_local! {
    static SECS: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    static ACC: std::cell::RefCell<Acc> = std::cell::RefCell::new(Acc::default());
}

/// 全スレッドの統計。
pub fn stats() -> [u64; st::N] {
    std::array::from_fn(|i| STATS[i].load(std::sync::atomic::Ordering::Relaxed))
}

/// このスレッドで解析に使った秒数 (時間の割合の上限に使う)。
pub fn secs() -> f64 {
    SECS.with(|s| s.get())
}

/// 全スレッドで解析に使った秒数。
pub fn total_secs() -> f64 {
    MICROS.load(std::sync::atomic::Ordering::Relaxed) as f64 * 1e-6
}

pub fn add_secs(t: f64) {
    SECS.with(|s| s.set(s.get() + t));
    MICROS.fetch_add((t * 1e6) as u64, std::sync::atomic::Ordering::Relaxed);
}

fn bump(i: usize, v: u64) {
    STATS[i].fetch_add(v, std::sync::atomic::Ordering::Relaxed);
}

/// `ENOMOTO_MIP_CUTCONF` の値 (付いていなければ `None`)。文字で形を選ぶ:
/// `m` 得た制約を双対証明と同じく CMIR で強めたカットも作る、`p` 証明のプールに入れない (`m` のカットだけ)、
/// `r` 伝播の矛盾 (行) からだけ、`d` 証明からだけ、`x` cut-based の衝突ができたらグラフ式の衝突は作らない、
/// `b` 一般整数列を含む制約は捨てる、`S` 証明のプールで双対証明と同じ分類にする。
pub fn mode() -> Option<&'static str> {
    env_str!("ENOMOTO_MIP_CUTCONF")
}

/// 診断用 `ENOMOTO_MIP_CUTCONF_CHECK` ([`analyze`])。
pub fn check_mode() -> bool {
    env_str!("ENOMOTO_MIP_CUTCONF_CHECK").is_some()
}

/// 衝突の制約の係数を持つ密な作業領域 (スレッドごとに使い回す)。
#[derive(Default)]
struct Acc {
    v: Vec<f64>,
    on: Vec<bool>,
    nz: Vec<usize>,
}

impl Acc {
    fn reset(&mut self, n: usize) {
        for &j in &self.nz {
            self.v[j] = 0.0;
            self.on[j] = false;
        }
        self.nz.clear();
        if self.v.len() < n {
            self.v.resize(n, 0.0);
            self.on.resize(n, false);
        }
    }

    fn add(&mut self, j: usize, a: f64) {
        if !self.on[j] {
            self.on[j] = true;
            self.nz.push(j);
        }
        self.v[j] += a;
    }

    /// 係数が 0 になった列を索引から外す。
    fn compact(&mut self) {
        let (v, on) = (&self.v, &mut self.on);
        self.nz.retain(|&j| {
            if v[j] == 0.0 {
                on[j] = false;
                false
            } else {
                true
            }
        });
    }
}

/// 境界の記録を位置ごとに引く表 (解析 1 回ごとに作る)。
struct Hist<'a> {
    dom: &'a Domain,
    last_lo: Vec<usize>,
    last_up: Vec<usize>,
    prev: Vec<usize>,
    glo: Vec<f64>,
    gup: Vec<f64>,
    /// 置き換えずに残した変更 (決定・外から来た変更) の (列, 上限か) → 位置。位置を遡っても、この境界は残る。
    kept: std::collections::HashMap<(usize, bool), usize>,
}

impl Hist<'_> {
    fn latest_before(&self, j: usize, upper: bool, before: usize) -> Option<usize> {
        let mut k = if upper { self.last_up[j] } else { self.last_lo[j] };
        while k != NONE && k >= before {
            k = self.prev[k];
        }
        if k == NONE { None } else { Some(k) }
    }

    /// (列, 側) の位置 `pos` の時点の変更: 残した変更があればそれ、なければ `pos` より前の最後の変更。
    fn change_at(&self, j: usize, upper: bool, pos: usize) -> Option<usize> {
        match self.kept.get(&(j, upper)) {
            Some(&k) => Some(k),
            None => self.latest_before(j, upper, pos),
        }
    }

    /// 位置 `pos` より前の変更 (と残した変更) を入れた下限 (大域的な境界で締める)。
    fn lo_at(&self, j: usize, pos: usize) -> f64 {
        match self.change_at(j, false, pos) {
            Some(k) => self.dom.trail_entry(k).2.max(self.glo[j]),
            None => self.glo[j],
        }
    }

    fn up_at(&self, j: usize, pos: usize) -> f64 {
        match self.change_at(j, true, pos) {
            Some(k) => self.dom.trail_entry(k).2.min(self.gup[j]),
            None => self.gup[j],
        }
    }

    /// 係数 `a` の列 `j` の最小活動量を、大域的な境界より上げている (位置 `pos` の時点の) 変更 (位置, 残した変更か)。
    fn rel_change(&self, j: usize, a: f64, pos: usize) -> Option<(usize, bool)> {
        let upper = a < 0.0;
        let kept = self.kept.get(&(j, upper)).copied();
        let k = kept.or_else(|| self.latest_before(j, upper, pos))?;
        let v = self.dom.trail_entry(k).2;
        let tighter = if upper { v < self.gup[j] } else { v > self.glo[j] };
        tighter.then_some((k, kept.is_some()))
    }

    /// `sum a_j x_j <= b` の、位置 `pos` より前の変更までを入れた境界での余裕 `b - 最小活動量` (無限なら +inf)。
    fn slack(&self, terms: impl Iterator<Item = (usize, f64)>, b: f64, pos: usize) -> f64 {
        let mut act = 0.0;
        for (j, a) in terms {
            if a == 0.0 {
                continue;
            }
            let v = if a > 0.0 { self.lo_at(j, pos) } else { self.up_at(j, pos) };
            if !v.is_finite() {
                return f64::INFINITY;
            }
            act += a * v;
        }
        b - act
    }
}

/// 理由の行 `R: sum r_j x_j <= e` (列 `p` の係数 `r_p != 0`) を `|r_p|` で割り、MIR で丸める (位置 `pos` の時点の境界に
/// 近い大域的な境界で補にする)。戻り値は `x` の空間の `R'` (列 `p` の係数は `±1`)。作れなければ `None`。
fn reduce(h: &Hist, p: &MipProblem, r: &[(usize, f64)], e: f64, col: usize, pos: usize) -> Option<(Vec<(usize, f64)>, f64)> {
    let rp = r.iter().find(|&&(j, _)| j == col)?.1;
    let delta = rp.abs();
    let integral = |v: f64| (v - v.round()).abs() <= 1e-9;
    // (列, α (割った後の補の変数の係数), 整数か, 補にした境界, 下限で補にしたか)
    let mut terms: Vec<(usize, f64, bool, f64, bool)> = Vec::with_capacity(r.len());
    let mut beta = e;
    for &(j, a) in r {
        if a == 0.0 {
            continue;
        }
        let (gl, gu) = (h.glo[j], h.gup[j]);
        let int = p.is_int[j] && (!gl.is_finite() || integral(gl)) && (!gu.is_finite() || integral(gu));
        let at_lower = if j == col {
            // 伝播した列: 係数が正 (上限を締めた) なら下限で、負なら上限で補にする (補の変数の係数は 1)
            a > 0.0
        } else if a > 0.0 {
            let v = h.lo_at(j, pos);
            if v <= gl {
                true
            } else if !gl.is_finite() {
                false
            } else if !gu.is_finite() {
                true
            } else if int {
                v - gl < gu - v
            } else {
                false
            }
        } else {
            let v = h.up_at(j, pos);
            if v >= gu {
                false
            } else if !gu.is_finite() {
                true
            } else if !gl.is_finite() {
                false
            } else if int {
                v - gl < gu - v
            } else {
                true
            }
        };
        let bnd = if at_lower { gl } else { gu };
        if !bnd.is_finite() {
            return None;
        }
        beta -= a * bnd;
        let alpha = if at_lower { a / delta } else { -a / delta };
        terms.push((j, alpha, int, bnd, at_lower));
    }
    let mut beta = beta / delta;
    if !beta.is_finite() {
        return None;
    }
    let mut f0 = beta - beta.floor();
    if f0 > 1.0 - 1e-9 {
        // 丸めの誤差: 右辺を上の整数に (緩める方向なので妥当)
        beta = beta.ceil();
        f0 = 0.0;
    }
    if f0 < 1e-9 {
        f0 = 0.0;
    }
    if 1.0 - f0 < 1e-4 {
        return None;
    }
    let mut rhs = beta.floor();
    let mut out: Vec<(usize, f64)> = Vec::with_capacity(terms.len());
    for (j, alpha, int, bnd, at_lower) in terms {
        let f = if int {
            let fl = alpha.floor();
            let fj = alpha - fl;
            fl + (fj - f0).max(0.0) / (1.0 - f0)
        } else if alpha < 0.0 {
            alpha / (1.0 - f0)
        } else {
            0.0
        };
        if f == 0.0 {
            continue;
        }
        // 補の変数 y = x - bnd (下限) なら f x <= .. + f bnd、z = bnd - x (上限) なら -f x <= .. - f bnd
        if at_lower {
            rhs += f * bnd;
            out.push((j, f));
        } else {
            rhs -= f * bnd;
            out.push((j, -f));
        }
    }
    if !rhs.is_finite() {
        return None;
    }
    Some((out, rhs))
}

/// cut-based の衝突解析。`start` から始め、境界の記録を遡って理由の行と線形結合する。得た制約 `sum coefs x <= rhs`
/// (今の境界で破れている、長さ `max_len` 以下) を返す。1 段も進まなければ `None`。
pub fn analyze(dom: &Domain, p: &MipProblem, start: &ConflictStart, max_len: usize, dbg: Option<&[f64]>) -> Option<(Vec<(usize, f64)>, f64)> {
    bump(st::CALLS, 1);
    let end = dom.stack_len();
    let mut dec: Vec<usize> = Vec::new();
    let n = p.n;
    let mut last_lo = vec![NONE; n];
    let mut last_up = vec![NONE; n];
    let mut prev = vec![NONE; end];
    for k in 0..end {
        let (j, upper, _, reason) = dom.trail_entry(k);
        if reason == -1 {
            dec.push(k);
        }
        let l = if upper { &mut last_up[j] } else { &mut last_lo[j] };
        prev[k] = *l;
        *l = k;
    }
    if dec.is_empty() {
        bump(st::NO_START, 1);
        return None;
    }
    // 診断用 `ENOMOTO_MIP_CUTCONF_CHECK`: どの実行可能解にも妥当なものだけを使う (問題の境界、元の行、伝播の矛盾からの始まり)。
    // 得た制約はデバッグ解 (どの実行可能解でもよい) を切ってはならないので、暫定解によらず確かめられる
    let check = check_mode();
    if check && matches!(start, ConflictStart::Proof(..)) {
        return None;
    }
    let (glo, gup): (Vec<f64>, Vec<f64>) = if check {
        (p.col_lo.clone(), p.col_up.clone())
    } else {
        let (rlo, rup) = dom.root_bounds();
        ((0..n).map(|j| rlo[j].max(dom.global_lo[j])).collect(), (0..n).map(|j| rup[j].min(dom.global_up[j])).collect())
    };
    let mut h = Hist { dom, last_lo, last_up, prev, glo, gup, kept: std::collections::HashMap::new() };
    ACC.with(|acc| {
        let mut acc = acc.borrow_mut();
        acc.reset(n);
        let mut b = match *start {
            ConflictStart::Src(ConflictSrc::Row(i, true)) => {
                for &(j, a) in &p.rows[i] {
                    acc.add(j, a);
                }
                p.row_up[i]
            }
            ConflictStart::Src(ConflictSrc::Row(i, false)) => {
                for &(j, a) in &p.rows[i] {
                    acc.add(j, -a);
                }
                -p.row_lo[i]
            }
            ConflictStart::Src(ConflictSrc::Col(_)) => f64::NAN,
            ConflictStart::Proof(c, r) => {
                for &(j, a) in c {
                    acc.add(j, a);
                }
                r
            }
        };
        let res = run(&mut h, p, &mut acc, &mut b, &dec, end, max_len, dbg);
        acc.reset(n);
        res
    })
}

fn normalize(acc: &mut Acc, b: &mut f64) -> bool {
    let m = acc.nz.iter().map(|&j| acc.v[j].abs()).fold(0.0, f64::max);
    if !(m > 0.0) || !m.is_finite() {
        return false;
    }
    let s = 1.0 / m;
    for &j in &acc.nz {
        acc.v[j] *= s;
    }
    *b *= s;
    true
}

/// 診断用 (デバッグ解): `sum terms x <= rhs` がデバッグ解 `x` で破れる量 (正なら切る)。
fn dbg_viol(x: &[f64], terms: impl Iterator<Item = (usize, f64)>, rhs: f64) -> f64 {
    let a: f64 = terms.map(|(j, v)| v * x[j]).sum();
    a - rhs - 1e-6 * (1.0 + rhs.abs())
}

#[allow(clippy::too_many_arguments)]
fn run(h: &mut Hist, p: &MipProblem, acc: &mut Acc, b: &mut f64, dec: &[usize], end: usize, max_len: usize, dbg: Option<&[f64]>) -> Option<(Vec<(usize, f64)>, f64)> {
    if !b.is_finite() || !normalize(acc, b) {
        bump(st::NO_START, 1);
        return None;
    }
    // 診断用: 始めの制約がデバッグ解を切るなら知らせて止める (始めの証明が誤り)。以後は段ごとに確かめる
    let mut dbg = dbg;
    if let Some(x) = dbg {
        let v = dbg_viol(x, acc.nz.iter().map(|&j| (j, acc.v[j])), *b);
        if v > 0.0 {
            eprintln!("MIP_DEBUG_SOL: cutconf start constraint already cuts off the debug solution (by {v:.3e}, len {})", acc.nz.len());
            dbg = None;
        }
    }
    let eps = 1e-6;
    let slack_now = |h: &Hist, acc: &Acc, b: f64, pos: usize| h.slack(acc.nz.iter().map(|&j| (j, acc.v[j])), b, pos);
    if !(slack_now(h, acc, *b, end) < -eps) {
        bump(st::NO_START, 1);
        return None;
    }
    let max_steps = tunable!("ENOMOTO_T_MIP_CUTCONF_STEPS", 200usize, usize);
    let budget = tunable!("ENOMOTO_T_MIP_CUTCONF_WORK", 200_000usize, usize);
    let cap = (4 * max_len + 200).min(tunable!("ENOMOTO_T_MIP_CUTCONF_CAP", 5000usize, usize));
    let mut work = acc.nz.len();
    let mut pos = end;
    let mut resolved = 0usize;
    let mut rel: Vec<(usize, bool)> = Vec::new();
    let mut saved: Vec<(usize, f64)> = Vec::new();
    loop {
        rel.clear();
        for &j in &acc.nz {
            let a = acc.v[j];
            if a != 0.0 {
                if let Some(k) = h.rel_change(j, a, pos) {
                    rel.push(k);
                }
            }
        }
        work += acc.nz.len();
        let Some(&(kmax, _)) = rel.iter().max() else {
            // 大域的な境界だけで破れている (根で矛盾、打ち切り値で刈れる): 理由のない段と同じに止める
            bump(st::STOP_DEC, 1);
            break;
        };
        // 衝突の段 (最小活動量を上げている変更の最も深い段) の変更が 1 つなら 1-UIP
        let lev = dec.partition_point(|&d| d <= kmax);
        let lstart = if lev == 0 { 0 } else { dec[lev - 1] };
        let cnt = rel.iter().filter(|&&(k, _)| k >= lstart).count();
        if cnt <= 1 {
            bump(st::STOP_UIP, 1);
            break;
        }
        // その段の、まだ残していない最も新しい変更
        let Some(&(kstar, _)) = rel.iter().filter(|&&(k, kept)| !kept && k >= lstart).max() else {
            bump(st::STOP_DEC, 1);
            break;
        };
        let (col, upper, _, reason) = h.dom.trail_entry(kstar);
        // 外から来た変更でも、伝播した行を覚えていれば (双対証明・目的関数・カットのプール) その行で置き換える
        let ext = if reason < -1 && !check_mode() { ext_row(h.dom, kstar).filter(|r| r.0.iter().any(|&(j, a)| j == col && (a > 0.0) == upper)) } else { None };
        if reason < 0 && ext.is_none() {
            // 理由のない変更 (決定・外から来た変更) は残して、同じ段の次の変更へ
            h.kept.insert((col, upper), kstar);
            continue;
        }
        if resolved >= max_steps || work > budget || acc.nz.len() > cap {
            bump(st::STOP_BUDGET, 1);
            break;
        }
        let (rvec, e): (Vec<(usize, f64)>, f64) = if let Some(x) = &ext {
            bump(st::EXT, 1);
            (x.0.clone(), x.1)
        } else {
            let r = reason as usize;
            let row = &p.rows[r];
            let Some(rp) = row.iter().find(|&&(j, _)| j == col).map(|&(_, a)| a) else {
                bump(st::FAIL_STEP, 1);
                break;
            };
            // R を `<=` の形に向ける: 上限を締めたなら R での係数が正、下限なら負になる側
            let sign = if upper == (rp > 0.0) { 1.0 } else { -1.0 };
            (row.iter().map(|&(j, a)| (j, sign * a)).collect(), if sign > 0.0 { p.row_up[r] } else { -p.row_lo[r] })
        };
        if !e.is_finite() {
            bump(st::FAIL_STEP, 1);
            break;
        }
        work += rvec.len();
        let cp = acc.v[col];
        let mut ok = false;
        let mut used_reduce = false;
        for attempt in 0..2 {
            let (rr, ee) = if attempt == 0 {
                (std::borrow::Cow::Borrowed(&rvec[..]), e)
            } else {
                if !p.is_int[col] {
                    break;
                }
                match reduce(&*h, p, &rvec, e, col, kstar) {
                    Some((v, s)) => (std::borrow::Cow::Owned(v), s),
                    None => break,
                }
            };
            if let Some(x) = dbg {
                let v = dbg_viol(x, rr.iter().copied(), ee);
                if v > 0.0 {
                    let (gl, gu) = (h.glo[col], h.gup[col]);
                    eprintln!(
                        "MIP_DEBUG_SOL: cutconf reason ({}, reason {reason}, ext {}) cuts off the debug solution by {v:.3e}: col x{col} int {} global [{gl}, {gu}] upper {upper} len {}",
                        if attempt == 0 { "plain" } else { "reduced" },
                        ext.is_some(),
                        p.is_int[col],
                        rr.len()
                    );
                    if attempt == 1 {
                        let terms: Vec<String> = rvec.iter().map(|&(j, a)| format!("{a}*x{j}[{}..{}|{}|x={}]", h.glo[j], h.gup[j], if a > 0.0 { h.lo_at(j, kstar) } else { h.up_at(j, kstar) }, x[j])).collect();
                        eprintln!("MIP_DEBUG_SOL:   R = {} <= {e}", terms.join(" + "));
                        eprintln!("MIP_DEBUG_SOL:   R' = {:?} <= {ee}", rr);
                    }
                    dbg = None;
                }
            }
            let Some(rho) = rr.iter().find(|&&(j, _)| j == col).map(|&(_, a)| a) else { continue };
            let lambda = -cp / rho;
            if !(lambda > 0.0) || !lambda.is_finite() {
                continue;
            }
            saved.clear();
            for &(j, a) in rr.iter() {
                saved.push((j, if acc.on[j] { acc.v[j] } else { 0.0 }));
                acc.add(j, lambda * a);
            }
            acc.v[col] = 0.0;
            let nb = *b + lambda * ee;
            let s = slack_now(h, acc, nb, kstar);
            let scale = 1.0 + lambda;
            if s < -eps * scale && nb.is_finite() {
                *b = nb;
                ok = true;
                used_reduce = attempt == 1;
                break;
            }
            // 戻す
            acc.v[col] = cp;
            for &(j, old) in saved.iter().rev() {
                acc.v[j] = old;
            }
            if acc.v[col] == 0.0 {
                acc.v[col] = cp;
            }
        }
        if !ok {
            bump(st::FAIL_STEP, 1);
            if !p.is_int[col] {
                bump(st::FAIL_CONT, 1);
            } else if !(h.glo[col] >= 0.0 && h.gup[col] <= 1.0) {
                bump(st::FAIL_GENINT, 1);
            }
            break;
        }
        if let Some(x) = dbg {
            let v = dbg_viol(x, acc.nz.iter().map(|&j| (j, acc.v[j])), *b);
            if v > 0.0 {
                eprintln!("MIP_DEBUG_SOL: cutconf combination at step {resolved} cuts off the debug solution by {v:.3e} (reduced {used_reduce}) though the parts do not");
                dbg = None;
            }
        }
        bump(if used_reduce { st::REDUCED } else { st::PLAIN }, 1);
        resolved += 1;
        pos = kstar;
        acc.compact();
        // 小さすぎる係数は大域的な境界で消す (弱める方向)
        let mut drop_b = 0.0;
        let mut kill: Vec<usize> = Vec::new();
        for &j in &acc.nz {
            let a = acc.v[j];
            if a.abs() < 1e-9 {
                let (gl, gu) = (h.glo[j], h.gup[j]);
                let m = (a * gl).min(a * gu);
                if m.is_finite() {
                    drop_b += m;
                    kill.push(j);
                }
            }
        }
        for j in kill {
            acc.v[j] = 0.0;
        }
        *b -= drop_b;
        acc.compact();
        if !normalize(acc, b) {
            return None;
        }
    }
    if resolved == 0 {
        bump(st::NO_STEP, 1);
        return None;
    }
    // 大域的に固定された列を右辺に移す
    let mut coefs: Vec<(usize, f64)> = Vec::with_capacity(acc.nz.len());
    let mut rhs = *b;
    for &j in &acc.nz {
        let a = acc.v[j];
        if a == 0.0 {
            continue;
        }
        if h.glo[j] == h.gup[j] && h.glo[j].is_finite() {
            rhs -= a * h.glo[j];
        } else {
            coefs.push((j, a));
        }
    }
    let s = h.slack(coefs.iter().copied(), rhs, end);
    if coefs.is_empty() || !(s < -eps) || coefs.len() > max_len || !rhs.is_finite() {
        bump(st::DROP, 1);
        return None;
    }
    let has_genint = coefs.iter().any(|&(j, _)| p.is_int[j] && !(h.glo[j] >= 0.0 && h.gup[j] <= 1.0));
    if has_genint && mode().unwrap_or("").contains('b') {
        // `b`: 一般整数列を含む制約は捨てる (論文の対象は 0-1 混合。一般整数の行は弱い締め付けを多く起こす)
        bump(st::DROP, 1);
        return None;
    }
    bump(st::OUT, 1);
    bump(st::OUT_LEN, coefs.len() as u64);
    if coefs.iter().any(|&(j, _)| !(p.is_int[j] && h.glo[j] >= 0.0 && h.gup[j] <= 1.0)) {
        bump(st::OUT_NONBIN, 1);
    }
    Some((coefs, rhs))
}

/// 伝播した行 `sum a_j x_j <= rhs` (双対証明・目的関数・カットのプール。元の行でないもの)。
pub type ExtRow = std::rc::Rc<(Vec<(usize, f64)>, f64)>;

thread_local! {
    /// 記録の位置ごとの、外から来た変更の理由の行と (列, 上限か, 値) (位置が使い回されたら値で見分ける)。
    static EXT: std::cell::RefCell<Vec<Option<(ExtRow, usize, bool, f64)>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// 記録の位置 `pos0..` の外から来た変更の理由として行を覚える (`row` は最初に要るときに `make` で作る)。
/// 試作が有効なときだけ呼ぶ。
pub fn note_range(dom: &Domain, pos0: usize, row: &mut Option<ExtRow>, make: impl FnOnce() -> (Vec<(usize, f64)>, f64)) {
    let end = dom.stack_len();
    if pos0 >= end {
        return;
    }
    let mut make = Some(make);
    EXT.with(|e| {
        let mut e = e.borrow_mut();
        if e.len() < end {
            e.resize(end, None);
        }
        for k in pos0..end {
            let (j, upper, v, reason) = dom.trail_entry(k);
            if reason >= -1 {
                continue;
            }
            if row.is_none() {
                *row = Some(std::rc::Rc::new((make.take().unwrap())()));
            }
            e[k] = Some((row.clone().unwrap(), j, upper, v));
        }
    });
}

/// 記録の位置 `k` の外から来た変更の理由の行 (覚えていて、変更が同じなら)。
fn ext_row(dom: &Domain, k: usize) -> Option<ExtRow> {
    let (j, upper, v, reason) = dom.trail_entry(k);
    if reason >= -1 {
        return None;
    }
    EXT.with(|e| match e.borrow().get(k) {
        Some(Some((r, j2, u2, v2))) if *j2 == j && *u2 == upper && *v2 == v => Some(r.clone()),
        _ => None,
    })
}

thread_local! {
    /// このスレッドで証明のプールに入れた cutconf の行の指紋 (証明のプールを詰め直すと番号が変わるので、中身で見分ける)。
    static MINE: std::cell::RefCell<std::collections::HashSet<(usize, u64, usize, u64)>> = std::cell::RefCell::new(std::collections::HashSet::new());
}

fn fingerprint(coefs: &[(usize, f64)], rhs: f64) -> (usize, u64, usize, u64) {
    let (j, a) = coefs.first().copied().unwrap_or((0, 0.0));
    (coefs.len(), rhs.to_bits(), j, a.to_bits())
}

/// 証明のプールに入れた cutconf の行を覚える (係数は入れた後の並びで)。
pub fn remember(coefs: &[(usize, f64)], rhs: f64) {
    MINE.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() > 200_000 {
            m.clear();
        }
        m.insert(fingerprint(coefs, rhs));
    });
}

/// 証明のプールの行が使われた (`prune` ならノードを刈った、そうでなければ境界を締めた): cutconf の行なら数える。
pub fn note_use(coefs: &[(usize, f64)], rhs: f64, prune: bool) {
    if MINE.with(|m| m.borrow().contains(&fingerprint(coefs, rhs))) {
        bump(if prune { st::USE_PRUNE } else { st::USE_TIGHTEN }, 1);
    }
}
