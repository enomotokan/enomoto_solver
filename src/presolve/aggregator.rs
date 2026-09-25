//! Aggregator(暗黙的自由列の消去)。HiGHS の `HPresolve::aggregator` に相当する。
//!
//! [`crate::presolve::colsingleton`] の「列がちょうど 1 行にしか現れない」という
//! 制約を「任意本数の行」に一般化したもの。対象は、箱制約 `[lb_j, ub_j]` が
//! 制約系全体によって既に *暗に* 強制されている列 (implied-free 列)。
//! そのような列は等式行のどれか 1 本を使って代入消去でき、`colsingleton` の
//! `extra_g_rows` のような境界保存行は不要 (消去した変数を事後処理
//! (`Substitution::value`) で復元した値は数学的に箱の内側に入る)。
//!
//! 3 つの版がある:
//! - [`eliminate_implied_free_columns`] / [`eliminate_implied_free_columns_if_any`]:
//!   行局所版。1 本の等式行だけで箱が冗長と示せる (行, 列) 対のみ消去する。
//!   `ENOMOTO_ROWLOCAL_AGGREGATOR` で選択。
//! - [`eliminate_implied_free_columns_xrow`]: 行横断版。列が現れる全等式行の
//!   含意範囲の共通部分で判定する。`ENOMOTO_XROW_AGGREGATOR` で選択。
//! - [`eliminate_implied_free_columns_v2`] / [`eliminate_implied_free_columns_v2_if_any`]:
//!   既定版。行横断版に、不等式行からの片側含意境界・等式行 1 本の列の許可・
//!   HiGHS 流の正味 fill-in 判定を加えたもの ([`AggOptions`] で個別に切替可能)。
//!
//! 共通の設計:
//! - 候補は安い順 (行長か列長が 2 の対を最優先、次に `行長 * 列長` 昇順) に処理し、
//!   ピボット比ガード (`SUBSTITUTION_PIVOT_RATIO`) と fill-in 上限 (`MAX_FILLIN`) を課す。
//!   fill-in 超過が `MAX_CONSECUTIVE_FILLIN_FAILURES` 回連続したら残り候補を打ち切る。
//! - 行横断判定では、消去を確定する直前に「生きている行・現在の内容」から
//!   含意範囲を必ず再計算する (先に別列のピボットとして消費された行を
//!   根拠に使ってしまう誤りを防ぐため)。
//! - 1 回の呼び出しは入力スナップショットから候補を 1 回だけ作る非連鎖処理であり、
//!   不動点までの反復は呼び出し側 (`presolve.rs` のラウンドループ) の責務。
//! - 完全な自由変数 (`lb == -inf && ub == inf`) は [`crate::presolve::freevar`] に任せ、
//!   ここでは扱わない。
//! - 消去した列を含む `G` の実不等式行 (`real_rows`) も代入で書き換える (fold)。
//!
//! 開発経緯・計測値は改良履歴メモを参照。

use crate::presolve::colsingleton::Substitution;
use crate::sparse::{FaerCsr, SparseAccum, axpy_row, csr_from_rows, csr_is_canonical, csr_rows};
use crate::params::presolve::{MAX_CONSECUTIVE_FILLIN_FAILURES, MAX_FILLIN, SUBSTITUTION_PIVOT_RATIO, TOL};

/// Aggregator の 1 回の呼び出し結果 (縮小後の問題と、事後復元用の代入列)。
pub struct AggregatorResult {
    /// 消去に使った等式行を取り除き、他の行へ代入を反映した後の等式制約行列 `A`。
    pub a: FaerCsr,
    /// `a` に対応する等式右辺 `b`。
    pub b: Vec<f64>,
    /// 消去列の目的係数を他列へ移し替えた後の目的係数 `c` (消去列の係数は 0)。
    pub c: Vec<f64>,
    /// 消去した列ごとの代入式 (事後処理で消去変数の値を復元するのに使う)。消去順。
    pub substitutions: Vec<Substitution>,
    /// 本パスの代入をすべて反映済みの実不等式行 (`G` のうち多変数行)。
    /// 呼び出し側は元のコピーではなく必ずこちらを使うこと
    /// (`freevar::FreeVarResult` の同名フィールドと同じ約束)。
    pub real_rows: Vec<Vec<(usize, f64)>>,
    /// `real_rows` に対応する右辺 (`real_rows[i] · x <= real_rhs[i]`)。
    pub real_rhs: Vec<f64>,
}

/// 1 本の行の活動量 (activity) の要約。各列の箱境界の下で行の値
/// `Σ a_k x_k` が取り得る範囲を、有限部分の和と無限大項の個数に分けて保持する。
/// これにより、任意の 1 列の項を除いた残差範囲を [`residual_range`] で O(1) で
/// 求められる (無限大の和から引き算して `inf - inf` になる事態を避けるため、
/// HiGHS の `getNumInfSumUpperOrig` / `getResidualSumLowerOrig` と同じ方式)。
#[derive(Clone, Copy)]
struct RowActivity {
    /// 行の最小値側: 有限な境界を持つ項の寄与 `a_k * (a_k>0 ? lb_k : ub_k)` の和。
    lo_finite_sum: f64,
    /// 行の最小値側で境界が無限大 (-inf に寄与) の項の個数。
    lo_inf_count: usize,
    /// 行の最大値側: 有限な境界を持つ項の寄与 `a_k * (a_k>0 ? ub_k : lb_k)` の和。
    hi_finite_sum: f64,
    /// 行の最大値側で境界が無限大 (+inf に寄与) の項の個数。
    hi_inf_count: usize,
}

/// 行 `row` の活動量要約 [`RowActivity`] を、各列の箱 `[lb, ub]` から 1 パスで計算する。
fn compute_row_activity(row: &[(usize, f64)], lb: &[f64], ub: &[f64]) -> RowActivity {
    let mut lo_finite_sum = 0.0f64;
    let mut lo_inf_count = 0usize;
    let mut hi_finite_sum = 0.0f64;
    let mut hi_inf_count = 0usize;
    for &(k, v) in row {
        // 係数の符号に応じて、行の最小側/最大側に効く境界を選ぶ。
        let (klo, khi) = if v > 0.0 { (lb[k], ub[k]) } else { (ub[k], lb[k]) };
        if klo.is_finite() {
            lo_finite_sum += v * klo;
        } else {
            lo_inf_count += 1;
        }
        if khi.is_finite() {
            hi_finite_sum += v * khi;
        } else {
            hi_inf_count += 1;
        }
    }
    RowActivity { lo_finite_sum, lo_inf_count, hi_finite_sum, hi_inf_count }
}

/// 列 `j` (係数 `j_coeff`) の項を除いた、行の残り部分が取り得る範囲 `(s_lo, s_hi)` を返す。
/// `activity` は `j` 自身の項も含めた同じ行の [`compute_row_activity`] の結果。O(1)。
fn residual_range(activity: &RowActivity, j: usize, j_coeff: f64, lb: &[f64], ub: &[f64]) -> (f64, f64) {
    let (jlo, jhi) = if j_coeff > 0.0 { (lb[j], ub[j]) } else { (ub[j], lb[j]) };
    let s_lo = if jlo.is_finite() {
        if activity.lo_inf_count == 0 {
            activity.lo_finite_sum - j_coeff * jlo
        } else {
            f64::NEG_INFINITY
        }
    } else if activity.lo_inf_count == 1 {
        // 無限大項が `j` 自身だけなら、それを除いた残り (有限和) がそのまま下限。
        activity.lo_finite_sum
    } else {
        f64::NEG_INFINITY
    };
    let s_hi = if jhi.is_finite() {
        if activity.hi_inf_count == 0 {
            activity.hi_finite_sum - j_coeff * jhi
        } else {
            f64::INFINITY
        }
    } else if activity.hi_inf_count == 1 {
        activity.hi_finite_sum
    } else {
        f64::INFINITY
    };
    (s_lo, s_hi)
}

/// 等式行 `(残り) + coeff*x_j = rhs` から、他の列を現在の箱の範囲で動かしたときに
/// `x_j` が取り得る範囲 (含意範囲) `(lo, hi)` を返す。
/// 行横断版では複数行のこの範囲の共通部分を取る。
fn implied_range(activity: &RowActivity, j: usize, coeff: f64, rhs: f64, lb: &[f64], ub: &[f64]) -> (f64, f64) {
    let (s_lo, s_hi) = residual_range(activity, j, coeff, lb, ub);
    let v1 = (rhs - s_hi) / coeff;
    let v2 = (rhs - s_lo) / coeff;
    (v1.min(v2), v1.max(v2))
}

/// 含意範囲 `[lo, hi]` が列 `j` の箱 `[lb[j], ub[j]]` に (許容誤差 `TOL` 込みで)
/// 収まるか、すなわち箱制約が冗長か。無限大側の境界は常に満たされるとみなす。
fn range_within_box(j: usize, lo: f64, hi: f64, lb: &[f64], ub: &[f64]) -> bool {
    (lb[j] == f64::NEG_INFINITY || lo >= lb[j] - TOL) && (ub[j] == f64::INFINITY || hi <= ub[j] + TOL)
}

/// 等式行 `(残り) + coeff*x_j = rhs` の *この行単独* の活動量 (`activity`) だけで、
/// `x_j` の箱 `[lb[j], ub[j]]` が冗長だと示せるか (行局所版の判定)。
/// 他の行の情報は一切使わない。
fn row_implies_own_bound(activity: &RowActivity, j: usize, coeff: f64, rhs: f64, lb: &[f64], ub: &[f64]) -> bool {
    let (lo, hi) = implied_range(activity, j, coeff, rhs, lb, ub);
    range_within_box(j, lo, hi, lb, ub)
}

/// ピボット行の残り項 `pivot_terms` を `targets` の各行へ代入したとき、
/// 新たに生じる非零の総数 (各行に未だ無い列の数の合計) を返す。
fn fillin_cost(pivot_terms: &[(usize, f64)], targets: &[&Vec<(usize, f64)>]) -> usize {
    let mut cost = 0usize;
    for target in targets {
        // この対象行に既にある列の集合。
        let existing: std::collections::BTreeSet<usize> = target.iter().map(|&(k, _)| k).collect();
        cost += pivot_terms.iter().filter(|&&(k, _)| !existing.contains(&k)).count();
    }
    cost
}

/// 行局所版 Aggregator。`A` の等式行 2 本以上に現れ、そのうちの 1 本単独で箱が
/// 冗長と示せる非自由列を消去する (行 1 本だけの列は `colsingleton` の担当)。
/// 何も消去しなかった場合は入力の (正準化した) コピーを返す。
///
/// 引数: `n` 列数、`a`/`b` 等式制約、`c` 目的係数、`lb`/`ub` 各列の箱、
/// `real_rows`/`real_rhs` 実不等式行 (代入は反映するが判定には使わない)。
#[cfg_attr(not(test), allow(dead_code))]
pub fn eliminate_implied_free_columns(n: usize, a: &FaerCsr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64]) -> AggregatorResult {
    eliminate_implied_free_columns_if_any(n, a, b, c, lb, ub, real_rows, real_rhs).unwrap_or_else(|| AggregatorResult {
        a: if csr_is_canonical(a) { a.clone() } else { csr_from_rows(&csr_rows(a), n) },
        b: b.to_vec(),
        c: c.to_vec(),
        substitutions: Vec::new(),
        real_rows: real_rows.to_vec(),
        real_rhs: real_rhs.to_vec(),
    })
}

/// [`eliminate_implied_free_columns`] の本体。1 列も消去しなかった場合は
/// 問題をコピーせず `None` を返す (多くのラウンドではこれが普通)。
/// 候補探索は `a` の CSR 行を直接読み、候補が 1 つ以上あったときだけ
/// 可変な行リスト形式へコピーする。`Some` の結果は無条件コピー版と同一。
pub fn eliminate_implied_free_columns_if_any(n: usize, a: &FaerCsr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64]) -> Option<AggregatorResult> {
    let ar = a.as_ref();
    // 等式行の本数。
    let p = ar.nrows();

    // 各列が `A` の何本の行に (非零で) 現れるか。
    let mut col_a_count = vec![0usize; n];
    for i in 0..p {
        for (j, &v) in ar.col_indices_of_row(i).zip(ar.values_of_row(i)) {
            if v != 0.0 {
                col_a_count[j] += 1;
            }
        }
    }

    // 候補 (行, 列) 対: その行単独で列の箱が冗長 (`row_implies_own_bound`) かつ
    // 列が等式行 2 本以上に現れるもの。活動量は行ごとに 1 回だけ計算する (全体 O(nnz))。
    // 完全な自由列はどの行でも自明に真になるが、`freevar` に任せるため除外する。
    let is_free = |j: usize| lb[j] == f64::NEG_INFINITY && ub[j] == f64::INFINITY;
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    // 行 `i` の (列, 係数) を詰め直す再利用バッファ。
    let mut row: Vec<(usize, f64)> = Vec::new();
    for i in 0..p {
        row.clear();
        row.extend(ar.col_indices_of_row(i).zip(ar.values_of_row(i)).map(|(j, &v)| (j, v)));
        let activity = compute_row_activity(&row, lb, ub);
        for &(j, v) in &row {
            if v != 0.0 && col_a_count[j] >= 2 && !is_free(j) && row_implies_own_bound(&activity, j, v, b[i], lb, ub) {
                candidates.push((i, j));
            }
        }
    }
    if candidates.is_empty() {
        return None;
    }
    // 安い順: 行長か列長が 2 の対を先頭に、次に `行長*列長` (fill-in の目安) 昇順
    // (HiGHS `HPresolve::aggregator` の比較関数と同じ)。
    candidates.sort_by_key(|&(i, j)| {
        let rowlen = ar.col_indices_of_row(i).len();
        let collen = col_a_count[j];
        let min_len = rowlen.min(collen);
        (min_len != 2, rowlen * collen, min_len, i, j)
    });

    // ここから先は可変な行リスト形式で作業する。
    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);
    // 全ての行 fold で共有する疎アキュムレータ。
    let mut accum = SparseAccum::new(n);
    let mut b: Vec<f64> = b.to_vec();
    let mut c: Vec<f64> = c.to_vec();
    let mut real_rows: Vec<Vec<(usize, f64)>> = real_rows.to_vec();
    let mut real_rhs: Vec<f64> = real_rhs.to_vec();

    // 列 -> その列を含む行番号の索引 (`a_rows` 用と `real_rows` 用)。
    // 列長ぶんのコストで該当行を探すためのもの。内容は上位集合
    // (係数の相殺や行削除で古くなり得る) なので、参照時に毎回
    // 実際の所属を確認し、ソート・重複除去してから使う。
    let mut a_col_idx: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, row) in a_rows.iter().enumerate() {
        for &(k, v) in row {
            if v != 0.0 {
                a_col_idx[k].push(i);
            }
        }
    }
    let mut g_col_idx: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, row) in real_rows.iter().enumerate() {
        for &(k, v) in row {
            if v != 0.0 {
                g_col_idx[k].push(i);
            }
        }
    }

    // ピボットとして消費済み (最終的に取り除く) の等式行。
    let mut row_deleted = vec![false; a_rows.len()];
    // 消去済みの列。
    let mut col_eliminated = vec![false; n];
    let mut substitutions = Vec::new();
    // fill-in 上限超過の連続回数。
    let mut consecutive_fillin_failures = 0usize;

    for (row_idx, j) in candidates {
        if row_deleted[row_idx] || col_eliminated[j] {
            continue;
        }
        let pivot_row = a_rows[row_idx].clone();
        let coeff = match pivot_row.iter().find(|&&(k, _)| k == j) {
            Some(&(_, v)) if v != 0.0 => v,
            _ => continue,
        };
        // 行の *現在の* 内容で再検証する (同じ呼び出し内の先行 fold で
        // この行が書き換わっている可能性があるため。健全性はこちらに依存する)。
        let activity = compute_row_activity(&pivot_row, lb, ub);
        if !row_implies_own_bound(&activity, j, coeff, b[row_idx], lb, ub) {
            continue;
        }
        // ピボット行の係数絶対値の最大値。
        let row_max = pivot_row.iter().map(|&(_, v)| v.abs()).fold(0.0f64, f64::max);
        if coeff.abs() < tunable!("ENOMOTO_T_SUBSTITUTION_PIVOT_RATIO", SUBSTITUTION_PIVOT_RATIO, f64) * row_max {
            // この (行, 列) 対だけが数値ガードで不合格。同じ列の別の行が
            // 後続の候補として残っている可能性がある (HiGHS と同様に対単位で処理)。
            continue;
        }
        // ピボット行から `j` を除いた残り項 (代入式 x_j = (rhs - Σ terms) / coeff の項)。
        let terms: Vec<(usize, f64)> = pivot_row.iter().filter(|&&(k, _)| k != j).copied().collect();
        if terms.is_empty() {
            // 行シングルトンが紛れ込んだ場合 (本来 rowsingleton が先に処理する)。
            continue;
        }

        // `j` をまだ含む他の等式行 (ピボット行と削除済み行を除く)。現在の状態から確認。
        let other_a: Vec<usize> = {
            let mut v = a_col_idx[j].clone();
            v.sort_unstable();
            v.dedup();
            v.retain(|&i2| i2 != row_idx && !row_deleted[i2] && a_rows[i2].iter().any(|&(k, v)| k == j && v != 0.0));
            v
        };
        // `j` を含む実不等式行。
        let other_g: Vec<usize> = {
            let mut v = g_col_idx[j].clone();
            v.sort_unstable();
            v.dedup();
            v.retain(|&i2| real_rows[i2].iter().any(|&(k, v)| k == j && v != 0.0));
            v
        };

        // 全対象行へ代入したときに増える非零数。
        let fillin = {
            let mut targets: Vec<&Vec<(usize, f64)>> = Vec::with_capacity(other_a.len() + other_g.len());
            for &i2 in &other_a {
                targets.push(&a_rows[i2]);
            }
            for &i2 in &other_g {
                targets.push(&real_rows[i2]);
            }
            fillin_cost(&terms, &targets)
        };
        if fillin > tunable!("ENOMOTO_T_MAX_FILLIN", MAX_FILLIN, usize) {
            consecutive_fillin_failures += 1;
            if consecutive_fillin_failures >= tunable!("ENOMOTO_T_MAX_CONSECUTIVE_FILLIN_FAILURES", MAX_CONSECUTIVE_FILLIN_FAILURES, usize) {
                break;
            }
            continue;
        }
        consecutive_fillin_failures = 0;

        // ピボット行の右辺。
        let rhs_i = b[row_idx];
        // 他の等式行へ代入: row_i2 -= (a_i2j / coeff) * pivot_row。
        for &i2 in &other_a {
            let a_i2j = a_rows[i2].iter().find(|&&(k, _)| k == j).unwrap().1;
            let factor = a_i2j / coeff;
            a_rows[i2] = axpy_row(&mut accum, &a_rows[i2], &pivot_row, factor, j, TOL);
            // fill-in はピボット行の残り列にしか生じないので、その列の索引に追記。
            for &(k, _) in &terms {
                a_col_idx[k].push(i2);
            }
            b[i2] -= factor * rhs_i;
        }
        // 実不等式行へ同様に代入。
        for &i2 in &other_g {
            let a_i2j = real_rows[i2].iter().find(|&&(k, _)| k == j).unwrap().1;
            let factor = a_i2j / coeff;
            real_rows[i2] = axpy_row(&mut accum, &real_rows[i2], &pivot_row, factor, j, TOL);
            for &(k, _) in &terms {
                g_col_idx[k].push(i2);
            }
            real_rhs[i2] -= factor * rhs_i;
        }

        // 目的関数へ代入: c_k -= (c_j / coeff) * a_k、c_j = 0。
        let cj = c[j];
        if cj != 0.0 {
            let factor = cj / coeff;
            for &(k, a_ik) in &terms {
                c[k] -= factor * a_ik;
            }
            c[j] = 0.0;
        }

        // 境界保存行は不要: `j` は implied-free なので復元値は箱の内側に入る。
        substitutions.push(Substitution { var: j, terms, rhs: rhs_i, coeff });
        col_eliminated[j] = true;
        row_deleted[row_idx] = true;
    }
    // 1 件も受理されなければ行は一切書き換わっていない (fold は受理時のみ) ので問題は不変。
    if substitutions.is_empty() {
        return None;
    }

    // 削除済み行を取り除いた最終的な `A`, `b`。
    let mut final_a_rows = Vec::with_capacity(a_rows.len());
    let mut final_b = Vec::with_capacity(b.len());
    for (i, row) in a_rows.into_iter().enumerate() {
        if !row_deleted[i] {
            final_a_rows.push(row);
            final_b.push(b[i]);
        }
    }

    Some(AggregatorResult {
        a: csr_from_rows(&final_a_rows, n),
        b: final_b,
        c,
        substitutions,
        real_rows,
        real_rhs,
    })
}

/// 行横断版 Aggregator ([`eliminate_implied_free_columns`] の一般化)。
/// 列の含意範囲を、その列を含む全ての生きている等式行の [`implied_range`] の
/// *共通部分* とし、1 本では足りないが複数本合わせれば箱が冗長になる列も消去する。
/// 既定パイプラインでは使わず、`presolve.rs` の `ENOMOTO_XROW_AGGREGATOR` で選択する。
///
/// 健全性の要点: 候補の根拠 (含意範囲) は、消去を確定する直前に
/// 「削除されていない行・その現在の内容」だけから必ず再計算する。
/// 根拠だった行が先行する別列の消去のピボットとして消費 (削除) されている場合、
/// スナップショットの根拠を信用すると箱を不正に落としてしまう (Netlib `shell` の
/// 偽 `Unbounded` の原因)。候補生成時のスナップショットは順序付けにのみ使う。
///
/// 引数は [`eliminate_implied_free_columns`] と同じ。常に結果を返す (無変更でもコピー)。
pub fn eliminate_implied_free_columns_xrow(n: usize, a: &FaerCsr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64]) -> AggregatorResult {
    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);
    // 全ての行 fold で共有する疎アキュムレータ。
    let mut accum = SparseAccum::new(n);
    let mut b: Vec<f64> = b.to_vec();
    let mut c: Vec<f64> = c.to_vec();
    let mut real_rows: Vec<Vec<(usize, f64)>> = real_rows.to_vec();
    let mut real_rhs: Vec<f64> = real_rhs.to_vec();

    // 各列が `A` の何本の行に (非零で) 現れるか。
    let mut col_a_count = vec![0usize; n];
    for row in &a_rows {
        for &(j, v) in row {
            if v != 0.0 {
                col_a_count[j] += 1;
            }
        }
    }
    let is_free = |j: usize| lb[j] == f64::NEG_INFINITY && ub[j] == f64::INFINITY;

    // 行ごとの `RowActivity` のキャッシュ (遅延計算)。`None` は未計算または無効。
    // 行が fold で書き換わった瞬間に `None` に戻すので、ヒットは常に現在の内容を反映する。
    // 長い行で O(行長^2) になるのを防ぐために必須。
    let mut row_activity: Vec<Option<RowActivity>> = vec![None; a_rows.len()];

    /// `rows` のうち (現在の内容で) 列 `j` の非零項を持つ行の含意範囲を
    /// すべて交差させた範囲を返す。該当行が 1 本も無ければ `None`。
    /// 行の内容は毎回 `a_rows` から読み、活動量要約だけ `row_activity` にキャッシュする。
    fn aggregate_range(j: usize, rows: &[usize], a_rows: &[Vec<(usize, f64)>], b: &[f64], lb: &[f64], ub: &[f64], row_activity: &mut [Option<RowActivity>]) -> Option<(f64, f64)> {
        let mut lo = f64::NEG_INFINITY;
        let mut hi = f64::INFINITY;
        // 1 本でも該当行があったか。
        let mut any = false;
        for &i in rows {
            let row = &a_rows[i];
            let Some(&(_, coeff)) = row.iter().find(|&&(k, _)| k == j) else { continue };
            if coeff == 0.0 {
                continue;
            }
            let activity = *row_activity[i].get_or_insert_with(|| compute_row_activity(row, lb, ub));
            let (rlo, rhi) = implied_range(&activity, j, coeff, b[i], lb, ub);
            lo = lo.max(rlo);
            hi = hi.min(rhi);
            any = true;
        }
        any.then_some((lo, hi))
    }

    // 入力時点のスナップショット: 列 -> その列を含む等式行。候補の選別と順序付けにのみ使い、
    // 消去時の判定には使わない。
    let mut col_rows: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, row) in a_rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 {
                col_rows[j].push(i);
            }
        }
    }

    // 候補列: 等式行 2 本以上に現れ、自由列でなく、スナップショット上の共通含意範囲が箱に収まる列。
    let mut candidates: Vec<usize> = Vec::new();
    for j in 0..n {
        if col_a_count[j] < 2 || is_free(j) {
            continue;
        }
        if let Some((lo, hi)) = aggregate_range(j, &col_rows[j], &a_rows, &b, lb, ub, &mut row_activity) {
            if range_within_box(j, lo, hi, lb, ub) {
                candidates.push(j);
            }
        }
    }
    // 安い順: 行局所版と同じ fill-in 目安を、その列を含む最短行で見積もる。
    candidates.sort_by_key(|&j| {
        let collen = col_a_count[j];
        let min_rowlen = col_rows[j].iter().map(|&i| a_rows[i].len()).min().unwrap_or(0);
        (min_rowlen.min(collen) != 2, min_rowlen * collen, min_rowlen.min(collen), j)
    });

    // ピボットとして消費済みの等式行。
    let mut row_deleted = vec![false; a_rows.len()];
    // 消去済みの列。
    let mut col_eliminated = vec![false; n];
    let mut substitutions = Vec::new();
    // fill-in 上限超過の連続回数。
    let mut consecutive_fillin_failures = 0usize;

    for j in candidates {
        if col_eliminated[j] {
            continue;
        }
        // 現在生きていて `j` を含む全等式行を毎回走査し直す (スナップショットは信用しない)。
        // fold で新たに `j` を含むようになった行を取りこぼしても範囲が広がるだけで安全側。
        let live_rows: Vec<usize> = a_rows
            .iter()
            .enumerate()
            .filter(|&(i, row)| !row_deleted[i] && row.iter().any(|&(k, v)| k == j && v != 0.0))
            .map(|(i, _)| i)
            .collect();
        let Some((lo, hi)) = aggregate_range(j, &live_rows, &a_rows, &b, lb, ub, &mut row_activity) else {
            continue;
        };
        if !range_within_box(j, lo, hi, lb, ub) {
            continue;
        }

        // ピボット行: 生きている行のうち最短で、ピボット比ガードを通る最初の行
        // (共通範囲で根拠は確認済みなので、どの行を使っても代数的に正しい)。
        let mut sorted_live = live_rows.clone();
        sorted_live.sort_by_key(|&i| a_rows[i].len());
        // 選んだ (ピボット行番号, `j` の係数)。
        let mut chosen: Option<(usize, f64)> = None;
        for &i in &sorted_live {
            let row = &a_rows[i];
            let Some(&(_, coeff)) = row.iter().find(|&&(k, _)| k == j) else { continue };
            let row_max = row.iter().map(|&(_, v)| v.abs()).fold(0.0f64, f64::max);
            if coeff.abs() >= tunable!("ENOMOTO_T_SUBSTITUTION_PIVOT_RATIO", SUBSTITUTION_PIVOT_RATIO, f64) * row_max {
                chosen = Some((i, coeff));
                break;
            }
        }
        let Some((row_idx, coeff)) = chosen else {
            continue;
        };

        let pivot_row = a_rows[row_idx].clone();
        // ピボット行から `j` を除いた残り項。
        let terms: Vec<(usize, f64)> = pivot_row.iter().filter(|&&(k, _)| k != j).copied().collect();
        if terms.is_empty() {
            continue;
        }

        // `j` を含む他の生きている等式行と、`j` を含む実不等式行。
        let other_a: Vec<usize> = a_rows
            .iter()
            .enumerate()
            .filter(|&(i2, row2)| i2 != row_idx && !row_deleted[i2] && row2.iter().any(|&(k, v)| k == j && v != 0.0))
            .map(|(i2, _)| i2)
            .collect();
        let other_g: Vec<usize> = real_rows.iter().enumerate().filter(|(_, row2)| row2.iter().any(|&(k, v)| k == j && v != 0.0)).map(|(i2, _)| i2).collect();

        // 全対象行へ代入したときに増える非零数。
        let fillin = {
            let mut targets: Vec<&Vec<(usize, f64)>> = Vec::with_capacity(other_a.len() + other_g.len());
            for &i2 in &other_a {
                targets.push(&a_rows[i2]);
            }
            for &i2 in &other_g {
                targets.push(&real_rows[i2]);
            }
            fillin_cost(&terms, &targets)
        };
        if fillin > tunable!("ENOMOTO_T_MAX_FILLIN", MAX_FILLIN, usize) {
            consecutive_fillin_failures += 1;
            if consecutive_fillin_failures >= tunable!("ENOMOTO_T_MAX_CONSECUTIVE_FILLIN_FAILURES", MAX_CONSECUTIVE_FILLIN_FAILURES, usize) {
                break;
            }
            continue;
        }
        consecutive_fillin_failures = 0;

        // ピボット行の右辺。
        let rhs_i = b[row_idx];
        for &i2 in &other_a {
            let a_i2j = a_rows[i2].iter().find(|&&(k, _)| k == j).unwrap().1;
            let factor = a_i2j / coeff;
            a_rows[i2] = axpy_row(&mut accum, &a_rows[i2], &pivot_row, factor, j, TOL);
            b[i2] -= factor * rhs_i;
            // 行の内容が変わったので活動量キャッシュを無効化。
            row_activity[i2] = None;
        }
        for &i2 in &other_g {
            let a_i2j = real_rows[i2].iter().find(|&&(k, _)| k == j).unwrap().1;
            let factor = a_i2j / coeff;
            real_rows[i2] = axpy_row(&mut accum, &real_rows[i2], &pivot_row, factor, j, TOL);
            real_rhs[i2] -= factor * rhs_i;
        }

        // 目的関数へ代入。
        let cj = c[j];
        if cj != 0.0 {
            let factor = cj / coeff;
            for &(k, a_ik) in &terms {
                c[k] -= factor * a_ik;
            }
            c[j] = 0.0;
        }

        substitutions.push(Substitution { var: j, terms, rhs: rhs_i, coeff });
        col_eliminated[j] = true;
        row_deleted[row_idx] = true;
    }

    // 削除済み行を取り除いた最終的な `A`, `b`。
    let mut final_a_rows = Vec::with_capacity(a_rows.len());
    let mut final_b = Vec::with_capacity(b.len());
    for (i, row) in a_rows.into_iter().enumerate() {
        if !row_deleted[i] {
            final_a_rows.push(row);
            final_b.push(b[i]);
        }
    }

    AggregatorResult {
        a: csr_from_rows(&final_a_rows, n),
        b: final_b,
        c,
        substitutions,
        real_rows,
        real_rhs,
    }
}

/// [`eliminate_implied_free_columns_v2`] で [`eliminate_implied_free_columns_xrow`] に
/// 追加した各拡張の ON/OFF。各項目は独立に切り替えられる
/// (既定はすべて ON、[`AggOptions::from_env`] の `ENOMOTO_AGG_*` で個別に OFF)。
#[derive(Clone, Copy, Debug)]
pub struct AggOptions {
    /// `G` の実不等式行から得られる片側の含意境界も列の含意範囲に交差させるか
    /// (HiGHS の `isImpliedFree` は等式に限らず任意の行の最も厳しい含意境界を使う)。
    pub use_ineq: bool,
    /// 候補列が現れなければならない等式行の最小本数。2 は旧来の条件、
    /// 1 は「等式行 1 本 + 不等式行 1 本以上」の列も許可する
    /// (この形は `colsingleton` も扱えない)。
    pub min_a_count: usize,
    /// HiGHS 流の正味 fill-in (`新規非零 - (行長 + 列長 - 1)`) を使い、
    /// ピボット行長か列長が 2 のときは fill-in 判定自体を省くか。false なら総 fill-in。
    pub net_fillin: bool,
    /// fill-in 超過が `MAX_CONSECUTIVE_FILLIN_FAILURES` 回連続したら残り候補を打ち切るか。
    pub fillin_break: bool,
}

impl AggOptions {
    /// 環境変数から設定を作る。既定は全拡張 ON (`min_a_count = 1`)。
    /// `ENOMOTO_AGG_NOINEQ` で `use_ineq` OFF、`ENOMOTO_AGG_MINACNT2` で `min_a_count = 2`、
    /// `ENOMOTO_AGG_GROSSFILL` で総 fill-in、`ENOMOTO_AGG_NOBREAK` で打ち切りなし。
    pub fn from_env() -> Self {
        AggOptions {
            use_ineq: env_str!("ENOMOTO_AGG_NOINEQ").is_none(),
            min_a_count: if env_str!("ENOMOTO_AGG_MINACNT2").is_some() { 2 } else { 1 },
            net_fillin: env_str!("ENOMOTO_AGG_GROSSFILL").is_none(),
            fillin_break: env_str!("ENOMOTO_AGG_NOBREAK").is_none(),
        }
    }
}

/// [`eliminate_implied_free_columns_v2`] の前段付き版 (既定の Aggregator 入口)。
/// まず [`v2_has_candidate`] で候補の有無だけを安価に調べ、候補が無ければ
/// 何もコピーせず `None` (問題は不変) を返す。候補があれば v2 の結果を `Some` で返す。
/// 前段判定は v2 の候補判定と同じ浮動小数点演算順序で行うので、
/// `Some` の結果は v2 を直接呼んだ場合と同一で、`None` は v2 が何も消去しない場合に限る。
pub fn eliminate_implied_free_columns_v2_if_any(n: usize, a: &FaerCsr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], opts: AggOptions) -> Option<AggregatorResult> {
    if !v2_has_candidate(n, a, b, lb, ub, real_rows, real_rhs, opts) {
        return None;
    }
    Some(eliminate_implied_free_columns_v2(n, a, b, c, lb, ub, real_rows, real_rhs, opts))
}

/// [`eliminate_implied_free_columns_v2`] の候補リストが空でないかを判定する。
/// v2 の候補生成を正確に再現する: 同じ適格性判定、列番号順にソートした行での
/// 活動量計算 (v2 は最初に未ソート行をソートするため)、列ごとの `max`/`min` の
/// 畳み込み順 (等式行を昇順 → 不等式行を昇順)。行列のコピーは作らない。
pub fn v2_has_candidate(n: usize, a: &FaerCsr, b: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], opts: AggOptions) -> bool {
    let ar = a.as_ref();
    // 等式行の本数。
    let p = ar.nrows();
    let is_free = |j: usize| lb[j] == f64::NEG_INFINITY && ub[j] == f64::INFINITY;
    // 各列が現れる等式行 / 実不等式行の本数。
    let mut col_a_count = vec![0usize; n];
    let mut col_g_count = vec![0usize; n];
    for i in 0..p {
        for (&j, &v) in ar.col_indices_of_row_raw(i).iter().zip(ar.values_of_row(i)) {
            if v != 0.0 {
                col_a_count[j] += 1;
            }
        }
    }
    for row in real_rows {
        for &(j, v) in row {
            if v != 0.0 {
                col_g_count[j] += 1;
            }
        }
    }
    // 等式行本数の下限 (最低 1)。
    let min_a = opts.min_a_count.max(1);
    // 列ごとの適格性 (v2 の候補条件のうち範囲判定以外の部分)。
    let eligible: Vec<bool> = (0..n).map(|j| !(col_a_count[j] < min_a || col_a_count[j] + col_g_count[j] < 2 || is_free(j))).collect();
    if !eligible.iter().any(|&e| e) {
        return false;
    }
    // 列ごとの含意範囲 (全行の交差)。
    let mut lo = vec![f64::NEG_INFINITY; n];
    let mut hi = vec![f64::INFINITY; n];
    // ソート済みコピー用の再利用バッファ。
    let mut row_buf: Vec<(usize, f64)> = Vec::new();
    /// `row` が既に列番号昇順ならそのまま、そうでなければ `buf` にソート済みコピーを作って返す
    /// (v2 は活動量計算の前に行をソートするので、同じ加算順序にするため)。
    fn sorted_by_col<'r>(row: &'r [(usize, f64)], buf: &'r mut Vec<(usize, f64)>) -> &'r [(usize, f64)] {
        if row.windows(2).all(|w| w[0].0 < w[1].0) {
            row
        } else {
            buf.clear();
            buf.extend_from_slice(row);
            buf.sort_unstable_by_key(|&(k, _)| k);
            buf
        }
    }
    // CSR 行を (列, 係数) 列に詰め直す再利用バッファ。
    let mut a_buf: Vec<(usize, f64)> = Vec::new();
    for i in 0..p {
        let cols = ar.col_indices_of_row_raw(i);
        let vals = ar.values_of_row(i);
        // 適格列を 1 つも含まない行は飛ばす。
        if !cols.iter().zip(vals).any(|(&j, &v)| v != 0.0 && eligible[j]) {
            continue;
        }
        a_buf.clear();
        a_buf.extend(cols.iter().copied().zip(vals.iter().copied()));
        let row = sorted_by_col(&a_buf, &mut row_buf);
        let act = compute_row_activity(row, lb, ub);
        for &(j, coeff) in row {
            if coeff == 0.0 || !eligible[j] {
                continue;
            }
            let (rlo, rhi) = implied_range(&act, j, coeff, b[i], lb, ub);
            lo[j] = lo[j].max(rlo);
            hi[j] = hi[j].min(rhi);
        }
    }
    if opts.use_ineq {
        for (i, row) in real_rows.iter().enumerate() {
            if !row.iter().any(|&(j, v)| v != 0.0 && eligible[j]) {
                continue;
            }
            let row = sorted_by_col(row, &mut row_buf);
            let act = compute_row_activity(row, lb, ub);
            for &(j, coeff) in row {
                if coeff == 0.0 || !eligible[j] {
                    continue;
                }
                // `coeff*x_j <= rhs - s_lo` から片側の境界を得る。
                let (s_lo, _) = residual_range(&act, j, coeff, lb, ub);
                if s_lo.is_finite() {
                    let v = (real_rhs[i] - s_lo) / coeff;
                    if coeff > 0.0 {
                        hi[j] = hi[j].min(v);
                    } else {
                        lo[j] = lo[j].max(v);
                    }
                }
            }
        }
    }
    (0..n).any(|j| eligible[j] && range_within_box(j, lo[j], hi[j], lb, ub))
}

/// 既定版 Aggregator (v2)。行横断版の「消去直前に生きている行・現在の内容から
/// 含意範囲を再計算する」方式を引き継ぎ、[`AggOptions`] の拡張を加えたもの。
///
/// 不等式行を根拠に使う場合の健全性: `G` の行は削除されず fold されるだけなので、
/// `x_j` の境界を含意した行は代入後も強制され続け、その行が使う他の列の箱も残る
/// (先に消去された列はもう生きている行に現れないので、根拠の依存関係は循環しない)。
///
/// 引数は [`eliminate_implied_free_columns`] と同じ + `opts`。常に結果を返す。
pub fn eliminate_implied_free_columns_v2(n: usize, a: &FaerCsr, b: &[f64], c: &[f64], lb: &[f64], ub: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], opts: AggOptions) -> AggregatorResult {
    let mut a_rows: Vec<Vec<(usize, f64)>> = csr_rows(a);
    // 全ての行 fold で共有する疎アキュムレータ。
    let mut accum = SparseAccum::new(n);
    let mut b: Vec<f64> = b.to_vec();
    let mut c: Vec<f64> = c.to_vec();
    let mut real_rows: Vec<Vec<(usize, f64)>> = real_rows.to_vec();
    let mut real_rhs: Vec<f64> = real_rhs.to_vec();
    let is_free = |j: usize| lb[j] == f64::NEG_INFINITY && ub[j] == f64::INFINITY;
    // 係数参照を二分探索にするため、全行を列番号順にソートしておく (密な行で二乗時間になるのを防ぐ)。
    for row in a_rows.iter_mut().chain(real_rows.iter_mut()) {
        if !row.windows(2).all(|w| w[0].0 < w[1].0) {
            row.sort_unstable_by_key(|&(k, _)| k);
        }
    }
    // ソート済み行 `row` における列 `j` の係数 (無ければ 0)。
    let coef_of = |row: &[(usize, f64)], j: usize| match row.binary_search_by_key(&j, |&(k, _)| k) {
        Ok(p) => row[p].1,
        Err(_) => 0.0,
    };

    // 先に列ごとの出現本数を数え、下の列別リストを最終サイズで一度に確保する。
    let mut col_a_count = vec![0usize; n];
    let mut col_g_count = vec![0usize; n];
    for row in &a_rows {
        for &(j, v) in row {
            if v != 0.0 {
                col_a_count[j] += 1;
            }
        }
    }
    for row in &real_rows {
        for &(j, v) in row {
            if v != 0.0 {
                col_g_count[j] += 1;
            }
        }
    }
    // 列 -> その列を含む行番号の索引 (等式行用 / 実不等式行用)。fold で追記されるので上位集合。
    let mut a_col_idx: Vec<Vec<usize>> = col_a_count.iter().map(|&k| Vec::with_capacity(k)).collect();
    let mut g_col_idx: Vec<Vec<usize>> = col_g_count.iter().map(|&k| Vec::with_capacity(k)).collect();
    // 初期状態の列別 `(行, 係数)` リスト (候補生成専用。その時点では全行が未加工)。
    // 列圧縮形式で、列 `j` の分は `a_col0[a_col0_ptr[j]..a_col0_ptr[j + 1]]`。
    // `col_ptr` は列ごとの本数から累積和ポインタ (長さ n+1) を作る。
    let col_ptr = |count: &[usize]| {
        let mut ptr = Vec::with_capacity(n + 1);
        let mut acc = 0usize;
        ptr.push(0);
        for &k in count {
            acc += k;
            ptr.push(acc);
        }
        ptr
    };
    let a_col0_ptr = col_ptr(&col_a_count);
    let g_col0_ptr = col_ptr(&col_g_count);
    let mut a_col0: Vec<(usize, f64)> = vec![(0, 0.0); a_col0_ptr[n]];
    let mut g_col0: Vec<(usize, f64)> = vec![(0, 0.0); g_col0_ptr[n]];
    // `a_col_idx[j]` / `g_col_idx[j]` が順序の乱れや重複を含み得る (= 使用前に
    // ソート・重複除去が必要) ことを示すフラグ。行順に構築した直後は昇順で、
    // 重複は同一列を重複して持つ行でのみ生じる。fold で追記すると dirty になる。
    // クリーンな列ではソート・重複除去は無操作なので省略しても結果は同じ。
    let mut a_dirty = vec![false; n];
    let mut g_dirty = vec![false; n];
    for (i, row) in a_rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 {
                if a_col_idx[j].last() == Some(&i) {
                    a_dirty[j] = true;
                }
                a_col0[a_col0_ptr[j] + a_col_idx[j].len()] = (i, v);
                a_col_idx[j].push(i);
            }
        }
    }
    for (i, row) in real_rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 {
                if g_col_idx[j].last() == Some(&i) {
                    g_dirty[j] = true;
                }
                g_col0[g_col0_ptr[j] + g_col_idx[j].len()] = (i, v);
                g_col_idx[j].push(i);
            }
        }
    }
    // 等式行ごとの係数絶対値最大値のキャッシュ (fold で `None` に戻す)。
    let mut row_max_a: Vec<Option<f64>> = vec![None; a_rows.len()];
    // fill-in 計数用のスタンプ配列: `stamp[k] == stamp_id` なら列 `k` は現在の対象行に既存。
    let mut stamp = vec![usize::MAX; n];
    let mut stamp_id = 0usize;
    // 等式行 / 実不等式行ごとの `RowActivity` キャッシュ (fold で `None` に戻す)。
    let mut act_a: Vec<Option<RowActivity>> = vec![None; a_rows.len()];
    let mut act_g: Vec<Option<RowActivity>> = vec![None; real_rows.len()];
    // ピボットとして消費済みの等式行。
    let mut row_deleted = vec![false; a_rows.len()];

    // 列 `j` の含意範囲: 等式行 `la` の含意範囲と、`use_ineq` なら不等式行 `lg` の
    // 片側境界をすべて交差させる (`la`/`lg` は `(行番号, j の係数)` の列)。
    let col_implied_range = |j: usize, la: &[(usize, f64)], lg: &[(usize, f64)], a_rows: &[Vec<(usize, f64)>], b: &[f64], real_rows: &[Vec<(usize, f64)>], real_rhs: &[f64], act_a: &mut [Option<RowActivity>], act_g: &mut [Option<RowActivity>]| -> (f64, f64) {
        let mut lo = f64::NEG_INFINITY;
        let mut hi = f64::INFINITY;
        for &(i, coeff) in la {
            let act = *act_a[i].get_or_insert_with(|| compute_row_activity(&a_rows[i], lb, ub));
            let (rlo, rhi) = implied_range(&act, j, coeff, b[i], lb, ub);
            lo = lo.max(rlo);
            hi = hi.min(rhi);
        }
        if opts.use_ineq {
            for &(i, coeff) in lg {
                let act = *act_g[i].get_or_insert_with(|| compute_row_activity(&real_rows[i], lb, ub));
                // `coeff*x_j <= rhs - s_lo` から片側の境界を得る。
                let (s_lo, _) = residual_range(&act, j, coeff, lb, ub);
                if s_lo.is_finite() {
                    let v = (real_rhs[i] - s_lo) / coeff;
                    if coeff > 0.0 {
                        hi = hi.min(v);
                    } else {
                        lo = lo.max(v);
                    }
                }
            }
        }
        (lo, hi)
    };

    // 等式行本数の下限 (最低 1)。
    let min_a = opts.min_a_count.max(1);
    // 候補列: 適格 (等式行 min_a 本以上・全出現 2 本以上・非自由) で、初期状態の含意範囲が箱に収まる列。
    let mut candidates: Vec<usize> = Vec::new();
    for j in 0..n {
        if col_a_count[j] < min_a || col_a_count[j] + col_g_count[j] < 2 || is_free(j) {
            continue;
        }
        let (la, lg) = (&a_col0[a_col0_ptr[j]..a_col0_ptr[j + 1]], &g_col0[g_col0_ptr[j]..g_col0_ptr[j + 1]]);
        let (lo, hi) = col_implied_range(j, la, lg, &a_rows, &b, &real_rows, &real_rhs, &mut act_a, &mut act_g);
        if range_within_box(j, lo, hi, lb, ub) {
            candidates.push(j);
        }
    }
    // 安い順: 長さ 2 の対を先頭に、次に `最短等式行長 * 列長` 昇順。
    candidates.sort_by_key(|&j| {
        let collen = col_a_count[j] + col_g_count[j];
        let min_rowlen = a_col_idx[j].iter().map(|&i| a_rows[i].len()).min().unwrap_or(0);
        (min_rowlen.min(collen) != 2, min_rowlen * collen, min_rowlen.min(collen), j)
    });

    let mut substitutions = Vec::new();
    // fill-in 上限超過の連続回数。
    let mut consecutive_fillin_failures = 0usize;
    for j in candidates {
        // `j` を含む生きている行を、現在の内容で確認し直して集める。
        if a_dirty[j] {
            a_col_idx[j].sort_unstable();
            a_col_idx[j].dedup();
            a_dirty[j] = false;
        }
        // `la`: `j` を含む生きている等式行の `(行, j の係数)`。
        let la: Vec<(usize, f64)> = crate::sparse::collect_with_capacity(a_col_idx[j].len(), a_col_idx[j].iter().filter(|&&i| !row_deleted[i]).map(|&i| (i, coef_of(&a_rows[i], j))).filter(|&(_, v)| v != 0.0));
        if la.is_empty() {
            continue;
        }
        if g_dirty[j] {
            g_col_idx[j].sort_unstable();
            g_col_idx[j].dedup();
            g_dirty[j] = false;
        }
        // `lg`: `j` を含む実不等式行の `(行, j の係数)`。
        let lg: Vec<(usize, f64)> = crate::sparse::collect_with_capacity(g_col_idx[j].len(), g_col_idx[j].iter().map(|&i| (i, coef_of(&real_rows[i], j))).filter(|&(_, v)| v != 0.0));
        let (lo, hi) = col_implied_range(j, &la, &lg, &a_rows, &b, &real_rows, &real_rhs, &mut act_a, &mut act_g);
        if !range_within_box(j, lo, hi, lb, ub) {
            continue;
        }
        // ピボット: ピボット比ガードを通る最短の生きている等式行。
        let mut by_len = la.clone();
        by_len.sort_by_key(|&(i, _)| a_rows[i].len());
        let Some((row_idx, coeff)) = by_len.into_iter().find(|&(i, coeff)| {
            let row_max = *row_max_a[i].get_or_insert_with(|| a_rows[i].iter().map(|&(_, v)| v.abs()).fold(0.0f64, f64::max));
            coeff.abs() >= tunable!("ENOMOTO_T_SUBSTITUTION_PIVOT_RATIO", SUBSTITUTION_PIVOT_RATIO, f64) * row_max
        }) else {
            continue;
        };
        let pivot_row = a_rows[row_idx].clone();
        // ピボット行から `j` を除いた残り項。
        let terms: Vec<(usize, f64)> = crate::sparse::collect_with_capacity(pivot_row.len(), pivot_row.iter().filter(|&&(k, _)| k != j).copied());
        if terms.is_empty() {
            continue;
        }
        // 代入先: ピボット以外の等式行と、全ての該当実不等式行。
        let other_a: Vec<usize> = crate::sparse::collect_with_capacity(la.len(), la.iter().map(|&(i, _)| i).filter(|&i| i != row_idx));
        let other_g: Vec<usize> = lg.iter().map(|&(i, _)| i).collect();
        let n_other = other_a.len() + other_g.len();
        // ピボット行長か列長 (= 代入先本数 + 1) が 2 か。正味 fill-in モードでは判定を省く。
        let is_size2_pair = pivot_row.len() == 2 || n_other + 1 == 2;
        if !(opts.net_fillin && is_size2_pair) {
            // 代入で新たに生じる非零数 (総 fill-in) をスタンプ配列で数える。
            let mut gross = 0i64;
            for target in other_a.iter().map(|&i| &a_rows[i]).chain(other_g.iter().map(|&i| &real_rows[i])) {
                stamp_id += 1;
                for &(k, _) in target.iter() {
                    stamp[k] = stamp_id;
                }
                gross += terms.iter().filter(|&&(k, _)| stamp[k] != stamp_id).count() as i64;
            }
            // 正味 fill-in = 総 fill-in - 消える非零 (ピボット行 + 代入先の `j` 項)。
            let fillin = if opts.net_fillin { gross - (pivot_row.len() + n_other) as i64 } else { gross };
            if fillin > tunable!("ENOMOTO_T_MAX_FILLIN", MAX_FILLIN, usize) as i64 {
                consecutive_fillin_failures += 1;
                if opts.fillin_break && consecutive_fillin_failures >= tunable!("ENOMOTO_T_MAX_CONSECUTIVE_FILLIN_FAILURES", MAX_CONSECUTIVE_FILLIN_FAILURES, usize) {
                    break;
                }
                continue;
            }
        }
        consecutive_fillin_failures = 0;
        // ピボット行の右辺。
        let rhs_i = b[row_idx];
        for &i2 in &other_a {
            let factor = coef_of(&a_rows[i2], j) / coeff;
            a_rows[i2] = axpy_row(&mut accum, &a_rows[i2], &pivot_row, factor, j, TOL);
            b[i2] -= factor * rhs_i;
            // 内容が変わったのでキャッシュを無効化し、新たに現れ得る列の索引へ追記。
            act_a[i2] = None;
            row_max_a[i2] = None;
            for &(k, _) in &terms {
                a_col_idx[k].push(i2);
                a_dirty[k] = true;
            }
        }
        for &i2 in &other_g {
            let factor = coef_of(&real_rows[i2], j) / coeff;
            real_rows[i2] = axpy_row(&mut accum, &real_rows[i2], &pivot_row, factor, j, TOL);
            real_rhs[i2] -= factor * rhs_i;
            act_g[i2] = None;
            for &(k, _) in &terms {
                g_col_idx[k].push(i2);
                g_dirty[k] = true;
            }
        }
        // 目的関数へ代入。
        let cj = c[j];
        if cj != 0.0 {
            let factor = cj / coeff;
            for &(k, a_ik) in &terms {
                c[k] -= factor * a_ik;
            }
            c[j] = 0.0;
        }
        substitutions.push(Substitution { var: j, terms, rhs: rhs_i, coeff });
        row_deleted[row_idx] = true;
    }

    // 削除済み行を取り除いた最終的な `A`, `b`。
    let mut final_a_rows = Vec::with_capacity(a_rows.len());
    let mut final_b = Vec::with_capacity(b.len());
    for (i, row) in a_rows.into_iter().enumerate() {
        if !row_deleted[i] {
            final_a_rows.push(row);
            final_b.push(b[i]);
        }
    }
    AggregatorResult { a: csr_from_rows(&final_a_rows, n), b: final_b, c, substitutions, real_rows, real_rhs }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 行リストから CSR を作るテスト用ヘルパ。
    fn csr(rows: &[Vec<(usize, f64)>], n: usize) -> FaerCsr {
        csr_from_rows(rows, n)
    }

    /// implied-free 列が無い問題では何も変えないことを確認する。
    #[test]
    fn no_implied_free_columns_is_a_no_op() {
        // x0 は自由 (freevar の担当)。x1/x2 は有界だが各 1 行にしか現れない
        // (colsingleton の担当)。そもそも row0 単独では x0 が自由なので x1 の箱は冗長にならない。
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![3.0, 1.0, 1.0];
        let lb = vec![f64::NEG_INFINITY, 0.0, 0.0];
        let ub = vec![f64::INFINITY, 10.0, 10.0];
        let r = eliminate_implied_free_columns(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(r.substitutions.is_empty());
        assert_eq!(r.b, b);
        assert_eq!(r.c, c);
    }

    /// `compute_row_activity` が手計算と一致することを確認する。
    #[test]
    fn row_activity_matches_hand_derivation() {
        let row = vec![(0, 1.0), (1, -1.0)];
        let lb = vec![0.0, -5.0];
        let ub = vec![10.0, 5.0];
        let activity = compute_row_activity(&row, &lb, &ub);
        // 項0 (係数 +1, [0,10]): 下側 0, 上側 10。
        // 項1 (係数 -1, [-5,5]): 下側は ub=5 を使い -5、上側は lb=-5 を使い +5。
        // 合計: 下側 = -5, 上側 = 15。
        assert_eq!(activity.lo_finite_sum, -5.0);
        assert_eq!(activity.lo_inf_count, 0);
        assert_eq!(activity.hi_finite_sum, 15.0);
        assert_eq!(activity.hi_inf_count, 0);
    }

    /// `residual_range` が指定した列の項だけを除くことを確認する。
    #[test]
    fn residual_range_excludes_only_the_named_columns_own_term() {
        // 行 [x0 + x1]、x0 ∈ [0,10] (有限)、x1 ∈ [-inf,inf] (両側で唯一の無限大項)。
        let row = vec![(0, 1.0), (1, 1.0)];
        let lb = vec![0.0, f64::NEG_INFINITY];
        let ub = vec![10.0, f64::INFINITY];
        let activity = compute_row_activity(&row, &lb, &ub);
        assert_eq!(activity.lo_inf_count, 1);
        assert_eq!(activity.hi_inf_count, 1);
        // col0 を除いても col1 の無限大は残る。
        assert_eq!(residual_range(&activity, 0, 1.0, &lb, &ub), (f64::NEG_INFINITY, f64::INFINITY));
        // 唯一の無限大項 col1 を除くと col0 の範囲 [0,10] だけが残る。
        assert_eq!(residual_range(&activity, 1, 1.0, &lb, &ub), (0.0, 10.0));
    }

    /// 2 行に現れる implied-free 列が境界行なしで消去されることを確認する。
    #[test]
    fn eliminates_an_implied_free_column_appearing_in_two_rows() {
        // x0 ∈ [0,10]、x1 ∈ [-5,5] なので row0 単独で x0 = 5-x1 ∈ [0,10] となり x0 は implied-free。
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![3.0, 1.0, 1.0];
        let lb = vec![0.0, -5.0, f64::NEG_INFINITY];
        let ub = vec![10.0, 5.0, f64::INFINITY];
        let r = eliminate_implied_free_columns(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert_eq!(r.substitutions.len(), 1);
        assert_eq!(r.substitutions[0].var, 0);
        assert_eq!(r.a.nrows(), 1);
        // row1 (x0 - x2 = 1) は -x1 - x2 = -4 に fold される (ピボットは row0)。
        assert_eq!(r.c[1], 1.0 - 3.0);
        assert_eq!(r.c[2], 1.0);
        // x1=2, x2=1 のとき x0 = 5 - 2 = 3 が復元される。
        let x = vec![3.0, 2.0, 1.0];
        assert!((r.substitutions[0].value(&x) - 3.0).abs() < 1e-9);
    }

    /// implied-free でない列は消去しないことを確認する。
    #[test]
    fn non_implied_free_column_is_left_alone() {
        // 同じ形だが x1 ∈ [0,100] と緩いので row0 単独では x0 ∈ [-95,5] で [0,10] に収まらない。
        // 他の行も助けにならないので x0 は消去してはならない。
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![3.0, 1.0, 1.0];
        let lb = vec![0.0, 0.0, f64::NEG_INFINITY];
        let ub = vec![10.0, 100.0, f64::INFINITY];
        let r = eliminate_implied_free_columns(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(r.substitutions.is_empty());
        assert_eq!(r.a.nrows(), 2);
    }

    /// 行局所版では、どの行も単独で箱を示せない列は消去されないことを確認する。
    #[test]
    fn neither_row_alone_proving_it_leaves_the_column_un_eliminated() {
        // x0 ∈ [0,10]。row0 (x0+x1=5, x1 ∈ [-100,100]) 単独では x0 ∈ [-95,105]、
        // row1 (x0-x2=1, x2 ∈ [-100,-4]) 単独では x0 ∈ [-99,-3]。どちらも [0,10] に収まらない。
        // 行局所版は 1 行ずつしか見ないので消去しない。
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![0.0, 0.0, 0.0];
        let lb = vec![0.0, -100.0, -100.0];
        let ub = vec![10.0, 100.0, -4.0];
        let r = eliminate_implied_free_columns(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(r.substitutions.is_empty());
    }

    /// 消去列を含む実不等式行にも代入が反映されることを確認する。
    #[test]
    fn fold_reaches_a_shared_real_row_too() {
        // x0 ∈ [0,10], x1 ∈ [-5,5] (row0 で implied-free)。実不等式行 1 本も x0 を含む。
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![0.0, 0.0, 0.0];
        let lb = vec![0.0, -5.0, f64::NEG_INFINITY];
        let ub = vec![10.0, 5.0, f64::INFINITY];
        let real_rows = vec![vec![(0, 1.0), (2, 1.0)]];
        let real_rhs = vec![7.0];
        let r = eliminate_implied_free_columns(3, &a, &b, &c, &lb, &ub, &real_rows, &real_rhs);
        assert_eq!(r.substitutions.len(), 1);
        // x0 + x2 <= 7 は (5 - x1) + x2 <= 7、すなわち -x1 + x2 <= 2 になる。
        assert_eq!(r.real_rows.len(), 1);
        assert_eq!(r.real_rows[0], vec![(1, -1.0), (2, 1.0)]);
        assert_eq!(r.real_rhs, vec![2.0]);
    }

    /// 1 回の呼び出しで最初から implied-free な 2 列が両方消去されることを確認する。
    #[test]
    fn cascading_elimination_within_one_call() {
        // Row0: x0 + x1 = 5. Row1: x0 - x2 = 1. Row2: x1 + x3 = 3.
        // x1 ∈ [-2,2] は row2 単独で implied-free (x3 ∈ [1,5] より x1 = 3-x3 ∈ [-2,2])。
        // x0 は row0 で x0 = 5-x1 ∈ [3,7] ⊂ [0,10] なので implied-free。
        // x0 は行 0,1、x1 は行 0,2 に現れるので両方が候補 (途中で含意範囲は再計算しない)。
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)], vec![(1, 1.0), (3, 1.0)]], 4);
        let b = vec![5.0, 1.0, 3.0];
        let c = vec![0.0, 0.0, 0.0, 0.0];
        let lb = vec![0.0, -2.0, f64::NEG_INFINITY, 1.0];
        let ub = vec![10.0, 2.0, f64::INFINITY, 5.0];
        let r = eliminate_implied_free_columns(4, &a, &b, &c, &lb, &ub, &[], &[]);
        assert_eq!(r.substitutions.len(), 2);
        let vars: std::collections::BTreeSet<usize> = r.substitutions.iter().map(|s| s.var).collect();
        assert_eq!(vars, [0usize, 1usize].into_iter().collect());
        assert_eq!(r.a.nrows(), 1);
    }

    /// `fillin_cost` が対象行に未だ無い列だけを数えることを確認する。
    #[test]
    fn fillin_cost_counts_only_genuinely_new_columns() {
        let terms = vec![(1, 1.0), (2, 1.0), (3, 1.0)];
        let existing_row = vec![(2, 5.0), (4, 1.0)];
        let targets: Vec<&Vec<(usize, f64)>> = vec![&existing_row];
        // 列 1 と 3 が新規、列 2 は既存。
        assert_eq!(fillin_cost(&terms, &targets), 2);
    }

    // --- eliminate_implied_free_columns_xrow --------------------------

    /// 行横断版が、単独では不十分な 2 行の共通範囲で列を消去することを確認する。
    #[test]
    fn xrow_eliminates_a_column_no_single_row_alone_justifies() {
        // x0 ∈ [0,10]。row0 (x0+x1=5, x1 ∈ [-3,8]) では x0 ∈ [-3,8] (下側がはみ出す)。
        // row1 (x0-x2=1, x2 ∈ [1,12]) では x0 ∈ [2,13] (上側がはみ出す)。
        // 共通部分 [2,8] は [0,10] に収まるので、両方の行が揃って初めて消去できる。
        let a = csr(&[vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, -1.0)]], 3);
        let b = vec![5.0, 1.0];
        let c = vec![0.0, 0.0, 0.0];
        let lb = vec![0.0, -3.0, 1.0];
        let ub = vec![10.0, 8.0, 12.0];
        // 行局所版は何も見つけてはならない。
        let row_local = eliminate_implied_free_columns(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert!(row_local.substitutions.is_empty());
        // 行横断版は共通部分で検出する。
        let xrow = eliminate_implied_free_columns_xrow(3, &a, &b, &c, &lb, &ub, &[], &[]);
        assert_eq!(xrow.substitutions.len(), 1);
        assert_eq!(xrow.substitutions[0].var, 0);
    }

    /// 行横断版が、先に消費された共有行を根拠に使わないことを確認する (`shell` 回帰テスト)。
    #[test]
    fn xrow_rejects_a_justification_whose_shared_row_was_already_consumed() {
        // x0 と x1 は共有行 row0 を通じて互いの箱を根拠づけ合う。x0 は先に処理され、
        // row0 を自身のピボットとして消費する。x1 のもう 1 本の行 row2 (x1 + x3 = 0,
        // x3 ∈ [-1000,1000]) は単独では緩すぎるので、x1 の唯一の実質的根拠は row0 だった。
        // その row0 は x1 の番には既に無いので、x1 を消去してはならない。
        //
        // x0, x1 ∈ [0,10]; x2 自由 (row1 を単独では無意味にし、x0 のピボットに選ばれないように);
        // x3 ∈ [-1000,1000] (row2 を緩くする); x4 ∈ [0,1] (row1 を row0 より長くする詰め物)。
        let a = csr(
            &[
                vec![(0, 1.0), (1, -1.0)],           // row0 (共有): x0 - x1 = 0
                vec![(0, 1.0), (2, 1.0), (4, 1.0)],  // row1 (x0 用, 無意味): x0 + x2 + x4 = 5
                vec![(1, 1.0), (3, 1.0)],             // row2 (x1 用, 緩い): x1 + x3 = 0
            ],
            5,
        );
        let b = vec![0.0, 5.0, 0.0];
        let c = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let lb = vec![0.0, 0.0, f64::NEG_INFINITY, -1000.0, 0.0];
        let ub = vec![10.0, 10.0, f64::INFINITY, 1000.0, 1.0];

        let xrow = eliminate_implied_free_columns_xrow(5, &a, &b, &c, &lb, &ub, &[], &[]);
        assert_eq!(xrow.substitutions.len(), 1, "expected only x0 eliminated, got {:?}", xrow.substitutions.iter().map(|s| s.var).collect::<Vec<_>>());
        assert_eq!(xrow.substitutions[0].var, 0);
        assert!(xrow.substitutions.iter().all(|s| s.var != 1), "x1 must not be eliminated via a since-deleted justifying row");
    }

    /// 全拡張を ON にした [`AggOptions`] (テスト用)。
    fn all_on() -> AggOptions {
        AggOptions { use_ineq: true, min_a_count: 1, net_fillin: true, fillin_break: true }
    }

    /// v2 でも消費済み共有行を根拠に使わないことを確認する (`shell` 回帰テスト)。
    #[test]
    fn v2_rejects_a_justification_whose_shared_row_was_already_consumed() {
        // 上の xrow テストと同じ形を v2 で。
        let a = csr(
            &[
                vec![(0, 1.0), (1, -1.0)],
                vec![(0, 1.0), (2, 1.0), (4, 1.0)],
                vec![(1, 1.0), (3, 1.0)],
            ],
            5,
        );
        let b = vec![0.0, 5.0, 0.0];
        let c = vec![0.0; 5];
        let lb = vec![0.0, 0.0, f64::NEG_INFINITY, -1000.0, 0.0];
        let ub = vec![10.0, 10.0, f64::INFINITY, 1000.0, 1.0];
        let r = eliminate_implied_free_columns_v2(5, &a, &b, &c, &lb, &ub, &[], &[], all_on());
        assert!(r.substitutions.iter().all(|s| s.var != 1), "x1 must not be eliminated via a since-deleted justifying row");
    }

    /// v2 が不等式行の片側境界を使って列を消去できること、`use_ineq` OFF ではできないことを確認する。
    #[test]
    fn v2_uses_an_inequality_row_to_justify_the_other_side() {
        // x0 - x1 = 0 (x1 >= 0) から x0 >= 0、実不等式 x0 + x2 <= 5 (x2 ∈ [0,1]) から x0 <= 5。
        // 合わせて x0 の箱 [0,5] を覆うので x0 (等式 1 本 + 不等式 1 本) は消去され、
        // 不等式行は x1 + x2 <= 5 に fold される。
        let a = csr(&[vec![(0, 1.0), (1, -1.0)]], 3);
        let b = vec![0.0];
        let c = vec![1.0, 0.0, 0.0];
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![5.0, f64::INFINITY, 1.0];
        let g = vec![vec![(0, 1.0), (2, 1.0)]];
        let h = vec![5.0];
        let r = eliminate_implied_free_columns_v2(3, &a, &b, &c, &lb, &ub, &g, &h, all_on());
        assert_eq!(r.substitutions.len(), 1);
        assert_eq!(r.substitutions[0].var, 0);
        assert_eq!(r.a.nrows(), 0);
        assert_eq!(r.real_rows, vec![vec![(1, 1.0), (2, 1.0)]]);
        assert_eq!(r.real_rhs, vec![5.0]);
        assert_eq!(r.c, vec![0.0, 1.0, 0.0]);

        // 不等式行を根拠に使わなければ上側が示せない。
        let off = AggOptions { use_ineq: false, ..all_on() };
        let r = eliminate_implied_free_columns_v2(3, &a, &b, &c, &lb, &ub, &g, &h, off);
        assert!(r.substitutions.is_empty());
    }

    /// `min_a_count = 2` で等式行 1 本の列を候補にしない旧条件になることを確認する。
    #[test]
    fn v2_min_a_count_two_keeps_the_old_gate() {
        let a = csr(&[vec![(0, 1.0), (1, -1.0)]], 3);
        let lb = vec![0.0, 0.0, 0.0];
        let ub = vec![5.0, f64::INFINITY, 1.0];
        let opts = AggOptions { min_a_count: 2, ..all_on() };
        let r = eliminate_implied_free_columns_v2(3, &a, &[0.0], &[0.0; 3], &lb, &ub, &[vec![(0, 1.0), (2, 1.0)]], &[5.0], opts);
        assert!(r.substitutions.is_empty());
    }

    /// ランダム問題で、`v2_has_candidate == false` なら v2 は何も消去しないこと、
    /// `_if_any` 版が `Some` のときは v2 本体と同じ結果を返すことを確認する。
    #[test]
    fn v2_has_candidate_is_consistent_with_v2() {
        // xorshift64 乱数の状態。
        let mut state: u64 = 0x2545_f491_4f6c_dd1d;
        let mut rnd = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut seen_none = 0;
        let mut seen_some = 0;
        for trial in 0..400 {
            let n = 3 + (trial % 5);
            let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
            for _ in 0..(1 + rnd() % 4) {
                let mut row: Vec<(usize, f64)> = Vec::new();
                for _ in 0..(1 + rnd() % 3) {
                    let j = (rnd() % n as u64) as usize;
                    if row.iter().all(|&(k, _)| k != j) {
                        row.push((j, ((rnd() % 5) as f64 - 2.0) + 0.5));
                    }
                }
                rows.push(row);
            }
            let b: Vec<f64> = rows.iter().map(|_| (rnd() % 7) as f64 - 3.0).collect();
            let mut g: Vec<Vec<(usize, f64)>> = Vec::new();
            for _ in 0..(rnd() % 3) {
                // わざと列順がソートされていない不等式行を作る。
                let j1 = (rnd() % n as u64) as usize;
                let j0 = (rnd() % n as u64) as usize;
                if j0 != j1 {
                    g.push(vec![(j1.max(j0), 1.0), (j1.min(j0), -0.5)]);
                }
            }
            let h: Vec<f64> = g.iter().map(|_| (rnd() % 9) as f64).collect();
            let bnd = |x: u64| [f64::NEG_INFINITY, -5.0, 0.0, 1.0][(x % 4) as usize];
            let lb: Vec<f64> = (0..n).map(|_| bnd(rnd())).collect();
            let ub: Vec<f64> = lb.iter().map(|&l| if rnd() % 4 == 0 { f64::INFINITY } else { l.max(0.0) + (rnd() % 10) as f64 + 1.0 }).collect();
            let c: Vec<f64> = (0..n).map(|_| (rnd() % 3) as f64).collect();
            let a = csr(&rows, n);
            let full = eliminate_implied_free_columns_v2(n, &a, &b, &c, &lb, &ub, &g, &h, all_on());
            match eliminate_implied_free_columns_v2_if_any(n, &a, &b, &c, &lb, &ub, &g, &h, all_on()) {
                None => {
                    seen_none += 1;
                    assert!(full.substitutions.is_empty(), "trial {trial}");
                }
                Some(r) => {
                    seen_some += 1;
                    assert_eq!(r.substitutions.len(), full.substitutions.len(), "trial {trial}");
                    assert_eq!(r.real_rows, full.real_rows, "trial {trial}");
                    assert_eq!(r.c, full.c, "trial {trial}");
                }
            }
        }
        assert!(seen_none > 0 && seen_some > 0, "none={seen_none} some={seen_some}");
    }
}
