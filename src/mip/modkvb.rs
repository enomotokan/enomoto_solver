//! 試作 `ENOMOTO_MIP_MODKVB` (既定では使わない): HiGHS の `HighsModkSeparator` に合わせた mod-k の行の組の探索
//! (`analysis/modkvb_20261107.md`)。
//!
//! HiGHS の mod-k は、LP で等号の行 (`<=` 側か `>=` 側) を `HighsTransformedLp::transform` で単純な上下限と変数上下限に
//! 置き換えた空間に移し、**LP 値が 0 でない (境界から feastol より離れた) 整数列**の係数と右辺を整数に倍率をかけた連立を作り、
//! GF(k) (k = 2, 3, 5, 7) の消去で「その列の係数がみな k で割り切れ、右辺が割り切れない」行の組を探す。見つかった組の
//! 重みで LP の行 (論理変数つきの等式) を足した集約行を `generateCut` (CMIR・lifted cover) に渡す。mushroom-best では、
//! これが「x0 の符号が逆のデータの行 2 本 (+ 小数の w を消す w-z の行)」を見つけ、変数上下限の置き換えで z だけのカットに
//! なる (`analysis/mushroom_cuts_20261106.md`)。ENOMOTO の zerohalf (`zerohalf.rs`) は元の行と単純な上下限だけで数えるので、
//! この組を見つけない。
//!
//! ここでの扱い (HiGHS に合わせたもの):
//! - 連続列: 単純な上下限と (連続列の) 変数上下限のうち LP の点で一番近いものからの距離が feastol より大きければ、その列を
//!   含む行は使わない (HiGHS の `skipRow`)。境界にあれば定数に (変数上下限なら相手の 0-1 列の項に) 置き換える。
//!   単純な上下限と変数上下限が両方 0 の距離なら単純な上下限を使う。
//! - 一般整数列 (幅 > 1.5): LP 値が整数で単純な上下限の内側にあり、一般整数列の変数上下限 (`VarBounds::ivub`/`ivlb`) が
//!   ちょうど効いている (距離が feastol 以下) なら置き換える (HiGHS の `HighsTransformedLp` の条件)。そうでなければ近い方の
//!   単純な上下限 (同点なら上限、HiGHS と同じ) で `x' = x - l` / `u - x` にする。
//! - 置き換えの後に残る整数列のうち、`x'` の LP 値が feastol より大きいものだけを連立に入れる。行の係数が整数でなければ、
//!   その列の係数と右辺が整数になる倍率 (1e6 まで) をかける。
//! - GF(k) の消去: 行を 1 本ずつ、それまでの軸の行で消していき、列が全部消えて右辺が残れば解 (行の組と重み)。残った列が
//!   あれば、列の出る数が一番少ない列を軸にして新しい軸の行にする。組の行の数が上限を超えたものは捨てる。
//!   組の多様さのために、行の順を変えて何回か回す (HiGHS は軸の入れ替えで別の解を列挙する)。
//! - 集約: 解の重み w から、HiGHS と同じ 2 通り (`((k-1) w mod k) / k` と、`w / k` で符号を反転したもの、k = 2 は前者の反転だけ)。
//!   LP の行は論理変数 (列 n + i、境界は行の上下限) つきの等式 `a x - s = 0` で足すので、どちらの向きでも妥当。
//! - カット: 集約行 (右辺 0) を [`super::cuts::vb_cover_cut`] と、一般整数列の変数上下限の置き換えを
//!   (`ENOMOTO_MIP_MODKVB_IV`、既定 3) 使った [`super::cuts::generate_cuts`] に渡す。解ごとに返す (呼び出し側が最良の 1 本を残す)。
//! - ある k でカットが出れば次の k は試さない (HiGHS と同じ)。
//! - 集約行の非零が `1000 + n / 10` (`ENOMOTO_T_MIP_MODKVB_MAXLEN`) を超えたら使わない。分離 1 回の時間の上限は
//!   `ENOMOTO_T_MIP_MODKVB_TIME` (既定 0.3 秒)、解の数は k ごとに `ENOMOTO_T_MIP_MODKVB_SOLS` (既定 300、1 本だけの解は別に 150 まで)。

use super::cuts::{CutVars, RawCut};

/// 試作 `ENOMOTO_MIP_MODKVB` の動き方 (既定では `None` = 使わない)。`cover` = [`super::cuts::vb_cover_cut`] だけ、`gen` =
/// [`super::cuts::generate_cuts`] だけ、ほかの値 (`1` など) は両方
pub(crate) fn modkvb_mode() -> Option<u8> {
    static M: std::sync::OnceLock<Option<u8>> = std::sync::OnceLock::new();
    *M.get_or_init(|| match env_str!("ENOMOTO_MIP_MODKVB") {
        None | Some("0") => None,
        Some("cover") => Some(1),
        Some("gen") => Some(2),
        Some(_) => Some(3),
    })
}

thread_local! {
    /// 6 章の診断: 直前の [`modkvb_cuts`] の返した組ごとの印 (1 = k が 2、2 = 変数上下限の置き換えを含む、4 = LP のカットの行を含む、8 = CG の式)
    pub(crate) static MODKVB_TAGS: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// [`modkvb_cuts`] の記録
#[derive(Default, Debug, Clone)]
pub(crate) struct ModkStats {
    /// LP で等号の行の数
    pub tight: usize,
    /// 連立に入れた行の数
    pub sys: usize,
    /// 連立の列 (LP 値が 0 でない整数列) の数
    pub cols: usize,
    /// 変数上下限で置き換えた項の数 (連続列, 一般整数列)
    pub vbsub: (usize, usize),
    /// カットを出した k (出なければ最後に試した k)
    pub k: u32,
    /// 見つけた解 (行の組) の数
    pub sols: usize,
    /// 解の行の数の和
    pub sol_rows: usize,
    /// 長すぎて使わなかった集約行の数
    pub dense: usize,
    /// 時間の上限で止まったか
    pub timed_out: bool,
    /// `ENOMOTO_MIP_SNFMODK` の `v`: 群の要約 / 諦めたか / 元の数
    pub snf: String,
}

/// 列の置き換え
#[derive(Clone, Copy)]
enum Sub {
    /// 固定
    Fixed(f64),
    /// 連続列が単純な上下限にある (その値)
    ContAt(f64),
    /// 変数上下限 `x = d y + e (± 余り)` (余りは LP で 0)
    Vb(usize, f64, f64),
    /// 整数列を単純な上下限で (下限か, 距離)
    Int(bool, f64),
    /// 使えない (連続列が境界から離れている、両方の境界が無限)
    Off,
}

/// 値の並びを整数にする倍率 (HiGHS の `HighsIntegers::integralScale` の簡略版)。1e6 を超えるか見つからなければ `None`
fn integral_scale(vals: &[f64]) -> Option<f64> {
    let minabs = vals.iter().map(|v| v.abs()).filter(|&v| v > 1e-9).fold(f64::INFINITY, f64::min);
    if !minabs.is_finite() {
        return Some(1.0);
    }
    let isint = |t: f64| (t - t.round()).abs() <= 1e-6 * t.abs().max(1.0);
    // まず倍率 1 で整数ならそのまま (整数の行)
    if vals.iter().all(|&v| isint(v)) {
        return Some(1.0);
    }
    let mut s = 1.0 / minabs;
    for _ in 0..4 {
        let mut changed = false;
        for &v in vals {
            let t = v * s;
            if isint(t) {
                continue;
            }
            // 小数部の連分数近似で分母を探す
            let f = t - t.floor();
            let (mut h0, mut h1, mut k0, mut k1) = (0.0f64, 1.0f64, 1.0f64, 0.0f64);
            let mut r = f;
            let mut q = 0.0;
            for _ in 0..20 {
                let a = r.floor();
                let (h2, k2) = (a * h1 + h0, a * k1 + k0);
                (h0, h1, k0, k1) = (h1, h2, k1, k2);
                if k1 > 1000.0 {
                    break;
                }
                if (f - h1 / k1).abs() <= 1e-9 * k1.max(1.0) {
                    q = k1;
                    break;
                }
                let fr = r - a;
                if fr < 1e-12 {
                    break;
                }
                r = 1.0 / fr;
            }
            if q < 1.5 {
                return None;
            }
            s *= q;
            changed = true;
            if s > 1e6 {
                return None;
            }
        }
        if !changed {
            break;
        }
    }
    if s > 1e6 || !vals.iter().all(|&v| isint(v * s)) {
        return None;
    }
    Some(s)
}

/// GF(k) の逆元
fn inv_mod(a: u32, k: u32) -> u32 {
    (1..k).find(|&b| (a * b) % k == 1).unwrap_or(1)
}

/// 連立の行 (LP の行 1 本の、置き換えた空間の整数の形)
struct SysRow {
    /// LP の行の番号
    row: usize,
    /// 向き (+1 は `<= 上限`、-1 は `>= 下限` を反転)
    sg: f64,
    /// 整数にした倍率
    scale: f64,
    /// (列, 整数の係数)
    ents: Vec<(usize, i64)>,
    rhs: i64,
    /// 変数上下限で置き換えた項の数 (連続列, 一般整数列、診断用)
    vbc: u32,
    vbi: u32,
    /// 変数上下限で置き換えた項があるか (6 章の診断)
    vbf: bool,
}

/// 簡単な擬似乱数 (xorshift)
fn xs(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

/// GF(k) の消去で、列が全部消えて右辺が 0 でない行の組 (連立の行の番号, 重み 1..k-1) を探す
fn gfk_solutions(sys: &[SysRow], k: u32, ncol: usize, max_sols: usize, max_comb: usize, passes: usize, deadline: std::time::Instant, timed_out: &mut bool) -> Vec<Vec<(usize, u32)>> {
    let kk = k as i64;
    let md = |v: i64| -> u32 { (((v % kk) + kk) % kk) as u32 };
    // mod k の行
    let rows: Vec<(Vec<(usize, u32)>, u32)> = sys.iter().map(|r| (r.ents.iter().filter_map(|&(j, a)| { let v = md(a); (v != 0).then_some((j, v)) }).collect(), md(r.rhs))).collect();
    let mut colcnt = vec![0u32; ncol];
    for (e, _) in &rows {
        for &(j, _) in e {
            colcnt[j] += 1;
        }
    }
    let inv: Vec<u32> = (0..k).map(|a| if a == 0 { 0 } else { inv_mod(a, k) }).collect();
    let mut seen: std::collections::HashSet<Vec<(usize, u32)>> = std::collections::HashSet::new();
    let mut sols: Vec<Vec<(usize, u32)>> = Vec::new();
    // 1 本だけで解になる行 (列がなく右辺が 0 でない)
    for (r, (e, b)) in rows.iter().enumerate() {
        // 1 本だけの解は消去の解と別に上限を数える (先に上限を埋めてしまわないように)
        if e.is_empty() && *b != 0 && sols.len() < max_sols / 2 {
            let s = vec![(r, 1)];
            if seen.insert(s.clone()) {
                sols.push(s);
            }
        }
    }
    let nsingle = sols.len();
    let max_sols = max_sols + nsingle;
    let cand: Vec<usize> = (0..rows.len()).filter(|&r| !rows[r].0.is_empty()).collect();
    let mut rng: u64 = 0x9e3779b97f4a7c15 ^ (k as u64);
    // 作業用
    let mut val = vec![0u32; ncol];
    let mut cw = vec![0u32; rows.len()];
    for pass in 0..passes.max(1) {
        if sols.len() >= max_sols {
            break;
        }
        let mut order = cand.clone();
        if pass == 0 {
            order.sort_by_key(|&r| (rows[r].0.len(), sys[r].ents.len(), r));
        } else {
            for i in (1..order.len()).rev() {
                let j = (xs(&mut rng) % (i as u64 + 1)) as usize;
                order.swap(i, j);
            }
        }
        // 軸: 列 -> 軸の行の番号
        let mut piv: Vec<u32> = vec![u32::MAX; ncol];
        // 軸の行 (列, 値), 右辺, 組 (連立の行, 重み)
        let mut prow: Vec<(Vec<(usize, u32)>, u32, Vec<(usize, u32)>)> = Vec::new();
        for (cnt, &r) in order.iter().enumerate() {
            if cnt % 64 == 0 && std::time::Instant::now() >= deadline {
                *timed_out = true;
                return sols;
            }
            if sols.len() >= max_sols {
                break;
            }
            let (e, b) = &rows[r];
            let mut touched: Vec<usize> = Vec::with_capacity(e.len() * 2);
            let mut heap: std::collections::BinaryHeap<std::cmp::Reverse<(u32, usize)>> = std::collections::BinaryHeap::new();
            for &(j, v) in e {
                val[j] = v;
                touched.push(j);
                if piv[j] != u32::MAX {
                    heap.push(std::cmp::Reverse((piv[j], j)));
                }
            }
            let mut rhs = *b;
            let mut ctouch: Vec<usize> = vec![r];
            cw[r] = 1;
            let mut too_big = false;
            while let Some(std::cmp::Reverse((pi, j))) = heap.pop() {
                let f = val[j];
                if f == 0 {
                    continue;
                }
                let (pe, pb, pc) = &prow[pi as usize];
                let m = k - f;
                for &(c, v) in pe {
                    let old = val[c];
                    let nv = (old + m * v) % k;
                    if old == 0 && nv != 0 {
                        touched.push(c);
                        if piv[c] != u32::MAX && piv[c] != pi {
                            heap.push(std::cmp::Reverse((piv[c], c)));
                        }
                    }
                    val[c] = nv;
                }
                rhs = (rhs + m * pb) % k;
                for &(q, w) in pc {
                    if cw[q] == 0 {
                        ctouch.push(q);
                    }
                    cw[q] = (cw[q] + m * w) % k;
                }
                if ctouch.len() > 4 * max_comb {
                    too_big = true;
                    break;
                }
            }
            let mut rest: Vec<(usize, u32)> = Vec::new();
            for &j in &touched {
                if val[j] != 0 {
                    rest.push((j, val[j]));
                    val[j] = 0;
                }
            }
            rest.sort_unstable();
            rest.dedup_by_key(|t| t.0);
            let mut comb: Vec<(usize, u32)> = Vec::new();
            for &q in &ctouch {
                if cw[q] != 0 {
                    comb.push((q, cw[q]));
                    cw[q] = 0;
                }
            }
            comb.sort_unstable();
            comb.dedup_by_key(|t| t.0);
            if too_big || comb.len() > max_comb {
                continue;
            }
            if rest.is_empty() {
                if rhs != 0 && seen.insert(comb.clone()) {
                    sols.push(comb);
                }
                continue;
            }
            // 新しい軸: 出る数の一番少ない列
            let &(pj, pv) = rest.iter().min_by_key(|&&(j, _)| (colcnt[j], j)).unwrap();
            let iv = inv[pv as usize];
            let pe: Vec<(usize, u32)> = rest.iter().map(|&(j, v)| (j, (v * iv) % k)).collect();
            let pc: Vec<(usize, u32)> = comb.iter().map(|&(q, w)| (q, (w * iv) % k)).collect();
            piv[pj] = prow.len() as u32;
            prow.push((pe, (rhs * iv) % k, pc));
        }
    }
    sols
}

/// 試作 `ENOMOTO_MIP_MODKVB` の分離。`vars` は [`super::separation`] の分離と同じ (列 0..n は構造変数、n..n+m は LP の行の
/// 活動量で、境界は行の上下限)、`lp_rows` は LP の行 (最初の `m_orig` 本が元の行)。返り値は (解ごとのカット, 記録)
pub(crate) fn modkvb_cuts(vars: &CutVars, lp_rows: &[Vec<(usize, f64)>], n: usize, m_orig: usize, mode: u8, flow_cover: bool, deadline: std::time::Instant) -> (Vec<Vec<RawCut>>, ModkStats) {
    const FT: f64 = 1e-6;
    // 診断 (どの部分が効くかを分ける比べ、`analysis/modkvb_20261107.md` の 5 章): `ENOMOTO_MIP_MODKVB_NOVB` = 変数上下限で置き換えない
    // (連続列は単純な上下限にあるときだけ、一般整数列は単純な上下限で)、`_NOCUTROWS` = LP のカットの行を使わない (元の行だけ)、`_K2` = k = 2 だけ
    let no_vb = env_str!("ENOMOTO_MIP_MODKVB_NOVB").is_some();
    let no_cutrows = env_str!("ENOMOTO_MIP_MODKVB_NOCUTROWS").is_some();
    let only_k2 = env_str!("ENOMOTO_MIP_MODKVB_K2").is_some();
    let mut st = ModkStats::default();
    let m = lp_rows.len();
    let x = vars.x;
    if vars.lo.len() < n + m || x.len() < n + m {
        return (Vec::new(), st);
    }
    // 列の置き換え
    let mut sub: Vec<Sub> = Vec::with_capacity(n);
    for j in 0..n {
        let (l, u) = (vars.lo[j], vars.up[j]);
        if u - l < 1e-9 {
            sub.push(Sub::Fixed(l));
            continue;
        }
        let dl = if l.is_finite() { (x[j] - l).max(0.0) } else { f64::INFINITY };
        let du = if u.is_finite() { (u - x[j]).max(0.0) } else { f64::INFINITY };
        if !vars.is_int[j] {
            let simple = dl.min(du);
            if simple <= FT {
                sub.push(Sub::ContAt(if dl <= du { l } else { u }));
                continue;
            }
            // 変数上下限 (連続列)
            let mut best: Option<(f64, usize, f64, f64, bool)> = None;
            if let Some(vb) = vars.vb {
                if j < vb.vub.len() {
                    for &(y, d, e) in &vb.vub[j] {
                        let dist = d * x[y] + e - x[j];
                        if dist >= -FT && best.is_none_or(|b| dist < b.0) {
                            best = Some((dist.max(0.0), y, d, e, true));
                        }
                    }
                    for &(y, d, e) in &vb.vlb[j] {
                        let dist = x[j] - (d * x[y] + e);
                        if dist >= -FT && best.is_none_or(|b| dist < b.0) {
                            best = Some((dist.max(0.0), y, d, e, false));
                        }
                    }
                }
            }
            match best {
                Some((dist, y, d, e, _)) if dist <= FT && !no_vb => sub.push(Sub::Vb(y, d, e)),
                _ => sub.push(Sub::Off),
            }
            continue;
        }
        if !dl.is_finite() && !du.is_finite() {
            sub.push(Sub::Off);
            continue;
        }
        // 一般整数列の変数上下限 (LP 値が整数で単純な上下限の内側、ちょうど効いているもの)
        if !no_vb && u - l > 1.5 && dl > FT && du > FT && (x[j] - x[j].round()).abs() < FT {
            let mut found: Option<Sub> = None;
            if let Some(vb) = vars.vb {
                if j < vb.ivub.len() {
                    for &(y, d, e) in &vb.ivub[j] {
                        if (d * x[y] + e - x[j]).abs() <= FT {
                            found = Some(Sub::Vb(y, d, e));
                            break;
                        }
                    }
                    if found.is_none() {
                        for &(y, d, e) in &vb.ivlb[j] {
                            if (x[j] - (d * x[y] + e)).abs() <= FT {
                                found = Some(Sub::Vb(y, d, e));
                                break;
                            }
                        }
                    }
                }
            }
            if let Some(s) = found {
                sub.push(s);
                continue;
            }
        }
        // 近い方の単純な上下限 (同点なら上限)
        if dl < du { sub.push(Sub::Int(true, dl)) } else { sub.push(Sub::Int(false, du)) }
    }
    // 連立
    let max_int_len = 1000 + n / 10;
    let mut acc = vec![0.0f64; n];
    let mut touched: Vec<usize> = Vec::new();
    let mut sys: Vec<SysRow> = Vec::new();
    let (mut nvc, mut nvi) = (0usize, 0usize);
    'rows: for (i, row) in lp_rows.iter().enumerate() {
        if no_cutrows && i >= m_orig {
            break;
        }
        let (rl, ru, ra) = (vars.lo[n + i], vars.up[n + i], x[n + i]);
        let sg = if ru.is_finite() && ru - ra <= FT {
            1.0
        } else if rl.is_finite() && ra - rl <= FT {
            -1.0
        } else {
            continue;
        };
        st.tight += 1;
        if row.iter().any(|&(j, _)| j >= n || matches!(sub[j], Sub::Off)) {
            continue;
        }
        let mut rhs = if sg > 0.0 { ru } else { -rl };
        let (mut c1, mut c2) = (0usize, 0usize);
        for &(j, a) in row {
            let a = sg * a;
            match sub[j] {
                Sub::Fixed(v) | Sub::ContAt(v) => rhs -= a * v,
                Sub::Vb(y, d, e) => {
                    rhs -= a * e;
                    if acc[y] == 0.0 {
                        touched.push(y);
                    }
                    acc[y] += a * d;
                    if acc[y] == 0.0 {
                        acc[y] = 1e-300;
                    }
                    if vars.is_int[j] { c2 += 1 } else { c1 += 1 }
                }
                Sub::Int(..) => {
                    if acc[j] == 0.0 {
                        touched.push(j);
                    }
                    acc[j] += a;
                    if acc[j] == 0.0 {
                        acc[j] = 1e-300;
                    }
                }
                Sub::Off => unreachable!(),
            }
        }
        // 置き換えの相手の列と整数列を単純な上下限で
        let mut ents: Vec<(usize, f64)> = Vec::new();
        let mut bad = false;
        for &j in &touched {
            let c = acc[j];
            acc[j] = 0.0;
            if bad || c.abs() <= 1e-9 {
                continue;
            }
            let (l, u) = (vars.lo[j], vars.up[j]);
            if u - l < 1e-9 {
                rhs -= c * l;
                continue;
            }
            let (lower, dist) = match sub[j] {
                Sub::Int(lw, d) => (lw, d),
                // 変数上下限の相手が整数列として置き換えの対象でない場合: 近い方の境界
                _ => {
                    let (dl, du) = (x[j] - l, u - x[j]);
                    if dl < du { (true, dl.max(0.0)) } else { (false, du.max(0.0)) }
                }
            };
            let (cc, b) = if lower { (c, l) } else { (-c, u) };
            if !b.is_finite() {
                bad = true;
                continue;
            }
            rhs -= c * b;
            if dist > FT && vars.is_int[j] {
                ents.push((j, cc));
            }
        }
        if bad {
            touched.clear();
            continue 'rows;
        }
        touched.clear();
        if ents.len() > max_int_len || (ents.is_empty() && rhs.abs() <= 1e-9) {
            continue;
        }
        let mut sv: Vec<f64> = ents.iter().map(|t| t.1).collect();
        if rhs.abs() > 1e-9 {
            sv.push(rhs);
        }
        let Some(scale) = integral_scale(&sv) else { continue };
        let ents: Vec<(usize, i64)> = ents.iter().map(|&(j, a)| (j, (a * scale).round() as i64)).collect();
        let irhs = (rhs * scale).round() as i64;
        nvc += c1;
        nvi += c2;
        sys.push(SysRow { row: i, sg, scale, ents, rhs: irhs, vbc: c1 as u32, vbi: c2 as u32 , vbf: c1 + c2 > 0 });
    }
    st.sys = sys.len();
    st.vbsub = (nvc, nvi);
    {
        let mut used = vec![false; n];
        for r in &sys {
            for &(j, _) in &r.ents {
                used[j] = true;
            }
        }
        st.cols = used.iter().filter(|&&b| b).count();
    }
    if sys.is_empty() || sys.iter().all(|r| r.rhs == 0) {
        MODKVB_TAGS.with(|t| t.borrow_mut().clear());
        return (Vec::new(), st);
    }
    let max_sols = tunable!("ENOMOTO_T_MIP_MODKVB_SOLS", 300usize, usize);
    let max_comb = tunable!("ENOMOTO_T_MIP_MODKVB_COMB", 40usize, usize);
    let passes = tunable!("ENOMOTO_T_MIP_MODKVB_PASSES", 3usize, usize);
    let max_agg_len = tunable!("ENOMOTO_T_MIP_MODKVB_MAXLEN", 0usize, usize);
    let max_agg_len = if max_agg_len > 0 { max_agg_len } else { 1000 + n / 10 };
    let iv = env_str!("ENOMOTO_MIP_MODKVB_IV").and_then(|v| v.parse::<u8>().ok()).unwrap_or(3);
    let dump = env_str!("ENOMOTO_MIP_MODKVB_DUMP").is_some();
    let mut out: Vec<Vec<RawCut>> = Vec::new();
    let mut tags: Vec<u8> = Vec::new();
    let cg_mode: u8 = match env_str!("ENOMOTO_MIP_MODKVB_CG") { None | Some("0") => 0, Some("both") => 2, Some(_) => 1 };
    let mut aggv = vec![0.0f64; n + m];
    let mut atouch: Vec<usize> = Vec::new();
    // 試作 `ENOMOTO_MIP_SNFMODK` の `v`: GF(k) の解の代わりに、連立の群 (Smith 標準形の対角化) の元を分母ごとに試す
    // (`k` も付けたら、群の元でカットが出なければ GF(k) も)。重み w / 分母 は GF(k) の解の w / k と同じ式で集約する
    let mut plan: Vec<(u32, Option<Vec<Vec<(usize, u32)>>>)> = Vec::new();
    if super::snfmodk::snfmodk_has('v') {
        let mat: Vec<Vec<(usize, i64)>> = sys.iter().map(|r| r.ents.clone()).collect();
        let gr = super::snfmodk::snf_group(&mat, n, tunable!("ENOMOTO_T_MIP_SNFMODK_CORE", 250_000usize, usize), 20_000_000, deadline);
        let rhs: Vec<i128> = sys.iter().map(|r| r.rhs as i128).collect();
        let els = super::snfmodk::group_elements(&gr, &rhs, max_sols, 65535);
        st.snf = format!("{}/{}/{}", super::snfmodk::group_summary(&gr), gr.gave_up, els.len());
        let mut by_den: Vec<(u32, Vec<Vec<(usize, u32)>>)> = Vec::new();
        for e in els.iter().filter(|e| e.w.len() <= 4 * max_comb) {
            let d = e.den as u32;
            let sol: Vec<(usize, u32)> = e.w.iter().map(|&(i, w)| (i, w as u32)).collect();
            match by_den.iter_mut().find(|t| t.0 == d) {
                Some(t) => t.1.push(sol),
                None => by_den.push((d, vec![sol])),
            }
        }
        plan.extend(by_den.into_iter().map(|(d, s)| (d, Some(s))));
    }
    if !super::snfmodk::snfmodk_has('v') || super::snfmodk::snfmodk_has('k') {
        if only_k2 {
            plan.push((2, None));
        } else {
            plan.extend([2u32, 3, 5, 7].map(|k| (k, None)));
        }
    }
    let mut snf_found = false;
    for (k, pre) in plan {
        st.k = k;
        if std::time::Instant::now() >= deadline {
            st.timed_out = true;
            break;
        }
        let is_snf = pre.is_some();
        if !is_snf && snf_found {
            break;
        }
        let sols = match pre {
            Some(s) => s,
            None => gfk_solutions(&sys, k, n, max_sols, max_comb, passes, deadline, &mut st.timed_out),
        };
        st.sols += sols.len();
        let n0 = out.len();
        for (si, sol) in sols.iter().enumerate() {
            if std::time::Instant::now() >= deadline {
                st.timed_out = true;
                break;
            }
            st.sol_rows += sol.len();
            let mut cs: Vec<RawCut> = Vec::new();
            let has_vb = sol.iter().any(|&(q, _)| sys[q].vbf);
            let has_cut = sol.iter().any(|&(q, _)| sys[q].row >= m_orig);
            let mut tag: u8 = (k == 2) as u8 | ((has_vb as u8) << 1) | ((has_cut as u8) << 2);
            // 6 章の診断 `ENOMOTO_MIP_MODKVB_CG` (=1 は CG の式だけ、=both は CG の式と従来の両方): 重み ((k-1) w mod k) / k の非負の乗数で
            // 向きをそろえた行を足し、単純な上下限で x' >= 0 に移して係数と右辺を切り下げる (Chvátal-Gomory)。変数上下限の置き換えを含む解と、
            // 係数が負の連続列 (x' >= 0 で落とせない) が残るものは作らない
            if cg_mode > 0 && !has_vb {
                if let Some(c) = cg_cut(vars, lp_rows, n, &sys, sol, k, &sub) {
                    cs.push(c);
                    tag |= 8;
                }
            }
            if cg_mode == 1 {
                if !cs.is_empty() {
                    out.push(cs);
                    tags.push(tag);
                }
                continue;
            }
            let (mut ncov, mut ngen, mut alen) = (0usize, 0usize, 0usize);
            let forms: &[bool] = if k == 2 { &[true] } else { &[false, true] };
            for &negform in forms {
                // negform: 重み w / k で足して符号を反転、そうでなければ ((k-1) w mod k) / k
                for &(q, w) in sol {
                    let r = &sys[q];
                    let lam = if negform { -(w as f64) / k as f64 } else { (((k - 1) * w) % k) as f64 / k as f64 } * r.scale * r.sg;
                    if lam == 0.0 {
                        continue;
                    }
                    for &(j, a) in &lp_rows[r.row] {
                        if aggv[j] == 0.0 {
                            atouch.push(j);
                        }
                        aggv[j] += lam * a;
                        if aggv[j] == 0.0 {
                            aggv[j] = 1e-300;
                        }
                    }
                    let s = n + r.row;
                    if aggv[s] == 0.0 {
                        atouch.push(s);
                    }
                    aggv[s] -= lam;
                    if aggv[s] == 0.0 {
                        aggv[s] = 1e-300;
                    }
                }
                let mut base: Vec<(usize, f64)> = Vec::with_capacity(atouch.len());
                for &j in &atouch {
                    if aggv[j].abs() > 1e-9 {
                        base.push((j, aggv[j]));
                    }
                    aggv[j] = 0.0;
                }
                atouch.clear();
                // 密な集約行は使わない (nw04 で 1 本 4 万非零のカットになり根が 5 倍遅くなった。HiGHS の連立の行の長さの上限と同じ式)
                if base.is_empty() || base.len() > max_agg_len {
                    st.dense += 1;
                    continue;
                }
                base.sort_unstable_by_key(|t| t.0);
                if mode & 1 != 0 {
                    if let Some(c) = super::cuts::vb_cover_cut(vars, &base, 0.0) {
                        cs.push(c);
                        ncov += 1;
                    }
                }
                if mode & 2 != 0 {
                    let g = super::cuts::with_intvub_mode(iv, || super::cuts::generate_cuts(vars, &base, 0.0, flow_cover, true));
                    ngen += g.len();
                    cs.extend(g);
                }
                alen = alen.max(base.len());
            }
            // 診断 (`ENOMOTO_MIP_MODKVB_DUMP`): カットの出た解の形 (k、行の数、そのうち LP のカットの行、変数上下限で置き換えた
            // 項のある行・項の数 (連続列, 一般整数列)、cover・generate_cuts のカットの数、集約行の長さ)
            if dump && !cs.is_empty() {
                let rows: Vec<&SysRow> = sol.iter().map(|&(q, _)| &sys[q]).collect();
                eprintln!(
                    "MODKVB_SOL k {k} rows {} cutrows {} rows_vbc {} rows_vbi {} vbc {} vbi {} cover {ncov} gen {ngen} agglen {alen}",
                    rows.len(),
                    rows.iter().filter(|r| r.row >= m_orig).count(),
                    rows.iter().filter(|r| r.vbc > 0).count(),
                    rows.iter().filter(|r| r.vbi > 0).count(),
                    rows.iter().map(|r| r.vbc).sum::<u32>(),
                    rows.iter().map(|r| r.vbi).sum::<u32>()
                );
            }
            if !cs.is_empty() {
                out.push(cs);
                tags.push(tag);
            }
        }
        if is_snf {
            snf_found |= out.len() > n0;
            if st.timed_out {
                break;
            }
            continue;
        }
        if out.len() > n0 || st.timed_out {
            break;
        }
    }
    MODKVB_TAGS.with(|t| *t.borrow_mut() = tags);
    (out, st)
}

/// 6 章の診断: 解 `sol` の mod-k の CG カット (非負の乗数 ((k-1) w mod k) / k で向きをそろえた LP の行を足し、単純な上下限で
/// `x' >= 0` に移して係数と右辺を切り下げる)。構造変数だけの式で返す。置き換えが使えない列があれば `None`
fn cg_cut(vars: &CutVars, lp_rows: &[Vec<(usize, f64)>], n: usize, sys: &[SysRow], sol: &[(usize, u32)], k: u32, sub: &[Sub]) -> Option<RawCut> {
    let mut acc: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
    let mut beta = 0.0f64;
    for &(q, w) in sol {
        let r = &sys[q];
        let u = (((k - 1) * w) % k) as f64 / k as f64 * r.scale;
        if u == 0.0 {
            continue;
        }
        let b = if r.sg > 0.0 { vars.up[n + r.row] } else { -vars.lo[n + r.row] };
        if !b.is_finite() {
            return None;
        }
        beta += u * b;
        for &(j, a) in &lp_rows[r.row] {
            *acc.entry(j).or_insert(0.0) += u * r.sg * a;
        }
    }
    let fl = |v: f64| (v + 1e-9 * v.abs().max(1.0)).floor();
    let mut coefs: Vec<(usize, f64)> = Vec::new();
    let mut rhs_const = 0.0f64;
    let mut terms: Vec<(usize, bool, f64)> = Vec::new();
    for (&j, &c) in &acc {
        if c.abs() <= 1e-12 {
            continue;
        }
        match sub[j] {
            Sub::Fixed(v) => beta -= c * v,
            Sub::ContAt(v) => {
                let lower = v == vars.lo[j];
                // x = l + x' (係数 c) か x = u - x' (係数 -c)。係数が負なら x' >= 0 で落とせない
                let cc = if lower { c } else { -c };
                if cc < -1e-9 {
                    return None;
                }
                beta -= c * v;
            }
            Sub::Int(lower, _) => {
                let (cc, b) = if lower { (c, vars.lo[j]) } else { (-c, vars.up[j]) };
                if !b.is_finite() {
                    return None;
                }
                beta -= c * b;
                terms.push((j, lower, cc));
            }
            Sub::Vb(..) | Sub::Off => return None,
        }
    }
    let fb = fl(beta);
    for (j, lower, cc) in terms {
        let f = fl(cc);
        if f == 0.0 {
            continue;
        }
        // f x' : 下限なら f (x - l)、上限なら f (u - x)
        if lower {
            coefs.push((j, f));
            rhs_const += f * vars.lo[j];
        } else {
            coefs.push((j, -f));
            rhs_const -= f * vars.up[j];
        }
    }
    if coefs.is_empty() {
        return None;
    }
    Some(RawCut { coefs, rhs: fb + rhs_const })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integral_scale_basic() {
        assert_eq!(integral_scale(&[1.0, 2.0, -3.0]), Some(1.0));
        let s = integral_scale(&[0.5, 1.5]).unwrap();
        assert!((0.5 * s - (0.5 * s).round()).abs() < 1e-9 && (1.5 * s - (1.5 * s).round()).abs() < 1e-9);
        let s = integral_scale(&[1.0 / 3.0, 0.5]).unwrap();
        assert!((s / 6.0 - (s / 6.0).round()).abs() < 1e-9, "{s}");
        assert!(integral_scale(&[1.0, std::f64::consts::PI]).is_none());
    }

    #[test]
    fn gfk_finds_pair() {
        // x0 の係数が逆の 2 本: x0 <= 3, -x0 <= -4 → mod 2 では右辺 3 - 4 = -1 で奇数
        let sys = vec![SysRow { row: 0, sg: 1.0, scale: 1.0, ents: vec![(0, 1)], rhs: 3, vbc: 0, vbi: 0, vbf: false }, SysRow { row: 1, sg: 1.0, scale: 1.0, ents: vec![(0, -1)], rhs: -4, vbc: 0, vbi: 0, vbf: false }];
        let mut to = false;
        let far = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let s = gfk_solutions(&sys, 2, 1, 10, 10, 1, far, &mut to);
        assert_eq!(s, vec![vec![(0, 1), (1, 1)]]);
        // mod 3: w0 - w1 ≡ 0, 3 w0 - 4 w1 ≡ -w ≢ 0
        let s = gfk_solutions(&sys, 3, 1, 10, 10, 1, far, &mut to);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].len(), 2);
        assert_eq!(s[0][0].1, s[0][1].1);
    }

    struct Rng(u64);
    impl Rng {
        fn f(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
        fn i(&mut self, lo: i64, hi: i64) -> i64 {
            lo + (self.f() * (hi - lo + 1) as f64).floor() as i64
        }
    }

    /// 列の値の全ての組 (整数列は上下限の中の整数、連続列は 0.5 刻み) のうち、全ての行を満たすもの
    fn feasible_points(lo: &[f64], up: &[f64], is_int: &[bool], rows: &[Vec<(usize, f64)>], rlo: &[f64], rup: &[f64]) -> Vec<Vec<f64>> {
        let n = lo.len();
        let mut out = Vec::new();
        let mut q: Vec<f64> = lo.to_vec();
        loop {
            if rows.iter().zip(rlo.iter().zip(rup)).all(|(r, (&l, &u))| {
                let a: f64 = r.iter().map(|&(j, c)| c * q[j]).sum();
                a >= l - 1e-9 && a <= u + 1e-9
            }) {
                out.push(q.clone());
            }
            let mut k = 0;
            while k < n {
                let st = if is_int[k] { 1.0 } else { 0.5 };
                if q[k] + st <= up[k] + 1e-9 {
                    q[k] += st;
                    break;
                }
                q[k] = lo[k];
                k += 1;
            }
            if k == n {
                return out;
            }
        }
    }

    /// 乱数の小さい問題 (x0 と変数上下限つきの一般整数列 w・連続列 c、0-1 列 z、x0 を含むデータの行を LP 点で等号にしたもの) で、
    /// [`modkvb_cuts`] のカット (論理変数を含む) が全ての点 (連続列は 0.5 刻み) で成り立つことを総当たりで確かめる
    #[test]
    fn modkvb_cuts_valid() {
        use super::super::cuts::VarBounds;
        let mut rng = Rng(1234567891);
        let (mut ncuts, mut nfeas, mut nslack) = (0usize, 0usize, 0usize);
        let far = std::time::Instant::now() + std::time::Duration::from_secs(3600);
        for _ in 0..3000 {
            let nw = rng.i(1, 3) as usize;
            let nc = rng.i(0, 1) as usize;
            // 列: 0 = x0, 1..=nw = w, 次の nw = z (w の相手), 次の nc = c (連続), 次の nc = zc (c の相手)
            let n = 1 + 2 * nw + 2 * nc;
            let mut lo = vec![0.0; n];
            let mut up = vec![1.0; n];
            let mut is_int = vec![true; n];
            lo[0] = -(rng.i(2, 3) as f64);
            up[0] = 3.0;
            for k in 1..=nw {
                lo[k] = -(rng.i(1, 3) as f64);
                up[k] = rng.i(1, 3) as f64;
            }
            let mut x = vec![0.0; n];
            x[0] = rng.i(-2, 1) as f64 + 0.25 + 0.5 * rng.f();
            let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
            let (mut rlo, mut rup) = (Vec::new(), Vec::new());
            for k in 1..=nw {
                let z = nw + k;
                let (d, d2) = (up[k] - rng.i(0, 1) as f64, -lo[k] - rng.i(0, 1) as f64);
                rows.push(vec![(k, 1.0), (z, -d)]);
                rlo.push(f64::NEG_INFINITY);
                rup.push(0.0);
                rows.push(vec![(k, -1.0), (z, -d2)]);
                rlo.push(f64::NEG_INFINITY);
                rup.push(0.0);
                if rng.f() < 0.3 {
                    x[z] = 0.2 + 0.6 * rng.f();
                    x[k] = (rng.f() * 2.0 - 1.0) * d.min(d2) * x[z];
                } else if rng.f() < 0.3 {
                    x[z] = 1.0;
                    x[k] = rng.i(-(d2 as i64), d as i64) as f64;
                }
            }
            for t in 0..nc {
                let (c, zc) = (1 + 2 * nw + t, 1 + 2 * nw + nc + t);
                is_int[c] = false;
                up[c] = 2.0;
                // c <= 2 zc
                rows.push(vec![(c, 1.0), (zc, -2.0)]);
                rlo.push(f64::NEG_INFINITY);
                rup.push(0.0);
                x[zc] = [0.0, 0.5, 1.0][rng.i(0, 2) as usize];
                // 多くは変数上限にちょうど (c = 2 zc)、ときどき 0
                x[c] = if rng.f() < 0.7 { 2.0 * x[zc] } else { 0.0 };
            }
            let nd = rng.i(2, 4) as usize;
            for t in 0..nd {
                let mut r = vec![(0, if t % 2 == 0 { 1.0 } else { -(rng.i(1, 2) as f64) })];
                for k in 1..=nw {
                    if rng.f() < 0.7 {
                        r.push((k, [-1.0, 1.0, 2.0, -2.0, 3.0][rng.i(0, 4) as usize]));
                    }
                }
                for t2 in 0..nc {
                    if rng.f() < 0.6 {
                        r.push((1 + 2 * nw + t2, [-1.0, 1.0, 0.5][rng.i(0, 2) as usize]));
                    }
                }
                let a: f64 = r.iter().map(|&(j, c)| c * x[j]).sum();
                rows.push(r);
                if rng.f() < 0.5 {
                    rlo.push(f64::NEG_INFINITY);
                    rup.push(a);
                } else if rng.f() < 0.8 {
                    rlo.push(a);
                    rup.push(f64::INFINITY);
                } else {
                    rlo.push(a);
                    rup.push(a);
                }
            }
            let feas = feasible_points(&lo, &up, &is_int, &rows, &rlo, &rup);
            nfeas += feas.len();
            let mut vb = VarBounds::from_rows(n, &is_int, &rows, &rlo, &rup);
            vb.add_int_vbs(&is_int, &lo, &up, &rows, &rlo, &rup, &[]);
            // 論理変数 (行の活動量) を足した列
            let m = rows.len();
            let (mut elo, mut eup, mut eint, mut ex) = (lo.clone(), up.clone(), is_int.clone(), x.clone());
            for i in 0..m {
                let integral = rows[i].iter().all(|&(j, a)| is_int[j] && (a - a.round()).abs() < 1e-9);
                let (l, u) = if integral { ((rlo[i] - 1e-6).ceil(), (rup[i] + 1e-6).floor()) } else { (rlo[i], rup[i]) };
                elo.push(l);
                eup.push(u);
                eint.push(integral);
                ex.push(rows[i].iter().map(|&(j, c)| c * x[j]).sum());
            }
            let vars = CutVars { lo: &elo, up: &eup, is_int: &eint, x: &ex, vb: Some(&vb), imp: None };
            let (groups, _st) = modkvb_cuts(&vars, &rows, n, rows.len(), 3, true, far);
            for cut in groups.into_iter().flatten() {
                ncuts += 1;
                if cut.coefs.iter().any(|&(k, _)| k >= n) {
                    nslack += 1;
                }
                for q in &feas {
                    let a: f64 = cut.coefs.iter().map(|&(k, c)| c * if k < n { q[k] } else { rows[k - n].iter().map(|&(j, cc)| cc * q[j]).sum::<f64>() }).sum();
                    assert!(a <= cut.rhs + 1e-7 * (1.0 + cut.rhs.abs()), "point {q:?} cut off by {:?} <= {} (rows {rows:?} [{rlo:?}, {rup:?}], x {x:?}, lo {lo:?} up {up:?} int {is_int:?})", cut.coefs, cut.rhs);
                }
            }
        }
        eprintln!("modkvb_cuts_valid: cuts {ncuts} (with slacks {nslack}), feasible points {nfeas}");
        assert!(ncuts > 300 && nfeas > 1000, "cuts {ncuts}, feasible points {nfeas}");
    }
}
