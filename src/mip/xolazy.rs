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
        let mut duals = vec![("opt", d.gap_opt.unwrap_or(0.0), d_opt.clone())];
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

thread_local! {
    /// 試作 `ENOMOTO_MIP_IPMRC` の `t`: 木に入るまで取っておく (再スタートの回数, 下界, 被約費用の一覧)
    static IPMRC_DEFER: RefCell<Option<(u32, f64, Vec<(usize, bool, f64, f64)>)>> = const { RefCell::new(None) };
}

impl<'a, L: MipLp> Solver<'a, L> {
    /// 試作 `ENOMOTO_MIP_IPMRC` の `t`: 木に入るとき (主、再スタートの判断の後) に、取っておいた内点法の双対の被約費用を
    /// [`Self::t4_rc_extra`] に足して締める (木の中の再スタートの判断には効く)
    pub(super) fn ipmrc_tree_start(&mut self) {
        let Some((r, lb, v)) = IPMRC_DEFER.with(|d| d.borrow_mut().take()) else { return };
        if r != self.params.restarts {
            return;
        }
        let fixed0 = self.newly_fixed_ints();
        let nv = v.len();
        self.t4_rc_extra.push((lb, v));
        if self.root_redcost.is_some() {
            self.root_redcost_fixing();
        }
        if self.params.verbose || crate::simplex::mip_lp::xolazy_log() {
            eprintln!(
                "XOLAZY {{\"ev\":\"ipmrc_tree\",\"restarts\":{},\"lb\":{},\"cols\":{nv},\"fixed_now\":{}}}",
                self.params.restarts,
                num(lb),
                self.newly_fixed_ints().saturating_sub(fixed0)
            );
        }
    }

    /// 試作 `ENOMOTO_MIP_IPMRC` の `u` (analysis/ipmrc_20261111.md): 根の LP の直後、同時実行で内点法の点が手に入っていれば、その双対の
    /// ラグランジュ下界 `L(y)` と被約費用 `d = c - A^T y` を [`Self::t4_rc_extra`] に足す。以後 [`Self::root_redcost_fixing`] が打ち切り値
    /// `U` で `L(y) + |d_j| (x_j - 境界) <= U` を満たす範囲に整数列を締める (`TREE4_RCUNION` の根のラウンドの被約費用と同じ形)。
    ///
    /// 正しさ: `L(y) = b·y + Σ_j min_{l_j <= x_j <= u_j} d_j x_j` はどの `y` でも LP の (したがって MIP の) 下界で、各項の最小を
    /// 境界の値に置けば `c·x >= L(y) + d_j (x_j - l_j)` (`d_j > 0`) が LP の境界の中のすべての `x` で成り立つ。内点法の双対は近似なので
    /// 頂点の双対と違って被約費用の符号が逆の列 (境界の内側) を持つが、それも `L(y)` の中で最小の側の境界に置いて数えている。
    /// スラック列の符号は射影で合わせ (`lagrange_std`)、浮動小数の和の誤差の余裕を引く。境界の基準は今の大域的な境界 (根の LP の境界と
    /// 同じか内側なので、上の不等式はそのまま成り立つ)。`L(y)` が LP の値を越えるとき (あり得ない) は使わない
    pub(super) fn ipmrc_store(&mut self, d: Option<&crate::simplex::mip_lp::RaceDiag>) {
        if self.params.submip || self.snap.is_some() {
            return;
        }
        let Some(d) = d else { return };
        let (Some(lb0), Some(dv)) = (d.lb_ipm, d.d_ipm.as_ref()) else { return };
        let p = self.p;
        if dv.len() != p.n || !lb0.is_finite() {
            return;
        }
        let lb = lb0 + p.offset;
        let z = self.lp.objective() + p.offset;
        if lb > z + 1e-6 * (1.0 + z.abs()) {
            if self.params.verbose {
                eprintln!("MIP: IPMRC: the interior-point bound {lb:.10e} exceeds the root LP value {z:.10e}; not used");
            }
            return;
        }
        let mut v = Vec::new();
        for j in 0..p.n {
            if !p.is_int[j] || self.dom.global_lo[j] >= self.dom.global_up[j] {
                continue;
            }
            let dj = dv[j];
            if dj > 1e-7 && self.dom.global_lo[j].is_finite() {
                v.push((j, true, dj, self.dom.global_lo[j]));
            } else if dj < -1e-7 && self.dom.global_up[j].is_finite() {
                v.push((j, false, dj, self.dom.global_up[j]));
            }
        }
        let fixed0 = self.newly_fixed_ints();
        let nv = v.len();
        // `t`: 再スタートの判断には使わない。木に入るとき ([`Self::ipmrc_tree_start`]) に足す
        if crate::simplex::mip_lp::ipmrc_has('t') {
            IPMRC_DEFER.with(|d| *d.borrow_mut() = Some((self.params.restarts, lb, v)));
            return;
        }
        self.t4_rc_extra.push((lb, v));
        // 暫定解がもうあれば、すぐ締める (根の被約費用の記録がまだなら次の呼び出しで締まる)
        if self.root_redcost.is_some() {
            self.root_redcost_fixing();
        }
        if self.params.verbose || crate::simplex::mip_lp::xolazy_log() {
            eprintln!(
                "XOLAZY {{\"ev\":\"ipmrc\",\"restarts\":{},\"lb\":{},\"z\":{},\"cols\":{nv},\"fixed_now\":{}}}",
                self.params.restarts,
                num(lb),
                num(z),
                self.newly_fixed_ints().saturating_sub(fixed0)
            );
        }
    }
}

/// JSON の数 (有限でなければ null)
fn num(v: f64) -> String {
    if v.is_finite() { format!("{v:.10e}") } else { "null".to_string() }
}
