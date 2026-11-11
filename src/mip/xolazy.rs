//! 課題 XOLAZY (analysis/xolazy_20261111.md) の診断 `ENOMOTO_MIP_XOLAZY_LOG` (振る舞いは変えない): 主の根の LP の同時実行で内点法 +
//! クロスオーバーが勝ったとき、クロスオーバーを省いたら使えた双対 (止められた二段解法の途中の基底の双対・内点法の双対) で、
//! 根の終わり (再スタートの判断・木に入る時点) の打ち切り値と大域的な境界のもとで被約費用による固定が何本できたかを数える。
//!
//! どの双対 `y` でもラグランジュ緩和の下界 `L(y)` は正しい下界で、整数列 `j` は `L(y) + |d_j| (x_j - 境界) > U` なら境界に固定できる
//! (`d = c - A^T y`、全部の列の境界が有限なとき)。最適な双対 (クロスオーバーの後の根の LP) でも同じ数え方をして比べる
//! (カットのループの後の LP の被約費用による実際の固定とは別)。

use super::lp_api::MipLp;
use super::solver::Solver;
use std::cell::RefCell;

struct Saved {
    /// 根の LP の目的値 (定数項を含む、打ち切り値と同じ単位)
    z: f64,
    restarts: u32,
    /// 根の LP の時点の大域的な境界 (整数列の固定を数える基準)
    lo: Vec<f64>,
    up: Vec<f64>,
    /// 診断 `ENOMOTO_MIP_IPMRC_WAIT`: 二段解法が勝った後に内点法の点を待った秒
    t_wait: Option<f64>,
    /// (名前, 最適値との下界の差, 構造列の被約費用)
    duals: Vec<(&'static str, f64, Vec<f64>)>,
}

thread_local! {
    static SAVED: RefCell<Option<Saved>> = const { RefCell::new(None) };
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 根の LP の直後に呼ぶ (同時実行の記録 `d` を受け取り、内点法が勝っていれば双対を控える)。控えた双対の下界の差と、
    /// 最適な双対の被約費用と `lp.reduced_costs` の食い違い (向きの確かめ) を返す
    pub(super) fn xolazy_save(&self, d: &crate::simplex::mip_lp::RaceDiag) -> Option<f64> {
        let d_opt = d.d_opt.as_ref()?;
        let rc = self.lp.reduced_costs();
        let diff = d_opt.iter().zip(&rc).map(|(a, b)| (a - b).abs()).fold(0.0f64, f64::max);
        let mut duals = vec![("opt", 0.0, d_opt.clone())];
        if let (Some(g), Some(v)) = (d.gap_simplex, d.d_simplex.as_ref()) {
            duals.push(("simplex", g, v.clone()));
        }
        if let (Some(g), Some(v)) = (d.gap_ipm, d.d_ipm.as_ref()) {
            duals.push(("ipm", g, v.clone()));
        }
        let z = self.lp.objective() + self.p.offset;
        SAVED.with(|s| *s.borrow_mut() = Some(Saved { z, restarts: self.params.restarts, lo: self.dom.global_lo.clone(), up: self.dom.global_up.clone(), t_wait: d.t_wait, duals }));
        Some(diff)
    }

    /// 根の終わり (再スタート・木に入る直前) に呼ぶ: 控えた双対ごとに、今の打ち切り値と根の LP の時点の大域的な境界で固定できる整数列の数を
    /// `{"opt":n,"simplex":n,"ipm":n}` の形で返す (控えがなければ空)
    pub(super) fn xolazy_fix_counts(&self) -> String {
        let Some(s) = SAVED.with(|s| s.borrow_mut().take()) else { return String::new() };
        if s.restarts != self.params.restarts {
            return String::new();
        }
        let u = self.prune_limit();
        let p = self.p;
        let mut parts = Vec::new();
        let mut sets: Vec<(&str, Vec<bool>)> = Vec::new();
        for (name, gap, dv) in &s.duals {
            let lb = s.z - gap;
            let room = u - lb;
            let mut fixed = vec![false; p.n];
            if room.is_finite() && room >= 0.0 && dv.len() == p.n {
                for j in 0..p.n {
                    let (lo, up) = (s.lo[j], s.up[j]);
                    if !p.is_int[j] || lo == up {
                        continue;
                    }
                    let dj = dv[j];
                    // 境界から 1 動かすと下界が `room` を越える (`|d_j| > room`、整数の丸めの余裕つき)
                    fixed[j] = if dj > 1e-7 && lo.is_finite() {
                        (lo + room / dj + 1e-6).floor() <= lo
                    } else if dj < -1e-7 && up.is_finite() {
                        (up - room / (-dj) - 1e-6).ceil() >= up
                    } else {
                        false
                    };
                }
            }
            let nfix = fixed.iter().filter(|&&f| f).count();
            parts.push(format!("\"fix_{name}\":{nfix},\"gap_{name}\":{}", num(*gap)));
            sets.push((name, fixed));
        }
        // 最適な双対と内点法の双対の和集合
        let get = |nm: &str| sets.iter().find(|(n, _)| *n == nm).map(|(_, v)| v);
        if let (Some(a), Some(b)) = (get("opt"), get("ipm")) {
            let nu = a.iter().zip(b).filter(|(x, y)| **x || **y).count();
            parts.push(format!("\"fix_union\":{nu}"));
        }
        // 実際の固定 (今の大域的な境界) に足される分: その双対で固定できるが、今はまだ固定されていない整数列
        for nm in ["opt", "ipm", "simplex"] {
            if let Some(a) = get(nm) {
                let k = (0..p.n).filter(|&j| a[j] && self.dom.global_lo[j] < self.dom.global_up[j]).count();
                parts.push(format!("\"fix_{nm}_new\":{k}"));
            }
        }
        let t_wait = s.t_wait.map_or("null".to_string(), |v| format!("{v:.4}"));
        parts.push(format!("\"t_wait\":{t_wait}"));
        format!(",\"newly\":{},\"U\":{},\"z\":{},{}", self.newly_fixed_ints(), num(u), num(s.z), parts.join(","))
    }
}

/// JSON の数 (有限でなければ null)
fn num(v: f64) -> String {
    if v.is_finite() { format!("{v:.10e}") } else { "null".to_string() }
}
