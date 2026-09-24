//! 自由変数の消去 (FreeVar)。
//!
//! 両側の境界が無限 (`lb=-inf`, `ub=+inf`) の真の自由変数を、それが現れる
//! 等式行を通じたガウス消去で `A x = b` から消去する。
//! [`crate::presolve::colsingleton`] の「1 行にしか現れない」制約を「何行でも可」
//! に一般化したもの:
//! 1. 各自由変数について、現れる行のうち係数の絶対値が最大の行をピボット行に
//!    選び (`SUBSTITUTION_PIVOT_RATIO` で相対的に小さすぎるピボットは拒否)、
//!    変数について解き、他の等式行・多変数不等式行 (`real_rows`)・目的関数に
//!    代入する。ある変数の消去で別の自由変数の出現が消えることがあるので、
//!    `A` に出現する自由変数がなくなるまで反復する。
//! 2. 残った自由変数のうち:
//!    - どこにも現れないもの: 目的係数が非ゼロなら問題は非有界、ゼロなら 0 に固定。
//!    - 不等式行にちょうど 1 回だけ現れるもの: 係数と目的係数の符号が異なる
//!      (目的がその境界へ押し付ける向き) ならその行の境界で代入して行を削除、
//!      同符号なら非有界。これも連鎖的に反復する。
//!    - 不等式行に 2 回以上現れるもの: 触らない (単体法側の `x = x+ - x-` 分割に任せる)。
//!
//! 自由変数は境界がないので、colsingleton/doubleton のような境界保存行は不要。
//! 目的関数から落ちる定数項も追跡しない (最終目的値は元のコストと復元した
//! `x` から再計算されるため)。
//!
//! 詳しい導出・経緯は改良履歴メモを参照。

use crate::presolve::colsingleton::Substitution;
use crate::sparse::{Csr, SparseAccum, axpy_row, csr_from_rows, csr_rows};
use crate::params::presolve::{SUBSTITUTION_PIVOT_RATIO, TOL};

/// [`eliminate_free_variables`] の結果。
pub struct FreeVarResult {
    /// 消去後の等式行列 `A` (ピボット行は削除済み)。
    pub a: Csr,
    /// 消去後の等式右辺 `b`。
    pub b: Vec<f64>,
    /// 消去後の目的係数 `c` (消去変数のコストは他の列に畳み込み済み、自身は 0)。
    pub c: Vec<f64>,
    /// 発見順の代入記録 (後処理で逆順に適用して消去変数の値を復元する)。
    pub substitutions: Vec<Substitution>,
    /// 目的係数ゼロで出現もない自由変数の固定 `(変数, 0.0)`。
    /// 呼び出し側は `dualfix` の固定と同様に `lb[j] = ub[j] = 値` とする。
    pub fixed: Vec<(usize, f64)>,
    /// 非有界な自由変数が見つかったら `true`。このとき問題は非有界であり、
    /// 他のフィールドは途中状態なので信用してはならない。
    pub unbounded: bool,
    /// 消去で削除・書き換えた後の多変数不等式行。呼び出し側は元のコピーでは
    /// なく必ずこちらを以後使うこと。
    pub real_rows: Vec<Vec<(usize, f64)>>,
    /// `real_rows` の右辺。
    pub real_rhs: Vec<f64>,
}

/// 変数 `j` が `real_rows` のいずれかの行に非ゼロ係数で現れるか。
fn appears_in(real_rows: &[Vec<(usize, f64)>], j: usize) -> bool {
    real_rows.iter().any(|row| row.iter().any(|&(k, v)| k == j && v != 0.0))
}

/// 自由変数の消去を行う (モジュール doc の手順 1, 2)。
///
/// - `n`: 構造変数の数。`lb`/`ub` が両方無限の変数が自由変数。
/// - `a`/`b`: 等式系 `A x = b`。
/// - `c`: 目的係数。
/// - `real_rows`/`real_rhs`: 多変数の不等式行 `G x <= h` (単一変数の境界行は除く)。
///
/// 自由変数がなければ入力のコピーをそのまま返す。
pub fn eliminate_free_variables(n: usize, a: &Csr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64]) -> FreeVarResult {
    // 変数ごとの「真の自由変数か」
    let is_free: Vec<bool> = (0..n).map(|j| lb[j] == f64::NEG_INFINITY && ub[j] == f64::INFINITY).collect();
    if !is_free.iter().any(|&f| f) {
        return FreeVarResult {
            a: a.clone(),
            b: b.to_vec(),
            c: c.to_vec(),
            substitutions: Vec::new(),
            fixed: Vec::new(),
            unbounded: false,
            real_rows: real_rows.to_vec(),
            real_rhs: real_rhs.to_vec(),
        };
    }

    // 書き換え中の等式行 (行ごとの疎ベクトル)
    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);
    // 全ての行の畳み込みで共有する疎アキュムレータ。
    let mut accum = SparseAccum::new(n);
    let mut b: Vec<f64> = b.to_vec();
    let mut c: Vec<f64> = c.to_vec();
    // 消去済みフラグ
    let mut eliminated = vec![false; n];
    let mut substitutions = Vec::new();
    // 多変数不等式行も両パスで書き換える: A 経由で消去した変数が real_rows にも
    // 現れていれば同じ代入で消さないと、下流で「消去変数 = 0 固定」と
    // 読まれて誤った問題になる。
    let mut real_rows: Vec<Vec<(usize, f64)>> = real_rows.to_vec();
    let mut real_rhs: Vec<f64> = real_rhs.to_vec();
    // A 行での最良ピボットが SUBSTITUTION_PIVOT_RATIO を満たさず消去を見送った自由変数。
    // 以後の A パス、不等式パス、最後の残り自由変数の処理のすべてから除外する
    // (まだ A 行に現れているので「制約なし」とは扱えない)。
    let mut weak_pivot_in_a = vec![false; n];

    // --- パス 1: 等式行 A を通じた消去 ---
    loop {
        // 各自由変数が現れる A 行の一覧 (消去のたびに出現が減りうるので毎回作り直す)。
        let mut appearances: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, row) in a_rows.iter().enumerate() {
            for &(j, v) in row {
                if is_free[j] && !eliminated[j] && v != 0.0 {
                    appearances[j].push(i);
                }
            }
        }
        // 次に消去する自由変数 j (添字最小のもの)
        let Some(j) = (0..n).find(|&j| is_free[j] && !eliminated[j] && !weak_pivot_in_a[j] && !appearances[j].is_empty()) else {
            break;
        };

        // j の係数の絶対値が最大の行をピボット行に選ぶ
        let mut best_i = appearances[j][0];
        let mut best_coeff = 0.0f64;
        for &i in &appearances[j] {
            let coeff = a_rows[i].iter().find(|&&(k, _)| k == j).unwrap().1;
            if coeff.abs() > best_coeff.abs() {
                best_coeff = coeff;
                best_i = i;
            }
        }
        // ピボット係数
        let coeff = best_coeff;
        let pivot_row = a_rows[best_i].clone();
        // ピボット行の係数の最大絶対値 (相対ピボット判定用)
        let row_max = pivot_row.iter().map(|&(_, v)| v.abs()).fold(0.0f64, f64::max);
        if coeff.abs() < tunable!("ENOMOTO_T_FV_SUBSTITUTION_PIVOT_RATIO", SUBSTITUTION_PIVOT_RATIO, f64) * row_max {
            weak_pivot_in_a[j] = true;
            continue;
        }
        let rhs_i = b[best_i];
        // ピボット行のうち j 以外の項 (代入式 x_j = (rhs_i - sum terms) / coeff の右辺)
        let terms: Vec<(usize, f64)> = pivot_row.iter().filter(|&&(k, _)| k != j).copied().collect();

        // j が現れる他の A 行へ代入 (行 i2 -= factor * ピボット行)
        for &i2 in &appearances[j] {
            if i2 == best_i {
                continue;
            }
            let a_i2j = a_rows[i2].iter().find(|&&(k, _)| k == j).unwrap().1;
            let factor = a_i2j / coeff;
            a_rows[i2] = axpy_row(&mut accum, &a_rows[i2], &pivot_row, factor, j, TOL);
            b[i2] -= factor * rhs_i;
        }

        // j が現れる多変数不等式行へも同じ代入 (行は削除せず書き換えるだけ)。
        for i2 in 0..real_rows.len() {
            let Some(&(_, a_i2j)) = real_rows[i2].iter().find(|&&(k, _)| k == j) else {
                continue;
            };
            let factor = a_i2j / coeff;
            real_rows[i2] = axpy_row(&mut accum, &real_rows[i2], &pivot_row, factor, j, TOL);
            real_rhs[i2] -= factor * rhs_i;
        }

        // 目的関数への代入: c_k -= c_j / coeff * a_ik、c_j = 0 (定数項は捨てる)
        let cj = c[j];
        if cj != 0.0 {
            let factor = cj / coeff;
            for &(k, a_ik) in &terms {
                c[k] -= factor * a_ik;
            }
            c[j] = 0.0;
        }

        substitutions.push(Substitution { var: j, terms, rhs: rhs_i, coeff });
        eliminated[j] = true;
        a_rows.remove(best_i);
        b.remove(best_i);
    }

    // --- パス 2: 不等式行にちょうど 1 回だけ現れる自由変数の消去 ---
    // ここに来る自由変数は A に出現しない (weak_pivot_in_a のものは除外)。
    // 目的が境界へ押し付ける向きで、かつピボットが相対閾値を満たせば、その行の
    // 境界で代入して行を削除する。行の削除で別の自由変数の出現数が減るので反復する。
    // 不等式行で相対ピボット判定に失敗した自由変数。
    let mut weak_pivot_in_g = vec![false; n];
    loop {
        // 各自由変数が現れる real_rows の行一覧
        let mut appearances: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, row) in real_rows.iter().enumerate() {
            for &(j, v) in row {
                if is_free[j] && !eliminated[j] && !weak_pivot_in_a[j] && v != 0.0 {
                    appearances[j].push(i);
                }
            }
        }
        let Some(j) = (0..n).find(|&j| is_free[j] && !eliminated[j] && !weak_pivot_in_a[j] && !weak_pivot_in_g[j] && appearances[j].len() == 1) else {
            break;
        };
        // j が現れる唯一の不等式行
        let i = appearances[j][0];
        let row = real_rows[i].clone();
        let coeff = row.iter().find(|&&(k, _)| k == j).unwrap().1;
        let row_max = row.iter().map(|&(_, v)| v.abs()).fold(0.0f64, f64::max);
        if coeff.abs() < tunable!("ENOMOTO_T_FV_SUBSTITUTION_PIVOT_RATIO", SUBSTITUTION_PIVOT_RATIO, f64) * row_max {
            weak_pivot_in_g[j] = true;
            continue;
        }
        let cj = c[j];

        // coeff と cj が (TOL を超えて) 同符号なら、この行は目的に関係ない側しか
        // 抑えておらず、逆向きに x_j はいくらでも動ける → 非有界。
        // cj ≈ 0 ならどの値でもよいので行の境界での代入は常に有効。
        if cj.abs() > TOL && (coeff > 0.0) == (cj > 0.0) {
            return FreeVarResult { a: csr_from_rows(&a_rows, n), b, c, substitutions, fixed: Vec::new(), unbounded: true, real_rows, real_rhs };
        }

        let rhs_i = real_rhs[i];
        // 行のうち j 以外の項
        let terms: Vec<(usize, f64)> = row.iter().filter(|&&(k, _)| k != j).copied().collect();

        // パス 1 と同じ目的関数への代入 (cj == 0 ちょうどのときだけ省略)。
        if cj != 0.0 {
            let factor = cj / coeff;
            for &(k, a_ik) in &terms {
                c[k] -= factor * a_ik;
            }
            c[j] = 0.0;
        }

        substitutions.push(Substitution { var: j, terms, rhs: rhs_i, coeff });
        eliminated[j] = true;
        real_rows.remove(i);
        real_rhs.remove(i);
    }

    // --- 残りの自由変数: どこにも現れないもの ---
    let mut fixed = Vec::new();
    for j in 0..n {
        if !is_free[j] || eliminated[j] || weak_pivot_in_a[j] || appears_in(&real_rows, j) {
            continue;
        }
        if c[j].abs() > TOL {
            // 目的係数が非ゼロ → 目的を無限に改善できるので非有界
            return FreeVarResult { a: csr_from_rows(&a_rows, n), b, c, substitutions, fixed: Vec::new(), unbounded: true, real_rows, real_rhs };
        }
        fixed.push((j, 0.0));
    }

    FreeVarResult { a: csr_from_rows(&a_rows, n), b, c, substitutions, fixed, unbounded: false, real_rows, real_rhs }
}

/// [`eliminate_free_variables`] のテスト。
#[cfg(test)]
mod tests {
    use super::*;

    /// 行リストから CSR を作るテスト用の短縮ヘルパ。
    fn csr(rows: &[Vec<(usize, f64)>], n: usize) -> Csr {
        csr_from_rows(rows, n)
    }

    /// 自由変数がなければ何も変わらないことを確認。
    #[test]
    fn no_free_variables_is_a_no_op() {
        let a = csr(&[vec![(0, 1.0), (1, 1.0)]], 2);
        let b = vec![5.0];
        let c = vec![1.0, 2.0];
        let lb = vec![0.0, 0.0];
        let ub = vec![10.0, 10.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(!r.unbounded);
        assert!(r.substitutions.is_empty());
        assert!(r.fixed.is_empty());
        assert_eq!(r.b, b);
        assert_eq!(r.c, c);
    }

    /// 2 本の等式行に現れる自由変数が消去され、他行・目的関数に正しく畳み込まれることを確認。
    #[test]
    fn eliminates_free_variable_appearing_in_two_rows() {
        // x0 自由, x1,x2 in [0,10]。行: x0 + x1 = 5, x0 - x2 = 1。
        // 行 0 で x0 = 5 - x1 と消去し行 1 へ代入: -x1 - x2 = -4。
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![3.0, 1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0, 0.0];
        let ub = vec![f64::INFINITY, 10.0, 10.0];
        let r = eliminate_free_variables(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(!r.unbounded);
        assert_eq!(r.substitutions.len(), 1);
        assert_eq!(r.substitutions[0].var, 0);
        // x0 のコスト 3.0 が x1 へ -3*1/1 = -3 として畳み込まれる
        assert_eq!(r.c[1], 1.0 - 3.0);
        assert_eq!(r.c[2], 1.0);
        assert_eq!(r.a.nrows(), 1);
        // x1=2, x2=1 から x0 = 5 - 2 = 3 を復元
        let x = vec![3.0, 2.0, 1.0];
        assert!((r.substitutions[0].value(&x) - 3.0).abs() < 1e-9);
    }

    /// A 行で消去した変数が共有する多変数不等式行にも代入されることを確認 (回帰テスト)。
    #[test]
    fn a_row_elimination_folds_into_a_shared_real_row_too() {
        // x0 自由, x1,x2 in [0,10]。A: x0 - x1 = 0 (x0 の唯一の A 出現)、
        // real_rows: x0 + x2 <= 5 (x0 がここにも現れる)。
        let a = csr(&[vec![(0, 1.0), (1, -1.0)]], 3);
        let b = vec![0.0];
        let c = vec![0.0, -2.0, -1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0, 0.0];
        let ub = vec![f64::INFINITY, 10.0, 10.0];
        let real_rows = vec![vec![(0, 1.0), (2, 1.0)]];
        let real_rhs = vec![5.0];
        let r = eliminate_free_variables(3, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert!(!r.unbounded);
        assert_eq!(r.substitutions.len(), 1);
        assert_eq!(r.substitutions[0].var, 0);
        // 共有行は x0 を含まず x1 + x2 <= 5 になっているべき (x0 = x1 の代入)
        assert_eq!(r.real_rows.len(), 1);
        assert_eq!(r.real_rows[0], vec![(1, 1.0), (2, 1.0)]);
        assert_eq!(r.real_rhs, vec![5.0]);
    }

    /// どこにも現れず目的係数が非ゼロの自由変数で非有界になることを確認。
    #[test]
    fn leftover_free_variable_with_nonzero_cost_is_unbounded() {
        // x0 自由、どこにも現れない。
        let a = csr(&[vec![(1, 1.0)]], 2);
        let b = vec![5.0];
        let c = vec![1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, 10.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(r.unbounded);
    }

    /// どこにも現れず目的係数ゼロの自由変数が 0 に固定されることを確認。
    #[test]
    fn leftover_free_variable_with_zero_cost_is_fixed() {
        let a = csr(&[vec![(1, 1.0)]], 2);
        let b = vec![5.0];
        let c = vec![0.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, 10.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(!r.unbounded);
        assert_eq!(r.fixed, vec![(0, 0.0)]);
    }

    /// 2 本の不等式行に現れる自由変数はそのまま残されることを確認。
    #[test]
    fn free_variable_in_two_inequality_rows_is_left_alone() {
        // x0 自由で 2 本の不等式行に現れる: どちらか 1 本では決まらないので触らない。
        let a = csr(&[vec![(1, 1.0)]], 2);
        let b = vec![5.0];
        let c = vec![1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, 10.0];
        let real_rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (1, -1.0)]];
        let real_rhs = vec![5.0, 5.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert!(!r.unbounded);
        assert!(r.fixed.is_empty());
        assert!(r.substitutions.is_empty());
        assert_eq!(r.real_rows, real_rows);
        assert_eq!(r.real_rhs, real_rhs);
    }

    /// 不等式行 1 本だけに現れ符号が有利な自由変数が、行の境界で代入され行が削除されることを確認。
    #[test]
    fn free_variable_in_one_inequality_row_with_favorable_sign_is_substituted() {
        // x0 自由, x1 in [0,10]; 行 x0 + x1 <= 5, コスト -x0 + x1。
        // 係数 +1 とコスト -1 が異符号 → x0 = 5 - x1 と代入し行を削除。
        let a = csr(&[], 2);
        let b = vec![];
        let c = vec![-1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, 10.0];
        let real_rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let real_rhs = vec![5.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert!(!r.unbounded);
        assert!(r.fixed.is_empty());
        assert_eq!(r.substitutions.len(), 1);
        assert_eq!(r.substitutions[0].var, 0);
        assert!(r.real_rows.is_empty(), "the now-redundant row must be dropped, real_rows={:?}", r.real_rows);
        assert!(r.real_rhs.is_empty());
        // x0 のコスト -1 が x1 へ畳み込まれ c[1] = 1 + 1 = 2
        assert_eq!(r.c[0], 0.0);
        assert_eq!(r.c[1], 2.0);
        // x1=2 から x0 = 5 - 2 = 3 を復元
        let x = vec![3.0, 2.0];
        assert!((r.substitutions[0].value(&x) - 3.0).abs() < 1e-9);
    }

    /// 不等式行 1 本だけに現れ係数とコストが同符号の自由変数で非有界になることを確認。
    #[test]
    fn free_variable_in_one_inequality_row_with_unfavorable_sign_is_unbounded() {
        // 上と同じ行、コスト x0 + x1: 行は x0 を上から抑えるだけで、目的は x0 を
        // -inf へ動かしたい → 非有界。
        let a = csr(&[], 2);
        let b = vec![];
        let c = vec![1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, 10.0];
        let real_rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let real_rhs = vec![5.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert!(r.unbounded);
    }

    /// 目的係数ゼロなら係数の符号によらず不等式行の境界で代入されることを確認。
    #[test]
    fn free_variable_in_one_inequality_row_with_zero_cost_is_substituted() {
        let a = csr(&[], 2);
        let b = vec![];
        let c = vec![0.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, 10.0];
        let real_rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let real_rhs = vec![5.0];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert!(!r.unbounded);
        assert_eq!(r.substitutions.len(), 1);
        assert!(r.real_rows.is_empty());
    }

    /// 不等式行の削除が別の自由変数の出現数を減らし、連鎖的に消去されることを確認。
    #[test]
    fn cascading_elimination_through_shared_inequality_row() {
        // x0, x1 自由, x2 in [0,10]。行 A: x0 + x1 <= 5、行 B: x1 + x2 <= 3。
        // まず x0 (出現 1) を行 A で代入 → c[1] = -2 + 1 = -1、行 A 削除。
        // x1 の出現が 1 になり、畳み込み後のコスト -1 で符号判定 → 行 B で代入、
        // c[2] = 0.5 + 1 = 1.5、行 B 削除。
        let a = csr(&[], 3);
        let b = vec![];
        let c = vec![-1.0, -2.0, 0.5];
        let lb = vec![f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0];
        let ub = vec![f64::INFINITY, f64::INFINITY, 10.0];
        let real_rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(1, 1.0), (2, 1.0)]];
        let real_rhs = vec![5.0, 3.0];
        let r = eliminate_free_variables(3, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert!(!r.unbounded);
        assert!(r.fixed.is_empty());
        assert_eq!(r.substitutions.len(), 2);
        assert_eq!(r.substitutions[0].var, 0);
        assert_eq!(r.substitutions[1].var, 1);
        assert!(r.real_rows.is_empty());
        assert!(r.real_rhs.is_empty());
        assert_eq!(r.c, vec![0.0, 0.0, 1.5]);
        // x2=2 から x1 = 3-2 = 1, x0 = 5-1 = 4 を復元 (両行ちょうど等号)
        let x = vec![4.0, 1.0, 2.0];
        assert!((r.substitutions[1].value(&x) - 1.0).abs() < 1e-9, "x1 sub: {}", r.substitutions[1].value(&x));
        assert!((r.substitutions[0].value(&x) - 4.0).abs() < 1e-9, "x0 sub: {}", r.substitutions[0].value(&x));
    }

    /// 等式行での消去が連鎖し、2 つの自由変数がともに消去されることを確認。
    #[test]
    fn cascading_elimination_of_two_free_variables() {
        // x0, x1 自由。行 0: x0 + x1 = 3、行 1: 2*x0 = 4。
        // |係数| 最大の行 1 で x0 = 2 と消去 → 行 0 が x1 の単独行になり、次の反復で x1 も消去。
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 2.0)]], 2);
        let b = vec![3.0, 4.0];
        let c = vec![1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, f64::NEG_INFINITY];
        let ub = vec![f64::INFINITY, f64::INFINITY];
        let r = eliminate_free_variables(2, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(!r.unbounded);
        assert_eq!(r.substitutions.len(), 2);
        assert_eq!(r.a.nrows(), 0);
    }
}
