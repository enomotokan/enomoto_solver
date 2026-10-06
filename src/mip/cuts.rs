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
}

/// 境界の代入 (補変数化) をした項と右辺。連続変数の正の係数の項は捨ててある。
fn substitute(vars: &CutVars, base: &[(usize, f64)], rhs: f64) -> Option<(Vec<Term>, f64)> {
    let mut terms: Vec<Term> = Vec::with_capacity(base.len());
    let mut beta = rhs;
    for &(k, a) in base {
        if a.abs() < 1e-12 {
            continue;
        }
        let (l, u, x) = (vars.lo[k], vars.up[k], vars.x[k]);
        if l == u {
            beta -= a * l;
            continue;
        }
        let int = vars.is_int[k];
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
            let yv = (x - l).max(0.0);
            if !int && a > 0.0 {
                continue; // 正の係数の連続変数は捨てる
            }
            terms.push(Term { k, a, comp: false, int, yv, yu: u - l });
        } else {
            // v = u - y
            beta -= a * u;
            let yv = (u - x).max(0.0);
            if !int && -a > 0.0 {
                continue;
            }
            terms.push(Term { k, a: -a, comp: true, int, yv, yu: u - l });
        }
    }
    Some((terms, beta))
}

/// CMIR で切除平面を作る。作れなければ `None`。
pub fn cmir(vars: &CutVars, base: &[(usize, f64)], rhs: f64) -> Option<RawCut> {
    let (terms, beta) = substitute(vars, base, rhs)?;
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
    let (delta, mut eff) = best?;
    // 4. 補変数化の向きを 1 つずつ反転して改善するか試す (両側有限の整数変数のみ)
    let mut terms = terms;
    let mut beta = beta;
    for idx in 0..terms.len() {
        let t = terms[idx];
        if !t.int || !t.yu.is_finite() || t.yv <= 1e-6 {
            continue;
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
        let (l, u) = (vars.lo[t.k], vars.up[t.k]);
        if t.comp {
            // y = u - v
            r -= c * u;
            coefs.push((t.k, -c));
        } else {
            // y = v - l
            r += c * l;
            coefs.push((t.k, c));
        }
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
        let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &x };
        let cut = extended_cover(&vars, &[(0, 3.0), (1, 4.0), (2, 5.0)], 6.0).expect("cover");
        let act: f64 = cut.coefs.iter().map(|&(k, c)| c * x[k]).sum();
        assert!(act > cut.rhs + 1e-6);
        for pt in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, 0.0]] {
            let a: f64 = cut.coefs.iter().map(|&(k, c)| c * pt[k]).sum();
            assert!(a <= cut.rhs + 1e-9);
        }
    }

    #[test]
    fn mir_on_simple_row() {
        // x integer in [0, 10], y continuous >= 0: x - y <= 2.5, LP point (2.5, 0) → cut x - 2y <= 2
        let lo = [0.0, 0.0];
        let up = [10.0, f64::INFINITY];
        let is_int = [true, false];
        let x = [2.5, 0.0];
        let vars = CutVars { lo: &lo, up: &up, is_int: &is_int, x: &x };
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
        let (l, u) = (vars.lo[t.k], vars.up[t.k]);
        if t.comp {
            // y = u - v
            r -= cy * u;
            coefs.push((t.k, -cy));
        } else {
            r += cy * l;
            coefs.push((t.k, cy));
        }
    }
    Some(RawCut { coefs, rhs: r })
}
