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
}

#[inline]
fn holds(dom: &Domain, &(j, upper, v): &Lit) -> bool {
    if upper {
        dom.up[j] <= v + 1e-9 * (1.0 + v.abs())
    } else {
        dom.lo[j] >= v - 1e-9 * (1.0 + v.abs())
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
        }
    }

    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// 衝突を加える (同じものがあれば加えずに偽)。
    pub fn add(&mut self, lits: &[Lit]) -> bool {
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
        let slot = if self.slots.len() < self.cap {
            self.slots.push(None);
            self.slot_key.push(0);
            self.seen.push(0);
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
            let changed = dom.changes_since(pos);
            pos = dom.stack_len();
            if changed.is_empty() {
                break;
            }
            self.stamp = self.stamp.wrapping_add(1);
            if self.stamp == 0 {
                self.seen.iter_mut().for_each(|s| *s = 0);
                self.stamp = 1;
            }
            let mut fixes: Vec<(usize, bool, f64)> = Vec::new();
            for &(j, _, _) in &changed {
                for &s in &self.col_index[j] {
                    let s = s as usize;
                    if self.seen[s] == self.stamp {
                        continue;
                    }
                    self.seen[s] = self.stamp;
                    let Some(c) = &self.slots[s] else { continue };
                    let mut open: Option<&Lit> = None;
                    let mut nopen = 0;
                    for lit in c {
                        if !holds(dom, lit) {
                            nopen += 1;
                            if nopen > 1 {
                                break;
                            }
                            open = Some(lit);
                        }
                    }
                    match (nopen, open) {
                        (0, _) => return Err(()),
                        (1, Some(&(k, upper, v))) => {
                            // 否定: x <= v の否定は x >= v + 1 (整数) / x >= v (連続、緩めたもの)
                            let int = p.is_int[k];
                            if upper {
                                fixes.push((k, false, if int { v.floor() + 1.0 } else { v }));
                            } else {
                                fixes.push((k, true, if int { v.ceil() - 1.0 } else { v }));
                            }
                        }
                        _ => {}
                    }
                }
            }
            if fixes.is_empty() {
                break;
            }
            dom.set_external(true);
            for (k, upper, v) in fixes {
                let ch = if upper {
                    v < dom.up[k] && { dom.tighten_upper(p, k, v); true }
                } else {
                    v > dom.lo[k] && { dom.tighten_lower(p, k, v); true }
                };
                if ch {
                    tightened += 1;
                }
                if dom.infeasible {
                    dom.set_external(false);
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
