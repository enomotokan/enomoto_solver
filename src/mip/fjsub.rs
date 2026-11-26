//! 試作 `ENOMOTO_MIP_FJSUB` (既定では使わない、analysis/treeclose_20261114.md 9.39 節、利用者の案): 木の探索の間、背景のスレッドで
//! FJ・Local-MIP を全体の問題に回し続け、主のスレッドが受け渡し箱に置いた「有望な部分問題」(ノードの定義域と LP 解、暫定解があれば
//! RINS 型の近傍も選べる) に、全体の問題で停滞したとき (または手間を半分ずつ) 差し替える。主のスレッドの時間は使わない
//! (主は箱に置くのと、見つかった解を受け取るだけ)。
//!
//! 今の根の FJ・Local-MIP のスレッド ([`super::heuristics::fj_lmip_mix`]) は根のカットのループの終わりで止めるので、木の間は別の
//! スレッドとして立てる。決定性は気にしていない (時計は壁時計)。受け渡しは箱 1 つ (最新の候補だけ) と解の箱なので、後で
//! `ENOMOTO_MIP_TICKS` の `j` (`ticks::Gate`) の区切りで読み書きする形に載せ替えられる。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::problem::MipProblem;

/// 主が置く部分問題の候補 (元の問題 = 主の `self.p` の空間)
pub(crate) struct FjSubCand {
    pub lo: Vec<f64>,
    pub up: Vec<f64>,
    /// ノードの LP 解 (Local-MIP の出発点、整数列は四捨五入される)
    pub x: Vec<f64>,
}

/// 主と背景のスレッドの受け渡し
#[derive(Default)]
pub(crate) struct FjSubShared {
    /// 候補の箱 (最新の 1 つだけ。古いものは上書き)
    pub cand: Mutex<Option<FjSubCand>>,
    /// 見つかった解 (全体の問題・部分問題のどちらも。部分問題の解も元の問題で実行可能)
    pub sols: Mutex<Vec<(Vec<f64>, bool)>>,
    pub stop: AtomicBool,
    /// 統計: 候補を置いた数 (主が数える)、差し替えた数、部分問題で見つけた解の数、部分問題に使った秒・全体の秒 (ナノ秒)
    pub n_offer: AtomicU64,
    /// 置かなかった数: 分数値の整数列が多すぎる、ノードの下界が範囲の外
    pub n_rej_frac: AtomicU64,
    pub n_rej_win: AtomicU64,
    pub n_swap: AtomicU64,
    pub n_subsol: AtomicU64,
    pub ns_sub: AtomicU64,
    pub ns_all: AtomicU64,
}

/// `ENOMOTO_MIP_FJSUB` の文字 (値 `1` は既定の形): `r` = 暫定解があれば RINS 型の近傍 (暫定解と LP 解が一致する整数列を固定) を候補にする、
/// `h` = 停滞を待たず全体と部分問題に時間を半分ずつ、`w` = 潜りの打ち切りのほか、`ENOMOTO_T_MIP_FJSUB_WORK` (既定 5 万) 反復ごとにも候補を置く
pub(crate) fn fjsub_on(c: char) -> bool {
    match env_str!("ENOMOTO_MIP_FJSUB") {
        None | Some("0") => false,
        Some(v) => c == '*' || v.contains(c),
    }
}

/// 背景のスレッドを立てる。`p` は主の問題 (木の問題)、`inc` は主の暫定解 (あれば全体の Local-MIP の出発点に入れる)
pub(crate) fn spawn(p: MipProblem, inc: Option<Vec<f64>>, seed: u64) -> (Arc<FjSubShared>, std::thread::JoinHandle<()>) {
    let sh = Arc::new(FjSubShared::default());
    let sh2 = sh.clone();
    let h = std::thread::spawn(move || run(&p, inc, seed, &sh2));
    (sh, h)
}

fn run(p: &MipProblem, inc: Option<Vec<f64>>, seed: u64, sh: &FjSubShared) {
    let slice = tunable!("ENOMOTO_T_MIP_FJSUB_SLICE", 0.2, f64).max(0.01);
    let stall = tunable!("ENOMOTO_T_MIP_FJSUB_STALL", 2.0, f64);
    let budget = tunable!("ENOMOTO_T_MIP_FJSUB_BUDGET", 1.0, f64).max(slice);
    let half = fjsub_on('h');
    let Some(mut lm) = super::localmip::LocalMip::new(p, &p.col_lo, &p.col_up, seed ^ 0x6A09_E667_F3BC_C909, true) else { return };
    if let Some(x) = &inc {
        lm.inject(x);
    }
    let mut best = inc.as_ref().map_or(f64::INFINITY, |x| p.objective(x));
    let mut last_imp = Instant::now();
    let (mut t_sub, mut t_all) = (0.0f64, 0.0f64);
    let mut k = 0u64;
    let stop = &sh.stop;
    let push = |x: &[f64], sub: bool| sh.sols.lock().unwrap().push((x.to_vec(), sub));
    while !stop.load(Ordering::Relaxed) {
        k += 1;
        let swap = {
            let has = sh.cand.lock().unwrap().is_some();
            has && if half { t_sub <= t_all } else { last_imp.elapsed().as_secs_f64() >= stall }
        };
        let t = Instant::now();
        if swap {
            let Some(c) = sh.cand.lock().unwrap().take() else { continue };
            sh.n_swap.fetch_add(1, Ordering::Relaxed);
            // 絞った範囲で FJ (FJ2 は自分の出発点から)、見つからなければ LP 解を丸めた点から Local-MIP。合わせて `budget` 秒まで
            let mut found = super::fj::fj2_search(p, &c.lo, &c.up, 0.5 * budget, seed.wrapping_add(k.wrapping_mul(0x9E37_79B9_7F4A_7C15)), Some(stop));
            if found.is_none() {
                if let Some(mut ls) = super::localmip::LocalMip::new(p, &c.lo, &c.up, seed ^ k, true) {
                    ls.set_start(&c.x);
                    let left = (budget - t.elapsed().as_secs_f64()).max(slice);
                    ls.run(Instant::now() + Duration::from_secs_f64(left), Some(stop), &|_| {});
                    found = ls.res.best.clone();
                }
            }
            if let Some(x) = found {
                sh.n_subsol.fetch_add(1, Ordering::Relaxed);
                let z = p.objective(&x);
                push(&x, true);
                if super::solver::better_obj(z, best, best) {
                    best = z;
                    last_imp = Instant::now();
                }
                lm.inject(&x);
            }
            t_sub += t.elapsed().as_secs_f64();
        } else {
            // 全体の問題: Local-MIP と FJ2 を 1 切れずつ交互に
            let got = if k % 2 == 0 && !lm.done {
                let before = lm.best_obj();
                lm.run(Instant::now() + Duration::from_secs_f64(slice), Some(stop), &|_| {});
                if lm.best_obj() < before { lm.res.best.clone() } else { None }
            } else {
                super::fj::fj2_search(p, &p.col_lo, &p.col_up, slice, seed.wrapping_add(k.wrapping_mul(0x9E37_79B9_7F4A_7C15)), Some(stop))
            };
            if let Some(x) = got {
                let z = p.objective(&x);
                if super::solver::better_obj(z, best, best) {
                    best = z;
                    last_imp = Instant::now();
                    push(&x, false);
                    lm.inject(&x);
                }
            }
            t_all += t.elapsed().as_secs_f64();
        }
        sh.ns_sub.store((t_sub * 1e9) as u64, Ordering::Relaxed);
        sh.ns_all.store((t_all * 1e9) as u64, Ordering::Relaxed);
    }
}
