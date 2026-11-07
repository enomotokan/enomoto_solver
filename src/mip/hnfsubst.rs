//! 試作 `ENOMOTO_MIP_HNFSUBST` (既定では使わない): [`super::hnf`] の列ごとの合同式 `x_j ≡ r_j (mod g_j)` (g_j > 1) の出た一般整数列を
//! `x_j = r_j + g_j y_j` (y_j は整数) に置き換えた問題を作る。
//!
//! 列の番号は保つ (列 j の意味が x_j から y_j に変わる)。後処理は自己参照の代入 `x_j = r_j - (-g_j) y_j` ([`PostsolveStep::Sub`]、
//! `terms = [(j, -g_j)]`) を前処理の手順の最後に足すだけで、逆順に戻す既存の後処理・再スタートのカットの代入による写し
//! ([`super::map_restart_cut_subst`]) がそのまま使える。r_j は下限 (有限なら) 以上で合同式に合う最小の値にとるので、y の下限は 0 になる。
//!
//! [`PostsolveStep::Sub`]: crate::presolve::PostsolveStep::Sub

use super::MipProblem;

/// `ENOMOTO_MIP_HNFSUBST` が立っているか。
pub(crate) fn on() -> bool {
    !matches!(env_str!("ENOMOTO_MIP_HNFSUBST"), None | Some("") | Some("0"))
}

/// 置き換える法の上限 (`ENOMOTO_T_MIP_HNFSUBST_MAXMOD`、既定 1000)。大きな法で係数を掛けると LP の数値が荒れる (rocI-4-11 は 1e6)。
fn max_mod() -> i64 {
    tunable!("ENOMOTO_T_MIP_HNFSUBST_MAXMOD", 1000.0, f64) as i64
}

/// 置き換え 1 本: 列 j を `x_j = r + g y_j` にした。
#[derive(Clone, Copy, Debug)]
pub(crate) struct Subst {
    pub j: usize,
    pub r: f64,
    pub g: f64,
}

/// 元の空間の値 `x_j` から置き換えた空間の値 `y_j = (x_j - r) / g` を求める (暫定解の写し用)。
pub(crate) fn forward(map: &[Subst], x: &mut [f64]) {
    for s in map {
        if s.j < x.len() {
            x[s.j] = (x[s.j] - s.r) / s.g;
        }
    }
}

/// `x_j = r + g y_j` の後処理の手順。
pub(crate) fn postsolve_step(s: &Subst) -> crate::presolve::PostsolveStep {
    crate::presolve::PostsolveStep::Sub(crate::presolve::colsingleton::Substitution { var: s.j, terms: vec![(s.j, -s.g)], rhs: s.r, coeff: 1.0 })
}

/// 2^52 未満なら f64 で整数を正確に表せる。
const EXACT: f64 = 4503599627370496.0;

/// 合同式の表 `info` ((列, r, g)、g = 0 は固定) から、置き換える列を選んで `prob` を書き換え、置き換えの一覧を返す。
/// 対象: 整数列 (`int_ok`、尺度の掛かっていない列)、固定されていない、0-1 列でない、1 < g <= 上限、値が 2^52 未満で正確に扱える。
pub(crate) fn substitute(prob: &mut MipProblem, info: &[(usize, i64, i64)], int_ok: &[bool]) -> Vec<Subst> {
    let gmax = max_mod();
    let mut map: Vec<Subst> = Vec::new();
    let mut is_sub = vec![false; prob.n];
    for &(j, r, g) in info {
        if g <= 1 || g > gmax || j >= prob.n || is_sub[j] || !prob.is_int[j] || !int_ok.get(j).copied().unwrap_or(false) {
            continue;
        }
        let (lo, up) = (prob.col_lo[j], prob.col_up[j]);
        if lo >= up || (lo >= 0.0 && up <= 1.0) {
            continue;
        }
        // r0: 下限が有限なら下限以上の最小の代表、上限だけ有限なら上限以下の最大の代表、両方無限なら [0, g) の代表
        let gi = g as i128;
        let r0: i128 = if lo.is_finite() {
            if lo.abs() >= EXACT {
                continue;
            }
            let l = lo.ceil() as i128;
            l + (r as i128 - l).rem_euclid(gi)
        } else if up.is_finite() {
            if up.abs() >= EXACT {
                continue;
            }
            let u = up.floor() as i128;
            u - (u - r as i128).rem_euclid(gi)
        } else {
            (r as i128).rem_euclid(gi)
        };
        if (r0.abs() as f64) >= EXACT {
            continue;
        }
        let (rf, gf) = (r0 as f64, g as f64);
        let ylo = if lo.is_finite() { ((lo - rf) / gf - 1e-9).ceil() } else { f64::NEG_INFINITY };
        let yup = if up.is_finite() { ((up - rf) / gf + 1e-9).floor() } else { f64::INFINITY };
        if ylo > yup {
            // 合同式に合う値がない (apply_bounds が先に実行不能を出すはず)。置き換えない
            continue;
        }
        // 係数・右辺が大きくなりすぎるなら置き換えない
        if prob.cols[j].iter().any(|&(_, a)| (a * gf).abs() > 1e9) {
            continue;
        }
        is_sub[j] = true;
        prob.col_lo[j] = ylo;
        prob.col_up[j] = yup;
        prob.offset += prob.cost[j] * rf;
        prob.cost[j] *= gf;
        for &(i, a) in &prob.cols[j] {
            let sh = a * rf;
            if prob.row_lo[i].is_finite() {
                prob.row_lo[i] -= sh;
            }
            if prob.row_up[i].is_finite() {
                prob.row_up[i] -= sh;
            }
        }
        for c in prob.cols[j].iter_mut() {
            c.1 *= gf;
        }
        map.push(Subst { j, r: rf, g: gf });
    }
    if map.is_empty() {
        return map;
    }
    let sub: std::collections::HashMap<usize, (f64, f64)> = map.iter().map(|s| (s.j, (s.r, s.g))).collect();
    for row in prob.rows.iter_mut() {
        for t in row.iter_mut() {
            if let Some(&(_, g)) = sub.get(&t.0) {
                t.1 *= g;
            }
        }
    }
    // 置き換えた列を含む行で、列がすべて整数列 (尺度なし) で係数が整数なら最大公約数で割り、右辺を丸める
    // (例: 2 x1 + 3 x2 = 12 は 6 y1 + 6 y2 = 6 -> y1 + y2 = 1)。`ENOMOTO_MIP_HNFSUBST=raw` ならしない
    if env_str!("ENOMOTO_MIP_HNFSUBST") != Some("raw") {
        let mut touched = vec![false; prob.m];
        for s in &map {
            for &(i, _) in &prob.cols[s.j] {
                touched[i] = true;
            }
        }
        let mut changed = false;
        for i in (0..prob.m).filter(|&i| touched[i]) {
            changed |= normalize_row(&mut prob.rows[i], &mut prob.row_lo[i], &mut prob.row_up[i], &prob.is_int, int_ok);
        }
        if changed {
            for c in prob.cols.iter_mut() {
                c.clear();
            }
            for (i, r) in prob.rows.iter().enumerate() {
                for &(j, a) in r {
                    prob.cols[j].push((i, a));
                }
            }
        }
    }
    // 含意 `x_j = v ⇒ x_k <= b` / `>= b` の k が置き換えた列なら y の値に直す
    for ib in prob.implied_bounds.iter_mut() {
        if let Some(&(r, g)) = sub.get(&(ib.2 as usize)) {
            let v = (ib.4 - r) / g;
            ib.4 = if ib.3 { (v + 1e-9).floor() } else { (v - 1e-9).ceil() };
        }
    }
    map
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// 整数列 (尺度なし) だけで係数が整数の行を最大公約数で割り、右辺を整数に丸める (下限は切り上げ、上限は切り下げ)。
/// 等式で右辺が割り切れない・丸めると下限 > 上限になるときは変えない (実行不能は解く側に任せる)。変えたら `true`
fn normalize_row(row: &mut [(usize, f64)], lo: &mut f64, up: &mut f64, is_int: &[bool], int_ok: &[bool]) -> bool {
    if row.is_empty() || !row.iter().all(|&(j, a)| is_int[j] && int_ok.get(j).copied().unwrap_or(false) && a.abs() < 1e9 && (a - a.round()).abs() <= 1e-9) {
        return false;
    }
    let g = row.iter().fold(0u64, |g, &(_, a)| gcd(g, a.round().abs() as u64));
    if g <= 1 {
        return false;
    }
    let g = g as f64;
    let rt = 1e-9;
    let nl = if lo.is_finite() { (*lo / g - rt).ceil() } else { *lo };
    let nu = if up.is_finite() { (*up / g + rt).floor() } else { *up };
    if nl > nu {
        return false;
    }
    if *lo == *up && ((*lo / g) - nl).abs() > 1e-9 {
        return false;
    }
    for e in row.iter_mut() {
        e.1 = (e.1 / g).round();
    }
    *lo = nl;
    *up = nu;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2 x1 + 3 x2 = 12 (x1 ∈ [0,10], x2 ∈ [0,10]): x1 ≡ 0 (mod 3)、x2 ≡ 0 (mod 2)。置き換えて後処理で戻すと元の解になる
    #[test]
    fn subst_roundtrip() {
        let mut p = MipProblem::from_rows(vec![1.0, 0.0], vec![10.0, 10.0], vec![1.0, -1.0], 0.5, 1.0, vec![true, true], vec![vec![(0, 2.0), (1, 3.0)]], vec![12.0], vec![12.0]);
        let q0 = p.clone();
        let map = substitute(&mut p, &[(0, 0, 3), (1, 0, 2)], &[true, true]);
        assert_eq!(map.len(), 2);
        // x1: 下限 1 以上の最小の 3 の倍数 = 3、y1 ∈ [0, 2]; x2: r = 0, y2 ∈ [0, 5]
        assert_eq!((map[0].r, map[0].g), (3.0, 3.0));
        assert_eq!((p.col_lo[0], p.col_up[0]), (0.0, 2.0));
        assert_eq!((p.col_lo[1], p.col_up[1]), (0.0, 5.0));
        // 6 y1 + 6 y2 = 6 を 6 で割って y1 + y2 = 1
        assert_eq!(p.rows[0], vec![(0, 1.0), (1, 1.0)]);
        assert_eq!((p.row_lo[0], p.row_up[0]), (1.0, 1.0));
        assert_eq!(p.cols[0], vec![(0, 1.0)]);
        // y = (1, 1) -> x = (6, 0) は 2*6 = 12 を満たす
        let y = vec![1.0, 0.0];
        assert!(p.is_feasible(&y, 1e-9));
        let mut x = y.clone();
        for s in map.iter().rev() {
            postsolve_step(s).apply(&mut x);
        }
        assert_eq!(x, vec![6.0, 0.0]);
        assert!(q0.is_feasible(&x, 1e-9));
        assert!((q0.objective(&x) - p.objective(&y)).abs() < 1e-12);
        let mut xf = x.clone();
        forward(&map, &mut xf);
        assert_eq!(xf, y);
    }
}
