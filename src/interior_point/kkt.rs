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

pub use crate::sparse::{FaerCsr, csr_row_iter, csr_mat_t_vec, csr_mat_t_vec_into, csr_mat_vec, csr_mat_vec_into, csr_transpose};
use crate::params::interior_point::{FACTOR_PAR_NNZ, KKT_PARALLELISM, MAX_FACTOR_NNZ};

/// 因子の非零数の上限 ([`MAX_FACTOR_NNZ`]、試験用 `ENOMOTO_T_IPM_MAX_FACTOR_NNZ`)。
fn max_factor_nnz() -> usize {
    tunable!("ENOMOTO_T_IPM_MAX_FACTOR_NNZ", MAX_FACTOR_NNZ, usize)
}

/// 正規方程式の記号分解の設定 (試験用)。`ENOMOTO_T_CHOL_RELAX`: 0 = faer の既定 (小さな supernode を、明示的な 0 を許して
/// 併合する: 4 列以下は 100%、16 列以下は 80%、48 列以下は 10%、それ以上は 5% まで)、1 = 併合しない (因子は最も疎)、
/// 2 = 控えめに併合 (4 列以下 50%、16 列以下 20%、それ以上 2%)。`ENOMOTO_T_CHOL_AMD_DENSE`: AMD が密な行とみなす
/// 次数の倍率 (既定 10、次数 > 倍率 √n)。`ENOMOTO_T_CHOL_SUPERNODAL`: supernodal を選ぶ閾値 (既定 1)。
fn chol_symbolic_params() -> faer::sparse::linalg::cholesky::CholeskySymbolicParams<'static> {
    chol_symbolic_params_with(FORCE_SUPERNODAL.with(|c| c.get()))
}

thread_local! {
    /// 記号分解で必ず supernodal にする (マルチフロンタル法の分解のため)。
    static FORCE_SUPERNODAL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn chol_symbolic_params_with(force_supernodal: bool) -> faer::sparse::linalg::cholesky::CholeskySymbolicParams<'static> {
    use faer::sparse::linalg::{amd::Control, cholesky::CholeskySymbolicParams, SupernodalThreshold, SymbolicSupernodalParams};
    static RELAX_TIGHT: [(usize, f64); 3] = [(4, 0.5), (16, 0.2), (usize::MAX, 0.02)];
    let relax: Option<&'static [(usize, f64)]> = match tunable!("ENOMOTO_T_CHOL_RELAX", 0u8, u8) {
        1 => None,
        2 => Some(&RELAX_TIGHT),
        _ => SymbolicSupernodalParams::default().relax,
    };
    CholeskySymbolicParams {
        amd_params: Control { dense: tunable!("ENOMOTO_T_CHOL_AMD_DENSE", 10.0f64, f64), ..Default::default() },
        supernodal_flop_ratio_threshold: SupernodalThreshold(if force_supernodal { 0.0 } else { tunable!("ENOMOTO_T_CHOL_SUPERNODAL", 1.0f64, f64) }),
        supernodal_params: SymbolicSupernodalParams { relax },
    }
}

/// METIS の nested dissection による並べ替え (上三角の非零の形 `pat` から隣接グラフを作る)。`(perm, perm_inv)` を返す
/// (`perm[新] = 旧`)。失敗したら `None`。
fn metis_ordering(pat: &SymbolicSparseColMat<usize>) -> Option<(Vec<usize>, Vec<usize>)> {
    let n = pat.nrows();
    if n == 0 || n > i32::MAX as usize {
        return None;
    }
    let cp = pat.col_ptrs();
    let ri = pat.row_indices();
    let mut deg = vec![0usize; n];
    for c in 0..n {
        for &r in &ri[cp[c]..cp[c + 1]] {
            if r != c {
                deg[r] += 1;
                deg[c] += 1;
            }
        }
    }
    let mut xadj = vec![0i32; n + 1];
    for v in 0..n {
        xadj[v + 1] = xadj[v] + deg[v] as i32;
    }
    let mut fill: Vec<usize> = xadj[..n].iter().map(|&v| v as usize).collect();
    let mut adj = vec![0i32; xadj[n] as usize];
    for c in 0..n {
        for &r in &ri[cp[c]..cp[c + 1]] {
            if r != c {
                adj[fill[r]] = c as i32;
                fill[r] += 1;
                adj[fill[c]] = r as i32;
                fill[c] += 1;
            }
        }
    }
    let mut nv = n as i32;
    let mut perm = vec![0i32; n];
    let mut iperm = vec![0i32; n];
    let mut options = [0i32; metis_sys::METIS_NOPTIONS as usize];
    // SAFETY: 配列の長さは METIS の要求どおり (xadj は n+1、adjncy は xadj[n]、perm・iperm は n、options は METIS_NOPTIONS)。
    unsafe {
        metis_sys::METIS_SetDefaultOptions(options.as_mut_ptr());
        let st = metis_sys::METIS_NodeND(
            &mut nv,
            xadj.as_mut_ptr(),
            adj.as_mut_ptr(),
            std::ptr::null_mut(),
            options.as_mut_ptr(),
            perm.as_mut_ptr(),
            iperm.as_mut_ptr(),
        );
        if st != metis_sys::rstatus_et_METIS_OK as i32 {
            return None;
        }
    }
    Some((perm.into_iter().map(|v| v as usize).collect(), iperm.into_iter().map(|v| v as usize).collect()))
}

/// 正規方程式の記号分解。並べ替えは `ENOMOTO_T_CHOL_ORDER`: 0 = AMD、1 = METIS (nested dissection)、
/// 2 (既定) = AMD の演算量が大きいときだけ METIS も試し、演算量の少ない方 (行数 `ENOMOTO_T_CHOL_ORDER_MIN_N` 以上のときだけ)。
/// 2 は 2026-10-09 の比較 (auto の内点法を成分ごとに解くのと合わせて) で既定にした: Mittelmann 9 問のシフト付き幾何平均
/// 0.818 倍 (nug08-3rd 425 → 160 秒、supportcase10 26 → 18 秒)、Netlib + Kennington 0.998 倍。以前 METIS で悪化した
/// fome13 は、auto で 8 成分をまとめて 1 つの内点法で解いていたのが原因で、成分ごとに解けば悪化しない。
fn symbolic_with_ordering(pat: &SymbolicSparseColMat<usize>, dbg: bool, limit: usize) -> Option<SymbolicCholesky<usize>> {
    let mode = tunable!("ENOMOTO_T_CHOL_ORDER", 2u8, u8);
    let min_n = tunable!("ENOMOTO_T_CHOL_ORDER_MIN_N", 1000usize, usize);
    let amd = || factorize_symbolic_cholesky::<usize>(pat.as_ref(), Side::Upper, SymmetricOrdering::Amd, chol_symbolic_params()).ok();
    if mode == 0 || pat.nrows() < min_n {
        return amd();
    }
    // 2: まず AMD。演算量の見積もりが `ENOMOTO_T_CHOL_ORDER_MIN_FLOPS` 未満なら METIS は試さない (並べ替えの時間が割に合わない)。
    let a = if mode == 2 { amd() } else { None };
    let a_flops = a.as_ref().map_or(f64::INFINITY, chol_flops);
    if mode == 2 && a_flops < tunable!("ENOMOTO_T_CHOL_ORDER_MIN_FLOPS", 1e8f64, f64) {
        return a;
    }
    // AMD の因子が分解の上限 (`limit`、上限なしは `usize::MAX`) の `ENOMOTO_T_CHOL_SKIP_METIS_RATIO` 倍 (既定 4、0 で使わない) を超えるなら、
    // METIS でも上限に収まらないので試さない (METIS の nnz(L) はこれまで AMD の 0.3〜0.5 倍。ex10 の正規方程式は AMD 7.5 億・
    // METIS 3.6 億で、どちらも上限 1 億を超えて捨てるのに METIS に 7.7 秒かけていた)。
    let skip_ratio = tunable!("ENOMOTO_T_CHOL_SKIP_METIS_RATIO", 4.0f64, f64);
    if mode == 2 && skip_ratio > 0.0 && limit < usize::MAX && a.as_ref().is_some_and(|c| c.len_values() as f64 > skip_ratio * limit as f64) {
        if dbg {
            eprintln!("NormalKkt: AMD nnz(L)={:?} exceeds {skip_ratio} x the factor limit; METIS skipped", a.as_ref().map(|c| c.len_values()));
        }
        return a;
    }
    let t0 = std::time::Instant::now();
    let nd = metis_ordering(pat).and_then(|(perm, perm_inv)| {
        let p = faer::perm::PermRef::<usize>::new_checked(&perm, &perm_inv, pat.nrows());
        factorize_symbolic_cholesky::<usize>(pat.as_ref(), Side::Upper, SymmetricOrdering::Custom(p), chol_symbolic_params()).ok()
    });
    let t_nd = t0.elapsed().as_secs_f64();
    if mode == 1 {
        if dbg {
            eprintln!("NormalKkt: METIS nnz(L)={:?} in {t_nd:.2}s", nd.as_ref().map(|c| c.len_values()));
        }
        return nd.or_else(amd);
    }
    let nd_flops = nd.as_ref().map_or(f64::INFINITY, chol_flops);
    if dbg {
        eprintln!(
            "NormalKkt: AMD nnz(L)={:?} flops={a_flops:.3e}, METIS nnz(L)={:?} flops={nd_flops:.3e} (METIS {t_nd:.2}s)",
            a.as_ref().map(|c| c.len_values()),
            nd.as_ref().map(|c| c.len_values())
        );
    }
    // 演算量の少ない方 (METIS は 0.8 倍未満のときだけ)。
    match (a, nd) {
        (Some(a), Some(nd)) => Some(if nd_flops < 0.8 * a_flops { nd } else { a }),
        (a, nd) => a.or(nd),
    }
}

/// 記号分解から数値分解の演算量を見積もる (各列の非零の数の 2 乗の和)。
fn chol_flops(c: &SymbolicCholesky<usize>) -> f64 {
    use faer::sparse::linalg::cholesky::SymbolicCholeskyRaw;
    match c.raw() {
        SymbolicCholeskyRaw::Simplicial(s) => {
            let cp = s.col_ptrs();
            (0..s.ncols()).map(|j| ((cp[j + 1] - cp[j]) as f64).powi(2)).sum()
        }
        SymbolicCholeskyRaw::Supernodal(s) => {
            let mut f = 0.0;
            for k in 0..s.n_supernodes() {
                let nc = s.supernode_end()[k] - s.supernode_begin()[k];
                let r = s.supernode(k).pattern().len();
                for j in 0..nc {
                    f += ((nc - j + r) as f64).powi(2);
                }
            }
            f
        }
    }
}

thread_local! {
    /// 計測用: 正規方程式の組み立てと数値分解の累計時間 (秒)。[`take_factor_prof`] で取り出して 0 に戻す。
    static FACTOR_PROF: std::cell::Cell<(f64, f64)> = const { std::cell::Cell::new((0.0, 0.0)) };
}

/// 正規方程式の組み立てと数値分解の累計時間 (秒) を取り出して 0 に戻す (計測用)。
pub fn take_factor_prof() -> (f64, f64) {
    FACTOR_PROF.with(|c| c.replace((0.0, 0.0)))
}

/// `Aᵀ y` を `out` に書く。試験用 `ENOMOTO_T_IPM_AT_PAR=1` で `Aᵀ` の行圧縮 `at` を使い行ごとに並列に計算する (既定は従来の
/// `A` の行圧縮のまま逐次に足し込む。比較用)。
pub fn at_mul(a: &FaerCsr, at: &FaerCsr, y: &[f64], out: &mut [f64]) {
    if tunable!("ENOMOTO_T_IPM_AT_PAR", 0u8, u8) != 0 {
        csr_mat_vec_into(at, y, out);
    } else {
        csr_mat_t_vec_into(a, y, out);
    }
}

thread_local! {
    /// このスレッドで分解を並列にしない ([`with_inner_seq`])。
    static INNER_SEQ: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `seq` なら `f` の間、このスレッドの分解 (faer・マルチフロンタル法) を並列にしない (独立な成分を外側で並列に解くとき)。
pub fn with_inner_seq<R>(seq: bool, f: impl FnOnce() -> R) -> R {
    let prev = INNER_SEQ.with(|c| c.replace(seq));
    let r = f();
    INNER_SEQ.with(|c| c.set(prev));
    r
}

/// このスレッドで分解を並列にしないか。
pub fn inner_seq() -> bool {
    INNER_SEQ.with(|c| c.get())
}

/// 数値分解に使う並列度。因子の非零数 `nnz_l` が `ENOMOTO_T_FACTOR_PAR_NNZ` (既定 [`FACTOR_PAR_NNZ`]、0 で使わない) 以上なら
/// 並列、それ未満は逐次 (Fable の調査と Netlib + Kennington の比較で、小さな疎 Cholesky では faer の並列分解の分割の手間が
/// 計算を上回った。一方 qap15 (nnz(L) 1,770 万) では 1 回の分解が 1.3 秒かかり、逐次では内点法の 9 割を占める)。
/// 試験用 `ENOMOTO_T_FACTOR_SEQ=0` で常に並列。作業領域の見積もり (`_req`) にも同じ値を使う。
fn factor_par(nnz_l: usize) -> faer::Parallelism<'static> {
    if inner_seq() {
        return faer::Parallelism::None;
    }
    let par_nnz = tunable!("ENOMOTO_T_FACTOR_PAR_NNZ", FACTOR_PAR_NNZ, usize);
    if tunable!("ENOMOTO_T_FACTOR_SEQ", 1u8, u8) == 0 || (par_nnz > 0 && nnz_l >= par_nnz) {
        KKT_PARALLELISM
    } else {
        faer::Parallelism::None
    }
}

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
    /// 自前のマルチフロンタル法 (LDLᵀ) で分解するとき (`ENOMOTO_T_AUG_BACKEND=2`、faer が supernodal を選び演算量の見積もりが
    /// `ENOMOTO_T_MF_MIN_FLOPS` 以上のとき)。
    mf: Option<super::multifrontal::Multifrontal>,
}

/// 反復改良を打ち切る残差 (右辺の無限大ノルム (1 以上) に対する相対値)。
const REFINE_TOL: f64 = 1e-13;

/// [`AugKkt`] の動的正則化: 期待符号側の値がこれ以下のピボットを置き換える。
const PIVOT_EPS: f64 = 1e-13;
/// [`AugKkt`] の動的正則化で置き換える値の大きさ。
const PIVOT_DELTA: f64 = 1e-9;

impl AugKkt {
    /// `A` の非零パターンから記号分解までを作る (数値はまだ分解しない)。
    pub fn new(a: &FaerCsr) -> Self {
        Self::build(a, usize::MAX).expect("symbolic factorization failed")
    }

    /// [`AugKkt::new`] と同じだが、因子の非零数が `max_nnz` を超えたら `None`。
    pub fn try_new(a: &FaerCsr, max_nnz: usize) -> Option<Self> {
        Self::build(a, max_nnz)
    }

    fn build(a: &FaerCsr, max_nnz: usize) -> Option<Self> {
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
        // `ENOMOTO_T_AUG_ORDER=1` (既定): 正規方程式と同じ並べ替えの選び方 (AMD と METIS の演算量の少ない方)。0 は AMD だけ。
        let chol_symbolic = if tunable!("ENOMOTO_T_AUG_ORDER", 1u8, u8) == 1 {
            symbolic_with_ordering(&symbolic_base, env_str!("ENOMOTO_DEBUG_IPM").is_some(), max_nnz)?
        } else {
            factorize_symbolic_cholesky::<usize>(symbolic_base.as_ref(), Side::Upper, SymmetricOrdering::Amd, Default::default()).ok()?
        };
        if chol_symbolic.len_values() > max_nnz {
            if env_str!("ENOMOTO_DEBUG_IPM").is_some() {
                eprintln!("AugKkt: nnz(L)={} exceeds {max_nnz}; giving up", chol_symbolic.len_values());
            }
            return None;
        }
        let signs: Vec<i8> = (0..dim).map(|i| if i < n { 1i8 } else { -1i8 }).collect();
        // `ENOMOTO_T_AUG_BACKEND=2` (既定): 自前のマルチフロンタル法 (LDLᵀ) で分解する。0 は faer。
        let mf = if tunable!("ENOMOTO_T_AUG_BACKEND", 2u8, u8) == 2 && chol_flops(&chol_symbolic) >= tunable!("ENOMOTO_T_MF_MIN_FLOPS", 2e7f64, f64) {
            super::multifrontal::Multifrontal::new_ldlt(&symbolic_base, &chol_symbolic, &signs)
        } else {
            None
        };
        if env_str!("ENOMOTO_DEBUG_IPM").is_some() {
            eprintln!("AugKkt: backend={} nnz(L)={} flops={:.2e}", if mf.is_some() { "multifrontal" } else { "faer" }, chol_symbolic.len_values(), chol_flops(&chol_symbolic));
        }
        let l_values = if mf.is_some() { Vec::new() } else { vec![0.0f64; chol_symbolic.len_values()] };
        let numeric_buf = if mf.is_some() {
            GlobalPodBuffer::new(faer::dyn_stack::StackReq::empty())
        } else {
            GlobalPodBuffer::new(chol_symbolic.factorize_numeric_ldlt_req::<f64>(true, factor_par(chol_symbolic.len_values())).unwrap())
        };
        let solve_buf = GlobalPodBuffer::new(chol_symbolic.solve_in_place_req::<f64>(1).unwrap());
        Some(AugKkt {
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
            signs,
            n_regularized: 0,
            mf,
        })
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
        if let Some(mf) = self.mf.as_mut() {
            let ok = mf.factor(a_upper.values(), reg.dynamic_regularization_delta, reg.dynamic_regularization_epsilon);
            self.n_regularized = super::multifrontal::DYNREG.swap(0, std::sync::atomic::Ordering::Relaxed);
            if env_str!("ENOMOTO_DEBUG_REFINE").is_some() {
                let vmax = a_upper.values().iter().fold(0.0f64, |m, v| m.max(v.abs()));
                eprintln!("FACTOR aug mf ok={ok} dynreg={} max|value|={vmax:.2e} top[min,max]=[{:.2e},{:.2e}] mid[min,max]=[{:.2e},{:.2e}]", self.n_regularized,
                    top.iter().cloned().fold(f64::INFINITY, f64::min), top.iter().cloned().fold(0.0, f64::max),
                    mid.iter().cloned().fold(f64::INFINITY, f64::min), mid.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
            }
            self.factored = true;
            return;
        }
        let _ = self.chol_symbolic.factorize_numeric_ldlt::<f64>(
            &mut self.l_values,
            a_upper.as_ref(),
            Side::Upper,
            reg,
            factor_par(self.chol_symbolic.len_values()),
            PodStack::new(&mut self.numeric_buf),
        );
        self.factored = true;
    }

    /// 直近の分解で `K x = rhs` を解き、`rhs` を解で上書きする。
    pub fn solve_in_place(&mut self, rhs: &mut [f64]) {
        assert!(self.factored, "AugKkt::solve_in_place before factor");
        if let Some(mf) = self.mf.as_ref() {
            mf.solve_in_place(rhs);
            return;
        }
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
/// pilot4 で NaN) が、こちらは列を先に消去する順序に固定されるので安定。
///
/// - **稠密な列**: 非零の多い列 (`dense_threshold` 超) が 1 本でもあると `A A^T` は密になる (fit2p は
///   3000 行に対し 25 本の列で L が完全に密)。そうした列 `A_d` は正規方程式から外し、
///   `M = M_s + U U^T` (`U = A_d D_d^{-1/2}`) を Sherman–Morrison–Woodbury で解く:
///   `M^{-1} r = M_s^{-1} r - W (I + U^T W)^{-1} U^T M_s^{-1} r`、`W = M_s^{-1} U` (分解ごとに k 回の求解)。
/// - **組み立て**: 疎な列の要素の組 `(i, k)` が CSC の値の配列のどこへ足されるか (`dest`) を最初に一度だけ
///   求め、毎回は値の配列に直接足し込む (並べ替えや非零パターンの複製をしない)。
/// - 組の数が多すぎる (稠密な列を外しても)、または因子の非零数が [`MAX_FACTOR_NNZ`] を超えるなら
///   [`NormalKkt::new`] は `None` を返し、呼び出し側は
///   [`AugKkt`] を使う。
pub struct NormalKkt {
    n: usize,
    p: usize,
    /// 列 `j` の要素 (行, 値) (CSC)。
    col_ptr: Vec<usize>,
    col_ent: Vec<(usize, f64)>,
    /// 正規方程式に入れる (疎な) 列と、外して Woodbury で扱う稠密な列。
    sparse_cols: Vec<usize>,
    dense_cols: Vec<usize>,
    /// 疎な列の組 (列ごとに連続、列 `sparse_cols[c]` の組は `dest[trip_start[c]..trip_start[c+1]]`) の行き先。
    trip_start: Vec<usize>,
    dest: Vec<u32>,
    /// 対角 `(i, i)` の行き先。
    diag_dest: Vec<u32>,
    /// CSC の値 (上三角)。
    values: Vec<f64>,
    symbolic_base: SymbolicSparseColMat<usize>,
    chol_symbolic: SymbolicCholesky<usize>,
    l_values: Vec<f64>,
    numeric_buf: GlobalPodBuffer,
    solve_buf: GlobalPodBuffer,
    /// 直近の分解の `d` の逆数。
    dinv: Vec<f64>,
    /// Woodbury: `W = M_s^{-1} U` (列優先 `p x k`) と `C = I + U^T W` の Cholesky 因子 (下三角、行優先 `k x k`)。
    w_mat: Vec<f64>,
    c_chol: Vec<f64>,
    /// 作業領域。
    tmp_n: Vec<f64>,
    tmp_p: Vec<f64>,
    factored: bool,
    /// 対角スケーリングの係数 `S = diag(M)^{-1/2}` (使わないなら空)。
    dscale: Vec<f64>,
    /// MKL PARDISO で分解するとき (試験用 `ENOMOTO_T_CHOL_BACKEND=1`、MKL を読み込めたとき)。
    pardiso: Option<super::pardiso::Pardiso>,
    /// 自前のマルチフロンタル法で分解するとき (既定の `ENOMOTO_T_CHOL_BACKEND=2`、演算量の見積もりが下限以上のとき)。
    mf: Option<super::multifrontal::Multifrontal>,
}

/// [`NormalKkt`] を使う三つ組の数の上限 (これを超えるなら拡大系を使う)。
/// (組 1 つにつき行き先の `u32` 4 バイト。1.5 億で 600 MB。scpm1 は 5000 行・50 万列で約 4,200 万組だが、正規方程式は
/// 5000 x 5000 で済み、拡大系 (記号分解 105 秒、数値分解 1 回 3.6 秒) よりはるかに軽い。試験用 `ENOMOTO_T_NORMAL_MAX_TRIPLETS`)。
const NORMAL_MAX_TRIPLETS: usize = 150_000_000;
/// 正規方程式の三つ組の行き先を p x p の位置表で引く行数の上限 (p^2 の要素数、`u32` で 128 MB)。
const DENSE_POS_MAX: usize = 32_000_000;
/// 稠密な列とみなす非零数: `max(DENSE_COL_MIN, DENSE_COL_AVG_FACTOR * 平均)` を超える列。
const DENSE_COL_MIN: usize = 50;
const DENSE_COL_AVG_FACTOR: f64 = 10.0;
/// Woodbury で扱う稠密な列の数の上限 (`W` の大きさ `p * k` とも比べる)。
const DENSE_COL_MAX: usize = 1000;
/// 稠密な列を外すのは行数がこれ以上で、稠密な列の組の数が他の列の組の数のこの倍以上のときだけ。
const DENSE_COL_MIN_ROWS: usize = 1000;
const DENSE_PAIR_RATIO: usize = 4;
const DENSE_W_MAX_ENTRIES: usize = 50_000_000;

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
        // 稠密な列の判定
        let avg = col_ptr[n] as f64 / n.max(1) as f64;
        let thr = (DENSE_COL_MIN as f64).max(DENSE_COL_AVG_FACTOR * avg);
        let mut dense_cols: Vec<usize> = (0..n).filter(|&j| (col_ptr[j + 1] - col_ptr[j]) as f64 > thr).collect();
        // 外すのは、行数が大きく (密な分解が重い) しかも稠密な列が組の大半を占めるときだけ。小さな問題
        // (fit1p、627 行) では密な分解でも安く、外すと残りの M_s の条件が悪くなって Woodbury の精度が落ちる。
        let pairs = |j: usize| {
            let k = col_ptr[j + 1] - col_ptr[j];
            k * (k + 1) / 2
        };
        let dense_pairs: usize = dense_cols.iter().map(|&j| pairs(j)).sum();
        let all_pairs: usize = (0..n).map(pairs).sum();
        if p < DENSE_COL_MIN_ROWS
            || dense_pairs < DENSE_PAIR_RATIO * (all_pairs - dense_pairs)
            || dense_cols.len() > DENSE_COL_MAX
            || dense_cols.len() * p > DENSE_W_MAX_ENTRIES
        {
            dense_cols.clear();
        }
        let mut is_dense = vec![false; n];
        for &j in &dense_cols {
            is_dense[j] = true;
        }
        let sparse_cols: Vec<usize> = (0..n).filter(|&j| !is_dense[j]).collect();
        let mut n_trip = 0usize;
        let mut trip_start = Vec::with_capacity(sparse_cols.len() + 1);
        for &j in &sparse_cols {
            trip_start.push(n_trip);
            let k = col_ptr[j + 1] - col_ptr[j];
            n_trip += k * (k + 1) / 2;
            if n_trip + p > tunable!("ENOMOTO_T_NORMAL_MAX_TRIPLETS", NORMAL_MAX_TRIPLETS, usize) {
                return None;
            }
        }
        trip_start.push(n_trip);
        let dbg = env_str!("ENOMOTO_DEBUG_IPM").is_some();
        let t0 = std::time::Instant::now();
        // `M = Σ_j a_j a_j^T + I` (疎な列だけ) の上三角のパターンを、組を並べ替えずに列ごとに作る (Gustavson)。
        // 列 `c` の行 `r <= c` は、行 `c` に非零を持つ疎な列 `j` の非零の行のうち `c` 以下のもの。
        let mut m_ptr = vec![0usize; p + 1];
        let mut m_rows: Vec<usize> = Vec::new();
        {
            let mut mark = vec![usize::MAX; p];
            for c in 0..p {
                let start = m_rows.len();
                mark[c] = c;
                m_rows.push(c);
                for (j, v) in csr_row_iter(a, c) {
                    if v == 0.0 || is_dense[j] {
                        continue;
                    }
                    for &(r, _) in &col_ent[col_ptr[j]..col_ptr[j + 1]] {
                        if r <= c && mark[r] != c {
                            mark[r] = c;
                            m_rows.push(r);
                        }
                    }
                }
                m_rows[start..].sort_unstable();
                m_ptr[c + 1] = m_rows.len();
            }
        }
        let symbolic_base = SymbolicSparseColMat::<usize>::new_checked(p, p, m_ptr, None, m_rows);
        // 各組の行き先 (CSC の値の位置) を列内の二分探索で求める (組の順は `trip_start` と同じ)。
        let col_ptrs = symbolic_base.col_ptrs();
        let row_idx = symbolic_base.row_indices();
        // 行数が小さい (p^2 <= DENSE_POS_MAX) なら p x p の位置表を引く (scpm1: 4,500 万組の二分探索に 18 秒かかっていた)。
        let pos_table: Option<Vec<u32>> = (p.checked_mul(p).is_some_and(|pp| pp <= DENSE_POS_MAX)).then(|| {
            let mut t = vec![u32::MAX; p * p];
            for c in 0..p {
                for k in col_ptrs[c]..col_ptrs[c + 1] {
                    t[c * p + row_idx[k]] = k as u32;
                }
            }
            t
        });
        let find = |r: usize, c: usize| -> u32 {
            if let Some(t) = &pos_table {
                return t[c * p + r];
            }
            let seg = &row_idx[col_ptrs[c]..col_ptrs[c + 1]];
            (col_ptrs[c] + seg.binary_search(&r).expect("entry in pattern")) as u32
        };
        let mut dest: Vec<u32> = Vec::with_capacity(n_trip);
        for &j in &sparse_cols {
            let e = &col_ent[col_ptr[j]..col_ptr[j + 1]];
            for a_ in 0..e.len() {
                for b_ in a_..e.len() {
                    let (r1, r2) = (e[a_].0, e[b_].0);
                    dest.push(find(r1.min(r2), r1.max(r2)));
                }
            }
        }
        let diag_dest: Vec<u32> = (0..p).map(|i| find(i, i)).collect();
        if dbg {
            eprintln!(
                "NormalKkt: dense_cols={} (threshold {thr:.0}) triplets={n_trip} pattern nnz={} built in {:.2}s",
                dense_cols.len(),
                symbolic_base.compute_nnz(),
                t0.elapsed().as_secs_f64()
            );
        }
        // 0: faer、1: MKL PARDISO (試験用)、2: 自前のマルチフロンタル法 (既定、第 61 回の比較で決めた)。
        let backend = tunable!("ENOMOTO_T_CHOL_BACKEND", 2u8, u8);
        let chol_symbolic = symbolic_with_ordering(&symbolic_base, dbg, max_factor_nnz())?;
        if dbg {
            eprintln!("NormalKkt: symbolic (AMD) nnz(L)={} at {:.2}s", chol_symbolic.len_values(), t0.elapsed().as_secs_f64());
        }
        // 因子の大きさの判定はマルチフロンタル法の準備 (因子の値の配列 `nnz(L)` 個を確保する) より前に行う
        // (後で判定していたので、rmine15 の元の問題で 61 億個 = 48.8 GB を確保しようとして落ちた)。
        if chol_symbolic.len_values() > max_factor_nnz() {
            return None;
        }
        // 2: faer が supernodal を選び、演算量の見積もりが `ENOMOTO_T_MF_MIN_FLOPS` 以上なら自前のマルチフロンタル法で
        // 分解する (小さな・simplicial 向きの因子は faer のまま: osa-60 は因子が 3.2 万で、supernodal にすると遅い)。
        let mf = if backend == 2 && chol_flops(&chol_symbolic) >= tunable!("ENOMOTO_T_MF_MIN_FLOPS", 2e7f64, f64) {
            super::multifrontal::Multifrontal::new(&symbolic_base, &chol_symbolic)
        } else {
            None
        };
        // 試験用 (`ENOMOTO_T_CHOL_BACKEND=1`): MKL PARDISO で分解する (読み込めなければ faer)。faer の因子の配列は作らない。
        let pardiso = if backend == 1 {
            super::pardiso::Pardiso::new(p, symbolic_base.col_ptrs(), symbolic_base.row_indices())
        } else {
            None
        };
        if dbg {
            eprintln!("NormalKkt: backend={} flops={:.2e}", if pardiso.is_some() { "pardiso" } else if mf.is_some() { "multifrontal" } else { "faer" }, chol_flops(&chol_symbolic));
        }
        let l_values = if pardiso.is_some() || mf.is_some() { Vec::new() } else { vec![0.0f64; chol_symbolic.len_values()] };
        let numeric_buf = if pardiso.is_some() || mf.is_some() {
            GlobalPodBuffer::new(faer::dyn_stack::StackReq::empty())
        } else {
            GlobalPodBuffer::new(chol_symbolic.factorize_numeric_llt_req::<f64>(factor_par(chol_symbolic.len_values())).ok()?)
        };
        let solve_buf = GlobalPodBuffer::new(chol_symbolic.solve_in_place_req::<f64>(1).ok()?);
        let nnz = symbolic_base.compute_nnz();
        let k = dense_cols.len();
        Some(NormalKkt {
            n,
            p,
            col_ptr,
            col_ent,
            sparse_cols,
            dense_cols,
            trip_start,
            dest,
            diag_dest,
            values: vec![0.0; nnz],
            symbolic_base,
            chol_symbolic,
            l_values,
            numeric_buf,
            solve_buf,
            dinv: vec![0.0; n],
            w_mat: vec![0.0; p * k],
            c_chol: vec![0.0; k * k],
            tmp_n: vec![0.0; n],
            tmp_p: vec![0.0; p],
            factored: false,
            dscale: Vec::new(),
            pardiso,
            mf,
        })
    }

    /// `L` の非零数。
    pub fn factor_nnz(&self) -> usize {
        self.chol_symbolic.len_values()
    }

    /// `M_s` (疎な列だけの正規方程式) の Cholesky で `rhs` を上書きして解く。
    fn solve_ms(&mut self, rhs: &mut [f64]) {
        // 対角スケーリングして分解したなら M^{-1} r = S (S M S)^{-1} S r。
        let scaled = !self.dscale.is_empty();
        if scaled {
            for (v, s) in rhs.iter_mut().zip(&self.dscale) {
                *v *= s;
            }
        }
        if let Some(pd) = self.pardiso.as_mut() {
            pd.solve_in_place(rhs);
        } else if let Some(mf) = self.mf.as_ref() {
            mf.solve_in_place(rhs);
        } else {
            let llt = faer::sparse::linalg::cholesky::LltRef::<usize, f64>::new(&self.chol_symbolic, &self.l_values);
            llt.solve_in_place_with_conj(Conj::No, from_column_major_slice_mut(rhs, self.p, 1), KKT_PARALLELISM, PodStack::new(&mut self.solve_buf));
        }
        if scaled {
            for (v, s) in rhs.iter_mut().zip(&self.dscale) {
                *v *= s;
            }
        }
    }

    /// `d` (上段対角、正) と `δ` で分解する。分解に失敗すれば `false`。
    pub fn factor(&mut self, d: &[f64], delta: f64) -> bool {
        use rayon::prelude::*;
        let (n, p) = (self.n, self.p);
        let t_asm = std::time::Instant::now();
        self.dinv.par_iter_mut().zip(d.par_iter()).for_each(|(o, &v)| *o = 1.0 / v);
        // 疎な列の組の値を CSC の値の配列に直接足し込む。
        self.values.fill(0.0);
        let vals = &mut self.values;
        for (c, &j) in self.sparse_cols.iter().enumerate() {
            let e = &self.col_ent[self.col_ptr[j]..self.col_ptr[j + 1]];
            let dst = &self.dest[self.trip_start[c]..self.trip_start[c + 1]];
            let di = self.dinv[j];
            let mut t = 0usize;
            for a_ in 0..e.len() {
                let va = e[a_].1 * di;
                for b_ in a_..e.len() {
                    vals[dst[t] as usize] += va * e[b_].1;
                    t += 1;
                }
            }
        }
        for &q in &self.diag_dest {
            vals[q as usize] += delta;
        }
        // 試験用 (`ENOMOTO_T_NORMAL_DIAG_SCALE=1`): 対角が 1 になるよう S M S (S = diag(M)^{-1/2}) にしてから分解する
        // (動的正則化の閾値は絶対値なので、終盤に対角の幅が広がると、どのピボットを置き換えるかが行の尺度に左右される)。
        if tunable!("ENOMOTO_T_NORMAL_DIAG_SCALE", 0u8, u8) != 0 {
            let p_ = self.p;
            self.dscale.resize(p_, 1.0);
            for i in 0..p_ {
                let dg = vals[self.diag_dest[i] as usize];
                self.dscale[i] = if dg > 0.0 && dg.is_finite() { 1.0 / dg.sqrt() } else { 1.0 };
            }
            let cp = self.symbolic_base.col_ptrs();
            let ri = self.symbolic_base.row_indices();
            for c in 0..p_ {
                let sc = self.dscale[c];
                for k in cp[c]..cp[c + 1] {
                    vals[k] *= sc * self.dscale[ri[k]];
                }
            }
        } else {
            self.dscale.clear();
        }
        let t_num = std::time::Instant::now();
        FACTOR_PROF.with(|c| {
            let (a, n_) = c.get();
            c.set((a + t_asm.elapsed().as_secs_f64(), n_));
        });
        let m = faer::sparse::SparseColMatRef::<usize, f64>::new(self.symbolic_base.as_ref(), &self.values);
        let reg = faer::sparse::linalg::cholesky::LltRegularization {
            dynamic_regularization_delta: tunable!("ENOMOTO_T_NORMAL_PIVOT_DELTA", 1e-8, f64),
            dynamic_regularization_epsilon: tunable!("ENOMOTO_T_NORMAL_PIVOT_EPS", 1e-14, f64),
        };
        let ok = if let Some(pd) = self.pardiso.as_mut() {
            let _ = (m, reg);
            pd.factor(&self.values)
        } else if let Some(mf) = self.mf.as_mut() {
            let _ = m;
            let ok = mf.factor(&self.values, reg.dynamic_regularization_delta, reg.dynamic_regularization_epsilon);
            if env_str!("ENOMOTO_DEBUG_REFINE").is_some() {
                eprintln!("FACTOR mf ok={ok} t={:.3}s prof={:?} dynreg={}", t_num.elapsed().as_secs_f64(), super::multifrontal::take_prof(), super::multifrontal::DYNREG.swap(0, std::sync::atomic::Ordering::Relaxed));
            }
            ok
        } else {
            let r = self.chol_symbolic
                .factorize_numeric_llt::<f64>(&mut self.l_values, m, Side::Upper, reg, factor_par(self.chol_symbolic.len_values()), PodStack::new(&mut self.numeric_buf));
            if env_str!("ENOMOTO_DEBUG_REFINE").is_some() {
                match &r {
                    Ok(_) => eprintln!("FACTOR faer ok"),
                    Err(e) => eprintln!("FACTOR faer err minor={} nan_in_values={}", e.non_positive_definite_minor, self.values.iter().any(|v| !v.is_finite())),
                }
            }
            r.is_ok()
        };
        self.factored = ok;
        FACTOR_PROF.with(|c| {
            let (a, n_) = c.get();
            c.set((a, n_ + t_num.elapsed().as_secs_f64()));
        });
        if !ok {
            return false;
        }
        // Woodbury: W = M_s^{-1} U (U の列 q = a_q * sqrt(dinv_q))、C = I + U^T W の Cholesky。
        let k = self.dense_cols.len();
        if k > 0 {
            let t_w = std::time::Instant::now();
            let mut w = std::mem::take(&mut self.w_mat);
            for q in 0..k {
                let j = self.dense_cols[q];
                let col = &mut w[q * p..(q + 1) * p];
                col.fill(0.0);
                let sq = self.dinv[j].sqrt();
                for &(i, v) in &self.col_ent[self.col_ptr[j]..self.col_ptr[j + 1]] {
                    col[i] = v * sq;
                }
            }
            // multifrontal なら列を束ねて行列積で解き、束どうしは並列に回す。束の大きさは列をスレッドに
            // 等分した数 (4 本以上、`ENOMOTO_T_WOODBURY_BLOCK` (既定 64) 本以下、0 なら 1 本ずつ)。
            let block_max = tunable!("ENOMOTO_T_WOODBURY_BLOCK", 64usize, usize);
            let block = k.div_ceil(rayon::current_num_threads().max(1)).max(4).min(block_max);
            if block > 0 && self.pardiso.is_none() && self.mf.is_some() {
                use rayon::prelude::*;
                let mf = self.mf.as_ref().unwrap();
                let dscale = &self.dscale;
                w.par_chunks_mut(p * block).for_each(|cols| {
                    if !dscale.is_empty() {
                        for col in cols.chunks_mut(p) {
                            for (v, s) in col.iter_mut().zip(dscale) {
                                *v *= s;
                            }
                        }
                    }
                    mf.solve_multi_in_place(cols, cols.len() / p);
                    if !dscale.is_empty() {
                        for col in cols.chunks_mut(p) {
                            for (v, s) in col.iter_mut().zip(dscale) {
                                *v *= s;
                            }
                        }
                    }
                });
            } else {
                for q in 0..k {
                    self.solve_ms(&mut w[q * p..(q + 1) * p]);
                }
            }
            self.w_mat = w;
            if env_str!("ENOMOTO_DEBUG_WOODBURY").is_some() {
                eprintln!("WOODBURY p={p} k={k} block={block} solve={:.3}s", t_w.elapsed().as_secs_f64());
            }
            let mut cm = vec![0.0f64; k * k];
            for r in 0..k {
                let jr = self.dense_cols[r];
                let sq = self.dinv[jr].sqrt();
                for c in 0..k {
                    let wc = &self.w_mat[c * p..(c + 1) * p];
                    let mut acc = 0.0;
                    for &(i, v) in &self.col_ent[self.col_ptr[jr]..self.col_ptr[jr + 1]] {
                        acc += v * wc[i];
                    }
                    cm[r * k + c] = sq * acc + if r == c { 1.0 } else { 0.0 };
                }
            }
            // 対称化してから密 Cholesky (行優先の下三角)
            for r in 0..k {
                for c in 0..r {
                    let v = 0.5 * (cm[r * k + c] + cm[c * k + r]);
                    cm[r * k + c] = v;
                    cm[c * k + r] = v;
                }
            }
            for c in 0..k {
                let mut dg = cm[c * k + c];
                for t in 0..c {
                    dg -= cm[c * k + t] * cm[c * k + t];
                }
                if !(dg > 0.0) {
                    self.factored = false;
                    return false;
                }
                let dg = dg.sqrt();
                cm[c * k + c] = dg;
                for r in c + 1..k {
                    let mut v = cm[r * k + c];
                    for t in 0..c {
                        v -= cm[r * k + t] * cm[c * k + t];
                    }
                    cm[r * k + c] = v / dg;
                }
            }
            self.c_chol = cm;
        }
        let _ = n;
        true
    }

    /// `[dx; dy]` を `rhs = [r_x; r_y]` に上書きする (`δ` は直近の分解のもの)。
    pub fn solve_in_place(&mut self, a: &FaerCsr, at: &FaerCsr, rhs: &mut [f64]) {
        use rayon::prelude::*;
        assert!(self.factored, "NormalKkt::solve_in_place before factor");
        let (n, p) = (self.n, self.p);
        let (rx, ry) = rhs.split_at_mut(n);
        // dy の右辺 = A D^{-1} r_x - r_y
        self.tmp_n.par_iter_mut().zip(rx.par_iter()).zip(self.dinv.par_iter()).for_each(|((t, &r), &di)| *t = r * di);
        csr_mat_vec_into(a, &self.tmp_n, &mut self.tmp_p);
        ry.par_iter_mut().zip(self.tmp_p.par_iter()).for_each(|(y, &t)| *y = t - *y);
        self.solve_ms(ry);
        // Woodbury の補正: dy -= W C^{-1} U^T dy0
        let k = self.dense_cols.len();
        if k > 0 {
            let mut g = vec![0.0f64; k];
            for (q, &j) in self.dense_cols.iter().enumerate() {
                let sq = self.dinv[j].sqrt();
                g[q] = sq * self.col_ent[self.col_ptr[j]..self.col_ptr[j + 1]].iter().map(|&(i, v)| v * ry[i]).sum::<f64>();
            }
            // C t = g (C = L L^T)
            let l = &self.c_chol;
            for r in 0..k {
                let mut v = g[r];
                for t in 0..r {
                    v -= l[r * k + t] * g[t];
                }
                g[r] = v / l[r * k + r];
            }
            for r in (0..k).rev() {
                let mut v = g[r];
                for t in r + 1..k {
                    v -= l[t * k + r] * g[t];
                }
                g[r] = v / l[r * k + r];
            }
            for q in 0..k {
                let gq = g[q];
                if gq != 0.0 {
                    let wq = &self.w_mat[q * p..(q + 1) * p];
                    ry.par_iter_mut().zip(wq.par_iter()).for_each(|(y, &w)| *y -= gq * w);
                }
            }
        }
        // dx = D^{-1} (r_x - A^T dy)
        at_mul(a, at, ry, &mut self.tmp_n);
        rx.par_iter_mut().zip(self.tmp_n.par_iter()).zip(self.dinv.par_iter()).for_each(|((x, &t), &di)| *x = (*x - t) * di);
    }
}

/// 内点法の Newton 系 `[diag(top) A^T; A -δI]` の解法: 正規方程式 (既定) か拡大系。
pub enum IpmKkt {
    Normal(NormalKkt),
    Aug(AugKkt),
}

impl IpmKkt {
    /// 正規方程式を作れればそれを、だめ (稠密な列・因子が大きすぎる) なら拡大系を使う。
    /// `ENOMOTO_IPM_AUGMENTED=1` で常に拡大系。どちらも因子の非零数が [`MAX_FACTOR_NNZ`] を超えるなら
    /// `None` (内点法を諦める)。
    pub fn new(a: &FaerCsr) -> Option<Self> {
        if env_str!("ENOMOTO_IPM_AUGMENTED").is_none() {
            if let Some(nk) = NormalKkt::new(a) {
                return Some(IpmKkt::Normal(nk));
            }
        }
        AugKkt::try_new(a, max_factor_nnz()).map(IpmKkt::Aug)
    }

    /// 拡大系 (`A` から新たに作る。正規方程式が停滞したときの切り替え用)。
    pub fn augmented(a: &FaerCsr) -> Self {
        IpmKkt::Aug(AugKkt::new(a))
    }

    pub fn is_normal(&self) -> bool {
        matches!(self, IpmKkt::Normal(_))
    }

    /// 正規方程式で、稠密な列を外して Woodbury で扱っているか。
    pub fn has_dense_cols(&self) -> bool {
        matches!(self, IpmKkt::Normal(k) if !k.dense_cols.is_empty())
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
    /// 戻り値は最後の (改良後の) 相対残差の目安 `‖b - K x‖∞ / max(1, ‖b‖∞)`。
    pub fn solve_refined(&mut self, a: &FaerCsr, at: &FaerCsr, top: &[f64], delta: f64, rhs: &mut [f64], refine: usize, work: &mut Vec<f64>) -> f64 {
        let n = top.len();
        let dim = rhs.len();
        work.resize(3 * dim, 0.0);
        let (b, rest) = work.split_at_mut(dim);
        let (r, kx) = rest.split_at_mut(dim);
        b.copy_from_slice(rhs);
        let bnorm = b.iter().fold(1.0f64, |m, v| m.max(v.abs()));
        self.solve_plain(a, at, rhs);
        let mut prev = f64::INFINITY;
        let mut last_rel = 0.0f64;
        let refine = tunable!("ENOMOTO_T_IPM_REFINE_STEPS", refine, usize);
        for _ in 0..refine {
            // kx = K rhs
            {
                let (vx, vy) = rhs.split_at(n);
                let (ox, oy) = kx.split_at_mut(n);
                at_mul(a, at, vy, ox);
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
            // 残差が十分小さければ追加の求解をしない (以前は 1 回目に必ず解き直していた)。
            // 試験用 `ENOMOTO_T_IPM_REFINE_RATIO`: 前回からこの倍率以下に減らなければやめる (既定 0.5)。
            last_rel = nrm / bnorm;
            if !(nrm < prev * tunable!("ENOMOTO_T_IPM_REFINE_RATIO", 0.5f64, f64)) || nrm <= REFINE_TOL * bnorm {
                break;
            }
            prev = nrm;
            self.solve_plain(a, at, r);
            for i in 0..dim {
                rhs[i] += r[i];
            }
        }
        if env_str!("ENOMOTO_DEBUG_REFINE").is_some() {
            eprintln!("REFINE rel_residual={last_rel:.2e}");
        }
        last_rel
    }

    fn solve_plain(&mut self, a: &FaerCsr, at: &FaerCsr, rhs: &mut [f64]) {
        match self {
            IpmKkt::Normal(k) => k.solve_in_place(a, at, rhs),
            IpmKkt::Aug(k) => k.solve_in_place(rhs),
        }
    }
}

#[cfg(test)]
mod chol_bench {
    use faer::dyn_stack::{GlobalPodBuffer, PodStack};
    use faer::sparse::linalg::cholesky::LltRegularization;
    use faer::sparse::SymbolicSparseColMat;
    use faer::{Parallelism, Side};

    /// 計測用 (`cargo test --release chol_bench -- --ignored --nocapture`): `ENOMOTO_CHOL_BENCH_FILE` の対称行列
    /// (Matrix Market、下三角) を faer の疎 Cholesky (AMD、既定の設定) で分解し、記号分解と数値分解の時間を出す
    /// (他の実装との比較用)。
    #[test]
    #[ignore]
    fn chol_bench() {
        let Ok(path) = std::env::var("ENOMOTO_CHOL_BENCH_FILE") else { return };
        let text = std::fs::read_to_string(&path).unwrap();
        let mut lines = text.lines().filter(|l| !l.starts_with('%'));
        let hdr: Vec<usize> = lines.next().unwrap().split_whitespace().map(|v| v.parse().unwrap()).collect();
        let n = hdr[0];
        // 上三角 (列優先) にする: 下三角の (i, j) (i >= j) を (j, i) に。
        let mut cols: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
        for l in lines {
            let mut it = l.split_whitespace();
            let i: usize = it.next().unwrap().parse::<usize>().unwrap() - 1;
            let j: usize = it.next().unwrap().parse::<usize>().unwrap() - 1;
            let v: f64 = it.next().unwrap().parse().unwrap();
            let (r, c) = if i <= j { (i, j) } else { (j, i) };
            cols[c].push((r, v));
        }
        let mut ptr = vec![0usize; n + 1];
        let mut idx = Vec::new();
        let mut val = Vec::new();
        for c in 0..n {
            cols[c].sort_by_key(|e| e.0);
            for &(r, v) in &cols[c] {
                idx.push(r);
                val.push(v);
            }
            ptr[c + 1] = idx.len();
        }
        let sym = SymbolicSparseColMat::<usize>::new_checked(n, n, ptr, None, idx);
        let t0 = std::time::Instant::now();
        let mf_mode = std::env::var("ENOMOTO_T_CHOL_BACKEND").map_or(false, |v| v == "2");
        super::FORCE_SUPERNODAL.with(|c| c.set(mf_mode));
        let chol = super::symbolic_with_ordering(&sym, false, usize::MAX).unwrap();
        super::FORCE_SUPERNODAL.with(|c| c.set(false));
        if mf_mode {
            let t0 = std::time::Instant::now();
            let mut mf = super::super::multifrontal::Multifrontal::new(&sym, &chol).unwrap();
            let t_sym = t0.elapsed().as_secs_f64();
            let mut best = f64::INFINITY;
            for _ in 0..3 {
                let t = std::time::Instant::now();
                assert!(mf.factor(&val, 1e-8, 1e-14));
                best = best.min(t.elapsed().as_secs_f64());
            }
            // 残差: M x = 1 (上三角の列圧縮から対称に掛ける)。
            let b = vec![1.0f64; n];
            let mut x = b.clone();
            mf.solve_in_place(&mut x);
            let mut mx = vec![0.0f64; n];
            let (cp, ri) = (sym.col_ptrs(), sym.row_indices());
            for c in 0..n {
                for k in cp[c]..cp[c + 1] {
                    let r = ri[k];
                    mx[r] += val[k] * x[c];
                    if r != c {
                        mx[c] += val[k] * x[r];
                    }
                }
            }
            let res = mx.iter().zip(&b).map(|(a, b)| (a - b).abs()).fold(0.0f64, f64::max);
            let _ = super::super::multifrontal::take_prof();
            let t = std::time::Instant::now();
            mf.factor(&val, 1e-8, 1e-14);
            println!("one={:.3}s prof(asm+ea, chol, trsm, syrk)={:?}", t.elapsed().as_secs_f64(), super::super::multifrontal::take_prof());
            println!("CHOLBENCH multifrontal threads={} n={n} ns={} nnz(L)={} setup={t_sym:.3}s numeric={best:.3}s resid={res:.2e}", rayon::current_num_threads(), mf.n_supernodes(), mf.len_values());
            return;
        }
        let t_sym = t0.elapsed().as_secs_f64();
        let mat = faer::sparse::SparseColMatRef::<usize, f64>::new(sym.as_ref(), &val);
        for (name, par) in [("seq", Parallelism::None), ("par", Parallelism::Rayon(0))] {
            let mut l_values = vec![0.0f64; chol.len_values()];
            let mut buf = GlobalPodBuffer::new(chol.factorize_numeric_llt_req::<f64>(par).unwrap());
            let mut best = f64::INFINITY;
            for _ in 0..3 {
                let t = std::time::Instant::now();
                chol.factorize_numeric_llt::<f64>(&mut l_values, mat, Side::Upper, LltRegularization::default(), par, PodStack::new(&mut buf)).unwrap();
                best = best.min(t.elapsed().as_secs_f64());
            }
            let kind = match chol.raw() {
                faer::sparse::linalg::cholesky::SymbolicCholeskyRaw::Supernodal(_) => "supernodal",
                _ => "simplicial",
            };
            println!("CHOLBENCH faer {name} n={n} nnz(L)={} kind={kind} symbolic={t_sym:.3}s numeric={best:.3}s", chol.len_values());
        }
    }
}
