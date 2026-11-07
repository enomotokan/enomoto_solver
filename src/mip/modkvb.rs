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
    /// 時間の上限で止まったか
    pub timed_out: bool,
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
/// 活動量で、境界は行の上下限)、`lp_rows` は LP の行。返り値は (解ごとのカット, 記録)
pub(crate) fn modkvb_cuts(vars: &CutVars, lp_rows: &[Vec<(usize, f64)>], n: usize, mode: u8, flow_cover: bool, deadline: std::time::Instant) -> (Vec<Vec<RawCut>>, ModkStats) {
    const FT: f64 = 1e-6;
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
                Some((dist, y, d, e, _)) if dist <= FT => sub.push(Sub::Vb(y, d, e)),
                _ => sub.push(Sub::Off),
            }
            continue;
        }
        if !dl.is_finite() && !du.is_finite() {
            sub.push(Sub::Off);
            continue;
        }
        // 一般整数列の変数上下限 (LP 値が整数で単純な上下限の内側、ちょうど効いているもの)
        if u - l > 1.5 && dl > FT && du > FT && (x[j] - x[j].round()).abs() < FT {
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
        sys.push(SysRow { row: i, sg, scale, ents, rhs: irhs });
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
        return (Vec::new(), st);
    }
    let max_sols = tunable!("ENOMOTO_T_MIP_MODKVB_SOLS", 300usize, usize);
    let max_comb = tunable!("ENOMOTO_T_MIP_MODKVB_COMB", 40usize, usize);
    let passes = tunable!("ENOMOTO_T_MIP_MODKVB_PASSES", 3usize, usize);
    let iv = env_str!("ENOMOTO_MIP_MODKVB_IV").and_then(|v| v.parse::<u8>().ok()).unwrap_or(3);
    let mut out: Vec<Vec<RawCut>> = Vec::new();
    let mut aggv = vec![0.0f64; n + m];
    let mut atouch: Vec<usize> = Vec::new();
    for k in [2u32, 3, 5, 7] {
        st.k = k;
        if std::time::Instant::now() >= deadline {
            st.timed_out = true;
            break;
        }
        let sols = gfk_solutions(&sys, k, n, max_sols, max_comb, passes, deadline, &mut st.timed_out);
        st.sols += sols.len();
        let n0 = out.len();
        for (si, sol) in sols.iter().enumerate() {
            if si % 16 == 0 && std::time::Instant::now() >= deadline {
                st.timed_out = true;
                break;
            }
            st.sol_rows += sol.len();
            let mut cs: Vec<RawCut> = Vec::new();
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
                if base.is_empty() {
                    continue;
                }
                base.sort_unstable_by_key(|t| t.0);
                if mode & 1 != 0 {
                    if let Some(c) = super::cuts::vb_cover_cut(vars, &base, 0.0) {
                        cs.push(c);
                    }
                }
                if mode & 2 != 0 {
                    cs.extend(super::cuts::with_intvub_mode(iv, || super::cuts::generate_cuts(vars, &base, 0.0, flow_cover, true)));
                }
            }
            if !cs.is_empty() {
                out.push(cs);
            }
        }
        if out.len() > n0 || st.timed_out {
            break;
        }
    }
    (out, st)
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
        let sys = vec![SysRow { row: 0, sg: 1.0, scale: 1.0, ents: vec![(0, 1)], rhs: 3 }, SysRow { row: 1, sg: 1.0, scale: 1.0, ents: vec![(0, -1)], rhs: -4 }];
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
}
