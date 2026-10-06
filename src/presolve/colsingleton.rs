//! 列シングルトン消去 (Achterberg et al. 2019 / PaPILO の `ColSingleton`)。
//!
//! 実制約 (A の行と G の多変数行) にちょうど 1 回しか現れない変数 `x_j` が
//! 等式行に現れる場合、その行を `x_j` について解いて代入消去する:
//! 目的係数を行内の他変数へ振り替え、変数と行の両方を問題から落とす。
//! `x_j` の値は後処理で [`Substitution::value`] により復元する。
//!
//! G の単一変数行 (箱制約) は出現回数に数えない。候補は A の等式行のみ
//! (不等式行の場合は符号による場合分けが必要なので扱わない)。
//!
//! 行を落とすと `lb_j <= x_j <= ub_j` を強制するものが無くなるため、
//! `lb_j <= (rhs - r)/coeff <= ub_j` (`r` = 行の他項の和) を `r` についての
//! 最大 2 本の `<=` 行に変換し、呼び出し側が G/h に追加する。無限の側は
//! 空の制約になるので出力しない (両側無限の「自由列シングルトン」は置換行 0 本)。

use crate::presolve::propagate::GView;
use crate::sparse::{FaerCsr, csr_from_rows, csr_is_canonical, csr_rows};
use crate::params::presolve::{IMPLIED_TOL, SUBSTITUTION_PIVOT_RATIO, TOL};

/// 代入式 `x[var] = (rhs - Σ terms[k].1 * x[terms[k].0]) / coeff`。
/// 後処理で他変数の解から消去変数の値を復元するのに使う。
pub struct Substitution {
    /// 消去された変数の列番号。
    pub var: usize,
    /// 行内の他変数 `(列番号, 係数)`。
    pub terms: Vec<(usize, f64)>,
    /// 行の右辺。
    pub rhs: f64,
    /// 消去変数の係数 (ピボット)。
    pub coeff: f64,
}

impl Substitution {
    /// `x` 中の他変数の値から `x[self.var]` を計算して返す。
    /// `terms` が参照する全添字の値が既に確定していることが前提。
    pub fn value(&self, x: &[f64]) -> f64 {
        let mut rhs = self.rhs;
        for &(k, v) in &self.terms {
            rhs -= v * x[k];
        }
        rhs / self.coeff
    }
}

/// `colsingleton`/`doubleton` が、残りの項の箱制約から既に含意される
/// 境界保存行を省略するかどうか (既定で省略。`ENOMOTO_KEEP_IMPLIED_BOUND_ROWS`
/// を設定すると省略しない)。
pub(crate) fn skip_implied_bound_rows() -> bool {
    env_str!("ENOMOTO_KEEP_IMPLIED_BOUND_ROWS").is_none()
}

/// 各項の箱 `[lb_k, ub_k]` 上での `Σ v * x_k` の値域 `(最小, 最大)`。
/// NaN (∞-∞) は該当側の無限大として扱う。
pub(crate) fn terms_range(terms: &[(usize, f64)], lb: &[f64], ub: &[f64]) -> (f64, f64) {
    let (mut lo, mut hi) = (0.0f64, 0.0f64);
    for &(k, v) in terms {
        let (a, b) = if v > 0.0 { (v * lb[k], v * ub[k]) } else { (v * ub[k], v * lb[k]) };
        lo += a;
        hi += b;
    }
    (if lo.is_nan() { f64::NEG_INFINITY } else { lo }, if hi.is_nan() { f64::INFINITY } else { hi })
}

/// 列シングルトン消去 1 パスの結果。
pub struct EliminationResult {
    /// 消去した行を除いた新しい等式行列 A。
    pub a: FaerCsr,
    /// 新しい等式右辺 b。
    pub b: Vec<f64>,
    /// 目的係数を振り替えた後の c (消去変数の係数は 0)。
    pub c: Vec<f64>,
    /// 呼び出し側が G に追加すべき `<=` 行: 消去変数の箱制約を
    /// 残りの変数に写したもの (正しさのため必須)。
    pub extra_g_rows: Vec<Vec<(usize, f64)>>,
    /// `extra_g_rows` に対応する右辺 h。
    pub extra_h: Vec<f64>,
    /// 行った代入 (後処理で逆順に適用する)。
    pub substitutions: Vec<Substitution>,
}

/// G を行列形式で受け取る版の列シングルトン消去 (1 パス、連鎖しない)。
/// 出現回数は入力から一度だけ数えるので、消去によって新たに生じた
/// シングルトンは次回の呼び出しで拾われる。
#[allow(dead_code)] // `run_extended` は `GView` 版を直接呼ぶ
pub fn eliminate_singleton_equalities(n: usize, a: &FaerCsr, b: &[f64], g: &FaerCsr, h: &[f64], c: &[f64]) -> EliminationResult {
    eliminate_singleton_equalities_view(n, a, b, GView::Mat { g, h }, c)
}

/// 列シングルトン消去の本体。G は [`GView`] のどちらの形式でもよい
/// (分割形式なら `lb`/`ub` と多変数行をそのまま使う)。
///
/// `n`: 変数数、`a`/`b`: 等式制約、`gv`: 不等式制約 (箱制約込み)、`c`: 目的係数。
pub fn eliminate_singleton_equalities_view(n: usize, a: &FaerCsr, b: &[f64], gv: GView<'_>, c: &[f64]) -> EliminationResult {
    let a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);

    // 各変数の箱制約。G の多変数行はコピーせずその場で出現回数だけ数える。
    let (lb, ub) = gv.bounds(n);

    // 実制約 (A の行 + G の多変数行) での各変数の出現回数。G の単一変数行は箱制約なので数えない。
    let mut appearances = vec![0usize; n];
    // 各変数が最後に現れた A の行 (出現 1 回の変数ではその唯一の行)。
    let mut owning_a_row = vec![None; n];
    for (i, row) in a_rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 {
                appearances[j] += 1;
                owning_a_row[j] = Some(i);
            }
        }
    }
    match gv {
        GView::Mat { g, .. } => {
            let gr = g.as_ref();
            for i in 0..gr.nrows() {
                let cols = gr.col_indices_of_row_raw(i);
                if cols.len() == 1 {
                    continue; // 箱制約行は数えない
                }
                for (&j, &v) in cols.iter().zip(gr.values_of_row(i)) {
                    if v != 0.0 {
                        appearances[j] += 1;
                    }
                }
            }
        }
        // 分割形式: rows は多変数行のみ。
        GView::Split { rows, .. } => {
            for row in rows {
                for &(j, _) in row {
                    appearances[j] += 1;
                }
            }
        }
    }

    // 代入に使って削除する A の行。
    let mut eliminated_rows = vec![false; a_rows.len()];
    let mut substitutions = Vec::new();
    let mut extra_g_rows = Vec::new();
    let mut extra_h = Vec::new();
    let mut new_c = c.to_vec();

    for j in 0..n {
        if appearances[j] != 1 {
            continue;
        }
        let Some(i) = owning_a_row[j] else { continue }; // 唯一の出現が G 側
        if eliminated_rows[i] {
            // この行は既に別のシングルトン列の消去に使われた (1 行で解けるのは 1 変数のみ)。
            // `j` は変数として残り、その代入式の項になる。
            continue;
        }
        let row = &a_rows[i];
        let coeff = row.iter().find(|&&(k, _)| k == j).unwrap().1;
        if coeff.abs() < TOL {
            continue;
        }
        // Markowitz 型のピボット判定: 行の最大絶対値に比べて小さすぎる係数で割ると、
        // 復元時に誤差が `max|row|/|coeff|` 倍に増幅されるので消去しない。
        let row_max = row.iter().map(|&(_, v)| v.abs()).fold(0.0f64, f64::max);
        if coeff.abs() < tunable!("ENOMOTO_T_CS_SUBSTITUTION_PIVOT_RATIO", SUBSTITUTION_PIVOT_RATIO, f64) * row_max {
            continue;
        }
        // x_j 以外の項。
        let terms: Vec<(usize, f64)> = crate::sparse::collect_with_capacity(row.len(), row.iter().filter(|&&(k, _)| k != j).cloned());
        let rhs = b[i];

        // 目的係数 c_j を他変数へ振り替える: c_k -= c_j * a_ik / coeff。
        let cj = new_c[j];
        if cj != 0.0 {
            for &(k, a_ik) in &terms {
                new_c[k] -= cj * a_ik / coeff;
            }
            new_c[j] = 0.0;
        }

        // x_j の箱制約を r = Σterms に写す:
        // lb_j <= (rhs - r)/coeff <= ub_j  <=>  rhs-hi <= r <= rhs-lo
        // (lo/hi = min/max(coeff*lb_j, coeff*ub_j) で coeff の符号を吸収)。
        // 無限の側は自明な制約になるので出力しない。
        // 符号で向きを決める (min/max で並べ替えると、矛盾した境界 lb_j > ub_j が
        // 正常な区間に化けて実行不能性が失われる。矛盾したまま lo > hi で出せば、
        // 出力される 2 本の行が互いに矛盾し、後段が実行不能を検出する)。
        let (lo, hi) = if coeff >= 0.0 { (coeff * lb[j], coeff * ub[j]) } else { (coeff * ub[j], coeff * lb[j]) };
        // 残りの項の箱から既に含意される側は冗長なので省略する。
        // r_lo/r_hi: 項の箱から得られる r の値域。
        let (r_lo, r_hi) = if skip_implied_bound_rows() { terms_range(&terms, &lb, &ub) } else { (f64::NEG_INFINITY, f64::INFINITY) };
        if lo.is_finite() && !(r_hi <= rhs - lo + IMPLIED_TOL * (1.0 + (rhs - lo).abs())) {
            extra_g_rows.push(terms.clone());
            extra_h.push(rhs - lo);
        }
        if hi.is_finite() && !(r_lo >= rhs - hi - IMPLIED_TOL * (1.0 + (rhs - hi).abs())) {
            extra_g_rows.push(terms.iter().map(|&(k, v)| (k, -v)).collect());
            extra_h.push(hi - rhs);
        }

        substitutions.push(Substitution { var: j, terms, rhs, coeff });
        eliminated_rows[i] = true;
    }

    let mut new_a_rows = Vec::with_capacity(a_rows.len());
    let mut new_b = Vec::with_capacity(b.len());
    for (i, row) in a_rows.into_iter().enumerate() {
        if !eliminated_rows[i] {
            new_a_rows.push(row);
            new_b.push(b[i]);
        }
    }

    // 何も消去せず `a` が既に正準形なら、再構築せずそのまま複製する。
    let new_a = if substitutions.is_empty() && csr_is_canonical(a) { a.clone() } else { csr_from_rows(&new_a_rows, n) };
    EliminationResult {
        a: new_a,
        b: new_b,
        c: new_c,
        extra_g_rows,
        extra_h,
        substitutions,
    }
}
