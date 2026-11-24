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

/// 診断 `ENOMOTO_MIP_CONFDEEP_DIAG` (analysis/confdeep_20261122、解き方は変えない): 衝突ごとに作ったときの形 (長さ・段・作り方・遡った理由) と、
/// その後の使われ方 (締め付け・矛盾)・捨てられ方・寿命を数える。サブ MIP の分は数えない
pub fn confdeep_diag() -> bool {
    static F: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *F.get_or_init(|| std::env::var_os("ENOMOTO_MIP_CONFDEEP_DIAG").is_some())
}

/// 同じ診断の、衝突を作ったときの形 [段 (解析で何段目の衝突か、0 はほかの経路), 作り方 (0 最初の段の組そのもの, 1 UIP, 2 再収束, 3 ほかの経路),
/// それまでに遡った変更の数: 行, 双対証明, 写しの表 (カット・リテラルの衝突)]
pub type DeepMeta = [u32; 5];

/// 同じ診断の集計の軸と欄。軸: 0 長さ (1, 2, 3-4, 5-8, 9-16, 17-32, 33-64, 65-)、1 段 (ほか, 1, 2, 3, 4, 5, 6-10, 11-)、
/// 2 作り方 (最初の段, UIP, 再収束, ほか)、3 遡った理由 (行だけ, 双対証明あり, 写しの表あり, 両方)。
/// 欄: 0 作った, 1 1 回以上使われた, 2 締め付け, 3 矛盾, 4 年齢で捨てた, 5 そのうち使われたことがある, 6 上書きで捨てた, 7 そのうち使われた,
/// 8 捨てるまでの寿命 (伝播の呼び出しの数) の和, 9 作ってから最初に使われるまでの呼び出しの数の和
pub struct DeepAgg {
    pub t: [[[u64; 10]; 8]; 4],
    /// 使われたときの年齢 (前に使われてから調べられた回数) の分布 (0, 1, 2-3, 4-7, ..., 512-1023, 1024-)
    pub age_hist: [u64; 12],
    /// 年齢で捨てた衝突をもう一度作った数 [全部, そのうち捨てた方が使われたことがある]
    pub relearn: [u64; 2],
    /// 伝播の呼び出し, 調べた衝突, 呼び出しの時点で残っていた衝突の数の和, 伝播の秒 (ナノ秒)
    pub prop: [u64; 4],
    /// 年齢で捨てた衝突の鍵 (使われたことがあるか)
    removed_keys: Option<std::collections::HashMap<u64, bool>>,
}

pub static DEEP: std::sync::Mutex<DeepAgg> = std::sync::Mutex::new(DeepAgg { t: [[[0; 10]; 8]; 4], age_hist: [0; 12], relearn: [0; 2], prop: [0; 4], removed_keys: None });

fn deep_on() -> bool {
    confdeep_diag() && !super::solver::in_submip_thread()
}

fn len_bin(n: usize) -> usize {
    match n {
        0 | 1 => 0,
        2 => 1,
        3..=4 => 2,
        5..=8 => 3,
        9..=16 => 4,
        17..=32 => 5,
        33..=64 => 6,
        _ => 7,
    }
}

/// 衝突の枠ごとの形 (集計の軸ごとの区分)、使われた回数、作ったときの呼び出しの番号、最初に使われたか
#[derive(Clone, Copy, Default)]
struct SlotDeep {
    bins: [u8; 4],
    used: bool,
    born: u64,
}

impl DeepAgg {
    fn add(&mut self, b: &[u8; 4], f: usize, v: u64) {
        for (d, &k) in b.iter().enumerate() {
            self.t[d][k as usize][f] += v;
        }
    }

    /// 集計を出す (1 行ずつ)
    pub fn report(&self) -> Vec<String> {
        let names = ["length (1,2,3-4,5-8,9-16,17-32,33-64,65-)", "level (other,1,2,3,4,5,6-10,11-)", "kind (first,uip,reconv,other)", "reasons (rows,proof,arena,both)"];
        let mut out = Vec::new();
        for (d, nm) in names.iter().enumerate() {
            let cols: Vec<String> = (0..8)
                .filter(|&k| self.t[d][k][0] > 0)
                .map(|k| {
                    let r = &self.t[d][k];
                    format!("[{k}] made {} used {} fix {} prune {} aged {} (used {}) overwritten {} (used {}) life {:.0} first-use {:.0}", r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7], r[8] as f64 / (r[4] + r[6]).max(1) as f64, r[9] as f64 / r[1].max(1) as f64)
                })
                .collect();
            out.push(format!("MIP: CONFDEEP by {nm}: {}", cols.join("; ")));
        }
        out.push(format!(
            "MIP: CONFDEEP pool: propagate calls {}, examined {}, mean live {:.1}, {:.2}s; age at use (0,1,2-3,..,512-1023,1024-) {:?}; relearned after aging {} (of used {})",
            self.prop[0],
            self.prop[1],
            self.prop[2] as f64 / self.prop[0].max(1) as f64,
            self.prop[3] as f64 * 1e-9,
            self.age_hist,
            self.relearn[0],
            self.relearn[1]
        ));
        out
    }
}

pub struct LitConflictPool {
    /// 衝突 (枠ごと。古いものから上書きする)。
    slots: Vec<Option<Vec<Lit>>>,
    next: usize,
    cap: usize,
    /// 列ごとの、その列を含む衝突の枠 (上書きで古くなった項目も残る。評価は衝突そのもので行うので害はない)。
    /// 値は 枠 × 2 + (その列のリテラルが上限 (`x <= v`) なら 1) (試作 `ENOMOTO_MIP_PROOFPROP` の `l` で向きを見る)。
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
    /// 診断 `ENOMOTO_MIP_CONFDEEP_DIAG`: 次に [`Self::add`] する衝突の形 (呼ぶ側が置く)、枠ごとの形、伝播の呼び出しの数
    pub pending_meta: Option<DeepMeta>,
    deep: Vec<SlotDeep>,
    calls: u64,
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
            pending_meta: None,
            deep: Vec::new(),
            calls: 0,
        }
    }

    /// 診断: 枠 `s` の衝突を捨てる (`aged` なら年齢で、そうでなければ上書きで)
    fn deep_remove(&self, s: usize, aged: bool, key: u64) {
        if !deep_on() || s >= self.deep.len() {
            return;
        }
        let d = self.deep[s];
        let mut g = DEEP.lock().unwrap();
        g.add(&d.bins, if aged { 4 } else { 6 }, 1);
        if d.used {
            g.add(&d.bins, if aged { 5 } else { 7 }, 1);
        }
        g.add(&d.bins, 8, self.calls - d.born);
        if aged {
            g.removed_keys.get_or_insert_with(Default::default).insert(key, d.used);
        }
    }

    /// 診断: 枠 `s` の衝突が使われた (`prune` なら矛盾、そうでなければ締め付け)。年齢はリセットする前の値
    fn deep_use(&mut self, s: usize, prune: bool, age: u32) {
        if !deep_on() || s >= self.deep.len() {
            return;
        }
        let d = &mut self.deep[s];
        let mut g = DEEP.lock().unwrap();
        g.add(&d.bins, if prune { 3 } else { 2 }, 1);
        if !d.used {
            d.used = true;
            g.add(&d.bins, 1, 1);
            g.add(&d.bins, 9, self.calls - d.born);
        }
        let b = if age == 0 { 0 } else { (32 - age.leading_zeros() as usize).min(11) };
        g.age_hist[b] += 1;
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
                self.deep_remove(s, false, self.slot_key[s]);
            }
            s
        };
        let meta = self.pending_meta.take();
        if deep_on() {
            let m = meta.unwrap_or([0, 3, 0, 0, 0]);
            let lev = match m[0] {
                0..=5 => m[0] as u8,
                6..=10 => 6,
                _ => 7,
            };
            let rs = u8::from(m[3] > 0) + 2 * u8::from(m[4] > 0);
            let bins = [len_bin(l.len()) as u8, lev, m[1].min(3) as u8, rs];
            if self.deep.len() < self.slots.len() {
                self.deep.resize(self.slots.len(), SlotDeep::default());
            }
            self.deep[slot] = SlotDeep { bins, used: false, born: self.calls };
            let mut g = DEEP.lock().unwrap();
            g.add(&bins, 0, 1);
            if let Some(u) = g.removed_keys.as_mut().and_then(|m| m.remove(&key)) {
                g.relearn[0] += 1;
                g.relearn[1] += u64::from(u);
            }
        }
        for &(j, u, _) in &l {
            self.col_index[j].push(((slot as u32) << 1) | u as u32);
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
                    for &(j, u, _) in c {
                        self.col_index[j].push(((s as u32) << 1) | u as u32);
                    }
                    self.index_len += c.len();
                }
            }
        }
        true
    }

    /// 試作 `ENOMOTO_MIP_TC` の `l` (treeclose 9.58 節): 全部の衝突を定義域 `dom` (大域的な境界の写し) で 1 巡見る。全部のリテラルが
    /// 成り立つ衝突があれば `Err`、1 つだけ成り立たないものは、その否定を `out` に足す (年齢・統計は変えない)
    pub fn scan_global(&self, p: &MipProblem, dom: &Domain, out: &mut Vec<(usize, bool, f64)>) -> Result<(), ()> {
        for c in self.slots.iter().flatten() {
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
                (0, _) => return Err(()),
                (1, Some(&(k, upper, v))) => {
                    let int = p.is_int[k];
                    if upper {
                        out.push((k, false, if int { v.floor() + 1.0 } else { v }));
                    } else {
                        out.push((k, true, if int { v.ceil() - 1.0 } else { v }));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// 試作 `ENOMOTO_MIP_CONFCUT` (analysis/confcut_20261124): 全てのリテラルが 0-1 列 (整数で大域的な境界が [0, 1]) の `x >= 1`・`x <= 0` で
    /// ある衝突を線形の不等式 Σ_{x>=1} x − Σ_{x<=0} x <= k − 1 − (x<=0 の数) (各リテラルの真の度合い x・1 − x の和 <= k − 1) にし、
    /// 点 `x` が破るもの (破れ > 1e-6 (1 + |右辺|)) を (係数 (列の昇順), 右辺) で `out` に足す。0-1 でない列・大域的に固定された列を
    /// 含む衝突は飛ばす (一般整数・連続のリテラルは線形にすると弱いか、1 つまでしか書けない: [`super::solver`] の `add_conflict_lits`)。
    /// 年齢・統計は変えない
    pub fn violated_binary_rows(&self, p: &MipProblem, glo: &[f64], gup: &[f64], x: &[f64], out: &mut Vec<(Vec<(usize, f64)>, f64)>) {
        'c: for c in self.slots.iter().flatten() {
            let (mut act, mut rhs) = (0.0, c.len() as f64 - 1.0);
            for &(j, upper, v) in c {
                if !p.is_int[j] || glo[j] != 0.0 || gup[j] != 1.0 {
                    continue 'c;
                }
                if !upper && v == 1.0 {
                    act += x[j];
                } else if upper && v == 0.0 {
                    act -= x[j];
                    rhs -= 1.0;
                } else {
                    continue 'c;
                }
            }
            if act - rhs <= 1e-6 * (1.0 + rhs.abs()) {
                continue;
            }
            // リテラルは列・向きの順に並べてある (`add`)。同じ列の両側 (それ自体で矛盾) は使わない
            if c.windows(2).any(|w| w[0].0 == w[1].0) {
                continue;
            }
            out.push((c.iter().map(|&(j, upper, _)| (j, if upper { -1.0 } else { 1.0 })).collect(), rhs));
        }
    }

    /// 診断 (`ENOMOTO_MIP_CONFCUT_DIAG`): 残っている衝突の数と、そのうち全てのリテラルが 0-1 列 (大域的な境界 [0, 1]) のものの数
    pub fn count_binary(&self, p: &MipProblem, glo: &[f64], gup: &[f64]) -> (usize, usize) {
        let live: Vec<&Vec<Lit>> = self.slots.iter().flatten().collect();
        let bin = live.iter().filter(|c| c.iter().all(|&(j, _, _)| p.is_int[j] && glo[j] == 0.0 && gup[j] == 1.0)).count();
        (live.len(), bin)
    }

    /// 定義域の記録の位置 `start` より後に境界が変わった列を含む衝突で伝播する (固定点まで、各回の後に行の伝播も)。
    /// 矛盾なら `Err`、そうでなければ締めた境界の数。
    pub fn propagate(&mut self, p: &MipProblem, dom: &mut Domain, start: usize) -> Result<usize, ()> {
        self.propagate_tag(p, dom, start, None)
    }

    /// [`Self::propagate`] と同じだが、`tagger` があれば境界を締める前に、締める根拠の衝突のリテラルを渡して呼ぶ
    /// (試作 `ENOMOTO_MIP_LIT_EXPLAIN`: 衝突を行にして変更の理由に付け、衝突解析で遡れるようにする)。
    pub fn propagate_tag(&mut self, p: &MipProblem, dom: &mut Domain, start: usize, tagger: Option<&mut dyn FnMut(&[Lit], &mut Domain)>) -> Result<usize, ()> {
        if self.live_lits == 0 {
            return Ok(0);
        }
        if deep_on() {
            self.calls += 1;
            let t0 = std::time::Instant::now();
            let ex0 = self.stats[0][0] + self.stats[1][0];
            let live = self.slots.iter().filter(|s| s.is_some()).count() as u64;
            let r = self.propagate_inner(p, dom, start, tagger);
            let mut g = DEEP.lock().unwrap();
            g.prop[0] += 1;
            g.prop[1] += self.stats[0][0] + self.stats[1][0] - ex0;
            g.prop[2] += live;
            g.prop[3] += t0.elapsed().as_nanos() as u64;
            return r;
        }
        self.propagate_inner(p, dom, start, tagger)
    }

    fn propagate_inner(&mut self, p: &MipProblem, dom: &mut Domain, start: usize, mut tagger: Option<&mut dyn FnMut(&[Lit], &mut Domain)>) -> Result<usize, ()> {
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
            let mut fix_age: Vec<u32> = Vec::new();
            // 試作 `ENOMOTO_MIP_PROOFPROP` の `l` ([`super::solver::proofprop`]): 境界の変更と同じ向きのリテラル (上限の変更なら `x <= v`) を持つ衝突だけを
            // 調べる (違う向きのリテラルは、この変更では新たに成り立たない)。調べた回数で数える年齢が変わるので道筋は変わりうる
            let side = super::solver::proofprop('l');
            for (j, up, _) in changed {
                crate::ticks::add(crate::ticks::PROP, self.col_index[j].len() as u64 + 1);
                for &e in &self.col_index[j] {
                    if side && (e & 1 == 1) != up {
                        continue;
                    }
                    let s = (e >> 1) as usize;
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
                            let age = self.ages[s];
                            self.ages[s] = 0;
                            self.stats[org][1] += 1;
                            if self.unshared[s] {
                                self.unshared[s] = false;
                                self.used_buf.push(c.clone());
                            }
                            self.deep_use(s, true, age);
                            return Err(());
                        }
                        (1, Some(&(k, upper, v))) => {
                            let age = self.ages[s];
                            self.ages[s] = 0;
                            // 否定: x <= v の否定は x >= v + 1 (整数) / x >= v (連続、緩めたもの)
                            let int = p.is_int[k];
                            if upper {
                                fixes.push((k, false, if int { v.floor() + 1.0 } else { v }, org, s));
                            } else {
                                fixes.push((k, true, if int { v.ceil() - 1.0 } else { v }, org, s));
                            }
                            fix_age.push(age);
                        }
                        _ => {
                            self.ages[s] += 1;
                            if self.ages[s] > self.max_age {
                                if let Some(old) = self.slots[s].take() {
                                    self.live_lits -= old.len();
                                    self.keys.remove(&self.slot_key[s]);
                                    self.deep_remove(s, true, self.slot_key[s]);
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
            for (i, (k, upper, v, org, s)) in fixes.into_iter().enumerate() {
                if let Some(t) = tagger.as_mut() {
                    if let Some(c) = &self.slots[s] {
                        t(c, dom);
                    }
                }
                let ch = if upper {
                    v < dom.up[k] && { dom.tighten_upper(p, k, v); true }
                } else {
                    v > dom.lo[k] && { dom.tighten_lower(p, k, v); true }
                };
                if ch {
                    tightened += 1;
                    self.deep_use(s, dom.infeasible, fix_age[i]);
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
                    if !ch {
                        self.deep_use(s, true, fix_age[i]);
                    }
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
