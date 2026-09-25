//! 前処理 ([`crate::presolve::redundancy`] の冗長行のブロック分解) と基底の LU 分解
//! ([`crate::simplex::lu`]) で共用する小さなグラフアルゴリズム集:
//! 最大二部マッチング (Kuhn 法)、強連結成分分解 (反復版 Tarjan 法)、および
//! それらを組み合わせた Dulmage-Mendelsohn 型のブロック分解。
//!
//! 入力は行ごとの隣接リスト `adj: &[Vec<usize>]` (行番号 → 非零のある列番号) だけで、
//! 値や右辺など呼び出し側固有の構造は仮定しない。

use std::collections::HashMap;

/// `adj` が定める二部グラフで、行 `0..adj.len()` と列 `0..n_cols` の最大マッチングを
/// Kuhn 法 (増加路探索の繰り返し。最悪 `O(行数 * nnz)`) で求める。各行について DFS で
/// 空いている列、または現在の相手を別の列へ付け替えられる列を探す。
///
/// 戻り値は `match_row[i] = Some(列)` (行 `i` がマッチした列) で、マッチできなかった行は
/// `None`。DFS は長い増加路でもスタックオーバーフローしないよう明示的なスタックで行う。
pub fn max_bipartite_matching(adj: &[Vec<usize>], n_cols: usize) -> Vec<Option<usize>> {
    let n_rows = adj.len();
    // 列 → その列にマッチしている行
    let mut match_col: Vec<Option<usize>> = vec![None; n_cols];

    // 増加路 DFS の反復版。再帰版の `for col in adj[row]` を、フレームごとの
    // カーソル `idx` で再開できる形にしたもの。成功した経路上の全フレームは
    // 「自分が試していた列を自分の行にマッチさせる」という同じ最終処理を行うので、
    // 1 回の成功で経路全体を一気に巻き戻して割り当てればよい。
    /// 増加路 DFS のスタックフレーム。
    struct Frame {
        /// 探索中の行。
        row: usize,
        /// `adj[row]` の次に試す位置。
        idx: usize,
    }
    /// 行 `start` から増加路を探し、見つかれば経路に沿ってマッチングを更新して true を返す。
    /// `visited[col] == stamp` の列はこの探索で訪問済み。
    fn try_augment(start: usize, adj: &[Vec<usize>], visited: &mut [u32], stamp: u32, match_col: &mut [Option<usize>]) -> bool {
        let mut stack: Vec<Frame> = vec![Frame { row: start, idx: 0 }];
        // `col_stack[k]` は `stack[k]` が `stack[k+1]` を積んだときに試していた列
        // (その時点で `match_col[col_stack[k]] == Some(stack[k+1].row)`)。
        // 常に `stack` より 1 つ短い (最下段のフレームは列のために積まれたのではない)。
        let mut col_stack: Vec<usize> = Vec::new();

        loop {
            let top = stack.len() - 1;
            let row = stack[top].row;
            // この行で次に試す未訪問の列
            let mut chosen: Option<usize> = None;
            while stack[top].idx < adj[row].len() {
                let col = adj[row][stack[top].idx];
                stack[top].idx += 1;
                if visited[col] == stamp {
                    continue;
                }
                visited[col] = stamp;
                chosen = Some(col);
                break;
            }

            let Some(col) = chosen else {
                // この行の隣接列を使い切って失敗 (再帰版で false を返すのに相当)。
                stack.pop();
                let Some(_) = stack.last() else { return false };
                col_stack.pop();
                continue;
            };

            match match_col[col] {
                Some(r2) => {
                    // 使用中: その列を持っている行へ「再帰」する。
                    stack.push(Frame { row: r2, idx: 0 });
                    col_stack.push(col);
                }
                None => {
                    // 空き列: 成功。経路上の全フレームで「試していた列 ← 自分の行」を
                    // 割り当てながら一気に巻き戻す。
                    match_col[col] = Some(row);
                    stack.pop();
                    while let Some(parent) = stack.pop() {
                        let parent_col = col_stack.pop().expect("one col_stack entry per stack frame above the bottom");
                        match_col[parent_col] = Some(parent.row);
                    }
                    return true;
                }
            }
        }
    }

    // 訪問済み印 (エポックスタンプ方式: 行ごとに番号を変えることで、毎回 O(n_cols) で
    // クリアせずに新しい訪問集合として使える)
    let mut visited: Vec<u32> = vec![0; n_cols];
    for i in 0..n_rows {
        try_augment(i, adj, &mut visited, (i + 1) as u32, &mut match_col);
    }

    let mut match_row: Vec<Option<usize>> = vec![None; n_rows];
    for (col, row) in match_col.into_iter().enumerate() {
        if let Some(r) = row {
            match_row[r] = Some(col);
        }
    }
    match_row
}

/// 反復版の Tarjan 強連結成分分解 (再帰版は長い依存鎖でスタックオーバーフローしうるため)。
/// `adj[i]` はノード `i` から出る有向辺の行き先。各強連結成分をノード番号の
/// `Vec<usize>` で返す。成分間の順序は不定 (決定性が必要なら呼び出し側で並べ替える)。
pub fn tarjan_scc(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = adj.len();
    // DFS の訪問順番号 (未訪問は None)
    let mut index: Vec<Option<u32>> = vec![None; n];
    // 到達可能な最小の訪問順番号
    let mut lowlink: Vec<u32> = vec![0; n];
    let mut on_stack: Vec<bool> = vec![false; n];
    // Tarjan のノードスタック
    let mut stack: Vec<usize> = Vec::new();
    let mut next_index: u32 = 0;
    let mut components: Vec<Vec<usize>> = Vec::new();

    /// 明示的な作業スタックのフレーム。
    enum Frame {
        /// ノードに初めて入る。
        Enter(usize),
        /// ノードの隣接リストを、指定位置から処理再開する (再帰呼び出しからの復帰に相当)。
        Continue(usize, usize),
    }

    for start in 0..n {
        if index[start].is_some() {
            continue;
        }
        let mut work: Vec<Frame> = vec![Frame::Enter(start)];
        while let Some(frame) = work.pop() {
            match frame {
                Frame::Enter(v) => {
                    index[v] = Some(next_index);
                    lowlink[v] = next_index;
                    next_index += 1;
                    stack.push(v);
                    on_stack[v] = true;
                    work.push(Frame::Continue(v, 0));
                }
                Frame::Continue(v, next_edge) => {
                    let mut i = next_edge;
                    // 未訪問の子に降りたか
                    let mut recursed = false;
                    while i < adj[v].len() {
                        let w = adj[v][i];
                        i += 1;
                        if index[w].is_none() {
                            work.push(Frame::Continue(v, i));
                            work.push(Frame::Enter(w));
                            recursed = true;
                            break;
                        } else if on_stack[w] {
                            lowlink[v] = lowlink[v].min(index[w].unwrap());
                        }
                    }
                    if recursed {
                        continue;
                    }
                    // `v` の隣接リストを処理し終えた: lowlink を呼び出し元
                    // (`work` の新しい先頭。DFS 木の根でなければ存在する) に伝える。
                    if let Some(Frame::Continue(parent, _)) = work.last() {
                        lowlink[*parent] = lowlink[*parent].min(lowlink[v]);
                    }
                    if lowlink[v] == index[v].unwrap() {
                        // `v` が成分の根: スタックから `v` までを 1 成分として取り出す
                        let mut comp = Vec::new();
                        loop {
                            let w = stack.pop().unwrap();
                            on_stack[w] = false;
                            comp.push(w);
                            if w == v {
                                break;
                            }
                        }
                        components.push(comp);
                    }
                }
            }
        }
    }
    components
}

/// 一般の疎行列 (行 → 非零列の隣接リスト、列数 `n_cols`) の行 `0..adj.len()` を、
/// Dulmage-Mendelsohn 型の分解でブロックに分ける。最大二部マッチング
/// ([`max_bipartite_matching`]) で各行に自分の列を 1 つ対応させ、「行 `i` が行 `j` の
/// マッチ列に非零を持つ」ときに辺 `i -> j` を張った行の有向グラフを強連結成分
/// ([`tarjan_scc`]) に分解する。各成分 (およびマッチしなかった各行の単独ブロック) が
/// 1 ブロックになる。決定性のため、各ブロック内は昇順、ブロックは最小行番号の順に並べる。
///
/// 列を共有しない 2 行は同じ成分に入らないので、単純な連結成分分解より細かい
/// (粗くはならない)。マッチしなかった行は辺の始点にはなれても終点にはならないので、
/// 自動的に単独の成分になる。
///
/// 「独立に処理してよい」の意味は用途による。正方で構造的に正則な行列なら各対角
/// ブロックも正方・構造的正則で、ブロックごとに独立に LU 分解できる。一般の長方形・
/// 階数落ち行列で行ごとの性質 (一次従属性など) を調べる用途での保証は
/// `presolve::redundancy::dulmage_mendelsohn_blocks` の説明を参照。
pub fn dulmage_mendelsohn_blocks(adj: &[Vec<usize>], n_cols: usize) -> Vec<Vec<usize>> {
    let n_rows = adj.len();
    let match_row = max_bipartite_matching(adj, n_cols);
    // 列 → その列をマッチ相手に持つ行
    let match_col_owner: HashMap<usize, usize> =
        match_row.iter().enumerate().filter_map(|(i, c)| c.map(|c| (c, i))).collect();

    // 行グラフの隣接リスト: 行 i → (i が非零を持つ列をマッチ相手に持つ他の行)
    let scc_adj: Vec<Vec<usize>> = adj
        .iter()
        .enumerate()
        .map(|(i, cols)| cols.iter().filter_map(|&j| match_col_owner.get(&j).copied()).filter(|&owner| owner != i).collect())
        .collect();

    let mut components = tarjan_scc(&scc_adj);
    components.sort_by_key(|c| c.iter().copied().min().unwrap_or(n_rows));
    for comp in &mut components {
        comp.sort_unstable();
    }
    components
}

/// [`dulmage_mendelsohn_blocks`] の正方行列・順序付き版。`adj.len() == n_cols` で
/// 完全マッチング (全行が相異なる列にマッチ。構造的に正則な行列、例えば単体法の基底)
/// がある場合に、同じ強連結成分ブロックを**位相順**で返す: マッチング誘導グラフの
/// 各辺 `i -> j` について、`i` を含むブロックが `j` を含むブロックより前に来る。
/// ブロック三角分解では、この順にステップ範囲を割り当てるとブロック外の要素が
/// すべて「ステップ空間で上三角」になる。
///
/// Tarjan 法は到達先の成分を先に完了させるので、生の出力はこの辺の向きに対して
/// 逆位相順になっている。ここではそれを反転して返す。
///
/// 完全マッチングがない (または正方でない) 場合は `None` を返す (呼び出し側は分解なしの
/// 通常の分解にフォールバックし、特異性の判定は数値分解に任せる)。
/// 成功時はブロックと一緒に `match_row` (行 → マッチ列) も返す
/// (各ブロックの部分行列を作るのに呼び出し側が再び必要とするため)。
///
/// 現在は本番コードから使われていない (テスト付きで保持。経緯は
/// `docs/improvement_history.md`)。`ENOMOTO_DEBUG_DM_SPLIT` を設定すると
/// 各段階の所要時間を標準エラーに出す。
#[allow(dead_code)]
pub fn dulmage_mendelsohn_blocks_topological(adj: &[Vec<usize>], n_cols: usize) -> Option<(Vec<Vec<usize>>, Vec<usize>)> {
    let n_rows = adj.len();
    if n_rows != n_cols {
        return None;
    }
    let debug = env_str!("ENOMOTO_DEBUG_DM_SPLIT").is_some();
    let t0 = std::time::Instant::now();
    let match_row = max_bipartite_matching(adj, n_cols);
    let matching_us = t0.elapsed().as_micros();
    if match_row.iter().any(|c| c.is_none()) {
        return None;
    }
    let match_row: Vec<usize> = match_row.into_iter().map(|c| c.unwrap()).collect();
    let t1 = std::time::Instant::now();
    // 列 → その列をマッチ相手に持つ行
    let match_col_owner: HashMap<usize, usize> = match_row.iter().enumerate().map(|(i, &c)| (c, i)).collect();

    // 行グラフの隣接リスト (`dulmage_mendelsohn_blocks` と同じ)
    let scc_adj: Vec<Vec<usize>> = adj
        .iter()
        .enumerate()
        .map(|(i, cols)| cols.iter().filter_map(|&j| match_col_owner.get(&j).copied()).filter(|&owner| owner != i).collect())
        .collect();
    let remap_us = t1.elapsed().as_micros();

    let t2 = std::time::Instant::now();
    let mut components = tarjan_scc(&scc_adj);
    let tarjan_us = t2.elapsed().as_micros();
    components.reverse();
    if debug {
        eprintln!("DM_SPLIT p={n_rows} matching_us={matching_us} remap_us={remap_us} tarjan_us={tarjan_us}");
    }
    Some((components, match_row))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 完全マッチングが存在するグラフで全行がマッチし、列が重複しないこと。
    #[test]
    fn max_bipartite_matching_covers_a_perfect_matching() {
        // 三角形状: row0->{0,1}, row1->{1,2}, row2->{0,1,2}
        let adj = vec![vec![0, 1], vec![1, 2], vec![0, 1, 2]];
        let m = max_bipartite_matching(&adj, 3);
        assert!(m.iter().all(|c| c.is_some()), "expected every row matched: {m:?}");
        let cols: std::collections::HashSet<usize> = m.iter().map(|c| c.unwrap()).collect();
        assert_eq!(cols.len(), 3, "matched columns must be distinct: {m:?}");
    }

    /// 列が足りないとき、余った行はマッチしないこと。
    #[test]
    fn max_bipartite_matching_leaves_excess_rows_unmatched() {
        // 2 行とも列 0 にしか触れない: 1 行しかマッチできない
        let adj = vec![vec![0], vec![0]];
        let m = max_bipartite_matching(&adj, 1);
        let matched = m.iter().filter(|c| c.is_some()).count();
        assert_eq!(matched, 1, "m={m:?}");
    }

    /// 単純な閉路が 1 つの強連結成分になること。
    #[test]
    fn tarjan_scc_finds_a_simple_cycle() {
        // 0 -> 1 -> 2 -> 0 が 1 つの閉路、3 は孤立
        let adj = vec![vec![1], vec![2], vec![0], vec![]];
        let mut comps = tarjan_scc(&adj);
        for c in &mut comps {
            c.sort_unstable();
        }
        comps.sort_by_key(|c| c[0]);
        assert_eq!(comps, vec![vec![0, 1, 2], vec![3]], "comps={comps:?}");
    }

    /// 閉路のない鎖は各ノードが単独の成分になること。
    #[test]
    fn tarjan_scc_splits_a_pure_chain_into_singletons() {
        let adj = vec![vec![1], vec![2], vec![]];
        let mut comps = tarjan_scc(&adj);
        for c in &mut comps {
            c.sort_unstable();
        }
        comps.sort_by_key(|c| c[0]);
        assert_eq!(comps, vec![vec![0], vec![1], vec![2]], "comps={comps:?}");
    }

    /// 列集合が互いに素な行グループがそれぞれ別ブロックになること。
    #[test]
    fn dulmage_mendelsohn_blocks_matches_disjoint_column_groups() {
        // 行 {0,1} は列 {0,1}、行 {2,3} は列 {2,3}、行 4 は列 4 のみ
        let adj = vec![vec![0, 1], vec![0, 1], vec![2, 3], vec![2, 3], vec![4]];
        let comps = dulmage_mendelsohn_blocks(&adj, 5);
        assert_eq!(comps, vec![vec![0, 1], vec![2, 3], vec![4]], "comps={comps:?}");
    }

    /// 閉路のない鎖では、位相順 (始点側が先) にブロックが並ぶこと。
    #[test]
    fn dulmage_mendelsohn_blocks_topological_orders_a_pure_chain_source_first() {
        // 正方 3x3 の上三角: マッチングは row_i -> col_i に決まり、辺 0->1->2 で閉路なし。
        // 各行が単独ブロックで、Tarjan の生の (逆) 順ではなく [0, 1, 2] の順になるはず。
        let adj = vec![vec![0, 1], vec![1, 2], vec![2]];
        let (blocks, match_row) = dulmage_mendelsohn_blocks_topological(&adj, 3).expect("perfect matching expected");
        assert_eq!(blocks, vec![vec![0], vec![1], vec![2]], "blocks={blocks:?}");
        assert_eq!(match_row, vec![0, 1, 2], "match_row={match_row:?}");
    }

    /// 本物の閉路はひとつのブロックにまとまること。
    #[test]
    fn dulmage_mendelsohn_blocks_topological_keeps_a_genuine_cycle_together() {
        // マッチングテストと同じ三角形: どの完全マッチングでも 3 行が 1 つの閉路になる
        let adj = vec![vec![0, 1], vec![1, 2], vec![0, 1, 2]];
        let (blocks, _match_row) = dulmage_mendelsohn_blocks_topological(&adj, 3).expect("perfect matching expected");
        assert_eq!(blocks.len(), 1, "blocks={blocks:?}");
        let mut only = blocks[0].clone();
        only.sort_unstable();
        assert_eq!(only, vec![0, 1, 2], "blocks={blocks:?}");
    }

    /// 完全マッチングがなければ `None` を返すこと。
    #[test]
    fn dulmage_mendelsohn_blocks_topological_none_when_matching_is_imperfect() {
        // 2 行とも列 0 にしか触れない: 完全マッチングなし
        let adj = vec![vec![0], vec![0]];
        assert!(dulmage_mendelsohn_blocks_topological(&adj, 2).is_none());
    }

    /// 正方でなければ `None` を返すこと。
    #[test]
    fn dulmage_mendelsohn_blocks_topological_none_when_not_square() {
        let adj = vec![vec![0], vec![1]];
        assert!(dulmage_mendelsohn_blocks_topological(&adj, 3).is_none());
    }

    /// 再帰先の付け替えが失敗したら、自分の次の候補 (空き列) に進むこと。
    #[test]
    fn max_bipartite_matching_falls_through_a_failed_recursion_to_its_own_free_column() {
        // row0 は col0 のみ。row1 は {col0, col1} で先に col0 を試して row0 へ再帰するが
        // row0 には行き場がなく失敗 → row1 は次の候補 col1 を取るはず。
        let adj = vec![vec![0], vec![0, 1]];
        let m = max_bipartite_matching(&adj, 2);
        assert_eq!(m, vec![Some(0), Some(1)], "m={m:?}");
    }

    /// 再帰先の付け替えが成功したら、既存のマッチを押し出すこと。
    #[test]
    fn max_bipartite_matching_displaces_an_existing_match_on_success() {
        // row0 は {col0, col1} で先に col0 を取る。row1 は col0 のみなので row0 へ再帰し、
        // row0 が col1 に移ることで成功 → row1 が col0、row0 が col1。
        let adj = vec![vec![0, 1], vec![0]];
        let m = max_bipartite_matching(&adj, 2);
        assert_eq!(m, vec![Some(1), Some(0)], "m={m:?}");
    }

    /// 非常に深い (最終的に失敗する) 増加路でもスタックオーバーフローしないこと。
    #[test]
    fn max_bipartite_matching_handles_a_deep_augmenting_chain_without_overflowing_the_stack() {
        // 第 1 段: 行 i (0..N) は {col i, col(i-1)} を自分の列を先にして持つので、
        // 昇順に処理すると各行は再帰なしで col i を取る (全体 O(N))。
        // 第 2 段: 最後に全列を降順に持つ探査行を追加する。空き列がないので
        // row(N-1) → row(N-2) → ... → row0 と深さ N の探索をして最終的に失敗する。
        // 第 1 段の行はすべて自分の列を保持し、探査行はマッチしないはず。
        // 第 1 段の行数 (= 探査の深さ)
        const N: usize = 100_000;
        let mut adj: Vec<Vec<usize>> = vec![vec![0]];
        adj.extend((1..N).map(|i| vec![i, i - 1]));
        adj.push((0..N).rev().collect());
        let m = max_bipartite_matching(&adj, N);
        assert_eq!(&m[..N], &(0..N).map(Some).collect::<Vec<_>>()[..], "expected every phase-1 row to keep its own column");
        assert_eq!(m[N], None, "expected the probe row to end up unmatched");
    }

    /// テスト用の小さな決定的乱数生成器 (xorshift64*)。`rand` に依存せず、
    /// 失敗時にシードから再現できるようにするため。
    struct XorShift64(u64);
    impl XorShift64 {
        /// 次の 64 ビット乱数。
        fn next_u64(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        /// `0..n` の一様乱数 (近似)。
        fn next_range(&mut self, n: usize) -> usize {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// [`max_bipartite_matching`] の再帰版 (反復版の参照実装)。小さなグラフでしか
    /// 使わないので再帰でも安全。
    fn max_bipartite_matching_recursive_reference(adj: &[Vec<usize>], n_cols: usize) -> Vec<Option<usize>> {
        let n_rows = adj.len();
        let mut match_col: Vec<Option<usize>> = vec![None; n_cols];
        /// 行 `row` から増加路を再帰的に探す。
        fn try_augment(row: usize, adj: &[Vec<usize>], visited: &mut [bool], match_col: &mut [Option<usize>]) -> bool {
            for &col in &adj[row] {
                if visited[col] {
                    continue;
                }
                visited[col] = true;
                if match_col[col].is_none_or(|r| try_augment(r, adj, visited, match_col)) {
                    match_col[col] = Some(row);
                    return true;
                }
            }
            false
        }
        for i in 0..n_rows {
            let mut visited = vec![false; n_cols];
            try_augment(i, adj, &mut visited, &mut match_col);
        }
        let mut match_row: Vec<Option<usize>> = vec![None; n_rows];
        for (col, row) in match_col.into_iter().enumerate() {
            if let Some(r) = row {
                match_row[r] = Some(col);
            }
        }
        match_row
    }

    /// 多数の小さなランダム二部グラフで、反復版が再帰版と完全に同じ `match_row` を返すこと
    /// (同じ行順・同じ候補順で同じ判断をするので、サイズだけでなく中身まで一致するはず)。
    #[test]
    fn max_bipartite_matching_matches_the_recursive_reference_on_many_random_graphs() {
        let mut rng = XorShift64(0x9E3779B97F4A7C15);
        for _ in 0..500 {
            let p = 1 + rng.next_range(10);
            let n_cols = 1 + rng.next_range(10);
            let adj: Vec<Vec<usize>> = (0..p)
                .map(|_| {
                    let mut row: Vec<usize> = (0..n_cols).filter(|_| rng.next_range(3) == 0).collect();
                    // 候補順が常に昇順にならないようシャッフルする
                    for k in (1..row.len()).rev() {
                        let j = rng.next_range(k + 1);
                        row.swap(k, j);
                    }
                    row
                })
                .collect();
            let got = max_bipartite_matching(&adj, n_cols);
            let want = max_bipartite_matching_recursive_reference(&adj, n_cols);
            assert_eq!(got, want, "adj={adj:?} n_cols={n_cols}");
        }
    }
}
