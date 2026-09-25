//! IP-PMM の Newton 方程式 (KKT 系) の疎な組み立てと求解。
//!
//! `A` と `G` は faer の CSR (`SparseRowMat`) のまま扱い、密行列には変換しない。
//! 初期化と各 Newton 反復で解く KKT 系はすべて同じブロック形
//!
//!   [ top_diag*I      A^T          G^T         ]
//!   [   A          mid_diag*I       0          ]
//!   [   G              0       diag(bottom)    ]
//!
//! で、対称準定値 (Vanderbei) なので faer の疎 Cholesky 系 (記号分解
//! `factorize_symbolic_cholesky` + 数値 LDLᵀ `factorize_numeric_ldlt`) で分解する。
//! simplicial/supernodal の選択と AMD 順序付けは faer が自動で行う。
//!
//! 呼び出しごとに変わるのは対角ブロックの値だけなので、非零パターン・AMD 順序・
//! 記号分解・作業バッファは最初の `solve_into` で一度だけ作って `Setup` に保持し、
//! 以後は値の上書きと数値分解・求解だけを行う (呼び出しごとの確保は
//! `symbolic_base` の軽い clone のみ)。

use std::ops::Range;

use faer::dyn_stack::{GlobalPodBuffer, PodStack};
use faer::mat::from_column_major_slice_mut;
use faer::sparse::linalg::cholesky::{factorize_symbolic_cholesky, LdltRegularization, SymbolicCholesky, SymmetricOrdering};
use faer::sparse::{SparseColMat, SymbolicSparseColMat, ValuesOrder};
use faer::{Conj, Side};

pub use crate::sparse::{FaerCsr, csr_row_iter, csr_mat_t_vec, csr_mat_t_vec_into, csr_mat_vec, csr_mat_vec_into};
use crate::params::interior_point::KKT_PARALLELISM;

/// `A`/`G` の非零パターンごとに一度だけ計算して使い回すもの一式:
/// 記号 Cholesky 分解 (AMD 順序と消去構造。数値には依存しない) と、
/// 数値分解・求解の作業バッファ (反復中に確保しないよう事前に確保)。
struct Setup {
    /// KKT 行列の次元 `n + p + m`。
    dim: usize,
    /// `values` 内で上段対角ブロック (主変数 `x`) の値が並ぶ範囲。
    top_range: Range<usize>,
    /// `values` 内で中段対角ブロック (等式の乗数 `y`) の値が並ぶ範囲。
    mid_range: Range<usize>,
    /// `values` 内で下段対角ブロック (不等式の乗数 `z`) の値が並ぶ範囲。
    bottom_range: Range<usize>,
    /// KKT 行列の非零値 (挿入順)。構造は固定で、毎回値だけ上書きする。
    values: Vec<f64>,
    /// 固定の非零パターン (faer の Cholesky 系が要求する上三角のみ)。
    symbolic_base: SymbolicSparseColMat<usize>,
    /// 三つ組の並べ替え・重複除去の順序。新しい `values` をソートし直さずに行列化できる。
    order: ValuesOrder<usize>,
    /// `symbolic_base` から一度だけ計算した AMD 順序付きの記号分解。
    chol_symbolic: SymbolicCholesky<usize>,
    /// 数値分解 `L` の値。毎回再計算するが、領域の確保は一度だけ。
    l_values: Vec<f64>,
    /// 数値 LDLᵀ 分解の作業領域。
    numeric_buf: GlobalPodBuffer,
    /// 三角求解の作業領域。
    solve_buf: GlobalPodBuffer,
}

/// KKT 系の寸法 (`n` 主変数 + `p` 等式乗数 + `m` 不等式乗数) と、最初の
/// `solve_into` 以降に使い回す `Setup`。問題ごとに 1 つ作り、Newton 反復全体で共有する。
pub struct SparseKkt {
    /// 主変数の数。
    pub n: usize,
    /// 等式制約の数。
    pub p: usize,
    /// 不等式制約の数。
    pub m: usize,
    /// 使い回す記号分解と作業領域 (最初の求解で作る)。
    setup: Option<Setup>,
}

impl SparseKkt {
    /// 寸法だけを持つ空の `SparseKkt` を作る (`Setup` は最初の求解時に作る)。
    pub fn new(n: usize, p: usize, m: usize) -> Self {
        SparseKkt { n, p, m, setup: None }
    }

    /// KKT 行列の次元 `n + p + m`。
    pub fn dim(&self) -> usize {
        self.n + self.p + self.m
    }

    /// 非零パターン (上三角) と初期値を組み立て、記号分解と作業領域を用意する。
    fn build_setup(&self, a: &FaerCsr, g: &FaerCsr, top_diag: f64, mid_diag: f64, bottom_diag: &[f64]) -> Setup {
        let (n, p, m) = (self.n, self.p, self.m);
        let dim = self.dim();

        // 非零の (行, 列) 位置と値 (挿入順)
        let mut positions: Vec<(usize, usize)> = Vec::new();
        let mut values: Vec<f64> = Vec::new();

        for i in 0..n {
            positions.push((i, i));
            values.push(top_diag);
        }
        let top_range = 0..n;

        for i in 0..p {
            for (j, v) in csr_row_iter(a, i) {
                if v != 0.0 {
                    // 行 = j < n <= n+i = 列 なので常に上三角
                    positions.push((j, n + i));
                    values.push(v);
                }
            }
        }
        for i in 0..m {
            for (j, v) in csr_row_iter(g, i) {
                if v != 0.0 {
                    positions.push((j, n + p + i));
                    values.push(v);
                }
            }
        }

        let mid_start = positions.len();
        for i in 0..p {
            positions.push((n + i, n + i));
            values.push(mid_diag);
        }
        let mid_range = mid_start..positions.len();

        let bottom_start = positions.len();
        for i in 0..m {
            positions.push((n + p + i, n + p + i));
            values.push(bottom_diag[i]);
        }
        let bottom_range = bottom_start..positions.len();

        let (symbolic_base, order) = SymbolicSparseColMat::<usize>::try_new_from_indices(dim, dim, &positions)
            .expect("valid KKT sparsity pattern");

        // 一度だけ: AMD 順序付けと simplicial/supernodal の記号解析 (faer が自動選択)。
        // パターンだけで決まるので、値がまだ本物でなくても実行できる。
        let chol_symbolic = factorize_symbolic_cholesky::<usize>(
            symbolic_base.as_ref(),
            Side::Upper,
            SymmetricOrdering::Amd,
            Default::default(),
        )
        .expect("symbolic factorization failed");

        let l_values = vec![0.0f64; chol_symbolic.len_values()];
        // `_req` が返す作業量は並列度に依存するので、`solve_into` の実際の呼び出しと
        // 同じ `KKT_PARALLELISM` を渡す (食い違うと作業領域が不足する)。
        let numeric_buf = GlobalPodBuffer::new(
            chol_symbolic.factorize_numeric_ldlt_req::<f64>(false, KKT_PARALLELISM).unwrap(),
        );
        let solve_buf = GlobalPodBuffer::new(chol_symbolic.solve_in_place_req::<f64>(1).unwrap());

        Setup {
            dim,
            top_range,
            mid_range,
            bottom_range,
            values,
            symbolic_base,
            order,
            chol_symbolic,
            l_values,
            numeric_buf,
            solve_buf,
        }
    }

    /// 上記ブロック形の KKT 系 `K x = rhs` を解き、長さ `dim()` の解を `out` に書く。
    /// `top_diag` / `mid_diag` は上段・中段の対角値 (スカラー)、`bottom_diag` は下段の対角。
    pub fn solve_into(&mut self, a: &FaerCsr, g: &FaerCsr, top_diag: f64, mid_diag: f64, bottom_diag: &[f64], rhs: &[f64], out: &mut [f64]) {
        if self.setup.is_none() {
            self.setup = Some(self.build_setup(a, g, top_diag, mid_diag, bottom_diag));
        }
        let setup = self.setup.as_mut().unwrap();

        // 呼び出しごとに変わるのは対角ブロックだけ。A/G 由来の非対角要素は
        // `build_setup` で書いたまま。
        for v in &mut setup.values[setup.top_range.clone()] {
            *v = top_diag;
        }
        for v in &mut setup.values[setup.mid_range.clone()] {
            *v = mid_diag;
        }
        setup.values[setup.bottom_range.clone()].copy_from_slice(bottom_diag);

        // `build_setup` で計算済みの並べ替え順序を再適用して行列化する (再ソートしない)。
        let a_upper = SparseColMat::<usize, f64>::new_from_order_and_values(
            setup.symbolic_base.clone(),
            &setup.order,
            &setup.values,
        )
        .expect("value reorder failed");

        let ldlt = setup.chol_symbolic.factorize_numeric_ldlt::<f64>(
            &mut setup.l_values,
            a_upper.as_ref(),
            Side::Upper,
            LdltRegularization::default(),
            KKT_PARALLELISM,
            PodStack::new(&mut setup.numeric_buf),
        );

        out.copy_from_slice(rhs);
        ldlt.solve_in_place_with_conj(
            Conj::No,
            from_column_major_slice_mut(out, setup.dim, 1),
            KKT_PARALLELISM,
            PodStack::new(&mut setup.solve_buf),
        );
    }
}
