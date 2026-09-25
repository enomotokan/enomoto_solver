//! 二項等式の消去 (DoubletonEquation; Achterberg et al., "Presolve Reductions
//! in Mixed Integer Programming" §4.5、PaPILO の同名プレソルバと同じ方針)。
//!
//! 非ゼロ係数がちょうど 2 つの等式行 `a_i*x_i + a_k*x_k = rhs` から
//! `x_i = (rhs - a_k*x_k) / a_i` として一方の変数を消去する。数値安定性のため
//! 消去するのは係数の絶対値が大きい方 (割る数が大きい方が誤差を増幅しない)。
//! colsingleton と違い消去変数は他の行にも現れうるので、`A` の他の行と `G`
//! の多変数行をすべて同じ代入式で書き換え、消去変数の箱境界は残る相手変数上の
//! `G` 行として保存する (導出は colsingleton と同一)。
//!
//! 1 回の呼び出しは連鎖しない単一パス。詳しい経緯は改良履歴メモを参照。

use crate::presolve::colsingleton::{self, Substitution};
use crate::presolve::propagate::{self, GView};
use crate::sparse::{FaerCsr, CsrRowBuilder, SparseAccum, csr_from_rows, csr_is_canonical, csr_rows_pruned};
use crate::params::presolve::{IMPLIED_TOL, TOL};

/// [`eliminate_doubleton_equalities`] の結果。
pub struct DoubletonResult {
    /// 消去後の等式行列 `A` (二項等式行は削除、他の行は書き換え済み)。
    pub a: FaerCsr,
    /// 消去後の等式右辺。
    pub b: Vec<f64>,
    /// 消去後の不等式行列 `G` (多変数行 + 残る変数の境界行 + 境界保存行)。
    pub g: FaerCsr,
    /// `g` の右辺。
    pub h: Vec<f64>,
    /// 消去後の目的係数 (消去変数のコストは相手変数へ畳み込み済み)。
    pub c: Vec<f64>,
    /// 発見 (行走査) 順の代入記録。後処理の復元順序に必要なので並べ替えないこと。
    pub substitutions: Vec<Substitution>,
    /// 候補なし高速経路で返されたとき `true` (`a`/`b`/`g`/`h`/`c` は入力の厳密なコピー。
    /// [`inputs_pass_through_unchanged`] 参照)。
    #[allow(dead_code)] // read by tests; `eliminate_doubleton_equalities_view` returns `None` instead
    pub unchanged: bool,
}

/// 完全なパスを走らせても入力がそのまま (ビット単位で) 出てくるだけなら `true`。
///
/// 条件: 刈り込み後の `A` に二項等式の候補行がなく、`A`/`G` が正準 CSR で
/// 全要素の絶対値が `TOL` 超、かつ `G` の単一変数行が `rebuild_g_ref` と同じ
/// 形の末尾境界ブロック (`h` も含めビット一致) になっていること。
/// 分離形 `G` ([`GView::Split`]) は構成上この形なので、多変数行の `TOL` 判定だけ行う。
fn inputs_pass_through_unchanged(n: usize, a: &FaerCsr, gv: GView<'_>) -> bool {
    let ar = a.as_ref();
    for i in 0..ar.nrows() {
        let vals = ar.values_of_row(i);
        // 本体ループと同じ候補判定 (最初の二項等式行は必ず変数を確保できる)。
        let mut nz = vals.iter().enumerate().filter(|&(_, &v)| v != 0.0);
        if let (Some((p0, &v0)), Some((p1, &v1)), None) = (nz.next(), nz.next(), nz.next()) {
            let cols = ar.col_indices_of_row_raw(i);
            // 絶対値の大きい方の係数 (消去側)
            let big = if v0.abs() < v1.abs() { v1 } else { v0 };
            if !(big.abs() < TOL || cols[p0] == cols[p1]) {
                return false;
            }
        }
    }
    if !csr_is_canonical(a) || ar.values().iter().any(|&v| v.abs() <= TOL) {
        return false;
    }
    let (g, h) = match gv {
        GView::Mat { g, h } => (g, h),
        GView::Split { rows, .. } => return rows.iter().all(|r| r.iter().all(|&(_, v)| v.abs() > TOL)),
    };
    if !csr_is_canonical(g) {
        return false;
    }
    let gr = g.as_ref();
    let m = gr.nrows();
    // 先頭の多変数行を走査 (i は最初の単一変数行の位置で止まる)
    let mut i = 0;
    while i < m && gr.col_indices_of_row_raw(i).len() != 1 {
        if gr.values_of_row(i).iter().any(|&v| v.abs() <= TOL) {
            return false;
        }
        i += 1;
    }
    // 残りが変数順の (上限行, 下限行) の並びとビット一致するか
    let (lb, ub) = propagate::extract_bounds_only(n, g, h);
    for j in 0..n {
        for (bound_finite, coef, rhs) in [(ub[j].is_finite(), 1.0f64, ub[j]), (lb[j].is_finite(), -1.0f64, -lb[j])] {
            if !bound_finite {
                continue;
            }
            if i >= m {
                return false;
            }
            let cols = gr.col_indices_of_row_raw(i);
            if cols.len() != 1 || cols[0] != j || gr.values_of_row(i)[0].to_bits() != coef.to_bits() || h[i].to_bits() != rhs.to_bits() {
                return false;
            }
            i += 1;
        }
    }
    i == m
}

/// 行 `row`/右辺 `rhs` 中の消去変数をすべて代入式で置き換えた新しい行と右辺を返す。
///
/// `by_var[j]` は変数 `j` を消去した代入の `subs` 内の添字 (なければ `None`)。
/// 代入で現れた項がさらに同じパスで消去された変数であることがある (連鎖) ので、
/// 新しい項はキューに戻して再度判定する (推移的)。同一パス内で確保済み変数を
/// 両側とも使わない規則により循環はなく、必ず停止する。重複列は `accum` で合算し、
/// `|v| <= TOL` の項は落とす。出力は列の昇順。
fn rewrite_row(accum: &mut SparseAccum, row: &[(usize, f64)], rhs: f64, subs: &[Substitution], by_var: &[Option<usize>]) -> (Vec<(usize, f64)>, f64) {
    // 高速経路 (一般経路とビット一致): 列が狭義増加で消去変数を含まない行は、
    // TOL 以下の項を落とすだけでそのまま返る。
    if row.windows(2).all(|w| w[0].0 < w[1].0) && row.iter().all(|&(j, _)| by_var[j].is_none()) {
        return (crate::sparse::collect_with_capacity(row.len(), row.iter().copied().filter(|&(_, v)| v.abs() > TOL)), rhs);
    }
    // 一般経路: 共有の疎アキュムレータに合算し、列の昇順で取り出す。
    accum.reset();
    let mut new_rhs = rhs;
    // 未処理の項 (代入で新たに生じた項もここに積む)
    let mut queue: Vec<(usize, f64)> = row.to_vec();
    while let Some((j, v)) = queue.pop() {
        if let Some(idx) = by_var[j] {
            let sub = &subs[idx];
            new_rhs -= v * sub.rhs / sub.coeff;
            for &(k, term_v) in &sub.terms {
                queue.push((k, -v * term_v / sub.coeff));
            }
        } else {
            accum.add(j, v);
        }
    }
    (accum.take_sorted(TOL), new_rhs)
}

/// 二項等式消去の単一パス (`G` を CSR で受け取る版)。
///
/// 候補行と消去変数はすべて入力 `a` の形から決め、書き換え後の再判定はしない
/// (連鎖しない)。同じ変数を 2 つの行が消去しないよう、パス内で変数を確保する。
/// 何も変わらない場合は `unchanged = true` で入力のコピーを返す。
#[allow(dead_code)] // `run_extended` calls the `GView` form directly
pub fn eliminate_doubleton_equalities(n: usize, a: &FaerCsr, b: &[f64], g: &FaerCsr, h: &[f64], c: &[f64]) -> DoubletonResult {
    match eliminate_doubleton_equalities_view(n, a, b, GView::Mat { g, h }, c) {
        Some(r) => r,
        None => DoubletonResult { a: a.clone(), b: b.to_vec(), g: g.clone(), h: h.to_vec(), c: c.to_vec(), substitutions: Vec::new(), unchanged: true },
    }
}

/// [`eliminate_doubleton_equalities`] の `G` をどちらの形 ([`GView`]) でも受け取る版。
/// 入力が変わらない場合は `None` を返す (コピーを作らない)。
pub fn eliminate_doubleton_equalities_view(n: usize, a: &FaerCsr, b: &[f64], gv: GView<'_>, c: &[f64]) -> Option<DoubletonResult> {
    if inputs_pass_through_unchanged(n, a, gv) {
        return None;
    }
    Some(eliminate_doubleton_equalities_full(n, a, b, gv, c))
}

/// 高速経路を使わない本体。候補選択 → 境界保存行の生成 → 全行の書き換え →
/// `G` の再組み立て → 目的関数への代入、の順に行う。
fn eliminate_doubleton_equalities_full(n: usize, a: &FaerCsr, b: &[f64], gv: GView<'_>, c: &[f64]) -> DoubletonResult {
    // 刈り込み済み (ゼロ要素なし) の A の各行
    let a_rows: Vec<Vec<(usize, f64)>> = csr_rows_pruned(a);
    // 全ての `rewrite_row` 呼び出しで共有する疎アキュムレータ。
    let mut accum = SparseAccum::new(n);

    // CSR 形のとき extract_bounds の結果を保持する (借用の寿命のため)
    let extracted;
    // 変数境界と G の多変数行
    let (lb, ub, real_g_rows, real_g_rhs): (&[f64], &[f64], &[Vec<(usize, f64)>], &[f64]) = match gv {
        GView::Mat { g, h } => {
            extracted = propagate::extract_bounds(n, g, h);
            (&extracted.0, &extracted.1, &extracted.2, &extracted.3)
        }
        GView::Split { rows, rhs, lb, ub } => (lb, ub, rows, rhs),
    };

    // subs: 発見 (行走査) 順の代入。後処理の復元順序に必要なので並べ替えない。
    // by_var: 変数 → subs 内の添字。
    // claimed: このパスで確保済みの変数。候補行の 2 変数のどちらかが確保済みなら
    //   その行は今回は見送る (書き換え後の行として残り、次のラウンドで再候補になる)。
    // eliminated_a_row: 二項等式として消費した A 行。
    let mut subs: Vec<Substitution> = Vec::new();
    let mut by_var: Vec<Option<usize>> = vec![None; n];
    let mut claimed: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    let mut eliminated_a_row = vec![false; a_rows.len()];

    for (i, row) in a_rows.iter().enumerate() {
        if row.len() != 2 {
            continue;
        }
        // 絶対値の大きい方を消去側 (term_elim)、小さい方を残す側 (term_keep) に並べる
        let (mut term_elim, mut term_keep) = (row[0], row[1]);
        if term_elim.1.abs() < term_keep.1.abs() {
            std::mem::swap(&mut term_elim, &mut term_keep);
        }
        let (var_elim, coeff_elim) = term_elim;
        let (var_keep, coeff_keep) = term_keep;
        if coeff_elim.abs() < TOL || claimed.contains(&var_elim) || claimed.contains(&var_keep) || var_elim == var_keep {
            continue;
        }
        let rhs = b[i];

        claimed.insert(var_elim);
        by_var[var_elim] = Some(subs.len());
        subs.push(Substitution { var: var_elim, terms: vec![(var_keep, coeff_keep)], rhs, coeff: coeff_elim });
        eliminated_a_row[i] = true;
    }

    // 消去変数の箱境界を、残る相手変数上の G 行として保存する (colsingleton と同じ導出)。
    // 相手変数自身が同じパスで消去されている (連鎖) ことがあるので rewrite_row を通す。
    // 無限の境界側は自明な行になるので出さない。
    let mut extra_g_rows: Vec<Vec<(usize, f64)>> = Vec::new();
    let mut extra_h: Vec<f64> = Vec::new();
    for sub in &subs {
        let (var_keep, coeff_keep) = sub.terms[0];
        // coeff_elim * x_elim の取りうる範囲 [lo, hi]
        let a_lb = sub.coeff * lb[sub.var];
        let a_ub = sub.coeff * ub[sub.var];
        let lo = a_lb.min(a_ub);
        let hi = a_lb.max(a_ub);
        // 相手変数自身の箱が既に含意する側は省く。相手変数がこのパスで消去されて
        // いない (その箱が引き続き有効な) ときだけ。
        let (r_lo, r_hi) = if colsingleton::skip_implied_bound_rows() && by_var[var_keep].is_none() {
            colsingleton::terms_range(&[(var_keep, coeff_keep)], &lb, &ub)
        } else {
            (f64::NEG_INFINITY, f64::INFINITY)
        };
        let tol = IMPLIED_TOL;
        // coeff_keep * x_keep <= rhs - lo
        if lo.is_finite() && !(r_hi <= sub.rhs - lo + tol * (1.0 + (sub.rhs - lo).abs())) {
            let (row1, rhs1) = rewrite_row(&mut accum, &[(var_keep, coeff_keep)], sub.rhs - lo, &subs, &by_var);
            extra_g_rows.push(row1);
            extra_h.push(rhs1);
        }
        // -coeff_keep * x_keep <= hi - rhs
        if hi.is_finite() && !(r_lo >= sub.rhs - hi - tol * (1.0 + (sub.rhs - hi).abs())) {
            let (row2, rhs2) = rewrite_row(&mut accum, &[(var_keep, -coeff_keep)], hi - sub.rhs, &subs, &by_var);
            extra_g_rows.push(row2);
            extra_h.push(rhs2);
        }
    }

    // 残る行 (A の二項等式以外の行、G の多変数行) を書き換える。
    let mut new_a_rows = Vec::with_capacity(a_rows.len());
    let mut new_b = Vec::with_capacity(b.len());
    for (i, row) in a_rows.into_iter().enumerate() {
        if eliminated_a_row[i] {
            continue;
        }
        let (new_row, new_rhs) = rewrite_row(&mut accum, &row, b[i], &subs, &by_var);
        new_a_rows.push(new_row);
        new_b.push(new_rhs);
    }

    let mut new_g_rows = Vec::with_capacity(real_g_rows.len());
    let mut new_h = Vec::with_capacity(real_g_rhs.len() + 2 * n + extra_h.len());
    for (row, &rhs) in real_g_rows.iter().zip(real_g_rhs) {
        let (new_row, new_rhs) = rewrite_row(&mut accum, row, rhs, &subs, &by_var);
        new_g_rows.push(new_row);
        new_h.push(new_rhs);
    }
    // G = [new_g_rows; 消去されていない変数の境界行; extra_g_rows] を組み立てる。
    // 境界行は build_a_g/propagate と同じ形 (lb/ub 自体はこのパスで不変)。
    // CsrRowBuilder で直接組み、ビルダーが行を拒否したら csr_from_rows に戻る (同じ行列)。
    // bound_rows: (有限か, 変数, 係数, 右辺) の反復子を返すクロージャ (有限のものだけ)。
    let bound_rows = || (0..n).filter(|&j| by_var[j].is_none()).flat_map(|j| [(ub[j].is_finite(), j, 1.0, ub[j]), (lb[j].is_finite(), j, -1.0, -lb[j])]).filter(|t| t.0);
    let nnz: usize = new_g_rows.iter().chain(&extra_g_rows).map(|r| r.len()).sum::<usize>() + 2 * n;
    let mut builder = CsrRowBuilder::with_capacity(n, new_g_rows.len() + 2 * n + extra_g_rows.len(), nnz);
    // 全行をビルダーに直接積めたか
    let mut built_directly = new_g_rows.iter().all(|r| builder.push_row(r));
    if built_directly {
        for (_, j, v, rhs) in bound_rows() {
            builder.push_singleton(j, v);
            new_h.push(rhs);
        }
        built_directly = extra_g_rows.iter().all(|r| builder.push_row(r));
    } else {
        for (_, _, _, rhs) in bound_rows() {
            new_h.push(rhs);
        }
    }
    new_h.extend(extra_h);
    let new_g = if built_directly {
        builder.finish()
    } else {
        let mut all = new_g_rows;
        all.extend(bound_rows().map(|(_, j, v, _)| vec![(j, v)]));
        all.extend(extra_g_rows);
        csr_from_rows(&all, n)
    };

    // 目的関数への代入: c_k -= c_elim * a_k / coeff、c_elim = 0。
    let mut new_c = c.to_vec();
    for sub in &subs {
        let cj = new_c[sub.var];
        if cj != 0.0 {
            for &(k, v) in &sub.terms {
                new_c[k] -= cj * v / sub.coeff;
            }
            new_c[sub.var] = 0.0;
        }
    }

    DoubletonResult {
        a: csr_from_rows(&new_a_rows, n),
        b: new_b,
        g: new_g,
        h: new_h,
        c: new_c,
        substitutions: subs,
        unchanged: false,
    }
}

/// 二項等式消去のテスト。
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparse::csr_row_vec;

    /// 候補なし高速経路が発動したとき、完全パスとビット単位で同じ結果になることを
    /// ランダム入力で確認 (分離形 `G` でも高速経路に入ることも確認)。
    #[test]
    fn no_candidate_fast_path_matches_full_pass() {
        let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut rnd = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut fired = 0;
        for trial in 0..400 {
            let n = 4 + (trial % 4);
            let mut rand_row = |len: usize, rnd: &mut dyn FnMut() -> u64| -> Vec<(usize, f64)> {
                let mut row: Vec<(usize, f64)> = Vec::new();
                for _ in 0..len {
                    let j = (rnd() % n as u64) as usize;
                    if row.iter().all(|&(k, _)| k != j) {
                        let v = [1.0, -2.0, 0.5, 3.0, 1e-12][(rnd() % 5) as usize];
                        row.push((j, v));
                    }
                }
                row.sort_by_key(|&(k, _)| k);
                row
            };
            let a_rows: Vec<Vec<(usize, f64)>> = (0..(1 + rnd() % 3)).map(|_| { let l = [1usize, 3, 3, 2][(rnd() % 4) as usize]; rand_row(l, &mut rnd) }).collect();
            let g_real: Vec<Vec<(usize, f64)>> = (0..(rnd() % 3)).map(|_| { let l = 2 + (rnd() % 2) as usize; rand_row(l, &mut rnd) }).collect();
            let g_rhs: Vec<f64> = g_real.iter().map(|_| (rnd() % 5) as f64).collect();
            let lb: Vec<f64> = (0..n).map(|_| [f64::NEG_INFINITY, 0.0, -1.0][(rnd() % 3) as usize]).collect();
            let ub: Vec<f64> = (0..n).map(|_| [f64::INFINITY, 2.0, 5.0][(rnd() % 3) as usize]).collect();
            let (g, h) = propagate::rebuild_g_ref(n, &g_real, &g_rhs, &lb, &ub);
            let a = csr_from_rows(&a_rows, n);
            let b: Vec<f64> = a_rows.iter().map(|_| (rnd() % 4) as f64).collect();
            let c: Vec<f64> = (0..n).map(|_| (rnd() % 3) as f64).collect();
            if !inputs_pass_through_unchanged(n, &a, GView::Mat { g: &g, h: &h }) {
                continue;
            }
            let fast = eliminate_doubleton_equalities(n, &a, &b, &g, &h, &c);
            assert!(fast.unchanged);
            // 同じ G の分離形でも高速経路に入ること
            assert!(inputs_pass_through_unchanged(n, &a, GView::Split { rows: &g_real, rhs: &g_rhs, lb: &lb, ub: &ub }) || !propagate::split_is_canonical(n, &g_real, &lb, &ub), "trial {trial}");
            fired += 1;
            let full = eliminate_doubleton_equalities_full(n, &a, &b, GView::Mat { g: &g, h: &h }, &c);
            assert!(full.substitutions.is_empty(), "trial {trial}");
            let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            for (x, y) in [(&fast.a, &full.a), (&fast.g, &full.g)] {
                assert_eq!(x.as_ref().row_ptrs(), y.as_ref().row_ptrs(), "trial {trial}");
                assert_eq!(x.as_ref().col_indices(), y.as_ref().col_indices(), "trial {trial}");
                assert_eq!(bits(x.as_ref().values()), bits(y.as_ref().values()), "trial {trial}");
            }
            assert_eq!(bits(&fast.b), bits(&full.b));
            assert_eq!(bits(&fast.h), bits(&full.h));
            assert_eq!(bits(&fast.c), bits(&full.c));
        }
        assert!(fired > 20, "fast path fired only {fired} times");
    }

    /// 係数の大きい方の変数が消去され、それを参照する他の A 行が書き換えられることを確認。
    #[test]
    fn eliminates_larger_coefficient_variable_and_rewrites_other_rows() {
        // 二項等式 4*x0 + 2*x1 = 12 → |4|>|2| なので x0 = 3 - 0.5*x1 と消去。
        // 他の A 行 x0 + x2 = 5 → 代入後 -0.5*x1 + x2 = 2。
        let a = csr_from_rows(&[vec![(0, 4.0), (1, 2.0)], vec![(0, 1.0), (2, 1.0)]], 3);
        let b = vec![12.0, 5.0];
        // 境界は g/h に畳み込み (build_a_g の規約): 全変数 [0,10]
        let g = csr_from_rows(
            &[
                vec![(0, 1.0)],
                vec![(0, -1.0)],
                vec![(1, 1.0)],
                vec![(1, -1.0)],
                vec![(2, 1.0)],
                vec![(2, -1.0)],
            ],
            3,
        );
        let h = vec![10.0, 0.0, 10.0, 0.0, 10.0, 0.0];
        let c = vec![0.0, 0.0, 0.0];

        let result = eliminate_doubleton_equalities(3, &a, &b, &g, &h, &c);
        assert_eq!(result.substitutions.len(), 1);
        let sub = &result.substitutions[0];
        assert_eq!(sub.var, 0);
        assert_eq!(sub.terms, vec![(1, 2.0)]);
        assert_eq!(sub.rhs, 12.0);
        assert_eq!(sub.coeff, 4.0);

        // 二項等式行は消え、もう一方の A 行は書き換えられて残る
        assert_eq!(result.a.nrows(), 1);
        let row0 = csr_row_vec(&result.a, 0);
        assert!(row0.iter().any(|&(j, v)| j == 1 && (v - (-0.5)).abs() < 1e-9));
        assert!(row0.iter().any(|&(j, v)| j == 2 && (v - 1.0).abs() < 1e-9));
        assert!(!row0.iter().any(|&(j, _)| j == 0));
        assert!((result.b[0] - 2.0).abs() < 1e-9);
    }

    /// 代入記録から消去変数の値が正しく復元されることを確認。
    #[test]
    fn recovers_eliminated_variable_value_within_its_own_bounds() {
        let a = csr_from_rows(&[vec![(0, 4.0), (1, 2.0)]], 2);
        let b = vec![12.0];
        let g = csr_from_rows(&[vec![(0, 1.0)], vec![(0, -1.0)], vec![(1, 1.0)], vec![(1, -1.0)]], 2);
        let h = vec![10.0, 0.0, 10.0, 0.0];
        let c = vec![0.0, 0.0];
        let result = eliminate_doubleton_equalities(2, &a, &b, &g, &h, &c);
        let sub = &result.substitutions[0];
        // x1 = 3 なら x0 = (12 - 2*3)/4 = 1.5
        let x = [0.0, 3.0];
        assert!((sub.value(&x) - 1.5).abs() < 1e-9);
    }
}
