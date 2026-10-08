//! 消去木の枝ごとに並列に分解するマルチフロンタル法の疎 Cholesky 分解 (試験用、`ENOMOTO_T_CHOL_BACKEND=2`)。
//!
//! faer の記号分解 (並べ替えと supernode の区切り・行の形) をそのまま使い、数値分解だけを自前で行う。
//! supernode `s` (列 `[b, e)`、その下の行の形 `pattern`) ごとに、前線行列 `F` (大きさ `nc + r`) に元の行列の値と
//! 子の更新行列を足し込み (extend-add)、密な部分 Cholesky
//! (`F11 = L11 L11ᵀ`、`L21 = F21 L11^{-T}`、`U = F22 - L21 L21ᵀ`) をして `U` を親に渡す。
//! 独立な子の部分木は rayon で並列に処理し、密な演算は faer の行列演算 (Cholesky・三角行列の解法・行列積) を使う。
//! 演算量の小さな部分木は 1 つの仕事として逐次に処理する (仕事を細かく分けすぎない)。

use faer::linalg::cholesky::llt::compute::{cholesky_in_place, cholesky_in_place_req, LltParams, LltRegularization};
use faer::linalg::matmul::triangular::BlockStructure;
use faer::mat::{from_column_major_slice, from_column_major_slice_mut};
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
}

/// 書き込み先が互いに重ならないことが分かっている配列への生ポインタ (並列に別々の範囲へ書く)。
#[derive(Clone, Copy)]
struct SyncPtr(*mut f64);
unsafe impl Send for SyncPtr {}
unsafe impl Sync for SyncPtr {}

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
        let begin: Vec<usize> = s.supernode_begin()[..ns].to_vec();
        let end: Vec<usize> = s.supernode_end()[..ns].to_vec();
        let mut pat_ptr = vec![0usize; ns + 1];
        let mut pat = Vec::new();
        for k in 0..ns {
            pat.extend_from_slice(s.supernode(k).pattern());
            pat_ptr[k + 1] = pat.len();
        }
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
        })
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
        this.roots.par_iter().for_each(|&r| {
            let _ = this.run(r, values, reg, lp, &ok);
        });
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
        // SAFETY: supernode ごとに `l_ptr` の範囲は重ならず、各 supernode はちょうど 1 回だけ処理される。
        let lsl: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(lp.0.add(self.l_ptr[s]), f * nc) };
        lsl.fill(0.0);
        let mut u = vec![0.0f64; r * r];
        for &(k, lr, lc) in &self.asm[s] {
            lsl[lc as usize * f + lr as usize] += values[k as usize];
        }
        for (c, uc) in ups {
            let map = &self.child_map[c];
            let rc = map.len();
            for j in 0..rc {
                let mj = map[j] as usize;
                let col = &uc[j * rc + j..(j + 1) * rc];
                let mi = &map[j..];
                if mj < nc {
                    let dst = &mut lsl[mj * f..(mj + 1) * f];
                    for (t, &v) in col.iter().enumerate() {
                        dst[mi[t] as usize] += v;
                    }
                } else {
                    let base = (mj - nc) * r;
                    let dst = &mut u[base..base + r];
                    for (t, &v) in col.iter().enumerate() {
                        dst[mi[t] as usize - nc] += v;
                    }
                }
            }
        }
        let big = (nc as f64) * (f as f64) * (f as f64) > tunable!("ENOMOTO_T_MF_PAR_FLOPS", 5e7f64, f64);
        let par = if big { Parallelism::Rayon(0) } else { Parallelism::None };
        let lm = from_column_major_slice_mut::<f64, usize, usize>(lsl, f, nc);
        let (mut l11, mut l21) = lm.split_at_row_mut(nc);
        let mut buf = GlobalPodBuffer::new(cholesky_in_place_req::<f64>(nc, par, LltParams::default()).unwrap());
        if cholesky_in_place(l11.rb_mut(), reg, par, PodStack::new(&mut buf), LltParams::default()).is_err() {
            ok.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        if r > 0 {
            // L21 = F21 L11^{-T}: L11 L21ᵀ = F21ᵀ を解く。
            faer::linalg::triangular_solve::solve_lower_triangular_in_place(l11.rb(), l21.rb_mut().transpose_mut(), par);
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
        u
    }

    /// `rhs` を `M^{-1} rhs` で上書きする。
    pub fn solve_in_place(&self, rhs: &mut [f64]) {
        let n = self.n;
        let mut x: Vec<f64> = (0..n).map(|i| rhs[self.perm_fwd[i]]).collect();
        let ns = self.begin.len();
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
                faer::linalg::triangular_solve::solve_lower_triangular_in_place(l11, xs, Parallelism::None);
            }
            if r > 0 {
                let pat = &self.pat[self.pat_ptr[s]..self.pat_ptr[s + 1]];
                let ls = &self.l[self.l_ptr[s]..self.l_ptr[s + 1]];
                for j in 0..nc {
                    let yj = x[b + j];
                    if yj != 0.0 {
                        let col = &ls[j * f + nc..(j + 1) * f];
                        for (t, &i) in pat.iter().enumerate() {
                            x[i] -= col[t] * yj;
                        }
                    }
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
                let pat = &self.pat[self.pat_ptr[s]..self.pat_ptr[s + 1]];
                let ls = &self.l[self.l_ptr[s]..self.l_ptr[s + 1]];
                for j in 0..nc {
                    let col = &ls[j * f + nc..(j + 1) * f];
                    let mut acc = 0.0;
                    for (t, &i) in pat.iter().enumerate() {
                        acc += col[t] * x[i];
                    }
                    x[b + j] -= acc;
                }
            }
            let xs = from_column_major_slice_mut::<f64, usize, usize>(&mut x[b..e], nc, 1);
            faer::linalg::triangular_solve::solve_upper_triangular_in_place(l11.transpose(), xs, Parallelism::None);
        }
        for i in 0..n {
            rhs[self.perm_fwd[i]] = x[i];
        }
        let _ = &self.perm_inv;
    }
}
