//! 試作 `ENOMOTO_MIP_ROOTLP3` (既定では使わない、analysis/rootlp3_20261111.md): 主の根の LP (最初・再スタートの後) を、
//! 渡した基底が特異なら基底を捨てて基底なしの同時実行 (二段解法と内点法 + クロスオーバー) で解く。
//!
//! 今の既定では、根の LP に渡した基底 (前処理の LP・再スタートの直前の LP の基底) の LU が特異だと、単体法の中で warm start を
//! やめて全スラック基底から 1 本で解き直す (analysis/basis_nb_diag_20261102.md §2、再スタートの後の根の 89%)。基底がないときの
//! 同時実行 ([`crate::simplex::mip_lp::TwoStageLp::solve`] の `cold`) に入らない。
//!
//! 値の文字: `s` 特異なら基底を捨てる (段階 1)。診断 `ENOMOTO_MIP_ROOTLP3_LOG` は振る舞いを変えず (試作なしでも)、根の LP ごとに
//! `ROOTLP3 {...}` の 1 行 (種類・基底の有無・特異か・状態・反復・秒) を stderr に出す。

use super::lp::LpStatus;
use super::lp_api::MipLp;
use super::solver::Solver;
use std::time::Instant;

/// 試作 `ENOMOTO_MIP_ROOTLP3` の値に文字 `c` が入っているか。
pub(crate) fn rootlp3_has(c: char) -> bool {
    env_str!("ENOMOTO_MIP_ROOTLP3").is_some_and(|v| v.contains(c))
}

fn rootlp3_log() -> bool {
    env_str!("ENOMOTO_MIP_ROOTLP3_LOG").is_some()
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 主の根の LP (`root_kind` 3 再スタートの後・4 最初) を解く前に呼ぶ。試作 `s` なら、今の基底が特異のとき基底を捨てる
    /// (続く `solve` が基底なしの同時実行になる)。戻り値は診断の行に使う (基底があったか, 特異か, 捨てたか)
    pub(super) fn rootlp3_before(&mut self, root_kind: Option<usize>) -> Option<(bool, bool, bool)> {
        if !matches!(root_kind, Some(3) | Some(4)) || !(rootlp3_has('s') || rootlp3_log()) {
            return None;
        }
        let warm = self.lp.has_warm_basis();
        let singular = warm && self.lp.warm_basis_singular();
        let drop = singular && rootlp3_has('s') && crate::simplex::mip_lp::cold_race_threads() >= 2;
        if drop {
            self.lp.drop_basis();
            if self.params.verbose {
                eprintln!("MIP: root LP: the basis is singular; solving without it (ENOMOTO_MIP_ROOTLP3)");
            }
        }
        Some((warm, singular, drop))
    }

    /// 診断 `ENOMOTO_MIP_ROOTLP3_LOG`: 根の LP の後に 1 行を出す (`xhash` は根の頂点の揺れを見るための列の値の指紋)。
    pub(super) fn rootlp3_after(&self, root_kind: Option<usize>, info: Option<(bool, bool, bool)>, st: LpStatus, iters: u64, t0: Instant) {
        let Some((warm, singular, drop)) = info else { return };
        if rootlp3_log() {
            eprintln!(
                "ROOTLP3 {{\"kind\":{},\"restarts\":{},\"rows\":{},\"warm\":{warm},\"singular\":{singular},\"dropped\":{drop},\"st\":\"{st:?}\",\"iters\":{iters},\"secs\":{:.4},\"obj\":{:.10e},\"xhash\":\"{:016x}\"}}",
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
