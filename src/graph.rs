//! Small, reusable graph algorithms shared across presolve
//! ([`crate::presolve::redundancy`]'s redundant-row block decomposition)
//! and the basis LU factorization ([`crate::simplex::lu`]): maximum
//! bipartite matching (Kuhn's algorithm) and strongly connected
//! components (iterative Tarjan), combined into a Dulmage-Mendelsohn-style
//! block decomposition of a general sparse matrix given as row-major
//! adjacency lists.
//!
//! Kept generic over just `adj: &[Vec<usize>]` (row index -> the column
//! indices it has a nonzero in) rather than either caller's own richer row
//! representation, so it carries no assumption about values, an augmented
//! right-hand side, or any other caller-specific structure.

use std::collections::HashMap;

/// Finds a maximum matching between rows `0..adj.len()` and columns
/// `0..n_cols` of the bipartite graph defined by `adj` — Kuhn's algorithm
/// (repeated augmenting-path search, `O(rows * nnz)` worst case, typically
/// far faster in practice on real sparse graphs): for each row in turn, a
/// DFS over its columns looks for either a free column or one whose
/// current match can itself be re-routed to a different column, freeing
/// this one up.
///
/// Returns `match_row[i] = Some(column)` for each matched row `i`, `None`
/// for rows the matching couldn't cover (more rows than the matching can
/// place, or a genuinely rank-deficient structural pattern) — see
/// [`dulmage_mendelsohn_blocks`]'s own docs for how unmatched rows are
/// still handled soundly despite having no designated column.
pub fn max_bipartite_matching(adj: &[Vec<usize>], n_cols: usize) -> Vec<Option<usize>> {
    let p = adj.len();
    let mut match_col: Vec<Option<usize>> = vec![None; n_cols];

    // Iterative augmenting-path DFS — an explicit stack instead of real
    // recursion, for exactly the reason [`tarjan_scc`]'s own docs give for
    // doing the same thing to Tarjan's algorithm just below: an augmenting
    // path's length is bounded only by `min(p, n_cols)`, which can run
    // into the thousands on this crate's own problem sizes, and a
    // genuinely long one previously *did* overflow the call stack on a
    // production Netlib instance (`degen3`) once a presolve change shifted
    // which rows got fed to this matcher in what order — this crate's own
    // project history has the full incident.
    //
    // Mirrors the recursive statement exactly: `for col in adj[row]` (skip
    // visited, mark visited, try the free-or-recursible case, `return true`
    // the moment one works) becomes a per-frame `idx` cursor so re-entering
    // a row already on the stack resumes its own loop instead of
    // restarting it. The one property worth spelling out because it's easy
    // to get wrong in this exact shape: *every* frame along a successful
    // path — not just the one that found a literally free column —
    // performs the identical final step `match_col[its_own_col] =
    // its_own_row` once its own attempt succeeds, whether that success
    // came from the column being free outright or from a deeper recursive
    // call re-placing whoever held it (the recursive code's `if
    // match_col[col].is_none_or(|r| try_augment(r, ..)) { match_col[col] =
    // Some(row); return true; }` runs that same assignment either way, and
    // both branches of `match match_col[col]` below reach it too — the
    // `None` branch immediately, the `Some(r2)` branch only after the
    // frame it pushes for `r2` eventually says so). So a single success
    // unwinds the *entire* active path in one pass: no separate "did my
    // child succeed?" branch is needed on the way back down, just walking
    // `stack`/`col_stack` off in lockstep and assigning each level.
    struct Frame {
        row: usize,
        idx: usize,
    }
    // `visited[col] == stamp` marks `col` as visited by the current
    // augmentation (one fresh `stamp` per starting row) — the same "reset
    // before every row" semantics as the fresh `vec![false; n_cols]` per
    // row this used to allocate, which cost `O(p * n_cols)` zeroing in
    // total (`maros-r7`: 3135 rows x 9406 columns). The DFS stacks are
    // likewise reused across rows instead of reallocated.
    fn try_augment(
        start: usize,
        adj: &[Vec<usize>],
        visited: &mut [u32],
        stamp: u32,
        match_col: &mut [Option<usize>],
        stack: &mut Vec<Frame>,
        col_stack: &mut Vec<usize>,
    ) -> bool {
        stack.clear();
        col_stack.clear();
        stack.push(Frame { row: start, idx: 0 });
        // `col_stack[k]` is the column `stack[k]` was trying when it
        // decided to push `stack[k+1]` — i.e. `match_col[col_stack[k]] ==
        // Some(stack[k+1].row)` at that moment. Always exactly one
        // shorter than `stack` itself (the bottom frame wasn't pushed to
        // satisfy any column of its own).

        loop {
            let top = stack.len() - 1;
            let row = stack[top].row;
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
                // `row`'s own neighbor list is exhausted with no success —
                // the recursive equivalent of falling off the end of the
                // `for` loop and returning `false`.
                stack.pop();
                let Some(_) = stack.last() else { return false };
                col_stack.pop();
                continue;
            };

            match match_col[col] {
                Some(r2) => {
                    // Occupied: recurse into whoever currently holds `col`.
                    stack.push(Frame { row: r2, idx: 0 });
                    col_stack.push(col);
                }
                None => {
                    // Free column: an immediate success that unwinds the
                    // whole active path in one go (see this function's own
                    // docs above for why every level performs the same
                    // assignment, not just this innermost one).
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

    let mut visited = vec![0u32; n_cols];
    let mut stack: Vec<Frame> = Vec::new();
    let mut col_stack: Vec<usize> = Vec::new();
    for i in 0..p {
        try_augment(i, adj, &mut visited, i as u32 + 1, &mut match_col, &mut stack, &mut col_stack);
    }

    let mut match_row: Vec<Option<usize>> = vec![None; p];
    for (col, row) in match_col.into_iter().enumerate() {
        if let Some(r) = row {
            match_row[r] = Some(col);
        }
    }
    match_row
}

/// Iterative Tarjan's strongly-connected-components algorithm (recursive
/// would risk stack overflow on a single long dependency chain — a real
/// possibility on this crate's problem sizes, where chain length is
/// bounded only by the number of nodes, which can run into the
/// thousands). `adj[i]` lists the directed out-edges from node `i`.
/// Returns each SCC as a `Vec<usize>` of node indices, in no particular
/// order between components (callers needing determinism sort afterward,
/// as [`dulmage_mendelsohn_blocks`] does).
pub fn tarjan_scc(adj: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = adj.len();
    let mut index: Vec<Option<u32>> = vec![None; n];
    let mut lowlink: Vec<u32> = vec![0; n];
    let mut on_stack: Vec<bool> = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut next_index: u32 = 0;
    let mut components: Vec<Vec<usize>> = Vec::new();

    // Explicit work-stack frame: which node, and how far through its
    // adjacency list we've already processed (so re-entering after a
    // recursive-equivalent call resumes rather than restarting the loop).
    enum Frame {
        Enter(usize),
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
                    // Finished `v`'s adjacency list: propagate its lowlink
                    // up to whichever node called into it (the new top of
                    // `work`, if this wasn't the root of this DFS tree).
                    if let Some(Frame::Continue(parent, _)) = work.last() {
                        lowlink[*parent] = lowlink[*parent].min(lowlink[v]);
                    }
                    if lowlink[v] == index[v].unwrap() {
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

/// Partitions the rows `0..adj.len()` of a general sparse matrix (given as
/// row -> nonzero-column adjacency, `n_cols` columns total) into blocks
/// via a Dulmage-Mendelsohn-style decomposition: a maximum bipartite
/// matching ([`max_bipartite_matching`]) pairs each (coverable) row with
/// one of its own columns, then a directed graph on rows — edge `i -> j`
/// when row `i` has a nonzero in row `j`'s matched column — is decomposed
/// into strongly connected components ([`tarjan_scc`]). Each SCC (plus
/// each unmatched row, on its own) becomes one block, returned sorted by
/// its smallest row index (each block's own rows sorted ascending too) for
/// determinism, since union-find/DFS-derived groupings are otherwise
/// traversal-order-dependent.
///
/// Strictly finer than a plain connected-components partition of the same
/// bipartite graph would be: two rows sharing no column at all can never
/// end up in the same SCC either, since the matching graph's edges are
/// themselves derived from real nonzeros — so this never *loses* the
/// block-diagonal structure a simpler decomposition would already find.
///
/// Unmatched rows need no special-casing to land in a sound singleton
/// block: lacking a column of their own, they can still be the *source*
/// of a graph edge (if one of their columns happens to be some other
/// row's match) but never a *target*, so nothing can complete a cycle
/// through them — Tarjan's algorithm places each in a trivial singleton
/// SCC automatically.
///
/// **What "safe to process independently" means depends on the caller.**
/// For a matrix known to be square and structurally nonsingular (a valid
/// LU factorization target), each diagonal block from this decomposition
/// is *itself* square and structurally nonsingular, and — because a block
/// upper-triangular matrix's off-diagonal spillover entries are carried
/// into the factored form completely unchanged, never touched by any
/// pivot outside their own row's block — every block can be **factored**
/// independently too, not just checked, with no value propagation between
/// blocks required (unlike using this decomposition for *solving*
/// `Lx = b`, which genuinely does need triangular order, since back- and
/// forward-substitution propagate computed values through those same
/// spillover entries). For a general rectangular/rank-deficient matrix
/// used only to *test* per-row properties (e.g. linear-dependence
/// detection), a weaker but still useful guarantee holds: see
/// `presolve::redundancy::dulmage_mendelsohn_blocks`'s own docs for that
/// argument specifically.
pub fn dulmage_mendelsohn_blocks(adj: &[Vec<usize>], n_cols: usize) -> Vec<Vec<usize>> {
    let p = adj.len();
    let match_row = max_bipartite_matching(adj, n_cols);
    // Column -> matched row as a flat array: looked up once per nonzero.
    let mut match_col_owner: Vec<Option<usize>> = vec![None; n_cols];
    for (i, c) in match_row.iter().enumerate() {
        if let Some(c) = *c {
            match_col_owner[c] = Some(i);
        }
    }

    let scc_adj: Vec<Vec<usize>> = adj
        .iter()
        .enumerate()
        .map(|(i, cols)| cols.iter().filter_map(|&j| match_col_owner[j]).filter(|&owner| owner != i).collect())
        .collect();

    let mut components = tarjan_scc(&scc_adj);
    components.sort_by_key(|c| c.iter().copied().min().unwrap_or(p));
    for comp in &mut components {
        comp.sort_unstable();
    }
    components
}

/// The square-matrix, order-sensitive counterpart to
/// [`dulmage_mendelsohn_blocks`]: for a matrix known to have `adj.len() ==
/// n_cols` (square) and a *perfect* matching (every row matched to a
/// distinct column — the generic case for a structurally nonsingular
/// matrix, e.g. a valid simplex basis), returns the same SCC blocks but in
/// a genuine **topological order** — for every edge `i -> j` in the
/// matching-induced graph (row `i` has a nonzero in row `j`'s matched
/// column), the block containing `i` comes *before* the block containing
/// `j`. That ordering is exactly what a block-triangular *factorization*
/// (as opposed to [`dulmage_mendelsohn_blocks`]'s order-independent
/// per-row *checking*) needs: assigning increasing global step ranges to
/// blocks in this order makes every spillover entry land at a column-step
/// greater than or equal to its own row-step, satisfying the same
/// "upper triangular in step-space" invariant a plain, undecomposed
/// factorization already produces.
///
/// Returns `None` (signalling "fall back to an undecomposed
/// factorization") when the matching isn't perfect — some row couldn't be
/// matched to its own column, which for a square input means the matrix
/// is not structurally nonsingular (or `adj.len() != n_cols`) and this
/// decomposition's whole premise (square, structurally-independent
/// diagonal blocks) doesn't apply; the caller's own numerical
/// factorization is what correctly detects and reports genuine
/// singularity in that case, not this purely structural pre-pass.
///
/// Also returns the underlying `match_row` (row -> its own matched
/// column) alongside the blocks, since a factorization caller needs it
/// again anyway (to know which columns belong to which block when
/// building each block's local sub-matrix).
///
/// **Why reversing Tarjan's own output order is the correct topological
/// order, not an arbitrary choice**: Tarjan's algorithm completes
/// (pops) a strongly connected component only after every node it can
/// reach has already been fully explored, so a "source" component (with
/// edges leading to others) is necessarily completed *after* the
/// components it points to — i.e. raw Tarjan output is already in
/// *reverse* topological order for this edge convention. Reversing it
/// once here, rather than asking every caller to remember to, is the
/// only change from [`dulmage_mendelsohn_blocks`]'s own version (which
/// instead sorts by minimum row index purely for reproducible test
/// output, since order doesn't matter for its own use case).
///
/// **Currently unused in production** (kept, with tests, for a possible
/// future revisit): `simplex::lu::factorize` was extended to use this for
/// a block-triangularized basis-matrix factorization, validated for
/// correctness, then reverted after measuring a net ~4% aggregate
/// regression on the Netlib benchmark — see that function's own doc
/// comment for the full write-up (wins on a few block-angular instances
/// outweighed by a broad tax elsewhere from paying bipartite-matching
/// cost on every refactorization, mirroring why HiGHS itself only does
/// cheap degree-1 peeling here, not full matching-based decomposition).
#[allow(dead_code)]
pub fn dulmage_mendelsohn_blocks_topological(adj: &[Vec<usize>], n_cols: usize) -> Option<(Vec<Vec<usize>>, Vec<usize>)> {
    let p = adj.len();
    if p != n_cols {
        return None;
    }
    let debug = std::env::var("ENOMOTO_DEBUG_DM_SPLIT").is_ok();
    let t0 = std::time::Instant::now();
    let match_row = max_bipartite_matching(adj, n_cols);
    let matching_us = t0.elapsed().as_micros();
    if match_row.iter().any(|c| c.is_none()) {
        return None;
    }
    let match_row: Vec<usize> = match_row.into_iter().map(|c| c.unwrap()).collect();
    let t1 = std::time::Instant::now();
    let match_col_owner: HashMap<usize, usize> = match_row.iter().enumerate().map(|(i, &c)| (c, i)).collect();

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
        eprintln!("DM_SPLIT p={p} matching_us={matching_us} remap_us={remap_us} tarjan_us={tarjan_us}");
    }
    Some((components, match_row))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_bipartite_matching_covers_a_perfect_matching() {
        // Triangle-shaped: row0->{0,1}, row1->{1,2}, row2->{0,1,2}.
        let adj = vec![vec![0, 1], vec![1, 2], vec![0, 1, 2]];
        let m = max_bipartite_matching(&adj, 3);
        assert!(m.iter().all(|c| c.is_some()), "expected every row matched: {m:?}");
        let cols: std::collections::HashSet<usize> = m.iter().map(|c| c.unwrap()).collect();
        assert_eq!(cols.len(), 3, "matched columns must be distinct: {m:?}");
    }

    #[test]
    fn max_bipartite_matching_leaves_excess_rows_unmatched() {
        // Two rows both only touching column 0 -- only one can be matched.
        let adj = vec![vec![0], vec![0]];
        let m = max_bipartite_matching(&adj, 1);
        let matched = m.iter().filter(|c| c.is_some()).count();
        assert_eq!(matched, 1, "m={m:?}");
    }

    #[test]
    fn tarjan_scc_finds_a_simple_cycle() {
        // 0 -> 1 -> 2 -> 0 is one cycle; 3 is isolated.
        let adj = vec![vec![1], vec![2], vec![0], vec![]];
        let mut comps = tarjan_scc(&adj);
        for c in &mut comps {
            c.sort_unstable();
        }
        comps.sort_by_key(|c| c[0]);
        assert_eq!(comps, vec![vec![0, 1, 2], vec![3]], "comps={comps:?}");
    }

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

    #[test]
    fn dulmage_mendelsohn_blocks_matches_disjoint_column_groups() {
        // rows {0,1} over cols {0,1} (mutually referencing), rows {2,3}
        // over cols {2,3} (same), row 4 isolated over col 4.
        let adj = vec![vec![0, 1], vec![0, 1], vec![2, 3], vec![2, 3], vec![4]];
        let comps = dulmage_mendelsohn_blocks(&adj, 5);
        assert_eq!(comps, vec![vec![0, 1], vec![2, 3], vec![4]], "comps={comps:?}");
    }

    #[test]
    fn dulmage_mendelsohn_blocks_topological_orders_a_pure_chain_source_first() {
        // Square 3x3, upper-triangular-by-construction: row0 touches
        // {0,1}, row1 touches {1,2}, row2 touches only {2} -- forces the
        // matching row0->0, row1->1, row2->2, giving edges 0->1->2 with no
        // cycle, so each row is its own block and must come out in the
        // order [0, 1, 2] (source before target), not Tarjan's own raw
        // (reverse) completion order.
        let adj = vec![vec![0, 1], vec![1, 2], vec![2]];
        let (blocks, match_row) = dulmage_mendelsohn_blocks_topological(&adj, 3).expect("perfect matching expected");
        assert_eq!(blocks, vec![vec![0], vec![1], vec![2]], "blocks={blocks:?}");
        assert_eq!(match_row, vec![0, 1, 2], "match_row={match_row:?}");
    }

    #[test]
    fn dulmage_mendelsohn_blocks_topological_keeps_a_genuine_cycle_together() {
        // Same triangle as the matching test above: a perfect matching
        // exists, and the induced graph among matched rows forms one
        // 3-cycle regardless of which valid matching is chosen, so all 3
        // rows must land in a single block.
        let adj = vec![vec![0, 1], vec![1, 2], vec![0, 1, 2]];
        let (blocks, _match_row) = dulmage_mendelsohn_blocks_topological(&adj, 3).expect("perfect matching expected");
        assert_eq!(blocks.len(), 1, "blocks={blocks:?}");
        let mut only = blocks[0].clone();
        only.sort_unstable();
        assert_eq!(only, vec![0, 1, 2], "blocks={blocks:?}");
    }

    #[test]
    fn dulmage_mendelsohn_blocks_topological_none_when_matching_is_imperfect() {
        // Two rows, both only touching column 0 -- no perfect matching.
        let adj = vec![vec![0], vec![0]];
        assert!(dulmage_mendelsohn_blocks_topological(&adj, 2).is_none());
    }

    #[test]
    fn dulmage_mendelsohn_blocks_topological_none_when_not_square() {
        let adj = vec![vec![0], vec![1]];
        assert!(dulmage_mendelsohn_blocks_topological(&adj, 3).is_none());
    }

    #[test]
    fn max_bipartite_matching_falls_through_a_failed_recursion_to_its_own_free_column() {
        // row0 only reaches col0 (no alternative once displaced); row1
        // reaches {col0, col1}, tries col0 first, recurses into row0, and
        // that recursion fails outright (row0 truly has nowhere else to
        // go) — row1 must then fall through to its own next candidate
        // (col1) rather than come back empty despite col1 being reachable.
        let adj = vec![vec![0], vec![0, 1]];
        let m = max_bipartite_matching(&adj, 2);
        assert_eq!(m, vec![Some(0), Some(1)], "m={m:?}");
    }

    #[test]
    fn max_bipartite_matching_displaces_an_existing_match_on_success() {
        // row0 reaches {col0, col1} and grabs col0 first (its own free
        // choice); row1 reaches only col0, forcing a recursion into row0
        // that this time *succeeds* (row0 relocates to its own remaining
        // col1) — row1 ends up with col0, row0 with col1: a genuine
        // one-level displacement, hand-verified here as a companion to the
        // failed-recursion case above and the randomized cross-check and
        // deep-chain tests below (which exercise longer versions of both
        // without a hand-computable expected answer).
        let adj = vec![vec![0, 1], vec![0]];
        let m = max_bipartite_matching(&adj, 2);
        assert_eq!(m, vec![Some(1), Some(0)], "m={m:?}");
    }

    #[test]
    fn max_bipartite_matching_handles_a_deep_augmenting_chain_without_overflowing_the_stack() {
        // Built in two phases so the whole test stays `O(N)` rather than
        // `O(N^2)` (an earlier version of this test listed each row's
        // *previous*-column preference first, which forces recursion
        // during every single row's own initial placement below — still
        // correct, but O(N) work apiece made the whole test O(N^2) and
        // far too slow to run routinely; the swapped preference order
        // here keeps every one of the first `N` rows' own placements
        // `O(1)`, so the only genuinely deep call is the one probe row
        // added afterward):
        //
        // Phase 1: row `i` (i in 0..N) reaches {col i, col(i-1)} — its own
        // column *first* — so processing rows in ascending order always
        // finds col `i` free immediately (no row before it could ever
        // want it) and every one of these `N` placements is `O(1)`, no
        // recursion at all: `row_i` ends up matched to `col_i`, but the
        // link to `col(i-1)` is still there, latent, in row `i`'s own
        // adjacency for phase 2 to walk.
        //
        // Phase 2: one more probe row reaches every column `N-1, N-2,
        // ..., 0` in descending order (no genuinely free column left for
        // it — `n_cols == N`, all already taken by phase 1). Its own
        // search must recurse into `row(N-1)` (which holds col `N-1`),
        // which — via its own latent `col(N-2)` link — recurses into
        // `row(N-2)`, and so on all the way down to `row0`, which finally
        // has nowhere left to go and fails, unwinding the whole probe as
        // one `O(N)`-deep, ultimately-unsuccessful call. That single call
        // is the exact shape (long, ultimately-failing recursive descent)
        // that overflowed the call stack with a genuinely recursive DFS
        // on a large production instance (`degen3`; see this function's
        // own docs) — `N = 1_000_000` here comfortably exceeds anything a
        // 1-2MB call stack could survive recursively at any plausible
        // per-frame size, so simply not hanging or crashing is itself
        // most of the test; the assertions below also confirm every
        // phase-1 row *kept* its own column (the failed probe must leave
        // every existing match untouched) and that the probe row itself
        // comes back unmatched, exactly as the old recursive version
        // would also have concluded (just by overflowing its own stack
        // before it could).
        //
        // `N = 100_000` rather than something larger: `max_bipartite_matching`'s
        // own per-row `vec![false; n_cols]` (one fresh visited buffer per
        // top-level call, `n_cols` long) makes the *whole test* — not
        // just the probe — `O(N^2)` regardless of how cheap each
        // individual row's own search is, so this is chosen as the
        // smallest value that still leaves no realistic doubt (100,000
        // call frames is far beyond what any plausible thread stack, even
        // a generously-sized one, could hold) without costing more than a
        // fraction of a second here.
        const N: usize = 100_000;
        let mut adj: Vec<Vec<usize>> = vec![vec![0]];
        adj.extend((1..N).map(|i| vec![i, i - 1]));
        adj.push((0..N).rev().collect());
        let m = max_bipartite_matching(&adj, N);
        assert_eq!(&m[..N], &(0..N).map(Some).collect::<Vec<_>>()[..], "expected every phase-1 row to keep its own column");
        assert_eq!(m[N], None, "expected the probe row to end up unmatched");
    }

    /// Small deterministic PRNG (xorshift64*) — no `rand` dependency in
    /// this crate, and a fixed, seedable generator keeps a failing
    /// property-test run reproducible from its own seed rather than from
    /// whatever `std`'s own unspecified default source would give.
    struct XorShift64(u64);
    impl XorShift64 {
        fn next_u64(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn next_range(&mut self, n: usize) -> usize {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// The original (pre-iterative-rewrite) recursive statement of
    /// [`max_bipartite_matching`]'s own augmenting-path DFS, kept only
    /// here as a from-first-principles reference for the property test
    /// below — safe to leave genuinely recursive since every call in this
    /// test module uses small graphs (bounded rows/cols), nowhere near
    /// deep enough to risk the stack overflow the real function was
    /// rewritten to avoid.
    fn max_bipartite_matching_recursive_reference(adj: &[Vec<usize>], n_cols: usize) -> Vec<Option<usize>> {
        let p = adj.len();
        let mut match_col: Vec<Option<usize>> = vec![None; n_cols];
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
        for i in 0..p {
            let mut visited = vec![false; n_cols];
            try_augment(i, adj, &mut visited, &mut match_col);
        }
        let mut match_row: Vec<Option<usize>> = vec![None; p];
        for (col, row) in match_col.into_iter().enumerate() {
            if let Some(r) = row {
                match_row[r] = Some(col);
            }
        }
        match_row
    }

    #[test]
    fn max_bipartite_matching_matches_the_recursive_reference_on_many_random_graphs() {
        // Cross-checks the iterative rewrite against the recursive
        // statement it replaces, across many small random bipartite
        // graphs (rows/cols small enough that the reference's own
        // recursion is safe — see its own docs). Both are Kuhn's
        // algorithm processing rows in the same `0..p` order with the
        // same per-row preference order, so they don't just find matchings
        // of the same *size* (true of any two correct maximum-matching
        // algorithms, tie-breaking or not) — they make the identical
        // sequence of greedy/augment decisions and so must produce the
        // exact same `match_row`, not merely an equally-sized one.
        let mut rng = XorShift64(0x9E3779B97F4A7C15);
        for _ in 0..500 {
            let p = 1 + rng.next_range(10);
            let n_cols = 1 + rng.next_range(10);
            let adj: Vec<Vec<usize>> = (0..p)
                .map(|_| {
                    let mut row: Vec<usize> = (0..n_cols).filter(|_| rng.next_range(3) == 0).collect();
                    // Shuffle so preference order isn't always ascending —
                    // exercises the "first free/recursible candidate isn't
                    // just the lowest index" path in both implementations.
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
