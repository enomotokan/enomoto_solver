//! 消去木の枝ごとに並列に分解するマルチフロンタル法の疎 Cholesky 分解 (内点法の正規方程式の既定。`ENOMOTO_T_CHOL_BACKEND=0` で faer)。
//!
//! faer の記号分解 (並べ替えと supernode の区切り・行の形) をそのまま使い、数値分解だけを自前で行う。
//! supernode `s` (列 `[b, e)`、その下の行の形 `pattern`) ごとに、前線行列 `F` (大きさ `nc + r`) に元の行列の値と
//! 子の更新行列を足し込み (extend-add)、密な部分 Cholesky
//! (`F11 = L11 L11ᵀ`、`L21 = F21 L11^{-T}`、`U = F22 - L21 L21ᵀ`) をして `U` を親に渡す。
//! 独立な子の部分木は rayon で並列に処理し、密な演算は faer の行列演算 (Cholesky・三角行列の解法・行列積) を使う。
//! 演算量の小さな部分木は 1 つの仕事として逐次に処理する (仕事を細かく分けすぎない)。

use faer::linalg::cholesky::llt::compute::{cholesky_in_place, cholesky_in_place_req, LltParams, LltRegularization};
use faer::linalg::matmul::triangular::BlockStructure;
use faer::mat::{from_column_major_slice, from_column_major_slice_mut, from_row_major_slice, from_row_major_slice_mut};
use faer::sparse::linalg::cholesky::{SymbolicCholesky, SymbolicCholeskyRaw};
use faer::sparse::SymbolicSparseColMat;
use faer::{MatRef, Parallelism, dyn_stack::{GlobalPodBuffer, PodStack}};
use rayon::prelude::*;
use faer::reborrow::{Reborrow, ReborrowMut};

/// マルチフロンタル法の記号情報と因子。
pub struct Multifrontal {
    n: usize,
    /// `perm_fwd[新] = 旧`、`perm_inv[旧] = 新`。
    perm_fwd: Vec<usize>,
    perm_inv: Vec<usize>,
    begin: Vec<usize>,
    end: Vec<usize>,
    /// supernode `s` の下の行の形 (新しい番号、昇順) は `pat[pat_ptr[s]..pat_ptr[s+1]]`。
    pat_ptr: Vec<usize>,
    pat: Vec<usize>,
    children: Vec<Vec<usize>>,
    roots: Vec<usize>,
    /// 元の値の位置 `k` を、どの supernode の前線のどこ (行, 列) に足すか。supernode ごとにまとめる。
    asm: Vec<Vec<(u32, u32, u32)>>,
    /// 子 `c` の下の行が、親の前線のどの行に当たるか。
    child_map: Vec<Vec<u32>>,
    /// 部分木の演算量の見積もり。
    subtree_flops: Vec<f64>,
    /// 因子の supernode `s` の列のかたまり (`(nc + r) x nc`、列優先) の位置。
    l_ptr: Vec<usize>,
    l: Vec<f64>,
    /// 逐次に処理する部分木の演算量の上限。
    seq_flops: f64,
    /// extend-add を並列にする子の更新行列の大きさ (要素数) の下限。
    ea_par: f64,
    /// LDLᵀ で分解するときのピボットの期待する符号 (新しい番号、上段 +1・下段 -1)。`None` なら LLᵀ。
    signs: Option<Vec<i8>>,
    /// 更新行列の配列の使い回し (確保と初めて触るときのページフォールトを減らす)。
    pool: std::sync::Mutex<Vec<Vec<f64>>>,
    /// 計測用 (`ENOMOTO_DEBUG_MF_TRACE`): 直近の分解での supernode ごとの所要時間 (ナノ秒、前線の用意を含む)。
    trace: Option<Vec<std::sync::atomic::AtomicU64>>,
    /// 更新行列を台形で持つときのかたまりの幅 ([`Front`]、0 なら正方形)。既定で台形 (`ENOMOTO_T_MF_PACKED`、既定 1。0 で
    /// 正方形)、幅 `ENOMOTO_T_MF_PACK_BW` (既定 128)。2026-10-13 の比較 (Mittelmann 10 問、`benchmarks/mf_bench/ab2_*`):
    /// 1 スレッドで分解の累計 0.927 倍、最大常駐メモリ 0.51〜1.00 倍。
    pack_bw: usize,
    /// 逐次に処理する部分木で、子の更新行列を受けていない節点の更新行列を親の前線へ直接書く (試験用 `ENOMOTO_T_MF_DIRECT=1`)。
    direct: bool,
    /// 直接書くのは、行の写像が連続な範囲がこれ以下のとき (`ENOMOTO_T_MF_DIRECT_RUNS`)。
    direct_runs: usize,
    /// 直接書くのは、下の行の数がこれ以上のとき (`ENOMOTO_T_MF_DIRECT_MIN_R`)。
    direct_min_r: usize,
    /// 更新行列の計算を区切る列の幅 (`ENOMOTO_T_MF_CHUNK`)。
    chunk: usize,
    /// 前線の密な演算を並列にする演算量 (`nc f^2`) の下限 (`ENOMOTO_T_MF_PAR_FLOPS`)。
    par_flops: f64,
    /// 並列に計算する大きな前線の対角ブロックの Cholesky を、この幅のかたまりごとに行う ([`chol_blocked`]、0 なら faer
    /// のまま)。既定で使う (`ENOMOTO_T_MF_BCHOL`、既定 1。0 で faer のまま)、幅 `ENOMOTO_T_MF_BCHOL_BS` (既定 256)。
    /// 4 スレッドの比較で台形 + 併合に対し分解の累計 0.968 倍 (qap15 0.92、rail4284 0.87)。1 スレッドでは faer と同じ。
    bchol_bs: usize,
    /// [`chol_blocked`] を使う対角ブロックの大きさの下限 (`ENOMOTO_T_MF_BCHOL_MIN`)。
    bchol_min: usize,
    /// 並列に計算する大きな前線の三角行列の解法を、L21 の行のかたまりごとに並列に行う ([`trsm_rows`]、試験用
    /// `ENOMOTO_T_MF_PAR_TRSM=1`)。
    par_trsm: bool,
    /// 並列に計算する大きな前線の L のかたまりの 0 埋めを並列に行う (試験用 `ENOMOTO_T_MF_PAR_PREP=1`)。
    par_prep: bool,
}

static PROF: [std::sync::atomic::AtomicU64; 4] = [const { std::sync::atomic::AtomicU64::new(0) }; 4];
fn prof_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("ENOMOTO_DEBUG_MF").is_ok())
}
/// 計測用: 各段階の時間の合計 (秒、スレッドの和) を返して 0 に戻す。
pub fn take_prof() -> [f64; 4] {
    let mut o = [0.0; 4];
    for i in 0..4 {
        o[i] = PROF[i].swap(0, std::sync::atomic::Ordering::Relaxed) as f64 * 1e-9;
    }
    o
}

/// 計測用: 前線の大きさ `f` で分けた区分 (`f` < 32, < 128, < 512, < 2048, それ以上、並列に計算した大きな前線) ごとの
/// 節点の数・各段階の時間 (ナノ秒、組み立てと extend-add・対角ブロックの分解・三角行列の解法・更新行列) と演算量
/// (分解 `nc^3/3`・三角行列の解法 `nc^2 r`・更新行列 `nc r^2`)。
pub const N_BUCKET: usize = 6;
static BUCKET_N: [std::sync::atomic::AtomicU64; N_BUCKET] = [const { std::sync::atomic::AtomicU64::new(0) }; N_BUCKET];
static BUCKET_T: [[std::sync::atomic::AtomicU64; 4]; N_BUCKET] = [const { [const { std::sync::atomic::AtomicU64::new(0) }; 4] }; N_BUCKET];
static BUCKET_F: [[std::sync::atomic::AtomicU64; 3]; N_BUCKET] = [const { [const { std::sync::atomic::AtomicU64::new(0) }; 3] }; N_BUCKET];

/// 計測用: 区分ごとの (節点の数, 各段階の秒, 各段階の演算量) を返して 0 に戻す。
pub fn take_bucket_prof() -> Vec<(u64, [f64; 4], [f64; 3])> {
    use std::sync::atomic::Ordering::Relaxed;
    (0..N_BUCKET)
        .map(|b| {
            let n = BUCKET_N[b].swap(0, Relaxed);
            let t = std::array::from_fn(|i| BUCKET_T[b][i].swap(0, Relaxed) as f64 * 1e-9);
            let f = std::array::from_fn(|i| BUCKET_F[b][i].swap(0, Relaxed) as f64);
            (n, t, f)
        })
        .collect()
}

/// 細い supernode (列が少なく下の行が多い) を親へまとめる。子 `c` を親 `p` にまとめると、`c` の更新行列 (`r_c x r_c`)
/// の extend-add と確保がなくなる代わりに、`c` の列が親の前線の大きさで消去される (0 の分の演算が増える)。
/// 増える演算量が `alpha * r_c (r_c + 1) / 2` より小さければまとめる。まとめた節点の列が並ぶように番号を付け直す
/// (木の後順。どの後順でも fill は変わらない)。
fn amalgamate(alpha: f64, perm_fwd: &mut Vec<usize>, perm_inv: &mut Vec<usize>, begin: &mut Vec<usize>, end: &mut Vec<usize>, pat_ptr: &mut Vec<usize>, pat: &mut Vec<usize>) {
    let ns = begin.len();
    let n = perm_fwd.len();
    let mut col_sn = vec![0usize; n];
    for k in 0..ns {
        for j in begin[k]..end[k] {
            col_sn[j] = k;
        }
    }
    let mut parent = vec![usize::MAX; ns];
    let mut children = vec![Vec::new(); ns];
    for k in 0..ns {
        if pat_ptr[k + 1] > pat_ptr[k] {
            parent[k] = col_sn[pat[pat_ptr[k]]];
            children[parent[k]].push(k);
        }
    }
    // 消去の演算量: 前線の大きさ f で nc 列を消すと Σ_{i<nc} (f - i)^2。
    let fl = |nc: usize, f: usize| -> f64 { (0..nc).map(|i| ((f - i) as f64).powi(2)).sum() };
    let r: Vec<usize> = (0..ns).map(|k| pat_ptr[k + 1] - pat_ptr[k]).collect();
    let mut nc: Vec<usize> = (0..ns).map(|k| end[k] - begin[k]).collect();
    // members[k]: まとめた元の supernode (k 自身を含む)。merged[k]: まとめられて消えたか。
    let mut members: Vec<Vec<usize>> = (0..ns).map(|k| vec![k]).collect();
    let mut merged = vec![false; ns];
    let mut new_children: Vec<Vec<usize>> = vec![Vec::new(); ns];
    for p in 0..ns {
        let mut queue: Vec<usize> = children[p].clone();
        let mut kept = Vec::new();
        while let Some(c) = queue.pop() {
            let f_m = nc[c] + nc[p] + r[p];
            let extra = fl(nc[c], f_m) - fl(nc[c], nc[c] + r[c]);
            let saved = alpha * (r[c] as f64) * (r[c] as f64 + 1.0) * 0.5;
            if extra < saved {
                merged[c] = true;
                nc[p] += nc[c];
                let m = std::mem::take(&mut members[c]);
                members[p].extend(m);
                queue.extend(std::mem::take(&mut new_children[c]));
            } else {
                kept.push(c);
            }
        }
        new_children[p] = kept;
    }
    // 新しい木の後順に、各節点の元の列 (faer の番号の昇順) を並べる。
    let mut newpos = vec![0usize; n];
    let mut nb = Vec::new();
    let mut ne = Vec::new();
    let mut top = Vec::new();
    let mut next = 0usize;
    let roots: Vec<usize> = (0..ns).filter(|&k| !merged[k] && parent[k] == usize::MAX).collect();
    for &rt in &roots {
        let mut stack: Vec<(usize, usize)> = vec![(rt, 0)];
        while let Some(&(node, ci)) = stack.last() {
            if ci < new_children[node].len() {
                stack.last_mut().unwrap().1 += 1;
                stack.push((new_children[node][ci], 0));
                continue;
            }
            stack.pop();
            let mut cols: Vec<usize> = members[node].iter().flat_map(|&m| begin[m]..end[m]).collect();
            cols.sort_unstable();
            nb.push(next);
            for c in cols {
                newpos[c] = next;
                next += 1;
            }
            ne.push(next);
            top.push(node);
        }
    }
    debug_assert_eq!(next, n);
    let mut np_ptr = vec![0usize; top.len() + 1];
    let mut np = Vec::with_capacity(pat.len());
    for (k, &node) in top.iter().enumerate() {
        let st = np.len();
        np.extend(pat[pat_ptr[node]..pat_ptr[node + 1]].iter().map(|&i| newpos[i]));
        np[st..].sort_unstable();
        np_ptr[k + 1] = np.len();
    }
    let mut pf = vec![0usize; n];
    for j in 0..n {
        pf[newpos[j]] = perm_fwd[j];
    }
    for (i, &o) in pf.iter().enumerate() {
        perm_inv[o] = i;
    }
    *perm_fwd = pf;
    *begin = nb;
    *end = ne;
    *pat_ptr = np_ptr;
    *pat = np;
}

/// 計測用: 分解中に置き換えたピボットの数。
pub static DYNREG: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// 使い回す配列の大きさの下限。
const POOL_MIN: usize = 1 << 14;

/// 書き込み先が互いに重ならないことが分かっている配列への生ポインタ (並列に別々の範囲へ書く)。
#[derive(Clone, Copy)]
struct SyncPtr(*mut f64);
unsafe impl Send for SyncPtr {}
unsafe impl Sync for SyncPtr {}
impl SyncPtr {
    fn get(self) -> *mut f64 {
        self.0
    }
}

/// 前線の書き込み先: 左の `nc` 列 (L のかたまり、`f x nc` 列優先、因子の配列の中) と、右下の更新行列 `U` (`r x r`、
/// 下三角だけを使う)。`U` は `bw == 0` なら正方形の列優先、`bw > 0` なら幅 `bw` の列のかたまりごとの台形
/// (かたまり `k` は列 `[k bw, (k+1) bw)` の行 `[k bw, r)` だけを列優先で持ち、かたまりを順に並べる)。
#[derive(Clone, Copy)]
struct Front {
    l: SyncPtr,
    f: usize,
    nc: usize,
    u: SyncPtr,
    r: usize,
    bw: usize,
}

/// 大きさ `r` の更新行列の配列の長さ (`bw` は [`Front`] と同じ)。
fn u_len(r: usize, bw: usize) -> usize {
    if bw == 0 || r <= bw {
        return r * r;
    }
    // 幅 bw のかたまり k 個 (高さ r, r - bw, ...) と、残りの rem 列 (高さ rem)。
    let k = r / bw;
    let rem = r - k * bw;
    bw * (k * r - bw * (k * (k - 1) / 2)) + rem * rem
}

impl Front {
    /// 更新行列だけの前線 (子の更新行列の配列)。
    fn u_only(u: *mut f64, r: usize, bw: usize) -> Self {
        Front { l: SyncPtr(std::ptr::null_mut()), f: 0, nc: 0, u: SyncPtr(u), r, bw }
    }

    /// 前線の列 `j` の対角 `(j, j)` の位置と、その列を含む部分の列の間隔 (列 `j` の行 `i >= j` は `位置 + (i - j)`)。
    fn col(&self, j: usize) -> (*mut f64, usize) {
        if j < self.nc {
            return (self.l.get().wrapping_add(j * self.f + j), self.f);
        }
        let c = j - self.nc;
        if self.bw == 0 {
            return (self.u.get().wrapping_add(c * self.r + c), self.r);
        }
        let k = c / self.bw;
        let j0 = k * self.bw;
        let h = self.r - j0;
        let off = self.bw * (k * self.r - self.bw * (k * k.saturating_sub(1) / 2));
        (self.u.get().wrapping_add(off + (c - j0) * (h + 1)), h)
    }

    /// 列 `j` と同じ間隔で並ぶ列の範囲の終わり。
    fn region_end(&self, j: usize) -> usize {
        if j < self.nc {
            self.nc
        } else if self.bw == 0 {
            self.nc + self.r
        } else {
            (self.nc + ((j - self.nc) / self.bw + 1) * self.bw).min(self.nc + self.r)
        }
    }
}

/// 対角ブロック `a` (`n x n`、下三角を使う) の Cholesky 分解を、幅 `bs` の列のかたまりごとに左から順に行う
/// (かたまりの対角の `bs x bs` は逐次に分解し、その下の三角行列の解法と右下の残りの更新は `par` の行列演算)。
/// faer の Cholesky は並列にしても伸びない (4 コアで `n = 2048` が 49 GFLOPS、`n = 512` は逐次より遅い。行列積は
/// 100 GFLOPS を超える) ので、大きな前線で使う。成功なら置き換えたピボットの数。
fn chol_blocked(mut a: faer::MatMut<'_, f64>, reg: LltRegularization<f64>, bs: usize, par: Parallelism) -> Result<usize, ()> {
    let n = a.nrows();
    let bs = bs.max(1);
    let mut buf = GlobalPodBuffer::new(cholesky_in_place_req::<f64>(bs.min(n), Parallelism::None, LltParams::default()).unwrap());
    let mut dynreg = 0;
    let mut k0 = 0;
    while k0 < n {
        let kb = bs.min(n - k0);
        let (mut a11, _, mut a21, a22) = a.rb_mut().submatrix_mut(k0, k0, n - k0, n - k0).split_at_mut(kb, kb);
        match cholesky_in_place(a11.rb_mut(), reg, Parallelism::None, PodStack::new(&mut buf), LltParams::default()) {
            Ok(info) => dynreg += info.dynamic_regularization_count,
            Err(_) => return Err(()),
        }
        if k0 + kb < n {
            faer::linalg::triangular_solve::solve_lower_triangular_in_place(a11.rb(), a21.rb_mut().transpose_mut(), par);
            faer::linalg::matmul::triangular::matmul(a22, BlockStructure::TriangularLower, a21.rb(), BlockStructure::Rectangular, a21.rb().transpose(), BlockStructure::Rectangular, Some(1.0), -1.0, par);
        }
        k0 += kb;
    }
    Ok(dynreg)
}

/// `L21 = F21 L11^{-T}` (`unit` なら単位下三角の `L11`) を、`L21` の行のかたまり (`rows` 行ずつ) ごとに並列に解く
/// (行どうしは独立。faer の並列の三角行列の解法は 4 コアで 20〜50 GFLOPS しか出ない)。`w` (LDLᵀ: `W` の配列
/// `r x nc` 列優先と、`D11` の逆数) があれば、解いた行を `W` に写してから `L21` を `D11` で割る。
fn trsm_rows(l11: MatRef<'_, f64>, mut l21: faer::MatMut<'_, f64>, unit: bool, w: Option<(SyncPtr, &[f64])>, rows: usize) {
    let r = l21.nrows();
    let nc = l21.ncols();
    let mut blocks = Vec::new();
    let mut start = 0;
    while start < r {
        let h = rows.min(r - start);
        let (top, rest) = l21.split_at_row_mut(h);
        blocks.push((start, top));
        l21 = rest;
        start += h;
    }
    blocks.into_par_iter().for_each(|(st, mut blk)| {
        if unit {
            faer::linalg::triangular_solve::solve_unit_lower_triangular_in_place(l11, blk.rb_mut().transpose_mut(), Parallelism::None);
        } else {
            faer::linalg::triangular_solve::solve_lower_triangular_in_place(l11, blk.rb_mut().transpose_mut(), Parallelism::None);
        }
        if let Some((wp, dinv)) = w {
            for j in 0..nc {
                for i in 0..blk.nrows() {
                    let v = blk.read(i, j);
                    // SAFETY: W の (st + i, j) はこのかたまりだけが書く。
                    unsafe { *wp.get().add(j * r + st + i) = v };
                    blk.write(i, j, v * dinv[j]);
                }
            }
        }
    });
}

/// 行の写像 `map` (昇順) が連続な範囲の数。
fn map_runs(map: &[u32]) -> usize {
    1 + map.windows(2).filter(|w| w[1] != w[0] + 1).count()
}

/// 前線 `dst` の下三角に `-A Bᵀ` を足す (`A`・`B` は `m x k`、行 `t` は前線の行 `map[t]`、`map` が `None` なら `t`。
/// `map` は昇順)。行の写像が連続な範囲・列の間隔が同じ列の範囲・幅 `cw` で列を区切り、区切りごとに書き先の部分行列へ
/// 行列積を直接書く (対角のかたまりは下三角だけ)。`big` なら並列に計算する。
fn sub_update(dst: Front, map: Option<&[u32]>, a: MatRef<'_, f64>, b: MatRef<'_, f64>, cw: usize, big: bool) {
    let m = a.nrows();
    let idx = |t: usize| map.map_or(t, |mp| mp[t] as usize);
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut t = 0;
    while t < m {
        let t0 = t;
        t += 1;
        while t < m && idx(t) == idx(t - 1) + 1 {
            t += 1;
        }
        runs.push((t0, t));
    }
    // 列の区切り (子の行の範囲 [ca, cb)、それを含む連続な範囲の番号)。
    let mut chunks: Vec<(usize, usize, usize)> = Vec::new();
    for (ri, &(t0, t1)) in runs.iter().enumerate() {
        let mut t = t0;
        while t < t1 {
            let e = (dst.region_end(idx(t)) - idx(t) + t).min(t1).min(t.saturating_add(cw));
            chunks.push((t, e, ri));
            t = e;
        }
    }
    let one = |&(ca, cb, ri): &(usize, usize, usize), par: Parallelism| {
        let j = idx(ca);
        let (p, ld) = dst.col(j);
        let w = cb - ca;
        let bt = b.subrows(ca, w).transpose();
        let block = |ra: usize, rb: usize, st: BlockStructure| {
            // SAFETY: 書き先は前線の列 [j, j + w) (同じ間隔で並ぶ部分の中) の行 idx(ra)..idx(rb - 1) で、区切りごとに
            // 列が重ならない。
            let d = unsafe { faer::mat::from_raw_parts_mut::<f64, usize, usize>(p.wrapping_add(idx(ra) - j), rb - ra, w, 1, ld as isize) };
            faer::linalg::matmul::triangular::matmul(d, st, a.subrows(ra, rb - ra), BlockStructure::Rectangular, bt, BlockStructure::Rectangular, Some(1.0), -1.0, par);
        };
        block(ca, cb, BlockStructure::TriangularLower);
        let run_end = runs[ri].1;
        if cb < run_end {
            block(cb, run_end, BlockStructure::Rectangular);
        }
        for &(t0, t1) in &runs[ri + 1..] {
            block(t0, t1, BlockStructure::Rectangular);
        }
    };
    let seq = super::kkt::inner_seq();
    if big && !seq && chunks.len() >= 4 {
        chunks.par_iter().for_each(|c| one(c, Parallelism::None));
    } else {
        let par = if big && !seq { Parallelism::Rayon(0) } else { Parallelism::None };
        for c in &chunks {
            one(c, par);
        }
    }
}

impl Multifrontal {
    /// 上三角の列圧縮の非零の形 `pat_in` と、faer の supernodal の記号分解 `sym` から作る。supernodal でなければ `None`。
    pub fn new(pat_in: &SymbolicSparseColMat<usize>, sym: &SymbolicCholesky<usize>) -> Option<Self> {
        let SymbolicCholeskyRaw::Supernodal(s) = sym.raw() else { return None };
        let n = pat_in.nrows();
        let (perm_fwd, perm_inv): (Vec<usize>, Vec<usize>) = match sym.perm() {
            Some(p) => {
                let (f, i) = p.arrays();
                (f.to_vec(), i.to_vec())
            }
            None => ((0..n).collect(), (0..n).collect()),
        };
        let ns = s.n_supernodes();
        let mut begin: Vec<usize> = s.supernode_begin()[..ns].to_vec();
        let mut end: Vec<usize> = s.supernode_end()[..ns].to_vec();
        let mut pat_ptr = vec![0usize; ns + 1];
        let mut pat = Vec::new();
        for k in 0..ns {
            pat.extend_from_slice(s.supernode(k).pattern());
            pat_ptr[k + 1] = pat.len();
        }
        let (mut perm_fwd, mut perm_inv) = (perm_fwd, perm_inv);
        // 細い supernode の併合 (`ENOMOTO_T_MF_AMALG`、既定 64、0 で併合しない)。2026-10-13 の比較 (台形と合わせて、
        // Mittelmann 10 問): 1 スレッドで分解の累計 0.771 倍 (s250r10 0.389、square41 0.377、nug08-3rd 0.830)、
        // 4 スレッドで 0.860 倍。alpha は 16・32・64・128 を試して 64 が最も速かった (s250r10・square41)。
        let alpha = tunable!("ENOMOTO_T_MF_AMALG", 64.0f64, f64);
        if alpha > 0.0 {
            amalgamate(alpha, &mut perm_fwd, &mut perm_inv, &mut begin, &mut end, &mut pat_ptr, &mut pat);
        }
        let ns = begin.len();
        let mut col_sn = vec![0usize; n];
        for k in 0..ns {
            for j in begin[k]..end[k] {
                col_sn[j] = k;
            }
        }
        let mut parent = vec![usize::MAX; ns];
        let mut children = vec![Vec::new(); ns];
        let mut roots = Vec::new();
        for k in 0..ns {
            if pat_ptr[k + 1] > pat_ptr[k] {
                let p = col_sn[pat[pat_ptr[k]]];
                parent[k] = p;
                children[p].push(k);
            } else {
                roots.push(k);
            }
        }
        // 前線の局所の行番号: 列 [b, e) は i - b、下の行は nc + (pattern の中の位置)。
        let local = |k: usize, i: usize| -> u32 {
            if i < end[k] {
                (i - begin[k]) as u32
            } else {
                let p = &pat[pat_ptr[k]..pat_ptr[k + 1]];
                ((end[k] - begin[k]) + p.binary_search(&i).expect("row in the supernode pattern")) as u32
            }
        };
        let mut asm = vec![Vec::new(); ns];
        let cp = pat_in.col_ptrs();
        let ri = pat_in.row_indices();
        for c in 0..n {
            for k in cp[c]..cp[c + 1] {
                let (pr, pc) = (perm_inv[ri[k]], perm_inv[c]);
                let (i, j) = if pr >= pc { (pr, pc) } else { (pc, pr) };
                let sn = col_sn[j];
                asm[sn].push((k as u32, local(sn, i), (j - begin[sn]) as u32));
            }
        }
        let mut child_map = vec![Vec::new(); ns];
        for k in 0..ns {
            if parent[k] != usize::MAX {
                let p = parent[k];
                child_map[k] = pat[pat_ptr[k]..pat_ptr[k + 1]].iter().map(|&i| local(p, i)).collect();
            }
        }
        let mut l_ptr = vec![0usize; ns + 1];
        let mut flops = vec![0.0f64; ns];
        for k in 0..ns {
            let nc = end[k] - begin[k];
            let f = nc + pat_ptr[k + 1] - pat_ptr[k];
            l_ptr[k + 1] = l_ptr[k] + f * nc;
            flops[k] = nc as f64 * (f as f64) * (f as f64);
        }
        // 子は親より小さい番号なので、番号の順に足せば部分木の和になる。
        let mut subtree_flops = flops;
        for k in 0..ns {
            if parent[k] != usize::MAX {
                let v = subtree_flops[k];
                subtree_flops[parent[k]] += v;
            }
        }
        let total = l_ptr[ns];
        Some(Multifrontal {
            n,
            perm_fwd,
            perm_inv,
            begin,
            end,
            pat_ptr,
            pat,
            children,
            roots,
            asm,
            child_map,
            subtree_flops,
            l_ptr,
            l: vec![0.0; total],
            seq_flops: tunable!("ENOMOTO_T_MF_SEQ_FLOPS", 2e6f64, f64),
            ea_par: tunable!("ENOMOTO_T_MF_EA_PAR", 1e6f64, f64),
            pool: std::sync::Mutex::new(Vec::new()),
            trace: std::env::var("ENOMOTO_DEBUG_MF_TRACE").is_ok().then(|| (0..ns).map(|_| std::sync::atomic::AtomicU64::new(0)).collect()),
            signs: None,
            pack_bw: if tunable!("ENOMOTO_T_MF_PACKED", 1u8, u8) != 0 { tunable!("ENOMOTO_T_MF_PACK_BW", 128usize, usize).max(1) } else { 0 },
            direct: tunable!("ENOMOTO_T_MF_DIRECT", 0u8, u8) != 0,
            direct_runs: tunable!("ENOMOTO_T_MF_DIRECT_RUNS", 4usize, usize),
            direct_min_r: tunable!("ENOMOTO_T_MF_DIRECT_MIN_R", 0usize, usize),
            chunk: tunable!("ENOMOTO_T_MF_CHUNK", 128usize, usize).max(1),
            par_flops: tunable!("ENOMOTO_T_MF_PAR_FLOPS", 5e7f64, f64),
            bchol_bs: if tunable!("ENOMOTO_T_MF_BCHOL", 1u8, u8) != 0 { tunable!("ENOMOTO_T_MF_BCHOL_BS", 256usize, usize).max(1) } else { 0 },
            bchol_min: tunable!("ENOMOTO_T_MF_BCHOL_MIN", 512usize, usize),
            par_trsm: tunable!("ENOMOTO_T_MF_PAR_TRSM", 0u8, u8) != 0,
            par_prep: tunable!("ENOMOTO_T_MF_PAR_PREP", 0u8, u8) != 0,
        })
    }

    /// 長さ `len` の 0 の配列 (使い回しがあればそれを使う)。
    fn take_buf(&self, len: usize) -> Vec<f64> {
        if len >= POOL_MIN {
            let mut p = self.pool.lock().unwrap();
            // 容量が足りるもののうち最小のもの。
            if let Some((i, _)) = p.iter().enumerate().filter(|(_, b)| b.capacity() >= len).min_by_key(|(_, b)| b.capacity()) {
                let mut b = p.swap_remove(i);
                drop(p);
                b.clear();
                b.resize(len, 0.0);
                return b;
            }
        }
        vec![0.0; len]
    }

    fn give_back(&self, b: Vec<f64>) {
        if b.capacity() >= POOL_MIN {
            self.pool.lock().unwrap().push(b);
        }
    }

    /// [`Multifrontal::new`] と同じだが、準定値の行列を LDLᵀ で分解する。`signs_orig` は元の番号でのピボットの期待する
    /// 符号 (動的正則化で、符号が合わない・小さいピボットを `±delta` に置き換える)。
    pub fn new_ldlt(pat_in: &SymbolicSparseColMat<usize>, sym: &SymbolicCholesky<usize>, signs_orig: &[i8]) -> Option<Self> {
        let mut mf = Self::new(pat_in, sym)?;
        mf.signs = Some(mf.perm_fwd.iter().map(|&o| signs_orig[o]).collect());
        Some(mf)
    }

    /// 試験用: 更新行列の形・直接書くか・区切りの幅・並列にする演算量の下限・逐次に処理する部分木の演算量の上限を変える。
    #[cfg(test)]
    pub fn set_variant(&mut self, pack_bw: usize, direct: bool, direct_runs: usize, chunk: usize, par_flops: f64, seq_flops: f64) {
        self.pack_bw = pack_bw;
        self.direct = direct;
        self.direct_runs = direct_runs;
        self.chunk = chunk;
        self.par_flops = par_flops;
        self.seq_flops = seq_flops;
    }

    /// [`trsm_rows`] の行のかたまりの大きさ (スレッドあたり 4 つ程度、32 行以上)。
    fn trsm_rows(&self, r: usize) -> usize {
        r.div_ceil(4 * rayon::current_num_threads()).max(32)
    }

    /// 試験用: 大きな前線の三角行列の解法と L のかたまりの 0 埋めを並列に行うか。
    #[cfg(test)]
    pub fn set_par_extra(&mut self, par_trsm: bool, par_prep: bool) {
        self.par_trsm = par_trsm;
        self.par_prep = par_prep;
    }

    /// 試験用: 大きな前線の対角ブロックの Cholesky をかたまりごとに行う幅 (0 なら faer のまま) と、使う大きさの下限。
    #[cfg(test)]
    pub fn set_bchol(&mut self, bs: usize, min: usize) {
        self.bchol_bs = bs;
        self.bchol_min = min;
    }

    /// 試験用: 下の行の数の最大・行の写像が飛び飛びの子の数・葉の数。
    #[cfg(test)]
    pub fn shape_stats(&self) -> (usize, usize, usize) {
        let ns = self.begin.len();
        let max_r = (0..ns).map(|s| self.pat_ptr[s + 1] - self.pat_ptr[s]).max().unwrap_or(0);
        let multi = self.child_map.iter().filter(|m| !m.is_empty() && map_runs(m) > 1).count();
        let leaves = self.children.iter().filter(|c| c.is_empty()).count();
        (max_r, multi, leaves)
    }

    /// 計測用 (`ENOMOTO_DEBUG_MF_TRACE`): 直近の分解の supernode ごとの所要時間から、(仕事の合計, 消去木の最長経路,
    /// 並列に計算した大きな前線の時間の合計, 最長経路のうち大きな前線の時間) を秒で返す。
    pub fn trace_summary(&self) -> Option<(f64, f64, f64, f64)> {
        use std::sync::atomic::Ordering::Relaxed;
        let tr = self.trace.as_ref()?;
        let ns = self.begin.len();
        let big = |s: usize| {
            let nc = (self.end[s] - self.begin[s]) as f64;
            let f = nc + (self.pat_ptr[s + 1] - self.pat_ptr[s]) as f64;
            nc * f * f > self.par_flops
        };
        // 子は親より小さい番号なので、番号の順に最長経路を伸ばせる。
        let mut cp = vec![0.0f64; ns];
        let mut cpb = vec![0.0f64; ns];
        let (mut work, mut bigsum) = (0.0, 0.0);
        for s in 0..ns {
            let d = tr[s].load(Relaxed) as f64 * 1e-9;
            work += d;
            if big(s) {
                bigsum += d;
            }
            let (c, cb) = self.children[s].iter().map(|&c| (cp[c], cpb[c])).fold((0.0, 0.0), |m, x| if x.0 > m.0 { x } else { m });
            cp[s] = c + d;
            cpb[s] = cb + if big(s) { d } else { 0.0 };
        }
        let (crit, critb) = self.roots.iter().map(|&r| (cp[r], cpb[r])).fold((0.0, 0.0), |m, x| if x.0 > m.0 { x } else { m });
        Some((work, crit, bigsum, critb))
    }

    /// supernode の数。
    pub fn n_supernodes(&self) -> usize {
        self.begin.len()
    }

    /// 因子の値の数。
    pub fn len_values(&self) -> usize {
        self.l.len()
    }

    /// 上三角の列圧縮の値 (`new` に渡した形の順) で数値分解する。ピボットが `eps` 以下なら `delta` に置き換える。
    pub fn factor(&mut self, values: &[f64], delta: f64, eps: f64) -> bool {
        let lp = SyncPtr(self.l.as_mut_ptr());
        let this = &*self;
        let reg = LltRegularization { dynamic_regularization_delta: delta, dynamic_regularization_epsilon: eps };
        let ok = std::sync::atomic::AtomicBool::new(true);
        if let Some(tr) = &this.trace {
            tr.iter().for_each(|t| t.store(0, std::sync::atomic::Ordering::Relaxed));
        }
        if super::kkt::inner_seq() {
            // 外側で独立な成分を並列に解いているとき: rayon を使わず逐次に分解する。
            for &r in &this.roots {
                let _ = this.run_seq(r, values, reg, lp, &ok);
            }
        } else {
            this.roots.par_iter().for_each(|&r| {
                let _ = this.run(r, values, reg, lp, &ok);
            });
        }
        ok.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 部分木 `s` を分解し、`s` の更新行列 (下の行どうし、形は [`Front`]) を返す (根なら `None`)。
    fn run(&self, s: usize, values: &[f64], reg: LltRegularization<f64>, lp: SyncPtr, ok: &std::sync::atomic::AtomicBool) -> Option<Vec<f64>> {
        if self.subtree_flops[s] <= self.seq_flops {
            return self.run_seq(s, values, reg, lp, ok);
        }
        // 子が 1 つだけの鎖は下へたどり、下から順に処理する (再帰を深くしない)。
        let mut chain = vec![s];
        loop {
            let last = *chain.last().unwrap();
            let ch = &self.children[last];
            if ch.len() == 1 && self.subtree_flops[ch[0]] > self.seq_flops {
                chain.push(ch[0]);
            } else {
                break;
            }
        }
        let bottom = *chain.last().unwrap();
        let ups: Vec<(usize, Vec<f64>)> = self.children[bottom].par_iter().filter_map(|&c| self.run(c, values, reg, lp, ok).map(|u| (c, u))).collect();
        self.prepare(bottom, values, lp);
        let mut u = self.finish(bottom, ups, None, None, reg, lp, ok);
        for w in (0..chain.len() - 1).rev() {
            let node = chain[w];
            self.prepare(node, values, lp);
            let ups = u.take().map(|v| vec![(chain[w + 1], v)]).unwrap_or_default();
            u = self.finish(node, ups, None, None, reg, lp, ok);
        }
        u
    }

    /// 部分木 `s` を後順で逐次に分解し、`s` の更新行列を返す (根なら `None`)。`direct` なら子は終わった時に更新行列を
    /// 親の前線へ足す (子の更新行列を受けていない子は、行列積で親の前線へ直接書く)。
    /// そうでなければ子の更新行列を親が終わるまで `ups` に積んでおく。
    fn run_seq(&self, s: usize, values: &[f64], reg: LltRegularization<f64>, lp: SyncPtr, ok: &std::sync::atomic::AtomicBool) -> Option<Vec<f64>> {
        struct Ent {
            node: usize,
            /// 次に見る子の番号。
            ci: usize,
            /// L のかたまりを用意したか (`direct` なら最初の子が終わる時、そうでなければ自身が終わる時に用意する)。
            prepared: bool,
            /// 前線の更新行列 (`direct` で子から受けたものがあれば)。
            u: Option<Vec<f64>>,
            /// まだ足していない子の更新行列。
            ups: Vec<(usize, Vec<f64>)>,
        }
        let ent = |node: usize| Ent { node, ci: 0, prepared: false, u: None, ups: Vec::new() };
        let mut stack = vec![ent(s)];
        loop {
            let top = stack.last_mut().unwrap();
            if top.ci < self.children[top.node].len() {
                let c = self.children[top.node][top.ci];
                top.ci += 1;
                stack.push(ent(c));
                continue;
            }
            let e = stack.pop().unwrap();
            if !e.prepared {
                self.prepare(e.node, values, lp);
            }
            match stack.last_mut() {
                Some(p) if self.direct => {
                    // 親の前線へ書く直前に親を用意する (下る時に用意すると、書く時にはキャッシュから追い出されている)。
                    if !p.prepared {
                        self.prepare(p.node, values, lp);
                        p.prepared = true;
                    }
                    let pn = p.node;
                    let res = self.finish(e.node, e.ups, e.u, Some((pn, &mut p.u)), reg, lp, ok);
                    debug_assert!(res.is_none());
                }
                Some(p) => {
                    if let Some(v) = self.finish(e.node, e.ups, e.u, None, reg, lp, ok) {
                        p.ups.push((e.node, v));
                    }
                }
                None => return self.finish(e.node, e.ups, e.u, None, reg, lp, ok),
            }
        }
    }

    /// supernode `s` の L のかたまり (前線の左の `nc` 列、因子の配列の中) を 0 にして元の行列の値を足す。
    fn prepare(&self, s: usize, values: &[f64], lp: SyncPtr) {
        let t0 = (prof_on() || self.trace.is_some()).then(std::time::Instant::now);
        let nc = self.end[s] - self.begin[s];
        let f = nc + self.pat_ptr[s + 1] - self.pat_ptr[s];
        // SAFETY: supernode ごとに `l_ptr` の範囲は重ならず、各 supernode はちょうど 1 回だけ処理される。
        let lsl: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(lp.0.add(self.l_ptr[s]), f * nc) };
        if self.par_prep && (nc as f64) * (f as f64) * (f as f64) > self.par_flops && !super::kkt::inner_seq() {
            lsl.par_chunks_mut(1 << 15).for_each(|c| c.fill(0.0));
        } else {
            lsl.fill(0.0);
        }
        for &(k, lr, lc) in &self.asm[s] {
            lsl[lc as usize * f + lr as usize] += values[k as usize];
        }
        if let Some(t0) = t0 {
            let ns = t0.elapsed().as_nanos() as u64;
            if prof_on() {
                PROF[0].fetch_add(ns, std::sync::atomic::Ordering::Relaxed);
            }
            if let Some(tr) = &self.trace {
                tr[s].fetch_add(ns, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    /// supernode `s` の前線 (更新行列は `u`)。
    fn front(&self, s: usize, lp: SyncPtr, u: &mut Option<Vec<f64>>) -> Front {
        let nc = self.end[s] - self.begin[s];
        let r = self.pat_ptr[s + 1] - self.pat_ptr[s];
        let up = u.as_mut().map_or(std::ptr::null_mut(), |v| v.as_mut_ptr());
        Front { l: SyncPtr(lp.0.wrapping_add(self.l_ptr[s])), f: nc + r, nc, u: SyncPtr(up), r, bw: self.pack_bw }
    }

    /// supernode `s` の更新行列 `u` がなければ (0 で) 作る。
    fn ensure_u(&self, s: usize, u: &mut Option<Vec<f64>>) {
        if u.is_none() {
            *u = Some(self.take_buf(u_len(self.pat_ptr[s + 1] - self.pat_ptr[s], self.pack_bw)));
        }
    }

    /// 子 `c` の更新行列 `uc` を、親 `p` の前線 (更新行列は `pu`、子が更新行列の部分に当たるなら作る) の行・列
    /// `child_map[c]` に足す (extend-add)。
    fn extend_add(&self, p: usize, pu: &mut Option<Vec<f64>>, lp: SyncPtr, c: usize, uc: &[f64]) {
        let map = &self.child_map[c];
        let rc = map.len();
        if rc == 0 {
            return;
        }
        if map[rc - 1] as usize >= self.end[p] - self.begin[p] {
            self.ensure_u(p, pu);
        }
        let dst = self.front(p, lp, pu);
        let src = Front::u_only(uc.as_ptr() as *mut f64, rc, self.pack_bw);
        // 子の更新行列の列 j を親の前線の列 map[j] に足す (列ごとに書き先が重ならない)。
        let add_col = |j: usize| {
            let mj = map[j] as usize;
            let mi = &map[j..];
            let last = mi[rc - j - 1] as usize;
            // SAFETY: 子の配列の列 j の対角から下 (`rc - j` 個) を読むだけ。map は単射なので列 j ごとに書き先の列が違い、
            // 前線の列 mj の行 mj..=last は前線の中。
            let (col, dst) = unsafe {
                (std::slice::from_raw_parts(src.col(j).0 as *const f64, rc - j), std::slice::from_raw_parts_mut(dst.col(mj).0, last - mj + 1))
            };
            // 行の写像が連続な部分は、まとめて足す。
            if last - mj == rc - j - 1 {
                for (d, &v) in dst.iter_mut().zip(col) {
                    *d += v;
                }
            } else {
                for (t, &v) in col.iter().enumerate() {
                    dst[mi[t] as usize - mj] += v;
                }
            }
        };
        if (rc as f64) * (rc as f64) > self.ea_par && !super::kkt::inner_seq() {
            (0..rc).into_par_iter().with_min_len(16).for_each(add_col);
        } else {
            (0..rc).for_each(add_col);
        }
    }

    /// supernode `s` の前線 (L のかたまりは [`Self::prepare`] 済み、更新行列は `u`、まだ足していない子の更新行列は
    /// `ups`) を部分分解して L のかたまりを書き、更新行列を作る。前線の左の `nc` 列 (L のかたまり、`f x nc`) は因子の
    /// 配列の中でそのまま組み立てて分解する。親 `parent` (`(親, 親の更新行列)`、親は prepare 済み) があれば、更新行列を
    /// 親の前線に足して `None` を返し (`direct` で子の更新行列を受けていなければ、行列積で親の前線へ直接書き、更新行列の
    /// 配列を作らない)、なければ更新行列を返す。
    #[allow(clippy::too_many_arguments)]
    fn finish(&self, s: usize, ups: Vec<(usize, Vec<f64>)>, mut u: Option<Vec<f64>>, parent: Option<(usize, &mut Option<Vec<f64>>)>, reg: LltRegularization<f64>, lp: SyncPtr, ok: &std::sync::atomic::AtomicBool) -> Option<Vec<f64>> {
        let nc = self.end[s] - self.begin[s];
        let r = self.pat_ptr[s + 1] - self.pat_ptr[s];
        let f = nc + r;
        let tp0 = std::time::Instant::now();
        if !self.direct {
            self.ensure_u(s, &mut u);
        }
        for (c, uc) in ups {
            self.extend_add(s, &mut u, lp, c, &uc);
            self.give_back(uc);
        }
        // SAFETY: supernode ごとに `l_ptr` の範囲は重ならず、各 supernode はちょうど 1 回だけ処理される。
        let lsl: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(lp.0.add(self.l_ptr[s]), f * nc) };
        let tp1 = std::time::Instant::now();
        let big = (nc as f64) * (f as f64) * (f as f64) > self.par_flops;
        let par = if big && !super::kkt::inner_seq() { Parallelism::Rayon(0) } else { Parallelism::None };
        let lm = from_column_major_slice_mut::<f64, usize, usize>(lsl, f, nc);
        let (mut l11, mut l21) = lm.split_at_row_mut(nc);
        let tp2;
        let mut tp3;
        // 更新行列 `U = F22 - A Bᵀ` の `A` (LDLᵀ なら `W = L21 D11`、LLᵀ なら `L21`)。
        let mut w: Option<Vec<f64>> = None;
        if let Some(signs) = &self.signs {
            // LDLᵀ (準定値の拡大系): F11 = L11 D11 L11ᵀ (期待する符号と合わない・小さいピボットは置き換える)、
            // W = F21 L11^{-T} (= L21 D11)、L21 = W D11^{-1}、U = F22 - W L21ᵀ。
            use faer::linalg::cholesky::ldlt_diagonal::compute::{raw_cholesky_in_place, raw_cholesky_in_place_req, LdltRegularization};
            let mut buf = GlobalPodBuffer::new(raw_cholesky_in_place_req::<f64>(nc, par, Default::default()).unwrap());
            let lreg = LdltRegularization {
                dynamic_regularization_signs: Some(&signs[self.begin[s]..self.end[s]]),
                dynamic_regularization_delta: reg.dynamic_regularization_delta,
                dynamic_regularization_epsilon: reg.dynamic_regularization_epsilon,
            };
            let info = raw_cholesky_in_place(l11.rb_mut(), lreg, par, PodStack::new(&mut buf), Default::default());
            if info.dynamic_regularization_count > 0 {
                DYNREG.fetch_add(info.dynamic_regularization_count, std::sync::atomic::Ordering::Relaxed);
            }
            if !(0..nc).all(|j| l11.read(j, j).is_finite() && l11.read(j, j) != 0.0) {
                ok.store(false, std::sync::atomic::Ordering::Relaxed);
            }
            tp2 = std::time::Instant::now();
            tp3 = tp2;
            if r > 0 {
                let mut wb = self.take_buf(r * nc);
                if self.par_trsm && par != Parallelism::None {
                    let dinv: Vec<f64> = (0..nc).map(|j| 1.0 / l11.read(j, j)).collect();
                    trsm_rows(l11.rb(), l21.rb_mut(), true, Some((SyncPtr(wb.as_mut_ptr()), &dinv)), self.trsm_rows(r));
                } else {
                    faer::linalg::triangular_solve::solve_unit_lower_triangular_in_place(l11.rb(), l21.rb_mut().transpose_mut(), par);
                    // W (= L21 D) を写してから L21 = W D^{-1}。
                    for j in 0..nc {
                        let dinv = 1.0 / l11.read(j, j);
                        for i in 0..r {
                            let v = l21.read(i, j);
                            wb[j * r + i] = v;
                            l21.write(i, j, v * dinv);
                        }
                    }
                }
                w = Some(wb);
                tp3 = std::time::Instant::now();
            }
        } else {
            let res = if self.bchol_bs > 0 && big && nc >= self.bchol_min {
                chol_blocked(l11.rb_mut(), reg, self.bchol_bs, par)
            } else {
                let mut buf = GlobalPodBuffer::new(cholesky_in_place_req::<f64>(nc, par, LltParams::default()).unwrap());
                cholesky_in_place(l11.rb_mut(), reg, par, PodStack::new(&mut buf), LltParams::default()).map(|info| info.dynamic_regularization_count).map_err(|_| ())
            };
            match res {
                Ok(cnt) => {
                    if cnt > 0 {
                        DYNREG.fetch_add(cnt, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                Err(_) => ok.store(false, std::sync::atomic::Ordering::Relaxed),
            }
            tp2 = std::time::Instant::now();
            tp3 = tp2;
            if r > 0 {
                // L21 = F21 L11^{-T}: L11 L21ᵀ = F21ᵀ を解く。
                if self.par_trsm && par != Parallelism::None {
                    trsm_rows(l11.rb(), l21.rb_mut(), false, None, self.trsm_rows(r));
                } else {
                    faer::linalg::triangular_solve::solve_lower_triangular_in_place(l11.rb(), l21.rb_mut().transpose_mut(), par);
                }
                tp3 = std::time::Instant::now();
            }
        }
        let mut ea_ns = 0u64;
        let mut res = None;
        if r > 0 {
            let b = l21.rb();
            let a = match &w {
                Some(wb) => from_column_major_slice::<f64, usize, usize>(wb, r, nc),
                None => b,
            };
            let map = &self.child_map[s];
            match parent {
                Some((p, pu)) if self.direct && u.is_none() && r >= self.direct_min_r && map_runs(map) <= self.direct_runs => {
                    // 子の更新行列を受けていない: U = -A Bᵀ を親の前線へ直接書く。
                    if map[r - 1] as usize >= self.end[p] - self.begin[p] {
                        self.ensure_u(p, pu);
                    }
                    sub_update(self.front(p, lp, pu), Some(map), a, b, self.chunk, big);
                }
                parent => {
                    // U = F22 - A Bᵀ (下三角だけ)。
                    self.ensure_u(s, &mut u);
                    let own = Front::u_only(u.as_mut().unwrap().as_mut_ptr(), r, self.pack_bw);
                    sub_update(own, None, a, b, if self.pack_bw == 0 { usize::MAX } else { self.chunk }, big);
                    let uv = u.take().unwrap();
                    if let Some((p, pu)) = parent {
                        let te = std::time::Instant::now();
                        self.extend_add(p, pu, lp, s, &uv);
                        self.give_back(uv);
                        ea_ns = te.elapsed().as_nanos() as u64;
                    } else {
                        res = Some(uv);
                    }
                }
            }
        }
        if let Some(wb) = w {
            self.give_back(wb);
        }
        if let Some(uv) = u {
            self.give_back(uv);
        }
        if prof_on() {
            let tp4 = std::time::Instant::now();
            let d = |a: std::time::Instant, b: std::time::Instant| (b - a).as_nanos() as u64;
            use std::sync::atomic::Ordering::Relaxed;
            PROF[0].fetch_add(d(tp0, tp1) + ea_ns, Relaxed);
            PROF[1].fetch_add(d(tp1, tp2), Relaxed);
            PROF[2].fetch_add(d(tp2, tp3), Relaxed);
            PROF[3].fetch_add(d(tp3, tp4) - ea_ns, Relaxed);
            let bk = if big { 5 } else { [32usize, 128, 512, 2048].iter().position(|&lim| f < lim).unwrap_or(4) };
            BUCKET_N[bk].fetch_add(1, Relaxed);
            for (i, v) in [d(tp0, tp1) + ea_ns, d(tp1, tp2), d(tp2, tp3), d(tp3, tp4) - ea_ns].into_iter().enumerate() {
                BUCKET_T[bk][i].fetch_add(v, Relaxed);
            }
            let (ncf, rf) = (nc as f64, r as f64);
            for (i, v) in [ncf * ncf * ncf / 3.0, ncf * ncf * rf, ncf * rf * rf].into_iter().enumerate() {
                BUCKET_F[bk][i].fetch_add(v as u64, Relaxed);
            }
            if big {
                eprintln!("MF big node s={s} nc={nc} r={r} children={} asm+ea={:.3}ms chol={:.3}ms trsm={:.3}ms syrk={:.3}ms", self.children[s].len(),
                    (d(tp0, tp1) + ea_ns) as f64 * 1e-6, d(tp1, tp2) as f64 * 1e-6, d(tp2, tp3) as f64 * 1e-6, (d(tp3, tp4) - ea_ns) as f64 * 1e-6);
            }
        }
        if let Some(tr) = &self.trace {
            tr[s].fetch_add(tp0.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
        }
        res
    }

    /// `rhs` を `M^{-1} rhs` で上書きする。
    pub fn solve_in_place(&self, rhs: &mut [f64]) {
        if tunable!("ENOMOTO_T_MF_PAR_SOLVE", 1u8, u8) != 0 && !super::kkt::inner_seq() {
            return self.solve_in_place_tree(rhs);
        }
        let n = self.n;
        let mut x: Vec<f64> = (0..n).map(|i| rhs[self.perm_fwd[i]]).collect();
        let ns = self.begin.len();
        let mut tmp: Vec<f64> = Vec::new();
        // 前進: L y = x (supernode の番号の順は子が先)。
        for s in 0..ns {
            let (b, e) = (self.begin[s], self.end[s]);
            let nc = e - b;
            let r = self.pat_ptr[s + 1] - self.pat_ptr[s];
            let f = nc + r;
            let lmat = from_column_major_slice::<f64, usize, usize>(&self.l[self.l_ptr[s]..self.l_ptr[s + 1]], f, nc);
            let (l11, _l21) = lmat.split_at_row(nc);
            {
                let xs = from_column_major_slice_mut::<f64, usize, usize>(&mut x[b..e], nc, 1);
                if self.signs.is_some() {
                    faer::linalg::triangular_solve::solve_unit_lower_triangular_in_place(l11, xs, Parallelism::None);
                } else {
                    faer::linalg::triangular_solve::solve_lower_triangular_in_place(l11, xs, Parallelism::None);
                }
            }
            if r > 0 {
                // tmp = L21 x_s (密な行列ベクトル積) を下の行に引く。
                let pat = &self.pat[self.pat_ptr[s]..self.pat_ptr[s + 1]];
                tmp.clear();
                tmp.resize(r, 0.0);
                let xs = from_column_major_slice::<f64, usize, usize>(&x[b..e], nc, 1);
                faer::linalg::matmul::matmul(
                    from_column_major_slice_mut::<f64, usize, usize>(&mut tmp, r, 1),
                    _l21,
                    xs,
                    None,
                    1.0,
                    Parallelism::None,
                );
                for (t, &i) in pat.iter().enumerate() {
                    x[i] -= tmp[t];
                }
            }
        }
        // LDLᵀ: 対角 D で割る。
        if self.signs.is_some() {
            for s in 0..ns {
                let (b, e) = (self.begin[s], self.end[s]);
                let f = e - b + self.pat_ptr[s + 1] - self.pat_ptr[s];
                let ls = &self.l[self.l_ptr[s]..self.l_ptr[s + 1]];
                for j in 0..e - b {
                    x[b + j] /= ls[j * f + j];
                }
            }
        }
        // 後退: Lᵀ x = y。
        for s in (0..ns).rev() {
            let (b, e) = (self.begin[s], self.end[s]);
            let nc = e - b;
            let r = self.pat_ptr[s + 1] - self.pat_ptr[s];
            let f = nc + r;
            let lmat = from_column_major_slice::<f64, usize, usize>(&self.l[self.l_ptr[s]..self.l_ptr[s + 1]], f, nc);
            let (l11, _l21) = lmat.split_at_row(nc);
            if r > 0 {
                // x_s -= L21ᵀ x[pat] (下の行を集めてから密な行列ベクトル積)。
                let pat = &self.pat[self.pat_ptr[s]..self.pat_ptr[s + 1]];
                tmp.clear();
                tmp.extend(pat.iter().map(|&i| x[i]));
                faer::linalg::matmul::matmul(
                    from_column_major_slice_mut::<f64, usize, usize>(&mut x[b..e], nc, 1),
                    _l21.transpose(),
                    from_column_major_slice::<f64, usize, usize>(&tmp, r, 1),
                    Some(1.0),
                    -1.0,
                    Parallelism::None,
                );
            }
            let xs = from_column_major_slice_mut::<f64, usize, usize>(&mut x[b..e], nc, 1);
            if self.signs.is_some() {
                faer::linalg::triangular_solve::solve_unit_upper_triangular_in_place(l11.transpose(), xs, Parallelism::None);
            } else {
                faer::linalg::triangular_solve::solve_upper_triangular_in_place(l11.transpose(), xs, Parallelism::None);
            }
        }
        for i in 0..n {
            rhs[self.perm_fwd[i]] = x[i];
        }
        let _ = &self.perm_inv;
    }

    /// [`Self::solve_in_place`] を分解と同じ木で並列に解く (`ENOMOTO_T_MF_PAR_SOLVE`、既定 1、0 で従来の逐次)。前進は supernode ごとに子の更新ベクトルを
    /// 子の順に集め (親の列には足し、親の下の行は自分の更新ベクトルに足す)、自分の列を解いて `-L21 x_s` を足した更新ベクトルを
    /// 親に返す。独立な部分木は並列。後退は祖先の列の値だけを読むので、親が終わった子どうしは並列。足す順はスレッド数に
    /// よらない (従来の逐次の解き方とは足す順が違う)。
    fn solve_in_place_tree(&self, rhs: &mut [f64]) {
        let n = self.n;
        let mut x: Vec<f64> = (0..n).map(|i| rhs[self.perm_fwd[i]]).collect();
        let xp = SyncPtr(x.as_mut_ptr());
        self.roots.par_iter().for_each(|&r| {
            let _ = self.fwd_par(r, xp);
        });
        self.roots.par_iter().for_each(|&r| self.bwd_par(r, xp));
        for i in 0..n {
            rhs[self.perm_fwd[i]] = x[i];
        }
    }

    fn fwd_par(&self, s: usize, xp: SyncPtr) -> Vec<f64> {
        if self.subtree_flops[s] <= self.seq_flops {
            return self.fwd_seq(s, xp);
        }
        let mut chain = vec![s];
        loop {
            let last = *chain.last().unwrap();
            let ch = &self.children[last];
            if ch.len() == 1 && self.subtree_flops[ch[0]] > self.seq_flops {
                chain.push(ch[0]);
            } else {
                break;
            }
        }
        let bottom = *chain.last().unwrap();
        let ups: Vec<(usize, Vec<f64>)> = self.children[bottom].par_iter().map(|&c| (c, self.fwd_par(c, xp))).collect();
        let mut u = self.fwd_node(bottom, ups, xp);
        for w in (0..chain.len() - 1).rev() {
            u = self.fwd_node(chain[w], vec![(chain[w + 1], u)], xp);
        }
        u
    }

    fn fwd_seq(&self, s: usize, xp: SyncPtr) -> Vec<f64> {
        let mut stack: Vec<(usize, usize)> = vec![(s, 0)];
        let mut pending: Vec<Vec<(usize, Vec<f64>)>> = vec![Vec::new()];
        loop {
            let (node, ci) = *stack.last().unwrap();
            if ci < self.children[node].len() {
                stack.last_mut().unwrap().1 += 1;
                stack.push((self.children[node][ci], 0));
                pending.push(Vec::new());
                continue;
            }
            stack.pop();
            let ups = pending.pop().unwrap();
            let u = self.fwd_node(node, ups, xp);
            match pending.last_mut() {
                Some(p) => p.push((node, u)),
                None => return u,
            }
        }
    }

    /// 前進の 1 つの supernode: 子の更新を集め、`L11 y = x_s` を解き、更新ベクトル `u - L21 y` を返す。LDLᵀ では最後に D で割る。
    fn fwd_node(&self, s: usize, ups: Vec<(usize, Vec<f64>)>, xp: SyncPtr) -> Vec<f64> {
        let (b, e) = (self.begin[s], self.end[s]);
        let nc = e - b;
        let r = self.pat_ptr[s + 1] - self.pat_ptr[s];
        let f = nc + r;
        // SAFETY: supernode ごとに列の範囲 [b, e) は重ならず、各 supernode はちょうど 1 回だけ処理される。
        let xs: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(xp.get().add(b), nc) };
        let mut u = vec![0.0f64; r];
        for (c, uc) in ups {
            let map = &self.child_map[c];
            for (j, &v) in uc.iter().enumerate() {
                let mj = map[j] as usize;
                if mj < nc {
                    xs[mj] += v;
                } else {
                    u[mj - nc] += v;
                }
            }
        }
        let ls = &self.l[self.l_ptr[s]..self.l_ptr[s + 1]];
        let lmat = from_column_major_slice::<f64, usize, usize>(ls, f, nc);
        let (l11, l21) = lmat.split_at_row(nc);
        {
            let xm = from_column_major_slice_mut::<f64, usize, usize>(xs, nc, 1);
            if self.signs.is_some() {
                faer::linalg::triangular_solve::solve_unit_lower_triangular_in_place(l11, xm, Parallelism::None);
            } else {
                faer::linalg::triangular_solve::solve_lower_triangular_in_place(l11, xm, Parallelism::None);
            }
        }
        if r > 0 {
            faer::linalg::matmul::matmul(
                from_column_major_slice_mut::<f64, usize, usize>(&mut u, r, 1),
                l21,
                from_column_major_slice::<f64, usize, usize>(xs, nc, 1),
                Some(1.0),
                -1.0,
                Parallelism::None,
            );
        }
        if self.signs.is_some() {
            for j in 0..nc {
                xs[j] /= ls[j * f + j];
            }
        }
        u
    }

    fn bwd_par(&self, s: usize, xp: SyncPtr) {
        if self.subtree_flops[s] <= self.seq_flops {
            // 部分木を前順で逐次に。
            let mut stack = vec![s];
            while let Some(node) = stack.pop() {
                self.bwd_node(node, xp);
                stack.extend(self.children[node].iter().rev());
            }
            return;
        }
        self.bwd_node(s, xp);
        self.children[s].par_iter().for_each(|&c| self.bwd_par(c, xp));
    }

    /// 後退の 1 つの supernode: `x_s -= L21ᵀ x[pat]` (祖先の値は確定済み) の後 `L11ᵀ x_s = ...` を解く。
    fn bwd_node(&self, s: usize, xp: SyncPtr) {
        let (b, e) = (self.begin[s], self.end[s]);
        let nc = e - b;
        let r = self.pat_ptr[s + 1] - self.pat_ptr[s];
        let f = nc + r;
        let lmat = from_column_major_slice::<f64, usize, usize>(&self.l[self.l_ptr[s]..self.l_ptr[s + 1]], f, nc);
        let (l11, l21) = lmat.split_at_row(nc);
        if r > 0 {
            let pat = &self.pat[self.pat_ptr[s]..self.pat_ptr[s + 1]];
            // SAFETY: pat は祖先の列で、祖先は前に処理済み (この間は書かれない)。
            let tmp: Vec<f64> = pat.iter().map(|&i| unsafe { *xp.get().add(i) }).collect();
            let xs: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(xp.get().add(b), nc) };
            faer::linalg::matmul::matmul(
                from_column_major_slice_mut::<f64, usize, usize>(xs, nc, 1),
                l21.transpose(),
                from_column_major_slice::<f64, usize, usize>(&tmp, r, 1),
                Some(1.0),
                -1.0,
                Parallelism::None,
            );
        }
        let xs: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(xp.get().add(b), nc) };
        let xm = from_column_major_slice_mut::<f64, usize, usize>(xs, nc, 1);
        if self.signs.is_some() {
            faer::linalg::triangular_solve::solve_unit_upper_triangular_in_place(l11.transpose(), xm, Parallelism::None);
        } else {
            faer::linalg::triangular_solve::solve_upper_triangular_in_place(l11.transpose(), xm, Parallelism::None);
        }
    }

    /// `k` 本の右辺 (`rhs` は列優先の `n x k`) をまとめて解く。中身は `solve_in_place` と同じで、
    /// 密な演算が行列ベクトル積から行列積になる。作業域は行優先の `n x k` (supernode の行がひと続き)。
    pub fn solve_multi_in_place(&self, rhs: &mut [f64], k: usize) {
        let n = self.n;
        debug_assert_eq!(rhs.len(), n * k);
        let mut x = vec![0.0f64; n * k];
        for c in 0..k {
            let col = &rhs[c * n..(c + 1) * n];
            for i in 0..n {
                x[i * k + c] = col[self.perm_fwd[i]];
            }
        }
        let ns = self.begin.len();
        let mut tmp: Vec<f64> = Vec::new();
        for s in 0..ns {
            let (b, e) = (self.begin[s], self.end[s]);
            let nc = e - b;
            let r = self.pat_ptr[s + 1] - self.pat_ptr[s];
            let f = nc + r;
            let lmat = from_column_major_slice::<f64, usize, usize>(&self.l[self.l_ptr[s]..self.l_ptr[s + 1]], f, nc);
            let (l11, l21) = lmat.split_at_row(nc);
            {
                let xs = from_row_major_slice_mut::<f64, usize, usize>(&mut x[b * k..e * k], nc, k);
                if self.signs.is_some() {
                    faer::linalg::triangular_solve::solve_unit_lower_triangular_in_place(l11, xs, Parallelism::None);
                } else {
                    faer::linalg::triangular_solve::solve_lower_triangular_in_place(l11, xs, Parallelism::None);
                }
            }
            if r > 0 {
                let pat = &self.pat[self.pat_ptr[s]..self.pat_ptr[s + 1]];
                tmp.clear();
                tmp.resize(r * k, 0.0);
                let xs = from_row_major_slice::<f64, usize, usize>(&x[b * k..e * k], nc, k);
                faer::linalg::matmul::matmul(
                    from_row_major_slice_mut::<f64, usize, usize>(&mut tmp, r, k),
                    l21,
                    xs,
                    None,
                    1.0,
                    Parallelism::None,
                );
                for (t, &i) in pat.iter().enumerate() {
                    for (xv, tv) in x[i * k..(i + 1) * k].iter_mut().zip(&tmp[t * k..(t + 1) * k]) {
                        *xv -= tv;
                    }
                }
            }
        }
        if self.signs.is_some() {
            for s in 0..ns {
                let (b, e) = (self.begin[s], self.end[s]);
                let f = e - b + self.pat_ptr[s + 1] - self.pat_ptr[s];
                let ls = &self.l[self.l_ptr[s]..self.l_ptr[s + 1]];
                for j in 0..e - b {
                    let d = ls[j * f + j];
                    for xv in &mut x[(b + j) * k..(b + j + 1) * k] {
                        *xv /= d;
                    }
                }
            }
        }
        for s in (0..ns).rev() {
            let (b, e) = (self.begin[s], self.end[s]);
            let nc = e - b;
            let r = self.pat_ptr[s + 1] - self.pat_ptr[s];
            let f = nc + r;
            let lmat = from_column_major_slice::<f64, usize, usize>(&self.l[self.l_ptr[s]..self.l_ptr[s + 1]], f, nc);
            let (l11, l21) = lmat.split_at_row(nc);
            if r > 0 {
                let pat = &self.pat[self.pat_ptr[s]..self.pat_ptr[s + 1]];
                tmp.clear();
                for &i in pat {
                    tmp.extend_from_slice(&x[i * k..(i + 1) * k]);
                }
                faer::linalg::matmul::matmul(
                    from_row_major_slice_mut::<f64, usize, usize>(&mut x[b * k..e * k], nc, k),
                    l21.transpose(),
                    from_row_major_slice::<f64, usize, usize>(&tmp, r, k),
                    Some(1.0),
                    -1.0,
                    Parallelism::None,
                );
            }
            let xs = from_row_major_slice_mut::<f64, usize, usize>(&mut x[b * k..e * k], nc, k);
            if self.signs.is_some() {
                faer::linalg::triangular_solve::solve_unit_upper_triangular_in_place(l11.transpose(), xs, Parallelism::None);
            } else {
                faer::linalg::triangular_solve::solve_upper_triangular_in_place(l11.transpose(), xs, Parallelism::None);
            }
        }
        for c in 0..k {
            let col = &mut rhs[c * n..(c + 1) * n];
            for i in 0..n {
                col[self.perm_fwd[i]] = x[i * k + c];
            }
        }
    }
}

#[cfg(test)]
mod kernel_bench {
    use faer::reborrow::ReborrowMut;
    use faer::linalg::cholesky::llt::compute::{cholesky_in_place, cholesky_in_place_req, LltParams, LltRegularization};
    use faer::linalg::matmul::triangular::BlockStructure;
    use faer::{dyn_stack::{GlobalPodBuffer, PodStack}, Mat, Parallelism};

    fn best<F: FnMut()>(reps: usize, mut f: F) -> f64 {
        let mut b = f64::INFINITY;
        for _ in 0..reps {
            let t = std::time::Instant::now();
            f();
            b = b.min(t.elapsed().as_secs_f64());
        }
        b
    }

    /// 計測用 (`cargo test --release front_update_bench -- --ignored --nocapture`): 大きな前線と同じ形 (nc 列、下の行 r) の
    /// 更新行列の計算を、確保と 0 埋めを含めて・含めずに、正方形 (faer の下三角の行列積 1 回) と台形 (かたまりごと) で測る。
    #[test]
    #[ignore]
    fn front_update_bench() {
        use super::{sub_update, u_len, Front};
        for (nc, r) in [(64usize, 1300usize), (200, 1300), (200, 2000), (400, 900), (100, 2600)] {
            let a = Mat::<f64>::from_fn(r, nc, |i, j| ((i * 7 + j * 3) % 11) as f64 * 0.1);
            let flops = (nc * r * r) as f64;
            for bw in [0usize, 128, 256] {
                let len = u_len(r, bw);
                let mut keep = vec![0.0f64; len];
                let reps = 20;
                // 配列を使い回す (0 埋めだけする)。
                let t_reuse = best(reps, || {
                    keep.iter_mut().for_each(|v| *v = 0.0);
                    let f = Front::u_only(keep.as_mut_ptr(), r, bw);
                    sub_update(f, None, a.as_ref(), a.as_ref(), if bw == 0 { usize::MAX } else { bw }, true);
                });
                // 0 埋めなし (行列積だけ)。
                let t_mm = best(reps, || {
                    let f = Front::u_only(keep.as_mut_ptr(), r, bw);
                    sub_update(f, None, a.as_ref(), a.as_ref(), if bw == 0 { usize::MAX } else { bw }, true);
                });
                // 毎回新しく確保する (ページフォールトを含む)。
                let t_fresh = best(reps, || {
                    let mut u = vec![0.0f64; len];
                    let f = Front::u_only(u.as_mut_ptr(), r, bw);
                    sub_update(f, None, a.as_ref(), a.as_ref(), if bw == 0 { usize::MAX } else { bw }, true);
                    std::hint::black_box(&u);
                });
                println!("FUBENCH nc={nc} r={r} bw={bw} matmul={:.2}ms ({:.1} GF) zero+matmul={:.2}ms ({:.1} GF) fresh={:.2}ms ({:.1} GF)",
                    t_mm * 1e3, flops / t_mm * 1e-9, t_reuse * 1e3, flops / t_reuse * 1e-9, t_fresh * 1e3, flops / t_fresh * 1e-9);
            }
            // 三角行列の解法 (L21 = F21 L11^{-T}、L21 は前線の中の列優先 f x nc の下の部分)。
            let l = Mat::<f64>::from_fn(nc, nc, |i, j| if i == j { 2.0 } else if i > j { 0.01 } else { 0.0 });
            let f0 = Mat::<f64>::from_fn(nc + r, nc, |i, j| ((i + 2 * j) % 5) as f64);
            let mut fm = f0.clone();
            for (pn, par) in [("seq", Parallelism::None), ("par", Parallelism::Rayon(0))] {
                let t = best(20, || {
                    fm.copy_from(&f0);
                    let (_, mut l21) = fm.as_mut().split_at_row_mut(nc);
                    faer::linalg::triangular_solve::solve_lower_triangular_in_place(l.as_ref(), l21.rb_mut().transpose_mut(), par);
                });
                println!("FUBENCH nc={nc} r={r} trsm {pn} {:.2}ms ({:.1} GF, 写しを含む)", t * 1e3, (nc * nc * r) as f64 / t * 1e-9);
            }
            let rows = r.div_ceil(4 * rayon::current_num_threads()).max(32);
            let t = best(20, || {
                fm.copy_from(&f0);
                let (_, l21) = fm.as_mut().split_at_row_mut(nc);
                super::trsm_rows(l.as_ref(), l21, false, None, rows);
            });
            println!("FUBENCH nc={nc} r={r} trsm rows(par, {rows} 行ずつ) {:.2}ms ({:.1} GF, 写しを含む)", t * 1e3, (nc * nc * r) as f64 / t * 1e-9);
        }
    }

    /// 計測用 (`cargo test --release kernel_bench -- --ignored --nocapture`): マルチフロンタル法の前線で使う faer の
    /// 密な演算 (下三角の行列積 `U -= L21 L21ᵀ`・Cholesky・三角行列の解法) の GFLOPS を、大きさと並列度ごとに出す
    /// (前線の大きさ別の内訳 `scripts/mf_bench/buckets.py` と比べて、faer が出せる速さとの差を見る)。
    #[test]
    #[ignore]
    fn kernel_bench() {
        let pars = [("seq", Parallelism::None), ("par", Parallelism::Rayon(0))];
        for (pn, par) in pars {
            // 下三角の行列積: r x r の下三角に r x nc と nc x r の積を足す。
            for (r, nc) in [(32usize, 16usize), (64, 32), (128, 32), (128, 128), (512, 32), (512, 128), (512, 512), (2048, 64), (2048, 256), (4096, 256), (4096, 1024)] {
                let a = Mat::<f64>::from_fn(r, nc, |i, j| ((i * 7 + j * 3) % 11) as f64 * 0.1);
                let mut u = Mat::<f64>::zeros(r, r);
                let reps = (2e9 / ((r * r * nc) as f64 + 1e5)).clamp(3.0, 2000.0) as usize;
                let t = best(reps, || {
                    faer::linalg::matmul::triangular::matmul(u.as_mut(), BlockStructure::TriangularLower, a.as_ref(), BlockStructure::Rectangular, a.as_ref().transpose(), BlockStructure::Rectangular, Some(1.0), -1.0, par);
                });
                println!("KBENCH {pn} syrk r={r} nc={nc} t={:.3e}s GFLOPS={:.1}", t, (r * r * nc) as f64 / t * 1e-9);
            }
            // Cholesky (nc x nc) と三角行列の解法 (r x nc を L11ᵀ で割る)。
            for (nc, r) in [(16usize, 64usize), (32, 128), (128, 512), (512, 2048), (1024, 1024), (2048, 2048), (4096, 256)] {
                let base = Mat::<f64>::from_fn(nc, nc, |i, j| if i == j { nc as f64 + 1.0 } else { 1.0 / (1.0 + (i + j) as f64) });
                let mut l = base.clone();
                let mut buf = GlobalPodBuffer::new(cholesky_in_place_req::<f64>(nc, par, LltParams::default()).unwrap());
                let reps = (2e9 / ((nc * nc * nc) as f64 + 1e5)).clamp(3.0, 2000.0) as usize;
                let tc = best(reps, || {
                    l.copy_from(&base);
                    cholesky_in_place(l.as_mut(), LltRegularization::default(), par, PodStack::new(&mut buf), LltParams::default()).unwrap();
                });
                let b0 = Mat::<f64>::from_fn(r, nc, |i, j| ((i + 2 * j) % 5) as f64);
                let mut b = b0.clone();
                let tt = best(reps, || {
                    b.copy_from(&b0);
                    faer::linalg::triangular_solve::solve_lower_triangular_in_place(l.as_ref(), b.as_mut().transpose_mut(), par);
                });
                println!("KBENCH {pn} chol nc={nc} t={tc:.3e}s GFLOPS={:.1} | trsm nc={nc} r={r} t={tt:.3e}s GFLOPS={:.1}",
                    (nc * nc * nc) as f64 / 3.0 / tc * 1e-9, (nc * nc * r) as f64 / tt * 1e-9);
                for bs in [64usize, 128, 256] {
                    if nc < 2 * bs {
                        continue;
                    }
                    let tb = best(reps, || {
                        l.copy_from(&base);
                        super::chol_blocked(l.as_mut(), LltRegularization::default(), bs, par).unwrap();
                    });
                    println!("KBENCH {pn} chol_blocked nc={nc} bs={bs} t={tb:.3e}s GFLOPS={:.1}", (nc * nc * nc) as f64 / 3.0 / tb * 1e-9);
                }
            }
        }
    }
}
