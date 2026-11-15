//! 試作 `ENOMOTO_MIP_SNFMODK` (既定では使わない): Smith 標準形 (の対角化) で CG カットの重みの群全体を見る mod-k の一般化
//! (`analysis/snfmodk_20261107.md`)。
//!
//! mod-k の分離 (Caprara–Fischetti 1996、Caprara–Fischetti–Letchford 2000) は、余裕 0 の行と LP 値が 0 でない (境界から離れた) 整数列の
//! 係数行列 A_N について `u A_N ≡ 0 (mod 1)` となる重み u (`u = w / k`) を GF(k) で探す。こうした u の全体は (A_N の核の部分を除いて)
//! 有限アーベル群 G で、整数の行・列の変形で `L A_N R = diag(e_i)` (L・R はユニモジュラ) にすれば `G ≅ ⊕ Z/e_i`、生成元は
//! `g_i = L_i / e_i` (L の行 i、mod 1) と丸ごと分かる (Gomory の群緩和と同じ理論)。素数 k の決め打ちでは分母 4・6 などの重みが
//! 見つからない (例: Z/6 ≅ Z/2 × Z/3 で、mod 2 の解と mod 3 の解を組むと違反の上限 5/6)。
//!
//! u で行を足して切り下げた CG カット `floor(u A') x' <= floor(u b')` (x' は境界で置き換えた変数、A' はその係数) の LP 点での違反は、
//! 使う行が余裕 0 で N の外の列の x' が 0 なので、ちょうど `frac(u b')`。`u ↦ frac(u b')` は群の準同型 χ (`χ(g_i) = c_i / e_i`、
//! `c_i = L_i b' mod e_i`) なので、違反の大きい元は χ の値から選べる。
//!
//! - [`snf_group`]: 整数の行列 (行ごとの疎な形) を受け取り、対角化して群 (e_i > 1 の因子と生成元) を返す。まず ±1 の要素を軸にした
//!   疎な消去 (行の組み合わせを持ち歩く)、残った核を密な i128 の対角化。算術はすべて溢れを検査し、溢れたら諦める。
//!   この関数は行列だけを受け取るので、zerohalf の連立 ([`snfmodk_cg_cuts`]) にも、変数上下限で置き換えた空間の連立
//!   (`ENOMOTO_MIP_MODKVB`、[`super::modkvb`]) にも使える。
//! - [`group_elements`]: 生成元 1 つの倍数と、χ の位数が増える 2 つの生成元の組 (中国剰余定理での組み合わせ) から、違反の大きい元を並べる。
//! - [`snfmodk_cg_cuts`]: zerohalf と同じ行・列の選び方 (整数列だけ・整数係数・整数右辺の元の行、LP 値に近い方の境界で置き換え) で、
//!   余裕 0 の行だけの連立から CG カットを厳密な整数の算術で作る。

use super::cuts::RawCut;

/// 試作 `ENOMOTO_MIP_SNFMODK` の文字 (`None` = 使わない)。`c` (または `1`): zerohalf の連立からの CG カット ([`snfmodk_cg_cuts`])。
/// `v`: `ENOMOTO_MIP_MODKVB` の連立 (変数上下限の置き換え) で GF(k) の代わりに群の元を使う (MODKVB も付けたときだけ)。
/// `k`: `v` のとき GF(k) の解も残す (群の元を先に試し、カットが出なければ GF(k))
pub(crate) fn snfmodk_mode() -> Option<&'static str> {
    static M: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    M.get_or_init(|| match std::env::var("ENOMOTO_MIP_SNFMODK") {
        Ok(v) if v != "0" => Some(if v == "1" { "c".to_string() } else { v }),
        _ => None,
    })
    .as_deref()
}

pub(crate) fn snfmodk_has(c: char) -> bool {
    snfmodk_mode().is_some_and(|m| m.contains(c))
}

/// 対角化で得た群
#[derive(Default, Debug, Clone)]
pub(crate) struct Group {
    /// 生成元 (位数 e > 1, 行の組み合わせ (行番号, 0 <= w < e) で、重みは w / e)
    pub gens: Vec<(i128, Vec<(usize, i128)>)>,
    /// 階数
    pub rank: usize,
    /// ±1 の軸で消した数
    pub unit_piv: usize,
    /// 密な対角化に回った核の大きさ (行, 列)
    pub core: (usize, usize),
    /// 溢れ・大きさの上限・時間で諦めたか (0 = 最後まで、1 = 溢れ、2 = 大きさ、3 = 時間)
    pub gave_up: u8,
}

fn axpy(a: &[(u32, i128)], f: i128, b: &[(u32, i128)]) -> Option<Vec<(u32, i128)>> {
    // a + f b (列の順に並んだ疎なベクトル)
    let mut out = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        if j >= b.len() || (i < a.len() && a[i].0 < b[j].0) {
            out.push(a[i]);
            i += 1;
        } else {
            let fb = f.checked_mul(b[j].1)?;
            let v = if i < a.len() && a[i].0 == b[j].0 {
                let v = a[i].1.checked_add(fb)?;
                i += 1;
                v
            } else {
                fb
            };
            j += 1;
            if v != 0 {
                out.push((b[j - 1].0, v));
            }
        }
    }
    Some(out)
}

fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a as i128
}

/// 整数の行列 `rows` (行ごとの (列, 係数)、列は `0..ncol`) を `L A R = diag(e)` に対角化し、e > 1 の因子と生成元 `L_i / e_i` を返す。
/// 生成元はそれぞれ `u A ≡ 0 (mod 1)` を確かめてから返す。溢れ・大きさの上限 (`max_core` = 核の行 × 列、`max_nnz` = 疎な消去の
/// 非零の数)・期限で諦めたときは、それまでに確定した生成元だけを返す (`gave_up` が 0 でない)
pub(crate) fn snf_group(rows: &[Vec<(usize, i64)>], ncol: usize, max_core: usize, max_nnz: usize, deadline: std::time::Instant) -> Group {
    let m = rows.len();
    let mut g = Group::default();
    let mut r: Vec<Vec<(u32, i128)>> = rows
        .iter()
        .map(|row| {
            let mut v: Vec<(u32, i128)> = row.iter().filter(|t| t.1 != 0).map(|&(j, a)| (j as u32, a as i128)).collect();
            v.sort_unstable_by_key(|t| t.0);
            v
        })
        .collect();
    let mut comb: Vec<Vec<(u32, i128)>> = (0..m).map(|i| vec![(i as u32, 1i128)]).collect();
    let mut alive: Vec<bool> = r.iter().map(|v| !v.is_empty()).collect();
    let mut col_rows: Vec<Vec<u32>> = vec![Vec::new(); ncol];
    for (i, row) in r.iter().enumerate() {
        for &(j, _) in row {
            col_rows[j as usize].push(i as u32);
        }
    }
    let mut col_alive = vec![true; ncol];
    let mut nnz: usize = r.iter().map(|v| v.len()).sum::<usize>() + m;
    let has = |row: &[(u32, i128)], c: u32| row.binary_search_by_key(&c, |t| t.0).ok().map(|k| row[k].1);
    // 疎な消去 (±1 の軸)
    let mut passes = 0;
    'outer: loop {
        passes += 1;
        let mut progress = false;
        let mut order: Vec<usize> = (0..ncol).filter(|&c| col_alive[c] && !col_rows[c].is_empty()).collect();
        order.sort_by_key(|&c| (col_rows[c].len(), c));
        for (cnt, &c) in order.iter().enumerate() {
            if cnt % 32 == 0 && std::time::Instant::now() >= deadline {
                g.gave_up = 3;
                break 'outer;
            }
            if !col_alive[c] {
                continue;
            }
            // 今この列を持つ生きた行だけにする
            let mut lst = std::mem::take(&mut col_rows[c]);
            lst.sort_unstable();
            lst.dedup();
            lst.retain(|&i| alive[i as usize] && has(&r[i as usize], c as u32).is_some());
            if lst.is_empty() {
                col_alive[c] = false;
                continue;
            }
            let piv = lst.iter().copied().filter(|&i| has(&r[i as usize], c as u32).is_some_and(|v| v.abs() == 1)).min_by_key(|&i| (r[i as usize].len() + comb[i as usize].len(), i));
            let Some(p) = piv else {
                col_rows[c] = lst;
                continue;
            };
            let p = p as usize;
            let s = has(&r[p], c as u32).unwrap();
            let (prow, pcomb) = (std::mem::take(&mut r[p]), std::mem::take(&mut comb[p]));
            for &i in &lst {
                let i = i as usize;
                if i == p {
                    continue;
                }
                let v = has(&r[i], c as u32).unwrap();
                let f = -(v * s);
                let Some(nr) = axpy(&r[i], f, &prow) else {
                    g.gave_up = 1;
                    break 'outer;
                };
                let Some(nc) = axpy(&comb[i], f, &pcomb) else {
                    g.gave_up = 1;
                    break 'outer;
                };
                // 新しく入った列の一覧にこの行を足す
                for &(j, _) in &nr {
                    if has(&r[i], j).is_none() {
                        col_rows[j as usize].push(i as u32);
                    }
                }
                nnz = nnz + nr.len() + nc.len() - r[i].len() - comb[i].len();
                r[i] = nr;
                comb[i] = nc;
                if r[i].is_empty() {
                    // 核の行 (u A_N = 0 ちょうど): 違反に効かない
                    alive[i] = false;
                }
            }
            alive[p] = false;
            col_alive[c] = false;
            g.rank += 1;
            g.unit_piv += 1;
            progress = true;
            if nnz > max_nnz {
                g.gave_up = 2;
                break 'outer;
            }
        }
        if !progress || passes > 50 {
            break;
        }
    }
    if g.gave_up != 0 {
        return g;
    }
    // 核: 残った行と列
    let crow: Vec<usize> = (0..m).filter(|&i| alive[i] && !r[i].is_empty()).collect();
    let mut cmap = vec![usize::MAX; ncol];
    let mut ccol: Vec<usize> = Vec::new();
    for &i in &crow {
        for &(j, _) in &r[i] {
            if cmap[j as usize] == usize::MAX {
                cmap[j as usize] = ccol.len();
                ccol.push(j as usize);
            }
        }
    }
    let (p, q) = (crow.len(), ccol.len());
    g.core = (p, q);
    if p == 0 {
        return g;
    }
    if p * q > max_core {
        g.gave_up = 2;
        return g;
    }
    let mut a = vec![vec![0i128; q]; p];
    let mut cb: Vec<Vec<(u32, i128)>> = Vec::with_capacity(p);
    for (k, &i) in crow.iter().enumerate() {
        for &(j, v) in &r[i] {
            a[k][cmap[j as usize]] = v;
        }
        cb.push(std::mem::take(&mut comb[i]));
    }
    // 密な対角化 (割り切れの鎖の条件はいらない: ⊕ Z/e_i と同型ならよい)
    let mut diag: Vec<i128> = Vec::new();
    let mut t = 0;
    'piv: while t < p.min(q) {
        if std::time::Instant::now() >= deadline {
            g.gave_up = 3;
            break;
        }
        // 絶対値の最も小さい非零を (t, t) へ
        let mut best: Option<(i128, usize, usize)> = None;
        for (i, row) in a.iter().enumerate().skip(t) {
            for (j, &v) in row.iter().enumerate().skip(t) {
                if v != 0 && best.is_none_or(|b| v.abs() < b.0) {
                    best = Some((v.abs(), i, j));
                }
            }
        }
        let Some((_, bi, bj)) = best else { break };
        a.swap(t, bi);
        cb.swap(t, bi);
        for row in a.iter_mut() {
            row.swap(t, bj);
        }
        loop {
            let mut dirty = false;
            // 列 t を行の変形で消す
            for i in t + 1..p {
                if a[i][t] == 0 {
                    continue;
                }
                let f = a[i][t].div_euclid(a[t][t]);
                if f != 0 {
                    let (top, rest) = a.split_at_mut(i);
                    for j in t..q {
                        let Some(v) = f.checked_mul(top[t][j]).and_then(|w| rest[0][j].checked_sub(w)) else {
                            g.gave_up = 1;
                            break 'piv;
                        };
                        rest[0][j] = v;
                    }
                    let Some(nc) = axpy(&cb[i], -f, &cb[t]) else {
                        g.gave_up = 1;
                        break 'piv;
                    };
                    cb[i] = nc;
                }
                if a[i][t] != 0 {
                    dirty = true;
                }
            }
            // 行 t を列の変形で消す (R は持ち歩かない)
            for j in t + 1..q {
                if a[t][j] == 0 {
                    continue;
                }
                let f = a[t][j].div_euclid(a[t][t]);
                if f != 0 {
                    for row in a.iter_mut().skip(t) {
                        let Some(v) = f.checked_mul(row[t]).and_then(|w| row[j].checked_sub(w)) else {
                            g.gave_up = 1;
                            break 'piv;
                        };
                        row[j] = v;
                    }
                }
                if a[t][j] != 0 {
                    dirty = true;
                }
            }
            if !dirty {
                break;
            }
            // 余りが残った: 行 t・列 t の中で絶対値の最も小さい非零を (t, t) へ
            let mut best: Option<(i128, usize, bool)> = None;
            for i in t + 1..p {
                if a[i][t] != 0 && a[i][t].abs() < a[t][t].abs() && best.is_none_or(|b| a[i][t].abs() < b.0) {
                    best = Some((a[i][t].abs(), i, true));
                }
            }
            for j in t + 1..q {
                if a[t][j] != 0 && a[t][j].abs() < a[t][t].abs() && best.is_none_or(|b| a[t][j].abs() < b.0) {
                    best = Some((a[t][j].abs(), j, false));
                }
            }
            match best {
                Some((_, i, true)) => {
                    a.swap(t, i);
                    cb.swap(t, i);
                }
                Some((_, j, false)) => {
                    for row in a.iter_mut() {
                        row.swap(t, j);
                    }
                }
                None => {
                    // 余りは軸より小さいはず (div_euclid の余りは 0 <= r < |軸|)
                    g.gave_up = 1;
                    break 'piv;
                }
            }
        }
        diag.push(a[t][t]);
        t += 1;
    }
    for (k, &d) in diag.iter().enumerate() {
        g.rank += 1;
        let e = d.abs();
        if e <= 1 {
            continue;
        }
        let w: Vec<(usize, i128)> = cb[k].iter().map(|&(i, v)| (i as usize, v.rem_euclid(e))).filter(|t| t.1 != 0).collect();
        // 確かめ: u A ≡ 0 (mod 1)、つまり Σ w_i A_i ≡ 0 (mod e)
        if verify(rows, ncol, &w, e) {
            g.gens.push((e, w));
        }
    }
    g
}

/// `Σ w_i A_i ≡ 0 (mod e)` を確かめる
fn verify(rows: &[Vec<(usize, i64)>], ncol: usize, w: &[(usize, i128)], e: i128) -> bool {
    let mut acc = vec![0i128; ncol];
    for &(i, wi) in w {
        for &(j, a) in &rows[i] {
            acc[j] = (acc[j] + (wi * (a as i128).rem_euclid(e)) % e) % e;
        }
    }
    acc.iter().all(|&v| v == 0)
}

/// 群の元 (重み `w_i / den`、`0 <= w_i < den`)、`χ` の値 (違反の上限 `chi / den`)、位数
#[derive(Clone, Debug)]
pub(crate) struct Elem {
    pub w: Vec<(usize, i128)>,
    pub den: i128,
    pub chi: i128,
    pub order: i128,
}

fn reduce(mut e: Elem) -> Elem {
    let mut g = gcd(e.den, e.chi);
    for &(_, v) in &e.w {
        g = gcd(g, v);
    }
    if g > 1 {
        e.den /= g;
        e.chi /= g;
        for t in e.w.iter_mut() {
            t.1 /= g;
        }
    }
    e.order = e.den;
    e
}

/// 生成元の組み合わせ `Σ t_k g_k` (`terms` = (生成元の番号, 倍数))。分母が `max_den` を超えるか溢れたら `None`
fn combine(gr: &Group, chis: &[i128], terms: &[(usize, i128)], max_den: i128) -> Option<Elem> {
    let mut den: i128 = 1;
    for &(k, _) in terms {
        let e = gr.gens[k].0;
        den = den.checked_mul(e / gcd(den, e))?;
    }
    if den > max_den {
        return None;
    }
    let mut acc: std::collections::BTreeMap<usize, i128> = std::collections::BTreeMap::new();
    let mut chi: i128 = 0;
    for &(k, t) in terms {
        let (e, w) = (&gr.gens[k].0, &gr.gens[k].1);
        let f = (den / e).checked_mul(t)?;
        for &(i, v) in w {
            let x = acc.entry(i).or_insert(0);
            *x = (*x + (f.checked_mul(v)?).rem_euclid(den)) % den;
        }
        chi = (chi + f.checked_mul(chis[k])?.rem_euclid(den)) % den;
    }
    let w: Vec<(usize, i128)> = acc.into_iter().filter(|t| t.1 != 0).collect();
    Some(reduce(Elem { w, den, chi, order: den }))
}

/// 群の元のうち違反の大きいもの。`rhs` は行ごとの右辺 (置き換えた空間の整数)。生成元 1 つの倍数 (位数が `all_mult` 以下なら
/// 全部、それより大きければ χ を最大にする倍数と 1 倍) と、χ の位数が両方より大きくなる 2 つの生成元の組 (χ を最大にする倍数) を
/// 作り、違反の上限 `chi / den` の大きい順に `max_elems` 個まで返す。分母は `max_den` まで
pub(crate) fn group_elements(gr: &Group, rhs: &[i128], max_elems: usize, max_den: i128) -> Vec<Elem> {
    let chis: Vec<i128> = gr
        .gens
        .iter()
        .map(|(e, w)| {
            let mut c: i128 = 0;
            for &(i, v) in w {
                c = (c + (v * rhs[i].rem_euclid(*e)) % e) % e;
            }
            c
        })
        .collect();
    let mut out: Vec<Elem> = Vec::new();
    let mut seen: std::collections::HashSet<Vec<(usize, i128)>> = std::collections::HashSet::new();
    let mut add = |el: Option<Elem>, out: &mut Vec<Elem>| {
        if let Some(el) = el {
            if el.chi != 0 && seen.insert(el.w.clone()) {
                out.push(el);
            }
        }
    };
    // χ の位数 (生成元ごと)
    let ord: Vec<i128> = gr.gens.iter().zip(&chis).map(|((e, _), &c)| if c == 0 { 1 } else { e / gcd(*e, c) }).collect();
    // χ を最大にする倍数: t c ≡ -gcd(c, e) (mod e)
    let best_t = |k: usize| -> i128 {
        let (e, c) = (gr.gens[k].0, chis[k]);
        let gg = gcd(c, e);
        let (e2, c2) = (e / gg, c / gg);
        // c2 の e2 での逆元 (拡張ユークリッド)
        let (mut a, mut b, mut x0, mut x1) = (c2.rem_euclid(e2), e2, 1i128, 0i128);
        while b != 0 {
            let q = a / b;
            (a, b) = (b, a - q * b);
            (x0, x1) = (x1, x0 - q * x1);
        }
        let inv = x0.rem_euclid(e2.max(1));
        (e2 - 1) * inv % e2.max(1)
    };
    let all_mult = 12;
    for k in 0..gr.gens.len() {
        if chis[k] == 0 {
            continue;
        }
        let e = gr.gens[k].0;
        if e <= all_mult {
            for t in 1..e {
                add(combine(gr, &chis, &[(k, t)], max_den), &mut out);
            }
        } else {
            add(combine(gr, &chis, &[(k, best_t(k))], max_den), &mut out);
            add(combine(gr, &chis, &[(k, 1)], max_den), &mut out);
        }
    }
    // 2 つの組: χ の位数の最小公倍数が両方より大きいもの (中国剰余定理での組み合わせ)。組み合わせる生成元は台の小さい順に 60 個まで
    let mut ks: Vec<usize> = (0..gr.gens.len()).filter(|&k| ord[k] > 1).collect();
    ks.sort_by_key(|&k| (gr.gens[k].1.len(), k));
    ks.truncate(60);
    for (a, &k1) in ks.iter().enumerate() {
        for &k2 in &ks[a + 1..] {
            let l = ord[k1] / gcd(ord[k1], ord[k2]) * ord[k2];
            if l <= ord[k1].max(ord[k2]) {
                continue;
            }
            add(combine(gr, &chis, &[(k1, best_t(k1)), (k2, best_t(k2))], max_den), &mut out);
        }
    }
    // 違反の上限の大きい順 (同じなら台の小さい順)
    out.sort_by(|x, y| ((y.chi as f64) / (y.den as f64)).total_cmp(&((x.chi as f64) / (x.den as f64))).then(x.w.len().cmp(&y.w.len())));
    out.truncate(max_elems);
    out
}

/// 素因数分解 (小さい数向け、試し割り) の (素数, 指数)
pub(crate) fn factor(mut v: i128) -> Vec<(i128, u32)> {
    let mut out = Vec::new();
    let mut p = 2i128;
    while p * p <= v && p < 1_000_000 {
        let mut e = 0;
        while v % p == 0 {
            v /= p;
            e += 1;
        }
        if e > 0 {
            out.push((p, e));
        }
        p += 1;
    }
    if v > 1 {
        out.push((v, 1));
    }
    out
}

/// 群の要約 (素数べきの因子の数): "2:3 4:1 3:2 ..." の形
pub(crate) fn group_summary(gr: &Group) -> String {
    let mut cnt: std::collections::BTreeMap<i128, usize> = std::collections::BTreeMap::new();
    for (e, _) in &gr.gens {
        for (p, k) in factor(*e) {
            *cnt.entry(p.pow(k)).or_insert(0) += 1;
        }
    }
    let s: Vec<String> = cnt.iter().map(|(q, c)| format!("{q}:{c}")).collect();
    // 台が行 1 本の生成元の数 (行の係数の最大公約数から出るだけの因子)
    let single = gr.gens.iter().filter(|g| g.1.len() == 1).count();
    if s.is_empty() { "-".to_string() } else { format!("{}#s{single}", s.join(",")) }
}

/// [`snfmodk_cg_cuts`] の記録
#[derive(Default, Debug, Clone)]
pub(crate) struct SnfStats {
    pub rows: usize,
    pub cols: usize,
    pub elems: usize,
    /// 作った (違反する) カット: 位数が 2・3・5・7 のもの, それ以外 (合成数・素数べき・ほかの素数)
    pub cuts_p: usize,
    pub cuts_new: usize,
    pub best_eff_new: f64,
    pub best_eff_p: f64,
    /// 作ったカットのうち、行 1 本だけの元 (行の係数の最大公約数で割るのと同じ) から出たもの
    pub cuts_single: usize,
    /// 位数が 2・3・5・7 以外で、行 2 本以上の元から出たカット (その数, 効き目の最大)
    pub cuts_new_multi: usize,
    pub best_eff_new_multi: f64,
}

/// zerohalf と同じ行・列の選び方で、余裕 0 の行の連立 A_N の群から CG カットを作る。引数は [`super::zerohalf::zerohalf_cuts`] と同じ。
/// カットは厳密な整数の算術 (i128、溢れたらその元は捨てる) で作る
#[allow(clippy::too_many_arguments)]
pub(crate) fn snfmodk_cg_cuts(rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64], is_int: &[bool], lo: &[f64], up: &[f64], x: &[f64], max_cuts: usize, deadline: std::time::Instant) -> (Vec<RawCut>, Group, SnfStats) {
    const FT: f64 = 1e-6;
    let n = x.len();
    let mut st = SnfStats::default();
    let is_integral = |v: f64| (v - v.round()).abs() <= 1e-9 && v.abs() < 1e15;
    let mut use_lower = vec![true; n];
    let mut xbar = vec![0.0; n];
    let mut col_ok = vec![false; n];
    for j in 0..n {
        if !is_int[j] {
            continue;
        }
        let (l, u) = (lo[j], up[j]);
        let lf = l.is_finite() && is_integral(l);
        let uf = u.is_finite() && is_integral(u);
        if !lf && !uf {
            continue;
        }
        col_ok[j] = true;
        use_lower[j] = lf && (!uf || x[j] - l <= u - x[j]);
        xbar[j] = if use_lower[j] { (x[j] - l).max(0.0) } else { (u - x[j]).max(0.0) };
    }
    // 余裕 0 の行の向き (元の行, 符号, 置き換えた右辺 b')
    let mut sides: Vec<(usize, i64, i128)> = Vec::new();
    let mut col_index = vec![usize::MAX; n];
    let mut ncols = 0usize;
    let mut mat: Vec<Vec<(usize, i64)>> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        if r.is_empty() || !r.iter().all(|&(j, a)| col_ok[j] && is_integral(a)) {
            continue;
        }
        let act: f64 = r.iter().map(|&(j, a)| a * x[j]).sum();
        let tol = FT * (1.0 + act.abs());
        let sg: i64 = if row_up[i].is_finite() && is_integral(row_up[i]) && row_up[i] - act <= tol {
            1
        } else if row_lo[i].is_finite() && is_integral(row_lo[i]) && act - row_lo[i] <= tol {
            -1
        } else {
            continue;
        };
        let b = if sg > 0 { row_up[i] } else { -row_lo[i] };
        let mut bp: i128 = b.round() as i128;
        let mut ents: Vec<(usize, i64)> = Vec::new();
        for &(j, a) in r {
            let a = sg * a.round() as i64;
            let bd = if use_lower[j] { lo[j] } else { up[j] };
            bp -= a as i128 * bd.round() as i128;
            if xbar[j] > FT {
                if col_index[j] == usize::MAX {
                    col_index[j] = ncols;
                    ncols += 1;
                }
                ents.push((col_index[j], if use_lower[j] { a } else { -a }));
            }
        }
        if ents.is_empty() {
            continue;
        }
        sides.push((i, sg, bp));
        mat.push(ents);
    }
    st.rows = sides.len();
    st.cols = ncols;
    if sides.is_empty() {
        return (Vec::new(), Group::default(), st);
    }
    let max_core = tunable!("ENOMOTO_T_MIP_SNFMODK_CORE", 250_000usize, usize);
    let gr = snf_group(&mat, ncols, max_core, 20_000_000, deadline);
    let rhs: Vec<i128> = sides.iter().map(|s| s.2).collect();
    let elems = group_elements(&gr, &rhs, tunable!("ENOMOTO_T_MIP_SNFMODK_ELEMS", 200usize, usize), 1 << 40);
    st.elems = elems.len();
    // カットを作る
    let mut cuts: Vec<(f64, bool, bool, RawCut)> = Vec::new();
    let mut acc = vec![0i128; n];
    let mut touched: Vec<usize> = Vec::new();
    'el: for el in &elems {
        if std::time::Instant::now() >= deadline {
            break;
        }
        let den = el.den;
        let mut bsum: i128 = 0;
        for &(s, w) in &el.w {
            let (i, sg, bp) = sides[s];
            let Some(v) = w.checked_mul(bp).and_then(|v| bsum.checked_add(v)) else {
                for &j in &touched {
                    acc[j] = 0;
                }
                touched.clear();
                continue 'el;
            };
            bsum = v;
            for &(j, a) in &rows[i] {
                let a = sg as i128 * a.round() as i128;
                let ap = if use_lower[j] { a } else { -a };
                if acc[j] == 0 {
                    touched.push(j);
                }
                match w.checked_mul(ap).and_then(|v| acc[j].checked_add(v)) {
                    Some(v) => acc[j] = if v == 0 { i128::MIN } else { v },
                    None => {
                        for &j in &touched {
                            acc[j] = 0;
                        }
                        touched.clear();
                        continue 'el;
                    }
                }
            }
        }
        // x' の空間で floor(c' / den) x' <= floor(b' / den)、元の変数へ
        let mut rhs_x = bsum.div_euclid(den) as f64;
        let mut coefs: Vec<(usize, f64)> = Vec::new();
        for &j in &touched {
            let c = if acc[j] == i128::MIN { 0 } else { acc[j] };
            acc[j] = 0;
            let f = c.div_euclid(den);
            if f == 0 {
                continue;
            }
            let f = f as f64;
            if use_lower[j] {
                coefs.push((j, f));
                rhs_x += f * lo[j];
            } else {
                coefs.push((j, -f));
                rhs_x -= f * up[j];
            }
        }
        touched.clear();
        if coefs.is_empty() || coefs.iter().any(|t| t.1.abs() > 1e9) {
            continue;
        }
        coefs.sort_unstable_by_key(|t| t.0);
        let act: f64 = coefs.iter().map(|&(j, a)| a * x[j]).sum();
        let nrm = coefs.iter().map(|t| t.1 * t.1).sum::<f64>().sqrt();
        if act > rhs_x + 1e-6 * (1.0 + rhs_x.abs()) {
            let eff = (act - rhs_x) / nrm;
            let newk = !matches!(el.order, 2 | 3 | 5 | 7);
            st.cuts_single += (el.w.len() == 1) as usize;
            if newk && el.w.len() > 1 {
                st.cuts_new_multi += 1;
                st.best_eff_new_multi = st.best_eff_new_multi.max(eff);
            }
            if newk {
                st.cuts_new += 1;
                st.best_eff_new = st.best_eff_new.max(eff);
            } else {
                st.cuts_p += 1;
                st.best_eff_p = st.best_eff_p.max(eff);
            }
            cuts.push((eff, newk, el.w.len() > 1, RawCut { coefs, rhs: rhs_x }));
        }
    }
    cuts.sort_by(|a, b| b.0.total_cmp(&a.0));
    // `ENOMOTO_MIP_SNFMODK` に `n` を付けたら位数が 2・3・5・7 以外の元のカットだけ、`m` を付けたら行 2 本以上の元のカットだけ
    // (行 1 本の元は行の係数の最大公約数で割って丸めるのと同じで、元の行 1 本ずつの分離と重なる)
    let (only_new, only_multi) = (snfmodk_has('n'), snfmodk_has('m'));
    let out: Vec<RawCut> = cuts.into_iter().filter(|c| (!only_new || c.1) && (!only_multi || c.2)).take(max_cuts).map(|c| c.3).collect();
    (out, gr, st)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn far() -> std::time::Instant {
        std::time::Instant::now() + std::time::Duration::from_secs(30)
    }

    #[test]
    fn group_of_diag() {
        // diag(2, 3) と 6 が混ざった形: 群は Z/2 × Z/3 × Z/6 (位数 36)
        let rows = vec![vec![(0, 2)], vec![(1, 3)], vec![(2, 6)], vec![(0, 1), (3, 1)]];
        let gr = snf_group(&rows, 4, 1000, 100000, far());
        assert_eq!(gr.gave_up, 0);
        let ord: i128 = gr.gens.iter().map(|g| g.0).product();
        // 行 3 は列 3 (係数 1) の軸で消え、残りは diag(2, 3, 6)
        assert_eq!(ord, 36, "{:?}", gr.gens);
    }

    #[test]
    fn random_group_order_matches_determinant() {
        // 正方で正則な乱数の行列: 群の位数 = |det|
        let mut seed = 12345u64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..300 {
            let k = 1 + (rnd() % 5) as usize;
            let mut m = vec![vec![0i64; k]; k];
            for row in m.iter_mut() {
                for v in row.iter_mut() {
                    *v = (rnd() % 9) as i64 - 4;
                }
            }
            // 行列式 (分数を使わず i128 の Bareiss)
            let mut b: Vec<Vec<i128>> = m.iter().map(|r| r.iter().map(|&v| v as i128).collect()).collect();
            let mut sign = 1i128;
            let mut prev = 1i128;
            let mut det = 0i128;
            let mut singular = false;
            for t in 0..k {
                if b[t][t] == 0 {
                    match (t + 1..k).find(|&i| b[i][t] != 0) {
                        Some(i) => {
                            b.swap(t, i);
                            sign = -sign;
                        }
                        None => {
                            singular = true;
                            break;
                        }
                    }
                }
                for i in t + 1..k {
                    for j in t + 1..k {
                        b[i][j] = (b[i][j] * b[t][t] - b[i][t] * b[t][j]) / prev;
                    }
                }
                prev = b[t][t];
                det = b[t][t];
            }
            if singular {
                continue;
            }
            let det = (sign * det).abs();
            let rows: Vec<Vec<(usize, i64)>> = m.iter().map(|r| r.iter().enumerate().filter(|t| *t.1 != 0).map(|(j, &v)| (j, v)).collect()).collect();
            let gr = snf_group(&rows, k, 1000, 100000, far());
            assert_eq!(gr.gave_up, 0);
            let ord: i128 = gr.gens.iter().map(|g| g.0).product();
            assert_eq!(ord, det, "{m:?} {:?}", gr.gens);
            assert_eq!(gr.rank, k);
        }
    }

    #[test]
    fn mod6_element() {
        // x0 = 2 y0 + 3 y1 ... 1 本の行 6 x0 = 5 (右辺) は Z/6、χ = 5/6 の元がある
        let rows = vec![vec![(0, 6)]];
        let gr = snf_group(&rows, 1, 100, 1000, far());
        let el = group_elements(&gr, &[5], 10, 1000);
        assert!(el.iter().any(|e| e.den == 6 && e.chi == 5), "{el:?}");
    }

    /// 乱数の小さな整数行で、CG カットが全ての整数実行可能点で成り立つことを総当たりで確かめる
    #[test]
    fn random_validity() {
        let mut seed = 4242u64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut found = 0;
        let mut found_new = 0;
        for _ in 0..3000 {
            let n = 2 + (rnd() * 4.0) as usize;
            let m = 1 + (rnd() * 4.0) as usize;
            let lo: Vec<f64> = (0..n).map(|_| -(rnd() * 2.0).floor()).collect();
            let up: Vec<f64> = (0..n).map(|j| lo[j] + 1.0 + (rnd() * 3.0).floor()).collect();
            let mut rows = Vec::new();
            let (mut rl, mut ru) = (Vec::new(), Vec::new());
            for _ in 0..m {
                let mut r: Vec<(usize, f64)> = Vec::new();
                for j in 0..n {
                    if rnd() < 0.6 {
                        let a = ((rnd() - 0.5) * 14.0).round();
                        if a != 0.0 {
                            r.push((j, a));
                        }
                    }
                }
                if r.is_empty() {
                    continue;
                }
                rows.push(r);
                let b = ((rnd() - 0.3) * 8.0).round();
                if rnd() < 0.4 {
                    rl.push(b);
                    ru.push(b);
                } else {
                    rl.push(f64::NEG_INFINITY);
                    ru.push(b);
                }
            }
            // 余裕 0 の行を作るため、LP 点を分母 6 (か 4) の有理数にし、行の値が整数になる行は右辺をその値にする
            let den = if rnd() < 0.5 { 6.0 } else { 4.0 };
            let xq: Vec<f64> = (0..n).map(|j| ((lo[j] + rnd() * (up[j] - lo[j])) * den).round() / den).collect();
            for (i, r) in rows.iter().enumerate() {
                let act: f64 = r.iter().map(|&(j, a)| a * xq[j]).sum();
                if (act - act.round()).abs() < 1e-9 && rnd() < 0.8 {
                    ru[i] = act.round();
                    if rl[i].is_finite() {
                        rl[i] = ru[i];
                    }
                }
            }
            let (cuts, _, st) = snfmodk_cg_cuts(&rows, &rl, &ru, &vec![true; n], &lo, &up, &xq, 20, far());
            found += cuts.len();
            found_new += st.cuts_new;
            let mut pt: Vec<f64> = lo.clone();
            loop {
                let feas = rows.iter().enumerate().all(|(i, r)| {
                    let a: f64 = r.iter().map(|&(j, v)| v * pt[j]).sum();
                    a <= ru[i] + 1e-9 && a >= rl[i] - 1e-9
                });
                if feas {
                    for c in &cuts {
                        let a: f64 = c.coefs.iter().map(|&(j, v)| v * pt[j]).sum();
                        assert!(a <= c.rhs + 1e-9, "invalid snfmodk cut {:?} <= {} at {pt:?}; rows {rows:?} {rl:?} {ru:?} x {xq:?}", c.coefs, c.rhs);
                    }
                }
                let mut k = 0;
                while k < n {
                    pt[k] += 1.0;
                    if pt[k] <= up[k] {
                        break;
                    }
                    pt[k] = lo[k];
                    k += 1;
                }
                if k == n {
                    break;
                }
            }
        }
        assert!(found > 50, "only {found} cuts ({found_new} new)");
        assert!(found_new > 5, "only {found_new} new-order cuts of {found}");
    }
}
