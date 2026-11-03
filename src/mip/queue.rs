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
}

impl NodeDse {
    /// 持ち物の大きさ [バイト] (目安)。
    pub fn bytes(&self) -> usize {
        match self {
            NodeDse::Pairs(v) => v.len() * 8 + 24,
            NodeDse::Packed(v) => v.len() * 4 + 24,
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
}

impl NodeQueue {
    pub fn new() -> Self {
        NodeQueue { nodes: Vec::new(), free: Vec::new(), by_lb: BTreeSet::new(), by_est: BTreeSet::new(), pops: 0, gen: Vec::new(), protect: Vec::new(), prio_only: false }
    }

    pub fn len(&self) -> usize {
        self.by_lb.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_lb.is_empty()
    }

    /// ノードを入れ、その (枠, 世代) を返す ([`Self::take`] で取り出すのに使う)。
    pub fn push(&mut self, node: OpenNode) -> (usize, u64) {
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
        let id = if let Some(id) = self.free.pop() {
            self.nodes[id] = Some(node);
            id
        } else {
            self.nodes.push(Some(node));
            self.gen.push(0);
            self.nodes.len() - 1
        };
        self.gen[id] += 1;
        self.by_lb.insert((Key(lb), id));
        self.by_est.insert((Key(est), Key(-depth), id));
        (id, self.gen[id])
    }

    /// `push` が返した (枠, 世代) のノードがまだあり、下界が `limit` 以下なら取り出す。
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

    fn remove(&mut self, id: usize) -> OpenNode {
        let mut node = self.nodes[id].take().unwrap();
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
    pub fn pop_best(&mut self) -> Option<OpenNode> {
        let id = self.by_lb.iter().next().map(|&(_, id)| id)?;
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
        self.nodes.iter().flatten().map(node_bytes).sum()
    }

    /// `ENOMOTO_MIP_MEM2` (試作): 持ち物を減らす。深いノード (同じ深さなら下界の大きいもの) から、`dse` なら DSE 重みを、
    /// `basis` なら基底 (と効くカットの番号) も捨て、減った大きさの目安が `target` に届いたらやめる。`keep` の (枠, 世代)
    /// (潜りの兄弟) は捨てない。減った大きさの目安を返す。
    pub fn strip(&mut self, target: usize, basis: bool, keep: &[(usize, u64)]) -> usize {
        let mut ids: Vec<(usize, usize, f64)> = Vec::new();
        for (id, n) in self.nodes.iter().enumerate() {
            if let Some(n) = n {
                if (n.dse.is_some() || (basis && n.basis.is_some())) && !keep.iter().any(|&(k, g)| k == id && self.gen[id] == g) {
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
            let n = self.nodes[id].as_mut().unwrap();
            let before = node_bytes(n);
            n.dse = None;
            let mut pr = None;
            if basis {
                n.basis = None;
                pr = n.prot.take();
            }
            freed += before.saturating_sub(node_bytes(self.nodes[id].as_ref().unwrap()));
            if let Some(pr) = pr {
                self.unprotect(&pr);
            }
        }
        freed
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
