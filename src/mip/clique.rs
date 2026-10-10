//! clique カット: 2 値列の衝突グラフ (両方を 1 にすると (否定を含む) 行を必ず破る組) を元の行から作り、LP 解で
//! 重みの和が 1 を超える clique を貪欲に探して `clique の中で 1 になれるのは高々 1 つ` のカットにする。
//! 文字 (literal) は `2j` が `x_j`、`2j + 1` が `1 - x_j`。

use super::cuts::RawCut;
use std::collections::HashSet;

/// 衝突グラフ (文字ごとに隣接する文字、昇順)。
pub struct CliqueGraph {
    adj: Vec<Vec<u32>>,
}

/// 1 行から衝突の組を作るときの文字数の上限 (これより長い行は使わない)。
const MAX_ROW_LITS: usize = 2000;
/// 1 行から作る辺の数の上限。
const MAX_ROW_EDGES: usize = 20_000;

impl CliqueGraph {
    /// 行 `row_lo <= sum a_j x_j <= row_up` から作る。`binary[j]` は 2 値列か、`lo`/`up` は列の境界 (大域的)。
    pub fn build(n: usize, rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64], binary: &[bool], lo: &[f64], up: &[f64]) -> Self {
        Self::build_with(n, rows, row_lo, row_up, binary, lo, up, &[])
    }

    /// [`Self::build`] に、行以外から分かった衝突の組 (`extra`、文字の組) を加える。
    #[allow(clippy::too_many_arguments)]
    pub fn build_with(n: usize, rows: &[Vec<(usize, f64)>], row_lo: &[f64], row_up: &[f64], binary: &[bool], lo: &[f64], up: &[f64], extra: &[(u32, u32)]) -> Self {
        let mut adj: Vec<HashSet<u32>> = vec![HashSet::new(); 2 * n];
        for &(a, b) in extra {
            let (ja, jb) = ((a / 2) as usize, (b / 2) as usize);
            if ja == jb || ja >= n || jb >= n || !binary[ja] || !binary[jb] || lo[ja] == up[ja] || lo[jb] == up[jb] {
                continue;
            }
            adj[a as usize].insert(b);
            adj[b as usize].insert(a);
        }
        for (i, r) in rows.iter().enumerate() {
            // `sum a x <= b` の形 (>= は符号を反転) ごとに
            for (sign, b) in [(1.0, row_up[i]), (-1.0, -row_lo[i])] {
                if !b.is_finite() {
                    continue;
                }
                let mut minact = 0.0;
                let mut lits: Vec<(f64, u32)> = Vec::new(); // (文字を真にしたときの増分, 文字)
                let mut ok = true;
                for &(j, a0) in r {
                    let a = sign * a0;
                    let m = if a > 0.0 { a * lo[j] } else { a * up[j] };
                    if !m.is_finite() {
                        ok = false;
                        break;
                    }
                    minact += m;
                    if binary[j] && lo[j] < up[j] {
                        if a > 0.0 {
                            lits.push((a, 2 * j as u32));
                        } else if a < 0.0 {
                            lits.push((-a, 2 * j as u32 + 1));
                        }
                    }
                }
                if !ok || lits.len() < 2 || lits.len() > MAX_ROW_LITS {
                    continue;
                }
                let slack = b - minact;
                let tol = 1e-9 * (1.0 + b.abs());
                lits.sort_by(|a, b| b.0.total_cmp(&a.0));
                // 増分の大きい順: 先頭 2 つでも行を破らなければ組はない
                if lits[0].0 + lits[1].0 <= slack + tol {
                    continue;
                }
                let mut edges = 0usize;
                'outer: for u in 0..lits.len() {
                    for v in (u + 1)..lits.len() {
                        if lits[u].0 + lits[v].0 <= slack + tol {
                            break;
                        }
                        if lits[u].1 / 2 == lits[v].1 / 2 {
                            continue;
                        }
                        adj[lits[u].1 as usize].insert(lits[v].1);
                        adj[lits[v].1 as usize].insert(lits[u].1);
                        edges += 1;
                        if edges >= MAX_ROW_EDGES {
                            break 'outer;
                        }
                    }
                }
            }
        }
        let adj = adj
            .into_iter()
            .map(|s| {
                let mut v: Vec<u32> = s.into_iter().collect();
                v.sort_unstable();
                v
            })
            .collect();
        CliqueGraph { adj }
    }

    /// 辺があるか。
    pub fn num_edges(&self) -> usize {
        self.adj.iter().map(|a| a.len()).sum::<usize>() / 2
    }

    fn adjacent(&self, u: u32, v: u32) -> bool {
        self.adj[u as usize].binary_search(&v).is_ok()
    }

    /// LP 解 `x` で破れる clique カットを最大 `max_cuts` 本探す。
    pub fn separate(&self, x: &[f64], max_cuts: usize) -> Vec<RawCut> {
        let n = x.len();
        let val = |l: u32| -> f64 {
            let j = (l / 2) as usize;
            if l % 2 == 0 { x[j] } else { 1.0 - x[j] }
        };
        let mut starts: Vec<u32> = (0..2 * n as u32).filter(|&l| !self.adj[l as usize].is_empty() && val(l) > 1e-6 && val(l) < 1.0 - 1e-6).collect();
        starts.sort_by(|&a, &b| val(b).total_cmp(&val(a)));
        let mut cuts: Vec<RawCut> = Vec::new();
        let mut seen: HashSet<Vec<u32>> = HashSet::new();
        for &u in starts.iter().take(500) {
            if cuts.len() >= max_cuts {
                break;
            }
            // 貪欲: 隣接する文字を値の大きい順に、clique の全員と隣接するものだけ加える
            let mut cand: Vec<u32> = self.adj[u as usize].iter().copied().filter(|&v| val(v) > 1e-9).collect();
            cand.sort_by(|&a, &b| val(b).total_cmp(&val(a)));
            let mut clique = vec![u];
            let mut w = val(u);
            for v in cand {
                if clique.iter().all(|&c| self.adjacent(c, v)) {
                    clique.push(v);
                    w += val(v);
                }
            }
            if w <= 1.0 + 1e-4 {
                continue;
            }
            // 値 0 の文字も全員と隣接すれば加える (カットが強くなる、少しだけ)
            for &v in self.adj[u as usize].iter().take(200) {
                if val(v) <= 1e-9 && !clique.contains(&v) && clique.iter().all(|&c| self.adjacent(c, v)) {
                    clique.push(v);
                }
            }
            let mut key = clique.clone();
            key.sort_unstable();
            if !seen.insert(key) {
                continue;
            }
            // sum_{x 文字} x_j + sum_{否定} (1 - x_j) <= 1
            let mut coefs: Vec<(usize, f64)> = Vec::with_capacity(clique.len());
            let mut neg = 0.0;
            for &l in &clique {
                let j = (l / 2) as usize;
                if l % 2 == 0 {
                    coefs.push((j, 1.0));
                } else {
                    coefs.push((j, -1.0));
                    neg += 1.0;
                }
            }
            coefs.sort_by_key(|&(j, _)| j);
            cuts.push(RawCut { coefs, rhs: 1.0 - neg });
        }
        cuts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle_clique_cut() {
        // x0 + x1 <= 1, x1 + x2 <= 1, x0 + x2 <= 1 で x = (0.5, 0.5, 0.5) は x0 + x1 + x2 <= 1 で切れる
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(1, 1.0), (2, 1.0)], vec![(0, 1.0), (2, 1.0)]];
        let g = CliqueGraph::build(3, &rows, &[f64::NEG_INFINITY; 3], &[1.0; 3], &[true; 3], &[0.0; 3], &[1.0; 3]);
        assert_eq!(g.num_edges(), 3);
        let cuts = g.separate(&[0.5, 0.5, 0.5], 10);
        assert_eq!(cuts.len(), 1);
        assert_eq!(cuts[0].coefs.len(), 3);
        assert!((cuts[0].rhs - 1.0).abs() < 1e-12);
    }

    #[test]
    fn complemented_literals() {
        // x0 - x1 <= 0 (x0 = 1 なら x1 = 1): 文字 x0 と not x1 が衝突。x = (0.8, 0.3) は x0 + (1 - x1) <= 1 で切れる
        let rows = vec![vec![(0, 1.0), (1, -1.0)]];
        let g = CliqueGraph::build(2, &rows, &[f64::NEG_INFINITY], &[0.0], &[true; 2], &[0.0; 2], &[1.0; 2]);
        let cuts = g.separate(&[0.8, 0.3], 10);
        assert_eq!(cuts.len(), 1);
        // x0 - x1 <= 0
        let mut c = cuts[0].coefs.clone();
        c.sort_by_key(|&(j, _)| j);
        assert_eq!(c, vec![(0, 1.0), (1, -1.0)]);
        assert!(cuts[0].rhs.abs() < 1e-12);
    }
}
