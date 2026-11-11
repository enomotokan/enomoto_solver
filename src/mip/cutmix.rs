//! 問題の型の割合と、割合に応じたカットの量の試作 (analysis/cutmix_20261111.md、docs/backlog.md の
//! 「問題の型によるカットの選び分けを、型の有無の 2 択から割合に応じた連続的な量・回数に変える」)。
//!
//! 診断 (`ENOMOTO_MIP_LOG` のとき): 根のカットのループの初め (再スタートごと) に、型を表す割合と今の 2 択の判定を
//! `CUTMIX types ...` の 1 行で出す。数えるのは前処理の後の問題 (固定された列・空の行は除く)。

use super::problem::MipProblem;

/// 型を表す割合 (分母は行の割合なら空でない行、列の割合なら固定されていない列)
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TypeFracs {
    /// 空でない行の数・固定されていない列の数
    pub m: usize,
    pub n: usize,
    /// 連続 1 本・整数 1 本の 2 非零の行 (VUB/VLB) の割合 (分母は `p.m`、[`super::separation`] の固定費用ネットワーク型の判定と同じ)
    pub vb_row: f64,
    /// VUB/VLB の行を持つ連続列の割合 (分母は連続列)
    pub vb_col: f64,
    /// 連続列の割合
    pub cont: f64,
    /// 整数列のうち 0-1 でないものの割合
    pub genint: f64,
    /// 0-1 列だけで係数の絶対値がそろっていない (ナップサック) 行の割合
    pub knap: f64,
    /// ナップサック行のうち根の LP の点で等号 (活性) の行の割合 (分母はナップサック行。点がないときは 0)
    pub knap_tight: f64,
    /// 整数列が 2 本以上で、その係数の絶対値がそろっておらず、連続列が整数列より多くない行 (一般整数・連続列を含むナップサック行) の割合
    pub mix_knap: f64,
    /// 0-1 列だけで係数の絶対値がそろった行 (集合分割・詰め込み・被覆・個数) の割合
    pub setp: f64,
    /// 整数列だけで係数が整数の行の割合
    pub int_row: f64,
    /// 整数列だけで係数が整数かつ絶対値 2 以下の行の割合
    pub int_small: f64,
    /// 係数が ±1 の等式 (流量保存らしい行) の割合
    pub flow: f64,
}

fn is_bin(p: &MipProblem, j: usize) -> bool {
    p.is_int[j] && p.col_lo[j] >= -1e-9 && p.col_up[j] <= 1.0 + 1e-9
}

/// 型を表す割合を数える。`x` があれば (根の LP の点) ナップサック行の活性も数える
pub(super) fn type_fracs(p: &MipProblem, x: Option<&[f64]>) -> TypeFracs {
    let mut f = TypeFracs::default();
    let live = |j: usize| !p.cols[j].is_empty() && p.col_lo[j] < p.col_up[j];
    let (mut ncont, mut nint, mut ngen) = (0usize, 0usize, 0usize);
    for j in 0..p.n {
        if !live(j) {
            continue;
        }
        f.n += 1;
        if p.is_int[j] {
            nint += 1;
            if !is_bin(p, j) {
                ngen += 1;
            }
        } else {
            ncont += 1;
        }
    }
    let mut has_vb = vec![false; p.n];
    let (mut nikn, mut nvb, mut nknap, mut ntight, mut nsetp, mut nintr, mut nsmall, mut nflow) = (0usize, 0usize, 0, 0, 0, 0, 0, 0);
    for (i, r) in p.rows.iter().enumerate() {
        if r.is_empty() {
            continue;
        }
        f.m += 1;
        if r.len() == 2 {
            let (a, b) = (r[0].0, r[1].0);
            let c = match (p.is_int[a], p.is_int[b]) {
                (false, true) => Some(a),
                (true, false) => Some(b),
                _ => None,
            };
            if let Some(c) = c {
                nvb += 1;
                has_vb[c] = true;
            }
        }
        let all_int = r.iter().all(|&(j, _)| p.is_int[j]);
        let all_bin = all_int && r.iter().all(|&(j, _)| is_bin(p, j));
        let (mut amin, mut amax) = (f64::INFINITY, 0.0f64);
        let mut int_coef = true;
        for &(_, a) in r {
            let aa = a.abs();
            amin = amin.min(aa);
            amax = amax.max(aa);
            int_coef &= (a - a.round()).abs() <= 1e-9;
        }
        {
            let (mut imin, mut imax, mut ni) = (f64::INFINITY, 0.0f64, 0usize);
            for &(j, a) in r.iter().filter(|&&(j, _)| p.is_int[j]) {
                let _ = j;
                imin = imin.min(a.abs());
                imax = imax.max(a.abs());
                ni += 1;
            }
            if ni >= 2 && r.len() - ni <= ni && imax > imin * (1.0 + 1e-9) {
                nikn += 1;
            }
        }
        if all_bin && r.len() >= 2 {
            if amax > amin * (1.0 + 1e-9) {
                nknap += 1;
                if let Some(x) = x {
                    let act: f64 = r.iter().map(|&(j, a)| a * x[j]).sum();
                    let tol = 1e-6 * (1.0 + act.abs());
                    if (act - p.row_lo[i]).abs() <= tol || (act - p.row_up[i]).abs() <= tol {
                        ntight += 1;
                    }
                }
            } else {
                nsetp += 1;
            }
        }
        if all_int && int_coef {
            nintr += 1;
            if amax <= 2.0 + 1e-9 {
                nsmall += 1;
            }
        }
        if r.len() >= 2 && p.row_lo[i] == p.row_up[i] && r.iter().all(|&(_, a)| (a.abs() - 1.0).abs() <= 1e-12) {
            nflow += 1;
        }
    }
    let fr = |a: usize, b: usize| if b == 0 { 0.0 } else { a as f64 / b as f64 };
    f.vb_row = fr(nvb, p.m);
    f.vb_col = fr(has_vb.iter().filter(|&&v| v).count(), ncont);
    f.cont = fr(ncont, f.n);
    f.genint = fr(ngen, nint);
    f.knap = fr(nknap, f.m);
    f.knap_tight = fr(ntight, nknap);
    f.setp = fr(nsetp, f.m);
    f.mix_knap = fr(nikn, f.m);
    f.int_row = fr(nintr, f.m);
    f.int_small = fr(nsmall, f.m);
    f.flow = fr(nflow, f.m);
    f
}

/// 診断のログ (`ENOMOTO_MIP_LOG`): 根のカットのループの初めに、割合と今の 2 択の判定 (`fc`: 根の固定費用ネットワーク型、
/// `fcnode`: ノードの分離を強める判定) を出す
pub(super) fn log_types(p: &MipProblem, x: Option<&[f64]>, restarts: u32, fc: bool, fcnode: bool) {
    if env_str!("ENOMOTO_MIP_LOG").is_none() {
        return;
    }
    let f = type_fracs(p, x);
    eprintln!(
        "CUTMIX types restarts {restarts} m {} n {} vbrow {:.3} vbcol {:.3} fc {} fcnode {} cont {:.3} genint {:.3} knap {:.3} knaptight {:.3} mixknap {:.3} setp {:.3} introw {:.3} intsmall {:.3} flow {:.3}",
        f.m,
        f.n,
        f.vb_row,
        f.vb_col,
        fc as u8,
        fcnode as u8,
        f.cont,
        f.genint,
        f.knap,
        f.knap_tight,
        f.mix_knap,
        f.setp,
        f.int_row,
        f.int_small,
        f.flow
    );
}
