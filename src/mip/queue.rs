//! 未処理ノードの待ち行列 (HiGHS の `HighsNodeQueue` を簡略化したもの)。
//!
//! 下界の順と「下界と推定値の平均」の順の 2 つの順序付き集合でノードを索引する。
//! 通常は後者 (hybrid estimate) で選び、一定回数ごとに下界最小のノードを選ぶ。

use super::lp::Basis;
use std::collections::BTreeSet;
use std::rc::Rc;

/// 全順序つきの f64 (BTreeSet のキー用)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key(pub f64);
impl Eq for Key {}
impl PartialOrd for Key {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Key {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&o.0)
    }
}

/// 境界の変更 1 つ (列, 上限か, 値)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundChange {
    pub col: usize,
    pub upper: bool,
    pub value: f64,
    /// 分枝ではなく、親で orbital fixing ([`super::sym_of`]) が固定したもの (子の固定群を決める分枝に数えない)。
    pub sym: bool,
}

/// 未処理のノード。
#[derive(Clone)]
pub struct OpenNode {
    /// 根からこのノードまでの分枝による境界の変更。
    pub changes: Vec<BoundChange>,
    /// 親の LP から得た下界 (最小化形、定数項込み)。
    pub lower_bound: f64,
    /// 解の推定値 (pseudocost による)。
    pub estimate: f64,
    pub depth: usize,
    /// 親の LP の最適基底 (あれば warm start に使う)。
    pub basis: Option<Rc<Basis>>,
    /// `basis` を保存したときの LP の行の変更の記録の位置 (`Solver::row_log` の長さ)。戻すときにその後の行の追加・削除を
    /// 基底に当てはめる。
    pub basis_epoch: usize,
    /// `basis` の DSE 重み (基底変数の番号との組、`basis_epoch` のときの行の番号)。`ENOMOTO_MIP_NODE_DSE` で保存する。
    pub dse: Option<Rc<NodeDse>>,
    /// このノードを作った分枝 (列, 上向きか, 親の LP 値, 親の LP 目的値)。pseudocost の更新に使う。
    pub branch: Option<(usize, bool, f64, f64)>,
    /// `basis` で効いている (論理変数が非基底の) カットの番号 (`Solver::cut_ids`)。待ち行列にある間、このカットは
    /// 年齢で外さない (外すと取り出したときに基底が合わず LP がほぼ解き直しになる)。
    pub prot: Option<Rc<Vec<u64>>>,
    /// ポートフォリオ並列の木の写し ([`super::solver::TreeSnapshot`]) での、このノードの祖先の番号 (写しの通し番号 << 32 | 写しの
    /// 中のノードの番号)。子は親のものを引き継ぐ。二分探索のスレッドが「この祖先の部分木には t 未満の解がない」と示したら、
    /// 下界を t に上げる。
    pub tags: Option<Rc<Vec<u64>>>,
    /// 強分岐で得た、部分木で成り立つ境界 (`ENOMOTO_MIP_SB_TIGHTEN_INHERIT`) と凸性の直線 (`ENOMOTO_MIP_SB_TIGHTEN_ANCESTOR`)。
    /// 親から子へ引き継ぎ、ノードの処理の始めに定義域に当てる ([`super::solver`] の `apply_sb_inherit`)
    pub sb: Option<Rc<SbInherit>>,
}

/// 待ち行列のノードに保存する DSE 重み ([`OpenNode::dse`])。
#[derive(Clone, Debug)]
pub enum NodeDse {
    /// (基底変数の番号, 重み) の組 (既定)。
    Pairs(Vec<(u32, f32)>),
    /// `ENOMOTO_MIP_MEM2` の `p` (試作): 重みだけを、一緒に保存した基底 ([`Basis`]) の基底変数の番号 (構造列 j、行 n + i) の
    /// 昇順に並べたもの。番号は基底の状態から戻す ([`Self::pairs`])。組の半分の大きさ。
    Packed(Vec<f32>),
    /// `ENOMOTO_MIP_MEMB` の `h` (試作): `Packed` と同じ並びの重みを bf16 (f32 の上 16 ビット、丸めて相対誤差 0.4% 以下) で持つ。
    /// `Packed` の半分の大きさ
    Half(Vec<u16>),
    /// `ENOMOTO_MIP_MEMB` の `q` (試作): `Packed` と同じ並びの重みを 8 ビットの対数 (2^((c − 128)/8)、2^-16〜2^15.9 に丸め、
    /// 相対誤差 4.4% 以下。0 は NaN) で持つ。`Packed` の 1/4 の大きさ
    Byte(Vec<u8>),
}

fn to_log8(x: f32) -> u8 {
    if !(x > 0.0) || x.is_nan() {
        return 0;
    }
    ((x.log2() * 8.0).round() + 128.0).clamp(1.0, 255.0) as u8
}

fn from_log8(c: u8) -> f32 {
    if c == 0 { f32::NAN } else { ((c as f32 - 128.0) / 8.0).exp2() }
}

/// f32 を bf16 に丸める (最近接偶数丸め。NaN はそのまま NaN)。
fn to_bf16(x: f32) -> u16 {
    if x.is_nan() {
        return 0x7fc0;
    }
    let b = x.to_bits();
    let r = b.wrapping_add(0x7fff + ((b >> 16) & 1));
    (r >> 16) as u16
}

fn from_bf16(h: u16) -> f32 {
    f32::from_bits((h as u32) << 16)
}

impl NodeDse {
    /// 持ち物の大きさ [バイト] (目安)。
    pub fn bytes(&self) -> usize {
        match self {
            NodeDse::Pairs(v) => v.len() * 8 + 24,
            NodeDse::Packed(v) => v.len() * 4 + 24,
            NodeDse::Half(v) => v.len() * 2 + 24,
            NodeDse::Byte(v) => v.len() + 24,
        }
    }

    /// `Packed` を `Half` にする (`MEMB` の `h`。ほかはそのまま)。
    pub fn halve(self) -> NodeDse {
        match self {
            NodeDse::Packed(v) => NodeDse::Half(v.into_iter().map(to_bf16).collect()),
            d => d,
        }
    }

    /// `Packed` を `Byte` にする (`MEMB` の `q`。ほかはそのまま)。
    pub fn quarter(self) -> NodeDse {
        match self {
            NodeDse::Packed(v) => NodeDse::Byte(v.into_iter().map(to_log8).collect()),
            d => d,
        }
    }

    /// `Pairs` を `Packed` にする (基底 `b` の基底変数と重みの番号がちょうど合うときだけ。合わなければ `None`)。
    pub fn pack(w: &[(u32, f32)], b: &Basis) -> Option<NodeDse> {
        let n = b.col.len();
        let nt = n + b.row.len();
        let mut map = vec![f32::NAN; nt];
        let mut seen = vec![false; nt];
        for &(v, x) in w {
            let v = v as usize;
            if v >= nt || seen[v] {
                return None;
            }
            seen[v] = true;
            map[v] = x;
        }
        let mut out = Vec::with_capacity(w.len());
        for (j, s) in b.col.iter().chain(b.row.iter()).enumerate() {
            if *s == super::lp::VarStatus::Basic {
                if !seen[j] {
                    return None;
                }
                out.push(map[j]);
            }
        }
        (out.len() == w.len()).then_some(NodeDse::Packed(out))
    }

    /// (基底変数の番号, 重み) の組。`Packed` なら基底 `b` (保存した基底。共有の待ち行列から受け取ったものは、行の後ろに
    /// カットの行の分が足されていてもよい: 重みのない後ろの基底変数は渡さない) から番号を戻す。
    pub fn pairs<'a>(&'a self, b: &Basis) -> std::borrow::Cow<'a, [(u32, f32)]> {
        match self {
            NodeDse::Pairs(v) => std::borrow::Cow::Borrowed(v.as_slice()),
            NodeDse::Packed(w) => {
                let it = b.col.iter().chain(b.row.iter()).enumerate().filter(|(_, s)| **s == super::lp::VarStatus::Basic).map(|(j, _)| j as u32);
                std::borrow::Cow::Owned(it.zip(w.iter().copied()).collect())
            }
            NodeDse::Half(w) => {
                let it = b.col.iter().chain(b.row.iter()).enumerate().filter(|(_, s)| **s == super::lp::VarStatus::Basic).map(|(j, _)| j as u32);
                std::borrow::Cow::Owned(it.zip(w.iter().map(|&h| from_bf16(h))).collect())
            }
            NodeDse::Byte(w) => {
                let it = b.col.iter().chain(b.row.iter()).enumerate().filter(|(_, s)| **s == super::lp::VarStatus::Basic).map(|(j, _)| j as u32);
                std::borrow::Cow::Owned(it.zip(w.iter().map(|&c| from_log8(c))).collect())
            }
        }
    }

    /// 構造列と最初の `m` 行の基底変数の分だけにする (共有の待ち行列に出すとき。`b` は保存した基底)。
    pub fn restrict(&self, b: &Basis, m: usize) -> NodeDse {
        let lim = (b.col.len() + m) as u32;
        match self {
            NodeDse::Pairs(w) => NodeDse::Pairs(w.iter().copied().filter(|&(v, _)| v < lim).collect()),
            NodeDse::Packed(w) => {
                let k = b.col.iter().chain(b.row.iter().take(m)).filter(|s| **s == super::lp::VarStatus::Basic).count();
                NodeDse::Packed(w[..k.min(w.len())].to_vec())
            }
            NodeDse::Half(w) => {
                let k = b.col.iter().chain(b.row.iter().take(m)).filter(|s| **s == super::lp::VarStatus::Basic).count();
                NodeDse::Half(w[..k.min(w.len())].to_vec())
            }
            NodeDse::Byte(w) => {
                let k = b.col.iter().chain(b.row.iter().take(m)).filter(|s| **s == super::lp::VarStatus::Basic).count();
                NodeDse::Byte(w[..k.min(w.len())].to_vec())
            }
        }
    }
}

/// `ENOMOTO_MIP_MEMC` (試作、analysis/memcap_c_20261104.md): 境界の変更の並びを u32 の並びに詰める (戻すと元と同じ並び)。
/// 1 つの変更は 1 語 (列 << 4 | 上限か << 3 | sym << 2 | 値の種類) で、値の種類は 0 = 0.0、1 = 1.0、2 = 後ろの 2 語に f64 のビット。
/// 2 値の列の固定 (orbital fixing の固定が子へ積み重なる supportcase18 では 1 ノード数百個) は 24 → 4 バイトになる。
/// 列の番号が 2^28 以上なら `None` (詰めない)。
pub fn pack_changes(ch: &[BoundChange]) -> Option<Vec<u32>> {
    let mut out = Vec::with_capacity(ch.len());
    for c in ch {
        if c.col >= 1 << 28 {
            return None;
        }
        let vb = c.value.to_bits();
        let vk = if vb == 0.0f64.to_bits() {
            0
        } else if vb == 1.0f64.to_bits() {
            1
        } else {
            2
        };
        out.push((c.col as u32) << 4 | (c.upper as u32) << 3 | (c.sym as u32) << 2 | vk);
        if vk == 2 {
            out.push(vb as u32);
            out.push((vb >> 32) as u32);
        }
    }
    out.shrink_to_fit();
    Some(out)
}

/// [`pack_changes`] で詰めた並びを戻す。
pub fn unpack_changes(w: &[u32]) -> Vec<BoundChange> {
    let mut out = Vec::with_capacity(w.len());
    let mut i = 0;
    while i < w.len() {
        let x = w[i];
        i += 1;
        let value = match x & 3 {
            0 => 0.0,
            1 => 1.0,
            _ => {
                let b = w[i] as u64 | (w[i + 1] as u64) << 32;
                i += 2;
                f64::from_bits(b)
            }
        };
        out.push(BoundChange { col: (x >> 4) as usize, upper: x & 8 != 0, value, sym: x & 4 != 0 });
    }
    out
}

/// `ENOMOTO_MIP_MEMC` (試作、analysis/memcap_c_20261104.md): 共有の待ち行列に出すノード (`SnapNode`) に付ける基底 (列と元の行の状態) を
/// 詰めた形。列が多く行が少ない問題 (supportcase18: 列 13410・行 240) では、状態の並び (1 変数 1 バイト) がノードの持ち物の大半に
/// なる。非基底で下限にある列がほとんどなので、列と行それぞれで最も多い状態を既定にして、それ以外の (番号, 状態) だけを持つ
/// (`Sparse`、1 つ 5 バイト)。例外が多いときは 1 変数 2 ビットの並び (`Bits`) にする (小さい方)。
#[derive(Clone, Debug)]
pub enum SparseBasis {
    Sparse { n: u32, m: u32, dcol: u8, drow: u8, idx: Vec<u32>, st: Vec<u8> },
    Bits { n: u32, m: u32, bits: Vec<u8> },
}

fn vs_code(s: super::lp::VarStatus) -> u8 {
    use super::lp::VarStatus::*;
    match s {
        Basic => 0,
        Lower => 1,
        Upper => 2,
        Zero => 3,
    }
}

fn vs_of(c: u8) -> super::lp::VarStatus {
    use super::lp::VarStatus::*;
    match c & 3 {
        0 => Basic,
        1 => Lower,
        2 => Upper,
        _ => Zero,
    }
}

impl SparseBasis {
    /// 列の状態 `col` と行の状態 `row` を詰める。
    pub fn encode(col: &[super::lp::VarStatus], row: &[super::lp::VarStatus]) -> SparseBasis {
        let most = |v: &[super::lp::VarStatus]| -> u8 {
            let mut c = [0usize; 4];
            for &s in v {
                c[vs_code(s) as usize] += 1;
            }
            (0..4u8).max_by_key(|&k| c[k as usize]).unwrap_or(1)
        };
        let (dcol, drow) = (most(col), most(row));
        let n = col.len();
        let exc = col.iter().filter(|&&s| vs_code(s) != dcol).count() + row.iter().filter(|&&s| vs_code(s) != drow).count();
        let nt = n + row.len();
        if exc * 5 <= nt.div_ceil(4) {
            let mut idx = Vec::with_capacity(exc);
            let mut st = Vec::with_capacity(exc);
            for (j, &s) in col.iter().enumerate() {
                if vs_code(s) != dcol {
                    idx.push(j as u32);
                    st.push(vs_code(s));
                }
            }
            for (i, &s) in row.iter().enumerate() {
                if vs_code(s) != drow {
                    idx.push((n + i) as u32);
                    st.push(vs_code(s));
                }
            }
            SparseBasis::Sparse { n: n as u32, m: row.len() as u32, dcol, drow, idx, st }
        } else {
            let mut bits = vec![0u8; nt.div_ceil(4)];
            for (j, &s) in col.iter().chain(row.iter()).enumerate() {
                bits[j >> 2] |= vs_code(s) << ((j & 3) * 2);
            }
            SparseBasis::Bits { n: n as u32, m: row.len() as u32, bits }
        }
    }

    /// 詰めた基底を (列の状態, 行の状態) に戻す。
    pub fn decode(&self) -> (Vec<super::lp::VarStatus>, Vec<super::lp::VarStatus>) {
        match self {
            SparseBasis::Sparse { n, m, dcol, drow, idx, st } => {
                let n = *n as usize;
                let mut col = vec![vs_of(*dcol); n];
                let mut row = vec![vs_of(*drow); *m as usize];
                for (&j, &s) in idx.iter().zip(st.iter()) {
                    let j = j as usize;
                    if j < n {
                        col[j] = vs_of(s);
                    } else {
                        row[j - n] = vs_of(s);
                    }
                }
                (col, row)
            }
            SparseBasis::Bits { n, m, bits } => {
                let n = *n as usize;
                let nt = n + *m as usize;
                let all: Vec<super::lp::VarStatus> = (0..nt).map(|j| vs_of(bits[j >> 2] >> ((j & 3) * 2))).collect();
                let row = all[n..].to_vec();
                let mut col = all;
                col.truncate(n);
                (col, row)
            }
        }
    }

    /// 持ち物の大きさ [バイト] (目安)。
    pub fn bytes(&self) -> usize {
        match self {
            SparseBasis::Sparse { idx, st, .. } => idx.capacity() * 4 + st.capacity() + 64,
            SparseBasis::Bits { bits, .. } => bits.capacity() + 48,
        }
    }
}

/// 強分岐の子の LP 値から得た凸性の直線 (列 `col` の上の子 (`up`) か下の子)。祖先のノードの LP 値 `z`・列の値 `v`・1 単位あたりの
/// 増分 `per` から、x_j <= t (下) の LP の値は z + (v − t)·per 以上 (上なら x_j >= t で z + (t − v)·per 以上)。子孫のノードの LP は
/// 祖先より制約が多いので、子孫でもこの下界は成り立つ
#[derive(Clone, Copy, Debug)]
pub struct SbLine {
    pub col: u32,
    pub up: bool,
    pub z: f64,
    pub v: f64,
    pub per: f64,
}

impl SbLine {
    /// 直線が下界として成り立つ端 (下の子の直線なら t <= ⌊v⌋、上なら t >= ⌈v⌉)
    pub fn end(&self) -> f64 {
        if self.up { self.v.ceil() } else { self.v.floor() }
    }

    /// 端 `t` での値
    pub fn at(&self, t: f64) -> f64 {
        if self.up { self.z + (t - self.v) * self.per } else { self.z + (self.v - t) * self.per }
    }

    /// `self` が `o` を支配するか (`o` が成り立つ範囲の全体で `self` も成り立ち、値が `o` 以上): その範囲は半直線なので、
    /// 端が `o` 以上に広く、傾きが `o` 以上で、`o` の端での値が `o` 以上なら足りる
    fn dominates(&self, o: &SbLine) -> bool {
        let wider = if self.up { self.end() <= o.end() } else { self.end() >= o.end() };
        wider && self.per >= o.per && self.at(o.end()) >= o.at(o.end()) - 1e-9 * (1.0 + o.at(o.end()).abs())
    }
}

/// 強分岐で得て子に引き継ぐもの ([`OpenNode::sb`])。
#[derive(Clone, Debug, Default)]
pub struct SbInherit {
    /// 強分岐の片側の打ち切りによる固定と凸性による締め付け (このノードの部分木で成り立つ)
    pub fixes: Vec<BoundChange>,
    /// 列と向きごとの直線の最大 (凸な折れ線、祖先のものも含む)。支配された直線は持たず、1 本の折れ線は最大
    /// `ENOMOTO_T_MIP_SB_POLY_MAX` 本の直線 (超えたら古いものから捨てる)。折れ線の数は列の数 × 2 以下
    pub polys: std::collections::BTreeMap<(u32, bool), Vec<SbLine>>,
}

impl SbInherit {
    /// 直線を折れ線に足す (支配されていれば足さない、足した直線が支配するものは消す)。足したら真
    pub fn insert(&mut self, l: SbLine, max: usize) -> bool {
        let poly = self.polys.entry((l.col, l.up)).or_default();
        if poly.iter().any(|o| o.dominates(&l)) {
            return false;
        }
        poly.retain(|o| !l.dominates(o));
        poly.push(l);
        if poly.len() > max {
            poly.remove(0);
        }
        true
    }
}

/// 待ち行列。
pub struct NodeQueue {
    nodes: Vec<Option<OpenNode>>,
    free: Vec<usize>,
    by_lb: BTreeSet<(Key, usize)>,
    by_est: BTreeSet<(Key, Key, usize)>,
    pops: u64,
    /// 枠ごとの世代 (push のたびに増やす。取り出した枠が使い回されても、古い (枠, 世代) では取り出せない)。
    gen: Vec<u64>,
    /// カットの番号ごとの、そのカットが効いている基底を持つ待ち行列のノードの数 (カットの番号は 0 からの連番なので
    /// 番号で引く。足りなければ伸ばす)。
    protect: Vec<u32>,
    /// 診断用 (`ENOMOTO_MIP_BRANCH_REPLAY`): `estimate` をそのまま優先度にし、常にその最小を取り出す
    pub prio_only: bool,
    /// `ENOMOTO_MIP_MEMB` の `c` (試作): 入れたノードの基底を 2 ビットに詰めて (`packed`) 持ち、取り出すときに戻す
    pub pack_basis: bool,
    /// 枠ごとの詰めた基底 (`pack_basis` のとき。ノードの `basis` は `None` にしておく)
    packed: Vec<Option<Rc<PackedBasis>>>,
    /// 最後に詰めた基底 (兄弟は同じ `Rc<Basis>` を持つので、続けて入れたら詰めたものを共有する)
    pack_last: Option<(std::rc::Weak<Basis>, Rc<PackedBasis>)>,
    /// 試作 `ENOMOTO_MIP_GAP1` の `q` (analysis/gap1_tree_20261109.md): 列ごとの、待ち行列のノードの境界の変更のうち
    /// 下向き (`[0]`、上限を下げる) と上向き (`[1]`) の数 (HiGHS の `HighsNodeQueue::numNodesDown/Up`)。`q` がなければ空
    pub col_cnt: Vec<[u32; 2]>,
}

/// `ENOMOTO_MIP_MEMB` の `c` (試作): 2 ビットに詰めた基底 (列, 行)。
pub type PackedBasis = (super::lp::PackedStatus, super::lp::PackedStatus);

impl NodeQueue {
    pub fn new() -> Self {
        NodeQueue { nodes: Vec::new(), free: Vec::new(), by_lb: BTreeSet::new(), by_est: BTreeSet::new(), pops: 0, gen: Vec::new(), protect: Vec::new(), prio_only: false, pack_basis: super::solver::memb('c'), packed: Vec::new(), pack_last: None, col_cnt: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.by_lb.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_lb.is_empty()
    }

    /// ノードを入れ、その (枠, 世代) を返す ([`Self::take`] で取り出すのに使う)。
    pub fn push(&mut self, mut node: OpenNode) -> (usize, u64) {
        let pk = if self.pack_basis { node.basis.take().map(|b| self.pack_of(&b)) } else { None };
        let lb = node.lower_bound;
        let est = self.est_of(&node);
        let depth = node.depth as f64;
        if let Some(pr) = &node.prot {
            for &c in pr.iter() {
                let c = c as usize;
                if c >= self.protect.len() {
                    self.protect.resize(c + 1, 0);
                }
                self.protect[c] += 1;
            }
        }
        if super::pseudocost::gap1().contains('q') {
            self.count_cols(&node.changes, true);
        }
        let id = if let Some(id) = self.free.pop() {
            self.nodes[id] = Some(node);
            id
        } else {
            self.nodes.push(Some(node));
            self.gen.push(0);
            self.nodes.len() - 1
        };
        if self.pack_basis || !self.packed.is_empty() {
            if self.packed.len() < self.nodes.len() {
                self.packed.resize(self.nodes.len(), None);
            }
            self.packed[id] = pk;
        }
        self.gen[id] += 1;
        self.by_lb.insert((Key(lb), id));
        self.by_est.insert((Key(est), Key(-depth), id));
        (id, self.gen[id])
    }

    /// `push` が返した (枠, 世代) のノードがまだあり、下界が `limit` 以下なら取り出す。
    /// (枠, 世代) のノードがまだ待ち行列にあるか。
    pub fn holds(&self, id: usize, gen: u64) -> bool {
        id < self.nodes.len() && self.gen[id] == gen && self.nodes[id].is_some()
    }

    pub fn take(&mut self, id: usize, gen: u64, limit: f64) -> Option<OpenNode> {
        if id >= self.nodes.len() || self.gen[id] != gen {
            return None;
        }
        let n = self.nodes[id].as_ref()?;
        if n.lower_bound > limit {
            return None;
        }
        Some(self.remove(id))
    }

    /// 基底を詰める (直前に詰めたものと同じ `Rc` なら共有する)。
    fn pack_of(&mut self, b: &Rc<Basis>) -> Rc<PackedBasis> {
        if let Some((w, p)) = &self.pack_last {
            if w.upgrade().is_some_and(|x| Rc::ptr_eq(&x, b)) {
                return p.clone();
            }
        }
        let p = Rc::new((super::lp::PackedStatus::pack(&b.col), super::lp::PackedStatus::pack(&b.row)));
        self.pack_last = Some((Rc::downgrade(b), p.clone()));
        p
    }

    /// 詰めた基底の持ち物の大きさの目安 [バイト] (共有の数で割る)。
    pub fn packed_bytes(&self) -> usize {
        self.packed.iter().flatten().map(|p| (p.0.bytes() + p.1.bytes()) / Rc::strong_count(p)).sum()
    }

    /// `col_cnt` を足す・引く (`ENOMOTO_MIP_GAP1` の `q`)
    fn count_cols(&mut self, ch: &[BoundChange], add: bool) {
        for c in ch {
            if c.col >= self.col_cnt.len() {
                self.col_cnt.resize(c.col + 1, [0; 2]);
            }
            let k = &mut self.col_cnt[c.col][(!c.upper) as usize];
            *k = if add { *k + 1 } else { k.saturating_sub(1) };
        }
    }

    /// 待ち行列のノードで列 `j` が (下向き, 上向き) に変更された数 (`ENOMOTO_MIP_GAP1` の `q`)
    pub fn col_count(&self, j: usize) -> (u32, u32) {
        self.col_cnt.get(j).map_or((0, 0), |c| (c[0], c[1]))
    }

    fn remove(&mut self, id: usize) -> OpenNode {
        let mut node = self.nodes[id].take().unwrap();
        if !self.col_cnt.is_empty() {
            let ch = std::mem::take(&mut node.changes);
            self.count_cols(&ch, false);
            node.changes = ch;
        }
        if let Some(p) = self.packed.get_mut(id).and_then(|x| x.take()) {
            node.basis = Some(Rc::new(Basis { col: p.0.unpack(), row: p.1.unpack() }));
        }
        if let Some(pr) = node.prot.take() {
            self.unprotect(&pr);
        }
        let est = self.est_of(&node);
        self.by_lb.remove(&(Key(node.lower_bound), id));
        self.by_est.remove(&(Key(est), Key(-(node.depth as f64)), id));
        self.free.push(id);
        node
    }

    fn est_of(&self, n: &OpenNode) -> f64 {
        if self.prio_only {
            n.estimate
        } else {
            0.5 * n.lower_bound + 0.5 * n.estimate
        }
    }

    /// `prio_only` の優先度が最小のノードの (枠, 世代, 優先度)。
    pub fn first_prio(&self) -> Option<(usize, u64, f64)> {
        self.by_est.iter().next().map(|&(k, _, id)| (id, self.gen[id], k.0))
    }

    /// 最小の下界 (空なら +inf)。
    pub fn best_lower_bound(&self) -> f64 {
        self.by_lb.iter().next().map(|&(k, _)| k.0).unwrap_or(f64::INFINITY)
    }

    /// 次のノードを取り出す。`bb_every` 回に 1 回は下界最小、それ以外は hybrid estimate 最小。
    pub fn pop(&mut self, bb_every: u64) -> Option<OpenNode> {
        self.pops += 1;
        let id = if self.pops % bb_every.max(1) == 0 && !self.prio_only {
            self.by_lb.iter().next().map(|&(_, id)| id)
        } else {
            self.by_est.iter().next().map(|&(_, _, id)| id)
        }?;
        Some(self.remove(id))
    }

    /// 下界最小のノードを取り出す。
    /// 次の [`Self::pop`] が下界最小を取る番か (`ENOMOTO_MIP_TREE1_POOL`)。
    pub fn next_is_best(&self, bb_every: u64) -> bool {
        !self.prio_only && (self.pops + 1) % bb_every.max(1) == 0
    }

    /// [`Self::pop`] の番を 1 つ進める (取り出しを別の待ち行列から行ったとき。`ENOMOTO_MIP_TREE1_POOL`)。
    pub fn skip_pop(&mut self) {
        self.pops += 1;
    }

    pub fn pop_best(&mut self) -> Option<OpenNode> {
        let id = self.by_lb.iter().next().map(|&(_, id)| id)?;
        Some(self.remove(id))
    }

    /// `ENOMOTO_MIP_POOL1_DONATE` (試作): 共有の待ち行列に出すノードを 1 つ取り出す。`mode` は 0 = 下界最小 (既定と同じ)、
    /// 1 = 下界最大、2 = 「下界と推定値の平均」が最大 (自分の選び方で最後に選ぶもの)、3 = 最も深い (同じ深さなら下界の大きいもの、
    /// 全ノードを走査する)、4 = 最も浅い (同じ深さなら下界の大きいもの)。
    pub fn pop_for_donate(&mut self, mode: u8) -> Option<OpenNode> {
        let id = match mode {
            1 => self.by_lb.iter().next_back().map(|&(_, id)| id),
            2 => self.by_est.iter().next_back().map(|&(_, _, id)| id),
            3 | 4 => self
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(i, n)| n.as_ref().map(|n| (i, n.depth, n.lower_bound)))
                .max_by(|a, b| if mode == 3 { a.1.cmp(&b.1) } else { b.1.cmp(&a.1) }.then(a.2.total_cmp(&b.2)))
                .map(|x| x.0),
            _ => self.by_lb.iter().next().map(|&(_, id)| id),
        }?;
        Some(self.remove(id))
    }

    /// 下界が `cutoff` 以上のノードを捨てる。
    pub fn prune(&mut self, cutoff: f64) {
        // 下界の小さい順に外す (外した枠を使い回す順を以前と同じにする)
        while let Some(&(_, id)) = self.by_lb.range((Key(cutoff), 0)..).next() {
            self.remove(id);
        }
    }

    /// `keep` の (枠, 世代) 以外のノードを下界の小さい順にすべて取り出す。
    pub fn take_all_except(&mut self, keep: &[(usize, u64)]) -> Vec<OpenNode> {
        let ids: Vec<usize> = self.by_lb.iter().map(|&(_, id)| id).filter(|&id| !keep.iter().any(|&(k, g)| k == id && self.gen[id] == g)).collect();
        ids.into_iter().map(|id| self.remove(id)).collect()
    }

    /// ノードをすべて取り出す (枠の世代は保つので、取り出す前の (枠, 世代) ではもう取り出せない)。
    pub fn take_all(&mut self) -> Vec<OpenNode> {
        let ids: Vec<usize> = self.by_lb.iter().map(|&(_, id)| id).collect();
        ids.into_iter().map(|id| self.remove(id)).collect()
    }

    /// 待ち行列のノード (順不同)。
    pub fn iter(&self) -> impl Iterator<Item = &OpenNode> {
        self.nodes.iter().flatten()
    }

    /// 保存している基底を捨てる (メモリ節約)。
    pub fn drop_bases(&mut self) {
        let mut prs = Vec::new();
        self.packed.iter_mut().for_each(|p| *p = None);
        for n in self.nodes.iter_mut().flatten() {
            n.basis = None;
            if let Some(pr) = n.prot.take() {
                prs.push(pr);
            }
        }
        for pr in prs {
            self.unprotect(&pr);
        }
    }

    /// `ENOMOTO_MIP_MEM2` (試作): 待ち行列のノードの持ち物 (基底・DSE 重み・効くカットの番号・境界の変更) の大きさの目安 [バイト]。
    /// 兄弟で共有しているもの (`Rc`) は共有の数で割る。
    pub fn mem_bytes(&self) -> usize {
        self.nodes.iter().flatten().map(node_bytes).sum::<usize>() + self.packed_bytes()
    }

    /// `ENOMOTO_MIP_MEM2` (試作): 持ち物を減らす。深いノード (同じ深さなら下界の大きいもの) から、`dse` なら DSE 重みを、
    /// `basis` なら基底 (と効くカットの番号) も捨て、減った大きさの目安が `target` に届いたらやめる。`keep` の (枠, 世代)
    /// (潜りの兄弟) は捨てない。減った大きさの目安を返す。
    pub fn strip(&mut self, target: usize, basis: bool, keep: &[(usize, u64)]) -> usize {
        let mut ids: Vec<(usize, usize, f64)> = Vec::new();
        for (id, n) in self.nodes.iter().enumerate() {
            if let Some(n) = n {
                let has_b = n.basis.is_some() || self.packed.get(id).is_some_and(|p| p.is_some());
                if (n.dse.is_some() || (basis && has_b)) && !keep.iter().any(|&(k, g)| k == id && self.gen[id] == g) {
                    ids.push((id, n.depth, n.lower_bound));
                }
            }
        }
        ids.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.total_cmp(&a.2)));
        let mut freed = 0usize;
        for (id, _, _) in ids {
            if freed >= target {
                break;
            }
            let pb = |q: &Self| q.packed.get(id).and_then(|p| p.as_ref()).map_or(0, |p| (p.0.bytes() + p.1.bytes()) / Rc::strong_count(p));
            let before = node_bytes(self.nodes[id].as_ref().unwrap()) + pb(self);
            let n = self.nodes[id].as_mut().unwrap();
            n.dse = None;
            let mut pr = None;
            if basis {
                n.basis = None;
                pr = n.prot.take();
                if let Some(p) = self.packed.get_mut(id) {
                    *p = None;
                }
            }
            let before = before - pb(self);
            freed += before.saturating_sub(node_bytes(self.nodes[id].as_ref().unwrap()));
            if let Some(pr) = pr {
                self.unprotect(&pr);
            }
        }
        freed
    }

    /// `ENOMOTO_MIP_MEMCAP` (試作): 持ち物を数でなく割合で減らす。`basis` でなければ DSE 重みを持つノード、`basis` なら DSE 重みか
    /// 基底を持つノードのうち、深いもの (同じ深さなら下界の大きいもの) から `frac` の割合 (切り上げ) の持ち物を捨てる。`keep` の
    /// (枠, 世代) (潜りの兄弟) は捨てない。(持ち物を捨てたノードの数, 捨てる前に対象だったノードの数) を返す。
    pub fn strip_frac(&mut self, frac: f64, basis: bool, keep: &[(usize, u64)]) -> (usize, usize) {
        let mut ids: Vec<(usize, usize, f64)> = Vec::new();
        for (id, n) in self.nodes.iter().enumerate() {
            if let Some(n) = n {
                let has_b = n.basis.is_some() || self.packed.get(id).is_some_and(|p| p.is_some());
                if (n.dse.is_some() || (basis && has_b)) && !keep.iter().any(|&(k, g)| k == id && self.gen[id] == g) {
                    ids.push((id, n.depth, n.lower_bound));
                }
            }
        }
        let total = ids.len();
        let want = ((total as f64 * frac).ceil() as usize).min(total);
        if want < total {
            ids.select_nth_unstable_by(want, |a, b| b.1.cmp(&a.1).then(b.2.total_cmp(&a.2)));
        }
        let mut prs = Vec::new();
        for &(id, _, _) in &ids[..want] {
            let n = self.nodes[id].as_mut().unwrap();
            n.dse = None;
            if basis {
                n.basis = None;
                if let Some(pr) = n.prot.take() {
                    prs.push(pr);
                }
                // `ENOMOTO_MIP_MEMB` の `c`: 詰めた基底も捨てる
                if let Some(p) = self.packed.get_mut(id) {
                    *p = None;
                }
            }
        }
        for pr in prs {
            self.unprotect(&pr);
        }
        (want, total)
    }

    fn unprotect(&mut self, pr: &[u64]) {
        for c in pr {
            if let Some(k) = self.protect.get_mut(*c as usize) {
                *k = k.saturating_sub(1);
            }
        }
    }

    /// カット `c` が待ち行列のノードの基底で効いているか。
    pub fn is_protected(&self, c: u64) -> bool {
        self.protect.get(c as usize).is_some_and(|&k| k > 0)
    }
}

/// `ENOMOTO_MIP_MEM2` (試作): ノード 1 つの持ち物の大きさの目安 [バイト] (`Rc` で共有しているものは共有の数で割る)。
/// 診断用 (`ENOMOTO_MIP_MEMB_DIAG`): 待ち行列のノードの持ち物を種類ごとに数える [ノード数, 境界の変更, 基底, DSE 重み, 効くカットの番号,
/// 強分岐の引き継ぎ, 祖先の番号, 枠の数] (バイト、`Rc` で共有しているものは共有の数で割る)。
pub fn diag_bytes<'a>(it: impl Iterator<Item = &'a OpenNode>) -> [usize; 8] {
    let mut r = [0usize; 8];
    for n in it {
        r[0] += 1;
        r[1] += n.changes.capacity() * std::mem::size_of::<BoundChange>();
        if let Some(x) = &n.basis {
            r[2] += (x.col.capacity() + x.row.capacity()) / Rc::strong_count(x);
        }
        if let Some(x) = &n.dse {
            r[3] += x.bytes() / Rc::strong_count(x);
        }
        if let Some(x) = &n.prot {
            r[4] += x.capacity() * 8 / Rc::strong_count(x);
        }
        if let Some(x) = &n.sb {
            let b = x.fixes.capacity() * std::mem::size_of::<BoundChange>() + x.polys.values().map(|v| 48 + v.capacity() * std::mem::size_of::<SbLine>()).sum::<usize>();
            r[5] += b / Rc::strong_count(x);
        }
        if let Some(x) = &n.tags {
            r[6] += x.capacity() * 8 / Rc::strong_count(x);
        }
    }
    r
}

impl NodeQueue {
    /// 診断用: 枠の数 (使っていない枠を含む)。
    pub fn slots(&self) -> usize {
        self.nodes.len()
    }
}

pub fn node_bytes(n: &OpenNode) -> usize {
    let mut b = std::mem::size_of::<OpenNode>() + n.changes.capacity() * std::mem::size_of::<BoundChange>();
    if let Some(x) = &n.basis {
        b += (x.col.len() + x.row.len()) / Rc::strong_count(x);
    }
    if let Some(x) = &n.dse {
        b += x.bytes() / Rc::strong_count(x);
    }
    if let Some(x) = &n.prot {
        b += x.len() * 8 / Rc::strong_count(x);
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mip::lp::{PackedStatus, VarStatus};

    /// `ENOMOTO_MIP_MEMB` の `c`・`h` (試作): 2 ビットに詰めた基底は元に戻り、bf16 の重みは相対誤差 0.4% 以下で戻る。
    #[test]
    fn memb_pack_roundtrip() {
        let st = [VarStatus::Basic, VarStatus::Lower, VarStatus::Upper, VarStatus::Zero];
        for len in [0usize, 1, 3, 4, 5, 17, 1000] {
            let v: Vec<VarStatus> = (0..len).map(|i| st[(i * 7 + i / 3) % 4]).collect();
            let p = PackedStatus::pack(&v);
            assert_eq!(p.unpack(), v);
        }
        let col = vec![VarStatus::Basic, VarStatus::Lower, VarStatus::Basic];
        let row = vec![VarStatus::Upper, VarStatus::Basic];
        let b = Basis { col, row };
        let w = vec![(0u32, 1.2345f32), (2, 1e-7), (4, 3.0e5)];
        let d = NodeDse::pack(&w, &b).unwrap().halve();
        assert!(matches!(d, NodeDse::Half(_)));
        // 8 ビットの対数は 2^-16〜2^15.9 の範囲で相対誤差 4.4% 以下 (範囲の外は端に丸める)
        let wq = vec![(0u32, 1.2345f32), (2, 1e-3), (4, 3.0e4)];
        let q = NodeDse::pack(&wq, &b).unwrap().quarter();
        for (a, c) in wq.iter().zip(q.pairs(&b).iter()) {
            assert!(((a.1 - c.1) / a.1).abs() <= 0.045, "{} {}", a.1, c.1);
        }
        let q = NodeDse::pack(&w, &b).unwrap().quarter();
        assert_eq!(q.pairs(&b)[1].1, (-127.0f32 / 8.0).exp2());
        let back = d.pairs(&b);
        assert_eq!(back.len(), 3);
        for (a, c) in w.iter().zip(back.iter()) {
            assert_eq!(a.0, c.0);
            assert!(((a.1 - c.1) / a.1).abs() <= 0.004, "{} {}", a.1, c.1);
        }
    }

    /// 詰めた基底は待ち行列から取り出すと元の基底に戻る。
    #[test]
    fn memb_queue_pack_basis() {
        let mut q = NodeQueue::new();
        q.pack_basis = true;
        let b = Rc::new(Basis { col: vec![VarStatus::Basic, VarStatus::Upper], row: vec![VarStatus::Lower] });
        let mk = |lb: f64| OpenNode { changes: Vec::new(), lower_bound: lb, estimate: lb, depth: 1, basis: Some(b.clone()), basis_epoch: 0, dse: None, branch: None, prot: None, tags: None, sb: None };
        q.push(mk(1.0));
        q.push(mk(2.0));
        assert!(q.iter().all(|n| n.basis.is_none()));
        assert_eq!(q.packed.iter().flatten().count(), 2);
        let n = q.pop_best().unwrap();
        assert_eq!(*n.basis.unwrap(), *b);
        let n = q.pop_best().unwrap();
        assert_eq!(*n.basis.unwrap(), *b);
    }
}

#[cfg(test)]
mod memc_tests {
    use super::*;
    use crate::mip::lp::VarStatus;

    #[test]
    fn changes_roundtrip() {
        let ch = vec![
            BoundChange { col: 3, upper: true, value: 0.0, sym: false },
            BoundChange { col: 13409, upper: false, value: 1.0, sym: true },
            BoundChange { col: 7, upper: false, value: -0.0, sym: false },
            BoundChange { col: 1 << 20, upper: true, value: 2.5, sym: true },
        ];
        let w = pack_changes(&ch).unwrap();
        let back = unpack_changes(&w);
        assert_eq!(back.len(), ch.len());
        for (a, b) in ch.iter().zip(back.iter()) {
            assert_eq!((a.col, a.upper, a.sym, a.value.to_bits()), (b.col, b.upper, b.sym, b.value.to_bits()));
        }
        assert!(pack_changes(&[BoundChange { col: 1 << 28, upper: true, value: 0.0, sym: false }]).is_none());
    }

    #[test]
    fn basis_roundtrip() {
        use VarStatus::*;
        // ほとんど下限 (Sparse になる)
        let mut col = vec![Lower; 1000];
        col[5] = Basic;
        col[17] = Upper;
        col[999] = Zero;
        let row = vec![Basic, Lower, Basic, Upper];
        let p = SparseBasis::encode(&col, &row);
        assert!(matches!(p, SparseBasis::Sparse { .. }));
        assert_eq!(p.decode(), (col.clone(), row.clone()));
        // 混ざっている (Bits になる)
        let col: Vec<VarStatus> = (0..37).map(|j| [Basic, Lower, Upper, Zero][j * 7 % 4]).collect();
        let row: Vec<VarStatus> = (0..11).map(|j| [Upper, Basic, Zero, Lower][j % 4]).collect();
        let p = SparseBasis::encode(&col, &row);
        assert!(matches!(p, SparseBasis::Bits { .. }));
        assert_eq!(p.decode(), (col, row));
        let p = SparseBasis::encode(&[], &[]);
        assert_eq!(p.decode(), (vec![], vec![]));
    }
}
