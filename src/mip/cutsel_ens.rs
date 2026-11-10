//! 試作 `ENOMOTO_MIP_CUTSEL_ENS` (既定では使わない、analysis/cutsel_ens_20261110.md): Turner・Berthold・Besançon
//! "A Context-Aware Cutting Plane Selection Algorithm for Mixed-Integer Programming" (2023) の選び方
//! (SCIP の `cutsel_ensemble.c`、master のソースの既定の重みを写した) で根のカットを選ぶ。
//!
//! 点数 (カット `a x <= b` ごと) = 0.75 × (log1p(効き目) / log1p(最大の効き目))^2
//! + 0.1 × (log1p(効き目 × 目的との平行度) / log1p(その最大))^2 (期待される改善)
//! + 0.25 × (1 − 目的との平行度) (SCIP の既定は平行度を罰する)
//! + 0.45 × 整数台 (非零のうち整数列の割合)
//! + 密度の点 max(0, 0.5 − (0.5 / 0.2) × 非零の数 / LP の列数)
//! + 0.25 × (1 − ロックの数の平均 / 最大) (SCIP の既定はロックの少ないものを良しとする)
//! (+ 木では 0.75 × 擬似費用の点。ここでは根だけで使うので入れない。directed cutoff distance・数値の点は SCIP の既定で重み 0)。
//!
//! 選び方: 密度 (非零の数 / LP の列数) が 0.425 を超えるものを先に捨て、点の高い順に貪欲に採る。採ったカットとの平行度が
//! 0.95 を超える残りは点から 0.25 を引く (ほぼ 1 なら捨てる)。採った非零の数の合計 / LP の列数 が予算 4.5 に近づいたら
//! (残りが 1 列ぶん未満)、残りの予算に収まらない密度のものを捨てる。点の下限は 0 (相対の閾値はない)。
//!
//! 値の文字: `w` = 1 ラウンドの本数の上限を `ENOMOTO_T_MIP_CUTSEL_ENS_MAX` (既定 2000、SCIP の根の `separating/maxcutsroot`)
//! にする (ないときは今の上限 (根 50 本、広げたラウンドはその本数) のまま)。`f` = 平行度で罰するのではなく捨てる (SCIP の
//! `filterparalcuts`)。`c` = 候補が `ENOMOTO_T_MIP_CUTSEL_ENS_MAXCANDS` (既定 200、SCIP の `maxcuts`) を超えたら既定の選び方に戻す
//! (SCIP は次の優先度の選び方に任せる)。`p` = 平行度を罰する重み・しきい値を `ENOMOTO_T_MIP_CUTSEL_ENS_PARPEN`・`_MAXPAR` から読む。

use super::problem::MipProblem;

/// 試作が入っているか (サブ MIP では使わない)。値 (`1` など) は文字の組 (上の説明)
pub(super) fn flags(submip: bool) -> Option<&'static str> {
    if submip {
        return None;
    }
    env_str!("ENOMOTO_MIP_CUTSEL_ENS")
}

/// 変数ごとのロックの数 (上向き・下向き)。行 `lo <= a x <= up` で、`up` が有限なら a > 0 の列は上向き、a < 0 は下向きにロックされる
/// (`lo` が有限なら逆)。SCIP の `SCIPvarGetNLocksUp`・`Down` に当たる (元の行だけ、カットの行は数えない)
fn var_locks(p: &MipProblem) -> Vec<(u32, u32)> {
    let mut lk = vec![(0u32, 0u32); p.n];
    for (i, r) in p.rows.iter().enumerate() {
        let (lo, up) = (p.row_lo[i].is_finite(), p.row_up[i].is_finite());
        for &(j, a) in r {
            if a == 0.0 {
                continue;
            }
            if (up && a > 0.0) || (lo && a < 0.0) {
                lk[j].0 += 1;
            }
            if (up && a < 0.0) || (lo && a > 0.0) {
                lk[j].1 += 1;
            }
        }
    }
    lk
}

fn norm(c: &[(usize, f64)]) -> f64 {
    c.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt()
}

fn dot(a: &[(usize, f64)], b: &[(usize, f64)]) -> f64 {
    let (mut i, mut k, mut d) = (0, 0, 0.0);
    while i < a.len() && k < b.len() {
        let (x, y) = (a[i].0, b[k].0);
        if x == y {
            d += a[i].1 * b[k].1;
            i += 1;
            k += 1;
        } else if x < y {
            i += 1;
        } else {
            k += 1;
        }
    }
    d
}

/// 選んだ候補の番号 (`cuts` の (係数 (列番号の昇順)、効き目))。`forced` は選択を通さずに入るカットの係数 (残りをそれで罰する)。
/// 候補が多すぎて使わない (`c`) なら `None`
pub(super) fn select(cuts: &[(&[(usize, f64)], f64)], forced: &[&[(usize, f64)]], p: &MipProblem, max_cuts: usize, flags: &str) -> Option<Vec<usize>> {
    if flags.contains('c') && cuts.len() > tunable!("ENOMOTO_T_MIP_CUTSEL_ENS_MAXCANDS", 200usize, usize) {
        return None;
    }
    let max_cuts = if flags.contains('w') { tunable!("ENOMOTO_T_MIP_CUTSEL_ENS_MAX", 2000usize, usize) } else { max_cuts };
    let filter_par = flags.contains('f');
    let (maxpar, parpen) = if flags.contains('p') {
        (tunable!("ENOMOTO_T_MIP_CUTSEL_ENS_MAXPAR", 0.95, f64), tunable!("ENOMOTO_T_MIP_CUTSEL_ENS_PARPEN", 0.25, f64))
    } else {
        (0.95, 0.25)
    };
    // SCIP の既定の重み (cutsel_ensemble.c の DEFAULT_*)
    const W_EFF: f64 = 0.75;
    const W_OBJPAR: f64 = 0.25;
    const W_INT: f64 = 0.45;
    const W_EXP: f64 = 0.1;
    const W_LOCKS: f64 = 0.25;
    const MAX_SPARSE_BONUS: f64 = 0.5;
    const END_SPARSE_BONUS: f64 = 0.2;
    const MAX_DENSITY: f64 = 0.425;
    let nz_budget = tunable!("ENOMOTO_T_MIP_CUTSEL_ENS_NZROOT", 4.5, f64);
    // LP の列の数 (前処理で消えた列は数えない)
    let ncols = (0..p.n).filter(|&j| !p.cols[j].is_empty() && p.col_lo[j] < p.col_up[j]).count().max(1) as f64;
    let cnorm = p.cost.iter().map(|c| c * c).sum::<f64>().sqrt();
    // 密なカットを捨てる
    let mut idx: Vec<usize> = (0..cuts.len()).filter(|&i| !cuts[i].0.is_empty() && cuts[i].0.len() as f64 / ncols <= MAX_DENSITY).collect();
    if idx.is_empty() {
        return Some(Vec::new());
    }
    let lk = var_locks(p);
    let n = cuts.len();
    let mut objpar = vec![0.0; n];
    let mut locks = vec![0.0; n];
    let mut nrm = vec![0.0; n];
    let (mut maxeff, mut maxexp, mut maxlocks) = (0.0f64, 0.0f64, 0.0f64);
    for &i in &idx {
        let (c, eff) = cuts[i];
        let nc = norm(c);
        nrm[i] = nc;
        objpar[i] = if cnorm > 0.0 && nc > 0.0 { c.iter().map(|&(j, v)| v * p.cost[j]).sum::<f64>().abs() / (cnorm * nc) } else { 0.0 };
        // `a x <= b` (左辺は -inf): a > 0 は上向き、a < 0 は下向きのロック
        let l: u32 = c.iter().map(|&(j, a)| if a > 0.0 { lk[j].0 } else { lk[j].1 }).sum();
        locks[i] = l as f64 / c.len() as f64;
        maxeff = maxeff.max(eff);
        maxexp = maxexp.max(eff * objpar[i]);
        maxlocks = maxlocks.max(locks[i]);
    }
    let maxlocks = maxlocks.max(1.0);
    let sq = |v: f64, m: f64| if v <= 1e-6 || m <= 0.0 { 0.0 } else { (v.ln_1p() / m.ln_1p()).powi(2) };
    let mut score = vec![0.0; n];
    for &i in &idx {
        let (c, eff) = cuts[i];
        let nint = c.iter().filter(|&&(j, _)| p.is_int[j]).count() as f64;
        let density = c.len() as f64 / ncols;
        let dens = (MAX_SPARSE_BONUS - MAX_SPARSE_BONUS / END_SPARSE_BONUS * density).max(0.0);
        score[i] = W_EFF * sq(eff, maxeff)
            + W_EXP * sq(eff * objpar[i], maxexp)
            + W_OBJPAR * (1.0 - objpar[i])
            + W_INT * nint / c.len() as f64
            + dens
            + W_LOCKS * (1.0 - locks[i] / maxlocks);
    }
    // 平行なものを罰する (ほぼ 1 なら捨てる)。`f` なら平行度のしきい値を超えたら捨てる
    let penalise = |sel: &[(usize, f64)], selnorm: f64, idx: &mut Vec<usize>, score: &mut [f64]| {
        idx.retain(|&i| {
            let par = if selnorm > 0.0 && nrm[i] > 0.0 { dot(sel, cuts[i].0).abs() / (selnorm * nrm[i]) } else { 0.0 };
            if filter_par {
                return par <= maxpar;
            }
            if par > 1.0 - 1e-9 {
                return false;
            }
            if par > maxpar {
                score[i] -= parpen;
            }
            true
        });
    };
    for f in forced {
        let nf = norm(f);
        penalise(f, nf, &mut idx, &mut score);
    }
    let mut chosen = Vec::new();
    let mut taken = 0.0;
    while !idx.is_empty() {
        // 残りのうち点の最も高いもの (同点は番号の小さい方)
        let (bk, &bi) = idx.iter().enumerate().max_by(|a, b| score[*a.1].total_cmp(&score[*b.1]).then(b.1.cmp(a.1))).unwrap();
        if score[bi] < 0.0 {
            break;
        }
        chosen.push(bi);
        if chosen.len() >= max_cuts {
            break;
        }
        taken += cuts[bi].0.len() as f64 / ncols;
        idx.swap_remove(bk);
        penalise(cuts[bi].0, nrm[bi], &mut idx, &mut score);
        if nz_budget - taken < 1.0 {
            let room = nz_budget - taken;
            idx.retain(|&i| cuts[i].0.len() as f64 / ncols <= room);
        }
    }
    Some(chosen)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prob() -> MipProblem {
        // 2 行 3 列: x0 + x1 <= 1, x1 + x2 >= 1、x0・x1 は整数、x2 は連続
        MipProblem::from_rows(
            vec![0.0; 3],
            vec![1.0; 3],
            vec![1.0, 1.0, 0.0],
            0.0,
            1.0,
            vec![true, true, false],
            vec![vec![(0, 1.0), (1, 1.0)], vec![(1, 1.0), (2, 1.0)]],
            vec![f64::NEG_INFINITY, 1.0],
            vec![1.0, f64::INFINITY],
        )
    }

    #[test]
    fn parallel_cut_dropped_and_order_by_score() {
        let p = prob();
        let a: Vec<(usize, f64)> = vec![(0, 1.0)];
        let b: Vec<(usize, f64)> = vec![(0, 2.0)]; // a と平行 (捨てる)
        let c: Vec<(usize, f64)> = vec![(1, 1.0)];
        let cuts = [(a.as_slice(), 0.5), (b.as_slice(), 0.4), (c.as_slice(), 0.1)];
        let s = select(&cuts, &[], &p, 10, "1").unwrap();
        assert_eq!(s, vec![0, 2]);
    }

    #[test]
    fn dense_cut_filtered() {
        let p = prob();
        let a: Vec<(usize, f64)> = vec![(0, 1.0), (1, 1.0), (2, 1.0)]; // 密度 1 > 0.425
        let cuts = [(a.as_slice(), 0.5)];
        assert!(select(&cuts, &[], &p, 10, "1").unwrap().is_empty());
    }
}
