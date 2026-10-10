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
use faer::{Parallelism, dyn_stack::{GlobalPodBuffer, PodStack}};
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
        let alpha = tunable!("ENOMOTO_T_MF_AMALG", 0.0f64, f64);
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
            signs: None,
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

    /// 部分木 `s` を分解し、`s` の更新行列 (下の行どうし、`r x r` 列優先) を返す。
    fn run(&self, s: usize, values: &[f64], reg: LltRegularization<f64>, lp: SyncPtr, ok: &std::sync::atomic::AtomicBool) -> Vec<f64> {
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
        let ups: Vec<(usize, Vec<f64>)> = self.children[bottom].par_iter().map(|&c| (c, self.run(c, values, reg, lp, ok))).collect();
        let mut u = self.factor_node(bottom, values, ups, reg, lp, ok);
        for w in (0..chain.len() - 1).rev() {
            let node = chain[w];
            u = self.factor_node(node, values, vec![(chain[w + 1], u)], reg, lp, ok);
        }
        u
    }

    /// 部分木 `s` を後順で逐次に分解する。
    fn run_seq(&self, s: usize, values: &[f64], reg: LltRegularization<f64>, lp: SyncPtr, ok: &std::sync::atomic::AtomicBool) -> Vec<f64> {
        // 明示的な後順の走査: (節点, 次に見る子の番号)。各節点の子の更新行列は `pending` に積む。
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
            let u = self.factor_node(node, values, ups, reg, lp, ok);
            match pending.last_mut() {
                Some(p) => p.push((node, u)),
                None => return u,
            }
        }
    }

    /// supernode `s` の前線を作って部分分解し、L のかたまりを書き、更新行列を返す。前線の左の `nc` 列 (L のかたまり、
    /// `f x nc`) は因子の配列の中でそのまま組み立てて分解し、右下の `r x r` (更新行列) は別の配列で組み立てる
    /// (前線全体の確保・0 埋めと、分解後の写しをしない)。
    fn factor_node(&self, s: usize, values: &[f64], ups: Vec<(usize, Vec<f64>)>, reg: LltRegularization<f64>, lp: SyncPtr, ok: &std::sync::atomic::AtomicBool) -> Vec<f64> {
        let nc = self.end[s] - self.begin[s];
        let r = self.pat_ptr[s + 1] - self.pat_ptr[s];
        let f = nc + r;
        let tp0 = std::time::Instant::now();
        // SAFETY: supernode ごとに `l_ptr` の範囲は重ならず、各 supernode はちょうど 1 回だけ処理される。
        let lsl: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(lp.0.add(self.l_ptr[s]), f * nc) };
        lsl.fill(0.0);
        let mut u = self.take_buf(r * r);
        for &(k, lr, lc) in &self.asm[s] {
            lsl[lc as usize * f + lr as usize] += values[k as usize];
        }
        let lptr = SyncPtr(lsl.as_mut_ptr());
        let uptr = SyncPtr(u.as_mut_ptr());
        for (c, uc) in ups {
            let map = &self.child_map[c];
            let rc = map.len();
            // 子の更新行列の列 j を親の前線の列 map[j] に足す (列ごとに書き先が重ならない)。
            let add_col = |j: usize| {
                let mj = map[j] as usize;
                let col = &uc[j * rc + j..(j + 1) * rc];
                let mi = &map[j..];
                // SAFETY: map は単射なので列 j ごとに書き先の列が違う。
                let (dst, off) = unsafe {
                    if mj < nc {
                        (std::slice::from_raw_parts_mut(lptr.get().add(mj * f), f), 0usize)
                    } else {
                        (std::slice::from_raw_parts_mut(uptr.get().add((mj - nc) * r), r), nc)
                    }
                };
                // 行の写像が連続な部分は、まとめて足す。
                let first = mi[0] as usize;
                if mi[rc - j - 1] as usize - first == rc - j - 1 {
                    for (d, &v) in dst[first - off..first - off + col.len()].iter_mut().zip(col) {
                        *d += v;
                    }
                } else {
                    for (t, &v) in col.iter().enumerate() {
                        dst[mi[t] as usize - off] += v;
                    }
                }
            };
            if (rc as f64) * (rc as f64) > self.ea_par && !super::kkt::inner_seq() {
                (0..rc).into_par_iter().with_min_len(16).for_each(add_col);
            } else {
                (0..rc).for_each(add_col);
            }
            self.give_back(uc);
        }
        let tp1 = std::time::Instant::now();
        let big = (nc as f64) * (f as f64) * (f as f64) > tunable!("ENOMOTO_T_MF_PAR_FLOPS", 5e7f64, f64);
        let par = if big && !super::kkt::inner_seq() { Parallelism::Rayon(0) } else { Parallelism::None };
        let lm = from_column_major_slice_mut::<f64, usize, usize>(lsl, f, nc);
        let (mut l11, mut l21) = lm.split_at_row_mut(nc);
        let tp2;
        let mut tp3;
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
                faer::linalg::triangular_solve::solve_unit_lower_triangular_in_place(l11.rb(), l21.rb_mut().transpose_mut(), par);
                // W (= L21 D) を写してから L21 = W D^{-1}。
                let mut w = self.take_buf(r * nc);
                for j in 0..nc {
                    let dinv = 1.0 / l11.read(j, j);
                    for i in 0..r {
                        let v = l21.read(i, j);
                        w[j * r + i] = v;
                        l21.write(i, j, v * dinv);
                    }
                }
                tp3 = std::time::Instant::now();
                let um = from_column_major_slice_mut::<f64, usize, usize>(&mut u, r, r);
                faer::linalg::matmul::triangular::matmul(
                    um,
                    BlockStructure::TriangularLower,
                    from_column_major_slice::<f64, usize, usize>(&w, r, nc),
                    BlockStructure::Rectangular,
                    l21.rb().transpose(),
                    BlockStructure::Rectangular,
                    Some(1.0),
                    -1.0,
                    par,
                );
                self.give_back(w);
            }
        } else {
        let mut buf = GlobalPodBuffer::new(cholesky_in_place_req::<f64>(nc, par, LltParams::default()).unwrap());
        match cholesky_in_place(l11.rb_mut(), reg, par, PodStack::new(&mut buf), LltParams::default()) {
            Ok(info) => {
                if info.dynamic_regularization_count > 0 {
                    DYNREG.fetch_add(info.dynamic_regularization_count, std::sync::atomic::Ordering::Relaxed);
                }
            }
            Err(_) => ok.store(false, std::sync::atomic::Ordering::Relaxed),
        }
        tp2 = std::time::Instant::now();
        tp3 = tp2;
        if r > 0 {
            // L21 = F21 L11^{-T}: L11 L21ᵀ = F21ᵀ を解く。
            faer::linalg::triangular_solve::solve_lower_triangular_in_place(l11.rb(), l21.rb_mut().transpose_mut(), par);
            tp3 = std::time::Instant::now();
            // U = F22 - L21 L21ᵀ (下三角だけ)。
            let um = from_column_major_slice_mut::<f64, usize, usize>(&mut u, r, r);
            faer::linalg::matmul::triangular::matmul(
                um,
                BlockStructure::TriangularLower,
                l21.rb(),
                BlockStructure::Rectangular,
                l21.rb().transpose(),
                BlockStructure::Rectangular,
                Some(1.0),
                -1.0,
                par,
            );
        }
        }
        if prof_on() {
            let tp4 = std::time::Instant::now();
            let d = |a: std::time::Instant, b: std::time::Instant| (b - a).as_nanos() as u64;
            use std::sync::atomic::Ordering::Relaxed;
            PROF[0].fetch_add(d(tp0, tp1), Relaxed);
            PROF[1].fetch_add(d(tp1, tp2), Relaxed);
            PROF[2].fetch_add(d(tp2, tp3), Relaxed);
            PROF[3].fetch_add(d(tp3, tp4), Relaxed);
            if big {
                eprintln!("MF big node s={s} nc={nc} r={r} children={} asm+ea={:.3}ms chol={:.3}ms trsm={:.3}ms syrk={:.3}ms", self.children[s].len(),
                    d(tp0, tp1) as f64 * 1e-6, d(tp1, tp2) as f64 * 1e-6, d(tp2, tp3) as f64 * 1e-6, d(tp3, tp4) as f64 * 1e-6);
            }
        }
        u
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
