//! 試作 `ENOMOTO_MIP_HNFPRE` (既定では使わない): 整数列だけの等式の Hermite 標準形による前処理
//! (Aardal–Hurkens–Lenstra 2000 の格子の書き直しの考えの一部)。
//!
//! 前処理の後の問題から、固定されていない列がすべて整数列で、係数と右辺が (小さい分母の) 有理数の等式を集め、
//! 分母を払って整数の連立 `A x = b` にする (固定された列は定数として右辺へ移す)。等式が列を共有する連結成分ごとに、
//! 単模な列の操作で `A U = [H 0]` (H は下三角) にし (整数のまま、i128 で溢れを検査する。溢れたらその成分は諦める)、
//! - `H y = b` が整数の解を持たなければ実行不能 (右辺が格子に入らない)、
//! - 持てば整数解の全体は `x = x0 + N λ` (N は U の残りの列 = 整数の核の基底) で、列 j について N の j 行の gcd を g とすると
//!   g = 0 なら `x_j = x0_j` に固定、g > 1 なら `x_j ≡ x0_j (mod g)` の合同式が出る。
//!
//! 使い方 (`ENOMOTO_MIP_HNFPRE`): `count` = 数えてログに出すだけ (問題は変えない)、`bounds` = 根で上下限を合同式に合う値まで詰める
//! (固定を含む) だけ、それ以外 (`1` など) = それに加えて分枝と伝播の上下限の丸めにも合同式を使う ([`CongTable`] を
//! [`super::domain::Domain`] が引く)。

use super::problem::MipProblem;
use std::sync::{Arc, Mutex};

/// 動き方。`None` = 使わない (既定)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mode {
    Count,
    Bounds,
    Full,
}

pub(crate) fn mode() -> Option<Mode> {
    match env_str!("ENOMOTO_MIP_HNFPRE") {
        // 試作 `ENOMOTO_MIP_HNFCUT` ([`super::hnfcut`]) だけなら `1` と同じ
        None | Some("") | Some("0") => super::hnfcut::on().then_some(Mode::Full),
        Some("count") => Some(Mode::Count),
        Some("bounds") => Some(Mode::Bounds),
        Some(_) => Some(Mode::Full),
    }
}

/// 列ごとの合同式 `x_j ≡ r_j (mod m_j)` (m_j <= 1 は無し)。行の指紋が同じ問題でだけ使う。
#[derive(Debug)]
pub(crate) struct CongTable {
    pub fp: u64,
    pub n: usize,
    pub r: Vec<i64>,
    pub m: Vec<i64>,
}

impl CongTable {
    /// `v` 以上で合同式に合う最小の値 (合同式がなければ `v` のまま)。`v` は整数に丸め済みであること。
    #[inline]
    pub fn round_up(&self, j: usize, v: f64) -> f64 {
        let m = self.m[j];
        if m <= 1 || !v.is_finite() || v.abs() > 1e15 {
            return v;
        }
        let vi = v as i64;
        let k = (self.r[j] - vi).rem_euclid(m);
        (vi + k) as f64
    }
    /// `v` 以下で合同式に合う最大の値。
    #[inline]
    pub fn round_down(&self, j: usize, v: f64) -> f64 {
        let m = self.m[j];
        if m <= 1 || !v.is_finite() || v.abs() > 1e15 {
            return v;
        }
        let vi = v as i64;
        let k = (vi - self.r[j]).rem_euclid(m);
        (vi - k) as f64
    }
}

static TABLES: Mutex<Vec<Arc<CongTable>>> = Mutex::new(Vec::new());

/// 合同式の表を登録する (同じ指紋の古いものは置き換える。多くても 8 個まで持つ)。
pub(crate) fn register(t: CongTable) {
    let mut v = TABLES.lock().unwrap();
    v.retain(|x| !(x.fp == t.fp && x.n == t.n));
    if v.len() >= 8 {
        v.remove(0);
    }
    v.push(Arc::new(t));
}

/// 問題 `p` に合う合同式の表 (`Full` のときだけ。行の指紋と列の数で照らす)。
pub(crate) fn table_for(p: &MipProblem) -> Option<Arc<CongTable>> {
    if mode() != Some(Mode::Full) {
        return None;
    }
    let v = TABLES.lock().unwrap();
    if v.is_empty() {
        return None;
    }
    let fp = super::rows_fingerprint(&p.rows);
    v.iter().rev().find(|t| t.fp == fp && t.n == p.n).cloned()
}

/// 解析の結果。
#[derive(Debug, Default, Clone)]
pub(crate) struct HnfStats {
    /// 等式の数・そのうち整数列だけ (固定された連続列を含んでもよい) で係数が有理数のもの。
    pub eq_rows: usize,
    pub int_eq_rows: usize,
    /// 使えなかった整数列だけの等式 (分母が大きい・係数が大きい)。
    pub rat_skipped: usize,
    /// 連結成分の数・解けた成分・諦めた成分 (大きさ・溢れ・手間)。
    pub comps: usize,
    pub comps_done: usize,
    pub comps_gave_up: usize,
    /// 諦めた理由: 溢れ (値が 2^96 を越えた)・手間・大きさ。
    pub gave_overflow: usize,
    pub gave_work: usize,
    pub gave_size: usize,
    /// 等式の列の数・階数の合計。
    pub cols: usize,
    pub rank: usize,
    /// 格子から出た固定 (g = 0) の数・そのうち今の上下限で既に固定されていないもの。
    pub fixes: usize,
    /// 合同式 (g > 1) の数・そのうち 0-1 でない列のもの・上下限が詰まるもの。
    pub congs: usize,
    pub congs_gen: usize,
    pub congs_tighten: usize,
    /// 合同式の法の最大。
    pub max_mod: i64,
    /// 実行不能が分かったか。
    pub infeasible: bool,
    pub secs: f64,
}

pub(crate) struct HnfResult {
    pub stats: HnfStats,
    /// (列, x0, 法 g): g = 0 なら固定、g > 1 なら合同式。
    pub info: Vec<(usize, i64, i64)>,
}

/// 浮動小数の値を小さい分母の有理数 (分子, 分母) にする。
fn to_rat(v: f64, maxden: i128) -> Option<(i128, i128)> {
    if !v.is_finite() || v.abs() > 1e15 {
        return None;
    }
    let tol = 1e-9 * v.abs().max(1.0);
    let r = v.round();
    if (v - r).abs() <= tol {
        return Some((r as i128, 1));
    }
    // 連分数
    let (mut h0, mut h1, mut k0, mut k1) = (0i128, 1i128, 1i128, 0i128);
    let mut x = v;
    for _ in 0..40 {
        let a = x.floor();
        if a.abs() > 1e15 {
            return None;
        }
        let ai = a as i128;
        let h2 = ai.checked_mul(h1)?.checked_add(h0)?;
        let k2 = ai.checked_mul(k1)?.checked_add(k0)?;
        if k2 > maxden {
            return None;
        }
        h0 = h1;
        h1 = h2;
        k0 = k1;
        k1 = k2;
        if (v - h1 as f64 / k1 as f64).abs() <= tol {
            return Some((h1, k1));
        }
        let f = x - a;
        if f.abs() < 1e-15 {
            return None;
        }
        x = 1.0 / f;
    }
    None
}

fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a as i128
}

/// 疎な列 (キーの昇順)。
type SCol = Vec<(u32, i128)>;

fn get(c: &SCol, k: u32) -> i128 {
    match c.binary_search_by_key(&k, |e| e.0) {
        Ok(i) => c[i].1,
        Err(_) => 0,
    }
}

/// `a - q b` (溢れたら `None`)。新しく非零になった A 側のキー (< `na`) を `fill` に入れる。
fn axpy(a: &SCol, q: i128, b: &SCol, na: u32, fill: &mut Vec<u32>) -> Option<SCol> {
    let mut out = Vec::with_capacity(a.len() + b.len());
    let (mut i, mut k) = (0, 0);
    while i < a.len() || k < b.len() {
        if k >= b.len() || (i < a.len() && a[i].0 < b[k].0) {
            out.push(a[i]);
            i += 1;
        } else if i >= a.len() || b[k].0 < a[i].0 {
            let v = q.checked_mul(b[k].1)?.checked_neg()?;
            if b[k].0 < na {
                fill.push(b[k].0);
            }
            out.push((b[k].0, v));
            k += 1;
        } else {
            let v = a[i].1.checked_sub(q.checked_mul(b[k].1)?)?;
            if v != 0 {
                out.push((a[i].0, v));
            }
            i += 1;
            k += 1;
        }
    }
    // 大きすぎる値は諦める (後の積で溢れやすい)
    if out.iter().any(|e| e.1.unsigned_abs() > (1u128 << 96)) {
        return None;
    }
    Some(out)
}

enum CompOut {
    Infeasible,
    /// 0 = 溢れ、1 = 手間
    GaveUp(u8),
    /// (局所の列, x0, g)、階数
    Done(Vec<(usize, i128, i128)>, usize),
}

/// 1 つの連結成分の `A x = b` (行は局所の列番号の疎な行) を解く。`work` は手間の残り (足りなくなったら諦める)。
fn solve_comp(rows: &[Vec<(usize, i128)>], b: &[i128], nc: usize, work: &mut i64) -> CompOut {
    let na = rows.len() as u32;
    // 列 c: A の列 (キー 0..na) と U の列 (キー na + c)
    let mut cols: Vec<SCol> = vec![Vec::new(); nc];
    for (i, r) in rows.iter().enumerate() {
        for &(c, a) in r {
            cols[c].push((i as u32, a));
        }
    }
    for (c, col) in cols.iter_mut().enumerate() {
        col.push((na + c as u32, 1));
    }
    // rowcols[i]: 行 i に非零を持つかもしれない列 (上位集合)
    let mut rowcols: Vec<Vec<u32>> = rows.iter().map(|r| r.iter().map(|&(c, _)| c as u32).collect()).collect();
    let mut active = vec![true; nc];
    // 行 i の軸の列 (なければ None = 従属な行)
    let mut pivot: Vec<Option<usize>> = vec![None; rows.len()];
    let mut fill = Vec::new();
    // 行は「活きた候補の列の少ない順」に動的に選ぶ (埋まりを減らす)。軸の列は先に選んだ行で 0 なので、選んだ順に下三角
    let mut order: Vec<usize> = Vec::with_capacity(rows.len());
    let mut done_row = vec![false; rows.len()];
    let mut heap: std::collections::BinaryHeap<std::cmp::Reverse<(usize, usize)>> = (0..rows.len()).map(|i| std::cmp::Reverse((rowcols[i].len(), i))).collect();
    while let Some(std::cmp::Reverse((key, i))) = heap.pop() {
        if done_row[i] {
            continue;
        }
        {
            let ik = i as u32;
            rowcols[i].sort_unstable();
            rowcols[i].dedup();
            rowcols[i].retain(|&c| active[c as usize] && get(&cols[c as usize], ik) != 0);
            let c = rowcols[i].len();
            if c > key && heap.peek().is_some_and(|t| t.0 .0 < c) {
                heap.push(std::cmp::Reverse((c, i)));
                continue;
            }
        }
        done_row[i] = true;
        order.push(i);
        let ik = i as u32;
        loop {
            // 行 i に非零を持つ活きた列
            let mut cand: Vec<usize> = Vec::new();
            rowcols[i].sort_unstable();
            rowcols[i].dedup();
            for &c in &rowcols[i] {
                let c = c as usize;
                if active[c] && get(&cols[c], ik) != 0 {
                    cand.push(c);
                }
            }
            rowcols[i] = cand.iter().map(|&c| c as u32).collect();
            if cand.is_empty() {
                break;
            }
            if cand.len() == 1 {
                pivot[i] = Some(cand[0]);
                active[cand[0]] = false;
                break;
            }
            // 絶対値の最小 (同じなら非零の少ない) 列で他を割った余りにする
            let p = *cand.iter().min_by_key(|&&c| (get(&cols[c], ik).unsigned_abs(), cols[c].len())).unwrap();
            let ep = get(&cols[p], ik);
            let colp = cols[p].clone();
            for &c in &cand {
                if c == p {
                    continue;
                }
                let ec = get(&cols[c], ik);
                // 最も近い整数の商 (余りの絶対値を |ep|/2 以下に)
                let q = {
                    let q0 = ec.div_euclid(ep);
                    let r0 = ec - q0 * ep;
                    if 2 * r0.abs() > ep.abs() {
                        q0 + if (r0 > 0) == (ep > 0) { 1 } else { -1 }
                    } else {
                        q0
                    }
                };
                *work -= (cols[c].len() + colp.len()) as i64;
                if *work < 0 {
                    return CompOut::GaveUp(1);
                }
                fill.clear();
                match axpy(&cols[c], q, &colp, na, &mut fill) {
                    Some(nv) => cols[c] = nv,
                    None => return CompOut::GaveUp(0),
                }
                for &t in &fill {
                    rowcols[t as usize].push(c as u32);
                }
            }
        }
    }
    // H y = b を前から解く
    let mut res: Vec<i128> = b.to_vec();
    let mut y: Vec<(usize, i128)> = Vec::new();
    for &i in &order {
        match pivot[i] {
            None => {
                if res[i] != 0 {
                    return CompOut::Infeasible;
                }
            }
            Some(p) => {
                let h = get(&cols[p], i as u32);
                if res[i] % h != 0 {
                    return CompOut::Infeasible;
                }
                let yp = res[i] / h;
                if yp != 0 {
                    for &(t, v) in &cols[p] {
                        if t >= na {
                            break;
                        }
                        let Some(d) = v.checked_mul(yp) else { return CompOut::GaveUp(0) };
                        let Some(nr) = res[t as usize].checked_sub(d) else { return CompOut::GaveUp(0) };
                        res[t as usize] = nr;
                    }
                    y.push((p, yp));
                }
            }
        }
    }
    // x0 = Σ y_p U_p
    let mut x0 = vec![0i128; nc];
    for &(p, yp) in &y {
        for &(t, v) in &cols[p] {
            if t < na {
                continue;
            }
            let Some(d) = v.checked_mul(yp) else { return CompOut::GaveUp(0) };
            let c = (t - na) as usize;
            let Some(s) = x0[c].checked_add(d) else { return CompOut::GaveUp(0) };
            x0[c] = s;
        }
    }
    // 核の基底 (活きたままの列の U の部分) の行ごとの gcd
    let mut g = vec![0i128; nc];
    for c in 0..nc {
        if !active[c] {
            continue;
        }
        for &(t, v) in &cols[c] {
            if t >= na {
                let k = (t - na) as usize;
                g[k] = gcd(g[k], v);
            }
        }
    }
    let rank = pivot.iter().filter(|p| p.is_some()).count();
    let out = (0..nc).filter(|&c| g[c] != 1).map(|c| (c, x0[c], g[c])).collect();
    CompOut::Done(out, rank)
}

/// 前処理の後の問題 `p` の整数列だけの等式を解析する。`int_ok[j]` は整数列として扱ってよいか (尺度が掛かっていない)。
pub(crate) fn analyze(p: &MipProblem, int_ok: &[bool]) -> HnfResult {
    let t0 = std::time::Instant::now();
    let mut st = HnfStats::default();
    let free = |j: usize| p.col_lo[j] < p.col_up[j];
    // 整数の等式 (列は元の番号)
    let mut erows: Vec<(Vec<(usize, i128)>, i128)> = Vec::new();
    'row: for i in 0..p.m {
        if p.row_lo[i] != p.row_up[i] || !p.row_lo[i].is_finite() {
            continue;
        }
        st.eq_rows += 1;
        let mut rhs = p.row_lo[i];
        let mut any = false;
        for &(j, a) in &p.rows[i] {
            if free(j) {
                if !(p.is_int[j] && int_ok[j]) {
                    continue 'row;
                }
                any = true;
            } else {
                rhs -= a * p.col_lo[j];
            }
        }
        if !any {
            continue;
        }
        st.int_eq_rows += 1;
        // 分母を払う
        let mut rat: Vec<(usize, i128, i128)> = Vec::new();
        let mut den: i128 = 1;
        for &(j, a) in &p.rows[i] {
            if !free(j) || a == 0.0 {
                continue;
            }
            let Some((nu, de)) = to_rat(a, 1000) else {
                st.rat_skipped += 1;
                continue 'row;
            };
            den = den / gcd(den, de) * de;
            if den > 1_000_000 {
                st.rat_skipped += 1;
                continue 'row;
            }
            rat.push((j, nu, de));
        }
        let Some((bn, bd)) = to_rat(rhs, 1000) else {
            st.rat_skipped += 1;
            continue 'row;
        };
        den = den / gcd(den, bd) * bd;
        if den > 1_000_000 {
            st.rat_skipped += 1;
            continue 'row;
        }
        let mut r: Vec<(usize, i128)> = Vec::with_capacity(rat.len());
        for (j, nu, de) in rat {
            let v = nu * (den / de);
            if v.unsigned_abs() > (1u128 << 60) {
                st.rat_skipped += 1;
                continue 'row;
            }
            r.push((j, v));
        }
        let b = bn * (den / bd);
        if b.unsigned_abs() > (1u128 << 60) {
            st.rat_skipped += 1;
            continue 'row;
        }
        erows.push((r, b));
    }
    // 連結成分 (列で)
    let mut parent: Vec<usize> = (0..p.n).collect();
    fn find(pa: &mut [usize], mut x: usize) -> usize {
        while pa[x] != x {
            pa[x] = pa[pa[x]];
            x = pa[x];
        }
        x
    }
    for (r, _) in &erows {
        for w in r.windows(2) {
            let (a, b) = (find(&mut parent, w[0].0), find(&mut parent, w[1].0));
            if a != b {
                parent[a] = b;
            }
        }
    }
    let mut comp_rows: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
    for (k, (r, _)) in erows.iter().enumerate() {
        let c = find(&mut parent, r[0].0);
        comp_rows.entry(c).or_default().push(k);
    }
    let mut comps: Vec<Vec<usize>> = comp_rows.into_values().collect();
    comps.sort_by_key(|c| c.len());
    st.comps = comps.len();
    let mut work: i64 = tunable!("ENOMOTO_T_MIP_HNFPRE_WORK", 300_000_000.0, f64) as i64;
    let mut info = Vec::new();
    let mut lmap = vec![usize::MAX; p.n];
    for comp in &comps {
        // 局所の列番号
        let mut gcols: Vec<usize> = Vec::new();
        for &k in comp {
            for &(j, _) in &erows[k].0 {
                if lmap[j] == usize::MAX {
                    lmap[j] = gcols.len();
                    gcols.push(j);
                }
            }
        }
        let rows: Vec<Vec<(usize, i128)>> = comp.iter().map(|&k| erows[k].0.iter().map(|&(j, a)| (lmap[j], a)).collect()).collect();
        let b: Vec<i128> = comp.iter().map(|&k| erows[k].1).collect();
        let out = if gcols.len() > 200_000 { CompOut::GaveUp(2) } else { solve_comp(&rows, &b, gcols.len(), &mut work) };
        for &j in &gcols {
            lmap[j] = usize::MAX;
        }
        match out {
            CompOut::Infeasible => {
                st.infeasible = true;
                st.comps_done += 1;
                break;
            }
            CompOut::GaveUp(why) => {
                st.comps_gave_up += 1;
                match why {
                    0 => st.gave_overflow += 1,
                    1 => st.gave_work += 1,
                    _ => st.gave_size += 1,
                }
            }
            CompOut::Done(v, rank) => {
                st.comps_done += 1;
                st.cols += gcols.len();
                st.rank += rank;
                for (c, x0, g) in v {
                    let j = gcols[c];
                    let (lo, up) = (p.col_lo[j], p.col_up[j]);
                    let bin = lo >= 0.0 && up <= 1.0;
                    if g == 0 {
                        if x0.unsigned_abs() > (1u128 << 53) {
                            continue;
                        }
                        st.fixes += 1;
                        info.push((j, x0 as i64, 0));
                    } else {
                        if g > (1i128 << 52) {
                            continue;
                        }
                        let r = x0.rem_euclid(g) as i64;
                        let g = g as i64;
                        st.congs += 1;
                        st.congs_gen += (!bin) as usize;
                        st.max_mod = st.max_mod.max(g);
                        let lo2 = if lo.is_finite() && lo.abs() < 1e15 { lo + ((r - lo as i64).rem_euclid(g)) as f64 } else { lo };
                        let up2 = if up.is_finite() && up.abs() < 1e15 { up - ((up as i64 - r).rem_euclid(g)) as f64 } else { up };
                        st.congs_tighten += (lo2 != lo || up2 != up) as usize;
                        info.push((j, r, g));
                    }
                }
            }
        }
    }
    st.secs = t0.elapsed().as_secs_f64();
    HnfResult { stats: st, info }
}

/// 解析の結果で上下限を詰める (固定・合同式)。実行不能なら `Err`、詰めた列の数を返す。
pub(crate) fn apply_bounds(col_lo: &mut [f64], col_up: &mut [f64], info: &[(usize, i64, i64)]) -> Result<usize, ()> {
    let mut k = 0;
    for &(j, r, g) in info {
        let (lo, up) = (col_lo[j], col_up[j]);
        let (mut l2, mut u2) = (lo, up);
        if g == 0 {
            let v = r as f64;
            if v < lo - 1e-9 || v > up + 1e-9 {
                return Err(());
            }
            l2 = v;
            u2 = v;
        } else {
            if lo.is_finite() && lo.abs() < 1e15 {
                l2 = lo + ((r - lo.round() as i64).rem_euclid(g)) as f64;
                if lo.round() != lo {
                    l2 = lo.ceil() + ((r - lo.ceil() as i64).rem_euclid(g)) as f64;
                }
            }
            if up.is_finite() && up.abs() < 1e15 {
                let uf = up.floor();
                u2 = uf - ((uf as i64 - r).rem_euclid(g)) as f64;
            }
            if l2 > u2 {
                return Err(());
            }
        }
        if l2 != lo || u2 != up {
            col_lo[j] = l2;
            col_up[j] = u2;
            k += 1;
        }
    }
    Ok(k)
}

/// 表を作る (合同式だけ。固定は上下限に入れてあるので要らない)。
pub(crate) fn make_table(p: &MipProblem, info: &[(usize, i64, i64)]) -> Option<CongTable> {
    let mut r = vec![0i64; p.n];
    let mut m = vec![0i64; p.n];
    let mut any = false;
    for &(j, rr, g) in info {
        if g > 1 && p.col_lo[j] < p.col_up[j] {
            r[j] = rr;
            m[j] = g;
            any = true;
        }
    }
    any.then(|| CongTable { fp: super::rows_fingerprint(&p.rows), n: p.n, r, m })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(rows: Vec<Vec<(usize, i128)>>, b: Vec<i128>, nc: usize) -> CompOut {
        let mut w = 1_000_000;
        solve_comp(&rows, &b, nc, &mut w)
    }

    #[test]
    fn hnf_infeasible_gcd() {
        // 2x + 4y = 3
        assert!(matches!(run(vec![vec![(0, 2), (1, 4)]], vec![3], 2), CompOut::Infeasible));
        // x + y = 1, x - y = 0 (有理数の解 1/2 だけ)
        assert!(matches!(run(vec![vec![(0, 1), (1, 1)], vec![(0, 1), (1, -1)]], vec![1, 0], 2), CompOut::Infeasible));
    }

    #[test]
    fn hnf_congruence() {
        // 3x + 6y + 9z = 12 → 合同式なし (x + 2y + 3z = 4 で x は自由)。
        // 2x + 3y = 7 → x = 2 + 3t, y = 1 - 2t: x ≡ 2 (mod 3), y ≡ 1 (mod 2)
        match run(vec![vec![(0, 2), (1, 3)]], vec![7], 2) {
            CompOut::Done(v, rank) => {
                assert_eq!(rank, 1);
                let mut v: Vec<(usize, i128, i128)> = v.into_iter().map(|(c, x, g)| (c, x.rem_euclid(g.max(1)), g)).collect();
                v.sort();
                assert_eq!(v, vec![(0, 2, 3), (1, 1, 2)]);
            }
            _ => panic!(),
        }
        // x + y + z = 3, x - y = 0 → x = y、z = 3 - 2x: z ≡ 1 (mod 2)
        match run(vec![vec![(0, 1), (1, 1), (2, 1)], vec![(0, 1), (1, -1)]], vec![3, 0], 3) {
            CompOut::Done(v, _) => {
                let z: Vec<_> = v.iter().filter(|e| e.0 == 2).map(|e| (e.1.rem_euclid(e.2), e.2)).collect();
                assert_eq!(z, vec![(1, 2)]);
            }
            _ => panic!(),
        }
        // x - y = 2, y = 5 → 固定 x = 7、y = 5
        match run(vec![vec![(0, 1), (1, -1)], vec![(1, 1)]], vec![2, 5], 2) {
            CompOut::Done(mut v, _) => {
                v.sort();
                assert_eq!(v, vec![(0, 7, 0), (1, 5, 0)]);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn cong_round() {
        let t = CongTable { fp: 0, n: 1, r: vec![2], m: vec![3] };
        assert_eq!(t.round_up(0, 3.0), 5.0);
        assert_eq!(t.round_up(0, 5.0), 5.0);
        assert_eq!(t.round_down(0, 4.0), 2.0);
        assert_eq!(t.round_down(0, -1.0), -1.0);
        assert_eq!(t.round_down(0, -2.0), -4.0);
    }

    #[test]
    fn rat() {
        assert_eq!(to_rat(0.5, 1000), Some((1, 2)));
        assert_eq!(to_rat(-1.0 / 3.0, 1000), Some((-1, 3)));
        assert_eq!(to_rat(3.0, 1000), Some((3, 1)));
        assert_eq!(to_rat(std::f64::consts::PI, 1000), None);
    }
}
