//! 診断用 (試作): 他のソルバー (HiGHS) の木の分枝をなぞる (`ENOMOTO_MIP_BRANCH_REPLAY=ファイル`)。
//!
//! ファイルは `analysis/tree_replay_20261022/highs_to_replay.py` が HiGHS の計装ログ (`HIGHS_PATH_LOG`) から作る。1 行 1 ノードで
//! `seq 結果 分枝した列 (なければ -1) 先の子が上か lp 経路...` (経路は根からの分枝 `列u1` / `列d0`、列は元の問題の番号)。
//! 木の主スレッド (1 スレッド、サブ MIP でない) で、待ち行列から取り出したノードの経路 (分枝した列と向きの集合) が
//! ファイルにあれば同じ列で分枝し、ノードの選び方は HiGHS の処理順 (seq) にする。ファイルにないノード (HiGHS が刈った
//! ノードの下) は自前の規則で分枝し、親の直後に処理する (深さ優先)。`ENOMOTO_MIP_REPLAY_LOG` でノードごとに `ER` 行を出す。

use super::queue::BoundChange;
use std::collections::HashMap;
use std::sync::OnceLock;

pub struct Entry {
    pub seq: u64,
    /// HiGHS の結果 (0 目的値で打ち切り、1 定義域の矛盾、2 LP が実行不能、4 下界で刈る、5 分枝)
    pub res: i32,
    /// 分枝した列 (-1 なら分枝していない)
    pub col: i64,
    pub up_first: bool,
    pub lp: f64,
    pub nfix: i64,
}

pub struct Replay {
    pub map: HashMap<Vec<(u32, bool)>, Entry>,
    pub max_seq: u64,
}

static REPLAY: OnceLock<Option<Replay>> = OnceLock::new();

pub fn get() -> Option<&'static Replay> {
    REPLAY
        .get_or_init(|| {
            let f = env_str!("ENOMOTO_MIP_BRANCH_REPLAY")?;
            let s = std::fs::read_to_string(f).ok()?;
            let mut map = HashMap::new();
            let mut max_seq = 0;
            for l in s.lines() {
                let t: Vec<&str> = l.split_whitespace().collect();
                if t.len() < 7 || t[0].starts_with('#') {
                    continue;
                }
                let seq: u64 = t[0].parse().unwrap();
                let mut key: Vec<(u32, bool)> = t[7..]
                    .iter()
                    .filter(|p| !p.starts_with('#'))
                    .map(|p| {
                        let k = p.find(['u', 'd']).unwrap();
                        (p[..k].parse::<u32>().unwrap(), &p[k..k + 1] == "u")
                    })
                    .collect();
                key.sort();
                max_seq = max_seq.max(seq);
                map.insert(key, Entry { seq, res: t[1].parse().unwrap(), col: t[2].parse().unwrap(), up_first: t[3] == "1", lp: t[4].parse().unwrap_or(f64::NAN), nfix: t[5].parse().unwrap_or(-1) });
            }
            eprintln!("MIP: branch replay: {} nodes loaded from {f}", map.len());
            Some(Replay { map, max_seq })
        })
        .as_ref()
}

pub fn key(changes: &[BoundChange]) -> Vec<(u32, bool)> {
    let mut k: Vec<(u32, bool)> = changes.iter().map(|c| (c.col as u32, !c.upper)).collect();
    k.sort();
    k.dedup();
    k
}

pub fn lookup(changes: &[BoundChange]) -> Option<&'static Entry> {
    get()?.map.get(&key(changes))
}

pub fn log_on() -> bool {
    env_str!("ENOMOTO_MIP_REPLAY_LOG").is_some()
}
