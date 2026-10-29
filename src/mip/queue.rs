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
    pub dse: Option<Rc<Vec<(u32, f32)>>>,
    /// このノードを作った分枝 (列, 上向きか, 親の LP 値, 親の LP 目的値)。pseudocost の更新に使う。
    pub branch: Option<(usize, bool, f64, f64)>,
    /// `basis` で効いている (論理変数が非基底の) カットの番号 (`Solver::cut_ids`)。待ち行列にある間、このカットは
    /// 年齢で外さない (外すと取り出したときに基底が合わず LP がほぼ解き直しになる)。
    pub prot: Option<Rc<Vec<u64>>>,
    /// ポートフォリオ並列の木の写し ([`super::solver::TreeSnapshot`]) での、このノードの祖先の番号 (写しの通し番号 << 32 | 写しの
    /// 中のノードの番号)。子は親のものを引き継ぐ。二分探索のスレッドが「この祖先の部分木には t 未満の解がない」と示したら、
    /// 下界を t に上げる。
    pub tags: Option<Rc<Vec<u64>>>,
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
