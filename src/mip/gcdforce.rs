//! 整数列だけの行の最大公約数による右辺の丸めと、上下限から決まる活動値の範囲を合わせた判定 (analysis/gcdforce_20261107.md)。
//!
//! - 診断 `ENOMOTO_MIP_DEBUG_GCDCOUNT` (問題は変えない): 前処理の丸めの直前と、前処理の最後の問題で、整数列だけ・係数が整数の
//!   不等式のうち、丸めた右辺と活動値の範囲 `[mn, mx]` を合わせると (a) 値が 1 つに決まる・(b) 冗長・(c) 実行不能・(d) 強制
//!   (値が `mn` か `mx` に決まり、列が全部固定できる) になる行を、丸める前・丸めた後に分けて数える。
//! - 試作 `ENOMOTO_MIP_GCDFORCE` (既定では使わない): 前処理の丸め ([`super::normalize_integer_rows`]) の直後に同じ判定をし、
//!   値が 1 つに決まる行は等式に、冗長な行は消し、実行不能なら実行不能を返す。変わった行があれば前処理をかけ直す。
//!
//! 行の活動値は `c0 + Σ a_j x_j` (`c0` は固定された列の寄与) で、固定されていない列がすべて整数列、係数が整数 (|a| < 1e9) のとき、
//! 可変部は `g` (係数の最大公約数) の倍数。整数列の上下限は整数に丸めて (`ceil(lo - 1e-6)`, `floor(up + 1e-6)`) 活動値の範囲を数えるので、
//! `mn`・`mx` も `g` の倍数 (有限のとき)。

/// 判定の分類: 0 値が 1 つに決まる (強制でない)、1 冗長、2 実行不能、3 強制 (値が活動値の端で 1 つに決まる)
pub(crate) const NCLS: usize = 4;

/// 行の型 (固定されていない列がすべて整数列で係数が整数): `(g, c0, mn, mx)`。範囲は可変部 (`c0` を除く) の値。
/// 当てはまらなければ `None`。活動値の範囲が大きすぎる (|値| > 1e15) 行も `None` (f64 の丸めで倍数の判定が狂う)
pub(crate) fn int_row(row: &[(usize, f64)], col_lo: &[f64], col_up: &[f64], is_int: &[bool]) -> Option<(f64, f64, f64, f64)> {
    fn gcd(a: u64, b: u64) -> u64 {
        if b == 0 { a } else { gcd(b, a % b) }
    }
    let (mut g, mut c0, mut mn, mut mx) = (0u64, 0.0f64, 0.0f64, 0.0f64);
    let mut nfree = 0;
    for &(j, a) in row {
        if a == 0.0 {
            continue;
        }
        if col_lo[j] == col_up[j] {
            c0 += a * col_lo[j];
            continue;
        }
        if !is_int[j] || (a - a.round()).abs() > 1e-9 || a.abs() >= 1e9 {
            return None;
        }
        nfree += 1;
        let ar = a.round();
        g = gcd(g, ar.abs() as u64);
        // 上下限の整数への丸めはソルバーの整数の許容誤差 (1e-6) の側にする (範囲を広げる側で、判定は保守的になる)。1e-9 で丸めると、
        // 前処理が `z <= 0.999999` と残した 0-1 列を z = 0 と見て `x + y + 1e6 z <= 999959` を冗長として消し、許容誤差の内で z = 1 の
        // 解が元の問題の行を破った (supportcase33、GCDFORCE の 7 回中 1 回で解が後処理で捨てられた)
        let (lo, up) = ((col_lo[j] - 1e-6).ceil(), (col_up[j] + 1e-6).floor());
        if lo > up {
            // 整数の値を持たない列: 前処理の別の手順が実行不能として扱う
            return None;
        }
        let (a_lo, a_up) = if ar > 0.0 { (ar * lo, ar * up) } else { (ar * up, ar * lo) };
        mn += if a_lo.is_nan() { f64::NEG_INFINITY } else { a_lo };
        mx += if a_up.is_nan() { f64::INFINITY } else { a_up };
    }
    if nfree == 0 || g == 0 || !c0.is_finite() {
        return None;
    }
    if (mn.is_finite() && mn.abs() > 1e15) || (mx.is_finite() && mx.abs() > 1e15) {
        return None;
    }
    Some((g as f64, c0, mn, mx))
}

/// 可変部の値の幅 `[l, u]` (`-inf`/`+inf` 可) と活動値の範囲 `[mn, mx]` の判定。どれでもなければ `None`
pub(crate) fn classify(l: f64, u: f64, mn: f64, mx: f64) -> Option<usize> {
    let tol = 1e-6;
    let lo = l.max(mn);
    let hi = u.min(mx);
    if lo > hi + tol {
        return Some(2);
    }
    if l <= mn + tol && u >= mx - tol {
        return Some(1);
    }
    if hi - lo <= tol {
        return Some(if (lo - mn).abs() <= tol || (hi - mx).abs() <= tol { 3 } else { 0 });
    }
    None
}

/// 幅 `[l, u]` を `g` の倍数に丸める (`normalize_integer_rows` と同じ余裕 1e-9)
pub(crate) fn round_side(l: f64, u: f64, g: f64) -> (f64, f64) {
    let nl = if l.is_finite() { g * (l / g - 1e-9).ceil() } else { l };
    let nu = if u.is_finite() { g * (u / g + 1e-9).floor() } else { u };
    (nl, nu)
}

#[derive(Default, Debug, Clone)]
pub(crate) struct GcdCounts {
    /// 整数列だけ・係数が整数の不等式 (等式でない行) の数
    pub ineq: usize,
    /// そのうち g > 1 の行
    pub g_gt1: usize,
    /// 丸める前の判定
    pub raw: [usize; NCLS],
    /// 丸めた後の判定
    pub rnd: [usize; NCLS],
    /// 丸める前は何もなく、丸めた後に判定が出た行 (= 丸めで初めて見える見落とし)
    pub only_rnd: [usize; NCLS],
    /// 等式で、右辺が g の倍数でない (実行不能) 行
    pub eq_infeas: usize,
}

impl GcdCounts {
    pub fn line(&self) -> String {
        format!(
            "ineq {} g>1 {} raw uniq/red/infeas/force {:?} rounded {:?} only_rounded {:?} eq_infeas {}",
            self.ineq, self.g_gt1, self.raw, self.rnd, self.only_rnd, self.eq_infeas
        )
    }
}

/// 行の並び (範囲つきの行は 1 本、`row_lo`/`row_up`) を数える (問題は変えない)
pub(crate) fn count(rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64], col_lo: &[f64], col_up: &[f64], is_int: &[bool]) -> GcdCounts {
    let mut c = GcdCounts::default();
    for i in 0..rows.len() {
        let Some((g, c0, mn, mx)) = int_row(&rows[i], col_lo, col_up, is_int) else { continue };
        let (l, u) = (row_lo[i] - c0, row_up[i] - c0);
        if row_lo[i] == row_up[i] {
            let (nl, nu) = round_side(l, u, g);
            if nl > nu + 1e-6 {
                c.eq_infeas += 1;
            }
            continue;
        }
        c.ineq += 1;
        if g > 1.0 {
            c.g_gt1 += 1;
        }
        let r0 = classify(l, u, mn, mx);
        let (nl, nu) = round_side(l, u, g);
        let r1 = classify(nl, nu, mn, mx);
        if let Some(k) = r0 {
            c.raw[k] += 1;
        }
        if let Some(k) = r1 {
            c.rnd[k] += 1;
            if r0.is_none() {
                c.only_rnd[k] += 1;
            }
        }
    }
    c
}

/// 試作 `ENOMOTO_MIP_GCDFORCE`: 丸めた幅と活動値の範囲を合わせ、値が 1 つに決まる行を等式にし、冗長な行を消す。
/// 実行不能なら `None`、そうでなければ `(等式にした行の数, 消した行の数)`。行の右辺は丸めた値 (`c0` を戻す) にする
pub(crate) fn apply(rows: &mut Vec<Vec<(usize, f64)>>, row_lo: &mut Vec<f64>, row_up: &mut Vec<f64>, col_lo: &[f64], col_up: &[f64], is_int: &[bool]) -> Option<(usize, usize)> {
    let (mut neq, mut keep) = (0, vec![true; rows.len()]);
    for i in 0..rows.len() {
        if row_lo[i] == row_up[i] {
            continue;
        }
        let Some((g, c0, mn, mx)) = int_row(&rows[i], col_lo, col_up, is_int) else { continue };
        let (nl, nu) = round_side(row_lo[i] - c0, row_up[i] - c0, g);
        match classify(nl, nu, mn, mx) {
            Some(2) => return None,
            Some(1) => {
                keep[i] = false;
                if env_str!("ENOMOTO_MIP_GCDFORCE_DEBUG").is_some() {
                    let sa: f64 = rows[i].iter().map(|e| e.1.abs()).sum();
                    eprintln!("GCDFORCE_DEBUG: redundant row {i} len {} g {g} c0 {c0} [{nl}, {nu}] act [{mn}, {mx}] sum|a| {sa}", rows[i].len());
                }
            }
            Some(_) => {
                // 値が 1 つ: 丸めた幅と活動値の範囲の共通部分の点 (g の倍数)
                let v = nl.max(mn);
                row_lo[i] = v + c0;
                row_up[i] = v + c0;
                neq += 1;
            }
            None => {}
        }
    }
    let nred = keep.iter().filter(|&&k| !k).count();
    if nred > 0 {
        let mut w = 0;
        for i in 0..rows.len() {
            if keep[i] {
                rows.swap(w, i);
                row_lo[w] = row_lo[i];
                row_up[w] = row_up[i];
                w += 1;
            }
        }
        rows.truncate(w);
        row_lo.truncate(w);
        row_up.truncate(w);
    }
    Some((neq, nred))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcd_rounding_meets_min_activity() {
        // x, y >= 1 の整数、2x + 2y <= 5 -> x + y <= 2 で x = y = 1
        let mut rows = vec![vec![(0, 2.0), (1, 2.0)]];
        let (mut lo, mut up) = (vec![f64::NEG_INFINITY], vec![5.0]);
        let (cl, cu, ii) = (vec![1.0, 1.0], vec![10.0, 10.0], vec![true, true]);
        let c = count(&rows, &lo, &up, &cl, &cu, &ii);
        assert_eq!(c.raw, [0; 4]);
        assert_eq!(c.only_rnd, [0, 0, 0, 1]);
        assert_eq!(apply(&mut rows, &mut lo, &mut up, &cl, &cu, &ii), Some((1, 0)));
        assert_eq!((lo[0], up[0]), (4.0, 4.0));
        // 2x + 2y >= 3、x, y <= 1 -> x + y >= 2 で強制。2x + 2y <= 1 で x, y >= 1 なら実行不能
        let (cl, cu) = (vec![0.0, 0.0], vec![1.0, 1.0]);
        let c = count(&[vec![(0, 2.0), (1, 2.0)]], &[3.0], &[f64::INFINITY], &cl, &cu, &ii);
        assert_eq!(c.only_rnd, [0, 0, 0, 1]);
        let mut rows = vec![vec![(0, 2.0), (1, 2.0)]];
        let (mut lo, mut up) = (vec![f64::NEG_INFINITY], vec![3.0]);
        assert_eq!(apply(&mut rows, &mut lo, &mut up, &[1.0, 1.0], &[5.0, 5.0], &ii), None);
        // 3x - 3y in [1, 2] (x, y in [0, 5]): 丸めると幅に 3 の倍数がない (実行不能)
        let c = count(&[vec![(0, 3.0), (1, -3.0)]], &[1.0], &[2.0], &[0.0, 0.0], &[5.0, 5.0], &ii);
        assert_eq!(c.only_rnd, [0, 0, 1, 0]);
        // 冗長: 2x + 2y <= 5 で x, y in [0, 1] (最大 4) は丸める前から冗長
        let c = count(&[vec![(0, 2.0), (1, 2.0)]], &[f64::NEG_INFINITY], &[5.0], &[0.0, 0.0], &[1.0, 1.0], &ii);
        assert_eq!((c.raw, c.only_rnd), ([0, 1, 0, 0], [0; 4]));
        // 丸めで冗長: 2x + 2y >= -1 (x, y >= 0) は丸めると >= 0 で冗長
        let c = count(&[vec![(0, 2.0), (1, 2.0)]], &[-1.0], &[f64::INFINITY], &[0.0, 0.0], &[1.0, 1.0], &ii);
        assert_eq!(c.raw, [0, 1, 0, 0]);
    }
}
