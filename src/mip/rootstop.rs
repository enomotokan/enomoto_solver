//! 根の段を打ち切って木に移る基準をそろえる試作 (ROOTSTOP、`ENOMOTO_MIP_ROOTSTOP`、既定では使わない。analysis/rootstop_20261111.md)
//!
//! docs/backlog.md の「根の段 (前処理・ヒューリスティクス・再スタート・カット・根の LP) を打ち切って木に移る基準を 1 つにそろえる」の案を、
//! 段ごとに文字で選べるようにする。値は文字の並び (例 `rb`)。
//! - `r` (案 1): 根のカットのループ (外側の主の根だけ。サブ MIP・裏のカットの写しでは使わない) を「双対限界の上がり ÷ 手間」で止める。
//!   手間はそのラウンドの LP の反復 (時刻では答えが揺れるので使わない)。基準の速さ = ループの始めから下界が初めて動いたラウンドまでの
//!   上がり ÷ その間の反復の和 (外側の最初の根で決め、再スタートの後の根も同じ値を使う)。直近 `ENOMOTO_T_MIP_ROOTSTOP_WIN` (既定 3)
//!   ラウンドの速さが基準の `ENOMOTO_T_MIP_ROOTSTOP_RATE` (既定 0.05) 倍を下回ったら止める。基準が決まった後は、今までの停滞の回数の
//!   判定 (SCIP 式の 10 回) は使わない (ラウンドの上限・時間の上限・目的値が整数刻みの判定はそのまま効く)。下界が一度も動いて
//!   いない間は今までの判定のまま。
//! - `b` (案 6): 根の終わりで先に再スタートを見込み (新たに固定できた整数列が再スタートの閾値以上)、見込まれる根では裏のカットを起こさない。
//!
//! 試作 `ENOMOTO_MIP_CUTSTALL_GAP`・`ENOMOTO_MIP_HO6_STALL`・`ENOMOTO_MIP_CUT_STALL_HIGHS` とは併用しない前提 (併用したときは `r` が先に判定する)。

use std::cell::Cell;

/// `ENOMOTO_MIP_ROOTSTOP` に文字 `c` が入っているか
pub(crate) fn on(c: char) -> bool {
    env_str!("ENOMOTO_MIP_ROOTSTOP").is_some_and(|v| v.contains(c))
}

thread_local! {
    /// `r` の基準の速さ (外側の最初の根で決めた値。再スタートの後の根も使う。決まっていなければ NaN)
    static REF_RATE: Cell<f64> = const { Cell::new(f64::NAN) };
}

/// [`RateStop::record`] の答え
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Rate {
    /// 基準がまだ決まらない (下界が動いていない) か、窓が埋まっていない: 今までの判定に任せる
    Undecided,
    /// 速さが基準の割合以上: 今までの停滞の回数は数えない
    Continue,
    /// 速さが基準の割合を下回った: ループを止める
    Stop,
}

/// `r` の判定の状態 (根のカットのループ 1 回分)
pub(crate) struct RateStop {
    first_obj: f64,
    /// 基準の速さ (このループで決めたか、前の根から引き継いだもの)
    ref_rate: Option<f64>,
    /// ループの始めからの反復の和 (基準が決まるまで)
    work0: f64,
    /// 基準が決まった後のラウンドの (目的値, 反復)
    hist: Vec<(f64, f64)>,
    win: usize,
    frac: f64,
    /// 直近の窓の速さと基準 (ログ用)
    pub(crate) last: (f64, f64),
}

impl RateStop {
    /// `restarts` は再スタートの回数 (0 なら外側の最初の根で、基準を決め直す)
    pub(crate) fn new(first_obj: f64, restarts: u32) -> Self {
        let inherited = if restarts == 0 {
            REF_RATE.with(|r| r.set(f64::NAN));
            None
        } else {
            REF_RATE.with(|r| r.get()).is_finite().then(|| REF_RATE.with(|r| r.get()))
        };
        RateStop {
            first_obj,
            ref_rate: inherited,
            work0: 0.0,
            hist: Vec::new(),
            win: tunable!("ENOMOTO_T_MIP_ROOTSTOP_WIN", 3usize, usize).max(1),
            frac: tunable!("ENOMOTO_T_MIP_ROOTSTOP_RATE", 0.05, f64),
            last: (f64::NAN, f64::NAN),
        }
    }

    /// 1 ラウンドの後に呼ぶ: そのラウンドの後の目的値 (前処理の定数を除いた LP の値) と、そのラウンドの LP の反復
    pub(crate) fn record(&mut self, obj: f64, iters: u64) -> Rate {
        let w = iters.max(1) as f64;
        let tiny = 1e-9 * obj.abs().max(1.0);
        let Some(rr) = self.ref_rate else {
            self.work0 += w;
            let gain = obj - self.first_obj;
            if gain > tiny {
                let r = gain / self.work0;
                self.ref_rate = Some(r);
                REF_RATE.with(|c| {
                    if !c.get().is_finite() {
                        c.set(r);
                    }
                });
                self.hist.push((obj, 0.0));
            }
            return Rate::Undecided;
        };
        if self.hist.is_empty() {
            // 引き継いだ基準: 今のループの始めの値から測る
            self.hist.push((self.first_obj, 0.0));
        }
        self.hist.push((obj, w));
        let k = self.hist.len();
        if k <= self.win {
            return Rate::Undecided;
        }
        let gain = obj - self.hist[k - 1 - self.win].0;
        let work: f64 = self.hist[k - self.win..].iter().map(|h| h.1).sum();
        let rate = gain.max(0.0) / work.max(1.0);
        self.last = (rate, rr);
        if rate < self.frac * rr { Rate::Stop } else { Rate::Continue }
    }
}

impl<'a, L: super::lp_api::MipLp> super::solver::Solver<'a, L> {
    /// `b`: 裏のカットを起こす前に呼ぶ。再スタートが見込まれるなら真 (ログを出す)。見込みは、すでにある新たな固定に、今の LP の
    /// 被約費用と打ち切り値で固定できる整数列 ([`Self::rootstop_redcost_would_fix`]) を足して数える (裏のカットを起こす時点では、
    /// 被約費用による固定はまだ記録されていないことが多い)
    pub(super) fn rootstop_skip_bg(&self) -> bool {
        let would = self.rootstop_redcost_would_fix();
        let skip = self.restart_expected(would);
        if self.params.verbose {
            eprintln!(
                "MIP: ROOTSTOP b: newly fixed {} + fixable by reduced costs {would} at the background cut start, restart {}expected; background cuts {} ({:.2}s)",
                self.newly_fixed_ints(),
                if skip { "" } else { "not " },
                if skip { "skipped" } else { "started" },
                self.start.elapsed().as_secs_f64()
            );
        }
        skip
    }

    /// 今の LP (カットは大域的に成り立つので緩和) の被約費用と打ち切り値で固定できる、まだ固定されていない整数列の数
    /// ([`Self::lose6_3_cut_restart_due`] と同じ数え方)。暫定解がなければ 0
    pub(super) fn rootstop_redcost_would_fix(&self) -> usize {
        use super::lp::VarStatus;
        let p = self.p;
        if self.incumbent.is_none() {
            return 0;
        }
        let gap = self.prune_limit() - (self.lp.objective() + p.offset);
        if !gap.is_finite() || gap < 0.0 {
            return 0;
        }
        let b = self.lp.basis_ref();
        let d = self.lp.reduced_costs_ref();
        (0..p.n)
            .filter(|&j| {
                let (lo, up) = (self.dom.global_lo[j], self.dom.global_up[j]);
                p.is_int[j]
                    && lo < up
                    && up - lo >= 1.0
                    && match b.col[j] {
                        VarStatus::Lower if d[j] > 1e-7 => (gap / d[j]).floor() < 1.0,
                        VarStatus::Upper if d[j] < -1e-7 => (gap / -d[j]).floor() < 1.0,
                        _ => false,
                    }
            })
            .count()
    }
}
