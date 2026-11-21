//! `ENOMOTO_MIP_ROOTLP3` (未設定は `e` で**既定で使う**、`=0` で切る。`e` は 2026-11-21 採用、analysis/biglp_20261121 7 節。`s`・`3` は試作、analysis/rootlp3_20261111.md): 主の根の LP (最初・再スタートの後) を、
//! 渡した基底が特異なら基底を捨てて基底なしの同時実行 (二段解法と内点法 + クロスオーバー) で解く。
//!
//! 今の既定では、根の LP に渡した基底 (前処理の LP・再スタートの直前の LP の基底) の LU が特異だと、単体法の中で warm start を
//! やめて全スラック基底から 1 本で解き直す (analysis/basis_nb_diag_20261102.md §2、再スタートの後の根の 89%)。基底がないときの
//! 同時実行 ([`crate::simplex::mip_lp::TwoStageLp::solve`] の `cold`) に入らない。
//!
//! 値の文字: `s` 特異なら基底を捨てる (段階 1)。`e` (analysis/biglp_20261121 9 節) 手間の見積りで決める: 再スタートの後の根で特異なら捨てる (`s` と同じ。最初の根では捨てない、analysis/biglp_20261121 7.10 節。
//! 特異な基底は単体法の中で全スラック基底からの 1 本に落ち、warm start にならない)。特異でなければ、再スタートの後の根 (種類 3) で、
//! 基底の出発の主実行不能な基底変数の数 (双対単体法の反復の下の見積り) が、この求解の最初の根の LP を基底なしで解いた反復
//! (全スラックからの手間の見積り) 以上なら捨てる。閾値は置かない (2 つの見積りの比べ)。`3` 基底からの単体法・全スラック基底からの二段解法・内点法 + クロスオーバーの
//! 3 本同時 (段階 2、[`Solver::rootlp3_before`] の条件)。診断 `ENOMOTO_MIP_ROOTLP3_LOG` は振る舞いを変えず (試作なしでも)、根の LP ごとに
//! `ROOTLP3 {...}` の 1 行 (種類・基底の有無・特異か・状態・反復・秒) を stderr に出す。

use super::lp::{LpStatus, SolveLimits};
use super::lp_api::MipLp;
use super::solver::Solver;
use std::time::Instant;

/// `ENOMOTO_MIP_ROOTLP3` の値 (未設定は `e`、`0`・`off` は空) に文字 `c` が入っているか。
pub(crate) fn rootlp3_has(c: char) -> bool {
    env_def!("ENOMOTO_MIP_ROOTLP3", "e").is_some_and(|v| v.contains(c))
}

/// 試作 `ENOMOTO_MIP_ROOTLP3` の `e`: この求解の最初の主の根の LP (種類 4) を基底なしで解いた反復 (0 なら知らない)
static COLD_ITERS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn rootlp3_log() -> bool {
    env_str!("ENOMOTO_MIP_ROOTLP3_LOG").is_some()
}

/// [`Solver::rootlp3_before`] の結果 (診断の行と 3 本同時の判定に使う)
#[derive(Clone, Copy)]
pub(super) struct RootLp3 {
    /// 基底があったか
    warm: bool,
    /// その基底が特異か
    singular: bool,
    /// 特異なので捨てたか (`s`)、見積りで捨てたか (`e`)
    dropped: bool,
    /// 3 本同時で解くか (`3`)
    race3: bool,
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 主の根の LP (`root_kind` 3 再スタートの後・4 最初。最初の根で基底があるのは LP プロービングの後) を解く前に呼ぶ。
    /// `s` なら、今の基底が特異のとき基底を捨てる (続く `solve` が基底なしの同時実行になる)。`3` なら、特異でない基底があり、
    /// 同時実行のスレッドが 3 本以上で、LP の大きさ (列 + 行 + 非零) が `ENOMOTO_T_MIP_ROOTLP3_SIZE` (既定 50000) 以上のとき、
    /// [`Self::rootlp3_solve`] が 3 本同時で解く。
    pub(super) fn rootlp3_before(&mut self, root_kind: Option<usize>) -> Option<RootLp3> {
        if !matches!(root_kind, Some(3) | Some(4)) || !(rootlp3_has('s') || rootlp3_has('3') || rootlp3_has('e') || rootlp3_log()) {
            return None;
        }
        if root_kind == Some(4) {
            COLD_ITERS.store(0, std::sync::atomic::Ordering::Relaxed);
        }
        let threads = crate::simplex::mip_lp::cold_race_threads();
        let warm = self.lp.has_warm_basis();
        let singular = warm && self.lp.warm_basis_singular();
        // `e` は再スタートの後の根 (`root_kind` 3) だけで捨てる。最初の根 (LP プロービングの基底) で捨てると、根の頂点が変わって
        // 木が大きく揺れる (neos-933966 の種 1・3 で 1.5〜5 倍遅い、analysis/biglp_20261121 7.10 節)。`s` は従来どおり両方。
        let mut dropped = singular && (rootlp3_has('s') || (rootlp3_has('e') && root_kind == Some(3))) && threads >= 2;
        if dropped {
            self.lp.drop_basis();
            if self.params.verbose {
                eprintln!("MIP: root LP: the basis is singular; solving without it (ENOMOTO_MIP_ROOTLP3)");
            }
        }
        // `e`: 特異でない基底からの再スタートの後の根で、warm の出発の主実行不能の数と、最初の根の LP の基底なしの反復を比べる
        let cold_it = COLD_ITERS.load(std::sync::atomic::Ordering::Relaxed);
        if rootlp3_has('e') && warm && !singular && root_kind == Some(3) && threads >= 2 && cold_it > 0 {
            let col = self.lp.basis().col;
            if let Some((pinf, _)) = self.lp.basis_start_info(&col) {
                let drop = pinf as u64 >= cold_it;
                if self.params.verbose {
                    eprintln!(
                        "MIP: ROOTLP3 e: warm basis with {pinf} primal infeasible basic variables, cold estimate {cold_it} iters (the first root LP) -> {}",
                        if drop { "solving without the basis" } else { "warm start" }
                    );
                }
                if drop {
                    self.lp.drop_basis();
                    dropped = true;
                }
            }
        }
        let size = self.p.n + self.p.m + self.p.rows.iter().map(|r| r.len()).sum::<usize>();
        let race3 = rootlp3_has('3') && warm && !singular && threads >= 3 && size as f64 >= tunable!("ENOMOTO_T_MIP_ROOTLP3_SIZE", 50000.0, f64);
        Some(RootLp3 { warm, singular, dropped, race3 })
    }

    /// `3` の判定 ([`Self::rootlp3_before`]) に当たれば、根の LP を 3 本同時 ([`crate::simplex::mip_lp::TwoStageLp::solve_race3`]) で解く。
    pub(super) fn rootlp3_solve(&mut self, info: Option<&RootLp3>) -> Option<LpStatus> {
        if !info.is_some_and(|i| i.race3) {
            return None;
        }
        let lim = SolveLimits { deadline: self.deadline, ..Default::default() };
        Some(self.lp.solve_race3(&lim, crate::simplex::mip_lp::cold_race_threads()))
    }

    /// 診断 `ENOMOTO_MIP_ROOTLP3_LOG`: 根の LP の後に 1 行を出す (`xhash` は根の頂点の揺れを見るための列の値の指紋)。
    pub(super) fn rootlp3_after(&self, root_kind: Option<usize>, info: Option<RootLp3>, st: LpStatus, iters: u64, t0: Instant) {
        let Some(RootLp3 { warm, singular, dropped, race3 }) = info else { return };
        // `e`: 最初の根の LP を基底なしで解いた反復を控える
        // 特異な基底は捨てなくても、単体法の中で基底なしから解き直すので、その反復も控える
        if root_kind == Some(4) && (!warm || dropped || singular) && st == LpStatus::Optimal {
            COLD_ITERS.store(iters.max(1), std::sync::atomic::Ordering::Relaxed);
        }
        let winner = if race3 { crate::simplex::mip_lp::take_race3_winner() } else { "" };
        if rootlp3_log() {
            eprintln!(
                "ROOTLP3 {{\"kind\":{},\"restarts\":{},\"rows\":{},\"warm\":{warm},\"singular\":{singular},\"dropped\":{dropped},\"race3\":\"{winner}\",\"st\":\"{st:?}\",\"iters\":{iters},\"secs\":{:.4},\"obj\":{:.10e},\"xhash\":\"{:016x}\"}}",
                root_kind.unwrap_or(0),
                self.params.restarts,
                self.lp.num_rows(),
                t0.elapsed().as_secs_f64(),
                self.lp.objective(),
                x_hash(&self.lp.col_values()),
            );
        }
    }
}

/// 列の値を 1e-6 に丸めた指紋 (FNV-1a)。同じ種の 2 回で根の頂点が同じかを見る。
fn x_hash(x: &[f64]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &v in x {
        let r = (v * 1e6).round() as i64;
        for b in r.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h
}
