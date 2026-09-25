//! 修正 Ruiz 均衡化 (スケーリング)。
//!
//! 変数と制約行を対角行列で再スケールし、`[A; G]` の各行・各列の無限大ノルムを
//! ほぼ 1 に揃えて、単体法の LU 系 (および内点法の KKT 系) の条件数を改善する。
//! `presolve::run_extended()` 1 回につき 1 度だけ、元データ上で計算する。
//!
//! スケール後の問題は `x = D x'`, `A' = diag(e_a) A diag(d)`, `b' = diag(e_a) b`,
//! `G' = diag(e_g) G diag(d)`, `h' = diag(e_g) h`, `c' = diag(d) c`。
//! 対角変換なので実行可能性・有界性は変わらず、`x = diag(d) x'` で元の解に戻る。
//!
//! 並列化: `apply`/`unscale_x` は常に逐次。`compute` の列ノルム集計だけは
//! 行数が [`RAYON_SIZE_THRESHOLD`] を超えたら rayon 版を使う。

use crate::sparse::{FaerCsr, CsrRowBuilder, csr_from_rows, csr_row_iter};
use crate::params::presolve::{RAYON_SIZE_THRESHOLD, SCALE_NOBOUNDS, SCALE_UNIT_FAST, SCALING_ZERO_TOL};

/// スケーリング係数一式。
pub struct Scaling {
    /// 列 (変数) スケール `d_j` (長さ n)。`x = d ∘ x'`。
    pub d: Vec<f64>,
    /// 等式行 (A) のスケール `e_a_i` (長さ p)。
    pub e_a: Vec<f64>,
    /// 不等式行 (G) のスケール `e_g_i` (長さ m)。
    pub e_g: Vec<f64>,
}

/// 列ノルム集計 (逐次版): `mat` の `rows` の各要素 `|v * d_j * e_i|` を
/// 列ごとの最大値として `acc` (長さ n) に畳み込む。A と G の両方で使う。
fn col_norm_fold(mat: faer::sparse::SparseRowMatRef<usize, f64>, d: &[f64], e: &[f64], rows: std::ops::Range<usize>, acc: &mut [f64]) {
    for i in rows {
        let ei = e[i];
        for (&j, &v) in mat.col_indices_of_row_raw(i).iter().zip(mat.values_of_row(i)) {
            let cand = (v * d[j] * ei).abs();
            if cand > acc[j] {
                acc[j] = cand;
            }
        }
    }
}

/// Ruiz 反復の行側の更新: `mat` の全行で
/// `e[i] <- e[i] / sqrt(max_j |v_ij * d_j * e[i]|)` (最大値がゼロ判定閾値以下なら据え置き)。
fn row_norm_update(mat: faer::sparse::SparseRowMatRef<usize, f64>, d: &[f64], e: &mut [f64]) {
    let zero_tol = tunable!("ENOMOTO_T_SCALING_ZERO_TOL", SCALING_ZERO_TOL, f64);
    for (i, e) in e.iter_mut().enumerate() {
        let old_e = *e;
        let mut row_norm = 0.0f64;
        for (&j, &v) in mat.col_indices_of_row_raw(i).iter().zip(mat.values_of_row(i)) {
            row_norm = row_norm.max((v * d[j] * old_e).abs());
        }
        if row_norm > zero_tol {
            *e = old_e / row_norm.sqrt();
        }
    }
}

/// `col_norm_fold` の rayon 版: スレッドごとの局所バッファに畳み込み (fold)、
/// 要素ごとの max で統合 (reduce) してから `acc` に反映する。
/// 異なる行が同じ列を更新しうるので、`acc` を直接並列更新はできない。
fn col_norm_fold_parallel(mat: faer::sparse::SparseRowMatRef<usize, f64>, d: &[f64], e: &[f64], rows: std::ops::Range<usize>, acc: &mut [f64]) {
    use rayon::prelude::*;
    let n = acc.len();
    // 葉ごとに O(n) のバッファを確保・統合するので、分割数をスレッド数程度に
    // 抑える (チャンク長 ≈ 行数 / スレッド数)。統合コストは O(threads * n)。
    let n_rows = rows.end - rows.start;
    let min_len = (n_rows / rayon::current_num_threads().max(1)).max(1);
    let folded = rows.into_par_iter().with_min_len(min_len).fold(
        || vec![0.0f64; n],
        |mut local, i| {
            for (j, &v) in mat.col_indices_of_row(i).zip(mat.values_of_row(i)) {
                let cand = (v * d[j] * e[i]).abs();
                if cand > local[j] {
                    local[j] = cand;
                }
            }
            local
        },
    );
    let merged = folded.reduce(
        || vec![0.0f64; n],
        |mut a, b| {
            for j in 0..n {
                if b[j] > a[j] {
                    a[j] = b[j];
                }
            }
            a
        },
    );
    for j in 0..n {
        if merged[j] > acc[j] {
            acc[j] = merged[j];
        }
    }
}

/// Ruiz 均衡化のスケール係数を計算する。`iters` は Ruiz 反復回数。
/// 目的係数 `c` も列ノルムに含める。
pub fn compute(n: usize, a: &FaerCsr, g: &FaerCsr, c: &[f64], iters: usize) -> Scaling {
    compute_impl(n, a, g, c, iters, tunable!("ENOMOTO_T_SCALE_UNIT_FAST", SCALE_UNIT_FAST, usize) != 0)
}

/// [`compute`] の本体。`unit_fast` が真なら G の単一要素行 (箱制約行) を
/// 専用リストで処理する高速経路を使う (結果は通常経路とビット一致)。
fn compute_impl(n: usize, a: &FaerCsr, g: &FaerCsr, c: &[f64], iters: usize, unit_fast: bool) -> Scaling {
    let p = a.nrows();
    let m = g.nrows();
    let mut d = vec![1.0; n];
    let mut e_a = vec![1.0; p];
    let mut e_g = vec![1.0; m];

    let ar = a.as_ref();
    let gr = g.as_ref();

    // 各列のスケール済み最大絶対値 (Ruiz 反復ごとに再計算)。
    let mut col_norm = vec![0.0f64; n];

    // 列ノルム集計を rayon で行うか (A+G の行数で一度だけ決める)。
    let use_parallel_fold = (p + m) > tunable!("ENOMOTO_T_SCALING_RAYON_SIZE_THRESHOLD", RAYON_SIZE_THRESHOLD, usize);

    // 実験用 (既定オフ): G の単一要素行 (箱制約行) を Ruiz 反復から除外し、
    // 最後に閉形式のスケール `1/|v * d_j|` を与える。スケール係数が変わるので数値経路も変わる。
    if tunable!("ENOMOTO_T_SCALE_NOBOUNDS", SCALE_NOBOUNDS, usize) != 0 {
        // G の多変数行の番号。
        let multi: Vec<usize> = (0..m).filter(|&i| gr.col_indices_of_row_raw(i).len() > 1).collect();
        for _ in 0..iters {
            col_norm.fill(0.0);
            col_norm_fold(ar, &d, &e_a, 0..p, &mut col_norm);
            for &i in &multi {
                col_norm_fold(gr, &d, &e_g, i..i + 1, &mut col_norm);
            }
            for j in 0..n {
                let cj = (c[j] * d[j]).abs();
                if cj > col_norm[j] {
                    col_norm[j] = cj;
                }
            }
            for j in 0..n {
                if col_norm[j] > tunable!("ENOMOTO_T_SCALING_ZERO_TOL", SCALING_ZERO_TOL, f64) {
                    d[j] /= col_norm[j].sqrt();
                }
            }
            row_norm_update(ar, &d, &mut e_a);
            let zero_tol = tunable!("ENOMOTO_T_SCALING_ZERO_TOL", SCALING_ZERO_TOL, f64);
            for &i in &multi {
                let old_e = e_g[i];
                let mut row_norm = 0.0f64;
                for (&j, &v) in gr.col_indices_of_row_raw(i).iter().zip(gr.values_of_row(i)) {
                    row_norm = row_norm.max((v * d[j] * old_e).abs());
                }
                if row_norm > zero_tol {
                    e_g[i] = old_e / row_norm.sqrt();
                }
            }
        }
        for i in 0..m {
            let cols = gr.col_indices_of_row_raw(i);
            if cols.len() == 1 {
                let s = (gr.values_of_row(i)[0] * d[cols[0]]).abs();
                if s > tunable!("ENOMOTO_T_SCALING_ZERO_TOL", SCALING_ZERO_TOL, f64) && s.is_finite() {
                    e_g[i] = 1.0 / s;
                }
            }
        }
        return Scaling { d, e_a, e_g };
    }

    // 逐次の高速経路: G の単一要素行 (箱制約行) を CSR 走査でなく
    // `(行, 列, 係数)` の平坦なリストで処理し、直前の単一要素行と列・|係数| が
    // 同じ行 (ub 行に続く lb 行など) はそのスケールを共有する (常にビット一致するため)。
    // 列ノルムは max なので順序非依存であり、結果は下の通常ループと完全に一致する。
    if !use_parallel_fold && unit_fast {
        let zero_tol = tunable!("ENOMOTO_T_SCALING_ZERO_TOL", SCALING_ZERO_TOL, f64);
        // G の多変数行の番号。
        let mut multi: Vec<usize> = Vec::new();
        // 代表となる単一要素行 `(行, 列, 係数)`。
        let mut unit: Vec<(usize, usize, f64)> = Vec::new();
        // スケールを代表に従わせる行 `(行, unit 内の代表の添字)`。
        let mut follow: Vec<(usize, usize)> = Vec::new();
        for i in 0..m {
            let cols = gr.col_indices_of_row_raw(i);
            if cols.len() == 1 {
                let (j, v) = (cols[0], gr.values_of_row(i)[0]);
                match unit.last() {
                    Some(&(_, lj, lv)) if lj == j && lv.abs().to_bits() == v.abs().to_bits() => follow.push((i, unit.len() - 1)),
                    _ => unit.push((i, j, v)),
                }
            } else {
                multi.push(i);
            }
        }
        // `unit` の各代表行のスケール (最後に e_g へ書き戻す)。
        let mut ue = vec![1.0f64; unit.len()];
        for _ in 0..iters {
            col_norm.fill(0.0);
            col_norm_fold(ar, &d, &e_a, 0..p, &mut col_norm);
            for &i in &multi {
                let ei = e_g[i];
                for (&j, &v) in gr.col_indices_of_row_raw(i).iter().zip(gr.values_of_row(i)) {
                    let cand = (v * d[j] * ei).abs();
                    if cand > col_norm[j] {
                        col_norm[j] = cand;
                    }
                }
            }
            for (&(_, j, v), &ei) in unit.iter().zip(&ue) {
                let cand = (v * d[j] * ei).abs();
                if cand > col_norm[j] {
                    col_norm[j] = cand;
                }
            }
            for j in 0..n {
                let cj = (c[j] * d[j]).abs();
                if cj > col_norm[j] {
                    col_norm[j] = cj;
                }
            }
            for j in 0..n {
                if col_norm[j] > zero_tol {
                    d[j] /= col_norm[j].sqrt();
                }
            }
            row_norm_update(ar, &d, &mut e_a);
            for &i in &multi {
                let old_e = e_g[i];
                let mut row_norm = 0.0f64;
                for (&j, &v) in gr.col_indices_of_row_raw(i).iter().zip(gr.values_of_row(i)) {
                    row_norm = row_norm.max((v * d[j] * old_e).abs());
                }
                if row_norm > zero_tol {
                    e_g[i] = old_e / row_norm.sqrt();
                }
            }
            for (&(_, j, v), e) in unit.iter().zip(ue.iter_mut()) {
                let old_e = *e;
                let row_norm = 0.0f64.max((v * d[j] * old_e).abs());
                if row_norm > zero_tol {
                    *e = old_e / row_norm.sqrt();
                }
            }
        }
        for (&(i, _, _), &e) in unit.iter().zip(&ue) {
            e_g[i] = e;
        }
        for &(i, k) in &follow {
            e_g[i] = ue[k];
        }
        return Scaling { d, e_a, e_g };
    }

    // 通常経路: CSR をそのまま走査する Ruiz 反復。
    for _ in 0..iters {
        col_norm.fill(0.0);
        if use_parallel_fold {
            col_norm_fold_parallel(ar, &d, &e_a, 0..p, &mut col_norm);
            col_norm_fold_parallel(gr, &d, &e_g, 0..m, &mut col_norm);
        } else {
            col_norm_fold(ar, &d, &e_a, 0..p, &mut col_norm);
            col_norm_fold(gr, &d, &e_g, 0..m, &mut col_norm);
        }
        for j in 0..n {
            let cj = (c[j] * d[j]).abs();
            if cj > col_norm[j] {
                col_norm[j] = cj;
            }
        }

        for j in 0..n {
            if col_norm[j] > tunable!("ENOMOTO_T_SCALING_ZERO_TOL", SCALING_ZERO_TOL, f64) {
                d[j] /= col_norm[j].sqrt();
            }
        }

        // 行側の更新 (各行は自分の e のみを書き換える)。
        row_norm_update(ar, &d, &mut e_a);
        row_norm_update(gr, &d, &mut e_g);
    }

    Scaling { d, e_a, e_g }
}

/// スケーリングを問題データに適用し、`(A', G', b', h', c')` を返す。
pub fn apply(scaling: &Scaling, a: &FaerCsr, g: &FaerCsr, b: &[f64], h: &[f64], c: &[f64]) -> (FaerCsr, FaerCsr, Vec<f64>, Vec<f64>, Vec<f64>) {
    let n = scaling.d.len();
    let p = a.nrows();
    let m = g.nrows();

    // 行ごとに独立だが逐次で処理する (モジュール冒頭の並列化の注記を参照)。
    let a_scaled = scale_rows(a, &scaling.e_a, &scaling.d, n);
    let g_scaled = scale_rows(g, &scaling.e_g, &scaling.d, n);
    debug_assert_eq!(a_scaled.nrows(), p);
    debug_assert_eq!(g_scaled.nrows(), m);
    let b_scaled: Vec<f64> = b.iter().zip(&scaling.e_a).map(|(v, e)| v * e).collect();
    let h_scaled: Vec<f64> = h.iter().zip(&scaling.e_g).map(|(v, e)| v * e).collect();
    let c_scaled: Vec<f64> = c.iter().zip(&scaling.d).map(|(v, d)| v * d).collect();

    (a_scaled, g_scaled, b_scaled, h_scaled, c_scaled)
}

/// 各要素を `v * e[i] * d[j]` に置き換えた行列を作る。[`CsrRowBuilder`] に
/// 使い回しの行バッファで直接書き込む (`csr_from_rows` と同一の結果)。
/// ビルダーが行を受け付けない場合は `csr_from_rows` での構築に切り替える。
fn scale_rows(mat: &FaerCsr, e: &[f64], d: &[f64], n: usize) -> FaerCsr {
    let r = mat.as_ref();
    let rows = r.nrows();
    let nnz: usize = (0..rows).map(|i| r.col_indices_of_row_raw(i).len()).sum();
    let mut builder = CsrRowBuilder::with_capacity(n, rows, nnz);
    let mut buf: Vec<(usize, f64)> = Vec::new();
    for i in 0..rows {
        let ei = e[i];
        buf.clear();
        buf.extend(r.col_indices_of_row_raw(i).iter().zip(r.values_of_row(i)).map(|(&j, &v)| (j, v * ei * d[j])));
        if !builder.push_row(&buf) {
            let all: Vec<Vec<(usize, f64)>> = (0..rows).map(|i| csr_row_iter(mat, i).map(|(j, v)| (j, v * e[i] * d[j])).collect()).collect();
            return csr_from_rows(&all, n);
        }
    }
    builder.finish()
}

/// スケール後の解 `x'` から元の問題の解 `x = diag(d) x'` を復元する
/// (求解終了後に一度だけ呼ぶ)。
pub fn unscale_x(scaling: &Scaling, x: &[f64]) -> Vec<f64> {
    x.iter().zip(&scaling.d).map(|(v, d)| v * d).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 単一要素行の高速経路が通常ループとビット単位で同じ係数を出すことを、
    /// 箱制約行の対・単独の箱制約行・係数付き単一行・空行・多変数行を含む乱数問題で確認する。
    #[test]
    fn unit_fast_path_matches_plain_loop_bit_for_bit() {
        let mut state: u64 = 0x5eed_1234_abcd_ef01;
        let mut rnd = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for trial in 0..100 {
            let n = 3 + (trial % 6);
            let val = |r: u64| [1.0, -2.5, 0.125, 7.0, -0.3, 1e3][(r % 6) as usize];
            let mut rows_of = |count: usize, rnd: &mut dyn FnMut() -> u64| -> Vec<Vec<(usize, f64)>> {
                (0..count)
                    .map(|_| {
                        let mut row: Vec<(usize, f64)> = Vec::new();
                        for _ in 0..(rnd() % 4) {
                            let j = (rnd() % n as u64) as usize;
                            if row.iter().all(|&(k, _)| k != j) {
                                row.push((j, val(rnd())));
                            }
                        }
                        row
                    })
                    .collect()
            };
            let a_rows = rows_of(1 + (rnd() % 3) as usize, &mut rnd);
            let mut g_rows = rows_of((rnd() % 4) as usize, &mut rnd);
            for j in 0..n {
                match rnd() % 4 {
                    0 => {
                        g_rows.push(vec![(j, 1.0)]);
                        g_rows.push(vec![(j, -1.0)]);
                    }
                    1 => g_rows.push(vec![(j, -1.0)]),
                    2 => g_rows.push(vec![(j, val(rnd()))]),
                    _ => {}
                }
            }
            let a = csr_from_rows(&a_rows, n);
            let g = csr_from_rows(&g_rows, n);
            let c: Vec<f64> = (0..n).map(|_| val(rnd())).collect();
            let fast = compute_impl(n, &a, &g, &c, 10, true);
            let plain = compute_impl(n, &a, &g, &c, 10, false);
            let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(&fast.d), bits(&plain.d), "trial {trial}");
            assert_eq!(bits(&fast.e_a), bits(&plain.e_a), "trial {trial}");
            assert_eq!(bits(&fast.e_g), bits(&plain.e_g), "trial {trial}");
        }
    }

    /// `col_norm_fold` と `col_norm_fold_parallel` の所要時間を行数を変えて比較する
    /// マイクロベンチ (1 行あたり約 5 非零)。既定では `#[ignore]`。実行は
    /// `cargo test --release col_norm_fold_rayon_threshold_microbench -- --ignored --nocapture`。
    #[test]
    #[ignore]
    fn col_norm_fold_rayon_threshold_microbench() {
        use std::time::Instant;

        for &rows in &[300usize, 1_000, 5_000, 10_000, 50_000, 200_000, 1_000_000, 4_000_000] {
            let n = (rows / 2).max(10);
            let row_data: Vec<Vec<(usize, f64)>> = (0..rows)
                .map(|i| {
                    (0..5)
                        .map(|k| {
                            let j = (i * 7 + k * 131) % n;
                            let v = ((i * 2654435761u64 as usize + k) % 1000) as f64 / 1000.0 + 0.1;
                            (j, v)
                        })
                        .collect()
                })
                .collect();
            let mat = csr_from_rows(&row_data, n);
            let mr = mat.as_ref();
            let d = vec![1.0f64; n];
            let e = vec![1.0f64; rows];
            const REPS: usize = 50;

            let t0 = Instant::now();
            for _ in 0..REPS {
                let mut acc = vec![0.0f64; n];
                col_norm_fold(mr, &d, &e, 0..rows, &mut acc);
                std::hint::black_box(&acc);
            }
            let seq = t0.elapsed() / REPS as u32;

            let t1 = Instant::now();
            for _ in 0..REPS {
                let mut acc = vec![0.0f64; n];
                col_norm_fold_parallel(mr, &d, &e, 0..rows, &mut acc);
                std::hint::black_box(&acc);
            }
            let par = t1.elapsed() / REPS as u32;

            println!("rows={rows:>7} sequential={seq:>10?} rayon={par:>10?} rayon/sequential={:.2}x", par.as_secs_f64() / seq.as_secs_f64());
        }
    }
}
