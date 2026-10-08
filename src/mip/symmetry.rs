//! 対称性の検出 (列の置換で問題全体 (費用・境界・整数性・行) が変わらないもの) と、対称性を崩す不等式。
//!
//! **検出**: 行と列の二部グラフ (辺の重みは係数) の色の細分化 (color refinement) で、入れ替えうる列の組
//! (同じ色の列) を求め、列 `a` を `b` に写す置換を個別化 (individualization) と細分化を繰り返して作る
//! (色が全て 1 列ずつになるまで。残った同じ色の列は、なるべく同じ列どうし (固定) を対応させる)。
//! 得た置換は費用・境界・整数性と全ての行 (行の多重集合が置換で写り合う) を確かめてから生成元にする。
//! 生成元で同じ軌道 (union-find) に入った列の組は試さない。HiGHS の `HighsSymmetryDetection` (個別化・細分化による
//! 自己同型の探索) を簡略化したもの (探索木の後戻りはせず、貪欲に 1 本だけ辿る)。
//!
//! **対称性を崩す不等式** (lex-leader): 列の番号の順の辞書式で最大の解 `x*` は、群の全ての元 `σ` について
//! `x* >=_lex σ(x*)` (`σ(x)_k = x_{σ^{-1}(k)}`) を満たす。`σ` が最初に動かす番号を `i` とすると、それより前は同じなので
//! `x_i >= x_{σ^{-1}(i)}` が成り立つ。これを (1) 生成元ごとに、(2) 全ての元の中で最初に動かされる番号 `i0` の軌道の
//! 全ての列 `k` について `x_{i0} >= x_k` (`i0` より前を動かす元はないので) 加える。どれも同じ辞書式の順から
//! 導くので同時に成り立つ (最適解のうち辞書式で最大のものを残す)。

use super::problem::MipProblem;

fn mix(a: u64, b: u64) -> u64 {
    // splitmix64 の混ぜ方
    let mut z = a ^ b.wrapping_add(0x9E37_79B9_7F4A_7C15).wrapping_add(a << 6).wrapping_add(a >> 2);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn fbits(v: f64) -> u64 {
    if v == 0.0 { 0 } else { v.to_bits() }
}

/// 見つけた対称性。
pub struct Symmetry {
    /// 生成元 (列の置換 `perm[j] = σ(j)`)。
    pub generators: Vec<Vec<usize>>,
    /// 列の軌道の番号 (動かない列は `usize::MAX`)。
    pub orbit: Vec<usize>,
    /// 軌道の数 (2 列以上のもの)。
    pub num_orbits: usize,
}

/// 色の細分化。色の種類の数が増えなくなるまで (最大 `max_iter` 回) 繰り返す。
fn refine(p: &MipProblem, col: &mut [u64], row: &mut [u64], max_iter: usize) {
    let mut prev = usize::MAX;
    let mut buf: Vec<u64> = Vec::new();
    for _ in 0..max_iter {
        let mut new_row = vec![0u64; p.m];
        for i in 0..p.m {
            buf.clear();
            buf.extend(p.rows[i].iter().map(|&(j, a)| mix(col[j], fbits(a))));
            buf.sort_unstable();
            new_row[i] = buf.iter().fold(row[i], |h, &v| mix(h, v));
        }
        let mut new_col = vec![0u64; p.n];
        for j in 0..p.n {
            buf.clear();
            buf.extend(p.cols[j].iter().map(|&(i, a)| mix(new_row[i], fbits(a))));
            buf.sort_unstable();
            new_col[j] = buf.iter().fold(col[j], |h, &v| mix(h, v));
        }
        row.copy_from_slice(&new_row);
        col.copy_from_slice(&new_col);
        let mut all: Vec<u64> = col.to_vec();
        all.extend_from_slice(row);
        all.sort_unstable();
        all.dedup();
        if all.len() == prev {
            break;
        }
        prev = all.len();
    }
}

/// 行の中身 (境界と (列, 係数) の並び) のハッシュ。
fn row_hash(lo: f64, up: f64, entries: &mut [(usize, f64)]) -> u64 {
    entries.sort_unstable_by_key(|&(j, _)| j);
    entries.iter().fold(mix(fbits(lo), fbits(up)), |h, &(j, a)| mix(mix(h, j as u64), fbits(a)))
}

/// 置換 `perm` が問題の対称性か確かめる。
fn verify(p: &MipProblem, perm: &[usize], row_counts: &std::collections::HashMap<u64, i64>) -> bool {
    for j in 0..p.n {
        let k = perm[j];
        if p.cost[j] != p.cost[k] || p.col_lo[j] != p.col_lo[k] || p.col_up[j] != p.col_up[k] || p.is_int[j] != p.is_int[k] {
            return false;
        }
    }
    let mut counts = row_counts.clone();
    let mut tmp: Vec<(usize, f64)> = Vec::new();
    for i in 0..p.m {
        tmp.clear();
        tmp.extend(p.rows[i].iter().map(|&(j, a)| (perm[j], a)));
        let h = row_hash(p.row_lo[i], p.row_up[i], &mut tmp);
        match counts.get_mut(&h) {
            Some(c) if *c > 0 => *c -= 1,
            _ => return false,
        }
    }
    true
}

fn find(uf: &mut [usize], mut x: usize) -> usize {
    while uf[x] != x {
        uf[x] = uf[uf[x]];
        x = uf[x];
    }
    x
}

/// 対称性を探す。`time_cap` 秒・`max_tries` 回で打ち切る。見つからなければ `None`。
pub fn detect(p: &MipProblem, time_cap: f64, max_tries: usize) -> Option<Symmetry> {
    let t0 = std::time::Instant::now();
    let n = p.n;
    if n < 2 || p.m == 0 {
        return None;
    }
    let mut col: Vec<u64> = (0..n).map(|j| mix(mix(fbits(p.cost[j]), fbits(p.col_lo[j])), mix(fbits(p.col_up[j]), p.is_int[j] as u64))).collect();
    let mut row: Vec<u64> = (0..p.m).map(|i| mix(fbits(p.row_lo[i]), fbits(p.row_up[i]))).collect();
    refine(p, &mut col, &mut row, 50);
    // 同じ色の列の組 (2 列以上)
    let mut cells: std::collections::HashMap<u64, Vec<usize>> = std::collections::HashMap::new();
    for j in 0..n {
        cells.entry(col[j]).or_default().push(j);
    }
    let mut cells: Vec<Vec<usize>> = cells.into_values().filter(|c| c.len() >= 2).collect();
    if cells.is_empty() {
        return None;
    }
    cells.sort_by_key(|c| c[0]);
    let mut row_counts: std::collections::HashMap<u64, i64> = std::collections::HashMap::new();
    let mut tmp: Vec<(usize, f64)> = Vec::new();
    for i in 0..p.m {
        tmp.clear();
        tmp.extend_from_slice(&p.rows[i]);
        *row_counts.entry(row_hash(p.row_lo[i], p.row_up[i], &mut tmp)).or_insert(0) += 1;
    }
    let mut uf: Vec<usize> = (0..n).collect();
    let mut generators: Vec<Vec<usize>> = Vec::new();
    let mut tries = 0usize;
    'cells: for cell in &cells {
        let a = cell[0];
        for &b in &cell[1..] {
            if tries >= max_tries || t0.elapsed().as_secs_f64() > time_cap {
                break 'cells;
            }
            if find(&mut uf, a) == find(&mut uf, b) {
                continue;
            }
            tries += 1;
            if let Some(perm) = search(p, &col, &row, a, b, t0, time_cap) {
                if verify(p, &perm, &row_counts) {
                    for j in 0..n {
                        let (x, y) = (find(&mut uf, j), find(&mut uf, perm[j]));
                        if x != y {
                            uf[x] = y;
                        }
                    }
                    generators.push(perm);
                }
            }
        }
    }
    if generators.is_empty() {
        return None;
    }
    // 軌道 (2 列以上)
    let mut size = vec![0usize; n];
    for j in 0..n {
        let r = find(&mut uf, j);
        size[r] += 1;
    }
    let mut id = vec![usize::MAX; n];
    let mut num = 0;
    let mut orbit = vec![usize::MAX; n];
    for j in 0..n {
        let r = find(&mut uf, j);
        if size[r] >= 2 {
            if id[r] == usize::MAX {
                id[r] = num;
                num += 1;
            }
            orbit[j] = id[r];
        }
    }
    Some(Symmetry { generators, orbit, num_orbits: num })
}

/// 列 `a` を `b` に写す置換を、個別化と細分化で 1 本作る (色が全て 1 列ずつになるまで)。作れなければ `None`。
fn search(p: &MipProblem, col0: &[u64], row0: &[u64], a: usize, b: usize, t0: std::time::Instant, time_cap: f64) -> Option<Vec<usize>> {
    let n = p.n;
    let (mut ca, mut ra) = (col0.to_vec(), row0.to_vec());
    let (mut cb, mut rb) = (col0.to_vec(), row0.to_vec());
    let mut level: u64 = 1;
    let indiv = |c: &mut [u64], j: usize, level: u64| c[j] = mix(c[j], 0xA5A5_0000 ^ level);
    indiv(&mut ca, a, level);
    indiv(&mut cb, b, level);
    refine(p, &mut ca, &mut ra, 50);
    refine(p, &mut cb, &mut rb, 50);
    for _ in 0..n {
        if t0.elapsed().as_secs_f64() > time_cap {
            return None;
        }
        // 色ごとの列 (A と B で同じ大きさでなければ失敗)
        let mut ma: std::collections::HashMap<u64, Vec<usize>> = std::collections::HashMap::new();
        let mut mb: std::collections::HashMap<u64, Vec<usize>> = std::collections::HashMap::new();
        for j in 0..n {
            ma.entry(ca[j]).or_default().push(j);
            mb.entry(cb[j]).or_default().push(j);
        }
        if ma.len() != mb.len() {
            return None;
        }
        let mut pick: Option<(usize, usize)> = None;
        for (c, va) in &ma {
            let vb = mb.get(c)?;
            if vb.len() != va.len() {
                return None;
            }
            if va.len() >= 2 {
                let x = va[0];
                if pick.is_none_or(|(px, _)| x < px) {
                    // B では同じ列があればそれ (固定)、なければ最小の列
                    let y = if vb.contains(&x) { x } else { vb[0] };
                    pick = Some((x, y));
                }
            }
        }
        let Some((x, y)) = pick else {
            // 全て 1 列ずつ: 色で対応させる
            let mut perm = vec![0usize; n];
            for (c, va) in &ma {
                perm[va[0]] = mb[c][0];
            }
            return Some(perm);
        };
        level += 1;
        indiv(&mut ca, x, level);
        indiv(&mut cb, y, level);
        refine(p, &mut ca, &mut ra, 50);
        refine(p, &mut cb, &mut rb, 50);
    }
    None
}

/// 完全オービトープ: 0-1 列の行列 `vars[行][列]` で、列をどう並べ替えても問題が変わらないもの (列の入れ替えが
/// 全て対称性)。行は最小の列番号の順、列は 1 行目の列番号の順に並べる。
#[derive(Clone, Debug)]
pub struct Orbitope {
    pub vars: Vec<Vec<usize>>,
    /// 全ての行がパッキング・分割の行に含まれる (行の和 <= 1)。
    pub packing: bool,
    /// 行ごとに、パッキング・分割の行に含まれるか (動的な固定で、分枝した行が全てパッキングならパッキング用の固定を使う)。
    pub row_packing: Vec<bool>,
    /// 列の入れ替えの生成元 (列 a, 列 b, 問題全体の置換 `perm[j] = σ(j)`。0-1 でない列も一緒に動かす)。
    pub swaps: Vec<(usize, usize, std::rc::Rc<Vec<usize>>)>,
}

/// 置換 `perm` (`perm[j] = σ(j)`) で解を写す: `y[σ(j)] = x[j]`。
fn apply_perm(perm: &[usize], x: &mut [f64]) {
    let old = x.to_vec();
    for (j, &k) in perm.iter().enumerate() {
        x[k] = old[j];
    }
}

impl Orbitope {
    /// 行 `rows` (この順) だけからなるオービトープ (動的な orbitopal fixing 用。行の部分集合でも列の入れ替えは
    /// 対称性なので、その順の辞書式で列を並べた解が必ずある)。
    pub fn sub_rows(&self, rows: &[usize]) -> Orbitope {
        Orbitope {
            vars: rows.iter().map(|&i| self.vars[i].clone()).collect(),
            packing: rows.iter().all(|&i| self.row_packing[i]),
            row_packing: rows.iter().map(|&i| self.row_packing[i]).collect(),
            swaps: Vec::new(),
        }
    }

    /// 列 `a` と `b` を入れ替える置換 (問題全体)。生成元の列の入れ替えをたどった道 `a = c0 - c1 - ... - ck = b` から、
    /// `(c0 ck) = (c0 c1)(c1 ... ck)(c0 c1)` (共役) で作る。道がなければ `None`。
    fn transposition(&self, a: usize, b: usize) -> Option<Vec<usize>> {
        let s = self.vars[0].len();
        // 列のグラフで幅優先探索 (辺: 生成元の番号)
        let mut prev: Vec<Option<(usize, usize)>> = vec![None; s];
        let mut seen = vec![false; s];
        seen[a] = true;
        let mut queue = std::collections::VecDeque::from([a]);
        while let Some(c) = queue.pop_front() {
            if c == b {
                break;
            }
            for (e, &(x, y, _)) in self.swaps.iter().enumerate() {
                let d = if x == c { y } else if y == c { x } else { continue };
                if !seen[d] {
                    seen[d] = true;
                    prev[d] = Some((c, e));
                    queue.push_back(d);
                }
            }
        }
        if !seen[b] {
            return None;
        }
        // b から a への辺の列 (a に近い順)
        let mut edges: Vec<usize> = Vec::new();
        let mut c = b;
        while c != a {
            let (p, e) = prev[c]?;
            edges.push(e);
            c = p;
        }
        edges.reverse();
        // 最後の辺の入れ替えを、手前の辺で順に共役する: T = g_0 g_1 ... g_{k-1} ... g_1 g_0
        let compose = |f: &[usize], g: &[usize]| -> Vec<usize> { (0..f.len()).map(|j| f[g[j]]).collect() };
        let mut t: Vec<usize> = (*self.swaps[*edges.last()?].2).clone();
        for &e in edges.iter().rev().skip(1) {
            let g = &self.swaps[e].2;
            t = compose(g, &compose(&t, g));
        }
        Some(t)
    }

    /// 解 `x` の列を、オービトープの行で見て辞書式で広義減少に並べ替える (列の入れ替えは対称性なので、得た解も
    /// 実行可能で目的値も同じ)。並べ替えられなければ偽。
    pub fn canonicalize(&self, x: &mut [f64]) -> bool {
        let s = self.vars[0].len();
        let key = |x: &[f64], c: usize| -> Vec<i64> { self.vars.iter().map(|line| x[line[c]].round() as i64).collect() };
        for pos in 0..s {
            let mut best = pos;
            for c in pos + 1..s {
                if key(x, c) > key(x, best) {
                    best = c;
                }
            }
            if best != pos {
                let Some(t) = self.transposition(pos, best) else { return false };
                apply_perm(&t, x);
            }
        }
        true
    }
}

/// 生成元のうち、0-1 列の 2-巡回だけからなるもの (列の入れ替え) をまとめて完全オービトープを作る。
/// 戻り値は (オービトープ、オービトープに使った生成元の印)。
///
/// 生成元を、動かす列の軌道 (行) を共有するものどうしでまとめ、各まとまりについて: 全ての行が同じ大きさ `s`、
/// 各生成元がどの行でもちょうど 2 列を入れ替える。1 行目の列を「列」とし、他の行の各列は「それを動かす生成元の組」が
/// 1 行目のどの列と同じかで列を決める。最後に、各生成元がどの行でも同じ 2 列を入れ替えることを確かめる。
pub fn find_orbitopes(p: &MipProblem, sym: &Symmetry) -> (Vec<Orbitope>, Vec<bool>) {
    let n = p.n;
    // 固定された 0-1 列も含める (前処理・プロービングで固定されたものは、固定として扱えばよい)
    let binary = |j: usize| p.is_int[j] && p.col_lo[j] >= 0.0 && p.col_up[j] <= 1.0;
    let ng = sym.generators.len();
    let mut used = vec![false; ng];
    // 2-巡回だけからなり、0-1 列を動かす生成元。0-1 でない列も一緒に動かしてよい: 列の入れ替えはそれらを含めて対称性
    // なので、0-1 の部分の行列の列を辞書式に並べ替えた解 (0-1 でない列も一緒に入れ替えたもの) が必ずある
    let cand: Vec<usize> = (0..ng)
        .filter(|&k| {
            let g = &sym.generators[k];
            (0..n).all(|j| g[g[j]] == j) && (0..n).any(|j| g[j] != j && binary(j))
        })
        .collect();
    if cand.is_empty() {
        return (Vec::new(), used);
    }
    // 候補の生成元だけでの軌道
    let mut uf: Vec<usize> = (0..n).collect();
    for &k in &cand {
        let g = &sym.generators[k];
        for j in 0..n {
            let (x, y) = (find(&mut uf, j), find(&mut uf, g[j]));
            if x != y {
                uf[x] = y;
            }
        }
    }
    // 生成元を、共有する軌道でまとめる (生成元の union-find)
    let mut guf: Vec<usize> = (0..cand.len()).collect();
    let mut owner: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (ci, &k) in cand.iter().enumerate() {
        let g = &sym.generators[k];
        for j in 0..n {
            if g[j] != j {
                let r = find(&mut uf, j);
                match owner.get(&r) {
                    Some(&o) => {
                        let (a, b) = (find(&mut guf, ci), find(&mut guf, o));
                        if a != b {
                            guf[a] = b;
                        }
                    }
                    None => {
                        owner.insert(r, ci);
                    }
                }
            }
        }
    }
    let mut clusters: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for ci in 0..cand.len() {
        let r = find(&mut guf, ci);
        clusters.entry(r).or_default().push(cand[ci]);
    }
    let mut out = Vec::new();
    'cl: for gens in clusters.values() {
        // まとまりの行 (軌道) と、その要素
        let mut rows_map: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
        for &k in gens {
            let g = &sym.generators[k];
            for j in 0..n {
                if g[j] != j && binary(j) {
                    rows_map.entry(find(&mut uf, j)).or_default().push(j);
                }
            }
        }
        let mut rows: Vec<Vec<usize>> = rows_map
            .into_values()
            .map(|mut v| {
                v.sort_unstable();
                v.dedup();
                v
            })
            .collect();
        rows.sort_by_key(|r| r[0]);
        let s = rows[0].len();
        if s < 2 || rows.iter().any(|r| r.len() != s) {
            if env_str!("ENOMOTO_MIP_SYM_DEBUG").is_some() {
                let mut sizes: Vec<usize> = rows.iter().map(|r| r.len()).collect();
                sizes.sort_unstable();
                sizes.dedup();
                eprintln!("SYM orbitope reject: {} gens, {} rows, row sizes {:?}", gens.len(), rows.len(), sizes);
            }
            continue;
        }
        // 各生成元はどの行でもちょうど 2 列を動かす
        for &k in gens {
            let g = &sym.generators[k];
            if rows.iter().any(|r| r.iter().filter(|&&j| g[j] != j).count() != 2) {
                if env_str!("ENOMOTO_MIP_SYM_DEBUG").is_some() {
                    eprintln!("SYM orbitope reject: generator {k} does not move 2 per row ({} rows)", rows.len());
                }
                continue 'cl;
            }
        }
        // 列 c (1 行目の c 番目) を動かす生成元の組
        let sig = |j: usize| -> Vec<usize> { gens.iter().copied().filter(|&k| sym.generators[k][j] != j).collect() };
        let col_sig: Vec<Vec<usize>> = rows[0].iter().map(|&j| sig(j)).collect();
        {
            let mut cs = col_sig.clone();
            cs.sort();
            cs.dedup();
            if cs.len() != s {
                if env_str!("ENOMOTO_MIP_SYM_DEBUG").is_some() {
                    eprintln!("SYM orbitope reject: columns not distinguishable");
                }
                continue; // 列を区別できない
            }
        }
        let mut mat: Vec<Vec<usize>> = Vec::with_capacity(rows.len());
        for r in &rows {
            let mut line = vec![usize::MAX; s];
            for &j in r {
                let sj = sig(j);
                let Some(c) = col_sig.iter().position(|x| *x == sj) else { continue 'cl };
                if line[c] != usize::MAX {
                    continue 'cl;
                }
                line[c] = j;
            }
            mat.push(line);
        }
        // 各生成元がどの行でも同じ 2 列を入れ替える
        for &k in gens {
            let g = &sym.generators[k];
            let cols: Vec<usize> = (0..s).filter(|&c| g[mat[0][c]] != mat[0][c]).collect();
            if cols.len() != 2 {
                continue 'cl;
            }
            let (a, b) = (cols[0], cols[1]);
            if mat.iter().any(|line| g[line[a]] != line[b] || g[line[b]] != line[a]) {
                continue 'cl;
            }
        }
        let mut swaps = Vec::with_capacity(gens.len());
        for &k in gens {
            used[k] = true;
            let g = &sym.generators[k];
            let cols: Vec<usize> = (0..s).filter(|&c| g[mat[0][c]] != mat[0][c]).collect();
            swaps.push((cols[0], cols[1], std::rc::Rc::new(g.clone())));
        }
        let nrows = mat.len();
        out.push(Orbitope { vars: mat, packing: false, row_packing: vec![false; nrows], swaps });
    }
    (out, used)
}

/// `w` 以下 (辞書式) で固定 `fix` を満たす最大の 0-1 列ベクトル (`w` が `None` なら上限なし)。なければ `None`。
fn lexmax_le(w: Option<&[u8]>, fix: &[Option<u8>]) -> Option<Vec<u8>> {
    let r = fix.len();
    let Some(w) = w else { return Some(fix.iter().map(|f| f.unwrap_or(1)).collect()) };
    let Some(i) = (0..r).find(|&k| fix[k].is_some_and(|f| f != w[k])) else { return Some(w.to_vec()) };
    let mut v = w.to_vec();
    let cut = if fix[i] == Some(0) {
        // w_i = 1 を 0 にすれば w より小さい
        i
    } else {
        // w_i = 0 だが 1 に固定: それより前で w_j = 1 の自由な行を 0 にする (最も後ろ)
        (0..i).rev().find(|&j| w[j] == 1 && fix[j].is_none())?
    };
    v[cut] = 0;
    for k in cut + 1..r {
        v[k] = fix[k].unwrap_or(1);
    }
    Some(v)
}

/// `w` 以上 (辞書式) で固定 `fix` を満たす最小の 0-1 列ベクトル (`w` が `None` なら下限なし)。なければ `None`。
fn lexmin_ge(w: Option<&[u8]>, fix: &[Option<u8>]) -> Option<Vec<u8>> {
    let r = fix.len();
    let Some(w) = w else { return Some(fix.iter().map(|f| f.unwrap_or(0)).collect()) };
    let Some(i) = (0..r).find(|&k| fix[k].is_some_and(|f| f != w[k])) else { return Some(w.to_vec()) };
    let mut v = w.to_vec();
    let cut = if fix[i] == Some(1) {
        i
    } else {
        (0..i).rev().find(|&j| w[j] == 0 && fix[j].is_none())?
    };
    v[cut] = 1;
    for k in cut + 1..r {
        v[k] = fix[k].unwrap_or(0);
    }
    Some(v)
}

/// 完全オービトープの固定 (orbitopal fixing、Bendotti・Fouilhoux・Rottner 2021): 列が辞書式で広義単調減少
/// (`列 0 >=_lex 列 1 >=_lex ...`) の解だけを残す。今の境界 (`lo`, `up`) から、辞書式で最大の解の列 (左から) と最小の解の
/// 列 (右から) を作り、ある列で最小 > 最大なら実行不能 (`None`)。そうでなければ各列で最大と最小が初めて違う行より上を
/// その値に固定する (`(列番号, 値)` の一覧を返す)。
pub fn orbitopal_fixing(orb: &Orbitope, lo: &[f64], up: &[f64]) -> Option<Vec<(usize, f64)>> {
    let r = orb.vars.len();
    let s = orb.vars[0].len();
    let fix_of = |c: usize| -> Vec<Option<u8>> {
        (0..r)
            .map(|i| {
                let j = orb.vars[i][c];
                if lo[j] >= 0.5 {
                    Some(1)
                } else if up[j] <= 0.5 {
                    Some(0)
                } else {
                    None
                }
            })
            .collect()
    };
    let fixes: Vec<Vec<Option<u8>>> = (0..s).map(fix_of).collect();
    let mut maxv: Vec<Vec<u8>> = Vec::with_capacity(s);
    for c in 0..s {
        let v = lexmax_le(maxv.last().map(|v: &Vec<u8>| v.as_slice()), &fixes[c])?;
        maxv.push(v);
    }
    let mut minv: Vec<Vec<u8>> = vec![Vec::new(); s];
    for c in (0..s).rev() {
        let w = if c + 1 < s { Some(minv[c + 1].as_slice()) } else { None };
        minv[c] = lexmin_ge(w, &fixes[c])?;
    }
    let mut out = Vec::new();
    if orb.packing && env_str!("ENOMOTO_MIP_NO_PACKING_FIXING").is_none() {
        out.extend(packing_fixing(orb, &fixes)?);
    }
    for c in 0..s {
        if minv[c] > maxv[c] {
            return None;
        }
        let first = (0..r).find(|&i| minv[c][i] != maxv[c][i]).unwrap_or(r);
        for i in 0..first {
            if fixes[c][i].is_none() {
                out.push((orb.vars[i][c], minv[c][i] as f64));
            }
        }
    }
    Some(out)
}

/// パッキング・オービトープ (各行の和 <= 1) の固定: 列が辞書式で広義減少なら、0 でない列の最初の 1 の行は列ごとに
/// 真に増え、0 の列は最後に来る。列 c の最初の 1 の行の範囲 `[L_c, U_c]` を、L は左から (前の列の L より後で 0 に
/// 固定されていない最初の行)、U は右から (1 に固定された最初の行、次の列の U - 1) 求める。L > U なら実行不能、L より上は
/// 0、L = U ならその行を 1 に固定する。1 を置ける行がない列は 0 の列なので、それ以降の列も全て 0 にする。
fn packing_fixing(orb: &Orbitope, fixes: &[Vec<Option<u8>>]) -> Option<Vec<(usize, f64)>> {
    let r = orb.vars.len();
    let s = orb.vars[0].len();
    const NONE: usize = usize::MAX;
    let mut lo = vec![NONE; s];
    let mut prev: Option<usize> = None;
    for c in 0..s {
        let start = prev.map_or(0, |p| p + 1);
        lo[c] = (start..r).find(|&i| fixes[c][i] != Some(0)).unwrap_or(NONE);
        if lo[c] == NONE {
            break;
        }
        prev = Some(lo[c]);
    }
    let mut upb = vec![NONE; s];
    for c in (0..s).rev() {
        let own = (0..r).find(|&i| fixes[c][i] == Some(1)).unwrap_or(NONE);
        let next = if c + 1 < s && upb[c + 1] != NONE { upb[c + 1].checked_sub(1)? } else { NONE };
        upb[c] = own.min(next);
    }
    let mut out = Vec::new();
    let mut zero_from = s;
    for c in 0..s {
        if lo[c] == NONE {
            // この列に 1 は置けない: この列と後の列は全て 0。1 に固定された列があれば矛盾
            if upb[c] != NONE {
                return None;
            }
            zero_from = c;
            break;
        }
        if upb[c] != NONE && lo[c] > upb[c] {
            return None;
        }
        for i in 0..lo[c] {
            match fixes[c][i] {
                Some(1) => return None,
                Some(_) => {}
                None => out.push((orb.vars[i][c], 0.0)),
            }
        }
        if upb[c] != NONE && lo[c] == upb[c] && fixes[c][lo[c]].is_none() {
            out.push((orb.vars[lo[c]][c], 1.0));
        }
    }
    for c in zero_from..s {
        for i in 0..r {
            match fixes[c][i] {
                Some(1) => return None,
                Some(_) => {}
                None => out.push((orb.vars[i][c], 0.0)),
            }
        }
    }
    Some(out)
}

/// 対称性を崩す不等式 `x_i - x_k >= 0` の組 `(i, k)` (lex-leader、モジュールの説明を参照)。`skip` の印のある生成元は
/// 使わない (オービトープで扱う)。印のない生成元のうち、オービトープの列を動かすものも使わない (別の順序の対称性の
/// 扱いと混ぜると成り立たないことがある)。
pub fn lex_leader_pairs_except(sym: &Symmetry, skip: &[bool], orbitope_vars: &[bool]) -> Vec<(usize, usize)> {
    let gens: Vec<&Vec<usize>> = sym.generators.iter().enumerate().filter(|&(k, g)| !skip[k] && !(0..g.len()).any(|j| g[j] != j && orbitope_vars[j])).map(|(_, g)| g).collect();
    let n = sym.orbit.len();
    let mut uf: Vec<usize> = (0..n).collect();
    for g in &gens {
        for j in 0..n {
            let (x, y) = (find(&mut uf, j), find(&mut uf, g[j]));
            if x != y {
                uf[x] = y;
            }
        }
    }
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for g in &gens {
        let Some(i) = (0..g.len()).find(|&j| g[j] != j) else { continue };
        if let Some(k) = (0..g.len()).find(|&k| g[k] == i) {
            pairs.push((i, k));
        }
    }
    if let Some(i0) = (0..n).find(|&j| gens.iter().any(|g| g[j] != j)) {
        let r = find(&mut uf, i0);
        for k in i0 + 1..n {
            if find(&mut uf, k) == r {
                pairs.push((i0, k));
            }
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

/// 対称性を崩す不等式 `x_i - x_k >= 0` の組 `(i, k)` (lex-leader、モジュールの説明を参照)。
pub fn lex_leader_pairs(sym: &Symmetry) -> Vec<(usize, usize)> {
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    // (1) 生成元ごと: 最初に動かす番号 i と σ^{-1}(i)
    for g in &sym.generators {
        let Some(i) = (0..g.len()).find(|&j| g[j] != j) else { continue };
        // σ^{-1}(i): g[k] == i となる k
        if let Some(k) = (0..g.len()).find(|&k| g[k] == i) {
            pairs.push((i, k));
        }
    }
    // (2) 最初に動かされる番号 i0 の軌道
    if let Some(i0) = (0..sym.orbit.len()).find(|&j| sym.orbit[j] != usize::MAX) {
        let o = sym.orbit[i0];
        for k in i0 + 1..sym.orbit.len() {
            if sym.orbit[k] == o {
                pairs.push((i0, k));
            }
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inf() -> f64 {
        f64::INFINITY
    }

    #[test]
    fn finds_swap_of_identical_machines() {
        // 2 台の同じ機械 m=0,1 に 2 つの仕事 t=0,1 を割り当てる: x_{t,m} (列 2t+m)
        // 各仕事はどれか 1 台: x_{t,0} + x_{t,1} = 1、各機械の容量: 3 x_{0,m} + 2 x_{1,m} <= 4
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(2, 1.0), (3, 1.0)], vec![(0, 3.0), (2, 2.0)], vec![(1, 3.0), (3, 2.0)]];
        let p = MipProblem::from_rows(vec![0.0; 4], vec![1.0; 4], vec![1.0, 1.0, 2.0, 2.0], 0.0, 1.0, vec![true; 4], rows, vec![1.0, 1.0, -inf(), -inf()], vec![1.0, 1.0, 4.0, 4.0]);
        let sym = detect(&p, 10.0, 100).expect("symmetry");
        assert_eq!(sym.generators.len(), 1);
        assert_eq!(sym.generators[0], vec![1, 0, 3, 2]);
        assert_eq!(sym.num_orbits, 2);
        // 0 番の列 (仕事 0 を機械 0 に) を優先: x_0 >= x_1
        assert_eq!(lex_leader_pairs(&sym), vec![(0, 1)]);
    }

    #[test]
    fn finds_orbitope_for_identical_machines() {
        // 3 台の同じ機械 m に 2 つの仕事 t: 列 3t+m。各仕事はどれか 1 台、各機械の容量
        let mut rows = vec![vec![(0, 1.0), (1, 1.0), (2, 1.0)], vec![(3, 1.0), (4, 1.0), (5, 1.0)]];
        for m in 0..3 {
            rows.push(vec![(m, 3.0), (3 + m, 2.0)]);
        }
        let p = MipProblem::from_rows(vec![0.0; 6], vec![1.0; 6], vec![1.0, 1.0, 1.0, 2.0, 2.0, 2.0], 0.0, 1.0, vec![true; 6], rows, vec![1.0, 1.0, -inf(), -inf(), -inf()], vec![1.0, 1.0, 4.0, 4.0, 4.0]);
        let sym = detect(&p, 10.0, 100).expect("symmetry");
        let (orbs, used) = find_orbitopes(&p, &sym);
        assert_eq!(orbs.len(), 1);
        assert_eq!(orbs[0].vars, vec![vec![0, 1, 2], vec![3, 4, 5]]);
        assert!(used.iter().all(|&u| u));
    }

    #[test]
    fn canonicalize_sorts_columns_with_star_generators() {
        // 3 台の同じ機械 (前のテストと同じ問題)。生成元は (列0 列1) と (列0 列2) の形 (星形)
        let mut rows = vec![vec![(0, 1.0), (1, 1.0), (2, 1.0)], vec![(3, 1.0), (4, 1.0), (5, 1.0)]];
        for m in 0..3 {
            rows.push(vec![(m, 3.0), (3 + m, 2.0)]);
        }
        let p = MipProblem::from_rows(vec![0.0; 6], vec![1.0; 6], vec![1.0, 1.0, 1.0, 2.0, 2.0, 2.0], 0.0, 1.0, vec![true; 6], rows, vec![1.0, 1.0, -inf(), -inf(), -inf()], vec![1.0, 1.0, 4.0, 4.0, 4.0]);
        let sym = detect(&p, 10.0, 100).unwrap();
        let (orbs, _) = find_orbitopes(&p, &sym);
        // 仕事 0 を機械 2、仕事 1 を機械 1 に: 列 (0,0) (0,1) (1,0)
        let mut x = vec![0.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        assert!(p.is_feasible(&x, 1e-9));
        assert!(orbs[0].canonicalize(&mut x));
        // 並べ替え後: 列 0 = (1,0)、列 1 = (0,1)、列 2 = (0,0)
        assert_eq!(x, vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        assert!(p.is_feasible(&x, 1e-9));
    }

    #[test]
    fn orbitopal_fixing_small_cases() {
        // 2 行 3 列。何も固定がなければ: 最大 [11,11,11]、最小 [00,00,00] で固定なし
        let orb = Orbitope { vars: vec![vec![0, 1, 2], vec![3, 4, 5]], packing: false, row_packing: vec![false; 2], swaps: Vec::new() };
        let (lo, up) = (vec![0.0; 6], vec![1.0; 6]);
        assert_eq!(orbitopal_fixing(&orb, &lo, &up), Some(vec![]));
        // x_{0,1} = 1 (列 1 の 1 行目): 列 0 >=_lex 列 1 なので x_{0,0} = 1 に固定される
        let mut lo2 = lo.clone();
        lo2[1] = 1.0;
        let f = orbitopal_fixing(&orb, &lo2, &up).unwrap();
        assert!(f.contains(&(0, 1.0)), "{f:?}");
        // x_{0,0} = 0 かつ x_{0,1} = 1 は矛盾 (列 0 <_lex 列 1)
        let mut up3 = up.clone();
        up3[0] = 0.0;
        assert_eq!(orbitopal_fixing(&orb, &lo2, &up3), None);
    }

    /// 乱数の固定で、orbitopal fixing が「列が辞書式で広義減少で固定を満たす 0-1 行列」を 1 つも切らないこと、
    /// 実行不能と言ったときは本当にないことを総当たりで確かめる。
    #[test]
    fn orbitopal_fixing_brute_force() {
        let mut seed = 7u64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        for _ in 0..2000 {
            let r = 1 + (rnd() * 3.0) as usize;
            let s = 2 + (rnd() * 2.0) as usize;
            let vars: Vec<Vec<usize>> = (0..r).map(|i| (0..s).map(|c| i * s + c).collect()).collect();
            let orb = Orbitope { vars: vars.clone(), packing: false, row_packing: vec![false; r], swaps: Vec::new() };
            let nv = r * s;
            let mut lo = vec![0.0; nv];
            let mut up = vec![1.0; nv];
            for j in 0..nv {
                let t = rnd();
                if t < 0.2 {
                    lo[j] = 1.0;
                } else if t < 0.4 {
                    up[j] = 0.0;
                }
            }
            // 総当たり: 固定を満たし列が辞書式で広義減少の行列
            let mut sols: Vec<Vec<u8>> = Vec::new();
            for mask in 0..(1u32 << nv) {
                let x: Vec<u8> = (0..nv).map(|j| ((mask >> j) & 1) as u8).collect();
                if (0..nv).any(|j| (x[j] as f64) < lo[j] || (x[j] as f64) > up[j]) {
                    continue;
                }
                let col = |c: usize| -> Vec<u8> { (0..r).map(|i| x[vars[i][c]]).collect() };
                if (0..s - 1).all(|c| col(c) >= col(c + 1)) {
                    sols.push(x);
                }
            }
            match orbitopal_fixing(&orb, &lo, &up) {
                None => assert!(sols.is_empty(), "declared infeasible but {} solutions exist", sols.len()),
                Some(fixes) => {
                    assert!(!sols.is_empty(), "feasible declared but no solution");
                    for x in &sols {
                        for &(j, v) in &fixes {
                            assert_eq!(x[j] as f64, v, "fixing x{j}={v} cuts off a valid matrix");
                        }
                    }
                }
            }
        }
    }

    /// パッキング・オービトープ (行の和 <= 1) でも、固定が「行の和 <= 1、列が辞書式で広義減少、固定を満たす」行列を
    /// 切らないことを総当たりで確かめる。
    #[test]
    fn packing_orbitopal_fixing_brute_force() {
        let mut seed = 11u64;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut fixed_total = 0;
        for _ in 0..3000 {
            let r = 1 + (rnd() * 4.0) as usize;
            let s = 2 + (rnd() * 2.0) as usize;
            let vars: Vec<Vec<usize>> = (0..r).map(|i| (0..s).map(|c| i * s + c).collect()).collect();
            let orb = Orbitope { vars: vars.clone(), packing: true, row_packing: vec![true; r], swaps: Vec::new() };
            let nv = r * s;
            let mut lo = vec![0.0; nv];
            let mut up = vec![1.0; nv];
            for j in 0..nv {
                let t = rnd();
                if t < 0.1 {
                    lo[j] = 1.0;
                } else if t < 0.3 {
                    up[j] = 0.0;
                }
            }
            let mut sols: Vec<Vec<u8>> = Vec::new();
            for mask in 0..(1u32 << nv) {
                let x: Vec<u8> = (0..nv).map(|j| ((mask >> j) & 1) as u8).collect();
                if (0..nv).any(|j| (x[j] as f64) < lo[j] || (x[j] as f64) > up[j]) {
                    continue;
                }
                if (0..r).any(|i| (0..s).map(|c| x[vars[i][c]] as u32).sum::<u32>() > 1) {
                    continue;
                }
                let col = |c: usize| -> Vec<u8> { (0..r).map(|i| x[vars[i][c]]).collect() };
                if (0..s - 1).all(|c| col(c) >= col(c + 1)) {
                    sols.push(x);
                }
            }
            match orbitopal_fixing(&orb, &lo, &up) {
                None => assert!(sols.is_empty(), "declared infeasible but {} solutions exist (lo {lo:?} up {up:?})", sols.len()),
                Some(fixes) => {
                    fixed_total += fixes.len();
                    for x in &sols {
                        for &(j, v) in &fixes {
                            assert_eq!(x[j] as f64, v, "fixing x{j}={v} cuts off a valid matrix (r {r} s {s} lo {lo:?} up {up:?})");
                        }
                    }
                }
            }
        }
        assert!(fixed_total > 100);
    }

    #[test]
    fn no_symmetry_when_costs_differ() {
        let rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let p = MipProblem::from_rows(vec![0.0; 2], vec![1.0; 2], vec![1.0, 2.0], 0.0, 1.0, vec![true; 2], rows, vec![1.0], vec![1.0]);
        assert!(detect(&p, 10.0, 100).is_none());
    }

    #[test]
    fn rejects_color_equal_but_non_symmetric() {
        // 色の細分化では区別できない (正則なグラフ) が、置換が行を写さない例でも誤った生成元を返さない:
        // 返したなら必ず検証を通っている
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(1, 1.0), (2, 1.0)], vec![(2, 1.0), (3, 1.0)], vec![(3, 1.0), (0, 1.0)]];
        let p = MipProblem::from_rows(vec![0.0; 4], vec![1.0; 4], vec![1.0; 4], 0.0, 1.0, vec![true; 4], rows, vec![-inf(); 4], vec![1.0; 4]);
        if let Some(sym) = detect(&p, 10.0, 100) {
            let mut counts = std::collections::HashMap::new();
            for i in 0..p.m {
                let mut r = p.rows[i].clone();
                *counts.entry(row_hash(p.row_lo[i], p.row_up[i], &mut r)).or_insert(0i64) += 1;
            }
            for g in &sym.generators {
                assert!(verify(&p, g, &counts));
            }
        }
    }
}
