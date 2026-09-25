//! 冗長な制約行の除去(プリソルブ)。
//!
//! 等式制約 `(A, b)` については、Ruiz スケーリング後の問題に対して
//! `presolve::run_extended` から主ループ開始前に一度だけ呼ばれる
//! (内点法・単体法の両エンジン共通)。不等式制約 `(G, h)` の重複行除去
//! [`reduce_inequalities`] は安価なハッシュ走査なので、`run_extended` の
//! 外側ラウンドの最後にも毎回呼ばれる(代入系の手法が後から重複行を
//! 生むことがあるため)。
//!
//! 等式側は 2 段階:
//!  1. **重複行の直接検出** ([`dedupe_rows`]): 右辺も含めて既存の行の
//!     完全一致またはスカラー倍である行を落とす。ハッシュ比較のみで
//!     線形代数は使わない。逐次走査なのは、どちらの行が残るかを実行ごとに
//!     決定的にするため。
//!  2. **ランク判定による一次従属行の除去**: 行密度に応じて
//!     [`reduce_equalities`] が次のどちらかを選ぶ。
//!     - 密な列ピボット付き QR ([`drop_linearly_dependent`])
//!     - 疎なガウス消去 ([`drop_linearly_dependent_sparse_blocked`] →
//!       ブロックごとに [`drop_linearly_dependent_sparse`])。事前に
//!       Dulmage-Mendelsohn 型ブロック分解 ([`dulmage_mendelsohn_blocks`]:
//!       最大二部マッチング + Tarjan の強連結成分分解)で小問題に分け、
//!       総行数が閾値以上なら `rayon` で並列に処理する。冗長性判定は
//!       各行について局所的な恒等式の確認にすぎないので、ブロックは
//!       任意の順序・並列で独立に調べてよい(見落としはあり得るが、
//!       独立な行を誤って落とすことはない)。
//!
//!     どちらも「既に残した行で消去した後の残差が、その行自身の元の
//!     ノルムに比べて無視できる行は他の行の一次結合である」という同じ
//!     判定を行う。右辺を追加の列として扱うので、係数だけ従属で右辺が
//!     矛盾する行(実行不能)は落とさずに残す。

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use crate::params::presolve::{DENSE_DENSITY_THRESHOLD, DEP_TOL, MIN_ROWS_FOR_BLOCK_DECOMPOSE, PARALLEL_DECOMPOSE_ROW_THRESHOLD, PIVOT_STABILITY, REDEQ_QR_RANK_TOL};
#[cfg(test)]
use std::collections::HashSet;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};

use faer::dyn_stack::{GlobalPodBuffer, PodStack};
use faer::linalg::qr::col_pivoting::compute as colpiv_qr;
use faer::{Mat, Parallelism};

use crate::presolve::smallcoeff;
use crate::sparse::{FaerCsr, CsrRowBuilder, csr_from_rows, csr_is_canonical, csr_row_iter, csr_row_vec};
/// 計測用カウンタ: [`drop_linearly_dependent_sparse`] が実行したピボット
/// ステップの総数(`ENOMOTO_PROF_REDUNDANCY` 診断で `presolve.rs` が読む)。
pub(crate) static PROF_TOTAL_STEPS: AtomicUsize = AtomicUsize::new(0);
/// 計測用カウンタ: そのうち Markowitz スコア
/// `(row_degree - 1) * (col_degree - 1)` が 0 だった「自明な」ステップ数
/// (BTF 前処理でも見つかる構造的に強制されたピボット。
/// `simplex::lu::MarkowitzState::find_best_pivot` と同じ定義)。
pub(crate) static PROF_TRIVIAL_STEPS: AtomicUsize = AtomicUsize::new(0);

/// 等式制約 `(A, b)` から重複行と一次従属行を取り除いた `(A', b')` を返す。
///
/// 重複除去 ([`dedupe_rows`]) の後、行密度が
/// [`DENSE_DENSITY_THRESHOLD`] を超えれば密 QR ([`drop_linearly_dependent`])、
/// そうでなければ疎ガウス消去 ([`drop_linearly_dependent_sparse_blocked`])
/// でランク判定する。
///
/// - `n`: 変数(列)数。
/// - `lb`/`ub`: 変数の下限・上限。疎経路のブロック分解
///   ([`dulmage_mendelsohn_blocks`]) で、無視できる小係数を二部グラフの
///   辺から外す判定にだけ使う。`a`/`b` 自体は変更しない。緩い(未伝播の)
///   境界を渡しても分割が粗くなるだけで正しさは損なわれない。
pub fn reduce_equalities(a: &FaerCsr, b: &[f64], n: usize, lb: &[f64], ub: &[f64]) -> (FaerCsr, Vec<f64>) {
    // 等式行数
    let p = a.nrows();
    if p == 0 {
        return (csr_from_rows(&[], n), Vec::new());
    }

    // (行の疎係数, 右辺) の組に展開
    let rows: Vec<(Vec<(usize, f64)>, f64)> = (0..p)
        .map(|i| {
            let row: Vec<(usize, f64)> = csr_row_vec(a, i);
            (row, b[i])
        })
        .collect();

    let deduped = dedupe_rows(rows);
    // 重複除去後の行密度(非零数 / (行数 × 列数))で密・疎経路を選ぶ
    let nnz: usize = deduped.iter().map(|(row, _)| row.len()).sum();
    let density = nnz as f64 / (deduped.len() as f64 * n.max(1) as f64);
    // 残す行の(deduped 内)インデックス
    let keep = if density > tunable!("ENOMOTO_T_DENSE_DENSITY_THRESHOLD", DENSE_DENSITY_THRESHOLD, f64) {
        drop_linearly_dependent(&deduped, n)
    } else {
        drop_linearly_dependent_sparse_blocked(&deduped, n, lb, ub)
    };

    let mut new_rows = Vec::with_capacity(keep.len());
    let mut new_b = Vec::with_capacity(keep.len());
    for &idx in &keep {
        new_rows.push(deduped[idx].0.clone());
        new_b.push(deduped[idx].1);
    }
    (csr_from_rows(&new_rows, n), new_b)
}

/// [`reduce_equalities`] の重複行除去 ([`dedupe_rows`]) だけを行い、
/// ランク判定はしない版。`run_extended` の既定動作
/// (`ENOMOTO_REDEQ_MODE` で切替)。
pub fn dedupe_equalities(a: &FaerCsr, b: &[f64], n: usize) -> (FaerCsr, Vec<f64>) {
    let p = a.nrows();
    if p == 0 {
        return (csr_from_rows(&[], n), Vec::new());
    }
    let rows: Vec<(Vec<(usize, f64)>, f64)> = (0..p).map(|i| (csr_row_vec(a, i), b[i])).collect();
    let deduped = dedupe_rows(rows);
    let (rows, rhs): (Vec<Vec<(usize, f64)>>, Vec<f64>) = deduped.into_iter().unzip();
    (csr_from_rows(&rows, n), rhs)
}

/// 段階 1: 完全一致またはスカラー倍の重複行を落とし、残った行を入力順で返す。
///
/// 各行(と右辺)を先頭係数で割って正規化し、そのビット列をハッシュして
/// 比較する(符号も含めて割るので、負のスカラー倍も同一視される。等式
/// なので問題ない)。構造的に空の行は、右辺がビット単位で 0 (`0 = 0`) なら
/// 落とし、非零なら下流の Farkas 実行不能判定のために残す。
///
/// 判定結果はテスト用の単純版 `dedupe_rows_reference` と完全に一致する。
fn dedupe_rows(rows: Vec<(Vec<(usize, f64)>, f64)>) -> Vec<(Vec<(usize, f64)>, f64)> {
    // 正規化済みシグネチャをその場でハッシュし、ハッシュ一致時は残した行の
    // シグネチャを再計算して要素ごとに照合する。
    /// ハッシュ値 `hash` に 64 ビット値 `x` を混ぜ込む乗算型ミキサ。
    #[inline]
    fn mix(hash: u64, x: u64) -> u64 {
        (hash.rotate_left(5) ^ x).wrapping_mul(0x517c_c1b7_2722_0a95)
    }
    // ハッシュ値 -> そのハッシュを持つ連鎖の先頭 (`chain` の添字)
    let mut heads: HashMap<u64, usize, std::hash::BuildHasherDefault<IdentityU64Hasher>> = HashMap::with_capacity_and_hasher(rows.len(), Default::default());
    // 残した(空でない)行ごとに: (先頭係数の逆数, 同じハッシュ連鎖の次の要素。末尾は usize::MAX)
    let mut chain: Vec<(f64, usize)> = Vec::with_capacity(rows.len());
    // 残した行(出力)
    let mut kept: Vec<(Vec<(usize, f64)>, f64)> = Vec::with_capacity(rows.len());
    // 空行は連鎖に入らないので `chain` と `kept` の添字はずれる。
    // `slot_of_chain[k]` は連鎖要素 k に対応する `kept` の添字。
    let mut slot_of_chain: Vec<usize> = Vec::with_capacity(rows.len());
    for (row, rhs) in rows {
        if row.is_empty() {
            if rhs != 0.0 {
                kept.push((row, rhs));
            }
            continue;
        }
        // 正規化に使う先頭係数とその逆数
        let pivot = row[0].1;
        let inv = 1.0 / pivot;
        let mut hash = row.len() as u64;
        for &(j, v) in &row {
            hash = mix(mix(hash, j as u64), (v * inv).to_bits());
        }
        // 正規化後の右辺のビット列
        let rhs_bits = (rhs * inv).to_bits();
        hash = mix(hash, rhs_bits);
        // 同じハッシュの連鎖を先頭からたどり、正規化後に完全一致する行を探す
        let head = heads.get(&hash).copied().unwrap_or(usize::MAX);
        let mut cur = head;
        let mut dup = false;
        while cur != usize::MAX {
            let (k_inv, next) = chain[cur];
            let (k_row, k_rhs) = &kept[slot_of_chain[cur]];
            if k_row.len() == row.len()
                && (k_rhs * k_inv).to_bits() == rhs_bits
                && k_row.iter().zip(&row).all(|(&(kj, kv), &(j, v))| kj == j && (kv * k_inv).to_bits() == (v * inv).to_bits())
            {
                dup = true;
                break;
            }
            cur = next;
        }
        if !dup {
            heads.insert(hash, chain.len());
            chain.push((inv, head));
            slot_of_chain.push(kept.len());
            kept.push((row, rhs));
        }
    }
    kept
}

/// テスト用の単純な参照実装: 正規化シグネチャ
/// `[(j, bits(v/pivot))..., (usize::MAX, bits(rhs/pivot))]` を `HashSet` に
/// 入れて重複判定する。[`dedupe_rows`] と同じ結果になるべきもの。
#[cfg(test)]
fn dedupe_rows_reference(rows: Vec<(Vec<(usize, f64)>, f64)>) -> Vec<(Vec<(usize, f64)>, f64)> {
    // 既出の正規化シグネチャ
    let mut seen: HashSet<Vec<(usize, u64)>> = HashSet::new();
    let mut kept = Vec::with_capacity(rows.len());
    for (row, rhs) in rows {
        if row.is_empty() {
            if rhs != 0.0 {
                kept.push((row, rhs));
            }
            continue;
        }
        let pivot = row[0].1;
        let inv = 1.0 / pivot;
        let mut sig: Vec<(usize, u64)> = row.iter().map(|&(j, v)| (j, (v * inv).to_bits())).collect();
        sig.push((usize::MAX, (rhs * inv).to_bits()));
        if seen.insert(sig) {
            kept.push((row, rhs));
        }
    }
    kept
}

/// 密な列ピボット付き QR によるランク判定。`rows` のうち極大な一次独立
/// 部分集合の添字(昇順)を返す。
///
/// 各行を列とした `(n+1) x p` の密行列(最後の行 `n` は右辺)を QR 分解し、
/// `|R[k,k]|`(それ以前のピボットを射影で除いた残差ノルム)がその行自身の
/// 元のノルムの [`REDEQ_QR_RANK_TOL`] 倍以下なら従属とみなす。コストは
/// 密度に関係なく `O((n+1) * p^2)`。行密度が [`DENSE_DENSITY_THRESHOLD`]
/// を超えるときに [`reduce_equalities`] がこちらを選ぶ。
fn drop_linearly_dependent(rows: &[(Vec<(usize, f64)>, f64)], n: usize) -> Vec<usize> {
    let p = rows.len();
    if p == 0 {
        return Vec::new();
    }

    // 列は [行の係数 ; 右辺] の n+1 成分。係数は従属でも右辺が矛盾する行は
    // (冗長ではなく実行不能なので)拡大した意味で独立となり、QR で残る。
    // その報告は下流の Farkas 判定が行う。
    let aug_n = n + 1;
    // 拡大係数行列(列 i = 行 i)。QR 後は上三角部分に R が入る
    let mut m = Mat::<f64>::zeros(aug_n, p);
    for (i, (row, rhs)) in rows.iter().enumerate() {
        for &(j, v) in row {
            m[(j, i)] = v;
        }
        m[(n, i)] = *rhs;
    }

    // 列ピボット付き QR をその場で、明示的に逐次 (`Parallelism::None`) で
    // 実行する(並列版はこの規模では得がなく、縮約順序で結果が実行ごとに
    // 揺れるため)。以下で使うのは R の対角と列置換だけ。
    // R の対角長 = 判定できる最大ランク
    let size = aug_n.min(p);
    let blocksize = colpiv_qr::recommended_blocksize::<f64>(aug_n, p);
    // Householder 係数の作業領域
    let mut householder = Mat::<f64>::zeros(blocksize, size);
    // col_perm[k] = ピボット位置 k に来た元の列(=元の行)番号
    let mut col_perm = vec![0usize; p];
    // その逆置換(未使用だが faer の API が要求する)
    let mut col_perm_inv = vec![0usize; p];
    let params = Default::default();
    colpiv_qr::qr_in_place(
        m.as_mut(),
        householder.as_mut(),
        &mut col_perm,
        &mut col_perm_inv,
        Parallelism::None,
        PodStack::new(&mut GlobalPodBuffer::new(
            colpiv_qr::qr_in_place_req::<usize, f64>(aug_n, p, blocksize, Parallelism::None, params).unwrap(),
        )),
        params,
    );
    let r = &m; // 上三角部分が R
    // ピボット位置 -> 元の行番号
    let pivot_to_row = &col_perm;

    // 対角成分を持つピボット位置の数
    let rank_dim = size;

    // 従属判定は行ごとに、その行自身のノルムに対する相対値で行う:
    // `|R[k,k]|` はピボット列 k から先行ピボットを射影で除いた残差ノルム。
    // 1e-300 は完全な零行でのゼロ除算相当を避ける下限。
    // 行 i の拡大ノルム ||[係数; 右辺]||_2
    let col_norm = |i: usize| -> f64 {
        let (row, rhs) = &rows[i];
        (row.iter().map(|&(_, v)| v * v).sum::<f64>() + rhs * rhs).sqrt()
    };
    // keep[i] = 行 i を残すか
    let mut keep = vec![true; p];
    for k in 0..rank_dim {
        let orig = pivot_to_row[k];
        if r[(k, k)].abs() <= tunable!("ENOMOTO_T_REDEQ_QR_RANK_TOL", REDEQ_QR_RANK_TOL, f64) * col_norm(orig).max(1e-300) {
            keep[orig] = false;
        }
    }
    // p > aug_n のとき独立な行は高々 aug_n 本: rank_dim 以降のピボット位置は
    // 対角成分を持たないので、その行は必ず従属。
    for &orig in &pivot_to_row[rank_dim..] {
        keep[orig] = false;
    }

    (0..p).filter(|&i| keep[i]).collect()
}

/// 疎ガウス消去によるランク判定。`rows_in` のうち極大な一次独立部分集合の
/// 添字(昇順)を返す。
///
/// 各行を `n+1` 列上の疎行として扱う(列 `n` は右辺を表す仮想列)。
/// 右辺を拡大列に含めるので、係数だけ従属で右辺が矛盾する行は独立と
/// みなされて残る(実行不能の報告は下流の Farkas 判定が行う)。コストは
/// 実際の fill-in に比例するので疎な問題向き。密な問題では
/// [`drop_linearly_dependent`] が使われる。
///
/// **ピボット選択**: Markowitz 次数の昇順バケット走査
/// (`simplex::lu::MarkowitzState::find_best_pivot` と同様、fill-in 抑制)
/// で候補を探すが、候補として受理するのは絶対値が行列全体の現在の最大
/// 活性要素の [`PIVOT_STABILITY`] 倍以上のものだけ(列内の相対値では
/// なく**全体**に対する閾値。ランク判定を正しく行うため)。
///
/// **従属判定**: 選ばれたピボット値が、その行の元の(拡大)ノルムの
/// [`DEP_TOL`] 倍以下なら、先行ピボットで消去した残りが無視できる、
/// すなわち従属とみなして行を落とし、消去せずに次へ進む。
/// L/U 因子は保持しない(どの行がピボットを得たかだけが必要)。
fn drop_linearly_dependent_sparse(rows_in: &[(Vec<(usize, f64)>, f64)], n: usize) -> Vec<usize> {
    let p = rows_in.len();
    if p == 0 {
        return Vec::new();
    }
    // 右辺列を含む拡大列数
    let aug_n = n + 1;

    // 作業用の行表現 (列 -> 値)。消去によって更新される
    let mut rows: Vec<BTreeMap<usize, f64>> = Vec::with_capacity(p);
    // 各行の元の拡大ノルム(従属判定の基準)
    let mut row_orig_norm = vec![0.0f64; p];
    for (i, (row, rhs)) in rows_in.iter().enumerate() {
        let mut m: BTreeMap<usize, f64> = BTreeMap::new();
        for &(j, v) in row {
            if v != 0.0 {
                *m.entry(j).or_insert(0.0) += v;
            }
        }
        m.retain(|_, v| *v != 0.0);
        if *rhs != 0.0 {
            m.insert(n, *rhs);
        }
        row_orig_norm[i] = m.values().map(|v| v * v).sum::<f64>().sqrt();
        rows.push(m);
    }

    // col_rows[j] = 列 j に(活性な)非零を持つ行の集合
    let mut col_rows: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); aug_n];
    // 各行の現在の非零数
    let mut row_degree = vec![0usize; p];
    for (i, row) in rows.iter().enumerate() {
        row_degree[i] = row.len();
        for &j in row.keys() {
            col_rows[j].insert(i);
        }
    }
    // 各列の現在の非零数 (= col_rows[j].len())
    let mut col_degree: Vec<usize> = col_rows.iter().map(|s| s.len()).collect();

    // 行列全体の最大活性要素を持つ列を、毎ステップ全走査せずに追跡する。
    // `heap`: (列の最大 |要素| のビット列, 列) の順序付き集合。非負の f64 は
    // `to_bits` で大小順が保たれるので、末尾が全体最大 (O(log aug_n))。
    let mut heap: BTreeSet<(u64, usize)> = BTreeSet::new();
    // col_bits[j] = 列 j が現在 `heap` に登録しているキー(None = 未登録 = 全零)
    let mut col_bits: Vec<Option<u64>> = vec![None; aug_n];
    /// 列 `j` の最大 |要素| を走査し直して `heap`/`col_bits` のキーを正確な値に更新する。
    fn refresh_col(col_rows: &[BTreeSet<usize>], rows: &[BTreeMap<usize, f64>], heap: &mut BTreeSet<(u64, usize)>, col_bits: &mut [Option<u64>], j: usize) {
        if let Some(old) = col_bits[j].take() {
            heap.remove(&(old, j));
        }
        let new_max = col_rows[j].iter().filter_map(|&i| rows[i].get(&j).map(|v| v.abs())).fold(0.0f64, f64::max);
        if new_max > 0.0 {
            let bits = new_max.to_bits();
            heap.insert((bits, j));
            col_bits[j] = Some(bits);
        }
    }
    for j in 0..aug_n {
        refresh_col(&col_rows, &rows, &mut heap, &mut col_bits, j);
    }
    // `heap` の遅延更新: 各列のキーは真の最大 |要素| の「上界」であることだけが
    // 保証され、`col_exact[j]` はそれが厳密に等しいと分かっているかを示す。
    // 最大値を下げうる変更は `col_exact[j]` を false にするだけ (O(1)) で、
    // 再走査はその列が `heap` の先頭に来たときだけ行う(各ステップ冒頭の
    // 検証ループ)。全キーが真の最大以上なので、最初に現れる厳密な先頭が
    // 真の全体最大となり、毎回再走査する版と結果は完全に一致する。
    let mut col_exact = vec![true; aug_n];
    /// 列 `j` の要素が変化したことを `heap` に反映する(遅延更新)。
    /// `old_abs`: 変化前の |値|(存在しなかった場合 0.0)、
    /// `new_abs`: 変化後の |値|(削除された場合 0.0)。
    fn note_change(heap: &mut BTreeSet<(u64, usize)>, col_bits: &mut [Option<u64>], col_exact: &mut [bool], j: usize, old_abs: f64, new_abs: f64) {
        // 現在のキー(未登録なら 0)
        let key = col_bits[j].map_or(0.0, f64::from_bits);
        if new_abs > key {
            match col_bits[j].take() {
                Some(old) => {
                    heap.remove(&(old, j));
                }
                // 未登録 = 列の真の最大が 0 だったので、新要素がちょうど最大
                None => col_exact[j] = true,
            }
            let bits = new_abs.to_bits();
            heap.insert((bits, j));
            col_bits[j] = Some(bits);
            // 厳密性は変わらない: 旧キーが厳密なら新要素が真の最大、
            // 上界にすぎなかったなら新キーも上界のまま。
        } else if old_abs >= key {
            // 最大要素が減った/消えた可能性があるので、上界扱いに落とす
            col_exact[j] = false;
        }
    }

    // Markowitz 次数昇順走査用の列バケット: col_buckets[d] = 次数 d の列。
    // 行側はバケット化しない(スコア計算には `row_degree` だけで足りる)。
    let mut col_buckets: Vec<VecDeque<usize>> = vec![VecDeque::new(); p + 1];
    // col_bucket_pos[j] = 列 j のバケット内位置(None = バケット外 = 使用済み)
    let mut col_bucket_pos: Vec<Option<usize>> = vec![None; aug_n];
    for j in 0..aug_n {
        col_bucket_pos[j] = Some(col_buckets[col_degree[j]].len());
        col_buckets[col_degree[j]].push_back(j);
    }
    /// 要素 `idx` を次数 `degree` のバケットから取り除く(末尾要素と入れ替える O(1) 削除)。
    fn remove_from_bucket(buckets: &mut [VecDeque<usize>], pos: &mut [Option<usize>], degree: usize, idx: usize) {
        if let Some(p) = pos[idx] {
            let bucket = &mut buckets[degree];
            if p < bucket.len() {
                let last = bucket.pop_back().unwrap();
                if p < bucket.len() {
                    bucket[p] = last;
                    pos[last] = Some(p);
                }
            }
            pos[idx] = None;
        }
    }
    /// 要素 `idx` を次数 `old_degree` のバケットから `new_degree` のバケットへ移す
    /// (`used[idx]`、つまりピボット済みなら何もしない)。
    fn move_bucket(buckets: &mut [VecDeque<usize>], pos: &mut [Option<usize>], old_degree: usize, new_degree: usize, idx: usize, used: &[bool]) {
        if used[idx] || old_degree == new_degree {
            return;
        }
        remove_from_bucket(buckets, pos, old_degree, idx);
        let p = buckets[new_degree].len();
        pos[idx] = Some(p);
        buckets[new_degree].push_back(idx);
    }

    // ピボット済みの列・行
    let mut col_used = vec![false; aug_n];
    let mut row_used = vec![false; p];
    // keep[i] = 行 i が独立としてピボットを得たか
    let mut keep = vec![false; p];

    // 最大ランクは min(p, aug_n)。それ以上は探す意味がない
    let max_steps = p.min(aug_n);
    for _step in 0..max_steps {
        // `heap` の先頭が厳密になるまで再走査し、全体最大 |要素| を得る
        let global_max_abs = loop {
            match heap.iter().next_back() {
                Some(&(bits, j)) => {
                    if col_exact[j] {
                        break Some(f64::from_bits(bits));
                    }
                    refresh_col(&col_rows, &rows, &mut heap, &mut col_bits, j);
                    col_exact[j] = true;
                }
                None => break None,
            }
        };
        let Some(global_max_abs) = global_max_abs else {
            break; // 活性な列がすべて零
        };
        // ピボット候補として受理する最小 |値|(全体最大に対する相対閾値)
        let threshold = tunable!("ENOMOTO_T_REDEQ_PIVOT_STABILITY", PIVOT_STABILITY, f64) * global_max_abs;

        // 列の次数昇順バケット走査 (`find_best_pivot` と同様)。ただし候補は
        // 全体閾値 `threshold` を満たすものだけを `best` に記録する。
        PROF_TOTAL_STEPS.fetch_add(1, Ordering::Relaxed);
        // 最良候補 (行, 列)、その Markowitz スコアと |値|
        let mut best: Option<(usize, usize)> = None;
        let mut best_score = usize::MAX;
        let mut best_abs = 0.0f64;
        'search: for deg_col in 1..col_buckets.len() {
            for &j in &col_buckets[deg_col] {
                if col_used[j] {
                    continue;
                }
                // `col_bits[j]` は列 j の最大 |要素| の上界 (None = 全零)。
                // それが閾値未満なら列内に候補はありえないので O(1) で飛ばす
                // (ピボット選択は走査した場合と完全に同じ)。
                if col_bits[j].map_or(true, |bits| f64::from_bits(bits) < threshold) {
                    continue;
                }
                for &i in &col_rows[j] {
                    if row_used[i] {
                        continue;
                    }
                    let Some(&v) = rows[i].get(&j) else { continue };
                    if v == 0.0 || v.abs() < threshold {
                        continue;
                    }
                    // Markowitz スコア(fill-in の見積もり)
                    let score = (row_degree[i] - 1) * (col_degree[j] - 1);
                    if score < best_score || (score == best_score && v.abs() > best_abs) {
                        best_score = score;
                        best = Some((i, j));
                        best_abs = v.abs();
                    }
                }
                if best_score == 0 {
                    PROF_TRIVIAL_STEPS.fetch_add(1, Ordering::Relaxed);
                    break 'search;
                }
            }
            // 候補のスコアが現在の列次数の 2 乗以下なら十分良いとみなして走査を打ち切る
            // (`find_best_pivot` と同じ打ち切り条件)
            if best.is_some() && best_score <= deg_col * deg_col {
                break;
            }
        }
        // 選ばれたピボットの行・列
        let Some((piv_row, piv_col)) = best else {
            // 全体最大を実現する列は必ず閾値を満たすので、活性な非零列が
            // 残る限りここには来ない。防御的に「ランク尽き」とみなす。
            break;
        };

        let pivot_val = *rows[piv_row].get(&piv_col).unwrap();
        row_used[piv_row] = true;
        col_used[piv_col] = true;
        remove_from_bucket(&mut col_buckets, &mut col_bucket_pos, col_degree[piv_col], piv_col);
        if let Some(old) = col_bits[piv_col].take() {
            heap.remove(&(old, piv_col));
        }

        // ピボット行は(残すか従属かにかかわらず)退場するので、ピボット列
        // 以外の各列の行集合から取り除き、次数と `heap` の上界情報を更新する。
        // ピボット行が持つピボット列以外の列
        let piv_row_other_cols: Vec<usize> = rows[piv_row].keys().copied().filter(|&j| j != piv_col).collect();
        for &j in &piv_row_other_cols {
            col_rows[j].remove(&piv_row);
            let new_deg = col_rows[j].len();
            move_bucket(&mut col_buckets, &mut col_bucket_pos, new_deg + 1, new_deg, j, &col_used);
            col_degree[j] = new_deg;
            if !col_used[j] {
                let old_abs = rows[piv_row].get(&j).map_or(0.0, |v| v.abs());
                note_change(&mut heap, &mut col_bits, &mut col_exact, j, old_abs, 0.0);
            }
        }

        // 従属判定: 選ばれた時点での残差(ピボット値)が、行の元のノルムに
        // 比べて無視できるか。1e-300 は零ノルム行でのゼロ除算相当を避ける下限。
        if pivot_val.abs() <= tunable!("ENOMOTO_T_REDEQ_DEP_TOL", DEP_TOL, f64) * row_orig_norm[piv_row].max(1e-300) {
            // 従属: 消去せずに落とす (`keep[piv_row] = false` のまま)。
            // `heap` への反映は上の `note_change` で済んでいる。
            continue;
        }
        keep[piv_row] = true;

        // ピボット列を他の全行から消去する(`simplex::lu::MarkowitzState::eliminate`
        // と同じ scatter。各 (行, 列) につき `entry()` 1 回)。
        // ピボット行の (列, 値) の写し
        let pivot_row_snapshot: Vec<(usize, f64)> = rows[piv_row].iter().map(|(&j, &v)| (j, v)).collect();
        // ピボット列に非零を持つ他の行
        let affected_rows: Vec<usize> = col_rows[piv_col].iter().copied().filter(|&i| i != piv_row).collect();
        for i in affected_rows {
            let Some(&aij) = rows[i].get(&piv_col) else { continue };
            if aij == 0.0 {
                continue;
            }
            // 消去乗数
            let mult = aij / pivot_val;
            for &(j, v) in &pivot_row_snapshot {
                if j == piv_col {
                    continue;
                }
                use std::collections::btree_map::Entry;
                match rows[i].entry(j) {
                    // 既存要素の更新(ちょうど 0 になれば削除して次数を下げる)
                    Entry::Occupied(mut e) => {
                        let old_val = *e.get();
                        let new_val = old_val - mult * v;
                        if !col_used[j] {
                            note_change(&mut heap, &mut col_bits, &mut col_exact, j, old_val.abs(), new_val.abs());
                        }
                        if new_val == 0.0 {
                            e.remove();
                            col_rows[j].remove(&i);
                            let new_deg = col_rows[j].len();
                            move_bucket(&mut col_buckets, &mut col_bucket_pos, new_deg + 1, new_deg, j, &col_used);
                            col_degree[j] = new_deg;
                        } else {
                            *e.get_mut() = new_val;
                        }
                    }
                    // fill-in: 新しい非零の追加(次数を上げる)
                    Entry::Vacant(e) => {
                        let new_val = -mult * v;
                        if new_val != 0.0 {
                            if !col_used[j] {
                                note_change(&mut heap, &mut col_bits, &mut col_exact, j, 0.0, new_val.abs());
                            }
                            e.insert(new_val);
                            col_rows[j].insert(i);
                            let new_deg = col_rows[j].len();
                            move_bucket(&mut col_buckets, &mut col_bucket_pos, new_deg - 1, new_deg, j, &col_used);
                            col_degree[j] = new_deg;
                        }
                    }
                }
            }
            rows[i].remove(&piv_col);
            let new_row_degree = rows[i].len();
            row_degree[i] = new_row_degree;
        }
        col_rows[piv_col].clear();

        // 値の変化はすべて `note_change` で `heap` に反映済みなので、列の再走査は不要
    }

    (0..p).filter(|&i| keep[i]).collect()
}

/// 等式行を Dulmage-Mendelsohn 型のブロックに分割し、ブロックごとの行番号
/// リスト(各ブロック内は昇順、ブロックは先頭行の昇順)を返す。
///
/// 共通実装 [`crate::graph::dulmage_mendelsohn_blocks`](最大二部マッチング +
/// Tarjan の強連結成分分解)への薄いラッパで、各行の非零列パターンを隣接
/// リストとして渡す。単純な連結成分分解より常に細かい(粗くはならない)。
///
/// ただし、その行の最悪ケース活動量に対して無視できる係数
/// ([`smallcoeff::clean_row`] の判定、Achterberg et al.
/// "Presolve Reductions in MIP" §3.1)は辺から外す。`rows` 自体は変更せず、
/// グラフが見る辺だけが変わる(`lb`/`ub` はこの判定用の変数境界)。
///
/// 各ブロックを独立に(任意の順序・並列で)調べてよい理由: 冗長性判定は
/// 「この行は特定の他の行の一次結合か」という局所的な恒等式の確認で、
/// 行の全内容で確認すれば他ブロックに関係なく成り立つ。独立化の代償は
/// 複数ブロックにまたがる冗長性を見落とすこと(その行は保守的に残る)だけで、
/// 独立な行を誤って落とすことはない。辺を外す場合も同様。
fn dulmage_mendelsohn_blocks(rows: &[(Vec<(usize, f64)>, f64)], n: usize, lb: &[f64], ub: &[f64]) -> Vec<Vec<usize>> {
    // 各行の隣接リスト(無視できる小係数を除いた非零列)。clean_row の右辺引数はダミー
    let adj: Vec<Vec<usize>> = rows
        .iter()
        .map(|(row, _)| smallcoeff::clean_row(row, 0.0, lb, ub).0.into_iter().map(|(j, _)| j).collect())
        .collect();
    crate::graph::dulmage_mendelsohn_blocks(&adj, n)
}

/// [`drop_linearly_dependent_sparse`] の前にブロック分解
/// ([`dulmage_mendelsohn_blocks`]) を挟むラッパ。戻り値は残す行の添字(昇順)。
///
/// - 行数が [`MIN_ROWS_FOR_BLOCK_DECOMPOSE`] 未満、または分解で 1 ブロック
///   しか得られなければ、分解せずそのまま呼ぶ。
/// - 1 行だけのブロックは無条件に残す(他の行と実列を共有しないので従属
///   になりえない。`0 = 0` の零行は [`dedupe_rows`] で既に除去済み)。
/// - 2 行以上のブロックは、列番号を `0..local_n` に詰め直してから個別に
///   ランク判定する(作業配列を全体の `n` ではなくブロックの大きさにするため)。
///   総行数が [`PARALLEL_DECOMPOSE_ROW_THRESHOLD`] 以上かつ複数ブロックなら
///   `rayon` で並列処理する。
///
/// ブロックを独立に扱ってよい理由は [`dulmage_mendelsohn_blocks`] 参照。
/// 右辺列はグラフの辺として扱わないので、係数がすべて 0 で右辺が異なる行
/// 同士(`0 = 5` と `0 = 3`)は別ブロックになり両方残る。どちらもそれ自体
/// 実行不能の証拠なので、分解しない場合(片方を落とす)と比べて保守的な
/// だけで正しさは変わらない。
fn drop_linearly_dependent_sparse_blocked(rows_in: &[(Vec<(usize, f64)>, f64)], n: usize, lb: &[f64], ub: &[f64]) -> Vec<usize> {
    if rows_in.len() < tunable!("ENOMOTO_T_MIN_ROWS_FOR_BLOCK_DECOMPOSE", MIN_ROWS_FOR_BLOCK_DECOMPOSE, usize) {
        return drop_linearly_dependent_sparse(rows_in, n);
    }
    // ブロック(各要素は rows_in の行番号リスト)
    let components = dulmage_mendelsohn_blocks(rows_in, n, lb, ub);
    if components.len() <= 1 {
        return drop_linearly_dependent_sparse(rows_in, n);
    }

    // 残す行(rows_in の添字)
    let mut kept: Vec<usize> = Vec::new();
    // 2 行以上のブロック(ランク判定が必要なもの)
    let mut nontrivial: Vec<Vec<usize>> = Vec::with_capacity(components.len());
    for comp in components {
        if comp.len() == 1 {
            // 単独ブロックの行は他の行と実列を共有しないので従属になりえない
            // (右辺まで零の行は dedupe_rows で除去済み)。無条件に残す。
            kept.push(comp[0]);
        } else {
            nontrivial.push(comp);
        }
    }

    // 大きいブロックから処理する (LPT 順: 並列時の負荷分散のため)
    nontrivial.sort_by_key(|c| std::cmp::Reverse(c.len()));

    // 1 ブロックを列番号を詰め直してランク判定し、残す行を元の添字で返す
    let solve_component = |comp: &[usize]| -> Vec<usize> {
        // 元の列番号 -> ブロック内の局所列番号(初出順に採番)
        let mut col_map: HashMap<usize, usize> = HashMap::new();
        let local_rows: Vec<(Vec<(usize, f64)>, f64)> = comp
            .iter()
            .map(|&i| {
                let (row, rhs) = &rows_in[i];
                let local_row = row
                    .iter()
                    .map(|&(j, v)| {
                        let next_id = col_map.len();
                        let lj = *col_map.entry(j).or_insert(next_id);
                        (lj, v)
                    })
                    .collect();
                (local_row, *rhs)
            })
            .collect();
        let local_n = col_map.len();
        drop_linearly_dependent_sparse(&local_rows, local_n)
            .into_iter()
            .map(|local_idx| comp[local_idx])
            .collect::<Vec<usize>>()
    };

    // 並列化の判断に使う、ランク判定対象の総行数
    let total_nontrivial_rows: usize = nontrivial.iter().map(|c| c.len()).sum();
    if nontrivial.len() > 1 && total_nontrivial_rows >= tunable!("ENOMOTO_T_PARALLEL_DECOMPOSE_ROW_THRESHOLD", PARALLEL_DECOMPOSE_ROW_THRESHOLD, usize) {
        use rayon::prelude::*;
        kept.extend(nontrivial.par_iter().flat_map(|c| solve_component(c)).collect::<Vec<usize>>());
    } else {
        kept.extend(nontrivial.iter().flat_map(|c| solve_component(c)));
    }

    kept.sort_unstable();
    kept
}

/// 単体テスト。
#[cfg(test)]
mod tests {
    use super::*;

    /// ハッシュ版 `reduce_inequalities` が参照実装と同じ判定をし、ビット単位で同一の `(G, h)` を返すことを
    /// 乱数入力(完全一致・スカラー倍・符号反転の重複、空行、未整列入力を含む)で確認する。
    #[test]
    fn reduce_inequalities_matches_reference_bit_for_bit() {
        let mut state: u64 = 0x1234_5678_9abc_def0;
        let mut rnd = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for trial in 0..200 {
            let n = 6 + (trial % 5);
            let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
            let mut h: Vec<f64> = Vec::new();
            let base_count = 3 + (rnd() % 6) as usize;
            for _ in 0..base_count {
                let len = (rnd() % 4) as usize;
                let mut row: Vec<(usize, f64)> = Vec::new();
                for _ in 0..len {
                    let j = (rnd() % n as u64) as usize;
                    if row.iter().all(|&(k, _)| k != j) {
                        row.push((j, ((rnd() % 7) as f64 - 3.0) * 0.5 + 0.25));
                    }
                }
                rows.push(row);
                h.push((rnd() % 11) as f64 - 5.0);
            }
            // ランダムな行のスカラー倍(正負の倍率)の写しを追加(右辺は少しずらすこともある)
            for _ in 0..base_count {
                let src = (rnd() % base_count as u64) as usize;
                let f = [1.0, 2.0, 0.5, 3.0, -1.0, 1.0 / 3.0][(rnd() % 6) as usize];
                rows.push(rows[src].iter().map(|&(j, v)| (j, v * f)).collect());
                h.push(h[src] * f + ((rnd() % 3) as f64 - 1.0));
            }
            let g = csr_from_rows(&rows, n);
            let (g_ref, h_ref) = reduce_inequalities_reference(&g, &h, n);
            let (g_new, h_new) = reduce_inequalities(&g, &h, n);
            assert_eq!(g_new.as_ref().row_ptrs(), g_ref.as_ref().row_ptrs(), "trial {trial}");
            assert_eq!(g_new.as_ref().col_indices(), g_ref.as_ref().col_indices(), "trial {trial}");
            let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(g_new.as_ref().values()), bits(g_ref.as_ref().values()), "trial {trial}");
            assert_eq!(bits(&h_new), bits(&h_ref), "trial {trial}");
        }
    }

    /// 分割表現の `G` に対する `reduce_inequality_rows` が、実体化した `G` に対する
    /// `reduce_inequalities` と同じ実制約行を残し、境界行はすべて残ることを確認する。
    #[test]
    fn reduce_inequality_rows_matches_materialized_g() {
        let mut state: u64 = 0x0bad_cafe_1234_5678;
        let mut rnd = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut dropped_any = 0;
        for trial in 0..300 {
            let n = 5 + (trial % 4);
            let mut rows: Vec<Vec<(usize, f64)>> = Vec::new();
            let mut rhs: Vec<f64> = Vec::new();
            let base = 2 + (rnd() % 5) as usize;
            for _ in 0..base {
                let mut row: Vec<(usize, f64)> = Vec::new();
                while row.len() < 2 + (rnd() % 2) as usize {
                    let j = (rnd() % n as u64) as usize;
                    if row.iter().all(|&(k, _)| k != j) {
                        row.push((j, ((rnd() % 7) as f64 - 3.0) * 0.5 + 0.25));
                    }
                }
                row.sort_by_key(|&(j, _)| j);
                rows.push(row);
                rhs.push((rnd() % 11) as f64 - 5.0);
            }
            for _ in 0..base {
                let src = (rnd() % base as u64) as usize;
                let f = [1.0, 2.0, 0.5, 3.0, -1.0, 1.0 / 3.0][(rnd() % 6) as usize];
                rows.push(rows[src].iter().map(|&(j, v)| (j, v * f)).collect());
                rhs.push(rhs[src] * f + ((rnd() % 3) as f64 - 1.0));
            }
            let lb: Vec<f64> = (0..n).map(|_| [f64::NEG_INFINITY, 0.0, -1.0][(rnd() % 3) as usize]).collect();
            let ub: Vec<f64> = (0..n).map(|_| [f64::INFINITY, 2.0, 5.0][(rnd() % 3) as usize]).collect();
            assert!(crate::presolve::propagate::split_is_canonical(n, &rows, &lb, &ub));
            let (g, h) = crate::presolve::propagate::rebuild_g_ref(n, &rows, &rhs, &lb, &ub);
            let (g_mat, h_mat) = reduce_inequalities(&g, &h, n);
            let keep = reduce_inequality_rows(&rows, &rhs);
            if keep.is_some() {
                dropped_any += 1;
            }
            let kept: Vec<usize> = (0..rows.len()).filter(|&i| keep.as_ref().is_none_or(|k| k[i])).collect();
            let kept_rows: Vec<Vec<(usize, f64)>> = kept.iter().map(|&i| rows[i].clone()).collect();
            let kept_rhs: Vec<f64> = kept.iter().map(|&i| rhs[i]).collect();
            let (g_split, h_split) = crate::presolve::propagate::rebuild_g_ref(n, &kept_rows, &kept_rhs, &lb, &ub);
            assert_eq!(g_split.as_ref().row_ptrs(), g_mat.as_ref().row_ptrs(), "trial {trial}");
            assert_eq!(g_split.as_ref().col_indices(), g_mat.as_ref().col_indices(), "trial {trial}");
            let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
            assert_eq!(bits(g_split.as_ref().values()), bits(g_mat.as_ref().values()), "trial {trial}");
            assert_eq!(bits(&h_split), bits(&h_mat), "trial {trial}");
        }
        assert!(dropped_any > 50, "only {dropped_any} trials dropped a row");
    }

    /// `dedupe_rows`(その場ハッシュ + 連鎖)が `HashSet` 版の参照実装と同じ行を同じ順序で残すことを確認する。
    #[test]
    fn dedupe_rows_matches_reference_bit_for_bit() {
        let mut state: u64 = 0x0fed_cba9_8765_4321;
        let mut rnd = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for trial in 0..300 {
            let n = 4 + (trial % 5);
            let mut rows: Vec<(Vec<(usize, f64)>, f64)> = Vec::new();
            let base_count = 2 + (rnd() % 6) as usize;
            for _ in 0..base_count {
                let len = (rnd() % 4) as usize;
                let mut row: Vec<(usize, f64)> = Vec::new();
                for _ in 0..len {
                    let j = (rnd() % n as u64) as usize;
                    if row.iter().all(|&(k, _)| k != j) {
                        row.push((j, ((rnd() % 7) as f64 - 3.0) * 0.5 + 0.25));
                    }
                }
                rows.push((row, [0.0, 1.0, -2.0, 0.5][(rnd() % 4) as usize]));
            }
            for _ in 0..base_count {
                let src = (rnd() % base_count as u64) as usize;
                let f = [1.0, 2.0, 0.5, 3.0, -1.0, 1.0 / 3.0][(rnd() % 6) as usize];
                let (r, b) = rows[src].clone();
                let db = [0.0, 0.0, 1.0][(rnd() % 3) as usize];
                rows.push((r.iter().map(|&(j, v)| (j, v * f)).collect(), b * f + db));
            }
            let got = dedupe_rows(rows.clone());
            let want = dedupe_rows_reference(rows);
            assert_eq!(got.len(), want.len(), "trial {trial}");
            for ((gr, gb), (wr, wb)) in got.iter().zip(&want) {
                assert_eq!(gb.to_bits(), wb.to_bits(), "trial {trial}");
                assert_eq!(gr.len(), wr.len(), "trial {trial}");
                for (&(gj, gv), &(wj, wv)) in gr.iter().zip(wr) {
                    assert_eq!((gj, gv.to_bits()), (wj, wv.to_bits()), "trial {trial}");
                }
            }
        }
    }

    /// 密 QR がピボット位置 -> 元の行の対応に faer の 2 つの置換配列のうち正しい方を使っていることを確認する。
    ///
    /// 行 0, 2, 3 が従属集合 (`r3 = r0 + r2`)、行 1 は独立。列ノルムによりピボット順が
    /// `r1, r3, ...`(自己逆でない置換)となるので、誤った配列を使うと行 1 が落ちる。
    #[test]
    fn drop_linearly_dependent_maps_pivot_positions_to_the_right_rows() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 3.0)], 3.0),
            (vec![(1, 10.0)], 10.0),
            (vec![(2, 5.0)], 5.0),
            (vec![(0, 3.0), (2, 5.0)], 8.0),
        ];
        let keep = drop_linearly_dependent(&rows, 3);
        assert_eq!(keep.len(), 3, "keep={keep:?}");
        assert!(keep.contains(&1), "independent row 1 must survive; keep={keep:?}");
        assert_eq!(keep.iter().filter(|&&i| i != 1).count(), 2, "keep={keep:?}");
    }

    /// 上と同じ例を疎版で確認する。従属集合のどの 2 行が残るかはピボット順次第なので、
    /// 残る行数と行 1 が残ることだけを確認する。
    #[test]
    fn drop_linearly_dependent_sparse_matches_dense_on_the_same_case() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 3.0)], 3.0),
            (vec![(1, 10.0)], 10.0),
            (vec![(2, 5.0)], 5.0),
            (vec![(0, 3.0), (2, 5.0)], 8.0),
        ];
        let keep = drop_linearly_dependent_sparse(&rows, 3);
        assert_eq!(keep.len(), 3, "keep={keep:?}");
        assert!(keep.contains(&1), "independent row 1 must survive; keep={keep:?}");
        assert_eq!(keep.iter().filter(|&&i| i != 1).count(), 2, "keep={keep:?}");
    }

    /// 実インスタンスの `(n, p, nnz)` 形状について、[`reduce_equalities`] の密度計算が
    /// [`DENSE_DENSITY_THRESHOLD`] に照らして期待どおり密/疎経路を選ぶことを確認する。
    #[test]
    fn dense_density_threshold_routes_known_instances_correctly() {
        let density = |n: usize, p: usize, nnz: usize| nnz as f64 / (p as f64 * n as f64);
        // wood1p: p=243, n=2594, nnz=70214 (密度 11.1%) -- 密経路になるべき
        assert!(density(2594, 243, 70214) > DENSE_DENSITY_THRESHOLD);
        // standmps: p=268, n=1075, nnz=2776 (密度 0.96%) -- 疎経路のままであるべき
        assert!(density(1075, 268, 2776) <= DENSE_DENSITY_THRESHOLD);
        // fffff800: p=350, n=854, nnz=4775 (密度 1.6%) -- 疎経路のままであるべき
        assert!(density(854, 350, 4775) <= DENSE_DENSITY_THRESHOLD);
        // ganges: p=1284, n=1681, nnz=6612 (密度 0.31%) -- 疎経路のままであるべき
        assert!(density(1681, 1284, 6612) <= DENSE_DENSITY_THRESHOLD);
    }

    /// 係数は一次結合だが右辺が矛盾する行(実行不能であって冗長ではない)を疎版が落とさないことを確認する。
    /// `r0 - r1` の係数 `(1,0,-1)` は `r2` と一致するが、`3-4=-1 != 7`。
    #[test]
    fn drop_linearly_dependent_sparse_keeps_rhs_inconsistent_rows() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 1.0), (1, 1.0)], 3.0),
            (vec![(1, 1.0), (2, 1.0)], 4.0),
            (vec![(0, 1.0), (2, -1.0)], 7.0),
        ];
        let keep = drop_linearly_dependent_sparse(&rows, 3);
        assert_eq!(keep, vec![0, 1, 2], "an rhs-inconsistent row must survive; keep={keep:?}");
    }

    /// 従属関係がないとき、疎版がすべての行を残すことを確認する。
    #[test]
    fn drop_linearly_dependent_sparse_keeps_everything_when_independent() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 1.0), (1, 2.0)], 5.0),
            (vec![(1, 1.0), (2, 3.0)], 7.0),
            (vec![(0, 2.0), (2, 1.0)], 4.0),
        ];
        let keep = drop_linearly_dependent_sparse(&rows, 3);
        assert_eq!(keep, vec![0, 1, 2], "keep={keep:?}");
    }

    /// 完全に重複した行(係数も右辺も同じ)を疎版が落とし、先に現れた方を残すことを確認する。
    #[test]
    fn drop_linearly_dependent_sparse_drops_an_exact_duplicate() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 1.0), (1, 1.0)], 5.0),
            (vec![(0, 1.0), (1, 1.0)], 5.0),
            (vec![(1, 1.0), (2, 1.0)], 3.0),
        ];
        let keep = drop_linearly_dependent_sparse(&rows, 3);
        assert_eq!(keep, vec![0, 2], "keep={keep:?}");
    }

    /// 自明でない係数の一次結合で作った従属行を含む 5 行の系で、疎版が正しいランク 4 を得ることを確認する
    /// (どの行が落ちるかは問わない)。
    #[test]
    fn drop_linearly_dependent_sparse_finds_the_right_rank_in_a_larger_system() {
        let n = 6;
        let base: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 2.0), (1, 1.0)], 5.0),
            (vec![(1, 3.0), (2, 1.0)], 8.0),
            (vec![(2, 1.0), (3, 4.0)], 2.0),
            (vec![(3, 1.0), (4, 2.0), (5, 1.0)], 6.0),
        ];
        // 行 4 = 2*行0 - 行1 + 3*行2(係数も右辺も同じ結合なので、矛盾ではなく冗長)
        let mut combo: BTreeMap<usize, f64> = BTreeMap::new();
        let mut rhs = 0.0;
        for (mult, (row, r)) in [(2.0, &base[0]), (-1.0, &base[1]), (3.0, &base[2])] {
            for &(j, v) in row {
                *combo.entry(j).or_insert(0.0) += mult * v;
            }
            rhs += mult * r;
        }
        let mut rows = base.clone();
        rows.push((combo.into_iter().collect(), rhs));

        let keep = drop_linearly_dependent_sparse(&rows, n);
        assert_eq!(keep.len(), 4, "expected rank 4 out of 5 rows; keep={keep:?}");
    }

    /// 互いに列を共有しない 2 つの巡回的な 2 行グループと孤立した 1 行が、ちょうど 3 ブロック
    /// (各ブロック内は行の昇順、ブロックは先頭行の昇順)に分かれることを確認する。
    #[test]
    fn dulmage_mendelsohn_blocks_splits_disjoint_cyclic_groups() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 1.0), (1, 2.0)], 3.0),
            (vec![(0, 2.0), (1, 1.0)], 4.0),
            (vec![(2, 1.0), (3, 1.0)], 1.0),
            (vec![(2, 1.0), (3, 2.0)], 2.0),
            (vec![(4, 5.0)], 5.0),
        ];
        let comps = dulmage_mendelsohn_blocks(&rows, 5, &[f64::NEG_INFINITY; 5], &[f64::INFINITY; 5]);
        assert_eq!(comps, vec![vec![0, 1], vec![2, 3], vec![4]], "comps={comps:?}");
    }

    /// 隣り合う行が列を共有するだけの鎖状の系(列共有では 1 つの連結成分だが、マッチンググラフに
    /// 巡回がない)が、単独行ブロックに分かれることを確認する。
    #[test]
    fn dulmage_mendelsohn_blocks_splits_a_pure_chain_into_singletons() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 1.0), (1, 1.0)], 1.0),
            (vec![(1, 1.0), (2, 1.0)], 1.0),
            (vec![(2, 1.0), (3, 1.0)], 1.0),
        ];
        let comps = dulmage_mendelsohn_blocks(&rows, 4, &[f64::NEG_INFINITY; 4], &[f64::INFINITY; 4]);
        assert_eq!(comps, vec![vec![0], vec![1], vec![2]], "comps={comps:?}");
    }

    /// 全体が 1 ブロックの場合(`components.len() <= 1` の直接呼び出し経路)に、ブロック版が
    /// 非ブロック版と同じ結果になることを確認する。
    #[test]
    fn drop_linearly_dependent_sparse_blocked_matches_unblocked_on_a_single_component() {
        let rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            (vec![(0, 1.0), (1, 2.0)], 5.0),
            (vec![(1, 1.0), (2, 3.0)], 7.0),
            (vec![(0, 2.0), (2, 1.0)], 4.0),
        ];
        let keep = drop_linearly_dependent_sparse_blocked(&rows, 3, &[f64::NEG_INFINITY; 3], &[f64::INFINITY; 3]);
        assert_eq!(keep, vec![0, 1, 2], "keep={keep:?}");
    }

    /// それぞれ 1 行ずつ従属な 2 つの独立ブロックが別々に処理され、各ブロックでランク 2 になることを確認する。
    ///
    /// 各ブロックの 3 行はマッチンググラフ上で巡回をなすので 1 つの SCC にまとまり、2 ブロックは列を
    /// 共有しないので別の SCC になる。[`MIN_ROWS_FOR_BLOCK_DECOMPOSE`] を超えて実際に分解経路を
    /// 通すため、独立な単独行を詰め物として加えている(すべて残るはず)。
    #[test]
    fn drop_linearly_dependent_sparse_blocked_handles_independent_blocks_separately() {
        let mut rows: Vec<(Vec<(usize, f64)>, f64)> = vec![
            // ブロック A: 列 {0,1,2}、行 2 = 行 0 + 行 1 (x0+2x1+x2=7)
            (vec![(0, 1.0), (1, 1.0)], 3.0),
            (vec![(1, 1.0), (2, 1.0)], 4.0),
            (vec![(0, 1.0), (1, 2.0), (2, 1.0)], 7.0),
            // ブロック B: 列 {3,4,5}、行 5 = 2*行 3 - 行 4 (2x3+x4-x5=-1)
            (vec![(3, 1.0), (4, 1.0)], 2.0),
            (vec![(4, 1.0), (5, 1.0)], 5.0),
            (vec![(3, 2.0), (4, 1.0), (5, -1.0)], -1.0),
        ];
        let filler_count = MIN_ROWS_FOR_BLOCK_DECOMPOSE;
        for k in 0..filler_count {
            rows.push((vec![(6 + k, 1.0)], 1.0));
        }
        let n = 6 + filler_count;
        assert!(rows.len() >= MIN_ROWS_FOR_BLOCK_DECOMPOSE, "test must actually exercise dulmage_mendelsohn_blocks");

        let keep = drop_linearly_dependent_sparse_blocked(&rows, n, &vec![f64::NEG_INFINITY; n], &vec![f64::INFINITY; n]);
        assert_eq!(keep.len(), 4 + filler_count, "expected rank 2+2+{filler_count} singletons; keep={keep:?}");
        let keep_a = keep.iter().filter(|&&i| i < 3).count();
        let keep_b = keep.iter().filter(|&&i| (3..6).contains(&i)).count();
        let keep_filler = keep.iter().filter(|&&i| i >= 6).count();
        assert_eq!(keep_a, 2, "block A must independently reduce to rank 2; keep={keep:?}");
        assert_eq!(keep_b, 2, "block B must independently reduce to rank 2; keep={keep:?}");
        assert_eq!(keep_filler, filler_count, "every isolated singleton row must survive; keep={keep:?}");
    }

    /// ブロック数を増やして [`MIN_ROWS_FOR_BLOCK_DECOMPOSE`] と [`PARALLEL_DECOMPOSE_ROW_THRESHOLD`] の
    /// 両方を超え、`rayon` 並列経路でも各ブロックがランク 2 になることを確認する。
    #[test]
    fn drop_linearly_dependent_sparse_blocked_matches_sequential_result_under_parallel_dispatch() {
        let block_count = 120; // 120 * 3 = 360 行で上記 2 つの閾値を両方超える
        let mut rows: Vec<(Vec<(usize, f64)>, f64)> = Vec::new();
        for b in 0..block_count {
            let base = b * 3;
            rows.push((vec![(base, 1.0), (base + 1, 1.0)], 3.0));
            rows.push((vec![(base + 1, 1.0), (base + 2, 1.0)], 4.0));
            rows.push((vec![(base, 1.0), (base + 1, 2.0), (base + 2, 1.0)], 7.0)); // = 行0 + 行1
        }
        let n = block_count * 3;
        assert!(rows.len() >= MIN_ROWS_FOR_BLOCK_DECOMPOSE, "test must clear the small-input fallback gate");
        assert!(rows.len() >= PARALLEL_DECOMPOSE_ROW_THRESHOLD, "test must actually exercise the parallel path");
        let keep = drop_linearly_dependent_sparse_blocked(&rows, n, &vec![f64::NEG_INFINITY; n], &vec![f64::INFINITY; n]);
        assert_eq!(keep.len(), block_count * 2, "expected rank 2 per block; keep={keep:?}");
        for b in 0..block_count {
            let in_block = keep.iter().filter(|&&i| i / 3 == b).count();
            assert_eq!(in_block, 2, "block {b} must independently reduce to rank 2; keep={keep:?}");
        }
    }
}

/// 不等式制約 `(G, h)`(`G x <= h`)から、正のスカラー倍で一致する重複行を
/// 取り除いた `(G', h')` を返す。PaPILO の "ParallelRows"
/// (Achterberg et al. 2019, §4.4) にあたり、[`reduce_equalities`] の段階 1
/// の不等式版(ランク判定に相当する処理はない)。
///
/// 不等式では符号が意味を持つ(`a.x <= h` と `(-a).x <= h'` は別の制約)ので、
/// 符号を変えない `|先頭係数|` で割って正規化する。正規化後の係数パターンが
/// 一致する行同士は同じ一次式を上から抑えているので、正規化後の右辺が
/// 小さい(きつい)方だけを残す。空行は常に残す。
///
/// 判定はテスト用の参照実装 `reduce_inequalities_reference` と完全に一致し、
/// 何も落とさず `g` が既に正準形ならそのまま複製を返す。
pub fn reduce_inequalities(g: &FaerCsr, h: &[f64], n: usize) -> (FaerCsr, Vec<f64>) {
    // 不等式行数
    let m = g.nrows();
    if m == 0 {
        return (csr_from_rows(&[], n), Vec::new());
    }
    let gr = g.as_ref();

    // 正規化シグネチャを CSR から直接その場でハッシュし、ハッシュ一致時は
    // クラス代表行のシグネチャを再計算して要素ごとに照合する。
    /// ハッシュ値 `hash` に 64 ビット値 `x` を混ぜ込む乗算型ミキサ。
    #[inline]
    fn mix(hash: u64, x: u64) -> u64 {
        (hash.rotate_left(5) ^ x).wrapping_mul(0x517c_c1b7_2722_0a95)
    }
    /// 正規化シグネチャが等しい行の同値類。
    struct Class {
        /// 代表行(この類を最初に作った行)の添字
        rep: usize,
        /// 代表行の正規化係数 `1 / |先頭係数|`
        rep_inv: f64,
        /// 現在残している行の添字(正規化右辺が最小の行)
        kept_idx: usize,
        /// 残している行の正規化右辺
        kept_h: f64,
        /// 同じハッシュ連鎖の次の類(末尾は usize::MAX)
        next: usize,
    }
    // 非零が 1 個の行(`rebuild_g` が出す変数境界行で、通常 `g` の大半)は
    // ハッシュせず、その列 j をキーに `unit_head[j]` から連鎖をたどる
    // (長さが違う行は同じ類にならないので判定は変わらない)。
    // 非零 2 個以上の行数(`heads` の初期容量)
    let multi_rows = (0..m).filter(|&i| gr.col_indices_of_row_raw(i).len() > 1).count();
    // ハッシュ値 -> 連鎖の先頭の類 (`classes` の添字)
    let mut heads: HashMap<u64, usize, std::hash::BuildHasherDefault<IdentityU64Hasher>> = HashMap::with_capacity_and_hasher(multi_rows, Default::default());
    // unit_head[j] = 列 j のみを持つ行の類連鎖の先頭 (usize::MAX = なし)
    let mut unit_head: Vec<usize> = vec![usize::MAX; n];
    let mut classes: Vec<Class> = Vec::with_capacity(m);
    // keep[i] = 行 i を残すか
    let mut keep = vec![true; m];
    // 1 行でも落としたか
    let mut any_dropped = false;
    for idx in 0..m {
        let cols = gr.col_indices_of_row_raw(idx);
        let vals = gr.values_of_row(idx);
        if cols.is_empty() {
            continue;
        }
        let hv = h[idx];
        // 正規化に使う |先頭係数| とその逆数
        let scale = vals[0].abs();
        let inv = 1.0 / scale;
        // 非零 1 個の行か
        let unit = cols.len() == 1;
        let mut hash = cols.len() as u64;
        if !unit {
            for (&j, &v) in cols.iter().zip(vals) {
                hash = mix(mix(hash, j as u64), (v * inv).to_bits());
            }
        }
        // 正規化後の右辺
        let normalized_h = hv * inv;
        // 類 c の代表行とこの行の正規化シグネチャがビット単位で一致するか
        let same_sig = |c: &Class| -> bool {
            let rc = gr.col_indices_of_row_raw(c.rep);
            if rc.len() != cols.len() {
                return false;
            }
            let rv = gr.values_of_row(c.rep);
            rc.iter().zip(rv).zip(cols.iter().zip(vals)).all(|((&rj, &rvv), (&j, &v))| rj == j && (rvv * c.rep_inv).to_bits() == (v * inv).to_bits())
        };
        // 一致する既存の類
        let mut found: Option<usize> = None;
        // この行が属しうる連鎖の先頭
        let head = if unit {
            Some(unit_head[cols[0]]).filter(|&h| h != usize::MAX)
        } else {
            heads.get(&hash).copied()
        };
        let mut cur = head.unwrap_or(usize::MAX);
        while cur != usize::MAX {
            if same_sig(&classes[cur]) {
                found = Some(cur);
                break;
            }
            cur = classes[cur].next;
        }
        match found {
            None => {
                let id = classes.len();
                classes.push(Class { rep: idx, rep_inv: inv, kept_idx: idx, kept_h: normalized_h, next: head.unwrap_or(usize::MAX) });
                if unit {
                    unit_head[cols[0]] = id;
                } else {
                    heads.insert(hash, id);
                }
            }
            // 既存の類に属する: きつい方を残し、もう一方を落とす
            Some(ci) => {
                any_dropped = true;
                let c = &mut classes[ci];
                if normalized_h < c.kept_h {
                    keep[c.kept_idx] = false;
                    c.kept_idx = idx;
                    c.kept_h = normalized_h;
                } else {
                    keep[idx] = false;
                }
            }
        }
    }

    // 何も落とさず `g` が既に正準形(格納された零なし、各行の列が狭義昇順:
    // `csr_from_rows` が作るのと同じ形)なら、作り直した行列は `g` そのもの。
    if !any_dropped {
        if csr_is_canonical(g) {
            return (g.clone(), h.to_vec());
        }
    }
    // 残す行の添字
    let kept_rows: Vec<usize> = (0..m).filter(|&i| keep[i]).collect();
    let nnz: usize = kept_rows.iter().map(|&i| gr.col_indices_of_row_raw(i).len()).sum();
    let mut builder = CsrRowBuilder::with_capacity(n, kept_rows.len(), nnz);
    // 1 行分の (列, 値) の再利用バッファ
    let mut row_buf: Vec<(usize, f64)> = Vec::new();
    for &i in &kept_rows {
        row_buf.clear();
        row_buf.extend(csr_row_iter(g, i));
        // 重複列や範囲外の列があって追加できなければ、汎用の csr_from_rows で作り直す
        if !builder.push_row(&row_buf) {
            let rows: Vec<Vec<(usize, f64)>> = kept_rows.iter().map(|&i| csr_row_vec(g, i)).collect();
            return (csr_from_rows(&rows, n), kept_rows.iter().map(|&i| h[i]).collect());
        }
    }
    (builder.finish(), kept_rows.iter().map(|&i| h[i]).collect())
}

/// 分割表現の `G`(`propagate::GView::Split`: 非零 2 個以上の正準な実制約行
/// `rows` と、有限な境界ごとの単位境界行からなる)に対する
/// [`reduce_inequalities`]。
///
/// 境界行は互いにも(列ごとに `(j, +1)` と `(j, -1)` が高々 1 本ずつ)、
/// 非零 2 個以上の行とも同じ類にならないので決して落ちず、`rows` 間の判定は
/// 実体化した `G` に対する [`reduce_inequalities`] と完全に一致する。
/// 戻り値は `rows` 上の残すかどうかのマスク。何も落とさなければ `None`。
/// `rhs[i]` は `rows[i]` の右辺。
pub fn reduce_inequality_rows(rows: &[Vec<(usize, f64)>], rhs: &[f64]) -> Option<Vec<bool>> {
    /// ハッシュ値 `hash` に 64 ビット値 `x` を混ぜ込む乗算型ミキサ([`reduce_inequalities`] と同じ)。
    #[inline]
    fn mix(hash: u64, x: u64) -> u64 {
        (hash.rotate_left(5) ^ x).wrapping_mul(0x517c_c1b7_2722_0a95)
    }
    /// 正規化シグネチャが等しい行の同値類(フィールドの意味は [`reduce_inequalities`] 内のものと同じ)。
    struct Class {
        /// 代表行の添字
        rep: usize,
        /// 代表行の正規化係数 `1 / |先頭係数|`
        rep_inv: f64,
        /// 現在残している行の添字
        kept_idx: usize,
        /// 残している行の正規化右辺
        kept_h: f64,
        /// 同じハッシュ連鎖の次の類(末尾は usize::MAX)
        next: usize,
    }
    let m = rows.len();
    // ハッシュ値 -> 連鎖の先頭の類
    let mut heads: HashMap<u64, usize, std::hash::BuildHasherDefault<IdentityU64Hasher>> = HashMap::with_capacity_and_hasher(m, Default::default());
    let mut classes: Vec<Class> = Vec::with_capacity(m);
    // 最初に落とす行が見つかった時点で確保する残す/落とすマスク
    let mut keep: Option<Vec<bool>> = None;
    for (idx, row) in rows.iter().enumerate() {
        debug_assert!(row.len() >= 2);
        // 正規化係数 1 / |先頭係数|
        let inv = 1.0 / row[0].1.abs();
        let mut hash = row.len() as u64;
        for &(j, v) in row {
            hash = mix(mix(hash, j as u64), (v * inv).to_bits());
        }
        // 正規化後の右辺
        let normalized_h = rhs[idx] * inv;
        // 類 c の代表行とこの行の正規化シグネチャがビット単位で一致するか
        let same_sig = |c: &Class| -> bool {
            let rep = &rows[c.rep];
            rep.len() == row.len() && rep.iter().zip(row).all(|(&(rj, rv), &(j, v))| rj == j && (rv * c.rep_inv).to_bits() == (v * inv).to_bits())
        };
        let head = heads.get(&hash).copied();
        // 一致する既存の類
        let mut found: Option<usize> = None;
        let mut cur = head.unwrap_or(usize::MAX);
        while cur != usize::MAX {
            if same_sig(&classes[cur]) {
                found = Some(cur);
                break;
            }
            cur = classes[cur].next;
        }
        match found {
            None => {
                let id = classes.len();
                classes.push(Class { rep: idx, rep_inv: inv, kept_idx: idx, kept_h: normalized_h, next: head.unwrap_or(usize::MAX) });
                heads.insert(hash, id);
            }
            Some(ci) => {
                let keep = keep.get_or_insert_with(|| vec![true; m]);
                let c = &mut classes[ci];
                if normalized_h < c.kept_h {
                    keep[c.kept_idx] = false;
                    c.kept_idx = idx;
                    c.kept_h = normalized_h;
                } else {
                    keep[idx] = false;
                }
            }
        }
    }
    keep
}

/// 既に十分混ぜ合わされた 64 ビットのハッシュ値をキーとする `HashMap` 用の
/// 恒等ハッシャ(値をそのまま返す)。
#[derive(Default)]
pub(crate) struct IdentityU64Hasher(u64);

impl std::hash::Hasher for IdentityU64Hasher {
    /// 保持している値をそのままハッシュ値として返す。
    fn finish(&self) -> u64 {
        self.0
    }
    /// 任意バイト列用(通常は使われない): バイトを順に左シフトで詰め込む。
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 << 8) | b as u64;
        }
    }
    /// `u64` キーをそのまま保持する。
    fn write_u64(&mut self, x: u64) {
        self.0 = x;
    }
}

/// テスト用の単純な参照実装: 行ごとに正規化シグネチャの `Vec` を作り
/// `HashMap` で重複判定する。[`reduce_inequalities`] と同じ結果になるべきもの。
#[cfg(test)]
fn reduce_inequalities_reference(g: &FaerCsr, h: &[f64], n: usize) -> (FaerCsr, Vec<f64>) {
    let m = g.nrows();
    if m == 0 {
        return (csr_from_rows(&[], n), Vec::new());
    }

    let rows: Vec<(Vec<(usize, f64)>, f64)> = (0..m)
        .map(|i| {
            let row: Vec<(usize, f64)> = csr_row_vec(g, i);
            (row, h[i])
        })
        .collect();

    // 正規化シグネチャ -> (現在残している行の添字, その正規化右辺)
    let mut best: HashMap<Vec<(usize, u64)>, (usize, f64)> = HashMap::new();
    let mut keep = vec![true; m];
    for (idx, (row, hv)) in rows.iter().enumerate() {
        if row.is_empty() {
            // `0 <= h` は重複の対象外なので残す(常に真か、実行不能の証拠で
            // それは `propagate` の活動量検査が検出する)。
            continue;
        }
        let scale = row[0].1.abs();
        let inv = 1.0 / scale;
        let sig: Vec<(usize, u64)> = row.iter().map(|&(j, v)| (j, (v * inv).to_bits())).collect();
        let normalized_h = hv * inv;
        match best.get_mut(&sig) {
            None => {
                best.insert(sig, (idx, normalized_h));
            }
            Some((kept_idx, kept_h)) => {
                if normalized_h < *kept_h {
                    keep[*kept_idx] = false;
                    *kept_idx = idx;
                    *kept_h = normalized_h;
                } else {
                    keep[idx] = false;
                }
            }
        }
    }

    let mut new_rows = Vec::new();
    let mut new_h = Vec::new();
    for (idx, (row, hv)) in rows.into_iter().enumerate() {
        if keep[idx] {
            new_rows.push(row);
            new_h.push(hv);
        }
    }
    (csr_from_rows(&new_rows, n), new_h)
}
