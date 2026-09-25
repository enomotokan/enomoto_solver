//! このクレートの疎データ構造を一か所に集めたモジュール。行優先の [`CsrMat`]、
//! 列優先の [`CscMat`]、疎ベクトル [`SparseVec`]、疎/密を切り替える [`HybridVec`]、
//! 疎アキュムレータ [`SparseAccum`]、エポック印 [`EpochMarks`] と、それらの相互変換・
//! 疎×密の演算を提供する。転置・`(index, value)` のマージ・行列ベクトル積は
//! 他のモジュールで書き直さず、ここを呼ぶ (`simplex`、`presolve/*`、`interior_point::kkt`)。
//!
//! # 記憶形式
//!
//! 行列はどちらも古典的な「オフセット + 平坦な要素列」の圧縮形:
//!
//! ```text
//!   offsets: [0, o1, o2, ..., nnz]     (外側の長さ + 1 個)
//!   entries: [(inner, value), ...]     (nnz 個)
//! ```
//!
//! 外側の添字 `k` の非零は `entries[offsets[k]..offsets[k+1]]`。外側が行なら CSR
//! (`inner` は列番号)、列なら CSC (`inner` は行番号) で、違いはそれだけなので
//! 両者は同じ [`Compressed`] 構造体を共有する。添字と値は並列配列ではなく
//! `(inner, value)` の組で交互に持つ (利用側は常に両方を一緒に読むため)。
//! `row()`/`col()` は `&[(usize, f64)]` をそのまま返す。
//!
//! どちらも構築後は不変 (挿入不可)。行列を書き換える前処理は
//! `Vec<Vec<(usize, f64)>>` で作業し、最後に [`CsrMat`] に固める。
//!
//! # faer の `FaerCsr` と自前の [`CsrMat`]
//!
//!   - [`FaerCsr`] は faer の `SparseRowMat` の別名。`presolve` の公開インターフェースと、
//!     faer の Cholesky に渡す `interior_point::kkt` で使う。
//!   - [`CsrMat`]/[`CscMat`] は自前の型。単体法の内側ループで行・列を
//!     `&[(usize, f64)]` として直接読みたい場合や、同じ行列の CSR と CSC を並べて
//!     持ちたい場合に使う。
//!
//! 両者の橋渡しは [`csr_rows`]、[`CsrMat::from_faer`]、[`CsrMat::to_faer`] など。
//!
//! # 並列化
//!
//! 行優先の `A x` は出力要素が行ごとに独立なので、確保済みの `out` に対する
//! `par_iter_mut` で並列化する (内部で確保しない)。行優先の `A^T y` は `out` への
//! 散布 (scatter) になり、並列化にはアトミックかスレッドごとのバッファ確保が必要なので
//! 逐次のまま。[`CscMat`] では軸が逆なので、転置積のほうが並列になる。

// 疎データ構造の道具箱として、各表現の演算を (現在の呼び出し元が使うかどうかに
// かかわらず) 一通りそろえている。そのためこのファイルに限り `dead_code` を許可する。
// 未使用の項目もファイル末尾の単体テストで検証している。
#![allow(dead_code)]

use rayon::prelude::*;

// ===========================================================================
// 疎ベクトル
// ===========================================================================

/// 長さ `len` の疎ベクトル。非零を `(添字, 値)` の組で、作られた順のまま持つ。
///
/// 中身はただの `Vec<(usize, f64)>` で、[`SparseVec::entries`] はそのスライスを返す
/// (`CsrMat::row` / `CscMat::col` や `simplex::lu` の疎右辺 FTRAN と同じ形なので
/// 変換が要らない)。この型が加えるのは、密バッファとの scatter/gather、密ベクトルとの
/// 内積、刈り込み、疎/密の切り替え判定に使う密度などの演算。
///
/// **並び順と重複は作り手の責任。** ここでは勝手にソートも重複除去もしない。
/// 正規形が必要なら [`Self::sort`] か [`Self::canonicalize`] を呼ぶ。
#[derive(Clone, Debug, PartialEq)]
pub struct SparseVec {
    /// 論理的な (密にしたときの) 長さ。
    len: usize,
    /// 格納している `(添字, 値)` の組。
    entries: Vec<(usize, f64)>,
}

impl SparseVec {
    /// 長さ `len` の零ベクトル。
    pub fn zeros(len: usize) -> Self {
        SparseVec { len, entries: Vec::new() }
    }

    /// 長さ `len` の零ベクトル。非零 `cap` 個分の領域を予約しておく。
    pub fn with_capacity(len: usize, cap: usize) -> Self {
        SparseVec { len, entries: Vec::with_capacity(cap) }
    }

    /// 既存の `(添字, 値)` の列をそのまま受け取る。添字は `< len` と信頼する
    /// (デバッグビルドでのみ検査)。
    pub fn from_entries(len: usize, entries: Vec<(usize, f64)>) -> Self {
        debug_assert!(entries.iter().all(|&(i, _)| i < len), "sparse index out of range");
        SparseVec { len, entries }
    }

    /// 密ベクトル `dense` の非零 (厳密に 0 でないもの) を添字の昇順で取り出す。
    /// 許容誤差付きは [`Self::from_dense_tol`]。
    pub fn from_dense(dense: &[f64]) -> Self {
        let entries = dense.iter().enumerate().filter(|&(_, &v)| v != 0.0).map(|(i, &v)| (i, v)).collect();
        SparseVec { len: dense.len(), entries }
    }

    /// [`Self::from_dense`] と同じだが、`|v| <= tol` の要素は 0 とみなして捨てる
    /// (ほとんど 0 の密な作業ベクトルを疎化し、次の段で `O(nnz)` で走査するため)。
    pub fn from_dense_tol(dense: &[f64], tol: f64) -> Self {
        let entries = dense.iter().enumerate().filter(|&(_, &v)| v.abs() > tol).map(|(i, &v)| (i, v)).collect();
        SparseVec { len: dense.len(), entries }
    }

    /// 論理的な (密にしたときの) 長さ。
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// 格納している要素数。演算で打ち消し合った明示的な 0 も
    /// [`Self::prune`] するまでは数に含む。
    #[inline]
    pub fn nnz(&self) -> usize {
        self.entries.len()
    }

    /// 格納要素が 1 つもないか。
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 密度 `nnz / len` (疎/密の処理切り替えの判定に使う)。長さ 0 なら NaN ではなく `0.0`。
    #[inline]
    pub fn density(&self) -> f64 {
        if self.len == 0 {
            0.0
        } else {
            self.entries.len() as f64 / self.len as f64
        }
    }

    /// 格納している `(添字, 値)` の組のスライス。
    #[inline]
    pub fn entries(&self) -> &[(usize, f64)] {
        &self.entries
    }

    /// 格納している `(添字, 値)` の組の可変スライス。
    #[inline]
    pub fn entries_mut(&mut self) -> &mut [(usize, f64)] {
        &mut self.entries
    }

    /// ベクトルを消費して要素の列を返す。
    #[inline]
    pub fn into_entries(self) -> Vec<(usize, f64)> {
        self.entries
    }

    /// `(添字, 値)` を値で返すイテレータ。
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (usize, f64)> + '_ {
        self.entries.iter().copied()
    }

    /// 非零を 1 つ追加する (重複は検査しない)。
    #[inline]
    pub fn push(&mut self, index: usize, value: f64) {
        debug_assert!(index < self.len, "sparse index out of range");
        self.entries.push((index, value));
    }

    /// 全要素を捨てる。確保済み領域と論理長は保つので、ループで再確保せずに使い回せる。
    #[inline]
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 添字順にソートする (重複はまとめない)。
    pub fn sort(&mut self) {
        self.entries.sort_unstable_by_key(|&(i, _)| i);
    }

    /// `|v| <= tol` の要素を捨てる。
    pub fn prune(&mut self, tol: f64) {
        self.entries.retain(|&(_, v)| v.abs() > tol);
    }

    /// 正規形にする: 添字順にソートし、同じ添字の値を合算し、`|v| <= tol` を捨てる。
    /// 前処理が行を書き戻す前に使う (出力順が後のタイブレークに効くため)。
    pub fn canonicalize(&mut self, tol: f64) {
        self.sort();
        // 詰めて書き込む位置
        let mut write = 0usize;
        for read in 0..self.entries.len() {
            if write > 0 && self.entries[write - 1].0 == self.entries[read].0 {
                self.entries[write - 1].1 += self.entries[read].1;
            } else {
                self.entries[write] = self.entries[read];
                write += 1;
            }
        }
        self.entries.truncate(write);
        self.prune(tol);
    }

    /// 密ベクトルを新しい `Vec` として作る。
    pub fn to_dense(&self) -> Vec<f64> {
        let mut out = vec![0.0; self.len];
        self.scatter_into(&mut out);
        out
    }

    /// 非零を `out` (長さ `len`) に書き込む。`out` を事前に 0 クリア**しない**
    /// (既に 0 だと分かっている呼び出し側が `O(len)` のクリアを省けるように)。
    #[inline]
    pub fn scatter_into(&self, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.len);
        for &(i, v) in &self.entries {
            out[i] = v;
        }
    }

    /// `out += alpha * self` (このベクトルの台の上だけ)。
    #[inline]
    pub fn scatter_add_into(&self, alpha: f64, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.len);
        for &(i, v) in &self.entries {
            out[i] += alpha * v;
        }
    }

    /// `out` のうちこのベクトルの台の位置だけを 0 に戻す ([`Self::scatter_into`] の逆、
    /// `O(nnz)`)。
    #[inline]
    pub fn unscatter_from(&self, out: &mut [f64]) {
        debug_assert_eq!(out.len(), self.len);
        for &(i, _) in &self.entries {
            out[i] = 0.0;
        }
    }

    /// 中身を `dense` の `|v| > tol` の要素で置き換える (既存の領域を再利用)。
    pub fn gather_from(&mut self, dense: &[f64], tol: f64) {
        self.entries.clear();
        self.len = dense.len();
        for (i, &v) in dense.iter().enumerate() {
            if v.abs() > tol {
                self.entries.push((i, v));
            }
        }
    }

    /// 密ベクトルとの内積 `self · dense` (`O(nnz)`)。
    #[inline]
    pub fn dot_dense(&self, dense: &[f64]) -> f64 {
        debug_assert_eq!(dense.len(), self.len);
        self.entries.iter().map(|&(i, v)| v * dense[i]).sum()
    }

    /// 全要素を `k` 倍する。
    #[inline]
    pub fn scale(&mut self, k: f64) {
        for e in self.entries.iter_mut() {
            e.1 *= k;
        }
    }

    /// ユークリッドノルム。
    pub fn norm2(&self) -> f64 {
        self.entries.iter().map(|&(_, v)| v * v).sum::<f64>().sqrt()
    }
}

/// 要素スライス (`CsrMat::row` / `CscMat::col` など) と密ベクトルの内積
/// ([`SparseVec::dot_dense`] のスライス版)。
#[inline]
pub fn sparse_dot_dense(sparse: &[(usize, f64)], dense: &[f64]) -> f64 {
    sparse.iter().map(|&(i, v)| v * dense[i]).sum()
}

/// `dense += alpha * sparse` (`sparse` の台の上だけ)。
#[inline]
pub fn sparse_axpy_dense(alpha: f64, sparse: &[(usize, f64)], dense: &mut [f64]) {
    for &(i, v) in sparse {
        dense[i] += alpha * v;
    }
}

/// `out` を 0 クリアしてから `sparse` を書き込み、密ベクトルにする (`O(len + nnz)`)。
#[inline]
pub fn scatter_dense(sparse: &[(usize, f64)], out: &mut [f64]) {
    out.iter_mut().for_each(|v| *v = 0.0);
    for &(i, v) in sparse {
        out[i] = v;
    }
}

// ===========================================================================
// 疎/密ハイブリッドベクトル
// ===========================================================================

thread_local! {
    /// [`HybridVec::pack_scaled_dense`] が使い回す詰め込み用バッファ (スレッドごと)。
    static PACK_SCRATCH: std::cell::RefCell<Vec<(usize, f64)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// `(添字, 値)` の列**または**密配列のどちらかで持つベクトル。どちらにするかは
/// 構築時に充填率で一度だけ決める ([`HybridVec::pack`])。
///
/// `simplex::lu` の Forrest-Tomlin 更新の eta ベクトル用。eta は更新が進むと
/// 埋まっていき、ある充填率を超えると `(添字, 値)` 形よりも密配列の直線走査
/// (ベクトル化が効く) のほうが速いので、両方の形を一つのインターフェースで扱う。
/// 利用側のループは密ベクトルとの内積 ([`Self::dot_dense`]) と密ベクトルへの axpy
/// ([`Self::axpy_into_dense`]) の 2 種類だけ。
///
/// # 除外される添字
///
/// eta 自身のピボット位置は非対角ベクトルから除かれる。疎形では単に格納せず、
/// **密形ではその位置に `0.0` を置く** (両ループが「ピボット位置か」の判定なしに
/// 配列全体を走査できるように)。[`Self::pack`] は自動的にこれを満たし、
/// [`Self::remove_index`] も保つ。
///
/// `nnz` は明示的に保持する (密形の配列長は充填率を表さないため)。これにより
/// `simplex::lu` の再分解トリガーは表現によらず本当の充填量を測れる。
#[derive(Clone, Debug)]
pub enum HybridVec {
    /// 疎形: `(添字, 値)` の列。
    Sparse(Vec<(usize, f64)>),
    /// 密形: 全長の配列 `data` と非零数 `nnz`。
    Dense { data: Box<[f64]>, nnz: usize },
}

impl HybridVec {
    /// `pairs` (添字は `< len` で重複なし) を包む。`pairs.len()` が
    /// `dense_fraction * len` を超えたら密形にする。
    pub fn pack(len: usize, pairs: Vec<(usize, f64)>, dense_fraction: f64) -> Self {
        let nnz = pairs.len();
        if nnz as f64 > dense_fraction * len as f64 {
            let mut data = vec![0.0; len];
            for (i, v) in pairs {
                data[i] = v;
            }
            HybridVec::Dense { data: data.into_boxed_slice(), nnz }
        } else {
            HybridVec::Sparse(pairs)
        }
    }

    /// 組の列 `{ (i, scale * src[i]) : i != skip, scale * src[i] != 0.0 }` から
    /// [`Self::pack`] が作るのと全く同じベクトルを、その列を実際には作らずに構築する。
    ///
    /// `simplex::lu` の Forrest-Tomlin 更新がピボットごとに作る 2 つのベクトル
    /// (置き換え列の非対角部分 `src = a_tilde, scale = 1.0` と、新しい `R` eta
    /// `src = e_tilde, scale = -old_pivot`) はどちらもこの形で、呼び出し側が既に持つ
    /// 密バッファから直接読む。一時的な組の列を作らないので余計なヒープ確保がない。
    ///
    /// `crate::simplex::tiny_drop()` が正なら、`|scale * x| < tiny` も 0 とみなす版
    /// ([`Self::pack_scaled_dense_drop`]) に回す。
    ///
    /// `skip >= src.len()` なら [`Self::pack`] の範囲外添字と同じく panic する。
    pub fn pack_scaled_dense(src: &[f64], skip: usize, scale: f64, dense_fraction: f64) -> Self {
        // これ未満の絶対値を 0 とみなす閾値 (0 なら無効)
        let tiny = crate::simplex::tiny_drop();
        if tiny > 0.0 {
            return Self::pack_scaled_dense_drop(src, skip, scale, dense_fraction, tiny);
        }
        let len = src.len();
        // 1 パスで処理: すべての `(i, scale * src[i])` を使い回しの作業領域に無条件で
        // 書き、残す要素のときだけ書き込み位置を進める (分岐なしの詰め込み)。
        // これで組の列とその個数が `src` の 1 回の読み取りで得られる。
        // `skip` の範囲検査 (範囲外なら panic)
        let _ = src[skip];
        PACK_SCRATCH.with(|cell| {
            let mut buf = cell.borrow_mut();
            if buf.len() < len {
                buf.resize(len, (0, 0.0));
            }
            // 残した要素数 (= 次の書き込み位置)
            let mut nnz = 0usize;
            for (i, &x) in src.iter().enumerate() {
                let v = scale * x;
                // 範囲内: `nnz <= i < len <= buf.len()`
                buf[nnz] = (i, v);
                nnz += usize::from(v != 0.0 && i != skip);
            }
            if nnz as f64 > dense_fraction * len as f64 {
                let mut data = src.to_vec();
                if scale != 1.0 {
                    for d in data.iter_mut() {
                        *d *= scale;
                    }
                }
                // 「除外される添字」の約束: 密形はピボット位置に `0.0` を置く
                data[skip] = 0.0;
                HybridVec::Dense { data: data.into_boxed_slice(), nnz }
            } else {
                HybridVec::Sparse(buf[..nnz].to_vec())
            }
        })
    }

    /// [`Self::pack_scaled_dense`] のうち、`|scale * x| < tiny` も 0 とみなす版。
    fn pack_scaled_dense_drop(src: &[f64], skip: usize, scale: f64, dense_fraction: f64, tiny: f64) -> Self {
        let len = src.len();
        // 添字 i の要素 x を残すか
        let keep = |i: usize, x: f64| i != skip && (scale * x).abs() >= tiny;
        let nnz = src.iter().enumerate().filter(|&(i, &x)| keep(i, x)).count();
        if nnz as f64 > dense_fraction * len as f64 {
            let data: Vec<f64> = src.iter().enumerate().map(|(i, &x)| if keep(i, x) { scale * x } else { 0.0 }).collect();
            HybridVec::Dense { data: data.into_boxed_slice(), nnz }
        } else {
            let mut pairs = Vec::with_capacity(nnz);
            for (i, &x) in src.iter().enumerate() {
                if keep(i, x) {
                    pairs.push((i, scale * x));
                }
            }
            HybridVec::Sparse(pairs)
        }
    }

    /// 本当の非零数 (どちらの表現でも)。
    #[inline]
    pub fn nnz(&self) -> usize {
        match self {
            HybridVec::Sparse(v) => v.len(),
            HybridVec::Dense { nnz, .. } => *nnz,
        }
    }

    /// 添字 `index` の要素を取り除く (密形では `nnz` も正しく減らす)。
    /// 実際に要素があったかを返す (呼び出し側が充填量の合計を再集計せずに調整できる)。
    pub fn remove_index(&mut self, index: usize) -> bool {
        match self {
            HybridVec::Sparse(v) => {
                let before = v.len();
                v.retain(|&(i, _)| i != index);
                v.len() != before
            }
            HybridVec::Dense { data, nnz } => {
                if data[index] != 0.0 {
                    data[index] = 0.0;
                    *nnz -= 1;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// 非零のある各添字について `f` を 1 回ずつ呼ぶ (値は渡さない)。
    /// 両表現を一つの具体的なイテレータ型で表せないので、イテレータではなく
    /// クロージャを取る (ジェネリックなのでインライン化され、確保もしない)。
    /// 密形は配列全体を走査する。
    #[inline]
    pub fn for_each_index(&self, mut f: impl FnMut(usize)) {
        match self {
            HybridVec::Sparse(v) => {
                for &(i, _) in v {
                    f(i);
                }
            }
            HybridVec::Dense { data, .. } => {
                for (i, &x) in data.iter().enumerate() {
                    if x != 0.0 {
                        f(i);
                    }
                }
            }
        }
    }

    /// [`Self::for_each_index`] と同じだが、各要素の値も渡す。
    #[inline]
    pub fn for_each_entry(&self, mut f: impl FnMut(usize, f64)) {
        match self {
            HybridVec::Sparse(v) => {
                for &(i, x) in v {
                    f(i, x);
                }
            }
            HybridVec::Dense { data, .. } => {
                for (i, &x) in data.iter().enumerate() {
                    if x != 0.0 {
                        f(i, x);
                    }
                }
            }
        }
    }

    /// 格納要素を `(添字, 値)` の組の `Vec` として作る。密形では `O(len)` なので
    /// 反復ごとのループではなく、頻度の低い経路専用。
    pub fn to_pairs(&self) -> Vec<(usize, f64)> {
        match self {
            HybridVec::Sparse(v) => v.clone(),
            HybridVec::Dense { data, .. } => data.iter().enumerate().filter(|&(_, &v)| v != 0.0).map(|(i, &v)| (i, v)).collect(),
        }
    }

    /// 密ベクトルとの内積 `self · dense`。密形は配列全体を掛け合わせる
    /// (除外添字は `0.0` なので寄与しない)。
    #[inline]
    pub fn dot_dense(&self, dense: &[f64]) -> f64 {
        match self {
            HybridVec::Sparse(v) => v.iter().map(|&(i, v)| v * dense[i]).sum(),
            HybridVec::Dense { data, .. } => data.iter().zip(dense.iter()).map(|(&v, &d)| v * d).sum(),
        }
    }

    /// `dense += alpha * self`。密形は配列全体に対して行う (除外添字では何も変わらない)。
    #[inline]
    pub fn axpy_into_dense(&self, alpha: f64, dense: &mut [f64]) {
        match self {
            HybridVec::Sparse(v) => {
                for &(i, v) in v {
                    dense[i] += alpha * v;
                }
            }
            HybridVec::Dense { data, .. } => {
                for (d, &v) in dense.iter_mut().zip(data.iter()) {
                    *d += alpha * v;
                }
            }
        }
    }
}

// ===========================================================================
// エポック印 (O(1) でクリアできる訪問済み集合)
// ===========================================================================

/// 「今回のパスでこの添字に触れたか」を表す再利用可能な集合。配列を書き直す代わりに
/// カウンタ (エポック) を 1 つ進めるだけで `O(1)` でクリアできる (エポックスタンプ法)。
/// カウンタの桁あふれ (2^32 パスごと) も [`Self::begin`] で正しく処理する。
///
/// スタンプは `u32` (FTRAN/BTRAN の最内ループで引くので、キャッシュ占有を小さくするため)。
#[derive(Clone)]
pub struct EpochMarks {
    /// 添字ごとの最後に印を付けたエポック。
    stamps: Vec<u32>,
    /// 現在のパスのエポック番号 (`stamps[i] == epoch` なら印あり)。
    epoch: u32,
}

impl EpochMarks {
    /// 添字空間 `0..n` の、印が 1 つもない集合を作る。
    /// `stamps` は 0 初期化なので、エポックは 0 ではなく 1 から始める
    /// (0 だと最初の [`Self::begin`] 前から全添字に印があることになる)。
    pub fn new(n: usize) -> Self {
        EpochMarks { stamps: vec![0; n], epoch: 1 }
    }

    /// 添字空間の大きさ。
    #[inline]
    pub fn len(&self) -> usize {
        self.stamps.len()
    }

    /// 添字空間が空か。
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.stamps.is_empty()
    }

    /// 全印を消して新しいパスを始める。通常 `O(1)`。カウンタが一周したときだけ
    /// `O(n)` で `stamps` を 0 に戻す (2^32 パス前の古い印を今回の印と誤認しないため)。
    #[inline]
    pub fn begin(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.stamps.iter_mut().for_each(|s| *s = 0);
            self.epoch = 1;
        }
    }

    /// 添字 `i` に印を付ける。
    #[inline]
    pub fn mark(&mut self, i: usize) {
        self.stamps[i] = self.epoch;
    }

    /// 添字 `i` に今回のパスの印があるか。
    #[inline]
    pub fn is_marked(&self, i: usize) -> bool {
        self.stamps[i] == self.epoch
    }

    /// テスト専用: エポックを任意の値にする ([`Self::begin`] の一周処理を
    /// 2^32 パス回さずに試すため)。
    #[cfg(test)]
    pub fn force_epoch_for_test(&mut self, epoch: u32) {
        self.epoch = epoch;
    }

    /// このパスの残りについて添字 `i` の印を消す ([`SparseAccum::remove`] が使う)。
    #[inline]
    pub fn unmark(&mut self, i: usize) {
        // 現在のエポック以外の値なら「印なし」。1 つ前のエポックなら、次の `begin` までに
        // 将来のエポックと衝突しない。
        self.stamps[i] = self.epoch.wrapping_sub(1);
    }
}

// ===========================================================================
// 疎アキュムレータ (SPA)
// ===========================================================================

/// **疎アキュムレータ** (Gilbert/Moler/Schreiber)。使い回す密な値配列とエポック印の
/// 占有表を持ち、いくつかの疎ベクトルを合成した 1 本の疎ベクトルを `O(総 nnz)` で作る。
/// 値配列は全マージで共有し、エポックで「今回書いた値」と「前回の残り」を区別するので、
/// マージ間のクリアも不要 (`O(n)` の費用は構築時の 1 回だけ)。前処理で 2 行を
/// 組み合わせる箇所で使う。
///
/// 決定性: [`Self::take_sorted`] は出力前にパターンをソートするので、結果は
/// 順序付きマップで同じ計算をした場合とバイト単位で一致する。加算の順序は
/// 呼び出し順そのままなので、浮動小数点の結果も同一。
pub struct SparseAccum {
    /// 添字ごとの累積値 (印のない添字の値は無意味)。
    values: Vec<f64>,
    /// 今回の累積で触れた添字の印。
    marks: EpochMarks,
    /// 今回触れた添字の列 (初めて触れた順。`remove` 済みの添字も残りうる)。
    pattern: Vec<usize>,
}

impl SparseAccum {
    /// 添字空間 `0..n` のアキュムレータを作る (`O(n)` の確保 1 回。マージのループの
    /// 外で作って使い回す)。
    pub fn new(n: usize) -> Self {
        SparseAccum { values: vec![0.0; n], marks: EpochMarks::new(n), pattern: Vec::new() }
    }

    /// 添字空間の大きさ。
    #[inline]
    pub fn capacity(&self) -> usize {
        self.values.len()
    }

    /// 新しい累積を始める。エポックを進めるだけなので `O(1)`。
    #[inline]
    pub fn reset(&mut self) {
        self.marks.begin();
        self.pattern.clear();
    }

    /// 添字 `i` が今回の累積で有効か。
    #[inline]
    fn live(&self, i: usize) -> bool {
        self.marks.is_marked(i)
    }

    /// `self[i] += v`。初めて触れた添字はパターンに登録する。
    #[inline]
    pub fn add(&mut self, i: usize, v: f64) {
        debug_assert!(i < self.values.len(), "sparse index out of range");
        if self.live(i) {
            self.values[i] += v;
        } else {
            self.marks.mark(i);
            self.values[i] = v;
            self.pattern.push(i);
        }
    }

    /// `self[i] = v` (後から書いた値が勝つ。[`Self::load`] が使う)。
    #[inline]
    pub fn set(&mut self, i: usize, v: f64) {
        debug_assert!(i < self.values.len(), "sparse index out of range");
        if !self.live(i) {
            self.marks.mark(i);
            self.pattern.push(i);
        }
        self.values[i] = v;
    }

    /// 添字 `i` の現在値 (今回触れていなければ `0.0`)。
    #[inline]
    pub fn get(&self, i: usize) -> f64 {
        if self.live(i) {
            self.values[i]
        } else {
            0.0
        }
    }

    /// 添字 `i` が累積パターンに含まれるか (所属判定)。
    #[inline]
    pub fn contains(&self, i: usize) -> bool {
        self.live(i)
    }

    /// 添字 `i` をパターンから強制的に外す。値は 0 にし、`pattern` には残るが出力時に
    /// 除外される。消去した列を、浮動小数点の打ち消しに頼らず確実に取り除くために使う。
    #[inline]
    pub fn remove(&mut self, i: usize) {
        if self.live(i) {
            self.marks.unmark(i);
            self.values[i] = 0.0;
        }
    }

    /// `entries` から新しい累積を始める (同じ添字は後の値が勝つ)。
    pub fn load(&mut self, entries: &[(usize, f64)]) {
        self.reset();
        for &(i, v) in entries {
            self.set(i, v);
        }
    }

    /// `self += alpha * entries`。
    pub fn axpy(&mut self, alpha: f64, entries: &[(usize, f64)]) {
        for &(i, v) in entries {
            self.add(i, alpha * v);
        }
    }

    /// 累積結果を添字順の `(添字, 値)` 列として出力する。`|v| <= tol` は捨てる
    /// (`0.0` なら厳密な 0 だけ、`f64::NEG_INFINITY` なら全要素を残す)。
    /// その後は次の [`Self::load`] / [`Self::reset`] に使える。
    pub fn take_sorted(&mut self, tol: f64) -> Vec<(usize, f64)> {
        self.pattern.sort_unstable();
        let mut out: Vec<(usize, f64)> = Vec::with_capacity(self.pattern.len());
        for &i in &self.pattern {
            if !self.marks.is_marked(i) {
                continue; // `remove` で外された添字
            }
            if out.last().map(|&(k, _)| k) == Some(i) {
                continue; // `remove` 後に再 `add` すると同じ添字が 2 回入りうる
            }
            let v = self.values[i];
            if v.abs() > tol {
                out.push((i, v));
            }
        }
        out
    }

    /// [`Self::take_sorted`] の結果を長さ [`Self::capacity`] の [`SparseVec`] で返す。
    pub fn take_sorted_vec(&mut self, tol: f64) -> SparseVec {
        let n = self.capacity();
        SparseVec::from_entries(n, self.take_sorted(tol))
    }
}

/// 行の消去演算 `row - factor * pivot` を添字順で返す。`drop_col` は必ず取り除き、
/// それ以外も `|v| <= tol` は捨てる。前処理の代入消去 (`freevar`、`aggregator`) で使う。
///
/// `factor` は `drop_col` を消すように選ばれるが、浮動小数点では丸め誤差分の係数が
/// 残りうる (変数が構造上残ってしまう) ので、明示的に取り除く。
///
/// `accum` は呼び出し側で使い回す [`SparseAccum`] (両行が参照する全列を添字空間に
/// 含むこと)。開始時にリセットされる。
pub fn axpy_row(accum: &mut SparseAccum, row: &[(usize, f64)], pivot: &[(usize, f64)], factor: f64, drop_col: usize, tol: f64) -> Vec<(usize, f64)> {
    accum.load(row);
    accum.axpy(-factor, pivot);
    accum.remove(drop_col);
    accum.take_sorted(tol)
}

// ===========================================================================
// CSR / CSC 行列
// ===========================================================================

/// [`CsrMat`] と [`CscMat`] に共通の物理形式: `(添字, 値)` の不揃い配列を
/// 2 つの配列 (`offsets` と `entries`) だけで持つもの。構築後は変更しない。
/// 構築・転置・切り出しは「外側/内側」の軸で一度だけ書き、行/列の名前や寸法、
/// 向きに依存する演算は 2 つのラッパー側に置く。
#[derive(Clone, Debug, PartialEq)]
pub struct Compressed {
    /// 外側の添字 `k` の要素が `entries[offsets[k]..offsets[k+1]]` にある (長さ = 外側の数 + 1)。
    offsets: Vec<usize>,
    /// 全要素 `(内側の添字, 値)` を外側の順に並べたもの。
    entries: Vec<(usize, f64)>,
}

impl Compressed {
    /// 外側の添字ごとにまとめた `outers` を平坦化する。要素はそのままコピーする
    /// (ソート・マージ・0 除去はしない)。
    fn from_groups(outers: &[Vec<(usize, f64)>]) -> Self {
        let mut offsets = Vec::with_capacity(outers.len() + 1);
        offsets.push(0);
        let mut entries = Vec::with_capacity(outers.iter().map(|r| r.len()).sum());
        for outer in outers {
            entries.extend_from_slice(outer);
            offsets.push(entries.len());
        }
        Compressed { offsets, entries }
    }

    /// `outers` の転置を計数ソートで直接平坦形式に作る (`n_inner` は内側の軸の大きさ =
    /// 結果の外側の数)。1 パス目で各スライスの大きさを数え、2 パス目で埋める。
    /// 各出力スライス内の要素は元の外側の添字の昇順に並ぶ。
    fn from_groups_transposed(outers: &[Vec<(usize, f64)>], n_inner: usize) -> Self {
        let mut offsets = vec![0usize; n_inner + 1];
        for outer in outers {
            for &(j, _) in outer {
                offsets[j + 1] += 1;
            }
        }
        for k in 0..n_inner {
            offsets[k + 1] += offsets[k];
        }
        let mut entries = vec![(0usize, 0.0f64); offsets[n_inner]];
        // 各出力スライスの次の書き込み位置
        let mut cursor = offsets.clone();
        for (i, outer) in outers.iter().enumerate() {
            for &(j, v) in outer {
                entries[cursor[j]] = (i, v);
                cursor[j] += 1;
            }
        }
        Compressed { offsets, entries }
    }

    /// 圧縮形式のまま転置する ([`Self::from_groups_transposed`] と同じ計数ソートを
    /// 平坦形式から行う)。CSR ⇔ CSC 変換を `O(nnz + n_inner)` で行うためのもの。
    fn transposed(&self, n_inner: usize) -> Self {
        let mut offsets = vec![0usize; n_inner + 1];
        for &(j, _) in &self.entries {
            offsets[j + 1] += 1;
        }
        for k in 0..n_inner {
            offsets[k + 1] += offsets[k];
        }
        let mut entries = vec![(0usize, 0.0f64); offsets[n_inner]];
        // 各出力スライスの次の書き込み位置
        let mut cursor = offsets.clone();
        for i in 0..self.outer_len() {
            for &(j, v) in self.outer_slice(i) {
                entries[cursor[j]] = (i, v);
                cursor[j] += 1;
            }
        }
        Compressed { offsets, entries }
    }

    /// 外側の添字の数。
    #[inline]
    fn outer_len(&self) -> usize {
        self.offsets.len() - 1
    }

    /// 外側の添字 `k` の要素スライス。
    #[inline]
    fn outer_slice(&self, k: usize) -> &[(usize, f64)] {
        &self.entries[self.offsets[k]..self.offsets[k + 1]]
    }

    /// 全要素数。
    #[inline]
    fn nnz(&self) -> usize {
        self.entries.len()
    }

    /// 外側の添字ごとの `Vec<Vec<_>>` に戻す。
    fn to_groups(&self) -> Vec<Vec<(usize, f64)>> {
        (0..self.outer_len()).map(|k| self.outer_slice(k).to_vec()).collect()
    }
}

/// **圧縮行 (CSR)** 形式の行列。行 `i` の非零は `(列, 値)` の組として共有バッファに
/// 連続して並ぶ。制約行列を行ごとに走査する処理 (活動量の上下限、行シングルトン・
/// ダブルトン検出、`y^T A` の累積、基底行の取り出しなど) 向け。
#[derive(Clone, Debug, PartialEq)]
pub struct CsrMat {
    /// 行数。
    n_rows: usize,
    /// 列数。
    n_cols: usize,
    /// 外側 = 行の圧縮データ。
    inner: Compressed,
}

/// **圧縮列 (CSC)** 形式の行列。列 `j` の非零は `(行, 値)` の組として共有バッファに
/// 連続して並ぶ。列ごとに走査する処理 (単体法のプライシングの `a_j · y`、入る列の
/// FTRAN 右辺、列シングルトン・優越列の検出、列の双対境界伝播など) 向け。
/// 単体法は同じ行列の [`CsrMat`] と並べて持つ。
#[derive(Clone, Debug, PartialEq)]
pub struct CscMat {
    /// 行数。
    n_rows: usize,
    /// 列数。
    n_cols: usize,
    /// 外側 = 列の圧縮データ。
    inner: Compressed,
}

impl CsrMat {
    /// 疎な行 (`(列, 値)` の組の列) の並びから作る。要素はそのまま使う
    /// (ソート・重複マージ・0 除去をしない。必要なら [`Self::from_rows_canonical`])。
    pub fn from_rows(rows: &[Vec<(usize, f64)>], n_cols: usize) -> Self {
        CsrMat { n_rows: rows.len(), n_cols, inner: Compressed::from_groups(rows) }
    }

    /// 平坦化済みの形式から作る: 行 `i` は `entries[offsets[i]..offsets[i + 1]]`
    /// (`offsets[0] == 0`、`offsets` は行数 + 1 個)。
    pub(crate) fn from_flat(n_cols: usize, offsets: Vec<usize>, entries: Vec<(usize, f64)>) -> Self {
        debug_assert!(offsets.first() == Some(&0) && offsets.last() == Some(&entries.len()) && offsets.windows(2).all(|w| w[0] <= w[1]));
        CsrMat { n_rows: offsets.len() - 1, n_cols, inner: Compressed { offsets, entries } }
    }

    /// [`Self::from_rows`] と同じだが、各行を先に正規化する (列順にソート、
    /// 同じ列を合算、`|v| <= tol` を除去)。
    pub fn from_rows_canonical(rows: &[Vec<(usize, f64)>], n_cols: usize, tol: f64) -> Self {
        let canon: Vec<Vec<(usize, f64)>> = rows
            .iter()
            .map(|r| {
                let mut v = SparseVec::from_entries(n_cols, r.clone());
                v.canonicalize(tol);
                v.into_entries()
            })
            .collect();
        CsrMat::from_rows(&canon, n_cols)
    }

    /// `(行, 列, 値)` の三つ組から作る。各行の要素は三つ組の順に並び、重複はそのまま残る。
    pub fn from_triplets(n_rows: usize, n_cols: usize, triplets: &[(usize, usize, f64)]) -> Self {
        let mut rows = vec![Vec::new(); n_rows];
        for &(i, j, v) in triplets {
            rows[i].push((j, v));
        }
        CsrMat::from_rows(&rows, n_cols)
    }

    /// faer の [`FaerCsr`] を自前の行優先形式に読み込む。
    pub fn from_faer(mat: &FaerCsr) -> Self {
        let r = mat.as_ref();
        CsrMat::from_rows(&csr_rows(mat), r.ncols())
    }

    /// faer の [`FaerCsr`] に変換する。厳密な 0 は捨てる。
    pub fn to_faer(&self) -> FaerCsr {
        csr_from_rows(&self.to_rows(), self.n_cols)
    }

    /// 行数。
    #[inline]
    pub fn n_rows(&self) -> usize {
        self.n_rows
    }

    /// 列数。
    #[inline]
    pub fn n_cols(&self) -> usize {
        self.n_cols
    }

    /// 非零要素数。
    #[inline]
    pub fn nnz(&self) -> usize {
        self.inner.nnz()
    }

    /// 行 `i` の `(列, 値)` の組 (共有バッファのスライス。確保なし)。
    #[inline]
    pub fn row(&self, i: usize) -> &[(usize, f64)] {
        self.inner.outer_slice(i)
    }

    /// 行 `i` を列空間上の [`SparseVec`] として複製する。
    pub fn row_vec(&self, i: usize) -> SparseVec {
        SparseVec::from_entries(self.n_cols, self.row(i).to_vec())
    }

    /// 全行を `Vec<Vec<_>>` にする (前処理が書き換えに使う可変形式)。
    pub fn to_rows(&self) -> Vec<Vec<(usize, f64)>> {
        self.inner.to_groups()
    }

    /// 同じ行列を列優先形式にする (`O(nnz + n_cols)` の計数ソート)。
    /// 各列の要素は行の昇順に並ぶ。
    pub fn to_csc(&self) -> CscMat {
        CscMat { n_rows: self.n_rows, n_cols: self.n_cols, inner: self.inner.transposed(self.n_cols) }
    }

    /// 転置 `A^T` を [`CsrMat`] で返す (`A` の CSC と `A^T` の CSR は同じデータ)。
    pub fn transpose(&self) -> CsrMat {
        CsrMat { n_rows: self.n_cols, n_cols: self.n_rows, inner: self.inner.transposed(self.n_cols) }
    }

    /// 列ごとの非零数 (`O(nnz)`、転置を作らずに数える)。
    pub fn col_counts(&self) -> Vec<usize> {
        let mut counts = vec![0usize; self.n_cols];
        for i in 0..self.n_rows {
            for &(j, _) in self.row(i) {
                counts[j] += 1;
            }
        }
        counts
    }

    /// `A * x` を `out` (長さ `n_rows`) に書く。確保なし、行ごとに並列。
    pub fn mat_vec_into(&self, x: &[f64], out: &mut [f64]) {
        debug_assert_eq!(x.len(), self.n_cols);
        debug_assert_eq!(out.len(), self.n_rows);
        out.par_iter_mut().enumerate().for_each(|(i, o)| {
            *o = sparse_dot_dense(self.row(i), x);
        });
    }

    /// `A * x` を新しい `Vec` で返す。
    pub fn mat_vec(&self, x: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.n_rows];
        self.mat_vec_into(x, &mut out);
        out
    }

    /// `A^T * y` を `out` (長さ `n_cols`) に書く。`out` への散布なので逐次。
    pub fn mat_t_vec_into(&self, y: &[f64], out: &mut [f64]) {
        debug_assert_eq!(y.len(), self.n_rows);
        debug_assert_eq!(out.len(), self.n_cols);
        out.iter_mut().for_each(|v| *v = 0.0);
        for i in 0..self.n_rows {
            let yi = y[i];
            if yi == 0.0 {
                continue;
            }
            sparse_axpy_dense(yi, self.row(i), out);
        }
    }

    /// `A^T * y` を新しい `Vec` で返す。
    pub fn mat_t_vec(&self, y: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.n_cols];
        self.mat_t_vec_into(y, &mut out);
        out
    }

    /// **疎な** `y` に対する `A^T * y`。`accum` に直接累積し、`y` が非零の行だけを
    /// 触る (`O(それらの行の nnz の和)`)。結果の `|v| <= tol` は捨てる。
    pub fn mat_t_vec_sparse(&self, y: &[(usize, f64)], accum: &mut SparseAccum, tol: f64) -> SparseVec {
        accum.reset();
        for &(i, yi) in y {
            if yi == 0.0 {
                continue;
            }
            accum.axpy(yi, self.row(i));
        }
        SparseVec::from_entries(self.n_cols, accum.take_sorted(tol))
    }
}

impl CscMat {
    /// 疎な列 (`(行, 値)` の組の列) の並びから、要素をそのまま使って作る。
    pub fn from_cols(cols: &[Vec<(usize, f64)>], n_rows: usize) -> Self {
        CscMat { n_rows, n_cols: cols.len(), inner: Compressed::from_groups(cols) }
    }

    /// 行優先の入力から、計数ソートによる転置 1 回で直接列優先形式を作る。
    pub fn from_rows(rows: &[Vec<(usize, f64)>], n_cols: usize) -> Self {
        CscMat { n_rows: rows.len(), n_cols, inner: Compressed::from_groups_transposed(rows, n_cols) }
    }

    /// 任意の**要素ストリーム**から列優先形式を作る。`emit` は 2 回呼ばれ
    /// (1 回目で各列の大きさを数え、2 回目で埋める)、どちらでも `push(行, 列, 値)` の
    /// 受け口が渡される。`A` の行と別の不等式行を 1 つの行番号で連結した行列の列表示を、
    /// 中間の連結なしに作るのに使う (前処理の双対縮小・平行列検出)。
    ///
    /// `emit` は 2 回とも**完全に同じ要素を同じ順に**出すこと (要素を絞り込むなら
    /// 両方で同じ条件を使う)。各列の要素は出力順に並ぶので、行順に出せば行の昇順になる。
    pub fn from_entry_stream<F>(n_rows: usize, n_cols: usize, mut emit: F) -> Self
    where
        F: FnMut(&mut dyn FnMut(usize, usize, f64)),
    {
        let mut offsets = vec![0usize; n_cols + 1];
        // 1 回目: 各列の要素数を数える
        emit(&mut |_i, j, _v| offsets[j + 1] += 1);
        for k in 0..n_cols {
            offsets[k + 1] += offsets[k];
        }
        let mut entries = vec![(0usize, 0.0f64); offsets[n_cols]];
        // 2 回目: 各列の次の書き込み位置に埋める
        let mut cursor = offsets.clone();
        emit(&mut |i, j, v| {
            entries[cursor[j]] = (i, v);
            cursor[j] += 1;
        });
        CscMat { n_rows, n_cols, inner: Compressed { offsets, entries } }
    }

    /// `n_rows x n_cols` の零行列 (全列が存在し、すべて空)。
    pub fn empty(n_rows: usize, n_cols: usize) -> Self {
        CscMat { n_rows, n_cols, inner: Compressed { offsets: vec![0; n_cols + 1], entries: Vec::new() } }
    }

    /// faer の [`FaerCsr`] を直接列優先形式に読み込む。
    pub fn from_faer(mat: &FaerCsr) -> Self {
        let r = mat.as_ref();
        CscMat::from_rows(&csr_rows(mat), r.ncols())
    }

    /// 行数。
    #[inline]
    pub fn n_rows(&self) -> usize {
        self.n_rows
    }

    /// 列数。
    #[inline]
    pub fn n_cols(&self) -> usize {
        self.n_cols
    }

    /// 非零要素数。
    #[inline]
    pub fn nnz(&self) -> usize {
        self.inner.nnz()
    }

    /// 列 `j` の `(行, 値)` の組 (共有バッファのスライス。確保なし)。
    /// 列との内積だけが必要なループ (入る変数の選択、`chuzc`、steepest-edge 重みの
    /// 更新など) は密にせずこれを使う。
    #[inline]
    pub fn col(&self, j: usize) -> &[(usize, f64)] {
        self.inner.outer_slice(j)
    }

    /// 列 `j` を行空間上の [`SparseVec`] として複製する。
    pub fn col_vec(&self, j: usize) -> SparseVec {
        SparseVec::from_entries(self.n_rows, self.col(j).to_vec())
    }

    /// 列 `j` を密にして `out` (長さ `n_rows`、先に 0 クリア) に書く (`O(nnz_j + n_rows)`)。
    #[inline]
    pub fn col_into_dense(&self, j: usize, out: &mut [f64]) {
        scatter_dense(self.col(j), out);
    }

    /// 列 `j` を新しい密な `Vec` で返す。
    pub fn col_dense(&self, j: usize) -> Vec<f64> {
        let mut out = vec![0.0; self.n_rows];
        self.col_into_dense(j, &mut out);
        out
    }

    /// 全列を `Vec<Vec<_>>` にする。
    pub fn to_cols(&self) -> Vec<Vec<(usize, f64)>> {
        self.inner.to_groups()
    }

    /// 行優先形式に戻す (`O(nnz + n_rows)` の計数ソート。[`CsrMat::to_csc`] の逆)。
    pub fn to_csr(&self) -> CsrMat {
        CsrMat { n_rows: self.n_rows, n_cols: self.n_cols, inner: self.inner.transposed(self.n_rows) }
    }

    /// 行ごとの非零数 (`O(nnz)`)。
    pub fn row_counts(&self) -> Vec<usize> {
        let mut counts = vec![0usize; self.n_rows];
        for j in 0..self.n_cols {
            for &(i, _) in self.col(j) {
                counts[i] += 1;
            }
        }
        counts
    }

    /// `A * x` を `out` (長さ `n_rows`) に書く。この向きでは `out` への散布なので逐次で、
    /// `x` の 0 成分は飛ばす。
    pub fn mat_vec_into(&self, x: &[f64], out: &mut [f64]) {
        debug_assert_eq!(x.len(), self.n_cols);
        debug_assert_eq!(out.len(), self.n_rows);
        out.iter_mut().for_each(|v| *v = 0.0);
        for j in 0..self.n_cols {
            let xj = x[j];
            if xj == 0.0 {
                continue;
            }
            sparse_axpy_dense(xj, self.col(j), out);
        }
    }

    /// `A * x` を新しい `Vec` で返す。
    pub fn mat_vec(&self, x: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.n_rows];
        self.mat_vec_into(x, &mut out);
        out
    }

    /// **疎な** `x` に対する `A * x`。`x` が非零の列だけが寄与する
    /// (単体法の右辺 `b - N x_N` の形)。結果の `|v| <= tol` は捨てる。
    pub fn mat_vec_sparse(&self, x: &[(usize, f64)], accum: &mut SparseAccum, tol: f64) -> SparseVec {
        accum.reset();
        for &(j, xj) in x {
            if xj == 0.0 {
                continue;
            }
            accum.axpy(xj, self.col(j));
        }
        SparseVec::from_entries(self.n_rows, accum.take_sorted(tol))
    }

    /// `A^T * y` を `out` (長さ `n_cols`) に書く。列ごとに独立なので並列。
    pub fn mat_t_vec_into(&self, y: &[f64], out: &mut [f64]) {
        debug_assert_eq!(y.len(), self.n_rows);
        debug_assert_eq!(out.len(), self.n_cols);
        out.par_iter_mut().enumerate().for_each(|(j, o)| {
            *o = sparse_dot_dense(self.col(j), y);
        });
    }

    /// `A^T * y` を新しい `Vec` で返す。
    pub fn mat_t_vec(&self, y: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.n_cols];
        self.mat_t_vec_into(y, &mut out);
        out
    }
}

/// 列を**昇順に 1 本ずつ**出力して [`CscMat`] を組み立てるビルダー。要素は最終の
/// 平坦バッファの末尾に直接追加し、列の境界はその時点のバッファ長になる
/// (左向き LU 分解のように列順に要素を生み出す処理向け。計数パスが不要)。
///
/// ```text
///     let mut b = CscBuilder::new(n_rows);
///     for j in 0..n_cols {
///         for (i, v) in column_j_entries() { b.push(i, v); }
///         b.end_column();
///     }
///     let mat = b.build();
/// ```
pub struct CscBuilder {
    /// 行数。
    n_rows: usize,
    /// 閉じた列の境界 (`Compressed::offsets` と同じ意味。最初は `[0]`)。
    offsets: Vec<usize>,
    /// これまでに追加した全要素。
    entries: Vec<(usize, f64)>,
}

impl CscBuilder {
    /// 行数 `n_rows`、列 0 本のビルダーを作る。
    pub fn new(n_rows: usize) -> Self {
        CscBuilder { n_rows, offsets: vec![0], entries: Vec::new() }
    }

    /// [`Self::new`] と同じだが、`n_cols` 列と `nnz` 要素分の領域を予約する。
    pub fn with_capacity(n_rows: usize, n_cols: usize, nnz: usize) -> Self {
        let mut offsets = Vec::with_capacity(n_cols + 1);
        offsets.push(0);
        CscBuilder { n_rows, offsets, entries: Vec::with_capacity(nnz) }
    }

    /// 構築中の列に要素を 1 つ追加する。
    #[inline]
    pub fn push(&mut self, row: usize, value: f64) {
        debug_assert!(row < self.n_rows, "row index out of range");
        self.entries.push((row, value));
    }

    /// 現在の列を閉じて次の列を始める。要素のない列も含め、列ごとに 1 回呼ぶこと。
    #[inline]
    pub fn end_column(&mut self) {
        self.offsets.push(self.entries.len());
    }

    /// これまでに閉じた列の数。
    #[inline]
    pub fn columns_built(&self) -> usize {
        self.offsets.len() - 1
    }

    /// 行列を完成させる。列数は閉じた列の数。
    pub fn build(self) -> CscMat {
        CscMat { n_rows: self.n_rows, n_cols: self.offsets.len() - 1, inner: Compressed { offsets: self.offsets, entries: self.entries } }
    }
}

// ===========================================================================
// faer との相互変換
// ===========================================================================

/// faer の行優先疎行列。`presolve` の公開インターフェースと `interior_point::kkt`
/// (faer の Cholesky に直接渡す) で使う型。[`CsrMat`] との使い分けはモジュール説明を参照。
pub type FaerCsr = faer::sparse::SparseRowMat<usize, f64>;

/// 疎な行 (`(列, 値)` の列) の並びから [`FaerCsr`] を作る。厳密な 0 は捨てる。
/// 行数は `rows.len()`、列数は `n_cols`。重複列がなければ高速経路
/// ([`csr_from_rows_direct`])、あれば faer の三つ組ビルダーを使う。
pub fn csr_from_rows(rows: &[Vec<(usize, f64)>], n_cols: usize) -> FaerCsr {
    if let Some(m) = csr_from_rows_direct(rows, n_cols) {
        return m;
    }
    // 重複列ありの場合: faer の三つ組ビルダーで重複を合算させる
    let mut triplets = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        for &(j, v) in row {
            if v != 0.0 {
                triplets.push((i, j, v));
            }
        }
    }
    FaerCsr::try_new_from_triplets(rows.len(), n_cols, &triplets).expect("valid CSR triplets")
}

/// [`csr_from_rows`] の `O(nnz)` 高速経路。どの行にも非零の重複列がなければ、faer の
/// `try_new_from_triplets` と完全に同じ行列 (行ポインタ・列添字・値がビット単位で一致)
/// になる: 各行の非零を列順に並べるだけでよい (行は通常ソート済み)。
///
/// 重複列があれば `None` (三つ組経路に戻る。faer は不安定ソート後に重複を合算するので、
/// 合算順序と丸めを再現するには faer を通すしかない)。範囲外の列でも `None`
/// (元のエラー/panic の経路をそのまま使うため)。
fn csr_from_rows_direct(rows: &[Vec<(usize, f64)>], n_cols: usize) -> Option<FaerCsr> {
    let nnz_upper: usize = rows.iter().map(|r| r.len()).sum();
    let mut builder = CsrRowBuilder::with_capacity(n_cols, rows.len(), nnz_upper);
    for row in rows {
        if !builder.push_row(row) {
            return None;
        }
    }
    Some(builder.finish())
}

/// `m` が既に [`csr_from_rows`]`(&csr_rows(m), ncols)` で作り直したものと同一か
/// (圧縮形式で行ごとの nnz 配列なし、格納された 0 なし、各行の列が狭義単調増加)。
/// そうなら、`Vec<Vec<_>>` を経由して作り直すだけの処理は `m.clone()` で代用できる
/// (ビット単位で同一、ソート不要)。
pub(crate) fn csr_is_canonical(m: &FaerCsr) -> bool {
    let r = m.as_ref();
    r.nnz_per_row().is_none() && r.values().iter().all(|&v| v != 0.0) && (0..r.nrows()).all(|i| r.col_indices_of_row_raw(i).windows(2).all(|w| w[0] < w[1]))
}

/// [`csr_from_rows_direct`] の逐次版 (faer の三つ組ビルダーとの完全一致の約束も同じ)。
/// `Vec<Vec<_>>` を作らずに複数の出所から行列を組み立てる呼び出し側向け
/// (例: `propagate::rebuild_g_ref` — 実際の行は参照で、有限境界ごとに単独行を追加)。
/// [`CsrRowBuilder::push_row`] が `false` (重複列または範囲外列) を返したら、
/// 呼び出し側は全行を [`csr_from_rows`] で作り直すこと。
pub(crate) struct CsrRowBuilder {
    /// 列数。
    n_cols: usize,
    /// 行ポインタ (`row_ptr[i]..row_ptr[i+1]` が行 `i`。最初は `[0]`)。
    row_ptr: Vec<usize>,
    /// 全行の列添字 (各行内で昇順)。
    col_ind: Vec<usize>,
    /// `col_ind` と同じ並びの値。
    values: Vec<f64>,
    /// 未ソートの行を並べ替えるための作業領域。
    scratch: Vec<(usize, f64)>,
}

impl CsrRowBuilder {
    /// 列数 `n_cols` のビルダーを作り、`rows` 行・`nnz` 要素分の領域を予約する。
    pub(crate) fn with_capacity(n_cols: usize, rows: usize, nnz: usize) -> Self {
        let mut row_ptr = Vec::with_capacity(rows + 1);
        row_ptr.push(0);
        CsrRowBuilder { n_cols, row_ptr, col_ind: Vec::with_capacity(nnz), values: Vec::with_capacity(nnz), scratch: Vec::new() }
    }

    /// 1 行を追加する (厳密な 0 は捨て、列順でなければ並べ替える)。重複列または
    /// 範囲外の列があれば `false` を返す (その場合ビルダーは途中状態なので捨てること)。
    pub(crate) fn push_row(&mut self, row: &[(usize, f64)]) -> bool {
        // この行の書き込み開始位置
        let start = self.col_ind.len();
        // 列が狭義単調増加で来たか
        let mut sorted = true;
        let mut prev: Option<usize> = None;
        for &(j, v) in row {
            if v == 0.0 {
                continue;
            }
            if j >= self.n_cols {
                return false;
            }
            if let Some(p) = prev {
                if j <= p {
                    sorted = false;
                }
            }
            prev = Some(j);
            self.col_ind.push(j);
            self.values.push(v);
        }
        if !sorted {
            self.scratch.clear();
            self.scratch.extend(self.col_ind[start..].iter().copied().zip(self.values[start..].iter().copied()));
            self.scratch.sort_unstable_by_key(|&(j, _)| j);
            for k in 1..self.scratch.len() {
                if self.scratch[k].0 == self.scratch[k - 1].0 {
                    return false;
                }
            }
            for (k, &(j, v)) in self.scratch.iter().enumerate() {
                self.col_ind[start + k] = j;
                self.values[start + k] = v;
            }
        }
        self.row_ptr.push(self.col_ind.len());
        true
    }

    /// 要素が 1 つ `(j, v)` だけの行 (`v != 0`, `j < n_cols`。例: 境界の行) を追加する。
    pub(crate) fn push_singleton(&mut self, j: usize, v: f64) {
        debug_assert!(v != 0.0);
        assert!(j < self.n_cols, "column out of range");
        self.col_ind.push(j);
        self.values.push(v);
        self.row_ptr.push(self.col_ind.len());
    }

    /// 組み立てを終えて faer の [`FaerCsr`] を返す。
    pub(crate) fn finish(self) -> FaerCsr {
        let nrows = self.row_ptr.len() - 1;
        // 全行は `push_row` (範囲内・狭義単調増加の列。重複は拒否) か `push_singleton`
        // (範囲内の 1 列) を通っており、`row_ptr` も構成上単調なので、faer の
        // `new_checked` による再検証は省く (デバッグビルドでのみ検査する)。
        debug_assert!(self.row_ptr.windows(2).all(|w| w[0] <= w[1]) && *self.row_ptr.last().unwrap() == self.col_ind.len());
        debug_assert!((0..nrows).all(|i| {
            let r = &self.col_ind[self.row_ptr[i]..self.row_ptr[i + 1]];
            r.iter().all(|&j| j < self.n_cols) && r.windows(2).all(|w| w[0] < w[1])
        }));
        // SAFETY: `new_checked` が確かめる不変条件 (`col_ind.len()` で終わる単調な
        // 行ポインタ、各行内で範囲内かつ狭義単調増加の列添字) は上記のとおり構成上成り立つ。
        let symbolic = unsafe { faer::sparse::SymbolicSparseRowMat::new_unchecked(nrows, self.n_cols, self.row_ptr, None, self.col_ind) };
        FaerCsr::new(symbolic, self.values)
    }
}

/// faer の [`FaerCsr`] の行 `i` を `(列, 値)` のイテレータとして返す
/// (格納された 0 もそのまま返す)。
#[inline]
pub fn csr_row_iter(mat: &FaerCsr, i: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
    let r = mat.as_ref();
    r.col_indices_of_row(i).zip(r.values_of_row(i)).map(|(j, &v)| (j, v))
}

/// 容量を先に予約してから `iter` を集める `collect` (`cap` は要素数の上限。
/// `filter` 付きイテレータは正確な長さが分からず、普通の `collect` だと段階的に伸長するため)。
#[inline]
pub fn collect_with_capacity<T>(cap: usize, iter: impl Iterator<Item = T>) -> Vec<T> {
    let mut v = Vec::with_capacity(cap);
    v.extend(iter);
    v
}

/// faer の [`FaerCsr`] の行 `i` を `(列, 値)` の `Vec` として複製する。
pub fn csr_row_vec(mat: &FaerCsr, i: usize) -> Vec<(usize, f64)> {
    csr_row_iter(mat, i).collect()
}

/// faer の [`FaerCsr`] の全行を `Vec<Vec<_>>` にする (前処理が書き換えに使い、
/// [`csr_from_rows`] で固め直す形式)。格納された 0 も含む ([`csr_rows_pruned`] は除く)。
pub fn csr_rows(mat: &FaerCsr) -> Vec<Vec<(usize, f64)>> {
    (0..mat.as_ref().nrows()).map(|i| csr_row_vec(mat, i)).collect()
}

/// [`csr_rows`] と同じだが、厳密な 0 を除く (行の長さやシングルトン/ダブルトン判定など、
/// 行の台で判断する処理向け。格納された 0 を数えると誤判定するため)。
pub fn csr_rows_pruned(mat: &FaerCsr) -> Vec<Vec<(usize, f64)>> {
    (0..mat.as_ref().nrows()).map(|i| csr_row_iter(mat, i).filter(|&(_, v)| v != 0.0).collect()).collect()
}

/// faer の [`FaerCsr`] の列ごとの非零数 (`O(nnz)`、転置を作らない)。
pub fn csr_col_counts(mat: &FaerCsr) -> Vec<usize> {
    let r = mat.as_ref();
    let mut counts = vec![0usize; r.ncols()];
    for i in 0..r.nrows() {
        for j in r.col_indices_of_row(i) {
            counts[j] += 1;
        }
    }
    counts
}

/// faer の [`FaerCsr`] の列優先表示 ([`CscMat`]) を計数ソート 1 回で作る。
pub fn csr_to_csc(mat: &FaerCsr) -> CscMat {
    CscMat::from_faer(mat)
}

/// faer の `mat` について `mat * x` を `out` (長さ `mat.nrows()`) に書く (確保なし、行ごとに並列)。
pub fn csr_mat_vec_into(mat: &FaerCsr, x: &[f64], out: &mut [f64]) {
    let r = mat.as_ref();
    out.par_iter_mut().enumerate().for_each(|(i, o)| {
        *o = r.col_indices_of_row(i).zip(r.values_of_row(i)).map(|(j, &v)| v * x[j]).sum();
    });
}

/// faer の `mat` について `mat^T * y` を `out` (長さ `mat.ncols()`) に書く (確保なし、逐次)。
pub fn csr_mat_t_vec_into(mat: &FaerCsr, y: &[f64], out: &mut [f64]) {
    for v in out.iter_mut() {
        *v = 0.0;
    }
    let r = mat.as_ref();
    for i in 0..r.nrows() {
        let yi = y[i];
        if yi == 0.0 {
            continue;
        }
        for (j, &v) in r.col_indices_of_row(i).zip(r.values_of_row(i)) {
            out[j] += v * yi;
        }
    }
}

/// `mat * x` を新しい `Vec` で返す (`mat_vec_into` の確保版)。
pub fn csr_mat_vec(mat: &FaerCsr, x: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; mat.nrows()];
    csr_mat_vec_into(mat, x, &mut out);
    out
}

/// `mat^T * y` を長さ `n_cols` の新しい `Vec` で返す (`csr_mat_t_vec_into` の確保版)。
pub fn csr_mat_t_vec(mat: &FaerCsr, n_cols: usize, y: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; n_cols];
    csr_mat_t_vec_into(mat, y, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2 つの値がほぼ等しいか (差が 1e-12 未満)。
    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    /// テスト用の 3x4 行列 (下のコメントの形) を行の並びで返す。
    fn sample_rows() -> Vec<Vec<(usize, f64)>> {
        //  [ 1  0  2  0 ]
        //  [ 0  3  0  0 ]
        //  [ 4  0  0  5 ]
        vec![vec![(0, 1.0), (2, 2.0)], vec![(1, 3.0)], vec![(0, 4.0), (3, 5.0)]]
    }

    /// CSR → CSC → CSR の往復で行列が変わらないこと。
    #[test]
    fn csr_round_trips_through_csc_unchanged() {
        let csr = CsrMat::from_rows(&sample_rows(), 4);
        let back = csr.to_csc().to_csr();
        assert_eq!(csr, back);
        assert_eq!(back.to_rows(), sample_rows());
    }

    /// CSC の各列が手計算の転置と一致すること。
    #[test]
    fn csc_columns_match_the_hand_transposed_matrix() {
        let csc = CsrMat::from_rows(&sample_rows(), 4).to_csc();
        assert_eq!(csc.n_rows(), 3);
        assert_eq!(csc.n_cols(), 4);
        assert_eq!(csc.col(0), &[(0, 1.0), (2, 4.0)]);
        assert_eq!(csc.col(1), &[(1, 3.0)]);
        assert_eq!(csc.col(2), &[(0, 2.0)]);
        assert_eq!(csc.col(3), &[(2, 5.0)]);
    }

    /// 行から直接作った CSC と CSR から変換した CSC が一致すること。
    #[test]
    fn csc_built_from_rows_matches_csc_built_by_conversion() {
        let rows = sample_rows();
        assert_eq!(CscMat::from_rows(&rows, 4), CsrMat::from_rows(&rows, 4).to_csc());
    }

    /// 転置の転置が元の行列に戻ること。
    #[test]
    fn transpose_of_transpose_is_the_original() {
        let csr = CsrMat::from_rows(&sample_rows(), 4);
        assert_eq!(csr.transpose().transpose(), csr);
    }

    /// CSR と CSC で `A x` と `A^T y` の結果が一致すること。
    #[test]
    fn both_orientations_agree_on_mat_vec_and_its_transpose() {
        let csr = CsrMat::from_rows(&sample_rows(), 4);
        let csc = csr.to_csc();
        let x = [1.0, 2.0, 3.0, 4.0];
        let y = [1.0, -1.0, 2.0];

        // A x = [1*1 + 2*3, 3*2, 4*1 + 5*4] = [7, 6, 24]
        assert_eq!(csr.mat_vec(&x), vec![7.0, 6.0, 24.0]);
        assert_eq!(csc.mat_vec(&x), vec![7.0, 6.0, 24.0]);

        // A^T y = [1*1 + 4*2, 3*(-1), 2*1, 5*2] = [9, -3, 2, 10]
        assert_eq!(csr.mat_t_vec(&y), vec![9.0, -3.0, 2.0, 10.0]);
        assert_eq!(csc.mat_t_vec(&y), vec![9.0, -3.0, 2.0, 10.0]);
    }

    /// 疎ベクトルとの積が密ベクトル版と一致すること。
    #[test]
    fn sparse_products_match_their_dense_counterparts() {
        let csr = CsrMat::from_rows(&sample_rows(), 4);
        let csc = csr.to_csc();
        let mut accum_cols = SparseAccum::new(4);
        let mut accum_rows = SparseAccum::new(3);

        let y_sparse = [(0usize, 1.0f64), (2usize, 2.0f64)];
        let mut y_dense = vec![0.0; 3];
        for &(i, v) in &y_sparse {
            y_dense[i] = v;
        }
        assert_eq!(csr.mat_t_vec_sparse(&y_sparse, &mut accum_cols, 0.0).to_dense(), csr.mat_t_vec(&y_dense));

        let x_sparse = [(2usize, 3.0f64), (3usize, 4.0f64)];
        let mut x_dense = vec![0.0; 4];
        for &(j, v) in &x_sparse {
            x_dense[j] = v;
        }
        assert_eq!(csc.mat_vec_sparse(&x_sparse, &mut accum_rows, 0.0).to_dense(), csc.mat_vec(&x_dense));
    }

    /// 列ごと・行ごとの非零数が両方の向きで一致すること。
    #[test]
    fn col_counts_and_row_counts_agree_across_orientations() {
        let csr = CsrMat::from_rows(&sample_rows(), 4);
        assert_eq!(csr.col_counts(), vec![2, 1, 1, 1]);
        assert_eq!(csr.to_csc().row_counts(), vec![2, 1, 2]);
    }

    /// faer の `FaerCsr` との相互変換が往復で一致すること。
    #[test]
    fn faer_interop_round_trips() {
        let rows = sample_rows();
        let faer = csr_from_rows(&rows, 4);
        assert_eq!(csr_rows(&faer), rows);
        assert_eq!(csr_row_vec(&faer, 2), vec![(0, 4.0), (3, 5.0)]);
        assert_eq!(csr_col_counts(&faer), vec![2, 1, 1, 1]);
        assert_eq!(CsrMat::from_faer(&faer), CsrMat::from_rows(&rows, 4));
        assert_eq!(csr_rows(&CsrMat::from_rows(&rows, 4).to_faer()), rows);
        assert_eq!(csr_to_csc(&faer), CscMat::from_rows(&rows, 4));
    }

    /// `CsrMat::from_flat` + `to_csc` が `from_rows` 系と一致すること (標準形の構築が依存)。
    #[test]
    fn csr_from_flat_and_to_csc_match_from_rows() {
        let rows: Vec<Vec<(usize, f64)>> = vec![vec![(0, 1.0), (3, -2.0), (5, 1.0)], vec![], vec![(1, 0.5), (6, 1.0)], vec![(0, -1.0), (1, 2.0), (2, 3.0), (7, 1.0)]];
        let mut offsets = vec![0usize];
        let mut entries = Vec::new();
        for r in &rows {
            entries.extend_from_slice(r);
            offsets.push(entries.len());
        }
        let flat = CsrMat::from_flat(8, offsets, entries);
        assert_eq!(flat, CsrMat::from_rows(&rows, 8));
        assert_eq!(flat.to_csc(), CscMat::from_rows(&rows, 8));
    }

    /// 重複なしの高速経路が faer の三つ組ビルダーと完全一致すること (構造・値・0 除去・未ソート行の並べ替え)。
    #[test]
    fn csr_from_rows_direct_matches_faer_triplets() {
        let rows: Vec<Vec<(usize, f64)>> = vec![
            vec![(3, 1.5), (0, -2.0), (2, 0.0), (1, 7.0)],
            vec![],
            vec![(0, 0.0)],
            vec![(4, -0.0), (2, 3.25), (4, 1.0)],
            vec![(0, 1.0), (1, 2.0), (2, 3.0), (3, 4.0), (4, 5.0)],
        ];
        let direct = csr_from_rows_direct(&rows, 5).expect("no duplicates");
        let mut triplets = Vec::new();
        for (i, row) in rows.iter().enumerate() {
            for &(j, v) in row {
                if v != 0.0 {
                    triplets.push((i, j, v));
                }
            }
        }
        let faer = FaerCsr::try_new_from_triplets(rows.len(), 5, &triplets).unwrap();
        assert_eq!(direct.as_ref().row_ptrs(), faer.as_ref().row_ptrs());
        assert_eq!(direct.as_ref().col_indices(), faer.as_ref().col_indices());
        let dv: Vec<u64> = direct.as_ref().values().iter().map(|v| v.to_bits()).collect();
        let fv: Vec<u64> = faer.as_ref().values().iter().map(|v| v.to_bits()).collect();
        assert_eq!(dv, fv);
        // 本物の重複があれば faer 自身のマージに任せること
        assert!(csr_from_rows_direct(&[vec![(1, 1.0), (1, 2.0)]], 3).is_none());
        assert_eq!(csr_rows(&csr_from_rows(&[vec![(1, 1.0), (1, 2.0)]], 3)), vec![vec![(1, 3.0)]]);
    }

    /// `csr_rows_pruned` が格納された 0 を除くこと。
    #[test]
    fn csr_rows_pruned_drops_explicitly_stored_zeros() {
        // `csr_from_rows` は 0 を捨てるので、格納された 0 は faer の三つ組ビルダーで直接作る
        let faer = FaerCsr::try_new_from_triplets(1, 3, &[(0, 0, 1.0), (0, 1, 0.0), (0, 2, 3.0)]).unwrap();
        assert_eq!(csr_row_vec(&faer, 0), vec![(0, 1.0), (1, 0.0), (2, 3.0)]);
        assert_eq!(csr_rows(&faer), vec![vec![(0, 1.0), (1, 0.0), (2, 3.0)]]);
        assert_eq!(csr_rows_pruned(&faer), vec![vec![(0, 1.0), (2, 3.0)]]);
    }

    /// 疎ベクトルの scatter/gather の往復と、内積が密計算と一致すること。
    #[test]
    fn sparse_vec_scatter_gather_round_trips_and_dot_matches_dense() {
        let v = SparseVec::from_entries(5, vec![(1, 2.0), (4, -3.0)]);
        assert_eq!(v.nnz(), 2);
        assert!(approx(v.density(), 0.4));
        assert_eq!(v.to_dense(), vec![0.0, 2.0, 0.0, 0.0, -3.0]);

        let mut dense = vec![7.0; 5];
        dense.iter_mut().for_each(|x| *x = 0.0);
        v.scatter_into(&mut dense);
        let mut gathered = SparseVec::zeros(5);
        gathered.gather_from(&dense, 0.0);
        assert_eq!(gathered, v);

        v.unscatter_from(&mut dense);
        assert_eq!(dense, vec![0.0; 5]);

        let w = [1.0, 10.0, 100.0, 1000.0, 10000.0];
        assert!(approx(v.dot_dense(&w), 2.0 * 10.0 - 3.0 * 10000.0));
        assert!(approx(sparse_dot_dense(v.entries(), &w), v.dot_dense(&w)));
    }

    /// `canonicalize` がソート・重複合算・刈り込みを行うこと。
    #[test]
    fn sparse_vec_canonicalize_sorts_merges_and_prunes() {
        let mut v = SparseVec::from_entries(6, vec![(4, 1.0), (1, 2.0), (4, -1.0), (0, 1e-14), (3, 5.0)]);
        v.canonicalize(1e-12);
        // (4, 1.0) + (4, -1.0) は打ち消し合い、(0, 1e-14) は `tol` 未満
        assert_eq!(v.entries(), &[(1, 2.0), (3, 5.0)]);
    }

    /// `from_dense_tol` が小さな雑音を捨てて疎化すること。
    #[test]
    fn from_dense_tol_sparsifies_a_noisy_dense_vector() {
        let dense = [1.0, 1e-15, -2.0, 0.0, 1e-13];
        assert_eq!(SparseVec::from_dense(&dense).nnz(), 4);
        assert_eq!(SparseVec::from_dense_tol(&dense, 1e-12).entries(), &[(0, 1.0), (2, -2.0)]);
    }

    /// 疎アキュムレータの行消去が `BTreeMap` によるマージと要素ごとに一致すること。
    #[test]
    fn accumulator_matches_a_btreemap_merge_entry_for_entry() {
        use std::collections::BTreeMap;
        let row = [(3usize, 1.0f64), (0usize, 2.0f64), (5usize, -4.0f64)];
        let pivot = [(5usize, 2.0f64), (1usize, 0.5f64), (0usize, 1.0f64)];
        let factor = 2.0;

        let mut map: BTreeMap<usize, f64> = row.iter().copied().collect();
        for &(k, v) in &pivot {
            *map.entry(k).or_insert(0.0) -= factor * v;
        }
        map.remove(&5);
        map.retain(|_, v| v.abs() > 1e-9);
        let expected: Vec<(usize, f64)> = map.into_iter().collect();

        let mut accum = SparseAccum::new(8);
        assert_eq!(axpy_row(&mut accum, &row, &pivot, factor, 5, 1e-9), expected);
    }

    /// リセット後に前回のマージの内容が残らないこと。
    #[test]
    fn accumulator_reset_isolates_successive_merges() {
        let mut accum = SparseAccum::new(4);
        accum.load(&[(0, 1.0), (2, 2.0)]);
        assert!(accum.contains(2));
        assert!(approx(accum.get(2), 2.0));
        assert_eq!(accum.take_sorted(0.0), vec![(0, 1.0), (2, 2.0)]);

        accum.load(&[(1, 5.0)]);
        assert!(!accum.contains(0), "previous merge's pattern must not leak in");
        assert!(!accum.contains(2));
        assert!(approx(accum.get(2), 0.0));
        assert_eq!(accum.take_sorted(0.0), vec![(1, 5.0)]);
    }

    /// `set` は後勝ち、`add` は累積であること。
    #[test]
    fn accumulator_set_is_last_write_wins_and_add_accumulates() {
        let mut accum = SparseAccum::new(4);
        accum.reset();
        accum.set(1, 3.0);
        accum.set(1, 7.0);
        accum.add(1, 1.0);
        accum.add(2, 4.0);
        assert_eq!(accum.take_sorted(0.0), vec![(1, 8.0), (2, 4.0)]);
    }

    /// `remove` が非零値の添字でも確実に取り除くこと。
    #[test]
    fn accumulator_remove_drops_an_index_even_when_its_value_is_nonzero() {
        let mut accum = SparseAccum::new(4);
        accum.load(&[(0, 1.0), (1, 2.0), (2, 3.0)]);
        accum.remove(1);
        assert!(!accum.contains(1));
        assert_eq!(accum.take_sorted(0.0), vec![(0, 1.0), (2, 3.0)]);
        // 外した添字は次のマージで再利用できる
        accum.load(&[(1, 9.0)]);
        assert_eq!(accum.take_sorted(0.0), vec![(1, 9.0)]);
    }

    /// `from_rows_canonical` が未ソート・重複入力を正規化すること。
    #[test]
    fn from_rows_canonical_fixes_unsorted_duplicated_input() {
        let csr = CsrMat::from_rows_canonical(&[vec![(2, 1.0), (0, 2.0), (2, 3.0)]], 3, 0.0);
        assert_eq!(csr.row(0), &[(0, 2.0), (2, 4.0)]);
    }

    /// 三つ組からの構築が行からの構築と一致すること。
    #[test]
    fn from_triplets_builds_the_same_matrix_as_from_rows() {
        let triplets = [(0usize, 0usize, 1.0f64), (0, 2, 2.0), (1, 1, 3.0), (2, 0, 4.0), (2, 3, 5.0)];
        assert_eq!(CsrMat::from_triplets(3, 4, &triplets), CsrMat::from_rows(&sample_rows(), 4));
    }

    /// 要素ストリームからの構築が、2 ブロックを連結して作った行列と一致すること。
    #[test]
    fn from_entry_stream_matches_a_concatenated_two_block_build() {
        let block_a = sample_rows();
        let block_g = vec![vec![(1usize, 7.0f64), (3usize, 0.0f64)], vec![(0usize, 8.0f64)]];
        // ストリーム側で厳密な 0 を除くので、比較用の構築でも除く
        let mut concat: Vec<Vec<(usize, f64)>> = block_a.clone();
        concat.extend(block_g.iter().map(|r| r.iter().copied().filter(|&(_, v)| v != 0.0).collect()));
        let expected = CscMat::from_rows(&concat, 4);

        let got = CscMat::from_entry_stream(concat.len(), 4, |emit| {
            for (i, row) in block_a.iter().enumerate() {
                for &(j, v) in row {
                    emit(i, j, v);
                }
            }
            for (gi, row) in block_g.iter().enumerate() {
                for &(j, v) in row {
                    if v != 0.0 {
                        emit(block_a.len() + gi, j, v);
                    }
                }
            }
        });
        assert_eq!(got, expected);
        assert_eq!(got.col(1), &[(1, 3.0), (3, 7.0)]);
    }

    /// 列の密化が正しい密な列になること。
    #[test]
    fn csc_col_dense_matches_the_dense_column() {
        let csc = CsrMat::from_rows(&sample_rows(), 4).to_csc();
        assert_eq!(csc.col_dense(0), vec![1.0, 0.0, 4.0]);
        let mut buf = vec![9.9; 3];
        csc.col_into_dense(3, &mut buf);
        assert_eq!(buf, vec![0.0, 0.0, 5.0]);
    }

    /// 疎ベクトルの追加・刈り込み・ソート・スケールなどの変更操作。
    #[test]
    fn sparse_vec_mutators_cover_the_incremental_build_path() {
        let mut v = SparseVec::with_capacity(6, 3);
        assert!(v.is_empty());
        assert_eq!(v.len(), 6);
        v.push(4, -2.0);
        v.push(1, 3.0);
        v.push(5, 1e-14);
        assert_eq!(v.nnz(), 3);

        v.prune(1e-12);
        v.sort();
        assert_eq!(v.entries(), &[(1, 3.0), (4, -2.0)]);
        assert!(approx(v.norm2(), 13.0f64.sqrt()));
        assert_eq!(v.iter().collect::<Vec<_>>(), vec![(1, 3.0), (4, -2.0)]);

        v.scale(2.0);
        let mut acc = vec![1.0; 6];
        v.scatter_add_into(0.5, &mut acc);
        assert_eq!(acc, vec![1.0, 1.0 + 3.0, 1.0, 1.0, 1.0 - 2.0, 1.0]);

        v.entries_mut()[0].1 = 0.0;
        v.prune(0.0);
        assert_eq!(v.into_entries(), vec![(4, -4.0)]);

        let mut z = SparseVec::zeros(3);
        z.push(0, 1.0);
        z.clear();
        assert!(z.is_empty());
    }

    /// 行・列の複製が正しい論理長を持つこと。
    #[test]
    fn owned_row_and_column_views_carry_the_right_logical_length() {
        let csr = CsrMat::from_rows(&sample_rows(), 4);
        let csc = csr.to_csc();
        let row = csr.row_vec(2);
        assert_eq!(row.len(), 4);
        assert_eq!(row.entries(), &[(0, 4.0), (3, 5.0)]);
        let col = csc.col_vec(0);
        assert_eq!(col.len(), 3);
        assert_eq!(col.entries(), &[(0, 1.0), (2, 4.0)]);
        assert_eq!(csc.to_cols(), vec![vec![(0, 1.0), (2, 4.0)], vec![(1, 3.0)], vec![(0, 2.0)], vec![(2, 5.0)]]);
        assert_eq!(csr.nnz(), 5);
        assert_eq!(csc.nnz(), 5);
    }

    /// `take_sorted_vec` の結果の長さが添字空間の大きさになること。
    #[test]
    fn accumulator_take_sorted_vec_reports_the_index_space_as_its_length() {
        let mut accum = SparseAccum::new(7);
        assert_eq!(accum.capacity(), 7);
        accum.load(&[(6, 1.0), (2, 2.0)]);
        let v = accum.take_sorted_vec(0.0);
        assert_eq!(v.len(), 7);
        assert_eq!(v.entries(), &[(2, 2.0), (6, 1.0)]);
    }

    /// `scatter_dense` と `sparse_axpy_dense` が期待どおりに動くこと。
    #[test]
    fn scatter_dense_and_axpy_helpers_match_their_sparse_vec_forms() {
        let entries = [(1usize, 2.0f64), (3usize, -4.0f64)];
        let mut buf = vec![9.9; 5];
        scatter_dense(&entries, &mut buf);
        assert_eq!(buf, vec![0.0, 2.0, 0.0, -4.0, 0.0]);
        sparse_axpy_dense(0.5, &entries, &mut buf);
        assert_eq!(buf, vec![0.0, 3.0, 0.0, -6.0, 0.0]);
    }

    /// `HybridVec` が充填率から表現を選ぶこと。
    #[test]
    fn hybrid_vec_picks_its_representation_from_fill() {
        let sparse = HybridVec::pack(10, vec![(1, 2.0), (7, -1.0)], 0.4);
        assert!(matches!(sparse, HybridVec::Sparse(_)));
        let dense = HybridVec::pack(4, vec![(0, 1.0), (1, 2.0), (3, 4.0)], 0.4);
        assert!(matches!(dense, HybridVec::Dense { .. }));
        // 除外される添字 (ここでは 2) は密形で `0.0` として格納される
        match &dense {
            HybridVec::Dense { data, nnz } => {
                assert_eq!(&data[..], &[1.0, 2.0, 0.0, 4.0]);
                assert_eq!(*nnz, 3);
            }
            _ => unreachable!(),
        }
    }

    /// `HybridVec` の疎形と密形で内積・axpy の結果が一致すること。
    #[test]
    fn hybrid_vec_both_representations_agree_on_dot_and_axpy() {
        let pairs = vec![(0usize, 1.0f64), (1, 2.0), (3, 4.0)];
        // 同じ中身を、閾値でそれぞれの表現に強制する
        let as_sparse = HybridVec::pack(4, pairs.clone(), 1.0);
        let as_dense = HybridVec::pack(4, pairs, 0.0);
        assert!(matches!(as_sparse, HybridVec::Sparse(_)));
        assert!(matches!(as_dense, HybridVec::Dense { .. }));

        let x = [10.0, 100.0, 1000.0, 10000.0];
        assert!(approx(as_sparse.dot_dense(&x), 1.0 * 10.0 + 2.0 * 100.0 + 4.0 * 10000.0));
        assert!(approx(as_dense.dot_dense(&x), as_sparse.dot_dense(&x)));

        let mut ds = vec![1.0; 4];
        let mut dd = vec![1.0; 4];
        as_sparse.axpy_into_dense(-2.0, &mut ds);
        as_dense.axpy_into_dense(-2.0, &mut dd);
        assert_eq!(ds, vec![-1.0, -3.0, 1.0, -7.0]);
        assert_eq!(ds, dd, "the skipped index must be untouched in both arms");

        assert_eq!(as_sparse.nnz(), 3);
        assert_eq!(as_dense.nnz(), 3);
        assert_eq!(as_sparse.to_pairs(), as_dense.to_pairs());
    }

    /// `pack_scaled_dense` が「組の列を作ってから `pack`」と両表現・両フィルタ (除外位置と積が厳密に 0) を含めて一致すること。
    #[test]
    fn pack_scaled_dense_matches_collect_then_pack() {
        // `src[2]` は除外位置。`src[5]` はそれ自体は非零だが scale 倍するとアンダーフローで
        // 0 になる値で、これも捨てられなければならない (でないと `nnz` がずれる)。
        let src = [1.5, 0.0, 7.0, -2.0, 0.0, 1e-320, 4.0, 0.0];
        for &scale in &[1.0f64, -3.0, 1e-8] {
            for &frac in &[1.0f64, 0.0, 0.4] {
                let pairs: Vec<(usize, f64)> = (0..src.len())
                    .filter(|&i| i != 2)
                    .map(|i| (i, scale * src[i]))
                    .filter(|&(_, v)| v != 0.0)
                    .collect();
                let expected = HybridVec::pack(src.len(), pairs, frac);
                let got = HybridVec::pack_scaled_dense(&src, 2, scale, frac);
                assert_eq!(
                    std::mem::discriminant(&expected),
                    std::mem::discriminant(&got),
                    "scale={scale} frac={frac}: same representation chosen"
                );
                assert_eq!(got.nnz(), expected.nnz(), "scale={scale} frac={frac}");
                assert_eq!(got.to_pairs(), expected.to_pairs(), "scale={scale} frac={frac}");
                // 密形は除外位置に 0 を置いたままであること
                if let HybridVec::Dense { data, .. } = &got {
                    assert_eq!(data[2], 0.0, "scale={scale} frac={frac}");
                }
            }
        }
    }

    /// `remove_index` が両表現で `nnz` を正しく保つこと。
    #[test]
    fn hybrid_vec_remove_index_keeps_nnz_honest_in_both_arms() {
        for frac in [1.0f64, 0.0] {
            let mut v = HybridVec::pack(4, vec![(0, 1.0), (1, 2.0), (3, 4.0)], frac);
            assert!(v.remove_index(1), "frac={frac}: removing a present index reports true");
            assert_eq!(v.nnz(), 2, "frac={frac}");
            assert_eq!(v.to_pairs(), vec![(0, 1.0), (3, 4.0)], "frac={frac}");
            let mut seen = Vec::new();
            v.for_each_index(|i| seen.push(i));
            assert_eq!(seen, vec![0, 3], "frac={frac}");
            // 何もない添字を外しても nnz は減らないこと
            assert!(!v.remove_index(2), "frac={frac}: removing an absent index reports false");
            assert_eq!(v.nnz(), 2, "frac={frac}");
        }
    }

    /// `begin` が前回までの印をすべて消すこと。
    #[test]
    fn epoch_marks_begin_clears_every_previous_mark() {
        let mut m = EpochMarks::new(4);
        assert_eq!(m.len(), 4);
        assert!(!m.is_marked(0), "nothing may read as marked before the first begin");

        m.begin();
        m.mark(1);
        m.mark(3);
        assert!(m.is_marked(1) && m.is_marked(3));
        assert!(!m.is_marked(0) && !m.is_marked(2));

        m.begin();
        assert!(!m.is_marked(1) && !m.is_marked(3), "a new pass starts clear");
    }

    /// `unmark` がこのパスの印だけを取り消すこと。
    #[test]
    fn epoch_marks_unmark_takes_one_mark_back_for_this_pass_only() {
        let mut m = EpochMarks::new(3);
        m.begin();
        m.mark(0);
        m.mark(2);
        m.unmark(0);
        assert!(!m.is_marked(0));
        assert!(m.is_marked(2));
        m.mark(0);
        assert!(m.is_marked(0), "an unmarked index is markable again");
        m.begin();
        assert!(!m.is_marked(0) && !m.is_marked(2));
    }

    /// エポックカウンタが一周しても古い印が有効に見えないこと。
    #[test]
    fn epoch_marks_survive_counter_wraparound() {
        let mut m = EpochMarks::new(2);
        // 次の `begin` で 0 に一周する値までカウンタを進める
        m.force_epoch_for_test(u32::MAX);
        m.mark(0);
        assert!(m.is_marked(0));
        m.begin(); // 一周: 0 を有効のままにせずスタンプをリセットすること
        assert!(!m.is_marked(0), "a stamp from the pre-wrap pass must not read as live");
        m.mark(1);
        assert!(m.is_marked(1) && !m.is_marked(0));
    }

    /// `CscBuilder` の結果が 2 パス構築と一致すること。
    #[test]
    fn csc_builder_matches_a_two_pass_build_of_the_same_matrix() {
        let cols: Vec<Vec<(usize, f64)>> = vec![vec![(0, 1.0), (2, 4.0)], vec![(1, 3.0)], vec![(0, 2.0)], vec![(2, 5.0)]];
        let mut b = CscBuilder::with_capacity(3, 4, 5);
        for col in &cols {
            for &(i, v) in col {
                b.push(i, v);
            }
            b.end_column();
        }
        assert_eq!(b.columns_built(), 4);
        let built = b.build();
        assert_eq!(built, CsrMat::from_rows(&sample_rows(), 4).to_csc());
        assert_eq!(built.n_rows(), 3);
        assert_eq!(built.n_cols(), 4);
    }

    /// `CscMat::empty` が全列を持ち、すべて空であること。
    #[test]
    fn csc_empty_has_every_column_present_and_empty() {
        let m = CscMat::empty(3, 4);
        assert_eq!((m.n_rows(), m.n_cols(), m.nnz()), (3, 4, 0));
        assert!((0..4).all(|j| m.col(j).is_empty()));
        assert_eq!(m.mat_t_vec(&[1.0, 2.0, 3.0]), vec![0.0; 4]);
    }

    /// `CscBuilder` が空の列を保つこと。
    #[test]
    fn csc_builder_keeps_empty_columns() {
        let mut b = CscBuilder::new(2);
        b.end_column(); // 列 0: 空
        b.push(1, 7.0);
        b.end_column(); // 列 1
        b.end_column(); // 列 2: 空
        let m = b.build();
        assert_eq!(m.n_cols(), 3);
        assert!(m.col(0).is_empty());
        assert_eq!(m.col(1), &[(1, 7.0)]);
        assert!(m.col(2).is_empty());
        assert_eq!(m.nnz(), 1);
    }

    /// 行数 0 の行列でも寸法と積が正しく定義されること。
    #[test]
    fn empty_matrix_dimensions_and_products_stay_well_defined() {
        let csr = CsrMat::from_rows(&[], 3);
        assert_eq!(csr.n_rows(), 0);
        assert_eq!(csr.n_cols(), 3);
        assert_eq!(csr.nnz(), 0);
        assert_eq!(csr.mat_t_vec(&[]), vec![0.0, 0.0, 0.0]);
        assert_eq!(csr.to_csc().n_cols(), 3);
        assert_eq!(csr.col_counts(), vec![0, 0, 0]);
    }
}
