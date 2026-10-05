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

/// 対角ブロックがベクトルの拡大系 (augmented system)
///
///   [ diag(top)   A^T       ]
///   [   A       diag(mid)   ]
///
/// の疎 LDLᵀ。`A` は `p x n` (`FaerCsr`)。[`SparseKkt`] と違い、対角の値を要素ごとに
/// 与えられ、分解 ([`AugKkt::factor`]) と求解 ([`AugKkt::solve_in_place`]) を分けている
/// (1 回の分解で予測子・修正子など複数の右辺を解く)。箱型制約付きの内点法
/// (`interior_point::boxed`) と、クロスオーバーの射影 (`simplex::crossover`) が共有する。
/// 非零パターン・AMD 順序・記号分解は構築時に一度だけ作り、以後は値の上書きと数値分解だけ。
pub struct AugKkt {
    /// 主変数 (上段) の数。
    pub n: usize,
    /// 等式乗数 (下段) の数。
    pub p: usize,
    /// `values` 内で上段対角が並ぶ範囲。
    top_range: Range<usize>,
    /// `values` 内で下段対角が並ぶ範囲。
    mid_range: Range<usize>,
    /// 非零値 (挿入順)。対角以外 (`A`) は構築時のまま。
    values: Vec<f64>,
    /// 固定の非零パターン (上三角)。
    symbolic_base: SymbolicSparseColMat<usize>,
    /// 三つ組の並べ替え順序。
    order: ValuesOrder<usize>,
    /// AMD 順序付きの記号分解。
    chol_symbolic: SymbolicCholesky<usize>,
    /// 数値分解 `L D Lᵀ` の値。
    l_values: Vec<f64>,
    /// 数値分解の作業領域。
    numeric_buf: GlobalPodBuffer,
    /// 三角求解の作業領域。
    solve_buf: GlobalPodBuffer,
    /// 直近の [`Self::factor`] で使った上段対角 (反復改良の残差計算用)。
    top: Vec<f64>,
    /// 直近の [`Self::factor`] で使った下段対角。
    mid: Vec<f64>,
    /// 分解済みか。
    factored: bool,
    /// 動的正則化で期待するピボットの符号 (上段 +1、下段 -1)。準定値系の LDLᵀ が
    /// 丸め誤差でほぼ 0・逆符号のピボットに出会ったとき、`±PIVOT_DELTA` に置き換える。
    signs: Vec<i8>,
    /// 直近の分解で置き換えたピボットの数。
    pub n_regularized: usize,
}

/// [`AugKkt`] の動的正則化: 期待符号側の値がこれ以下のピボットを置き換える。
const PIVOT_EPS: f64 = 1e-13;
/// [`AugKkt`] の動的正則化で置き換える値の大きさ。
const PIVOT_DELTA: f64 = 1e-9;

impl AugKkt {
    /// `A` の非零パターンから記号分解までを作る (数値はまだ分解しない)。
    pub fn new(a: &FaerCsr) -> Self {
        let p = a.nrows();
        let n = a.ncols();
        let dim = n + p;
        let mut positions: Vec<(usize, usize)> = Vec::with_capacity(dim + a.compute_nnz());
        let mut values: Vec<f64> = Vec::with_capacity(dim + a.compute_nnz());
        for i in 0..n {
            positions.push((i, i));
            values.push(1.0);
        }
        let top_range = 0..n;
        for i in 0..p {
            for (j, v) in csr_row_iter(a, i) {
                if v != 0.0 {
                    positions.push((j, n + i));
                    values.push(v);
                }
            }
        }
        let mid_start = positions.len();
        for i in 0..p {
            positions.push((n + i, n + i));
            values.push(-1.0);
        }
        let mid_range = mid_start..positions.len();
        let (symbolic_base, order) =
            SymbolicSparseColMat::<usize>::try_new_from_indices(dim, dim, &positions).expect("valid KKT sparsity pattern");
        let chol_symbolic = factorize_symbolic_cholesky::<usize>(symbolic_base.as_ref(), Side::Upper, SymmetricOrdering::Amd, Default::default())
            .expect("symbolic factorization failed");
        let l_values = vec![0.0f64; chol_symbolic.len_values()];
        let numeric_buf = GlobalPodBuffer::new(chol_symbolic.factorize_numeric_ldlt_req::<f64>(true, KKT_PARALLELISM).unwrap());
        let solve_buf = GlobalPodBuffer::new(chol_symbolic.solve_in_place_req::<f64>(1).unwrap());
        AugKkt {
            n,
            p,
            top_range,
            mid_range,
            values,
            symbolic_base,
            order,
            chol_symbolic,
            l_values,
            numeric_buf,
            solve_buf,
            top: vec![1.0; n],
            mid: vec![-1.0; p],
            factored: false,
            signs: (0..dim).map(|i| if i < n { 1i8 } else { -1i8 }).collect(),
            n_regularized: 0,
        }
    }

    /// 系の次元 `n + p`。
    pub fn dim(&self) -> usize {
        self.n + self.p
    }

    /// `L` の非零数 (分解の手間の目安)。
    pub fn factor_nnz(&self) -> usize {
        self.chol_symbolic.len_values()
    }

    /// 対角 `top` (長さ n)・`mid` (長さ p) で数値分解する。
    pub fn factor(&mut self, top: &[f64], mid: &[f64]) {
        debug_assert_eq!(top.len(), self.n);
        debug_assert_eq!(mid.len(), self.p);
        self.values[self.top_range.clone()].copy_from_slice(top);
        self.values[self.mid_range.clone()].copy_from_slice(mid);
        self.top.copy_from_slice(top);
        self.mid.copy_from_slice(mid);
        let a_upper = SparseColMat::<usize, f64>::new_from_order_and_values(self.symbolic_base.clone(), &self.order, &self.values)
            .expect("value reorder failed");
        let reg = LdltRegularization {
            dynamic_regularization_signs: Some(&self.signs),
            dynamic_regularization_delta: tunable!("ENOMOTO_T_KKT_PIVOT_DELTA", PIVOT_DELTA, f64),
            dynamic_regularization_epsilon: tunable!("ENOMOTO_T_KKT_PIVOT_EPS", PIVOT_EPS, f64),
        };
        let _ = self.chol_symbolic.factorize_numeric_ldlt::<f64>(
            &mut self.l_values,
            a_upper.as_ref(),
            Side::Upper,
            reg,
            KKT_PARALLELISM,
            PodStack::new(&mut self.numeric_buf),
        );
        self.factored = true;
    }

    /// 直近の分解で `K x = rhs` を解き、`rhs` を解で上書きする。
    pub fn solve_in_place(&mut self, rhs: &mut [f64]) {
        assert!(self.factored, "AugKkt::solve_in_place before factor");
        let dim = self.dim();
        let ldlt = faer::sparse::linalg::cholesky::LdltRef::<usize, f64>::new(&self.chol_symbolic, &self.l_values);
        ldlt.solve_in_place_with_conj(Conj::No, from_column_major_slice_mut(rhs, dim, 1), KKT_PARALLELISM, PodStack::new(&mut self.solve_buf));
    }

    /// `out = K v` (直近の分解の対角を使う)。
    pub fn matvec(&self, a: &FaerCsr, v: &[f64], out: &mut [f64]) {
        let (n, p) = (self.n, self.p);
        let (vx, vy) = v.split_at(n);
        let (ox, oy) = out.split_at_mut(n);
        csr_mat_t_vec_into(a, vy, ox);
        for j in 0..n {
            ox[j] += self.top[j] * vx[j];
        }
        csr_mat_vec_into(a, vx, oy);
        for i in 0..p {
            oy[i] += self.mid[i] * vy[i];
        }
    }

    /// 反復改良付きの求解: `rhs` を解で上書きする (`refine` 回まで、残差が減らなくなれば止める)。
    pub fn solve_refined(&mut self, a: &FaerCsr, rhs: &mut [f64], refine: usize, work: &mut Vec<f64>) {
        if refine == 0 {
            self.solve_in_place(rhs);
            return;
        }
        let dim = self.dim();
        work.resize(3 * dim, 0.0);
        let (b, rest) = work.split_at_mut(dim);
        let (r, kx) = rest.split_at_mut(dim);
        b.copy_from_slice(rhs);
        self.solve_in_place(rhs);
        let mut prev = f64::INFINITY;
        for _ in 0..refine {
            self.matvec(a, rhs, kx);
            let mut nrm = 0.0f64;
            for i in 0..dim {
                r[i] = b[i] - kx[i];
                nrm = nrm.max(r[i].abs());
            }
            if !(nrm < prev * 0.5) || nrm == 0.0 {
                break;
            }
            prev = nrm;
            self.solve_in_place(r);
            for i in 0..dim {
                rhs[i] += r[i];
            }
        }
    }
}

/// 正規方程式 `(A diag(d)^{-1} A^T + δ I) dy = A diag(d)^{-1} r_x - r_y` による拡大系
/// `[diag(d) A^T; A -δI] [dx; dy] = [r_x; r_y]` の求解 (`dx = (r_x - A^T dy) / d`)。
///
/// 正規方程式の行列は正定値なので、疎 Cholesky (`L Lᵀ`、AMD 順序) で分解する。準定値の
/// 拡大系の LDLᵀ は正則化 δ が小さい (1e-10) と順序によってはピボットが崩れる (greenbea、
/// pilot4 で NaN) が、こちらは列を先に消去する順序に固定されるので安定。非零パターン
/// (`A A^T` の上三角) と、各列の要素の組 `(i, k)` が値の配列のどこへ足されるかを一度だけ作り、
/// 毎回は組ごとの積を書き込んで数値分解するだけ。稠密な列があると組の数が爆発するので、
/// その場合は [`NormalKkt::new`] が `None` を返し、呼び出し側は [`AugKkt`] を使う。
pub struct NormalKkt {
    n: usize,
    p: usize,
    /// 列 `j` の要素 (行, 値) (CSC)。
    col_ptr: Vec<usize>,
    col_ent: Vec<(usize, f64)>,
    /// 三つ組の値 (列ごとの組の積 + 対角の δ)。並びは構築時の三つ組の順。
    trip_values: Vec<f64>,
    /// 三つ組のうち対角 (δ を足す) の開始位置。
    diag_start: usize,
    symbolic_base: SymbolicSparseColMat<usize>,
    order: ValuesOrder<usize>,
    chol_symbolic: SymbolicCholesky<usize>,
    l_values: Vec<f64>,
    numeric_buf: GlobalPodBuffer,
    solve_buf: GlobalPodBuffer,
    /// 直近の分解の `d` の逆数。
    dinv: Vec<f64>,
    /// 作業領域 (長さ n)。
    tmp_n: Vec<f64>,
    factored: bool,
}

/// [`NormalKkt`] を使う三つ組の数の上限 (これを超えるなら稠密な列があるとみなし拡大系を使う)。
const NORMAL_MAX_TRIPLETS: usize = 40_000_000;

impl NormalKkt {
    /// `A` (`p x n`) から作る。組の数が多すぎれば `None`。
    pub fn new(a: &FaerCsr) -> Option<Self> {
        let p = a.nrows();
        let n = a.ncols();
        let mut cnt = vec![0usize; n + 1];
        for i in 0..p {
            for (j, v) in csr_row_iter(a, i) {
                if v != 0.0 {
                    cnt[j + 1] += 1;
                }
            }
        }
        for j in 0..n {
            cnt[j + 1] += cnt[j];
        }
        let col_ptr = cnt.clone();
        let mut fill = cnt;
        let mut col_ent = vec![(0usize, 0.0f64); col_ptr[n]];
        for i in 0..p {
            for (j, v) in csr_row_iter(a, i) {
                if v != 0.0 {
                    col_ent[fill[j]] = (i, v);
                    fill[j] += 1;
                }
            }
        }
        let mut n_trip = p;
        for j in 0..n {
            let k = col_ptr[j + 1] - col_ptr[j];
            n_trip += k * (k + 1) / 2;
            if n_trip > NORMAL_MAX_TRIPLETS {
                return None;
            }
        }
        let mut positions: Vec<(usize, usize)> = Vec::with_capacity(n_trip);
        for j in 0..n {
            let e = &col_ent[col_ptr[j]..col_ptr[j + 1]];
            for a_ in 0..e.len() {
                for b_ in a_..e.len() {
                    let (r1, r2) = (e[a_].0, e[b_].0);
                    positions.push((r1.min(r2), r1.max(r2)));
                }
            }
        }
        let diag_start = positions.len();
        for i in 0..p {
            positions.push((i, i));
        }
        let dbg = env_str!("ENOMOTO_DEBUG_IPM").is_some();
        let t0 = std::time::Instant::now();
        let (symbolic_base, order) = SymbolicSparseColMat::<usize>::try_new_from_indices(p, p, &positions).ok()?;
        drop(positions);
        if dbg {
            eprintln!("NormalKkt: triplets={n_trip} pattern nnz={} built in {:.2}s", symbolic_base.compute_nnz(), t0.elapsed().as_secs_f64());
        }
        let chol_symbolic =
            factorize_symbolic_cholesky::<usize>(symbolic_base.as_ref(), Side::Upper, SymmetricOrdering::Amd, Default::default()).ok()?;
        if dbg {
            eprintln!("NormalKkt: symbolic (AMD) nnz(L)={} at {:.2}s", chol_symbolic.len_values(), t0.elapsed().as_secs_f64());
        }
        let l_values = vec![0.0f64; chol_symbolic.len_values()];
        let numeric_buf = GlobalPodBuffer::new(chol_symbolic.factorize_numeric_llt_req::<f64>(KKT_PARALLELISM).ok()?);
        let solve_buf = GlobalPodBuffer::new(chol_symbolic.solve_in_place_req::<f64>(1).ok()?);
        Some(NormalKkt {
            n,
            p,
            col_ptr,
            col_ent,
            trip_values: vec![0.0; n_trip],
            diag_start,
            symbolic_base,
            order,
            chol_symbolic,
            l_values,
            numeric_buf,
            solve_buf,
            dinv: vec![0.0; n],
            tmp_n: vec![0.0; n],
            factored: false,
        })
    }

    /// `L` の非零数。
    pub fn factor_nnz(&self) -> usize {
        self.chol_symbolic.len_values()
    }

    /// `d` (上段対角、正) と `δ` で分解する。分解に失敗すれば `false`。
    pub fn factor(&mut self, d: &[f64], delta: f64) -> bool {
        use rayon::prelude::*;
        for j in 0..self.n {
            self.dinv[j] = 1.0 / d[j];
        }
        // 列ごとの組の値。列ごとに三つ組の位置が連続なので、列の開始位置を数えて並列に書く。
        let mut starts = Vec::with_capacity(self.n + 1);
        let mut acc = 0usize;
        for j in 0..self.n {
            starts.push(acc);
            let k = self.col_ptr[j + 1] - self.col_ptr[j];
            acc += k * (k + 1) / 2;
        }
        let (tv, _) = self.trip_values.split_at_mut(self.diag_start);
        let col_ptr = &self.col_ptr;
        let col_ent = &self.col_ent;
        let dinv = &self.dinv;
        // 列を塊に分けて、各塊が自分の範囲の三つ組だけを書く。
        let chunk = 4096usize;
        let n = self.n;
        let tv_ptr = tv.as_mut_ptr() as usize;
        (0..n.div_ceil(chunk)).into_par_iter().for_each(|c| {
            let tvp = tv_ptr as *mut f64;
            for j in c * chunk..((c + 1) * chunk).min(n) {
                let e = &col_ent[col_ptr[j]..col_ptr[j + 1]];
                let mut t = starts[j];
                let di = dinv[j];
                for a_ in 0..e.len() {
                    let va = e[a_].1 * di;
                    for b_ in a_..e.len() {
                        // SAFETY: 各列の三つ組の範囲 [starts[j], starts[j+1]) は互いに交わらない。
                        unsafe { *tvp.add(t) = va * e[b_].1 };
                        t += 1;
                    }
                }
            }
        });
        for v in &mut self.trip_values[self.diag_start..] {
            *v = delta;
        }
        let m = SparseColMat::<usize, f64>::new_from_order_and_values(self.symbolic_base.clone(), &self.order, &self.trip_values)
            .expect("value reorder failed");
        let reg = faer::sparse::linalg::cholesky::LltRegularization {
            dynamic_regularization_delta: tunable!("ENOMOTO_T_NORMAL_PIVOT_DELTA", 1e-8, f64),
            dynamic_regularization_epsilon: tunable!("ENOMOTO_T_NORMAL_PIVOT_EPS", 1e-14, f64),
        };
        let ok = self
            .chol_symbolic
            .factorize_numeric_llt::<f64>(&mut self.l_values, m.as_ref(), Side::Upper, reg, KKT_PARALLELISM, PodStack::new(&mut self.numeric_buf))
            .is_ok();
        self.factored = ok;
        ok
    }

    /// `[dx; dy]` を `rhs = [r_x; r_y]` に上書きする (`δ` は直近の分解のもの)。
    pub fn solve_in_place(&mut self, a: &FaerCsr, rhs: &mut [f64]) {
        assert!(self.factored, "NormalKkt::solve_in_place before factor");
        let (n, p) = (self.n, self.p);
        let (rx, ry) = rhs.split_at_mut(n);
        // dy の右辺 = A D^{-1} r_x - r_y
        for j in 0..n {
            self.tmp_n[j] = rx[j] * self.dinv[j];
        }
        let mut t = vec![0.0; p];
        csr_mat_vec_into(a, &self.tmp_n, &mut t);
        for i in 0..p {
            ry[i] = t[i] - ry[i];
        }
        let llt = faer::sparse::linalg::cholesky::LltRef::<usize, f64>::new(&self.chol_symbolic, &self.l_values);
        llt.solve_in_place_with_conj(Conj::No, from_column_major_slice_mut(ry, p, 1), KKT_PARALLELISM, PodStack::new(&mut self.solve_buf));
        // dx = D^{-1} (r_x - A^T dy)
        csr_mat_t_vec_into(a, ry, &mut self.tmp_n);
        for j in 0..n {
            rx[j] = (rx[j] - self.tmp_n[j]) * self.dinv[j];
        }
    }
}

/// 内点法の Newton 系 `[diag(top) A^T; A -δI]` の解法: 正規方程式 (既定) か拡大系。
pub enum IpmKkt {
    Normal(NormalKkt),
    Aug(AugKkt),
}

impl IpmKkt {
    /// 正規方程式を作れればそれを、だめ (稠密な列) なら拡大系を使う。
    /// `ENOMOTO_IPM_AUGMENTED=1` で常に拡大系。
    pub fn new(a: &FaerCsr) -> Self {
        if env_str!("ENOMOTO_IPM_AUGMENTED").is_none() {
            if let Some(nk) = NormalKkt::new(a) {
                return IpmKkt::Normal(nk);
            }
        }
        IpmKkt::Aug(AugKkt::new(a))
    }

    pub fn is_normal(&self) -> bool {
        matches!(self, IpmKkt::Normal(_))
    }

    pub fn factor_nnz(&self) -> usize {
        match self {
            IpmKkt::Normal(k) => k.factor_nnz(),
            IpmKkt::Aug(k) => k.factor_nnz(),
        }
    }

    /// 上段 `top`、下段 `-delta` で分解する。
    pub fn factor(&mut self, top: &[f64], delta: f64, mid_buf: &mut [f64]) -> bool {
        match self {
            IpmKkt::Normal(k) => k.factor(top, delta),
            IpmKkt::Aug(k) => {
                mid_buf.fill(-delta);
                k.factor(top, mid_buf);
                true
            }
        }
    }

    /// 反復改良付きで解く (`top`, `delta` は直近の分解のもの)。
    pub fn solve_refined(&mut self, a: &FaerCsr, top: &[f64], delta: f64, rhs: &mut [f64], refine: usize, work: &mut Vec<f64>) {
        let n = top.len();
        let dim = rhs.len();
        work.resize(3 * dim, 0.0);
        let (b, rest) = work.split_at_mut(dim);
        let (r, kx) = rest.split_at_mut(dim);
        b.copy_from_slice(rhs);
        self.solve_plain(a, rhs);
        let mut prev = f64::INFINITY;
        for _ in 0..refine {
            // kx = K rhs
            {
                let (vx, vy) = rhs.split_at(n);
                let (ox, oy) = kx.split_at_mut(n);
                csr_mat_t_vec_into(a, vy, ox);
                for j in 0..n {
                    ox[j] += top[j] * vx[j];
                }
                csr_mat_vec_into(a, vx, oy);
                for i in 0..oy.len() {
                    oy[i] -= delta * vy[i];
                }
            }
            let mut nrm = 0.0f64;
            for i in 0..dim {
                r[i] = b[i] - kx[i];
                nrm = nrm.max(r[i].abs());
            }
            if !(nrm < prev * 0.5) || nrm == 0.0 {
                break;
            }
            prev = nrm;
            self.solve_plain(a, r);
            for i in 0..dim {
                rhs[i] += r[i];
            }
        }
    }

    fn solve_plain(&mut self, a: &FaerCsr, rhs: &mut [f64]) {
        match self {
            IpmKkt::Normal(k) => k.solve_in_place(a, rhs),
            IpmKkt::Aug(k) => k.solve_in_place(rhs),
        }
    }
}
