//! 切除平面の生成 (HiGHS の `HighsCutGeneration` の CMIR 部分を簡略化したもの)。
//!
//! 元になる不等式 `sum_k a_k v_k <= b` (変数 `v_k` は構造変数と行の活動量 = 論理変数) から、
//! 境界の代入 (補変数化) → スケール `delta` の選択 → MIR の丸め、で切除平面を作る (Marchand & Wolsey 2001)。
//!
//! - 整数変数 `v` は LP 値に近い方の境界を基準に `v = l + y` か `v = u - y` (`y >= 0` は整数) に置き換える。
//! - 連続変数も近い方の境界で置き換え、係数が正になった `y` は捨て (`<=` を緩めるだけ)、負なら残す。
//! - MIR: `delta` で割った式 `sum ã_k y_k + sum g_k y_k <= β` (g_k < 0 は連続) に対し、`f0 = frac(β)` として
//!   `sum (floor(ã_k) + max(0, f_k - f0)/(1 - f0)) y_k + sum g_k/(1 - f0) y_k <= floor(β)`。
//! - 最後に `y` を元の変数に戻し、論理変数は行の式 `a_i x` に展開する (呼び出し側)。

/// 切除平面の生成に使う変数の情報。
pub struct CutVars<'a> {
    pub lo: &'a [f64],
    pub up: &'a [f64],
    pub is_int: &'a [bool],
    /// 現在の LP 解での値。
    pub x: &'a [f64],
    /// 連続変数の変数上下限 (なければ空)。
    pub vb: Option<&'a VarBounds>,
}

/// 連続変数 `x_k` の変数上下限 `x_k <= d y + e` (VUB) / `x_k >= d y + e` (VLB) (`y` は整数変数)。
/// 2 変数の行から集める。CMIR の置き換えで単純な上下限より近ければこちらを使う (HiGHS の
/// `HighsTransformedLp` と同じ)。
#[derive(Default)]
pub struct VarBounds {
    pub vub: Vec<Vec<(usize, f64, f64)>>,
    pub vlb: Vec<Vec<(usize, f64, f64)>>,
}

impl VarBounds {
    /// 行 `row_lo <= sum a_j x_j <= row_up` のうち、連続変数 1 つと整数変数 1 つからなるものを集める。
    pub fn from_rows(n: usize, is_int: &[bool], rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64]) -> Self {
        let mut vb = VarBounds { vub: vec![Vec::new(); n], vlb: vec![Vec::new(); n] };
        for (i, r) in rows.iter().enumerate() {
            if r.len() != 2 {
                continue;
            }
            let ((xk, a), (y, b)) = match (is_int[r[0].0], is_int[r[1].0]) {
                (false, true) => (r[0], r[1]),
                (true, false) => (r[1], r[0]),
                _ => continue,
            };
            if a.abs() < 1e-9 || b.abs() < 1e-9 {
                continue;
            }
            // a x + b y <= up  /  a x + b y >= lo
            for (rhs, le) in [(row_up[i], true), (row_lo[i], false)] {
                if !rhs.is_finite() {
                    continue;
                }
                let (d, e) = (-b / a, rhs / a);
                // a > 0 かつ <= なら上限、a < 0 なら向きが反転
                let upper = le == (a > 0.0);
                let list = if upper { &mut vb.vub[xk] } else { &mut vb.vlb[xk] };
                if list.len() < 4 {
                    list.push((y, d, e));
                }
            }
        }
        vb
    }
}

/// 連続変数の項の置き換え方。
#[derive(Clone, Copy, PartialEq)]
enum Sub {
    /// 単純な上下限 (`comp` で向き)。
    Simple,
    /// `x = d y + e - s` (VUB)。項の変数は `s`。
    Vub(usize, f64, f64),
    /// `x = d y + e + s` (VLB)。項の変数は `s`。
    Vlb(usize, f64, f64),
}

/// 生成した切除平面 `sum coef_k v_k <= rhs` と、その効き目 (違反量 / 係数のノルム)。
pub struct RawCut {
    pub coefs: Vec<(usize, f64)>,
    pub rhs: f64,
}

const MIN_FRAC: f64 = 0.005;
const MAX_SCALE: f64 = 1e6;

/// 置き換え後の項 (変数, 係数 (y について), 補変数化したか (v = u - y))。
#[derive(Clone, Copy)]
struct Term {
    k: usize,
    a: f64,
    comp: bool,
    int: bool,
    /// y の LP 値。
    yv: f64,
    /// y の上限 (u - l)。
    yu: f64,
    /// 連続変数を変数上下限で置き換えたか。
    sub: Sub,
}

/// 境界の代入 (補変数化) をした項と右辺。連続変数の正の係数の項は捨ててある。
/// 連続変数は単純な上下限と変数上下限のうち LP 値に近いものを使う (変数上下限なら整数変数の係数が増える)。
fn substitute(vars: &CutVars, base: &[(usize, f64)], rhs: f64) -> Option<(Vec<Term>, f64)> {
    let mut terms: Vec<Term> = Vec::with_capacity(base.len());
    let mut beta = rhs;
    // 整数変数の係数 (変数上下限の置き換えで増える分を含む)。base の順を保つ。
    let mut int_coef: Vec<(usize, f64)> = Vec::new();
    // 列 → int_coef の位置 (同じ列が何度現れても線形時間で足し込む)
    let mut int_pos: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let add_int = |k: usize, a: f64, int_coef: &mut Vec<(usize, f64)>, int_pos: &mut std::collections::HashMap<usize, usize>| match int_pos.get(&k) {
        Some(&p) => int_coef[p].1 += a,
        None => {
            int_pos.insert(k, int_coef.len());
            int_coef.push((k, a));
        }
    };
    for &(k, a) in base {
        if a.abs() < 1e-12 {
            continue;
        }
        let (l, u, x) = (vars.lo[k], vars.up[k], vars.x[k]);
        if l == u {
            beta -= a * l;
            continue;
        }
        if vars.is_int[k] {
            add_int(k, a, &mut int_coef, &mut int_pos);
            continue;
        }
        // 連続変数: 近い方の有限の境界 (単純な上下限) と、変数上下限の余裕を比べる
        let mut best: (f64, Sub, bool) = (f64::INFINITY, Sub::Simple, false); // (余裕, 置き換え, 上側か)
        if l.is_finite() {
            best = (x - l, Sub::Simple, false);
        }
        if u.is_finite() && u - x < best.0 {
            best = (u - x, Sub::Simple, true);
        }
        if let Some(vb) = vars.vb {
            if k < vb.vub.len() {
                for &(y, d, e) in &vb.vub[k] {
                    let slack = d * vars.x[y] + e - x;
                    if slack >= -1e-9 && slack < best.0 - 1e-9 {
                        best = (slack, Sub::Vub(y, d, e), true);
                    }
                }
                for &(y, d, e) in &vb.vlb[k] {
                    let slack = x - d * vars.x[y] - e;
                    if slack >= -1e-9 && slack < best.0 - 1e-9 {
                        best = (slack, Sub::Vlb(y, d, e), false);
                    }
                }
            }
        }
        if !best.0.is_finite() {
            return None;
        }
        match best.1 {
            Sub::Simple => {
                if !best.2 {
                    // v = l + y
                    beta -= a * l;
                    if a > 0.0 {
                        continue; // 正の係数の連続変数は捨てる
                    }
                    terms.push(Term { k, a, comp: false, int: false, yv: (x - l).max(0.0), yu: u - l, sub: Sub::Simple });
                } else {
                    // v = u - y
                    beta -= a * u;
                    if a < 0.0 {
                        continue;
                    }
                    terms.push(Term { k, a: -a, comp: true, int: false, yv: (u - x).max(0.0), yu: u - l, sub: Sub::Simple });
                }
            }
            Sub::Vub(y, d, e) => {
                // x = d y + e - s: a x = a d y + a e - a s
                beta -= a * e;
                add_int(y, a * d, &mut int_coef, &mut int_pos);
                if a < 0.0 {
                    continue; // s の係数 -a > 0: 捨てる
                }
                terms.push(Term { k, a: -a, comp: false, int: false, yv: best.0.max(0.0), yu: f64::INFINITY, sub: best.1 });
            }
            Sub::Vlb(y, d, e) => {
                // x = d y + e + s: a x = a d y + a e + a s
                beta -= a * e;
                add_int(y, a * d, &mut int_coef, &mut int_pos);
                if a > 0.0 {
                    continue;
                }
                terms.push(Term { k, a, comp: false, int: false, yv: best.0.max(0.0), yu: f64::INFINITY, sub: best.1 });
            }
        }
    }
    for (k, a) in int_coef {
        if a.abs() < 1e-12 {
            continue;
        }
        let (l, u, x) = (vars.lo[k], vars.up[k], vars.x[k]);
        if l == u {
            beta -= a * l;
            continue;
        }
        // 近い方の有限の境界を選ぶ
        let use_lower = match (l.is_finite(), u.is_finite()) {
            (true, true) => (x - l) <= (u - x),
            (true, false) => true,
            (false, true) => false,
            (false, false) => return None,
        };
        if use_lower {
            // v = l + y
            beta -= a * l;
            terms.push(Term { k, a, comp: false, int: true, yv: (x - l).max(0.0), yu: u - l, sub: Sub::Simple });
        } else {
            // v = u - y
            beta -= a * u;
            terms.push(Term { k, a: -a, comp: true, int: true, yv: (u - x).max(0.0), yu: u - l, sub: Sub::Simple });
        }
    }
    Some((terms, beta))
}

/// 置き換えた項 `c * (項の変数)` を元の変数の式に戻して `coefs` に足し、右辺の変化を返す
/// (戻り値を右辺に足す)。
fn unsubstitute(vars: &CutVars, t: &Term, c: f64, coefs: &mut Vec<(usize, f64)>) -> f64 {
    match t.sub {
        Sub::Simple => {
            let (l, u) = (vars.lo[t.k], vars.up[t.k]);
            if t.comp {
                // y = u - v
                coefs.push((t.k, -c));
                -c * u
            } else {
                // y = v - l
                coefs.push((t.k, c));
                c * l
            }
        }
        Sub::Vub(y, d, e) => {
            // s = d y + e - x
            coefs.push((t.k, -c));
            coefs.push((y, c * d));
            -c * e
        }
        Sub::Vlb(y, d, e) => {
            // s = x - d y - e
            coefs.push((t.k, c));
            coefs.push((y, -c * d));
            c * e
        }
    }
}

/// CMIR で切除平面を作る。作れなければ `None`。
pub fn cmir(vars: &CutVars, base: &[(usize, f64)], rhs: f64) -> Option<RawCut> {
    let (terms, beta) = substitute(vars, base, rhs)?;
    cmir_terms(vars, terms, beta)
}

/// [`cmir`] の本体 (置き換え済みの項と右辺から)。
fn cmir_terms(vars: &CutVars, terms: Vec<Term>, beta: f64) -> Option<RawCut> {
    if !terms.iter().any(|t| t.int) {
        return None;
    }
    // 2. delta の候補: LP 値が境界の内側にある整数変数の係数
    let mut deltas: Vec<f64> = Vec::new();
    for t in &terms {
        if t.int && t.yv > 1e-6 && t.yv < t.yu - 1e-6 && t.a.abs() > 1e-6 {
            let d = t.a.abs();
            if !deltas.iter().any(|&e| (e - d).abs() <= 1e-9 * d) {
                deltas.push(d);
            }
        }
        if deltas.len() >= 8 {
            break;
        }
    }
    if deltas.is_empty() {
        // 整数変数の係数の最大値で試す
        let m = terms.iter().filter(|t| t.int).map(|t| t.a.abs()).fold(0.0, f64::max);
        if m > 0.0 {
            deltas.push(m);
        }
    }
    let base_deltas = deltas.clone();
    for d in base_deltas {
        for div in [2.0, 4.0, 8.0] {
            deltas.push(d / div);
        }
    }
    // 3. 各 delta で MIR を作り、違反量 / ノルムの最大のものを選ぶ
    let mut best: Option<(f64, f64)> = None; // (delta, efficacy)
    for &delta in &deltas {
        if let Some(eff) = mir_efficacy(&terms, beta, delta) {
            if best.is_none_or(|(_, e)| eff > e) {
                best = Some((delta, eff));
            }
        }
    }
    let (delta, eff) = best?;
    cmir_build(vars, terms, beta, delta, eff)
}

/// δ を決めた後の CMIR: 補変数化の向きを 1 つずつ反転して効き目が上がるか試し、切除平面を作って元の変数に戻す。
fn cmir_build(vars: &CutVars, terms: Vec<Term>, beta: f64, delta: f64, eff: f64) -> Option<RawCut> {
    let mut eff = eff;
    // 4. 補変数化の向きを 1 つずつ反転して改善するか試す (両側有限の整数変数のみ)
    let mut terms = terms;
    let mut beta = beta;
    let mut flips_tried = 0;
    for idx in 0..terms.len() {
        let t = terms[idx];
        if !t.int || !t.yu.is_finite() || t.yv <= 1e-6 {
            continue;
        }
        // 長い行で手間が 2 乗にならないよう、試す数を制限する
        flips_tried += 1;
        if flips_tried > 50 {
            break;
        }
        // y' = yu - y: a y = a yu - a y'
        let mut t2 = t;
        t2.a = -t.a;
        t2.comp = !t.comp;
        t2.yv = t.yu - t.yv;
        let beta2 = beta - t.a * t.yu;
        let old = terms[idx];
        terms[idx] = t2;
        match mir_efficacy(&terms, beta2, delta) {
            Some(e) if e > eff + 1e-9 => {
                eff = e;
                beta = beta2;
            }
            _ => terms[idx] = old,
        }
    }
    // 5. 切除平面を作って元の変数に戻す
    let (ycoef, ybeta) = mir_coefs(&terms, beta, delta)?;
    let mut coefs: Vec<(usize, f64)> = Vec::with_capacity(terms.len());
    let mut r = ybeta;
    for (t, &c) in terms.iter().zip(&ycoef) {
        if c == 0.0 {
            continue;
        }
        r += unsubstitute(vars, t, c, &mut coefs);
    }
    if coefs.is_empty() {
        return None;
    }
    Some(RawCut { coefs, rhs: r })
}

/// MIR の係数 (y について) と右辺。`f0` が極端なら `None`。
fn mir_coefs(terms: &[Term], beta: f64, delta: f64) -> Option<(Vec<f64>, f64)> {
    let b = beta / delta;
    let f0 = b - b.floor();
    if !(MIN_FRAC..=1.0 - MIN_FRAC).contains(&f0) || b.abs() > MAX_SCALE {
        return None;
    }
    let mut c = Vec::with_capacity(terms.len());
    for t in terms {
        let at = t.a / delta;
        if t.int {
            let fj = at - at.floor();
            c.push(at.floor() + (fj - f0).max(0.0) / (1.0 - f0));
        } else {
            // 残っている連続変数は係数が負
            c.push(at / (1.0 - f0));
        }
    }
    Some((c, b.floor()))
}

/// MIR の効き目 (y 空間での違反量 / 係数のノルム)。違反しなければ `None`。
fn mir_efficacy(terms: &[Term], beta: f64, delta: f64) -> Option<f64> {
    let (c, rb) = mir_coefs(terms, beta, delta)?;
    let mut act = 0.0;
    let mut norm = 0.0;
    for (t, &cj) in terms.iter().zip(&c) {
        act += cj * t.yv;
        norm += cj * cj;
    }
    let viol = act - rb;
    if viol <= 1e-6 || norm <= 0.0 {
        return None;
    }
    Some(viol / norm.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_on_knapsack() {
        // 3x0 + 4x1 + 5x2 <= 7 (0-1)、LP 点 (1, 1, 0) は違反しない; (0.8, 0.8, 0.2) なら cover {0,1}: x0 + x1 <= 1 ... 拡張で x2 も
        let lo = [0.0; 3];
        let up = [1.0; 3];
        let is_int = [true; 3];
        let x = [0.9, 0.9, 0.2];
        let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &x, vb: None };
        let cut = extended_cover(&vars, &[(0, 3.0), (1, 4.0), (2, 5.0)], 6.0).expect("cover");
        let act: f64 = cut.coefs.iter().map(|&(k, c)| c * x[k]).sum();
        assert!(act > cut.rhs + 1e-6);
        for pt in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, 0.0]] {
            let a: f64 = cut.coefs.iter().map(|&(k, c)| c * pt[k]).sum();
            assert!(a <= cut.rhs + 1e-9);
        }
    }

    #[test]
    fn mir_with_variable_upper_bound() {
        // x continuous in [0, 100], y binary, x <= 10 y (VUB), base row x >= 3. LP point (3, 0.3)
        // → with the VUB substitution the cut is y >= 1
        let lo = [0.0, 0.0];
        let up = [100.0, 1.0];
        let is_int = [false, true];
        let x = [3.0, 0.3];
        let vb = VarBounds::from_rows(2, &is_int, &[vec![(0, 1.0), (1, -10.0)]], &[f64::NEG_INFINITY], &[0.0]);
        assert_eq!(vb.vub[0], vec![(1, 10.0, 0.0)]);
        let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &x, vb: Some(&vb) };
        let cut = cmir(&vars, &[(0, -1.0)], -3.0).expect("cut");
        let act: f64 = cut.coefs.iter().map(|&(k, c)| c * x[k]).sum();
        assert!(act > cut.rhs + 1e-6, "LP point must be cut off");
        for pt in [[3.0, 1.0], [10.0, 1.0], [5.0, 1.0]] {
            let a: f64 = cut.coefs.iter().map(|&(k, c)| c * pt[k]).sum();
            assert!(a <= cut.rhs + 1e-9, "valid point {pt:?} cut off: {a} > {}", cut.rhs);
        }
        // without the VUB no cut exists (the row has no integer variable)
        let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &x, vb: None };
        assert!(cmir(&vars, &[(0, -1.0)], -3.0).is_none());
    }

    #[test]
    fn flow_cover_textbook_example() {
        // x1 <= 3 y1, x2 <= 5 y2, x1 + x2 <= 6、LP 点 (3, 3, 1, 0.6) → x1 + x2 - y1 - 3 y2 <= 2 相当
        let lo = [0.0, 0.0, 0.0, 0.0];
        let up = [3.0, 5.0, 1.0, 1.0];
        let is_int = [false, false, true, true];
        let x = [3.0, 3.0, 1.0, 0.6];
        let vb = VarBounds::from_rows(4, &is_int, &[vec![(0, 1.0), (2, -3.0)], vec![(1, 1.0), (3, -5.0)]], &[f64::NEG_INFINITY; 2], &[0.0; 2]);
        let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &x, vb: Some(&vb) };
        let cut = lifted_flow_cover(&vars, &[(0, 1.0), (1, 1.0)], 6.0).expect("flow cover");
        let act: f64 = cut.coefs.iter().map(|&(k, c)| c * x[k]).sum();
        assert!(act > cut.rhs + 0.5, "violation {} too small", act - cut.rhs);
        for pt in [[3.0, 3.0, 1.0, 1.0], [1.0, 5.0, 1.0, 1.0], [3.0, 0.0, 1.0, 0.0], [0.0, 5.0, 0.0, 1.0], [0.0, 0.0, 0.0, 0.0]] {
            let a: f64 = cut.coefs.iter().map(|&(k, c)| c * pt[k]).sum();
            assert!(a <= cut.rhs + 1e-9, "valid point {pt:?} cut off");
        }
    }

    /// 乱数の単一行 (変数上限つきフロー・0-1 変数・有界な連続変数) で、lifted flow cover が全ての混合整数
    /// 実行可能点で成り立つことを総当たりで確かめる。
    #[test]
    fn flow_cover_random_validity() {
        let mut seed = 12345u64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut cuts_found = 0;
        for _case in 0..3000 {
            // 変数: 0-1 変数 nb 個、連続変数 nc 個 (うち一部は 0-1 変数の変数上限つき)
            let nb = 1 + (rnd() * 4.0) as usize;
            let nc = (rnd() * 4.0) as usize;
            let n = nb + nc;
            let mut lo = vec![0.0; n];
            let mut up = vec![1.0; n];
            let mut is_int = vec![true; n];
            let mut vub_rows: Vec<Vec<(usize, f64)>> = Vec::new();
            let mut vub_of: Vec<Option<(usize, f64)>> = vec![None; n];
            let mut used = vec![false; nb];
            for k in nb..n {
                is_int[k] = false;
                let cap = (1.0 + (rnd() * 8.0).floor()).max(1.0);
                // 変数上限に使う 0-1 変数 (行には入れない)
                let y = (rnd() * nb as f64) as usize % nb;
                if rnd() < 0.7 && !used[y] {
                    used[y] = true;
                    lo[k] = 0.0;
                    up[k] = cap;
                    vub_rows.push(vec![(k, 1.0), (y, -cap)]);
                    vub_of[k] = Some((y, cap));
                } else {
                    lo[k] = -(rnd() * 3.0).floor();
                    up[k] = lo[k] + cap;
                }
            }
            // 行: 変数上限に使われない 0-1 変数と、全ての連続変数
            let mut base: Vec<(usize, f64)> = Vec::new();
            for k in 0..n {
                if k < nb && used[k] {
                    continue;
                }
                if rnd() < 0.85 {
                    let a = ((rnd() - 0.35) * 10.0).round();
                    if a != 0.0 {
                        base.push((k, a));
                    }
                }
            }
            if base.is_empty() {
                continue;
            }
            let rhs = ((rnd() - 0.2) * 12.0).round();
            // LP 点: 範囲内の乱数 (変数上限も満たす)
            let mut x = vec![0.0; n];
            for k in 0..n {
                x[k] = lo[k] + rnd() * (up[k] - lo[k]);
            }
            for k in nb..n {
                if let Some((y, d)) = vub_of[k] {
                    x[k] = x[k].min(d * x[y]);
                }
            }
            let vb = VarBounds::from_rows(n, &is_int, &vub_rows, &vec![f64::NEG_INFINITY; vub_rows.len()], &vec![0.0; vub_rows.len()]);
            let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &x, vb: Some(&vb) };
            let Some(cut) = lifted_flow_cover(&vars, &base, rhs) else { continue };
            cuts_found += 1;
            let ccoef = |k: usize| cut.coefs.iter().filter(|&&(j, _)| j == k).map(|&(_, c)| c).sum::<f64>();
            // 0-1 変数を全て列挙し、連続変数は箱 ∩ 行 の頂点 (高々 1 つが境界の内側) を列挙して最大違反を調べる
            let conts: Vec<usize> = (nb..n).collect();
            for mask in 0..(1u32 << nb) {
                let yv: Vec<f64> = (0..nb).map(|i| ((mask >> i) & 1) as f64).collect();
                let box_of = |k: usize| -> (f64, f64) {
                    match vub_of[k] {
                        Some((y, d)) => (0.0, d * yv[y]),
                        None => (lo[k], up[k]),
                    }
                };
                let a_of = |k: usize| base.iter().filter(|&&(j, _)| j == k).map(|&(_, a)| a).sum::<f64>();
                let row_bin: f64 = (0..nb).map(|k| a_of(k) * yv[k]).sum();
                let cut_bin: f64 = (0..nb).map(|k| ccoef(k) * yv[k]).sum();
                let nc = conts.len();
                for free in 0..=nc {
                    for bmask in 0..(1u32 << nc) {
                        let mut xc = vec![0.0; nc];
                        for (t, &k) in conts.iter().enumerate() {
                            let (l, u) = box_of(k);
                            xc[t] = if (bmask >> t) & 1 == 1 { u } else { l };
                        }
                        if free < nc {
                            // 自由な 1 変数で行を等号にする
                            let k = conts[free];
                            let a = a_of(k);
                            if a == 0.0 {
                                continue;
                            }
                            let rest: f64 = conts.iter().enumerate().filter(|&(t, _)| t != free).map(|(t, &j)| a_of(j) * xc[t]).sum();
                            let v = (rhs - row_bin - rest) / a;
                            let (l, u) = box_of(k);
                            if v < l - 1e-9 || v > u + 1e-9 {
                                continue;
                            }
                            xc[free] = v;
                        }
                        let row: f64 = row_bin + conts.iter().enumerate().map(|(t, &k)| a_of(k) * xc[t]).sum::<f64>();
                        if row > rhs + 1e-9 {
                            continue;
                        }
                        let lhs = cut_bin + conts.iter().enumerate().map(|(t, &k)| ccoef(k) * xc[t]).sum::<f64>();
                        assert!(
                            lhs <= cut.rhs + 1e-6,
                            "invalid flow cover: base {base:?} <= {rhs}, vubs {vub_of:?}, bounds {lo:?} {up:?}, x* {x:?}\n cut {:?} <= {}\n point y {yv:?} x {xc:?}: {lhs}",
                            cut.coefs,
                            cut.rhs
                        );
                    }
                }
            }
        }
        assert!(cuts_found > 100, "only {cuts_found} cuts generated");
    }

    #[test]
    fn mir_on_simple_row() {
        // x integer in [0, 10], y continuous >= 0: x - y <= 2.5, LP point (2.5, 0) → cut x - 2y <= 2
        let lo = [0.0, 0.0];
        let up = [10.0, f64::INFINITY];
        let is_int = [true, false];
        let x = [2.5, 0.0];
        let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &x, vb: None };
        let cut = cmir(&vars, &[(0, 1.0), (1, -1.0)], 2.5).expect("cut");
        let act: f64 = cut.coefs.iter().map(|&(k, c)| c * x[k]).sum();
        assert!(act > cut.rhs + 1e-6, "LP point must be cut off");
        // 整数点 (2, 0), (3, 0.5) は満たす
        for pt in [[2.0, 0.0], [3.0, 0.5], [5.0, 2.5]] {
            let a: f64 = cut.coefs.iter().map(|&(k, c)| c * pt[k]).sum();
            assert!(a <= cut.rhs + 1e-9, "valid point {pt:?} cut off: {a} > {}", cut.rhs);
        }
    }
}

/// 拡張カバー不等式 (lifted cover の簡略版): 置き換え後の式が 0-1 変数だけのナップサック
/// `sum a_k y_k <= beta` なら、LP 値の大きい順にカバー C (`sum_C a_k > beta`) を作り、
/// `sum_{E(C)} y_k <= |C| - 1` (`E(C)` は C と、C の最大係数以上の係数の変数) を作る。
pub fn extended_cover(vars: &CutVars, base: &[(usize, f64)], rhs: f64) -> Option<RawCut> {
    let (terms, beta) = substitute(vars, base, rhs)?;
    // 0-1 変数だけ (連続変数の項が残っていればナップサックではない)
    if terms.is_empty() || terms.iter().any(|t| !t.int || t.yu != 1.0) {
        return None;
    }
    // 係数を正にそろえる (負の係数の y は 1 - y に置き換える)
    let mut items: Vec<(usize, f64, bool, f64)> = Vec::with_capacity(terms.len()); // (項の番号, 係数, 反転, y の LP 値)
    let mut b = beta;
    for (idx, t) in terms.iter().enumerate() {
        if t.a > 0.0 {
            items.push((idx, t.a, false, t.yv));
        } else if t.a < 0.0 {
            b -= t.a;
            items.push((idx, -t.a, true, 1.0 - t.yv));
        }
    }
    if b < 0.0 {
        return None;
    }
    // カバー: LP 値の大きい順 (同点は係数の大きい順)
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&x, &y| items[y].3.total_cmp(&items[x].3).then(items[y].1.total_cmp(&items[x].1)));
    let mut sum = 0.0;
    let mut cover: Vec<usize> = Vec::new();
    for &k in &order {
        cover.push(k);
        sum += items[k].1;
        if sum > b + 1e-9 * (1.0 + b.abs()) {
            break;
        }
    }
    if sum <= b + 1e-9 * (1.0 + b.abs()) {
        return None;
    }
    let amax = cover.iter().map(|&k| items[k].1).fold(0.0, f64::max);
    let mut in_cover = vec![false; items.len()];
    for &k in &cover {
        in_cover[k] = true;
    }
    let ext: Vec<usize> = (0..items.len()).filter(|&k| in_cover[k] || items[k].1 >= amax).collect();
    let rhs_y = cover.len() as f64 - 1.0;
    let act: f64 = ext.iter().map(|&k| items[k].3).sum();
    if act <= rhs_y + 1e-6 {
        return None;
    }
    // 元の変数に戻す: z (反転後の 0-1) = y または 1 - y、y = v - l または u - v
    let mut coefs: Vec<(usize, f64)> = Vec::with_capacity(ext.len());
    let mut r = rhs_y;
    for &k in &ext {
        let (idx, _, flip, _) = items[k];
        let t = terms[idx];
        // z = y (flip なし) または 1 - y
        let (cy, c0) = if flip { (-1.0, 1.0) } else { (1.0, 0.0) };
        r -= c0;
        r += unsubstitute(vars, &t, cy, &mut coefs);
    }
    Some(RawCut { coefs, rhs: r })
}

/// 0-1 単一節点フロー集合の 1 項 (lifted flow cover 用)。`sign` が +1 なら N1、-1 なら N2。
/// フロー `f = sum flow + flow_const` (元の変数の式) は `0 <= f <= u * x` を満たす (`x` は 0-1 変数 `bin`、
/// `None` なら定数 1)。
struct SnfItem {
    sign: f64,
    u: f64,
    bin: Option<usize>,
    /// x の LP 値 (`bin` が `None` なら 1)。
    xv: f64,
    flow: Vec<(usize, f64)>,
    flow_const: f64,
}

/// 2 値 (大域的な境界が [0, 1] の整数) か。
fn is_binary(vars: &CutVars, k: usize) -> bool {
    vars.is_int[k] && vars.lo[k] == 0.0 && vars.up[k] == 1.0
}

/// 不等式 `sum a_k v_k <= rhs` を 0-1 単一節点フロー緩和 `sum_{N1} f - sum_{N2} f <= b`、`0 <= f_i <= u_i x_i` に
/// 直す (SCIP `cuts.c` の `constructSNFRelaxation` の簡略版)。
/// - 連続変数 x (係数 a) は、0-1 変数 y の変数上限 `x <= d y` (y がこの行に現れないもの) があり下限が 0 で、
///   LP 値で単純な上限より近ければ `|a| x <= |a| d y` の項にする。
/// - それ以外の連続変数と一般整数・行の活動量は、下限 l (有限) で `x = l + x'` として `|a| x' <= |a| (u - l) * 1`
///   (0-1 変数は定数 1) の項にする。係数が正で上限が無限なら (外しても成り立つので) 下限に固定して外す。
/// - 0-1 変数 y (変数上限に使われないもの) は `|a| y <= |a| y` の項にする。
fn snf_relaxation(vars: &CutVars, base: &[(usize, f64)], rhs: f64) -> Option<(Vec<SnfItem>, f64)> {
    let mut b = rhs;
    // 行に現れる変数の係数
    let row_set: std::collections::HashSet<usize> = base.iter().filter(|&&(_, a)| a != 0.0).map(|&(j, _)| j).collect();
    let in_row = |k: usize| row_set.contains(&k);
    let mut used_bin: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut items: Vec<SnfItem> = Vec::with_capacity(base.len());
    // 先に連続変数 (変数上限で 0-1 変数を使うかを決める)
    let mut pending_bin: Vec<(usize, f64)> = Vec::new();
    for &(k, a) in base {
        if a == 0.0 {
            continue;
        }
        let (l, u, x) = (vars.lo[k], vars.up[k], vars.x[k]);
        if l == u {
            b -= a * l;
            continue;
        }
        if is_binary(vars, k) {
            pending_bin.push((k, a));
            continue;
        }
        let sign = if a > 0.0 { 1.0 } else { -1.0 };
        // 変数上限 x <= d y (e = 0、下限 0、y は行に現れない 0-1 変数)
        let mut vub: Option<(usize, f64)> = None;
        if !vars.is_int[k] && l == 0.0 {
            if let Some(vb) = vars.vb {
                if k < vb.vub.len() {
                    let simple_slack = if u.is_finite() { u - x } else { f64::INFINITY };
                    for &(y, d, e) in &vb.vub[k] {
                        if e == 0.0 && d > 0.0 && is_binary(vars, y) && !in_row(y) && !used_bin.contains(&y) {
                            let slack = d * vars.x[y] - x;
                            if slack <= simple_slack && vub.is_none_or(|(y0, d0)| slack < d0 * vars.x[y0] - x) {
                                vub = Some((y, d));
                            }
                        }
                    }
                }
            }
        }
        if let Some((y, d)) = vub {
            used_bin.insert(y);
            items.push(SnfItem { sign, u: a.abs() * d, bin: Some(y), xv: vars.x[y], flow: vec![(k, a.abs())], flow_const: 0.0 });
            continue;
        }
        // 単純な上下限 (LP 値に近い有限の側) で置き換える
        let use_lower = l.is_finite() && (!u.is_finite() || x - l <= u - x);
        if !use_lower && !u.is_finite() {
            return None;
        }
        let cap = a.abs() * (u - l);
        if use_lower {
            // x = l + x': a x = a l + a x'
            b -= a * l;
            if a > 0.0 && !cap.is_finite() {
                continue; // a x' >= 0 を外す (緩和)
            }
            if !cap.is_finite() {
                return None; // N2 の容量が無限だと被覆を作れない
            }
            items.push(SnfItem { sign, u: cap, bin: None, xv: 1.0, flow: vec![(k, a.abs())], flow_const: -a.abs() * l });
        } else {
            // x = u - x': a x = a u - a x' (向きが反転する)
            b -= a * u;
            if a < 0.0 && !cap.is_finite() {
                continue; // -a x' >= 0 を外す (緩和)
            }
            if !cap.is_finite() {
                return None;
            }
            items.push(SnfItem { sign: -sign, u: cap, bin: None, xv: 1.0, flow: vec![(k, -a.abs())], flow_const: a.abs() * u });
        }
    }
    for (k, a) in pending_bin {
        if used_bin.contains(&k) {
            return None; // 行に現れる 0-1 変数は変数上限に使わないので来ないはず
        }
        let sign = if a > 0.0 { 1.0 } else { -1.0 };
        items.push(SnfItem { sign, u: a.abs(), bin: Some(k), xv: vars.x[k], flow: vec![(k, a.abs())], flow_const: 0.0 });
    }
    if !items.iter().any(|it| it.bin.is_some()) {
        return None;
    }
    Some((items, b))
}

/// lifted simple generalized flow cover 不等式 (Gu, Nemhauser & Savelsbergh 1999) を作る。SCIP `cuts.c` の
/// `getFlowCover` (被覆はナップサックの貪欲解) と `generateLiftedFlowCoverCut` (順序に依らない持ち上げ) の移植。
pub fn lifted_flow_cover(vars: &CutVars, base: &[(usize, f64)], rhs: f64) -> Option<RawCut> {
    let r = lifted_flow_cover_impl(vars, base, rhs);
    if env_str!("ENOMOTO_MIP_DEBUG_FC").is_some() {
        FC_STATS.with(|s| {
            let mut s = s.borrow_mut();
            s.0 += 1;
            if r.is_some() {
                s.1 += 1;
            }
        });
    }
    r
}

thread_local! {
    /// 診断用: lifted flow cover の (呼び出し, 生成) 回数。
    pub(crate) static FC_STATS: std::cell::RefCell<(u64, u64, u64)> = const { std::cell::RefCell::new((0, 0, 0)) };
}

fn lifted_flow_cover_impl(vars: &CutVars, base: &[(usize, f64)], rhs: f64) -> Option<RawCut> {
    const FEAS: f64 = 1e-6;
    let Some((items, b)) = snf_relaxation(vars, base, rhs) else {
        if env_str!("ENOMOTO_MIP_DEBUG_FC").is_some() {
            FC_STATS.with(|s| s.borrow_mut().2 += 1);
        }
        return None;
    };
    let n = items.len();
    // 1. 被覆 (C1 ⊆ N1, C2 ⊆ N2): x* が整数の項は先に決め、残りをナップサックの貪欲解で決める
    let mut in_cover = vec![false; n];
    let mut cover_w = 0.0; // sum_{C1} u - sum_{C2} u
    let mut free: Vec<usize> = Vec::new();
    let mut n1_free_w = 0.0;
    for (i, it) in items.iter().enumerate() {
        if it.u <= FEAS {
            continue;
        }
        let frac = it.xv > FEAS && it.xv < 1.0 - FEAS;
        if frac {
            free.push(i);
            if it.sign > 0.0 {
                n1_free_w += it.u;
            }
        } else if it.xv > 0.5 {
            in_cover[i] = true;
            cover_w += it.sign * it.u;
        }
    }
    let cap = -b + cover_w + n1_free_w;
    if cap / 10.0 <= FEAS {
        return None;
    }
    if !free.is_empty() {
        // KP: N1 は補変数 z° (利益 1 - x*)、N2 は z (利益 x*)、重み u、容量 cap 未満
        let profit = |i: usize| if items[i].sign > 0.0 { 1.0 - items[i].xv } else { items[i].xv };
        free.sort_by(|&p, &q| (profit(q) / items[q].u).total_cmp(&(profit(p) / items[p].u)));
        let mut w = 0.0;
        for &i in &free {
            // 貪欲に詰める (入らない項は飛ばして続ける)
            let take = w + items[i].u < cap - FEAS * cap.abs().max(1.0);
            // 解に入った N1 の項は被覆の外、入らなかった N1 の項は被覆、N2 はその逆
            if take {
                w += items[i].u;
                if items[i].sign < 0.0 {
                    in_cover[i] = true;
                    cover_w -= items[i].u;
                }
            } else if items[i].sign > 0.0 {
                in_cover[i] = true;
                cover_w += items[i].u;
            }
        }
    }
    let lambda = cover_w - b;
    if lambda <= FEAS {
        return None;
    }
    // 2. 持ち上げ関数のデータ (computeLiftingData)
    let mut m: Vec<f64> = Vec::new();
    let (mut sum_n2mc2_le, mut sum_n2mc2_gt, mut sum_c1_le, mut sum_c2) = (0.0, 0.0, 0.0, 0.0);
    let mut mp = f64::INFINITY;
    for (i, it) in items.iter().enumerate() {
        match (it.sign > 0.0, in_cover[i]) {
            (false, false) => {
                if it.u > lambda + FEAS {
                    sum_n2mc2_gt += it.u;
                    m.push(it.u);
                } else {
                    sum_n2mc2_le += it.u;
                }
            }
            (false, true) => sum_c2 += it.u,
            (true, true) => {
                if it.u > lambda + FEAS {
                    m.push(it.u);
                    mp = mp.min(it.u);
                } else {
                    sum_c1_le += it.u;
                }
            }
            (true, false) => {}
        }
    }
    if !mp.is_finite() {
        return None;
    }
    let _ = sum_n2mc2_gt;
    let ml = lambda.min(sum_c1_le + sum_n2mc2_le);
    let d1 = sum_c2 + b;
    m.sort_by(|a, b| b.total_cmp(a));
    let r = m.len();
    let mut mm = vec![0.0; r + 1];
    for i in 0..r {
        mm[i + 1] = mm[i] + m[i];
    }
    // t: m[t-1] == mp となる最大の t (1 始まり)
    let mut t = m.iter().position(|&v| v == mp).map_or(r, |p| p + 1);
    while t < r && m[t] == mp {
        t += 1;
    }
    let eps = 1e-9;
    let lift = |x: f64| -> f64 {
        let xl = x + lambda;
        let mut i = 0;
        while i < r && xl > mm[i + 1] + eps {
            i += 1;
        }
        if i < t {
            if mm[i] <= x + eps {
                return i as f64 * lambda;
            }
            return i as f64 * lambda + x - mm[i];
        }
        if i < r {
            let p = (m[i] - mp - ml + lambda).max(0.0);
            if mm[i] + ml + p < xl - eps {
                return i as f64 * lambda;
            }
            return i as f64 * lambda + x - mm[i];
        }
        r as f64 * lambda + x - mm[r]
    };
    let alpha_beta = |u: f64| -> (bool, f64) {
        let ul = u + lambda;
        let mut i = 0;
        while i < r && ul > mm[i + 1] + eps {
            i += 1;
        }
        if u < mm[i] - eps {
            (true, mm[i] - i as f64 * lambda)
        } else {
            (false, 0.0)
        }
    };
    // 3. 切除平面 (generateLiftedFlowCoverCut)
    let mut coefs: Vec<(usize, f64)> = Vec::new();
    let mut r_hs = d1;
    for (i, it) in items.iter().enumerate() {
        match (it.sign > 0.0, in_cover[i]) {
            (false, false) => {
                if it.u > lambda + FEAS {
                    // L-: -λ x
                    match it.bin {
                        Some(y) => coefs.push((y, -lambda)),
                        None => r_hs += lambda,
                    }
                } else {
                    // L--: -f
                    for &(k, c) in &it.flow {
                        coefs.push((k, -c));
                    }
                    r_hs += it.flow_const;
                }
            }
            (false, true) => {
                // C2: -g(u) x と右辺 -g(u)
                if let Some(y) = it.bin {
                    let g = lift(it.u);
                    if g != 0.0 {
                        coefs.push((y, -g));
                        r_hs -= g;
                    }
                }
            }
            (true, false) => {
                // N1 \ C1: α = 1 なら f - β x
                let (alpha, beta) = alpha_beta(it.u);
                if alpha {
                    for &(k, c) in &it.flow {
                        coefs.push((k, c));
                    }
                    match it.bin {
                        Some(y) => coefs.push((y, -beta)),
                        None => r_hs += beta,
                    }
                    r_hs -= it.flow_const;
                }
            }
            (true, true) => {
                // C1: f + (u - λ)^+ (1 - x)
                for &(k, c) in &it.flow {
                    coefs.push((k, c));
                }
                let mut constant = it.flow_const;
                if let Some(y) = it.bin {
                    if it.u > lambda + FEAS {
                        constant += it.u - lambda;
                        coefs.push((y, -(it.u - lambda)));
                    }
                }
                r_hs -= constant;
            }
        }
    }
    if coefs.is_empty() {
        return None;
    }
    // 違反しているか (finish_cut でも確かめるが、ここで早めに捨てる)
    let act: f64 = coefs.iter().map(|&(k, c)| c * vars.x[k]).sum();
    if act <= r_hs + 1e-6 {
        return None;
    }
    Some(RawCut { coefs, rhs: r_hs })
}

/// 診断用: HiGHS の経路集約が `generateCut` に渡した集約行 (HiGHS をソースから計測して書き出したもの) を、こちらの
/// CMIR・lifted flow cover に通し、できたカットの効き目を HiGHS のカットと比べる
/// (`HIGHS_CUT_DUMP=ファイル cargo test --release highs_cut_dump -- --ignored --nocapture`)。
#[cfg(test)]
mod highs_dump {
    use super::*;

    #[test]
    #[ignore]
    fn highs_cut_dump() {
        let Ok(path) = std::env::var("HIGHS_CUT_DUMP") else { return };
        let text = std::fs::read_to_string(path).unwrap();
        let (mut n, mut m) = (0usize, 0usize);
        let (mut lo, mut up, mut isint, mut x) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
        let (mut rlo, mut rup) = (Vec::new(), Vec::new());
        let mut bases: Vec<Vec<(usize, f64)>> = Vec::new();
        let mut res: Vec<Option<(Vec<(usize, f64)>, f64)>> = Vec::new();
        let num = |s: &str| -> f64 {
            match s {
                "inf" => f64::INFINITY,
                "-inf" => f64::NEG_INFINITY,
                _ => s.parse().unwrap(),
            }
        };
        for line in text.lines() {
            let w: Vec<&str> = line.split_whitespace().collect();
            match w[0] {
                "N" => {
                    n = w[1].parse().unwrap();
                    m = w[2].parse().unwrap();
                }
                "C" => {
                    lo.push(num(w[1]));
                    up.push(num(w[2]));
                    isint.push(w[3] == "1");
                    x.push(num(w[4]));
                }
                "R" => {
                    rlo.push(num(w[1]));
                    rup.push(num(w[2]));
                    let act = num(w[3]);
                    let len: usize = w[4].parse().unwrap();
                    rows.push((0..len).map(|k| (w[5 + 2 * k].parse().unwrap(), num(w[6 + 2 * k]))).collect());
                    let _ = act;
                }
                "B" => {
                    let len: usize = w[1].parse().unwrap();
                    bases.push((0..len).map(|k| (w[2 + 2 * k].parse().unwrap(), num(w[3 + 2 * k]))).collect());
                }
                "X" => {
                    if w[1] == "1" {
                        let rhs = num(w[2]);
                        let len: usize = w[3].parse().unwrap();
                        res.push(Some(((0..len).map(|k| (w[4 + 2 * k].parse().unwrap(), num(w[5 + 2 * k]))).collect(), rhs)));
                    } else {
                        res.push(None);
                    }
                }
                _ => {}
            }
        }
        assert_eq!(bases.len(), res.len());
        // 行の活動量 (論理変数) も変数として並べる
        let mut vlo = lo.clone();
        let mut vup = up.clone();
        let mut vx = x.clone();
        let mut vint = isint.clone();
        for i in 0..m {
            vlo.push(rlo[i]);
            vup.push(rup[i]);
            vx.push(rows[i].iter().map(|&(j, a)| a * x[j]).sum());
            vint.push(false);
        }
        let vb = VarBounds::from_rows(n, &isint, &rows, &rlo, &rup);
        let vars = CutVars { lo: &vlo, up: &vup, is_int: &vint, x: &vx, vb: Some(&vb) };
        // 論理変数を行の式に展開して、効き目 (違反量 / ノルム) を測る
        let eff = |coefs: &[(usize, f64)], rhs: f64| -> f64 {
            let mut d: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
            for &(k, a) in coefs {
                if k < n {
                    *d.entry(k).or_default() += a;
                } else {
                    for &(j, b) in &rows[k - n] {
                        *d.entry(j).or_default() += a * b;
                    }
                }
            }
            let act: f64 = d.iter().map(|(&j, &a)| a * x[j]).sum();
            let norm = d.values().map(|a| a * a).sum::<f64>().sqrt();
            if norm < 1e-12 { 0.0 } else { (act - rhs) / norm }
        };
        let (mut both, mut only_h, mut only_o, mut none) = (0, 0, 0, 0);
        let (mut sum_h, mut sum_o) = (0.0, 0.0);
        let mut shown = 0;
        for (b, r) in bases.iter().zip(&res) {
            let mut best_o: Option<f64> = None;
            for c in [cmir(&vars, b, 0.0), lifted_flow_cover(&vars, b, 0.0), lifted_cover(&vars, b, 0.0)].into_iter().flatten() {
                let e = eff(&c.coefs, c.rhs);
                if e > 1e-6 {
                    best_o = Some(best_o.map_or(e, |v: f64| v.max(e)));
                }
            }
            let h = r.as_ref().map(|(c, rhs)| eff(c, *rhs));
            match (h, best_o) {
                (Some(eh), Some(eo)) => {
                    both += 1;
                    sum_h += eh;
                    sum_o += eo;
                    if eo < 0.9 * eh && std::env::var("HIGHS_CUT_DUMP_WEAK").is_ok() {
                        let ni = b.iter().filter(|&&(k, _)| k < n && isint[k] && !(lo[k] == 0.0 && up[k] == 1.0)).count();
                        let nb = b.iter().filter(|&&(k, _)| k < n && isint[k] && lo[k] == 0.0 && up[k] == 1.0).count();
                        let nc = b.iter().filter(|&&(k, _)| k < n && !isint[k]).count();
                        let ns = b.iter().filter(|&&(k, _)| k >= n).count();
                        println!("weak: highs {eh:.4} ours {eo:.4} base len {} (bin {nb} genint {ni} cont {nc} slack {ns})", b.len());
                        if let Some((c, r)) = r {
                            let c2: Vec<String> = c.iter().map(|&(k, a)| format!("{k}:{a:.4}{}{}", if isint[k] { "i" } else { "c" }, if lo[k] == 0.0 && up[k] == 1.0 { "b" } else { "" })).collect();
                            println!("   highs cut rhs {r:.4}: {}", c2.join(" "));
                        }
                        for (nm, cc) in [("cmir", cmir(&vars, b, 0.0)), ("lfc", lifted_flow_cover(&vars, b, 0.0))] {
                            if let Some(cc) = cc {
                                let c2: Vec<String> = cc.coefs.iter().map(|&(k, a)| format!("{k}:{a:.4}")).collect();
                                println!("   ours {nm} rhs {:.4} eff {:.4}: {}", cc.rhs, eff(&cc.coefs, cc.rhs), c2.join(" "));
                            }
                        }
                        let bb: Vec<String> = b.iter().map(|&(k, a)| format!("{k}:{a}[{},{}]x={:.3}", vlo[k], vup[k], vx[k])).collect();
                        println!("   base: {}", bb.join(" "));
                    }
                    if shown < 15 {
                        shown += 1;
                        println!("both: highs {eh:.4} ours {eo:.4} base len {}", b.len());
                    }
                }
                (Some(eh), None) => {
                    only_h += 1;
                    if shown < 15 {
                        shown += 1;
                        println!("only highs: {eh:.4} base len {}: {:?}", b.len(), &b[..b.len().min(12)]);
                    }
                }
                (None, Some(_)) => only_o += 1,
                (None, None) => none += 1,
            }
        }
        println!("bases {} both {both} only_highs {only_h} only_ours {only_o} none {none}; mean eff on both: highs {:.4} ours {:.4}", bases.len(), sum_h / both.max(1) as f64, sum_o / both.max(1) as f64);
        let mut eh: Vec<f64> = res.iter().flatten().map(|(c, r)| eff(c, *r)).collect();
        eh.sort_by(|a, b| b.total_cmp(a));
        println!(
            "HiGHS path cuts {} mean eff {:.4} max {:.4} top10 {:?} >0.1: {} >0.01: {}",
            eh.len(),
            eh.iter().sum::<f64>() / eh.len().max(1) as f64,
            eh.first().copied().unwrap_or(0.0),
            eh.iter().take(10).map(|v| (v * 1e4).round() / 1e4).collect::<Vec<_>>(),
            eh.iter().filter(|&&v| v > 0.1).count(),
            eh.iter().filter(|&&v| v > 0.01).count()
        );
    }
}

/// HiGHS の lifted cover (`HighsCutGeneration::tryGenerateCut` の前半) の移植: 置き換え後の式
/// `sum a_k y_k + sum g_k s_k <= beta` (整数変数 y の係数は正にそろえる、連続変数 s は負の係数だけ) から
/// カバーを作り (`determineCover`、LP 解を使う版)、0-1 変数だけなら lifted knapsack cover、一般整数を含めば
/// lifted mixed integer cover、連続変数を含めば lifted mixed binary cover を作る。
pub fn lifted_cover(vars: &CutVars, base: &[(usize, f64)], rhs: f64) -> Option<RawCut> {
    let (terms, beta) = substitute(vars, base, rhs)?;
    lifted_cover_terms(vars, terms, beta)
}

/// [`lifted_cover`] の本体 (置き換え済みの項と右辺から)。
fn lifted_cover_terms(vars: &CutVars, terms: Vec<Term>, beta: f64) -> Option<RawCut> {
    const FEAS: f64 = 1e-6;
    const EPS: f64 = 1e-9;
    let (mut terms, mut beta) = (terms, beta);
    // 整数変数の係数を正にそろえる (負なら補変数 y' = yu - y)。上限のない整数変数があれば使わない (HiGHS も CMIR だけ)
    for t in terms.iter_mut() {
        if !t.int {
            continue;
        }
        if !t.yu.is_finite() {
            return None;
        }
        if t.a < 0.0 {
            beta -= t.a * t.yu;
            t.a = -t.a;
            t.comp = !t.comp;
            t.yv = t.yu - t.yv;
        }
    }
    let n = terms.len();
    let isint: Vec<bool> = terms.iter().map(|t| t.int && t.a.abs() > 10.0 * FEAS).collect();
    // 係数の小さい整数変数は連続変数として扱う: 正の係数なら捨てる (緩める)
    let mut vals: Vec<f64> = terms.iter().map(|t| t.a).collect();
    for k in 0..n {
        if !isint[k] && vals[k] > 0.0 {
            vals[k] = 0.0;
        }
    }
    let upper: Vec<f64> = terms.iter().map(|t| t.yu).collect();
    let solval: Vec<f64> = terms.iter().map(|t| t.yv).collect();
    let has_cont = (0..n).any(|k| !isint[k] && vals[k] < 0.0);
    let has_gen = (0..n).any(|k| isint[k] && upper[k] != 1.0);
    // determineCover
    if beta <= 10.0 * FEAS {
        return None;
    }
    let mut cover: Vec<usize> = (0..n).filter(|&k| isint[k] && vals[k] > 0.0 && solval[k] > FEAS).collect();
    let (mut at_up, mut rest): (Vec<usize>, Vec<usize>) = cover.drain(..).partition(|&k| solval[k] >= upper[k] - FEAS);
    let mut weight: f64 = at_up.iter().map(|&k| vals[k] * upper[k]).sum();
    rest.sort_by(|&i, &j| {
        let (bi, bj) = (upper[i] < 1.5, upper[j] < 1.5);
        if bi != bj {
            return bj.cmp(&bi);
        }
        let (ca, cb) = (solval[i] * vals[i], solval[j] * vals[j]);
        if ca > cb + FEAS {
            return std::cmp::Ordering::Less;
        }
        if ca < cb - FEAS {
            return std::cmp::Ordering::Greater;
        }
        vals[j].total_cmp(&vals[i])
    });
    let minlambda = (10.0 * FEAS).max(FEAS * beta.abs());
    let mut it = rest.into_iter();
    while weight - beta <= minlambda {
        match it.next() {
            Some(k) => {
                weight += vals[k] * upper[k];
                at_up.push(k);
            }
            None => break,
        }
    }
    let mut cover = at_up;
    let lambda = weight - beta;
    if cover.is_empty() || lambda <= minlambda {
        return None;
    }
    let mut cvals = vals.clone();
    let mut crhs;
    if !has_cont && !has_gen {
        // lifted knapsack cover
        cover.sort_by(|&a, &b| vals[b].total_cmp(&vals[a]));
        let cs = cover.len();
        let mut abar = vals[cover[0]];
        let mut sigma = lambda;
        for i in 1..cs {
            let delta = abar - vals[cover[i]];
            let kdelta = i as f64 * delta;
            if kdelta < sigma {
                abar = vals[cover[i]];
                sigma -= kdelta;
            } else {
                abar -= sigma / i as f64;
                sigma = 0.0;
                break;
            }
        }
        if sigma > 0.0 {
            abar = beta / cs as f64;
        }
        let mut s = vec![0.0; cs];
        let mut sum = 0.0;
        let mut cplus = 0usize;
        let mut flag = vec![0i8; n];
        for i in 0..cs {
            sum += abar.min(vals[cover[i]]);
            s[i] = sum;
            if vals[cover[i]] > abar + FEAS {
                cplus += 1;
                flag[cover[i]] = 1;
            } else {
                flag[cover[i]] = -1;
            }
        }
        let mut halfint = false;
        let mut g = |z: f64| -> f64 {
            let hfrac = z / abar;
            let mut coef = 0.0;
            let h0 = (hfrac + 0.5).floor() as i64;
            if h0 != 0 && (hfrac - h0 as f64).abs() * abar.max(1.0) <= EPS && h0 <= cplus as i64 - 1 {
                halfint = true;
                coef = 0.5;
            }
            let mut h = (h0 - 1).max(0) as usize;
            while h < cs {
                if z <= s[h] + FEAS {
                    break;
                }
                h += 1;
            }
            coef + h as f64
        };
        crhs = cs as f64 - 1.0;
        for k in 0..n {
            if cvals[k] == 0.0 {
                continue;
            }
            cvals[k] = if flag[k] == -1 { 1.0 } else { g(cvals[k]) };
        }
        if halfint {
            crhs *= 2.0;
            for v in cvals.iter_mut() {
                *v *= 2.0;
            }
        }
    } else if has_gen {
        // lifted mixed integer cover
        cover.sort_by(|&a, &b| vals[b].total_cmp(&vals[a]));
        let cs = cover.len();
        let mut coverflag = vec![false; n];
        for &k in &cover {
            coverflag[k] = true;
        }
        let mut a: Vec<f64> = Vec::with_capacity(cs);
        let mut u = vec![0.0; cs + 1];
        let mut m = vec![0.0; cs + 1];
        let (mut usum, mut msum) = (0.0, 0.0);
        for c in 0..cs {
            let i = cover[c];
            u[c] = usum;
            m[c] = msum;
            a.push(vals[i]);
            usum += upper[i];
            msum += upper[i] * vals[i];
        }
        u[cs] = usum;
        m[cs] = msum;
        let mut lpos: Option<usize> = None;
        let mut best_cplusend = 0usize;
        let mut best_val = 0.0;
        let mut best_at_upper = true;
        for i in 0..cs {
            let j = cover[i];
            let ub = upper[j];
            let at_upper = solval[j] >= ub - FEAS;
            if at_upper && !best_at_upper {
                continue;
            }
            let mju = ub * vals[j];
            let mu = mju - lambda;
            if mu <= 10.0 * FEAS || vals[j].abs() < 1000.0 * FEAS {
                continue;
            }
            let mudival = mu / vals[j];
            if (mudival - mudival.round()).abs() <= FEAS {
                continue;
            }
            let eta = mudival.ceil();
            let thr = (ub - eta + 1.0) * vals[j];
            let cplusend = cover.partition_point(|&k| vals[k] >= thr);
            let mut mcplus = m[cplusend];
            if i < cplusend {
                mcplus -= mju;
            }
            let jl = mcplus + eta * vals[j];
            if jl > best_val || (!at_upper && best_at_upper) {
                lpos = Some(i);
                best_cplusend = cplusend;
                best_val = jl;
                best_at_upper = at_upper;
            }
        }
        let lpos = lpos?;
        let l = cover[lpos];
        let al = vals[l];
        let upperl = upper[l];
        let mlu = upperl * al;
        let mu = mlu - lambda;
        a.truncate(best_cplusend);
        let mut cov2: Vec<usize> = cover[..best_cplusend].to_vec();
        u.truncate(best_cplusend + 1);
        m.truncate(best_cplusend + 1);
        if lpos < best_cplusend {
            a.remove(lpos);
            cov2.remove(lpos);
            u.remove(lpos + 1);
            m.remove(lpos + 1);
            for i in (lpos + 1)..best_cplusend {
                u[i] -= upperl;
                m[i] -= mlu;
            }
        }
        let cplussize = a.len();
        let mudival = mu / al;
        let eta = mudival.ceil();
        let r = (mu - mudival.floor() * al).max(0.0);
        let ulme1 = upperl - eta + 1.0;
        let cthr = ulme1 * al;
        let kmin = (eta - upperl - 0.5).floor() as i64;
        let phi_l = |av: f64| -> f64 {
            let mut k = ((av / al) as i64).min(-1);
            while k >= kmin {
                let kf = k as f64;
                if av >= kf * al + r {
                    return av - (kf + 1.0) * r;
                }
                if av >= kf * al {
                    return kf * (al - r);
                }
                k -= 1;
            }
            kmin as f64 * (al - r)
        };
        let kmax = (upperl - eta + 0.5).floor() as i64;
        let gamma_l = |z: f64| -> Option<f64> {
            for i in 0..cplussize {
                let upperi = upper[cov2[i]] as i64;
                for h in 0..=upperi {
                    let hf = h as f64;
                    let mih = m[i] + hf * a[i];
                    let uih = u[i] + hf;
                    let md = mih + a[i] - cthr;
                    if z <= md {
                        return Some(uih * ulme1 * (al - r));
                    }
                    let mut k = ((z - md) / al) as i64 - 1;
                    while k <= kmax {
                        let kf = k as f64;
                        if z <= md + kf * al + r {
                            return Some((uih * ulme1 + kf) * (al - r));
                        }
                        if z <= md + (kf + 1.0) * al {
                            return Some(uih * ulme1 * (al - r) + z - mih - a[i] + cthr - (kf + 1.0) * r);
                        }
                        k += 1;
                    }
                }
            }
            let mut p = ((z - m[cplussize]) / al) as i64 - 1;
            for _ in 0..1_000_000 {
                let pf = p as f64;
                if z <= m[cplussize] + pf * al + r {
                    return Some((u[cplussize] * ulme1 + pf) * (al - r));
                }
                if z <= m[cplussize] + (pf + 1.0) * al {
                    return Some(u[cplussize] * ulme1 * (al - r) + z - m[cplussize] - (pf + 1.0) * r);
                }
                p += 1;
            }
            None
        };
        crhs = (upperl - eta) * r - lambda;
        for k in 0..n {
            if cvals[k] == 0.0 {
                continue;
            }
            if !isint[k] {
                if cvals[k] > 0.0 {
                    cvals[k] = 0.0;
                }
                continue;
            }
            if coverflag[k] {
                cvals[k] = -phi_l(-cvals[k]);
                crhs += cvals[k] * upper[k];
            } else {
                cvals[k] = gamma_l(cvals[k])?;
            }
        }
    } else {
        // lifted mixed binary cover
        cover.sort_by(|&a, &b| vals[b].total_cmp(&vals[a]));
        let cs = cover.len();
        let mut coverflag = vec![false; n];
        for &k in &cover {
            coverflag[k] = true;
        }
        let mut s = vec![0.0; cs];
        let mut sum = 0.0;
        let mut p = cs;
        for i in 0..cs {
            if vals[cover[i]] - lambda <= EPS {
                p = i;
                break;
            }
            sum += vals[cover[i]];
            s[i] = sum;
        }
        if p == 0 {
            return None;
        }
        let phi = |av: f64| -> f64 {
            for i in 0..p {
                if av <= s[i] - lambda {
                    return i as f64 * lambda;
                }
                if av <= s[i] {
                    return (i as f64 + 1.0) * lambda + (av - s[i]);
                }
            }
            p as f64 * lambda + (av - s[p - 1])
        };
        crhs = -lambda;
        for k in 0..n {
            if !isint[k] {
                if cvals[k] > 0.0 {
                    cvals[k] = 0.0;
                }
                continue;
            }
            if coverflag[k] {
                cvals[k] = cvals[k].min(lambda);
                crhs += cvals[k];
            } else {
                cvals[k] = phi(cvals[k]);
            }
        }
    }
    // y の式から元の変数に戻す
    let mut coefs: Vec<(usize, f64)> = Vec::with_capacity(n);
    let mut r = crhs;
    for (t, &c) in terms.iter().zip(&cvals) {
        if c == 0.0 || !c.is_finite() {
            continue;
        }
        r += unsubstitute(vars, t, c, &mut coefs);
    }
    if coefs.is_empty() || !r.is_finite() {
        return None;
    }
    Some(RawCut { coefs, rhs: r })
}

#[cfg(test)]
mod lifted_cover_tests {
    use super::*;

    /// 小さな乱数の行 `sum a_j x_j + g s <= b` (x は 0-1 / 一般整数、s >= 0 は連続) に対し、lifted cover が
    /// 全ての整数点 (s は最小の実行可能な値) を満たすことを確かめる。
    #[test]
    fn lifted_cover_is_valid() {
        let mut seed: u64 = 12345;
        let mut rnd = |m: u64| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) % m
        };
        let mut found = 0;
        for _ in 0..4000 {
            let nint = 2 + rnd(4) as usize;
            let cont = rnd(3) == 0;
            let n = nint + cont as usize;
            let mut lo = vec![0.0; n];
            let mut up = vec![1.0; n];
            let mut is_int = vec![true; n];
            let mut base: Vec<(usize, f64)> = Vec::new();
            for j in 0..nint {
                up[j] = if rnd(3) == 0 { (1 + rnd(4)) as f64 } else { 1.0 };
                let a = (1 + rnd(20)) as f64 * if rnd(5) == 0 { -1.0 } else { 1.0 };
                base.push((j, a));
                lo[j] = 0.0;
            }
            if cont {
                is_int[nint] = false;
                up[nint] = f64::INFINITY;
                base.push((nint, -((1 + rnd(5)) as f64)));
            }
            let b = (1 + rnd(30)) as f64;
            // LP 点: 乱数 (実行可能でなくてもよい。カットの妥当性だけを調べる)
            let x: Vec<f64> = (0..n).map(|j| if is_int[j] { rnd(100) as f64 / 100.0 * up[j] } else { rnd(100) as f64 / 50.0 }).collect();
            let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &x, vb: None };
            let Some(cut) = lifted_cover(&vars, &base, b) else { continue };
            found += 1;
            // 整数点の列挙
            let mut pt = vec![0i64; nint];
            loop {
                let lhs: f64 = (0..nint).map(|j| base[j].1 * pt[j] as f64).sum();
                let mut xs: Vec<f64> = pt.iter().map(|&v| v as f64).collect();
                let feasible = if cont {
                    let g = base[nint].1;
                    let s = ((lhs - b) / -g).max(0.0);
                    xs.push(s);
                    true
                } else {
                    lhs <= b + 1e-9
                };
                if feasible {
                    let act: f64 = cut.coefs.iter().map(|&(k, c)| c * xs[k]).sum();
                    assert!(act <= cut.rhs + 1e-6, "invalid lifted cover: base {base:?} <= {b}, ub {up:?}, cut {:?} <= {}, point {xs:?} act {act}", cut.coefs, cut.rhs);
                }
                let mut j = 0;
                while j < nint {
                    pt[j] += 1;
                    if pt[j] as f64 <= up[j] {
                        break;
                    }
                    pt[j] = 0;
                    j += 1;
                }
                if j == nint {
                    break;
                }
            }
        }
        assert!(found > 100, "only {found} cuts");
    }
}


/// HiGHS の `HighsCutGeneration::generateCut` と同じ流れのカット生成の入口: 集約行 `sum a_k v_k <= rhs` を
/// 1 回だけ境界の置き換え・補変数化し、行の種類 (0-1 だけ・一般整数・連続変数の有無) に合ったカットを作る:
/// lifted cover (0-1 だけなら knapsack、一般整数なら mixed integer、連続を含めば mixed binary)、CMIR、
/// (`ext_cover` なら) extended cover、(連続変数を含み `flow_cover` なら) lifted flow cover。
/// 候補を全て返す (どれが一番強いかは呼び出し側が構造変数の式に直してから効き目で比べ、1 本だけ残す)。
/// lifted cover は `ENOMOTO_MIP_LIFTED_COVER` のときだけ試す (40 問で入れると 18 -> 16 問 (sgeomean 28.34 -> 29.15)
/// と悪化した: neos-860300・neos5・neos-911970 が遅くなる)。
pub fn generate_cuts(vars: &CutVars, base: &[(usize, f64)], rhs: f64, flow_cover: bool, ext_cover: bool) -> Vec<RawCut> {
    let mut out = Vec::new();
    let Some((terms, beta)) = substitute(vars, base, rhs) else {
        return out;
    };
    let has_cont = base.iter().any(|&(k, a)| a != 0.0 && !vars.is_int[k] && vars.lo[k] < vars.up[k]);
    // `ENOMOTO_MIP_ROW_LIFTED_COVER`: 元の行 1 本ずつ (`ext_cover` の呼び出し) でだけ lifted cover も試す
    // (集約行・tableau 行では試さない。どれを残すかは呼び出し側がカットの質で決める)
    // 40 問: 使わない 20 問 28.04、使う 19 問 27.66 (mik-250 53.3 -> 20.7 秒、binkar10_1 28.1 -> 24.5 秒、rout 31.9 秒 -> 時間切れ)。
    // 差はばらつきの範囲なので既定では使わない
    if env_str!("ENOMOTO_MIP_LIFTED_COVER").is_some() || (ext_cover && env_str!("ENOMOTO_MIP_ROW_LIFTED_COVER").is_some()) {
        out.extend(lifted_cover_terms(vars, terms.clone(), beta));
    }
    // δ の探索 (`cmir_terms_multi`) は `ENOMOTO_MIP_CMIR_SEARCH` のときだけ。40 問で、従来 18 問 (sgeomean 28.12) に対し
    // 探索 + 上位 3 本は 16 問 (29.95)、割る数 1..8 だけ (局所探索なし・1 本) は 18 問 (28.86) と悪化した
    // (分離が遅くなり、カットが密になり、根の下界も下がった)。
    if env_str!("ENOMOTO_MIP_CMIR_SEARCH").is_none() {
        out.extend(cmir_terms(vars, terms, beta));
    } else {
        out.extend(cmir_terms_multi(vars, terms, beta, tunable!("ENOMOTO_T_CMIR_K", 3usize, usize)));
    }
    if ext_cover && !has_cont {
        out.extend(extended_cover(vars, base, rhs));
    }
    if has_cont && flow_cover {
        out.extend(lifted_flow_cover(vars, base, rhs));
    }
    out
}

/// δ を探索して効き目の上位 `k` 本の CMIR を返す (`generate_cuts` から。どれを残すかは呼び出し側がカットの質で決める)。
///
/// δ を変数とみた 1 次元の探索: 初期点は LP 値が境界の内側にある整数変数の係数 a を 1..=`ENOMOTO_T_CMIR_MAXDIV` (既定 8)
/// で割ったもの (従来は 1, 2, 4, 8 だけ)。効き目 (置き換え後の空間の違反量 / ノルム) の上位から、log δ 上の
/// パターン探索 (歩幅 0.1 から、改善しなければ半分、`ENOMOTO_T_CMIR_LOCAL_ITERS` 回まで) で局所的に改善する。
/// 床関数で効き目は区分的だが、区分の中では δ について連続なので局所探索が効く。
/// 上位 `k` 個の δ (互いに 1e-6 以上離れたもの) それぞれで補変数化の向きの反転を試してカットを作る。
fn cmir_terms_multi(vars: &CutVars, terms: Vec<Term>, beta: f64, k: usize) -> Vec<RawCut> {
    let mut out = Vec::new();
    if !terms.iter().any(|t| t.int) {
        return out;
    }
    let maxdiv = tunable!("ENOMOTO_T_CMIR_MAXDIV", 8usize, usize).max(1);
    let iters = tunable!("ENOMOTO_T_CMIR_LOCAL_ITERS", 12usize, usize);
    let mut base: Vec<f64> = Vec::new();
    for t in &terms {
        if t.int && t.yv > 1e-6 && t.yv < t.yu - 1e-6 && t.a.abs() > 1e-6 {
            let d = t.a.abs();
            if !base.iter().any(|&e| (e - d).abs() <= 1e-9 * d) {
                base.push(d);
            }
        }
        if base.len() >= 8 {
            break;
        }
    }
    if base.is_empty() {
        let m = terms.iter().filter(|t| t.int).map(|t| t.a.abs()).fold(0.0, f64::max);
        if m > 0.0 {
            base.push(m);
        }
    }
    let mut cands: Vec<(f64, f64)> = Vec::new(); // (delta, efficacy)
    for &d in &base {
        for div in 1..=maxdiv {
            let delta = d / div as f64;
            if let Some(e) = mir_efficacy(&terms, beta, delta) {
                cands.push((delta, e));
            }
        }
    }
    cands.sort_by(|a, b| b.1.total_cmp(&a.1));
    // 局所探索 (上位 2k 個の初期点から)
    let mut found: Vec<(f64, f64)> = Vec::new();
    for &(d0, e0) in cands.iter().take(2 * k.max(1)) {
        let (mut ld, mut e) = (d0.ln(), e0);
        let mut step = 0.1;
        for _ in 0..iters {
            let mut moved = false;
            for s in [step, -step] {
                if let Some(e2) = mir_efficacy(&terms, beta, (ld + s).exp()) {
                    if e2 > e + 1e-12 {
                        ld += s;
                        e = e2;
                        moved = true;
                        break;
                    }
                }
            }
            if !moved {
                step *= 0.5;
            }
        }
        found.push((ld.exp(), e));
        found.push((d0, e0));
    }
    found.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut chosen: Vec<(f64, f64)> = Vec::new();
    for (d, e) in found {
        if chosen.len() >= k {
            break;
        }
        if chosen.iter().all(|&(c, _)| (c - d).abs() > 1e-6 * c) {
            chosen.push((d, e));
        }
    }
    for (d, e) in chosen {
        out.extend(cmir_build(vars, terms.clone(), beta, d, e));
    }
    out
}
