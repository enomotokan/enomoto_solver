//! 試作 `ENOMOTO_MIP_SYMDYN`・`ENOMOTO_MIP_SYMREFL` (analysis/symdyn_20261110.md、既定では使わない)。
//!
//! - `ENOMOTO_MIP_SYMDYN`: 動的な辞書式の縮小 (dynamic lexicographic reduction、van Doornmalen・Hojny 2022/2024 の
//!   「A Unified Framework for Symmetry Handling」、SCIP 9 の lexred)。各ノードで、根からそのノードまでに分枝した列の
//!   (最初に分枝した) 順を辞書式の比べの順 σ とし、生成元 γ ごとに `σ(x) ≥_lex σ(γ(x))` を伝播する。先頭から両辺が等しく
//!   決まっている間は進み、最初に差が出うる位置で `x_v ≥ γ(x)_v` になるよう境界を締める。その位置で等号が 1 つの値でしか
//!   成り立たず、等号を仮に置くと後ろの位置で破れるなら、等号を除く (整数列なら 1 だけ離す)。子の順は親の順を延ばすだけ
//!   なので、どのノードでも (そのノードの順での) 辞書式で最大の軌道の代表が残る。0-1 以外の整数列・連続列にも使える。
//!   orbital fixing (HiGHS 流、[`super::sym_of`]) の生成元をそのまま使い、orbital fixing の代わりに呼ぶ (同じ群に別の
//!   規則の代表を残す処理を重ねると最適解を全部切ることがあるので、両方は使わない)。オービトープ (動的な orbitopal fixing)
//!   とは、生成元がオービトープの列を動かさない (群が直積に分かれる) ので混ぜてよい。lex-leader の行 (`ENOMOTO_MIP_NO_SYM_OF`)
//!   とは混ぜない (行は列の番号の順の代表を残す)。
//! - `ENOMOTO_MIP_SYMREFL`: 反転を含む対称性 (符号つきの置換 `x_v → lo_v + up_v − x_{g(v)}`、Hojny・Pfetsch・Verschae ほか 2024
//!   「Detecting and Handling Reflection Symmetries in Mixed-Integer (Nonlinear) Programming」) を、列ごとに x と反転した x̄ の
//!   2 列を持つ写し ([`detect_reflections`]) の置換として検出し、確かめ ([`verify_signed`]) を通ったものを上の動的な辞書式の
//!   縮小に足す (`SYMDYN` も入る)。0-1 列 v を自分の反転に写す生成元なら、v を最初に分枝したノードで `x_v ≥ 1 − x_v` から
//!   `x_v = 1` が出る (0 の側の子が刈られる)。
use super::domain::Domain;
use super::problem::MipProblem;
use std::cell::RefCell;
use std::rc::Rc;

/// 符号つきの置換 T: `(T x)_v = r ? (lo_v + up_v − x_b) : x_b` を、動かす位置 (v, b, r) の並び (v の昇順) で持つ。
/// 動かさない位置 (b = v、r = 偽) は持たない。
#[derive(Clone, Debug, PartialEq)]
pub struct SignedGen {
    pub moves: Vec<(u32, u32, bool)>,
}

impl SignedGen {
    pub fn from_perm(g: &[usize]) -> SignedGen {
        SignedGen { moves: (0..g.len()).filter(|&v| g[v] != v).map(|v| (v as u32, g[v] as u32, false)).collect() }
    }
    fn inverse(&self) -> SignedGen {
        let mut moves: Vec<(u32, u32, bool)> = self.moves.iter().map(|&(v, b, r)| (b, v, r)).collect();
        moves.sort_unstable();
        SignedGen { moves }
    }
    pub fn reflects(&self) -> bool {
        self.moves.iter().any(|m| m.2)
    }
    #[inline]
    fn img(&self, v: usize) -> (usize, bool) {
        match self.moves.binary_search_by_key(&(v as u32), |m| m.0) {
            Ok(i) => (self.moves[i].1 as usize, self.moves[i].2),
            Err(_) => (v, false),
        }
    }
}

pub fn symdyn() -> bool {
    env_str!("ENOMOTO_MIP_SYMDYN").is_some_and(|v| v != "0") || symrefl()
}

/// `ENOMOTO_MIP_SYMDYN=o` (値に `o` を含む): 生成元ごとの比べに加えて、順の先頭 v_1..v_{i-1} を動かさない部分群 (Schreier の
/// 生成元で求める、打ち切れば部分群) の軌道 O について `x_{v_i} ≥ x_u` (u ∈ O) を締める (`DynSym::orbital`)。部分群の元 γ は
/// 先頭の位置を動かさないので、`x ≥_lex γ(x)` は位置 v_i の比べ `x_{v_i} ≥ x_{γ^{-1}(v_i)}` を含む。同じ辞書式で最大の代表の
/// 規則の一部なので、生成元ごとの比べと混ぜてよい。推移的な群で v_1 を 0 に分けた子では軌道の全ての列が 0 になる
/// (orbital fixing と同じ強さ。生成元ごとの比べだけでは生成元の数の列しか締まらない、cod105)
pub fn symdyn_orbital() -> bool {
    env_str!("ENOMOTO_MIP_SYMDYN").is_some_and(|v| v.contains('o'))
}

pub fn symrefl() -> bool {
    env_str!("ENOMOTO_MIP_SYMREFL").is_some_and(|v| v != "0")
}

thread_local! {
    /// 反転を含む生成元 ([`symrefl`]。[`super::SYM_OF_GENS`] と同じく外側の求解の終わりに消す)
    pub(crate) static REFL_GENS: RefCell<Option<Rc<Vec<SignedGen>>>> = const { RefCell::new(None) };
}

pub(crate) fn reset() {
    REFL_GENS.with(|t| *t.borrow_mut() = None);
}

pub(crate) fn set_refl(g: Vec<SignedGen>) {
    REFL_GENS.with(|t| *t.borrow_mut() = Some(Rc::new(g)));
}

pub(crate) fn refl_gens() -> Vec<SignedGen> {
    REFL_GENS.with(|t| t.borrow().as_ref().map(|g| (**g).clone())).unwrap_or_default()
}

/// 再スタートの後の問題 `p` で反転の生成元を確かめ直す (前処理が片方の列の境界だけを締めると反転は崩れる)。
pub(crate) fn reverify(p: &MipProblem, verbose: bool) {
    let Some(g) = REFL_GENS.with(|t| t.borrow().clone()) else { return };
    let kept: Vec<SignedGen> = g.iter().filter(|s| s.moves.iter().all(|m| (m.0 as usize) < p.n && (m.1 as usize) < p.n) && verify_signed(p, s)).cloned().collect();
    if verbose {
        eprintln!("MIP: symmetry: SYMREFL after the restart keeps {} of {} reflection generators", kept.len(), g.len());
    }
    set_refl(kept);
}

const EPS: f64 = 1e-9;

fn feq(a: f64, b: f64) -> bool {
    (a - b).abs() <= EPS * (1.0 + a.abs().max(b.abs()))
}

/// 行を (列の昇順の (列, 係数のビット)) のキーにする。先頭の係数が負なら行を -1 倍する (区間も)。
fn row_key(mut e: Vec<(usize, f64)>, lo: f64, up: f64) -> (Vec<(usize, u64)>, f64, f64) {
    e.sort_unstable_by_key(|t| t.0);
    let neg = e.first().is_some_and(|t| t.1 < 0.0);
    let (lo, up) = if neg { (-up, -lo) } else { (lo, up) };
    let key = e.iter().map(|&(j, a)| (j, (if neg { -a } else { a } + 0.0).to_bits())).collect();
    (key, lo, up)
}

/// 符号つきの置換 `s` が `p` の対称性か (実行可能な集合を自分に写し、目的関数を定数の差を除いて保つ) を直接確かめる。
/// 十分条件: 動かす位置で境界・整数性が同じ、反転する位置で境界が有限、`c_v s_v = c_{g(v)}`、各行に T を代入した行が
/// (-1 倍を許して) 同じ区間で `p` の行にある。T は有限位数の全単射なので逆も実行可能な集合を保つ。
pub fn verify_signed(p: &MipProblem, s: &SignedGen) -> bool {
    let n = p.n;
    let mut img: Vec<(usize, bool)> = (0..n).map(|v| (v, false)).collect();
    let mut hit = vec![false; n];
    for &(v, b, r) in &s.moves {
        let (v, b) = (v as usize, b as usize);
        if v >= n || b >= n || hit[b] {
            return false;
        }
        hit[b] = true;
        img[v] = (b, r);
    }
    // 置換であること (動かす位置の像の集合 = 動かす位置の集合)
    for &(v, _, _) in &s.moves {
        if !hit[v as usize] {
            return false;
        }
    }
    for &(v, b, r) in &s.moves {
        let (v, b) = (v as usize, b as usize);
        if p.col_lo[v] != p.col_lo[b] || p.col_up[v] != p.col_up[b] || p.is_int[v] != p.is_int[b] {
            return false;
        }
        if r && !(p.col_lo[v].is_finite() && p.col_up[v].is_finite()) {
            return false;
        }
        let sv = if r { -1.0 } else { 1.0 };
        if !feq(p.cost[v] * sv, p.cost[b]) {
            return false;
        }
    }
    let mut set: std::collections::HashMap<Vec<(usize, u64)>, Vec<(f64, f64)>> = std::collections::HashMap::with_capacity(p.m);
    for i in 0..p.m {
        let (k, lo, up) = row_key(p.rows[i].clone(), p.row_lo[i], p.row_up[i]);
        set.entry(k).or_default().push((lo, up));
    }
    let close = |a: f64, b: f64| a == b || (a.is_finite() && b.is_finite() && feq(a, b));
    for i in 0..p.m {
        let row = &p.rows[i];
        if !row.iter().any(|&(j, _)| img[j] != (j, false)) {
            continue;
        }
        let mut shift = 0.0;
        let mut e = Vec::with_capacity(row.len());
        for &(v, a) in row {
            let (b, r) = img[v];
            if r {
                shift += a * (p.col_lo[v] + p.col_up[v]);
                e.push((b, -a));
            } else {
                e.push((b, a));
            }
        }
        let (k, lo, up) = row_key(e, p.row_lo[i] - shift, p.row_up[i] - shift);
        match set.get(&k) {
            Some(iv) if iv.iter().any(|&(l, u)| close(l, lo) && close(u, up)) => {}
            _ => return false,
        }
    }
    true
}

/// 反転を含む対称性の検出: 有限の境界を持ち固定されていない列 j ごとに反転した列 x̄_j (同じ境界、費用 −c_j) を足し、
/// 各行を中心 ((lo + up)/2) からのずれで書いて x_j に係数 a、x̄_j に −a を置き、区間を中心の分だけずらした行と、
/// その -1 倍の行の両方を置いた写し Q を作る (行の -1 倍は同じ制約)。x_j と x̄_j は (右辺が他と重ならない) 2 列の行で
/// 結ぶので、Q の列の置換は組を組に写し、x_j → x̄_k は「x_j を x_k の反転に写す」と読める。既存の検出
/// ([`super::symmetry::detect`]) で Q の生成元を求め、反転を 1 つ以上含み [`verify_signed`] を通るもの (向きが逆なら逆元) を返す。
pub fn detect_reflections(p: &MipProblem, time_cap: f64, verbose: bool) -> Vec<SignedGen> {
    let t0 = std::time::Instant::now();
    let n = p.n;
    if p.nnz > tunable!("ENOMOTO_T_MIP_SYMREFL_MAX_NNZ", 2_000_000usize, usize) || n == 0 || p.m == 0 {
        return Vec::new();
    }
    let mut rc = vec![usize::MAX; n];
    let mut owner: Vec<usize> = Vec::new();
    for j in 0..n {
        if p.col_lo[j].is_finite() && p.col_up[j].is_finite() && p.col_lo[j] < p.col_up[j] {
            rc[j] = n + owner.len();
            owner.push(j);
        }
    }
    if owner.is_empty() {
        return Vec::new();
    }
    let nq = n + owner.len();
    let mut col_lo = p.col_lo.clone();
    let mut col_up = p.col_up.clone();
    let mut cost = p.cost.clone();
    let mut is_int = p.is_int.clone();
    for &j in &owner {
        col_lo.push(p.col_lo[j]);
        col_up.push(p.col_up[j]);
        cost.push(-p.cost[j] + 0.0);
        is_int.push(p.is_int[j]);
    }
    let center = |j: usize| if rc[j] != usize::MAX { 0.5 * (p.col_lo[j] + p.col_up[j]) } else { 0.0 };
    let mut rows: Vec<Vec<(usize, f64)>> = Vec::with_capacity(2 * p.m + owner.len());
    let (mut lo, mut up) = (Vec::with_capacity(rows.capacity()), Vec::with_capacity(rows.capacity()));
    for i in 0..p.m {
        let mut e: Vec<(usize, f64)> = Vec::with_capacity(2 * p.rows[i].len());
        let mut s = 0.0;
        for &(j, a) in &p.rows[i] {
            e.push((j, a));
            if rc[j] != usize::MAX {
                e.push((rc[j], -a + 0.0));
                s += a * center(j);
            }
        }
        e.sort_unstable_by_key(|t| t.0);
        let (l, u) = (p.row_lo[i] - s + 0.0, p.row_up[i] - s + 0.0);
        let neg: Vec<(usize, f64)> = e.iter().map(|&(j, a)| (j, -a + 0.0)).collect();
        rows.push(e);
        lo.push(l);
        up.push(u);
        rows.push(neg);
        lo.push(-u + 0.0);
        up.push(-l + 0.0);
    }
    const KAPPA: f64 = 0.707_106_781_186_547_6 * 1e7 + 0.123_456_789;
    for &j in &owner {
        rows.push(vec![(j, 1.0), (rc[j], 1.0)]);
        lo.push(KAPPA);
        up.push(KAPPA);
    }
    let q = MipProblem::from_rows(col_lo, col_up, cost, 0.0, 1.0, is_int, rows, lo, up);
    let (sym, _) = super::symmetry::detect(&q, time_cap, tunable!("ENOMOTO_T_MIP_SYMREFL_TRIES", 2000usize, usize), u64::MAX);
    let mut out: Vec<SignedGen> = Vec::new();
    let (mut found, mut bad) = (0usize, 0usize);
    if let Some(sym) = sym {
        for g in &sym.generators {
            if g.len() != nq {
                continue;
            }
            let mut moves = Vec::new();
            let mut ok = true;
            for j in 0..n {
                let t = g[j];
                let (b, r) = if t < n { (t, false) } else { (owner[t - n], true) };
                // 組を組に写すこと (x̄_j の像は x_j の像の相方)
                if rc[j] != usize::MAX {
                    let tb = g[rc[j]];
                    let want = if t < n { rc[t] } else { owner[t - n] };
                    if tb != want {
                        ok = false;
                        break;
                    }
                } else if t >= n {
                    ok = false;
                    break;
                }
                if b != j || r {
                    moves.push((j as u32, b as u32, r));
                }
            }
            if !ok {
                bad += 1;
                continue;
            }
            let s = SignedGen { moves };
            if !s.reflects() {
                continue;
            }
            found += 1;
            if verify_signed(p, &s) {
                out.push(s);
            } else {
                let si = s.inverse();
                if verify_signed(p, &si) {
                    out.push(si);
                } else {
                    bad += 1;
                }
            }
        }
    }
    if verbose {
        let refl_cols: usize = out.iter().map(|s| s.moves.iter().filter(|m| m.2).count()).sum();
        eprintln!("MIP: symmetry: SYMREFL {} reflection generators ({} reflected positions; {found} found, {bad} rejected) on {} of {n} columns ({:.2}s)", out.len(), refl_cols, owner.len(), t0.elapsed().as_secs_f64());
    }
    out
}

/// 動的な辞書式の縮小の生成元 (置換と符号つきの置換) と、列ごとのそれを動かす生成元。
pub struct DynSym {
    gens: Vec<SignedGen>,
    nplain: usize,
    lsum: Vec<f64>,
    moving: Vec<Vec<u32>>,
    /// [`symdyn_orbital`]: 置換の生成元と、順の先頭 (位置の並び) ごとの固定群の生成元・軌道の控え
    orb: Option<OrbState>,
}

struct OrbState {
    og: super::symmetry::OrbitalGens,
    stab: RefCell<std::collections::HashMap<Vec<u32>, Rc<Vec<Rc<Vec<u32>>>>>>,
    orbit: RefCell<std::collections::HashMap<(Vec<u32>, u32), Rc<Vec<usize>>>>,
}

/// 集計: [締めた境界, 締めたノード, 刈ったノード, 等号を除いた (1 だけ離した) 回数]
pub type DynStats = [u64; 4];

impl DynSym {
    pub fn new(p: &MipProblem, plain: &[Vec<usize>], refl: &[SignedGen]) -> Option<DynSym> {
        let mut gens: Vec<SignedGen> = plain.iter().filter(|g| g.len() == p.n).map(|g| SignedGen::from_perm(g)).filter(|s| !s.moves.is_empty()).collect();
        let nplain = gens.len();
        gens.extend(refl.iter().filter(|s| s.moves.iter().all(|m| (m.0 as usize) < p.n && (m.1 as usize) < p.n)).cloned());
        if gens.is_empty() {
            return None;
        }
        let mut moving = vec![Vec::new(); p.n];
        for (k, s) in gens.iter().enumerate() {
            for &(v, _, _) in &s.moves {
                moving[v as usize].push(k as u32);
            }
        }
        let lsum = (0..p.n).map(|j| p.col_lo[j] + p.col_up[j]).collect();
        let plain_ok: Vec<Vec<usize>> = plain.iter().filter(|g| g.len() == p.n && g.iter().enumerate().any(|(j, &t)| j != t)).cloned().collect();
        let orb = (symdyn_orbital() && !plain_ok.is_empty()).then(|| OrbState {
            og: super::symmetry::OrbitalGens::new(plain_ok),
            stab: RefCell::new(std::collections::HashMap::new()),
            orbit: RefCell::new(std::collections::HashMap::new()),
        });
        Some(DynSym { gens, nplain, lsum, moving, orb })
    }

    pub fn counts(&self) -> (usize, usize) {
        (self.nplain, self.gens.len() - self.nplain)
    }

    pub fn refl(&self) -> Vec<SignedGen> {
        self.gens[self.nplain..].to_vec()
    }

    #[inline]
    fn ybounds(&self, lo: &[f64], up: &[f64], v: usize, b: usize, r: bool) -> (f64, f64) {
        if r {
            let l = self.lsum[v];
            (l - up[b], l - lo[b])
        } else {
            (lo[b], up[b])
        }
    }

    /// ノードの順 `order` (分枝した列、最初に分枝した順) で、全ての生成元について `x ≥_lex γ(x)` を伝播する。
    /// 締めたら行の伝播も回し、変わらなくなるまで繰り返す。矛盾なら偽。
    pub fn propagate(&self, p: &MipProblem, dom: &mut Domain, order: &[usize], stats: &mut DynStats) -> bool {
        if order.is_empty() {
            return true;
        }
        let mut cand: Vec<u32> = order.iter().flat_map(|&v| self.moving[v].iter().copied()).collect();
        cand.sort_unstable();
        cand.dedup();
        if cand.is_empty() {
            return true;
        }
        let orbits = self.orbits_for(order);
        let mut node_fixed = false;
        for _pass in 0..tunable!("ENOMOTO_T_MIP_SYMDYN_PASSES", 20usize, usize) {
            let mut changed = 0usize;
            for (i, orb) in &orbits {
                let v = order[*i];
                for &u in orb.iter() {
                    // x_v ≥ x_u
                    if !Self::tighten(p, dom, v, false, dom.lo[u], &mut changed) || !Self::tighten(p, dom, u, true, dom.up[v], &mut changed) {
                        stats[2] += 1;
                        return false;
                    }
                }
            }
            for &k in &cand {
                match self.prop_one(p, dom, order, &self.gens[k as usize], stats) {
                    Some(c) => changed += c,
                    None => {
                        stats[2] += 1;
                        return false;
                    }
                }
            }
            if changed == 0 {
                break;
            }
            stats[0] += changed as u64;
            if !node_fixed {
                node_fixed = true;
                stats[1] += 1;
            }
            if !dom.propagate(p) {
                stats[2] += 1;
                return false;
            }
        }
        true
    }

    /// [`symdyn_orbital`]: 順の位置 i ごとに、先頭 v_1..v_{i-1} を動かさない部分群での v_i の軌道 (v_i を除く列)。
    fn orbits_for(&self, order: &[usize]) -> Vec<(usize, Rc<Vec<usize>>)> {
        let Some(os) = &self.orb else { return Vec::new() };
        let max_orbit = tunable!("ENOMOTO_T_MIP_SYMDYN_ORBIT", 4096usize, usize);
        let max_gens = tunable!("ENOMOTO_T_MIP_SYMDYN_GENS", 64usize, usize).max(os.og.gens.len());
        let memo_cap = tunable!("ENOMOTO_T_MIP_SYMDYN_MEMO", 20000usize, usize);
        let mut stab = os.stab.borrow_mut();
        let mut orbm = os.orbit.borrow_mut();
        if stab.len() > memo_cap {
            stab.clear();
        }
        if orbm.len() > memo_cap {
            orbm.clear();
        }
        let mut h: Rc<Vec<Rc<Vec<u32>>>> = stab.entry(Vec::new()).or_insert_with(|| Rc::new(os.og.base_perms())).clone();
        let mut key: Vec<u32> = Vec::new();
        let mut out = Vec::new();
        for (i, &v) in order.iter().enumerate() {
            if h.is_empty() {
                break;
            }
            let Some(q) = os.og.position(v) else { continue };
            let ok = (key.clone(), q);
            let orb = match orbm.get(&ok) {
                Some(o) => o.clone(),
                None => {
                    let o: Vec<usize> = os.og.orbits_of(&h, |_| true).into_iter().find(|o| o.contains(&v)).map(|o| o.into_iter().filter(|&u| u != v).collect()).unwrap_or_default();
                    let o = Rc::new(o);
                    orbm.insert(ok, o.clone());
                    o
                }
            };
            if !orb.is_empty() {
                out.push((i, orb));
            }
            key.push(q);
            h = match stab.get(&key) {
                Some(g) => g.clone(),
                None => {
                    let g = Rc::new(super::symmetry::OrbitalGens::schreier_stabilizer(&h, q, max_orbit, max_gens));
                    stab.insert(key.clone(), g.clone());
                    g
                }
            };
        }
        out
    }

    fn tighten(p: &MipProblem, dom: &mut Domain, j: usize, upper: bool, v: f64, changed: &mut usize) -> bool {
        dom.set_external_tag(3);
        let r = if upper { dom.tighten_upper(p, j, v) } else { dom.tighten_lower(p, j, v) };
        dom.set_external(false);
        if r {
            *changed += 1;
        }
        !dom.infeasible
    }

    /// 1 つの生成元の伝播。締めた数 (矛盾なら `None`)。
    fn prop_one(&self, p: &MipProblem, dom: &mut Domain, order: &[usize], g: &SignedGen, stats: &mut DynStats) -> Option<usize> {
        let mut changed = 0usize;
        let mut i = 0usize;
        while i < order.len() {
            let v = order[i];
            let (b, r) = g.img(v);
            if b == v && !r {
                i += 1;
                continue;
            }
            let (alo, aup) = (dom.lo[v], dom.up[v]);
            let (ylo, yup) = self.ybounds(&dom.lo, &dom.up, v, b, r);
            if alo == aup && ylo == yup && feq(alo, ylo) {
                i += 1;
                continue;
            }
            if alo > yup + EPS {
                // 真に大きいと決まっている
                return Some(changed);
            }
            // x_v ≥ γ(x)_v を締める
            if b == v {
                // x_v ≥ L − x_v (自分の反転): x_v ≥ L/2
                if !Self::tighten(p, dom, v, false, 0.5 * self.lsum[v], &mut changed) {
                    return None;
                }
            } else {
                if !Self::tighten(p, dom, v, false, ylo, &mut changed) {
                    return None;
                }
                let ok = if r { Self::tighten(p, dom, b, false, self.lsum[v] - dom.up[v], &mut changed) } else { Self::tighten(p, dom, b, true, dom.up[v], &mut changed) };
                if !ok {
                    return None;
                }
            }
            let (alo, aup) = (dom.lo[v], dom.up[v]);
            let (ylo, yup) = self.ybounds(&dom.lo, &dom.up, v, b, r);
            if alo == aup && ylo == yup && feq(alo, ylo) {
                i += 1;
                continue;
            }
            // 等号は値 c = alo = yup でしか成り立たない。仮に置くと後ろで破れるなら等号を除く (整数列のみ)
            if feq(alo, yup) && p.is_int[v] && b != v && (!r || self.lsum[v].fract() == 0.0) && !self.peek_equal_feasible(dom, order, i, g, alo) {
                stats[3] += 1;
                let c = alo;
                if !Self::tighten(p, dom, v, false, c + 1.0, &mut changed) {
                    return None;
                }
                let ok = if r { Self::tighten(p, dom, b, false, self.lsum[v] - c + 1.0, &mut changed) } else { Self::tighten(p, dom, b, true, c - 1.0, &mut changed) };
                if !ok {
                    return None;
                }
            }
            let _ = aup;
            return Some(changed);
        }
        Some(changed)
    }

    /// 位置 `i` で `x_v = γ(x)_v = c` と仮に置いたとき、後ろの位置で `≥_lex` が成り立ちうるか (境界だけで見る。
    /// 偽を返すのは等号が本当に成り立たないときだけ)。
    fn peek_equal_feasible(&self, dom: &Domain, order: &[usize], i: usize, g: &SignedGen, c: f64) -> bool {
        let mut ov: Vec<(usize, f64)> = Vec::new();
        let get = |ov: &Vec<(usize, f64)>, j: usize| -> (f64, f64) {
            match ov.iter().find(|t| t.0 == j) {
                Some(&(_, x)) => (x, x),
                None => (dom.lo[j], dom.up[j]),
            }
        };
        // 列 j を値 x に仮に置く (境界の外や別の値と重なれば偽)
        let put = |ov: &mut Vec<(usize, f64)>, j: usize, x: f64| -> bool {
            let (l, u) = get(ov, j);
            if x < l - EPS || x > u + EPS {
                return false;
            }
            if !ov.iter().any(|t| t.0 == j) {
                ov.push((j, x));
            }
            true
        };
        let v = order[i];
        let (b, r) = g.img(v);
        if !put(&mut ov, v, c) || !put(&mut ov, b, if r { self.lsum[v] - c } else { c }) {
            return false;
        }
        for &v2 in &order[i + 1..] {
            let (b2, r2) = g.img(v2);
            if b2 == v2 && !r2 {
                continue;
            }
            let (alo, aup) = get(&ov, v2);
            let (blo, bup) = get(&ov, b2);
            let (ylo, yup) = if r2 { (self.lsum[v2] - bup, self.lsum[v2] - blo) } else { (blo, bup) };
            if alo == aup && ylo == yup && feq(alo, ylo) {
                continue;
            }
            if aup < ylo - EPS {
                return false;
            }
            if aup > ylo + EPS {
                return true;
            }
            // 等号だけが残る: 値 aup に仮に置いて進む
            let c2 = aup;
            if b2 == v2 {
                if !feq(c2, self.lsum[v2] - c2) || !put(&mut ov, v2, c2) {
                    return false;
                }
            } else if !put(&mut ov, v2, c2) || !put(&mut ov, b2, if r2 { self.lsum[v2] - c2 } else { c2 }) {
                return false;
            }
        }
        true
    }
}

/// ノードの分枝の並び (根から順) から、辞書式の順 (最初に分枝した順、親の固定 `sym` を除く) を作る。
pub fn branch_order(changes: &[super::queue::BoundChange], n: usize) -> Vec<usize> {
    let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::with_capacity(changes.len());
    let mut out = Vec::with_capacity(changes.len());
    for c in changes {
        if !c.sym && c.col < n && seen.insert(c.col) {
            out.push(c.col);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prob(n: usize, rows: Vec<Vec<(usize, f64)>>, lo: Vec<f64>, up: Vec<f64>, cost: Vec<f64>) -> MipProblem {
        MipProblem::from_rows(vec![0.0; n], vec![1.0; n], cost, 0.0, 1.0, vec![true; n], rows, lo, up)
    }

    /// max-cut の形 (y_e <= x_i + x_j, y_e <= 2 − x_i − x_j) は x を全て反転しても変わらない。
    #[test]
    fn symrefl_maxcut_complement_verifies() {
        // 列 0..3: x、列 3..6: y (辺 01, 12, 02)
        let edges = [(0usize, 1usize), (1, 2), (0, 2)];
        let mut rows = Vec::new();
        let (mut lo, mut up) = (Vec::new(), Vec::new());
        for (e, &(i, j)) in edges.iter().enumerate() {
            rows.push(vec![(i, -1.0), (j, -1.0), (3 + e, 1.0)]);
            lo.push(f64::NEG_INFINITY);
            up.push(0.0);
            rows.push(vec![(i, 1.0), (j, 1.0), (3 + e, 1.0)]);
            lo.push(f64::NEG_INFINITY);
            up.push(2.0);
        }
        let p = prob(6, rows, lo, up, vec![0.0, 0.0, 0.0, -1.0, -2.0, -3.0]);
        let s = SignedGen { moves: vec![(0, 0, true), (1, 1, true), (2, 2, true)] };
        assert!(verify_signed(&p, &s));
        let bad = SignedGen { moves: vec![(0, 0, true)] };
        assert!(!verify_signed(&p, &bad));
        let found = detect_reflections(&p, 1.0, false);
        assert!(!found.is_empty());
        assert!(found.iter().all(|s| verify_signed(&p, s)));
    }

    /// 置換 (0 1) で x_0 を先に分枝して 0 にすると、x_0 ≥ x_1 から x_1 = 0。
    #[test]
    fn symdyn_lex_fixes() {
        let rows = vec![vec![(0, 1.0), (1, 1.0)]];
        let p = prob(2, rows, vec![f64::NEG_INFINITY], vec![2.0], vec![1.0, 1.0]);
        let ds = DynSym::new(&p, &[vec![1, 0]], &[]).unwrap();
        let mut dom = Domain::new(&p);
        dom.tighten_upper(&p, 0, 0.0);
        let mut st = [0; 4];
        assert!(ds.propagate(&p, &mut dom, &[0], &mut st));
        assert_eq!(dom.up[1], 0.0);
    }

    /// `SYMDYN=o` の軌道の比べ: 巡回 (0 1 2 3) の生成元 1 個で、x_0 を先に分枝して 0 にすると軌道の全ての列が 0。
    /// 生成元ごとの比べ (x_0 ≥ x_1) だけでは x_1 しか締まらない。
    #[test]
    fn symdyn_orbit_fixes_whole_orbit() {
        let rows = vec![vec![(0, 1.0), (1, 1.0), (2, 1.0), (3, 1.0)]];
        let p = prob(4, rows, vec![f64::NEG_INFINITY], vec![4.0], vec![1.0; 4]);
        let mut ds = DynSym::new(&p, &[vec![1, 2, 3, 0]], &[]).unwrap();
        let mut dom = Domain::new(&p);
        dom.tighten_upper(&p, 0, 0.0);
        let mut st = [0; 4];
        assert!(ds.propagate(&p, &mut dom, &[0], &mut st));
        assert_eq!(dom.up[1], 0.0);
        assert_eq!(dom.up[2], 1.0);
        ds.orb = Some(OrbState {
            og: crate::mip::symmetry::OrbitalGens::new(vec![vec![1, 2, 3, 0]]),
            stab: RefCell::new(std::collections::HashMap::new()),
            orbit: RefCell::new(std::collections::HashMap::new()),
        });
        let mut dom = Domain::new(&p);
        dom.tighten_upper(&p, 0, 0.0);
        assert!(ds.propagate(&p, &mut dom, &[0], &mut st));
        assert!((1..4).all(|j| dom.up[j] == 0.0));
    }
}
