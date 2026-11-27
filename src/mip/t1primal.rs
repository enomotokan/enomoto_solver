//! 試作 `ENOMOTO_MIP_T1PRIMAL` (既定では使わない、analysis/t1primal_20261126、gap240 の型 1): 1 スレッドの解探しの手間の配分。
//!
//! 4 スレッドでは FJ のスレッドが根の間ずっと FJ + Local-MIP の組 ([`super::heuristics::fj_lmip_mix`]) を回し、30n20b8・CMS750_4 の
//! 最初の解・最適解はそこから出る (30n20b8 は 4.2 秒で 302)。1 スレッドではこの組が回らず、最初の解は木から 400 秒台に出る。
//! 取り下げた T1MIX (根で 1 回だけ回す) は、易しい問題にも根までの時間ぶんを足して P の 1 スレッドで +6.5% だった
//! (analysis/t1gap_20261121 6 節)。
//!
//! ここでは、4 スレッドの FJ のスレッド (CPU の取り分 [`role_frac`]) を 1 スレッドで時分けして真似る:
//! - 補助のスレッドを 1 本作り、FJ と Local-MIP の状態をその中に持つ (止めた所から続ける)。主はそのスレッドに「s 秒回せ」と
//!   頼んで待つので、同時に動く CPU は常に 1 本。
//! - 取り分 s = [`role_frac`] × σ(logit h + logit f + logit g)。入力は 3 つの割合 (どれも 0〜1) で、新しい閾値は置かない:
//!   - h: 当たった率 (改善した回 + 1) / (回した回 + 2)。外れが続くと下がる。
//!   - f: 経過時間のうち、最後に暫定解が良くなってから (なければ始めから) の割合。改善が止まっているほど上がる。
//!   - g: 暫定解と下界の相対ギャップ (暫定解がなければ 1)。ギャップが小さい (証明だけの) 問題では下がる。
//!   - f と g は h と同じ回数 n (回した回) で (x·n + 1)/(n + 2) にならす。初めの版は g = 1 (markshare_4_0: 暫定解が最適で下界 0)
//!     や f = 1 (暫定解なし) で logit が +∞ になり、外れが続いても取り分が上限に張り付いた (P 1 スレッド +0.6%、README 3 節)。
//!     ならすと、外れが続けば logit h の下がりが f・g の上がりと打ち消し合い、取り分が下がる。
//! - 3 版目: 要る側 (f・g) は平均して h と重みをそろえ (σ(logit h + (logit f + logit g)/2))、取り分に (n + 1)/(n + 2) を掛けて
//!   事前を 0 寄りにする (n = 0 で取り分 0.0625、回すほど外れる)。短い問題で事前の 0.125 がそのまま効く損を減らす。
//! - 主は、経過時間 × s から使った秒を引いた「貯え」が [`slice_min`] 秒 (Local-MIP の 1 切れ) 以上たまったときだけ、
//!   根の終わりと木のノード (潜りの外) でその貯えの分だけ回す。易しい問題では貯えがたまる前に終わる。
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::adapt::{laplace, logit, sigmoid};
use super::lp_api::MipLp;
use super::problem::MipProblem;
use super::solver::Solver;

/// `ENOMOTO_MIP_T1PRIMAL` が付いているか (`0` は使わない)。
pub(super) fn enabled() -> bool {
    env_str!("ENOMOTO_MIP_T1PRIMAL").is_some_and(|v| v != "0")
}

/// 取り分の上限: 4 スレッドのヒューリスティクス役の割合 (`ENOMOTO_T_MIP_HEUR_ROLE_FRAC`、既定 0.25) と同じ数。
fn role_frac() -> f64 {
    tunable!("ENOMOTO_T_MIP_HEUR_ROLE_FRAC", 0.25, f64)
}

/// 1 回に回す最短の秒: Local-MIP の 1 切れ (`ENOMOTO_T_MIP_FJ_MIX_SLICE`、既定 0.2 秒、[`super::heuristics::fj_lmip_mix`] と同じ)。
fn slice_min() -> f64 {
    tunable!("ENOMOTO_T_MIP_FJ_MIX_SLICE", 0.2, f64).max(0.01)
}

enum Msg {
    /// この秒数だけ回して、良くなった解を返す
    Slice(f64),
    Quit,
}

struct Reply {
    sols: Vec<Vec<f64>>,
    used: f64,
    fj_runs: u64,
    lm_slices: u64,
}

/// 補助のスレッドの中の FJ + Local-MIP の状態 (止めた所から続ける)。中身は [`super::heuristics::fj_lmip_mix`] の繰り返しと同じ。
struct Mix<'p> {
    p: &'p MipProblem,
    lo: Vec<f64>,
    up: Vec<f64>,
    seed: u64,
    lm: Option<super::localmip::LocalMip<'p>>,
    t_fj: f64,
    t_lm: f64,
    leader: u8,
    best: Option<f64>,
    k: u64,
}

impl<'p> Mix<'p> {
    fn run(&mut self, secs: f64, stop: &AtomicBool) -> Reply {
        let slice = slice_min();
        let lead = tunable!("ENOMOTO_T_MIP_FJ_MIX_LEAD", 0.75, f64).clamp(0.0, 1.0);
        let t0 = Instant::now();
        let now = || t0.elapsed().as_secs_f64();
        let mut sols = Vec::new();
        let (mut fj_runs, mut lm_slices) = (0u64, 0u64);
        super::fj::SKIP_REPAIR_ON_STOP.with(|c| c.set(true));
        while now() < secs && !stop.load(Ordering::Relaxed) {
            let lm_share = match self.leader {
                0 => 0.5,
                2 => lead,
                _ => 1.0 - lead,
            };
            let use_lm = self.lm.as_ref().is_some_and(|l| !l.done) && self.t_lm < lm_share * (self.t_fj + self.t_lm) - 1e-9;
            let t = now();
            let left = (secs - t).max(0.0);
            let got = if use_lm {
                let l = self.lm.as_mut().unwrap();
                let before = l.best_obj();
                l.run(Instant::now() + Duration::from_secs_f64(slice.min(left)), Some(stop), &|_| {});
                self.t_lm += now() - t;
                lm_slices += 1;
                if l.best_obj() < before { l.res.best.clone() } else { None }
            } else {
                let r = super::fj::fj2_search(self.p, &self.lo, &self.up, left, self.seed.wrapping_add(self.k.wrapping_mul(0x9E37_79B9_7F4A_7C15)), Some(stop));
                self.k += 1;
                self.t_fj += now() - t;
                fj_runs += 1;
                r
            };
            if let Some(x) = got {
                let z = self.p.objective(&x);
                if self.best.is_none_or(|bz| super::solver::better_obj(z, bz, bz)) {
                    self.leader = if use_lm { 2 } else { 1 };
                    if !use_lm {
                        if let Some(l) = self.lm.as_mut() {
                            l.inject(&x);
                        }
                    }
                    self.best = Some(z);
                    sols.push(x);
                }
            }
            if self.lm.as_ref().is_none_or(|l| l.done) && secs - now() < 1e-3 {
                break;
            }
        }
        super::fj::SKIP_REPAIR_ON_STOP.with(|c| c.set(false));
        Reply { sols, used: now(), fj_runs, lm_slices }
    }
}

/// 主の側の状態 ([`Solver`] の `t1p`)。落とすと補助のスレッドを止めて待つ。
pub(super) struct T1Primal {
    tx: mpsc::Sender<Msg>,
    rx: mpsc::Receiver<Reply>,
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
    /// 回した秒の和
    spent: f64,
    slices: u32,
    hits: u32,
    /// 最後に見た暫定解の値と、それが良くなった時刻 (求解の始めからの秒)
    last_z: Option<f64>,
    last_imp: f64,
    fj_runs: u64,
    lm_slices: u64,
}

impl Drop for T1Primal {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.tx.send(Msg::Quit);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 補助のスレッドを立てる (根のカットのループの後、1 スレッド・外側の求解・最初の走りだけ)。
    pub(super) fn t1primal_start(&mut self) {
        if self.t1p.is_some() {
            return;
        }
        let (q, lo, up) = match &self.orig {
            Some(o) => ((**o).clone(), o.col_lo.clone(), o.col_up.clone()),
            None => (self.p.clone(), self.dom.lo.clone(), self.dom.up.clone()),
        };
        let q = Arc::new(q);
        let seed = self.rng ^ 0x9E37_79B9_7F4A_7C15;
        let (tx, rx_in) = mpsc::channel::<Msg>();
        let (tx_out, rx) = mpsc::channel::<Reply>();
        let stop = Arc::new(AtomicBool::new(false));
        let st = stop.clone();
        let handle = std::thread::Builder::new()
            .name("t1primal".into())
            .stack_size(64 << 20)
            .spawn(move || {
                let p: &MipProblem = &q;
                let lm = super::localmip::LocalMip::new(p, &lo, &up, seed ^ 0x6A09_E667_F3BC_C909, true);
                let mut mix = Mix { p, lo: lo.clone(), up: up.clone(), seed, lm, t_fj: 0.0, t_lm: 0.0, leader: 0, best: None, k: 0 };
                while let Ok(Msg::Slice(secs)) = rx_in.recv() {
                    let r = mix.run(secs, &st);
                    if tx_out.send(r).is_err() {
                        break;
                    }
                }
            })
            .expect("failed to spawn the T1PRIMAL thread");
        let now = self.start.elapsed().as_secs_f64();
        self.t1p = Some(Box::new(T1Primal {
            tx,
            rx,
            stop,
            handle: Some(handle),
            spent: 0.0,
            slices: 0,
            hits: 0,
            last_z: self.incumbent.as_ref().map(|(z, _)| *z),
            last_imp: if self.incumbent.is_some() { now } else { 0.0 },
            fj_runs: 0,
            lm_slices: 0,
        }));
    }

    /// 取り分 s (上の説明) と、その入力 (h, f, g)。
    fn t1primal_share(&self, now: f64) -> (f64, f64, f64, f64) {
        let Some(t) = self.t1p.as_ref() else { return (0.0, 0.0, 0.0, 0.0) };
        // h・f・g は同じ回数 n (切れ端の数) でラプラスの式 (x·n+1)/(n+2) にならす。外れが続くと logit h の下がりが
        // logit f・logit g の上がり (暫定解がすでに最適で g が 1 のまま、暫定解なしで f が 1 のまま) と打ち消し合い、取り分が下がる。
        // 共通の関数 (super::adapt、FVbj の DIVESEL と同じ式)
        let n = t.slices as f64;
        let h = (t.hits as f64 + 1.0) / (n + 2.0);
        let f = laplace(if now > 0.0 { (now - t.last_imp) / now } else { 1.0 }, n);
        let g = laplace(self.gap_frac(), n);
        // 「効いているか」(h) 1 つに対し「要るか」(f・g) が 2 つなので、要る側は平均して重みをそろえる (3 版目)。
        // 2 版目の和では、h と g が打ち消し合ったあと f だけが残り、markshare_4_0 で取り分が 0.21 に残った (README 7・8 節)。
        // 事前は 0 寄り: 回した回数 n で (n + 1)/(n + 2) を掛け、回すほど事前 (n = 0 で 1/2) を外す (3 版目)。
        let s = role_frac() * sigmoid(logit(h) + 0.5 * (logit(f) + logit(g))) * laplace(1.0, n);
        (s, h, f, g)
    }

    /// 貯えがたまっていれば補助のスレッドを回し、出た解を暫定解に試す。根の終わり (`at_root`) と木のノードで呼ぶ。
    pub(super) fn t1primal_tick(&mut self, at_root: bool) {
        let now = self.start.elapsed().as_secs_f64();
        let z = self.incumbent.as_ref().map(|(z, _)| *z);
        {
            let Some(t) = self.t1p.as_mut() else { return };
            if z != t.last_z {
                t.last_z = z;
                t.last_imp = now;
            }
        }
        let (s, h, f, g) = self.t1primal_share(now);
        let t = self.t1p.as_ref().unwrap();
        // 貯えは求解の始めから数える。始めた時刻から数えると、根の終わりに貯えがなく、木では新しい潜りの頭でしか
        // 呼ばれないので、長く潜る 30n20b8 の最初の切れ端が 57 秒まで遅れた (3 版目の試し、README 8 節)。
        let bank = s * now - t.spent;
        if bank < slice_min() {
            return;
        }
        let secs = bank.min(0.5 * self.secs_left());
        if secs < slice_min() {
            return;
        }
        let t = self.t1p.as_mut().unwrap();
        if t.tx.send(Msg::Slice(secs)).is_err() {
            return;
        }
        let Ok(r) = t.rx.recv() else { return };
        t.spent += r.used;
        t.slices += 1;
        t.fj_runs += r.fj_runs;
        t.lm_slices += r.lm_slices;
        let n = r.sols.len();
        let z0 = self.incumbent.as_ref().map(|(z, _)| *z);
        let mut acc = false;
        for x in r.sols.into_iter().rev() {
            self.heur_label = "T1PRIMAL";
            let ok = self.try_incumbent(x);
            self.heur_label = "";
            if ok {
                acc = true;
                break;
            }
        }
        let t = self.t1p.as_mut().unwrap();
        if acc {
            t.hits += 1;
        }
        if self.params.verbose {
            eprintln!(
                "MIP: T1PRIMAL{}: ran {:.2}s (bank {bank:.2}s, share {s:.3}: h {h:.3} f {f:.3} g {g:.3}), {n} improving, accepted {acc}, incumbent {z0:?} -> {:?}, slices {} hits {} spent {:.2}s, FJ runs {} Local-MIP slices {} ({:.2}s)",
                if at_root { " (root)" } else { "" },
                r.used,
                self.incumbent.as_ref().map(|(z, _)| *z),
                t.slices,
                t.hits,
                t.spent,
                t.fj_runs,
                t.lm_slices,
                self.start.elapsed().as_secs_f64()
            );
        }
        if acc {
            t.last_z = self.incumbent.as_ref().map(|(z, _)| *z);
            t.last_imp = self.start.elapsed().as_secs_f64();
        }
    }
}
