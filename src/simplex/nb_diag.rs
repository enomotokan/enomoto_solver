//! 診断用 (`ENOMOTO_MIP_NB_DIAG`): [`super::mip_lp::TwoStageLp::set_basis`] に渡された基底 ([`crate::mip::lp::Basis`]) の
//! 非基底列・行の置き場所 (Upper / Lower) と、次の求解が実際に使う置き場所の希望 (`nb_hint`、直前に解いた点 `x` から
//! 推し量る) の食い違いを、`set_basis` を呼んだ場所ごとに数える (analysis/basis_nb_diag_20261102.md)。
//! 求解の振る舞いは変えない (数えるだけ)。結果は [`crate::mip::solve_mip`] の終わりに 1 場所 1 行で出す。
//!
//! 数えるもの (1 回の `set_basis` の直後の求解 1 回ごと):
//! - `cmp`: 保存した置き場所が Lower / Upper (Zero・基底を除く) で、今も非基底で、固定でない列・行の数
//! - `mis`: そのうち `nb_hint` が保存した側と違う数 (`opp`: 反対の側、`unk`: 0 (どちらの境界にもない))
//! - `eff`: 求解の始めに実際に置いた側 (被約費用の符号と希望で決まる) が保存した側と違う数
//! - `fix`: `eff` のうち、保存した側を希望として渡せばその側に置かれた数 (真の費用での被約費用がその側で双対実行可能)
//!
//! LP は `mis > 0` かどうか、`fix > 0` かどうかで分け、反復数と時間を足す。

use std::cell::Cell;
use std::sync::Mutex;

/// `set_basis` を呼んだ場所
pub(crate) const UNTAGGED: u8 = 0;
pub(crate) const TREE_SIBLING: u8 = 1;
pub(crate) const TREE_QUEUE: u8 = 2;
pub(crate) const TREE_POOL: u8 = 3;
pub(crate) const SUBMIP_ROOT: u8 = 4;
pub(crate) const BG_COPY: u8 = 5;
pub(crate) const BG_RETURN: u8 = 6;
pub(crate) const RESTART: u8 = 7;
pub(crate) const ROOT_PROBE: u8 = 8;
pub(crate) const TREE_ROOT: u8 = 9;
pub(crate) const SLACK_RETRY: u8 = 10;
/// `set_basis` の後でない求解 (前の解からそのまま)
pub(crate) const NO_SET: u8 = 11;
const NSITE: usize = 12;
const NAMES: [&str; NSITE] =
    ["untagged", "tree_sibling", "tree_queue", "tree_pool", "submip_root", "bg_copy", "bg_return", "restart", "root_probe", "tree_root", "slack_retry", "no_set_basis"];

pub(crate) fn on() -> bool {
    env_str!("ENOMOTO_MIP_NB_DIAG").is_some()
}

thread_local! {
    static SITE: Cell<u8> = const { Cell::new(UNTAGGED) };
    static IN_SUB: Cell<bool> = const { Cell::new(false) };
    /// 求解の始めの置き場所と比べる保存した置き場所 (標準形の列ごと、-1 / +1 / 0)
    static SAVED: std::cell::RefCell<Option<Vec<i8>>> = const { std::cell::RefCell::new(None) };
    /// 求解の始めの比較の結果 (eff, fix)
    static EFF: Cell<Option<(u32, u32)>> = const { Cell::new(None) };
}

/// 次の `set_basis` の場所の印を付ける (その `set_basis` が読んで消す)
pub(crate) fn tag(site: u8) {
    if on() {
        SITE.with(|s| s.set(site));
    }
}

pub(crate) fn take_tag() -> u8 {
    SITE.with(|s| s.replace(UNTAGGED))
}

/// このスレッドがサブ MIP を解いている間 `true` (戻り値の値に戻すこと)
pub(crate) fn set_in_sub(v: bool) -> bool {
    IN_SUB.with(|s| s.replace(v))
}

/// `set_basis` の時点で覚えておくもの
#[derive(Clone)]
pub(crate) struct Pending {
    site: u8,
    /// 標準形の列ごとの保存した置き場所 (-1 下限、+1 上限、0 基底・Zero・不明)
    saved: Vec<i8>,
    compared: bool,
    cmp: u32,
    opp: u32,
    unk: u32,
}

impl Pending {
    /// `col`・`row` は渡された基底の状態、`sigma` は行の符号 (スラック s_i = b_i - σ_i a_i x が 0 なら、σ > 0 で行の上限、
    /// σ < 0 で下限)
    pub(crate) fn new(col: &[crate::mip::lp::VarStatus], row: &[crate::mip::lp::VarStatus], sigma: &[f64], n: usize, m: usize) -> Self {
        use crate::mip::lp::VarStatus as V;
        let mut saved = vec![0i8; n + m];
        for j in 0..n.min(col.len()) {
            saved[j] = match col[j] {
                V::Lower => -1,
                V::Upper => 1,
                _ => 0,
            };
        }
        for i in 0..m.min(row.len()) {
            let at_up = match row[i] {
                V::Upper => true,
                V::Lower => false,
                _ => continue,
            };
            saved[n + i] = if at_up == (sigma[i] > 0.0) { -1 } else { 1 };
        }
        Pending { site: take_tag(), saved, compared: false, cmp: 0, opp: 0, unk: 0 }
    }

    /// 求解の始めの `nb_hint` と比べる (`basis` は今の基底、`lb`・`ub` は標準形の境界)。比べた保存した置き場所を
    /// [`take_saved`] に渡す
    pub(crate) fn compare(&mut self, hint: &[i8], basis: &[usize], lb: &[f64], ub: &[f64]) {
        if hint.len() != self.saved.len() || lb.len() < hint.len() {
            return;
        }
        let mut is_basic = vec![false; hint.len()];
        for &v in basis {
            if v < is_basic.len() {
                is_basic[v] = true;
            }
        }
        let mut sv = self.saved.clone();
        for v in 0..hint.len() {
            let s = self.saved[v];
            if s == 0 || is_basic[v] || lb[v] == ub[v] {
                sv[v] = 0;
                continue;
            }
            self.cmp += 1;
            if hint[v] == 0 {
                self.unk += 1;
            } else if hint[v] != s {
                self.opp += 1;
            }
        }
        self.compared = true;
        SAVED.with(|x| *x.borrow_mut() = Some(sv));
        EFF.with(|e| e.set(None));
    }
}

/// 求解の始めに置き場所と比べる保存した置き場所 (`ENOMOTO_MIP_NB_DIAG` で `set_basis` の直後の求解のときだけ)
pub(crate) fn take_saved() -> Option<Vec<i8>> {
    SAVED.with(|x| x.borrow_mut().take())
}

pub(crate) fn set_eff(eff: u32, fix: u32) {
    EFF.with(|e| e.set(Some((eff, fix))));
}

#[derive(Default, Clone, Copy)]
struct Stat {
    n: u64,
    n_cmp: u64,
    n_mis: u64,
    n_fix: u64,
    /// 比べたが、求解の始めの置き場所を数えられなかった LP (warm start の初期化を通らなかった)
    n_noeff: u64,
    sum_cmp: u64,
    sum_opp: u64,
    sum_unk: u64,
    sum_eff: u64,
    sum_fix: u64,
    it_mis: u64,
    it_ok: u64,
    t_mis: f64,
    t_ok: f64,
    it_fix: u64,
    t_fix: f64,
    max_it: u64,
}

static STATS: Mutex<Vec<Stat>> = Mutex::new(Vec::new());

pub(crate) fn reset() {
    if let Ok(mut s) = STATS.lock() {
        *s = vec![Stat::default(); 2 * NSITE];
    }
}

/// 1 回の求解を足す (`p` は `set_basis` の後の最初の求解なら `Some`)
pub(crate) fn record(p: Option<Pending>, iters: u64, secs: f64) {
    let sub = IN_SUB.with(|s| s.get());
    let site = p.as_ref().map_or(NO_SET, |p| p.site) as usize;
    let eff = EFF.with(|e| e.take());
    SAVED.with(|x| *x.borrow_mut() = None);
    let Ok(mut st) = STATS.lock() else { return };
    if st.len() != 2 * NSITE {
        *st = vec![Stat::default(); 2 * NSITE];
    }
    let s = &mut st[site + if sub { NSITE } else { 0 }];
    s.n += 1;
    s.max_it = s.max_it.max(iters);
    let (mis, fix) = match &p {
        Some(p) if p.compared => {
            s.n_cmp += 1;
            s.sum_cmp += p.cmp as u64;
            s.sum_opp += p.opp as u64;
            s.sum_unk += p.unk as u64;
            if eff.is_none() {
                s.n_noeff += 1;
            }
            let (e, f) = eff.unwrap_or((0, 0));
            s.sum_eff += e as u64;
            s.sum_fix += f as u64;
            (p.opp + p.unk > 0, f > 0)
        }
        _ => (false, false),
    };
    if mis {
        s.n_mis += 1;
        s.it_mis += iters;
        s.t_mis += secs;
    } else {
        s.it_ok += iters;
        s.t_ok += secs;
    }
    if fix {
        s.n_fix += 1;
        s.it_fix += iters;
        s.t_fix += secs;
    }
}

/// 場所ごとに 1 行ずつ出す (`total` は求解全体の経過時間)
pub(crate) fn print(total: f64) {
    let Ok(st) = STATS.lock() else { return };
    eprintln!("NB_DIAG total {total:.3}");
    for (k, s) in st.iter().enumerate() {
        if s.n == 0 {
            continue;
        }
        let (sub, site) = (k >= NSITE, NAMES[k % NSITE]);
        eprintln!(
            "NB_DIAG {} {site} n={} n_cmp={} n_mis={} n_fix={} n_noeff={} sum_cmp={} sum_opp={} sum_unk={} sum_eff={} sum_fix={} it_mis={} it_ok={} t_mis={:.4} t_ok={:.4} it_fix={} t_fix={:.4} max_it={}",
            if sub { "sub" } else { "main" },
            s.n,
            s.n_cmp,
            s.n_mis,
            s.n_fix,
            s.n_noeff,
            s.sum_cmp,
            s.sum_opp,
            s.sum_unk,
            s.sum_eff,
            s.sum_fix,
            s.it_mis,
            s.it_ok,
            s.t_mis,
            s.t_ok,
            s.it_fix,
            s.t_fix,
            s.max_it
        );
    }
}
