//! 試作 `ENOMOTO_LP_FOLD` / `ENOMOTO_MIP_FOLD` (既定では使わない。analysis/drcr_20261110.md): 色の細分化による LP の次元削減
//! (DRCR、Grohe・Kersting・Mladenov・Selman 2014) と、その反転への拡張 (R-DRCR、van der Hulst 2026「Folding Mixed-Integer Linear
//! Programs and Reflection Symmetries」)。
//!
//! 問題 `min c x, lo_r ≤ a_r x ≤ up_r, l ≤ x ≤ u` を、行ごとにスラック列 `s_r` (係数 −1、境界 `[lo_r, up_r]`、費用 0) を足した
//! 等式形 `A x − s = 0` として扱う (論文の第 2.2 節の命題 4: スラックを足しても等分割のまま。スラックが構造の列とまとまることもある)。
//!
//! - 反転なし (DRCR): 行と列の等分割 (P, Q) (どの組 (P, Q) でも、P の各行の Q の係数の和が等しく、Q の各列の P の係数の和も
//!   等しい。Q の中で費用・境界が等しい) を色の細分化で求める。列の組 Q ごとに平均の変数 z_Q (x_w = z_Q) を 1 つ、行の組 P ごとに
//!   代表の行 Σ_Q R_PQ z_Q = 0 (R_PQ は P の 1 行の Q の係数の和) を 1 つ置いた問題は、元と最適値が同じで、z の最適解を配った
//!   x は元の最適解 (分数の自己同型、論文の命題 3。ここでは和ではなく平均の変数にしたので境界が元のまま)。
//! - 反転あり (R-DRCR): 列を境界の中央 δ^C に移して (論文の第 3.5 節)、x' = x − δ を正負 x'^+ − x'^− に分けた分割形 (Definition 6、
//!   行も ± の写しを持つ) の対称な等分割を、元の行列の上で「符号つきの色の細分化」として求める (論文の第 3.7 節の実装の方針と同じ):
//!   組ごとに単極 (列の符号 λ_w を持つ) か双極 (x'^+ と x'^− が同じ組) かを持ち、和を符号つきで (λ_w・γ_v をかけて) 取る。
//!   双極の組の和はいつも 0 なので細分化に効かない (系 10)。双極の組の行・列が非零の和を持てば単極に分かれ、符号は和が正になる
//!   向きに決める。単極の列の組 Q は x_w = δ_w + λ_w z_Q、双極の列は x_w = δ_w (中央に固定)、双極の行は消える (Theorem 19)。
//!   δ は組ごとに λ_w ν_Q だけずらしてよい (Theorem 20) ので、多くの列で δ_w = 0 になる ν を選ぶ (第 3.7 節の疎にする工夫)。
//! - 整数列 ([`FoldIn::int_key`]): `None` なら整数列はどれも 1 列だけの組にする (論文の Lemma 28。和が等しい行は整数列の係数が
//!   同じになり、畳んだ問題の整数解が元の整数解に戻る)。`Some` なら同じ鍵の整数列だけ同じ組にしてよく、2 列以上の整数の組は
//!   和の変数 y_Q = Σ λ_w x'_w (整数) にする (Theorem 26。元に戻すには整数の繊維の LP を解く。src/mip/fold.rs)。整数列は双極にしない
//!   (中央が整数とは限らない)。
//!
//! 浮動小数の和は相対 2^-32 ほどに丸めた鍵で比べ、細分化が止まった後に、組ごとの和・右辺・境界・費用が許容誤差の内で
//! 等しいかを全部の行と列で確かめる ([`Fold::build`] の検査)。外れたら畳まない。

use std::collections::HashMap;

/// 畳む元の問題 (行は (列, 係数) の疎な並び、境界は ±inf 可)。
pub struct FoldIn<'a> {
    pub col_lo: &'a [f64],
    pub col_up: &'a [f64],
    pub cost: &'a [f64],
    pub is_int: &'a [bool],
    pub rows: &'a [Vec<(usize, f64)>],
    pub row_lo: &'a [f64],
    pub row_up: &'a [f64],
    /// 整数列の初めの鍵 (同じ鍵の整数列だけ同じ組にしてよい)。`None` なら整数列はどれも 1 列だけの組。
    pub int_key: Option<&'a [u64]>,
    /// 1 列だけの組にする列 (`None` ならなし)。
    pub single: Option<&'a [bool]>,
}

/// 畳む設定。
#[derive(Clone, Copy, Debug)]
pub struct FoldOpts {
    /// 反転も使う (R-DRCR)。
    pub reflect: bool,
    /// 細分化の回 (行と列を 1 回ずつ) の上限。超えたら畳まない。
    pub max_rounds: usize,
    /// 畳んだ問題の (列 + 行) が元の (列 + 行) のこの割合以下でなければ畳まない。
    pub max_ratio: f64,
    /// 細分化の時間の上限 (秒)。
    pub max_secs: f64,
    /// 素朴な細分化 (毎回全部の行・列の鍵を作り直す、比べる用。`ENOMOTO_T_FOLD_NAIVE`)。既定は [`refine_fast`]。
    pub naive: bool,
}

impl Default for FoldOpts {
    fn default() -> Self {
        FoldOpts { reflect: false, max_rounds: 1000, max_ratio: 0.95, max_secs: 30.0, naive: false }
    }
}

/// 畳んだ問題 (行は範囲の形 `row_lo ≤ Σ ≤ row_up`)。目的は `cost · y + offset` (元の `c x` と同じ値)。
#[derive(Clone, Debug)]
pub struct FoldLp {
    pub col_lo: Vec<f64>,
    pub col_up: Vec<f64>,
    pub cost: Vec<f64>,
    pub is_int: Vec<bool>,
    pub rows: Vec<Vec<(usize, f64)>>,
    pub row_lo: Vec<f64>,
    pub row_up: Vec<f64>,
    pub offset: f64,
}

/// 数 (記録用)。
#[derive(Clone, Debug, Default)]
pub struct FoldStats {
    pub n: usize,
    pub m: usize,
    pub n_red: usize,
    pub m_red: usize,
    pub rounds: usize,
    pub col_parts: usize,
    pub row_parts: usize,
    pub bip_cols: usize,
    pub bip_rows: usize,
    /// λ = −1 で畳んだ構造の列の数。
    pub neg_cols: usize,
    /// 2 列以上の組に入った構造の列の数。
    pub grouped_cols: usize,
    /// 2 列以上の整数の組の数とその列の数。
    pub int_groups: usize,
    pub int_grouped_cols: usize,
    /// 吸収したスラックの組の数。
    pub absorbed: usize,
    pub secs: f64,
}

/// 畳み方 (元の構造の列から畳んだ問題の列への写像など)。
#[derive(Clone, Debug)]
pub struct Fold {
    pub red: FoldLp,
    /// 構造の列 w の畳んだ問題の列 (`u32::MAX` は双極で δ_w に固定)。
    pub col_red: Vec<u32>,
    /// 符号 λ_w。
    pub col_sign: Vec<f64>,
    /// ずらし δ_w。
    pub col_off: Vec<f64>,
    /// 畳んだ問題の列の倍率 (x_w = δ_w + λ_w y_q / scale_q。平均の変数は 1、和の変数は組の大きさ)。
    pub red_scale: Vec<f64>,
    /// 畳んだ問題の列 q に入る構造の列 (2 列以上の整数の組の繊維の LP に使う)。
    pub red_members: Vec<Vec<u32>>,
    /// 元の行 v の畳んだ問題の行 (`u32::MAX` は双極で消えた行) と符号 γ_v、畳んだ行の組の大きさ。
    pub row_red: Vec<u32>,
    pub row_sign: Vec<f64>,
    pub red_row_size: Vec<usize>,
    pub stats: FoldStats,
}

/// 相対 2^-32 ほどに丸めた鍵 (−v の鍵は v の鍵の符号を変えたもの)。ごく小さい値は 0。
fn qk(v: f64) -> i64 {
    if v.abs() < 1e-12 {
        return 0;
    }
    if v.is_infinite() {
        return if v > 0.0 { i64::MAX } else { i64::MIN };
    }
    let b = v.abs().to_bits();
    let r = ((b + (1u64 << 19)) >> 20) as i64;
    if v < 0.0 {
        -r
    } else {
        r
    }
}

fn center(lo: f64, up: f64) -> f64 {
    match (lo.is_finite(), up.is_finite()) {
        (true, true) => 0.5 * (lo + up),
        (false, false) => 0.0,
        (true, false) => lo,
        (false, true) => up,
    }
}

fn close(a: f64, b: f64, scale: f64) -> bool {
    if a == b {
        return true;
    }
    if !a.is_finite() || !b.is_finite() {
        return false;
    }
    (a - b).abs() <= 1e-9 * (1.0 + scale + a.abs().max(b.abs()))
}

/// 片側 (行か列) の分割の状態 ([`refine_fast`])。
struct Side {
    cls: Vec<u32>,
    sg: Vec<f64>,
    bip: Vec<bool>,
    members: Vec<Vec<u32>>,
    pos: Vec<u32>,
    queued: Vec<bool>,
}

impl Side {
    fn new(cls: Vec<u32>, sg: Vec<f64>, bip: Vec<bool>) -> Side {
        let mut members = vec![Vec::new(); bip.len()];
        let mut pos = vec![0u32; cls.len()];
        for (e, &c) in cls.iter().enumerate() {
            pos[e] = members[c as usize].len() as u32;
            members[c as usize].push(e as u32);
        }
        let queued = vec![false; bip.len()];
        Side { cls, sg, bip, members, pos, queued }
    }

    /// 要素 e を組 y に移す。
    fn mv(&mut self, e: u32, y: u32) {
        let x = self.cls[e as usize] as usize;
        let i = self.pos[e as usize] as usize;
        let last = *self.members[x].last().unwrap();
        self.members[x][i] = last;
        self.pos[last as usize] = i as u32;
        self.members[x].pop();
        self.pos[e as usize] = self.members[y as usize].len() as u32;
        self.members[y as usize].push(e);
        self.cls[e as usize] = y;
    }
}

/// 速い色の細分化 (Hopcroft・Paige-Tarjan の形。Grohe・Kersting・Mladenov・Selman 2014 の第 3 節と同じく和で割る): 割る側の組を
/// 待ち行列に入れ、取り出した組 C の要素から隣の要素へ符号つきの和 Σ_{w∈C} a λ_w を足し、触れた要素の組をその和 (単極なら γ をかけた値、
/// 双極なら絶対値で、0 でない片は単極になり符号は和の符号) で割る。単極の組が割れたら、その組が待ち行列になければ一番大きい片を除いて
/// 入れる (和は線形なので、一番大きい片の和は親の和と他の片の和の差で決まる)。双極の組から分かれた単極の片は全部入れる (双極の組の
/// 和はいつも 0 で、片の和を差で決められない。論文の第 3.7 節の注意)。双極の組は割る側にしない (和が 0、系 10)。
/// 結果の組の番号は 0.. で詰まり、空の組はない。手間は O((n + nnz) log n) (双極の組から分かれる分は要素ごとに 1 回)。
/// 戻り値は処理した割る側の組の数。時間の上限か、縮みの下限 (素朴な形と同じ) が上限を超えたら `Err`。
#[allow(clippy::too_many_arguments)]
fn refine_fast(
    n: usize,
    m: usize,
    rows: &[Vec<(usize, f64)>],
    cols: &[Vec<(u32, f64)>],
    row_cls: &mut Vec<u32>,
    row_sg: &mut Vec<f64>,
    rcls_bip: &mut Vec<bool>,
    col_cls: &mut Vec<u32>,
    col_sg: &mut Vec<f64>,
    ccls_bip: &mut Vec<bool>,
    opts: FoldOpts,
    t0: std::time::Instant,
) -> Result<usize, ()> {
    let ne = n + m;
    let mut sides = [
        Side::new(std::mem::take(row_cls), std::mem::take(row_sg), std::mem::take(rcls_bip)),
        Side::new(std::mem::take(col_cls), std::mem::take(col_sg), std::mem::take(ccls_bip)),
    ];
    // 待ち行列 (側 0 = 行、1 = 列、組)
    let mut queue: std::collections::VecDeque<(usize, u32)> = std::collections::VecDeque::new();
    for sd in 0..2 {
        for c in 0..sides[sd].bip.len() {
            if !sides[sd].bip[c] {
                sides[sd].queued[c] = true;
                queue.push_back((sd, c as u32));
            }
        }
    }
    let mut acc = vec![0.0f64; ne.max(m)];
    let mut mark = vec![false; ne.max(m)];
    let mut touched: Vec<u32> = Vec::new();
    let mut processed = 0usize;
    let mut work = 0usize;
    let mut work_check = 0usize;
    let mut splitter: Vec<u32> = Vec::new();
    let mut keyed: Vec<(u32, i64, u32)> = Vec::new(); // (組, 鍵, 要素)
    while let Some((sd, c)) = queue.pop_front() {
        sides[sd].queued[c as usize] = false;
        if sides[sd].bip[c as usize] {
            continue;
        }
        processed += 1;
        let td = 1 - sd;
        // 割る側の組の要素から隣へ和を足す
        splitter.clear();
        splitter.extend_from_slice(&sides[sd].members[c as usize]);
        for &w in &splitter {
            let sw = sides[sd].sg[w as usize];
            if sd == 1 {
                // 列 w → 行
                for &(v, a) in &cols[w as usize] {
                    let v = v as usize;
                    if !mark[v] {
                        mark[v] = true;
                        touched.push(v as u32);
                    }
                    acc[v] += a * sw;
                }
                work += cols[w as usize].len();
            } else {
                // 行 w → 列 (構造の列とスラック)
                let v = w as usize;
                for &(e, a) in rows[v].iter().chain(std::iter::once(&(n + v, -1.0))) {
                    if !mark[e] {
                        mark[e] = true;
                        touched.push(e as u32);
                    }
                    acc[e] += a * sw;
                }
                work += rows[v].len() + 1;
            }
        }
        // 触れた要素を組ごとに鍵で並べる
        keyed.clear();
        {
            let t = &sides[td];
            for &e in &touched {
                let x = t.cls[e as usize];
                if t.bip[x as usize] {
                    let k = qk(acc[e as usize].abs());
                    if k != 0 {
                        keyed.push((x, k, e));
                    }
                } else {
                    let k = qk(t.sg[e as usize] * acc[e as usize]);
                    if k != 0 {
                        keyed.push((x, k, e));
                    }
                }
            }
        }
        keyed.sort_unstable();
        let mut i = 0;
        while i < keyed.len() {
            let x = keyed[i].0;
            let mut j = i;
            while j < keyed.len() && keyed[j].0 == x {
                j += 1;
            }
            // keyed[i..j] は組 x の触れた要素 (鍵が 0 でないもの)、鍵の順
            let t = &mut sides[td];
            let size = t.members[x as usize].len();
            let n0 = size - (j - i);
            let bipx = t.bip[x as usize];
            // 鍵ごとの片
            let mut groups: Vec<(usize, usize)> = Vec::new();
            let mut a = i;
            while a < j {
                let mut b = a;
                while b < j && keyed[b].1 == keyed[a].1 {
                    b += 1;
                }
                groups.push((a, b));
                a = b;
            }
            if !bipx && n0 == 0 && groups.len() == 1 {
                i = j;
                continue;
            }
            // 残る組: 鍵 0 の要素があればそれが x に残る、なければ最初の片が x に残る
            let first_moved = if n0 > 0 { 0 } else { 1 };
            let mut pieces: Vec<(u32, usize)> = Vec::new(); // (組, 大きさ)
            if n0 > 0 {
                pieces.push((x, n0));
            } else {
                let (ga, gb) = groups[0];
                if bipx {
                    // 組 x が丸ごと単極になる
                    t.bip[x as usize] = false;
                    for k in ga..gb {
                        let e = keyed[k].2 as usize;
                        t.sg[e] = if acc[e] < 0.0 { -1.0 } else { 1.0 };
                    }
                }
                pieces.push((x, gb - ga));
            }
            for &(ga, gb) in &groups[first_moved..] {
                let y = t.bip.len() as u32;
                t.bip.push(false);
                t.members.push(Vec::new());
                t.queued.push(false);
                for k in ga..gb {
                    let e = keyed[k].2;
                    if bipx {
                        t.sg[e as usize] = if acc[e as usize] < 0.0 { -1.0 } else { 1.0 };
                    }
                    t.mv(e, y);
                }
                pieces.push((y, gb - ga));
            }
            // 待ち行列に入れる
            if bipx {
                for &(y, _) in &pieces {
                    if !t.bip[y as usize] && !t.queued[y as usize] {
                        t.queued[y as usize] = true;
                        queue.push_back((td, y));
                    }
                }
            } else {
                let skip = if t.queued[x as usize] { u32::MAX } else { pieces.iter().max_by_key(|p| p.1).unwrap().0 };
                for &(y, _) in &pieces {
                    if y != skip && !t.queued[y as usize] {
                        t.queued[y as usize] = true;
                        queue.push_back((td, y));
                    }
                }
            }
            i = j;
        }
        for &e in &touched {
            acc[e as usize] = 0.0;
            mark[e as usize] = false;
        }
        touched.clear();
        // 時間の上限と、縮みの下限 (働いた量が n + m を越えるごと、確かめる手間を働いた量で抑える)
        if work - work_check >= ne + m {
            work_check = work;
            if t0.elapsed().as_secs_f64() > opts.max_secs {
                LAST_FAIL.with(|c| c.set(line!()));
                return Err(());
            }
            if opts.max_ratio < 1.0 {
                let lb = sides[0].bip.iter().filter(|&&b| !b).count()
                    + (0..sides[1].bip.len()).filter(|&c| !sides[1].bip[c] && sides[1].members[c].iter().any(|&e| (e as usize) < n)).count();
                if lb as f64 > opts.max_ratio * (n + m) as f64 {
                    LAST_RATIO.with(|c| c.set(lb as f64 / (n + m) as f64));
                    LAST_FAIL.with(|c| c.set(line!()));
                    return Err(());
                }
            }
        }
    }
    let [r, c] = sides;
    *row_cls = r.cls;
    *row_sg = r.sg;
    *rcls_bip = r.bip;
    *col_cls = c.cls;
    *col_sg = c.sg;
    *ccls_bip = c.bip;
    Ok(processed)
}

thread_local! {
    /// 最後に畳まなかった理由 (lp_fold.rs の行番号) と、縮みが足りなかったときの割合 (診断用)。
    pub static LAST_FAIL: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    pub static LAST_RATIO: std::cell::Cell<f64> = const { std::cell::Cell::new(1.0) };
}

macro_rules! nope {
    () => {{
        LAST_FAIL.with(|c| c.set(line!()));
        return None;
    }};
}

#[derive(Hash, PartialEq, Eq)]
struct ColKey {
    kind: u8,
    c: i64,
    lo: i64,
    up: i64,
    int: bool,
    extra: u64,
}

impl Fold {
    /// 等分割を求めて畳んだ問題を作る。畳めない (縮まない・細分化が止まらない・検査に外れた) なら `None`。
    pub fn build(p: &FoldIn, opts: FoldOpts) -> Option<Fold> {
        let t0 = std::time::Instant::now();
        let n = p.col_lo.len();
        let m = p.rows.len();
        let ne = n + m; // 拡張した列 (構造 + スラック)
        if n == 0 || m == 0 {
            nope!();
        }
        // 拡張した列の性質
        let elo = |e: usize| if e < n { p.col_lo[e] } else { p.row_lo[e - n] };
        let eup = |e: usize| if e < n { p.col_up[e] } else { p.row_up[e - n] };
        let ecost = |e: usize| if e < n { p.cost[e] } else { 0.0 };
        let eint = |e: usize| e < n && p.is_int[e];
        // 列方向 (拡張した列 → (行, 係数))
        let mut cols: Vec<Vec<(u32, f64)>> = vec![Vec::new(); ne];
        for (v, r) in p.rows.iter().enumerate() {
            for &(w, a) in r {
                if a != 0.0 {
                    cols[w].push((v as u32, a));
                }
            }
            cols[n + v].push((v as u32, -1.0));
        }
        let row_ext = |v: usize| p.rows[v].iter().map(|&(w, a)| (w, a)).chain(std::iter::once((n + v, -1.0)));
        // 中央 δ^C (反転なしなら 0)
        let dc: Vec<f64> = (0..ne).map(|e| if opts.reflect { center(elo(e), eup(e)) } else { 0.0 }).collect();
        // 初めの列の組
        let mut col_cls = vec![0u32; ne];
        let mut col_sg = vec![1.0f64; ne];
        let mut ccls_bip: Vec<bool> = Vec::new();
        {
            let mut ids: HashMap<ColKey, u32> = HashMap::new();
            for e in 0..ne {
                let int = eint(e);
                let single = e < n && p.single.is_some_and(|s| s[e]);
                let extra = if single || (int && p.int_key.is_none()) {
                    (1u64 << 63) | e as u64
                } else if int {
                    p.int_key.unwrap()[e] & !(1u64 << 63)
                } else {
                    0
                };
                let (lo, up, c) = (elo(e) - dc[e], eup(e) - dc[e], ecost(e));
                let a = (qk(c), qk(lo), qk(up));
                let (key, sg, bip) = if !opts.reflect {
                    (ColKey { kind: 0, c: a.0, lo: a.1, up: a.2, int, extra }, 1.0, false)
                } else {
                    let b = (qk(-c), qk(-up), qk(-lo));
                    if a == b && !int && !single {
                        (ColKey { kind: 2, c: 0, lo: a.1, up: a.2, int, extra }, 1.0, true)
                    } else if a <= b {
                        (ColKey { kind: 1, c: a.0, lo: a.1, up: a.2, int, extra }, 1.0, false)
                    } else {
                        (ColKey { kind: 1, c: b.0, lo: b.1, up: b.2, int, extra }, -1.0, false)
                    }
                };
                let k = ids.len() as u32;
                let id = *ids.entry(key).or_insert(k);
                if id as usize == ccls_bip.len() {
                    ccls_bip.push(bip);
                }
                col_cls[e] = id;
                col_sg[e] = sg;
            }
        }
        // 初めの行の組 (右辺 b'_v = −(A_ext δ^C)_v、反転なしなら 0 で 1 つの組)
        let mut row_cls = vec![0u32; m];
        let mut row_sg = vec![1.0f64; m];
        let mut rcls_bip: Vec<bool> = Vec::new();
        {
            let mut ids: HashMap<(bool, i64), u32> = HashMap::new();
            for v in 0..m {
                let (mut b, mut sc) = (0.0, 0.0);
                for (w, a) in row_ext(v) {
                    b -= a * dc[w];
                    sc += (a * dc[w]).abs();
                }
                let (key, sg, bip) = if !opts.reflect {
                    ((false, 0), 1.0, false)
                } else if b.abs() <= 1e-9 * (1.0 + sc) {
                    ((true, 0), 1.0, true)
                } else {
                    ((false, qk(b.abs())), b.signum(), false)
                };
                let k = ids.len() as u32;
                let id = *ids.entry(key).or_insert(k);
                if id as usize == rcls_bip.len() {
                    rcls_bip.push(bip);
                }
                row_cls[v] = id;
                row_sg[v] = sg;
            }
        }
        // 細分化。既定は待ち行列で割る側の組だけ処理する速い形 ([`refine_fast`])、`naive` なら行 → 列を止まるまで全部やり直す形
        let mut rounds = 0;
        if !opts.naive {
            match refine_fast(n, m, &p.rows, &cols, &mut row_cls, &mut row_sg, &mut rcls_bip, &mut col_cls, &mut col_sg, &mut ccls_bip, opts, t0) {
                Ok(r) => rounds = r,
                Err(()) => nope!(),
            }
        }
        let mut acc: Vec<f64> = Vec::new();
        let mut mark: Vec<bool> = Vec::new();
        let mut touched: Vec<u32> = Vec::new();
        let mut sig: Vec<(u32, i64)> = Vec::new();
        while opts.naive {
            rounds += 1;
            if rounds > opts.max_rounds || t0.elapsed().as_secs_f64() > opts.max_secs {
                nope!();
            }
            let (nr0, nc0) = (rcls_bip.len(), ccls_bip.len());
            // 行
            {
                acc.clear();
                acc.resize(nc0, 0.0);
                mark.clear();
                mark.resize(nc0, false);
                let mut ids: HashMap<(u32, bool, Vec<(u32, i64)>), u32> = HashMap::new();
                let mut nbip: Vec<bool> = Vec::new();
                for v in 0..m {
                    for (w, a) in row_ext(v) {
                        let c = col_cls[w];
                        if ccls_bip[c as usize] {
                            continue;
                        }
                        if !mark[c as usize] {
                            mark[c as usize] = true;
                            touched.push(c);
                        }
                        acc[c as usize] += a * col_sg[w];
                    }
                    sig.clear();
                    for &c in &touched {
                        let s = qk(acc[c as usize]);
                        if s != 0 {
                            sig.push((c, s));
                        }
                        acc[c as usize] = 0.0;
                        mark[c as usize] = false;
                    }
                    touched.clear();
                    sig.sort_unstable();
                    let old = row_cls[v];
                    let (bip, g) = if rcls_bip[old as usize] {
                        if sig.is_empty() {
                            (true, 1.0)
                        } else {
                            (false, if sig[0].1 > 0 { 1.0 } else { -1.0 })
                        }
                    } else {
                        (false, row_sg[v])
                    };
                    if g < 0.0 {
                        for s in sig.iter_mut() {
                            s.1 = -s.1;
                        }
                    }
                    row_sg[v] = g;
                    let k = ids.len() as u32;
                    let id = *ids.entry((old, bip, sig.clone())).or_insert(k);
                    if id as usize == nbip.len() {
                        nbip.push(bip);
                    }
                    row_cls[v] = id;
                }
                rcls_bip = nbip;
            }
            // 列
            {
                let nr = rcls_bip.len();
                acc.clear();
                acc.resize(nr, 0.0);
                mark.clear();
                mark.resize(nr, false);
                let mut ids: HashMap<(u32, bool, Vec<(u32, i64)>), u32> = HashMap::new();
                let mut nbip: Vec<bool> = Vec::new();
                for e in 0..ne {
                    for &(v, a) in &cols[e] {
                        let r = row_cls[v as usize];
                        if rcls_bip[r as usize] {
                            continue;
                        }
                        if !mark[r as usize] {
                            mark[r as usize] = true;
                            touched.push(r);
                        }
                        acc[r as usize] += a * row_sg[v as usize];
                    }
                    sig.clear();
                    for &r in &touched {
                        let s = qk(acc[r as usize]);
                        if s != 0 {
                            sig.push((r, s));
                        }
                        acc[r as usize] = 0.0;
                        mark[r as usize] = false;
                    }
                    touched.clear();
                    sig.sort_unstable();
                    let old = col_cls[e];
                    let (bip, g) = if ccls_bip[old as usize] {
                        if sig.is_empty() {
                            (true, 1.0)
                        } else {
                            (false, if sig[0].1 > 0 { 1.0 } else { -1.0 })
                        }
                    } else {
                        (false, col_sg[e])
                    };
                    if g < 0.0 {
                        for s in sig.iter_mut() {
                            s.1 = -s.1;
                        }
                    }
                    col_sg[e] = g;
                    let k = ids.len() as u32;
                    let id = *ids.entry((old, bip, sig.clone())).or_insert(k);
                    if id as usize == nbip.len() {
                        nbip.push(bip);
                    }
                    col_cls[e] = id;
                }
                ccls_bip = nbip;
            }
            if rcls_bip.len() == nr0 && ccls_bip.len() == nc0 {
                break;
            }
            // 縮みようがなければ早めにやめる
            if (rcls_bip.len() + ccls_bip.len()) as f64 > opts.max_ratio * (n + 2 * m) as f64 + 1.0 && rcls_bip.len() == m && ccls_bip.len() == ne {
                nope!();
            }
            // 組は細かくなる一方で、双極の組は単極に分かれることはあっても戻らないので、構造の列を含む単極の列の組の数と
            // 単極の行の組の数の和は畳んだ問題の (列 + 行) のおおよその下限 (列のない行は数えすぎ)。上限を超えたらやめる
            if opts.max_ratio < 1.0 {
                let mut seen = vec![false; ccls_bip.len()];
                let mut lb = rcls_bip.iter().filter(|&&b| !b).count();
                for e in 0..n {
                    let c = col_cls[e] as usize;
                    if !ccls_bip[c] && !seen[c] {
                        seen[c] = true;
                        lb += 1;
                    }
                }
                if lb as f64 > opts.max_ratio * (n + m) as f64 {
                    LAST_RATIO.with(|c| c.set(lb as f64 / (n + m) as f64));
                    nope!();
                }
            }
        }
        let ncc = ccls_bip.len();
        let nrc = rcls_bip.len();
        // 単極の組の符号は組ごとに丸ごと反転してよい (Proposition 15 の後の注意)。補う列・行が少ないよう、多数が +1 の向きにする
        {
            let mut cneg = vec![0i64; ncc];
            for e in 0..ne {
                cneg[col_cls[e] as usize] += if col_sg[e] < 0.0 { 1 } else { -1 };
            }
            for e in 0..ne {
                if !ccls_bip[col_cls[e] as usize] && cneg[col_cls[e] as usize] > 0 {
                    col_sg[e] = -col_sg[e];
                }
            }
            let mut rneg = vec![0i64; nrc];
            for v in 0..m {
                rneg[row_cls[v] as usize] += if row_sg[v] < 0.0 { 1 } else { -1 };
            }
            for v in 0..m {
                if !rcls_bip[row_cls[v] as usize] && rneg[row_cls[v] as usize] > 0 {
                    row_sg[v] = -row_sg[v];
                }
            }
        }
        // 組の大きさ
        let mut csize = vec![0usize; ncc];
        let mut csize_struct = vec![0usize; ncc];
        for e in 0..ne {
            csize[col_cls[e] as usize] += 1;
            if e < n {
                csize_struct[col_cls[e] as usize] += 1;
            }
        }
        let mut rsize = vec![0usize; nrc];
        for v in 0..m {
            rsize[row_cls[v] as usize] += 1;
        }
        // 組ごとの最初の要素
        let mut cfirst = vec![usize::MAX; ncc];
        for e in (0..ne).rev() {
            cfirst[col_cls[e] as usize] = e;
        }
        // δ: 双極は中央。単極は δ_w = δ^C_w + λ_w ν_Q で、多くの列が 0 になる ν を選ぶ (Theorem 20)
        let mut delta = dc.clone();
        if opts.reflect {
            let mut members: Vec<Vec<usize>> = vec![Vec::new(); ncc];
            for e in 0..ne {
                members[col_cls[e] as usize].push(e);
            }
            let mut freq: HashMap<i64, (usize, f64)> = HashMap::new();
            for (c, mem) in members.iter().enumerate() {
                if ccls_bip[c] {
                    continue;
                }
                let ok = |nu: f64| {
                    mem.iter().all(|&e| {
                        let d = dc[e] + col_sg[e] * nu;
                        d.is_finite() && d >= elo(e) - 1e-9 * (1.0 + d.abs()) && d <= eup(e) + 1e-9 * (1.0 + d.abs()) && (!eint(e) || (d - d.round()).abs() < 1e-9)
                    })
                };
                freq.clear();
                for &e in mem {
                    let v = -col_sg[e] * dc[e];
                    let ent = freq.entry(qk(v)).or_insert((0, v));
                    ent.0 += 1;
                }
                let mut cands: Vec<(usize, f64)> = freq.values().copied().collect();
                cands.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)));
                let e0 = mem[0];
                let mut nus: Vec<f64> = cands.iter().take(3).map(|c| c.1).collect();
                if elo(e0).is_finite() && eup(e0).is_finite() {
                    nus.push(-0.5 * (eup(e0) - elo(e0)));
                }
                nus.push(0.0);
                let nu = nus.into_iter().find(|&nu| ok(nu));
                let Some(nu) = nu else {
                    nope!();
                };
                for &e in mem {
                    let d = dc[e] + col_sg[e] * nu;
                    delta[e] = if eint(e) { d.round() } else if d.abs() < 1e-12 { 0.0 } else { d };
                }
            }
        }
        // 行の右辺 b_v = −(A_ext δ)_v
        let mut rhs = vec![0.0f64; m];
        let mut rhs_sc = vec![0.0f64; m];
        for v in 0..m {
            for (w, a) in row_ext(v) {
                rhs[v] -= a * delta[w];
                rhs_sc[v] += (a * delta[w]).abs();
            }
        }
        // 組の和 (P, Q) の合計 T_PQ = Σ_{v∈P, w∈Q} γ_v A_vw λ_w
        let mut tot: HashMap<(u32, u32), f64> = HashMap::new();
        for v in 0..m {
            let pr = row_cls[v];
            if rcls_bip[pr as usize] {
                continue;
            }
            for (w, a) in row_ext(v) {
                let q = col_cls[w];
                if ccls_bip[q as usize] {
                    continue;
                }
                *tot.entry((pr, q)).or_insert(0.0) += row_sg[v] * a * col_sg[w];
            }
        }
        // 検査: 行ごと・列ごとの組の和が R_PQ = T/|P|、C_PQ = T/|Q| に等しい (双極の組との和は 0)
        {
            let get = |pr: u32, q: u32| tot.get(&(pr, q)).copied().unwrap_or(0.0);
            let mut accm: HashMap<u32, (f64, f64)> = HashMap::new();
            // 行の組ごとに非零の列の組の数 (行ごとの非零の組の数と比べる)
            let mut nz_per_rc: Vec<usize> = vec![0; nrc];
            let mut nz_per_cc: Vec<usize> = vec![0; ncc];
            for (&(pr, q), &t) in tot.iter() {
                if qk(t / rsize[pr as usize] as f64) != 0 {
                    nz_per_rc[pr as usize] += 1;
                }
                if qk(t / csize[q as usize] as f64) != 0 {
                    nz_per_cc[q as usize] += 1;
                }
            }
            for v in 0..m {
                accm.clear();
                let pr = row_cls[v];
                for (w, a) in row_ext(v) {
                    let q = col_cls[w];
                    if ccls_bip[q as usize] {
                        continue;
                    }
                    let ent = accm.entry(q).or_insert((0.0, 0.0));
                    ent.0 += row_sg[v] * a * col_sg[w];
                    ent.1 += a.abs();
                }
                let bip = rcls_bip[pr as usize];
                let mut nz = 0;
                for (&q, &(s, sc)) in accm.iter() {
                    let want = if bip { 0.0 } else { get(pr, q) / rsize[pr as usize] as f64 };
                    if !close(s, want, sc) {
                        nope!();
                    }
                    if qk(want) != 0 {
                        nz += 1;
                    }
                }
                if !bip && nz != nz_per_rc[pr as usize] {
                    nope!();
                }
            }
            for e in 0..ne {
                accm.clear();
                let q = col_cls[e];
                for &(v, a) in &cols[e] {
                    let pr = row_cls[v as usize];
                    if rcls_bip[pr as usize] {
                        continue;
                    }
                    let ent = accm.entry(pr).or_insert((0.0, 0.0));
                    ent.0 += row_sg[v as usize] * a * col_sg[e];
                    ent.1 += a.abs();
                }
                let bip = ccls_bip[q as usize];
                let mut nz = 0;
                for (&pr, &(s, sc)) in accm.iter() {
                    let want = if bip { 0.0 } else { get(pr, q) / csize[q as usize] as f64 };
                    if !close(s, want, sc) {
                        nope!();
                    }
                    if qk(want) != 0 {
                        nz += 1;
                    }
                }
                if !bip && nz != nz_per_cc[q as usize] {
                    nope!();
                }
            }
        }
        // 検査: 行の右辺 (γ_v b_v が組で等しい、双極なら 0)
        let mut rrhs = vec![f64::NAN; nrc];
        for v in 0..m {
            let pr = row_cls[v] as usize;
            let b = row_sg[v] * rhs[v];
            if rcls_bip[pr] {
                if !close(b, 0.0, rhs_sc[v]) {
                    nope!();
                }
            } else if rrhs[pr].is_nan() {
                rrhs[pr] = b;
            } else if !close(rrhs[pr], b, rhs_sc[v]) {
                nope!();
            }
        }
        // 検査: 列の境界・費用 (z の範囲と λ c が組で等しい。双極は費用 0・範囲が δ について対称)
        let zr = |e: usize| -> (f64, f64) {
            if col_sg[e] > 0.0 {
                (elo(e) - delta[e], eup(e) - delta[e])
            } else {
                (delta[e] - eup(e), delta[e] - elo(e))
            }
        };
        for e in 0..ne {
            let q = col_cls[e] as usize;
            let (l, u) = zr(e);
            if ccls_bip[q] {
                if ecost(e) != 0.0 || !close(l, -u, delta[e].abs()) || eint(e) {
                    nope!();
                }
                continue;
            }
            let f = cfirst[q];
            let (l0, u0) = zr(f);
            if !close(l, l0, delta[e].abs()) || !close(u, u0, delta[e].abs()) || !close(col_sg[e] * ecost(e), col_sg[f] * ecost(f), 0.0) || eint(e) != eint(f) {
                nope!();
            }
        }
        // 畳んだ問題を作る。列: 単極の組、行: 単極の組
        let mut red_col = vec![u32::MAX; ncc];
        let mut red_row = vec![u32::MAX; nrc];
        // スラックだけの組で、畳んだ行列の非零が 1 つなら行の範囲に吸収する
        let mut c_slack_only = vec![true; ncc];
        for e in 0..n {
            c_slack_only[col_cls[e] as usize] = false;
        }
        let mut c_nnz = vec![0usize; ncc];
        let mut c_row = vec![u32::MAX; ncc];
        for (&(pr, q), &t) in tot.iter() {
            if qk(t) != 0 {
                c_nnz[q as usize] += 1;
                c_row[q as usize] = pr;
            }
        }
        let mut row_abs: Vec<Option<u32>> = vec![None; nrc];
        let mut absorbed = 0;
        let mut ncol_red = 0u32;
        let mut red_scale: Vec<f64> = Vec::new();
        let mut red_members: Vec<Vec<u32>> = Vec::new();
        let mut col_lo_r = Vec::new();
        let mut col_up_r = Vec::new();
        let mut cost_r = Vec::new();
        let mut int_r = Vec::new();
        let mut int_groups = 0;
        let mut int_grouped_cols = 0;
        for q in 0..ncc {
            if ccls_bip[q] {
                continue;
            }
            if c_slack_only[q] {
                if c_nnz[q] == 0 {
                    // どの行にも効かないスラック (範囲が空でなければ捨てる)
                    let (l, u) = zr(cfirst[q]);
                    if l > u + 1e-9 * (1.0 + l.abs()) {
                        nope!();
                    }
                    continue;
                }
                if c_nnz[q] == 1 && row_abs[c_row[q] as usize].is_none() {
                    row_abs[c_row[q] as usize] = Some(q as u32);
                    absorbed += 1;
                    continue;
                }
            }
            let f = cfirst[q];
            let int = eint(f);
            let sz = csize[q] as f64;
            let scale = if int && csize[q] > 1 { sz } else { 1.0 };
            if int && csize[q] > 1 {
                int_groups += 1;
                int_grouped_cols += csize[q];
            }
            let (l, u) = zr(f);
            red_col[q] = ncol_red;
            ncol_red += 1;
            red_scale.push(scale);
            red_members.push(Vec::new());
            col_lo_r.push(l * scale);
            col_up_r.push(u * scale);
            cost_r.push(sz * col_sg[f] * ecost(f) / scale);
            int_r.push(int);
        }
        for e in 0..n {
            let q = col_cls[e] as usize;
            if red_col[q] != u32::MAX {
                red_members[red_col[q] as usize].push(e as u32);
            }
        }
        let mut nrow_red = 0u32;
        for pr in 0..nrc {
            if !rcls_bip[pr] {
                red_row[pr] = nrow_red;
                nrow_red += 1;
            }
        }
        let mut rows_r: Vec<Vec<(usize, f64)>> = vec![Vec::new(); nrow_red as usize];
        let mut row_lo_r = vec![0.0; nrow_red as usize];
        let mut row_up_r = vec![0.0; nrow_red as usize];
        let mut keys: Vec<(&(u32, u32), &f64)> = tot.iter().collect();
        keys.sort_by_key(|a| *a.0);
        for (&(pr, q), &t) in keys {
            let rq = red_col[q as usize];
            if rq == u32::MAX || qk(t) == 0 {
                continue;
            }
            let rr = red_row[pr as usize] as usize;
            let coef = t / rsize[pr as usize] as f64 / red_scale[rq as usize];
            rows_r[rr].push((rq as usize, coef));
        }
        for pr in 0..nrc {
            if rcls_bip[pr] {
                continue;
            }
            let rr = red_row[pr] as usize;
            rows_r[rr].sort_by_key(|a| a.0);
            let b = rrhs[pr];
            let (lo, up) = match row_abs[pr] {
                None => (b, b),
                Some(q) => {
                    let alpha = tot[&(pr as u32, q)] / rsize[pr] as f64;
                    let (l, u) = zr(cfirst[q as usize]);
                    // Σ others + α z = b, z ∈ [l, u]
                    let (a1, a2) = (b - alpha * u, b - alpha * l);
                    if alpha > 0.0 {
                        (a1, a2)
                    } else {
                        (a2, a1)
                    }
                }
            };
            let lo = if lo.is_nan() { f64::NEG_INFINITY } else { lo };
            let up = if up.is_nan() { f64::INFINITY } else { up };
            if rows_r[rr].is_empty() {
                // 列のない行: 0 が範囲に入らなければ実行不能 (畳まずに元の求解に任せる)
                if lo > 1e-9 * (1.0 + lo.abs()) || up < -1e-9 * (1.0 + up.abs()) {
                    nope!();
                }
            }
            row_lo_r[rr] = lo;
            row_up_r[rr] = up;
        }
        // 列のない行を除く
        let mut keep_row = vec![u32::MAX; nrow_red as usize];
        let mut k = 0u32;
        let mut rows_f = Vec::new();
        let mut lo_f = Vec::new();
        let mut up_f = Vec::new();
        let mut size_f = Vec::new();
        let mut rsize_of_red = vec![0usize; nrow_red as usize];
        for pr in 0..nrc {
            if red_row[pr] != u32::MAX {
                rsize_of_red[red_row[pr] as usize] = rsize[pr];
            }
        }
        for rr in 0..nrow_red as usize {
            if rows_r[rr].is_empty() {
                continue;
            }
            keep_row[rr] = k;
            k += 1;
            rows_f.push(std::mem::take(&mut rows_r[rr]));
            lo_f.push(row_lo_r[rr]);
            up_f.push(row_up_r[rr]);
            size_f.push(rsize_of_red[rr]);
        }
        let n_red = ncol_red as usize;
        let m_red = rows_f.len();
        if (n_red + m_red) as f64 > opts.max_ratio * (n + m) as f64 {
            LAST_RATIO.with(|c| c.set((n_red + m_red) as f64 / (n + m) as f64));
            nope!();
        }
        let mut offset = 0.0;
        for e in 0..n {
            offset += p.cost[e] * delta[e];
        }
        let mut col_red = vec![u32::MAX; n];
        let mut col_sign = vec![1.0; n];
        let mut col_off = vec![0.0; n];
        let mut stats = FoldStats { n, m, n_red, m_red, rounds, col_parts: ncc, row_parts: nrc, absorbed, int_groups, int_grouped_cols, ..Default::default() };
        for e in 0..n {
            let q = col_cls[e] as usize;
            col_red[e] = red_col[q];
            col_sign[e] = col_sg[e];
            col_off[e] = delta[e];
            if ccls_bip[q] {
                stats.bip_cols += 1;
            } else {
                if col_sg[e] < 0.0 {
                    stats.neg_cols += 1;
                }
                if csize_struct[q] > 1 || csize[q] > 1 {
                    stats.grouped_cols += 1;
                }
            }
        }
        let mut row_red = vec![u32::MAX; m];
        for v in 0..m {
            let pr = row_cls[v] as usize;
            if rcls_bip[pr] {
                stats.bip_rows += 1;
            } else {
                row_red[v] = keep_row[red_row[pr] as usize];
            }
        }
        stats.secs = t0.elapsed().as_secs_f64();
        Some(Fold {
            red: FoldLp { col_lo: col_lo_r, col_up: col_up_r, cost: cost_r, is_int: int_r, rows: rows_f, row_lo: lo_f, row_up: up_f, offset },
            col_red,
            col_sign,
            col_off,
            red_scale,
            red_members,
            row_red,
            row_sign: row_sg,
            red_row_size: size_f,
            stats,
        })
    }

    /// 畳んだ問題の解 y を元の構造の列に配る (x_w = δ_w + λ_w y_q / scale_q)。
    pub fn unfold(&self, y: &[f64]) -> Vec<f64> {
        (0..self.col_red.len())
            .map(|w| {
                let q = self.col_red[w];
                if q == u32::MAX {
                    self.col_off[w]
                } else {
                    self.col_off[w] + self.col_sign[w] * y[q as usize] / self.red_scale[q as usize]
                }
            })
            .collect()
    }

    /// 畳んだ問題の行の双対値 pi' を元の行に配る (π_v = γ_v π'_P / |P|、双極の行は 0)。
    pub fn unfold_duals(&self, pi: &[f64]) -> Vec<f64> {
        (0..self.row_red.len())
            .map(|v| {
                let r = self.row_red[v];
                if r == u32::MAX {
                    0.0
                } else {
                    self.row_sign[v] * pi[r as usize] / self.red_row_size[r as usize] as f64
                }
            })
            .collect()
    }
}

/// 元の問題 (最小化) で x が実行可能か (許容誤差 tol、行は係数の絶対値の和で尺度をとる)。最大の違反 (尺度で割った値) を返す。
pub fn max_violation(p: &FoldIn, x: &[f64]) -> f64 {
    let mut worst = 0.0f64;
    for w in 0..x.len() {
        let s = 1.0 + x[w].abs();
        if x[w] < p.col_lo[w] {
            worst = worst.max((p.col_lo[w] - x[w]) / s);
        }
        if x[w] > p.col_up[w] {
            worst = worst.max((x[w] - p.col_up[w]) / s);
        }
        if !x[w].is_finite() {
            return f64::INFINITY;
        }
    }
    for (v, r) in p.rows.iter().enumerate() {
        let (mut a, mut s) = (0.0, 1.0);
        for &(w, c) in r {
            a += c * x[w];
            s += (c * x[w]).abs();
        }
        if a < p.row_lo[v] {
            worst = worst.max((p.row_lo[v] - a) / (s + p.row_lo[v].abs()));
        }
        if a > p.row_up[v] {
            worst = worst.max((a - p.row_up[v]) / (s + p.row_up[v].abs()));
        }
    }
    worst
}

/// 弱双対の下界 g(π) = min_{x∈箱} (c − Aᵀπ) x + min_{r∈[lo,up]} π r (最小化。π の符号の約束によらないよう ±π の大きい方)。
pub fn dual_bound(p: &FoldIn, pi: &[f64]) -> f64 {
    let n = p.col_lo.len();
    // ごく小さい双対値 (丸め誤差で符号が揺れる) は 0 とみなす
    let pmax = pi.iter().fold(0.0f64, |a, &b| a.max(b.abs()));
    let pi: Vec<f64> = pi.iter().map(|&y| if y.abs() <= 1e-12 * (1.0 + pmax) { 0.0 } else { y }).collect();
    let one = |sg: f64| -> f64 {
        let mut d: Vec<f64> = p.cost.to_vec();
        for (v, r) in p.rows.iter().enumerate() {
            let y = sg * pi[v];
            if y != 0.0 {
                for &(w, a) in r {
                    d[w] -= a * y;
                }
            }
        }
        let mut g = 0.0;
        for w in 0..n {
            let dw = d[w];
            if dw.abs() <= 1e-9 * (1.0 + p.cost[w].abs()) {
                continue;
            }
            let b = if dw > 0.0 { p.col_lo[w] } else { p.col_up[w] };
            g += dw * b;
        }
        for v in 0..p.rows.len() {
            let y = sg * pi[v];
            if y == 0.0 {
                continue;
            }
            let b = if y > 0.0 { p.row_lo[v] } else { p.row_up[v] };
            g += y * b;
        }
        if g.is_nan() {
            f64::NEG_INFINITY
        } else {
            g
        }
    };
    one(1.0).max(one(-1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fin<'a>(lo: &'a [f64], up: &'a [f64], c: &'a [f64], int: &'a [bool], rows: &'a [Vec<(usize, f64)>], rlo: &'a [f64], rup: &'a [f64]) -> FoldIn<'a> {
        FoldIn { col_lo: lo, col_up: up, cost: c, is_int: int, rows, row_lo: rlo, row_up: rup, int_key: None, single: None }
    }

    #[test]
    fn folds_symmetric_assignment_lp() {
        // 3 × 3 の割当の LP 緩和 (費用が全部 1): 9 列 → 1 列にまとまる
        let n = 9;
        let lo = vec![0.0; n];
        let up = vec![1.0; n];
        let c = vec![1.0; n];
        let int = vec![false; n];
        let mut rows = Vec::new();
        for i in 0..3 {
            rows.push((0..3).map(|j| (3 * i + j, 1.0)).collect::<Vec<_>>());
        }
        for j in 0..3 {
            rows.push((0..3).map(|i| (3 * i + j, 1.0)).collect::<Vec<_>>());
        }
        let rlo = vec![1.0; 6];
        let rup = vec![1.0; 6];
        let p = fin(&lo, &up, &c, &int, &rows, &rlo, &rup);
        let f = Fold::build(&p, FoldOpts::default()).expect("folds");
        assert_eq!(f.red.cost.len(), 1);
        let x = f.unfold(&[1.0 / 3.0]);
        assert!(max_violation(&p, &x) < 1e-12);
    }

    #[test]
    fn reflection_folds_paper_example() {
        // 論文の例 (3): min x1 − x2, [2 1 1; −1 −2 −1; −1 1 0] x ≤ [5; −3; 1], 0 ≤ x ≤ 2
        let lo = vec![0.0; 3];
        let up = vec![2.0; 3];
        let c = vec![1.0, -1.0, 0.0];
        let int = vec![false; 3];
        let rows = vec![vec![(0, 2.0), (1, 1.0), (2, 1.0)], vec![(0, -1.0), (1, -2.0), (2, -1.0)], vec![(0, -1.0), (1, 1.0)]];
        let rlo = vec![f64::NEG_INFINITY; 3];
        let rup = vec![5.0, -3.0, 1.0];
        let p = fin(&lo, &up, &c, &int, &rows, &rlo, &rup);
        assert!(Fold::build(&p, FoldOpts { max_ratio: 1.0, ..Default::default() }).is_none_or(|f| f.red.cost.len() >= 3));
        let f = Fold::build(&p, FoldOpts { reflect: true, max_ratio: 1.0, ..Default::default() }).expect("folds with reflection");
        // x1, x2 は反転でまとまり、x3 は中央 1 に固定
        assert_eq!(f.col_red[0], f.col_red[1]);
        assert_eq!(f.col_red[2], u32::MAX);
        assert!((f.col_off[2] - 1.0).abs() < 1e-12);
        assert_eq!(f.red.cost.len(), 1);
        // 畳んだ問題: min z s.t. 行の範囲。最適は z の最小 (x1 = 0.5, x2 = 1.5 など)。範囲の下端を配って確かめる
        let mut best = None;
        for k in 0..=400 {
            let z = -2.0 + 4.0 * k as f64 / 400.0;
            let y = [z];
            let ok = f.red.rows.iter().enumerate().all(|(i, r)| {
                let a: f64 = r.iter().map(|&(_, c)| c * y[0]).sum();
                a >= f.red.row_lo[i] - 1e-9 && a <= f.red.row_up[i] + 1e-9
            }) && z >= f.red.col_lo[0] - 1e-9
                && z <= f.red.col_up[0] + 1e-9;
            if ok && best.is_none_or(|b: f64| f.red.cost[0] * z < f.red.cost[0] * b) {
                best = Some(z);
            }
        }
        let z = best.expect("feasible");
        let x = f.unfold(&[z]);
        assert!(max_violation(&p, &x) < 1e-9, "x = {x:?}");
        let obj = x[0] - x[1];
        assert!((obj - (f.red.cost[0] * z + f.red.offset)).abs() < 1e-9);
        assert!((obj - (-1.0)).abs() < 1e-9, "obj = {obj}");
    }
    #[test]
    fn reflection_duals_map_to_optimal_duals() {
        // fuzz_fold.py の seed 5 (lp): 反転で畳んだ LP の双対値を配ると元の LP の最適な双対値になる
        let lo = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let up = vec![1.0, 1.0, 2.0, 1.0, 2.0, 4.0, 1.0, 4.0, 2.0, 4.0, 4.0, 2.0];
        let c = vec![-5.0, 5.0, -3.0, 5.0, -3.0, -5.0, 5.0, -5.0, 3.0, -5.0, 5.0, -3.0];
        let int = vec![false; lo.len()];
        let rows: Vec<Vec<(usize, f64)>> = vec![vec![(1, 1.0), (7, 2.0), (2, -1.0)], vec![(5, 3.0), (4, -2.0)], vec![(1, 3.0), (7, 1.0), (2, -2.0)], vec![(3, 1.0), (9, 2.0), (11, -1.0)], vec![(11, 1.0), (8, -1.0), (4, 1.0), (2, 1.0)], vec![(6, 1.0), (5, 2.0), (4, -1.0)], vec![(0, -1.0), (10, -2.0), (8, 1.0)], vec![(9, 3.0), (11, -2.0)], vec![(7, 3.0), (2, -2.0)], vec![(6, 3.0), (5, 1.0), (4, -2.0)], vec![(3, 1.0), (0, -1.0), (6, 1.0), (1, 1.0)], vec![(10, -3.0), (8, 2.0)], vec![(0, -3.0), (10, -1.0), (8, 2.0)], vec![(3, 3.0), (9, 1.0), (11, -2.0)]];
        let rlo = vec![f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0, f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        let rup = vec![4.0, 4.0, 0.0, 4.0, 1.0, 4.0, -3.0, 4.0, 4.0, 0.0, 0.0, -4.0, -3.0, 0.0];
        let p = fin(&lo, &up, &c, &int, &rows, &rlo, &rup);
        let mut lp = crate::simplex::mip_lp::TwoStageLp::new(&lo, &up, &c, &rows, &rlo, &rup);
        lp.solve(&crate::mip::lp::SolveLimits::default());
        let pi = lp.row_duals();
        eprintln!("orig obj {} duals {:?} bound {}", lp.objective(), pi, dual_bound(&p, &pi));
        let f = Fold::build(&p, FoldOpts { reflect: true, max_ratio: 1.0, ..Default::default() }).unwrap();
        let r = &f.red;
        eprintln!("red {:?}", r);
        eprintln!("col_red {:?} sign {:?} off {:?} row_red {:?} row_sign {:?}", f.col_red, f.col_sign, f.col_off, f.row_red, f.row_sign);
        let mut lp2 = crate::simplex::mip_lp::TwoStageLp::new(&r.col_lo, &r.col_up, &r.cost, &r.rows, &r.row_lo, &r.row_up);
        lp2.solve(&crate::mip::lp::SolveLimits::default());
        let pi2 = f.unfold_duals(&lp2.row_duals());
        let g = dual_bound(&p, &pi2);
        eprintln!("red obj {} rduals {:?} mapped {:?} bound {}", lp2.objective() + r.offset, lp2.row_duals(), pi2, g);
        assert!((g - lp.objective()).abs() < 1e-7, "dual bound {g}");
    }

}

/// `ENOMOTO_LP_FOLD` の値 (付けていなければ `None`)。値の文字: `r` 反転も使う (R-DRCR)、`v` 検査 (畳んだ問題を MIP の LP の
/// 実装でも解いて双対値を配り、元の問題の弱双対の下界と主の目的値の差 (相補性) を確かめる。外れたら畳まずに解く)、
/// `q` ログを出さない、`c` 畳んだ数を出すだけで解かない (`not_solved` を返す。数える用)。`1` は反転なし。
pub fn lp_fold_flags() -> Option<&'static str> {
    env_str!("ENOMOTO_LP_FOLD").filter(|v| *v != "0" && *v != "off")
}

/// 細分化の設定を環境変数から (`ENOMOTO_T_FOLD_MAX_RATIO`: 縮みの割合の上限、既定 0.95)。
pub fn opts_from_env(reflect: bool) -> FoldOpts {
    FoldOpts { reflect, max_ratio: tunable!("ENOMOTO_T_FOLD_MAX_RATIO", 0.95, f64), naive: env_str!("ENOMOTO_T_FOLD_NAIVE").is_some(), ..Default::default() }
}

/// `ENOMOTO_LP_FOLD`: LP を畳んで解き、解を配る。畳まない・安全網に外れたなら `None` (呼び出し側が元の LP を解く)。
pub fn try_solve_lp(
    variables: &[crate::types::VariableData],
    objective: &crate::types::Objective,
    constraints: &[crate::types::ConstraintRow],
    solve: &dyn Fn(&[crate::types::VariableData], &crate::types::Objective, &[crate::types::ConstraintRow]) -> crate::types::SolveResult,
) -> Option<crate::types::SolveResult> {
    use crate::types::{ConstraintRow, LinearExpr, Objective, RowSense, Sense, SolveResult, Status, VarType, VariableData};
    let flags = lp_fold_flags()?;
    let log = !flags.contains('q');
    let t0 = std::time::Instant::now();
    let n = variables.len();
    let col_lo: Vec<f64> = variables.iter().map(|v| v.lb).collect();
    let col_up: Vec<f64> = variables.iter().map(|v| v.ub).collect();
    let sg = if objective.sense == Sense::Maximize { -1.0 } else { 1.0 };
    let mut cost = vec![0.0; n];
    for (&j, &c) in &objective.expr.coeffs {
        cost[j] += sg * c;
    }
    let is_int = vec![false; n];
    let mut rows = Vec::with_capacity(constraints.len());
    let mut row_lo = Vec::with_capacity(constraints.len());
    let mut row_up = Vec::with_capacity(constraints.len());
    for c in constraints {
        rows.push(c.expr.coeffs.iter().filter(|(_, &a)| a != 0.0).map(|(&j, &a)| (j, a)).collect::<Vec<_>>());
        let (lo, up) = match c.sense {
            RowSense::Le => (f64::NEG_INFINITY, c.rhs),
            RowSense::Ge => (c.rhs, f64::INFINITY),
            RowSense::Eq => (c.rhs, c.rhs),
        };
        row_lo.push(lo);
        row_up.push(up);
    }
    let fin = FoldIn { col_lo: &col_lo, col_up: &col_up, cost: &cost, is_int: &is_int, rows: &rows, row_lo: &row_lo, row_up: &row_up, int_key: None, single: None };
    let reflect = flags.contains('r');
    LAST_FAIL.with(|c| c.set(0));
    LAST_RATIO.with(|c| c.set(1.0));
    let Some(f) = Fold::build(&fin, opts_from_env(reflect)) else {
        if flags.contains('c') {
            eprintln!("LP_FOLD: no fold (n {} m {}) in {:.3}s (reason line {}, ratio {:.3})", n, rows.len(), t0.elapsed().as_secs_f64(), LAST_FAIL.with(|c| c.get()), LAST_RATIO.with(|c| c.get()));
            return Some(SolveResult { status: Status::NotSolved, objective: None, x: None, node_limit_hit: false, mip: None });
        }
        if log {
            eprintln!(
                "LP_FOLD: no fold (n {} m {}) in {:.3}s (reason line {}, ratio {:.3})",
                n,
                rows.len(),
                t0.elapsed().as_secs_f64(),
                LAST_FAIL.with(|c| c.get()),
                LAST_RATIO.with(|c| c.get())
            );
        }
        return None;
    };
    let st = &f.stats;
    let t_build = t0.elapsed().as_secs_f64();
    if log {
        eprintln!(
            "LP_FOLD: n {} -> {}, m {} -> {} (bip cols {}, bip rows {}, neg cols {}, absorbed {}, rounds {}) in {:.3}s",
            st.n, st.n_red, st.m, st.m_red, st.bip_cols, st.bip_rows, st.neg_cols, st.absorbed, st.rounds, t_build
        );
    }
    if flags.contains('c') {
        // 数えるだけ (解かずに終える)
        return Some(SolveResult { status: Status::NotSolved, objective: None, x: None, node_limit_hit: false, mip: None });
    }
    // 畳んだ問題を元の形 (変数・目的・制約) にする
    let r = &f.red;
    let vars: Vec<VariableData> = (0..r.cost.len()).map(|q| VariableData { vtype: VarType::Continuous, lb: r.col_lo[q], ub: r.col_up[q] }).collect();
    let mut oc = std::collections::BTreeMap::new();
    for (q, &c) in r.cost.iter().enumerate() {
        if c != 0.0 {
            oc.insert(q, c);
        }
    }
    let obj = Objective { expr: LinearExpr { coeffs: oc, constant: 0.0 }, sense: Sense::Minimize };
    let mut cons = Vec::new();
    for (i, row) in r.rows.iter().enumerate() {
        let expr = LinearExpr { coeffs: row.iter().copied().collect(), constant: 0.0 };
        let (lo, up) = (r.row_lo[i], r.row_up[i]);
        if lo == up {
            cons.push(ConstraintRow { expr, sense: RowSense::Eq, rhs: lo });
            continue;
        }
        if up.is_finite() {
            cons.push(ConstraintRow { expr: expr.clone(), sense: RowSense::Le, rhs: up });
        }
        if lo.is_finite() {
            cons.push(ConstraintRow { expr, sense: RowSense::Ge, rhs: lo });
        }
    }
    let res = solve(&vars, &obj, &cons);
    if res.status != Status::Optimal {
        if log {
            eprintln!("LP_FOLD: the folded LP ended with {:?}; solving the original", res.status);
        }
        return None;
    }
    let y = res.x.as_ref()?;
    let x = f.unfold(y);
    let viol = max_violation(&fin, &x);
    let obj_x: f64 = (0..n).map(|w| cost[w] * x[w]).sum();
    let obj_y: f64 = (0..y.len()).map(|q| r.cost[q] * y[q]).sum::<f64>() + r.offset;
    let obj_ok = (obj_x - obj_y).abs() <= 1e-7 * (1.0 + obj_x.abs());
    if viol > 1e-6 || !obj_ok {
        if log {
            eprintln!("LP_FOLD: safety net: violation {viol:.2e}, objective {obj_x} vs folded {obj_y}; solving the original");
        }
        return None;
    }
    if flags.contains('v') {
        // 検査: 畳んだ問題を MIP の LP の実装で解いて双対値を得て、元の行に配り、弱双対の下界と比べる
        let mut lp = crate::simplex::mip_lp::TwoStageLp::new(&r.col_lo, &r.col_up, &r.cost, &r.rows, &r.row_lo, &r.row_up);
        let stl = lp.solve(&crate::mip::lp::SolveLimits::default());
        let ok = if stl == crate::mip::lp::LpStatus::Optimal {
            let pi = f.unfold_duals(&lp.row_duals());
            let g = dual_bound(&fin, &pi);
            let gap = (obj_x - g) / (1.0 + obj_x.abs());
            eprintln!("LP_FOLD: verify: primal {obj_x:.10e} dual bound {g:.10e} rel gap {gap:.2e} (folded LP objective by the MIP LP {:.10e})", lp.objective() + r.offset);
            gap.abs() <= 1e-6
        } else {
            eprintln!("LP_FOLD: verify: the MIP LP of the folded problem ended with {stl:?}");
            false
        };
        if !ok {
            eprintln!("LP_FOLD: verify failed; solving the original");
            return None;
        }
    }
    let constant = objective.expr.constant;
    let obj_val = constant + objective.expr.coeffs.iter().map(|(&j, &c)| c * x[j]).sum::<f64>();
    if log {
        eprintln!("LP_FOLD: done in {:.3}s (fold {:.3}s), violation {viol:.1e}", t0.elapsed().as_secs_f64(), t_build);
    }
    Some(SolveResult { status: Status::Optimal, objective: Some(obj_val), x: Some(x), node_limit_hit: false, mip: None })
}
