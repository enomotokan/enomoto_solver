//! 内点法の解から整数解を作るヒューリスティクス (HiGHS の central rounding に、クロスオーバーの頂点の丸めを加えたもの)。
//!
//! 根の LP (カット込み) を内点法で解いた点 `x_c` は、最適面 (費用 0 なら実行可能領域) の中心付近にある。
//! 単体法の頂点 `x_lp` は境界に張り付いていて丸めると実行不能になりやすいが、中心に寄せた点は行に余裕があるので
//! 丸めが通りやすい。次の点を丸め、整数列を (整数に近い順に) 固定・伝播し、連続部分を LP で解いて試す
//! ([`Solver::propagate_rounding_from`]):
//!
//! 1. 直線 `x(α) = x_lp + α (x_c - x_lp)` 上の点 (`α = 1, 0.8, …, 0.2`。丸めた整数列が同じ点は 1 回だけ)。
//! 2. `x_c` からクロスオーバーの押し出し (主の押し出し・基底の選択・Megiddo 式の押し出し) だけで作った頂点。
//!    この頂点の連続部分の LP は、押し出しで得た基底から始める。この基底は主実行可能な頂点を与えるが
//!    **双対実行可能とは限らない** (仕上げの単体法を走らせていない) ので、LP の最適基底としては扱わない
//!    (下界・被約費用に使わない)。warm start にだけ使い、二段解法の warm start が双対実行不能な列を
//!    被約費用の符号の側 (無限なら記号的な M) に置き直すか費用をずらして始め、真の費用で仕上げる。

use super::lp_api::MipLp;
use super::solver::Solver;
use std::time::{Duration, Instant};

impl<'a, L: MipLp> Solver<'a, L> {
    /// 内点法の解からの丸め。`xlp` は根の LP の (単体法の) 解。
    pub(super) fn interior_rounding(&mut self, xlp: &[f64]) -> bool {
        let p = self.p;
        if self.params.submip || !p.is_int.iter().any(|&b| b) {
            return false;
        }
        // 時間の上限: 全体の 5% (時間制限がなければ 10 秒)、残り時間の半分まで
        let frac = tunable!("ENOMOTO_T_MIP_IPM_TIME_FRAC", 0.05, f64);
        let mut budget = if self.params.time_limit.is_finite() { frac * self.params.time_limit } else { 10.0 };
        if let Some(d) = self.deadline {
            budget = budget.min(0.5 * d.saturating_duration_since(Instant::now()).as_secs_f64());
        }
        if budget <= 0.01 {
            return false;
        }
        let deadline = Instant::now() + Duration::from_secs_f64(budget);
        // 0: 費用つき (最適面の中心付近) + クロスオーバーの頂点、1: 費用 0 (実行可能領域の中心付近、頂点なし)、2: 両方
        let mode = tunable!("ENOMOTO_T_MIP_IPM_MODE", 0u8, u8);
        let mut centers: Vec<(&str, Vec<f64>)> = Vec::new();
        let mut vertex: Option<(Vec<usize>, Vec<f64>)> = None;
        let t0 = Instant::now();
        if mode != 1 {
            match self.lp.interior_point(false, env_str!("ENOMOTO_MIP_IPM_NO_VERTEX").is_none(), deadline) {
                Some(ip) => {
                    centers.push(("optimal face", ip.x));
                    if let Some((b, Some(xv))) = ip.vertex {
                        vertex = Some((b, xv));
                    }
                }
                None => {
                    if self.params.verbose {
                        eprintln!("MIP:   interior rounding: IPM (cost) failed ({:.2}s)", t0.elapsed().as_secs_f64());
                    }
                }
            }
        }
        if mode != 0 && Instant::now() < deadline {
            if let Some(ip) = self.lp.interior_point(true, false, deadline) {
                centers.push(("analytic center", ip.x));
            }
        }
        if self.params.verbose {
            eprintln!("MIP:   interior rounding: {} interior points, vertex {} ({:.2}s)", centers.len(), vertex.is_some(), t0.elapsed().as_secs_f64());
        }
        // 丸めて試す点 (名前, 点, 連続部分の LP の開始基底)
        let mut seen: std::collections::HashSet<Vec<i64>> = std::collections::HashSet::new();
        let key = |x: &[f64]| -> Vec<i64> { (0..p.n).filter(|&j| p.is_int[j]).map(|j| x[j].round() as i64).collect() };
        let mut tried = 0usize;
        let try_point = |s: &mut Self, x: &[f64], start: Option<(&[usize], &[f64])>, seen: &mut std::collections::HashSet<Vec<i64>>, tried: &mut usize| -> bool {
            if !seen.insert(key(x)) {
                return false;
            }
            *tried += 1;
            // 整数に近い列から固定する
            let mut order: Vec<usize> = (0..p.n).filter(|&j| p.is_int[j]).collect();
            order.sort_by(|&a, &b| (x[a] - x[a].round()).abs().total_cmp(&(x[b] - x[b].round()).abs()));
            s.propagate_rounding_from(x, &order, None, start)
        };
        let mut found = false;
        let alphas = [1.0, 0.8, 0.6, 0.4, 0.2];
        for (_, xc) in &centers {
            for &a in &alphas {
                if found || self.time_up() || Instant::now() >= deadline {
                    break;
                }
                let x: Vec<f64> = (0..p.n).map(|j| xlp[j] + a * (xc[j] - xlp[j])).collect();
                found |= try_point(self, &x, None, &mut seen, &mut tried);
            }
        }
        if let Some((b, xv)) = &vertex {
            if !found && !self.time_up() {
                found |= try_point(self, xv, Some((b.as_slice(), xv.as_slice())), &mut seen, &mut tried);
            }
        }
        if self.params.verbose {
            eprintln!("MIP:   interior rounding: {tried} rounded points tried, found {found}");
        }
        found
    }
}
