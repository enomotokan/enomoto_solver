//! 試作 `ENOMOTO_MIP_PRECHAIN` (既定では使わない。analysis/prechain_20261110.md): 前処理と FJ・Local-MIP を並べて回し、暫定解と境界を
//! 互いに渡し続ける (docs/backlog.md の「根の前のスレッドの割り当て」の案 1〜3)。
//!
//! 流れ (値の文字で段を選ぶ。`1` は `ebi`):
//! - `e`: 前処理の軽い縮約の後、プロービングなど重い前処理の前に ([`super::presolve_mip_depth`] の LP による固定の前)、
//!   与えられたスレッド数 − 1 本で FJ・Local-MIP を回し始める (種を変えて複数本)。根に入ったら主の根の FJ・Local-MIP のスレッドは
//!   立てず、このスレッドを `ENOMOTO_T_MIP_PRECHAIN_ROOT_KEEP` (1) 本に減らして続ける。根の問題 (対称性の行を足す前の問題) が届いたら
//!   次の区切りで乗り換える (前処理は列の番号を保つので、解は固定された列を根の問題の値にそろえて写す)。
//! - `b`: 暫定解が良くなったら目的の行 `c x <= z - δ` を足した伝播で境界を締め、FJ の範囲を狭める。根に入ってからは主の大域的な
//!   境界 (被約費用による固定など) も渡す。締めた範囲は今の暫定解を外すので、FJ にだけ使い、暫定解から磨く Local-MIP には渡さない。
//! - `i`: スレッドの間と、主 → スレッドの向きに暫定解を渡す (Local-MIP の [`super::localmip::LocalMip::inject`])。
//! - `r`: 根のカットのループの終わりで止めず、再スタートの後の根でも続ける (再スタートの前処理で縮んだ問題に乗り換え、主の
//!   大域的な境界・暫定解を受け取り続ける)。木に入ったら止める。
//! - `R` (`r` を含む。RESTCHAIN、analysis/restchain_20261111.md): `r` に加えて、(1) 根のカットのループの後も打ち切りを延ばして
//!   再スタートの判定 (木に入る直前) まで回し、木に入るときに止める。(2) 再スタートの後の根でスレッドが終わっていれば (打ち切りに
//!   達した・木の中の再スタート)、再スタートの前処理で縮んだ問題で立て直す。(3) 根の終わりの被約費用による固定の後にも主の大域的な
//!   境界・暫定解を渡す。
//! - `p`: 並列のプロービング (`ENOMOTO_MIP_PREPAR`) と分け合う (FJ・Local-MIP をスレッド数 − 2 本、プロービングに 2 本)。
//!
//! 主への解の受け渡しは今の根の FJ のスレッドと同じ受け渡し箱 (根のカットのループの各ラウンドで受け取る) なので、前処理の結果は
//! スレッドの解に左右されない (前処理は決定的なまま)。
use super::problem::MipProblem;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 値 (`1` は `ebi`)
fn val() -> &'static str {
    static V: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    V.get_or_init(|| match env_str!("ENOMOTO_MIP_PRECHAIN") {
        None | Some("0") => String::new(),
        Some("1") => "ebi".to_string(),
        // `R` は `r` を含む
        Some(v) if v.contains('R') => format!("{v}r"),
        Some(v) => v.to_string(),
    })
}

/// `ENOMOTO_MIP_PRECHAIN` が文字 `c` を含むか
pub(crate) fn has(c: char) -> bool {
    val().contains(c)
}

/// 段 (a) を使うか
pub(crate) fn on() -> bool {
    has('e')
}

/// 問題とその版。版 0 は連鎖を立てたときの前処理の途中の問題、前処理で縮むたびに 1 つ増え、最後が根の問題 (`root_ver`)
struct Epoch {
    ver: u64,
    prob: Arc<MipProblem>,
}

/// スレッドの間で分け合う状態
pub(crate) struct Shared {
    stop: Arc<AtomicBool>,
    /// 番号がこれ以上のスレッドは次の区切りで抜ける (根に入ったら減らす)
    active: AtomicUsize,
    /// 走っているスレッドの数 (抜けるときに減らす)
    running: AtomicUsize,
    /// スレッドごとの止める合図 (全体を止めるときと、根に入って減らすときに立てる。FJ・Local-MIP・FJ_P3 の前処理に渡す)
    wstop: Vec<Arc<AtomicBool>>,
    /// 打ち切りの時刻 (根に入るまでは求解の時間の上限、根に入ったら今の FJ のスレッドと同じ秒数)
    deadline: Mutex<(Instant, Instant)>,
    epoch: Mutex<Epoch>,
    epoch_ver: AtomicU64,
    /// 根の問題の版 (根に入るまでは `u64::MAX`)
    root_ver: AtomicU64,
    /// スレッドの最良の解 (版, 目的値, 解)。`best_ser` は変わるたびに増える
    best: Mutex<Option<(u64, f64, Arc<Vec<f64>>)>>,
    best_ser: AtomicU64,
    /// 主から届いた暫定解 (根の問題の空間)
    main_inc: Mutex<Option<(f64, Arc<Vec<f64>>)>>,
    main_ser: AtomicU64,
    /// FJ の範囲 (版, 下限, 上限)。`bnd_ser` は変わるたびに増える
    bounds: Mutex<Option<(u64, Arc<(Vec<f64>, Vec<f64>)>)>>,
    bnd_ser: AtomicU64,
    bnd_busy: AtomicBool,
    /// 根の問題の FJ_P3 の問題 (版, 作れなければ `None`)。`p3_ser` は置くたびに増える
    p3: Mutex<Option<(u64, Option<P3>)>>,
    p3_ser: AtomicU64,
    /// 主への受け渡し箱 (根の問題の空間の解)
    pub(crate) mailbox: Arc<Mutex<Vec<Vec<f64>>>>,
    /// 主が FJ のスレッド以外から暫定解を得たか (立てば延長をやめる)
    pub(crate) has_inc: Arc<AtomicBool>,
    t0: Instant,
    /// 計測: 最初の解の時刻 (前処理の始めから, 秒) と、写しで捨てた解の数、締めた境界の数
    first_at: Mutex<Option<f64>>,
    n_found: AtomicU64,
    n_drop: AtomicU64,
    n_tight: AtomicU64,
    n_inject: AtomicU64,
    /// `R`: 範囲の外の主の暫定解から探し直させた数
    n_seed: AtomicU64,
    verbose: bool,
}

/// 目的値 `z` が `b` より良いか (`b` が無限大なら真。`b - 1e-9 |b|` は無限大で NaN になるので分ける)
fn better(z: f64, b: f64) -> bool {
    !b.is_finite() || z < b - 1e-9 * b.abs().max(1.0)
}

/// 前処理の途中の解 `x` を根の問題 `q` に写す: `q` で固定された列は `q` の値にそろえ、整数列は丸める。`q` で実行可能でなければ `None`
fn map_to(q: &MipProblem, x: &[f64]) -> Option<Vec<f64>> {
    if x.len() != q.n {
        return None;
    }
    let y: Vec<f64> = (0..q.n).map(|j| if q.col_lo[j] == q.col_up[j] { q.col_lo[j] } else if q.is_int[j] { x[j].round() } else { x[j] }).collect();
    q.is_feasible(&y, 1e-6).then_some(y)
}

impl Shared {
    fn epoch(&self) -> (u64, Arc<MipProblem>) {
        let e = self.epoch.lock().unwrap();
        (e.ver, e.prob.clone())
    }

    fn deadline(&self) -> Instant {
        let (t_end, t_ext) = *self.deadline.lock().unwrap();
        if self.has_inc.load(Ordering::Relaxed) { t_end } else { t_ext }
    }

    fn quit(&self, id: usize) -> bool {
        self.stop.load(Ordering::Relaxed) || id >= self.active.load(Ordering::Relaxed) || Instant::now() >= self.deadline() || crate::cancel::is_cancelled()
    }

    /// スレッドが見つけた解 (版 `ver` の問題の空間) を出す。最良より良ければ置き、根の問題の版なら主の受け渡し箱にも入れる。良くなったら真
    fn offer(&self, ver: u64, q: &MipProblem, x: &[f64]) -> bool {
        // 根の問題が届いた後に前処理の途中の問題で見つけた解は、根の問題に写す
        let (cur, qc) = self.epoch();
        if ver < cur {
            return match map_to(&qc, x) {
                Some(y) => self.offer(cur, &qc, &y),
                None => {
                    self.n_drop.fetch_add(1, Ordering::Relaxed);
                    false
                }
            };
        }
        let z = q.objective(x);
        {
            let mut b = self.best.lock().unwrap();
            if b.as_ref().is_some_and(|(bv, bz, _)| *bv == ver && z >= *bz - 1e-9 * bz.abs().max(1.0)) {
                return false;
            }
            // 版が古い最良 (前処理の途中の問題) より新しい版の解を優先する
            if b.as_ref().is_some_and(|(bv, _, _)| *bv > ver) {
                return false;
            }
            *b = Some((ver, z, Arc::new(x.to_vec())));
            self.best_ser.fetch_add(1, Ordering::Release);
        }
        self.n_found.fetch_add(1, Ordering::Relaxed);
        self.first_at.lock().unwrap().get_or_insert(self.t0.elapsed().as_secs_f64());
        if ver == self.root_ver.load(Ordering::Acquire) {
            self.mailbox.lock().unwrap().push(x.to_vec());
        }
        true
    }

    /// 新しい問題を置く (主のスレッドから、前処理で問題が縮んだとき (`root` が偽) と根に入るとき (`root` が真) に呼ぶ)。
    /// 今までの最良の解を写し、根の問題なら主の受け渡し箱にも入れる
    fn publish(&self, q: Arc<MipProblem>, root: bool) {
        let old = self.best.lock().unwrap().clone();
        let ver = {
            let mut e = self.epoch.lock().unwrap();
            e.ver += 1;
            e.prob = q.clone();
            e.ver
        };
        if root {
            self.root_ver.store(ver, Ordering::Release);
            // 主の暫定解は根の問題ごとに渡し直す (再スタートの後の主は新しい問題の空間で渡す)
            *self.main_inc.lock().unwrap() = None;
        }
        self.epoch_ver.store(ver, Ordering::Release);
        if let Some((_, _, x)) = old {
            match map_to(&q, &x) {
                Some(y) => {
                    let z = q.objective(&y);
                    *self.best.lock().unwrap() = Some((ver, z, Arc::new(y.clone())));
                    self.best_ser.fetch_add(1, Ordering::Release);
                    if root {
                        self.mailbox.lock().unwrap().push(y);
                    }
                }
                None => {
                    self.n_drop.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    /// `b`: 目的値 `z` の解が出たので、`c x <= z - δ` を足した伝播で境界を締め、FJ の範囲として置く (ほかのスレッドが締めている間は飛ばす)
    fn tighten(&self, ver: u64, q: &MipProblem, z: f64) {
        if self.bnd_busy.swap(true, Ordering::Acquire) {
            return;
        }
        let t = Instant::now();
        // 目的値の刻み (目的が整数値なら 1 刻み) で少し良い値を求める
        let step = q.objective_step().unwrap_or(0.0);
        let delta = if step > 0.0 { step - 1e-6 * step.max(1.0) } else { 1e-6 * z.abs().max(1.0) };
        let mut rows = q.rows.clone();
        let mut row_lo = q.row_lo.clone();
        let mut row_up = q.row_up.clone();
        rows.push((0..q.n).filter(|&j| q.cost[j] != 0.0).map(|j| (j, q.cost[j])).collect());
        row_lo.push(f64::NEG_INFINITY);
        row_up.push(z - q.offset - delta);
        let (mut lo, mut up) = (q.col_lo.clone(), q.col_up.clone());
        // 主から届いた大域的な境界があれば重ねる
        if let Some((bv, b)) = self.bounds.lock().unwrap().clone() {
            if bv == ver {
                for j in 0..q.n {
                    lo[j] = lo[j].max(b.0[j]);
                    up[j] = up[j].min(b.1[j]);
                }
            }
        }
        let q2 = MipProblem::from_rows(lo, up, q.cost.clone(), q.offset, q.sense_sign, q.is_int.clone(), rows, row_lo, row_up);
        let mut d = super::domain::Domain::new(&q2);
        let ok = d.propagate(&q2);
        let k = (0..q.n).filter(|&j| d.lo[j] > q.col_lo[j] || d.up[j] < q.col_up[j]).count();
        if self.verbose {
            eprintln!("MIP: PRECHAIN: cutoff {z:.6e} propagation: {} bounds tightened{} ({:.3}s)", k, if ok { "" } else { ", infeasible" }, t.elapsed().as_secs_f64());
        }
        // 実行不能 (今の暫定解より良い解がない) なら範囲は置かない (FJ は今の範囲のまま。主の木が最適性を示す)
        if ok && k > 0 {
            self.n_tight.store(k as u64, Ordering::Relaxed);
            *self.bounds.lock().unwrap() = Some((ver, Arc::new((d.lo.clone(), d.up.clone()))));
            self.bnd_ser.fetch_add(1, Ordering::Release);
        }
        self.bnd_busy.store(false, Ordering::Release);
    }
}

/// FJ_P3 の問題 (FJ 用にかけ直した問題) と、その解を元の問題に戻す後処理
type P3 = Arc<(MipProblem, Vec<crate::presolve::PostsolveStep>, Option<crate::presolve::scaling::Scaling>)>;

/// 1 本のスレッド: FJ (種を変えた FJ2) と Local-MIP + 連鎖に時間を分け合う ([`super::heuristics::fj_lmip_mix`] と同じ配分)。
/// 区切りごとに、止める合図・問題の乗り換え・ほかのスレッドと主の解・FJ の範囲を見る。根の問題では、今の根の FJ のスレッドと同じく
/// FJ_P3 の問題 ([`super::fj_p3_problem`]) で探す (番号 0 のスレッドが作り、ほかのスレッドはできたら乗り換える)
fn worker(sh: Arc<Shared>, id: usize, seed: u64) {
    super::fj::SKIP_REPAIR_ON_STOP.with(|c| c.set(true));
    let slice = tunable!("ENOMOTO_T_MIP_FJ_MIX_SLICE", 0.2, f64).max(0.01);
    let lead = tunable!("ENOMOTO_T_MIP_FJ_MIX_LEAD", 0.75, f64).clamp(0.0, 1.0);
    let fj_p3 = super::fj_p3_on();
    let mut k = 0u64;
    let (mut n_fj, mut n_lm, mut n_ep) = (0u64, 0u64, 0u64);
    'epochs: loop {
        n_ep += 1;
        let (ver, q) = sh.epoch();
        let is_root = ver == sh.root_ver.load(Ordering::Acquire);
        // 根の問題: 番号 0 のスレッドが FJ_P3 の問題を作る (止める合図・求解の期限で打ち切る)
        if is_root && fj_p3 && id == 0 && sh.p3.lock().unwrap().as_ref().is_none_or(|(v, _)| *v != ver) {
            let dl = Some(sh.deadline());
            // 止める合図 (主の根のカットのループの終わり) と求解の期限で打ち切る (今の根の FJ のスレッドと同じ)
            let r = crate::cancel::with_halt(Some(sh.wstop[id].clone()), || super::with_presolve_deadline(dl, || super::fj_p3_problem(&q)));
            if sh.verbose {
                if let Some((q3, _, _)) = &r {
                    eprintln!("MIP: PRECHAIN: FJ problem (FJ_P3): rows {} -> {}, free columns {} -> {} ({:.2}s)", q.m, q3.m, (0..q.n).filter(|&j| q.col_lo[j] < q.col_up[j]).count(), (0..q3.n).filter(|&j| q3.col_lo[j] < q3.col_up[j]).count(), sh.t0.elapsed().as_secs_f64());
                }
            }
            *sh.p3.lock().unwrap() = Some((ver, r.map(Arc::new)));
            sh.p3_ser.fetch_add(1, Ordering::Release);
        }
        if sh.quit(id) {
            break 'epochs;
        }
        let seen_p3 = sh.p3_ser.load(Ordering::Acquire);
        let p3: Option<P3> = if is_root { sh.p3.lock().unwrap().as_ref().filter(|(v, _)| *v == ver).and_then(|(_, p)| p.clone()) } else { None };
        // 探す問題 `sp` (FJ_P3 があればそれ) と、その解を版 `ver` の問題 `q` に戻す写し
        let sp: &MipProblem = p3.as_ref().map_or(&*q, |t| &t.0);
        let to_q = |x: Vec<f64>| match &p3 {
            Some(t) => super::fj_p3_solution(x, &t.1, &t.2),
            None => x,
        };
        let to_sp = |x: &[f64]| -> Option<Vec<f64>> {
            match &p3 {
                Some(t) => map_to(&t.0, x),
                None => Some(x.to_vec()),
            }
        };
        let (lo0, up0) = (sp.col_lo.clone(), sp.col_up.clone());
        let mut lm = super::localmip::LocalMip::new(sp, &lo0, &up0, seed ^ 0x6A09_E667_F3BC_C909 ^ ver, true);
        let (mut fj_lo, mut fj_up) = (lo0.clone(), up0.clone());
        let (mut seen_best, mut seen_main, mut seen_bnd) = (u64::MAX, u64::MAX, u64::MAX);
        let mut my_best = f64::INFINITY;
        let (mut t_fj, mut t_lm) = (0.0f64, 0.0f64);
        let mut leader = 0u8;
        loop {
            if sh.quit(id) {
                break 'epochs;
            }
            if sh.epoch_ver.load(Ordering::Acquire) != ver || (is_root && p3.is_none() && sh.p3_ser.load(Ordering::Acquire) != seen_p3) {
                continue 'epochs;
            }
            // `i`: ほかのスレッドの最良・主の暫定解を Local-MIP に渡す (版 `ver` の問題の空間から探す問題の空間へ写す)
            if has('i') {
                let bs = sh.best_ser.load(Ordering::Acquire);
                if bs != seen_best {
                    seen_best = bs;
                    let b = sh.best.lock().unwrap().clone();
                    if let (Some((bv, _, x)), Some(l)) = (b, lm.as_mut()) {
                        if let Some(y) = (bv == ver).then(|| to_sp(&x)).flatten() {
                            let z = sp.objective(&y);
                            if better(z, my_best) && l.inject(&y) {
                                my_best = z;
                                sh.n_inject.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
                let ms = sh.main_ser.load(Ordering::Acquire);
                if ms != seen_main && is_root {
                    seen_main = ms;
                    let b = sh.main_inc.lock().unwrap().clone();
                    if let (Some((_, x)), Some(l)) = (b, lm.as_mut()) {
                        let y = to_sp(&x).filter(|y| sp.is_feasible(y, 1e-6));
                        if let Some(y) = y {
                            let z = sp.objective(&y);
                            if better(z, my_best) && l.inject(&y) {
                                my_best = z;
                                sh.n_inject.fetch_add(1, Ordering::Relaxed);
                            }
                        } else if has('R') && x.len() == sp.n && better(sp.objective(&x), my_best) && l.seed_outside(&x, sp.objective(&x)) {
                            // `R`: 主の暫定解が締まった範囲 (再スタートで固定した列など) の外: 範囲に収めた点から目的の行だけ締めて探す
                            // (目的値は探す問題の費用・定数で測り直す)
                            sh.n_seed.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
            // `b`: FJ の範囲を締める (列の番号は同じ)
            if has('b') {
                let s = sh.bnd_ser.load(Ordering::Acquire);
                if s != seen_bnd {
                    seen_bnd = s;
                    if let Some((bv, b)) = sh.bounds.lock().unwrap().clone() {
                        if bv == ver && b.0.len() == sp.n {
                            for j in 0..sp.n {
                                let (l, u) = (lo0[j].max(b.0[j]), up0[j].min(b.1[j]));
                                if l <= u {
                                    fj_lo[j] = l;
                                    fj_up[j] = u;
                                }
                            }
                        }
                    }
                }
            }
            let t_end = sh.deadline();
            let lm_share = match leader {
                0 => 0.5,
                2 => lead,
                _ => 1.0 - lead,
            };
            let use_lm = lm.as_ref().is_some_and(|l| !l.done) && t_lm < lm_share * (t_fj + t_lm) - 1e-9;
            let t = Instant::now();
            let got: Option<Vec<f64>> = if use_lm {
                let l = lm.as_mut().unwrap();
                let before = l.best_obj();
                l.run((t + Duration::from_secs_f64(slice)).min(t_end), Some(&*sh.wstop[id]), &|_| {});
                t_lm += t.elapsed().as_secs_f64();
                n_lm += 1;
                if l.best_obj() < before { l.res.best.clone() } else { None }
            } else {
                let left = t_end.saturating_duration_since(t).as_secs_f64();
                // 今の根の FJ のスレッドと同じく、FJ の LP 修復などを有効にする ([`crate::set_fj_p3_force`])
                crate::set_fj_p3_force(fj_p3);
                let r = super::fj::fj2_search(sp, &fj_lo, &fj_up, left, seed.wrapping_add(k.wrapping_mul(0x9E37_79B9_7F4A_7C15)), Some(&*sh.wstop[id]));
                crate::set_fj_p3_force(false);
                k += 1;
                n_fj += 1;
                t_fj += t.elapsed().as_secs_f64();
                r
            };
            if let Some(x) = got {
                let z = sp.objective(&x);
                if better(z, my_best) {
                    my_best = z;
                    leader = if use_lm { 2 } else { 1 };
                    if !use_lm {
                        if let Some(l) = lm.as_mut() {
                            l.inject(&x);
                        }
                    }
                    let y = to_q(x);
                    if q.is_feasible(&y, 1e-6) {
                        let zq = q.objective(&y);
                        if sh.offer(ver, &q, &y) && has('b') && q.nnz <= 20_000_000 {
                            sh.tighten(ver, &q, zq);
                        }
                    } else {
                        sh.n_drop.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
            if lm.as_ref().is_none_or(|l| l.done) && t_end.saturating_duration_since(Instant::now()).as_secs_f64() < 1e-3 {
                break 'epochs;
            }
        }
    }
    super::fj::SKIP_REPAIR_ON_STOP.with(|c| c.set(false));
    sh.running.fetch_sub(1, Ordering::AcqRel);
    if sh.verbose && env_str!("ENOMOTO_MIP_FJ_DEBUG").is_some() {
        eprintln!("MIP: PRECHAIN thread {id}: {n_fj} FJ runs, {n_lm} Local-MIP slices, epochs {n_ep}, ended at {:.2}s", sh.t0.elapsed().as_secs_f64());
    }
}

/// 主のスレッドが持つ連鎖 (分け合う状態と、スレッドの手)
pub(crate) struct Chain {
    pub(crate) sh: Arc<Shared>,
    handles: Vec<std::thread::JoinHandle<()>>,
}

impl Chain {
    /// スレッドが全部終わったか
    pub(crate) fn all_done(&self) -> bool {
        self.handles.iter().all(|h| h.is_finished())
    }

    /// 止めて待つ
    pub(crate) fn finish(self) {
        self.sh.stop.store(true, Ordering::Relaxed);
        for w in &self.sh.wstop {
            w.store(true, Ordering::Relaxed);
        }
        for h in self.handles {
            let _ = h.join();
        }
        let sh = &self.sh;
        if sh.verbose {
            eprintln!(
                "MIP: PRECHAIN summary: found {} improvements (first at {}), dropped {} when mapped to the root problem, injected {}, last tightened bounds {}, seeded outside {}",
                sh.n_found.load(Ordering::Relaxed),
                sh.first_at.lock().unwrap().map_or("-".to_string(), |t| format!("{t:.2}s")),
                sh.n_drop.load(Ordering::Relaxed),
                sh.n_inject.load(Ordering::Relaxed),
                sh.n_tight.load(Ordering::Relaxed),
                sh.n_seed.load(Ordering::Relaxed)
            );
        }
    }
}

thread_local! {
    /// 最初の求解の前処理の始めに置く: (スレッドの数, 求解の時間の上限 (秒), 前処理の始めの時刻, ログ)
    static PENDING: std::cell::Cell<Option<(usize, f64, Instant, bool)>> = const { std::cell::Cell::new(None) };
    /// 走っている連鎖 (前処理で立て、根で受け取る)
    static CUR: std::cell::RefCell<Option<Chain>> = const { std::cell::RefCell::new(None) };
}

/// 最初の求解の始めに呼ぶ ([`super::solve_problem_inner`])
pub(crate) fn arm(threads: usize, time_limit: f64, verbose: bool) {
    if on() && threads >= 2 {
        PENDING.with(|p| p.set(Some((threads, time_limit, Instant::now(), verbose))));
    }
}

/// 最初の求解の終わりに呼ぶ: 残っていれば止める
pub(crate) fn disarm() {
    PENDING.with(|p| p.set(None));
    if let Some(c) = CUR.with(|c| c.borrow_mut().take()) {
        c.finish();
    }
}

/// `p` のとき、並列のプロービングに回すスレッドの数
pub(crate) fn prepar_share(threads: usize) -> Option<usize> {
    (has('p') && threads >= 3).then_some(2)
}

/// `k` 本のスレッドを問題 `q` で立てる。`root` なら `q` を根の問題 (版 0) とする (`R` の再スタートの後の立て直し)。
/// `salt` は種に混ぜる (再スタートの回数)
fn spawn(q: Arc<MipProblem>, k: usize, deadline: (Instant, Instant), t0: Instant, verbose: bool, salt: u64, root: bool) -> (Arc<Shared>, Vec<std::thread::JoinHandle<()>>) {
    let sh = Arc::new(Shared {
        stop: Arc::new(AtomicBool::new(false)),
        active: AtomicUsize::new(k),
        running: AtomicUsize::new(k),
        wstop: (0..k).map(|_| Arc::new(AtomicBool::new(false))).collect(),
        deadline: Mutex::new(deadline),
        epoch: Mutex::new(Epoch { ver: 0, prob: q }),
        epoch_ver: AtomicU64::new(0),
        root_ver: AtomicU64::new(if root { 0 } else { u64::MAX }),
        best: Mutex::new(None),
        best_ser: AtomicU64::new(0),
        main_inc: Mutex::new(None),
        main_ser: AtomicU64::new(0),
        bounds: Mutex::new(None),
        bnd_ser: AtomicU64::new(0),
        bnd_busy: AtomicBool::new(false),
        p3: Mutex::new(None),
        p3_ser: AtomicU64::new(0),
        mailbox: Default::default(),
        has_inc: Arc::new(AtomicBool::new(false)),
        t0,
        first_at: Mutex::new(None),
        n_found: AtomicU64::new(0),
        n_drop: AtomicU64::new(0),
        n_tight: AtomicU64::new(0),
        n_inject: AtomicU64::new(0),
        n_seed: AtomicU64::new(0),
        verbose,
    });
    let base = tunable!("ENOMOTO_T_MIP_SEED", 0u64, u64).wrapping_mul(0xD1B5_4A32_D192_ED03) ^ 0x5851_F42D_4C95_7F2D ^ salt.wrapping_mul(0xA076_1D64_78BD_642F);
    let handles = (0..k)
        .map(|id| {
            let sh = sh.clone();
            let seed = base.wrapping_add((id as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
            std::thread::spawn(move || worker(sh, id, seed))
        })
        .collect();
    (sh, handles)
}

/// 前処理の途中 (軽い縮約の後、重い前処理の前) で呼ぶ: 立っていれば FJ・Local-MIP のスレッドを立てる。戻り値はプロービングに回すスレッドの数
pub(crate) fn start(prob: &MipProblem) -> Option<usize> {
    let (threads, tl, t0, verbose) = PENDING.with(|p| p.take())?;
    if super::heuristics::fj_feas_extend(prob) || prob.n == 0 {
        return None;
    }
    let par = prepar_share(threads);
    let k = threads.saturating_sub(par.unwrap_or(1)).max(1);
    let far = if tl.is_finite() { t0 + Duration::from_secs_f64(tl.max(0.0)) } else { t0 + Duration::from_secs(1 << 30) };
    let (sh, handles) = spawn(Arc::new(prob.clone()), k, (far, far), t0, verbose, 0, false);
    if verbose {
        eprintln!("MIP: PRECHAIN: started {k} FJ/Local-MIP threads on the problem in the presolve ({} rows, {} free columns, {:.2}s)", prob.m, (0..prob.n).filter(|&j| prob.col_lo[j] < prob.col_up[j]).count(), t0.elapsed().as_secs_f64());
    }
    CUR.with(|c| *c.borrow_mut() = Some(Chain { sh, handles }));
    par
}

/// 前処理の途中で問題が縮んだら呼ぶ ([`super::presolve_mip_depth`] の連鎖を立てる位置を、かけ直しの前処理が通るたび):
/// 自由な列が今の問題の `ENOMOTO_T_MIP_PRECHAIN_REFRESH` (0.8) 倍以下になっていれば、スレッドを次の区切りでその問題に乗り換えさせる
pub(crate) fn refresh(prob: &MipProblem) {
    CUR.with(|c| {
        let c = c.borrow();
        let Some(c) = c.as_ref() else { return };
        let (_, q0) = c.sh.epoch();
        if q0.n != prob.n {
            return;
        }
        let free = |q: &MipProblem| (0..q.n).filter(|&j| q.col_lo[j] < q.col_up[j]).count();
        let (f0, f1) = (free(&q0), free(prob));
        if (f1 as f64) <= tunable!("ENOMOTO_T_MIP_PRECHAIN_REFRESH", 0.8, f64) * f0 as f64 || prob.m as f64 <= 0.8 * q0.m as f64 {
            if c.sh.verbose {
                eprintln!("MIP: PRECHAIN: the FJ/Local-MIP threads move to the smaller problem in the presolve ({} -> {} rows, {f0} -> {f1} free columns, {:.2}s)", q0.m, prob.m, c.sh.t0.elapsed().as_secs_f64());
            }
            c.sh.publish(Arc::new(prob.clone()), false);
        }
    });
}

/// 根に入るときに主が呼ぶ: 走っている連鎖があれば根の問題 `q` を渡し、分け合う状態を返す (`None` なら今までどおり主が FJ のスレッドを
/// 立てる)。連鎖そのものは主のスレッドに置いたまま ([`stop_current`] か [`disarm`] で止める)。`t_end` と `t_ext` は今の FJ のスレッドと
/// 同じ打ち切り (解があるとき / 主が FJ のスレッド以外の暫定解を持たないとき)。`r` なら再スタートの後の根でも呼び、再スタートの
/// 前処理で縮んだ問題に乗り換えさせる
pub(crate) fn attach(q: Arc<MipProblem>, t_end: Instant, t_ext: Instant, restarts: u32, verbose: bool) -> Option<Arc<Shared>> {
    let cur = CUR.with(|c| c.borrow().as_ref().map(|c| c.sh.clone()));
    let alive = cur.as_ref().is_some_and(|sh| sh.epoch().1.n == q.n && !sh.stop.load(Ordering::Relaxed) && sh.running.load(Ordering::Acquire) > 0);
    if !alive {
        stop_current();
        // `R`: 再スタートの後の根で、スレッドが終わっていれば (打ち切りに達した・木に入って止めた) 縮んだ問題で立て直す
        if !(has('R') && restarts > 0 && q.n > 0) {
            return None;
        }
        let k = tunable!("ENOMOTO_T_MIP_PRECHAIN_ROOT_KEEP", 1usize, usize).max(1);
        let (sh, handles) = spawn(q.clone(), k, (t_end, t_ext.max(t_end)), Instant::now(), verbose, restarts as u64, true);
        if verbose {
            eprintln!("MIP: PRECHAIN R: restarted {k} FJ/Local-MIP threads on the problem after restart {restarts} ({} rows, {} free columns)", q.m, (0..q.n).filter(|&j| q.col_lo[j] < q.col_up[j]).count());
        }
        CUR.with(|c| *c.borrow_mut() = Some(Chain { sh: sh.clone(), handles }));
        return Some(sh);
    }
    let sh = cur.unwrap();
    *sh.deadline.lock().unwrap() = (t_end, t_ext.max(t_end));
    let keep = tunable!("ENOMOTO_T_MIP_PRECHAIN_ROOT_KEEP", 1usize, usize).max(1);
    sh.active.fetch_min(keep, Ordering::Relaxed);
    for w in sh.wstop.iter().skip(keep) {
        w.store(true, Ordering::Relaxed);
    }
    sh.publish(q, true);
    Some(sh)
}

/// `R`: 根のカットのループの終わりに呼ぶ。打ち切りを `t` (求解の期限) まで延ばし、再スタートの判定 (木に入る直前) まで回す
pub(crate) fn extend_to(t: Instant) {
    CUR.with(|c| {
        if let Some(c) = c.borrow().as_ref() {
            *c.sh.deadline.lock().unwrap() = (t, t);
        }
    });
}

/// 走っている連鎖を止めて待つ (根のカットのループの終わり、`r` なら木に入るとき)
pub(crate) fn stop_current() {
    if let Some(c) = CUR.with(|c| c.borrow_mut().take()) {
        c.finish();
    }
}

/// 連鎖が走っているか (主のスレッド)
pub(crate) fn running() -> bool {
    CUR.with(|c| c.borrow().as_ref().is_some_and(|c| !c.all_done()))
}

impl Shared {
    /// スレッドが全部終わったか
    pub(crate) fn all_done(&self) -> bool {
        self.running.load(Ordering::Acquire) == 0
    }

    /// 主の暫定解を渡す (`i`)
    pub(crate) fn post_main(&self, z: f64, x: &[f64]) {
        if !has('i') {
            return;
        }
        let mut m = self.main_inc.lock().unwrap();
        if m.as_ref().is_some_and(|(mz, _)| z >= *mz - 1e-9 * mz.abs().max(1.0)) {
            return;
        }
        *m = Some((z, Arc::new(x.to_vec())));
        self.main_ser.fetch_add(1, Ordering::Release);
    }

    /// 主の大域的な境界を渡す (`b`、根の問題の空間)
    pub(crate) fn post_bounds(&self, lo: &[f64], up: &[f64]) {
        if !has('b') {
            return;
        }
        let mut b = self.bounds.lock().unwrap();
        let (l, u) = match b.as_ref() {
            Some((v, old)) if *v == self.root_ver.load(Ordering::Acquire) && old.0.len() == lo.len() => ((0..lo.len()).map(|j| lo[j].max(old.0[j])).collect(), (0..lo.len()).map(|j| up[j].min(old.1[j])).collect()),
            _ => (lo.to_vec(), up.to_vec()),
        };
        *b = Some((self.root_ver.load(Ordering::Acquire), Arc::new((l, u))));
        self.bnd_ser.fetch_add(1, Ordering::Release);
    }
}
