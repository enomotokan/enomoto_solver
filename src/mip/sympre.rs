//! 対称性の検出の前処理の試作 (`ENOMOTO_MIP_SYMPRE`、既定では使わない。analysis/sassy_20261110.md) と、検出に渡す問題の書き出し
//! (`ENOMOTO_MIP_DEBUG_DUMP_SYMGRAPH`、外部の検出プログラムとの比べ用)。
//!
//! 前処理は Anders・Schweitzer・Stieß (2023) の sassy の考えのうち 2 つを、ENOMOTO の行と列の二部グラフ (辺の重みは係数) に合わせたもの:
//! (1) 次数 0・1 の頂点を繰り返し外し、外した部分木の形 (ハッシュ) を残った頂点の色に書き込む。核 (残ったグラフ) の自己同型は
//! 部分木を同じ形どうし対応させて元の問題の置換に戻し、同じ親の下の同じ形の部分木の入れ替えも生成元に足す。
//! (3) 疎な自己同型の探り: 色の組の列を 1 つ個別化して細分化し、変わった列だけを対応させた置換を確かめ、組の全部が 1 つの軌道に
//! なれば片付いた組とする (軌道と安定化部分群の定理: 組の代表の列を個別化した残りの探索の生成元と合わせて群を作る)。
//! どの生成元も元の (検出に渡す) 問題で確かめてから使う。

use super::problem::MipProblem;

/// f64 をビットのまま 16 進で書く (外部のプログラムが色を正確に分けられるように)
fn hx(v: f64) -> String {
    format!("{:016x}", if v == 0.0 { 0u64 } else { v.to_bits() })
}

/// 検出に渡す問題 `p`、双子の組 `twins`、見つけた生成元 `gens` を `path` に書く。形式 (行ごと):
/// `n m` / 列ごとに `C cost lo up int` / 行ごとに `R lo up k j a ...` / 双子の組ごとに `T k j ...` / 生成元ごとに `G` + 動かす列の `j σ(j)` の組。
/// 値は f64 のビットの 16 進。
pub fn dump(p: &MipProblem, twins: &[Vec<usize>], gens: &[Vec<usize>], secs: f64, path: &str) {
    use std::io::Write;
    let Ok(f) = std::fs::File::create(path) else { return };
    let mut w = std::io::BufWriter::new(f);
    let _ = writeln!(w, "{} {} {:.6}", p.n, p.m, secs);
    for j in 0..p.n {
        let _ = writeln!(w, "C {} {} {} {}", hx(p.cost[j]), hx(p.col_lo[j]), hx(p.col_up[j]), p.is_int[j] as u8);
    }
    for i in 0..p.m {
        let _ = write!(w, "R {} {} {}", hx(p.row_lo[i]), hx(p.row_up[i]), p.rows[i].len());
        for &(j, a) in &p.rows[i] {
            let _ = write!(w, " {} {}", j, hx(a));
        }
        let _ = writeln!(w);
    }
    for t in twins {
        let _ = write!(w, "T {}", t.len());
        for &j in t {
            let _ = write!(w, " {j}");
        }
        let _ = writeln!(w);
    }
    for g in gens {
        let _ = write!(w, "G");
        for (j, &k) in g.iter().enumerate() {
            if j != k {
                let _ = write!(w, " {j} {k}");
            }
        }
        let _ = writeln!(w);
    }
}

use super::symmetry::{self, Symmetry};

/// `ENOMOTO_MIP_SYMPRE` (試作、既定では使わない): 値が `1`・`all` なら (1) と (3) の両方、`d1` なら (1) だけ、`probe` なら (3) だけ。
fn sympre_mode() -> (bool, bool) {
    match env_str!("ENOMOTO_MIP_SYMPRE").as_deref() {
        Some("d1") => (true, false),
        Some("probe") => (false, true),
        Some("0") | None => (false, false),
        Some(_) => (true, true),
    }
}

/// (3) の疎な自己同型の探りを使うか ([`symmetry`] の検出の中から呼ぶ)。
pub(crate) fn probe_on() -> bool {
    sympre_mode().1
}

fn fbits(v: f64) -> u64 {
    if v == 0.0 { 0 } else { v.to_bits() }
}

/// 色 (ハッシュ) を、検出の写しの費用・行の境界に書き込める値 (2^53 未満の整数の f64) にする
fn enc(h: u64) -> f64 {
    (h >> 11) as f64
}

/// 対称性を探す。`ENOMOTO_MIP_SYMPRE` が (1) を含まなければ [`symmetry::detect_skip_twins`] そのもの。
#[allow(clippy::too_many_arguments)]
pub fn detect(p: &MipProblem, time_cap: f64, max_tries: usize, work_cap: u64, log_cap: u64, twins: &[Vec<usize>], verbose: bool) -> (Option<Symmetry>, u64) {
    if !sympre_mode().0 {
        return symmetry::detect_skip_twins(p, time_cap, max_tries, work_cap, log_cap, twins);
    }
    detect_d1(p, time_cap, max_tries, work_cap, log_cap, twins, verbose)
}

/// (1) 次数 0・1 の頂点を外してから核で検出し、生成元を元の問題に戻す ([`detect`])。
#[allow(clippy::too_many_arguments)]
fn detect_d1(p: &MipProblem, time_cap: f64, max_tries: usize, work_cap: u64, log_cap: u64, twins: &[Vec<usize>], verbose: bool) -> (Option<Symmetry>, u64) {
    let t0 = std::time::Instant::now();
    let (n, m) = (p.n, p.m);
    let nv = n + m;
    let root = nv;
    // 頂点: 列 0..n、行 n..n+m。次数 0・1 の頂点を繰り返し外す (元から次数 0 の頂点は外さない: 固定された空の列を動かさない
    // `ENOMOTO_MIP_SYM_SKIP_FIXED` と合わせる)
    let deg0: Vec<usize> = (0..nv).map(|v| if v < n { p.cols[v].len() } else { p.rows[v - n].len() }).collect();
    let mut deg = deg0.clone();
    let mut alive = vec![true; nv];
    let base: Vec<u64> = (0..nv)
        .map(|v| {
            if v < n {
                symmetry::mix(symmetry::mix(fbits(p.cost[v]), fbits(p.col_lo[v])), symmetry::mix(fbits(p.col_up[v]), p.is_int[v] as u64))
            } else {
                symmetry::mix(symmetry::mix(fbits(p.row_lo[v - n]), fbits(p.row_up[v - n])), 0x5EED_0000_0001)
            }
        })
        .collect();
    // 外した子の部分木のハッシュ (親への係数つき)
    let mut kids: Vec<Vec<(u64, usize)>> = vec![Vec::new(); nv + 1];
    let mut par = vec![usize::MAX; nv];
    let mut sub = vec![0u64; nv];
    let mut order: Vec<usize> = Vec::new();
    let mut h = vec![0u64; nv];
    // 回ごとに、その回の始めに次数 1 以下の頂点を全部外す (木の葉から中心へ。1 本ずつ外すと、全部が外れる木の根が中心からずれ、
    // 根を動かす木の自己同型を失う)。最後に辺 1 本 (列と行) が残れば、行を列に付ける (二部グラフなので列と行は入れ替わらない)
    let fin = |v: usize, kids: &mut Vec<Vec<(u64, usize)>>| -> u64 {
        kids[v].sort_unstable();
        kids[v].iter().fold(base[v], |acc, &(s, _)| symmetry::mix(acc, s))
    };
    let mut cur: Vec<usize> = (0..nv).filter(|&v| deg0[v] == 1).collect();
    let mut in_round = vec![false; nv];
    while !cur.is_empty() {
        for &v in &cur {
            in_round[v] = true;
        }
        let mut next: Vec<usize> = Vec::new();
        for &v in &cur {
            if !alive[v] || deg[v] > 1 {
                continue;
            }
            // 生きている隣 (なければ根に付ける: 部分木で連結成分の全部が外れた)
            let nb: Option<(usize, f64)> = if v < n {
                p.cols[v].iter().find(|&&(i, _)| alive[n + i]).map(|&(i, a)| (n + i, a))
            } else {
                p.rows[v - n].iter().find(|&&(j, _)| alive[j]).map(|&(j, a)| (j, a))
            };
            if let Some((u, _)) = nb {
                if v < n && in_round[u] && deg[u] <= 1 {
                    // 残った辺 1 本: 列は残し、行をこの列に付ける (行の番で)
                    continue;
                }
            }
            alive[v] = false;
            h[v] = fin(v, &mut kids);
            order.push(v);
            match nb {
                Some((u, a)) => {
                    sub[v] = symmetry::mix(symmetry::mix(h[v], fbits(a)), (v < n) as u64);
                    par[v] = u;
                    kids[u].push((sub[v], v));
                    deg[u] -= 1;
                    if deg[u] <= 1 {
                        next.push(u);
                    }
                }
                None => {
                    sub[v] = symmetry::mix(h[v], 0x0F0F_0000 | (v < n) as u64);
                    par[v] = root;
                    kids[root].push((sub[v], v));
                }
            }
        }
        for &v in &cur {
            in_round[v] = false;
            if alive[v] {
                next.push(v);
            }
        }
        next.sort_unstable();
        next.dedup();
        next.retain(|&v| alive[v] && deg[v] <= 1);
        // 外れるものが無い回 (残った列が次数 1 のまま) で止まらないよう、前の回と同じなら止める
        if next == cur {
            break;
        }
        cur = next;
    }
    for v in 0..nv {
        if alive[v] {
            h[v] = fin(v, &mut kids);
        }
    }
    kids[root].sort_unstable();
    let removed = order.len();
    let (rc, rr) = (order.iter().filter(|&&v| v < n).count(), order.iter().filter(|&&v| v >= n).count());
    // 核の問題 (生きている列・行。費用と行の境界に色を書き込む)
    let core_cols: Vec<usize> = (0..n).filter(|&j| alive[j]).collect();
    let core_rows: Vec<usize> = (0..m).filter(|&i| alive[n + i]).collect();
    let mut cidx = vec![usize::MAX; n];
    for (k, &j) in core_cols.iter().enumerate() {
        cidx[j] = k;
    }
    let core = if removed == 0 {
        None
    } else {
        let rows: Vec<Vec<(usize, f64)>> = core_rows.iter().map(|&i| p.rows[i].iter().filter(|&&(j, _)| alive[j]).map(|&(j, a)| (cidx[j], a)).collect()).collect();
        let rl: Vec<f64> = core_rows.iter().map(|&i| enc(h[n + i])).collect();
        Some(MipProblem::from_rows(
            core_cols.iter().map(|&j| p.col_lo[j]).collect(),
            core_cols.iter().map(|&j| p.col_up[j]).collect(),
            core_cols.iter().map(|&j| enc(h[j])).collect(),
            0.0,
            1.0,
            core_cols.iter().map(|&j| p.is_int[j]).collect(),
            rows,
            rl.clone(),
            rl,
        ))
    };
    // 双子の組を核の番号に
    let mut twin_id = vec![usize::MAX; n];
    for (k, t) in twins.iter().enumerate() {
        for &j in t {
            twin_id[j] = k;
        }
    }
    let core_twins: Vec<Vec<usize>> = twins.iter().map(|t| t.iter().filter(|&&j| alive[j]).map(|&j| cidx[j]).collect::<Vec<_>>()).filter(|t| t.len() >= 2).collect();
    let t_red = t0.elapsed().as_secs_f64();
    let (core_sym, w) = match &core {
        None => symmetry::detect_skip_twins(p, time_cap, max_tries, work_cap, log_cap, twins),
        Some(q) if q.n >= 2 && q.m > 0 => symmetry::detect_skip_twins(q, (time_cap - t_red).max(0.0), max_tries, work_cap, log_cap, &core_twins),
        Some(_) => (None, 0),
    };
    let Some(core) = core else {
        if verbose {
            eprintln!("MIP: symmetry: SYMPRE removed 0 vertices of degree <= 1 ({t_red:.3}s); {:.2}s", t0.elapsed().as_secs_f64());
        }
        return (core_sym, w);
    };
    // 置換を元の問題に戻す: 核の列は核の生成元、核の行は (色, 写した中身) で対応させ、外した頂点は上から親の像の子のうち同じ形の
    // 同じ順位の子に写す
    let pos_in_kids = {
        // 各頂点の、親の子の並び (形, 番号の順) の中の位置
        let mut pos = vec![0usize; nv];
        for u in 0..=nv {
            for (k, &(_, v)) in kids[u].iter().enumerate() {
                pos[v] = k;
            }
        }
        pos
    };
    let lift = |img: &mut Vec<usize>| -> bool {
        // img: 長さ nv + 1 (列は埋まっている。行は usize::MAX)。核の行を写す
        let mut table: std::collections::HashMap<u64, Vec<usize>> = std::collections::HashMap::new();
        let key = |i: usize, f: &dyn Fn(usize) -> usize| -> u64 {
            let mut e: Vec<(usize, u64)> = p.rows[i].iter().filter(|&&(j, _)| alive[j]).map(|&(j, a)| (f(j), fbits(a))).collect();
            e.sort_unstable();
            e.iter().fold(h[n + i], |acc, &(j, a)| symmetry::mix(symmetry::mix(acc, j as u64), a))
        };
        for &i in core_rows.iter().rev() {
            table.entry(key(i, &|j| j)).or_default().push(i);
        }
        for &i in &core_rows {
            let k = key(i, &|j| img[j]);
            let Some(r) = table.get_mut(&k).and_then(|v| v.pop()) else { return false };
            img[n + i] = n + r;
        }
        img[root] = root;
        for &v in order.iter().rev() {
            let u = par[v];
            let pu = img[u];
            // 親の像の子のうち、同じ形の組の同じ順位
            let ks = &kids[u];
            let first = ks.partition_point(|&(s, _)| s < sub[v]);
            let rank = pos_in_kids[v] - first;
            let kt = &kids[pu];
            let f2 = kt.partition_point(|&(s, _)| s < sub[v]);
            match kt.get(f2 + rank) {
                Some(&(s, w)) if s == sub[v] => img[v] = w,
                _ => return false,
            }
        }
        true
    };
    let mut row_counts: std::collections::HashMap<u64, i64> = std::collections::HashMap::new();
    {
        let mut tmp: Vec<(usize, f64)> = Vec::new();
        for i in 0..m {
            tmp.clear();
            tmp.extend_from_slice(&p.rows[i]);
            *row_counts.entry(symmetry::row_hash(p.row_lo[i], p.row_up[i], &mut tmp)).or_insert(0) += 1;
        }
    }
    let cap_gens = tunable!("ENOMOTO_T_MIP_SYMPRE_GENS", 2000usize, usize).min((2e7 / n.max(1) as f64) as usize).max(1);
    let mut gens: Vec<Vec<usize>> = Vec::new();
    let (mut lifted, mut bad) = (0usize, 0usize);
    let core_gens = core_sym.map(|s| s.generators).unwrap_or_default();
    for g in &core_gens {
        let mut img = vec![usize::MAX; nv + 1];
        for (k, &j) in core_cols.iter().enumerate() {
            img[j] = core_cols[g[k]];
        }
        if lift(&mut img) {
            let perm: Vec<usize> = img[..n].to_vec();
            if symmetry::verify_perm(p, &perm, &row_counts) {
                gens.push(perm);
                lifted += 1;
                continue;
            }
        }
        bad += 1;
    }
    // 同じ親の下の同じ形の部分木の入れ替え (隣どうしの互換)。双子 (呼び出し側が扱う) どうしは除く。
    // 核で同じ行 (色と中身が同じ) の組も、行に付いた部分木の入れ替えを足す (核の列を動かさないので、核の列の置換では見えない)
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for u in 0..=nv {
        for w in kids[u].windows(2) {
            let ((s1, x), (s2, y)) = (w[0], w[1]);
            if s1 == s2 && !(x < n && y < n && twin_id[x] != usize::MAX && twin_id[x] == twin_id[y]) {
                pairs.push((x, y));
            }
        }
    }
    {
        let mut same: std::collections::HashMap<u64, Vec<usize>> = std::collections::HashMap::new();
        for &i in &core_rows {
            if kids[n + i].is_empty() {
                continue;
            }
            let mut e: Vec<(usize, u64)> = p.rows[i].iter().filter(|&&(j, _)| alive[j]).map(|&(j, a)| (j, fbits(a))).collect();
            e.sort_unstable();
            let k = e.iter().fold(h[n + i], |acc, &(j, a)| symmetry::mix(symmetry::mix(acc, j as u64), a));
            same.entry(k).or_default().push(n + i);
        }
        let mut groups: Vec<Vec<usize>> = same.into_values().filter(|g| g.len() >= 2).collect();
        groups.sort_unstable();
        for g in groups {
            for w in g.windows(2) {
                pairs.push((w[0], w[1]));
            }
        }
    }
    let (mut leaf_gens, mut leaf_bad, mut leaf_cut) = (0usize, 0usize, 0usize);
    for (x, y) in pairs {
        {
            if gens.len() >= cap_gens {
                leaf_cut += 1;
                continue;
            }
            let mut perm: Vec<usize> = (0..n).collect();
            let mut st = vec![(x, y)];
            let mut moves_col = false;
            while let Some((a, b)) = st.pop() {
                if a < n {
                    perm[a] = b;
                    perm[b] = a;
                    moves_col = true;
                }
                for (ka, kb) in kids[a].iter().zip(&kids[b]) {
                    st.push((ka.1, kb.1));
                }
            }
            if !moves_col {
                continue;
            }
            if symmetry::verify_perm(p, &perm, &row_counts) {
                gens.push(perm);
                leaf_gens += 1;
            } else {
                leaf_bad += 1;
            }
        }
    }
    if verbose {
        eprintln!(
            "MIP: symmetry: SYMPRE removed {rc} columns and {rr} rows of degree <= 1 ({} trees at the root, {t_red:.3}s); core {} columns {} rows; core generators {} (lifted {lifted}, failed {bad}); subtree swaps {leaf_gens} (failed {leaf_bad}, cut {leaf_cut}); {:.2}s",
            kids[root].len(),
            core.n,
            core.m,
            core_gens.len(),
            t0.elapsed().as_secs_f64()
        );
    }
    (Symmetry::from_generators(n, gens), w)
}

/// (3) 疎な自己同型の探り ([`symmetry`] の検出の中、深い探索の前に呼ぶ)。色の組ごとに先頭の列 `a` を個別化して細分化した色
/// (1 回だけ作る) と、組の各列 `b` を個別化して細分化した色を比べ、色が変わった列だけを対応させた置換 ([`symmetry::shortcut_perm`]) を
/// 確かめて生成元にする。組の全部が `a` の軌道に入れば、その組は片付く (深い探索も組の中の対を試さない)。
#[allow(clippy::too_many_arguments)]
pub(super) fn probe(
    p: &MipProblem,
    col0: &[u64],
    row0: &[u64],
    cells: &[Vec<usize>],
    uf: &mut [usize],
    generators: &mut Vec<Vec<usize>>,
    row_counts: &std::collections::HashMap<u64, i64>,
    caps: (std::time::Instant, f64, (u64, u64)),
) {
    use symmetry::{find, mix};
    let (t0, time_cap, work_end) = caps;
    let t1 = std::time::Instant::now();
    let n = p.n;
    let g0 = generators.len();
    let (mut tried, mut done, mut stopped) = (0usize, 0usize, false);
    let groups = |c: &[u64]| {
        let mut mm: std::collections::HashMap<u64, Vec<usize>> = std::collections::HashMap::new();
        for (j, &x) in c.iter().enumerate() {
            mm.entry(x).or_default().push(j);
        }
        mm
    };
    'cells: for cell in cells {
        let a = cell[0];
        if cell[1..].iter().all(|&b| find(uf, a) == find(uf, b)) {
            done += 1;
            continue;
        }
        if t0.elapsed().as_secs_f64() > time_cap || symmetry::over(work_end) {
            stopped = true;
            break;
        }
        let (mut ca, mut ra) = (col0.to_vec(), row0.to_vec());
        ca[a] = mix(ca[a], 0xA5A5_0000 ^ 1);
        symmetry::refine_inc(p, &mut ca, &mut ra, &[a], symmetry::refine_iters());
        let ma = groups(&ca);
        for &b in &cell[1..] {
            if t0.elapsed().as_secs_f64() > time_cap || symmetry::over(work_end) {
                stopped = true;
                break 'cells;
            }
            if find(uf, a) == find(uf, b) {
                continue;
            }
            tried += 1;
            let (mut cb, mut rb) = (col0.to_vec(), row0.to_vec());
            cb[b] = mix(cb[b], 0xA5A5_0000 ^ 1);
            symmetry::refine_inc(p, &mut cb, &mut rb, &[b], symmetry::refine_iters());
            let mb = groups(&cb);
            if ma.len() != mb.len() {
                continue;
            }
            if let Some(perm) = symmetry::shortcut_perm(n, &ma, &mb) {
                if symmetry::verify_perm(p, &perm, row_counts) {
                    for j in 0..n {
                        let (x, y) = (find(uf, j), find(uf, perm[j]));
                        if x != y {
                            uf[x] = y;
                        }
                    }
                    generators.push(perm);
                }
            }
        }
        if cell[1..].iter().all(|&b| find(uf, a) == find(uf, b)) {
            done += 1;
        }
    }
    if env_str!("ENOMOTO_MIP_LOG").is_some() {
        eprintln!(
            "MIP: symmetry: SYMPRE probe: {} cells, {done} done as orbits, {tried} probes, {} generators ({:.3}s{})",
            cells.len(),
            generators.len() - g0,
            t1.elapsed().as_secs_f64(),
            if stopped { ", stopped by the caps" } else { "" }
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2 つの同じ部品 (列 x_k と、行 g_k: x_k + s_k + 2 t_k = 1 の 1 行だけの列 s_k・t_k) が行 r0: x_1 + x_2 <= 1 でつながる問題は、
    /// (1) で全部が木として外れ (核は空)、根の下の部分木の入れ替え 1 つ (x_1<->x_2, s_1<->s_2, t_1<->t_2) が生成元になる
    #[test]
    fn d1_collapses_trees_and_swaps_subtrees() {
        // 列: x1 x2 s1 t1 s2 t2
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(0, 1.0), (2, 1.0), (3, 2.0)], vec![(1, 1.0), (4, 1.0), (5, 2.0)]];
        let p = MipProblem::from_rows(vec![0.0; 6], vec![1.0; 6], vec![1.0, 1.0, 2.0, 3.0, 2.0, 3.0], 0.0, 1.0, vec![true; 6], rows, vec![f64::NEG_INFINITY, 1.0, 1.0], vec![1.0, 1.0, 1.0]);
        let (sym, _) = detect_d1(&p, 10.0, 100, u64::MAX, u64::MAX, &[], false);
        let sym = sym.expect("対称性があるはず");
        assert_eq!(sym.generators, vec![vec![1, 0, 4, 5, 2, 3]]);
        assert_eq!(sym.num_orbits, 3);
    }

    /// 核が残る問題: 2 本の行 a + b + c = 1, a + b + d = 1 (c・d は 1 行だけ) と、a・b 両方に付いた葉の列。核の生成元 (a<->b) を
    /// 戻すと葉も入れ替わり、確かめを通る
    #[test]
    fn d1_lifts_core_generators() {
        // 列: a b c d la lb (la は a だけの行 a + la <= 1、lb は b だけの行 b + lb <= 1)
        let rows = vec![
            vec![(0, 1.0), (1, 1.0), (2, 1.0)],
            vec![(0, 1.0), (1, 1.0), (3, 1.0)],
            vec![(0, 1.0), (4, 1.0)],
            vec![(1, 1.0), (5, 1.0)],
        ];
        let p = MipProblem::from_rows(vec![0.0; 6], vec![1.0; 6], vec![1.0, 1.0, 0.0, 0.0, 5.0, 5.0], 0.0, 1.0, vec![true; 6], rows, vec![1.0, 1.0, f64::NEG_INFINITY, f64::NEG_INFINITY], vec![1.0; 4]);
        let (sym, _) = detect_d1(&p, 10.0, 100, u64::MAX, u64::MAX, &[], false);
        let sym = sym.expect("対称性があるはず");
        assert!(sym.generators.contains(&vec![1, 0, 2, 3, 5, 4]), "{:?}", sym.generators);
        assert!(sym.generators.contains(&vec![0, 1, 3, 2, 4, 5]), "{:?}", sym.generators);
    }
}
