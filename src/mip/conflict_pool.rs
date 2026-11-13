//! 境界リテラルの衝突プール (HiGHS の `HighsConflictPool` を簡略化したもの)。
//!
//! 衝突は「同時には成り立たない境界の組」`{x_j >= v}`・`{x_j <= v}` のまま持つ (連続列・一般整数列を含んでもよい。
//! 0-1 列だけの衝突は線形の行にして証明のプールに入れるので、ここには行にできないものだけが来る)。
//! 伝播は節 (clause) と同じ: 全てのリテラルが成り立てば矛盾、1 つを除いて成り立てば残りの否定を課す
//! (整数列の `x >= v` の否定は `x <= v - 1`、連続列では緩めて `x <= v`)。

use super::domain::Domain;
use super::problem::MipProblem;

/// 衝突の 1 リテラル (列, 上限か (`x <= v`) / 下限か (`x >= v`), 値)。
pub type Lit = (usize, bool, f64);

pub struct LitConflictPool {
    /// 衝突 (枠ごと。古いものから上書きする)。
    slots: Vec<Option<Vec<Lit>>>,
    next: usize,
    cap: usize,
    /// 列ごとの、その列を含む衝突の枠 (上書きで古くなった項目も残る。評価は衝突そのもので行うので害はない)。
    col_index: Vec<Vec<u32>>,
    index_len: usize,
    live_lits: usize,
    keys: std::collections::HashSet<u64>,
    slot_key: Vec<u64>,
    /// 評価済みの印 (呼び出しごとに `stamp` を進める)。
    seen: Vec<u32>,
    stamp: u32,
    /// 衝突ごとの年齢 (伝播・矛盾に使われずに調べられた回数。上限を超えたら消す: HiGHS の conflict aging)
    ages: Vec<u32>,
    max_age: u32,
    pub removed: u64,
    /// 年齢で消して空いた枠
    free: Vec<usize>,
    /// 前回の伝播の後に衝突を加えたか (加えたら次は記録の最初から調べる)
    pub dirty: bool,
    /// 診断用: 枠ごとの、ほかのスレッドから取り込んだ衝突か。
    imported: Vec<bool>,
    /// 枠ごとの、使われたらほかのスレッドに知らせる (自前でまだ知らせていない) 衝突か (既定。`ENOMOTO_MIP_TREE_SHARE_LITS_AT_ONCE` なら作ったときに知らせる)。
    unshared: Vec<bool>,
    /// 使われた、まだ知らせていない自前の衝突 ([`Self::take_used_unshared`] で取り出す)。
    used_buf: Vec<Vec<Lit>>,
    /// 診断用: [自前, 取り込み] ごとの [調べた, 矛盾 (ノードを刈った), 境界を締めた, 使われずに年齢で消えた]。
    pub stats: [[u64; 4]; 2],
}

/// リテラルが今の定義域で成り立つか。整数列は境界が整数なので許容 1e-9 で比べる。連続列は許容なしで比べる
/// (WRONGANS blend2、analysis/wrongans_blend2_20261113.md): 連続列の境界は伝播で 1e-9 (1 + |v|) だけ緩めて置かれる
/// ([`Domain::propose_upper`]) ので、同じ大きさの許容で比べると、上限 2127.999997871 (= 2128 − 1e-9 × 2129) の下で作った衝突
/// `x ≤ 2127.999997871` を上限 2128 のノードでも「成り立つ」と読み、x = 2128 の最適解を含むノードで残りのリテラルの否定を課して
/// 最適解を捨てていた。リテラルの値は定義域の境界そのものなので、許容なしの比べで足りる (成り立つと読む場合が減るだけで、安全側)
#[inline]
fn holds(p: &MipProblem, dom: &Domain, &(j, upper, v): &Lit) -> bool {
    let tol = if p.is_int[j] { 1e-9 * (1.0 + v.abs()) } else { 0.0 };
    if upper {
        dom.up[j] <= v + tol
    } else {
        dom.lo[j] >= v - tol
    }
}

impl LitConflictPool {
    pub fn new(n: usize, cap: usize) -> Self {
        LitConflictPool {
            slots: Vec::new(),
            next: 0,
            cap: cap.max(1),
            col_index: vec![Vec::new(); n],
            index_len: 0,
            live_lits: 0,
            keys: std::collections::HashSet::new(),
            slot_key: Vec::new(),
            seen: Vec::new(),
            stamp: 0,
            dirty: false,
            ages: Vec::new(),
            max_age: tunable!("ENOMOTO_T_MIP_LIT_CONFLICT_AGE", 1000u32, u32),
            removed: 0,
            free: Vec::new(),
            imported: Vec::new(),
            unshared: Vec::new(),
            used_buf: Vec::new(),
            stats: [[0; 4]; 2],
        }
    }

    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// 使われた、まだ知らせていない自前の衝突を取り出す。
    pub fn take_used_unshared(&mut self) -> Vec<Vec<Lit>> {
        std::mem::take(&mut self.used_buf)
    }

    /// 診断用: 残っている衝突の数 ([自前, 取り込み])。
    pub fn live_by_origin(&self) -> [usize; 2] {
        let mut n = [0; 2];
        for (s, c) in self.slots.iter().enumerate() {
            if c.is_some() {
                n[self.imported[s] as usize] += 1;
            }
        }
        n
    }

    /// 衝突を加える (同じものがあれば加えずに偽)。`imported` はほかのスレッドから取り込んだものか (診断用)。
    pub fn add(&mut self, lits: &[Lit], imported: bool) -> bool {
        if lits.is_empty() {
            return false;
        }
        let mut l: Vec<Lit> = lits.to_vec();
        l.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)).then(a.2.total_cmp(&b.2)));
        use std::hash::{Hash, Hasher};
        let mut hs = std::collections::hash_map::DefaultHasher::new();
        for &(j, u, v) in &l {
            (j, u, v.to_bits()).hash(&mut hs);
        }
        let key = hs.finish();
        if !self.keys.insert(key) {
            return false;
        }
        let slot = if let Some(f) = self.free.pop() {
            f
        } else if self.slots.len() < self.cap {
            self.slots.push(None);
            self.slot_key.push(0);
            self.seen.push(0);
            self.ages.push(0);
            self.imported.push(false);
            self.unshared.push(false);
            self.slots.len() - 1
        } else {
            let s = self.next % self.cap;
            self.next += 1;
            if let Some(old) = self.slots[s].take() {
                self.live_lits -= old.len();
                self.keys.remove(&self.slot_key[s]);
            }
            s
        };
        for &(j, _, _) in &l {
            self.col_index[j].push(slot as u32);
        }
        self.index_len += l.len();
        self.live_lits += l.len();
        self.slot_key[slot] = key;
        self.slots[slot] = Some(l);
        self.ages[slot] = 0;
        self.imported[slot] = imported;
        self.unshared[slot] = !imported && env_str!("ENOMOTO_MIP_TREE_SHARE_LITS_AT_ONCE").is_none();
        self.dirty = true;
        // 古い索引の項目が増えたら作り直す
        if self.index_len > 4 * self.live_lits + 1000 {
            for v in self.col_index.iter_mut() {
                v.clear();
            }
            self.index_len = 0;
            for (s, c) in self.slots.iter().enumerate() {
                if let Some(c) = c {
                    for &(j, _, _) in c {
                        self.col_index[j].push(s as u32);
                    }
                    self.index_len += c.len();
                }
            }
        }
        true
    }

    /// 定義域の記録の位置 `start` より後に境界が変わった列を含む衝突で伝播する (固定点まで、各回の後に行の伝播も)。
    /// 矛盾なら `Err`、そうでなければ締めた境界の数。
    pub fn propagate(&mut self, p: &MipProblem, dom: &mut Domain, start: usize) -> Result<usize, ()> {
        if self.live_lits == 0 {
            return Ok(0);
        }
        let mut pos = start;
        let mut tightened = 0usize;
        for _ in 0..10 {
            let end = dom.stack_len();
            if pos >= end {
                break;
            }
            let changed = dom.changes_since(pos);
            pos = end;
            self.stamp = self.stamp.wrapping_add(1);
            if self.stamp == 0 {
                self.seen.iter_mut().for_each(|s| *s = 0);
                self.stamp = 1;
            }
            let mut fixes: Vec<(usize, bool, f64, usize, usize)> = Vec::new();
            for (j, _, _) in changed {
                for &s in &self.col_index[j] {
                    let s = s as usize;
                    if self.seen[s] == self.stamp {
                        continue;
                    }
                    self.seen[s] = self.stamp;
                    let Some(c) = &self.slots[s] else { continue };
                    let org = self.imported[s] as usize;
                    self.stats[org][0] += 1;
                    let mut open: Option<&Lit> = None;
                    let mut nopen = 0;
                    for lit in c {
                        if !holds(p, dom, lit) {
                            nopen += 1;
                            if nopen > 1 {
                                break;
                            }
                            open = Some(lit);
                        }
                    }
                    match (nopen, open) {
                        (0, _) => {
                            self.ages[s] = 0;
                            self.stats[org][1] += 1;
                            if self.unshared[s] {
                                self.unshared[s] = false;
                                self.used_buf.push(c.clone());
                            }
                            return Err(());
                        }
                        (1, Some(&(k, upper, v))) => {
                            self.ages[s] = 0;
                            // 否定: x <= v の否定は x >= v + 1 (整数) / x >= v (連続、緩めたもの)
                            let int = p.is_int[k];
                            if upper {
                                fixes.push((k, false, if int { v.floor() + 1.0 } else { v }, org, s));
                            } else {
                                fixes.push((k, true, if int { v.ceil() - 1.0 } else { v }, org, s));
                            }
                        }
                        _ => {
                            self.ages[s] += 1;
                            if self.ages[s] > self.max_age {
                                if let Some(old) = self.slots[s].take() {
                                    self.live_lits -= old.len();
                                    self.keys.remove(&self.slot_key[s]);
                                    self.removed += 1;
                                    self.stats[org][3] += 1;
                                    self.free.push(s);
                                }
                            }
                        }
                    }
                }
            }
            if fixes.is_empty() {
                break;
            }
            dom.set_external_tag(5);
            for (k, upper, v, org, s) in fixes {
                let ch = if upper {
                    v < dom.up[k] && { dom.tighten_upper(p, k, v); true }
                } else {
                    v > dom.lo[k] && { dom.tighten_lower(p, k, v); true }
                };
                if ch {
                    tightened += 1;
                    self.stats[org][2] += 1;
                    if self.unshared[s] {
                        self.unshared[s] = false;
                        if let Some(c) = &self.slots[s] {
                            self.used_buf.push(c.clone());
                        }
                    }
                }
                if dom.infeasible {
                    dom.set_external(false);
                    self.stats[org][1] += 1;
                    return Err(());
                }
            }
            dom.set_external(false);
            if !dom.propagate(p) {
                return Err(());
            }
        }
        Ok(tightened)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WRONGANS blend2 (analysis/wrongans_blend2_20261113.md): 連続列の上限 2128 は、衝突のリテラル `x0 ≤ 2128 − 1e-9 × 2129`
    /// (伝播の緩めと同じ大きさだけ締まった上限の下で作ったもの) を満たさない。旧い比べ (許容 1e-9 (1 + |v|)) は満たすと読み、
    /// 残りのリテラル x1 ≥ 1 の否定 x1 ≤ 0 を課していた (x0 = 2128, x1 = 1 の解を捨てる)
    #[test]
    fn continuous_literal_is_compared_without_tolerance() {
        // x0 連続 [0, 2128]、x1・x2 は 0-1 整数。x0 + x1 + x2 <= 1e6 (何も伝播しない)
        let p = MipProblem::from_rows(
            vec![0.0; 3],
            vec![2128.0, 1.0, 1.0],
            vec![1.0, 1.0, 1.0],
            0.0,
            1.0,
            vec![false, true, true],
            vec![vec![(0, 1.0), (1, 1.0), (2, 1.0)]],
            vec![f64::NEG_INFINITY],
            vec![1e6],
        );
        let v = 2128.0 - 1e-9 * 2129.0;
        let mut pool = LitConflictPool::new(p.n, 100);
        assert!(pool.add(&[(0, true, v), (1, false, 1.0), (2, false, 1.0)], false));
        let mut d = Domain::new(&p);
        assert!(d.propagate(&p));
        let pos = d.stack_len();
        d.tighten_lower(&p, 2, 1.0);
        assert!(d.propagate(&p));
        // 上限 2128 では x0 のリテラルは成り立たないので、x1 は締めない
        assert_eq!(pool.propagate(&p, &mut d, pos), Ok(0));
        assert_eq!(d.up[1], 1.0);
        // 上限をリテラルの値まで締めれば成り立ち、x1 ≤ 0 を課す
        let pos = d.stack_len();
        d.tighten_upper(&p, 0, v);
        assert_eq!(pool.propagate(&p, &mut d, pos), Ok(1));
        assert_eq!(d.up[1], 0.0);
    }
}
