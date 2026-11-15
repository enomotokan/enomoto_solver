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
    /// ナップサック行 (`knap`) のうち、同じ GUB の行 (0-1 列だけ・係数がそろい・文字の和 <= 1 か = 1) に入る列を 2 本以上持つ行の割合
    /// (分母はナップサック行。GUB 被覆が使える形)
    pub knap_gub: f64,
    /// ナップサック行の列のうち、どれかの GUB の行に入る列の割合 (分母はナップサック行の非零)
    pub knap_gub_col: f64,
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
    // GUB の行 (0-1 列だけ、係数がすべて同じ符号で絶対値がそろい、上の側が「1 本まで」: 正規化した右辺 <= 1) と、列ごとの最初の GUB
    let mut gub_of = vec![usize::MAX; p.n];
    for (i, r) in p.rows.iter().enumerate() {
        if r.len() < 2 || !r.iter().all(|&(j, _)| is_bin(p, j)) {
            continue;
        }
        let a0 = r[0].1;
        if !r.iter().all(|&(_, a)| (a - a0).abs() <= 1e-9 * a0.abs()) {
            continue;
        }
        let ub = if a0 > 0.0 { p.row_up[i] / a0 } else { -p.row_lo[i] / -a0 };
        if ub.is_finite() && ub <= 1.0 + 1e-9 {
            for &(j, _) in r {
                if gub_of[j] == usize::MAX {
                    gub_of[j] = i;
                }
            }
        }
    }
    let (mut nkg, mut knz, mut kgc) = (0usize, 0usize, 0usize);
    for r in &p.rows {
        if r.len() < 2 || !r.iter().all(|&(j, _)| is_bin(p, j)) {
            continue;
        }
        let (amin, amax) = r.iter().fold((f64::INFINITY, 0.0f64), |(lo, hi), &(_, a)| (lo.min(a.abs()), hi.max(a.abs())));
        if amax <= amin * (1.0 + 1e-9) {
            continue;
        }
        let mut g: Vec<usize> = r.iter().map(|&(j, _)| gub_of[j]).filter(|&g| g != usize::MAX).collect();
        knz += r.len();
        kgc += g.len();
        g.sort_unstable();
        if g.windows(2).any(|w| w[0] == w[1]) {
            nkg += 1;
        }
    }
    let fr = |a: usize, b: usize| if b == 0 { 0.0 } else { a as f64 / b as f64 };
    f.knap_gub = fr(nkg, nknap);
    f.knap_gub_col = fr(kgc, knz);
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
        "CUTMIX types restarts {restarts} m {} n {} vbrow {:.3} vbcol {:.3} fc {} fcnode {} cont {:.3} genint {:.3} knap {:.3} knaptight {:.3} knapgub {:.3} knapgubcol {:.3} mixknap {:.3} setp {:.3} introw {:.3} intsmall {:.3} flow {:.3}",
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
        f.knap_gub,
        f.knap_gub_col,
        f.mix_knap,
        f.setp,
        f.int_row,
        f.int_small,
        f.flow
    );
}

// ---------------------------------------------------------------------------------------------
// 試作 `ENOMOTO_MIP_CUTMIX` (既定では使わない)。値の文字で使う部品を選ぶ:
// - `f`: 根のカット選択の固定費用ネットワーク型の緩め (`fc_relax`: 質の閾値 0.5 → 0、平行度の上限 0.3 → 0.9) を、VUB 行の割合と
//   VUB を持つ連続列の割合から作る重み w ∈ [0, 1] で連続に変える ([`fc_weight`])。境目から離れた問題では今と同じ
// - `h`: 固定費用ネットワーク型の 2 択の判定 (根・ノード) にヒステリシスを持たせる ([`fc_hyst`])。一度型と判定したら、再スタートの
//   後は VUB 行の割合がしきい値から `ENOMOTO_T_CUTMIX_HYST_ROW` (0.5)、VUB を持つ連続列の割合が `ENOMOTO_T_CUTMIX_HYST` (0.1)
//   下がるまで型のまま
// - `c`: 持ち上げつきの被覆 (`ENOMOTO_MIP_ROW_LIFTED_COVER` と同じ、元の行 1 本ずつ) を、ナップサック行の割合が
//   `ENOMOTO_T_CUTMIX_LC_MIN` 以上の問題でだけ使う ([`lifted_cover_on`])
// - `z`: zero-half (既定で切ってある) を、全部整数で係数が小さい行の割合が `ENOMOTO_T_CUTMIX_ZH_MIN` 以上の問題でだけ使う ([`zh_on`])
// 割合は外側の求解の根のカットのループの初め (再スタートごと) に数え直す ([`at_root`])。状態はプロセスで 1 つ (外側の求解の始めに
// [`reset`] で戻す。同じプロセスで同時に別の求解をすると混ざる: `ENOMOTO_MIP_CUTMIX` を付けたときだけ)
// ---------------------------------------------------------------------------------------------

use std::sync::Mutex;

#[derive(Default)]
struct State {
    /// 最後の根で数えた割合
    fracs: Option<TypeFracs>,
    /// ヒステリシスの判定 (0: 根の判定、1: ノードの判定) の前の値
    fc_prev: [Option<bool>; 2],
}

static STATE: Mutex<State> = Mutex::new(State { fracs: None, fc_prev: [None, None] });

fn mode() -> Option<&'static str> {
    env_str!("ENOMOTO_MIP_CUTMIX")
}

pub(super) fn has(c: char) -> bool {
    mode().is_some_and(|m| m.contains(c))
}

/// 外側の求解の始めに状態を戻す
pub(super) fn reset() {
    if mode().is_some() {
        *STATE.lock().unwrap() = State::default();
    }
}

/// 根のカットのループの初め (外側の求解、再スタートごと): 割合を数えて覚え、`ENOMOTO_MIP_LOG` なら診断の行を出す
pub(super) fn at_root(p: &MipProblem, x: Option<&[f64]>, restarts: u32, fc: bool, fcnode: bool) {
    if mode().is_some() {
        STATE.lock().unwrap().fracs = Some(type_fracs(p, x));
    }
    log_types(p, x, restarts, fc, fcnode);
}

/// 根のループで割合を数えるか (診断のログか試作のとき)
pub(super) fn want_root() -> bool {
    mode().is_some() || env_str!("ENOMOTO_MIP_LOG").is_some()
}

fn fracs() -> Option<TypeFracs> {
    STATE.lock().unwrap().fracs
}

fn ramp(v: f64, lo: f64, hi: f64) -> f64 {
    if hi <= lo {
        return if v >= hi { 1.0 } else { 0.0 };
    }
    ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
}

/// `f`: 根のカット選択の緩めの重み (0: 緩めない、1: 今の固定費用ネットワーク型の緩め)。`fc` は今の 2 択の判定。
/// w = ramp(VUB 行の割合; `ENOMOTO_T_CUTMIX_FC_LO` 0.6, `..._FC_HI` 0.8) × ramp(VUB を持つ連続列の割合; 0.8, 1.0)。
/// `f` でないときは今の 2 択 (1 か 0)
pub(super) fn fc_weight(p: &MipProblem, fc: bool) -> f64 {
    if !has('f') {
        return if fc { 1.0 } else { 0.0 };
    }
    let f = type_fracs(p, None);
    if f.cont == 0.0 {
        return 0.0;
    }
    let w = ramp(f.vb_row, tunable!("ENOMOTO_T_CUTMIX_FC_LO", 0.6, f64), tunable!("ENOMOTO_T_CUTMIX_FC_HI", 0.8, f64))
        * ramp(f.vb_col, tunable!("ENOMOTO_T_CUTMIX_FC_COL_LO", 0.8, f64), tunable!("ENOMOTO_T_CUTMIX_FC_COL_HI", 1.0, f64));
    // 今の判定で型の問題は少なくとも半分は緩める (しきい値の上の側で今より弱くしすぎない)
    if fc { w.max(0.5) } else { w }
}

/// `f`: 重み w での根のカット選択の (質の閾値, 平行度の上限)。w = 1 で今の固定費用型 (0, 0.9)、w = 0 で既定 (0.5, 0.3)
pub(super) fn fc_sel_params(w: f64) -> (f64, f64) {
    let (r0, p0) = (tunable!("ENOMOTO_T_CUTSEL_MIN_REL", 0.5, f64), tunable!("ENOMOTO_T_CUTSEL_MAXPAR", 0.3, f64));
    let (r1, p1) = (tunable!("ENOMOTO_T_CUTSEL_FC_MIN_REL", 0.0, f64), tunable!("ENOMOTO_T_CUTSEL_FC_MAXPAR", 0.9, f64));
    (r0 + (r1 - r0) * w, p0 + (p1 - p0) * w)
}

/// `h`: 固定費用ネットワーク型の判定のヒステリシス。`raw` は今の判定、`row_frac`・`col_frac` はしきい値、`vbrow`・`vbcol` は割合。
/// `h` でなければ `raw` のまま
pub(super) fn fc_hyst(raw: bool, row_frac: f64, col_frac: f64, vbrow: f64, vbcol: f64) -> bool {
    if !has('h') {
        return raw;
    }
    let slot = if row_frac < 0.6 { 1 } else { 0 };
    let mut st = STATE.lock().unwrap();
    let v = match st.fc_prev[slot] {
        Some(true) if !raw => {
            // 再スタートで 0-1 列が固定されると VUB 行は消えるが流量保存の行は残るので、VUB 行の割合は大きく下がる (h80x6320d:
            // 0.97 → 0.57、p200x1188c: 0.86 → 0.59 → 0.39 → 0.24)。残る連続列はみな VUB を持つまま (1.0) なので、行の割合の幅は広く取る
            let hr = tunable!("ENOMOTO_T_CUTMIX_HYST_ROW", 0.5, f64);
            let hc = tunable!("ENOMOTO_T_CUTMIX_HYST", 0.1, f64);
            vbrow >= row_frac - hr && vbcol >= col_frac - hc
        }
        _ => raw,
    };
    st.fc_prev[slot] = Some(v);
    v
}

/// `c`: 持ち上げつきの被覆を使うか (ナップサック行 (0-1 だけ、または一般整数・連続列を含む) の割合が `ENOMOTO_T_CUTMIX_LC_MIN` 以上)
pub(super) fn lifted_cover_on() -> bool {
    has('c') && fracs().is_some_and(|f| f.knap.max(f.mix_knap) >= tunable!("ENOMOTO_T_CUTMIX_LC_MIN", 0.1, f64))
}

/// `z`: zero-half を使うか (全部整数で係数が絶対値 2 以下の行の割合が `ENOMOTO_T_CUTMIX_ZH_MIN` 以上)
pub(super) fn zh_on() -> bool {
    has('z') && fracs().is_some_and(|f| f.int_small >= tunable!("ENOMOTO_T_CUTMIX_ZH_MIN", 0.5, f64))
}

/// 診断 (`ENOMOTO_MIP_LOG` のとき): 衝突グラフの辺のうち、行から作る辺とプロービングの含意だけから来る辺の数を
/// `CUTMIX clique edges ...` の 1 行で出す (クリーク行の拡張の初め、前処理の後の問題)。行から作る辺は行の伝播で
/// すでに分かる固定しか生まないので、伝播に衝突グラフを使って増える固定の上限の目安は含意だけの辺の数になる
pub(super) fn log_clique(p: &MipProblem, binary: &[bool], total_edges: usize) {
    if env_str!("ENOMOTO_MIP_LOG").is_none() {
        return;
    }
    let g = super::clique::CliqueGraph::build_with(p.n, &p.rows, &p.row_lo, &p.row_up, binary, &p.col_lo, &p.col_up, &[]);
    let rows = g.num_edges();
    let nbin = binary.iter().filter(|&&b| b).count();
    eprintln!("CUTMIX clique edges total {total_edges} rows {rows} implonly {} implpairs {} bin {nbin}", total_edges.saturating_sub(rows), p.conflict_edges.len());
}
