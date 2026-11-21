//! 診断 `ENOMOTO_MIP_TREEDEPTH_DIAG=<ファイル>` (既定では使わない、analysis/treedepth_20261121/README.md): 木の中の分離
//! (`ENOMOTO_MIP_TREE_SEP` の取り出したノードの分離) を、分離したノードの深さごとに見るための生の記録を集める。
//! 求解の道筋は変えない (数えるだけで、判定には使わない)。スレッドごとに、終わりに 1 行の JSON を `<ファイル>` の末尾に足す。
//!
//! 記録するもの:
//! - `rounds`: 木の分離の 1 ラウンドごとに [深さ, 分離したノードの通し番号, 種類, 伸び, 加えた本数, 手間 (分離), 手間 (解き直し), 秒 (分離), 秒 (解き直し)]。
//!   種類 0 = 解き直しの LP が最適 (伸び = (後の LP の値 − 前の LP の値) / (打ち切り値 − 前の LP の値))、1 = 打ち切り・実行不能 (伸び 1)、
//!   2 = 打ち切り値がない (暫定解なし) か LP が最適でない (伸びは -1)、3 = 違反するカットが見つからず加わらなかった (伸びは -1)。
//! - `nodes`: 分離したノードごとに [深さ, そのノードの後にこのスレッドで LP を解いた部分木の中のノードの数 (自分を含まない)]。
//! - `cuts`: 木の分離で加えたカットごとに [深さ, ノードの通し番号, LP にあったノードの LP の数, 外されるまでのノードの LP の数 (終わりに
//!   残っていれば -1), 双対値が非零 (余裕 0) で張った回数 (どこでも), そのうち部分木の中のノード, 余裕 0 だが双対値 0 の回数]。
//!   同じカットがプールから戻ったとき (新しい番号) は数えない (最初の 1 回の滞在だけ)。
//! - `root`: 根のカットのループの呼び出しごとに [前の LP の値, 後の LP の値, 手間, 成否 (1/0)] (再スタートの後の分も)。
//! - `start`: 木の探索の最初のノードの時点の {暫定解の値 (なければ null), 探索全体の下界, この求解の始めからの手間, 秒}。
//! - `fin`: 終わりの [暫定解の値, 探索全体の下界]。`root`・`start`・`fin` の値は最小化形 (定数項込み)。
//! - `pop`: 取り出したノード (潜りの途中でない) の深さごとの数、`all`: 処理したノードの深さごとの数。
//!
//! 部分木の判定は、根からの分枝の変更の列 (`OpenNode::changes`) の前の部分が一致するか (前の部分ごとのハッシュで比べる)。
//! 4 スレッドでは部分木の一部が他のスレッドで解かれ、その分は数えない。

use std::collections::HashMap;

use super::queue::BoundChange;

pub(super) fn on() -> bool {
    static ON: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    ON.get_or_init(|| std::env::var("ENOMOTO_MIP_TREEDEPTH_DIAG").ok().filter(|s| !s.is_empty())).is_some()
}

fn path() -> Option<String> {
    std::env::var("ENOMOTO_MIP_TREEDEPTH_DIAG").ok().filter(|s| !s.is_empty())
}

struct CutRec {
    depth: u32,
    seq: u32,
    /// 分離したノードの (変更の列の長さ, ハッシュ)
    key: (u32, u64),
    born: u64,
    last_seen: u64,
    lps_in: u32,
    bind_all: u32,
    bind_sub: u32,
    tight0: u32,
}

#[derive(Default)]
pub(super) struct TdDiag {
    /// 今のノードの変更の列の前の部分ごとのハッシュ (`pre[k]` = 最初の k 個)
    pre: Vec<u64>,
    cur_depth: usize,
    /// このスレッドで解いたノードの LP (最適) の数
    lps: u64,
    /// 分離したノードの (長さ, ハッシュ) → 通し番号 (同じ前の部分のノードが 2 度分離されたら後のもので上書き)
    sep_key: HashMap<(u32, u64), u32>,
    /// 分離したノードごとの [深さ, 部分木の中で LP を解いたノードの数]
    nodes: Vec<[u64; 2]>,
    /// 今のノードの通し番号 (分離したら)
    cur_seq: Option<u32>,
    /// 部分木の数を数えたノード (ノードの LP を何度か解き直しても 1 回)
    counted: bool,
    rounds: Vec<[f64; 9]>,
    /// 伸びを測っているラウンド ([`Self::rounds`] の位置) と前の LP の値
    pending: Option<(usize, f64)>,
    live: HashMap<u64, CutRec>,
    dead: Vec<[i64; 7]>,
    pop: Vec<u64>,
    all: Vec<u64>,
    root: Vec<[f64; 4]>,
    start: Option<(Option<f64>, f64, f64, f64)>,
    /// 終わりの (暫定解の値, 探索全体の下界) (最小化形、定数項込み)
    pub(super) fin: (Option<f64>, f64),
}

fn h(c: &BoundChange, prev: u64) -> u64 {
    let mut x = prev ^ (c.col as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= c.value.to_bits().rotate_left(17) ^ ((c.upper as u64) << 63) ^ ((c.sym as u64) << 62);
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^ (x >> 31)
}

fn bump(v: &mut Vec<u64>, d: usize) {
    if v.len() <= d {
        v.resize(d + 1, 0);
    }
    v[d] += 1;
}

impl TdDiag {
    pub(super) fn root_cuts(&mut self, o0: f64, o1: f64, ticks: f64, ok: bool) {
        self.root.push([o0, o1, ticks, ok as u8 as f64]);
    }

    pub(super) fn started(&self) -> bool {
        self.start.is_some()
    }

    pub(super) fn tree_start(&mut self, inc: Option<f64>, lb: f64, ticks: f64, secs: f64) {
        self.start = Some((inc, lb, ticks, secs));
    }

    /// ノードの処理の始め (分離・LP の前)
    pub(super) fn begin_node(&mut self, changes: &[BoundChange], depth: usize, popped: bool) {
        self.pre.clear();
        self.pre.push(0x1234_5678);
        let mut x = 0x1234_5678u64;
        for c in changes {
            x = h(c, x);
            self.pre.push(x);
        }
        self.cur_depth = depth;
        self.cur_seq = None;
        self.counted = false;
        self.pending = None;
        bump(&mut self.all, depth);
        if popped {
            bump(&mut self.pop, depth);
        }
    }

    fn key_now(&self) -> (u32, u64) {
        let l = self.pre.len() - 1;
        (l as u32, self.pre[l])
    }

    fn in_sub(&self, key: (u32, u64)) -> bool {
        let l = key.0 as usize;
        l < self.pre.len() - 1 && self.pre[l] == key.1
    }

    /// 木の分離の 1 ラウンドの後。`first` = このノードの最初のラウンド、`ids` = 加えたカットの番号の範囲
    #[allow(clippy::too_many_arguments)]
    pub(super) fn round(&mut self, first: bool, obj0: f64, ids: std::ops::Range<u64>, ticks: f64, secs: f64) {
        if first || self.cur_seq.is_none() {
            let s = self.nodes.len() as u32;
            self.nodes.push([self.cur_depth as u64, 0]);
            let k = self.key_now();
            self.sep_key.insert(k, s);
            self.cur_seq = Some(s);
        }
        let seq = self.cur_seq.unwrap();
        let n = ids.end.saturating_sub(ids.start);
        let kind = if n > 0 { 2.0 } else { 3.0 };
        self.rounds.push([self.cur_depth as f64, seq as f64, kind, -1.0, n as f64, ticks, 0.0, secs, 0.0]);
        if n > 0 {
            self.pending = Some((self.rounds.len() - 1, obj0));
            let key = self.key_now();
            for id in ids {
                self.live.insert(id, CutRec { depth: self.cur_depth as u32, seq, key, born: self.lps, last_seen: self.lps, lps_in: 0, bind_all: 0, bind_sub: 0, tight0: 0 });
            }
        }
    }

    /// 木の分離の後の解き直しの LP の後 (`st_kind`: 0 = 最適で値 `obj`、1 = 打ち切り・実行不能、2 = ほか)。`lim` = 打ち切り値
    pub(super) fn resolved(&mut self, st_kind: u8, obj: f64, lim: f64, ticks: f64, secs: f64) {
        let Some((i, o0)) = self.pending.take() else {
            return;
        };
        let r = &mut self.rounds[i];
        r[6] = ticks;
        r[8] = secs;
        match st_kind {
            1 => {
                r[2] = 1.0;
                r[3] = 1.0;
            }
            0 if lim.is_finite() && lim > o0 => {
                r[2] = if obj >= lim { 1.0 } else { 0.0 };
                r[3] = if obj >= lim { 1.0 } else { (obj - o0) / (lim - o0) };
            }
            _ => {}
        }
    }

    /// ノードの LP が最適で解けた後 (年齢を数える所)。`stat(k)` = LP の k 番目のカットの行の状態 (2 = 双対値が非零、1 = 余裕 0 で双対値 0、0 = 余裕あり)
    pub(super) fn after_lp(&mut self, cut_ids: &[u64], stat: impl Fn(usize) -> u8) {
        self.lps += 1;
        if !self.counted {
            self.counted = true;
            // 祖先の分離したノードの部分木の数を増やす (自分自身は含まない)
            let l = self.pre.len() - 1;
            for k in 0..l {
                if let Some(&s) = self.sep_key.get(&(k as u32, self.pre[k])) {
                    self.nodes[s as usize][1] += 1;
                }
            }
        }
        if self.live.is_empty() {
            return;
        }
        let now = self.lps;
        for (k, id) in cut_ids.iter().enumerate() {
            if let Some(rec) = self.live.get(id) {
                let key = rec.key;
                let sub = self.in_sub(key);
                let s = stat(k);
                let rec = self.live.get_mut(id).unwrap();
                rec.last_seen = now;
                rec.lps_in += 1;
                match s {
                    2 => {
                        rec.bind_all += 1;
                        rec.bind_sub += sub as u32;
                    }
                    1 => rec.tight0 += 1,
                    _ => {}
                }
            }
        }
        // 今の LP にないものは外された
        let gone: Vec<u64> = self.live.iter().filter(|(_, r)| r.last_seen < now && (r.lps_in > 0 || r.born + 1 < now)).map(|(&id, _)| id).collect();
        for id in gone {
            let r = self.live.remove(&id).unwrap();
            self.dead.push([r.depth as i64, r.seq as i64, r.lps_in as i64, (r.last_seen - r.born) as i64, r.bind_all as i64, r.bind_sub as i64, r.tight0 as i64]);
        }
    }

    /// 終わりに 1 行の JSON をファイルに足す
    pub(super) fn dump(&mut self, tid: usize, secs: f64, nodes: u64) {
        let Some(p) = path() else {
            return;
        };
        use std::fmt::Write as _;
        let mut s = String::new();
        let _ = write!(s, "{{\"tid\":{tid},\"secs\":{secs:.2},\"nodes\":{nodes},\"lps\":{},\"rounds\":[", self.lps);
        for (i, r) in self.rounds.iter().enumerate() {
            let _ = write!(
                s,
                "{}[{},{},{},{:.6e},{},{:.0},{:.0},{:.5},{:.5}]",
                if i > 0 { "," } else { "" },
                r[0] as u64, r[1] as u64, r[2] as u64, r[3], r[4] as u64, r[5], r[6], r[7], r[8]
            );
        }
        s.push_str("],\"nodes_sep\":[");
        for (i, n) in self.nodes.iter().enumerate() {
            let _ = write!(s, "{}[{},{}]", if i > 0 { "," } else { "" }, n[0], n[1]);
        }
        s.push_str("],\"cuts\":[");
        let mut first = true;
        let alive: Vec<[i64; 7]> = self.live.values().map(|r| [r.depth as i64, r.seq as i64, r.lps_in as i64, -1, r.bind_all as i64, r.bind_sub as i64, r.tight0 as i64]).collect();
        for c in self.dead.iter().chain(alive.iter()) {
            let _ = write!(s, "{}[{},{},{},{},{},{},{}]", if first { "" } else { "," }, c[0], c[1], c[2], c[3], c[4], c[5], c[6]);
            first = false;
        }
        s.push_str("],\"root\":[");
        for (i, r) in self.root.iter().enumerate() {
            let _ = write!(s, "{}[{:e},{:e},{:.0},{}]", if i > 0 { "," } else { "" }, r[0], r[1], r[2], r[3]);
        }
        s.push_str("],\"start\":");
        match self.start {
            Some((inc, lb, tk, sec)) => {
                let f = |v: f64| if v.is_finite() { format!("{v:e}") } else { "null".to_string() };
                let _ = write!(s, "[{},{},{:.0},{:.3}]", inc.map_or("null".to_string(), f), f(lb), tk, sec);
            }
            None => s.push_str("null"),
        }
        {
            let f = |v: f64| if v.is_finite() { format!("{v:e}") } else { "null".to_string() };
            let _ = write!(s, ",\"fin\":[{},{}]", self.fin.0.map_or("null".to_string(), f), f(self.fin.1));
        }
        let _ = write!(s, ",\"pop\":{:?},\"all\":{:?}}}\n", self.pop, self.all);
        use std::io::Write as _;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
            let _ = f.write_all(s.as_bytes());
        }
    }
}
