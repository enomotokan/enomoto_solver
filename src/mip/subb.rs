//! 担当 B の試作 (analysis/submip_impr_20261106/B.md、どれも既定では使わない):
//!
//! - `ENOMOTO_MIP_SUBB_DIV=1`: 暫定解を改善するサブ MIP の近傍の多様化。外側の求解のサブ MIP ごとに「固定の集合の指紋」
//!   (ラベル・足した行の数・目的関数を変えたか・固定した整数列とその値・暫定解の目的値) を覚え、同じ指紋のサブ MIP を
//!   繰り返すときは固定を変える (前の回が近傍を解き切った (最適・実行不能) なら固定を乱数で外して広げ、上限で止まったなら
//!   自由な整数列を暫定解の値に乱数で固定して狭める。割合は繰り返しの回数で大きくする)。改善のない回が
//!   `ENOMOTO_T_MIP_SUBB_STREAK` (4) 回続いたら、繰り返しでなくても、それまでの結果の向き (解き切りが多ければ広げ、上限が多ければ狭める)
//!   で固定を変える。
//! - `ENOMOTO_MIP_SUBB_POOL=1`: ALNS の Crossover の腕を、解のプールの複数の解から作る近傍に替える (乱数で選んだ 2〜3 個の解が
//!   一致する列を固定する交差と、最良でないプールの解を中心に整数列の一部を固定する変異を交互に)。
//! - `ENOMOTO_MIP_SUBB_SYM=1`: サブ MIP の中でも対称性を検出して使う。対称性の表 (オービトープ・生成元・対称性の行の前の問題) は
//!   スレッドごとの変数なので、サブ MIP の求解の始めに外側の表を退避し、終わりに戻す (求解ごとの表)。検出の時間の上限は
//!   `ENOMOTO_T_MIP_SUBB_SYM_TIME` (0.05 秒)。
//! - `ENOMOTO_MIP_SUBB_COUNT=1` (診断だけ、解き方は変えない): 指紋の繰り返しを数えてログの終わりに出す (`DIV` でも数える)。

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

pub(crate) fn div_on() -> bool {
    env_str!("ENOMOTO_MIP_SUBB_DIV").is_some_and(|s| s != "0")
}

pub(crate) fn pool_on() -> bool {
    env_str!("ENOMOTO_MIP_SUBB_POOL").is_some_and(|s| s != "0")
}

pub(crate) fn sym_on() -> bool {
    env_str!("ENOMOTO_MIP_SUBB_SYM").is_some_and(|s| s != "0")
}

/// 指紋を覚えるか (`DIV` か、診断の `COUNT`)。
pub(crate) fn count_on() -> bool {
    div_on() || env_str!("ENOMOTO_MIP_SUBB_COUNT").is_some_and(|s| s != "0")
}

/// 全スレッドの計数: [外側のサブ MIP の数, 同じ指紋の繰り返し, 広げた回数, 狭めた回数, 連続の空振りで変えた回数, プールの交差, プールの変異,
/// 多様化した回の改善]
pub(crate) static DIV_STATS: [AtomicU64; 8] = [const { AtomicU64::new(0) }; 8];

/// 全スレッドの計数: [検出したサブ MIP, 対称性が見つかったサブ MIP, 生成元の合計, オービトープの合計, 軌道の列の合計,
/// orbitopal fixing の固定, orbital fixing の固定, 検出の時間の合計 (マイクロ秒)]
pub(crate) static SYM_STATS: [AtomicU64; 8] = [const { AtomicU64::new(0) }; 8];

pub(crate) fn add(a: &AtomicU64, v: u64) {
    a.fetch_add(v, Relaxed);
}

thread_local! {
    /// 今の (サブ MIP の) 求解が自分で対称性を検出したか ([`SymScope`] が立てる。求解の始めに立て、終わりに前の値に戻す)
    pub(crate) static SUB_SYM_SOLVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// 対称性の表 (スレッドごとの変数) の退避。サブ MIP の求解の間だけ表を空にし、終わりに外側の表を戻す。
pub(crate) struct SymScope {
    orbitopes: Option<std::rc::Rc<Vec<super::symmetry::Orbitope>>>,
    orig: Option<std::rc::Rc<super::problem::MipProblem>>,
    lex: Option<std::rc::Rc<Vec<Vec<usize>>>>,
    of: Option<std::rc::Rc<Vec<Vec<usize>>>>,
    cut: Option<std::rc::Rc<Vec<Vec<usize>>>>,
    prev_flag: bool,
}

impl SymScope {
    pub(crate) fn enter() -> Self {
        SymScope {
            orbitopes: super::ORBITOPES.with(|t| t.borrow_mut().take()),
            orig: super::SYM_ORIG.with(|t| t.borrow_mut().take()),
            lex: super::SYM_LEX_GENS.with(|t| t.borrow_mut().take()),
            of: super::SYM_OF_GENS.with(|t| t.borrow_mut().take()),
            cut: super::SYM_CUT_GENS.with(|t| t.borrow_mut().take()),
            prev_flag: SUB_SYM_SOLVE.with(|f| f.replace(true)),
        }
    }
}

impl Drop for SymScope {
    fn drop(&mut self) {
        super::ORBITOPES.with(|t| *t.borrow_mut() = self.orbitopes.take());
        super::SYM_ORIG.with(|t| *t.borrow_mut() = self.orig.take());
        super::SYM_LEX_GENS.with(|t| *t.borrow_mut() = self.lex.take());
        super::SYM_OF_GENS.with(|t| *t.borrow_mut() = self.of.take());
        super::SYM_CUT_GENS.with(|t| *t.borrow_mut() = self.cut.take());
        SUB_SYM_SOLVE.with(|f| f.set(self.prev_flag));
    }
}

/// サブ MIP の求解がこのスレッドの対称性の表を使ってよいか (`SUBB_SYM` で自分の表を作った求解の中)。
pub(crate) fn sub_tables() -> bool {
    SUB_SYM_SOLVE.with(|f| f.get())
}

/// 検出の後の表を数える (サブ MIP の検出 1 回分)。
pub(crate) fn note_sub_detection(secs: f64) {
    add(&SYM_STATS[0], 1);
    add(&SYM_STATS[7], (secs * 1e6) as u64);
    let orbs = super::ORBITOPES.with(|t| t.borrow().as_ref().map_or(0, |o| o.len()));
    let of = super::SYM_OF_GENS.with(|t| t.borrow().as_ref().map_or(0, |g| g.len()));
    let lex = super::SYM_LEX_GENS.with(|t| t.borrow().as_ref().map_or(0, |g| g.len()));
    let cols = super::ORBITOPES.with(|t| t.borrow().as_ref().map_or(0, |o| o.iter().map(|o| o.vars.len() * o.vars.first().map_or(0, |l| l.len())).sum::<usize>()));
    if orbs + of + lex > 0 {
        add(&SYM_STATS[1], 1);
        add(&SYM_STATS[2], (of + lex) as u64);
        add(&SYM_STATS[3], orbs as u64);
        add(&SYM_STATS[4], cols as u64);
    }
}

/// 外側の求解の終わりのログ (全スレッドの計数)。
pub(crate) fn report() {
    let l = |a: &[AtomicU64; 8]| a.iter().map(|x| x.load(Relaxed)).collect::<Vec<u64>>();
    if count_on() || pool_on() {
        let d = l(&DIV_STATS);
        eprintln!(
            "MIP: SUBB div: sub-MIPs {}, repeated fingerprints {}, widened {}, narrowed {}, changed after a streak {}, pool crossovers {}, pool mutations {}, improved after a change {} (DIV {}, POOL {})",
            d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7], div_on(), pool_on()
        );
    }
    if sym_on() {
        let s = l(&SYM_STATS);
        eprintln!(
            "MIP: SUBB sym: sub-MIP detections {}, with symmetry {}, generators {}, orbitopes {} (columns {}), orbitopal fixings {}, orbital fixings {}, detection {:.2}s",
            s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7] as f64 * 1e-6
        );
    }
}

/// 外側の求解の始めに計数を消す。
pub(crate) fn reset() {
    for a in DIV_STATS.iter().chain(SYM_STATS.iter()) {
        a.store(0, Relaxed);
    }
}

/// 近傍の多様化の状態 (外側の求解の各スレッドの [`super::solver::Solver`] が持つ)。
#[derive(Default)]
pub(crate) struct DivState {
    /// 指紋 → (見た回数, 最後の結果が解き切り (最適・実行不能) か)
    pub(crate) seen: std::collections::HashMap<u64, (u32, bool)>,
    /// 今解いているサブ MIP の指紋 (結果を覚えるため)
    pub(crate) pending: Option<u64>,
    /// 改善のない回の連続
    pub(crate) streak: u32,
    /// 連続の空振りのうち解き切り (+1) と上限 (-1) の和
    pub(crate) lean: i32,
    /// 今の回で固定を変えたか
    pub(crate) changed: bool,
    /// `SUBB_POOL` の交差と変異を交互に使う番
    pub(crate) pool_turn: u32,
}

use super::domain::FEASTOL;
use super::lp_api::MipLp;
use super::problem::MipProblem;
use super::solver::{MipStatus, Solver};

impl<'a, L: MipLp> Solver<'a, L> {
    /// 固定の集合の指紋: ラベル・行の数・目的関数を変えたか・(大域的に自由な整数列のうち) 固定した列とその値・暫定解の目的値。
    fn subb_fingerprint(&self, sub: &MipProblem, obj_changed: bool, z: f64) -> u64 {
        use std::hash::{Hash, Hasher};
        let p = self.p;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.heur_label.hash(&mut h);
        sub.m.hash(&mut h);
        obj_changed.hash(&mut h);
        z.to_bits().hash(&mut h);
        for j in 0..p.n {
            if p.is_int[j] && sub.col_lo[j] == sub.col_up[j] && self.dom.global_lo[j] < self.dom.global_up[j] {
                j.hash(&mut h);
                sub.col_lo[j].to_bits().hash(&mut h);
            }
        }
        h.finish()
    }

    /// `ENOMOTO_MIP_SUBB_DIV` / `_COUNT`: サブ MIP を解く前に指紋を覚え、(`DIV` なら) 同じ指紋の繰り返し・連続の空振りで固定を変える。
    pub(super) fn subb_diversify(&mut self, sub: &mut MipProblem, obj_changed: bool) {
        self.subb.pending = None;
        self.subb.changed = false;
        let Some((z, inc)) = self.incumbent.as_ref().map(|(z, x)| (*z, x.clone())) else { return };
        let p = self.p;
        if sub.n != p.n {
            return;
        }
        let key = self.subb_fingerprint(sub, obj_changed, z);
        add(&DIV_STATS[0], 1);
        if self.subb.seen.len() > 100_000 {
            self.subb.seen.clear();
        }
        let prev = self.subb.seen.get(&key).copied();
        if prev.is_some() {
            add(&DIV_STATS[1], 1);
        }
        self.subb.seen.entry(key).or_insert((0, false)).0 += 1;
        self.subb.pending = Some(key);
        if !div_on() {
            return;
        }
        let lim = tunable!("ENOMOTO_T_MIP_SUBB_STREAK", 4u32, u32);
        let (dir, frac, by_streak) = match prev {
            // 繰り返し: 前の回が解き切ったなら広げ、上限で止まったなら狭める。割合は 0.2 から回数ごとに 1.5 倍 (最大 0.8)
            Some((cnt, exhausted)) => (if exhausted { 1 } else { -1 }, (tunable!("ENOMOTO_T_MIP_SUBB_FRAC", 0.2, f64) * 1.5f64.powi(cnt as i32 - 1)).min(0.8), false),
            None if self.subb.streak >= lim && self.subb.lean != 0 => (self.subb.lean.signum(), (0.1 * (self.subb.streak - lim + 1) as f64).min(0.6), true),
            None => return,
        };
        let fixed: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j] && sub.col_lo[j] == sub.col_up[j] && self.dom.global_lo[j] < self.dom.global_up[j]).collect();
        let free: Vec<usize> = (0..p.n)
            .filter(|&j| p.is_int[j] && sub.col_lo[j] < sub.col_up[j] && inc[j] >= sub.col_lo[j] - FEASTOL && inc[j] <= sub.col_up[j] + FEASTOL)
            .collect();
        // 固定のない近傍 (local branching・proximity) は狭めるだけ
        let dir = if fixed.is_empty() { -1 } else { dir };
        let mut k = 0usize;
        if dir > 0 {
            for &j in &fixed {
                if self.rand() < frac {
                    sub.col_lo[j] = self.dom.global_lo[j];
                    sub.col_up[j] = self.dom.global_up[j];
                    k += 1;
                }
            }
        } else {
            if free.len() < 10 {
                return;
            }
            for &j in &free {
                if self.rand() < frac {
                    let v = inc[j].round().clamp(sub.col_lo[j], sub.col_up[j]);
                    sub.col_lo[j] = v;
                    sub.col_up[j] = v;
                    k += 1;
                }
            }
        }
        if k == 0 {
            return;
        }
        add(&DIV_STATS[if dir > 0 { 2 } else { 3 }], 1);
        if by_streak {
            add(&DIV_STATS[4], 1);
        }
        self.subb.changed = true;
        if self.params.verbose {
            eprintln!(
                "MIP:   SUBB div: {} {} {k} columns (fraction {frac:.2}, {}; fixed {}, free {}), {:.2}s",
                self.heur_label,
                if dir > 0 { "unfixed" } else { "fixed" },
                if by_streak { format!("streak {}", self.subb.streak) } else { format!("repeat {}", prev.map_or(0, |c| c.0)) },
                fixed.len(),
                free.len(),
                self.start.elapsed().as_secs_f64()
            );
        }
    }

    /// [`Self::subb_diversify`] の後: サブ MIP の結果を覚える。
    pub(super) fn subb_note(&mut self, st: MipStatus, acc: bool) {
        let Some(key) = self.subb.pending.take() else { return };
        let exhausted = matches!(st, MipStatus::Optimal | MipStatus::Infeasible) && !acc;
        if let Some(e) = self.subb.seen.get_mut(&key) {
            e.1 = exhausted;
        }
        if acc {
            if self.subb.changed {
                add(&DIV_STATS[7], 1);
            }
            self.subb.streak = 0;
            self.subb.lean = 0;
        } else {
            self.subb.streak += 1;
            self.subb.lean = (self.subb.lean + if exhausted { 1 } else { -1 }).clamp(-8, 8);
        }
    }

    /// `ENOMOTO_MIP_SUBB_POOL`: 解のプールの複数の解から作る近傍 (交差と変異を交互に)。
    pub(super) fn subb_pool_lns(&mut self) -> bool {
        let p = self.p;
        let np = self.sol_pool.len();
        if self.params.submip || np < 2 {
            return false;
        }
        self.subb.pool_turn += 1;
        let mut lo = self.dom.global_lo.clone();
        let mut up = self.dom.global_up.clone();
        let nint = (0..p.n).filter(|&j| p.is_int[j] && lo[j] < up[j]).count();
        if nint == 0 {
            return false;
        }
        let mut idx: Vec<usize> = (0..np).collect();
        // 乱数で並べ替える
        for i in (1..np).rev() {
            let r = ((self.rand() * (i + 1) as f64) as usize).min(i);
            idx.swap(i, r);
        }
        let crossover = self.subb.pool_turn % 2 == 1;
        let mut nfix = 0usize;
        if crossover {
            // 乱数で選んだ 2〜3 個の解 (最良とは限らない) が一致する列を固定する
            let k = if np >= 3 && self.rand() < 0.5 { 3 } else { 2 };
            let picks = &idx[..k];
            for j in 0..p.n {
                if !p.is_int[j] || lo[j] >= up[j] {
                    continue;
                }
                let v = self.sol_pool[picks[0]].1[j].round();
                if picks.iter().all(|&q| (self.sol_pool[q].1[j] - v).abs() <= FEASTOL) && v >= lo[j] && v <= up[j] {
                    lo[j] = v;
                    up[j] = v;
                    nfix += 1;
                }
            }
            if (nfix as f64) < tunable!("ENOMOTO_T_MIP_SUBB_POOL_XRATE", 0.4, f64) * nint as f64 || nfix == nint {
                return false;
            }
            self.heur_label = "tree pool crossover";
            add(&DIV_STATS[5], 1);
        } else {
            // 最良でない解を中心に、整数列の一部 (割合 `ENOMOTO_T_MIP_SUBB_POOL_MRATE`) を乱数でその値に固定する
            let c = idx.iter().copied().find(|&q| q != 0).unwrap_or(1);
            let rate = tunable!("ENOMOTO_T_MIP_SUBB_POOL_MRATE", 0.7, f64);
            for j in 0..p.n {
                if !p.is_int[j] || lo[j] >= up[j] {
                    continue;
                }
                let v = self.sol_pool[c].1[j].round();
                if self.rand() < rate && v >= lo[j] && v <= up[j] {
                    lo[j] = v;
                    up[j] = v;
                    nfix += 1;
                }
            }
            if nfix == 0 || nfix == nint {
                return false;
            }
            self.heur_label = "tree pool mutation";
            add(&DIV_STATS[6], 1);
        }
        let r = self.solve_submip(lo, up, 500);
        if self.params.verbose {
            eprintln!("MIP:   SUBB pool {} ({nfix} of {nint} fixed, pool {np}): improved {r}", if crossover { "crossover" } else { "mutation" });
        }
        r
    }
}
